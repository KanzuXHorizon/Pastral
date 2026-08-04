use pastral_domain::{ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{FrameLimits, MAX_PREVIEWS, MAX_QUERY_BYTES, MAX_QUERY_TERMS};
use pastral_ipc_schema::{
    decode_bulk_end, decode_client_hello, decode_protocol_error, decode_request, decode_response,
    decode_server_accepted, decode_server_hello, generated,
};
fn serialize<M: protobuf::Message>(message: &M) -> Vec<u8> {
    message.serialize().unwrap()
}

fn valid_server_hello() -> generated::ServerHello {
    let mut message = generated::ServerHello::new();
    message.set_protocol_major(1);
    message.set_min_minor(0);
    message.set_max_minor(1);
    message.set_server_nonce([0x11; 32].as_slice());
    message.set_instance_id(ClipEventId::new_v4().as_uuid().as_bytes().as_slice());
    message
        .capabilities_mut()
        .push(generated::Capability::Health);
    message
}

fn valid_client_hello() -> generated::ClientHello {
    let mut message = generated::ClientHello::new();
    message.set_protocol_major(1);
    message.set_min_minor(0);
    message.set_max_minor(1);
    message.set_client_nonce([0x22; 32].as_slice());
    message.set_echoed_server_nonce([0x11; 32].as_slice());
    message
        .capabilities_mut()
        .push(generated::Capability::Health);
    message.set_authentication_proof([0x33; 32].as_slice());
    message
}

fn valid_server_accepted() -> generated::ServerAccepted {
    let mut message = generated::ServerAccepted::new();
    message.set_selected_minor(1);
    message
        .accepted_capabilities_mut()
        .push(generated::Capability::Health);
    message.set_authentication_proof([0x44; 32].as_slice());
    message
}

fn valid_preview() -> generated::ClipPreview {
    let mut message = generated::ClipPreview::new();
    message.set_event_id(ClipEventId::new_v4().as_uuid().as_bytes().as_slice());
    message.set_capture_order(1);
    message.set_observed_at_utc_micros(1_700_000_000_000_000);
    message.set_kind(generated::ClipPreviewKind::Text);
    message.set_preview("preview");
    message.set_pinned(false);
    message.set_unavailable(false);
    message
}

#[test]
fn missing_operations_and_oversized_control_bodies_are_rejected() {
    let request = generated::RequestEnvelope::new();
    assert!(decode_request(&serialize(&request)).is_err());

    let response = generated::ResponseEnvelope::new();
    assert!(decode_response(&serialize(&response)).is_err());

    let limit = usize::try_from(FrameLimits::default().max_control_body_bytes()).unwrap();
    assert!(decode_request(&vec![0; limit + 1]).is_err());
    assert!(decode_response(&vec![0; limit + 1]).is_err());
}

#[test]
fn hello_presence_nonce_uuid_and_capability_values_are_rejected_fail_closed() {
    let missing = generated::ServerHello::new();
    assert!(decode_server_hello(&serialize(&missing)).is_err());

    let mut nonce = valid_server_hello();
    nonce.set_server_nonce([0x11; 31].as_slice());
    assert!(decode_server_hello(&serialize(&nonce)).is_err());

    let mut instance = valid_server_hello();
    instance.set_instance_id([0x44; 16].as_slice());
    assert!(decode_server_hello(&serialize(&instance)).is_err());

    let mut zero_capability = valid_server_hello();
    zero_capability
        .capabilities_mut()
        .push(generated::Capability::Unspecified);
    assert!(decode_server_hello(&serialize(&zero_capability)).is_err());

    let mut unknown_capability = valid_server_hello();
    unknown_capability
        .capabilities_mut()
        .push(generated::Capability::from(99));
    assert!(decode_server_hello(&serialize(&unknown_capability)).is_err());

    let mut client = valid_client_hello();
    client.clear_echoed_server_nonce();
    assert!(decode_client_hello(&serialize(&client)).is_err());

    let mut missing_client_proof = valid_client_hello();
    missing_client_proof.clear_authentication_proof();
    assert!(decode_client_hello(&serialize(&missing_client_proof)).is_err());

    let mut short_client_proof = valid_client_hello();
    short_client_proof.set_authentication_proof([0x33; 31].as_slice());
    assert!(decode_client_hello(&serialize(&short_client_proof)).is_err());

    let mut zero_client_proof = valid_client_hello();
    zero_client_proof.set_authentication_proof([0; 32].as_slice());
    assert!(decode_client_hello(&serialize(&zero_client_proof)).is_err());

    let missing_accepted = generated::ServerAccepted::new();
    assert!(decode_server_accepted(&serialize(&missing_accepted)).is_err());

    let mut zero_capability = valid_server_accepted();
    zero_capability
        .accepted_capabilities_mut()
        .push(generated::Capability::Unspecified);
    assert!(decode_server_accepted(&serialize(&zero_capability)).is_err());

    let mut short_server_proof = valid_server_accepted();
    short_server_proof.set_authentication_proof([0x44; 31].as_slice());
    assert!(decode_server_accepted(&serialize(&short_server_proof)).is_err());

    let mut zero_server_proof = valid_server_accepted();
    zero_server_proof.set_authentication_proof([0; 32].as_slice());
    assert!(decode_server_accepted(&serialize(&zero_server_proof)).is_err());
}

#[test]
fn request_limits_and_presence_are_rejected_after_parse() {
    let mut history = generated::HistoryPageRequest::new();
    history.set_limit(0);
    let mut envelope = generated::RequestEnvelope::new();
    envelope.set_history_page(history);
    assert!(decode_request(&serialize(&envelope)).is_err());

    let mut history = generated::HistoryPageRequest::new();
    history.set_limit(10);
    history.set_before_capture_order(0);
    let mut envelope = generated::RequestEnvelope::new();
    envelope.set_history_page(history);
    assert!(decode_request(&serialize(&envelope)).is_err());

    let mut search = generated::SearchRequest::new();
    search.set_query("x".repeat(MAX_QUERY_BYTES + 1));
    search.set_limit(10);
    let mut envelope = generated::RequestEnvelope::new();
    envelope.set_search(search);
    assert!(decode_request(&serialize(&envelope)).is_err());

    let too_many_terms = (0..=MAX_QUERY_TERMS)
        .map(|index| format!("t{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut search = generated::SearchRequest::new();
    search.set_query(too_many_terms);
    search.set_limit(10);
    let mut envelope = generated::RequestEnvelope::new();
    envelope.set_search(search);
    assert!(decode_request(&serialize(&envelope)).is_err());

    let mut search = generated::SearchRequest::new();
    search.set_query("valid");
    let mut envelope = generated::RequestEnvelope::new();
    envelope.set_search(search);
    assert!(decode_request(&serialize(&envelope)).is_err());
}

#[test]
fn preview_identity_order_time_kind_text_and_result_count_are_rejected() {
    let cases = [
        {
            let mut value = valid_preview();
            value.set_event_id([0; 15].as_slice());
            value
        },
        {
            let mut value = valid_preview();
            value.set_capture_order(0);
            value
        },
        {
            let mut value = valid_preview();
            value.set_observed_at_utc_micros(UtcUnixMicros::MAX + 1);
            value
        },
        {
            let mut value = valid_preview();
            value.set_kind(generated::ClipPreviewKind::Unspecified);
            value
        },
        {
            let mut value = valid_preview();
            value.set_kind(generated::ClipPreviewKind::Unavailable);
            value.set_unavailable(true);
            value.set_preview("leak");
            value
        },
    ];

    for preview in cases {
        let mut response = generated::HistoryPageResponse::new();
        response.items_mut().push(preview);
        response.set_has_more(false);
        let mut envelope = generated::ResponseEnvelope::new();
        envelope.set_history_page(response);
        assert!(decode_response(&serialize(&envelope)).is_err());
    }

    let mut response = generated::HistoryPageResponse::new();
    for _ in 0..=MAX_PREVIEWS {
        response.items_mut().push(valid_preview());
    }
    response.set_has_more(false);
    let mut envelope = generated::ResponseEnvelope::new();
    envelope.set_history_page(response);
    assert!(decode_response(&serialize(&envelope)).is_err());
}

#[test]
fn error_and_bulk_end_zero_unknown_or_missing_fields_are_rejected() {
    let mut error = generated::ProtocolError::new();
    error.set_code(generated::ProtocolErrorCode::Unspecified);
    error.set_retryable(false);
    assert!(decode_protocol_error(&serialize(&error)).is_err());

    let mut unknown = generated::ProtocolError::new();
    unknown.set_code(generated::ProtocolErrorCode::from(99));
    unknown.set_retryable(false);
    assert!(decode_protocol_error(&serialize(&unknown)).is_err());

    let missing = generated::ProtocolError::new();
    assert!(decode_protocol_error(&serialize(&missing)).is_err());

    let mut end = generated::BulkEnd::new();
    end.set_total_bytes(0);
    end.set_chunk_count(1);
    assert!(decode_bulk_end(&serialize(&end)).is_err());

    let missing = generated::BulkEnd::new();
    assert!(decode_bulk_end(&serialize(&missing)).is_err());
}

#[test]
fn malformed_wire_data_is_rejected() {
    for bytes in [vec![0xff], vec![0x0a, 0xff], vec![0x80]] {
        assert!(decode_request(&bytes).is_err());
        assert!(decode_response(&bytes).is_err());
        assert!(decode_server_hello(&bytes).is_err());
    }
}
