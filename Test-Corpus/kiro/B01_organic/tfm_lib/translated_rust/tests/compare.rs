use libloading::{Library, Symbol};
use std::os::raw::c_int;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtfm_lib.so");

type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

fn rust_lib_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    // cargo builds cdylib in target/debug or target/release
    let debug = format!("{dir}/target/debug/libtfm_lib.so");
    let release = format!("{dir}/target/release/libtfm_lib.so");
    if std::path::Path::new(&release).exists() { release } else { debug }
}

fn call_tfm(lib_path: &str, src: &[f32], count: i32) -> Vec<f32> {
    let mut dest = vec![0.0f32; count as usize * 2];
    unsafe {
        let lib = Library::new(lib_path).unwrap_or_else(|e| panic!("load {lib_path}: {e}"));
        let func: Symbol<TfmFn> = lib.get(b"tfm").unwrap();
        func(dest.as_mut_ptr(), src.as_ptr(), count as c_int);
    }
    dest
}

fn assert_byte_equal(c: &[f32], r: &[f32], label: &str) {
    let cb = unsafe { std::slice::from_raw_parts(c.as_ptr() as *const u8, c.len() * 4) };
    let rb = unsafe { std::slice::from_raw_parts(r.as_ptr() as *const u8, r.len() * 4) };
    assert_eq!(cb, rb, "{label}: C={c:?} Rust={r:?}");
}

fn compare(src: &[f32], count: i32, label: &str) {
    let rlib = rust_lib_path();
    let c = call_tfm(C_LIB, src, count);
    let r = call_tfm(&rlib, src, count);
    assert_byte_equal(&c, &r, label);
}

#[test] fn test_src0_lt_src1()  { compare(&[1.0, 3.0, 0.5], 1, "src0<src1"); }
#[test] fn test_src0_ge_src1()  { compare(&[5.0, 2.0, 0.5], 1, "src0>=src1"); }
#[test] fn test_equal()         { compare(&[2.0, 2.0, 1.0], 1, "equal"); }
#[test] fn test_zeros()         { compare(&[0.0, 0.0, 0.0], 1, "zeros"); }
#[test] fn test_negative()      { compare(&[-1.0, -3.0, -0.5], 1, "negative"); }
#[test] fn test_multi()         { compare(&[1.0, 3.0, 0.5, 5.0, 2.0, 0.5, 2.0, 2.0, 1.0], 3, "multi"); }
#[test] fn test_count_zero()    { compare(&[1.0, 2.0, 3.0], 0, "count_zero"); }
#[test] fn test_neg_sqd()       { compare(&[1.0, 1.0, 0.0], 1, "neg_sqd"); }
#[test] fn test_large()         { compare(&[1e10, 1e12, 1e8], 1, "large"); }
#[test] fn test_subnormal()     { compare(&[1e-38, 1e-39, 1e-40], 1, "subnormal"); }
