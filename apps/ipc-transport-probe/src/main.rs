use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    time::{Duration, Instant},
};

use pastral_ipc_auth::NonceReplayCache;
use pastral_ipc_core::{
    Capability, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits, HealthRequestDto,
    HealthResponseDto, RequestDto, ResponseDto,
};
use pastral_ipc_schema::{decode_request, decode_response, encode_request, encode_response};
use pastral_ipc_win::{
    PipeFrameStream, build_logon_sid_pipe_security, client_handshake, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material, open_pipe_client,
    random_bytes, server_handshake,
};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(5);
const CHILD_FLAG: &str = "--server-child";
const ROOT_FLAG: &str = "--root";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Parent,
    ServerChild { root: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeError {
    InvalidArguments,
    Environment,
    Material,
    Process,
    Transport,
    Authentication,
    Protocol,
    ChildFailure,
    Cleanup,
}

impl core::fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::InvalidArguments => "invalid arguments",
            Self::Environment => "environment setup failed",
            Self::Material => "transport material failed",
            Self::Process => "process operation failed",
            Self::Transport => "transport operation failed",
            Self::Authentication => "authentication failed",
            Self::Protocol => "protocol validation failed",
            Self::ChildFailure => "server child failed",
            Self::Cleanup => "cleanup failed",
        };
        formatter.write_str(text)
    }
}

impl std::error::Error for ProbeError {}

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn create() -> Result<Self, ProbeError> {
        let suffix = random_bytes::<16>().map_err(|_| ProbeError::Environment)?;
        let mut name = format!("pastral-ipc-transport-{}-", std::process::id());
        for byte in suffix {
            use core::fmt::Write;
            write!(&mut name, "{byte:02x}").map_err(|_| ProbeError::Environment)?;
        }
        let path = env::temp_dir().join(name);
        fs::create_dir(&path).map_err(|_| ProbeError::Environment)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn cleanup(self) -> Result<(), ProbeError> {
        fs::remove_dir_all(&self.0).map_err(|_| ProbeError::Cleanup)
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> ExitCode {
    let mode = match parse_arguments(env::args_os().skip(1)) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("ipc-transport-probe={error}");
            return ExitCode::from(2);
        }
    };
    let result = match mode {
        Mode::Parent => run_parent(),
        Mode::ServerChild { root } => run_server_child(&root),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ipc-transport-probe={error}");
            ExitCode::from(1)
        }
    }
}

fn parse_arguments(args: impl IntoIterator<Item = OsString>) -> Result<Mode, ProbeError> {
    let values = args.into_iter().collect::<Vec<_>>();
    match values.as_slice() {
        [] => Ok(Mode::Parent),
        [child, root_flag, root]
            if child == CHILD_FLAG && root_flag == ROOT_FLAG && !root.is_empty() =>
        {
            Ok(Mode::ServerChild {
                root: PathBuf::from(root),
            })
        }
        _ => Err(ProbeError::InvalidArguments),
    }
}

fn run_parent() -> Result<(), ProbeError> {
    let total_start = Instant::now();
    let root = TemporaryRoot::create()?;
    let material =
        load_or_create_transport_material(root.path()).map_err(|_| ProbeError::Material)?;
    let current = current_token_identity().map_err(|_| ProbeError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| ProbeError::Material)?;
    let executable = env::current_exe().map_err(|_| ProbeError::Process)?;
    let child = Command::new(executable)
        .arg(CHILD_FLAG)
        .arg(ROOT_FLAG)
        .arg(root.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ProbeError::Process)?;

    let connect_start = Instant::now();
    let client = open_pipe_client(&name, deadline()).map_err(|_| ProbeError::Transport)?;
    let server_peer = client.peer_identity().map_err(|_| ProbeError::Transport)?;
    if server_peer.process_id() != child.id() {
        return Err(ProbeError::Protocol);
    }
    let connect_elapsed = connect_start.elapsed();

    let handshake_start = Instant::now();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake(stream, &material, server_peer, deadline())
        .map_err(|_| ProbeError::Authentication)?;
    if authenticated.capabilities() != [Capability::Health] {
        return Err(ProbeError::Protocol);
    }
    let handshake_elapsed = handshake_start.elapsed();

    let health_start = Instant::now();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let request = RequestDto::Health(HealthRequestDto);
    let body = encode_request(&request).map_err(|_| ProbeError::Protocol)?;
    stream
        .write_frame(&control_frame(body, correlation)?, deadline())
        .map_err(|_| ProbeError::Transport)?;
    let response_frame = stream
        .read_frame(deadline())
        .map_err(|_| ProbeError::Transport)?;
    if response_frame.header().kind() != FrameKind::ControlProto
        || response_frame.header().correlation() != correlation
    {
        return Err(ProbeError::Protocol);
    }
    let response = decode_response(response_frame.body()).map_err(|_| ProbeError::Protocol)?;
    match response {
        ResponseDto::Health(value)
            if value.storage_schema_version() == 1
                && !value.capture_enabled()
                && value.privacy_policy_ok()
                && value.storage_integrity_ok() => {}
        _ => return Err(ProbeError::Protocol),
    }
    let health_elapsed = health_start.elapsed();
    drop(stream);

    let child_pid = child.id();
    let output = child.wait_with_output().map_err(|_| ProbeError::Process)?;
    if !output.status.success()
        || !String::from_utf8_lossy(&output.stdout).contains("ipc-transport-server=ok")
    {
        return Err(ProbeError::ChildFailure);
    }
    let session_id = current.session_id();
    let total_elapsed = total_start.elapsed();
    root.cleanup()?;

    println!("ipc-transport-probe=ok");
    println!("cross-process=true");
    println!("client-pid={}", std::process::id());
    println!("server-pid={child_pid}");
    println!("session-id={session_id}");
    println!("connect-us={}", connect_elapsed.as_micros());
    println!("handshake-us={}", handshake_elapsed.as_micros());
    println!("health-us={}", health_elapsed.as_micros());
    println!("total-us={}", total_elapsed.as_micros());
    Ok(())
}

fn run_server_child(root: &Path) -> Result<(), ProbeError> {
    let material = load_or_create_transport_material(root).map_err(|_| ProbeError::Material)?;
    let current = current_token_identity().map_err(|_| ProbeError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| ProbeError::Material)?;
    let security = build_logon_sid_pipe_security(&current).map_err(|_| ProbeError::Transport)?;
    let mut server =
        create_first_pipe_server(&name, &security).map_err(|_| ProbeError::Transport)?;
    server
        .connect(deadline())
        .map_err(|_| ProbeError::Transport)?;
    let client_peer = server.peer_identity().map_err(|_| ProbeError::Transport)?;
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let mut replay_cache = NonceReplayCache::new(64).map_err(|_| ProbeError::Authentication)?;
    let authenticated = server_handshake(
        stream,
        &material,
        client_peer,
        &mut replay_cache,
        deadline(),
    )
    .map_err(|_| ProbeError::Authentication)?;
    let mut stream = authenticated.into_stream();
    let request_frame = stream
        .read_frame(deadline())
        .map_err(|_| ProbeError::Transport)?;
    if request_frame.header().kind() != FrameKind::ControlProto
        || request_frame.header().correlation().is_zero()
    {
        return Err(ProbeError::Protocol);
    }
    let request = decode_request(request_frame.body()).map_err(|_| ProbeError::Protocol)?;
    if request != RequestDto::Health(HealthRequestDto) {
        return Err(ProbeError::Protocol);
    }
    let response = ResponseDto::Health(
        HealthResponseDto::new(1, false, true, true).map_err(|_| ProbeError::Protocol)?,
    );
    let body = encode_response(&response).map_err(|_| ProbeError::Protocol)?;
    stream
        .write_frame(
            &control_frame(body, request_frame.header().correlation())?,
            deadline(),
        )
        .map_err(|_| ProbeError::Transport)?;
    println!("ipc-transport-server=ok");
    Ok(())
}

fn control_frame(body: Vec<u8>, correlation: CorrelationId) -> Result<Frame, ProbeError> {
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).map_err(|_| ProbeError::Protocol)?,
        0,
        correlation,
        FrameLimits::default(),
    )
    .map_err(|_| ProbeError::Protocol)?;
    Frame::new(header, body).map_err(|_| ProbeError::Protocol)
}

fn deadline() -> Instant {
    Instant::now() + OPERATION_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_accepts_only_parent_or_exact_server_child_shape() {
        assert!(matches!(
            parse_arguments(Vec::<OsString>::new()).unwrap(),
            Mode::Parent
        ));
        assert!(matches!(
            parse_arguments([CHILD_FLAG.into(), ROOT_FLAG.into(), "C:\\temp".into()]).unwrap(),
            Mode::ServerChild { .. }
        ));
        for invalid in [
            vec!["--unknown".into()],
            vec![CHILD_FLAG.into()],
            vec![CHILD_FLAG.into(), ROOT_FLAG.into()],
            vec![CHILD_FLAG.into(), ROOT_FLAG.into(), OsString::new()],
            vec![
                CHILD_FLAG.into(),
                ROOT_FLAG.into(),
                "x".into(),
                "extra".into(),
            ],
        ] {
            assert_eq!(parse_arguments(invalid), Err(ProbeError::InvalidArguments));
        }
    }
}
