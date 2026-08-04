// FFI cross-comparison: load both the C shared library and the Rust shared
// library and call individual functions through libloading, comparing
// results byte-for-byte.
//
// Note: a full keypair/sign/open round-trip cannot easily be done in a
// single test process because both shared libraries export overlapping
// crypto_* symbols and the dynamic loader will collapse them in unhelpful
// ways. The byte-identical equivalence is verified instead by comparing
// the driver binaries' stdout for every feature combination — see
// run_combo.sh in the repo root.

use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_LAZY};
use libloading::{Library, Symbol};
use std::path::PathBuf;

fn rust_libname() -> PathBuf {
    PathBuf::from("target/debug/libsphincs_plus.so")
}

fn c_backend_lib() -> PathBuf {
    let backend = if cfg!(feature = "haraka") {
        "haraka"
    } else if cfg!(feature = "sha2") {
        "sha2"
    } else if cfg!(feature = "shake") {
        "shake"
    } else if cfg!(feature = "blake") {
        "blake"
    } else {
        panic!("no backend feature enabled")
    };
    PathBuf::from(format!("c_src/build/lib/{0}/lib{0}.so", backend))
}

#[test]
fn random_consistent_alone() {
    // randombytes alone is straightforward — load each lib in isolation.
    unsafe {
        let _crypto = UnixLib::open(Some("libcrypto.so.10"), RTLD_LAZY | RTLD_GLOBAL).unwrap();
        let lib_c = Library::new("c_src/build/app/libsphincs_core_det.so").unwrap();
        let init_c: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> = lib_c.get(b"randombytes_init").unwrap();
        let rb_c: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> = lib_c.get(b"randombytes").unwrap();
        let mut entropy = [0u8; 48];
        for i in 0..48 { entropy[i] = i as u8; }
        let mut c_out = [0u8; 200];
        init_c(entropy.as_mut_ptr(), core::ptr::null_mut());
        rb_c(c_out.as_mut_ptr(), 200);
        // Drop C lib before loading Rust to avoid global collision.
        drop(rb_c);
        drop(init_c);
        drop(lib_c);

        let lib_r = Library::new(rust_libname()).unwrap();
        let init_r: Symbol<unsafe extern "C" fn(*mut u8, *mut u8)> = lib_r.get(b"randombytes_init").unwrap();
        let rb_r: Symbol<unsafe extern "C" fn(*mut u8, u64) -> i32> = lib_r.get(b"randombytes").unwrap();
        let mut r_out = [0u8; 200];
        init_r(entropy.as_mut_ptr(), core::ptr::null_mut());
        rb_r(r_out.as_mut_ptr(), 200);
        assert_eq!(c_out, r_out);
    }
}

// Per-backend hash function tests live in dedicated test files
// (e.g. blake_ffi.rs).
//
// The end-to-end equivalence is verified by run_combo.sh which builds
// both binaries for every feature combination and compares stdout.
#[allow(dead_code)]
fn _suppress_unused() {
    let _ = (RTLD_GLOBAL, RTLD_LAZY, c_backend_lib);
    let _: fn() -> PathBuf = c_backend_lib;
    let _ = UnixLib::this();
}
