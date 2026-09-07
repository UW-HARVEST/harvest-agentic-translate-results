// Phase C — error-path differential tests, one test per ERRORS.md row.
//
// Each test constructs the exact condition that makes glibc `pow` set `errno`,
// drives BOTH shared objects, and asserts:
//   * they agree byte-for-byte (return bits, caller-visible errno, stderr bytes), AND
//   * the C really did take the intended rejection branch — i.e. it returned the
//     `-1.0` sentinel AND emitted the specific diagnostic. This second assertion
//     is what stops a row from passing vacuously because neither side errored.

mod common;

use common::*;

const EDOM: i32 = 33;
const ERANGE: i32 = 34;

const DOMAIN_MSG_PREFIX: &str = "Domain error: pow(";
const DOMAIN_MSG_SUFFIX: &str = ") is undefined in the real number domain.\n";
const RANGE_MSG_PREFIX: &str = "Range error: pow(";
const RANGE_MSG_SUFFIX: &str = ") caused overflow or underflow.\n";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Expect {
    Domain,
    Range,
    NoError,
}

/// Differential-check every case AND assert the C took the expected branch.
fn check_rows(label: &str, expect: Expect, cases: &[(f64, f64)]) {
    let p = Pair::load();
    let mut cap = StderrCapture::new();

    let mut diffs = Vec::new();
    let mut branch_problems: Vec<String> = Vec::new();

    for &(base, exp) in cases {
        match diff_once(&mut cap, &p, base, exp, 0) {
            Some(d) => {
                if diffs.len() < 25 {
                    diffs.push(d);
                }
                continue;
            }
            None => {}
        }
        // Both agreed; now verify the branch actually taken by the C.
        let o = observe(&mut cap, p.c, base, exp, 0);
        let text = String::from_utf8_lossy(&o.stderr).to_string();
        let ok = match expect {
            Expect::Domain => {
                o.bits == (-1.0f64).to_bits()
                    && o.errno == EDOM
                    && text.starts_with(DOMAIN_MSG_PREFIX)
                    && text.ends_with(DOMAIN_MSG_SUFFIX)
            }
            Expect::Range => {
                o.bits == (-1.0f64).to_bits()
                    && o.errno == ERANGE
                    && text.starts_with(RANGE_MSG_PREFIX)
                    && text.ends_with(RANGE_MSG_SUFFIX)
            }
            Expect::NoError => o.errno == 0 && o.stderr.is_empty(),
        };
        if !ok && branch_problems.len() < 25 {
            branch_problems.push(format!(
                "my_pow({:?} [{:#018x}], {:?} [{:#018x}]) expected {:?} branch but C gave \
                 bits={:#018x} ({:?}) errno={} stderr={:?}",
                base,
                base.to_bits(),
                exp,
                exp.to_bits(),
                expect,
                o.bits,
                f64::from_bits(o.bits),
                o.errno,
                text
            ));
        }
    }

    drop(cap); // restore fd 2 before reporting

    if !diffs.is_empty() {
        let mut msg = format!("[{}] C/Rust divergence:\n", label);
        for d in &diffs {
            msg.push_str(&format!("  - {}\n", d));
        }
        panic!("{}", msg);
    }
    if !branch_problems.is_empty() {
        let mut msg = format!(
            "[{}] both libraries agreed, but the C did NOT take the {:?} branch \
             (the row would have passed vacuously):\n",
            label, expect
        );
        for b in &branch_problems {
            msg.push_str(&format!("  - {}\n", b));
        }
        panic!("{}", msg);
    }
    assert!(!cases.is_empty(), "[{}] no cases", label);
    println!(
        "[{}] {} cases matched, all took the {:?} branch",
        label,
        cases.len(),
        expect
    );
}

// ===========================================================================
// E1 — EDOM: negative finite base, non-integral finite exponent (e.g. ±0.5)
// ===========================================================================
#[test]
fn e01_edom_neg_base_half_integral_exp() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-2.0, 0.5),
        (-2.0, -0.5),
        (-1.0, 0.5),
        (-1.0, -0.5),
        (-4.0, 1.5),
        (-9.0, 2.5),
        (-16.0, -3.5),
    ];
    for _ in 0..2000 {
        let base = -r.range(0.001, 1e6);
        let e = (r.below(41) as f64 - 20.0) + 0.5;
        cases.push((base, e));
    }
    check_rows("E1 EDOM neg base half-int exp", Expect::Domain, &cases);
}

// ===========================================================================
// E2 — EDOM: negative base with general non-integral exponent
// ===========================================================================
#[test]
fn e02_edom_neg_base_general_nonintegral_exp() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-1.5, 1.5),
        (-3.0, 2.7),
        (-0.5, -1.3),
        (-2.0, 0.1),
        (-2.0, 3.999999),
        (-1.0000001, 0.7),
        (-0.9999999, -0.7),
        (-f64::MIN_POSITIVE, 0.25),
        (f64::from_bits(0x8000_0000_0000_0001), 0.25), // negative subnormal
        (-f64::MAX, 0.25),
        (-1.0, 1e-300),  // tiny non-integral exponent
        (-1.0, -1e-300),
    ];
    for _ in 0..3000 {
        let base = -r.range(1e-6, 1e6);
        let mut e = r.range(-30.0, 30.0);
        if e.fract() == 0.0 {
            e += 0.123;
        }
        // keep the result in range so it is EDOM (not ERANGE)
        if base.abs().ln() * e > 700.0 || base.abs().ln() * e < -700.0 {
            e = 0.5;
        }
        cases.push((base, e));
    }
    check_rows("E2 EDOM neg base non-int exp", Expect::Domain, &cases);
}

// ===========================================================================
// E3 — ERANGE: overflow to +Inf from a large base
// ===========================================================================
#[test]
fn e03_erange_overflow_to_plus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (1e300, 3.0),
        (2.0, 100000.0),
        (10.0, 400.0),
        (1e300, 2.0),
        (f64::MAX, 2.0),
        (f64::MAX, 1.5),
        (2.0, 1024.0),
        (2.0, f64::MAX),
        (1e300, f64::INFINITY.min(1e10)),
    ];
    for _ in 0..2000 {
        let base = r.range(1.1, 1e10);
        let e = r.range(400.0, 1e6) / base.ln() * 800.0; // comfortably overflowing
        cases.push((base, e.max(1e4)));
    }
    for _ in 0..1000 {
        cases.push((r.range(1e200, 1e308), r.range(2.0, 100.0)));
    }
    check_rows("E3 ERANGE overflow +inf", Expect::Range, &cases);
}

// ===========================================================================
// E4 — ERANGE: overflow just one step past DBL_MAX
// ===========================================================================
#[test]
fn e04_erange_overflow_one_step_past_dbl_max() {
    let mut cases: Vec<(f64, f64)> = vec![
        (1.7976931348623157e308, 1.0000001),
        (f64::MAX, 1.0000000000001),
        (f64::MAX, 1.0 + f64::EPSILON),
        (f64::MAX, 1.0 + 2.0 * f64::EPSILON),
        (2.0, 1024.0),
        (-f64::MAX, 3.0),
        (-2.0, 1024.0),
    ];
    // Sweep the immediate neighbourhood *above* the 2^1024 overflow threshold.
    for k in 0..64u64 {
        let e = 1024.0 + (k as f64) * 1e-12;
        cases.push((2.0, e));
    }
    check_rows(
        "E4 ERANGE overflow one step past DBL_MAX",
        Expect::Range,
        &cases,
    );
}

// ===========================================================================
// E4-inverse — ONE STEP INSIDE the overflow threshold: must NOT error.
// (The mirror of E4; `2.0^1023.9999999` is still finite, so glibc leaves
// `errno` at 0 and the C must return the value, not the -1.0 sentinel.)
// ===========================================================================
#[test]
fn e04b_one_step_inside_overflow_threshold_no_error() {
    let mut cases: Vec<(f64, f64)> = vec![
        (2.0, 1023.9999999),
        (2.0, 1023.0),
        (2.0, 1023.5),
        // NB `(-2.0, 1023.9999999)` belongs to E2, not here: a negative base with
        // a non-integral exponent is EDOM regardless of the result's magnitude.
        (-2.0, 1023.0),
        (f64::MAX, 1.0),
        (f64::MAX, 0.9999999),
        (f64::MIN, 1.0),
    ];
    // Just below 2^1024, walking down towards it.
    for k in 1..64u64 {
        cases.push((2.0, 1024.0 - (k as f64) * 1e-9));
    }
    check_rows(
        "E4b one step inside overflow threshold (no error)",
        Expect::NoError,
        &cases,
    );
}

// ===========================================================================
// E5 — ERANGE: overflow to -Inf (negative base, odd integral exponent)
// ===========================================================================
#[test]
fn e05_erange_overflow_to_minus_inf() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-1e300, 3.0),
        (-1e300, 5.0),
        (-f64::MAX, 3.0),
        (-2.0, 100001.0),
        (-10.0, 401.0),
    ];
    for _ in 0..1000 {
        let base = -r.range(1e200, 1e308);
        let e = (r.below(20) * 2 + 3) as f64; // odd >= 3
        cases.push((base, e));
    }
    check_rows("E5 ERANGE overflow -inf", Expect::Range, &cases);
}

// ===========================================================================
// E6 — ERANGE: underflow flushing to +0
// ===========================================================================
#[test]
fn e06_erange_underflow_to_plus_zero() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (1e-300, 3.0),
        (2.0, -100000.0),
        (1e-300, 2.0),
        (0.5, 2000.0),
        (10.0, -400.0),
        (f64::MIN_POSITIVE, 3.0),
        (2.0, -1075.0),
        (2.0, -2000.0),
    ];
    for _ in 0..2000 {
        let base = r.range(1e-300, 0.5);
        let e = r.range(50.0, 5000.0);
        // ensure real underflow to zero
        if base.ln() * e < -760.0 {
            cases.push((base, e));
        } else {
            cases.push((base, 5000.0));
        }
    }
    check_rows("E6 ERANGE underflow +0", Expect::Range, &cases);
}

// ===========================================================================
// E7 — ERANGE: underflow with a non-integral exponent
// ===========================================================================
#[test]
fn e07_erange_underflow_nonintegral_exp() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (1e-300, 1.2),
        (1e-300, 3.7),
        (0.5, 2500.5),
        (1e-100, 11.5),
        (2.0, -1100.5),
    ];
    for _ in 0..2000 {
        let base = r.range(1e-300, 0.4);
        let mut e = r.range(60.0, 5000.0);
        if e.fract() == 0.0 {
            e += 0.5;
        }
        if base.ln() * e < -760.0 {
            cases.push((base, e));
        } else {
            cases.push((1e-300, 3.5));
        }
    }
    check_rows("E7 ERANGE underflow non-int exp", Expect::Range, &cases);
}

// ===========================================================================
// E8 — ERANGE: underflow to -0 (negative base, odd integral exponent)
// ===========================================================================
#[test]
fn e08_erange_underflow_to_minus_zero() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-1e-300, 3.0),
        (-1e-300, 5.0),
        (-0.5, 2001.0),
        (-2.0, -1075.0),
        (-2.0, -2001.0),
        (-f64::MIN_POSITIVE, 3.0),
    ];
    for _ in 0..1000 {
        let base = -r.range(1e-300, 1e-200);
        let e = (r.below(20) * 2 + 5) as f64; // odd >= 5
        cases.push((base, e));
    }
    check_rows("E8 ERANGE underflow -0", Expect::Range, &cases);
}

// ===========================================================================
// E9 — ERANGE: pole, +0.0 base with negative ODD integral exponent
// ===========================================================================
#[test]
fn e09_erange_pole_plus_zero_neg_odd_int() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![(0.0, -1.0), (0.0, -3.0), (0.0, -5.0), (0.0, -7.0)];
    for _ in 0..500 {
        cases.push((0.0, -((r.below(200) * 2 + 1) as f64)));
    }
    check_rows("E9 ERANGE pole +0 / -odd int", Expect::Range, &cases);
}

// ===========================================================================
// E10 — ERANGE: pole, +0.0 base with negative EVEN integral exponent
// ===========================================================================
#[test]
fn e10_erange_pole_plus_zero_neg_even_int() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![(0.0, -2.0), (0.0, -4.0), (0.0, -6.0)];
    for _ in 0..500 {
        cases.push((0.0, -(((r.below(200) + 1) * 2) as f64)));
    }
    check_rows("E10 ERANGE pole +0 / -even int", Expect::Range, &cases);
}

// ===========================================================================
// E11 — ERANGE: pole, +0.0 base with negative NON-integral exponent
// ===========================================================================
#[test]
fn e11_erange_pole_plus_zero_neg_nonint() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (0.0, -0.5),
        (0.0, -1.5),
        (0.0, -2.7),
        (0.0, -1e-300),
        (0.0, -f64::MIN_POSITIVE),
        (0.0, -f64::MAX),
    ];
    for _ in 0..1000 {
        let mut e = -r.range(0.0, 1000.0);
        if e.fract() == 0.0 {
            e -= 0.5;
        }
        cases.push((0.0, e));
    }
    check_rows("E11 ERANGE pole +0 / -non-int", Expect::Range, &cases);
}

// ===========================================================================
// E12 — ERANGE: pole, -0.0 base with negative ODD integral exponent
// ===========================================================================
#[test]
fn e12_erange_pole_minus_zero_neg_odd_int() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![(-0.0, -1.0), (-0.0, -3.0), (-0.0, -5.0), (-0.0, -7.0)];
    for _ in 0..500 {
        cases.push((-0.0, -((r.below(200) * 2 + 1) as f64)));
    }
    check_rows("E12 ERANGE pole -0 / -odd int", Expect::Range, &cases);
}

// ===========================================================================
// E13 — ERANGE: pole, -0.0 base with negative EVEN integral exponent
// ===========================================================================
#[test]
fn e13_erange_pole_minus_zero_neg_even_int() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![(-0.0, -2.0), (-0.0, -4.0), (-0.0, -6.0)];
    for _ in 0..500 {
        cases.push((-0.0, -(((r.below(200) + 1) * 2) as f64)));
    }
    check_rows("E13 ERANGE pole -0 / -even int", Expect::Range, &cases);
}

// ===========================================================================
// E14 — ERANGE: pole, -0.0 base with negative NON-integral exponent
// ===========================================================================
#[test]
fn e14_erange_pole_minus_zero_neg_nonint() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (-0.0, -0.5),
        (-0.0, -1.5),
        (-0.0, -2.7),
        (-0.0, -1e-300),
        (-0.0, -f64::MAX),
    ];
    for _ in 0..1000 {
        let mut e = -r.range(0.0, 1000.0);
        if e.fract() == 0.0 {
            e -= 0.5;
        }
        cases.push((-0.0, e));
    }
    check_rows("E14 ERANGE pole -0 / -non-int", Expect::Range, &cases);
}

// ===========================================================================
// N1 — negative base with INTEGRAL exponent: must NOT error
// ===========================================================================
#[test]
fn n01_neg_base_integral_exp_no_error() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![(-2.0, 3.0), (-2.0, 4.0), (-3.0, 5.0), (-1.5, 2.0)];
    for _ in 0..2000 {
        let base = -r.range(0.5, 2.0);
        let e = (r.below(41) as i64 - 20) as f64;
        cases.push((base, e));
    }
    check_rows("N1 neg base int exp (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N2 — NaN operands: must NOT error
// ===========================================================================
#[test]
fn n02_nan_operands_no_error() {
    let cases: Vec<(f64, f64)> = vec![
        (f64::NAN, 2.0),
        (2.0, f64::NAN),
        (f64::NAN, f64::NAN),
        (-2.0, f64::NAN),
        (f64::NAN, -2.0),
        (f64::NAN, 0.5),
        (f64::from_bits(0x7ff8_dead_beef_cafe), 3.0),
        (3.0, f64::from_bits(0xfff8_dead_beef_cafe)),
        (f64::from_bits(0x7ff0_0000_0000_0001), 3.0), // signalling NaN
        (3.0, f64::from_bits(0x7ff0_0000_0000_0001)),
        (f64::NAN, f64::INFINITY),
        (f64::INFINITY, f64::NAN),
        (f64::NAN, f64::NEG_INFINITY),
        (f64::NEG_INFINITY, f64::NAN),
        (0.0, f64::NAN),
        (-0.0, f64::NAN),
    ];
    check_rows("N2 NaN operands (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N3 — pow(1.0, NaN) == 1.0 and pow(NaN, 0.0) == 1.0: must NOT error
// ===========================================================================
#[test]
fn n03_one_pow_nan_and_nan_pow_zero_no_error() {
    let cases: Vec<(f64, f64)> = vec![
        (1.0, f64::NAN),
        (1.0, -f64::NAN),
        (1.0, f64::from_bits(0x7ff0_0000_0000_0001)),
        (f64::NAN, 0.0),
        (f64::NAN, -0.0),
        (-f64::NAN, 0.0),
        (f64::from_bits(0x7ff8_dead_beef_cafe), 0.0),
        (1.0, f64::INFINITY),
        (1.0, f64::NEG_INFINITY),
        (1.0, f64::MAX),
        (1.0, -f64::MAX),
    ];
    check_rows("N3 1^NaN / NaN^0 (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N4 — infinite operands: must NOT error (incl. pow(-0.0, -inf) -> +inf)
// ===========================================================================
#[test]
fn n04_infinite_operands_no_error() {
    let cases: Vec<(f64, f64)> = vec![
        (f64::INFINITY, 0.0),
        (f64::INFINITY, -0.0),
        (f64::INFINITY, 2.0),
        (f64::INFINITY, -2.0),
        (f64::NEG_INFINITY, 3.0),
        (f64::NEG_INFINITY, -3.0),
        (f64::NEG_INFINITY, 4.0),
        (f64::NEG_INFINITY, -4.0),
        (f64::NEG_INFINITY, 2.5),
        (f64::NEG_INFINITY, -2.5),
        (f64::NEG_INFINITY, 0.0),
        (f64::NEG_INFINITY, -0.0),
        (0.5, f64::INFINITY),
        (-0.5, f64::INFINITY),
        (2.0, f64::NEG_INFINITY),
        (-2.0, f64::NEG_INFINITY),
        (2.0, f64::INFINITY),
        (-2.0, f64::INFINITY),
        (0.5, f64::NEG_INFINITY),
        (-0.5, f64::NEG_INFINITY),
        (f64::INFINITY, f64::INFINITY),
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::NEG_INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
        (0.0, f64::INFINITY),
        (0.0, f64::NEG_INFINITY),
        (-0.0, f64::INFINITY),
        (-0.0, f64::NEG_INFINITY), // -> +inf WITHOUT ERANGE (contrast with E12)
        (1.0, f64::INFINITY),
        (-1.0, f64::INFINITY),
        (-1.0, f64::NEG_INFINITY),
    ];
    check_rows("N4 infinite operands (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N5 — +0.0 base with POSITIVE exponent: must NOT error
// ===========================================================================
#[test]
fn n05_plus_zero_positive_exp_no_error() {
    let mut r = Rng::new();
    let mut cases: Vec<(f64, f64)> = vec![
        (0.0, 2.0),
        (0.0, 1.0),
        (0.0, 0.5),
        (0.0, 1e-300),
        (0.0, f64::MAX),
        (-0.0, 2.0),
        (-0.0, 3.0),
        (-0.0, 0.5),
        (-0.0, f64::MAX),
    ];
    for _ in 0..1000 {
        let e = r.range(1e-8, 1000.0);
        cases.push((0.0, e));
        cases.push((-0.0, e));
    }
    check_rows("N5 ±0 base, +exp (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N6 — result is exactly the smallest normal: must NOT error
// ===========================================================================
#[test]
fn n06_smallest_normal_result_no_error() {
    let cases: Vec<(f64, f64)> = vec![
        (2.0, -1022.0),
        (2.0, -1021.0),
        (2.0, -1000.0),
        (0.5, 1022.0),
        (-2.0, -1022.0),
    ];
    check_rows("N6 smallest normal (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// N7 — result is subnormal but nonzero: glibc does NOT set ERANGE
// ===========================================================================
#[test]
fn n07_subnormal_nonzero_result_no_error() {
    let mut cases: Vec<(f64, f64)> = vec![(2.0, -1040.0), (-2.0, -1040.0)];
    // Every subnormal power of two: 2^-1023 .. 2^-1074
    for k in 1023..=1074 {
        cases.push((2.0, -(k as f64)));
        cases.push((0.5, k as f64));
    }
    check_rows(
        "N7 subnormal nonzero result (no error)",
        Expect::NoError,
        &cases,
    );
}

// ===========================================================================
// N8 — genuine -1.0 result via the happy path: must NOT print
// ===========================================================================
#[test]
fn n08_genuine_minus_one_no_error() {
    let cases: Vec<(f64, f64)> = vec![
        (-1.0, 1.0),
        (-1.0, 3.0),
        (-1.0, 5.0),
        (-1.0, -1.0),
        (-1.0, -3.0),
        (-1.0, 9007199254740993.0),
        (-1.0, 1e17 + 1.0),
    ];
    check_rows("N8 genuine -1.0 (no error)", Expect::NoError, &cases);
}

// ===========================================================================
// Generic C-API boundary sweeps
// ===========================================================================

/// Every out-of-band `errno` value a caller could leave behind, including
/// values that are not valid `errno` codes at all (the FFI analogue of an
/// out-of-range enum: C accepts any `int` here). The C zeroes `errno` first, so
/// none of these may leak into the branch decision.
#[test]
fn boundary_out_of_range_preset_errno_values() {
    let presets: Vec<i32> = vec![
        0,
        1,
        22,
        EDOM,
        ERANGE,
        EDOM - 1,
        EDOM + 1,
        ERANGE - 1,
        ERANGE + 1,
        -1,
        i32::MIN,
        i32::MAX,
        4095,
        0x7fff_fffe,
        133,  // EDOM + 100
        134,  // ERANGE + 100
    ];
    let inputs: Vec<(f64, f64)> = vec![
        (2.0, 10.0),   // valid
        (-2.0, 3.0),   // valid, negative base integral exp
        (-2.0, 0.5),   // EDOM
        (1e300, 3.0),  // ERANGE overflow
        (1e-300, 3.0), // ERANGE underflow
        (0.0, -3.0),   // ERANGE pole
        (-0.0, -3.0),  // ERANGE pole
        (1.0, f64::NAN),
        (f64::NAN, 0.0),
        (f64::NAN, f64::NAN),
        (2.0, -1040.0), // subnormal result, no ERANGE
        (-1.0, 3.0),    // genuine -1.0
        (f64::INFINITY, f64::NEG_INFINITY),
        (-0.0, f64::NEG_INFINITY),
    ];
    let mut cases: Vec<(f64, f64, i32)> = Vec::new();
    for &pe in &presets {
        for &(b, e) in &inputs {
            cases.push((b, e, pe));
        }
    }
    assert_all_match_with_errno("boundary: out-of-range preset errno", cases);
}

/// Exhaustive sweep of the *exponent* bit-field boundaries of both operands:
/// every canonical exponent field with a fixed mantissa, for both signs. This
/// walks the entire representable magnitude range, one step past each class
/// boundary (zero / subnormal / normal / infinity / NaN).
#[test]
fn boundary_full_exponent_field_sweep() {
    let mut cases: Vec<(f64, f64)> = Vec::new();
    let mut reps: Vec<f64> = Vec::new();
    for expfield in 0u64..=0x7ff {
        for &mant in &[0u64, 1, 0x0008_0000_0000_0000, 0x000f_ffff_ffff_ffff] {
            for &sign in &[0u64, 0x8000_0000_0000_0000] {
                reps.push(f64::from_bits(sign | (expfield << 52) | mant));
            }
        }
    }
    // Pair each representative with a small fixed set of exponents, and with a
    // deterministic partner from the same list.
    let fixed_exps = [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        3.0,
        -3.0,
        0.5,
        -0.5,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for (i, &b) in reps.iter().enumerate() {
        for &e in &fixed_exps {
            cases.push((b, e));
        }
        cases.push((b, reps[(i * 7 + 3) % reps.len()]));
        cases.push((reps[(i * 13 + 5) % reps.len()], b));
    }
    assert_all_match("boundary: full exponent-field sweep", cases);
}

/// The two documented `errno` constants and their immediate neighbours, driven
/// from the *result* side: inputs chosen so the C observes each distinct
/// `errno` value glibc `pow` can produce (0, EDOM, ERANGE) and nothing else.
#[test]
fn boundary_observed_errno_is_only_0_edom_erange() {
    let p = Pair::load();
    let mut cap = StderrCapture::new();
    let mut r = Rng::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut diffs = Vec::new();
    for _ in 0..50_000 {
        let (b, e) = (r.any_f64(), r.any_f64());
        if let Some(d) = diff_once(&mut cap, &p, b, e, 0) {
            if diffs.len() < 10 {
                diffs.push(d);
            }
        } else {
            let o = observe(&mut cap, p.c, b, e, 0);
            seen.insert(o.errno);
        }
    }
    drop(cap);
    if !diffs.is_empty() {
        let mut msg = String::from("[boundary errno domain] divergences:\n");
        for d in &diffs {
            msg.push_str(&format!("  - {}\n", d));
        }
        panic!("{}", msg);
    }
    println!("[boundary errno domain] observed errno values: {:?}", seen);
    // Both error branches AND the no-error path must actually have been reached,
    // otherwise this sweep proved nothing.
    assert!(seen.contains(&0), "no-error path never exercised");
    assert!(seen.contains(&EDOM), "EDOM branch never exercised: {:?}", seen);
    assert!(
        seen.contains(&ERANGE),
        "ERANGE branch never exercised: {:?}",
        seen
    );
    for v in &seen {
        assert!(
            *v == 0 || *v == EDOM || *v == ERANGE,
            "unexpected errno {} observed from C",
            v
        );
    }
}

// ===========================================================================
// Harness self-check — proves the differential comparison is NOT vacuous
// ===========================================================================

/// A deliberately wrong "implementation": same signature, silently different
/// behaviour. `diff_once` must flag it. Without this, a broken capture or a
/// broken `Obs` comparison would let every other test in this file pass
/// regardless of what the Rust library does.
extern "C" fn mutant_wrong_value(_b: f64, _e: f64) -> f64 {
    0.0
}

extern "C" fn mutant_uses_rust_powf(b: f64, e: f64) -> f64 {
    // Mirrors the shape of the C, but with Rust's `powf`, which never sets
    // `errno` — so the error branches are silently skipped.
    b.powf(e)
}

#[test]
fn harness_self_check_detects_divergence() {
    let p = Pair::load();
    let mut cap = StderrCapture::new();

    let mutant_a: MyPowFn = mutant_wrong_value;
    let mutant_b: MyPowFn = mutant_uses_rust_powf;

    // Cases spanning: valid, EDOM, ERANGE.
    let cases: Vec<(f64, f64)> = vec![
        (2.0, 10.0),
        (-2.0, 0.5),
        (1e300, 3.0),
        (1e-300, 3.0),
        (0.0, -3.0),
        (-0.0, -3.0),
    ];

    let mut caught_a = 0usize;
    let mut caught_b = 0usize;
    let mut real_diffs = 0usize;
    let mut stderr_seen = 0usize;
    let mut errno_nonzero_seen = 0usize;

    for &(b, e) in &cases {
        // The real pair must agree.
        if diff_once(&mut cap, &p, b, e, 0).is_some() {
            real_diffs += 1;
        }
        let c_obs = observe(&mut cap, p.c, b, e, 0);
        if !c_obs.stderr.is_empty() {
            stderr_seen += 1;
        }
        if c_obs.errno != 0 {
            errno_nonzero_seen += 1;
        }
        // The mutants must be caught.
        let a_obs = observe(&mut cap, mutant_a, b, e, 0);
        if a_obs != c_obs {
            caught_a += 1;
        }
        let b_obs = observe(&mut cap, mutant_b, b, e, 0);
        if b_obs != c_obs {
            caught_b += 1;
        }
    }

    drop(cap);

    assert_eq!(real_diffs, 0, "real C/Rust pair diverged");
    assert_eq!(
        caught_a,
        cases.len(),
        "harness failed to detect an always-0.0 implementation ({}/{} caught)",
        caught_a,
        cases.len()
    );
    assert!(
        caught_b >= 4,
        "harness failed to detect a `f64::powf` implementation that never sets errno \
         ({}/{} caught)",
        caught_b,
        cases.len()
    );
    assert!(
        stderr_seen >= 4,
        "stderr capture returned nothing for the error cases ({} of {})",
        stderr_seen,
        cases.len()
    );
    assert!(
        errno_nonzero_seen >= 4,
        "errno observation never saw a nonzero value ({} of {})",
        errno_nonzero_seen,
        cases.len()
    );
    println!(
        "[harness self-check] mutant A caught {}/{}, mutant B caught {}/{}, \
         stderr captured {}, nonzero errno {}",
        caught_a,
        cases.len(),
        caught_b,
        cases.len(),
        stderr_seen,
        errno_nonzero_seen
    );
}
