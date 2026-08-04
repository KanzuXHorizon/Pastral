use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_ipc_core::{CorrelationId, FrameLimits, HealthRequestDto, RequestDto, ResponseDto};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, client_handshake, current_token_identity, derive_pipe_name,
    load_or_create_transport_material, open_pipe_client, random_bytes,
};

use crate::{AdmissionError, protocol::control_frame};

const SERVER_CHILD_FLAG: &str = "--server-child";
const DATA_ROOT_FLAG: &str = "--data-root";
const READINESS_LINE: &str = "agent-health-server-ready=ok\n";
const READINESS_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_READINESS_BYTES: usize = 64;

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn create() -> Result<Self, AdmissionError> {
        let random = random_bytes::<16>().map_err(|_| AdmissionError::Environment)?;
        let mut suffix = String::with_capacity(32);
        for byte in random {
            use core::fmt::Write;
            write!(&mut suffix, "{byte:02x}").map_err(|_| AdmissionError::Environment)?;
        }
        let path = env::temp_dir().join(format!(
            "pastral-agent-ipc-admission-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).map_err(|_| AdmissionError::Environment)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn cleanup(self) -> Result<(), AdmissionError> {
        fs::remove_dir_all(&self.0).map_err(|_| AdmissionError::Cleanup)
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn run_parent<W: Write>(mut output: W) -> Result<(), AdmissionError> {
    let total_start = Instant::now();
    let root = TemporaryRoot::create()?;
    let material =
        load_or_create_transport_material(root.path()).map_err(|_| AdmissionError::Material)?;
    let current = current_token_identity().map_err(|_| AdmissionError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| AdmissionError::Material)?;
    let executable = env::current_exe().map_err(|_| AdmissionError::Process)?;
    let mut child = Command::new(executable)
        .arg(SERVER_CHILD_FLAG)
        .arg(DATA_ROOT_FLAG)
        .arg(root.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AdmissionError::Process)?;
    let server_process_id = child.id();
    wait_for_readiness(&mut child, READINESS_LINE, READINESS_TIMEOUT)?;

    let connect_start = Instant::now();
    let client = open_pipe_client(&name, Instant::now() + CONNECT_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)?;
    let peer = client
        .peer_identity()
        .map_err(|_| AdmissionError::Transport)?;
    if peer.process_id() != server_process_id {
        return fail_child(&mut child, AdmissionError::Protocol);
    }
    let connect_elapsed = connect_start.elapsed();

    let handshake_start = Instant::now();
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated =
        client_handshake(stream, &material, peer, Instant::now() + OPERATION_TIMEOUT)
            .map_err(|_| AdmissionError::Authentication)?;
    let handshake_elapsed = handshake_start.elapsed();

    let health_start = Instant::now();
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&RequestDto::Health(HealthRequestDto))
        .map_err(|_| AdmissionError::Protocol)?;
    stream
        .write_frame(
            &control_frame(body, correlation)?,
            Instant::now() + OPERATION_TIMEOUT,
        )
        .map_err(|_| AdmissionError::Transport)?;
    let response = stream
        .read_frame(Instant::now() + OPERATION_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)?;
    if response.header().correlation() != correlation {
        return fail_child(&mut child, AdmissionError::Protocol);
    }
    match decode_response(response.body()).map_err(|_| AdmissionError::Protocol)? {
        ResponseDto::Health(value)
            if value.storage_schema_version() > 0
                && !value.capture_enabled()
                && value.privacy_policy_ok()
                && value.storage_integrity_ok() => {}
        _ => return fail_child(&mut child, AdmissionError::Protocol),
    }
    let health_elapsed = health_start.elapsed();
    drop(stream);

    let result = child
        .wait_with_output()
        .map_err(|_| AdmissionError::Process)?;
    if !result.status.success() || !result.stderr.is_empty() {
        return Err(AdmissionError::ChildFailure);
    }
    let total_elapsed = total_start.elapsed();
    let session_id = current.session_id();
    root.cleanup()?;

    writeln!(output, "agent-ipc-admission=ok")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "cross-process=true")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "health=ok")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "client-pid={}", std::process::id())
        .map_err(|error| AdmissionError::io("write client PID", &error))?;
    writeln!(output, "server-pid={server_process_id}")
        .map_err(|error| AdmissionError::io("write server PID", &error))?;
    writeln!(output, "session-id={session_id}")
        .map_err(|error| AdmissionError::io("write session ID", &error))?;
    writeln!(output, "connect-us={}", connect_elapsed.as_micros())
        .map_err(|error| AdmissionError::io("write connect metric", &error))?;
    writeln!(output, "handshake-us={}", handshake_elapsed.as_micros())
        .map_err(|error| AdmissionError::io("write handshake metric", &error))?;
    writeln!(output, "health-us={}", health_elapsed.as_micros())
        .map_err(|error| AdmissionError::io("write Health metric", &error))?;
    writeln!(output, "total-us={}", total_elapsed.as_micros())
        .map_err(|error| AdmissionError::io("write total metric", &error))?;
    Ok(())
}

fn wait_for_readiness(
    child: &mut Child,
    expected: &'static str,
    timeout: Duration,
) -> Result<(), AdmissionError> {
    let stdout = child.stdout.take().ok_or(AdmissionError::Readiness)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = read_readiness_line(stdout);
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(timeout) {
        Ok(Ok(line)) if line == expected => Ok(()),
        Ok(_) => fail_child(child, AdmissionError::Readiness),
        Err(mpsc::RecvTimeoutError::Timeout) => fail_child(child, AdmissionError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => fail_child(child, AdmissionError::Readiness),
    }
}

fn read_readiness_line(stdout: ChildStdout) -> Result<String, AdmissionError> {
    let mut reader = BufReader::new(stdout);
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(u64::try_from(MAX_READINESS_BYTES + 1).expect("small bound fits u64"))
        .read_until(b'\n', &mut bytes)
        .map_err(|error| AdmissionError::io("read child readiness", &error))?;
    if bytes.is_empty() || bytes.len() > MAX_READINESS_BYTES {
        return Err(AdmissionError::Readiness);
    }
    String::from_utf8(bytes).map_err(|_| AdmissionError::Readiness)
}

fn fail_child<T>(child: &mut Child, error: AdmissionError) -> Result<T, AdmissionError> {
    let _ = child.kill();
    let _ = child.wait();
    Err(error)
}
