use crate::{
    CaptureOrder, ClipEventId, ClipRepresentation, DomainError, ProfileId, ProtectionDomain,
    UtcUnixMicros,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipEvent {
    id: ClipEventId,
    observed_at: UtcUnixMicros,
    capture_order: CaptureOrder,
    captured_profile_id: ProfileId,
    captured_protection_domain: ProtectionDomain,
    representations: Vec<ClipRepresentation>,
}

impl ClipEvent {
    pub fn new(
        id: ClipEventId,
        observed_at: UtcUnixMicros,
        capture_order: CaptureOrder,
        captured_profile_id: ProfileId,
        captured_protection_domain: ProtectionDomain,
        representations: Vec<ClipRepresentation>,
    ) -> Result<Self, DomainError> {
        if representations.is_empty() {
            return Err(DomainError::ClipEventRequiresRepresentation);
        }
        if representations
            .iter()
            .any(|value| !value.is_usable_capture())
        {
            return Err(DomainError::UnusableCapturedRepresentation);
        }
        Ok(Self {
            id,
            observed_at,
            capture_order,
            captured_profile_id,
            captured_protection_domain,
            representations,
        })
    }

    #[must_use]
    pub const fn id(&self) -> ClipEventId {
        self.id
    }

    #[must_use]
    pub const fn observed_at(&self) -> UtcUnixMicros {
        self.observed_at
    }

    #[must_use]
    pub const fn capture_order(&self) -> CaptureOrder {
        self.capture_order
    }

    #[must_use]
    pub const fn captured_profile_id(&self) -> ProfileId {
        self.captured_profile_id
    }

    #[must_use]
    pub const fn captured_protection_domain(&self) -> ProtectionDomain {
        self.captured_protection_domain
    }

    #[must_use]
    pub fn representations(&self) -> &[ClipRepresentation] {
        &self.representations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClipRepresentationId, ClipboardFormatIdentity, Fidelity, ProtectionDomainId,
        StandardFormatId,
    };

    fn representation(fidelity: Fidelity) -> ClipRepresentation {
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        ClipRepresentation::new(
            ClipRepresentationId::new_v4(),
            ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
            domain,
            1,
            None,
            fidelity,
        )
        .unwrap()
    }

    fn event(representations: Vec<ClipRepresentation>) -> Result<ClipEvent, DomainError> {
        ClipEvent::new(
            ClipEventId::new_v4(),
            UtcUnixMicros::new(0).unwrap(),
            CaptureOrder::new(1).unwrap(),
            ProfileId::new_v4(),
            ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
            representations,
        )
    }

    #[test]
    fn zero_representations_are_rejected() {
        assert_eq!(
            event(vec![]),
            Err(DomainError::ClipEventRequiresRepresentation)
        );
    }

    #[test]
    fn one_and_many_representations_are_accepted() {
        assert!(event(vec![representation(Fidelity::FullFidelity)]).is_ok());
        assert!(
            event(vec![
                representation(Fidelity::FullFidelity),
                representation(Fidelity::FallbackOnly),
            ])
            .is_ok()
        );
    }

    #[test]
    fn unavailable_descriptor_cannot_satisfy_cardinality() {
        assert_eq!(
            event(vec![representation(Fidelity::Unavailable)]),
            Err(DomainError::UnusableCapturedRepresentation)
        );
    }

    #[test]
    fn identical_payload_context_never_merges_copy_occurrences() {
        let first = event(vec![representation(Fidelity::FullFidelity)]).unwrap();
        let second = event(vec![representation(Fidelity::FullFidelity)]).unwrap();
        assert_ne!(first.id(), second.id());
    }
}
