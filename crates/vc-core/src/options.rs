//! Per-core options: validation of registry definitions, and resolving the
//! user's chosen values into effective values for the core config file
//! (see `coreconfig.rs` for where they are written).
//!
//! Semantics (mirrored by the frontend in `coreOptionsData.js`):
//! - An option is *applicable* when the selected controller is in its
//!   `devices` list (or the list is empty) **and** its `visible_when`
//!   condition holds, evaluated against the *effective* value of an earlier
//!   option.
//! - Inapplicable options always resolve to their `default`, whatever the
//!   client sent, so a stale value for a hidden option can never leak into
//!   the config file.

use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::coreconfig::validate_option_key;
use crate::error::VcError;
use crate::registry::{CoreDefinition, CoreOption, InputDevice, OptionCondition, OptionKind};

fn bad_def(core: &CoreDefinition, msg: impl std::fmt::Display) -> VcError {
    VcError::InvalidRegistry(format!("core '{}': {msg}", core.id))
}

fn bad_val(opt: &CoreOption, msg: impl std::fmt::Display) -> VcError {
    VcError::InvalidOption(format!("'{}': {msg}", opt.id))
}

/// JSON equality that treats `1` and `1.0` as the same number.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) if a.is_number() && b.is_number() => x == y,
        _ => a == b,
    }
}

/// Is `value` acceptable for this option's kind (type, choice list, range)?
fn check_value(opt: &CoreOption, value: &Value) -> Result<(), String> {
    match opt.kind {
        OptionKind::Toggle => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err("expected true or false".into())
            }
        }
        OptionKind::Select => match value.as_str() {
            Some(s) if opt.choices.iter().any(|c| c.value == s) => Ok(()),
            Some(s) => Err(format!("'{s}' is not one of the available choices")),
            None => Err("expected a string choice".into()),
        },
        OptionKind::Number => {
            let n = value.as_f64().filter(|_| value.is_number()).ok_or("expected a number")?;
            let (min, max) = (opt.min.unwrap_or(0.0), opt.max.unwrap_or(255.0));
            if n.is_finite() && n >= min && n <= max {
                Ok(())
            } else {
                Err(format!("{n} is outside {min}..={max}"))
            }
        }
    }
}

/// Canonical JSON form: whole numbers are written as integers (`30`, not `30.0`).
fn normalize(opt: &CoreOption, value: Value) -> Value {
    if opt.kind == OptionKind::Number {
        if let Some(n) = value.as_f64() {
            if n.fract() == 0.0 {
                return Value::from(n as i64);
            }
        }
    }
    value
}

/// Check a core's option and button definitions are internally consistent.
/// Run when the registry loads so a typo fails loudly at startup rather than
/// mid-build.
pub fn validate_definitions(core: &CoreDefinition) -> Result<(), VcError> {
    let mut ids: HashSet<&str> = HashSet::new();
    let mut keys: HashSet<String> = HashSet::new();

    for opt in &core.options {
        if opt.id.is_empty() || opt.id.contains(char::is_whitespace) {
            return Err(bad_def(core, format!("option id '{}' must be non-empty with no spaces", opt.id)));
        }
        if !ids.insert(opt.id.as_str()) {
            return Err(bad_def(core, format!("duplicate option id '{}'", opt.id)));
        }

        let key = opt.config_key();
        validate_option_key(opt, &key).map_err(|e| bad_def(core, format!("option '{}': {e}", opt.id)))?;
        if !keys.insert(key.clone()) {
            return Err(bad_def(core, format!("option '{}' reuses config key '{key}'", opt.id)));
        }

        match opt.kind {
            OptionKind::Toggle => {}
            OptionKind::Select => {
                if opt.choices.is_empty() || opt.choices.len() > 256 {
                    return Err(bad_def(core, format!("select option '{}' needs 1..=256 choices", opt.id)));
                }
                let mut seen = HashSet::new();
                if !opt.choices.iter().all(|c| seen.insert(c.value.as_str())) {
                    return Err(bad_def(core, format!("select option '{}' has duplicate choice values", opt.id)));
                }
            }
            OptionKind::Number => {
                let (Some(min), Some(max)) = (opt.min, opt.max) else {
                    return Err(bad_def(core, format!("number option '{}' needs min and max", opt.id)));
                };
                if !(0.0..=255.0).contains(&min) || !(0.0..=255.0).contains(&max) || min > max {
                    return Err(bad_def(core, format!("number option '{}' needs 0 <= min <= max <= 255", opt.id)));
                }
                if opt.step.is_some_and(|s| s <= 0.0) {
                    return Err(bad_def(core, format!("number option '{}' step must be positive", opt.id)));
                }
            }
        }

        check_value(opt, &opt.default)
            .map_err(|e| bad_def(core, format!("option '{}' default: {e}", opt.id)))?;

        if let Some(cond) = &opt.visible_when {
            // Conditions may only look backwards, so evaluation order is well defined.
            let Some(dep) = core.options.iter().take_while(|o| o.id != opt.id).find(|o| o.id == cond.option) else {
                return Err(bad_def(core, format!(
                    "option '{}' visible_when refers to '{}', which must be declared earlier",
                    opt.id, cond.option
                )));
            };
            check_value(dep, &cond.equals)
                .map_err(|e| bad_def(core, format!("option '{}' visible_when value: {e}", opt.id)))?;
        }
    }

    validate_ports(core)?;

    // Buttons that only exist while an option has a given value.
    let known_buttons: HashSet<&str> = core
        .default_mappings
        .iter()
        .flat_map(|m| m.map.keys().map(String::as_str))
        .collect();
    for (button, cond) in &core.button_requires {
        if !known_buttons.contains(button.as_str()) {
            return Err(bad_def(core, format!("button_requires: '{button}' is not a button in default_mappings")));
        }
        let Some(dep) = core.options.iter().find(|o| o.id == cond.option) else {
            return Err(bad_def(core, format!("button_requires: '{button}' refers to unknown option '{}'", cond.option)));
        };
        check_value(dep, &cond.equals)
            .map_err(|e| bad_def(core, format!("button_requires: '{button}' value: {e}")))?;
    }
    Ok(())
}

fn check_condition(core: &CoreDefinition, cond: &OptionCondition, what: &str) -> Result<(), VcError> {
    let Some(dep) = core.options.iter().find(|o| o.id == cond.option) else {
        return Err(bad_def(core, format!("{what} refers to unknown option '{}'", cond.option)));
    };
    check_value(dep, &cond.equals).map_err(|e| bad_def(core, format!("{what}: {e}")))
}

/// Emulated controller ports and the (possibly conditional) default maps for them.
fn validate_ports(core: &CoreDefinition) -> Result<(), VcError> {
    let mut declared: HashSet<u8> = HashSet::new();
    for p in &core.controller_ports {
        if !(2..=4).contains(&p.port) {
            return Err(bad_def(core, format!("controller_ports: port {} must be 2..=4 (port 1 always exists)", p.port)));
        }
        if !declared.insert(p.port) {
            return Err(bad_def(core, format!("controller_ports: port {} declared twice", p.port)));
        }
        if let Some(cond) = &p.when {
            check_condition(core, cond, &format!("controller port {}", p.port))?;
        }
    }

    let mut seen: HashSet<(InputDevice, u8, String)> = HashSet::new();
    for m in &core.default_mappings {
        if !(1..=4).contains(&m.port) {
            return Err(bad_def(core, format!("default_mappings: port {} must be 1..=4", m.port)));
        }
        if m.port > 1 && !declared.contains(&m.port) {
            return Err(bad_def(core, format!("default_mappings: port {} is not declared in controller_ports", m.port)));
        }
        let variant = match &m.when {
            Some(c) => {
                check_condition(core, c, "default_mappings 'when'")?;
                format!("{}={}", c.option, c.equals)
            }
            None => String::new(),
        };
        if !seen.insert((m.device, m.port, variant)) {
            return Err(bad_def(core, format!(
                "default_mappings: duplicate map for {} on port {} (same condition)",
                m.device.as_str(), m.port
            )));
        }
    }
    for port in &declared {
        if !core.default_mappings.iter().any(|m| m.port == *port) {
            return Err(bad_def(core, format!("controller_ports: port {port} has no default_mappings")));
        }
    }
    Ok(())
}

fn is_applicable(opt: &CoreOption, device: InputDevice, effective: &BTreeMap<String, Value>) -> bool {
    if !opt.devices.is_empty() && !opt.devices.contains(&device) {
        return false;
    }
    opt.visible_when.as_ref().map_or(true, |c| condition_holds(c, effective))
}

/// Does `cond` hold given the effective option values?
pub fn condition_holds(cond: &OptionCondition, effective: &BTreeMap<String, Value>) -> bool {
    effective.get(&cond.option).is_some_and(|v| values_equal(v, &cond.equals))
}

/// One option with the value that will actually be written.
#[derive(Debug, Clone)]
pub struct ResolvedOption<'a> {
    pub option: &'a CoreOption,
    /// The user's value if the option applies, otherwise its default.
    pub value: Value,
    /// False when the option's condition or device filter excluded it.
    pub applicable: bool,
}

/// Validate the user's chosen `values` (option id -> JSON value) for `core`
/// with `device` selected. Options the user didn't set, and options that
/// don't apply, resolve to their default. Returned in registry order.
pub fn resolve_options<'a>(
    core: &'a CoreDefinition,
    device: InputDevice,
    values: &BTreeMap<String, Value>,
) -> Result<Vec<ResolvedOption<'a>>, VcError> {
    if let Some(unknown) = values.keys().find(|k| !core.options.iter().any(|o| &o.id == *k)) {
        return Err(VcError::InvalidOption(format!(
            "'{unknown}' is not an option of {}",
            core.system
        )));
    }

    let mut effective: BTreeMap<String, Value> = BTreeMap::new();
    let mut out = Vec::with_capacity(core.options.len());

    for opt in &core.options {
        if let Some(v) = values.get(&opt.id) {
            check_value(opt, v).map_err(|e| bad_val(opt, e))?;
        }
        let applicable = is_applicable(opt, device, &effective);
        let value = match values.get(&opt.id) {
            Some(v) if applicable => normalize(opt, v.clone()),
            _ => normalize(opt, opt.default.clone()),
        };
        effective.insert(opt.id.clone(), value.clone());
        out.push(ResolvedOption { option: opt, value, applicable });
    }
    Ok(out)
}

/// Parse repeated `ID=VALUE` strings (as given to `vc-cli --opt`) into typed
/// option values, using each option's kind to decide how to read the text.
pub fn parse_option_args(
    core: &CoreDefinition,
    args: &[String],
) -> Result<BTreeMap<String, Value>, VcError> {
    let mut out = BTreeMap::new();
    for arg in args {
        let (id, raw) = arg
            .split_once('=')
            .ok_or_else(|| VcError::InvalidOption(format!("expected ID=VALUE, got '{arg}'")))?;
        let opt = core
            .options
            .iter()
            .find(|o| o.id == id)
            .ok_or_else(|| VcError::InvalidOption(format!("'{id}' is not an option of {}", core.system)))?;
        let value = match opt.kind {
            OptionKind::Toggle => match raw {
                "true" | "on" | "1" => Value::Bool(true),
                "false" | "off" | "0" => Value::Bool(false),
                _ => return Err(bad_val(opt, "expected true or false")),
            },
            OptionKind::Select => Value::String(raw.to_string()),
            OptionKind::Number => raw
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .ok_or_else(|| bad_val(opt, "expected a number"))?,
        };
        out.insert(id.to_string(), value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn core(options: Value) -> CoreDefinition {
        serde_json::from_value(json!({
            "system": "Test", "id": "test",
            "core_source": { "kind": "bundled", "dol_path": "x.dol" },
            "valid_extensions": ["bin"], "header_size": null,
            "default_mappings": [], "title_id_prefix": [0, 1, 0, 1], "ios_version": 58,
            "options": options,
        }))
        .unwrap()
    }

    fn sample() -> CoreDefinition {
        core(json!([
            { "id": "pak", "label": "Pak", "kind": "select", "default": "none",
              "choices": [ {"value":"none","label":"None"}, {"value":"rumble","label":"Rumble"} ] },
            { "id": "strength", "label": "Strength", "kind": "number", "default": 50,
              "key": "input.rumble_strength", "min": 0, "max": 100, "step": 10,
              "devices": ["gamecube"], "visible_when": { "option": "pak", "equals": "rumble" } },
            { "id": "ram", "label": "RAM", "kind": "toggle", "default": false, "key": "accessories.expansion_ram" }
        ]))
    }

    fn vals(v: Value) -> BTreeMap<String, Value> {
        serde_json::from_value(v).unwrap()
    }

    fn resolved(c: &CoreDefinition, d: InputDevice, v: Value) -> Vec<Value> {
        resolve_options(c, d, &vals(v)).unwrap().into_iter().map(|r| r.value).collect()
    }

    #[test]
    fn defaults_are_used_when_nothing_is_chosen() {
        let out = resolved(&sample(), InputDevice::ClassicController, json!({}));
        assert_eq!(out, vec![json!("none"), json!(50), json!(false)]);
    }

    #[test]
    fn chosen_values_are_used_and_conditions_apply() {
        let out = resolved(&sample(), InputDevice::Gamecube, json!({ "pak": "rumble", "strength": 80, "ram": true }));
        assert_eq!(out, vec![json!("rumble"), json!(80), json!(true)]);
    }

    #[test]
    fn device_filter_and_hidden_options_fall_back_to_default() {
        let c = sample();
        // Classic Controller can't rumble: strength is inapplicable -> default 50.
        let out = resolve_options(&c, InputDevice::ClassicController, &vals(json!({ "pak": "rumble", "strength": 80 }))).unwrap();
        assert_eq!(out[1].value, json!(50));
        assert!(!out[1].applicable);
        // Pak not set to rumble: strength hidden even on GameCube.
        assert_eq!(resolved(&c, InputDevice::Gamecube, json!({ "strength": 80 }))[1], json!(50));
    }

    #[test]
    fn whole_numbers_are_written_as_integers() {
        let out = resolved(&sample(), InputDevice::Gamecube, json!({ "pak": "rumble", "strength": 30.0 }));
        assert_eq!(out[1].to_string(), "30");
    }

    #[test]
    fn bad_values_are_rejected() {
        let c = sample();
        for bad in [json!({ "pak": "nope" }), json!({ "ram": "yes" }), json!({ "strength": 101 }), json!({ "ghost": 1 })] {
            assert!(resolve_options(&c, InputDevice::Gamecube, &vals(bad)).is_err());
        }
    }

    #[test]
    fn cli_args_are_parsed_by_option_kind() {
        let c = sample();
        let parsed = parse_option_args(&c, &["pak=rumble".into(), "strength=30".into(), "ram=on".into()]).unwrap();
        assert_eq!(parsed["pak"], json!("rumble"));
        assert_eq!(parsed["strength"], json!(30.0));
        assert_eq!(parsed["ram"], json!(true));
        assert!(parse_option_args(&c, &["ram=maybe".into()]).is_err());
        assert!(parse_option_args(&c, &["nope=1".into()]).is_err());
        assert!(parse_option_args(&c, &["pak".into()]).is_err());
    }

    #[test]
    fn bad_definitions_are_rejected() {
        let toggle = |id: &str, extra: Value| {
            let mut o = json!({ "id": id, "label": id, "kind": "toggle", "default": false });
            o.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            o
        };
        // two options writing to the same key
        assert!(validate_definitions(&core(json!([toggle("a", json!({ "key": "extra.x" })), toggle("b", json!({ "key": "extra.x" }))]))).is_err());
        // default not among choices
        assert!(validate_definitions(&core(json!([
            { "id": "a", "label": "A", "kind": "select", "default": "z", "choices": [ {"value":"x","label":"X"} ] }
        ]))).is_err());
        // condition refers to a later option
        assert!(validate_definitions(&core(json!([
            toggle("a", json!({ "visible_when": { "option": "b", "equals": true } })),
            toggle("b", json!({}))
        ]))).is_err());
        // key outside the standard catalogue and not under extra.
        assert!(validate_definitions(&core(json!([toggle("a", json!({ "key": "video.my_thing" }))]))).is_err());
        // standard key with the wrong kind
        assert!(validate_definitions(&core(json!([toggle("a", json!({ "key": "video.timing" }))]))).is_err());
    }
}
