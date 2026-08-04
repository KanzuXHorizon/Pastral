use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    AUTH_PROOF_BYTES, BulkEndDto, Capability, ClientHelloDto, ClipPreviewDto, ClipPreviewKind,
    CorrelationId, HealthResponseDto, HistoryPageRequestDto, HistoryPageResponseDto, IpcError,
    MAX_ERROR_DETAIL_BYTES, MAX_PAGE_LIMIT, MAX_PREVIEW_BYTES, MAX_PREVIEWS, MAX_QUERY_BYTES,
    MAX_QUERY_TERMS, MAX_SOURCE_LABEL_BYTES, ProtocolErrorCode, ProtocolErrorDto, SearchRequestDto,
    SearchResponseDto, ServerAcceptedDto, ServerHelloDto,
};

fn nonce(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn preview(index: u64) -> ClipPreviewDto {
    ClipPreviewDto::new(
        ClipEventId::new_v4(),
        CaptureOrder::new(index + 1).unwrap(),
        UtcUnixMicros::new(1_700_000_000_000_000 + i64::try_from(index).unwrap()).unwrap(),
        ClipPreviewKind::Text,
        format!("preview-{index}"),
        Some("notepad.exe".to_owned()),
        false,
        false,
    )
    .unwrap()
}

#[test]
fn hello_versions_nonce_instance_and_capabilities_are_validated() {
    let instance = CorrelationId::new_v4();
    let hello = ServerHelloDto::new(
        1,
        0,
        2,
        nonce(1),
        instance,
        [
            Capability::Health,
            Capability::HistoryPage,
            Capability::Search,
        ],
    )
    .unwrap();
    assert_eq!(hello.protocol_major(), 1);
    assert_eq!(hello.minor_range(), (0, 2));
    assert_eq!(hello.instance_id(), instance);
    assert_eq!(hello.capabilities().len(), 3);

    assert_eq!(
        ServerHelloDto::new(0, 0, 0, nonce(1), instance, [Capability::Health]).err(),
        Some(IpcError::InvalidDto("protocol major must be nonzero"))
    );
    assert_eq!(
        ServerHelloDto::new(1, 3, 2, nonce(1), instance, [Capability::Health]).err(),
        Some(IpcError::InvalidDto("minor range is invalid"))
    );
    assert_eq!(
        ServerHelloDto::new(1, 0, 0, [0; 32], instance, [Capability::Health]).err(),
        Some(IpcError::InvalidDto("nonce must not be all zero"))
    );
    assert_eq!(
        ServerHelloDto::new(
            1,
            0,
            0,
            nonce(1),
            CorrelationId::initial_server_hello(),
            [Capability::Health],
        )
        .err(),
        Some(IpcError::InvalidDto("instance ID must be nonzero"))
    );
    assert_eq!(
        ServerHelloDto::new(
            1,
            0,
            0,
            nonce(1),
            instance,
            [Capability::Health, Capability::Health],
        )
        .err(),
        Some(IpcError::InvalidDto("capability is duplicated"))
    );

    let client = ClientHelloDto::new(
        1,
        0,
        1,
        nonce(2),
        nonce(1),
        [Capability::Health],
        [0x33; AUTH_PROOF_BYTES],
    )
    .unwrap();
    assert_eq!(client.protocol_major(), 1);
    assert_eq!(client.client_nonce(), &nonce(2));
    assert_eq!(client.echoed_server_nonce(), &nonce(1));
    assert_eq!(client.authentication_proof(), &[0x33; AUTH_PROOF_BYTES]);

    assert_eq!(
        ClientHelloDto::new(
            1,
            0,
            1,
            nonce(2),
            nonce(1),
            [Capability::Health],
            [0; AUTH_PROOF_BYTES],
        )
        .err(),
        Some(IpcError::InvalidDto(
            "authentication proof must not be all zero"
        ))
    );

    let accepted = ServerAcceptedDto::new(
        1,
        [Capability::Search, Capability::Health],
        [0x44; AUTH_PROOF_BYTES],
    )
    .unwrap();
    assert_eq!(accepted.selected_minor(), 1);
    assert_eq!(
        accepted.accepted_capabilities(),
        &[Capability::Health, Capability::Search]
    );
    assert_eq!(accepted.authentication_proof(), &[0x44; AUTH_PROOF_BYTES]);
    assert_eq!(
        ServerAcceptedDto::new(1, [Capability::Health], [0; AUTH_PROOF_BYTES]).err(),
        Some(IpcError::InvalidDto(
            "authentication proof must not be all zero"
        ))
    );
}

#[test]
fn page_limits_are_positive_and_bounded() {
    assert!(HistoryPageRequestDto::new(1, None).is_ok());
    assert!(HistoryPageRequestDto::new(MAX_PAGE_LIMIT, None).is_ok());
    assert_eq!(
        HistoryPageRequestDto::new(0, None).err(),
        Some(IpcError::InvalidDto("page limit must be between 1 and 100"))
    );
    assert_eq!(
        HistoryPageRequestDto::new(MAX_PAGE_LIMIT + 1, None).err(),
        Some(IpcError::InvalidDto("page limit must be between 1 and 100"))
    );
}

#[test]
fn search_query_bytes_terms_and_content_are_bounded() {
    let request = SearchRequestDto::new("alpha beta".to_owned(), 10).unwrap();
    assert_eq!(request.query(), "alpha beta");
    assert_eq!(request.limit(), 10);

    for invalid in [String::new(), "   ".to_owned(), "a\0b".to_owned()] {
        assert!(SearchRequestDto::new(invalid, 10).is_err());
    }

    let max_query = "x".repeat(MAX_QUERY_BYTES);
    assert!(SearchRequestDto::new(max_query, 10).is_ok());
    assert_eq!(
        SearchRequestDto::new("x".repeat(MAX_QUERY_BYTES + 1), 10).err(),
        Some(IpcError::InvalidDto("search query exceeds byte limit"))
    );

    let max_terms = (0..MAX_QUERY_TERMS)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(SearchRequestDto::new(max_terms, 10).is_ok());
    let too_many_terms = (0..=MAX_QUERY_TERMS)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        SearchRequestDto::new(too_many_terms, 10).err(),
        Some(IpcError::InvalidDto("search query has too many terms"))
    );
}

#[test]
fn preview_text_source_and_unavailable_state_are_bounded() {
    let event_id = ClipEventId::new_v4();
    let order = CaptureOrder::new(1).unwrap();
    let observed = UtcUnixMicros::new(1_700_000_000_000_000).unwrap();
    let valid = ClipPreviewDto::new(
        event_id,
        order,
        observed,
        ClipPreviewKind::Text,
        "x".repeat(MAX_PREVIEW_BYTES),
        Some("s".repeat(MAX_SOURCE_LABEL_BYTES)),
        true,
        false,
    )
    .unwrap();
    assert_eq!(valid.event_id(), event_id);
    assert_eq!(valid.capture_order(), order);
    assert_eq!(valid.observed_at(), observed);
    assert_eq!(valid.preview().len(), MAX_PREVIEW_BYTES);
    assert_eq!(valid.source_label().unwrap().len(), MAX_SOURCE_LABEL_BYTES);
    assert!(valid.pinned());

    assert_eq!(
        ClipPreviewDto::new(
            event_id,
            order,
            observed,
            ClipPreviewKind::Text,
            "x".repeat(MAX_PREVIEW_BYTES + 1),
            None,
            false,
            false,
        )
        .err(),
        Some(IpcError::InvalidDto("preview exceeds byte limit"))
    );
    assert_eq!(
        ClipPreviewDto::new(
            event_id,
            order,
            observed,
            ClipPreviewKind::Text,
            String::new(),
            Some("s".repeat(MAX_SOURCE_LABEL_BYTES + 1)),
            false,
            false,
        )
        .err(),
        Some(IpcError::InvalidDto("source label exceeds byte limit"))
    );
    assert!(
        ClipPreviewDto::new(
            event_id,
            order,
            observed,
            ClipPreviewKind::Unavailable,
            String::new(),
            None,
            false,
            true,
        )
        .is_ok()
    );
    assert_eq!(
        ClipPreviewDto::new(
            event_id,
            order,
            observed,
            ClipPreviewKind::Unavailable,
            "must not leak".to_owned(),
            None,
            false,
            true,
        )
        .err(),
        Some(IpcError::InvalidDto("unavailable preview must be empty"))
    );
    assert_eq!(
        ClipPreviewDto::new(
            event_id,
            order,
            observed,
            ClipPreviewKind::Text,
            String::new(),
            None,
            false,
            true,
        )
        .err(),
        Some(IpcError::InvalidDto(
            "preview kind and unavailable state disagree"
        ))
    );
}

#[test]
fn response_item_counts_are_bounded() {
    let items = (0..MAX_PREVIEWS as u64).map(preview).collect::<Vec<_>>();
    let history = HistoryPageResponseDto::new(items.clone(), true).unwrap();
    assert_eq!(history.items().len(), MAX_PREVIEWS);
    assert!(history.has_more());
    let search = SearchResponseDto::new(items, false).unwrap();
    assert_eq!(search.items().len(), MAX_PREVIEWS);

    let too_many = (0..=MAX_PREVIEWS as u64).map(preview).collect::<Vec<_>>();
    assert_eq!(
        HistoryPageResponseDto::new(too_many.clone(), false).err(),
        Some(IpcError::InvalidDto("response contains too many previews"))
    );
    assert_eq!(
        SearchResponseDto::new(too_many, false).err(),
        Some(IpcError::InvalidDto("response contains too many previews"))
    );
}

#[test]
fn health_error_and_bulk_end_metadata_are_validated() {
    let health = HealthResponseDto::new(1, true, true, true).unwrap();
    assert_eq!(health.storage_schema_version(), 1);
    assert!(health.capture_enabled());
    assert_eq!(
        HealthResponseDto::new(0, true, true, true).err(),
        Some(IpcError::InvalidDto(
            "storage schema version must be nonzero"
        ))
    );

    let error = ProtocolErrorDto::new(
        ProtocolErrorCode::InvalidRequest,
        false,
        Some("d".repeat(MAX_ERROR_DETAIL_BYTES)),
    )
    .unwrap();
    assert_eq!(error.code(), ProtocolErrorCode::InvalidRequest);
    assert_eq!(
        error.developer_detail().unwrap().len(),
        MAX_ERROR_DETAIL_BYTES
    );
    assert_eq!(
        ProtocolErrorDto::new(
            ProtocolErrorCode::Internal,
            false,
            Some("d".repeat(MAX_ERROR_DETAIL_BYTES + 1)),
        )
        .err(),
        Some(IpcError::InvalidDto("error detail exceeds byte limit"))
    );

    let end = BulkEndDto::new(1, 1).unwrap();
    assert_eq!(end.total_bytes(), 1);
    assert_eq!(end.chunk_count(), 1);
    assert!(BulkEndDto::new(0, 1).is_err());
    assert!(BulkEndDto::new(1, 0).is_err());
}
