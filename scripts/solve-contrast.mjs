/**
 * Solves for the `--<token>-strong` lightness each theme needs.
 *
 * A `-strong` token is the same hue as its base at a lightness that stays legible on this
 * theme's surfaces: darker in light mode, lighter in dark mode. It is used for text on a
 * same-hue tint (badge labels) and for solid chart marks, while the pale `/15` fills keep
 * their original colour.
 *
 * Each token must clear the hardest constraint it faces:
 *   4.5:1 as a label on its own `/15` tint over `surface`
 *   4.5:1 as a label on its own `/8` tint over `surface`  (blocked row, selected row)
 *   3:1   as a solid mark on `surface-muted`               (bars, switch knob)
 *
 * Run with `node scripts/solve-contrast.mjs`.
 */

const hslToRgb = ([h, s, l]) => {
  const saturation = s / 100;
  const lightness = l / 100;
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const huePrime = h / 60;
  const secondary = chroma * (1 - Math.abs((huePrime % 2) - 1));
  const [r, g, b] = (() => {
    if (huePrime < 1) return [chroma, secondary, 0];
    if (huePrime < 2) return [secondary, chroma, 0];
    if (huePrime < 3) return [0, chroma, secondary];
    if (huePrime < 4) return [0, secondary, chroma];
    if (huePrime < 5) return [secondary, 0, chroma];
    return [chroma, 0, secondary];
  })();
  const match = lightness - chroma / 2;
  return [r + match, g + match, b + match];
};

const luminance = ([r, g, b]) => {
  const channel = (v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
};

const contrast = (fg, bg) => {
  const [hi, lo] = [luminance(fg), luminance(bg)].sort((a, b) => b - a);
  return (hi + 0.05) / (lo + 0.05);
};

const blend = (over, under, alpha) => over.map((c, i) => c * alpha + under[i] * (1 - alpha));

const THEMES = {
  light: {
    // `-strong` searches downward from the base lightness.
    direction: -1,
    surface: [0, 0, 98],
    surfaceMuted: [0, 0, 96],
    tokens: {
      primary: [220, 90, 56],
      success: [142, 71, 45],
      warning: [38, 92, 50],
      danger: [0, 72, 51],
      "proto-dns": [199, 89, 48],
      "proto-http": [38, 92, 50],
      "proto-tls": [142, 71, 45],
      "proto-quic": [270, 70, 60],
      "proto-icmp": [0, 0, 45],
      "proto-other": [0, 0, 65],
    },
  },
  dark: {
    direction: 1,
    surface: [0, 0, 7],
    surfaceMuted: [0, 0, 10],
    tokens: {
      primary: [213, 94, 68],
      success: [142, 69, 58],
      warning: [38, 92, 60],
      danger: [0, 72, 62],
      "proto-dns": [199, 89, 60],
      "proto-http": [38, 92, 62],
      "proto-tls": [142, 69, 58],
      "proto-quic": [270, 75, 72],
      "proto-icmp": [0, 0, 60],
      "proto-other": [0, 0, 45],
    },
  },
};

/** Tints a token is used behind as a label, and the marks it is used as. */
const CONSTRAINTS = [
  { kind: "label", tint: 0.15, target: 4.5 },
  { kind: "label", tint: 0.08, target: 4.5 },
  { kind: "label", tint: 0.1, target: 4.5 },
  { kind: "mark", tint: null, target: 3 },
];

for (const [themeName, theme] of Object.entries(THEMES)) {
  console.log(`\n${themeName} mode\n`);
  console.log("token          base    -strong   worst ratio at that value");
  for (const [name, [hue, saturation, base]] of Object.entries(theme.tokens)) {
    const surface = hslToRgb(theme.surface);
    const surfaceMuted = hslToRgb(theme.surfaceMuted);
    // The fill keeps the base colour; only the ink moves.
    const fillOf = (tint) => blend(hslToRgb([hue, saturation, base]), surface, tint);

    let solved = null;
    let worst = 0;
    for (let step = 0; step <= 70; step += 0.5) {
      const lightness = base + theme.direction * step;
      if (lightness < 4 || lightness > 96) break;
      const ink = hslToRgb([hue, saturation, lightness]);
      const ratios = CONSTRAINTS.map(({ kind, tint, target }) => ({
        ratio: contrast(ink, kind === "mark" ? surfaceMuted : fillOf(tint)),
        target,
      }));
      if (ratios.every(({ ratio, target }) => ratio >= target)) {
        solved = lightness;
        worst = Math.min(...ratios.map(({ ratio, target }) => ratio / target));
        break;
      }
    }
    const label =
      solved === null
        ? "impossible"
        : solved === base
          ? `${base}% (base is fine)`
          : `${solved}%`;
    console.log(
      `${name.padEnd(14)} ${String(`${base}%`).padEnd(7)} ${label.padEnd(9)} ` +
        `${solved === null ? "" : `${(worst * 100).toFixed(0)}% of the tightest target`}`,
    );
  }

  // Neutral text tokens, measured against the plainest background they sit on.
  const plainest = themeName === "light" ? [0, 0, 100] : [0, 0, 4];
  for (const [name, base] of [["foreground-subtle", themeName === "light" ? 55 : 45]]) {
    let solved = null;
    for (let step = 0; step <= 60; step += 0.5) {
      const lightness = base + theme.direction * step;
      if (contrast(hslToRgb([0, 0, lightness]), hslToRgb(plainest)) >= 4.5) {
        solved = lightness;
        break;
      }
    }
    console.log(`${name.padEnd(14)} ${String(`${base}%`).padEnd(7)} ${solved}%`);
  }
}
