//! Builds banner.bin: the animated Wii Menu icon/banner content. Kept
//! isolated so the compositing approach (resize/pad cover art, overlay
//! title text) can change without touching packing or crypto code.

use crate::error::VcError;

pub struct BannerInput<'a> {
    /// Optional cover art; falls back to a generic placeholder if absent.
    pub cover_art: Option<&'a [u8]>,
    pub title: String,
}

/// Produce the raw bytes for the banner content file. Placeholder
/// implementation: real version composites cover art onto the standard
/// Wii banner template dimensions and encodes the TPL/animation frames
/// the system menu expects.
pub fn build_banner(input: &BannerInput) -> Result<Vec<u8>, VcError> {
    // TODO: load banner template, composite cover_art via `image`, encode
    // to the Wii's banner texture format, assemble banner.bin container
    // (icon.bin + banner.bin + sound.bin equivalents packed together).
    let _ = input;
    Err(VcError::BannerError("banner generation not yet implemented".into()))
}
