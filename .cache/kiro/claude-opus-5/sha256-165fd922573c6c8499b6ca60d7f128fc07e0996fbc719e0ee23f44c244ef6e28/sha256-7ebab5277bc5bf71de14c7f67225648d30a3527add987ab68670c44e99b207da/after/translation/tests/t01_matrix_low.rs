//! Lowest layer: `allocate_matrix` / `free_matrix` / `initialize_matrix_from_string`.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn allocate_matrix_shapes() {
    let p = pair();
    // Positive shapes plus the degenerate zero cases. Negative dimensions
    // become huge size_t values in C and make malloc fail.
    let cases: &[(c_int, c_int)] = &[
        (1, 1),
        (1, 5),
        (5, 1),
        (3, 4),
        (10, 10),
        (0, 0),
        (0, 3),
        (3, 0),
        (-1, 3),
        (3, -1),
        (-1, -1),
        (-5, -5),
    ];

    for &(w, h) in cases {
        unsafe {
            let cm = (p.c.allocate_matrix)(w, h);
            let rm = (p.rs.allocate_matrix)(w, h);
            assert_eq!(
                cm.is_null(),
                rm.is_null(),
                "allocate_matrix({w},{h}) nullness differs (c={:?}, rust={:?})",
                cm.is_null(),
                rm.is_null()
            );
            // Freshly allocated cells are uninitialised, so only shape is observable.
            let cs = snapshot(cm, false);
            let rs = snapshot(rm, false);
            assert_eq!(cs, rs, "allocate_matrix({w},{h}) shape differs");
            (p.c.free_matrix)(cm);
            (p.rs.free_matrix)(rm);
        }
    }
}

#[test]
fn free_matrix_null_is_noop() {
    let p = pair();
    unsafe {
        (p.c.free_matrix)(std::ptr::null_mut());
        (p.rs.free_matrix)(std::ptr::null_mut());
    }
}

#[test]
fn free_matrix_zero_height() {
    let p = pair();
    unsafe {
        let cm = (p.c.allocate_matrix)(4, 0);
        let rm = (p.rs.allocate_matrix)(4, 0);
        (p.c.free_matrix)(cm);
        (p.rs.free_matrix)(rm);
    }
}

fn init_case(input: &str, w: c_int, h: c_int) {
    let p = pair();
    let s = cstr(input);
    unsafe {
        let cm = (p.c.initialize_matrix_from_string)(s.as_ptr(), w, h);
        let rm = (p.rs.initialize_matrix_from_string)(s.as_ptr(), w, h);
        assert_eq!(
            cm.is_null(),
            rm.is_null(),
            "init({input:?},{w},{h}) nullness differs: c_null={}, rust_null={}",
            cm.is_null(),
            rm.is_null()
        );
        let cs = snapshot(cm, true);
        let rs = snapshot(rm, true);
        assert_eq!(cs, rs, "init({input:?},{w},{h}) contents differ");
        (p.c.free_matrix)(cm);
        (p.rs.free_matrix)(rm);
    }
}

#[test]
fn init_from_string_well_formed() {
    init_case("1 2\n3 4\n", 2, 2);
    init_case("1 2\n3 4", 2, 2);
    init_case("1 2 3\n4 5 6\n", 3, 2);
    init_case("7\n", 1, 1);
    init_case("1 2 3 4 5 6 7 8 9 10\n", 10, 1);
    init_case("1\n2\n3\n4\n5\n", 1, 5);
    // Extra trailing rows / columns are simply ignored.
    init_case("1 2\n3 4\n5 6\n", 2, 2);
    init_case("1 2 3 4\n5 6 7 8\n", 2, 2);
}

#[test]
fn init_from_string_whitespace_quirks() {
    // strtok_r collapses runs of delimiters.
    init_case("1   2\n3   4\n", 2, 2);
    init_case("\n\n1 2\n3 4\n", 2, 2);
    init_case("  1 2\n  3 4\n", 2, 2);
    init_case("1 2\n\n\n3 4\n", 2, 2);
    init_case("1 2 \n3 4 \n", 2, 2);
    // Tabs are not delimiters for strtok_r but atoi skips leading whitespace.
    init_case("1\t2 3\n", 2, 1);
    init_case("\t1 2\n", 2, 1);
}

#[test]
fn init_from_string_atoi_quirks() {
    init_case("abc def\nghi jkl\n", 2, 2);
    init_case("12abc 7x\n", 2, 1);
    init_case("+5 -5\n", 2, 1);
    init_case("-0 0\n", 2, 1);
    init_case("007 -007\n", 2, 1);
    init_case("2147483647 -2147483648\n", 2, 1);
    // Out of range: glibc atoi is (int)strtol, saturating at LONG_MAX/MIN.
    init_case("2147483648 -2147483649\n", 2, 1);
    init_case("9999999999999999999999 -9999999999999999999999\n", 2, 1);
    init_case("4294967296 4294967297\n", 2, 1);
    init_case("- + --5 ++5\n", 4, 1);
    init_case(". , x\n", 3, 1);
    init_case("0x10 0b1 010\n", 3, 1);
}

#[test]
fn init_from_string_error_paths() {
    // Insufficient rows.
    init_case("1 2\n", 2, 2);
    init_case("", 1, 1);
    init_case("\n", 1, 1);
    init_case("   \n", 1, 1);
    init_case("1 2\n3 4\n", 2, 5);
    // Insufficient columns (message includes the 1-based row number).
    init_case("1\n2\n", 2, 2);
    init_case("1 2\n3\n", 2, 2);
    init_case("1 2 3\n4 5 6\n7 8\n", 3, 3);
}

#[test]
fn init_from_string_degenerate_dims() {
    init_case("1 2\n3 4\n", 0, 0);
    init_case("1 2\n3 4\n", 2, 0);
    init_case("1 2\n3 4\n", 0, 2);
    init_case("1 2\n3 4\n", -1, 2);
    init_case("1 2\n3 4\n", 2, -1);
    init_case("1 2\n3 4\n", -3, -3);
    init_case("", 0, 0);
    init_case("", 3, 0);
}
