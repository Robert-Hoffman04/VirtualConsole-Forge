//! Core plugin registry.
//!
//! Each supported system (NES, SNES, Genesis, etc.) is described by a
//! `CoreDefinition` loaded from `cores/registry.json`. Adding a new system
//! means adding a DOL + a JSON entry here — no changes to packing code.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreDefinition {
    /// Human-readable system name, e.g. "SNES"
    pub system: String,
    /// Unique id used in title-id allocation / CLI selection, e.g. "snes"
    pub id: String,
    /// Path to the prebuilt DOL, relative to the registry file
    pub dol_path: PathBuf,
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
/// DOL paths inside each definition are resolved relative to the registry
/// file's parent directory.
pub fn load_registry(path: &Path) -> Result<Vec<CoreDefinition>, VcError> {
    let raw = fs::read_to_string(path)?;
    let mut defs: Vec<CoreDefinition> = serde_json::from_str(&raw)?;

    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for def in &mut defs {
        if def.dol_path.is_relative() {
            def.dol_path = base.join(&def.dol_path);
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
