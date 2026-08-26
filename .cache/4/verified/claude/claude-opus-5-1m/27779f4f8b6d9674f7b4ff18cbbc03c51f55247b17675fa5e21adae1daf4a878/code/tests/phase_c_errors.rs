//! Phase C — error/rejection-path differential tests, one test per row of
//! ERRORS.md (rows E1..E23, F5, F9, F10).
//!
//! Rows F1..F4 and F6..F8 need an fd-1 capture and therefore live in the
//! single-threaded `tests/ffi_inproc.rs` binary.
//!
//! Every test
//!   * builds the exact invalid input / condition,
//!   * asserts that the **C** implementation produces the result documented in
//!     ERRORS.md (so the test really does hit the intended rejection path), and
//!   * asserts that the Rust implementation produces the identical bytes,
//!     exit status and signal.

mod common;

use common::*;

/// Asserts C's output is exactly `expect`, and that Rust matches C bit for bit
/// (stdout, exit code, signal, empty stderr).
fn assert_both(input: &[u8], expect: &[u8], label: &str) {
    let c = run_with_stdin(&c_exe(), &[], input);
    assert_eq!(
        c.stdout,
        expect,
        "[{label}] the C implementation did not produce the result documented \
         in ERRORS.md for stdin {:?} (got {:?})",
        Preview(input),
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(c.code, Some(0), "[{label}] C exit code");
    let r = run_with_stdin(&rust_exe(), &[], input);
    assert_eq!(c, r, "[{label}] Rust diverges for stdin {:?}", Preview(input));
    // …and through the shared objects as well.
    assert_so_main_same(input, label);
}

// ---------------------------------------------------------------------------
// E1..E9 — `%u` conversions
// ---------------------------------------------------------------------------

/// E1: empty stdin ⇒ input failure on all four conversions.
#[test]
fn err_e01_empty_stdin() {
    assert_both(b"", b"0 0 0 0\n", "E1 empty stdin");
}

/// E2: white space only ⇒ EOF after the white-space skip.
#[test]
fn err_e02_whitespace_only() {
    for input in [
        &b" "[..],
        b"\n",
        b"\t",
        b"\r",
        b"\x0b",
        b"\x0c",
        b" \t\n\x0b\x0c\r",
        &[b' '; 10_000][..],
    ] {
        assert_both(input, b"0 0 0 0\n", "E2 white space only");
    }
}

/// E3: matching failure on a non-numeric byte, and the cascade caused by the
/// `ungetc` of that byte.
#[test]
fn err_e03_matching_failure_cascade() {
    for input in [&b"abc"[..], b"x", b"Z 1 2 3", b"/", b":", b"@1 2 3 4"] {
        assert_both(input, b"0 0 0 0\n", "E3 matching failure cascade");
    }
}

/// E4: a lone sign followed by EOF.
#[test]
fn err_e04_lone_sign_eof() {
    assert_both(b"-", b"0 0 0 0\n", "E4 lone '-'");
    assert_both(b"+", b"0 0 0 0\n", "E4 lone '+'");
    assert_both(b"   -", b"0 0 0 0\n", "E4 ws + '-'");
    // three good tokens then a dangling sign
    assert_both(b"1 2 3 -", b"1 2 1 0\n", "E4 trailing '-'");
    assert_both(b"1 2 3 +", b"1 2 1 0\n", "E4 trailing '+'");
}

/// E5: sign followed by a non-digit — the non-digit is pushed back, so the
/// *next* conversion resumes there.
#[test]
fn err_e05_sign_then_nondigit() {
    assert_both(b"-a 1 2 3", b"0 0 0 0\n", "E5 '-a'");
    assert_both(b"+ 1 2 3", b"0 1 1 3\n", "E5 '+ '");
    assert_both(b"- 1 2 3", b"0 1 1 3\n", "E5 '- '");
    // "1 - 2 3": x=1; the `-` is consumed by conversion #2 which then fails on
    // the space, so y keeps 0; `2` is read into b (⇒ !!2 == 1) and `3` into z.
    assert_both(b"1 - 2 3", b"1 0 1 3\n", "E5 sign in the middle");
}

/// E6: `%u` magnitude beyond `ULONG_MAX` ⇒ `ERANGE`, `ULONG_MAX`.
#[test]
fn err_e06_u_overflow_ulong() {
    // (u32)ULONG_MAX = 0xFFFFFFFF ⇒ x & 3 == 3
    assert_both(b"99999999999999999999999 1 1 1", b"3 1 1 1\n", "E6 23 nines");
    assert_both(b"18446744073709551616 1 1 1", b"3 1 1 1\n", "E6 2^64");
    assert_both(b"18446744073709551617 1 1 1", b"3 1 1 1\n", "E6 2^64+1");
    // y as well: 0xFFFFFFFF & 7 == 7
    assert_both(b"1 18446744073709551616 1 1", b"1 7 1 1\n", "E6 on y");
}

/// E7: negative `%u` value ⇒ `strtoul` negates modulo 2^64.
#[test]
fn err_e07_u_negative() {
    assert_both(b"-1 -1 1 1", b"3 7 1 1\n", "E7 -1");
    assert_both(b"-4294967295 -4294967296 1 1", b"1 0 1 1\n", "E7 -2^32(+1)");
    assert_both(b"-2 -3 1 1", b"2 5 1 1\n", "E7 -2/-3");
    assert_both(b"-0 -0 0 0", b"0 0 0 0\n", "E7 -0");
}

/// E8: `UINT_MAX < v <= ULONG_MAX` ⇒ silent narrowing.
#[test]
fn err_e08_u_narrowing() {
    assert_both(b"4294967296 4294967297 1 1", b"0 1 1 1\n", "E8 2^32");
    assert_both(b"4294967299 4294967304 1 1", b"3 0 1 1\n", "E8 2^32+3 / +8");
    assert_both(b"18446744073709551615 1 1 1", b"3 1 1 1\n", "E8 ULONG_MAX");
}

/// E9: negative **and** overflowing ⇒ `ULONG_MAX` regardless of the sign.
#[test]
fn err_e09_u_negative_overflow() {
    assert_both(b"-99999999999999999999999 1 1 1", b"3 1 1 1\n", "E9 -23 nines");
    assert_both(b"-18446744073709551616 1 1 1", b"3 1 1 1\n", "E9 -2^64");
}

// ---------------------------------------------------------------------------
// E10..E15 — `%d` conversions
// ---------------------------------------------------------------------------

/// E10: fewer than four tokens ⇒ the remaining variables keep their value.
#[test]
fn err_e10_too_few_tokens() {
    assert_both(b"7", b"3 0 0 0\n", "E10 one token");
    assert_both(b"7 8", b"3 0 0 0\n", "E10 two tokens");
    assert_both(b"7 8 9", b"3 0 1 0\n", "E10 three tokens");
    assert_both(b"7 8 0", b"3 0 0 0\n", "E10 three tokens, b=0");
    assert_both(b"7 8 9 ", b"3 0 1 0\n", "E10 three tokens + ws");
}

/// E11: `%d` matching failure.
#[test]
fn err_e11_d_matching_failure() {
    assert_both(b"1 2 x 4", b"1 2 0 0\n", "E11 non-numeric b");
    assert_both(b"1 2 3 x", b"1 2 1 0\n", "E11 non-numeric z");
    assert_both(b"1 2 3 4x", b"1 2 1 4\n", "E11 trailing junk after z");
}

/// E12: `%d` above `LONG_MAX` ⇒ saturates to `LONG_MAX`, narrowed to `-1`.
#[test]
fn err_e12_d_overflow_long_max() {
    assert_both(b"1 1 1 9223372036854775808", b"1 1 1 -1\n", "E12 2^63");
    assert_both(b"1 1 1 99999999999999999999999", b"1 1 1 -1\n", "E12 23 nines");
    // the same path for the `b` conversion: LONG_MAX ⇒ !!b == 1
    assert_both(b"1 1 9223372036854775808 5", b"1 1 1 5\n", "E12 on b");
}

/// E13: `%d` below `LONG_MIN` ⇒ saturates to `LONG_MIN`, narrowed to `0`.
#[test]
fn err_e13_d_overflow_long_min() {
    assert_both(b"1 1 1 -9223372036854775809", b"1 1 1 0\n", "E13 -(2^63+1)");
    assert_both(b"1 1 1 -99999999999999999999999", b"1 1 1 0\n", "E13 -23 nines");
    // LONG_MIN narrowed to int is 0 ⇒ !!b == 0
    assert_both(b"1 1 -99999999999999999999999 5", b"1 1 0 5\n", "E13 on b");
}

/// E14: `INT_MAX < v <= LONG_MAX` ⇒ silent narrowing.
#[test]
fn err_e14_d_narrowing() {
    assert_both(b"1 1 1 2147483648", b"1 1 1 -2147483648\n", "E14 2^31");
    assert_both(b"1 1 1 4294967296", b"1 1 1 0\n", "E14 2^32");
    assert_both(b"1 1 1 -2147483649", b"1 1 1 2147483647\n", "E14 -(2^31+1)");
    assert_both(b"1 1 4294967296 7", b"1 1 0 7\n", "E14 on b ⇒ !!0");
}

/// E15: exactly `LONG_MIN` / `LONG_MAX` — the boundary that is *not* an
/// overflow.
#[test]
fn err_e15_d_long_boundaries() {
    assert_both(b"1 1 1 -9223372036854775808", b"1 1 1 0\n", "E15 LONG_MIN");
    assert_both(b"1 1 1 9223372036854775807", b"1 1 1 -1\n", "E15 LONG_MAX");
    assert_both(b"1 1 1 -9223372036854775807", b"1 1 1 1\n", "E15 LONG_MIN+1");
    assert_both(b"1 1 1 9223372036854775806", b"1 1 1 -2\n", "E15 LONG_MAX-1");
}

// ---------------------------------------------------------------------------
// E16..E23 — lexical / stream level rejections
// ---------------------------------------------------------------------------

/// E16: the conversion base is 10, so an `0x` prefix is not accepted.
#[test]
fn err_e16_hex_prefix_rejected() {
    assert_both(b"0x1f 1 1 1", b"0 0 0 0\n", "E16 0x1f");
    assert_both(b"0X1F 1 1 1", b"0 0 0 0\n", "E16 0X1F");
    assert_both(b"1 0x10 1 1", b"1 0 0 0\n", "E16 0x on y");
    assert_both(b"0b11 1 1 1", b"0 0 0 0\n", "E16 0b11");
    // a leading zero on its own is fine
    assert_both(b"017 1 1 1", b"1 1 1 1\n", "E16 017 is decimal 17");
}

/// E17: an embedded NUL byte is just another non-digit.
#[test]
fn err_e17_nul_byte() {
    assert_both(b"1\x002 3 4", b"1 0 0 0\n", "E17 NUL after digit");
    assert_both(b"\x00 1 2 3", b"0 0 0 0\n", "E17 leading NUL");
    assert_both(b"1 2 3 4\x00", b"1 2 1 4\n", "E17 trailing NUL");
}

/// E18: bytes >= 0x80 are neither digits nor space in the "C" locale.
#[test]
fn err_e18_high_bytes() {
    assert_both(b"\x80\x81 1 2 3", b"0 0 0 0\n", "E18 high bytes");
    assert_both(b"1 \xff 2 3", b"1 0 0 0\n", "E18 0xff");
    assert_both("é 1 2 3".as_bytes(), b"0 0 0 0\n", "E18 utf-8");
    assert_both(b"\xa0 1 2 3", b"0 0 0 0\n", "E18 nbsp byte");
}

/// E19: float / exponent syntax is not an integer.
#[test]
fn err_e19_float_syntax() {
    assert_both(b"1.5 2.5 3.5 4.5", b"1 0 0 0\n", "E19 1.5");
    assert_both(b".5 1 1 1", b"0 0 0 0\n", "E19 .5");
    assert_both(b"1e3 1 1 1", b"1 0 0 0\n", "E19 1e3");
    assert_both(b"1 2 3 4.5", b"1 2 1 4\n", "E19 float in z");
}

/// E20: a doubled sign — the second one is pushed back and consumed by the
/// next conversion.
#[test]
fn err_e20_double_sign() {
    assert_both(b"--1 2 3 4", b"0 7 1 3\n", "E20 --1");
    assert_both(b"+-1 2 3 4", b"0 7 1 3\n", "E20 +-1");
    assert_both(b"-+1 2 3 4", b"0 1 1 3\n", "E20 -+1");
    assert_both(b"++1 2 3 4", b"0 1 1 3\n", "E20 ++1");
}

/// E21: stdin that cannot be read at all.
#[test]
fn err_e21_unreadable_stdin() {
    use std::process::{Command, Stdio};
    // /dev/null (immediate EOF)
    for prog in [c_exe(), rust_exe()] {
        let out = Command::new(&prog)
            .stdin(Stdio::from(std::fs::File::open("/dev/null").unwrap()))
            .output()
            .unwrap();
        assert_eq!(out.stdout, b"0 0 0 0\n", "E21 /dev/null: {}", prog.display());
        assert_eq!(out.status.code(), Some(0));
    }
    // a directory: read(2) fails with EISDIR
    let dir = crate_root();
    for prog in [c_exe(), rust_exe()] {
        let out = Command::new(&prog)
            .stdin(Stdio::from(std::fs::File::open(&dir).unwrap()))
            .output()
            .unwrap();
        assert_eq!(out.stdout, b"0 0 0 0\n", "E21 directory: {}", prog.display());
        assert_eq!(out.status.code(), Some(0));
    }
    // stdin closed entirely (fd 0 is not open ⇒ EBADF)
    for prog in [c_exe(), rust_exe()] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("exec 0<&- ; exec {}", prog.display()))
            .output()
            .unwrap();
        assert_eq!(out.stdout, b"0 0 0 0\n", "E21 closed fd: {}", prog.display());
        assert_eq!(out.status.code(), Some(0));
    }
}

/// E22: a digit run of thousands of characters.
#[test]
fn err_e22_very_long_digit_run() {
    for n in [19usize, 20, 21, 100, 1000, 5000, 100_000] {
        let mut v = vec![b'9'; n];
        v.extend_from_slice(b" 1 1 1");
        // 10^n - 1 > ULONG_MAX for n >= 20 ⇒ 0xFFFFFFFF ⇒ 3
        let expect: &[u8] = if n >= 20 { b"3 1 1 1\n" } else { b"3 1 1 1\n" };
        assert_both(&v, expect, "E22 long digit run");

        // a long run of zeros followed by a digit is *not* an overflow
        let mut v = vec![b'0'; n];
        v.extend_from_slice(b"5 1 1 1");
        assert_both(&v, b"1 1 1 1\n", "E22 long zero run");

        // negative overflow of `%d`
        let mut v = b"1 1 1 -".to_vec();
        v.extend(std::iter::repeat(b'9').take(n));
        let expect: &[u8] = if n >= 19 { b"1 1 1 0\n" } else { b"1 1 1 -999999999\n" };
        if n <= 9 || n >= 19 {
            assert_both(&v, expect, "E22 long negative run");
        } else {
            // 10..18 digits: compare against C without a hard-coded value
            assert_exe_same(&v, "E22 long negative run (diff only)");
        }
    }
}

/// E23: a token straddling the reader's buffer boundary.
#[test]
fn err_e23_buffer_boundary() {
    for off in [4095usize, 4096, 4097, 8191, 8192, 8193] {
        // the pushback byte is the first byte after a full buffer
        let mut v = vec![b'1'; off];
        v.extend_from_slice(b"z 1 2 3");
        assert_exe_same(&v, "E23 pushback at boundary");

        // white space run ending exactly at the boundary, then a bad token
        let mut v = vec![b' '; off];
        v.extend_from_slice(b"q");
        assert_both(&v, b"0 0 0 0\n", "E23 ws to boundary then junk");

        // EOF exactly at the boundary
        let v = vec![b' '; off];
        assert_both(&v, b"0 0 0 0\n", "E23 EOF at boundary");
    }
}

// ---------------------------------------------------------------------------
// F5, F9, F10 — FFI level rejections that need their own process
// ---------------------------------------------------------------------------

/// F5: `print_foo(NULL)` — the C code has no null check and faults.
#[test]
fn err_f05_print_foo_null() {
    let cso = c_shared_lib();
    let rso = rust_shared_lib();
    let runner = so_runner();
    let c = run_with_stdin(&runner, &[cso.to_str().unwrap(), "print_foo_null"], b"");
    let r = run_with_stdin(&runner, &[rso.to_str().unwrap(), "print_foo_null"], b"");
    assert_eq!(
        c.signal,
        Some(11),
        "F5: the C print_foo(NULL) was expected to raise SIGSEGV, got {c:?}"
    );
    assert!(c.stdout.is_empty(), "F5: C printed something: {:?}", c.stdout);
    assert_eq!(
        (c.code, c.signal, &c.stdout),
        (r.code, r.signal, &r.stdout),
        "F5: Rust print_foo(NULL) differs from C"
    );
}

/// F9: the `.so`'s `main` with an empty / EOF stdin.
#[test]
fn err_f09_so_main_eof() {
    let cso = c_shared_lib();
    let rso = rust_shared_lib();
    let runner = so_runner();
    for input in [&b""[..], b" ", b"junk"] {
        let c = run_with_stdin(&runner, &[cso.to_str().unwrap(), "main"], input);
        let r = run_with_stdin(&runner, &[rso.to_str().unwrap(), "main"], input);
        assert_eq!(c.stdout, b"0 0 0 0\n", "F9: C .so main output");
        assert_eq!(c.code, Some(0), "F9: C .so main returned non-zero");
        assert_eq!(
            (c.code, c.signal, &c.stdout),
            (r.code, r.signal, &r.stdout),
            "F9: Rust .so main differs for {:?}",
            Preview(input)
        );
    }
}

/// F10: there is no failure exit path — `main` always returns 0.
#[test]
fn err_f10_exit_code_always_zero() {
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..300 {
        let n = rng.below(24) as usize;
        let v: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        let c = run_with_stdin(&c_exe(), &[], &v);
        let r = run_with_stdin(&rust_exe(), &[], &v);
        assert_eq!(c.code, Some(0), "F10: C exit code for {:?}", Preview(&v));
        assert_eq!(c.signal, None, "F10: C signal for {:?}", Preview(&v));
        assert_eq!(c, r, "F10: divergence for {:?}", Preview(&v));
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks (not tied to a single ERRORS.md row)
// ---------------------------------------------------------------------------

/// Oversized input: a megabyte of digits, and a megabyte of white space.
#[test]
fn err_gen_oversized_input() {
    let mut v = vec![b'1'; 1 << 20];
    v.extend_from_slice(b" 2 3 4");
    assert_exe_same(&v, "oversized digits");

    let mut v = vec![b' '; 1 << 20];
    v.extend_from_slice(b"9 8 7 6");
    assert_both(&v, b"1 0 1 6\n", "oversized white space");

    let v = vec![b'\n'; 1 << 20];
    assert_both(&v, b"0 0 0 0\n", "oversized newlines");
}

/// `driver` reached out of process, including the out-of-range `_Bool` byte and
/// `x`/`y` values one step past the bit-field ranges.
#[test]
fn err_gen_driver_out_of_process() {
    for (x, y, b, z) in [
        ("0", "0", "0", "0"),
        ("4", "8", "2", "-1"),
        ("3", "7", "1", "-2147483648"),
        ("4294967295", "4294967295", "255", "2147483647"),
        ("4294967295", "4294967295", "254", "2147483647"),
    ] {
        assert_runner_same(&["driver", x, y, b, z], b"", "driver out of process");
    }
}

/// `print_foo` reached out of process, including a **misaligned** `foo_t`
/// pointer (byte 0 and the `int` at +4 are read with unaligned loads on
/// x86-64, so the C code copes; the Rust export must cope identically).
#[test]
fn err_gen_print_foo_out_of_process() {
    for (bits, p0, p1, p2, z) in [
        ("0", "0", "0", "0", "0"),
        ("255", "255", "255", "255", "-1"),
        ("42", "170", "85", "240", "2147483647"),
        ("32", "0", "0", "0", "-2147483648"),
    ] {
        assert_runner_same(
            &["print_foo", bits, p0, p1, p2, z],
            b"",
            "print_foo out of process",
        );
    }
}
