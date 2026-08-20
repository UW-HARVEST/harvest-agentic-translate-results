// Phase B — valid-path differential tests for the leaf entry points.
// CONFIGS.md rows C1 .. C12.
//
// Every call goes through both `.so` files' exported symbols via libloading.

mod common;

use common::*;
use std::ffi::c_char;

const SAMPLES: usize = 512;

/// The `{0, ±1, ±2, INT_MIN, INT_MAX, ...}` matrix referenced by C4..C8.
fn matrix() -> Vec<i32> {
    BOUNDARY.to_vec()
}

// ---------------------------------------------------------------------------
// C1 — is_valid_operation, exhaustive over all 256 char bit patterns.
// ---------------------------------------------------------------------------

#[test]
fn c1_is_valid_operation_exhaustive() {
    let (c, r) = both();
    for v in i8::MIN..=i8::MAX {
        let ch = v as c_char;
        let cv = unsafe { (c.is_valid_operation)(ch) };
        let rv = unsafe { (r.is_valid_operation)(ch) };
        assert_eq!(cv, rv, "is_valid_operation({v}) C={cv} Rust={rv}");
        // The byte must be a canonical _Bool as well.
        assert!(cv <= 1, "C returned non-boolean byte {cv} for {v}");
        assert!(rv <= 1, "Rust returned non-boolean byte {rv} for {v}");
    }
}

// ---------------------------------------------------------------------------
// C2 / C3 — get_operation_priority: exhaustive small sweep + overflow region.
// ---------------------------------------------------------------------------

#[test]
fn c2_get_operation_priority_small_sweep() {
    let (c, r) = both();
    for op in -16..=16 {
        let cv = unsafe { (c.get_operation_priority)(op) };
        let rv = unsafe { (r.get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
}

#[test]
fn c3_get_operation_priority_overflow_and_random() {
    let (c, r) = both();
    let mut fixed = matrix();
    fixed.extend_from_slice(&[
        i32::MAX / 10,
        i32::MAX / 10 + 1,
        i32::MAX / 10 - 1,
        i32::MIN / 10,
        i32::MIN / 10 - 1,
        i32::MIN / 10 + 1,
        214748364,
        -214748364,
    ]);
    for &op in &fixed {
        let cv = unsafe { (c.get_operation_priority)(op) };
        let rv = unsafe { (r.get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
    let mut rng = Rng::new(0xC3_0000_0001);
    for _ in 0..SAMPLES * 4 {
        let op = rng.spicy_i32();
        let cv = unsafe { (c.get_operation_priority)(op) };
        let rv = unsafe { (r.get_operation_priority)(op) };
        assert_eq!(cv, rv, "get_operation_priority({op})");
    }
}

// ---------------------------------------------------------------------------
// C4..C9 — the five arithmetic ops.
// ---------------------------------------------------------------------------

/// Drive one op index over the boundary matrix and randomized inputs.
/// `idx`: 0 add, 1 mul, 2 sub, 3 div, 4 mod.
fn diff_binop(idx: usize, seed: u64, skip_ub: bool) {
    let (c, r) = both();
    let cf = c.op_by_index(idx);
    let rf = r.op_by_index(idx);
    let m = matrix();

    // Full boundary cross-product, with `unused_param` varied too (C9).
    let unused = [0i32, 1, -1, i32::MIN, i32::MAX, 7];
    let mut k = 0usize;
    for &a in &m {
        for &b in &m {
            if skip_ub && is_idiv_ub(a, b) {
                continue;
            }
            let u = unused[k % unused.len()];
            k += 1;
            let cv = unsafe { cf(a, b, u) };
            let rv = unsafe { rf(a, b, u) };
            assert_eq!(cv, rv, "op#{idx}({a}, {b}, {u}) C={cv} Rust={rv}");
        }
    }

    // Randomized, property-style.
    let mut rng = Rng::new(seed);
    for _ in 0..SAMPLES * 8 {
        let a = rng.spicy_i32();
        let b = rng.spicy_i32();
        if skip_ub && is_idiv_ub(a, b) {
            continue;
        }
        let u = rng.spicy_i32();
        let cv = unsafe { cf(a, b, u) };
        let rv = unsafe { rf(a, b, u) };
        assert_eq!(cv, rv, "op#{idx}({a}, {b}, {u}) C={cv} Rust={rv}");
    }
}

#[test]
fn c4_add_operation() {
    diff_binop(0, 0xADD_0001, false);
}

#[test]
fn c5_multiply_operation() {
    diff_binop(1, 0x111_0002, false);
}

#[test]
fn c6_subtract_operation() {
    diff_binop(2, 0x5B_0003, false);
}

#[test]
fn c7_divide_operation() {
    diff_binop(3, 0xD1D_0004, true);
}

#[test]
fn c8_modulo_operation() {
    diff_binop(4, 0x0D_0005, true);
}

/// C9 — `unused_param` must be ignored identically: for every op, holding
/// `(a, b)` fixed and sweeping the third argument must leave the result
/// unchanged in BOTH libraries (it is a real ABI slot, so a translation that
/// accidentally read it would show up here).
#[test]
fn c9_unused_param_is_ignored() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC9_0009);
    for idx in 0..5 {
        let cf = c.op_by_index(idx);
        let rf = r.op_by_index(idx);
        let is_div_like = idx == 3 || idx == 4;
        for _ in 0..64 {
            let a = rng.spicy_i32();
            let mut b = rng.spicy_i32();
            if is_div_like && is_idiv_ub(a, b) {
                b = 3;
            }
            let mut baseline: Option<i32> = None;
            for &u in &[0i32, 1, -1, 2, -2, 12345, i32::MIN, i32::MAX, rng.i32()] {
                let cv = unsafe { cf(a, b, u) };
                let rv = unsafe { rf(a, b, u) };
                assert_eq!(cv, rv, "op#{idx}({a},{b},{u})");
                match baseline {
                    None => baseline = Some(cv),
                    Some(x) => assert_eq!(
                        x, cv,
                        "op#{idx}({a},{b},_) must not depend on unused_param (u={u})"
                    ),
                }
            }
        }
    }
}

/// C7/C8 detail: truncation direction for mixed signs must match C exactly.
#[test]
fn c7_c8_truncation_semantics() {
    let (c, r) = both();
    let cases = [
        (-7, 2),
        (7, -2),
        (-7, -2),
        (7, 2),
        (-3, 5),
        (3, -5),
        (i32::MIN, 2),
        (i32::MIN, 3),
        (i32::MAX, -3),
        (i32::MIN + 1, -1),
    ];
    for &(a, b) in &cases {
        let cd = unsafe { (c.divide_operation)(a, b, 0) };
        let rd = unsafe { (r.divide_operation)(a, b, 0) };
        assert_eq!(cd, rd, "divide_operation({a},{b})");
        let cm = unsafe { (c.modulo_operation)(a, b, 0) };
        let rm = unsafe { (r.modulo_operation)(a, b, 0) };
        assert_eq!(cm, rm, "modulo_operation({a},{b})");
    }
}

// ---------------------------------------------------------------------------
// C10 — select_operation: which function was selected (identity, not value).
// ---------------------------------------------------------------------------

fn selected_index(api: &Api, op: i32) -> usize {
    let addr = unsafe { (api.select_operation)(op) };
    api.identify_op(addr).unwrap_or_else(|| {
        panic!(
            "{} select_operation({op}) returned {addr:#x}, which is none of its five \
             exported op symbols {:#x?}",
            api.name, api.op_addrs
        )
    })
}

#[test]
fn c10_select_operation_identity() {
    let (c, r) = both();
    let mut ops: Vec<i32> = (-16..=16).collect();
    ops.extend_from_slice(&matrix());
    for &op in &ops {
        let ci = selected_index(c, op);
        let ri = selected_index(r, op);
        assert_eq!(ci, ri, "select_operation({op}) picked C#{ci} but Rust#{ri}");
    }
    // Documented mapping, so a matching-but-wrong pair would still be caught.
    assert_eq!(selected_index(c, OP_ADD), 0);
    assert_eq!(selected_index(c, OP_MULTIPLY), 1);
    assert_eq!(selected_index(c, OP_SUBTRACT), 2);
    assert_eq!(selected_index(c, OP_DIVIDE), 3);
    assert_eq!(selected_index(c, OP_MODULO), 4);
    assert_eq!(selected_index(c, 0), 0, "default: -> add_operation");
    assert_eq!(selected_index(c, 6), 0, "default: -> add_operation");

    let mut rng = Rng::new(0x5E1EC_0010);
    for _ in 0..SAMPLES {
        let op = rng.spicy_i32();
        assert_eq!(
            selected_index(c, op),
            selected_index(r, op),
            "select_operation({op})"
        );
    }
}

// ---------------------------------------------------------------------------
// C11 — invoke the pointer select_operation handed back.
// ---------------------------------------------------------------------------

#[test]
fn c11_dispatch_through_selected_pointer() {
    let (c, r) = both();
    let mut ops: Vec<i32> = (-8..=8).collect();
    ops.extend_from_slice(&[i32::MIN, i32::MAX, 100, -100]);
    let m = matrix();
    let mut rng = Rng::new(0xD15_0011);

    for &op in &ops {
        let ci = selected_index(c, op);
        let cf = c.op_by_index(ci);
        let rf = r.op_by_index(selected_index(r, op));
        let is_div_like = ci == 3 || ci == 4;

        for &a in &m {
            for &b in &m {
                if is_div_like && is_idiv_ub(a, b) {
                    continue;
                }
                let cv = unsafe { cf(a, b, 0) };
                let rv = unsafe { rf(a, b, 0) };
                assert_eq!(cv, rv, "dispatch op={op} ({a},{b})");
            }
        }
        for _ in 0..SAMPLES {
            let a = rng.spicy_i32();
            let b = rng.spicy_i32();
            if is_div_like && is_idiv_ub(a, b) {
                continue;
            }
            let cv = unsafe { cf(a, b, 0) };
            let rv = unsafe { rf(a, b, 0) };
            assert_eq!(cv, rv, "dispatch op={op} ({a},{b})");
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — get_computation_timestamp.
// ---------------------------------------------------------------------------

#[test]
fn c12_get_computation_timestamp() {
    let (c, r) = both();
    for _ in 0..64 {
        let before = now() >> 29;
        let cv = unsafe { (c.get_computation_timestamp)() };
        let rv = unsafe { (r.get_computation_timestamp)() };
        let after = now() >> 29;
        // Identical unless the (≈17 year) shift boundary was crossed mid-test.
        if before == after {
            assert_eq!(cv, rv, "get_computation_timestamp");
            assert_eq!(cv, before, "value must be time() >> 29 (arithmetic shift)");
        }
    }
}
