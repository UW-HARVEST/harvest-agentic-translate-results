//! CONFIGS.md row 1 — pristine global state.
//!
//! `the_house` is a file-scope global with no reset entry point, so its
//! initial value `{floors=2, bedrooms=5, bathrooms=2.5}` is observable only on
//! the very first call made in a fresh process. This is its own test binary
//! (= its own process) so that `run` is the first symbol ever invoked.
//!
//! There is exactly ONE `#[test]` here on purpose: a second test in this
//! binary could run first and consume the pristine state.

mod common;
use common::*;

/// Row 1 — `run(0)` as the first-ever call: verifies the static initialiser
/// is byte-identical between C and Rust.
#[test]
fn row1_pristine_state_first_ever_call_to_run() {
    let mut h = lock();
    assert_eq!(h.floors(), 2);
    assert_eq!(h.bedrooms(), 5);
    assert_eq!(h.bathrooms(), 2.5);

    let out = h.run(0, "row1 pristine");
    let s = String::from_utf8(out).unwrap();

    // The exact bytes the C static initialiser produces.
    assert_eq!(
        s,
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
         The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n",
        "pristine-state output must match the C static initialiser exactly"
    );

    // Post-conditions of one `run`: +1 floor, +1.0 bathrooms, bedrooms unchanged.
    assert_eq!(h.floors(), 3);
    assert_eq!(h.bedrooms(), 5);
    assert_eq!(h.bathrooms(), 3.5);
}
