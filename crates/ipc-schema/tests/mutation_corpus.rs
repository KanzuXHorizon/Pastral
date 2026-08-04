use std::panic::AssertUnwindSafe;

use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    AUTH_PROOF_BYTES, BulkEndDto, Capability, ClientHelloDto, ClipPreviewDto, ClipPreviewKind,
    CorrelationId, FrameLimits, HealthRequestDto, HealthResponseDto, HistoryPageRequestDto,
    HistoryPageResponseDto, IpcError, ProtocolErrorCode, ProtocolErrorDto, RequestDto, ResponseDto,
    SearchRequestDto, SearchResponseDto, ServerAcceptedDto, ServerHelloDto,
};
use pastral_ipc_schema::{
    decode_bulk_end, decode_client_hello, decode_protocol_error, decode_request, decode_response,
    decode_server_accepted, decode_server_hello, encode_bulk_end, encode_client_hello,
    encode_protocol_error, encode_request, encode_response, encode_server_accepted,
    encode_server_hello,
};

const UNKNOWN_VARINT_FIELD: [u8; 3] = [0xf8, 0x07, 0x01];
const UNKNOWN_BYTES_FIELD: [u8; 6] = [0xfa, 0x07, 0x03, b'x', b'y', b'z'];
const TRUNCATED_UNKNOWN_BYTES_FIELD: [u8; 4] = [0xfa, 0x07, 0x05, b'x'];

#[test]
fn every_control_decoder_is_panic_free_under_deterministic_mutations() {
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
    assert_mutation_hardened(
        encode_server_hello(&server).unwrap(),
        server,
        decode_server_hello,
    );

    let client = ClientHelloDto::new(
        1,
        0,
        2,
        [0x22; 32],
        [0x11; 32],
        [Capability::Health, Capability::Search],
        [0x33; AUTH_PROOF_BYTES],
    )
    .unwrap();
    assert_mutation_hardened(
        encode_client_hello(&client).unwrap(),
        client,
        decode_client_hello,
    );

    let accepted = ServerAcceptedDto::new(
        1,
        [Capability::Health, Capability::Search],
        [0x44; AUTH_PROOF_BYTES],
    )
    .unwrap();
    assert_mutation_hardened(
        encode_server_accepted(&accepted).unwrap(),
        accepted,
        decode_server_accepted,
    );

    for request in request_corpus() {
        assert_mutation_hardened(encode_request(&request).unwrap(), request, decode_request);
    }
    for response in response_corpus() {
        assert_mutation_hardened(
            encode_response(&response).unwrap(),
            response,
            decode_response,
        );
    }

    let error = ProtocolErrorDto::new(
        ProtocolErrorCode::ResourceLimit,
        true,
        Some("bounded detail".to_owned()),
    )
    .unwrap();
    assert_mutation_hardened(
        encode_protocol_error(&error).unwrap(),
        error,
        decode_protocol_error,
    );

    let bulk_end = BulkEndDto::new(4096, 4).unwrap();
    assert_mutation_hardened(
        encode_bulk_end(bulk_end).unwrap(),
        bulk_end,
        decode_bulk_end,
    );
}

fn request_corpus() -> Vec<RequestDto> {
    vec![
        RequestDto::Health(HealthRequestDto),
        RequestDto::HistoryPage(HistoryPageRequestDto::new(100, None).unwrap()),
        RequestDto::HistoryPage(
            HistoryPageRequestDto::new(25, Some(CaptureOrder::new(42).unwrap())).unwrap(),
        ),
        RequestDto::Search(SearchRequestDto::new("alpha OR beta".to_owned(), 50).unwrap()),
    ]
}

fn response_corpus() -> Vec<ResponseDto> {
    let items = vec![
        preview(0, ClipPreviewKind::Text),
        preview(1, ClipPreviewKind::Code),
        preview(2, ClipPreviewKind::Link),
        preview(3, ClipPreviewKind::Unavailable),
    ];
    vec![
        ResponseDto::Health(HealthResponseDto::new(1, true, true, true).unwrap()),
        ResponseDto::HistoryPage(HistoryPageResponseDto::new(items.clone(), true).unwrap()),
        ResponseDto::Search(SearchResponseDto::new(items, false).unwrap()),
        ResponseDto::Error(
            ProtocolErrorDto::new(ProtocolErrorCode::InvalidRequest, false, None).unwrap(),
        ),
    ]
}

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
        Some("source.exe".to_owned()),
        index.is_multiple_of(2),
        unavailable,
    )
    .unwrap()
}

fn assert_mutation_hardened<T, F>(encoded: Vec<u8>, expected: T, decode: F)
where
    T: PartialEq,
    F: Fn(&[u8]) -> Result<T, IpcError> + Copy,
{
    assert!(decode(&encoded).unwrap() == expected);

    for extension in [
        UNKNOWN_VARINT_FIELD.as_slice(),
        UNKNOWN_BYTES_FIELD.as_slice(),
    ] {
        let mut compatible = encoded.clone();
        compatible.extend_from_slice(extension);
        assert!(decode_without_panic(&decode, &compatible).unwrap() == expected);
    }

    let mut malformed_extension = encoded.clone();
    malformed_extension.extend_from_slice(&TRUNCATED_UNKNOWN_BYTES_FIELD);
    assert!(decode_without_panic(&decode, &malformed_extension).is_err());

    for end in 0..encoded.len() {
        let _ = decode_without_panic(&decode, &encoded[..end]);
    }

    for index in 0..encoded.len() {
        for bit in 0..u8::BITS {
            let mut mutated = encoded.clone();
            mutated[index] ^= 1u8 << bit;
            let _ = decode_without_panic(&decode, &mutated);
        }
    }

    let oversized = vec![0u8; FrameLimits::default().max_control_body_bytes() as usize + 1];
    assert!(decode_without_panic(&decode, &oversized).is_err());
}

fn decode_without_panic<T, F>(decode: &F, bytes: &[u8]) -> Result<T, IpcError>
where
    F: Fn(&[u8]) -> Result<T, IpcError>,
{
    std::panic::catch_unwind(AssertUnwindSafe(|| decode(bytes)))
        .unwrap_or_else(|_| panic!("decoder panicked for a {}-byte mutation", bytes.len()))
}
