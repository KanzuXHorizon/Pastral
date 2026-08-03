use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use crate::StorageError;

pub(crate) const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageRuntimeInfo {
    pub sqlite_version: String,
    pub schema_version: i64,
    pub journal_mode: String,
    pub synchronous: i64,
    pub foreign_keys: bool,
    pub fts5_enabled: bool,
}

pub(crate) fn configure_and_migrate(connection: &mut Connection) -> Result<(), StorageError> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = FULL;")?;

    let journal_mode: String =
        connection.query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("delete") {
        return Err(StorageError::RequiredPragmaMismatch("journal_mode"));
    }

    let foreign_keys: i64 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys != 1 {
        return Err(StorageError::RequiredPragmaMismatch("foreign_keys"));
    }

    let synchronous: i64 = connection.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
    if synchronous != 2 {
        return Err(StorageError::RequiredPragmaMismatch("synchronous"));
    }

    let fts5_enabled = fts5_enabled(connection)?;
    if !fts5_enabled {
        return Err(StorageError::Fts5Unavailable);
    }

    let current_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current_version > SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchemaVersion(current_version));
    }
    if current_version == 0 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(SCHEMA_V1)?;
        transaction.execute_batch("PRAGMA user_version = 1;")?;
        transaction.commit()?;
    }
    Ok(())
}

pub(crate) fn runtime_info(connection: &Connection) -> Result<StorageRuntimeInfo, StorageError> {
    Ok(StorageRuntimeInfo {
        sqlite_version: connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))?,
        schema_version: connection.query_row("PRAGMA user_version", [], |row| row.get(0))?,
        journal_mode: connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?,
        synchronous: connection.query_row("PRAGMA synchronous", [], |row| row.get(0))?,
        foreign_keys: connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))?
            == 1,
        fts5_enabled: fts5_enabled(connection)?,
    })
}

fn fts5_enabled(connection: &Connection) -> Result<bool, StorageError> {
    let value = connection
        .query_row(
            "SELECT 1 FROM pragma_compile_options WHERE compile_options = 'ENABLE_FTS5' LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    Ok(value == Some(1))
}

const SCHEMA_V1: &str = r#"
CREATE TABLE clip_events (
    clip_event_id BLOB PRIMARY KEY CHECK(length(clip_event_id) = 16),
    observed_at_utc_us INTEGER NOT NULL,
    capture_order INTEGER NOT NULL UNIQUE CHECK(capture_order > 0),
    captured_profile_id BLOB NOT NULL CHECK(length(captured_profile_id) = 16),
    protection_kind INTEGER NOT NULL CHECK(protection_kind IN (1, 2, 3)),
    protection_domain_id BLOB NOT NULL CHECK(length(protection_domain_id) = 16),
    aggregate_fidelity INTEGER NOT NULL CHECK(aggregate_fidelity BETWEEN 1 AND 6),
    fidelity_version INTEGER NOT NULL CHECK(fidelity_version = 1)
) STRICT;

CREATE TABLE blob_objects (
    blob_object_id BLOB PRIMARY KEY CHECK(length(blob_object_id) = 16),
    protection_kind INTEGER NOT NULL CHECK(protection_kind IN (1, 2, 3)),
    protection_domain_id BLOB NOT NULL CHECK(length(protection_domain_id) = 16),
    digest_suite INTEGER,
    digest BLOB,
    raw_length INTEGER NOT NULL CHECK(raw_length >= 0),
    backend_kind INTEGER NOT NULL CHECK(backend_kind IN (1, 2)),
    internal_payload BLOB,
    external_key TEXT,
    storage_policy_version INTEGER NOT NULL CHECK(storage_policy_version > 0),
    reference_count INTEGER NOT NULL CHECK(reference_count >= 0),
    lifecycle_state INTEGER NOT NULL CHECK(lifecycle_state IN (1, 2)),
    CHECK((digest_suite IS NULL AND digest IS NULL) OR
          (digest_suite = 1 AND digest IS NOT NULL AND length(digest) = 32)),
    CHECK((backend_kind = 1 AND internal_payload IS NOT NULL AND external_key IS NULL) OR
          (backend_kind = 2 AND internal_payload IS NULL AND external_key IS NOT NULL))
) STRICT;

CREATE UNIQUE INDEX blob_objects_ordinary_dedup
ON blob_objects(protection_domain_id, digest_suite, digest, raw_length)
WHERE digest IS NOT NULL AND lifecycle_state = 1;

CREATE INDEX blob_objects_pending_delete
ON blob_objects(lifecycle_state, backend_kind);

CREATE TABLE clip_representations (
    clip_representation_id BLOB PRIMARY KEY CHECK(length(clip_representation_id) = 16),
    clip_event_id BLOB NOT NULL REFERENCES clip_events(clip_event_id) ON DELETE CASCADE,
    format_kind INTEGER NOT NULL CHECK(format_kind IN (1, 2)),
    standard_format_id INTEGER,
    registered_format_name TEXT,
    protection_kind INTEGER NOT NULL CHECK(protection_kind IN (1, 2, 3)),
    protection_domain_id BLOB NOT NULL CHECK(length(protection_domain_id) = 16),
    raw_length INTEGER NOT NULL CHECK(raw_length >= 0),
    digest_suite INTEGER,
    digest BLOB,
    fidelity INTEGER NOT NULL CHECK(fidelity BETWEEN 1 AND 6),
    blob_object_id BLOB NOT NULL REFERENCES blob_objects(blob_object_id),
    CHECK((format_kind = 1 AND standard_format_id IS NOT NULL AND registered_format_name IS NULL) OR
          (format_kind = 2 AND standard_format_id IS NULL AND registered_format_name IS NOT NULL AND length(registered_format_name) > 0)),
    CHECK((digest_suite IS NULL AND digest IS NULL) OR
          (digest_suite = 1 AND digest IS NOT NULL AND length(digest) = 32))
) STRICT;

CREATE INDEX clip_representations_event
ON clip_representations(clip_event_id);

CREATE INDEX clip_representations_blob
ON clip_representations(blob_object_id);

CREATE TABLE search_documents (
    search_row_id INTEGER PRIMARY KEY,
    clip_event_id BLOB NOT NULL UNIQUE REFERENCES clip_events(clip_event_id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK(length(body) > 0)
) STRICT;

CREATE VIRTUAL TABLE clip_search_fts USING fts5(
    body,
    content = '',
    contentless_delete = 1,
    tokenize = 'unicode61'
);

CREATE TABLE capture_audit_events (
    capture_audit_event_id BLOB PRIMARY KEY CHECK(length(capture_audit_event_id) = 16),
    audit_kind INTEGER NOT NULL CHECK(audit_kind BETWEEN 1 AND 7),
    occurred_at_utc_us INTEGER NOT NULL,
    capture_order INTEGER CHECK(capture_order IS NULL OR capture_order > 0),
    profile_id BLOB NOT NULL CHECK(length(profile_id) = 16),
    result_code INTEGER NOT NULL CHECK(result_code BETWEEN 1 AND 5)
) STRICT;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encoding::{
            BACKEND_EXTERNAL, BACKEND_INTERNAL, BLOB_PENDING_DELETE, BLOB_READY,
            PROTECTION_ORDINARY,
        },
        test_support::open_test_connection,
    };

    #[test]
    fn schema_creation_and_reopen_preserve_version_and_pragmas() {
        let (_root, mut connection) = open_test_connection();
        configure_and_migrate(&mut connection).unwrap();
        let first = runtime_info(&connection).unwrap();
        assert_eq!(first.sqlite_version, "3.53.2");
        eprintln!("SQLite runtime: {}", first.sqlite_version);
        assert_eq!(first.schema_version, SCHEMA_VERSION);
        assert_eq!(first.journal_mode.to_ascii_lowercase(), "delete");
        assert_eq!(first.synchronous, 2);
        assert!(first.foreign_keys);
        assert!(first.fts5_enabled);
        configure_and_migrate(&mut connection).unwrap();
        assert_eq!(runtime_info(&connection).unwrap().schema_version, 1);
    }

    #[test]
    fn newer_schema_is_rejected() {
        let (_root, mut connection) = open_test_connection();
        connection
            .execute_batch("PRAGMA user_version = 99;")
            .unwrap();
        assert!(matches!(
            configure_and_migrate(&mut connection),
            Err(StorageError::UnsupportedSchemaVersion(99))
        ));
    }

    #[test]
    fn schema_tags_match_constants() {
        assert_eq!(PROTECTION_ORDINARY, 1);
        assert_eq!(BACKEND_INTERNAL, 1);
        assert_eq!(BACKEND_EXTERNAL, 2);
        assert_eq!(BLOB_READY, 1);
        assert_eq!(BLOB_PENDING_DELETE, 2);
    }
}
