// Pure data + helpers for the button-binding UI (no DOM access, so it can be
// unit-tested in isolation). mapping.js does the rendering.
//
// Vocabulary:
//   "console button"  - a button on the emulated system (NES "A", N64 "C-Up",
//                       Genesis "Mode", ...). Comes from the registry's
//                       `default_mappings` keys, so each system exposes its
//                       own set automatically.
//   "physical input"  - something on the Wii-side controller the user can
//                       bind it to. The ids (e.g. "classic_zl") are the same
//                       strings used as values in the registry.

const cap = (s) => s[0].toUpperCase() + s.slice(1);
const DIRS = ["up", "down", "left", "right"];

const dpad = (prefix) =>
  DIRS.map((d) => ({ id: `${prefix}_dpad_${d}`, label: `D-Pad ${cap(d)}`, group: "D-Pad" }));

/** A whole analog stick, plus its four directions. */
const stick = (id, label) => [
  { id, label, group: "Sticks" },
  ...DIRS.map((d) => ({ id: `${id}_${d}`, label: `${label} ${cap(d)}`, group: "Sticks" })),
];

const btn = (id, label) => ({ id, label, group: "Buttons" });

/** The controllers a build can enable, in display order. Ids match the registry and the core config file. */
export const DEVICES = [
  { id: "classic_controller", label: "Classic Controller", blurb: "Two sticks, shoulder buttons and triggers." },
  { id: "wiimote_sideways", label: "Wiimote \u2014 Sideways", blurb: "Held like an NES pad. Few buttons." },
  { id: "wiimote_nunchuk", label: "Wiimote + Nunchuk", blurb: "One analog stick plus the Wiimote's buttons." },
  { id: "gamecube", label: "GameCube Controller", blurb: "Control stick, C-stick and triggers." },
];

export const deviceLabel = (id) => DEVICES.find((d) => d.id === id)?.label ?? id;

/** Physical inputs available on each controller type (keys = the ids in DEVICES). */
export const PHYSICAL_INPUTS = {
  wiimote_sideways: [
    btn("wiimote_1", "1"),
    btn("wiimote_2", "2"),
    btn("wiimote_a", "A"),
    btn("wiimote_b", "B"),
    btn("wiimote_plus", "+"),
    btn("wiimote_minus", "\u2212"),
    btn("wiimote_home", "Home"),
    ...dpad("wiimote"),
  ],
  wiimote_nunchuk: [
    btn("wiimote_a", "A"),
    btn("wiimote_b", "B (trigger)"),
    btn("wiimote_1", "1"),
    btn("wiimote_2", "2"),
    btn("wiimote_plus", "+"),
    btn("wiimote_minus", "\u2212"),
    btn("wiimote_home", "Home"),
    btn("nunchuk_c", "Nunchuk C"),
    btn("nunchuk_z", "Nunchuk Z"),
    ...dpad("wiimote"),
    ...stick("nunchuk_stick", "Nunchuk Stick"),
  ],
  classic_controller: [
    btn("classic_a", "A"),
    btn("classic_b", "B"),
    btn("classic_x", "X"),
    btn("classic_y", "Y"),
    btn("classic_l", "L"),
    btn("classic_r", "R"),
    btn("classic_zl", "ZL"),
    btn("classic_zr", "ZR"),
    btn("classic_plus", "+"),
    btn("classic_minus", "\u2212"),
    btn("classic_home", "Home"),
    ...dpad("classic"),
    ...stick("classic_lstick", "Left Stick"),
    ...stick("classic_rstick", "Right Stick"),
  ],
  gamecube: [
    btn("gc_a", "A"),
    btn("gc_b", "B"),
    btn("gc_x", "X"),
    btn("gc_y", "Y"),
    btn("gc_z", "Z"),
    btn("gc_l", "L"),
    btn("gc_r", "R"),
    btn("gc_start", "Start"),
    ...dpad("gc"),
    ...stick("gc_lstick", "Control Stick"),
    ...stick("gc_cstick", "C-Stick"),
  ],
};

/**
 * Every controller can be enabled for every system: whether a game needs more
 * buttons than a controller has is the user's call. When the registry has no
 * explicit default map for a controller, one is derived from the core's
 * Classic Controller map using these tables (Classic input suffix -> input id
 * on the target controller; "" = no equivalent, so the button starts unmapped).
 */
const DPAD_TO = (prefix) => ({
  dpad_up: `${prefix}_dpad_up`, dpad_down: `${prefix}_dpad_down`,
  dpad_left: `${prefix}_dpad_left`, dpad_right: `${prefix}_dpad_right`,
});
const STICK_TO = (from, to) => ({
  [from]: to, [`${from}_up`]: `${to}_up`, [`${from}_down`]: `${to}_down`,
  [`${from}_left`]: `${to}_left`, [`${from}_right`]: `${to}_right`,
});

const DERIVE_FROM_CLASSIC = {
  gamecube: {
    a: "gc_a", b: "gc_b", x: "gc_x", y: "gc_y", l: "gc_l", r: "gc_r", zl: "gc_z", zr: "",
    plus: "gc_start", minus: "", home: "",
    ...DPAD_TO("gc"), ...STICK_TO("lstick", "gc_lstick"), ...STICK_TO("rstick", "gc_cstick"),
  },
  // Held sideways like an NES pad: 1 and 2 are the face buttons.
  wiimote_sideways: {
    a: "wiimote_2", b: "wiimote_1", x: "", y: "", l: "", r: "", zl: "", zr: "",
    plus: "wiimote_plus", minus: "wiimote_minus", home: "wiimote_home",
    ...DPAD_TO("wiimote"),
  },
  // Wiimote upright with a Nunchuk: one analog stick, C and Z, and the Wiimote's buttons.
  wiimote_nunchuk: {
    a: "wiimote_a", b: "wiimote_b", x: "wiimote_1", y: "wiimote_2", l: "nunchuk_c", r: "", zl: "nunchuk_z", zr: "",
    plus: "wiimote_plus", minus: "wiimote_minus", home: "wiimote_home",
    ...DPAD_TO("wiimote"), ...STICK_TO("lstick", "nunchuk_stick"),
  },
};

const portOf = (m) => m.port ?? 1;

/**
 * The default-mapping entry for (device, port). An entry whose `when`
 * condition holds is preferred over the unconditional one, which is how an
 * option (e.g. N64 dual-controller mode) swaps the default layout.
 */
function pickEntry(core, device, port, effective) {
  const candidates = (core?.default_mappings ?? []).filter((m) => m.device === device && portOf(m) === port);
  return (
    candidates.find((m) => m.when && effective[m.when.option] === m.when.equals) ??
    candidates.find((m) => !m.when) ??
    null
  );
}

/**
 * Default bindings for `core` on `device` for emulated controller `port`,
 * given the effective option values.
 * Returns { supported, derived, map }:
 *  - supported: false when there is nothing to start from (no registry map
 *    for the device and no Classic map to derive from); the bindings then
 *    all start unmapped;
 *  - derived: true when the map was translated from the Classic layout
 *    rather than read from the registry.
 */
export function defaultsFor(core, device, port = 1, effective = {}) {
  const exact = pickEntry(core, device, port, effective);
  if (exact) return { supported: true, derived: false, map: { ...exact.map } };

  const table = DERIVE_FROM_CLASSIC[device];
  const classic = table && pickEntry(core, "classic_controller", port, effective);
  if (classic) {
    const map = {};
    for (const [button, id] of Object.entries(classic.map)) {
      map[button] = id ? (table[id.replace(/^classic_/, "")] ?? "") : "";
    }
    return { supported: true, derived: true, map };
  }
  return { supported: false, derived: false, map: {} };
}

/** Emulated controllers that exist right now, in port order: [{ port, label }]. */
export function activePorts(core, effective) {
  const extra = (core?.controller_ports ?? [])
    .filter((p) => !p.when || effective[p.when.option] === p.when.equals)
    .map((p) => ({ port: p.port, label: p.label }));
  return [{ port: 1, label: "Controller 1" }, ...extra.sort((a, b) => a.port - b.port)];
}

/** Every port the core could ever have (active or not), so bindings can be kept for all of them. */
export function allPorts(core) {
  return [1, ...(core?.controller_ports ?? []).map((p) => p.port)];
}

/**
 * Identifies which default layout is in force for this core/controller/options
 * (which ports exist and which map variant each uses). When it changes, the
 * bindings are reset to the new layout's defaults instead of being kept.
 */
export function layoutKey(core, device, effective) {
  return JSON.stringify(
    activePorts(core, effective).map(({ port }) => {
      const entry = pickEntry(core, device, port, effective) ?? pickEntry(core, "classic_controller", port, effective);
      return [port, entry?.when ?? null];
    }),
  );
}

// Display order for console buttons. The backend serializes each map from a
// BTreeMap (alphabetical), so ordering has to be re-established here.
const BUTTON_ORDER = [
  "Up", "Down", "Left", "Right", "Stick", "C-Up", "C-Down", "C-Left", "C-Right",
  "A", "B", "C", "D", "X", "Y", "Z", "L", "R",
  "Button 1", "Button 2", "Button 3", "I", "II", "III", "IV", "V", "VI", "Fire",
  "Coin", "Start", "Select", "Mode", "Run", "Pause",
];

/** Every console button emulated controller `port` has (union across its mappings), in display order. */
export function consoleButtons(core, port = 1) {
  const names = new Set();
  for (const m of core?.default_mappings ?? []) if (portOf(m) === port) Object.keys(m.map).forEach((k) => names.add(k));
  const rank = (n) => {
    const i = BUTTON_ORDER.indexOf(n);
    return i === -1 ? BUTTON_ORDER.length : i;
  };
  return [...names].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}

/**
 * The console buttons that exist right now. Some buttons only appear while an
 * option has a given value (the registry's `button_requires`), e.g. Genesis
 * X/Y/Z/Mode with the six-button pad.
 */
export function activeButtons(core, effective, port = 1) {
  const requires = core?.button_requires ?? {};
  return consoleButtons(core, port).filter((b) => !requires[b] || effective[requires[b].option] === requires[b].equals);
}

/** Directional / analog buttons go in the left column, everything else on the right. */
export const isDirectional = (name) => /^(Up|Down|Left|Right|Stick|C-.+)$/.test(name);

const DIR_LABELS = { Up: "D-Pad Up", Down: "D-Pad Down", Left: "D-Pad Left", Right: "D-Pad Right", Stick: "Analog Stick" };

/** Human label for a console button row. */
export function buttonLabel(name) {
  if (DIR_LABELS[name]) return DIR_LABELS[name];
  if (/^[A-DXYZLR]$/.test(name)) return `${name} Button`;
  if (name === "I" || name === "II") return `Button ${name}`; // TurboGrafx
  return name;
}
