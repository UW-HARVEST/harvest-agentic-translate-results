mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

#[test]
fn both_libraries_load_and_init() {
    // `libs()` is initialised lazily by `both()`.
    let (c, r) = both::<unsafe extern "C" fn() -> *const c_char>("sodium_version_string");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c()).to_owned();
        let rs = std::ffi::CStr::from_ptr(r()).to_owned();
        assert_eq!(cs, rs);
    }
}

#[test]
fn deterministic_rng_is_wired_into_both() {
    let (c, r) = both::<unsafe extern "C" fn(*mut c_void, usize)>("randombytes_buf");
    rng_reset();
    let mut a = vec![0u8; 137];
    let mut b = vec![0u8; 137];
    unsafe {
        c(a.as_mut_ptr() as *mut c_void, a.len());
        r(b.as_mut_ptr() as *mut c_void, b.len());
    }
    eqb("randombytes_buf", &a, &b);
    assert!(a.iter().any(|&x| x != 0));
}

#[test]
fn sodium_init_is_idempotent_in_both() {
    let (c, r) = both::<unsafe extern "C" fn() -> c_int>("sodium_init");
    unsafe {
        eqi("sodium_init (second call)", c(), r());
    }
}
