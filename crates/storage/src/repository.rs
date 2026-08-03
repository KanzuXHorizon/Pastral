use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
};

use pastral_domain::{
    BlobObjectId, CaptureAuditEvent, CaptureOrder, ClipEvent, ClipEventId, ClipRepresentation,
    ClipRepresentationId, RawDigest, UtcUnixMicros,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::{
    BlobPlacement, BlobPlacementContext, BlobPlacementPolicy, ClipCommit, StorageError,
    StorageLimits, StorageRuntimeInfo,
    blob::{
        BlobPlacementSummary, PreparedExternal, find_ordinary_blob, increment_reference,
        insert_new_blob, placement_summary, prepare_external, read_blob,
    },
    commit::{ValidatedRepresentation, validate_commit},
    encoding::{
        FIDELITY_VERSION_V1, decode_digest_suite, decode_fidelity, decode_format,
        decode_protection, encode_audit_kind, encode_audit_result, encode_digest_suite,
        encode_fidelity, encode_format, encode_protection,
    },
    ids::typed_id_from_blob,
    schema::{configure_and_migrate, runtime_info},
    search::insert_projection,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    clip_event_id: ClipEventId,
    blob_objects_created: usize,
    blob_objects_reused: usize,
}

impl CommitReceipt {
    #[must_use]
    pub const fn clip_event_id(&self) -> ClipEventId {
        self.clip_event_id
    }

    #[must_use]
    pub const fn blob_objects_created(&self) -> usize {
        self.blob_objects_created
    }

    #[must_use]
    pub const fn blob_objects_reused(&self) -> usize {
        self.blob_objects_reused
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRepresentation {
    representation_id: ClipRepresentationId,
    blob_object_id: BlobObjectId,
    placement: BlobPlacementSummary,
}

impl StoredRepresentation {
    #[must_use]
    pub const fn representation_id(&self) -> ClipRepresentationId {
        self.representation_id
    }

    #[must_use]
    pub const fn blob_object_id(&self) -> BlobObjectId {
        self.blob_object_id
    }

    #[must_use]
    pub const fn placement(&self) -> BlobPlacementSummary {
        self.placement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredClip {
    event: ClipEvent,
    representations: Vec<StoredRepresentation>,
}

impl StoredClip {
    #[must_use]
    pub const fn event(&self) -> &ClipEvent {
        &self.event
    }

    #[must_use]
    pub fn representations(&self) -> &[StoredRepresentation] {
        &self.representations
    }
}

struct PendingRepresentation<'a> {
    validated: &'a ValidatedRepresentation<'a>,
    placement: BlobPlacement,
    external: Option<PreparedExternal>,
}

pub struct Storage<P: BlobPlacementPolicy> {
    pub(crate) root: PathBuf,
    pub(crate) connection: Connection,
    pub(crate) limits: StorageLimits,
    pub(crate) policy: P,
    _single_owner: PhantomData<Rc<()>>,
}

impl<P: BlobPlacementPolicy> Storage<P> {
    pub fn open(
        root: impl AsRef<Path>,
        limits: StorageLimits,
        policy: P,
    ) -> Result<Self, StorageError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|error| StorageError::io("create data root", &error))?;
        fs::create_dir_all(root.join(".staging"))
            .map_err(|error| StorageError::io("create staging root", &error))?;
        fs::create_dir_all(root.join("objects"))
            .map_err(|error| StorageError::io("create object root", &error))?;

        let mut connection = Connection::open(root.join("metadata.sqlite3"))?;
        configure_and_migrate(&mut connection)?;

        Ok(Self {
            root,
            connection,
            limits,
            policy,
            _single_owner: PhantomData,
        })
    }

    pub fn runtime_info(&self) -> Result<StorageRuntimeInfo, StorageError> {
        runtime_info(&self.connection)
    }

    #[must_use]
    pub const fn limits(&self) -> StorageLimits {
        self.limits
    }

    #[must_use]
    pub const fn policy(&self) -> &P {
        &self.policy
    }

    pub fn commit_clip(&mut self, commit: ClipCommit) -> Result<CommitReceipt, StorageError> {
        let validated = validate_commit(&commit, self.limits)?;
        let mut pending = Vec::with_capacity(validated.representations.len());
        for representation in &validated.representations {
            let context = BlobPlacementContext::new(
                representation.representation.raw_logical_length(),
                representation.representation.protection_domain(),
                representation.representation.format().clone(),
            );
            let placement = self.policy.select(&context);
            let external = if placement == BlobPlacement::ExternalFile {
                Some(prepare_external(
                    &self.root,
                    representation.representation.protection_domain(),
                    &representation.digest,
                    representation.bytes,
                )?)
            } else {
                None
            };
            pending.push(PendingRepresentation {
                validated: representation,
                placement,
                external,
            });
        }

        let policy_version = self.policy.version();
        let outcome = (|| -> Result<CommitReceipt, StorageError> {
            let transaction = self
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            reject_duplicate_keys(&transaction, validated.event)?;

            let (protection_kind, protection_domain_id) =
                encode_protection(validated.event.captured_protection_domain());
            let capture_order = i64::try_from(validated.event.capture_order().get())
                .map_err(|_| StorageError::IntegerOutOfRange("capture order"))?;
            transaction.execute(
                "INSERT INTO clip_events (
                    clip_event_id, observed_at_utc_us, capture_order, captured_profile_id,
                    protection_kind, protection_domain_id, aggregate_fidelity, fidelity_version
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    validated.event.id().as_uuid().as_bytes().as_slice(),
                    validated.event.observed_at().get(),
                    capture_order,
                    validated
                        .event
                        .captured_profile_id()
                        .as_uuid()
                        .as_bytes()
                        .as_slice(),
                    protection_kind,
                    protection_domain_id.as_slice(),
                    encode_fidelity(validated.aggregate_fidelity),
                    FIDELITY_VERSION_V1,
                ],
            )?;

            let mut created = 0usize;
            let mut reused = 0usize;
            for item in &mut pending {
                let representation = item.validated.representation;
                let blob = if let Some(existing) = find_ordinary_blob(
                    &transaction,
                    representation.protection_domain(),
                    &item.validated.digest,
                    representation.raw_logical_length(),
                )? {
                    increment_reference(&transaction, existing.id)?;
                    if let Some(external) = &item.external {
                        external.cleanup_after_failure();
                    }
                    reused += 1;
                    existing
                } else {
                    if let Some(external) = &mut item.external {
                        external.finalize(
                            representation.protection_domain(),
                            &item.validated.digest,
                            representation.raw_logical_length(),
                        )?;
                    }
                    let blob = insert_new_blob(
                        &transaction,
                        item.placement,
                        policy_version,
                        representation.protection_domain(),
                        &item.validated.digest,
                        item.validated.bytes,
                        item.external.as_ref(),
                    )?;
                    created += 1;
                    blob
                };

                let encoded_format = encode_format(representation.format());
                let (representation_protection_kind, representation_domain_id) =
                    encode_protection(representation.protection_domain());
                let raw_length = i64::try_from(representation.raw_logical_length())
                    .map_err(|_| StorageError::IntegerOutOfRange("representation raw length"))?;
                transaction.execute(
                    "INSERT INTO clip_representations (
                        clip_representation_id, clip_event_id, format_kind,
                        standard_format_id, registered_format_name,
                        protection_kind, protection_domain_id, raw_length,
                        digest_suite, digest, fidelity, blob_object_id
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        representation.id().as_uuid().as_bytes().as_slice(),
                        validated.event.id().as_uuid().as_bytes().as_slice(),
                        encoded_format.kind,
                        encoded_format.standard_id,
                        encoded_format.registered_name,
                        representation_protection_kind,
                        representation_domain_id.as_slice(),
                        raw_length,
                        encode_digest_suite(item.validated.digest.suite()),
                        item.validated.digest.bytes().as_slice(),
                        encode_fidelity(representation.fidelity()),
                        blob.id.as_uuid().as_bytes().as_slice(),
                    ],
                )?;
            }

            if let Some(projection) = validated.search_projection {
                insert_projection(&transaction, validated.event.id(), projection.as_str())?;
            }

            transaction.commit()?;
            Ok(CommitReceipt {
                clip_event_id: validated.event.id(),
                blob_objects_created: created,
                blob_objects_reused: reused,
            })
        })();

        if outcome.is_err() {
            for item in &pending {
                if let Some(external) = &item.external {
                    external.cleanup_after_failure();
                }
            }
        }
        outcome
    }

    pub fn load_clip(&self, id: ClipEventId) -> Result<Option<StoredClip>, StorageError> {
        let event_row = self
            .connection
            .query_row(
                "SELECT observed_at_utc_us, capture_order, captured_profile_id,
                        protection_kind, protection_domain_id
                 FROM clip_events WHERE clip_event_id = ?1",
                [id.as_uuid().as_bytes().as_slice()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some(event_row) = event_row else {
            return Ok(None);
        };

        let profile_id = typed_id_from_blob(&event_row.2, pastral_domain::ProfileId::from_uuid)?;
        let protection_domain = decode_protection(event_row.3, &event_row.4)?;
        let observed_at = UtcUnixMicros::new(event_row.0)
            .map_err(|error| StorageError::Domain(error.to_string()))?;
        let capture_order = CaptureOrder::new(
            u64::try_from(event_row.1)
                .map_err(|_| StorageError::IntegerOutOfRange("capture order"))?,
        )
        .map_err(|error| StorageError::Domain(error.to_string()))?;

        let mut statement = self.connection.prepare(
            "SELECT r.clip_representation_id, r.format_kind, r.standard_format_id,
                    r.registered_format_name, r.protection_kind, r.protection_domain_id,
                    r.raw_length, r.digest_suite, r.digest, r.fidelity,
                    r.blob_object_id, b.backend_kind
             FROM clip_representations r
             JOIN blob_objects b ON b.blob_object_id = r.blob_object_id
             WHERE r.clip_event_id = ?1
             ORDER BY r.rowid",
        )?;
        let rows = statement.query_map([id.as_uuid().as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<Vec<u8>>>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, Vec<u8>>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })?;

        let mut domain_representations = Vec::new();
        let mut stored_representations = Vec::new();
        for row in rows {
            let row = row?;
            let representation_id = typed_id_from_blob(&row.0, ClipRepresentationId::from_uuid)?;
            let format = decode_format(row.1, row.2, row.3)?;
            let representation_domain = decode_protection(row.4, &row.5)?;
            let raw_length = u64::try_from(row.6)
                .map_err(|_| StorageError::IntegerOutOfRange("representation raw length"))?;
            let digest = match (row.7, row.8) {
                (Some(suite), Some(bytes)) => {
                    decode_digest_suite(suite)?;
                    let bytes: [u8; 32] = bytes
                        .try_into()
                        .map_err(|_| StorageError::Domain("invalid digest length".into()))?;
                    Some(
                        RawDigest::from_sha256_raw_v1_bytes(representation_domain, bytes)
                            .map_err(|error| StorageError::Domain(error.to_string()))?,
                    )
                }
                (None, None) => None,
                _ => return Err(StorageError::Domain("invalid digest columns".into())),
            };
            let fidelity = decode_fidelity(row.9)?;
            let representation = ClipRepresentation::new(
                representation_id,
                format,
                representation_domain,
                raw_length,
                digest,
                fidelity,
            )
            .map_err(|error| StorageError::Domain(error.to_string()))?;
            let blob_object_id = typed_id_from_blob(&row.10, BlobObjectId::from_uuid)?;
            stored_representations.push(StoredRepresentation {
                representation_id,
                blob_object_id,
                placement: placement_summary(row.11)?,
            });
            domain_representations.push(representation);
        }

        let event = ClipEvent::new(
            id,
            observed_at,
            capture_order,
            profile_id,
            protection_domain,
            domain_representations,
        )
        .map_err(|error| StorageError::Domain(error.to_string()))?;
        Ok(Some(StoredClip {
            event,
            representations: stored_representations,
        }))
    }

    pub fn read_representation(
        &self,
        id: ClipRepresentationId,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT r.protection_kind, r.protection_domain_id, r.digest,
                        r.raw_length, r.blob_object_id
                 FROM clip_representations r
                 WHERE r.clip_representation_id = ?1",
                [id.as_uuid().as_bytes().as_slice()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<Vec<u8>>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some(row) = row else {
            return Ok(None);
        };
        let domain = decode_protection(row.0, &row.1)?;
        let digest_bytes: [u8; 32] = row
            .2
            .ok_or(StorageError::BlobIntegrityMismatch)?
            .try_into()
            .map_err(|_| StorageError::BlobIntegrityMismatch)?;
        let digest = RawDigest::from_sha256_raw_v1_bytes(domain, digest_bytes)
            .map_err(|error| StorageError::Domain(error.to_string()))?;
        let raw_length = u64::try_from(row.3)
            .map_err(|_| StorageError::IntegerOutOfRange("representation raw length"))?;
        let blob_id = typed_id_from_blob(&row.4, BlobObjectId::from_uuid)?;
        read_blob(
            &self.connection,
            &self.root,
            blob_id,
            domain,
            &digest,
            raw_length,
        )
        .map(Some)
    }

    pub fn insert_audit_event(&mut self, event: CaptureAuditEvent) -> Result<(), StorageError> {
        let capture_order = event
            .capture_order()
            .map(|value| {
                i64::try_from(value.get())
                    .map_err(|_| StorageError::IntegerOutOfRange("audit capture order"))
            })
            .transpose()?;
        self.connection.execute(
            "INSERT INTO capture_audit_events (
                capture_audit_event_id, audit_kind, occurred_at_utc_us,
                capture_order, profile_id, result_code
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id().as_uuid().as_bytes().as_slice(),
                encode_audit_kind(event.kind()),
                event.occurred_at().get(),
                capture_order,
                event.profile_id().as_uuid().as_bytes().as_slice(),
                encode_audit_result(event.result()),
            ],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

fn reject_duplicate_keys(
    transaction: &rusqlite::Transaction<'_>,
    event: &ClipEvent,
) -> Result<(), StorageError> {
    let event_exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM clip_events WHERE clip_event_id = ?1)",
        [event.id().as_uuid().as_bytes().as_slice()],
        |row| row.get(0),
    )?;
    if event_exists {
        return Err(StorageError::DuplicateClipEvent(event.id()));
    }
    let capture_order = i64::try_from(event.capture_order().get())
        .map_err(|_| StorageError::IntegerOutOfRange("capture order"))?;
    let order_exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM clip_events WHERE capture_order = ?1)",
        [capture_order],
        |row| row.get(0),
    )?;
    if order_exists {
        return Err(StorageError::DuplicateCaptureOrder);
    }
    for representation in event.representations() {
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clip_representations WHERE clip_representation_id = ?1
             )",
            [representation.id().as_uuid().as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        if exists {
            return Err(StorageError::DuplicateRepresentation(representation.id()));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use pastral_domain::{
        AuditResultCode, CaptureAuditEventId, CaptureAuditKind, ClipboardFormatIdentity, Fidelity,
        ProfileId, ProtectionDomain, ProtectionDomainId, StandardFormatId,
    };

    use super::*;
    use crate::{
        RepresentationPayload, SearchProjection, StoragePolicyVersion, test_support::TestRoot,
    };

    #[derive(Debug, Clone, Copy)]
    pub(crate) struct FixedPolicy(pub(crate) BlobPlacement);

    impl BlobPlacementPolicy for FixedPolicy {
        fn version(&self) -> StoragePolicyVersion {
            StoragePolicyVersion::new(1).unwrap()
        }

        fn select(&self, _context: &BlobPlacementContext) -> BlobPlacement {
            self.0
        }
    }

    pub(crate) fn limits() -> StorageLimits {
        StorageLimits::new(1024 * 1024, 1024, 256, 16, 128).unwrap()
    }

    pub(crate) fn clip(order: u64, domain: ProtectionDomain, bytes: &[u8]) -> ClipCommit {
        let digest = RawDigest::sha256_raw_v1(domain, bytes).unwrap();
        let representation = ClipRepresentation::new(
            ClipRepresentationId::new_v4(),
            ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
            domain,
            bytes.len() as u64,
            Some(digest),
            Fidelity::FullFidelity,
        )
        .unwrap();
        let payload = RepresentationPayload::new(representation.id(), bytes.to_vec());
        let event = ClipEvent::new(
            ClipEventId::new_v4(),
            UtcUnixMicros::new(1_700_000_000_000_000 + order as i64).unwrap(),
            CaptureOrder::new(order).unwrap(),
            ProfileId::new_v4(),
            domain,
            vec![representation],
        )
        .unwrap();
        ClipCommit::new(event, vec![payload], None)
    }

    #[test]
    fn open_creates_schema_and_required_pragmas() {
        let root = TestRoot::new();
        let storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        let info = storage.runtime_info().unwrap();
        assert_eq!(info.schema_version, 1);
        assert_eq!(info.journal_mode.to_ascii_lowercase(), "delete");
        assert_eq!(info.synchronous, 2);
        assert!(info.foreign_keys);
        assert!(info.fts5_enabled);
        assert!(storage.root().join("objects").is_dir());
        assert!(storage.root().join(".staging").is_dir());
    }

    #[test]
    fn internal_and_external_payloads_round_trip_exact_bytes() {
        for placement in [BlobPlacement::InternalSqlite, BlobPlacement::ExternalFile] {
            let root = TestRoot::new();
            let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
            let commit = clip(1, domain, b"exact bytes");
            let event_id = commit.event().id();
            let representation_id = commit.event().representations()[0].id();
            let mut storage = Storage::open(root.path(), limits(), FixedPolicy(placement)).unwrap();
            let receipt = storage.commit_clip(commit).unwrap();
            assert_eq!(receipt.clip_event_id(), event_id);
            assert_eq!(
                storage.read_representation(representation_id).unwrap(),
                Some(b"exact bytes".to_vec())
            );
            let loaded = storage.load_clip(event_id).unwrap().unwrap();
            assert_eq!(loaded.event().id(), event_id);
            assert_eq!(
                loaded.representations()[0].placement(),
                match placement {
                    BlobPlacement::InternalSqlite => BlobPlacementSummary::InternalSqlite,
                    BlobPlacement::ExternalFile => BlobPlacementSummary::ExternalFile,
                }
            );
        }
    }

    #[test]
    fn empty_internal_payload_round_trips() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let commit = clip(1, domain, b"");
        let representation_id = commit.event().representations()[0].id();
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        storage.commit_clip(commit).unwrap();
        assert_eq!(
            storage.read_representation(representation_id).unwrap(),
            Some(Vec::new())
        );
    }

    #[test]
    fn same_domain_digest_reuses_blob_without_merging_events() {
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
        assert_eq!(
            storage.commit_clip(first).unwrap().blob_objects_created(),
            1
        );
        assert_eq!(
            storage.commit_clip(second).unwrap().blob_objects_reused(),
            1
        );
        assert_ne!(first_id, second_id);
        let blob_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM blob_objects", [], |row| row.get(0))
            .unwrap();
        let event_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM clip_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 1);
        assert_eq!(event_count, 2);
    }

    #[test]
    fn identical_bytes_in_different_domain_ids_do_not_deduplicate() {
        let root = TestRoot::new();
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        storage
            .commit_clip(clip(
                1,
                ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
                b"same",
            ))
            .unwrap();
        storage
            .commit_clip(clip(
                2,
                ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
                b"same",
            ))
            .unwrap();
        let blob_count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM blob_objects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(blob_count, 2);
    }

    #[test]
    fn invalid_payload_mapping_is_rejected_atomically() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let original = clip(1, domain, b"abc");
        let event = original.event().clone();
        let id = event.representations()[0].id();
        let cases = [
            ClipCommit::new(event.clone(), Vec::new(), None),
            ClipCommit::new(
                event.clone(),
                vec![
                    RepresentationPayload::new(id, b"abc".to_vec()),
                    RepresentationPayload::new(id, b"abc".to_vec()),
                ],
                None,
            ),
            ClipCommit::new(
                event.clone(),
                vec![RepresentationPayload::new(
                    ClipRepresentationId::new_v4(),
                    b"abc".to_vec(),
                )],
                None,
            ),
            ClipCommit::new(
                event,
                vec![RepresentationPayload::new(id, b"wrong".to_vec())],
                None,
            ),
        ];
        for case in cases {
            let mut storage = Storage::open(
                root.path(),
                limits(),
                FixedPolicy(BlobPlacement::InternalSqlite),
            )
            .unwrap();
            assert!(storage.commit_clip(case).is_err());
            let event_count: i64 = storage
                .connection
                .query_row("SELECT count(*) FROM clip_events", [], |row| row.get(0))
                .unwrap();
            assert_eq!(event_count, 0);
        }
    }

    #[test]
    fn protected_payloads_are_rejected_before_persistence() {
        for domain in [
            ProtectionDomain::Sensitive(ProtectionDomainId::new_v4()),
            ProtectionDomain::Private(ProtectionDomainId::new_v4()),
        ] {
            let root = TestRoot::new();
            let representation = ClipRepresentation::new(
                ClipRepresentationId::new_v4(),
                ClipboardFormatIdentity::Standard(StandardFormatId::new(13)),
                domain,
                6,
                None,
                Fidelity::FullFidelity,
            )
            .unwrap();
            let event = ClipEvent::new(
                ClipEventId::new_v4(),
                UtcUnixMicros::new(0).unwrap(),
                CaptureOrder::new(1).unwrap(),
                ProfileId::new_v4(),
                domain,
                vec![representation.clone()],
            )
            .unwrap();
            let commit = ClipCommit::new(
                event,
                vec![RepresentationPayload::new(
                    representation.id(),
                    b"secret".to_vec(),
                )],
                None,
            );
            let mut storage = Storage::open(
                root.path(),
                limits(),
                FixedPolicy(BlobPlacement::ExternalFile),
            )
            .unwrap();
            assert!(matches!(
                storage.commit_clip(commit),
                Err(StorageError::ProtectedPayloadRequiresEncryption(_))
            ));
            assert_eq!(
                fs::read_dir(root.path().join("objects")).unwrap().count(),
                0
            );
        }
    }

    #[test]
    fn explicit_search_projection_is_indexed_and_ordered() {
        let root = TestRoot::new();
        let domain = ProtectionDomain::Ordinary(ProtectionDomainId::new_v4());
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        let first = clip(1, domain, b"first");
        let first = ClipCommit::new(
            first.event().clone(),
            first.payloads().to_vec(),
            Some(SearchProjection::new("alpha OR beta", limits()).unwrap()),
        );
        let first_id = first.event().id();
        let second = clip(2, domain, b"second");
        let second = ClipCommit::new(
            second.event().clone(),
            second.payloads().to_vec(),
            Some(SearchProjection::new("alpha beta", limits()).unwrap()),
        );
        let second_id = second.event().id();
        let unindexed = clip(3, domain, b"alpha unindexed payload");
        storage.commit_clip(first).unwrap();
        storage.commit_clip(second).unwrap();
        storage.commit_clip(unindexed).unwrap();

        let hits = storage.search("alpha", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].clip_event_id(), second_id);
        assert_eq!(hits[1].clip_event_id(), first_id);
        assert_eq!(
            storage.search("OR", 10).unwrap()[0].clip_event_id(),
            first_id
        );
        assert!(storage.search("NOT", 10).unwrap().is_empty());
    }

    #[test]
    fn audit_schema_and_api_are_content_free() {
        let root = TestRoot::new();
        let mut storage = Storage::open(
            root.path(),
            limits(),
            FixedPolicy(BlobPlacement::InternalSqlite),
        )
        .unwrap();
        let event = CaptureAuditEvent::new(
            CaptureAuditEventId::new_v4(),
            CaptureAuditKind::SensitiveItemSkipped,
            UtcUnixMicros::new(0).unwrap(),
            None,
            ProfileId::new_v4(),
            AuditResultCode::Skipped,
        );
        storage.insert_audit_event(event).unwrap();
        let count: i64 = storage
            .connection
            .query_row("SELECT count(*) FROM capture_audit_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);

        let mut statement = storage
            .connection
            .prepare("PRAGMA table_info(capture_audit_events)")
            .unwrap();
        let columns: Vec<String> = statement
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for column in columns {
            let normalized = column.to_ascii_lowercase();
            for forbidden in [
                "payload", "preview", "digest", "blob", "snippet", "title", "url", "path",
            ] {
                assert!(
                    !normalized.contains(forbidden),
                    "forbidden audit column: {column}"
                );
            }
        }
    }

    #[test]
    fn search_projection_constructor_enforces_bounds() {
        assert!(SearchProjection::new("", limits()).is_err());
        assert!(SearchProjection::new("a\0b", limits()).is_err());
        assert!(SearchProjection::new("allowed", limits()).is_ok());
    }
}
