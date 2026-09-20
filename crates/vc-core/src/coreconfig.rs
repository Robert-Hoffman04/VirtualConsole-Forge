//! The unified core config file.
//!
//! Every emulator core reads the same JSON document, so the common
//! interface layer can be written once. Its layout is fixed:
//!
//! ```text
//! { "format": "vcforge-config", "version": 1,
//!   "core":        { "id", "system" },
//!   "input":       { "device", "buttons": {console button -> physical input},
//!                    "ports": {"2": {"buttons": {...}}}, ...settings },
//!   "video":       { ... },   "audio":  { ... },   "system":      { ... },
//!   "save":        { ... },   "accessories": { ... },
//!   "extra":       { ... } }  // settings only this core understands
//! ```
//!
//! `input.buttons` is emulated controller 1. When one physical controller
//! drives several emulated controllers (e.g. N64 dual-analog), the others are
//! listed under `input.ports`, keyed by port number (`{}` when there are none).
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
pub const FORMAT_VERSION: u32 = 1;

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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InputSection {
    /// `wiimote_sideways`, `classic_controller` or `gamecube`.
    pub device: &'static str,
    /// Console button -> physical input id (prefixed by the device:
    /// `wiimote_`, `classic_`, `gc_`). Only buttons that exist for this
    /// configuration are listed; unmapped buttons are omitted.
    pub buttons: BTreeMap<String, String>,
    /// Additional emulated controllers driven by this physical controller,
    /// keyed by port number ("2", "3", ...). Empty for single-controller setups.
    pub ports: BTreeMap<String, PortConfig>,
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
        if !physical.starts_with(device.physical_prefix()) {
            return Err(VcError::InvalidMapping(format!(
                "'{physical}' is not an input of the {} (expected '{}…')",
                device.as_str(),
                device.physical_prefix()
            )));
        }
        buttons.insert(button, physical);
    }
    Ok(buttons)
}

/// Assemble the config file for `core` with `device` selected.
///
/// - `mapping`: the user's bindings for each emulated controller. `None`
///   uses the core's defaults (for the layout the options select). Extra
///   ports missing from `mapping.ports` also fall back to defaults; ports
///   that don't exist under the current options are ignored. Buttons that
///   don't exist under the current options are dropped; empty ids mean
///   "unmapped".
/// - `values`: the user's option values (option id -> value); anything unset
///   or inapplicable takes its default.
pub fn build_core_config(
    core: &CoreDefinition,
    device: InputDevice,
    mapping: Option<&InputMapping>,
    values: &BTreeMap<String, Value>,
) -> Result<CoreConfig, VcError> {
    let resolved = resolve_options(core, device, values)?;
    let effective: BTreeMap<String, Value> = resolved
        .iter()
        .map(|r| (r.option.id.clone(), r.value.clone()))
        .collect();
    let is_active = |button: &str| {
        core.button_requires
            .get(button)
            .map_or(true, |cond| condition_holds(cond, &effective))
    };

    let requested_p1 = match mapping {
        Some(m) => m.buttons.clone(),
        None => default_map(core, device, 1, &effective),
    };
    let buttons = finalize_buttons(core, device, 1, requested_p1, &is_active)?;

    let mut ports = BTreeMap::new();
    for cp in &core.controller_ports {
        if !cp.when.as_ref().map_or(true, |c| condition_holds(c, &effective)) {
            continue;
        }
        let requested = match mapping.and_then(|m| m.ports.get(&cp.port)) {
            Some(m) => m.clone(),
            None => default_map(core, device, cp.port, &effective),
        };
        let buttons = finalize_buttons(core, device, cp.port, requested, &is_active)?;
        ports.insert(cp.port.to_string(), PortConfig { buttons });
    }

    let mut config = CoreConfig {
        format: FORMAT,
        version: FORMAT_VERSION,
        core: CoreInfo { id: core.id.clone(), system: core.system.clone() },
        input: InputSection { device: device.as_str(), buttons, ports, settings: Section::new() },
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

    fn repo(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
    }

    fn registry() -> Vec<CoreDefinition> {
        load_registry(&repo("cores/registry.json")).expect("cores/registry.json should load and validate")
    }

    fn vals(v: Value) -> BTreeMap<String, Value> {
        serde_json::from_value(v).unwrap()
    }

    fn buttons(c: &CoreConfig) -> Vec<&str> {
        c.input.buttons.keys().map(String::as_str).collect()
    }

    #[test]
    fn every_shipped_core_builds_a_well_formed_config() {
        for core in registry() {
            for m in &core.default_mappings {
                let c = build_core_config(&core, m.device, None, &BTreeMap::new())
                    .unwrap_or_else(|e| panic!("{} / {}: {e}", core.id, m.device.as_str()));
                let parsed: Value = serde_json::from_str(&c.to_json()).unwrap();
                assert_eq!(parsed["format"], FORMAT);
                assert_eq!(parsed["core"]["id"], core.id.as_str());
                assert_eq!(parsed["input"]["device"], m.device.as_str());
                assert!(parsed["input"]["ports"].is_object(), "{}: input.ports must always be present", core.id);
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
    fn shared_concepts_share_keys_across_cores() {
        let reg = registry();
        let get = |id: &str| find_core(&reg, id).unwrap();
        let cfg = |id: &str| build_core_config(get(id), InputDevice::ClassicController, None, &BTreeMap::new()).unwrap();
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
        let three = build_core_config(genesis, InputDevice::ClassicController, None, &BTreeMap::new()).unwrap();
        assert!(!buttons(&three).contains(&"X") && buttons(&three).contains(&"A"));

        let six = build_core_config(genesis, InputDevice::ClassicController, None, &vals(json!({ "six_button": true }))).unwrap();
        for b in ["X", "Y", "Z", "Mode"] {
            assert!(buttons(&six).contains(&b), "six-button pad should add {b}");
        }
        assert_eq!(six.input.settings["six_button"], json!(true));
    }

    #[test]
    fn dropped_buttons_are_removed_even_if_the_client_sends_them() {
        let reg = registry();
        let genesis = find_core(&reg, "genesis").unwrap();
        let mapping: BTreeMap<String, String> =
            [("A", "classic_a"), ("X", "classic_x"), ("Mode", "classic_minus")].map(|(a, b)| (a.to_string(), b.to_string())).into();
        let mapping = InputMapping { buttons: mapping, ports: BTreeMap::new() };
        let c = build_core_config(genesis, InputDevice::ClassicController, Some(&mapping), &BTreeMap::new()).unwrap();
        assert_eq!(buttons(&c), vec!["A"]);
    }

    fn n64(reg: &[CoreDefinition]) -> &CoreDefinition {
        find_core(reg, "n64").unwrap()
    }

    #[test]
    fn single_controller_has_no_extra_ports() {
        let reg = registry();
        let c = build_core_config(n64(&reg), InputDevice::ClassicController, None, &BTreeMap::new()).unwrap();
        assert!(c.input.ports.is_empty());
        assert_eq!(c.input.settings["dual_controller"], json!(false));
        // Default single layout: C buttons on the right stick.
        assert_eq!(c.input.buttons["C-Up"], "classic_rstick_up");
    }

    #[test]
    fn dual_controller_splits_one_pad_across_two_emulated_controllers() {
        let reg = registry();
        let c = build_core_config(n64(&reg), InputDevice::ClassicController, None, &vals(json!({ "dual_controller": true }))).unwrap();
        let p2 = &c.input.ports["2"].buttons;
        // Left stick + left-hand inputs -> controller 1, right stick + right-hand -> controller 2.
        assert_eq!(c.input.buttons["Stick"], "classic_lstick");
        assert_eq!(c.input.buttons["Z"], "classic_zl");
        assert_eq!(p2["Stick"], "classic_rstick");
        assert_eq!(p2["Z"], "classic_zr");
        assert_eq!(c.input.settings["dual_controller"], json!(true));
        // The layout swap means controller 1 no longer uses the right stick for C buttons.
        assert!(!c.input.buttons.contains_key("C-Up"));
        // No physical input is bound twice across the two controllers.
        let mut all: Vec<&String> = c.input.buttons.values().chain(p2.values()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "default dual layout must not double-bind an input");
    }

    #[test]
    fn extra_port_bindings_are_validated_and_ignored_when_the_port_is_off() {
        let reg = registry();
        let core = n64(&reg);
        let ports = |p: &[(&str, &str)]| -> BTreeMap<u8, BTreeMap<String, String>> {
            [(2u8, p.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect())].into()
        };
        let mapping = |p| InputMapping { buttons: BTreeMap::new(), ports: ports(p) };

        // Port 2 doesn't exist in single mode: its bindings are ignored.
        let c = build_core_config(core, InputDevice::ClassicController, Some(&mapping(&[("Stick", "classic_rstick")])), &BTreeMap::new()).unwrap();
        assert!(c.input.ports.is_empty());

        let dual = vals(json!({ "dual_controller": true }));
        // A custom binding on port 2 is used...
        let c = build_core_config(core, InputDevice::ClassicController, Some(&mapping(&[("Stick", "classic_lstick")])), &dual).unwrap();
        assert_eq!(c.input.ports["2"].buttons["Stick"], "classic_lstick");
        // ...but must be a real button on the right device.
        assert!(build_core_config(core, InputDevice::ClassicController, Some(&mapping(&[("Stick", "gc_lstick")])), &dual).is_err());
        assert!(build_core_config(core, InputDevice::ClassicController, Some(&mapping(&[("Turbo", "classic_a")])), &dual).is_err());
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
        let n64 = find_core(&reg, "n64").unwrap();
        let v = vals(json!({ "pak": "rumble_pak", "rumble_strength": 80 }));
        let classic = build_core_config(n64, InputDevice::ClassicController, None, &v).unwrap();
        assert_eq!(classic.input.settings["rumble_strength"], json!(50));
        let gc = build_core_config(n64, InputDevice::Gamecube, None, &v).unwrap();
        assert_eq!(gc.input.settings["rumble_strength"], json!(80));
    }

    #[test]
    fn bad_mappings_are_rejected() {
        let reg = registry();
        let nes = find_core(&reg, "nes").unwrap();
        let m = |a: &str, b: &str| InputMapping {
            buttons: [(a.to_string(), b.to_string())].into(),
            ports: BTreeMap::new(),
        };
        // physical input from the wrong controller
        assert!(build_core_config(nes, InputDevice::ClassicController, Some(&m("A", "gc_a")), &BTreeMap::new()).is_err());
        // button the system doesn't have
        assert!(build_core_config(nes, InputDevice::ClassicController, Some(&m("Turbo", "classic_a")), &BTreeMap::new()).is_err());
    }

    /// Example files double as documentation for whoever writes the core-side
    /// reader. Regenerate with `UPDATE_EXAMPLES=1 cargo test -p vc-core`.
    #[test]
    fn example_files_are_current() {
        struct Example {
            file: &'static str,
            core: &'static str,
            device: InputDevice,
            mapping: Option<Vec<(&'static str, &'static str)>>,
            options: Value,
        }
        let reg = registry();
        let mut examples: Vec<Example> = reg
            .iter()
            .map(|c| Example {
                file: Box::leak(format!("{}.json", c.id).into_boxed_str()),
                core: Box::leak(c.id.clone().into_boxed_str()),
                device: InputDevice::ClassicController,
                mapping: None,
                options: json!({}),
            })
            .collect();
        examples.push(Example {
            file: "n64-gamecube-rumble.json", core: "n64", device: InputDevice::Gamecube,
            mapping: Some(vec![("A", "gc_a"), ("B", "gc_b"), ("Z", "gc_z"), ("Start", "gc_start"), ("Stick", "gc_lstick"), ("C-Up", "gc_cstick_up")]),
            options: json!({ "expansion_pak": true, "pak": "rumble_pak", "rumble_strength": 80 }),
        });
        examples.push(Example {
            file: "n64-dual-controller.json", core: "n64", device: InputDevice::ClassicController,
            mapping: None, options: json!({ "dual_controller": true, "pak2": "controller_pak" }),
        });
        examples.push(Example {
            file: "genesis-six-button.json", core: "genesis", device: InputDevice::ClassicController,
            mapping: None, options: json!({ "six_button": true }),
        });
        examples.push(Example {
            file: "nds-memory-expansion.json", core: "nds", device: InputDevice::ClassicController,
            mapping: None, options: json!({ "slot2": "memory_expansion", "layout": "side_by_side" }),
        });

        let dir = repo("docs/config-examples");
        let update = std::env::var_os("UPDATE_EXAMPLES").is_some();
        if update {
            std::fs::create_dir_all(&dir).unwrap();
        }
        for ex in examples {
            let core = find_core(&reg, ex.core).unwrap();
            let mapping: Option<InputMapping> = ex.mapping.map(|m| InputMapping {
                buttons: m.into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
                ports: BTreeMap::new(),
            });
            let json = build_core_config(core, ex.device, mapping.as_ref(), &vals(ex.options)).unwrap().to_json();
            let path = dir.join(ex.file);
            if update {
                std::fs::write(&path, &json).unwrap();
            } else {
                let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
                assert_eq!(on_disk, json, "{} is stale; run `UPDATE_EXAMPLES=1 cargo test -p vc-core`", ex.file);
            }
        }
    }
}
