use std::path::Path;

use pastral_storage::Storage;

use crate::{
    AgentIdentity, AgentRuntimeError, DiagnosticStoragePolicy, PrivacyPolicyConfig,
    diagnostic_storage_limits,
};

pub struct AgentHealthSnapshot {
    storage_schema_version: u32,
    capture_enabled: bool,
    privacy_policy_ok: bool,
    storage_integrity_ok: bool,
}

impl AgentHealthSnapshot {
    #[must_use]
    pub const fn storage_schema_version(&self) -> u32 {
        self.storage_schema_version
    }

    #[must_use]
    pub const fn capture_enabled(&self) -> bool {
        self.capture_enabled
    }

    #[must_use]
    pub const fn privacy_policy_ok(&self) -> bool {
        self.privacy_policy_ok
    }

    #[must_use]
    pub const fn storage_integrity_ok(&self) -> bool {
        self.storage_integrity_ok
    }
}

pub fn load_health_snapshot(data_root: &Path) -> Result<AgentHealthSnapshot, AgentRuntimeError> {
    let _identity = AgentIdentity::load_or_create(data_root)?;
    let _privacy_policy = PrivacyPolicyConfig::load_or_create(data_root)?;
    let storage = open_storage(data_root)?;
    let runtime = storage
        .runtime_info()
        .map_err(|_| AgentRuntimeError::Storage("runtime-info"))?;
    let integrity = storage
        .integrity_check()
        .map_err(|_| AgentRuntimeError::Storage("integrity-check"))?;
    let storage_integrity_ok = integrity.sqlite_ok
        && integrity.fts_ok
        && integrity.metadata_ok
        && integrity.search_mapping_ok;
    if !storage_integrity_ok {
        return Err(AgentRuntimeError::IntegrityFailed);
    }

    let storage_schema_version = u32::try_from(runtime.schema_version)
        .map_err(|_| AgentRuntimeError::Storage("runtime-schema-version"))?;

    Ok(AgentHealthSnapshot {
        storage_schema_version,
        capture_enabled: false,
        privacy_policy_ok: true,
        storage_integrity_ok,
    })
}

pub(crate) fn open_storage(
    data_root: &Path,
) -> Result<Storage<DiagnosticStoragePolicy>, AgentRuntimeError> {
    Storage::open(
        data_root.join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .map_err(|_| AgentRuntimeError::Storage("open"))
}
