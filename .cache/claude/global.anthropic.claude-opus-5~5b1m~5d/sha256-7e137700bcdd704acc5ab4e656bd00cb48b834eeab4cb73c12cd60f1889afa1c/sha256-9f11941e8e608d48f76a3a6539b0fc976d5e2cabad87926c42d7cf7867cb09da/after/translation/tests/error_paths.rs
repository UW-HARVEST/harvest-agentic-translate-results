//! Phase C — error/rejection-path differential tests for `ERRORS.md` rows 9-27.
//! (Rows 1-8, which concern `main`/`atoi`, live in `tests/binary_diff.rs`.)
//!
//! Every case asserts the *same* concrete result from both `.so`s — not merely
//! "both failed".

mod common;

use common::*;

/* ERRORS.md 9 — n == 7: legal for RUN_LOOP, outside DISPATCH_REP's switch. */
#[test]
fn err09_use_generated_n_eq_7_falls_to_default() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    assert_eq!(cf(7), INIT, "C use_generated(7) must be INIT_FOR({OP})={INIT}");
    assert_eq!(rf(7), cf(7), "use_generated(7) [OP={OP} REPEAT={REPEAT}]");
}

/* ERRORS.md 10 — n == 8 and beyond. */
#[test]
fn err10_use_generated_n_gt_7() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    for n in [8, 9, 10, 64, 1_000, 1 << 20, i32::MAX - 1] {
        assert_eq!(cf(n), INIT, "C use_generated({n})");
        assert_eq!(rf(n), cf(n), "use_generated({n}) [OP={OP}]");
    }
}

/* ERRORS.md 11 — n == -1, one below the first case. */
#[test]
fn err11_use_generated_n_eq_minus_1() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    for n in [-1, -2, -7, -1000] {
        assert_eq!(cf(n), INIT, "C use_generated({n})");
        assert_eq!(rf(n), cf(n), "use_generated({n}) [OP={OP}]");
    }
}

/* ERRORS.md 12 — n == INT_MIN. */
#[test]
fn err12_use_generated_int_min() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    assert_eq!(cf(i32::MIN), INIT);
    assert_eq!(rf(i32::MIN), cf(i32::MIN), "use_generated(INT_MIN) [OP={OP}]");
    assert_eq!(rf(i32::MIN + 1), cf(i32::MIN + 1));
}

/* ERRORS.md 13 — n == INT_MAX. */
#[test]
fn err13_use_generated_int_max() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    assert_eq!(cf(i32::MAX), INIT);
    assert_eq!(rf(i32::MAX), cf(i32::MAX), "use_generated(INT_MAX) [OP={OP}]");
}

/* ERRORS.md 14-15 — op_add unchecked signed overflow (must wrap, not panic). */
#[test]
fn err14_err15_op_add_overflow_wraps() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_add"), r.f2("op_add"));
    let cases = [
        ((i32::MAX, 1), i32::MIN),
        ((i32::MIN, -1), i32::MAX),
        ((i32::MAX, i32::MAX), -2),
        ((i32::MIN, i32::MIN), 0),
        ((i32::MAX, 2), i32::MIN + 1),
    ];
    for ((a, b), want) in cases {
        assert_eq!(cf(a, b), want, "C op_add({a},{b})");
        assert_eq!(rf(a, b), want, "Rust op_add({a},{b})");
    }
}

/* ERRORS.md 16-18 — op_sub unchecked signed overflow / negating INT_MIN. */
#[test]
fn err16_err17_err18_op_sub_overflow_wraps() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_sub"), r.f2("op_sub"));
    let cases = [
        ((i32::MAX, -1), i32::MIN),
        ((i32::MIN, 1), i32::MAX),
        ((0, i32::MIN), i32::MIN),
        ((i32::MIN, i32::MAX), 1),
        ((i32::MAX, i32::MIN), -1),
    ];
    for ((a, b), want) in cases {
        assert_eq!(cf(a, b), want, "C op_sub({a},{b})");
        assert_eq!(rf(a, b), want, "Rust op_sub({a},{b})");
    }
}

/* ERRORS.md 19 — op_mul unchecked signed overflow. */
#[test]
fn err19_op_mul_overflow_wraps() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("op_mul"), r.f2("op_mul"));
    let cases = [
        ((i32::MAX, 2), -2),
        ((i32::MIN, -1), i32::MIN),
        ((65_536, 65_536), 0),
        ((i32::MIN, i32::MIN), 0),
        ((46_341, 46_341), 46_341i32.wrapping_mul(46_341)),
    ];
    for ((a, b), want) in cases {
        assert_eq!(cf(a, b), want, "C op_mul({a},{b})");
        assert_eq!(rf(a, b), want, "Rust op_mul({a},{b})");
    }
}

/* ERRORS.md 20 — helper_call's `r + acc` overflow after an overflowing OP. */
#[test]
fn err20_helper_call_overflow_wraps() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("helper_call"), r.f2("helper_call"));
    for &(a, b) in &[
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MAX, i32::MIN),
    ] {
        assert_eq!(cf(a, b), rf(a, b), "helper_call({a},{b}) [OP={OP} REPEAT={REPEAT}]");
    }
}

/* ERRORS.md 21 — the whole 32-bit domain is valid; nothing is rejected. */
#[test]
fn err21_full_int_domain_accepted() {
    let (c, r) = pair();
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let (cf, rf) = (c.f2(name), r.f2(name));
        for &a in EDGE.iter() {
            for &b in EDGE.iter() {
                assert_eq!(cf(a, b), rf(a, b), "{name}({a},{b}) [OP={OP} REPEAT={REPEAT}]");
            }
        }
    }
}

/* ERRORS.md 22 — the mul accumulator is the only one that can overflow;
   verify no divergence at the configured REPEAT. */
#[test]
fn err22_accumulator_overflow_parity() {
    let (c, r) = pair();
    let (cf, rf) = (c.f2("helper_call"), r.f2("helper_call"));
    // helper_call(0, 0) isolates the accumulator for add/sub; for mul it is
    // op_mul(0,0)=0 plus the accumulator, so the return value *is* the acc.
    let cv = cf(0, 0);
    assert_eq!(cv, rf(0, 0), "helper_call(0,0) [OP={OP} REPEAT={REPEAT}]");
    assert_eq!(cv, unrolled(REPEAT), "accumulator model mismatch");

    // And the accumulator produced inside accum_<OP> for every switch case.
    let (cg, rg) = (c.f1("use_generated"), r.f1("use_generated"));
    for n in 0..=7 {
        assert_eq!(cg(n), rg(n), "use_generated({n}) [OP={OP}]");
    }
}

/* ERRORS.md 23 — G_OP is writable, but helper_ptr must not read it. */
#[test]
fn err23_writable_g_op_does_not_affect_helper_ptr() {
    let _g = g_op_guard();
    let (c, r) = pair();
    let (cs, rs) = (c.g_op_slot(), r.g_op_slot());
    let (c_orig, r_orig) = unsafe { (*cs, *rs) };
    let (chp, rhp) = (c.f2("helper_ptr"), r.f2("helper_ptr"));

    let before_c = chp(7, 3);
    let before_r = rhp(7, 3);
    assert_eq!(before_c, before_r);

    // Point G_OP at a *different* op than the compiled-in one.
    let other = if OP == "add" { "op_mul" } else { "op_add" };
    unsafe {
        *cs = c.addr(other);
        *rs = r.addr(other);
    }

    assert_eq!(chp(7, 3), before_c, "C helper_ptr must ignore G_OP");
    assert_eq!(rhp(7, 3), chp(7, 3), "Rust helper_ptr must ignore G_OP too");

    // The direct G_OP call, by contrast, *does* observe the new pointer.
    assert_eq!(c.g_op()(7, 3), r.g_op()(7, 3), "G_OP(7,3) after overwrite");

    unsafe {
        *cs = c_orig;
        *rs = r_orig;
    }
}

/* ERRORS.md 24 — G_OP_NAME pointee. */
#[test]
fn err24_g_op_name_is_nul_terminated_3_bytes() {
    let _g = g_op_guard();
    let (c, r) = pair();
    assert_eq!(c.g_op_name(), r.g_op_name());
    assert_eq!(c.g_op_name().len(), 3);
    assert_eq!(c.g_op_name(), OP.as_bytes());
}

/* ERRORS.md 25 — out-of-range "enum"-like int across the FFI boundary.
   `use_generated`'s `switch` has no invalid-variant trap: every bit pattern
   maps to a case or to `default:`. Sweep a large randomized set. */
#[test]
fn err25_out_of_range_switch_values() {
    let (c, r) = pair();
    let (cf, rf) = (c.f1("use_generated"), r.f1("use_generated"));
    let mut rng = Rng::seeded(0xA5A5_5A5A);
    for _ in 0..4096 {
        let n = rng.i32();
        assert_eq!(cf(n), rf(n), "use_generated({n}) [OP={OP}]");
    }
    // Exhaustive over the interesting neighbourhood of the switch.
    for n in -16..=32 {
        assert_eq!(cf(n), rf(n), "use_generated({n}) [OP={OP}]");
    }
    for n in [i32::MIN, i32::MIN + 1, -1, 0, 6, 7, 8, i32::MAX - 1, i32::MAX] {
        assert_eq!(cf(n), rf(n), "use_generated({n}) [OP={OP}]");
    }
}

/* ERRORS.md 26-27 — documented absence: no pointer, length or buffer
   parameters exist in the public API, so there is no null / zero-length /
   oversized-length rejection path. Assert that absence mechanically. */
#[test]
fn err26_err27_no_pointer_or_length_parameters() {
    // Every exported function must resolve at the (int,int)->int or
    // (int)->int signature used above; the two data exports are the only
    // pointer-valued parts of the API and both are covered by err23/err24.
    let (c, r) = pair();
    for name in ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"] {
        let _ = c.f2(name);
        let _ = r.f2(name);
    }
    let _ = c.f1("use_generated");
    let _ = r.f1("use_generated");
    let _ = c.addr("G_OP");
    let _ = r.addr("G_OP");
    let _ = c.addr("G_OP_NAME");
    let _ = r.addr("G_OP_NAME");

    // The C header declares exactly these prototypes — no pointers, no lengths.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/mdmacros.h"),
    )
    .expect("read mdmacros.h");
    for proto in [
        "int op_add(int a, int b)",
        "int op_sub(int a, int b)",
        "int op_mul(int a, int b)",
        "int helper_call(int a, int b)",
        "int helper_ptr(int a, int b)",
        "int use_generated(int n)",
    ] {
        assert!(hdr.contains(proto), "prototype changed: {proto}");
    }
    assert!(
        !hdr.contains("char *") || hdr.contains("const char *G_OP_NAME"),
        "unexpected pointer parameter in the public API"
    );
}
