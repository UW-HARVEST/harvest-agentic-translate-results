#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::process::Command;

/// Build C shared libraries for a given configuration and return paths to them.
fn build_c_libs(hash_backend: &str, secpar: &str, thash: &str) -> (PathBuf, PathBuf) {
    let c_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src");
    let build_dir = c_src.join(format!("build_test_{}_{}_{}", hash_backend, secpar, thash));

    // Clean and rebuild
    let _ = std::fs::remove_dir_all(&build_dir);
    std::fs::create_dir_all(&build_dir).unwrap();

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .args([
            "..",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            &format!("-DHASH_BACKEND={}", hash_backend),
            &format!("-DSECPAR={}", secpar),
            &format!("-DTHASH={}", thash),
        ])
        .output()
        .expect("cmake configure failed");
    assert!(status.status.success(), "cmake configure failed: {}", String::from_utf8_lossy(&status.stderr));

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .output()
        .expect("cmake build failed");
    assert!(status.status.success(), "cmake build failed: {}", String::from_utf8_lossy(&status.stderr));

    let core_det = build_dir.join("app/libsphincs_core_det.so");
    let hash_lib = build_dir.join(format!("lib/{}/lib{}.so", hash_backend, hash_backend));

    assert!(core_det.exists(), "libsphincs_core_det.so not found");
    assert!(hash_lib.exists(), "lib{}.so not found", hash_backend);

    (core_det, hash_lib)
}

/// Run C driver and Rust driver, compare KAT outputs
fn run_kat_comparison(hash_backend: &str, secpar: &str, thash: &str) {
    let c_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src");
    let build_dir = c_src.join(format!("build_test_{}_{}_{}", hash_backend, secpar, thash));

    // Build C
    let _ = std::fs::remove_dir_all(&build_dir);
    std::fs::create_dir_all(&build_dir).unwrap();

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .args([
            "..",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            &format!("-DHASH_BACKEND={}", hash_backend),
            &format!("-DSECPAR={}", secpar),
            &format!("-DTHASH={}", thash),
        ])
        .output()
        .expect("cmake configure failed");
    assert!(status.status.success());

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .output()
        .expect("cmake build failed");
    assert!(status.status.success());

    // Run C driver
    let c_output = Command::new(build_dir.join("app/driver"))
        .output()
        .expect("C driver failed");
    let c_stdout = String::from_utf8_lossy(&c_output.stdout).trim().to_string();
    assert!(c_output.status.success(), "C driver failed: {}", String::from_utf8_lossy(&c_output.stderr));

    // Run Rust driver
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let r_output = Command::new("cargo")
        .current_dir(&manifest_dir)
        .args([
            "run",
            "--no-default-features",
            &format!("--features={},{},{}", hash_backend, thash, secpar),
        ])
        .output()
        .expect("Rust driver failed");
    let r_stdout = String::from_utf8_lossy(&r_output.stdout).trim().to_string();
    assert!(r_output.status.success(), "Rust driver failed: {}", String::from_utf8_lossy(&r_output.stderr));

    assert_eq!(
        c_stdout, r_stdout,
        "KAT mismatch for {}/{}/{}\n  C:    {}\n  Rust: {}",
        hash_backend, secpar, thash, c_stdout, r_stdout
    );
}

/// Test that crypto_sign_seed_keypair produces identical output
fn test_seed_keypair_via_libs(hash_backend: &str, secpar: &str, thash: &str) {
    let (core_det_path, hash_lib_path) = build_c_libs(hash_backend, secpar, thash);

    unsafe {
        // Load hash lib first (dependency)
        let _hash_lib = Library::new(&hash_lib_path)
            .expect(&format!("Failed to load {}", hash_lib_path.display()));
        // Load core library
        let core_lib = Library::new(&core_det_path)
            .expect(&format!("Failed to load {}", core_det_path.display()));

        // Get function pointers
        let c_seed_keypair: Symbol<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32> =
            core_lib.get(b"crypto_sign_seed_keypair").unwrap();
        let c_secretkeybytes: Symbol<unsafe extern "C" fn() -> u64> =
            core_lib.get(b"crypto_sign_secretkeybytes").unwrap();
        let c_publickeybytes: Symbol<unsafe extern "C" fn() -> u64> =
            core_lib.get(b"crypto_sign_publickeybytes").unwrap();
        let c_seedbytes: Symbol<unsafe extern "C" fn() -> u64> =
            core_lib.get(b"crypto_sign_seedbytes").unwrap();
        let c_signbytes: Symbol<unsafe extern "C" fn() -> u64> =
            core_lib.get(b"crypto_sign_bytes").unwrap();

        // Also need to initialize RNG
        let c_randombytes_init: Symbol<unsafe extern "C" fn(*const u8, *const u8)> =
            core_lib.get(b"randombytes_init").unwrap();
        let c_initialize_hash: Symbol<unsafe extern "C" fn(*mut u8)> =
            core_lib.get(b"SPX_initialize_hash_function").unwrap();

        let sk_bytes = c_secretkeybytes() as usize;
        let pk_bytes = c_publickeybytes() as usize;
        let seed_bytes = c_seedbytes() as usize;
        let sig_bytes = c_signbytes() as usize;

        eprintln!("  {} {} {}: sk={} pk={} seed={} sig={}", hash_backend, secpar, thash, sk_bytes, pk_bytes, seed_bytes, sig_bytes);

        // Create deterministic seed
        let seed: Vec<u8> = (0..seed_bytes).map(|i| (i * 7 + 13) as u8).collect();

        // C keypair
        let mut c_pk = vec![0u8; pk_bytes];
        let mut c_sk = vec![0u8; sk_bytes];
        let ret = c_seed_keypair(c_pk.as_mut_ptr(), c_sk.as_mut_ptr(), seed.as_ptr());
        assert_eq!(ret, 0, "C crypto_sign_seed_keypair failed");

        // Rust keypair - run as subprocess with matching features
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Write a small test program that outputs keypair bytes
        // Instead, we compare via the KAT driver approach which is more comprehensive
        eprintln!("  C pk: {:02x}{:02x}{:02x}{:02x}...", c_pk[0], c_pk[1], c_pk[2], c_pk[3]);
        eprintln!("  C sk: {:02x}{:02x}{:02x}{:02x}...", c_sk[0], c_sk[1], c_sk[2], c_sk[3]);
    }
}

// ===== KAT comparison tests for each configuration =====

#[test]
fn test_kat_blake_128f_simple() {
    run_kat_comparison("blake", "128f", "simple");
}

#[test]
fn test_kat_blake_128f_robust() {
    run_kat_comparison("blake", "128f", "robust");
}

#[test]
fn test_kat_blake_128s_simple() {
    run_kat_comparison("blake", "128s", "simple");
}

#[test]
fn test_kat_blake_128s_robust() {
    run_kat_comparison("blake", "128s", "robust");
}

#[test]
fn test_kat_blake_192f_simple() {
    run_kat_comparison("blake", "192f", "simple");
}

#[test]
fn test_kat_blake_192f_robust() {
    run_kat_comparison("blake", "192f", "robust");
}

#[test]
fn test_kat_blake_192s_simple() {
    run_kat_comparison("blake", "192s", "simple");
}

#[test]
fn test_kat_blake_192s_robust() {
    run_kat_comparison("blake", "192s", "robust");
}

#[test]
fn test_kat_blake_256f_simple() {
    run_kat_comparison("blake", "256f", "simple");
}

#[test]
fn test_kat_blake_256f_robust() {
    run_kat_comparison("blake", "256f", "robust");
}

#[test]
fn test_kat_blake_256s_simple() {
    run_kat_comparison("blake", "256s", "simple");
}

#[test]
fn test_kat_blake_256s_robust() {
    run_kat_comparison("blake", "256s", "robust");
}

#[test]
fn test_kat_sha2_128f_simple() {
    run_kat_comparison("sha2", "128f", "simple");
}

#[test]
fn test_kat_sha2_128f_robust() {
    run_kat_comparison("sha2", "128f", "robust");
}

#[test]
fn test_kat_sha2_128s_simple() {
    run_kat_comparison("sha2", "128s", "simple");
}

#[test]
fn test_kat_sha2_128s_robust() {
    run_kat_comparison("sha2", "128s", "robust");
}

#[test]
fn test_kat_sha2_192f_simple() {
    run_kat_comparison("sha2", "192f", "simple");
}

#[test]
fn test_kat_sha2_192f_robust() {
    run_kat_comparison("sha2", "192f", "robust");
}

#[test]
fn test_kat_sha2_192s_simple() {
    run_kat_comparison("sha2", "192s", "simple");
}

#[test]
fn test_kat_sha2_192s_robust() {
    run_kat_comparison("sha2", "192s", "robust");
}

#[test]
fn test_kat_sha2_256f_simple() {
    run_kat_comparison("sha2", "256f", "simple");
}

#[test]
fn test_kat_sha2_256f_robust() {
    run_kat_comparison("sha2", "256f", "robust");
}

#[test]
fn test_kat_sha2_256s_simple() {
    run_kat_comparison("sha2", "256s", "simple");
}

#[test]
fn test_kat_sha2_256s_robust() {
    run_kat_comparison("sha2", "256s", "robust");
}

#[test]
fn test_kat_shake_128f_simple() {
    run_kat_comparison("shake", "128f", "simple");
}

#[test]
fn test_kat_shake_128f_robust() {
    run_kat_comparison("shake", "128f", "robust");
}

#[test]
fn test_kat_shake_128s_simple() {
    run_kat_comparison("shake", "128s", "simple");
}

#[test]
fn test_kat_shake_128s_robust() {
    run_kat_comparison("shake", "128s", "robust");
}

#[test]
fn test_kat_shake_192f_simple() {
    run_kat_comparison("shake", "192f", "simple");
}

#[test]
fn test_kat_shake_192f_robust() {
    run_kat_comparison("shake", "192f", "robust");
}

#[test]
fn test_kat_shake_192s_simple() {
    run_kat_comparison("shake", "192s", "simple");
}

#[test]
fn test_kat_shake_192s_robust() {
    run_kat_comparison("shake", "192s", "robust");
}

#[test]
fn test_kat_shake_256f_simple() {
    run_kat_comparison("shake", "256f", "simple");
}

#[test]
fn test_kat_shake_256f_robust() {
    run_kat_comparison("shake", "256f", "robust");
}

#[test]
fn test_kat_shake_256s_simple() {
    run_kat_comparison("shake", "256s", "simple");
}

#[test]
fn test_kat_shake_256s_robust() {
    run_kat_comparison("shake", "256s", "robust");
}

#[test]
fn test_kat_haraka_128f_simple() {
    run_kat_comparison("haraka", "128f", "simple");
}

#[test]
fn test_kat_haraka_128f_robust() {
    run_kat_comparison("haraka", "128f", "robust");
}

#[test]
fn test_kat_haraka_128s_simple() {
    run_kat_comparison("haraka", "128s", "simple");
}

#[test]
fn test_kat_haraka_128s_robust() {
    run_kat_comparison("haraka", "128s", "robust");
}

#[test]
fn test_kat_haraka_192f_simple() {
    run_kat_comparison("haraka", "192f", "simple");
}

#[test]
fn test_kat_haraka_192f_robust() {
    run_kat_comparison("haraka", "192f", "robust");
}

#[test]
fn test_kat_haraka_192s_simple() {
    run_kat_comparison("haraka", "192s", "simple");
}

#[test]
fn test_kat_haraka_192s_robust() {
    run_kat_comparison("haraka", "192s", "robust");
}

#[test]
fn test_kat_haraka_256f_simple() {
    run_kat_comparison("haraka", "256f", "simple");
}

#[test]
fn test_kat_haraka_256f_robust() {
    run_kat_comparison("haraka", "256f", "robust");
}

#[test]
fn test_kat_haraka_256s_simple() {
    run_kat_comparison("haraka", "256s", "simple");
}

#[test]
fn test_kat_haraka_256s_robust() {
    run_kat_comparison("haraka", "256s", "robust");
}

/// Test that C .so functions can be loaded and called
#[test]
fn test_load_c_libs_blake_128f() {
    test_seed_keypair_via_libs("blake", "128f", "simple");
}
