import { initStepper, initSidebarNav, setStep } from "./nav.js";
import { initTitleIdChecks } from "./titleid.js";
import { initBuild } from "./build.js";
import { initSourceFields } from "./source.js";
import { initPathFields } from "./paths.js";
import { loadCores } from "./cores.js";
import { updateSummary } from "./summary.js";
import { initNativeDragDrop } from "./dropzones.js";

(async function init() {
  initStepper();
  initSidebarNav();
  initTitleIdChecks();
  initBuild();
  initSourceFields();
  initPathFields();

  // Registers the window-level listener that makes drag-and-drop onto any
  // .dropzone actually resolve to a real filesystem path -- see
  // dropzones.js for why this can't be done with plain HTML5 drag events.
  try {
    await initNativeDragDrop();
  } catch (err) {
    console.error("native drag-drop init failed:", err);
  }

  try {
    await loadCores(); // also (re)registers the ROM dropzone for the default core
  } catch (err) {
    console.error("loadCores failed:", err);
  }
  updateSummary();
  setStep(0);
})();