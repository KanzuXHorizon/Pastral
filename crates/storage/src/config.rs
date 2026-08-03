use pastral_domain::{ClipboardFormatIdentity, ProtectionDomain};

use crate::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoragePolicyVersion(u32);

impl StoragePolicyVersion {
    pub fn new(value: u32) -> Result<Self, StorageError> {
        if value == 0 {
            return Err(StorageError::InvalidStoragePolicyVersion);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlobPlacement {
    InternalSqlite,
    ExternalFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPlacementContext {
    raw_length: u64,
    protection_domain: ProtectionDomain,
    format: ClipboardFormatIdentity,
}

impl BlobPlacementContext {
    #[must_use]
    pub const fn raw_length(&self) -> u64 {
        self.raw_length
    }

    #[must_use]
    pub const fn protection_domain(&self) -> ProtectionDomain {
        self.protection_domain
    }

    #[must_use]
    pub const fn format(&self) -> &ClipboardFormatIdentity {
        &self.format
    }

    pub(crate) fn new(
        raw_length: u64,
        protection_domain: ProtectionDomain,
        format: ClipboardFormatIdentity,
    ) -> Self {
        Self {
            raw_length,
            protection_domain,
            format,
        }
    }
}

pub trait BlobPlacementPolicy {
    fn version(&self) -> StoragePolicyVersion;
    fn select(&self, context: &BlobPlacementContext) -> BlobPlacement;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageLimits {
    max_payload_bytes: u64,
    max_search_projection_bytes: usize,
    max_search_query_bytes: usize,
    max_search_terms: usize,
    reconciliation_entry_limit: usize,
}

impl StorageLimits {
    pub fn new(
        max_payload_bytes: u64,
        max_search_projection_bytes: usize,
        max_search_query_bytes: usize,
        max_search_terms: usize,
        reconciliation_entry_limit: usize,
    ) -> Result<Self, StorageError> {
        if max_payload_bytes == 0 {
            return Err(StorageError::InvalidStorageLimit("max_payload_bytes"));
        }
        if max_search_projection_bytes == 0 {
            return Err(StorageError::InvalidStorageLimit(
                "max_search_projection_bytes",
            ));
        }
        if max_search_query_bytes == 0 {
            return Err(StorageError::InvalidStorageLimit("max_search_query_bytes"));
        }
        if max_search_terms == 0 {
            return Err(StorageError::InvalidStorageLimit("max_search_terms"));
        }
        if reconciliation_entry_limit == 0 {
            return Err(StorageError::InvalidStorageLimit(
                "reconciliation_entry_limit",
            ));
        }
        Ok(Self {
            max_payload_bytes,
            max_search_projection_bytes,
            max_search_query_bytes,
            max_search_terms,
            reconciliation_entry_limit,
        })
    }

    #[must_use]
    pub const fn max_payload_bytes(self) -> u64 {
        self.max_payload_bytes
    }

    #[must_use]
    pub const fn max_search_projection_bytes(self) -> usize {
        self.max_search_projection_bytes
    }

    #[must_use]
    pub const fn max_search_query_bytes(self) -> usize {
        self.max_search_query_bytes
    }

    #[must_use]
    pub const fn max_search_terms(self) -> usize {
        self.max_search_terms
    }

    #[must_use]
    pub const fn reconciliation_entry_limit(self) -> usize {
        self.reconciliation_entry_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_limits_and_policy_versions_are_rejected() {
        assert!(StoragePolicyVersion::new(0).is_err());
        assert!(StorageLimits::new(0, 1, 1, 1, 1).is_err());
        assert!(StorageLimits::new(1, 0, 1, 1, 1).is_err());
        assert!(StorageLimits::new(1, 1, 0, 1, 1).is_err());
        assert!(StorageLimits::new(1, 1, 1, 0, 1).is_err());
        assert!(StorageLimits::new(1, 1, 1, 1, 0).is_err());
    }
}
