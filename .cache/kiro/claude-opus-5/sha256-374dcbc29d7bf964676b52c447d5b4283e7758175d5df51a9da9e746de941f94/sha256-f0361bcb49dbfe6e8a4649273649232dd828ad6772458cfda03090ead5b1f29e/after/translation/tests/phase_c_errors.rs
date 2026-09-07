//! Phase C — error-path / boundary differential tests.
//!
//! `ERRORS.md` establishes that the C library has **no** rejection path: no error
//! return, no sentinel, no assert, no range check, no null check, no enum, no
//! length parameter. The rows below therefore verify the *generic* boundaries
//! every C API has, and in particular that C and Rust agree on ACCEPTING them
//! (identical return value / stdout) rather than one of them trapping,
//! saturating, or panicking.
//!
//! Each `err_*` test corresponds to one row E1..E11 of `ERRORS.md`.

mod common;

use common::{harness, Model};

/// Moves the shared accumulator to exactly `target` in both libraries.
fn drive_to(h: &mut common::Harness, target: i32) {
    let cur = h.peek();
    let delta = target.wrapping_sub(cur);
    assert_eq!(h.static_sum(delta), target);
}

// E1 — update = 0 (the zero-length / identity analogue).
#[test]
fn err_e1_static_sum_zero() {
    let mut h = harness();
    let before = h.peek();
    // Repeated zero updates must be a no-op in both, not an error.
    for _ in 0..5 {
        assert_eq!(h.static_sum(0), before);
    }
}

// E2 — update = INT_MAX.
#[test]
fn err_e2_static_sum_int_max() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut m = Model::new(0);
    assert_eq!(h.static_sum(i32::MAX), m.static_sum(i32::MAX));
    assert_eq!(h.peek(), i32::MAX, "no saturation, no rejection");
}

// E3 — update = INT_MIN (one step past -INT_MAX).
#[test]
fn err_e3_static_sum_int_min() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut m = Model::new(0);
    assert_eq!(h.static_sum(i32::MIN), m.static_sum(i32::MIN));
    assert_eq!(h.peek(), i32::MIN);
    // And one step past, in the negative direction.
    assert_eq!(h.static_sum(-1), m.static_sum(-1));
    assert_eq!(h.peek(), i32::MAX, "INT_MIN - 1 wraps to INT_MAX in both");
}

// E4 — accumulator driven past INT_MAX.
#[test]
fn err_e4_static_sum_overflow_positive() {
    let mut h = harness();
    drive_to(&mut h, i32::MAX);
    let mut m = Model::new(i32::MAX);
    for update in [1, 1, i32::MAX, 2, 100] {
        assert_eq!(
            h.static_sum(update),
            m.static_sum(update),
            "positive overflow by {update} diverged"
        );
    }
}

// E5 — accumulator driven past INT_MIN.
#[test]
fn err_e5_static_sum_overflow_negative() {
    let mut h = harness();
    drive_to(&mut h, i32::MIN);
    let mut m = Model::new(i32::MIN);
    for update in [-1, -1, i32::MIN, -2, -100] {
        assert_eq!(
            h.static_sum(update),
            m.static_sum(update),
            "negative overflow by {update} diverged"
        );
    }
}

// E6 — driver(0).
#[test]
fn err_e6_driver_zero_stride() {
    let mut h = harness();
    drive_to(&mut h, -7);
    let out = h.driver(0);
    assert_eq!(out, b"-7\n-7\n-7\n-7\n-7\n-7\n-7\n-7\n-7\n-7\n");
    assert_eq!(h.peek(), -7);
}

// E7 — driver(INT_MAX): `i * stride` overflows from i = 2 onwards.
#[test]
fn err_e7_driver_int_max() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut m = Model::new(0);
    assert_eq!(h.driver(i32::MAX), m.driver(i32::MAX));
    assert_eq!(h.peek(), m.sum);
}

// E8 — driver(INT_MIN).
#[test]
fn err_e8_driver_int_min() {
    let mut h = harness();
    drive_to(&mut h, 0);
    let mut m = Model::new(0);
    assert_eq!(h.driver(i32::MIN), m.driver(i32::MIN));
    assert_eq!(h.peek(), m.sum);
}

// E9 — one step past the largest stride for which no product overflows.
#[test]
fn err_e9_driver_one_past_nonoverflow_range() {
    let mut h = harness();
    let ok = i32::MAX / 9;
    for stride in [ok, ok + 1, -ok, -ok - 1, i32::MIN / 9, i32::MIN / 9 - 1] {
        let before = h.peek();
        let mut m = Model::new(before);
        assert_eq!(h.driver(stride), m.driver(stride), "driver({stride}) diverged");
        assert_eq!(h.peek(), m.sum);
    }
}

// E10 — arbitrary bit patterns reinterpreted as `int` (the out-of-range-enum
// analogue: C accepts any int for any parameter, so must Rust).
#[test]
fn err_e10_arbitrary_bit_patterns() {
    let mut h = harness();
    let patterns: [u32; 10] = [
        0x0000_0000,
        0x0000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x8000_0001,
        0xAAAA_AAAA,
    ];
    let mut m = Model::new(h.peek());
    for p in patterns {
        let v = p as i32;
        assert_eq!(h.static_sum(v), m.static_sum(v), "static_sum(0x{p:08X}) diverged");
        assert_eq!(h.driver(v), m.driver(v), "driver(0x{p:08X}) diverged");
    }
}

// E11 — dirty high 32 bits in the argument register: only the low 32 bits are
// part of the ABI, so both libraries must agree on the truncated value.
#[test]
fn err_e11_dirty_high_bits() {
    let mut h = harness();
    let wide: [i64; 6] = [
        0x0000_0001_0000_0007,
        -0x0000_0001_0000_0007,
        0x7FFF_FFFF_FFFF_FFFF,
        i64::MIN,
        0xFFFF_FFFF_0000_0000u64 as i64,
        0x1234_5678_9ABC_DEF0u64 as i64,
    ];
    let mut m = Model::new(h.peek());
    for w in wide {
        let v = w as i32; // what the C ABI actually delivers
        assert_eq!(h.static_sum(v), m.static_sum(v), "static_sum({w} as i32) diverged");
        assert_eq!(h.driver(v), m.driver(v), "driver({w} as i32) diverged");
    }
}

// Generic boundary sweep beyond the table: every value one step around each
// interesting point, for both entry points.
#[test]
fn err_generic_boundary_sweep() {
    let mut h = harness();
    let mut m = Model::new(h.peek());
    let points = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
        i32::MAX / 9 - 1,
        i32::MAX / 9,
        i32::MAX / 9 + 1,
        i32::MIN / 9 - 1,
        i32::MIN / 9,
        i32::MIN / 9 + 1,
    ];
    for v in points {
        assert_eq!(h.static_sum(v), m.static_sum(v), "static_sum({v}) diverged");
        assert_eq!(h.driver(v), m.driver(v), "driver({v}) diverged");
        assert_eq!(h.peek(), m.sum, "state after {v} diverged");
    }
}
