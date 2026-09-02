//! CONFIGS.md row C1 — pristine-process test.
//!
//! This file deliberately contains EXACTLY ONE test so that it is the first and
//! only thing to touch either library's function-local `static int sum` in this
//! process. That is the only way to assert the zero initialisation the C
//! standard guarantees. (Cargo builds one test binary per file in `tests/`, so
//! this test gets its own process.)

mod common;

use common::with_libs;

#[test]
fn c1_accumulator_starts_at_zero_in_both_libraries() {
    with_libs(|h| {
        // The very first observation of the accumulator in this process.
        let v = h.static_sum(0, "C1");
        assert_eq!(v, 0, "C1: `static int sum = 0` must start at 0 in both libs");

        // First real update from the pristine state.
        let v = h.static_sum(5, "C1");
        assert_eq!(v, 5, "C1: first update must yield exactly the update value");

        // The accumulator is now 5 in both libraries; keep going in lockstep.
        let v = h.static_sum(-5, "C1");
        assert_eq!(v, 0, "C1: accumulator must return to 0");
    });
}
