//! Harness smoke test + Phase D symbol-parity assertion.
mod common;
use common::*;
use libc::{c_char, c_int, c_uchar};
use std::ffi::CStr;

#[test]
fn version_and_constants_match() {
    init_both();
    let (c, r) = fnpair!("sodium_version_string", unsafe extern "C" fn() -> *const c_char);
    let (cs, rs) = unsafe { (CStr::from_ptr(c()), CStr::from_ptr(r())) };
    assert_eq!(cs, rs, "sodium_version_string differs");

    for n in [
        "sodium_library_version_major",
        "sodium_library_version_minor",
        "sodium_library_minimal",
    ] {
        let (c, r) = unsafe { pair::<unsafe extern "C" fn() -> c_int>(n) };
        assert_eq!(unsafe { c() }, unsafe { r() }, "{n} differs");
    }
}

#[test]
fn verify_16_32_64_differential() {
    init_both();
    let mut rng = Rng::new(SEED);
    for (name, n) in [("crypto_verify_16", 16usize), ("crypto_verify_32", 32), ("crypto_verify_64", 64)] {
        let (c, r) =
            unsafe { pair::<unsafe extern "C" fn(*const c_uchar, *const c_uchar) -> c_int>(name) };
        for _ in 0..64 {
            let a = rng.bytes(n);
            // equal
            assert_eq!(unsafe { c(a.as_ptr(), a.as_ptr()) }, unsafe {
                r(a.as_ptr(), a.as_ptr())
            });
            // differ at every position
            for i in 0..n {
                let mut b = a.clone();
                b[i] ^= 0x80;
                let (rc, rr) = unsafe { (c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr())) };
                assert_eq!(rc, rr, "{name} differs at byte {i}");
            }
        }
        // all-zero vs all-ff
        let z = vec![0u8; n];
        let f = vec![0xffu8; n];
        assert_eq!(unsafe { c(z.as_ptr(), f.as_ptr()) }, unsafe {
            r(z.as_ptr(), f.as_ptr())
        });
    }
}

/// ERRORS row 2: `sodium_bin2hex` misuse when `hex_maxlen <= bin_len*2`.
/// Demonstrates the forked-abort mechanism used for every `misuse` row.
#[test]
fn forked_misuse_mechanism_works() {
    init_both();
    let l = libs();
    let run = |lib: &'static libloading::Library| -> Outcome {
        forked(|| {
            let f = unsafe {
                sym::<unsafe extern "C" fn(*mut c_char, usize, *const c_uchar, usize) -> *mut c_char>(
                    lib,
                    "sodium_bin2hex",
                )
            };
            let bin = [1u8, 2, 3];
            let mut out = [0u8; 8];
            // hex_maxlen == 6 == bin_len*2  -> must abort
            unsafe { f(out.as_mut_ptr() as *mut c_char, 6, bin.as_ptr(), 3) };
            0
        })
    };
    let (oc, or) = (run(&l.c), run(&l.r));
    assert_same_fatal("sodium_bin2hex hex_maxlen<=2*bin_len", oc, or);
    assert_eq!(oc, Outcome::Signaled(SIGABRT), "expected SIGABRT, got {oc:?}");
}

/// Phase D: the Rust `.so` must export every symbol the C `.so` exports.
#[test]
fn symbol_parity() {
    use std::process::Command;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nm = |p: &std::path::Path| -> Vec<String> {
        let o = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("run nm");
        let mut v: Vec<String> = String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let cs = nm(&root.join("c_src/build/libsodium.so"));
    let rs = {
        let d = root.join("target/debug/liblibsodium.so");
        let p = if d.exists() { d } else { root.join("target/release/liblibsodium.so") };
        nm(&p)
    };
    assert!(!cs.is_empty(), "nm produced no C symbols");
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by C but MISSING from Rust: {:?}",
        missing.len(),
        missing
    );
    eprintln!("symbol parity OK: {} C symbols, {} Rust symbols", cs.len(), rs.len());
}
