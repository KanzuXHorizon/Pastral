use pastral_ipc_core::{
    CorrelationId, FRAME_HEADER_BYTES, Frame, FrameDecoder, FrameHeader, FrameKind, FrameLimits,
    IpcError,
};

fn frame(kind: FrameKind, body: &[u8], sequence: u32) -> Frame {
    let correlation = CorrelationId::new_v4();
    let header = FrameHeader::new(
        kind,
        u32::try_from(body.len()).unwrap(),
        sequence,
        correlation,
        FrameLimits::default(),
    )
    .unwrap();
    Frame::new(header, body.to_vec()).unwrap()
}

fn encoded(frame: &Frame) -> Vec<u8> {
    let mut bytes = frame.header().encode().to_vec();
    bytes.extend_from_slice(frame.body());
    bytes
}

#[test]
fn every_header_split_reassembles_without_allocating_early() {
    let expected = frame(FrameKind::ControlProto, b"header split", 0);
    let bytes = encoded(&expected);

    for split in 0..=FRAME_HEADER_BYTES {
        let mut decoder = FrameDecoder::new(FrameLimits::default());
        let first = decoder.push(&bytes[..split]).unwrap();
        assert!(first.is_empty(), "split {split}");
        if split < FRAME_HEADER_BYTES {
            assert_eq!(decoder.allocated_body_capacity(), 0, "split {split}");
        }
        let second = decoder.push(&bytes[split..]).unwrap();
        assert_eq!(second, vec![expected.clone()], "split {split}");
        assert_eq!(decoder.allocated_body_capacity(), expected.body().len());
        decoder.finish().unwrap();
    }
}

#[test]
fn every_representative_body_split_reassembles() {
    let expected = frame(
        FrameKind::ControlProto,
        b"body split coverage with enough bytes",
        0,
    );
    let bytes = encoded(&expected);

    for body_split in 0..=expected.body().len() {
        let split = FRAME_HEADER_BYTES + body_split;
        let mut decoder = FrameDecoder::new(FrameLimits::default());
        let mut frames = decoder.push(&bytes[..split]).unwrap();
        frames.extend(decoder.push(&bytes[split..]).unwrap());
        assert_eq!(frames, vec![expected.clone()], "body split {body_split}");
        decoder.finish().unwrap();
    }
}

#[test]
fn maximum_control_body_uses_only_validated_capacity_at_critical_splits() {
    let limits = FrameLimits::default();
    let body = vec![0xA5; usize::try_from(limits.max_control_body_bytes()).unwrap()];
    let expected = frame(FrameKind::ControlProto, &body, 0);
    let bytes = encoded(&expected);
    let body_len = expected.body().len();

    for body_split in [0, 1, body_len / 2, body_len - 1, body_len] {
        let split = FRAME_HEADER_BYTES + body_split;
        let mut decoder = FrameDecoder::new(limits);
        let mut frames = decoder.push(&bytes[..split]).unwrap();
        assert_eq!(decoder.allocated_body_capacity(), body_len);
        frames.extend(decoder.push(&bytes[split..]).unwrap());
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].body().len(), body_len);
        assert_eq!(frames[0].body()[0], 0xA5);
        assert_eq!(frames[0].body()[body_len - 1], 0xA5);
        decoder.finish().unwrap();
    }
}

#[test]
fn one_byte_feeds_and_coalesced_frames_are_boundary_independent() {
    let frames = vec![
        frame(FrameKind::HelloProto, b"hello", 0),
        frame(FrameKind::ControlProto, b"request", 0),
        frame(FrameKind::ProtocolErrorProto, b"error", 0),
    ];
    let stream = frames.iter().flat_map(encoded).collect::<Vec<_>>();

    let mut one_byte = FrameDecoder::new(FrameLimits::default());
    let mut decoded = Vec::new();
    for byte in &stream {
        decoded.extend(one_byte.push(core::slice::from_ref(byte)).unwrap());
    }
    assert_eq!(decoded, frames);
    one_byte.finish().unwrap();

    let mut coalesced = FrameDecoder::new(FrameLimits::default());
    assert_eq!(coalesced.push(&stream).unwrap(), frames);
    coalesced.finish().unwrap();
}

#[test]
fn complete_frame_plus_partial_next_frame_preserves_progress() {
    let first = frame(FrameKind::ControlProto, b"first", 0);
    let second = frame(FrameKind::ControlProto, b"second", 0);
    let first_bytes = encoded(&first);
    let second_bytes = encoded(&second);
    let mut stream = first_bytes.clone();
    stream.extend_from_slice(&second_bytes[..10]);

    let mut decoder = FrameDecoder::new(FrameLimits::default());
    assert_eq!(decoder.push(&stream).unwrap(), vec![first]);
    assert_eq!(decoder.finish(), Err(IpcError::TruncatedFrame));

    let mut decoder = FrameDecoder::new(FrameLimits::default());
    assert_eq!(decoder.push(&first_bytes).unwrap().len(), 1);
    assert!(decoder.push(&second_bytes[..10]).unwrap().is_empty());
    assert_eq!(decoder.push(&second_bytes[10..]).unwrap(), vec![second]);
    decoder.finish().unwrap();
}

#[test]
fn empty_body_frames_emit_without_body_allocation() {
    let expected = frame(FrameKind::ControlProto, b"", 0);
    let bytes = encoded(&expected);
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    assert_eq!(decoder.push(&bytes).unwrap(), vec![expected]);
    assert_eq!(decoder.allocated_body_capacity(), 0);
    decoder.finish().unwrap();
}

#[test]
fn too_many_frames_poison_the_decoder() {
    let limits = FrameLimits::new(1024, 1024, 1, 1).unwrap();
    let first = frame(FrameKind::ControlProto, b"", 0);
    let second = frame(FrameKind::ControlProto, b"", 0);
    let mut stream = encoded(&first);
    stream.extend_from_slice(&encoded(&second));

    let mut decoder = FrameDecoder::new(limits);
    assert_eq!(decoder.push(&stream), Err(IpcError::TooManyFrames));
    assert!(decoder.is_poisoned());
    assert_eq!(decoder.push(&[]), Err(IpcError::DecoderPoisoned));
    assert_eq!(decoder.finish(), Err(IpcError::DecoderPoisoned));
}

#[test]
fn invalid_header_poison_and_truncated_finish_are_distinct() {
    let mut invalid = frame(FrameKind::ControlProto, b"body", 0).header().encode();
    invalid[0] = b'X';
    let mut decoder = FrameDecoder::new(FrameLimits::default());
    assert_eq!(decoder.push(&invalid), Err(IpcError::InvalidMagic));
    assert!(decoder.is_poisoned());
    assert_eq!(decoder.push(b"more"), Err(IpcError::DecoderPoisoned));

    let valid = encoded(&frame(FrameKind::ControlProto, b"body", 0));
    let mut partial_header = FrameDecoder::new(FrameLimits::default());
    assert!(partial_header.push(&valid[..10]).unwrap().is_empty());
    assert_eq!(partial_header.finish(), Err(IpcError::TruncatedFrame));

    let mut partial_body = FrameDecoder::new(FrameLimits::default());
    assert!(
        partial_body
            .push(&valid[..FRAME_HEADER_BYTES + 1])
            .unwrap()
            .is_empty()
    );
    assert_eq!(partial_body.finish(), Err(IpcError::TruncatedFrame));
}
