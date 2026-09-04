# VirtualConsole-Forge

A desktop tool for building channel-style Wii WADs from user-supplied ROMs
— same packaging shape as Nintendo's Virtual Console (DOL + ROM + banner,
packed as a signed content archive).

This project does not ship, bundle, or embed any Nintendo (or third-party)
copyrighted emulator core or key material. Each core in the registry is
either:

- **Bundled** — an original DOL this project actually owns/built itself
  (e.g. a from-scratch libretro-based port under a license that permits
  redistribution), shipped directly under `cores/`; or
- **Donor-sourced** — nothing shipped at all. The core is ripped at
  runtime out of an official WAD the *user* supplies, one they already
  legitimately own, using a Wii common key the user supplies from their
  own dump. Neither the donor WAD nor the key ever ships with this tool.

See [`docs/DESIGN.md`](docs/DESIGN.md) for the full pipeline, content
layout, donor-extraction flow, and signing notes.

## Layout

```
crates/
  vc-core/    format/crypto/packing/donor-extraction library — no UI dependency
  vc-cli/     CLI wrapper, doubles as the validation/test harness
  vc-tauri/
    src-tauri/          Rust backend: commands, dialog plugin, capabilities
    src/
      index.html        page structure only
      styles.css         all styling
      js/                one module per concern (see below)
cores/        registry.json (core plugin definitions) + any Bundled DOLs
docs/         design notes
```

### `vc-tauri/src/js` module map

| Module | Responsibility |
|---|---|
| `dom.js` | `$`/`$$` query helpers, `escapeHtml`, `basename` |
| `tauri.js` | `invoke()` wrapper, dialog-plugin `pickFile`/`pickSaveFile`, `assetUrl`, native drag-drop subscription |
| `state.js` | shared build-wizard state object |
| `dropzones.js` | wires `.dropzone` elements to real filesystem paths — see below |
| `cores.js` | loads the registry (`list_cores`), builds the core `<select>`, renders donor/keys fields |
| `summary.js` | keeps every summary/preview element in sync with state |
| `nav.js` | wizard step navigation + sidebar view switching |
| `titleid.js` | Title ID conflict-check prototype (local-only, see comments in the file) |
| `source.js` | Source-step field wiring (cover art dropzone, plain inputs) |
| `paths.js` | "Browse" buttons for the output/registry path inputs |
| `build.js` | calls `build_wad_command`, handles the reset button |
| `main.js` | entry point — wires everything together on load |

## Status

Early skeleton. Module boundaries and struct layouts are in place; the
actual WAD container assembly, real signature-block layout, common-key
title-key decryption, donor WAD parsing, banner encoding, and translating
the controller-mapping UI into `VcConfig.button_map` are stubbed with
`TODO`s — see `docs/DESIGN.md#open-todos`. App icons under
`crates/vc-tauri/src-tauri/icons/` are placeholders (required for Tauri's
`generate_context!()` macro to compile at all) — swap them for real
branding before shipping a built app.

The Tauri UI's file pickers (ROM, cover art, donor WAD, key file, output
path, registry path) are wired to real native dialogs and real native
drag-and-drop via `tauri-plugin-dialog` and the `tauri://drag-drop` event
— not the browser's HTML5 `<input type="file">`/`DragEvent`, which the
Tauri v2 webview doesn't populate with a usable path. See `dropzones.js`
and `tauri.js` for details, and `capabilities/default.json` +
`tauri.conf.json`'s `app.security.assetProtocol` for the permissions this
requires. `assetProtocol.scope` is set broadly (`"**"`) since cover art
can be selected from anywhere on disk — worth tightening if that becomes
a concern.

## Building

```
cargo build --workspace
```

`cores/registry.json` defines the available cores. For a `Donor`-sourced
core you don't need a bundled DOL — you'll instead supply a donor WAD and
a key file at build time (see below).

## Building a WAD (CLI)

```
vc-cli --core nes \
  --rom path/to/your/rom.nes \
  --title "My Game" \
  --output my_game.wad \
  --donor path/to/your/legally-owned-nes-vc-title.wad \
  --keys path/to/your/common-key.bin
```

`--donor` and `--keys` are only required for cores configured as
`CoreSource::Donor` in the registry — both must point to files you
sourced yourself. This tool has no built-in donor WAD and no built-in key,
by design.

## Legal / compatibility note

Users must source their own ROMs, and, for donor-sourced cores, their own
donor WADs and their own Wii common key dump — none of these are
included with, downloaded by, or inferred by this tool. Output WADs are
"fakesigned" and require a cIOS with the Trucha bug patch (the standard
setup for installing homebrew WADs) to install — there is no path to real
Nintendo-signed output without Nintendo's private signing key.