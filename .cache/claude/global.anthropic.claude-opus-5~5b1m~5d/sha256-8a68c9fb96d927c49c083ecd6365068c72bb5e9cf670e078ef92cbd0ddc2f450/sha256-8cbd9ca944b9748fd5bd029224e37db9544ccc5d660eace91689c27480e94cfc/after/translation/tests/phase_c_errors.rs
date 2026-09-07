//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! `driver` returns `void` and the C code contains no `return`, `assert`,
//! null check, range check or error enum (see `ERRORS.md`), so its rejection
//! surface is empty: every input is "accepted" and non-positive inputs simply
//! produce zero iterations. The strongest available assertion is therefore that
//! C and Rust agree on the *observable* result — the exact stdout bytes and a
//! normal return — for each boundary condition a caller can construct across
//! the FFI boundary.

mod common;

use common::*;
use std::ffi::c_int;

/// Row 1: `x == 0` — zero "length".
#[test]
fn err_row01_zero_length() {
    assert_same_and_empty(0);
}

/// Row 2: `x == -1` — one step past the valid (`> 0`) range.
#[test]
fn err_row02_negative_one() {
    assert_same_and_empty(-1);
}

/// Row 3: `x == INT_MIN` — most-negative int.
#[test]
fn err_row03_int_min() {
    assert_same_and_empty(i32::MIN as c_int);
}

/// Row 4: `x == INT_MIN + 1`.
#[test]
fn err_row04_int_min_plus_one() {
    assert_same_and_empty((i32::MIN + 1) as c_int);
}

/// Row 5: randomized negatives across the whole rejected half-range.
#[test]
fn err_row05_random_negatives() {
    let mut rng = Rng::new(0xBAD_5EED_5);
    for _ in 0..300 {
        let x = rng.range(i32::MIN as i64, -1) as i32;
        assert_same_and_empty(x);
    }
}

/// Row 6: out-of-range "enum-like" bit patterns passed as `int`. A C enum
/// parameter accepts any `int`, so these are real inputs; `driver` performs no
/// validation at all, and only the sign matters.
#[test]
fn err_row06_out_of_range_enum_bits() {
    // Negative bit patterns: must be silent no-ops in both implementations.
    for bits in [0x8000_0000u32, 0xFFFF_FFFFu32, 0xDEAD_BEEFu32, 0x8000_0001u32] {
        let x = bits as i32;
        assert!(x < 0, "sanity: {bits:#x} is negative as i32");
        assert_same_and_empty(x);
    }

    // INT_MAX as a bit pattern is positive: the C code accepts it and loops.
    // Running 2^31 iterations is not feasible, so assert on the largest
    // positive patterns that terminate, plus the shape of the value itself.
    assert_eq!(0x7FFF_FFFFu32 as i32, i32::MAX);
    for bits in [0x0000_07FFu32, 0x0000_0FFFu32, 0x0000_1FFFu32] {
        assert_same(bits as i32);
    }
}

/// Row 7: interleaved invalid and valid calls — no state to corrupt, no
/// cumulative error; the invalid calls must not perturb the valid ones.
#[test]
fn err_row07_interleaved_invalid_calls() {
    let seqs: &[&[c_int]] = &[
        &[0, 3, 0],
        &[-1, 3, -1],
        &[i32::MIN, 5, 0, -7, 2],
        &[0, 0, 0],
        &[-1, -1, -1],
        &[7, i32::MIN, 7],
    ];
    for xs in seqs {
        assert_same_seq(xs);
    }

    // An all-invalid sequence must produce no output at all.
    let empty = c_output_seq(&[0, -1, i32::MIN, -12345]);
    assert!(empty.is_empty(), "invalid-only sequence printed {empty:?}");
    let empty_rs = rust_output_seq(&[0, -1, i32::MIN, -12345]);
    assert_eq!(empty, empty_rs);

    // Interleaving must equal the concatenation of the individual valid calls.
    let xs: &[c_int] = &[0, 3, -1, 4, i32::MIN, 1];
    let joined = c_output_seq(xs);
    let mut per_call = Vec::new();
    for &x in xs {
        per_call.extend_from_slice(&c_output(x));
    }
    assert_eq!(joined, per_call, "C output is not call-independent");
    assert_eq!(joined, rust_output_seq(xs));
}

/// Row 8: `x == 1` — the smallest value that is *not* rejected, i.e. one step
/// past the rejected range.
#[test]
fn err_row08_smallest_accepted() {
    assert_same_and_eq(1, "0 0\n");
    // and the transition across the boundary in both directions
    assert_same_and_empty(0);
    assert_same_and_eq(1, "0 0\n");
    assert_same_and_eq(2, "0 0\n1 2\n");
}

/// Generic FFI boundary sweep: every value in a window straddling the
/// accept/reject boundary, plus the extremes of the type.
#[test]
fn err_generic_boundary_sweep() {
    for x in -32i32..=32 {
        assert_same(x);
    }
    for x in [i32::MIN, i32::MIN + 1, -1, 0, 1] {
        assert_same(x);
    }
}
