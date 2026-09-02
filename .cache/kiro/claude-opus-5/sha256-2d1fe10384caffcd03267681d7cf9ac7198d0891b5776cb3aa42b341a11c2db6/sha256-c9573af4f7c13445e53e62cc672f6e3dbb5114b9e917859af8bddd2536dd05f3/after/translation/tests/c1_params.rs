//! Phase B rows 11–16, 52–55 and Phase C rows 1–19, 24:
//! the parameter / bounds / cParams surface.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnCBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters;
type FnAdjust = unsafe extern "C" fn(
    ZSTD_compressionParameters, c_ulonglong, size_t,
) -> ZSTD_compressionParameters;
type FnCheck = unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t;
type FnSetCParams = unsafe extern "C" fn(*mut c_void, ZSTD_compressionParameters) -> size_t;
type FnSetFParams = unsafe extern "C" fn(*mut c_void, ZSTD_frameParameters) -> size_t;
type FnSetParams = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> size_t;
type FnCCtxParamsInitAdv = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> size_t;
type FnCCtxParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> size_t;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

struct Ctx {
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
}

fn new_cctx() -> Ctx {
    unsafe {
        let (a, b) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let x = a();
        let y = b();
        assert!(!x.is_null() && !y.is_null());
        Ctx { cctx_c: x, cctx_r: y }
    }
}
impl Drop for Ctx {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrToSize>("ZSTD_freeCCtx");
            a(self.cctx_c);
            b(self.cctx_r);
        }
    }
}

// ---------------------------------------------------------------- Phase B / C

/// CONFIGS row 11 + ERRORS rows 1–2: `cParam_getBounds` / `dParam_getBounds`
/// over every recognised id AND every out-of-range enum value.
#[test]
fn bounds_all_and_out_of_range_enums() {
    unsafe {
        let (cb, rb) = both::<FnCBounds>("ZSTD_cParam_getBounds");
        let (cd, rd) = both::<FnCBounds>("ZSTD_dParam_getBounds");
        for (name, id) in ALL_CPARAMS {
            assert_eq!(cb(*id), rb(*id), "ZSTD_cParam_getBounds({name}={id})");
        }
        for id in BAD_CPARAMS {
            let a = cb(*id);
            let b = rb(*id);
            assert_eq!(a, b, "ZSTD_cParam_getBounds(bad {id})");
            assert!(Err2::new().c.is_err(a.error), "bad cparam {id} should error");
        }
        for (name, id) in ALL_DPARAMS {
            assert_eq!(cd(*id), rd(*id), "ZSTD_dParam_getBounds({name}={id})");
        }
        for id in BAD_DPARAMS {
            let a = cd(*id);
            let b = rd(*id);
            assert_eq!(a, b, "ZSTD_dParam_getBounds(bad {id})");
            assert!(Err2::new().c.is_err(a.error), "bad dparam {id} should error");
        }
        // exhaustive sweep over a wide id range
        let mut rng = Rng::new(0x9111);
        for id in -600..2100i32 {
            assert_eq!(cb(id), rb(id), "cParam_getBounds sweep id={id}");
            assert_eq!(cd(id), rd(id), "dParam_getBounds sweep id={id}");
        }
        for _ in 0..2000 {
            let id = rng.next_u32() as c_int;
            assert_eq!(cb(id), rb(id), "cParam_getBounds random id={id}");
            assert_eq!(cd(id), rd(id), "dParam_getBounds random id={id}");
        }
    }
}

/// CONFIGS row 13 + ERRORS rows 3–6, 8: `CCtx_setParameter` / `getParameter`
/// value sweep including out-of-bound, clamped, and unknown params.
#[test]
fn cctx_set_get_parameter_full_sweep() {
    unsafe {
        let e = Err2::new();
        let (cbnd, _) = both::<FnCBounds>("ZSTD_cParam_getBounds");
        let (cs, rs_) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cg, rg) = both::<FnGetParam>("ZSTD_CCtx_getParameter");
        let mut rng = Rng::new(0x9112);
        let cx = new_cctx();
        for (name, id) in ALL_CPARAMS {
            let b = cbnd(*id);
            let vals = param_probe_values(b.lowerBound, b.upperBound, &mut rng);
            for v in vals {
                let a = cs(cx.cctx_c, *id, v);
                let bb = rs_(cx.cctx_r, *id, v);
                e.eq(&format!("CCtx_setParameter({name}, {v})"), a, bb);
                let mut o1: c_int = -12345;
                let mut o2: c_int = -12345;
                let x = cg(cx.cctx_c, *id, &mut o1);
                let y = rg(cx.cctx_r, *id, &mut o2);
                e.eq(&format!("CCtx_getParameter({name}) after set {v}"), x, y);
                assert_eq!(o1, o2, "CCtx_getParameter({name}) value after set {v}");
            }
        }
        // ERRORS row 3 / 8: unknown param ids
        for id in BAD_CPARAMS {
            let a = cs(cx.cctx_c, *id, 1);
            let b = rs_(cx.cctx_r, *id, 1);
            e.eq(&format!("CCtx_setParameter(bad {id})"), a, b);
            assert_eq!(e.c.classify(a), Ret::Err {
                code: E_parameter_unsupported,
                name: e.c.classify(a).err_name(),
            }, "bad param {id} must be parameter_unsupported (C said {:?})", e.c.classify(a));
            let mut o1: c_int = 0;
            let mut o2: c_int = 0;
            let x = cg(cx.cctx_c, *id, &mut o1);
            let y = rg(cx.cctx_r, *id, &mut o2);
            e.eq(&format!("CCtx_getParameter(bad {id})"), x, y);
            assert_eq!(o1, o2);
        }
    }
}

trait RetName {
    fn err_name(&self) -> String;
}
impl RetName for Ret {
    fn err_name(&self) -> String {
        match self {
            Ret::Err { name, .. } => name.clone(),
            Ret::Ok(_) => String::new(),
        }
    }
}

/// CONFIGS rows 14–15 + ERRORS rows 9–10: the standalone `ZSTD_CCtx_params`
/// object.
#[test]
fn cctxparams_full_sweep() {
    unsafe {
        let e = Err2::new();
        let (ccreate, rcreate) = both::<FnVoidToPtr>("ZSTD_createCCtxParams");
        let (cfree, rfree) = both::<FnPtrToSize>("ZSTD_freeCCtxParams");
        let (cbnd, _) = both::<FnCBounds>("ZSTD_cParam_getBounds");
        let (cs, rs_) = both::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (cg, rg) = both::<FnGetParam>("ZSTD_CCtxParams_getParameter");
        let (cinit, rinit) = both::<FnCCtxParamsInit>("ZSTD_CCtxParams_init");
        let (cinita, rinita) = both::<FnCCtxParamsInitAdv>("ZSTD_CCtxParams_init_advanced");
        let (crst, rrst) = both::<FnPtrToSize>("ZSTD_CCtxParams_reset");
        let (cgp, _) = both::<FnGetParams>("ZSTD_getParams");

        let p1 = ccreate();
        let p2 = rcreate();
        assert!(!p1.is_null() && !p2.is_null());
        let mut rng = Rng::new(0x9113);

        // CONFIGS row 15: init / init_advanced / reset
        for lvl in [-131072i32, -5, 0, 1, 3, 9, 19, 22, 23, 100] {
            e.eq(&format!("CCtxParams_init({lvl})"), cinit(p1, lvl), rinit(p2, lvl));
            let params = cgp(lvl.clamp(-131072, 22), 1 << 16, 0);
            e.eq(
                &format!("CCtxParams_init_advanced({lvl})"),
                cinita(p1, params),
                rinita(p2, params),
            );
            e.eq("CCtxParams_reset", crst(p1), rrst(p2));
        }
        // ERRORS row 10: init_advanced with invalid cParams
        for bad in bad_cparams_structs() {
            let params = ZSTD_parameters { cParams: bad, fParams: Default::default() };
            e.eq(
                &format!("CCtxParams_init_advanced(bad {bad:?})"),
                cinita(p1, params),
                rinita(p2, params),
            );
        }
        // full value sweep
        for (name, id) in ALL_CPARAMS {
            let b = cbnd(*id);
            for v in param_probe_values(b.lowerBound, b.upperBound, &mut rng) {
                e.eq(
                    &format!("CCtxParams_setParameter({name}, {v})"),
                    cs(p1, *id, v),
                    rs_(p2, *id, v),
                );
                let mut o1: c_int = -1;
                let mut o2: c_int = -1;
                e.eq(
                    &format!("CCtxParams_getParameter({name})"),
                    cg(p1, *id, &mut o1),
                    rg(p2, *id, &mut o2),
                );
                assert_eq!(o1, o2, "CCtxParams_getParameter({name}) after set {v}");
            }
        }
        for id in BAD_CPARAMS {
            e.eq(&format!("CCtxParams_setParameter(bad {id})"), cs(p1, *id, 1), rs_(p2, *id, 1));
            let mut o1: c_int = 0;
            let mut o2: c_int = 0;
            e.eq(
                &format!("CCtxParams_getParameter(bad {id})"),
                cg(p1, *id, &mut o1),
                rg(p2, *id, &mut o2),
            );
        }
        cfree(p1);
        rfree(p2);
    }
}

fn bad_cparams_structs() -> Vec<ZSTD_compressionParameters> {
    let base = ZSTD_compressionParameters {
        windowLog: 20,
        chainLog: 16,
        hashLog: 17,
        searchLog: 1,
        minMatch: 5,
        targetLength: 0,
        strategy: 1,
    };
    let mut v = Vec::new();
    // one step outside each bound
    for (i, (lo, hi)) in [(10u32, 31u32), (6, 30), (6, 30), (1, 30), (3, 7), (0, 131072), (1, 9)]
        .iter()
        .enumerate()
    {
        for bad in [lo.wrapping_sub(1), hi + 1, u32::MAX, 0] {
            let mut c = base;
            match i {
                0 => c.windowLog = bad,
                1 => c.chainLog = bad,
                2 => c.hashLog = bad,
                3 => c.searchLog = bad,
                4 => c.minMatch = bad,
                5 => c.targetLength = bad,
                _ => c.strategy = bad,
            }
            v.push(c);
        }
    }
    v
}

/// CONFIGS rows 52–54 + ERRORS row 15: `getCParams` / `getParams` /
/// `adjustCParams` / `checkCParams`.
#[test]
fn cparams_derivation_and_check() {
    unsafe {
        let e = Err2::new();
        let (cgc, rgc) = both::<FnGetCParams>("ZSTD_getCParams");
        let (cgp, rgp) = both::<FnGetParams>("ZSTD_getParams");
        let (cadj, radj) = both::<FnAdjust>("ZSTD_adjustCParams");
        let (cchk, rchk) = both::<FnCheck>("ZSTD_checkCParams");
        let (minl, _) = both::<FnVoidToInt>("ZSTD_minCLevel");
        let lo = minl();

        let sizes: &[c_ulonglong] = &[
            0,
            ZSTD_CONTENTSIZE_UNKNOWN,
            1,
            2,
            255,
            256,
            1 << 10,
            (1 << 10) + 1,
            1 << 16,
            1 << 17,
            (1 << 17) + 1,
            1 << 20,
            1 << 27,
            1 << 30,
            1u64 << 40,
            u64::MAX,
        ];
        let dicts: &[size_t] = &[0, 1, 100, 1 << 10, 1 << 16, 1 << 20, 112640];
        let mut levels: Vec<c_int> = (-8..=22).collect();
        levels.extend([lo, lo + 1, 23, 100, -131072, -131073]);

        let mut collected = Vec::new();
        for &lvl in &levels {
            for &s in sizes {
                for &d in dicts {
                    let a = cgc(lvl, s, d);
                    let b = rgc(lvl, s, d);
                    assert_eq!(a, b, "ZSTD_getCParams({lvl}, {s}, {d})");
                    let pa = cgp(lvl, s, d);
                    let pb = rgp(lvl, s, d);
                    assert_eq!(pa, pb, "ZSTD_getParams({lvl}, {s}, {d})");
                    collected.push(a);
                }
            }
        }
        // CONFIGS row 53: adjustCParams over the derived structs
        for c in collected.iter().take(400) {
            for &s in sizes {
                for &d in &[0usize, 1 << 10, 1 << 20] {
                    assert_eq!(cadj(*c, s, d), radj(*c, s, d), "adjustCParams({c:?},{s},{d})");
                }
            }
        }
        // CONFIGS row 54 + ERRORS row 15
        for c in collected.iter() {
            e.eq(&format!("checkCParams({c:?})"), cchk(*c), rchk(*c));
        }
        let mut rejected = 0usize;
        for c in bad_cparams_structs() {
            let a = cchk(c);
            let b = rchk(c);
            e.eq(&format!("checkCParams(bad {c:?})"), a, b);
            if e.c.is_err(a) {
                rejected += 1;
                assert_eq!(
                    e.c.classify(a),
                    Ret::Err { code: E_parameter_outOfBound, name: e.c.classify(a).err_name2() },
                    "out-of-bound cParams must give parameter_outOfBound: {c:?}"
                );
            }
            // adjustCParams / getCParams on bad structs must also match
            assert_eq!(cadj(c, 1 << 16, 0), radj(c, 1 << 16, 0), "adjustCParams(bad {c:?})");
        }
        assert!(rejected >= 20, "expected many out-of-bound rejections, got {rejected}");
        // fully random cParams structs
        let mut rng = Rng::new(0x9114);
        for _ in 0..4000 {
            let c = ZSTD_compressionParameters {
                windowLog: rng.range(0, 40) as c_uint,
                chainLog: rng.range(0, 40) as c_uint,
                hashLog: rng.range(0, 40) as c_uint,
                searchLog: rng.range(0, 40) as c_uint,
                minMatch: rng.range(0, 12) as c_uint,
                targetLength: rng.range(0, 200_000) as c_uint,
                strategy: rng.range(0, 12) as c_uint,
            };
            e.eq(&format!("checkCParams(rand {c:?})"), cchk(c), rchk(c));
            assert_eq!(cadj(c, 1 << 20, 1 << 10), radj(c, 1 << 20, 1 << 10),
                       "adjustCParams(rand {c:?})");
        }
    }
}

/// CONFIGS row 55 + ERRORS row 16: `CCtx_setCParams` / `setFParams` / `setParams`.
#[test]
fn cctx_set_cparams_fparams_params() {
    unsafe {
        let e = Err2::new();
        let (csc, rsc) = both::<FnSetCParams>("ZSTD_CCtx_setCParams");
        let (csf, rsf) = both::<FnSetFParams>("ZSTD_CCtx_setFParams");
        let (csp, rsp) = both::<FnSetParams>("ZSTD_CCtx_setParams");
        let (cgp, _) = both::<FnGetParams>("ZSTD_getParams");
        let cx = new_cctx();
        for lvl in [-5i32, 1, 3, 9, 19, 22] {
            let p = cgp(lvl, 1 << 16, 0);
            e.eq(&format!("setCParams(lvl {lvl})"), csc(cx.cctx_c, p.cParams), rsc(cx.cctx_r, p.cParams));
            e.eq(&format!("setFParams(lvl {lvl})"), csf(cx.cctx_c, p.fParams), rsf(cx.cctx_r, p.fParams));
            e.eq(&format!("setParams(lvl {lvl})"), csp(cx.cctx_c, p), rsp(cx.cctx_r, p));
        }
        for bad in bad_cparams_structs() {
            e.eq(&format!("setCParams(bad {bad:?})"), csc(cx.cctx_c, bad), rsc(cx.cctx_r, bad));
            let p = ZSTD_parameters { cParams: bad, fParams: Default::default() };
            e.eq(&format!("setParams(bad {bad:?})"), csp(cx.cctx_c, p), rsp(cx.cctx_r, p));
        }
        // fParams accepts any int; sweep out-of-range values
        for a in [-1i32, 0, 1, 2, i32::MIN, i32::MAX] {
            for b in [-1i32, 0, 1, i32::MAX] {
                for c in [-1i32, 0, 1, i32::MIN] {
                    let f = ZSTD_frameParameters { contentSizeFlag: a, checksumFlag: b, noDictIDFlag: c };
                    e.eq(&format!("setFParams({f:?})"), csf(cx.cctx_c, f), rsf(cx.cctx_r, f));
                }
            }
        }
    }
}

/// ERRORS rows 7, 17, 19, 24: stage errors and reset directives.
#[test]
fn stage_and_reset_directives() {
    unsafe {
        let e = Err2::new();
        let (cs, rs_) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cplg, rplg) = both::<FnSetPledged>("ZSTD_CCtx_setPledgedSrcSize");
        let (cdrst, rdrst) = both::<FnReset>("ZSTD_DCtx_reset");
        let (cdc, rdc) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");

        let cx = new_cctx();
        let d1 = cdc();
        let d2 = rdc();

        // ERRORS rows 17-18: bad reset directives on a fresh context
        for r in [0i32, 4, 5, -1, 100, i32::MIN, i32::MAX] {
            e.eq(&format!("CCtx_reset(bad {r})"), crst(cx.cctx_c, r), rrst(cx.cctx_r, r));
            e.eq(&format!("DCtx_reset(bad {r})"), cdrst(d1, r), rdrst(d2, r));
        }
        for r in [1i32, 2, 3] {
            e.eq(&format!("CCtx_reset({r})"), crst(cx.cctx_c, r), rrst(cx.cctx_r, r));
            e.eq(&format!("DCtx_reset({r})"), cdrst(d1, r), rdrst(d2, r));
        }

        // Drive the CCtx mid-frame via streaming, then retry the setters.
        type FnCStream2 = unsafe extern "C" fn(
            *mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int,
        ) -> size_t;
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let mut rng = Rng::new(0x9115);
        let src = gen(Shape::Text, 40_000, &mut rng);
        let mut o1 = vec![0u8; 4096];
        let mut o2 = vec![0u8; 4096];
        let mut ib1 = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
        let mut ib2 = ib1;
        let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
        let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
        let a = ccs(cx.cctx_c, &mut ob1, &mut ib1, ZSTD_e_continue);
        let b = rcs(cx.cctx_r, &mut ob2, &mut ib2, ZSTD_e_continue);
        e.eq("compressStream2 warm-up", a, b);
        assert_bytes_eq("warm-up output", &o1[..ob1.pos], &o2[..ob2.pos]);
        assert_eq!(ib1.pos, ib2.pos);

        // ERRORS row 7: setParameter mid-frame
        for (name, id) in ALL_CPARAMS {
            e.eq(
                &format!("CCtx_setParameter({name}) mid-frame"),
                cs(cx.cctx_c, *id, 1),
                rs_(cx.cctx_r, *id, 1),
            );
        }
        // ERRORS row 24: setPledgedSrcSize mid-frame
        e.eq("setPledgedSrcSize mid-frame", cplg(cx.cctx_c, 100), rplg(cx.cctx_r, 100));
        // ERRORS row 19: reset_parameters mid-frame
        e.eq(
            "CCtx_reset(reset_parameters) mid-frame",
            crst(cx.cctx_c, ZSTD_reset_parameters),
            rrst(cx.cctx_r, ZSTD_reset_parameters),
        );
        // session_only mid-frame is legal
        e.eq(
            "CCtx_reset(session_only) mid-frame",
            crst(cx.cctx_c, ZSTD_reset_session_only),
            rrst(cx.cctx_r, ZSTD_reset_session_only),
        );
        cdf(d1);
        rdf(d2);
    }
}

/// ERRORS rows 11–14: DCtx parameters, including mid-stream stage errors.
#[test]
fn dctx_parameter_sweep_and_stage() {
    unsafe {
        let e = Err2::new();
        let (cdc, rdc) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        let (cbnd, _) = both::<FnCBounds>("ZSTD_dParam_getBounds");
        let (cs, rs_) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (cg, rg) = both::<FnGetParam>("ZSTD_DCtx_getParameter");
        let (cmw, rmw) = both::<unsafe extern "C" fn(*mut c_void, size_t) -> size_t>("ZSTD_DCtx_setMaxWindowSize");
        let d1 = cdc();
        let d2 = rdc();
        let mut rng = Rng::new(0x9116);
        for (name, id) in ALL_DPARAMS {
            let b = cbnd(*id);
            for v in param_probe_values(b.lowerBound, b.upperBound, &mut rng) {
                e.eq(
                    &format!("DCtx_setParameter({name}, {v})"),
                    cs(d1, *id, v),
                    rs_(d2, *id, v),
                );
                let mut o1: c_int = -1;
                let mut o2: c_int = -1;
                e.eq(
                    &format!("DCtx_getParameter({name})"),
                    cg(d1, *id, &mut o1),
                    rg(d2, *id, &mut o2),
                );
                assert_eq!(o1, o2, "DCtx_getParameter({name}) after set {v}");
            }
        }
        for id in BAD_DPARAMS {
            e.eq(&format!("DCtx_setParameter(bad {id})"), cs(d1, *id, 1), rs_(d2, *id, 1));
            let mut o1: c_int = 0;
            let mut o2: c_int = 0;
            e.eq(&format!("DCtx_getParameter(bad {id})"), cg(d1, *id, &mut o1), rg(d2, *id, &mut o2));
        }
        // setMaxWindowSize sweep
        for w in [0usize, 1, 1 << 9, 1 << 10, 1 << 27, 1usize << 31, usize::MAX, usize::MAX / 2] {
            e.eq(&format!("DCtx_setMaxWindowSize({w})"), cmw(d1, w), rmw(d2, w));
        }

        // ERRORS row 13: mid-frame stage error
        type FnDStream = unsafe extern "C" fn(
            *mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer,
        ) -> size_t;
        let (cds, rds) = both::<FnDStream>("ZSTD_decompressStream");
        let (cc, _) = both::<unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t>("ZSTD_compress");
        let (cbnd2, _) = both::<FnCompressBoundT>("ZSTD_compressBound");
        let src = gen(Shape::Text, 60_000, &mut rng);
        let mut frame = vec![0u8; cbnd2(src.len()) + 64];
        let n = cc(frame.as_mut_ptr() as *mut c_void, frame.len(),
                   src.as_ptr() as *const c_void, src.len(), 3);
        assert!(!e.c.is_err(n));
        frame.truncate(n);
        // reset both contexts, then feed only part of the frame
        let (crst, rrst) = both::<FnReset>("ZSTD_DCtx_reset");
        crst(d1, ZSTD_reset_session_and_parameters);
        rrst(d2, ZSTD_reset_session_and_parameters);
        let mut o1 = vec![0u8; 1024];
        let mut o2 = vec![0u8; 1024];
        let half = frame.len() / 2;
        let mut ib1 = ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: half, pos: 0 };
        let mut ib2 = ib1;
        let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
        let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
        e.eq("decompressStream warm-up", cds(d1, &mut ob1, &mut ib1), rds(d2, &mut ob2, &mut ib2));
        assert_bytes_eq("warm-up out", &o1[..ob1.pos], &o2[..ob2.pos]);
        for (name, id) in ALL_DPARAMS {
            e.eq(
                &format!("DCtx_setParameter({name}) mid-frame"),
                cs(d1, *id, 1),
                rs_(d2, *id, 1),
            );
        }
        e.eq("DCtx_setMaxWindowSize mid-frame", cmw(d1, 1 << 20), rmw(d2, 1 << 20));
        cdf(d1);
        rdf(d2);
    }
}

type FnCompressBoundT = unsafe extern "C" fn(size_t) -> size_t;

/// ERRORS row 23 + CONFIGS row 32: pledged src size mismatch through
/// `ZSTD_compress2`.
#[test]
fn pledged_src_size() {
    unsafe {
        let e = Err2::new();
        let (cplg, rplg) = both::<FnSetPledged>("ZSTD_CCtx_setPledgedSrcSize");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cb, _) = both::<FnCompressBoundT>("ZSTD_compressBound");
        let cx = new_cctx();
        let mut rng = Rng::new(0x9117);
        for &len in &[0usize, 1, 100, 5000, 70_000] {
            let src = gen(Shape::Text, len, &mut rng);
            for pledged in [
                src.len() as c_ulonglong,
                ZSTD_CONTENTSIZE_UNKNOWN,
                0,
                1,
                (src.len() as c_ulonglong).wrapping_add(1),
                (src.len() as c_ulonglong).wrapping_sub(1),
                u64::MAX - 1,
                1 << 40,
            ] {
                crst(cx.cctx_c, ZSTD_reset_session_and_parameters);
                rrst(cx.cctx_r, ZSTD_reset_session_and_parameters);
                let a = cplg(cx.cctx_c, pledged);
                let b = rplg(cx.cctx_r, pledged);
                e.eq(&format!("setPledgedSrcSize({pledged})"), a, b);
                if e.c.is_err(a) {
                    continue;
                }
                let cap = cb(src.len()) + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let x = cc2(cx.cctx_c, o1.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len());
                let y = rc2(cx.cctx_r, o2.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len());
                let ctx = format!("compress2 len={len} pledged={pledged}");
                e.eq(&ctx, x, y);
                if !e.c.is_err(x) {
                    assert_bytes_eq(&ctx, &o1[..x], &o2[..y]);
                }
            }
        }
    }
}

trait RetName2 { fn err_name2(&self) -> String; }
impl RetName2 for Ret {
    fn err_name2(&self) -> String {
        match self { Ret::Err { name, .. } => name.clone(), Ret::Ok(_) => String::new() }
    }
}
