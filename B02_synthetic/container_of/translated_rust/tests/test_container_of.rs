use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Test {
    a: i32,
    b: i32,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libcontainer_of.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libcontainer_of.so")
}

#[test]
fn test_find_container_of_a() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> *mut Test> =
        unsafe { c_lib.get(b"find_container_of_a").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut i32) -> *mut Test> =
        unsafe { r_lib.get(b"find_container_of_a").unwrap() };

    for (a_val, b_val) in [(10, 20), (0, 0), (-1, 42), (i32::MAX, i32::MIN)] {
        let mut c_t = Test { a: a_val, b: b_val };
        let mut r_t = Test { a: a_val, b: b_val };

        let c_result = unsafe { *c_fn(&mut c_t.a) };
        let r_result = unsafe { *r_fn(&mut r_t.a) };

        assert_eq!(
            c_result, r_result,
            "find_container_of_a mismatch for a={a_val}, b={b_val}: C={c_result:?} Rust={r_result:?}"
        );
    }
}

#[test]
fn test_find_container_of_b() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> *mut Test> =
        unsafe { c_lib.get(b"find_container_of_b").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut i32) -> *mut Test> =
        unsafe { r_lib.get(b"find_container_of_b").unwrap() };

    for (a_val, b_val) in [(10, 20), (0, 0), (-1, 42), (i32::MAX, i32::MIN)] {
        let mut c_t = Test { a: a_val, b: b_val };
        let mut r_t = Test { a: a_val, b: b_val };

        let c_result = unsafe { *c_fn(&mut c_t.b) };
        let r_result = unsafe { *r_fn(&mut r_t.b) };

        assert_eq!(
            c_result, r_result,
            "find_container_of_b mismatch for a={a_val}, b={b_val}: C={c_result:?} Rust={r_result:?}"
        );
    }
}
