import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { state } from "./state.js";

/** Refresh every summary/preview element from current wizard state. Called after any field changes. */
export function updateSummary() {
  const title = $("#title").value || "Untitled Channel";
  const core = state.cores.find((c) => c.id === $("#core").value);
  const system = core?.system || "—";
  const controller = $("#controller").selectedOptions[0]?.text || "—";
  const rom = state.rom?.name || "—";
  const id = $("#title-id").value || "—";

  ["#source-preview-title", "#large-preview-title"].forEach((s) => ($(s).textContent = title));
  $("#summary-rom").textContent = rom;
  $("#summary-core").textContent = core ? `${system} — ${core.id}` : "—";
  $("#summary-title").textContent = title;
  $("#summary-id").textContent = id;
  $("#summary-controller").textContent = controller;

  $("#preview-title").textContent = title;
  $("#preview-system").textContent = system;
  $("#preview-core").textContent = core?.id || "—";
  $("#preview-id").textContent = id;

  $("#build-rom").innerHTML = `${escapeHtml(rom)} <span class="check">✓</span>`;
  $("#build-core").innerHTML = `${escapeHtml(core?.id || "—")} <span class="check">✓</span>`;
  $("#build-title").innerHTML = `${escapeHtml(title)} <span class="check">✓</span>`;
  $("#build-id").innerHTML = `${escapeHtml(id)} <span class="check">✓</span>`;
  $("#build-controller").innerHTML = `${escapeHtml(controller)} <span class="check">✓</span>`;
  $("#build-cover").innerHTML = state.cover
    ? `${escapeHtml(state.cover.name)} <span class="check">✓</span>`
    : "Not supplied";

  $("#source-validation").innerHTML =
    state.rom && title
      ? `<span>✓</span><span>Required source information is present. You can continue.</span>`
      : `<span>!</span><span>Add a ROM and title before building.</span>`;
  $("#source-validation").classList.toggle("warning", !(state.rom && title));
}