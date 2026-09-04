import { $, $$ } from "./dom.js";
import { state } from "./state.js";
import { updateSummary } from "./summary.js";

const STEP_COUNT = 4;

export function setStep(step) {
  state.step = Math.max(0, Math.min(STEP_COUNT - 1, step));
  $$(".step").forEach((el, i) => {
    el.classList.toggle("current", i === state.step);
    el.classList.toggle("complete", i < state.step);
  });
  $$(".step-panel").forEach((el, i) => (el.hidden = i !== state.step));
  updateSummary();
}

export function initStepper() {
  $$(".step").forEach((el) =>
    el.addEventListener("click", () => {
      const target = Number(el.dataset.step);
      if (target <= state.step + 1) setStep(target);
    })
  );
  $$(".next-step").forEach((b) => b.addEventListener("click", () => setStep(state.step + 1)));
  $$(".previous-step").forEach((b) => b.addEventListener("click", () => setStep(state.step - 1)));
}

const VIEW_META = {
  build: ["New Build", "Create and validate a Virtual Console title."],
  ids: ["Title ID Checker", "Check Title IDs for conflicts without managing WADs."],
  settings: ["Settings", "Application-level configuration."],
};

export function initSidebarNav() {
  $$(".nav button").forEach((button) =>
    button.addEventListener("click", () => {
      $$(".nav button").forEach((b) => b.classList.remove("active"));
      button.classList.add("active");
      $$(".view").forEach((v) => v.classList.remove("active"));
      $(`#view-${button.dataset.view}`).classList.add("active");

      const meta = VIEW_META[button.dataset.view];
      $("#page-title").textContent = meta[0];
      $("#page-description").textContent = meta[1];
    })
  );
}