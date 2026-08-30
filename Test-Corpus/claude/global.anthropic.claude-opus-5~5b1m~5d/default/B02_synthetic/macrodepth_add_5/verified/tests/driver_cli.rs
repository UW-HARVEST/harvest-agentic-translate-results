//! End-to-end differential tests for the `driver` executable (`mdmain.c`).
//!
//! Covers `CONFIGS.md` rows 1–24 axis 5 level 7 (the composed pipeline: op +
//! unroll + all three helpers + the `G_OP` call + both `printf`s) and
//! `ERRORS.md` rows 18–21 (`argc < 3`, `atoi` behaviour, extra args).
//!
//! `argv[0]` is forced to the same string for both binaries with `arg0`, so the
//! `usage:` message — which interpolates `argv[0]` — is byte-comparable.

mod common;

use std::ffi::{c_char, c_int, CString};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use common::{c_driver_path, rust_driver_path, OP_NAME, REPEAT};

extern "C" {
    fn execve(path: *const c_char, argv: *const *const c_char, envp: *const *const c_char)
        -> c_int;
}

fn run(prog: &std::path::Path, args: &[&str]) -> Output {
    Command::new(prog)
        .arg0("driver")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawning {} failed: {e}", prog.display()))
}

#[track_caller]
fn assert_same(args: &[&str]) {
    let c = run(&c_driver_path(), args);
    let r = run(&rust_driver_path(), args);

    let ctx = format!("driver {args:?} [OP={OP_NAME} REPEAT={REPEAT}]");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "stdout differs for {ctx}"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "stderr differs for {ctx}"
    );
    assert_eq!(c.stdout, r.stdout, "stdout bytes differ for {ctx}");
    assert_eq!(c.stderr, r.stderr, "stderr bytes differ for {ctx}");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for {ctx}: C={:?} Rust={:?}",
        c.status.code(),
        r.status.code()
    );
}

/// `CONFIGS.md` rows 1–24, axis 5 level 7: the whole pipeline, over the value
/// shapes of axis 4.
#[test]
fn driver_happy_path_matches() {
    let cases: &[[&str; 2]] = &[
        ["0", "0"],
        ["1", "2"],
        ["2", "1"],
        ["-1", "-2"],
        ["7", "-7"],
        ["100", "3"],
        ["-100", "3"],
        ["46341", "46341"],
        ["65536", "65536"],
        ["2147483647", "1"],
        ["-2147483648", "-1"],
        ["2147483647", "2147483647"],
        ["-2147483648", "-2147483648"],
        ["2147483647", "-2147483648"],
        ["1", "-2147483648"],
        ["32767", "-32768"],
    ];
    for c in cases {
        assert_same(c);
    }
}

/// Randomised (fixed-seed) sweep of the full pipeline.
#[test]
fn driver_randomised_matches() {
    let mut rng = common::Rng::new(common::SEED ^ 0xD00D);
    for _ in 0..60 {
        let a = rng.next_i32_biased().to_string();
        let b = rng.next_i32_biased().to_string();
        assert_same(&[&a, &b]);
    }
}

/// `ERRORS.md` row 18: `argc < 3` ⇒ `usage: <argv[0]> A B` on **stderr**, empty
/// stdout, exit status **2**.
#[test]
fn row_18_too_few_arguments() {
    for args in [&[][..], &["5"][..]] {
        assert_same(args);

        // Pin the exact contract rather than only "both agree".
        let c = run(&c_driver_path(), args);
        assert_eq!(c.status.code(), Some(2), "exit status must be 2 for {args:?}");
        assert!(c.stdout.is_empty(), "stdout must be empty for {args:?}");
        assert_eq!(
            c.stderr, b"usage: driver A B\n",
            "unexpected usage message for {args:?}"
        );

        let r = run(&rust_driver_path(), args);
        assert_eq!(r.status.code(), Some(2));
        assert!(r.stdout.is_empty());
        assert_eq!(r.stderr, b"usage: driver A B\n");
    }
}

/// `ERRORS.md` row 19: non-numeric arguments — `atoi` reports no error and
/// yields `0` (and stops at the first non-digit).
#[test]
fn row_19_non_numeric_arguments() {
    let cases: &[[&str; 2]] = &[
        ["abc", "def"],
        ["", ""],
        ["12abc", "34xyz"],
        ["  42", "\t-7"],
        ["+5", "-5"],
        ["--5", "++5"],
        ["-", "+"],
        [".5", "5."],
        ["0x10", "010"],
        ["5 6", "7 8"],
        ["١٢", "1e3"],
    ];
    for c in cases {
        assert_same(c);
    }
    // And pin the documented outcome for the clearly-zero cases: with A=B=0 the
    // output must be identical to passing literal "0" "0".
    for zeroish in [["abc", "def"], ["", ""], ["-", "+"], [".5", "x"]] {
        let a = run(&c_driver_path(), &zeroish);
        let b = run(&c_driver_path(), &["0", "0"]);
        assert_eq!(
            a.stdout, b.stdout,
            "atoi({:?}) should be 0,0 per ERRORS.md row 19",
            zeroish
        );
        let ar = run(&rust_driver_path(), &zeroish);
        assert_eq!(ar.stdout, b.stdout, "Rust atoi({zeroish:?}) should be 0,0");
    }
}

/// `ERRORS.md` row 20: arguments that overflow `long` — `atoi` is
/// `(int)strtol(...)`, so `strtol` saturates to `LONG_MAX`/`LONG_MIN` and the
/// cast truncates (`LONG_MAX & 0xFFFFFFFF == -1`, `LONG_MIN` ⇒ `0`).
#[test]
fn row_20_overflowing_arguments() {
    let cases: &[[&str; 2]] = &[
        ["99999999999999999999", "1"],
        ["-99999999999999999999", "1"],
        ["9223372036854775807", "0"],       // LONG_MAX exactly
        ["9223372036854775808", "0"],       // LONG_MAX + 1
        ["-9223372036854775808", "0"],      // LONG_MIN exactly
        ["-9223372036854775809", "0"],      // LONG_MIN - 1
        ["4294967296", "4294967297"],       // 2^32, 2^32+1 -> truncate to 0, 1
        ["2147483648", "-2147483649"],      // INT_MAX+1, INT_MIN-1
        [
            "123456789012345678901234567890123456789",
            "-123456789012345678901234567890123456789",
        ],
        ["00000000000000000000000000005", "-000000000000007"],
    ];
    for c in cases {
        assert_same(c);
    }

    // Pin the saturate-then-truncate contract explicitly.
    let big = run(&c_driver_path(), &["99999999999999999999", "0"]);
    let minus1 = run(&c_driver_path(), &["-1", "0"]);
    assert_eq!(
        big.stdout, minus1.stdout,
        "ERRORS.md row 20: (int)LONG_MAX must be -1"
    );
    let negbig = run(&c_driver_path(), &["-99999999999999999999", "0"]);
    let zero = run(&c_driver_path(), &["0", "0"]);
    assert_eq!(
        negbig.stdout, zero.stdout,
        "ERRORS.md row 20: (int)LONG_MIN must be 0"
    );
}

/// `ERRORS.md` row 21: `argc > 3` — extra arguments are silently ignored.
#[test]
fn row_21_extra_arguments_ignored() {
    assert_same(&["3", "4", "5"]);
    assert_same(&["3", "4", "ignored", "also-ignored", "-9"]);

    let two = run(&c_driver_path(), &["3", "4"]);
    let many = run(&c_driver_path(), &["3", "4", "5", "6"]);
    assert_eq!(two.stdout, many.stdout, "extra args must be ignored");
    let two_r = run(&rust_driver_path(), &["3", "4"]);
    let many_r = run(&rust_driver_path(), &["3", "4", "5", "6"]);
    assert_eq!(two_r.stdout, many_r.stdout);
    assert_eq!(two.stdout, two_r.stdout);
}

/// The stdout of the driver encodes the configuration (`op=%s`), so this also
/// double-checks that the Rust build's active feature set really is the
/// configuration the C reference was compiled for — i.e. that the whole
/// per-configuration test run is not silently comparing mismatched builds.
#[test]
fn driver_reports_the_expected_configuration() {
    let out = run(&rust_driver_path(), &["10", "3"]);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains(&format!("op={OP_NAME} ")),
        "Rust driver reported the wrong OP; expected op={OP_NAME} in:\n{s}"
    );
    let c = run(&c_driver_path(), &["10", "3"]);
    assert_eq!(c.stdout, out.stdout, "OP={OP_NAME} REPEAT={REPEAT}");
}

/// The `argc == 0` boundary: a program `execve`'d with an **empty** `argv` array
/// has `argv[0] == NULL`, and `mdmain.c` feeds that straight into
/// `fprintf(stderr, "usage: %s A B\n", argv[0])` — a null pointer crossing into
/// `printf`'s `%s`. This is unreachable through `std::process::Command` (which
/// always supplies an `argv[0]`), so the child re-`execve`s itself from
/// `pre_exec` with `argv = { NULL }`.
///
/// glibc renders it as the empty string here, giving `"usage:  A B\n"` with two
/// spaces; the Rust translation must produce the same bytes and the same exit
/// status, not `"(null)"` and not a panic.
#[test]
fn argc_zero_null_argv0() {
    fn run_with_empty_argv(prog: &Path) -> Output {
        let c_path = CString::new(prog.to_str().unwrap()).unwrap();
        let mut cmd = Command::new(prog);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        // SAFETY: `pre_exec` runs in the forked child between `fork` and `exec`.
        // It only calls `execve`, which is async-signal-safe, and allocates
        // nothing; `c_path` was built before the fork. If `execve` succeeds the
        // closure never returns, so nothing else in the child observes the state.
        unsafe {
            cmd.pre_exec(move || {
                let argv: [*const c_char; 1] = [std::ptr::null()];
                let envp: [*const c_char; 1] = [std::ptr::null()];
                execve(c_path.as_ptr(), argv.as_ptr(), envp.as_ptr());
                Err(std::io::Error::last_os_error())
            });
        }
        cmd.output().expect("spawn with empty argv")
    }

    let c = run_with_empty_argv(&c_driver_path());
    let r = run_with_empty_argv(&rust_driver_path());

    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "argc==0 stderr differs [OP={OP_NAME} REPEAT={REPEAT}]"
    );
    assert_eq!(c.stderr, r.stderr, "argc==0 stderr bytes differ");
    assert_eq!(c.stdout, r.stdout, "argc==0 stdout bytes differ");
    assert_eq!(c.status.code(), r.status.code(), "argc==0 exit status differs");

    // Pin the observed glibc contract so a mutual regression is caught too.
    assert_eq!(c.stderr, b"usage:  A B\n");
    assert!(c.stdout.is_empty());
    assert_eq!(c.status.code(), Some(2));
}
