//! Final exhaustive layer over the highest-risk value windows.
//!
//! Rationale: `memchra2`'s return value depends on its arguments only through
//! (1) their decimal renderings in the `snprintf` buffer, (2) the wrapping sum,
//! (3) the low byte of each, and (4) the `int`->`float` pun of `a`. These tests
//! close (3) and (4) exhaustively and sweep (1)/(2) over full 2^24 ranges.

mod common;
use common::check;

/// EXHAUSTIVE over the entire float-accept window `1.0 <= f < 1000.0`
/// (`a` in `0x3F800000 ..= 0x4479FFFF`, ~16.4M values) — the only window where
/// `result += (int)f` fires, so the only place a truncation/rounding bug could
/// hide.
#[test]
fn exhaustive_float_accept_window() {
    let lo = 1.0f32.to_bits();
    let hi = 1000.0f32.to_bits() - 1;
    for bits in lo..=hi {
        check(bits as i32, 1, -2, 3);
    }
}

/// EXHAUSTIVE over `b` in `0 ..< 2^24` (decimal widths 1-8, all low bytes).
#[test]
fn exhaustive_b_low24() {
    for b in 0..(1i64 << 24) {
        check(7, b as i32, -5, 9);
    }
}

/// EXHAUSTIVE over `c` in `0 ..< 2^24`.
#[test]
fn exhaustive_c_low24() {
    for c in 0..(1i64 << 24) {
        check(-7, 5, c as i32, -9);
    }
}

/// EXHAUSTIVE over `d` in `0 ..< 2^24`.
#[test]
fn exhaustive_d_low24() {
    for d in 0..(1i64 << 24) {
        check(0, -5, 9, d as i32);
    }
}

/// EXHAUSTIVE over the NEGATIVE mirror of the low-24 window for `b`
/// (adds the extra `'-'` to the dash count).
#[test]
fn exhaustive_b_low24_negative() {
    for b in 0..(1i64 << 24) {
        check(3, -(b as i32), 4, -5);
    }
}

/// EXHAUSTIVE over the TOP of the `i32` range for `a` (10-digit renderings and
/// large finite / inf / NaN float puns): `a` in `INT_MAX-2^24 ..= INT_MAX`.
#[test]
fn exhaustive_a_top24() {
    let start = (i32::MAX as i64) - (1i64 << 24);
    for a in start..=(i32::MAX as i64) {
        check(a as i32, 11, -22, 33);
    }
}

/// EXHAUSTIVE over the BOTTOM of the `i32` range for `a` (`INT_MIN` upward):
/// negative-signed renderings and negative float puns.
#[test]
fn exhaustive_a_bottom24() {
    let start = i32::MIN as i64;
    for a in start..=(start + (1i64 << 24)) {
        check(a as i32, -11, 22, -33);
    }
}
