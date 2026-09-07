//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The C library has no error codes, no `assert`, no null checks and no error
//! enums; every rejection is either the *silent* `default: break;` of
//! `DISPATCH_REP` or the `argc < 3` → `return 2` guard in `mdmain.c`. Each row
//! below constructs that exact condition and asserts C and Rust produce the
//! same sentinel (`INIT_FOR(OP)`), the same exit status, and the same bytes.

mod common;

use common::*;
use std::ffi::c_int;
use std::process::Command;

/// The sentinel every out-of-range `use_generated` must return: `DISPATCH_REP`
/// falls through `default: break;` leaving `acc` at `INIT_FOR(OP)`.
fn sentinel() -> c_int {
    init_for_op()
}

#[track_caller]
fn assert_use_generated_rejects(n: c_int) {
    let bo = both();
    let cf = fn1(&bo.c, "use_generated");
    let rf = fn1(&bo.r, "use_generated");
    let (c, cout) = capture_fd1(|| unsafe { cf(n) });
    let (r, rout) = capture_fd1(|| unsafe { rf(n) });
    assert_eq!(
        c,
        sentinel(),
        "[{}] C use_generated({n}) must return INIT_FOR(OP)={}",
        tag(),
        sentinel()
    );
    assert_eq!(
        r,
        sentinel(),
        "[{}] Rust use_generated({n}) must return INIT_FOR(OP)={}",
        tag(),
        sentinel()
    );
    assert_eq!(c, r, "[{}] use_generated({n}) C/Rust", tag());
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{}] use_generated({n}) stdout",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        format!("gen.acc={}\n", sentinel()),
        "[{}] use_generated({n}) must still print gen.acc",
        tag()
    );
}

/* ---- E1: n == 7 (REP7 exists, but DISPATCH_REP has no `case 7`) ---- */

#[test]
fn e1_use_generated_n_7() {
    assert_use_generated_rejects(7);
}

/* ---- E2: n == 8, first value past the last case ---- */

#[test]
fn e2_use_generated_n_8() {
    assert_use_generated_rejects(8);
}

/* ---- E3: large positives ---- */

#[test]
fn e3_use_generated_large_positive() {
    for n in [9, 10, 11, 16, 32, 100, 1000, 65_536, 1_000_000] {
        assert_use_generated_rejects(n);
    }
}

/* ---- E4: n == INT_MAX ---- */

#[test]
fn e4_use_generated_int_max() {
    assert_use_generated_rejects(c_int::MAX);
    assert_use_generated_rejects(c_int::MAX - 1);
}

/* ---- E5: n == -1, one step below `case 0` ---- */

#[test]
fn e5_use_generated_minus_one() {
    assert_use_generated_rejects(-1);
}

/* ---- E6: other negatives ---- */

#[test]
fn e6_use_generated_negatives() {
    for n in [-2, -3, -6, -7, -8, -100, -65_536, -1_000_000] {
        assert_use_generated_rejects(n);
    }
}

/* ---- E7: n == INT_MIN ---- */

#[test]
fn e7_use_generated_int_min() {
    assert_use_generated_rejects(c_int::MIN);
    assert_use_generated_rejects(c_int::MIN + 1);
}

/* ---- E8: out-of-range "enum-like" selector across the FFI boundary ---- */

/// `use_generated` takes a plain `int` and feeds it to a `switch` with only
/// `case 0..6`. A C `enum` accepts any `int`, so the analogous "value with no
/// valid variant" input here is any `n` outside `0..=6`. Every value in
/// `-64..=64` plus 4096 randomized draws over the whole `i32` range.
#[test]
fn e8_use_generated_out_of_range_selector_exhaustive_and_random() {
    let bo = both();
    let cf = fn1(&bo.c, "use_generated");
    let rf = fn1(&bo.r, "use_generated");
    let s = sentinel();

    let mut ns: Vec<c_int> = (-64..=64).collect();
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..4096 {
        ns.push(rng.next_i32());
    }

    // Batch the fd-1 capture: one redirect per library for the whole sweep.
    // C and Rust must be captured separately, not interleaved: glibc's `stdout`
    // is block-buffered onto fd 1 while the Rust `.so`'s `LineWriter` flushes
    // every line, so an interleaved capture would not preserve call order.
    let (cvals, cout) = capture_fd1(|| ns.iter().map(|&n| unsafe { cf(n) }).collect::<Vec<_>>());
    let (rvals, rout) = capture_fd1(|| ns.iter().map(|&n| unsafe { rf(n) }).collect::<Vec<_>>());

    let mut expected_out = String::new();
    for (idx, &n) in ns.iter().enumerate() {
        let (c, r) = (cvals[idx], rvals[idx]);
        assert_eq!(c, r, "[{}] use_generated({n}) C={c} Rust={r}", tag());
        let want = if (0..=6).contains(&n) {
            let mut acc = init_for_op();
            for i in 0..n {
                acc = step(acc, i);
            }
            acc
        } else {
            s
        };
        assert_eq!(c, want, "[{}] use_generated({n})", tag());
        expected_out.push_str(&format!("gen.acc={want}\n"));
    }
    assert_eq!(
        String::from_utf8_lossy(&cout),
        expected_out,
        "[{}] C gen.acc output over the sweep",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&rout),
        expected_out,
        "[{}] Rust gen.acc output over the sweep",
        tag()
    );
}

/* ---- E9 / E10 / E11: the `argc < 3` guard in mdmain.c ---- */

#[test]
fn e9_driver_argc_1_no_operands() {
    let a = artifacts();
    let c = run_driver(&a.c_bin_dir, &[]);
    let r = run_driver(&a.r_bin_dir, &[]);
    assert_eq!(c.code, Some(2), "[{}] C exit status must be 2", tag());
    assert_eq!(r.code, Some(2), "[{}] Rust exit status must be 2", tag());
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        "usage: ./driver A B\n",
        "[{}] C usage message",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{}] usage message",
        tag()
    );
    assert!(c.stdout.is_empty(), "[{}] C must print nothing", tag());
    assert!(r.stdout.is_empty(), "[{}] Rust must print nothing", tag());
}

#[test]
fn e10_driver_argc_2_one_operand() {
    for args in [vec!["5"], vec![""], vec!["abc"], vec!["-2147483648"]] {
        let a = artifacts();
        let c = run_driver(&a.c_bin_dir, &args);
        let r = run_driver(&a.r_bin_dir, &args);
        assert_eq!(c.code, Some(2), "[{}] C {args:?}", tag());
        assert_eq!(r.code, Some(2), "[{}] Rust {args:?}", tag());
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            "usage: ./driver A B\n",
            "[{}] C usage {args:?}",
            tag()
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "[{}] usage {args:?}",
            tag()
        );
        assert_eq!(c.stdout, r.stdout, "[{}] stdout {args:?}", tag());
    }
}

/// `execve(prog, {NULL}, {NULL})`: `argc == 0`, so `argv[0]` is not a usable
/// program name. Both implementations must render the same thing (measured on
/// this platform: the empty string, not `(null)`).
#[test]
fn e11_driver_argc_0() {
    let a = artifacts();
    let mut outs = Vec::new();
    for dir in [&a.c_bin_dir, &a.r_bin_dir] {
        let out = Command::new(&a.argc0)
            .arg(dir.join("driver"))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "[{}] argc==0 exit status", tag());
        outs.push((
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    assert_eq!(outs[0], outs[1], "[{}] argc==0 output", tag());
    assert!(
        outs[0].1.starts_with("usage:") && outs[0].1.ends_with(" A B\n"),
        "[{}] argc==0 stderr shape: {:?}",
        tag(),
        outs[0].1
    );

    // Also `argc == 1` with an empty argv[0], which is reachable without execve.
    use std::os::unix::process::CommandExt;
    let mut outs = Vec::new();
    for dir in [&a.c_bin_dir, &a.r_bin_dir] {
        let out = Command::new(dir.join("driver")).arg0("").output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        outs.push(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    assert_eq!(outs[0], "usage:  A B\n", "[{}] empty argv[0]", tag());
    assert_eq!(outs[0], outs[1], "[{}] empty argv[0]", tag());
}

/* ---- E12 - E17: `atoi` silent-failure surface ---- */

#[track_caller]
fn assert_driver_pair_matches(a_arg: &str, b_arg: &str) {
    assert_driver_matches(&[a_arg, b_arg]);
}

/// E12 — nothing convertible: `strtol` converts no characters, `atoi` yields 0
/// and reports nothing. The program continues and exits 0.
#[test]
fn e12_atoi_non_numeric() {
    let a = artifacts();
    // Each entry: the unconvertible text, and the numeric literal `atoi` must
    // be equivalent to. Only the FIRST operand is varied so the equivalence is
    // unambiguous; the second is held at a fixed valid value.
    for text in [
        "abc", "def", "--5", "+", "-", "x1", ".5", ",7", "", "++1", "nan", "inf", "  ", "\t",
        "e5", "０", "-+3",
    ] {
        assert_driver_pair_matches(text, "3");
        let got = run_driver(&a.c_bin_dir, &[text, "3"]);
        let want = run_driver(&a.c_bin_dir, &["0", "3"]);
        assert_eq!(
            got.code,
            Some(0),
            "[{}] {text:?} must not be an error",
            tag()
        );
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            String::from_utf8_lossy(&want.stdout),
            "[{}] atoi({text:?}) must yield 0",
            tag()
        );
    }
    // And with both operands unconvertible: identical to `0 0`.
    for (x, y) in [("abc", "def"), ("--5", "++5"), ("+", "-"), ("", "")] {
        assert_driver_pair_matches(x, y);
        let got = run_driver(&a.c_bin_dir, &[x, y]);
        let zero_zero = run_driver(&a.c_bin_dir, &["0", "0"]);
        assert_eq!(
            String::from_utf8_lossy(&got.stdout),
            String::from_utf8_lossy(&zero_zero.stdout),
            "[{}] {x:?} {y:?} must behave exactly like 0 0",
            tag()
        );
    }
}

/// E13 — numeric prefix then garbage; conversion stops at the first non-digit,
/// base 10 (so `0x10` is `0` and `010` is ten, not eight).
#[test]
fn e13_atoi_numeric_prefix() {
    let a = artifacts();
    for (text, value) in [
        ("12x", "12"),
        ("34y", "34"),
        ("0x10", "0"),
        ("010", "10"),
        ("9e9", "9"),
        ("-5-6", "-5"),
        ("7 8", "7"),
    ] {
        assert_driver_pair_matches(text, "1");
        let got = run_driver(&a.c_bin_dir, &[text, "1"]);
        let want = run_driver(&a.c_bin_dir, &[value, "1"]);
        assert_eq!(
            got.stdout, want.stdout,
            "[{}] atoi({text:?}) must equal {value}",
            tag()
        );
    }
}

/// E14 / E15 — overflowing `long`: glibc `atoi` is `(int)strtol(...)`, and
/// `strtol` saturates at `LONG_MAX`/`LONG_MIN` (the `ERANGE` is discarded), so
/// the visible results are `(int)LONG_MAX == -1` and `(int)LONG_MIN == 0`.
#[test]
fn e14_e15_atoi_long_overflow_saturates() {
    let a = artifacts();
    for (text, equivalent) in [
        ("9999999999999999999999", "-1"),
        ("99999999999999999999999999999999", "-1"),
        ("9223372036854775808", "-1"),
        ("-9999999999999999999999", "0"),
        ("-9223372036854775809", "0"),
    ] {
        assert_driver_pair_matches(text, "0");
        let got = run_driver(&a.c_bin_dir, &[text, "0"]);
        let want = run_driver(&a.c_bin_dir, &[equivalent, "0"]);
        assert_eq!(
            got.stdout, want.stdout,
            "[{}] atoi({text:?}) must equal {equivalent}",
            tag()
        );
    }
}

/// E16 — fits in `long` but not in `int`: silent truncation.
#[test]
fn e16_atoi_int_truncation() {
    let a = artifacts();
    for (text, equivalent) in [
        ("2147483648", "-2147483648"),
        ("-2147483649", "2147483647"),
        ("4294967296", "0"),
        ("4294967297", "1"),
        ("-4294967296", "0"),
        ("9223372036854775807", "-1"),
    ] {
        assert_driver_pair_matches(text, "0");
        let got = run_driver(&a.c_bin_dir, &[text, "0"]);
        let want = run_driver(&a.c_bin_dir, &[equivalent, "0"]);
        assert_eq!(
            got.stdout, want.stdout,
            "[{}] atoi({text:?}) must truncate to {equivalent}",
            tag()
        );
    }
}

/// E17 — leading whitespace and explicit sign forms.
#[test]
fn e17_atoi_whitespace_and_sign() {
    for (x, y) in [
        ("  12", "\t-3"),
        ("\n\r\x0b\x0c 5", " +6"),
        ("+0", "-0"),
        ("   -0007", "+0007"),
        (" 2147483647", " -2147483648"),
    ] {
        assert_driver_pair_matches(x, y);
    }
}

/* ---- E18 - E21: signed-overflow boundaries (no check exists in C) ---- */

#[test]
fn e18_e19_e20_op_overflow_wraps_identically() {
    for name in ["op_add", "op_sub", "op_mul"] {
        for (a, b) in overflow_pairs().into_iter().chain(boundary_pairs()) {
            assert_fn2_matches(name, a, b);
        }
    }
}

#[test]
fn e21_helper_call_return_overflow() {
    // `return r + acc` with r at the int boundary and acc = REP<REPEAT>.
    for (a, b) in overflow_pairs().into_iter().chain(boundary_pairs()) {
        assert_fn2_matches("helper_call", a, b);
        assert_fn2_matches("helper_ptr", a, b);
        assert_g_op_matches(a, b);
    }
}

/* ---- E22: the exported globals are writable, and never NULL ---- */

#[test]
fn e22_exported_globals_are_writable_and_non_null() {
    let bo = both();
    for (which, lib) in [("C", &bo.c), ("Rust", &bo.r)] {
        // Neither slot address may be NULL, and neither stored value may be NULL:
        // the only pointers in this API are these two, so this is the whole
        // null-pointer surface.
        let gslot = g_op_slot(lib);
        let nslot = g_op_name_slot(lib);
        assert!(!gslot.is_null(), "{which}: G_OP slot address");
        assert!(!nslot.is_null(), "{which}: G_OP_NAME slot address");
        assert!(
            !unsafe { *nslot }.is_null(),
            "{which}: G_OP_NAME value must not be NULL"
        );

        // The store must succeed rather than trap (C `.data`, not RELRO).
        let g0 = unsafe { *gslot };
        let n0 = unsafe { *nslot };
        unsafe {
            *gslot = fn2(lib, "op_sub");
            *nslot = n0.add(1);
        }
        assert_eq!(unsafe { *gslot } as usize, fn2(lib, "op_sub") as usize);
        assert_eq!(unsafe { *nslot }, unsafe { n0.add(1) });
        unsafe {
            *gslot = g0;
            *nslot = n0;
        }
        assert_eq!(unsafe { *gslot } as usize, g0 as usize);
        assert_eq!(unsafe { *nslot }, n0);
    }
}

/* ---- Generic FFI boundaries not tied to a specific row ---- */

/// Every exported symbol must be resolvable in both libraries — a `dlsym`
/// failure is itself an error-path divergence.
#[test]
fn generic_all_symbols_resolve_in_both() {
    let bo = both();
    for lib in [&bo.c, &bo.r] {
        for name in PUBLIC_FNS_2 {
            let _ = fn2(lib, name);
        }
        let _ = fn1(lib, "use_generated");
        let _ = g_op_slot(lib);
        let _ = g_op_name_slot(lib);
    }
    // `accum_<OP>` is `static` in the C, so it must NOT be exported by either.
    for lib in [&bo.c, &bo.r] {
        for name in ["accum_add", "accum_sub", "accum_mul", "main", "accum"] {
            let mut s = name.as_bytes().to_vec();
            s.push(0);
            let r: Result<libloading::Symbol<Op1>, _> = unsafe { lib.get(&s) };
            assert!(
                r.is_err(),
                "[{}] {name} must not be a dynamic symbol",
                tag()
            );
        }
    }
}

/// Repeated calls must be pure: neither implementation may accumulate state
/// between invocations (the C `acc` is a local, re-initialised every call).
#[test]
fn generic_calls_are_stateless() {
    let bo = both();
    for name in ["helper_call", "helper_ptr"] {
        let cf = fn2(&bo.c, name);
        let rf = fn2(&bo.r, name);
        let (first, _) = capture_fd1(|| (unsafe { cf(3, 4) }, unsafe { rf(3, 4) }));
        for _ in 0..8 {
            let (again, _) = capture_fd1(|| (unsafe { cf(3, 4) }, unsafe { rf(3, 4) }));
            assert_eq!(again, first, "[{}] {name} must be stateless", tag());
        }
    }
    let cg = fn1(&bo.c, "use_generated");
    let rg = fn1(&bo.r, "use_generated");
    for n in [0, 3, 6, 7, -1] {
        let (first, _) = capture_fd1(|| (unsafe { cg(n) }, unsafe { rg(n) }));
        for _ in 0..8 {
            let (again, _) = capture_fd1(|| (unsafe { cg(n) }, unsafe { rg(n) }));
            assert_eq!(again, first, "[{}] use_generated({n}) stateless", tag());
        }
    }
}
