use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidSequence,
    InvalidCapturedText(&'static str),
    InvalidRetrySchedule(&'static str),
    NonOrdinaryProtectionDomain,
    ClockFailure,
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSequence => write!(f, "capture sequence must be nonzero"),
            Self::InvalidCapturedText(reason) => {
                write!(f, "captured Unicode text is invalid: {reason}")
            }
            Self::InvalidRetrySchedule(reason) => {
                write!(f, "capture retry schedule is invalid: {reason}")
            }
            Self::NonOrdinaryProtectionDomain => {
                write!(
                    f,
                    "capture foundation accepts only ordinary protection domains"
                )
            }
            Self::ClockFailure => write!(f, "UTC capture clock is unavailable"),
        }
    }
}

impl std::error::Error for AgentError {}
