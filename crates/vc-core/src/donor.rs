//! Donor-WAD extraction.
//!
//! This project does not ship, embed, or bundle any Nintendo (or
//! third-party) copyrighted emulator core. For any `CoreSource::Donor`
//! entry in the registry, the DOL is instead ripped out of an official
//! WAD the *user* supplies — one they already legitimately own — at build
//! time. Nothing here downloads, sources, or infers a donor WAD on the
//! user's behalf; `donor_store` only manages files the user explicitly
//! imported themselves.
//!
//! The same principle applies to key material: `KeyProvider` never embeds
//! Nintendo's Wii common key. It is loaded from a local file the user
//! places themselves, the same way tools like Dolphin require a
//! self-dumped keys file rather than shipping one. If that file is
//! missing, donor extraction simply fails with a clear error — there is
//! no fallback value anywhere in this codebase.

use crate::crypto::{decrypt_content, sha1_hash};
use crate::error::VcError;
use crate::registry::{CoreDefinition, CoreSource};
use std::fs;
use std::path::Path;

/// User-supplied 16-byte Wii common key, loaded from a local file. Users
/// obtain this themselves (dumped from their own console), exactly as
/// they would to use Dolphin or any other Wii software that touches
/// encrypted NAND content. This tool provides no key material and no
/// means of acquiring one.
pub struct KeyProvider {
    common_key: [u8; 16],
}

impl KeyProvider {
    /// Accepts either a raw 16-byte file or a 32-character hex string
    /// (with or without surrounding whitespace/newline), for convenience.
    pub fn from_file(path: &Path) -> Result<Self, VcError> {
        let raw = fs::read(path)?;
        let common_key = if raw.len() == 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&raw);
            arr
        } else {
            let text = String::from_utf8_lossy(&raw);
            let hex: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
            decode_hex16(&hex)
                .ok_or_else(|| VcError::DonorKey("expected 16 raw bytes or 32 hex chars".into()))?
        };
        Ok(KeyProvider { common_key })
    }

    pub fn common_key(&self) -> &[u8; 16] {
        &self.common_key
    }
}

fn decode_hex16(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// One content entry located inside a parsed donor WAD.
pub struct DonorContent {
    pub index: u16,
    pub size: u64,
    pub sha1: [u8; 20],
    offset_in_data: usize,
}

/// Minimal parsed view of a donor WAD: just enough to locate and decrypt
/// one content entry (the DOL) by index, and to confirm the WAD's title
/// id matches what the target core expects before trusting its contents.
/// This is a reader, not a Nintendo-signature validator — we don't have
/// Nintendo's public key material to check against, and don't need it: we
/// trust the user's own legitimately-sourced dump, the same way Dolphin
/// does.
pub struct DonorWad {
    pub title_id: [u8; 8],
    pub encrypted_title_key: [u8; 16],
    pub contents: Vec<DonorContent>,
    data: Vec<u8>,
}

/// Parse a raw WAD file: header, cert chain (skipped), ticket, tmd,
/// content data.
pub fn parse_donor_wad(_bytes: &[u8]) -> Result<DonorWad, VcError> {
    // TODO: real WAD container parsing — read the header's section sizes,
    // skip the cert chain, parse the ticket for `encrypted_title_key` and
    // `title_id`, parse the TMD's content table for per-content
    // size/index/sha1, then slice `data` out at each content's 64-byte-
    // aligned offset. This mirrors `wad::build_wad` in reverse and should
    // be validated by round-tripping a WAD this project builds itself
    // before trusting it against real official WADs.
    Err(VcError::DonorWad(
        "WAD container parsing not yet implemented".into(),
    ))
}

/// Confirm this donor WAD is actually the title the selected core expects,
/// so we don't silently rip a core out of the wrong system's channel.
pub fn verify_donor_matches(donor: &DonorWad, core: &CoreDefinition) -> Result<(), VcError> {
    let CoreSource::Donor { title_id, .. } = &core.core_source else {
        return Err(VcError::DonorWad(format!(
            "{} is not configured as a donor-sourced core",
            core.system
        )));
    };

    if &donor.title_id != title_id {
        return Err(VcError::DonorWad(format!(
            "donor WAD title id {:02x?} does not match {} requirement {:02x?} — \
             wrong donor supplied for this core",
            donor.title_id, core.system, title_id
        )));
    }
    Ok(())
}

/// Decrypt and return the raw DOL bytes for the content index the core's
/// donor requirement specifies, verifying its hash against the TMD-derived
/// value before handing it back. Fails closed: any mismatch (wrong donor,
/// wrong content index, corrupt dump, wrong key) is an error, never a
/// best-effort fallback.
pub fn extract_core(
    donor: &DonorWad,
    core: &CoreDefinition,
    keys: &KeyProvider,
) -> Result<Vec<u8>, VcError> {
    verify_donor_matches(donor, core)?;

    let CoreSource::Donor { dol_content_index, .. } = &core.core_source else {
        unreachable!("verify_donor_matches already confirmed this is a Donor source");
    };

    let content = donor
        .contents
        .iter()
        .find(|c| c.index == *dol_content_index)
        .ok_or_else(|| {
            VcError::DonorWad(format!(
                "donor WAD has no content at index {dol_content_index}"
            ))
        })?;

    // TODO: decrypt `donor.encrypted_title_key` with `keys.common_key()`
    // (AES-CBC, IV = title id zero-padded to 16 bytes) to get the
    // plaintext per-title key. Passing the common key straight through
    // below is a placeholder — content encryption uses the *title* key,
    // not the common key directly.
    let title_key = keys.common_key();

    let start = content.offset_in_data;
    let end = start + content.size as usize;
    let ciphertext = donor
        .data
        .get(start..end)
        .ok_or_else(|| VcError::DonorWad("donor content offset out of range".into()))?;

    let plaintext = decrypt_content(ciphertext, title_key, content.index);
    let actual_hash = sha1_hash(&plaintext[..content.size as usize]);
    if actual_hash != content.sha1 {
        return Err(VcError::DonorWad(
            "extracted core failed hash verification — donor WAD may be corrupt \
             or the wrong key was supplied"
                .into(),
        ));
    }

    Ok(plaintext[..content.size as usize].to_vec())
}
