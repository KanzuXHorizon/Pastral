use sha2::{Digest as _, Sha256};

use crate::{DomainError, ProtectionDomain};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestSuite {
    Sha256RawV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawDigest {
    suite: DigestSuite,
    bytes: [u8; 32],
    protection_domain: ProtectionDomain,
}

impl RawDigest {
    pub fn sha256_raw_v1(
        protection_domain: ProtectionDomain,
        logical_bytes: &[u8],
    ) -> Result<Self, DomainError> {
        if !protection_domain.permits_persistent_plaintext_digest() {
            return Err(DomainError::PersistentPlaintextDigestForbidden);
        }
        let bytes: [u8; 32] = Sha256::digest(logical_bytes).into();
        Ok(Self {
            suite: DigestSuite::Sha256RawV1,
            bytes,
            protection_domain,
        })
    }

    #[must_use]
    pub const fn suite(&self) -> DigestSuite {
        self.suite
    }

    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    #[must_use]
    pub const fn protection_domain(&self) -> ProtectionDomain {
        self.protection_domain
    }

    #[must_use]
    pub fn can_deduplicate_with(&self, other: &Self) -> bool {
        self.suite == other.suite
            && self.bytes == other.bytes
            && self
                .protection_domain
                .permits_digest_dedup_with(other.protection_domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtectionDomainId;

    fn ordinary() -> ProtectionDomain {
        ProtectionDomain::Ordinary(ProtectionDomainId::new_v4())
    }

    #[test]
    fn exact_bytes_are_stable_and_one_byte_changes_digest() {
        let domain = ordinary();
        let a = RawDigest::sha256_raw_v1(domain, b"abc").unwrap();
        let b = RawDigest::sha256_raw_v1(domain, b"abc").unwrap();
        let c = RawDigest::sha256_raw_v1(domain, b"abd").unwrap();
        assert_eq!(a, b);
        assert_ne!(a.bytes(), c.bytes());
    }

    #[test]
    fn text_is_not_normalized() {
        let domain = ordinary();
        let composed = RawDigest::sha256_raw_v1(domain, "é".as_bytes()).unwrap();
        let decomposed = RawDigest::sha256_raw_v1(domain, "e\u{301}".as_bytes()).unwrap();
        assert_ne!(composed.bytes(), decomposed.bytes());
    }

    #[test]
    fn sensitive_and_private_plaintext_digests_are_rejected() {
        for domain in [
            ProtectionDomain::Sensitive(ProtectionDomainId::new_v4()),
            ProtectionDomain::Private(ProtectionDomainId::new_v4()),
        ] {
            assert_eq!(
                RawDigest::sha256_raw_v1(domain, b"secret"),
                Err(DomainError::PersistentPlaintextDigestForbidden)
            );
        }
    }

    #[test]
    fn deduplication_never_crosses_protection_domain() {
        let a = RawDigest::sha256_raw_v1(ordinary(), b"same").unwrap();
        let b = RawDigest::sha256_raw_v1(ordinary(), b"same").unwrap();
        assert!(!a.can_deduplicate_with(&b));
    }
}
