// Controller selection and button-binding UI for the Configuration step.
//
// The step starts as a set of checkboxes, one per controller. Checking one
// shows that controller's own panel (picture + bindings); every enabled
// controller is active at once in the finished core config, each with its
// own bindings. No controller is ever disabled for a system: a controller
// with fewer buttons than the system just leaves some buttons unmapped, which
// is the user's call since many games don't use them all.
//
// Within a panel, the rows depend on the selected core (each system has its
// own buttons) and the core's options (some buttons only exist with e.g. the
// six-button pad; some cores expose several emulated controllers driven by
// the one physical controller, such as N64 dual-controller mode).
//
// Changing the core resets every controller's bindings to the registry
// defaults. Changing an option keeps the user's edits, unless the option
// switches which default layout is in force (e.g. dual-controller mode), in
// which case that controller's bindings are reset to the new layout.

import { $, escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { effectiveValues } from "./coreOptionsData.js";
import { controllerArt } from "./controllerPreview.js";
import {
  DEVICES, PHYSICAL_INPUTS, deviceLabel, defaultsFor, consoleButtons, activeButtons, activePorts, allPorts,
  layoutKey, isDirectional, buttonLabel,
} from "./mappingData.js";

const selectedCore = () => state.cores.find((c) => c.id === $("#core").value);
const effectiveNow = (core) => effectiveValues(core, state.devices, state.options.values);

// ---------------------------------------------------------------- controller picker

/** Build the controller checkboxes. `onChange` runs after the enabled set changes. */
export function initControllerPicker({ onChange = () => {} } = {}) {
  const host = $("#controller-picker");
  if (!host) return;

  host.innerHTML = DEVICES.map(
    (d) => `<label class="check-card" data-device="${d.id}">
      <input type="checkbox" value="${d.id}">
      <span class="check-card-art">${controllerArt(d.id).svg}</span>
      <span class="check-card-text">
        <span class="check-card-title">${escapeHtml(d.label)}</span>
        <span class="check-card-blurb">${escapeHtml(d.blurb)}</span>
      </span>
    </label>`,
  ).join("");

  host.addEventListener("change", (event) => {
    const box = event.target.closest?.('input[type="checkbox"]');
    if (!box) return;
    // Keep the order the user enabled them in: the first is the "primary" one
    // recorded for the legacy (donor) config blob.
    state.devices = box.checked
      ? [...state.devices.filter((d) => d !== box.value), box.value]
      : state.devices.filter((d) => d !== box.value);
    host.querySelector(`[data-device="${box.value}"]`).classList.toggle("checked", box.checked);
    onChange();
  });

  const panels = $("#device-panels");
  panels.addEventListener("change", recordBinding);
  panels.addEventListener("click", (event) => {
    const reset = event.target.closest?.("[data-reset-device]");
    if (reset) {
      resetDevice(reset.dataset.resetDevice, "Bindings reset to the defaults.");
      renderPanels();
    }
  });
}

/** Uncheck every controller (used when the wizard is reset). */
export function clearDevices() {
  state.devices = [];
  document.querySelectorAll("#controller-picker input").forEach((box) => {
    box.checked = false;
    box.closest(".check-card")?.classList.remove("checked");
  });
}

/** Show a hint under the checkboxes and gate "Continue" on having at least one. */
export function updatePickerHint() {
  const none = state.devices.length === 0;
  const hint = $("#controller-hint");
  if (hint) {
    hint.classList.toggle("warning", none);
    hint.textContent = none
      ? "Select at least one controller to continue."
      : "Each enabled controller is active at the same time and gets its own bindings.";
  }
  const next = document.querySelector("#step-config .next-step");
  if (next) next.disabled = none;
}

// ---------------------------------------------------------------- state

function noteFor(core, device, portMaps) {
  if (!core) return "";
  const { supported, derived } = defaultsFor(core, device, 1, effectiveNow(core));
  const buttons = consoleButtons(core, 1);
  const unmapped = buttons.filter((b) => !portMaps[1]?.[b]).length;
  const base = !supported
    ? `No default bindings for ${core.system} on this controller; choose the inputs you need.`
    : derived
      ? `Defaults derived from the Classic Controller layout for ${core.system}.`
      : `Defaults for ${core.system}.`;
  return unmapped > 0
    ? `${base} ${unmapped} of ${buttons.length} buttons start unmapped; map the ones your games use.`
    : `${base} Change any binding below.`;
}

/** (Re)build the bindings for one controller from the registry defaults for the selected core and options. */
function resetDevice(device, note) {
  const core = selectedCore();
  const effective = effectiveNow(core);

  // Keep a map for every port the core can ever have (active or not) and for
  // every button, so edits persist while options toggle them on and off.
  const ports = {};
  for (const port of allPorts(core)) {
    const d = defaultsFor(core, device, port, effective);
    ports[port] = Object.fromEntries(consoleButtons(core, port).map((b) => [b, d.map[b] ?? ""]));
  }
  state.mapping.byDevice[device] = {
    ports,
    layout: layoutKey(core, device, effective),
    note: note ?? noteFor(core, device, ports),
  };
}

/** Throw away every controller's bindings (new core) and start again for the enabled ones. */
export function resetMapping() {
  state.mapping = { byDevice: {} };
  syncMappingRows();
}

/**
 * The enabled controllers or an option changed: give newly enabled
 * controllers their default bindings, show/hide the controllers and buttons
 * that depend on options, and redraw. Edits are kept, unless an option
 * switched a controller's default layout, in which case that controller's
 * bindings are reset.
 */
export function syncMappingRows() {
  const core = selectedCore();
  const effective = effectiveNow(core);
  for (const device of state.devices) {
    const current = state.mapping.byDevice[device];
    if (!current) {
      resetDevice(device);
    } else if (current.layout !== layoutKey(core, device, effective)) {
      resetDevice(device, "Bindings were reset because an option changed the default layout.");
    }
  }
  renderPanels();
}

// ---------------------------------------------------------------- rendering

function optionsHtml(device, selected) {
  const inputs = PHYSICAL_INPUTS[device] ?? [];
  const groups = [];
  for (const input of inputs) {
    let g = groups.find((x) => x.name === input.group);
    if (!g) groups.push((g = { name: input.group, items: [] }));
    g.items.push(input);
  }
  const opt = (id, label) =>
    `<option value="${escapeHtml(id)}"${id === selected ? " selected" : ""}>${escapeHtml(label)}</option>`;

  let html = opt("", "Unmapped");
  for (const g of groups) {
    html += `<optgroup label="${escapeHtml(g.name)}">${g.items.map((i) => opt(i.id, i.label)).join("")}</optgroup>`;
  }
  // A registry default we don't have a catalog entry for must not silently vanish.
  if (selected && !inputs.some((i) => i.id === selected)) html += opt(selected, selected);
  return html;
}

function rowHtml(name, device, port) {
  const value = state.mapping.byDevice[device]?.ports[port]?.[name] ?? "";
  return `<div class="mapping-row" data-button="${escapeHtml(name)}" data-port="${port}">
    <span class="mapping-label">${escapeHtml(buttonLabel(name))}</span>
    <select data-device="${device}" data-button="${escapeHtml(name)}" data-port="${port}">${optionsHtml(device, value)}</select>
  </div>`;
}

function panelHtml(core, effective, device) {
  const ports = activePorts(core, effective);
  const multi = ports.length > 1;
  const p1 = activeButtons(core, effective, 1);
  const art = controllerArt(device);

  const extra = ports
    .slice(1)
    .map(({ port, label }) => {
      const buttons = activeButtons(core, effective, port);
      return `<div class="port-block" data-port-block="${port}">
        <div class="port-title">${escapeHtml(label)}</div>
        <div class="port-grid">${buttons.map((b) => rowHtml(b, device, port)).join("")}</div>
      </div>`;
    })
    .join("");

  return `<section class="panel device-panel" data-device-panel="${device}">
    <div class="panel-header device-panel-header">
      <div>
        <div class="panel-title">${escapeHtml(deviceLabel(device))}</div>
        <div class="panel-description">${escapeHtml(state.mapping.byDevice[device]?.note ?? "")}</div>
      </div>
      <button type="button" class="button ghost" data-reset-device="${device}">Reset bindings</button>
    </div>
    <div class="panel-body">
      ${multi ? `<div class="port-title">${escapeHtml(ports[0].label)}</div>` : ""}
      <div class="controller" style="margin-top:${multi ? 8 : 0}px">
        <div class="mapping-list">${p1.filter(isDirectional).map((b) => rowHtml(b, device, 1)).join("")}</div>
        <div>
          <div class="controller-visual">
            <div class="controller-art" role="img" aria-label="${escapeHtml(art.name)} preview">${art.svg}</div>
          </div>
          <div class="mapping-status" data-status></div>
        </div>
        <div class="mapping-list">${
          p1.length
            ? p1.filter((b) => !isDirectional(b)).map((b) => rowHtml(b, device, 1)).join("")
            : `<div class="empty-note">This core defines no bindable buttons.</div>`
        }</div>
      </div>
      ${extra}
    </div>
  </section>`;
}

/** Draw a panel for each enabled controller, in the order they were enabled. */
export function renderPanels() {
  const core = selectedCore();
  const effective = effectiveNow(core);
  const host = $("#device-panels");
  if (!host) return;

  host.innerHTML = core ? state.devices.map((device) => panelHtml(core, effective, device)).join("") : "";
  host.querySelectorAll("[data-device-panel]").forEach((panel) => markConflicts(panel));
  updatePickerHint();
}

/** Flag physical inputs bound twice within one controller, including across its emulated controllers. */
function markConflicts(panel) {
  const device = panel.dataset.devicePanel;
  const rows = [...panel.querySelectorAll(".mapping-row")];
  const idOf = (row) => state.mapping.byDevice[device]?.ports[row.dataset.port]?.[row.dataset.button];

  const used = {};
  for (const row of rows) {
    const id = idOf(row);
    if (id) used[id] = (used[id] ?? 0) + 1;
  }
  let shared = 0;
  rows.forEach((row) => {
    const id = idOf(row);
    const conflict = Boolean(id) && used[id] > 1;
    row.classList.toggle("conflict", conflict);
    if (conflict) shared++;
  });

  const status = panel.querySelector("[data-status]");
  status.classList.toggle("warning", shared > 0);
  status.textContent = shared > 0 ? `${shared} buttons share the same input.` : "";
}

function recordBinding(event) {
  const select = event.target.closest?.("select[data-button]");
  if (!select) return;
  const { device, button } = select.dataset;
  const port = select.dataset.port ?? "1";
  ((state.mapping.byDevice[device] ??= { ports: {} }).ports[port] ??= {})[button] = select.value;
  markConflicts(select.closest("[data-device-panel]"));
}

// ---------------------------------------------------------------- outputs

/**
 * Current input setup in the shape `build_wad_command` / `preview_core_config`
 * expect: one entry per enabled controller, in the order they were enabled.
 * `map` is emulated controller 1, `ports` holds controllers 2 and up.
 * Unmapped buttons and buttons/ports that don't currently exist are omitted.
 */
export function currentInput() {
  const core = selectedCore();
  const effective = effectiveNow(core);

  return state.devices.map((device) => {
    const byPort = state.mapping.byDevice[device]?.ports ?? {};
    const forPort = (port) => {
      const active = new Set(activeButtons(core, effective, port));
      const map = {};
      for (const [button, id] of Object.entries(byPort[port] ?? {})) if (id && active.has(button)) map[button] = id;
      return map;
    };
    const ports = {};
    for (const { port } of activePorts(core, effective).slice(1)) ports[port] = forPort(port);
    return { device, map: forPort(1), ports };
  });
}
