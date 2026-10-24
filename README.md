# Cognton Firewall

> Next-generation high-performance Windows network analyzer & firewall with L2–L7 packet decode, process & Windows user attribution, and kernel-level WinDivert enforcement.

---

## Overview

Cognton is an elevated Windows firewall and traffic inspection application built with **Tauri v2**, **Rust**, and **React**. It brings kernel-level packet filtering, real-time protocol breakdown, process path and Windows user attribution, and low-latency rule enforcement into a sleek Vercel/Geist-inspired desktop UI.

---

## Key Features

- **L2–L7 Deep Packet Inspection**: Promiscuous Npcap packet capture across active network adapters with real-time parsers for IPv4/IPv6, ARP, TCP/UDP, ICMP, DNS, HTTP, TLS (SNI extraction), QUIC, DHCP, mDNS.
- **Process & User Attribution**: Authoritative 5-tuple ownership resolution mapping live network flows to exact executable image paths (`QueryFullProcessImageNameW`), process IDs, and Windows user accounts (`domain\user` via token SID lookup).
- **Kernel-Level Enforcement**: Zero-cost default posture using **WinDivert 2.x**. Network-layer intercept loop evaluates packets against active rules in real-time, dropping matches and reinjecting safe traffic.
- **Rules Management UI**: Full priority-ordered rule matching (Allow beats Block at same priority, lower priority number evaluated first). Prefill rules directly from live connection rows or packet drawer details.
- **Virtualized High-Volume Tables**: TanStack Virtual rendered tables (Live Traffic, Connections, Rules) capable of sustaining 50,000+ packets/sec without UI thread lag.
- **Settings & Retention Pruning**: Configurable rolling history retention (default 24h, up to 7200h) and SQLite row caps (default 100,000 rows) with manual pruning and live diagnostic health metrics.

---

## Prerequisites & Elevation

1. **Windows 10/11 64-bit**
2. **Npcap Driver**: Install Npcap with **"WinPcap API-compatible mode"** checked.
3. **WinDivert Runtime**: `WinDivert.dll` and `WinDivert64.sys` staged in the output folder (automatically handled by `build.rs` or bundled by installer).
4. **Administrator Privileges**: Cognton must be launched with elevated privileges ("Run as Administrator") for Npcap interface binding and WinDivert kernel driver handles.

---

## Getting Started

### Installation / Building from Source

```powershell
# 1. Clone repo & install frontend dependencies
git clone https://github.com/vraj/cognton.git
cd cognton
npm install

# 2. Run backend tests (in src-tauri)
cd src-tauri
cargo test
cd ..

# 3. Run frontend tests
npm test

# 4. Launch in development mode (from an elevated Developer PowerShell)
npm run tauri dev
```

### Packaging / Installer Build

```powershell
npm run tauri build
```

Installers land under `src-tauri/target/release/bundle/`.

`WinDivert.dll` and `WinDivert64.sys` are bundled into the resource root, which on Windows
is the install directory itself — the DLL is a load-time import of the executable, so it
must sit directly beside `Cognton.exe`, and `WinDivertOpen` loads the `.sys` driver
relative to the DLL's own folder.

**Npcap is not bundled.** Its licence does not permit redistribution without a paid OEM
agreement, so it has to be installed separately (step 2 above). If it is missing, interface
enumeration fails with an actionable message and the Settings health card shows
**Npcap Capture: Not detected** — enforcement and the rules engine still work, but there is
nothing to observe.

---

## Architecture Summary

```
                  ┌──────────────────────────────────────────────┐
                  │                 React + Vite                 │
                  │  (Zustand Stores, TanStack Virtual, Recharts)│
                  └──────────────────────▲───────────────────────┘
                                         │  Tauri IPC (Channel @ ~15 Hz)
                  ┌──────────────────────▼───────────────────────┐
                  │               Rust Core Engine               │
                  ├──────────────────────┬───────────────────────┤
                  │     Capture Engine   │  Enforcement Engine   │
                  │   (Npcap / Promisc)  │     (WinDivert 2.x)   │
                  ├──────────────────────┼───────────────────────┤
                  │ L2–L7 Decoders       │ Attribution Resolver  │
                  │ (etherparse, SNI)    │ (Win32 PID/SID Cache) │
                  ├──────────────────────┴───────────────────────┤
                  │          rusqlite Store (Local DB)           │
                  └──────────────────────────────────────────────┘
```

---

## License & Compliance

All network packet interception and rule enforcement operate strictly **locally** on your device. Zero metrics or packet data are transmitted off the system.
