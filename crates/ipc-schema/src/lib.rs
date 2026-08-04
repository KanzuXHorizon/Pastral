#![deny(unsafe_code)]

mod convert;

use sha2::{Digest, Sha256};

pub use convert::{
    decode_bulk_end, decode_client_hello, decode_protocol_error, decode_request, decode_response,
    decode_server_hello, encode_bulk_end, encode_client_hello, encode_protocol_error,
    encode_request, encode_response, encode_server_hello,
};

pub const PROTOBUF_RELEASE: &str = "4.35.0-release";

#[allow(
    unsafe_code,
    clippy::derivable_impls,
    clippy::manual_range_patterns,
    clippy::needless_borrow,
    clippy::unnecessary_fallible_conversions,
    clippy::useless_conversion
)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/protobuf_generated/generated.rs"));
}

#[must_use]
pub fn schema_sha256() -> [u8; 32] {
    let bytes = include_bytes!("../../../protocols/ipc-schema/pastral_ipc_v1.proto");
    Sha256::digest(bytes).into()
}
