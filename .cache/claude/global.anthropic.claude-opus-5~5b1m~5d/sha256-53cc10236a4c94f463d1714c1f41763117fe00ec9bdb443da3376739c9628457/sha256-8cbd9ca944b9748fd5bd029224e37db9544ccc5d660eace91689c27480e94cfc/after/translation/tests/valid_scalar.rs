//! Phase B — valid-path differential tests for the scalar entry points:
//! `convert_double_to_int`, `process_negation`, `calculate_with_doubles`.
//!
//! CONFIGS.md rows 1–11 and 28–33.

mod common;

use common::*;

const EDGE_I32: [i32; 15] = [
    i32::MIN,
    i32::MIN + 1,
    -1_000_000,
    -256,
    -255,
    -128,
    -10,
    -1,
    0,
    1,
    10,
    128,
    256,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// convert_double_to_int
// ---------------------------------------------------------------------------

fn check_convert(values: &[f64], row: &str) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnConvertDoubleToInt>(SYM_CONVERT) };
    for &v in values {
        let (cv, rv) = unsafe { (c(v), r(v)) };
        assert_eq!(
            cv,
            rv,
            "{row}: convert_double_to_int({}) => C {cv} vs Rust {rv}",
            show_f64(v)
        );
    }
}

#[test]
fn row01_convert_in_range_integral() {
    let mut v = vec![0.0, 1.0, -1.0, 42.0, -42.0, 1000.0, -1000.0];
    let mut rng = Rng::new(0x1001);
    for _ in 0..5000 {
        v.push(f64::from(rng.next_i32()));
    }
    check_convert(&v, "row01");
}

#[test]
fn row02_convert_in_range_fractional() {
    let mut v = vec![
        0.5, -0.5, 0.9, -0.9, 1.5, -1.5, 2.5, -2.5, 1e-300, -1e-300, 0.999999999, -0.999999999,
    ];
    let mut rng = Rng::new(0x1002);
    for _ in 0..5000 {
        let base = f64::from(rng.next_i32() / 2);
        let frac = (rng.next_u32() as f64) / (u32::MAX as f64);
        v.push(base + frac);
        v.push(base - frac);
    }
    check_convert(&v, "row02");
}

#[test]
fn row03_convert_zeros_and_subnormals() {
    check_convert(
        &[
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            5e-324,
            -5e-324,
            f64::from_bits(1),
            f64::from_bits(0x8000_0000_0000_0001),
        ],
        "row03",
    );
}

#[test]
fn row04_convert_exact_range_endpoints() {
    check_convert(&[2147483647.0, -2147483648.0], "row04");
}

#[test]
fn row05_convert_one_step_past_endpoints() {
    check_convert(
        &[
            2147483648.0,
            -2147483649.0,
            2147483647.5,
            -2147483648.5,
            2147483647.9999998,
            -2147483648.9999995,
            2147483649.0,
            -2147483650.0,
        ],
        "row05",
    );
}

#[test]
fn row06_convert_huge_magnitudes() {
    check_convert(
        &[
            1e300,
            -1e300,
            2f64.powi(40),
            -(2f64.powi(40)),
            2f64.powi(31),
            -(2f64.powi(31)),
            2f64.powi(63),
            -(2f64.powi(63)),
            f64::MAX,
            f64::MIN,
            1e16,
            -1e16,
        ],
        "row06",
    );
}

#[test]
fn row07_convert_infinities_and_nan() {
    check_convert(
        &[
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            // signalling NaN bit pattern
            f64::from_bits(0x7FF0_0000_0000_0001),
            f64::from_bits(0xFFF8_0000_DEAD_BEEF),
        ],
        "row07",
    );
}

#[test]
fn row08_convert_random_bit_patterns() {
    let mut rng = Rng::new(0xC0FFEE);
    let v: Vec<f64> = (0..20_000).map(|_| rng.next_f64_bits()).collect();
    check_convert(&v, "row08");
}

#[test]
fn row09_convert_random_scaled() {
    let mut rng = Rng::new(0xBEEF01);
    let v: Vec<f64> = (0..10_000).map(|_| rng.next_f64_scaled()).collect();
    check_convert(&v, "row09");
}

// ---------------------------------------------------------------------------
// process_negation
// ---------------------------------------------------------------------------

#[test]
fn row10_process_negation_edges() {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnProcessNegation>(SYM_NEG) };
    for &v in &EDGE_I32 {
        let (cv, rv) = unsafe { (c(v), r(v)) };
        assert_eq!(cv, rv, "row10: process_negation({v}) => C {cv} vs Rust {rv}");
    }
}

#[test]
fn row11_process_negation_random() {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnProcessNegation>(SYM_NEG) };
    let mut rng = Rng::new(0x2011);
    for _ in 0..20_000 {
        let v = rng.next_i32();
        let (cv, rv) = unsafe { (c(v), r(v)) };
        assert_eq!(cv, rv, "row11: process_negation({v}) => C {cv} vs Rust {rv}");
    }
}

// ---------------------------------------------------------------------------
// calculate_with_doubles
// ---------------------------------------------------------------------------

fn check_calc(cases: &[(i32, i32, i32)], row: &str) {
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnCalculateWithDoubles>(SYM_CALC) };
    for &(a, b, cc) in cases {
        let (cv, rv) = unsafe { (c(a, b, cc), r(a, b, cc)) };
        assert!(
            same_f64_bits(cv, rv),
            "{row}: calculate_with_doubles({a}, {b}, {cc}) => C {} vs Rust {}",
            show_f64(cv),
            show_f64(rv)
        );
    }
}

#[test]
fn row28_calc_zero_divisor_guard() {
    let mut cases = Vec::new();
    for &a in &EDGE_I32 {
        for cc in -25..=25 {
            cases.push((a, 0, cc));
        }
    }
    let mut rng = Rng::new(0x2801);
    for _ in 0..3000 {
        cases.push((rng.next_i32(), 0, rng.next_i32()));
    }
    check_calc(&cases, "row28");
}

#[test]
fn row29_calc_zero_numerator_signed_zero() {
    let mut cases = Vec::new();
    for &b in &EDGE_I32 {
        if b == 0 {
            continue;
        }
        for cc in -12..=12 {
            cases.push((0, b, cc));
        }
    }
    check_calc(&cases, "row29");
}

#[test]
fn row30_calc_sign_cross_product() {
    let mut cases = Vec::new();
    for &a in &[-7, -1, 0, 1, 7, -123456, 123456] {
        for &b in &[-7, -1, 1, 7, -123456, 123456] {
            for cc in -3..=3 {
                cases.push((a, b, cc));
            }
        }
    }
    check_calc(&cases, "row30");
}

#[test]
fn row31_calc_exponent_sweep() {
    let mut cases = Vec::new();
    for cc in -25..=25 {
        cases.push((355, 113, cc));
        cases.push((-355, 113, cc));
        cases.push((355, -113, cc));
        cases.push((1, 3, cc));
        cases.push((i32::MAX, 7, cc));
    }
    check_calc(&cases, "row31");
}

#[test]
fn row32_calc_extreme_operands() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX];
    let mut cases = Vec::new();
    for &a in &extremes {
        for &b in &extremes {
            for cc in [i32::MIN, -11, -10, -9, -1, 0, 1, 9, 10, 11, i32::MAX] {
                cases.push((a, b, cc));
            }
        }
    }
    check_calc(&cases, "row32");
}

#[test]
fn row33_calc_random() {
    let mut rng = Rng::new(0x3301);
    let cases: Vec<(i32, i32, i32)> = (0..20_000)
        .map(|_| (rng.next_i32(), rng.next_i32(), rng.next_i32()))
        .collect();
    check_calc(&cases, "row33");
}
