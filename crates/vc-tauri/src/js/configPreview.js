// "Core config file" preview on the Build step: shows the exact JSON the
// emulator core will receive for the current core, controllers, bindings and
// options, as produced by the backend (so it can't drift from the real file).

import { $ } from "./dom.js";
import { invoke } from "./tauri.js";
import { currentInput } from "./mapping.js";
import { state } from "./state.js";
import { currentOptions } from "./coreOptions.js";

let requestId = 0;

/** Refresh the preview if the section is open. Cheap to call when it isn't. */
export async function refreshConfigPreview() {
  const details = $("#config-preview-details");
  const pre = $("#config-preview");
  if (!details?.open || !pre) return;

  const mine = ++requestId; // ignore out-of-order responses
  if (!state.devices.length) {
    pre.textContent = "Select at least one controller to see the config file.";
    return;
  }
  let text;
  try {
    text = await invoke("preview_core_config", {
      registryPath: $("#registry-path").value || "cores/registry.json",
      coreId: $("#core").value,
      input: currentInput(),
      options: currentOptions(),
    });
    text ??= "(The config preview needs the desktop app.)";
  } catch (error) {
    text = `Could not build the config: ${error}`;
  }
  if (mine === requestId) pre.textContent = text;
}

export function initConfigPreview() {
  $("#config-preview-details")?.addEventListener("toggle", refreshConfigPreview);
  document.addEventListener("stepchange", refreshConfigPreview);
}
