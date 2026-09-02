// Harness self-checks: symbol presence, and the assumption that a private
// `.so` copy gets private (freshly initialised) hidden statics.

mod common;
use common::*;

#[test]
fn both_sos_export_all_eight_symbols() {
    let p = LibPair::fresh("smoke");
    // Binding panics with a clear message if any symbol is missing.
    let (_c, _r) = p.apis();
}

#[test]
fn fresh_pair_starts_from_initial_static_state() {
    // accumulator starts at 0, multiplier at 1, operation_count at 0.
    // add_to_accumulator(0,0) on a fresh library must return 0.
    for i in 0..4 {
        let p = LibPair::fresh(&format!("freshstate{i}"));
        let (c, r) = p.apis();
        let cv = unsafe { (c.add_to_accumulator)(0, 0) };
        let rv = unsafe { (r.add_to_accumulator)(0, 0) };
        assert_eq!(cv, 0, "C accumulator was not fresh on iteration {i}");
        assert_eq!(rv, 0, "Rust accumulator was not fresh on iteration {i}");

        let cm = unsafe { (c.multiply_with_multiplier)(1, 1) };
        let rm = unsafe { (r.multiply_with_multiplier)(1, 1) };
        assert_eq!(cm, 1, "C multiplier was not fresh on iteration {i}");
        assert_eq!(rm, 1, "Rust multiplier was not fresh on iteration {i}");
    }
}

#[test]
fn findrep_first_call_matches() {
    let p = LibPair::fresh("firstcall");
    let (c, r) = p.apis();
    let cv = unsafe { (c.findrep)(1, 2, 3, 4) };
    let rv = unsafe { (r.findrep)(1, 2, 3, 4) };
    assert_eq!(cv, rv, "findrep(1,2,3,4) first call: C={cv} Rust={rv}");
}
