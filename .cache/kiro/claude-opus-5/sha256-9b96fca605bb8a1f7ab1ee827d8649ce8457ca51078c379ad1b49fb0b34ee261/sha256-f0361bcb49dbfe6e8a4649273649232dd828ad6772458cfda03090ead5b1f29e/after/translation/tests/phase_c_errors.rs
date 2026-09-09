//! Phase C — error-path differential tests, part 1:
//! `ERRORS.md` rows A1–A5, B1–B16, C1–C8, M1–M10.
//!
//! Every call goes through `libloading` into BOTH `.so` files.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// =============================================================== A. error codes

#[test]
fn a1_a2_a3_is_error_get_code_get_name() {
    let p = libs();
    let (c_is, r_is) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_gc, r_gc) = p.sym::<FnGetErrCode>("ZSTD_getErrorCode");
    let (c_gn, r_gn) = p.sym::<FnGetErrName>("ZSTD_getErrorName");

    for code in 0..=200usize {
        // A1
        unsafe { eq(&format!("ZSTD_isError({code})"), c_is(code), r_is(code)) };
        // also the wrapped-negative form the API actually returns
        let neg = 0usize.wrapping_sub(code);
        unsafe { eq(&format!("ZSTD_isError(-{code})"), c_is(neg), r_is(neg)) };
        // A2
        unsafe {
            eq(
                &format!("ZSTD_getErrorCode(-{code})"),
                c_gc(neg),
                r_gc(neg),
            )
        };
        // A3 / A5
        unsafe {
            eq(
                &format!("ZSTD_getErrorName(-{code})"),
                cstr(c_gn(neg)),
                cstr(r_gn(neg)),
            )
        };
    }
}

#[test]
fn a4_get_error_string_out_of_range_enum() {
    let p = libs();
    type F = unsafe extern "C" fn(c_int) -> *const c_char;
    let (c, r) = p.sym::<F>("ZSTD_getErrorString");
    // C enums accept any int: sweep well past the valid range.
    for v in -5..=130i32 {
        unsafe { eq(&format!("ZSTD_getErrorString({v})"), cstr(c(v)), cstr(r(v))) };
    }
    for v in [
        -1000,
        -1,
        121,
        200,
        9999,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
    ] {
        unsafe { eq(&format!("ZSTD_getErrorString({v})"), cstr(c(v)), cstr(r(v))) };
    }
}

#[test]
fn a_fse_huf_zdict_zbuff_error_names() {
    let p = libs();
    for name in [
        "FSE_getErrorName",
        "HUF_getErrorName",
        "ZDICT_getErrorName",
        "ZBUFF_getErrorName",
    ] {
        let (c, r) = p.sym::<FnGetErrName>(name);
        for code in 0..=130usize {
            let neg = 0usize.wrapping_sub(code);
            unsafe { eq(&format!("{name}(-{code})"), cstr(c(neg)), cstr(r(neg))) };
        }
    }
    for name in ["FSE_isError", "HUF_isError", "ZDICT_isError", "ZBUFF_isError"] {
        let (c, r) = p.sym::<FnIsError>(name);
        for code in 0..=130usize {
            let neg = 0usize.wrapping_sub(code);
            unsafe { eq(&format!("{name}(-{code})"), c(neg), r(neg)) };
        }
    }
}

// ========================================================= B. cParam bounds

fn bounds_pair(p: &Pair, sym: &str, param: c_int) -> (Bounds, Bounds) {
    let (c, r) = p.sym::<FnGetBounds>(sym);
    unsafe { (c(param), r(param)) }
}

#[test]
fn b1_cparam_get_bounds_valid() {
    let p = libs();
    for (v, name) in C_PARAMS {
        let (cb, rb) = bounds_pair(p, "ZSTD_cParam_getBounds", v);
        eq(&format!("ZSTD_cParam_getBounds({name}={v})"), cb, rb);
        assert_eq!(cb.error, 0, "C says {name} is unsupported; table is wrong");
    }
}

#[test]
fn b2_cparam_get_bounds_out_of_range_enum() {
    let p = libs();
    for v in BAD_ENUM_INTS {
        let (cb, rb) = bounds_pair(p, "ZSTD_cParam_getBounds", v);
        eq(&format!("ZSTD_cParam_getBounds(bad {v})"), cb, rb);
    }
    for v in [-1, 0, 1, 99, 108, 131, 165, 203, 403, 501, 1018, 1_000_000] {
        let (cb, rb) = bounds_pair(p, "ZSTD_cParam_getBounds", v);
        eq(&format!("ZSTD_cParam_getBounds(bad {v})"), cb, rb);
    }
}

#[test]
fn c1_dparam_get_bounds_valid() {
    let p = libs();
    for (v, name) in D_PARAMS {
        let (cb, rb) = bounds_pair(p, "ZSTD_dParam_getBounds", v);
        eq(&format!("ZSTD_dParam_getBounds({name}={v})"), cb, rb);
        assert_eq!(cb.error, 0, "C says d-{name} is unsupported; table is wrong");
    }
}

#[test]
fn c2_dparam_get_bounds_out_of_range_enum() {
    let p = libs();
    for v in BAD_ENUM_INTS
        .iter()
        .copied()
        .chain([-1, 0, 1, 99, 101, 999, 1006, 1007, 1_000_000])
    {
        let (cb, rb) = bounds_pair(p, "ZSTD_dParam_getBounds", v);
        eq(&format!("ZSTD_dParam_getBounds(bad {v})"), cb, rb);
    }
}

/// B3/B4/B5/B6/B15/B16 — drive every parameter one step outside its bounds.
#[test]
fn b3_b6_cctx_set_parameter_out_of_bounds() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_get, r_get) = p.sym::<FnGetParam>("ZSTD_CCtx_getParameter");
    let (c_gb, _r_gb) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");

    unsafe {
        let cc = c_new();
        let rc = r_new();
        assert!(!cc.is_null() && !rc.is_null());

        for (v, name) in C_PARAMS {
            let b = c_gb(v);
            assert_eq!(b.error, 0);
            let probes: Vec<c_int> = vec![
                b.lower.saturating_sub(1),
                b.lower,
                b.upper,
                b.upper.saturating_add(1),
                c_int::MIN,
                c_int::MAX,
            ];
            for val in probes {
                let cres = c_set(cc, v, val);
                let rres = r_set(rc, v, val);
                eq(
                    &format!("ZSTD_CCtx_setParameter({name}={v}, {val})"),
                    cres,
                    rres,
                );
                // B9-adjacent: read-back must agree too
                let mut cv: c_int = 0;
                let mut rv: c_int = 0;
                let cg = c_get(cc, v, &mut cv);
                let rg = r_get(rc, v, &mut rv);
                eq(&format!("ZSTD_CCtx_getParameter({name}) ret"), cg, rg);
                eq(&format!("ZSTD_CCtx_getParameter({name}) value"), cv, rv);
            }
        }

        // B6 — out-of-range param enum ints
        for bad in BAD_ENUM_INTS {
            eq(
                &format!("ZSTD_CCtx_setParameter(bad param {bad}, 1)"),
                c_set(cc, bad, 1),
                r_set(rc, bad, 1),
            );
            let mut cv: c_int = 0;
            let mut rv: c_int = 0;
            eq(
                &format!("ZSTD_CCtx_getParameter(bad param {bad})"),
                c_get(cc, bad, &mut cv),
                r_get(rc, bad, &mut rv),
            );
        }

        c_free(cc);
        r_free(rc);
    }
}

/// B5 — compressionLevel is clamped, not rejected.
#[test]
fn b5_compression_level_clamps() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_get, r_get) = p.sym::<FnGetParam>("ZSTD_CCtx_getParameter");
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for lvl in [
            c_int::MIN,
            -1_000_000,
            -131_073,
            -131_072,
            -1,
            0,
            1,
            19,
            22,
            23,
            1000,
            c_int::MAX,
        ] {
            eq(
                &format!("setParameter(compressionLevel, {lvl})"),
                c_set(cc, 100, lvl),
                r_set(rc, 100, lvl),
            );
            let mut cv = 0;
            let mut rv = 0;
            c_get(cc, 100, &mut cv);
            r_get(rc, 100, &mut rv);
            eq(&format!("getParameter(compressionLevel) after {lvl}"), cv, rv);
        }
        c_free(cc);
        r_free(rc);
    }
}

/// B11/B12/B14 — the CCtxParams object mirror, and reset directives.
#[test]
fn b11_b14_cctx_params_and_reset() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtxParams");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtxParams");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtxParams_setParameter");
    let (c_get, r_get) = p.sym::<FnGetParam>("ZSTD_CCtxParams_getParameter");
    let (c_rst, r_rst) = p.sym::<FnFreeCtx>("ZSTD_CCtxParams_reset");
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");

    unsafe {
        let cp = c_new();
        let rp = r_new();
        assert!(!cp.is_null() && !rp.is_null());
        for (v, name) in C_PARAMS {
            let b = c_gb(v);
            for val in [
                b.lower.saturating_sub(1),
                b.lower,
                (b.lower / 2).saturating_add(b.upper / 2),
                b.upper,
                b.upper.saturating_add(1),
            ] {
                eq(
                    &format!("CCtxParams_setParameter({name}, {val})"),
                    c_set(cp, v, val),
                    r_set(rp, v, val),
                );
                let mut cv = 0;
                let mut rv = 0;
                eq(
                    &format!("CCtxParams_getParameter({name}) ret"),
                    c_get(cp, v, &mut cv),
                    r_get(rp, v, &mut rv),
                );
                eq(&format!("CCtxParams_getParameter({name}) val"), cv, rv);
            }
        }
        for bad in BAD_ENUM_INTS {
            eq(
                &format!("CCtxParams_setParameter(bad {bad})"),
                c_set(cp, bad, 1),
                r_set(rp, bad, 1),
            );
            let mut cv = 0;
            let mut rv = 0;
            eq(
                &format!("CCtxParams_getParameter(bad {bad})"),
                c_get(cp, bad, &mut cv),
                r_get(rp, bad, &mut rv),
            );
        }
        eq("CCtxParams_reset", c_rst(cp), r_rst(rp));
        eq("freeCCtxParams", c_free(cp), r_free(rp));
    }

    // B14 — ZSTD_CCtx_reset with out-of-range directive
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_freec, r_freec) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for d in [0, 1, 2, -1, 3, 4, 99, c_int::MIN, c_int::MAX] {
            eq(
                &format!("ZSTD_CCtx_reset({d})"),
                c_reset(cc, d),
                r_reset(rc, d),
            );
        }
        c_freec(cc);
        r_freec(rc);
    }

    // C6 — ZSTD_DCtx_reset with out-of-range directive
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_freed, r_freed) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_DCtx_reset");
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for d in [0, 1, 2, -1, 3, 4, 99, c_int::MIN, c_int::MAX] {
            eq(
                &format!("ZSTD_DCtx_reset({d})"),
                c_reset(cc, d),
                r_reset(rc, d),
            );
        }
        c_freed(cc);
        r_freed(rc);
    }
}

// ========================================================= C. dParam bounds

#[test]
fn c3_c8_dctx_set_parameter_out_of_bounds() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_get, r_get) = p.sym::<FnGetParam>("ZSTD_DCtx_getParameter");
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_dParam_getBounds");
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for (v, name) in D_PARAMS {
            let b = c_gb(v);
            assert_eq!(b.error, 0);
            for val in [
                b.lower.saturating_sub(1),
                b.lower,
                b.upper,
                b.upper.saturating_add(1),
                c_int::MIN,
                c_int::MAX,
            ] {
                eq(
                    &format!("ZSTD_DCtx_setParameter({name}, {val})"),
                    c_set(cc, v, val),
                    r_set(rc, v, val),
                );
                let mut cv = 0;
                let mut rv = 0;
                eq(
                    &format!("ZSTD_DCtx_getParameter({name}) ret"),
                    c_get(cc, v, &mut cv),
                    r_get(rc, v, &mut rv),
                );
                eq(&format!("ZSTD_DCtx_getParameter({name}) val"), cv, rv);
            }
        }
        for bad in BAD_ENUM_INTS {
            eq(
                &format!("ZSTD_DCtx_setParameter(bad {bad})"),
                c_set(cc, bad, 1),
                r_set(rc, bad, 1),
            );
        }
        c_free(cc);
        r_free(rc);
    }
}

/// C7 — `ZSTD_DCtx_setMaxWindowSize`, C8 — `ZSTD_DCtx_setFormat`.
#[test]
fn c7_c8_set_max_window_size_and_format() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    type FnMw = unsafe extern "C" fn(*mut c_void, Sz) -> Sz;
    let (c_mw, r_mw) = p.sym::<FnMw>("ZSTD_DCtx_setMaxWindowSize");
    let (c_fmt, r_fmt) = p.sym::<FnSetParam>("ZSTD_DCtx_setFormat");
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for w in [
            0usize,
            1,
            1 << 9,
            1 << 10,
            1 << 20,
            1 << 27,
            1 << 30,
            1usize << 31,
            usize::MAX,
            usize::MAX / 2,
        ] {
            eq(
                &format!("ZSTD_DCtx_setMaxWindowSize({w})"),
                c_mw(cc, w),
                r_mw(rc, w),
            );
        }
        for f in [0, 1, -1, 2, 99, c_int::MIN, c_int::MAX] {
            eq(
                &format!("ZSTD_DCtx_setFormat({f})"),
                c_fmt(cc, f, 0),
                r_fmt(rc, f, 0),
            );
        }
        c_free(cc);
        r_free(rc);
    }
}

// ================================================= M. generic FFI boundaries

/// M1/M2/M3/M4 — NULL handles into every free / sizeof / dictID accessor.
#[test]
fn m1_m4_null_handles() {
    let p = libs();
    for name in [
        "ZSTD_freeCCtx",
        "ZSTD_freeDCtx",
        "ZSTD_freeCStream",
        "ZSTD_freeDStream",
        "ZSTD_freeCDict",
        "ZSTD_freeDDict",
        "ZSTD_freeCCtxParams",
    ] {
        let (c, r) = p.sym::<FnFreeCtx>(name);
        unsafe { eq(&format!("{name}(NULL)"), c(std::ptr::null_mut()), r(std::ptr::null_mut())) };
    }
    for name in [
        "ZSTD_sizeof_CCtx",
        "ZSTD_sizeof_DCtx",
        "ZSTD_sizeof_CStream",
        "ZSTD_sizeof_DStream",
        "ZSTD_sizeof_CDict",
        "ZSTD_sizeof_DDict",
    ] {
        type F = unsafe extern "C" fn(*const c_void) -> Sz;
        let (c, r) = p.sym::<F>(name);
        unsafe { eq(&format!("{name}(NULL)"), c(std::ptr::null()), r(std::ptr::null())) };
    }
    for name in ["ZSTD_getDictID_fromCDict", "ZSTD_getDictID_fromDDict"] {
        type F = unsafe extern "C" fn(*const c_void) -> c_uint;
        let (c, r) = p.sym::<F>(name);
        unsafe { eq(&format!("{name}(NULL)"), c(std::ptr::null()), r(std::ptr::null())) };
    }
}

/// M8/M9/M10 — version, level range, and stream-size constants.
#[test]
fn m8_m10_constants() {
    let p = libs();
    for name in ["ZSTD_versionNumber"] {
        let (c, r) = p.sym::<FnVoidUint>(name);
        unsafe { eq(name, c(), r()) };
    }
    {
        let (c, r) = p.sym::<FnVoidStr>("ZSTD_versionString");
        unsafe { eq("ZSTD_versionString", cstr(c()), cstr(r())) };
    }
    for name in ["ZSTD_minCLevel", "ZSTD_maxCLevel", "ZSTD_defaultCLevel"] {
        let (c, r) = p.sym::<FnVoidInt>(name);
        unsafe { eq(name, c(), r()) };
    }
    for name in [
        "ZSTD_CStreamInSize",
        "ZSTD_CStreamOutSize",
        "ZSTD_DStreamInSize",
        "ZSTD_DStreamOutSize",
        "ZBUFF_recommendedCInSize",
        "ZBUFF_recommendedCOutSize",
        "ZBUFF_recommendedDInSize",
        "ZBUFF_recommendedDOutSize",
    ] {
        let (c, r) = p.sym::<FnVoidSz>(name);
        unsafe { eq(name, c(), r()) };
    }
    for name in [
        "ZSTDv01_magicNumber",
        "ZSTDv02_magicNumber",
        "ZSTDv03_magicNumber",
        "ZSTDv04_magicNumber",
    ] {
        if p.has(name) {
            let (c, r) = p.sym::<FnVoidUint>(name);
            unsafe { eq(name, c(), r()) };
        }
    }
}

/// M7/D6 — `ZSTD_compressBound` / `ZSTD_sequenceBound` / `ZSTD_decompressBound`
/// over the full size axis including overflow territory.
#[test]
fn d6_m7_bound_functions() {
    let p = libs();
    let (c_cb, r_cb) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let mut sizes: Vec<usize> = SIZE_AXIS.to_vec();
    sizes.extend([
        1 << 20,
        1 << 24,
        (1usize << 31) - 1,
        1usize << 31,
        usize::MAX / 4,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ]);
    for s in &sizes {
        unsafe { eq(&format!("ZSTD_compressBound({s})"), c_cb(*s), r_cb(*s)) };
    }
    let (c_sb, r_sb) = p.sym::<FnCompressBound>("ZSTD_sequenceBound");
    for s in &sizes {
        unsafe { eq(&format!("ZSTD_sequenceBound({s})"), c_sb(*s), r_sb(*s)) };
    }
    // FSE_compressBound / HUF_compressBound
    for name in ["FSE_compressBound", "HUF_compressBound"] {
        let (c, r) = p.sym::<FnCompressBound>(name);
        for s in SIZE_AXIS.iter().chain([1usize << 20].iter()) {
            unsafe { eq(&format!("{name}({s})"), c(*s), r(*s)) };
        }
    }
}
