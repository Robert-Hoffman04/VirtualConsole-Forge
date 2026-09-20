import { $ } from "./dom.js";
import { escapeHtml } from "./dom.js";
import { state } from "./state.js";
import { invoke, hasTauri } from "./tauri.js";
import { setStep } from "./nav.js";
import { markTitleIdUsed } from "./titleid.js";
import { currentInput, clearDevices } from "./mapping.js";
import { currentOptions } from "./coreOptions.js";
import { refreshConfiguration } from "./configuration.js";
import {
  isForwarder, forwarderProblem, deviceRomPath, deviceCorePath, onDevice, resetForwarderFields, DEFAULT_LOADER,
} from "./forwarder.js";

export function initBuild() {
  $("#build-button").addEventListener("click", onBuild);
  $("#reset-build").addEventListener("click", resetBuild);
  $("#reset-build-bottom").addEventListener("click", resetBuild);
}

function showStatus(kind, lines) {
  const status = $("#build-status");
  const icon = { error: "!", warning: "i", working: "…", ok: "✓" }[kind] ?? "";
  status.className = kind === "error" ? "validation error" : kind === "warning" ? "validation warning" : "validation";
  status.innerHTML = `<span>${icon}</span><span>${[].concat(lines).map(escapeHtml).join("<br>")}</span>`;
}

async function onBuild() {
  const forwarder = isForwarder();

  if (!$("#title").value.trim()) return showStatus("error", "A title is required before building.");
  if (forwarder) {
    const problem = forwarderProblem();
    if (problem) return showStatus("error", problem);
  } else if (!state.rom) {
    return showStatus("error", "A ROM and title are required before building.");
  }
  if (!state.devices.length) return showStatus("error", "Select at least one controller on the Configuration step.");
  const outputPath = $("#output-path").value.trim();
  if (!outputPath) return showStatus("error", "Choose an output path before building.");
  if (!hasTauri()) {
    return showStatus("warning", "No Tauri host detected in this context, so the real build command can't run here.");
  }

  showStatus("working", "Building…");

  try {
    // Bindings + option values become the unified core config file
    // (validated by the backend; see docs/CONFIG_FORMAT.md).
    const input = currentInput();
    const common = {
      registryPath: $("#registry-path").value || "cores/registry.json",
      coreId: $("#core").value,
      coverPath: state.cover?.path || null,
      title: $("#title").value,
      outputPath,
      input,
      options: currentOptions(),
    };

    if (forwarder) {
      const result = await invoke("build_forwarder_command", {
        ...common,
        loaderPath: $("#loader-path").value.trim() || DEFAULT_LOADER,
        device: state.forwarder.device,
        deviceRomPath: deviceRomPath(),
        deviceCorePath: deviceCorePath() || null,
        launchCfgOut: $("#launch-cfg-path").value.trim() || null,
      });
      markTitleIdUsed($("#title-id").value);
      showStatus("ok", [
        "Forwarder WAD built.",
        `Put these on the ${result.device.toUpperCase()} before launching the channel:`,
        `  core: ${result.core_source_path ?? "the core DOL"}  →  ${result.device}:${result.core_device_path}`,
        `  ROM:  your ROM  →  ${result.device}:${result.rom_device_path}`,
        ...(result.launch_cfg_path ? [`launch.cfg saved to ${result.launch_cfg_path}`] : []),
      ]);
      return;
    }

    await invoke("build_wad_command", {
      ...common,
      romPath: state.rom.path,
      donorPath: state.donor?.path || null,
      keysPath: state.keys?.path || null,
    });

    markTitleIdUsed($("#title-id").value);
    showStatus("ok", "WAD built successfully.");
  } catch (error) {
    showStatus("error", String(error));
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
  state.optionMemory = {};
  clearDevices();
  resetForwarderFields();
  refreshConfiguration();
}