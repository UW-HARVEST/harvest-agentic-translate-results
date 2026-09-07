//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1 … C35). Every test drives both `.so`s
//! through their exported `my_pow` symbol and compares return bits, `stderr`
//! bytes and `errno`.

mod common;

use common::{diff, diff_with_errno, EDOM, ERANGE, Rng, SPECIALS};

/// Small integer exponents split by parity, plus the classes of `base`.
const POS_NORMALS: &[f64] = &[1e-8, 0.25, 0.5, 0.9999, 1.0, 1.5, 2.0, 7.0, 10.0, 123.456, 1e8];

#[test]
fn c1_positive_base_positive_even_integer_exponent() {
    for &b in POS_NORMALS {
        for e in [2.0, 4.0, 6.0, 8.0, 10.0, 42.0] {
            diff(b, e, "C1");
        }
    }
}

#[test]
fn c2_positive_base_positive_odd_integer_exponent() {
    for &b in POS_NORMALS {
        for e in [1.0, 3.0, 5.0, 7.0, 9.0, 41.0] {
            diff(b, e, "C2");
        }
    }
}

#[test]
fn c3_positive_base_negative_integer_exponent() {
    for &b in POS_NORMALS {
        for e in [-1.0, -2.0, -3.0, -4.0, -17.0, -40.0] {
            diff(b, e, "C3");
        }
    }
}

#[test]
fn c4_positive_base_non_integer_positive_exponent() {
    for &b in POS_NORMALS {
        for e in [0.5, 1.0 / 3.0, 0.25, 1.5, 2.5, 3.75, 1e-9, 0.9999999] {
            diff(b, e, "C4");
        }
    }
}

#[test]
fn c5_positive_base_non_integer_negative_exponent() {
    for &b in POS_NORMALS {
        for e in [-0.5, -1.0 / 3.0, -0.25, -1.5, -2.5, -3.75, -1e-9] {
            diff(b, e, "C5");
        }
    }
}

#[test]
fn c6_negative_base_even_integer_exponent() {
    for b in [-1e-8, -0.5, -1.0, -1.5, -2.0, -7.0, -123.456, -1e8] {
        for e in [2.0, 4.0, 8.0, 30.0] {
            diff(b, e, "C6");
        }
    }
}

#[test]
fn c7_negative_base_odd_integer_exponent() {
    for b in [-1e-8, -0.5, -1.0, -1.5, -2.0, -7.0, -123.456, -1e8] {
        for e in [1.0, 3.0, 9.0, 31.0] {
            diff(b, e, "C7");
        }
    }
}

#[test]
fn c8_negative_base_negative_even_integer_exponent() {
    for b in [-1e-8, -0.5, -1.0, -1.5, -2.0, -7.0, -123.456, -1e8] {
        for e in [-2.0, -4.0, -8.0, -30.0] {
            diff(b, e, "C8");
        }
    }
}

#[test]
fn c9_negative_base_negative_odd_integer_exponent() {
    for b in [-1e-8, -0.5, -1.0, -1.5, -2.0, -7.0, -123.456, -1e8] {
        for e in [-1.0, -3.0, -9.0, -31.0] {
            diff(b, e, "C9");
        }
    }
}

#[test]
fn c10_zero_exponent_every_base_class() {
    for &b in SPECIALS {
        diff(b, 0.0, "C10");
    }
    for b in [42.0, -42.0, 1e300, -1e-300] {
        diff(b, 0.0, "C10");
    }
}

#[test]
fn c11_negative_zero_exponent_every_base_class() {
    for &b in SPECIALS {
        diff(b, -0.0, "C11");
    }
}

#[test]
fn c12_base_one_any_exponent() {
    for &e in SPECIALS {
        diff(1.0, e, "C12");
    }
    for e in [1e300, -1e300, 12345.678] {
        diff(1.0, e, "C12");
    }
}

#[test]
fn c13_base_negative_one() {
    for &e in SPECIALS {
        diff(-1.0, e, "C13");
    }
    for e in [3.0, 4.0, -3.0, -4.0, 1e300, -1e300, 2.5, -2.5] {
        diff(-1.0, e, "C13");
    }
}

#[test]
fn c14_base_positive_zero_positive_exponent() {
    for e in [1.0, 2.0, 3.0, 0.5, 2.5, 1e-9, 1e300, f64::INFINITY] {
        diff(0.0, e, "C14");
    }
}

#[test]
fn c15_base_negative_zero_positive_exponent() {
    for e in [1.0, 2.0, 3.0, 4.0, 0.5, 2.5, 1e-9, 1e300, f64::INFINITY] {
        diff(-0.0, e, "C15");
    }
}

#[test]
fn c16_base_positive_infinity() {
    for e in [1.0, 2.0, 3.0, 0.5, -1.0, -2.0, -0.5, 0.0, -0.0, 1e300, -1e300] {
        diff(f64::INFINITY, e, "C16");
    }
}

#[test]
fn c17_base_negative_infinity_positive_exponent() {
    for e in [1.0, 3.0, 5.0, 2.0, 4.0, 0.5, 2.5, 1e300, 9007199254740992.0] {
        diff(f64::NEG_INFINITY, e, "C17");
    }
}

#[test]
fn c18_base_negative_infinity_negative_exponent() {
    for e in [-1.0, -3.0, -5.0, -2.0, -4.0, -0.5, -2.5, -1e300] {
        diff(f64::NEG_INFINITY, e, "C18");
    }
}

#[test]
fn c19_exponent_positive_infinity() {
    for b in [
        2.0, -2.0, 1.5, -1.5, 1e300, -1e300, 0.5, -0.5, 0.999, -0.999, 0.0, -0.0, 1.0, -1.0,
        f64::MIN_POSITIVE, 5e-324,
    ] {
        diff(b, f64::INFINITY, "C19");
    }
}

#[test]
fn c20_exponent_negative_infinity() {
    for b in [
        2.0, -2.0, 1.5, -1.5, 1e300, -1e300, 0.5, -0.5, 0.999, -0.999, 0.0, -0.0, 1.0, -1.0,
        f64::MIN_POSITIVE, 5e-324,
    ] {
        diff(b, f64::NEG_INFINITY, "C20");
    }
}

#[test]
fn c21_base_nan_nonzero_exponent() {
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_DEAD_BEEF_CAFE),
        f64::from_bits(0xFFF8_0000_0000_0001),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
    ];
    for b in nans {
        for e in [1.0, 2.0, -1.0, 0.5, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            diff(b, e, "C21");
        }
    }
}

#[test]
fn c22_exponent_nan_base_not_one() {
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_DEAD_BEEF_CAFE),
        f64::from_bits(0x7FF0_0000_0000_0001),
    ];
    for e in nans {
        for b in [2.0, -2.0, 0.0, -0.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, 1e300] {
            diff(b, e, "C22");
        }
    }
}

#[test]
fn c23_both_nan() {
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_DEAD_BEEF_CAFE),
        f64::from_bits(0xFFF8_0000_0000_0001),
    ];
    for b in nans {
        for e in nans {
            diff(b, e, "C23");
        }
    }
}

#[test]
fn c24_subnormal_base() {
    for b in [5e-324, 1e-320, 2.2250738585072011e-308, -5e-324, -1e-320] {
        for e in [1.0, 2.0, 3.0, -1.0, -2.0, 0.5, -0.5, 0.0] {
            diff(b, e, "C24");
        }
    }
}

#[test]
fn c25_extreme_magnitudes_near_unit_exponent() {
    for b in [f64::MAX, f64::MIN, f64::MIN_POSITIVE, -f64::MIN_POSITIVE] {
        for e in [1.0, -1.0, 0.9999999999, 1.0000000001, -0.9999999999, 0.5, 3.0, -3.0] {
            diff(b, e, "C25");
        }
    }
}

#[test]
fn c26_huge_integer_exponents() {
    for b in [
        1.0000000000000002, // nextafter(1, +inf)
        0.9999999999999999,
        -1.0000000000000002,
        -0.9999999999999999,
        1.0,
        -1.0,
    ] {
        for e in [
            1e18,
            -1e18,
            9007199254740992.0,
            9007199254740993.0,
            -9007199254740992.0,
            1e300,
        ] {
            diff(b, e, "C26");
        }
    }
}

#[test]
fn c27_random_both_in_minus_ten_ten() {
    let mut rng = Rng::new(0xC27_5EED);
    for _ in 0..1200 {
        let b = rng.range(-10.0, 10.0);
        let e = rng.range(-10.0, 10.0);
        diff_with_errno(b, e, 0, "C27");
    }
}

#[test]
fn c28_random_positive_base_wide_exponent() {
    let mut rng = Rng::new(0xC28_5EED);
    for _ in 0..1200 {
        let b = rng.range(f64::MIN_POSITIVE, 100.0);
        let e = rng.range(-50.0, 50.0);
        diff_with_errno(b, e, 0, "C28");
    }
}

#[test]
fn c29_random_positive_base_integer_exponent() {
    let mut rng = Rng::new(0xC29_5EED);
    for _ in 0..1200 {
        let b = rng.range(0.0, 20.0);
        let e = rng.int(-64, 64) as f64;
        diff_with_errno(b, e, 0, "C29");
    }
}

#[test]
fn c30_random_negative_base_integer_exponent() {
    let mut rng = Rng::new(0xC30_5EED);
    for _ in 0..1200 {
        let b = -rng.range(0.0, 20.0);
        let e = rng.int(-64, 64) as f64;
        diff_with_errno(b, e, 0, "C30");
    }
}

#[test]
fn c31_random_raw_bit_patterns() {
    let mut rng = Rng::new(0xC31_5EED);
    for _ in 0..1500 {
        let b = rng.raw_f64();
        let e = rng.raw_f64();
        diff_with_errno(b, e, 0, "C31");
    }
}

#[test]
fn c32_special_value_cross_product() {
    for &b in SPECIALS {
        for &e in SPECIALS {
            diff(b, e, "C32");
        }
    }
}

#[test]
fn c33_interleaved_call_sequencing() {
    // A long alternating sequence of erroring and valid calls. Each call is
    // compared independently, so any cross-call state leakage (a missing
    // `errno = 0`, a sticky flag) shows up as a divergence.
    let script: &[(f64, f64)] = &[
        (-2.0, 0.5),    // EDOM
        (2.0, 3.0),     // valid  -> must not report an error
        (1e300, 2.0),   // ERANGE overflow
        (2.0, 0.5),     // valid
        (0.0, -1.0),    // ERANGE pole
        (-8.0, 3.0),    // valid, negative result
        (-8.0, 1.0 / 3.0), // EDOM
        (1e-300, 2.0),  // ERANGE underflow
        (10.0, 1.0),    // valid
        (f64::NAN, 2.0),// NaN, no errno
        (4.0, 0.5),     // valid
    ];
    // Run the script three times through the same loaded libraries, and also
    // in a single capture-free pass to confirm order-independence.
    for _ in 0..3 {
        for &(b, e) in script {
            diff(b, e, "C33");
        }
    }
    for &(b, e) in script.iter().rev() {
        diff(b, e, "C33-reversed");
    }
}

#[test]
fn c34_caller_preset_errno_then_valid_call() {
    for errno_in in [0, EDOM, ERANGE, 1, 4, 11, 22, 75, -1, i32::MAX, i32::MIN] {
        for (b, e) in [(2.0, 3.0), (0.5, 2.0), (-2.0, 4.0), (1.0, 1.0), (9.0, 0.5)] {
            diff_with_errno(b, e, errno_in, "C34");
        }
    }
}

#[test]
fn c35_stderr_formatting_edge_cases() {
    // Every value below reaches an `fprintf` with `%.2f`, or would if the
    // branch were taken; the harness compares the produced bytes exactly.
    let cases: &[(f64, f64)] = &[
        (-0.0, -1.0),      // "-0.00"
        (-0.0, -3.0),
        (0.0, -1.0),       // "0.00"
        (-1e-300, 0.5),    // "-0.00" from a tiny negative
        (-0.005, 0.5),     // rounding right at 2 dp
        (-0.004999, 0.5),
        (-0.015, 0.5),
        (-2.345, 0.5),     // round-half behaviour
        (-2.355, 0.5),
        (-1.7976931348623157e308, 1.5), // very long %.2f expansion
        (-1e300, 0.5),
        (f64::NEG_INFINITY, 0.5),
        (-3.0, 2.001),     // exponent prints as "2.00" but is non-integer
        (-3.0, 1.9999999999), // prints as "2.00"
        (1e300, 1e300),    // range error with huge operands
        (-1e-320, 0.5),    // subnormal negative base
    ];
    for &(b, e) in cases {
        diff(b, e, "C35");
    }
}
