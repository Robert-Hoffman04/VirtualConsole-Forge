//! Tauri command handlers. Deliberately thin: the frontend never touches
//! binary format details, it just collects inputs (ROM path, core id,
//! button mapping, cover art) and displays progress. All real work goes
//! through vc_core::wad::build_wad.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vc_core::config::{InputDeviceId, SaveTarget, VcConfig};
use vc_core::donor::KeyProvider;
use vc_core::coreconfig::{build_core_config, CoreConfig, DeviceSetup, InputMapping};
use vc_core::registry::{
    find_core, load_registry, ButtonMapping, ControllerPort, CoreDefinition, CoreOption, CoreSource, InputDevice,
    OptionCondition,
};
use vc_core::wad::{build_wad, WadBuildRequest};

#[derive(Serialize)]
pub struct CoreSummary {
    id: String,
    system: String,
    /// File extensions the ROM picker should filter on (e.g. ["nes"]).
    valid_extensions: Vec<String>,
    /// "donor" = official emulator core ripped from a user-supplied
    /// official WAD (ROM swap); "bundled" = a DOL this project ships itself
    /// (unofficial). The UI groups the core dropdown on this.
    source: &'static str,
    /// Console button -> physical input defaults, one entry per supported
    /// controller. The UI builds its button-binding rows from these.
    default_mappings: Vec<ButtonMapping>,
    /// Core-specific settings (accessories, video, compatibility, ...). The
    /// UI builds this core's "Core Options" panel from them.
    options: Vec<CoreOption>,
    /// Console buttons that only exist while an option has a given value
    /// (e.g. Genesis X/Y/Z/Mode with the six-button pad).
    button_requires: std::collections::BTreeMap<String, OptionCondition>,
    /// Extra emulated controllers one physical controller can drive (e.g. N64
    /// controller 2 in dual-controller mode).
    controller_ports: Vec<ControllerPort>,
    /// Present when the core needs a user-supplied donor WAD; the
    /// frontend should prompt for one (and for the keys file) before
    /// calling build_wad_command.
    donor_label: Option<String>,
}

/// The UI's default registry path ("cores/registry.json") is relative, and
/// the process's working directory differs between `cargo run` (repo root),
/// `cargo tauri dev` (src-tauri/) and a packaged app. If the path doesn't
/// exist as given, look for it in the cwd's, the executable's and the
/// source tree's ancestor directories.
fn resolve_registry_path(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() || p.exists() {
        return p;
    }
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            starts.push(dir.to_path_buf());
        }
    }
    starts.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for start in starts {
        for dir in start.ancestors() {
            let candidate = dir.join(&p);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    p
}

#[tauri::command]
pub fn list_cores(registry_path: String) -> Result<Vec<CoreSummary>, String> {
    let resolved = resolve_registry_path(&registry_path);
    let cores = load_registry(&resolved)
        .map_err(|e| format!("could not load core registry '{}': {e}", resolved.display()))?;
    Ok(cores
        .into_iter()
        .map(|c: CoreDefinition| {
            let (source, donor_label) = match &c.core_source {
                CoreSource::Donor { donor_label, .. } => ("donor", Some(donor_label.clone())),
                CoreSource::Bundled { .. } => ("bundled", None),
            };
            CoreSummary {
                id: c.id,
                system: c.system,
                valid_extensions: c.valid_extensions,
                source,
                default_mappings: c.default_mappings,
                options: c.options,
                button_requires: c.button_requires,
                controller_ports: c.controller_ports,
                donor_label,
            }
        })
        .collect())
}

/// Bindings for one enabled physical controller, as sent by the UI.
#[derive(Deserialize)]
pub struct DeviceInput {
    /// "wiimote_sideways" | "wiimote_nunchuk" | "classic_controller" | "gamecube"
    pub device: String,
    /// Bindings for emulated controller 1. May be empty: that means the user
    /// unmapped everything, not "use defaults".
    #[serde(default)]
    pub map: std::collections::BTreeMap<String, String>,
    /// Bindings for emulated controllers 2 and up, keyed by port number.
    #[serde(default)]
    pub ports: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
}

type OptionValues = std::collections::BTreeMap<String, serde_json::Value>;

/// Build the unified core config from what the UI sent. `input` lists every
/// enabled controller in the order the user enabled them; the first is the
/// one recorded in the legacy binary blob used by donor builds.
fn assemble_config(
    core: &CoreDefinition,
    input: Vec<DeviceInput>,
    options: Option<OptionValues>,
) -> Result<(CoreConfig, InputDeviceId), String> {
    let mut setups = Vec::with_capacity(input.len());
    for DeviceInput { device, map, ports } in input {
        let device: InputDevice = device.parse()?;
        let ports = ports
            .into_iter()
            .map(|(port, buttons)| {
                port.parse::<u8>()
                    .map(|p| (p, buttons))
                    .map_err(|_| format!("invalid controller port '{port}'"))
            })
            .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
        setups.push(DeviceSetup { device, mapping: Some(InputMapping { buttons: map, ports }) });
    }
    let primary = setups
        .first()
        .map(|s| InputDeviceId::from(s.device))
        .ok_or("select at least one controller")?;
    let config = build_core_config(core, &setups, &options.unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok((config, primary))
}

/// The exact config file a build with these settings would hand to the core,
/// so the UI can show it.
#[tauri::command]
pub fn preview_core_config(
    registry_path: String,
    core_id: String,
    input: Vec<DeviceInput>,
    options: Option<OptionValues>,
) -> Result<String, String> {
    let cores = load_registry(&resolve_registry_path(&registry_path)).map_err(|e| e.to_string())?;
    let core = find_core(&cores, &core_id).map_err(|e| e.to_string())?;
    assemble_config(core, input, options).map(|(config, _)| config.to_json())
}

#[tauri::command]
pub fn build_wad_command(
    registry_path: String,
    core_id: String,
    rom_path: String,
    cover_path: Option<String>,
    title: String,
    output_path: String,
    input: Vec<DeviceInput>,
    options: Option<OptionValues>,
    donor_path: Option<String>,
    keys_path: Option<String>,
) -> Result<String, String> {
    let cores = load_registry(&resolve_registry_path(&registry_path)).map_err(|e| e.to_string())?;
    let core = find_core(&cores, &core_id).map_err(|e| e.to_string())?;

    if let CoreSource::Donor { donor_label, .. } = &core.core_source {
        if donor_path.is_none() || keys_path.is_none() {
            return Err(format!(
                "{} requires a donor WAD and a keys file. Supply: {donor_label}",
                core.system
            ));
        }
    }

    let key_provider = keys_path
        .as_deref()
        .map(|p| KeyProvider::from_file(std::path::Path::new(p)))
        .transpose()
        .map_err(|e| e.to_string())?;

    let cover_bytes = cover_path
        .map(std::fs::read)
        .transpose()
        .map_err(|e| e.to_string())?;

    // Unified core config file: bindings + option values, validated against
    // the core's registry entry. Options that don't apply to the enabled
    // controllers resolve to their defaults.
    let (core_config, device_id) = assemble_config(core, input, options)?;

    // The legacy binary blob below is only used by donor (official VC) builds;
    // its `button_map` is still not translated.
    let title_id: [u8; 8] = [0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD, 0xBE, 0xEF];
    let title_key: [u8; 16] = [0u8; 16];

    let config = VcConfig {
        console_id: 0,
        input_device: device_id,
        button_map: [0u8; 16],
        save_target: SaveTarget::NandSavePartition,
        video_mode: 0,
    };

    let wad_bytes = build_wad(WadBuildRequest {
        core,
        rom_path: &PathBuf::from(rom_path),
        cover_art: cover_bytes.as_deref(),
        title,
        config,
        // Bundled cores read the JSON config; donor builds keep the legacy blob.
        core_config: matches!(core.core_source, CoreSource::Bundled { .. }).then(|| core_config.to_bytes()),
        title_id,
        title_key,
        donor_wad_path: donor_path.as_ref().map(std::path::Path::new),
        keys: key_provider.as_ref(),
    })
    .map_err(|e| e.to_string())?;

    std::fs::write(&output_path, wad_bytes).map_err(|e| e.to_string())?;
    Ok(output_path)
}
