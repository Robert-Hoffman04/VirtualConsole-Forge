//! ROM ingestion: validation and normalization only. No packaging concerns
//! live here — this module just makes sure the bytes handed to `wad::build_wad`
//! are in the shape the target core expects.

use crate::error::VcError;
use crate::registry::CoreDefinition;
use std::path::Path;

/// Check the ROM's extension and (where feasible) header bytes against what
/// the selected core expects. This is a compatibility sanity check, not a
/// legal gate — it exists to catch mismatched-extension or corrupted dumps
/// before they get baked into a WAD and fail silently on hardware.
pub fn validate_rom(path: &Path, bytes: &[u8], core: &CoreDefinition) -> Result<(), VcError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !core.valid_extensions.iter().any(|e| e == &ext) {
        return Err(VcError::InvalidRom(format!(
            "extension '.{ext}' not valid for {} (expected one of {:?})",
            core.system, core.valid_extensions
        )));
    }

    if bytes.is_empty() {
        return Err(VcError::InvalidRom("file is empty".into()));
    }

    // Per-core header sniffing hooks go here as cores are added, e.g.
    // checking for a valid SNES/NES header signature at the expected offset.

    Ok(())
}

/// Apply per-core normalization: strip copier headers, byteswap, etc.
/// Returns a new buffer; never mutates in place so callers can keep the
/// original bytes around for diagnostics.
pub fn normalize_rom(bytes: &[u8], core: &CoreDefinition) -> Vec<u8> {
    match core.header_size {
        Some(n) if bytes.len() > n as usize => bytes[n as usize..].to_vec(),
        _ => bytes.to_vec(),
    }
}
