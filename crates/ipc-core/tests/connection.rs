use pastral_ipc_core::{
    AcceptedFrame, ConnectionPhase, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits,
    IpcError, ServerConnection,
};

fn frame(kind: FrameKind, body: &[u8], sequence: u32, correlation: CorrelationId) -> Frame {
    let header = FrameHeader::new(
        kind,
        u32::try_from(body.len()).unwrap(),
        sequence,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    Frame::new(header, body.to_vec()).unwrap()
}

fn client_hello(correlation: CorrelationId) -> Frame {
    frame(FrameKind::HelloProto, b"client-hello", 0, correlation)
}

fn control(correlation: CorrelationId) -> Frame {
    frame(FrameKind::ControlProto, b"control", 0, correlation)
}

fn ready_connection() -> ServerConnection {
    let mut connection = ServerConnection::new(FrameLimits::default());
    let hello = client_hello(CorrelationId::new_v4());
    assert!(matches!(
        connection.accept(&hello).unwrap(),
        AcceptedFrame::ClientHello { .. }
    ));
    assert_eq!(connection.phase(), ConnectionPhase::Ready);
    connection
}

#[test]
fn control_before_hello_closes_connection() {
    let mut connection = ServerConnection::new(FrameLimits::default());
    let request = control(CorrelationId::new_v4());
    assert_eq!(
        connection.accept(&request),
        Err(IpcError::InvalidConnectionState)
    );
    assert_eq!(connection.phase(), ConnectionPhase::Closed);
    assert_eq!(
        connection.accept(&client_hello(CorrelationId::new_v4())),
        Err(IpcError::InvalidConnectionState)
    );
}

#[test]
fn valid_nonzero_client_hello_transitions_to_ready() {
    let correlation = CorrelationId::new_v4();
    let hello = client_hello(correlation);
    let mut connection = ServerConnection::new(FrameLimits::default());
    assert_eq!(
        connection.accept(&hello).unwrap(),
        AcceptedFrame::ClientHello {
            correlation,
            body: b"client-hello",
        }
    );
    assert_eq!(connection.phase(), ConnectionPhase::Ready);

    let zero_hello = frame(
        FrameKind::HelloProto,
        b"zero",
        0,
        CorrelationId::initial_server_hello(),
    );
    let mut zero_connection = ServerConnection::new(FrameLimits::default());
    assert_eq!(
        zero_connection.accept(&zero_hello),
        Err(IpcError::InvalidCorrelation)
    );
    assert_eq!(zero_connection.phase(), ConnectionPhase::Closed);
}

#[test]
fn second_hello_is_rejected_and_closes() {
    let mut connection = ready_connection();
    assert_eq!(
        connection.accept(&client_hello(CorrelationId::new_v4())),
        Err(IpcError::InvalidConnectionState)
    );
    assert_eq!(connection.phase(), ConnectionPhase::Closed);
}

#[test]
fn correlations_are_unique_bounded_and_releasable() {
    let mut connection = ready_connection();
    let mut correlations = Vec::new();
    for index in 0..FrameLimits::default().max_in_flight_requests() {
        let correlation = CorrelationId::new_v4();
        correlations.push(correlation);
        let request = control(correlation);
        assert_eq!(
            connection.accept(&request).unwrap(),
            AcceptedFrame::Control {
                correlation,
                body: b"control",
            },
            "request {index}"
        );
    }
    assert_eq!(
        connection.in_flight_count(),
        FrameLimits::default().max_in_flight_requests()
    );
    let overflow = control(CorrelationId::new_v4());
    assert_eq!(
        connection.accept(&overflow),
        Err(IpcError::InFlightLimitExceeded)
    );
    assert_eq!(connection.phase(), ConnectionPhase::Closed);

    let mut connection = ready_connection();
    let correlation = CorrelationId::new_v4();
    let request = control(correlation);
    connection.accept(&request).unwrap();
    assert_eq!(
        connection.accept(&request),
        Err(IpcError::DuplicateCorrelation)
    );
    assert_eq!(connection.phase(), ConnectionPhase::Closed);

    let mut connection = ready_connection();
    let first = CorrelationId::new_v4();
    connection.accept(&control(first)).unwrap();
    connection.complete_request(first).unwrap();
    assert_eq!(connection.in_flight_count(), 0);
    let second = CorrelationId::new_v4();
    connection.accept(&control(second)).unwrap();
    connection.cancel_request(second).unwrap();
    assert_eq!(connection.in_flight_count(), 0);
    assert_eq!(
        connection.complete_request(second),
        Err(IpcError::UnknownCorrelation)
    );
}

#[test]
fn bulk_requires_explicit_authorization() {
    let transfer = CorrelationId::new_v4();
    let chunk = frame(FrameKind::BulkChunk, b"a", 0, transfer);
    let mut connection = ready_connection();
    assert_eq!(connection.accept(&chunk), Err(IpcError::BulkNotAuthorized));
    assert_eq!(connection.phase(), ConnectionPhase::Closed);
}

#[test]
fn authorized_bulk_chunks_and_end_are_strictly_ordered() {
    let transfer = CorrelationId::new_v4();
    let mut connection = ready_connection();
    connection.authorize_bulk(transfer, 8, 3).unwrap();
    assert_eq!(connection.phase(), ConnectionPhase::BulkReceiving);

    let first = frame(FrameKind::BulkChunk, b"ab", 0, transfer);
    assert_eq!(
        connection.accept(&first).unwrap(),
        AcceptedFrame::BulkChunk {
            transfer,
            sequence: 0,
            bytes: b"ab",
        }
    );
    let second = frame(FrameKind::BulkChunk, b"c", 1, transfer);
    connection.accept(&second).unwrap();
    let progress = connection.bulk_progress().unwrap();
    assert_eq!(progress.accepted_chunks(), 2);
    assert_eq!(progress.accepted_bytes(), 3);
    assert_eq!(progress.next_sequence(), 2);

    let end = frame(FrameKind::BulkEndProto, b"end-metadata", 2, transfer);
    assert_eq!(
        connection.accept(&end).unwrap(),
        AcceptedFrame::BulkEnd {
            transfer,
            accepted_chunks: 2,
            accepted_bytes: 3,
            body: b"end-metadata",
        }
    );
    connection.complete_bulk(3, 2).unwrap();
    assert_eq!(connection.phase(), ConnectionPhase::Ready);
    assert_eq!(connection.bulk_progress(), None);
}

#[test]
fn bulk_sequence_transfer_and_limits_fail_closed() {
    let frame_value = frame(FrameKind::BulkChunk, b"x", 1, CorrelationId::new_v4());
    let transfer = CorrelationId::new_v4();
    let mut connection = ready_connection();
    connection.authorize_bulk(transfer, 4, 2).unwrap();
    assert_eq!(
        connection.accept(&frame_value),
        Err(IpcError::InvalidCorrelation)
    );
    assert_eq!(connection.phase(), ConnectionPhase::Closed);

    let transfer = CorrelationId::new_v4();
    let mut gap = ready_connection();
    gap.authorize_bulk(transfer, 4, 2).unwrap();
    assert_eq!(
        gap.accept(&frame(FrameKind::BulkChunk, b"a", 1, transfer)),
        Err(IpcError::BulkSequenceMismatch {
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(gap.phase(), ConnectionPhase::Closed);

    let transfer = CorrelationId::new_v4();
    let mut length = ready_connection();
    length.authorize_bulk(transfer, 2, 2).unwrap();
    assert_eq!(
        length.accept(&frame(FrameKind::BulkChunk, b"abc", 0, transfer)),
        Err(IpcError::BulkLengthExceeded)
    );

    let transfer = CorrelationId::new_v4();
    let mut chunks = ready_connection();
    chunks.authorize_bulk(transfer, 4, 1).unwrap();
    chunks
        .accept(&frame(FrameKind::BulkChunk, b"a", 0, transfer))
        .unwrap();
    assert_eq!(
        chunks.accept(&frame(FrameKind::BulkChunk, b"b", 1, transfer)),
        Err(IpcError::BulkChunkLimitExceeded)
    );
}

#[test]
fn bulk_end_requires_exact_progress_and_no_post_end_chunks() {
    let transfer = CorrelationId::new_v4();
    let mut wrong_end = ready_connection();
    wrong_end.authorize_bulk(transfer, 8, 3).unwrap();
    wrong_end
        .accept(&frame(FrameKind::BulkChunk, b"ab", 0, transfer))
        .unwrap();
    assert_eq!(
        wrong_end.accept(&frame(FrameKind::BulkEndProto, b"end", 0, transfer)),
        Err(IpcError::BulkSequenceMismatch {
            expected: 1,
            actual: 0,
        })
    );

    let transfer = CorrelationId::new_v4();
    let mut metadata = ready_connection();
    metadata.authorize_bulk(transfer, 8, 3).unwrap();
    metadata
        .accept(&frame(FrameKind::BulkChunk, b"ab", 0, transfer))
        .unwrap();
    metadata
        .accept(&frame(FrameKind::BulkEndProto, b"end", 1, transfer))
        .unwrap();
    assert_eq!(metadata.complete_bulk(3, 1), Err(IpcError::BulkEndMismatch));
    assert_eq!(metadata.phase(), ConnectionPhase::Closed);

    let transfer = CorrelationId::new_v4();
    let mut post_end = ready_connection();
    post_end.authorize_bulk(transfer, 8, 3).unwrap();
    post_end
        .accept(&frame(FrameKind::BulkChunk, b"a", 0, transfer))
        .unwrap();
    post_end
        .accept(&frame(FrameKind::BulkEndProto, b"end", 1, transfer))
        .unwrap();
    assert_eq!(
        post_end.accept(&frame(FrameKind::BulkChunk, b"b", 1, transfer)),
        Err(IpcError::InvalidConnectionState)
    );
    assert_eq!(post_end.phase(), ConnectionPhase::Closed);
}
