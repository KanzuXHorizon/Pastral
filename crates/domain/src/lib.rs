#![forbid(unsafe_code)]

mod audit_event;
mod capture_order;
mod clip_event;
mod derived;
mod digest;
mod error;
mod fidelity;
mod format_identity;
mod id;
mod protection;
mod representation;
mod time;

pub use audit_event::{AuditResultCode, CaptureAuditEvent, CaptureAuditKind, CaptureOutcome};
pub use capture_order::CaptureOrder;
pub use clip_event::ClipEvent;
pub use derived::DerivedRepresentationProvenance;
pub use digest::{DigestSuite, RawDigest};
pub use error::DomainError;
pub use fidelity::{Fidelity, FidelityAggregationVersion, aggregate_fidelity_v1};
pub use format_identity::{ClipboardFormatIdentity, RegisteredFormatName, StandardFormatId};
pub use id::{
    BlobObjectId, CaptureAuditEventId, ClipEventId, ClipRepresentationId, DerivedRepresentationId,
    ProfileId, ProtectionDomainId, TransformationId,
};
pub use protection::ProtectionDomain;
pub use representation::ClipRepresentation;
pub use time::UtcUnixMicros;
