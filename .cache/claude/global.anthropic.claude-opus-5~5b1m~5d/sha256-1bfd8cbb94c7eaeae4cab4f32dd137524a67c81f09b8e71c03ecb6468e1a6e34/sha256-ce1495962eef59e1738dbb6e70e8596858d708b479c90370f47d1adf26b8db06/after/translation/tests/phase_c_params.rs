//! Phase C — ERRORS.md rows covered by `phase_c_params`:
//! `parameter_outOfBound`, `parameter_unsupported`,
//! `parameter_combination_unsupported`, plus every out-of-range enum value that
//! crosses the FFI boundary (C enums accept any `int`).
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        assert_eq!(
            c,
            r,
            "{what}: C={c:#x} (code {} = {}), Rust={r:#x} (code {} = {})",
            gcc(c),
            cstr(nc(c)),
            gcr(r),
            cstr(nr(r))
        );
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: error name mismatch");
    }
}

/// Every "interesting" int for a parameter with bounds [lo, hi].
fn probe_values(lo: c_int, hi: c_int) -> Vec<c_int> {
    let mut v = vec![
        lo,
        hi,
        lo.saturating_sub(1),
        hi.saturating_add(1),
        lo.saturating_sub(2),
        hi.saturating_add(2),
        0,
        1,
        -1,
        2,
        -2,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    if hi > lo {
        v.push(lo + (hi - lo) / 2);
    }
    v.sort();
    v.dedup();
    v
}

// ------------------------------------------------------------------ CCtx params

#[test]
fn err_cctx_setparameter_out_of_bound() {
    unsafe {
        let (gb, _) = duo::<FnGetBounds>("ZSTD_cParam_getBounds");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (gpc, gpr) = duo::<FnGetParam>("ZSTD_CCtx_getParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let cctx = CtxPair::cctx();
        for (name, p) in ALL_CPARAMS {
            let b = gb(*p);
            assert!(!is_err(b.error), "{name}: no bounds");
            for v in probe_values(b.lowerBound, b.upperBound) {
                eqcode(
                    "reset",
                    rc(cctx.c, ZSTD_reset_session_and_parameters),
                    rr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(
                    &format!("CCtx_setParameter({name}={v})"),
                    spc(cctx.c, *p, v),
                    spr(cctx.r, *p, v),
                );
                let mut xc: c_int = -0x5A5A;
                let mut xr: c_int = -0x5A5A;
                eqcode(
                    &format!("CCtx_getParameter({name}) after {v}"),
                    gpc(cctx.c, *p, &mut xc),
                    gpr(cctx.r, *p, &mut xr),
                );
                eqv(&format!("CCtx_getParameter({name}) value after {v}"), xc, xr);
            }
        }
        // parameters that do not exist at all
        let mut rng = Rng::new(0xD001);
        let mut bogus: Vec<c_int> = vec![
            -1, 0, 1, 2, 9, 11, 12, 99, 108, 109, 129, 131, 159, 165, 166, 199, 203, 300, 399, 403,
            499, 501, 999, 1003, 1018, 1019, 1020, 5000, i32::MIN, i32::MAX,
        ];
        for _ in 0..2000 {
            bogus.push(rng.next_u32() as c_int);
        }
        for p in bogus {
            if ALL_CPARAMS.iter().any(|(_, q)| *q == p) {
                continue;
            }
            let bb = gb(p);
            eqv(&format!("cParam_getBounds({p})"), bb, gb(p));
            for v in [0, 1, -1, i32::MAX, i32::MIN] {
                eqcode(
                    "reset",
                    rc(cctx.c, ZSTD_reset_session_and_parameters),
                    rr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(
                    &format!("CCtx_setParameter(bogus {p}, {v})"),
                    spc(cctx.c, p, v),
                    spr(cctx.r, p, v),
                );
                let mut xc: c_int = 7;
                let mut xr: c_int = 7;
                eqcode(
                    &format!("CCtx_getParameter(bogus {p})"),
                    gpc(cctx.c, p, &mut xc),
                    gpr(cctx.r, p, &mut xr),
                );
                eqv(&format!("CCtx_getParameter(bogus {p}) value"), xc, xr);
            }
        }
    }
}

#[test]
fn err_cctxparams_setparameter_out_of_bound() {
    unsafe {
        let (gb, _) = duo::<FnGetBounds>("ZSTD_cParam_getBounds");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (gpc, gpr) = duo::<FnGetParam>("ZSTD_CCtxParams_getParameter");
        let (rc, rr) = duo::<FnFreePtr>("ZSTD_CCtxParams_reset");
        let p_ = CtxPair::cctx_params();
        for (name, p) in ALL_CPARAMS {
            let b = gb(*p);
            for v in probe_values(b.lowerBound, b.upperBound) {
                eqcode("CCtxParams_reset", rc(p_.c), rr(p_.r));
                eqcode(
                    &format!("CCtxParams_setParameter({name}={v})"),
                    spc(p_.c, *p, v),
                    spr(p_.r, *p, v),
                );
                let mut xc: c_int = -0x5A5A;
                let mut xr: c_int = -0x5A5A;
                eqcode(
                    &format!("CCtxParams_getParameter({name})"),
                    gpc(p_.c, *p, &mut xc),
                    gpr(p_.r, *p, &mut xr),
                );
                eqv(&format!("CCtxParams_getParameter({name}) value"), xc, xr);
            }
        }
        let mut rng = Rng::new(0xD002);
        for _ in 0..1500 {
            let p = rng.next_u32() as c_int;
            if ALL_CPARAMS.iter().any(|(_, q)| *q == p) {
                continue;
            }
            eqcode(
                &format!("CCtxParams_setParameter(bogus {p})"),
                spc(p_.c, p, 1),
                spr(p_.r, p, 1),
            );
            let mut xc: c_int = 3;
            let mut xr: c_int = 3;
            eqcode(
                &format!("CCtxParams_getParameter(bogus {p})"),
                gpc(p_.c, p, &mut xc),
                gpr(p_.r, p, &mut xr),
            );
            eqv(&format!("CCtxParams_getParameter(bogus {p}) value"), xc, xr);
        }
        // NOTE: `ZSTD_CCtxParams_setParameter` / `_getParameter` have NO NULL
        // check in the C (zstd_compress.c:770 dereferences `CCtxParams`
        // immediately, unlike `ZSTD_CCtxParams_init` which does
        // `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!")`), so passing
        // NULL segfaults the C reference. Not differentiable; excluded.
        // `ZSTD_CCtxParams_init(NULL, lvl)` *is* checked, so that one is tested:
        {
            let (ic, ir) =
                duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
            for lvl in [-131072, -1, 0, 3, 22, 23, i32::MAX] {
                eqcode(
                    &format!("CCtxParams_init(NULL,{lvl})"),
                    ic(std::ptr::null_mut(), lvl),
                    ir(std::ptr::null_mut(), lvl),
                );
            }
            let (iac, iar) = duo::<unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> usize>(
                "ZSTD_CCtxParams_init_advanced",
            );
            eqcode(
                "CCtxParams_init_advanced(NULL)",
                iac(std::ptr::null_mut(), ZSTD_parameters::default()),
                iar(std::ptr::null_mut(), ZSTD_parameters::default()),
            );
        }
    }
}

// ------------------------------------------------------------------ DCtx params

#[test]
fn err_dctx_setparameter_out_of_bound() {
    unsafe {
        let (gb, _) = duo::<FnGetBounds>("ZSTD_dParam_getBounds");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (gpc, gpr) = duo::<FnGetParam>("ZSTD_DCtx_getParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_DCtx_reset");
        let dctx = CtxPair::dctx();
        for (name, p) in ALL_DPARAMS {
            let b = gb(*p);
            assert!(!is_err(b.error), "{name}: no bounds");
            for v in probe_values(b.lowerBound, b.upperBound) {
                eqcode(
                    "DCtx_reset",
                    rc(dctx.c, ZSTD_reset_session_and_parameters),
                    rr(dctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(
                    &format!("DCtx_setParameter({name}={v})"),
                    spc(dctx.c, *p, v),
                    spr(dctx.r, *p, v),
                );
                let mut xc: c_int = -0x5A5A;
                let mut xr: c_int = -0x5A5A;
                eqcode(
                    &format!("DCtx_getParameter({name})"),
                    gpc(dctx.c, *p, &mut xc),
                    gpr(dctx.r, *p, &mut xr),
                );
                eqv(&format!("DCtx_getParameter({name}) value"), xc, xr);
            }
        }
        let mut rng = Rng::new(0xD003);
        let mut bogus: Vec<c_int> = vec![
            -1, 0, 1, 99, 101, 102, 999, 1006, 1007, 2000, i32::MIN, i32::MAX,
        ];
        for _ in 0..2000 {
            bogus.push(rng.next_u32() as c_int);
        }
        for p in bogus {
            if ALL_DPARAMS.iter().any(|(_, q)| *q == p) {
                continue;
            }
            eqv(&format!("dParam_getBounds({p})"), gb(p), gb(p));
            for v in [0, 1, -1, i32::MAX] {
                eqcode(
                    &format!("DCtx_setParameter(bogus {p},{v})"),
                    spc(dctx.c, p, v),
                    spr(dctx.r, p, v),
                );
                let mut xc: c_int = 5;
                let mut xr: c_int = 5;
                eqcode(
                    &format!("DCtx_getParameter(bogus {p})"),
                    gpc(dctx.c, p, &mut xc),
                    gpr(dctx.r, p, &mut xr),
                );
                eqv(&format!("DCtx_getParameter(bogus {p}) value"), xc, xr);
            }
        }
        // ZSTD_DCtx_setFormat with an out-of-range ZSTD_format_e
        let (sfc, sfr) =
            duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_DCtx_setFormat");
        for f in [-2, -1, 0, 1, 2, 3, 99, i32::MIN, i32::MAX] {
            let d = CtxPair::dctx();
            eqcode(
                &format!("DCtx_setFormat({f})"),
                sfc(d.c, f),
                sfr(d.r, f),
            );
        }
    }
}

// ------------------------------------------------------------------ reset directives

#[test]
fn err_reset_directive_out_of_range() {
    unsafe {
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (drc, drr) = duo::<FnReset>("ZSTD_DCtx_reset");
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let mut rng = Rng::new(0xD004);
        let mut vals: Vec<c_int> = vec![-2, -1, 0, 1, 2, 3, 4, 5, 100, i32::MIN, i32::MAX];
        for _ in 0..500 {
            vals.push(rng.next_u32() as c_int);
        }
        for v in vals {
            eqcode(&format!("CCtx_reset({v})"), rc(cctx.c, v), rr(cctx.r, v));
            eqcode(&format!("DCtx_reset({v})"), drc(dctx.c, v), drr(dctx.r, v));
        }
    }
}

// ------------------------------------------------------------------ parameter combinations

#[test]
fn err_parameter_combinations() {
    unsafe {
        let cctx = CtxPair::cctx();
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (plc, plr) = duo::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize>(
            "ZSTD_CCtx_setPledgedSrcSize",
        );

        // hashLog/chainLog/searchLog/windowLog combinations that
        // ZSTD_checkCParams rejects, driven through the real API
        let combos: Vec<Vec<(c_int, c_int)>> = vec![
            vec![(ZSTD_c_windowLog, 10), (ZSTD_c_hashLog, 30)],
            vec![(ZSTD_c_windowLog, 10), (ZSTD_c_chainLog, 30)],
            vec![(ZSTD_c_strategy, 1), (ZSTD_c_chainLog, 30)],
            vec![(ZSTD_c_strategy, 9), (ZSTD_c_targetLength, 0)],
            vec![(ZSTD_c_minMatch, 3), (ZSTD_c_strategy, 1)],
            vec![(ZSTD_c_minMatch, 7), (ZSTD_c_strategy, 9)],
            // targetCBlockSize vs maxBlockSize
            vec![(ZSTD_c_targetCBlockSize, 131072), (ZSTD_c_maxBlockSize, 1024)],
            vec![(ZSTD_c_maxBlockSize, 1024), (ZSTD_c_targetCBlockSize, 131072)],
            // LDM with a tiny window
            vec![
                (ZSTD_c_enableLongDistanceMatching, 1),
                (ZSTD_c_windowLog, 10),
                (ZSTD_c_ldmHashLog, 6),
                (ZSTD_c_ldmMinMatch, 4),
                (ZSTD_c_ldmBucketSizeLog, 8),
                (ZSTD_c_ldmHashRateLog, 0),
            ],
            // stableInBuffer/stableOutBuffer together
            vec![(ZSTD_c_stableInBuffer, 1), (ZSTD_c_stableOutBuffer, 1)],
            // nbWorkers in a single-threaded build combined with rsyncable/jobSize
            vec![(ZSTD_c_nbWorkers, 1), (ZSTD_c_rsyncable, 1)],
            vec![(ZSTD_c_nbWorkers, 4), (ZSTD_c_jobSize, 1 << 20)],
            // external sequences + LDM
            vec![
                (ZSTD_c_enableLongDistanceMatching, 1),
                (ZSTD_c_ldmMinMatch, 64),
                (ZSTD_c_blockDelimiters, 1),
                (ZSTD_c_validateSequences, 1),
            ],
            // block splitter + super block
            vec![(ZSTD_c_blockSplitterLevel, 6), (ZSTD_c_targetCBlockSize, 1340)],
            // dedicated dict search without a dict
            vec![(ZSTD_c_enableDedicatedDictSearch, 1), (ZSTD_c_strategy, 3)],
        ];
        let mut rng = Rng::new(0xD005);
        for (ci, combo) in combos.iter().enumerate() {
            for &sz in &[0usize, 1, 1000, 70_000, 300_000] {
                eqcode(
                    "reset",
                    rc(cctx.c, ZSTD_reset_session_and_parameters),
                    rr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                for (p, v) in combo {
                    eqcode(
                        &format!("combo#{ci} set({p},{v})"),
                        spc(cctx.c, *p, *v),
                        spr(cctx.r, *p, *v),
                    );
                }
                for pledged in [ZSTD_CONTENTSIZE_UNKNOWN, sz as c_ulonglong, 1] {
                    eqcode(
                        &format!("combo#{ci} pledged={pledged}"),
                        plc(cctx.c, pledged),
                        plr(cctx.r, pledged),
                    );
                    let src = gen_class(rng.below(N_CLASSES), sz, ci as u64);
                    let cap = bd(sz) + 64;
                    let mut oc = vec![0x81u8; cap];
                    let mut or_ = vec![0x81u8; cap];
                    let a = c2c(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let b = c2r(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    eqcode(
                        &format!("combo#{ci} sz={sz} pledged={pledged} compress2"),
                        a,
                        b,
                    );
                    eqbuf(
                        &format!("combo#{ci} sz={sz} pledged={pledged} dst"),
                        &oc,
                        &or_,
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------------ enum values across FFI

#[test]
fn err_out_of_range_enums() {
    unsafe {
        let mut rng = Rng::new(0xD006);
        let bad: Vec<c_int> = {
            let mut v = vec![-3, -2, -1, 3, 4, 5, 16, 99, 1000, i32::MIN, i32::MAX];
            for _ in 0..200 {
                v.push(rng.next_u32() as c_int);
            }
            v
        };

        // ZSTD_EndDirective
        {
            let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
            let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
            let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
            let cctx = CtxPair::cctx();
            let src = gen_class(4, 5000, 1);
            for &op in bad.iter().chain([0, 1, 2].iter()) {
                eqcode(
                    "reset",
                    rc(cctx.c, ZSTD_reset_session_and_parameters),
                    rr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(
                    "set level",
                    spc(cctx.c, ZSTD_c_compressionLevel, 3),
                    spr(cctx.r, ZSTD_c_compressionLevel, 3),
                );
                let mut oc = vec![0u8; 8192];
                let mut or_ = vec![0u8; 8192];
                let mut ic = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: src.len(),
                    pos: 0,
                };
                let mut ir = ic;
                let mut obc = ZSTD_outBuffer {
                    dst: oc.as_mut_ptr() as *mut c_void,
                    size: oc.len(),
                    pos: 0,
                };
                let mut obr = ZSTD_outBuffer {
                    dst: or_.as_mut_ptr() as *mut c_void,
                    size: or_.len(),
                    pos: 0,
                };
                let a = s2c(cctx.c, &mut obc, &mut ic, op);
                let b = s2r(cctx.r, &mut obr, &mut ir, op);
                eqcode(&format!("compressStream2(endOp={op})"), a, b);
                eqv(&format!("compressStream2(endOp={op}) in.pos"), ic.pos, ir.pos);
                eqv(&format!("compressStream2(endOp={op}) out.pos"), obc.pos, obr.pos);
                eqbuf(&format!("compressStream2(endOp={op}) dst"), &oc, &or_);
            }
        }

        // ZSTD_dictLoadMethod_e / ZSTD_dictContentType_e on every advanced ctor
        {
            let dict = gen_class(4, 4096, 2);
            let dp = dict.as_ptr() as *const c_void;
            let ds = dict.len();
            let (gc, _) =
                duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                    "ZSTD_getCParams",
                );
            let cp = gc(3, 0, ds);
            let (ccc, ccr) = duo::<
                unsafe extern "C" fn(
                    *const c_void,
                    usize,
                    c_int,
                    c_int,
                    ZSTD_compressionParameters,
                    ZSTD_customMem,
                ) -> *mut c_void,
            >("ZSTD_createCDict_advanced");
            let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCDict");
            let (ddc, ddr) = duo::<
                unsafe extern "C" fn(*const c_void, usize, c_int, c_int, ZSTD_customMem) -> *mut c_void,
            >("ZSTD_createDDict_advanced");
            let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
            let m = ZSTD_customMem::default();
            for &dlm in bad.iter().chain([0, 1].iter()) {
                for &dct in bad.iter().chain([0, 1, 2].iter()) {
                    let a = ccc(dp, ds, dlm, dct, cp, m);
                    let b = ccr(dp, ds, dlm, dct, cp, m);
                    eqv(
                        &format!("createCDict_advanced(dlm={dlm},dct={dct}) null?"),
                        a.is_null(),
                        b.is_null(),
                    );
                    if !a.is_null() {
                        let (sc, sr) =
                            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_CDict");
                        eqv(
                            &format!("sizeof_CDict(dlm={dlm},dct={dct})"),
                            sc(a),
                            sr(b),
                        );
                        fcc(a);
                        fcr(b);
                    }
                    let a = ddc(dp, ds, dlm, dct, m);
                    let b = ddr(dp, ds, dlm, dct, m);
                    eqv(
                        &format!("createDDict_advanced(dlm={dlm},dct={dct}) null?"),
                        a.is_null(),
                        b.is_null(),
                    );
                    if !a.is_null() {
                        let (sc, sr) =
                            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_DDict");
                        eqv(
                            &format!("sizeof_DDict(dlm={dlm},dct={dct})"),
                            sc(a),
                            sr(b),
                        );
                        fdc(a);
                        fdr(b);
                    }
                }
            }
            // ZSTD_CCtx_loadDictionary_advanced / ZSTD_DCtx_loadDictionary_advanced
            let (clc, clr) = duo::<
                unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int, c_int) -> usize,
            >("ZSTD_CCtx_loadDictionary_advanced");
            let (dlc, dlr) = duo::<
                unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int, c_int) -> usize,
            >("ZSTD_DCtx_loadDictionary_advanced");
            let (rpc, rpr) = duo::<
                unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize,
            >("ZSTD_CCtx_refPrefix_advanced");
            let (dpc, dpr) = duo::<
                unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize,
            >("ZSTD_DCtx_refPrefix_advanced");
            for &dlm in bad.iter().chain([0, 1].iter()) {
                for &dct in bad.iter().chain([0, 1, 2].iter()) {
                    let cctx = CtxPair::cctx();
                    let dctx = CtxPair::dctx();
                    eqcode(
                        &format!("CCtx_loadDictionary_advanced(dlm={dlm},dct={dct})"),
                        clc(cctx.c, dp, ds, dlm, dct),
                        clr(cctx.r, dp, ds, dlm, dct),
                    );
                    eqcode(
                        &format!("DCtx_loadDictionary_advanced(dlm={dlm},dct={dct})"),
                        dlc(dctx.c, dp, ds, dlm, dct),
                        dlr(dctx.r, dp, ds, dlm, dct),
                    );
                    eqcode(
                        &format!("CCtx_refPrefix_advanced(dct={dct})"),
                        rpc(cctx.c, dp, ds, dct),
                        rpr(cctx.r, dp, ds, dct),
                    );
                    eqcode(
                        &format!("DCtx_refPrefix_advanced(dct={dct})"),
                        dpc(dctx.c, dp, ds, dct),
                        dpr(dctx.r, dp, ds, dct),
                    );
                }
            }
        }

        // ZSTD_format_e on ZSTD_getFrameHeader_advanced
        {
            let src = gen_class(4, 2000, 3);
            let frame = c_compress(&src, 3);
            let (gac, gar) = duo::<
                unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize, c_int) -> usize,
            >("ZSTD_getFrameHeader_advanced");
            for &f in bad.iter().chain([0, 1].iter()) {
                let mut hc = ZSTD_frameHeader::default();
                let mut hr = ZSTD_frameHeader::default();
                let a = gac(
                    &mut hc,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    f,
                );
                let b = gar(
                    &mut hr,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    f,
                );
                eqcode(&format!("getFrameHeader_advanced(format={f})"), a, b);
                eqv(&format!("getFrameHeader_advanced(format={f}) out"), hc, hr);
            }
        }

        // ZSTD_strategy on ZSTD_cycleLog / ZSTD_selectBlockCompressor
        {
            let (clc, clr) = duo::<unsafe extern "C" fn(c_uint, c_int) -> c_uint>("ZSTD_cycleLog");
            for hl in [0u32, 1, 2, 10, 20, 31, 32, 63] {
                for &st in bad.iter().chain((0..12).collect::<Vec<c_int>>().iter()) {
                    eqv(
                        &format!("cycleLog({hl},{st})"),
                        clc(hl, st),
                        clr(hl, st),
                    );
                }
            }
            // ZSTD_selectBlockCompressor is an *internal-use-only* export whose
            // contract is documented in zstd_compress.c as
            //   "assumption : strat is a valid strategy"
            // and it indexes `blockCompressor[dictMode][strat]` /
            // `rowBasedBlockCompressors[dictMode][strat - ZSTD_greedy]` with no
            // validation. Out-of-range `dictMode`/`strat` are therefore an
            // out-of-bounds *read* in the C (silently returning a garbage
            // function pointer) while Rust's bounds-checked indexing panics.
            // That is a C precondition violation, not a translation
            // difference, so the probe stays inside the documented domain:
            // strat in [ZSTD_fast, ZSTD_btultra2], dictMode in [0,3],
            // useRowMatchFinder in [0,2].
            let (sbc, sbr) = duo::<
                unsafe extern "C" fn(c_int, c_int, c_int) -> *const c_void,
            >("ZSTD_selectBlockCompressor");
            for st in 1..=9i32 {
                for rmf in 0..=2i32 {
                    for dm in 0..=3i32 {
                        let a = sbc(st, rmf, dm);
                        let b = sbr(st, rmf, dm);
                        eqv(
                            &format!("selectBlockCompressor({st},{rmf},{dm}) null?"),
                            a.is_null(),
                            b.is_null(),
                        );
                    }
                }
            }
        }

        // ZSTD_CParamMode_e on ZSTD_getCParamsFromCCtxParams
        {
            let p_ = CtxPair::cctx_params();
            let (ic, ir) =
                duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
            eqcode("CCtxParams_init", ic(p_.c, 3), ir(p_.r, 3));
            let (fc, fr) = duo::<
                unsafe extern "C" fn(*const c_void, u64, usize, c_int) -> ZSTD_compressionParameters,
            >("ZSTD_getCParamsFromCCtxParams");
            for &mode in bad.iter().chain([0, 1, 2, 3].iter()) {
                for ss in [0u64, 1024, ZSTD_CONTENTSIZE_UNKNOWN] {
                    for ds in [0usize, 1024] {
                        eqv(
                            &format!("getCParamsFromCCtxParams(mode={mode},{ss},{ds})"),
                            fc(p_.c, ss, ds, mode),
                            fr(p_.r, ss, ds, mode),
                        );
                    }
                }
            }
        }
    }
}
