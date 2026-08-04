#![cfg(all(windows, feature = "ipc-health"))]

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{AgentIpcError, HealthServerConfig, load_health_snapshot, serve_health};
use pastral_domain::ClipEventId;
use pastral_ipc_core::{
    CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto,
    HistoryPageRequestDto, RequestDto, ResponseDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, build_logon_sid_pipe_security, client_handshake, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material, open_pipe_client,
};

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-shared-ipc-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn request(
    material: &pastral_ipc_win::TransportMaterial,
    name: &pastral_ipc_win::PipeName,
    operation: RequestDto,
) -> Result<ResponseDto, pastral_ipc_win::TransportError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(name, deadline)?;
    let peer = client.peer_identity()?;
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake(stream, material, peer, deadline)?;
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&operation).unwrap();
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).unwrap(),
        0,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    stream.write_frame(&Frame::new(header, body).unwrap(), deadline)?;
    let response = stream.read_frame(deadline)?;
    assert_eq!(response.header().correlation(), correlation);
    Ok(decode_response(response.body()).unwrap())
}

#[test]
fn serves_two_real_health_requests_and_exits_at_bound() {
    let root = TestRoot::new();
    let expected = load_health_snapshot(root.path()).unwrap();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let server_root = root.path().to_path_buf();

    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::new(2).unwrap(),
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut output = Vec::new();
        serve_health(config, &mut output).map(|summary| (summary, output))
    });

    for _ in 0..2 {
        match request(&material, &name, RequestDto::Health(HealthRequestDto)).unwrap() {
            ResponseDto::Health(value) => {
                assert_eq!(
                    value.storage_schema_version(),
                    expected.storage_schema_version()
                );
                assert_eq!(value.capture_enabled(), expected.capture_enabled());
                assert_eq!(value.privacy_policy_ok(), expected.privacy_policy_ok());
                assert_eq!(
                    value.storage_integrity_ok(),
                    expected.storage_integrity_ok()
                );
            }
            _ => panic!("unexpected response variant"),
        }
    }

    let (summary, output) = server.join().unwrap().unwrap();
    assert_eq!(summary.connections_served(), 2);
    assert_eq!(summary.session_id(), current.session_id());
    assert_eq!(
        output,
        b"agent-ipc-ready=1\nagent-ipc-connections-served=2\n"
    );
}

#[test]
fn first_instance_collision_fails_before_readiness() {
    let root = TestRoot::new();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&current).unwrap();
    let blocker = create_first_pipe_server(&name, &security).unwrap();
    let config = HealthServerConfig::new(
        root.path().to_path_buf(),
        NonZeroUsize::MIN,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    let mut output = Vec::new();

    assert_eq!(
        serve_health(config, &mut output),
        Err(AgentIpcError::Transport)
    );
    assert!(output.is_empty());
    drop(blocker);
}

#[test]
fn authenticated_non_health_request_is_rejected() {
    let root = TestRoot::new();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::MIN,
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut output = Vec::new();
        (serve_health(config, &mut output), output)
    });

    let result = request(
        &material,
        &name,
        RequestDto::HistoryPage(HistoryPageRequestDto::new(1, None).unwrap()),
    );
    assert!(result.is_err());

    let (server_result, output) = server.join().unwrap();
    assert_eq!(server_result, Err(AgentIpcError::Protocol));
    assert_eq!(output, b"agent-ipc-ready=1\n");
}
