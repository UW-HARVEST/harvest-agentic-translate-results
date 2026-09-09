//! Harness smoke test: both .so files load, symbols resolve, and an
//! error-callback unwind propagates out of C and Rust frames alike.
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_int, c_void};

type FnCreateRead = unsafe extern "C-unwind" fn(
    png_const_charp,
    png_voidp,
    Option<PngErrorFn>,
    Option<PngErrorFn>,
) -> png_structp;
type FnDestroyRead =
    unsafe extern "C-unwind" fn(*mut png_structp, *mut png_infop, *mut png_infop);
type FnVersion = unsafe extern "C-unwind" fn() -> png_uint_32;
type FnSetSigBytes = unsafe extern "C-unwind" fn(png_structp, c_int);

#[test]
fn version_matches() {
    let (a, b) = both(|l| unsafe { sym::<FnVersion>(l, "png_access_version_number")() });
    assert_eq!(a, b);
    assert_eq!(a, 10659);
}

#[test]
fn error_unwind_works_through_both_libraries() {
    for (name, lib) in [("C", &libs().c), ("Rust", &libs().rs)] {
        let r = capture_strict(|| unsafe {
            let create: Symbol<FnCreateRead> = sym(lib, "png_create_read_struct");
            let pp = create(
                PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
                std::ptr::null_mut::<c_void>(),
                Some(rec_error),
                Some(rec_warning),
            );
            assert!(!pp.is_null(), "{name}: create failed");
            // png_set_sig_bytes(pp, 9) -> png_error "Too many bytes for PNG signature"
            let f: Symbol<FnSetSigBytes> = sym(lib, "png_set_sig_bytes");
            f(pp, 9);
            let d: Symbol<FnDestroyRead> = sym(lib, "png_destroy_read_struct");
            let mut p = pp;
            d(&mut p, std::ptr::null_mut(), std::ptr::null_mut());
            0u32
        });
        assert!(r.out.is_none(), "{name}: expected the error to unwind");
        assert_eq!(
            r.log,
            vec![Msg::Err(b"Too many bytes for PNG signature".to_vec())],
            "{name}: message log"
        );
    }
}
