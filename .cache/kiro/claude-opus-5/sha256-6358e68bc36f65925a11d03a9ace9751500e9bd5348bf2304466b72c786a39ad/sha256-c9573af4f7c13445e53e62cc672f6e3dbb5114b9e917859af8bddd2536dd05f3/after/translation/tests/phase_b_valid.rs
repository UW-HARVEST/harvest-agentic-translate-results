//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and
//! the Rust `.so` through `libloading` and asserts the returned `double` bit
//! patterns, the resulting `errno`, and the `stderr` bytes all match.

mod common;

use common::{Rng, assert_same, special_values};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Cross-product helper.
fn cross(bases: &[f64], exps: &[f64]) -> Vec<(f64, f64)> {
    let mut v = Vec::with_capacity(bases.len() * exps.len());
    for &b in bases {
        for &e in exps {
            v.push((b, e));
        }
    }
    v
}

// --- row 1 -----------------------------------------------------------------
#[test]
fn cfg_01_base_gt_one_positive_int_exponent() {
    let mut rng = Rng::new(SEED);
    let mut inputs = cross(&[2.0, 10.0, 1.0001, 7.5, 1e3], &[1.0, 2.0, 3.0, 10.0, 53.0]);
    for _ in 0..2000 {
        let b = rng.range(1.0, 1000.0);
        let e = rng.below(60) as f64 + 1.0;
        inputs.push((b, e));
    }
    assert_same("cfg_01", &inputs);
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn cfg_02_base_gt_one_negative_int_exponent() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut inputs = cross(&[2.0, 10.0, 1.5, 1e3], &[-1.0, -2.0, -10.0, -53.0, -300.0]);
    for _ in 0..2000 {
        let b = rng.range(1.0, 1000.0);
        let e = -(rng.below(60) as f64 + 1.0);
        inputs.push((b, e));
    }
    assert_same("cfg_02", &inputs);
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn cfg_03_base_gt_one_fractional_exponent() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut inputs = cross(&[2.0, 3.0, 10.0, 1.0000001], &[0.5, 0.25, 1.0 / 3.0, 2.718281828]);
    for _ in 0..3000 {
        inputs.push((rng.range(1.0, 1e6), rng.range(-30.0, 30.0)));
    }
    assert_same("cfg_03", &inputs);
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn cfg_04_base_between_zero_and_one_fractional_exponent() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut inputs = cross(&[0.5, 0.1, 0.9999999, 1e-10], &[0.5, 1.5, -0.5, 3.25]);
    for _ in 0..3000 {
        inputs.push((rng.range(f64::MIN_POSITIVE, 1.0), rng.range(-50.0, 50.0)));
    }
    assert_same("cfg_04", &inputs);
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn cfg_05_negative_base_odd_integer_exponent() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut inputs = cross(&[-2.0, -3.0, -0.5, -1e10], &[1.0, 3.0, 5.0, 51.0, -3.0, -7.0]);
    for _ in 0..2000 {
        let b = -rng.range(f64::MIN_POSITIVE, 1e3);
        let e = (2 * rng.below(40) as i64 + 1) as f64 * if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        inputs.push((b, e));
    }
    assert_same("cfg_05", &inputs);
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn cfg_06_negative_base_even_integer_exponent() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut inputs = cross(&[-2.0, -3.0, -0.5, -1e10], &[0.0, 2.0, 4.0, 50.0, -2.0, -8.0]);
    for _ in 0..2000 {
        let b = -rng.range(f64::MIN_POSITIVE, 1e3);
        let e = (2 * rng.below(40) as i64) as f64 * if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        inputs.push((b, e));
    }
    assert_same("cfg_06", &inputs);
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn cfg_07_negative_base_huge_integral_exponent() {
    // Above 2^53 every double is an even integer; parity is undetectable.
    let inputs = cross(
        &[-2.0, -1.0000001, -0.9999999, -1e-300],
        &[
            9007199254740992.0,   // 2^53
            9007199254740994.0,
            -9007199254740992.0,
            1e300,
            -1e300,
            f64::MAX,
            f64::MIN,
        ],
    );
    assert_same("cfg_07", &inputs);
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn cfg_08_base_one_any_exponent() {
    let mut rng = Rng::new(SEED ^ 8);
    let mut inputs: Vec<(f64, f64)> = special_values().iter().map(|&e| (1.0, e)).collect();
    for _ in 0..2000 {
        inputs.push((1.0, rng.any_f64()));
    }
    assert_same("cfg_08", &inputs);
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn cfg_09_base_minus_one() {
    let inputs = cross(
        &[-1.0],
        &[
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            0.0,
            -0.0,
            1.0,
            2.0,
            3.0,
            -1.0,
            -2.0,
            0.5,
            -0.5,
            1e300,
            f64::MAX,
        ],
    );
    assert_same("cfg_09", &inputs);
}

// --- rows 10 & 11 ----------------------------------------------------------
#[test]
fn cfg_10_11_zero_exponent_every_base_class() {
    let bases = special_values();
    let mut inputs = cross(&bases, &[0.0, -0.0]);
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..1000 {
        let b = rng.any_f64();
        inputs.push((b, 0.0));
        inputs.push((b, -0.0));
    }
    assert_same("cfg_10_11", &inputs);
}

// --- rows 12 & 13 ----------------------------------------------------------
#[test]
fn cfg_12_13_exponent_one_and_minus_one() {
    let bases = special_values();
    let mut inputs = cross(&bases, &[1.0, -1.0]);
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..1000 {
        let b = rng.any_f64();
        inputs.push((b, 1.0));
        inputs.push((b, -1.0));
    }
    assert_same("cfg_12_13", &inputs);
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn cfg_14_exponent_one_half_sqrt_path() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut inputs = cross(
        &[0.0, -0.0, 1.0, 2.0, 4.0, 1e300, f64::MAX, f64::MIN_POSITIVE, 5e-324, f64::INFINITY],
        &[0.5, -0.5, 1.5, -1.5],
    );
    for _ in 0..3000 {
        inputs.push((rng.range(0.0, 1e10), 0.5));
    }
    assert_same("cfg_14", &inputs);
}

// --- rows 15, 16, 17, 18 --------------------------------------------------
#[test]
fn cfg_15_16_17_18_zero_bases_signed_zero_results() {
    let exps = [
        0.0, -0.0, 1.0, 2.0, 3.0, 4.0, 51.0, 50.0, 0.5, 1.5, 2.5, 1e300, 9007199254740993.0,
        f64::INFINITY, f64::NEG_INFINITY, f64::NAN, f64::MIN_POSITIVE, 5e-324,
    ];
    let inputs = cross(&[0.0, -0.0], &exps);
    assert_same("cfg_15_16_17_18", &inputs);
}

// --- rows 19 & 20 ----------------------------------------------------------
#[test]
fn cfg_19_20_infinite_bases() {
    let exps = [
        0.0, -0.0, 1.0, -1.0, 2.0, 3.0, 4.0, -2.0, -3.0, 0.5, -0.5, 2.5, 1e300, -1e300,
        f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 5e-324, -5e-324,
    ];
    let inputs = cross(&[f64::INFINITY, f64::NEG_INFINITY], &exps);
    assert_same("cfg_19_20", &inputs);
}

// --- rows 21 & 22 ----------------------------------------------------------
#[test]
fn cfg_21_22_infinite_exponents() {
    let mut rng = Rng::new(SEED ^ 21);
    let bases = [
        2.0, -2.0, 0.5, -0.5, 1.0, -1.0, 1.0000000000000002, 0.9999999999999999, 0.0, -0.0,
        f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
    ];
    let mut inputs = cross(&bases, &[f64::INFINITY, f64::NEG_INFINITY]);
    for _ in 0..1000 {
        let b = rng.any_f64();
        inputs.push((b, f64::INFINITY));
        inputs.push((b, f64::NEG_INFINITY));
    }
    assert_same("cfg_21_22", &inputs);
}

// --- row 23 ----------------------------------------------------------------
#[test]
fn cfg_23_subnormal_base() {
    let subnormals = [
        5e-324,
        -5e-324,
        1e-320,
        -1e-320,
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        f64::from_bits(0x0000_0000_0000_0002),
        f64::from_bits(0x800F_FFFF_FFFF_FFFF),
    ];
    let mut inputs = cross(&subnormals, &[1.0, -1.0, 2.0, 3.0, 0.5, 0.0, -0.0, 0.001, -0.001]);
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..2000 {
        // Random subnormal: exponent field zero, random mantissa.
        let bits = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | (rng.next_u64() & 0x8000_0000_0000_0000);
        inputs.push((f64::from_bits(bits), rng.range(-3.0, 3.0)));
    }
    assert_same("cfg_23", &inputs);
}

// --- row 24 ----------------------------------------------------------------
#[test]
fn cfg_24_subnormal_exponent() {
    let subnormals = [5e-324, -5e-324, 1e-320, -1e-320, f64::from_bits(0x000F_FFFF_FFFF_FFFF)];
    let bases = [0.0, -0.0, 1.0, -1.0, 2.0, -2.0, 1e300, -1e300, f64::INFINITY, f64::NAN, 5e-324];
    let inputs = cross(&bases, &subnormals);
    assert_same("cfg_24", &inputs);
}

// --- row 25 ----------------------------------------------------------------
#[test]
fn cfg_25_boundary_magnitudes() {
    let vals = [
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        -f64::EPSILON,
        5e-324,
        -5e-324,
        1.0,
        -1.0,
    ];
    let inputs = cross(&vals, &vals);
    assert_same("cfg_25", &inputs);
}

// --- row 26 ----------------------------------------------------------------
#[test]
fn cfg_26_overflow_boundary() {
    let inputs = vec![
        (f64::MAX, 1.0),
        (f64::MAX, 1.0000000000000002),
        (f64::MAX, 2.0),
        (2.0, 1023.0),
        (2.0, 1024.0),
        (2.0, 1023.9999999999999),
        (2.0, 1024.0000000000002),
        (10.0, 308.0),
        (10.0, 309.0),
        (1.7976931348623157e308, 0.9999999999999999),
        (-2.0, 1023.0),
        (-2.0, 1024.0),
        (f64::MIN, 1.0),
        (f64::MIN, 3.0),
    ];
    assert_same("cfg_26", &inputs);
}

// --- row 27 ----------------------------------------------------------------
#[test]
fn cfg_27_underflow_boundary() {
    let inputs = vec![
        (2.0, -1022.0),
        (2.0, -1023.0),
        (2.0, -1074.0),
        (2.0, -1075.0),
        (2.0, -1076.0),
        (0.5, 1022.0),
        (0.5, 1074.0),
        (0.5, 1075.0),
        (10.0, -308.0),
        (10.0, -324.0),
        (10.0, -400.0),
        (-2.0, -1074.0),
        (-2.0, -1075.0),
        (f64::MIN_POSITIVE, 1.0),
        (f64::MIN_POSITIVE, 2.0),
        (5e-324, 1.0),
        (5e-324, 2.0),
    ];
    assert_same("cfg_27", &inputs);
}

// --- row 28 ----------------------------------------------------------------
#[test]
fn cfg_28_integral_exponent_sweep() {
    let bases = [
        2.0, -2.0, 3.0, -3.0, 0.5, -0.5, 1.0, -1.0, 0.0, -0.0, 1.1, -1.1, 10.0, -10.0,
        f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
    ];
    let mut inputs = Vec::new();
    for &b in bases.iter() {
        for e in -64i32..=64 {
            inputs.push((b, e as f64));
        }
    }
    assert_same("cfg_28", &inputs);
}

// --- row 29 ----------------------------------------------------------------
#[test]
fn cfg_29_state_carry_over_across_calls() {
    // Deliberately interleaves erroring and valid calls in both orders so a
    // stale `errno` in either implementation would show up.
    let inputs = vec![
        (-2.0, 0.5),   // EDOM
        (2.0, 10.0),   // valid
        (0.0, -1.0),   // ERANGE
        (2.0, 10.0),   // valid
        (10.0, 400.0), // ERANGE overflow
        (-2.0, 3.0),   // valid, negative result
        (-2.0, 0.5),   // EDOM
        (-2.0, 0.5),   // EDOM again
        (3.0, 0.5),    // valid
        (10.0, -400.0),// ERANGE underflow
        (1.0, 1.0),    // valid
    ];
    assert_same("cfg_29_forward", &inputs);
    let mut reversed = inputs.clone();
    reversed.reverse();
    assert_same("cfg_29_reverse", &reversed);
}

// --- row 30 ----------------------------------------------------------------
#[test]
fn cfg_30_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 30);
    let inputs: Vec<(f64, f64)> = (0..20_000).map(|_| (rng.any_f64(), rng.any_f64())).collect();
    assert_same("cfg_30", &inputs);
}

// --- row 31 ----------------------------------------------------------------
#[test]
fn cfg_31_random_reasonable_magnitudes() {
    let mut rng = Rng::new(SEED ^ 31);
    let inputs: Vec<(f64, f64)> = (0..20_000)
        .map(|_| (rng.range(-1e3, 1e3), rng.range(-300.0, 300.0)))
        .collect();
    assert_same("cfg_31", &inputs);
}

// --- row 32 ----------------------------------------------------------------
#[test]
fn cfg_32_random_domain_errors() {
    let mut rng = Rng::new(SEED ^ 32);
    let inputs: Vec<(f64, f64)> = (0..10_000)
        .map(|_| {
            let base = -rng.range(f64::MIN_POSITIVE, 1e6);
            // Non-integral by construction.
            let mut e = rng.range(-100.0, 100.0);
            if e.fract() == 0.0 {
                e += 0.5;
            }
            (base, e)
        })
        .collect();
    assert_same("cfg_32", &inputs);
}

// --- row 33 ----------------------------------------------------------------
#[test]
fn cfg_33_random_integral_exponents() {
    let mut rng = Rng::new(SEED ^ 33);
    let inputs: Vec<(f64, f64)> = (0..20_000)
        .map(|_| {
            let base = rng.range(-1e4, 1e4);
            let e = (rng.below(2001) as i64 - 1000) as f64;
            (base, e)
        })
        .collect();
    assert_same("cfg_33", &inputs);
}

// --- row 34 ----------------------------------------------------------------
#[test]
fn cfg_34_special_value_full_cross_product() {
    let v = special_values();
    let inputs = cross(&v, &v);
    assert_same("cfg_34", &inputs);
}
