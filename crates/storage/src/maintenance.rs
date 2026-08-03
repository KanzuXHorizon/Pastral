use std::{
    fs,
    path::{Path, PathBuf},
};

use pastral_domain::{BlobObjectId, ClipEventId};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::{
    BlobPlacementPolicy, Storage, StorageError,
    blob::resolve_external_key,
    encoding::{BACKEND_EXTERNAL, BACKEND_INTERNAL, BLOB_PENDING_DELETE, BLOB_READY},
    ids::typed_id_from_blob,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteReceipt {
    event_found: bool,
    internal_blobs_removed: usize,
    external_blobs_removed: usize,
    external_blobs_pending: usize,
}

impl DeleteReceipt {
    #[must_use]
    pub const fn event_found(self) -> bool {
        self.event_found
    }

    #[must_use]
    pub const fn internal_blobs_removed(self) -> usize {
        self.internal_blobs_removed
    }

    #[must_use]
    pub const fn external_blobs_removed(self) -> usize {
        self.external_blobs_removed
    }

    #[must_use]
    pub const fn external_blobs_pending(self) -> usize {
        self.external_blobs_pending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconciliationReport {
    pub staging_removed: usize,
    pub staging_retained: usize,
    pub pending_deletes_completed: usize,
    pub pending_deletes_retained: usize,
    pub unreferenced_objects_removed: usize,
    pub unreferenced_objects_retained: usize,
    pub missing_external_objects: usize,
    pub unsafe_external_locators: usize,
    pub reference_count_mismatches: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrityReport {
    pub sqlite_ok: bool,
    pub fts_ok: bool,
    pub metadata_ok: bool,
    pub search_mapping_ok: bool,
}

type PendingExternalRow = (BlobObjectId, Option<String>);

struct BoundedFiles {
    files: Vec<PathBuf>,
    entries_scanned: usize,
    truncated: bool,
}

struct BlobReference {
    id: BlobObjectId,
    backend_kind: i64,
    external_key: Option<String>,
    references_in_event: i64,
}

impl<P: BlobPlacementPolicy> Storage<P> {
    pub fn delete_clip(&mut self, id: ClipEventId) -> Result<DeleteReceipt, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let event_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM clip_events WHERE clip_event_id = ?1)",
            [id.as_uuid().as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        if !event_exists {
            transaction.commit()?;
            return Ok(DeleteReceipt {
                event_found: false,
                internal_blobs_removed: 0,
                external_blobs_removed: 0,
                external_blobs_pending: 0,
            });
        }

        let search_row_id: Option<i64> = transaction
            .query_row(
                "SELECT search_row_id FROM search_documents WHERE clip_event_id = ?1",
                [id.as_uuid().as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(search_row_id) = search_row_id {
            transaction.execute(
                "DELETE FROM clip_search_fts WHERE rowid = ?1",
                [search_row_id],
            )?;
            transaction.execute(
                "DELETE FROM search_documents WHERE search_row_id = ?1",
                [search_row_id],
            )?;
        }

        let references = {
            let mut statement = transaction.prepare(
                "SELECT b.blob_object_id, b.backend_kind, b.external_key, count(*)
                 FROM clip_representations r
                 JOIN blob_objects b ON b.blob_object_id = r.blob_object_id
                 WHERE r.clip_event_id = ?1
                 GROUP BY b.blob_object_id, b.backend_kind, b.external_key",
            )?;
            let rows = statement.query_map([id.as_uuid().as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            let mut values = Vec::new();
            for row in rows {
                let row = row?;
                values.push(BlobReference {
                    id: typed_id_from_blob(&row.0, BlobObjectId::from_uuid)?,
                    backend_kind: row.1,
                    external_key: row.2,
                    references_in_event: row.3,
                });
            }
            values
        };

        transaction.execute(
            "DELETE FROM clip_events WHERE clip_event_id = ?1",
            [id.as_uuid().as_bytes().as_slice()],
        )?;

        let mut internal_removed = 0usize;
        let mut pending_external = Vec::new();
        for reference in &references {
            transaction.execute(
                "UPDATE blob_objects
                 SET reference_count = reference_count - ?2
                 WHERE blob_object_id = ?1",
                params![
                    reference.id.as_uuid().as_bytes().as_slice(),
                    reference.references_in_event,
                ],
            )?;
            let remaining: i64 = transaction.query_row(
                "SELECT reference_count FROM blob_objects WHERE blob_object_id = ?1",
                [reference.id.as_uuid().as_bytes().as_slice()],
                |row| row.get(0),
            )?;
            if remaining == 0 {
                match reference.backend_kind {
                    BACKEND_INTERNAL => {
                        transaction.execute(
                            "DELETE FROM blob_objects WHERE blob_object_id = ?1",
                            [reference.id.as_uuid().as_bytes().as_slice()],
                        )?;
                        internal_removed += 1;
                    }
                    BACKEND_EXTERNAL => {
                        transaction.execute(
                            "UPDATE blob_objects SET lifecycle_state = ?2
                             WHERE blob_object_id = ?1",
                            params![
                                reference.id.as_uuid().as_bytes().as_slice(),
                                BLOB_PENDING_DELETE,
                            ],
                        )?;
                        pending_external.push((reference.id, reference.external_key.clone()));
                    }
                    _ => return Err(StorageError::Domain("unknown blob backend tag".into())),
                }
            }
        }
        transaction.commit()?;

        let mut external_removed = 0usize;
        let mut external_pending = 0usize;
        for (blob_id, key) in pending_external {
            match self.complete_external_delete(blob_id, key.as_deref()) {
                Ok(true) => external_removed += 1,
                Ok(false) => external_pending += 1,
                Err(_) => external_pending += 1,
            }
        }
        Ok(DeleteReceipt {
            event_found: true,
            internal_blobs_removed: internal_removed,
            external_blobs_removed: external_removed,
            external_blobs_pending: external_pending,
        })
    }

    pub fn reconcile(&mut self) -> Result<ReconciliationReport, StorageError> {
        let limit = self.limits.reconciliation_entry_limit();
        let mut report = ReconciliationReport::default();
        let mut processed = 0usize;

        let staging =
            immediate_files(&self.root.join(".staging"), limit.saturating_sub(processed))?;
        processed += staging.entries_scanned;
        for path in staging.files {
            match fs::remove_file(&path) {
                Ok(()) => report.staging_removed += 1,
                Err(_) => report.staging_retained += 1,
            }
        }
        if staging.truncated {
            report.truncated = true;
            return Ok(report);
        }

        let (pending, pending_truncated) =
            self.pending_external_rows(limit.saturating_sub(processed))?;
        processed += pending.len();
        for (id, key) in pending {
            match self.complete_external_delete(id, key.as_deref()) {
                Ok(true) => report.pending_deletes_completed += 1,
                Ok(false) => report.pending_deletes_retained += 1,
                Err(StorageError::UnsafeExternalLocator) => {
                    report.pending_deletes_retained += 1;
                    report.unsafe_external_locators += 1;
                }
                Err(error) => return Err(error),
            }
        }
        if pending_truncated {
            report.truncated = true;
            return Ok(report);
        }

        let objects = recursive_files(&self.root.join("objects"), limit.saturating_sub(processed))?;
        processed += objects.entries_scanned;
        for path in objects.files {
            let key = match relative_key(&self.root, &path) {
                Some(key) => key,
                None => {
                    report.unsafe_external_locators += 1;
                    continue;
                }
            };
            let referenced: bool = self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM blob_objects WHERE external_key = ?1)",
                [key.as_str()],
                |row| row.get(0),
            )?;
            if !referenced {
                match fs::remove_file(&path) {
                    Ok(()) => report.unreferenced_objects_removed += 1,
                    Err(_) => report.unreferenced_objects_retained += 1,
                }
            }
        }
        if objects.truncated {
            report.truncated = true;
            return Ok(report);
        }

        let (ready_external_keys, ready_truncated) =
            self.ready_external_keys(limit.saturating_sub(processed))?;
        for key in ready_external_keys {
            match resolve_external_key(&self.root, &key) {
                Ok(path) if !path.is_file() => report.missing_external_objects += 1,
                Ok(_) => {}
                Err(_) => report.unsafe_external_locators += 1,
            }
        }
        if ready_truncated {
            report.truncated = true;
            return Ok(report);
        }

        report.reference_count_mismatches = usize::try_from(self.connection.query_row(
            "SELECT count(*) FROM blob_objects b
             WHERE b.reference_count != (
                 SELECT count(*) FROM clip_representations r
                 WHERE r.blob_object_id = b.blob_object_id
             )",
            [],
            |row| row.get::<_, i64>(0),
        )?)
        .map_err(|_| StorageError::IntegerOutOfRange("reference mismatch count"))?;
        Ok(report)
    }

    pub fn integrity_check(&self) -> Result<IntegrityReport, StorageError> {
        let sqlite_result: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        let sqlite_ok = sqlite_result.eq_ignore_ascii_case("ok");
        self.connection.execute(
            "INSERT INTO clip_search_fts(clip_search_fts) VALUES('integrity-check')",
            [],
        )?;

        let metadata_mismatches: i64 = self.connection.query_row(
            "SELECT count(*)
             FROM clip_representations r
             JOIN clip_events e ON e.clip_event_id = r.clip_event_id
             JOIN blob_objects b ON b.blob_object_id = r.blob_object_id
             WHERE r.protection_kind != e.protection_kind
                OR r.protection_domain_id != e.protection_domain_id
                OR r.protection_kind != b.protection_kind
                OR r.protection_domain_id != b.protection_domain_id
                OR r.raw_length != b.raw_length
                OR r.digest_suite IS NOT b.digest_suite
                OR r.digest IS NOT b.digest
                OR b.lifecycle_state != ?1
                OR b.reference_count != (
                    SELECT count(*) FROM clip_representations counted
                    WHERE counted.blob_object_id = b.blob_object_id
                )",
            [BLOB_READY],
            |row| row.get(0),
        )?;

        let search_mapping_mismatches: i64 = self.connection.query_row(
            "SELECT
                (SELECT count(*) FROM search_documents d
                 LEFT JOIN clip_search_fts f ON f.rowid = d.search_row_id
                 WHERE f.rowid IS NULL)
              + (SELECT count(*) FROM clip_search_fts f
                 LEFT JOIN search_documents d ON d.search_row_id = f.rowid
                 WHERE d.search_row_id IS NULL)",
            [],
            |row| row.get(0),
        )?;

        Ok(IntegrityReport {
            sqlite_ok,
            fts_ok: true,
            metadata_ok: metadata_mismatches == 0,
            search_mapping_ok: search_mapping_mismatches == 0,
        })
    }

    fn pending_external_rows(
        &self,
        limit: usize,
    ) -> Result<(Vec<PendingExternalRow>, bool), StorageError> {
        let query_limit = i64::try_from(limit.saturating_add(1))
            .map_err(|_| StorageError::IntegerOutOfRange("reconciliation limit"))?;
        let mut statement = self.connection.prepare(
            "SELECT blob_object_id, external_key FROM blob_objects
             WHERE backend_kind = ?1 AND lifecycle_state = ?2
             ORDER BY rowid
             LIMIT ?3",
        )?;
        let rows = statement.query_map(
            params![BACKEND_EXTERNAL, BLOB_PENDING_DELETE, query_limit],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<String>>(1)?)),
        )?;
        let mut values = Vec::new();
        for row in rows {
            let row = row?;
            values.push((typed_id_from_blob(&row.0, BlobObjectId::from_uuid)?, row.1));
        }
        let truncated = values.len() > limit;
        values.truncate(limit);
        Ok((values, truncated))
    }

    fn ready_external_keys(&self, limit: usize) -> Result<(Vec<String>, bool), StorageError> {
        let query_limit = i64::try_from(limit.saturating_add(1))
            .map_err(|_| StorageError::IntegerOutOfRange("reconciliation limit"))?;
        let mut statement = self.connection.prepare(
            "SELECT external_key FROM blob_objects
             WHERE backend_kind = ?1 AND lifecycle_state = ?2
             ORDER BY rowid
             LIMIT ?3",
        )?;
        let rows = statement
            .query_map(params![BACKEND_EXTERNAL, BLOB_READY, query_limit], |row| {
                row.get::<_, String>(0)
            })?;
        let mut values = rows.collect::<Result<Vec<_>, _>>()?;
        let truncated = values.len() > limit;
        values.truncate(limit);
        Ok((values, truncated))
    }

    fn complete_external_delete(
        &mut self,
        blob_id: BlobObjectId,
        key: Option<&str>,
    ) -> Result<bool, StorageError> {
        let key = key.ok_or(StorageError::UnsafeExternalLocator)?;
        let path = resolve_external_key(&self.root, key)?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Ok(false),
        }
        self.connection.execute(
            "DELETE FROM blob_objects
             WHERE blob_object_id = ?1 AND lifecycle_state = ?2 AND reference_count = 0",
            params![blob_id.as_uuid().as_bytes().as_slice(), BLOB_PENDING_DELETE],
        )?;
        Ok(true)
    }
}

fn immediate_files(root: &Path, limit: usize) -> Result<BoundedFiles, StorageError> {
    if !root.exists() {
        return Ok(BoundedFiles {
            files: Vec::new(),
            entries_scanned: 0,
            truncated: false,
        });
    }
    let mut files = Vec::new();
    let mut entries_scanned = 0usize;
    let mut truncated = false;
    for entry in
        fs::read_dir(root).map_err(|error| StorageError::io("enumerate staging root", &error))?
    {
        if entries_scanned >= limit {
            truncated = true;
            break;
        }
        entries_scanned += 1;
        let entry = entry.map_err(|error| StorageError::io("read staging entry", &error))?;
        if entry
            .file_type()
            .map_err(|error| StorageError::io("inspect staging entry", &error))?
            .is_file()
        {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(BoundedFiles {
        files,
        entries_scanned,
        truncated,
    })
}

fn recursive_files(root: &Path, limit: usize) -> Result<BoundedFiles, StorageError> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    let mut entries_scanned = 0usize;
    let mut truncated = false;
    'directories: while let Some(directory) = directories.pop() {
        if !directory.exists() {
            continue;
        }
        for entry in fs::read_dir(&directory)
            .map_err(|error| StorageError::io("enumerate object directory", &error))?
        {
            if entries_scanned >= limit {
                truncated = true;
                break 'directories;
            }
            entries_scanned += 1;
            let entry = entry.map_err(|error| StorageError::io("read object entry", &error))?;
            let file_type = entry
                .file_type()
                .map_err(|error| StorageError::io("inspect object entry", &error))?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(BoundedFiles {
        files,
        entries_scanned,
        truncated,
    })
}

fn relative_key(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(value) => parts.push(value.to_str()?.to_owned()),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use pastral_domain::{ProtectionDomain, ProtectionDomainId};

    use super::*;
    use crate::{
        BlobPlacement, StorageLimits, StoragePolicyVersion,
        repository::tests::{FixedPolicy, clip, limits},
        test_support::TestRoot,
    };

    #[test]
    fn deleting_shared_internal_blob_preserves_then_removes_it() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        let first = clip(1, domain, b"same");
        let second = clip(2, domain, b"same");
        let first_id = first.event().id();
        let second_id = second.event().id();
        storage.commit_clip(first).unwrap();
        storage.commit_clip(second).unwrap();

        let first_receipt = storage.delete_clip(first_id).unwrap();
        assert!(first_receipt.event_found());
        assert_eq!(first_receipt.internal_blobs_removed(), 0);
        let blob_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM blob_objects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 1);

        let second_receipt = storage.delete_clip(second_id).unwrap();
        assert_eq!(second_receipt.internal_blobs_removed(), 1);
        let blob_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM blob_objects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 0);
    }

    #[test]
    fn external_delete_and_reconciliation_remove_unreferenced_files() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::ExternalFile),
        )
        .unwrap();
        let commit = clip(1, domain, b"external");
        let event_id = commit.event().id();
        storage.commit_clip(commit).unwrap();
        let receipt = storage.delete_clip(event_id).unwrap();
        assert_eq!(receipt.external_blobs_removed(), 1);

        let orphan = root.path().join("objects/ordinary/orphan/test.blob");
        fs::create_dir_all(orphan.parent().unwrap()).unwrap();
        File::create(&orphan).unwrap().write_all(b"orphan").unwrap();
        let staging = root.path().join(".staging/leftover.tmp");
        File::create(&staging)
            .unwrap()
            .write_all(b"staging")
            .unwrap();
        let report = storage.reconcile().unwrap();
        assert_eq!(report.staging_removed, 1);
        assert_eq!(report.unreferenced_objects_removed, 1);
        assert!(!orphan.exists());
        assert!(!staging.exists());
    }

    #[test]
    fn external_delete_failure_is_retained_and_retried() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::ExternalFile),
        )
        .unwrap();
        let commit = clip(1, domain, b"external");
        let event_id = commit.event().id();
        storage.commit_clip(commit).unwrap();
        let key: String = storage
            .connection
            .query_row("SELECT external_key FROM blob_objects", [], |row| {
                row.get(0)
            })
            .unwrap();
        let path = resolve_external_key(root.path(), &key).unwrap();
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let receipt = storage.delete_clip(event_id).unwrap();
        assert_eq!(receipt.external_blobs_removed(), 0);
        assert_eq!(receipt.external_blobs_pending(), 1);
        fs::remove_dir(&path).unwrap();

        let report = storage.reconcile().unwrap();
        assert_eq!(report.pending_deletes_completed, 1);
        assert_eq!(report.pending_deletes_retained, 0);
        let blob_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM blob_objects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 0);
    }

    #[test]
    fn missing_external_file_is_reported_without_deleting_metadata() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::ExternalFile),
        )
        .unwrap();
        storage.commit_clip(clip(1, domain, b"external")).unwrap();
        let key: String = storage
            .connection
            .query_row("SELECT external_key FROM blob_objects", [], |row| {
                row.get(0)
            })
            .unwrap();
        let path = resolve_external_key(root.path(), &key).unwrap();
        fs::remove_file(path).unwrap();
        let report = storage.reconcile().unwrap();
        assert_eq!(report.missing_external_objects, 1);
        let event_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM clip_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn sqlite_and_fts_integrity_checks_pass_after_operations() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        storage.commit_clip(clip(1, domain, b"data")).unwrap();
        let report = storage.integrity_check().unwrap();
        assert!(report.sqlite_ok);
        assert!(report.fts_ok);
        assert!(report.metadata_ok);
        assert!(report.search_mapping_ok);
    }

    #[test]
    fn integrity_check_reports_metadata_and_search_mapping_corruption() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        storage.commit_clip(clip(1, domain, b"data")).unwrap();

        let wrong_domain = ProtectionDomainId::new_v4();
        storage
            .connection
            .execute(
                "UPDATE clip_representations SET protection_domain_id = ?1",
                [wrong_domain.as_uuid().as_bytes().as_slice()],
            )
            .unwrap();
        storage
            .connection
            .execute(
                "INSERT INTO clip_search_fts(rowid, body) VALUES (999999, 'orphan')",
                [],
            )
            .unwrap();

        let report = storage.integrity_check().unwrap();
        assert!(report.sqlite_ok);
        assert!(report.fts_ok);
        assert!(!report.metadata_ok);
        assert!(!report.search_mapping_ok);
    }

    #[test]
    fn reconciliation_entry_limit_bounds_staging_and_directory_traversal() {
        let root = TestRoot::new();
        let bounded_limits = StorageLimits::new(1024, 1024, 256, 16, 2).unwrap();
        let mut storage = Storage::open(
            root.path(),
            bounded_limits,
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();

        for name in ["one.tmp", "two.tmp", "three.tmp"] {
            File::create(root.path().join(".staging").join(name)).unwrap();
        }
        let staging_report = storage.reconcile().unwrap();
        assert!(staging_report.truncated);
        assert_eq!(staging_report.staging_removed, 2);
        assert_eq!(
            fs::read_dir(root.path().join(".staging")).unwrap().count(),
            1
        );

        for entry in fs::read_dir(root.path().join(".staging")).unwrap() {
            fs::remove_file(entry.unwrap().path()).unwrap();
        }
        fs::create_dir_all(root.path().join("objects/a/b/c")).unwrap();
        let directory_report = storage.reconcile().unwrap();
        assert!(directory_report.truncated);
        assert_eq!(directory_report.unreferenced_objects_removed, 0);
    }

    #[test]
    fn policy_type_remains_explicit_in_maintenance_tests() {
        assert_eq!(StoragePolicyVersion::new(1).unwrap().get(), 1);
    }
}
