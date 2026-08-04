#![deny(unsafe_code)]

mod config;
mod dpapi;
mod error;
mod pipe;
mod random;
mod security;
#[allow(unsafe_code)]
mod sys;
mod token;

pub use config::{
    IDENTITY_FILE_NAME, PipeName, SECRET_FILE_NAME, TransportIdentity, TransportMaterial,
    derive_pipe_name, load_or_create_transport_material,
};
pub use dpapi::{
    MAX_SECRET_ENVELOPE_BYTES, protect_installation_secret, unprotect_installation_secret,
};
pub use error::TransportError;
pub use pipe::{PipeServer, create_first_pipe_server};
pub use random::random_bytes;
pub use security::{
    PipeSecurity, SecurityInspection, build_logon_sid_pipe_security, inspect_pipe_security,
};
pub use token::{
    PeerMismatch, SidBytes, TokenIdentity, ValidatedPeer, current_token_identity,
    process_token_identity, validate_peer,
};
