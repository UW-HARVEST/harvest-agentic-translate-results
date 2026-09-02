//! CONFIGS.md row C1b — `driver` from a freshly-parked zero accumulator.
//!
//! Kept in its own test binary so it cannot perturb the pristine-state
//! assertion in `initial_state.rs`.

mod common;

use common::with_libs;

#[test]
fn c1b_driver_from_zero_prints_triangular_numbers() {
    with_libs(|h| {
        h.park_accumulator_at(0, "C1b");
        let out = h.driver(1, "C1b");
        assert_eq!(
            out,
            b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n".to_vec(),
            "C1b: got {:?}",
            String::from_utf8_lossy(&out)
        );
    });
}

#[test]
fn c1b_driver_from_zero_stride_two() {
    with_libs(|h| {
        h.park_accumulator_at(0, "C1b");
        let out = h.driver(2, "C1b");
        assert_eq!(
            out,
            b"0\n2\n6\n12\n20\n30\n42\n56\n72\n90\n".to_vec(),
            "C1b: got {:?}",
            String::from_utf8_lossy(&out)
        );
    });
}
