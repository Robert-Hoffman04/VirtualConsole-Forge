import { $ } from "./dom.js";

// Prototype-level bookkeeping only. This project intentionally treats a
// built WAD as a one-shot output artifact rather than something tracked
// in a managed library (see docs/DESIGN.md and the Settings view copy).
// The "already used" check below is local-only, per-user bookkeeping to
// help avoid accidental Title ID collisions across a single person's own
// builds -- it is not a shared registry and has no bearing on whether an
// ID collides with anything already installed on an actual console.

function checkId(id) {
  const result = $("#id-result");
  const normalized = id.trim().toUpperCase();
  if (!normalized) {
    result.className = "id-result";
    result.innerHTML = `<div class="id-state">Enter a Title ID</div><div class="id-detail">Nothing to check yet.</div>`;
    return;
  }
  const used = JSON.parse(localStorage.getItem("vcf.usedTitleIds") || "[]");
  const isUsed = used.includes(normalized);
  result.className = `id-result ${isUsed ? "used" : "available"}`;
  result.innerHTML = isUsed
    ? `<div class="id-state">⚠ Already used</div><div class="id-detail">This Title ID is recorded as used by VirtualConsole-Forge.</div>`
    : `<div class="id-state">✓ Available</div><div class="id-detail">No recorded conflict for this Title ID.</div>`;
}

export function markTitleIdUsed(id) {
  const normalized = id.trim().toUpperCase();
  if (!normalized) return;
  const used = JSON.parse(localStorage.getItem("vcf.usedTitleIds") || "[]");
  if (!used.includes(normalized)) {
    localStorage.setItem("vcf.usedTitleIds", JSON.stringify([...used, normalized]));
  }
}

export function initTitleIdChecks() {
  $("#standalone-check").addEventListener("click", () => checkId($("#standalone-id").value));
  $("#check-id-inline").addEventListener("click", () => checkId($("#title-id").value));
}