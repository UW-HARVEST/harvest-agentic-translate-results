// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Phase C - error/rejection-path differential tests.
//
// One test per row of ERRORS.md (rows 1-18; rows 19-22 are the FFI rows in
// `differential_ffi.rs`).
//
// The C program discards every `scanf` return value, so a rejection is *silent*:
// it leaves the destination variable at its initialiser `0` and pushes back at
// most one character. Each test therefore checks two things:
//
//   1. C and Rust agree exactly (stdout bytes, stderr, exit code, signal); and
//   2. the shared result is the specific value the C semantics predict, so that
//      "both sides are broken the same way" cannot pass as success.

mod common;

use common::{assert_same, assert_same_cfg, c_exe, run, run_cfg, rust_exe, StdinKind, StdoutKind};

/// Assert both builds agree *and* that the agreed-on stdout is `expected`.
fn assert_same_and(row: &str, input: &[u8], expected: &str) {
    assert_same(row, input);
    let c = run(c_exe(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected,
        "[{row}] C produced an unexpected result for {:?}; the hand-derived \
         expectation in ERRORS.md is wrong",
        String::from_utf8_lossy(input)
    );
    assert_eq!(c.code, Some(0), "[{row}] C must exit 0");
    assert!(c.stderr.is_empty(), "[{row}] C must be silent on stderr");
}

// ---------------------------------------------------------------------------
// Rows 1 & 2 - input failure (nothing to convert at all)
// ---------------------------------------------------------------------------

/// Row 1: empty stdin. Both conversions fail, `x == y == 0`, `0 | ~0 == -1`.
#[test]
fn err01_empty_stdin() {
    assert_same_and("err01", b"", "-1\n");
}

/// Row 2: whitespace only. The whitespace is consumed, then EOF.
#[test]
fn err02_whitespace_only() {
    for input in [
        &b" "[..],
        &b"\t"[..],
        &b"\n"[..],
        &b"\x0b"[..],
        &b"\x0c"[..],
        &b"\r"[..],
        &b"   \t\r\n\x0b\x0c  "[..],
    ] {
        assert_same_and("err02", input, "-1\n");
    }
}

// ---------------------------------------------------------------------------
// Rows 3 & 4 - matching failure on the very first byte
// ---------------------------------------------------------------------------

/// Row 3: a letter. The byte is pushed back, so directive 2 fails on it too.
#[test]
fn err03_leading_alpha() {
    for input in [&b"abc"[..], &b"a"[..], &b"z 5"[..], &b"  qqq  "[..], &b"e5"[..]] {
        assert_same_and("err03", input, "-1\n");
    }
}

/// Row 4: punctuation.
#[test]
fn err04_leading_punct() {
    for c in b".,*/#()[]{}%$@!?~^&|<>=\"'\\`;:_" {
        let input = [*c];
        assert_same_and("err04", &input, "-1\n");
        // Also with a valid number behind it: the number is never reached,
        // because the offending byte is pushed back and re-read by directive 2.
        let input = [*c, b'5', b' ', b'7'];
        assert_same_and("err04/blocked", &input, "-1\n");
    }
}

// ---------------------------------------------------------------------------
// Rows 5-7 - the sign is consumed even when the directive fails
// ---------------------------------------------------------------------------

/// Row 5: sign then a non-digit. The sign stays consumed; only the non-digit is
/// pushed back, so directive 2 sees the text *after* the sign.
#[test]
fn err05_sign_then_nondigit() {
    // "- 5": directive 1 eats '-', fails on ' '; directive 2 then reads " 5"
    // as +5, so the result is 0 | ~5 == -6.
    assert_same_and("err05", b"- 5", "-6\n");
    // "+ 5" behaves the same way.
    assert_same_and("err05", b"+ 5", "-6\n");
    // "- -5": directive 2 reads "-5", so 0 | ~(-5) == 0 | 4 == 4.
    assert_same_and("err05", b"- -5", "4\n");
    // Non-digit that is not whitespace: nothing convertible remains.
    for input in [&b"-a"[..], &b"+."[..], &b"-x"[..], &b"+#"[..]] {
        assert_same_and("err05", input, "-1\n");
    }
}

/// Row 6: sign then immediate EOF. Nothing is pushed back.
#[test]
fn err06_sign_then_eof() {
    assert_same_and("err06", b"-", "-1\n");
    assert_same_and("err06", b"+", "-1\n");
    assert_same_and("err06", b"   -", "-1\n");
    // A sign for directive 1 and a sign for directive 2.
    assert_same_and("err06", b"- -", "-1\n");
}

/// Row 7: two signs in a row. Directive 1 eats one and fails, pushing back the
/// second; directive 2 then converts the signed remainder.
#[test]
fn err07_double_sign() {
    // "--5": directive 2 reads "-5" -> 0 | ~(-5) == 4.
    assert_same_and("err07", b"--5", "4\n");
    // "+-5": same, the '+' is eaten and '-' pushed back.
    assert_same_and("err07", b"+-5", "4\n");
    // "-+5": directive 2 reads "+5" -> 0 | ~5 == -6.
    assert_same_and("err07", b"-+5", "-6\n");
    // "++5": likewise.
    assert_same_and("err07", b"++5", "-6\n");
    // Three signs: directive 1 eats one, directive 2 eats one and fails.
    assert_same_and("err07", b"---5", "-1\n");
}

// ---------------------------------------------------------------------------
// Rows 8 & 9 - bytes that are neither digits nor whitespace in the "C" locale
// ---------------------------------------------------------------------------

/// Row 8: an embedded NUL is an ordinary non-matching byte.
#[test]
fn err08_nul_byte() {
    assert_same_and("err08", b"\x00", "-1\n");
    assert_same_and("err08", b"\x00\x00\x00", "-1\n");
    // A NUL blocks the digits behind it.
    assert_same_and("err08", b"\x005 7", "-1\n");
    // A NUL after a valid token terminates it, and blocks directive 2.
    assert_same_and("err08", b"5\x007", "-1\n");
    // NUL after the sign: sign consumed, NUL pushed back and re-read.
    assert_same_and("err08", b"-\x005", "-1\n");
}

/// Row 9: high bytes. None of `0x80..=0xff` is a digit or a space in the `"C"`
/// locale, so every one of them is a matching failure.
#[test]
fn err09_high_byte() {
    for b in 0x80u8..=0xff {
        assert_same_and("err09", &[b], "-1\n");
        assert_same_and("err09", &[b, b'5', b' ', b'7'], "-1\n");
    }
    // Also as a UTF-8 sequence, which is just two high bytes to the scanner.
    assert_same_and("err09/utf8", "é".as_bytes(), "-1\n");
    assert_same_and("err09/nbsp", "\u{a0}5".as_bytes(), "-1\n");
}

// ---------------------------------------------------------------------------
// Row 10 - the second directive fails
// ---------------------------------------------------------------------------

/// Row 10: `x` converts, `y` stays 0. Then `x | ~0 == x | -1 == -1` for every
/// possible `x`, which is why the output is always `-1`.
#[test]
fn err10_second_directive_fails() {
    for input in [
        &b"5"[..],
        &b"5 "[..],
        &b"5\n"[..],
        &b"5 abc"[..],
        &b"5 -"[..],
        &b"5 --3"[..],
        &b"5 ."[..],
        &b"5 \x00"[..],
        &b"-2147483648"[..],
        &b"2147483647 x"[..],
        &b"0"[..],
    ] {
        assert_same_and("err10", input, "-1\n");
    }

    // Property form: whatever the first token is, a failing second directive
    // always yields -1.
    let mut rng = common::Rng::new(common::SEED ^ 10);
    for _ in 0..200 {
        let input = format!("{} zzz", rng.i32v());
        assert_same_and("err10/random", input.as_bytes(), "-1\n");
    }
}

// ---------------------------------------------------------------------------
// Rows 11-14 - out-of-range conversions
// ---------------------------------------------------------------------------

/// Row 11: `ERANGE` overflow. `strtol` clamps to `LONG_MAX`
/// (`0x7fff_ffff_ffff_ffff`); assigning that to `int` truncates to `-1`.
#[test]
fn err11_erange_overflow() {
    // x = -1, y = 0  ->  -1 | ~0 == -1
    assert_same_and("err11", b"9223372036854775808", "-1\n");
    assert_same_and("err11", b"99999999999999999999999999", "-1\n");
    // Pin the truncation down by putting the clamped value in `y`:
    // x = 0, y = -1  ->  0 | ~(-1) == 0 | 0 == 0
    assert_same_and("err11/y", b"0 9223372036854775808", "0\n");
    assert_same_and("err11/y", b"0 99999999999999999999999999", "0\n");
    assert_same_and("err11/y", b"0 18446744073709551616", "0\n");
}

/// Row 12: `ERANGE` underflow. `strtol` clamps to `LONG_MIN`
/// (`0x8000_0000_0000_0000`); truncating that to `int` gives `0`.
#[test]
fn err12_erange_underflow() {
    // x = 0, y = 0  ->  0 | ~0 == -1
    assert_same_and("err12", b"-9223372036854775809", "-1\n");
    // Put the clamped value in `y`: y = 0 -> x | ~0 == -1 as well, so use `x`
    // and a known `y` to observe it: x = 0, y = -1 -> 0 | 0 == 0
    assert_same_and("err12/x", b"-9223372036854775809 -1", "0\n");
    assert_same_and("err12/x", b"-1111111111111111111111111111111111111111 -1", "0\n");
}

/// Row 13: exactly `LONG_MAX` / `LONG_MIN`, where no `ERANGE` occurs but the
/// truncation to `int` still does.
#[test]
fn err13_long_boundaries_exact() {
    // LONG_MAX truncates to -1  ->  -1 | ~0 == -1
    assert_same_and("err13", b"9223372036854775807", "-1\n");
    // ...and as `y`: 0 | ~(-1) == 0
    assert_same_and("err13", b"0 9223372036854775807", "0\n");
    // LONG_MIN truncates to 0  ->  0 | ~0 == -1
    assert_same_and("err13", b"-9223372036854775808", "-1\n");
    // ...and as `y`: 0 | ~0 == -1
    assert_same_and("err13", b"0 -9223372036854775808", "-1\n");
}

/// Row 14: within `long` range but outside `int` range - plain modulo-2^32 wrap.
#[test]
fn err14_long_to_int_truncation() {
    // 2147483648 -> INT_MIN; INT_MIN | ~0 == -1
    assert_same_and("err14", b"2147483648", "-1\n");
    // as y: 0 | ~INT_MIN == 0 | INT_MAX == 2147483647
    assert_same_and("err14", b"0 2147483648", "2147483647\n");
    // -2147483649 -> INT_MAX; as y: 0 | ~INT_MAX == INT_MIN
    assert_same_and("err14", b"0 -2147483649", "-2147483648\n");
    // 4294967296 -> 0; as y: 0 | ~0 == -1
    assert_same_and("err14", b"0 4294967296", "-1\n");
    // 4294967297 -> 1; as y: 0 | ~1 == -2
    assert_same_and("err14", b"0 4294967297", "-2\n");
    // 4294967295 -> -1; as y: 0 | ~(-1) == 0
    assert_same_and("err14", b"0 4294967295", "0\n");
}

/// Row 15: a 10 000-digit run must be clamped, not crash or hang.
#[test]
fn err15_very_long_digit_run() {
    let long = "9".repeat(10_000);
    // clamps to LONG_MAX -> -1 ; as y: 0 | ~(-1) == 0
    assert_same_and("err15", format!("0 {long}").as_bytes(), "0\n");
    // negative clamps to LONG_MIN -> 0 ; as y: 0 | ~0 == -1
    assert_same_and("err15", format!("0 -{long}").as_bytes(), "-1\n");
    // and in the first position
    assert_same_and("err15", format!("{long} -1").as_bytes(), "-1\n");

    // A leading-zero run of the same size is just zero.
    let zeros = "0".repeat(10_000);
    assert_same_and("err15/zeros", format!("0 {zeros}").as_bytes(), "-1\n");
}

// ---------------------------------------------------------------------------
// Row 16 - partial match: the digits seen so far are kept
// ---------------------------------------------------------------------------

#[test]
fn err16_digits_then_nondigit() {
    // "5abc": x = 5, then 'a' fails -> y = 0 -> 5 | ~0 == -1
    assert_same_and("err16", b"5abc", "-1\n");
    // "0 5abc": x = 0, y = 5 -> 0 | ~5 == -6
    assert_same_and("err16", b"0 5abc", "-6\n");
    // "0 5.75": y = 5 (the '.' terminates it)
    assert_same_and("err16", b"0 5.75", "-6\n");
    // "0 1e5": y = 1 -> 0 | ~1 == -2
    assert_same_and("err16", b"0 1e5", "-2\n");
    // "0 0x5": base 10 stops at 'x', so y = 0 -> 0 | ~0 == -1
    assert_same_and("err16", b"0 0x5", "-1\n");
    // "0x5 7": x = 0, then directive 2 fails on the pushed-back 'x' -> y = 0
    assert_same_and("err16", b"0x5 7", "-1\n");
    // "12,34": x = 12, y = 0 -> -1
    assert_same_and("err16", b"12,34", "-1\n");
}

// ---------------------------------------------------------------------------
// Rows 17 & 18 - output write failures
// ---------------------------------------------------------------------------

/// Row 17: stdout is a closed descriptor. `printf`/`puts` fail with `EBADF`,
/// the return values are discarded, and the process still exits 0.
#[test]
fn err17_stdout_closed_ebadf() {
    for input in [&b"5 7"[..], &b""[..], &b"abc"[..]] {
        assert_same_cfg("err17", input, StdinKind::Pipe, StdoutKind::Closed, &[]);

        let c = run_cfg(c_exe(), input, StdinKind::Pipe, StdoutKind::Closed, &[]);
        assert_eq!(c.code, Some(0), "[err17] C must still exit 0: {c:?}");
        assert_eq!(c.signal, None, "[err17] C must not be signalled: {c:?}");
        assert!(c.stdout.is_empty(), "[err17] no output is possible: {c:?}");
    }
}

/// Row 18: stdout is a pipe whose reader is gone. `SIGPIPE` is at its default
/// disposition in a C program, so the process is killed by signal 13 rather than
/// exiting 0.
///
/// This is exactly where the Rust runtime differs by default: it installs
/// `SIG_IGN` for `SIGPIPE` before `main`, which would turn the write into a
/// silently ignored `EPIPE` and a `0` exit status.
#[test]
fn err18_stdout_epipe_sigpipe() {
    for input in [&b"5 7"[..], &b""[..], &b"abc"[..], &b"-2147483648 0"[..]] {
        assert_same_cfg("err18", input, StdinKind::Pipe, StdoutKind::BrokenPipe, &[]);

        let c = run_cfg(c_exe(), input, StdinKind::Pipe, StdoutKind::BrokenPipe, &[]);
        let r = run_cfg(rust_exe(), input, StdinKind::Pipe, StdoutKind::BrokenPipe, &[]);
        assert_eq!(
            c.signal,
            Some(13),
            "[err18] the C build must be killed by SIGPIPE: {c:?}"
        );
        assert_eq!(
            r.signal,
            Some(13),
            "[err18] the Rust build must also be killed by SIGPIPE, not exit quietly: {r:?}"
        );
        assert_eq!(c.code, None);
        assert_eq!(r.code, None);
    }
}
