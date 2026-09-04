import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { invoke } from "./tauri.js";
import { state } from "./state.js";
import { registerDropzone } from "./dropzones.js";
import { updateSummary } from "./summary.js";

/** Load the core registry (via the `list_cores` Tauri command) and populate the core <select>. */
export async function loadCores() {
  const registryPath = $("#registry-path").value || "cores/registry.json";
  let cores = await invoke("list_cores", { registryPath });
  if (!cores) {
    // No Tauri host present -- e.g. previewing this file directly in a
    // plain browser during frontend-only work. Fall back to a stand-in
    // so the rest of the UI is still exercisable.
    cores = [
      {
        id: "nes",
        system: "NES",
        valid_extensions: ["nes"],
        donor_label:
          "Any legitimately-owned NES Virtual Console title (donor-sourced core).",
      },
    ];
  }
  state.cores = cores;

  const select = $("#core");
  select.innerHTML = cores
    .map((c) => `<option value="${escapeHtml(c.id)}">${escapeHtml(c.system || c.id)} — ${escapeHtml(c.id)}</option>`)
    .join("");
  $("#core-count").textContent = cores.length;

  updateCoreRequirements();
  updateSummary();
}

/** Rebuild the ROM dropzone's filter and the donor/keys fields for the currently selected core. */
export function updateCoreRequirements() {
  const core = state.cores.find((c) => c.id === $("#core").value);

  registerDropzone("rom-drop", {
    filters: core?.valid_extensions?.length
      ? [{ name: `${core.system || "ROM"} files`, extensions: core.valid_extensions }]
      : undefined,
    title: "Select a ROM file",
    onPick: (path, name) => {
      state.rom = { path, name };
      $("#rom-name").textContent = name;
      updateSummary();
    },
  });

  const donor = core?.donor_label;
  const box = $("#donor-fields");
  if (!donor) {
    state.donor = state.keys = null;
    box.innerHTML = `<div class="empty-note">This core does not currently require a donor WAD or common key.</div>`;
    return;
  }

  box.innerHTML = `
    <div class="field">
      <label>Donor WAD <span class="hint">Required for this core</span></label>
      <div class="dropzone" id="donor-drop">
        <div>
          <div class="drop-title">Click or drop donor WAD here</div>
          <div class="drop-subtitle">${escapeHtml(donor)}</div>
          <div class="file-name" id="donor-name"></div>
        </div>
      </div>
    </div>
    <div class="field" style="margin-top:15px">
      <label>Common Key File <span class="hint">Required for this core</span></label>
      <div class="dropzone" id="keys-drop">
        <div>
          <div class="drop-title">Click or drop keys file here</div>
          <div class="drop-subtitle">Used locally during the build. Never uploaded anywhere.</div>
          <div class="file-name" id="keys-name"></div>
        </div>
      </div>
    </div>`;

  registerDropzone("donor-drop", {
    filters: [{ name: "Wii WAD", extensions: ["wad"] }],
    title: "Select a donor WAD you own",
    onPick: (path, name) => {
      state.donor = { path, name };
      $("#donor-name").textContent = name;
    },
  });
  registerDropzone("keys-drop", {
    title: "Select your common key file",
    onPick: (path, name) => {
      state.keys = { path, name };
      $("#keys-name").textContent = name;
    },
  });
}