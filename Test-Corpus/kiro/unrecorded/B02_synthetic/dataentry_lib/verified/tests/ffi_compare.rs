use libloading::{Library, Symbol};
use std::path::PathBuf;

type DataentryFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdataentry_lib.so")
}

fn load_fn(lib: &Library) -> Symbol<DataentryFn> {
    unsafe { lib.get(b"dataentry").expect("symbol not found") }
}

fn compare(mode: i32, p1: i32, p2: i32, p3: i32, c_lib: &Library, r_lib: &Library) {
    let c_fn = load_fn(c_lib);
    let r_fn = load_fn(r_lib);
    let c_result = unsafe { c_fn(mode, p1, p2, p3) };
    let r_result = unsafe { r_fn(mode, p1, p2, p3) };
    assert_eq!(
        c_result, r_result,
        "MISMATCH: dataentry({mode}, {p1}, {p2}, {p3}) => C={c_result}, Rust={r_result}"
    );
}

#[test]
fn test_mode1_basic() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // count=5, find entry at offset 0..4
    for offset in 0..6 {
        compare(1, 5, offset, 0, &c_lib, &r_lib);
    }
}

#[test]
fn test_mode1_default_count() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // param1 <= 0 => count defaults to 5
    compare(1, 0, 0, 0, &c_lib, &r_lib);
    compare(1, -1, 2, 0, &c_lib, &r_lib);
}

#[test]
fn test_mode1_not_found() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // offset beyond count => not found
    compare(1, 3, 10, 0, &c_lib, &r_lib);
}

#[test]
fn test_mode2_basic() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    for count in [1, 2, 3, 5] {
        for mult in [0, 1, 2, -1] {
            for p3 in [0, 10, -5] {
                compare(2, count, mult, p3, &c_lib, &r_lib);
            }
        }
    }
}

#[test]
fn test_mode2_default_count() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    compare(2, 0, 2, 5, &c_lib, &r_lib);
    compare(2, -1, 1, 0, &c_lib, &r_lib);
}

#[test]
fn test_mode3_all_cells() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    for row in 0..4 {
        for col in 0..3 {
            for p3 in [0, 7, -3] {
                compare(3, row, col, p3, &c_lib, &r_lib);
            }
        }
    }
}

#[test]
fn test_mode3_out_of_bounds() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    // Out of bounds => result stays 0
    compare(3, -1, 0, 5, &c_lib, &r_lib);
    compare(3, 4, 0, 5, &c_lib, &r_lib);
    compare(3, 0, 3, 5, &c_lib, &r_lib);
    compare(3, 0, -1, 5, &c_lib, &r_lib);
}

#[test]
fn test_default_mode() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    for p1 in [0, 1, 3, -2] {
        compare(0, p1, 0, 0, &c_lib, &r_lib);
        compare(99, p1, 0, 0, &c_lib, &r_lib);
    }
}
