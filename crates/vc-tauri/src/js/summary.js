import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { summarizeOptions } from "./coreOptionsData.js";
import { deviceLabel } from "./mappingData.js";

// The summary is spread over several places and the page layout changes
// often, so every write tolerates a missing element. One removed element must
// never stop the rest of the page from updating (a throw here used to abort
// everything after it, including the controller panels).
const setText = (selector, text) => {
  const el = $(selector);
  if (el) el.textContent = text;
};
const setHtml = (selector, html) => {
  const el = $(selector);
  if (el) el.innerHTML = html;
};
const checked = (text) => `${escapeHtml(text)} <span class="check">✓</span>`;

/** Refresh every summary/preview element from current wizard state. Called after any field changes. */
export function updateSummary() {
  const title = $("#title")?.value || "Untitled Channel";
  const core = state.cores.find((c) => c.id === $("#core")?.value);
  const system = core?.system || "—";
  const controllers = state.devices.length ? state.devices.map(deviceLabel).join(", ") : "None selected";
  const options = core ? summarizeOptions(core, state.devices, state.options.values) : "—";
  const rom = state.rom?.name || "—";
  const id = $("#title-id")?.value || "—";

  ["#source-preview-title", "#large-preview-title"].forEach((s) => setText(s, title));

  // Optional summary card (not present in every layout).
  setText("#summary-rom", rom);
  setText("#summary-core", core ? `${system} — ${core.id}` : "—");
  setText("#summary-title", title);
  setText("#summary-id", id);
  setText("#summary-controller", controllers);
  setText("#summary-options", options);

  setText("#preview-title", title);
  setText("#preview-system", system);
  setText("#preview-core", core?.id || "—");
  setText("#preview-id", id);

  setHtml("#build-rom", checked(rom));
  setHtml("#build-core", checked(core?.id || "—"));
  setHtml("#build-title", checked(title));
  setHtml("#build-id", checked(id));
  setHtml("#build-controller", state.devices.length ? checked(controllers) : "None selected");
  setHtml("#build-options", checked(options));
  setHtml("#build-cover", state.cover ? checked(state.cover.name) : "Not supplied");

  const source = $("#source-validation");
  if (source) {
    source.innerHTML =
      state.rom && title
        ? `<span>✓</span><span>Required source information is present. You can continue.</span>`
        : `<span>!</span><span>Add a ROM and title before building.</span>`;
    source.classList.toggle("warning", !(state.rom && title));
  }
}
