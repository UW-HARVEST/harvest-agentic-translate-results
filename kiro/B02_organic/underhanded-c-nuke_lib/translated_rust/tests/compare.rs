use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

// ---- spectral_contrast tests (lower-level, operates on f32/float) ----

#[test]
fn test_spectral_contrast_identical_vectors() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_sc: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64> =
        unsafe { c_lib.get(b"spectral_contrast").unwrap() };

    let base: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    let mut c_a = base.clone();
    let mut c_b = base.clone();
    let c_result = unsafe { c_sc(c_a.as_mut_ptr(), c_b.as_mut_ptr(), base.len() as i32) };

    let mut r_a = base.clone();
    let mut r_b = base.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::spectral_contrast(r_a.as_mut_ptr(), r_b.as_mut_ptr(), base.len() as i32)
    };

    assert_eq!(c_result.to_bits(), r_result.to_bits(), "spectral_contrast identical: C={c_result} Rust={r_result}");
    assert_eq!(c_a, r_a, "spectral_contrast identical: mutated a differs");
    assert_eq!(c_b, r_b, "spectral_contrast identical: mutated b differs");
}

#[test]
fn test_spectral_contrast_orthogonal() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_sc: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64> =
        unsafe { c_lib.get(b"spectral_contrast").unwrap() };

    let a_base: Vec<f32> = vec![1.0, 0.0, 0.0];
    let b_base: Vec<f32> = vec![0.0, 1.0, 0.0];

    let mut c_a = a_base.clone();
    let mut c_b = b_base.clone();
    let c_result = unsafe { c_sc(c_a.as_mut_ptr(), c_b.as_mut_ptr(), 3) };

    let mut r_a = a_base.clone();
    let mut r_b = b_base.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::spectral_contrast(r_a.as_mut_ptr(), r_b.as_mut_ptr(), 3)
    };

    assert_eq!(c_result.to_bits(), r_result.to_bits(), "spectral_contrast orthogonal: C={c_result} Rust={r_result}");
}

#[test]
fn test_spectral_contrast_large() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_sc: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64> =
        unsafe { c_lib.get(b"spectral_contrast").unwrap() };

    let n = 64;
    let a_base: Vec<f32> = (0..n).map(|i| (i as f32 * 0.3).sin()).collect();
    let b_base: Vec<f32> = (0..n).map(|i| (i as f32 * 0.7).cos()).collect();

    let mut c_a = a_base.clone();
    let mut c_b = b_base.clone();
    let c_result = unsafe { c_sc(c_a.as_mut_ptr(), c_b.as_mut_ptr(), n as i32) };

    let mut r_a = a_base.clone();
    let mut r_b = b_base.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::spectral_contrast(r_a.as_mut_ptr(), r_b.as_mut_ptr(), n as i32)
    };

    assert_eq!(c_result.to_bits(), r_result.to_bits(), "spectral_contrast large: C={c_result} Rust={r_result}");
    assert_eq!(c_a, r_a, "spectral_contrast large: mutated a differs");
    assert_eq!(c_b, r_b, "spectral_contrast large: mutated b differs");
}

// ---- match tests (higher-level, operates on f64/double) ----

#[test]
fn test_match_identical_spectra() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_match: Symbol<unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32> =
        unsafe { c_lib.get(b"match").unwrap() };

    let n = 32;
    let base: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0) * 10.0).collect();
    let threshold = 0.5;

    let mut c_t = base.clone();
    let mut c_r = base.clone();
    let c_result = unsafe { c_match(c_t.as_mut_ptr(), c_r.as_mut_ptr(), n as i32, threshold) };

    let mut r_t = base.clone();
    let mut r_r = base.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::match_(r_t.as_mut_ptr(), r_r.as_mut_ptr(), n as i32, threshold)
    };

    assert_eq!(c_result, r_result, "match identical: C={c_result} Rust={r_result}");
}

#[test]
fn test_match_below_threshold() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_match: Symbol<unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32> =
        unsafe { c_lib.get(b"match").unwrap() };

    let n = 32;
    let test: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0) * 0.01).collect();
    let reference: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0) * 100.0).collect();
    let threshold = 0.9;

    let mut c_t = test.clone();
    let mut c_r = reference.clone();
    let c_result = unsafe { c_match(c_t.as_mut_ptr(), c_r.as_mut_ptr(), n as i32, threshold) };

    let mut r_t = test.clone();
    let mut r_r = reference.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::match_(r_t.as_mut_ptr(), r_r.as_mut_ptr(), n as i32, threshold)
    };

    assert_eq!(c_result, r_result, "match below threshold: C={c_result} Rust={r_result}");
}

#[test]
fn test_match_different_spectra() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_match: Symbol<unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32> =
        unsafe { c_lib.get(b"match").unwrap() };

    let n = 64;
    let test: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin() + 2.0).collect();
    let reference: Vec<f64> = (0..n).map(|i| (i as f64 * 0.5).cos() + 2.0).collect();
    let threshold = 0.8;

    let mut c_t = test.clone();
    let mut c_r = reference.clone();
    let c_result = unsafe { c_match(c_t.as_mut_ptr(), c_r.as_mut_ptr(), n as i32, threshold) };

    let mut r_t = test.clone();
    let mut r_r = reference.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::match_(r_t.as_mut_ptr(), r_r.as_mut_ptr(), n as i32, threshold)
    };

    assert_eq!(c_result, r_result, "match different: C={c_result} Rust={r_result}");
}

#[test]
fn test_match_edge_small_bins() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_match: Symbol<unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32> =
        unsafe { c_lib.get(b"match").unwrap() };

    let n = 4;
    let base: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0];
    let threshold = 0.5;

    let mut c_t = base.clone();
    let mut c_r = base.clone();
    let c_result = unsafe { c_match(c_t.as_mut_ptr(), c_r.as_mut_ptr(), n as i32, threshold) };

    let mut r_t = base.clone();
    let mut r_r = base.clone();
    let r_result = unsafe {
        underhanded_c_nuke_lib::match_(r_t.as_mut_ptr(), r_r.as_mut_ptr(), n as i32, threshold)
    };

    assert_eq!(c_result, r_result, "match small bins: C={c_result} Rust={r_result}");
}
