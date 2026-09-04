// Small DOM helpers shared across the frontend. Kept dependency-free and
// framework-free on purpose -- this project has no JS bundler, so every
// other module imports directly from here.

/** querySelector shorthand. */
export function $(selector, root = document) {
  return root.querySelector(selector);
}

/** querySelectorAll shorthand, returned as a real array (not a NodeList) so callers can map/filter directly. */
export function $$(selector, root = document) {
  return Array.from(root.querySelectorAll(selector));
}

/** Escape text for safe interpolation into innerHTML. */
export function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Last path segment, handling both `/` and `\` separators (absolute paths from the native file dialog can be either depending on OS). */
export function basename(path) {
  if (!path) return "";
  const parts = String(path).split(/[\\/]/);
  return parts[parts.length - 1] || path;
}