#![deny(unsafe_code)]

mod convert;

use sha2::{Digest, Sha256};

pub use convert::{
    decode_bulk_end, decode_client_hello, decode_protocol_error, decode_request, decode_response,
    decode_server_accepted, decode_server_hello, encode_bulk_end, encode_client_hello,
    encode_protocol_error, encode_request, encode_response, encode_server_accepted,
    encode_server_hello,
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
    canonical_schema_sha256(include_bytes!(
        "../../../protocols/ipc-schema/pastral_ipc_v1.proto"
    ))
}

fn canonical_schema_sha256(bytes: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    let mut segment_start = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            digest.update(&bytes[segment_start..index]);
            digest.update(b"\n");
            index += 2;
            segment_start = index;
        } else {
            index += 1;
        }
    }
    digest.update(&bytes[segment_start..]);
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::canonical_schema_sha256;

    #[test]
    fn schema_digest_is_independent_of_git_line_endings() {
        let lf = b"edition = \"2024\";\nmessage Example {}\n";
        let crlf = b"edition = \"2024\";\r\nmessage Example {}\r\n";
        assert_eq!(canonical_schema_sha256(lf), canonical_schema_sha256(crlf));
    }
}
