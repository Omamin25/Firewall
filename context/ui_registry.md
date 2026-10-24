# UI Registry — Cognton Firewall

> **Living document.** Every reusable UI component **must** be logged here the moment it's created, and updated when its API changes. Before building a new component, **search this file first** — reuse or extend an existing one instead of duplicating. A component that isn't here doesn't exist for reuse purposes.

## How to log a component
Add a row with: **Name** (PascalCase), **File** (`src/components/...`, kebab-case), **Props** (key props / variants), **Status** (`planned` / `built` / `stable`), **Notes**. Keep entries one line where possible; link to `ui_tokens.md` / `ui_rules.md` for behavior.

---

## Primitives (`src/components/ui/`)
| Component | File | Props / Variants | Status | Notes |
|---|---|---|---|---|
| Button | `ui/button.tsx` | `variant: primary\|accent\|outline\|ghost\|destructive`, `size: sm\|md`, `icon` | built | cva-based; see `ui_rules.md` §3 |
| Input | `ui/input.tsx` | `invalid`, `mono`, standard input attrs | built | mono variant for IP/port/MAC |
| Select | `ui/select.tsx` | `options: {value,label}[]`, standard select attrs | built | native element; matches Input styling |
| Badge | `ui/badge.tsx` | `variant: success\|danger\|warning\|neutral`, `title?` | built | protocol colors live in ProtocolBadge |
| Card | `ui/card.tsx` | `title?`, `action?`, children | built | surface + hairline border |
| Drawer | `ui/drawer.tsx` | `open`, `onClose`, `title`, `footer?` | built | right-side, `w-drawer`, z-drawer, Esc closes + focus return |
| Modal | `ui/modal.tsx` | `open`, `onClose`, `title`, `description?`, `footer?`, `dismissible?` | built | scrim, focus trap + return, Esc; `dismissible={false}` for confirms (`ui_rules.md` §8) |
| ConfirmDialog | `ui/confirm-dialog.tsx` | `title`, `detail`, `confirmLabel`, `onConfirm`, `onCancel`, `busy?` | built | destructive confirmation on top of Modal; Cancel is the only way out |
| Toast / Toaster | `ui/toast.tsx` | driven by `store/use-toast-store` (`toastSuccess`/`toastError`) | built | bottom-right, z-toast, 4s auto-dismiss, icon + text (never colour alone) |
| Menu | `ui/menu.tsx` | `open`, `onClose`, `anchor: {x,y}`, `items: MenuEntry[]`, `label`, `elevated?` | built | row hover + right-click menu; arrows move, Esc/outside click closes; `elevated` lifts it above a drawer |
| Tooltip | `ui/tooltip.tsx` | `content`, `side` | planned | icon-only controls currently use IconButton's `label` (aria-label + native title) |
| Skeleton | `ui/skeleton.tsx` | `rows?`, `variant: rows\|block`; `TableSkeleton` | built | `motion-safe` pulse; TableSkeleton frames a table body |
| EmptyState | `ui/empty-state.tsx` | `icon`, `title`, `description`, `action?` | built | see `ui_rules.md` §9 |
| ErrorState | `ui/error-state.tsx` | `title`, `detail`, `action?` | built | danger-accented card |
| Switch / Toggle | `ui/switch.tsx` | `checked`, `onChange`, `label`, `disabled?` | built | `role="switch"` + `aria-checked`; rule enable/disable |
| IconButton | `ui/icon-button.tsx` | `icon`, `label`(a11y + title), `variant: ghost\|outline\|destructive`, `size` | built | ghost by default; ≥32px hit target |
| KeyValueRow | `ui/key-value-row.tsx` | `label`, `value`, `mono?` | built | inspector field rows |

## Data display (`src/components/data/`)
| Component | File | Props / Variants | Status | Notes |
|---|---|---|---|---|
| DataTable | `data/data-table.tsx` | generic `columns`, `data`, `virtualized`, `onRowSelect`, `rowActions` | planned | the three data tables share the `COLUMNS` grid + TanStack Virtual convention instead; a generic wrapper is only worth it if a fourth appears |
| ProtocolBadge | `data/protocol-badge.tsx` | `protocol`, `token: ProtocolToken` | built | classes in `lib/protocol-classes.ts`; token chosen in Rust |
| DirectionIcon | `data/direction-icon.tsx` | `direction: in\|out` | built | arrow + text label (never color-only) |
| BytesCell | `data/bytes-cell.tsx` | `bytes` | planned | superseded for now by `formatBytes` + `title` |
| TimeCell | `data/time-cell.tsx` | `timestamp` | planned | `formatTime` used inline in the live table |
| Sparkline | `data/sparkline.tsx` | `series`, `color?`, `label`, `className` | built | Recharts area, no axes/tooltip/animation; colour is a `hsl(var(--token))` string |
| StatTile | `data/stat-tile.tsx` | `label`, `value`, `delta?`, `accent?`, `icon?`, `hint?` | built | overview KPIs; `text-display` mono value |

## Layout & shell (`src/components/layout/`)
| Component | File | Props / Variants | Status | Notes |
|---|---|---|---|---|
| AppShell | `layout/app-shell.tsx` | `children`, `title`, `dark`, `status`, `enforcement` | built | sidebar + topbar + content |
| Sidebar | `layout/sidebar.tsx` | `items`, `collapsed` | planned | nav |
| TopBar | `layout/top-bar.tsx` | search, theme toggle, status | planned | |
| EngineStatusPill | `layout/engine-status-pill.tsx` | `status: AppStatus\|null`, `enforcement: EnforcementStatus\|null` | built | foundation health, elevation, and enforcement state (`ui_rules.md` §1); tone always paired with an icon |
| ThemeToggle | `layout/theme-toggle.tsx` | `dark`, `onToggle` | built | light/dark toggle |

## Feature components (`src/features/`)
| Component | File | Props / Variants | Status | Notes |
|---|---|---|---|---|
| LiveTrafficTable | `features/traffic/live-traffic-table.tsx` | `packets`, `filtered` | built | virtualized, 36px rows, Process column, row select → inspector; roving tabindex + ↑/↓/Home/End row navigation |
| CaptureToolbar | `features/traffic/capture-toolbar.tsx` | `onStart`, `onStop` | built | start/stop/pause + throughput, connection count & dropped stats |
| ConnectionsTable | `features/traffic/connections-table.tsx` | `flows`, `loading`, `filtered`, `onRowMenu` | built | virtualized aggregated flows: state, dir, remote, protocol, process, user, matched rule, bytes, duration; blocked rows tinted `danger/8` with a Blocked badge (`ui_rules.md` §5); hover/right-click row menu |
| FilterBar | `features/traffic/filter-bar.tsx` | `filters`, `onChange`, `onReset`, `protocols`, `matching`, `total` | built | process/user/remote/port/protocol/direction; shared by Live + Connections |
| FlowHistoryStrip | `features/traffic/flow-history-strip.tsx` | `history: FlowHistorySummary\|null` | built | stored flow count, retention policy, oldest/newest — makes pruning observable |
| PacketInspector | `features/inspector/packet-inspector.tsx` | — (reads store) | built | drawer, L2–L7 + attribution (process/PID/path/user/SID) + pinned "create rule from this packet"; hex view pending |
| LayerSection | `features/inspector/layer-section.tsx` | `title`, children | built | titled layer group in the inspector |
| ApplicationLayerView | `features/inspector/application-layer-view.tsx` | `application: ApplicationLayer` | built | one row set per L7 classifier |
| RuleForm | `features/rules/rule-form.tsx` | `form: RuleFormController`, `formId`, `serverError`, `onSubmit` | built | validates on blur + submit; every field labelled and `aria-describedby` its note |
| RuleDialog | `features/rules/rule-dialog.tsx` | `rule: Rule\|null`, `initial?`, `description?`, `onClose` | built | create/edit in a Modal; mounted only while open so prefill always applies |
| RulesTable | `features/rules/rules-table.tsx` | `rules`, `loading`, `onEdit`, `onDelete` | built | virtualized, evaluation order, enable Switch, criteria summary, live hit counts |
| EnforcementSummary | `features/rules/enforcement-summary.tsx` | `enforcement: EnforcementStatus\|null` | built | engine state badge + enabled/blocking/dropped counters |
| CreateRuleFromTraffic | `features/rules/create-rule-from-traffic.tsx` | `anchor`, `subject`, `scopes`, `loadDraft`, `extraItems?` | built | scope menu → Rust draft → prefilled RuleDialog; nothing is written until confirmed |
| CreateRuleFromFlow | `features/rules/create-rule-from-flow.tsx` | `flow`, `anchor`, `onClose` | built | endpoint/host/port/process scopes + copy actions |
| CreateRuleFromPacket | `features/rules/create-rule-from-packet.tsx` | `packet`, `anchor`, `onClose` | built | inspector footer action; Rust maps the packet back to its flow |
| OverviewStats | `features/overview/overview-stats.tsx` | `stats`, `counts`, `throughput`, `enforcement` | built | four StatTiles: throughput, active, blocked, captured |
| ThroughputCard | `features/overview/throughput-card.tsx` | `series`, `capturing` | built | Sparkline + average/peak/window; empty state before the first samples |
| TopTalkers | `features/overview/top-talkers.tsx` | `byProcess`, `byHost` | built | two ranked lists with proportion bars |
| ProtocolBreakdown | `features/overview/protocol-breakdown.tsx` | `slices` | built | stacked proportion bar + labelled legend (`--proto-*`) |
| SettingsPage | `routes/settings-page.tsx` | — | built | history retention settings, manual pruning, system health & resilience diagnostics; uses `Input` + `Skeleton`, no bespoke form styling |

## Hooks (`src/features/`)
| Hook | File | Purpose | Status |
|---|---|---|---|
| useCaptureController | `features/traffic/use-capture-controller.ts` | Registers the stream channel once (fanning batches to the traffic + flow stores), seeds the flow table from `list_flows`, exposes start/stop/refreshInterfaces | built |
| useFilteredFlows | `features/traffic/use-filtered-flows.ts` | Filtered + sorted Connections rows, row totals, and the protocol option list | built |
| useFilteredPackets | `features/traffic/use-filtered-packets.ts` | Same, for the Live Traffic table | built |
| useRuleForm | `features/rules/use-rule-form.ts` | Rule form values, touched tracking, visible errors, validity and the submittable draft | built |

## Pure logic (`src/features/`, `src/lib/`)
| Module | File | Purpose | Status |
|---|---|---|---|
| traffic-filters | `features/traffic/traffic-filters.ts` | `TrafficFilters` shape + `matchesFilters` predicate shared by packets and flows (unit-tested) | built |
| rule-criteria | `features/rules/rule-criteria.ts` | Rule form ⇄ draft conversion, validation mirroring `ipc/dto/rule.rs`, criteria summary (unit-tested) | built |
| overview-metrics | `features/overview/overview-metrics.ts` | Connection counts, top talkers, protocol shares derived from the flow store (unit-tested) | built |
| throughput-series | `features/overview/throughput-series.ts` | 1 Hz throughput sampling + window summaries (unit-tested) | built |
| formatters | `lib/formatters.ts` | bytes/rate/time/duration/relative-time cell formatting (unit-tested) | built |
| protocol-classes | `lib/protocol-classes.ts` | `--proto-*` badge and fill class maps, so one protocol keeps one colour everywhere | built |
| clipboard | `lib/clipboard.ts` | `copyToClipboard` with success/failure toast | built |
| overlay-stack | `lib/overlay-stack.ts` | Tracks stacked dismissible overlays so Esc closes only the topmost one (Modal/Drawer/Menu) | built |

## Global CSS (`src/styles/index.css`)
Not components, but shared classes and global chrome every view inherits — check here before hand-rolling equivalent styles.
| Class / rule | Purpose | Status |
|---|---|---|
| `::-webkit-scrollbar-*` (global) | Tokenized scrollbars for every scroll container: transparent track, inset `--radius` pill thumb, no steppers (`ui_rules.md` §11). No per-component opt-in needed | built |
| `color-scheme` on `:root` / `.dark` | Themes the chrome the engine draws itself — native `<select>` popups and their scrollbars | built |
| `.scrollbar-gutter-stable` | Reserves the scrollbar's width on a table body so its columns don't shift when rows start overflowing; pairs with `pr-table-gutter` on the sticky header | built |
| `.virt-row`, `.virt-row-selected`, `.virt-row-blocked` | Opaque row background + 8% tints for virtualized rows, so a fast-prepending table repaints cleanly (`ui_rules.md` §5) | built |

*(Statuses update as phases in `build_plan.md` complete. New components — add a row before merging.)*
