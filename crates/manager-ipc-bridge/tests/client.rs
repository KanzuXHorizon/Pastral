#![cfg(windows)]

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{
    DiagnosticStoragePolicy, HealthServerConfig, diagnostic_storage_limits, serve_health,
    serve_read,
};
use pastral_domain::{
    CaptureOrder, ClipEvent, ClipEventId, ClipRepresentation, ClipRepresentationId,
    ClipboardFormatIdentity, Fidelity, ProfileId, ProtectionDomain, ProtectionDomainId, RawDigest,
    StandardFormatId, UtcUnixMicros,
};
use pastral_ipc_auth::{InstallationSecret, NonceReplayCache};
use pastral_ipc_core::{
    Frame, FrameHeader, FrameKind, FrameLimits, HealthResponseDto, ProtocolErrorCode,
    ProtocolErrorDto, ResponseDto,
};
use pastral_ipc_schema::encode_response;
use pastral_ipc_win::{
    PipeFrameStream, SECRET_FILE_NAME, build_logon_sid_pipe_security, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material,
    load_transport_material, protect_installation_secret, server_handshake,
};
use pastral_manager_ipc_bridge::{
    ManagerClipKind, ManagerHealthStatus, query_health, query_history, query_search,
};
use pastral_storage::{ClipCommit, RepresentationPayload, SearchProjection, Storage};

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-manager-bridge-{label}-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn missing(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "pastral-manager-bridge-{label}-{}",
            ClipEventId::new_v4()
        ))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn commit_text(
    storage: &mut Storage<DiagnosticStoragePolicy>,
    domain: ProtectionDomain,
    order: u64,
    projection: Option<&str>,
) -> ClipEventId {
    let bytes = format!("payload-{order}").into_bytes();
    let digest = RawDigest::sha256_raw_v1(domain, &bytes).unwrap();
    let representation = ClipRepresentation::new(
        ClipRepresentationId::new_v4(),
        ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
        domain,
        bytes.len() as u64,
        Some(digest),
        Fidelity::FullFidelity,
    )
    .unwrap();
    let event_id = ClipEventId::new_v4();
    let event = ClipEvent::new(
        event_id,
        UtcUnixMicros::new(1_700_000_000_000_000 + order as i64).unwrap(),
        CaptureOrder::new(order).unwrap(),
        ProfileId::new_v4(),
        domain,
        vec![representation.clone()],
    )
    .unwrap();
    let payload = RepresentationPayload::new(representation.id(), bytes);
    let projection = projection
        .map(|value| SearchProjection::new(value.to_owned(), diagnostic_storage_limits()).unwrap());
    storage
        .commit_clip(ClipCommit::new(event, vec![payload], projection))
        .unwrap();
    event_id
}

#[test]
fn read_client_returns_authenticated_history_and_literal_search() {
    let root = TestRoot::new("read");
    let mut storage = Storage::open(
        root.path().join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .unwrap();
    let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
    let first = commit_text(&mut storage, domain, 1, Some("alpha beta"));
    let second = commit_text(&mut storage, domain, 2, Some("alpha OR beta"));
    let third = commit_text(&mut storage, domain, 3, None);
    drop(storage);

    let _material = load_or_create_transport_material(root.path()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::new(2).unwrap(),
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap()
        .without_summary();
        serve_read(config, &mut Vec::new()).unwrap();
    });

    let history = query_history(root.path(), Duration::from_secs(2), 2, None).unwrap();
    assert_eq!(history.items().len(), 2);
    assert!(history.has_more());
    assert_eq!(history.items()[0].event_id(), third);
    assert_eq!(history.items()[0].capture_order().get(), 3);
    assert_eq!(history.items()[0].kind(), ManagerClipKind::Unavailable);
    assert!(history.items()[0].unavailable());
    assert_eq!(history.items()[0].preview(), "");
    assert_eq!(history.items()[1].event_id(), second);
    assert_eq!(history.items()[1].preview(), "alpha OR beta");
    assert_ne!(history.server_process_id(), 0);
    assert_eq!(
        history.session_id(),
        current_token_identity().unwrap().session_id()
    );

    let search = query_search(root.path(), Duration::from_secs(2), "alpha OR", 10).unwrap();
    assert_eq!(search.items().len(), 1);
    assert!(!search.has_more());
    assert_eq!(search.items()[0].event_id(), second);
    assert_ne!(search.items()[0].event_id(), first);
    assert_eq!(search.items()[0].kind(), ManagerClipKind::Text);
    assert!(!search.items()[0].pinned());
    assert_eq!(search.items()[0].source_label(), None);
    assert!(search.request_elapsed() <= Duration::from_secs(2));
    server.join().unwrap();
}

#[test]
fn connects_to_real_agent_health_without_opening_storage_directly() {
    let root = TestRoot::new("connected");
    let _material = load_or_create_transport_material(root.path()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::MIN,
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap()
        .without_summary();
        serve_health(config, &mut Vec::new()).unwrap();
    });

    let result = query_health(root.path(), Duration::from_secs(2));
    assert_eq!(result.status(), ManagerHealthStatus::Connected);
    assert_eq!(result.storage_schema_version(), 1);
    assert!(!result.capture_enabled());
    assert!(result.privacy_policy_ok());
    assert!(result.storage_integrity_ok());
    assert_ne!(result.server_process_id(), 0);
    assert_eq!(
        result.session_id(),
        current_token_identity().unwrap().session_id()
    );
    assert!(result.connect_elapsed() <= Duration::from_secs(2));
    assert!(result.handshake_elapsed() <= Duration::from_secs(2));
    assert!(result.health_elapsed() <= Duration::from_secs(2));
    server.join().unwrap();
}

#[test]
fn missing_material_is_disconnected_and_never_creates_root() {
    let root = TestRoot::missing("missing");
    assert!(!root.exists());

    let result = query_health(&root, Duration::from_millis(100));
    assert_eq!(result.status(), ManagerHealthStatus::Disconnected);
    assert!(!root.exists());
}

#[test]
fn protocol_error_response_maps_to_protocol_mismatch() {
    let root = TestRoot::new("protocol");
    let response = ResponseDto::Error(
        ProtocolErrorDto::new(
            ProtocolErrorCode::UnsupportedVersion,
            false,
            Some("fixture".to_owned()),
        )
        .unwrap(),
    );
    let server = spawn_custom_response(root.path(), response);

    let result = query_health(root.path(), Duration::from_secs(2));
    assert_eq!(result.status(), ManagerHealthStatus::ProtocolMismatch);
    server.join().unwrap();
}

#[test]
fn wrong_installation_secret_maps_to_authentication_failed() {
    let root = TestRoot::new("wrong-secret");
    let server_material = load_or_create_transport_material(root.path()).unwrap();
    let replacement =
        protect_installation_secret(&InstallationSecret::from_bytes([7; 32])).unwrap();
    fs::write(root.path().join(SECRET_FILE_NAME), replacement).unwrap();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let server = thread::spawn(move || {
        let current = current_token_identity().unwrap();
        let name = derive_pipe_name(server_material.identity(), current.session_id()).unwrap();
        let security = build_logon_sid_pipe_security(&current).unwrap();
        let mut pipe = create_first_pipe_server(&name, &security).unwrap();
        ready_sender.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        pipe.connect(deadline).unwrap();
        let peer = pipe.peer_identity().unwrap();
        let stream = PipeFrameStream::from_server(pipe, FrameLimits::default());
        let mut replay = NonceReplayCache::new(8).unwrap();
        assert!(server_handshake(stream, &server_material, peer, &mut replay, deadline).is_err());
    });
    ready_receiver.recv_timeout(Duration::from_secs(2)).unwrap();

    let result = query_health(root.path(), Duration::from_secs(2));
    assert_eq!(result.status(), ManagerHealthStatus::AuthenticationFailed);
    server.join().unwrap();
}

#[test]
fn silent_authenticated_server_maps_to_timeout() {
    let root = TestRoot::new("timeout");
    let server = spawn_silent_server(root.path());

    let result = query_health(root.path(), Duration::from_millis(150));
    assert_eq!(result.status(), ManagerHealthStatus::Timeout);
    server.join().unwrap();
}

#[test]
fn unhealthy_health_response_maps_to_unhealthy() {
    let root = TestRoot::new("unhealthy");
    let response = ResponseDto::Health(HealthResponseDto::new(1, false, false, false).unwrap());
    let server = spawn_custom_response(root.path(), response);

    let result = query_health(root.path(), Duration::from_secs(2));
    assert_eq!(result.status(), ManagerHealthStatus::Unhealthy);
    server.join().unwrap();
}

fn spawn_silent_server(root: &std::path::Path) -> thread::JoinHandle<()> {
    let _created = load_or_create_transport_material(root).unwrap();
    let server_root = root.to_path_buf();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let handle = thread::spawn(move || {
        let material = load_transport_material(&server_root).unwrap();
        let current = current_token_identity().unwrap();
        let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
        let security = build_logon_sid_pipe_security(&current).unwrap();
        let mut server = create_first_pipe_server(&name, &security).unwrap();
        ready_sender.send(()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        server.connect(deadline).unwrap();
        let peer = server.peer_identity().unwrap();
        let stream = PipeFrameStream::from_server(server, FrameLimits::default());
        let mut replay = NonceReplayCache::new(8).unwrap();
        let authenticated =
            server_handshake(stream, &material, peer, &mut replay, deadline).unwrap();
        let mut stream = authenticated.into_stream();
        let _request = stream.read_frame(deadline).unwrap();
        thread::sleep(Duration::from_millis(300));
    });
    ready_receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    handle
}

fn spawn_custom_response(root: &std::path::Path, response: ResponseDto) -> thread::JoinHandle<()> {
    let _created = load_or_create_transport_material(root).unwrap();
    let server_root = root.to_path_buf();
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let handle = thread::spawn(move || {
        let material = load_transport_material(&server_root).unwrap();
        let current = current_token_identity().unwrap();
        let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
        let security = build_logon_sid_pipe_security(&current).unwrap();
        let mut server = create_first_pipe_server(&name, &security).unwrap();
        ready_sender.send(()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        server.connect(deadline).unwrap();
        let peer = server.peer_identity().unwrap();
        let stream = PipeFrameStream::from_server(server, FrameLimits::default());
        let mut replay = NonceReplayCache::new(8).unwrap();
        let authenticated =
            server_handshake(stream, &material, peer, &mut replay, deadline).unwrap();
        let mut stream = authenticated.into_stream();
        let request = stream.read_frame(deadline).unwrap();
        let body = encode_response(&response).unwrap();
        let header = FrameHeader::new(
            FrameKind::ControlProto,
            u32::try_from(body.len()).unwrap(),
            0,
            request.header().correlation(),
            FrameLimits::default(),
        )
        .unwrap();
        stream
            .write_frame(&Frame::new(header, body).unwrap(), deadline)
            .unwrap();
    });
    ready_receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    handle
}
