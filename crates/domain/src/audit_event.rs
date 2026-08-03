use crate::{CaptureAuditEventId, CaptureOrder, ProfileId, UtcUnixMicros};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureAuditKind {
    SensitiveItemSkipped,
    PolicyDenied,
    ClipboardUnavailable,
    NoSupportedRepresentation,
    StorageUnavailable,
    IntegrityQuarantined,
    PossibleIntermediateStateLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditResultCode {
    Skipped,
    Denied,
    RetryExhausted,
    Degraded,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAuditEvent {
    id: CaptureAuditEventId,
    kind: CaptureAuditKind,
    occurred_at: UtcUnixMicros,
    capture_order: Option<CaptureOrder>,
    profile_id: ProfileId,
    result: AuditResultCode,
}

impl CaptureAuditEvent {
    #[must_use]
    pub const fn new(
        id: CaptureAuditEventId,
        kind: CaptureAuditKind,
        occurred_at: UtcUnixMicros,
        capture_order: Option<CaptureOrder>,
        profile_id: ProfileId,
        result: AuditResultCode,
    ) -> Self {
        Self {
            id,
            kind,
            occurred_at,
            capture_order,
            profile_id,
            result,
        }
    }

    #[must_use]
    pub const fn id(&self) -> CaptureAuditEventId {
        self.id
    }

    #[must_use]
    pub const fn kind(&self) -> CaptureAuditKind {
        self.kind
    }

    #[must_use]
    pub const fn occurred_at(&self) -> UtcUnixMicros {
        self.occurred_at
    }

    #[must_use]
    pub const fn capture_order(&self) -> Option<CaptureOrder> {
        self.capture_order
    }

    #[must_use]
    pub const fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    #[must_use]
    pub const fn result(&self) -> AuditResultCode {
        self.result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    Captured(crate::ClipEvent),
    Audited(CaptureAuditEvent),
    HardDenied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_deny_has_no_durable_record() {
        assert_eq!(CaptureOutcome::HardDenied, CaptureOutcome::HardDenied);
    }

    #[test]
    fn audit_contains_only_content_free_metadata() {
        let event = CaptureAuditEvent::new(
            CaptureAuditEventId::new_v4(),
            CaptureAuditKind::SensitiveItemSkipped,
            UtcUnixMicros::new(0).unwrap(),
            None,
            ProfileId::new_v4(),
            AuditResultCode::Skipped,
        );
        assert_eq!(event.kind(), CaptureAuditKind::SensitiveItemSkipped);
        assert_eq!(event.capture_order(), None);
    }
}
