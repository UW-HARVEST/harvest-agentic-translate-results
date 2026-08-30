//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input or
//! degenerate condition, calls BOTH libraries, and asserts they reject/degrade
//! IDENTICALLY — comparing the precise sentinel bytes, not merely "both did
//! something".

mod common;

use common::*;
use std::ffi::c_char;

/// The x86-64 `cvttsd2si` "integer indefinite" result, which is what the C
/// produces for every out-of-range / NaN `double`->`int` conversion.
const INT_INDEFINITE: &[u8] = b"-2147483648\n";
const DBZ_MSG: &[u8] = b"This would result in a divide by zero\n";

// ---------------------------------------------------------------------------
// E1 — printLine(NULL): the only literal null check in the library (line 32)
// ---------------------------------------------------------------------------

#[test]
fn e1_printline_null_is_silent_noop() {
    let out = diff_one("E1/printLine(NULL)", |api| {
        (api.print_line)(std::ptr::null::<c_char>())
    });
    assert!(
        out.is_empty(),
        "printLine(NULL) must emit NOTHING, got {}",
        show(&out)
    );

    // Repeated NULL calls must stay silent and must not corrupt later output.
    let out = diff_one("E1/printLine(NULL) x100 then a real line", |api| {
        for _ in 0..100 {
            (api.print_line)(std::ptr::null::<c_char>());
        }
        (api.print_int_line)(7);
    });
    assert_eq!(out, b"7\n");
}

// ---------------------------------------------------------------------------
// E2 / E11 / E12 — goodB2G's `else` branch (line 66)
// ---------------------------------------------------------------------------

#[test]
fn e2_good_threshold_false_prints_message() {
    for &v in &[0.0f32, -0.0, 1e-9, -1e-9, 1e-30, 1e-45, -1e-45, 1e-7, -1e-7] {
        let out = diff_one(&format!("E2/good({v:e})"), |api| (api.good)(v));
        let mut want = b"50\n".to_vec();
        want.extend_from_slice(DBZ_MSG);
        assert_eq!(out, want, "good({v:e}) must take the rejection branch");
    }
}

#[test]
fn e11_good_nan_takes_rejection_branch() {
    // NaN compares false against everything, so `fabs(NaN) > 0.000001` is false.
    for &v in &[f32::NAN, -f32::NAN] {
        let out = diff_one("E11/good(NaN)", |api| (api.good)(v));
        let mut want = b"50\n".to_vec();
        want.extend_from_slice(DBZ_MSG);
        assert_eq!(out, want, "NaN must take the else branch, not divide");
    }
    // Also exercise assorted NaN payloads / both sign bits.
    let nans: Vec<f32> = vec![
        f32::from_bits(0x7fc0_0000),
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0xffff_ffff),
    ];
    diff_samples("E11/good NaN payloads", &nans, |api, v| (api.good)(v));
    diff_samples("E11/bad NaN payloads", &nans, |api, v| (api.bad)(v));
}

#[test]
fn e12_good_threshold_exact_float_epsilon() {
    // (double)1e-6f == 9.99999997475242708e-07 < 1e-06, so `>` is FALSE.
    let out = diff_one("E12/good(1e-6f)", |api| (api.good)(1e-6));
    let mut want = b"50\n".to_vec();
    want.extend_from_slice(DBZ_MSG);
    assert_eq!(
        out, want,
        "the float nearest 1e-6 is BELOW the double literal 0.000001"
    );

    // One ULP up is above the threshold and must divide instead.
    let up = f32::from_bits(1e-6f32.to_bits() + 1);
    let out = diff_one("E12/good(1e-6f + 1ulp)", |api| (api.good)(up));
    assert_ne!(out, want, "one ULP up must NOT take the rejection branch");
    // 100.0 / 1.00000001e-6 ~= 9.9999988e7, truncated to 99999988.
    assert_eq!(out, b"50\n99999988\n");

    // Same story for the negative side (fabs is applied first).
    let out = diff_one("E12/good(-1e-6f)", |api| (api.good)(-1e-6));
    assert_eq!(out, want);
    let out = diff_one("E12/good(-(1e-6f + 1ulp))", |api| (api.good)(-up));
    assert_eq!(out, b"50\n-99999988\n");
}

// ---------------------------------------------------------------------------
// E3..E9 — bad()'s unguarded division: out-of-range double->int conversion
// ---------------------------------------------------------------------------

#[test]
fn e3_bad_positive_zero_divide() {
    let out = diff_one("E3/bad(0.0)", |api| (api.bad)(0.0));
    assert_eq!(out, INT_INDEFINITE, "100.0/0.0 = +inf, (int)+inf = INT_MIN");
}

#[test]
fn e4_bad_negative_zero_divide() {
    let out = diff_one("E4/bad(-0.0)", |api| (api.bad)(-0.0));
    assert_eq!(out, INT_INDEFINITE, "100.0/-0.0 = -inf, (int)-inf = INT_MIN");
    // -0.0 must really be negative zero, not folded to +0.0.
    assert!((-0.0f32).is_sign_negative());
}

#[test]
fn e5_bad_nan() {
    let out = diff_one("E5/bad(NaN)", |api| (api.bad)(f32::NAN));
    assert_eq!(out, INT_INDEFINITE, "(int)NaN = INT_MIN");
}

#[test]
fn e6_bad_tiny_positive_overflows() {
    for &v in &[1e-8f32, 1e-12, 1e-20, 1e-30, 1e-38, 1e-45, f32::MIN_POSITIVE] {
        let out = diff_one(&format!("E6/bad({v:e})"), |api| (api.bad)(v));
        assert_eq!(out, INT_INDEFINITE, "100.0/{v:e} overflows int");
    }
}

#[test]
fn e7_bad_tiny_negative_overflows() {
    for &v in &[-1e-8f32, -1e-12, -1e-20, -1e-30, -1e-38, -1e-45, -f32::MIN_POSITIVE] {
        let out = diff_one(&format!("E7/bad({v:e})"), |api| (api.bad)(v));
        assert_eq!(out, INT_INDEFINITE, "100.0/{v:e} underflows int");
    }
}

#[test]
fn e8_e9_bad_int_conversion_edge() {
    // Walk the exact overflow boundary: the cast overflows iff the truncated
    // quotient is >= 2^31. Sweep floats around data == 100.0/2^31 by ULPs so we
    // straddle it from both sides, and confirm C and Rust agree on every step
    // AND that E9 (just inside) really is NOT the indefinite value.
    let edge = (100.0f64 / 2147483648.0f64) as f32;
    let mut values: Vec<f32> = Vec::new();
    for d in -40i32..=40 {
        let bits = (edge.to_bits() as i64 + d as i64) as u32;
        values.push(f32::from_bits(bits));
        values.push(-f32::from_bits(bits));
    }
    diff_samples("E8/E9 bad int-conversion edge", &values, |api, v| {
        (api.bad)(v)
    });

    // Explicitly prove both sides of the boundary exist (i.e. the Rust is not
    // blanket-saturating nor blanket-returning INT_MIN).
    let mut saw_indefinite = false;
    let mut saw_in_range = false;
    for &v in &values {
        if v <= 0.0 {
            continue;
        }
        let out = diff_one("E8/E9 edge single", |api| (api.bad)(v));
        if out == INT_INDEFINITE {
            saw_indefinite = true;
        } else {
            saw_in_range = true;
        }
    }
    assert!(saw_indefinite, "expected overflow on one side of the edge");
    assert!(saw_in_range, "expected an in-range result on the other side");

    // And the negative edge (truncated == -2^31 is REPRESENTABLE, one step
    // further is not).
    let neg_edge = (100.0f64 / -2147483648.0f64) as f32;
    let neg: Vec<f32> = (-40i32..=40)
        .map(|d| f32::from_bits((neg_edge.abs().to_bits() as i64 + d as i64) as u32))
        .map(|f| -f)
        .collect();
    diff_samples("E8/E9 bad negative edge", &neg, |api, v| (api.bad)(v));
}

// ---------------------------------------------------------------------------
// E10 — infinities are NOT an error path (100/inf == 0)
// ---------------------------------------------------------------------------

#[test]
fn e10_bad_infinities_yield_zero_not_error() {
    let out = diff_one("E10/bad(+inf)", |api| (api.bad)(f32::INFINITY));
    assert_eq!(out, b"0\n", "100.0/+inf = +0.0, (int)+0.0 = 0");
    let out = diff_one("E10/bad(-inf)", |api| (api.bad)(f32::NEG_INFINITY));
    assert_eq!(out, b"0\n", "100.0/-inf = -0.0, (int)-0.0 = 0");

    // good(+/-inf) takes the TRUE branch (fabs(inf) > 1e-6) and prints 0.
    let out = diff_one("E10/good(+inf)", |api| (api.good)(f32::INFINITY));
    assert_eq!(out, b"50\n0\n");
    let out = diff_one("E10/good(-inf)", |api| (api.good)(f32::NEG_INFINITY));
    assert_eq!(out, b"50\n0\n");
}

// ---------------------------------------------------------------------------
// E13 — printLine must not interpret its argument as a format string
// ---------------------------------------------------------------------------

#[test]
fn e13_printline_no_format_string_interpretation() {
    // %n is the dangerous one: if `line` were used as the FORMAT, printf would
    // try to write through a pointer argument. Both must print it literally.
    let cases: &[&[u8]] = &[
        b"%n",
        b"%n%n%n%n",
        b"%s%s%s%s%s%s%s%s%s%s",
        b"%99999999d",
        b"%p",
        b"AAAA%08x.%08x.%08x.%08x",
        b"%.2000f",
    ];
    for case in cases {
        let s = cstring(case);
        let out = diff_one("E13/printLine hostile format", |api| {
            (api.print_line)(s.as_ptr())
        });
        let mut want = case.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "must be printed verbatim as the %s argument");
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary conditions (see the second table in ERRORS.md)
// ---------------------------------------------------------------------------

#[test]
fn generic_printline_oversized() {
    for &len in &[8192usize, 65536, 262144] {
        let bytes: Vec<u8> = std::iter::repeat(b'Z').take(len).collect();
        let s = cstring(&bytes);
        let out = diff_one("generic/printLine oversized", |api| {
            (api.print_line)(s.as_ptr())
        });
        assert_eq!(out.len(), len + 1);
    }
}

#[test]
fn generic_printline_non_utf8() {
    // Byte sequences that are NOT valid UTF-8. The C `printf("%s")` copies raw
    // bytes; a Rust translation that routed through `str` would corrupt these.
    let cases: &[&[u8]] = &[
        &[0xff],
        &[0xfe, 0xff],
        &[0x80, 0x81, 0x82],
        &[0xc3],             // truncated 2-byte sequence
        &[0xe2, 0x82],       // truncated 3-byte sequence
        &[0xf0, 0x9f, 0x98], // truncated 4-byte sequence
        &[0xed, 0xa0, 0x80], // UTF-16 surrogate encoded as UTF-8
        &[0xff, 0xfe, 0xfd, 0xfc, 0xfb],
        &[0x41, 0xff, 0x42, 0x80, 0x43],
    ];
    for case in cases {
        let s = cstring(case);
        let out = diff_one("generic/printLine non-utf8", |api| {
            (api.print_line)(s.as_ptr())
        });
        let mut want = case.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "raw bytes must survive unchanged");
    }
}

#[test]
fn generic_printintline_full_int_domain() {
    // printIntLine's whole 32-bit domain is valid; there is no enum or mode
    // parameter anywhere in this library, so "out-of-range enum" reduces to
    // "any int", which we sweep at the extremes plus a dense random sample.
    let mut values: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new();
    values.extend((0..1024).map(|_| rng.next_i32()));
    // Powers of two and their neighbours (digit-count boundaries in %d).
    for k in 0..31 {
        let p = 1i32 << k;
        values.extend_from_slice(&[p - 1, p, p + 1, -p, -p + 1]);
    }
    diff_samples("generic/printIntLine full domain", &values, |api, v| {
        (api.print_int_line)(v)
    });
}

#[test]
fn generic_all_entry_points_exhaustive_float_sweep() {
    // Every exported float-taking entry point, over the full interesting set,
    // one step past each documented range boundary included.
    diff_samples("generic/bad sweep", INTERESTING_FLOATS, |api, v| (api.bad)(v));
    diff_samples("generic/good sweep", INTERESTING_FLOATS, |api, v| {
        (api.good)(v)
    });
    diff_samples("generic/driver sweep g", INTERESTING_FLOATS, |api, v| {
        (api.driver)(v, 3.0)
    });
    diff_samples("generic/driver sweep b", INTERESTING_FLOATS, |api, v| {
        (api.driver)(3.0, v)
    });
}
