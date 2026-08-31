//! Harness smoke tests: symbol loading, version strings, and that a
//! `png_error` unwind can be caught in both libraries.
mod common;
use common::*;

#[test]
fn version_strings_match() {
    unsafe {
        let c = c_api();
        let r = rs_api();
        assert_eq!(
            (c.png_access_version_number)(),
            (r.png_access_version_number)()
        );
        assert_eq!(
            rs_str((c.png_get_libpng_ver)(std::ptr::null())),
            rs_str((r.png_get_libpng_ver)(std::ptr::null()))
        );
        assert_eq!(
            rs_str((c.png_get_header_ver)(std::ptr::null())),
            rs_str((r.png_get_header_ver)(std::ptr::null()))
        );
        assert_eq!(
            rs_str((c.png_get_header_version)(std::ptr::null())),
            rs_str((r.png_get_header_version)(std::ptr::null()))
        );
        assert_eq!(
            rs_str((c.png_get_copyright)(std::ptr::null())),
            rs_str((r.png_get_copyright)(std::ptr::null()))
        );
    }
}

#[test]
fn srgb_tables_match() {
    unsafe {
        let c = c_api();
        let r = rs_api();
        let ct = std::slice::from_raw_parts(c.png_sRGB_table, 256);
        let rt = std::slice::from_raw_parts(r.png_sRGB_table, 256);
        assert_eq!(ct, rt);
        let cb = std::slice::from_raw_parts(c.png_sRGB_base, 512);
        let rb = std::slice::from_raw_parts(r.png_sRGB_base, 512);
        assert_eq!(cb, rb);
        let cd = std::slice::from_raw_parts(c.png_sRGB_delta, 512);
        let rd = std::slice::from_raw_parts(r.png_sRGB_delta, 512);
        assert_eq!(cd, rd);
    }
}

/// Verifies the panic-based longjmp emulation works through BOTH libraries.
#[test]
fn error_unwind_works() {
    for api in both() {
        unsafe {
            set_current_api(api);
            diag_reset();
            let v = ver();
            let png = (api.png_create_read_struct)(
                v.as_ptr(),
                std::ptr::null_mut(),
                Some(cb_error),
                Some(cb_warning),
            );
            assert!(!png.is_null(), "{}: create_read_struct failed", api.name);
            let msg = cs("deliberate");
            let r = guard(|| (api.png_error)(png, msg.as_ptr()));
            assert!(r.is_none(), "{}: png_error returned normally", api.name);
            let d = diag_take();
            assert_eq!(d.errors, vec!["deliberate".to_string()], "{}", api.name);
            let mut p = png;
            (api.png_destroy_read_struct)(
                &mut p,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}
