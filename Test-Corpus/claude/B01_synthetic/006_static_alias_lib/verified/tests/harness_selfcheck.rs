// Validates the harness's own state-control primitives against the C library.
//
// These primitives (`probe`, `set_inner`) are what makes the rest of the suite
// able to reach configurations like `inner == 0` / `INT_MIN`, so they are
// checked against the C ground truth first.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn selfcheck_probe_is_non_mutating() {
    let mut h = harness();
    let a = h.probe();
    let b = h.probe();
    let c = h.probe();
    assert_eq!((a, b), (c, c), "probe() must not disturb inner");
}

#[test]
fn selfcheck_then_branch_updates_inner_by_outer() {
    let mut h = harness();
    let before = h.probe();
    // Choose a value >= inner so the then branch is taken.
    let v = before.wrapping_abs().max(1);
    let v = if v >= before { v } else { before };
    let o = h.sa(v);
    assert_eq!(o.cls, Cls::Inner);
    assert_eq!(o.ret_val, before.wrapping_add(v));
    assert_eq!(h.probe(), before.wrapping_add(v));
}

#[test]
fn selfcheck_set_inner_reaches_targets() {
    let mut h = harness();
    // Ordinary values, boundary values, and values that require the wrap route.
    for t in [7, 1, 0, -1, -12345, c_int::MAX, 1 << 30, 2, 0, 100, -100] {
        h.set_inner(t);
        assert_eq!(h.probe(), t, "set_inner({t})");
    }
    // INT_MIN cannot be probed without mutating it (every value satisfies
    // `>= INT_MIN`), so verify it by its unique observable consequence: the
    // then branch is taken even for INT_MIN, leaving inner == 0.
    h.set_inner(c_int::MIN);
    let o = h.sa_np(c_int::MIN);
    assert_eq!(o.cls, Cls::Inner, "inner==INT_MIN: then branch must be taken");
    assert_eq!(o.ret_val, 0, "INT_MIN + INT_MIN wraps to 0");
    assert_eq!(h.probe(), 0);
    h.set_inner(1); // leave a tidy state
}

#[test]
fn selfcheck_inner_addr_is_stable_and_aliasing_is_observable() {
    let mut h = harness();
    h.set_inner(3);
    // Distinct pointer, value below inner -> else branch returns caller's ptr.
    let o = h.sa(2);
    assert_eq!(o.cls, Cls::Outer);
    assert_eq!(o.buf_after, 5);
    assert_eq!(o.ret_val, 5);
    assert_eq!(h.probe(), 3, "else branch must leave inner alone");

    // Aliased call doubles inner and returns &inner.
    let o = h.sa_aliased();
    assert_eq!(o.cls, Cls::Inner);
    assert_eq!(o.ret_val, 6);
    assert_eq!(h.probe(), 6);
    h.set_inner(1);
}

#[test]
fn selfcheck_driver_capture_roundtrip() {
    let mut h = harness();
    h.set_inner(1);
    // inner=1, initial=5: step1 5>=1 -> inner=6, print 6; step2 aliased 6>=6
    // -> inner=12, print 12.
    let out = h.driver(5, 2);
    assert_eq!(out, expect_lines(&[6, 12]), "captured bytes");
    assert_eq!(h.probe(), 12);
    h.set_inner(1);
}

#[test]
fn selfcheck_chain_matches_driver_values() {
    let mut h = harness();
    h.set_inner(1);
    let steps = h.chain(5, 2);
    let vals: Vec<c_int> = steps.iter().map(|s| s.val).collect();
    assert_eq!(vals, vec![6, 12]);
    h.set_inner(1);
}
