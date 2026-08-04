use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    Capability, ClipPreviewDto, ClipPreviewKind, CorrelationId, HealthRequestDto,
    HealthResponseDto, HistoryPageRequestDto, HistoryPageResponseDto, ProtocolErrorCode,
    ProtocolErrorDto, RequestDto, ResponseDto, SearchRequestDto, SearchResponseDto, ServerHelloDto,
};
use pastral_ipc_schema::{
    decode_request, decode_response, decode_server_hello, encode_request, encode_response,
    encode_server_hello, schema_sha256,
};
use uuid::Uuid;

const SCHEMA_SHA256: &str = "2029ac9b19f7eb1644a2c12b3cd570586af9b62c40e130558b63c376676e3077";
const SERVER_HELLO_WIRE: &str = "080110001800222011111111111111111111111111111111111111111111111111111111111111112a10101112131415461798191a1b1c1d1e1f3203010203";
const REQUEST_WIRES: [&str; 3] = [
    "0a00",
    "12040819102a",
    "1a110a0d616c706861204f5220626574611032",
];
const RESPONSE_WIRES: [&str; 4] = [
    "0a080801100118012001",
    "123c0a380a102021222324254627a8292a2b2c2d2e2f1007188e80f2818389850620012a0770726576696577320a736f757263652e657865380140001001",
    "1a3c0a380a102021222324254627a8292a2b2c2d2e2f1007188e80f2818389850620012a0770726576696577320a736f757263652e657865380140001000",
    "220408011000",
];

#[test]
fn schema_digest_and_control_wire_vectors_are_exact() {
    let server = ServerHelloDto::new(
        1,
        0,
        0,
        [0x11; 32],
        correlation(0x10),
        [
            Capability::Health,
            Capability::HistoryPage,
            Capability::Search,
        ],
    )
    .unwrap();
    let requests = [
        RequestDto::Health(HealthRequestDto),
        RequestDto::HistoryPage(
            HistoryPageRequestDto::new(25, Some(CaptureOrder::new(42).unwrap())).unwrap(),
        ),
        RequestDto::Search(SearchRequestDto::new("alpha OR beta".to_owned(), 50).unwrap()),
    ];
    let item = ClipPreviewDto::new(
        ClipEventId::from_uuid(uuid(0x20)).unwrap(),
        CaptureOrder::new(7).unwrap(),
        UtcUnixMicros::new(1_700_000_000_000_007).unwrap(),
        ClipPreviewKind::Text,
        "preview".to_owned(),
        Some("source.exe".to_owned()),
        true,
        false,
    )
    .unwrap();
    let responses = [
        ResponseDto::Health(HealthResponseDto::new(1, true, true, true).unwrap()),
        ResponseDto::HistoryPage(HistoryPageResponseDto::new(vec![item.clone()], true).unwrap()),
        ResponseDto::Search(SearchResponseDto::new(vec![item], false).unwrap()),
        ResponseDto::Error(
            ProtocolErrorDto::new(ProtocolErrorCode::InvalidRequest, false, None).unwrap(),
        ),
    ];

    assert_eq!(hex(&schema_sha256()), SCHEMA_SHA256);
    assert_eq!(
        hex(&encode_server_hello(&server).unwrap()),
        SERVER_HELLO_WIRE
    );
    for (request, expected) in requests.iter().zip(REQUEST_WIRES) {
        assert_eq!(hex(&encode_request(request).unwrap()), expected);
    }
    for (response, expected) in responses.iter().zip(RESPONSE_WIRES) {
        assert_eq!(hex(&encode_response(response).unwrap()), expected);
    }

    assert_eq!(
        decode_server_hello(&encode_server_hello(&server).unwrap()).unwrap(),
        server
    );
    for request in requests {
        assert!(decode_request(&encode_request(&request).unwrap()).unwrap() == request);
    }
    for response in responses {
        assert!(decode_response(&encode_response(&response).unwrap()).unwrap() == response);
    }
}

fn correlation(seed: u8) -> CorrelationId {
    CorrelationId::from_bytes(uuid_bytes(seed)).unwrap()
}

fn uuid(seed: u8) -> Uuid {
    Uuid::from_bytes(uuid_bytes(seed))
}

fn uuid_bytes(seed: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.wrapping_add(u8::try_from(index).unwrap());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
