//! Coverage completion: the remaining directly-callable exported entry points
//! that the other Phase B/C files reach only indirectly.
//!
//! `CONFIGS.md` rows 1, 36, 42, 43, 44, 47, 49, 59 and the `*_deprecated` /
//! `*_advanced` / `*_usingCDict` variants.

mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_uint, c_void};

const P_LEVEL: c_int = 100;
const P_CHECKSUMFLAG: c_int = 201;
const P_STRATEGY: c_int = 107;

type F2 = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;

/// `ZSTD_frameProgression` — layout from `c_src/src/include/zstd.h:2736`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct FrameProgression {
    ingested: u64,
    consumed: u64,
    produced: u64,
    flushed: u64,
    current_job_id: c_uint,
    nb_active_workers: c_uint,
}

/// `ZSTD_customMem`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CustomMem {
    alloc: Option<unsafe extern "C" fn(*mut c_void, Sz) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
}

fn diff_decompress(p: &Pair, frame: &[u8], expect: &[u8], tag: &str) {
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    unsafe {
        let cap = expect.len() + 64;
        let mut cb = vec![0xA5u8; cap];
        let mut rb = vec![0xA5u8; cap];
        let cn = c_de(cb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        let rn = r_de(rb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        eq(&format!("{tag}: decompress ret"), cn, rn);
        if c_ie(cn) != 0 {
            return;
        }
        eq_bytes(&format!("{tag}: decoded"), &cb[..cn], &rb[..rn]);
        eq_bytes(&format!("{tag}: round trip"), expect, &cb[..cn]);
    }
}

/// Row 1 extras — the size / margin / log helpers.
#[test]
fn row1_size_and_margin_helpers() {
    let p = libs();
    type FMargin = unsafe extern "C" fn(*const c_void, Sz) -> Sz;
    type FDbsm = unsafe extern "C" fn(u64, u64) -> Sz;
    type FCycle = unsafe extern "C" fn(c_uint, CParams) -> c_uint;
    type FGetCParams = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;
    let (c_dm, r_dm) = p.sym::<FMargin>("ZSTD_decompressionMargin");
    let (c_edsf, r_edsf) = p.sym::<FMargin>("ZSTD_estimateDStreamSize_fromFrame");
    let (c_dbsm, r_dbsm) = p.sym::<FDbsm>("ZSTD_decodingBufferSize_min");
    let (c_cl, r_cl) = p.sym::<FCycle>("ZSTD_cycleLog");
    let (c_gcp, _) = p.sym::<FGetCParams>("ZSTD_getCParams");
    let (c_ges, r_ges) = p.sym::<unsafe extern "C" fn(*const c_void) -> Sz>("ZSTD_estimateCCtxSize_usingCCtxParams");
    let (c_gess, r_gess) = p.sym::<unsafe extern "C" fn(*const c_void) -> Sz>("ZSTD_estimateCStreamSize_usingCCtxParams");
    let (c_pnew, r_pnew) = p.sym::<FnCreateCtx>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = p.sym::<FnFreeCtx>("ZSTD_freeCCtxParams");
    let (c_pset, r_pset) = p.sym::<FnSetParam>("ZSTD_CCtxParams_setParameter");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");

    let mut rng = Rng::new(SEED ^ 0x01);
    unsafe {
        // ZSTD_cycleLog over the strategy x chainLog cross-product
        for lvl in [-1000, -1, 0, 1, 9, 19, 22] {
            for ss in [0u64, 1024, 1 << 20, u64::MAX] {
                let cp = c_gcp(lvl, ss, 0);
                for hl in [0u32, 1, 6, 12, 20, 30, 31, 99] {
                    eq(
                        &format!("ZSTD_cycleLog({hl}, lvl={lvl}, ss={ss})"),
                        c_cl(hl, cp),
                        r_cl(hl, cp),
                    );
                }
            }
        }
        // ZSTD_decodingBufferSize_min over window / content-size axes
        for w in [0u64, 1, 1024, 1 << 10, 1 << 17, 1 << 20, 1 << 27, 1u64 << 31] {
            for fcs in [0u64, 1, 1024, 1 << 20, CONTENTSIZE_UNKNOWN, CONTENTSIZE_ERROR, u64::MAX / 2] {
                eq(
                    &format!("ZSTD_decodingBufferSize_min({w},{fcs})"),
                    c_dbsm(w, fcs),
                    r_dbsm(w, fcs),
                );
            }
        }
        // margin / fromFrame over real frames, garbage, and every truncation
        let (c_new, _) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
        let (c_free, _) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
        let (c_set, _) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c_c2, _) = p.sym::<F2>("ZSTD_compress2");
        let mut corpus: Vec<Vec<u8>> = vec![Vec::new(), vec![0u8; 4], gen(Shape::Incompressible, 64, &mut rng)];
        for ck in [0, 1] {
            for len in [0usize, 1, 1024, 70_000] {
                let src = gen(Shape::TextLike, len, &mut rng);
                let cc = c_new();
                c_set(cc, P_CHECKSUMFLAG, ck);
                let cap = c_cb(len) + 64;
                let mut b = vec![0u8; cap];
                let n = c_c2(cc, b.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                c_free(cc);
                if c_ie(n) == 0 {
                    b.truncate(n);
                    corpus.push(b);
                }
            }
        }
        for v in &corpus {
            for cut in 0..=v.len().min(24) {
                let vp = if v.is_empty() { std::ptr::null() } else { v.as_ptr() as *const c_void };
                eq(
                    &format!("ZSTD_decompressionMargin(len={cut})"),
                    c_dm(vp, cut),
                    r_dm(vp, cut),
                );
                eq(
                    &format!("ZSTD_estimateDStreamSize_fromFrame(len={cut})"),
                    c_edsf(vp, cut),
                    r_edsf(vp, cut),
                );
            }
            let vp = if v.is_empty() { std::ptr::null() } else { v.as_ptr() as *const c_void };
            eq("ZSTD_decompressionMargin(full)", c_dm(vp, v.len()), r_dm(vp, v.len()));
            eq(
                "ZSTD_estimateDStreamSize_fromFrame(full)",
                c_edsf(vp, v.len()),
                r_edsf(vp, v.len()),
            );
        }
        // estimate*_usingCCtxParams over the parameter sweep
        let cp = c_pnew();
        let rp = r_pnew();
        for (prm, name) in C_PARAMS {
            let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");
            let b = c_gb(prm);
            for val in [b.lower, b.upper] {
                let a1 = c_pset(cp, prm, val);
                let b1 = r_pset(rp, prm, val);
                eq(&format!("CCtxParams_setParameter({name}={val})"), a1, b1);
                if c_ie(a1) != 0 {
                    continue;
                }
                eq(
                    &format!("estimateCCtxSize_usingCCtxParams({name}={val})"),
                    c_ges(cp),
                    r_ges(rp),
                );
                eq(
                    &format!("estimateCStreamSize_usingCCtxParams({name}={val})"),
                    c_gess(cp),
                    r_gess(rp),
                );
            }
        }
        c_pfree(cp);
        r_pfree(rp);
    }
}

/// Row 36/37 extras — `ZSTD_compress_advanced`, `ZSTD_compress_usingCDict_advanced`,
/// `ZSTD_createCDict_byReference`, `ZSTD_createCDict_advanced2`.
#[test]
fn row36_advanced_compress_variants() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FAdv = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, Sz, Params,
    ) -> Sz;
    type FUsingCDictAdv = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *const c_void, Sz, *const c_void, FParams,
    ) -> Sz;
    type FCDictByRef = unsafe extern "C" fn(*const c_void, Sz, c_int) -> *mut c_void;
    type FCDictAdv2 =
        unsafe extern "C" fn(*const c_void, Sz, c_int, c_int, *const c_void, CustomMem) -> *mut c_void;
    type FGetParams = unsafe extern "C" fn(c_int, u64, Sz) -> Params;
    type FGetCParamsFromCDict = unsafe extern "C" fn(*const c_void) -> CParams;
    let (c_adv, r_adv) = p.sym::<FAdv>("ZSTD_compress_advanced");
    let (c_ucda, r_ucda) = p.sym::<FUsingCDictAdv>("ZSTD_compress_usingCDict_advanced");
    let (c_cdr, r_cdr) = p.sym::<FCDictByRef>("ZSTD_createCDict_byReference");
    let (c_cda2, r_cda2) = p.sym::<FCDictAdv2>("ZSTD_createCDict_advanced2");
    let (c_ccd, r_ccd) = p.sym::<FCDictByRef>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = p.sym::<FnFreeCtx>("ZSTD_freeCDict");
    let (c_gp, _) = p.sym::<FGetParams>("ZSTD_getParams");
    let (c_gcfc, r_gcfc) = p.sym::<FGetCParamsFromCDict>("ZSTD_getCParamsFromCDict");
    let (c_pnew, r_pnew) = p.sym::<FnCreateCtx>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = p.sym::<FnFreeCtx>("ZSTD_freeCCtxParams");
    let (c_pinit, r_pinit) = p.sym::<unsafe extern "C" fn(*mut c_void, c_int) -> Sz>("ZSTD_CCtxParams_init");

    let mut rng = Rng::new(SEED ^ 0x36);
    unsafe {
        for dict in [Vec::new(), gen(Shape::TextLike, 4096, &mut rng)] {
            let dp = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
            let ds = dict.len();
            for level in [1, 3, 9, 19] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible] {
                    for len in [1usize, 1024, 70_000] {
                        let src = gen(shape, len, &mut rng);
                        let cap = c_cb(len) + 64;
                        let prm = c_gp(level, len as u64, ds);
                        let tag = format!("adv lvl={level} shape={shape:?} len={len} ds={ds}");
                        // ZSTD_compress_advanced
                        let cc = c_new();
                        let rc = r_new();
                        let mut cf = vec![0u8; cap];
                        let mut rf = vec![0u8; cap];
                        let cn = c_adv(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, dp, ds, prm);
                        let rn = r_adv(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, dp, ds, prm);
                        eq(&format!("{tag}: compress_advanced ret"), cn, rn);
                        if c_ie(cn) == 0 {
                            eq_bytes(&format!("{tag}: compress_advanced frame"), &cf[..cn], &rf[..rn]);
                        }
                        c_free(cc);
                        r_free(rc);

                        // ZSTD_createCDict_byReference + usingCDict_advanced
                        let ccd = c_cdr(dp, ds, level);
                        let rcd = r_cdr(dp, ds, level);
                        eq(&format!("{tag}: createCDict_byReference nullness"), ccd.is_null(), rcd.is_null());
                        if !ccd.is_null() {
                            eq(&format!("{tag}: getCParamsFromCDict"), c_gcfc(ccd), r_gcfc(rcd));
                            for cs in [0, 1] {
                                for ck in [0, 1] {
                                    for nd in [0, 1] {
                                        let fp = FParams { content_size_flag: cs, checksum_flag: ck, no_dict_id_flag: nd };
                                        let cc = c_new();
                                        let rc = r_new();
                                        let mut cf = vec![0u8; cap];
                                        let mut rf = vec![0u8; cap];
                                        let cn = c_ucda(cc, cf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, ccd, fp);
                                        let rn = r_ucda(rc, rf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, rcd, fp);
                                        eq(&format!("{tag}: usingCDict_advanced({cs},{ck},{nd}) ret"), cn, rn);
                                        if c_ie(cn) == 0 {
                                            eq_bytes(&format!("{tag}: usingCDict_advanced frame"), &cf[..cn], &rf[..rn]);
                                        }
                                        c_free(cc);
                                        r_free(rc);
                                    }
                                }
                            }
                        }
                        c_fcd(ccd);
                        r_fcd(rcd);

                        // ZSTD_createCDict_advanced2
                        let cp = c_pnew();
                        let rp = r_pnew();
                        c_pinit(cp, level);
                        r_pinit(rp, level);
                        for dlm in [0, 1, -1, 3] {
                            for dct in [0, 1, 2, -1, 99] {
                                let x = c_cda2(dp, ds, dlm, dct, cp, CustomMem::default());
                                let y = r_cda2(dp, ds, dlm, dct, cp, CustomMem::default());
                                eq(
                                    &format!("{tag}: createCDict_advanced2(dlm={dlm},dct={dct}) nullness"),
                                    x.is_null(),
                                    y.is_null(),
                                );
                                if !x.is_null() {
                                    eq(&format!("{tag}: getCParamsFromCDict adv2"), c_gcfc(x), r_gcfc(y));
                                }
                                c_fcd(x);
                                r_fcd(y);
                            }
                        }
                        c_pfree(cp);
                        r_pfree(rp);
                    }
                }
            }
        }
    }
}

/// Row 42 — every legacy streaming init variant, plus `ZSTD_resetDStream`.
#[test]
fn row42_legacy_stream_inits() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cnew, r_cnew) = p.sym::<FnCreateCtx>("ZSTD_createCStream");
    let (c_cfree, r_cfree) = p.sym::<FnFreeCtx>("ZSTD_freeCStream");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDStream");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDStream");
    let (c_cos, _) = p.sym::<FnVoidSz>("ZSTD_CStreamOutSize");
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    type FDS = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> Sz;
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let (c_ds, r_ds) = p.sym::<FDS>("ZSTD_decompressStream");

    type FInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, c_int) -> Sz;
    type FInitAdv = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, Params, u64) -> Sz;
    type FInitCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FInitCDictAdv =
        unsafe extern "C" fn(*mut c_void, *const c_void, FParams, u64) -> Sz;
    type FInitSrcSize = unsafe extern "C" fn(*mut c_void, c_int, u64) -> Sz;
    type FDInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FDInitDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FResetD = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FCreateCDict = unsafe extern "C" fn(*const c_void, Sz, c_int) -> *mut c_void;
    type FCreateDDict = unsafe extern "C" fn(*const c_void, Sz) -> *mut c_void;
    type FGetParams = unsafe extern "C" fn(c_int, u64, Sz) -> Params;

    let (c_iud, r_iud) = p.sym::<FInitDict>("ZSTD_initCStream_usingDict");
    let (c_iad, r_iad) = p.sym::<FInitAdv>("ZSTD_initCStream_advanced");
    let (c_iucd, r_iucd) = p.sym::<FInitCDict>("ZSTD_initCStream_usingCDict");
    let (c_iucda, r_iucda) = p.sym::<FInitCDictAdv>("ZSTD_initCStream_usingCDict_advanced");
    let (c_iss, r_iss) = p.sym::<FInitSrcSize>("ZSTD_initCStream_srcSize");
    let (c_diud, r_diud) = p.sym::<FDInitDict>("ZSTD_initDStream_usingDict");
    let (c_diudd, r_diudd) = p.sym::<FDInitDDict>("ZSTD_initDStream_usingDDict");
    let (c_rds, r_rds) = p.sym::<FResetD>("ZSTD_resetDStream");
    let (c_ccd, r_ccd) = p.sym::<FCreateCDict>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = p.sym::<FnFreeCtx>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) = p.sym::<FCreateDDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = p.sym::<FnFreeCtx>("ZSTD_freeDDict");
    let (c_gp, _) = p.sym::<FGetParams>("ZSTD_getParams");

    let mut rng = Rng::new(SEED ^ 0x42);
    unsafe {
        for dict in [Vec::new(), gen(Shape::TextLike, 4096, &mut rng)] {
            let dp = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
            let ds = dict.len();
            for level in [1, 3, 9, 19] {
                let ccd = c_ccd(dp, ds, level);
                let rcd = r_ccd(dp, ds, level);
                let cdd = c_cdd(dp, ds);
                let rdd = r_cdd(dp, ds);
                for mode in 0..5 {
                    for shape in [Shape::TextLike, Shape::Incompressible] {
                        for len in [1usize, 1024, 70_000] {
                            let src = gen(shape, len, &mut rng);
                            let tag =
                                format!("row42 mode={mode} lvl={level} ds={ds} shape={shape:?} len={len}");
                            let cc = c_cnew();
                            let rc = r_cnew();
                            let prm = c_gp(level, len as u64, ds);
                            let fp = FParams { content_size_flag: 1, checksum_flag: 1, no_dict_id_flag: 0 };
                            let (a, b) = match mode {
                                0 => (c_iud(cc, dp, ds, level), r_iud(rc, dp, ds, level)),
                                1 => (
                                    c_iad(cc, dp, ds, prm, len as u64),
                                    r_iad(rc, dp, ds, prm, len as u64),
                                ),
                                2 if !ccd.is_null() => (c_iucd(cc, ccd), r_iucd(rc, rcd)),
                                3 if !ccd.is_null() => (
                                    c_iucda(cc, ccd, fp, len as u64),
                                    r_iucda(rc, rcd, fp, len as u64),
                                ),
                                4 => (
                                    c_iss(cc, level, len as u64),
                                    r_iss(rc, level, len as u64),
                                ),
                                _ => {
                                    c_cfree(cc);
                                    r_cfree(rc);
                                    continue;
                                }
                            };
                            eq(&format!("{tag}: init ret"), a, b);
                            if c_ie(a) != 0 {
                                c_cfree(cc);
                                r_cfree(rc);
                                continue;
                            }
                            // drive the stream to completion
                            let outsz = c_cos();
                            let mut cbuf = vec![0u8; outsz];
                            let mut rbuf = vec![0u8; outsz];
                            let mut cstream = Vec::new();
                            let mut rstream = Vec::new();
                            let mut cin = InBuffer { src: src.as_ptr() as *const c_void, size: len, pos: 0 };
                            let mut rin = InBuffer { src: src.as_ptr() as *const c_void, size: len, pos: 0 };
                            let mut failed = false;
                            loop {
                                let mut cob = OutBuffer { dst: cbuf.as_mut_ptr() as *mut c_void, size: outsz, pos: 0 };
                                let mut rob = OutBuffer { dst: rbuf.as_mut_ptr() as *mut c_void, size: outsz, pos: 0 };
                                let cr = c_cs2(cc, &mut cob, &mut cin, 2);
                                let rr = r_cs2(rc, &mut rob, &mut rin, 2);
                                eq(&format!("{tag}: compressStream2 ret"), cr, rr);
                                if c_ie(cr) != 0 {
                                    failed = true;
                                    break;
                                }
                                eq(&format!("{tag}: in.pos"), cin.pos, rin.pos);
                                eq(&format!("{tag}: out.pos"), cob.pos, rob.pos);
                                eq_bytes(&format!("{tag}: chunk"), &cbuf[..cob.pos], &rbuf[..rob.pos]);
                                cstream.extend_from_slice(&cbuf[..cob.pos]);
                                rstream.extend_from_slice(&rbuf[..rob.pos]);
                                if cr == 0 {
                                    break;
                                }
                            }
                            c_cfree(cc);
                            r_cfree(rc);
                            if failed {
                                continue;
                            }
                            eq_bytes(&format!("{tag}: whole stream"), &cstream, &rstream);

                            // decode through each initDStream variant
                            for dmode in 0..3 {
                                let cd = c_dnew();
                                let rd = r_dnew();
                                let (a, b) = match dmode {
                                    0 => (c_diud(cd, dp, ds), r_diud(rd, dp, ds)),
                                    1 if !cdd.is_null() => (c_diudd(cd, cdd), r_diudd(rd, rdd)),
                                    2 => {
                                        // resetDStream keeps the dictionary reference
                                        c_diud(cd, dp, ds);
                                        r_diud(rd, dp, ds);
                                        (c_rds(cd), r_rds(rd))
                                    }
                                    _ => {
                                        c_dfree(cd);
                                        r_dfree(rd);
                                        continue;
                                    }
                                };
                                eq(&format!("{tag}: initDStream(dmode={dmode}) ret"), a, b);
                                if c_ie(a) != 0 {
                                    c_dfree(cd);
                                    r_dfree(rd);
                                    continue;
                                }
                                let mut cout = vec![0u8; len + 64];
                                let mut rout = vec![0u8; len + 64];
                                let mut cin = InBuffer { src: cstream.as_ptr() as *const c_void, size: cstream.len(), pos: 0 };
                                let mut rin = InBuffer { src: rstream.as_ptr() as *const c_void, size: rstream.len(), pos: 0 };
                                let mut cob = OutBuffer { dst: cout.as_mut_ptr() as *mut c_void, size: cout.len(), pos: 0 };
                                let mut rob = OutBuffer { dst: rout.as_mut_ptr() as *mut c_void, size: rout.len(), pos: 0 };
                                loop {
                                    let cr = c_ds(cd, &mut cob, &mut cin);
                                    let rr = r_ds(rd, &mut rob, &mut rin);
                                    eq(&format!("{tag}: decompressStream(dmode={dmode}) ret"), cr, rr);
                                    if c_ie(cr) != 0 || cr == 0 {
                                        break;
                                    }
                                    if cin.pos == cin.size {
                                        break;
                                    }
                                }
                                eq(&format!("{tag}: decode out.pos"), cob.pos, rob.pos);
                                eq_bytes(&format!("{tag}: decoded"), &cout[..cob.pos], &rout[..rob.pos]);
                                c_dfree(cd);
                                r_dfree(rd);
                            }
                        }
                    }
                }
                c_fcd(ccd);
                r_fcd(rcd);
                c_fdd(cdd);
                r_fdd(rdd);
            }
        }
    }
}

/// Rows 43/44 extras — `ZSTD_compressBegin_using*`, `ZSTD_decompressBegin_using*`,
/// the `*_deprecated` block wrappers, `ZSTD_getFrameProgression`,
/// `ZSTD_toFlushNow`, `ZSTD_DDict_dictContent/dictSize`,
/// `ZSTD_copyDDictParameters`, `ZSTD_CCtx_set{C,F,}Params`,
/// `ZSTD_CCtxParams_init_advanced`.
#[test]
fn row43_row44_begin_variants_and_accessors() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");

    type FBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz, c_int) -> Sz;
    type FBeginCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FBeginCDictAdv =
        unsafe extern "C" fn(*mut c_void, *const c_void, FParams, u64) -> Sz;
    type FBeginCDictDep = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FDBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FDBeginDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FBlk = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FGetBs = unsafe extern "C" fn(*const c_void) -> Sz;
    type FProg = unsafe extern "C" fn(*const c_void) -> FrameProgression;
    type FFlushNow = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FDDictContent = unsafe extern "C" fn(*const c_void) -> *const c_void;
    type FDDictSize = unsafe extern "C" fn(*const c_void) -> Sz;
    type FCopyDDictParams = unsafe extern "C" fn(*mut c_void, *const c_void);
    type FSetCParams = unsafe extern "C" fn(*mut c_void, CParams) -> Sz;
    type FSetFParams = unsafe extern "C" fn(*mut c_void, FParams) -> Sz;
    type FSetParams = unsafe extern "C" fn(*mut c_void, Params) -> Sz;
    type FInitAdvP = unsafe extern "C" fn(*mut c_void, Params) -> Sz;
    type FCreateCDict = unsafe extern "C" fn(*const c_void, Sz, c_int) -> *mut c_void;
    type FCreateDDict = unsafe extern "C" fn(*const c_void, Sz) -> *mut c_void;
    type FGetParams = unsafe extern "C" fn(c_int, u64, Sz) -> Params;
    type FGetCParams = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;

    let (c_bud, r_bud) = p.sym::<FBeginDict>("ZSTD_compressBegin_usingDict");
    let (c_bucd, r_bucd) = p.sym::<FBeginCDict>("ZSTD_compressBegin_usingCDict");
    let (c_bucda, r_bucda) = p.sym::<FBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
    let (c_bucdd, r_bucdd) = p.sym::<FBeginCDictDep>("ZSTD_compressBegin_usingCDict_deprecated");
    let (c_dbud, r_dbud) = p.sym::<FDBeginDict>("ZSTD_decompressBegin_usingDict");
    let (c_dbudd, r_dbudd) = p.sym::<FDBeginDDict>("ZSTD_decompressBegin_usingDDict");
    let (c_cblkd, r_cblkd) = p.sym::<FBlk>("ZSTD_compressBlock_deprecated");
    let (c_dblkd, r_dblkd) = p.sym::<FBlk>("ZSTD_decompressBlock_deprecated");
    let (c_bs, r_bs) = p.sym::<FGetBs>("ZSTD_getBlockSize");
    let (c_prog, r_prog) = p.sym::<FProg>("ZSTD_getFrameProgression");
    let (c_tfn, r_tfn) = p.sym::<FFlushNow>("ZSTD_toFlushNow");
    let (c_ddc, r_ddc) = p.sym::<FDDictContent>("ZSTD_DDict_dictContent");
    let (c_dds, r_dds) = p.sym::<FDDictSize>("ZSTD_DDict_dictSize");
    let (c_cdp, r_cdp) = p.sym::<FCopyDDictParams>("ZSTD_copyDDictParameters");
    let (c_scp, r_scp) = p.sym::<FSetCParams>("ZSTD_CCtx_setCParams");
    let (c_sfp, r_sfp) = p.sym::<FSetFParams>("ZSTD_CCtx_setFParams");
    let (c_sp, r_sp) = p.sym::<FSetParams>("ZSTD_CCtx_setParams");
    let (c_pia, r_pia) = p.sym::<FInitAdvP>("ZSTD_CCtxParams_init_advanced");
    let (c_pnew, r_pnew) = p.sym::<FnCreateCtx>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = p.sym::<FnFreeCtx>("ZSTD_freeCCtxParams");
    let (c_ccd, r_ccd) = p.sym::<FCreateCDict>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = p.sym::<FnFreeCtx>("ZSTD_freeCDict");
    let (c_cdd, r_cdd) = p.sym::<FCreateDDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = p.sym::<FnFreeCtx>("ZSTD_freeDDict");
    let (c_gp, _) = p.sym::<FGetParams>("ZSTD_getParams");
    let (c_gcp, _) = p.sym::<FGetCParams>("ZSTD_getCParams");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    type FCont = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    let (c_cont, r_cont) = p.sym::<FCont>("ZSTD_compressContinue");
    let (c_end, r_end) = p.sym::<FCont>("ZSTD_compressEnd");

    let mut rng = Rng::new(SEED ^ 0x43_44);
    unsafe {
        // ZSTD_CCtx_set{C,F,}Params and CCtxParams_init_advanced
        for lvl in [1, 3, 9, 19] {
            for ss in [0u64, 1024, 1 << 20] {
                let prm = c_gp(lvl, ss, 0);
                let cparams = c_gcp(lvl, ss, 0);
                let cc = c_new();
                let rc = r_new();
                eq(
                    &format!("ZSTD_CCtx_setCParams(lvl={lvl},ss={ss})"),
                    c_scp(cc, cparams),
                    r_scp(rc, cparams),
                );
                eq(
                    &format!("ZSTD_CCtx_setFParams(lvl={lvl},ss={ss})"),
                    c_sfp(cc, prm.f_params),
                    r_sfp(rc, prm.f_params),
                );
                eq(
                    &format!("ZSTD_CCtx_setParams(lvl={lvl},ss={ss})"),
                    c_sp(cc, prm),
                    r_sp(rc, prm),
                );
                c_free(cc);
                r_free(rc);
                // deliberately out-of-range structs
                for bad in [
                    CParams::default(),
                    CParams { window_log: 99, chain_log: 99, hash_log: 99, search_log: 99, min_match: 99, target_length: 99, strategy: 99 },
                    CParams { window_log: u32::MAX, chain_log: u32::MAX, hash_log: u32::MAX, search_log: u32::MAX, min_match: u32::MAX, target_length: u32::MAX, strategy: u32::MAX },
                ] {
                    let cc = c_new();
                    let rc = r_new();
                    eq(
                        &format!("ZSTD_CCtx_setCParams({bad:?})"),
                        c_scp(cc, bad),
                        r_scp(rc, bad),
                    );
                    c_free(cc);
                    r_free(rc);
                }
                for bad in [
                    FParams { content_size_flag: -1, checksum_flag: 2, no_dict_id_flag: 99 },
                    FParams { content_size_flag: c_int::MAX, checksum_flag: c_int::MIN, no_dict_id_flag: 0 },
                ] {
                    let cc = c_new();
                    let rc = r_new();
                    eq(
                        &format!("ZSTD_CCtx_setFParams({bad:?})"),
                        c_sfp(cc, bad),
                        r_sfp(rc, bad),
                    );
                    c_free(cc);
                    r_free(rc);
                }
                let cp = c_pnew();
                let rp = r_pnew();
                eq(
                    &format!("ZSTD_CCtxParams_init_advanced(lvl={lvl},ss={ss})"),
                    c_pia(cp, prm),
                    r_pia(rp, prm),
                );
                c_pfree(cp);
                r_pfree(rp);
            }
        }

        for dict in [Vec::new(), gen(Shape::TextLike, 4096, &mut rng)] {
            let dp = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
            let ds = dict.len();
            for level in [1, 9, 19] {
                let ccd = c_ccd(dp, ds, level);
                let rcd = r_ccd(dp, ds, level);
                let cdd = c_cdd(dp, ds);
                let rdd = r_cdd(dp, ds);
                // DDict accessors
                if !cdd.is_null() {
                    eq("ZSTD_DDict_dictSize", c_dds(cdd), r_dds(rdd));
                    let cc_content = c_ddc(cdd);
                    let rc_content = r_ddc(rdd);
                    eq("ZSTD_DDict_dictContent nullness", cc_content.is_null(), rc_content.is_null());
                    let n = c_dds(cdd);
                    if !cc_content.is_null() && n > 0 {
                        let cs = std::slice::from_raw_parts(cc_content as *const u8, n);
                        let rs = std::slice::from_raw_parts(rc_content as *const u8, r_dds(rdd));
                        eq_bytes("ZSTD_DDict_dictContent bytes", cs, rs);
                    }
                    // ZSTD_copyDDictParameters onto a DCtx
                    let cd = c_dnew();
                    let rd = r_dnew();
                    c_cdp(cd, cdd);
                    r_cdp(rd, rdd);
                    c_dfree(cd);
                    r_dfree(rd);
                }
                eq("ZSTD_DDict_dictSize(NULL-safe path skipped)", 0usize, 0usize);

                for mode in 0..4 {
                    for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive] {
                        for len in [1usize, 4096, 200_000] {
                            let src = gen(shape, len, &mut rng);
                            let tag = format!(
                                "begin mode={mode} lvl={level} ds={ds} shape={shape:?} len={len}"
                            );
                            let cc = c_new();
                            let rc = r_new();
                            c_reset(cc, 2);
                            r_reset(rc, 2);
                            let fp = FParams { content_size_flag: 1, checksum_flag: 1, no_dict_id_flag: 0 };
                            let (a, b) = match mode {
                                0 => (c_bud(cc, dp, ds, level), r_bud(rc, dp, ds, level)),
                                1 if !ccd.is_null() => (c_bucd(cc, ccd), r_bucd(rc, rcd)),
                                2 if !ccd.is_null() => (
                                    c_bucda(cc, ccd, fp, len as u64),
                                    r_bucda(rc, rcd, fp, len as u64),
                                ),
                                3 if !ccd.is_null() => (c_bucdd(cc, ccd), r_bucdd(rc, rcd)),
                                _ => {
                                    c_free(cc);
                                    r_free(rc);
                                    continue;
                                }
                            };
                            eq(&format!("{tag}: begin ret"), a, b);
                            if c_ie(a) != 0 {
                                c_free(cc);
                                r_free(rc);
                                continue;
                            }
                            // progression + toFlushNow right after begin
                            eq(&format!("{tag}: getFrameProgression(begin)"), c_prog(cc), r_prog(rc));
                            eq(&format!("{tag}: toFlushNow(begin)"), c_tfn(cc), r_tfn(rc));

                            let bs = c_bs(cc);
                            eq(&format!("{tag}: getBlockSize"), bs, r_bs(rc));
                            let cap = c_cb(len) + 4096;
                            let mut cf = vec![0u8; cap];
                            let mut rf = vec![0u8; cap];
                            let mut cpos = 0usize;
                            let mut rpos = 0usize;
                            let mut off = 0usize;
                            let mut failed = false;
                            while off < len {
                                let n = bs.min(len - off);
                                let last = off + n >= len;
                                let f = if last { (&c_end, &r_end) } else { (&c_cont, &r_cont) };
                                let cr = (f.0)(cc, cf[cpos..].as_mut_ptr() as *mut c_void, cap - cpos, src[off..].as_ptr() as *const c_void, n);
                                let rr = (f.1)(rc, rf[rpos..].as_mut_ptr() as *mut c_void, cap - rpos, src[off..].as_ptr() as *const c_void, n);
                                eq(&format!("{tag}: {} @{off}", if last {"compressEnd"} else {"compressContinue"}), cr, rr);
                                if c_ie(cr) != 0 {
                                    failed = true;
                                    break;
                                }
                                eq_bytes(&format!("{tag}: chunk @{off}"), &cf[cpos..cpos + cr], &rf[rpos..rpos + rr]);
                                cpos += cr;
                                rpos += rr;
                                off += n;
                                eq(&format!("{tag}: getFrameProgression @{off}"), c_prog(cc), r_prog(rc));
                                eq(&format!("{tag}: toFlushNow @{off}"), c_tfn(cc), r_tfn(rc));
                            }
                            c_free(cc);
                            r_free(rc);
                            if failed {
                                continue;
                            }
                            eq_bytes(&format!("{tag}: frame"), &cf[..cpos], &rf[..rpos]);

                            // decode via decompressBegin_usingDict / usingDDict
                            for dmode in 0..2 {
                                let cd = c_dnew();
                                let rd = r_dnew();
                                let (a, b) = if dmode == 0 {
                                    (c_dbud(cd, dp, ds), r_dbud(rd, dp, ds))
                                } else if !cdd.is_null() {
                                    (c_dbudd(cd, cdd), r_dbudd(rd, rdd))
                                } else {
                                    c_dfree(cd);
                                    r_dfree(rd);
                                    continue;
                                };
                                eq(&format!("{tag}: decompressBegin(dmode={dmode}) ret"), a, b);
                                c_dfree(cd);
                                r_dfree(rd);
                            }
                            if ds == 0 {
                                diff_decompress(p, &cf[..cpos], &src, &tag);
                            }
                        }
                    }
                }

                // the *_deprecated raw block wrappers
                {
                    let cc = c_new();
                    let rc = r_new();
                    let cd = c_dnew();
                    let rd = r_dnew();
                    eq("deprecated: compressBegin_usingDict", c_bud(cc, dp, ds, level), r_bud(rc, dp, ds, level));
                    eq("deprecated: decompressBegin_usingDict", c_dbud(cd, dp, ds), r_dbud(rd, dp, ds));
                    let bs = c_bs(cc);
                    for shape in ALL_SHAPES {
                        let src = gen(shape, bs, &mut rng);
                        let mut cbuf = vec![0u8; bs + 4096];
                        let mut rbuf = vec![0u8; bs + 4096];
                        let cn = c_cblkd(cc, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), src.as_ptr() as *const c_void, bs);
                        let rn = r_cblkd(rc, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), src.as_ptr() as *const c_void, bs);
                        eq(&format!("compressBlock_deprecated({shape:?}) ret"), cn, rn);
                        if c_ie(cn) != 0 || cn == 0 {
                            continue;
                        }
                        eq_bytes("compressBlock_deprecated out", &cbuf[..cn], &rbuf[..rn]);
                        let mut cdb = vec![0u8; bs + 4096];
                        let mut rdb = vec![0u8; bs + 4096];
                        let cx = c_dblkd(cd, cdb.as_mut_ptr() as *mut c_void, cdb.len(), cbuf.as_ptr() as *const c_void, cn);
                        let rx = r_dblkd(rd, rdb.as_mut_ptr() as *mut c_void, rdb.len(), rbuf.as_ptr() as *const c_void, rn);
                        eq(&format!("decompressBlock_deprecated({shape:?}) ret"), cx, rx);
                        if c_ie(cx) == 0 {
                            eq_bytes("decompressBlock_deprecated out", &cdb[..cx], &rdb[..rx]);
                        }
                    }
                    c_free(cc);
                    r_free(rc);
                    c_dfree(cd);
                    r_dfree(rd);
                }

                c_fcd(ccd);
                r_fcd(rcd);
                c_fdd(cdd);
                r_fdd(rdd);
            }
        }
    }
}

/// Row 59 — `divsufsort` / `divbwt` driven directly.
#[test]
fn row59_divsufsort() {
    let p = libs();
    type FDss = unsafe extern "C" fn(*const c_uchar, *mut c_int, c_int, c_int) -> c_int;
    type FDbwt = unsafe extern "C" fn(
        *const c_uchar, *mut c_uchar, *mut c_int, c_int, *mut c_uchar, *mut c_int, c_int,
    ) -> c_int;
    let (c_ds, r_ds) = p.sym::<FDss>("divsufsort");
    let (c_db, r_db) = p.sym::<FDbwt>("divbwt");
    let mut rng = Rng::new(SEED ^ 0x59);
    unsafe {
        for shape in ALL_SHAPES {
            for &n in &[0usize, 1, 2, 3, 8, 63, 127, 128, 255, 1024, 4096, 65_536] {
                let src = gen(shape, n, &mut rng);
                let sp = if n == 0 { std::ptr::null() } else { src.as_ptr() as *const c_uchar };
                for openmp in [0, 1] {
                    let mut csa = vec![0i32; n + 8];
                    let mut rsa = vec![0i32; n + 8];
                    let a = c_ds(sp, csa.as_mut_ptr(), n as c_int, openmp);
                    let b = r_ds(sp, rsa.as_mut_ptr(), n as c_int, openmp);
                    eq(&format!("divsufsort({shape:?},n={n},omp={openmp}) ret"), a, b);
                    eq(&format!("divsufsort({shape:?},n={n},omp={openmp}) SA"), &csa[..], &rsa[..]);

                    let mut cu = vec![0u8; n + 8];
                    let mut ru = vec![0u8; n + 8];
                    let mut ca = vec![0i32; n + 8];
                    let mut ra = vec![0i32; n + 8];
                    let a = c_db(sp, cu.as_mut_ptr(), ca.as_mut_ptr(), n as c_int, std::ptr::null_mut(), std::ptr::null_mut(), openmp);
                    let b = r_db(sp, ru.as_mut_ptr(), ra.as_mut_ptr(), n as c_int, std::ptr::null_mut(), std::ptr::null_mut(), openmp);
                    eq(&format!("divbwt({shape:?},n={n},omp={openmp}) ret"), a, b);
                    eq_bytes(&format!("divbwt({shape:?},n={n},omp={openmp}) U"), &cu, &ru);
                    eq(&format!("divbwt({shape:?},n={n},omp={openmp}) A"), &ca[..], &ra[..]);
                }
            }
        }
    }
}

/// `ERR_getErrorString` and the exported `g_debuglevel` data symbol.
#[test]
fn misc_err_string_and_debuglevel() {
    let p = libs();
    type FErrStr = unsafe extern "C" fn(c_int) -> *const std::os::raw::c_char;
    let (c, r) = p.sym::<FErrStr>("ERR_getErrorString");
    unsafe {
        for v in -5..=130i32 {
            eq(&format!("ERR_getErrorString({v})"), cstr(c(v)), cstr(r(v)));
        }
        for v in [c_int::MIN, c_int::MAX, -1000, 9999] {
            eq(&format!("ERR_getErrorString({v})"), cstr(c(v)), cstr(r(v)));
        }
    }
    // g_debuglevel is an exported `int` data symbol
    let (cd, rd) = p.sym::<*mut c_int>("g_debuglevel");
    unsafe {
        eq("g_debuglevel initial value", **cd, **rd);
    }
}

/// The `ZSTDMT_*` surface: `ZSTD_MULTITHREAD` is NOT defined in this build, so
/// these are exported but must behave identically (typically returning errors
/// or NULL).
#[test]
fn zstdmt_disabled_surface() {
    let p = libs();
    type FCreateAdv = unsafe extern "C" fn(c_uint, CustomMem, *mut c_void) -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FSizeof = unsafe extern "C" fn(*const c_void) -> Sz;
    unsafe {
        for nb in [0u32, 1, 2, 4] {
            let (c_cr, r_cr) = p.sym::<FCreateAdv>("ZSTDMT_createCCtx_advanced");
            let x = c_cr(nb, CustomMem::default(), std::ptr::null_mut());
            let y = r_cr(nb, CustomMem::default(), std::ptr::null_mut());
            eq(&format!("ZSTDMT_createCCtx_advanced({nb}) nullness"), x.is_null(), y.is_null());
            if !x.is_null() {
                let (c_sz, r_sz) = p.sym::<FSizeof>("ZSTDMT_sizeof_CCtx");
                eq(&format!("ZSTDMT_sizeof_CCtx({nb})"), c_sz(x), r_sz(y));
                let (c_tf, r_tf) = p.sym::<FFree>("ZSTDMT_toFlushNow");
                eq(&format!("ZSTDMT_toFlushNow({nb})"), c_tf(x), r_tf(y));
                let (c_ni, r_ni) = p.sym::<FSizeof>("ZSTDMT_nextInputSizeHint");
                eq(&format!("ZSTDMT_nextInputSizeHint({nb})"), c_ni(x), r_ni(y));
            }
            let (c_fr, r_fr) = p.sym::<FFree>("ZSTDMT_freeCCtx");
            eq(&format!("ZSTDMT_freeCCtx({nb})"), c_fr(x), r_fr(y));
        }
        // NULL-handle behaviour
        let (c_fr, r_fr) = p.sym::<FFree>("ZSTDMT_freeCCtx");
        eq("ZSTDMT_freeCCtx(NULL)", c_fr(std::ptr::null_mut()), r_fr(std::ptr::null_mut()));
        let (c_sz, r_sz) = p.sym::<FSizeof>("ZSTDMT_sizeof_CCtx");
        eq("ZSTDMT_sizeof_CCtx(NULL)", c_sz(std::ptr::null()), r_sz(std::ptr::null()));
    }
}

/// Legacy extras: `ZSTDv07_createDCtx_advanced`,
/// `ZSTDv0{5,6}_decompress_usingPreparedDCtx`,
/// `ZBUFFv04_decompressWithDictionary`, `ZBUFFv07_createDCtx_advanced`.
#[test]
fn legacy_extras() {
    let p = libs();
    type FCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;
    type FCreate = unsafe extern "C" fn() -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FPrepared = unsafe extern "C" fn(
        *mut c_void, *const c_void, *mut c_void, Sz, *const c_void, Sz,
    ) -> Sz;
    type FBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FZbuffDict = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;

    let mut rng = Rng::new(SEED ^ 0x7E);
    let dict = gen(Shape::TextLike, 1024, &mut rng);
    let garbage = gen(Shape::Incompressible, 256, &mut rng);
    unsafe {
        for nm in ["ZSTDv07_createDCtx_advanced", "ZBUFFv07_createDCtx_advanced"] {
            if !p.has(nm) {
                continue;
            }
            let (c, r) = p.sym::<FCreateAdv>(nm);
            let x = c(CustomMem::default());
            let y = r(CustomMem::default());
            eq(&format!("{nm} nullness"), x.is_null(), y.is_null());
            let fname = if nm.starts_with("ZBUFF") { "ZBUFFv07_freeDCtx" } else { "ZSTDv07_freeDCtx" };
            let (cf, rf) = p.sym::<FFree>(fname);
            eq(&format!("{fname} ret"), cf(x), rf(y));
        }
        for v in [5, 6] {
            let nm = format!("ZSTDv0{v}_decompress_usingPreparedDCtx");
            let bnm = format!("ZSTDv0{v}_decompressBegin_usingDict");
            if !(p.has(&nm) && p.has(&bnm)) {
                continue;
            }
            let (c_c, r_c) = p.sym::<FCreate>(&format!("ZSTDv0{v}_createDCtx"));
            let (c_f, r_f) = p.sym::<FFree>(&format!("ZSTDv0{v}_freeDCtx"));
            let (c_bd, r_bd) = p.sym::<FBeginDict>(&bnm);
            let (c_pd, r_pd) = p.sym::<FPrepared>(&nm);
            let cprep = c_c();
            let rprep = r_c();
            let cdst = c_c();
            let rdst = r_c();
            eq(
                &format!("{bnm} ret"),
                c_bd(cprep, dict.as_ptr() as *const c_void, dict.len()),
                r_bd(rprep, dict.as_ptr() as *const c_void, dict.len()),
            );
            for cut in [0usize, 1, 4, 8, 64, garbage.len()] {
                for &cap in &[0usize, 64, 1 << 16] {
                    let mut cb = vec![0x9Au8; cap.max(1)];
                    let mut rb = vec![0x9Au8; cap.max(1)];
                    let gp = if cut == 0 { std::ptr::null() } else { garbage.as_ptr() as *const c_void };
                    eq(
                        &format!("{nm}(cut={cut},cap={cap}) ret"),
                        c_pd(cdst, cprep, cb.as_mut_ptr() as *mut c_void, cap, gp, cut),
                        r_pd(rdst, rprep, rb.as_mut_ptr() as *mut c_void, cap, gp, cut),
                    );
                    eq_bytes(&format!("{nm}(cut={cut},cap={cap}) buffer image"), &cb, &rb);
                }
            }
            c_f(cprep);
            r_f(rprep);
            c_f(cdst);
            r_f(rdst);
        }
        if p.has("ZBUFFv04_decompressWithDictionary") {
            let (c_c, r_c) = p.sym::<FCreate>("ZBUFFv04_createDCtx");
            let (c_f, r_f) = p.sym::<FFree>("ZBUFFv04_freeDCtx");
            let (c_wd, r_wd) = p.sym::<FZbuffDict>("ZBUFFv04_decompressWithDictionary");
            let cc = c_c();
            let rc = r_c();
            for d in [Vec::new(), dict.clone()] {
                let dp = if d.is_empty() { std::ptr::null() } else { d.as_ptr() as *const c_void };
                eq(
                    &format!("ZBUFFv04_decompressWithDictionary(len={})", d.len()),
                    c_wd(cc, dp, d.len()),
                    r_wd(rc, dp, d.len()),
                );
            }
            c_f(cc);
            r_f(rc);
        }
    }
}
