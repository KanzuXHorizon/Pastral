use std::{
    io::{BufRead, Read, Write},
    path::Path,
};

use pastral_agent::load_health_snapshot;

use crate::AdmissionError;

const MAX_CHILD_INPUT_BYTES: usize = 64;

pub fn run_baseline_child<R: BufRead, W: Write>(
    data_root: &Path,
    mut input: R,
    mut output: W,
) -> Result<(), AdmissionError> {
    let snapshot = load_health_snapshot(data_root).map_err(|_| AdmissionError::AgentHealth)?;
    if snapshot.storage_schema_version() == 0
        || snapshot.capture_enabled()
        || !snapshot.privacy_policy_ok()
        || !snapshot.storage_integrity_ok()
    {
        return Err(AdmissionError::AgentHealth);
    }

    output
        .write_all(b"agent-baseline-ready=ok\n")
        .map_err(|error| AdmissionError::io("write baseline readiness", &error))?;
    output
        .flush()
        .map_err(|error| AdmissionError::io("flush baseline readiness", &error))?;

    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(u64::try_from(MAX_CHILD_INPUT_BYTES + 1).expect("small bound fits u64"))
        .read_to_end(&mut bytes)
        .map_err(|error| AdmissionError::io("read baseline child stdin", &error))?;
    if bytes.len() > MAX_CHILD_INPUT_BYTES || bytes.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err(AdmissionError::InvalidChildInput);
    }
    Ok(())
}
