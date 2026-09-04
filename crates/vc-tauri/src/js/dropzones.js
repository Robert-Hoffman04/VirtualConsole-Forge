// Wires up `.dropzone` elements to real filesystem paths, via two
// complementary paths:
//
// 1. Click -> native "open file" dialog (tauri.js#pickFile). Always gives
//    a real absolute path.
// 2. Native window-level drag-and-drop (tauri.js#onWindowDragDrop) -> we
//    hit-test the drop position against registered zones and dispatch the
//    dropped path to whichever one the cursor was over. This is real
//    OS-level drag-drop, not the browser's HTML5 DragEvent (which the
//    Tauri v2 webview doesn't populate with a usable path).
//
// The plain `dragenter`/`dragover`/`dragleave` DOM listeners below are
// kept only for the `.dragging` CSS hover affordance; they don't carry
// any file data themselves.

import { $ } from "./dom.js";
import { basename } from "./dom.js";
import { pickFile, onWindowDragDrop } from "./tauri.js";

const zones = new Map(); // element id -> { filters, title, onPick }
const wiredElements = new WeakSet(); // elements that already have DOM listeners attached

/**
 * Register a `.dropzone` element by id. `onPick(path, name)` is called
 * with a real absolute path whenever the user picks or drops a file on
 * it. `filters` follows tauri-plugin-dialog's filter shape:
 * [{ name: "ROM files", extensions: ["nes"] }].
 *
 * Safe to call repeatedly for the same id (e.g. to refresh the filter
 * list when the selected core changes) -- the zone's config is always
 * updated, but DOM listeners are attached to a given element only once,
 * so re-registering never stacks duplicate click/drag handlers.
 */
export function registerDropzone(id, { filters, title, onPick } = {}) {
  zones.set(id, { filters, title, onPick });

  const el = $(`#${id}`);
  if (!el || wiredElements.has(el)) return;
  wiredElements.add(el);

  el.addEventListener("click", () => pickForZone(id));

  ["dragenter", "dragover"].forEach((evt) =>
    el.addEventListener(evt, (ev) => {
      ev.preventDefault();
      el.classList.add("dragging");
    })
  );
  ["dragleave", "drop"].forEach((evt) =>
    el.addEventListener(evt, (ev) => {
      ev.preventDefault();
      el.classList.remove("dragging");
    })
  );
}

async function pickForZone(id) {
  const zone = zones.get(id);
  if (!zone) return;
  const path = await pickFile({ filters: zone.filters, title: zone.title });
  if (path) zone.onPick(path, basename(path));
}

/**
 * Start listening for native window-level drag-drop and route dropped
 * files to whichever registered zone the cursor was over. Call this once
 * during app init. Safe to call even before any zones are registered --
 * later drops just look them up at drop time.
 */
export async function initNativeDragDrop() {
  return onWindowDragDrop((payload) => {
    const paths = payload?.paths;
    const position = payload?.position;
    if (!paths?.length || !position) return;

    // Drag-drop position is reported in physical pixels; elementFromPoint
    // expects CSS pixels.
    const scale = window.devicePixelRatio || 1;
    const target = document.elementFromPoint(position.x / scale, position.y / scale);
    const zoneEl = target?.closest(".dropzone");
    if (!zoneEl) return;

    const zone = zones.get(zoneEl.id);
    if (!zone) return;
    zone.onPick(paths[0], basename(paths[0]));
  });
}