use pastral_agent_core::{
    CaptureSink, CaptureSinkError, CaptureSinkOutcome, StoredCapture, TextCaptureRequest,
    detect_high_confidence_secret,
};
use pastral_domain::{
    AuditResultCode, CaptureAuditEvent, CaptureAuditEventId, CaptureAuditKind, ClipEventId,
    ClipRepresentation, ClipRepresentationId, ClipboardFormatIdentity, Fidelity, RawDigest,
    StandardFormatId,
};
use pastral_storage::{
    BlobPlacement, BlobPlacementContext, BlobPlacementPolicy, NewClipCommit, RepresentationPayload,
    SearchProjection, Storage, StorageLimits, StoragePolicyVersion,
};

const CF_UNICODETEXT_ID: u32 = 13;
const MAX_UNICODE_TEXT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SEARCH_PROJECTION_BYTES: usize = 1024 * 1024;
const MAX_SEARCH_QUERY_BYTES: usize = 4 * 1024;
const MAX_SEARCH_TERMS: usize = 64;
const RECONCILIATION_ENTRY_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticStoragePolicy;

impl BlobPlacementPolicy for DiagnosticStoragePolicy {
    fn version(&self) -> StoragePolicyVersion {
        StoragePolicyVersion::new(1).expect("diagnostic storage policy version is nonzero")
    }

    fn select(&self, _context: &BlobPlacementContext) -> BlobPlacement {
        BlobPlacement::InternalSqlite
    }
}

#[must_use]
pub fn diagnostic_storage_limits() -> StorageLimits {
    StorageLimits::new(
        MAX_UNICODE_TEXT_BYTES,
        MAX_SEARCH_PROJECTION_BYTES,
        MAX_SEARCH_QUERY_BYTES,
        MAX_SEARCH_TERMS,
        RECONCILIATION_ENTRY_LIMIT,
    )
    .expect("diagnostic storage limits are positive")
}

pub struct StorageCaptureSink<P: BlobPlacementPolicy> {
    storage: Storage<P>,
}

impl<P: BlobPlacementPolicy> StorageCaptureSink<P> {
    #[must_use]
    pub const fn new(storage: Storage<P>) -> Self {
        Self { storage }
    }

    #[must_use]
    pub const fn storage(&self) -> &Storage<P> {
        &self.storage
    }

    #[must_use]
    pub fn storage_mut(&mut self) -> &mut Storage<P> {
        &mut self.storage
    }

    #[must_use]
    pub fn into_storage(self) -> Storage<P> {
        self.storage
    }
}

impl<P: BlobPlacementPolicy> CaptureSink for StorageCaptureSink<P> {
    fn store_text(
        &mut self,
        request: TextCaptureRequest,
    ) -> Result<CaptureSinkOutcome, CaptureSinkError> {
        if detect_high_confidence_secret(request.captured_text().text()).is_some() {
            let audit = CaptureAuditEvent::new(
                CaptureAuditEventId::new_v4(),
                CaptureAuditKind::SensitiveItemSkipped,
                request.observed_at(),
                None,
                request.profile_id(),
                AuditResultCode::Skipped,
            );
            self.storage
                .insert_audit_event(audit)
                .map_err(|_| CaptureSinkError::StorageFailure)?;
            return Ok(CaptureSinkOutcome::SensitiveSkipped);
        }

        let event_id = ClipEventId::new_v4();
        let representation_id = ClipRepresentationId::new_v4();
        let protection_domain = request.protection_domain();
        let bytes = request.captured_text().raw_utf16le().to_vec();
        let raw_length =
            u64::try_from(bytes.len()).map_err(|_| CaptureSinkError::StorageFailure)?;
        let digest = RawDigest::sha256_raw_v1(protection_domain, &bytes)
            .map_err(|_| CaptureSinkError::StorageFailure)?;
        let representation = ClipRepresentation::new(
            representation_id,
            ClipboardFormatIdentity::Standard(StandardFormatId::new(CF_UNICODETEXT_ID)),
            protection_domain,
            raw_length,
            Some(digest),
            Fidelity::FullFidelity,
        )
        .map_err(|_| CaptureSinkError::StorageFailure)?;
        let payload = RepresentationPayload::new(representation_id, bytes);
        let search_projection = if request
            .captured_text()
            .text()
            .chars()
            .any(|character| !character.is_whitespace())
        {
            Some(
                SearchProjection::new(
                    request.captured_text().text().to_owned(),
                    self.storage.limits(),
                )
                .map_err(|_| CaptureSinkError::StorageFailure)?,
            )
        } else {
            None
        };
        let commit = NewClipCommit::new(
            event_id,
            request.observed_at(),
            request.profile_id(),
            protection_domain,
            vec![representation],
            vec![payload],
            search_projection,
        );
        let receipt = self
            .storage
            .commit_new_clip(commit)
            .map_err(|_| CaptureSinkError::StorageFailure)?;
        Ok(CaptureSinkOutcome::Stored(StoredCapture::new(
            receipt.clip_event_id(),
            receipt.capture_order(),
        )))
    }
}
