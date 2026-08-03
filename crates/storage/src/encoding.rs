use pastral_domain::{
    AuditResultCode, CaptureAuditKind, ClipboardFormatIdentity, DigestSuite, Fidelity,
    ProtectionDomain, ProtectionDomainId, RegisteredFormatName, StandardFormatId,
};

use crate::{StorageError, ids::uuid_from_blob};

pub(crate) const PROTECTION_ORDINARY: i64 = 1;
pub(crate) const PROTECTION_SENSITIVE: i64 = 2;
pub(crate) const PROTECTION_PRIVATE: i64 = 3;
pub(crate) const FORMAT_STANDARD: i64 = 1;
pub(crate) const FORMAT_REGISTERED: i64 = 2;
pub(crate) const DIGEST_SHA256_RAW_V1: i64 = 1;
pub(crate) const FIDELITY_VERSION_V1: i64 = 1;
pub(crate) const BACKEND_INTERNAL: i64 = 1;
pub(crate) const BACKEND_EXTERNAL: i64 = 2;
pub(crate) const BLOB_READY: i64 = 1;
pub(crate) const BLOB_PENDING_DELETE: i64 = 2;

pub(crate) struct EncodedFormat<'a> {
    pub kind: i64,
    pub standard_id: Option<i64>,
    pub registered_name: Option<&'a str>,
}

pub(crate) fn encode_protection(value: ProtectionDomain) -> (i64, [u8; 16]) {
    let kind = match value {
        ProtectionDomain::Ordinary(_) => PROTECTION_ORDINARY,
        ProtectionDomain::Sensitive(_) => PROTECTION_SENSITIVE,
        ProtectionDomain::Private(_) => PROTECTION_PRIVATE,
    };
    (kind, *value.id().as_uuid().as_bytes())
}

pub(crate) fn decode_protection(kind: i64, id: &[u8]) -> Result<ProtectionDomain, StorageError> {
    let id = ProtectionDomainId::from_uuid(uuid_from_blob(id)?)
        .map_err(|error| StorageError::Domain(error.to_string()))?;
    match kind {
        PROTECTION_ORDINARY => Ok(ProtectionDomain::Ordinary(id)),
        PROTECTION_SENSITIVE => Ok(ProtectionDomain::Sensitive(id)),
        PROTECTION_PRIVATE => Ok(ProtectionDomain::Private(id)),
        _ => Err(StorageError::Domain("unknown protection-domain tag".into())),
    }
}

pub(crate) fn encode_format(value: &ClipboardFormatIdentity) -> EncodedFormat<'_> {
    match value {
        ClipboardFormatIdentity::Standard(id) => EncodedFormat {
            kind: FORMAT_STANDARD,
            standard_id: Some(i64::from(id.get())),
            registered_name: None,
        },
        ClipboardFormatIdentity::Registered(name) => EncodedFormat {
            kind: FORMAT_REGISTERED,
            standard_id: None,
            registered_name: Some(name.as_str()),
        },
    }
}

pub(crate) fn decode_format(
    kind: i64,
    standard_id: Option<i64>,
    registered_name: Option<String>,
) -> Result<ClipboardFormatIdentity, StorageError> {
    match kind {
        FORMAT_STANDARD => {
            let value = standard_id.ok_or_else(|| {
                StorageError::Domain("standard format is missing numeric ID".into())
            })?;
            let value = u32::try_from(value)
                .map_err(|_| StorageError::IntegerOutOfRange("standard format ID"))?;
            Ok(ClipboardFormatIdentity::Standard(StandardFormatId::new(
                value,
            )))
        }
        FORMAT_REGISTERED => {
            let value = registered_name.ok_or_else(|| {
                StorageError::Domain("registered format is missing stable name".into())
            })?;
            RegisteredFormatName::new(value)
                .map(ClipboardFormatIdentity::Registered)
                .map_err(|error| StorageError::Domain(error.to_string()))
        }
        _ => Err(StorageError::Domain("unknown clipboard-format tag".into())),
    }
}

pub(crate) const fn encode_fidelity(value: Fidelity) -> i64 {
    match value {
        Fidelity::FullFidelity => 1,
        Fidelity::CommonFormatsPreserved => 2,
        Fidelity::FallbackOnly => 3,
        Fidelity::ReferenceOnly => 4,
        Fidelity::Unavailable => 5,
        Fidelity::UnsafeOrUnsupported => 6,
    }
}

pub(crate) fn decode_fidelity(value: i64) -> Result<Fidelity, StorageError> {
    match value {
        1 => Ok(Fidelity::FullFidelity),
        2 => Ok(Fidelity::CommonFormatsPreserved),
        3 => Ok(Fidelity::FallbackOnly),
        4 => Ok(Fidelity::ReferenceOnly),
        5 => Ok(Fidelity::Unavailable),
        6 => Ok(Fidelity::UnsafeOrUnsupported),
        _ => Err(StorageError::Domain("unknown fidelity tag".into())),
    }
}

pub(crate) const fn encode_digest_suite(value: DigestSuite) -> i64 {
    match value {
        DigestSuite::Sha256RawV1 => DIGEST_SHA256_RAW_V1,
    }
}

pub(crate) fn decode_digest_suite(value: i64) -> Result<DigestSuite, StorageError> {
    match value {
        DIGEST_SHA256_RAW_V1 => Ok(DigestSuite::Sha256RawV1),
        _ => Err(StorageError::Domain("unknown digest-suite tag".into())),
    }
}

pub(crate) const fn encode_audit_kind(value: CaptureAuditKind) -> i64 {
    match value {
        CaptureAuditKind::SensitiveItemSkipped => 1,
        CaptureAuditKind::PolicyDenied => 2,
        CaptureAuditKind::ClipboardUnavailable => 3,
        CaptureAuditKind::NoSupportedRepresentation => 4,
        CaptureAuditKind::StorageUnavailable => 5,
        CaptureAuditKind::IntegrityQuarantined => 6,
        CaptureAuditKind::PossibleIntermediateStateLoss => 7,
    }
}

pub(crate) const fn encode_audit_result(value: AuditResultCode) -> i64 {
    match value {
        AuditResultCode::Skipped => 1,
        AuditResultCode::Denied => 2,
        AuditResultCode::RetryExhausted => 3,
        AuditResultCode::Degraded => 4,
        AuditResultCode::Quarantined => 5,
    }
}
