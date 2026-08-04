use pastral_ipc_core::{
    CorrelationId, FRAME_HEADER_BYTES, FrameHeader, FrameKind, FrameLimits, IpcError,
};

fn valid_control() -> FrameHeader {
    FrameHeader::new(
        FrameKind::ControlProto,
        17,
        0,
        CorrelationId::new_v4(),
        FrameLimits::default(),
    )
    .unwrap()
}

#[test]
fn canonical_header_round_trips_exactly_36_bytes() {
    assert_eq!(FRAME_HEADER_BYTES, 36);
    let header = valid_control();
    let encoded = header.encode();
    assert_eq!(encoded.len(), FRAME_HEADER_BYTES);
    assert_eq!(
        FrameHeader::decode(&encoded, FrameLimits::default()).unwrap(),
        header
    );
}

#[test]
fn canonical_offsets_and_little_endian_fields_are_stable() {
    let correlation = CorrelationId::new_v4();
    let header = FrameHeader::new(
        FrameKind::BulkChunk,
        0x0001_0203,
        0x0506_0708,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    let encoded = header.encode();
    assert_eq!(&encoded[0..4], b"PSTR");
    assert_eq!(&encoded[4..6], &1u16.to_le_bytes());
    assert_eq!(&encoded[6..8], &3u16.to_le_bytes());
    assert_eq!(&encoded[8..12], &0u32.to_le_bytes());
    assert_eq!(&encoded[12..16], &0x0001_0203u32.to_le_bytes());
    assert_eq!(&encoded[16..20], &0x0506_0708u32.to_le_bytes());
    assert_eq!(&encoded[20..36], correlation.as_bytes());
}

#[test]
fn invalid_magic_major_kind_and_flags_are_rejected() {
    let encoded = valid_control().encode();

    let mut invalid = encoded;
    invalid[0] = b'X';
    assert_eq!(
        FrameHeader::decode(&invalid, FrameLimits::default()),
        Err(IpcError::InvalidMagic)
    );

    let mut invalid = encoded;
    invalid[4..6].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        FrameHeader::decode(&invalid, FrameLimits::default()),
        Err(IpcError::UnsupportedFramingMajor(2))
    );

    let mut invalid = encoded;
    invalid[6..8].copy_from_slice(&99u16.to_le_bytes());
    assert_eq!(
        FrameHeader::decode(&invalid, FrameLimits::default()),
        Err(IpcError::UnknownFrameKind(99))
    );

    let mut invalid = encoded;
    invalid[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        FrameHeader::decode(&invalid, FrameLimits::default()),
        Err(IpcError::ReservedFlags(1))
    );
}

#[test]
fn body_limits_are_enforced_by_kind() {
    let limits = FrameLimits::default();
    let correlation = CorrelationId::new_v4();
    assert!(
        FrameHeader::new(
            FrameKind::ControlProto,
            limits.max_control_body_bytes(),
            0,
            correlation,
            limits,
        )
        .is_ok()
    );
    assert_eq!(
        FrameHeader::new(
            FrameKind::ControlProto,
            limits.max_control_body_bytes() + 1,
            0,
            correlation,
            limits,
        ),
        Err(IpcError::BodyLengthExceeded {
            kind: FrameKind::ControlProto,
            length: limits.max_control_body_bytes() + 1,
            limit: limits.max_control_body_bytes(),
        })
    );
    assert_eq!(
        FrameHeader::new(FrameKind::BulkChunk, 0, 0, correlation, limits),
        Err(IpcError::EmptyBulkChunk)
    );
    assert!(
        FrameHeader::new(
            FrameKind::BulkChunk,
            limits.max_bulk_chunk_bytes(),
            0,
            correlation,
            limits,
        )
        .is_ok()
    );
    assert_eq!(
        FrameHeader::new(
            FrameKind::BulkChunk,
            limits.max_bulk_chunk_bytes() + 1,
            0,
            correlation,
            limits,
        ),
        Err(IpcError::BodyLengthExceeded {
            kind: FrameKind::BulkChunk,
            length: limits.max_bulk_chunk_bytes() + 1,
            limit: limits.max_bulk_chunk_bytes(),
        })
    );
}

#[test]
fn sequence_rules_are_structurally_enforced() {
    let limits = FrameLimits::default();
    let correlation = CorrelationId::new_v4();
    for kind in [
        FrameKind::HelloProto,
        FrameKind::ControlProto,
        FrameKind::ProtocolErrorProto,
    ] {
        assert_eq!(
            FrameHeader::new(kind, 0, 1, correlation, limits),
            Err(IpcError::InvalidFrameSequence)
        );
    }
    assert!(FrameHeader::new(FrameKind::BulkChunk, 1, u32::MAX, correlation, limits).is_ok());
    assert!(FrameHeader::new(FrameKind::BulkEndProto, 0, u32::MAX, correlation, limits).is_ok());
}

#[test]
fn zero_correlation_is_only_structurally_valid_for_hello() {
    let limits = FrameLimits::default();
    let zero = CorrelationId::initial_server_hello();
    assert!(FrameHeader::new(FrameKind::HelloProto, 0, 0, zero, limits).is_ok());
    for kind in [
        FrameKind::ControlProto,
        FrameKind::BulkChunk,
        FrameKind::BulkEndProto,
        FrameKind::ProtocolErrorProto,
    ] {
        let body_length = if kind == FrameKind::BulkChunk { 1 } else { 0 };
        assert_eq!(
            FrameHeader::new(kind, body_length, 0, zero, limits),
            Err(IpcError::InvalidCorrelation)
        );
    }
}

#[test]
fn correlation_accepts_only_zero_or_canonical_uuid_v4_rfc4122_bytes() {
    assert!(CorrelationId::from_bytes([0; 16]).is_ok());
    let valid = CorrelationId::new_v4();
    assert_eq!(CorrelationId::from_bytes(*valid.as_bytes()).unwrap(), valid);

    let mut v1 = *valid.as_bytes();
    v1[6] = (v1[6] & 0x0f) | 0x10;
    assert_eq!(
        CorrelationId::from_bytes(v1),
        Err(IpcError::InvalidCorrelation)
    );

    let mut microsoft_variant = *valid.as_bytes();
    microsoft_variant[8] = (microsoft_variant[8] & 0x1f) | 0xc0;
    assert_eq!(
        CorrelationId::from_bytes(microsoft_variant),
        Err(IpcError::InvalidCorrelation)
    );
}

#[test]
fn zero_limits_are_rejected() {
    assert_eq!(
        FrameLimits::new(0, 1, 1, 1),
        Err(IpcError::InvalidLimit("max control body bytes"))
    );
    assert_eq!(
        FrameLimits::new(1, 0, 1, 1),
        Err(IpcError::InvalidLimit("max bulk chunk bytes"))
    );
    assert_eq!(
        FrameLimits::new(1, 1, 0, 1),
        Err(IpcError::InvalidLimit("max frames per push"))
    );
    assert_eq!(
        FrameLimits::new(1, 1, 1, 0),
        Err(IpcError::InvalidLimit("max in-flight requests"))
    );
}
