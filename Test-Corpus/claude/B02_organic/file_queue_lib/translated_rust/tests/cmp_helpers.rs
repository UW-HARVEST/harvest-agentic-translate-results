//! Compare helper functions os_calloc / os_realloc / os_strdup / merror.

mod common;

use common::*;
use std::ffi::c_char;

#[test]
fn os_calloc_matches() {
    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnOsCalloc> = sym(&c, b"os_calloc");
        let rf: libloading::Symbol<FnOsCalloc> = sym(&r, b"os_calloc");

        let cp = cf(8, 4);
        let rp = rf(8, 4);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        // Calloc must zero memory.
        let cs = std::slice::from_raw_parts(cp as *const u8, 32);
        let rs = std::slice::from_raw_parts(rp as *const u8, 32);
        assert_eq!(cs, rs);
        assert!(cs.iter().all(|&b| b == 0));
        libc::free(cp);
        libc::free(rp);
    }
}

#[test]
fn os_realloc_matches() {
    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnOsRealloc> = sym(&c, b"os_realloc");
        let rf: libloading::Symbol<FnOsRealloc> = sym(&r, b"os_realloc");

        let cp = cf(std::ptr::null_mut(), 16);
        let rp = rf(std::ptr::null_mut(), 16);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        let cp = cf(cp, 64);
        let rp = rf(rp, 64);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        libc::free(cp);
        libc::free(rp);
    }
}

#[test]
fn os_strdup_matches() {
    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnOsStrdup> = sym(&c, b"os_strdup");
        let rf: libloading::Symbol<FnOsStrdup> = sym(&r, b"os_strdup");

        let s = b"hello, world\0".as_ptr() as *const c_char;
        let cp = cf(s);
        let rp = rf(s);
        let cs = cstr_to_string(cp).unwrap();
        let rs = cstr_to_string(rp).unwrap();
        assert_eq!(cs, rs);
        assert_eq!(cs, "hello, world");
        libc::free(cp as *mut _);
        libc::free(rp as *mut _);
    }
}

#[test]
fn merror_does_not_crash() {
    // We can't easily capture stderr from a dlopen'd lib, but we verify the
    // function exists and is callable without crashing.
    let c = load_c();
    let r = load_rust();
    unsafe {
        let cf: libloading::Symbol<FnMerror> = sym(&c, b"merror");
        let rf: libloading::Symbol<FnMerror> = sym(&r, b"merror");
        let tpl = b"file=%s err=%d msg=%s\0".as_ptr() as *const c_char;
        let f = b"foo\0".as_ptr() as *const c_char;
        let m = b"bar\0".as_ptr() as *const c_char;
        cf(tpl, f, 7, m);
        rf(tpl, f, 7, m);
    }
}
