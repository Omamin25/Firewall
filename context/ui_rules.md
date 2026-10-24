# UI Rules — Cognton Firewall

Visual + interaction behavior for global elements. All values reference `ui_tokens.md`. Guiding principle: **Vercel/Geist minimalism** — flat surfaces, hairline borders, monochrome base, one accent, generous whitespace, data-dense but calm. The product is a serious security tool: **clarity and trust over decoration.**

## 1. Global Layout
- **App chrome:** left sidebar nav (icon + label; collapsible to icons) + top bar (title, global search, theme toggle, engine-status indicator) + main content area.
- **Engine status pill** (top bar, always visible): green `success` = capturing + enforcing; amber `warning` = degraded (e.g. capture only); red `danger` = engine down / not admin. Click → details popover.
- Content max-width is fluid (data tables use full width); page gutter `px-6 py-5`.

## 2. Cards / Panels
- `bg-surface`, `border border-border`, `rounded-lg`, `shadow-sm`. Header: `text-h2` + optional action on the right. Body padding `p-4`/`p-6`.
- No nested heavy shadows. Dark mode: rely on `surface` vs `background` contrast + hairline border, not glow.

## 3. Buttons
Variants via `cva`:
- **Primary** (`solid`): `bg-secondary text-secondary-foreground` (Geist black-on-light / white-on-dark) — the main action per view. Hover: 90% opacity; active: 80%.
- **Accent/CTA:** `bg-primary text-primary-foreground` — reserved for the single most important action (e.g. "Add Rule").
- **Secondary/Outline:** `border border-border-strong bg-transparent text-foreground` hover `bg-surface-muted`.
- **Ghost:** transparent, hover `bg-surface-muted` — toolbar/icon actions.
- **Destructive:** `bg-danger text-danger-foreground` — block/delete; **always requires confirmation** for irreversible actions.
- All: `rounded-md`, `text-sm font-medium`, height 32px (sm) / 36px (default), focus ring, disabled = 50% opacity + `cursor-not-allowed`. Icon+label spacing `gap-2`. Show a spinner + disable during async.

## 4. Inputs / Forms
- `bg-background border border-border rounded-md`, height 36px, `text-sm`, `px-3`. Focus: `border-strong` + `ring`. Placeholder `foreground-subtle`.
- Labels `text-xs font-medium text-foreground-muted` above the field. Technical inputs (IP, port, CIDR) use **mono** font.
- **Validation:** inline, below field, `text-xs text-danger`; invalid field border `danger`. Validate on blur + submit, never block typing. Disable submit until valid.
- Selects/comboboxes match input styling; dropdown panel = card styling, z-20.

## 5. Tables (the core surface)
- **Virtualized** (TanStack Virtual) for packet/flow/rule lists. Row height 36px (28px compact). Sticky header `bg-surface border-b border-border`, `text-xs uppercase tracking-wide text-foreground-muted`.
- Zebra optional via `surface-muted`; hover row `bg-surface-muted`; selected row `bg-primary/8` + left `2px` `primary` accent bar.
- **Cell conventions:** IPs/ports/MACs/hex in **mono**; timestamps relative ("2s ago") with absolute on hover; bytes human-readable (KB/MB) with raw on hover; direction shown as ↑ out / ↓ in icon.
- **Live updates:** new rows append/prepend without layout shift; **no per-row animation** under load. A subtle "live" indicator + pause button lets the user freeze the stream to inspect.
- Row actions on hover (right-aligned): inspect, "create rule from this", copy. Right-click context menu mirrors these.
- **Blocked flows:** row tinted `danger/8`, a `danger` "Blocked" badge, and the matched rule name.

## 6. Badges
- `rounded-full px-2 py-0.5 text-xs font-medium`. Semantic fills at low opacity + solid text:
  - Allowed/OK → `success`; Blocked → `danger`; Warning → `warning`.
  - **Protocol badges** use the `--proto-*` tokens (DNS/HTTP/TLS/QUIC/ICMP/Other) — consistent color per protocol everywhere (table, inspector, charts).
  - Direction badges: In / Out, neutral with arrow icon.
- Keep badges monochrome-leaning; never more than 2 accent colors visible in one row.

## 7. Packet Inspector (drawer)
- Right-side **drawer** (z-30), width ~480px, slides in (`transform`, `base` duration). Overlay scrim on small windows only.
- Content = **collapsible layer sections L2 → L7**, each a titled group (MAC/EtherType; IP/TTL/flags; TCP/UDP ports/flags/seq; L7 detail) plus an **Attribution** section (process path, PID, user account) and a raw **hex/bytes** view at the bottom.
- Field rows: `label (muted, left) : value (mono, right)`, copyable. A "Create rule from this packet" primary action pinned at the drawer footer.

## 8. Modals / Dialogs
- Centered, `bg-surface`, `rounded-lg`, `shadow-md`, max-width 480–560px, scrim `bg-black/50`, z-40. Title `text-h2`, body `text-body`, actions bottom-right (secondary + primary). Escape + scrim click close (except mid-destructive-confirm). Focus-trap + return focus.
- Use modals for **create/edit rule** and **destructive confirmations** only. Prefer inline/drawer elsewhere.

## 9. Feedback States (mandatory in every view)
- **Loading:** skeleton rows/cards (`surface-muted` shimmer, reduced-motion aware) — never a bare spinner for full pages.
- **Empty:** centered icon + one-line explanation + primary action (e.g. "No traffic yet — waiting for packets").
- **Error:** inline card with `danger` accent, plain-language cause + recovery action (e.g. "Npcap not found — install it and relaunch as Administrator").
- **Toasts** (z-50, bottom-right): transient confirmations (rule created/blocked), auto-dismiss, `success`/`danger` variants. Never use toasts for critical errors that need action.

## 10. Dark Mode & Accessibility
- Dark is a first-class mode (`class` strategy), default follows OS. Verify contrast in **both** modes: body text ≥ 4.5:1, large/UI ≥ 3:1.
- **Never encode meaning by color alone** — pair status color with an icon/label (colorblind users). Protocol legend always labeled.
- Full keyboard nav: visible focus ring on every interactive element, logical tab order, table row keyboard selection, `Esc` closes drawers/modals. Respect `prefers-reduced-motion`.
- Hit targets ≥ 32px. Tooltips for icon-only controls.

## 11. Scrollbars
The platform default — light track, stepper arrows, square corners, full-bleed to the border — is the one piece of chrome the app doesn't otherwise draw, and it reads as unfinished against a dark Geist surface. Every scroll container in the app is styled globally in `src/styles/index.css` (values from `ui_tokens.md` §7):

- **Track transparent, no steppers.** The thumb floats over the content it scrolls; a painted track reads as an extra vertical border beside the table's own. `::-webkit-scrollbar-button { display: none }` removes the arrows.
- **Thumb = a `--radius` pill inset `--scrollbar-thumb-inset` from a `--scrollbar-size` track.** The inset is a transparent border with `background-clip: padding-box`, so the *hit target* stays the full 12px while the *paint* is 6px — thin look, comfortable grab.
- **Use `::-webkit-scrollbar-*`, not `scrollbar-width`/`scrollbar-color`.** In Chromium (and therefore WebView2) setting either standard property makes the engine ignore the pseudo-elements entirely and fall back to its own thin bar — you cannot mix them, and only the pseudo-elements can drop the steppers and inset the thumb.
- **`color-scheme` is set per theme** on `:root`/`.dark` so chrome the engine draws itself — native `<select>` popups and their scrollbars — follows the theme instead of flashing a light panel.
- **Tables reserve the gutter.** Scroll bodies take `.scrollbar-gutter-stable` and their sticky headers take `pr-table-gutter`. A header doesn't scroll, so it never loses width to the bar the way its body does; without both, right-aligned columns (Bytes, Duration) drift by the bar's width, and a table that grows past its height shifts every column mid-stream.
