//! Phase B — rows C1..C6 of CONFIGS.md: leaf arithmetic entry points.
//!
//! Both implementations are reached only through `dlsym` on their `.so`.

mod common;

use common::*;

/// Boundary matrix used by C1..C4.
const BOUNDS: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -(1 << 30),
    -(1 << 16),
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    1 << 16,
    1 << 30,
    i32::MAX - 1,
    i32::MAX,
];

fn drive_binop(
    row: &str,
    cf: OpFn,
    rf: OpFn,
    skip: impl Fn(i32, i32) -> bool,
    seed: u64,
) {
    // Exhaustive boundary matrix.
    for &a in BOUNDS {
        for &b in BOUNDS {
            if skip(a, b) {
                continue;
            }
            for &(u1, u2) in &[(0, 0), (-1, i32::MAX), (i32::MIN, 12345)] {
                let cv = unsafe { cf(a, b, u1, u2) };
                let rv = unsafe { rf(a, b, u1, u2) };
                eq_i32(row, (a, b, u1, u2), cv, rv);
            }
        }
    }
    // Randomised sweep.
    let mut rng = Rng::new(seed);
    for _ in 0..4096 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        if skip(a, b) {
            continue;
        }
        let u1 = rng.next_i32();
        let u2 = rng.next_i32();
        let cv = unsafe { cf(a, b, u1, u2) };
        let rv = unsafe { rf(a, b, u1, u2) };
        eq_i32(row, (a, b, u1, u2), cv, rv);
    }
}

#[test]
fn c1_add_operation() {
    let (c, r) = both();
    drive_binop("C1 add_operation", c.add_operation, r.add_operation, |_, _| false, 0xC1);
}

#[test]
fn c2_multiply_operation() {
    let (c, r) = both();
    drive_binop(
        "C2 multiply_operation",
        c.multiply_operation,
        r.multiply_operation,
        |_, _| false,
        0xC2,
    );
}

#[test]
fn c3_subtract_operation() {
    let (c, r) = both();
    drive_binop(
        "C3 subtract_operation",
        c.subtract_operation,
        r.subtract_operation,
        |_, _| false,
        0xC3,
    );
}

#[test]
fn c4_modulo_operation() {
    let (c, r) = both();
    // `INT_MIN % -1` traps on x86-64 in the C build (ERRORS.md row E30); it is
    // covered separately by `phase_c_errors::phase_c_e30_*`.
    drive_binop(
        "C4 modulo_operation",
        c.modulo_operation,
        r.modulo_operation,
        |a, b| a == i32::MIN && b == -1,
        0xC4,
    );
}

/// Row C5 — every input class of `safe_double_to_int`.
#[test]
fn c5_safe_double_to_int() {
    let (c, r) = both();
    let imax = i32::MAX as f64; // 2147483647.0 exactly
    let imin = i32::MIN as f64; // -2147483648.0 exactly

    let mut cases: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        0.9999999999,
        -0.9999999999,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,  // smallest subnormal
        -5e-324,
        f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FF8_ABCD_EF01_2345), // quiet NaN with payload
        imax,
        imin,
        imax - 1.0,
        imin + 1.0,
        imax + 1.0,
        imin - 1.0,
        imax * 2.0,
        imin * 2.0,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        2147483646.5,
        -2147483647.5,
        2147483647.5,
        -2147483648.5,
    ];
    // Exact neighbours of the two comparison constants.
    for base in [imax, imin] {
        cases.push(nextafter(base, f64::INFINITY));
        cases.push(nextafter(base, f64::NEG_INFINITY));
        cases.push(nextafter(nextafter(base, f64::INFINITY), f64::INFINITY));
        cases.push(nextafter(
            nextafter(base, f64::NEG_INFINITY),
            f64::NEG_INFINITY,
        ));
    }
    for d in &cases {
        let cv = unsafe { (c.safe_double_to_int)(*d) };
        let rv = unsafe { (r.safe_double_to_int)(*d) };
        eq_i32("C5 safe_double_to_int", (d, d.to_bits()), cv, rv);
    }

    // Random raw bit patterns (hits NaNs, infinities, huge and tiny values).
    let mut rng = Rng::new(0xC5_0000);
    for _ in 0..8192 {
        let d = rng.any_f64();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("C5 safe_double_to_int/raw", (d, d.to_bits()), cv, rv);
    }
    // Random values clustered around the i32 range.
    for _ in 0..8192 {
        let d = rng.nearby_f64();
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("C5 safe_double_to_int/near", (d, d.to_bits()), cv, rv);
    }
}

/// Row C6 — `compute_scaled_value` over the full base x scale cross-product.
#[test]
fn c6_compute_scaled_value() {
    let (c, r) = both();
    let bases: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -(1 << 30),
        -1000,
        -1,
        0,
        1,
        1000,
        1 << 30,
        i32::MAX - 1,
        i32::MAX,
    ];
    let scales: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        1.5,
        -1.5,
        0.333,
        0.75,
        0.8,
        2.0,
        1e300,
        -1e300,
        1e-300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::EPSILON,
        f64::MAX,
        f64::MIN,
    ];
    for &b in bases {
        for &s in scales {
            let cv = unsafe { (c.compute_scaled_value)(b, s) };
            let rv = unsafe { (r.compute_scaled_value)(b, s) };
            eq_i32("C6 compute_scaled_value", (b, s, s.to_bits()), cv, rv);
        }
    }

    let mut rng = Rng::new(0xC6_0000);
    for _ in 0..4096 {
        let b = rng.interesting_i32();
        let s = rng.nearby_f64();
        let cv = unsafe { (c.compute_scaled_value)(b, s) };
        let rv = unsafe { (r.compute_scaled_value)(b, s) };
        eq_i32("C6 compute_scaled_value/near", (b, s, s.to_bits()), cv, rv);
    }
    for _ in 0..4096 {
        let b = rng.interesting_i32();
        let s = rng.any_f64();
        let cv = unsafe { (c.compute_scaled_value)(b, s) };
        let rv = unsafe { (r.compute_scaled_value)(b, s) };
        eq_i32("C6 compute_scaled_value/raw", (b, s, s.to_bits()), cv, rv);
    }
}

/// Minimal `nextafter` (avoids depending on libm from the test crate).
fn nextafter(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() {
        return f64::NAN;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        return if toward > 0.0 { 5e-324 } else { -5e-324 };
    }
    let bits = x.to_bits();
    let up = (toward > x) == (x > 0.0);
    f64::from_bits(if up { bits + 1 } else { bits - 1 })
}
