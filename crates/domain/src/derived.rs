use crate::{
    ClipEventId, ClipRepresentationId, DerivedRepresentationId, DomainError, ProtectionDomain,
    RawDigest, TransformationId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRepresentationProvenance {
    id: DerivedRepresentationId,
    parent_event_id: ClipEventId,
    parent_representation_id: ClipRepresentationId,
    transformation_id: TransformationId,
    transformation_version: u32,
    output_protection_domain: ProtectionDomain,
    output_digest: Option<RawDigest>,
}

impl DerivedRepresentationProvenance {
    pub fn new(
        id: DerivedRepresentationId,
        parent_event_id: ClipEventId,
        parent_representation_id: ClipRepresentationId,
        transformation_id: TransformationId,
        transformation_version: u32,
        output_protection_domain: ProtectionDomain,
        output_digest: Option<RawDigest>,
    ) -> Result<Self, DomainError> {
        if transformation_version == 0 {
            return Err(DomainError::TransformationVersionMustBePositive);
        }
        if output_digest.is_some_and(|value| value.protection_domain() != output_protection_domain)
        {
            return Err(DomainError::DigestProtectionDomainMismatch);
        }
        Ok(Self {
            id,
            parent_event_id,
            parent_representation_id,
            transformation_id,
            transformation_version,
            output_protection_domain,
            output_digest,
        })
    }

    #[must_use]
    pub const fn id(&self) -> DerivedRepresentationId {
        self.id
    }

    #[must_use]
    pub const fn parent_event_id(&self) -> ClipEventId {
        self.parent_event_id
    }

    #[must_use]
    pub const fn parent_representation_id(&self) -> ClipRepresentationId {
        self.parent_representation_id
    }

    #[must_use]
    pub const fn transformation_id(&self) -> TransformationId {
        self.transformation_id
    }

    #[must_use]
    pub const fn transformation_version(&self) -> u32 {
        self.transformation_version
    }

    #[must_use]
    pub const fn output_protection_domain(&self) -> ProtectionDomain {
        self.output_protection_domain
    }

    #[must_use]
    pub const fn output_digest(&self) -> Option<&RawDigest> {
        self.output_digest.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtectionDomainId;

    #[test]
    fn provenance_preserves_parents_and_version() {
        let event = ClipEventId::new_v4();
        let representation = ClipRepresentationId::new_v4();
        let value = DerivedRepresentationProvenance::new(
            DerivedRepresentationId::new_v4(),
            event,
            representation,
            TransformationId::new_v4(),
            1,
            ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
            None,
        )
        .unwrap();
        assert_eq!(value.parent_event_id(), event);
        assert_eq!(value.parent_representation_id(), representation);
        assert_eq!(value.transformation_version(), 1);
    }

    #[test]
    fn zero_transformation_version_is_rejected() {
        let result = DerivedRepresentationProvenance::new(
            DerivedRepresentationId::new_v4(),
            ClipEventId::new_v4(),
            ClipRepresentationId::new_v4(),
            TransformationId::new_v4(),
            0,
            ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
            None,
        );
        assert_eq!(
            result,
            Err(DomainError::TransformationVersionMustBePositive)
        );
    }
}
