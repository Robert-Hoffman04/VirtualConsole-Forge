import { $ } from "./dom.js";
import { pickFile, pickSaveFile } from "./tauri.js";
import { loadCores } from "./cores.js";

/** Wire the "Browse" buttons next to the output-path and registry-path text inputs. */
export function initPathFields() {
  $("#browse-output").addEventListener("click", async () => {
    const path = await pickSaveFile({
      defaultPath: "output.wad",
      filters: [{ name: "Wii WAD", extensions: ["wad"] }],
      title: "Choose where to save the built WAD",
    });
    if (path) $("#output-path").value = path;
  });

  $("#browse-registry").addEventListener("click", async () => {
    const path = await pickFile({
      filters: [{ name: "JSON", extensions: ["json"] }],
      title: "Select cores/registry.json",
    });
    if (path) {
      $("#registry-path").value = path;
      await loadCores();
    }
  });
}