use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libunderhanded_c_nuke_lib.so")
}

type SpectralContrastFn = unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64;
type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32;

unsafe fn call_spectral_contrast(lib: &Library, a: &[f32], b: &[f32]) -> (f64, Vec<f32>, Vec<f32>) {
    let func: Symbol<SpectralContrastFn> = lib.get(b"spectral_contrast").unwrap();
    let mut a_buf = a.to_vec();
    let mut b_buf = b.to_vec();
    let result = func(a_buf.as_mut_ptr(), b_buf.as_mut_ptr(), a.len() as i32);
    (result, a_buf, b_buf)
}

unsafe fn call_match(lib: &Library, test: &[f64], reference: &[f64], threshold: f64) -> i32 {
    let func: Symbol<MatchFn> = lib.get(b"match").unwrap();
    let mut t = test.to_vec();
    let mut r = reference.to_vec();
    func(t.as_mut_ptr(), r.as_mut_ptr(), test.len() as i32, threshold)
}

fn assert_f64_identical(c: f64, r: f64, ctx: &str) {
    assert!(
        c.to_bits() == r.to_bits(),
        "{ctx}: C={c:?} (bits {:016x}) != Rust={r:?} (bits {:016x})",
        c.to_bits(), r.to_bits()
    );
}

fn assert_f32_slices_identical(c: &[f32], r: &[f32], ctx: &str) {
    assert_eq!(c.len(), r.len(), "{ctx}: length mismatch");
    for (i, (cv, rv)) in c.iter().zip(r.iter()).enumerate() {
        assert!(
            cv.to_bits() == rv.to_bits(),
            "{ctx}[{i}]: C={cv:?} (bits {:08x}) != Rust={rv:?} (bits {:08x})",
            cv.to_bits(), rv.to_bits()
        );
    }
}

// ---- spectral_contrast tests (f32 inputs) ----

#[test]
fn test_spectral_contrast_identical_vectors() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    let b: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    let (c_ret, c_a, c_b) = unsafe { call_spectral_contrast(&c_lib, &a, &b) };
    let (r_ret, r_a, r_b) = unsafe { call_spectral_contrast(&r_lib, &a, &b) };
    assert_f64_identical(c_ret, r_ret, "spectral_contrast return (identical)");
    assert_f32_slices_identical(&c_a, &r_a, "a after (identical)");
    assert_f32_slices_identical(&c_b, &r_b, "b after (identical)");
}

#[test]
fn test_spectral_contrast_orthogonal() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a: Vec<f32> = vec![1.0, 0.0, 0.0];
    let b: Vec<f32> = vec![0.0, 1.0, 0.0];
    let (c_ret, c_a, c_b) = unsafe { call_spectral_contrast(&c_lib, &a, &b) };
    let (r_ret, r_a, r_b) = unsafe { call_spectral_contrast(&r_lib, &a, &b) };
    assert_f64_identical(c_ret, r_ret, "spectral_contrast return (orthogonal)");
    assert_f32_slices_identical(&c_a, &r_a, "a after (orthogonal)");
    assert_f32_slices_identical(&c_b, &r_b, "b after (orthogonal)");
}

#[test]
fn test_spectral_contrast_opposite() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a: Vec<f32> = vec![1.0, 2.0, 3.0];
    let b: Vec<f32> = vec![-1.0, -2.0, -3.0];
    let (c_ret, c_a, c_b) = unsafe { call_spectral_contrast(&c_lib, &a, &b) };
    let (r_ret, r_a, r_b) = unsafe { call_spectral_contrast(&r_lib, &a, &b) };
    assert_f64_identical(c_ret, r_ret, "spectral_contrast return (opposite)");
    assert_f32_slices_identical(&c_a, &r_a, "a after (opposite)");
    assert_f32_slices_identical(&c_b, &r_b, "b after (opposite)");
}

#[test]
fn test_spectral_contrast_large() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let n = 256;
    let a: Vec<f32> = (0..n).map(|i| (i as f32 * 0.1).sin()).collect();
    let b: Vec<f32> = (0..n).map(|i| (i as f32 * 0.2).cos()).collect();
    let (c_ret, c_a, c_b) = unsafe { call_spectral_contrast(&c_lib, &a, &b) };
    let (r_ret, r_a, r_b) = unsafe { call_spectral_contrast(&r_lib, &a, &b) };
    assert_f64_identical(c_ret, r_ret, "spectral_contrast return (large)");
    assert_f32_slices_identical(&c_a, &r_a, "a after (large)");
    assert_f32_slices_identical(&c_b, &r_b, "b after (large)");
}

#[test]
fn test_spectral_contrast_single_element() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let a: Vec<f32> = vec![5.0];
    let b: Vec<f32> = vec![3.0];
    let (c_ret, c_a, c_b) = unsafe { call_spectral_contrast(&c_lib, &a, &b) };
    let (r_ret, r_a, r_b) = unsafe { call_spectral_contrast(&r_lib, &a, &b) };
    assert_f64_identical(c_ret, r_ret, "spectral_contrast return (single)");
    assert_f32_slices_identical(&c_a, &r_a, "a after (single)");
    assert_f32_slices_identical(&c_b, &r_b, "b after (single)");
}

// ---- match tests (f64 inputs, as match.h defines float_t = double) ----

#[test]
fn test_match_identical_above_threshold() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let test: Vec<f64> = (0..32).map(|i| (i as f64) + 1.0).collect();
    let reference = test.clone();
    let c_ret = unsafe { call_match(&c_lib, &test, &reference, 0.5) };
    let r_ret = unsafe { call_match(&r_lib, &test, &reference, 0.5) };
    assert_eq!(c_ret, r_ret, "match return (identical, threshold=0.5)");
}

#[test]
fn test_match_below_total_threshold() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let test = vec![0.001; 32];
    let reference = vec![100.0; 32];
    let c_ret = unsafe { call_match(&c_lib, &test, &reference, 0.9) };
    let r_ret = unsafe { call_match(&r_lib, &test, &reference, 0.9) };
    assert_eq!(c_ret, r_ret, "match return (below total threshold)");
}

#[test]
fn test_match_various_thresholds() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let n = 64;
    let test: Vec<f64> = (0..n).map(|i| ((i as f64) * 0.3).sin().abs() + 0.1).collect();
    let reference: Vec<f64> = (0..n).map(|i| ((i as f64) * 0.3).cos().abs() + 0.1).collect();
    for &threshold in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
        let c_ret = unsafe { call_match(&c_lib, &test, &reference, threshold) };
        let r_ret = unsafe { call_match(&r_lib, &test, &reference, threshold) };
        assert_eq!(c_ret, r_ret, "match return (threshold={threshold})");
    }
}

#[test]
fn test_match_large_bins() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let n = 512;
    let test: Vec<f64> = (0..n).map(|i| (i as f64 * 0.05).sin() + 1.5).collect();
    let reference: Vec<f64> = (0..n).map(|i| (i as f64 * 0.05).cos() + 1.5).collect();
    let c_ret = unsafe { call_match(&c_lib, &test, &reference, 0.5) };
    let r_ret = unsafe { call_match(&r_lib, &test, &reference, 0.5) };
    assert_eq!(c_ret, r_ret, "match return (large bins)");
}

#[test]
fn test_match_zero_threshold() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let test = vec![1.0_f64; 32];
    let reference = vec![1.0_f64; 32];
    let c_ret = unsafe { call_match(&c_lib, &test, &reference, 0.0) };
    let r_ret = unsafe { call_match(&r_lib, &test, &reference, 0.0) };
    assert_eq!(c_ret, r_ret, "match return (zero threshold)");
}
