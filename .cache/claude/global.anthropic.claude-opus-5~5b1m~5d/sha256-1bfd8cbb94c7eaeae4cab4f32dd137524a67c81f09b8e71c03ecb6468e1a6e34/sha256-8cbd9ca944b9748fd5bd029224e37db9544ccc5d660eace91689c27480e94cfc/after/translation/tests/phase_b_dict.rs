//! Phase B: dictionary APIs — CDict / DDict / loadDictionary / refPrefix /
//! dictContentType / dictLoadMethod / attach-pref, plus dictID queries.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompressUsingCDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
type FnDecompressUsingDDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const c_void) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int, c_int) -> usize;
type FnRefDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDictIDFromBuf = unsafe extern "C" fn(*const u8, usize) -> c_uint;
type FnDictIDFromDict = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnSizeofDict = unsafe extern "C" fn(*const c_void) -> usize;
type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, c_uint) -> usize;

const C_COMPRESSIONLEVEL: c_int = 100;
const C_CHECKSUMFLAG: c_int = 201;
const C_DICTIDFLAG: c_int = 202;
const C_FORCEATTACHDICT: c_int = 1001;
const C_ENABLEDEDICATEDDICTSEARCH: c_int = 1005;
const C_PREFETCHCDICTTABLES: c_int = 1013;
const C_DETERMINISTICREFPREFIX: c_int = 1012;
const D_REFMULTIPLEDDICTS: c_int = 1003;

/// Build a trained dictionary with the C library (used as test *input*; the
/// dictBuilder itself is differentially tested in phase_b_dictbuilder.rs).
fn train_dict(rng: &mut Rng, dict_cap: usize) -> Vec<u8> {
    let (ctrain, rtrain) = unsafe { pair::<FnTrain>("ZDICT_trainFromBuffer") };
    let nb = 400;
    let mut samples = Vec::new();
    let mut sizes = Vec::new();
    for _ in 0..nb {
        let n = rng.range(64, 512);
        let s = gen(Shape::Text, n, rng);
        sizes.push(s.len());
        samples.extend_from_slice(&s);
    }
    let mut cbuf = vec![0u8; dict_cap];
    let mut rbuf = vec![0u8; dict_cap];
    let cn = unsafe {
        ctrain(cbuf.as_mut_ptr(), dict_cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint)
    };
    let rn = unsafe {
        rtrain(rbuf.as_mut_ptr(), dict_cap, samples.as_ptr(), sizes.as_ptr(), nb as c_uint)
    };
    assert_eq!(cn, rn, "ZDICT_trainFromBuffer return");
    assert!(!is_error(cn), "ZDICT_trainFromBuffer failed: {}", err_code(cn));
    assert_bytes_eq("trained dictionary bytes", &cbuf[..cn], &rbuf[..rn]);
    cbuf.truncate(cn);
    cbuf
}

fn compress2_diff(
    setup: &dyn Fn(*mut c_void, bool) -> usize,
    src: &[u8],
    ctx: &str,
) -> Option<Vec<u8>> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cap = unsafe { cb(src.len()) } + 64;

    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    let sa = setup(cc, false);
    let sb = setup(rc, true);
    assert_eq!(sa, sb, "{ctx}: dictionary setup return differs");
    let mut out = None;
    if !is_error(sa) {
        let mut cbuf = vec![0xAAu8; cap];
        let mut rbuf = vec![0x55u8; cap];
        let a = unsafe { c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { r2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(a, b, "{ctx}: compress2 return (C err {})", err_code(a));
        if !is_error(a) {
            assert_bytes_eq(&format!("{ctx}: compress2 bytes"), &cbuf[..a], &rbuf[..b]);
            cbuf.truncate(a);
            out = Some(cbuf);
        }
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
    out
}

fn decompress_diff(
    setup: &dyn Fn(*mut c_void, bool) -> usize,
    frame: &[u8],
    expect: &[u8],
    ctx: &str,
) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cd, rd) = unsafe { pair::<FnDecompressDCtx>("ZSTD_decompressDCtx") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    let sa = setup(cc, false);
    let sb = setup(rc, true);
    assert_eq!(sa, sb, "{ctx}: dctx setup return differs");
    if !is_error(sa) {
        let cap = expect.len() + 64;
        let mut co = vec![0u8; cap];
        let mut ro = vec![0u8; cap];
        let a = unsafe { cd(cc, co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
        let b = unsafe { rd(rc, ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
        assert_eq!(a, b, "{ctx}: decompressDCtx return (C err {})", err_code(a));
        if !is_error(a) {
            assert_bytes_eq(&format!("{ctx}: dec bytes"), &co[..a], &ro[..b]);
            assert_bytes_eq(&format!("{ctx}: dec vs orig"), &co[..a], expect);
        }
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

#[test]
fn cfg_cdict_ddict_levels_and_shapes() {
    let (c_ccd, r_ccd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (c_fcd, r_fcd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (c_cdd, r_cdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (c_fdd, r_fdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (c_cu, r_cu) = unsafe { pair::<FnCompressUsingCDict>("ZSTD_compress_usingCDict") };
    let (c_du, r_du) = unsafe { pair::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict") };
    let (c_cn, r_cn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (c_cf, r_cf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c_dn, r_dn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (c_df, r_df) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (c_szcd, r_szcd) = unsafe { pair::<FnSizeofDict>("ZSTD_sizeof_CDict") };
    let (c_szdd, r_szdd) = unsafe { pair::<FnSizeofDict>("ZSTD_sizeof_DDict") };
    let (c_idd, r_idd) = unsafe { pair::<FnDictIDFromDict>("ZSTD_getDictID_fromCDict") };
    let (c_idD, r_idD) = unsafe { pair::<FnDictIDFromDict>("ZSTD_getDictID_fromDDict") };
    let (c_idb, r_idb) = unsafe { pair::<FnDictIDFromBuf>("ZSTD_getDictID_fromDict") };
    let (c_idf, r_idf) = unsafe { pair::<FnDictIDFromBuf>("ZSTD_getDictID_fromFrame") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let mut rng = Rng::new(0xD1C7);
    let trained = train_dict(&mut rng, 16384);
    let raw = gen(Shape::Text, 20000, &mut rng);

    for (dname, dict) in [("trained", &trained), ("raw", &raw)] {
        unsafe {
            assert_eq!(
                c_idb(dict.as_ptr(), dict.len()),
                r_idb(dict.as_ptr(), dict.len()),
                "getDictID_fromDict {dname}"
            );
        }
        for &lvl in &[1, 3, 6, 12, 19, -3] {
            let ccd = unsafe { c_ccd(dict.as_ptr(), dict.len(), lvl) };
            let rcd = unsafe { r_ccd(dict.as_ptr(), dict.len(), lvl) };
            assert!(!ccd.is_null() && !rcd.is_null());
            unsafe {
                assert_eq!(c_szcd(ccd), r_szcd(rcd), "sizeof_CDict {dname} lvl={lvl}");
                assert_eq!(c_idd(ccd), r_idd(rcd), "getDictID_fromCDict {dname}");
            }
            let cdd = unsafe { c_cdd(dict.as_ptr(), dict.len()) };
            let rdd = unsafe { r_cdd(dict.as_ptr(), dict.len()) };
            unsafe {
                assert_eq!(c_szdd(cdd), r_szdd(rdd), "sizeof_DDict");
                assert_eq!(c_idD(cdd), r_idD(rdd), "getDictID_fromDDict");
            }

            let cctx = unsafe { c_cn() };
            let rctx = unsafe { r_cn() };
            let dctx = unsafe { c_dn() };
            let sctx = unsafe { r_dn() };
            for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive] {
                for &size in &[0usize, 1, 50, 3000, 70000] {
                    let src = gen(shape, size, &mut rng);
                    let cap = unsafe { cb(src.len()) } + 64;
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let a = unsafe {
                        c_cu(cctx, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), ccd)
                    };
                    let b = unsafe {
                        r_cu(rctx, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), rcd)
                    };
                    let ctx = format!(
                        "usingCDict {dname} lvl={lvl} shape={shape:?} size={size}"
                    );
                    assert_eq!(a, b, "{ctx}");
                    assert!(!is_error(a), "{ctx} err {}", err_code(a));
                    assert_bytes_eq(&ctx, &cbuf[..a], &rbuf[..b]);
                    unsafe {
                        assert_eq!(
                            c_idf(cbuf.as_ptr(), a),
                            r_idf(rbuf.as_ptr(), b),
                            "{ctx}: getDictID_fromFrame"
                        );
                    }
                    let mut co = vec![0u8; size + 8];
                    let mut ro = vec![0u8; size + 8];
                    let da = unsafe {
                        c_du(dctx, co.as_mut_ptr(), co.len(), cbuf.as_ptr(), a, cdd)
                    };
                    let db = unsafe {
                        r_du(sctx, ro.as_mut_ptr(), ro.len(), rbuf.as_ptr(), b, rdd)
                    };
                    assert_eq!(da, db, "{ctx}: decompress_usingDDict");
                    assert!(!is_error(da), "{ctx}: dec err {}", err_code(da));
                    assert_bytes_eq(&format!("{ctx} dec"), &co[..da], &src);
                    assert_bytes_eq(&format!("{ctx} decr"), &ro[..db], &src);
                }
            }
            unsafe {
                c_cf(cctx);
                r_cf(rctx);
                c_df(dctx);
                r_df(sctx);
                c_fcd(ccd);
                r_fcd(rcd);
                c_fdd(cdd);
                r_fdd(rdd);
            }
        }
    }
}

#[test]
fn cfg_loadDictionary_variants() {
    let (c_ld, r_ld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (c_ldr, r_ldr) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference") };
    let (c_lda, r_lda) = unsafe { pair::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced") };
    let (c_dld, r_dld) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
    let (c_dldr, r_dldr) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference") };
    let (c_dlda, r_dlda) = unsafe { pair::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };

    let mut rng = Rng::new(0x10AD);
    let trained = train_dict(&mut rng, 8192);
    let raw = gen(Shape::Text, 30000, &mut rng);

    for (dname, dict) in [("trained", trained.clone()), ("raw", raw.clone())] {
        // 0 = ZSTD_dlm_byCopy, 1 = ZSTD_dlm_byRef; dct: 0 auto, 1 rawContent, 2 fullDict
        for &(dlm, dct) in &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)] {
            for &lvl in &[1, 5, 13, 19] {
                for &(shape, size) in
                    &[(Shape::Text, 5000usize), (Shape::Random, 3000), (Shape::Rle, 100000)]
                {
                    let src = gen(shape, size, &mut rng);
                    let d = dict.clone();
                    let ctx = format!(
                        "loadDict_advanced {dname} dlm={dlm} dct={dct} lvl={lvl} shape={shape:?} size={size}"
                    );
                    let dref = &d;
                    let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
                        let sp = if is_rust { &r_sp } else { &c_sp };
                        let rc = unsafe { sp(cctx, C_COMPRESSIONLEVEL, lvl) };
                        if is_error(rc) {
                            return rc;
                        }
                        let f = if is_rust { &r_lda } else { &c_lda };
                        unsafe { f(cctx, dref.as_ptr(), dref.len(), dlm, dct) }
                    };
                    if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
                        let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                            let f = if is_rust { &r_dlda } else { &c_dlda };
                            unsafe { f(dctx, dref.as_ptr(), dref.len(), dlm, dct) }
                        };
                        decompress_diff(&dsetup, &frame, &src, &ctx);
                    }
                }
            }
        }
        // plain + byReference wrappers
        for &which in &[0, 1] {
            let src = gen(Shape::Text, 12000, &mut rng);
            let d = dict.clone();
            let dref = &d;
            let ctx = format!("loadDict {dname} wrapper={which}");
            let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
                let f = match (which, is_rust) {
                    (0, false) => &c_ld,
                    (0, true) => &r_ld,
                    (_, false) => &c_ldr,
                    (_, true) => &r_ldr,
                };
                unsafe { f(cctx, dref.as_ptr(), dref.len()) }
            };
            if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
                let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                    let f = match (which, is_rust) {
                        (0, false) => &c_dld,
                        (0, true) => &r_dld,
                        (_, false) => &c_dldr,
                        (_, true) => &r_dldr,
                    };
                    unsafe { f(dctx, dref.as_ptr(), dref.len()) }
                };
                decompress_diff(&dsetup, &frame, &src, &ctx);
            }
        }
    }
}

#[test]
fn cfg_refPrefix_variants() {
    let (c_rp, r_rp) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_refPrefix") };
    let (c_rpa, r_rpa) = unsafe { pair::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced") };
    let (c_drp, r_drp) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_refPrefix") };
    let (c_drpa, r_drpa) = unsafe { pair::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };

    let mut rng = Rng::new(0x9EF1);
    for &psize in &[0usize, 1, 100, 20000, 200000] {
        let prefix = gen(Shape::Text, psize, &mut rng);
        for &dct in &[0, 1, 2] {
            for &lvl in &[1, 7, 19] {
                for &det in &[0, 1] {
                    let size = rng.range(0, 50000);
                    let src = gen(Shape::Text, size, &mut rng);
                    let pref = &prefix;
                    let ctx = format!(
                        "refPrefix psize={psize} dct={dct} lvl={lvl} det={det} size={size}"
                    );
                    let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
                        let sp = if is_rust { &r_sp } else { &c_sp };
                        let rc = unsafe { sp(cctx, C_COMPRESSIONLEVEL, lvl) };
                        if is_error(rc) {
                            return rc;
                        }
                        let rc = unsafe { sp(cctx, C_DETERMINISTICREFPREFIX, det) };
                        if is_error(rc) {
                            return rc;
                        }
                        let f = if is_rust { &r_rpa } else { &c_rpa };
                        unsafe { f(cctx, pref.as_ptr(), pref.len(), dct) }
                    };
                    if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
                        let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                            let f = if is_rust { &r_drpa } else { &c_drpa };
                            unsafe { f(dctx, pref.as_ptr(), pref.len(), dct) }
                        };
                        decompress_diff(&dsetup, &frame, &src, &ctx);
                    }
                }
            }
        }
        // plain wrappers
        let src = gen(Shape::Text, 9000, &mut rng);
        let pref = &prefix;
        let ctx = format!("refPrefix plain psize={psize}");
        let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
            let f = if is_rust { &r_rp } else { &c_rp };
            unsafe { f(cctx, pref.as_ptr(), pref.len()) }
        };
        if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
            let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                let f = if is_rust { &r_drp } else { &c_drp };
                unsafe { f(dctx, pref.as_ptr(), pref.len()) }
            };
            decompress_diff(&dsetup, &frame, &src, &ctx);
        }
    }
}

#[test]
fn cfg_refCDict_attach_prefs() {
    let (c_ccd, r_ccd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (c_ccdr, r_ccdr) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict_byReference") };
    let (c_fcd, r_fcd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (c_rcd, r_rcd) = unsafe { pair::<FnRefDict>("ZSTD_CCtx_refCDict") };
    let (c_cdd, r_cdd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (c_cddr, r_cddr) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict_byReference") };
    let (c_fdd, r_fdd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (c_rdd, r_rdd) = unsafe { pair::<FnRefDict>("ZSTD_DCtx_refDDict") };
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c_dsp, r_dsp) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };

    let mut rng = Rng::new(0xA77A);
    let trained = train_dict(&mut rng, 12000);

    for &byref in &[false, true] {
        for &dlvl in &[1, 6, 19] {
            let ccd = unsafe {
                if byref {
                    c_ccdr(trained.as_ptr(), trained.len(), dlvl)
                } else {
                    c_ccd(trained.as_ptr(), trained.len(), dlvl)
                }
            };
            let rcd = unsafe {
                if byref {
                    r_ccdr(trained.as_ptr(), trained.len(), dlvl)
                } else {
                    r_ccd(trained.as_ptr(), trained.len(), dlvl)
                }
            };
            let cdd = unsafe {
                if byref {
                    c_cddr(trained.as_ptr(), trained.len())
                } else {
                    c_cdd(trained.as_ptr(), trained.len())
                }
            };
            let rdd = unsafe {
                if byref {
                    r_cddr(trained.as_ptr(), trained.len())
                } else {
                    r_cdd(trained.as_ptr(), trained.len())
                }
            };
            assert!(!ccd.is_null() && !rcd.is_null() && !cdd.is_null() && !rdd.is_null());
            // attachPref: 0 default, 1 attach, 2 copy, 3 load, 4 forceLoad?
            for &attach in &[0, 1, 2, 3] {
                for &dds in &[0, 1] {
                    for &prefetch in &[0, 1, 2] {
                        for &lvl in &[1, 9, 19] {
                            for &(shape, size) in
                                &[(Shape::Text, 4000usize), (Shape::Random, 200), (Shape::Text, 90000)]
                            {
                                let src = gen(shape, size, &mut rng);
                                let ctx = format!(
                                    "refCDict byref={byref} dlvl={dlvl} attach={attach} dds={dds} prefetch={prefetch} lvl={lvl} shape={shape:?} size={size}"
                                );
                                let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
                                    let sp = if is_rust { &r_sp } else { &c_sp };
                                    for (p, v) in [
                                        (C_COMPRESSIONLEVEL, lvl),
                                        (C_FORCEATTACHDICT, attach),
                                        (C_ENABLEDEDICATEDDICTSEARCH, dds),
                                        (C_PREFETCHCDICTTABLES, prefetch),
                                    ] {
                                        let rc = unsafe { sp(cctx, p, v) };
                                        if is_error(rc) {
                                            return rc;
                                        }
                                    }
                                    let f = if is_rust { &r_rcd } else { &c_rcd };
                                    let d = if is_rust { rcd } else { ccd };
                                    unsafe { f(cctx, d) }
                                };
                                if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
                                    let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                                        let sp = if is_rust { &r_dsp } else { &c_dsp };
                                        let rc = unsafe { sp(dctx, D_REFMULTIPLEDDICTS, 0) };
                                        if is_error(rc) {
                                            return rc;
                                        }
                                        let f = if is_rust { &r_rdd } else { &c_rdd };
                                        let d = if is_rust { rdd } else { cdd };
                                        unsafe { f(dctx, d) }
                                    };
                                    decompress_diff(&dsetup, &frame, &src, &ctx);
                                }
                            }
                        }
                    }
                }
            }
            unsafe {
                c_fcd(ccd);
                r_fcd(rcd);
                c_fdd(cdd);
                r_fdd(rdd);
            }
        }
    }
}

#[test]
fn cfg_dictID_flag_and_queries() {
    let (c_sp, r_sp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c_ld, r_ld) = unsafe { pair::<FnLoadDict>("ZSTD_CCtx_loadDictionary") };
    let (c_idf, r_idf) = unsafe { pair::<FnDictIDFromBuf>("ZSTD_getDictID_fromFrame") };
    let mut rng = Rng::new(0xDDDD);
    let trained = train_dict(&mut rng, 6000);
    for &dictid in &[0, 1] {
        let src = gen(Shape::Text, 15000, &mut rng);
        let d = &trained;
        let ctx = format!("dictIDFlag={dictid}");
        let setup = |cctx: *mut c_void, is_rust: bool| -> usize {
            let sp = if is_rust { &r_sp } else { &c_sp };
            let rc = unsafe { sp(cctx, C_DICTIDFLAG, dictid) };
            if is_error(rc) {
                return rc;
            }
            let f = if is_rust { &r_ld } else { &c_ld };
            unsafe { f(cctx, d.as_ptr(), d.len()) }
        };
        if let Some(frame) = compress2_diff(&setup, &src, &ctx) {
            unsafe {
                assert_eq!(
                    c_idf(frame.as_ptr(), frame.len()),
                    r_idf(frame.as_ptr(), frame.len()),
                    "{ctx}: getDictID_fromFrame"
                );
            }
            let dsetup = |dctx: *mut c_void, is_rust: bool| -> usize {
                let f = if is_rust { &r_ld } else { &c_ld };
                let _ = f;
                let (cd, rd) = unsafe { pair::<FnLoadDict>("ZSTD_DCtx_loadDictionary") };
                let g = if is_rust { rd } else { cd };
                unsafe { g(dctx, d.as_ptr(), d.len()) }
            };
            decompress_diff(&dsetup, &frame, &src, &ctx);
        }
    }
}
