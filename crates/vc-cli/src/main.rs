use clap::Parser;
use std::path::PathBuf;
use vc_core::config::{InputDeviceId, SaveTarget, VcConfig};
use vc_core::donor::KeyProvider;
use vc_core::coreconfig::{build_core_config, DeviceSetup};
use vc_core::options::parse_option_args;
use vc_core::registry::{find_core, load_registry, CoreSource, InputDevice};
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

    /// Path to a donor WAD (required if the selected core is donor-sourced —
    /// see `CoreSource::Donor` in the registry). Must be a WAD the user
    /// already legitimately owns; this tool never supplies one.
    #[arg(long)]
    donor: Option<PathBuf>,

    /// Path to a local file holding the user's own Wii common key (raw 16
    /// bytes or 32 hex chars). Required alongside --donor. Never has a
    /// default or built-in value — see `donor::KeyProvider`.
    #[arg(long)]
    keys: Option<PathBuf>,

    /// Core-specific option as ID=VALUE; repeat for several, e.g.
    /// `--opt expansion_pak=true --opt pak=rumble_pak`. Valid ids/values are
    /// the `options` listed for the core in the registry.
    #[arg(long = "opt", value_name = "ID=VALUE")]
    opts: Vec<String>,

    /// Controller to enable; repeat for several (classic_controller,
    /// wiimote_sideways, wiimote_nunchuk, gamecube). Defaults to
    /// classic_controller. Each gets the core's default bindings.
    #[arg(long = "controller", value_name = "DEVICE")]
    controllers: Vec<String>,

    /// Also write the unified core config file (JSON) to this path. Bundled
    /// (non-official) cores get this file embedded in the WAD as content 3.
    #[arg(long, value_name = "PATH")]
    config_out: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let cores = load_registry(&args.registry)?;
    let core = find_core(&cores, &args.core)?;

    let cover_bytes = args.cover.as_ref().map(std::fs::read).transpose()?;

    if matches!(core.core_source, CoreSource::Donor { .. }) && (args.donor.is_none() || args.keys.is_none()) {
        if let CoreSource::Donor { donor_label, .. } = &core.core_source {
            eprintln!(
                "{} is a donor-sourced core and requires both --donor and --keys.\n\
                 Supply: {donor_label}",
                core.system
            );
        }
        std::process::exit(1);
    }

    let key_provider = args.keys.as_deref().map(KeyProvider::from_file).transpose()?;

    // Placeholder title id / title key allocation — real version tracks
    // allocated ids in a local database to avoid collisions across builds.
    let title_id: [u8; 8] = [0x00, 0x01, 0x00, 0x01, 0xDE, 0xAD, 0xBE, 0xEF];
    let title_key: [u8; 16] = [0u8; 16];

    let option_values = parse_option_args(core, &args.opts)?;
    let devices: Vec<InputDevice> = if args.controllers.is_empty() {
        vec![InputDevice::ClassicController]
    } else {
        args.controllers.iter().map(|c| c.parse()).collect::<Result<_, String>>()?
    };
    let setups: Vec<DeviceSetup> = devices.iter().map(|&device| DeviceSetup { device, mapping: None }).collect();
    let core_config = build_core_config(core, &setups, &option_values)?;
    if let Some(path) = &args.config_out {
        std::fs::write(path, core_config.to_json())?;
        println!("wrote core config to {}", path.display());
    }

    let config = VcConfig {
        console_id: 0,
        input_device: InputDeviceId::from(devices[0]),
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
        core_config: matches!(core.core_source, CoreSource::Bundled { .. }).then(|| core_config.to_bytes()),
        title_id,
        title_key,
        donor_wad_path: args.donor.as_deref(),
        keys: key_provider.as_ref(),
    })?;

    std::fs::write(&args.output, wad_bytes)?;
    println!("wrote {}", args.output.display());
    Ok(())
}
