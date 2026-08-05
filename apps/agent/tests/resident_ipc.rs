#![cfg(all(windows, feature = "ipc-health"))]

use std::{
    fs,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{ResidentReadServerConfig, serve_read_until_stopped};
use pastral_domain::ClipEventId;
use pastral_ipc_core::{
    Capability, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto,
    RequestDto, ResponseDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, client_handshake_with_capabilities, current_token_identity, derive_pipe_name,
    load_or_create_transport_material, open_pipe_client,
};

const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

#[test]
fn stopped_resident_server_creates_nothing() {
    let root = std::env::temp_dir().join(format!(
        "pastral-resident-stopped-{}",
        ClipEventId::new_v4()
    ));
    let stop = Arc::new(AtomicBool::new(true));
    let config = ResidentReadServerConfig::new(
        root.clone(),
        Duration::from_millis(50),
        Duration::from_secs(1),
        None,
    )
    .unwrap();
    let mut output = Vec::new();
    let summary = serve_read_until_stopped(config, stop, &mut output).unwrap();
    assert_eq!(summary.connections_served(), 0);
    assert!(output.is_empty());
    assert!(!root.exists());
}

#[test]
fn rejected_client_is_contained_before_valid_health_and_bounded_shutdown() {
    let root = std::env::temp_dir().join(format!("pastral-resident-ipc-{}", ClipEventId::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let material = load_or_create_transport_material(&root).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = Arc::clone(&stop);
    let server_root = root.clone();
    let server = thread::spawn(move || {
        let config = ResidentReadServerConfig::new(
            server_root,
            Duration::from_millis(250),
            Duration::from_secs(2),
            Some(NonZeroUsize::MIN),
        )
        .unwrap();
        let mut output = Vec::new();
        serve_read_until_stopped(config, server_stop, &mut output).map(|summary| (summary, output))
    });

    thread::sleep(Duration::from_millis(750));
    let rejected = open_pipe_client(&name, Instant::now() + Duration::from_secs(5)).unwrap();
    drop(rejected);

    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(&name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated =
        client_handshake_with_capabilities(stream, &material, peer, &READ_CAPABILITIES, deadline)
            .unwrap();
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
    assert!(matches!(
        decode_response(response.body()).unwrap(),
        ResponseDto::Health(_)
    ));

    let (summary, output) = server.join().unwrap().unwrap();
    assert_eq!(summary.connections_served(), 1);
    assert!(stop.load(Ordering::Acquire));
    assert_eq!(
        output,
        b"agent-resident-ipc-ready=1\nagent-ipc-client-rejected=1\nagent-resident-ipc-connections-served=1\n"
    );
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains(root.to_string_lossy().as_ref()));
    assert!(!output.contains("secret"));
    fs::remove_dir_all(root).unwrap();
}
