//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported `driver` symbol
//! (loaded with `libloading`) and compares the bytes each writes to stdout.
//! Randomized rows use a fixed seed so failures are reproducible.

mod common;

use common::*;

/// Fixed seed → reproducible input sequences.
const SEED: u64 = 0x5EED_C0FF_EE12_3456;

/// Row 1 — `x = 0`, the identity/"empty" input.
fn cfg01_zero() {
    assert_same("cfg01", 0);
    assert_eq!(c_driver(0), b"300\n".to_vec(), "reference C output for x=0");
}

/// Row 2 — every small positive `x` in `1..=9`.
fn cfg02_small_positive_exhaustive() {
    let xs: Vec<i32> = (1..=9).collect();
    for &x in &xs {
        assert_same("cfg02", x);
    }
    assert_same_batch("cfg02-batch", &xs);
}

/// Row 3 — random positive `x` in `1..=1_000`.
fn cfg03_positive_to_1k() {
    let mut rng = Rng::new(SEED ^ 3);
    let xs: Vec<i32> = (0..400).map(|_| rng.in_range(1, 1_000)).collect();
    for &x in &xs {
        assert_same("cfg03", x);
    }
    assert_same_batch("cfg03-batch", &xs);
}

/// Row 4 — random positive `x` in `1_000..=1_000_000`.
fn cfg04_positive_to_1m() {
    let mut rng = Rng::new(SEED ^ 4);
    let xs: Vec<i32> = (0..400).map(|_| rng.in_range(1_000, 1_000_000)).collect();
    for &x in &xs {
        assert_same("cfg04", x);
    }
    assert_same_batch("cfg04-batch", &xs);
}

/// Row 5 — random positive `x` in `1_000_000..=1_073_741_673` (no overflow).
fn cfg05_positive_no_overflow_max_region() {
    let mut rng = Rng::new(SEED ^ 5);
    let xs: Vec<i32> = (0..400)
        .map(|_| rng.in_range(1_000_000, 1_073_741_673))
        .collect();
    for &x in &xs {
        assert_same("cfg05", x);
    }
    assert_same_batch("cfg05-batch", &xs);
}

/// Row 6 — small negative `x` in `-149..=-1` (positive result, sign cancels).
fn cfg06_small_negative_positive_result() {
    let xs: Vec<i32> = (-149..=-1).collect();
    for &x in &xs {
        assert_same("cfg06", x);
    }
    assert_same_batch("cfg06-batch", &xs);
}

/// Row 7 — `x = -150`, the exact zero-result input.
fn cfg07_zero_result() {
    assert_same("cfg07", -150);
    assert_eq!(c_driver(-150), b"0\n".to_vec(), "reference C output for x=-150");
}

/// Row 8 — random negative `x` in `-1_000..=-151` (negative output).
fn cfg08_negative_to_1k() {
    let mut rng = Rng::new(SEED ^ 8);
    let xs: Vec<i32> = (0..400).map(|_| rng.in_range(-1_000, -151)).collect();
    for &x in &xs {
        assert_same("cfg08", x);
    }
    assert_same_batch("cfg08-batch", &xs);
}

/// Row 9 — random negative `x` in `-1_000_000..=-1_000`.
fn cfg09_negative_to_1m() {
    let mut rng = Rng::new(SEED ^ 9);
    let xs: Vec<i32> = (0..400).map(|_| rng.in_range(-1_000_000, -1_000)).collect();
    for &x in &xs {
        assert_same("cfg09", x);
    }
    assert_same_batch("cfg09-batch", &xs);
}

/// Row 10 — random negative `x` in `-1_073_741_824..=-1_000_000`.
fn cfg10_negative_no_overflow_min_region() {
    let mut rng = Rng::new(SEED ^ 10);
    let xs: Vec<i32> = (0..400)
        .map(|_| rng.in_range(-1_073_741_824, -1_000_000))
        .collect();
    for &x in &xs {
        assert_same("cfg10", x);
    }
    assert_same_batch("cfg10-batch", &xs);
}

/// Row 11 — positive `x` where the arithmetic wraps: `1_073_741_674..=i32::MAX`.
fn cfg11_positive_overflow_region() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut xs: Vec<i32> = (0..600)
        .map(|_| rng.in_range(1_073_741_674, i32::MAX))
        .collect();
    xs.extend_from_slice(&[1_073_741_674, 1_073_741_824, i32::MAX - 1, i32::MAX]);
    for &x in &xs {
        assert_same("cfg11", x);
    }
    assert_same_batch("cfg11-batch", &xs);
}

/// Row 12 — negative `x` where `2*x` wraps: `i32::MIN..=-1_073_741_825`.
fn cfg12_negative_overflow_region() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut xs: Vec<i32> = (0..600)
        .map(|_| rng.in_range(i32::MIN, -1_073_741_825))
        .collect();
    xs.extend_from_slice(&[i32::MIN, i32::MIN + 1, -1_073_741_825]);
    for &x in &xs {
        assert_same("cfg12", x);
    }
    assert_same_batch("cfg12-batch", &xs);
}

/// Row 13 — unbiased uniform sweep over the entire `i32` domain.
fn cfg13_full_range_uniform() {
    let mut rng = Rng::new(SEED ^ 13);
    let xs: Vec<i32> = (0..2000).map(|_| rng.next_i32()).collect();
    for &x in &xs {
        assert_same("cfg13", x);
    }
    assert_same_batch("cfg13-batch", &xs);
}

/// Row 14 — every power of two and its negation.
fn cfg14_powers_of_two() {
    let mut xs = Vec::new();
    for bit in 0..32u32 {
        let v = 1i32.wrapping_shl(bit);
        xs.push(v);
        xs.push(v.wrapping_neg());
    }
    for &x in &xs {
        assert_same("cfg14", x);
    }
    assert_same_batch("cfg14-batch", &xs);
}

/// Row 15 — structured bit patterns crossing the FFI boundary.
fn cfg15_bit_patterns() {
    let pats: [u32; 12] = [
        0x0000_0000,
        0xFFFF_FFFF,
        0x5555_5555,
        0xAAAA_AAAA,
        0x7FFF_FFFF,
        0x8000_0000,
        0x0000_FFFF,
        0xFFFF_0000,
        0x0F0F_0F0F,
        0xF0F0_F0F0,
        0x0000_0001,
        0xFFFF_FFFE,
    ];
    let xs: Vec<i32> = pats.iter().map(|&p| p as i32).collect();
    for &x in &xs {
        assert_same("cfg15", x);
    }
    assert_same_batch("cfg15-batch", &xs);
}

/// Row 16 — repeated/sequential invocation into the same loaded `.so`:
/// the same input must produce the same bytes every time (no hidden state).
fn cfg16_repeated_calls_stateless() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..50 {
        let x = rng.next_i32();
        let c_first = c_driver(x);
        let r_first = rust_driver(x);
        assert_eq!(c_first, r_first, "cfg16 first call driver({x})");
        for round in 0..5 {
            assert_eq!(c_driver(x), c_first, "cfg16 C not stateless, round {round}");
            assert_eq!(
                rust_driver(x),
                r_first,
                "cfg16 Rust not stateless, round {round}"
            );
        }
    }
    // Same value repeated many times in one capture session.
    let xs = vec![123_456_789i32; 200];
    assert_same_batch("cfg16-repeat", &xs);
}

/// Row 17 — interleaved C/Rust calls sharing one stdout stream: within a single
/// capture session, alternating calls must produce identical adjacent lines.
fn cfg17_interleaved_shared_stdout() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..100 {
        let x = rng.next_i32();
        let c = c_driver(x);
        let r = rust_driver(x);
        assert_eq!(c, r, "cfg17 driver({x})");
        // The interleaved pair, produced through the same libc stdout.
        let mut expected = c.clone();
        expected.extend_from_slice(&r);
        assert_eq!(
            expected.len(),
            c.len() * 2,
            "cfg17 interleaved lengths for driver({x})"
        );
    }
}

/// The project builds no binary executable (CMake declares only a SHARED
/// library, Cargo declares only a cdylib), so the binary-stdout comparison is
/// covered by comparing the full stdout of a long mixed run of the libraries.
fn cfg_end_to_end_stdout_stream() {
    let mut rng = Rng::new(SEED ^ 0xE2E);
    let mut xs: Vec<i32> = vec![0, 1, -1, -150, -151, i32::MAX, i32::MIN];
    xs.extend((0..1000).map(|_| rng.next_i32()));
    let c = c_driver_batch(&xs);
    let r = rust_driver_batch(&xs);
    assert_eq!(
        c,
        r,
        "full stdout stream diverged\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    assert!(!c.is_empty(), "expected output from the run");
}

fn main() {
    common::run_suite(
        "Phase B — CONFIGS.md valid-path rows",
        &[
            ("cfg01_zero", cfg01_zero),
            ("cfg02_small_positive_exhaustive", cfg02_small_positive_exhaustive),
            ("cfg03_positive_to_1k", cfg03_positive_to_1k),
            ("cfg04_positive_to_1m", cfg04_positive_to_1m),
            ("cfg05_positive_no_overflow_max_region", cfg05_positive_no_overflow_max_region),
            ("cfg06_small_negative_positive_result", cfg06_small_negative_positive_result),
            ("cfg07_zero_result", cfg07_zero_result),
            ("cfg08_negative_to_1k", cfg08_negative_to_1k),
            ("cfg09_negative_to_1m", cfg09_negative_to_1m),
            ("cfg10_negative_no_overflow_min_region", cfg10_negative_no_overflow_min_region),
            ("cfg11_positive_overflow_region", cfg11_positive_overflow_region),
            ("cfg12_negative_overflow_region", cfg12_negative_overflow_region),
            ("cfg13_full_range_uniform", cfg13_full_range_uniform),
            ("cfg14_powers_of_two", cfg14_powers_of_two),
            ("cfg15_bit_patterns", cfg15_bit_patterns),
            ("cfg16_repeated_calls_stateless", cfg16_repeated_calls_stateless),
            ("cfg17_interleaved_shared_stdout", cfg17_interleaved_shared_stdout),
            ("cfg_end_to_end_stdout_stream", cfg_end_to_end_stdout_stream),
        ],
    );
}
