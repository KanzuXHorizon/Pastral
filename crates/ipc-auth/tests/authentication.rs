use pastral_ipc_auth::{
    AuthError, HandshakeTranscript, InstallationSecret, Nonce, PeerTranscriptIdentity, ProofRole,
    compute_proof, verify_proof,
};
use pastral_ipc_core::{Capability, CorrelationId};

fn secret(seed: u8) -> InstallationSecret {
    InstallationSecret::from_bytes([seed; 32])
}

fn transcript() -> HandshakeTranscript {
    HandshakeTranscript::new(
        [0xA5; 32],
        1,
        0,
        2,
        1,
        2,
        1,
        Nonce::from_bytes([0x11; 32]).unwrap(),
        Nonce::from_bytes([0x22; 32]).unwrap(),
        CorrelationId::from_bytes([
            0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44,
            0x00, 0x00,
        ])
        .unwrap(),
        PeerTranscriptIdentity::new(4100, 3, 0x2000).unwrap(),
        PeerTranscriptIdentity::new(4200, 3, 0x2000).unwrap(),
        [Capability::Search, Capability::Health],
        [Capability::Health],
    )
    .unwrap()
}

#[test]
fn deterministic_client_and_server_proofs_are_role_separated() {
    let transcript = transcript();
    let installation_secret = secret(0x42);
    let client = compute_proof(&installation_secret, &transcript, ProofRole::Client);
    let server = compute_proof(&installation_secret, &transcript, ProofRole::Server);

    assert_ne!(client.as_bytes(), server.as_bytes());
    assert_eq!(
        client.as_bytes(),
        &[
            0xa1, 0x63, 0x97, 0x55, 0x9c, 0x0e, 0x48, 0x65, 0xa9, 0x98, 0xfb, 0xcd, 0xcb, 0xdb,
            0x00, 0xab, 0x4a, 0xba, 0x9f, 0x35, 0x6c, 0xd4, 0xf2, 0xa5, 0xdd, 0xb2, 0x61, 0xae,
            0xa6, 0x2d, 0xb6, 0x1e,
        ]
    );
}

#[test]
fn valid_proofs_verify_and_wrong_secret_role_or_bit_flip_fail() {
    let transcript = transcript();
    let installation_secret = secret(0x42);
    let proof = compute_proof(&installation_secret, &transcript, ProofRole::Client);
    verify_proof(&installation_secret, &transcript, ProofRole::Client, &proof).unwrap();

    assert_eq!(
        verify_proof(&secret(0x43), &transcript, ProofRole::Client, &proof),
        Err(AuthError::ProofMismatch)
    );
    assert_eq!(
        verify_proof(&installation_secret, &transcript, ProofRole::Server, &proof),
        Err(AuthError::ProofMismatch)
    );

    let mut tampered = *proof.as_bytes();
    tampered[31] ^= 1;
    assert_eq!(
        verify_proof(
            &installation_secret,
            &transcript,
            ProofRole::Client,
            &pastral_ipc_auth::AuthenticationProof::from_bytes(tampered),
        ),
        Err(AuthError::ProofMismatch)
    );
}

#[test]
fn every_transcript_field_is_bound_to_the_proof() {
    let base = transcript();
    let installation_secret = secret(0x42);
    let proof = compute_proof(&installation_secret, &base, ProofRole::Client);

    let variants = [
        base.with_schema_digest([0xA4; 32]).unwrap(),
        base.with_protocol_major(2).unwrap(),
        base.with_server_minor_range(1, 2).unwrap(),
        base.with_client_minor_range(0, 1).unwrap(),
        base.with_selected_minor(2).unwrap(),
        base.with_server_nonce(Nonce::from_bytes([0x12; 32]).unwrap()),
        base.with_client_nonce(Nonce::from_bytes([0x23; 32]).unwrap()),
        base.with_instance_id(CorrelationId::new_v4()).unwrap(),
        base.with_server_identity(PeerTranscriptIdentity::new(4101, 3, 0x2000).unwrap()),
        base.with_server_identity(PeerTranscriptIdentity::new(4100, 4, 0x2000).unwrap()),
        base.with_server_identity(PeerTranscriptIdentity::new(4100, 3, 0x3000).unwrap()),
        base.with_client_identity(PeerTranscriptIdentity::new(4201, 3, 0x2000).unwrap()),
        base.with_client_identity(PeerTranscriptIdentity::new(4200, 4, 0x2000).unwrap()),
        base.with_client_identity(PeerTranscriptIdentity::new(4200, 3, 0x3000).unwrap()),
        base.with_requested_capabilities([Capability::Health])
            .unwrap(),
        base.with_accepted_capabilities([Capability::Search])
            .unwrap(),
    ];

    for variant in variants {
        assert_eq!(
            verify_proof(&installation_secret, &variant, ProofRole::Client, &proof),
            Err(AuthError::ProofMismatch)
        );
    }
}

#[test]
fn transcript_rejects_invalid_versions_nonces_peers_and_capabilities() {
    let valid = transcript();
    assert!(matches!(
        valid.with_protocol_major(0),
        Err(AuthError::InvalidTranscript(
            "protocol major must be nonzero"
        ))
    ));
    assert!(matches!(
        valid.with_server_minor_range(3, 2),
        Err(AuthError::InvalidTranscript(
            "server minor range is invalid"
        ))
    ));
    assert!(matches!(
        valid.with_selected_minor(3),
        Err(AuthError::InvalidTranscript(
            "selected minor is outside negotiated ranges"
        ))
    ));
    assert!(matches!(
        Nonce::from_bytes([0; 32]),
        Err(AuthError::InvalidNonce)
    ));
    assert!(PeerTranscriptIdentity::new(0, 1, 0x2000).is_err());
    assert!(PeerTranscriptIdentity::new(1, 1, 0).is_err());
    assert!(matches!(
        valid.with_requested_capabilities([Capability::Health, Capability::Health]),
        Err(AuthError::InvalidTranscript(
            "requested capability is duplicated"
        ))
    ));
    assert!(matches!(
        valid.with_accepted_capabilities([Capability::HistoryPage]),
        Err(AuthError::InvalidTranscript(
            "accepted capability was not requested"
        ))
    ));
}
