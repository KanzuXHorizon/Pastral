use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    BulkEndDto, Capability, ClientHelloDto, ClipPreviewDto, ClipPreviewKind, CorrelationId,
    FrameLimits, HealthRequestDto, HealthResponseDto, HistoryPageRequestDto,
    HistoryPageResponseDto, IpcError, ProtocolErrorCode, ProtocolErrorDto, RequestDto, ResponseDto,
    SearchRequestDto, SearchResponseDto, ServerHelloDto,
};
use protobuf::prelude::*;
use uuid::Uuid;

use crate::generated;

pub fn encode_server_hello(value: &ServerHelloDto) -> Result<Vec<u8>, IpcError> {
    let mut message = generated::ServerHello::new();
    message.set_protocol_major(value.protocol_major());
    let (min_minor, max_minor) = value.minor_range();
    message.set_min_minor(min_minor);
    message.set_max_minor(max_minor);
    message.set_server_nonce(value.server_nonce().as_slice());
    message.set_instance_id(value.instance_id().as_bytes().as_slice());
    push_capabilities(message.capabilities_mut(), value.capabilities());
    serialize(&message)
}

pub fn decode_server_hello(bytes: &[u8]) -> Result<ServerHelloDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::ServerHello::parse(bytes).map_err(schema_error)?;
    require(
        message.has_protocol_major(),
        "server hello protocol major is missing",
    )?;
    require(
        message.has_min_minor(),
        "server hello minimum minor is missing",
    )?;
    require(
        message.has_max_minor(),
        "server hello maximum minor is missing",
    )?;
    require(message.has_server_nonce(), "server hello nonce is missing")?;
    require(
        message.has_instance_id(),
        "server hello instance ID is missing",
    )?;
    let server_nonce = fixed_bytes::<32>(message.server_nonce(), "server hello nonce length")?;
    let instance_id = correlation_id(message.instance_id())?;
    let capabilities = decode_capabilities(message.capabilities())?;
    ServerHelloDto::new(
        message.protocol_major(),
        message.min_minor(),
        message.max_minor(),
        server_nonce,
        instance_id,
        capabilities,
    )
}

pub fn encode_client_hello(value: &ClientHelloDto) -> Result<Vec<u8>, IpcError> {
    let mut message = generated::ClientHello::new();
    message.set_protocol_major(value.protocol_major());
    let (min_minor, max_minor) = value.minor_range();
    message.set_min_minor(min_minor);
    message.set_max_minor(max_minor);
    message.set_client_nonce(value.client_nonce().as_slice());
    message.set_echoed_server_nonce(value.echoed_server_nonce().as_slice());
    push_capabilities(message.capabilities_mut(), value.capabilities());
    serialize(&message)
}

pub fn decode_client_hello(bytes: &[u8]) -> Result<ClientHelloDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::ClientHello::parse(bytes).map_err(schema_error)?;
    require(
        message.has_protocol_major(),
        "client hello protocol major is missing",
    )?;
    require(
        message.has_min_minor(),
        "client hello minimum minor is missing",
    )?;
    require(
        message.has_max_minor(),
        "client hello maximum minor is missing",
    )?;
    require(message.has_client_nonce(), "client hello nonce is missing")?;
    require(
        message.has_echoed_server_nonce(),
        "client hello echoed server nonce is missing",
    )?;
    let client_nonce = fixed_bytes::<32>(message.client_nonce(), "client hello nonce length")?;
    let echoed_server_nonce = fixed_bytes::<32>(
        message.echoed_server_nonce(),
        "client hello echoed server nonce length",
    )?;
    let capabilities = decode_capabilities(message.capabilities())?;
    ClientHelloDto::new(
        message.protocol_major(),
        message.min_minor(),
        message.max_minor(),
        client_nonce,
        echoed_server_nonce,
        capabilities,
    )
}

pub fn encode_request(value: &RequestDto) -> Result<Vec<u8>, IpcError> {
    let mut envelope = generated::RequestEnvelope::new();
    match value {
        RequestDto::Health(_) => envelope.set_health(generated::HealthRequest::new()),
        RequestDto::HistoryPage(request) => {
            let mut message = generated::HistoryPageRequest::new();
            message.set_limit(request.limit());
            if let Some(order) = request.before_capture_order() {
                message.set_before_capture_order(order.get());
            }
            envelope.set_history_page(message);
        }
        RequestDto::Search(request) => {
            let mut message = generated::SearchRequest::new();
            message.set_query(request.query());
            message.set_limit(request.limit());
            envelope.set_search(message);
        }
    }
    serialize(&envelope)
}

pub fn decode_request(bytes: &[u8]) -> Result<RequestDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::RequestEnvelope::parse(bytes).map_err(schema_error)?;
    match message.operation_case() {
        generated::request_envelope::OperationCase::Health => {
            Ok(RequestDto::Health(HealthRequestDto))
        }
        generated::request_envelope::OperationCase::HistoryPage => {
            let request = message.history_page();
            require(request.has_limit(), "history page limit is missing")?;
            let before_capture_order = if request.has_before_capture_order() {
                Some(capture_order(request.before_capture_order())?)
            } else {
                None
            };
            Ok(RequestDto::HistoryPage(HistoryPageRequestDto::new(
                request.limit(),
                before_capture_order,
            )?))
        }
        generated::request_envelope::OperationCase::Search => {
            let request = message.search();
            require(request.has_query(), "search query is missing")?;
            require(request.has_limit(), "search limit is missing")?;
            Ok(RequestDto::Search(SearchRequestDto::new(
                proto_string(request.query())?,
                request.limit(),
            )?))
        }
        generated::request_envelope::OperationCase::not_set => {
            Err(IpcError::InvalidDto("request operation is missing"))
        }
    }
}

pub fn encode_response(value: &ResponseDto) -> Result<Vec<u8>, IpcError> {
    let mut envelope = generated::ResponseEnvelope::new();
    match value {
        ResponseDto::Health(response) => {
            let mut message = generated::HealthResponse::new();
            message.set_storage_schema_version(response.storage_schema_version());
            message.set_capture_enabled(response.capture_enabled());
            message.set_privacy_policy_ok(response.privacy_policy_ok());
            message.set_storage_integrity_ok(response.storage_integrity_ok());
            envelope.set_health(message);
        }
        ResponseDto::HistoryPage(response) => {
            let mut message = generated::HistoryPageResponse::new();
            push_previews(message.items_mut(), response.items())?;
            message.set_has_more(response.has_more());
            envelope.set_history_page(message);
        }
        ResponseDto::Search(response) => {
            let mut message = generated::SearchResponse::new();
            push_previews(message.items_mut(), response.items())?;
            message.set_has_more(response.has_more());
            envelope.set_search(message);
        }
        ResponseDto::Error(error) => envelope.set_error(protocol_error_message(error)),
    }
    serialize(&envelope)
}

pub fn decode_response(bytes: &[u8]) -> Result<ResponseDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::ResponseEnvelope::parse(bytes).map_err(schema_error)?;
    match message.operation_case() {
        generated::response_envelope::OperationCase::Health => {
            let response = message.health();
            require(
                response.has_storage_schema_version(),
                "health storage schema version is missing",
            )?;
            require(
                response.has_capture_enabled(),
                "health capture state is missing",
            )?;
            require(
                response.has_privacy_policy_ok(),
                "health privacy state is missing",
            )?;
            require(
                response.has_storage_integrity_ok(),
                "health storage integrity state is missing",
            )?;
            Ok(ResponseDto::Health(HealthResponseDto::new(
                response.storage_schema_version(),
                response.capture_enabled(),
                response.privacy_policy_ok(),
                response.storage_integrity_ok(),
            )?))
        }
        generated::response_envelope::OperationCase::HistoryPage => {
            let response = message.history_page();
            require(response.has_has_more(), "history has-more state is missing")?;
            Ok(ResponseDto::HistoryPage(HistoryPageResponseDto::new(
                decode_previews(response.items())?,
                response.has_more(),
            )?))
        }
        generated::response_envelope::OperationCase::Search => {
            let response = message.search();
            require(response.has_has_more(), "search has-more state is missing")?;
            Ok(ResponseDto::Search(SearchResponseDto::new(
                decode_previews(response.items())?,
                response.has_more(),
            )?))
        }
        generated::response_envelope::OperationCase::Error => Ok(ResponseDto::Error(
            decode_protocol_error_view(message.error())?,
        )),
        generated::response_envelope::OperationCase::not_set => {
            Err(IpcError::InvalidDto("response operation is missing"))
        }
    }
}

pub fn encode_protocol_error(value: &ProtocolErrorDto) -> Result<Vec<u8>, IpcError> {
    serialize(&protocol_error_message(value))
}

pub fn decode_protocol_error(bytes: &[u8]) -> Result<ProtocolErrorDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::ProtocolError::parse(bytes).map_err(schema_error)?;
    decode_protocol_error_view(message.as_view())
}

pub fn encode_bulk_end(value: BulkEndDto) -> Result<Vec<u8>, IpcError> {
    let mut message = generated::BulkEnd::new();
    message.set_total_bytes(value.total_bytes());
    message.set_chunk_count(value.chunk_count());
    serialize(&message)
}

pub fn decode_bulk_end(bytes: &[u8]) -> Result<BulkEndDto, IpcError> {
    ensure_control_body_bound(bytes)?;
    let message = generated::BulkEnd::parse(bytes).map_err(schema_error)?;
    require(message.has_total_bytes(), "bulk total bytes are missing")?;
    require(message.has_chunk_count(), "bulk chunk count is missing")?;
    BulkEndDto::new(message.total_bytes(), message.chunk_count())
}

fn push_capabilities(
    mut target: protobuf::RepeatedMut<'_, generated::Capability>,
    capabilities: &[Capability],
) {
    for capability in capabilities {
        target.push(encode_capability(*capability));
    }
}

fn decode_capabilities(
    values: protobuf::RepeatedView<'_, generated::Capability>,
) -> Result<Vec<Capability>, IpcError> {
    values.into_iter().map(decode_capability).collect()
}

fn encode_capability(value: Capability) -> generated::Capability {
    match value {
        Capability::Health => generated::Capability::Health,
        Capability::HistoryPage => generated::Capability::HistoryPage,
        Capability::Search => generated::Capability::Search,
    }
}

fn decode_capability(value: generated::Capability) -> Result<Capability, IpcError> {
    match i32::from(value) {
        1 => Ok(Capability::Health),
        2 => Ok(Capability::HistoryPage),
        3 => Ok(Capability::Search),
        _ => Err(IpcError::InvalidDto("capability is zero or unknown")),
    }
}

fn push_previews(
    mut target: protobuf::RepeatedMut<'_, generated::ClipPreview>,
    previews: &[ClipPreviewDto],
) -> Result<(), IpcError> {
    for preview in previews {
        target.push(encode_preview(preview)?);
    }
    Ok(())
}

fn encode_preview(value: &ClipPreviewDto) -> Result<generated::ClipPreview, IpcError> {
    let mut message = generated::ClipPreview::new();
    message.set_event_id(value.event_id().as_uuid().as_bytes().as_slice());
    message.set_capture_order(value.capture_order().get());
    message.set_observed_at_utc_micros(value.observed_at().get());
    message.set_kind(encode_preview_kind(value.kind()));
    message.set_preview(value.preview());
    if let Some(source_label) = value.source_label() {
        message.set_source_label(source_label);
    }
    message.set_pinned(value.pinned());
    message.set_unavailable(value.unavailable());
    Ok(message)
}

fn decode_previews(
    values: protobuf::RepeatedView<'_, generated::ClipPreview>,
) -> Result<Vec<ClipPreviewDto>, IpcError> {
    values.into_iter().map(decode_preview).collect()
}

fn decode_preview(value: generated::ClipPreviewView<'_>) -> Result<ClipPreviewDto, IpcError> {
    require(value.has_event_id(), "preview event ID is missing")?;
    require(
        value.has_capture_order(),
        "preview capture order is missing",
    )?;
    require(
        value.has_observed_at_utc_micros(),
        "preview timestamp is missing",
    )?;
    require(value.has_kind(), "preview kind is missing")?;
    require(value.has_preview(), "preview text is missing")?;
    require(value.has_pinned(), "preview pinned state is missing")?;
    require(
        value.has_unavailable(),
        "preview unavailable state is missing",
    )?;
    let event_id = clip_event_id(value.event_id())?;
    let capture_order = capture_order(value.capture_order())?;
    let observed_at = UtcUnixMicros::new(value.observed_at_utc_micros())
        .map_err(|_| IpcError::InvalidDto("preview timestamp is invalid"))?;
    let source_label = if value.has_source_label() {
        Some(proto_string(value.source_label())?)
    } else {
        None
    };
    ClipPreviewDto::new(
        event_id,
        capture_order,
        observed_at,
        decode_preview_kind(value.kind())?,
        proto_string(value.preview())?,
        source_label,
        value.pinned(),
        value.unavailable(),
    )
}

fn encode_preview_kind(value: ClipPreviewKind) -> generated::ClipPreviewKind {
    match value {
        ClipPreviewKind::Text => generated::ClipPreviewKind::Text,
        ClipPreviewKind::Code => generated::ClipPreviewKind::Code,
        ClipPreviewKind::Link => generated::ClipPreviewKind::Link,
        ClipPreviewKind::Image => generated::ClipPreviewKind::Image,
        ClipPreviewKind::Files => generated::ClipPreviewKind::Files,
        ClipPreviewKind::Unavailable => generated::ClipPreviewKind::Unavailable,
    }
}

fn decode_preview_kind(value: generated::ClipPreviewKind) -> Result<ClipPreviewKind, IpcError> {
    match i32::from(value) {
        1 => Ok(ClipPreviewKind::Text),
        2 => Ok(ClipPreviewKind::Code),
        3 => Ok(ClipPreviewKind::Link),
        4 => Ok(ClipPreviewKind::Image),
        5 => Ok(ClipPreviewKind::Files),
        6 => Ok(ClipPreviewKind::Unavailable),
        _ => Err(IpcError::InvalidDto("preview kind is zero or unknown")),
    }
}

fn protocol_error_message(value: &ProtocolErrorDto) -> generated::ProtocolError {
    let mut message = generated::ProtocolError::new();
    message.set_code(encode_error_code(value.code()));
    message.set_retryable(value.retryable());
    if let Some(detail) = value.developer_detail() {
        message.set_developer_detail(detail);
    }
    message
}

fn decode_protocol_error_view(
    value: generated::ProtocolErrorView<'_>,
) -> Result<ProtocolErrorDto, IpcError> {
    require(value.has_code(), "protocol error code is missing")?;
    require(value.has_retryable(), "protocol retryable state is missing")?;
    let detail = if value.has_developer_detail() {
        Some(proto_string(value.developer_detail())?)
    } else {
        None
    };
    ProtocolErrorDto::new(decode_error_code(value.code())?, value.retryable(), detail)
}

fn encode_error_code(value: ProtocolErrorCode) -> generated::ProtocolErrorCode {
    match value {
        ProtocolErrorCode::InvalidRequest => generated::ProtocolErrorCode::InvalidRequest,
        ProtocolErrorCode::UnsupportedVersion => generated::ProtocolErrorCode::UnsupportedVersion,
        ProtocolErrorCode::UnsupportedCapability => {
            generated::ProtocolErrorCode::UnsupportedCapability
        }
        ProtocolErrorCode::Unauthorized => generated::ProtocolErrorCode::Unauthorized,
        ProtocolErrorCode::ResourceLimit => generated::ProtocolErrorCode::ResourceLimit,
        ProtocolErrorCode::Internal => generated::ProtocolErrorCode::Internal,
    }
}

fn decode_error_code(value: generated::ProtocolErrorCode) -> Result<ProtocolErrorCode, IpcError> {
    match i32::from(value) {
        1 => Ok(ProtocolErrorCode::InvalidRequest),
        2 => Ok(ProtocolErrorCode::UnsupportedVersion),
        3 => Ok(ProtocolErrorCode::UnsupportedCapability),
        4 => Ok(ProtocolErrorCode::Unauthorized),
        5 => Ok(ProtocolErrorCode::ResourceLimit),
        6 => Ok(ProtocolErrorCode::Internal),
        _ => Err(IpcError::InvalidDto(
            "protocol error code is zero or unknown",
        )),
    }
}

fn correlation_id(bytes: &[u8]) -> Result<CorrelationId, IpcError> {
    CorrelationId::from_bytes(fixed_bytes::<16>(bytes, "correlation ID length")?)
}

fn clip_event_id(bytes: &[u8]) -> Result<ClipEventId, IpcError> {
    let bytes = fixed_bytes::<16>(bytes, "clip event ID length")?;
    ClipEventId::from_uuid(Uuid::from_bytes(bytes))
        .map_err(|_| IpcError::InvalidDto("clip event ID is not UUIDv4"))
}

fn capture_order(value: u64) -> Result<CaptureOrder, IpcError> {
    CaptureOrder::new(value).map_err(|_| IpcError::InvalidDto("capture order must be nonzero"))
}

fn fixed_bytes<const N: usize>(bytes: &[u8], reason: &'static str) -> Result<[u8; N], IpcError> {
    bytes.try_into().map_err(|_| IpcError::InvalidDto(reason))
}

fn proto_string(value: &protobuf::ProtoStr) -> Result<String, IpcError> {
    value
        .to_str()
        .map(str::to_owned)
        .map_err(|_| IpcError::InvalidDto("schema string is invalid UTF-8"))
}

fn serialize<M: protobuf::Message>(message: &M) -> Result<Vec<u8>, IpcError> {
    message.serialize().map_err(schema_error)
}

fn ensure_control_body_bound(bytes: &[u8]) -> Result<(), IpcError> {
    let limit = usize::try_from(FrameLimits::default().max_control_body_bytes())
        .map_err(|_| IpcError::IntegerOverflow)?;
    if bytes.len() > limit {
        return Err(IpcError::InvalidDto("schema body exceeds control limit"));
    }
    Ok(())
}

fn require(condition: bool, reason: &'static str) -> Result<(), IpcError> {
    if condition {
        Ok(())
    } else {
        Err(IpcError::InvalidDto(reason))
    }
}

fn schema_error(error: impl core::fmt::Display) -> IpcError {
    IpcError::Schema(error.to_string())
}
