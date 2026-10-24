# Build Plan — Cognton Firewall

Six logical phases, sequenced so each builds on a working foundation. Each phase ends in a **verifiable, runnable** state. Update `progress_tracker.md` as items complete.

> **Prerequisite (before Phase 1):** Install **Npcap** (with "WinPcap API-compatible" mode) and the **WinDivert 2.x** driver/DLL. Confirm Rust toolchain (MSVC), Node 20+, and that the dev shell runs **as Administrator**.

---

## Phase 1 — Foundation, Scaffolding & DB
Goal: an elevated Tauri app boots, shows an empty dashboard shell, and can read/write the local DB.
- Scaffold Tauri v2 app (`create-tauri-app`, React + TS + Vite template).
- Configure Tailwind + tokens (`ui_tokens.md`), base layout, light/dark theme, routing (Overview / Live / Connections / Rules).
- Set app manifest to `requireAdministrator`; add "must run as admin" guard + UI state.
- Add `rusqlite`, create `store/` with migrations for `rules`, `flows`, `settings`. Verify CRUD.
- Establish IPC skeleton: one demo command + one `Channel` stream emitting mock packets so the UI pipeline (Zustand ← Channel) is proven end-to-end.
- Set up `tracing` logging to file; error-to-UI plumbing.
- **Exit criteria:** app launches elevated, shows the shell + theme, mock stream renders in a virtualized table, DB migrations run clean.

## Phase 2 — Capture & L2–L7 Decode
Goal: real packets flowing, correctly decoded across all layers, streamed to the UI.
- `capture/`: enumerate interfaces via Npcap; open promiscuous capture on selected/all interfaces in a background thread; feed a bounded channel.
- `parse/`: decode L2 (MAC, EtherType), L3 (IPv4/IPv6/ARP), L4 (TCP/UDP/ICMP incl. flags/seq) with `etherparse`.
- L7 classifiers: **DNS, HTTP, TLS (SNI/cert), QUIC, DHCP, mDNS**; opaque fallback for the rest.
- Define `model/` domain structs + `ipc/` DTOs (`serde`, camelCase); wire real packets through the batched `Channel` (replace mock).
- Frontend: Live Traffic table (virtualized) + Packet Inspector panel showing the full L2→L7 breakdown.
- **Exit criteria:** browsing the web produces correctly-decoded, layer-by-layer packets in the UI within ~1s; DNS names and TLS SNI appear.

## Phase 3 — Flow Aggregation & OS Attribution
Goal: packets are grouped into connections and attributed to the exact process **and** Windows user.
- `flow/`: 5-tuple flow keys, aggregation, byte/packet counters, duration, direction inference; ring buffer + Connections view.
- `attribution/`: `GetExtendedTcpTable`/`GetExtendedUdpTable` → PID; PID → process name/full path; PID → token → SID → `domain\user` (via `windows` crate). TTL cache to avoid per-packet syscalls.
- Persist aggregated flows to `flows` table with retention/pruning.
- Frontend: Connections table (process, user, remote host, protocol, bytes, duration); enrich Live view + Inspector with process/user; add filters/search (process, user, remote IP/domain, port, protocol, direction).
- **Exit criteria:** every active connection shows correct process **and** user account; filters isolate flows correctly; flow history persists and prunes.

## Phase 4 — Enforcement Engine (the actual firewall)
Goal: rules block/allow real traffic at the packet level, immediately.
- `enforcement/`: integrate WinDivert (network layer) in a background thread — receive, evaluate against active rules, **drop** matches / re-inject the rest; consume SOCKET-layer events for authoritative real-time PID→flow mapping (feeds attribution).
- `rules/`: rule model + matching engine (direction, protocol, remote/local IP+CIDR, ports, process path, user SID; priority ordering; allow overrides block by priority). Load enabled rules from DB into a hot in-memory set; hot-reload on change.
- IPC commands: create/update/toggle/delete rule; "create rule from flow". Increment `hit_count`, mark blocked flows.
- **Exit criteria:** creating a block rule for a live flow stalls/fails that traffic within ~1s; toggling it off restores traffic; blocked flows are visibly marked and counted.

## Phase 5 — Dashboard, Rules UX & Overview
Goal: the clean, professional dashboard is complete and pleasant to use.
- Overview page: live throughput sparkline, top talkers (by process/host), active connection count, blocked count, protocol breakdown.
- Rules manager: full CRUD UI, enable/disable toggles, priority, hit counts, empty/loading/error states.
- Create-rule-from-flow flow (right-click / action button on a connection → prefilled rule form).
- Polish: light/dark contrast pass, keyboard navigation, virtualization tuning, consistent components logged in `ui_registry.md`, all values from `ui_tokens.md`.
- **Exit criteria:** all views feature-complete, consistent, responsive under load; no hard-coded visual values; matches the Vercel/Geist minimal aesthetic.

## Phase 6 — Hardening, Performance & Packaging
Goal: robust, performant, installable.
- Performance: sustain real-world traffic without UI jank — verify batching cadence, ring-buffer caps, table virtualization, attribution cache hit rate; back-pressure handling on the capture channel.
- Resilience: graceful handling of missing Npcap/WinDivert, non-admin launch, interface changes, driver reload; structured error states in UI.
- Security review: validate all command inputs, confirm capability/permission allow-list is minimal, no path/rule injection.
- Persistence hygiene: retention pruning verified; settings persisted.
- Packaging: Tauri build → signed (if cert available) NSIS/MSI installer that bundles/prompts for Npcap + WinDivert; first-run checks; README + run-as-admin instructions.
- **Exit criteria:** installable build runs on a clean Win10/11 machine, passes the success criteria in `project_overview.md`, and survives a sustained-traffic soak test.

---

### Sequencing rationale
Visibility (P1–P3) must be correct before enforcement (P4) — you can't sanely block what you can't attribute. Attribution (P3) precedes enforcement because rules match on process/user. UX polish (P5) comes after behavior is proven. Hardening/packaging (P6) is last so it targets the real, complete surface.
