use hmac::{Hmac, Mac};
use pastral_ipc_core::{Capability, CorrelationId};
use sha2::Sha256;

use crate::{AuthError, AuthenticationProof, InstallationSecret, Nonce};

type HmacSha256 = Hmac<Sha256>;

const TRANSCRIPT_CONTEXT: &[u8] = b"Pastral IPC authentication transcript";
const TRANSCRIPT_VERSION: u32 = 1;
const CLIENT_ROLE: &[u8] = b"client";
const SERVER_ROLE: &[u8] = b"server";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofRole {
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerTranscriptIdentity {
    process_id: u32,
    session_id: u32,
    integrity_rid: u32,
}

impl PeerTranscriptIdentity {
    pub const fn new(
        process_id: u32,
        session_id: u32,
        integrity_rid: u32,
    ) -> Result<Self, AuthError> {
        if process_id == 0 {
            return Err(AuthError::InvalidPeerIdentity("process ID must be nonzero"));
        }
        if integrity_rid == 0 {
            return Err(AuthError::InvalidPeerIdentity(
                "integrity RID must be nonzero",
            ));
        }
        Ok(Self {
            process_id,
            session_id,
            integrity_rid,
        })
    }

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

#[derive(Clone, PartialEq, Eq)]
pub struct HandshakeTranscript {
    schema_digest: [u8; 32],
    protocol_major: u32,
    server_min_minor: u32,
    server_max_minor: u32,
    client_min_minor: u32,
    client_max_minor: u32,
    selected_minor: u32,
    server_nonce: Nonce,
    client_nonce: Nonce,
    instance_id: CorrelationId,
    server_identity: PeerTranscriptIdentity,
    client_identity: PeerTranscriptIdentity,
    requested_capabilities: Vec<Capability>,
    accepted_capabilities: Vec<Capability>,
}

impl HandshakeTranscript {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema_digest: [u8; 32],
        protocol_major: u32,
        server_min_minor: u32,
        server_max_minor: u32,
        client_min_minor: u32,
        client_max_minor: u32,
        selected_minor: u32,
        server_nonce: Nonce,
        client_nonce: Nonce,
        instance_id: CorrelationId,
        server_identity: PeerTranscriptIdentity,
        client_identity: PeerTranscriptIdentity,
        requested_capabilities: impl IntoIterator<Item = Capability>,
        accepted_capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, AuthError> {
        let requested_capabilities = normalize_capabilities(
            requested_capabilities,
            "requested capabilities must not be empty",
            "requested capability is duplicated",
        )?;
        let accepted_capabilities = normalize_capabilities(
            accepted_capabilities,
            "accepted capabilities must not be empty",
            "accepted capability is duplicated",
        )?;
        let value = Self {
            schema_digest,
            protocol_major,
            server_min_minor,
            server_max_minor,
            client_min_minor,
            client_max_minor,
            selected_minor,
            server_nonce,
            client_nonce,
            instance_id,
            server_identity,
            client_identity,
            requested_capabilities,
            accepted_capabilities,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn with_schema_digest(&self, value: [u8; 32]) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.schema_digest = value;
        next.validate()?;
        Ok(next)
    }

    pub fn with_protocol_major(&self, value: u32) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.protocol_major = value;
        next.validate()?;
        Ok(next)
    }

    pub fn with_server_minor_range(&self, min: u32, max: u32) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.server_min_minor = min;
        next.server_max_minor = max;
        next.validate()?;
        Ok(next)
    }

    pub fn with_client_minor_range(&self, min: u32, max: u32) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.client_min_minor = min;
        next.client_max_minor = max;
        next.validate()?;
        Ok(next)
    }

    pub fn with_selected_minor(&self, value: u32) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.selected_minor = value;
        next.validate()?;
        Ok(next)
    }

    #[must_use]
    pub fn with_server_nonce(&self, value: Nonce) -> Self {
        let mut next = self.clone();
        next.server_nonce = value;
        next
    }

    #[must_use]
    pub fn with_client_nonce(&self, value: Nonce) -> Self {
        let mut next = self.clone();
        next.client_nonce = value;
        next
    }

    pub fn with_instance_id(&self, value: CorrelationId) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.instance_id = value;
        next.validate()?;
        Ok(next)
    }

    #[must_use]
    pub fn with_server_identity(&self, value: PeerTranscriptIdentity) -> Self {
        let mut next = self.clone();
        next.server_identity = value;
        next
    }

    #[must_use]
    pub fn with_client_identity(&self, value: PeerTranscriptIdentity) -> Self {
        let mut next = self.clone();
        next.client_identity = value;
        next
    }

    pub fn with_requested_capabilities(
        &self,
        values: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.requested_capabilities = normalize_capabilities(
            values,
            "requested capabilities must not be empty",
            "requested capability is duplicated",
        )?;
        next.validate()?;
        Ok(next)
    }

    pub fn with_accepted_capabilities(
        &self,
        values: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, AuthError> {
        let mut next = self.clone();
        next.accepted_capabilities = normalize_capabilities(
            values,
            "accepted capabilities must not be empty",
            "accepted capability is duplicated",
        )?;
        next.validate()?;
        Ok(next)
    }

    #[must_use]
    pub const fn server_nonce(&self) -> &Nonce {
        &self.server_nonce
    }

    #[must_use]
    pub const fn client_nonce(&self) -> &Nonce {
        &self.client_nonce
    }

    fn validate(&self) -> Result<(), AuthError> {
        if self.schema_digest.iter().all(|byte| *byte == 0) {
            return Err(AuthError::InvalidTranscript(
                "schema digest must not be all zero",
            ));
        }
        if self.protocol_major == 0 {
            return Err(AuthError::InvalidTranscript(
                "protocol major must be nonzero",
            ));
        }
        if self.server_min_minor > self.server_max_minor {
            return Err(AuthError::InvalidTranscript(
                "server minor range is invalid",
            ));
        }
        if self.client_min_minor > self.client_max_minor {
            return Err(AuthError::InvalidTranscript(
                "client minor range is invalid",
            ));
        }
        if self.selected_minor < self.server_min_minor
            || self.selected_minor > self.server_max_minor
            || self.selected_minor < self.client_min_minor
            || self.selected_minor > self.client_max_minor
        {
            return Err(AuthError::InvalidTranscript(
                "selected minor is outside negotiated ranges",
            ));
        }
        if self.instance_id.is_zero() {
            return Err(AuthError::InvalidTranscript("instance ID must be nonzero"));
        }
        for accepted in &self.accepted_capabilities {
            if !self.requested_capabilities.contains(accepted) {
                return Err(AuthError::InvalidTranscript(
                    "accepted capability was not requested",
                ));
            }
        }
        Ok(())
    }

    fn canonical_bytes(&self, role: ProofRole) -> Result<Vec<u8>, AuthError> {
        let requested_bytes = self
            .requested_capabilities
            .len()
            .checked_mul(4)
            .ok_or(AuthError::IntegerOverflow)?;
        let accepted_bytes = self
            .accepted_capabilities
            .len()
            .checked_mul(4)
            .ok_or(AuthError::IntegerOverflow)?;
        let capacity = 64usize
            .checked_add(TRANSCRIPT_CONTEXT.len())
            .and_then(|value| value.checked_add(requested_bytes))
            .and_then(|value| value.checked_add(accepted_bytes))
            .ok_or(AuthError::IntegerOverflow)?;
        let mut bytes = Vec::with_capacity(capacity);
        append_bytes(&mut bytes, TRANSCRIPT_CONTEXT)?;
        append_u32(&mut bytes, TRANSCRIPT_VERSION);
        bytes.extend_from_slice(&self.schema_digest);
        append_u32(&mut bytes, self.protocol_major);
        append_u32(&mut bytes, self.server_min_minor);
        append_u32(&mut bytes, self.server_max_minor);
        append_u32(&mut bytes, self.client_min_minor);
        append_u32(&mut bytes, self.client_max_minor);
        append_u32(&mut bytes, self.selected_minor);
        bytes.extend_from_slice(self.server_nonce.as_bytes());
        bytes.extend_from_slice(self.client_nonce.as_bytes());
        bytes.extend_from_slice(self.instance_id.as_bytes());
        append_peer(&mut bytes, self.server_identity);
        append_peer(&mut bytes, self.client_identity);
        append_capabilities(&mut bytes, &self.requested_capabilities)?;
        append_capabilities(&mut bytes, &self.accepted_capabilities)?;
        append_bytes(
            &mut bytes,
            match role {
                ProofRole::Client => CLIENT_ROLE,
                ProofRole::Server => SERVER_ROLE,
            },
        )?;
        Ok(bytes)
    }
}

#[must_use]
pub fn compute_proof(
    secret: &InstallationSecret,
    transcript: &HandshakeTranscript,
    role: ProofRole,
) -> AuthenticationProof {
    let bytes = transcript
        .canonical_bytes(role)
        .expect("validated bounded transcript cannot overflow");
    secret.expose(|secret_bytes| {
        let mut mac =
            HmacSha256::new_from_slice(secret_bytes).expect("HMAC-SHA256 accepts a 32-byte key");
        mac.update(&bytes);
        AuthenticationProof::from_bytes(mac.finalize().into_bytes().into())
    })
}

pub fn verify_proof(
    secret: &InstallationSecret,
    transcript: &HandshakeTranscript,
    role: ProofRole,
    proof: &AuthenticationProof,
) -> Result<(), AuthError> {
    let bytes = transcript.canonical_bytes(role)?;
    secret.expose(|secret_bytes| {
        let mut mac =
            HmacSha256::new_from_slice(secret_bytes).expect("HMAC-SHA256 accepts a 32-byte key");
        mac.update(&bytes);
        mac.verify_slice(proof.as_bytes())
            .map_err(|_| AuthError::ProofMismatch)
    })
}

fn normalize_capabilities(
    values: impl IntoIterator<Item = Capability>,
    empty_reason: &'static str,
    duplicate_reason: &'static str,
) -> Result<Vec<Capability>, AuthError> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return Err(AuthError::InvalidTranscript(empty_reason));
    }
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(AuthError::InvalidTranscript(duplicate_reason));
    }
    Ok(values)
}

fn append_peer(bytes: &mut Vec<u8>, peer: PeerTranscriptIdentity) {
    append_u32(bytes, peer.process_id());
    append_u32(bytes, peer.session_id());
    append_u32(bytes, peer.integrity_rid());
}

fn append_capabilities(bytes: &mut Vec<u8>, capabilities: &[Capability]) -> Result<(), AuthError> {
    append_u32(
        bytes,
        u32::try_from(capabilities.len()).map_err(|_| AuthError::IntegerOverflow)?,
    );
    for capability in capabilities {
        append_u32(bytes, capability_id(*capability));
    }
    Ok(())
}

fn capability_id(value: Capability) -> u32 {
    match value {
        Capability::Health => 1,
        Capability::HistoryPage => 2,
        Capability::Search => 3,
    }
}

fn append_bytes(target: &mut Vec<u8>, value: &[u8]) -> Result<(), AuthError> {
    append_u32(
        target,
        u32::try_from(value.len()).map_err(|_| AuthError::IntegerOverflow)?,
    );
    target.extend_from_slice(value);
    Ok(())
}

fn append_u32(target: &mut Vec<u8>, value: u32) {
    target.extend_from_slice(&value.to_le_bytes());
}
