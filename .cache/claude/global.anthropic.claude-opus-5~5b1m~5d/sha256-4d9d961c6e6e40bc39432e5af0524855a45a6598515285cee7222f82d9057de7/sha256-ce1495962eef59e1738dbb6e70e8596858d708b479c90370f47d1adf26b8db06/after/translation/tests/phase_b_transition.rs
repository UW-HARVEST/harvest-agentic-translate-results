//! Phase B — CONFIGS.md row 23: the hidden static-state transition.
//!
//! This lives in its own integration-test binary (= its own process) so that
//! the libraries' `node_count`/`node_storage` are guaranteed pristine when the
//! first call is made. The SAME argument tuple must yield the not-found error
//! before `initialize_test_data()` and the computed value afterwards, in BOTH
//! libraries.

#![cfg(feature = "expose_init_test_data")]

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn cfg_row23_state_transition() {
    let p = Pair::with_init();

    let probes: [(c_int, c_int, c_int, c_int, c_int); 6] = [
        // (mode, node_id, depth, flags, expected-before-init)
        (0o1, 1, 5, 0, ERR_MODE1_NOT_FOUND),
        (0o1, 7, 3, 11, ERR_MODE1_NOT_FOUND),
        (0o2, 1, 0, 0, ERR_MODE2_NOT_FOUND),
        (0o2, 4, 9, -3, ERR_MODE2_NOT_FOUND),
        (0o4, 1, 0, 0, ERR_MODE4_NOT_FOUND),
        (0o4, 6, 2, 7, ERR_MODE4_NOT_FOUND),
    ];

    // --- before init: node_count == 0, so find_node_by_id always fails ---
    let mut before = Vec::new();
    for &(m, n, d, f, expected) in &probes {
        before.push(p.assert_same_eq(m, n, d, f, expected));
    }
    // Ids that never exist behave identically before init.
    p.assert_same_eq(0o1, 999, 4, 0, ERR_MODE1_NOT_FOUND);
    p.assert_same_eq(0o2, 999, 4, 0, ERR_MODE2_NOT_FOUND);
    p.assert_same_eq(0o4, 999, 4, 0, ERR_MODE4_NOT_FOUND);

    // Case 0003 is state-independent; record it to prove it does NOT change.
    let mode3_before = p.assert_same_eq(0o3, 42, -42, 99, expect_mode3(42, -42, 99));

    // --- transition ---
    p.init_both();

    // --- after init: the very same tuples now take the computed path ---
    for (i, &(m, n, d, f, _)) in probes.iter().enumerate() {
        let after = p.assert_same(m, n, d, f);
        assert_ne!(
            after, before[i],
            "jumpnode({m},{n},{d},{f}) did not change after initialize_test_data \
             (before={}, after={after}) — the static state transition was not observed",
            before[i]
        );
    }

    // Case 0003 must be unaffected by the state change.
    p.assert_same_eq(0o3, 42, -42, 99, mode3_before);

    // Concrete values, straight from the C source.
    p.assert_same_eq(0o1, 1, 5, 0, 100); // root 100.5 -> 100
    p.assert_same_eq(0o2, 1, 0, 0, 1438); // full 16-wide backward sum

    // A node id that never exists keeps giving the same error after init.
    p.assert_same_eq(0o1, 999, 4, 0, ERR_MODE1_NOT_FOUND);
    p.assert_same_eq(0o2, 999, 4, 0, ERR_MODE2_NOT_FOUND);
    p.assert_same_eq(0o4, 999, 4, 0, ERR_MODE4_NOT_FOUND);
}
