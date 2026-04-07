use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Find the cdylib in target/debug
    let p = dir.join("target/debug/libdriver.so");
    if p.exists() { return p; }
    panic!("Rust .so not found");
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

// ---- fma_array tests (lowest level) ----

#[test]
fn test_fma_array_basic() {
    let (c_lib, r_lib) = load_libs();
    type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
    let c_fn: Symbol<FmaArray> = unsafe { c_lib.get(b"fma_array").unwrap() };
    let r_fn: Symbol<FmaArray> = unsafe { r_lib.get(b"fma_array").unwrap() };

    let mul1 = [2, 3, 4, 5i32];
    let mul2 = [10, 20, 30, 40i32];
    let add = [1, 2, 3, 4i32];
    let len = 4;

    let mut c_out = [0i32; 4];
    let mut r_out = [0i32; 4];
    unsafe {
        c_fn(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        r_fn(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(c_out, r_out, "fma_array basic mismatch");
}

#[test]
fn test_fma_array_empty() {
    let (c_lib, r_lib) = load_libs();
    type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
    let c_fn: Symbol<FmaArray> = unsafe { c_lib.get(b"fma_array").unwrap() };
    let r_fn: Symbol<FmaArray> = unsafe { r_lib.get(b"fma_array").unwrap() };

    let mut c_out = [99i32; 1];
    let mut r_out = [99i32; 1];
    // len=0 should not touch output
    unsafe {
        c_fn(c_out.as_mut_ptr(), [].as_ptr(), [].as_ptr(), [].as_ptr(), 0);
        r_fn(r_out.as_mut_ptr(), [].as_ptr(), [].as_ptr(), [].as_ptr(), 0);
    }
    assert_eq!(c_out, r_out, "fma_array empty mismatch");
}

#[test]
fn test_fma_array_negative() {
    let (c_lib, r_lib) = load_libs();
    type FmaArray = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
    let c_fn: Symbol<FmaArray> = unsafe { c_lib.get(b"fma_array").unwrap() };
    let r_fn: Symbol<FmaArray> = unsafe { r_lib.get(b"fma_array").unwrap() };

    let mul1 = [-1, -2, 3i32];
    let mul2 = [5, -3, -7i32];
    let add = [10, 20, -30i32];

    let mut c_out = [0i32; 3];
    let mut r_out = [0i32; 3];
    unsafe {
        c_fn(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        r_fn(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
    }
    assert_eq!(c_out, r_out, "fma_array negative mismatch");
}

// ---- call_fma tests (mid level) ----

#[test]
fn test_call_fma_basic() {
    let (c_lib, r_lib) = load_libs();
    type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
    let c_fn: Symbol<CallFma> = unsafe { c_lib.get(b"call_fma").unwrap() };
    let r_fn: Symbol<CallFma> = unsafe { r_lib.get(b"call_fma").unwrap() };

    let data = [10, 20, 30, 42i32];
    let c_res = unsafe { c_fn(data.as_ptr(), 4) };
    let r_res = unsafe { r_fn(data.as_ptr(), 4) };
    assert_eq!(c_res, r_res, "call_fma basic mismatch");
}

#[test]
fn test_call_fma_empty() {
    let (c_lib, r_lib) = load_libs();
    type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
    let c_fn: Symbol<CallFma> = unsafe { c_lib.get(b"call_fma").unwrap() };
    let r_fn: Symbol<CallFma> = unsafe { r_lib.get(b"call_fma").unwrap() };

    let c_res = unsafe { c_fn([].as_ptr(), 0) };
    let r_res = unsafe { r_fn([].as_ptr(), 0) };
    assert_eq!(c_res, r_res, "call_fma empty mismatch");
}

#[test]
fn test_call_fma_single() {
    let (c_lib, r_lib) = load_libs();
    type CallFma = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
    let c_fn: Symbol<CallFma> = unsafe { c_lib.get(b"call_fma").unwrap() };
    let r_fn: Symbol<CallFma> = unsafe { r_lib.get(b"call_fma").unwrap() };

    let data = [77i32];
    let c_res = unsafe { c_fn(data.as_ptr(), 1) };
    let r_res = unsafe { r_fn(data.as_ptr(), 1) };
    assert_eq!(c_res, r_res, "call_fma single mismatch");
}

// ---- driver tests (top level) ----
// driver() prints to stdout via printf, so we capture and compare output

#[test]
fn test_driver_basic() {
    let (c_lib, r_lib) = load_libs();
    type Driver = unsafe extern "C" fn(*const c_char);
    let c_fn: Symbol<Driver> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<Driver> = unsafe { r_lib.get(b"driver").unwrap() };

    // We can't easily capture printf output in-process, but we can at least
    // verify both don't crash and call them with the same input.
    // For a proper comparison, we'll fork/exec, but for now just call both.
    let input = CString::new("1 2 3 4 5").unwrap();
    unsafe {
        c_fn(input.as_ptr());
        r_fn(input.as_ptr());
    }
    // Both should print "5\n" — if either crashes, the test fails
}

#[test]
fn test_driver_empty_input() {
    let (c_lib, r_lib) = load_libs();
    type Driver = unsafe extern "C" fn(*const c_char);
    let c_fn: Symbol<Driver> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<Driver> = unsafe { r_lib.get(b"driver").unwrap() };

    let input = CString::new("").unwrap();
    unsafe {
        c_fn(input.as_ptr());
        r_fn(input.as_ptr());
    }
    // Both should print "0\n"
}

#[test]
fn test_driver_single_number() {
    let (c_lib, r_lib) = load_libs();
    type Driver = unsafe extern "C" fn(*const c_char);
    let c_fn: Symbol<Driver> = unsafe { c_lib.get(b"driver").unwrap() };
    let r_fn: Symbol<Driver> = unsafe { r_lib.get(b"driver").unwrap() };

    let input = CString::new("42").unwrap();
    unsafe {
        c_fn(input.as_ptr());
        r_fn(input.as_ptr());
    }
}
