//! Tauri command handlers. Deliberately thin: the frontend never touches
//! binary format details, it just collects inputs (ROM path, core id,
//! button mapping, cover art) and displays progress. All real work goes
//! through vc_core::wad::build_wad.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vc_core::config::{InputDeviceId, SaveTarget, VcConfig};
use vc_core::registry::{find_core, load_registry, CoreDefinition};
use vc_core::wad::{build_wad, WadBuildRequest};

#[derive(Serialize)]
pub struct CoreSummary {
    id: String,
    system: String,
}

#[tauri::command]
pub fn list_cores(registry_path: String) -> Result<Vec<CoreSummary>, String> {
    let cores = load_registry(&PathBuf::from(registry_path)).map_err(|e| e.to_string())?;
    Ok(cores
        .into_iter()
        .map(|c: CoreDefinition| CoreSummary {
            id: c.id,
            system: c.system,
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
) -> Result<String, String> {
    let cores = load_registry(&PathBuf::from(registry_path)).map_err(|e| e.to_string())?;
    let core = find_core(&cores, &core_id).map_err(|e| e.to_string())?;

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
    })
    .map_err(|e| e.to_string())?;

    std::fs::write(&output_path, wad_bytes).map_err(|e| e.to_string())?;
    Ok(output_path)
}
