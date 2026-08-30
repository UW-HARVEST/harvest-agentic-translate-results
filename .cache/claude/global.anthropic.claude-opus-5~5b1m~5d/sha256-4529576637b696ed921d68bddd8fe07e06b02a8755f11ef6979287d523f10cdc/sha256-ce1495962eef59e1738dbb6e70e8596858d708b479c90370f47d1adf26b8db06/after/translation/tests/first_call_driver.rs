//! CONFIGS.md row 22 — `driver` as the first symbol ever resolved and called.
//!
//! Own test binary (= own process) so the global is pristine and `driver`
//! runs before `run` is ever called directly. This pins down the wrapper's
//! composition (`run(x); run(x);`) against the static initialiser.
//!
//! Exactly ONE `#[test]` on purpose — see `first_call_run.rs`.

mod common;
use common::*;

#[test]
fn row22_pristine_state_first_ever_call_to_driver() {
    let mut h = lock();
    assert_eq!(h.floors(), 2);
    assert_eq!(h.bedrooms(), 5);
    assert_eq!(h.bathrooms(), 2.5);

    // Use a non-zero argument so the "applied twice" behaviour is visible.
    let out = h.driver(10, "row22 pristine driver");
    let s = String::from_utf8(out).unwrap();

    let expected = [
        // --- first run(10) ---
        "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms",
        "The house has 3 floors, 5 bedrooms, and 2.5 bathrooms",
        "The house has 3 floors, 5 bedrooms, and 3.5 bathrooms",
        "The house has 3 floors, 15 bedrooms, and 3.5 bathrooms",
        // --- second run(10) ---
        "The house has 3 floors, 15 bedrooms, and 3.5 bathrooms",
        "The house has 4 floors, 15 bedrooms, and 3.5 bathrooms",
        "The house has 4 floors, 15 bedrooms, and 4.5 bathrooms",
        "The house has 4 floors, 25 bedrooms, and 4.5 bathrooms",
    ]
    .map(|l| format!("{l}\n"))
    .concat();

    assert_eq!(
        s, expected,
        "driver() on pristine state must match the C byte-for-byte"
    );

    assert_eq!(h.floors(), 4);
    assert_eq!(h.bedrooms(), 25);
    assert_eq!(h.bathrooms(), 4.5);
}
