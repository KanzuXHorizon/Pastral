#![cfg(all(windows, feature = "ipc-health"))]

use std::{
    fs,
    io::{BufRead, BufReader, Read},
    num::NonZeroUsize,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use pastral_agent::{
    DiagnosticStoragePolicy, HealthServerConfig, diagnostic_storage_limits, serve_read,
};
use pastral_domain::{
    CaptureOrder, ClipEvent, ClipEventId, ClipRepresentation, ClipRepresentationId,
    ClipboardFormatIdentity, Fidelity, ProfileId, ProtectionDomain, ProtectionDomainId, RawDigest,
    StandardFormatId, UtcUnixMicros,
};
use pastral_ipc_core::{
    Capability, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto,
    HistoryPageRequestDto, ProtocolErrorCode, RequestDto, ResponseDto, SearchRequestDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, TransportMaterial, client_handshake_with_capabilities, current_token_identity,
    derive_pipe_name, load_or_create_transport_material, open_pipe_client,
};
use pastral_storage::{ClipCommit, RepresentationPayload, SearchProjection, Storage};

const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("pastral-agent-read-ipc-{}", ClipEventId::new_v4()));
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

fn request(
    material: &TransportMaterial,
    name: &pastral_ipc_win::PipeName,
    operation: RequestDto,
) -> ResponseDto {
    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated =
        client_handshake_with_capabilities(stream, material, peer, &READ_CAPABILITIES, deadline)
            .unwrap();
    assert_eq!(authenticated.capabilities(), READ_CAPABILITIES);
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
fn malformed_authenticated_read_request_returns_content_free_protocol_error() {
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
        serve_read(config, &mut output).map(|summary| (summary, output))
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    let client = open_pipe_client(&name, deadline).unwrap();
    let peer = client.peer_identity().unwrap();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated =
        client_handshake_with_capabilities(stream, &material, peer, &READ_CAPABILITIES, deadline)
            .unwrap();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let malformed_body = vec![0xff, 0xff, 0xff];
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(malformed_body.len()).unwrap(),
        0,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    stream
        .write_frame(&Frame::new(header, malformed_body).unwrap(), deadline)
        .unwrap();
    let response = stream.read_frame(deadline).unwrap();
    assert_eq!(response.header().correlation(), correlation);
    match decode_response(response.body()).unwrap() {
        ResponseDto::Error(error) => {
            assert_eq!(error.code(), ProtocolErrorCode::InvalidRequest);
            assert!(!error.retryable());
            assert_eq!(error.developer_detail(), None);
        }
        _ => panic!("unexpected malformed-request response"),
    }

    let (summary, output) = server.join().unwrap().unwrap();
    assert_eq!(summary.connections_served(), 1);
    assert_eq!(
        output,
        b"agent-ipc-ready=1\nagent-ipc-connections-served=1\n"
    );
}

#[test]
fn authenticated_read_server_serves_health_history_and_literal_search() {
    let root = TestRoot::new();
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

    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let server_root = root.path().to_path_buf();
    let server = thread::spawn(move || {
        let config = HealthServerConfig::new(
            server_root,
            NonZeroUsize::new(3).unwrap(),
            Duration::from_secs(5),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut output = Vec::new();
        serve_read(config, &mut output).map(|summary| (summary, output))
    });

    match request(&material, &name, RequestDto::Health(HealthRequestDto)) {
        ResponseDto::Health(health) => {
            assert_eq!(health.storage_schema_version(), 1);
            assert!(health.privacy_policy_ok());
            assert!(health.storage_integrity_ok());
        }
        _ => panic!("unexpected Health response"),
    }

    match request(
        &material,
        &name,
        RequestDto::HistoryPage(HistoryPageRequestDto::new(2, None).unwrap()),
    ) {
        ResponseDto::HistoryPage(page) => {
            assert_eq!(page.items().len(), 2);
            assert!(page.has_more());
            assert_eq!(page.items()[0].event_id(), third);
            assert!(page.items()[0].unavailable());
            assert!(page.items()[0].preview().is_empty());
            assert_eq!(page.items()[1].event_id(), second);
            assert_eq!(page.items()[1].preview(), "alpha OR beta");
            assert_eq!(page.items()[1].source_label(), None);
            assert!(!page.items()[1].pinned());
        }
        _ => panic!("unexpected History response"),
    }

    match request(
        &material,
        &name,
        RequestDto::Search(SearchRequestDto::new("alpha OR".to_owned(), 10).unwrap()),
    ) {
        ResponseDto::Search(page) => {
            assert_eq!(page.items().len(), 1);
            assert!(!page.has_more());
            assert_eq!(page.items()[0].event_id(), second);
            assert_ne!(page.items()[0].event_id(), first);
            assert_eq!(page.items()[0].preview(), "alpha OR beta");
        }
        _ => panic!("unexpected Search response"),
    }

    let (summary, output) = server.join().unwrap().unwrap();
    assert_eq!(summary.connections_served(), 3);
    assert_eq!(summary.session_id(), current.session_id());
    assert_eq!(
        output,
        b"agent-ipc-ready=1\nagent-ipc-connections-served=3\n"
    );
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains("alpha"));
    assert!(!output.contains(root.path().to_string_lossy().as_ref()));
}

#[test]
fn serve_read_binary_runs_cross_process_and_exits_at_connection_bound() {
    let root = TestRoot::new();
    let mut storage = Storage::open(
        root.path().join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .unwrap();
    let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
    let event_id = commit_text(&mut storage, domain, 1, Some("cross process alpha"));
    drop(storage);

    let material = load_or_create_transport_material(root.path()).unwrap();
    let current = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), current.session_id()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pastral-agent-ipc"))
        .arg("serve-read")
        .arg("--data-root")
        .arg(root.path())
        .arg("--max-connections")
        .arg("3")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert_ne!(child.id(), std::process::id());

    let stdout = child.stdout.take().unwrap();
    let mut stdout = BufReader::new(stdout);
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready, "agent-ipc-ready=1\n");

    assert!(matches!(
        request(&material, &name, RequestDto::Health(HealthRequestDto)),
        ResponseDto::Health(_)
    ));
    match request(
        &material,
        &name,
        RequestDto::HistoryPage(HistoryPageRequestDto::new(10, None).unwrap()),
    ) {
        ResponseDto::HistoryPage(page) => {
            assert_eq!(page.items().len(), 1);
            assert_eq!(page.items()[0].event_id(), event_id);
        }
        _ => panic!("unexpected cross-process History response"),
    }
    match request(
        &material,
        &name,
        RequestDto::Search(SearchRequestDto::new("alpha".to_owned(), 10).unwrap()),
    ) {
        ResponseDto::Search(page) => {
            assert_eq!(page.items().len(), 1);
            assert_eq!(page.items()[0].event_id(), event_id);
        }
        _ => panic!("unexpected cross-process Search response"),
    }

    let mut summary = String::new();
    stdout.read_to_string(&mut summary).unwrap();
    assert_eq!(summary, "agent-ipc-connections-served=3\n");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(stderr.is_empty());
}

#[test]
fn large_history_page_trims_previews_to_the_control_frame_budget() {
    let root = TestRoot::new();
    let mut storage = Storage::open(
        root.path().join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .unwrap();
    let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
    let long_preview = "é".repeat(3000);
    for order in 1..=101 {
        commit_text(&mut storage, domain, order, Some(&long_preview));
    }
    drop(storage);

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
        serve_read(config, &mut output).map(|summary| (summary, output))
    });

    match request(
        &material,
        &name,
        RequestDto::HistoryPage(HistoryPageRequestDto::new(100, None).unwrap()),
    ) {
        ResponseDto::HistoryPage(page) => {
            assert_eq!(page.items().len(), 100);
            assert!(page.has_more());
            assert!(page.items().iter().all(|item| item.preview().len() <= 4096));
            assert!(page.items().iter().any(|item| item.preview().len() < 4096));
            assert!(
                page.items()
                    .iter()
                    .all(|item| item.preview().is_char_boundary(item.preview().len()))
            );
        }
        _ => panic!("unexpected large History response"),
    }

    let (summary, output) = server.join().unwrap().unwrap();
    assert_eq!(summary.connections_served(), 1);
    assert_eq!(
        output,
        b"agent-ipc-ready=1\nagent-ipc-connections-served=1\n"
    );
}
