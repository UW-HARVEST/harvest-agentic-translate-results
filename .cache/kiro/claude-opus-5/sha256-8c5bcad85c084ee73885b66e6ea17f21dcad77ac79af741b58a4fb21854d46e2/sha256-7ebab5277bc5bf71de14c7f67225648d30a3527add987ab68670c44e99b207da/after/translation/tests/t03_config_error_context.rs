//! `pcre2_config.c`, `pcre2_error.c` and `pcre2_context.c`.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

type ConfigFn = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
type GetErrMsgFn = unsafe extern "C" fn(c_int, *mut PCRE2_UCHAR, PCRE2_SIZE) -> c_int;

#[test]
fn config_matches() {
    let (c, r) = both::<ConfigFn>("pcre2_config_8");
    // Every documented option, plus a few out-of-range values.
    for what in 0u32..=32 {
        unsafe {
            // First: query the required size with a NULL argument.
            let sc = c(what, std::ptr::null_mut());
            let sr = r(what, std::ptr::null_mut());
            assert_eq!(sc, sr, "config({what}) NULL size");
            if sc < 0 {
                continue;
            }
            // Then fetch the value into a generously sized, pre-poisoned buffer.
            let mut bc = [0xAAu8; 256];
            let mut br = [0xAAu8; 256];
            let vc = c(what, bc.as_mut_ptr() as *mut c_void);
            let vr = r(what, br.as_mut_ptr() as *mut c_void);
            assert_eq!(vc, vr, "config({what}) return");
            assert_bytes_eq(&format!("config({what}) buffer"), &bc, &br);
        }
    }
    // Also check that huge/negative-looking selectors agree.
    for what in [0x7fff_ffffu32, 0xffff_ffff, 1000, 100] {
        unsafe {
            let mut bc = [0xAAu8; 256];
            let mut br = [0xAAu8; 256];
            assert_eq!(
                c(what, bc.as_mut_ptr() as *mut c_void),
                r(what, br.as_mut_ptr() as *mut c_void),
                "config({what})"
            );
            assert_bytes_eq(&format!("config({what}) buffer"), &bc, &br);
        }
    }
}

#[test]
fn get_error_message_matches() {
    let (c, r) = both::<GetErrMsgFn>("pcre2_get_error_message_8");
    // Compile errors are 100..=199, match/other errors are negative.
    let codes: Vec<c_int> = (-200..=200).collect();
    for code in codes {
        for &size in &[0usize, 1, 2, 3, 8, 16, 64, 256] {
            let mut bc = [0xAAu8; 300];
            let mut br = [0xAAu8; 300];
            unsafe {
                let rc = c(code, bc.as_mut_ptr(), size);
                let rr = r(code, br.as_mut_ptr(), size);
                assert_eq!(rc, rr, "get_error_message({code}, size={size}) return");
                assert_bytes_eq(
                    &format!("get_error_message({code}, size={size}) buffer"),
                    &bc,
                    &br,
                );
            }
        }
    }
}

/* ------------------------------- contexts ------------------------------- */

type GCtxCreateFn = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> *mut c_void;
type CtxCreateFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxCopyFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFreeFn = unsafe extern "C" fn(*mut c_void);
type SetU32Fn = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type SetSizeFn = unsafe extern "C" fn(*mut c_void, PCRE2_SIZE) -> c_int;
type SetPtrFn = unsafe extern "C" fn(*mut c_void, *const u8) -> c_int;

/// Compare a context's bytes, zeroing the ranges that hold addresses.
unsafe fn cmp_ctx(label: &str, a: *const c_void, b: *const c_void, size: usize, skip: &[(usize, usize)]) {
    unsafe {
        let mut x = slice_at(a as *const u8, size).to_vec();
        let mut y = slice_at(b as *const u8, size).to_vec();
        for &(off, len) in skip {
            for i in off..off + len {
                x[i] = 0;
                y[i] = 0;
            }
        }
        assert_bytes_eq(label, &x, &y);
    }
}

const COMPILE_CTX_SIZE: usize = 0x58;
const MATCH_CTX_SIZE: usize = 0x60;
const CONVERT_CTX_SIZE: usize = 0x20;
/// memctl.malloc + memctl.free hold addresses of each library's own defaults.
const MEMCTL_SKIP: &[(usize, usize)] = &[(0, 16)];
/// Compile contexts also carry a `tables` pointer.
const COMPILE_SKIP: &[(usize, usize)] = &[(0, 16), (40, 8)];

#[test]
fn general_context_create_and_copy() {
    let (cc, rc) = both::<GCtxCreateFn>("pcre2_general_context_create_8");
    let (ccp, rcp) = both::<CtxCopyFn>("pcre2_general_context_copy_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_general_context_free_8");
    unsafe {
        let gc = cc(None, None, std::ptr::null_mut());
        let gr = rc(None, None, std::ptr::null_mut());
        assert!(!gc.is_null() && !gr.is_null());
        cmp_ctx("general_context", gc, gr, 24, MEMCTL_SKIP);

        let gc2 = ccp(gc);
        let gr2 = rcp(gr);
        cmp_ctx("general_context copy", gc2, gr2, 24, MEMCTL_SKIP);

        // Freeing NULL must behave identically (copying NULL is UB in the C code).
        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());

        cf(gc2);
        rf(gr2);
        cf(gc);
        rf(gr);
    }
}

#[test]
fn compile_context_setters() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_compile_context_create_8");
    let (ccp, rcp) = both::<CtxCopyFn>("pcre2_compile_context_copy_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_compile_context_free_8");
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        assert!(!a.is_null() && !b.is_null());
        cmp_ctx("fresh compile_context", a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);

        // bsr
        let (sc, sr) = both::<SetU32Fn>("pcre2_set_bsr_8");
        for v in 0u32..=4 {
            assert_eq!(sc(a, v), sr(b, v), "set_bsr({v})");
            cmp_ctx(&format!("after set_bsr({v})"), a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);
        }
        // newline
        let (sc, sr) = both::<SetU32Fn>("pcre2_set_newline_8");
        for v in 0u32..=8 {
            assert_eq!(sc(a, v), sr(b, v), "set_newline({v})");
            cmp_ctx(&format!("after set_newline({v})"), a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);
        }
        // parens nest limit / extra options / max varlookbehind / optimize
        for name in [
            "pcre2_set_parens_nest_limit_8",
            "pcre2_set_compile_extra_options_8",
            "pcre2_set_max_varlookbehind_8",
        ] {
            let (sc, sr) = both::<SetU32Fn>(name);
            for v in [0u32, 1, 7, 250, 0xffff, 0xffff_ffff] {
                assert_eq!(sc(a, v), sr(b, v), "{name}({v})");
                cmp_ctx(&format!("after {name}({v})"), a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);
            }
        }
        // set_optimize validates its argument.
        let (sc, sr) = both::<SetU32Fn>("pcre2_set_optimize_8");
        for v in 0u32..=6 {
            assert_eq!(sc(a, v), sr(b, v), "set_optimize({v})");
            cmp_ctx(&format!("after set_optimize({v})"), a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);
        }
        for v in [100u32, 0xffff_ffff] {
            assert_eq!(sc(a, v), sr(b, v), "set_optimize({v})");
        }
        // sizes
        for name in [
            "pcre2_set_max_pattern_length_8",
            "pcre2_set_max_pattern_compiled_length_8",
        ] {
            let (sc, sr) = both::<SetSizeFn>(name);
            for v in [0usize, 1, 1000, usize::MAX, usize::MAX - 1] {
                assert_eq!(sc(a, v), sr(b, v), "{name}({v})");
                cmp_ctx(&format!("after {name}({v})"), a, b, COMPILE_CTX_SIZE, COMPILE_SKIP);
            }
        }
        // character tables: pass NULL and each library's own default tables.
        let (sc, sr) = both::<SetPtrFn>("pcre2_set_character_tables_8");
        assert_eq!(sc(a, std::ptr::null()), sr(b, std::ptr::null()));
        let (cdt, rdt) = both_data("_pcre2_default_tables_8");
        assert_eq!(sc(a, cdt), sr(b, rdt));

        // A copy must reproduce the same state.
        let a2 = ccp(a);
        let b2 = rcp(b);
        cmp_ctx("compile_context copy", a2, b2, COMPILE_CTX_SIZE, COMPILE_SKIP);

        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
        cf(a2);
        rf(b2);
        cf(a);
        rf(b);
    }
}

#[test]
fn match_context_setters() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_match_context_create_8");
    let (ccp, rcp) = both::<CtxCopyFn>("pcre2_match_context_copy_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_match_context_free_8");
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        cmp_ctx("fresh match_context", a, b, MATCH_CTX_SIZE, MEMCTL_SKIP);

        for name in [
            "pcre2_set_heap_limit_8",
            "pcre2_set_match_limit_8",
            "pcre2_set_depth_limit_8",
            "pcre2_set_recursion_limit_8",
        ] {
            let (sc, sr) = both::<SetU32Fn>(name);
            for v in [0u32, 1, 1000, 0xffff_ffff] {
                assert_eq!(sc(a, v), sr(b, v), "{name}({v})");
                cmp_ctx(&format!("after {name}({v})"), a, b, MATCH_CTX_SIZE, MEMCTL_SKIP);
            }
        }
        let (sc, sr) = both::<SetSizeFn>("pcre2_set_offset_limit_8");
        for v in [0usize, 1, 1000, usize::MAX] {
            assert_eq!(sc(a, v), sr(b, v), "set_offset_limit({v})");
            cmp_ctx(
                &format!("after set_offset_limit({v})"),
                a,
                b,
                MATCH_CTX_SIZE,
                MEMCTL_SKIP,
            );
        }

        let a2 = ccp(a);
        let b2 = rcp(b);
        cmp_ctx("match_context copy", a2, b2, MATCH_CTX_SIZE, MEMCTL_SKIP);

        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
        cf(a2);
        rf(b2);
        cf(a);
        rf(b);
    }
}

#[test]
fn convert_context_setters() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_convert_context_create_8");
    let (ccp, rcp) = both::<CtxCopyFn>("pcre2_convert_context_copy_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_convert_context_free_8");
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        cmp_ctx("fresh convert_context", a, b, CONVERT_CTX_SIZE, MEMCTL_SKIP);

        for name in ["pcre2_set_glob_separator_8", "pcre2_set_glob_escape_8"] {
            let (sc, sr) = both::<SetU32Fn>(name);
            // Valid values are '/', '\\', '.' (separator) and 0, '\\', '~' (escape).
            for v in [
                0u32, b'/' as u32, b'\\' as u32, b'.' as u32, b'~' as u32, b'x' as u32, 0x100,
                0xffff_ffff,
            ] {
                assert_eq!(sc(a, v), sr(b, v), "{name}({v})");
                cmp_ctx(
                    &format!("after {name}({v})"),
                    a,
                    b,
                    CONVERT_CTX_SIZE,
                    MEMCTL_SKIP,
                );
            }
        }

        let a2 = ccp(a);
        let b2 = rcp(b);
        cmp_ctx("convert_context copy", a2, b2, CONVERT_CTX_SIZE, MEMCTL_SKIP);

        cf(std::ptr::null_mut());
        rf(std::ptr::null_mut());
        cf(a2);
        rf(b2);
        cf(a);
        rf(b);
    }
}
