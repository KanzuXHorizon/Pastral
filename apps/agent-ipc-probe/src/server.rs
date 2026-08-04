use std::{
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use pastral_agent::load_health_snapshot;
use pastral_ipc_auth::NonceReplayCache;
use pastral_ipc_core::{FrameKind, FrameLimits, HealthRequestDto, RequestDto};
use pastral_ipc_schema::decode_request;
use pastral_ipc_win::{
    PipeFrameStream, build_logon_sid_pipe_security, create_first_pipe_server,
    current_token_identity, derive_pipe_name, inspect_pipe_security,
    load_or_create_transport_material, server_handshake,
};

use crate::{AdmissionError, protocol::health_response_frame};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);
const EXPECTED_PIPE_ACCESS_MASK: u32 = 0xc010_0000;

pub fn run_server_child<W: Write>(data_root: &Path, mut output: W) -> Result<(), AdmissionError> {
    let snapshot = load_health_snapshot(data_root).map_err(|_| AdmissionError::AgentHealth)?;
    let material =
        load_or_create_transport_material(data_root).map_err(|_| AdmissionError::Material)?;
    let current = current_token_identity().map_err(|_| AdmissionError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| AdmissionError::Material)?;
    let security =
        build_logon_sid_pipe_security(&current).map_err(|_| AdmissionError::Transport)?;
    let inspection = inspect_pipe_security(&security).map_err(|_| AdmissionError::Transport)?;
    if !inspection.dacl_present()
        || inspection.dacl_defaulted()
        || !inspection.dacl_protected()
        || inspection.ace_count() != 1
        || inspection.allow_ace_count() != 1
        || !inspection.exact_logon_sid_match()
        || inspection.access_mask() != EXPECTED_PIPE_ACCESS_MASK
    {
        return Err(AdmissionError::Transport);
    }
    let mut server =
        create_first_pipe_server(&name, &security).map_err(|_| AdmissionError::Transport)?;

    output
        .write_all(b"agent-health-server-ready=ok\n")
        .map_err(|error| AdmissionError::io("write server readiness", &error))?;
    output
        .flush()
        .map_err(|error| AdmissionError::io("flush server readiness", &error))?;

    server
        .connect(Instant::now() + CONNECT_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)?;
    let peer = server
        .peer_identity()
        .map_err(|_| AdmissionError::Transport)?;
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let mut replay_cache = NonceReplayCache::new(64).map_err(|_| AdmissionError::Authentication)?;
    let authenticated = server_handshake(
        stream,
        &material,
        peer,
        &mut replay_cache,
        Instant::now() + OPERATION_TIMEOUT,
    )
    .map_err(|_| AdmissionError::Authentication)?;
    let mut stream = authenticated.into_stream();
    let request = stream
        .read_frame(Instant::now() + OPERATION_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)?;
    if request.header().kind() != FrameKind::ControlProto
        || request.header().correlation().is_zero()
    {
        return Err(AdmissionError::Protocol);
    }
    if decode_request(request.body()).map_err(|_| AdmissionError::Protocol)?
        != RequestDto::Health(HealthRequestDto)
    {
        return Err(AdmissionError::Protocol);
    }
    let response = health_response_frame(&snapshot, request.header().correlation())?;
    stream
        .write_frame(&response, Instant::now() + OPERATION_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)
}
