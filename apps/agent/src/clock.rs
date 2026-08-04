use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pastral_agent_core::{AgentError, Clock, Sleeper};
use pastral_domain::UtcUnixMicros;

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc_micros(&mut self) -> Result<UtcUnixMicros, AgentError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AgentError::ClockFailure)?;
        let micros = i64::try_from(duration.as_micros()).map_err(|_| AgentError::ClockFailure)?;
        UtcUnixMicros::new(micros).map_err(|_| AgentError::ClockFailure)
    }
}

#[derive(Debug, Default)]
pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
