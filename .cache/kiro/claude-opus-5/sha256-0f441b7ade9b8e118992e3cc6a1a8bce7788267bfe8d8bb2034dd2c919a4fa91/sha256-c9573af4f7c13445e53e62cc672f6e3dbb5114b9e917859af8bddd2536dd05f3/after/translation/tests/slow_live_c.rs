//! Fully live differential tests for the `long_exec` pipeline: the C `.so`'s
//! real `long_exec` is executed in-process next to the Rust one, so nothing here
//! depends on the recorded files in `tests/ground_truth/`.
//!
//! Each C call runs 2000 * 262144 * 100 kernel steps (~8 minutes in the
//! unoptimised C build), which is why these are `#[ignore]`d by default:
//!
//! ```sh
//! cargo test --release --test slow_live_c -- --ignored --test-threads=1 --nocapture
//! ```

mod common;

use common::*;
use std::ffi::c_int;

fn to_i32s(bytes: &[u8]) -> Vec<c_int> {
    bytes
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// CONFIGS rows 27 + 34, fully live: identical stdout, identical 1 MiB `array`
/// after the pipeline, and still identical after one further low-level pass.
#[test]
#[ignore = "runs the real C long_exec: ~8 minutes"]
fn live_full_pipeline_and_extra_pass() {
    let seed = 42u32;
    let l = libs();

    let c_out = capture_stdout(|| l.c.long_exec(seed));
    let c_arr = l.c.read_array_bytes();

    let rs_out = capture_stdout(|| l.rs.long_exec(seed));
    let rs_arr = l.rs.read_array_bytes();

    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rs_out),
        "live long_exec stdout differs"
    );
    assert!(!c_out.is_empty(), "C printed nothing");

    if c_arr != rs_arr {
        let ne = c_arr
            .chunks(4)
            .zip(rs_arr.chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        panic!("live long_exec: `array` differs in {ne} of {ARRAY_LEN} elements");
    }

    // Row 34: one more low-level pass on top of the post-pipeline state.
    let before = to_i32s(&c_arr);
    l.c.peo();
    l.rs.peo();
    assert_arrays_eq(
        "live long_exec + extra pass",
        &before,
        &l.c.read_array(),
        &l.rs.read_array(),
    );
}

/// CONFIGS rows 36 + 37 / ERRORS rows 7 + 8, fully live: repeated calls with the
/// same and with different seeds, interleaved between the two libraries.
#[test]
#[ignore = "runs the real C long_exec three times: ~24 minutes"]
fn live_repeated_and_alternating_seeds() {
    let l = libs();

    let c_a1 = capture_stdout(|| l.c.long_exec(7));
    let r_a1 = capture_stdout(|| l.rs.long_exec(7));
    assert_eq!(c_a1, r_a1, "seed 7, first call");

    let c_b = capture_stdout(|| l.c.long_exec(12345));
    let r_b = capture_stdout(|| l.rs.long_exec(12345));
    assert_eq!(c_b, r_b, "seed 12345");
    assert_ne!(c_a1, c_b, "different seeds gave the same output");

    // Back to the first seed: must reproduce exactly, on both sides.
    let c_a2 = capture_stdout(|| l.c.long_exec(7));
    let r_a2 = capture_stdout(|| l.rs.long_exec(7));
    assert_eq!(c_a2, c_a1, "C: state carried over between calls");
    assert_eq!(r_a2, r_a1, "Rust: state carried over between calls");
    assert_eq!(c_a2, r_a2, "seed 7, repeat call");
}
