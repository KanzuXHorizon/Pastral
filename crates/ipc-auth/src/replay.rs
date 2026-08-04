use std::collections::{BTreeSet, VecDeque};

use sha2::{Digest, Sha256};

use crate::{AuthError, Nonce};

pub const MAX_REPLAY_CACHE_ENTRIES: usize = 1024;

pub struct NonceReplayCache {
    capacity: usize,
    entries: BTreeSet<[u8; 32]>,
    order: VecDeque<[u8; 32]>,
}

impl NonceReplayCache {
    pub fn new(capacity: usize) -> Result<Self, AuthError> {
        if capacity == 0 || capacity > MAX_REPLAY_CACHE_ENTRIES {
            return Err(AuthError::InvalidReplayCapacity);
        }
        Ok(Self {
            capacity,
            entries: BTreeSet::new(),
            order: VecDeque::new(),
        })
    }

    pub fn record(
        &mut self,
        server_nonce: &Nonce,
        client_nonce: &Nonce,
        client_process_id: u32,
        session_id: u32,
    ) -> Result<(), AuthError> {
        if client_process_id == 0 {
            return Err(AuthError::InvalidPeerIdentity(
                "replay client process ID must be nonzero",
            ));
        }
        let key = replay_key(server_nonce, client_nonce, client_process_id, session_id);
        if self.entries.contains(&key) {
            return Err(AuthError::ReplayDetected);
        }
        if self.entries.len() == self.capacity {
            let oldest = self.order.pop_front().ok_or(AuthError::IntegerOverflow)?;
            self.entries.remove(&oldest);
        }
        self.entries.insert(key);
        self.order.push_back(key);
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn replay_key(
    server_nonce: &Nonce,
    client_nonce: &Nonce,
    client_process_id: u32,
    session_id: u32,
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"Pastral IPC replay cache v1");
    hash.update(server_nonce.as_bytes());
    hash.update(client_nonce.as_bytes());
    hash.update(client_process_id.to_le_bytes());
    hash.update(session_id.to_le_bytes());
    hash.finalize().into()
}
