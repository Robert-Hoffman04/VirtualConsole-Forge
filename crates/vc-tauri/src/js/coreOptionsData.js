// Pure helpers for per-core options (no DOM access, so they can be unit
// tested). The rules mirror `vc-core/src/options.rs`:
//
//  - An option is *applicable* when at least one enabled controller is in its
//    `devices` list (or the list is empty) AND its `visible_when` condition
//    holds, evaluated against the *effective* value of an earlier option.
//  - Inapplicable options behave as their default, whatever the user set.
//
// Option definitions come from the registry (`CoreSummary.options`).

export const DEFAULT_GROUP = "General";

/** Equality for option values (booleans, strings and numbers; 1 and 1.0 are the same number in JS). */
export const valuesEqual = (a, b) => a === b;

/** { optionId: default } for every option of the core. */
export function defaultValues(core) {
  return Object.fromEntries((core?.options ?? []).map((o) => [o.id, o.default]));
}

/**
 * Evaluate every option for the enabled controllers (`devices`, an array of ids) and current values.
 * Returns, in registry order:
 *   { option, value, shown, enabled }
 *  - shown:   false when its visible_when condition isn't met (row hidden)
 *  - enabled: false when none of the enabled controllers can use the option
 *             (row shown but disabled, with an explanation)
 *  - value:   the effective value (the default when not applicable)
 */
export function evaluateOptions(core, devices, values) {
  const effective = {};
  const out = [];
  for (const option of core?.options ?? []) {
    const cond = option.visible_when;
    const shown = !cond || (cond.option in effective && valuesEqual(effective[cond.option], cond.equals));
    const deviceOk = !option.devices?.length || option.devices.some((d) => devices.includes(d));
    const enabled = shown && deviceOk;
    const value = enabled && option.id in values ? values[option.id] : option.default;
    effective[option.id] = value;
    out.push({ option, value, shown, enabled });
  }
  return out;
}

/** { optionId: effective value } for the current controller and values. */
export function effectiveValues(core, devices, values) {
  return Object.fromEntries(evaluateOptions(core, devices, values).map((s) => [s.option.id, s.value]));
}

/** Human-readable form of a value, e.g. "On", "Rumble Pak", "60%". */
export function describeValue(option, value) {
  switch (option.kind) {
    case "toggle":
      return value ? "On" : "Off";
    case "select":
      return option.choices.find((c) => c.value === value)?.label ?? String(value);
    default:
      return `${value}${option.unit ?? ""}`;
  }
}

/** Short summary of what differs from the defaults, for the summary panels. */
export function summarizeOptions(core, devices, values) {
  if (!core?.options?.length) return "None available";
  const changed = evaluateOptions(core, devices, values)
    .filter((s) => s.enabled && !valuesEqual(s.value, s.option.default))
    .map(({ option, value }) => {
      if (option.kind === "toggle" && value) return option.label;
      return `${option.label}: ${describeValue(option, value)}`;
    });
  return changed.length ? changed.join(", ") : "Defaults";
}
