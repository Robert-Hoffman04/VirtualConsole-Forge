//! Ticket structure and builder.
//!
//! Carries the title key (encrypted with the Wii common key) plus metadata
//! tying the install to a title id. Like `tmd.rs`, kept dependency-light and
//! offset-commented for diffing against reference dumps.

#[derive(Debug, Clone)]
pub struct TicketBuildRequest {
    pub title_id: [u8; 8],
    /// Plaintext title key for this WAD's content encryption. Encrypted
    /// with the Wii common key (indexed by `common_key_index`) before
    /// being written into the ticket body.
    pub title_key: [u8; 16],
    pub common_key_index: u8,
}

/// Build the unsigned ticket body. The caller prepends the signature block
/// and certificate chain, same as `tmd::build_tmd`.
///
/// Layout (simplified):
/// - title key (encrypted, AES-CBC keyed by the common key, IV = title id
///   padded to 16 bytes)
/// - ticket id, console id, title id, title version
/// - common key index
pub fn build_ticket(req: &TicketBuildRequest, encrypted_title_key: [u8; 16]) -> Vec<u8> {
    let mut body = Vec::new();

    body.extend_from_slice(&encrypted_title_key);
    body.extend_from_slice(&[0u8; 8]); // ticket id, placeholder
    body.extend_from_slice(&[0u8; 4]); // console id, placeholder
    body.extend_from_slice(&req.title_id);
    body.push(req.common_key_index);
    body.extend_from_slice(&[0u8; 2]); // padding / reserved

    body
}
