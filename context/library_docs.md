# Library Docs — Cognton Firewall

Implementation rules for the key libraries. **Always confirm current APIs via Context7 before writing integration code** (Tauri v2 and the Windows crate move fast). Patterns below reflect Tauri **v2**.

---

## 1. Tauri v2 (app shell + IPC)

### Commands (frontend → Rust, request/response)
Define with `#[tauri::command]`, register in `generate_handler!`, access shared state via `tauri::State`.
```rust
#[tauri::command]
async fn create_rule(state: State<'_, AppState>, rule: NewRule) -> Result<Rule, String> {
    state.rules.create(rule).await.map_err(|e| e.to_string())
}
```
- Commands return `Result<T, String>` (or a serializable error DTO). **Validate all inputs** — the webview is untrusted.
- Every command needs a **permission** entry (`src-tauri/permissions/<cmd>.toml`) and must be included in a **capability** (`src-tauri/capabilities/*.json`) granted to the main window. Keep the allow-list **minimal**.

### Managed state
```rust
tauri::Builder::default()
    .manage(AppState::new()?)              // Arc-backed engines, db handle, rule set
    .invoke_handler(tauri::generate_handler![create_rule, list_flows, toggle_rule, /* ... */])
    .setup(|app| { spawn_engines(app.handle().clone()); Ok(()) })
    .run(tauri::generate_context!())?;
```
- Wrap mutable state in `Mutex`/`RwLock` inside the managed struct. Engines hold `AppHandle` clones to emit.

### Streaming live packets/flows (Rust → frontend) — **use `ipc::Channel`, batched**
High-throughput data goes over a **single `tauri::ipc::Channel`**, not per-packet `emit`.
```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
enum StreamMsg {
    Packets(Vec<PacketDto>),   // batched, capped size
    FlowUpdate(Vec<FlowDto>),  // diffs
    Stats(StatsDto),
}

#[tauri::command]
fn subscribe_stream(state: State<'_, AppState>, channel: Channel<StreamMsg>) {
    state.streams.register(channel); // engine pushes batches ~15 Hz
}
```
- **Batch on a timer (~15 Hz) with a max batch size**; drop-oldest with a visible "N dropped" counter under back-pressure. Never block the capture thread on IPC.
- Low-frequency lifecycle signals (engine ready/failed, rule changed) may use `app.emit("event-name", payload)` with a matching TS `listen`.

### Frontend side
```ts
import { invoke, Channel } from '@tauri-apps/api/core';
const channel = new Channel<StreamMsg>();
channel.onmessage = (msg) => applyToStore(msg);
await invoke('subscribe_stream', { channel });
```
- All `invoke`/Channel wiring lives in `src/lib/ipc.ts` only.

### App config / manifest
- `tauri.conf.json`: single main window; enable required plugins only.
- **Elevation:** ship a Windows manifest with `requestedExecutionLevel = requireAdministrator` (via `build.rs`/`tauri.conf` bundle manifest). Detect insufficient privilege at startup → emit an "elevation required" state.

---

## 2. `pcap` crate + Npcap (visibility engine)
- Requires **Npcap installed** (WinPcap-compatible mode) and **Admin**.
- **Linking:** the crate links `wpcap.lib`, which ships only in the Npcap **SDK** — not the runtime installer. If the SDK is present, put its `Lib\x64` on the `LIB` environment variable. Otherwise run `scripts/generate-wpcap-lib.ps1` once: it rebuilds an import library from the installed `wpcap.dll` (dumpbin → .def → lib.exe) into `vendor/npcap/lib/`, which `build.rs` adds to the link search path. `LIB` is searched first, so an SDK always wins.
- Enumerate with `Device::list()`; open each chosen device: `Capture::from_device(dev)?.promisc(true).immediate_mode(true).timeout(...).open()?`.
- **Enumeration blocks without elevation.** `pcap_findalldevs` can hang indefinitely when the process is not elevated or the driver is unreachable, so `CaptureEngine::list_interfaces` runs it on a worker thread behind a 5s timeout and reports `EnumerateTimeout` — never let it block a Tauri command.
- Run each capture in its **own thread**; push raw frames into a bounded channel to the parser. **Never parse on the capture thread's hot path more than necessary.**
- Set a reasonable snaplen; enable immediate mode for low latency. Handle interface add/remove gracefully.
- `next_packet()` returns `Error::TimeoutExpired` on idle interfaces — that is normal, loop and re-check the stop flag rather than treating it as failure.
- This path is **read-only** — it never blocks; it only observes and gives full L2 frames.

## 3. `etherparse` + custom L7 (decode)
- Use `etherparse::SlicedPacket::from_ethernet(&frame)` for zero-copy L2–L4 slicing; read Ethernet (MAC, EtherType), IP (v4/v6, TTL, flags), transport (TCP flags/seq/ports, UDP).
- v0.21 shape: `sliced.link` is a `LinkSlice`, `sliced.net` a `NetSlice` (`Ipv4`/`Ipv6`/`Arp`), `sliced.transport` a `TransportSlice` (`Tcp`/`Udp`/`Icmpv4`/`Icmpv6`/`Igmp`). ICMP type/code come from `type_u8()`/`code_u8()`; ARP fields from `ArpPacketSlice` (`operation()`, `sender_protocol_addr()`, …).
- **L7 classifiers** (custom, in `parse/l7/`), each a pure `fn(payload, ctx) -> Option<L7Detail>`:
  - **DNS** (port 53 / mDNS 5353): parse queries/answers → names. Compression pointers must point backwards only, or a malformed packet loops.
  - **HTTP** (cleartext): method, host, path, status.
  - **TLS**: parse ClientHello → **SNI**; ServerHello/cert metadata where visible.
  - **QUIC**: detect long/short header + version. **SNI is not extracted** — it lives in an AEAD-encrypted CRYPTO frame keyed off the destination connection ID, which would need HKDF + AES-GCM on the hot path.
  - **DHCP**, **mDNS**: basic identification.
  - Fallback: `L7Detail::Opaque { hint }`.
- Parsers must be **panic-free and fuzz-friendly** — malformed packets return `None`/`Err`, never crash. Cover each with byte-fixture tests (`src-tauri/tests/fixtures/`, exercised over every truncation prefix).

## 4. `windivert` crate + WinDivert 2.x (enforcement engine)
- Requires the **WinDivert driver/DLL** present and **Admin**. This is the path that makes Cognton a real firewall.
- Open a handle at the **network layer** with a filter (start broad, refine): receive packet → look up flow/attribution → evaluate against the hot rule set → **drop** (don't reinject) on a matching enabled block rule, else **reinject** unchanged.
- Open the **SOCKET layer** (flow-established/closed events) to get authoritative **PID ↔ 5-tuple** mapping in real time; feed this into `attribution`.
- Enforcement runs on its own thread. **Correctness over cleverness:** reinject faithfully; a bug here breaks the user's connectivity. Default action when no rule matches = **allow (reinject)**.
- Keep the WinDivert filter and the in-memory rule set consistent; hot-reload rules on DB change without dropping the handle when avoidable.

## 5. `windows` crate (attribution)
- **PID:** `GetExtendedTcpTable`/`GetExtendedUdpTable` (`TCP_TABLE_OWNER_PID_ALL` / `UDP_TABLE_OWNER_PID`) for **both** `AF_INET` and `AF_INET6` → map `(local_addr, local_port)` → PID. Refresh on a short interval; cache with TTL.
- **Process:** open process (limited query rights) → `QueryFullProcessImageNameW` for full path; derive name.
- **User:** `OpenProcessToken` → `GetTokenInformation(TokenUser)` → SID → `LookupAccountSidW` → `domain\user`. Cache SID→name.
- **All FFI is `unsafe`** — wrap in a safe `attribution/windows/` boundary with `// SAFETY:` notes; handle null/last-error paths. Never call these per packet — resolve per new flow and cache.
- Version specifics for **0.58** (these moved between releases — check before editing): `PSID` and `TOKEN_USER` live in `Win32::Security`, `ConvertSidToStringSidW` in `Win32::Security::Authorization`, `LocalFree` in `Win32::Foundation` (not `System::Memory`), and `LookupAccountSidW` takes bare `PWSTR` buffers, not `Option<PWSTR>`. Required features: `Win32_Foundation`, `Win32_NetworkManagement_IpHelper`, `Win32_Networking_WinSock`, `Win32_Security`, `Win32_Security_Authorization`, `Win32_System_Threading`.
- The `MIB_*TABLE_OWNER_PID` structs are **flexible arrays** (`dwNumEntries` then `[Row; 1]`), so the row slice is rebuilt by hand — always clamp the declared count to what the returned buffer can hold. Back the buffer with `Vec<u32>` (and the `TOKEN_USER` buffer with `Vec<u64>`): `Vec<u8>` makes no alignment promise.
- `dwLocalAddr`/`dwLocalPort` are **network byte order** inside host-order `DWORD`s: read the address bytes as-is and the port from the low two bytes big-endian.
- A row with `dwOwningPid == 0` has **no owner** (TIME_WAIT remnants). Skip it rather than attributing traffic to the Idle process. PID 4 is `System` and cannot be opened at all.
- Attribution needs **no elevation**; only capture does. Without elevation `OpenProcess` fails for processes owned by other accounts, so their PID resolves but path/user do not.

## 6. `rusqlite` (persistence)
- Bundled SQLite (`features = ["bundled"]`). Single connection guarded by a `Mutex`, or a small pool; open the DB in the app data dir.
- **Migrations** run at startup in `store/migrations.rs` (idempotent, versioned via `PRAGMA user_version`).
- Use **prepared statements + bound params** — never string-format SQL. Schema is in `architecture.md`.
- `flows` retention pruning on a timer; per-packet data is **never** persisted.

## 7. Frontend libraries
- **Tailwind:** theme extended from `ui_tokens.md`; utilities only, variants via `cva`. Dark mode via `class` strategy.
- **Zustand:** one store per concern (`stream`, `rules`, `ui`); use selectors + `shallow` to minimize re-renders; the live buffer is a ring in the store, capped.
- **TanStack Table + Virtual:** all packet/flow/rule tables; virtualized rows, column defs typed against the TS DTOs.
- **Recharts:** only for the overview throughput sparkline + protocol breakdown; keep minimal, tokenized colors.
- **lucide-react:** icons; consistent stroke width.
