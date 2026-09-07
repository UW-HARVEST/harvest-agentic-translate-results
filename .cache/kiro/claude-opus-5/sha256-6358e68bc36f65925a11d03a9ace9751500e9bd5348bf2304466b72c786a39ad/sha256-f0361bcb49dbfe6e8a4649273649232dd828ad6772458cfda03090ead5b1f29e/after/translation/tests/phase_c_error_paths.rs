//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 … E15) plus the generic FFI boundary
//! rows (G1 … G8). Every test asserts C and Rust return the *same* sentinel
//! (`-1.0` where the C errors) and emit the *same* diagnostic bytes — not
//! merely that "both failed somehow".

mod common;

use common::{
    c_pow, call_capturing, diff, diff_with_errno, rust_pow, EDOM, ERANGE, Rng, SPECIALS,
};

/// Asserts both implementations agree AND that the shared behaviour really is
/// the `-1.0` + diagnostic-line rejection described by the given `ERRORS.md`
/// row, so the row cannot be "passed" by neither side erroring.
#[track_caller]
fn diff_expect_rejection(base: f64, exponent: f64, expect_prefix: &str, row: &str) {
    diff(base, exponent, row);

    let (cv, ce, cerr) = call_capturing(c_pow(), base, exponent, 0);
    let (rv, re, rerr) = call_capturing(rust_pow(), base, exponent, 0);

    assert_eq!(cv.to_bits(), rv.to_bits(), "[{row}] value bits differ");
    assert_eq!(cerr, rerr, "[{row}] stderr differs");
    assert_eq!(ce, re, "[{row}] errno differs");

    assert_eq!(
        cv, -1.0,
        "[{row}] expected the C sentinel -1.0 for my_pow({base:?}, {exponent:?}), got {cv:?}"
    );
    let text = String::from_utf8_lossy(&cerr);
    assert!(
        text.starts_with(expect_prefix),
        "[{row}] expected a diagnostic starting with {expect_prefix:?} for \
         my_pow({base:?}, {exponent:?}), got {text:?}"
    );
    assert!(
        ce == EDOM || ce == ERANGE,
        "[{row}] expected errno EDOM/ERANGE after the call, got {ce}"
    );
}

const DOMAIN: &str = "Domain error: pow(";
const RANGE: &str = "Range error: pow(";

// ---------------------------------------------------------------- EDOM rows

#[test]
fn e1_edom_negative_base_non_integer_exponent() {
    for (b, e) in [
        (-2.0, 0.5),
        (-2.0, -0.5),
        (-1.5, 1.0 / 3.0),
        (-10.0, 2.5),
        (-0.5, 0.5),
        (-7.25, -3.75),
    ] {
        diff_expect_rejection(b, e, DOMAIN, "E1");
    }
}

#[test]
fn e2_edom_non_integer_exponent_that_prints_as_integer() {
    for (b, e) in [
        (-3.0, 2.001),
        (-3.0, 1.9999999999),
        (-3.0, 2.0000000000000004),
        (-3.0, -4.001),
    ] {
        diff_expect_rejection(b, e, DOMAIN, "E2");
    }
}

#[test]
fn e3_edom_subnormal_negative_base() {
    for (b, e) in [(-5e-324, 0.5), (-1e-320, 0.5), (-2.2250738585072011e-308, 1.5)] {
        diff_expect_rejection(b, e, DOMAIN, "E3");
    }
}

#[test]
fn e4_edom_huge_negative_base() {
    for (b, e) in [
        (-1.7976931348623157e308, 1.5),
        (-1e300, 0.5),
        (-1e300, -0.5),
        (f64::MIN, 0.5),
    ] {
        diff_expect_rejection(b, e, DOMAIN, "E4");
    }
}

// -------------------------------------------------------------- ERANGE rows

#[test]
fn e5_erange_pole_positive_zero_negative_odd_exponent() {
    for e in [-1.0, -3.0, -5.0, -101.0] {
        diff_expect_rejection(0.0, e, RANGE, "E5");
    }
}

#[test]
fn e6_erange_pole_negative_zero_negative_odd_exponent() {
    for e in [-1.0, -3.0, -5.0, -101.0] {
        diff_expect_rejection(-0.0, e, RANGE, "E6");
    }
}

#[test]
fn e7_erange_pole_even_and_non_integer_exponents() {
    for b in [0.0, -0.0] {
        for e in [-2.0, -4.0, -100.0, -0.5, -2.5, -1e300] {
            diff_expect_rejection(b, e, RANGE, "E7");
        }
    }
}

#[test]
fn e8_erange_overflow_positive() {
    for (b, e) in [
        (1e300, 2.0),
        (10.0, 400.0),
        (2.0, 1100.0),
        (f64::MAX, 2.0),
        (1.5, 10000.0),
        (0.5, -10000.0),
    ] {
        diff_expect_rejection(b, e, RANGE, "E8");
    }
}

#[test]
fn e9_erange_overflow_negative_base_odd_exponent() {
    for (b, e) in [(-1e300, 3.0), (-10.0, 401.0), (-2.0, 1101.0), (f64::MIN, 3.0)] {
        diff_expect_rejection(b, e, RANGE, "E9");
    }
}

#[test]
fn e10_erange_underflow_to_zero() {
    for (b, e) in [
        (1e-300, 2.0),
        (10.0, -400.0),
        (2.0, -1100.0),
        (0.5, 1100.0),
        (-1e-300, 2.0),
        (-2.0, -1101.0),
    ] {
        diff_expect_rejection(b, e, RANGE, "E10");
    }
}

#[test]
fn e11_underflow_into_subnormal_range() {
    // Whether glibc raises ERANGE for a *gradual* underflow that still yields a
    // non-zero subnormal is implementation-defined; the differential assertion
    // holds either way, which is exactly what must be verified.
    for (b, e) in [
        (2.0, -1070.0),
        (2.0, -1060.0),
        (2.0, -1074.0),
        (10.0, -320.0),
        (-2.0, -1071.0),
    ] {
        diff(b, e, "E11");
    }
}

#[test]
fn e12_erange_overflow_just_past_boundary() {
    for (b, e) in [
        (f64::MAX, 1.0000000001),
        (f64::MAX, 1.0000000000000002),
        (1.7976931348623157e308, 1.0 + f64::EPSILON),
        (f64::MIN, 1.0000000001),
    ] {
        diff(b, e, "E12");
    }
    // And the immediately-below-boundary companions, which must NOT error.
    for (b, e) in [(f64::MAX, 1.0), (f64::MAX, 0.9999999999), (f64::MIN, 1.0)] {
        diff(b, e, "E12-below");
    }
}

#[test]
fn e13_edom_branch_takes_precedence_over_erange() {
    // A negative base with a non-integer exponent whose magnitude would also
    // overflow: the C code checks EDOM first (line 35) and must report the
    // domain error, never the range error.
    //
    // Note `1e300` is *integer-valued* as a double, so `my_pow(-1e300, 1e300)`
    // is deliberately NOT listed here — it is a pure overflow (ERANGE), which
    // `e9`/`e8` already cover.
    for (b, e) in [
        (-1e300, 2.5),
        (-1e300, 1000.5),
        (-1e-300, 2.5),
        (f64::MIN, 3.5),
        (-2.0, 1100.5),
        (-2.0, -1100.5),
    ] {
        let (cv, _, cerr) = call_capturing(c_pow(), b, e, 0);
        let (rv, _, rerr) = call_capturing(rust_pow(), b, e, 0);
        assert_eq!(cv.to_bits(), rv.to_bits(), "[E13] value bits differ");
        assert_eq!(cerr, rerr, "[E13] stderr differs");
        let text = String::from_utf8_lossy(&cerr);
        assert!(
            text.starts_with(DOMAIN),
            "[E13] expected the EDOM branch to win for my_pow({b:?}, {e:?}), got {text:?}"
        );
        assert!(
            !text.contains("Range error"),
            "[E13] the ERANGE branch must not run for my_pow({b:?}, {e:?}): {text:?}"
        );
    }
}

#[test]
fn e14_errno_cleared_between_calls() {
    // Fail, then succeed: the second call must be silent and return the real
    // result, proving `errno = 0` at the top of the function is reproduced.
    let failing: &[(f64, f64)] = &[(-2.0, 0.5), (1e300, 2.0), (0.0, -1.0), (1e-300, 2.0)];
    let valid: &[(f64, f64, f64)] = &[
        (2.0, 3.0, 8.0),
        (9.0, 0.5, 3.0),
        (-2.0, 3.0, -8.0),
        (5.0, 0.0, 1.0),
    ];

    for &(fb, fe) in failing {
        for &(vb, ve, want) in valid {
            // Erroring call, un-captured errno left dirty on purpose.
            let (cf, _, _) = call_capturing(c_pow(), fb, fe, 0);
            let (rf, _, _) = call_capturing(rust_pow(), fb, fe, 0);
            assert_eq!(cf.to_bits(), rf.to_bits(), "[E14] setup call diverged");

            // Now the valid call, with errno pre-loaded with the error value.
            for dirty in [EDOM, ERANGE] {
                let (cv, _, cerr) = call_capturing(c_pow(), vb, ve, dirty);
                let (rv, _, rerr) = call_capturing(rust_pow(), vb, ve, dirty);
                assert_eq!(cv.to_bits(), rv.to_bits(), "[E14] value bits differ");
                assert_eq!(cerr, rerr, "[E14] stderr differs");
                assert_eq!(cv, want, "[E14] C returned {cv:?}, expected {want:?}");
                assert!(
                    cerr.is_empty(),
                    "[E14] a valid call must be silent, got {:?}",
                    String::from_utf8_lossy(&cerr)
                );
            }
        }
    }
}

#[test]
fn e15_caller_preset_errno_does_not_leak_into_the_result() {
    for errno_in in [0, 1, 4, EDOM, ERANGE, 35, 75, -1, i32::MIN, i32::MAX] {
        for (b, e) in [(2.0, 10.0), (0.5, -2.0), (-3.0, 3.0), (1.0, f64::NAN)] {
            diff_with_errno(b, e, errno_in, "E15");
        }
    }
}

// ------------------------------------------------- generic FFI boundary rows
//
// `my_pow` takes two `double`s by value and returns a `double`: the ABI has no
// pointers, lengths, arrays or enums, so the null-pointer / zero-length /
// oversized-length / out-of-range-enum classes are unrepresentable here. Their
// analogue for a `double` parameter — every bit pattern the type admits,
// including the ones with no "valid" numeric meaning — is covered below.

#[test]
fn g1_quiet_nan_operands() {
    for b in [f64::NAN, -f64::NAN, 2.0, 1.0] {
        for e in [f64::NAN, -f64::NAN, 2.0, 0.0] {
            diff(b, e, "G1");
        }
    }
}

#[test]
fn g2_signalling_nan_bit_patterns() {
    let snans = [
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FF7_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFF7_FFFF_FFFF_FFFF),
    ];
    for b in snans {
        for e in [1.0, 2.0, 0.0, -0.0, f64::INFINITY, snans[0]] {
            diff(b, e, "G2");
        }
    }
    for e in snans {
        for b in [1.0, 2.0, -2.0, 0.0, f64::INFINITY] {
            diff(b, e, "G2");
        }
    }
}

#[test]
fn g3_nan_payload_propagation() {
    let payloads = [
        0x7FF8_DEAD_BEEF_CAFEu64,
        0xFFF8_DEAD_BEEF_CAFE,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FF8_0000_0000_0001,
    ];
    for pb in payloads {
        for pe in payloads {
            diff(f64::from_bits(pb), f64::from_bits(pe), "G3");
        }
        diff(f64::from_bits(pb), 3.0, "G3");
        diff(3.0, f64::from_bits(pb), "G3");
    }
}

#[test]
fn g4_all_infinity_sign_combinations() {
    let infs = [f64::INFINITY, f64::NEG_INFINITY];
    for b in infs {
        for e in infs {
            diff(b, e, "G4");
        }
        for e in [0.0, -0.0, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.0, -3.0] {
            diff(b, e, "G4");
        }
    }
    for e in infs {
        for b in [0.0, -0.0, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 1e300, -1e300] {
            diff(b, e, "G4");
        }
    }
}

#[test]
fn g5_both_signs_of_zero() {
    let zeros = [0.0, -0.0];
    for b in zeros {
        for e in zeros {
            diff(b, e, "G5");
        }
        for e in [1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.0, -3.0, f64::NAN] {
            diff(b, e, "G5");
        }
    }
    for e in zeros {
        for &b in SPECIALS {
            diff(b, e, "G5");
        }
    }
}

#[test]
fn g6_extreme_finite_magnitudes() {
    let extremes = [
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
    ];
    for b in extremes {
        for e in [1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 0.0, 3.0, -3.0] {
            diff(b, e, "G6");
        }
    }
    for e in extremes {
        for b in [2.0, -2.0, 0.5, -0.5, 1.0, -1.0, 0.0, -0.0] {
            diff(b, e, "G6");
        }
    }
}

#[test]
fn g7_exponent_one_step_past_exact_integer_range() {
    let exps = [
        9007199254740991.0,  // 2^53 - 1 (odd, exact)
        9007199254740992.0,  // 2^53     (even, exact)
        9007199254740993.0,  // 2^53 + 1 (not representable -> rounds to 2^53)
        -9007199254740991.0,
        -9007199254740992.0,
        -9007199254740993.0,
        4503599627370495.0,
        4503599627370496.0,
    ];
    for e in exps {
        for b in [1.0, -1.0, 1.0000000000000002, 0.9999999999999999, -1.0000000000000002] {
            diff(b, e, "G7");
        }
    }
}

#[test]
fn g8_random_raw_bit_patterns() {
    let mut rng = Rng::new(0x6857_1234_ABCD);
    for _ in 0..2000 {
        let b = rng.raw_f64();
        let e = rng.raw_f64();
        diff_with_errno(b, e, 0, "G8");
    }
    // Random patterns paired with a well-behaved operand, so the exotic value
    // is guaranteed to reach the arithmetic rather than short-circuiting.
    let mut rng = Rng::new(0x9999_5EED);
    for _ in 0..800 {
        diff_with_errno(rng.raw_f64(), 2.0, 0, "G8-base");
        diff_with_errno(2.0, rng.raw_f64(), 0, "G8-exp");
    }
}
