//! Phase C, ERRORS.md row 110: out-of-range enum values across the FFI
//! boundary.
//!
//! C enums accept any `int`, so a value with no valid variant is a real input
//! the C library handles somehow. This file passes `-1`, `INT_MIN`, `INT_MAX`,
//! `lower-1`, `upper+1` and random `i32`s to EVERY public function that takes
//! an enum, on both `.so`s, and asserts identical results.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

/// Every "out of range" integer worth trying for an enum-typed parameter.
fn wild(lo: c_int, hi: c_int, rng: &mut Rng) -> Vec<c_int> {
    let mut v = vec![
        lo.saturating_sub(1), lo.saturating_sub(2), hi.saturating_add(1), hi.saturating_add(2),
        -1, -2, -100, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 1 << 30, -(1 << 30),
    ];
    for _ in 0..12 {
        v.push(rng.next_u32() as c_int);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// `ZSTD_cParameter` / `ZSTD_dParameter` enum ids with no valid variant.
#[test]
fn parameter_enum_ids() {
    unsafe {
        let e = Err2::new();
        let (cb, rb) = both::<FnBounds>("ZSTD_cParam_getBounds");
        let (cdb, rdb) = both::<FnBounds>("ZSTD_dParam_getBounds");
        let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cs, rs_) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cg, rg) = both::<FnGetParam>("ZSTD_CCtx_getParameter");
        let (cds, rds) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (cdg, rdg) = both::<FnGetParam>("ZSTD_DCtx_getParameter");
        let (cpc, rpc) = both::<FnVoidToPtr>("ZSTD_createCCtxParams");
        let (cps, rps) = both::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (cpg, rpg) = both::<FnGetParam>("ZSTD_CCtxParams_getParameter");
        let cc = cn();
        let rc = rn();
        let d1 = cdn();
        let d2 = rdn();
        let p1 = cpc();
        let p2 = rpc();
        let mut rng = Rng::new(0xC1601);
        let mut ids: Vec<c_int> = BAD_CPARAMS.to_vec();
        ids.extend(BAD_DPARAMS.iter().copied());
        for _ in 0..3000 {
            ids.push(rng.next_u32() as c_int);
        }
        ids.extend(-700..2200i32);
        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            assert_eq!(cb(id), rb(id), "cParam_getBounds({id})");
            assert_eq!(cdb(id), rdb(id), "dParam_getBounds({id})");
            for v in [0i32, 1, -1, i32::MAX, i32::MIN] {
                e.eq(&format!("CCtx_setParameter({id},{v})"), cs(cc, id, v), rs_(rc, id, v));
                e.eq(&format!("CCtxParams_setParameter({id},{v})"), cps(p1, id, v), rps(p2, id, v));
                e.eq(&format!("DCtx_setParameter({id},{v})"), cds(d1, id, v), rds(d2, id, v));
            }
            let mut o1: c_int = 0x5A5A;
            let mut o2: c_int = 0x5A5A;
            e.eq(&format!("CCtx_getParameter({id})"), cg(cc, id, &mut o1), rg(rc, id, &mut o2));
            assert_eq!(o1, o2, "CCtx_getParameter({id}) out");
            let mut o1: c_int = 0x5A5A;
            let mut o2: c_int = 0x5A5A;
            e.eq(&format!("CCtxParams_getParameter({id})"),
                 cpg(p1, id, &mut o1), rpg(p2, id, &mut o2));
            assert_eq!(o1, o2, "CCtxParams_getParameter({id}) out");
            let mut o1: c_int = 0x5A5A;
            let mut o2: c_int = 0x5A5A;
            e.eq(&format!("DCtx_getParameter({id})"), cdg(d1, id, &mut o1), rdg(d2, id, &mut o2));
            assert_eq!(o1, o2, "DCtx_getParameter({id}) out");
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cc);
        rf(rc);
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cdf(d1);
        rdf(d2);
        let (cpf, rpf) = both::<FnPtrToSize>("ZSTD_freeCCtxParams");
        cpf(p1);
        rpf(p2);
    }
}

/// Out-of-range VALUES for every enum-typed parameter (strategy, format,
/// forceAttachDict, literalCompressionMode, ParamSwitch, bufferMode,
/// SequenceFormat), followed by an actual compression to prove the state the
/// library ended up in behaves identically.
#[test]
fn enum_valued_parameters() {
    unsafe {
        let e = Err2::new();
        let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cs, rs_) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (cg, rg) = both::<FnGetParam>("ZSTD_CCtx_getParameter");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cb, _) = both::<FnBounds>("ZSTD_cParam_getBounds");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let cc = cn();
        let rc = rn();
        let _d1 = cdn();
        let _d2 = rdn();
        let mut rng = Rng::new(0xC1602);
        let src = gen(Shape::Text, 30_000, &mut rng);

        // the enum-typed cParams
        let enum_params: &[(&str, c_int)] = &[
            ("strategy", ZSTD_c_strategy),
            ("format", ZSTD_c_format),
            ("forceAttachDict", ZSTD_c_forceAttachDict),
            ("literalCompressionMode", ZSTD_c_literalCompressionMode),
            ("enableLongDistanceMatching", ZSTD_c_enableLongDistanceMatching),
            ("useRowMatchFinder", ZSTD_c_useRowMatchFinder),
            ("prefetchCDictTables", ZSTD_c_prefetchCDictTables),
            ("splitAfterSequences", ZSTD_c_splitAfterSequences),
            ("repcodeResolution", ZSTD_c_repcodeResolution),
            ("stableInBuffer", ZSTD_c_stableInBuffer),
            ("stableOutBuffer", ZSTD_c_stableOutBuffer),
            ("blockDelimiters", ZSTD_c_blockDelimiters),
        ];
        for (name, id) in enum_params {
            let b = cb(*id);
            for v in wild(b.lowerBound, b.upperBound, &mut rng) {
                crst(cc, ZSTD_reset_session_and_parameters);
                rrst(rc, ZSTD_reset_session_and_parameters);
                let a = cs(cc, *id, v);
                let bb = rs_(rc, *id, v);
                e.eq(&format!("set {name}={v}"), a, bb);
                let mut o1: c_int = 0x5A5A;
                let mut o2: c_int = 0x5A5A;
                e.eq(&format!("get {name} after {v}"), cg(cc, *id, &mut o1), rg(rc, *id, &mut o2));
                assert_eq!(o1, o2, "get {name} value after set {v}");
                // and compress with whatever state resulted
                let cap = bnd(src.len()) + 64;
                let mut b1 = vec![0u8; cap];
                let mut b2 = vec![0u8; cap];
                let x = cc2(cc, b1.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len());
                let y = rc2(rc, b2.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len());
                if !e.eq_or_oom(&format!("compress2 after {name}={v}"), x, y) {
                    continue;
                }
                if !e.c.is_err(x) {
                    assert_bytes_eq(&format!("frame after {name}={v}"), &b1[..x], &b2[..y]);
                }
            }
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cc);
        rf(rc);
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cdf(_d1);
        rdf(_d2);
    }
}

/// `ZSTD_ResetDirective` and `ZSTD_EndDirective` with no valid variant.
#[test]
fn reset_and_end_directives() {
    unsafe {
        let e = Err2::new();
        let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cdrst, rdrst) = both::<FnReset>("ZSTD_DCtx_reset");
        type FnCS2 = unsafe extern "C" fn(
            *mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int,
        ) -> size_t;
        let (ccs, rcs) = both::<FnCS2>("ZSTD_compressStream2");
        let cc = cn();
        let rc = rn();
        let d1 = cdn();
        let d2 = rdn();
        let mut rng = Rng::new(0xC1603);
        let src = gen(Shape::Text, 4000, &mut rng);
        let mut vals = wild(1, 3, &mut rng);
        vals.extend([0, 1, 2, 3, 4]);
        vals.sort_unstable();
        vals.dedup();
        for v in &vals {
            e.eq(&format!("CCtx_reset({v})"), crst(cc, *v), rrst(rc, *v));
            e.eq(&format!("DCtx_reset({v})"), cdrst(d1, *v), rdrst(d2, *v));
        }
        let mut ends = wild(0, 2, &mut rng);
        ends.extend([0, 1, 2, 3]);
        ends.sort_unstable();
        ends.dedup();
        for v in &ends {
            crst(cc, ZSTD_reset_session_and_parameters);
            rrst(rc, ZSTD_reset_session_and_parameters);
            let mut o1 = vec![0u8; 1 << 16];
            let mut o2 = vec![0u8; 1 << 16];
            let mut cib =
                ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rib = cib;
            let mut cob =
                ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            let a = ccs(cc, &mut cob, &mut cib, *v);
            let b = rcs(rc, &mut rob, &mut rib, *v);
            e.eq(&format!("compressStream2 endOp={v}"), a, b);
            assert_eq!(cib.pos, rib.pos, "endOp={v}: in pos");
            assert_eq!(cob.pos, rob.pos, "endOp={v}: out pos");
            assert_bytes_eq(&format!("endOp={v}"), &o1[..cob.pos], &o2[..rob.pos]);
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cc);
        rf(rc);
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cdf(d1);
        rdf(d2);
    }
}

/// `ZSTD_dictContentType_e`, `ZSTD_dictLoadMethod_e`, `ZSTD_format_e` passed
/// directly to the `_advanced` dictionary and frame-header entry points.
#[test]
fn dict_and_format_enums() {
    unsafe {
        let e = Err2::new();
        let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        type FnLoadAdv = unsafe extern "C" fn(
            *mut c_void, *const c_void, size_t, c_int, c_int,
        ) -> size_t;
        type FnRefPrefixAdv =
            unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
        type FnGetFHAdv =
            unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t, c_int) -> size_t;
        type FnCreateCDictAdv = unsafe extern "C" fn(
            *const c_void, size_t, c_int, c_int, ZSTD_compressionParameters, ZSTD_customMem,
        ) -> *mut c_void;
        type FnCreateDDictAdv = unsafe extern "C" fn(
            *const c_void, size_t, c_int, c_int, ZSTD_customMem,
        ) -> *mut c_void;
        let (ccla, rcla) = both::<FnLoadAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (cdla, rdla) = both::<FnLoadAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (ccrp, rcrp) = both::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (cdrp, rdrp) = both::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
        let (cgfa, rgfa) = both::<FnGetFHAdv>("ZSTD_getFrameHeader_advanced");
        let (ccda, rcda) = both::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (cdda, rdda) = both::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (cfcd, rfcd) = both::<FnPtrToSize>("ZSTD_freeCDict");
        let (cfdd, rfdd) = both::<FnPtrToSize>("ZSTD_freeDDict");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cdrst, rdrst) = both::<FnReset>("ZSTD_DCtx_reset");
        let (cgcp, _) = both::<unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters>(
            "ZSTD_getCParams",
        );

        let cc = cn();
        let rc = rn();
        let d1 = cdn();
        let d2 = rdn();
        let mut rng = Rng::new(0xC1604);
        let dict = gen(Shape::Text, 4096, &mut rng);
        let src = gen(Shape::Text, 5000, &mut rng);
        let frame = {
            let (c, _) = both::<unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t>("ZSTD_compress");
            let (bd, _) = both::<FnBound>("ZSTD_compressBound");
            let cap = bd(src.len()) + 64;
            let mut o = vec![0u8; cap];
            let n = c(o.as_mut_ptr() as *mut c_void, cap,
                      src.as_ptr() as *const c_void, src.len(), 3);
            o.truncate(n);
            o
        };
        let cm = ZSTD_customMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() };
        let cparams = cgcp(3, src.len() as c_ulonglong, dict.len());

        let mut vals = wild(0, 2, &mut rng);
        vals.extend([0, 1, 2, 3]);
        vals.sort_unstable();
        vals.dedup();
        let dp = dict.as_ptr() as *const c_void;
        for &dlm in &vals {
            for &dct in &vals {
                crst(cc, ZSTD_reset_session_and_parameters);
                rrst(rc, ZSTD_reset_session_and_parameters);
                cdrst(d1, ZSTD_reset_session_and_parameters);
                rdrst(d2, ZSTD_reset_session_and_parameters);
                e.eq(&format!("CCtx_loadDictionary_advanced(dlm={dlm},dct={dct})"),
                     ccla(cc, dp, dict.len(), dlm, dct),
                     rcla(rc, dp, dict.len(), dlm, dct));
                e.eq(&format!("DCtx_loadDictionary_advanced(dlm={dlm},dct={dct})"),
                     cdla(d1, dp, dict.len(), dlm, dct),
                     rdla(d2, dp, dict.len(), dlm, dct));
                let a = ccda(dp, dict.len(), dlm, dct, cparams, cm);
                let b = rcda(dp, dict.len(), dlm, dct, cparams, cm);
                assert_eq!(a.is_null(), b.is_null(),
                           "createCDict_advanced(dlm={dlm},dct={dct}) nullness");
                if !a.is_null() { cfcd(a); }
                if !b.is_null() { rfcd(b); }
                let a = cdda(dp, dict.len(), dlm, dct, cm);
                let b = rdda(dp, dict.len(), dlm, dct, cm);
                assert_eq!(a.is_null(), b.is_null(),
                           "createDDict_advanced(dlm={dlm},dct={dct}) nullness");
                if !a.is_null() { cfdd(a); }
                if !b.is_null() { rfdd(b); }
            }
            crst(cc, ZSTD_reset_session_and_parameters);
            rrst(rc, ZSTD_reset_session_and_parameters);
            cdrst(d1, ZSTD_reset_session_and_parameters);
            rdrst(d2, ZSTD_reset_session_and_parameters);
            e.eq(&format!("CCtx_refPrefix_advanced(dct={dlm})"),
                 ccrp(cc, dp, dict.len(), dlm), rcrp(rc, dp, dict.len(), dlm));
            e.eq(&format!("DCtx_refPrefix_advanced(dct={dlm})"),
                 cdrp(d1, dp, dict.len(), dlm), rdrp(d2, dp, dict.len(), dlm));
        }
        // ZSTD_format_e on getFrameHeader_advanced
        let mut fmts = wild(0, 1, &mut rng);
        fmts.extend([0, 1, 2]);
        fmts.sort_unstable();
        fmts.dedup();
        for &fmt in &fmts {
            for cut in [0usize, 1, 2, 3, 4, 5, 6, 8, 13, frame.len()] {
                if cut > frame.len() { continue; }
                let mut h1: ZSTD_frameHeader = std::mem::zeroed();
                let mut h2: ZSTD_frameHeader = std::mem::zeroed();
                let a = cgfa(&mut h1, frame.as_ptr() as *const c_void, cut, fmt);
                let b = rgfa(&mut h2, frame.as_ptr() as *const c_void, cut, fmt);
                e.eq(&format!("getFrameHeader_advanced(fmt={fmt},cut={cut})"), a, b);
                if a == 0 {
                    assert_eq!(h1, h2, "getFrameHeader_advanced struct fmt={fmt} cut={cut}");
                }
            }
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cc);
        rf(rc);
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cdf(d1);
        rdf(d2);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

/// `ZSTD_ErrorCode` values with no valid variant, through the error API.
#[test]
fn error_code_enum() {
    unsafe {
        let (cgs, rgs) = both::<unsafe extern "C" fn(c_int) -> *const std::os::raw::c_char>(
            "ZSTD_getErrorString",
        );
        let (ces, res) = both::<unsafe extern "C" fn(c_int) -> *const std::os::raw::c_char>(
            "ERR_getErrorString",
        );
        for v in -500..800i32 {
            assert_eq!(cstr(cgs(v)), cstr(rgs(v)), "ZSTD_getErrorString({v})");
            assert_eq!(cstr(ces(v)), cstr(res(v)), "ERR_getErrorString({v})");
        }
        let mut rng = Rng::new(0xC1605);
        for _ in 0..5000 {
            let v = rng.next_u32() as c_int;
            assert_eq!(cstr(cgs(v)), cstr(rgs(v)), "ZSTD_getErrorString({v})");
            assert_eq!(cstr(ces(v)), cstr(res(v)), "ERR_getErrorString({v})");
        }
        // and the size_t-sentinel side
        let (cis, ris) = both::<FnIsError>("ZSTD_isError");
        let (cgc, rgc) = both::<FnGetErrorCode>("ZSTD_getErrorCode");
        let (cgn, rgn) = both::<FnGetErrorName>("ZSTD_getErrorName");
        let mut vals: Vec<usize> = (0..400).collect();
        vals.extend((1..400usize).map(|i| 0usize.wrapping_sub(i)));
        vals.extend([usize::MAX, usize::MAX / 2, 1 << 40, 1 << 63]);
        for _ in 0..5000 {
            vals.push(rng.next_u64() as usize);
        }
        for v in vals {
            assert_eq!(cis(v), ris(v), "ZSTD_isError({v:#x})");
            assert_eq!(cgc(v), rgc(v), "ZSTD_getErrorCode({v:#x})");
            assert_eq!(cstr(cgn(v)), cstr(rgn(v)), "ZSTD_getErrorName({v:#x})");
        }
    }
}

/// `ZSTD_strategy` inside a `ZSTD_compressionParameters` struct handed to the
/// struct-taking entry points.
#[test]
fn strategy_in_struct() {
    unsafe {
        let e = Err2::new();
        let (cchk, rchk) = both::<unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t>(
            "ZSTD_checkCParams",
        );
        let (cadj, radj) = both::<
            unsafe extern "C" fn(
                ZSTD_compressionParameters, c_ulonglong, size_t,
            ) -> ZSTD_compressionParameters,
        >("ZSTD_adjustCParams");
        let (cn, rn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (csc, rsc) = both::<unsafe extern "C" fn(*mut c_void, ZSTD_compressionParameters) -> size_t>(
            "ZSTD_CCtx_setCParams",
        );
        let cc = cn();
        let rc = rn();
        let mut rng = Rng::new(0xC1606);
        for _ in 0..20_000 {
            let c = ZSTD_compressionParameters {
                windowLog: if rng.bool() { rng.range(0, 40) as c_uint } else { rng.next_u32() },
                chainLog: if rng.bool() { rng.range(0, 40) as c_uint } else { rng.next_u32() },
                hashLog: if rng.bool() { rng.range(0, 40) as c_uint } else { rng.next_u32() },
                searchLog: if rng.bool() { rng.range(0, 40) as c_uint } else { rng.next_u32() },
                minMatch: if rng.bool() { rng.range(0, 12) as c_uint } else { rng.next_u32() },
                targetLength: if rng.bool() {
                    rng.range(0, 200_000) as c_uint
                } else {
                    rng.next_u32()
                },
                strategy: if rng.bool() { rng.range(0, 14) as c_uint } else { rng.next_u32() },
            };
            e.eq(&format!("checkCParams({c:?})"), cchk(c), rchk(c));
            assert_eq!(cadj(c, 1 << 16, 0), radj(c, 1 << 16, 0), "adjustCParams({c:?})");
            assert_eq!(
                cadj(c, ZSTD_CONTENTSIZE_UNKNOWN, 1 << 10),
                radj(c, ZSTD_CONTENTSIZE_UNKNOWN, 1 << 10),
                "adjustCParams({c:?}, unknown)"
            );
            e.eq(&format!("CCtx_setCParams({c:?})"), csc(cc, c), rsc(rc, c));
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cc);
        rf(rc);
    }
}
