#![deny(unsafe_code)]

mod config;
mod dpapi;
mod error;
mod random;
#[allow(unsafe_code)]
mod sys;

pub use config::{
    IDENTITY_FILE_NAME, PipeName, SECRET_FILE_NAME, TransportIdentity, TransportMaterial,
    derive_pipe_name, load_or_create_transport_material,
};
pub use dpapi::{
    MAX_SECRET_ENVELOPE_BYTES, protect_installation_secret, unprotect_installation_secret,
};
pub use error::TransportError;
pub use random::random_bytes;
