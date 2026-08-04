use pastral_ipc_auth::InstallationSecret;
use pastral_ipc_win::{
    MAX_SECRET_ENVELOPE_BYTES, TransportError, protect_installation_secret, random_bytes,
    unprotect_installation_secret,
};

fn exposed(secret: &InstallationSecret) -> [u8; 32] {
    secret.expose(|bytes| *bytes)
}

#[test]
fn system_rng_is_nonzero_and_nonrepeating_for_32_byte_samples() {
    let first = random_bytes::<32>().unwrap();
    let second = random_bytes::<32>().unwrap();
    assert_ne!(first, [0; 32]);
    assert_ne!(second, [0; 32]);
    assert_ne!(first, second);
}

#[test]
fn installation_secret_round_trips_exactly_through_user_scope_dpapi() {
    let original = InstallationSecret::from_bytes([0x5a; 32]);
    let envelope = protect_installation_secret(&original).unwrap();
    assert!(envelope.len() <= MAX_SECRET_ENVELOPE_BYTES);
    assert_eq!(&envelope[..4], b"PSE1");

    let recovered = unprotect_installation_secret(&envelope).unwrap();
    assert_eq!(exposed(&recovered), [0x5a; 32]);
}

#[test]
fn truncated_length_mismatch_reserved_and_oversized_envelopes_fail_closed() {
    for length in 0..12 {
        assert!(matches!(
            unprotect_installation_secret(&vec![0; length]),
            Err(TransportError::InvalidSecretEnvelope(_))
        ));
    }

    let original = InstallationSecret::from_bytes([0x33; 32]);
    let envelope = protect_installation_secret(&original).unwrap();

    let mut wrong_magic = envelope.clone();
    wrong_magic[0] ^= 1;
    assert!(matches!(
        unprotect_installation_secret(&wrong_magic),
        Err(TransportError::InvalidSecretEnvelope(_))
    ));

    let mut reserved = envelope.clone();
    reserved[6] = 1;
    assert!(matches!(
        unprotect_installation_secret(&reserved),
        Err(TransportError::InvalidSecretEnvelope(_))
    ));

    let mut length_mismatch = envelope.clone();
    length_mismatch[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(matches!(
        unprotect_installation_secret(&length_mismatch),
        Err(TransportError::InvalidSecretEnvelope(_))
    ));

    assert!(matches!(
        unprotect_installation_secret(&vec![0; MAX_SECRET_ENVELOPE_BYTES + 1]),
        Err(TransportError::InvalidSecretEnvelope(_))
    ));
}
