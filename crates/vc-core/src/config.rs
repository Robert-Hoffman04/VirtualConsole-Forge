//! Fixed-layout config blob embedded as its own content file inside the WAD
//! (alongside the DOL, ROM, and banner). The desktop tool writes this at
//! build time; the core reads it at boot via the same content-loading path
//! used for the ROM. No SD card, no runtime dependency — the WAD is fully
//! self-contained.
//!
//! Layout is fixed-size and C-struct-friendly on purpose: the Wii-side core
//! should be able to treat this as a plain memcpy target, not something it
//! needs a parser for.

use crate::error::VcError;

pub const CONFIG_MAGIC: u32 = 0x5643_4346; // 'VCCF'
pub const CONFIG_VERSION: u16 = 1;
pub const CONFIG_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputDeviceId {
    WiimoteSideways = 0,
    ClassicController = 1,
    Gamecube = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SaveTarget {
    None = 0,
    NandSavePartition = 1,
}

#[derive(Debug, Clone)]
pub struct VcConfig {
    pub console_id: u8,
    pub input_device: InputDeviceId,
    /// Slot -> physical button id. Slot ordering is core-defined; the
    /// desktop tool and the core agree on this via the registry's
    /// default_mappings key order.
    pub button_map: [u8; 16],
    pub save_target: SaveTarget,
    pub video_mode: u8,
}

impl VcConfig {
    /// Serialize into the exact fixed-size byte layout the core reads at boot.
    pub fn to_bytes(&self) -> [u8; CONFIG_SIZE] {
        let mut buf = [0u8; CONFIG_SIZE];
        buf[0..4].copy_from_slice(&CONFIG_MAGIC.to_be_bytes());
        buf[4..6].copy_from_slice(&CONFIG_VERSION.to_be_bytes());
        buf[6] = self.console_id;
        buf[7] = self.input_device as u8;
        buf[8..24].copy_from_slice(&self.button_map);
        buf[24] = self.save_target as u8;
        buf[25] = self.video_mode;
        // buf[26..64] reserved for future fields without bumping version
        buf
    }

    /// Parse and validate a config blob (used mainly by tests / the CLI's
    /// verify command to confirm round-trip correctness before packaging).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VcError> {
        if bytes.len() < CONFIG_SIZE {
            return Err(VcError::InvalidConfig(format!(
                "expected {CONFIG_SIZE} bytes, got {}",
                bytes.len()
            )));
        }
        let magic = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        if magic != CONFIG_MAGIC {
            return Err(VcError::InvalidConfig("bad magic".into()));
        }
        let version = u16::from_be_bytes(bytes[4..6].try_into().unwrap());
        if version != CONFIG_VERSION {
            return Err(VcError::InvalidConfig(format!(
                "unsupported config version {version}"
            )));
        }

        let input_device = match bytes[7] {
            0 => InputDeviceId::WiimoteSideways,
            1 => InputDeviceId::ClassicController,
            2 => InputDeviceId::Gamecube,
            other => return Err(VcError::InvalidConfig(format!("bad input_device {other}"))),
        };
        let save_target = match bytes[24] {
            0 => SaveTarget::None,
            1 => SaveTarget::NandSavePartition,
            other => return Err(VcError::InvalidConfig(format!("bad save_target {other}"))),
        };

        let mut button_map = [0u8; 16];
        button_map.copy_from_slice(&bytes[8..24]);

        Ok(VcConfig {
            console_id: bytes[6],
            input_device,
            button_map,
            save_target,
            video_mode: bytes[25],
        })
    }
}
