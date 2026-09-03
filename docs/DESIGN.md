# Design notes

## Pipeline

```
[user picks ROM + core] -> [validate/normalize ROM] -> [build banner + config
contents] -> [assemble content list, hash each] -> [build TMD + ticket]
-> [encrypt each content] -> [assemble WAD] -> [.wad ready to install]
```

## Content layout inside each WAD

| Index | Content            | Source                                   |
|-------|---------------------|-------------------------------------------|
| 0     | boot.dol            | prebuilt per-core, from `cores/<id>.dol`  |
| 1     | ROM data             | user-supplied, validated/normalized       |
| 2     | banner.bin           | generated from cover art + title          |
| 3     | config.bin (`VcConfig`) | generated from controller mapping UI  |

Controller mapping and other settings are baked into content #3 at build
time — no SD card dependency, no first-boot config screen. The WAD is
self-contained and ready to install as-is. (A later "reconfigure without
rebuild" in-core menu was considered and deliberately deferred — config
is kept as its own discrete content file specifically so that door stays
open cheaply if it's ever wanted.)

## Signing

Real Nintendo private signing keys aren't available to this project.
Output WADs are "fakesigned" — structurally valid signature blocks that
only pass verification on consoles running a cIOS with the Trucha bug
patch applied, which is the standard baseline for any Wii already set up
to install homebrew WADs. This should be stated clearly in end-user
documentation, not left as a silent assumption.

## Module -> crate mapping

- `vc-core` — all format/crypto/packing logic, no UI dependency.
- `vc-cli` — thin CLI wrapper, doubles as the validation harness (diff
  output against known-good reference WADs when debugging install
  failures).
- `vc-tauri` — GUI shell, calls the same `vc_core::wad::build_wad` entry
  point as the CLI.

## Suggested build order

1. `crypto.rs` + `tmd.rs` + `ticket.rs` + `wad.rs` against one hardcoded
   test ROM/DOL — validate the output actually installs (real hardware or
   Dolphin with signature checks patched) before anything else.
2. `registry.rs` + `rom.rs` once the packer itself is proven.
3. `config.rs` — straightforward once the content-list machinery exists.
4. `banner.rs` — cosmetic, lowest risk, do whenever.
5. `vc-cli` wraps all of the above for scripted testing.
6. `vc-tauri` last, once `vc-core`'s public API is stable.

## Open TODOs (marked in source)

- `crypto::sign_fakesigned` — needs the exact signature-block layout
  (type + RSA-2048-sized padding + issuer string) and a hash-starts-with-
  0x00 search loop, per Trucha bug patch requirements.
- `crypto` — title key encryption under the Wii common key (currently a
  passthrough placeholder in `wad.rs`).
- `wad::build_wad` — real WAD container assembly (header type/size
  fields, cert chain, 64-byte section alignment, footer). Current version
  concatenates sections without the container header/alignment.
- `banner::build_banner` — actual TPL/animation encoding; currently
  returns an error stub.
- Title id allocation / collision tracking across multiple builds (not
  yet implemented in either `vc-cli` or `vc-tauri`).
