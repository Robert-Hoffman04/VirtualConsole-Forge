import { $ } from "./dom.js";
import { state } from "./state.js";
import { registerDropzone } from "./dropzones.js";
import { assetUrl } from "./tauri.js";
import { updateSummary } from "./summary.js";
import { updateCoreRequirements } from "./cores.js";

/** Wire the cover-art dropzone and the plain (non-file) Source-step inputs. Note: the ROM
 * dropzone is (re)registered by cores.js#updateCoreRequirements, since its file-type filter
 * depends on which core is selected. */
export function initSourceFields() {
  registerDropzone("cover-drop", {
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "bmp"] }],
    title: "Select cover art",
    onPick: (path, name) => {
      state.cover = { path, name };
      $("#cover-name").textContent = name;

      const previewUrl = assetUrl(path);
      ["#source-preview-art", "#large-preview-art"].forEach((sel) => {
        const art = $(sel);
        if (previewUrl) {
          art.classList.remove("placeholder");
          art.style.backgroundImage = `url("${previewUrl}")`;
        }
      });
      updateSummary();
    },
  });

  $("#title").addEventListener("input", updateSummary);
  $("#title-id").addEventListener("input", updateSummary);
  $("#controller").addEventListener("change", updateSummary);
  $("#core").addEventListener("change", updateCoreRequirements);
}