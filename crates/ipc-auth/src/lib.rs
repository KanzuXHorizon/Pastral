#![forbid(unsafe_code)]

mod error;
mod material;
mod replay;
mod transcript;

pub use error::AuthError;
pub use material::{AUTH_MATERIAL_BYTES, AuthenticationProof, InstallationSecret, Nonce};
pub use replay::{MAX_REPLAY_CACHE_ENTRIES, NonceReplayCache};
pub use transcript::{
    HandshakeTranscript, PeerTranscriptIdentity, ProofRole, compute_proof, verify_proof,
};
