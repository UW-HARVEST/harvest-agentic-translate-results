//! The exported `*_implementation` data symbols.
//!
//! This lives in its own test binary because installing a different
//! `randombytes` implementation mutates global library state, which would race
//! with the deterministic-RNG tests if they shared a process.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// exported `*_implementation` data symbols
// ---------------------------------------------------------------------------

#[test]
fn exported_implementation_structs_match() {
    let l = libs();
    // Each of these is exported data (a struct of function pointers). Verify
    // both libraries export a symbol of that name and that it is non-NULL.
    for name in [
        "aegis128l_soft_implementation",
        "aegis256_soft_implementation",
        "crypto_onetimeauth_poly1305_donna_implementation",
        "crypto_scalarmult_curve25519_ref10_implementation",
        "crypto_stream_chacha20_ref_implementation",
        "crypto_stream_salsa20_ref_implementation",
        "ipcrypt_soft_implementation",
        "randombytes_internal_implementation",
        "randombytes_sysrandom_implementation",
    ] {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            let c = l.c.get::<*mut c_void>(&n);
            let r = l.rs.get::<*mut c_void>(&n);
            assert!(c.is_ok(), "C .so missing data symbol {name}");
            assert!(r.is_ok(), "Rust .so missing data symbol {name}");
            assert!(!c.unwrap().is_null(), "C {name} resolves to NULL");
            assert!(!r.unwrap().is_null(), "Rust {name} resolves to NULL");
        }
    }

    // The two randombytes implementations are usable as
    // randombytes_set_implementation arguments; their names must agree. Install
    // each in turn, read the name back, then restore the deterministic one.
    unsafe {
        type FnSet = unsafe extern "C" fn(*const c_void) -> c_int;
        type FnName = unsafe extern "C" fn() -> *const c_char;
        let (cset, rset): (FnSet, FnSet) = pair("randombytes_set_implementation");
        let (cname, rname): (FnName, FnName) = pair("randombytes_implementation_name");
        for name in [
            "randombytes_internal_implementation",
            "randombytes_sysrandom_implementation",
        ] {
            let mut n = name.as_bytes().to_vec();
            n.push(0);
            let cimpl = *l.c.get::<*const c_void>(&n).unwrap();
            let rimpl = *l.rs.get::<*const c_void>(&n).unwrap();
            assert_eq!(cset(cimpl), rset(rimpl), "set_implementation({name}) return");
            let cs = std::ffi::CStr::from_ptr(cname()).to_bytes().to_vec();
            let rs = std::ffi::CStr::from_ptr(rname()).to_bytes().to_vec();
            assert_eq!(
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs),
                "implementation_name after installing {name}"
            );
        }
        // randombytes_stir / randombytes_close on the real implementations
        type FnVoid = unsafe extern "C" fn();
        type FnIntFn = unsafe extern "C" fn() -> c_int;
        let (cstir, rstir): (FnVoid, FnVoid) = pair("randombytes_stir");
        cstir();
        rstir();
        let (cclose, rclose): (FnIntFn, FnIntFn) = pair("randombytes_close");
        assert_eq!(cclose(), rclose(), "randombytes_close return");
    }
    // The randombytes implementation pointer is global library state, so put
    // the deterministic one back for the rest of this test binary.
    restore_det_rng();
}

