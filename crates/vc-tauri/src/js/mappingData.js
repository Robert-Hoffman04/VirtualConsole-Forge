import { effectiveValues } from "./coreOptionsData.js";

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

/** Physical inputs available on each controller type (keys = the #controller <option> values). */
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
 * The GameCube pad is supported by every Virtual Console title, but the
 * registry only ships Classic Controller / Wiimote defaults. When a core has
 * no explicit GameCube mapping we derive one from its Classic mapping using
 * this table (Classic input suffix -> GameCube input id; "" = no equivalent).
 */
const CLASSIC_TO_GC = {
  a: "gc_a", b: "gc_b", x: "gc_x", y: "gc_y",
  l: "gc_l", r: "gc_r", zl: "gc_z", zr: "",
  plus: "gc_start", minus: "", home: "",
  dpad_up: "gc_dpad_up", dpad_down: "gc_dpad_down", dpad_left: "gc_dpad_left", dpad_right: "gc_dpad_right",
  lstick: "gc_lstick", lstick_up: "gc_lstick_up", lstick_down: "gc_lstick_down",
  lstick_left: "gc_lstick_left", lstick_right: "gc_lstick_right",
  rstick: "gc_cstick", rstick_up: "gc_cstick_up", rstick_down: "gc_cstick_down",
  rstick_left: "gc_cstick_left", rstick_right: "gc_cstick_right",
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
 *  - supported: false when the registry has no map for that device/port;
 *  - derived: true when the map was translated (GameCube from Classic)
 *    rather than read from the registry.
 */
export function defaultsFor(core, device, port = 1, effective = {}) {
  const exact = pickEntry(core, device, port, effective);
  if (exact) return { supported: true, derived: false, map: { ...exact.map } };

  if (device === "gamecube") {
    const classic = pickEntry(core, "classic_controller", port, effective);
    if (classic) {
      const map = {};
      for (const [button, id] of Object.entries(classic.map)) {
        map[button] = id ? (CLASSIC_TO_GC[id.replace(/^classic_/, "")] ?? "") : "";
      }
      return { supported: true, derived: true, map };
    }
  }
  return { supported: false, derived: false, map: {} };
}

/**
 * Can this core be played with `device` at all? False for e.g. the sideways
 * Wiimote on systems that need more buttons. (Independent of options.)
 */
export function deviceSupported(core, device) {
  const maps = core?.default_mappings ?? [];
  const has = (d) => maps.some((m) => m.device === d && portOf(m) === 1);
  return has(device) || (device === "gamecube" && has("classic_controller"));
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
export function layoutKey(core, device, values) {
  const effective = effectiveValues(core, device, values);
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
export function activeButtons(core, device, values, port = 1) {
  const requires = core?.button_requires ?? {};
  const effective = effectiveValues(core, device, values);
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
