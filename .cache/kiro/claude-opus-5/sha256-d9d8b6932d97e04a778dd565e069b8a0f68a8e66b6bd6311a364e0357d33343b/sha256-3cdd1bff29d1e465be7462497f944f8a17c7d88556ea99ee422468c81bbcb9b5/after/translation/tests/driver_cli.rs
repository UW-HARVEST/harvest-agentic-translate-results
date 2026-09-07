//! Driver-level differential tests — `mdmain.c` vs `src/main.rs`.
//!
//! `main` is not part of the `.so`, so these run the two `driver` executables
//! as subprocesses and compare stdout, stderr and exit status byte for byte.
//!
//! Covers ERRORS.md rows 7-10 and CONFIGS.md rows 24-25.

mod common;

use common::*;
use std::process::Command;

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} stdout={:?} stderr={:?}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(exe: &std::path::Path, args: &[&str]) -> Run {
    let out = Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Compares the two drivers on one argument vector.
///
/// `argv[0]` differs between the two executables, and `mdmain.c` prints it in
/// the usage message, so the usage line is compared with the program name
/// elided -- everything else is compared verbatim.
fn diff_driver(args: &[&str]) {
    let (c_exe, rust_exe) = driver_paths();
    let c = run(c_exe, args);
    let r = run(rust_exe, args);

    assert_eq!(
        c.code, r.code,
        "[{}] exit status for {args:?}\n  C:    {c:?}\n  Rust: {r:?}",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "[{}] stdout for {args:?}",
        tag()
    );

    let strip = |bytes: &[u8], exe: &std::path::Path| -> String {
        let s = String::from_utf8_lossy(bytes).into_owned();
        s.replace(&exe.display().to_string(), "<argv0>")
    };
    assert_eq!(
        strip(&c.stderr, c_exe),
        strip(&r.stderr, rust_exe),
        "[{}] stderr for {args:?}\n  C:    {c:?}\n  Rust: {r:?}",
        tag()
    );
}

/// CONFIGS.md row 24 — the ordinary `argc == 3` path.
#[test]
fn row24_numeric_operands() {
    let fixed = [
        ["0", "0"],
        ["1", "1"],
        ["7", "3"],
        ["-5", "9"],
        ["9", "-5"],
        ["-1", "-1"],
        ["2147483647", "1"],
        ["-2147483648", "-1"],
        ["2147483647", "2147483647"],
        ["-2147483648", "-2147483648"],
        ["65536", "65536"],
        ["+12", "+34"],
        ["  12", "\t34"],
        ["007", "008"],
    ];
    for args in fixed {
        diff_driver(&[args[0], args[1]]);
    }

    let mut rng = Rng::new();
    for _ in 0..128 {
        let a = rng.next_operand().to_string();
        let b = rng.next_operand().to_string();
        diff_driver(&[&a, &b]);
    }
}

/// ERRORS.md row 7 — `argc < 3` is the library's single explicit rejection:
/// usage line on stderr, exit status exactly 2, nothing on stdout.
#[test]
fn row07_argc_too_small() {
    for args in [vec![], vec!["5"]] {
        diff_driver(&args);
        let (c_exe, rust_exe) = driver_paths();
        let c = run(c_exe, &args);
        let r = run(rust_exe, &args);
        assert_eq!(c.code, Some(2), "[{}] C exit status must be 2", tag());
        assert_eq!(r.code, Some(2), "[{}] Rust exit status must be 2", tag());
        assert!(
            c.stdout.is_empty() && r.stdout.is_empty(),
            "[{}] nothing may reach stdout on the usage path",
            tag()
        );
        assert!(
            c.stderr.ends_with(b" A B\n") && r.stderr.ends_with(b" A B\n"),
            "[{}] usage line shape: C={:?} Rust={:?}",
            tag(),
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr)
        );
        assert!(
            c.stderr.starts_with(b"usage: ") && r.stderr.starts_with(b"usage: "),
            "[{}] usage line prefix",
            tag()
        );
    }
}

/// ERRORS.md row 8 / CONFIGS.md row 25 — surplus operands are ignored, not rejected.
#[test]
fn row08_extra_args_ignored() {
    diff_driver(&["1", "2", "3"]);
    diff_driver(&["1", "2", "3", "4", "5"]);
    diff_driver(&["7", "3", "ignored", "-999"]);

    let (c_exe, rust_exe) = driver_paths();
    let two = run(c_exe, &["1", "2"]);
    let four = run(c_exe, &["1", "2", "3", "4"]);
    assert_eq!(
        two.stdout, four.stdout,
        "[{}] C: argv[3..] must not change the output",
        tag()
    );
    let two = run(rust_exe, &["1", "2"]);
    let four = run(rust_exe, &["1", "2", "3", "4"]);
    assert_eq!(
        two.stdout, four.stdout,
        "[{}] Rust: argv[3..] must not change the output",
        tag()
    );
    assert_eq!(two.code, Some(0));
    assert_eq!(four.code, Some(0));
}

/// ERRORS.md row 9 — `atoi` on input with no (or partial) digits: silently 0 or
/// the parsed prefix, no diagnostic, exit 0.
#[test]
fn row09_non_numeric_operands() {
    for args in [
        ["", ""],
        ["abc", "xyz"],
        ["+", "-"],
        ["-", "+"],
        ["--3", "++4"],
        ["0x10", "0X20"],
        ["12abc", "34xyz"],
        [".5", ",5"],
        ["1e3", "2E4"],
        ["   ", "\t\n"],
        ["-abc", "+abc"],
        ["1 2", "3 4"],
        ["\u{00e9}", "\u{20ac}"],
        ["1-2", "3+4"],
        ["9999999999999999999999x", "y"],
    ] {
        diff_driver(&[args[0], args[1]]);
        let (c_exe, _) = driver_paths();
        let c = run(c_exe, &[args[0], args[1]]);
        assert_eq!(
            c.code,
            Some(0),
            "[{}] non-numeric operands are not rejected: {args:?}",
            tag()
        );
        assert!(
            c.stderr.is_empty(),
            "[{}] no diagnostic expected for {args:?}",
            tag()
        );
    }
}

/// ERRORS.md row 10 — operands outside `int` range. glibc `atoi` is
/// `(int)strtol(...)`, so the value saturates at `LONG_MAX`/`LONG_MIN` and is
/// then truncated to 32 bits; no diagnostic, exit 0.
#[test]
fn row10_out_of_range_operands() {
    for args in [
        ["2147483648", "0"],
        ["-2147483649", "0"],
        ["4294967296", "0"],
        ["4294967295", "0"],
        ["9223372036854775807", "0"],
        ["9223372036854775808", "0"],
        ["-9223372036854775808", "0"],
        ["-9223372036854775809", "0"],
        ["99999999999999999999999", "0"],
        ["-99999999999999999999999", "0"],
        ["0", "99999999999999999999999"],
        [
            "123456789012345678901234567890123456789012345678901234567890",
            "-123456789012345678901234567890",
        ],
        ["000000000000000000002147483648", "0"],
    ] {
        diff_driver(&[args[0], args[1]]);
        let (c_exe, _) = driver_paths();
        let c = run(c_exe, &[args[0], args[1]]);
        assert_eq!(c.code, Some(0), "[{}] out-of-range is not rejected", tag());
        assert!(c.stderr.is_empty(), "[{}] no diagnostic expected", tag());
    }
}

/// The driver's stdout is five lines whose shape is fixed by `mdmain.c`; assert
/// the shape as well as the equality, so an "empty == empty" pass is impossible.
#[test]
fn driver_output_shape() {
    let (c_exe, rust_exe) = driver_paths();
    for exe in [c_exe, rust_exe] {
        let out = run(exe, &["7", "3"]);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5, "[{}] {text:?}", tag());
        assert!(lines[0].starts_with("helper.call=") && lines[0].contains(" helper.acc="));
        assert!(lines[1].starts_with("helper.ptr="));
        assert!(lines[2].starts_with("gen.acc="));
        assert!(lines[3].starts_with(&format!("op={} call=", OP.cmake_value())));
        assert!(lines[3].contains(" acc=") && lines[3].contains(" g.call="));
        assert!(lines[4].starts_with("summary="));
        assert_eq!(out.code, Some(0));
    }
    diff_driver(&["7", "3"]);
}

/// `use_generated(REPEAT)` is what `main` calls, so `gen.acc` on line 3 is the
/// place the REPEAT=7 switch asymmetry becomes visible end to end.
#[test]
fn driver_gen_acc_reflects_dispatch_rep() {
    let (c_exe, rust_exe) = driver_paths();
    let want = format!("gen.acc={}", dispatch_reference(REPEAT));
    for exe in [c_exe, rust_exe] {
        let out = run(exe, &["7", "3"]);
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let line = text.lines().nth(2).unwrap_or("");
        assert_eq!(line, want, "[{}] {}", tag(), exe.display());
    }
}

/// Randomized `atoi` fuzz, property-style with the fixed harness seed.
///
/// The operand strings are the only textual input the program has, and glibc
/// `atoi` has a sign-asymmetric overflow clamp (`LONG_MAX` vs `LONG_MIN`) that a
/// hand-written parser gets wrong easily. Generating digit strings of every
/// length from 1 to 30, with and without signs, whitespace and trailing junk,
/// crosses that boundary from both sides many times.
#[test]
fn atoi_randomized_strings() {
    let mut rng = Rng::new();
    let mut cases: Vec<String> = Vec::new();

    for _ in 0..300 {
        let len = 1 + (rng.next_u64() % 30) as usize;
        let mut s = String::new();
        match rng.next_u64() % 6 {
            0 => s.push('-'),
            1 => s.push('+'),
            2 => s.push_str("  "),
            3 => s.push_str(" -"),
            4 => s.push('\t'),
            _ => {}
        }
        for _ in 0..len {
            s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
        }
        match rng.next_u64() % 5 {
            0 => s.push('x'),
            1 => s.push_str(".5"),
            2 => s.push(' '),
            _ => {}
        }
        cases.push(s);
    }

    // Every decimal string within one of the two clamp boundaries.
    for base in [
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "9223372036854775809",
        "18446744073709551615",
        "18446744073709551616",
    ] {
        cases.push(base.to_string());
        cases.push(format!("-{base}"));
        cases.push(format!("+{base}"));
        cases.push(format!("000{base}"));
        cases.push(format!("-000{base}"));
    }

    for a in &cases {
        diff_driver(&[a.as_str(), "0"]);
    }
    // And as the second operand, so both call sites are covered.
    for a in cases.iter().take(60) {
        diff_driver(&["0", a.as_str()]);
    }
}
