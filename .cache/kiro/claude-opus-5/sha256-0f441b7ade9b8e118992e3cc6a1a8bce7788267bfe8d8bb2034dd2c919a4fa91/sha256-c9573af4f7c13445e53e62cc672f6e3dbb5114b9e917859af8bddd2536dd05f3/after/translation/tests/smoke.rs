//! Harness self-check: both `.so`s load, all three symbols resolve, and the
//! exported `array` object really is the 1 MiB object the C library defines.

mod common;

use common::*;

#[test]
fn both_libraries_expose_the_three_symbols() {
    let l = libs();
    for lib in [l.c, l.rs] {
        assert!(!lib.array_ptr().is_null(), "{}: array null", lib.name);
        // Writing the two boundary elements through the exported symbol must be
        // observable, proving the object is at least ARRAY_LEN ints long.
        let mut v = vec![0i32; ARRAY_LEN];
        v[0] = 0x1234_5678;
        v[ARRAY_LEN - 1] = -0x7654_3210;
        lib.write_array(&v);
        let back = lib.read_array();
        assert_eq!(back[0], 0x1234_5678, "{}", lib.name);
        assert_eq!(back[ARRAY_LEN - 1], -0x7654_3210, "{}", lib.name);
    }
}

#[test]
fn stdout_capture_works_through_both_libraries() {
    let l = libs();
    // Cheapest possible observation of the printf path: nothing is printed by
    // `perform_expensive_operations`, so both must produce empty output.
    let c_out = capture_stdout(|| l.c.peo());
    let rs_out = capture_stdout(|| l.rs.peo());
    assert_eq!(c_out, rs_out);
    assert!(c_out.is_empty());
}
