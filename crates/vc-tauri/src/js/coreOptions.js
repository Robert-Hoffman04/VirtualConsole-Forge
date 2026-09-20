// The per-core "Core Options" panel on the Configuration step. Everything
// shown is driven by the selected core's `options` in the registry, so each
// system gets its own accessories/settings (N64 Expansion Pak, DS Slot-2
// accessory, ...) without any frontend changes.

import { $, escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { DEFAULT_GROUP, defaultValues, evaluateOptions, describeValue } from "./coreOptionsData.js";

const selectedCore = () => state.cores.find((c) => c.id === $("#core").value);

function controlHtml(option, value, enabled) {
  const id = `opt-${option.id}`;
  const attrs = `id="${id}" data-option="${escapeHtml(option.id)}"${enabled ? "" : " disabled"}`;

  if (option.kind === "toggle") {
    return `<label class="switch"><input type="checkbox" ${attrs}${value ? " checked" : ""}><span class="switch-track"></span></label>`;
  }
  if (option.kind === "select") {
    const opts = option.choices
      .map((c) => `<option value="${escapeHtml(c.value)}"${c.value === value ? " selected" : ""}>${escapeHtml(c.label)}</option>`)
      .join("");
    return `<select ${attrs}>${opts}</select>`;
  }
  // number
  return `<div class="range-control">
    <input type="range" ${attrs} min="${option.min}" max="${option.max}" step="${option.step ?? 1}" value="${value}">
    <output for="${id}">${escapeHtml(describeValue(option, value))}</output>
  </div>`;
}

function rowHtml(option) {
  const desc = option.description ? `<span class="option-desc">${escapeHtml(option.description)}</span>` : "";
  return `<div class="option-row" data-option-row="${escapeHtml(option.id)}">
    <div class="option-text">
      <label for="opt-${escapeHtml(option.id)}" title="Config key: ${escapeHtml(option.key ?? `extra.${option.id}`)}">${escapeHtml(option.label)}</label>${desc}
      <span class="option-hint" hidden>Not available with the selected controllers.</span>
    </div>
    <div class="option-control"></div>
  </div>`;
}

/** Show/hide/disable rows to match the current values and enabled controllers. Cheap; safe to call on every change. */
function applyState(core) {
  const states = evaluateOptions(core, state.devices, state.options.values);
  const rows = new Map([...document.querySelectorAll("[data-option-row]")].map((r) => [r.dataset.optionRow, r]));

  for (const { option, value, shown, enabled } of states) {
    const row = rows.get(option.id);
    if (!row) continue;
    row.hidden = !shown;
    row.classList.toggle("unavailable", shown && !enabled);
    row.querySelector(".option-hint").hidden = !(shown && !enabled);

    const input = row.querySelector("[data-option]");
    if (input) {
      input.disabled = !enabled;
      // Inapplicable options display (and are sent as) their default.
      if (option.kind === "toggle") input.checked = Boolean(value);
      else if (input.value !== String(value)) input.value = value;
      const out = row.querySelector("output");
      if (out) out.textContent = describeValue(option, value);
    }
  }

  // Hide a group heading when none of its rows are visible.
  document.querySelectorAll(".option-group").forEach((g) => {
    g.hidden = [...g.querySelectorAll(".option-row")].every((r) => r.hidden);
  });
}

/** Re-evaluate which options apply (e.g. after a controller was enabled/disabled) without rebuilding the panel. */
export function applyOptionState() {
  const core = selectedCore();
  if (core) applyState(core);
}

/**
 * Rebuild the panel for the currently selected core. Values the user set for
 * this core earlier in the session are restored.
 */
export function refreshCoreOptions() {
  const core = selectedCore();
  const options = core?.options ?? [];

  state.options.coreId = core?.id ?? null;
  state.options.values = { ...defaultValues(core), ...(state.optionMemory[core?.id] ?? {}) };

  $("#core-options-title").textContent = core ? `${core.system} Options` : "Core Options";
  $("#core-options-description").textContent = !core
    ? "Select a core to see its options."
    : options.length
      ? "Settings and accessories specific to this system. Options that don't apply to the enabled controllers are disabled."
      : "This core has no additional options.";
  $("#reset-options").hidden = options.length === 0;

  const host = $("#core-options");
  if (!options.length) {
    host.innerHTML = `<div class="empty-note">${core ? "Nothing to configure for this core." : "No core selected."}</div>`;
    return;
  }

  // Group rows by heading, keeping registry order within and between groups.
  const groups = [];
  for (const option of options) {
    const name = option.group || DEFAULT_GROUP;
    let g = groups.find((x) => x.name === name);
    if (!g) groups.push((g = { name, items: [] }));
    g.items.push(option);
  }
  host.innerHTML = groups
    .map((g) => `<div class="option-group"><div class="option-group-title">${escapeHtml(g.name)}</div>${g.items.map(rowHtml).join("")}</div>`)
    .join("");

  // Controls depend on the effective value, so fill them in after the rows exist.
  const byId = Object.fromEntries(options.map((o) => [o.id, o]));
  host.querySelectorAll(".option-row").forEach((row) => {
    const option = byId[row.dataset.optionRow];
    row.querySelector(".option-control").innerHTML = controlHtml(option, state.options.values[option.id], true);
  });
  applyState(core);
}

/**
 * Attach the delegated listeners (call once at startup). `onChange` runs
 * after any option value changes (or is reset), so dependents can update.
 */
export function initCoreOptions({ onChange = () => {} } = {}) {
  const record = (event) => {
    const input = event.target.closest?.("[data-option]");
    const core = selectedCore();
    if (!input || !core) return;
    const option = core.options.find((o) => o.id === input.dataset.option);
    if (!option) return;

    const value =
      option.kind === "toggle" ? input.checked : option.kind === "number" ? Number(input.value) : input.value;
    state.options.values[option.id] = value;
    (state.optionMemory[core.id] ??= {})[option.id] = value;
    applyState(core);
    onChange();
  };
  const host = $("#core-options");
  host.addEventListener("change", record);
  host.addEventListener("input", record); // live update while dragging a slider

  $("#reset-options").addEventListener("click", () => {
    const core = selectedCore();
    if (core) delete state.optionMemory[core.id];
    refreshCoreOptions();
    onChange();
  });
}

/** Current option values in the shape `build_wad_command` expects. */
export function currentOptions() {
  return { ...state.options.values };
}
