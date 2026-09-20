//! The unified core config file.
//!
//! Every emulator core reads the same JSON document, so the common
//! interface layer can be written once. Its layout is fixed:
//!
//! ```text
//! { "format": "vcforge-config", "version": 1,
//!   "core":        { "id", "system" },
//!   "input":       { "devices": { "<device>": { "buttons": {console button -> physical input},
//!                                                "ports": {"2": {"buttons": {...}}} } },
//!                    ...settings },
//!   "video":       { ... },   "audio":  { ... },   "system":      { ... },
//!   "save":        { ... },   "accessories": { ... },
//!   "extra":       { ... } }  // settings only this core understands
//! ```
//!
//! `input.devices` has one entry per enabled physical controller (classic
//! controller, Wiimote, Wiimote + Nunchuk, GameCube pad). They are all active
//! at once; each has its own bindings. Within a device, `buttons` is emulated
//! controller 1. When one physical controller drives several emulated
//! controllers (e.g. N64 dual-analog), the others are listed under `ports`,
//! keyed by port number (`{}` when there are none).
//!
//! All sections are always present (possibly empty). Every option the core
//! defines is written with its effective value, so the reader never needs to
//! know defaults. Cores share the *same keys* for the same concept (see
//! [`STANDARD_KEYS`]); anything genuinely core-specific goes under `extra`.
//! Unknown keys must be ignored by readers, which is what lets the format
//! grow without breaking older cores.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::error::VcError;
use crate::options::{condition_holds, resolve_options};
use crate::registry::{CoreDefinition, CoreOption, InputDevice, OptionKind};

/// The user's bindings for every emulated controller. `buttons` is port 1;
/// `ports` holds ports 2 and up. Ids are physical inputs (`classic_a`, ...).
#[derive(Debug, Clone, Default)]
pub struct InputMapping {
    pub buttons: BTreeMap<String, String>,
    pub ports: BTreeMap<u8, BTreeMap<String, String>>,
}

pub const FORMAT: &str = "vcforge-config";
pub const FORMAT_VERSION: u32 = 2;

/// Sections a standard key may live in (besides `extra`).
pub const SECTIONS: [&str; 6] = ["input", "video", "audio", "system", "save", "accessories"];
/// Section for core-specific settings.
pub const EXTRA: &str = "extra";

/// A setting shared across cores. Options that use one of these keys must
/// match its kind and, where given, its allowed values or range, so every
/// core spells the same concept the same way.
pub struct StandardKey {
    pub key: &'static str,
    pub kind: OptionKind,
    /// Select only: the values a choice may take. `None` = core-defined.
    pub values: Option<&'static [&'static str]>,
    /// Number only: inclusive bounds an option's min/max must stay within.
    pub range: Option<(u8, u8)>,
    pub doc: &'static str,
}

const fn key(
    key: &'static str,
    kind: OptionKind,
    values: Option<&'static [&'static str]>,
    range: Option<(u8, u8)>,
    doc: &'static str,
) -> StandardKey {
    StandardKey { key, kind, values, range, doc }
}

use OptionKind::{Number, Select, Toggle};

pub const STANDARD_KEYS: &[StandardKey] = &[
    key("input.turbo", Toggle, None, None, "Auto-fire on the primary action buttons."),
    key("input.dual_controller", Toggle, None, None, "One physical controller drives two emulated controllers (see `input.ports`)."),
    key("input.six_button", Toggle, None, None, "Use the six-button pad variant (adds buttons; see `button_requires`)."),
    key("input.rumble_strength", Number, None, Some((0, 100)), "Host controller rumble strength in percent; 0 disables."),
    key("video.timing", Select, Some(&["ntsc", "pal"]), None, "Refresh-rate standard the emulated system runs at."),
    key("video.crop_overscan", Toggle, None, None, "Crop the scanlines a TV would have hidden."),
    key("video.filter", Select, Some(&["sharp", "smooth"]), None, "Scaling/texture filter."),
    key("video.layout", Select, Some(&["stacked", "side_by_side", "single"]), None, "Arrangement of multiple screens."),
    key("video.palette", Select, None, None, "Color palette (values are core-defined)."),
    key("video.color_correction", Toggle, None, None, "Approximate the original LCD's colors."),
    key("audio.filter", Toggle, None, None, "Low-pass filter approximating the original output."),
    key("system.region", Select, Some(&["us", "eu", "jp"]), None, "Console region reported to software."),
    key("save.type", Select, None, None, "Save hardware type (values are core-defined)."),
    key("accessories.multitap", Toggle, None, None, "Multiplayer adapter."),
    key("accessories.memory_card", Toggle, None, None, "Removable save memory."),
    key("accessories.expansion_ram", Toggle, None, None, "Console RAM expansion."),
    key("accessories.rtc", Toggle, None, None, "Real-time clock."),
];

pub fn standard_key(key: &str) -> Option<&'static StandardKey> {
    STANDARD_KEYS.iter().find(|k| k.key == key)
}

/// Check that `key` is a legal destination for `opt`: either a standard key
/// (with a matching kind/values/range) or `extra.<name>`.
pub fn validate_option_key(opt: &CoreOption, key: &str) -> Result<(), String> {
    let (section, name) = key
        .split_once('.')
        .ok_or_else(|| format!("config key '{key}' must look like '<section>.<name>'"))?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(format!("config key '{key}': the name must be lowercase letters, digits and '_' (no further dots)"));
    }
    if section == EXTRA {
        return Ok(());
    }

    let std = standard_key(key).ok_or_else(|| {
        format!("'{key}' is not a standard key; core-specific settings belong under 'extra.' (e.g. 'extra.{}')", opt.id)
    })?;
    if std.kind != opt.kind {
        return Err(format!("'{key}' is a {:?} setting but the option is {:?}", std.kind, opt.kind));
    }
    if let (OptionKind::Select, Some(allowed)) = (opt.kind, std.values) {
        if let Some(bad) = opt.choices.iter().find(|c| !allowed.contains(&c.value.as_str())) {
            return Err(format!("choice '{}' is not allowed for '{key}' (allowed: {})", bad.value, allowed.join(", ")));
        }
    }
    if let (OptionKind::Number, Some((lo, hi))) = (opt.kind, std.range) {
        let (min, max) = (opt.min.unwrap_or(0.0), opt.max.unwrap_or(255.0));
        if min < f64::from(lo) || max > f64::from(hi) {
            return Err(format!("'{key}' must stay within {lo}..={hi}"));
        }
    }
    Ok(())
}

type Section = BTreeMap<String, Value>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CoreInfo {
    pub id: String,
    pub system: String,
}

/// Bindings for an additional emulated controller.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PortConfig {
    pub buttons: BTreeMap<String, String>,
}

/// Bindings for one enabled physical controller.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeviceConfig {
    /// Console button -> physical input id (prefixed by the device, e.g.
    /// `classic_`, `gc_`, `wiimote_`/`nunchuk_`). Only buttons that exist for
    /// this configuration are listed; unmapped buttons are omitted. This is
    /// emulated controller 1.
    pub buttons: BTreeMap<String, String>,
    /// Additional emulated controllers driven by this physical controller,
    /// keyed by port number ("2", "3", ...). Empty for single-controller setups.
    pub ports: BTreeMap<String, PortConfig>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InputSection {
    /// One entry per enabled physical controller, keyed by device name
    /// (`classic_controller`, `wiimote_sideways`, `wiimote_nunchuk`,
    /// `gamecube`). All listed devices are active at the same time.
    pub devices: BTreeMap<String, DeviceConfig>,
    #[serde(flatten)]
    pub settings: Section,
}

/// The document handed to the emulator core. Field order here is the order
/// keys appear in the file.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CoreConfig {
    pub format: &'static str,
    pub version: u32,
    pub core: CoreInfo,
    pub input: InputSection,
    pub video: Section,
    pub audio: Section,
    pub system: Section,
    pub save: Section,
    pub accessories: Section,
    pub extra: Section,
}

impl CoreConfig {
    /// Pretty-printed JSON with a trailing newline (keys inside sections sorted).
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("config is always serializable");
        s.push('\n');
        s
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_json().into_bytes()
    }
}

/// Every console button an emulated controller `port` has (any device, any layout variant).
fn known_buttons(core: &CoreDefinition, port: u8) -> BTreeSet<&str> {
    core.default_mappings
        .iter()
        .filter(|m| m.port == port)
        .flat_map(|m| m.map.keys().map(String::as_str))
        .collect()
}

/// The default bindings for `device` on `port`. Among the maps for that
/// device and port, one whose `when` holds is preferred over the
/// unconditional one. Empty when the core has none for that device.
pub fn default_map(
    core: &CoreDefinition,
    device: InputDevice,
    port: u8,
    effective: &BTreeMap<String, Value>,
) -> BTreeMap<String, String> {
    let candidates = core.default_mappings.iter().filter(|m| m.device == device && m.port == port);
    candidates
        .clone()
        .find(|m| m.when.as_ref().is_some_and(|c| condition_holds(c, effective)))
        .or_else(|| candidates.clone().find(|m| m.when.is_none()))
        .map(|m| m.map.clone())
        .unwrap_or_default()
}

/// Validate `requested` bindings for `port` and keep the buttons that exist
/// (under the current options) and are mapped.
fn finalize_buttons(
    core: &CoreDefinition,
    device: InputDevice,
    port: u8,
    requested: BTreeMap<String, String>,
    is_active: &dyn Fn(&str) -> bool,
) -> Result<BTreeMap<String, String>, VcError> {
    let known = known_buttons(core, port);
    let mut buttons = BTreeMap::new();
    for (button, physical) in requested {
        if !known.contains(button.as_str()) {
            return Err(VcError::InvalidMapping(format!(
                "'{button}' is not a button of {} controller {port}",
                core.system
            )));
        }
        if physical.is_empty() || !is_active(&button) {
            continue;
        }
        if !device.physical_prefixes().iter().any(|p| physical.starts_with(p)) {
            return Err(VcError::InvalidMapping(format!(
                "'{physical}' is not an input of the {} (expected '{}…')",
                device.as_str(),
                device.physical_prefixes().join("…' or '")
            )));
        }
        buttons.insert(button, physical);
    }
    Ok(buttons)
}

/// One enabled physical controller and the user's bindings for it.
#[derive(Debug, Clone)]
pub struct DeviceSetup {
    pub device: InputDevice,
    /// `None` uses the core's registry defaults for this device. `Some` is
    /// taken as-is: an empty map means the user unmapped everything.
    pub mapping: Option<InputMapping>,
}

/// The bindings of one device.
fn device_config(
    core: &CoreDefinition,
    setup: &DeviceSetup,
    effective: &BTreeMap<String, Value>,
) -> Result<DeviceConfig, VcError> {
    let device = setup.device;
    let is_active = |button: &str| {
        core.button_requires
            .get(button)
            .map_or(true, |cond| condition_holds(cond, effective))
    };

    let requested_p1 = match &setup.mapping {
        Some(m) => m.buttons.clone(),
        None => default_map(core, device, 1, effective),
    };
    let buttons = finalize_buttons(core, device, 1, requested_p1, &is_active)?;

    let mut ports = BTreeMap::new();
    for cp in &core.controller_ports {
        if !cp.when.as_ref().map_or(true, |c| condition_holds(c, effective)) {
            continue;
        }
        let requested = match setup.mapping.as_ref().and_then(|m| m.ports.get(&cp.port)) {
            Some(m) => m.clone(),
            None => default_map(core, device, cp.port, effective),
        };
        let buttons = finalize_buttons(core, device, cp.port, requested, &is_active)?;
        ports.insert(cp.port.to_string(), PortConfig { buttons });
    }
    Ok(DeviceConfig { buttons, ports })
}

/// Assemble the config file for `core` with the given controllers enabled.
///
/// - `devices`: at least one, each at most once. Every core accepts every
///   controller: a controller with fewer buttons than the system has just
///   leaves some console buttons unmapped, which is up to the user (many
///   games don't use them all).
/// - Per device, `mapping` holds the bindings for each emulated controller.
///   Extra ports missing from `mapping.ports` fall back to defaults; ports
///   that don't exist under the current options are ignored. Buttons that
///   don't exist under the current options are dropped; empty ids mean
///   "unmapped".
/// - `values`: the user's option values (option id -> value); anything unset
///   or inapplicable takes its default.
pub fn build_core_config(
    core: &CoreDefinition,
    devices: &[DeviceSetup],
    values: &BTreeMap<String, Value>,
) -> Result<CoreConfig, VcError> {
    if devices.is_empty() {
        return Err(VcError::InvalidMapping("select at least one controller".into()));
    }
    let mut seen = std::collections::HashSet::new();
    if let Some(dup) = devices.iter().find(|d| !seen.insert(d.device)) {
        return Err(VcError::InvalidMapping(format!("{} selected twice", dup.device.as_str())));
    }

    let enabled: Vec<InputDevice> = devices.iter().map(|d| d.device).collect();
    let resolved = resolve_options(core, &enabled, values)?;
    let effective: BTreeMap<String, Value> = resolved
        .iter()
        .map(|r| (r.option.id.clone(), r.value.clone()))
        .collect();

    let mut device_configs = BTreeMap::new();
    for setup in devices {
        device_configs.insert(setup.device.as_str().to_string(), device_config(core, setup, &effective)?);
    }

    let mut config = CoreConfig {
        format: FORMAT,
        version: FORMAT_VERSION,
        core: CoreInfo { id: core.id.clone(), system: core.system.clone() },
        input: InputSection { devices: device_configs, settings: Section::new() },
        video: Section::new(),
        audio: Section::new(),
        system: Section::new(),
        save: Section::new(),
        accessories: Section::new(),
        extra: Section::new(),
    };

    for r in resolved {
        let key = r.option.config_key();
        // Keys were validated when the registry loaded.
        let (section, name) = key.split_once('.').expect("validated key");
        let target = match section {
            "input" => &mut config.input.settings,
            "video" => &mut config.video,
            "audio" => &mut config.audio,
            "system" => &mut config.system,
            "save" => &mut config.save,
            "accessories" => &mut config.accessories,
            _ => &mut config.extra,
        };
        target.insert(name.to_string(), r.value);
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{find_core, load_registry};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    use InputDevice::{ClassicController as Classic, Gamecube, WiimoteNunchuk as Nunchuk, WiimoteSideways as Sideways};

    fn repo(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
    }

    fn registry() -> Vec<CoreDefinition> {
        load_registry(&repo("cores/registry.json")).expect("cores/registry.json should load and validate")
    }

    fn vals(v: Value) -> BTreeMap<String, Value> {
        serde_json::from_value(v).unwrap()
    }

    fn setup(device: InputDevice) -> DeviceSetup {
        DeviceSetup { device, mapping: None }
    }

    fn with(device: InputDevice, buttons: &[(&str, &str)]) -> DeviceSetup {
        DeviceSetup {
            device,
            mapping: Some(InputMapping {
                buttons: buttons.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
                ports: BTreeMap::new(),
            }),
        }
    }

    /// Build with a single controller, default bindings.
    fn one(core: &CoreDefinition, device: InputDevice, values: Value) -> CoreConfig {
        build_core_config(core, &[setup(device)], &vals(values)).unwrap()
    }

    fn dev(c: &CoreConfig, d: InputDevice) -> &DeviceConfig {
        &c.input.devices[d.as_str()]
    }

    fn buttons(c: &CoreConfig, d: InputDevice) -> Vec<&str> {
        dev(c, d).buttons.keys().map(String::as_str).collect()
    }

    #[test]
    fn every_shipped_core_builds_a_well_formed_config() {
        for core in registry() {
            for m in &core.default_mappings {
                let c = one(&core, m.device, json!({}));
                let parsed: Value = serde_json::from_str(&c.to_json()).unwrap();
                assert_eq!(parsed["format"], FORMAT);
                assert_eq!(parsed["version"], FORMAT_VERSION);
                assert_eq!(parsed["core"]["id"], core.id.as_str());
                let d = &parsed["input"]["devices"][m.device.as_str()];
                assert!(d["buttons"].is_object(), "{}: device entry needs buttons", core.id);
                assert!(d["ports"].is_object(), "{}: device entry must always have ports", core.id);
                for section in SECTIONS.iter().chain([&EXTRA]) {
                    assert!(parsed[*section].is_object(), "{}: missing section {section}", core.id);
                }
                // every option lands in the file
                for opt in &core.options {
                    let key = opt.config_key();
                    let (s, n) = key.split_once('.').unwrap();
                    assert!(!parsed[s][n].is_null(), "{}: {key} missing", core.id);
                }
            }
        }
    }

    #[test]
    fn every_core_accepts_every_controller() {
        // No controller is refused for having too few buttons: that's the
        // user's call, since many games don't use every button.
        for core in registry() {
            for device in InputDevice::ALL {
                let c = build_core_config(&core, &[setup(device)], &BTreeMap::new())
                    .unwrap_or_else(|e| panic!("{} / {}: {e}", core.id, device.as_str()));
                assert!(c.input.devices.contains_key(device.as_str()));
            }
        }
    }

    #[test]
    fn several_controllers_can_be_enabled_at_once_each_with_its_own_bindings() {
        let reg = registry();
        let nes = find_core(&reg, "nes").unwrap();
        let c = build_core_config(
            nes,
            &[
                setup(Classic),
                setup(Sideways),
                with(Nunchuk, &[("A", "wiimote_a"), ("B", "nunchuk_z"), ("Up", "nunchuk_stick_up")]),
            ],
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(c.input.devices.len(), 3);
        assert_eq!(dev(&c, Classic).buttons["A"], "classic_a");
        assert_eq!(dev(&c, Sideways).buttons["A"], "wiimote_2");
        // The nunchuk device accepts both wiimote_* and nunchuk_* inputs.
        assert_eq!(dev(&c, Nunchuk).buttons["B"], "nunchuk_z");
        assert_eq!(dev(&c, Nunchuk).buttons["A"], "wiimote_a");
        let parsed: Value = serde_json::from_str(&c.to_json()).unwrap();
        assert!(parsed["input"]["devices"]["wiimote_nunchuk"]["buttons"]["B"] == "nunchuk_z");
    }

    #[test]
    fn a_controller_with_few_buttons_is_allowed_and_can_be_bound_by_hand() {
        let reg = registry();
        let snes = find_core(&reg, "snes").unwrap();
        // No registry defaults for the sideways Wiimote on SNES: starts empty, no error.
        let c = build_core_config(snes, &[setup(Sideways)], &BTreeMap::new()).unwrap();
        assert!(dev(&c, Sideways).buttons.is_empty());
        // The user maps the buttons the game actually needs.
        let c = build_core_config(snes, &[with(Sideways, &[("A", "wiimote_2"), ("B", "wiimote_1"), ("Start", "wiimote_plus")])], &BTreeMap::new()).unwrap();
        assert_eq!(buttons(&c, Sideways), vec!["A", "B", "Start"]);
    }

    #[test]
    fn at_least_one_controller_is_required_and_none_twice() {
        let reg = registry();
        let nes = find_core(&reg, "nes").unwrap();
        assert!(build_core_config(nes, &[], &BTreeMap::new()).is_err());
        assert!(build_core_config(nes, &[setup(Classic), setup(Classic)], &BTreeMap::new()).is_err());
    }

    #[test]
    fn shared_concepts_share_keys_across_cores() {
        let reg = registry();
        let cfg = |id: &str| one(find_core(&reg, id).unwrap(), Classic, json!({}));
        // Same key, different cores.
        assert!(cfg("nes").video.contains_key("crop_overscan") && cfg("snes").video.contains_key("crop_overscan"));
        assert!(cfg("sms").system.contains_key("region") && cfg("genesis").system.contains_key("region"));
        assert!(cfg("tg16").accessories.contains_key("multitap") && cfg("tgcd").accessories.contains_key("multitap"));
        // Core-specific settings sit under extra.
        assert!(cfg("nds").extra.contains_key("slot2"));
        assert!(!cfg("nds").accessories.contains_key("slot2"));
    }

    #[test]
    fn options_can_add_buttons() {
        let reg = registry();
        let genesis = find_core(&reg, "genesis").unwrap();
        let three = one(genesis, Classic, json!({}));
        assert!(!buttons(&three, Classic).contains(&"X") && buttons(&three, Classic).contains(&"A"));

        let six = one(genesis, Classic, json!({ "six_button": true }));
        for b in ["X", "Y", "Z", "Mode"] {
            assert!(buttons(&six, Classic).contains(&b), "six-button pad should add {b}");
        }
        assert_eq!(six.input.settings["six_button"], json!(true));
    }

    #[test]
    fn dropped_buttons_are_removed_even_if_the_client_sends_them() {
        let reg = registry();
        let genesis = find_core(&reg, "genesis").unwrap();
        let setup = with(Classic, &[("A", "classic_a"), ("X", "classic_x"), ("Mode", "classic_minus")]);
        let c = build_core_config(genesis, &[setup], &BTreeMap::new()).unwrap();
        assert_eq!(buttons(&c, Classic), vec!["A"]);
    }

    fn n64(reg: &[CoreDefinition]) -> &CoreDefinition {
        find_core(reg, "n64").unwrap()
    }

    #[test]
    fn single_controller_has_no_extra_ports() {
        let reg = registry();
        let c = one(n64(&reg), Classic, json!({}));
        assert!(dev(&c, Classic).ports.is_empty());
        assert_eq!(c.input.settings["dual_controller"], json!(false));
        // Default single layout: C buttons on the right stick.
        assert_eq!(dev(&c, Classic).buttons["C-Up"], "classic_rstick_up");
    }

    #[test]
    fn dual_controller_splits_one_pad_across_two_emulated_controllers() {
        let reg = registry();
        let c = one(n64(&reg), Classic, json!({ "dual_controller": true }));
        let d = dev(&c, Classic);
        let p2 = &d.ports["2"].buttons;
        // Left stick + left-hand inputs -> controller 1, right stick + right-hand -> controller 2.
        assert_eq!(d.buttons["Stick"], "classic_lstick");
        assert_eq!(d.buttons["Z"], "classic_zl");
        assert_eq!(p2["Stick"], "classic_rstick");
        assert_eq!(p2["Z"], "classic_zr");
        assert_eq!(c.input.settings["dual_controller"], json!(true));
        // The layout swap means controller 1 no longer uses the right stick for C buttons.
        assert!(!d.buttons.contains_key("C-Up"));
        // No physical input is bound twice across the two controllers.
        let mut all: Vec<&String> = d.buttons.values().chain(p2.values()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "default dual layout must not double-bind an input");
    }

    #[test]
    fn dual_mode_applies_per_controller_when_several_are_enabled() {
        let reg = registry();
        let c = build_core_config(n64(&reg), &[setup(Classic), setup(Gamecube)], &vals(json!({ "dual_controller": true }))).unwrap();
        // Classic has registry defaults for both emulated controllers...
        assert_eq!(dev(&c, Classic).ports["2"].buttons["Stick"], "classic_rstick");
        // ...the GameCube pad has none of its own, so its bindings start empty but port 2 still exists.
        assert!(dev(&c, Gamecube).ports.contains_key("2"));
    }

    #[test]
    fn extra_port_bindings_are_validated_and_ignored_when_the_port_is_off() {
        let reg = registry();
        let core = n64(&reg);
        let with_p2 = |p: &[(&str, &str)]| DeviceSetup {
            device: Classic,
            mapping: Some(InputMapping {
                buttons: BTreeMap::new(),
                ports: [(2u8, p.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect())].into(),
            }),
        };

        // Port 2 doesn't exist in single mode: its bindings are ignored.
        let c = build_core_config(core, &[with_p2(&[("Stick", "classic_rstick")])], &BTreeMap::new()).unwrap();
        assert!(dev(&c, Classic).ports.is_empty());

        let dual = vals(json!({ "dual_controller": true }));
        // A custom binding on port 2 is used...
        let c = build_core_config(core, &[with_p2(&[("Stick", "classic_lstick")])], &dual).unwrap();
        assert_eq!(dev(&c, Classic).ports["2"].buttons["Stick"], "classic_lstick");
        // ...but must be a real button on the right device.
        assert!(build_core_config(core, &[with_p2(&[("Stick", "gc_lstick")])], &dual).is_err());
        assert!(build_core_config(core, &[with_p2(&[("Turbo", "classic_a")])], &dual).is_err());
    }

    #[test]
    fn bad_port_definitions_are_rejected() {
        let mut core: CoreDefinition = serde_json::from_value(json!({
            "system": "T", "id": "t", "core_source": { "kind": "bundled", "dol_path": "x.dol" },
            "valid_extensions": [], "header_size": null, "title_id_prefix": [0,1,0,1], "ios_version": 58,
            "options": [ { "id": "dual", "label": "Dual", "kind": "toggle", "default": false } ],
            "default_mappings": [
                { "device": "classic_controller", "map": { "A": "classic_a" } },
                { "device": "classic_controller", "port": 2, "map": { "A": "classic_a" } }
            ],
            "controller_ports": [ { "port": 2, "label": "Two", "when": { "option": "dual", "equals": true } } ]
        })).unwrap();
        assert!(crate::options::validate_definitions(&core).is_ok());
        // undeclared port used by a map
        let mut bad = core.clone();
        bad.controller_ports.clear();
        assert!(crate::options::validate_definitions(&bad).is_err());
        // declared port with no map
        let mut bad = core.clone();
        bad.default_mappings.pop();
        assert!(crate::options::validate_definitions(&bad).is_err());
        // condition on unknown option
        core.controller_ports[0].when.as_mut().unwrap().option = "nope".into();
        assert!(crate::options::validate_definitions(&core).is_err());
    }

    #[test]
    fn inapplicable_option_values_do_not_leak() {
        let reg = registry();
        let n64 = n64(&reg);
        let v = json!({ "pak": "rumble_pak", "rumble_strength": 80 });
        // Rumble strength only applies to controllers that can rumble.
        let classic = one(n64, Classic, v.clone());
        assert_eq!(classic.input.settings["rumble_strength"], json!(50));
        let gc = one(n64, Gamecube, v.clone());
        assert_eq!(gc.input.settings["rumble_strength"], json!(80));
        // With several controllers it applies if any of them can.
        let both = build_core_config(n64, &[setup(Classic), setup(Gamecube)], &vals(v)).unwrap();
        assert_eq!(both.input.settings["rumble_strength"], json!(80));
    }

    #[test]
    fn bad_mappings_are_rejected() {
        let reg = registry();
        let nes = find_core(&reg, "nes").unwrap();
        let build = |s: DeviceSetup| build_core_config(nes, &[s], &BTreeMap::new());
        // physical input from the wrong controller
        assert!(build(with(Classic, &[("A", "gc_a")])).is_err());
        // a wiimote input is not a classic-controller input, and vice versa
        assert!(build(with(Sideways, &[("A", "classic_a")])).is_err());
        assert!(build(with(Nunchuk, &[("A", "classic_a")])).is_err());
        // button the system doesn't have
        assert!(build(with(Classic, &[("Turbo", "classic_a")])).is_err());
    }

    /// Example files double as documentation for whoever writes the core-side
    /// reader. Regenerate with `UPDATE_EXAMPLES=1 cargo test -p vc-core`.
    #[test]
    fn example_files_are_current() {
        struct Example {
            file: String,
            core: String,
            devices: Vec<DeviceSetup>,
            options: Value,
        }
        let ex = |file: &str, core: &str, devices: Vec<DeviceSetup>, options: Value| Example {
            file: file.to_string(),
            core: core.to_string(),
            devices,
            options,
        };
        let reg = registry();
        let mut examples: Vec<Example> = reg
            .iter()
            .map(|c| ex(&format!("{}.json", c.id), &c.id, vec![setup(Classic)], json!({})))
            .collect();
        examples.push(ex(
            "n64-gamecube-rumble.json", "n64",
            vec![with(Gamecube, &[("A", "gc_a"), ("B", "gc_b"), ("Z", "gc_z"), ("Start", "gc_start"), ("Stick", "gc_lstick"), ("C-Up", "gc_cstick_up")])],
            json!({ "expansion_pak": true, "pak": "rumble_pak", "rumble_strength": 80 }),
        ));
        examples.push(ex("n64-dual-controller.json", "n64", vec![setup(Classic)], json!({ "dual_controller": true, "pak2": "controller_pak" })));
        examples.push(ex("genesis-six-button.json", "genesis", vec![setup(Classic)], json!({ "six_button": true })));
        examples.push(ex("nds-memory-expansion.json", "nds", vec![setup(Classic)], json!({ "slot2": "memory_expansion", "layout": "side_by_side" })));
        examples.push(ex(
            "nes-multiple-controllers.json", "nes",
            vec![
                setup(Classic),
                setup(Sideways),
                with(Nunchuk, &[("A", "wiimote_a"), ("B", "nunchuk_z"), ("Start", "wiimote_plus"), ("Select", "wiimote_minus"), ("Up", "nunchuk_stick_up"), ("Down", "nunchuk_stick_down"), ("Left", "nunchuk_stick_left"), ("Right", "nunchuk_stick_right")]),
            ],
            json!({}),
        ));

        let dir = repo("docs/config-examples");
        let update = std::env::var_os("UPDATE_EXAMPLES").is_some();
        if update {
            std::fs::create_dir_all(&dir).unwrap();
        }
        for ex in examples {
            let core = find_core(&reg, &ex.core).unwrap();
            let json = build_core_config(core, &ex.devices, &vals(ex.options)).unwrap().to_json();
            let path = dir.join(&ex.file);
            if update {
                std::fs::write(&path, &json).unwrap();
            } else {
                let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
                assert_eq!(on_disk, json, "{} is stale; run `UPDATE_EXAMPLES=1 cargo test -p vc-core`", ex.file);
            }
        }
    }
}
