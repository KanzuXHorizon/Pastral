use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidSecret,
    InvalidNonce,
    InvalidPeerIdentity(&'static str),
    InvalidTranscript(&'static str),
    ProofMismatch,
    InvalidReplayCapacity,
    ReplayDetected,
    IntegerOverflow,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSecret => write!(f, "IPC installation secret is invalid"),
            Self::InvalidNonce => write!(f, "IPC authentication nonce is invalid"),
            Self::InvalidPeerIdentity(reason) => {
                write!(f, "IPC transcript peer identity is invalid: {reason}")
            }
            Self::InvalidTranscript(reason) => {
                write!(f, "IPC authentication transcript is invalid: {reason}")
            }
            Self::ProofMismatch => write!(f, "IPC authentication proof does not match"),
            Self::InvalidReplayCapacity => write!(f, "IPC replay cache capacity is invalid"),
            Self::ReplayDetected => write!(f, "IPC handshake replay was detected"),
            Self::IntegerOverflow => write!(f, "IPC authentication arithmetic overflowed"),
        }
    }
}

impl std::error::Error for AuthError {}
