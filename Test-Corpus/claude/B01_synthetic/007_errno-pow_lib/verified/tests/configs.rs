//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. `my_pow` is the library's only public entry
//! point and is itself the lowest-level one (there is no wrapper layered over an
//! internal API), so every row drives it directly. Rows are compared on the full
//! observable triple: return bit pattern, stderr bytes, residual `errno`.
//!
//! Randomized rows use a fixed-seed SplitMix64 PRNG, so failures reproduce.

mod common;

use common::*;

/// Helper: a random integer-valued exponent with the requested parity.
fn rand_int_exp(rng: &mut Rng, lo: i64, hi: i64, parity: Option<u8>) -> f64 {
    loop {
        let n = rng.int_range(lo, hi);
        if let Some(p) = parity {
            if n.rem_euclid(2) as u8 != p {
                continue;
            }
        }
        return n as f64;
    }
}

// ===========================================================================
// C1..C5 — positive normal bases, the four exponent quadrants
// ===========================================================================

#[test]
fn c1_positive_base_gt1_positive_integer_exponent() {
    let mut rng = Rng::new(0xC1);
    for _ in 0..400 {
        let base = rng.range(1.0, 1000.0);
        let exp = rng.int_range(0, 64) as f64;
        diff_ctx(base, exp, "C1 base>1, +int exp");
    }
    for &(b, e) in &[(2.0, 10.0), (3.0, 5.0), (10.0, 3.0), (1.5, 2.0), (7.0, 1.0)] {
        diff_expect_clean(b, e, "C1 fixed");
    }
}

#[test]
fn c2_positive_base_gt1_negative_integer_exponent() {
    let mut rng = Rng::new(0xC2);
    for _ in 0..400 {
        let base = rng.range(1.0, 1000.0);
        let exp = -(rng.int_range(0, 64) as f64);
        diff_ctx(base, exp, "C2 base>1, -int exp");
    }
}

#[test]
fn c3_positive_base_gt1_non_integer_exponent() {
    let mut rng = Rng::new(0xC3);
    for _ in 0..400 {
        let base = rng.range(1.0, 1000.0);
        let exp = rng.range(-60.0, 60.0);
        diff_ctx(base, exp, "C3 base>1, frac exp");
    }
}

#[test]
fn c4_positive_base_lt1_positive_non_integer_exponent() {
    let mut rng = Rng::new(0xC4);
    for _ in 0..400 {
        let base = rng.range(0.0, 1.0);
        let exp = rng.range(0.0, 60.0);
        diff_ctx(base, exp, "C4 base<1, +frac exp");
    }
}

#[test]
fn c5_positive_base_lt1_negative_non_integer_exponent() {
    let mut rng = Rng::new(0xC5);
    for _ in 0..400 {
        let base = rng.range(0.0, 1.0);
        let exp = rng.range(-60.0, 0.0);
        diff_ctx(base, exp, "C5 base<1, -frac exp");
    }
}

// ===========================================================================
// C6..C8 — the C99 "always 1.0" special cases
// ===========================================================================

/// `pow(x, 0)` is 1.0 for every base EXCEPT a signaling NaN, which glibc
/// returns quieted instead. Verified against the C, not assumed.
fn expect_one_unless_snan(base: f64, exp: f64, ctx: &str) {
    if is_snan(base) || is_snan(exp) {
        diff_ctx(base, exp, ctx);
        let o = c_outcome(base, exp);
        assert!(
            f64::from_bits(o.bits).is_nan(),
            "[{ctx}] signaling NaN input should yield a NaN, got {o:?}"
        );
        // The payload survives; only the quiet bit is set.
        let src = if is_snan(base) { base } else { exp };
        assert_eq!(
            o.bits,
            src.to_bits() | 0x0008_0000_0000_0000,
            "[{ctx}] expected the quieted sNaN with its payload preserved"
        );
    } else {
        diff_expect_bits(base, exp, 1.0, ctx);
    }
}

#[test]
fn c6_exponent_positive_zero_is_always_one() {
    for (name, base) in base_classes() {
        expect_one_unless_snan(base, 0.0, &format!("C6 pow({name}, +0.0) == 1.0"));
        diff_expect_clean(base, 0.0, &format!("C6 clean pow({name}, +0.0)"));
    }
    let mut rng = Rng::new(0xC6);
    for _ in 0..300 {
        expect_one_unless_snan(rng.any_f64(), 0.0, "C6 randomized base, +0.0 exp");
    }
}

#[test]
fn c7_exponent_negative_zero_is_always_one() {
    for (name, base) in base_classes() {
        expect_one_unless_snan(base, -0.0, &format!("C7 pow({name}, -0.0) == 1.0"));
    }
    let mut rng = Rng::new(0xC7);
    for _ in 0..300 {
        expect_one_unless_snan(rng.any_f64(), -0.0, "C7 randomized base, -0.0 exp");
    }
}

#[test]
fn c8_base_one_is_always_one() {
    for (name, exp) in exponent_classes() {
        expect_one_unless_snan(1.0, exp, &format!("C8 pow(1.0, {name}) == 1.0"));
        diff_expect_clean(1.0, exp, &format!("C8 clean pow(1.0, {name})"));
    }
    let mut rng = Rng::new(0xC8);
    for _ in 0..300 {
        expect_one_unless_snan(1.0, rng.any_f64(), "C8 1.0 ^ randomized exp");
    }
}

// ===========================================================================
// C9..C11 — negative normal bases: parity path and the EDOM path
// ===========================================================================

#[test]
fn c9_negative_base_odd_integer_exponent() {
    let mut rng = Rng::new(0xC9);
    for _ in 0..400 {
        let base = -rng.range(0.0, 100.0);
        let exp = rand_int_exp(&mut rng, -40, 40, Some(1));
        diff_ctx(base, exp, "C9 neg base, odd int exp");
    }
    // Sign must be preserved through the odd-integer path.
    for &(b, e, want) in &[(-2.0, 3.0, -8.0), (-2.0, 1.0, -2.0), (-3.0, 3.0, -27.0)] {
        diff_expect_bits(b, e, want, "C9 fixed odd");
    }
}

#[test]
fn c10_negative_base_even_integer_exponent() {
    let mut rng = Rng::new(0xC10);
    for _ in 0..400 {
        let base = -rng.range(0.0, 100.0);
        let exp = rand_int_exp(&mut rng, -40, 40, Some(0));
        diff_ctx(base, exp, "C10 neg base, even int exp");
    }
    for &(b, e, want) in &[(-2.0, 4.0, 16.0), (-2.0, 2.0, 4.0), (-3.0, 2.0, 9.0)] {
        diff_expect_bits(b, e, want, "C10 fixed even");
    }
}

#[test]
fn c11_negative_base_non_integer_exponent_is_edom() {
    let mut rng = Rng::new(0xC11);
    for _ in 0..400 {
        let base = -rng.range(1e-30, 100.0);
        // Guarantee a non-integer exponent.
        let exp = rng.int_range(-40, 40) as f64 + rng.range(0.05, 0.95);
        if exp.fract() == 0.0 {
            continue;
        }
        diff_expect_domain_error(base, exp, "C11 neg base, frac exp");
    }
}

// ===========================================================================
// C12..C15 — zero bases
// ===========================================================================

#[test]
fn c12_positive_zero_base_positive_exponent() {
    for exp in [1.0f64, 2.0, 3.0, 0.5, 1.5, 1e18, f64::MAX, INF] {
        diff_expect_bits(0.0, exp, 0.0, "C12 +0 ^ +exp == +0");
        diff_expect_clean(0.0, exp, "C12 clean");
    }
    let mut rng = Rng::new(0xC12);
    for _ in 0..300 {
        let exp = rng.log_uniform(-300.0, 300.0).abs();
        diff_expect_bits(0.0, exp, 0.0, "C12 randomized +exp");
    }
}

#[test]
fn c13_negative_zero_base_positive_odd_integer_exponent_keeps_sign() {
    // pow(-0.0, odd positive int) == -0.0; the sign of zero must survive.
    for exp in [1.0f64, 3.0, 5.0, 101.0, 9007199254740991.0] {
        diff_expect_bits(-0.0, exp, -0.0, "C13 -0 ^ odd == -0");
        let o = c_outcome(-0.0, exp);
        assert_eq!(o.bits, (-0.0f64).to_bits(), "must be NEGATIVE zero");
        assert_ne!(o.bits, (0.0f64).to_bits());
    }
    let mut rng = Rng::new(0xC13);
    for _ in 0..200 {
        let exp = rand_int_exp(&mut rng, 1, 1001, Some(1));
        diff_expect_bits(-0.0, exp, -0.0, "C13 randomized odd");
    }
}

#[test]
fn c14_negative_zero_base_positive_even_or_fractional_exponent() {
    for exp in [2.0f64, 4.0, 100.0, 0.5, 1.5, 2.5, INF] {
        diff_expect_bits(-0.0, exp, 0.0, "C14 -0 ^ even/frac == +0");
    }
    let mut rng = Rng::new(0xC14);
    for _ in 0..200 {
        let exp = rand_int_exp(&mut rng, 2, 1000, Some(0));
        diff_expect_bits(-0.0, exp, 0.0, "C14 randomized even");
    }
    for _ in 0..200 {
        let exp = rng.range(0.05, 100.0);
        if exp.fract() == 0.0 {
            continue;
        }
        diff_expect_bits(-0.0, exp, 0.0, "C14 randomized frac");
    }
}

#[test]
fn c15_zero_base_negative_exponent_is_pole_erange() {
    for base in [0.0f64, -0.0] {
        for exp in [-1.0f64, -2.0, -3.0, -0.5, -1.5, -1e18, f64::MIN] {
            diff_expect_range_error(base, exp, "C15 pole");
        }
    }
    // pow(+-0, -INF) is +INF and is NOT an errno error.
    for base in [0.0f64, -0.0] {
        diff_expect_bits(base, -INF, INF, "C15 pow(+-0, -INF) == +INF");
    }
}

// ===========================================================================
// C16..C18 — infinite bases
// ===========================================================================

#[test]
fn c16_positive_infinite_base() {
    let mut rng = Rng::new(0xC16);
    for exp in [1.0f64, 2.0, 0.5, 1e18, f64::MAX] {
        diff_expect_bits(INF, exp, INF, "C16 +INF ^ +exp == +INF");
    }
    for exp in [-1.0f64, -2.0, -0.5, -1e18, f64::MIN] {
        diff_expect_bits(INF, exp, 0.0, "C16 +INF ^ -exp == +0");
    }
    for _ in 0..200 {
        let m = rng.log_uniform(-300.0, 300.0).abs();
        diff_expect_bits(INF, m, INF, "C16 randomized +exp");
        diff_expect_bits(INF, -m, 0.0, "C16 randomized -exp");
    }
}

#[test]
fn c17_negative_infinite_base_positive_exponent() {
    // odd integer -> -INF, everything else positive -> +INF
    for exp in [1.0f64, 3.0, 5.0, 101.0] {
        diff_expect_bits(-INF, exp, -INF, "C17 -INF ^ odd == -INF");
    }
    for exp in [2.0f64, 4.0, 100.0, 0.5, 1.5, 2.5, 1e18, f64::MAX] {
        diff_expect_bits(-INF, exp, INF, "C17 -INF ^ even/frac == +INF");
    }
    let mut rng = Rng::new(0xC17);
    for _ in 0..200 {
        let odd = rand_int_exp(&mut rng, 1, 1001, Some(1));
        let even = rand_int_exp(&mut rng, 2, 1000, Some(0));
        diff_expect_bits(-INF, odd, -INF, "C17 randomized odd");
        diff_expect_bits(-INF, even, INF, "C17 randomized even");
    }
}

#[test]
fn c18_negative_infinite_base_negative_exponent() {
    for exp in [-1.0f64, -3.0, -5.0, -101.0] {
        diff_expect_bits(-INF, exp, -0.0, "C18 -INF ^ -odd == -0");
    }
    for exp in [-2.0f64, -4.0, -100.0, -0.5, -1.5, -2.5, -1e18, f64::MIN] {
        diff_expect_bits(-INF, exp, 0.0, "C18 -INF ^ -even/frac == +0");
    }
    let mut rng = Rng::new(0xC18);
    for _ in 0..200 {
        let odd = rand_int_exp(&mut rng, 1, 1001, Some(1));
        let even = rand_int_exp(&mut rng, 2, 1000, Some(0));
        diff_expect_bits(-INF, -odd, -0.0, "C18 randomized -odd");
        diff_expect_bits(-INF, -even, 0.0, "C18 randomized -even");
    }
}

// ===========================================================================
// C19..C22 — infinite exponents, and base == -1.0
// ===========================================================================

#[test]
fn c19_positive_infinite_exponent() {
    let mut rng = Rng::new(0xC19);
    for _ in 0..300 {
        let big = 1.0 + rng.range(1e-12, 1e12);
        let small = rng.range(0.0, 1.0);
        diff_expect_bits(big, INF, INF, "C19 |base|>1 ^ +INF == +INF");
        diff_expect_bits(-big, INF, INF, "C19 -|base|>1 ^ +INF == +INF");
        diff_expect_bits(small, INF, 0.0, "C19 |base|<1 ^ +INF == +0");
        diff_expect_bits(-small, INF, 0.0, "C19 -|base|<1 ^ +INF == +0");
    }
    // One ULP either side of 1.0.
    diff_expect_bits(f64::from_bits(0x3FF0_0000_0000_0001), INF, INF, "C19 1+ulp");
    diff_expect_bits(f64::from_bits(0x3FEF_FFFF_FFFF_FFFF), INF, 0.0, "C19 1-ulp");
}

#[test]
fn c20_negative_infinite_exponent() {
    let mut rng = Rng::new(0xC20);
    for _ in 0..300 {
        let big = 1.0 + rng.range(1e-12, 1e12);
        let small = rng.range(0.0, 1.0);
        diff_expect_bits(big, -INF, 0.0, "C20 |base|>1 ^ -INF == +0");
        diff_expect_bits(-big, -INF, 0.0, "C20 -|base|>1 ^ -INF == +0");
        diff_expect_bits(small, -INF, INF, "C20 |base|<1 ^ -INF == +INF");
        diff_expect_bits(-small, -INF, INF, "C20 -|base|<1 ^ -INF == +INF");
    }
    diff_expect_bits(f64::from_bits(0x3FF0_0000_0000_0001), -INF, 0.0, "C20 1+ulp");
    diff_expect_bits(f64::from_bits(0x3FEF_FFFF_FFFF_FFFF), -INF, INF, "C20 1-ulp");
}

#[test]
fn c21_base_minus_one_with_infinite_exponent_is_one() {
    // The C99 carve-out: |base| == 1 exactly, so neither the >1 nor the <1 rule
    // applies, and the result is 1.0 rather than +0/+INF.
    diff_expect_bits(-1.0, INF, 1.0, "C21 pow(-1, +INF) == 1");
    diff_expect_bits(-1.0, -INF, 1.0, "C21 pow(-1, -INF) == 1");
    diff_expect_bits(1.0, INF, 1.0, "C21 pow(1, +INF) == 1");
    diff_expect_bits(1.0, -INF, 1.0, "C21 pow(1, -INF) == 1");
    for b in [-1.0f64, 1.0] {
        for e in [INF, -INF] {
            diff_expect_clean(b, e, "C21 clean");
        }
    }
}

#[test]
fn c22_base_minus_one_integer_exponents() {
    let mut rng = Rng::new(0xC22);
    for _ in 0..300 {
        let odd = rand_int_exp(&mut rng, -1001, 1001, Some(1));
        let even = rand_int_exp(&mut rng, -1000, 1000, Some(0));
        // Includes the -1.0 result that collides with the error sentinel.
        diff_expect_bits(-1.0, odd, -1.0, "C22 (-1)^odd == -1");
        diff_expect_bits(-1.0, even, 1.0, "C22 (-1)^even == 1");
        diff_expect_clean(-1.0, odd, "C22 (-1)^odd is not an error");
    }
}

// ===========================================================================
// C23..C25 — overflow / underflow shapes and their exact boundaries
// ===========================================================================

#[test]
fn c23_overflow_shapes() {
    let mut rng = Rng::new(0xC23);
    let mut hits = 0usize;
    for _ in 0..300 {
        // base > 1 with a large positive exponent
        let base = rng.range(1.5, 1e6);
        let exp = rng.range(200.0, 1e5);
        diff_ctx(base, exp, "C23 base>1, large +exp");
        if is_range_err(base, exp) {
            hits += 1;
        }
        // base < 1 with a large negative exponent
        let small = rng.range(1e-6, 0.9);
        let nexp = -rng.range(200.0, 1e5);
        diff_ctx(small, nexp, "C23 base<1, large -exp");
    }
    assert!(hits > 0, "C23 never produced an overflow");
}

#[test]
fn c24_underflow_shapes() {
    let mut rng = Rng::new(0xC24);
    let mut hits = 0usize;
    for _ in 0..300 {
        let base = rng.range(1.5, 1e6);
        let exp = -rng.range(200.0, 1e5);
        diff_ctx(base, exp, "C24 base>1, large -exp");
        if is_range_err(base, exp) {
            hits += 1;
        }
        let small = rng.range(1e-6, 0.9);
        let pexp = rng.range(200.0, 1e5);
        diff_ctx(small, pexp, "C24 base<1, large +exp");
    }
    assert!(hits > 0, "C24 never produced an underflow");
}

#[test]
fn c25_overflow_and_underflow_boundaries_for_random_bases() {
    let mut rng = Rng::new(0xC25);
    let mut checked = 0usize;
    for _ in 0..40 {
        // A base strictly greater than 1 so an overflow threshold exists.
        let base = 1.0 + rng.range(1e-6, 1e4);
        for negate in [false, true] {
            if let Some((clean, err)) = try_bisect_range_boundary(base, 1.0, negate) {
                // Adjacent doubles on opposite sides of the threshold.
                diff_expect_clean(base, clean, "C25 boundary clean side");
                diff_expect_range_error(base, err, "C25 boundary error side");
                // And a few ULPs out on both sides.
                for k in 1..=4u64 {
                    let lo = f64::from_bits(clean.abs().to_bits() - k);
                    let hi = f64::from_bits(err.abs().to_bits() + k);
                    diff_ctx(base, if negate { -lo } else { lo }, "C25 further clean");
                    diff_ctx(base, if negate { -hi } else { hi }, "C25 further error");
                }
                checked += 1;
            }
        }
    }
    assert!(checked >= 40, "C25 examined too few boundaries: {checked}");
}

// ===========================================================================
// C26..C31 — NaN handling, bit-exact including payloads
// ===========================================================================

#[test]
fn c26_quiet_nan_base_propagates() {
    for exp in [2.0f64, -2.0, 0.5, -0.5, 3.0, 1e18, INF, -INF, f64::MAX] {
        diff_ctx(QNAN, exp, "C26 qNaN base");
        let o = c_outcome(QNAN, exp);
        assert!(
            f64::from_bits(o.bits).is_nan(),
            "qNaN base must propagate a NaN for exp={exp}: {o:?}"
        );
        assert!(o.stderr.is_empty(), "NaN must not raise an errno error");
    }
    // Payload preservation for an ordinary quiet NaN.
    for payload in [1u64, 2, 0x1234, 0x7_FFFF_FFFF_FFFF] {
        let nan = f64::from_bits(0x7FF8_0000_0000_0000 | payload);
        diff_ctx(nan, 2.0, "C26 qNaN payload");
        let o = c_outcome(nan, 2.0);
        assert_eq!(o.bits, nan.to_bits(), "quiet NaN payload must pass through");
    }
}

#[test]
fn c27_signaling_nan_base_is_quieted_with_payload_preserved() {
    for payload in [1u64, 2, 0x1234, 0x3_FFFF_FFFF_FFFF] {
        let snan = f64::from_bits(0x7FF0_0000_0000_0000 | payload);
        assert!(is_snan(snan));
        diff_ctx(snan, 2.0, "C27 sNaN base");
        let o = c_outcome(snan, 2.0);
        assert_eq!(
            o.bits,
            snan.to_bits() | 0x0008_0000_0000_0000,
            "sNaN must be quieted with its payload preserved"
        );
    }
    // The canonical sNaN from the catalogue.
    diff_ctx(SNAN, 2.0, "C27 SNAN const");
    assert_eq!(c_outcome(SNAN, 2.0).bits, 0x7FF8_0000_0000_0001);
}

#[test]
fn c28_negative_nan_sign_handling() {
    for exp in [2.0f64, -2.0, 0.5, 3.0, INF, -INF] {
        diff_ctx(NEG_QNAN, exp, "C28 -qNaN base");
        diff_ctx(NEG_SNAN, exp, "C28 -sNaN base");
    }
    for base in [2.0f64, -2.0, 0.5, 10.0] {
        diff_ctx(base, NEG_QNAN, "C28 -qNaN exp");
        diff_ctx(base, NEG_SNAN, "C28 -sNaN exp");
    }
    // Whether the sign of the NaN survives is glibc's business; we only require
    // the two implementations to agree, and record what the C actually does.
    let o = c_outcome(NEG_QNAN, 2.0);
    assert!(f64::from_bits(o.bits).is_nan());
}

#[test]
fn c29_nan_exponent_propagates() {
    for base in [2.0f64, -2.0, 0.5, -0.5, 10.0, 0.0, -0.0, INF, -INF, f64::MAX] {
        diff_ctx(base, QNAN, "C29 qNaN exp");
        diff_ctx(base, SNAN, "C29 sNaN exp");
    }
    // base 1.0 and exponent 0.0 are the two documented carve-outs, covered by
    // C6/C7/C8; every other base must propagate the NaN.
    for base in [2.0f64, -2.0, 0.5, 10.0] {
        let o = c_outcome(base, QNAN);
        assert!(f64::from_bits(o.bits).is_nan(), "NaN exp must propagate");
        assert!(o.stderr.is_empty());
    }
}

#[test]
fn c30_both_arguments_nan() {
    let payloads = [1u64, 2, 0x1234, 0x5_5555_5555_5555];
    for &pb in &payloads {
        for &pe in &payloads {
            let b = f64::from_bits(0x7FF8_0000_0000_0000 | pb);
            let e = f64::from_bits(0x7FF8_0000_0000_0000 | pe);
            diff_ctx(b, e, "C30 both qNaN");
            // Which of the two payloads propagates is implementation-defined;
            // the requirement is that both implementations pick the same one.
            let o = c_outcome(b, e);
            assert!(f64::from_bits(o.bits).is_nan());
        }
    }
    diff_ctx(QNAN, QNAN, "C30 qNaN/qNaN");
    diff_ctx(SNAN, SNAN, "C30 sNaN/sNaN");
    diff_ctx(QNAN, SNAN, "C30 qNaN/sNaN");
    diff_ctx(SNAN, QNAN, "C30 sNaN/qNaN");
    diff_ctx(NEG_QNAN, QNAN, "C30 -qNaN/qNaN");
}

#[test]
fn c31_randomized_nan_payloads() {
    let mut rng = Rng::new(0xC31);
    for _ in 0..500 {
        let nan = rng.any_nan();
        let exp = rng.range(-100.0, 100.0);
        diff_ctx(nan, exp, "C31 random NaN base");
        diff_ctx(exp, nan, "C31 random NaN exp");
        diff_ctx(nan, rng.any_nan(), "C31 random NaN both");
    }
}

// ===========================================================================
// C32..C36 — extreme finite magnitudes
// ===========================================================================

#[test]
fn c32_subnormal_bases() {
    let subs = [
        5e-324f64,
        -5e-324,
        f64::from_bits(0x0000_0000_0000_0002),
        f64::from_bits(0x0008_0000_0000_0000),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        -f64::from_bits(0x000F_FFFF_FFFF_FFFF),
    ];
    for &b in &subs {
        for e in [1.0f64, -1.0, 2.0, -2.0, 3.0, -3.0, 0.5, -0.5, 0.0, INF, -INF, QNAN] {
            diff_ctx(b, e, "C32 subnormal base");
        }
    }
    let mut rng = Rng::new(0xC32);
    for _ in 0..400 {
        let b = rng.subnormal();
        let e = rng.range(-10.0, 10.0);
        diff_ctx(b, e, "C32 randomized subnormal base");
    }
}

#[test]
fn c33_subnormal_exponents() {
    let subs = [
        5e-324f64,
        -5e-324,
        f64::from_bits(0x0008_0000_0000_0000),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
    ];
    for &e in &subs {
        for (name, b) in base_classes() {
            diff_ctx(b, e, &format!("C33 subnormal exp, base={name}"));
        }
    }
    // A subnormal exponent is so close to zero that the result is 1.0 — but only
    // for a POSITIVE base. A subnormal exponent is not an integer, so a negative
    // base still takes the EDOM path (the domain check wins over the
    // "exponent is essentially zero" shortcut).
    for b in [2.0f64, 10.0, 0.5, 1e300, f64::MIN_POSITIVE] {
        diff_expect_bits(b, 5e-324, 1.0, "C33 +base ^ min_subnormal == 1.0");
        diff_expect_clean(b, 5e-324, "C33 clean");
    }
    for b in [-2.0f64, -10.0, -0.5, -1e300] {
        diff_expect_domain_error(b, 5e-324, "C33 -base ^ min_subnormal is EDOM");
    }
    let mut rng = Rng::new(0xC33);
    for _ in 0..300 {
        diff_ctx(rng.finite(), rng.subnormal(), "C33 randomized subnormal exp");
    }
}

#[test]
fn c34_dbl_max_bases() {
    for b in [f64::MAX, f64::MIN] {
        for (name, e) in exponent_classes() {
            diff_ctx(b, e, &format!("C34 DBL_MAX base, exp={name}"));
        }
    }
    // Exercises the 309-digit %.2f rendering in the error message.
    let o = c_outcome(f64::MAX, 2.0);
    assert!(o.stderr.len() > 300, "expected the long rendering");
    diff_expect_range_error(f64::MAX, 2.0, "C34 DBL_MAX^2");
    diff_expect_bits(f64::MAX, 1.0, f64::MAX, "C34 DBL_MAX^1");
    diff_expect_bits(f64::MIN, 1.0, f64::MIN, "C34 -DBL_MAX^1");
    diff_expect_bits(f64::MAX, 0.0, 1.0, "C34 DBL_MAX^0");
}

#[test]
fn c35_dbl_min_bases() {
    for b in [f64::MIN_POSITIVE, -f64::MIN_POSITIVE] {
        for (name, e) in exponent_classes() {
            diff_ctx(b, e, &format!("C35 DBL_MIN base, exp={name}"));
        }
    }
    diff_expect_range_error(f64::MIN_POSITIVE, 2.0, "C35 DBL_MIN^2");
    diff_expect_bits(f64::MIN_POSITIVE, 1.0, f64::MIN_POSITIVE, "C35 DBL_MIN^1");
    // The rounds-to-0.00 rendering path.
    let o = c_outcome(f64::MIN_POSITIVE, 2.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(0.00, 2.00) caused overflow or underflow.\n"
    );
}

#[test]
fn c36_base_near_one_with_huge_exponents() {
    let bases = [
        f64::from_bits(0x3FF0_0000_0000_0001), // 1 + 1ulp
        f64::from_bits(0x3FEF_FFFF_FFFF_FFFF), // 1 - 1ulp
        1.0 + 2f64.powi(-52),
        1.0 - 2f64.powi(-53),
        1.0000000000000002,
        0.9999999999999999,
        -1.0000000000000002,
        -0.9999999999999999,
    ];
    for &b in &bases {
        for e in [1e15f64, 1e16, 1e17, 1e18, -1e15, -1e18, 1e300, -1e300] {
            diff_ctx(b, e, "C36 base near 1, huge exp");
        }
    }
    // Extreme cancellation but no errno error.
    diff_expect_clean(1.0000000000000002, 1e18, "C36 no error");
    let mut rng = Rng::new(0xC36);
    for _ in 0..300 {
        let b = 1.0 + rng.range(-1e-13, 1e-13);
        let e = rng.log_uniform(10.0, 18.0);
        diff_ctx(b, e, "C36 randomized near-1 base");
    }
}

// ===========================================================================
// C37..C39 — exponents a compiler or libm may special-case
// ===========================================================================

#[test]
fn c37_identity_and_squaring_exponents() {
    // pow(x, 1), pow(x, -1), pow(x, 2), pow(x, -2) are commonly strength-reduced;
    // if either side did that, errno behaviour would diverge.
    for (name, b) in base_classes() {
        for e in [1.0f64, -1.0, 2.0, -2.0] {
            diff_ctx(b, e, &format!("C37 base={name}, exp={e}"));
        }
    }
    let mut rng = Rng::new(0xC37);
    for _ in 0..400 {
        let b = rng.finite();
        for e in [1.0f64, -1.0, 2.0, -2.0] {
            diff_ctx(b, e, "C37 randomized base");
        }
    }
}

#[test]
fn c38_sqrt_cbrt_and_half_integer_exponents() {
    for (name, b) in base_classes() {
        for e in [0.5f64, -0.5, 1.0 / 3.0, -1.0 / 3.0, 1.5, -1.5, 2.5, -2.5] {
            diff_ctx(b, e, &format!("C38 base={name}, exp={e}"));
        }
    }
    let mut rng = Rng::new(0xC38);
    for _ in 0..400 {
        let b = rng.finite();
        for e in [0.5f64, -0.5, 1.5, 2.5] {
            diff_ctx(b, e, "C38 randomized base");
        }
    }
}

#[test]
fn c39_integrality_and_parity_detection_boundaries() {
    // At and beyond 2^53 the spacing of doubles exceeds 1, so every value is an
    // integer and parity detection changes character.
    let exps = [
        4503599627370495.0f64,   // 2^52 - 1
        4503599627370496.0,      // 2^52
        4503599627370497.0,      // 2^52 + 1
        9007199254740991.0,      // 2^53 - 1 (odd)
        9007199254740992.0,      // 2^53     (even)
        9007199254740993.0,      // rounds to 2^53
        9007199254740994.0,      // 2^53 + 2
        18014398509481984.0,     // 2^54
        1.0 / 0.0f64 * 0.0,      // NaN, as a control
    ];
    for &e in &exps {
        for b in [-1.0f64, 1.0, -0.0, 0.0, -2.0, 2.0, -INF, INF] {
            diff_ctx(b, e, "C39 huge integral exponent");
            diff_ctx(b, -e, "C39 huge negative integral exponent");
        }
    }
    // Half-integers just below the 2^52 threshold, where .5 is still representable.
    for k in 0..32u64 {
        let e = 2251799813685248.0f64 + 0.5 + k as f64;
        diff_ctx(-1.0, e, "C39 half-integer exponent");
        diff_ctx(-2.0, e, "C39 half-integer exponent, base -2");
    }
}

// ===========================================================================
// C40 — the %.2f rendering matrix inside error messages
// ===========================================================================

#[test]
fn c40_printf_rendering_matrix() {
    // Each pair produces an error message whose %.2f rendering has a different
    // shape. Expected strings are the C's actual output.
    let cases: &[(f64, f64, &str)] = &[
        (-2.0, 0.5, "Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"),
        (-8.0, 1.0 / 3.0, "Domain error: pow(-8.00, 0.33) is undefined in the real number domain.\n"),
        (0.0, -1.0, "Range error: pow(0.00, -1.00) caused overflow or underflow.\n"),
        (-0.0, -1.0, "Range error: pow(-0.00, -1.00) caused overflow or underflow.\n"),
        (5e-324, 2.0, "Range error: pow(0.00, 2.00) caused overflow or underflow.\n"),
        (-5e-324, 3.0, "Range error: pow(-0.00, 3.00) caused overflow or underflow.\n"),
        (10.0, 400.0, "Range error: pow(10.00, 400.00) caused overflow or underflow.\n"),
        (10.0, -400.0, "Range error: pow(10.00, -400.00) caused overflow or underflow.\n"),
        (10.0, 1e18, "Range error: pow(10.00, 1000000000000000000.00) caused overflow or underflow.\n"),
        (10.0, -1e18, "Range error: pow(10.00, -1000000000000000000.00) caused overflow or underflow.\n"),
        (-1.5, 0.25, "Domain error: pow(-1.50, 0.25) is undefined in the real number domain.\n"),
        (-1e-30, 0.5, "Domain error: pow(-0.00, 0.50) is undefined in the real number domain.\n"),
    ];
    for &(b, e, want) in cases {
        diff_ctx(b, e, "C40 rendering");
        let o = c_outcome(b, e);
        assert_eq!(
            String::from_utf8_lossy(&o.stderr),
            want,
            "C40 rendering mismatch for base={b:?} exp={e:?}"
        );
    }

    // The 309-digit case, checked separately because of its length.
    let o = c_outcome(f64::MAX, 2.0);
    let s = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(s.starts_with("Range error: pow(1797693134862315708"));
    assert!(s.ends_with("858368.00, 2.00) caused overflow or underflow.\n"));
    diff_ctx(f64::MAX, 2.0, "C40 309-digit rendering");

    // `inf`/`nan` renderings are UNREACHABLE in these messages: glibc's pow never
    // sets errno when either argument is non-finite. Prove that, rather than
    // assuming it.
    for (bn, b) in base_classes() {
        for (en, e) in exponent_classes() {
            if b.is_finite() && e.is_finite() {
                continue;
            }
            let o = c_outcome(b, e);
            assert!(
                o.stderr.is_empty(),
                "non-finite argument unexpectedly produced a message \
                 (base={bn}, exp={en}): {:?}",
                String::from_utf8_lossy(&o.stderr)
            );
            diff_ctx(b, e, "C40 non-finite produces no message");
        }
    }
}

// ===========================================================================
// C41..C42 — statefulness and cross-implementation interleaving
// ===========================================================================

#[test]
fn c41_repeated_and_alternating_calls_are_stateless() {
    // The same input many times must give the same answer every time.
    for _ in 0..50 {
        diff_ctx(-2.0, 0.5, "C41 repeated edom");
        diff_ctx(2.0, 10.0, "C41 repeated clean");
    }
    // An error-producing call followed immediately by a valid one: no residual
    // errno or stderr state may leak forward.
    for _ in 0..50 {
        diff_expect_domain_error(-2.0, 0.5, "C41 edom then...");
        diff_expect_clean(2.0, 10.0, "C41 ...clean");
        diff_expect_range_error(10.0, 400.0, "C41 erange then...");
        diff_expect_clean(3.0, 3.0, "C41 ...clean again");
        diff_expect_range_error(0.0, -1.0, "C41 pole then...");
        diff_expect_clean(-1.0, 3.0, "C41 ...legit -1.0");
    }
}

#[test]
fn c42_interleaved_call_order_does_not_matter() {
    // Both .so's share one errno TLS slot and one stderr FILE*. Calling Rust
    // first must give the same comparison result as calling C first.
    let cases: &[(f64, f64)] = &[
        (-2.0, 0.5),
        (10.0, 400.0),
        (10.0, -400.0),
        (0.0, -1.0),
        (-0.0, -1.0),
        (2.0, 10.0),
        (-1.0, 3.0),
        (QNAN, 2.0),
        (SNAN, 2.0),
        (INF, -2.0),
        (f64::MAX, 2.0),
        (f64::MIN_POSITIVE, 2.0),
    ];
    for &(b, e) in cases {
        diff_ctx(b, e, "C42 C-first");
        diff_reversed(b, e, "C42 Rust-first");
    }
    // Long interleaved run mixing error and clean inputs.
    let mut rng = Rng::new(0xC42);
    for i in 0..300 {
        let (b, e) = if i % 3 == 0 {
            (-rng.range(0.1, 10.0), rng.range(0.1, 3.0) + 0.5)
        } else if i % 3 == 1 {
            (rng.range(1.1, 100.0), rng.range(200.0, 1000.0))
        } else {
            (rng.range(0.1, 100.0), rng.range(-10.0, 10.0))
        };
        if i % 2 == 0 {
            diff_ctx(b, e, "C42 interleaved");
        } else {
            diff_reversed(b, e, "C42 interleaved reversed");
        }
    }
}

// ===========================================================================
// C43..C45 — broad randomized sweeps
// ===========================================================================

#[test]
fn c43_structured_random_sweep() {
    let mut rng = Rng::new(0xC43);
    for _ in 0..1500 {
        // Log-uniform base over the whole exponent range, both signs.
        let base = rng.log_uniform(-308.0, 308.0);
        let exp = rng.range(-1000.0, 1000.0);
        diff_ctx(base, exp, "C43 structured sweep");
    }
}

#[test]
fn c44_full_entropy_fuzz() {
    let mut rng = Rng::new(0xC44);
    let mut classes = [0usize; 5]; // nan, inf, zero, subnormal, normal
    for _ in 0..3000 {
        let base = rng.any_f64();
        let exp = rng.any_f64();
        diff_ctx(base, exp, "C44 full-entropy fuzz");
        let idx = if base.is_nan() {
            0
        } else if base.is_infinite() {
            1
        } else if base == 0.0 {
            2
        } else if base.is_subnormal() {
            3
        } else {
            4
        };
        classes[idx] += 1;
    }
    eprintln!(
        "C44 base classes: nan={} inf={} zero={} subnormal={} normal={}",
        classes[0], classes[1], classes[2], classes[3], classes[4]
    );
    assert!(classes[0] > 0, "fuzz never produced a NaN base");
    assert!(classes[4] > 0, "fuzz never produced a normal base");
}

#[test]
fn c45_small_integer_grid() {
    // Exhaustive cross-product, including both signed zeros.
    let mut vals: Vec<f64> = Vec::new();
    for n in -20i32..=20 {
        vals.push(n as f64);
    }
    vals.push(-0.0);
    for &b in &vals {
        for &e in &vals {
            diff_ctx(b, e, "C45 small-integer grid");
        }
    }
}
