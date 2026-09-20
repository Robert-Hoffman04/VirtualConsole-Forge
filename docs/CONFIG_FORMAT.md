# Core config file (`vcforge-config` v1)

Every emulator core reads the **same JSON document** for its input bindings
and settings, so the common interface layer on the Wii side can be written
once. The layout is identical for every core; cores differ only in *which
keys they define* and in a dedicated `extra` section for things no other core
has.

For bundled (non-official) cores the file is embedded in the WAD as content
index 3. Donor-based (official Virtual Console) builds are unchanged and keep
the legacy 64-byte binary blob there. `vc-cli --config-out PATH` writes the
file standalone, and the desktop app shows the exact file under
**Build → Core config file**.

Ready-made samples for every core are in [`config-examples/`](config-examples/)
(checked by a test, so they never drift from the code).

## Layout

```json
{
  "format": "vcforge-config",
  "version": 1,
  "core":  { "id": "n64", "system": "Nintendo 64" },
  "input": {
    "device": "gamecube",
    "buttons": { "A": "gc_a", "Z": "gc_z", "Stick": "gc_lstick" },
    "ports": {},
    "rumble_strength": 80
  },
  "video":       { "filter": "smooth" },
  "audio":       {},
  "system":      {},
  "save":        {},
  "accessories": { "expansion_ram": true },
  "extra":       { "pak": "rumble_pak" }
}
```

- **All sections are always present**, possibly `{}`, so a reader can index
  `cfg["video"]` without checking. `extra` too.
- **Every setting the core defines is always written** with its effective
  value. The reader never needs to know defaults, and a missing key means the
  core doesn't define that setting.
- **Readers must ignore unknown keys** and check `format` and `version`. New
  keys are added without bumping `version`; it only changes for incompatible
  layout changes.
- Values are JSON booleans, strings or integers. Selects are written as their
  choice `value` string.

## Sections

| Section | Contents |
|---|---|
| `core` | `id` and `system` of the core the file was made for. |
| `input` | `device`, `buttons`, `ports`, plus input settings (`turbo`, `six_button`, `dual_controller`, `rumble_strength`, ...). |
| `video` | Timing, filtering, cropping, layout, palette. |
| `audio` | Audio processing. |
| `system` | What the emulated machine is (region, ...). |
| `save` | Save-data hardware. |
| `accessories` | Peripherals and expansions (multitap, memory card, RAM expansion, RTC). |
| `extra` | Settings only this core understands. |

## Input

`input.device` is `wiimote_sideways`, `classic_controller` or `gamecube`.

`input.buttons` maps each **console button** (as named by the core: `A`,
`Start`, `C-Up`, `Mode`, `Button 1`, `II`, ...) to a **physical input id**.

- Only buttons that exist under the current options are listed (see
  [Buttons that depend on options](#buttons-that-depend-on-options)).
  Unmapped buttons are omitted.
- Physical ids are prefixed by the device, which is validated:

| Device | Ids |
|---|---|
| `wiimote_sideways` | `wiimote_1`, `_2`, `_a`, `_b`, `_plus`, `_minus`, `_home`, `_dpad_{up,down,left,right}` |
| `classic_controller` | `classic_{a,b,x,y,l,r,zl,zr,plus,minus,home}`, `classic_dpad_{up,down,left,right}`, `classic_lstick`, `classic_rstick` (whole stick) and `classic_{l,r}stick_{up,down,left,right}` |
| `gamecube` | `gc_{a,b,x,y,z,l,r,start}`, `gc_dpad_{up,down,left,right}`, `gc_lstick`, `gc_cstick` and `gc_{l,c}stick_{up,down,left,right}` |

Analog console inputs (N64 `Stick`) map to a whole-stick id; digital ones that
live on a stick (N64 `C-Up`) map to a direction id.

### One physical controller, several emulated controllers

`input.buttons` is always emulated controller 1. When the core exposes more
emulated controllers that the same physical controller drives, the others are
listed in `input.ports`, keyed by port number. It is `{}` when there is only
one.

```json
"input": {
  "device": "classic_controller",
  "buttons": { "Stick": "classic_lstick", "Z": "classic_zl", "Start": "classic_plus" },
  "ports": { "2": { "buttons": { "Stick": "classic_rstick", "Z": "classic_zr", "A": "classic_a" } } },
  "dual_controller": true
}
```

The N64 core's **Dual controller mode** does this for games with a
two-controller dual-analog scheme, such as GoldenEye 007 and Perfect Dark,
where the original hardware needed one controller in each hand with a thumb
on each stick and an index finger on each Z trigger. With a Classic Controller
or GameCube pad, the left stick and left-hand inputs go to controller 1 and the
right stick (the C-stick on a GameCube pad) and right-hand inputs to controller
2. A physical input should not be bound twice, and the app warns about that
across controllers.

In the registry this is:

- `controller_ports`: `[ { "port": 2, "label": "Controller 2", "when": { "option": "dual_controller", "equals": true } } ]`.
  The port only exists while `when` holds (always, if omitted).
- `default_mappings` entries gain `port` (default 1) and an optional `when`.
  An entry whose `when` holds beats the unconditional one for the same
  device and port, so an option can swap the whole default layout. Dual mode
  needs this because the right stick stops being the C buttons.
  An empty id (`""`) means the button exists but starts out unmapped.
- Emulated-controller settings that repeat per port are ordinary options,
  such as `extra.pak2` (controller 2's pak slot), visible only in dual mode.

## Standard keys

Settings shared by several cores use the **same key, type and values
everywhere**. The registry rejects an option that uses one of these keys with a
different type or disallowed values, which is what keeps cores consistent.

| Key | Type | Values / range |
|---|---|---|
| `input.turbo` | toggle | |
| `input.six_button` | toggle | adds buttons, see below |
| `input.dual_controller` | toggle | one pad drives two emulated controllers, see below |
| `input.rumble_strength` | number | 0-100 (%) |
| `video.timing` | select | `ntsc`, `pal` |
| `video.crop_overscan` | toggle | |
| `video.filter` | select | `sharp`, `smooth` |
| `video.layout` | select | `stacked`, `side_by_side`, `single` |
| `video.palette` | select | core-defined |
| `video.color_correction` | toggle | |
| `audio.filter` | toggle | |
| `system.region` | select | `us`, `eu`, `jp` |
| `save.type` | select | core-defined |
| `accessories.multitap` | toggle | |
| `accessories.memory_card` | toggle | |
| `accessories.expansion_ram` | toggle | |
| `accessories.rtc` | toggle | |

The authoritative list is `STANDARD_KEYS` in `crates/vc-core/src/coreconfig.rs`.
For example, `system.region` is written by both the Master System and the
Genesis, and `accessories.multitap` by both TurboGrafx cores, so one piece of
shared code can handle each.

## Core-specific settings (`extra`)

Anything that only one core (or a system family) understands lives under
`extra`, for example `extra.slot2` (Nintendo DS Slot-2 accessory), `extra.pak`
(N64 pak slot) or `extra.mvs_credits` (Neo Geo). These are the "deviations":
the shared layer ignores `extra`, and the individual core reads what it needs.
If a second core later needs the same concept, promote it to a standard key.

## Buttons that depend on options

Some hardware variants add buttons. A registry entry can list
`button_requires`, mapping a button to an option value:

```json
"button_requires": { "X": { "option": "six_button", "equals": true } }
```

With the option off (the default) the button is absent from `input.buttons`
and from the UI; with it on it appears, bound to its default. Examples:
Genesis `X`, `Y`, `Z`, `Mode` and TurboGrafx `III` to `VI` need `six_button`.
Because the option is also written (`input.six_button`), a core can tell
which variant was requested.

## Adding or changing options (registry)

Each entry in a core's `options` list becomes one control in the app and one
key in the file:

```json
{ "id": "slot2", "label": "Slot-2 accessory", "kind": "select", "default": "none",
  "group": "Accessories", "description": "...",
  "choices": [ { "value": "none", "label": "None" },
               { "value": "memory_expansion", "label": "Memory Expansion Pak" } ] }
```

- `key` is where the value is written. Omit it to use `extra.<id>`; set it to a
  standard key to share the concept with other cores.
- `kind`: `toggle` (bool default), `select` (needs `choices`, default is a
  choice `value`) or `number` (`min`/`max` within 0-255, optional
  `step`/`unit`).
- `visible_when: { option, equals }` shows the option only while an
  **earlier** option has that value.
- `devices` restricts the option to certain controllers (e.g. rumble). In the
  app it is shown disabled with a hint.
- Options that don't apply (condition unmet, wrong controller) are written
  with their `default`, whatever the UI sent.
- Choice values are part of the contract with the core: renaming one is a
  breaking change.

Definitions are validated when the registry loads (unique ids and keys, valid
defaults, conditions that refer to earlier options, standard-key rules), so a
mistake fails at startup rather than mid-build.
