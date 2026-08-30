//! Level 1 — the leaf operations `op_add`, `op_sub`, `op_mul`.
//!
//! These are configuration independent: `mdcore.c` always defines all three,
//! regardless of `-DOP`. Both `.so`s are exercised over the full cross-product
//! of interesting operands, including the values where the C arithmetic wraps.

mod common;

use common::{Impl, operand_pairs, operands};

#[test]
fn op_add_matches() {
    let (c, r) = Impl::pair();
    let (cf, rf) = (c.fn2("op_add"), r.fn2("op_add"));
    for (a, b) in operand_pairs() {
        assert_eq!(cf(a, b), rf(a, b), "op_add({a}, {b})");
    }
}

#[test]
fn op_sub_matches() {
    let (c, r) = Impl::pair();
    let (cf, rf) = (c.fn2("op_sub"), r.fn2("op_sub"));
    for (a, b) in operand_pairs() {
        assert_eq!(cf(a, b), rf(a, b), "op_sub({a}, {b})");
    }
}

#[test]
fn op_mul_matches() {
    let (c, r) = Impl::pair();
    let (cf, rf) = (c.fn2("op_mul"), r.fn2("op_mul"));
    for (a, b) in operand_pairs() {
        assert_eq!(cf(a, b), rf(a, b), "op_mul({a}, {b})");
    }
}

/// A denser sweep on one operand axis to catch off-by-one style divergence.
#[test]
fn ops_match_over_dense_range() {
    let (c, r) = Impl::pair();
    for name in ["op_add", "op_sub", "op_mul"] {
        let (cf, rf) = (c.fn2(name), r.fn2(name));
        for a in -600..=600 {
            for &b in &operands() {
                assert_eq!(cf(a, b), rf(a, b), "{name}({a}, {b})");
                assert_eq!(cf(b, a), rf(b, a), "{name}({b}, {a})");
            }
        }
    }
}
