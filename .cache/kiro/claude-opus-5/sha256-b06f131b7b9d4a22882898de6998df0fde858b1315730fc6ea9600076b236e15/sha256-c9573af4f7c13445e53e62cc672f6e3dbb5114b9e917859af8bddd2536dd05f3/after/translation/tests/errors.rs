//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row. Each asserts C and Rust return the *same specific* value (the sentinel
//! the C actually produces), not merely that both "failed somehow".

mod common;

use common::{sweep, Pair, Rng, I32_MAX, I32_MIN};

// ---------------------------------------------------------------------------
// Rows 1-5: the `v2 == 0` divide-by-zero guard -> sentinel 0
// ---------------------------------------------------------------------------

#[test]
fn err01_div_by_zero_positive_numerator() {
    let p = Pair::load();
    p.check_eq(7, 0, 0);
    p.check_eq(1, 0, 0);
    // randomized across the whole positive numerator range
    sweep(&p, 100_000, |r| (r.pos(), 0));
    let mut r = Rng::fixed();
    for _ in 0..100_000 {
        let v1 = r.pos();
        p.check_eq(v1, 0, 0);
    }
}

#[test]
fn err02_div_by_zero_zero_numerator() {
    Pair::load().check_eq(0, 0, 0);
}

#[test]
fn err03_div_by_zero_negative_numerator() {
    let p = Pair::load();
    p.check_eq(-7, 0, 0);
    p.check_eq(-1, 0, 0);
    let mut r = Rng::fixed();
    for _ in 0..100_000 {
        let v1 = r.neg_non_min();
        p.check_eq(v1, 0, 0);
    }
}

#[test]
fn err04_div_by_zero_int_min_numerator() {
    // The guard runs before the `v1 != INT_MIN` check.
    Pair::load().check_eq(I32_MIN, 0, 0);
}

#[test]
fn err05_div_by_zero_int_max_numerator() {
    Pair::load().check_eq(I32_MAX, 0, 0);
}

// ---------------------------------------------------------------------------
// Rows 6-7: v1 >= 0, v2 == INT_MIN  -> q = 0, r = v1 >= 0  -> 0
// ---------------------------------------------------------------------------

#[test]
fn err06_nonneg_over_int_min_returns_zero() {
    let p = Pair::load();
    p.check_eq(1, I32_MIN, 0);
    p.check_eq(2, I32_MIN, 0);
    p.check_eq(I32_MAX, I32_MIN, 0);
    p.check_eq(I32_MAX - 1, I32_MIN, 0);
    let mut r = Rng::fixed();
    for _ in 0..200_000 {
        let v1 = r.range(0, I32_MAX);
        p.check_eq(v1, I32_MIN, 0);
    }
}

#[test]
fn err07_zero_over_int_min_returns_zero() {
    Pair::load().check_eq(0, I32_MIN, 0);
}

// ---------------------------------------------------------------------------
// Rows 8-10: INT_MIN < v1 < 0, v2 == INT_MIN -> q = 1, r = v1 - INT_MIN > 0 -> 1
// ---------------------------------------------------------------------------

#[test]
fn err08_neg_over_int_min_returns_one() {
    let p = Pair::load();
    p.check_eq(-7, I32_MIN, 1);
    p.check_eq(-100, I32_MIN, 1);
    let mut r = Rng::fixed();
    for _ in 0..200_000 {
        let v1 = r.neg_non_min();
        p.check_eq(v1, I32_MIN, 1);
    }
}

#[test]
fn err09_minus_one_over_int_min_returns_one() {
    Pair::load().check_eq(-1, I32_MIN, 1);
}

#[test]
fn err10_min_plus_one_over_int_min_returns_one() {
    Pair::load().check_eq(I32_MIN + 1, I32_MIN, 1);
}

// ---------------------------------------------------------------------------
// Row 11: INT_MIN / INT_MIN -> q = 1, r = 0 -> 1
// ---------------------------------------------------------------------------

#[test]
fn err11_int_min_over_int_min_returns_one() {
    Pair::load().check_eq(I32_MIN, I32_MIN, 1);
}

// ---------------------------------------------------------------------------
// Row 12: v1 == INT_MIN, v2 > 0 - rewritten `-(v1 + v2)` path
// ---------------------------------------------------------------------------

#[test]
fn err12_int_min_over_positive_guarded_path() {
    let p = Pair::load();
    // Values documented in ERRORS.md row 12.
    p.check_eq(I32_MIN, 1, I32_MIN);
    p.check_eq(I32_MIN, 2, -1_073_741_824);
    p.check_eq(I32_MIN, 3, -715_827_883);
    p.check_eq(I32_MIN, I32_MAX, -2);
    let mut r = Rng::fixed();
    for _ in 0..200_000 {
        let v2 = r.pos();
        p.check(I32_MIN, v2);
    }
    for v2 in 1..=8192 {
        p.check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// Row 13: v1 == INT_MIN, INT_MIN < v2 < 0 - rewritten `-(v1 - v2)` path.
// v2 == -1 makes `q = INT_MAX + 1` overflow in the C; the compiled C `.so` is
// the ground truth and the Rust must agree with it exactly.
// ---------------------------------------------------------------------------

#[test]
fn err13_int_min_over_negative_guarded_path() {
    let p = Pair::load();
    p.check_eq(I32_MIN, -2, 1_073_741_824);
    p.check_eq(I32_MIN, I32_MIN + 1, 2);
    // The C-overflow case: assert agreement, and record the observed value.
    let observed = p.agreed(I32_MIN, -1);
    assert_eq!(
        observed, I32_MIN,
        "compiled C .so yielded {observed} for div_euclid(INT_MIN, -1); \
         ERRORS.md row 13 recorded INT_MIN - update the table and the Rust to match"
    );
    let mut r = Rng::fixed();
    for _ in 0..200_000 {
        let v2 = r.neg_non_min();
        p.check(I32_MIN, v2);
    }
    for v2 in -8192..=-1 {
        p.check(I32_MIN, v2);
    }
}

// ---------------------------------------------------------------------------
// Row 14: the trailing `if (r < 0)` correction branch
// ---------------------------------------------------------------------------

#[test]
fn err14_remainder_correction_branch() {
    let p = Pair::load();
    // v1 < 0, v2 > 0 -> q - 1
    p.check_eq(-7, 2, -4);
    p.check_eq(-1, 2, -1);
    p.check_eq(-1, I32_MAX, -1);
    // v1 < 0, v2 < 0 -> q + 1
    p.check_eq(-7, -2, 4);
    p.check_eq(-1, -2, 1);
    p.check_eq(-1, I32_MIN + 1, 1);
    // v1 == INT_MIN with a non-exact divisor
    p.check_eq(I32_MIN, 3, -715_827_883);
    p.check_eq(I32_MIN, -3, 715_827_883);
    // randomized: force a non-zero remainder on both sign combinations
    let mut r = Rng::fixed();
    for _ in 0..200_000 {
        let m = r.range(2, 1 << 15);
        let rem = r.range(1, m - 1);
        let k = r.range(1, (I32_MAX - rem) / m);
        let v1 = -(k * m + rem);
        p.check(v1, m);
        p.check(v1, -m);
    }
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries.
//
// `div_euclid` takes two by-value `int`s: there is no pointer, length, or enum
// parameter, so the null-pointer / zero-length / oversized-length / invalid-enum
// classes have no applicable argument. The faithful generalisation is that every
// 32-bit pattern is a legal argument, including the ones just past each guard
// constant, so those are exercised exhaustively here.
// ---------------------------------------------------------------------------

#[test]
fn generic_one_step_past_every_guard_constant() {
    let p = Pair::load();
    let interesting = [
        I32_MIN,
        I32_MIN + 1,
        I32_MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        I32_MAX - 2,
        I32_MAX - 1,
        I32_MAX,
    ];
    for &v1 in &interesting {
        for &v2 in &interesting {
            p.check(v1, v2);
        }
    }
}

/// The nearest analogue of "out-of-range enum across the FFI boundary": pass raw
/// bit patterns with no distinguished meaning, reinterpreted as `int`, straight
/// through the ABI. Both `.so`s must agree on every one.
#[test]
fn generic_arbitrary_bit_patterns_across_ffi() {
    let p = Pair::load();
    let mut pats: Vec<i32> = Vec::new();
    for bit in 0..32u32 {
        let m = 1i64 << bit;
        pats.push(m as i32);
        pats.push((m - 1) as i32);
        pats.push(!(m as i32));
        pats.push((m as i32).wrapping_neg());
    }
    for pat in [0x5555_5555u32, 0xAAAA_AAAA, 0xFFFF_FFFF, 0x8000_0000, 0x7FFF_FFFF] {
        pats.push(pat as i32);
    }
    pats.sort();
    pats.dedup();
    for &v1 in &pats {
        for &v2 in &pats {
            p.check(v1, v2);
        }
    }
}

/// Repeated / interleaved calls: confirms neither `.so` carries hidden state
/// that would make the sequence of calls matter.
#[test]
fn generic_no_hidden_state_across_calls() {
    let p = Pair::load();
    let seq = [
        (0, 0),
        (I32_MIN, -1),
        (7, -2),
        (I32_MIN, I32_MIN),
        (-7, 2),
        (I32_MAX, 0),
        (I32_MIN, 3),
        (0, I32_MIN),
    ];
    for _ in 0..1000 {
        for &(a, b) in &seq {
            p.check(a, b);
        }
        for &(a, b) in seq.iter().rev() {
            p.check(a, b);
        }
    }
}
