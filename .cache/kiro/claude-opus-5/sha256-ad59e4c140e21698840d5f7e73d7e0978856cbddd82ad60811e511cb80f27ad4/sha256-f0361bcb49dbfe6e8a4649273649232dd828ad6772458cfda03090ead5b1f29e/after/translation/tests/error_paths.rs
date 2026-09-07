//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`, plus the generic FFI-boundary abuse cases.
//!
//! Every function in this library returns `void`, so "the same error/rejection"
//! means: the same *sentinel output*. The two distinguishable rejections are
//!   * `printLine(NULL)`  -> no bytes at all, and
//!   * the `goodB2G` guard -> the literal line `This would result in a divide by zero`,
//! and the divide-by-zero flaw in `bad()` surfaces as the integer-indefinite
//! sentinel `-2147483648`. Each test asserts the C and Rust bytes are equal AND
//! pins the exact expected sentinel, so "both failed somehow" cannot pass.

mod common;

use common::{capture, c_api, diff, rust_api, Api};
use std::ffi::{c_char, CString};

const DIVZERO: &str = "This would result in a divide by zero\n";
const INDEFINITE: &str = "-2147483648\n";

/// Asserts C == Rust *and* that the shared output is exactly `expected`.
fn diff_exact<F>(label: &str, expected: &str, f: F)
where
    F: Fn(&Api),
{
    diff(label, &f);
    let got = capture(|| f(c_api()));
    assert_eq!(
        String::from_utf8_lossy(&got),
        expected,
        "[{label}] C sentinel changed: expected {expected:?}"
    );
    let got_r = capture(|| f(rust_api()));
    assert_eq!(
        String::from_utf8_lossy(&got_r),
        expected,
        "[{label}] Rust sentinel mismatch: expected {expected:?}"
    );
}

// --- Row 1: printLine(NULL) -----------------------------------------------

#[test]
fn err_01_print_line_null() {
    diff_exact("err1 printLine(NULL)", "", |api| unsafe {
        (api.print_line)(std::ptr::null())
    });
    // Repeated and interleaved with a valid call: the guard must not consume or
    // emit anything, and must not disturb the following line.
    let ok = CString::new("after").unwrap();
    diff_exact("err1 printLine(NULL) x3 then valid", "after\n", |api| unsafe {
        (api.print_line)(std::ptr::null());
        (api.print_line)(std::ptr::null());
        (api.print_line)(std::ptr::null());
        (api.print_line)(ok.as_ptr());
    });
}

// --- Row 2: printLine("") --------------------------------------------------

#[test]
fn err_02_print_line_empty() {
    let empty = CString::new("").unwrap();
    diff_exact("err2 printLine(\"\")", "\n", |api| unsafe {
        (api.print_line)(empty.as_ptr())
    });
}

// --- Row 3: good(0.0) ------------------------------------------------------

#[test]
fn err_03_good_zero() {
    let expected = format!("50\n{DIVZERO}");
    diff_exact("err3 good(+0.0)", &expected, |api| unsafe { (api.good)(0.0) });
}

// --- Row 4: good(-0.0) ----------------------------------------------------

#[test]
fn err_04_good_negative_zero() {
    let expected = format!("50\n{DIVZERO}");
    diff_exact("err4 good(-0.0)", &expected, |api| unsafe { (api.good)(-0.0) });
}

// --- Row 5: good(tiny but finite) -----------------------------------------

#[test]
fn err_05_good_tiny_but_finite() {
    let expected = format!("50\n{DIVZERO}");
    for (name, v) in [
        ("5e-07", 5e-7f32),
        ("-5e-07", -5e-7f32),
        ("1e-30", 1e-30f32),
        ("-1e-30", -1e-30f32),
        ("FLT_MIN", f32::MIN_POSITIVE),
        ("-FLT_MIN", -f32::MIN_POSITIVE),
        ("FLT_TRUE_MIN", f32::from_bits(1)),
        ("-FLT_TRUE_MIN", -f32::from_bits(1)),
        ("1e-45", 1e-45f32),
    ] {
        diff_exact(&format!("err5 good({name})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

// --- Row 6: good(1e-6f) exactly at the threshold --------------------------

#[test]
fn err_06_good_threshold_exact() {
    let expected = format!("50\n{DIVZERO}");
    // (double)1e-6f == 9.99999997475243e-07 < 1e-6, so this is REJECTED.
    diff_exact("err6 good(1e-6f)", &expected, |api| unsafe {
        (api.good)(common::GUARD_F32)
    });
    diff_exact("err6 good(-1e-6f)", &expected, |api| unsafe {
        (api.good)(-common::GUARD_F32)
    });
    // And one ULP below stays rejected.
    let down = f32::from_bits(common::GUARD_F32.to_bits() - 1);
    diff_exact("err6 good(1e-6f - 1ulp)", &expected, |api| unsafe {
        (api.good)(down)
    });
}

// --- Row 7: good(NaN) -----------------------------------------------------

#[test]
fn err_07_good_nan() {
    let expected = format!("50\n{DIVZERO}");
    for (name, bits) in [
        ("qnan", 0x7FC0_0000u32),
        ("-qnan", 0xFFC0_0000),
        ("snan", 0x7F80_0001),
        ("-snan", 0xFF80_0001),
        ("nan_payload", 0x7FDE_ADBE),
    ] {
        let v = f32::from_bits(bits);
        diff_exact(&format!("err7 good({name})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

// --- Row 8: good(nextafterf(1e-6f, 1)) — first accepted value -------------

#[test]
fn err_08_good_one_step_past_threshold() {
    let up = common::guard_next_up();
    // Guard passes; 100.0 / 1.00000011e-06 == 99999989.something -> 99999989.
    let out = capture(|| unsafe { (c_api().good)(up) });
    let text = String::from_utf8_lossy(&out).to_string();
    assert!(
        text.starts_with("50\n") && !text.contains("divide by zero"),
        "err8: expected the guard to ACCEPT {up:e}, C printed {text:?}"
    );
    diff_exact("err8 good(guard+1ulp)", &text, |api| unsafe { (api.good)(up) });

    let up_neg = -up;
    let out_n = capture(|| unsafe { (c_api().good)(up_neg) });
    let text_n = String::from_utf8_lossy(&out_n).to_string();
    assert!(!text_n.contains("divide by zero"), "err8: -guard-1ulp should be accepted");
    diff_exact("err8 good(-(guard+1ulp))", &text_n, |api| unsafe {
        (api.good)(up_neg)
    });
}

// --- Row 9: bad(0.0) -> +inf -> integer indefinite ------------------------

#[test]
fn err_09_bad_zero() {
    diff_exact("err9 bad(+0.0)", INDEFINITE, |api| unsafe { (api.bad)(0.0) });
}

// --- Row 10: bad(-0.0) -> -inf -> integer indefinite ----------------------

#[test]
fn err_10_bad_negative_zero() {
    diff_exact("err10 bad(-0.0)", INDEFINITE, |api| unsafe { (api.bad)(-0.0) });
}

// --- Row 11: bad(NaN) -----------------------------------------------------

#[test]
fn err_11_bad_nan() {
    for (name, bits) in [
        ("qnan", 0x7FC0_0000u32),
        ("-qnan", 0xFFC0_0000),
        ("snan", 0x7F80_0001),
        ("-snan", 0xFF80_0001),
        ("nan_payload", 0x7FDE_ADBE),
    ] {
        let v = f32::from_bits(bits);
        diff_exact(&format!("err11 bad({name})"), INDEFINITE, |api| unsafe {
            (api.bad)(v)
        });
    }
}

// --- Row 12: bad(tiny positive) -> quotient past INT_MAX ------------------

#[test]
fn err_12_bad_overflow_positive() {
    for (name, v) in [
        ("1e-30", 1e-30f32),
        ("1e-40", 1e-40f32),
        ("FLT_MIN", f32::MIN_POSITIVE),
        ("FLT_TRUE_MIN", f32::from_bits(1)),
        ("1e-20", 1e-20f32),
        ("1e-10", 1e-10f32),
        ("cast_limit", common::CAST_LIMIT as f32),
    ] {
        diff_exact(&format!("err12 bad({name})"), INDEFINITE, |api| unsafe {
            (api.bad)(v)
        });
    }
}

// --- Row 13: bad(tiny negative) -> quotient past INT_MIN ------------------

#[test]
fn err_13_bad_overflow_negative() {
    for (name, v) in [
        ("-1e-30", -1e-30f32),
        ("-1e-40", -1e-40f32),
        ("-FLT_MIN", -f32::MIN_POSITIVE),
        ("-FLT_TRUE_MIN", -f32::from_bits(1)),
        ("-1e-20", -1e-20f32),
        ("-1e-10", -1e-10f32),
        ("-cast_limit", -(common::CAST_LIMIT as f32)),
    ] {
        diff_exact(&format!("err13 bad({name})"), INDEFINITE, |api| unsafe {
            (api.bad)(v)
        });
    }
}

// --- Row 14: bad(±inf) -> 0, NOT an error --------------------------------

#[test]
fn err_14_bad_infinity() {
    diff_exact("err14 bad(+inf)", "0\n", |api| unsafe {
        (api.bad)(f32::INFINITY)
    });
    diff_exact("err14 bad(-inf)", "0\n", |api| unsafe {
        (api.bad)(f32::NEG_INFINITY)
    });
    // good() also accepts infinities (inf > 1e-6) and prints 0.
    diff_exact("err14 good(+inf)", "50\n0\n", |api| unsafe {
        (api.good)(f32::INFINITY)
    });
    diff_exact("err14 good(-inf)", "50\n0\n", |api| unsafe {
        (api.good)(f32::NEG_INFINITY)
    });
}

// --- Row 15: truncation toward zero, both signs ---------------------------

#[test]
fn err_15_truncation_toward_zero() {
    diff_exact("err15 bad(3.0)", "33\n", |api| unsafe { (api.bad)(3.0) });
    diff_exact("err15 bad(-3.0)", "-33\n", |api| unsafe { (api.bad)(-3.0) });
    diff_exact("err15 bad(7.0)", "14\n", |api| unsafe { (api.bad)(7.0) });
    diff_exact("err15 bad(-7.0)", "-14\n", |api| unsafe { (api.bad)(-7.0) });
    diff_exact("err15 bad(101.0)", "0\n", |api| unsafe { (api.bad)(101.0) });
    diff_exact("err15 bad(-101.0)", "0\n", |api| unsafe { (api.bad)(-101.0) });
    diff_exact("err15 bad(200.0)", "0\n", |api| unsafe { (api.bad)(200.0) });
    diff_exact("err15 bad(-200.0)", "0\n", |api| unsafe { (api.bad)(-200.0) });
    diff_exact("err15 good(3.0)", "50\n33\n", |api| unsafe { (api.good)(3.0) });
    diff_exact("err15 good(-3.0)", "50\n-33\n", |api| unsafe { (api.good)(-3.0) });
}

// --- Row 16: printIntLine with no validation -----------------------------

#[test]
fn err_16_print_int_line_extremes() {
    diff_exact("err16 printIntLine(INT_MIN)", "-2147483648\n", |api| unsafe {
        (api.print_int_line)(i32::MIN)
    });
    diff_exact("err16 printIntLine(INT_MAX)", "2147483647\n", |api| unsafe {
        (api.print_int_line)(i32::MAX)
    });
    diff_exact("err16 printIntLine(-1)", "-1\n", |api| unsafe {
        (api.print_int_line)(-1)
    });
    diff_exact("err16 printIntLine(0)", "0\n", |api| unsafe {
        (api.print_int_line)(0)
    });
    // Out-of-range "enum-like" values crossing the FFI boundary: this API takes a
    // plain `int`, so every bit pattern is a legal input and must round-trip.
    for v in [
        -2i32, 12345, -12345, 0x7FFF_FFFE, -0x7FFF_FFFF, 1 << 30, -(1 << 30), 999, -999,
    ] {
        let expected = format!("{v}\n");
        diff_exact(
            &format!("err16 printIntLine({v})"),
            &expected,
            |api| unsafe { (api.print_int_line)(v) },
        );
    }
}

// --- Row 17: driver() with both arguments invalid ------------------------

#[test]
fn err_17_driver_both_invalid() {
    let expected = format!(
        "Calling good()...\n50\n{DIVZERO}Finished good()\nCalling bad()...\n{INDEFINITE}Finished bad()\n"
    );
    diff_exact("err17 driver(0.0, 0.0)", &expected, |api| unsafe {
        (api.driver)(0.0, 0.0)
    });
    diff_exact("err17 driver(NaN, NaN)", &expected, |api| unsafe {
        (api.driver)(f32::NAN, f32::NAN)
    });
    diff_exact("err17 driver(-0.0, -0.0)", &expected, |api| unsafe {
        (api.driver)(-0.0, -0.0)
    });
    diff_exact("err17 driver(1e-30, 1e-30)", &expected, |api| unsafe {
        (api.driver)(1e-30, 1e-30)
    });
    // Mixed validity: good rejected, bad fine.
    let mixed = "Calling good()...\n50\n".to_string()
        + DIVZERO
        + "Finished good()\nCalling bad()...\n50\nFinished bad()\n";
    diff_exact("err17 driver(0.0, 2.0)", &mixed, |api| unsafe {
        (api.driver)(0.0, 2.0)
    });
    // good accepted, bad divides by zero — the canonical CWE-369 demo run.
    let canonical = "Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n".to_string()
        + INDEFINITE
        + "Finished bad()\n";
    diff_exact("err17 driver(2.0, 0.0)", &canonical, |api| unsafe {
        (api.driver)(2.0, 0.0)
    });
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary abuse, beyond the table
// ---------------------------------------------------------------------------

/// A pointer that is non-NULL but points at a lone NUL: the shortest legal
/// string. Combined with a pointer into the middle of a buffer, this checks the
/// null guard is a *pointer* test and nothing more.
#[test]
fn generic_print_line_pointer_shapes() {
    let buf = b"abc\0def\0";
    // Pointer into the middle of a buffer, past the first NUL.
    diff_exact("generic printLine(&buf[4])", "def\n", |api| unsafe {
        (api.print_line)(buf[4..].as_ptr() as *const c_char)
    });
    // Pointer directly at a NUL byte.
    diff_exact("generic printLine(&buf[3])", "\n", |api| unsafe {
        (api.print_line)(buf[3..].as_ptr() as *const c_char)
    });
    // A heap string freed... no: instead a string that is exactly the stdio
    // buffer size, to catch off-by-one flushing differences.
    for len in [4095usize, 4096, 4097, 8191, 8192] {
        let mut v: Vec<u8> = std::iter::repeat(b'z').take(len).collect();
        v.push(0);
        let expected = format!("{}\n", "z".repeat(len));
        diff_exact(
            &format!("generic printLine(len={len})"),
            &expected,
            |api| unsafe { (api.print_line)(v.as_ptr() as *const c_char) },
        );
    }
}

/// C enums accept any `int`. `printIntLine` is the only int-taking entry point,
/// so exhaustively probe values one step past every "documented" range we could
/// construct, plus a swept sample of the full domain.
#[test]
fn generic_out_of_range_int_values() {
    let mut probes: Vec<i32> = Vec::new();
    for shift in 0..31 {
        probes.push(1i32 << shift);
        probes.push(-(1i32 << shift));
        probes.push((1i32 << shift) - 1);
        probes.push((1i32 << shift).wrapping_neg().wrapping_sub(1));
    }
    probes.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1, 1]);
    for v in probes {
        let expected = format!("{v}\n");
        diff_exact(&format!("generic printIntLine({v})"), &expected, |api| unsafe {
            (api.print_int_line)(v)
        });
    }
}

/// Every float whose exponent field is extreme, at both signs — the "one step
/// past the valid range" class for the `float` entry points. Because these are
/// UB in C, the point is only that Rust reproduces whatever the C binary does.
#[test]
fn generic_float_exponent_sweep() {
    for exp in 0u32..=255 {
        for &mant in &[0u32, 1, 0x0040_0000, 0x007F_FFFF] {
            for &sign in &[0u32, 1] {
                let bits = (sign << 31) | (exp << 23) | mant;
                let v = f32::from_bits(bits);
                diff(&format!("generic bad(0x{bits:08x})"), |api| unsafe {
                    (api.bad)(v)
                });
                diff(&format!("generic good(0x{bits:08x})"), |api| unsafe {
                    (api.good)(v)
                });
            }
        }
    }
}

/// Zero-length / oversized "lengths" have no analogue in this API (no length
/// parameters exist), so the equivalent boundary is calling each entry point
/// zero times, once, and many times in a row and checking the output stream is
/// identical — i.e. that neither library carries hidden state between calls.
#[test]
fn generic_statelessness_across_repeated_calls() {
    diff_exact("generic no calls", "", |_api| {});
    let expected: String = std::iter::repeat("50\n").take(100).collect();
    diff_exact("generic bad(2.0) x100", &expected, |api| unsafe {
        for _ in 0..100 {
            (api.bad)(2.0);
        }
    });
    let expected2: String = std::iter::repeat(format!("50\n{DIVZERO}")).take(50).collect();
    diff_exact("generic good(0.0) x50", &expected2, |api| unsafe {
        for _ in 0..50 {
            (api.good)(0.0);
        }
    });
}
