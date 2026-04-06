use libloading::{Library, Symbol};
use std::ffi::c_int;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // cdylib output is in target/debug/
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug");
    let path = format!("{}/libmaxnmin_lib.so", dir);
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

#[test]
fn test_process_string() {
    let clib = c_lib();
    let rlib = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_int> =
        unsafe { clib.get(b"process_string").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const u8) -> c_int> =
        unsafe { rlib.get(b"process_string").unwrap() };

    for s in &[b"hello\0" as &[u8], b"\0", b"A\0", b"root\0", b"grandchild1\0"] {
        let c_r = unsafe { c_fn(s.as_ptr()) };
        let r_r = unsafe { r_fn(s.as_ptr()) };
        assert_eq!(c_r, r_r, "process_string mismatch for {:?}", s);
    }
}

#[test]
fn test_safe_double_to_int() {
    let clib = c_lib();
    let rlib = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { clib.get(b"safe_double_to_int").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { rlib.get(b"safe_double_to_int").unwrap() };

    for &d in &[
        0.0, 1.0, -1.0, 10.5, -10.5, 2147483647.0, -2147483648.0,
        1e20, -1e20, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        0.9, -0.9, 100.7, -100.7,
    ] {
        let c_r = unsafe { c_fn(d) };
        let r_r = unsafe { r_fn(d) };
        assert_eq!(c_r, r_r, "safe_double_to_int mismatch for {}", d);
    }
}

#[test]
fn test_maxnmin() {
    let clib = c_lib();
    let rlib = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { clib.get(b"maxnmin").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { rlib.get(b"maxnmin").unwrap() };

    for &(a, b, c, d) in &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (5, 5, 5, 5),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (6, 12, 1, 3),
        (7, 0, 2, 1),
        (0, 0, 1, 1),
        (3, 3, 3, 3),
        (1, 1, 0, 0),
        (2, 4, 6, 8),
        (11, 13, 17, 19),
    ] {
        let c_r = unsafe { c_fn(a, b, c, d) };
        let r_r = unsafe { r_fn(a, b, c, d) };
        assert_eq!(
            c_r, r_r,
            "maxnmin({},{},{},{}): C={} Rust={}", a, b, c, d, c_r, r_r
        );
    }
}
