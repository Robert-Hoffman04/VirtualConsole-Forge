//! vc-core
//!
//! Pure format/packing/crypto library for building Wii-channel-style WADs
//! from user-supplied ROMs and community emulator cores.
//!
//! This crate has no UI dependency. Both `vc-cli` and `vc-tauri` are thin
//! wrappers that call into `wad::build_wad` as the single public entry point.
//!
//! Module map:
//! - `registry`  — loads core plugin definitions (system, DOL, defaults)
//! - `rom`       — ROM validation/normalization per core
//! - `config`    — VcConfig struct: baked-in button mapping + settings
//! - `options`   — per-core option validation and resolution
//! - `coreconfig` — the unified core config file (JSON) handed to emulator cores
//! - `banner`    — banner.bin generation from cover art + title text
//! - `tmd`       — Title Metadata structure/builder
//! - `ticket`    — Ticket structure/builder
//! - `crypto`    — AES-128-CBC content encryption, SHA-1 hashing, fakesigning
//! - `wad`       — top-level orchestrator: assembles everything into a WAD
//! - `error`     — unified error type

pub mod banner;
pub mod config;
pub mod crypto;
pub mod donor;
pub mod donor_store;
pub mod coreconfig;
pub mod error;
pub mod options;
pub mod registry;
pub mod rom;
pub mod ticket;
pub mod tmd;
pub mod wad;

pub use donor::KeyProvider;
pub use donor_store::DonorStore;
pub use error::VcError;
pub use registry::CoreDefinition;
pub use wad::{build_wad, WadBuildRequest};
