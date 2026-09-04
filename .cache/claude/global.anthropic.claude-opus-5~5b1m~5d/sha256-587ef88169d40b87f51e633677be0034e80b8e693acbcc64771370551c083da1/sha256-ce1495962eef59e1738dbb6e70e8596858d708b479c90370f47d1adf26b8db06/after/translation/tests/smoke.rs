#[macro_use]
mod common;

use core::ffi::{c_char, c_int};

#[test]
fn version_and_sizes() {
    let (c, r) = both!("sodium_version_string", unsafe extern "C" fn() -> *const c_char);
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c());
        let rs = std::ffi::CStr::from_ptr(r());
        assert_eq!(cs, rs);
    }
    let (c, r) = both!("sodium_library_version_major", unsafe extern "C" fn() -> c_int);
    assert_eq!(unsafe { c() }, unsafe { r() });
}

#[test]
fn sha256_basic() {
    let (c, r) = both!(
        "crypto_hash_sha256",
        unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int
    );
    let mut rng = common::Rng::new(1);
    for n in [0usize, 1, 55, 56, 64, 100, 1000] {
        let msg = rng.bytes(n);
        let mut co = [0u8; 32];
        let mut ro = [0u8; 32];
        let rc = unsafe { c(co.as_mut_ptr(), msg.as_ptr(), n as u64) };
        let rr = unsafe { r(ro.as_mut_ptr(), msg.as_ptr(), n as u64) };
        assert_eq!(rc, rr);
        common::eqb(&format!("sha256 n={}", n), &co, &ro);
    }
}
