// Entry points for "something on the Configuration step changed": rebuild
// everything that depends on it, in the right order.

import { syncControllerChoices, resetMapping, syncMappingRows } from "./mapping.js";
import { refreshCoreOptions } from "./coreOptions.js";
import { updateSummary } from "./summary.js";
import { refreshConfigPreview } from "./configPreview.js";

/**
 * The selected core or controller changed. Order matters: controller
 * availability first (it can switch the controller), then options (their
 * applicability depends on the controller), then bindings (which buttons
 * exist depends on the options).
 */
export function refreshConfiguration() {
  syncControllerChoices();
  refreshCoreOptions();
  resetMapping();
  updateSummary();
  refreshConfigPreview();
}

/** A core option changed: buttons may appear/disappear. */
export function onOptionsChanged() {
  syncMappingRows();
  updateSummary();
  refreshConfigPreview();
}
