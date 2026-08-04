#![cfg(all(windows, feature = "ipc-health"))]

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
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
    HistoryPageRequestDto, RequestDto, ResponseDto, SearchRequestDto,
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
