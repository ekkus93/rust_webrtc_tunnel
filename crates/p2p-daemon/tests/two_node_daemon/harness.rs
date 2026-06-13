//! Shared test harness for the two-node daemon integration tests: an in-memory
//! signaling transport with fault injection, config/identity builders, TCP client
//! and echo-target helpers, status-file polling/predicates, signaling-trace decoding,
//! and the `run_one_in_memory_session` scenario driver. Pulled in as a module by the
//! `two_node_daemon` test binary so every test shares one harness.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU16, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use p2p_core::{
    AppConfig, BrokerConfig, BrokerTlsConfig, ForwardAnswerConfig, ForwardOfferConfig, ForwardRule,
    HealthConfig, LoggingConfig, MessageType, NodeConfig, NodeRole, PathConfig, PeerConfig,
    ReconnectConfig, SecurityConfig, TunnelConfig, WebRtcConfig,
};
use p2p_crypto::{AuthorizedKeys, GeneratedIdentity, IdentityFile, generate_identity};
use p2p_daemon::{
    DaemonSignalingTransport, OfferSessionTestHandle, run_answer_daemon_with_transport,
    run_offer_daemon_with_transport_and_test_hook,
};
use p2p_signaling::{ReplayCache, SignalCodec};
use p2p_webrtc::IceConnectionState;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};

#[derive(Clone, Default)]
pub(crate) struct TransportTrace {
    attempts: Arc<Mutex<Vec<TransportAttempt>>>,
    payloads_by_recipient: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
}

impl TransportTrace {
    pub(crate) fn record(
        &self,
        from_peer_id: &str,
        peer_id: &p2p_core::PeerId,
        payload: &[u8],
        delivered: bool,
    ) {
        self.attempts.lock().expect("trace mutex should lock").push(TransportAttempt {
            from_peer_id: from_peer_id.to_owned(),
            to_peer_id: peer_id.to_string(),
            payload: payload.to_vec(),
            delivered,
        });
        let mut payloads = self.payloads_by_recipient.lock().expect("trace mutex should lock");
        payloads.entry(peer_id.to_string()).or_default().push(payload.to_vec());
    }

    pub(crate) fn payloads_for(&self, peer_id: &str) -> Vec<Vec<u8>> {
        self.payloads_by_recipient
            .lock()
            .expect("trace mutex should lock")
            .get(peer_id)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn attempts(&self) -> Vec<TransportAttempt> {
        self.attempts.lock().expect("trace mutex should lock").clone()
    }
}

#[derive(Clone)]
pub(crate) struct TransportAttempt {
    pub(crate) from_peer_id: String,
    pub(crate) to_peer_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) delivered: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct RouteKey {
    pub(crate) from_peer_id: String,
    pub(crate) to_peer_id: String,
}

impl RouteKey {
    pub(crate) fn new(from_peer_id: impl Into<String>, to_peer_id: impl Into<String>) -> Self {
        Self { from_peer_id: from_peer_id.into(), to_peer_id: to_peer_id.into() }
    }
}

#[derive(Default)]
pub(crate) struct TransportFaults {
    publish_failures: HashMap<RouteKey, usize>,
    dropped_deliveries: HashMap<RouteKey, usize>,
    duplicate_deliveries: HashMap<RouteKey, usize>,
    delayed_deliveries_ms: HashMap<RouteKey, u64>,
}

#[derive(Clone, Default)]
pub(crate) struct TransportFaultControl {
    faults: Arc<Mutex<TransportFaults>>,
    routes: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<InMemoryEvent>>>>,
}

impl TransportFaultControl {
    pub(crate) fn fail_next_publish(&self, from_peer_id: &str, to_peer_id: &str, count: usize) {
        self.faults
            .lock()
            .expect("fault mutex should lock")
            .publish_failures
            .insert(RouteKey::new(from_peer_id, to_peer_id), count);
    }

    pub(crate) fn drop_next_delivery(&self, from_peer_id: &str, to_peer_id: &str, count: usize) {
        self.faults
            .lock()
            .expect("fault mutex should lock")
            .dropped_deliveries
            .insert(RouteKey::new(from_peer_id, to_peer_id), count);
    }

    pub(crate) fn duplicate_next_delivery(
        &self,
        from_peer_id: &str,
        to_peer_id: &str,
        count: usize,
    ) {
        self.faults
            .lock()
            .expect("fault mutex should lock")
            .duplicate_deliveries
            .insert(RouteKey::new(from_peer_id, to_peer_id), count);
    }

    pub(crate) fn delay_next_delivery(&self, from_peer_id: &str, to_peer_id: &str, delay_ms: u64) {
        self.faults
            .lock()
            .expect("fault mutex should lock")
            .delayed_deliveries_ms
            .insert(RouteKey::new(from_peer_id, to_peer_id), delay_ms);
    }

    pub(crate) fn inject_poll_failure(&self, peer_id: &str) {
        let sender = self
            .routes
            .lock()
            .expect("routes mutex should lock")
            .get(peer_id)
            .cloned()
            .expect("poll failure route should exist");
        sender
            .send(InMemoryEvent::PollFailure("injected in-memory poll failure".to_owned()))
            .expect("poll failure receiver should be alive");
    }

    pub(crate) fn inject_payload(&self, peer_id: &str, payload: Vec<u8>) {
        let sender = self
            .routes
            .lock()
            .expect("routes mutex should lock")
            .get(peer_id)
            .cloned()
            .expect("payload route should exist");
        sender.send(InMemoryEvent::Payload(payload)).expect("payload receiver should be alive");
    }
}

#[derive(Clone)]
pub(crate) enum InMemoryEvent {
    Payload(Vec<u8>),
    PollFailure(String),
}

pub(crate) struct InMemoryTransport {
    peer_id: String,
    inbox: mpsc::UnboundedReceiver<InMemoryEvent>,
    routes: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<InMemoryEvent>>>>,
    faults: Arc<Mutex<TransportFaults>>,
    trace: TransportTrace,
}

impl DaemonSignalingTransport for InMemoryTransport {
    async fn subscribe_own_topic(&mut self) -> Result<(), p2p_signaling::SignalingError> {
        Ok(())
    }

    async fn publish_signal(
        &mut self,
        peer_id: &p2p_core::PeerId,
        _topic_prefix: &str,
        payload: Vec<u8>,
    ) -> Result<(), p2p_signaling::SignalingError> {
        let route = self
            .routes
            .lock()
            .expect("routes mutex should lock")
            .get(peer_id.as_str())
            .cloned()
            .ok_or_else(|| {
                p2p_signaling::SignalingError::Protocol(format!(
                    "missing in-memory route for {}",
                    peer_id
                ))
            })?;
        let route_key = RouteKey::new(self.peer_id.clone(), peer_id.to_string());
        let (fail_publish, drop_delivery, duplicate_count, delay_ms) = {
            let mut faults = self.faults.lock().expect("fault mutex should lock");
            let fail_publish = decrement_fault(&mut faults.publish_failures, &route_key);
            let drop_delivery = decrement_fault(&mut faults.dropped_deliveries, &route_key);
            let duplicate_count =
                faults.duplicate_deliveries.remove(&route_key).unwrap_or_default();
            let delay_ms = faults.delayed_deliveries_ms.remove(&route_key).unwrap_or_default();
            (fail_publish, drop_delivery, duplicate_count, delay_ms)
        };
        if fail_publish {
            self.trace.record(&self.peer_id, peer_id, &payload, false);
            return Err(p2p_signaling::SignalingError::Protocol(format!(
                "injected publish failure from {} to {}",
                self.peer_id, peer_id
            )));
        }
        self.trace.record(&self.peer_id, peer_id, &payload, !drop_delivery);
        if delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms)).await;
        }
        if !drop_delivery {
            route.send(InMemoryEvent::Payload(payload.clone())).map_err(|_| {
                p2p_signaling::SignalingError::Protocol(format!(
                    "in-memory route for {} is closed",
                    peer_id
                ))
            })?;
            for _ in 0..duplicate_count {
                route.send(InMemoryEvent::Payload(payload.clone())).map_err(|_| {
                    p2p_signaling::SignalingError::Protocol(format!(
                        "in-memory duplicate route for {} is closed",
                        peer_id
                    ))
                })?;
            }
        }
        Ok(())
    }

    async fn poll_signal_payload(
        &mut self,
    ) -> Result<Option<Vec<u8>>, p2p_signaling::SignalingError> {
        match self.inbox.recv().await {
            Some(InMemoryEvent::Payload(payload)) => Ok(Some(payload)),
            Some(InMemoryEvent::PollFailure(error)) => {
                Err(p2p_signaling::SignalingError::Protocol(error))
            }
            None => Ok(None),
        }
    }
}

pub(crate) fn decrement_fault(faults: &mut HashMap<RouteKey, usize>, route_key: &RouteKey) -> bool {
    match faults.get_mut(route_key) {
        Some(remaining) if *remaining > 0 => {
            *remaining -= 1;
            if *remaining == 0 {
                faults.remove(route_key);
            }
            true
        }
        _ => false,
    }
}

pub(crate) struct InMemoryTransportMesh {
    routes: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<InMemoryEvent>>>>,
    faults: Arc<Mutex<TransportFaults>>,
    trace: TransportTrace,
}

impl InMemoryTransportMesh {
    pub(crate) fn new() -> Self {
        Self {
            routes: Arc::new(Mutex::new(HashMap::new())),
            faults: Arc::new(Mutex::new(TransportFaults::default())),
            trace: TransportTrace::default(),
        }
    }

    pub(crate) fn add_transport(&self, peer_id: &str) -> InMemoryTransport {
        let (tx, rx) = mpsc::unbounded_channel();
        self.routes.lock().expect("routes mutex should lock").insert(peer_id.to_owned(), tx);
        InMemoryTransport {
            peer_id: peer_id.to_owned(),
            inbox: rx,
            routes: Arc::clone(&self.routes),
            faults: Arc::clone(&self.faults),
            trace: self.trace.clone(),
        }
    }

    pub(crate) fn control(&self) -> TransportFaultControl {
        TransportFaultControl { faults: Arc::clone(&self.faults), routes: Arc::clone(&self.routes) }
    }

    pub(crate) fn trace(&self) -> TransportTrace {
        self.trace.clone()
    }
}

pub(crate) fn transport_pair(
    duplicate_answer_to_offer_payloads: usize,
    delay_first_answer_to_offer_ms: u64,
) -> (InMemoryTransport, InMemoryTransport, TransportTrace) {
    let mesh = InMemoryTransportMesh::new();
    let offer_transport = mesh.add_transport("offer-home");
    let answer_transport = mesh.add_transport("answer-office");
    let control = mesh.control();
    if duplicate_answer_to_offer_payloads > 0 {
        control.duplicate_next_delivery(
            "answer-office",
            "offer-home",
            duplicate_answer_to_offer_payloads,
        );
    }
    if delay_first_answer_to_offer_ms > 0 {
        control.delay_next_delivery("answer-office", "offer-home", delay_first_answer_to_offer_ms);
    }
    (offer_transport, answer_transport, mesh.trace())
}

pub(crate) fn transport_mesh(peer_ids: &[&str]) -> HashMap<String, InMemoryTransport> {
    let mesh = InMemoryTransportMesh::new();
    peer_ids.iter().map(|peer_id| ((*peer_id).to_owned(), mesh.add_transport(peer_id))).collect()
}

pub(crate) fn unique_path(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("p2ptunnel-{name}-{suffix}"))
}

pub(crate) fn sample_config(
    role: NodeRole,
    status_file: PathBuf,
    listen_port: u16,
    target_port: u16,
) -> AppConfig {
    let peer_id = match role {
        NodeRole::Offer => "offer-home",
        NodeRole::Answer => "answer-office",
    };
    sample_config_for(role, status_file, listen_port, target_port, peer_id, vec!["offer-home"])
}

pub(crate) fn sample_config_for(
    role: NodeRole,
    status_file: PathBuf,
    listen_port: u16,
    target_port: u16,
    peer_id: &str,
    allow_remote_peers: Vec<&str>,
) -> AppConfig {
    let peer_id: p2p_core::PeerId = peer_id.parse().expect("peer id");
    let client_id = peer_id.to_string();

    AppConfig {
        format: "p2ptunnel-config-v3".to_owned(),
        node: NodeConfig { peer_id, role },
        peer: Some(PeerConfig { remote_peer_id: "answer-office".parse().expect("answer peer id") }),
        paths: PathConfig {
            identity: PathBuf::from("/tmp/identity"),
            authorized_keys: PathBuf::from("/tmp/authorized_keys"),
            state_dir: PathBuf::from("/tmp/p2ptunnel-state"),
            log_dir: PathBuf::from("/tmp/p2ptunnel-log"),
        },
        broker: BrokerConfig {
            url: "mqtts://in-memory.invalid:8883".to_owned(),
            client_id,
            topic_prefix: "p2ptunnel-tests".to_owned(),
            username: String::new(),
            password_file: PathBuf::new(),
            qos: 1,
            keepalive_secs: 30,
            clean_session: false,
            connect_timeout_secs: 5,
            session_expiry_secs: 0,
            tls: BrokerTlsConfig {
                ca_file: PathBuf::from("/etc/ssl/certs/ca-certificates.crt"),
                client_cert_file: PathBuf::new(),
                client_key_file: PathBuf::new(),
                insecure_skip_verify: false,
            },
        },
        webrtc: WebRtcConfig {
            stun_urls: Vec::new(),
            enable_trickle_ice: false,
            enable_ice_restart: true,
        },
        tunnel: TunnelConfig {
            read_chunk_size: 16_384,
            local_eof_grace_ms: 250,
            remote_eof_grace_ms: 250,
        },
        forwards: vec![ForwardRule {
            id: "ssh".to_owned(),
            offer: Some(ForwardOfferConfig { listen_host: "127.0.0.1".to_owned(), listen_port }),
            answer: Some(ForwardAnswerConfig {
                target_host: "127.0.0.1".to_owned(),
                target_port,
                allow_remote_peers: allow_remote_peers
                    .into_iter()
                    .map(|peer_id| peer_id.parse().expect("allowed peer id"))
                    .collect(),
            }),
        }],
        reconnect: ReconnectConfig {
            enable_auto_reconnect: true,
            strategy: "ice_then_renegotiate".to_owned(),
            ice_restart_timeout_secs: 8,
            renegotiate_timeout_secs: 20,
            backoff_initial_ms: 1000,
            backoff_max_ms: 30_000,
            backoff_multiplier: 2.0,
            jitter_ratio: 0.20,
            max_attempts: 0,
            hold_local_client_during_reconnect: false,
            local_client_hold_secs: 0,
        },
        security: SecurityConfig {
            require_mqtt_tls: true,
            require_message_encryption: true,
            require_message_signatures: true,
            require_authorized_keys: true,
            max_clock_skew_secs: 120,
            max_message_age_secs: 300,
            replay_cache_size: 10_000,
            reject_unknown_config_keys: true,
            refuse_world_readable_identity: true,
            refuse_world_writable_paths: true,
        },
        logging: LoggingConfig {
            level: "info".to_owned(),
            format: "text".to_owned(),
            file_logging: false,
            stdout_logging: false,
            log_file: PathBuf::from("/tmp/p2ptunnel.log"),
            redact_secrets: true,
            redact_sdp: true,
            redact_candidates: true,
            log_rotation: "none".to_owned(),
        },
        health: HealthConfig {
            status_socket: PathBuf::new(),
            write_status_file: true,
            status_file,
        },
    }
}

pub(crate) fn authorized_keys_for(remote: &GeneratedIdentity) -> AuthorizedKeys {
    AuthorizedKeys::parse(&remote.public_identity.render()).expect("authorized keys should parse")
}

pub(crate) fn authorized_keys_for_many(remotes: &[&GeneratedIdentity]) -> AuthorizedKeys {
    let content = remotes
        .iter()
        .map(|identity| identity.public_identity.render())
        .collect::<Vec<_>>()
        .join("\n");
    AuthorizedKeys::parse(&content).expect("authorized keys should parse")
}

pub(crate) fn unused_local_port() -> u16 {
    static NEXT_TEST_PORT: AtomicU16 = AtomicU16::new(30_000);
    loop {
        let port = NEXT_TEST_PORT.fetch_add(1, Ordering::SeqCst);
        assert!(port < 60_000, "test port range exhausted");
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

pub(crate) async fn connect_with_retry(port: u16) -> TcpStream {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(stream) => return stream,
            Err(error) if tokio::time::Instant::now() < deadline => {
                let _ = error;
                sleep(Duration::from_millis(50)).await;
            }
            Err(error) => panic!("offer listener did not start in time: {error}"),
        }
    }
}

pub(crate) async fn assert_client_round_trip(
    port: u16,
    request: &'static [u8; 4],
    response: &'static [u8; 4],
) {
    let mut client = connect_with_retry(port).await;
    client.write_all(request).await.expect("client write");
    let mut received = [0_u8; 4];
    timeout(Duration::from_secs(10), client.read_exact(&mut received))
        .await
        .expect("client should receive response in time")
        .expect("client should read response");
    assert_eq!(&received, response);
    client.shutdown().await.expect("client shutdown");
}

pub(crate) async fn try_client_round_trip(
    port: u16,
    request: &[u8; 4],
    response: &[u8; 4],
) -> Result<(), String> {
    let mut client = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|error| format!("connect: {error}"))?;
    client.write_all(request).await.map_err(|error| format!("write: {error}"))?;
    let mut received = [0_u8; 4];
    timeout(Duration::from_secs(10), client.read_exact(&mut received))
        .await
        .map_err(|_| "read timeout".to_owned())?
        .map_err(|error| format!("read: {error}"))?;
    if received != *response {
        return Err(format!("response mismatch: got {received:?}, expected {response:?}"));
    }
    let _ = client.shutdown().await;
    Ok(())
}

pub(crate) async fn assert_client_round_trip_eventually(
    port: u16,
    request: [u8; 4],
    response: [u8; 4],
    description: &str,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        match try_client_round_trip(port, &request, &response).await {
            Ok(()) => return,
            Err(error) => {
                if tokio::time::Instant::now() >= deadline {
                    panic!("{description} did not complete in time; last error: {error}");
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

pub(crate) async fn assert_client_round_trip_owned(port: u16, request: [u8; 4], response: [u8; 4]) {
    let mut client = connect_with_retry(port).await;
    client.write_all(&request).await.expect("client write");
    let mut received = [0_u8; 4];
    timeout(Duration::from_secs(10), client.read_exact(&mut received))
        .await
        .expect("client should receive response in time")
        .expect("client should read response");
    assert_eq!(received, response);
    client.shutdown().await.expect("client shutdown");
}

pub(crate) async fn assert_client_stream_fails(port: u16, request: &'static [u8; 4]) {
    let mut client = connect_with_retry(port).await;
    client.write_all(request).await.expect("client write");
    let mut received = [0_u8; 4];
    let result = timeout(Duration::from_secs(5), client.read_exact(&mut received)).await;
    assert!(
        !matches!(result, Ok(Ok(_))),
        "denied stream unexpectedly returned bytes: {received:?}"
    );
}

pub(crate) async fn spawn_echo_target(
    expected_connections: usize,
) -> (u16, JoinHandle<()>, Arc<AtomicUsize>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("target listener should bind");
    let port = listener.local_addr().expect("target addr").port();
    let accepted = Arc::new(AtomicUsize::new(0));
    let accepted_for_task = Arc::clone(&accepted);
    let task = tokio::spawn(async move {
        for _ in 0..expected_connections {
            let (mut stream, _) = listener.accept().await.expect("target accept");
            let accepted_for_stream = Arc::clone(&accepted_for_task);
            tokio::spawn(async move {
                let mut request = [0_u8; 4];
                stream.read_exact(&mut request).await.expect("target read");
                stream.write_all(&request).await.expect("target write");
                stream.shutdown().await.expect("target shutdown");
                accepted_for_stream.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    (port, task, accepted)
}

pub(crate) async fn spawn_counting_echo_target() -> (u16, JoinHandle<()>, Arc<AtomicUsize>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("target listener should bind");
    let port = listener.local_addr().expect("target addr").port();
    let accepted = Arc::new(AtomicUsize::new(0));
    let accepted_for_task = Arc::clone(&accepted);
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.expect("target accept");
            let accepted_for_stream = Arc::clone(&accepted_for_task);
            tokio::spawn(async move {
                let mut request = [0_u8; 4];
                if stream.read_exact(&mut request).await.is_ok() {
                    let _ = stream.write_all(&request).await;
                    let _ = stream.shutdown().await;
                    accepted_for_stream.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    });
    (port, task, accepted)
}

pub(crate) fn add_offer_forward(
    config: &mut AppConfig,
    id: &str,
    listen_port: u16,
    target_port: u16,
) {
    config.forwards.push(ForwardRule {
        id: id.to_owned(),
        offer: Some(ForwardOfferConfig { listen_host: "127.0.0.1".to_owned(), listen_port }),
        answer: Some(ForwardAnswerConfig {
            target_host: "127.0.0.1".to_owned(),
            target_port,
            allow_remote_peers: vec![config.node.peer_id.clone()],
        }),
    });
}

pub(crate) fn add_answer_forward(
    config: &mut AppConfig,
    id: &str,
    target_port: u16,
    allow_remote_peer: &str,
) {
    config.forwards.push(ForwardRule {
        id: id.to_owned(),
        offer: Some(ForwardOfferConfig {
            listen_host: "127.0.0.1".to_owned(),
            listen_port: unused_local_port(),
        }),
        answer: Some(ForwardAnswerConfig {
            target_host: "127.0.0.1".to_owned(),
            target_port,
            allow_remote_peers: vec![allow_remote_peer.parse().expect("allowed peer id")],
        }),
    });
}

pub(crate) async fn wait_for_status(path: &Path, expected_state: &str) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json["current_state"] == expected_state {
                    return json;
                }
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "status {expected_state} not observed in time"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) async fn wait_for_session_count(
    path: &Path,
    expected_count: usize,
) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json["active_session_count"] == expected_count {
                    return json;
                }
            }
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "active_session_count {expected_count} not observed in time"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) async fn wait_for_status_matching(
    path: &Path,
    description: &str,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    wait_for_status_matching_with_timeout(path, description, predicate, Duration::from_secs(10))
        .await
}

pub(crate) async fn wait_for_status_matching_with_timeout(
    path: &Path,
    description: &str,
    predicate: impl Fn(&serde_json::Value) -> bool,
    timeout_duration: Duration,
) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + timeout_duration;
    loop {
        if let Ok(content) = tokio::fs::read_to_string(path).await
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && predicate(&json)
        {
            return json;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "status condition {description} not observed in time"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) async fn wait_for_mqtt_disconnected_after_poll_failure(
    control: &TransportFaultControl,
    peer_id: &str,
    path: &Path,
    description: &str,
    timeout_duration: Duration,
) -> serde_json::Value {
    let deadline = tokio::time::Instant::now() + timeout_duration;
    loop {
        control.inject_poll_failure(peer_id);
        if let Ok(content) = tokio::fs::read_to_string(path).await
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && mqtt_connected_is(false)(&json)
        {
            return json;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "status condition {description} not observed in time"
        );
        sleep(Duration::from_millis(100)).await;
    }
}

pub(crate) async fn wait_for_failed_publish_attempt(
    trace: &TransportTrace,
    from_peer_id: &str,
    to_peer_id: &str,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if trace.attempts().iter().any(|attempt| {
            attempt.from_peer_id == from_peer_id
                && attempt.to_peer_id == to_peer_id
                && !attempt.delivered
        }) {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "failed publish attempt from {from_peer_id} to {to_peer_id} not observed in time"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) fn session_count_is(expected_count: usize) -> impl Fn(&serde_json::Value) -> bool {
    move |status| status["active_session_count"] == expected_count
}

pub(crate) fn mqtt_connected_is(expected: bool) -> impl Fn(&serde_json::Value) -> bool {
    move |status| status["mqtt_connected"] == expected
}

pub(crate) fn has_remote_peer(remote_peer_id: &'static str) -> impl Fn(&serde_json::Value) -> bool {
    move |status| {
        status["sessions"].as_array().is_some_and(|sessions| {
            sessions.iter().any(|session| session["remote_peer_id"] == remote_peer_id)
        })
    }
}

pub(crate) fn lacks_remote_peer(
    remote_peer_id: &'static str,
) -> impl Fn(&serde_json::Value) -> bool {
    move |status| {
        status["sessions"].as_array().is_some_and(|sessions| {
            !sessions.iter().any(|session| session["remote_peer_id"] == remote_peer_id)
        })
    }
}

pub(crate) fn current_state_is(
    expected_state: &'static str,
) -> impl Fn(&serde_json::Value) -> bool {
    move |status| status["current_state"] == expected_state
}

pub(crate) fn configured_forwards_include(
    expected_forward_id: &'static str,
) -> impl Fn(&serde_json::Value) -> bool {
    move |status| {
        status["configured_forwards"]
            .as_array()
            .is_some_and(|forwards| forwards.iter().any(|forward| forward == expected_forward_id))
    }
}

pub(crate) fn assert_status_schema_is_consistent(status: &serde_json::Value) {
    let sessions = status["sessions"].as_array().expect("sessions should be an array");
    assert_eq!(status["active_session_count"], sessions.len());
    assert!(
        status.get("active_stream_count").is_none(),
        "status must not expose misleading active_stream_count"
    );
    assert!(
        status.get("open_forward_ids").is_none(),
        "status must not expose misleading open_forward_ids"
    );
    assert!(matches!(
        status["current_state"].as_str(),
        Some(
            "idle"
                | "listening"
                | "connecting_signaling"
                | "connecting_webrtc"
                | "connecting_data_channel"
                | "tunnel_open"
                | "serving"
                | "failed"
                | "closed"
        )
    ));
}

#[derive(Clone)]
pub(crate) struct DecodedSignalRecord {
    pub(crate) session_id: p2p_core::SessionId,
    pub(crate) sender_peer_id: p2p_core::PeerId,
    pub(crate) message_type: MessageType,
}

pub(crate) fn decode_signal_records(
    payloads: &[Vec<u8>],
    codec: &SignalCodec<'_>,
) -> Vec<DecodedSignalRecord> {
    payloads
        .iter()
        .map(|payload| {
            let mut replay_cache = ReplayCache::new(64);
            let (_envelope, message, _sender) = codec
                .decode(payload, &mut replay_cache, None)
                .expect("recorded signaling payload should decode");
            DecodedSignalRecord {
                session_id: message.session_id,
                sender_peer_id: message.sender_peer_id,
                message_type: message.message_type,
            }
        })
        .collect()
}

pub(crate) fn count_records_from(
    records: &[DecodedSignalRecord],
    sender_peer_id: &str,
    message_type: MessageType,
) -> usize {
    records
        .iter()
        .filter(|record| {
            record.sender_peer_id.as_str() == sender_peer_id && record.message_type == message_type
        })
        .count()
}

pub(crate) fn assert_answer_trace_is_passive(records: &[DecodedSignalRecord]) {
    assert!(
        !records.iter().any(|record| matches!(
            record.message_type,
            MessageType::Offer | MessageType::IceRestartRequest | MessageType::RenegotiateRequest
        )),
        "answer side must not initiate fresh-session or reconnect signaling"
    );
}

pub(crate) fn clone_identity(identity: &IdentityFile) -> IdentityFile {
    IdentityFile::from_toml(&identity.render_toml()).expect("identity clone should parse")
}

pub(crate) async fn run_one_in_memory_session(
    duplicate_answer_to_offer_payloads: usize,
    inject_offer_disconnect: bool,
    enable_ice_restart: bool,
    expect_success: bool,
) {
    let offer_identity = generate_identity("offer-home").expect("offer identity should build");
    let answer_identity = generate_identity("answer-office").expect("answer identity should build");
    let offer_keys = authorized_keys_for(&answer_identity);
    let answer_keys = authorized_keys_for(&offer_identity);
    let offer_codec = SignalCodec::new(&offer_identity.identity, &offer_keys, 120, 300);
    let answer_codec = SignalCodec::new(&answer_identity.identity, &answer_keys, 120, 300);
    let offer_identity_for_task = clone_identity(&offer_identity.identity);
    let answer_identity_for_task = clone_identity(&answer_identity.identity);
    let offer_keys_for_task = offer_keys.clone();
    let answer_keys_for_task = answer_keys.clone();

    let offer_status_path = unique_path("offer-status.json");
    let answer_status_path = unique_path("answer-status.json");
    let offer_port = unused_local_port();

    let target_listener =
        TcpListener::bind(("127.0.0.1", 0)).await.expect("target listener should bind");
    let target_port = target_listener.local_addr().expect("target local addr should exist").port();

    let mut offer_config =
        sample_config(NodeRole::Offer, offer_status_path.clone(), offer_port, target_port);
    let mut answer_config =
        sample_config(NodeRole::Answer, answer_status_path.clone(), offer_port, target_port);
    offer_config.webrtc.enable_ice_restart = enable_ice_restart;
    answer_config.webrtc.enable_ice_restart = enable_ice_restart;
    let (offer_transport, answer_transport, trace) = transport_pair(
        duplicate_answer_to_offer_payloads,
        if inject_offer_disconnect { 300 } else { 0 },
    );
    let (hook_tx, mut hook_rx) = mpsc::unbounded_channel();
    let mut injected_session_id = None;

    let answer_server = tokio::spawn(async move {
        let (mut stream, _) = target_listener.accept().await.expect("target accept should succeed");
        let mut received = [0_u8; 4];
        stream.read_exact(&mut received).await.expect("target should read request bytes");
        assert_eq!(&received, b"ping");
        stream.write_all(b"pong").await.expect("target should write response bytes");
        stream.shutdown().await.expect("target should shutdown cleanly");
    });

    let offer_task = tokio::spawn(run_offer_daemon_with_transport_and_test_hook(
        offer_config,
        offer_identity_for_task,
        offer_keys_for_task,
        offer_transport,
        Some(hook_tx),
    ));
    let answer_task = tokio::spawn(run_answer_daemon_with_transport(
        answer_config,
        answer_identity_for_task,
        answer_keys_for_task,
        answer_transport,
    ));

    let mut client = connect_with_retry(offer_port).await;
    if inject_offer_disconnect {
        let OfferSessionTestHandle { session_id, ice_state_injector } =
            timeout(Duration::from_secs(10), hook_rx.recv())
                .await
                .expect("offer session hook should arrive in time")
                .expect("offer session hook should contain a handle");
        injected_session_id = Some(session_id);
        ice_state_injector
            .inject(IceConnectionState::Disconnected)
            .await
            .expect("offer-side ice fault injection should succeed");
    }
    client.write_all(b"ping").await.expect("client should write request bytes");
    let mut response = [0_u8; 4];
    let client_result = timeout(Duration::from_secs(15), client.read_exact(&mut response)).await;

    if expect_success {
        let first_round_trip_succeeded = matches!(client_result, Ok(Ok(_))) && response == *b"pong";
        if first_round_trip_succeeded {
            client.shutdown().await.expect("client should shutdown cleanly");
        } else if inject_offer_disconnect && enable_ice_restart {
            assert_client_round_trip_eventually(
                offer_port,
                *b"ping",
                *b"pong",
                "offer-side reconnect should recover local client after injected ICE drop",
            )
            .await;
        } else {
            client_result
                .expect("client should receive tunnel response in time")
                .expect("client should read response bytes");
            assert_eq!(&response, b"pong");
            client.shutdown().await.expect("client should shutdown cleanly");
        }

        timeout(Duration::from_secs(15), answer_server)
            .await
            .expect("target server should finish in time")
            .expect("target server task should succeed");
    } else {
        let error = client_result
            .expect("client failure should arrive in time")
            .expect_err("client should not receive a successful tunnel response");
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
        answer_server.abort();
        let _ = answer_server.await;
    }

    let offer_status = wait_for_status(&offer_status_path, "tunnel_open").await;
    let answer_status = wait_for_status(&answer_status_path, "serving").await;
    assert_eq!(offer_status["current_state"], "tunnel_open");
    assert_eq!(offer_status["role"], "offer");
    assert_eq!(offer_status["mqtt_connected"], true);
    assert_eq!(answer_status["current_state"], "serving");
    assert_eq!(answer_status["role"], "answer");
    assert_eq!(answer_status["mqtt_connected"], true);

    if inject_offer_disconnect {
        let offer_to_answer =
            decode_signal_records(&trace.payloads_for("answer-office"), &answer_codec);
        let answer_to_offer =
            decode_signal_records(&trace.payloads_for("offer-home"), &offer_codec);
        assert!(
            offer_to_answer
                .iter()
                .filter(|record| record.message_type == MessageType::Offer)
                .count()
                >= 2,
            "offer side should publish a replacement offer after the injected disconnect"
        );
        assert!(
            !answer_to_offer.iter().any(|record| matches!(
                record.message_type,
                MessageType::Offer
                    | MessageType::IceRestartRequest
                    | MessageType::RenegotiateRequest
            )),
            "answer side must not initiate reconnect signaling"
        );
        if enable_ice_restart {
            assert!(
                offer_to_answer.iter().any(|record| {
                    record.message_type == MessageType::Offer
                        && Some(record.session_id) != injected_session_id
                }),
                "offer side should fall back to a replacement session when ICE fails before the data channel opens"
            );
        }
    }

    offer_task.abort();
    answer_task.abort();
    let _ = offer_task.await;
    let _ = answer_task.await;
    let _ = tokio::fs::remove_file(offer_status_path).await;
    let _ = tokio::fs::remove_file(answer_status_path).await;
}
