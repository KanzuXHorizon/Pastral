use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    AUTH_PROOF_BYTES, BulkEndDto, Capability, ClientHelloDto, ClipPreviewDto, ClipPreviewKind,
    CorrelationId, HealthRequestDto, HealthResponseDto, HistoryPageRequestDto,
    HistoryPageResponseDto, ProtocolErrorCode, ProtocolErrorDto, RequestDto, ResponseDto,
    SearchRequestDto, SearchResponseDto, ServerAcceptedDto, ServerHelloDto,
};
use pastral_ipc_schema::{
    PROTOBUF_RELEASE, decode_bulk_end, decode_client_hello, decode_protocol_error, decode_request,
    decode_response, decode_server_accepted, decode_server_hello, encode_bulk_end,
    encode_client_hello, encode_protocol_error, encode_request, encode_response,
    encode_server_accepted, encode_server_hello, schema_sha256,
};

fn preview(index: u64, kind: ClipPreviewKind) -> ClipPreviewDto {
    let unavailable = kind == ClipPreviewKind::Unavailable;
    ClipPreviewDto::new(
        ClipEventId::new_v4(),
        CaptureOrder::new(index + 1).unwrap(),
        UtcUnixMicros::new(1_700_000_000_000_000 + i64::try_from(index).unwrap()).unwrap(),
        kind,
        if unavailable {
            String::new()
        } else {
            format!("preview-{index}")
        },
        Some("notepad.exe".to_owned()),
        index.is_multiple_of(2),
        unavailable,
    )
    .unwrap()
}

#[test]
fn exact_runtime_marker_and_schema_digest_are_stable_shapes() {
    assert_eq!(PROTOBUF_RELEASE, "4.35.0-release");
    let digest = schema_sha256();
    assert_ne!(digest, [0; 32]);
    assert_eq!(digest, schema_sha256());
}

#[test]
fn server_and_client_hello_round_trip_through_generated_wire_types() {
    let server = ServerHelloDto::new(
        1,
        0,
        2,
        [0x11; 32],
        CorrelationId::new_v4(),
        [
            Capability::Health,
            Capability::HistoryPage,
            Capability::Search,
        ],
    )
    .unwrap();
    let decoded_server = decode_server_hello(&encode_server_hello(&server).unwrap()).unwrap();
    assert!(decoded_server == server);

    let client = ClientHelloDto::new(
        1,
        1,
        2,
        [0x22; 32],
        [0x11; 32],
        [Capability::Health, Capability::Search],
        [0x33; AUTH_PROOF_BYTES],
    )
    .unwrap();
    let decoded_client = decode_client_hello(&encode_client_hello(&client).unwrap()).unwrap();
    assert!(decoded_client == client);

    let accepted = ServerAcceptedDto::new(
        1,
        [Capability::Search, Capability::Health],
        [0x44; AUTH_PROOF_BYTES],
    )
    .unwrap();
    let decoded_accepted =
        decode_server_accepted(&encode_server_accepted(&accepted).unwrap()).unwrap();
    assert!(decoded_accepted == accepted);
}

#[test]
fn every_control_request_round_trips() {
    let requests = [
        RequestDto::Health(HealthRequestDto),
        RequestDto::HistoryPage(
            HistoryPageRequestDto::new(100, Some(CaptureOrder::new(42).unwrap())).unwrap(),
        ),
        RequestDto::Search(SearchRequestDto::new("alpha beta".to_owned(), 25).unwrap()),
    ];

    for request in requests {
        let decoded = decode_request(&encode_request(&request).unwrap()).unwrap();
        assert!(decoded == request);
    }
}

#[test]
fn every_control_response_round_trips() {
    let items = vec![
        preview(0, ClipPreviewKind::Text),
        preview(1, ClipPreviewKind::Code),
        preview(2, ClipPreviewKind::Link),
        preview(3, ClipPreviewKind::Unavailable),
    ];
    let responses = [
        ResponseDto::Health(HealthResponseDto::new(1, true, true, true).unwrap()),
        ResponseDto::HistoryPage(HistoryPageResponseDto::new(items.clone(), true).unwrap()),
        ResponseDto::Search(SearchResponseDto::new(items, false).unwrap()),
        ResponseDto::Error(
            ProtocolErrorDto::new(
                ProtocolErrorCode::InvalidRequest,
                false,
                Some("bounded developer detail".to_owned()),
            )
            .unwrap(),
        ),
    ];

    for response in responses {
        let decoded = decode_response(&encode_response(&response).unwrap()).unwrap();
        assert!(decoded == response);
    }
}

#[test]
fn standalone_error_and_bulk_end_round_trip() {
    let error = ProtocolErrorDto::new(ProtocolErrorCode::ResourceLimit, true, None).unwrap();
    let decoded_error = decode_protocol_error(&encode_protocol_error(&error).unwrap()).unwrap();
    assert!(decoded_error == error);

    let end = BulkEndDto::new(1024, 4).unwrap();
    let decoded_end = decode_bulk_end(&encode_bulk_end(end).unwrap()).unwrap();
    assert_eq!(decoded_end, end);
}
