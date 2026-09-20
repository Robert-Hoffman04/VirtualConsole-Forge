//! The launch image: what the forwarder loader hands to a core (Option B).
//!
//! This is the Rust writer/reader for the layout defined in
//! `forwarder/include/vcf_launch.h`. The two must stay byte-identical; both
//! sides check `forwarder/tests/golden_launch.txt`.
//!
//! An image is a fixed 544-byte header followed by the payload (the core
//! config JSON, see `coreconfig`), zero-padded to a multiple of 32. It is
//! stored as WAD content 2 (`launch.cfg`) and copied unchanged by the loader
//! to a fixed MEM2 address. Everything is big-endian.
//!
//! ```text
//! 0x000 magic 'LNCH'   0x004 version   0x008 header size   0x00C checksum
//! 0x010 flags (0)      0x014 device    0x018 payload size  0x01C reserved
//! 0x020 core_path[256] 0x120 rom_path[256]   0x220 payload...
//! ```

use crate::error::VcError;
use std::str::FromStr;

pub const LAUNCH_MAGIC: u32 = 0x4C4E_4348; // 'LNCH'
pub const LAUNCH_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 544;
/// Bytes per path field, including the terminating NUL.
pub const PATH_FIELD: usize = 256;
/// Header + padded payload must fit in the hand-off region (`VCF_LAUNCH_MAX`).
pub const MAX_TOTAL: usize = 64 * 1024;
/// WAD content index of `launch.cfg` (0 = banner, 1 = main.dol).
pub const CFG_CONTENT_INDEX: u16 = 2;

const OFF_CHECKSUM: usize = 12;
const OFF_CORE_PATH: usize = 32;
const OFF_ROM_PATH: usize = 288;

/// Storage device holding both the core DOL and the ROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Device {
    Sd = 0,
    Usb = 1,
}

impl Device {
    pub fn as_str(self) -> &'static str {
        match self {
            Device::Sd => "sd",
            Device::Usb => "usb",
        }
    }
}

impl FromStr for Device {
    type Err = VcError;
    fn from_str(s: &str) -> Result<Self, VcError> {
        match s.to_ascii_lowercase().as_str() {
            "sd" => Ok(Device::Sd),
            "usb" => Ok(Device::Usb),
            other => Err(VcError::Forwarder(format!(
                "unknown device '{other}' (expected 'sd' or 'usb')"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchImage {
    pub device: Device,
    /// Core DOL, relative to the device root: absolute-style, e.g. `/vcforge/cores/gb.dol`.
    pub core_path: String,
    /// ROM, same convention.
    pub rom_path: String,
    /// Core config JSON, passed through untouched.
    pub payload: Vec<u8>,
}

fn round32(n: usize) -> usize {
    (n + 31) & !31
}

/// Same running checksum as `vcf_checksum_bytes` (sum = sum * 31 + byte).
fn checksum_bytes(mut sum: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        sum = sum.wrapping_mul(31).wrapping_add(b as u32);
    }
    sum
}

/// Header (bytes 12..16 skipped) then `payload`, like `vcf_launch_checksum`.
fn checksum(header: &[u8], payload: &[u8]) -> u32 {
    let sum = checksum_bytes(0, &header[..OFF_CHECKSUM]);
    let sum = checksum_bytes(sum, &header[OFF_CHECKSUM + 4..HEADER_SIZE]);
    checksum_bytes(sum, payload)
}

/// Same rule as `vcf_path_ok`, with the extra restrictions that keep paths
/// sane on FAT: device-relative (leading `/`), names something, fits the
/// field with its NUL, no NUL/control characters, no backslashes.
pub fn validate_path(what: &str, p: &str) -> Result<(), VcError> {
    let bad = |why: &str| Err(VcError::Forwarder(format!("{what} path {p:?}: {why}")));
    if !p.starts_with('/') {
        return bad("must start with '/' (it is relative to the device root, without \"sd:\"/\"usb:\")");
    }
    if p.len() < 2 {
        return bad("must name a file");
    }
    if p.len() >= PATH_FIELD {
        return bad(&format!("too long ({} bytes, max {})", p.len(), PATH_FIELD - 1));
    }
    if p.chars().any(|c| c.is_control() || c == '\\') {
        return bad("contains a control character or backslash");
    }
    Ok(())
}

impl LaunchImage {
    /// Serialize to the exact bytes stored as WAD content 2. The result is a
    /// multiple of 32 bytes long.
    pub fn to_bytes(&self) -> Result<Vec<u8>, VcError> {
        validate_path("core", &self.core_path)?;
        validate_path("rom", &self.rom_path)?;
        let total = HEADER_SIZE + round32(self.payload.len());
        if total > MAX_TOTAL {
            return Err(VcError::Forwarder(format!(
                "launch payload too large: {} bytes (image would be {total}, max {MAX_TOTAL})",
                self.payload.len()
            )));
        }

        let mut buf = vec![0u8; total];
        buf[0..4].copy_from_slice(&LAUNCH_MAGIC.to_be_bytes());
        buf[4..8].copy_from_slice(&LAUNCH_VERSION.to_be_bytes());
        buf[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_be_bytes());
        // 12..16 checksum, filled in below; 16..20 flags = 0
        buf[20..24].copy_from_slice(&(self.device as u32).to_be_bytes());
        buf[24..28].copy_from_slice(&(self.payload.len() as u32).to_be_bytes());
        buf[OFF_CORE_PATH..OFF_CORE_PATH + self.core_path.len()]
            .copy_from_slice(self.core_path.as_bytes());
        buf[OFF_ROM_PATH..OFF_ROM_PATH + self.rom_path.len()]
            .copy_from_slice(self.rom_path.as_bytes());
        buf[HEADER_SIZE..HEADER_SIZE + self.payload.len()].copy_from_slice(&self.payload);

        let sum = checksum(&buf[..HEADER_SIZE], &self.payload);
        buf[OFF_CHECKSUM..OFF_CHECKSUM + 4].copy_from_slice(&sum.to_be_bytes());
        Ok(buf)
    }

    /// Parse and fully validate an image, applying the same checks as
    /// `vcf_launch_validate` on the Wii side.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VcError> {
        let err = |m: String| Err(VcError::Forwarder(format!("launch image: {m}")));
        if bytes.len() < HEADER_SIZE {
            return err(format!("{} bytes is shorter than the {HEADER_SIZE}-byte header", bytes.len()));
        }
        let u32_at = |o: usize| u32::from_be_bytes(bytes[o..o + 4].try_into().unwrap());

        if u32_at(0) != LAUNCH_MAGIC {
            return err("bad magic".into());
        }
        if u32_at(4) != LAUNCH_VERSION {
            return err(format!("unsupported version {}", u32_at(4)));
        }
        if u32_at(8) as usize != HEADER_SIZE {
            return err(format!("header size {} (expected {HEADER_SIZE})", u32_at(8)));
        }
        if u32_at(16) != 0 {
            return err(format!("unknown flags {:#x}", u32_at(16)));
        }
        let device = match u32_at(20) {
            0 => Device::Sd,
            1 => Device::Usb,
            d => return err(format!("bad device {d}")),
        };
        let payload_len = u32_at(24) as usize;
        if HEADER_SIZE + round32(payload_len) > MAX_TOTAL {
            return err(format!("payload size {payload_len} too large"));
        }
        if bytes.len() < HEADER_SIZE + payload_len {
            return err(format!(
                "truncated: header says {payload_len} payload bytes, only {} present",
                bytes.len() - HEADER_SIZE
            ));
        }

        let read_path = |off: usize, what: &str| -> Result<String, VcError> {
            let field = &bytes[off..off + PATH_FIELD];
            let n = field
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| VcError::Forwarder(format!("launch image: {what} path not NUL-terminated")))?;
            let s = std::str::from_utf8(&field[..n])
                .map_err(|_| VcError::Forwarder(format!("launch image: {what} path is not UTF-8")))?;
            validate_path(what, s)?;
            Ok(s.to_owned())
        };
        let core_path = read_path(OFF_CORE_PATH, "core")?;
        let rom_path = read_path(OFF_ROM_PATH, "rom")?;

        let payload = &bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
        if u32_at(OFF_CHECKSUM) != checksum(&bytes[..HEADER_SIZE], payload) {
            return err("checksum mismatch".into());
        }
        Ok(LaunchImage { device, core_path, rom_path, payload: payload.to_vec() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs shared with forwarder/tests/test_launch.c.
    fn golden_image() -> LaunchImage {
        LaunchImage {
            device: Device::Sd,
            core_path: "/vcforge/cores/gb.dol".into(),
            rom_path: "/vcforge/roms/test.gb".into(),
            payload: br#"{"a":1}"#.to_vec(),
        }
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// The C side (built for big-endian PPC, run under qemu) wrote this file.
    /// If this fails, the Rust and C layouts have drifted apart.
    #[test]
    fn matches_the_c_golden_image_byte_for_byte() {
        let golden = include_str!("../../../forwarder/tests/golden_launch.txt");
        let mut lines = golden.lines();
        let want_sum = lines.next().unwrap().strip_prefix("checksum=").unwrap();
        let want_hex = lines.next().unwrap();

        let bytes = golden_image().to_bytes().unwrap();
        assert_eq!(hex(&bytes), want_hex);
        assert_eq!(hex(&bytes[12..16]), want_sum);
        assert_eq!(bytes.len(), HEADER_SIZE + 32);
    }

    #[test]
    fn round_trips() {
        let img = LaunchImage {
            device: Device::Usb,
            core_path: "/apps/vcforge/cores/gba.dol".into(),
            rom_path: "/roms/Some Game (USA).gba".into(),
            payload: vec![0xAB; 1000],
        };
        let bytes = img.to_bytes().unwrap();
        assert_eq!(bytes.len() % 32, 0);
        assert_eq!(LaunchImage::from_bytes(&bytes).unwrap(), img);
    }

    #[test]
    fn empty_payload_is_just_the_header() {
        let mut img = golden_image();
        img.payload.clear();
        let bytes = img.to_bytes().unwrap();
        assert_eq!(bytes.len(), HEADER_SIZE);
        assert_eq!(LaunchImage::from_bytes(&bytes).unwrap(), img);
    }

    #[test]
    fn bad_paths_are_rejected_on_write() {
        for bad in ["relative/x.gb", "/", "", "sd:/x.gb", "/a\\b.gb", "/a\nb.gb"] {
            let mut img = golden_image();
            img.rom_path = bad.into();
            let r = img.to_bytes();
            // "sd:/x.gb" is not absolute-style and is caught by the leading '/' rule
            assert!(r.is_err(), "{bad:?} should be rejected");
        }
        let mut img = golden_image();
        img.core_path = format!("/{}", "a".repeat(PATH_FIELD - 1)); // 256 bytes: no room for the NUL
        assert!(img.to_bytes().is_err());
        img.core_path = format!("/{}", "a".repeat(PATH_FIELD - 2)); // 255 bytes: fits
        assert!(img.to_bytes().is_ok());
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let mut img = golden_image();
        img.payload = vec![0; MAX_TOTAL - HEADER_SIZE]; // exactly fits
        assert!(img.to_bytes().is_ok());
        img.payload = vec![0; MAX_TOTAL - HEADER_SIZE + 1];
        assert!(img.to_bytes().is_err());
    }

    #[test]
    fn corruption_is_detected_on_read() {
        let good = golden_image().to_bytes().unwrap();
        assert!(LaunchImage::from_bytes(&good).is_ok());

        let mut b = good.clone();
        b[0] ^= 1;
        assert!(LaunchImage::from_bytes(&b).unwrap_err().to_string().contains("magic"));

        let mut b = good.clone();
        b[HEADER_SIZE] ^= 1; // payload byte
        assert!(LaunchImage::from_bytes(&b).unwrap_err().to_string().contains("checksum"));

        let mut b = good.clone();
        b[OFF_ROM_PATH + 3] ^= 1; // path byte
        assert!(LaunchImage::from_bytes(&b).unwrap_err().to_string().contains("checksum"));

        let mut b = good.clone();
        b[23] = 2; // device
        assert!(LaunchImage::from_bytes(&b).unwrap_err().to_string().contains("device"));

        assert!(LaunchImage::from_bytes(&good[..HEADER_SIZE - 1]).is_err());
        assert!(LaunchImage::from_bytes(&good[..HEADER_SIZE]).unwrap_err().to_string().contains("truncated"));
    }

    #[test]
    fn device_parses_from_cli_text() {
        assert_eq!("SD".parse::<Device>().unwrap(), Device::Sd);
        assert_eq!("usb".parse::<Device>().unwrap(), Device::Usb);
        assert!("nand".parse::<Device>().is_err());
    }
}
