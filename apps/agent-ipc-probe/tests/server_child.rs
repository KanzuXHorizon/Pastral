#![cfg(windows)]

use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use pastral_agent::load_health_snapshot;
use pastral_agent_ipc_probe::{AdmissionError, run_server_child};
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
            "pastral-agent-ipc-server-{}",
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

#[test]
fn server_child_returns_real_agent_health_once_and_exits() {
    let root = TestRoot::new();
    let expected = load_health_snapshot(root.path()).unwrap();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let child_root = root.path().to_path_buf();

    let server = thread::spawn(move || {
        let mut output = Vec::new();
        run_server_child(&child_root, &mut output).map(|()| output)
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(&name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake(stream, &material, peer, deadline).unwrap();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&RequestDto::Health(HealthRequestDto)).unwrap();
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).unwrap(),
        0,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    stream
        .write_frame(&Frame::new(header, body).unwrap(), deadline)
        .unwrap();
    let response = stream.read_frame(deadline).unwrap();
    assert_eq!(response.header().correlation(), correlation);
    match decode_response(response.body()).unwrap() {
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
        _ => panic!("unexpected Health response variant"),
    }

    assert_eq!(server.join().unwrap().unwrap(), b"agent-ipc-ready=1\n");
}

#[test]
fn first_instance_collision_fails_before_readiness() {
    let root = TestRoot::new();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&current).unwrap();
    let blocker = create_first_pipe_server(&name, &security).unwrap();
    let mut output = Vec::new();

    assert_eq!(
        run_server_child(root.path(), &mut output),
        Err(AdmissionError::Transport)
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
    let child_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let mut output = Vec::new();
        let result = run_server_child(&child_root, &mut output);
        (result, output)
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(&name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake(stream, &material, peer, deadline).unwrap();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&RequestDto::HistoryPage(
        HistoryPageRequestDto::new(1, None).unwrap(),
    ))
    .unwrap();
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).unwrap(),
        0,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    stream
        .write_frame(&Frame::new(header, body).unwrap(), deadline)
        .unwrap();
    drop(stream);

    let (result, output) = server.join().unwrap();
    assert_eq!(result, Err(AdmissionError::Protocol));
    assert_eq!(output, b"agent-ipc-ready=1\n");
}
