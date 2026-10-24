/**
 * Asserts the token pairs the UI actually renders against `ui_rules.md` §10: body text
 * >= 4.5:1, large/UI marks >= 3:1, in both themes. Exits non-zero on a regression, so the
 * contrast pass is repeatable rather than a one-off eyeball.
 *
 * Each entry names the real class combination from a component, so adding a pair here
 * means "the UI renders this". See `ui_tokens.md` §1.1 for base vs `-strong`.
 *
 * Run with `node scripts/check-contrast.mjs`.
 */

const LIGHT = {
  background: [0, 0, 100],
  surface: [0, 0, 98],
  "surface-muted": [0, 0, 96],
  foreground: [0, 0, 9],
  "foreground-muted": [0, 0, 40],
  "foreground-subtle": [0, 0, 44],
  border: [0, 0, 92],
  "border-strong": [0, 0, 85],
  ring: [0, 0, 9],
  primary: [220, 90, 56],
  "primary-strong": [220, 90, 48],
  secondary: [0, 0, 9],
  "secondary-foreground": [0, 0, 100],
  success: [142, 71, 45],
  "success-strong": [142, 71, 27],
  warning: [38, 92, 50],
  "warning-strong": [38, 92, 30],
  danger: [0, 72, 51],
  "danger-strong": [0, 72, 44],
  "danger-foreground": [0, 0, 100],
  "proto-dns": [199, 89, 48],
  "proto-dns-strong": [199, 89, 32],
  "proto-http": [38, 92, 50],
  "proto-http-strong": [38, 92, 30],
  "proto-tls": [142, 71, 45],
  "proto-tls-strong": [142, 71, 27],
  "proto-quic": [270, 70, 60],
  "proto-quic-strong": [270, 70, 53],
  "proto-icmp": [0, 0, 45],
  "proto-icmp-strong": [0, 0, 39],
  "proto-other": [0, 0, 65],
  "proto-other-strong": [0, 0, 41],
  "scrollbar-thumb": [0, 0, 52],
  "scrollbar-thumb-hover": [0, 0, 35],
};

const DARK = {
  background: [0, 0, 4],
  surface: [0, 0, 7],
  "surface-muted": [0, 0, 10],
  foreground: [0, 0, 98],
  "foreground-muted": [0, 0, 63],
  "foreground-subtle": [0, 0, 51],
  border: [0, 0, 15],
  "border-strong": [0, 0, 22],
  ring: [0, 0, 83],
  primary: [213, 94, 68],
  "primary-strong": [213, 94, 68],
  secondary: [0, 0, 98],
  "secondary-foreground": [0, 0, 9],
  success: [142, 69, 58],
  "success-strong": [142, 69, 58],
  warning: [38, 92, 60],
  "warning-strong": [38, 92, 60],
  danger: [0, 72, 62],
  "danger-strong": [0, 72, 64],
  "danger-foreground": [0, 0, 9],
  "proto-dns": [199, 89, 60],
  "proto-dns-strong": [199, 89, 60],
  "proto-http": [38, 92, 62],
  "proto-http-strong": [38, 92, 62],
  "proto-tls": [142, 69, 58],
  "proto-tls-strong": [142, 69, 58],
  "proto-quic": [270, 75, 72],
  "proto-quic-strong": [270, 75, 72],
  "proto-icmp": [0, 0, 60],
  "proto-icmp-strong": [0, 0, 60],
  "proto-other": [0, 0, 45],
  "proto-other-strong": [0, 0, 54],
  "scrollbar-thumb": [0, 0, 45],
  "scrollbar-thumb-hover": [0, 0, 60],
};

/**
 * `{ ink, on, tint?, tintWith?, min, where }`
 *   ink      — the foreground token
 *   on       — the surface underneath
 *   tint     — alpha of a fill between them (the `bg-x/15` pattern)
 *   tintWith — which token is the fill; defaults to `ink`'s base
 *   min      — required ratio: 4.5 for text, 3 for non-text marks
 */
const PAIRS = [
  // Neutral text on every surface it can land on.
  { ink: "foreground", on: "background", min: 4.5, where: "body text" },
  { ink: "foreground", on: "surface", min: 4.5, where: "body text on a card" },
  { ink: "foreground", on: "surface-muted", min: 4.5, where: "body text on a hovered row" },
  { ink: "foreground-muted", on: "background", min: 4.5, where: "secondary text" },
  { ink: "foreground-muted", on: "surface", min: 4.5, where: "table headers, stat labels" },
  { ink: "foreground-muted", on: "surface-muted", min: 4.5, where: "secondary text, hovered row" },
  { ink: "foreground-subtle", on: "background", min: 4.5, where: "input placeholders" },
  { ink: "foreground-subtle", on: "surface", min: 4.5, where: "form hints, menu headings" },
  { ink: "foreground-subtle", on: "surface-muted", min: 4.5, where: "hints on a hovered row" },

  // Badge labels: `-strong` ink on the base token's own /15 tint.
  { ink: "success-strong", on: "surface", tint: 0.15, tintWith: "success", min: 4.5, where: "Live badge" },
  { ink: "danger-strong", on: "surface", tint: 0.15, tintWith: "danger", min: 4.5, where: "Blocked badge" },
  { ink: "warning-strong", on: "surface", tint: 0.15, tintWith: "warning", min: 4.5, where: "Paused badge" },
  { ink: "proto-dns-strong", on: "surface", tint: 0.15, tintWith: "proto-dns", min: 4.5, where: "DNS badge" },
  { ink: "proto-http-strong", on: "surface", tint: 0.15, tintWith: "proto-http", min: 4.5, where: "HTTP badge" },
  { ink: "proto-tls-strong", on: "surface", tint: 0.15, tintWith: "proto-tls", min: 4.5, where: "TLS badge" },
  { ink: "proto-quic-strong", on: "surface", tint: 0.15, tintWith: "proto-quic", min: 4.5, where: "QUIC badge" },
  { ink: "proto-icmp-strong", on: "surface", tint: 0.15, tintWith: "proto-icmp", min: 4.5, where: "ICMP badge" },
  { ink: "proto-other-strong", on: "surface", tint: 0.15, tintWith: "proto-other", min: 4.5, where: "Other badge" },

  // Status pill and error surfaces use a /10 tint.
  { ink: "success-strong", on: "surface", tint: 0.1, tintWith: "success", min: 4.5, where: "status pill, healthy" },
  { ink: "warning-strong", on: "surface", tint: 0.1, tintWith: "warning", min: 4.5, where: "status pill, capture only" },
  { ink: "danger-strong", on: "surface", tint: 0.1, tintWith: "danger", min: 4.5, where: "status pill + ErrorState icon" },
  { ink: "primary-strong", on: "surface", tint: 0.1, tintWith: "primary", min: 4.5, where: "active nav item" },
  { ink: "foreground-muted", on: "surface", tint: 0.1, tintWith: "danger", min: 4.5, where: "ErrorState detail text" },

  // Row tints at 8%.
  { ink: "danger-strong", on: "surface", tint: 0.08, tintWith: "danger", min: 4.5, where: "rule name on a blocked row" },
  { ink: "foreground", on: "surface", tint: 0.08, tintWith: "danger", min: 4.5, where: "cells on a blocked row" },
  { ink: "foreground", on: "surface", tint: 0.08, tintWith: "primary", min: 4.5, where: "cells on the selected row" },
  { ink: "foreground-muted", on: "surface", tint: 0.08, tintWith: "danger", min: 4.5, where: "muted cells, blocked row" },

  // Solid fills with their own foreground.
  { ink: "secondary-foreground", on: "secondary", min: 4.5, where: "primary button" },
  { ink: "danger-foreground", on: "danger", min: 4.5, where: "destructive button" },

  // Text tokens on a plain surface, no tint.
  { ink: "danger-strong", on: "surface", min: 4.5, where: "danger menu item, inline error" },
  { ink: "success-strong", on: "surface", min: 4.5, where: "success StatTile value" },
  { ink: "warning-strong", on: "surface", min: 4.5, where: "warning StatTile value" },
  { ink: "primary-strong", on: "surface", min: 4.5, where: "primary StatTile value" },

  // Non-text marks: 3:1 is the bar (ui_rules.md §10).
  { ink: "primary-strong", on: "surface", min: 3, where: "sparkline stroke" },
  { ink: "primary-strong", on: "surface-muted", min: 3, where: "top-talker proportion bar" },
  { ink: "danger-strong", on: "surface-muted", min: 3, where: "blocked proportion bar" },
  { ink: "success-strong", on: "surface-muted", min: 3, where: "enabled switch knob" },
  { ink: "foreground-subtle", on: "surface-muted", min: 3, where: "disabled switch knob" },
  { ink: "proto-dns-strong", on: "surface-muted", min: 3, where: "DNS bar segment + legend dot" },
  { ink: "proto-http-strong", on: "surface-muted", min: 3, where: "HTTP bar segment + legend dot" },
  { ink: "proto-tls-strong", on: "surface-muted", min: 3, where: "TLS bar segment + legend dot" },
  { ink: "proto-quic-strong", on: "surface-muted", min: 3, where: "QUIC bar segment + legend dot" },
  { ink: "proto-icmp-strong", on: "surface-muted", min: 3, where: "ICMP bar segment + legend dot" },
  { ink: "proto-other-strong", on: "surface-muted", min: 3, where: "Other bar segment + legend dot" },
  { ink: "primary-strong", on: "surface", tint: 0.08, tintWith: "primary", min: 3, where: "selected-row accent bar" },
  // The scrollbar track is transparent, so the thumb is a UI mark on whatever it crosses:
  // `surface` in a table body, `surface-muted` on the row under the cursor, `background`
  // on the page itself. All three have to hold 3:1.
  { ink: "scrollbar-thumb", on: "background", min: 3, where: "scrollbar thumb on the page" },
  { ink: "scrollbar-thumb", on: "surface", min: 3, where: "scrollbar thumb in a table body" },
  { ink: "scrollbar-thumb", on: "surface-muted", min: 3, where: "scrollbar thumb over a hovered row" },
  { ink: "scrollbar-thumb-hover", on: "surface-muted", min: 3, where: "scrollbar thumb, grabbed" },
  // Borders are deliberately excluded. `ui_tokens.md` §4 specifies 1px hairlines as
  // decoration — the Geist look depends on them being quiet — and no state is conveyed by
  // a border alone: an invalid field pairs `border-danger` with text, and focus uses a 2px
  // ring against `--ring`, which is what actually has to be visible.
  { ink: "ring", on: "background", min: 3, where: "focus ring" },
  { ink: "ring", on: "surface", min: 3, where: "focus ring on a card" },
];

function hslToRgb([h, s, l]) {
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
}

function luminance([r, g, b]) {
  const channel = (value) => (value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(foreground, background) {
  const [light, dark] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (light + 0.05) / (dark + 0.05);
}

/** Composites `over` at `alpha` on `under`, the way a `/15` fill renders. */
function blend(over, under, alpha) {
  return over.map((channel, index) => channel * alpha + under[index] * (1 - alpha));
}

let failures = 0;
for (const [themeName, tokens] of [
  ["light", LIGHT],
  ["dark", DARK],
]) {
  const rows = [];
  for (const { ink, on, tint, tintWith, min, where } of PAIRS) {
    const inkColor = tokens[ink];
    const baseColor = tokens[on];
    if (!inkColor || !baseColor) throw new Error(`unknown token in pair: ${ink} on ${on}`);

    const foreground = hslToRgb(inkColor);
    let background = hslToRgb(baseColor);
    if (tint !== undefined) {
      const fillToken = tokens[tintWith ?? ink];
      if (!fillToken) throw new Error(`unknown tint token: ${tintWith}`);
      background = blend(hslToRgb(fillToken), background, tint);
    }

    const ratio = contrast(foreground, background);
    const ok = ratio >= min;
    if (!ok) failures += 1;
    rows.push({ ok, ratio, min, where, ink, on, tint });
  }

  console.log(`\n${themeName} mode — ${rows.filter((r) => r.ok).length}/${rows.length} pass`);
  for (const row of rows.filter((r) => !r.ok)) {
    console.log(
      `  FAIL ${row.ratio.toFixed(2)}:1 (needs ${row.min}:1)  ` +
        `${row.ink} on ${row.on}${row.tint === undefined ? "" : `/${row.tint * 100}`} — ${row.where}`,
    );
  }
  // The tightest passing margin is worth seeing: it shows how much headroom a token has.
  const tightest = rows
    .filter((r) => r.ok)
    .sort((a, b) => a.ratio / a.min - b.ratio / b.min)
    .slice(0, 3);
  for (const row of tightest) {
    console.log(
      `  ok   ${row.ratio.toFixed(2)}:1 (needs ${row.min}:1)  ${row.where} — tightest margin`,
    );
  }
}

console.log(
  failures === 0
    ? `\nAll ${PAIRS.length * 2} rendered pairs meet ui_rules.md §10.`
    : `\n${failures} pair(s) below target.`,
);
process.exit(failures === 0 ? 0 : 1);
