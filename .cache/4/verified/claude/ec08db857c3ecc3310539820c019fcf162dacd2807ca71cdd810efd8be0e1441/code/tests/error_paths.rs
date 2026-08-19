//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact condition the C rejects on, runs *both*
//! executables, and asserts they agree on the concrete result: the same stdout
//! bytes (`"An error occurred\n"` for a rejection), the same stderr, the same
//! exit code and the same terminating signal. Where the C *accepts* a value one
//! step inside a bound, the expected transcript is generated from an independent
//! model of the C program so the assertion is not merely "both did the same".

mod common;

use common::*;
use std::path::PathBuf;

struct Env {
    c: PathBuf,
    r: PathBuf,
    dir: PathBuf,
    row: &'static str,
}

fn env(row: &'static str) -> Env {
    Env {
        c: c_exe().to_path_buf(),
        r: rust_exe(),
        dir: scratch(&format!("err/{row}")),
        row,
    }
}

const ERR_MSG: &[u8] = b"An error occurred\n";

/// Independent re-derivation of the C program's success transcript.
///
/// ```text
/// the_house = {2, 5, 2.5}
/// run(x): print; floors++; print; bathrooms += 1; print; bedrooms += x; print
/// main:   run(x); run(x);
/// ```
fn expected_success(x: i32) -> Vec<u8> {
    let b1 = 5i32.wrapping_add(x);
    let b2 = b1.wrapping_add(x);
    let line = |f: i32, b: i32, ba: &str| format!("The house has {f} floors, {b} bedrooms, and {ba} bathrooms\n");
    let mut s = String::new();
    s += &line(2, 5, "2.5");
    s += &line(3, 5, "2.5");
    s += &line(3, 5, "3.5");
    s += &line(3, b1, "3.5");
    s += &line(3, b1, "3.5");
    s += &line(4, b1, "3.5");
    s += &line(4, b1, "4.5");
    s += &line(4, b2, "4.5");
    s.into_bytes()
}

impl Env {
    /// Both must reject with exactly `An error occurred\n` and exit status 0.
    fn expect_rejected(&self, input: &[u8]) {
        let c = run_stdin_file(&self.c, &self.dir, input);
        let r = run_stdin_file(&self.r, &self.dir, input);
        assert_same(self.row, &describe(input), &c, &r);
        assert_eq!(
            c.stdout,
            ERR_MSG,
            "[{}] C did not reject {} (stdout: {})",
            self.row,
            describe(input),
            describe(&c.stdout)
        );
        assert_eq!(c.code, Some(0), "[{}] C exit code", self.row);
        assert_eq!(c.signal, None, "[{}] C signal", self.row);
        assert!(c.stderr.is_empty(), "[{}] C stderr", self.row);
    }

    /// Both must accept and print the transcript for `x`.
    fn expect_accepted(&self, input: &[u8], x: i32) {
        let c = run_stdin_file(&self.c, &self.dir, input);
        let r = run_stdin_file(&self.r, &self.dir, input);
        assert_same(self.row, &describe(input), &c, &r);
        assert_eq!(
            c.stdout,
            expected_success(x),
            "[{}] unexpected C transcript for {} (x={})",
            self.row,
            describe(input),
            x
        );
        assert_eq!(c.code, Some(0), "[{}] C exit code", self.row);
    }
}

// --- E1 -------------------------------------------------------------------

#[test]
fn err_e1_immediate_eof() {
    let e = env("E1");
    e.expect_rejected(b"");

    // /dev/null is the same condition through a different file type.
    use std::process::{Command, Stdio};
    let mk = || Stdio::from(std::fs::File::open("/dev/null").unwrap());
    let co = Command::new(&e.c).stdin(mk()).output().unwrap();
    let ro = Command::new(&e.r).stdin(mk()).output().unwrap();
    assert_eq!(co.stdout, ERR_MSG);
    assert_eq!(co.stdout, ro.stdout);
    assert_eq!(co.status.code(), ro.status.code());
}

// --- E2 -------------------------------------------------------------------

#[test]
fn err_e2_unreadable_stdin() {
    let e = env("E2");
    // fd 0 closed before exec: read() fails with EBADF.
    let c = run_with_closed_fds(&e.c, &e.dir, b"42\n", &[0]);
    let r = run_with_closed_fds(&e.r, &e.dir, b"42\n", &[0]);
    assert_same("E2", "fd0 closed", &c, &r);
    assert_eq!(c.stdout, ERR_MSG);
    assert_eq!(c.code, Some(0));

    // stdin on a directory: read() fails with EISDIR.
    let c = run_stdin_directory(&e.c);
    let r = run_stdin_directory(&e.r);
    assert_same("E2", "stdin=directory", &c, &r);
    assert_eq!(c.stdout, ERR_MSG);
    assert_eq!(c.code, Some(0));
}

// --- E3 / E4 / E5 / E6 / E7 ----------------------------------------------

#[test]
fn err_e3_empty_string() {
    let e = env("E3");
    e.expect_rejected(b"");
    e.expect_rejected(b"\n");
}

#[test]
fn err_e4_no_digits() {
    let e = env("E4");
    for s in [
        "abc", "abc\n", "x1", "x1\n", ".5", ".5\n", "/9", ":9", "e5", "E5", "inf", "nan",
        "0b101".trim_start_matches('0'), "#42", "!", "~", "[42]", "'42'", "\"42\"",
    ] {
        e.expect_rejected(s.as_bytes());
    }
}

#[test]
fn err_e5_whitespace_only() {
    let e = env("E5");
    for s in [
        " ", "  ", "\t", "\x0b", "\x0c", "\r", "\n", " \n", "\t\x0b\x0c\r ", "\t\x0b\x0c\r \n",
        &" ".repeat(99), &" ".repeat(150),
    ] {
        e.expect_rejected(s.as_bytes());
    }
}

#[test]
fn err_e6_sign_without_digits() {
    let e = env("E6");
    for s in [
        "-", "+", "-\n", "+\n", "- 5", "+ 5", "--5", "++5", "+-5", "-+5", "-x", "+x", "-.5",
        "+.5", "-\t5", "-abc", "   -   ", "   +", "-\n5\n", "+\n5\n",
    ] {
        e.expect_rejected(s.as_bytes());
    }
}

#[test]
fn err_e7_leading_nul() {
    let e = env("E7");
    let cases: &[&[u8]] = &[b"\x0042\n", b"\x00", b"\x00\n", b"\x00-1\n", b"\x00\x00\x0042\n"];
    for c in cases {
        e.expect_rejected(c);
    }
}

// --- E8 / E9 --------------------------------------------------------------

#[test]
fn err_e8_erange_positive() {
    let e = env("E8");
    for s in [
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
        "99999999999999999999",
        "+9223372036854775808",
        "00009223372036854775808",
    ] {
        e.expect_rejected(format!("{s}\n").as_bytes());
        e.expect_rejected(s.as_bytes());
    }
    // Every digit count from 20 to 98 is beyond LONG_MAX for a leading 9.
    for n in 20..=98usize {
        e.expect_rejected(format!("{}\n", "9".repeat(n)).as_bytes());
    }
}

#[test]
fn err_e9_erange_negative() {
    let e = env("E9");
    for s in [
        "-9223372036854775809",
        "-9223372036854775810",
        "-18446744073709551615",
        "-99999999999999999999",
        "-00009223372036854775809",
    ] {
        e.expect_rejected(format!("{s}\n").as_bytes());
        e.expect_rejected(s.as_bytes());
    }
    for n in 20..=98usize {
        e.expect_rejected(format!("-{}\n", "9".repeat(n)).as_bytes());
    }
}

// --- E10 / E11 ------------------------------------------------------------

#[test]
fn err_e10_above_int_max() {
    let e = env("E10");
    for v in [
        2147483648i64,
        2147483649,
        4294967295,
        4294967296,
        1 << 40,
        i64::MAX - 1,
        i64::MAX,
    ] {
        e.expect_rejected(format!("{v}\n").as_bytes());
        e.expect_rejected(format!("+{v}\n").as_bytes());
        e.expect_rejected(format!("  {v}  \n").as_bytes());
        e.expect_rejected(format!("{v}xyz\n").as_bytes());
    }
}

#[test]
fn err_e11_below_int_min() {
    let e = env("E11");
    for v in [
        -2147483649i64,
        -2147483650,
        -4294967296,
        -(1i64 << 40),
        i64::MIN + 1,
        i64::MIN,
    ] {
        e.expect_rejected(format!("{v}\n").as_bytes());
        e.expect_rejected(format!("  {v}  \n").as_bytes());
        e.expect_rejected(format!("{v}xyz\n").as_bytes());
    }
}

// --- E12 ------------------------------------------------------------------

#[test]
fn err_e12_int_bounds_accepted() {
    let e = env("E12");
    for v in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, 0, 1, -1] {
        e.expect_accepted(format!("{v}\n").as_bytes(), v);
        e.expect_accepted(format!("{v}").as_bytes(), v);
        e.expect_accepted(format!("  {v}\n").as_bytes(), v);
    }
    e.expect_accepted(b"+2147483647\n", i32::MAX);
    e.expect_accepted(b"0000002147483647\n", i32::MAX);
    e.expect_accepted(b"-0000002147483648\n", i32::MIN);
}

// --- E13 / E14 ------------------------------------------------------------

#[test]
fn err_e13_truncated_at_99() {
    let e = env("E13");
    // 98 spaces + "42": fgets keeps 98 spaces and the '4' only -> value 4.
    let input = format!("{}42\n", " ".repeat(98));
    e.expect_accepted(input.as_bytes(), 4);
    // 97 spaces + "42": both digits survive -> value 42.
    let input = format!("{}42\n", " ".repeat(97));
    e.expect_accepted(input.as_bytes(), 42);
    // 99 spaces + "42": nothing but whitespace survives -> rejected.
    let input = format!("{}42\n", " ".repeat(99));
    e.expect_rejected(input.as_bytes());
    // 90 spaces + 12 digits: only the first 9 digits survive.
    let input = format!("{}123456789012\n", " ".repeat(90));
    e.expect_accepted(input.as_bytes(), 123456789);
}

#[test]
fn err_e14_truncation_causes_erange() {
    let e = env("E14");
    for n in [99usize, 100, 120, 200, 500] {
        e.expect_rejected(format!("{}\n", "9".repeat(n)).as_bytes());
    }
    // 99 leading '1's is 99 digits -> ERANGE even though the tail is dropped.
    e.expect_rejected(format!("{}{}\n", "1".repeat(99), "2".repeat(50)).as_bytes());
}

// --- E15 ------------------------------------------------------------------

#[test]
fn err_e15_embedded_nul_truncates() {
    let e = env("E15");
    e.expect_accepted(b"12\x0034\n", 12);
    e.expect_accepted(b"4\x002\n", 4);
    e.expect_accepted(b"42\x00\n", 42);
    e.expect_accepted(b"-4\x002\n", -4);
    e.expect_accepted(b"2147483647\x008\n", i32::MAX);
    e.expect_accepted(b"214748364\x007\n", 214748364);
    // Without the NUL the same bytes are out of range, proving the truncation
    // really is what makes it succeed.
    e.expect_rejected(b"21474836478\n");
}

// --- E16 ------------------------------------------------------------------

#[test]
fn err_e16_trailing_garbage_ok() {
    let e = env("E16");
    e.expect_accepted(b"42abc\n", 42);
    e.expect_accepted(b"0x10\n", 0);
    e.expect_accepted(b"0X10\n", 0);
    e.expect_accepted(b"0b101\n", 0);
    e.expect_accepted(b"1e5\n", 1);
    e.expect_accepted(b"5.9\n", 5);
    e.expect_accepted(b"5 6\n", 5);
    e.expect_accepted(b"5-6\n", 5);
    e.expect_accepted(b"-3xyz\n", -3);
    e.expect_accepted(b"  +0042xyz\n", 42);
    e.expect_accepted(b"9\n8\n", 9);
}

// --- E17 / E18 ------------------------------------------------------------

#[test]
fn err_e17_sigpipe() {
    let e = env("E17");
    for s in ["5\n", "-7\n", "2147483647\n", "abc\n", ""] {
        let c = run_stdout_closed_pipe(&e.c, &e.dir, s.as_bytes());
        let r = run_stdout_closed_pipe(&e.r, &e.dir, s.as_bytes());
        assert_same("E17", s, &c, &r);
        // Every case writes something (either the 8 house lines or the error
        // message), so both implementations must die from SIGPIPE (signal 13).
        assert_eq!(
            c.signal,
            Some(13),
            "[E17] expected C to die from SIGPIPE for {s:?}, got code={:?} signal={:?}",
            c.code,
            c.signal
        );
        assert_eq!(c.code, None, "[E17] C must not have a normal exit code");
    }
}

#[test]
fn err_e18_closed_stdout() {
    let e = env("E18");
    for s in ["5\n", "abc\n", "", "2147483648\n"] {
        let c = run_with_closed_fds(&e.c, &e.dir, s.as_bytes(), &[1]);
        let r = run_with_closed_fds(&e.r, &e.dir, s.as_bytes(), &[1]);
        assert_same("E18", s, &c, &r);
        assert_eq!(c.code, Some(0), "[E18] C must exit 0 for {s:?}");
        assert_eq!(c.signal, None, "[E18] C must not be signalled for {s:?}");
    }
}

// --- E19 ------------------------------------------------------------------

#[test]
fn err_e19_bedroom_overflow_wraps() {
    let e = env("E19");
    for v in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 4,
        i32::MAX - 5,
        i32::MAX - 6,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 5,
        1073741821,
        1073741822,
        1073741823,
        1073741824,
        -1073741824,
        -1073741825,
        i32::MAX / 2,
        i32::MIN / 2,
    ] {
        // `expected_success` uses wrapping_add, i.e. it asserts the *wrapped*
        // values are what the C actually prints.
        e.expect_accepted(format!("{v}\n").as_bytes(), v);
    }
}

// --- E21 ------------------------------------------------------------------

#[test]
fn err_e21_argv_ignored() {
    let e = env("E21");
    let baseline = run_stdin_file(&e.c, &e.dir, b"5\n");
    for args in [
        vec!["7"],
        vec!["--help"],
        vec!["-1", "-2", "-3"],
        vec![""],
        vec!["999999999999999999999"],
    ] {
        let c = run_with_args(&e.c, &e.dir, b"5\n", &args);
        let r = run_with_args(&e.r, &e.dir, b"5\n", &args);
        assert_same("E21", &format!("{args:?}"), &c, &r);
        assert_eq!(c.stdout, baseline.stdout, "[E21] argv changed the C output");
    }
}
