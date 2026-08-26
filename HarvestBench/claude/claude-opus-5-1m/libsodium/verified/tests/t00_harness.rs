//! Harness self-check: both `.so`s load, the deterministic RNG is installed in
//! both, and RNG-dependent entry points therefore agree byte-for-byte.

mod common;
use common::*;

#[test]
fn harness_loads_both_libraries() {
    setup();
    let (c, r) = pair::<unsafe extern "C" fn() -> usize>("crypto_box_macbytes");
    unsafe {
        assert_eq!(c(), 16);
        assert_eq!(r(), 16);
    }
}

#[test]
fn deterministic_rng_is_installed_in_both() {
    setup();
    // randombytes_buf must yield the same stream on both sides after a reset.
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, usize)>("randombytes_buf");
    for n in [1usize, 7, 8, 9, 31, 32, 33, 64, 100, 1000] {
        let mut a = vec![0u8; n];
        let mut b = vec![0u8; n];
        reset_rngs(RNG_SEED_BASE ^ n as u64);
        unsafe { c(a.as_mut_ptr(), n) };
        reset_rngs(RNG_SEED_BASE ^ n as u64);
        unsafe { r(b.as_mut_ptr(), n) };
        eq_bytes(&format!("randombytes_buf({n})"), &a, &b);
    }
}

#[test]
fn keygen_is_deterministic_and_matches() {
    setup();
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8)>("crypto_secretbox_keygen");
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    reset_rngs(99);
    unsafe { c(a.as_mut_ptr()) };
    reset_rngs(99);
    unsafe { r(b.as_mut_ptr()) };
    eq_bytes("crypto_secretbox_keygen", &a, &b);
    assert_ne!(a, [0u8; 32]);
}

#[test]
fn version_strings_match() {
    setup();
    let (c, r) = pair::<unsafe extern "C" fn() -> *const std::ffi::c_char>("sodium_version_string");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs, "sodium_version_string");
    }
}
