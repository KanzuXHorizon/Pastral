use core::{fmt, str::FromStr};
use uuid::Uuid;

use crate::DomainError;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(value: Uuid) -> Result<Self, DomainError> {
                if value.get_version_num() != 4 {
                    return Err(DomainError::InvalidUuidVersion);
                }
                Ok(Self(value))
            }

            pub fn parse_str(value: &str) -> Result<Self, DomainError> {
                let uuid = Uuid::parse_str(value).map_err(|_| DomainError::InvalidUuidText)?;
                Self::from_uuid(uuid)
            }

            #[must_use]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0.hyphenated())
            }
        }

        impl FromStr for $name {
            type Err = DomainError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse_str(value)
            }
        }
    };
}

typed_id!(BlobObjectId);
typed_id!(ClipEventId);
typed_id!(ClipRepresentationId);
typed_id!(CaptureAuditEventId);
typed_id!(DerivedRepresentationId);
typed_id!(ProfileId);
typed_id!(ProtectionDomainId);
typed_id!(TransformationId);

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{Variant, Version};

    fn accepts_clip_event_id(_: ClipEventId) {}

    #[test]
    fn generated_ids_are_uuid_v4_rfc4122() {
        let id = ClipEventId::new_v4();
        assert_eq!(id.as_uuid().get_version(), Some(Version::Random));
        assert_eq!(id.as_uuid().get_variant(), Variant::RFC4122);
    }

    #[test]
    fn text_is_canonical_lowercase_hyphenated() {
        let id = ClipEventId::parse_str("550E8400-E29B-41D4-A716-446655440000").unwrap();
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn non_v4_uuid_is_rejected() {
        let v1 = Uuid::parse_str("550e8400-e29b-11d4-a716-446655440000").unwrap();
        assert_eq!(
            ClipEventId::from_uuid(v1),
            Err(DomainError::InvalidUuidVersion)
        );
    }

    #[test]
    fn typed_api_boundary_accepts_only_expected_id() {
        accepts_clip_event_id(ClipEventId::new_v4());
        let representation = ClipRepresentationId::new_v4();
        assert_ne!(ClipEventId::new_v4().as_uuid(), representation.as_uuid());
    }
}
