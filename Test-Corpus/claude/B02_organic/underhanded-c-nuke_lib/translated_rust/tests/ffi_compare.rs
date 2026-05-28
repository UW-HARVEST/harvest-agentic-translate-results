// Integration tests comparing the C .so and Rust .so via libloading.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type SpectralContrastFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int) -> f64;
type MatchFn = unsafe extern "C" fn(*mut f64, *mut f64, c_int, f64) -> c_int;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    // The integration test runs after `cargo build` of the cdylib (cargo test
    // builds it automatically because it's part of the package).
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Determine whether the cargo test profile is debug or release. cargo test
    // uses debug by default; if running with --release, the user must provide
    // RUST_SO_DIR. We just probe.
    let candidates = [
        "debug/libunderhanded_c_nuke_lib.so",
        "release/libunderhanded_c_nuke_lib.so",
    ];
    for c in candidates {
        let mut q = p.clone();
        q.push(c);
        if q.exists() {
            return q;
        }
    }
    p.push("debug/libunderhanded_c_nuke_lib.so");
    p
}

fn load_libs() -> (Library, Library) {
    let c_path = c_so_path();
    let r_path = rust_so_path();
    assert!(
        c_path.exists(),
        "C .so missing at {:?}; build with cmake first",
        c_path
    );
    assert!(
        r_path.exists(),
        "Rust .so missing at {:?}; run `cargo build` first",
        r_path
    );
    unsafe { (Library::new(c_path).unwrap(), Library::new(r_path).unwrap()) }
}

fn call_spectral_contrast(lib: &Library, a: &mut [f64], b: &mut [f64]) -> f64 {
    unsafe {
        let f: Symbol<SpectralContrastFn> = lib.get(b"spectral_contrast\0").unwrap();
        f(a.as_mut_ptr(), b.as_mut_ptr(), a.len() as c_int)
    }
}

fn call_match(lib: &Library, test: &mut [f64], reference: &mut [f64], threshold: f64) -> c_int {
    unsafe {
        let f: Symbol<MatchFn> = lib.get(b"match\0").unwrap();
        f(
            test.as_mut_ptr(),
            reference.as_mut_ptr(),
            test.len() as c_int,
            threshold,
        )
    }
}

fn assert_bits_equal(a: f64, b: f64, ctx: &str) {
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "mismatch in {}: c={} ({:#x}) rust={} ({:#x})",
        ctx,
        a,
        a.to_bits(),
        b,
        b.to_bits()
    );
}

#[test]
fn spectral_contrast_basic() {
    let (c_lib, r_lib) = load_libs();
    let cases: Vec<(Vec<f64>, Vec<f64>)> = vec![
        (vec![1.0, 2.0, 3.0, 4.0], vec![1.0, 2.0, 3.0, 4.0]),
        (vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]),
        (vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]),
        (
            vec![0.5, 1.5, 2.5, 3.5, 4.5, 5.5],
            vec![5.5, 4.5, 3.5, 2.5, 1.5, 0.5],
        ),
        (
            (0..32).map(|i| (i as f64).sin().abs() + 0.001).collect(),
            (0..32).map(|i| (i as f64).cos().abs() + 0.001).collect(),
        ),
    ];

    for (i, (a, b)) in cases.into_iter().enumerate() {
        let mut a_c = a.clone();
        let mut b_c = b.clone();
        let mut a_r = a.clone();
        let mut b_r = b.clone();
        let c_out = call_spectral_contrast(&c_lib, &mut a_c, &mut b_c);
        let r_out = call_spectral_contrast(&r_lib, &mut a_r, &mut b_r);
        assert_bits_equal(c_out, r_out, &format!("spectral_contrast case {}", i));
        // also confirm the in-place mutations match
        for (j, (cv, rv)) in a_c.iter().zip(a_r.iter()).enumerate() {
            assert_bits_equal(*cv, *rv, &format!("a[{}] case {}", j, i));
        }
        for (j, (cv, rv)) in b_c.iter().zip(b_r.iter()).enumerate() {
            assert_bits_equal(*cv, *rv, &format!("b[{}] case {}", j, i));
        }
    }
}

#[test]
fn match_basic() {
    let (c_lib, r_lib) = load_libs();
    // bins at least N_SMOOTH (16) keeps the smoothing path interesting; smaller
    // sizes are also valid and exercise edge cases.
    let cases: Vec<(Vec<f64>, Vec<f64>, f64)> = vec![
        // identical -> should match
        (
            (0..32).map(|i| (i as f64).sin().abs() + 0.5).collect(),
            (0..32).map(|i| (i as f64).sin().abs() + 0.5).collect(),
            0.9,
        ),
        // very different
        (
            (0..32).map(|i| (i as f64).sin().abs() + 0.5).collect(),
            (0..32).map(|i| (i as f64).cos().abs() + 0.5).collect(),
            0.5,
        ),
        // total below threshold (early-exit)
        (vec![0.1; 16], vec![1.0; 16], 0.5),
        // small arrays
        (vec![1.0, 2.0, 3.0, 4.0], vec![1.0, 2.0, 3.0, 4.0], 0.5),
        (vec![1.0; 20], vec![2.0; 20], 0.6),
        // length 1 (differentiate edge case)
        (vec![5.0], vec![5.0], 0.5),
        // length 2
        (vec![1.0, 2.0], vec![3.0, 4.0], 0.5),
    ];

    for (i, (test, reference, thr)) in cases.into_iter().enumerate() {
        let mut tc = test.clone();
        let mut rc = reference.clone();
        let mut tr = test.clone();
        let mut rr = reference.clone();
        let c_out = call_match(&c_lib, &mut tc, &mut rc, thr);
        let r_out = call_match(&r_lib, &mut tr, &mut rr, thr);
        assert_eq!(c_out, r_out, "match case {}: thr={}", i, thr);
        // match takes pointers to const-ish data; should not mutate inputs.
        for (j, (cv, rv)) in tc.iter().zip(tr.iter()).enumerate() {
            assert_bits_equal(*cv, *rv, &format!("test[{}] case {}", j, i));
        }
        for (j, (cv, rv)) in rc.iter().zip(rr.iter()).enumerate() {
            assert_bits_equal(*cv, *rv, &format!("reference[{}] case {}", j, i));
        }
    }
}

#[test]
fn match_thresholds_sweep() {
    let (c_lib, r_lib) = load_libs();
    let test: Vec<f64> = (0..64).map(|i| ((i as f64) * 0.13).sin().abs() + 0.1).collect();
    let reference: Vec<f64> = (0..64).map(|i| ((i as f64) * 0.17).cos().abs() + 0.1).collect();
    for k in 0..21 {
        let thr = (k as f64) / 20.0; // 0.0 .. 1.0
        let mut tc = test.clone();
        let mut rc = reference.clone();
        let mut tr = test.clone();
        let mut rr = reference.clone();
        let c_out = call_match(&c_lib, &mut tc, &mut rc, thr);
        let r_out = call_match(&r_lib, &mut tr, &mut rr, thr);
        assert_eq!(c_out, r_out, "match threshold {}", thr);
    }
}
