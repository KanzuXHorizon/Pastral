#![cfg(windows)]

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{HealthServerConfig, serve_health};
use pastral_domain::ClipEventId;
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
use pastral_manager_ipc_bridge::{ManagerHealthStatus, query_health};

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
