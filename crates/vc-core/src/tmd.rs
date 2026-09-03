//! Title Metadata (TMD) structure and builder.
//!
//! Lists the title id, IOS dependency, and a table of every content file
//! in the package with its size and SHA-1 hash — this is what the IOS
//! checks each installed content against. Byte offsets are intentionally
//! called out in comments since this module is the one worth diffing
//! against reference WAD dumps when something fails to install.

/// One row in the TMD's content table.
#[derive(Debug, Clone)]
pub struct ContentEntry {
    /// Unique content id (arbitrary but conventionally matches index)
    pub id: u32,
    /// Position within the content list (0 = DOL, 1 = ROM, 2 = banner, 3 = config, ...)
    pub index: u16,
    /// Content type flags (normal vs shared, etc.)
    pub content_type: u16,
    /// Decrypted size in bytes
    pub size: u64,
    pub sha1: [u8; 20],
}

#[derive(Debug, Clone)]
pub struct TmdBuildRequest {
    pub title_id: [u8; 8],
    pub title_version: u16,
    pub ios_version: u16,
    pub contents: Vec<ContentEntry>,
}

/// Build the unsigned TMD body (header + content records). The caller
/// (`wad.rs`) is responsible for prepending the signature block produced
/// by `crypto::sign_fakesigned` and the certificate chain.
///
/// Layout (simplified, matches the public WAD/TMD documentation):
/// - 0x000..0x1C4  signature block (added by caller)
/// - 0x1C4..0x1CC  issuer string (added by caller)
/// - header fields: sys version, title id, title type, group id, ios version,
///   title version, boot index, content count
/// - 0x1E4..        content table, 36 bytes per ContentEntry
///   (content id[4] + index[2] + type[2] + size[8] + sha1[20])
pub fn build_tmd(req: &TmdBuildRequest) -> Vec<u8> {
    let mut body = Vec::new();

    // --- header fields ---
    body.extend_from_slice(&req.ios_version.to_be_bytes());
    body.extend_from_slice(&[0u8; 6]); // sys version padding, placeholder
    body.extend_from_slice(&req.title_id);
    body.extend_from_slice(&[0u8; 4]); // title type
    body.extend_from_slice(&[0u8; 2]); // group id
    body.extend_from_slice(&req.title_version.to_be_bytes());
    body.extend_from_slice(&[0u8; 2]); // boot index
    body.extend_from_slice(&(req.contents.len() as u16).to_be_bytes());

    // --- content table ---
    for c in &req.contents {
        body.extend_from_slice(&c.id.to_be_bytes());
        body.extend_from_slice(&c.index.to_be_bytes());
        body.extend_from_slice(&c.content_type.to_be_bytes());
        body.extend_from_slice(&c.size.to_be_bytes());
        body.extend_from_slice(&c.sha1);
    }

    body
}
