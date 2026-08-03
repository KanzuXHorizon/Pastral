use crate::ProtectionDomainId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtectionDomain {
    Ordinary(ProtectionDomainId),
    Sensitive(ProtectionDomainId),
    Private(ProtectionDomainId),
}

impl ProtectionDomain {
    #[must_use]
    pub const fn permits_persistent_plaintext_digest(self) -> bool {
        matches!(self, Self::Ordinary(_))
    }

    #[must_use]
    pub fn permits_digest_dedup_with(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Ordinary(left), Self::Ordinary(right)) if left == right
        )
    }

    #[must_use]
    pub const fn id(self) -> ProtectionDomainId {
        match self {
            Self::Ordinary(id) | Self::Sensitive(id) | Self::Private(id) => id,
        }
    }
}
