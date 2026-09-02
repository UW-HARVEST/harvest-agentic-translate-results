//! Phase C — error/boundary-path differential tests.
//!
//! `ERRORS.md` shows the C library has an EMPTY explicit error surface: no
//! `assert`, no `NULL` check, no error enum, no `return -1`, no range check, no
//! pointer or enum parameter. So these tests discharge the mandated generic
//! boundaries (rows B1..B11) by proving that C *accepts* each boundary input and
//! that Rust returns the byte-identical result rather than panicking, aborting,
//! or trapping on overflow.
//!
//! Each assertion compares the exact returned `int` / the exact stdout bytes,
//! never merely "both failed somehow".

mod common;

use common::{LANDMARKS, with_libs};
use std::ffi::c_int;

// --------------------------------------------------------------------------
// B1 — static_sum(0): the no-op / "zero length" input
// --------------------------------------------------------------------------
#[test]
fn b1_zero_update() {
    with_libs(|h| {
        for &park in LANDMARKS {
            h.park_accumulator_at(park, "B1");
            let v = h.static_sum(0, "B1");
            assert_eq!(v, park, "B1: static_sum(0) must be a no-op");
        }
    });
}

// --------------------------------------------------------------------------
// B2 — static_sum(INT_MAX): maximum in-range value
// --------------------------------------------------------------------------
#[test]
fn b2_int_max_update() {
    with_libs(|h| {
        for &park in LANDMARKS {
            h.park_accumulator_at(park, "B2");
            let v = h.static_sum(c_int::MAX, "B2");
            assert_eq!(
                v,
                park.wrapping_add(c_int::MAX),
                "B2: expected two's-complement wrap from {park}"
            );
        }
    });
}

// --------------------------------------------------------------------------
// B3 — static_sum(INT_MIN): minimum in-range value
// --------------------------------------------------------------------------
#[test]
fn b3_int_min_update() {
    with_libs(|h| {
        for &park in LANDMARKS {
            h.park_accumulator_at(park, "B3");
            let v = h.static_sum(c_int::MIN, "B3");
            assert_eq!(
                v,
                park.wrapping_add(c_int::MIN),
                "B3: expected two's-complement wrap from {park}"
            );
        }
    });
}

// --------------------------------------------------------------------------
// B4 — static_sum(-1): the classic C error sentinel is a *valid* input here
// --------------------------------------------------------------------------
#[test]
fn b4_minus_one_sentinel() {
    with_libs(|h| {
        // Land the return value exactly on -1, the value a C caller might
        // mistake for an error, and confirm both libs report it as data.
        h.park_accumulator_at(0, "B4");
        let v = h.static_sum(-1, "B4");
        assert_eq!(v, -1, "B4: static_sum must return -1 as ordinary data");

        for &park in LANDMARKS {
            h.park_accumulator_at(park, "B4");
            let v = h.static_sum(-1, "B4");
            assert_eq!(v, park.wrapping_sub(1), "B4: wrong result from {park}");
        }
    });
}

// --------------------------------------------------------------------------
// B5 — one step past the valid range, upward: INT_MAX + 1
// --------------------------------------------------------------------------
#[test]
fn b5_overflow_one_past_max() {
    with_libs(|h| {
        h.park_accumulator_at(c_int::MAX, "B5");
        let v = h.static_sum(1, "B5");
        assert_eq!(v, c_int::MIN, "B5: INT_MAX + 1 must wrap to INT_MIN");

        // Keep walking past the boundary; must not trap.
        for expect in 0..8 {
            let v = h.static_sum(1, "B5");
            assert_eq!(v, c_int::MIN + 1 + expect, "B5: post-wrap sequence wrong");
        }

        // And the largest possible overflow: INT_MAX + INT_MAX == -2.
        h.park_accumulator_at(c_int::MAX, "B5");
        let v = h.static_sum(c_int::MAX, "B5");
        assert_eq!(v, -2, "B5: INT_MAX + INT_MAX must wrap to -2");
    });
}

// --------------------------------------------------------------------------
// B6 — one step past the valid range, downward: INT_MIN - 1
// --------------------------------------------------------------------------
#[test]
fn b6_underflow_one_past_min() {
    with_libs(|h| {
        h.park_accumulator_at(c_int::MIN, "B6");
        let v = h.static_sum(-1, "B6");
        assert_eq!(v, c_int::MAX, "B6: INT_MIN - 1 must wrap to INT_MAX");

        for expect in 0..8 {
            let v = h.static_sum(-1, "B6");
            assert_eq!(v, c_int::MAX - 1 - expect, "B6: post-wrap sequence wrong");
        }

        // Largest possible underflow: INT_MIN + INT_MIN == 0.
        h.park_accumulator_at(c_int::MIN, "B6");
        let v = h.static_sum(c_int::MIN, "B6");
        assert_eq!(v, 0, "B6: INT_MIN + INT_MIN must wrap to 0");
    });
}

// --------------------------------------------------------------------------
// B7 — driver(0): degenerate stride
// --------------------------------------------------------------------------
#[test]
fn b7_driver_zero_stride() {
    with_libs(|h| {
        for &park in &[0, 1, -1, c_int::MAX, c_int::MIN, 987_654_321] {
            h.park_accumulator_at(park, "B7");
            let out = h.driver(0, "B7");
            let expected = format!("{park}\n").repeat(10);
            assert_eq!(
                out,
                expected.as_bytes(),
                "B7: driver(0) from {park} printed {:?}",
                String::from_utf8_lossy(&out)
            );
        }
    });
}

// --------------------------------------------------------------------------
// B8 — driver(INT_MAX): `i * stride` overflows on every i >= 2
// --------------------------------------------------------------------------
#[test]
fn b8_driver_int_max_stride() {
    with_libs(|h| {
        for &park in &[0, 1, -1, c_int::MAX, c_int::MIN] {
            h.park_accumulator_at(park, "B8");
            let out = h.driver(c_int::MAX, "B8");
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                10,
                "B8: driver must still print exactly 10 lines"
            );
        }
    });
}

// --------------------------------------------------------------------------
// B9 — driver(INT_MIN): `i * stride` overflows, negative direction
// --------------------------------------------------------------------------
#[test]
fn b9_driver_int_min_stride() {
    with_libs(|h| {
        for &park in &[0, 1, -1, c_int::MAX, c_int::MIN] {
            h.park_accumulator_at(park, "B9");
            let out = h.driver(c_int::MIN, "B9");
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                10,
                "B9: driver must still print exactly 10 lines"
            );
        }
    });
}

// --------------------------------------------------------------------------
// B10 — driver(-1): negative sentinel-shaped stride
// --------------------------------------------------------------------------
#[test]
fn b10_driver_minus_one() {
    with_libs(|h| {
        h.park_accumulator_at(0, "B10");
        let out = h.driver(-1, "B10");
        // sum after step i is -(0+1+..+i) => 0,-1,-3,-6,-10,-15,-21,-28,-36,-45
        let expected = b"0\n-1\n-3\n-6\n-10\n-15\n-21\n-28\n-36\n-45\n";
        assert_eq!(
            out,
            expected.to_vec(),
            "B10: got {:?}",
            String::from_utf8_lossy(&out)
        );
    });
}

// --------------------------------------------------------------------------
// B11 — the *accumulated* sum overflows mid-loop
// --------------------------------------------------------------------------
#[test]
fn b11_driver_sum_overflow_midloop() {
    with_libs(|h| {
        // 45 * (INT_MAX/8) overflows well before the loop ends.
        for &stride in &[c_int::MAX / 8, c_int::MIN / 8, c_int::MAX / 3, c_int::MIN / 3] {
            h.park_accumulator_at(0, "B11");
            let out = h.driver(stride, "B11");
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                10,
                "B11: driver must still print exactly 10 lines for stride {stride}"
            );
        }
    });
}

// --------------------------------------------------------------------------
// B12/B13/B14 are N/A (no enum, no pointer, no length parameter). What *is*
// possible across this FFI boundary is any bit pattern of the two `int`
// parameters, so we sweep the landmark set through both entry points, which is
// the direct analogue of "an out-of-range enum value the C still handles".
// --------------------------------------------------------------------------
#[test]
fn b12_b14_full_int_domain_landmarks_are_accepted() {
    with_libs(|h| {
        for &v in LANDMARKS {
            h.static_sum(v, "B12");
            h.driver(v, "B12");
        }
        // Bit patterns that look like out-of-range enum values in real C APIs.
        for &v in &[
            -2147483648i32,
            -999999,
            -100,
            99,
            100,
            1000,
            0x7FFF_FFFE,
            0x0BAD_F00Du32 as i32,
            0xDEAD_BEEFu32 as i32,
            0xFFFF_FFFFu32 as i32,
        ] {
            h.static_sum(v, "B12");
            h.driver(v, "B12");
        }
    });
}
