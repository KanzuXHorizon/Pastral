use std::time::Instant;

use crate::TransportError;

pub(crate) fn remaining_millis(
    deadline: Instant,
    operation: &'static str,
) -> Result<u32, TransportError> {
    let now = Instant::now();
    if now >= deadline {
        return Err(TransportError::Timeout(operation));
    }
    let remaining = deadline.duration_since(now);
    let millis = remaining.as_millis();
    let rounded = if remaining.subsec_nanos().is_multiple_of(1_000_000) {
        millis
    } else {
        millis.saturating_add(1)
    };
    Ok(u32::try_from(rounded.min(u128::from(u32::MAX - 1)))
        .expect("deadline milliseconds are clamped to u32"))
}
