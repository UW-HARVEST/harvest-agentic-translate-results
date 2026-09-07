//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact condition, calls
//! BOTH `.so` exports, and asserts the same rejection/result — the same bytes,
//! or for the stream-failure rows the same process outcome (exit code /
//! signal), not merely "both did something".

mod common;

use common::*;

/// Asserts C and Rust agree AND that the C output is exactly `expected`.
/// Pinning the C text as well guards against the pair agreeing on the *wrong*
/// thing (e.g. both silently emitting nothing).
fn expect_exact(row: &str, bits: u64, expected: &str) {
    let got = check_one(row, bits);
    assert_eq!(got, expected, "{row}: C reference text changed");
}

// --- row 1 ------------------------------------------------------------------

#[test]
fn err_01_no_error_return() {
    // `driver` is declared `void driver(double)`: there is no return value, no
    // out-parameter, and no errno contract, so the only observable channel is
    // stdout. Confirm that even the most hostile inputs make both libraries
    // terminate normally with matching output — i.e. nothing is ever rejected.
    let p = pair();
    let hostile = [
        0x0000_0000_0000_0000u64,
        0xffff_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
    ];
    check_row("ERRORS row 1 (no rejection branch exists)", &hostile);
    for &b in &hostile {
        let v = f64::from_bits(b);
        assert_eq!(
            outcome_normal(p.c_driver, v),
            ChildOutcome::Exited(0),
            "C aborted on 0x{b:016x}"
        );
        assert_eq!(
            outcome_normal(p.r_driver, v),
            ChildOutcome::Exited(0),
            "Rust aborted on 0x{b:016x}"
        );
    }
}

// --- row 2 ------------------------------------------------------------------

#[test]
fn err_02_printf_failure_ignored() {
    // fd 1 points at a read-only description, so every write fails with EBADF.
    // The C discards printf's return value; the Rust must likewise ignore the
    // failure and return normally (a panic would be SIGABRT under
    // `panic = "abort"`).
    let p = pair();
    for &b in &[
        1.0f64.to_bits(),
        f64::MAX.to_bits(),
        0u64,
        0x7ff8_0000_0000_0000u64,
    ] {
        let v = f64::from_bits(b);
        let c = outcome_with_broken_stdout(p.c_driver, v, BreakStdout::ReadOnly);
        let r = outcome_with_broken_stdout(p.r_driver, v, BreakStdout::ReadOnly);
        assert_eq!(c, ChildOutcome::Exited(0), "C did not survive a failing write");
        assert_eq!(r, c, "ERRORS row 2: outcome differs for 0x{b:016x} (C {c:?}, Rust {r:?})");
    }
}

// --- rows 3, 4 --------------------------------------------------------------

#[test]
fn err_03_null_pointer_na_but_abi_checked() {
    // `driver` has no pointer parameter, so there is nothing to null-check.
    // The substitutable check at this boundary is the calling convention: the
    // `double` must arrive in xmm0. If either side had the argument in an
    // integer register, `1.0` would not print as 3ff0000000000000, and a value
    // whose bit pattern is a small integer (0x0000000000000001, a plausible
    // "bad pointer") would not print as the subnormal it is.
    expect_exact(
        "ERRORS row 3 (ABI: double passed in xmm0, value 1.0)",
        1.0f64.to_bits(),
        "3ff0000000000000 0x1p+0 1.0000",
    );
    expect_exact(
        "ERRORS row 3 (ABI: bit pattern 1 is a subnormal, not a pointer)",
        1u64,
        "1 0x0.0000000000001p-1022 0.0000",
    );
    expect_exact(
        "ERRORS row 3 (ABI: all-ones bit pattern)",
        u64::MAX,
        "ffffffffffffffff -nan -nan",
    );
}

#[test]
fn err_04_enum_and_length_na_no_value_is_rejected() {
    // No enum and no length parameter exist, so the analogous "value with no
    // valid variant" is a `double` bit pattern with no valid numeric meaning:
    // NaNs, and every other pattern. Assert that *every* pattern is accepted —
    // exactly one output line per call, identical on both sides.
    let mut rng = Rng::new(SEED ^ 4);
    let v: Vec<u64> = (0..200_000).map(|_| rng.next_u64()).collect();
    check_row("ERRORS row 4 (no bit pattern is rejected, 200k)", &v);

    // And explicitly: one line per call, never zero and never two.
    let p = pair();
    let probe: Vec<u64> = v.iter().copied().take(512).collect();
    for (name, f) in [("C", p.c_driver), ("Rust", p.r_driver)] {
        let out = capture(|| {
            for &b in &probe {
                unsafe { f(f64::from_bits(b)) }
            }
        });
        assert_eq!(
            out.iter().filter(|&&c| c == b'\n').count(),
            probe.len(),
            "{name} did not emit exactly one line per call"
        );
    }
}

// --- rows 5..8: NaN ---------------------------------------------------------

#[test]
fn err_05_qnan_positive() {
    expect_exact("ERRORS row 5", 0x7ff8_0000_0000_0000, "7ff8000000000000 nan nan");
}

#[test]
fn err_06_qnan_negative() {
    expect_exact("ERRORS row 6", 0xfff8_0000_0000_0000, "fff8000000000000 -nan -nan");
}

#[test]
fn err_07_snan() {
    expect_exact("ERRORS row 7", 0x7ff4_0000_0000_0000, "7ff4000000000000 nan nan");
    expect_exact("ERRORS row 7 (negative sNaN)", 0xfff4_0000_0000_0000, "fff4000000000000 -nan -nan");
}

#[test]
fn err_08_nan_noncanonical_payloads() {
    expect_exact(
        "ERRORS row 8 (smallest NaN payload)",
        0x7ff0_0000_0000_0001,
        "7ff0000000000001 nan nan",
    );
    expect_exact(
        "ERRORS row 8 (largest NaN payload, sign set)",
        0xffff_ffff_ffff_ffff,
        "ffffffffffffffff -nan -nan",
    );
    // The payload shows up in %llx but never in %a / %.4f.
    let mut rng = Rng::new(SEED ^ 8);
    let v: Vec<u64> = (0..4096)
        .map(|_| {
            let m = loop {
                let m = rng.next_u64() & MANT_MASK;
                if m != 0 {
                    break m;
                }
            };
            bits_of(rng.next_u64() & 1 == 1, 0x7ff, m)
        })
        .collect();
    check_row("ERRORS row 8 (random NaN payloads)", &v);
}

// --- rows 9, 10: infinities -------------------------------------------------

#[test]
fn err_09_pos_inf() {
    expect_exact("ERRORS row 9", 0x7ff0_0000_0000_0000, "7ff0000000000000 inf inf");
}

#[test]
fn err_10_neg_inf() {
    expect_exact("ERRORS row 10", 0xfff0_0000_0000_0000, "fff0000000000000 -inf -inf");
}

// --- rows 11, 12: zeros -----------------------------------------------------

#[test]
fn err_11_pos_zero() {
    expect_exact("ERRORS row 11", 0, "0 0x0p+0 0.0000");
}

#[test]
fn err_12_neg_zero() {
    expect_exact("ERRORS row 12", SIGN, "8000000000000000 -0x0p+0 -0.0000");
}

// --- rows 13..16: subnormal / normal boundary -------------------------------

#[test]
fn err_13_min_subnormal() {
    expect_exact("ERRORS row 13", 1, "1 0x0.0000000000001p-1022 0.0000");
}

#[test]
fn err_14_max_subnormal() {
    expect_exact(
        "ERRORS row 14",
        MANT_MASK,
        "fffffffffffff 0x0.fffffffffffffp-1022 0.0000",
    );
}

#[test]
fn err_15_neg_min_subnormal() {
    expect_exact(
        "ERRORS row 15",
        SIGN | 1,
        "8000000000000001 -0x0.0000000000001p-1022 -0.0000",
    );
}

#[test]
fn err_16_dbl_min_normal() {
    // Same p-1022 exponent as the subnormals, but leading digit 1.
    expect_exact("ERRORS row 16", 0x0010_0000_0000_0000, "10000000000000 0x1p-1022 0.0000");
}

// --- rows 17, 18: DBL_MAX ---------------------------------------------------

#[test]
fn err_17_dbl_max() {
    let line = check_one("ERRORS row 17", 0x7fef_ffff_ffff_ffff);
    let mut parts = line.split(' ');
    assert_eq!(parts.next(), Some("7fefffffffffffff"));
    assert_eq!(parts.next(), Some("0x1.fffffffffffffp+1023"));
    let fixed = parts.next().expect("third field");
    assert_eq!(parts.next(), None);
    assert!(fixed.ends_with(".0000"), "got {fixed}");
    assert_eq!(
        fixed.len() - ".0000".len(),
        309,
        "DBL_MAX should have 309 integer digits, got {}",
        fixed.len() - 5
    );
}

#[test]
fn err_18_neg_dbl_max() {
    let line = check_one("ERRORS row 18", 0xffef_ffff_ffff_ffff);
    assert!(line.starts_with("ffefffffffffffff -0x1.fffffffffffffp+1023 -"), "got {line}");
    assert!(line.ends_with(".0000"), "got {line}");
}

// --- rows 19..23: %.4f rounding --------------------------------------------

#[test]
fn err_19_just_below_rounding_boundary() {
    // Largest double strictly below 0.00005.
    let b = 5e-5f64.to_bits() - 1;
    let line = check_one("ERRORS row 19", b);
    assert!(line.ends_with(" 0.0000"), "expected round-down, got {line}");
}

#[test]
fn err_20_nearest_double_to_tie() {
    // 0.00005 is not representable; the nearest double is above the tie, so the
    // exact expansion must round up.
    let b = 5e-5f64.to_bits();
    let line = check_one("ERRORS row 20", b);
    assert!(
        line.ends_with(" 0.0001"),
        "nearest double to 0.00005 is above the tie, expected round-up, got {line}"
    );
}

#[test]
fn err_21_exact_tie_even() {
    // 0.03125 == 1/32 exactly -> fraction digits 03125; 4th digit 2 is even, so
    // round-half-to-even keeps it.
    expect_exact("ERRORS row 21", 0.03125f64.to_bits(), "3fa0000000000000 0x1p-5 0.0312");
    expect_exact(
        "ERRORS row 21 (negative)",
        (-0.03125f64).to_bits(),
        "bfa0000000000000 -0x1p-5 -0.0312",
    );
}

#[test]
fn err_22_exact_tie_odd() {
    // 0.09375 == 3/32 exactly -> digits 09375; 4th digit 7 is odd, so it rounds up.
    expect_exact("ERRORS row 22", 0.09375f64.to_bits(), "3fb8000000000000 0x1.8p-4 0.0938");
    expect_exact(
        "ERRORS row 22 (negative)",
        (-0.09375f64).to_bits(),
        "bfb8000000000000 -0x1.8p-4 -0.0938",
    );
}

#[test]
fn err_23_rounding_carry() {
    for v in [0.99999f64, 9.99999, -0.99999, -9.99999, 0.99995, 1.99995] {
        let line = check_one("ERRORS row 23", v.to_bits());
        // The point of the row is C/Rust agreement plus a visible carry.
        assert!(line.contains('.'), "got {line}");
    }
    let mut rng = Rng::new(SEED ^ 23);
    let v: Vec<u64> = (0..4096)
        .map(|_| {
            let k = rng.below(1_000_000);
            let x = (k as f64) + 0.99999_f64;
            if rng.next_u64() & 1 == 1 { (-x).to_bits() } else { x.to_bits() }
        })
        .collect();
    check_row("ERRORS row 23 (carry into the integer part)", &v);
}

// --- rows 24..29: field boundaries -----------------------------------------

#[test]
fn err_24_max_exponent_field_not_inf() {
    expect_exact(
        "ERRORS row 24",
        0x7fe0_0000_0000_0000,
        &format!(
            "7fe0000000000000 0x1p+1023 {}",
            check_one("ERRORS row 24 (probe)", 0x7fe0_0000_0000_0000)
                .split(' ')
                .nth(2)
                .expect("fixed field")
        ),
    );
    // The distinguishing assertion: it must NOT be "inf".
    let line = check_one("ERRORS row 24 (not inf)", 0x7fe0_0000_0000_0000);
    assert!(!line.contains("inf"), "0x7fe0000000000000 must not print as inf: {line}");
}

#[test]
fn err_25_trailing_zero_trim() {
    expect_exact(
        "ERRORS row 25",
        0x3ff1_2300_0000_0000,
        "3ff1230000000000 0x1.123p+0 1.0710",
    );
}

#[test]
fn err_26_low_nibble_set_no_trim() {
    expect_exact(
        "ERRORS row 26",
        0x3ff0_0000_0000_0001,
        "3ff0000000000001 0x1.0000000000001p+0 1.0000",
    );
}

#[test]
fn err_27_exponent_zero_boundary() {
    expect_exact("ERRORS row 27", 0x3ff0_0000_0000_0000, "3ff0000000000000 0x1p+0 1.0000");
}

#[test]
fn err_28_exponent_minus_one_boundary() {
    expect_exact(
        "ERRORS row 28",
        0x3fef_ffff_ffff_ffff,
        "3fefffffffffffff 0x1.fffffffffffffp-1 1.0000",
    );
}

#[test]
fn err_29_single_hex_digit_llx() {
    for b in 0u64..=0xf {
        let line = check_one("ERRORS row 29", b);
        let first = line.split(' ').next().expect("llx field");
        assert_eq!(first.len(), 1, "%llx of {b} must be one digit, got {first:?}");
        assert_eq!(first, &format!("{b:x}"));
    }
}

// --- row 30: stdout closed --------------------------------------------------

#[test]
fn err_30_stdout_closed() {
    let p = pair();
    for &v in &[1.0f64, 0.0, f64::MAX, f64::NAN, f64::NEG_INFINITY] {
        let c = outcome_with_broken_stdout(p.c_driver, v, BreakStdout::Closed);
        let r = outcome_with_broken_stdout(p.r_driver, v, BreakStdout::Closed);
        assert_eq!(
            c,
            ChildOutcome::Exited(0),
            "C did not survive close(1) for {v}"
        );
        assert_eq!(
            r, c,
            "ERRORS row 30: outcome differs for {v} (C {c:?}, Rust {r:?})"
        );
    }
}

// --- row 31: shared buffered FILE ------------------------------------------

#[test]
fn err_31_repeated_calls_share_buffer() {
    let p = pair();
    let vals = [1.0f64, 2.0, 0.5, -0.0, f64::MAX, 1e-300];

    // Reference: each value's line, produced by C alone.
    let mut reference = Vec::new();
    for &v in &vals {
        reference.extend_from_slice(&capture(|| unsafe { (p.c_driver)(v) }));
    }

    // Now interleave C and Rust calls inside ONE capture with no intervening
    // flush. If the Rust library used its own descriptor or its own buffer
    // instead of glibc's `stdout` FILE, the ordering here would break.
    let interleaved = capture(|| {
        for (i, &v) in vals.iter().enumerate() {
            unsafe {
                if i % 2 == 0 {
                    (p.c_driver)(v)
                } else {
                    (p.r_driver)(v)
                }
            }
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&interleaved),
        String::from_utf8_lossy(&reference),
        "ERRORS row 31: interleaved C/Rust output through the shared FILE differs"
    );

    // And the reverse interleaving, to catch order-dependent buffering.
    let interleaved2 = capture(|| {
        for (i, &v) in vals.iter().enumerate() {
            unsafe {
                if i % 2 == 0 {
                    (p.r_driver)(v)
                } else {
                    (p.c_driver)(v)
                }
            }
        }
    });
    assert_eq!(
        String::from_utf8_lossy(&interleaved2),
        String::from_utf8_lossy(&reference)
    );
}

// --- row 32: wide-oriented stream -------------------------------------------

#[test]
fn err_32_wide_oriented_stream() {
    // `fwide(stdout, 1)` makes the stream wide-oriented. glibc's byte functions
    // (`printf`, `fwrite`) then refuse to operate on it. Orientation is sticky
    // for the life of the stream, so this runs in a forked child.
    let p = pair();
    for &v in &[1.0f64, -0.0, f64::MAX, f64::NAN] {
        let (c_out, c_bytes) =
            outcome_and_output_in_child(p.c_driver, v, |s| {
                assert!(fwide_stream(s, 1) > 0, "could not force wide orientation");
            });
        let (r_out, r_bytes) =
            outcome_and_output_in_child(p.r_driver, v, |s| {
                assert!(fwide_stream(s, 1) > 0, "could not force wide orientation");
            });
        assert_eq!(c_out, ChildOutcome::Exited(0), "C did not survive fwide for {v}");
        assert_eq!(
            r_out, c_out,
            "ERRORS row 32: outcome differs for {v} (C {c_out:?}, Rust {r_out:?})"
        );
        assert_eq!(
            String::from_utf8_lossy(&r_bytes),
            String::from_utf8_lossy(&c_bytes),
            "ERRORS row 32: output on a wide-oriented stream differs for {v}"
        );
    }
}

// --- row 33: buffering mode -------------------------------------------------

#[test]
fn err_33_buffering_modes() {
    // Unbuffered / line-buffered / fully-buffered `stdout`. `setvbuf` must be
    // called before any I/O on the stream, so this runs in a forked child too.
    let p = pair();
    for (name, mode) in [("_IONBF", IONBF), ("_IOLBF", IOLBF), ("_IOFBF", IOFBF)] {
        for &v in &[1.0f64, -0.0, f64::MAX, f64::NAN, 0.09375] {
            let (c_out, c_bytes) = outcome_and_output_in_child(p.c_driver, v, |s| {
                setvbuf_stream(s, mode);
            });
            let (r_out, r_bytes) = outcome_and_output_in_child(p.r_driver, v, |s| {
                setvbuf_stream(s, mode);
            });
            assert_eq!(c_out, ChildOutcome::Exited(0), "C failed under {name} for {v}");
            assert_eq!(r_out, c_out, "ERRORS row 33 ({name}): outcome differs for {v}");
            assert!(!c_bytes.is_empty(), "C wrote nothing under {name}");
            assert_eq!(
                String::from_utf8_lossy(&r_bytes),
                String::from_utf8_lossy(&c_bytes),
                "ERRORS row 33 ({name}): output differs for {v}"
            );
        }
    }
}

// --- soak: heavier randomized sweep (run explicitly) ------------------------

/// Not part of the default run because of its duration. Execute with:
/// `cargo test --release -- --ignored soak`
#[test]
#[ignore]
fn soak_20m_random_values() {
    let mut rng = Rng::new(SEED ^ 0x50AC);
    let total = 20_000_000usize;
    let batch = 100_000usize;
    let mut done = 0usize;
    while done < total {
        let n = batch.min(total - done);
        let v: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
        check_row(&format!("soak batch at {done}"), &v);
        done += n;
    }
    eprintln!("soak: {total} random bit patterns verified");
}

/// Exhaustive over a contiguous 2^22 slice of the mantissa for several
/// exponents, including the subnormal and max-normal ones. Also `--ignored`.
#[test]
#[ignore]
fn soak_exhaustive_mantissa_slices() {
    for e in [0u64, 1, 0x3fd, 0x3fe, 0x3ff, 0x400, 0x7fe, 0x7ff] {
        for sign in [false, true] {
            let v: Vec<u64> = (0..(1u64 << 22)).map(|m| bits_of(sign, e, m)).collect();
            check_row(&format!("soak exhaustive mantissa slice e={e:#x} sign={sign}"), &v);
        }
    }
}
