use std::{
    path::Path,
    time::{Duration, Instant},
};

use pastral_ipc_core::{
    CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto, ProtocolErrorCode,
    RequestDto, ResponseDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, TransportError, client_handshake, current_token_identity, derive_pipe_name,
    load_transport_material, open_pipe_client,
};

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
