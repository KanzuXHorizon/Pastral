use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use rusqlite::params;

use crate::{
    BlobPlacementPolicy, Storage, StorageError, ids::typed_id_from_blob, search::literal_query,
};

const MAX_PAGE_LIMIT: usize = 100;
const MAX_PREVIEW_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipListItem {
    clip_event_id: ClipEventId,
    capture_order: CaptureOrder,
    observed_at: UtcUnixMicros,
    preview: Option<String>,
}

impl ClipListItem {
    #[must_use]
    pub const fn clip_event_id(&self) -> ClipEventId {
        self.clip_event_id
    }

    #[must_use]
    pub const fn capture_order(&self) -> CaptureOrder {
        self.capture_order
    }

    #[must_use]
    pub const fn observed_at(&self) -> UtcUnixMicros {
        self.observed_at
    }

    #[must_use]
    pub fn preview(&self) -> Option<&str> {
        self.preview.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipPage {
    items: Vec<ClipListItem>,
    has_more: bool,
}

impl ClipPage {
    #[must_use]
    pub fn items(&self) -> &[ClipListItem] {
        &self.items
    }

    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.has_more
    }
}

impl<P: BlobPlacementPolicy> Storage<P> {
    pub fn history_page(
        &self,
        before_capture_order: Option<CaptureOrder>,
        limit: usize,
    ) -> Result<ClipPage, StorageError> {
        validate_page_limit(limit)?;
        let before_capture_order = before_capture_order
            .map(|value| {
                i64::try_from(value.get())
                    .map_err(|_| StorageError::IntegerOutOfRange("capture order"))
            })
            .transpose()?;
        let query_limit = query_limit(limit)?;
        let mut statement = self.connection.prepare(
            "SELECT e.clip_event_id, e.capture_order, e.observed_at_utc_us,
                    substr(d.body, 1, 4096)
             FROM clip_events e
             LEFT JOIN search_documents d ON d.clip_event_id = e.clip_event_id
             WHERE (?1 IS NULL OR e.capture_order < ?1)
             ORDER BY e.capture_order DESC, e.clip_event_id ASC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![before_capture_order, query_limit], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        collect_page(rows, limit)
    }

    pub fn search_page(&self, query: &str, limit: usize) -> Result<ClipPage, StorageError> {
        validate_page_limit(limit)?;
        let expression = literal_query(
            query,
            self.limits.max_search_query_bytes(),
            self.limits.max_search_terms(),
        )?;
        let query_limit = query_limit(limit)?;
        let mut statement = self.connection.prepare(
            "SELECT e.clip_event_id, e.capture_order, e.observed_at_utc_us,
                    substr(d.body, 1, 4096)
             FROM clip_search_fts
             JOIN search_documents d ON d.search_row_id = clip_search_fts.rowid
             JOIN clip_events e ON e.clip_event_id = d.clip_event_id
             WHERE clip_search_fts MATCH ?1
             ORDER BY e.capture_order DESC, e.clip_event_id ASC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![expression, query_limit], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        collect_page(rows, limit)
    }
}

fn query_limit(limit: usize) -> Result<i64, StorageError> {
    let limit = limit
        .checked_add(1)
        .ok_or(StorageError::IntegerOutOfRange("read page limit"))?;
    i64::try_from(limit).map_err(|_| StorageError::IntegerOutOfRange("read page limit"))
}

fn collect_page(
    rows: impl Iterator<Item = rusqlite::Result<(Vec<u8>, i64, i64, Option<String>)>>,
    limit: usize,
) -> Result<ClipPage, StorageError> {
    let mut items = Vec::with_capacity(limit.saturating_add(1));
    for row in rows {
        let (event_id, capture_order, observed_at, preview) = row?;
        items.push(ClipListItem {
            clip_event_id: typed_id_from_blob(&event_id, ClipEventId::from_uuid)?,
            capture_order: CaptureOrder::new(
                u64::try_from(capture_order)
                    .map_err(|_| StorageError::IntegerOutOfRange("capture order"))?,
            )
            .map_err(|error| StorageError::Domain(error.to_string()))?,
            observed_at: UtcUnixMicros::new(observed_at)
                .map_err(|error| StorageError::Domain(error.to_string()))?,
            preview: preview.map(truncate_preview),
        });
    }
    let has_more = items.len() > limit;
    if has_more {
        items.truncate(limit);
    }
    Ok(ClipPage { items, has_more })
}

fn validate_page_limit(limit: usize) -> Result<(), StorageError> {
    if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
        return Err(StorageError::IntegerOutOfRange("read page limit"));
    }
    Ok(())
}

fn truncate_preview(value: String) -> String {
    if value.len() <= MAX_PREVIEW_BYTES {
        return value;
    }
    let mut end = MAX_PREVIEW_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use pastral_domain::{
        ClipEvent, ClipRepresentation, ClipRepresentationId, ClipboardFormatIdentity, Fidelity,
        ProfileId, ProtectionDomain, ProtectionDomainId, RawDigest, StandardFormatId,
    };

    use super::*;
    use crate::{
        BlobPlacement, ClipCommit, RepresentationPayload, SearchProjection, StorageLimits,
        repository::tests::FixedPolicy, test_support::TestRoot,
    };

    fn limits() -> StorageLimits {
        StorageLimits::new(1024 * 1024, 8192, 1024, 32, 128).unwrap()
    }

    fn commit(
        storage: &mut Storage<FixedPolicy>,
        domain: ProtectionDomain,
        order: u64,
        preview: Option<String>,
    ) -> ClipEventId {
        let bytes = format!("payload-{order}").into_bytes();
        let digest = RawDigest::sha256_raw_v1(domain, &bytes).unwrap();
        let representation = ClipRepresentation::new(
            ClipRepresentationId::new_v4(),
            ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
            domain,
            bytes.len() as u64,
            Some(digest),
            Fidelity::FullFidelity,
        )
        .unwrap();
        let event_id = ClipEventId::new_v4();
        let event = ClipEvent::new(
            event_id,
            UtcUnixMicros::new(1_700_000_000_000_000 + order as i64).unwrap(),
            CaptureOrder::new(order).unwrap(),
            ProfileId::new_v4(),
            domain,
            vec![representation.clone()],
        )
        .unwrap();
        let payload = RepresentationPayload::new(representation.id(), bytes);
        let projection = preview.map(|value| SearchProjection::new(value, limits()).unwrap());
        storage
            .commit_clip(ClipCommit::new(event, vec![payload], projection))
            .unwrap();
        event_id
    }

    fn storage(root: &TestRoot) -> Storage<FixedPolicy> {
        Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap()
    }

    #[test]
    fn history_is_cursor_bounded_and_reports_unavailable_rows() {
        let root = TestRoot::new();
        let mut storage = storage(&root);
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let first = commit(&mut storage, domain, 1, Some("alpha".to_owned()));
        let second = commit(&mut storage, domain, 2, Some("beta".to_owned()));
        let third = commit(&mut storage, domain, 3, None);

        let page = storage.history_page(None, 2).unwrap();
        assert_eq!(page.items().len(), 2);
        assert!(page.has_more());
        assert_eq!(page.items()[0].clip_event_id(), third);
        assert_eq!(page.items()[0].capture_order().get(), 3);
        assert_eq!(page.items()[0].preview(), None);
        assert_eq!(page.items()[1].clip_event_id(), second);
        assert_eq!(page.items()[1].preview(), Some("beta"));

        let next = storage
            .history_page(Some(CaptureOrder::new(2).unwrap()), 2)
            .unwrap();
        assert_eq!(next.items().len(), 1);
        assert!(!next.has_more());
        assert_eq!(next.items()[0].clip_event_id(), first);
    }

    #[test]
    fn search_is_literal_ordered_and_reports_more_results() {
        let root = TestRoot::new();
        let mut storage = storage(&root);
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let first = commit(&mut storage, domain, 1, Some("alpha beta".to_owned()));
        let second = commit(&mut storage, domain, 2, Some("alpha OR beta".to_owned()));
        commit(&mut storage, domain, 3, Some("gamma".to_owned()));

        let alpha = storage.search_page("alpha", 1).unwrap();
        assert_eq!(alpha.items().len(), 1);
        assert!(alpha.has_more());
        assert_eq!(alpha.items()[0].clip_event_id(), second);

        let literal = storage.search_page("alpha OR", 10).unwrap();
        assert_eq!(literal.items().len(), 1);
        assert!(!literal.has_more());
        assert_eq!(literal.items()[0].clip_event_id(), second);
        assert_ne!(literal.items()[0].clip_event_id(), first);
    }

    #[test]
    fn page_limits_and_utf8_preview_bytes_are_bounded() {
        let root = TestRoot::new();
        let mut storage = storage(&root);
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let long = "é".repeat(3000);
        commit(&mut storage, domain, 1, Some(long));

        assert!(storage.history_page(None, 0).is_err());
        assert!(storage.history_page(None, 101).is_err());
        assert!(storage.search_page("é", 0).is_err());
        assert!(storage.search_page("é", 101).is_err());

        let page = storage.history_page(None, 1).unwrap();
        let preview = page.items()[0].preview().unwrap();
        assert!(preview.len() <= MAX_PREVIEW_BYTES);
        assert!(preview.is_char_boundary(preview.len()));
        assert_eq!(preview.len(), MAX_PREVIEW_BYTES);
    }

    #[test]
    fn truncation_preserves_valid_utf8_boundary() {
        let preview = truncate_preview("é".repeat(3000));
        assert_eq!(preview.len(), MAX_PREVIEW_BYTES);
        assert!(preview.is_char_boundary(preview.len()));
    }
}
