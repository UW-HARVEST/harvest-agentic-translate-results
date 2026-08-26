use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_lib/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

type FmaArrayFn = unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32);
type CallFmaFn = unsafe extern "C" fn(*const i32, i32) -> i32;

fn load_fma_array(lib: &Library) -> Symbol<FmaArrayFn> {
    unsafe { lib.get(b"fma_array").unwrap() }
}

fn load_call_fma(lib: &Library) -> Symbol<CallFmaFn> {
    unsafe { lib.get(b"call_fma").unwrap() }
}

#[test]
fn test_fma_array_basic() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fma_array(&c);
    let r_fn = load_fma_array(&r);

    let mul1 = [2, 3, 4];
    let mul2 = [5, 6, 7];
    let add = [10, 20, 30];
    let mut c_out = [0i32; 3];
    let mut r_out = [0i32; 3];

    unsafe {
        c_fn(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        r_fn(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
    }
    assert_eq!(c_out, r_out, "fma_array basic mismatch");
}

#[test]
fn test_fma_array_empty() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fma_array(&c);
    let r_fn = load_fma_array(&r);

    let mut c_out = [99i32; 1];
    let mut r_out = [99i32; 1];
    let dummy = [0i32; 0];

    unsafe {
        c_fn(c_out.as_mut_ptr(), dummy.as_ptr(), dummy.as_ptr(), dummy.as_ptr(), 0);
        r_fn(r_out.as_mut_ptr(), dummy.as_ptr(), dummy.as_ptr(), dummy.as_ptr(), 0);
    }
    assert_eq!(c_out, r_out, "fma_array empty mismatch");
}

#[test]
fn test_fma_array_negative() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_fma_array(&c);
    let r_fn = load_fma_array(&r);

    let mul1 = [-1, i32::MAX, i32::MIN];
    let mul2 = [2, 2, 1];
    let add = [0, 1, 0];
    let mut c_out = [0i32; 3];
    let mut r_out = [0i32; 3];

    unsafe {
        c_fn(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
        r_fn(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 3);
    }
    assert_eq!(c_out, r_out, "fma_array negative/overflow mismatch");
}

#[test]
fn test_call_fma_basic() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_call_fma(&c);
    let r_fn = load_call_fma(&r);

    let data = [10, 20, 30, 40, 50];
    let c_res = unsafe { c_fn(data.as_ptr(), 5) };
    let r_res = unsafe { r_fn(data.as_ptr(), 5) };
    assert_eq!(c_res, r_res, "call_fma basic mismatch");
}

#[test]
fn test_call_fma_empty() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_call_fma(&c);
    let r_fn = load_call_fma(&r);

    let data = [0i32; 0];
    let c_res = unsafe { c_fn(data.as_ptr(), 0) };
    let r_res = unsafe { r_fn(data.as_ptr(), 0) };
    assert_eq!(c_res, r_res, "call_fma empty mismatch");
}

#[test]
fn test_call_fma_single() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_call_fma(&c);
    let r_fn = load_call_fma(&r);

    let data = [42];
    let c_res = unsafe { c_fn(data.as_ptr(), 1) };
    let r_res = unsafe { r_fn(data.as_ptr(), 1) };
    assert_eq!(c_res, r_res, "call_fma single mismatch");
}

#[test]
fn test_call_fma_negative() {
    let c = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let c_fn = load_call_fma(&c);
    let r_fn = load_call_fma(&r);

    let data = [-5, -10, -15];
    let c_res = unsafe { c_fn(data.as_ptr(), 3) };
    let r_res = unsafe { r_fn(data.as_ptr(), 3) };
    assert_eq!(c_res, r_res, "call_fma negative mismatch");
}
