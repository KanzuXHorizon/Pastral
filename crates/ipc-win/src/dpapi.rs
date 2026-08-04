use pastral_ipc_auth::{AUTH_MATERIAL_BYTES, InstallationSecret};

use crate::{TransportError, sys};

const ENVELOPE_MAGIC: [u8; 4] = *b"PSE1";
const ENVELOPE_VERSION: u16 = 1;
const HEADER_BYTES: usize = 12;
const PURPOSE_ENTROPY: &[u8] = b"Pastral IPC installation secret v1";

pub const MAX_SECRET_ENVELOPE_BYTES: usize = 16 * 1024;

pub fn protect_installation_secret(secret: &InstallationSecret) -> Result<Vec<u8>, TransportError> {
    protect_with_entropy(secret, PURPOSE_ENTROPY)
}

pub fn unprotect_installation_secret(
    envelope: &[u8],
) -> Result<InstallationSecret, TransportError> {
    unprotect_with_entropy(envelope, PURPOSE_ENTROPY)
}

fn protect_with_entropy(
    secret: &InstallationSecret,
    entropy: &[u8],
) -> Result<Vec<u8>, TransportError> {
    let ciphertext = secret.expose(|bytes| {
        sys::protect_user_data(bytes, entropy, MAX_SECRET_ENVELOPE_BYTES - HEADER_BYTES)
    })?;
    let ciphertext_length = u32::try_from(ciphertext.len())
        .map_err(|_| TransportError::SizeLimit("DPAPI ciphertext exceeds u32"))?;
    let total = HEADER_BYTES
        .checked_add(ciphertext.len())
        .ok_or(TransportError::SizeLimit("secret envelope length overflow"))?;
    if total > MAX_SECRET_ENVELOPE_BYTES {
        return Err(TransportError::SizeLimit("secret envelope exceeds bound"));
    }

    let mut envelope = Vec::with_capacity(total);
    envelope.extend_from_slice(&ENVELOPE_MAGIC);
    envelope.extend_from_slice(&ENVELOPE_VERSION.to_le_bytes());
    envelope.extend_from_slice(&0u16.to_le_bytes());
    envelope.extend_from_slice(&ciphertext_length.to_le_bytes());
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

fn unprotect_with_entropy(
    envelope: &[u8],
    entropy: &[u8],
) -> Result<InstallationSecret, TransportError> {
    if envelope.len() < HEADER_BYTES {
        return Err(TransportError::InvalidSecretEnvelope(
            "envelope is truncated",
        ));
    }
    if envelope.len() > MAX_SECRET_ENVELOPE_BYTES {
        return Err(TransportError::InvalidSecretEnvelope(
            "envelope exceeds maximum length",
        ));
    }
    if envelope[..4] != ENVELOPE_MAGIC {
        return Err(TransportError::InvalidSecretEnvelope("magic is invalid"));
    }
    let version = u16::from_le_bytes([envelope[4], envelope[5]]);
    if version != ENVELOPE_VERSION {
        return Err(TransportError::InvalidSecretEnvelope(
            "version is unsupported",
        ));
    }
    let reserved = u16::from_le_bytes([envelope[6], envelope[7]]);
    if reserved != 0 {
        return Err(TransportError::InvalidSecretEnvelope(
            "reserved field is nonzero",
        ));
    }
    let ciphertext_length = usize::try_from(u32::from_le_bytes([
        envelope[8],
        envelope[9],
        envelope[10],
        envelope[11],
    ]))
    .map_err(|_| TransportError::InvalidSecretEnvelope("ciphertext length is invalid"))?;
    if ciphertext_length == 0 || HEADER_BYTES + ciphertext_length != envelope.len() {
        return Err(TransportError::InvalidSecretEnvelope(
            "ciphertext length does not match envelope",
        ));
    }

    let plaintext =
        sys::unprotect_user_data(&envelope[HEADER_BYTES..], entropy, AUTH_MATERIAL_BYTES)?;
    if plaintext.len() != AUTH_MATERIAL_BYTES {
        return Err(TransportError::InvalidSecretEnvelope(
            "plaintext secret length is invalid",
        ));
    }
    let mut secret = [0u8; AUTH_MATERIAL_BYTES];
    secret.copy_from_slice(&plaintext);
    Ok(InstallationSecret::from_bytes(secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_optional_entropy_fails_closed() {
        let secret = InstallationSecret::from_bytes([0x42; AUTH_MATERIAL_BYTES]);
        let envelope = protect_with_entropy(&secret, b"purpose-a").unwrap();
        assert!(matches!(
            unprotect_with_entropy(&envelope, b"purpose-b"),
            Err(TransportError::Windows { .. })
        ));
    }
}
