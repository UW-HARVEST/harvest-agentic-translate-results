//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each row constructs the exact triggering
//! condition, calls BOTH implementations through their `.so` exports, and
//! asserts they reject identically — same return bits, same stderr bytes, same
//! residual `errno` — not merely "both failed somehow".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// E1 — EDOM: negative finite base, non-integer finite exponent
// ---------------------------------------------------------------------------

#[test]
fn e1_edom_negative_base_fractional_exponent() {
    for &(base, exp) in &[
        (-2.0, 0.5),
        (-2.0, 1.5),
        (-2.0, -0.5),
        (-0.5, 0.5),
        (-1.0, 0.5),
        (-10.0, 2.5),
        (-3.0, 0.25),
        (-1e100, 0.5),
        (-1e-100, 0.5),
        (f64::MIN, 0.5),
        (-f64::MIN_POSITIVE, 0.5),
    ] {
        diff_expect_domain_error(base, exp, "E1 edom fractional");
    }

    // Randomized: any negative finite base with any non-integer exponent.
    let mut rng = Rng::new(0xE1_0001);
    for _ in 0..400 {
        let base = -rng.log_uniform(-150.0, 150.0).abs();
        // Force a non-integer exponent.
        let exp = rng.range(-50.0, 50.0) + 0.5;
        if exp.fract() == 0.0 {
            continue;
        }
        diff_expect_domain_error(base, exp, "E1 edom randomized");
    }
}

// ---------------------------------------------------------------------------
// E2 — EDOM with %.2f truncation of the exponent (1/3 prints as 0.33)
// ---------------------------------------------------------------------------

#[test]
fn e2_edom_cube_root_prints_truncated_exponent() {
    let o = c_outcome(-8.0, 1.0 / 3.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Domain error: pow(-8.00, 0.33) is undefined in the real number domain.\n"
    );
    diff_expect_domain_error(-8.0, 1.0 / 3.0, "E2 cube root");
    diff_expect_domain_error(-27.0, 1.0 / 3.0, "E2 cube root 27");
    diff_expect_domain_error(-8.0, -1.0 / 3.0, "E2 negative cube root");
    // A family of exponents that all render identically under %.2f but are
    // distinct doubles: the message must be the same and the rejection too.
    for k in 0..64u64 {
        let exp = f64::from_bits((1.0f64 / 3.0).to_bits() + k);
        diff_expect_domain_error(-8.0, exp, "E2 %.2f-colliding exponents");
    }
}

// ---------------------------------------------------------------------------
// E3 — EDOM: exponent one ULP away from an integer
// ---------------------------------------------------------------------------

#[test]
fn e3_edom_exponent_one_ulp_off_integer() {
    for n in [1.0f64, 2.0, 3.0, 4.0, 10.0, 63.0] {
        for delta in [1i64, -1, 2, -2] {
            let bits = if delta > 0 {
                n.to_bits() + delta as u64
            } else {
                n.to_bits() - (-delta) as u64
            };
            let exp = f64::from_bits(bits);
            assert_ne!(exp, n, "must not be exactly the integer");
            // A negative base with a non-integer exponent is EDOM even when the
            // exponent is only one ULP from an integer, and %.2f still prints
            // the rounded integer.
            diff_expect_domain_error(-2.0, exp, "E3 ulp-off integer");
        }
    }
    // Confirm the neighbouring exact integers are NOT rejected, i.e. the
    // boundary really is integrality.
    for n in [1.0f64, 2.0, 3.0, 4.0, 10.0, 63.0] {
        diff_expect_clean(-2.0, n, "E3 exact integer is clean");
    }
}

// ---------------------------------------------------------------------------
// E4..E7 — ERANGE pole errors: base == +-0.0 with a negative exponent
// ---------------------------------------------------------------------------

#[test]
fn e4_erange_pole_positive_zero_negative_odd_integer() {
    for exp in [-1.0f64, -3.0, -5.0, -101.0] {
        diff_expect_range_error(0.0, exp, "E4 pole +0 odd");
    }
    let o = c_outcome(0.0, -1.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(0.00, -1.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e5_erange_pole_negative_zero_negative_odd_integer() {
    for exp in [-1.0f64, -3.0, -5.0, -101.0] {
        diff_expect_range_error(-0.0, exp, "E5 pole -0 odd");
    }
    // The sign of zero is visible in the %.2f rendering.
    let o = c_outcome(-0.0, -1.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(-0.00, -1.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e6_erange_pole_zero_negative_even_integer() {
    for base in [0.0f64, -0.0] {
        for exp in [-2.0f64, -4.0, -100.0] {
            diff_expect_range_error(base, exp, "E6 pole even");
        }
    }
    let o = c_outcome(0.0, -2.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(0.00, -2.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e7_erange_pole_zero_negative_non_integer() {
    for base in [0.0f64, -0.0] {
        for exp in [-0.5f64, -1.5, -2.5, -1.0 / 3.0, -1e18] {
            diff_expect_range_error(base, exp, "E7 pole non-integer");
        }
    }
    let o = c_outcome(0.0, -0.5);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(0.00, -0.50) caused overflow or underflow.\n"
    );

    // Randomized negative exponents against both signed zeros.
    let mut rng = Rng::new(0xE7_0007);
    for _ in 0..200 {
        let exp = -rng.log_uniform(-100.0, 100.0).abs();
        diff_expect_range_error(0.0, exp, "E7 pole randomized +0");
        diff_expect_range_error(-0.0, exp, "E7 pole randomized -0");
    }
}

// ---------------------------------------------------------------------------
// E8 / E9 / E10 — ERANGE overflow
// ---------------------------------------------------------------------------

#[test]
fn e8_erange_overflow_large_exponent() {
    for &(base, exp) in &[
        (10.0f64, 400.0f64),
        (10.0, 1e6),
        (2.0, 2000.0),
        (-2.0, 2000.0),
        (-2.0, 2001.0),
        (1e300, 2.0),
        (0.1, -400.0),
        (-0.1, -401.0),
    ] {
        diff_expect_range_error(base, exp, "E8 overflow");
    }
    let o = c_outcome(10.0, 400.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(10.00, 400.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e9_erange_overflow_dbl_max_base_prints_309_digits() {
    diff_expect_range_error(f64::MAX, 2.0, "E9 DBL_MAX^2");
    diff_expect_range_error(f64::MIN, 2.0, "E9 -DBL_MAX^2");
    diff_expect_range_error(f64::MIN, 3.0, "E9 -DBL_MAX^3");
    diff_expect_range_error(f64::MAX, 1.5, "E9 DBL_MAX^1.5");

    // The %.2f rendering of DBL_MAX is a 309-digit integer part; this exercises
    // a very different fprintf path than ordinary values.
    let o = c_outcome(f64::MAX, 2.0);
    let s = String::from_utf8_lossy(&o.stderr).to_string();
    let expected_dbl_max = "179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368.00";
    assert_eq!(
        s,
        format!("Range error: pow({expected_dbl_max}, 2.00) caused overflow or underflow.\n")
    );
    // Sanity: 309 integer digits + '.' + 2 decimals.
    assert_eq!(expected_dbl_max.len(), 309 + 3);

    let o = c_outcome(f64::MIN, 2.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        format!("Range error: pow(-{expected_dbl_max}, 2.00) caused overflow or underflow.\n")
    );
}

#[test]
fn e10_erange_overflow_subnormal_base_negative_exponent() {
    // The minimum subnormal overflows even at exponent -1.
    for base in [5e-324f64, -5e-324] {
        for exp in [-1.0f64, -2.0, -3.0, -10.0] {
            diff_expect_range_error(base, exp, "E10 min-subnormal^negative");
        }
    }
    // The LARGEST subnormal is only ~2.2e-308, so `^-1` is a finite 4.49e307 and
    // is NOT an error; overflow needs |exp| >= 2. Verified against the C.
    let max_sub = f64::from_bits(0x000F_FFFF_FFFF_FFFF);
    diff_expect_clean(max_sub, -1.0, "E10 max-subnormal^-1 is finite");
    for exp in [-2.0f64, -3.0, -10.0] {
        diff_expect_range_error(max_sub, exp, "E10 max-subnormal^negative");
    }
    // The subnormal rounds to 0.00 under %.2f, making this message textually
    // identical to E4's. That collision is the C's behaviour and must be kept.
    let subnormal = c_outcome(5e-324, -1.0);
    let zero = c_outcome(0.0, -1.0);
    assert_eq!(
        String::from_utf8_lossy(&subnormal.stderr),
        "Range error: pow(0.00, -1.00) caused overflow or underflow.\n"
    );
    assert_eq!(subnormal.stderr, zero.stderr, "the %.2f collision is expected");
}

// ---------------------------------------------------------------------------
// E11 / E12 / E13 — ERANGE underflow
// ---------------------------------------------------------------------------

#[test]
fn e11_erange_underflow_large_negative_exponent() {
    for &(base, exp) in &[
        (10.0f64, -400.0f64),
        (10.0, -1e6),
        (2.0, -2000.0),
        (-2.0, -2000.0),
        (-2.0, -2001.0),
        (0.1, 400.0),
        (-0.1, 401.0),
    ] {
        diff_expect_range_error(base, exp, "E11 underflow");
    }
    let o = c_outcome(10.0, -400.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(10.00, -400.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e12_erange_underflow_dbl_min_base() {
    diff_expect_range_error(f64::MIN_POSITIVE, 2.0, "E12 DBL_MIN^2");
    diff_expect_range_error(-f64::MIN_POSITIVE, 3.0, "E12 -DBL_MIN^3");
    diff_expect_range_error(f64::MIN_POSITIVE, 10.0, "E12 DBL_MIN^10");

    // DBL_MIN renders as 0.00 under %.2f.
    let o = c_outcome(f64::MIN_POSITIVE, 2.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(0.00, 2.00) caused overflow or underflow.\n"
    );
    let o = c_outcome(-f64::MIN_POSITIVE, 3.0);
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "Range error: pow(-0.00, 3.00) caused overflow or underflow.\n"
    );
}

#[test]
fn e13_erange_underflow_subnormal_base_positive_exponent() {
    for base in [
        5e-324f64,
        -5e-324,
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        -f64::from_bits(0x000F_FFFF_FFFF_FFFF),
    ] {
        for exp in [2.0f64, 3.0, 4.0, 1.5, 100.0] {
            if base < 0.0 && exp.fract() != 0.0 {
                // Negative base + non-integer exponent is EDOM, not ERANGE:
                // the domain check happens before any underflow can occur.
                diff_expect_domain_error(base, exp, "E13 negative subnormal, frac exp");
            } else {
                diff_expect_range_error(base, exp, "E13 subnormal^positive");
            }
        }
    }
    // Randomized subnormal bases.
    let mut rng = Rng::new(0xE13_0013);
    for _ in 0..200 {
        let base = rng.subnormal();
        let exp = rng.range(2.0, 50.0);
        // A negative base with a non-integer exponent would be EDOM instead.
        if base < 0.0 && exp.fract() != 0.0 {
            continue;
        }
        diff_expect_range_error(base, exp, "E13 subnormal randomized");
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 — one ULP either side of the overflow / underflow boundary
// ---------------------------------------------------------------------------

#[test]
fn e14_erange_overflow_boundary_straddle() {
    for base in [2.0f64, 3.0, 10.0, 1.5, 1.0000001, 1e10, 7.25] {
        let (clean, err) = bisect_range_boundary(base, 1.0, false);
        // The two exponents are adjacent doubles on opposite sides of the
        // overflow threshold; both must classify identically in C and Rust.
        diff_expect_clean(base, clean, "E14 largest non-overflowing exponent");
        diff_expect_range_error(base, err, "E14 smallest overflowing exponent");
        assert_eq!(
            err.to_bits() - clean.to_bits(),
            1,
            "boundary pair must be one ULP apart"
        );
        // And a few ULPs either side, for good measure.
        for k in 1..=8u64 {
            diff_ctx(
                base,
                f64::from_bits(clean.to_bits() - k),
                "E14 below boundary",
            );
            diff_ctx(base, f64::from_bits(err.to_bits() + k), "E14 above boundary");
        }
    }
}

#[test]
fn e15_erange_underflow_boundary_straddle() {
    for base in [2.0f64, 3.0, 10.0, 1.5, 1.0000001, 1e10, 7.25] {
        let (clean, err) = bisect_range_boundary(base, 1.0, true);
        diff_expect_clean(base, clean, "E15 largest non-underflowing exponent");
        diff_expect_range_error(base, err, "E15 smallest underflowing exponent");
        for k in 1..=8u64 {
            diff_ctx(
                base,
                -f64::from_bits((-clean).to_bits() - k),
                "E15 below boundary",
            );
            diff_ctx(
                base,
                -f64::from_bits((-err).to_bits() + k),
                "E15 above boundary",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 / E18 — the leading `errno = 0` must discard caller state
// ---------------------------------------------------------------------------

#[test]
fn e16_preset_edom_is_discarded() {
    // If a translation read errno without clearing it first, this would wrongly
    // report a domain error and return -1.
    diff_preset_errno(2.0, 10.0, EDOM, "E16 preset EDOM");
    let im = impls();
    let o = call_once(im.rust, 2.0, 10.0, EDOM, StderrMode::Capture);
    assert_eq!(o.bits, 1024.0f64.to_bits(), "must ignore the preset errno");
    assert!(o.stderr.is_empty(), "must not print: {:?}", o.stderr);
}

#[test]
fn e17_preset_erange_is_discarded() {
    diff_preset_errno(2.0, 3.0, ERANGE, "E17 preset ERANGE");
    let im = impls();
    let o = call_once(im.rust, 2.0, 3.0, ERANGE, StderrMode::Capture);
    assert_eq!(o.bits, 8.0f64.to_bits());
    assert!(o.stderr.is_empty());
}

#[test]
fn e18_preset_unrelated_errno_is_discarded() {
    for preset in [EINVAL, 1, 2, 11, 32, 35, 75, 512, -1, i32::MAX, i32::MIN] {
        diff_preset_errno(3.0, 3.0, preset, "E18 preset unrelated");
        diff_preset_errno(2.0, 0.5, preset, "E18 preset unrelated frac");
        // Also with error-producing inputs: the reported error must come from
        // this call's pow, not from the preset.
        diff_preset_errno(-2.0, 0.5, preset, "E18 preset + edom input");
        diff_preset_errno(10.0, 400.0, preset, "E18 preset + erange input");
    }
}

// ---------------------------------------------------------------------------
// E19 — `-1.0` is a legal result, not a sentinel
// ---------------------------------------------------------------------------

#[test]
fn e19_legitimate_minus_one_is_not_a_rejection() {
    for &(base, exp) in &[
        (-1.0f64, 3.0f64),
        (-1.0, 1.0),
        (-1.0, -1.0),
        (-1.0, -3.0),
        (-1.0, 101.0),
        (-1.0, -101.0),
    ] {
        diff_expect_clean(base, exp, "E19 legit -1.0");
        let o = c_outcome(base, exp);
        assert_eq!(
            o.bits,
            (-1.0f64).to_bits(),
            "expected a legitimate -1.0 result"
        );
        assert!(
            o.stderr.is_empty(),
            "a legitimate -1.0 must not print an error"
        );
        assert_eq!(o.errno, 0);
    }
}

// ---------------------------------------------------------------------------
// E20 — IEEE specials set no errno and take neither branch
// ---------------------------------------------------------------------------

#[test]
fn e20_ieee_specials_take_neither_branch() {
    for &(base, exp) in &[
        (QNAN, 2.0f64),
        (2.0, QNAN),
        (1.0, QNAN),
        (QNAN, 0.0),
        (QNAN, -0.0),
        (QNAN, QNAN),
        (SNAN, 2.0),
        (2.0, SNAN),
        (NEG_QNAN, 2.0),
        (INF, 2.0),
        (INF, -2.0),
        (-INF, 3.0),
        (-INF, 2.0),
        (-INF, -3.0),
        (-INF, -2.0),
        (2.0, INF),
        (2.0, -INF),
        (0.5, INF),
        (0.5, -INF),
        (-1.0, INF),
        (-1.0, -INF),
        (1.0, INF),
        (0.0, 3.0),
        (-0.0, 3.0),
        (0.0, 0.0),
        (INF, INF),
        (-INF, INF),
        (INF, -INF),
        (-INF, -INF),
    ] {
        diff_expect_clean(base, exp, "E20 ieee special");
    }
}

// ---------------------------------------------------------------------------
// E21 — unwritable stderr must not change behaviour
// ---------------------------------------------------------------------------

#[test]
fn e21_stderr_write_failure_does_not_change_behaviour() {
    // /dev/full: every write fails with ENOSPC. The C ignores fprintf's return
    // value, so the return value must be unaffected and nothing may abort.
    for &(base, exp) in &[
        (-2.0f64, 0.5f64),
        (10.0, 400.0),
        (0.0, -1.0),
        (2.0, 10.0),
        (f64::MAX, 2.0),
    ] {
        diff_stderr_mode(base, exp, StderrMode::Full, "E21 /dev/full");
    }

    // fd 2 closed entirely: every write fails with EBADF.
    for &(base, exp) in &[
        (-2.0f64, 0.5f64),
        (10.0, 400.0),
        (0.0, -1.0),
        (2.0, 10.0),
        (f64::MAX, 2.0),
    ] {
        diff_stderr_mode(base, exp, StderrMode::Closed, "E21 fd2 closed");
    }

    // Behaviour must still be normal afterwards (no latched error state on the
    // shared stderr FILE* that would desynchronise the two implementations).
    diff_expect_domain_error(-2.0, 0.5, "E21 recovery");
    diff_expect_clean(2.0, 10.0, "E21 recovery clean");
}

// ---------------------------------------------------------------------------
// E22 — randomized branch-classification agreement, with coverage proof
// ---------------------------------------------------------------------------

#[test]
fn e22_randomized_branch_classification_agrees() {
    let mut rng = Rng::new(0xE22_0022);
    let mut n_edom = 0usize;
    let mut n_erange = 0usize;
    let mut n_clean = 0usize;

    for i in 0..1500 {
        // Mix of generators so all three branches are reached.
        let (base, exp) = match i % 5 {
            0 => (rng.log_uniform(-320.0, 320.0), rng.range(-400.0, 400.0)),
            1 => (rng.log_uniform(-10.0, 10.0), rng.range(-20.0, 20.0)),
            2 => (
                -rng.log_uniform(-50.0, 50.0).abs(),
                rng.range(-10.0, 10.0),
            ),
            3 => (rng.any_f64(), rng.any_f64()),
            _ => (rng.log_uniform(-3.0, 3.0), rng.log_uniform(0.0, 4.0)),
        };

        // Compare, then record which branch the C actually chose.
        diff_ctx(base, exp, "E22 randomized");
        match c_outcome(base, exp).errno {
            e if e == EDOM => n_edom += 1,
            e if e == ERANGE => n_erange += 1,
            _ => n_clean += 1,
        }
    }

    // Guard against the row passing vacuously: every branch must be exercised.
    assert!(n_edom > 0, "randomized sweep never hit the EDOM branch");
    assert!(n_erange > 0, "randomized sweep never hit the ERANGE branch");
    assert!(n_clean > 0, "randomized sweep never hit the clean path");
    eprintln!("E22 branch coverage: EDOM={n_edom} ERANGE={n_erange} clean={n_clean}");
}

// ---------------------------------------------------------------------------
// Generic boundary sweep: every IEEE class against every IEEE class.
// (This is the analogue of "out-of-range enum values" for a float-only API:
//  an f64 parameter accepts all 2^64 bit patterns.)
// ---------------------------------------------------------------------------

#[test]
fn generic_boundary_full_class_cross_product() {
    for (bn, base) in base_classes() {
        for (en, exp) in exponent_classes() {
            diff_ctx(base, exp, &format!("class cross-product base={bn} exp={en}"));
        }
    }
}
