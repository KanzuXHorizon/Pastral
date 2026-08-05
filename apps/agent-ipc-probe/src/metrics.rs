use crate::AdmissionError;

const MIB: u64 = 1024 * 1024;
const MAX_SERVER_PRIVATE_BYTES: u64 = 25 * MIB;
const MAX_PRIVATE_DELTA_BYTES: i64 = 8 * MIB as i64;
const MAX_WORKING_SET_DELTA_BYTES: i64 = 12 * MIB as i64;
const MAX_BINARY_DELTA_BYTES: u64 = 6 * MIB;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FootprintMetrics {
    binary_delta_bytes: u64,
    working_set_delta_bytes: i64,
    private_delta_bytes: i64,
}

impl FootprintMetrics {
    #[must_use]
    pub const fn binary_delta_bytes(self) -> u64 {
        self.binary_delta_bytes
    }

    #[must_use]
    pub const fn working_set_delta_bytes(self) -> i64 {
        self.working_set_delta_bytes
    }

    #[must_use]
    pub const fn private_delta_bytes(self) -> i64 {
        self.private_delta_bytes
    }
}

pub fn calculate_footprint(
    default_agent_binary_bytes: u64,
    admission_binary_bytes: u64,
    baseline_working_set_bytes: u64,
    baseline_private_bytes: u64,
    server_working_set_bytes: u64,
    server_private_bytes: u64,
) -> Result<FootprintMetrics, AdmissionError> {
    for value in [
        default_agent_binary_bytes,
        admission_binary_bytes,
        baseline_working_set_bytes,
        baseline_private_bytes,
        server_working_set_bytes,
        server_private_bytes,
    ] {
        if value == 0 {
            return Err(AdmissionError::InvalidMetric);
        }
    }
    let binary_delta_bytes = admission_binary_bytes.abs_diff(default_agent_binary_bytes);
    let working_set_delta_bytes =
        signed_delta(server_working_set_bytes, baseline_working_set_bytes)?;
    let private_delta_bytes = signed_delta(server_private_bytes, baseline_private_bytes)?;
    Ok(FootprintMetrics {
        binary_delta_bytes,
        working_set_delta_bytes,
        private_delta_bytes,
    })
}

pub fn enforce_footprint(
    metrics: FootprintMetrics,
    server_private_bytes: u64,
) -> Result<(), AdmissionError> {
    if metrics.binary_delta_bytes > MAX_BINARY_DELTA_BYTES
        || metrics.working_set_delta_bytes > MAX_WORKING_SET_DELTA_BYTES
        || metrics.private_delta_bytes > MAX_PRIVATE_DELTA_BYTES
        || server_private_bytes > MAX_SERVER_PRIVATE_BYTES
    {
        return Err(AdmissionError::FootprintCeiling);
    }
    Ok(())
}

pub fn evaluate_footprint(
    default_agent_binary_bytes: u64,
    admission_binary_bytes: u64,
    baseline_working_set_bytes: u64,
    baseline_private_bytes: u64,
    server_working_set_bytes: u64,
    server_private_bytes: u64,
) -> Result<FootprintMetrics, AdmissionError> {
    let metrics = calculate_footprint(
        default_agent_binary_bytes,
        admission_binary_bytes,
        baseline_working_set_bytes,
        baseline_private_bytes,
        server_working_set_bytes,
        server_private_bytes,
    )?;
    enforce_footprint(metrics, server_private_bytes)?;
    Ok(metrics)
}

fn signed_delta(new_value: u64, baseline: u64) -> Result<i64, AdmissionError> {
    if new_value >= baseline {
        i64::try_from(new_value - baseline).map_err(|_| AdmissionError::InvalidMetric)
    } else {
        i64::try_from(baseline - new_value)
            .map(|value| -value)
            .map_err(|_| AdmissionError::InvalidMetric)
    }
}
