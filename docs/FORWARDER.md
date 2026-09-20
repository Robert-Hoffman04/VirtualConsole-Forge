# Forwarder mode

A forwarder WAD contains **no ROM and no emulator core**. It holds one generic,
precompiled loader (`main.dol`) and a small configuration file (`launch.cfg`).
When the channel is launched the loader reads `launch.cfg`, loads the named
core DOL from the SD card or USB drive, and starts it, handing it the ROM path
and the core's settings. The same `main.dol` is used for every channel; only
`launch.cfg` differs.

Hand-off mechanism: **Option B** of the forwarder spec (our own launch struct at
a fixed address, not `argv`). Code: `forwarder/` (C, devkitPPC) and
`crates/vc-core/src/{launch,forwarder}.rs`.

```
vc-cli --forwarder --core gb --device usb --device-rom /vcforge/roms/game.gb \
       --title "My Game" --output my_game.wad
```

## WAD layout

| Index | Content      | Notes |
|-------|--------------|-------|
| 0     | `banner.bin` | System Menu reads content 0 for the banner |
| 1     | `main.dol`   | TMD boot index = 1. Byte-identical in every forwarder WAD |
| 2     | `launch.cfg` | a launch image (below) |

The legacy layout (DOL at 0, banner at 2) is untouched; `wad::assemble_wad` is
shared by both and takes the boot index as a parameter.

## Runtime flow

```
System Menu -> main.dol (loader, normal libogc program)
  1. video + console
  2. ES_OpenContent(2): read launch.cfg, validate (magic/version/size/device/paths/checksum)
  3. mount sd:/usb: per launch.cfg, read core DOL into the stage region
  4. parse the DOL, check no section lands on stage/booter/launch regions
  5. copy booter + launch image into place, flush caches
  6. unmount, SYS_ResetSystem(SYS_SHUTDOWN), IRQ_Disable, jump to booter
booter (freestanding, own stack)
  7. copy each DOL section to its load address, dcbst/icbi, jump to entry
core
  8. FIRST thing in main(): vcf_launch_take() copies the launch image out and consumes it
```

Any failure before step 6 prints a message and returns to the System Menu.

## The launch image (`forwarder/include/vcf_launch.h`, `launch.rs`)

544-byte header + payload padded to 32. Big-endian, no pointers.

| Offset | Field | |
|---|---|---|
| 0x000 | magic `'LNCH'`, version (1), header size (544) | |
| 0x00C | checksum | `sum*31+byte` over header (this field skipped) + payload |
| 0x010 | flags | must be 0 |
| 0x014 | device | 0 = SD, 1 = USB |
| 0x018 | payload size | |
| 0x020 | `core_path[256]` | device-relative, leading `/`, e.g. `/vcforge/cores/gb.dol` |
| 0x120 | `rom_path[256]` | same |
| 0x220 | payload | the core config JSON ([CONFIG_FORMAT.md](CONFIG_FORMAT.md)), untouched |

Paths are relative to the device root, without `sd:`/`usb:`; the device field
says which. `launch.cfg` **is** this image, so the loader copies it to the
hand-off address unchanged. Rust and C are checked against the same golden image
(`forwarder/tests/golden_launch.txt`, produced by the C code as big-endian PPC).

## Memory map (`vcf_memmap.h`)

| Address | Size | |
|---|---|---|
| `0x92000000` | up to 15 MiB | raw bytes of the core DOL |
| `0x92F00000` | 64 KiB | booter code (bottom) + stack (top) |
| `0x93300800` | up to 64 KiB | launch image |

The linker script, `crt0.S`, loader and booter all take these from one header.
The loader refuses a core whose sections or bss overlap any of them (compared on
physical addresses, so uncached mirrors are caught).

## Contract for cores

A core is a normal DOL that includes `vcf_launch_client.h` and, first thing in
`main()` (before any big allocation):

```c
static uint8_t g_launch[VCF_LAUNCH_MAX] __attribute__((aligned(32)));
const VcfLaunchInfo *li = NULL;
if (vcf_launch_take(g_launch, sizeof g_launch) == VCF_OK)
    li = (const VcfLaunchInfo *)g_launch;   /* else: no launch image, use defaults */
/* li->rom_path, li->device, vcf_launch_payload(li) / li->payload_size = config JSON */
```

The core re-mounts the device itself (the loader unmounts before jumping) and
reads its config from the payload instead of WAD content 3.

**Only project-owned (`Bundled`) cores can be forwarded.** Nintendo's donor-ripped
VC emulators never read a launch image, so `--forwarder` refuses them (11 of the
15 registry entries today; `gb`, `gbc`, `gba`, `nds` work). Donor builds keep the
regular `build_wad` path. Whether donor systems get a forwarder story at all is
an open decision.

## Desktop app

The Source step has a **Build mode** choice: *Standard* (the usual build) or *Forwarder*.
In forwarder mode:

- **Storage device** (SD card or USB drive) and **ROM location on the device** (e.g.
  `/vcforge/roms/game.gb`) are asked for. Picking a local ROM file is optional; if you do,
  its file name pre-fills the device path (unless you already typed one). The ROM is never
  read or embedded.
- **Core location on the device** is optional and defaults to `/vcforge/cores/<core id>.dol`.
- The Build step gains the **loader DOL** (default `forwarder/prebuilt/main.dol`) and an
  optional **launch.cfg** path to save a copy. The controllers, bindings and options chosen
  on the Configuration step become the core config JSON carried in `launch.cfg`, exactly as
  `vc-cli --forwarder` does; the **Core config file** preview shows it.
- Official (donor-sourced) cores stay selectable but are explained and refused, since they
  can't read a launch image. Only unofficial cores can be forwarded.
- After a successful build the status lists what to copy onto the device (core DOL and ROM
  paths), as the CLI does. Nothing is copied automatically.

Backend: the `build_forwarder_command` Tauri command (same steps and order as
`vc-cli --forwarder`: `launch.cfg` is saved before the WAD stage, so it survives the
unfinished parts), and `list_cores` reports `forwardable` and `forwarder_core_path` per core.

## Building and testing

```
make -C forwarder booter    # any powerpc gcc: CROSS=powerpc-linux-gnu- make -C forwarder booter
make -C forwarder           # main.dol: needs devkitPPC + libogc + libfat
make -C forwarder install   # copies it to forwarder/prebuilt/main.dol (what vc-cli --forwarder reads)
make -C forwarder test      # layout/DOL tests, big-endian PPC under qemu-ppc
make -C forwarder check-cfg CFG=launch.cfg   # validate a vc-cli --launch-cfg-out file with the loader's C checks
cargo test -p vc-core
```

## Differences from the spec (from reading current libogc)

- **Step 8 does not build.** `__lwp_thread_stopmultitasking` and
  `__exception_closeall` no longer exist: libogc's threading and exception code
  moved into the new `tuxedo` kernel. The loader instead calls
  `SYS_ResetSystem(SYS_SHUTDOWN)`, `IRQ_Disable()`, then calls the booter directly.
  The target's own startup reinstalls exception vectors.
- **libogc `malloc` spills into MEM2** (`MALLOC_MEM2` defaults to 1), so a big
  heap in a core can be handed the launch image's memory. Hence "take it first".
  The loader sets `MALLOC_MEM2 = 0` so its own heap stays in MEM1.
- ELF cores are rejected for now (the spec's booter handles ELF; DOL only here).

## Not verified / open

- **The loader has not been built with devkitPPC or run** in Dolphin or on
  hardware; it was only syntax-checked against the real libogc/libfat headers.
  The booter was built and disassembled, not executed. The DOL and launch logic
  is tested as big-endian PPC under qemu; the jump sequence is not.
- Reading `launch.cfg` through `ES_OpenContent` in a channel launched from the
  System Menu, and USB spin-up timing (40 x 250 ms retry).
- Whether the hand-off address is safe against a core's allocator on hardware
  (`SYS_GetArena2Hi()` was not compared; the spec flagged the same).
- The **WAD itself can't be installed yet**: `build_wad`/`assemble_wad` still
  have the unfinished container header, fakesigning and title-key encryption
  (see DESIGN.md open TODOs), and `banner::build_banner` is a stub, so
  `vc-cli --forwarder` currently stops at the banner. `--launch-cfg-out` writes
  `launch.cfg` independently of those.
- Content 0 = banner / boot index 1 is my reading of the channel WAD
  convention; confirm against a reference channel WAD.
- The desktop app's forwarder mode (below) has been compiled and exercised against a
  stand-in backend, but not run end to end: the WAD stage it calls is the unfinished one.
- Nothing copies the core DOL or ROM onto the SD card yet; `vc-cli` only prints
  where they must go.
