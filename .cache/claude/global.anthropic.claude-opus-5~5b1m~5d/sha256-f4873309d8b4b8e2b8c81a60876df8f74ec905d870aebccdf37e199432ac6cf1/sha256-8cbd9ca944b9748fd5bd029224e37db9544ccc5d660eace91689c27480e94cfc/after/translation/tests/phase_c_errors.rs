//! Phase C — error / rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! Every function in this library returns `void`, so the "error code" observable
//! across the FFI boundary is the exact byte sequence written to stdout. Each
//! test therefore asserts THREE things:
//!   1. the C `.so` produces the exact bytes recorded in `ERRORS.md`,
//!   2. the Rust `.so` produces the exact same bytes,
//!   3. (via `diff`) the two captures are byte-identical.
//! Pinning the literal bytes is what makes this stronger than "both rejected
//! somehow".

mod common;

use common::{api, capture, cstr, diff, render, Impl, Rng, BOTH};

/// `(int)` indefinite value, as rendered by `printf("%d\n", ...)`.
const INT_MIN_LINE: &[u8] = b"-2147483648\n";
const DIV_ZERO_MSG: &[u8] = b"This would result in a divide by zero\n";
/// `good()` always prints the constant `goodG2B()` result first.
const G2B_LINE: &[u8] = b"50\n";

/// Asserts both implementations produce exactly `expected`.
fn expect<F: Fn(&common::Api)>(what: &str, expected: &[u8], body: F) {
    for imp in BOTH {
        let a = api(imp);
        let out = capture(|| body(&a));
        assert_eq!(
            out,
            expected,
            "\n{what}\n  {} .so produced {}\n  expected           {}",
            imp.name(),
            render(&out),
            render(expected)
        );
    }
    // Redundant but keeps the differential assertion explicit for every row.
    diff(what, body);
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

// ===========================================================================
// Explicit guards
// ===========================================================================

/// Row 1: `printLine(NULL)` — the `if(line != NULL)` guard rejects, no output.
#[test]
fn err01_print_line_null_writes_nothing() {
    expect("ERRORS row 1: printLine(NULL)", b"", |a| unsafe {
        (a.print_line)(std::ptr::null())
    });
    // Also confirm rejection is silent even when repeated / interleaved.
    expect(
        "ERRORS row 1: printLine(NULL) x3 interleaved",
        b"",
        |a| unsafe {
            (a.print_line)(std::ptr::null());
            (a.print_line)(std::ptr::null());
            (a.print_line)(std::ptr::null());
        },
    );
}

/// Row 2: `goodB2G` rejects `data == 0.0f`.
#[test]
fn err02_good_rejects_positive_zero() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    expect("ERRORS row 2: good(+0.0f)", &expected, |a| unsafe {
        (a.good)(f(0x0000_0000))
    });
}

/// Row 3: `goodB2G` rejects `data == -0.0f` (`fabs` yields `+0.0`).
#[test]
fn err03_good_rejects_negative_zero() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    expect("ERRORS row 3: good(-0.0f)", &expected, |a| unsafe {
        (a.good)(f(0x8000_0000))
    });
}

/// Row 4: `goodB2G` rejects NaN — every `>` comparison against NaN is false, so
/// the guard FAILS and the else branch runs. (Classic blind spot.)
#[test]
fn err04_good_rejects_nan_via_failed_comparison() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    for (bits, name) in [
        (0x7fc0_0000u32, "qNaN"),
        (0xffc0_0000, "-qNaN"),
        (0x7fa0_0000, "sNaN"),
        (0xffa0_0000, "-sNaN"),
        (0x7f80_0001, "sNaN(min payload)"),
        (0x7fff_ffff, "qNaN(max payload)"),
        (0xffff_ffff, "-qNaN(max payload)"),
    ] {
        expect(
            &format!("ERRORS row 4: good({name} 0x{bits:08x})"),
            &expected,
            |a| unsafe { (a.good)(f(bits)) },
        );
    }
}

/// Row 5: `1e-6f` widens to `9.99999997475e-7` < `0.000001`, so the guard
/// fails — one ULP BELOW the threshold.
#[test]
fn err05_good_rejects_exactly_1e6_float_literal() {
    assert!(
        (1.0e-6f32 as f64) <= 0.000001,
        "precondition: 1e-6f must widen to <= 1e-6"
    );
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    for (bits, name) in [(0x3586_37bd_u32, "1e-6f"), (0xb586_37bd, "-1e-6f")] {
        expect(
            &format!("ERRORS row 5: good({name} 0x{bits:08x})"),
            &expected,
            |a| unsafe { (a.good)(f(bits)) },
        );
    }
}

/// Row 6: one ULP PAST the threshold — the guard passes and the division
/// branch runs, printing `99999988`.
#[test]
fn err06_good_accepts_one_ulp_past_threshold() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(b"99999988\n");
    expect(
        "ERRORS row 6: good(nextafter(1e-6f,+inf) 0x358637be)",
        &expected,
        |a| unsafe { (a.good)(f(0x3586_37be)) },
    );
    let mut expected_neg = G2B_LINE.to_vec();
    expected_neg.extend_from_slice(b"-99999988\n");
    expect(
        "ERRORS row 6: good(-nextafter(1e-6f,+inf) 0xb58637be)",
        &expected_neg,
        |a| unsafe { (a.good)(f(0xb586_37be)) },
    );
}

/// Row 7: smallest positive subnormal is under the threshold -> rejected.
#[test]
fn err07_good_rejects_smallest_subnormal() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    for (bits, name) in [
        (0x0000_0001u32, "1e-45f (min subnormal)"),
        (0x8000_0001, "-1e-45f"),
        (0x007f_ffff, "max subnormal"),
        (0x0080_0000, "FLT_MIN (min normal)"),
        (0x8080_0000, "-FLT_MIN"),
    ] {
        expect(
            &format!("ERRORS row 7: good({name} 0x{bits:08x})"),
            &expected,
            |a| unsafe { (a.good)(f(bits)) },
        );
    }
}

/// Row 8: negative value with magnitude under the threshold -> rejected.
#[test]
fn err08_good_rejects_small_negative() {
    let mut expected = G2B_LINE.to_vec();
    expected.extend_from_slice(DIV_ZERO_MSG);
    for v in [-1.0e-7f32, -1.0e-8, -1.0e-20, -5.0e-7, -9.999e-7] {
        expect(
            &format!("ERRORS row 8: good({v:e})"),
            &expected,
            |a| unsafe { (a.good)(v) },
        );
    }
}

// ===========================================================================
// Unguarded paths in `bad` (the injected CWE-369 defect)
// ===========================================================================

/// Row 9: `bad(+0.0f)` -> `100.0/0.0` = `+inf` -> `(int)` indefinite.
#[test]
fn err09_bad_positive_zero_divide() {
    expect("ERRORS row 9: bad(+0.0f)", INT_MIN_LINE, |a| unsafe {
        (a.bad)(f(0x0000_0000))
    });
}

/// Row 10: `bad(-0.0f)` -> `-inf` -> `(int)` indefinite.
#[test]
fn err10_bad_negative_zero_divide() {
    expect("ERRORS row 10: bad(-0.0f)", INT_MIN_LINE, |a| unsafe {
        (a.bad)(f(0x8000_0000))
    });
}

/// Row 11: `bad(NaN)` -> NaN quotient -> `(int)` indefinite.
#[test]
fn err11_bad_nan() {
    for (bits, name) in [
        (0x7fc0_0000u32, "qNaN"),
        (0xffc0_0000, "-qNaN"),
        (0x7fa0_0000, "sNaN"),
        (0xffa0_0000, "-sNaN"),
        (0x7f80_0001, "sNaN(min payload)"),
        (0x7fff_ffff, "qNaN(max payload)"),
        (0xffff_ffff, "-qNaN(max payload)"),
    ] {
        expect(
            &format!("ERRORS row 11: bad({name} 0x{bits:08x})"),
            INT_MIN_LINE,
            |a| unsafe { (a.bad)(f(bits)) },
        );
    }
}

/// Row 12: tiny subnormal -> quotient overflows `int`.
#[test]
fn err12_bad_subnormal_overflow() {
    for (bits, name) in [
        (0x0000_0001u32, "1e-45f"),
        (0x8000_0001, "-1e-45f"),
        (0x007f_ffff, "max subnormal"),
        (0x8000_00ff, "-subnormal"),
        (0x0080_0000, "FLT_MIN"),
        (0x8080_0000, "-FLT_MIN"),
    ] {
        expect(
            &format!("ERRORS row 12: bad({name} 0x{bits:08x})"),
            INT_MIN_LINE,
            |a| unsafe { (a.bad)(f(bits)) },
        );
    }
}

/// Row 13: quotient EXACTLY `INT_MAX+1` (`2147483648.0`) — one step past the
/// representable range -> indefinite. And one ULP further, where it becomes a
/// genuine in-range result. This pins the conversion boundary exactly.
#[test]
fn err13_bad_quotient_exactly_int_max_plus_one() {
    // 100.0f64 / 2^31, rounded to f32 == 0x33480000; 100/that == 2^31 exactly.
    let pivot_bits = 0x3348_0000u32;
    assert_eq!(
        (100.0f64 / f(pivot_bits) as f64),
        2147483648.0,
        "precondition: pivot must give exactly INT_MAX+1"
    );
    expect(
        "ERRORS row 13: bad(0x33480000) q == 2^31 -> indefinite",
        INT_MIN_LINE,
        |a| unsafe { (a.bad)(f(pivot_bits)) },
    );
    // One ULP up in `data` => smaller quotient => in range, real value.
    expect(
        "ERRORS row 13: bad(0x33480001) q == 2147483484.16 -> in range",
        b"2147483484\n",
        |a| unsafe { (a.bad)(f(pivot_bits + 1)) },
    );
    // One ULP down in `data` => larger quotient => still out of range.
    expect(
        "ERRORS row 13: bad(0x3347ffff) q == 2147483811.84 -> indefinite",
        INT_MIN_LINE,
        |a| unsafe { (a.bad)(f(pivot_bits - 1)) },
    );
}

/// Row 14: quotient exactly `-2147483648.0` — the lowest STILL-VALID value.
/// Must not be treated as overflow, even though it prints the same text.
#[test]
fn err14_bad_quotient_exactly_int_min_is_valid() {
    let neg_pivot = 0xb348_0000u32;
    assert_eq!(
        (100.0f64 / f(neg_pivot) as f64),
        -2147483648.0,
        "precondition: -pivot must give exactly INT_MIN"
    );
    expect(
        "ERRORS row 14: bad(0xb3480000) q == INT_MIN (in range)",
        INT_MIN_LINE,
        |a| unsafe { (a.bad)(f(neg_pivot)) },
    );
    expect(
        "ERRORS row 14: bad(0xb3480001) q == -2147483484.16 -> in range",
        b"-2147483484\n",
        |a| unsafe { (a.bad)(f(neg_pivot + 1)) },
    );
    expect(
        "ERRORS row 14: bad(0xb347ffff) q == -2147483811.84 -> indefinite",
        INT_MIN_LINE,
        |a| unsafe { (a.bad)(f(neg_pivot - 1)) },
    );
}

/// Row 15: `bad(+inf)` -> `100.0/inf` = `+0.0` -> prints `0`.
#[test]
fn err15_bad_positive_infinity() {
    expect("ERRORS row 15: bad(+inf)", b"0\n", |a| unsafe {
        (a.bad)(f32::INFINITY)
    });
}

/// Row 16: `bad(-inf)` -> `-0.0` -> `(int)-0.0` == `0`, printed as `0` not `-0`.
#[test]
fn err16_bad_negative_infinity_prints_unsigned_zero() {
    expect("ERRORS row 16: bad(-inf)", b"0\n", |a| unsafe {
        (a.bad)(f32::NEG_INFINITY)
    });
}

/// Row 17: exhaustive-ish sweep of NaN / signalling-NaN bit patterns.
#[test]
fn err17_bad_all_nan_bit_pattern_classes() {
    let mut rng = Rng::new(0x17_c0de_17c0);
    let mut bits_list: Vec<u32> = Vec::new();
    for sign in [0u32, 0x8000_0000] {
        for payload in [1u32, 2, 0x0000_00ff, 0x0040_0000, 0x0055_5555, 0x007f_ffff] {
            bits_list.push(sign | 0x7f80_0000 | payload);
        }
    }
    for _ in 0..200 {
        let payload = (rng.next_u32() & 0x007f_ffff) | 1; // nonzero => NaN
        let sign = (rng.next_u32() & 1) << 31;
        bits_list.push(sign | 0x7f80_0000 | payload);
    }
    for bits in bits_list {
        let v = f(bits);
        assert!(v.is_nan(), "0x{bits:08x} should be NaN");
        expect(
            &format!("ERRORS row 17: bad(NaN 0x{bits:08x})"),
            INT_MIN_LINE,
            |a| unsafe { (a.bad)(v) },
        );
    }
}

/// Row 18: negative quotient truncates TOWARD ZERO, not floor.
#[test]
fn err18_bad_negative_truncates_toward_zero() {
    for (v, want) in [
        (-3.0f32, "-33\n"),
        (3.0f32, "33\n"),
        (-7.0f32, "-14\n"),
        (7.0f32, "14\n"),
        (-6.0f32, "-16\n"),
        (-101.0f32, "0\n"),
        (101.0f32, "0\n"),
        (-1000.0f32, "0\n"),
    ] {
        expect(
            &format!("ERRORS row 18: bad({v}) truncation"),
            want.as_bytes(),
            |a| unsafe { (a.bad)(v) },
        );
    }
}

/// Row 19: `driver` with valid `goodData` and invalid `badData` — ordering of
/// the four labels plus the `good`/`bad` lines must match exactly.
#[test]
fn err19_driver_valid_good_invalid_bad_ordering() {
    for (bad_bits, bad_line) in [
        (0x0000_0000u32, INT_MIN_LINE),
        (0x8000_0000, INT_MIN_LINE),
        (0x7f80_0000, b"0\n" as &[u8]),
        (0xff80_0000, b"0\n"),
        (0x7fc0_0000, INT_MIN_LINE),
        (0x0000_0001, INT_MIN_LINE),
    ] {
        let mut expected = Vec::new();
        expected.extend_from_slice(b"Calling good()...\n");
        expected.extend_from_slice(G2B_LINE); // goodG2B: 100/2.0 == 50
        expected.extend_from_slice(b"50\n"); // goodB2G: 100/2.0 == 50
        expected.extend_from_slice(b"Finished good()\n");
        expected.extend_from_slice(b"Calling bad()...\n");
        expected.extend_from_slice(bad_line);
        expected.extend_from_slice(b"Finished bad()\n");
        expect(
            &format!("ERRORS row 19: driver(2.0, 0x{bad_bits:08x})"),
            &expected,
            |a| unsafe { (a.driver)(2.0, f(bad_bits)) },
        );
    }
}

/// Row 20: `driver` with BOTH arguments invalid — the rejection message from
/// `goodB2G` followed by the indefinite value from `bad`.
#[test]
fn err20_driver_both_arguments_invalid() {
    let good_bits: &[u32] = &[
        0x0000_0000,
        0x8000_0000,
        0x3586_37bd, // 1e-6f, just under the threshold
        0x7fc0_0000, // NaN
        0x0000_0001, // min subnormal
    ];
    let bad_cases: &[(u32, &[u8])] = &[
        (0x0000_0000, INT_MIN_LINE),
        (0x8000_0000, INT_MIN_LINE),
        (0x7fc0_0000, INT_MIN_LINE),
        (0x0000_0001, INT_MIN_LINE),
        (0x7f80_0000, b"0\n"),
        (0xff80_0000, b"0\n"),
        (0x3348_0000, INT_MIN_LINE),
    ];
    for &g in good_bits {
        for &(b, bad_line) in bad_cases {
            let mut expected = Vec::new();
            expected.extend_from_slice(b"Calling good()...\n");
            expected.extend_from_slice(G2B_LINE);
            expected.extend_from_slice(DIV_ZERO_MSG);
            expected.extend_from_slice(b"Finished good()\n");
            expected.extend_from_slice(b"Calling bad()...\n");
            expected.extend_from_slice(bad_line);
            expected.extend_from_slice(b"Finished bad()\n");
            expect(
                &format!("ERRORS row 20: driver(0x{g:08x}, 0x{b:08x})"),
                &expected,
                |a| unsafe { (a.driver)(f(g), f(b)) },
            );
        }
    }
}

// ===========================================================================
// Generic FFI boundary cases
// ===========================================================================

/// Row 21: zero-length string.
#[test]
fn err21_print_line_zero_length() {
    let s = cstr(b"");
    expect("ERRORS row 21: printLine(\"\")", b"\n", |a| unsafe {
        (a.print_line)(s.as_ptr().cast())
    });
}

/// Row 22: format specifiers in the DATA must not be interpreted — a Rust
/// translation that passed `line` as the format string would diverge here (and
/// `%n` would be a security hole).
#[test]
fn err22_print_line_format_specifiers_not_interpreted() {
    for pat in [
        &b"%d"[..],
        b"%s",
        b"%n",
        b"%%",
        b"%p%p%p%p%p%p%p%p%p%p",
        b"%99999999d",
        b"%.2000f",
        b"%hn%hhn%ln%lln",
        b"%1$n",
        b"%",
    ] {
        let s = cstr(pat);
        let mut expected = pat.to_vec();
        expected.push(b'\n');
        expect(
            &format!(
                "ERRORS row 22: printLine({:?}) literal",
                String::from_utf8_lossy(pat)
            ),
            &expected,
            |a| unsafe { (a.print_line)(s.as_ptr().cast()) },
        );
    }
}

/// Row 23: oversized string crossing the stdio buffer boundary.
#[test]
fn err23_print_line_oversized() {
    for &len in &[4096usize, 65536, 1 << 20] {
        let bytes = vec![b'Z'; len];
        let s = cstr(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        expect(
            &format!("ERRORS row 23: printLine(len={len})"),
            &expected,
            |a| unsafe { (a.print_line)(s.as_ptr().cast()) },
        );
    }
}

/// Row 24: high bytes / invalid UTF-8 must pass through unvalidated. A Rust
/// translation using `CStr::to_str()` would reject these.
#[test]
fn err24_print_line_invalid_utf8_passes_through() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xc3],             // truncated 2-byte sequence
        vec![0xe2, 0x82],       // truncated 3-byte sequence
        vec![0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        vec![0xed, 0xa0, 0x80], // UTF-16 surrogate encoded as UTF-8
        vec![0xc0, 0x80],       // overlong NUL encoding
        (0x80u16..=0xff).map(|b| b as u8).collect(),
    ];
    for bytes in cases {
        let s = cstr(&bytes);
        let mut expected = bytes.clone();
        expected.push(b'\n');
        expect(
            &format!("ERRORS row 24: printLine({bytes:02x?})"),
            &expected,
            |a| unsafe { (a.print_line)(s.as_ptr().cast()) },
        );
    }
}

/// Row 25: `printIntLine` at the extremes of the `int` range.
#[test]
fn err25_print_int_line_extremes() {
    expect(
        "ERRORS row 25: printIntLine(INT_MIN)",
        b"-2147483648\n",
        |a| unsafe { (a.print_int_line)(i32::MIN) },
    );
    expect(
        "ERRORS row 25: printIntLine(INT_MAX)",
        b"2147483647\n",
        |a| unsafe { (a.print_int_line)(i32::MAX) },
    );
    expect(
        "ERRORS row 25: printIntLine(INT_MIN+1)",
        b"-2147483647\n",
        |a| unsafe { (a.print_int_line)(i32::MIN + 1) },
    );
    expect(
        "ERRORS row 25: printIntLine(INT_MAX-1)",
        b"2147483646\n",
        |a| unsafe { (a.print_int_line)(i32::MAX - 1) },
    );
}

/// Row 26: `printIntLine(0)`.
#[test]
fn err26_print_int_line_zero() {
    expect("ERRORS row 26: printIntLine(0)", b"0\n", |a| unsafe {
        (a.print_int_line)(0)
    });
    expect("ERRORS row 26: printIntLine(-1)", b"-1\n", |a| unsafe {
        (a.print_int_line)(-1)
    });
}

/// Row 27: the public API declares no enum, so the analogue of an out-of-range
/// enum value crossing the FFI boundary is a non-canonical / out-of-range FLOAT
/// bit pattern. Sweep every float class, including every exponent value, and
/// require the two implementations to agree on all of them.
#[test]
fn err27_out_of_range_bit_patterns_across_ffi() {
    // Every biased exponent, with a few mantissas and both signs: covers zero,
    // subnormal, all normal magnitudes, infinity and NaN in one sweep.
    for exp in 0u32..=0xff {
        for mant in [0u32, 1, 0x0040_0000, 0x007f_ffff] {
            for sign in [0u32, 0x8000_0000] {
                let bits = sign | (exp << 23) | mant;
                let v = f(bits);
                diff(
                    &format!("ERRORS row 27: bad(0x{bits:08x}) exp={exp} mant=0x{mant:06x}"),
                    |a| unsafe { (a.bad)(v) },
                );
                diff(
                    &format!("ERRORS row 27: good(0x{bits:08x})"),
                    |a| unsafe { (a.good)(v) },
                );
            }
        }
    }
    // And the same for `int` values crossing the boundary at printIntLine.
    let mut rng = Rng::new(0x27_dead_beef);
    for _ in 0..500 {
        let v = rng.next_i32();
        diff(&format!("ERRORS row 27: printIntLine({v})"), |a| unsafe {
            (a.print_int_line)(v)
        });
    }
}

/// Row 28: `printLine` is the only pointer-taking entry point; confirm there is
/// no other null-pointer surface by exercising every export with its degenerate
/// argument and requiring agreement.
#[test]
fn err28_no_other_null_pointer_surface() {
    expect("ERRORS row 28: printLine(NULL)", b"", |a| unsafe {
        (a.print_line)(std::ptr::null())
    });
    // A NULL-adjacent (misaligned/low) non-null pointer is UB to dereference in
    // both implementations, so it is deliberately NOT exercised. Instead assert
    // the remaining exports take no pointers and behave identically on their
    // degenerate scalar inputs.
    expect("ERRORS row 28: printIntLine(0)", b"0\n", |a| unsafe {
        (a.print_int_line)(0)
    });
    expect("ERRORS row 28: bad(0.0)", INT_MIN_LINE, |a| unsafe {
        (a.bad)(0.0)
    });
    let mut good_expected = G2B_LINE.to_vec();
    good_expected.extend_from_slice(DIV_ZERO_MSG);
    expect("ERRORS row 28: good(0.0)", &good_expected, |a| unsafe {
        (a.good)(0.0)
    });
    let mut driver_expected = Vec::new();
    driver_expected.extend_from_slice(b"Calling good()...\n");
    driver_expected.extend_from_slice(G2B_LINE);
    driver_expected.extend_from_slice(DIV_ZERO_MSG);
    driver_expected.extend_from_slice(b"Finished good()\n");
    driver_expected.extend_from_slice(b"Calling bad()...\n");
    driver_expected.extend_from_slice(INT_MIN_LINE);
    driver_expected.extend_from_slice(b"Finished bad()\n");
    expect(
        "ERRORS row 28: driver(0.0, 0.0)",
        &driver_expected,
        |a| unsafe { (a.driver)(0.0, 0.0) },
    );
}

/// Extra guard: confirm the C `.so` really does reject NULL rather than the
/// harness silently swallowing output (protects against a false pass on row 1).
#[test]
fn err_harness_sanity_null_vs_nonnull() {
    let a = api(Impl::C);
    let s = cstr(b"sentinel");
    let non_null = capture(|| unsafe { (a.print_line)(s.as_ptr().cast()) });
    let null = capture(|| unsafe { (a.print_line)(std::ptr::null()) });
    assert_eq!(non_null, b"sentinel\n", "harness lost real output");
    assert!(null.is_empty(), "C printed {} for NULL", render(&null));
}
