import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { invoke, hasTauri } from "./tauri.js";
import { updateSummary } from "./summary.js";
import { setStep } from "./nav.js";
import { markTitleIdUsed } from "./titleid.js";

export function initBuild() {
  $("#build-button").addEventListener("click", onBuild);
  $("#reset-build").addEventListener("click", resetBuild);
  $("#reset-build-bottom").addEventListener("click", resetBuild);
}

async function onBuild() {
  const status = $("#build-status");

  if (!state.rom || !$("#title").value) {
    status.className = "validation error";
    status.innerHTML = `<span>!</span><span>A ROM and title are required before building.</span>`;
    return;
  }
  const outputPath = $("#output-path").value.trim();
  if (!outputPath) {
    status.className = "validation error";
    status.innerHTML = `<span>!</span><span>Choose an output path before building.</span>`;
    return;
  }
  if (!hasTauri()) {
    status.className = "validation warning";
    status.innerHTML = `<span>i</span><span>No Tauri host detected in this context, so the real build command can't run here.</span>`;
    return;
  }

  status.className = "validation";
  status.innerHTML = `<span>…</span><span>Building…</span>`;

  try {
    // TODO(backend): button_map is still a placeholder in
    // commands::build_wad_command -- see the TODO there. Real per-button
    // overrides collected from the Configuration step aren't wired into
    // `map` yet; only the selected device is used today.
    const mapping = { device: $("#controller").value, map: {} };

    await invoke("build_wad_command", {
      registryPath: $("#registry-path").value || "cores/registry.json",
      coreId: $("#core").value,
      romPath: state.rom.path,
      coverPath: state.cover?.path || null,
      title: $("#title").value,
      outputPath,
      mapping,
      donorPath: state.donor?.path || null,
      keysPath: state.keys?.path || null,
    });

    markTitleIdUsed($("#title-id").value);
    status.className = "validation";
    status.innerHTML = `<span>✓</span><span>WAD built successfully.</span>`;
  } catch (error) {
    status.className = "validation error";
    status.innerHTML = `<span>!</span><span>${escapeHtml(String(error))}</span>`;
  }
}

function resetBuild() {
  state.rom = state.donor = state.keys = state.cover = null;
  $("#rom-name").textContent = "";
  $("#cover-name").textContent = "";
  $("#title").value = "";
  $("#title-id").value = "000100014E455341";
  $("#output-path").value = "";
  ["#source-preview-art", "#large-preview-art"].forEach((sel) => {
    const art = $(sel);
    art.classList.add("placeholder");
    art.style.backgroundImage = "";
  });
  setStep(0);
  updateSummary();
}