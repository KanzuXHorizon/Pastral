use std::collections::{HashMap, HashSet};

use pastral_domain::{
    ClipEvent, ClipRepresentation, ClipRepresentationId, Fidelity, ProtectionDomain, RawDigest,
    aggregate_fidelity_v1,
};

use crate::{StorageError, StorageLimits};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationPayload {
    representation_id: ClipRepresentationId,
    bytes: Vec<u8>,
}

impl RepresentationPayload {
    #[must_use]
    pub fn new(representation_id: ClipRepresentationId, bytes: Vec<u8>) -> Self {
        Self {
            representation_id,
            bytes,
        }
    }

    #[must_use]
    pub const fn representation_id(&self) -> ClipRepresentationId {
        self.representation_id
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchProjection {
    text: String,
}

impl SearchProjection {
    pub fn new(text: impl Into<String>, limits: StorageLimits) -> Result<Self, StorageError> {
        let text = text.into();
        validate_search_projection(&text, limits)?;
        Ok(Self { text })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipCommit {
    event: ClipEvent,
    payloads: Vec<RepresentationPayload>,
    search_projection: Option<SearchProjection>,
}

impl ClipCommit {
    #[must_use]
    pub fn new(
        event: ClipEvent,
        payloads: Vec<RepresentationPayload>,
        search_projection: Option<SearchProjection>,
    ) -> Self {
        Self {
            event,
            payloads,
            search_projection,
        }
    }

    #[must_use]
    pub const fn event(&self) -> &ClipEvent {
        &self.event
    }

    #[must_use]
    pub fn payloads(&self) -> &[RepresentationPayload] {
        &self.payloads
    }

    #[must_use]
    pub const fn search_projection(&self) -> Option<&SearchProjection> {
        self.search_projection.as_ref()
    }
}

pub(crate) struct ValidatedRepresentation<'a> {
    pub representation: &'a ClipRepresentation,
    pub bytes: &'a [u8],
    pub digest: RawDigest,
}

pub(crate) struct ValidatedCommit<'a> {
    pub event: &'a ClipEvent,
    pub representations: Vec<ValidatedRepresentation<'a>>,
    pub aggregate_fidelity: Fidelity,
    pub search_projection: Option<&'a SearchProjection>,
}

pub(crate) fn validate_commit<'a>(
    commit: &'a ClipCommit,
    limits: StorageLimits,
) -> Result<ValidatedCommit<'a>, StorageError> {
    if !matches!(
        commit.event().captured_protection_domain(),
        ProtectionDomain::Ordinary(_)
    ) {
        let id = commit
            .event()
            .representations()
            .first()
            .map(ClipRepresentation::id)
            .ok_or_else(|| StorageError::Domain("clip event has no representation".into()))?;
        return Err(StorageError::ProtectedPayloadRequiresEncryption(id));
    }

    if let Some(projection) = commit.search_projection() {
        validate_search_projection(projection.as_str(), limits)?;
    }

    let event_ids: HashSet<ClipRepresentationId> = commit
        .event()
        .representations()
        .iter()
        .map(ClipRepresentation::id)
        .collect();
    let mut payloads = HashMap::with_capacity(commit.payloads().len());
    for payload in commit.payloads() {
        let id = payload.representation_id();
        if !event_ids.contains(&id) {
            return Err(StorageError::PayloadMappingUnknown(id));
        }
        if payloads.insert(id, payload).is_some() {
            return Err(StorageError::PayloadMappingDuplicate(id));
        }
    }

    let mut representations = Vec::with_capacity(commit.event().representations().len());
    for representation in commit.event().representations() {
        let id = representation.id();
        let payload = payloads
            .get(&id)
            .copied()
            .ok_or(StorageError::PayloadMappingMissing(id))?;
        if representation.protection_domain() != commit.event().captured_protection_domain() {
            return Err(StorageError::Domain(
                "representation protection domain differs from captured event domain".into(),
            ));
        }
        if !matches!(
            representation.protection_domain(),
            ProtectionDomain::Ordinary(_)
        ) {
            return Err(StorageError::ProtectedPayloadRequiresEncryption(id));
        }
        let actual_length = u64::try_from(payload.bytes().len())
            .map_err(|_| StorageError::IntegerOutOfRange("payload length"))?;
        if actual_length > limits.max_payload_bytes() {
            return Err(StorageError::PayloadLimitExceeded(id));
        }
        if actual_length != representation.raw_logical_length() {
            return Err(StorageError::PayloadLengthMismatch(id));
        }
        let digest = RawDigest::sha256_raw_v1(representation.protection_domain(), payload.bytes())
            .map_err(|error| StorageError::Domain(error.to_string()))?;
        if representation.digest() != Some(&digest) {
            return Err(StorageError::PayloadDigestMismatch(id));
        }
        representations.push(ValidatedRepresentation {
            representation,
            bytes: payload.bytes(),
            digest,
        });
    }

    let aggregate_fidelity = aggregate_fidelity_v1(
        &commit
            .event()
            .representations()
            .iter()
            .map(ClipRepresentation::fidelity)
            .collect::<Vec<_>>(),
    )
    .map_err(|error| StorageError::Domain(error.to_string()))?;

    Ok(ValidatedCommit {
        event: commit.event(),
        representations,
        aggregate_fidelity,
        search_projection: commit.search_projection(),
    })
}

fn validate_search_projection(text: &str, limits: StorageLimits) -> Result<(), StorageError> {
    if text.is_empty() || text.trim_matches(char::is_whitespace).is_empty() {
        return Err(StorageError::SearchProjectionInvalid("empty"));
    }
    if text.contains('\0') {
        return Err(StorageError::SearchProjectionInvalid("contains NUL"));
    }
    if text.len() > limits.max_search_projection_bytes() {
        return Err(StorageError::SearchProjectionInvalid("too large"));
    }
    Ok(())
}
