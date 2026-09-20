// Controller illustrations for the Configuration step. Each controller type
// (the `device` ids used in cores/registry.json) maps to an inline SVG, so
// there are no image files to load. Panels call `controllerArt(device)` when
// they render, rather than relying on one shared element being updated.

const label = (x, y, text, size = 7, fill = "#cbd3dc") =>
  `<text x="${x}" y="${y}" font-size="${size}" fill="${fill}" text-anchor="middle" dominant-baseline="central" font-family="system-ui, sans-serif">${text}</text>`;

const dpad = (cx, cy, fill) =>
  `<path d="M${cx - 4} ${cy - 12}h8v8h8v8h-8v8h-8v-8h-8v-8h8z" fill="${fill}"/>`;

const CLASSIC = `
<svg viewBox="0 0 200 140" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
  <rect x="24" y="3" width="44" height="10" rx="5" fill="#4a5561"/>
  <rect x="132" y="3" width="44" height="10" rx="5" fill="#4a5561"/>
  ${label(46, 8, "L", 6)}${label(154, 8, "R", 6)}
  <rect x="30" y="15" width="32" height="9" rx="4" fill="#5d6a77"/>
  <rect x="138" y="15" width="32" height="9" rx="4" fill="#5d6a77"/>
  ${label(46, 19.5, "ZL", 5)}${label(154, 19.5, "ZR", 5)}
  <path d="M40 28H160C178 28 190 44 188 68L182 106C180 121 166 125 158 115L146 97H54L42 115C34 125 20 121 18 106L12 68C10 44 22 28 40 28Z"
        fill="#e4e8ec" stroke="#9aa5b1" stroke-width="2"/>
  ${dpad(50, 50, "#3a444e")}
  <circle cx="76" cy="80" r="12" fill="#c9d0d7" stroke="#8a95a1" stroke-width="2"/>
  <circle cx="76" cy="80" r="7" fill="#3a444e"/>
  <circle cx="124" cy="80" r="12" fill="#c9d0d7" stroke="#8a95a1" stroke-width="2"/>
  <circle cx="124" cy="80" r="7" fill="#3a444e"/>
  <circle cx="148" cy="38" r="7" fill="#56626f"/>${label(148, 38, "X")}
  <circle cx="148" cy="62" r="7" fill="#56626f"/>${label(148, 62, "B")}
  <circle cx="136" cy="50" r="7" fill="#56626f"/>${label(136, 50, "Y")}
  <circle cx="160" cy="50" r="7" fill="#56626f"/>${label(160, 50, "A")}
  <circle cx="88" cy="50" r="4" fill="#56626f"/>
  <circle cx="100" cy="50" r="5" fill="#56626f"/>
  <circle cx="112" cy="50" r="4" fill="#56626f"/>
</svg>`;

const WIIMOTE_SIDEWAYS = `
<svg viewBox="0 0 200 140" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
  <rect x="12" y="40" width="176" height="60" rx="12" fill="#e4e8ec" stroke="#9aa5b1" stroke-width="2"/>
  <rect x="12" y="40" width="26" height="60" rx="12" fill="#d3d9df"/>
  ${dpad(58, 70, "#3a444e")}
  <circle cx="100" cy="58" r="9" fill="#56626f"/>${label(100, 58, "A", 8)}
  <circle cx="82" cy="84" r="4" fill="#56626f"/>${label(82, 84, "−", 6)}
  <circle cx="100" cy="84" r="5" fill="#56626f"/>
  <circle cx="118" cy="84" r="4" fill="#56626f"/>${label(118, 84, "+", 6)}
  <circle cx="142" cy="70" r="9" fill="#56626f"/>${label(142, 70, "1", 8)}
  <circle cx="168" cy="70" r="9" fill="#56626f"/>${label(168, 70, "2", 8)}
</svg>`;

const GAMECUBE = `
<svg viewBox="0 0 200 140" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
  <rect x="26" y="12" width="40" height="11" rx="5" fill="#4a5561"/>
  <rect x="134" y="12" width="40" height="11" rx="5" fill="#4a5561"/>
  ${label(46, 17.5, "L", 6)}${label(154, 17.5, "R", 6)}
  <rect x="150" y="1" width="22" height="7" rx="3" fill="#3d4a8a"/>${label(161, 4.5, "Z", 5)}
  <path d="M46 26H154C176 26 192 44 190 68L184 104C182 120 164 122 156 110L146 96H54L44 110C36 122 18 120 16 104L10 68C8 44 24 26 46 26Z"
        fill="#6f6bc4" stroke="#4c4899" stroke-width="2"/>
  <circle cx="46" cy="52" r="13" fill="#c9d0d7" stroke="#8a95a1" stroke-width="2"/>
  <circle cx="46" cy="52" r="8" fill="#3a444e"/>
  ${dpad(74, 88, "#d5d9f0")}
  <circle cx="100" cy="52" r="5" fill="#d5d9f0"/>
  <circle cx="150" cy="62" r="14" fill="#4cc38a" stroke="#2f8f63" stroke-width="2"/>${label(150, 62, "A", 10, "#0e2a1d")}
  <circle cx="131" cy="78" r="7" fill="#e56a6a"/>${label(131, 78, "B", 7, "#3a1010")}
  <ellipse cx="172" cy="60" rx="5" ry="8" fill="#d5d9f0" transform="rotate(20 172 60)"/>${label(172, 60, "X", 6, "#2a2d55")}
  <ellipse cx="148" cy="38" rx="8" ry="5" fill="#d5d9f0" transform="rotate(20 148 38)"/>${label(148, 38, "Y", 6, "#2a2d55")}
  <circle cx="117" cy="89" r="8" fill="#e8b45c" stroke="#b58a3e" stroke-width="2"/>
</svg>`;

const WIIMOTE_NUNCHUK = `
<svg viewBox="0 0 200 140" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
  <path d="M44 132C44 140 138 140 138 124" fill="none" stroke="#56626f" stroke-width="3" stroke-linecap="round"/>
  <rect x="22" y="6" width="44" height="126" rx="10" fill="#e4e8ec" stroke="#9aa5b1" stroke-width="2"/>
  ${dpad(44, 30, "#3a444e")}
  <circle cx="44" cy="58" r="9" fill="#56626f"/>${label(44, 58, "A", 8)}
  <circle cx="32" cy="82" r="4" fill="#56626f"/>${label(32, 82, "\u2212", 6)}
  <circle cx="44" cy="82" r="5" fill="#56626f"/>
  <circle cx="56" cy="82" r="4" fill="#56626f"/>${label(56, 82, "+", 6)}
  <circle cx="44" cy="102" r="6" fill="#56626f"/>${label(44, 102, "1", 6)}
  <circle cx="44" cy="119" r="6" fill="#56626f"/>${label(44, 119, "2", 6)}
  <path d="M138 8C160 8 168 36 166 72C165 102 154 124 138 124C122 124 111 102 110 72C108 36 116 8 138 8Z"
        fill="#e4e8ec" stroke="#9aa5b1" stroke-width="2"/>
  <circle cx="138" cy="38" r="13" fill="#c9d0d7" stroke="#8a95a1" stroke-width="2"/>
  <circle cx="138" cy="38" r="8" fill="#3a444e"/>
  <circle cx="138" cy="70" r="8" fill="#56626f"/>${label(138, 70, "C", 8)}
  <rect x="120" y="88" width="36" height="14" rx="7" fill="#56626f"/>${label(138, 95, "Z", 8)}
</svg>`;

export const CONTROLLERS = {
  classic_controller: { name: "Classic Controller", svg: CLASSIC },
  wiimote_sideways: { name: "Wiimote (sideways)", svg: WIIMOTE_SIDEWAYS },
  wiimote_nunchuk: { name: "Wiimote + Nunchuk", svg: WIIMOTE_NUNCHUK },
  gamecube: { name: "GameCube Controller", svg: GAMECUBE },
};

/**
 * Illustration for a controller id, as { name, svg }. Each controller panel
 * draws its own, so the pictures never depend on any single shared element.
 */
export function controllerArt(device) {
  return CONTROLLERS[device] ?? CONTROLLERS.classic_controller;
}
