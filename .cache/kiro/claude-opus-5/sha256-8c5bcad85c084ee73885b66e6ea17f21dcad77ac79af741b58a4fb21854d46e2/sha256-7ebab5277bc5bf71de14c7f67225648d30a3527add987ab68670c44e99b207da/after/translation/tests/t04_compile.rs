//! `pcre2_compile.c` and everything it pulls in (parse/branch/class/cgroup/scan/
//! tables, `_pcre2_check_escape`, `_pcre2_auto_possessify`, `_pcre2_study`,
//! `_pcre2_find_bracket`, `_pcre2_ord2utf`). The whole compiled block is
//! compared byte-for-byte, which is the strongest check available.
mod common;

use common::*;
use std::ffi::c_void;

type CtxCreateFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type CtxFreeFn = unsafe extern "C" fn(*mut c_void);
type SetU32Fn = unsafe extern "C" fn(*mut c_void, u32) -> std::ffi::c_int;
type SetSizeFn = unsafe extern "C" fn(*mut c_void, PCRE2_SIZE) -> std::ffi::c_int;
type SetPtrFn = unsafe extern "C" fn(*mut c_void, *const u8) -> std::ffi::c_int;

#[test]
fn compile_all_patterns_default_options() {
    for p in patterns() {
        let _ = compile_both(p, 0);
    }
}

#[test]
fn compile_all_patterns_all_option_sets() {
    let pats = patterns();
    for &opts in &compile_option_sets() {
        for p in &pats {
            let _ = compile_both(p, opts);
        }
    }
}

#[test]
fn compile_zero_terminated_length() {
    // PCRE2_ZERO_TERMINATED takes a different path through the length handling.
    let (cc, rc) = both::<CompileFn>("pcre2_compile_8");
    let (cf, rf) = both::<CodeFreeFn>("pcre2_code_free_8");
    for p in patterns() {
        if p.contains(&0) {
            continue; // not representable as a zero-terminated string
        }
        let mut z = p.to_vec();
        z.push(0);
        for &opts in &[0u32, PCRE2_UTF, PCRE2_CASELESS, PCRE2_EXTENDED] {
            let mut ec_c = -999;
            let mut eo_c = usize::MAX;
            let mut ec_r = -999;
            let mut eo_r = usize::MAX;
            unsafe {
                let a = cc(
                    z.as_ptr(),
                    PCRE2_ZERO_TERMINATED,
                    opts,
                    &mut ec_c,
                    &mut eo_c,
                    std::ptr::null_mut(),
                );
                let b = rc(
                    z.as_ptr(),
                    PCRE2_ZERO_TERMINATED,
                    opts,
                    &mut ec_r,
                    &mut eo_r,
                    std::ptr::null_mut(),
                );
                let show = String::from_utf8_lossy(p).to_string();
                assert_eq!(ec_c, ec_r, "zt errorcode {show:?} opts={opts:#x}");
                assert_eq!(eo_c, eo_r, "zt erroroffset {show:?} opts={opts:#x}");
                assert_eq!(a.is_null(), b.is_null(), "zt nullness {show:?}");
                if !a.is_null() {
                    let (bm_c, tail_c, body_c) = code_snapshot(a);
                    let (bm_r, tail_r, body_r) = code_snapshot(b);
                    assert_bytes_eq(&format!("zt bitmap {show:?}"), &bm_c, &bm_r);
                    assert_eq!(tail_c, tail_r, "zt header {show:?} opts={opts:#x}");
                    assert_bytes_eq(&format!("zt body {show:?}"), &body_c, &body_r);
                    cf(a);
                    rf(b);
                }
            }
        }
    }
}

#[test]
fn compile_with_extra_options() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_compile_context_create_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_compile_context_free_8");
    let (sec, ser) = both::<SetU32Fn>("pcre2_set_compile_extra_options_8");
    let pats = patterns();
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        for &xo in &extra_option_sets() {
            assert_eq!(sec(a, xo), ser(b, xo));
            for &opts in &[0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
                for p in &pats {
                    let _ = compile_both_ctx(p, opts, a, b);
                }
            }
        }
        cf(a);
        rf(b);
    }
}

#[test]
fn compile_with_newline_and_bsr_conventions() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_compile_context_create_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_compile_context_free_8");
    let (snc, snr) = both::<SetU32Fn>("pcre2_set_newline_8");
    let (sbc, sbr) = both::<SetU32Fn>("pcre2_set_bsr_8");
    let pats = patterns();
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        for nl in 1u32..=6 {
            assert_eq!(snc(a, nl), snr(b, nl));
            for bsr in 1u32..=2 {
                assert_eq!(sbc(a, bsr), sbr(b, bsr));
                for p in &pats {
                    let _ = compile_both_ctx(p, 0, a, b);
                    let _ = compile_both_ctx(p, PCRE2_UTF, a, b);
                }
            }
        }
        cf(a);
        rf(b);
    }
}

#[test]
fn compile_with_limits_and_optimization_flags() {
    let (cc, rc) = both::<CtxCreateFn>("pcre2_compile_context_create_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_compile_context_free_8");
    let pats = patterns();
    unsafe {
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());

        // Optimization directives: 0 = FULL_OPTIMIZE, 1 = NO_OPTIMIZE, ...
        let (soc, sor) = both::<SetU32Fn>("pcre2_set_optimize_8");
        for v in 0u32..=6 {
            assert_eq!(soc(a, v), sor(b, v), "set_optimize({v})");
            for p in &pats {
                let _ = compile_both_ctx(p, 0, a, b);
            }
        }
        assert_eq!(soc(a, 0), sor(b, 0));

        // Max pattern length rejections.
        let (smc, smr) = both::<SetSizeFn>("pcre2_set_max_pattern_length_8");
        for lim in [0usize, 1, 2, 5, 10, 20] {
            assert_eq!(smc(a, lim), smr(b, lim));
            for p in &pats {
                let _ = compile_both_ctx(p, 0, a, b);
            }
        }
        assert_eq!(smc(a, PCRE2_UNSET), smr(b, PCRE2_UNSET));

        // Max compiled length rejections.
        let (scc, scr) = both::<SetSizeFn>("pcre2_set_max_pattern_compiled_length_8");
        for lim in [0usize, 1, 16, 64, 128, 256] {
            assert_eq!(scc(a, lim), scr(b, lim));
            for p in &pats {
                let _ = compile_both_ctx(p, 0, a, b);
            }
        }
        assert_eq!(scc(a, PCRE2_UNSET), scr(b, PCRE2_UNSET));

        // Parens nest limit.
        let (spc, spr) = both::<SetU32Fn>("pcre2_set_parens_nest_limit_8");
        for lim in [0u32, 1, 2, 3, 5, 250] {
            assert_eq!(spc(a, lim), spr(b, lim));
            for p in &pats {
                let _ = compile_both_ctx(p, 0, a, b);
            }
        }
        assert_eq!(spc(a, 250), spr(b, 250));

        // Max varlookbehind.
        let (svc, svr) = both::<SetU32Fn>("pcre2_set_max_varlookbehind_8");
        for lim in [0u32, 1, 2, 255] {
            assert_eq!(svc(a, lim), svr(b, lim));
            for p in &pats {
                let _ = compile_both_ctx(p, 0, a, b);
            }
        }

        cf(a);
        rf(b);
    }
}

#[test]
fn compile_with_custom_character_tables() {
    // Tables built by pcre2_maketables must drive both compilers identically.
    let (mtc, mtr) = both::<unsafe extern "C" fn(*mut c_void) -> *const u8>("pcre2_maketables_8");
    let (mfc, mfr) =
        both::<unsafe extern "C" fn(*mut c_void, *const u8)>("pcre2_maketables_free_8");
    let (cc, rc) = both::<CtxCreateFn>("pcre2_compile_context_create_8");
    let (cf, rf) = both::<CtxFreeFn>("pcre2_compile_context_free_8");
    let (stc, str_) = both::<SetPtrFn>("pcre2_set_character_tables_8");
    unsafe {
        let tc = mtc(std::ptr::null_mut());
        let tr = mtr(std::ptr::null_mut());
        let a = cc(std::ptr::null_mut());
        let b = rc(std::ptr::null_mut());
        assert_eq!(stc(a, tc), str_(b, tr));
        for p in patterns() {
            let _ = compile_both_ctx(p, 0, a, b);
            let _ = compile_both_ctx(p, PCRE2_CASELESS, a, b);
        }
        cf(a);
        rf(b);
        mfc(std::ptr::null_mut(), tc);
        mfr(std::ptr::null_mut(), tr);
    }
}

#[test]
fn code_copy_matches() {
    let (ccc, crc) = both::<unsafe extern "C" fn(*const c_void) -> *mut c_void>("pcre2_code_copy_8");
    let (cwc, cwr) =
        both::<unsafe extern "C" fn(*const c_void) -> *mut c_void>("pcre2_code_copy_with_tables_8");
    let (cf, rf) = both::<CodeFreeFn>("pcre2_code_free_8");
    for p in patterns() {
        let Some(pair) = compile_both(p, 0) else {
            continue;
        };
        unsafe {
            let a = ccc(pair.c);
            let b = crc(pair.r);
            assert_eq!(a.is_null(), b.is_null());
            if !a.is_null() {
                let (bm_c, tail_c, body_c) = code_snapshot(a);
                let (bm_r, tail_r, body_r) = code_snapshot(b);
                assert_bytes_eq("code_copy bitmap", &bm_c, &bm_r);
                assert_eq!(tail_c, tail_r, "code_copy header");
                assert_bytes_eq("code_copy body", &body_c, &body_r);
                cf(a);
                rf(b);
            }

            let a = cwc(pair.c);
            let b = cwr(pair.r);
            assert_eq!(a.is_null(), b.is_null());
            if !a.is_null() {
                let (bm_c, tail_c, body_c) = code_snapshot(a);
                let (bm_r, tail_r, body_r) = code_snapshot(b);
                assert_bytes_eq("code_copy_with_tables bitmap", &bm_c, &bm_r);
                assert_eq!(tail_c, tail_r, "code_copy_with_tables header");
                assert_bytes_eq("code_copy_with_tables body", &body_c, &body_r);
                // The copied tables must be identical too (they trail the block).
                let ta = std::ptr::read_unaligned((a as *const u8).add(CODE_OFF_TABLES)
                    as *const *const u8);
                let tb = std::ptr::read_unaligned((b as *const u8).add(CODE_OFF_TABLES)
                    as *const *const u8);
                assert_bytes_eq(
                    "code_copy_with_tables tables",
                    slice_at(ta, 1088),
                    slice_at(tb, 1088),
                );
                cf(a);
                rf(b);
            }
        }
    }
}
