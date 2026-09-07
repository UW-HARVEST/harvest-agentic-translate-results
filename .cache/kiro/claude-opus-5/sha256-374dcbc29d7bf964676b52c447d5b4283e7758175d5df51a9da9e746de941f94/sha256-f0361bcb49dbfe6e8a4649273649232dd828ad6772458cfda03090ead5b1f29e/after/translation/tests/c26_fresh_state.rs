//! `CONFIGS.md` row C26 — the *very first* call after loading.
//!
//! This lives in its own test binary because the accumulator is process-wide
//! and never resettable: only the first call in a fresh process can observe the
//! zero-initialized `static int sum`. This binary therefore contains exactly one
//! test.

mod common;

use common::harness;

#[test]
fn c26_fresh_accumulator_is_zero_in_both_libraries() {
    let mut h = harness();
    // Unmirrored on purpose: we want each library's own answer to its first-ever
    // call, to prove both start from a zero-initialized accumulator.
    let (c, r) = h.raw_first_call(0);
    assert_eq!(c, 0, "C: fresh accumulator must read 0");
    assert_eq!(r, 0, "Rust: fresh accumulator must read 0");
    assert_eq!(c, r);

    // And the first mutating call is likewise the identity on 0.
    assert_eq!(h.static_sum(5), 5);
    assert_eq!(h.static_sum(-5), 0);

    // A driver() run from the zero state prints the triangular numbers.
    let out = h.driver(1);
    assert_eq!(out, b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n");
}
