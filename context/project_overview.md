# Project Overview — Cognton Firewall

## 1. Product
**Cognton** is a local, host-based firewall and network inspector for Windows. It runs as a single native desktop application on the user's laptop, captures **all** internet traffic reaching (and leaving) the machine, decodes it across the full stack (**L2–L7**), attributes every flow to the exact **process and OS user account** responsible, and lets the user **actively block or allow** traffic through rules — all from a clean, minimal, professional dashboard running locally.

It is a **true firewall**, not just a monitor: capture + inspect + enforce.

## 2. Core Purpose
Give a technical user complete visibility and control over what their machine is talking to on the network:
- **See everything** — every packet and every connection, in real time, with per-layer detail.
- **Understand it** — which application, which Windows user account, which remote host/domain, which protocol.
- **Control it** — block or allow flows by IP, port, process, user, protocol, or direction, and have those decisions enforced immediately at the packet level.

## 3. Target Audience / Users
- **Security-conscious power users & developers** who want to audit what software on their machine is sending where.
- **Security engineers / analysts** doing local triage, malware behavior inspection, or egress auditing.
- **Learners** studying networking who want a tangible L2–L7 view of live traffic.

Assumption: the user is technical, runs the app **with Administrator privileges** (required for capture and enforcement), and is on **Windows 10/11 x64**.

## 4. Primary User Flow
1. **Launch** Cognton (elevated). It initializes the capture engine (Npcap) and the enforcement engine (WinDivert), then begins streaming live traffic to the dashboard.
2. **Live Traffic view** shows packets/flows arriving in real time: timestamp, direction, remote host/domain, protocol (L7), process, user, bytes.
3. User **clicks a packet or flow** → a detail panel decodes it layer by layer: L2 (MAC, EtherType), L3 (IP, TTL, flags), L4 (ports, TCP flags/seq), L7 (DNS query, HTTP host/method, TLS SNI, etc.), plus attribution (process path, PID, user account).
4. User **filters/searches** (by process, user, remote IP/domain, port, protocol, direction) to isolate what matters.
5. User **creates a rule** — e.g. "block all outbound from `telemetry.exe`" or "block IP 1.2.3.4:443" — from a selected flow or manually. The rule is persisted and enforced **immediately** (matching packets are dropped by WinDivert).
6. **Dashboard overview** summarizes: total throughput, top talkers (by process/host), active connections, blocked count, protocol breakdown.

## 5. In Scope (MVP)
- Live capture of **all** interfaces' traffic (Npcap), full L2 Ethernet frames.
- **L2–L7 decode**: MAC/EtherType (L2), IPv4/IPv6/ARP (L3), TCP/UDP/ICMP (L4), and L7 classification for **DNS, HTTP, TLS (SNI), QUIC, DHCP, mDNS** at minimum, with graceful "unknown/opaque" fallback.
- **Per-connection attribution**: process name + full path + PID, and **Windows user account** (domain\user via SID lookup), resolved from OS connection tables and WinDivert socket events.
- **Flow aggregation**: group packets into connections/flows with byte/packet counters and duration.
- **Active enforcement**: rule-based **block/allow** enforced via WinDivert; rules match on direction, protocol, remote/local IP + port, process, and user.
- **Rules management UI**: create/edit/enable/disable/delete rules; create-rule-from-flow.
- **Persistence**: rules and flow history stored locally (SQLite).
- **Dashboard**: live traffic table, connections table, packet-detail inspector, rules manager, and an overview page with throughput + top-talkers.
- **Clean, minimal, Vercel/Geist-style UI**, light + dark mode, running fully offline on the local machine.

## 6. Out of Scope (explicitly, to prevent feature creep)
- **Non-Windows platforms** (macOS/Linux). Architecture keeps the OS layer behind a trait so it *could* be ported later, but no other OS is built for the MVP.
- **Deep TLS decryption / MITM** — we read cleartext metadata (SNI, cert info) only; no interception of encrypted payloads.
- **Full DPI / signature-based IDS/IPS**, threat-intel feeds, or reputation scoring.
- **Cloud sync, multi-device management, accounts, or any remote/hosted component.** The app is 100% local.
- **Historical analytics beyond a rolling local window**, long-term data warehousing, or reporting exports (CSV export is a possible post-MVP nicety, not MVP).
- **Bandwidth shaping / QoS**, VPN, or NAT features.
- **Mobile builds.**
- **Automatic rule suggestions / ML.**

## 7. Success Criteria
- With the app running, opening a browser or app produces visible, correctly-decoded flows within ~1 second, attributed to the right process **and** user.
- Creating a block rule for a running flow stops that traffic (verifiable: the connection stalls / fails) within ~1 second.
- The UI stays smooth (no jank) under sustained real-world traffic via batched, virtualized rendering.
- A reviewer can open a packet and see a correct, readable L2→L7 breakdown.
