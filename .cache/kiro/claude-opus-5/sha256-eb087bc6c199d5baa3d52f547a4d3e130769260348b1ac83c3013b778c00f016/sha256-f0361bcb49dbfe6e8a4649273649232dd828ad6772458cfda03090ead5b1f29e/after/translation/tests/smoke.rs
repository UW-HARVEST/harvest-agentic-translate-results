//! Plumbing smoke test: both `.so`s load, export `driver`, and the stdout
//! capture works. Run this first when debugging the harness.

mod common;

use common::*;

#[test]
fn smoke_both_libraries_load_and_export_driver() {
    let p = pair();
    let out = capture(|| unsafe { (p.c_driver)(1.0) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "3ff0000000000000 0x1p+0 1.0000\n",
        "C reference output for 1.0 changed unexpectedly"
    );
    let out = capture(|| unsafe { (p.r_driver)(1.0) });
    assert_eq!(String::from_utf8_lossy(&out), "3ff0000000000000 0x1p+0 1.0000\n");
}

#[test]
fn smoke_capture_is_reentrant_and_restores_fd1() {
    let p = pair();
    for _ in 0..4 {
        let a = capture(|| unsafe { (p.c_driver)(0.5) });
        let b = capture(|| unsafe { (p.r_driver)(0.5) });
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }
}

#[test]
fn smoke_batched_capture_matches_line_count() {
    let bits: Vec<u64> = (0..64u64).map(|i| (i as f64 * 1.5).to_bits()).collect();
    let divs = compare_batch(&bits);
    assert!(divs.is_empty(), "unexpected divergence: {:?}", divs.first());
}
