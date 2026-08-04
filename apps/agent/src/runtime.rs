use core::num::{NonZeroU32, NonZeroUsize};
use std::{io::Write, path::Path, time::Duration};

use crate::health::open_storage;
use crate::{
    AgentCommand, AgentIdentity, AgentRuntimeError, PrivacyPolicyConfig, StorageCaptureSink,
    SystemClock, ThreadSleeper, WindowsClipboardSource, load_health_snapshot,
};
use pastral_agent_core::{CaptureConfig, CaptureCoordinator, CaptureOutcome, CaptureSequence};
use pastral_clipboard_win::{ClipboardListener, NotificationReceiveError};

const MAX_UNICODE_TEXT_BYTES: usize = 16 * 1024 * 1024;
const RETRY_DELAYS: [Duration; 4] = [
    Duration::ZERO,
    Duration::from_millis(5),
    Duration::from_millis(15),
    Duration::from_millis(35),
];

pub fn run_command<W: Write>(
    command: AgentCommand,
    output: &mut W,
) -> Result<(), AgentRuntimeError> {
    match command {
        AgentCommand::HealthCheck { data_root } => run_health_check(&data_root, output),
        AgentCommand::CaptureCurrent { data_root } => run_capture_current(&data_root, output),
        AgentCommand::Listen {
            data_root,
            max_events,
        } => run_listener(&data_root, max_events, output),
    }
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

fn run_listener<W: Write>(
    data_root: &Path,
    max_events: Option<NonZeroUsize>,
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

    loop {
        match notifications.recv_timeout(Duration::from_secs(1)) {
            Ok(notification) => {
                let Some(raw_sequence) = notification.sequence().raw() else {
                    write_line(output, "capture-outcome=sequence-unavailable")?;
                    terminal_outcomes += 1;
                    if reached_limit(terminal_outcomes, max_events) {
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
