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
  await initNativeDragDrop();

  await loadCores(); // also (re)registers the ROM dropzone for the default core
  updateSummary();
  setStep(0);
})();