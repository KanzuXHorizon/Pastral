use std::{collections::VecDeque, num::NonZeroUsize, time::Duration};

use pastral_agent_core::{
    CaptureConfig, CaptureCoordinator, CaptureOutcome, CaptureSequence, CaptureSink,
    CaptureSinkError, CaptureSource, CaptureSourceError, CapturedText, Clock, Sleeper,
    StoredCapture, TextCaptureRequest,
};
use pastral_domain::{
    CaptureOrder, ClipEventId, ProfileId, ProtectionDomain, ProtectionDomainId, UtcUnixMicros,
};

fn encoded(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn config() -> CaptureConfig {
    CaptureConfig::new(
        ProfileId::new_v4(),
        ProtectionDomain::Ordinary(ProtectionDomainId::new_v4()),
        NonZeroUsize::new(16 * 1024 * 1024).unwrap(),
        vec![
            Duration::ZERO,
            Duration::from_millis(5),
            Duration::from_millis(15),
            Duration::from_millis(35),
        ],
    )
    .unwrap()
}

#[derive(Default)]
struct FakeSource {
    attempts: usize,
    results: VecDeque<Result<Option<CapturedText>, CaptureSourceError>>,
}

impl FakeSource {
    fn with_results(
        results: impl IntoIterator<Item = Result<Option<CapturedText>, CaptureSourceError>>,
    ) -> Self {
        Self {
            attempts: 0,
            results: results.into_iter().collect(),
        }
    }
}

impl CaptureSource for FakeSource {
    fn capture_unicode_text(
        &mut self,
        _max_bytes: NonZeroUsize,
    ) -> Result<Option<CapturedText>, CaptureSourceError> {
        self.attempts += 1;
        self.results
            .pop_front()
            .expect("test source must provide one result per expected attempt")
    }
}

#[derive(Default)]
struct FakeSink {
    requests: Vec<TextCaptureRequest>,
    results: VecDeque<Result<StoredCapture, CaptureSinkError>>,
}

impl FakeSink {
    fn with_results(
        results: impl IntoIterator<Item = Result<StoredCapture, CaptureSinkError>>,
    ) -> Self {
        Self {
            requests: Vec::new(),
            results: results.into_iter().collect(),
        }
    }
}

impl CaptureSink for FakeSink {
    fn store_text(
        &mut self,
        request: TextCaptureRequest,
    ) -> Result<StoredCapture, CaptureSinkError> {
        self.requests.push(request);
        self.results
            .pop_front()
            .expect("test sink must provide one result per expected commit")
    }
}

struct FixedClock(UtcUnixMicros);

impl Clock for FixedClock {
    fn now_utc_micros(&mut self) -> Result<UtcUnixMicros, pastral_agent_core::AgentError> {
        Ok(self.0)
    }
}

#[derive(Default)]
struct RecordingSleeper {
    durations: Vec<Duration>,
}

impl Sleeper for RecordingSleeper {
    fn sleep(&mut self, duration: Duration) {
        self.durations.push(duration);
    }
}

fn stored(order: u64) -> StoredCapture {
    StoredCapture::new(ClipEventId::new_v4(), CaptureOrder::new(order).unwrap())
}

fn clock() -> FixedClock {
    FixedClock(UtcUnixMicros::new(1_700_000_000_000_000).unwrap())
}

#[test]
fn immediate_success_attempts_once_and_commits_once() {
    let captured = CapturedText::new("plain".to_owned(), encoded("plain")).unwrap();
    let mut source = FakeSource::with_results([Ok(Some(captured))]);
    let expected = stored(1);
    let mut sink = FakeSink::with_results([Ok(expected)]);
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();

    let outcome = coordinator.handle_notification(
        CaptureSequence::new(1).unwrap(),
        &mut source,
        &mut sink,
        &mut clock,
        &mut sleeper,
    );

    assert_eq!(
        outcome,
        CaptureOutcome::Stored {
            clip_event_id: expected.clip_event_id(),
            capture_order: expected.capture_order(),
        }
    );
    assert_eq!(source.attempts, 1);
    assert_eq!(sink.requests.len(), 1);
    assert!(sleeper.durations.is_empty());
}

#[test]
fn successful_sequence_is_suppressed_on_repeat() {
    let captured = CapturedText::new("plain".to_owned(), encoded("plain")).unwrap();
    let mut source = FakeSource::with_results([Ok(Some(captured))]);
    let mut sink = FakeSink::with_results([Ok(stored(1))]);
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();
    let sequence = CaptureSequence::new(7).unwrap();

    assert!(matches!(
        coordinator
            .handle_notification(sequence, &mut source, &mut sink, &mut clock, &mut sleeper,),
        CaptureOutcome::Stored { .. }
    ));
    assert_eq!(
        coordinator
            .handle_notification(sequence, &mut source, &mut sink, &mut clock, &mut sleeper,),
        CaptureOutcome::DuplicateNotification
    );
    assert_eq!(source.attempts, 1);
    assert_eq!(sink.requests.len(), 1);
}

#[test]
fn transient_failures_follow_exact_retry_schedule() {
    let captured = CapturedText::new("retry".to_owned(), encoded("retry")).unwrap();
    let mut source = FakeSource::with_results([
        Err(CaptureSourceError::Busy),
        Err(CaptureSourceError::Busy),
        Ok(Some(captured)),
    ]);
    let mut sink = FakeSink::with_results([Ok(stored(1))]);
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();

    assert!(matches!(
        coordinator.handle_notification(
            CaptureSequence::new(1).unwrap(),
            &mut source,
            &mut sink,
            &mut clock,
            &mut sleeper,
        ),
        CaptureOutcome::Stored { .. }
    ));
    assert_eq!(source.attempts, 3);
    assert_eq!(
        sleeper.durations,
        vec![Duration::from_millis(5), Duration::from_millis(15)]
    );
}

#[test]
fn retry_exhaustion_never_commits() {
    let mut source = FakeSource::with_results([
        Err(CaptureSourceError::Busy),
        Err(CaptureSourceError::Busy),
        Err(CaptureSourceError::Busy),
        Err(CaptureSourceError::Busy),
    ]);
    let mut sink = FakeSink::default();
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();

    assert_eq!(
        coordinator.handle_notification(
            CaptureSequence::new(2).unwrap(),
            &mut source,
            &mut sink,
            &mut clock,
            &mut sleeper,
        ),
        CaptureOutcome::RetryExhausted { attempts: 4 }
    );
    assert_eq!(source.attempts, 4);
    assert!(sink.requests.is_empty());
    assert_eq!(
        sleeper.durations,
        vec![
            Duration::from_millis(5),
            Duration::from_millis(15),
            Duration::from_millis(35),
        ]
    );
}

#[test]
fn no_text_is_terminal_without_sleep() {
    let mut source = FakeSource::with_results([Ok(None)]);
    let mut sink = FakeSink::default();
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();

    assert_eq!(
        coordinator.handle_notification(
            CaptureSequence::new(3).unwrap(),
            &mut source,
            &mut sink,
            &mut clock,
            &mut sleeper,
        ),
        CaptureOutcome::NoSupportedRepresentation
    );
    assert_eq!(source.attempts, 1);
    assert!(sink.requests.is_empty());
    assert!(sleeper.durations.is_empty());
}

#[test]
fn sink_failure_leaves_sequence_retryable() {
    let first = CapturedText::new("retry store".to_owned(), encoded("retry store")).unwrap();
    let second = first.clone();
    let mut source = FakeSource::with_results([Ok(Some(first)), Ok(Some(second))]);
    let expected = stored(1);
    let mut sink = FakeSink::with_results([Err(CaptureSinkError::StorageFailure), Ok(expected)]);
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let mut coordinator = CaptureCoordinator::new(config()).unwrap();
    let sequence = CaptureSequence::new(4).unwrap();

    assert_eq!(
        coordinator
            .handle_notification(sequence, &mut source, &mut sink, &mut clock, &mut sleeper,),
        CaptureOutcome::StorageFailed
    );
    assert_eq!(
        coordinator
            .handle_notification(sequence, &mut source, &mut sink, &mut clock, &mut sleeper,),
        CaptureOutcome::Stored {
            clip_event_id: expected.clip_event_id(),
            capture_order: expected.capture_order(),
        }
    );
    assert_eq!(source.attempts, 2);
    assert_eq!(sink.requests.len(), 2);
}

#[test]
fn empty_text_remains_valid_and_reaches_the_sink_exactly() {
    let raw = encoded("");
    let captured = CapturedText::new(String::new(), raw.clone()).unwrap();
    let mut source = FakeSource::with_results([Ok(Some(captured))]);
    let mut sink = FakeSink::with_results([Ok(stored(1))]);
    let mut clock = clock();
    let mut sleeper = RecordingSleeper::default();
    let config = config();
    let expected_profile = config.profile_id();
    let expected_domain = config.protection_domain();
    let mut coordinator = CaptureCoordinator::new(config).unwrap();

    assert!(matches!(
        coordinator.handle_notification(
            CaptureSequence::new(5).unwrap(),
            &mut source,
            &mut sink,
            &mut clock,
            &mut sleeper,
        ),
        CaptureOutcome::Stored { .. }
    ));
    let request = &sink.requests[0];
    assert_eq!(request.captured_text().text(), "");
    assert_eq!(request.captured_text().raw_utf16le(), raw);
    assert_eq!(request.profile_id(), expected_profile);
    assert_eq!(request.protection_domain(), expected_domain);
}

#[test]
fn text_and_exact_utf16_bytes_are_not_normalized() {
    let precomposed = CapturedText::new("é".to_owned(), encoded("é")).unwrap();
    let decomposed = CapturedText::new("e\u{301}".to_owned(), encoded("e\u{301}")).unwrap();

    assert_ne!(precomposed.text(), decomposed.text());
    assert_ne!(precomposed.raw_utf16le(), decomposed.raw_utf16le());
}
