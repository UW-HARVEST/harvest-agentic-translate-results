use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libgotomach_lib.so")
}

fn load_c_lib() -> Library {
    unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") }
}

fn load_rust_lib() -> Library {
    unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") }
}

type ValFn = unsafe extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32;
type GotomachFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

#[test]
fn test_process_value() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let c_fn: Symbol<ValFn> = unsafe { c_lib.get(b"process_value").unwrap() };
    let r_fn: Symbol<ValFn> = unsafe { r_lib.get(b"process_value").unwrap() };

    for &v in &[0, 1, -1, 100, -100, i32::MAX - 10, i32::MIN] {
        let c = unsafe { c_fn(v, 0, std::ptr::null_mut()) };
        let r = unsafe { r_fn(v, 0, std::ptr::null_mut()) };
        assert_eq!(c, r, "process_value mismatch for input {v}");
    }
}

#[test]
fn test_double_value() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let c_fn: Symbol<ValFn> = unsafe { c_lib.get(b"double_value").unwrap() };
    let r_fn: Symbol<ValFn> = unsafe { r_lib.get(b"double_value").unwrap() };

    for &v in &[0, 1, -1, 50, -50, 1000, i32::MIN / 2] {
        let c = unsafe { c_fn(v, 0, std::ptr::null_mut()) };
        let r = unsafe { r_fn(v, 0, std::ptr::null_mut()) };
        assert_eq!(c, r, "double_value mismatch for input {v}");
    }
}

#[test]
fn test_triple_value() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let c_fn: Symbol<ValFn> = unsafe { c_lib.get(b"triple_value").unwrap() };
    let r_fn: Symbol<ValFn> = unsafe { r_lib.get(b"triple_value").unwrap() };

    for &v in &[0, 1, -1, 33, -33, 500] {
        let c = unsafe { c_fn(v, 0, std::ptr::null_mut()) };
        let r = unsafe { r_fn(v, 0, std::ptr::null_mut()) };
        assert_eq!(c, r, "triple_value mismatch for input {v}");
    }
}

#[test]
fn test_gotomach() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let c_fn: Symbol<GotomachFn> = unsafe { c_lib.get(b"gotomach").unwrap() };
    let r_fn: Symbol<GotomachFn> = unsafe { r_lib.get(b"gotomach").unwrap() };

    let cases: &[(i32, i32, i32, i32)] = &[
        (0, 0, 0, 100),
        (1, 5, 0, 100),
        (10, 5, 0, 100),
        (10, 5, 1, 100),
        (10, 5, 2, 100),
        (10, 5, 3, 100),
        (5, 0, 0, 0),
        (5, 100, 0, 500),
        (100, 42, 0, 200),
        (100, 42, 1, 200),
        (100, 42, 2, 200),
        (-1, 0, 0, 0),
        (0, -1, 0, 0),
        (70000, 0, 0, 0),
        (0, 70000, 0, 0),
    ];

    for &(iter, seed, mode, thresh) in cases {
        let c = unsafe { c_fn(iter, seed, mode, thresh) };
        let r = unsafe { r_fn(iter, seed, mode, thresh) };
        assert_eq!(c, r, "gotomach mismatch for ({iter}, {seed}, {mode}, {thresh}): C={c}, Rust={r}");
    }
}
