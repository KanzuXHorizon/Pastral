use pastral_domain::{CaptureOrder, ClipEventId, Fidelity, UtcUnixMicros};
use rusqlite::{Transaction, params};

use crate::{
    BlobPlacementPolicy, Storage, StorageError, encoding::decode_fidelity, ids::typed_id_from_blob,
};

const MAX_SEARCH_RESULTS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchHit {
    clip_event_id: ClipEventId,
    capture_order: CaptureOrder,
    observed_at: UtcUnixMicros,
    aggregate_fidelity: Fidelity,
}

impl SearchHit {
    #[must_use]
    pub const fn clip_event_id(self) -> ClipEventId {
        self.clip_event_id
    }

    #[must_use]
    pub const fn capture_order(self) -> CaptureOrder {
        self.capture_order
    }

    #[must_use]
    pub const fn observed_at(self) -> UtcUnixMicros {
        self.observed_at
    }

    #[must_use]
    pub const fn aggregate_fidelity(self) -> Fidelity {
        self.aggregate_fidelity
    }
}

pub(crate) fn insert_projection(
    transaction: &Transaction<'_>,
    clip_event_id: ClipEventId,
    text: &str,
) -> Result<(), StorageError> {
    transaction.execute(
        "INSERT INTO search_documents (clip_event_id, body) VALUES (?1, ?2)",
        params![clip_event_id.as_uuid().as_bytes().as_slice(), text],
    )?;
    let row_id = transaction.last_insert_rowid();
    transaction.execute(
        "INSERT INTO clip_search_fts (rowid, body) VALUES (?1, ?2)",
        params![row_id, text],
    )?;
    Ok(())
}

impl<P: BlobPlacementPolicy> Storage<P> {
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, StorageError> {
        if limit == 0 {
            return Err(StorageError::SearchQueryInvalid("limit must be positive"));
        }
        let expression = literal_query(
            query,
            self.limits.max_search_query_bytes(),
            self.limits.max_search_terms(),
        )?;
        let limit = i64::try_from(limit.min(MAX_SEARCH_RESULTS))
            .map_err(|_| StorageError::IntegerOutOfRange("search limit"))?;
        let mut statement = self.connection.prepare(
            "SELECT e.clip_event_id, e.capture_order, e.observed_at_utc_us,
                    e.aggregate_fidelity
             FROM clip_search_fts
             JOIN search_documents d ON d.search_row_id = clip_search_fts.rowid
             JOIN clip_events e ON e.clip_event_id = d.clip_event_id
             WHERE clip_search_fts MATCH ?1
             ORDER BY e.capture_order DESC, e.clip_event_id ASC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![expression, limit], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let row = row?;
            hits.push(SearchHit {
                clip_event_id: typed_id_from_blob(&row.0, ClipEventId::from_uuid)?,
                capture_order: CaptureOrder::new(
                    u64::try_from(row.1)
                        .map_err(|_| StorageError::IntegerOutOfRange("capture order"))?,
                )
                .map_err(|error| StorageError::Domain(error.to_string()))?,
                observed_at: UtcUnixMicros::new(row.2)
                    .map_err(|error| StorageError::Domain(error.to_string()))?,
                aggregate_fidelity: decode_fidelity(row.3)?,
            });
        }
        Ok(hits)
    }
}

fn literal_query(
    query: &str,
    max_query_bytes: usize,
    max_terms: usize,
) -> Result<String, StorageError> {
    if query.is_empty() || query.trim_matches(char::is_whitespace).is_empty() {
        return Err(StorageError::SearchQueryInvalid("empty"));
    }
    if query.contains('\0') {
        return Err(StorageError::SearchQueryInvalid("contains NUL"));
    }
    if query.len() > max_query_bytes {
        return Err(StorageError::SearchQueryInvalid("too large"));
    }
    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.len() > max_terms {
        return Err(StorageError::SearchQueryInvalid("too many terms"));
    }
    Ok(terms
        .into_iter()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_is_literal_and_bounded() {
        assert_eq!(
            literal_query("alpha OR beta", 100, 4).unwrap(),
            "\"alpha\" AND \"OR\" AND \"beta\""
        );
        assert_eq!(literal_query("a\"b", 100, 4).unwrap(), "\"a\"\"b\"");
        assert!(literal_query("", 100, 4).is_err());
        assert!(literal_query("a\0b", 100, 4).is_err());
        assert!(literal_query("one two", 100, 1).is_err());
    }
}
