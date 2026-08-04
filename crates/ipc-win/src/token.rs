use crate::{TransportError, sys};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SidBytes(Vec<u8>);

impl SidBytes {
    pub(crate) fn new(bytes: Vec<u8>) -> Result<Self, TransportError> {
        if bytes.is_empty() {
            return Err(TransportError::InvalidTokenIdentity(
                "SID must not be empty",
            ));
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl core::fmt::Debug for SidBytes {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "SidBytes(len={})", self.0.len())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TokenIdentity {
    user_sid: SidBytes,
    logon_sid: SidBytes,
    session_id: u32,
    integrity_rid: u32,
    process_id: u32,
}

impl TokenIdentity {
    pub(crate) fn new(
        user_sid: Vec<u8>,
        logon_sid: Vec<u8>,
        session_id: u32,
        integrity_rid: u32,
        process_id: u32,
    ) -> Result<Self, TransportError> {
        if integrity_rid == 0 {
            return Err(TransportError::InvalidTokenIdentity(
                "integrity RID must be nonzero",
            ));
        }
        if process_id == 0 {
            return Err(TransportError::InvalidTokenIdentity(
                "process ID must be nonzero",
            ));
        }
        Ok(Self {
            user_sid: SidBytes::new(user_sid)?,
            logon_sid: SidBytes::new(logon_sid)?,
            session_id,
            integrity_rid,
            process_id,
        })
    }

    #[doc(hidden)]
    pub fn for_test(
        user_sid: Vec<u8>,
        logon_sid: Vec<u8>,
        session_id: u32,
        integrity_rid: u32,
        process_id: u32,
    ) -> Result<Self, TransportError> {
        Self::new(user_sid, logon_sid, session_id, integrity_rid, process_id)
    }

    #[must_use]
    pub const fn user_sid(&self) -> &SidBytes {
        &self.user_sid
    }
    #[must_use]
    pub const fn logon_sid(&self) -> &SidBytes {
        &self.logon_sid
    }
    #[must_use]
    pub const fn session_id(&self) -> u32 {
        self.session_id
    }
    #[must_use]
    pub const fn integrity_rid(&self) -> u32 {
        self.integrity_rid
    }
    #[must_use]
    pub const fn process_id(&self) -> u32 {
        self.process_id
    }
}

impl core::fmt::Debug for TokenIdentity {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("TokenIdentity")
            .field("user_sid", &self.user_sid)
            .field("logon_sid", &self.logon_sid)
            .field("session_id", &self.session_id)
            .field("integrity_rid", &self.integrity_rid)
            .field("process_id", &self.process_id)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerMismatch {
    UserSid,
    LogonSid,
    Session,
    Integrity,
    ProcessId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedPeer {
    process_id: u32,
    session_id: u32,
    integrity_rid: u32,
}

impl ValidatedPeer {
    #[must_use]
    pub const fn process_id(self) -> u32 {
        self.process_id
    }
    #[must_use]
    pub const fn session_id(self) -> u32 {
        self.session_id
    }
    #[must_use]
    pub const fn integrity_rid(self) -> u32 {
        self.integrity_rid
    }
}

pub fn current_token_identity() -> Result<TokenIdentity, TransportError> {
    process_token_identity(sys::current_process_id())
}

pub fn process_token_identity(process_id: u32) -> Result<TokenIdentity, TransportError> {
    sys::query_process_token_identity(process_id)
}

pub fn validate_peer(
    expected_current: &TokenIdentity,
    kernel_process_id: u32,
    kernel_session_id: u32,
    observed: &TokenIdentity,
) -> Result<ValidatedPeer, PeerMismatch> {
    if kernel_process_id == 0 || observed.process_id != kernel_process_id {
        return Err(PeerMismatch::ProcessId);
    }
    if expected_current.user_sid != observed.user_sid {
        return Err(PeerMismatch::UserSid);
    }
    if expected_current.logon_sid != observed.logon_sid {
        return Err(PeerMismatch::LogonSid);
    }
    if expected_current.session_id != kernel_session_id || observed.session_id != kernel_session_id
    {
        return Err(PeerMismatch::Session);
    }
    if expected_current.integrity_rid != observed.integrity_rid {
        return Err(PeerMismatch::Integrity);
    }
    Ok(ValidatedPeer {
        process_id: observed.process_id,
        session_id: observed.session_id,
        integrity_rid: observed.integrity_rid,
    })
}
