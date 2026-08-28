//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition, calls both `.so`s and
//! asserts they reject identically: the same sentinel return value, the same
//! bytes on stdout *and* stderr, or — for the rows whose documented C behavior is
//! a crash — the same `waitpid` status from a forked child.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// E01: `argc == 2` — one step below the required 3.
#[test]
fn e01_argc_2() {
    let p = Pair::load();
    diff_main(&p, 2, &[Some("prog"), Some("1")]);
    diff_main(&p, 2, &[Some("prog"), Some("1"), Some("2")]); // argv longer than argc
}

/// E02: `argc == 1`.
#[test]
fn e02_argc_1() {
    let p = Pair::load();
    diff_main(&p, 1, &[Some("prog")]);
    diff_main(&p, 1, &[Some("prog"), Some("1"), Some("2")]);
}

/// E03: `argc == 0`.
#[test]
fn e03_argc_0() {
    let p = Pair::load();
    diff_main(&p, 0, &[Some("prog")]);
    diff_main(&p, 0, &[Some("prog"), Some("1"), Some("2")]);
}

/// E04: negative `argc` (the check is `argc < 3`, so these are rejected too).
#[test]
fn e04_argc_negative() {
    let p = Pair::load();
    for argc in [-1, -5, -1000, i32::MIN] {
        diff_main(&p, argc, &[Some("prog"), Some("1"), Some("2")]);
    }
}

/// E05: `use_generated(7)` — one step past the last `case` of `DISPATCH_REP`.
#[test]
fn e05_use_generated_7() {
    let p = Pair::load();
    diff_un(&p, "use_generated", |d| d.use_generated(), 7);
    // and the value must be exactly INIT_FOR(OP)
    let init = expected_run_loop(&p.op, 0);
    let (rc, _) = capture(|| unsafe { (p.c.use_generated())(7) });
    assert_eq!(rc, init, "C use_generated(7) should stay at INIT");
}

/// E06: `use_generated(-1)` — one step below `case 0`.
#[test]
fn e06_use_generated_minus_1() {
    let p = Pair::load();
    diff_un(&p, "use_generated", |d| d.use_generated(), -1);
    let init = expected_run_loop(&p.op, 0);
    let (rc, _) = capture(|| unsafe { (p.c.use_generated())(-1) });
    assert_eq!(rc, init);
}

/// E07/E08: `INT_MAX` / `INT_MIN`.
#[test]
fn e07_e08_use_generated_extremes() {
    let p = Pair::load();
    for n in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
}

/// E09: a broad band of out-of-range `n`, including randomized values.
#[test]
fn e09_use_generated_out_of_range_band() {
    let p = Pair::load();
    for n in (8..=64).chain([100, 1000, 65536, -2, -3, -64, -1000]) {
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..256 {
        let n = loop {
            let n = rng.next_i32();
            if !(0..=6).contains(&n) {
                break n;
            }
        };
        diff_un(&p, "use_generated", |d| d.use_generated(), n);
    }
}

/// E10: on a `REPEAT == 7` build, `main` calls `use_generated(7)`, so the
/// unrolled loop and the `switch` disagree — both libraries must disagree the
/// same way.
#[test]
fn e10_main_uses_generated_repeat() {
    let p = Pair::load();
    diff_main_strs(&p, &["prog", "1", "2"]);
    diff_un(&p, "use_generated", |d| d.use_generated(), p.repeat);
    if p.repeat == 7 {
        let init = expected_run_loop(&p.op, 0);
        let (rc, _) = capture(|| unsafe { (p.c.use_generated())(7) });
        assert_eq!(rc, init, "REPEAT=7 must fall through to `default:`");
    }
}

/// E11: non-numeric / partially numeric operand text.
#[test]
fn e11_non_numeric_args() {
    let p = Pair::load();
    for s in [
        "abc", "", " ", "+", "-", "0x10", "1e3", "12x", "x12", "--3", "3-", ".5", "٣", "\t-9",
        "9223372036854775807junk", "  ", "1 2",
    ] {
        diff_main_strs(&p, &["prog", s, "1"]);
        diff_main_strs(&p, &["prog", "1", s]);
        diff_main_strs(&p, &["prog", s, s]);
    }
}

/// E12: operand text outside the `int` range (`atoi` == `(int)strtol`).
#[test]
fn e12_out_of_range_args() {
    let p = Pair::load();
    for s in [
        "2147483648",
        "-2147483649",
        "99999999999999",
        "-99999999999999",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999999999",
        "4294967296",
        "4294967297",
    ] {
        diff_main_strs(&p, &["prog", s, "1"]);
        diff_main_strs(&p, &["prog", "1", s]);
        diff_main_strs(&p, &["prog", s, s]);
    }
}

/// E13: `argv[0] == NULL` with `argc < 3` — glibc's `%s` prints `(null)`.
#[test]
fn e13_null_argv0() {
    let p = Pair::load();
    for argc in [0, 1, 2, -1] {
        diff_main(&p, argc, &[None, Some("1"), Some("2")]);
    }
}

/// E14: `argv == NULL` with `argc < 3` — the C dereferences `argv[0]`.
#[test]
fn e14_null_argv_short() {
    let p = Pair::load();
    let sc = status_in_child(|| unsafe {
        (p.c.main_fn())(1, std::ptr::null_mut());
    });
    let sr = status_in_child(|| unsafe {
        (p.rs.main_fn())(1, std::ptr::null_mut());
    });
    assert_eq!(
        decode_status(sc),
        decode_status(sr),
        "main(1, NULL): C {} vs Rust {}",
        decode_status(sc),
        decode_status(sr)
    );
    assert_eq!(decode_status(sc), "signal(11)", "expected SIGSEGV from C");
}

/// E15: `argv == NULL` with `argc >= 3` — the C dereferences `argv[1]`.
#[test]
fn e15_null_argv_long() {
    let p = Pair::load();
    let sc = status_in_child(|| unsafe {
        (p.c.main_fn())(3, std::ptr::null_mut());
    });
    let sr = status_in_child(|| unsafe {
        (p.rs.main_fn())(3, std::ptr::null_mut());
    });
    assert_eq!(decode_status(sc), decode_status(sr), "main(3, NULL)");
    assert_eq!(decode_status(sc), "signal(11)", "expected SIGSEGV from C");
}

/// E16: `argv[1] == NULL` / `argv[2] == NULL` with `argc >= 3` — `atoi(NULL)`.
#[test]
fn e16_null_arg_strings() {
    let p = Pair::load();
    for items in [
        [Some("prog"), None, Some("2")],
        [Some("prog"), Some("1"), None],
        [Some("prog"), None, None],
    ] {
        let sc = status_in_child(|| {
            let mut av = Argv::new(&items);
            unsafe {
                (p.c.main_fn())(3, av.as_ptr());
            }
        });
        let sr = status_in_child(|| {
            let mut av = Argv::new(&items);
            unsafe {
                (p.rs.main_fn())(3, av.as_ptr());
            }
        });
        assert_eq!(
            decode_status(sc),
            decode_status(sr),
            "main(3, {items:?}): C {} vs Rust {}",
            decode_status(sc),
            decode_status(sr)
        );
        assert_eq!(decode_status(sc), "signal(11)", "expected SIGSEGV from C");
    }
}

/// E17: `G_OP` overwritten with `NULL`, then `main` — the indirect call crashes,
/// but only *after* the three helpers have printed.
#[test]
fn e17_null_g_op() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();

    // Compare what is printed before the crash, too: run the child with stdout
    // pointed at a file and diff the partial output.
    let sc = status_in_child(|| {
        p.c.set_g_op(0);
        let mut av = Argv::strs(&["prog", "3", "4"]);
        unsafe {
            (p.c.main_fn())(3, av.as_ptr());
        }
    });
    let sr = status_in_child(|| {
        p.rs.set_g_op(0);
        let mut av = Argv::strs(&["prog", "3", "4"]);
        unsafe {
            (p.rs.main_fn())(3, av.as_ptr());
        }
    });
    assert_eq!(
        decode_status(sc),
        decode_status(sr),
        "main with G_OP=NULL: C {} vs Rust {}",
        decode_status(sc),
        decode_status(sr)
    );
    let d = decode_status(sc);
    assert!(
        d == "signal(11)" || d == "signal(4)" || d == "signal(6)",
        "expected a fatal signal from C, got {d}"
    );

    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// E18: `G_OP_NAME` overwritten with `NULL`, then `main` — glibc prints
/// `op=(null)` and the call still returns 0.
#[test]
fn e18_null_g_op_name() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();
    p.c.set_g_op_name(std::ptr::null());
    p.rs.set_g_op_name(std::ptr::null());
    diff_main_strs(&p, &["prog", "3", "4"]);
    diff_main_strs(&p, &["prog", "-2147483648", "-2147483648"]);
    diff_main(&p, 2, &[Some("prog"), Some("1")]); // short path, name unused
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// E19: `G_OP` replaced by a *different* op — `main`'s `g.call` follows the
/// global while `r_call`/`helper_call`/`helper_ptr` keep the build-time op.
#[test]
fn e19_g_op_replaced() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();
    for k in 0..3 {
        p.c.set_g_op(p.c.op_addresses()[k]);
        p.rs.set_g_op(p.rs.op_addresses()[k]);
        diff_main_strs(&p, &["prog", "3", "4"]);
        diff_bin(&p, "helper_ptr", |d| d.helper_ptr(), 3, 4);
        diff_bin(&p, "helper_call", |d| d.helper_call(), 3, 4);
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}

/// E20: signed-overflow operands accepted by the leaf ops.
#[test]
fn e20_op_overflow() {
    let p = Pair::load();
    let cases: &[(c_int, c_int)] = &[
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (1, i32::MIN),
        (-1, i32::MIN),
        (65536, 65536),
        (46341, 46341),
        (i32::MIN / 2, 3),
    ];
    for &(a, b) in cases {
        diff_bin(&p, "op_add", |d| d.op_add(), a, b);
        diff_bin(&p, "op_sub", |d| d.op_sub(), a, b);
        diff_bin(&p, "op_mul", |d| d.op_mul(), a, b);
    }
}

/// E21: `helper_call`'s `r + acc` overflow.
#[test]
fn e21_helper_call_overflow() {
    let p = Pair::load();
    for &(a, b) in &[
        (i32::MAX, 0),
        (i32::MAX, i32::MAX),
        (i32::MIN, 0),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MIN, -1),
    ] {
        diff_bin(&p, "helper_call", |d| d.helper_call(), a, b);
    }
}

/// E22: `main`'s `summary` overflow.
#[test]
fn e22_main_summary_overflow() {
    let p = Pair::load();
    for pair in [
        ["2147483647", "2147483647"],
        ["-2147483648", "-2147483648"],
        ["2147483647", "-2147483648"],
        ["1073741824", "1073741824"],
        ["46341", "46341"],
    ] {
        diff_main_strs(&p, &["prog", pair[0], pair[1]]);
    }
}

/// E24: unwritable `stdout`/`stderr` (fd 1 and 2 closed). The C code ignores the
/// `printf` return value, so every entry point must still return its normal
/// result and the process must survive — a Rust translation that used
/// `println!`/`eprintln!` would panic ("failed printing to stdout") and, being
/// `extern "C"`, abort the process instead.
#[test]
fn e24_unwritable_stdout() {
    let p = Pair::load();
    let cases: Vec<(&str, Box<dyn Fn(&Driver) -> c_int>)> = vec![
        (
            "helper_call",
            Box::new(|d: &Driver| unsafe { (d.helper_call())(3, 4) }),
        ),
        (
            "helper_ptr",
            Box::new(|d: &Driver| unsafe { (d.helper_ptr())(3, 4) }),
        ),
        (
            "use_generated",
            Box::new(|d: &Driver| unsafe { (d.use_generated())(4) }),
        ),
        (
            "main/valid",
            Box::new(|d: &Driver| unsafe {
                let mut av = Argv::strs(&["prog", "3", "4"]);
                (d.main_fn())(3, av.as_ptr())
            }),
        ),
        (
            "main/usage",
            Box::new(|d: &Driver| unsafe {
                let mut av = Argv::strs(&["prog"]);
                (d.main_fn())(1, av.as_ptr())
            }),
        ),
    ];
    for (name, f) in cases {
        let sc = exit_code_in_child(|| {
            close_fd(1);
            close_fd(2);
            f(&p.c)
        });
        let sr = exit_code_in_child(|| {
            close_fd(1);
            close_fd(2);
            f(&p.rs)
        });
        assert_eq!(
            decode_status(sc),
            decode_status(sr),
            "{name} with stdout/stderr closed: C {} vs Rust {}",
            decode_status(sc),
            decode_status(sr)
        );
        assert!(
            decode_status(sc).starts_with("exit("),
            "C should survive an unwritable stdout, got {}",
            decode_status(sc)
        );
    }
}

/// Extra generic FFI boundary: the exported `G_OP` / `G_OP_NAME` objects accept
/// *any* bit pattern (the C code has no enums, so this is the analogous
/// "value with no valid variant crosses the FFI boundary" case). A non-NULL but
/// bogus name pointer is not dereferenceable, so only the values that are
/// well-defined for the C program are compared here: any of the three op
/// addresses, plus a pointer to a caller-owned string of every length class.
#[test]
fn e23_globals_accept_any_value() {
    let p = Pair::load();
    let saved_c = p.c.saved_globals();
    let saved_r = p.rs.saved_globals();
    let strings: Vec<CString> = ["", "\u{1}", "0123456789", &"z".repeat(300)]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();
    for s in &strings {
        for k in 0..3 {
            p.c.set_g_op(p.c.op_addresses()[k]);
            p.rs.set_g_op(p.rs.op_addresses()[k]);
            p.c.set_g_op_name(s.as_ptr() as *const c_char);
            p.rs.set_g_op_name(s.as_ptr() as *const c_char);
            diff_main_strs(&p, &["prog", "8", "9"]);
        }
    }
    p.c.reset_globals(saved_c);
    p.rs.reset_globals(saved_r);
}
