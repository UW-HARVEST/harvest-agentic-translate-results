//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every test builds the exact rejecting input/condition, drives BOTH shared
//! objects (or both linked executables) with it, and asserts that the rejection
//! is identical: same bytes on stdout AND the same exit code / terminating
//! signal — not merely "both produced nothing".

#[macro_use]
mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1..6 — `driver`'s only guard, `i < x`
// ---------------------------------------------------------------------------

/// Row 1: `x == 0`.
fn err_01_driver_zero() {
    let c = driver_out_file(c_lib(), &[0]);
    let r = driver_out_file(rust_lib(), &[0]);
    assert_eq!(c, Vec::<u8>::new(), "C driver(0) must write nothing");
    assert_bytes_eq(&c, &r, "driver(0)");
}

/// Row 2: `x == -1`.
fn err_02_driver_minus_one() {
    let c = driver_out_file(c_lib(), &[-1]);
    let r = driver_out_file(rust_lib(), &[-1]);
    assert_eq!(c, Vec::<u8>::new(), "C driver(-1) must write nothing");
    assert_bytes_eq(&c, &r, "driver(-1)");
}

/// Row 3: `x == INT_MIN`.
fn err_03_driver_int_min() {
    let c = driver_out_file(c_lib(), &[i32::MIN]);
    let r = driver_out_file(rust_lib(), &[i32::MIN]);
    assert_eq!(c, Vec::<u8>::new(), "C driver(INT_MIN) must write nothing");
    assert_bytes_eq(&c, &r, "driver(INT_MIN)");
}

/// Row 4: `x == INT_MIN + 1` (one step past the extreme).
fn err_04_driver_int_min_plus_one() {
    for x in [i32::MIN + 1, i32::MIN + 2, -2, -1, 0, 1] {
        assert_driver_eq(&[x]);
    }
}

/// Row 5: randomized negative `x`.
fn err_05_driver_random_negative() {
    let mut rng = Rng::new(0xbad0_0005);
    for _ in 0..300 {
        let x = rng.range_i64(i32::MIN as i64, -1) as i32;
        let c = driver_out_file(c_lib(), &[x]);
        let r = driver_out_file(rust_lib(), &[x]);
        assert!(c.is_empty(), "C driver({x}) must write nothing");
        assert_bytes_eq(&c, &r, &format!("driver({x})"));
    }
}

/// Row 6: `x == INT_MAX`, compared on a bounded output prefix (the full output
/// would be ~2^31 lines).  Reached through `main` because that is how a caller
/// hands `driver` an unbounded `x`.
fn err_06_driver_int_max_prefix() {
    assert_exe_prefix_eq(b"2147483647\n", 64 * 1024);
}

// ---------------------------------------------------------------------------
// Rows 7..22 — every distinct `scanf("%d", &x)` rejection
// ---------------------------------------------------------------------------

/// Every row from 7 to 22 must leave stdout empty and exit 0; the helper checks
/// C and Rust agree *and* that C really rejected.
#[track_caller]
fn assert_rejected_identically(input: &[u8]) {
    // Through the `.so`'s exported `main`.
    let c = run_main_via_so(&c_so(), input, StdinKind::File);
    let r = run_main_via_so(&rust_so(), input, StdinKind::File);
    assert!(
        c.stdout.is_empty(),
        "C main() should have written nothing for {:?}, got {:?}",
        Show(input),
        Show(&c.stdout)
    );
    assert_bytes_eq(&c.stdout, &r.stdout, &format!("main(.so) {:?}", Show(input)));
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status mismatch for main(.so) {:?}",
        Show(input)
    );
    assert_eq!(c.code, Some(0), "C main() returns 0 for {:?}", Show(input));

    // And end to end through both executables, on a pipe and on a file.
    assert_exe_eq(input, ExeIo::Pipes);
    assert_exe_eq(input, ExeIo::Files);
}

/// Row 7: empty stdin → `scanf` input failure.
fn err_07_main_empty_stdin() {
    assert_rejected_identically(b"");
}

/// Row 8: whitespace only.
fn err_08_main_whitespace_only() {
    for s in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
        b"\n\n\n\n",
    ] {
        assert_rejected_identically(s);
    }
}

/// Row 9: first non-space byte is neither sign nor digit.
fn err_09_main_non_numeric() {
    for s in [
        &b"abc"[..],
        b"  abc",
        b"z",
        b"/",
        b":",
        b"\x7f",
        b"\xff\xfe",
        b"\x80",
    ] {
        assert_rejected_identically(s);
    }
}

/// Row 10: `'-'` followed by a non-digit.
fn err_10_main_minus_then_non_digit() {
    for s in ["-a", "- 5", "-\n5", "-.", "-/"] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 11: `'+'` followed by a non-digit.
fn err_11_main_plus_then_non_digit() {
    for s in ["+x", "+ 5", "+\n5", "+.", "+:"] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 12: `'-'` then EOF.
fn err_12_main_minus_then_eof() {
    assert_rejected_identically(b"-");
    assert_rejected_identically(b"   -");
}

/// Row 13: `'+'` then EOF.
fn err_13_main_plus_then_eof() {
    assert_rejected_identically(b"+");
    assert_rejected_identically(b" \t+");
}

/// Row 14: a NUL byte where a digit was expected.
fn err_14_main_leading_nul() {
    assert_rejected_identically(b"\x005");
    assert_rejected_identically(b"\x00");
    assert_rejected_identically(b"-\x005");
}

/// Row 15: two signs in a row.
fn err_15_main_double_sign() {
    for s in ["--5", "+-5", "-+5", "++5"] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 16: a leading decimal point (valid for `%f`, not for `%d`).
fn err_16_main_leading_dot() {
    for s in [".5", "-.5", ".", "-."] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 17: positive value overflowing `long` → `LONG_MAX` → `(int)-1`.
fn err_17_main_overflow_long_pos() {
    for s in [
        "99999999999999999999999",
        "9223372036854775808",
        "9223372036854775807",
        "+99999999999999999999999\n",
        "18446744073709551616",
    ] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 18: negative value overflowing `long` → `LONG_MIN` → `(int)0`.
fn err_18_main_overflow_long_neg() {
    for s in [
        "-99999999999999999999999",
        "-9223372036854775809",
        "-9223372036854775808",
        "-18446744073709551616\n",
    ] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 19: pathologically long digit run.
fn err_19_main_10k_digits() {
    let mut s = Vec::with_capacity(10_002);
    s.push(b'1');
    s.extend(std::iter::repeat(b'9').take(9_999));
    s.push(b'\n');
    assert_rejected_identically(&s);

    let mut s = Vec::with_capacity(10_003);
    s.push(b'-');
    s.push(b'1');
    s.extend(std::iter::repeat(b'7').take(9_999));
    s.push(b'\n');
    assert_rejected_identically(&s);

    // 10k leading zeros then a value: no overflow, converts to 5.
    let mut s: Vec<u8> = std::iter::repeat(b'0').take(10_000).collect();
    s.extend_from_slice(b"5\n");
    assert_main_so_eq(&s, StdinKind::File);
    assert_exe_eq(&s, ExeIo::Pipes);
}

/// Row 20: inside `long`, outside `int`, truncating to a non-positive `int`.
fn err_20_main_int_truncation_nonpositive() {
    for s in [
        "4294967296",     // 2^32      -> 0
        "2147483648",     // 2^31      -> INT_MIN
        "2147483649",     // 2^31+1    -> INT_MIN+1
        "-4294967296",    // -2^32     -> 0
        "-2147483648",    // fits      -> INT_MIN
        "4294967295",     // 2^32-1    -> -1
        "8589934592",     // 2^33      -> 0
        "-8589934592\n",  // -2^33     -> 0
        "1099511627776",  // 2^40      -> 0
    ] {
        assert_rejected_identically(s.as_bytes());
    }
}

/// Row 21: inside `long`, outside `int`, truncating to `INT_MAX` — unbounded
/// output, so compared on a bounded prefix.
fn err_21_main_int_truncation_to_int_max_prefix() {
    // -(2^31 + 1) -> INT_MAX
    assert_exe_prefix_eq(b"-2147483649\n", 64 * 1024);
    // -(2^32 - 2000000000) -> 2000000000
    assert_exe_prefix_eq(b"-2294967296\n", 64 * 1024);
}

/// Row 22: `%d` is base 10, so `"0x10"` converts `0` and stops at `'x'`.
fn err_22_main_hex_prefix() {
    for s in ["0x10", "0X10", "0b101", "0o7", "08", "-0x1"] {
        // `"08"` actually converts to 8, so only check for equality there.
        if s == "08" {
            assert_main_so_eq(s.as_bytes(), StdinKind::File);
            assert_exe_eq(s.as_bytes(), ExeIo::Pipes);
        } else {
            assert_rejected_identically(s.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 23..25 — the descriptor-level failures (`printf` / `scanf` I/O errors)
// ---------------------------------------------------------------------------

/// Row 23: stdout closed → `printf` fails with `EBADF`, return value discarded,
/// process still exits 0.
fn err_23_main_stdout_closed() {
    for s in ["5\n", "1", "1000\n", "", "abc"] {
        assert_exe_eq(s.as_bytes(), ExeIo::StdoutClosed);
        assert_main_so_eq_opts(
            s.as_bytes(),
            StdinKind::File,
            MainSoOpts {
                close_stdout: true,
                ..Default::default()
            },
        );
    }
    // C really does exit 0 in this configuration.
    let c = run_exe(&c_exe(), b"5\n", ExeIo::StdoutClosed);
    assert_eq!((c.code, c.signal), (Some(0), None));
}

/// Row 24: stdout is a pipe whose reader closed → killed by `SIGPIPE`.
fn err_24_main_sigpipe() {
    let c = run_exe_sigpipe(&c_exe(), b"200000\n");
    let r = run_exe_sigpipe(&rust_exe(), b"200000\n");
    assert_eq!(
        (c.code, c.signal),
        (None, Some(13)),
        "the C program must die from SIGPIPE"
    );
    assert_eq!(
        (r.code, r.signal),
        (c.code, c.signal),
        "SIGPIPE disposition mismatch: C={:?} rust={:?}",
        (c.code, c.signal),
        (r.code, r.signal)
    );
    assert_bytes_eq(&c.stdout, &r.stdout, "sigpipe prefix");
}

/// Row 25: stdin is a closed descriptor → `read` fails with `EBADF`, which
/// `scanf` reports as an input failure, so `x` keeps its initializer.
fn err_25_main_stdin_closed() {
    let c = run_exe(&c_exe(), b"5\n", ExeIo::StdinClosed);
    let r = run_exe(&rust_exe(), b"5\n", ExeIo::StdinClosed);
    assert!(
        c.stdout.is_empty(),
        "C must print nothing with stdin closed, got {:?}",
        Show(&c.stdout)
    );
    assert_bytes_eq(&c.stdout, &r.stdout, "exe with stdin closed");
    assert_eq!((c.code, c.signal), (r.code, r.signal));
    assert_eq!(c.code, Some(0));

    assert_main_so_eq_opts(
        b"5\n",
        StdinKind::File,
        MainSoOpts {
            close_stdin: true,
            ..Default::default()
        },
    );
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks (beyond the table)
// ---------------------------------------------------------------------------

/// `driver` takes a C `int`, so there is no invalid value: every 32-bit pattern
/// must behave identically.  Sweep the extremes and one step past them.
fn err_26_driver_full_int_range_boundaries() {
    let xs = [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -65536,
        -65535,
        -32769,
        -32768,
        -32767,
        -256,
        -255,
        -2,
        -1,
        0,
        1,
        2,
    ];
    for x in xs {
        assert_driver_eq(&[x]);
    }
}

/// A C `int` argument only occupies the low 32 bits of the argument register;
/// the upper half is unspecified.  Calling both `driver` symbols through a
/// `u64`-typed prototype must therefore give identical results.
fn err_27_driver_garbage_in_high_arg_bits() {
    for raw in [
        0xdead_beef_0000_0000u64,       // low half == 0
        0xdead_beef_0000_0005u64,       // low half == 5
        0xffff_ffff_ffff_ffffu64,       // low half == -1
        0x0000_0001_8000_0000u64,       // low half == INT_MIN
        0x7fff_ffff_0000_000au64,       // low half == 10
    ] {
        assert_driver_wide_arg_eq(raw);
    }
}

/// There is no `enum` in the C API, but the same class of bug — an integer the
/// API never expects — is covered by feeding `driver` values drawn from the
/// whole `i32` domain, and by feeding `main` decimal strings that land on those
/// values.  Both must agree.
fn err_28_driver_uniform_random_full_range() {
    let mut rng = Rng::new(0xbad0_0028);
    for _ in 0..200 {
        // Uniform over the full i32 domain, but clamp positives so the output
        // stays finite.
        let v = rng.next_u32() as i32;
        let x = if v > 4000 { v % 4000 } else { v };
        assert_driver_eq(&[x]);
    }
}

/// `scanf` failure must leave `x` at its initializer even when the *previous*
/// bytes on the stream were a perfectly good number that a second directive
/// would have consumed — there is only one directive in the C source.
fn err_29_main_only_one_directive() {
    for s in ["", "x1", "1x", "1 x", "x 1"] {
        let c = run_main_via_so(&c_so(), s.as_bytes(), StdinKind::Pipe);
        let r = run_main_via_so(&rust_so(), s.as_bytes(), StdinKind::Pipe);
        assert_bytes_eq(&c.stdout, &r.stdout, &format!("main(.so) {:?}", Show(s.as_bytes())));
        assert_eq!((c.code, c.signal), (r.code, r.signal));
    }
}

// ---------------------------------------------------------------------------
// Entry point (this target uses `harness = false`; see common::run_cases)
// ---------------------------------------------------------------------------

fn main() {
    common::run_cases(&[
        case!(err_01_driver_zero),
        case!(err_02_driver_minus_one),
        case!(err_03_driver_int_min),
        case!(err_04_driver_int_min_plus_one),
        case!(err_05_driver_random_negative),
        case!(err_06_driver_int_max_prefix),
        case!(err_07_main_empty_stdin),
        case!(err_08_main_whitespace_only),
        case!(err_09_main_non_numeric),
        case!(err_10_main_minus_then_non_digit),
        case!(err_11_main_plus_then_non_digit),
        case!(err_12_main_minus_then_eof),
        case!(err_13_main_plus_then_eof),
        case!(err_14_main_leading_nul),
        case!(err_15_main_double_sign),
        case!(err_16_main_leading_dot),
        case!(err_17_main_overflow_long_pos),
        case!(err_18_main_overflow_long_neg),
        case!(err_19_main_10k_digits),
        case!(err_20_main_int_truncation_nonpositive),
        case!(err_21_main_int_truncation_to_int_max_prefix),
        case!(err_22_main_hex_prefix),
        case!(err_23_main_stdout_closed),
        case!(err_24_main_sigpipe),
        case!(err_25_main_stdin_closed),
        case!(err_26_driver_full_int_range_boundaries),
        case!(err_27_driver_garbage_in_high_arg_bits),
        case!(err_28_driver_uniform_random_full_range),
        case!(err_29_main_only_one_directive),
    ]);
}
