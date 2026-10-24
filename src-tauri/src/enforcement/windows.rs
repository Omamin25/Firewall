//! WinDivert 2.x backend. Every WinDivert type stays inside this file so the rest of the
//! enforcement module — and its tests — remain platform-neutral.

use std::{
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use windivert::{
    address::WinDivertAddress,
    error::{WinDivertError, WinDivertRecvError},
    layer::{NetworkLayer, SocketLayer},
    prelude::{WinDivertEvent, WinDivertFlags, WinDivertParam},
    ShutdownHandle, WinDivert,
};

use super::{
    Decider, Decision, EnforcementBackend, EnforcementError, Observer, PacketMeta, Ready,
    SocketEvent, SocketEventKind,
};
use crate::model::TransportProtocol;

/// Loopback is deliberately excluded: diverting it would put Cognton between local-only
/// services (including the dev server) and the stack for no security benefit. Rules
/// therefore govern traffic that actually leaves or enters the host.
const NETWORK_FILTER: &str = "(ip || ipv6) && !loopback";
/// Bind (3), connect (4), accept (6) and close (7). Listen carries no peer and tells us
/// nothing an accept will not.
const SOCKET_FILTER: &str = "event == 3 || event == 4 || event == 6 || event == 7";
/// Handle priority. Mid-range so other WinDivert users can sit deliberately above or
/// below us; the value only matters relative to other handles.
const PRIORITY: i16 = 0;
/// How long a network receive waits before the loop re-checks its stop flag.
const RECV_TIMEOUT_MS: u32 = 200;
/// How often the socket watcher checks whether it should shut its handle down.
const SHUTDOWN_POLL: Duration = Duration::from_millis(200);
/// Covers the largest possible IP packet, so a diverted packet is never too big to
/// receive — dropping one for lack of buffer would be a silent block.
const BUFFER_SIZE: usize = u16::MAX as usize;
/// Kernel queue sizing. The defaults are modest and an overrun means the driver silently
/// drops the user's packets, so both are raised.
const QUEUE_LENGTH: u64 = 8_192;
const QUEUE_TIME_MS: u64 = 2_000;

pub struct WinDivertBackend;
impl EnforcementBackend for WinDivertBackend {
    fn run_network(
        &self,
        decide: Decider,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        // Divert mode (no SNIFF): a packet reaches the stack only if we reinject it. That
        // is what gives a block rule teeth.
        let handle = WinDivert::network(NETWORK_FILTER, PRIORITY, WinDivertFlags::new())
            .map_err(|error| EnforcementError::Open(error.to_string()))?;
        tune(&handle);
        ready();

        let mut buffer = vec![0u8; BUFFER_SIZE];
        while !stop.load(Ordering::Relaxed) {
            let packet = match handle.recv_wait(&mut buffer, RECV_TIMEOUT_MS) {
                Ok(Some(packet)) => packet,
                // Timed out with nothing queued: normal on an idle link.
                Ok(None) => continue,
                Err(WinDivertError::Recv(WinDivertRecvError::InsufficientBuffer)) => {
                    tracing::warn!("an intercepted packet exceeded the receive buffer");
                    continue;
                }
                Err(error) => {
                    return Err(EnforcementError::Handle {
                        layer: "network",
                        message: error.to_string(),
                    })
                }
            };

            // Correctness over cleverness (`library_docs.md` §4): anything we cannot
            // decode, and anything no rule blocks, goes back unchanged.
            let verdict = PacketMeta::decode(&packet.data, packet.address.outbound())
                .map(|meta| decide(&meta))
                .unwrap_or(Decision::Reinject);
            if verdict == Decision::Drop {
                continue;
            }
            if let Err(error) = handle.send(&packet) {
                // One failed reinjection loses one packet and TCP retransmits. Giving up
                // here would blackhole everything after it, so only log.
                tracing::warn!(%error, "could not reinject a packet");
            }
        }
        Ok(())
    }

    fn run_socket(
        &self,
        observe: Observer,
        ready: Ready,
        stop: Arc<AtomicBool>,
    ) -> Result<(), EnforcementError> {
        // SNIFF is mandatory here: a socket-layer handle *without* it blocks the socket
        // operations it matches, which would break every connection on the machine.
        let flags = WinDivertFlags::new().set_sniff().set_recv_only();
        let handle = WinDivert::socket(SOCKET_FILTER, PRIORITY, flags)
            .map_err(|error| EnforcementError::Open(error.to_string()))?;
        ready();

        // Socket events carry no payload, and this crate's timeout-based receive reports
        // a zero-length read as an error, so the loop blocks instead and a watcher shuts
        // the handle down when the engine asks it to stop.
        let watcher = Watcher::spawn(handle.shutdown_handle(), Arc::clone(&stop));
        let outcome = loop {
            if stop.load(Ordering::Relaxed) {
                break Ok(());
            }
            match handle.recv() {
                Ok(packet) => {
                    if let Some(event) = socket_event(&packet.address) {
                        observe(event);
                    }
                }
                // The queue is drained and the handle is shut down: an orderly stop.
                Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => break Ok(()),
                Err(error) => {
                    break Err(EnforcementError::Handle {
                        layer: "socket",
                        message: error.to_string(),
                    })
                }
            }
        };
        watcher.join();
        outcome
    }
}

/// Interrupts a blocking receive. WinDivert has no per-call timeout for the socket layer,
/// so stopping means shutting the handle down from another thread.
struct Watcher {
    done: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl Watcher {
    fn spawn(shutdown: ShutdownHandle, stop: Arc<AtomicBool>) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let thread_done = Arc::clone(&done);
        let join = thread::Builder::new()
            .name("cognton-sockets-stop".to_owned())
            .spawn(move || {
                while !thread_done.load(Ordering::Relaxed) {
                    if stop.load(Ordering::Relaxed) {
                        if let Err(error) = shutdown.shutdown() {
                            tracing::warn!(%error, "could not shut the socket handle down");
                        }
                        return;
                    }
                    thread::sleep(SHUTDOWN_POLL);
                }
            })
            .ok();
        Self { done, join }
    }

    fn join(mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Raises the kernel queue limits. Failures are logged, not fatal: the handle still works
/// with the driver's defaults.
fn tune(handle: &WinDivert<NetworkLayer>) {
    for (param, value) in [
        (WinDivertParam::QueueLength, QUEUE_LENGTH),
        (WinDivertParam::QueueTime, QUEUE_TIME_MS),
    ] {
        if let Err(error) = handle.set_param(param, value) {
            tracing::warn!(?param, %error, "could not tune the WinDivert queue");
        }
    }
}

/// Translates one socket event into the platform-neutral shape attribution consumes.
/// Events on a transport rules cannot name are ignored.
fn socket_event(address: &WinDivertAddress<SocketLayer>) -> Option<SocketEvent> {
    let kind = match address.event() {
        WinDivertEvent::SocketBind
        | WinDivertEvent::SocketConnect
        | WinDivertEvent::SocketAccept => SocketEventKind::Opened,
        WinDivertEvent::SocketClose => SocketEventKind::Closed,
        _ => return None,
    };
    Some(SocketEvent {
        kind,
        pid: address.process_id(),
        protocol: transport(address.protocol())?,
        local_ip: normalise(address.local_address()),
        local_port: address.local_port(),
    })
}

/// IANA protocol numbers, limited to the transports rules can name.
fn transport(protocol: u8) -> Option<TransportProtocol> {
    match protocol {
        6 => Some(TransportProtocol::Tcp),
        17 => Some(TransportProtocol::Udp),
        1 | 58 => Some(TransportProtocol::Icmp),
        2 => Some(TransportProtocol::Igmp),
        _ => None,
    }
}

/// WinDivert reports dual-stack sockets as IPv4-mapped IPv6. Unmapping them means a hint
/// keys the same way the capture path's flow does.
fn normalise(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn maps_iana_protocol_numbers() {
        assert_eq!(transport(6), Some(TransportProtocol::Tcp));
        assert_eq!(transport(17), Some(TransportProtocol::Udp));
        assert_eq!(transport(58), Some(TransportProtocol::Icmp));
        assert_eq!(transport(89), None);
    }

    #[test]
    fn unmaps_ipv4_mapped_addresses() {
        let mapped = IpAddr::V6(Ipv4Addr::new(192, 168, 1, 20).to_ipv6_mapped());
        assert_eq!(
            normalise(mapped),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))
        );
        assert_eq!(
            normalise(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            IpAddr::V6(Ipv6Addr::LOCALHOST)
        );
    }
}
