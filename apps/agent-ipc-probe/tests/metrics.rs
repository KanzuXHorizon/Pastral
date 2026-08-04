#![cfg(windows)]

use pastral_agent_ipc_probe::{AdmissionError, evaluate_footprint};

const MIB: u64 = 1024 * 1024;

#[test]
fn exact_ceilings_and_negative_runtime_deltas_are_accepted() {
    let exact =
        evaluate_footprint(2 * MIB, 8 * MIB, 10 * MIB, 12 * MIB, 22 * MIB, 20 * MIB).unwrap();
    assert_eq!(exact.binary_delta_bytes(), 6 * MIB);
    assert_eq!(exact.working_set_delta_bytes(), 12 * MIB as i64);
    assert_eq!(exact.private_delta_bytes(), 8 * MIB as i64);

    let negative =
        evaluate_footprint(2 * MIB, 3 * MIB, 20 * MIB, 10 * MIB, 18 * MIB, 9 * MIB).unwrap();
    assert_eq!(negative.working_set_delta_bytes(), -(2 * MIB as i64));
    assert_eq!(negative.private_delta_bytes(), -(MIB as i64));
}

#[test]
fn zero_underflow_and_one_byte_over_each_ceiling_fail_closed() {
    let invalid = [
        evaluate_footprint(0, 1, 1, 1, 1, 1),
        evaluate_footprint(2, 1, 1, 1, 1, 1),
        evaluate_footprint(1, 1 + 6 * MIB + 1, 1, 1, 1, 1),
        evaluate_footprint(1, 1, 1, 1, 1 + 12 * MIB + 1, 1),
        evaluate_footprint(1, 1, 1, 1, 1, 1 + 8 * MIB + 1),
        evaluate_footprint(1, 1, 1, 1, 1, 25 * MIB + 1),
    ];
    for result in invalid {
        assert!(matches!(
            result,
            Err(AdmissionError::InvalidMetric) | Err(AdmissionError::FootprintCeiling)
        ));
    }
}
