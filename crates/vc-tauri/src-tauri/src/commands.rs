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
use vc_core::forwarder::{build_forwarder_wad, build_launch_image, default_core_path, ForwarderWadRequest};
use vc_core::launch::Device;
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
    /// Can a forwarder WAD launch this core? Only project-owned (bundled)
    /// cores can: donor-ripped official emulators never read a launch image.
    forwardable: bool,
    /// Where the core DOL is expected on the SD/USB device in forwarder mode.
    forwarder_core_path: String,
    /// Present when the core needs a user-supplied donor WAD; the
    /// frontend should prompt for one (and for the keys file) before
    /// calling build_wad_command.
    donor_label: Option<String>,
}

/// Paths the UI defaults to ("cores/registry.json", "forwarder/prebuilt/main.dol") are
/// relative to the repository, and
/// the process's working directory differs between `cargo run` (repo root),
/// `cargo tauri dev` (src-tauri/) and a packaged app. If the path doesn't
/// exist as given, look for it in the cwd's, the executable's and the
/// source tree's ancestor directories.
fn resolve_repo_path(path: &str) -> PathBuf {
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
    let resolved = resolve_repo_path(&registry_path);
    let cores = load_registry(&resolved)
        .map_err(|e| format!("could not load core registry '{}': {e}", resolved.display()))?;
    Ok(cores
        .into_iter()
        .map(|c: CoreDefinition| {
            let forwardable = matches!(c.core_source, CoreSource::Bundled { .. });
            let forwarder_core_path = default_core_path(&c);
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
                forwardable,
                forwarder_core_path,
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
    let cores = load_registry(&resolve_repo_path(&registry_path)).map_err(|e| e.to_string())?;
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
    let cores = load_registry(&resolve_repo_path(&registry_path)).map_err(|e| e.to_string())?;
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


/// What a forwarder build produced, so the UI can tell the user what to put on the device.
#[derive(Debug, Serialize)]
pub struct ForwarderBuildResult {
    /// The WAD that was written.
    wad_path: String,
    /// launch.cfg, if the caller asked for a copy.
    launch_cfg_path: Option<String>,
    /// "sd" or "usb".
    device: String,
    /// Where the core DOL must be on the device.
    core_device_path: String,
    /// The core DOL on this computer (what to copy there).
    core_source_path: Option<String>,
    /// Where the ROM must be on the device.
    rom_device_path: String,
}

/// Build a forwarder WAD: a generic loader plus a small `launch.cfg`, with no
/// ROM and no core inside. The core and ROM are read from the SD card or USB
/// drive when the channel starts. Bundled cores only.
#[tauri::command]
pub fn build_forwarder_command(
    registry_path: String,
    core_id: String,
    cover_path: Option<String>,
    title: String,
    output_path: String,
    loader_path: Option<String>,
    device: String,
    device_rom_path: String,
    device_core_path: Option<String>,
    launch_cfg_out: Option<String>,
    input: Vec<DeviceInput>,
    options: Option<OptionValues>,
) -> Result<ForwarderBuildResult, String> {
    let cores = load_registry(&resolve_repo_path(&registry_path)).map_err(|e| e.to_string())?;
    let core = find_core(&cores, &core_id).map_err(|e| e.to_string())?;

    let device: Device = device.parse().map_err(|e: vc_core::VcError| e.to_string())?;
    let non_empty = |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let cover_bytes = non_empty(cover_path).map(std::fs::read).transpose().map_err(|e| e.to_string())?;
    let (core_config, _) = assemble_config(core, input, options)?;

    let loader = resolve_repo_path(non_empty(loader_path).as_deref().unwrap_or("forwarder/prebuilt/main.dol"));
    let req = ForwarderWadRequest {
        core,
        loader_dol: &loader,
        title,
        cover_art: cover_bytes.as_deref(),
        device,
        core_path: non_empty(device_core_path),
        rom_path: device_rom_path.trim().to_string(),
        core_config: core_config.to_bytes(),
        title_id: [0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD, 0xBE, 0xEF],
        title_key: [0u8; 16],
    };

    // Same order as vc-cli: launch.cfg doesn't depend on the loader, banner
    // or WAD container, so it can be saved even while those are unfinished.
    let launch = build_launch_image(&req).map_err(|e| e.to_string())?;
    let launch_cfg_path = non_empty(launch_cfg_out);
    if let Some(path) = &launch_cfg_path {
        let bytes = launch.to_bytes().map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| format!("cannot write launch.cfg to {path}: {e}"))?;
    }

    let saved_note = launch_cfg_path
        .as_ref()
        .map(|p| format!(" (launch.cfg was saved to {p})"))
        .unwrap_or_default();
    let wad_bytes = build_forwarder_wad(req).map_err(|e| format!("{e}{saved_note}"))?;
    std::fs::write(&output_path, wad_bytes).map_err(|e| e.to_string())?;

    Ok(ForwarderBuildResult {
        wad_path: output_path,
        launch_cfg_path,
        device: device.as_str().to_string(),
        core_device_path: launch.core_path,
        core_source_path: match &core.core_source {
            CoreSource::Bundled { dol_path } => Some(dol_path.display().to_string()),
            CoreSource::Donor { .. } => None,
        },
        rom_device_path: launch.rom_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vc_core::launch::LaunchImage;

    fn classic() -> Vec<DeviceInput> {
        vec![DeviceInput { device: "classic_controller".into(), map: Default::default(), ports: Default::default() }]
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vcforge-tauri-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[allow(clippy::too_many_arguments)]
    fn forward(core: &str, rom: &str, device: &str, dir: &std::path::Path, input: Vec<DeviceInput>) -> Result<ForwarderBuildResult, String> {
        build_forwarder_command(
            "cores/registry.json".into(), core.into(), None, "Test".into(),
            dir.join("out.wad").display().to_string(), None, device.into(), rom.into(), None,
            Some(dir.join("launch.cfg").display().to_string()), input, None,
        )
    }

    #[test]
    fn cores_report_whether_they_can_be_forwarded() {
        let cores = list_cores("cores/registry.json".into()).unwrap();
        let by = |id: &str| cores.iter().find(|c| c.id == id).unwrap();
        assert!(by("gb").forwardable && !by("nes").forwardable);
        assert_eq!(by("gb").forwarder_core_path, "/vcforge/cores/gb.dol");
    }

    #[test]
    fn forwarder_writes_a_launch_cfg_carrying_the_core_config() {
        let dir = scratch("cfg");
        // The WAD stage may still be unfinished (banner/container); launch.cfg is written before it.
        let outcome = forward("gb", "/vcforge/roms/Tetris.gb", "usb", &dir, classic());
        if let Err(e) = &outcome {
            assert!(e.contains("launch.cfg was saved"), "unexpected failure: {e}");
        }
        let img = LaunchImage::from_bytes(&std::fs::read(dir.join("launch.cfg")).unwrap()).unwrap();
        assert_eq!(img.device, Device::Usb);
        assert_eq!(img.rom_path, "/vcforge/roms/Tetris.gb");
        assert_eq!(img.core_path, "/vcforge/cores/gb.dol");
        let cfg: serde_json::Value = serde_json::from_slice(&img.payload).unwrap();
        assert_eq!(cfg["format"], "vcforge-config");
        assert!(cfg["input"]["devices"]["classic_controller"].is_object());
        if let Ok(r) = outcome {
            assert_eq!((r.device.as_str(), r.rom_device_path.as_str()), ("usb", "/vcforge/roms/Tetris.gb"));
        }
    }

    #[test]
    fn forwarder_refuses_what_cannot_be_forwarded() {
        let dir = scratch("refuse");
        let e = forward("nes", "/vcforge/roms/a.nes", "sd", &dir, classic()).unwrap_err();
        assert!(e.contains("donor-sourced"), "{e}");
        assert!(forward("gb", "/vcforge/roms/a.nes", "sd", &dir, classic()).is_err(), "wrong ROM extension");
        assert!(forward("gb", "roms/a.gb", "sd", &dir, classic()).is_err(), "device paths start with /");
        assert!(forward("gb", "/roms/a.gb", "tape", &dir, classic()).is_err(), "unknown device");
        assert!(forward("gb", "/roms/a.gb", "sd", &dir, vec![]).is_err(), "needs a controller");
    }
}
