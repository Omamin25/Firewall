# UI Tokens — Cognton Firewall

Design primitives. **Every visual value in the app comes from here.** No raw hex/px in components. Aesthetic: **Vercel / Geist** — minimal, high-contrast, monochrome base with restrained accent and semantic colors. Implemented as CSS variables (HSL) consumed by `tailwind.config.ts`.

## 1. Color — CSS variables (`src/styles/tokens.css`)
Neutral-first, near-black/near-white. Semantic tokens map to raw scales so components reference *roles*, not shades.

```css
:root {
  /* Surfaces */
  --background:        0 0% 100%;      /* app bg (light) */
  --surface:           0 0% 98%;       /* cards/panels */
  --surface-muted:     0 0% 96%;       /* subtle fills, table stripes */
  --foreground:        0 0% 9%;        /* primary text */
  --foreground-muted:  0 0% 40%;       /* secondary text */
  --foreground-subtle: 0 0% 44%;       /* tertiary/placeholder — 4.5:1 on every surface */
  --border:            0 0% 92%;       /* hairline borders */
  --border-strong:     0 0% 85%;
  --ring:              0 0% 9%;        /* focus ring (matches fg) */

  /* Brand accent (used sparingly) */
  --primary:           220 90% 56%;    /* action blue */
  --primary-foreground: 0 0% 100%;
  --secondary:         0 0% 9%;        /* solid dark button (Geist) */
  --secondary-foreground: 0 0% 100%;
  --accent:            270 70% 60%;    /* rare highlights / active nav */

  /* Semantic — firewall status */
  --success:           142 71% 45%;    /* allowed / healthy */
  --success-foreground: 0 0% 100%;
  --warning:           38 92% 50%;     /* attention */
  --warning-foreground: 0 0% 9%;
  --danger:            0 72% 51%;       /* blocked / drop */
  --danger-foreground: 0 0% 100%;
  --info:              199 89% 48%;

  /* Protocol legend (charts / L7 badges) — tokenized, colorblind-aware */
  --proto-dns:  199 89% 48%;
  --proto-http: 38 92% 50%;
  --proto-tls:  142 71% 45%;
  --proto-quic: 270 70% 60%;
  --proto-icmp: 0 0% 45%;
  --proto-other: 0 0% 65%;

  /* Legible pairs (see §1.1). Same hue, adjusted lightness — for text on a same-hue
     tint and for solid chart marks. The base tokens above stay the fill colour. */
  --primary-strong:    220 90% 48%;
  --success-strong:    142 71% 27%;
  --warning-strong:    38 92% 30%;
  --danger-strong:     0 72% 44%;
  --proto-dns-strong:  199 89% 32%;
  --proto-http-strong: 38 92% 30%;
  --proto-tls-strong:  142 71% 27%;
  --proto-quic-strong: 270 70% 53%;
  --proto-icmp-strong: 0 0% 39%;
  --proto-other-strong: 0 0% 41%;

  /* Elevation */
  --shadow-sm: 0 1px 2px 0 hsl(0 0% 0% / 0.05);
  --shadow-md: 0 4px 12px -2px hsl(0 0% 0% / 0.08);
  --radius: 0.5rem;
}

.dark {
  --background:        0 0% 4%;         /* near-black */
  --surface:           0 0% 7%;
  --surface-muted:     0 0% 10%;
  --foreground:        0 0% 98%;
  --foreground-muted:  0 0% 63%;
  --foreground-subtle: 0 0% 51%;        /* 4.5:1 on every dark surface */
  --border:            0 0% 15%;
  --border-strong:     0 0% 22%;
  --ring:              0 0% 83%;

  --primary:           213 94% 68%;
  --primary-foreground: 0 0% 4%;
  --secondary:         0 0% 98%;        /* light solid button on dark */
  --secondary-foreground: 0 0% 9%;
  --accent:            270 75% 70%;

  --success: 142 69% 58%;  --warning: 38 92% 60%;
  --danger:  0 72% 62%;    --info:    199 89% 60%;
  /* The dark danger fill is light enough that white text on it is only 3.59:1; the
     near-black used elsewhere in this theme gives 4.99:1. */
  --danger-foreground: 0 0% 9%;

  --proto-dns: 199 89% 60%; --proto-http: 38 92% 62%; --proto-tls: 142 69% 58%;
  --proto-quic: 270 75% 72%; --proto-icmp: 0 0% 60%; --proto-other: 0 0% 45%;

  /* Dark surfaces are already generous, so most `-strong` pairs are the base value.
     Only danger and proto-other need lifting off their own tint. */
  --primary-strong: 213 94% 68%; --success-strong: 142 69% 58%;
  --warning-strong: 38 92% 60%;  --danger-strong:  0 72% 64%;
  --proto-dns-strong: 199 89% 60%; --proto-http-strong: 38 92% 62%;
  --proto-tls-strong: 142 69% 58%; --proto-quic-strong: 270 75% 72%;
  --proto-icmp-strong: 0 0% 60%;   --proto-other-strong: 0 0% 54%;

  --shadow-sm: 0 1px 2px 0 hsl(0 0% 0% / 0.4);
  --shadow-md: 0 4px 16px -2px hsl(0 0% 0% / 0.5);
}
```

Tailwind maps these as `hsl(var(--token))` so classes read `bg-background`, `text-foreground-muted`, `border-border`, `bg-danger`, etc.

## 1.1 Base vs `-strong`: which one to reach for
A semantic or protocol colour is legible as a **solid fill** but not as **ink on its own pale tint** — `text-proto-http` on `bg-proto-http/15` is 1.84:1 in light mode, far under the 4.5:1 `ui_rules.md` §10 requires. Every semantic and protocol token therefore comes as a pair:

| Use | Token | Example |
|---|---|---|
| Pale tinted fill behind something | **base** | `bg-danger/15`, `bg-primary/8` row tint, `bg-danger/10` error card |
| Text or icon **on** that tint | **`-strong`** | `text-danger-strong` inside a `bg-danger/15` badge |
| Solid fill with white/black text on it | **base** | `bg-danger text-danger-foreground` button |
| Solid chart mark on a surface | **`-strong`** | proportion-bar segments, sparkline stroke, switch knob |
| Text on a plain surface (no tint) | **either**; base already clears 4.5:1 for `danger`/`primary` | `text-danger` for a blocked rule name |

Rule of thumb: **if the colour is the ink, use `-strong`; if it is the paper, use the base.** `-strong` is the same hue and saturation at a different lightness, so a protocol keeps its identity between its badge and its chart segment.

Verify with `node scripts/check-contrast.mjs`, which asserts every rendered pair against §10's thresholds in both themes and exits non-zero on a regression.

## 2. Typography
- **UI sans:** `Geist`, fallback `Inter`, `system-ui, -apple-system, sans-serif`.
- **Mono** (IPs, ports, MACs, hex, packet fields): `Geist Mono`, fallback `ui-monospace, "SF Mono", Menlo, monospace`. **All network/technical values render in mono.**

| Token | Size / line-height | Weight | Use |
|---|---|---|---|
| `text-display` | 30 / 36 | 600 | page hero numbers (overview stats) |
| `text-h1` | 20 / 28 | 600 | page titles |
| `text-h2` | 16 / 24 | 600 | section headers |
| `text-body` | 14 / 20 | 400 | default UI text |
| `text-sm` | 13 / 18 | 400 | table cells, secondary |
| `text-xs` | 12 / 16 | 500 | badges, labels, meta |
| `text-mono` | 13 / 18 | 400 | technical values (mono family) |

Letter-spacing: slight negative (`-0.01em`) on headings, normal elsewhere.

## 3. Spacing
4px base scale (Tailwind default): `1=4px 2=8px 3=12px 4=16px 5=20px 6=24px 8=32px 10=40px 12=48px`.
- Card/panel padding: `p-4` (16px) small, `p-6` (24px) large.
- Table row height: 36px (compact density for data-dense views).
- Section gap: `gap-6`; inline control gap: `gap-2`.
- Page gutter: `px-6 py-5`.

## 4. Radius, Border, Elevation
- Radius: `--radius` (8px) default; `rounded-md` controls, `rounded-lg` cards, `rounded-full` badges/avatars.
- Borders: **1px hairline** `border-border` everywhere; `border-strong` for emphasis. Cards use border **or** `shadow-sm`, not both heavily (flat, Geist-like).
- Focus: 2px `ring` offset 2px on all interactive elements.

## 5. Motion
- Durations: `fast 120ms`, `base 180ms`, `slow 240ms`; easing `cubic-bezier(0.16,1,0.3,1)` (ease-out) for enters.
- Only animate `opacity` / `transform`. Respect `prefers-reduced-motion`. Live-updating table rows must **not** animate on every tick (performance).

## 6. Density & Z-index
- Two densities: default (36px rows) and compact (28px) toggle for power users.
- Z-index scale: base `0`, sticky header `10`, dropdown `20`, drawer/inspector `30`, modal `40`, toast `50`.

## 7. Scrollbars
Scrollbars are **chrome, not content**, so they get their own neutral pair rather than reusing `border-strong` (too dark on near-black: 1.6:1) or `foreground-subtle` (reads as text weight). Rendering behaviour is `ui_rules.md` §11.

```css
:root {
  --scrollbar-size:        12px;      /* track width = hit target */
  --scrollbar-thumb-inset: 3px;       /* → a 6px visible pill inside a 12px target */
  --scrollbar-thumb:       0 0% 52%;
  --scrollbar-thumb-hover: 0 0% 35%;
}
.dark {
  --scrollbar-thumb:       0 0% 45%;
  --scrollbar-thumb-hover: 0 0% 60%;
}
```

- **Off the 4px scale on purpose.** 12px is the smallest bar that stays comfortably grabbable with a mouse while reading as a hairline; it is a pointer-ergonomics figure, not layout spacing.
- The thumb is a **non-text UI mark over a transparent track**, so it must hold **3:1 on `background`, `surface` *and* `surface-muted`** — it crosses all three. Worst case is 3.40:1 (light) / 3.68:1 (dark), enforced by `scripts/check-contrast.mjs`.
- Hover always moves **toward** more contrast: darker in light mode, lighter in dark.
- `pr-table-gutter` (`calc(0.75rem + var(--scrollbar-size))`) is the table-header padding that keeps a non-scrolling header aligned with its scrolling body.
