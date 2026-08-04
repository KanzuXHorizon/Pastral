use core::fmt;
use std::{
    io::{self, Write},
    num::NonZeroUsize,
    path::PathBuf,
    time::{Duration, Instant},
};

use pastral_ipc_auth::NonceReplayCache;
use pastral_ipc_core::{
    CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto, HealthResponseDto,
    RequestDto, ResponseDto,
};
use pastral_ipc_schema::{decode_request, encode_response};
use pastral_ipc_win::{
    PipeFrameStream, PipeSecurity, PipeServer, TransportMaterial, build_logon_sid_pipe_security,
    create_first_pipe_server, current_token_identity, derive_pipe_name, inspect_pipe_security,
    load_or_create_transport_material, server_handshake,
};

use crate::{AgentHealthSnapshot, load_health_snapshot};

const MAX_CONNECTIONS: usize = 16;
const EXPECTED_PIPE_ACCESS_MASK: u32 = 0xc010_0000;

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

pub fn serve_health<W: Write>(
    config: HealthServerConfig,
    output: &mut W,
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
) -> Result<(), AgentIpcError> {
    server
        .connect(Instant::now() + connect_timeout)
        .map_err(|_| AgentIpcError::Transport)?;
    let peer = server
        .peer_identity()
        .map_err(|_| AgentIpcError::Transport)?;
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let authenticated = server_handshake(
        stream,
        material,
        peer,
        replay_cache,
        Instant::now() + operation_timeout,
    )
    .map_err(|_| AgentIpcError::Authentication)?;
    let mut stream = authenticated.into_stream();
    let request = stream
        .read_frame(Instant::now() + operation_timeout)
        .map_err(|_| AgentIpcError::Transport)?;
    if request.header().kind() != FrameKind::ControlProto
        || request.header().correlation().is_zero()
        || decode_request(request.body()).map_err(|_| AgentIpcError::Protocol)?
            != RequestDto::Health(HealthRequestDto)
    {
        return Err(AgentIpcError::Protocol);
    }

    let snapshot = load_health_snapshot(data_root).map_err(|_| AgentIpcError::AgentHealth)?;
    let response = health_response_frame(&snapshot, request.header().correlation())?;
    stream
        .write_frame(&response, Instant::now() + operation_timeout)
        .map_err(|_| AgentIpcError::Transport)
}

fn health_response_frame(
    snapshot: &AgentHealthSnapshot,
    correlation: CorrelationId,
) -> Result<Frame, AgentIpcError> {
    let response = ResponseDto::Health(
        HealthResponseDto::new(
            snapshot.storage_schema_version(),
            snapshot.capture_enabled(),
            snapshot.privacy_policy_ok(),
            snapshot.storage_integrity_ok(),
        )
        .map_err(|_| AgentIpcError::Protocol)?,
    );
    let body = encode_response(&response).map_err(|_| AgentIpcError::Protocol)?;
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
