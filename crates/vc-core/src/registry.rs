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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputDevice {
    WiimoteSideways,
    ClassicController,
    Gamecube,
}

/// Default button mapping shipped with a core, keyed by the console's own
/// button names (e.g. "A", "B", "L", "R") pointing at a physical button
/// enum understood by the DOL at boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    pub device: InputDevice,
    /// console button name -> physical button id understood by the core
    pub map: std::collections::BTreeMap<String, String>,
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

