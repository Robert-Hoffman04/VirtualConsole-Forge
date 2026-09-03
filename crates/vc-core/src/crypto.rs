//! Crypto primitives used by the WAD format: AES-128-CBC content encryption,
//! SHA-1 content hashing, and "fakesigning" (the homebrew-scene technique of
//! producing a structurally-valid but not Nintendo-signed signature block,
//! which only installs on consoles with the Trucha bug patch applied via a
//! cIOS). Nothing else in the codebase should touch raw AES/SHA-1 directly —
//! go through these functions so the primitives stay swappable/auditable
//! in one place.

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha1::{Digest, Sha1};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// Compute the SHA-1 hash of a content blob, as stored in the TMD's content
/// table for integrity verification.
pub fn sha1_hash(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&out);
    arr
}

/// Derive the per-content IV from its content index, per the WAD format
/// convention (index in the high two bytes, zero-padded).
fn content_iv(content_index: u16) -> [u8; 16] {
    let mut iv = [0u8; 16];
    iv[0..2].copy_from_slice(&content_index.to_be_bytes());
    iv
}

/// Encrypt one content file with the title key derived for this WAD.
/// Data must already be padded to a 16-byte boundary by the caller (the
/// WAD spec pads content with zero bytes to the next AES block).
pub fn encrypt_content(data: &[u8], title_key: &[u8; 16], content_index: u16) -> Vec<u8> {
    let iv = content_iv(content_index);
    let padded_len = data.len().div_ceil(16) * 16;
    let mut buf = vec![0u8; padded_len];
    buf[..data.len()].copy_from_slice(data);

    let enc = Aes128CbcEnc::new(title_key.into(), &iv.into());
    enc.encrypt_padded_mut::<NoPadding>(&mut buf, data.len())
        .expect("buffer pre-padded to block size")
        .to_vec()
}

/// Decrypt a content file — used by the CLI's verify/round-trip checks.
pub fn decrypt_content(data: &[u8], title_key: &[u8; 16], content_index: u16) -> Vec<u8> {
    let iv = content_iv(content_index);
    let mut buf = data.to_vec();
    let dec = Aes128CbcDec::new(title_key.into(), &iv.into());
    dec.decrypt_padded_mut::<NoPadding>(&mut buf)
        .expect("valid ciphertext")
        .to_vec()
}

/// Produce a structurally-valid but Nintendo-unsigned signature block
/// ("fakesigning"). Real signature verification is bypassed on the console
/// side by the Trucha bug patch present in essentially every cIOS used to
/// install homebrew WADs — this function does not attempt to forge a real
/// RSA signature, it emits the all-zero/placeholder pattern the patched
/// IOS accepts.
pub struct FakeSignature {
    pub bytes: Vec<u8>,
}

pub fn sign_fakesigned(_data: &[u8]) -> FakeSignature {
    // TODO: emit the exact signature-block layout (type + RSA-2048 sized
    // padding + issuer string) expected by the ticket/TMD signed-blob
    // format, with content that hashes to a value starting 0x00 as the
    // Trucha bug patch checks for.
    FakeSignature { bytes: vec![0u8; 0x140] }
}
