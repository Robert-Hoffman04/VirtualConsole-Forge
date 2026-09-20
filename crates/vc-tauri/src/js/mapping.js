// Button-binding UI for the Configuration step. The rows shown depend on the
// selected core (each system has its own buttons), the selected controller
// (which decides what they can be bound to) and the core's options (some
// buttons only exist with e.g. the six-button pad). A core can also expose
// several emulated controllers driven by the one physical controller (N64
// dual-controller mode), each with its own set of bindings.
//
// Changing the core or controller resets the bindings to the registry
// defaults. Changing an option keeps the user's edits, unless the option
// switches which default layout is in force (e.g. dual-controller mode), in
// which case the bindings are reset to that layout.

import { $, escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { effectiveValues } from "./coreOptionsData.js";
import {
  PHYSICAL_INPUTS, defaultsFor, deviceSupported, consoleButtons, activeButtons, activePorts, allPorts, layoutKey,
  isDirectional, buttonLabel,
} from "./mappingData.js";

const FALLBACK_DEVICE = "classic_controller";

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
  const value = state.mapping.ports[port]?.[name] ?? "";
  return `<div class="mapping-row" data-button="${escapeHtml(name)}" data-port="${port}">
    <span class="mapping-label">${escapeHtml(buttonLabel(name))}</span>
    <select data-button="${escapeHtml(name)}" data-port="${port}">${optionsHtml(device, value)}</select>
  </div>`;
}

const selectedCore = () => state.cores.find((c) => c.id === $("#core").value);

/** Enable only the controllers the selected core supports; move off an unsupported selection. */
export function syncControllerChoices() {
  const core = selectedCore();
  const select = $("#controller");
  for (const option of select.options) {
    option.dataset.label ??= option.textContent;
    const ok = !core || deviceSupported(core, option.value);
    option.disabled = !ok;
    option.textContent = ok ? option.dataset.label : `${option.dataset.label} \u2014 not supported`;
  }
  if (select.selectedOptions[0]?.disabled) {
    const fallback =
      [...select.options].find((o) => o.value === FALLBACK_DEVICE && !o.disabled) ??
      [...select.options].find((o) => !o.disabled);
    if (fallback) select.value = fallback.value;
  }
}

function markConflicts() {
  // A physical input bound twice is a conflict, including across emulated controllers.
  const rows = [...document.querySelectorAll(".mapping-row")];
  const idOf = (row) => state.mapping.ports[row.dataset.port]?.[row.dataset.button];
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

  const status = $("#mapping-status");
  if (!status) return;
  const { note } = state.mapping;
  status.classList.toggle("warning", shared > 0);
  status.textContent = shared > 0 ? `${shared} buttons share the same input.` : note;
}

/** Draw the rows for the emulated controllers and buttons that currently exist. Values come from state.mapping.ports, so edits survive. */
function renderRows() {
  const core = selectedCore();
  const device = $("#controller").value;
  const effective = effectiveValues(core, device, state.options.values);
  const ports = activePorts(core, effective);
  const multi = ports.length > 1;

  // Controller 1 uses the two side columns around the controller picture.
  const p1 = activeButtons(core, device, state.options.values, 1);
  $("#mapping-left").innerHTML = p1.filter(isDirectional).map((b) => rowHtml(b, device, 1)).join("");
  $("#mapping-right").innerHTML = p1.filter((b) => !isDirectional(b)).map((b) => rowHtml(b, device, 1)).join("");
  if (!p1.length) $("#mapping-right").innerHTML = `<div class="empty-note">This core defines no bindable buttons.</div>`;

  const title = $("#port1-title");
  title.hidden = !multi;
  title.textContent = ports[0].label;

  // Additional emulated controllers each get their own block below.
  $("#extra-ports").innerHTML = ports
    .slice(1)
    .map(({ port, label }) => {
      const buttons = activeButtons(core, device, state.options.values, port);
      return `<div class="port-block" data-port-block="${port}">
        <div class="port-title">${escapeHtml(label)}</div>
        <div class="port-grid">${buttons.map((b) => rowHtml(b, device, port)).join("")}</div>
      </div>`;
    })
    .join("");
  markConflicts();
}

/**
 * Reset the bindings to the registry defaults for the selected core,
 * controller and current options, and redraw. Call after core option values
 * are up to date, since they decide which controllers and buttons exist.
 */
export function resetMapping(note) {
  const core = selectedCore();
  const device = $("#controller").value;
  const effective = effectiveValues(core, device, state.options.values);

  // Keep a map for every port the core can ever have (active or not) and for
  // every button, so edits persist while options toggle them on and off.
  const portMaps = {};
  let derived = false;
  for (const port of allPorts(core)) {
    const d = defaultsFor(core, device, port, effective);
    derived ||= d.derived && d.supported;
    portMaps[port] = Object.fromEntries(consoleButtons(core, port).map((b) => [b, d.map[b] ?? ""]));
  }

  state.mapping = {
    device,
    ports: portMaps,
    layout: layoutKey(core, device, state.options.values),
    note:
      note ??
      (!core
        ? ""
        : derived
          ? `Defaults derived from the Classic Controller layout for ${core.system}.`
          : `Defaults for ${core.system}. Change any binding below.`),
  };
  renderRows();
}

/**
 * A core option changed: show/hide the controllers and buttons that depend on
 * it. Edits are kept, unless the option switched the default layout, in which
 * case the old bindings no longer make sense and are reset.
 */
export function syncMappingRows() {
  const core = selectedCore();
  const device = $("#controller").value;
  if (layoutKey(core, device, state.options.values) !== state.mapping.layout) {
    resetMapping("Bindings were reset because this option changes the default layout.");
    return;
  }
  renderRows();
}

/** Attach the (delegated) listeners that record edits to individual bindings. */
export function initMapping() {
  const record = (event) => {
    const select = event.target.closest?.("select[data-button]");
    if (!select) return;
    const port = select.dataset.port ?? "1";
    (state.mapping.ports[port] ??= {})[select.dataset.button] = select.value;
    markConflicts();
  };
  $(".controller").addEventListener("change", record);
  $("#extra-ports").addEventListener("change", record);
}

/**
 * Current bindings in the shape `build_wad_command` / `preview_core_config`
 * expect: `map` is controller 1, `ports` holds controllers 2 and up.
 * Unmapped buttons and buttons/ports that don't currently exist are omitted.
 */
export function currentMapping() {
  const core = selectedCore();
  const device = state.mapping.device ?? $("#controller").value;
  const effective = effectiveValues(core, device, state.options.values);

  const forPort = (port) => {
    const active = new Set(activeButtons(core, device, state.options.values, port));
    const map = {};
    for (const [button, id] of Object.entries(state.mapping.ports[port] ?? {})) if (id && active.has(button)) map[button] = id;
    return map;
  };

  const ports = {};
  for (const { port } of activePorts(core, effective).slice(1)) ports[port] = forPort(port);
  return { device, map: forPort(1), ports };
}
