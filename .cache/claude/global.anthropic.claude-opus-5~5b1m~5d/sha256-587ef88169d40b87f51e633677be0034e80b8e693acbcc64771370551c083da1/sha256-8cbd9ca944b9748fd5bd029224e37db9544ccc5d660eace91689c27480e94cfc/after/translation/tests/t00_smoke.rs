mod common;
use common::*;

#[test]
fn both_libraries_load_and_init() {
    let l = libs();
    let _ = l.c;
    let _ = l.rs;
}

#[test]
fn version_strings_match() {
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn() -> *const std::os::raw::c_char>(
            "sodium_version_string",
        );
        let cs = std::ffi::CStr::from_ptr(c()).to_owned();
        let rs = std::ffi::CStr::from_ptr(r()).to_owned();
        assert_eq!(cs, rs);

        for n in [
            "sodium_library_version_major",
            "sodium_library_version_minor",
        ] {
            let (c, r) = pair::<unsafe extern "C" fn() -> i32>(n);
            eq_i32(n, c(), r());
        }
        let (c, r) = pair::<unsafe extern "C" fn() -> i32>("sodium_library_minimal");
        eq_i32("sodium_library_minimal", c(), r());
    }
}

#[test]
fn deterministic_rng_install_and_agree() {
    install_det_random();
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn() -> *const std::os::raw::c_char>(
            "randombytes_implementation_name",
        );
        let cs = std::ffi::CStr::from_ptr(c()).to_owned();
        let rs = std::ffi::CStr::from_ptr(r()).to_owned();
        assert_eq!(cs.to_bytes(), b"harness_det");
        assert_eq!(cs, rs);
    }
}

#[test]
fn generichash_smoke() {
    unsafe {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32,
        >("crypto_generichash");
        let msg = b"abc";
        let mut co = [0u8; 32];
        let mut ro = [0u8; 32];
        eq_i32(
            "crypto_generichash",
            c(co.as_mut_ptr(), 32, msg.as_ptr(), 3, std::ptr::null(), 0),
            r(ro.as_mut_ptr(), 32, msg.as_ptr(), 3, std::ptr::null(), 0),
        );
        eq_bytes("crypto_generichash", &co, &ro);
    }
}
