//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! `my_pow` symbol and asserts the returned `double`s are bit-for-bit equal.
//! Each row uses many randomized inputs from a fixed seed.

mod common;

use common::*;

/// Iterations per randomized row.
const N: usize = 4000;

// ---------------------------------------------------------------------------
// C1 — positive base > 1, positive integral exponent, result in range
// ---------------------------------------------------------------------------
#[test]
fn c1_pos_base_gt1_pos_integral_exponent() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        // Keep base^exponent well inside range: base in (1, 100], exponent
        // chosen so the result cannot overflow.
        let base = rng.range(1.0 + f64::EPSILON, 100.0);
        let max_e = (1000.0 / base.log2()).floor().max(1.0);
        let exponent = rng.int_range(1, max_e as i64) as f64;
        pairs.push((base, exponent));
    }
    check_pairs("C1", &pairs);
}

// ---------------------------------------------------------------------------
// C2 — positive base in (0,1), positive integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c2_pos_base_lt1_pos_integral_exponent() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = rng.range(f64::MIN_POSITIVE, 1.0);
        let exponent = rng.int_range(1, 40) as f64;
        pairs.push((base, exponent));
    }
    check_pairs("C2", &pairs);
}

// ---------------------------------------------------------------------------
// C3 — positive base, negative integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c3_pos_base_neg_integral_exponent() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = rng.range(1.0, 50.0);
        let max_e = (1000.0 / base.log2()).floor().max(1.0);
        let exponent = -(rng.int_range(1, max_e as i64) as f64);
        pairs.push((base, exponent));
    }
    check_pairs("C3", &pairs);
}

// ---------------------------------------------------------------------------
// C4 — positive base, positive non-integral exponent (roots)
// ---------------------------------------------------------------------------
#[test]
fn c4_pos_base_pos_fractional_exponent() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = rng.log_uniform(-30.0, 30.0, false);
        let exponent = rng.range(0.0, 8.0);
        pairs.push((base, exponent));
    }
    check_pairs("C4", &pairs);
}

// ---------------------------------------------------------------------------
// C5 — positive base, negative non-integral exponent
// ---------------------------------------------------------------------------
#[test]
fn c5_pos_base_neg_fractional_exponent() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = rng.log_uniform(-30.0, 30.0, false);
        let exponent = -rng.range(0.0, 8.0);
        pairs.push((base, exponent));
    }
    check_pairs("C5", &pairs);
}

// ---------------------------------------------------------------------------
// C6 — negative base, EVEN integral exponent -> positive result
// ---------------------------------------------------------------------------
#[test]
fn c6_neg_base_even_integral_exponent() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = -rng.range(1.0, 20.0);
        let max_e = (1000.0 / base.abs().log2()).floor().max(2.0) as i64;
        let mut e = rng.int_range(1, max_e);
        if e % 2 != 0 {
            e += 1;
        }
        let exponent = e as f64 * if rng.bool() { 1.0 } else { -1.0 };
        pairs.push((base, exponent));
    }
    check_pairs("C6", &pairs);
}

// ---------------------------------------------------------------------------
// C7 — negative base, ODD integral exponent -> negative result
// ---------------------------------------------------------------------------
#[test]
fn c7_neg_base_odd_integral_exponent() {
    let mut rng = Rng::new(SEED ^ 7);
    let mut pairs = Vec::with_capacity(N);
    for _ in 0..N {
        let base = -rng.range(1.0, 20.0);
        let max_e = (1000.0 / base.abs().log2()).floor().max(3.0) as i64;
        let mut e = rng.int_range(1, max_e);
        if e % 2 == 0 {
            e += 1;
        }
        let exponent = e as f64 * if rng.bool() { 1.0 } else { -1.0 };
        pairs.push((base, exponent));
    }
    check_pairs("C7", &pairs);
}

// ---------------------------------------------------------------------------
// C8 — exponent == 0.0 (and -0.0) with ARBITRARY base -> always 1.0
// ---------------------------------------------------------------------------
#[test]
fn c8_zero_exponent_any_base() {
    let mut pairs = Vec::new();
    for &b in SPECIALS {
        pairs.push((b, 0.0));
        pairs.push((b, -0.0));
    }
    for &bits in NAN_BITS {
        pairs.push((f64::from_bits(bits), 0.0));
        pairs.push((f64::from_bits(bits), -0.0));
    }
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        pairs.push((rng.any_f64(), 0.0));
        pairs.push((rng.any_f64(), -0.0));
    }
    check_pairs("C8", &pairs);
}

// ---------------------------------------------------------------------------
// C9 — exponent == 1.0 with arbitrary base -> identity
// ---------------------------------------------------------------------------
#[test]
fn c9_unit_exponent_any_base() {
    let mut pairs: Vec<(f64, f64)> = SPECIALS.iter().map(|&b| (b, 1.0)).collect();
    for &bits in NAN_BITS {
        pairs.push((f64::from_bits(bits), 1.0));
    }
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        pairs.push((rng.any_f64(), 1.0));
        pairs.push((rng.any_f64(), -1.0));
    }
    check_pairs("C9", &pairs);
}

// ---------------------------------------------------------------------------
// C10 — base == 1.0 with arbitrary exponent -> always 1.0
// ---------------------------------------------------------------------------
#[test]
fn c10_base_one_any_exponent() {
    let mut pairs: Vec<(f64, f64)> = SPECIALS.iter().map(|&e| (1.0, e)).collect();
    for &bits in NAN_BITS {
        pairs.push((1.0, f64::from_bits(bits)));
    }
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        pairs.push((1.0, rng.any_f64()));
    }
    check_pairs("C10", &pairs);
}

// ---------------------------------------------------------------------------
// C11 — base == -1.0 with +/-Inf and integral exponents
// ---------------------------------------------------------------------------
#[test]
fn c11_base_neg_one() {
    let mut pairs = vec![
        (-1.0, f64::INFINITY),
        (-1.0, f64::NEG_INFINITY),
        (-1.0, f64::NAN),
        (-1.0, 0.0),
        (-1.0, -0.0),
    ];
    for e in -300i64..=300 {
        pairs.push((-1.0, e as f64));
        pairs.push((-1.0, e as f64 + 0.5));
    }
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..N {
        pairs.push((-1.0, rng.any_f64()));
        pairs.push((-1.0, rng.int_range(-2000, 2000) as f64));
    }
    check_pairs("C11", &pairs);
}

// ---------------------------------------------------------------------------
// C12 — base == +/-0.0 with POSITIVE exponent -> +/-0.0, sign-of-zero path
// ---------------------------------------------------------------------------
#[test]
fn c12_zero_base_positive_exponent() {
    let mut pairs = Vec::new();
    for &b in &[0.0f64, -0.0f64] {
        for e in 1i64..=64 {
            pairs.push((b, e as f64)); // even and odd integral
            pairs.push((b, e as f64 + 0.5)); // non-integral
        }
        pairs.push((b, f64::INFINITY));
        pairs.push((b, f64::MIN_POSITIVE));
        pairs.push((b, 5e-324));
        pairs.push((b, f64::MAX));
    }
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..N {
        let b = if rng.bool() { 0.0 } else { -0.0 };
        pairs.push((b, rng.range(0.0, 1e6)));
        let b2 = if rng.bool() { 0.0 } else { -0.0 };
        pairs.push((b2, rng.int_range(1, 1000) as f64));
    }
    check_pairs("C12", &pairs);
}

// ---------------------------------------------------------------------------
// C13 — base == +Inf against every exponent class
// ---------------------------------------------------------------------------
#[test]
fn c13_base_pos_infinity() {
    let mut pairs: Vec<(f64, f64)> = SPECIALS
        .iter()
        .map(|&e| (f64::INFINITY, e))
        .collect();
    for &bits in NAN_BITS {
        pairs.push((f64::INFINITY, f64::from_bits(bits)));
    }
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..N {
        pairs.push((f64::INFINITY, rng.any_f64()));
        pairs.push((f64::INFINITY, rng.int_range(-500, 500) as f64));
        pairs.push((f64::INFINITY, rng.range(-500.0, 500.0)));
    }
    check_pairs("C13", &pairs);
}

// ---------------------------------------------------------------------------
// C14 — base == -Inf against every exponent class (odd/even sign path)
// ---------------------------------------------------------------------------
#[test]
fn c14_base_neg_infinity() {
    let mut pairs: Vec<(f64, f64)> = SPECIALS
        .iter()
        .map(|&e| (f64::NEG_INFINITY, e))
        .collect();
    for &bits in NAN_BITS {
        pairs.push((f64::NEG_INFINITY, f64::from_bits(bits)));
    }
    for e in -200i64..=200 {
        pairs.push((f64::NEG_INFINITY, e as f64));
        pairs.push((f64::NEG_INFINITY, e as f64 + 0.5));
    }
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..N {
        pairs.push((f64::NEG_INFINITY, rng.any_f64()));
        pairs.push((f64::NEG_INFINITY, rng.int_range(-500, 500) as f64));
    }
    check_pairs("C14", &pairs);
}

// ---------------------------------------------------------------------------
// C15 — exponent == +/-Inf crossed with |base| < 1, == 1, > 1
// ---------------------------------------------------------------------------
#[test]
fn c15_infinite_exponent_cross_base_magnitude() {
    let mut pairs = Vec::new();
    let bases = [
        0.0, -0.0, 0.5, -0.5, 0.999_999_999_999_999_9, 1.0, -1.0,
        1.000_000_000_000_000_2, 2.0, -2.0, f64::MAX, f64::MIN, f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE, 5e-324, -5e-324, f64::INFINITY, f64::NEG_INFINITY,
        f64::NAN,
    ];
    for &b in &bases {
        pairs.push((b, f64::INFINITY));
        pairs.push((b, f64::NEG_INFINITY));
    }
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let b = rng.log_uniform(-30.0, 30.0, true);
        pairs.push((b, f64::INFINITY));
        pairs.push((b, f64::NEG_INFINITY));
    }
    check_pairs("C15", &pairs);
}

// ---------------------------------------------------------------------------
// C16 — quiet NaN payload propagation
// ---------------------------------------------------------------------------
#[test]
fn c16_quiet_nan_propagation() {
    let mut pairs = Vec::new();
    let quiet_nans: Vec<f64> = NAN_BITS
        .iter()
        .filter(|&&b| b & 0x0008_0000_0000_0000 != 0) // quiet bit set
        .map(|&b| f64::from_bits(b))
        .collect();
    for &n in &quiet_nans {
        for &e in SPECIALS {
            pairs.push((n, e)); // NaN base
            pairs.push((e, n)); // NaN exponent
        }
        for &m in &quiet_nans {
            pairs.push((n, m)); // NaN in both
        }
    }
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let n = quiet_nans[(rng.next_u64() as usize) % quiet_nans.len()];
        pairs.push((n, rng.any_f64()));
        pairs.push((rng.any_f64(), n));
    }
    check_pairs("C16", &pairs);
}

// ---------------------------------------------------------------------------
// C17 — SIGNALLING NaN bit patterns; exact returned bits must match
// ---------------------------------------------------------------------------
#[test]
fn c17_signalling_nan_bit_patterns() {
    let mut pairs = Vec::new();
    let snans: Vec<f64> = NAN_BITS
        .iter()
        .filter(|&&b| b & 0x0008_0000_0000_0000 == 0) // quiet bit clear -> sNaN
        .map(|&b| f64::from_bits(b))
        .collect();
    assert!(!snans.is_empty(), "corpus must contain signalling NaNs");
    for &s in &snans {
        for &e in SPECIALS {
            pairs.push((s, e));
            pairs.push((e, s));
        }
        for &t in &snans {
            pairs.push((s, t));
        }
        // The specified special cases that ignore NaN-ness:
        pairs.push((s, 0.0));
        pairs.push((1.0, s));
    }
    // Randomized sNaN payloads.
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let payload = rng.next_u64() & 0x0007_FFFF_FFFF_FFFF;
        if payload == 0 {
            continue; // that bit pattern is an infinity, not a NaN
        }
        let sign = if rng.bool() { 1u64 << 63 } else { 0 };
        let s = f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload);
        pairs.push((s, rng.any_f64()));
        pairs.push((rng.any_f64(), s));
    }
    check_pairs("C17", &pairs);
}

// ---------------------------------------------------------------------------
// C18 — subnormal base with small positive exponent
// ---------------------------------------------------------------------------
#[test]
fn c18_subnormal_base() {
    let mut pairs = Vec::new();
    let subnormals = [
        5e-324f64,
        -5e-324f64,
        1e-320,
        -1e-320,
        2.2250738585072011e-308, // largest subnormal
        -2.2250738585072011e-308,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
    ];
    for &b in &subnormals {
        for &e in &[0.0f64, 1.0, -1.0, 0.5, -0.5, 2.0, -2.0, 0.001, -0.001, 1e-10] {
            pairs.push((b, e));
        }
    }
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..N {
        // Random subnormal: exponent field zero, non-zero mantissa.
        let mant = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        if mant == 0 {
            continue;
        }
        let sign = if rng.bool() { 1u64 << 63 } else { 0 };
        let b = f64::from_bits(sign | mant);
        pairs.push((b, rng.range(-2.0, 2.0)));
        pairs.push((b, rng.int_range(-4, 4) as f64));
    }
    check_pairs("C18", &pairs);
}

// ---------------------------------------------------------------------------
// C19 — +/-DBL_MAX, +/-DBL_MIN boundary magnitudes without overflow
// ---------------------------------------------------------------------------
#[test]
fn c19_extreme_magnitude_bases() {
    let extremes = [
        f64::MAX,
        f64::MIN,
        -f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::EPSILON,
        -f64::EPSILON,
        1.7976931348623155e308, // nextafter(DBL_MAX, 0)
    ];
    let mut pairs = Vec::new();
    for &b in &extremes {
        for &e in &[
            0.0f64, -0.0, 1.0, -1.0, 0.5, -0.5, 2.0, -2.0, 3.0, -3.0, 0.25,
            1e-10, -1e-10, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        ] {
            pairs.push((b, e));
        }
    }
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..N {
        let b = extremes[(rng.next_u64() as usize) % extremes.len()];
        pairs.push((b, rng.range(-3.0, 3.0)));
    }
    check_pairs("C19", &pairs);
}

// ---------------------------------------------------------------------------
// C20 — exponent straddling the OVERFLOW threshold via nextafter
// ---------------------------------------------------------------------------
#[test]
fn c20_overflow_threshold_straddle() {
    let mut pairs = Vec::new();
    let bases = [2.0f64, 10.0, 1.5, 3.0, 7.0, 1e100, 1.0000001, -2.0, -10.0, -1.5];
    for &b in &bases {
        // Exponent where |b|^e == DBL_MAX.
        let thresh = (f64::MAX.ln()) / b.abs().ln();
        // Walk many ULP steps either side of the threshold.
        let mut e = thresh;
        for _ in 0..64 {
            e = next_after(e, f64::NEG_INFINITY);
        }
        for _ in 0..128 {
            pairs.push((b, e));
            e = next_after(e, f64::INFINITY);
        }
        // Also integral steps around it.
        let t = thresh.floor();
        for d in -4..=4 {
            pairs.push((b, t + d as f64));
        }
    }
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..N {
        let b = bases[(rng.next_u64() as usize) % bases.len()];
        let thresh = (f64::MAX.ln()) / b.abs().ln();
        let jitter = rng.range(-1.0, 1.0);
        pairs.push((b, thresh + jitter));
    }
    check_pairs("C20", &pairs);
}

// ---------------------------------------------------------------------------
// C21 — exponent straddling the UNDERFLOW threshold via nextafter
// ---------------------------------------------------------------------------
#[test]
fn c21_underflow_threshold_straddle() {
    let mut pairs = Vec::new();
    let bases = [2.0f64, 10.0, 1.5, 3.0, 7.0, 0.5, 0.1, -2.0, -10.0, -1.5];
    for &b in &bases {
        // Exponent where |b|^e == smallest subnormal (5e-324).
        let thresh = (5e-324f64).ln() / b.abs().ln();
        let mut e = thresh;
        for _ in 0..64 {
            e = next_after(e, f64::NEG_INFINITY);
        }
        for _ in 0..128 {
            pairs.push((b, e));
            e = next_after(e, f64::INFINITY);
        }
        let t = thresh.floor();
        for d in -4..=4 {
            pairs.push((b, t + d as f64));
        }
        // Also the DBL_MIN (normal) threshold.
        let tn = (f64::MIN_POSITIVE).ln() / b.abs().ln();
        for d in -4..=4 {
            pairs.push((b, tn + d as f64));
        }
    }
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..N {
        let b = bases[(rng.next_u64() as usize) % bases.len()];
        let thresh = (5e-324f64).ln() / b.abs().ln();
        pairs.push((b, thresh + rng.range(-1.0, 1.0)));
    }
    check_pairs("C21", &pairs);
}

// ---------------------------------------------------------------------------
// C22 — unbiased fuzz over the whole 2^128 input space
// ---------------------------------------------------------------------------
#[test]
fn c22_full_bitspace_fuzz() {
    let mut rng = Rng::new(SEED ^ 22);
    let mut pairs = Vec::with_capacity(200_000);
    for _ in 0..200_000 {
        pairs.push((rng.any_f64(), rng.any_f64()));
    }
    check_pairs("C22", &pairs);
}

// ---------------------------------------------------------------------------
// C23 — randomized integral exponents across the whole [-1074, 1024] range
// ---------------------------------------------------------------------------
#[test]
fn c23_integral_exponent_full_range() {
    let mut rng = Rng::new(SEED ^ 23);
    let mut pairs = Vec::new();
    for _ in 0..N * 4 {
        let base = rng.log_uniform(-20.0, 20.0, true);
        let exponent = rng.int_range(-1074, 1024) as f64;
        pairs.push((base, exponent));
    }
    // Exhaustive sweep for a handful of bases.
    for &b in &[2.0f64, -2.0, 0.5, -0.5, 10.0, -10.0, 1.0000000001] {
        for e in -1100i64..=1100 {
            pairs.push((b, e as f64));
        }
    }
    check_pairs("C23", &pairs);
}

// ---------------------------------------------------------------------------
// C24 — errno hygiene / statelessness across repeated interleaved calls
// ---------------------------------------------------------------------------
#[test]
fn c24_errno_hygiene_and_statelessness() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);

    // Inputs that leave errno set, interleaved with valid ones.
    let poisoners: [(f64, f64); 4] = [
        (-2.0, 0.5),    // EDOM
        (0.0, -1.0),    // ERANGE (pole)
        (1e300, 2.0),   // ERANGE (overflow)
        (1e-300, 2.0),  // ERANGE (underflow)
    ];

    let mut mismatches: Vec<String> = Vec::new();
    {
        let _q = quiet();
        for i in 0..2000usize {
            let (pb, pe) = poisoners[i % poisoners.len()];
            let vb = rng.range(1.0, 10.0);
            let ve = rng.int_range(1, 20) as f64;

            // C sequence: poison, then valid.
            let c_poison = unsafe { (l.c_pow)(pb, pe) };
            let c_errno_after = errno_get();
            let c_valid = unsafe { (l.c_pow)(vb, ve) };

            // Rust sequence: identical order.
            let r_poison = unsafe { (l.r_pow)(pb, pe) };
            let r_errno_after = errno_get();
            let r_valid = unsafe { (l.r_pow)(vb, ve) };

            if c_poison.to_bits() != r_poison.to_bits() {
                mismatches.push(format!(
                    "poison call my_pow({pb}, {pe}): C={c_poison} RUST={r_poison}"
                ));
            }
            if c_valid.to_bits() != r_valid.to_bits() {
                mismatches.push(format!(
                    "follow-up my_pow({vb}, {ve}) after poison ({pb}, {pe}): \
                     C={c_valid} RUST={r_valid}"
                ));
            }
            // Both must leave errno in the same state.
            if c_errno_after != r_errno_after {
                mismatches.push(format!(
                    "errno after my_pow({pb}, {pe}): C={c_errno_after} \
                     RUST={r_errno_after}"
                ));
            }
            // And a stale errno must NOT make a valid call return -1.
            errno_set(EDOM);
            let c_after_stale = unsafe { (l.c_pow)(vb, ve) };
            errno_set(EDOM);
            let r_after_stale = unsafe { (l.r_pow)(vb, ve) };
            if c_after_stale.to_bits() != r_after_stale.to_bits() {
                mismatches.push(format!(
                    "stale-errno my_pow({vb}, {ve}): C={c_after_stale} \
                     RUST={r_after_stale}"
                ));
            }
            if mismatches.len() > 20 {
                break;
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "C24 errno hygiene divergences:\n{}",
        mismatches.join("\n")
    );
}

// ---------------------------------------------------------------------------
// C25 — argument-order asymmetry sweep (catches swapped parameters)
// ---------------------------------------------------------------------------
#[test]
fn c25_argument_order_sweep() {
    let mut pairs = Vec::new();
    let mut corpus: Vec<f64> = SPECIALS.to_vec();
    corpus.extend(NAN_BITS.iter().map(|&b| f64::from_bits(b)));
    for &a in &corpus {
        for &b in &corpus {
            pairs.push((a, b));
            pairs.push((b, a));
        }
    }
    check_pairs("C25", &pairs);

    // Additionally prove the two orders are actually distinguishable, so this
    // test would fail if the Rust wrapper swapped its parameters.
    let l = libs();
    let (c_ab, c_ba) = {
        let _q = quiet();
        (unsafe { (l.c_pow)(2.0, 10.0) }, unsafe { (l.c_pow)(10.0, 2.0) })
    };
    assert_ne!(
        c_ab, c_ba,
        "sanity: my_pow(2,10) and my_pow(10,2) must differ"
    );
    assert_eq!(c_ab, 1024.0);
    assert_eq!(c_ba, 100.0);
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `nextafter(x, y)` for finite work; enough for the threshold straddles.
fn next_after(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == y {
        return y;
    }
    if x == 0.0 {
        return if y > 0.0 { 5e-324 } else { -5e-324 };
    }
    let bits = x.to_bits();
    let up = (y > x) == (x > 0.0);
    let next = if up { bits + 1 } else { bits - 1 };
    f64::from_bits(next)
}
