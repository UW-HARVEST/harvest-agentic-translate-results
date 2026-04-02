use libloading::{Library, Symbol};
use std::ffi::c_int;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libnormalize_lib.so");

type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

fn call_c_normalize(src: &[f32]) -> Vec<f32> {
    let lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let func: Symbol<NormalizeFn> = unsafe { lib.get(b"normalize").expect("find normalize") };
    let mut dest = vec![0.0f32; src.len()];
    unsafe { func(dest.as_mut_ptr(), src.as_ptr(), src.len() as c_int) };
    dest
}

fn call_rust_normalize(src: &[f32]) -> Vec<f32> {
    let mut dest = vec![0.0f32; src.len()];
    unsafe { normalize_lib::normalize(dest.as_mut_ptr(), src.as_ptr(), src.len() as c_int) };
    dest
}

fn assert_byte_equal(c: &[f32], r: &[f32], label: &str) {
    let cb = unsafe { std::slice::from_raw_parts(c.as_ptr() as *const u8, c.len() * 4) };
    let rb = unsafe { std::slice::from_raw_parts(r.as_ptr() as *const u8, r.len() * 4) };
    assert_eq!(cb, rb, "{}: C={:?} Rust={:?}", label, c, r);
}

#[test]
fn test_normalize_basic() {
    let src = [3.0f32, 4.0];
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "basic [3,4]");
}

#[test]
fn test_normalize_zeros() {
    let src = [0.0f32, 0.0, 0.0];
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "zeros");
}

#[test]
fn test_normalize_single() {
    let src = [5.0f32];
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "single");
}

#[test]
fn test_normalize_negative() {
    let src = [-1.0f32, 2.0, -3.0, 4.0];
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "negative");
}

#[test]
fn test_normalize_large() {
    let src: Vec<f32> = (1..=100).map(|i| i as f32).collect();
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "large 1..100");
}

#[test]
fn test_normalize_subnormal() {
    let src = [1e-38f32, 1e-38];
    let c = call_c_normalize(&src);
    let r = call_rust_normalize(&src);
    assert_byte_equal(&c, &r, "subnormal");
}

#[test]
fn test_normalize_inplace() {
    // Test dest == src (in-place normalization)
    let mut c_buf = [3.0f32, 4.0];
    let mut r_buf = [3.0f32, 4.0];
    let lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let func: Symbol<NormalizeFn> = unsafe { lib.get(b"normalize").expect("find normalize") };
    unsafe { func(c_buf.as_mut_ptr(), c_buf.as_ptr(), 2) };
    unsafe { normalize_lib::normalize(r_buf.as_mut_ptr(), r_buf.as_ptr(), 2) };
    assert_byte_equal(&c_buf, &r_buf, "inplace");
}
