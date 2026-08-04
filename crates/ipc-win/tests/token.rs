use pastral_ipc_win::{
    PeerMismatch, TokenIdentity, current_token_identity, process_token_identity, validate_peer,
};

#[test]
fn current_and_current_process_token_identity_match() {
    let current = current_token_identity().unwrap();
    let process = process_token_identity(std::process::id()).unwrap();

    assert_eq!(current, process);
    assert_eq!(current.process_id(), std::process::id());
    assert!(!current.user_sid().as_bytes().is_empty());
    assert!(!current.logon_sid().as_bytes().is_empty());
    assert_ne!(current.integrity_rid(), 0);
}

#[test]
fn invalid_process_id_fails_closed() {
    assert!(process_token_identity(u32::MAX).is_err());
}

#[test]
fn exact_kernel_and_token_evidence_validates() {
    let current = current_token_identity().unwrap();
    let validated = validate_peer(
        &current,
        current.process_id(),
        current.session_id(),
        &current,
    )
    .unwrap();
    assert_eq!(validated.process_id(), current.process_id());
    assert_eq!(validated.session_id(), current.session_id());
    assert_eq!(validated.integrity_rid(), current.integrity_rid());
}

#[test]
fn every_peer_evidence_mismatch_is_distinct() {
    let baseline = TokenIdentity::for_test(vec![1, 2, 3], vec![4, 5, 6], 7, 0x2000, 100).unwrap();

    let cases = [
        (
            101,
            7,
            TokenIdentity::for_test(vec![1, 2, 3], vec![4, 5, 6], 7, 0x2000, 100).unwrap(),
            PeerMismatch::ProcessId,
        ),
        (
            100,
            8,
            TokenIdentity::for_test(vec![1, 2, 3], vec![4, 5, 6], 7, 0x2000, 100).unwrap(),
            PeerMismatch::Session,
        ),
        (
            100,
            7,
            TokenIdentity::for_test(vec![9], vec![4, 5, 6], 7, 0x2000, 100).unwrap(),
            PeerMismatch::UserSid,
        ),
        (
            100,
            7,
            TokenIdentity::for_test(vec![1, 2, 3], vec![9], 7, 0x2000, 100).unwrap(),
            PeerMismatch::LogonSid,
        ),
        (
            100,
            7,
            TokenIdentity::for_test(vec![1, 2, 3], vec![4, 5, 6], 8, 0x2000, 100).unwrap(),
            PeerMismatch::Session,
        ),
        (
            100,
            7,
            TokenIdentity::for_test(vec![1, 2, 3], vec![4, 5, 6], 7, 0x3000, 100).unwrap(),
            PeerMismatch::Integrity,
        ),
    ];

    for (kernel_pid, kernel_session, observed, expected) in cases {
        assert_eq!(
            validate_peer(&baseline, kernel_pid, kernel_session, &observed),
            Err(expected)
        );
    }
}

#[test]
fn synthetic_token_identity_rejects_empty_sid_zero_pid_and_zero_integrity() {
    assert!(TokenIdentity::for_test(vec![], vec![1], 0, 0x2000, 1).is_err());
    assert!(TokenIdentity::for_test(vec![1], vec![], 0, 0x2000, 1).is_err());
    assert!(TokenIdentity::for_test(vec![1], vec![2], 0, 0, 1).is_err());
    assert!(TokenIdentity::for_test(vec![1], vec![2], 0, 0x2000, 0).is_err());
}
