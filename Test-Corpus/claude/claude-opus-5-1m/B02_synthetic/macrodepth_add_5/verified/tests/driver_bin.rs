//! `CONFIGS.md` rows 28-32/37 and `ERRORS.md` rows 1-4 / 18-23 — the `driver`
//! executable (`mdmain.c`). stdout, stderr and exit status are all compared.

mod common;

use common::*;
use std::process::{Command, Output};

fn run(exe: &std::path::Path, args: &[&str]) -> Output {
    Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {exe:?}: {e}"))
}

/// Run both executables with `args` and assert stdout, stderr and status match.
fn diff_run(args: &[&str]) -> String {
    let c = run(c_exe_path(), args);
    let r = run(rust_exe_path(), args);

    assert_eq!(
        show(&c.stdout),
        show(&r.stdout),
        "stdout mismatch for args {args:?} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        show(&c.stderr).replace(&*c_exe_path().to_string_lossy(), "<exe>"),
        show(&r.stderr).replace(&*rust_exe_path().to_string_lossy(), "<exe>"),
        "stderr mismatch for args {args:?} (argv[0] normalised)"
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for args {args:?}: C={:?} Rust={:?}",
        c.status.code(),
        r.status.code()
    );
    show(&c.stdout)
}

/* -------------------- ERRORS.md rows 1-4: the argc guard -------------------- */

#[test]
fn usage_no_args() {
    // Row 1: argc == 1
    let c = run(c_exe_path(), &[]);
    let r = run(rust_exe_path(), &[]);
    assert_eq!(c.status.code(), Some(2), "C must exit 2");
    assert_eq!(r.status.code(), Some(2), "Rust must exit 2");
    assert!(c.stdout.is_empty(), "C must print nothing on stdout");
    assert!(r.stdout.is_empty(), "Rust must print nothing on stdout");
    // "usage: %s A B\n" with argv[0] = the path we invoked.
    assert_eq!(
        show(&c.stderr),
        format!("usage: {} A B\n", c_exe_path().display())
    );
    assert_eq!(
        show(&r.stderr),
        format!("usage: {} A B\n", rust_exe_path().display())
    );
}

#[test]
fn usage_one_arg() {
    // Row 2: argc == 2
    let c = run(c_exe_path(), &["5"]);
    let r = run(rust_exe_path(), &["5"]);
    assert_eq!(c.status.code(), Some(2));
    assert_eq!(r.status.code(), Some(2));
    assert!(c.stdout.is_empty() && r.stdout.is_empty());
    assert!(show(&c.stderr).ends_with(" A B\n"));
    assert_eq!(
        show(&c.stderr).replace(&*c_exe_path().to_string_lossy(), "<exe>"),
        show(&r.stderr).replace(&*rust_exe_path().to_string_lossy(), "<exe>")
    );
}

#[test]
fn argc_boundary_two_args() {
    // Row 3: argc == 3, the first accepted value.
    let out = diff_run(&["3", "4"]);
    assert!(!out.is_empty(), "the 2-argument case must produce output");
    let c = run(c_exe_path(), &["3", "4"]);
    assert_eq!(c.status.code(), Some(0));
}

#[test]
fn extra_args_ignored() {
    // Row 4: surplus argv entries are never read.
    let two = diff_run(&["3", "4"]);
    let many = diff_run(&["3", "4", "5", "ignored", "-9"]);
    assert_eq!(two, many, "extra arguments must not change the output");
}

/* -------------------- CONFIGS.md rows 28-32: full output -------------------- */

#[test]
fn rows28_32_fixed_arguments() {
    // The whole 5-line stdout (helper.call/helper.acc, helper.ptr, gen.acc,
    // op=/call=/acc=/g.call=, summary=) for the configuration under test.
    let out = diff_run(&["3", "4"]);
    assert!(
        out.contains(&format!("op={OP} ")),
        "output should name the configured OP: {out:?}"
    );
    assert!(out.contains("summary="), "output should end with summary=");
    assert_eq!(out.lines().count(), 5, "mdmain prints 5 lines: {out:?}");
}

#[test]
fn rows28_32_main_calls_use_generated_with_repeat() {
    // At REPEAT=7 main's `use_generated(REPEAT)` hits the switch's default arm,
    // so gen.acc collapses to INIT even though helper.acc performed 7 steps.
    let out = diff_run(&["3", "4"]);
    let gen = out
        .lines()
        .find(|l| l.starts_with("gen.acc="))
        .expect("gen.acc line")
        .trim_start_matches("gen.acc=")
        .parse::<i32>()
        .expect("parse gen.acc");
    if REPEAT == 7 {
        assert_eq!(
            gen,
            init_for_op(),
            "REPEAT=7 must take the default: arm of DISPATCH_REP"
        );
    }
}

#[test]
fn row31_randomized_argument_pairs() {
    let mut rng = Rng::with_seed(0xD00D_0031);
    let mut args: Vec<(String, String)> = vec![
        ("0".into(), "0".into()),
        ("-1".into(), "1".into()),
        ("2147483647".into(), "1".into()),
        ("-2147483648".into(), "-1".into()),
        ("2147483647".into(), "2147483647".into()),
        ("-2147483648".into(), "-2147483648".into()),
    ];
    for _ in 0..40 {
        args.push((rng.next_mixed().to_string(), rng.next_mixed().to_string()));
    }
    for (a, b) in args {
        diff_run(&[&a, &b]);
    }
}

/* -------------------- ERRORS.md rows 18-23: atoi behaviour ------------------ */

#[test]
fn atoi_non_numeric() {
    // Row 18: atoi returns 0, main proceeds normally and exits 0.
    for (a, b) in [
        ("abc", "def"),
        ("", ""),
        ("+", "-"),
        ("x1", "y2"),
        (" ", "\t"),
        ("--5", "++5"),
        (".5", ","),
        ("0x10", "0b1"),
    ] {
        diff_run(&[a, b]);
    }
    let zero = diff_run(&["0", "0"]);
    let junk = diff_run(&["abc", "def"]);
    assert_eq!(zero, junk, "non-numeric arguments must behave exactly like 0");
    assert_eq!(run(c_exe_path(), &["abc", "def"]).status.code(), Some(0));
}

#[test]
fn atoi_trailing_garbage() {
    // Row 19: leading digits only.
    for (a, b) in [
        ("12abc", "3.9"),
        ("7 8", "9,10"),
        ("5-3", "6+2"),
        ("42\n", "13\t"),
    ] {
        diff_run(&[a, b]);
    }
    assert_eq!(diff_run(&["12abc", "3.9"]), diff_run(&["12", "3"]));
}

#[test]
fn atoi_whitespace_and_sign() {
    // Row 20
    for (a, b) in [
        ("  42", "\t-7"),
        ("+5", "-5"),
        ("\n\r\x0b\x0c 8", " +9"),
        ("   -0", "+0"),
    ] {
        diff_run(&[a, b]);
    }
    assert_eq!(diff_run(&["  42", "\t-7"]), diff_run(&["42", "-7"]));
    assert_eq!(diff_run(&["+5", "-5"]), diff_run(&["5", "-5"]));
}

#[test]
fn atoi_int_boundaries() {
    // Row 23
    for (a, b) in [
        ("2147483647", "2147483647"),
        ("-2147483648", "-2147483648"),
        ("2147483646", "1"),
        ("-2147483647", "-1"),
    ] {
        diff_run(&[a, b]);
    }
}

#[test]
fn atoi_int_overflow() {
    // Row 21: fits in long, truncated to int by the (int) cast in atoi.
    for (a, b) in [
        ("2147483648", "0"),
        ("-2147483649", "0"),
        ("4294967296", "0"),
        ("4294967297", "0"),
        ("8589934592", "-8589934592"),
        ("2147483648", "-2147483649"),
    ] {
        diff_run(&[a, b]);
    }
    // 2147483648 truncates to INT_MIN, so it must behave like -2147483648.
    assert_eq!(diff_run(&["2147483648", "0"]), diff_run(&["-2147483648", "0"]));
    // 4294967296 == 2^32 truncates to 0.
    assert_eq!(diff_run(&["4294967296", "0"]), diff_run(&["0", "0"]));
}

#[test]
fn atoi_long_overflow() {
    // Row 22: strtol saturates at LONG_MAX/LONG_MIN, then truncates.
    for (a, b) in [
        ("99999999999999999999", "0"),
        ("-99999999999999999999", "0"),
        ("9223372036854775807", "0"),
        ("9223372036854775808", "0"),
        ("-9223372036854775808", "0"),
        ("-9223372036854775809", "0"),
        (
            "1234567890123456789012345678901234567890",
            "-1234567890123456789012345678901234567890",
        ),
    ] {
        diff_run(&[a, b]);
    }
    // LONG_MAX truncates to -1; LONG_MIN truncates to 0.
    assert_eq!(
        diff_run(&["99999999999999999999", "0"]),
        diff_run(&["-1", "0"])
    );
    assert_eq!(
        diff_run(&["-99999999999999999999", "0"]),
        diff_run(&["0", "0"])
    );
}

#[test]
fn atoi_leading_zeros_and_long_digit_strings() {
    for (a, b) in [
        ("0000000000000000000000007", "-0000000005"),
        ("000", "-000"),
        ("00000000002147483647", "0"),
    ] {
        diff_run(&[a, b]);
    }
    assert_eq!(
        diff_run(&["0000000000000000000000007", "-0000000005"]),
        diff_run(&["7", "-5"])
    );
}

#[test]
fn atoi_randomized_fuzz() {
    // `atoi` is the only piece of libc that main.rs reimplements by hand
    // (glibc defines it as `(int)strtol(nptr, NULL, 10)`), so it is the most
    // divergence-prone logic in the translation. Fuzz it with a fixed seed over
    // an alphabet that mixes digits, signs, whitespace and junk, plus digit runs
    // long enough to overflow `int` and `long`.
    let mut rng = Rng::with_seed(0xA701_F022);
    const ALPHABET: &[u8] = b"0123456789+- \t\n\rxabcdef.,e";

    let mut cases: Vec<String> = Vec::new();
    for _ in 0..90 {
        let len = rng.range(0, 12) as usize;
        let s: String = (0..len)
            .map(|_| ALPHABET[rng.range(0, ALPHABET.len() as i32 - 1) as usize] as char)
            .collect();
        cases.push(s);
    }
    // Long digit runs around the int/long boundaries.
    for _ in 0..30 {
        let len = rng.range(1, 25) as usize;
        let mut s = String::new();
        if rng.range(0, 2) == 0 {
            s.push(if rng.range(0, 1) == 0 { '-' } else { '+' });
        }
        for _ in 0..len {
            s.push((b'0' + rng.range(0, 9) as u8) as char);
        }
        cases.push(s);
    }
    // Digit strings of every length from 1 to 21 (straddles INT and LONG width).
    for n in 1..=21 {
        cases.push("9".repeat(n));
        cases.push(format!("-{}", "9".repeat(n)));
        cases.push(format!("1{}", "0".repeat(n)));
    }
    // A pathologically long one.
    cases.push("1".repeat(5000));
    cases.push(format!("-{}", "7".repeat(3000)));

    for chunk in cases.chunks(2) {
        let a = &chunk[0];
        let b = chunk.get(1).map(String::as_str).unwrap_or("1");
        diff_run(&[a, b]);
    }
}

#[test]
fn non_utf8_arguments_are_handled_like_c() {
    // C's atoi sees raw bytes; the Rust must not require valid UTF-8 (a
    // `String`-based translation would panic or lose bytes here).
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let cases: Vec<(OsString, OsString)> = vec![
        (
            OsString::from_vec(vec![0xff, 0xfe]),
            OsString::from_vec(vec![0x80]),
        ),
        (
            OsString::from_vec(vec![b'4', b'2', 0xff]),
            OsString::from_vec(vec![0xc3, b'7']),
        ),
        (
            OsString::from_vec(vec![b'-', b'9', 0x80, b'1']),
            OsString::from_vec(vec![0xff, b'5']),
        ),
    ];
    for (a, b) in cases {
        let c = Command::new(c_exe_path())
            .arg(&a)
            .arg(&b)
            .output()
            .expect("spawn C");
        let r = Command::new(rust_exe_path())
            .arg(&a)
            .arg(&b)
            .output()
            .expect("spawn Rust");
        assert_eq!(
            show(&c.stdout),
            show(&r.stdout),
            "stdout mismatch for non-UTF-8 args {a:?} {b:?}"
        );
        assert_eq!(c.status.code(), r.status.code());
    }
}

#[test]
fn empty_argv0_is_formatted_identically() {
    // `%s` on an empty argv[0]. (argc == 0, only reachable via a bare execve,
    // was additionally verified out-of-band to be byte-identical: both print
    // "usage:  A B\n" and exit 2.)
    use std::os::unix::process::CommandExt;
    let c = Command::new(c_exe_path())
        .arg0("")
        .output()
        .expect("spawn C");
    let r = Command::new(rust_exe_path())
        .arg0("")
        .output()
        .expect("spawn Rust");
    assert_eq!(show(&c.stderr), "usage:  A B\n");
    assert_eq!(show(&c.stderr), show(&r.stderr));
    assert_eq!(c.status.code(), Some(2));
    assert_eq!(r.status.code(), Some(2));
}

/* ------------------ CONFIGS.md row 37: the CMake build itself --------------- */

#[test]
fn row37_cmake_build_matches_the_harness_build() {
    // Only meaningful for the CMake cache defaults (OP=add, REPEAT=5), which is
    // also what the header's #ifndef fallbacks produce.
    let cmake_exe = manifest_dir().join("c_src/build/driver");
    if !cmake_exe.exists() {
        eprintln!("skipping: CMake build not present at {cmake_exe:?}");
        return;
    }
    let is_default_cfg = (OP == "add") && REPEAT == 5;
    if !is_default_cfg {
        return;
    }
    for args in [
        vec!["3", "4"],
        vec!["0", "0"],
        vec!["-7", "11"],
        vec!["2147483647", "1"],
    ] {
        let cm = run(&cmake_exe, &args);
        let gcc = run(c_exe_path(), &args);
        let rust = run(rust_exe_path(), &args);
        assert_eq!(
            show(&cm.stdout),
            show(&gcc.stdout),
            "the harness's gcc flags do not reproduce the CMake build for {args:?}"
        );
        assert_eq!(
            show(&cm.stdout),
            show(&rust.stdout),
            "Rust driver differs from the CMake-built C driver for {args:?}"
        );
        assert_eq!(cm.status.code(), rust.status.code());
    }
}
