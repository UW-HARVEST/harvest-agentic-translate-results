//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact triggering
//! condition, calls BOTH `.so`s, and asserts the same rejection: the same
//! sentinel return value, the same `errno`, and the same `stderr` message
//! bytes — not merely "both failed somehow".

mod common;

use common::{Rng, assert_same, assert_same_one, assert_stderr_eq, capture_stderr, pair, run, set_errno};

const EDOM: i32 = 33;
const ERANGE: i32 = 34;

/// Asserts the C/Rust agreement plus the concrete C-side contract from
/// `ERRORS.md`: sentinel `-1.0`, the given `errno`, and the exact message.
fn assert_rejects(label: &str, base: f64, exponent: f64, expect_errno: i32, expect_msg: &str) {
    let one = assert_same_one(label, base, exponent);
    assert_eq!(
        one.value.to_bits(),
        (-1.0f64).to_bits(),
        "[{label}] my_pow({base:?}, {exponent:?}) should return the -1.0 sentinel, got {:?}",
        one.value
    );
    assert_eq!(
        one.errno, expect_errno,
        "[{label}] expected errno {expect_errno}, got {}",
        one.errno
    );
    assert_eq!(
        String::from_utf8_lossy(&one.stderr),
        expect_msg,
        "[{label}] stderr message mismatch"
    );
}

/// Asserts the input is NOT rejected: C/Rust agree, nothing printed, and the
/// value is whatever the C produced (never a spurious sentinel).
fn assert_accepts(label: &str, base: f64, exponent: f64) -> f64 {
    let one = assert_same_one(label, base, exponent);
    assert!(
        one.stderr.is_empty(),
        "[{label}] my_pow({base:?}, {exponent:?}) unexpectedly printed {:?}",
        String::from_utf8_lossy(&one.stderr)
    );
    one.value
}

// --- row 1 -----------------------------------------------------------------
#[test]
fn err_01_domain_negative_base_fractional_exponent() {
    assert_rejects(
        "err_01",
        -2.0,
        0.5,
        EDOM,
        "Domain error: pow(-2.00, 0.50) is undefined in the real number domain.\n",
    );
    // Same branch, several magnitudes.
    for &(b, e) in &[(-3.0, 1.5), (-1e10, 0.25), (-7.0, -0.5), (-1.5, 2.5)] {
        let one = assert_same_one("err_01_more", b, e);
        assert_eq!(one.errno, EDOM);
        assert_eq!(one.value.to_bits(), (-1.0f64).to_bits());
        assert!(String::from_utf8_lossy(&one.stderr).starts_with("Domain error: pow("));
    }
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn err_02_domain_negative_fraction_base() {
    assert_rejects(
        "err_02",
        -0.5,
        0.5,
        EDOM,
        "Domain error: pow(-0.50, 0.50) is undefined in the real number domain.\n",
    );
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn err_03_pole_pos_zero_negative_odd_int() {
    assert_rejects(
        "err_03",
        0.0,
        -1.0,
        ERANGE,
        "Range error: pow(0.00, -1.00) caused overflow or underflow.\n",
    );
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn err_04_pole_neg_zero_negative_odd_int() {
    assert_rejects(
        "err_04",
        -0.0,
        -3.0,
        ERANGE,
        "Range error: pow(-0.00, -3.00) caused overflow or underflow.\n",
    );
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn err_05_pole_zero_negative_even_int() {
    assert_rejects(
        "err_05",
        -0.0,
        -2.0,
        ERANGE,
        "Range error: pow(-0.00, -2.00) caused overflow or underflow.\n",
    );
    assert_rejects(
        "err_05b",
        0.0,
        -2.0,
        ERANGE,
        "Range error: pow(0.00, -2.00) caused overflow or underflow.\n",
    );
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn err_06_pole_zero_negative_fractional() {
    assert_rejects(
        "err_06",
        0.0,
        -0.5,
        ERANGE,
        "Range error: pow(0.00, -0.50) caused overflow or underflow.\n",
    );
    assert_rejects(
        "err_06b",
        -0.0,
        -0.5,
        ERANGE,
        "Range error: pow(-0.00, -0.50) caused overflow or underflow.\n",
    );
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn err_07_overflow() {
    assert_rejects(
        "err_07",
        10.0,
        400.0,
        ERANGE,
        "Range error: pow(10.00, 400.00) caused overflow or underflow.\n",
    );
    // Negative base with an (even) integral exponent that overflows.
    let one = assert_same_one("err_07b", -2.0, 1e300);
    assert_eq!(one.errno, ERANGE);
    assert_eq!(one.value.to_bits(), (-1.0f64).to_bits());
    assert!(String::from_utf8_lossy(&one.stderr).starts_with("Range error: pow(-2.00, 1000"));
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn err_08_underflow() {
    assert_rejects(
        "err_08",
        10.0,
        -400.0,
        ERANGE,
        "Range error: pow(10.00, -400.00) caused overflow or underflow.\n",
    );
    assert_rejects(
        "err_08b",
        1e-300,
        10.0,
        ERANGE,
        "Range error: pow(0.00, 10.00) caused overflow or underflow.\n",
    );
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn err_09_edom_checked_before_erange() {
    // Negative base, non-integral exponent, magnitude far outside the range:
    // glibc reports EDOM, and the C tests EDOM first, so the *domain* message
    // must win.
    for &(b, e) in &[(-1e300, 1.5), (-1e-300, -1.5), (-2.0, 1e5 + 0.5)] {
        let one = assert_same_one("err_09", b, e);
        assert_eq!(
            one.errno, EDOM,
            "glibc should report EDOM for pow({b:?}, {e:?})"
        );
        assert!(
            String::from_utf8_lossy(&one.stderr).starts_with("Domain error: "),
            "EDOM branch must be taken before ERANGE; got {:?}",
            String::from_utf8_lossy(&one.stderr)
        );
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn err_10_preexisting_errno_cleared() {
    let p = pair();
    for preset in [EDOM, ERANGE, 1, 22, 9999, -1] {
        for &(b, e) in &[(2.0, 10.0), (-2.0, 3.0), (1.0, 1.0), (0.0, 1.0)] {
            let (cv, cerr) = capture_stderr(|| {
                set_errno(preset);
                p.c.call(b, e)
            });
            let (rv, rerr) = capture_stderr(|| {
                set_errno(preset);
                p.rust.call(b, e)
            });
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "err_10: preset errno {preset}, my_pow({b:?}, {e:?}): C {cv:?} vs Rust {rv:?}"
            );
            assert_stderr_eq("err_10", &cerr, &rerr);
            assert!(
                cerr.is_empty(),
                "err_10: a stale errno={preset} must be cleared by `errno = 0`, but C printed {:?}",
                String::from_utf8_lossy(&cerr)
            );
            assert_ne!(
                cv.to_bits(),
                (-1.0f64).to_bits(),
                "err_10: spurious sentinel for my_pow({b:?}, {e:?}) with preset errno {preset}"
            );
        }
        set_errno(0);
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn err_11_errno_not_leaked_across_calls() {
    // An erroring call leaves errno set; the next valid call must clear it.
    let inputs = vec![
        (-2.0, 0.5),
        (2.0, 10.0),
        (0.0, -1.0),
        (3.0, 3.0),
        (10.0, 400.0),
        (2.0, 0.5),
        (10.0, -400.0),
        (7.0, 2.0),
    ];
    assert_same("err_11", &inputs);

    let p = pair();
    let c = run(&p.c, &inputs);
    for i in [1usize, 3, 5, 7] {
        assert_eq!(
            c.errnos[i], 0,
            "err_11: valid call #{i} should leave errno == 0, got {}",
            c.errnos[i]
        );
        assert_ne!(
            c.bits[i],
            (-1.0f64).to_bits(),
            "err_11: valid call #{i} returned the error sentinel"
        );
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn err_12_nan_inputs_are_not_errors() {
    let v = assert_accepts("err_12_nan_base", f64::NAN, 2.0);
    assert!(v.is_nan(), "expected NaN, got {v:?}");

    let v = assert_accepts("err_12_nan_base_zero_exp", f64::NAN, 0.0);
    assert_eq!(v, 1.0);

    let v = assert_accepts("err_12_nan_exp", 2.0, f64::NAN);
    assert!(v.is_nan(), "expected NaN, got {v:?}");

    let v = assert_accepts("err_12_one_nan_exp", 1.0, f64::NAN);
    assert_eq!(v, 1.0);

    let v = assert_accepts("err_12_both_nan", f64::NAN, f64::NAN);
    assert!(v.is_nan(), "expected NaN, got {v:?}");
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn err_13_nan_payloads() {
    let mut nans = vec![
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // all-payload-bits NaN
        f64::from_bits(0x7FF8_0000_DEAD_BEEF), // arbitrary payload
    ];
    let mut rng = Rng::new(0x13AD_5EED_0000_0013);
    for _ in 0..200 {
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        if payload == 0 {
            continue;
        }
        let sign = rng.next_u64() & 0x8000_0000_0000_0000;
        nans.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload));
    }

    let others = [2.0, -2.0, 0.5, 0.0, -0.0, 1.0, -1.0, f64::INFINITY, f64::NEG_INFINITY];
    let mut inputs = Vec::new();
    for &n in &nans {
        for &o in &others {
            inputs.push((n, o));
            inputs.push((o, n));
        }
        inputs.push((n, n));
    }
    assert_same("err_13", &inputs);
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn err_14_infinities() {
    let cases = [
        (f64::INFINITY, 2.0),
        (f64::NEG_INFINITY, 3.0),
        (f64::NEG_INFINITY, 2.0),
        (f64::NEG_INFINITY, 2.5),
        (f64::NEG_INFINITY, -3.0),
        (2.0, f64::INFINITY),
        (-1.0, f64::INFINITY),
        (-2.0, f64::INFINITY),
        (0.5, f64::INFINITY),
        (2.0, f64::NEG_INFINITY),
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
        (f64::INFINITY, 0.0),
        (f64::NEG_INFINITY, -0.0),
    ];
    for &(b, e) in &cases {
        assert_accepts("err_14", b, e);
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn err_15_successful_minus_one_is_not_an_error() {
    // pow(-1, 1) == -1.0 with errno == 0: a *successful* result numerically
    // identical to the error sentinel. The C returns it unchanged and prints
    // nothing; the Rust must too.
    for &(b, e) in &[(-1.0, 1.0), (-1.0, 3.0), (-1.0, 51.0), (-1.0, -1.0), (-1.0, -3.0)] {
        let one = assert_same_one("err_15", b, e);
        assert_eq!(one.value.to_bits(), (-1.0f64).to_bits());
        assert_eq!(one.errno, 0);
        assert!(
            one.stderr.is_empty(),
            "err_15: my_pow({b:?}, {e:?}) is a success, must print nothing"
        );
    }
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn err_16_subnormal_result_no_erange() {
    // glibc does not flag this one, so the C must return the subnormal.
    let v = assert_accepts("err_16", 2.0, -1070.0);
    assert!(
        v > 0.0 && v < f64::MIN_POSITIVE,
        "expected a positive subnormal, got {v:?}"
    );
    assert_accepts("err_16b", 2.0, -1060.0);
    assert_accepts("err_16c", 0.5, 1070.0);
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn err_17_message_formatting_pathological_values() {
    // Inputs chosen so the *arguments* printed with %.2f are pathological
    // while the call still takes an error branch.
    let inputs = vec![
        (-0.0, -1.0),           // "-0.00"
        (-0.0, -2.0),           // "-0.00"
        (0.0, -1.0),            // "0.00"
        (-1e300, 0.5),          // 301-digit integer part
        (-1e-300, 0.5),         // "-0.00"
        (f64::MIN, 0.5),        // -DBL_MAX, 309 digits
        (10.0, 1e300),          // huge exponent printed
        (10.0, -1e300),         // huge negative exponent printed
        (-0.005, 0.5),          // half-way rounding -> "-0.01"
        (-0.004999, 0.5),       // -> "-0.00"
        (-2.675, 0.5),          // classic binary-rounding case -> "-2.67"
        (-1.005, 0.5),
        (-1e15 + 0.5, 0.5),
        (-123456789.987654, 0.5),
        (0.0, -5e-324),         // subnormal exponent -> "-0.00"
        (-5e-324, 0.5),
        (2.0, 1e308),
        (-1.5, f64::MAX),
    ];
    assert_same("err_17", &inputs);

    // Spot-check the exact C bytes for the signed-zero and rounding cases.
    let one = assert_same_one("err_17_negzero", -0.0, -1.0);
    assert_eq!(
        String::from_utf8_lossy(&one.stderr),
        "Range error: pow(-0.00, -1.00) caused overflow or underflow.\n"
    );
    let one = assert_same_one("err_17_round", -2.675, 0.5);
    assert_eq!(
        String::from_utf8_lossy(&one.stderr),
        "Domain error: pow(-2.67, 0.50) is undefined in the real number domain.\n"
    );
}

// --- generic FFI boundary sweep --------------------------------------------
/// The API has no pointers, lengths or enums, so the analogue of "every
/// out-of-range value a C caller could pass" is the whole 64-bit space —
/// swept exhaustively over exponent-field boundaries here.
#[test]
fn err_18_exhaustive_exponent_field_boundaries() {
    let mut inputs = Vec::new();
    // One representative value per biased exponent field, both signs.
    for exp_field in 0u64..=2047 {
        for &sign in &[0u64, 1u64 << 63] {
            for &mant in &[0u64, 1, 0x000F_FFFF_FFFF_FFFF, 0x0008_0000_0000_0000] {
                inputs.push(f64::from_bits(sign | (exp_field << 52) | mant));
            }
        }
    }
    let partners = [2.0, -2.0, 0.5, -0.5, 3.0, -3.0, 0.0, -0.0, 1.0, -1.0];
    let mut pairs = Vec::with_capacity(inputs.len() * partners.len() * 2);
    for &v in &inputs {
        for &p in &partners {
            pairs.push((v, p));
            pairs.push((p, v));
        }
    }
    assert_same("err_18", &pairs);
}
