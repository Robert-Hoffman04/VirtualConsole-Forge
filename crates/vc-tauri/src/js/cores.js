import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { invoke } from "./tauri.js";
import { state } from "./state.js";
import { registerDropzone } from "./dropzones.js";
import { updateSummary } from "./summary.js";
import { refreshConfiguration } from "./configuration.js";

// Dropdown groups, in display order. `source` matches CoreSummary.source from
// the backend: "donor" cores are the official emulator (ripped from a WAD the
// user owns) with the ROM swapped; "bundled" cores are this project's own,
// unofficial emulator DOLs.
const CORE_GROUPS = [
  { source: "donor", label: "Official \u2014 ROM swap (needs donor WAD)" },
  { source: "bundled", label: "Unofficial \u2014 bundled core" },
];

const sourceOf = (c) => c.source ?? (c.donor_label ? "donor" : "bundled");

/** Build the <select> markup: one <optgroup> per source type, skipping empty groups. */
function renderCoreOptions(cores) {
  return CORE_GROUPS.map(({ source, label }) => {
    const options = cores
      .filter((c) => sourceOf(c) === source)
      .map((c) => `<option value="${escapeHtml(c.id)}">${escapeHtml(c.system || c.id)} \u2014 ${escapeHtml(c.id)}</option>`)
      .join("");
    return options ? `<optgroup label="${escapeHtml(label)}">${options}</optgroup>` : "";
  }).join("");
}

/** Load the core registry (via the `list_cores` Tauri command) and populate the core <select>. */
export async function loadCores() {
  const registryPath = $("#registry-path").value || "cores/registry.json";
  let cores = null;
  try {
    cores = await invoke("list_cores", { registryPath });
  } catch (err) {
    // Don't let a bad registry path abort startup: the dropzones still
    // need to be wired up so the user can at least pick files.
    console.error("list_cores failed:", err);
    showError(`Could not load core registry: ${err}`);
    cores = [];
  }
  if (!cores) {
    // No Tauri host present -- e.g. previewing this file directly in a
    // plain browser during frontend-only work. Fall back to a stand-in
    // so the rest of the UI is still exercisable.
    cores = [
      {
        id: "nes",
        system: "NES",
        valid_extensions: ["nes"],
        source: "donor",
        donor_label:
          "Any legitimately-owned NES Virtual Console title (donor-sourced core).",
      },
    ];
  }
  state.cores = cores;

  const select = $("#core");
  const previous = select.value;
  select.innerHTML = renderCoreOptions(cores);
  if (cores.some((c) => c.id === previous)) select.value = previous;
  $("#core-count").textContent = cores.length;

  updateCoreRequirements();
  updateSummary();
}

function showError(message) {
  const el = $("#core-count");
  if (el) el.title = message;
  const zone = $("#rom-name");
  if (zone) zone.textContent = message;
}

/** Rebuild the ROM dropzone's filter and the donor/keys fields for the currently selected core. */
export function updateCoreRequirements() {
  const core = state.cores.find((c) => c.id === $("#core").value);

  refreshConfiguration(); // each system has its own buttons and options

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