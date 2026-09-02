// Phase C -- error/rejection-path differential tests, gated on ERRORS.md.
//
// `c_src` contains no `assert`, no NULL check, no `errno` use, no error enum
// and no negative sentinel return (verified by grep -- see ERRORS.md). The
// rejection surface is therefore: the `argc < 3` usage path in `mdmain.c`, the
// `default:` arm of `DISPATCH_REP`, `atoi`'s silent rejections, signed-overflow
// wrap-around, and the build-time macro range limits. Every one of those has a
// row in ERRORS.md and a test here.

#[path = "support/mod.rs"]
mod support;

use std::ffi::c_int;
use std::process::Command;
use support::*;

/* ---- E-02 .. E-07: DISPATCH_REP `default:` -----------------------------
 * mdmacros.h:82-93 has `case 0:` .. `case 6:` and `default: break;`. Any `n`
 * outside 0..=6 leaves `acc` at `INIT_FOR(OP)`. `use_generated` then prints
 * `gen.acc=<INIT>` and returns it. This is the out-of-range-dispatch class:
 * the C `switch` accepts any `int`, so a value with no matching case is a real
 * input that must be handled identically -- not a compile-time impossibility.
 * -------------------------------------------------------------------- */

#[test]
fn e02_to_e07_dispatch_default_arm() {
    for &n in DISPATCH_OUT_OF_RANGE.iter() {
        diff_unary("use_generated", n);
        // And the documented value: `default:` leaves acc == INIT_FOR(OP).
        let p = pair();
        let (cr, _) = with_stdout_capture(|| unsafe { (p.c.unary("use_generated"))(n) });
        assert_eq!(
            cr, INIT,
            "C use_generated({n}) should fall through to `default:` and return INIT_FOR({OP})={INIT}"
        );
    }
}

/// One step past each end of the valid `case` range, explicitly.
#[test]
fn e02_e04_one_past_each_end_of_switch_range() {
    let p = pair();
    for &(n, in_range) in &[(-1, false), (0, true), (6, true), (7, false)] {
        diff_unary("use_generated", n);
        let (cr, _) = with_stdout_capture(|| unsafe { (p.c.unary("use_generated"))(n) });
        let (rr, _) = with_stdout_capture(|| unsafe { (p.rust.unary("use_generated"))(n) });
        assert_eq!(cr, rr);
        if !in_range {
            assert_eq!(cr, INIT, "n={n} must hit `default:`");
        }
    }
}

/// The full int domain sampled aggressively: nothing may panic, abort, or
/// diverge for any `int` the caller can pass across the FFI boundary.
#[test]
fn e05_e06_extreme_dispatch_values() {
    let mut rng = Rng::new(0xDEAD_0BAD);
    for &n in &[i32::MIN, i32::MIN + 1, -1, 0, 6, 7, i32::MAX - 1, i32::MAX] {
        diff_unary("use_generated", n);
    }
    for _ in 0..2048 {
        let n = rng.next_i32();
        let p = pair();
        let (cr, cout) = with_stdout_capture(|| unsafe { (p.c.unary("use_generated"))(n) });
        let (rr, rout) = with_stdout_capture(|| unsafe { (p.rust.unary("use_generated"))(n) });
        assert_eq!(cr, rr, "use_generated({n})");
        assert_eq!(cout, rout, "use_generated({n}) stdout");
    }
}

/* ---- E-12 .. E-15: signed overflow ------------------------------------
 * `a + b`, `a - b`, `a * b` and `r + acc` are UB in ISO C on overflow; gcc
 * -O2 emits plain add/sub/imul, so the observable behaviour is two's-complement
 * wrap-around. The Rust side uses wrapping_* and must agree bit for bit.
 * -------------------------------------------------------------------- */

#[test]
fn e12_op_add_overflow_wraps() {
    for &(a, b) in &[
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (1, i32::MAX),
        (-1, i32::MIN),
    ] {
        let p = pair();
        let cr = unsafe { (p.c.op("op_add"))(a, b) };
        let rr = unsafe { (p.rust.op("op_add"))(a, b) };
        assert_eq!(cr, rr, "op_add({a},{b})");
        assert_eq!(cr, a.wrapping_add(b), "C op_add({a},{b}) did not wrap");
    }
}

#[test]
fn e13_op_sub_overflow_wraps() {
    for &(a, b) in &[
        (i32::MIN, 1),
        (i32::MIN, i32::MAX),
        (i32::MAX, -1),
        (i32::MAX, i32::MIN),
        (0, i32::MIN),
    ] {
        let p = pair();
        let cr = unsafe { (p.c.op("op_sub"))(a, b) };
        let rr = unsafe { (p.rust.op("op_sub"))(a, b) };
        assert_eq!(cr, rr, "op_sub({a},{b})");
        assert_eq!(cr, a.wrapping_sub(b), "C op_sub({a},{b}) did not wrap");
    }
}

#[test]
fn e14_op_mul_overflow_wraps() {
    for &(a, b) in &[
        (i32::MIN, -1),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (46341, 46341),
        (65536, 65536),
        (-46341, 46341),
        (i32::MAX, 2),
    ] {
        let p = pair();
        let cr = unsafe { (p.c.op("op_mul"))(a, b) };
        let rr = unsafe { (p.rust.op("op_mul"))(a, b) };
        assert_eq!(cr, rr, "op_mul({a},{b})");
        assert_eq!(cr, a.wrapping_mul(b), "C op_mul({a},{b}) did not wrap");
    }
}

#[test]
fn e15_helper_call_return_overflow_wraps() {
    // `return r + acc;` -- with a = b = INT_MAX and OP=add, r already wrapped
    // and the accumulator pushes it further.
    for &(a, b) in &[
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MIN),
    ] {
        diff_op("helper_call", a, b);
        diff_op("helper_ptr", a, b);
    }
}

/* ---- E-11: mutable exported globals ---------------------------------- */

/// `int (*G_OP)(int,int)` and `const char *G_OP_NAME` are mutable objects with
/// external linkage. A store through the exported address must succeed in both
/// libraries (i.e. the object must live in writable `.data`, *outside*
/// `PT_GNU_RELRO`). A plain Rust `static` would be mprotect'ed read-only and
/// this test would SIGSEGV.
#[test]
fn e11_g_op_is_writable_data() {
    let p = pair();
    for imp in [&p.c, &p.rust] {
        let slot = imp.g_op_slot();
        let orig = unsafe { *slot };
        let replacement = imp.op("op_sub");
        unsafe {
            *slot = replacement;
            assert_eq!((*slot)(10, 4), 6, "{}: store into G_OP not observed", imp.name);
            *slot = orig;
            assert_eq!(*slot as usize, orig as usize);
        }
        let _ = replacement;

        let nslot = imp.g_op_name_slot();
        let norig = unsafe { *nslot };
        let other = b"zz\0";
        unsafe {
            *nslot = other.as_ptr() as *const std::ffi::c_char;
            assert_eq!(imp.g_op_name(), b"zz", "{}: store into G_OP_NAME not observed", imp.name);
            *nslot = norig;
        }
        assert_eq!(imp.g_op_name(), OP.as_bytes());
    }
}

/* ---- E-18: absence of pointer parameters ----------------------------- */

/// There is no pointer parameter anywhere in the library surface, so there is
/// no null-pointer path to compare: all six functions take and return `int`
/// only. Documented here so the absence is explicit rather than overlooked.
/// What *is* testable is the pointer-shaped part of the surface: the two `.so`
/// data objects must both be non-null and dereferenceable.
#[test]
fn e18_no_pointer_parameters_but_data_objects_are_valid() {
    let p = pair();
    for imp in [&p.c, &p.rust] {
        assert!(!imp.g_op_slot().is_null(), "{}: G_OP address null", imp.name);
        assert!(
            !imp.g_op_name_slot().is_null(),
            "{}: G_OP_NAME address null",
            imp.name
        );
        assert!(
            !unsafe { *imp.g_op_name_slot() }.is_null(),
            "{}: G_OP_NAME contents null",
            imp.name
        );
    }
}

/* ---- E-01, E-08 .. E-10: the `main` rejection surface ----------------
 * These live in mdmain.c, which is the program entry point rather than part of
 * the .so, so they are compared between the two *executables*.
 * -------------------------------------------------------------------- */

fn run(exe: &std::path::Path, args: &[&str]) -> (Option<i32>, Vec<u8>, Vec<u8>) {
    let out = Command::new(exe)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    (out.status.code(), out.stdout, out.stderr)
}

/// E-01 -- `if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }`
#[test]
fn e01_usage_error_for_too_few_arguments() {
    let c = c_exe_path();
    let r = rust_exe_path();
    if !c.exists() || !r.exists() {
        eprintln!("skipping e01: executables not built");
        return;
    }
    for args in [vec![], vec!["1"]] {
        let (cst, cout, cerr) = run(&c, &args);
        let (rst, rout, rerr) = run(&r, &args);
        assert_eq!(cst, Some(2), "C exit status for argc={}", args.len() + 1);
        assert_eq!(cst, rst, "exit status for args={args:?}");
        assert_eq!(cout, rout, "stdout for args={args:?} (both must be empty)");
        assert!(cout.is_empty());
        // argv[0] differs between the two binaries; compare the rest verbatim.
        let norm = |b: &[u8], exe: &std::path::Path| -> Vec<u8> {
            let s = String::from_utf8_lossy(b).into_owned();
            s.replace(exe.to_str().unwrap(), "PROG").into_bytes()
        };
        assert_eq!(
            norm(&cerr, &c),
            norm(&rerr, &r),
            "stderr for args={args:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&cerr),
            String::from_utf8_lossy(&rerr)
        );
        assert_eq!(norm(&cerr, &c), b"usage: PROG A B\n".to_vec());
    }
}

/// E-08 / E-09 / E-10 -- `atoi` never reports an error: non-numeric input
/// yields 0, a numeric prefix is taken and the rest discarded, and out-of-range
/// magnitudes go through glibc's `(int)strtol` saturation.
#[test]
fn e08_e10_atoi_rejections_are_silent_and_identical() {
    let c = c_exe_path();
    let r = rust_exe_path();
    if !c.exists() || !r.exists() {
        eprintln!("skipping e08_e10: executables not built");
        return;
    }
    let cases: &[[&str; 2]] = &[
        ["abc", "def"],                                   // E-08 fully non-numeric
        ["", ""],                                         // empty strings
        ["+", "-"],                                       // sign with no digits
        ["12x", "7"],                                     // E-09 numeric prefix
        ["  -12abc", "+9"],                               // whitespace + sign + prefix
        ["\t\n 42", "0007"],                              // other whitespace, leading zeros
        ["99999999999999999999", "3"],                    // E-10 > LONG_MAX
        ["-99999999999999999999", "3"],                   // < LONG_MIN
        ["9223372036854775807", "1"],                     // exactly LONG_MAX
        ["9223372036854775808", "1"],                     // LONG_MAX + 1
        ["-9223372036854775808", "1"],                    // exactly LONG_MIN
        ["-9223372036854775809", "1"],                    // LONG_MIN - 1
        ["2147483647", "1"],                              // INT_MAX
        ["2147483648", "1"],                              // INT_MAX + 1
        ["-2147483648", "-1"],                            // INT_MIN
        ["-2147483649", "-1"],                            // INT_MIN - 1
        ["4294967296", "1"],                              // 2^32
        ["0x10", "010"],                                  // atoi is decimal-only
        ["1e3", "1.5"],                                   // no exponent / fraction
        ["-0", "+0"],
        ["   ", "5"],                                     // whitespace only
    ];
    for case in cases {
        let args: Vec<&str> = case.to_vec();
        let (cst, cout, cerr) = run(&c, &args);
        let (rst, rout, rerr) = run(&r, &args);
        assert_eq!(cst, rst, "exit status for {args:?}");
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "stdout for {args:?}"
        );
        assert_eq!(cerr, rerr, "stderr for {args:?} (both must be empty)");
        assert!(cerr.is_empty(), "atoi must not diagnose anything");
    }
}

/// E-16 / E-17 -- build-time range limits. `CHOOSE_REP(n)` pastes `REP` and `n`,
/// and only `REP0`..`REP7` exist, so `REPEAT=8` fails to compile; likewise an
/// `OP` token with no `STEP_<op>` / `INIT_<op>` / `op_<op>` fails. The Rust
/// crate mirrors this by simply not offering `repeat_8` / other OP features --
/// `cargo` rejects an unknown feature name. Asserted mechanically here.
#[test]
fn e16_e17_out_of_range_build_configurations_do_not_exist() {
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    for absent in ["repeat_8", "repeat_9", "\"8\" =", "\"9\" ="] {
        assert!(
            !manifest.contains(absent),
            "Cargo.toml offers {absent}, but mdmacros.h defines only REP0..REP7"
        );
    }
    for present in ["repeat_0", "repeat_7", "add", "sub", "mul"] {
        assert!(manifest.contains(present), "Cargo.toml is missing {present}");
    }
    // And REPEAT is inside the range the header supports.
    assert!(
        (0..=7).contains(&REPEAT),
        "REPEAT={REPEAT} is outside REP0..REP7"
    );
    let _: c_int = REPEAT;
}
