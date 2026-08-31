//! Sanity checks for the harness itself: both shared objects must load, export
//! `driver`, and produce identical, well-formed output.

mod common;

use common::*;

#[test]
fn both_libraries_export_driver() {
    assert_driver_symbol_exported(&c_library_path());
    assert_driver_symbol_exported(&rust_library_path());
}

#[test]
fn capture_returns_the_expected_shape() {
    let cs = String::from_utf8(c_output(1.0)).expect("C output is UTF-8");
    let rs = String::from_utf8(rust_output(1.0)).expect("Rust output is UTF-8");

    assert_eq!(cs, "3ff0000000000000 0x1p+0 1.0000\n", "C output changed");
    assert_eq!(rs, cs);
}

#[test]
fn batching_matches_individual_calls() {
    let inputs = [0.0, -0.0, 1.5, -2.25, f64::NAN, f64::INFINITY];
    let batched = c_run(&inputs);
    let mut concatenated = Vec::new();
    for f in inputs {
        concatenated.extend_from_slice(&c_output(f));
    }
    assert_eq!(batched, concatenated, "batching changes the captured bytes");
}
