use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fidelity {
    FullFidelity,
    CommonFormatsPreserved,
    FallbackOnly,
    ReferenceOnly,
    Unavailable,
    UnsafeOrUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FidelityAggregationVersion {
    V1,
}

pub fn aggregate_fidelity_v1(values: &[Fidelity]) -> Result<Fidelity, DomainError> {
    if values.is_empty() {
        return Err(DomainError::FidelityInputEmpty);
    }
    let usable: Vec<Fidelity> = values
        .iter()
        .copied()
        .filter(|value| !matches!(value, Fidelity::Unavailable | Fidelity::UnsafeOrUnsupported))
        .collect();
    if usable.is_empty() {
        return Err(DomainError::FidelityHasNoUsableRepresentation);
    }
    if usable.iter().all(|value| *value == Fidelity::FullFidelity) && usable.len() == values.len() {
        return Ok(Fidelity::FullFidelity);
    }
    if usable.iter().any(|value| {
        matches!(
            value,
            Fidelity::FullFidelity | Fidelity::CommonFormatsPreserved
        )
    }) {
        return Ok(Fidelity::CommonFormatsPreserved);
    }
    if usable.contains(&Fidelity::FallbackOnly) {
        return Ok(Fidelity::FallbackOnly);
    }
    Ok(Fidelity::ReferenceOnly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregation_is_deterministic_for_primary_combinations() {
        assert_eq!(
            aggregate_fidelity_v1(&[Fidelity::FullFidelity]).unwrap(),
            Fidelity::FullFidelity
        );
        assert_eq!(
            aggregate_fidelity_v1(&[Fidelity::FullFidelity, Fidelity::FallbackOnly]).unwrap(),
            Fidelity::CommonFormatsPreserved
        );
        assert_eq!(
            aggregate_fidelity_v1(&[Fidelity::FallbackOnly, Fidelity::ReferenceOnly]).unwrap(),
            Fidelity::FallbackOnly
        );
        assert_eq!(
            aggregate_fidelity_v1(&[Fidelity::ReferenceOnly]).unwrap(),
            Fidelity::ReferenceOnly
        );
    }

    #[test]
    fn empty_or_unusable_inputs_are_rejected() {
        assert_eq!(
            aggregate_fidelity_v1(&[]),
            Err(DomainError::FidelityInputEmpty)
        );
        assert_eq!(
            aggregate_fidelity_v1(&[Fidelity::Unavailable, Fidelity::UnsafeOrUnsupported]),
            Err(DomainError::FidelityHasNoUsableRepresentation)
        );
    }
}
