// Thin wrapper around the Tauri v2 core IPC bridge.
//
// This project has no JS bundler and doesn't pull in npm packages like
// `@tauri-apps/plugin-dialog` -- every Tauri plugin command is reachable
// through the core `invoke()` bridge (exposed globally because
// `app.withGlobalTauri` is set in tauri.conf.json) using the
// `plugin:<name>|<command>` naming convention, so a raw fetch-style
// invoke call is all that's needed.

export function hasTauri() {
  return Boolean(window.__TAURI__?.core?.invoke);
}

export async function invoke(command, args) {
  if (!hasTauri()) return null;
  return window.__TAURI__.core.invoke(command, args);
}

/**
 * Open a native "pick a file" dialog (tauri-plugin-dialog). Returns an
 * absolute path, or null if the user cancelled or no Tauri host is
 * present (e.g. previewing this UI in a plain browser).
 */
export async function pickFile({ filters, title } = {}) {
  const result = await invoke("plugin:dialog|open", {
    multiple: false,
    directory: false,
    filters,
    title,
  });
  return typeof result === "string" ? result : null;
}

/**
 * Open a native "save file as" dialog. Returns an absolute path, or null
 * if cancelled / no Tauri host present.
 */
export async function pickSaveFile({ defaultPath, filters, title } = {}) {
  const result = await invoke("plugin:dialog|save", { defaultPath, filters, title });
  return typeof result === "string" ? result : null;
}

/**
 * Convert an absolute local filesystem path into a URL the webview is
 * allowed to load (used for cover-art preview). Requires
 * `app.security.assetProtocol` to be enabled with a scope covering the
 * path, which tauri.conf.json sets broadly ("**") since users pick cover
 * art from anywhere on disk -- see docs/DESIGN.md for the tradeoff.
 */
export function assetUrl(path) {
  if (!path || !window.__TAURI__?.core?.convertFileSrc) return "";
  return window.__TAURI__.core.convertFileSrc(path);
}

/**
 * Subscribe to whole-window native drag-and-drop. Tauri v2 delivers real
 * absolute file paths this way -- unlike the browser's HTML5 DragEvent,
 * which the webview does not populate with a usable `path` for security
 * reasons, so this is what actually makes drag-and-drop functional
 * against the backend rather than just a visual affordance.
 *
 * Returns an unlisten function, or a no-op if Tauri isn't present.
 */
export async function onWindowDragDrop(handler) {
  if (!window.__TAURI__?.event?.listen) return () => {};
  return window.__TAURI__.event.listen("tauri://drag-drop", (event) => {
    handler(event.payload);
  });
}