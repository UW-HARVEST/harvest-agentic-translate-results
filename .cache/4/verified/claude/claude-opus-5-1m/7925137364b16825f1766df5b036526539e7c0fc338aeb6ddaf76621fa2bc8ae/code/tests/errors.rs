// Phase C — error-path differential tests, one test per ERRORS.md row (E1–E25).
//
// Each test constructs the exact invalid input/condition, runs BOTH binaries and
// asserts they reject identically: same stdout bytes (which carry the specific
// error message and the `Result:` code), same stderr, same wait status.

mod common;

use common::*;
use std::path::{Path, PathBuf};

const R1: &str = "Error: x != 1\n";
const R2: &str = "Error: x == 1 but y != 2\n";
const R3: &str = "Error: x == 1 and y == 2, but z != 3\n";
const F: &str = "Operation failed\n";

fn e1_out() -> String {
    format!("{R1}{F}Result: 1\n")
}
fn e2_out() -> String {
    format!("{R2}{F}Result: 2\n")
}
fn e3_out() -> String {
    format!("{R3}{F}Result: 3\n")
}

/// E1 — `if (x != 1)`.
#[test]
fn e1_x_not_one() {
    let mut rng = Rng::new(0xE1);
    for _ in 0..200 {
        let mut x = rng.next_i32();
        if x == 1 {
            x = 2;
        }
        let s = format!("{x} 2 3");
        assert_same_and_expect(s.as_bytes(), &e1_out(), "E1 x != 1");
    }
    for x in [0, -1, 2, i32::MIN, i32::MAX] {
        assert_same_and_expect(format!("{x} 2 3").as_bytes(), &e1_out(), "E1 boundary");
    }
}

/// E2 — `x == 1` but `if (y != 2)`.
#[test]
fn e2_y_not_two() {
    let mut rng = Rng::new(0xE2);
    for _ in 0..200 {
        let mut y = rng.next_i32();
        if y == 2 {
            y = 3;
        }
        assert_same_and_expect(format!("1 {y} 3").as_bytes(), &e2_out(), "E2 y != 2");
    }
    for y in [0, 1, 3, 123, -2, i32::MIN, i32::MAX] {
        assert_same_and_expect(format!("1 {y} 3").as_bytes(), &e2_out(), "E2 boundary");
    }
}

/// E3 — `x == 1`, `y == 2`, but `if (z != 3)`.
#[test]
fn e3_z_not_three() {
    let mut rng = Rng::new(0xE3);
    for _ in 0..200 {
        let mut z = rng.next_i32();
        if z == 3 {
            z = 4;
        }
        assert_same_and_expect(format!("1 2 {z}").as_bytes(), &e3_out(), "E3 z != 3");
    }
    for z in [0, 2, 4, -3, i32::MIN, i32::MAX] {
        assert_same_and_expect(format!("1 2 {z}").as_bytes(), &e3_out(), "E3 boundary");
    }
}

/// E4 — the shared `fail:` epilogue: "Operation failed" always follows the
/// specific message, and `Result:` is never 0 on a failing path.
#[test]
fn e4_fail_epilogue() {
    for (input, expect) in [
        ("0 2 3", e1_out()),
        ("1 0 3", e2_out()),
        ("1 2 0", e3_out()),
    ] {
        assert_same_and_expect(input.as_bytes(), &expect, "E4 fail epilogue");
        let c = run(&c_bin(), input.as_bytes());
        let text = String::from_utf8_lossy(&c.stdout).to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "E4 three lines: {text:?}");
        assert_eq!(lines[1], "Operation failed", "E4 second line");
        assert!(lines[2].starts_with("Result: "), "E4 third line");
        assert_ne!(lines[2], "Result: 0", "E4 failing paths never return 0");
    }
    // The success path has exactly two lines and never prints the epilogue.
    let c = run(&c_bin(), b"1 2 3");
    assert_eq!(String::from_utf8_lossy(&c.stdout), "Ok!\nResult: 0\n");
}

/// E5 — scanf input failure with zero conversions (immediate EOF).
#[test]
fn e5_immediate_eof() {
    assert_same_and_expect(b"", &e1_out(), "E5 empty stdin");
    assert_same_cfg(&In::DevNull, Out::Pipe, &[], "E5 /dev/null stdin");
}

/// E6 — whitespace-only stdin (EOF while skipping whitespace).
#[test]
fn e6_whitespace_only() {
    for s in [
        " ", "\n", "\t", "\r", "\x0b", "\x0c", "   ", "\n\n\n", " \t\r\n\x0b\x0c",
        &" ".repeat(10000),
    ] {
        assert_same_and_expect(s.as_bytes(), &e1_out(), "E6 whitespace only");
    }
}

/// E7 — matching failure on the first `%d` (first non-space byte not in [0-9+-]).
#[test]
fn e7_matching_failure_first() {
    for s in [
        "abc", ".", ",", "x 2 3", "?", "/1 2 3", ":", "e", "E", "'", "\"", "#",
        "*1 2 3", "[1 2 3]", "one two three", "NaN", "inf", "0b101", "_1",
        "  \n\t hello world", "--1 2 3", "++1 2 3", "-+1", "+-1",
    ] {
        assert_same_and_expect(s.as_bytes(), &e1_out(), "E7 matching failure");
    }
}

/// E8 — sign with no digit after it.
#[test]
fn e8_sign_without_digits() {
    for s in ["-", "+", "- 1 2", "+ 1 2", "-\n", "+\t", "-x", "+x", "-.", "+.", "- ", "+ "] {
        assert_same_and_expect(s.as_bytes(), &e1_out(), "E8 lone sign");
    }
}

/// E9 — one conversion then input failure (EOF).
#[test]
fn e9_one_then_eof() {
    assert_same_and_expect(b"1", &e2_out(), "E9 x==1 then EOF");
    assert_same_and_expect(b"1\n", &e2_out(), "E9 x==1, newline, EOF");
    assert_same_and_expect(b"1   ", &e2_out(), "E9 x==1, trailing ws");
    // x != 1 short-circuits at stage 1 instead.
    for x in ["0", "2", "-1", "2147483647"] {
        assert_same_and_expect(format!("{x}").as_bytes(), &e1_out(), "E9 x!=1 then EOF");
    }
}

/// E10 — one conversion then matching failure.
#[test]
fn e10_one_then_matching_failure() {
    for s in ["1 x 3", "1 - 3", "1 . 3", "1 + 3", "1 abc 3", "1 , 3", "1 -- 3", "1 .5 3", "1 x", "1\nx\n3"] {
        assert_same_and_expect(s.as_bytes(), &e2_out(), "E10 second token invalid");
    }
}

/// E11 — two conversions then input failure (EOF).
#[test]
fn e11_two_then_eof() {
    assert_same_and_expect(b"1 2", &e3_out(), "E11 EOF after 2nd");
    assert_same_and_expect(b"1 2\n", &e3_out(), "E11 newline then EOF");
    assert_same_and_expect(b"1 2 \t\n ", &e3_out(), "E11 trailing whitespace");
    assert_same_and_expect(b"1 5", &e2_out(), "E11 y!=2 short-circuits");
}

/// E12 — two conversions then matching failure.
#[test]
fn e12_two_then_matching_failure() {
    for s in ["1 2 x", "1 2 -", "1 2 +", "1 2 .", "1 2 abc", "1 2 -x", "1 2 +.", "1 2 \x00"] {
        assert_same_and_expect(s.as_bytes(), &e3_out(), "E12 third token invalid");
    }
}

/// E13 — a later scanf failure does not change the order of the stage checks.
#[test]
fn e13_check_order_wins() {
    for s in ["7 x 3", "7", "0 x", "-5 . .", "999", "0"] {
        assert_same_and_expect(s.as_bytes(), &e1_out(), "E13 x!=1 checked first");
    }
    // and y is checked before z
    for s in ["1 9 x", "1 9", "1 9 9"] {
        assert_same_and_expect(s.as_bytes(), &e2_out(), "E13 y!=2 checked before z");
    }
}

/// E14 — positive out-of-`long` range: strtol saturates to LONG_MAX, narrowed
/// to int == -1.
#[test]
fn e14_positive_saturation() {
    let huge = [
        "9223372036854775808".to_string(),
        "18446744073709551616".to_string(),
        "99999999999999999999".to_string(),
        "9".repeat(400),
        "1".to_string() + &"0".repeat(100),
        "9".repeat(100_000),
    ];
    for v in &huge {
        // Saturated LONG_MAX narrows to -1, so stage 1 rejects.
        assert_same_and_expect(format!("{v} 2 3").as_bytes(), &e1_out(), "E14 x saturates to -1");
        assert_same_and_expect(format!("1 {v} 3").as_bytes(), &e2_out(), "E14 y saturates to -1");
        assert_same_and_expect(format!("1 2 {v}").as_bytes(), &e3_out(), "E14 z saturates to -1");
    }
}

/// E15 — negative out-of-`long` range: saturates to LONG_MIN, narrowed to 0.
#[test]
fn e15_negative_saturation() {
    let huge = [
        "-9223372036854775809".to_string(),
        "-18446744073709551616".to_string(),
        "-99999999999999999999".to_string(),
        "-".to_string() + &"9".repeat(400),
        "-".to_string() + &"9".repeat(100_000),
    ];
    for v in &huge {
        assert_same_and_expect(format!("{v} 2 3").as_bytes(), &e1_out(), "E15 x saturates to 0");
        assert_same_and_expect(format!("1 {v} 3").as_bytes(), &e2_out(), "E15 y saturates to 0");
        assert_same_and_expect(format!("1 2 {v}").as_bytes(), &e3_out(), "E15 z saturates to 0");
    }
}

/// E16 — in-`long`, out-of-`int` values: narrowed mod 2^32.
#[test]
fn e16_int_narrowing() {
    // 2147483648 -> INT_MIN, so stage 1 rejects.
    assert_same_and_expect(b"2147483648 2 3", &e1_out(), "E16 INT_MAX+1");
    // 4294967297 -> 1, so stage 1 *passes*.
    assert_same_and_expect(b"4294967297 2 3", "Ok!\nResult: 0\n", "E16 narrows to 1");
    assert_same_and_expect(b"-4294967295 2 3", "Ok!\nResult: 0\n", "E16 negative narrows to 1");
    assert_same_and_expect(b"1 4294967298 4294967299", "Ok!\nResult: 0\n", "E16 y,z narrow");
    let mut rng = Rng::new(0xE16);
    for _ in 0..200 {
        let hi = 1 + rng.below(1_000_000) as i64;
        let v = hi * 4_294_967_296 + rng.below(4_294_967_296) as i64;
        let v = if rng.below(2) == 0 { v } else { -v };
        assert_same_str(&format!("{v} {v} {v}"), "E16 random narrowing");
    }
}

/// E17 — `%d` is base 10, so "0x10" converts 0 and then jams the scan on 'x'.
#[test]
fn e17_hex_prefix() {
    assert_same_and_expect(b"0x10 2 3", &e1_out(), "E17 0x10 -> x=0");
    assert_same_and_expect(b"1 0x2 3", &e2_out(), "E17 y jams");
    assert_same_and_expect(b"1 2 0x3", &e3_out(), "E17 z jams");
    assert_same_and_expect(b"0X1F 2 3", &e1_out(), "E17 uppercase");
    assert_same_and_expect(b"010 2 3", &e1_out(), "E17 no octal for %d (010 == 10)");
    assert_same_and_expect(b"1 010 3", &e2_out(), "E17 010 != 2");
    assert_same_and_expect(b"0b1 2 3", &e1_out(), "E17 binary prefix");
}

/// E18 — float-looking input stops at the '.'.
#[test]
fn e18_float_like() {
    assert_same_and_expect(b"1.5 2.5 3.5", &e2_out(), "E18 x=1 then '.' fails");
    assert_same_and_expect(b"0.5 2 3", &e1_out(), "E18 x=0");
    // y converts as 2 (the scan stops at '.'), then the z conversion fails on
    // that same '.', so z keeps 0 and stage 3 rejects.
    assert_same_and_expect(b"1 2.0 3", &e3_out(), "E18 y=2 but '.' stops before z");
    assert_same_and_expect(b"1 2 3.0", "Ok!\nResult: 0\n", "E18 z=3 then '.' unread");
    assert_same_and_expect(b"1e5 2 3", &e2_out(), "E18 exponent form");
    assert_same_and_expect(b"1 2 3e5", "Ok!\nResult: 0\n", "E18 trailing exponent unread");
}

/// E19 — NUL and non-ASCII bytes.
#[test]
fn e19_nul_and_non_ascii() {
    let cases: [&[u8]; 10] = [
        b"\x00 1 2",
        b"\xff 1 2",
        b"\xc3\xa9 1 2",
        b"\x01\x02\x03",
        b"\x80",
        b"\xa0 1 2 3", // NBSP is not isspace in the C locale
        b"1\x002 3",
        b"1 \x002 3",
        b"\x7f 1 2",
        b"\x1b[0m 1 2 3",
    ];
    for c in cases {
        assert_same(c, "E19 non-numeric bytes");
    }
    assert_same_and_expect(b"\x00 1 2", &e1_out(), "E19 NUL first");
    assert_same_and_expect(b"1\x002 3", &e2_out(), "E19 NUL stops after x");
}

/// E20 — stdin closed (fd 0 not open) => read fails, nothing converted.
#[test]
fn e20_stdin_closed() {
    assert_same_cfg(&In::Closed, Out::Pipe, &[], "E20 stdin closed");
    let c = run_cfg(&c_bin(), &In::Closed, Out::Pipe, &[]);
    assert_eq!(String::from_utf8_lossy(&c.stdout), e1_out(), "E20 C ground truth");
    assert_eq!(c.code, Some(0));
}

/// E21 — stdin is a directory => read fails with EISDIR.
#[test]
fn e21_stdin_is_directory() {
    let dir: PathBuf = std::env::temp_dir();
    assert_same_cfg(&In::Path(dir.clone()), Out::Pipe, &[], "E21 stdin is a directory");
    let c = run_cfg(&c_bin(), &In::Path(dir), Out::Pipe, &[]);
    assert_eq!(String::from_utf8_lossy(&c.stdout), e1_out(), "E21 C ground truth");
    assert_eq!(c.code, Some(0));
}

/// E22 — the ignored scanf return value never affects the exit status and never
/// produces a diagnostic of its own.
#[test]
fn e22_scanf_return_ignored() {
    for s in ["", "abc", "1", "1 2", "1 2 3", "-", "\x00"] {
        let c = run(&c_bin(), s.as_bytes());
        let r = run(Path::new(RUST_BIN), s.as_bytes());
        assert_eq!(c.code, Some(0), "E22 C exit status for {s:?}");
        assert_eq!(r.code, Some(0), "E22 Rust exit status for {s:?}");
        assert!(c.stderr.is_empty() && r.stderr.is_empty(), "E22 nothing on stderr");
        assert_eq!(c, r, "E22 identical for {s:?}");
        let lines = String::from_utf8_lossy(&c.stdout).lines().count();
        assert!(lines == 2 || lines == 3, "E22 no extra diagnostics: {lines}");
    }
}

/// E23 — printf write errors are ignored (stdout on /dev/full => ENOSPC).
#[test]
fn e23_write_error_ignored() {
    for s in ["1 2 3", "0 2 3", "", "1"] {
        assert_same_cfg(&In::Pipe(s.as_bytes()), Out::DevFull, &[], "E23 /dev/full");
        let c = run_cfg(&c_bin(), &In::Pipe(s.as_bytes()), Out::DevFull, &[]);
        assert_eq!(c.code, Some(0), "E23 C still exits 0");
        assert_eq!(c.signal, None, "E23 C not signalled");
    }
}

/// E24 — broken stdout pipe: a C process keeps the default SIGPIPE disposition
/// and is killed by signal 13; the Rust translation must do the same.
#[test]
fn e24_broken_stdout_pipe_sigpipe() {
    for s in ["1 2 3", "0 2 3", "", "1 2"] {
        assert_same_cfg(&In::Pipe(s.as_bytes()), Out::BrokenPipe, &[], "E24 broken pipe");
        let c = run_cfg(&c_bin(), &In::Pipe(s.as_bytes()), Out::BrokenPipe, &[]);
        assert_eq!(c.signal, Some(13), "E24 C dies from SIGPIPE for {s:?}");
        let r = run_cfg(Path::new(RUST_BIN), &In::Pipe(s.as_bytes()), Out::BrokenPipe, &[]);
        assert_eq!(r.signal, Some(13), "E24 Rust dies from SIGPIPE for {s:?}");
    }
}

/// E25 — argv is never inspected (`int main()` takes no parameters).
#[test]
fn e25_argv_ignored() {
    let argvs: [&[&str]; 6] = [
        &[],
        &["a", "b", "c"],
        &[""],
        &["--help"],
        &["-1", "-2", "-3"],
        &["with space", "with\nnewline", "\u{1f600}"],
    ];
    for args in argvs {
        for s in ["1 2 3", "0 2 3", "", "1"] {
            assert_same_cfg(&In::Pipe(s.as_bytes()), Out::Pipe, args, "E25 argv ignored");
        }
    }
}

/// Extra generic FFI-boundary boundaries required by Phase C: values one step
/// past every documented range, and "enum-like" out-of-range ints for the three
/// stage constants.
#[test]
fn e_generic_boundaries() {
    // One step past each magic constant, in every slot.
    for magic in [(1i64, 0usize), (2, 1), (3, 2)] {
        for delta in [-1i64, 0, 1] {
            let v = magic.0 + delta;
            let mut t = [1i64, 2, 3];
            t[magic.1] = v;
            assert_same_str(&format!("{} {} {}", t[0], t[1], t[2]), "boundary ±1");
        }
    }
    // "Out-of-range enum" analogue: ints far outside any meaningful set.
    for v in [i32::MIN, i32::MIN + 1, -2, 4, 5, 99, i32::MAX - 1, i32::MAX] {
        assert_same_str(&format!("{v} {v} {v}"), "out-of-range ints");
        assert_same_str(&format!("{v} 2 3"), "out-of-range x");
        assert_same_str(&format!("1 {v} 3"), "out-of-range y");
        assert_same_str(&format!("1 2 {v}"), "out-of-range z");
    }
    // Zero-length and oversized inputs.
    assert_same(b"", "zero length stdin");
    let big = format!("{}1 2 3", " ".repeat(200_000));
    assert_same_str(&big, "oversized leading whitespace");
    let many = "1 2 3 ".repeat(50_000);
    assert_same_str(&many, "oversized trailing tokens");
}
