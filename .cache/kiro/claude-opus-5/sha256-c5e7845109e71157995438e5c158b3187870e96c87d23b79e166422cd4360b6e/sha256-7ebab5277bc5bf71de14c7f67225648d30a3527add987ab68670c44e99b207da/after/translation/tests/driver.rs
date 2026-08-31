//! Differential tests for `void driver(int x)`.
//!
//! `driver` is the entire public API of the library (see `c_src/include/driver.h`),
//! so there is a single level in the call hierarchy: every test below drives it
//! through the `.so` exports of both implementations and compares stdout.

mod common;

use common::{assert_driver_matches, capture_stdout, libs};

#[test]
fn both_libraries_export_driver() {
    // Panics inside `libs()` if either `dlsym("driver")` fails.
    let l = libs();
    assert!(!(l.c_driver as usize == 0));
    assert!(!(l.rust_driver as usize == 0));
}

#[test]
fn zero_and_negative_produce_no_output() {
    for x in [0, -1, -2, -7, -100, -1000, i32::MIN, i32::MIN + 1] {
        assert_driver_matches(x);
        let out = capture_stdout(|| unsafe { (libs().c_driver)(x) });
        assert!(
            out.is_empty(),
            "C driver({x}) unexpectedly produced output: {out:?}"
        );
    }
}

#[test]
fn small_positive_values() {
    for x in 1..=64 {
        assert_driver_matches(x);
    }
}

#[test]
fn exhaustive_small_range() {
    // Covers every loop count up to 512, including the transitions where the
    // printed widths of `i` and `j` change (9->10, 99->100, ...).
    for x in 0..=512 {
        assert_driver_matches(x);
    }
}

#[test]
fn digit_width_boundaries() {
    // `j` is `2*i`, so its width flips at half the `i` boundaries.
    for x in [
        4, 5, 6, 9, 10, 11, 49, 50, 51, 99, 100, 101, 499, 500, 501, 999, 1000, 1001, 4999, 5000,
        5001,
    ] {
        assert_driver_matches(x);
    }
}

#[test]
fn larger_values() {
    for x in [2_048, 10_000, 65_536, 100_000] {
        assert_driver_matches(x);
    }
}

#[test]
fn output_format_matches_expected_text() {
    // Independent check that the shared behaviour is the *C* behaviour, not a
    // mismatch that happens to agree.
    let x = 12;
    let expected: String = (0..x).map(|i| format!("{i} {}\n", i * 2)).collect();
    let l = libs();
    let c_out = capture_stdout(|| unsafe { (l.c_driver)(x) });
    let rust_out = capture_stdout(|| unsafe { (l.rust_driver)(x) });
    assert_eq!(c_out, expected.as_bytes(), "C output shape changed");
    assert_eq!(rust_out, expected.as_bytes(), "Rust output shape differs");
}

#[test]
fn repeated_and_interleaved_calls_agree() {
    // Successive calls must be stateless, and mixing the two libraries in one
    // captured region must not perturb libc stream state.
    let l = libs();
    for _ in 0..3 {
        let c_out = capture_stdout(|| unsafe {
            (l.c_driver)(3);
            (l.c_driver)(0);
            (l.c_driver)(5);
        });
        let rust_out = capture_stdout(|| unsafe {
            (l.rust_driver)(3);
            (l.rust_driver)(0);
            (l.rust_driver)(5);
        });
        assert_eq!(c_out, rust_out);

        let mixed_c_first = capture_stdout(|| unsafe {
            (l.c_driver)(4);
            (l.rust_driver)(4);
        });
        let mixed_rust_first = capture_stdout(|| unsafe {
            (l.rust_driver)(4);
            (l.c_driver)(4);
        });
        assert_eq!(mixed_c_first, mixed_rust_first);
    }
}
