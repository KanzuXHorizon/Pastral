use std::{
    path::Path,
    time::{Duration, Instant},
};

use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    Capability, ClipPreviewDto, ClipPreviewKind, CorrelationId, Frame, FrameHeader, FrameKind,
    FrameLimits, HealthRequestDto, HistoryPageRequestDto, ProtocolErrorCode, RequestDto,
    ResponseDto, SearchRequestDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, TransportError, client_handshake, client_handshake_with_capabilities,
    current_token_identity, derive_pipe_name, load_transport_material, open_pipe_client,
};

const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerHealthStatus {
    Connected,
    Disconnected,
    Timeout,
    ProtocolMismatch,
    AuthenticationFailed,
    Unhealthy,
    InternalError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagerHealthSnapshot {
    status: ManagerHealthStatus,
    storage_schema_version: u32,
    capture_enabled: bool,
    privacy_policy_ok: bool,
    storage_integrity_ok: bool,
    server_process_id: u32,
    session_id: u32,
    connect_elapsed: Duration,
    handshake_elapsed: Duration,
    health_elapsed: Duration,
}

impl ManagerHealthSnapshot {
    fn failed(status: ManagerHealthStatus) -> Self {
        Self {
            status,
            storage_schema_version: 0,
            capture_enabled: false,
            privacy_policy_ok: false,
            storage_integrity_ok: false,
            server_process_id: 0,
            session_id: 0,
            connect_elapsed: Duration::ZERO,
            handshake_elapsed: Duration::ZERO,
            health_elapsed: Duration::ZERO,
        }
    }

    #[must_use]
    pub const fn status(self) -> ManagerHealthStatus {
        self.status
    }

    #[must_use]
    pub const fn storage_schema_version(self) -> u32 {
        self.storage_schema_version
    }

    #[must_use]
    pub const fn capture_enabled(self) -> bool {
        self.capture_enabled
    }

    #[must_use]
    pub const fn privacy_policy_ok(self) -> bool {
        self.privacy_policy_ok
    }

    #[must_use]
    pub const fn storage_integrity_ok(self) -> bool {
        self.storage_integrity_ok
    }

    #[must_use]
    pub const fn server_process_id(self) -> u32 {
        self.server_process_id
    }

    #[must_use]
    pub const fn session_id(self) -> u32 {
        self.session_id
    }

    #[must_use]
    pub const fn connect_elapsed(self) -> Duration {
        self.connect_elapsed
    }

    #[must_use]
    pub const fn handshake_elapsed(self) -> Duration {
        self.handshake_elapsed
    }

    #[must_use]
    pub const fn health_elapsed(self) -> Duration {
        self.health_elapsed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerClipKind {
    Text,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerClipItemSnapshot {
    event_id: ClipEventId,
    capture_order: CaptureOrder,
    observed_at: UtcUnixMicros,
    kind: ManagerClipKind,
    preview: String,
    source_label: Option<String>,
    pinned: bool,
    unavailable: bool,
    preview_truncated: bool,
}

impl ManagerClipItemSnapshot {
    #[must_use]
    pub const fn event_id(&self) -> ClipEventId {
        self.event_id
    }

    #[must_use]
    pub const fn capture_order(&self) -> CaptureOrder {
        self.capture_order
    }

    #[must_use]
    pub const fn observed_at(&self) -> UtcUnixMicros {
        self.observed_at
    }

    #[must_use]
    pub const fn kind(&self) -> ManagerClipKind {
        self.kind
    }

    #[must_use]
    pub fn preview(&self) -> &str {
        &self.preview
    }

    #[must_use]
    pub fn source_label(&self) -> Option<&str> {
        self.source_label.as_deref()
    }

    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }

    #[must_use]
    pub const fn unavailable(&self) -> bool {
        self.unavailable
    }

    #[must_use]
    pub const fn preview_truncated(&self) -> bool {
        self.preview_truncated
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerReadPageSnapshot {
    items: Vec<ManagerClipItemSnapshot>,
    has_more: bool,
    server_process_id: u32,
    session_id: u32,
    connect_elapsed: Duration,
    handshake_elapsed: Duration,
    request_elapsed: Duration,
}

impl ManagerReadPageSnapshot {
    #[must_use]
    pub fn items(&self) -> &[ManagerClipItemSnapshot] {
        &self.items
    }

    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.has_more
    }

    #[must_use]
    pub const fn server_process_id(&self) -> u32 {
        self.server_process_id
    }

    #[must_use]
    pub const fn session_id(&self) -> u32 {
        self.session_id
    }

    #[must_use]
    pub const fn connect_elapsed(&self) -> Duration {
        self.connect_elapsed
    }

    #[must_use]
    pub const fn handshake_elapsed(&self) -> Duration {
        self.handshake_elapsed
    }

    #[must_use]
    pub const fn request_elapsed(&self) -> Duration {
        self.request_elapsed
    }
}

pub fn query_health(data_root: &Path, timeout: Duration) -> ManagerHealthSnapshot {
    if data_root.as_os_str().is_empty() || timeout.is_zero() {
        return ManagerHealthSnapshot::failed(ManagerHealthStatus::InternalError);
    }

    let material = match load_transport_material(data_root) {
        Ok(material) => material,
        Err(error) => return ManagerHealthSnapshot::failed(material_error_status(&error)),
    };
    let current = match current_token_identity() {
        Ok(current) => current,
        Err(_) => return ManagerHealthSnapshot::failed(ManagerHealthStatus::InternalError),
    };
    let name = match derive_pipe_name(material.identity(), current.session_id()) {
        Ok(name) => name,
        Err(_) => return ManagerHealthSnapshot::failed(ManagerHealthStatus::InternalError),
    };

    let total_deadline = Instant::now() + timeout;
    let connect_start = Instant::now();
    let client = match open_pipe_client(&name, total_deadline) {
        Ok(client) => client,
        Err(error) => return ManagerHealthSnapshot::failed(connect_error_status(&error)),
    };
    let connect_elapsed = connect_start.elapsed();
    let peer = match client.peer_identity() {
        Ok(peer) => peer,
        Err(error) => return ManagerHealthSnapshot::failed(transport_error_status(&error)),
    };

    let handshake_start = Instant::now();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = match client_handshake(stream, &material, peer, total_deadline) {
        Ok(connection) => connection,
        Err(error) => return ManagerHealthSnapshot::failed(handshake_error_status(&error)),
    };
    let handshake_elapsed = handshake_start.elapsed();

    let health_start = Instant::now();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = match encode_request(&RequestDto::Health(HealthRequestDto)) {
        Ok(body) => body,
        Err(_) => return ManagerHealthSnapshot::failed(ManagerHealthStatus::InternalError),
    };
    let frame = match control_frame(body, correlation) {
        Ok(frame) => frame,
        Err(_) => return ManagerHealthSnapshot::failed(ManagerHealthStatus::InternalError),
    };
    if let Err(error) = stream.write_frame(&frame, total_deadline) {
        return ManagerHealthSnapshot::failed(transport_error_status(&error));
    }
    let response = match stream.read_frame(total_deadline) {
        Ok(response) => response,
        Err(error) => return ManagerHealthSnapshot::failed(transport_error_status(&error)),
    };
    let health_elapsed = health_start.elapsed();
    if response.header().kind() != FrameKind::ControlProto
        || response.header().correlation() != correlation
    {
        return ManagerHealthSnapshot::failed(ManagerHealthStatus::ProtocolMismatch);
    }

    match decode_response(response.body()) {
        Ok(ResponseDto::Health(value)) => {
            if value.storage_schema_version() == 0
                || !value.privacy_policy_ok()
                || !value.storage_integrity_ok()
            {
                return ManagerHealthSnapshot::failed(ManagerHealthStatus::Unhealthy);
            }
            ManagerHealthSnapshot {
                status: ManagerHealthStatus::Connected,
                storage_schema_version: value.storage_schema_version(),
                capture_enabled: value.capture_enabled(),
                privacy_policy_ok: value.privacy_policy_ok(),
                storage_integrity_ok: value.storage_integrity_ok(),
                server_process_id: peer.process_id(),
                session_id: peer.session_id(),
                connect_elapsed,
                handshake_elapsed,
                health_elapsed,
            }
        }
        Ok(ResponseDto::Error(error)) => {
            let status = match error.code() {
                ProtocolErrorCode::UnsupportedVersion
                | ProtocolErrorCode::UnsupportedCapability
                | ProtocolErrorCode::InvalidRequest => ManagerHealthStatus::ProtocolMismatch,
                ProtocolErrorCode::Unauthorized => ManagerHealthStatus::AuthenticationFailed,
                ProtocolErrorCode::ResourceLimit | ProtocolErrorCode::Internal => {
                    ManagerHealthStatus::InternalError
                }
            };
            ManagerHealthSnapshot::failed(status)
        }
        Ok(_) | Err(_) => ManagerHealthSnapshot::failed(ManagerHealthStatus::ProtocolMismatch),
    }
}

pub fn query_history(
    data_root: &Path,
    timeout: Duration,
    limit: u32,
    before_capture_order: Option<CaptureOrder>,
) -> Result<ManagerReadPageSnapshot, ManagerHealthStatus> {
    let request = HistoryPageRequestDto::new(limit, before_capture_order)
        .map_err(|_| ManagerHealthStatus::InternalError)?;
    let response = query_read(data_root, timeout, RequestDto::HistoryPage(request))?;
    match response.response {
        ResponseDto::HistoryPage(ref page) => response.page(page.items(), page.has_more()),
        ResponseDto::Error(error) => Err(protocol_error_status(error.code())),
        _ => Err(ManagerHealthStatus::ProtocolMismatch),
    }
}

pub fn query_search(
    data_root: &Path,
    timeout: Duration,
    query: &str,
    limit: u32,
) -> Result<ManagerReadPageSnapshot, ManagerHealthStatus> {
    let request = SearchRequestDto::new(query.to_owned(), limit)
        .map_err(|_| ManagerHealthStatus::InternalError)?;
    let response = query_read(data_root, timeout, RequestDto::Search(request))?;
    match response.response {
        ResponseDto::Search(ref page) => response.page(page.items(), page.has_more()),
        ResponseDto::Error(error) => Err(protocol_error_status(error.code())),
        _ => Err(ManagerHealthStatus::ProtocolMismatch),
    }
}

struct ReadResponse {
    response: ResponseDto,
    server_process_id: u32,
    session_id: u32,
    connect_elapsed: Duration,
    handshake_elapsed: Duration,
    request_elapsed: Duration,
}

impl ReadResponse {
    fn page(
        &self,
        items: &[ClipPreviewDto],
        has_more: bool,
    ) -> Result<ManagerReadPageSnapshot, ManagerHealthStatus> {
        let items = items
            .iter()
            .map(map_preview)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ManagerReadPageSnapshot {
            items,
            has_more,
            server_process_id: self.server_process_id,
            session_id: self.session_id,
            connect_elapsed: self.connect_elapsed,
            handshake_elapsed: self.handshake_elapsed,
            request_elapsed: self.request_elapsed,
        })
    }
}

fn query_read(
    data_root: &Path,
    timeout: Duration,
    request: RequestDto,
) -> Result<ReadResponse, ManagerHealthStatus> {
    if data_root.as_os_str().is_empty() || timeout.is_zero() {
        return Err(ManagerHealthStatus::InternalError);
    }

    let material =
        load_transport_material(data_root).map_err(|error| material_error_status(&error))?;
    let current = current_token_identity().map_err(|_| ManagerHealthStatus::InternalError)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| ManagerHealthStatus::InternalError)?;
    let total_deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(ManagerHealthStatus::InternalError)?;

    let connect_start = Instant::now();
    let client =
        open_pipe_client(&name, total_deadline).map_err(|error| connect_error_status(&error))?;
    let connect_elapsed = connect_start.elapsed();
    let peer = client
        .peer_identity()
        .map_err(|error| transport_error_status(&error))?;

    let handshake_start = Instant::now();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake_with_capabilities(
        stream,
        &material,
        peer,
        &READ_CAPABILITIES,
        total_deadline,
    )
    .map_err(|error| handshake_error_status(&error))?;
    if authenticated.capabilities() != READ_CAPABILITIES {
        return Err(ManagerHealthStatus::ProtocolMismatch);
    }
    let handshake_elapsed = handshake_start.elapsed();

    let request_start = Instant::now();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&request).map_err(|_| ManagerHealthStatus::InternalError)?;
    let frame = control_frame(body, correlation).map_err(|_| ManagerHealthStatus::InternalError)?;
    stream
        .write_frame(&frame, total_deadline)
        .map_err(|error| transport_error_status(&error))?;
    let response = stream
        .read_frame(total_deadline)
        .map_err(|error| transport_error_status(&error))?;
    let request_elapsed = request_start.elapsed();
    if response.header().kind() != FrameKind::ControlProto
        || response.header().correlation() != correlation
    {
        return Err(ManagerHealthStatus::ProtocolMismatch);
    }
    let response =
        decode_response(response.body()).map_err(|_| ManagerHealthStatus::ProtocolMismatch)?;

    Ok(ReadResponse {
        response,
        server_process_id: peer.process_id(),
        session_id: peer.session_id(),
        connect_elapsed,
        handshake_elapsed,
        request_elapsed,
    })
}

fn map_preview(value: &ClipPreviewDto) -> Result<ManagerClipItemSnapshot, ManagerHealthStatus> {
    let kind = match value.kind() {
        ClipPreviewKind::Text => ManagerClipKind::Text,
        ClipPreviewKind::Unavailable => ManagerClipKind::Unavailable,
        _ => return Err(ManagerHealthStatus::ProtocolMismatch),
    };
    if (kind == ManagerClipKind::Unavailable) != value.unavailable()
        || (value.unavailable() && !value.preview().is_empty())
    {
        return Err(ManagerHealthStatus::ProtocolMismatch);
    }
    Ok(ManagerClipItemSnapshot {
        event_id: value.event_id(),
        capture_order: value.capture_order(),
        observed_at: value.observed_at(),
        kind,
        preview: value.preview().to_owned(),
        source_label: value.source_label().map(str::to_owned),
        pinned: value.pinned(),
        unavailable: value.unavailable(),
        preview_truncated: false,
    })
}

fn protocol_error_status(code: ProtocolErrorCode) -> ManagerHealthStatus {
    match code {
        ProtocolErrorCode::UnsupportedVersion
        | ProtocolErrorCode::UnsupportedCapability
        | ProtocolErrorCode::InvalidRequest => ManagerHealthStatus::ProtocolMismatch,
        ProtocolErrorCode::Unauthorized => ManagerHealthStatus::AuthenticationFailed,
        ProtocolErrorCode::ResourceLimit | ProtocolErrorCode::Internal => {
            ManagerHealthStatus::InternalError
        }
    }
}

fn control_frame(body: Vec<u8>, correlation: CorrelationId) -> Result<Frame, ()> {
    let length = u32::try_from(body.len()).map_err(|_| ())?;
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        length,
        0,
        correlation,
        FrameLimits::default(),
    )
    .map_err(|_| ())?;
    Frame::new(header, body).map_err(|_| ())
}

fn material_error_status(error: &TransportError) -> ManagerHealthStatus {
    match error {
        TransportError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        } => ManagerHealthStatus::Disconnected,
        _ => ManagerHealthStatus::InternalError,
    }
}

fn connect_error_status(error: &TransportError) -> ManagerHealthStatus {
    match error {
        TransportError::Timeout("open named-pipe client") | TransportError::Disconnected => {
            ManagerHealthStatus::Disconnected
        }
        _ => transport_error_status(error),
    }
}

fn handshake_error_status(error: &TransportError) -> ManagerHealthStatus {
    match error {
        TransportError::Authentication(_) | TransportError::Disconnected => {
            ManagerHealthStatus::AuthenticationFailed
        }
        _ => transport_error_status(error),
    }
}

fn transport_error_status(error: &TransportError) -> ManagerHealthStatus {
    match error {
        TransportError::Authentication(_) => ManagerHealthStatus::AuthenticationFailed,
        TransportError::Timeout(_) => ManagerHealthStatus::Timeout,
        TransportError::Disconnected => ManagerHealthStatus::Disconnected,
        TransportError::Protocol(_) => ManagerHealthStatus::ProtocolMismatch,
        _ => ManagerHealthStatus::InternalError,
    }
}
