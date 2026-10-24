# Code Standards — Cognton Firewall

Applies to the whole repo. These are **rules, not suggestions**. When a request conflicts with them, stop and flag it (see `agents.md`).

## 1. General
- **No hard-coded visual values, ever.** Colors, spacing, radii, typography come from `ui_tokens.md` / Tailwind theme. A raw hex, `px`, or magic color in a component is a bug.
- **No secrets, no network calls in the frontend.** All OS/privileged work lives in Rust behind Tauri commands.
- **Small, single-purpose modules/files.** If a file exceeds ~300 lines, split it.
- **Every new reusable UI component is logged in `ui_registry.md`; every completed feature updates `progress_tracker.md`.**
- Comments explain *why*, not *what*. Match the density of surrounding code.

## 2. Rust (core)
- **Edition 2021**, `rustfmt` (default config) and `clippy` (deny warnings in CI). No `#[allow(...)]` without a one-line justification.
- **Error handling:** libraries/modules return `Result<T, E>` with typed errors via **`thiserror`**; the app boundary (Tauri commands, `main`) uses **`anyhow`** for context. **Never `unwrap()`/`expect()` on fallible runtime paths** — allowed only in tests and truly-infallible setup. Add `.context("...")` when propagating.
- **No `panic!` in engine threads.** A capture/enforcement thread must catch its errors, log via `tracing`, surface a UI error state, and stay alive or restart cleanly — never take down the app.
- **Concurrency:** background engines run on dedicated threads / `tokio` tasks and communicate via bounded channels (`crossbeam`/`tokio::mpsc`). Shared state is `Arc<Mutex<...>>` / `Arc<RwLock<...>>`; **hold locks for the minimum span** and never across `.await` or a syscall batch.
- **Attribution/OS syscalls are cached** (TTL) — never call `GetExtendedTcpTable` or token APIs per packet.
- **`unsafe`** only where Win32 FFI requires it, wrapped in a safe module boundary with a `// SAFETY:` comment justifying each block.
- **Naming:** `snake_case` fns/vars, `CamelCase` types/traits, `SCREAMING_SNAKE_CASE` consts. Modules are `snake_case` folders with `mod.rs` or `<name>.rs`.
- **IPC DTOs:** plain `#[derive(Serialize, Deserialize, Clone)]` structs with `#[serde(rename_all = "camelCase")]`, kept in `ipc/`. Domain types (`model/`) are separate from wire types when they differ.
- **Logging:** `tracing` spans/events with fields; no `println!` in shipped code. Levels: `error` (surfaced), `warn`, `info` (lifecycle), `debug`/`trace` (dev).

## 3. TypeScript / React (frontend)
- **`strict: true`** in `tsconfig` (plus `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`). **No `any`** — use `unknown` + narrowing. **No non-null `!`** except with a justifying comment.
- **Function components + hooks only.** No class components.
- **Props typed with explicit `type` aliases** (not `interface` unless extending). Export the props type when the component is reusable.
- **State:** local UI state via `useState`/`useReducer`; cross-view/live-stream state via **Zustand** stores in `src/store/`. No prop-drilling more than 2 levels — lift to a store instead. Keep the live packet/flow buffer in a store fed by the IPC Channel, with selectors to avoid re-render storms.
- **Performance:** high-volume tables **must** be virtualized (TanStack Virtual). Memoize row renderers; use stable keys (flow id / packet id). Never map thousands of DOM nodes unvirtualized.
- **IPC access is centralized** in `src/lib/ipc.ts` — components never call `invoke`/`Channel` directly; they call typed wrapper functions. This keeps command names and payload types in one place, mirroring Rust.
- **Data fetching / commands** return typed results; handle `loading` / `empty` / `error` explicitly in every view (see `ui_rules.md`).
- **Naming & files:**
  - Files: **kebab-case** (`live-traffic-table.tsx`, `use-flow-store.ts`).
  - Components: **PascalCase** (`LiveTrafficTable`).
  - Hooks: **camelCase** starting `use` (`useFlowStore`).
  - Constants: `SCREAMING_SNAKE_CASE`.
- **Styling:** Tailwind utilities driven by theme tokens. Compose variants with **`cva`** (class-variance-authority) + a `cn()` helper (clsx + tailwind-merge). No inline `style={{}}` for anything themable.
- **Imports:** absolute via `@/` alias to `src/`. Group: external → `@/` internal → relative. No unused imports (lint-enforced).

## 4. Types Sync Contract (Rust ⇄ TS)
- Every IPC command and event payload has a Rust DTO **and** a matching TS type in `src/types/`. They must stay identical in shape (camelCase). Treat drift as a build-breaking bug.
- Prefer discriminated unions for layered/variant data (e.g. L7 protocol detail): Rust enum with `#[serde(tag = "type")]` ⇄ TS `type` union on a `type` field.

## 5. Testing
- **Rust:** unit tests for parsers (decode known packet byte fixtures for each L2–L7 case), the rule-matching engine (table-driven: rule × packet → block/allow), and flow aggregation. Put fixtures under `src-tauri/tests/fixtures/`.
- **TS:** unit-test pure logic (formatters, filter predicates, store reducers) with Vitest. Component tests for stateful widgets (rule form validation, filter bar).
- A feature is **not done** until its parser/engine logic has tests and its UI handles loading/empty/error.

## 6. Git / Commits
- Conventional Commits (`feat:`, `fix:`, `refactor:`, `perf:`, `docs:`, `test:`, `chore:`). One logical change per commit. Reference the phase where useful (`feat(capture): ...`).
