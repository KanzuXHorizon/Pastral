use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_ipc_core::{
    Capability, CorrelationId, FrameLimits, HealthRequestDto, HistoryPageRequestDto, RequestDto,
    ResponseDto, SearchRequestDto,
};
use pastral_ipc_schema::{decode_response, encode_request};
use pastral_ipc_win::{
    PipeFrameStream, PipeName, TransportMaterial, client_handshake,
    client_handshake_with_capabilities, current_token_identity, derive_pipe_name,
    load_or_create_transport_material, open_pipe_client, process_memory_snapshot, random_bytes,
};

use crate::{AdmissionError, calculate_footprint, enforce_footprint, protocol::control_frame};

const BASELINE_CHILD_FLAG: &str = "--baseline-child";
const SERVER_CHILD_FLAG: &str = "--server-child";
const READ_SERVER_CHILD_FLAG: &str = "--read-server-child";
const DATA_ROOT_FLAG: &str = "--data-root";
const BASELINE_READINESS_LINE: &str = "agent-baseline-ready=ok\n";
const SERVER_READINESS_LINE: &str = "agent-ipc-ready=1\n";
const READINESS_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_READINESS_BYTES: usize = 64;
const READ_CAPABILITIES: [Capability; 3] = [
    Capability::Health,
    Capability::HistoryPage,
    Capability::Search,
];

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

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn(
        executable: &Path,
        mode: &str,
        data_root: &Path,
        stdin: Stdio,
    ) -> Result<Self, AdmissionError> {
        let child = Command::new(executable)
            .arg(mode)
            .arg(DATA_ROOT_FLAG)
            .arg(data_root)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| AdmissionError::Process)?;
        Ok(Self(Some(child)))
    }

    fn child_mut(&mut self) -> Result<&mut Child, AdmissionError> {
        self.0.as_mut().ok_or(AdmissionError::Process)
    }

    fn id(&self) -> Result<u32, AdmissionError> {
        self.0
            .as_ref()
            .map(Child::id)
            .ok_or(AdmissionError::Process)
    }

    fn wait_with_output(mut self) -> Result<Output, AdmissionError> {
        self.0
            .take()
            .ok_or(AdmissionError::Process)?
            .wait_with_output()
            .map_err(|_| AdmissionError::Process)
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

    let mut baseline = ChildGuard::spawn(
        &executable,
        BASELINE_CHILD_FLAG,
        root.path(),
        Stdio::piped(),
    )?;
    wait_for_readiness(
        baseline.child_mut()?,
        BASELINE_READINESS_LINE,
        READINESS_TIMEOUT,
    )?;
    let baseline_memory =
        process_memory_snapshot(baseline.id()?).map_err(|_| AdmissionError::InvalidMetric)?;
    drop(
        baseline
            .child_mut()?
            .stdin
            .take()
            .ok_or(AdmissionError::Process)?,
    );
    let baseline_output = baseline.wait_with_output()?;
    if !baseline_output.status.success() || !baseline_output.stderr.is_empty() {
        return Err(AdmissionError::ChildFailure);
    }

    let mut server = ChildGuard::spawn(&executable, SERVER_CHILD_FLAG, root.path(), Stdio::null())?;
    let server_process_id = server.id()?;
    wait_for_readiness(
        server.child_mut()?,
        SERVER_READINESS_LINE,
        READINESS_TIMEOUT,
    )?;
    let server_memory =
        process_memory_snapshot(server_process_id).map_err(|_| AdmissionError::InvalidMetric)?;

    let connect_start = Instant::now();
    let client = open_pipe_client(&name, Instant::now() + CONNECT_TIMEOUT)
        .map_err(|_| AdmissionError::Transport)?;
    let peer = client
        .peer_identity()
        .map_err(|_| AdmissionError::Transport)?;
    if peer.process_id() != server_process_id {
        return Err(AdmissionError::Protocol);
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
        return Err(AdmissionError::Protocol);
    }
    match decode_response(response.body()).map_err(|_| AdmissionError::Protocol)? {
        ResponseDto::Health(value)
            if value.storage_schema_version() > 0
                && !value.capture_enabled()
                && value.privacy_policy_ok()
                && value.storage_integrity_ok() => {}
        _ => return Err(AdmissionError::Protocol),
    }
    let health_elapsed = health_start.elapsed();
    drop(stream);

    let server_output = server.wait_with_output()?;
    if !server_output.status.success() || !server_output.stderr.is_empty() {
        return Err(AdmissionError::ChildFailure);
    }

    let executable_directory = executable
        .parent()
        .ok_or(AdmissionError::MissingReleaseArtifact)?;
    let default_agent = executable_directory.join("pastral-agent.exe");
    let default_agent_binary_bytes = fs::metadata(default_agent)
        .map_err(|_| AdmissionError::MissingReleaseArtifact)?
        .len();
    let admission_binary_bytes = fs::metadata(&executable)
        .map_err(|_| AdmissionError::MissingReleaseArtifact)?
        .len();
    let footprint = calculate_footprint(
        default_agent_binary_bytes,
        admission_binary_bytes,
        baseline_memory.working_set_bytes(),
        baseline_memory.private_usage_bytes(),
        server_memory.working_set_bytes(),
        server_memory.private_usage_bytes(),
    )?;
    let ceiling_state = if cfg!(debug_assertions) {
        "debug-not-enforced"
    } else {
        enforce_footprint(footprint, server_memory.private_usage_bytes())?;
        "passed"
    };

    let total_elapsed = total_start.elapsed();
    let session_id = current.session_id();
    root.cleanup()?;

    writeln!(output, "agent-ipc-admission=ok")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "cross-process=true")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "health=ok")
        .map_err(|error| AdmissionError::io("write parent result", &error))?;
    writeln!(output, "admission-ceilings={ceiling_state}")
        .map_err(|error| AdmissionError::io("write ceiling state", &error))?;
    writeln!(output, "client-pid={}", std::process::id())
        .map_err(|error| AdmissionError::io("write client PID", &error))?;
    writeln!(output, "server-pid={server_process_id}")
        .map_err(|error| AdmissionError::io("write server PID", &error))?;
    writeln!(output, "session-id={session_id}")
        .map_err(|error| AdmissionError::io("write session ID", &error))?;
    writeln!(
        output,
        "default-agent-binary-bytes={default_agent_binary_bytes}"
    )
    .map_err(|error| AdmissionError::io("write agent binary metric", &error))?;
    writeln!(output, "admission-binary-bytes={admission_binary_bytes}")
        .map_err(|error| AdmissionError::io("write admission binary metric", &error))?;
    writeln!(
        output,
        "binary-delta-bytes={}",
        footprint.binary_delta_bytes()
    )
    .map_err(|error| AdmissionError::io("write binary delta", &error))?;
    writeln!(
        output,
        "baseline-working-set-bytes={}",
        baseline_memory.working_set_bytes()
    )
    .map_err(|error| AdmissionError::io("write baseline working set", &error))?;
    writeln!(
        output,
        "baseline-private-bytes={}",
        baseline_memory.private_usage_bytes()
    )
    .map_err(|error| AdmissionError::io("write baseline private usage", &error))?;
    writeln!(
        output,
        "server-working-set-bytes={}",
        server_memory.working_set_bytes()
    )
    .map_err(|error| AdmissionError::io("write server working set", &error))?;
    writeln!(
        output,
        "server-private-bytes={}",
        server_memory.private_usage_bytes()
    )
    .map_err(|error| AdmissionError::io("write server private usage", &error))?;
    writeln!(
        output,
        "working-set-delta-bytes={}",
        footprint.working_set_delta_bytes()
    )
    .map_err(|error| AdmissionError::io("write working-set delta", &error))?;
    writeln!(
        output,
        "private-delta-bytes={}",
        footprint.private_delta_bytes()
    )
    .map_err(|error| AdmissionError::io("write private delta", &error))?;
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

pub fn run_read_parent<W: Write>(mut output: W) -> Result<(), AdmissionError> {
    let root = TemporaryRoot::create()?;
    let material =
        load_or_create_transport_material(root.path()).map_err(|_| AdmissionError::Material)?;
    let current = current_token_identity().map_err(|_| AdmissionError::Transport)?;
    let name = derive_pipe_name(material.identity(), current.session_id())
        .map_err(|_| AdmissionError::Material)?;
    let executable = env::current_exe().map_err(|_| AdmissionError::Process)?;

    let mut server = ChildGuard::spawn(
        &executable,
        READ_SERVER_CHILD_FLAG,
        root.path(),
        Stdio::null(),
    )?;
    let server_process_id = server.id()?;
    wait_for_readiness(
        server.child_mut()?,
        SERVER_READINESS_LINE,
        READINESS_TIMEOUT,
    )?;

    match read_request(
        &name,
        &material,
        server_process_id,
        RequestDto::Health(HealthRequestDto),
    )? {
        ResponseDto::Health(value)
            if value.storage_schema_version() > 0
                && !value.capture_enabled()
                && value.privacy_policy_ok()
                && value.storage_integrity_ok() => {}
        _ => return Err(AdmissionError::Protocol),
    }

    let history = HistoryPageRequestDto::new(10, None).map_err(|_| AdmissionError::Protocol)?;
    match read_request(
        &name,
        &material,
        server_process_id,
        RequestDto::HistoryPage(history),
    )? {
        ResponseDto::HistoryPage(value) if value.items().is_empty() && !value.has_more() => {}
        _ => return Err(AdmissionError::Protocol),
    }

    let search =
        SearchRequestDto::new("read probe".to_owned(), 10).map_err(|_| AdmissionError::Protocol)?;
    match read_request(
        &name,
        &material,
        server_process_id,
        RequestDto::Search(search),
    )? {
        ResponseDto::Search(value) if value.items().is_empty() && !value.has_more() => {}
        _ => return Err(AdmissionError::Protocol),
    }

    let server_output = server.wait_with_output()?;
    if !server_output.status.success()
        || !server_output.stdout.is_empty()
        || !server_output.stderr.is_empty()
    {
        return Err(AdmissionError::ChildFailure);
    }

    let session_id = current.session_id();
    root.cleanup()?;
    writeln!(output, "agent-ipc-read=ok")
        .map_err(|error| AdmissionError::io("write read result", &error))?;
    writeln!(output, "cross-process=true")
        .map_err(|error| AdmissionError::io("write read result", &error))?;
    writeln!(output, "health=ok")
        .map_err(|error| AdmissionError::io("write read result", &error))?;
    writeln!(output, "history=ok")
        .map_err(|error| AdmissionError::io("write read result", &error))?;
    writeln!(output, "search=ok")
        .map_err(|error| AdmissionError::io("write read result", &error))?;
    writeln!(output, "client-pid={}", std::process::id())
        .map_err(|error| AdmissionError::io("write read client PID", &error))?;
    writeln!(output, "server-pid={server_process_id}")
        .map_err(|error| AdmissionError::io("write read server PID", &error))?;
    writeln!(output, "session-id={session_id}")
        .map_err(|error| AdmissionError::io("write read session ID", &error))?;
    Ok(())
}

fn read_request(
    name: &PipeName,
    material: &TransportMaterial,
    expected_server_process_id: u32,
    request: RequestDto,
) -> Result<ResponseDto, AdmissionError> {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    let client = open_pipe_client(name, deadline).map_err(|_| AdmissionError::Transport)?;
    let peer = client
        .peer_identity()
        .map_err(|_| AdmissionError::Transport)?;
    if peer.process_id() != expected_server_process_id {
        return Err(AdmissionError::Protocol);
    }
    let stream = PipeFrameStream::from_client(client, FrameLimits::default());
    let authenticated = client_handshake_with_capabilities(
        stream,
        material,
        peer,
        &READ_CAPABILITIES,
        Instant::now() + OPERATION_TIMEOUT,
    )
    .map_err(|_| AdmissionError::Authentication)?;
    if authenticated.capabilities() != READ_CAPABILITIES {
        return Err(AdmissionError::Protocol);
    }
    let mut stream = authenticated.into_stream();
    let correlation = CorrelationId::new_v4();
    let body = encode_request(&request).map_err(|_| AdmissionError::Protocol)?;
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
        return Err(AdmissionError::Protocol);
    }
    decode_response(response.body()).map_err(|_| AdmissionError::Protocol)
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
        Ok(_) => Err(AdmissionError::Readiness),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(AdmissionError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(AdmissionError::Readiness),
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
