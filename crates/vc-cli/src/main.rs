use clap::Parser;
use std::path::PathBuf;
use vc_core::config::{InputDeviceId, SaveTarget, VcConfig};
use vc_core::registry::{find_core, load_registry};
use vc_core::wad::{build_wad, WadBuildRequest};

/// Build a channel-style WAD from a ROM and a registered core.
#[derive(Parser, Debug)]
#[command(name = "vc-cli")]
struct Args {
    /// Path to registry.json (defaults to ../../cores/registry.json)
    #[arg(long, default_value = "cores/registry.json")]
    registry: PathBuf,

    /// Core id from the registry, e.g. "nes", "snes"
    #[arg(long)]
    core: String,

    /// Path to the ROM file the user sourced themselves
    #[arg(long)]
    rom: PathBuf,

    /// Optional cover art for the banner
    #[arg(long)]
    cover: Option<PathBuf>,

    /// Channel title as shown on the Wii Menu
    #[arg(long)]
    title: String,

    /// Output WAD path
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let cores = load_registry(&args.registry)?;
    let core = find_core(&cores, &args.core)?;

    let cover_bytes = args.cover.as_ref().map(std::fs::read).transpose()?;

    // Placeholder title id / title key allocation — real version tracks
    // allocated ids in a local database to avoid collisions across builds.
    let title_id: [u8; 8] = [0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD, 0xBE, 0xEF];
    let title_key: [u8; 16] = [0u8; 16];

    let config = VcConfig {
        console_id: 0,
        input_device: InputDeviceId::ClassicController,
        button_map: [0u8; 16],
        save_target: SaveTarget::NandSavePartition,
        video_mode: 0,
    };

    let wad_bytes = build_wad(WadBuildRequest {
        core,
        rom_path: &args.rom,
        cover_art: cover_bytes.as_deref(),
        title: args.title,
        config,
        title_id,
        title_key,
    })?;

    std::fs::write(&args.output, wad_bytes)?;
    println!("wrote {}", args.output.display());
    Ok(())
}
