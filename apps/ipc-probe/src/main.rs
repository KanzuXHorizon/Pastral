use std::{
    env,
    ffi::OsString,
    fmt, fs,
    hint::black_box,
    process::ExitCode,
    time::{Duration, Instant},
};

use pastral_domain::{CaptureOrder, ClipEventId, UtcUnixMicros};
use pastral_ipc_core::{
    ClipPreviewDto, ClipPreviewKind, CorrelationId, Frame, FrameDecoder, FrameHeader, FrameKind,
    FrameLimits, HealthRequestDto, HistoryPageResponseDto, RequestDto, ResponseDto,
};
use pastral_ipc_schema::{
    PROTOBUF_RELEASE, decode_request, decode_response, encode_request, encode_response,
    schema_sha256,
};

const DEFAULT_ITERATIONS: u32 = 10_000;
const MAX_ITERATIONS: u32 = 100_000;
const RESPONSE_ITEMS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProbeConfig {
    iterations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProbeError {
    InvalidArguments,
    IterationLimit,
    Protocol(&'static str),
    Ipc(String),
    Io(&'static str),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments => write!(
                f,
                "usage: pastral-ipc-probe [--iterations <1..={MAX_ITERATIONS}>]"
            ),
            Self::IterationLimit => write!(f, "IPC probe iteration count is outside bounds"),
            Self::Protocol(reason) => write!(f, "IPC probe invariant failed: {reason}"),
            Self::Ipc(reason) => write!(f, "IPC probe protocol operation failed: {reason}"),
            Self::Io(operation) => write!(f, "IPC probe I/O failed: {operation}"),
        }
    }
}

impl std::error::Error for ProbeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ProbeMetrics {
    iterations: u32,
    round_trips: u32,
    total: Duration,
    one_byte: Duration,
    coalesced: Duration,
    max_body_capacity: usize,
}

fn main() -> ExitCode {
    match parse_arguments(env::args_os().skip(1)).and_then(run_probe) {
        Ok(metrics) => match print_metrics(metrics) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn parse_arguments(args: impl IntoIterator<Item = OsString>) -> Result<ProbeConfig, ProbeError> {
    let values = args.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return Ok(ProbeConfig {
            iterations: DEFAULT_ITERATIONS,
        });
    }
    if values.len() != 2 || values[0] != "--iterations" {
        return Err(ProbeError::InvalidArguments);
    }
    let value = values[1]
        .to_str()
        .ok_or(ProbeError::InvalidArguments)?
        .parse::<u32>()
        .map_err(|_| ProbeError::InvalidArguments)?;
    if value == 0 || value > MAX_ITERATIONS {
        return Err(ProbeError::IterationLimit);
    }
    Ok(ProbeConfig { iterations: value })
}

fn run_probe(config: ProbeConfig) -> Result<ProbeMetrics, ProbeError> {
    let request = RequestDto::Health(HealthRequestDto);
    let response = ResponseDto::HistoryPage(
        HistoryPageResponseDto::new(build_previews()?, false).map_err(ipc_error)?,
    );
    let request_correlation = fixed_correlation(1)?;
    let response_correlation = fixed_correlation(2)?;
    let limits = FrameLimits::default();
    let mut metrics = ProbeMetrics {
        iterations: config.iterations,
        ..ProbeMetrics::default()
    };

    for _ in 0..config.iterations {
        let total_start = Instant::now();
        let request_body = encode_request(black_box(&request)).map_err(ipc_error)?;
        let response_body = encode_response(black_box(&response)).map_err(ipc_error)?;
        let request_frame = encode_frame(&request_body, request_correlation, limits)?;
        let response_frame = encode_frame(&response_body, response_correlation, limits)?;

        let one_byte_start = Instant::now();
        let mut one_byte_decoder = FrameDecoder::new(limits);
        let mut one_byte_frames = Vec::with_capacity(1);
        for byte in &request_frame {
            one_byte_frames.extend(
                one_byte_decoder
                    .push(core::slice::from_ref(byte))
                    .map_err(ipc_error)?,
            );
        }
        one_byte_decoder.finish().map_err(ipc_error)?;
        metrics.one_byte += one_byte_start.elapsed();
        if one_byte_frames.len() != 1 {
            return Err(ProbeError::Protocol("one-byte frame count"));
        }
        let decoded_request = decode_request(one_byte_frames[0].body()).map_err(ipc_error)?;
        if decoded_request != request {
            return Err(ProbeError::Protocol("request semantic round trip"));
        }

        let mut stream = Vec::with_capacity(request_frame.len() + response_frame.len());
        stream.extend_from_slice(&request_frame);
        stream.extend_from_slice(&response_frame);
        let coalesced_start = Instant::now();
        let mut coalesced_decoder = FrameDecoder::new(limits);
        let coalesced_frames = coalesced_decoder.push(&stream).map_err(ipc_error)?;
        metrics.coalesced += coalesced_start.elapsed();
        metrics.max_body_capacity = metrics
            .max_body_capacity
            .max(coalesced_decoder.allocated_body_capacity());
        coalesced_decoder.finish().map_err(ipc_error)?;
        if coalesced_frames.len() != 2 {
            return Err(ProbeError::Protocol("coalesced frame count"));
        }
        validate_frame(&coalesced_frames[0], request_correlation)?;
        validate_frame(&coalesced_frames[1], response_correlation)?;
        let coalesced_request = decode_request(coalesced_frames[0].body()).map_err(ipc_error)?;
        let decoded_response = decode_response(coalesced_frames[1].body()).map_err(ipc_error)?;
        if coalesced_request != request || decoded_response != response {
            return Err(ProbeError::Protocol("coalesced semantic round trip"));
        }
        metrics.total += total_start.elapsed();
        metrics.round_trips = metrics
            .round_trips
            .checked_add(1)
            .ok_or(ProbeError::Protocol("round-trip count overflow"))?;
    }

    if metrics.round_trips != config.iterations {
        return Err(ProbeError::Protocol("incomplete round-trip count"));
    }
    Ok(metrics)
}

fn build_previews() -> Result<Vec<ClipPreviewDto>, ProbeError> {
    (0..RESPONSE_ITEMS)
        .map(|index| {
            let kind = match index % 6 {
                0 => ClipPreviewKind::Text,
                1 => ClipPreviewKind::Code,
                2 => ClipPreviewKind::Link,
                3 => ClipPreviewKind::Image,
                4 => ClipPreviewKind::Files,
                _ => ClipPreviewKind::Unavailable,
            };
            let unavailable = kind == ClipPreviewKind::Unavailable;
            ClipPreviewDto::new(
                fixed_event_id(index)?,
                CaptureOrder::new(index + 1).map_err(|_| ProbeError::Protocol("capture order"))?,
                UtcUnixMicros::new(1_700_000_000_000_000 + index as i64)
                    .map_err(|_| ProbeError::Protocol("timestamp"))?,
                kind,
                if unavailable {
                    String::new()
                } else {
                    format!("synthetic-preview-{index:03}")
                },
                Some("synthetic-source.exe".to_owned()),
                index.is_multiple_of(7),
                unavailable,
            )
            .map_err(ipc_error)
        })
        .collect()
}

fn fixed_event_id(index: u64) -> Result<ClipEventId, ProbeError> {
    let text = format!("00000000-0000-4000-8000-{index:012x}");
    ClipEventId::parse_str(&text).map_err(|_| ProbeError::Protocol("fixed event ID"))
}

fn fixed_correlation(last_byte: u8) -> Result<CorrelationId, ProbeError> {
    let mut bytes = [0u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = last_byte;
    CorrelationId::from_bytes(bytes).map_err(ipc_error)
}

fn encode_frame(
    body: &[u8],
    correlation: CorrelationId,
    limits: FrameLimits,
) -> Result<Vec<u8>, ProbeError> {
    let body_length = u32::try_from(body.len())
        .map_err(|_| ProbeError::Protocol("frame body length conversion"))?;
    let header = FrameHeader::new(FrameKind::ControlProto, body_length, 0, correlation, limits)
        .map_err(ipc_error)?;
    let mut bytes = Vec::with_capacity(header.encode().len() + body.len());
    bytes.extend_from_slice(&header.encode());
    bytes.extend_from_slice(body);
    Ok(bytes)
}

fn validate_frame(frame: &Frame, correlation: CorrelationId) -> Result<(), ProbeError> {
    let header = frame.header();
    if header.kind() != FrameKind::ControlProto
        || header.sequence() != 0
        || header.correlation() != correlation
        || usize::try_from(header.body_length()).ok() != Some(frame.body().len())
    {
        return Err(ProbeError::Protocol("decoded frame metadata"));
    }
    Ok(())
}

fn print_metrics(metrics: ProbeMetrics) -> Result<(), ProbeError> {
    let executable = env::current_exe().map_err(|_| ProbeError::Io("resolve executable"))?;
    let executable_bytes = fs::metadata(executable)
        .map_err(|_| ProbeError::Io("read executable metadata"))?
        .len();
    println!("ipc-probe=ok");
    println!("protobuf-release={PROTOBUF_RELEASE}");
    println!("schema-sha256={}", hex(&schema_sha256()));
    println!("iterations={}", metrics.iterations);
    println!("round-trips={}", metrics.round_trips);
    println!("executable-bytes={executable_bytes}");
    println!(
        "average-roundtrip-ns={}",
        average_nanos(metrics.total, metrics.iterations)
    );
    println!(
        "one-byte-average-ns={}",
        average_nanos(metrics.one_byte, metrics.iterations)
    );
    println!(
        "coalesced-average-ns={}",
        average_nanos(metrics.coalesced, metrics.iterations)
    );
    println!("max-body-capacity={}", metrics.max_body_capacity);
    Ok(())
}

fn average_nanos(duration: Duration, count: u32) -> u128 {
    duration.as_nanos() / u128::from(count)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

fn ipc_error(error: impl fmt::Display) -> ProbeError {
    ProbeError::Ipc(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parser_defaults_and_accepts_exact_bounded_iteration_argument() {
        assert_eq!(
            parse_arguments(Vec::<OsString>::new()).unwrap(),
            ProbeConfig {
                iterations: DEFAULT_ITERATIONS
            }
        );
        assert_eq!(
            parse_arguments(args(&["--iterations", "1"])).unwrap(),
            ProbeConfig { iterations: 1 }
        );
        assert_eq!(
            parse_arguments(args(&["--iterations", "100000"])).unwrap(),
            ProbeConfig {
                iterations: MAX_ITERATIONS
            }
        );
    }

    #[test]
    fn parser_rejects_zero_overflow_unknown_duplicate_and_positional_arguments() {
        for invalid in [
            args(&["--iterations"]),
            args(&["--iterations", "0"]),
            args(&["--iterations", "100001"]),
            args(&["--iterations", "not-a-number"]),
            args(&["--unknown", "1"]),
            args(&["10"]),
            args(&["--iterations", "1", "--iterations", "2"]),
        ] {
            assert!(parse_arguments(invalid).is_err());
        }
    }

    #[test]
    fn one_iteration_completes_all_semantic_round_trips() {
        let metrics = run_probe(ProbeConfig { iterations: 1 }).unwrap();
        assert_eq!(metrics.round_trips, 1);
        assert!(metrics.max_body_capacity > 0);
    }
}
