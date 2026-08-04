use core::fmt;
use std::{
    io::{self, Write},
    num::NonZeroUsize,
    path::PathBuf,
    time::{Duration, Instant},
};

use pastral_ipc_auth::NonceReplayCache;
use pastral_ipc_core::{
    Capability, ClipPreviewDto, ClipPreviewKind, CorrelationId, Frame, FrameHeader, FrameKind,
    FrameLimits, HealthRequestDto, HealthResponseDto, HistoryPageResponseDto, ProtocolErrorCode,
    ProtocolErrorDto, RequestDto, ResponseDto, SearchResponseDto,
};
use pastral_ipc_schema::{decode_request, encode_response};
use pastral_ipc_win::{
    PipeFrameStream, PipeSecurity, PipeServer, TransportMaterial, build_logon_sid_pipe_security,
    create_first_pipe_server, current_token_identity, derive_pipe_name, inspect_pipe_security,
    load_or_create_transport_material, server_handshake_with_capabilities,
};
use pastral_storage::{ClipListItem, ClipPage, StorageError};

use crate::{health::open_storage, load_health_snapshot};

const MAX_CONNECTIONS: usize = 16;
const EXPECTED_PIPE_ACCESS_MASK: u32 = 0xc010_0000;
const HEALTH_CAPABILITIES: [Capability; 1] = [Capability::Health];
const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIpcError {
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    InvalidConfiguration,
    AgentHealth,
    Material,
    Transport,
    Authentication,
    Protocol,
}

impl AgentIpcError {
    fn io(operation: &'static str, error: &io::Error) -> Self {
        Self::Io {
            operation,
            kind: error.kind(),
        }
    }
}

impl fmt::Display for AgentIpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, kind } => write!(formatter, "{operation} failed: {kind:?}"),
            Self::InvalidConfiguration => formatter.write_str("invalid IPC Health configuration"),
            Self::AgentHealth => formatter.write_str("agent Health verification failed"),
            Self::Material => formatter.write_str("IPC transport material failed"),
            Self::Transport => formatter.write_str("IPC transport operation failed"),
            Self::Authentication => formatter.write_str("IPC authentication failed"),
            Self::Protocol => formatter.write_str("IPC Health protocol failed"),
        }
    }
}

impl std::error::Error for AgentIpcError {}

pub struct HealthServerConfig {
    data_root: PathBuf,
    max_connections: NonZeroUsize,
    connect_timeout: Duration,
    operation_timeout: Duration,
    write_summary: bool,
}

impl HealthServerConfig {
    pub fn new(
        data_root: PathBuf,
        max_connections: NonZeroUsize,
        connect_timeout: Duration,
        operation_timeout: Duration,
    ) -> Result<Self, AgentIpcError> {
        if data_root.as_os_str().is_empty()
            || max_connections.get() > MAX_CONNECTIONS
            || connect_timeout.is_zero()
            || operation_timeout.is_zero()
        {
            return Err(AgentIpcError::InvalidConfiguration);
        }
        Ok(Self {
            data_root,
            max_connections,
            connect_timeout,
            operation_timeout,
            write_summary: true,
        })
    }

    #[must_use]
    pub fn data_root(&self) -> &std::path::Path {
        &self.data_root
    }

    #[must_use]
    pub const fn max_connections(&self) -> NonZeroUsize {
        self.max_connections
    }

    #[must_use]
    pub const fn without_summary(mut self) -> Self {
        self.write_summary = false;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthServerSummary {
    connections_served: usize,
    session_id: u32,
}

impl HealthServerSummary {
    #[must_use]
    pub const fn connections_served(self) -> usize {
        self.connections_served
    }

    #[must_use]
    pub const fn session_id(self) -> u32 {
        self.session_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerMode {
    HealthOnly,
    ReadOnly,
}

impl ServerMode {
    const fn capabilities(self) -> &'static [Capability] {
        match self {
            Self::HealthOnly => &HEALTH_CAPABILITIES,
            Self::ReadOnly => &READ_CAPABILITIES,
        }
    }
}

pub fn serve_health<W: Write>(
    config: HealthServerConfig,
    output: &mut W,
) -> Result<HealthServerSummary, AgentIpcError> {
    serve(config, output, ServerMode::HealthOnly)
}

pub fn serve_read<W: Write>(
    config: HealthServerConfig,
    output: &mut W,
) -> Result<HealthServerSummary, AgentIpcError> {
    serve(config, output, ServerMode::ReadOnly)
}

fn serve<W: Write>(
    config: HealthServerConfig,
    output: &mut W,
    mode: ServerMode,
) -> Result<HealthServerSummary, AgentIpcError> {
    let _initial_snapshot =
        load_health_snapshot(config.data_root()).map_err(|_| AgentIpcError::AgentHealth)?;
    let material = load_or_create_transport_material(config.data_root())
        .map_err(|_| AgentIpcError::Material)?;
    let current = current_token_identity().map_err(|_| AgentIpcError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| AgentIpcError::Material)?;
    let security = build_logon_sid_pipe_security(&current).map_err(|_| AgentIpcError::Transport)?;
    verify_security(&security)?;
    let mut server =
        create_first_pipe_server(&name, &security).map_err(|_| AgentIpcError::Transport)?;

    write_marker(output, "agent-ipc-ready=1")?;

    let mut replay_cache = NonceReplayCache::new(64).map_err(|_| AgentIpcError::Authentication)?;
    let mut served = 0usize;
    loop {
        serve_one(
            server,
            &material,
            &mut replay_cache,
            config.data_root(),
            config.connect_timeout,
            config.operation_timeout,
            mode,
        )?;
        served += 1;
        if served >= config.max_connections.get() {
            break;
        }
        server =
            create_first_pipe_server(&name, &security).map_err(|_| AgentIpcError::Transport)?;
    }

    if config.write_summary {
        write_marker(output, &format!("agent-ipc-connections-served={served}"))?;
    }
    Ok(HealthServerSummary {
        connections_served: served,
        session_id: current.session_id(),
    })
}

fn verify_security(security: &PipeSecurity) -> Result<(), AgentIpcError> {
    let inspection = inspect_pipe_security(security).map_err(|_| AgentIpcError::Transport)?;
    if !inspection.dacl_present()
        || inspection.dacl_defaulted()
        || !inspection.dacl_protected()
        || inspection.ace_count() != 1
        || inspection.allow_ace_count() != 1
        || !inspection.exact_logon_sid_match()
        || inspection.access_mask() != EXPECTED_PIPE_ACCESS_MASK
    {
        return Err(AgentIpcError::Transport);
    }
    Ok(())
}

fn serve_one(
    mut server: PipeServer,
    material: &TransportMaterial,
    replay_cache: &mut NonceReplayCache,
    data_root: &std::path::Path,
    connect_timeout: Duration,
    operation_timeout: Duration,
    mode: ServerMode,
) -> Result<(), AgentIpcError> {
    server
        .connect(Instant::now() + connect_timeout)
        .map_err(|_| AgentIpcError::Transport)?;
    let peer = server
        .peer_identity()
        .map_err(|_| AgentIpcError::Transport)?;
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let authenticated = server_handshake_with_capabilities(
        stream,
        material,
        peer,
        replay_cache,
        mode.capabilities(),
        Instant::now() + operation_timeout,
    )
    .map_err(|_| AgentIpcError::Authentication)?;
    let mut stream = authenticated.into_stream();
    let request = stream
        .read_frame(Instant::now() + operation_timeout)
        .map_err(|_| AgentIpcError::Transport)?;
    if request.header().kind() != FrameKind::ControlProto
        || request.header().correlation().is_zero()
    {
        return Err(AgentIpcError::Protocol);
    }

    let correlation = request.header().correlation();
    let request = match decode_request(request.body()) {
        Ok(request) => request,
        Err(_) if mode == ServerMode::ReadOnly => {
            let response = error_response(ProtocolErrorCode::InvalidRequest, false)?;
            let frame = response_frame(&response, correlation)?;
            return stream
                .write_frame(&frame, Instant::now() + operation_timeout)
                .map_err(|_| AgentIpcError::Transport);
        }
        Err(_) => return Err(AgentIpcError::Protocol),
    };
    let response = response_for_request(mode, data_root, request)?;
    let frame = response_frame(&response, correlation)?;
    stream
        .write_frame(&frame, Instant::now() + operation_timeout)
        .map_err(|_| AgentIpcError::Transport)
}

fn response_for_request(
    mode: ServerMode,
    data_root: &std::path::Path,
    request: RequestDto,
) -> Result<ResponseDto, AgentIpcError> {
    match (mode, request) {
        (_, RequestDto::Health(HealthRequestDto)) => health_response(data_root),
        (ServerMode::HealthOnly, _) => Err(AgentIpcError::Protocol),
        (ServerMode::ReadOnly, RequestDto::HistoryPage(request)) => {
            let storage = match open_storage(data_root) {
                Ok(storage) => storage,
                Err(_) => return error_response(ProtocolErrorCode::Internal, true),
            };
            let limit = usize::try_from(request.limit()).map_err(|_| AgentIpcError::Protocol)?;
            match storage.history_page(request.before_capture_order(), limit) {
                Ok(page) => Ok(ResponseDto::HistoryPage(
                    HistoryPageResponseDto::new(map_page(&page)?, page.has_more())
                        .map_err(|_| AgentIpcError::Protocol)?,
                )),
                Err(error) => storage_error_response(&error),
            }
        }
        (ServerMode::ReadOnly, RequestDto::Search(request)) => {
            let storage = match open_storage(data_root) {
                Ok(storage) => storage,
                Err(_) => return error_response(ProtocolErrorCode::Internal, true),
            };
            let limit = usize::try_from(request.limit()).map_err(|_| AgentIpcError::Protocol)?;
            match storage.search_page(request.query(), limit) {
                Ok(page) => Ok(ResponseDto::Search(
                    SearchResponseDto::new(map_page(&page)?, page.has_more())
                        .map_err(|_| AgentIpcError::Protocol)?,
                )),
                Err(error) => storage_error_response(&error),
            }
        }
    }
}

fn health_response(data_root: &std::path::Path) -> Result<ResponseDto, AgentIpcError> {
    let snapshot = load_health_snapshot(data_root).map_err(|_| AgentIpcError::AgentHealth)?;
    Ok(ResponseDto::Health(
        HealthResponseDto::new(
            snapshot.storage_schema_version(),
            snapshot.capture_enabled(),
            snapshot.privacy_policy_ok(),
            snapshot.storage_integrity_ok(),
        )
        .map_err(|_| AgentIpcError::Protocol)?,
    ))
}

fn map_page(page: &ClipPage) -> Result<Vec<ClipPreviewDto>, AgentIpcError> {
    page.items().iter().map(map_item).collect()
}

fn map_item(item: &ClipListItem) -> Result<ClipPreviewDto, AgentIpcError> {
    let (kind, preview, unavailable) = match item.preview() {
        Some(preview) => (ClipPreviewKind::Text, preview.to_owned(), false),
        None => (ClipPreviewKind::Unavailable, String::new(), true),
    };
    ClipPreviewDto::new(
        item.clip_event_id(),
        item.capture_order(),
        item.observed_at(),
        kind,
        preview,
        None,
        false,
        unavailable,
    )
    .map_err(|_| AgentIpcError::Protocol)
}

fn storage_error_response(error: &StorageError) -> Result<ResponseDto, AgentIpcError> {
    match error {
        StorageError::SearchQueryInvalid(_) => {
            error_response(ProtocolErrorCode::InvalidRequest, false)
        }
        StorageError::IntegerOutOfRange(_) => {
            error_response(ProtocolErrorCode::ResourceLimit, false)
        }
        _ => error_response(ProtocolErrorCode::Internal, true),
    }
}

fn error_response(code: ProtocolErrorCode, retryable: bool) -> Result<ResponseDto, AgentIpcError> {
    Ok(ResponseDto::Error(
        ProtocolErrorDto::new(code, retryable, None).map_err(|_| AgentIpcError::Protocol)?,
    ))
}

fn response_frame(
    response: &ResponseDto,
    correlation: CorrelationId,
) -> Result<Frame, AgentIpcError> {
    let body = encode_response(response).map_err(|_| AgentIpcError::Protocol)?;
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).map_err(|_| AgentIpcError::Protocol)?,
        0,
        correlation,
        FrameLimits::default(),
    )
    .map_err(|_| AgentIpcError::Protocol)?;
    Frame::new(header, body).map_err(|_| AgentIpcError::Protocol)
}

fn write_marker<W: Write>(output: &mut W, marker: &str) -> Result<(), AgentIpcError> {
    writeln!(output, "{marker}").map_err(|error| AgentIpcError::io("write IPC marker", &error))?;
    output
        .flush()
        .map_err(|error| AgentIpcError::io("flush IPC marker", &error))
}
