//! Phase B: advanced dictionary construction and attach/copy/load heuristics
//! (CONFIGS.md rows 128-131, 132-137, 141-143, 145).
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnPledged = unsafe extern "C" fn(*mut c_void, u64) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnChunk = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void;
type FnCreateCDictAdv =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CParams, CustomMem) -> *mut c_void;
type FnCreateCDictAdv2 =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, *const c_void, CustomMem) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const u8, usize, c_int, c_int, CustomMem) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnRefDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnDictContent = unsafe extern "C" fn(*const c_void) -> *const u8;
type FnDictSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnCopyDDictParams = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnCompressUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void, *mut u8, usize, *const u8, usize, *const c_void, FParams,
) -> usize;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> CParams;
type FnGetCParamsFromCDict = unsafe extern "C" fn(*const c_void) -> CParams;
type FnCParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnDictID = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnSizeofDict = unsafe extern "C" fn(*const c_void) -> usize;
type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, c_uint) -> usize;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CParams {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FParams {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomMem {
    pub alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;
const C_HASHLOG: c_int = 102;
const C_CHAINLOG: c_int = 103;
const C_STRATEGY: c_int = 107;
const C_FORCEMAXWINDOW: c_int = 1000;
const C_FORCEATTACHDICT: c_int = 1001;
const C_ENABLEDDS: c_int = 1005;
const C_PREFETCH: c_int = 1013;
const D_REFMULTIPLE: c_int = 1003;
const NOMEM: CustomMem = CustomMem { alloc: None, free: None, opaque: std::ptr::null_mut() };

fn train_dict(rng: &mut Rng, shape: Shape, cap: usize) -> Vec<u8> {
    let (ct, rt) = unsafe { pair::<FnTrain>("ZDICT_trainFromBuffer") };
    let nb = 350;
    let mut samples = Vec::new();
    let mut sizes = Vec::new();
    for _ in 0..nb {
        let s = gen(shape, rng.range(64, 500), rng);
        sizes.push(s.len());
        samples.extend_from_slice(&s);
    }
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { ct(a.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    let y = unsafe { rt(b.as_mut_ptr(), cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint) };
    assert_eq!(x, y, "ZDICT_trainFromBuffer");
    assert!(!is_error(x), "train failed: {}", err_code(x));
    assert_bytes_eq("trained dict", &a[..x], &b[..y]);
    a.truncate(x);
    a
}

fn decompress_with_ddict(cdd: *mut c_void, rdd: *mut c_void, frame: &[u8], expect: &[u8],
                         ctx: &str) {
    type FnUsingDDict =
        unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cd, rd) = unsafe { pair::<FnUsingDDict>("ZSTD_decompress_usingDDict") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    let cap = expect.len() + 64;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let x = unsafe { cd(cc, a.as_mut_ptr(), cap, frame.as_ptr(), frame.len(), cdd) };
    let y = unsafe { rd(rc, b.as_mut_ptr(), cap, frame.as_ptr(), frame.len(), rdd) };
    assert_eq!(x, y, "{ctx}: decompress_usingDDict");
    if !is_error(x) {
        assert_bytes_eq(&format!("{ctx}: dec bytes"), &a[..x], &b[..y]);
        assert_bytes_eq(&format!("{ctx}: dec vs orig"), &a[..x], expect);
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

/// Rows 128-131: createCDict_advanced / _advanced2 / compress_usingCDict_advanced.
#[test]
fn cfg_cdict_advanced_matrix() {
    let (cca, rca) = unsafe { pair::<FnCreateCDictAdv>("ZSTD_createCDict_advanced") };
    let (cca2, rca2) = unsafe { pair::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2") };
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (cua, rua) =
        unsafe { pair::<FnCompressUsingCDictAdv>("ZSTD_compress_usingCDict_advanced") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cgc, rgc) = unsafe { pair::<FnGetCParams>("ZSTD_getCParams") };
    let (cgcd, rgcd) = unsafe { pair::<FnGetCParamsFromCDict>("ZSTD_getCParamsFromCDict") };
    let (csz, rsz) = unsafe { pair::<FnSizeofDict>("ZSTD_sizeof_CDict") };
    let (cid, rid) = unsafe { pair::<FnDictID>("ZSTD_getDictID_fromCDict") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (cpi, rpi) = unsafe { pair::<FnCParamsInit>("ZSTD_CCtxParams_init") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0x0D10);
    let trained = train_dict(&mut rng, Shape::Text, 8192);
    let raw = gen(Shape::Text, 8192, &mut rng);
    let src = gen(Shape::Text, 40000, &mut rng);
    let cap = unsafe { cb(src.len()) } + 64;
    let cctx = unsafe { cn() };
    let rctx = unsafe { rn() };
    let cparams_obj = unsafe { cpn() };
    let rparams_obj = unsafe { rpn() };

    for (dname, dict) in [("trained", &trained), ("raw", &raw)] {
        let cdict_dd = unsafe { cdd(dict.as_ptr(), dict.len()) };
        let rdict_dd = unsafe { rdd(dict.as_ptr(), dict.len()) };
        for dlm in 0..2 {
            for dct in 0..3 {
                for &lvl in &[1, 9, 19] {
                    let cp = unsafe { cgc(lvl, src.len() as u64, dict.len()) };
                    let rp = unsafe { rgc(lvl, src.len() as u64, dict.len()) };
                    assert_eq!(cp, rp, "getCParams");
                    let ctx = format!("cdict_adv {dname} dlm={dlm} dct={dct} lvl={lvl}");
                    let a = unsafe { cca(dict.as_ptr(), dict.len(), dlm, dct, cp, NOMEM) };
                    let b = unsafe { rca(dict.as_ptr(), dict.len(), dlm, dct, rp, NOMEM) };
                    assert_eq!(a.is_null(), b.is_null(), "{ctx}: createCDict_advanced null-ness");
                    if !a.is_null() {
                        assert_eq!(unsafe { csz(a) }, unsafe { rsz(b) }, "{ctx}: sizeof_CDict");
                        assert_eq!(unsafe { cid(a) }, unsafe { rid(b) }, "{ctx}: dictID");
                        assert_eq!(
                            unsafe { cgcd(a) },
                            unsafe { rgcd(b) },
                            "{ctx}: getCParamsFromCDict"
                        );
                        for fp in [
                            FParams { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 1 },
                            FParams { contentSizeFlag: 0, checksumFlag: 0, noDictIDFlag: 0 },
                            FParams { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 },
                        ] {
                            let mut o1 = vec![0u8; cap];
                            let mut o2 = vec![0u8; cap];
                            let x = unsafe {
                                cua(cctx, o1.as_mut_ptr(), cap, src.as_ptr(), src.len(), a, fp)
                            };
                            let y = unsafe {
                                rua(rctx, o2.as_mut_ptr(), cap, src.as_ptr(), src.len(), b, fp)
                            };
                            assert_eq!(x, y, "{ctx}: compress_usingCDict_advanced({fp:?})");
                            if !is_error(x) {
                                assert_bytes_eq(&ctx, &o1[..x], &o2[..y]);
                                o1.truncate(x);
                                decompress_with_ddict(cdict_dd, rdict_dd, &o1, &src, &ctx);
                            }
                        }
                        unsafe {
                            cfd(a);
                            rfd(b);
                        }
                    }
                    // _advanced2 with a CCtx_params object
                    assert_eq!(unsafe { cpi(cparams_obj, lvl) }, unsafe { rpi(rparams_obj, lvl) });
                    let a =
                        unsafe { cca2(dict.as_ptr(), dict.len(), dlm, dct, cparams_obj, NOMEM) };
                    let b =
                        unsafe { rca2(dict.as_ptr(), dict.len(), dlm, dct, rparams_obj, NOMEM) };
                    assert_eq!(a.is_null(), b.is_null(), "{ctx}: createCDict_advanced2 null-ness");
                    if !a.is_null() {
                        assert_eq!(unsafe { csz(a) }, unsafe { rsz(b) }, "{ctx}: sizeof adv2");
                        let mut o1 = vec![0u8; cap];
                        let mut o2 = vec![0u8; cap];
                        let fp = FParams { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 };
                        let x = unsafe {
                            cua(cctx, o1.as_mut_ptr(), cap, src.as_ptr(), src.len(), a, fp)
                        };
                        let y = unsafe {
                            rua(rctx, o2.as_mut_ptr(), cap, src.as_ptr(), src.len(), b, fp)
                        };
                        assert_eq!(x, y, "{ctx}: compress with adv2 cdict");
                        if !is_error(x) {
                            assert_bytes_eq(&format!("{ctx} adv2"), &o1[..x], &o2[..y]);
                        }
                        unsafe {
                            cfd(a);
                            rfd(b);
                        }
                    }
                    // plain createCDict at the same level must match too
                    let a = unsafe { ccd(dict.as_ptr(), dict.len(), lvl) };
                    let b = unsafe { rcd(dict.as_ptr(), dict.len(), lvl) };
                    assert_eq!(a.is_null(), b.is_null());
                    if !a.is_null() {
                        assert_eq!(unsafe { csz(a) }, unsafe { rsz(b) });
                        unsafe {
                            cfd(a);
                            rfd(b);
                        }
                    }
                }
            }
        }
        unsafe {
            cfdd(cdict_dd);
            rfdd(rdict_dd);
        }
    }
    unsafe {
        cf(cctx);
        rf(rctx);
        cpf(cparams_obj);
        rpf(rparams_obj);
    }
}

/// Row 145: createDDict_byReference / _advanced + DDict accessors.
#[test]
fn cfg_ddict_advanced_matrix() {
    let (cda, rda) = unsafe { pair::<FnCreateDDictAdv>("ZSTD_createDDict_advanced") };
    let (cdr, rdr) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict_byReference") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cdc, rdc) = unsafe { pair::<FnDictContent>("ZSTD_DDict_dictContent") };
    let (cds, rds) = unsafe { pair::<FnDictSize>("ZSTD_DDict_dictSize") };
    let (ccp, rcp) = unsafe { pair::<FnCopyDDictParams>("ZSTD_copyDDictParameters") };
    let (csz, rsz) = unsafe { pair::<FnSizeofDict>("ZSTD_sizeof_DDict") };
    let (cid, rid) = unsafe { pair::<FnDictID>("ZSTD_getDictID_fromDDict") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdec, rdec) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (ccn, rcn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (ccf, rcf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cld, rld) =
        unsafe { pair::<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize>("ZSTD_CCtx_loadDictionary") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0x0D20);
    let trained = train_dict(&mut rng, Shape::Text, 8192);
    let raw = gen(Shape::Text, 4096, &mut rng);
    let src = gen(Shape::Text, 30000, &mut rng);

    // dict-compressed frame
    let cc = unsafe { ccn() };
    let rc = unsafe { rcn() };
    assert_eq!(unsafe { cld(cc, trained.as_ptr(), trained.len()) },
               unsafe { rld(rc, trained.as_ptr(), trained.len()) });
    let cap = unsafe { cb(src.len()) } + 64;
    let mut f1 = vec![0u8; cap];
    let mut f2 = vec![0u8; cap];
    let x = unsafe { c2(cc, f1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let y = unsafe { r2(rc, f2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(x, y);
    assert_bytes_eq("frame", &f1[..x], &f2[..y]);
    f1.truncate(x);
    unsafe {
        ccf(cc);
        rcf(rc);
    }

    for (dname, dict) in [("trained", &trained), ("raw", &raw)] {
        for dlm in 0..2 {
            for dct in 0..3 {
                let ctx = format!("ddict_adv {dname} dlm={dlm} dct={dct}");
                let a = unsafe { cda(dict.as_ptr(), dict.len(), dlm, dct, NOMEM) };
                let b = unsafe { rda(dict.as_ptr(), dict.len(), dlm, dct, NOMEM) };
                assert_eq!(a.is_null(), b.is_null(), "{ctx}: null-ness");
                if a.is_null() {
                    continue;
                }
                assert_eq!(unsafe { cds(a) }, unsafe { rds(b) }, "{ctx}: DDict_dictSize");
                let n = unsafe { cds(a) };
                let ca = unsafe { cdc(a) };
                let cbp = unsafe { rdc(b) };
                if n > 0 && !ca.is_null() && !cbp.is_null() {
                    let sa = unsafe { std::slice::from_raw_parts(ca, n) };
                    let sb = unsafe { std::slice::from_raw_parts(cbp, n) };
                    assert_bytes_eq(&format!("{ctx}: DDict_dictContent"), sa, sb);
                }
                assert_eq!(unsafe { csz(a) }, unsafe { rsz(b) }, "{ctx}: sizeof_DDict");
                assert_eq!(unsafe { cid(a) }, unsafe { rid(b) }, "{ctx}: dictID");
                // copyDDictParameters into a DCtx, then decode
                let dc = unsafe { cn() };
                let dr = unsafe { rn() };
                unsafe {
                    ccp(dc, a);
                    rcp(dr, b);
                }
                let cap = src.len() + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let x = unsafe { cdec(dc, o1.as_mut_ptr(), cap, f1.as_ptr(), f1.len()) };
                let y = unsafe { rdec(dr, o2.as_mut_ptr(), cap, f2.as_ptr(), f1.len()) };
                assert_eq!(x, y, "{ctx}: decode after copyDDictParameters");
                if !is_error(x) {
                    assert_bytes_eq(&format!("{ctx}: bytes"), &o1[..x], &o2[..y]);
                }
                unsafe {
                    cf(dc);
                    rf(dr);
                    cfd(a);
                    rfd(b);
                }
            }
        }
        // byReference vs byCopy must behave identically
        let a1 = unsafe { cdr(dict.as_ptr(), dict.len()) };
        let b1 = unsafe { rdr(dict.as_ptr(), dict.len()) };
        let a2 = unsafe { cdd(dict.as_ptr(), dict.len()) };
        let b2 = unsafe { rdd(dict.as_ptr(), dict.len()) };
        assert_eq!(a1.is_null(), b1.is_null());
        assert_eq!(a2.is_null(), b2.is_null());
        if !a1.is_null() {
            assert_eq!(unsafe { cds(a1) }, unsafe { rds(b1) });
            assert_eq!(unsafe { csz(a1) }, unsafe { rsz(b1) });
            decompress_with_ddict(a1, b1, &f1, &src, &format!("{dname} byRef"));
            unsafe {
                cfd(a1);
                rfd(b1);
            }
        }
        if !a2.is_null() {
            decompress_with_ddict(a2, b2, &f1, &src, &format!("{dname} byCopy"));
            unsafe {
                cfd(a2);
                rfd(b2);
            }
        }
    }
}

/// Rows 132-137: the CDict attach/copy/load heuristic, pledged size cutoffs,
/// forceAttachDict and forceMaxWindow.
#[test]
fn cfg_attach_cutoffs() {
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (crc, rrc) = unsafe { pair::<FnRefDict>("ZSTD_CCtx_refCDict") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cp, rp) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x0D30);
    let dict = train_dict(&mut rng, Shape::Text, 65536);
    let cddict = unsafe { cdd(dict.as_ptr(), dict.len()) };
    let rddict = unsafe { rdd(dict.as_ptr(), dict.len()) };

    // cutoffs: 8 KB (fast), 16 KB (dfast), 32 KB (greedy/lazy/lazy2)
    let sizes: [usize; 10] =
        [1, 4095, 8191, 8192, 8193, 16383, 16384, 32768, 32769, 200000];
    for &strategy in &[1, 2, 3, 4, 5, 7, 9] {
        for &attach in &[0, 1, 2, 3] {
            for &fmw in &[0, 1] {
                for &known in &[true, false] {
                    for &size in &sizes {
                        let src = gen(Shape::Text, size, &mut rng);
                        let ctx = format!(
                            "attach strat={strategy} pref={attach} fmw={fmw} known={known} n={size}"
                        );
                        let cdict = unsafe { ccd(dict.as_ptr(), dict.len(), 6) };
                        let rdict = unsafe { rcd(dict.as_ptr(), dict.len(), 6) };
                        let cc = unsafe { cn() };
                        let rc = unsafe { rn() };
                        let mut bad = false;
                        for (p, v) in [
                            (C_STRATEGY, strategy),
                            (C_FORCEATTACHDICT, attach),
                            (C_FORCEMAXWINDOW, fmw),
                        ] {
                            let x = unsafe { cs(cc, p, v) };
                            let y = unsafe { rs(rc, p, v) };
                            assert_eq!(x, y, "{ctx}: setParameter({p},{v})");
                            bad |= is_error(x);
                        }
                        if known {
                            let x = unsafe { cp(cc, size as u64) };
                            let y = unsafe { rp(rc, size as u64) };
                            assert_eq!(x, y, "{ctx}: pledged");
                            bad |= is_error(x);
                        }
                        let x = unsafe { crc(cc, cdict) };
                        let y = unsafe { rrc(rc, rdict) };
                        assert_eq!(x, y, "{ctx}: refCDict");
                        bad |= is_error(x);
                        if !bad {
                            let cap = unsafe { cb(size) } + 64;
                            let mut o1 = vec![0u8; cap];
                            let mut o2 = vec![0u8; cap];
                            let x =
                                unsafe { c2(cc, o1.as_mut_ptr(), cap, src.as_ptr(), size) };
                            let y =
                                unsafe { r2(rc, o2.as_mut_ptr(), cap, src.as_ptr(), size) };
                            assert_eq!(x, y, "{ctx}: compress2 (C err {})", err_code(x));
                            if !is_error(x) {
                                assert_bytes_eq(&ctx, &o1[..x], &o2[..y]);
                                o1.truncate(x);
                                decompress_with_ddict(cddict, rddict, &o1, &src, &ctx);
                            }
                        }
                        unsafe {
                            cf(cc);
                            rf(rc);
                            cfd(cdict);
                            rfd(rdict);
                        }
                    }
                }
            }
        }
    }
    unsafe {
        cfdd(cddict);
        rfdd(rddict);
    }
}

/// Rows 141-142: dedicated dict search (exercised through the public flag; the
/// internal ZSTD_dedicatedDictSearch_lazy_loadDictionary is reached from here).
#[test]
fn cfg_dedicated_dict_search() {
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cca, rca) = unsafe { pair::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (crc, rrc) = unsafe { pair::<FnRefDict>("ZSTD_CCtx_refCDict") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cpn, rpn) = unsafe { pair::<FnNew>("ZSTD_createCCtxParams") };
    let (cpf, rpf) = unsafe { pair::<FnFree>("ZSTD_freeCCtxParams") };
    let (cpi, rpi) = unsafe { pair::<FnCParamsInit>("ZSTD_CCtxParams_init") };
    let (cps, rps) = unsafe { pair::<FnSetParam>("ZSTD_CCtxParams_setParameter") };
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x0D40);
    let dict = train_dict(&mut rng, Shape::Text, 65536);
    let cddict = unsafe { cdd(dict.as_ptr(), dict.len()) };
    let rddict = unsafe { rdd(dict.as_ptr(), dict.len()) };

    for &dds in &[0, 1] {
        for &strategy in &[1, 3, 4, 5, 6, 7] {
            for &(hashLog, chainLog) in &[(20, 16), (20, 24), (20, 25), (16, 16), (25, 15)] {
                for &size in &[100usize, 8192, 60000] {
                    let src = gen(Shape::Text, size, &mut rng);
                    let ctx = format!(
                        "dds={dds} strat={strategy} h={hashLog} c={chainLog} n={size}"
                    );
                    // build the CDict through a CCtx_params object so the DDS flag
                    // is part of the dictionary construction
                    let po_c = unsafe { cpn() };
                    let po_r = unsafe { rpn() };
                    assert_eq!(unsafe { cpi(po_c, 6) }, unsafe { rpi(po_r, 6) });
                    for (p, v) in [
                        (C_ENABLEDDS, dds),
                        (C_STRATEGY, strategy),
                        (C_HASHLOG, hashLog),
                        (C_CHAINLOG, chainLog),
                    ] {
                        assert_eq!(
                            unsafe { cps(po_c, p, v) },
                            unsafe { rps(po_r, p, v) },
                            "{ctx}: CCtxParams_setParameter({p},{v})"
                        );
                    }
                    let a = unsafe { cca(dict.as_ptr(), dict.len(), 0, 0, po_c, NOMEM) };
                    let b = unsafe { rca(dict.as_ptr(), dict.len(), 0, 0, po_r, NOMEM) };
                    assert_eq!(a.is_null(), b.is_null(), "{ctx}: createCDict_advanced2");
                    if !a.is_null() {
                        let cc = unsafe { cn() };
                        let rc = unsafe { rn() };
                        let mut bad = false;
                        for (p, v) in [
                            (C_ENABLEDDS, dds),
                            (C_STRATEGY, strategy),
                            (C_HASHLOG, hashLog),
                            (C_CHAINLOG, chainLog),
                        ] {
                            let x = unsafe { cs(cc, p, v) };
                            let y = unsafe { rs(rc, p, v) };
                            assert_eq!(x, y, "{ctx}: setParameter({p},{v})");
                            bad |= is_error(x);
                        }
                        let x = unsafe { crc(cc, a) };
                        let y = unsafe { rrc(rc, b) };
                        assert_eq!(x, y, "{ctx}: refCDict");
                        bad |= is_error(x);
                        if !bad {
                            let cap = unsafe { cb(size) } + 64;
                            let mut o1 = vec![0u8; cap];
                            let mut o2 = vec![0u8; cap];
                            let x = unsafe { c2(cc, o1.as_mut_ptr(), cap, src.as_ptr(), size) };
                            let y = unsafe { r2(rc, o2.as_mut_ptr(), cap, src.as_ptr(), size) };
                            assert_eq!(x, y, "{ctx}: compress2");
                            if !is_error(x) {
                                assert_bytes_eq(&ctx, &o1[..x], &o2[..y]);
                                o1.truncate(x);
                                decompress_with_ddict(cddict, rddict, &o1, &src, &ctx);
                            }
                        }
                        unsafe {
                            cf(cc);
                            rf(rc);
                            cfd(a);
                            rfd(b);
                        }
                    }
                    unsafe {
                        cpf(po_c);
                        rpf(po_r);
                    }
                }
            }
        }
    }
    let _ = (ccd, rcd);
    unsafe {
        cfdd(cddict);
        rfdd(rddict);
    }
}

/// Row 143: d_refMultipleDDicts with several dictionaries.
#[test]
fn cfg_ref_multiple_ddicts() {
    let (cdd, rdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfdd, rfdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (crd, rrd) = unsafe { pair::<FnRefDict>("ZSTD_DCtx_refDDict") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cdec, rdec) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (ccn, rcn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (ccf, rcf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cld, rld) =
        unsafe { pair::<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize>("ZSTD_CCtx_loadDictionary") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x0D50);

    let d1 = train_dict(&mut rng, Shape::Text, 8192);
    let d2 = train_dict(&mut rng, Shape::Repetitive, 8192);
    let src = gen(Shape::Text, 20000, &mut rng);

    // two frames, each with a different dictionary
    let mut frames = Vec::new();
    for d in [&d1, &d2] {
        let cc = unsafe { ccn() };
        let rc = unsafe { rcn() };
        assert_eq!(unsafe { cld(cc, d.as_ptr(), d.len()) },
                   unsafe { rld(rc, d.as_ptr(), d.len()) });
        let cap = unsafe { cb(src.len()) } + 64;
        let mut o1 = vec![0u8; cap];
        let mut o2 = vec![0u8; cap];
        let x = unsafe { c2(cc, o1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let y = unsafe { r2(rc, o2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(x, y);
        assert_bytes_eq("multi-dict frame", &o1[..x], &o2[..y]);
        o1.truncate(x);
        frames.push(o1);
        unsafe {
            ccf(cc);
            rcf(rc);
        }
    }

    for &multi in &[0, 1] {
        let cdd1 = unsafe { cdd(d1.as_ptr(), d1.len()) };
        let rdd1 = unsafe { rdd(d1.as_ptr(), d1.len()) };
        let cdd2 = unsafe { cdd(d2.as_ptr(), d2.len()) };
        let rdd2 = unsafe { rdd(d2.as_ptr(), d2.len()) };
        let dc = unsafe { cn() };
        let dr = unsafe { rn() };
        assert_eq!(unsafe { cs(dc, D_REFMULTIPLE, multi) },
                   unsafe { rs(dr, D_REFMULTIPLE, multi) }, "refMultipleDDicts={multi}");
        assert_eq!(unsafe { crd(dc, cdd1) }, unsafe { rrd(dr, rdd1) }, "refDDict(d1)");
        assert_eq!(unsafe { crd(dc, cdd2) }, unsafe { rrd(dr, rdd2) }, "refDDict(d2)");
        for (i, f) in frames.iter().enumerate() {
            let cap = src.len() + 64;
            let mut o1 = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let x = unsafe { cdec(dc, o1.as_mut_ptr(), cap, f.as_ptr(), f.len()) };
            let y = unsafe { rdec(dr, o2.as_mut_ptr(), cap, f.as_ptr(), f.len()) };
            assert_eq!(x, y, "multi={multi} frame {i}");
            if !is_error(x) {
                assert_bytes_eq(&format!("multi={multi} frame {i}"), &o1[..x], &o2[..y]);
            }
        }
        // a frame referencing an unknown dictID
        let mut bogus = frames[0].clone();
        if bogus.len() > 8 {
            bogus[5] ^= 0xFF;
            bogus[6] ^= 0xFF;
            let cap = src.len() + 64;
            let mut o1 = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let x = unsafe { cdec(dc, o1.as_mut_ptr(), cap, bogus.as_ptr(), bogus.len()) };
            let y = unsafe { rdec(dr, o2.as_mut_ptr(), cap, bogus.as_ptr(), bogus.len()) };
            assert_eq!(x, y, "multi={multi} unknown dictID");
        }
        unsafe {
            cf(dc);
            rf(dr);
            cfdd(cdd1);
            rfdd(rdd1);
            cfdd(cdd2);
            rfdd(rdd2);
        }
    }
}

/// Row 139: a prefix is single-use — the second frame must not use it.
#[test]
fn cfg_prefix_single_use() {
    let (crp, rrp) =
        unsafe { pair::<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize>("ZSTD_CCtx_refPrefix") };
    let (cdrp, rdrp) =
        unsafe { pair::<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize>("ZSTD_DCtx_refPrefix") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cdn, rdn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cdf, rdf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cdec, rdec) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x0D60);
    let prefix = gen(Shape::Text, 16384, &mut rng);

    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    let dc = unsafe { cdn() };
    let dr = unsafe { rdn() };
    for round in 0..4 {
        let src = gen(Shape::Text, rng.range(100, 30000), &mut rng);
        let use_prefix = round % 2 == 0;
        let ctx = format!("prefix round={round} use={use_prefix} n={}", src.len());
        if use_prefix {
            assert_eq!(unsafe { crp(cc, prefix.as_ptr(), prefix.len()) },
                       unsafe { rrp(rc, prefix.as_ptr(), prefix.len()) }, "{ctx}: refPrefix");
        }
        let cap = unsafe { cb(src.len()) } + 64;
        let mut o1 = vec![0u8; cap];
        let mut o2 = vec![0u8; cap];
        let x = unsafe { c2(cc, o1.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let y = unsafe { r2(rc, o2.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(x, y, "{ctx}: compress2");
        assert_bytes_eq(&ctx, &o1[..x], &o2[..y]);
        o1.truncate(x);
        if use_prefix {
            assert_eq!(unsafe { cdrp(dc, prefix.as_ptr(), prefix.len()) },
                       unsafe { rdrp(dr, prefix.as_ptr(), prefix.len()) }, "{ctx}: DCtx refPrefix");
        }
        let cap = src.len() + 64;
        let mut d1 = vec![0u8; cap];
        let mut d2 = vec![0u8; cap];
        let a = unsafe { cdec(dc, d1.as_mut_ptr(), cap, o1.as_ptr(), o1.len()) };
        let b = unsafe { rdec(dr, d2.as_mut_ptr(), cap, o1.as_ptr(), o1.len()) };
        assert_eq!(a, b, "{ctx}: decompress");
        if !is_error(a) {
            assert_bytes_eq(&format!("{ctx} dec"), &d1[..a], &d2[..b]);
            assert_bytes_eq(&format!("{ctx} dec vs orig"), &d1[..a], &src);
        }
    }
    unsafe {
        cf(cc);
        rf(rc);
        cdf(dc);
        rdf(dr);
    }
}
