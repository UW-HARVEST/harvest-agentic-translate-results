//! Phase B — valid-path differential tests, rows 1..21 of `CONFIGS.md`.
//!
//! Both implementations are driven only through `dlopen`/`dlsym`, so the Rust
//! `#[unsafe(no_mangle)]` export wrappers are part of what is being compared.
//! Lowest-level entry points first (`op_add`/`op_sub`/`op_mul`), then the
//! function-pointer helper, then the composed helper, then the macro-generated
//! accumulator driver.

mod common;

use std::ffi::c_int;

use common::*;

/* ---------------- rows 1-3: op_add (lowest level) ---------------- */

#[test]
fn r01_op_add_small() {
    diff_binop("op_add", SMALL_PAIRS.iter().copied());
}

#[test]
fn r02_op_add_random() {
    diff_binop("op_add", random_pairs(SEED, RANDOM_CASES));
}

#[test]
fn r03_op_add_boundary() {
    diff_binop("op_add", BOUNDARY_PAIRS.iter().copied());
}

/* ---------------- rows 4-6: op_sub ---------------- */

#[test]
fn r04_op_sub_small() {
    diff_binop("op_sub", SMALL_PAIRS.iter().copied());
}

#[test]
fn r05_op_sub_random() {
    diff_binop("op_sub", random_pairs(SEED ^ 0x11, RANDOM_CASES));
}

#[test]
fn r06_op_sub_boundary() {
    diff_binop("op_sub", BOUNDARY_PAIRS.iter().copied());
}

/* ---------------- rows 7-9: op_mul ---------------- */

#[test]
fn r07_op_mul_small() {
    diff_binop("op_mul", SMALL_PAIRS.iter().copied());
}

#[test]
fn r08_op_mul_random() {
    diff_binop("op_mul", random_pairs(SEED ^ 0x22, RANDOM_CASES));
}

#[test]
fn r09_op_mul_boundary() {
    diff_binop("op_mul", BOUNDARY_PAIRS.iter().copied());
}

/* ---------------- rows 10-12: helper_ptr (indirect call) ---------------- */

#[test]
fn r10_helper_ptr_small() {
    diff_binop("helper_ptr", SMALL_PAIRS.iter().copied());
}

#[test]
fn r11_helper_ptr_random() {
    diff_binop("helper_ptr", random_pairs(SEED ^ 0x33, RANDOM_CASES));
}

#[test]
fn r12_helper_ptr_boundary() {
    diff_binop("helper_ptr", BOUNDARY_PAIRS.iter().copied());
}

/* ---------------- rows 13-16: helper_call (OP x REPEAT) ---------------- */

#[test]
fn r13_helper_call_small() {
    diff_binop("helper_call", SMALL_PAIRS.iter().copied());
}

#[test]
fn r14_helper_call_random() {
    diff_binop("helper_call", random_pairs(SEED ^ 0x44, RANDOM_CASES));
}

#[test]
fn r15_helper_call_boundary() {
    diff_binop("helper_call", BOUNDARY_PAIRS.iter().copied());
}

/// Row 16 — pin the unrolled `RUN_LOOP` accumulator on its own.
///
/// `helper_call(a,b)` is `op_<OP>(a,b) + acc`, where `acc` depends only on
/// `OP`/`REPEAT`. Recovering `acc` from each library and comparing it directly
/// isolates a wrong unroll depth (e.g. an off-by-one `REPn`) from a wrong
/// operation, and would flag it even if the sum happened to agree.
#[test]
fn r16_helper_call_accumulator_isolated() {
    let (c, r) = (c_impl(), rust_impl());
    let op_sym = format!("op_{OP}");

    let (c_op, r_op) = (c.binop(&op_sym), r.binop(&op_sym));
    let (c_hc, r_hc) = (c.binop("helper_call"), r.binop("helper_call"));

    let c_acc = unsafe { c_hc(0, 0).wrapping_sub(c_op(0, 0)) };
    let r_acc = unsafe { r_hc(0, 0).wrapping_sub(r_op(0, 0)) };
    assert_eq!(
        c_acc, r_acc,
        "[{}] RUN_LOOP accumulator differs: C={c_acc} Rust={r_acc}",
        config_label()
    );

    // Independent closed form of `INIT_<OP>` folded with `REPEAT` steps.
    let expected = {
        let mut acc: c_int = INIT;
        for i in 0..REPEAT {
            acc = match OP {
                "add" => acc.wrapping_add(i),
                "sub" => acc.wrapping_sub(i),
                _ => acc.wrapping_mul(i.wrapping_add(1)),
            };
        }
        acc
    };
    assert_eq!(c_acc, expected, "[{}] C accumulator is not INIT+REPEAT steps", config_label());

    // And the decomposition holds for arbitrary operands in both libraries.
    for (a, b) in all_pairs() {
        let (cv, rv) = unsafe { (c_hc(a, b), r_hc(a, b)) };
        assert_eq!(cv, rv, "[{}] helper_call({a}, {b})", config_label());
        let (co, ro) = unsafe { (c_op(a, b), r_op(a, b)) };
        assert_eq!(
            cv,
            co.wrapping_add(c_acc),
            "[{}] C helper_call({a}, {b}) != op_{OP} + acc",
            config_label()
        );
        assert_eq!(
            rv,
            ro.wrapping_add(r_acc),
            "[{}] Rust helper_call({a}, {b}) != op_{OP} + acc",
            config_label()
        );
    }
}

/* ---------------- rows 17-21: use_generated / accum_<OP> ---------------- */

/// Row 17 — one `DISPATCH_REP` `case` per value; asserted individually so a
/// failure names the exact case.
#[test]
fn r17_use_generated_each_case() {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.unop("use_generated"), r.unop("use_generated"));
    for &n in IN_RANGE_N {
        let (cv, rv) = unsafe { (cf(n), rf(n)) };
        assert_eq!(
            cv, rv,
            "[{}] use_generated({n}) (DISPATCH_REP case {n}): C={cv} Rust={rv}",
            config_label()
        );
    }
}

#[test]
fn r18_use_generated_seven_is_default_case() {
    diff_unop("use_generated", [7]);
}

#[test]
fn r19_use_generated_out_of_range() {
    diff_unop("use_generated", OUT_OF_RANGE_N.iter().copied());
}

#[test]
fn r20_use_generated_random() {
    diff_unop("use_generated", random_ints(SEED ^ 0x55, RANDOM_CASES));
}

/// Row 21 — exactly the call `main` makes. Notable for `REPEAT == 7`, where the
/// `switch` has no `case 7` and therefore returns `INIT_<OP>`.
#[test]
fn r21_use_generated_repeat() {
    diff_unop("use_generated", [REPEAT]);
}

/* ---------------- cross-checks over the whole surface ---------------- */

/// Every exported function, over the union of all input shapes, in one sweep.
/// Guards against a symbol that resolves but is wired to the wrong body.
#[test]
fn all_exports_agree_over_all_shapes() {
    for sym in ["op_add", "op_sub", "op_mul", "helper_ptr", "helper_call"] {
        diff_binop(sym, all_pairs());
    }
    let mut ns: Vec<c_int> = IN_RANGE_N.to_vec();
    ns.extend_from_slice(OUT_OF_RANGE_N);
    ns.extend(random_ints(SEED ^ 0x66, RANDOM_CASES));
    diff_unop("use_generated", ns);
}

/// The three leaf operations must be OP/REPEAT-independent: `op_add` is always
/// `a+b`, regardless of which `OP` the library was configured with.
#[test]
fn leaf_ops_are_configuration_independent() {
    let (c, r) = (c_impl(), rust_impl());
    for (sym, f) in [
        ("op_add", (|a: c_int, b: c_int| a.wrapping_add(b)) as fn(c_int, c_int) -> c_int),
        ("op_sub", |a, b| a.wrapping_sub(b)),
        ("op_mul", |a, b| a.wrapping_mul(b)),
    ] {
        let (cf, rf) = (c.binop(sym), r.binop(sym));
        for (a, b) in all_pairs() {
            let want = f(a, b);
            let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
            assert_eq!(cv, want, "[{}] C {sym}({a},{b})={cv}, want {want}", config_label());
            assert_eq!(rv, want, "[{}] Rust {sym}({a},{b})={rv}, want {want}", config_label());
        }
    }
}
