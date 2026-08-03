use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use pastral_domain::{BlobObjectId, DigestSuite, ProtectionDomain, RawDigest};
use rusqlite::{Connection, MAIN_DB, OptionalExtension, Transaction, blob::ZeroBlob, params};

use crate::{
    BlobPlacement, StorageError, StoragePolicyVersion,
    encoding::{
        BACKEND_EXTERNAL, BACKEND_INTERNAL, BLOB_READY, DIGEST_SHA256_RAW_V1, encode_protection,
    },
    ids::{hex_lower, uuid_from_blob},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobPlacementSummary {
    InternalSqlite,
    ExternalFile,
}

#[derive(Debug, Clone)]
pub(crate) struct BlobRow {
    pub id: BlobObjectId,
}

pub(crate) struct PreparedExternal {
    staging_path: PathBuf,
    final_relative_key: String,
    final_path: PathBuf,
    finalized_by_this_commit: bool,
}

impl PreparedExternal {
    pub(crate) fn final_relative_key(&self) -> &str {
        &self.final_relative_key
    }

    pub(crate) fn finalize(
        &mut self,
        expected_domain: ProtectionDomain,
        expected_digest: &RawDigest,
        expected_length: u64,
    ) -> Result<(), StorageError> {
        if self.final_path.exists() {
            let bytes = read_file_bounded(&self.final_path, expected_length)?;
            verify_bytes(expected_domain, expected_digest, expected_length, &bytes)?;
            fs::remove_file(&self.staging_path)
                .map_err(|error| StorageError::io("remove redundant staging file", &error))?;
            return Ok(());
        }
        if let Some(parent) = self.final_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| StorageError::io("create external object directory", &error))?;
        }
        fs::rename(&self.staging_path, &self.final_path)
            .map_err(|error| StorageError::io("finalize external object", &error))?;
        self.finalized_by_this_commit = true;
        Ok(())
    }

    pub(crate) fn cleanup_after_failure(&self) {
        let _ = fs::remove_file(&self.staging_path);
        if self.finalized_by_this_commit {
            let _ = fs::remove_file(&self.final_path);
        }
    }
}

pub(crate) fn prepare_external(
    root: &Path,
    domain: ProtectionDomain,
    digest: &RawDigest,
    bytes: &[u8],
) -> Result<PreparedExternal, StorageError> {
    let staging_path = root
        .join(".staging")
        .join(format!("{}.tmp", BlobObjectId::new_v4()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staging_path)
        .map_err(|error| StorageError::io("create external staging file", &error))?;
    file.write_all(bytes)
        .map_err(|error| StorageError::io("write external staging file", &error))?;
    file.flush()
        .map_err(|error| StorageError::io("flush external staging file", &error))?;
    file.sync_all()
        .map_err(|error| StorageError::io("sync external staging file", &error))?;
    drop(file);

    let final_relative_key = external_relative_key(domain, digest);
    let final_path = resolve_external_key(root, &final_relative_key)?;
    Ok(PreparedExternal {
        staging_path,
        final_relative_key,
        final_path,
        finalized_by_this_commit: false,
    })
}

pub(crate) fn find_ordinary_blob(
    transaction: &Transaction<'_>,
    domain: ProtectionDomain,
    digest: &RawDigest,
    raw_length: u64,
) -> Result<Option<BlobRow>, StorageError> {
    let (_, domain_id) = encode_protection(domain);
    let raw_length = i64::try_from(raw_length)
        .map_err(|_| StorageError::IntegerOutOfRange("blob raw length"))?;
    transaction
        .query_row(
            "SELECT blob_object_id
             FROM blob_objects
             WHERE protection_domain_id = ?1
               AND digest_suite = ?2
               AND digest = ?3
               AND raw_length = ?4
               AND lifecycle_state = ?5",
            params![
                domain_id.as_slice(),
                DIGEST_SHA256_RAW_V1,
                digest.bytes().as_slice(),
                raw_length,
                BLOB_READY,
            ],
            |row| {
                let id: Vec<u8> = row.get(0)?;
                Ok(id)
            },
        )
        .optional()?
        .map(|id| {
            let id = BlobObjectId::from_uuid(uuid_from_blob(&id)?)
                .map_err(|error| StorageError::Domain(error.to_string()))?;
            Ok(BlobRow { id })
        })
        .transpose()
}

pub(crate) fn increment_reference(
    transaction: &Transaction<'_>,
    id: BlobObjectId,
) -> Result<(), StorageError> {
    transaction.execute(
        "UPDATE blob_objects SET reference_count = reference_count + 1 WHERE blob_object_id = ?1",
        [id.as_uuid().as_bytes().as_slice()],
    )?;
    Ok(())
}

pub(crate) fn insert_new_blob(
    transaction: &Transaction<'_>,
    placement: BlobPlacement,
    policy_version: StoragePolicyVersion,
    domain: ProtectionDomain,
    digest: &RawDigest,
    bytes: &[u8],
    prepared_external: Option<&PreparedExternal>,
) -> Result<BlobRow, StorageError> {
    let blob_id = BlobObjectId::new_v4();
    let (protection_kind, protection_domain_id) = encode_protection(domain);
    let raw_length = i64::try_from(bytes.len())
        .map_err(|_| StorageError::IntegerOutOfRange("blob raw length"))?;
    match placement {
        BlobPlacement::InternalSqlite => {
            if bytes.is_empty() {
                transaction.execute(
                    "INSERT INTO blob_objects (
                        blob_object_id, protection_kind, protection_domain_id,
                        digest_suite, digest, raw_length, backend_kind,
                        internal_payload, external_key, storage_policy_version,
                        reference_count, lifecycle_state
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 1, ?10)",
                    params![
                        blob_id.as_uuid().as_bytes().as_slice(),
                        protection_kind,
                        protection_domain_id.as_slice(),
                        DIGEST_SHA256_RAW_V1,
                        digest.bytes().as_slice(),
                        raw_length,
                        BACKEND_INTERNAL,
                        bytes,
                        i64::from(policy_version.get()),
                        BLOB_READY,
                    ],
                )?;
            } else {
                let zero_blob_length = i32::try_from(bytes.len())
                    .map_err(|_| StorageError::IntegerOutOfRange("incremental blob length"))?;
                transaction.execute(
                    "INSERT INTO blob_objects (
                        blob_object_id, protection_kind, protection_domain_id,
                        digest_suite, digest, raw_length, backend_kind,
                        internal_payload, external_key, storage_policy_version,
                        reference_count, lifecycle_state
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 1, ?10)",
                    params![
                        blob_id.as_uuid().as_bytes().as_slice(),
                        protection_kind,
                        protection_domain_id.as_slice(),
                        DIGEST_SHA256_RAW_V1,
                        digest.bytes().as_slice(),
                        raw_length,
                        BACKEND_INTERNAL,
                        ZeroBlob(zero_blob_length),
                        i64::from(policy_version.get()),
                        BLOB_READY,
                    ],
                )?;
                let row_id = transaction.last_insert_rowid();
                let mut blob = transaction.blob_open(
                    MAIN_DB,
                    "blob_objects",
                    "internal_payload",
                    row_id,
                    false,
                )?;
                blob.write_all_at(bytes, 0)?;
                blob.close()?;
            }
            Ok(BlobRow { id: blob_id })
        }
        BlobPlacement::ExternalFile => {
            let prepared = prepared_external.ok_or(StorageError::BlobMissing)?;
            transaction.execute(
                "INSERT INTO blob_objects (
                    blob_object_id, protection_kind, protection_domain_id,
                    digest_suite, digest, raw_length, backend_kind,
                    internal_payload, external_key, storage_policy_version,
                    reference_count, lifecycle_state
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, 1, ?10)",
                params![
                    blob_id.as_uuid().as_bytes().as_slice(),
                    protection_kind,
                    protection_domain_id.as_slice(),
                    DIGEST_SHA256_RAW_V1,
                    digest.bytes().as_slice(),
                    raw_length,
                    BACKEND_EXTERNAL,
                    prepared.final_relative_key(),
                    i64::from(policy_version.get()),
                    BLOB_READY,
                ],
            )?;
            Ok(BlobRow { id: blob_id })
        }
    }
}

pub(crate) fn read_blob(
    connection: &Connection,
    root: &Path,
    blob_id: BlobObjectId,
    domain: ProtectionDomain,
    digest: &RawDigest,
    expected_length: u64,
) -> Result<Vec<u8>, StorageError> {
    let row = connection
        .query_row(
            "SELECT backend_kind, internal_payload, external_key, raw_length
             FROM blob_objects WHERE blob_object_id = ?1 AND lifecycle_state = ?2",
            params![blob_id.as_uuid().as_bytes().as_slice(), BLOB_READY],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or(StorageError::BlobMissing)?;
    let stored_length =
        u64::try_from(row.3).map_err(|_| StorageError::IntegerOutOfRange("blob raw length"))?;
    if stored_length != expected_length {
        return Err(StorageError::BlobLengthMismatch);
    }
    let bytes = match row.0 {
        BACKEND_INTERNAL => row.1.ok_or(StorageError::BlobMissing)?,
        BACKEND_EXTERNAL => {
            let key = row.2.ok_or(StorageError::BlobMissing)?;
            let path = resolve_external_key(root, &key)?;
            read_file_bounded(&path, expected_length)?
        }
        _ => return Err(StorageError::Domain("unknown blob backend tag".into())),
    };
    verify_bytes(domain, digest, expected_length, &bytes)?;
    Ok(bytes)
}

pub(crate) fn placement_summary(backend_kind: i64) -> Result<BlobPlacementSummary, StorageError> {
    match backend_kind {
        BACKEND_INTERNAL => Ok(BlobPlacementSummary::InternalSqlite),
        BACKEND_EXTERNAL => Ok(BlobPlacementSummary::ExternalFile),
        _ => Err(StorageError::Domain("unknown blob backend tag".into())),
    }
}

pub(crate) fn resolve_external_key(root: &Path, key: &str) -> Result<PathBuf, StorageError> {
    if key.is_empty() || key.contains('\\') || key.starts_with('/') {
        return Err(StorageError::UnsafeExternalLocator);
    }
    let relative = Path::new(key);
    if relative.is_absolute() {
        return Err(StorageError::UnsafeExternalLocator);
    }
    let mut components = relative.components();
    match components.next() {
        Some(Component::Normal(value)) if value == "objects" => {}
        _ => return Err(StorageError::UnsafeExternalLocator),
    }
    if components.any(|component| !matches!(component, Component::Normal(_))) {
        return Err(StorageError::UnsafeExternalLocator);
    }
    Ok(root.join(relative))
}

pub(crate) fn external_relative_key(domain: ProtectionDomain, digest: &RawDigest) -> String {
    let suite = match digest.suite() {
        DigestSuite::Sha256RawV1 => "sha256-raw-v1",
    };
    format!(
        "objects/ordinary/{}/{}/{}.blob",
        domain.id(),
        suite,
        hex_lower(digest.bytes())
    )
}

fn read_file_bounded(path: &Path, expected_length: u64) -> Result<Vec<u8>, StorageError> {
    let metadata =
        fs::metadata(path).map_err(|error| StorageError::io("inspect external object", &error))?;
    if metadata.len() != expected_length {
        return Err(StorageError::BlobLengthMismatch);
    }
    let capacity = usize::try_from(expected_length)
        .map_err(|_| StorageError::IntegerOutOfRange("external object length"))?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .map_err(|error| StorageError::io("open external object", &error))?
        .read_to_end(&mut bytes)
        .map_err(|error| StorageError::io("read external object", &error))?;
    Ok(bytes)
}

fn verify_bytes(
    domain: ProtectionDomain,
    digest: &RawDigest,
    expected_length: u64,
    bytes: &[u8],
) -> Result<(), StorageError> {
    let actual_length =
        u64::try_from(bytes.len()).map_err(|_| StorageError::IntegerOutOfRange("blob length"))?;
    if actual_length != expected_length {
        return Err(StorageError::BlobLengthMismatch);
    }
    let actual = RawDigest::sha256_raw_v1(domain, bytes)
        .map_err(|error| StorageError::Domain(error.to_string()))?;
    if &actual != digest {
        return Err(StorageError::BlobIntegrityMismatch);
    }
    Ok(())
}
