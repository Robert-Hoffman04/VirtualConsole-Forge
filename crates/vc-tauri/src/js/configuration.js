// Entry points for "something on the Configuration step changed": rebuild
// everything that depends on it, in the right order.

import { resetMapping, syncMappingRows } from "./mapping.js";
import { refreshCoreOptions, applyOptionState } from "./coreOptions.js";
import { updateSummary } from "./summary.js";
import { refreshConfigPreview } from "./configPreview.js";

/**
 * The selected core changed. Order matters: options first (their
 * applicability depends on which controllers are enabled), then bindings
 * (which buttons exist depends on the options).
 */
export function refreshConfiguration() {
  refreshCoreOptions();
  resetMapping();
  updateSummary();
  refreshConfigPreview();
}

/** A controller was checked or unchecked: options that are controller-specific may (un)apply. */
export function onDevicesChanged() {
  applyOptionState();
  syncMappingRows();
  updateSummary();
  refreshConfigPreview();
}

/** A core option changed: buttons may appear/disappear. */
export function onOptionsChanged() {
  syncMappingRows();
  updateSummary();
  refreshConfigPreview();
}
