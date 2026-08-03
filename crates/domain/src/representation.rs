use crate::{
    ClipRepresentationId, ClipboardFormatIdentity, DomainError, Fidelity, ProtectionDomain,
    RawDigest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipRepresentation {
    id: ClipRepresentationId,
    format: ClipboardFormatIdentity,
    protection_domain: ProtectionDomain,
    raw_logical_length: u64,
    digest: Option<RawDigest>,
    fidelity: Fidelity,
}

impl ClipRepresentation {
    pub fn new(
        id: ClipRepresentationId,
        format: ClipboardFormatIdentity,
        protection_domain: ProtectionDomain,
        raw_logical_length: u64,
        digest: Option<RawDigest>,
        fidelity: Fidelity,
    ) -> Result<Self, DomainError> {
        if digest.is_some_and(|value| value.protection_domain() != protection_domain) {
            return Err(DomainError::DigestProtectionDomainMismatch);
        }
        Ok(Self {
            id,
            format,
            protection_domain,
            raw_logical_length,
            digest,
            fidelity,
        })
    }

    #[must_use]
    pub const fn id(&self) -> ClipRepresentationId {
        self.id
    }

    #[must_use]
    pub const fn format(&self) -> &ClipboardFormatIdentity {
        &self.format
    }

    #[must_use]
    pub const fn protection_domain(&self) -> ProtectionDomain {
        self.protection_domain
    }

    #[must_use]
    pub const fn raw_logical_length(&self) -> u64 {
        self.raw_logical_length
    }

    #[must_use]
    pub const fn digest(&self) -> Option<&RawDigest> {
        self.digest.as_ref()
    }

    #[must_use]
    pub const fn fidelity(&self) -> Fidelity {
        self.fidelity
    }

    #[must_use]
    pub const fn is_usable_capture(&self) -> bool {
        !matches!(
            self.fidelity,
            Fidelity::Unavailable | Fidelity::UnsafeOrUnsupported
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProtectionDomainId, StandardFormatId};

    #[test]
    fn digest_must_match_representation_domain() {
        let first = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let second = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let digest = RawDigest::sha256_raw_v1(first, b"x").unwrap();
        let result = ClipRepresentation::new(
            ClipRepresentationId::new_v4(),
            ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
            second,
            1,
            Some(digest),
            Fidelity::FullFidelity,
        );
        assert_eq!(result, Err(DomainError::DigestProtectionDomainMismatch));
    }
}
