use pastral_ipc_auth::{AuthError, Nonce, NonceReplayCache};

fn nonce(seed: u8) -> Nonce {
    Nonce::from_bytes([seed; 32]).unwrap()
}

#[test]
fn replay_cache_rejects_duplicate_active_handshake() {
    let mut cache = NonceReplayCache::new(4).unwrap();
    cache.record(&nonce(1), &nonce(2), 100, 3).unwrap();
    assert_eq!(
        cache.record(&nonce(1), &nonce(2), 100, 3),
        Err(AuthError::ReplayDetected)
    );
    assert_eq!(cache.len(), 1);
}

#[test]
fn replay_key_binds_both_nonces_pid_and_session() {
    let mut cache = NonceReplayCache::new(8).unwrap();
    cache.record(&nonce(1), &nonce(2), 100, 3).unwrap();
    cache.record(&nonce(3), &nonce(2), 100, 3).unwrap();
    cache.record(&nonce(1), &nonce(4), 100, 3).unwrap();
    cache.record(&nonce(1), &nonce(2), 101, 3).unwrap();
    cache.record(&nonce(1), &nonce(2), 100, 4).unwrap();
    assert_eq!(cache.len(), 5);
}

#[test]
fn replay_cache_is_fifo_bounded() {
    let mut cache = NonceReplayCache::new(2).unwrap();
    cache.record(&nonce(1), &nonce(2), 100, 3).unwrap();
    cache.record(&nonce(3), &nonce(4), 100, 3).unwrap();
    cache.record(&nonce(5), &nonce(6), 100, 3).unwrap();
    assert_eq!(cache.len(), 2);
    cache.record(&nonce(1), &nonce(2), 100, 3).unwrap();
    assert_eq!(cache.len(), 2);
    assert_eq!(
        cache.record(&nonce(5), &nonce(6), 100, 3),
        Err(AuthError::ReplayDetected)
    );
}

#[test]
fn replay_cache_capacity_is_bounded() {
    assert!(matches!(
        NonceReplayCache::new(0),
        Err(AuthError::InvalidReplayCapacity)
    ));
    assert!(matches!(
        NonceReplayCache::new(1025),
        Err(AuthError::InvalidReplayCapacity)
    ));
    assert!(NonceReplayCache::new(1024).is_ok());
}
