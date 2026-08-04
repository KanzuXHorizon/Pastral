use std::collections::BTreeSet;

use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};

use crate::{CorrelationId, IpcError};

pub const NONCE_BYTES: usize = 32;
pub const MAX_PAGE_LIMIT: u32 = 100;
pub const MAX_QUERY_BYTES: usize = 1024;
pub const MAX_QUERY_TERMS: usize = 32;
pub const MAX_PREVIEW_BYTES: usize = 4096;
pub const MAX_SOURCE_LABEL_BYTES: usize = 256;
pub const MAX_ERROR_DETAIL_BYTES: usize = 512;
pub const MAX_PREVIEWS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    Health,
    HistoryPage,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClipPreviewKind {
    Text,
    Code,
    Link,
    Image,
    Files,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolErrorCode {
    InvalidRequest,
    UnsupportedVersion,
    UnsupportedCapability,
    Unauthorized,
    ResourceLimit,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHelloDto {
    protocol_major: u32,
    min_minor: u32,
    max_minor: u32,
    server_nonce: [u8; NONCE_BYTES],
    instance_id: CorrelationId,
    capabilities: Vec<Capability>,
}

impl ServerHelloDto {
    pub fn new(
        protocol_major: u32,
        min_minor: u32,
        max_minor: u32,
        server_nonce: [u8; NONCE_BYTES],
        instance_id: CorrelationId,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, IpcError> {
        validate_protocol(protocol_major, min_minor, max_minor)?;
        validate_nonce(&server_nonce)?;
        if instance_id.is_zero() {
            return Err(IpcError::InvalidDto("instance ID must be nonzero"));
        }
        let capabilities = validate_capabilities(capabilities)?;
        Ok(Self {
            protocol_major,
            min_minor,
            max_minor,
            server_nonce,
            instance_id,
            capabilities,
        })
    }

    #[must_use]
    pub const fn protocol_major(&self) -> u32 {
        self.protocol_major
    }

    #[must_use]
    pub const fn minor_range(&self) -> (u32, u32) {
        (self.min_minor, self.max_minor)
    }

    #[must_use]
    pub const fn server_nonce(&self) -> &[u8; NONCE_BYTES] {
        &self.server_nonce
    }

    #[must_use]
    pub const fn instance_id(&self) -> CorrelationId {
        self.instance_id
    }

    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientHelloDto {
    protocol_major: u32,
    min_minor: u32,
    max_minor: u32,
    client_nonce: [u8; NONCE_BYTES],
    echoed_server_nonce: [u8; NONCE_BYTES],
    capabilities: Vec<Capability>,
}

impl ClientHelloDto {
    pub fn new(
        protocol_major: u32,
        min_minor: u32,
        max_minor: u32,
        client_nonce: [u8; NONCE_BYTES],
        echoed_server_nonce: [u8; NONCE_BYTES],
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, IpcError> {
        validate_protocol(protocol_major, min_minor, max_minor)?;
        validate_nonce(&client_nonce)?;
        validate_nonce(&echoed_server_nonce)?;
        let capabilities = validate_capabilities(capabilities)?;
        Ok(Self {
            protocol_major,
            min_minor,
            max_minor,
            client_nonce,
            echoed_server_nonce,
            capabilities,
        })
    }

    #[must_use]
    pub const fn protocol_major(&self) -> u32 {
        self.protocol_major
    }

    #[must_use]
    pub const fn minor_range(&self) -> (u32, u32) {
        (self.min_minor, self.max_minor)
    }

    #[must_use]
    pub const fn client_nonce(&self) -> &[u8; NONCE_BYTES] {
        &self.client_nonce
    }

    #[must_use]
    pub const fn echoed_server_nonce(&self) -> &[u8; NONCE_BYTES] {
        &self.echoed_server_nonce
    }

    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HealthRequestDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthResponseDto {
    storage_schema_version: u32,
    capture_enabled: bool,
    privacy_policy_ok: bool,
    storage_integrity_ok: bool,
}

impl HealthResponseDto {
    pub const fn new(
        storage_schema_version: u32,
        capture_enabled: bool,
        privacy_policy_ok: bool,
        storage_integrity_ok: bool,
    ) -> Result<Self, IpcError> {
        if storage_schema_version == 0 {
            return Err(IpcError::InvalidDto(
                "storage schema version must be nonzero",
            ));
        }
        Ok(Self {
            storage_schema_version,
            capture_enabled,
            privacy_policy_ok,
            storage_integrity_ok,
        })
    }

    #[must_use]
    pub const fn storage_schema_version(self) -> u32 {
        self.storage_schema_version
    }

    #[must_use]
    pub const fn capture_enabled(self) -> bool {
        self.capture_enabled
    }

    #[must_use]
    pub const fn privacy_policy_ok(self) -> bool {
        self.privacy_policy_ok
    }

    #[must_use]
    pub const fn storage_integrity_ok(self) -> bool {
        self.storage_integrity_ok
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPageRequestDto {
    limit: u32,
    before_capture_order: Option<CaptureOrder>,
}

impl HistoryPageRequestDto {
    pub fn new(limit: u32, before_capture_order: Option<CaptureOrder>) -> Result<Self, IpcError> {
        validate_page_limit(limit)?;
        Ok(Self {
            limit,
            before_capture_order,
        })
    }

    #[must_use]
    pub const fn limit(self) -> u32 {
        self.limit
    }

    #[must_use]
    pub const fn before_capture_order(self) -> Option<CaptureOrder> {
        self.before_capture_order
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SearchRequestDto {
    query: String,
    limit: u32,
}

impl SearchRequestDto {
    pub fn new(query: String, limit: u32) -> Result<Self, IpcError> {
        validate_page_limit(limit)?;
        validate_search_query(&query)?;
        Ok(Self { query, limit })
    }

    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClipPreviewDto {
    event_id: ClipEventId,
    capture_order: CaptureOrder,
    observed_at: UtcUnixMicros,
    kind: ClipPreviewKind,
    preview: String,
    source_label: Option<String>,
    pinned: bool,
    unavailable: bool,
}

impl ClipPreviewDto {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: ClipEventId,
        capture_order: CaptureOrder,
        observed_at: UtcUnixMicros,
        kind: ClipPreviewKind,
        preview: String,
        source_label: Option<String>,
        pinned: bool,
        unavailable: bool,
    ) -> Result<Self, IpcError> {
        validate_text(&preview, MAX_PREVIEW_BYTES, "preview exceeds byte limit")?;
        if let Some(source_label) = &source_label {
            validate_text(
                source_label,
                MAX_SOURCE_LABEL_BYTES,
                "source label exceeds byte limit",
            )?;
        }
        if kind == ClipPreviewKind::Unavailable && !preview.is_empty() {
            return Err(IpcError::InvalidDto("unavailable preview must be empty"));
        }
        if (kind == ClipPreviewKind::Unavailable) != unavailable {
            return Err(IpcError::InvalidDto(
                "preview kind and unavailable state disagree",
            ));
        }
        Ok(Self {
            event_id,
            capture_order,
            observed_at,
            kind,
            preview,
            source_label,
            pinned,
            unavailable,
        })
    }

    #[must_use]
    pub const fn event_id(&self) -> ClipEventId {
        self.event_id
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
    pub const fn kind(&self) -> ClipPreviewKind {
        self.kind
    }

    #[must_use]
    pub fn preview(&self) -> &str {
        &self.preview
    }

    #[must_use]
    pub fn source_label(&self) -> Option<&str> {
        self.source_label.as_deref()
    }

    #[must_use]
    pub const fn pinned(&self) -> bool {
        self.pinned
    }

    #[must_use]
    pub const fn unavailable(&self) -> bool {
        self.unavailable
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct HistoryPageResponseDto {
    items: Vec<ClipPreviewDto>,
    has_more: bool,
}

impl HistoryPageResponseDto {
    pub fn new(items: Vec<ClipPreviewDto>, has_more: bool) -> Result<Self, IpcError> {
        validate_preview_count(&items)?;
        Ok(Self { items, has_more })
    }

    #[must_use]
    pub fn items(&self) -> &[ClipPreviewDto] {
        &self.items
    }

    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.has_more
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SearchResponseDto {
    items: Vec<ClipPreviewDto>,
    has_more: bool,
}

impl SearchResponseDto {
    pub fn new(items: Vec<ClipPreviewDto>, has_more: bool) -> Result<Self, IpcError> {
        validate_preview_count(&items)?;
        Ok(Self { items, has_more })
    }

    #[must_use]
    pub fn items(&self) -> &[ClipPreviewDto] {
        &self.items
    }

    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.has_more
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProtocolErrorDto {
    code: ProtocolErrorCode,
    retryable: bool,
    developer_detail: Option<String>,
}

impl ProtocolErrorDto {
    pub fn new(
        code: ProtocolErrorCode,
        retryable: bool,
        developer_detail: Option<String>,
    ) -> Result<Self, IpcError> {
        if let Some(detail) = &developer_detail {
            validate_text(
                detail,
                MAX_ERROR_DETAIL_BYTES,
                "error detail exceeds byte limit",
            )?;
        }
        Ok(Self {
            code,
            retryable,
            developer_detail,
        })
    }

    #[must_use]
    pub const fn code(&self) -> ProtocolErrorCode {
        self.code
    }

    #[must_use]
    pub const fn retryable(&self) -> bool {
        self.retryable
    }

    #[must_use]
    pub fn developer_detail(&self) -> Option<&str> {
        self.developer_detail.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkEndDto {
    total_bytes: u64,
    chunk_count: u32,
}

impl BulkEndDto {
    pub const fn new(total_bytes: u64, chunk_count: u32) -> Result<Self, IpcError> {
        if total_bytes == 0 {
            return Err(IpcError::InvalidDto("bulk total bytes must be nonzero"));
        }
        if chunk_count == 0 {
            return Err(IpcError::InvalidDto("bulk chunk count must be nonzero"));
        }
        Ok(Self {
            total_bytes,
            chunk_count,
        })
    }

    #[must_use]
    pub const fn total_bytes(self) -> u64 {
        self.total_bytes
    }

    #[must_use]
    pub const fn chunk_count(self) -> u32 {
        self.chunk_count
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum RequestDto {
    Health(HealthRequestDto),
    HistoryPage(HistoryPageRequestDto),
    Search(SearchRequestDto),
}

#[derive(Clone, PartialEq, Eq)]
pub enum ResponseDto {
    Health(HealthResponseDto),
    HistoryPage(HistoryPageResponseDto),
    Search(SearchResponseDto),
    Error(ProtocolErrorDto),
}

fn validate_protocol(protocol_major: u32, min_minor: u32, max_minor: u32) -> Result<(), IpcError> {
    if protocol_major == 0 {
        return Err(IpcError::InvalidDto("protocol major must be nonzero"));
    }
    if min_minor > max_minor {
        return Err(IpcError::InvalidDto("minor range is invalid"));
    }
    Ok(())
}

fn validate_nonce(nonce: &[u8; NONCE_BYTES]) -> Result<(), IpcError> {
    if nonce.iter().all(|byte| *byte == 0) {
        return Err(IpcError::InvalidDto("nonce must not be all zero"));
    }
    Ok(())
}

fn validate_capabilities(
    capabilities: impl IntoIterator<Item = Capability>,
) -> Result<Vec<Capability>, IpcError> {
    let mut unique = BTreeSet::new();
    let mut values = Vec::new();
    for capability in capabilities {
        if !unique.insert(capability) {
            return Err(IpcError::InvalidDto("capability is duplicated"));
        }
        values.push(capability);
    }
    if values.is_empty() {
        return Err(IpcError::InvalidDto("capabilities must not be empty"));
    }
    Ok(values)
}

const fn validate_page_limit(limit: u32) -> Result<(), IpcError> {
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        return Err(IpcError::InvalidDto("page limit must be between 1 and 100"));
    }
    Ok(())
}

fn validate_search_query(query: &str) -> Result<(), IpcError> {
    if query.trim().is_empty() {
        return Err(IpcError::InvalidDto("search query must not be empty"));
    }
    if query.contains('\0') {
        return Err(IpcError::InvalidDto("search query contains NUL"));
    }
    if query.len() > MAX_QUERY_BYTES {
        return Err(IpcError::InvalidDto("search query exceeds byte limit"));
    }
    if query.split_whitespace().count() > MAX_QUERY_TERMS {
        return Err(IpcError::InvalidDto("search query has too many terms"));
    }
    Ok(())
}

fn validate_text(value: &str, limit: usize, limit_message: &'static str) -> Result<(), IpcError> {
    if value.contains('\0') {
        return Err(IpcError::InvalidDto("text contains NUL"));
    }
    if value.len() > limit {
        return Err(IpcError::InvalidDto(limit_message));
    }
    Ok(())
}

fn validate_preview_count(items: &[ClipPreviewDto]) -> Result<(), IpcError> {
    if items.len() > MAX_PREVIEWS {
        return Err(IpcError::InvalidDto("response contains too many previews"));
    }
    Ok(())
}
