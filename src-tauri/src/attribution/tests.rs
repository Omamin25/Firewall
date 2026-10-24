use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::atomic::AtomicUsize,
};

use super::*;

const V4_LOCAL: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20));
const V4_REMOTE: IpAddr = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));
const V4_ANY: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const V6_ANY: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);

#[derive(Default)]
struct FakeBackend {
    rows: Vec<ConnectionOwner>,
    processes: HashMap<u32, ProcessInfo>,
    table_reads: AtomicUsize,
    process_reads: AtomicUsize,
    fail_table: bool,
}

impl FakeBackend {
    fn with_rows(rows: Vec<ConnectionOwner>) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }

    fn with_process(mut self, pid: u32, info: ProcessInfo) -> Self {
        self.processes.insert(pid, info);
        self
    }
}

impl AttributionBackend for FakeBackend {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError> {
        self.table_reads.fetch_add(1, Ordering::Relaxed);
        if self.fail_table {
            return Err(AttributionError::ConnectionTable("test failure".to_owned()));
        }
        Ok(self.rows.clone())
    }

    fn process(&self, pid: u32) -> Result<ProcessInfo, AttributionError> {
        self.process_reads.fetch_add(1, Ordering::Relaxed);
        self.processes
            .get(&pid)
            .cloned()
            .ok_or_else(|| AttributionError::Process {
                pid,
                message: "no such process".to_owned(),
            })
    }
}

fn tcp_row(
    local_ip: IpAddr,
    local_port: u16,
    remote: Option<(IpAddr, u16)>,
    pid: u32,
) -> ConnectionOwner {
    ConnectionOwner {
        protocol: TransportProtocol::Tcp,
        local_ip,
        local_port,
        remote,
        pid,
    }
}

fn udp_row(local_ip: IpAddr, local_port: u16, pid: u32) -> ConnectionOwner {
    ConnectionOwner {
        protocol: TransportProtocol::Udp,
        local_ip,
        local_port,
        remote: None,
        pid,
    }
}

fn key(protocol: TransportProtocol, local_port: u16, remote_port: u16) -> FlowKey {
    FlowKey {
        protocol,
        local_ip: V4_LOCAL,
        local_port,
        remote_ip: V4_REMOTE,
        remote_port,
    }
}

fn resolver(backend: &Arc<FakeBackend>, config: ResolverConfig) -> AttributionResolver {
    AttributionResolver::with_config(Arc::clone(backend) as Arc<dyn AttributionBackend>, config)
}

/// TTLs long enough that nothing expires mid-test.
fn stable_config() -> ResolverConfig {
    ResolverConfig {
        table_ttl: Duration::from_secs(600),
        min_refresh_interval: Duration::from_secs(600),
        process_ttl: Duration::from_secs(600),
    }
}

#[test]
fn prefers_the_exact_five_tuple_over_looser_matches() {
    let backend = Arc::new(FakeBackend::with_rows(vec![
        // A listener and a connected socket share the port; only the peer disambiguates.
        tcp_row(V4_ANY, 443, None, 100),
        tcp_row(V4_LOCAL, 51000, Some((V4_REMOTE, 443)), 4242),
    ]));
    let resolver = resolver(&backend, stable_config());

    let attribution = resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    assert_eq!(attribution.pid, Some(4242));
}

#[test]
fn falls_back_to_the_local_endpoint_when_the_peer_is_unknown() {
    let backend = Arc::new(FakeBackend::with_rows(vec![udp_row(V4_LOCAL, 5353, 777)]));
    let resolver = resolver(&backend, stable_config());

    let attribution = resolver.resolve(&key(TransportProtocol::Udp, 5353, 5353));
    assert_eq!(attribution.pid, Some(777));
}

#[test]
fn falls_back_to_a_wildcard_bound_port_last() {
    // A dual-stack socket on [::]:5353 must still own IPv4 traffic on that port.
    let backend = Arc::new(FakeBackend::with_rows(vec![udp_row(V6_ANY, 5353, 999)]));
    let resolver = resolver(&backend, stable_config());

    let attribution = resolver.resolve(&key(TransportProtocol::Udp, 5353, 5353));
    assert_eq!(attribution.pid, Some(999));
}

#[test]
fn port_less_protocols_are_never_looked_up() {
    let backend = Arc::new(FakeBackend::default());
    let resolver = resolver(&backend, stable_config());

    let attribution = resolver.resolve(&key(TransportProtocol::Icmp, 0, 0));
    assert_eq!(attribution, Attribution::default());
    assert_eq!(backend.table_reads.load(Ordering::Relaxed), 0);
}

#[test]
fn the_connection_table_is_read_once_within_its_ttl() {
    let backend = Arc::new(FakeBackend::with_rows(vec![tcp_row(
        V4_LOCAL,
        51000,
        Some((V4_REMOTE, 443)),
        4242,
    )]));
    let resolver = resolver(&backend, stable_config());

    for _ in 0..50 {
        resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    }
    assert_eq!(backend.table_reads.load(Ordering::Relaxed), 1);
}

#[test]
fn a_miss_forces_one_extra_refresh_when_the_floor_has_elapsed() {
    let backend = Arc::new(FakeBackend::default());
    let config = ResolverConfig {
        min_refresh_interval: Duration::ZERO,
        ..stable_config()
    };
    let resolver = resolver(&backend, config);

    resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    // One scheduled read plus exactly one forced retry — not a read per lookup.
    assert_eq!(backend.table_reads.load(Ordering::Relaxed), 2);
}

#[test]
fn a_miss_respects_the_refresh_floor() {
    let backend = Arc::new(FakeBackend::default());
    let resolver = resolver(&backend, stable_config());

    for _ in 0..10 {
        resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    }
    assert_eq!(backend.table_reads.load(Ordering::Relaxed), 1);
}

#[test]
fn process_metadata_is_memoised_per_pid() {
    let backend = Arc::new(
        FakeBackend::with_rows(vec![tcp_row(V4_LOCAL, 51000, Some((V4_REMOTE, 443)), 4242)])
            .with_process(
                4242,
                ProcessInfo {
                    name: Some("firefox.exe".to_owned()),
                    path: Some(r"C:\Program Files\Mozilla Firefox\firefox.exe".to_owned()),
                    user_account: Some(r"COGNTON\vraj".to_owned()),
                    user_sid: Some("S-1-5-21-1".to_owned()),
                },
            ),
    );
    let resolver = resolver(&backend, stable_config());

    for _ in 0..20 {
        let attribution = resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
        assert_eq!(attribution.process_name.as_deref(), Some("firefox.exe"));
        assert_eq!(attribution.user_account.as_deref(), Some(r"COGNTON\vraj"));
    }
    assert_eq!(backend.process_reads.load(Ordering::Relaxed), 1);
}

#[test]
fn an_unreadable_process_is_still_attributed_by_pid() {
    let backend = Arc::new(FakeBackend::with_rows(vec![tcp_row(
        V4_LOCAL,
        51000,
        Some((V4_REMOTE, 443)),
        4,
    )]));
    let resolver = resolver(&backend, stable_config());

    let attribution = resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    assert_eq!(attribution.pid, Some(4));
    assert_eq!(attribution.process_name, None);
    // The failure is cached, so a protected process costs one syscall, not one per frame.
    resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443));
    assert_eq!(backend.process_reads.load(Ordering::Relaxed), 1);
}

#[test]
fn an_unreadable_connection_table_degrades_to_empty_attribution() {
    let backend = Arc::new(FakeBackend {
        fail_table: true,
        ..FakeBackend::default()
    });
    let resolver = resolver(&backend, stable_config());

    for _ in 0..10 {
        assert_eq!(
            resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443)),
            Attribution::default()
        );
    }
    // A failing backend must not be retried per packet.
    assert_eq!(backend.table_reads.load(Ordering::Relaxed), 1);
}

#[test]
fn sockets_with_no_owning_process_are_ignored() {
    // Windows reports PID 0 for TIME_WAIT remnants; attributing those to Idle would be
    // worse than showing nothing.
    let backend = Arc::new(FakeBackend::with_rows(vec![tcp_row(
        V4_LOCAL,
        51000,
        Some((V4_REMOTE, 443)),
        0,
    )]));
    let resolver = resolver(&backend, stable_config());

    assert_eq!(
        resolver.resolve(&key(TransportProtocol::Tcp, 51000, 443)),
        Attribution::default()
    );
}
