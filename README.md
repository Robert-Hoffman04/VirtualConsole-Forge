# VirtualConsole-Forge

A desktop tool for building channel-style Wii WADs from user-supplied ROMs
and community emulator cores — same packaging shape as Nintendo's Virtual
Console (DOL + ROM + banner, packed as a signed content archive), but built
around your own cores instead of injecting into Nintendo's official WADs.

See [`docs/DESIGN.md`](docs/DESIGN.md) for the pipeline, content layout,
and signing notes.

## Layout

```
crates/
  vc-core/    format/crypto/packing library — no UI dependency
  vc-cli/     CLI wrapper, doubles as the validation/test harness
  vc-tauri/   Tauri GUI shell
cores/        prebuilt core DOLs + registry.json (core plugin definitions)
docs/         design notes
```

## Status

Early skeleton. Module boundaries and struct layouts are in place; the
actual WAD container assembly, real signature-block layout, common-key
title-key encryption, and banner encoding are stubbed with `TODO`s — see
`docs/DESIGN.md#open-todos`.

## Building

```
cargo build --workspace
```

Requires a populated `cores/registry.json` entry and a corresponding DOL
for each core you want to package against — see `cores/registry.json` for
the expected shape.

## Legal / compatibility note

Users must source their own ROMs. Output WADs are "fakesigned" and require
a cIOS with the Trucha bug patch (the standard setup for installing
homebrew WADs) to install — there is no path to real Nintendo-signed
output without Nintendo's private signing key.
