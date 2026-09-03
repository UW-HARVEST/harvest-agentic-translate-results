//! Phase C — error / rejection path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI boundaries (null
//! pointers, zero/oversized lengths, one-past-range values, out-of-range
//! "enum" ints). Every call is made through `dlsym` on both `.so`s.

mod common;

use common::{capture_stdout, pair, Rng, INIT, OP, REPEAT};
use std::ffi::c_int;
use std::process::Command;

/* ============================ ERRORS.md row 1 =========================== */
/* main: argc < 3 -> usage on stderr, exit status 2, empty stdout.          */

#[test]
fn err01_main_argc_less_than_3() {
    let cexe = common::c_exe_path();
    let rexe = common::rust_exe_path();

    for args in [vec![], vec!["7".to_string()]] {
        let co = Command::new(&cexe).args(&args).output().expect("run C");
        let ro = Command::new(&rexe).args(&args).output().expect("run Rust");

        assert_eq!(
            co.status.code(),
            Some(2),
            "C: argc<3 must exit 2 (args={:?})",
            args
        );
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status for args={:?} [OP={} REPEAT={}]",
            args,
            OP,
            REPEAT
        );
        assert!(co.stdout.is_empty(), "C wrote to stdout on the usage path");
        assert_eq!(
            co.stdout, ro.stdout,
            "stdout must both be empty for args={:?}",
            args
        );

        // stderr must be `usage: <argv[0]> A B\n`; argv[0] necessarily differs
        // between the two binaries, so compare with argv[0] normalised out.
        let cerr = String::from_utf8_lossy(&co.stderr).replace(cexe.to_str().unwrap(), "PROG");
        let rerr = String::from_utf8_lossy(&ro.stderr).replace(rexe.to_str().unwrap(), "PROG");
        assert_eq!(cerr, "usage: PROG A B\n", "C usage text changed?");
        assert_eq!(cerr, rerr, "stderr for args={:?}", args);
    }
}

/* ============================ ERRORS.md row 2 =========================== */
/* main: operands that atoi cannot parse -> silent 0 / prefix / truncation. */

#[test]
fn err02_main_unparsable_operands() {
    let cexe = common::c_exe_path();
    let rexe = common::rust_exe_path();

    let cases: &[[&str; 2]] = &[
        ["abc", "def"],
        ["", ""],
        ["  12x", "0034"],
        ["+8", "-8"],
        ["-", "+"],
        ["99999999999999999999", "-99999999999999999999"],
        ["2147483648", "-2147483649"],
        ["9223372036854775807", "-9223372036854775808"],
        ["9223372036854775808", "-9223372036854775809"],
        ["0x10", "010"],
        ["\t\n 42junk", "  -0"],
        ["1e3", "3.9"],
        ["--5", "++5"],
        ["4294967296", "4294967297"],
    ];

    for args in cases {
        let co = Command::new(&cexe).args(args).output().expect("run C");
        let ro = Command::new(&rexe).args(args).output().expect("run Rust");
        assert_eq!(
            co.status.code(),
            Some(0),
            "C: atoi has no error channel, must still exit 0 for {:?}",
            args
        );
        assert_eq!(co.status.code(), ro.status.code(), "exit status {:?}", args);
        assert_eq!(
            String::from_utf8_lossy(&co.stdout),
            String::from_utf8_lossy(&ro.stdout),
            "stdout for {:?} [OP={} REPEAT={}]",
            args,
            OP,
            REPEAT
        );
        assert_eq!(
            String::from_utf8_lossy(&co.stderr),
            String::from_utf8_lossy(&ro.stderr),
            "stderr for {:?}",
            args
        );
    }
}

/* ======================= ERRORS.md rows 3-7, 9 ========================== */
/* use_generated / DISPATCH_REP: the `default: break;` rejection.           */

fn check_use_generated(n: c_int, expect_default_branch: bool) {
    let p = pair();
    let c = p.c.un1("use_generated");
    let r = p.r.un1("use_generated");
    let (cv, cout) = capture_stdout(|| unsafe { c(n) });
    let (rv, rout) = capture_stdout(|| unsafe { r(n) });

    assert_eq!(
        cv, rv,
        "use_generated({}): C returned {} but Rust returned {} [OP={} REPEAT={}]",
        n, cv, rv, OP, REPEAT
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "use_generated({}) stdout [OP={} REPEAT={}]",
        n,
        OP,
        REPEAT
    );
    if expect_default_branch {
        // The rejection is "accumulator untouched", i.e. exactly INIT_FOR(OP) —
        // not merely "some value".
        assert_eq!(
            cv, INIT,
            "C: use_generated({}) must hit `default: break` and return INIT_FOR({})",
            n, OP
        );
        assert_eq!(
            String::from_utf8_lossy(&cout),
            format!("gen.acc={}\n", INIT),
            "C: default-branch print for n={}",
            n
        );
    }
}

#[test]
fn err03_use_generated_negative_n() {
    for n in [-1, -2, -3, -7, -100, -1000000] {
        check_use_generated(n, true);
    }
}

#[test]
fn err04_use_generated_n_above_6() {
    // NB: REP7 exists in mdmacros.h but is NOT a switch case, so n == 7 is
    // rejected even in a REPEAT=7 build.
    for n in [7, 8, 9, 100, 1000, 65536] {
        check_use_generated(n, true);
    }
}

#[test]
fn err05_use_generated_int_min() {
    check_use_generated(i32::MIN, true);
    check_use_generated(i32::MIN + 1, true);
}

#[test]
fn err06_use_generated_int_max() {
    check_use_generated(i32::MAX, true);
    check_use_generated(i32::MAX - 1, true);
}

#[test]
fn err07_use_generated_inrange_boundaries() {
    // One step inside the valid switch range on both ends.
    check_use_generated(0, true); // REP0 is empty -> also equals INIT
    check_use_generated(1, false);
    check_use_generated(5, false);
    check_use_generated(6, false);
    // ...and one step outside on both ends.
    check_use_generated(-1, true);
    check_use_generated(7, true);
}

/// Out-of-range "enum-like" integers across the FFI boundary. `use_generated`'s
/// `n` is the only parameter in the API with a restricted valid domain
/// (`switch` cases 0..=6); every other int is total. Sweep values with no valid
/// variant, including ones that alias in-range values modulo a smaller width.
#[test]
fn err09_out_of_range_domain_values() {
    let p = pair();
    let c = p.c.un1("use_generated");
    let r = p.r.un1("use_generated");

    let mut vals: Vec<c_int> = Vec::new();
    vals.extend(-64..=64); // dense window over the whole switch domain
    vals.extend([
        0x100,
        0x1_0000,
        0x0100_0000,
        i32::MIN,
        i32::MAX,
        -0x8000_0000i64 as c_int,
        0x7FFF_FFFF,
        // values whose low byte / low bits alias a *valid* case, which would
        // catch a truncating dispatch in the translation:
        256,
        257,
        262,
        -256,
        -249,
        65536,
        65542,
        i32::MIN + 6,
        i32::MAX - 6,
    ]);
    let mut rng = Rng::new(0xE770_0000_0000_0009);
    for _ in 0..512 {
        vals.push(rng.next_i32());
    }

    let ((), _) = capture_stdout(|| {
        for n in vals {
            let cv = unsafe { c(n) };
            let rv = unsafe { r(n) };
            assert_eq!(
                cv, rv,
                "use_generated({}) diverged: C={} Rust={} [OP={} REPEAT={}]",
                n, cv, rv, OP, REPEAT
            );
            if !(0..=6).contains(&n) {
                assert_eq!(cv, INIT, "C: out-of-domain n={} must yield INIT", n);
            }
        }
    });

    // There is no pointer parameter anywhere in the API, so there is no
    // null-pointer path to compare; the two exported pointers are outputs and
    // must be non-null and identical in content on both sides.
    assert!(!p.c.g_op_name().is_empty());
    assert_eq!(p.c.g_op_name(), p.r.g_op_name());
    assert_ne!(p.c.g_op() as usize, 0);
    assert_ne!(p.r.g_op() as usize, 0);
}

/* ============================ ERRORS.md row 8 =========================== */
/* Signed overflow: no range check in C, wrap-around in the emitted code.   */

#[test]
fn err08_signed_overflow_operands() {
    let p = pair();
    let overflow_cases: &[(c_int, c_int)] = &[
        (i32::MAX, 1),
        (1, i32::MAX),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (i32::MAX, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (46341, 46341),
        (-46341, 46341),
        (0x1_0000, 0x1_0000),
        (i32::MIN, 2),
        (i32::MAX, 2),
    ];

    for sym in ["op_add", "op_sub", "op_mul"] {
        let c = p.c.bin2(sym);
        let r = p.r.bin2(sym);
        for &(a, b) in overflow_cases {
            assert_eq!(
                unsafe { c(a, b) },
                unsafe { r(a, b) },
                "{}({}, {}) overflow behaviour diverged",
                sym,
                a,
                b
            );
        }
    }

    // Same operands through the helpers and the G_OP slot.
    let ((), _) = capture_stdout(|| {
        let cg = p.c.g_op();
        let rg = p.r.g_op();
        for sym in ["helper_call", "helper_ptr"] {
            let c = p.c.bin2(sym);
            let r = p.r.bin2(sym);
            for &(a, b) in overflow_cases {
                assert_eq!(
                    unsafe { c(a, b) },
                    unsafe { r(a, b) },
                    "{}({}, {}) overflow behaviour diverged [OP={} REPEAT={}]",
                    sym,
                    a,
                    b,
                    OP,
                    REPEAT
                );
            }
        }
        for &(a, b) in overflow_cases {
            assert_eq!(
                unsafe { cg(a, b) },
                unsafe { rg(a, b) },
                "G_OP({}, {}) overflow behaviour diverged",
                a,
                b
            );
        }
    });
}

/* =========================== ERRORS.md row 10 =========================== */
/* `OP` undefined -> `#define OP add`.                                      */

#[test]
fn err10_op_undefined_falls_back_to_add() {
    let (exe, so) = build_c_without(&["REPEAT=5"], "op_undef");
    // The fallback library must behave exactly like an explicit OP=add build.
    let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen fallback .so");
    let name = read_g_op_name(&lib);
    assert_eq!(name, b"add\0", "C: undefined OP must fall back to `add`");

    // And the Rust default feature set must agree when this run *is* add/5.
    if OP == "add" && REPEAT == 5 {
        let rexe = common::rust_exe_path();
        for args in [["3", "4"], ["-9", "2"]] {
            let co = Command::new(&exe).args(args).output().expect("run");
            let ro = Command::new(&rexe).args(args).output().expect("run");
            assert_eq!(
                String::from_utf8_lossy(&co.stdout),
                String::from_utf8_lossy(&ro.stdout),
                "implicit OP fallback vs Rust default for {:?}",
                args
            );
        }
    }
}

/* =========================== ERRORS.md row 11 =========================== */
/* `REPEAT` undefined -> `#define REPEAT 5`.                                */

#[test]
fn err11_repeat_undefined_falls_back_to_5() {
    let (exe, so) = build_c_without(&["OP=add"], "rep_undef");
    let lib = unsafe { libloading::Library::new(&so) }.expect("dlopen fallback .so");

    // helper_call's accumulator reveals REPEAT: for OP=add it is 0+1+2+3+4 = 10.
    let hc: libloading::Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"helper_call") }.expect("helper_call");
    let (v, _) = capture_stdout(|| unsafe { hc(0, 0) });
    assert_eq!(v, 10, "C: undefined REPEAT must fall back to 5 (acc == 10)");

    if OP == "add" && REPEAT == 5 {
        let rexe = common::rust_exe_path();
        for args in [["3", "4"], ["-9", "2"]] {
            let co = Command::new(&exe).args(args).output().expect("run");
            let ro = Command::new(&rexe).args(args).output().expect("run");
            assert_eq!(
                String::from_utf8_lossy(&co.stdout),
                String::from_utf8_lossy(&ro.stdout),
                "implicit REPEAT fallback vs Rust default for {:?}",
                args
            );
        }
    }
}

/* =========================== ERRORS.md row 12 =========================== */
/* REPEAT outside 0..=7 -> CHOOSE_REP picks an undefined REP<n>: the C does  */
/* not compile. The Rust mirror makes the value unrepresentable (features    */
/* "0".."7" only), so parity means "both refuse the configuration".          */

#[test]
fn err12_repeat_out_of_range_is_rejected_at_build_time() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_src = root.parent().unwrap().join("c_src").join("src");
    let dir = target_scratch("rep_range");

    for bad in ["8", "9", "-1", "100"] {
        let out = dir.join(format!("bad_{}.o", bad.replace('-', "m")));
        let st = Command::new(cc())
            .arg("-c")
            .arg("-O2")
            .arg("-DOP=add")
            .arg(format!("-DREPEAT={}", bad))
            .arg("-o")
            .arg(&out)
            .arg(c_src.join("mdcore.c"))
            .output()
            .expect("spawn cc");
        assert!(
            !st.status.success(),
            "C unexpectedly accepted REPEAT={} — ERRORS.md row 12 is wrong",
            bad
        );
        let msg = String::from_utf8_lossy(&st.stderr);
        assert!(
            msg.contains(&format!("REP{}", bad)) || msg.contains("REP"),
            "expected an undefined-REP<n> diagnostic for REPEAT={}, got: {}",
            bad,
            msg
        );
    }

    // The Rust side cannot express those values at all: the feature set is
    // exactly {"0",..,"7"}, matching REP0..REP7 in mdmacros.h.
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
    for bad in ["\"8\" =", "\"9\" =", "\"-1\" ="] {
        assert!(
            !manifest.contains(bad),
            "Cargo.toml must not offer an out-of-range REPEAT feature ({})",
            bad
        );
    }
    for good in 0..=7 {
        assert!(
            manifest.contains(&format!("\"{}\" = []", good)),
            "Cargo.toml is missing REPEAT feature \"{}\"",
            good
        );
    }
}

/* ------------------------------- helpers -------------------------------- */

fn cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

fn target_scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("cdiff_err")
        .join(tag);
    std::fs::create_dir_all(&dir).expect("mkdir scratch");
    dir
}

/// Build the C driver + a shared object with only the given `-D` defines, so the
/// `#ifndef` fallbacks in `mdmacros.h` are exercised.
fn build_c_without(defines: &[&str], tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_src = root.parent().unwrap().join("c_src").join("src");
    let dir = target_scratch(tag);
    let exe = dir.join("driver");
    let so = dir.join("libcmd.so");

    let mut cmd = Command::new(cc());
    cmd.arg("-O2");
    for d in defines {
        cmd.arg(format!("-D{}", d));
    }
    let st = cmd
        .arg("-o")
        .arg(&exe)
        .arg(c_src.join("mdcore.c"))
        .arg(c_src.join("mdmain.c"))
        .status()
        .expect("spawn cc");
    assert!(st.success(), "fallback C exe build failed");

    let mut cmd = Command::new(cc());
    cmd.arg("-O2").arg("-fPIC").arg("-shared");
    for d in defines {
        cmd.arg(format!("-D{}", d));
    }
    let st = cmd
        .arg("-o")
        .arg(&so)
        .arg(c_src.join("mdcore.c"))
        .status()
        .expect("spawn cc");
    assert!(st.success(), "fallback C .so build failed");

    (exe, so)
}

fn read_g_op_name(lib: &libloading::Library) -> Vec<u8> {
    let slot: libloading::Symbol<*mut *const std::ffi::c_char> =
        unsafe { lib.get(b"G_OP_NAME") }.expect("G_OP_NAME");
    unsafe {
        let p = **slot;
        let mut v = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i) as u8;
            v.push(b);
            if b == 0 {
                break;
            }
            i += 1;
        }
        v
    }
}
