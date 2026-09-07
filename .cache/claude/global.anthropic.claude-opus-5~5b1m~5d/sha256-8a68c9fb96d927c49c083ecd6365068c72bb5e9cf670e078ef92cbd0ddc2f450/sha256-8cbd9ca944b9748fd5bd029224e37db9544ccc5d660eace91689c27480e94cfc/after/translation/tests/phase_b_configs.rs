//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` with `libloading` and
//! calls the exported `driver` symbol through the FFI boundary, comparing the
//! captured stdout byte-for-byte. Rows that say "randomized" use a fixed seed.

mod common;

use common::*;
use std::ffi::c_int;

/// Reference model of the C loop, used only to double-check that the *shared*
/// output is the genuinely expected text (not just "both sides agree").
fn expected(x: c_int) -> String {
    let mut s = String::new();
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        s.push_str(&format!("{i} {j}\n"));
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
    s
}

#[test]
fn both_libraries_load_and_are_distinct() {
    assert_libs_loaded();
    assert_ne!(c_driver() as usize, rust_driver() as usize);
}

/// Row 1: `x == 0` — zero iterations, empty output.
#[test]
fn cfg_row01_zero_iterations() {
    assert_same_and_empty(0);
}

/// Row 2: randomized negative `x` over `INT_MIN..0` — zero iterations.
#[test]
fn cfg_row02_random_negative() {
    let mut rng = Rng::new(0xC0FF_EE01);
    for _ in 0..200 {
        let x = -(rng.range(1, i32::MAX as i64) as i32);
        assert_same_and_empty(x);
    }
    // plus the extremes of the negative half-range
    for x in [-1, -2, i32::MIN, i32::MIN + 1, i32::MIN / 2] {
        assert_same_and_empty(x);
    }
}

/// Row 3: `x == 1` — one line, 1-digit `i` and `j`.
#[test]
fn cfg_row03_single_iteration() {
    assert_same_and_eq(1, "0 0\n");
}

/// Row 4: `x == 2` — two lines.
#[test]
fn cfg_row04_two_iterations() {
    assert_same_and_eq(2, "0 0\n1 2\n");
}

/// Row 5: `x == 6` — `i` stays 1 digit while `j` crosses into 2 digits.
#[test]
fn cfg_row05_j_width_crosses_first() {
    assert_same_and_eq(6, "0 0\n1 2\n2 4\n3 6\n4 8\n5 10\n");
}

/// Row 6: `x == 60` — `i` crosses 1→2 digits, `j` crosses 1→2→3 digits.
#[test]
fn cfg_row06_i_and_j_cross_decades() {
    assert_same(60);
    assert_same_and_eq(60, &expected(60));
}

/// Row 7: `x == 600` — `i` reaches 3 digits, `j` reaches 4 digits.
#[test]
fn cfg_row07_three_and_four_digits() {
    assert_same_and_eq(600, &expected(600));
}

/// Row 8: exact decade boundaries for both `i` and `j = 2*i`.
#[test]
fn cfg_row08_decade_boundaries() {
    for x in [
        5, 6, 7, 9, 10, 11, 49, 50, 51, 99, 100, 101, 499, 500, 501, 999, 1000, 1001, 4999, 5000,
        5001,
    ] {
        assert_same_and_eq(x, &expected(x));
    }
}

/// Row 9: randomized small counts `1..=64`.
#[test]
fn cfg_row09_random_small() {
    let mut rng = Rng::new(0x5EED_0009);
    for _ in 0..150 {
        let x = rng.range(1, 64) as i32;
        assert_same_and_eq(x, &expected(x));
    }
}

/// Row 10: randomized larger counts `1..=5000`; output spans many stdio
/// buffer flushes.
#[test]
fn cfg_row10_random_large_buffered() {
    let mut rng = Rng::new(0x5EED_0010);
    for _ in 0..40 {
        let x = rng.range(1, 5000) as i32;
        assert_same(x);
    }
}

/// Row 11: randomized *sequences* of calls mixing rejected (`x <= 0`) and
/// accepted (`x > 0`) inputs, captured as one stdout stream.
#[test]
fn cfg_row11_random_call_sequences() {
    let mut rng = Rng::new(0x5EED_0011);
    for _ in 0..40 {
        let len = rng.range(1, 8) as usize;
        let mut xs: Vec<c_int> = Vec::with_capacity(len);
        for _ in 0..len {
            let x = match rng.range(0, 3) {
                0 => 0,
                1 => -(rng.range(1, 1000) as i32),
                2 => rng.range(1, 12) as i32,
                _ => rng.range(1, 300) as i32,
            };
            xs.push(x);
        }
        assert_same_seq(&xs);

        // The concatenation must also match the per-call reference model.
        let want: String = xs.iter().map(|&x| expected(x)).collect();
        let got = c_output_seq(&xs);
        assert_eq!(String::from_utf8_lossy(&got), want, "sequence {xs:?}");
    }
}

/// Row 12: largest bounded volume — ~200 KB of output, many buffer refills.
#[test]
fn cfg_row12_large_volume() {
    let out = {
        assert_same(20000);
        c_output(20000)
    };
    assert!(out.len() > 100_000, "expected a large capture, got {} bytes", out.len());
    assert_eq!(String::from_utf8_lossy(&out), expected(20000));
}

/// Row 13: randomized `x` drawn from the *entire* `c_int` domain. Positives are
/// clamped so the test terminates, but the draw itself covers the real input
/// space (huge positives, huge negatives, zero).
#[test]
fn cfg_row13_random_full_int_domain() {
    let mut rng = Rng::new(0x5EED_0013);
    for _ in 0..200 {
        let raw = rng.next_i32();
        // Keep the value verbatim when it cannot blow up the runtime.
        let x = if raw > 4096 { raw % 4096 } else { raw };
        assert_same(x);
    }
}
