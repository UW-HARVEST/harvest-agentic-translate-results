// Tests for shape_* functions, comparing C and Rust implementations.

#[path = "common.rs"]
mod common;

use common::*;
use std::ffi::CStr;
use std::ffi::c_int;

fn cstr_to_string(p: *const std::ffi::c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { Some(CStr::from_ptr(p).to_string_lossy().into_owned()) }
}

#[test]
fn shape_type_name_matches() {
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    for i in -2..=12 {
        let cn = cstr_to_string(unsafe { (c.shape_type_name)(i) });
        let rn = cstr_to_string(unsafe { (r.shape_type_name)(i) });
        assert_eq!(cn, rn, "shape_type_name({}) mismatch", i);
    }
}

#[test]
fn shape_manager_init_and_get() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        for i in 0..SHAPE_COUNT {
            let cs = (c.shape_get)(i);
            let rs = (r.shape_get)(i);
            assert!(!cs.is_null(), "C shape {} should not be null", i);
            assert!(!rs.is_null(), "Rust shape {} should not be null", i);
            // Same shape_type
            assert_eq!((*cs).shape_type, (*rs).shape_type, "shape_type[{}]", i);
            // Same height/width
            assert_eq!((*cs).height, (*rs).height, "height[{}]", i);
            assert_eq!((*cs).width, (*rs).width, "width[{}]", i);
            // Name bytes
            let cname = buf_to_string(&(*cs).name);
            let rname = buf_to_string(&(*rs).name);
            assert_eq!(cname, rname, "name[{}]", i);
            // Art rows up to height
            for row in 0..(*cs).height as usize {
                let crow = buf_to_string(&(*cs).art[row]);
                let rrow = buf_to_string(&(*rs).art[row]);
                assert_eq!(crow, rrow, "art[{}][{}]", i, row);
            }
        }

        // Out-of-range should return null.
        assert!((c.shape_get)(-1).is_null());
        assert!((r.shape_get)(-1).is_null());
        assert!((c.shape_get)(SHAPE_COUNT).is_null());
        assert!((r.shape_get)(SHAPE_COUNT).is_null());

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}

#[test]
fn shape_equals_matches() {
    let _g = common::serialize();
    let c = ApiSyms::load(&c_lib_path());
    let r = ApiSyms::load(&rust_lib_path());

    unsafe {
        (c.shape_manager_init)();
        (r.shape_manager_init)();

        for i in 0..SHAPE_COUNT {
            for j in 0..SHAPE_COUNT {
                let ci = (c.shape_get)(i);
                let cj = (c.shape_get)(j);
                let ri = (r.shape_get)(i);
                let rj = (r.shape_get)(j);
                let cv = (c.shape_equals)(ci, cj);
                let rv = (r.shape_equals)(ri, rj);
                let expected: c_int = if i == j { 1 } else { 0 };
                assert_eq!(cv, expected, "C shape_equals({},{})", i, j);
                assert_eq!(rv, expected, "Rust shape_equals({},{})", i, j);
            }
        }

        // Null pointers: C uses pointer equality, so equal nulls -> 1.
        let c_null = (c.shape_equals)(std::ptr::null(), std::ptr::null());
        let r_null = (r.shape_equals)(std::ptr::null(), std::ptr::null());
        assert_eq!(c_null, r_null);

        (c.shape_manager_cleanup)();
        (r.shape_manager_cleanup)();
    }
}
