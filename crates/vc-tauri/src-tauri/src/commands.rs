//! Tauri command handlers. Deliberately thin: the frontend never touches
//! binary format details, it just collects inputs (ROM path, core id,
//! button mapping, cover art) and displays progress. All real work goes
//! through vc_core::wad::build_wad.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vc_core::config::{InputDeviceId, SaveTarget, VcConfig};
use vc_core::donor::KeyProvider;
use vc_core::registry::{find_core, load_registry, CoreDefinition, CoreSource};
use vc_core::wad::{build_wad, WadBuildRequest};

#[derive(Serialize)]
pub struct CoreSummary {
    id: String,
    system: String,
    /// File extensions the ROM picker should filter on (e.g. ["nes"]).
    valid_extensions: Vec<String>,
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
            let donor_label = match &c.core_source {
                CoreSource::Donor { donor_label, .. } => Some(donor_label.clone()),
                CoreSource::Bundled { .. } => None,
            };
            CoreSummary {
                id: c.id,
                system: c.system,
                valid_extensions: c.valid_extensions,
                donor_label,
            }
        })
        .collect())
}

#[derive(Deserialize)]
pub struct ButtonMappingInput {
    pub device: String, // "wiimote_sideways" | "classic_controller" | "gamecube"
    pub map: std::collections::BTreeMap<String, String>,
}

#[tauri::command]
pub fn build_wad_command(
    registry_path: String,
    core_id: String,
    rom_path: String,
    cover_path: Option<String>,
    title: String,
    output_path: String,
    _mapping: ButtonMappingInput,
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

    // TODO: translate `_mapping` into VcConfig.button_map using the core's
    // default_mappings key order once that lookup helper is written.
    let title_id: [u8; 8] = [0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD, 0xBE, 0xEF];
    let title_key: [u8; 16] = [0u8; 16];

    let config = VcConfig {
        console_id: 0,
        input_device: InputDeviceId::ClassicController,
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
        title_id,
        title_key,
        donor_wad_path: donor_path.as_ref().map(std::path::Path::new),
        keys: key_provider.as_ref(),
    })
    .map_err(|e| e.to_string())?;

    std::fs::write(&output_path, wad_bytes).map_err(|e| e.to_string())?;
    Ok(output_path)
}
