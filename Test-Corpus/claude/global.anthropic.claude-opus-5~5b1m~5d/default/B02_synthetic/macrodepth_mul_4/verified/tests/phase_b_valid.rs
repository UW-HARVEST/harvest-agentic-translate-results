//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row
//! group. Every call crosses the FFI boundary via `dlsym` into the C `.so` and
//! into the Rust `.so`.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Row 0 — the build matrix: both libraries exist for this configuration and
/// export every symbol `nm -D` shows on the C side.
#[test]
fn configs_00_build_matrix_matches() {
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let (c, r) = fn_addr(name);
        assert!(c != 0 && r != 0, "{name} not resolvable in both .so");
    }
    let (c, r) = sym1("use_generated");
    assert!(c as usize != 0 && r as usize != 0);
    for name in ["G_OP", "G_OP_NAME"] {
        let (c, r) = data_sym(name);
        assert!(!c.is_null() && !r.is_null(), "{name} not resolvable");
    }
    // `REPEAT` is limited to 0..=7 in both languages: `REP8` does not exist in
    // mdmacros.h and there is no `"8"` Cargo feature.
    assert!((0..=7).contains(&repeat()));
    assert!(["add", "sub", "mul"].contains(&op_name()));
}

/// Rows 1, 8, 15, ... — the three leaf operations, called directly.
#[test]
fn b_01_leaf_ops() {
    let mut rng = Rng::new();
    for name in ["op_add", "op_sub", "op_mul"] {
        for &a in BOUNDARY.iter() {
            for &b in BOUNDARY.iter() {
                assert_fn2_eq(name, a, b);
            }
        }
        for _ in 0..512 {
            let (a, b) = (rng.next_i32(), rng.next_i32());
            assert_fn2_eq(name, a, b);
        }
        for _ in 0..128 {
            let (a, b) = (rng.next_small(), rng.next_small());
            assert_fn2_eq(name, a, b);
        }
    }
}

/// Rows 2, 9, 16, ... — the `G_OP` global: initialised by `OP_FN(OP)` at load
/// time, must point at this library's own `op_<OP>` and behave like it.
#[test]
fn b_02_g_op_global() {
    let (c_slot, r_slot) = data_sym("G_OP");
    let c_fp = unsafe { *(c_slot as *const usize) };
    let r_fp = unsafe { *(r_slot as *const usize) };
    let (c_expected, r_expected) = fn_addr(&format!("op_{}", op_name()));
    assert_eq!(
        c_fp, c_expected,
        "C G_OP does not point at op_{}",
        op_name()
    );
    assert_eq!(
        r_fp, r_expected,
        "Rust G_OP does not point at op_{} (OP_FN(OP) mis-translated)",
        op_name()
    );

    // Call through the pointer read out of `.data` in both libraries.
    type F = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let cf: F = unsafe { std::mem::transmute(c_fp) };
    let rf: F = unsafe { std::mem::transmute(r_fp) };
    let mut rng = Rng::new();
    let mut inputs: Vec<(c_int, c_int)> = Vec::new();
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            inputs.push((a, b));
        }
    }
    for _ in 0..256 {
        inputs.push((rng.next_i32(), rng.next_i32()));
    }
    for (a, b) in inputs {
        let cr = unsafe { cf(a, b) };
        let rr = unsafe { rf(a, b) };
        assert_eq!(cr, rr, "G_OP({a}, {b}) mismatch [OP={}]", op_name());
    }
}

/// Rows 3, 10, 17, ... — the `G_OP_NAME` global (`STR(OP)`).
#[test]
fn b_03_g_op_name_global() {
    let (c_slot, r_slot) = data_sym("G_OP_NAME");
    let c_ptr = unsafe { *(c_slot as *const *const c_char) };
    let r_ptr = unsafe { *(r_slot as *const *const c_char) };
    assert!(!c_ptr.is_null() && !r_ptr.is_null());
    let cb = unsafe { cstr_bytes(c_ptr) };
    let rb = unsafe { cstr_bytes(r_ptr) };
    assert_eq!(
        String::from_utf8_lossy(&cb),
        String::from_utf8_lossy(&rb),
        "G_OP_NAME bytes mismatch"
    );
    assert_eq!(String::from_utf8_lossy(&cb), op_name());
}

/// Rows 4, 11, 18, ... — `helper_ptr`: the indirect-call path plus its printf.
#[test]
fn b_04_helper_ptr() {
    let mut rng = Rng::new();
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            assert_fn2_eq("helper_ptr", a, b);
        }
    }
    for _ in 0..256 {
        assert_fn2_eq("helper_ptr", rng.next_i32(), rng.next_i32());
    }
    for _ in 0..64 {
        assert_fn2_eq("helper_ptr", rng.next_small(), rng.next_small());
    }
}

/// Rows 5, 12, 19, ... — `helper_call`: `OP_FN(OP)` plus the `REP<REPEAT>`
/// unrolled accumulator and the two-value printf.
#[test]
fn b_05_helper_call() {
    let mut rng = Rng::new();
    for &a in BOUNDARY.iter() {
        for &b in BOUNDARY.iter() {
            assert_fn2_eq("helper_call", a, b);
        }
    }
    for _ in 0..256 {
        assert_fn2_eq("helper_call", rng.next_i32(), rng.next_i32());
    }
    for _ in 0..64 {
        assert_fn2_eq("helper_call", rng.next_small(), rng.next_small());
    }
}

/// Rows 6, 13, 20, ... — `use_generated`: `static accum_<OP>` + `DISPATCH_REP`
/// over every reachable `switch` case and the `default`.
#[test]
fn b_06_use_generated() {
    let mut rng = Rng::new();
    for n in -64..=64 {
        assert_fn1_eq("use_generated", n);
    }
    for &n in &[
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MAX,
        c_int::MAX - 1,
        c_int::MIN / 2,
        c_int::MAX / 2,
        repeat(),
    ] {
        assert_fn1_eq("use_generated", n);
    }
    for _ in 0..512 {
        assert_fn1_eq("use_generated", rng.next_i32());
    }
}

/// Harness self-check + independent oracle: the captured stdout must be exactly
/// the text `printf` produces, recomputed here from `mdmacros.h` semantics.
/// This proves the capture machinery is not silently returning empty buffers.
#[test]
fn b_08_printf_text_oracle() {
    // STEP_<OP>(acc, i) and INIT_FOR(OP), re-derived from mdmacros.h:48-58.
    fn step(acc: i32, i: i32) -> i32 {
        match op_name() {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_mul(i.wrapping_add(1)),
        }
    }
    fn op(a: i32, b: i32) -> i32 {
        match op_name() {
            "add" => a.wrapping_add(b),
            "sub" => a.wrapping_sub(b),
            _ => a.wrapping_mul(b),
        }
    }
    // RUN_LOOP == REP<REPEAT>
    let mut acc = init();
    for i in 0..repeat() {
        acc = step(acc, i);
    }

    let mut rng = Rng::new();
    let (cc, rc) = sym2("helper_call");
    let (cp, rp) = sym2("helper_ptr");
    let (cg, rg) = sym1("use_generated");
    for _ in 0..32 {
        let (a, b) = (rng.next_i32(), rng.next_i32());
        let r = op(a, b);

        let (cret, cout) = capture(|| unsafe { cc(a, b) });
        let (rret, rout) = capture(|| unsafe { rc(a, b) });
        let want = format!("helper.call={} helper.acc={}\n", r, acc);
        assert_eq!(String::from_utf8_lossy(&cout), want, "C helper_call text");
        assert_eq!(String::from_utf8_lossy(&rout), want, "Rust helper_call text");
        assert_eq!(cret, r.wrapping_add(acc));
        assert_eq!(rret, r.wrapping_add(acc));

        let (cret, cout) = capture(|| unsafe { cp(a, b) });
        let (rret, rout) = capture(|| unsafe { rp(a, b) });
        let want = format!("helper.ptr={}\n", r);
        assert_eq!(String::from_utf8_lossy(&cout), want, "C helper_ptr text");
        assert_eq!(String::from_utf8_lossy(&rout), want, "Rust helper_ptr text");
        assert_eq!(cret, r);
        assert_eq!(rret, r);
    }
    // DISPATCH_REP: cases 0..=6 run REP<n>, everything else keeps INIT.
    for n in -3..=9 {
        let mut want_acc = init();
        if (0..=6).contains(&n) {
            for i in 0..n {
                want_acc = step(want_acc, i);
            }
        }
        let (cret, cout) = capture(|| unsafe { cg(n) });
        let (rret, rout) = capture(|| unsafe { rg(n) });
        let want = format!("gen.acc={}\n", want_acc);
        assert_eq!(String::from_utf8_lossy(&cout), want, "C use_generated({n})");
        assert_eq!(
            String::from_utf8_lossy(&rout),
            want,
            "Rust use_generated({n})"
        );
        assert_eq!(cret, want_acc);
        assert_eq!(rret, want_acc);
    }
}

/// Rows 7, 14, 21, ... — the whole composed pipeline through `driver` `main`.
#[test]
fn b_07_driver_pipeline() {
    let cases: &[&[&str]] = &[
        &["0", "0"],
        &["1", "1"],
        &["3", "4"],
        &["-7", "12"],
        &["2147483647", "1"],
        &["-2147483648", "-1"],
        &["99999", "99999"],
        &["65536", "65536"],
        &["  12", "-0"],
        &["+5", "-5"],
        &["7", "6", "extra"],
        &["7", "6", "extra", "more", "and", "more", "and", "more"],
        &["abc", "5"],
        &["", "5"],
        &["12abc", "3x"],
        &["0x10", "010"],
    ];
    for c in cases {
        assert_driver_eq(c);
    }
    let mut rng = Rng::new();
    for _ in 0..64 {
        let a = rng.next_i32().to_string();
        let b = rng.next_i32().to_string();
        assert_driver_eq(&[&a, &b]);
    }
    for _ in 0..64 {
        let a = rng.next_small().to_string();
        let b = rng.next_small().to_string();
        assert_driver_eq(&[&a, &b]);
    }
}
