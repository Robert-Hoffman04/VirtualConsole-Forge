//! Top-level orchestrator. `build_wad` is the single public entry point
//! both `vc-cli` and `vc-tauri` call — everything upstream of it
//! (registry, rom, config, banner, tmd, ticket, crypto) is implementation
//! detail this module wires together in the correct order.

use crate::banner::{build_banner, BannerInput};
use crate::config::VcConfig;
use crate::crypto::{encrypt_content, sha1_hash, sign_fakesigned};
use crate::donor::{extract_core, parse_donor_wad, KeyProvider};
use crate::error::VcError;
use crate::registry::{CoreDefinition, CoreSource};
use crate::rom::{normalize_rom, validate_rom};
use crate::ticket::{build_ticket, TicketBuildRequest};
use crate::tmd::{build_tmd, ContentEntry, TmdBuildRequest};

use std::fs;
use std::path::Path;

pub struct WadBuildRequest<'a> {
    pub core: &'a CoreDefinition,
    pub rom_path: &'a Path,
    pub cover_art: Option<&'a [u8]>,
    pub title: String,
    pub config: VcConfig,
    /// 8-byte title id; typically derived from `core.title_id_prefix` plus
    /// an allocated suffix, done by the caller so it can track collisions
    /// across multiple builds.
    pub title_id: [u8; 8],
    pub title_key: [u8; 16],
    /// Required when `core.core_source` is `CoreSource::Donor`; ignored
    /// for `CoreSource::Bundled`. Path to a WAD the user supplied
    /// themselves (typically resolved via `DonorStore::lookup` before
    /// calling `build_wad`).
    pub donor_wad_path: Option<&'a Path>,
    /// Required alongside `donor_wad_path`. Never has a default value —
    /// see `donor::KeyProvider`.
    pub keys: Option<&'a KeyProvider>,
}

/// Assemble a complete, installable WAD from a validated ROM, a core
/// definition, and a baked-in config. Order of operations:
/// 1. validate + normalize the ROM for this core
/// 2. build the banner content
/// 3. serialize the config content
/// 4. obtain the core's DOL — either read from `cores/` (Bundled) or
///    ripped from a user-supplied donor WAD (Donor)
/// 5. assemble the content list, compute per-content hashes
/// 6. build TMD + ticket around that content list
/// 7. encrypt each content with the title key
/// 8. concatenate header + cert chain + ticket + tmd + contents + footer
pub fn build_wad(req: WadBuildRequest) -> Result<Vec<u8>, VcError> {
    let rom_bytes = fs::read(req.rom_path)?;
    validate_rom(req.rom_path, &rom_bytes, req.core)?;
    let rom_bytes = normalize_rom(&rom_bytes, req.core);

    let banner_bytes = build_banner(&BannerInput {
        cover_art: req.cover_art,
        title: req.title.clone(),
    })?;

    let config_bytes = req.config.to_bytes().to_vec();

    let dol_bytes = match &req.core.core_source {
        CoreSource::Bundled { dol_path } => fs::read(dol_path)?,
        CoreSource::Donor { .. } => {
            let donor_path = req.donor_wad_path.ok_or_else(|| {
                VcError::DonorWad(format!(
                    "{} requires a donor WAD but none was supplied",
                    req.core.system
                ))
            })?;
            let keys = req.keys.ok_or_else(|| {
                VcError::DonorKey(
                    "donor extraction requires a KeyProvider (user-supplied common key)".into(),
                )
            })?;
            let donor_bytes = fs::read(donor_path)?;
            let donor = parse_donor_wad(&donor_bytes)?;
            extract_core(&donor, req.core, keys)?
        }
    };

    // Content order: DOL, ROM, banner, config — matches the table in
    // build_wad's module doc and the earlier design discussion.
    let raw_contents: Vec<(u16, u16, Vec<u8>)> = vec![
        (0, 0x0001, dol_bytes),
        (1, 0x0001, rom_bytes),
        (2, 0x0001, banner_bytes),
        (3, 0x0001, config_bytes),
    ];

    let mut content_entries = Vec::new();
    let mut encrypted_blobs = Vec::new();

    for (index, content_type, data) in &raw_contents {
        let hash = sha1_hash(data);
        content_entries.push(ContentEntry {
            id: *index as u32,
            index: *index,
            content_type: *content_type,
            size: data.len() as u64,
            sha1: hash,
        });
        encrypted_blobs.push(encrypt_content(data, &req.title_key, *index));
    }

    let tmd_body = build_tmd(&TmdBuildRequest {
        title_id: req.title_id,
        title_version: 0,
        ios_version: req.core.ios_version,
        contents: content_entries,
    });
    let tmd_sig = sign_fakesigned(&tmd_body);

    // Title key would be encrypted with the Wii common key here before
    // being embedded in the ticket; left as a passthrough placeholder
    // until crypto::encrypt_title_key is implemented.
    let encrypted_title_key = req.title_key;
    let ticket_body = build_ticket(
        &TicketBuildRequest {
            title_id: req.title_id,
            title_key: req.title_key,
            common_key_index: 0,
        },
        encrypted_title_key,
    );
    let ticket_sig = sign_fakesigned(&ticket_body);

    // TODO: assemble the real WAD container — header (type/size fields),
    // cert chain, [ticket_sig + ticket_body], [tmd_sig + tmd_body],
    // encrypted_blobs in content order, footer — each section 64-byte
    // aligned per the WAD spec.
    let mut out = Vec::new();
    out.extend_from_slice(&ticket_sig.bytes);
    out.extend_from_slice(&ticket_body);
    out.extend_from_slice(&tmd_sig.bytes);
    out.extend_from_slice(&tmd_body);
    for blob in &encrypted_blobs {
        out.extend_from_slice(blob);
    }

    Ok(out)
}
