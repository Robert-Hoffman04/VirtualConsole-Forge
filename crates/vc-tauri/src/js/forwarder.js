// Forwarder mode: instead of packing the ROM (and core) into the WAD, build a
// small WAD whose loader reads the core DOL and the ROM from the SD card or
// USB drive when the channel is launched (see docs/FORWARDER.md). Only
// project-owned ("bundled") cores can be launched this way.

import { $ } from "./dom.js";
import { state } from "./state.js";
import { pickFile, pickSaveFile } from "./tauri.js";
import { updateSummary } from "./summary.js";

export const DEFAULT_LOADER = "forwarder/prebuilt/main.dol";
export const DEFAULT_ROM_DIR = "/vcforge/roms";

const selectedCore = () => state.cores.find((c) => c.id === $("#core")?.value);

export const isForwarder = () => state.mode === "forwarder";

/** Can a forwarder WAD launch this core? (Donor-ripped official emulators can't read the launch image.) */
export const canForward = (core) => Boolean(core && (core.forwardable ?? core.source === "bundled"));

/** Where the core DOL is expected on the device when the user doesn't say. */
export const defaultCorePath = (core = selectedCore()) =>
  core ? (core.forwarder_core_path ?? `/vcforge/cores/${core.id}.dol`) : "";

export const deviceRomPath = () => $("#device-rom")?.value.trim() ?? "";
/** The user's core location, or "" to use the default. */
export const deviceCorePath = () => $("#device-core")?.value.trim() ?? "";

/** "sd:/vcforge/roms/game.gb" style display of a device path. */
export const onDevice = (path) => `${state.forwarder.device}:${path}`;

/** Why a forwarder build can't proceed right now, or null if it can. */
export function forwarderProblem() {
  const core = selectedCore();
  if (!canForward(core)) {
    return `${core?.system ?? "This core"} is an official (donor-sourced) core, and forwarder mode can only launch unofficial cores. Pick an unofficial core, or switch to Standard mode.`;
  }
  if (!deviceRomPath()) return "Enter where the ROM will be on the SD card or USB drive (e.g. /vcforge/roms/game.gb).";
  return null;
}

/** Called when a local ROM is picked: use its file name for the device path unless the user typed one. */
export function onRomPicked(name) {
  const input = $("#device-rom");
  if (input && !input.dataset.touched) input.value = `${DEFAULT_ROM_DIR}/${name}`;
}

/** Forget the typed ROM location (wizard reset). */
export function resetForwarderFields() {
  const input = $("#device-rom");
  if (input) {
    input.value = "";
    delete input.dataset.touched;
  }
  const core = $("#device-core");
  if (core) core.value = "";
  const launch = $("#launch-cfg-path");
  if (launch) launch.value = "";
}

/** Show/hide the forwarder-only fields and adjust the wording that depends on the mode. */
export function updateForwarderUi() {
  const on = isForwarder();
  const core = selectedCore();

  document.querySelectorAll(".forwarder-only").forEach((el) => (el.hidden = !on));
  document.querySelectorAll("#mode-picker .check-card").forEach((card) => {
    card.classList.toggle("checked", card.dataset.mode === state.mode);
  });

  const build = $("#build-button");
  if (build) build.textContent = on ? "\u2692 Build forwarder WAD" : "\u2692 Build WAD";
  const hint = $("#rom-hint");
  if (hint) hint.textContent = on ? "Optional: only its name is used" : "Required";

  const corePath = $("#device-core");
  if (corePath) corePath.placeholder = defaultCorePath(core) || "/vcforge/cores/<core>.dol";

  const warning = $("#forwarder-warning");
  if (warning) {
    const blocked = on && core && !canForward(core);
    warning.hidden = !blocked;
    warning.querySelector("span:last-child").textContent = blocked ? forwarderProblem() : "";
  }
}

/** Wire the mode and device pickers and the loader / launch.cfg browse buttons. */
export function initForwarder() {
  $("#mode-picker")?.addEventListener("change", (event) => {
    const radio = event.target.closest?.('input[type="radio"]');
    if (!radio) return;
    state.mode = radio.value;
    updateForwarderUi();
    updateSummary();
  });

  $("#device-picker")?.addEventListener("change", (event) => {
    const radio = event.target.closest?.('input[type="radio"]');
    if (!radio) return;
    state.forwarder.device = radio.value;
    document.querySelectorAll("#device-picker .check-card").forEach((card) => {
      card.classList.toggle("checked", card.dataset.device === radio.value);
    });
    updateSummary();
  });

  $("#device-rom")?.addEventListener("input", (event) => {
    event.target.dataset.touched = "1";
    updateSummary();
  });
  $("#device-core")?.addEventListener("input", updateSummary);

  $("#browse-loader")?.addEventListener("click", async () => {
    const path = await pickFile({
      filters: [{ name: "Wii DOL", extensions: ["dol"] }],
      title: "Select the forwarder loader DOL",
    });
    if (path) $("#loader-path").value = path;
  });

  $("#browse-launch-cfg")?.addEventListener("click", async () => {
    const path = await pickSaveFile({
      defaultPath: "launch.cfg",
      filters: [{ name: "launch.cfg", extensions: ["cfg"] }],
      title: "Save a copy of launch.cfg",
    });
    if (path) $("#launch-cfg-path").value = path;
  });

  updateForwarderUi();
}
