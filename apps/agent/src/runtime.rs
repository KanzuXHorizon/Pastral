use core::num::{NonZeroU32, NonZeroUsize};
use std::{
    env,
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crate::health::open_storage;
use crate::{
    AgentCommand, AgentIdentity, AgentRuntimeError, PrivacyPolicyConfig, StorageCaptureSink,
    SystemClock, ThreadSleeper, WindowsClipboardSource, load_health_snapshot,
};
#[cfg(feature = "ipc-health")]
use crate::{ResidentReadServerConfig, serve_read_until_stopped};
use pastral_agent_core::{CaptureConfig, CaptureCoordinator, CaptureOutcome, CaptureSequence};
use pastral_clipboard_win::{ClipboardListener, NotificationReceiveError};

const MAX_UNICODE_TEXT_BYTES: usize = 16 * 1024 * 1024;
const RETRY_DELAYS: [Duration; 4] = [
    Duration::ZERO,
    Duration::from_millis(5),
    Duration::from_millis(15),
    Duration::from_millis(35),
];

pub fn run_command<W: Write + Send>(
    command: AgentCommand,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    match command {
        AgentCommand::Run {
            data_root,
            max_events,
            max_connections,
        } => {
            let data_root = resolve_resident_data_root(data_root)?;
            #[cfg(feature = "ipc-health")]
            {
                run_resident(&data_root, max_events, max_connections, output)
            }
            #[cfg(not(feature = "ipc-health"))]
            {
                let _ = (max_events, max_connections, output);
                Err(AgentRuntimeError::ResidentIpc)
            }
        }
        AgentCommand::HealthCheck { data_root } => run_health_check(&data_root, output),
        AgentCommand::CaptureCurrent { data_root } => run_capture_current(&data_root, output),
        AgentCommand::Listen {
            data_root,
            max_events,
        } => run_listener(&data_root, max_events, output),
    }
}

pub fn resolve_resident_data_root(explicit: Option<PathBuf>) -> Result<PathBuf, AgentRuntimeError> {
    resolve_resident_data_root_from(explicit, env::var_os("LOCALAPPDATA"))
}

#[doc(hidden)]
pub fn resolve_resident_data_root_from(
    explicit: Option<PathBuf>,
    local_app_data: Option<OsString>,
) -> Result<PathBuf, AgentRuntimeError> {
    let root = match explicit {
        Some(path) => path,
        None => {
            let local_app_data = local_app_data.ok_or(AgentRuntimeError::InvalidDataRoot)?;
            if local_app_data.is_empty() {
                return Err(AgentRuntimeError::InvalidDataRoot);
            }
            PathBuf::from(local_app_data).join("Pastral")
        }
    };
    if !root.is_absolute() || root.as_os_str().is_empty() {
        return Err(AgentRuntimeError::InvalidDataRoot);
    }
    let native = root.as_os_str().to_string_lossy();
    if native.starts_with(r"\\") {
        return Err(AgentRuntimeError::InvalidDataRoot);
    }
    Ok(root)
}

fn run_health_check<W: Write>(data_root: &Path, output: &mut W) -> Result<(), AgentRuntimeError> {
    let snapshot = load_health_snapshot(data_root)?;

    write_line(output, &format!("data-root={}", data_root.display()))?;
    write_line(output, "agent-health=ok")?;
    write_line(
        output,
        if snapshot.privacy_policy_ok() {
            "privacy-policy=ok"
        } else {
            "privacy-policy=failed"
        },
    )?;
    write_line(
        output,
        &format!("storage-schema={}", snapshot.storage_schema_version()),
    )?;
    let integrity_marker = if snapshot.storage_integrity_ok() {
        "ok"
    } else {
        "failed"
    };
    write_line(output, &format!("sqlite-integrity={integrity_marker}"))?;
    write_line(output, &format!("fts-integrity={integrity_marker}"))?;
    write_line(output, &format!("metadata-integrity={integrity_marker}"))?;
    write_line(
        output,
        &format!("search-mapping-integrity={integrity_marker}"),
    )?;
    Ok(())
}

fn run_capture_current<W: Write>(
    data_root: &Path,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    let identity = AgentIdentity::load_or_create(data_root)?;
    let privacy_policy = PrivacyPolicyConfig::load_or_create(data_root)?;
    let mut coordinator = capture_coordinator(identity)?;
    let mut source = WindowsClipboardSource::new(privacy_policy.source_policy().clone());
    let mut sink = StorageCaptureSink::new(open_storage(data_root)?);
    let mut clock = SystemClock;
    let mut sleeper = ThreadSleeper;
    let sequence = CaptureSequence::new(NonZeroU32::MIN.get())
        .map_err(|_| AgentRuntimeError::CoordinatorConfiguration)?;
    let outcome =
        coordinator.handle_notification(sequence, &mut source, &mut sink, &mut clock, &mut sleeper);
    write_outcome(output, outcome)
}

#[cfg(feature = "ipc-health")]
fn run_resident<W: Write + Send>(
    data_root: &Path,
    max_events: Option<NonZeroUsize>,
    max_connections: Option<NonZeroUsize>,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    let _preflight = load_health_snapshot(data_root)?;
    let stop = Arc::new(AtomicBool::new(false));
    let ipc_config = ResidentReadServerConfig::new(
        data_root.to_path_buf(),
        Duration::from_millis(250),
        Duration::from_secs(2),
        max_connections,
    )
    .map_err(|_| AgentRuntimeError::ResidentIpc)?;

    let (capture_result, ipc_result, ipc_output) = thread::scope(|scope| {
        let ipc_stop = Arc::clone(&stop);
        let ipc_thread = scope.spawn(move || {
            let mut ipc_output = Vec::new();
            let result =
                serve_read_until_stopped(ipc_config, Arc::clone(&ipc_stop), &mut ipc_output);
            if result.is_err() {
                ipc_stop.store(true, Ordering::Release);
            }
            (result, ipc_output)
        });

        let capture_result =
            run_listener_until_stopped(data_root, max_events, Arc::clone(&stop), output);
        stop.store(true, Ordering::Release);
        let (ipc_result, ipc_output) = ipc_thread
            .join()
            .map_err(|_| AgentRuntimeError::ResidentIpc)?;
        Ok::<_, AgentRuntimeError>((capture_result, ipc_result, ipc_output))
    })?;

    output
        .write_all(&ipc_output)
        .map_err(|error| AgentRuntimeError::io("write resident IPC output", &error))?;
    capture_result?;
    ipc_result.map_err(|_| AgentRuntimeError::ResidentIpc)?;
    Ok(())
}

fn run_listener<W: Write>(
    data_root: &Path,
    max_events: Option<NonZeroUsize>,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    run_listener_until_stopped(
        data_root,
        max_events,
        Arc::new(AtomicBool::new(false)),
        output,
    )
}

fn run_listener_until_stopped<W: Write>(
    data_root: &Path,
    max_events: Option<NonZeroUsize>,
    stop: Arc<AtomicBool>,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    let identity = AgentIdentity::load_or_create(data_root)?;
    let privacy_policy = PrivacyPolicyConfig::load_or_create(data_root)?;
    let mut coordinator = capture_coordinator(identity)?;
    let mut source = WindowsClipboardSource::new(privacy_policy.source_policy().clone());
    let mut sink = StorageCaptureSink::new(open_storage(data_root)?);
    let mut clock = SystemClock;
    let mut sleeper = ThreadSleeper;
    let (listener, notifications) =
        ClipboardListener::start().map_err(|_| AgentRuntimeError::Clipboard("listener-start"))?;
    let mut terminal_outcomes = 0usize;

    while !stop.load(Ordering::Acquire) {
        match notifications.recv_timeout(Duration::from_secs(1)) {
            Ok(notification) => {
                let Some(raw_sequence) = notification.sequence().raw() else {
                    write_line(output, "capture-outcome=sequence-unavailable")?;
                    terminal_outcomes += 1;
                    if reached_limit(terminal_outcomes, max_events) {
                        stop.store(true, Ordering::Release);
                        break;
                    }
                    continue;
                };
                let sequence = CaptureSequence::new(raw_sequence)
                    .map_err(|_| AgentRuntimeError::CoordinatorConfiguration)?;
                let outcome = coordinator.handle_notification(
                    sequence,
                    &mut source,
                    &mut sink,
                    &mut clock,
                    &mut sleeper,
                );
                write_outcome(output, outcome)?;
                terminal_outcomes += 1;
                if reached_limit(terminal_outcomes, max_events) {
                    stop.store(true, Ordering::Release);
                    break;
                }
            }
            Err(NotificationReceiveError::Timeout | NotificationReceiveError::Empty) => {}
            Err(NotificationReceiveError::Disconnected) => {
                return Err(AgentRuntimeError::NotificationChannelClosed);
            }
        }
    }

    listener
        .stop()
        .map_err(|_| AgentRuntimeError::Clipboard("listener-stop"))
}

fn capture_coordinator(identity: AgentIdentity) -> Result<CaptureCoordinator, AgentRuntimeError> {
    let max_bytes = NonZeroUsize::new(MAX_UNICODE_TEXT_BYTES)
        .ok_or(AgentRuntimeError::CoordinatorConfiguration)?;
    let config = CaptureConfig::new(
        identity.profile_id(),
        identity.protection_domain(),
        max_bytes,
        RETRY_DELAYS.to_vec(),
    )
    .map_err(|_| AgentRuntimeError::CoordinatorConfiguration)?;
    CaptureCoordinator::new(config).map_err(|_| AgentRuntimeError::CoordinatorConfiguration)
}

fn reached_limit(count: usize, limit: Option<NonZeroUsize>) -> bool {
    limit.is_some_and(|value| count >= value.get())
}

fn write_outcome<W: Write>(
    output: &mut W,
    outcome: CaptureOutcome,
) -> Result<(), AgentRuntimeError> {
    match outcome {
        CaptureOutcome::Stored {
            clip_event_id,
            capture_order,
        } => write_line(
            output,
            &format!(
                "capture-outcome=stored event-id={clip_event_id} capture-order={}",
                capture_order.get()
            ),
        ),
        CaptureOutcome::DuplicateNotification => {
            write_line(output, "capture-outcome=duplicate-notification")
        }
        CaptureOutcome::NoSupportedRepresentation => {
            write_line(output, "capture-outcome=no-supported-representation")
        }
        CaptureOutcome::HardDenied => write_line(output, "capture-outcome=hard-denied"),
        CaptureOutcome::PolicyDenied => write_line(output, "capture-outcome=policy-denied"),
        CaptureOutcome::SensitiveSkipped => write_line(output, "capture-outcome=sensitive-skipped"),
        CaptureOutcome::RetryExhausted { attempts } => write_line(
            output,
            &format!("capture-outcome=retry-exhausted attempts={attempts}"),
        ),
        CaptureOutcome::InvalidCapture => write_line(output, "capture-outcome=invalid-capture"),
        CaptureOutcome::PlatformFailure => write_line(output, "capture-outcome=platform-failure"),
        CaptureOutcome::StorageFailed => write_line(output, "capture-outcome=storage-failed"),
    }
}

fn write_line<W: Write>(output: &mut W, value: &str) -> Result<(), AgentRuntimeError> {
    writeln!(output, "{value}")
        .map_err(|error| AgentRuntimeError::io("write diagnostic output", &error))
}
