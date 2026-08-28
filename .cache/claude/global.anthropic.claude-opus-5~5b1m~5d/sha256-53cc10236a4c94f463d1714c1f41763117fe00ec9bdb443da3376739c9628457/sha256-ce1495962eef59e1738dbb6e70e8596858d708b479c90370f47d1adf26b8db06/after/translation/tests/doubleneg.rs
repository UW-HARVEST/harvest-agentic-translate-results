//! Phase B — `doubleneg` differential tests (CONFIGS rows 25-32).
//!
//! `doubleneg` is the library's headline entry point and produces most of its
//! observable behaviour through ~40 `printf` calls. These tests therefore
//! compare BOTH the returned `int` AND the complete stdout byte stream.
//!
//! stdout capture redirects the process's fd 1, which is process-global state.
//! This binary therefore sets `harness = false` (see Cargo.toml) and runs its
//! tests sequentially through `common::run_sequentially`, reporting progress on
//! stderr -- otherwise libtest's own "test foo ... ok" lines, written to fd 1
//! from other threads, would land inside a capture and corrupt the comparison.

mod common;

use common::assert_doubleneg_matches;
use common::both;
use common::capture_stdout;
use common::Rng;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Sanity/negative control: the harness must actually observe output, and the C
/// library must really be the one producing it.
fn cfg_00_capture_harness_has_teeth() {
    let (c, r) = both();

    let (ret, out) = capture_stdout(|| unsafe { (c.doubleneg)(1, 2, 3, 4) });
    assert!(
        out.len() > 500,
        "expected substantial captured output, got {} bytes",
        out.len()
    );
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("=== Starting foo() execution ==="), "{text}");
    assert!(text.contains("Parameters: 1, 2, 3, 4"), "{text}");
    assert!(text.contains("=== Final Result ==="), "{text}");
    assert!(
        text.contains(&format!("Accumulated result: {ret}")),
        "return value {ret} not echoed in output:\n{text}"
    );
    // The C code converts `-1.0 * pow(2.0, 40)` with `cvttsd2si`, so the
    // out-of-range answer must be INT_MIN. If this ever prints something else,
    // the whole `cvt` emulation premise is wrong and every other test is void.
    assert!(
        text.contains("Converted to int (UB likely): -2147483648"),
        "C library did not produce the expected cvttsd2si indefinite value:\n{text}"
    );
    assert!(
        text.contains("Converting INFINITY to int: -2147483648 (undefined behavior)"),
        "{text}"
    );
    assert!(
        text.contains("Converting NAN to int: -2147483648 (undefined behavior)"),
        "{text}"
    );

    // The two libraries must be distinct objects (not the same .so loaded twice).
    let cp = c.doubleneg as usize;
    let rp = r.doubleneg as usize;
    assert_ne!(cp, rp, "C and Rust `doubleneg` resolved to the same address");
}

/// CONFIGS row 25 — all 16 zero/non-zero combinations, small representatives.
fn cfg_25_doubleneg_truthiness_16() {
    for mask in 0..16u32 {
        let p = |bit: u32| if mask & (1 << bit) != 0 { 1 } else { 0 };
        assert_doubleneg_matches(p(0), p(1), p(2), p(3));
    }
}

/// CONFIGS row 26 — the same 16 combinations with large non-zero values.
fn cfg_26_doubleneg_truthiness_large() {
    for mask in 0..16u32 {
        let p = |bit: u32| if mask & (1 << bit) != 0 { 123_456 } else { 0 };
        assert_doubleneg_matches(p(0), p(1), p(2), p(3));
    }
    for mask in 0..16u32 {
        let p = |bit: u32| if mask & (1 << bit) != 0 { -987_654 } else { 0 };
        assert_doubleneg_matches(p(0), p(1), p(2), p(3));
    }
}

/// CONFIGS row 27 — `param1` drives the `create_numeric_buffer` seed, so this
/// sweep walks every possible generated-buffer rotation (all 256 residues of
/// `param1 mod 256`, plus values either side).
fn cfg_27_doubleneg_seed_sweep() {
    for p1 in -8..=264 {
        assert_doubleneg_matches(p1, 3, 4, 5);
    }
}

/// CONFIGS row 28 — `param2` drives `search_values[0]`, the `i*param2` stride in
/// the combined-feature loop, and the `b` divisor (including `b == 0`).
fn cfg_28_doubleneg_param2_sweep() {
    let mut p2 = -300;
    while p2 <= 300 {
        assert_doubleneg_matches(7, p2, 4, 5);
        p2 += 17;
    }
    // Exact divisor edge cases.
    for p2 in [-1, 0, 1, 256, -256, 255, -255] {
        assert_doubleneg_matches(7, p2, 4, 5);
        assert_doubleneg_matches(0, p2, 0, 0);
    }
}

/// CONFIGS row 29 — `param3` drives `c % 10` (every exponent) and
/// `search_values[1]`.
fn cfg_29_doubleneg_param3_sweep() {
    for p3 in -25..=25 {
        assert_doubleneg_matches(7, 3, p3, 5);
        assert_doubleneg_matches(-7, 3, p3, 5);
        assert_doubleneg_matches(0, 3, p3, 5);
    }
}

/// CONFIGS row 30 — `param4` only feeds `search_values[2]` and `!!param4`.
fn cfg_30_doubleneg_param4_sweep() {
    let mut p4 = -300;
    while p4 <= 300 {
        assert_doubleneg_matches(7, 3, 4, p4);
        p4 += 13;
    }
}

/// CONFIGS row 31 — extremes cross-product.
fn cfg_31_doubleneg_extremes_cross() {
    let vals = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    assert_doubleneg_matches(a, b, c, d);
                }
            }
        }
    }
}

/// CONFIGS row 32 — fully random parameters over the whole `i32` range.
fn cfg_32_doubleneg_random() {
    let mut rng = Rng::new(SEED ^ 32);
    for i in 0..2_000 {
        let (a, b, c, d) = match i % 4 {
            // Mix magnitudes so small/large/negative all appear.
            0 => (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()),
            1 => (
                rng.next_i32() % 512,
                rng.next_i32() % 512,
                rng.next_i32() % 32,
                rng.next_i32() % 512,
            ),
            2 => (
                rng.next_i32() & 0xFF,
                rng.next_i32() & 0xFF,
                rng.next_i32() & 0xF,
                rng.next_i32() & 0xFF,
            ),
            _ => (
                rng.next_i32() >> 1,
                rng.next_i32() >> 2,
                rng.next_i32() >> 3,
                rng.next_i32() >> 4,
            ),
        };
        assert_doubleneg_matches(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Sequential entry point (`harness = false`; see Cargo.toml for why).
// ---------------------------------------------------------------------------
fn main() {
    common::run_sequentially(
        "doubleneg",
        &[
            ("cfg_00_capture_harness_has_teeth", cfg_00_capture_harness_has_teeth as fn()),
            ("cfg_25_doubleneg_truthiness_16", cfg_25_doubleneg_truthiness_16 as fn()),
            ("cfg_26_doubleneg_truthiness_large", cfg_26_doubleneg_truthiness_large as fn()),
            ("cfg_27_doubleneg_seed_sweep", cfg_27_doubleneg_seed_sweep as fn()),
            ("cfg_28_doubleneg_param2_sweep", cfg_28_doubleneg_param2_sweep as fn()),
            ("cfg_29_doubleneg_param3_sweep", cfg_29_doubleneg_param3_sweep as fn()),
            ("cfg_30_doubleneg_param4_sweep", cfg_30_doubleneg_param4_sweep as fn()),
            ("cfg_31_doubleneg_extremes_cross", cfg_31_doubleneg_extremes_cross as fn()),
            ("cfg_32_doubleneg_random", cfg_32_doubleneg_random as fn()),
        ],
    );
}
