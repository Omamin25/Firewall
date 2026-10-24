# Cognton Firewall

**A Windows network monitoring and firewall application for real-time traffic inspection, process attribution, and rule-based packet filtering.**

Cognton combines a Rust-powered backend with a modern desktop interface to help users inspect network activity, understand application-level connections, and manage firewall rules from one place.

Built with **Tauri v2, Rust, React, and TypeScript**, Cognton brings packet visibility and network control together in a desktop application designed for Windows.

> **Platform:** Windows 10/11 (64-bit) · **Interface:** React + Tauri · **Core Engine:** Rust

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Technology Stack](#technology-stack)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
- [Building the Installer](#building-the-installer)
- [Important Notes](#important-notes)
- [Privacy and Security](#privacy-and-security)
- [License and Attribution](#license-and-attribution)

---

## Overview

Modern network monitoring requires more than a list of IP addresses. Understanding which applications generate traffic, inspecting protocol metadata, and applying filtering rules can make network activity easier to investigate.

Cognton brings these capabilities into a unified Windows desktop interface.

The application combines packet capture, protocol parsing, process and Windows user attribution, persistent local storage, and packet-filtering rules through a Rust backend and a React-based frontend.

### Why Cognton?

- **Visibility:** Inspect network traffic and supported protocol metadata in real time.
- **Attribution:** Associate network activity with processes and Windows user accounts where resolution is available.
- **Control:** Configure prioritized allow and block rules using the enforcement engine.
- **Performance:** Use virtualized tables and asynchronous backend processing to support high-volume activity.
- **Local-first design:** Keep packet inspection and application data on the local machine, as described by the project implementation.

---

## Features

### 1. Deep Packet Inspection

Inspect packet headers and supported protocol metadata across multiple network layers.

Supported protocol parsers include:

- **Network:** IPv4, IPv6, ARP
- **Transport:** TCP, UDP, ICMP
- **Application:** DNS, HTTP, TLS Server Name Indication (SNI), QUIC, DHCP, mDNS

Capture availability and the metadata exposed depend on the network adapter, operating system, traffic characteristics, and installed dependencies.

### 2. Process and User Attribution

Associate observed network flows with the processes responsible for generating them.

- Resolve process IDs (PIDs) and executable paths.
- Resolve Windows user identity using security token information and SID lookup.
- Use flow information, including source and destination addresses and ports, to help match network activity to processes.
- Cache attribution information to reduce repeated lookup overhead.

Attribution depends on the availability of operating-system information and the lifetime of the process or connection.

### 3. Rule-Based Packet Filtering

Apply prioritized firewall rules through the WinDivert-based enforcement engine.

- Configure allow and block actions.
- Evaluate rules according to priority.
- Support the documented tie-breaking behavior: allow takes precedence over block at the same priority.
- Drop packets that match applicable blocking rules and reinject traffic permitted by the enforcement logic.
- Create rules using information from live connections and packet details.

**Important:** Filtering behavior depends on the configured rules, packet direction, and enforcement implementation. Test rule changes carefully to avoid disrupting network connectivity.

### 4. Real-Time Traffic and Connection Views

Explore network activity through a desktop interface designed for high-volume data.

- Inspect live traffic and connection details.
- Browse and manage firewall rules.
- Use virtualized tables powered by TanStack Virtual.
- Inspect packet details and use available information to create rules.

The project is designed to reduce rendering overhead when handling large data sets. Actual throughput and responsiveness depend on the system, workload, and capture configuration.

### 5. Local Storage and Retention

Use SQLite-backed storage and configurable retention controls for historical data.

- Configure rolling history retention.
- Manage stored-row limits.
- Prune historical records.
- Review diagnostic information through the settings interface.

The documented defaults are a 24-hour retention period and a 100,000-row cap, with retention configurable up to 7,200 hours.

---

## Technology Stack

| Component | Technologies |
|---|---|
| Desktop framework | Tauri v2 |
| Backend | Rust |
| Frontend | React, TypeScript, Vite |
| State management | Zustand |
| Table virtualization | TanStack Virtual |
| Charts | Recharts |
| Packet capture | Npcap |
| Packet enforcement | WinDivert 2.x |
| Packet parsing | `etherparse` and protocol-specific parsers |
| Local database | SQLite via `rusqlite` |
| Windows integration | Win32 process and security-token APIs |

---

## Architecture

Cognton separates the user interface, network-processing components, and local persistence into distinct layers.

```text
┌───────────────────────────────────────────┐
│              React + Vite UI              │
│  Traffic · Connections · Rules · Settings │
└─────────────────────┬─────────────────────┘
                      │
                      │ Tauri IPC
                      ▼
┌───────────────────────────────────────────┐
│                Rust Core                  │
├─────────────────────┬─────────────────────┤
│   Capture Engine    │ Enforcement Engine  │
│       Npcap         │    WinDivert 2.x    │
├─────────────────────┴─────────────────────┤
│           Packet and Flow Processing      │
│       Protocol Parsing · Attribution      │
├───────────────────────────────────────────┤
│            SQLite Local Storage           │
└───────────────────────────────────────────┘
```

### Component responsibilities

- **Frontend:** Displays traffic, connection information, firewall rules, settings, and diagnostics.
- **Capture engine:** Uses Npcap to capture traffic from supported network interfaces.
- **Enforcement engine:** Uses WinDivert to intercept and process traffic according to applicable rules.
- **Protocol processing:** Extracts supported packet and protocol metadata.
- **Attribution resolver:** Uses Windows process and security information to associate traffic with processes and users.
- **Local storage:** Persists application data and supports retention management.

---

## Prerequisites

Before building or running Cognton, prepare the following:

1. **Windows 10 or Windows 11, 64-bit**
2. **Node.js and npm** compatible with the project's dependencies
3. **Rust toolchain** with Cargo installed
4. **Npcap**, installed separately, with WinPcap API-compatible mode enabled if required by the capture implementation
5. **WinDivert 2.x runtime files**, including the appropriate DLL and driver files
6. **Administrator privileges** for operations that require elevated network capture or packet-filtering access

Use the versions and installation instructions specified by the project and its dependencies.

---

## Getting Started

### 1. Clone the repository

```powershell
git clone https://github.com/Omamin25/firewall.git
cd firewall
```

If you are working from an existing local copy, open PowerShell in the project directory instead.

### 2. Install frontend dependencies

```powershell
npm install
```

### 3. Run backend tests

```powershell
cd src-tauri
cargo test
cd ..
```

### 4. Run frontend tests

```powershell
npm test
```

Run this command only if the project's `package.json` defines the corresponding test script.

### 5. Start the desktop application

Open an elevated PowerShell or Developer PowerShell when required by the capture and enforcement components.

```powershell
npm run tauri dev
```

Install Npcap and prepare the required WinDivert runtime files before testing the relevant features.

---

## Building the Installer

To create a release build, run the following from the project root:

```powershell
npm run tauri build
```

The generated application bundles and installers are generally placed under:

```text
src-tauri/target/release/bundle/
```

The exact output depends on the configured Tauri bundlers and build environment.

### Runtime dependencies

- **Npcap:** Must be installed separately. Do not redistribute it unless the applicable license and distribution terms permit it.
- **WinDivert:** Ensure the correct runtime DLL and driver files are included in the appropriate locations required by the build and loading configuration.
- **Permissions:** Packet capture and enforcement features may require administrator privileges.

---

## Important Notes

- Cognton is designed for Windows; it is not a cross-platform firewall implementation.
- Packet capture and packet enforcement are separate capabilities and may have different dependency requirements.
- Protocol inspection does not imply that encrypted application payloads can be decrypted. For TLS, SNI extraction is limited to cases where the relevant metadata is observable.
- Incorrect firewall rules may interrupt network connectivity.
- Performance figures should be benchmarked on the target machine and workload rather than assumed from design goals.

---

## Privacy and Security

Cognton is designed around local network inspection and local data storage.

Before using it in a sensitive environment:

- Review the source code and third-party dependencies.
- Verify the application's actual network behavior.
- Avoid collecting or sharing packet captures that contain sensitive information.
- Use elevated privileges only when necessary.
- Review the original project's license and security implications before redistribution.

The privacy statements in this README describe the intended design and should be verified against the actual implementation.


