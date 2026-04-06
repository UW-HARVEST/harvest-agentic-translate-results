use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdataentry_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn call_c(mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> c_int {
    let lib = c_lib();
    unsafe {
        let func: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            lib.get(b"dataentry").unwrap();
        func(mode, p1, p2, p3)
    }
}

fn call_rust(mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> c_int {
    dataentry_lib::dataentry(mode, p1, p2, p3)
}

fn check(mode: c_int, p1: c_int, p2: c_int, p3: c_int) {
    let c = call_c(mode, p1, p2, p3);
    let r = call_rust(mode, p1, p2, p3);
    assert_eq!(
        c, r,
        "MISMATCH: dataentry({mode}, {p1}, {p2}, {p3}) => C={c}, Rust={r}"
    );
}

// Mode 1: create entries + find by id
#[test]
fn mode1_basic() {
    check(1, 5, 0, 0);
    check(1, 5, 1, 0);
    check(1, 5, 4, 0);
}

#[test]
fn mode1_not_found() {
    check(1, 5, 10, 0); // target_id=110, out of range
}

#[test]
fn mode1_default_count() {
    check(1, 0, 0, 0); // count defaults to 5
    check(1, -1, 0, 0);
}

#[test]
fn mode1_single() {
    check(1, 1, 0, 0);
    check(1, 1, 1, 0); // out of range
}

// Mode 2: create + modify entries
#[test]
fn mode2_basic() {
    check(2, 3, 2, 10);
    check(2, 5, 3, 0);
}

#[test]
fn mode2_default_count() {
    check(2, 0, 2, 5); // count defaults to 3
    check(2, -1, 2, 5);
}

#[test]
fn mode2_zero_multiplier() {
    check(2, 3, 0, 10);
}

#[test]
fn mode2_negative() {
    check(2, 3, -1, 0);
}

// Mode 3: lookup table
#[test]
fn mode3_all_cells() {
    for row in 0..4 {
        for col in 0..3 {
            check(3, row, col, 0);
            check(3, row, col, 7);
        }
    }
}

#[test]
fn mode3_out_of_bounds() {
    check(3, -1, 0, 0);
    check(3, 4, 0, 0);
    check(3, 0, -1, 0);
    check(3, 0, 3, 0);
}

// Default mode: process_name string ops
#[test]
fn mode_default() {
    check(0, 1, 0, 0);
    check(0, 2, 0, 0);
    check(0, 0, 0, 0);
    check(0, -1, 0, 0);
    check(99, 3, 0, 0);
}
