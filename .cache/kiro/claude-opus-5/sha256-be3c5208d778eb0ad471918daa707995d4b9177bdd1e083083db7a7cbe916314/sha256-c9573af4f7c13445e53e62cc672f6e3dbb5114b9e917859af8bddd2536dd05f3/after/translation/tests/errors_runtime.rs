//! Phase C — run-time (non-compile) error-path differential tests.
//!
//! One test per `ERRORS.md` run-time row. Every case drives BOTH shared objects
//! with the identical invalid input and asserts the returned error code (not
//! merely "both failed") is the same.

mod common;
use common::*;
use std::ffi::c_void;

const MAGIC_NUMBER: u32 = 0x5043_5245;
const MODE_MASK: u32 = 0x0000_0007;
const MODE16: u32 = 0x0000_0002;

/// Locate `magic_number` inside a compiled code block and return a mutable
/// pointer to the `flags` field that follows it (magic, compile_options,
/// overall_options, extra_options, flags — see `pcre2_real_code`).
unsafe fn flags_ptr(code: Code) -> *mut u32 {
    let base = code as *mut u8;
    for off in (0..256usize).step_by(4) {
        let p = base.add(off) as *mut u32;
        if unsafe { *p } == MAGIC_NUMBER {
            return unsafe { p.add(4) };
        }
    }
    panic!("magic number not found in compiled code block");
}

unsafe fn magic_ptr(code: Code) -> *mut u32 {
    let base = code as *mut u8;
    for off in (0..256usize).step_by(4) {
        let p = base.add(off) as *mut u32;
        if unsafe { *p } == MAGIC_NUMBER {
            return p;
        }
    }
    panic!("magic number not found in compiled code block");
}

fn compile_ok(p: &'static Pair, pat: &[u8], opts: u32) -> CodePair {
    compile_both(p, pat, pat.len(), opts, std::ptr::null_mut(), std::ptr::null_mut(), "setup")
        .expect("pattern should compile")
}

// ===========================================================================
// pcre2_config
// ===========================================================================

#[test]
fn config_rejects_unknown_requests_identically() {
    let p = libs();
    let mut buf = [0u8; 256];
    for w in [17u32, 18, 100, 1000, u32::MAX, u32::MAX - 1, 0x8000_0000] {
        let a = unsafe { (p.c.config)(w, buf.as_mut_ptr() as *mut c_void) };
        let b = unsafe { (p.r.config)(w, buf.as_mut_ptr() as *mut c_void) };
        assert_eq!(a, b, "config({}) rc", w);
        assert_eq!(a, err::BADOPTION, "config({}) should be BADOPTION", w);
    }
}

#[test]
fn config_with_null_where_returns_size() {
    // C: `where == NULL` means "return the required size".
    let p = libs();
    for w in 0..=16u32 {
        let a = unsafe { (p.c.config)(w, std::ptr::null_mut()) };
        let b = unsafe { (p.r.config)(w, std::ptr::null_mut()) };
        assert_eq!(a, b, "config({}, NULL) rc", w);
    }
}

// ===========================================================================
// Context setters: BADDATA / BADOPTION / NULL
// ===========================================================================

#[test]
fn set_bsr_rejects_out_of_range() {
    let p = libs();
    let cc = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
    for v in [0u32, 1, 2, 3, 4, 255, u32::MAX] {
        let a = unsafe { (p.c.set_bsr)(cc, v) };
        let b = unsafe { (p.r.set_bsr)(cr, v) };
        assert_eq!(a, b, "set_bsr({})", v);
        if v == 1 || v == 2 {
            assert_eq!(a, 0);
        } else {
            assert_eq!(a, err::BADDATA, "set_bsr({}) should be BADDATA", v);
        }
    }
    unsafe {
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(cr);
    }
}

#[test]
fn set_newline_rejects_out_of_range() {
    let p = libs();
    let cc = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
    for v in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 255, u32::MAX] {
        let a = unsafe { (p.c.set_newline)(cc, v) };
        let b = unsafe { (p.r.set_newline)(cr, v) };
        assert_eq!(a, b, "set_newline({})", v);
        if (1..=6).contains(&v) {
            assert_eq!(a, 0);
        } else {
            assert_eq!(a, err::BADDATA, "set_newline({}) should be BADDATA", v);
        }
    }
    unsafe {
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(cr);
    }
}

#[test]
fn set_optimize_rejects_unknown_directives() {
    let p = libs();
    let cc = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
    for v in [0u32, 1, 2, 3, 63, 64, 65, 66, 67, 68, 69, 70, 1000, u32::MAX] {
        let a = unsafe { (p.c.set_optimize)(cc, v) };
        let b = unsafe { (p.r.set_optimize)(cr, v) };
        assert_eq!(a, b, "set_optimize({})", v);
    }
    // NULL context
    let a = unsafe { (p.c.set_optimize)(std::ptr::null_mut(), o::OPTIMIZATION_FULL) };
    let b = unsafe { (p.r.set_optimize)(std::ptr::null_mut(), o::OPTIMIZATION_FULL) };
    assert_eq!(a, b);
    assert_eq!(a, err::NULL);
    unsafe {
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(cr);
    }
}

#[test]
fn set_glob_escape_and_separator_reject_bad_values() {
    let p = libs();
    let cc = unsafe { (p.c.convert_context_create)(std::ptr::null_mut()) };
    let cr = unsafe { (p.r.convert_context_create)(std::ptr::null_mut()) };
    for v in [
        0u32, 1, 0x1f, 0x20, b'\\' as u32, b'/' as u32, b'.' as u32, b'a' as u32, 0x7e, 0x7f, 0x80,
        0xff, 0x100, u32::MAX,
    ] {
        let a = unsafe { (p.c.set_glob_escape)(cc, v) };
        let b = unsafe { (p.r.set_glob_escape)(cr, v) };
        assert_eq!(a, b, "set_glob_escape({:#x})", v);
        let a = unsafe { (p.c.set_glob_separator)(cc, v) };
        let b = unsafe { (p.r.set_glob_separator)(cr, v) };
        assert_eq!(a, b, "set_glob_separator({:#x})", v);
    }
    unsafe {
        (p.c.convert_context_free)(cc);
        (p.r.convert_context_free)(cr);
    }
}

#[test]
fn all_scalar_setters_accept_extremes_identically() {
    let p = libs();
    let cc = unsafe { (p.c.compile_context_create)(std::ptr::null_mut()) };
    let cr = unsafe { (p.r.compile_context_create)(std::ptr::null_mut()) };
    let mc = unsafe { (p.c.match_context_create)(std::ptr::null_mut()) };
    let mr = unsafe { (p.r.match_context_create)(std::ptr::null_mut()) };
    for v in [0u32, 1, 0xffff, 0x1_0000, u32::MAX] {
        unsafe {
            assert_eq!((p.c.set_max_varlookbehind)(cc, v), (p.r.set_max_varlookbehind)(cr, v));
            assert_eq!((p.c.set_parens_nest_limit)(cc, v), (p.r.set_parens_nest_limit)(cr, v));
            assert_eq!((p.c.set_compile_extra_options)(cc, v), (p.r.set_compile_extra_options)(cr, v));
            assert_eq!((p.c.set_depth_limit)(mc, v), (p.r.set_depth_limit)(mr, v));
            assert_eq!((p.c.set_heap_limit)(mc, v), (p.r.set_heap_limit)(mr, v));
            assert_eq!((p.c.set_match_limit)(mc, v), (p.r.set_match_limit)(mr, v));
            assert_eq!((p.c.set_recursion_limit)(mc, v), (p.r.set_recursion_limit)(mr, v));
        }
    }
    for v in [0usize, 1, usize::MAX, usize::MAX - 1] {
        unsafe {
            assert_eq!((p.c.set_max_pattern_length)(cc, v), (p.r.set_max_pattern_length)(cr, v));
            assert_eq!(
                (p.c.set_max_pattern_compiled_length)(cc, v),
                (p.r.set_max_pattern_compiled_length)(cr, v)
            );
            assert_eq!((p.c.set_offset_limit)(mc, v), (p.r.set_offset_limit)(mr, v));
        }
    }
    unsafe {
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(cr);
        (p.c.match_context_free)(mc);
        (p.r.match_context_free)(mr);
    }
}

#[test]
fn context_free_and_copy_with_null_are_safe() {
    let p = libs();
    unsafe {
        (p.c.general_context_free)(std::ptr::null_mut());
        (p.r.general_context_free)(std::ptr::null_mut());
        (p.c.compile_context_free)(std::ptr::null_mut());
        (p.r.compile_context_free)(std::ptr::null_mut());
        (p.c.match_context_free)(std::ptr::null_mut());
        (p.r.match_context_free)(std::ptr::null_mut());
        (p.c.convert_context_free)(std::ptr::null_mut());
        (p.r.convert_context_free)(std::ptr::null_mut());
        (p.c.code_free)(std::ptr::null_mut());
        (p.r.code_free)(std::ptr::null_mut());
        (p.c.match_data_free)(std::ptr::null_mut());
        (p.r.match_data_free)(std::ptr::null_mut());
        (p.c.substring_free)(std::ptr::null_mut());
        (p.r.substring_free)(std::ptr::null_mut());
        (p.c.substring_list_free)(std::ptr::null_mut());
        (p.r.substring_list_free)(std::ptr::null_mut());
        (p.c.serialize_free)(std::ptr::null_mut());
        (p.r.serialize_free)(std::ptr::null_mut());
        (p.c.converted_pattern_free)(std::ptr::null_mut());
        (p.r.converted_pattern_free)(std::ptr::null_mut());
        assert_eq!(
            (p.c.code_copy)(std::ptr::null_mut()).is_null(),
            (p.r.code_copy)(std::ptr::null_mut()).is_null()
        );
        assert_eq!(
            (p.c.code_copy_with_tables)(std::ptr::null_mut()).is_null(),
            (p.r.code_copy_with_tables)(std::ptr::null_mut()).is_null()
        );
        // NOTE: pcre2_{general,compile,match,convert}_context_copy() do NOT
        // NULL-check their argument in the C source (they dereference
        // `ctx->memctl.malloc` immediately), so passing NULL is undefined
        // behaviour there and is not part of the error surface. The Rust
        // translation reproduces the same unchecked dereference; we deliberately
        // do not call them with NULL.
    }
}

// ===========================================================================
// pcre2_pattern_info
// ===========================================================================

#[test]
fn pattern_info_error_paths() {
    let p = libs();
    let cp = compile_ok(libs(), b"a(b)c", 0);
    let mut out = [0u8; 64];
    // NULL code
    let a = unsafe { (p.c.pattern_info)(std::ptr::null_mut(), info::CAPTURECOUNT, out.as_mut_ptr() as *mut c_void) };
    let b = unsafe { (p.r.pattern_info)(std::ptr::null_mut(), info::CAPTURECOUNT, out.as_mut_ptr() as *mut c_void) };
    assert_eq!(a, b);
    assert_eq!(a, err::NULL);
    // NULL where -> returns the size of the item
    for w in 0..=26u32 {
        let a = unsafe { (p.c.pattern_info)(cp.c, w, std::ptr::null_mut()) };
        let b = unsafe { (p.r.pattern_info)(cp.r, w, std::ptr::null_mut()) };
        assert_eq!(a, b, "pattern_info({}, NULL)", w);
    }
    // out-of-range request codes
    for w in [27u32, 28, 100, 1000, u32::MAX] {
        let a = unsafe { (p.c.pattern_info)(cp.c, w, out.as_mut_ptr() as *mut c_void) };
        let b = unsafe { (p.r.pattern_info)(cp.r, w, out.as_mut_ptr() as *mut c_void) };
        assert_eq!(a, b, "pattern_info({})", w);
        assert_eq!(a, err::BADOPTION);
    }
    // UNSET for limits that the pattern does not set
    for w in [info::MATCHLIMIT, info::DEPTHLIMIT, info::HEAPLIMIT] {
        let a = unsafe { (p.c.pattern_info)(cp.c, w, out.as_mut_ptr() as *mut c_void) };
        let b = unsafe { (p.r.pattern_info)(cp.r, w, out.as_mut_ptr() as *mut c_void) };
        assert_eq!(a, b, "pattern_info({})", w);
        assert_eq!(a, err::UNSET);
    }
    free_code_pair(p, cp);
}

#[test]
fn pattern_info_badmagic_and_badmode() {
    let p = libs();
    // BADMAGIC: a zeroed block masquerading as a pcre2_code.
    let zeroed_c = vec![0u8; 4096];
    let zeroed_r = vec![0u8; 4096];
    let mut v: u32 = 0;
    let a = unsafe {
        (p.c.pattern_info)(zeroed_c.as_ptr() as Code, info::CAPTURECOUNT, &mut v as *mut _ as *mut c_void)
    };
    let b = unsafe {
        (p.r.pattern_info)(zeroed_r.as_ptr() as Code, info::CAPTURECOUNT, &mut v as *mut _ as *mut c_void)
    };
    assert_eq!(a, b);
    assert_eq!(a, err::BADMAGIC);

    // BADMODE: correct magic but the wrong code-unit-width flag.
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        let fc = flags_ptr(cp.c);
        let fr = flags_ptr(cp.r);
        let oc = *fc;
        let or = *fr;
        assert_eq!(oc & MODE_MASK, or & MODE_MASK, "mode bits differ between libs");
        *fc = (oc & !MODE_MASK) | MODE16;
        *fr = (or & !MODE_MASK) | MODE16;
        let a = (p.c.pattern_info)(cp.c, info::CAPTURECOUNT, &mut v as *mut _ as *mut c_void);
        let b = (p.r.pattern_info)(cp.r, info::CAPTURECOUNT, &mut v as *mut _ as *mut c_void);
        assert_eq!(a, b);
        assert_eq!(a, err::BADMODE);
        // Also through pcre2_match / pcre2_dfa_match.
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let subj = b"abc";
        let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b);
        assert_eq!(a, err::BADMODE);
        let mut ws = [0i32; 64];
        let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
        let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
        assert_eq!(a, b);
        assert_eq!(a, err::BADMODE);
        // Restore, then free.
        *fc = oc;
        *fr = or;
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn match_badmagic() {
    let p = libs();
    let zc = vec![0u8; 4096];
    let zr = vec![0u8; 4096];
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let subj = b"abc";
        let a = (p.c.pcre2_match)(zc.as_ptr() as Code, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(zr.as_ptr() as Code, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b);
        assert_eq!(a, err::BADMAGIC);
        let mut ws = [0i32; 64];
        let a = (p.c.dfa_match)(zc.as_ptr() as Code, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
        let b = (p.r.dfa_match)(zr.as_ptr() as Code, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
        assert_eq!(a, b);
        assert_eq!(a, err::BADMAGIC);
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
}

// ===========================================================================
// pcre2_match / pcre2_dfa_match error paths
// ===========================================================================

#[test]
fn match_null_arguments() {
    let p = libs();
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let subj = b"abc";
        // NULL code
        let a = (p.c.pcre2_match)(std::ptr::null_mut(), subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(std::ptr::null_mut(), subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!((a, b), (err::NULL, err::NULL));
        // NULL match data
        let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!((a, b), (err::NULL, err::NULL));
        // NULL subject with zero length is legal; with non-zero length it is NULL error
        let a = (p.c.pcre2_match)(cp.c, std::ptr::null(), 0, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, std::ptr::null(), 0, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b, "NULL subject len 0");
        let a = (p.c.pcre2_match)(cp.c, std::ptr::null(), 3, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, std::ptr::null(), 3, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b, "NULL subject len 3");
        // dfa_match NULL workspace
        let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut(), std::ptr::null_mut(), 64);
        let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut(), std::ptr::null_mut(), 64);
        assert_eq!(a, b, "dfa NULL workspace");
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn match_bad_start_offset() {
    let p = libs();
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let subj = b"abcdef";
        let mut ws = [0i32; 64];
        for start in [0usize, 3, 6, 7, 100, usize::MAX] {
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 6, start, 0, mdc, std::ptr::null_mut());
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 6, start, 0, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "match start={}", start);
            if start > 6 {
                assert_eq!(a, err::BADOFFSET, "match start={} should be BADOFFSET", start);
            }
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 6, start, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 6, start, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            assert_eq!(a, b, "dfa start={}", start);
            if start > 6 {
                assert_eq!(a, err::BADOFFSET, "dfa start={} should be BADOFFSET", start);
            }
        }
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn match_bad_offset_limit() {
    let p = libs();
    // PCRE2_ERROR_BADOFFSETLIMIT: offset limit set without PCRE2_USE_OFFSET_LIMIT.
    for opts in [0u32, o::USE_OFFSET_LIMIT] {
        let p2 = libs();
        let cp = compile_ok(p2, b"abc", opts);
        unsafe {
            let mc = (p.c.match_context_create)(std::ptr::null_mut());
            let mr = (p.r.match_context_create)(std::ptr::null_mut());
            for lim in [0usize, 1, 3, 6, 100, usize::MAX] {
                assert_eq!((p.c.set_offset_limit)(mc, lim), (p.r.set_offset_limit)(mr, lim));
                let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
                let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
                let subj = b"xxabcxx";
                let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 7, 0, 0, mdc, mc);
                let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 7, 0, 0, mdr, mr);
                assert_eq!(a, b, "offset_limit={} opts={:#x}", lim, opts);
                let mut ws = [0i32; 64];
                let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 7, 0, 0, mdc, mc, ws.as_mut_ptr(), 64);
                let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 7, 0, 0, mdr, mr, ws.as_mut_ptr(), 64);
                assert_eq!(a, b, "dfa offset_limit={} opts={:#x}", lim, opts);
                (p.c.match_data_free)(mdc);
                (p.r.match_data_free)(mdr);
            }
            (p.c.match_context_free)(mc);
            (p.r.match_context_free)(mr);
        }
        free_code_pair(p, cp);
    }
}

#[test]
fn match_rejects_unknown_option_bits() {
    let p = libs();
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let subj = b"abc";
        let mut ws = [0i32; 64];
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 3, 0, opt, mdc, std::ptr::null_mut());
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 3, 0, opt, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "match optbit {}", bit);
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 3, 0, opt, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 3, 0, opt, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            assert_eq!(a, b, "dfa optbit {}", bit);
            let a = (p.c.jit_match)(cp.c, subj.as_ptr(), 3, 0, opt, mdc, std::ptr::null_mut());
            let b = (p.r.jit_match)(cp.r, subj.as_ptr(), 3, 0, opt, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "jit_match optbit {}", bit);
        }
        // Both PARTIAL_SOFT and PARTIAL_HARD, and everything at once.
        for opt in [o::PARTIAL_SOFT | o::PARTIAL_HARD, u32::MAX, 0xFFFF_0000] {
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 3, 0, opt, mdc, std::ptr::null_mut());
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 3, 0, opt, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "match opts {:#x}", opt);
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 3, 0, opt, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 3, 0, opt, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 64);
            assert_eq!(a, b, "dfa opts {:#x}", opt);
        }
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn dfa_match_workspace_size_errors() {
    let p = libs();
    let cp = compile_ok(libs(), b"a(b|c)+d", 0);
    unsafe {
        let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
        let subj = b"abcbcbd";
        let mut ws = [0i32; 4096];
        for n in [0usize, 1, 9, 19, 20, 21, 100, 4096] {
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 7, 0, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), n);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 7, 0, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), n);
            assert_eq!(a, b, "dfa wscount={}", n);
            if n < 20 {
                assert_eq!(a, err::DFA_WSSIZE, "dfa wscount={} should be DFA_WSSIZE", n);
            }
        }
        // PCRE2_DFA_RESTART with a workspace that was never used -> DFA_BADRESTART
        let mut ws2 = [0i32; 64];
        let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 7, 0, o::DFA_RESTART, mdc, std::ptr::null_mut(), ws2.as_mut_ptr(), 64);
        let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 7, 0, o::DFA_RESTART, mdr, std::ptr::null_mut(), ws2.as_mut_ptr(), 64);
        assert_eq!(a, b);
        assert_eq!(a, err::DFA_BADRESTART);
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn dfa_match_unsupported_items() {
    // DFA matching does not support backreferences (DFA_UITEM) or \C etc.
    let p = libs();
    for pat in [
        &b"(a)\\1"[..],
        &b"(?<n>a)\\k<n>"[..],
        &b"(?(1)a)(b)"[..],
        &b"(?=a)(?<=a)b"[..],
        &b"a(?C1)b"[..],
        &b"(?(?=a)b|c)"[..],
        &b"(*ACCEPT)"[..],
        &b"a\\Kb"[..],
    ] {
        let cp = compile_ok(libs(), pat, 0);
        unsafe {
            let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
            let subj = b"aab";
            let mut ws = [0i32; 256];
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
            assert_eq!(a, b, "dfa on {:?}", String::from_utf8_lossy(pat));
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
        free_code_pair(p, cp);
    }
}

#[test]
fn match_limits_produce_same_error() {
    let p = libs();
    // A pathological pattern that blows the match / depth / heap limits.
    let cp = compile_ok(libs(), b"(a+)+b", o::NO_START_OPTIMIZE);
    let subj = vec![b'a'; 40];
    unsafe {
        let mc = (p.c.match_context_create)(std::ptr::null_mut());
        let mr = (p.r.match_context_create)(std::ptr::null_mut());
        for lim in [0u32, 1, 10, 100, 1000, 100000] {
            assert_eq!((p.c.set_match_limit)(mc, lim), (p.r.set_match_limit)(mr, lim));
            let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), subj.len(), 0, 0, mdc, mc);
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), subj.len(), 0, 0, mdr, mr);
            assert_eq!(a, b, "match_limit={}", lim);
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
        assert_eq!((p.c.set_match_limit)(mc, 10_000_000), (p.r.set_match_limit)(mr, 10_000_000));
        for lim in [0u32, 1, 5, 50, 1000] {
            assert_eq!((p.c.set_depth_limit)(mc, lim), (p.r.set_depth_limit)(mr, lim));
            let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), subj.len(), 0, 0, mdc, mc);
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), subj.len(), 0, 0, mdr, mr);
            assert_eq!(a, b, "depth_limit={}", lim);
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
        assert_eq!((p.c.set_depth_limit)(mc, 10_000_000), (p.r.set_depth_limit)(mr, 10_000_000));
        for lim in [0u32, 1, 2, 16, 20_000_000] {
            assert_eq!((p.c.set_heap_limit)(mc, lim), (p.r.set_heap_limit)(mr, lim));
            let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), subj.len(), 0, 0, mdc, mc);
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), subj.len(), 0, 0, mdr, mr);
            assert_eq!(a, b, "heap_limit={}", lim);
            let mut ws = [0i32; 256];
            let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), subj.len(), 0, 0, mdc, mc, ws.as_mut_ptr(), 256);
            let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), subj.len(), 0, 0, mdr, mr, ws.as_mut_ptr(), 256);
            assert_eq!(a, b, "dfa heap_limit={}", lim);
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
        (p.c.match_context_free)(mc);
        (p.r.match_context_free)(mr);
    }
    free_code_pair(p, cp);
}

#[test]
fn match_recurse_loop_detection() {
    let p = libs();
    // Recursion that can loop: PCRE2_ERROR_RECURSELOOP unless disabled.
    for pat in [&b"(a*)*(?1)"[..], &b"(?:(?1)|a)*"[..], &b"(a|(?R))*"[..]] {
        if let Ok(cp) = compile_both(p, pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), "recurse")
        {
            unsafe {
                let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
                let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
                let subj = b"aaaa";
                for opt in [0u32, o::DISABLE_RECURSELOOP_CHECK] {
                    let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 4, 0, opt, mdc, std::ptr::null_mut());
                    let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 4, 0, opt, mdr, std::ptr::null_mut());
                    assert_eq!(a, b, "recurse {:?} opt {:#x}", String::from_utf8_lossy(pat), opt);
                }
                (p.c.match_data_free)(mdc);
                (p.r.match_data_free)(mdr);
            }
            free_code_pair(p, cp);
        }
    }
}

#[test]
fn match_bad_backslash_k() {
    let p = libs();
    // \K in an assertion moving start before match start => BAD_BACKSLASH_K
    for pat in [&b"(?<=a\\Kb)c"[..], &b"a\\Kb"[..], &b"(?:\\Ka)+"[..]] {
        if let Ok(cp) = compile_both(
            p,
            pat,
            pat.len(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            "bsk",
        ) {
            unsafe {
                let mdc = (p.c.match_data_create)(8, std::ptr::null_mut());
                let mdr = (p.r.match_data_create)(8, std::ptr::null_mut());
                let subj = b"abcabc";
                let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 6, 0, 0, mdc, std::ptr::null_mut());
                let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 6, 0, 0, mdr, std::ptr::null_mut());
                assert_eq!(a, b, "bsk {:?}", String::from_utf8_lossy(pat));
                (p.c.match_data_free)(mdc);
                (p.r.match_data_free)(mdr);
            }
            free_code_pair(p, cp);
        }
    }
}

// ===========================================================================
// UTF validity: PCRE2_ERROR_UTF8_ERR1..21 and BADUTFOFFSET
// ===========================================================================

/// Malformed UTF-8 subjects covering every distinct `PCRE2_ERROR_UTF8_ERRn`.
static BAD_UTF8: &[(&str, &[u8])] = &[
    ("1-byte-missing", &[0xC2]),
    ("2-bytes-missing", &[0xE0, 0xA0]),
    ("3-bytes-missing", &[0xF0, 0x90, 0x80]),
    ("4-bytes-missing", &[0xF8, 0x88, 0x80, 0x80]),
    ("5-bytes-missing", &[0xFC, 0x84, 0x80, 0x80, 0x80]),
    ("bad-2nd-byte", &[0xC2, 0x41]),
    ("bad-3rd-byte", &[0xE0, 0xA0, 0x41]),
    ("bad-4th-byte", &[0xF0, 0x90, 0x80, 0x41]),
    ("bad-5th-byte", &[0xF8, 0x88, 0x80, 0x80, 0x41]),
    ("bad-6th-byte", &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x41]),
    ("5-byte-char", &[0xF8, 0x88, 0x80, 0x80, 0x80]),
    ("6-byte-char", &[0xFC, 0x84, 0x80, 0x80, 0x80, 0x80]),
    ("fe", &[0xFE]),
    ("ff", &[0xFF]),
    ("overlong-2", &[0xC0, 0x80]),
    ("overlong-3", &[0xE0, 0x80, 0x80]),
    ("overlong-4", &[0xF0, 0x80, 0x80, 0x80]),
    ("overlong-5", &[0xF8, 0x80, 0x80, 0x80, 0x80]),
    ("overlong-6", &[0xFC, 0x80, 0x80, 0x80, 0x80, 0x80]),
    ("surrogate", &[0xED, 0xA0, 0x80]),
    ("gt-10ffff", &[0xF4, 0x90, 0x80, 0x80]),
    ("isolated-cont", &[0x80]),
    ("isolated-cont-bf", &[0xBF]),
    ("valid-mixed-bad", &[b'a', 0xC2, b'b', b'c']),
    ("truncated-at-end", &[b'a', b'b', 0xE2, 0x82]),
];

#[test]
fn priv_valid_utf_agrees_on_every_malformation() {
    let p = libs();
    for (name, bytes) in BAD_UTF8 {
        let mut oc: Sz = 0xDEAD;
        let mut or: Sz = 0xBEEF;
        let a = unsafe { (p.c.priv_valid_utf)(bytes.as_ptr(), bytes.len(), &mut oc) };
        let b = unsafe { (p.r.priv_valid_utf)(bytes.as_ptr(), bytes.len(), &mut or) };
        assert_eq!(a, b, "_pcre2_valid_utf rc for {}", name);
        assert_eq!(oc, or, "_pcre2_valid_utf erroroffset for {}", name);
        assert!(a < 0, "{} should be invalid UTF-8 (got {})", name, a);
    }
    // Valid sequences must return 0 with the offset untouched.
    for good in [
        &b"abc"[..],
        &[0xC2, 0xA9][..],
        &[0xE2, 0x82, 0xAC][..],
        &[0xF0, 0x9F, 0x98, 0x80][..],
        &[0xF4, 0x8F, 0xBF, 0xBF][..],
        &[][..],
    ] {
        let mut oc: Sz = 0xDEAD;
        let mut or: Sz = 0xDEAD;
        let a = unsafe { (p.c.priv_valid_utf)(good.as_ptr(), good.len(), &mut oc) };
        let b = unsafe { (p.r.priv_valid_utf)(good.as_ptr(), good.len(), &mut or) };
        assert_eq!((a, oc), (b, or), "valid utf {:02x?}", good);
        assert_eq!(a, 0);
    }
}

#[test]
fn match_utf_subject_errors() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", o::UTF);
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let mut ws = [0i32; 256];
        for (name, bytes) in BAD_UTF8 {
            // NOTE: PCRE2_NO_UTF_CHECK on an invalid-UTF subject is documented
            // undefined behaviour (the C library then indexes its Unicode tables
            // out of range), so it is deliberately excluded here.
            for opt in [0u32, o::PARTIAL_HARD, o::PARTIAL_SOFT] {
                let a = (p.c.pcre2_match)(cp.c, bytes.as_ptr(), bytes.len(), 0, opt, mdc, std::ptr::null_mut());
                let b = (p.r.pcre2_match)(cp.r, bytes.as_ptr(), bytes.len(), 0, opt, mdr, std::ptr::null_mut());
                assert_eq!(a, b, "match utf {} opt {:#x}", name, opt);
                if opt == 0 {
                    let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), 2);
                    let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), 2);
                    assert_eq!(oc, or, "match utf {} ovector", name);
                }
                let a = (p.c.dfa_match)(cp.c, bytes.as_ptr(), bytes.len(), 0, opt, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
                let b = (p.r.dfa_match)(cp.r, bytes.as_ptr(), bytes.len(), 0, opt, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
                assert_eq!(a, b, "dfa utf {} opt {:#x}", name, opt);
            }
        }
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
    // MATCH_INVALID_UTF changes the handling entirely.
    let cp = compile_ok(libs(), b"a", o::UTF | o::MATCH_INVALID_UTF);
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        for (name, bytes) in BAD_UTF8 {
            let a = (p.c.pcre2_match)(cp.c, bytes.as_ptr(), bytes.len(), 0, 0, mdc, std::ptr::null_mut());
            let b = (p.r.pcre2_match)(cp.r, bytes.as_ptr(), bytes.len(), 0, 0, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "invalid-utf match {}", name);
            let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), 2);
            let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), 2);
            assert_eq!(oc, or, "invalid-utf ovector {}", name);
        }
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn match_badutfoffset() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", o::UTF);
    let subj: &[u8] = &[0xE2, 0x82, 0xAC, b'a']; // euro sign + 'a'
    unsafe {
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let mut ws = [0i32; 256];
        // NOTE: a start offset that is not on a character boundary combined with
        // PCRE2_NO_UTF_CHECK is documented undefined behaviour, so only the
        // checking form is exercised for the interior offsets.
        for start in [0usize, 1, 2, 3, 4] {
            for opt in if start == 0 || start == 3 || start == 4 {
                &[0u32, o::NO_UTF_CHECK][..]
            } else {
                &[0u32][..]
            } {
                let opt = *opt;
                let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 4, start, opt, mdc, std::ptr::null_mut());
                let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 4, start, opt, mdr, std::ptr::null_mut());
                assert_eq!(a, b, "badutfoffset start={} opt={:#x}", start, opt);
                let a = (p.c.dfa_match)(cp.c, subj.as_ptr(), 4, start, opt, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
                let b = (p.r.dfa_match)(cp.r, subj.as_ptr(), 4, start, opt, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
                assert_eq!(a, b, "dfa badutfoffset start={} opt={:#x}", start, opt);
            }
        }
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

// ===========================================================================
// pcre2_substring_*
// ===========================================================================

#[test]
fn substring_error_paths() {
    let p = libs();
    let cp = compile_ok(libs(), b"(a)(b)?(?<nm>c)", 0);
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
        let subj = b"ac";
        let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 2, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 2, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b);
        assert!(a > 0, "setup match must succeed, got {}", a);

        // length_bynumber: valid, unset, out of range
        for n in [0u32, 1, 2, 3, 4, 100, u32::MAX] {
            let mut lc: Sz = 0xAAAA;
            let mut lr: Sz = 0x5555;
            let a = (p.c.substring_length_bynumber)(mdc, n, &mut lc);
            let b = (p.r.substring_length_bynumber)(mdr, n, &mut lr);
            assert_eq!(a, b, "substring_length_bynumber({})", n);
            if a == 0 {
                assert_eq!(lc, lr, "length value for {}", n);
            }
            // NULL out pointer is allowed (just validates)
            let a = (p.c.substring_length_bynumber)(mdc, n, std::ptr::null_mut());
            let b = (p.r.substring_length_bynumber)(mdr, n, std::ptr::null_mut());
            assert_eq!(a, b, "substring_length_bynumber({}, NULL)", n);
        }

        // copy_bynumber with too-small buffers
        for n in [0u32, 1, 2, 3, 100] {
            for cap in [0usize, 1, 2, 3, 16] {
                let mut bc = vec![0u8; 32];
                let mut br = vec![0u8; 32];
                let mut sc: Sz = cap;
                let mut sr: Sz = cap;
                let a = (p.c.substring_copy_bynumber)(mdc, n, bc.as_mut_ptr(), &mut sc);
                let b = (p.r.substring_copy_bynumber)(mdr, n, br.as_mut_ptr(), &mut sr);
                assert_eq!(a, b, "copy_bynumber({}, cap {})", n, cap);
                assert_eq!(sc, sr, "copy_bynumber({}, cap {}) size out", n, cap);
                if a == 0 {
                    assert_eq!(bc, br, "copy_bynumber({}, cap {}) bytes", n, cap);
                }
            }
        }

        // get_bynumber
        for n in [0u32, 1, 2, 3, 100] {
            let mut pc: *mut u8 = std::ptr::null_mut();
            let mut pr: *mut u8 = std::ptr::null_mut();
            let mut lc: Sz = 0;
            let mut lr: Sz = 0;
            let a = (p.c.substring_get_bynumber)(mdc, n, &mut pc, &mut lc);
            let b = (p.r.substring_get_bynumber)(mdr, n, &mut pr, &mut lr);
            assert_eq!(a, b, "get_bynumber({})", n);
            assert_eq!(lc, lr, "get_bynumber({}) len", n);
            if a == 0 {
                let sc = std::slice::from_raw_parts(pc, lc);
                let sr = std::slice::from_raw_parts(pr, lr);
                assert_eq!(sc, sr, "get_bynumber({}) bytes", n);
                (p.c.substring_free)(pc);
                (p.r.substring_free)(pr);
            }
        }

        // by-name variants: existing, non-existent, empty
        for name in [&b"nm\0"[..], &b"nope\0"[..], &b"\0"[..], &b"NM\0"[..]] {
            let mut lc: Sz = 0;
            let mut lr: Sz = 0;
            let a = (p.c.substring_length_byname)(mdc, name.as_ptr(), &mut lc);
            let b = (p.r.substring_length_byname)(mdr, name.as_ptr(), &mut lr);
            assert_eq!(a, b, "length_byname({:?})", String::from_utf8_lossy(name));
            if a == 0 {
                assert_eq!(lc, lr);
            }
            let mut bc = vec![0u8; 32];
            let mut br = vec![0u8; 32];
            let mut sc: Sz = 32;
            let mut sr: Sz = 32;
            let a = (p.c.substring_copy_byname)(mdc, name.as_ptr(), bc.as_mut_ptr(), &mut sc);
            let b = (p.r.substring_copy_byname)(mdr, name.as_ptr(), br.as_mut_ptr(), &mut sr);
            assert_eq!(a, b, "copy_byname({:?})", String::from_utf8_lossy(name));
            assert_eq!((sc, &bc), (sr, &br));
            let mut pc: *mut u8 = std::ptr::null_mut();
            let mut pr: *mut u8 = std::ptr::null_mut();
            let mut lc2: Sz = 0;
            let mut lr2: Sz = 0;
            let a = (p.c.substring_get_byname)(mdc, name.as_ptr(), &mut pc, &mut lc2);
            let b = (p.r.substring_get_byname)(mdr, name.as_ptr(), &mut pr, &mut lr2);
            assert_eq!(a, b, "get_byname({:?})", String::from_utf8_lossy(name));
            assert_eq!(lc2, lr2);
            if a == 0 {
                assert_eq!(
                    std::slice::from_raw_parts(pc, lc2),
                    std::slice::from_raw_parts(pr, lr2)
                );
                (p.c.substring_free)(pc);
                (p.r.substring_free)(pr);
            }
        }

        // number_from_name / nametable_scan
        for name in [&b"nm\0"[..], &b"nope\0"[..], &b"\0"[..]] {
            let a = (p.c.substring_number_from_name)(cp.c, name.as_ptr());
            let b = (p.r.substring_number_from_name)(cp.r, name.as_ptr());
            assert_eq!(a, b, "number_from_name({:?})", String::from_utf8_lossy(name));
            let mut f1: *const u8 = std::ptr::null();
            let mut l1: *const u8 = std::ptr::null();
            let mut f2: *const u8 = std::ptr::null();
            let mut l2: *const u8 = std::ptr::null();
            let a = (p.c.substring_nametable_scan)(cp.c, name.as_ptr(), &mut f1, &mut l1);
            let b = (p.r.substring_nametable_scan)(cp.r, name.as_ptr(), &mut f2, &mut l2);
            assert_eq!(a, b, "nametable_scan({:?})", String::from_utf8_lossy(name));
        }
        // Also exercise the documented `firstptr == NULL` form, which returns
        // either the group number or NOUNIQUESUBSTRING.
        for name in [&b"nm\0"[..], &b"nope\0"[..]] {
            let mut l1: *const u8 = std::ptr::null();
            let mut l2: *const u8 = std::ptr::null();
            let a = (p.c.substring_nametable_scan)(cp.c, name.as_ptr(), std::ptr::null_mut(), &mut l1);
            let b = (p.r.substring_nametable_scan)(cp.r, name.as_ptr(), std::ptr::null_mut(), &mut l2);
            assert_eq!(a, b, "nametable_scan({:?}, NULL first)", String::from_utf8_lossy(name));
        }

        // substring_list_get
        let mut lc: *mut *mut u8 = std::ptr::null_mut();
        let mut lr: *mut *mut u8 = std::ptr::null_mut();
        let mut oc: *mut Sz = std::ptr::null_mut();
        let mut or: *mut Sz = std::ptr::null_mut();
        let a = (p.c.substring_list_get)(mdc, &mut lc, &mut oc);
        let b = (p.r.substring_list_get)(mdr, &mut lr, &mut or);
        assert_eq!(a, b, "substring_list_get");
        if a == 0 {
            let mut i = 0;
            loop {
                let ec = *lc.add(i);
                let er = *lr.add(i);
                assert_eq!(ec.is_null(), er.is_null(), "list entry {} null-ness", i);
                if ec.is_null() {
                    break;
                }
                let nc = *oc.add(i);
                let nr = *or.add(i);
                assert_eq!(nc, nr, "list entry {} length", i);
                assert_eq!(
                    std::slice::from_raw_parts(ec, nc),
                    std::slice::from_raw_parts(er, nr),
                    "list entry {} bytes",
                    i
                );
                i += 1;
            }
            (p.c.substring_list_free)(lc);
            (p.r.substring_list_free)(lr);
        }
        // and with NULL lengths pointer
        let mut lc: *mut *mut u8 = std::ptr::null_mut();
        let mut lr: *mut *mut u8 = std::ptr::null_mut();
        let a = (p.c.substring_list_get)(mdc, &mut lc, std::ptr::null_mut());
        let b = (p.r.substring_list_get)(mdr, &mut lr, std::ptr::null_mut());
        assert_eq!(a, b, "substring_list_get(NULL lens)");
        if a == 0 {
            (p.c.substring_list_free)(lc);
            (p.r.substring_list_free)(lr);
        }

        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn substring_on_failed_and_partial_match() {
    let p = libs();
    let cp = compile_ok(libs(), b"(abc)(def)", 0);
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
        // NOTE: a *fresh* match_data has uninitialised `rc`/`code` fields in the C
        // library (pcre2_match_data_create only sets oveccount/flags/heapframes),
        // so calling the substring accessors before any match is undefined
        // behaviour there. We therefore start from a real (partial) match.
        // Partial match then substring access
        let subj = b"abcde";
        let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 5, 0, o::PARTIAL_HARD, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 5, 0, o::PARTIAL_HARD, mdr, std::ptr::null_mut());
        assert_eq!(a, b);
        assert_eq!(a, err::PARTIAL);
        for n in [0u32, 1, 2] {
            let mut lc: Sz = 0;
            let mut lr: Sz = 0;
            let a = (p.c.substring_length_bynumber)(mdc, n, &mut lc);
            let b = (p.r.substring_length_bynumber)(mdr, n, &mut lr);
            assert_eq!(a, b, "partial length_bynumber({})", n);
            let mut pc: *mut u8 = std::ptr::null_mut();
            let mut pr: *mut u8 = std::ptr::null_mut();
            let mut l1: Sz = 0;
            let mut l2: Sz = 0;
            let a = (p.c.substring_get_bynumber)(mdc, n, &mut pc, &mut l1);
            let b = (p.r.substring_get_bynumber)(mdr, n, &mut pr, &mut l2);
            assert_eq!(a, b, "partial get_bynumber({})", n);
            if a == 0 {
                (p.c.substring_free)(pc);
                (p.r.substring_free)(pr);
            }
        }
        // get_mark / get_startchar on a partial match
        assert_eq!(
            (p.c.get_startchar)(mdc),
            (p.r.get_startchar)(mdr),
            "get_startchar after partial"
        );
        let mc = (p.c.get_mark)(mdc);
        let mr = (p.r.get_mark)(mdr);
        assert_eq!(mc.is_null(), mr.is_null(), "get_mark null-ness");
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

#[test]
fn substring_duplicate_names() {
    let p = libs();
    let cp = compile_ok(libs(), b"(?<d>a)|(?<d>b)", o::DUPNAMES);
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
        let subj = b"b";
        assert_eq!(
            (p.c.pcre2_match)(cp.c, subj.as_ptr(), 1, 0, 0, mdc, std::ptr::null_mut()),
            (p.r.pcre2_match)(cp.r, subj.as_ptr(), 1, 0, 0, mdr, std::ptr::null_mut())
        );
        // number_from_name on a duplicated name -> NOUNIQUESUBSTRING
        let a = (p.c.substring_number_from_name)(cp.c, b"d\0".as_ptr());
        let b = (p.r.substring_number_from_name)(cp.r, b"d\0".as_ptr());
        assert_eq!(a, b);
        assert_eq!(a, err::NOUNIQUESUBSTRING);
        // but the by-name accessors resolve duplicates
        let mut lc: Sz = 0;
        let mut lr: Sz = 0;
        let a = (p.c.substring_length_byname)(mdc, b"d\0".as_ptr(), &mut lc);
        let b = (p.r.substring_length_byname)(mdr, b"d\0".as_ptr(), &mut lr);
        assert_eq!((a, lc), (b, lr));
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

// ===========================================================================
// pcre2_serialize_*
// ===========================================================================

#[test]
fn serialize_error_paths() {
    let p = libs();
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        let mut bufc: *mut u8 = std::ptr::null_mut();
        let mut bufr: *mut u8 = std::ptr::null_mut();
        let mut lc: Sz = 0;
        let mut lr: Sz = 0;
        let codes_c = [cp.c];
        let codes_r = [cp.r];

        // NULL arguments
        assert_eq!(
            (p.c.serialize_encode)(std::ptr::null(), 1, &mut bufc, &mut lc, std::ptr::null_mut()),
            (p.r.serialize_encode)(std::ptr::null(), 1, &mut bufr, &mut lr, std::ptr::null_mut())
        );
        assert_eq!(
            (p.c.serialize_encode)(codes_c.as_ptr(), 1, std::ptr::null_mut(), &mut lc, std::ptr::null_mut()),
            (p.r.serialize_encode)(codes_r.as_ptr(), 1, std::ptr::null_mut(), &mut lr, std::ptr::null_mut())
        );
        assert_eq!(
            (p.c.serialize_encode)(codes_c.as_ptr(), 1, &mut bufc, std::ptr::null_mut(), std::ptr::null_mut()),
            (p.r.serialize_encode)(codes_r.as_ptr(), 1, &mut bufr, std::ptr::null_mut(), std::ptr::null_mut())
        );
        // Bad counts
        for n in [0i32, -1, -100, i32::MIN] {
            let a = (p.c.serialize_encode)(codes_c.as_ptr(), n, &mut bufc, &mut lc, std::ptr::null_mut());
            let b = (p.r.serialize_encode)(codes_r.as_ptr(), n, &mut bufr, &mut lr, std::ptr::null_mut());
            assert_eq!(a, b, "serialize_encode count={}", n);
            assert_eq!(a, err::BADDATA);
        }
        // A NULL code inside the vector
        let bad_c: [Code; 2] = [cp.c, std::ptr::null_mut()];
        let bad_r: [Code; 2] = [cp.r, std::ptr::null_mut()];
        let a = (p.c.serialize_encode)(bad_c.as_ptr(), 2, &mut bufc, &mut lc, std::ptr::null_mut());
        let b = (p.r.serialize_encode)(bad_r.as_ptr(), 2, &mut bufr, &mut lr, std::ptr::null_mut());
        assert_eq!(a, b, "serialize_encode with NULL entry");

        // Mixed tables -> MIXEDTABLES
        let tc = (p.c.maketables)(std::ptr::null_mut());
        let tr = (p.r.maketables)(std::ptr::null_mut());
        assert!(!tc.is_null() && !tr.is_null());
        let ccc = (p.c.compile_context_create)(std::ptr::null_mut());
        let ccr = (p.r.compile_context_create)(std::ptr::null_mut());
        assert_eq!((p.c.set_character_tables)(ccc, tc), (p.r.set_character_tables)(ccr, tr));
        let cp2 = compile_both(p, b"xyz", 3, 0, ccc, ccr, "mixedtables").unwrap();
        let two_c = [cp.c, cp2.c];
        let two_r = [cp.r, cp2.r];
        let a = (p.c.serialize_encode)(two_c.as_ptr(), 2, &mut bufc, &mut lc, std::ptr::null_mut());
        let b = (p.r.serialize_encode)(two_r.as_ptr(), 2, &mut bufr, &mut lr, std::ptr::null_mut());
        assert_eq!(a, b, "serialize_encode mixed tables");
        assert_eq!(a, err::MIXEDTABLES);
        free_code_pair(p, cp2);
        (p.c.compile_context_free)(ccc);
        (p.r.compile_context_free)(ccr);
        (p.c.maketables_free)(std::ptr::null_mut(), tc);
        (p.r.maketables_free)(std::ptr::null_mut(), tr);

        // decode / get_number_of_codes with NULL and garbage
        let mut outc: [Code; 4] = [std::ptr::null_mut(); 4];
        let mut outr: [Code; 4] = [std::ptr::null_mut(); 4];
        assert_eq!(
            (p.c.serialize_decode)(std::ptr::null_mut(), 1, std::ptr::null(), std::ptr::null_mut()),
            (p.r.serialize_decode)(std::ptr::null_mut(), 1, std::ptr::null(), std::ptr::null_mut())
        );
        assert_eq!(
            (p.c.serialize_decode)(outc.as_mut_ptr(), 1, std::ptr::null(), std::ptr::null_mut()),
            (p.r.serialize_decode)(outr.as_mut_ptr(), 1, std::ptr::null(), std::ptr::null_mut())
        );
        assert_eq!(
            (p.c.serialize_get_number_of_codes)(std::ptr::null()),
            (p.r.serialize_get_number_of_codes)(std::ptr::null())
        );
        let garbage = [0u8; 64];
        assert_eq!(
            (p.c.serialize_get_number_of_codes)(garbage.as_ptr()),
            (p.r.serialize_get_number_of_codes)(garbage.as_ptr())
        );
        let a = (p.c.serialize_decode)(outc.as_mut_ptr(), 1, garbage.as_ptr(), std::ptr::null_mut());
        let b = (p.r.serialize_decode)(outr.as_mut_ptr(), 1, garbage.as_ptr(), std::ptr::null_mut());
        assert_eq!(a, b, "serialize_decode garbage");
        assert!(a < 0);
    }
    free_code_pair(p, cp);
}

#[test]
fn serialize_roundtrip_and_truncation() {
    let p = libs();
    for pat in [
        &b"abc"[..],
        &b"(a)(?<n>b)|c+"[..],
        &b"[\\x{100}-\\x{200}]"[..],
        &b"(?i)abc"[..],
    ] {
        let opts = if pat.contains(&b'x') { o::UTF } else { 0 };
        let cp = compile_ok(libs(), pat, opts);
        unsafe {
            let mut bufc: *mut u8 = std::ptr::null_mut();
            let mut bufr: *mut u8 = std::ptr::null_mut();
            let mut lc: Sz = 0;
            let mut lr: Sz = 0;
            let cc = [cp.c];
            let rr = [cp.r];
            let a = (p.c.serialize_encode)(cc.as_ptr(), 1, &mut bufc, &mut lc, std::ptr::null_mut());
            let b = (p.r.serialize_encode)(rr.as_ptr(), 1, &mut bufr, &mut lr, std::ptr::null_mut());
            assert_eq!((a, lc), (b, lr));
            assert_eq!(
                std::slice::from_raw_parts(bufc, lc),
                std::slice::from_raw_parts(bufr, lr),
                "serialized bytes for {:?}",
                String::from_utf8_lossy(pat)
            );
            assert_eq!(
                (p.c.serialize_get_number_of_codes)(bufc),
                (p.r.serialize_get_number_of_codes)(bufr)
            );
            // Cross-decode: the Rust blob must decode in C and vice versa.
            let mut o1: [Code; 2] = [std::ptr::null_mut(); 2];
            let mut o2: [Code; 2] = [std::ptr::null_mut(); 2];
            let a = (p.c.serialize_decode)(o1.as_mut_ptr(), 1, bufr, std::ptr::null_mut());
            let b = (p.r.serialize_decode)(o2.as_mut_ptr(), 1, bufc, std::ptr::null_mut());
            assert_eq!(a, b, "cross-decode rc");
            if a > 0 {
                let dp = CodePair { c: o1[0], r: o2[0] };
                cmp_all_pattern_info(p, &dp, "cross-decoded");
                cmp_compiled_bytes(p, &dp, "cross-decoded");
                free_code_pair(p, dp);
            }
            // Ask for fewer codes than present, and more.
            for n in [0i32, 1, 2, -1] {
                let mut oc: [Code; 4] = [std::ptr::null_mut(); 4];
                let mut or: [Code; 4] = [std::ptr::null_mut(); 4];
                let a = (p.c.serialize_decode)(oc.as_mut_ptr(), n, bufc, std::ptr::null_mut());
                let b = (p.r.serialize_decode)(or.as_mut_ptr(), n, bufr, std::ptr::null_mut());
                assert_eq!(a, b, "serialize_decode n={}", n);
                if a > 0 {
                    for i in 0..(a as usize) {
                        let dp = CodePair { c: oc[i], r: or[i] };
                        cmp_compiled_bytes(p, &dp, "decoded");
                        free_code_pair(p, dp);
                    }
                }
            }
            // Corrupt each header field in turn.
            for off in 0..32usize.min(lc) {
                let mut tc = std::slice::from_raw_parts(bufc, lc).to_vec();
                let mut tr = std::slice::from_raw_parts(bufr, lr).to_vec();
                tc[off] ^= 0xFF;
                tr[off] ^= 0xFF;
                let mut oc: [Code; 4] = [std::ptr::null_mut(); 4];
                let mut or: [Code; 4] = [std::ptr::null_mut(); 4];
                let a = (p.c.serialize_get_number_of_codes)(tc.as_ptr());
                let b = (p.r.serialize_get_number_of_codes)(tr.as_ptr());
                assert_eq!(a, b, "get_number_of_codes corrupt@{}", off);
                let a = (p.c.serialize_decode)(oc.as_mut_ptr(), 1, tc.as_ptr(), std::ptr::null_mut());
                let b = (p.r.serialize_decode)(or.as_mut_ptr(), 1, tr.as_ptr(), std::ptr::null_mut());
                assert_eq!(a, b, "serialize_decode corrupt@{}", off);
                if a > 0 {
                    for i in 0..(a as usize) {
                        (p.c.code_free)(oc[i]);
                        (p.r.code_free)(or[i]);
                    }
                }
            }
            (p.c.serialize_free)(bufc);
            (p.r.serialize_free)(bufr);
        }
        free_code_pair(p, cp);
    }
}

// ===========================================================================
// pcre2_pattern_convert
// ===========================================================================

#[test]
fn convert_error_paths() {
    let p = libs();
    let mut bc: *mut u8 = std::ptr::null_mut();
    let mut br: *mut u8 = std::ptr::null_mut();
    let mut lc: Sz = 0;
    let mut lr: Sz = 0;
    unsafe {
        // NULL pattern / NULL out pointer
        for (pat, plen) in [(std::ptr::null::<u8>(), 3usize), (std::ptr::null(), 0)] {
            let a = (p.c.pattern_convert)(pat, plen, o::CONVERT_GLOB, &mut bc, &mut lc, std::ptr::null_mut());
            let b = (p.r.pattern_convert)(pat, plen, o::CONVERT_GLOB, &mut br, &mut lr, std::ptr::null_mut());
            assert_eq!(a, b, "convert NULL pattern len {}", plen);
        }
        let pat = b"a*b";
        let a = (p.c.pattern_convert)(pat.as_ptr(), 3, o::CONVERT_GLOB, std::ptr::null_mut(), &mut lc, std::ptr::null_mut());
        let b = (p.r.pattern_convert)(pat.as_ptr(), 3, o::CONVERT_GLOB, std::ptr::null_mut(), &mut lr, std::ptr::null_mut());
        assert_eq!(a, b, "convert NULL buffer");

        // Every single option bit, plus invalid combinations (two conversion types).
        for bit in 0..32u32 {
            let opt = 1u32 << bit;
            let a = (p.c.pattern_convert)(pat.as_ptr(), 3, opt, &mut bc, &mut lc, std::ptr::null_mut());
            let b = (p.r.pattern_convert)(pat.as_ptr(), 3, opt, &mut br, &mut lr, std::ptr::null_mut());
            assert_eq!(a, b, "convert optbit {}", bit);
            if a == 0 {
                assert_eq!(lc, lr, "convert optbit {} len", bit);
                assert_eq!(
                    std::slice::from_raw_parts(bc, lc),
                    std::slice::from_raw_parts(br, lr),
                    "convert optbit {} bytes",
                    bit
                );
                (p.c.converted_pattern_free)(bc);
                (p.r.converted_pattern_free)(br);
            }
        }
        for opt in [
            o::CONVERT_POSIX_BASIC | o::CONVERT_POSIX_EXTENDED,
            o::CONVERT_GLOB | o::CONVERT_POSIX_BASIC,
            o::CONVERT_GLOB_NO_WILD_SEPARATOR | o::CONVERT_GLOB_NO_STARSTAR,
            0,
            u32::MAX,
        ] {
            let a = (p.c.pattern_convert)(pat.as_ptr(), 3, opt, &mut bc, &mut lc, std::ptr::null_mut());
            let b = (p.r.pattern_convert)(pat.as_ptr(), 3, opt, &mut br, &mut lr, std::ptr::null_mut());
            assert_eq!(a, b, "convert opts {:#x}", opt);
            if a == 0 {
                assert_eq!(
                    std::slice::from_raw_parts(bc, lc),
                    std::slice::from_raw_parts(br, lr)
                );
                (p.c.converted_pattern_free)(bc);
                (p.r.converted_pattern_free)(br);
            }
        }
    }
}

#[test]
fn convert_syntax_errors() {
    let p = libs();
    let cases: &[(&[u8], u32)] = &[
        (b"[", o::CONVERT_GLOB),
        (b"[a", o::CONVERT_GLOB),
        (b"[!", o::CONVERT_GLOB),
        (b"[]", o::CONVERT_GLOB),
        (b"[a-", o::CONVERT_GLOB),
        (b"\\", o::CONVERT_GLOB),
        (b"a\\", o::CONVERT_GLOB),
        (b"**", o::CONVERT_GLOB),
        (b"a**b", o::CONVERT_GLOB),
        (b"/**/", o::CONVERT_GLOB),
        (b"[", o::CONVERT_POSIX_BASIC),
        (b"[a", o::CONVERT_POSIX_BASIC),
        (b"a\\", o::CONVERT_POSIX_BASIC),
        (b"[", o::CONVERT_POSIX_EXTENDED),
        (b"a\\", o::CONVERT_POSIX_EXTENDED),
        (b"[[:foo:]]", o::CONVERT_POSIX_EXTENDED),
        (b"a{1,2}", o::CONVERT_POSIX_EXTENDED),
        (b"*", o::CONVERT_POSIX_EXTENDED),
        (b"\\x{d800}", o::CONVERT_GLOB | o::CONVERT_UTF),
        (&[0xC2], o::CONVERT_GLOB | o::CONVERT_UTF),
        (&[0xC2], o::CONVERT_GLOB | o::CONVERT_UTF | o::CONVERT_NO_UTF_CHECK),
    ];
    unsafe {
        for (pat, opt) in cases {
            let mut bc: *mut u8 = std::ptr::null_mut();
            let mut br: *mut u8 = std::ptr::null_mut();
            let mut lc: Sz = 0xAAAA;
            let mut lr: Sz = 0x5555;
            let a = (p.c.pattern_convert)(pat.as_ptr(), pat.len(), *opt, &mut bc, &mut lc, std::ptr::null_mut());
            let b = (p.r.pattern_convert)(pat.as_ptr(), pat.len(), *opt, &mut br, &mut lr, std::ptr::null_mut());
            assert_eq!(
                a, b,
                "convert {:?} opts {:#x}",
                String::from_utf8_lossy(pat), opt
            );
            assert_eq!(lc, lr, "convert {:?} length out", String::from_utf8_lossy(pat));
            if a == 0 {
                assert_eq!(
                    std::slice::from_raw_parts(bc, lc),
                    std::slice::from_raw_parts(br, lr),
                    "convert {:?} bytes",
                    String::from_utf8_lossy(pat)
                );
                (p.c.converted_pattern_free)(bc);
                (p.r.converted_pattern_free)(br);
            }
        }
    }
}

// ===========================================================================
// pcre2_get_error_message
// ===========================================================================

#[test]
fn get_error_message_every_code_and_boundary() {
    let p = libs();
    for code in -80i32..=225 {
        for cap in [0usize, 1, 2, 5, 512] {
            let mut bc = vec![0u8; 600];
            let mut br = vec![0u8; 600];
            let a = unsafe { (p.c.get_error_message)(code, bc.as_mut_ptr(), cap) };
            let b = unsafe { (p.r.get_error_message)(code, br.as_mut_ptr(), cap) };
            assert_eq!(a, b, "get_error_message({}, cap {}) rc", code, cap);
            assert_eq!(bc, br, "get_error_message({}, cap {}) buffer", code, cap);
        }
    }
    // Extremes
    for code in [i32::MIN, i32::MIN + 1, -1000, 226, 1000, i32::MAX] {
        let mut bc = vec![0u8; 600];
        let mut br = vec![0u8; 600];
        let a = unsafe { (p.c.get_error_message)(code, bc.as_mut_ptr(), 512) };
        let b = unsafe { (p.r.get_error_message)(code, br.as_mut_ptr(), 512) };
        assert_eq!(a, b, "get_error_message({}) rc", code);
        assert_eq!(bc, br, "get_error_message({}) buffer", code);
    }
}

// ===========================================================================
// pcre2_next_match
// ===========================================================================

#[test]
fn next_match_error_paths() {
    let p = libs();
    let cp = compile_ok(libs(), b"a|b", 0);
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
        // NOTE: pcre2_next_match() reads `match_data->rc`, which
        // pcre2_match_data_create() leaves uninitialised, so calling it before any
        // match is undefined behaviour in the C library. Start from a real match.
        let mut oc: Sz = 0;
        let mut or: Sz = 0;
        let mut lc: u32 = 0;
        let mut lr: u32 = 0;
        let _ = (&mut oc, &mut or, &mut lc, &mut lr);
        // After a match, iterate to exhaustion
        let subj = b"ab";
        assert_eq!(
            (p.c.pcre2_match)(cp.c, subj.as_ptr(), 2, 0, 0, mdc, std::ptr::null_mut()),
            (p.r.pcre2_match)(cp.r, subj.as_ptr(), 2, 0, 0, mdr, std::ptr::null_mut())
        );
        for i in 0..5 {
            let mut oc: Sz = 0xAAAA;
            let mut or: Sz = 0x5555;
            let mut lc: u32 = 0xAAAA;
            let mut lr: u32 = 0x5555;
            let a = (p.c.next_match)(mdc, &mut oc, &mut lc);
            let b = (p.r.next_match)(mdr, &mut or, &mut lr);
            assert_eq!(a, b, "next_match iteration {}", i);
            if a > 0 {
                assert_eq!((oc, lc), (or, lr), "next_match iteration {} values", i);
            }
        }
        // NOTE: pcre2_next_match() writes through `pstart_offset`/`poptions`
        // without NULL checks (see pcre2_match_next.c), so NULL out-pointers are
        // undefined behaviour in the C library, not a testable rejection.
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp);
}

// ===========================================================================
// match_data creation edge cases
// ===========================================================================

#[test]
fn match_data_create_edge_cases() {
    let p = libs();
    for n in [0u32, 1, 2, 3, 65535, 65536, u32::MAX] {
        unsafe {
            let mdc = (p.c.match_data_create)(n, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(n, std::ptr::null_mut());
            assert_eq!(mdc.is_null(), mdr.is_null(), "match_data_create({}) null-ness", n);
            if !mdc.is_null() {
                assert_eq!(
                    (p.c.get_ovector_count)(mdc),
                    (p.r.get_ovector_count)(mdr),
                    "ovector count for {}",
                    n
                );
                assert_eq!(
                    (p.c.get_match_data_size)(mdc),
                    (p.r.get_match_data_size)(mdr),
                    "match_data_size for {}",
                    n
                );
                assert_eq!(
                    (p.c.get_match_data_heapframes_size)(mdc),
                    (p.r.get_match_data_heapframes_size)(mdr),
                    "heapframes size for {}",
                    n
                );
                (p.c.match_data_free)(mdc);
                (p.r.match_data_free)(mdr);
            }
        }
    }
    // create_from_pattern with NULL code
    unsafe {
        let a = (p.c.match_data_create_from_pattern)(std::ptr::null_mut(), std::ptr::null_mut());
        let b = (p.r.match_data_create_from_pattern)(std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(a.is_null(), b.is_null());
        if !a.is_null() {
            (p.c.match_data_free)(a);
            (p.r.match_data_free)(b);
        }
    }
}

#[test]
fn ovector_too_small_is_reported_identically() {
    let p = libs();
    let cp = compile_ok(libs(), b"(a)(b)(c)(d)", 0);
    unsafe {
        for ovn in [0u32, 1, 2, 3, 4, 5, 6] {
            let mdc = (p.c.match_data_create)(ovn, std::ptr::null_mut());
            let mdr = (p.r.match_data_create)(ovn, std::ptr::null_mut());
            let subj = b"abcd";
            let a = (p.c.pcre2_match)(cp.c, subj.as_ptr(), 4, 0, 0, mdc, std::ptr::null_mut());
            let b = (p.r.pcre2_match)(cp.r, subj.as_ptr(), 4, 0, 0, mdr, std::ptr::null_mut());
            assert_eq!(a, b, "ovector {} rc", ovn);
            // Only the pairs the C library actually fills are defined:
            //   rc > 0  -> `rc` pairs
            //   rc == 0 -> the whole (too-small) ovector
            // Anything beyond that is untouched malloc memory in BOTH libraries.
            let defined = if a > 0 { a as usize } else { (p.c.get_ovector_count)(mdc) as usize };
            let n = defined * 2;
            let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), n);
            let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), n);
            assert_eq!(oc, or, "ovector {} contents", ovn);
            assert_eq!(
                (p.c.get_ovector_count)(mdc),
                (p.r.get_ovector_count)(mdr),
                "ovector {} count",
                ovn
            );
            assert_eq!(
                (p.c.get_startchar)(mdc),
                (p.r.get_startchar)(mdr),
                "ovector {} startchar",
                ovn
            );
            (p.c.match_data_free)(mdc);
            (p.r.match_data_free)(mdr);
        }
    }
    free_code_pair(p, cp);
}

// ===========================================================================
// JIT stubs
// ===========================================================================

#[test]
fn jit_functions_agree() {
    let p = libs();
    let cp = compile_ok(libs(), b"abc", 0);
    unsafe {
        for opt in [
            0u32,
            o::JIT_COMPLETE,
            o::JIT_PARTIAL_SOFT,
            o::JIT_PARTIAL_HARD,
            o::JIT_INVALID_UTF,
            o::JIT_TEST_ALLOC,
            u32::MAX,
        ] {
            let a = (p.c.jit_compile)(cp.c, opt);
            let b = (p.r.jit_compile)(cp.r, opt);
            assert_eq!(a, b, "jit_compile({:#x})", opt);
        }
        assert_eq!(
            (p.c.jit_compile)(std::ptr::null_mut(), o::JIT_COMPLETE),
            (p.r.jit_compile)(std::ptr::null_mut(), o::JIT_COMPLETE)
        );
        let sc = (p.c.jit_stack_create)(1024, 1024 * 1024, std::ptr::null_mut());
        let sr = (p.r.jit_stack_create)(1024, 1024 * 1024, std::ptr::null_mut());
        assert_eq!(sc.is_null(), sr.is_null(), "jit_stack_create");
        (p.c.jit_stack_free)(sc);
        (p.r.jit_stack_free)(sr);
        (p.c.jit_stack_free)(std::ptr::null_mut());
        (p.r.jit_stack_free)(std::ptr::null_mut());
        (p.c.jit_free_unused_memory)(std::ptr::null_mut());
        (p.r.jit_free_unused_memory)(std::ptr::null_mut());
        let tc = (p.c.priv_jit_get_target)();
        let tr = (p.r.priv_jit_get_target)();
        assert_eq!(tc.is_null(), tr.is_null(), "_pcre2_jit_get_target null-ness");
        if !tc.is_null() {
            let a = std::ffi::CStr::from_ptr(tc);
            let b = std::ffi::CStr::from_ptr(tr);
            assert_eq!(a, b, "_pcre2_jit_get_target");
        }
        assert_eq!(
            (p.c.priv_jit_get_size)(std::ptr::null_mut()),
            (p.r.priv_jit_get_size)(std::ptr::null_mut())
        );
        // JITSIZE via pattern_info
        cmp_info_usize(p, &cp, info::JITSIZE, "jitsize");
    }
    free_code_pair(p, cp);
}
