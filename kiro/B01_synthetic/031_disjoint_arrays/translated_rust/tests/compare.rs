use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

#[test]
fn test_fma_array() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
        unsafe { c_lib.get(b"fma_array").unwrap() };
    let r_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
        unsafe { r_lib.get(b"fma_array").unwrap() };

    // Test case 1: basic
    let mul1 = [1, 2, 3, 4, 5];
    let mul2 = [5, 4, 3, 2, 1];
    let add = [10, 20, 30, 40, 50];

    let mut c_out = [0i32; 5];
    let mut r_out = [0i32; 5];
    unsafe {
        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        r_fma(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
    }
    assert_eq!(c_out, r_out, "fma_array basic mismatch: C={:?} Rust={:?}", c_out, r_out);

    // Test case 2: empty
    unsafe {
        c_fma(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0);
        r_fma(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0);
    }

    // Test case 3: negative values
    let mul1n = [-1, -2, 3];
    let mul2n = [4, -5, -6];
    let addn = [7, 8, -9];
    let mut c_out3 = [0i32; 3];
    let mut r_out3 = [0i32; 3];
    unsafe {
        c_fma(c_out3.as_mut_ptr(), mul1n.as_ptr(), mul2n.as_ptr(), addn.as_ptr(), 3);
        r_fma(r_out3.as_mut_ptr(), mul1n.as_ptr(), mul2n.as_ptr(), addn.as_ptr(), 3);
    }
    assert_eq!(c_out3, r_out3, "fma_array negative mismatch: C={:?} Rust={:?}", c_out3, r_out3);
}

#[test]
fn test_call_fma() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<unsafe extern "C" fn(*const i32, i32) -> i32> =
        unsafe { c_lib.get(b"call_fma").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*const i32, i32) -> i32> =
        unsafe { r_lib.get(b"call_fma").unwrap() };

    // basic
    let data = [10, 20, 30, 40, 50];
    let c_r = unsafe { c_fn(data.as_ptr(), 5) };
    let r_r = unsafe { r_fn(data.as_ptr(), 5) };
    assert_eq!(c_r, r_r, "call_fma basic: C={} Rust={}", c_r, r_r);

    // empty
    let c_e = unsafe { c_fn(std::ptr::null(), 0) };
    let r_e = unsafe { r_fn(std::ptr::null(), 0) };
    assert_eq!(c_e, r_e, "call_fma empty");

    // single
    let single = [42];
    let c_s = unsafe { c_fn(single.as_ptr(), 1) };
    let r_s = unsafe { r_fn(single.as_ptr(), 1) };
    assert_eq!(c_s, r_s, "call_fma single: C={} Rust={}", c_s, r_s);

    // negative
    let neg = [-5, -10, -15];
    let c_n = unsafe { c_fn(neg.as_ptr(), 3) };
    let r_n = unsafe { r_fn(neg.as_ptr(), 3) };
    assert_eq!(c_n, r_n, "call_fma negative: C={} Rust={}", c_n, r_n);
}
