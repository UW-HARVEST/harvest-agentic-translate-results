//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test constructs the exact invalid input/condition, calls BOTH `.so`s and
//! asserts they return the SAME sentinel (`-1.0`, the only error indication the
//! C API has) AND emit byte-identical diagnostics on stderr.

mod common;

use common::*;

/// The C sentinel: `return -1;` from a `double` function.
const SENTINEL: f64 = -1.0;

/// Assert both libraries return the sentinel and produce identical stderr.
#[track_caller]
fn check_error_row(row: &str, base: f64, exponent: f64, expect_prefix: &str) {
    let l = libs();

    // Capture each library's diagnostic separately so we can compare bytes.
    let mut c_ret = 0.0f64;
    let c_err = capture_stderr(|| {
        c_ret = unsafe { (l.c_pow)(base, exponent) };
    });
    let mut r_ret = 0.0f64;
    let r_err = capture_stderr(|| {
        r_ret = unsafe { (l.r_pow)(base, exponent) };
    });

    assert_eq!(
        c_ret.to_bits(),
        SENTINEL.to_bits(),
        "{row}: expected the C implementation to return the -1.0 sentinel for \
         my_pow({base:e}, {exponent:e}), got {c_ret:e}"
    );
    assert_eq!(
        r_ret.to_bits(),
        c_ret.to_bits(),
        "{row}: return value diverged for my_pow({base:e}, {exponent:e}): \
         C={c_ret:e} [{:#018x}] RUST={r_ret:e} [{:#018x}]",
        c_ret.to_bits(),
        r_ret.to_bits()
    );

    assert!(
        !c_err.is_empty(),
        "{row}: expected the C implementation to write a diagnostic to stderr"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_err),
        String::from_utf8_lossy(&r_err),
        "{row}: stderr diagnostic diverged for my_pow({base:e}, {exponent:e})\n\
         C   ({} bytes)\nRUST ({} bytes)",
        c_err.len(),
        r_err.len()
    );
    assert_eq!(
        c_err, r_err,
        "{row}: stderr bytes diverged for my_pow({base:e}, {exponent:e})"
    );

    let text = String::from_utf8_lossy(&c_err);
    assert!(
        text.starts_with(expect_prefix),
        "{row}: expected diagnostic to start with {expect_prefix:?}, got {:?}",
        &text[..text.len().min(120)]
    );
}

const DOMAIN_PREFIX: &str = "Domain error: pow(";
const RANGE_PREFIX: &str = "Range error: pow(";

// ---------------------------------------------------------------------------
// E1 — EDOM: negative base, non-integral exponent
// ---------------------------------------------------------------------------
#[test]
fn err_e1_edom_negative_base_fractional_exponent() {
    check_error_row("E1", -2.0, 0.5, DOMAIN_PREFIX);

    // Exact message the C produces, spelled out, to pin the format string.
    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(-2.0, 0.5) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n"
    );

    // Randomized EDOM triggers: any negative base with a non-integral exponent.
    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut pairs = Vec::new();
    for _ in 0..3000 {
        let base = -rng.range(f64::MIN_POSITIVE, 1e10);
        let frac = rng.range(0.05, 0.95);
        let exponent = rng.int_range(-50, 50) as f64 + frac;
        pairs.push((base, exponent));
    }
    check_pairs("E1 randomized", &pairs);
    assert_all_return_sentinel("E1 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E2 — EDOM: |negative base| > 1 with non-integral exponent > 1
// ---------------------------------------------------------------------------
#[test]
fn err_e2_edom_negative_base_large_fractional_exponent() {
    check_error_row("E2", -1.5, 2.5, DOMAIN_PREFIX);

    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(-1.5, 2.5) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Domain error: pow(-1.50, 2.50) is undefined in the real number domain.\n"
    );

    let mut rng = Rng::new(SEED ^ 0xE2);
    let mut pairs = Vec::new();
    for _ in 0..3000 {
        let base = -rng.range(1.0 + f64::EPSILON, 1000.0);
        let exponent = rng.range(1.0, 20.0) + 0.5;
        pairs.push((base, exponent));
    }
    check_pairs("E2 randomized", &pairs);
    assert_all_return_sentinel("E2 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E3 — ERANGE: pole error, base == +0.0, negative exponent
// ---------------------------------------------------------------------------
#[test]
fn err_e3_erange_pole_positive_zero() {
    check_error_row("E3", 0.0, -1.0, RANGE_PREFIX);

    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(0.0, -1.0) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Range error: pow(0.00, -1.00) caused overflow or underflow.\n"
    );

    let mut pairs = Vec::new();
    for e in 1i64..=200 {
        pairs.push((0.0f64, -(e as f64))); // even and odd
    }
    let mut rng = Rng::new(SEED ^ 0xE3);
    for _ in 0..2000 {
        pairs.push((0.0f64, -rng.range(f64::MIN_POSITIVE, 1e6)));
        pairs.push((0.0f64, -(rng.int_range(1, 1000) as f64)));
    }
    pairs.push((0.0, f64::NEG_INFINITY));
    check_pairs("E3 randomized", &pairs);
    // pow(+0, -inf) is +inf but errno is NOT set for it in every libm; only
    // assert the sentinel for the finite negative exponents.
    let finite: Vec<(f64, f64)> = pairs.iter().copied().filter(|p| p.1.is_finite()).collect();
    assert_all_return_sentinel("E3 randomized", &finite);
}

// ---------------------------------------------------------------------------
// E4 — ERANGE: pole error, base == -0.0, negative odd-integer exponent
// ---------------------------------------------------------------------------
#[test]
fn err_e4_erange_pole_negative_zero() {
    check_error_row("E4", -0.0, -1.0, RANGE_PREFIX);

    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(-0.0, -1.0) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Range error: pow(-0.00, -1.00) caused overflow or underflow.\n"
    );

    let mut pairs = Vec::new();
    for e in 1i64..=200 {
        pairs.push((-0.0f64, -(e as f64)));
    }
    let mut rng = Rng::new(SEED ^ 0xE4);
    for _ in 0..2000 {
        // odd integers exercise the -Inf result, even integers the +Inf result
        let mut n = rng.int_range(1, 999);
        if rng.bool() && n % 2 == 0 {
            n += 1;
        }
        pairs.push((-0.0f64, -(n as f64)));
        pairs.push((-0.0f64, -rng.range(f64::MIN_POSITIVE, 1e6)));
    }
    check_pairs("E4 randomized", &pairs);
    assert_all_return_sentinel("E4 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E5 — ERANGE: pole error with negative NON-INTEGRAL exponent
// ---------------------------------------------------------------------------
#[test]
fn err_e5_erange_pole_fractional_exponent() {
    check_error_row("E5", 0.0, -0.5, RANGE_PREFIX);

    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(0.0, -0.5) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Range error: pow(0.00, -0.50) caused overflow or underflow.\n"
    );

    let mut rng = Rng::new(SEED ^ 0xE5);
    let mut pairs = Vec::new();
    for _ in 0..2000 {
        let frac = rng.range(0.05, 0.95);
        let exponent = -(rng.int_range(0, 100) as f64 + frac);
        pairs.push((0.0f64, exponent));
        pairs.push((-0.0f64, exponent));
    }
    check_pairs("E5 randomized", &pairs);
    assert_all_return_sentinel("E5 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E6 — ERANGE: overflow (incl. the 309-digit %.2f expansion)
// ---------------------------------------------------------------------------
#[test]
fn err_e6_erange_overflow() {
    check_error_row("E6", 1e300, 2.0, RANGE_PREFIX);
    check_error_row("E6b", 2.0, 10000.0, RANGE_PREFIX);

    let l = libs();
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(2.0, 10000.0) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Range error: pow(2.00, 10000.00) caused overflow or underflow.\n"
    );

    // The 1e300 case makes fprintf print ~309 integer digits; the C and Rust
    // byte streams must match exactly. check_error_row already compared them,
    // assert the shape here.
    let big = capture_stderr(|| {
        unsafe { (l.c_pow)(1e300, 2.0) };
    });
    let big_r = capture_stderr(|| {
        unsafe { (l.r_pow)(1e300, 2.0) };
    });
    assert_eq!(big, big_r, "E6: 309-digit %.2f expansion diverged");
    assert!(
        big.len() > 300,
        "E6: expected a long %.2f expansion, got {} bytes",
        big.len()
    );

    let mut rng = Rng::new(SEED ^ 0xE6);
    let mut pairs = Vec::new();
    for _ in 0..2000 {
        let base = rng.log_uniform(150.0, 308.0, false);
        let exponent = rng.range(2.0, 100.0);
        pairs.push((base, exponent));
        pairs.push((rng.range(1.1, 100.0), rng.range(1e4, 1e6)));
    }
    check_pairs("E6 randomized", &pairs);
    assert_all_return_sentinel("E6 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E7 — ERANGE: underflow
// ---------------------------------------------------------------------------
#[test]
fn err_e7_erange_underflow() {
    check_error_row("E7", 1e-300, 2.0, RANGE_PREFIX);
    check_error_row("E7b", 2.0, -10000.0, RANGE_PREFIX);

    let l = libs();
    // %.2f of 1e-300 prints as "0.00" -- the C does exactly this, so the Rust
    // must too.
    let msg = capture_stderr(|| {
        unsafe { (l.r_pow)(1e-300, 2.0) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg),
        "Range error: pow(0.00, 2.00) caused overflow or underflow.\n"
    );
    let msg2 = capture_stderr(|| {
        unsafe { (l.r_pow)(2.0, -10000.0) };
    });
    assert_eq!(
        String::from_utf8_lossy(&msg2),
        "Range error: pow(2.00, -10000.00) caused overflow or underflow.\n"
    );

    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut pairs = Vec::new();
    for _ in 0..2000 {
        let base = rng.log_uniform(-308.0, -150.0, false);
        pairs.push((base, rng.range(2.0, 100.0)));
        pairs.push((rng.range(1.1, 100.0), -rng.range(1e4, 1e6)));
    }
    check_pairs("E7 randomized", &pairs);
    assert_all_return_sentinel("E7 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// E8 — ERANGE: overflow with a NEGATIVE result (-Inf)
// ---------------------------------------------------------------------------
#[test]
fn err_e8_erange_overflow_negative() {
    check_error_row("E8", -1e300, 3.0, RANGE_PREFIX);

    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut pairs = Vec::new();
    for _ in 0..2000 {
        let base = -rng.log_uniform(150.0, 308.0, false);
        // odd integral exponent -> -Inf
        let mut n = rng.int_range(3, 99);
        if n % 2 == 0 {
            n += 1;
        }
        pairs.push((base, n as f64));
        // even integral exponent -> +Inf
        let mut m = rng.int_range(2, 98);
        if m % 2 != 0 {
            m += 1;
        }
        pairs.push((base, m as f64));
    }
    check_pairs("E8 randomized", &pairs);
    assert_all_return_sentinel("E8 randomized", &pairs);
}

// ---------------------------------------------------------------------------
// Generic FFI boundary coverage (ERRORS.md rows B1..B7)
// ---------------------------------------------------------------------------

/// B1/B2/B3/B4 — non-finite inputs are VALID for this API: `pow(NaN, 0) == 1`,
/// `pow(-1, Inf) == 1`, `pow(0, 0) == 1`, all with `errno == 0`, so the C must
/// NOT return the sentinel. Verify C and Rust agree, including on that.
#[test]
fn bnd_b1_b4_non_finite_inputs_are_not_rejected() {
    let l = libs();
    let cases: &[(f64, f64, f64)] = &[
        (f64::NAN, 0.0, 1.0),
        (f64::NAN, -0.0, 1.0),
        (1.0, f64::NAN, 1.0),
        (-1.0, f64::INFINITY, 1.0),
        (-1.0, f64::NEG_INFINITY, 1.0),
        (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        (0.0, 0.0, 1.0),
        (-0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0),
        (-0.0, 3.0, -0.0),
    ];
    let _q = quiet();
    for &(b, e, expected) in cases {
        let c = unsafe { (l.c_pow)(b, e) };
        let r = unsafe { (l.r_pow)(b, e) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "B1..B4: my_pow({b:e}, {e:e}) diverged: C={c:e} RUST={r:e}"
        );
        assert_eq!(
            c.to_bits(),
            expected.to_bits(),
            "B1..B4: my_pow({b:e}, {e:e}) should be {expected:e} (not the \
             error sentinel), got {c:e}"
        );
    }
}

/// B2 — NaN payloads, including signalling NaNs, must propagate identically.
#[test]
fn bnd_b2_nan_payload_propagation() {
    let mut pairs = Vec::new();
    for &nb in NAN_BITS {
        let n = f64::from_bits(nb);
        for &e in SPECIALS {
            pairs.push((n, e));
            pairs.push((e, n));
        }
    }
    check_pairs("B2", &pairs);
}

/// B3 — all 9 combinations of {-Inf, +Inf, finite} x {-Inf, +Inf, finite}.
#[test]
fn bnd_b3_infinity_cross_product() {
    let vals = [
        f64::NEG_INFINITY,
        f64::INFINITY,
        -2.0,
        2.0,
        -0.5,
        0.5,
        0.0,
        -0.0,
        1.0,
        -1.0,
    ];
    let mut pairs = Vec::new();
    for &a in &vals {
        for &b in &vals {
            pairs.push((a, b));
        }
    }
    check_pairs("B3", &pairs);
}

/// B5 — DBL_MAX / DBL_MIN / subnormal / EPSILON one step past the range.
#[test]
fn bnd_b5_representable_range_edges() {
    let edges = [
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::EPSILON,
        -f64::EPSILON,
        1.0 + f64::EPSILON,
        1.0 - f64::EPSILON / 2.0,
        -(1.0 + f64::EPSILON),
    ];
    let mut pairs = Vec::new();
    for &a in &edges {
        for &b in &edges {
            pairs.push((a, b));
        }
        for &b in &[0.0f64, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, f64::INFINITY] {
            pairs.push((a, b));
            pairs.push((b, a));
        }
    }
    check_pairs("B5", &pairs);
}

/// B6 — one step past the ERANGE boundary: the value just inside must return
/// the real result, the value just outside must return the -1.0 sentinel.
/// Both libraries must flip at the SAME input.
#[test]
fn bnd_b6_one_step_past_erange_boundary() {
    let l = libs();
    let mut flips_c: Vec<(u64, bool)> = Vec::new();
    let mut flips_r: Vec<(u64, bool)> = Vec::new();
    {
        let _q = quiet();
        for &base in &[2.0f64, 10.0, 3.0, 1.5, 0.5, 0.1] {
            let thresh = f64::MAX.ln() / base.abs().ln();
            let mut e = thresh - 2.0;
            for _ in 0..400 {
                let c = unsafe { (l.c_pow)(base, e) };
                let r = unsafe { (l.r_pow)(base, e) };
                flips_c.push((e.to_bits(), c.to_bits() == (-1.0f64).to_bits()));
                flips_r.push((e.to_bits(), r.to_bits() == (-1.0f64).to_bits()));
                e = f64::from_bits(e.to_bits() + 1);
            }
        }
    }
    assert_eq!(
        flips_c, flips_r,
        "B6: the C and Rust implementations reject at different inputs around \
         the ERANGE overflow boundary"
    );
    // Sanity: the sweep must actually contain both outcomes, otherwise the test
    // is not exercising the boundary at all.
    assert!(
        flips_c.iter().any(|&(_, rejected)| rejected),
        "B6: sweep never hit the ERANGE branch"
    );
    assert!(
        flips_c.iter().any(|&(_, rejected)| !rejected),
        "B6: sweep never hit the success branch"
    );
}

/// B7 — errno hygiene: a stale `errno` must not cause a spurious rejection,
/// because the C body resets `errno = 0` first (pow.c line 34).
#[test]
fn bnd_b7_stale_errno_does_not_cause_spurious_rejection() {
    let l = libs();
    let _q = quiet();
    for &stale in &[0, EDOM, ERANGE, 1, 22, 9999, -1, i32::MAX, i32::MIN] {
        for &(b, e) in &[(2.0f64, 10.0f64), (9.0, 0.5), (-2.0, 3.0), (1.0, 1.0)] {
            errno_set(stale);
            let c = unsafe { (l.c_pow)(b, e) };
            errno_set(stale);
            let r = unsafe { (l.r_pow)(b, e) };
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "B7: my_pow({b}, {e}) with stale errno={stale} diverged: \
                 C={c:e} RUST={r:e}"
            );
            assert_ne!(
                c.to_bits(),
                (-1.0f64).to_bits(),
                "B7: my_pow({b}, {e}) must not be rejected just because \
                 errno was {stale} on entry"
            );
        }
    }
}

/// `errno` side-effect parity. The return value is not the only observable
/// output: `my_pow` leaves `errno` set as libm left it, and a caller may read
/// it. Assert the C and Rust `.so` leave the SAME `errno` for the same input,
/// and that it is always one of {0, EDOM, ERANGE} -- the property that makes
/// `err == EDOM` equivalent to `err != 0 && err != ERANGE` (see
/// .verif/mutate.sh).
#[test]
fn bnd_errno_side_effect_parity() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE0E0);
    let mut bad: Vec<String> = Vec::new();
    let mut observed = std::collections::BTreeSet::new();
    {
        let _q = quiet();
        for i in 0..60_000u32 {
            // Mix full-bitspace fuzz with integral exponents spanning the
            // over/underflow band, so all three errno outcomes occur.
            let (b, e) = match i % 3 {
                0 => (rng.any_f64(), rng.any_f64()),
                1 => (
                    rng.int_range(-1000, 1000) as f64,
                    rng.int_range(-2000, 2000) as f64,
                ),
                _ => (rng.any_f64(), rng.int_range(-1100, 1100) as f64),
            };
            errno_set(12345);
            let c = unsafe { (l.c_pow)(b, e) };
            let c_errno = errno_get();
            errno_set(12345);
            let r = unsafe { (l.r_pow)(b, e) };
            let r_errno = errno_get();

            observed.insert(c_errno);
            if c_errno != r_errno && bad.len() < 15 {
                bad.push(format!(
                    "my_pow({b:e}, {e:e}) left errno C={c_errno} RUST={r_errno}"
                ));
            }
            if c.to_bits() != r.to_bits() && bad.len() < 15 {
                bad.push(format!("my_pow({b:e}, {e:e}) returned C={c:e} RUST={r:e}"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "errno / return-value side effects diverged:\n{}",
        bad.join("\n")
    );
    // The reachable-errno claim the equivalence argument depends on.
    let unexpected: Vec<i32> = observed
        .iter()
        .copied()
        .filter(|&e| e != 0 && e != EDOM && e != ERANGE)
        .collect();
    assert!(
        unexpected.is_empty(),
        "pow() set an errno outside {{0, EDOM, ERANGE}}: {unexpected:?}. The \
         equivalence argument in .verif/mutate.sh must be revisited."
    );
    // And the fuzz must actually have reached all three outcomes, else the test
    // proves nothing.
    for want in [0, EDOM, ERANGE] {
        assert!(
            observed.contains(&want),
            "fuzz never produced errno={want}; observed {observed:?}"
        );
    }
}

/// No pointer/length/enum parameter exists in this API, so the classic
/// null-pointer and out-of-range-enum cases are structurally impossible.
/// This test documents that mechanically from the built C `.so`.
#[test]
fn bnd_no_pointer_or_enum_parameters_in_api() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/pow.h"),
    )
    .expect("read pow.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .collect();
    assert_eq!(
        decls,
        vec!["double my_pow(double base, double exponent);"],
        "pow.h declares more than the single known entry point; the error \
         surface in ERRORS.md must be re-derived"
    );
    assert!(
        !header.contains('*') || !header.contains("my_pow("),
        "unexpected pointer in the public API"
    );
    assert!(!header.contains("enum"), "unexpected enum in the public API");
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Assert every pair makes BOTH libraries return the `-1.0` sentinel, i.e. that
/// the inputs really do hit an error branch (guards against a "test" that only
/// compares two identical success paths).
#[track_caller]
fn assert_all_return_sentinel(ctx: &str, pairs: &[(f64, f64)]) {
    let l = libs();
    let mut bad = Vec::new();
    {
        let _q = quiet();
        for &(b, e) in pairs {
            let c = unsafe { (l.c_pow)(b, e) };
            let r = unsafe { (l.r_pow)(b, e) };
            if c.to_bits() != SENTINEL.to_bits() || r.to_bits() != SENTINEL.to_bits() {
                if bad.len() < 10 {
                    bad.push(format!(
                        "my_pow({b:e}, {e:e}) -> C={c:e} RUST={r:e} (expected -1.0)"
                    ));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{ctx}: {} input(s) did not hit the expected error branch:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
