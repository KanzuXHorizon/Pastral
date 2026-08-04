#![cfg(windows)]

use std::{fs, path::PathBuf};

use pastral_agent::{
    AgentIdentity, DiagnosticStoragePolicy, StorageCaptureSink, diagnostic_storage_limits,
};
use pastral_agent_core::{CaptureSink, CapturedText, TextCaptureRequest};
use pastral_domain::{ClipEventId, ClipboardFormatIdentity, StandardFormatId, UtcUnixMicros};
use pastral_storage::Storage;

fn encoded(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-adapter-test-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn identity_is_created_once_and_reused_across_reopen() {
    let root = TestRoot::new();

    let first = AgentIdentity::load_or_create(root.path()).unwrap();
    let second = AgentIdentity::load_or_create(root.path()).unwrap();

    assert_eq!(first, second);
    let content = fs::read_to_string(root.path().join("agent-identity.txt")).unwrap();
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "version=1");
    assert_eq!(lines[1], format!("profile_id={}", first.profile_id()));
    assert_eq!(
        lines[2],
        format!("ordinary_domain_id={}", first.ordinary_domain_id())
    );
    assert!(!content.to_ascii_lowercase().contains("clipboard"));
    assert!(!content.to_ascii_lowercase().contains("preview"));
}

#[test]
fn malformed_identity_file_fails_closed() {
    let root = TestRoot::new();
    fs::write(
        root.path().join("agent-identity.txt"),
        "version=1\nprofile_id=not-a-uuid\nordinary_domain_id=also-invalid\n",
    )
    .unwrap();

    assert!(AgentIdentity::load_or_create(root.path()).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("agent-identity.txt")).unwrap(),
        "version=1\nprofile_id=not-a-uuid\nordinary_domain_id=also-invalid\n"
    );
}

#[test]
fn storage_sink_persists_exact_text_and_assigns_order() {
    let root = TestRoot::new();
    let identity = AgentIdentity::load_or_create(root.path()).unwrap();
    let storage = Storage::open(
        root.path().join("storage"),
        diagnostic_storage_limits(),
        DiagnosticStoragePolicy,
    )
    .unwrap();
    let mut sink = StorageCaptureSink::new(storage);
    let raw = encoded("alpha e\u{301}");
    let request = TextCaptureRequest::new(
        UtcUnixMicros::new(1_700_000_000_000_000).unwrap(),
        identity.profile_id(),
        identity.protection_domain(),
        CapturedText::new("alpha e\u{301}".to_owned(), raw.clone()).unwrap(),
    );

    let first = sink.store_text(request).unwrap();
    let empty = sink
        .store_text(TextCaptureRequest::new(
            UtcUnixMicros::new(1_700_000_000_000_001).unwrap(),
            identity.profile_id(),
            identity.protection_domain(),
            CapturedText::new(String::new(), encoded("")).unwrap(),
        ))
        .unwrap();

    assert_eq!(first.capture_order().get(), 1);
    assert_eq!(empty.capture_order().get(), 2);
    let loaded = sink
        .storage()
        .load_clip(first.clip_event_id())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.event().capture_order(), first.capture_order());
    let representation = &loaded.event().representations()[0];
    assert_eq!(
        representation.format(),
        &ClipboardFormatIdentity::Standard(StandardFormatId::new(13))
    );
    assert_eq!(
        sink.storage()
            .read_representation(representation.id())
            .unwrap()
            .unwrap(),
        raw
    );
    let hits = sink.storage().search("alpha", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].clip_event_id(), first.clip_event_id());
}
