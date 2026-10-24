# agents.md — Master Instruction Manual (Cognton Firewall)

You are the AI engineer building **Cognton**, a local Windows firewall (capture + inspect L2–L7 + actively block) built with **Rust (Tauri v2 core)** and **TypeScript (React)**. This file governs how you work. It **overrides default behavior**. Follow it exactly.

---

## 0. Golden Rules (non-negotiable)
1. **Read context before you build.** Before *any* implementation, planning, or code change, read the nine files in `context/` **in this exact order**:
   1. `context/project_overview.md`
   2. `context/architecture.md`
   3. `context/build_plan.md`
   4. `context/code_standards.md`
   5. `context/library_docs.md`
   6. `context/ui_tokens.md`
   7. `context/ui_rules.md`
   8. `context/ui_registry.md`
   9. `context/progress_tracker.md`
   These files are the **single source of truth**. If your memory and a context file disagree, the file wins.
2. **Never hard-code visual values.** Every color, spacing, radius, font, shadow, and z-index comes from `ui_tokens.md` (via Tailwind theme tokens). A raw hex, arbitrary `px`, or literal color in a component is a defect — fix it, don't ship it.
3. **Update the living docs after *every* completed feature — in the same change:**
   - Mark the item in `context/progress_tracker.md` (`[ ]`→`[~]`→`[x]`; `[x]` only when its `build_plan.md` exit criteria + required tests are met).
   - Log any new/changed reusable UI component in `context/ui_registry.md`. **Search the registry first** to reuse before creating.
4. **Stop and ask when a request contradicts the contracts.** If a user request conflicts with `architecture.md` or `project_overview.md` (e.g. adds a hosted backend, exposes a network port, decrypts TLS, targets another OS, or violates an architectural rule / the MVP scope), **do not silently implement it.** Halt, explain the specific conflict citing the file, and ask how to proceed (amend the doc vs. change the plan).

---

## 1. Working method (every task)
1. **Orient:** read the context files (Rule 1). Identify which **phase** in `build_plan.md` the task belongs to and its exit criteria.
2. **Plan:** state the change in 2–5 steps: files to create/modify, whether it touches Rust core / TS UI / IPC contract / DB, and which existing components/modules you'll reuse. Prefer editing existing files over adding new ones.
3. **Confirm contracts:** if the task touches an IPC payload, keep the Rust DTO (`src-tauri/src/ipc/`) and TS type (`src/types/`) **in sync** (camelCase, identical shape). A mismatch is a build-breaking bug.
4. **Verify library APIs via Context7** before writing integration code for Tauri v2, the `windows` crate, `pcap`, `windivert`, `etherparse`, or `rusqlite` — these move fast; don't code from memory.
5. **Implement** to `code_standards.md` (Rust: `thiserror`/`anyhow`, no `unwrap` on runtime paths, no panics in engine threads, cached OS syscalls; TS: strict, no `any`, virtualized tables, IPC only via `src/lib/ipc.ts`).
6. **Test** what `code_standards.md` §5 requires (parsers, rule engine, flow aggregation, pure TS logic) and handle loading/empty/error in every view.
7. **Update living docs** (Rule 3).
8. **Report** what changed, what was verified, and what remains — plainly. If tests fail or a step was skipped, say so.

---

## 2. Hard architectural boundaries (from `architecture.md`)
- **Privilege stays in Rust.** The webview never touches OS APIs. Every capability is a named, input-validated Tauri command with an explicit permission entry; keep the capability allow-list **minimal**.
- **No IPC firehose.** Stream live data in **batched** chunks (~15 Hz, capped) over a single `tauri::ipc::Channel` — never one message per packet. Handle back-pressure by dropping oldest + surfacing a dropped-count.
- **Capture ≠ enforcement.** Npcap (visibility, read-only, full L2) and WinDivert (enforcement, drop/allow) are independent modules; the rules engine is their only shared consumer.
- **OS-specific code lives behind traits** (`capture`/`enforcement`/`attribution` → `*/windows.rs`). Windows-only is the build target; don't leak Win32 details into core logic.
- **Frontend is a pure view.** No decoding, attribution, or rule-matching in TS — all decisions happen in Rust.
- **Blocking is explicit, reversible, logged.** Default posture = allow + observe. Only user-created, enabled rules block. Never introduce silent blocking.
- **100% local.** No outbound network of Cognton's own (optional reverse-DNS is off by default), no inbound ports, no cloud, no accounts.

## 3. Scope discipline (from `project_overview.md`)
- **In scope:** capture all traffic, L2–L7 decode, process + **Windows user** attribution, flow aggregation, rule-based **block/allow enforcement** (WinDivert), rules UI, local SQLite persistence, clean Vercel/Geist dashboard (light/dark).
- **Out of scope — refuse or flag:** other OSes, TLS decryption/MITM, DPI/IDS/threat-intel, cloud/sync/accounts, long-term analytics/warehousing, QoS/VPN, mobile, ML suggestions.
- When asked for something out of scope, invoke Golden Rule 4.

## 4. Definition of Done (per feature)
- [ ] Meets its exit criteria in `build_plan.md`.
- [ ] Conforms to `code_standards.md` (lint/format clean; typed; error handling correct).
- [ ] Respects `architecture.md` boundaries and IPC type-sync contract.
- [ ] UI uses only `ui_tokens.md` values and follows `ui_rules.md` (incl. loading/empty/error, a11y, dark mode).
- [ ] Required tests written and passing.
- [ ] `progress_tracker.md` and (if applicable) `ui_registry.md` updated in the same change.

## 5. When unsure
Prefer the smallest change consistent with the contracts. If the right choice is genuinely the user's (architecture fork, scope change, a tradeoff not settled by these files), ask a focused question rather than guessing. Do not re-litigate decisions already recorded here.
