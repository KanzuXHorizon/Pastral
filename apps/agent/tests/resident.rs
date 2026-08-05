#![cfg(all(windows, feature = "ipc-health"))]

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{AgentCommand, run_command};
use pastral_domain::ClipEventId;
use pastral_ipc_core::{
    Capability, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto,
    HistoryPageRequestDto, RequestDto, ResponseDto, SearchRequestDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, TransportMaterial, client_handshake_with_capabilities, current_token_identity,
    derive_pipe_name, load_or_create_transport_material, open_pipe_client,
};

const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("pastral-resident-agent-{}", ClipEventId::new_v4()));
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
    material: &TransportMaterial,
    name: &pastral_ipc_win::PipeName,
    operation: RequestDto,
) -> ResponseDto {
    let deadline = Instant::now() + Duration::from_secs(10);
    let client = open_pipe_client(name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated =
        client_handshake_with_capabilities(stream, material, peer, &READ_CAPABILITIES, deadline)
            .unwrap();
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
    stream
        .write_frame(&Frame::new(header, body).unwrap(), deadline)
        .unwrap();
    let response = stream.read_frame(deadline).unwrap();
    assert_eq!(response.header().correlation(), correlation);
    decode_response(response.body()).unwrap()
}

#[test]
fn resident_supervises_clipboard_listener_and_authenticated_reads_until_bound() {
    let root = TestRoot::new();
    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let resident_root = root.path().to_path_buf();
    let resident = thread::spawn(move || {
        let mut output = Vec::new();
        let result = run_command(
            AgentCommand::Run {
                data_root: Some(resident_root),
                max_events: None,
                max_connections: Some(NonZeroUsize::new(3).unwrap()),
            },
            &mut output,
        );
        (result, output)
    });

    assert!(matches!(
        request(&material, &name, RequestDto::Health(HealthRequestDto)),
        ResponseDto::Health(_)
    ));
    match request(
        &material,
        &name,
        RequestDto::HistoryPage(HistoryPageRequestDto::new(50, None).unwrap()),
    ) {
        ResponseDto::HistoryPage(page) => {
            assert!(page.items().is_empty());
            assert!(!page.has_more());
        }
        _ => panic!("unexpected History response"),
    }
    match request(
        &material,
        &name,
        RequestDto::Search(SearchRequestDto::new("literal".to_owned(), 50).unwrap()),
    ) {
        ResponseDto::Search(page) => {
            assert!(page.items().is_empty());
            assert!(!page.has_more());
        }
        _ => panic!("unexpected Search response"),
    }

    let (result, output) = resident.join().unwrap();
    result.unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("agent-resident-ipc-ready=1"));
    assert!(output.contains("agent-resident-ipc-connections-served=3"));
    assert!(!output.contains(root.path().to_string_lossy().as_ref()));
    assert!(!output.contains("literal"));
    assert!(!output.contains("secret"));
}
