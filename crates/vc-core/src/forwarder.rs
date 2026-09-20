//! Forwarder-mode WAD: a generic, precompiled `main.dol` plus a small
//! `launch.cfg`, with no ROM and no emulator core inside the WAD.
//!
//! ```text
//! content 0  banner.bin   generated from cover art + title
//! content 1  main.dol     the generic loader (forwarder/), byte-identical for every channel
//! content 2  launch.cfg   a launch image: device + core path + ROM path + core config JSON
//! ```
//!
//! At runtime the loader reads content 2, mounts the SD card or USB drive it
//! names, stages the core DOL from there, and hands the same launch image to
//! the core at a fixed address (Option B in docs/FORWARDER.md).
//!
//! Only project-owned (`CoreSource::Bundled`) cores can be launched this way:
//! the core has to read the launch image itself, which Nintendo's donor-ripped
//! Virtual Console emulators never will. Donor builds keep using
//! `wad::build_wad`.

use crate::banner::{build_banner, BannerInput};
use crate::error::VcError;
use crate::launch::{validate_path, Device, LaunchImage, CFG_CONTENT_INDEX};
use crate::registry::{CoreDefinition, CoreSource};
use crate::wad::assemble_wad;

use std::fs;
use std::path::Path;

/// WAD content index of the banner (the System Menu reads content 0).
pub const BANNER_INDEX: u16 = 0;
/// WAD content index of the loader DOL; also the TMD boot index.
pub const BOOT_INDEX: u16 = 1;

/// Where a core DOL is expected on the device unless told otherwise.
pub fn default_core_path(core: &CoreDefinition) -> String {
    format!("/vcforge/cores/{}.dol", core.id)
}

pub struct ForwarderWadRequest<'a> {
    pub core: &'a CoreDefinition,
    /// The precompiled generic loader (`forwarder/prebuilt/main.dol`).
    pub loader_dol: &'a Path,
    pub title: String,
    pub cover_art: Option<&'a [u8]>,
    /// Device the user will put the core and ROM on.
    pub device: Device,
    /// Core DOL location on that device; `None` uses `default_core_path`.
    pub core_path: Option<String>,
    /// ROM location on that device. The ROM itself is not read or embedded.
    pub rom_path: String,
    /// The core config file (see `coreconfig`), carried as the launch payload.
    pub core_config: Vec<u8>,
    pub title_id: [u8; 8],
    pub title_key: [u8; 16],
}

/// Build the launch image for a request, enforcing the forwarder rules.
pub fn build_launch_image(req: &ForwarderWadRequest) -> Result<LaunchImage, VcError> {
    if !matches!(req.core.core_source, CoreSource::Bundled { .. }) {
        return Err(VcError::Forwarder(format!(
            "{} is a donor-sourced core. Forwarder mode needs a core that reads the launch \
             image, which official Virtual Console emulators don't; use the regular build \
             for this system",
            req.core.system
        )));
    }

    let core_path = req.core_path.clone().unwrap_or_else(|| default_core_path(req.core));
    validate_path("core", &core_path)?;
    validate_path("rom", &req.rom_path)?;

    let ext = req.rom_path.rsplit('.').next().unwrap_or("").to_lowercase();
    if !req.rom_path.contains('.') || !req.core.valid_extensions.iter().any(|e| *e == ext) {
        return Err(VcError::InvalidRom(format!(
            "ROM path {:?}: extension not valid for {} (expected one of {:?})",
            req.rom_path, req.core.system, req.core.valid_extensions
        )));
    }

    Ok(LaunchImage {
        device: req.device,
        core_path,
        rom_path: req.rom_path.clone(),
        payload: req.core_config.clone(),
    })
}

/// Cheap "is this plausibly a DOL" check on the loader file, so a wrong path
/// (an ELF, a WAD, a text file) fails here rather than on a console.
fn check_loader(bytes: &[u8]) -> Result<(), VcError> {
    if bytes.len() < 0x100 {
        return Err(VcError::Forwarder(format!("loader is {} bytes, too small to be a DOL", bytes.len())));
    }
    if bytes.starts_with(b"\x7FELF") {
        return Err(VcError::Forwarder("loader is an ELF; convert it to a DOL (elf2dol) first".into()));
    }
    let entry = u32::from_be_bytes(bytes[0xE0..0xE4].try_into().unwrap());
    if entry == 0 {
        return Err(VcError::Forwarder("loader has no entry point; is it really a DOL?".into()));
    }
    Ok(())
}

/// The three plaintext contents in index order: `(index, type, bytes)`.
pub fn forwarder_contents(banner: Vec<u8>, loader: Vec<u8>, launch_cfg: Vec<u8>) -> Vec<(u16, u16, Vec<u8>)> {
    vec![
        (BANNER_INDEX, 0x0001, banner),
        (BOOT_INDEX, 0x0001, loader),
        (CFG_CONTENT_INDEX, 0x0001, launch_cfg),
    ]
}

/// Assemble a forwarder WAD. Same output stage as `wad::build_wad`
/// (including its unfinished pieces: container header, real signing,
/// title-key encryption), with boot index 1.
pub fn build_forwarder_wad(req: ForwarderWadRequest) -> Result<Vec<u8>, VcError> {
    let launch = build_launch_image(&req)?.to_bytes()?;

    let loader = fs::read(req.loader_dol).map_err(|e| {
        VcError::Forwarder(format!(
            "cannot read the loader DOL {}: {e} (build it with `make -C forwarder install`)",
            req.loader_dol.display()
        ))
    })?;
    check_loader(&loader)?;

    let banner = build_banner(&BannerInput {
        cover_art: req.cover_art,
        title: req.title.clone(),
    })?;

    Ok(assemble_wad(
        req.title_id,
        req.title_key,
        req.core.ios_version,
        BOOT_INDEX,
        forwarder_contents(banner, loader, launch),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::load_registry;
    use std::path::PathBuf;

    fn registry() -> Vec<CoreDefinition> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../cores/registry.json");
        load_registry(&p).unwrap()
    }

    fn req<'a>(core: &'a CoreDefinition, rom: &str) -> ForwarderWadRequest<'a> {
        ForwarderWadRequest {
            core,
            loader_dol: Path::new("unused"),
            title: "Test".into(),
            cover_art: None,
            device: Device::Usb,
            core_path: None,
            rom_path: rom.into(),
            core_config: br#"{"format":"vcforge-config"}"#.to_vec(),
            title_id: [0, 1, 0, 1, 1, 2, 3, 4],
            title_key: [0; 16],
        }
    }

    fn core<'a>(cores: &'a [CoreDefinition], id: &str) -> &'a CoreDefinition {
        cores.iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn launch_image_carries_paths_device_and_config() {
        let cores = registry();
        let gb = core(&cores, "gb");
        let img = build_launch_image(&req(gb, "/roms/Tetris.GB")).unwrap();
        assert_eq!(img.device, Device::Usb);
        assert_eq!(img.core_path, "/vcforge/cores/gb.dol");
        assert_eq!(img.rom_path, "/roms/Tetris.GB");
        assert_eq!(img.payload, br#"{"format":"vcforge-config"}"#);

        // and it survives the byte round trip the loader will perform
        let back = LaunchImage::from_bytes(&img.to_bytes().unwrap()).unwrap();
        assert_eq!(back, img);
    }

    #[test]
    fn custom_core_path_is_used() {
        let cores = registry();
        let mut r = req(core(&cores, "gba"), "/roms/a.gba");
        r.core_path = Some("/apps/mine/gba-core.dol".into());
        assert_eq!(build_launch_image(&r).unwrap().core_path, "/apps/mine/gba-core.dol");
    }

    #[test]
    fn donor_cores_are_refused_with_an_explanation() {
        let cores = registry();
        let e = build_launch_image(&req(core(&cores, "nes"), "/roms/a.nes")).unwrap_err();
        assert!(e.to_string().contains("donor-sourced"), "{e}");
    }

    #[test]
    fn every_bundled_core_can_be_forwarded_and_no_donor_core_can() {
        for c in registry() {
            let ext = c.valid_extensions[0].clone();
            let r = build_launch_image(&req(&c, &format!("/roms/game.{ext}")));
            assert_eq!(
                r.is_ok(),
                matches!(c.core_source, CoreSource::Bundled { .. }),
                "core {}",
                c.id
            );
        }
    }

    #[test]
    fn rom_extension_and_paths_are_checked() {
        let cores = registry();
        let gb = core(&cores, "gb");
        for bad in ["/roms/game.nes", "/roms/game", "roms/game.gb", "/roms/"] {
            assert!(build_launch_image(&req(gb, bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn contents_are_banner_then_loader_then_config() {
        let c = forwarder_contents(vec![1], vec![2], vec![3]);
        let idx: Vec<u16> = c.iter().map(|(i, _, _)| *i).collect();
        assert_eq!(idx, vec![0, 1, 2]);
        assert_eq!(BOOT_INDEX, 1);
        assert_eq!(CFG_CONTENT_INDEX, 2); // must match VCF_CFG_CONTENT_INDEX in vcf_launch.h
        assert_eq!(c[CFG_CONTENT_INDEX as usize].2, vec![3]);
    }

    #[test]
    fn loader_sanity_check_rejects_non_dols() {
        assert!(check_loader(&[0u8; 16]).is_err());
        let mut elf = vec![0u8; 0x200];
        elf[..4].copy_from_slice(b"\x7FELF");
        assert!(check_loader(&elf).is_err());
        let mut dol = vec![0u8; 0x200];
        assert!(check_loader(&dol).is_err()); // entry point 0
        dol[0xE0..0xE4].copy_from_slice(&0x8000_3100u32.to_be_bytes());
        assert!(check_loader(&dol).is_ok());
    }

    #[test]
    fn boot_index_reaches_the_tmd() {
        use crate::tmd::{build_tmd, TmdBuildRequest};
        let tmd = |boot_index| {
            build_tmd(&TmdBuildRequest {
                title_id: [0; 8],
                title_version: 0,
                ios_version: 58,
                boot_index,
                contents: vec![],
            })
        };
        // boot index sits right before the 2-byte content count (see tmd.rs layout)
        let (a, b) = (tmd(0), tmd(1));
        let at = a.len() - 4;
        assert_eq!(&a[at..at + 2], &[0, 0]);
        assert_eq!(&b[at..at + 2], &[0, 1]);
    }

    #[test]
    fn missing_loader_gives_a_helpful_error() {
        let cores = registry();
        let mut r = req(core(&cores, "gb"), "/roms/a.gb");
        r.loader_dol = Path::new("/definitely/not/here/main.dol");
        let e = build_forwarder_wad(r).unwrap_err().to_string();
        assert!(e.contains("cannot read the loader DOL") && e.contains("make -C forwarder"), "{e}");
    }
}
