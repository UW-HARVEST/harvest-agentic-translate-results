// Integration tests comparing C and Rust implementations through FFI.
//
// Important: in this codebase the C source has a subtle quirk:
//   - match.h does `typedef double float_t;` so `match()` takes `double*`.
//   - spectral_contrast.c does NOT include match.h; it only includes
//     <math.h>, where `float_t` is `float` on x86-64 Linux glibc
//     (FLT_EVAL_METHOD == 0).
// So the exported `spectral_contrast` actually takes `float*`, not
// `double*`. The Rust translation must reproduce this byte-identically.

use libloading::{Library, Symbol};
use std::path::PathBuf;

type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, i32, f64) -> i32;
type SpectralContrastFn = unsafe extern "C" fn(*mut f32, *mut f32, i32) -> f64;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is named via [lib] name = "underhanded_c_nuke_lib".
    // tests run with the release/debug profile based on cargo's invocation.
    let candidates = [
        project_root().join("target/release/libunderhanded_c_nuke_lib.so"),
        project_root().join("target/debug/libunderhanded_c_nuke_lib.so"),
    ];
    for p in candidates.iter() {
        if p.exists() {
            return p.clone();
        }
    }
    panic!("Rust .so not built; run `cargo build` first");
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

fn call_match(lib: &Library, test: &[f64], reference: &[f64], threshold: f64) -> i32 {
    assert_eq!(test.len(), reference.len());
    let mut t = test.to_vec();
    let mut r = reference.to_vec();
    unsafe {
        let f: Symbol<MatchFn> = lib.get(b"match\0").expect("symbol match");
        f(t.as_mut_ptr(), r.as_mut_ptr(), t.len() as i32, threshold)
    }
}

fn call_spectral_contrast(lib: &Library, a: &[f32], b: &[f32]) -> f64 {
    assert_eq!(a.len(), b.len());
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    unsafe {
        let f: Symbol<SpectralContrastFn> =
            lib.get(b"spectral_contrast\0").expect("symbol spectral_contrast");
        f(a.as_mut_ptr(), b.as_mut_ptr(), a.len() as i32)
    }
}

// Helpers to bit-compare floating-point so NaN compares equal by bits.
fn f64_bits_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

#[test]
fn test_spectral_contrast_simple() {
    let (c, r) = load_libs();

    let cases: Vec<(Vec<f32>, Vec<f32>)> = vec![
        (vec![1.0, 2.0, 3.0, 4.0], vec![1.0, 2.0, 3.0, 4.0]),
        (vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]),
        (vec![1.0; 8], vec![2.0; 8]),
        (vec![0.5, 0.25, 0.125, 0.0625], vec![1.0, 1.0, 1.0, 1.0]),
        (
            vec![3.14159, 2.71828, 1.41421, 0.57721],
            vec![1.61803, 0.69315, 2.30259, 0.43429],
        ),
        (vec![-1.0, 2.0, -3.0, 4.0], vec![5.0, -6.0, 7.0, -8.0]),
        (vec![1e10, 1e10, 1e10], vec![1e-10, 1e-10, 1e-10]),
    ];

    for (i, (a, b)) in cases.iter().enumerate() {
        let cv = call_spectral_contrast(&c, a, b);
        let rv = call_spectral_contrast(&r, a, b);
        assert!(
            f64_bits_eq(cv, rv),
            "case {} mismatch: C={:?} ({:#x}) Rust={:?} ({:#x})",
            i,
            cv,
            cv.to_bits(),
            rv,
            rv.to_bits()
        );
    }
}

#[test]
fn test_spectral_contrast_random() {
    let (c, r) = load_libs();

    // Simple deterministic LCG for reproducible "random" floats.
    let mut state: u64 = 0xdead_beef_cafe_babe;
    let mut next_f32 = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = ((state >> 32) as u32) as f32 / (u32::MAX as f32);
        u * 100.0 - 50.0
    };

    for &length in &[1usize, 2, 5, 16, 17, 100, 257] {
        let a: Vec<f32> = (0..length).map(|_| next_f32()).collect();
        let b: Vec<f32> = (0..length).map(|_| next_f32()).collect();
        let cv = call_spectral_contrast(&c, &a, &b);
        let rv = call_spectral_contrast(&r, &a, &b);
        assert!(
            f64_bits_eq(cv, rv),
            "length {} mismatch: C={:?} ({:#x}) Rust={:?} ({:#x})",
            length,
            cv,
            cv.to_bits(),
            rv,
            rv.to_bits()
        );
    }
}

#[test]
fn test_match_basic() {
    let (c, r) = load_libs();

    // Identical signals: should match.
    let test: Vec<f64> = (0..32).map(|i| (i as f64).sin() + 1.0).collect();
    let reference = test.clone();

    for &thr in &[0.0_f64, 0.1, 0.5, 0.9, 1.0] {
        let cv = call_match(&c, &test, &reference, thr);
        let rv = call_match(&c, &test, &reference, thr); // sanity: c==c
        assert_eq!(cv, rv);
        let rv2 = call_match(&r, &test, &reference, thr);
        assert_eq!(cv, rv2, "match threshold={} mismatch C={} Rust={}", thr, cv, rv2);
    }
}

#[test]
fn test_match_total_threshold_path() {
    // A case where total(test) < threshold * total(reference), so match
    // should short-circuit and return 0.
    let (c, r) = load_libs();
    let test = vec![0.1; 8];
    let reference = vec![10.0; 8];
    let threshold = 0.5;
    let cv = call_match(&c, &test, &reference, threshold);
    let rv = call_match(&r, &test, &reference, threshold);
    assert_eq!(cv, rv);
}

#[test]
fn test_match_various_inputs() {
    let (c, r) = load_libs();

    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next_f64 = |scale: f64| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = ((state >> 32) as u32) as f64 / (u32::MAX as f64);
        u * scale
    };

    for &length in &[1usize, 2, 8, 16, 17, 32, 100, 257] {
        for _trial in 0..4 {
            let test: Vec<f64> = (0..length).map(|_| next_f64(10.0)).collect();
            let reference: Vec<f64> = (0..length).map(|_| next_f64(10.0)).collect();
            for &thr in &[0.0_f64, 0.25, 0.5, 0.75, 1.0] {
                let cv = call_match(&c, &test, &reference, thr);
                let rv = call_match(&r, &test, &reference, thr);
                assert_eq!(
                    cv, rv,
                    "match length={} thr={} cv={} rv={} test={:?} ref={:?}",
                    length, thr, cv, rv, test, reference
                );
            }
        }
    }
}

#[test]
fn test_match_zero_length_via_threshold() {
    // Length 1 edge case.
    let (c, r) = load_libs();
    let test = vec![1.0_f64];
    let reference = vec![1.0_f64];
    let cv = call_match(&c, &test, &reference, 0.5);
    let rv = call_match(&r, &test, &reference, 0.5);
    assert_eq!(cv, rv);
}
