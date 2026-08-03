use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidUuidText,
    InvalidUuidVersion,
    UtcMicrosOutOfRange,
    CaptureOrderMustBePositive,
    RegisteredFormatNameEmpty,
    RegisteredFormatNameContainsNul,
    PersistentPlaintextDigestForbidden,
    DigestProtectionDomainMismatch,
    ClipEventRequiresRepresentation,
    RepresentationProtectionDomainMismatch,
    UnusableCapturedRepresentation,
    FidelityInputEmpty,
    FidelityHasNoUsableRepresentation,
    TransformationVersionMustBePositive,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidUuidText => "UUID text is invalid",
            Self::InvalidUuidVersion => "UUID must be version 4",
            Self::UtcMicrosOutOfRange => "UTC Unix microseconds are outside the supported range",
            Self::CaptureOrderMustBePositive => "capture order must be strictly positive",
            Self::RegisteredFormatNameEmpty => "registered clipboard format name must not be empty",
            Self::RegisteredFormatNameContainsNul => {
                "registered clipboard format name must not contain NUL"
            }
            Self::PersistentPlaintextDigestForbidden => {
                "persistent plaintext digest is forbidden for this protection domain"
            }
            Self::DigestProtectionDomainMismatch => {
                "digest protection domain does not match the representation"
            }
            Self::ClipEventRequiresRepresentation => {
                "clip event requires at least one captured representation"
            }
            Self::RepresentationProtectionDomainMismatch => {
                "captured representation protection domain does not match the clip event"
            }
            Self::UnusableCapturedRepresentation => {
                "unavailable or unsupported descriptors are not captured representations"
            }
            Self::FidelityInputEmpty => "fidelity aggregation requires at least one value",
            Self::FidelityHasNoUsableRepresentation => {
                "fidelity aggregation has no usable representation"
            }
            Self::TransformationVersionMustBePositive => "transformation version must be positive",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DomainError {}
