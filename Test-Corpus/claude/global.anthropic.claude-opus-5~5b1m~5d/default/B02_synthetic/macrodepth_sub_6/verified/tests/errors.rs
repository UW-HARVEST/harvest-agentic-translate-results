//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`, comparing the C `.so` and Rust `.so` through
//! `libloading` (rows 16-20 compare the two `driver` executables instead, since
//! the trigger is `main`'s argument handling).

mod common;

use common::*;
use std::ffi::c_int;
use std::process::Command;

// ---------------------------------------------------------------------------
// Rows 1-6: DISPATCH_REP's `default:` arm (the library's only rejection path)
// ---------------------------------------------------------------------------

/// Row 1 — `n == 7`, the first value past `case 6`.
#[test]
fn err_01_use_generated_n_eq_7() {
    diff_un("use_generated", 7);
    // And confirm the C really falls through to `INIT`.
    let (c, _) = pair();
    let f = c.un("use_generated");
    // SAFETY: matches `int use_generated(int)`.
    let (v, out) = capture_stdout(|| unsafe { f(7) });
    assert_eq!(v, INIT, "C use_generated(7) should return INIT_FOR({OP})");
    assert_eq!(String::from_utf8_lossy(&out), format!("gen.acc={INIT}\n"));
}

/// Row 2 — `n > 7`.
#[test]
fn err_02_use_generated_n_gt_7() {
    for n in [8, 9, 10, 100, 1000, 65536, c_int::MAX - 1, c_int::MAX] {
        diff_un("use_generated", n);
    }
}

/// Row 3 — `n == -1`, the first value below `case 0`.
#[test]
fn err_03_use_generated_n_neg_1() {
    diff_un("use_generated", -1);
    let (c, _) = pair();
    let f = c.un("use_generated");
    // SAFETY: matches `int use_generated(int)`.
    let (v, _out) = capture_stdout(|| unsafe { f(-1) });
    assert_eq!(v, INIT, "C use_generated(-1) should return INIT_FOR({OP})");
}

/// Row 4 — deeply negative `n`.
#[test]
fn err_04_use_generated_n_very_negative() {
    for n in [-2, -3, -100, -65536, c_int::MIN + 1, c_int::MIN] {
        diff_un("use_generated", n);
    }
}

/// Row 5 — the in-range values must NOT be rejected (boundary check both ways).
#[test]
fn err_05_use_generated_in_range_not_rejected() {
    let (c, _) = pair();
    let f = c.un("use_generated");
    for n in 0..=6 {
        diff_un("use_generated", n);
        // SAFETY: matches `int use_generated(int)`.
        let (v, _) = capture_stdout(|| unsafe { f(n) });
        let mut acc = INIT;
        let mut i: c_int = 0;
        while i < n {
            acc = step(acc, i);
            i += 1;
        }
        assert_eq!(v, acc, "C use_generated({n}) unexpected for OP={OP}");
    }
}

/// Row 6 — out-of-range "enum-like" `int` fuzzed across the FFI boundary.
#[test]
fn err_06_use_generated_ffi_fuzz_all_int() {
    let (c, r) = pair();
    let cf = c.un("use_generated");
    let rf = r.un("use_generated");
    let mut rng = Rng::new(SEED ^ 0x1234);
    let mut ns: Vec<c_int> = vec![c_int::MIN, c_int::MIN + 1, c_int::MAX, c_int::MAX - 1, -1, 7];
    for _ in 0..ITERS {
        ns.push(rng.next_int());
    }
    let (cv, cout) = capture_stdout(|| {
        // SAFETY: matches `int use_generated(int)`.
        ns.iter().map(|&n| unsafe { cf(n) }).collect::<Vec<_>>()
    });
    let (rv, rout) = capture_stdout(|| {
        // SAFETY: matches `int use_generated(int)`.
        ns.iter().map(|&n| unsafe { rf(n) }).collect::<Vec<_>>()
    });
    for (i, (&cval, &rval)) in cv.iter().zip(rv.iter()).enumerate() {
        assert_eq!(cval, rval, "use_generated({}) mismatch", ns[i]);
    }
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "fuzz stdout mismatch"
    );
    // Anything outside 0..=6 must yield INIT in both.
    for (i, &n) in ns.iter().enumerate() {
        if !(0..=6).contains(&n) {
            assert_eq!(cv[i], INIT, "C use_generated({n}) should be INIT");
            assert_eq!(rv[i], INIT, "Rust use_generated({n}) should be INIT");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-12: unchecked signed overflow (C has no guard; must wrap identically)
// ---------------------------------------------------------------------------

/// Row 7 — `op_add` overflow.
#[test]
fn err_07_op_add_overflow() {
    let cases = [
        (c_int::MAX, 1),
        (1, c_int::MAX),
        (c_int::MIN, -1),
        (-1, c_int::MIN),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
    ];
    for (a, b) in cases {
        diff_bin("op_add", a, b);
    }
}

/// Row 8 — `op_sub` overflow.
#[test]
fn err_08_op_sub_overflow() {
    let cases = [
        (c_int::MIN, 1),
        (c_int::MIN, c_int::MAX),
        (c_int::MAX, c_int::MIN),
        (0, c_int::MIN),
        (-1, c_int::MAX),
    ];
    for (a, b) in cases {
        diff_bin("op_sub", a, b);
    }
}

/// Row 9 — `op_mul` overflow.
#[test]
fn err_09_op_mul_overflow() {
    let cases = [
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, -1),
        (-1, c_int::MIN),
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, 2),
        (65536, 65536),
        (c_int::MAX, 2),
    ];
    for (a, b) in cases {
        diff_bin("op_mul", a, b);
    }
}

/// Row 10 — `helper_call`: overflow in the op *and* in `r + acc`.
#[test]
fn err_10_helper_call_overflow() {
    for (a, b) in bounds_grid() {
        diff_bin("helper_call", a, b);
    }
}

/// Row 11 — `helper_ptr`: overflow inside the indirect call.
#[test]
fn err_11_helper_ptr_overflow() {
    for (a, b) in bounds_grid() {
        diff_bin("helper_ptr", a, b);
    }
}

/// Row 12 — accumulator overflow inside the unrolled `STEP_mul` chain.
///
/// Only reachable as an actual overflow when `OP=mul`, but the test is valid (and
/// run) in every configuration: it simply exercises the accumulator chain at
/// every `n` and asserts C == Rust.
#[test]
fn err_12_mul_accumulator_overflow() {
    for n in 0..=7 {
        diff_un("use_generated", n);
    }
    // `helper_call`'s unrolled chain (fixed at REPEAT) with extreme `a`/`b`.
    for (a, b) in [
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, c_int::MIN),
    ] {
        diff_bin("helper_call", a, b);
    }
}

// ---------------------------------------------------------------------------
// Rows 13-15: the writable globals
// ---------------------------------------------------------------------------

/// Row 13 — `helper_ptr` uses `OP_FN(OP)` directly, so writing `G_OP` must not
/// change its behaviour. Verified identically for C and Rust.
#[test]
fn err_13_g_op_write_does_not_affect_helper_ptr() {
    let (c, r) = pair();

    let probe = |l: &Loaded| -> (c_int, Vec<u8>) {
        let hp = l.bin("helper_ptr");
        // SAFETY: matches `int helper_ptr(int,int)`.
        capture_stdout(|| unsafe { hp(6, 7) })
    };

    let before_c = probe(c);
    let before_r = probe(r);
    assert_eq!(before_c.0, before_r.0, "helper_ptr baseline return mismatch");
    assert_eq!(
        String::from_utf8_lossy(&before_c.1),
        String::from_utf8_lossy(&before_r.1),
        "helper_ptr baseline stdout mismatch"
    );

    // Pick a *different* op to install into G_OP.
    let victim = if OP == "sub" { "op_mul" } else { "op_sub" };
    let saved_c = c.g_op();
    let saved_r = r.g_op();
    // SAFETY: `G_OP` is a writable global in both objects; the harness holds the
    // capture lock inside `probe`, and no other thread touches these libraries.
    unsafe {
        c.set_g_op(*c.bin(victim));
        r.set_g_op(*r.bin(victim));
    }

    let after_c = probe(c);
    let after_r = probe(r);

    // Restore before asserting, so a failure cannot poison other tests.
    // SAFETY: as above.
    unsafe {
        c.set_g_op(saved_c);
        r.set_g_op(saved_r);
    }

    assert_eq!(
        after_c.0, before_c.0,
        "C helper_ptr changed after writing G_OP — it must use OP_FN(OP) directly"
    );
    assert_eq!(
        after_r.0, after_c.0,
        "Rust helper_ptr diverges from C after G_OP was overwritten"
    );
    assert_eq!(
        String::from_utf8_lossy(&after_c.1),
        String::from_utf8_lossy(&after_r.1),
        "helper_ptr stdout diverges after G_OP was overwritten"
    );

    // The write itself must be observable through G_OP in both.
    assert_eq!(c.g_op_value(), saved_c as usize, "C G_OP not restored");
    assert_eq!(r.g_op_value(), saved_r as usize, "Rust G_OP not restored");
}

/// Row 14 — call through the `G_OP` pointer with overflowing arguments.
#[test]
fn err_14_g_op_pointer_overflow() {
    let (c, r) = pair();
    for (a, b) in bounds_grid() {
        diff_g_op(a, b);
        // and identical to calling op_<OP> directly, in both objects
        let direct = format!("op_{OP}");
        let cd = c.bin(&direct);
        let rd = r.bin(&direct);
        let cg = c.g_op();
        let rg = r.g_op();
        // SAFETY: all four have the signature `int f(int,int)`.
        unsafe {
            assert_eq!(cd(a, b), cg(a, b), "C: G_OP != {direct} for ({a},{b})");
            assert_eq!(rd(a, b), rg(a, b), "Rust: G_OP != {direct} for ({a},{b})");
        }
    }
}

/// Row 15 — `G_OP_NAME` is a valid non-NULL C string equal to `STR(OP)`.
#[test]
fn err_15_g_op_name_string() {
    let (c, r) = pair();
    let cn = c.g_op_name();
    let rn = r.g_op_name();
    assert_eq!(cn, rn, "G_OP_NAME bytes differ");
    assert_eq!(cn.len(), 3, "G_OP_NAME should be 3 bytes + NUL");
    assert_eq!(String::from_utf8_lossy(&cn), OP);
}

// ---------------------------------------------------------------------------
// Rows 16-20: `main`'s argument handling (the only explicit error return)
// ---------------------------------------------------------------------------

struct Run {
    stdout: String,
    stderr_shape: String,
    status: Option<i32>,
}

/// Runs an executable and normalises `stderr` so the (necessarily different)
/// `argv[0]` path does not cause a false mismatch — the *shape* of the usage
/// message and the exit status are what the C defines.
fn run_bin(path: &std::path::Path, args: &[&str]) -> Run {
    let out = Command::new(path)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", path.display()));
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let shape = if stderr.is_empty() {
        String::new()
    } else {
        // "usage: <argv0> A B\n" -> "usage: <ARGV0> A B\n"
        let mut s = stderr.clone();
        if let (Some(i), Some(j)) = (s.find("usage: "), s.rfind(" A B\n")) {
            s.replace_range(i + 7..j, "<ARGV0>");
        }
        s
    };
    Run {
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr_shape: shape,
        status: out.status.code(),
    }
}

fn diff_bin_run(args: &[&str]) {
    let c = run_bin(&c_bin_path(), args);
    let r = run_bin(&rust_bin_path(), args);
    assert_eq!(c.stdout, r.stdout, "driver {args:?}: stdout mismatch");
    assert_eq!(
        c.stderr_shape, r.stderr_shape,
        "driver {args:?}: stderr mismatch"
    );
    assert_eq!(c.status, r.status, "driver {args:?}: exit status mismatch");
}

/// Row 16 — `argc < 3`: no arguments.
#[test]
fn err_16_main_no_args() {
    diff_bin_run(&[]);
    let c = run_bin(&c_bin_path(), &[]);
    assert_eq!(c.status, Some(2), "C driver with no args must exit 2");
    assert!(c.stdout.is_empty(), "C driver must print nothing to stdout");
    assert!(
        c.stderr_shape.starts_with("usage: "),
        "C driver must print usage to stderr; got {:?}",
        c.stderr_shape
    );
}

/// Row 17 — `argc < 3`: one argument.
#[test]
fn err_17_main_one_arg() {
    diff_bin_run(&["1"]);
    let c = run_bin(&c_bin_path(), &["1"]);
    assert_eq!(c.status, Some(2), "C driver with one arg must exit 2");
}

/// Row 18 — `argc > 3`: extra args are ignored (no upper-bound check).
#[test]
fn err_18_main_extra_args_ignored() {
    diff_bin_run(&["3", "4", "5"]);
    diff_bin_run(&["3", "4", "5", "6", "7"]);
    let three = run_bin(&c_bin_path(), &["3", "4"]);
    let five = run_bin(&c_bin_path(), &["3", "4", "5", "6", "7"]);
    assert_eq!(three.stdout, five.stdout, "C must ignore extra argv entries");
    assert_eq!(five.status, Some(0));
}

/// Row 19 — un-parsable `atoi` input.
#[test]
fn err_19_main_atoi_unparsable() {
    for args in [
        ["", ""],
        ["abc", "def"],
        ["12abc", "34xyz"],
        [" 7 ", " -8 "],
        ["+5", "-0"],
        ["0x10", "010"],
        ["007", "-007"],
        [".5", "5."],
        ["--3", "++3"],
        ["\t9", "\n9"],
        ["2 3", "4,5"],
        ["1e3", "-1e3"],
    ] {
        diff_bin_run(&[args[0], args[1]]);
    }
}

/// Row 20 — numeric text that overflows `int` / `long`.
#[test]
fn err_20_main_atoi_overflow() {
    for args in [
        ["2147483647", "-2147483648"],
        ["2147483648", "-2147483649"],
        ["4294967296", "-4294967296"],
        ["9223372036854775807", "-9223372036854775808"],
        ["9223372036854775808", "-9223372036854775809"],
        ["99999999999999999999", "-99999999999999999999"],
        [
            "1000000000000000000000000000000000",
            "-1000000000000000000000000000000000",
        ],
    ] {
        diff_bin_run(&[args[0], args[1]]);
    }
}
