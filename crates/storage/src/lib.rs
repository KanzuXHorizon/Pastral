#![forbid(unsafe_code)]

mod blob;
mod commit;
mod config;
mod encoding;
mod error;
mod ids;
mod maintenance;
mod repository;
mod schema;
mod search;

#[cfg(test)]
mod test_support;

pub use blob::BlobPlacementSummary;
pub use commit::{ClipCommit, RepresentationPayload, SearchProjection};
pub use config::{
    BlobPlacement, BlobPlacementContext, BlobPlacementPolicy, StorageLimits, StoragePolicyVersion,
};
pub use error::StorageError;
pub use maintenance::{DeleteReceipt, IntegrityReport, ReconciliationReport};
pub use repository::{CommitReceipt, Storage, StoredClip, StoredRepresentation};
pub use schema::StorageRuntimeInfo;
pub use search::SearchHit;
