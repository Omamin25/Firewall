# Architecture — Cognton Firewall

## 1. Technology Stack
| Layer | Choice | Why |
|---|---|---|
| App shell | **Tauri v2** | Bundles a Rust core + web UI into one small native Windows app. Rust owns privileged capture/enforcement; UI is a local webview. No exposed ports. |
| Core language | **Rust (2021 edition)** | Safe, fast, direct access to Npcap/WinDivert/Win32 APIs. |
| Frontend | **React 18 + TypeScript 5 (strict)** | Component model for the dashboard; strict typing end-to-end. |
| Bundler/dev | **Vite** | Tauri's default; fast HMR. |
| Styling | **Tailwind CSS v3** | Utility-first, token-driven (see `ui_tokens.md`). Vercel/Geist-inspired minimal aesthetic. |
| Client state | **Zustand** | Minimal, fast store for live streams + UI state. No Redux boilerplate. |
| Tables | **TanStack Table + TanStack Virtual** | Virtualized rendering of high-volume packet/flow tables. |
| Charts | **Recharts** (throughput/sparklines only) | Small, declarative; used sparingly per `ui_rules.md`. |
| Icons | **lucide-react** | Clean, consistent line icons. |
| Packet capture | **`pcap` crate** (Npcap on Windows) | Full L2 Ethernet frame capture, promiscuous, all interfaces. Read-only visibility path. |
| Packet parsing | **`etherparse`** (L2–L4) + custom L7 classifiers | Robust zero-copy parsing; custom code for DNS/HTTP/TLS-SNI/QUIC/DHCP/mDNS. |
| Enforcement | **`windivert` crate** (WinDivert 2.x) | Userland packet **drop/allow** at WFP layer; SOCKET layer for real-time flow+PID events. This is the "firewall" path. |
| OS attribution | **`windows` crate** (IP Helper + token APIs) | `GetExtendedTcpTable`/`GetExtendedUdpTable` → PID; PID → process path; PID → SID → `domain\user`. |
| Persistence | **`rusqlite`** (SQLite, bundled) | Local, file-based store for rules + flow history. |
| Async runtime | **`tokio`** (via Tauri) | Background capture/enforcement threads + async command handlers. |
| Serialization | **`serde` / `serde_json`** | Typed IPC payloads between Rust and TS. |
| Logging | **`tracing` + `tracing-subscriber`** | Structured logs to file. |

## 2. System Boundaries
- **Fully local. Offline. No network egress of its own** (aside from optional DNS reverse lookups, which are configurable and default off).
- **No inbound server ports.** All Rust↔UI communication is Tauri IPC (in-process), **not** HTTP/WebSocket.
- **Trust boundary:** the Rust core is privileged (Admin, kernel-adjacent via Npcap/WinDivert). The webview is untrusted UI. All privileged actions go through explicitly allow-listed Tauri **commands** with validated inputs (see `code_standards.md`).
- **Elevation:** the app manifest requests `requireAdministrator`. Without it, capture/enforcement engines fail to init and the UI shows a clear "relaunch as administrator" state.

## 3. Runtime Data Flow
```
                       ┌──────────────────────────── Rust Core (privileged) ───────────────────────────┐
 NIC(s) ──Npcap──▶ capture::source ──▶ parse::decode (L2–L7) ──▶ flow::aggregator ──▶ ring buffer
                                                                      │                     │
 WinDivert SOCKET events ──▶ attribution::resolver (PID→process→user, cached) ◀────────────┘
                                                                      │
                       rules::engine (SQLite) ◀── enforcement::windivert (drop/allow) ◀─────┘
                                                                      │
                                              ipc::stream  (batched @ ~15 Hz via tauri::ipc::Channel)
                       └───────────────────────────────────────────────┬───────────────────────────────┘
                                                                        │  events + command results
                       ┌────────────────────────────────────────────── ▼ ─ React UI (webview) ─────────┐
                        Zustand store ─▶ Live Traffic | Connections | Inspector | Rules | Overview
                       └───────────────────────────────────────────────────────────────────────────────┘
```
Two engines run in parallel:
- **Visibility engine (Npcap):** promiscuous, read-only, gives full L2 frames for rich decode/display.
- **Enforcement engine (WinDivert):** intercepts at network layer, drops packets matching active block rules and re-injects the rest; its SOCKET layer supplies authoritative real-time PID→flow mapping for attribution.

## 4. Project Folder Structure
```
cognton/
├─ agents.md                      # Master AI instruction manual (root)
├─ context/                       # The 9 context files (source of truth)
│  ├─ project_overview.md
│  ├─ architecture.md
│  ├─ build_plan.md
│  ├─ code_standards.md
│  ├─ library_docs.md
│  ├─ ui_tokens.md
│  ├─ ui_rules.md
│  ├─ ui_registry.md
│  └─ progress_tracker.md
├─ src/                           # Frontend (React + TS)
│  ├─ main.tsx
│  ├─ App.tsx
│  ├─ routes/                     # Page-level views (overview, live, connections, rules)
│  ├─ components/                 # Reusable UI (logged in ui_registry.md)
│  ├─ features/                   # Feature slices (traffic, inspector, rules)
│  ├─ store/                      # Zustand stores
│  ├─ lib/                        # ipc client, formatters, types
│  ├─ types/                      # Shared TS types (mirror Rust IPC structs)
│  └─ styles/                     # Tailwind entry + tokens
├─ src-tauri/                     # Rust core
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ build.rs
│  ├─ capabilities/               # Tauri v2 capability files
│  ├─ permissions/                # Command permission definitions
│  ├─ icons/
│  └─ src/
│     ├─ main.rs                  # Thin entry → lib::run()
│     ├─ lib.rs                   # Tauri builder, state, setup hook, handlers
│     ├─ capture/                 # Npcap source + interface mgmt
│     ├─ parse/                   # L2–L7 decoders (etherparse + L7 classifiers)
│     ├─ flow/                    # Flow keys + aggregation + ring buffer
│     ├─ attribution/             # PID/process/user resolution (windows crate)
│     ├─ enforcement/             # WinDivert drop/allow + socket events
│     ├─ rules/                   # Rule model + matching engine
│     ├─ store/                   # rusqlite (rules, flow history, migrations)
│     ├─ ipc/                     # commands + Channel stream + DTOs
│     └─ model/                   # Shared domain structs (serde)
├─ index.html
├─ package.json
├─ tsconfig.json
├─ tailwind.config.ts
└─ vite.config.ts
```

## 5. Strict Architectural Rules
1. **Privilege stays in Rust.** The webview never touches OS APIs. Every capability is a named Tauri command with validated args and an explicit permission entry in `src-tauri/permissions/`.
2. **No IPC firehose.** Never emit one IPC message per packet. The Rust core aggregates into a ring buffer and pushes **batched** updates (~15 Hz, capped batch size) over a single `tauri::ipc::Channel`. Flow/table state is diffed, not resent wholesale.
3. **OS-specific code is isolated behind traits.** `capture`, `enforcement`, and `attribution` expose platform-agnostic traits; the Windows implementation lives in `*/windows.rs`. This keeps a future port possible without touching core logic (out of scope to *build* other OSes now).
4. **Capture ≠ enforcement.** The Npcap visibility path and the WinDivert enforcement path are independent modules; they must not depend on each other's internals. The rules engine is the only shared consumer.
5. **One source of truth for types.** Rust IPC DTOs (`ipc/`) are `serde`-serialized; the matching TS types live in `src/types/` and MUST be kept in sync (a mismatch is a bug). Field naming: Rust structs use `#[serde(rename_all = "camelCase")]`.
6. **Frontend is a pure view of streamed state.** No business logic (rule matching, decoding, attribution) in TS. UI derives, filters, and displays; all decisions happen in Rust.
7. **Blocking is explicit and reversible.** No rule silently blocks. Every enforcement action is logged and every rule is toggleable/deletable from the UI. Default posture is **allow + observe**; blocking only occurs for user-created enabled rules.
8. **Errors surface, never swallow.** Engine init failures (missing Npcap, no admin, WinDivert driver load fail) propagate to the UI as actionable states, not silent no-ops.

## 6. Initial Database Schema (SQLite)
```sql
-- Firewall rules (persisted, enforced by WinDivert)
CREATE TABLE rules (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  name          TEXT    NOT NULL,
  action        TEXT    NOT NULL CHECK (action IN ('block','allow')),
  direction     TEXT    NOT NULL CHECK (direction IN ('in','out','both')),
  protocol      TEXT             CHECK (protocol IN ('tcp','udp','icmp','any')) DEFAULT 'any',
  remote_ip     TEXT,            -- CIDR or exact; NULL = any
  remote_port   INTEGER,         -- NULL = any
  local_port    INTEGER,         -- NULL = any
  process_path  TEXT,            -- NULL = any
  user_sid      TEXT,            -- NULL = any
  enabled       INTEGER NOT NULL DEFAULT 1,
  priority      INTEGER NOT NULL DEFAULT 100,  -- lower = evaluated first
  hit_count     INTEGER NOT NULL DEFAULT 0,
  created_at    TEXT    NOT NULL,              -- ISO-8601
  updated_at    TEXT    NOT NULL
);

-- Rolling flow/connection history (aggregated, not per-packet)
CREATE TABLE flows (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  first_seen    TEXT    NOT NULL,
  last_seen     TEXT    NOT NULL,
  direction     TEXT    NOT NULL,
  l7_protocol   TEXT,            -- dns/http/tls/quic/... or NULL
  local_ip      TEXT    NOT NULL,
  local_port    INTEGER,
  remote_ip     TEXT    NOT NULL,
  remote_port   INTEGER,
  remote_host   TEXT,            -- SNI / DNS name if known
  protocol      TEXT    NOT NULL,-- tcp/udp/icmp
  pid           INTEGER,
  process_path  TEXT,
  user_account  TEXT,            -- domain\user
  user_sid      TEXT,
  bytes_sent    INTEGER NOT NULL DEFAULT 0,
  bytes_recv    INTEGER NOT NULL DEFAULT 0,
  packets       INTEGER NOT NULL DEFAULT 0,
  blocked       INTEGER NOT NULL DEFAULT 0,   -- 1 if a rule dropped it
  matched_rule  INTEGER          REFERENCES rules(id)
);
CREATE INDEX idx_flows_last_seen ON flows(last_seen);
CREATE INDEX idx_flows_remote_ip ON flows(remote_ip);
CREATE INDEX idx_flows_pid ON flows(pid);

-- App settings (key/value)
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```
Retention: `flows` is capped (e.g. rolling N rows / N hours) and pruned on a timer; live per-packet data stays only in the in-memory ring buffer, never persisted.
