# Design notes

## Pipeline

```
[user picks ROM + core] -> [validate/normalize ROM] -> [obtain core DOL:
bundled, or ripped from a user-supplied donor WAD] -> [build banner +
config contents] -> [assemble content list, hash each] -> [build TMD +
ticket] -> [encrypt each content] -> [assemble WAD] -> [.wad ready to
install]
```

## Core sourcing: bundled vs. donor-ripped

Every `CoreDefinition` in `cores/registry.json` gets its DOL from one of
two places (`CoreSource` in `registry.rs`):

- **`Bundled`** — a DOL this project actually owns/built itself (e.g. an
  original from-scratch libretro-based port with a license that permits
  redistribution). Shipped directly under `cores/`.
- **`Donor`** — nothing is shipped. The DOL is ripped at runtime out of an
  official WAD the *user* supplies, one they already legitimately own.
  This is the default for anything derived from or resembling an official
  Nintendo VC emulator — redistributing that binary ourselves would mean
  redistributing Nintendo's copyrighted code, which is not something this
  project does under any circumstances.

This mirrors standard practice elsewhere in the Wii/GameCube homebrew and
emulation scene (Dolphin, for instance, never ships the Wii common key or
any Nintendo firmware — it requires the user to dump their own). Nothing
in this repository, including test fixtures, should ever contain a real
donor WAD, a real common key, or any other Nintendo-owned binary.

### Donor extraction flow (`donor.rs`)

1. User supplies a WAD file for a title matching the core's
   `CoreSource::Donor { title_id, .. }` requirement (via CLI `--donor` or
   the GUI's donor-import screen), plus a local key file for
   `KeyProvider::from_file` (their own dumped Wii common key — this tool
   contains no key material and no way to acquire one).
2. `donor::parse_donor_wad` reads the donor's header/ticket/TMD/content
   table. This is a reader, not a Nintendo-signature validator — we trust
   the user's own dump rather than attempting to verify Nintendo's
   signature, since we don't have Nintendo's public key material either.
3. `donor::verify_donor_matches` checks the donor's title id against what
   the selected core expects, so a wrong donor fails loudly instead of
   silently ripping the wrong system's core.
4. `donor::extract_core` decrypts the target content (the DOL, almost
   always content index 0) and verifies its SHA-1 against the TMD's
   recorded hash before returning it. Any mismatch — wrong donor, wrong
   key, corrupt dump — is a hard error, never a best-effort fallback.

### Local donor library (`donor_store.rs`)

`DonorStore` manages a directory (chosen by the CLI/GUI shell, typically a
per-user app-data path — never anything inside the source tree) where
donor WADs the user has already imported are kept, keyed by title id, so
they don't need to be re-selected on every build. The store starts empty
and is populated exclusively by `DonorStore::import`, which the user
triggers explicitly. See `.gitignore` for the backstop patterns that keep
any of this out of version control.

One known simplification worth revisiting: `CoreSource::Donor` currently
pins to one exact known-good donor title id per system, even though most
VC titles on a given system/region actually share the same core. A future
version could accept *any* WAD whose content matches an allow-listed set
of known-good hashes for that system, rather than a single exact title —
useful since not every user will own that one specific pinned title.

## Content layout inside each output WAD

| Index | Content            | Source                                   |
|-------|---------------------|-------------------------------------------|
| 0     | boot.dol            | bundled DOL, or ripped from a donor WAD via `donor::extract_core` |
| 1     | ROM data             | user-supplied, validated/normalized       |
| 2     | banner.bin           | generated from cover art + title          |
| 3     | config.bin (`VcConfig`) | generated from controller mapping UI  |

Controller mapping and other settings are baked into content #3 at build
time — no SD card dependency, no first-boot config screen. The output WAD
is self-contained and ready to install as-is regardless of whether its
core came from `cores/` or a donor rip. (A later "reconfigure without
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

- `vc-core` — all format/crypto/packing/donor-extraction logic, no UI
  dependency.
- `vc-cli` — thin CLI wrapper, doubles as the validation harness (diff
  output against known-good reference WADs when debugging install
  failures). Also the reference implementation for the `--donor`/`--keys`
  flags a GUI's donor-import screen needs to replicate.
- `vc-tauri` — GUI shell, calls the same `vc_core::wad::build_wad` entry
  point as the CLI, including the donor/keys path.

## Suggested build order

1. `crypto.rs` + `tmd.rs` + `ticket.rs` + `wad.rs` against one hardcoded
   test ROM + a `Bundled`-source test DOL — validate the output actually
   installs (real hardware or Dolphin with signature checks patched)
   before anything else.
2. `registry.rs` + `rom.rs` once the packer itself is proven.
3. `config.rs` — straightforward once the content-list machinery exists.
4. `banner.rs` — cosmetic, lowest risk, do whenever.
5. `donor.rs` + `donor_store.rs` — build `parse_donor_wad` by round-
   tripping against WADs `wad::build_wad` produces itself first (known
   plaintext, known key), before pointing it at a real official WAD.
6. `vc-cli` wraps all of the above for scripted testing.
7. `vc-tauri` last, once `vc-core`'s public API is stable.

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
- `donor::parse_donor_wad` — real WAD container parsing (header section
  sizes, cert chain skip, ticket/TMD offsets, content slicing). Currently
  a stub error.
- `donor::extract_core` — real title-key decryption (AES-CBC of the
  ticket's encrypted title key under the user's common key, IV = title id
  padded to 16 bytes). Currently passes the common key straight through
  as a placeholder, which is not correct.
- Title id allocation / collision tracking across multiple builds (not
  yet implemented in either `vc-cli` or `vc-tauri`).
- Donor "any title of this system" acceptance via an allow-listed hash
  set, instead of pinning to one exact donor title id (see note above).
