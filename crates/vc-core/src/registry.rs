//! Core plugin registry.
//!
//! Each supported system (NES, SNES, Genesis, etc.) is described by a
//! `CoreDefinition` loaded from `cores/registry.json`. Adding a new system
//! means adding a JSON entry here — no changes to packing code.
//!
//! A core's DOL comes from one of two sources (`CoreSource`):
//! - `Bundled`: a DOL this project actually owns/built (e.g. an original
//!   from-scratch libretro-based port) and can ship directly.
//! - `Donor`: nothing is shipped. The DOL is ripped at runtime out of a
//!   donor WAD the *user* supplies — see `donor.rs`. This is the default
//!   for any core derived from or resembling an official Nintendo VC
//!   emulator, since redistributing that binary ourselves would be
//!   redistributing Nintendo's copyrighted code.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::VcError;

/// Physical/virtual input device a mapping targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InputDevice {
    WiimoteSideways,
    ClassicController,
    Gamecube,
}

impl InputDevice {
    /// Snake-case name used in the registry, the UI and the core config file.
    pub fn as_str(self) -> &'static str {
        match self {
            InputDevice::WiimoteSideways => "wiimote_sideways",
            InputDevice::ClassicController => "classic_controller",
            InputDevice::Gamecube => "gamecube",
        }
    }

    /// Every physical input id of this device starts with this prefix.
    pub fn physical_prefix(self) -> &'static str {
        match self {
            InputDevice::WiimoteSideways => "wiimote_",
            InputDevice::ClassicController => "classic_",
            InputDevice::Gamecube => "gc_",
        }
    }
}

/// Default button mapping shipped with a core, keyed by the console's own
/// button names (e.g. "A", "B", "L", "R") pointing at a physical button
/// enum understood by the DOL at boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    pub device: InputDevice,
    /// Which emulated controller this map is for (1 = the first/only one).
    /// Ports above 1 must be declared in `CoreDefinition::controller_ports`.
    #[serde(default = "first_port")]
    pub port: u8,
    /// If set, this map is the default only while the condition holds and
    /// takes precedence over the unconditional map for the same device and
    /// port. Lets an option (e.g. dual-controller mode) switch the default
    /// layout.
    #[serde(default)]
    pub when: Option<OptionCondition>,
    /// console button name -> physical button id understood by the core.
    /// An empty id means the button exists but starts out unmapped.
    pub map: std::collections::BTreeMap<String, String>,
}

fn first_port() -> u8 {
    1
}

/// An additional emulated controller driven by the same physical controller
/// (e.g. N64 controller 2 in dual-analog mode). Port 1 always exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerPort {
    /// 2..=4.
    pub port: u8,
    pub label: String,
    /// The port only exists while this holds; always present if omitted.
    #[serde(default)]
    pub when: Option<OptionCondition>,
}

/// How a core option is presented in the UI and typed in the core config file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionKind {
    /// On/off switch. `default` is a JSON bool; written as a JSON bool.
    Toggle,
    /// One of `choices`. `default` is a choice `value`; written as that
    /// string, so choice values are part of the core-facing contract.
    Select,
    /// Integer in `min..=max` (both required, within 0..=255); written as a
    /// JSON number.
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionChoice {
    pub value: String,
    pub label: String,
}

/// Show an option only while another (earlier-declared) option has a given value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionCondition {
    /// Id of an option declared *before* this one.
    pub option: String,
    pub equals: serde_json::Value,
}

/// A user-selectable setting specific to one core, e.g. an N64 Expansion
/// Pak or a DS Slot-2 accessory. The UI builds the core's "Core Options"
/// panel from these; `coreconfig::build_core_config` validates the chosen
/// values and writes them into the core config file at `key`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreOption {
    /// Unique within the core, snake_case.
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Section heading the option is listed under in the UI.
    #[serde(default)]
    pub group: Option<String>,
    pub kind: OptionKind,
    pub default: serde_json::Value,
    /// Where the value lands in the core config file, as `<section>.<name>`.
    /// Either a standard key from `coreconfig::STANDARD_KEYS` (shared by all
    /// cores, so the common interface layer can read it uniformly) or
    /// `extra.<name>` for a setting only this core understands. Defaults to
    /// `extra.<id>` when omitted.
    #[serde(default)]
    pub key: Option<String>,
    /// Required for `select`.
    #[serde(default)]
    pub choices: Vec<OptionChoice>,
    /// `number` only.
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    /// If non-empty, the option only applies when the selected controller is
    /// one of these (e.g. host rumble needs a controller that can rumble).
    #[serde(default)]
    pub devices: Vec<InputDevice>,
    #[serde(default)]
    pub visible_when: Option<OptionCondition>,
}

impl CoreOption {
    /// Destination key in the core config file (`section.name`).
    pub fn config_key(&self) -> String {
        self.key.clone().unwrap_or_else(|| format!("extra.{}", self.id))
    }
}

/// Where a core's DOL comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreSource {
    /// A DOL this project owns outright and can ship in `cores/`.
    Bundled { dol_path: PathBuf },
    /// No binary shipped with this tool. At build time the DOL must be
    /// extracted from a donor WAD the user supplies themselves — see
    /// `donor::extract_core`.
    Donor {
        /// Title id of the official WAD this core's DOL should be ripped
        /// from. NES/SNES/etc. VC titles largely share one core per
        /// region across different games, but this pins to one
        /// known-good donor rather than accepting any title of that
        /// system — see docs/DESIGN.md for the tradeoff.
        title_id: [u8; 8],
        /// Which content index within the donor WAD holds the DOL
        /// (content 0 in essentially every real VC WAD).
        dol_content_index: u16,
        /// Human-readable description shown to the user when asking them
        /// to supply this donor, e.g. "Any legitimately-owned NES VC
        /// title, US region (tested against Super Mario Bros.)".
        donor_label: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreDefinition {
    /// Human-readable system name, e.g. "SNES"
    pub system: String,
    /// Unique id used in title-id allocation / CLI selection, e.g. "snes"
    pub id: String,
    pub core_source: CoreSource,
    /// Accepted ROM file extensions, lowercase, no leading dot
    pub valid_extensions: Vec<String>,
    /// Bytes to strip from the start of the ROM before embedding, if any
    /// (e.g. 512-byte SMC headers on some SNES dumps)
    pub header_size: Option<u32>,
    /// Default per-console button mapping(s), one per supported input device
    pub default_mappings: Vec<ButtonMapping>,
    /// 4-byte title id prefix used when allocating new title ids for WADs
    /// built with this core (kept in the homebrew-safe range)
    pub title_id_prefix: [u8; 4],
    /// IOS version this core's DOL expects to run under
    pub ios_version: u16,
    /// Per-core settings shown in the Configuration step (accessories,
    /// video/compatibility switches, ...). Empty for cores with none.
    #[serde(default)]
    pub options: Vec<CoreOption>,
    /// Console buttons that only exist while an option has a given value,
    /// e.g. Genesis X/Y/Z/Mode need the 6-button pad. Buttons not listed
    /// here are always present. Keys must appear in `default_mappings`.
    #[serde(default)]
    pub button_requires: std::collections::BTreeMap<String, OptionCondition>,
    /// Extra emulated controllers the single physical controller can drive.
    #[serde(default)]
    pub controller_ports: Vec<ControllerPort>,
}

/// Load every core definition listed in `registry.json` at the given path.
/// Bundled DOL paths are resolved relative to the registry file's parent
/// directory; donor requirements need no path resolution since nothing is
/// shipped for them.
pub fn load_registry(path: &Path) -> Result<Vec<CoreDefinition>, VcError> {
    let raw = fs::read_to_string(path)?;
    let mut defs: Vec<CoreDefinition> = serde_json::from_str(&raw)?;

    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for def in &mut defs {
        crate::options::validate_definitions(def)?;
        if let CoreSource::Bundled { dol_path } = &mut def.core_source {
            if dol_path.is_relative() {
                *dol_path = base.join(&dol_path);
            }
        }
    }
    Ok(defs)
}

/// Convenience lookup by core id (e.g. "nes", "snes").
pub fn find_core<'a>(defs: &'a [CoreDefinition], id: &str) -> Result<&'a CoreDefinition, VcError> {
    defs.iter()
        .find(|d| d.id == id)
        .ok_or_else(|| VcError::UnknownCore(id.to_string()))
}

