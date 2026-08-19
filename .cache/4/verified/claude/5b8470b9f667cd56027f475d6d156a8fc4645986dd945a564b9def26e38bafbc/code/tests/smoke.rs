//! Harness self-check: the capture machinery must actually capture, and both
//! `.so`s must be loadable through `libloading` and export `driver`.
//!
//! Without this, a broken capture would make every differential test compare
//! two empty buffers and "pass" vacuously.

mod common;

use common::*;

#[test]
fn both_libraries_load_and_export_driver() {
    let _serial = state_lock();
    let b = both();
    println!("C   : {}", b.c.path.display());
    println!("Rust: {}", b.rust.path.display());
    assert!(b.c.path.is_file());
    assert!(b.rust.path.is_file());
}

#[test]
fn capture_actually_captures_14_lines_from_each() {
    let _serial = state_lock();
    let b = both();
    reset_locale();
    let out_c = run_one(&b.c, b'A' as i8);
    let out_rust = run_one(&b.rust, b'A' as i8);

    println!("C   output ({} bytes): {}", out_c.len(), escape(&out_c));
    println!("Rust output ({} bytes): {}", out_rust.len(), escape(&out_rust));

    assert!(!out_c.is_empty(), "captured nothing from the C library");
    assert!(!out_rust.is_empty(), "captured nothing from the Rust library");
    assert_eq!(out_c.iter().filter(|&&x| x == b'\n').count(), 14);
    assert_eq!(out_rust.iter().filter(|&&x| x == b'\n').count(), 14);
    for label in LABELS {
        assert!(
            out_c.windows(label.len()).any(|w| w == label.as_bytes()),
            "C output is missing the `{label}` line"
        );
    }
    assert_eq!(escape(&out_c), escape(&out_rust));
}

#[test]
fn capture_is_not_a_no_op_for_differing_inputs() {
    let _serial = state_lock();
    // Sanity: different inputs must produce different captures, otherwise the
    // harness could be returning a constant.
    let b = both();
    reset_locale();
    let a = run_one(&b.c, b'a' as i8);
    let z = run_one(&b.c, b'0' as i8);
    assert_ne!(a, z, "capture returns the same bytes for 'a' and '0'");
}
