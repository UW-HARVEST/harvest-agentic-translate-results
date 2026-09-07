//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row group.
//!
//! This binary is compiled once per feature combination; the rows that name a
//! specific `OP`/`REPEAT` are asserted for whichever configuration is active,
//! and `run_all_features.sh` iterates every combination so all 117 rows are
//! covered.
//!
//! Both implementations are reached only through `dlopen`/`dlsym` on their
//! `.so`, or by executing their `driver` binary.

mod common;

use common::*;
use std::ffi::c_int;

/* ================================================================== */
/* G1 — the three operation primitives (the lowest-level entry points) */
/* ================================================================== */

fn g1_shapes(seed_base: u64) -> Vec<(c_int, c_int)> {
    let mut v = Vec::new();
    // G1 shape 1: both operands zero
    v.push((0, 0));
    // G1 shape 2: small positives
    v.extend(random_pairs_in(64, SEED ^ seed_base ^ 1, 1, 1000));
    // G1 shape 3: small negatives
    v.extend(random_pairs_in(64, SEED ^ seed_base ^ 2, -1000, -1));
    // G1 shape 4: mixed signs, full i32
    v.extend(random_pairs(256, SEED ^ seed_base ^ 3));
    // G1 shape 5: the boundary set, squared
    v.extend(boundary_pairs());
    // G1 shape 6: overflow-inducing pairs
    v.extend(overflow_pairs());
    v
}

#[test]
fn g1_op_add() {
    for (a, b) in g1_shapes(0xA1) {
        assert_fn2_matches("op_add", a, b);
    }
}

#[test]
fn g1_op_sub() {
    for (a, b) in g1_shapes(0xA2) {
        assert_fn2_matches("op_sub", a, b);
    }
}

#[test]
fn g1_op_mul() {
    for (a, b) in g1_shapes(0xA3) {
        assert_fn2_matches("op_mul", a, b);
    }
}

/* ================================================================== */
/* G2 — the exported `G_OP` function-pointer datum                    */
/* ================================================================== */

#[test]
fn g2_g_op_call() {
    for (a, b) in random_pairs(256, SEED ^ 0xB1) {
        assert_g_op_matches(a, b);
    }
    for (a, b) in boundary_pairs() {
        assert_g_op_matches(a, b);
    }
    for (a, b) in overflow_pairs() {
        assert_g_op_matches(a, b);
    }
}

/// `int (*G_OP)(int,int) = OP_FN(OP);` — the stored pointer must be the address
/// of that library's own exported `op_<OP>`, so a consumer's `G_OP == op_add`
/// comparison gives the same answer for both.
#[test]
fn g2_g_op_identity() {
    let bo = both();
    let expected = format!("op_{OP}");
    for lib in [&bo.c, &bo.r] {
        let stored = unsafe { *g_op_slot(lib) } as usize;
        let mut got = None;
        for name in ["op_add", "op_sub", "op_mul"] {
            if fn2(lib, name) as usize == stored {
                got = Some(name);
            }
        }
        assert_eq!(
            got,
            Some(expected.as_str()),
            "[{}] G_OP must point at {expected} (stored={stored:#x})",
            tag()
        );
    }
}

/* ================================================================== */
/* G3 — the exported `G_OP_NAME` string datum, `STR(OP)`              */
/* ================================================================== */

#[test]
fn g3_g_op_name() {
    let bo = both();
    let c = read_g_op_name(&bo.c);
    let r = read_g_op_name(&bo.r);
    assert_eq!(
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r),
        "[{}] G_OP_NAME contents",
        tag()
    );
    assert_eq!(
        c,
        OP.as_bytes(),
        "[{}] G_OP_NAME must be STR(OP) = {OP:?}",
        tag()
    );
}

/* ================================================================== */
/* G4 — `helper_ptr` (calls through a local fn pointer)               */
/* ================================================================== */

#[test]
fn g4_helper_ptr() {
    for (a, b) in random_pairs(256, SEED ^ 0xC1) {
        assert_fn2_matches("helper_ptr", a, b);
    }
    for (a, b) in boundary_pairs() {
        assert_fn2_matches("helper_ptr", a, b);
    }
    for (a, b) in overflow_pairs() {
        assert_fn2_matches("helper_ptr", a, b);
    }
}

/* ================================================================== */
/* G5 — `helper_call`: depends on BOTH `OP` and `REPEAT`              */
/* ================================================================== */

#[test]
fn g5_helper_call() {
    for (a, b) in random_pairs(256, SEED ^ 0xD1) {
        assert_fn2_matches("helper_call", a, b);
    }
    for (a, b) in boundary_pairs() {
        assert_fn2_matches("helper_call", a, b);
    }
    for (a, b) in overflow_pairs() {
        assert_fn2_matches("helper_call", a, b);
    }
}

/// Independently re-derives `RUN_LOOP(OP, acc, REPEAT)` = `REP<REPEAT>` from the
/// header's macro definitions and pins the `helper.acc=` value both libraries
/// print, so a shared bug in the two `acc` computations could not hide behind
/// "they agree with each other".
#[test]
fn g5_helper_call_acc_is_rep_repeat() {
    let mut expected_acc = init_for_op();
    for i in 0..REPEAT {
        expected_acc = step(expected_acc, i);
    }

    let bo = both();
    let cf = fn2(&bo.c, "helper_call");
    let (ret, out) = capture_fd1(|| unsafe { cf(11, 5) });
    let text = String::from_utf8_lossy(&out).into_owned();
    let expected_r: c_int = match OP {
        "add" => 11i32.wrapping_add(5),
        "sub" => 11i32.wrapping_sub(5),
        _ => 11i32.wrapping_mul(5),
    };
    assert_eq!(
        text,
        format!("helper.call={expected_r} helper.acc={expected_acc}\n"),
        "[{}] C helper_call stdout",
        tag()
    );
    assert_eq!(ret, expected_r.wrapping_add(expected_acc), "[{}]", tag());
}

/* ================================================================== */
/* G6 — `use_generated` -> `accum_<OP>`, one row per `DISPATCH_REP` case */
/* ================================================================== */

#[test]
fn g6_use_generated_each_case() {
    for n in 0..=6 {
        assert_fn1_matches("use_generated", n);
    }
}

/// Pins each `case` against the header's unrolled `REPn` chain, independently
/// re-derived, for the same reason as `g5_helper_call_acc_is_rep_repeat`.
#[test]
fn g6_use_generated_matches_unrolled_chain() {
    let bo = both();
    let cf = fn1(&bo.c, "use_generated");
    let rf = fn1(&bo.r, "use_generated");
    for n in 0..=6 {
        let mut expected = init_for_op();
        for i in 0..n {
            expected = step(expected, i);
        }
        let (c, cout) = capture_fd1(|| unsafe { cf(n) });
        let (r, rout) = capture_fd1(|| unsafe { rf(n) });
        assert_eq!(c, expected, "[{}] C use_generated({n})", tag());
        assert_eq!(r, expected, "[{}] Rust use_generated({n})", tag());
        assert_eq!(
            String::from_utf8_lossy(&cout),
            format!("gen.acc={expected}\n"),
            "[{}] C use_generated({n}) stdout",
            tag()
        );
        assert_eq!(
            String::from_utf8_lossy(&rout),
            format!("gen.acc={expected}\n"),
            "[{}] Rust use_generated({n}) stdout",
            tag()
        );
    }
}

/* ================================================================== */
/* G7 — the exported globals are WRITABLE in the C `.so`              */
/* ================================================================== */

/// `mdmacros.h` declares `extern int (*G_OP)(int, int);` — non-`const` — and the
/// C `.so` places it in `.data`, so a consumer may reassign it. A Rust
/// translation using an immutable `static` lands in `.data.rel.ro`, which the
/// loader maps read-only (`PT_GNU_RELRO`), turning the same store into a
/// `SIGSEGV`. This test performs the store against both libraries.
#[test]
fn g7_g_op_is_writable() {
    let bo = both();
    for (which, lib) in [("C", &bo.c), ("Rust", &bo.r)] {
        let slot = g_op_slot(lib);
        let original = unsafe { *slot };
        for name in ["op_add", "op_sub", "op_mul"] {
            let target = fn2(lib, name);
            unsafe { *slot = target };
            let readback = unsafe { *slot };
            assert_eq!(
                readback as usize, target as usize,
                "[{}] {which}: store into G_OP did not stick ({name})",
                tag()
            );
            for (a, b) in random_pairs(32, SEED ^ 0xE1) {
                let via_global = unsafe { readback(a, b) };
                let direct = unsafe { target(a, b) };
                assert_eq!(via_global, direct, "[{}] {which}: G_OP:={name}", tag());
            }
        }
        unsafe { *slot = original };
        assert_eq!(unsafe { *slot } as usize, original as usize);
    }
}

#[test]
fn g7_g_op_name_is_writable() {
    let bo = both();
    for (which, lib) in [("C", &bo.c), ("Rust", &bo.r)] {
        let slot = g_op_name_slot(lib);
        let original = unsafe { *slot };
        // Point it one byte further into its own string literal: still a valid
        // NUL-terminated C string inside the same library's mapping.
        let shifted = unsafe { original.add(1) };
        unsafe { *slot = shifted };
        assert_eq!(
            unsafe { *slot },
            shifted,
            "[{}] {which}: store into G_OP_NAME did not stick",
            tag()
        );
        assert_eq!(
            read_g_op_name(lib),
            OP.as_bytes()[1..].to_vec(),
            "[{}] {which}: readback after G_OP_NAME store",
            tag()
        );
        unsafe { *slot = original };
        assert_eq!(read_g_op_name(lib), OP.as_bytes());
    }
}

/// In the C, `helper_call`/`helper_ptr` use `OP_FN(OP)` directly, **not** the
/// `G_OP` global, so mutating `G_OP` must not change what they do.
#[test]
fn g7_mutating_g_op_does_not_affect_helpers() {
    let bo = both();
    let baseline: Vec<(c_int, Vec<u8>, c_int, Vec<u8>)> = random_pairs(16, SEED ^ 0xE2)
        .into_iter()
        .map(|(a, b)| {
            let cf = fn2(&bo.c, "helper_call");
            let cp = fn2(&bo.c, "helper_ptr");
            let (r1, o1) = capture_fd1(|| unsafe { cf(a, b) });
            let (r2, o2) = capture_fd1(|| unsafe { cp(a, b) });
            (r1, o1, r2, o2)
        })
        .collect();

    // Point both libraries' G_OP at something else entirely.
    let other = if OP == "mul" { "op_add" } else { "op_mul" };
    let mut saved = Vec::new();
    for lib in [&bo.c, &bo.r] {
        let slot = g_op_slot(lib);
        saved.push((slot, unsafe { *slot }));
        unsafe { *slot = fn2(lib, other) };
    }

    for (idx, (a, b)) in random_pairs(16, SEED ^ 0xE2).into_iter().enumerate() {
        for lib in [&bo.c, &bo.r] {
            let hc = fn2(lib, "helper_call");
            let hp = fn2(lib, "helper_ptr");
            let (r1, o1) = capture_fd1(|| unsafe { hc(a, b) });
            let (r2, o2) = capture_fd1(|| unsafe { hp(a, b) });
            assert_eq!(
                (r1, &o1, r2, &o2),
                (
                    baseline[idx].0,
                    &baseline[idx].1,
                    baseline[idx].2,
                    &baseline[idx].3
                ),
                "[{}] helpers must ignore G_OP",
                tag()
            );
        }
    }

    for (slot, original) in saved {
        unsafe { *slot = original };
    }
}

/* ================================================================== */
/* G8 — the `driver` executable, end to end                           */
/* ================================================================== */

fn driver_arg_shapes() -> Vec<Vec<String>> {
    let mut v: Vec<Vec<String>> = Vec::new();

    // 60 randomized full-i32 operand pairs, fixed seed.
    for (a, b) in random_pairs(60, SEED ^ 0xF1) {
        v.push(vec![a.to_string(), b.to_string()]);
    }
    // small ranges
    for (a, b) in random_pairs_in(20, SEED ^ 0xF2, -1000, 1000) {
        v.push(vec![a.to_string(), b.to_string()]);
    }
    // boundary / overflow operands
    for (a, b) in boundary_pairs().into_iter().chain(overflow_pairs()) {
        v.push(vec![a.to_string(), b.to_string()]);
    }
    // `atoi` text shapes that are still *valid* input for the program
    for s in [
        ["  12", "\t-3"],
        ["+5", "-6"],
        ["-0", "+0"],
        ["010", "0x10"],
        ["7abc", "8 9"],
        ["0000000009", "-0000000009"],
    ] {
        v.push(vec![s[0].to_string(), s[1].to_string()]);
    }
    // extra trailing argv (mdmain.c ignores argv[3..])
    v.push(vec![
        "3".into(),
        "4".into(),
        "ignored".into(),
        "also-ignored".into(),
    ]);
    v.push(vec!["-9".into(), "2".into(), "".into()]);
    v
}

#[test]
fn g8_driver_stdout_matches() {
    for args in driver_arg_shapes() {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        assert_driver_matches(&refs);
    }
}

/// The driver's `use_generated(REPEAT)` call and `op=%s` line make `OP`/`REPEAT`
/// observable in stdout; pin them so a matching-but-wrong pair is caught.
#[test]
fn g8_driver_reports_active_config() {
    let a = artifacts();
    let out = run_driver(&a.c_bin_dir, &["11", "5"]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains(&format!("op={OP} ")),
        "[{}] driver stdout should name OP: {text}",
        tag()
    );
    let rout = run_driver(&a.r_bin_dir, &["11", "5"]);
    assert_eq!(
        text,
        String::from_utf8_lossy(&rout.stdout),
        "[{}] driver stdout",
        tag()
    );
}

/* ================================================================== */
/* G9 — the implicit "no macro defined" build                         */
/* ================================================================== */

/// Row G9: the C `#ifndef OP / #define OP add` + `#ifndef REPEAT / #define
/// REPEAT 5` fallbacks must equal what Cargo produces with no feature at all.
/// Compiled and compared here directly (independently of the active feature
/// set) because no single feature combination can express "unset".
#[test]
fn g9_unset_macros_equal_add_repeat_5() {
    use std::path::PathBuf;
    use std::process::Command;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("../c_src/src").canonicalize().unwrap();
    let out = manifest.join("target/difftest/g9");
    std::fs::create_dir_all(out.join("cbin")).unwrap();
    std::fs::create_dir_all(out.join("rbin")).unwrap();

    // C with NO -DOP / -DREPEAT at all.
    let st = Command::new("gcc")
        .args([
            "-o",
            out.join("cbin/driver").to_str().unwrap(),
            src.join("mdcore.c").to_str().unwrap(),
            src.join("mdmain.c").to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(st.success(), "gcc (no -D flags) failed");

    let clib = out.join("libcdriver_nodefs.so");
    let st = Command::new("gcc")
        .args([
            "-shared",
            "-fPIC",
            "-o",
            clib.to_str().unwrap(),
            src.join("mdcore.c").to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(st.success(), "gcc -shared (no -D flags) failed");

    // Rust with NO feature at all.
    let rtd = out.join("rust");
    let st = Command::new(env!("CARGO"))
        .current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &rtd)
        .args(["build", "--release", "--quiet", "--no-default-features"])
        .status()
        .unwrap();
    assert!(st.success(), "cargo build --no-default-features failed");
    std::fs::copy(rtd.join("release/driver"), out.join("rbin/driver")).unwrap();
    let rlib = rtd.join("release/libdriver.so");

    // Same 8 exported symbols.
    let names = |p: &std::path::Path| -> Vec<String> {
        let o = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .unwrap();
        let mut v: Vec<String> = String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        v.sort();
        v
    };
    assert_eq!(names(&clib), names(&rlib), "no-macro build symbol parity");

    // Same behaviour through both `.so`s and both executables.
    unsafe {
        let cl = libloading::Library::new(&clib).unwrap();
        let rl = libloading::Library::new(&rlib).unwrap();
        for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
            let mut s = name.as_bytes().to_vec();
            s.push(0);
            let cf: libloading::Symbol<Op2> = cl.get(&s).unwrap();
            let rf: libloading::Symbol<Op2> = rl.get(&s).unwrap();
            for (a, b) in random_pairs(64, SEED ^ 0x9)
                .into_iter()
                .chain(boundary_pairs())
            {
                assert_eq!((*cf)(a, b), (*rf)(a, b), "no-macro build: {name}({a},{b})");
            }
        }
        let cg: libloading::Symbol<Op1> = cl.get(b"use_generated\0").unwrap();
        let rg: libloading::Symbol<Op1> = rl.get(b"use_generated\0").unwrap();
        for n in -8..=12 {
            assert_eq!((*cg)(n), (*rg)(n), "no-macro build: use_generated({n})");
        }
        let cn: libloading::Symbol<*mut *const std::ffi::c_char> =
            cl.get(b"G_OP_NAME\0").unwrap();
        let rn: libloading::Symbol<*mut *const std::ffi::c_char> =
            rl.get(b"G_OP_NAME\0").unwrap();
        assert_eq!(
            std::ffi::CStr::from_ptr(**cn).to_bytes(),
            b"add",
            "unset OP must fall back to add"
        );
        assert_eq!(
            std::ffi::CStr::from_ptr(**cn).to_bytes(),
            std::ffi::CStr::from_ptr(**rn).to_bytes()
        );
    }

    for args in [
        vec!["3", "4"],
        vec!["-7", "5"],
        vec!["0", "0"],
        vec!["2147483647", "1"],
        vec!["abc", "def"],
        vec!["1"],
        vec![],
    ] {
        let c = run_driver(&out.join("cbin"), &args);
        let r = run_driver(&out.join("rbin"), &args);
        assert_eq!(
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout),
            "no-macro build: driver {args:?} stdout"
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
            "no-macro build: driver {args:?} stderr"
        );
        assert_eq!(c.code, r.code, "no-macro build: driver {args:?} status");
    }

    // `REPEAT` unset must equal 5: acc for the default `add` build is 0+1+2+3+4.
    let c = run_driver(&out.join("cbin"), &["0", "0"]);
    assert!(
        String::from_utf8_lossy(&c.stdout).contains("acc=10"),
        "unset REPEAT must fall back to 5 (acc=0+1+2+3+4=10): {}",
        String::from_utf8_lossy(&c.stdout)
    );
}
