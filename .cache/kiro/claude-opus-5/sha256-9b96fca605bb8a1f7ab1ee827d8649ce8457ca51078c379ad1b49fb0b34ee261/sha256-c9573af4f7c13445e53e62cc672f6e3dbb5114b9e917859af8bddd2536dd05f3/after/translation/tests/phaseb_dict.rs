//! Phase B — valid-path differential tests for the DICTIONARY surface.
//!
//! Covers `CONFIGS.md` section "Dictionary loading modes (CDict/DDict/prefix,
//! byRef vs byCopy)" plus the `*_usingCDict` / `*_usingDDict` entry points and
//! `ZSTD_copyCCtx` / `ZSTD_copyDCtx`.
//!
//! Every axis the C branches on is crossed: dictContentType (auto / rawContent
//! / fullDict, plus the out-of-range value 3), dictLoadMethod (byCopy / byRef,
//! plus out-of-range 2), forceAttachDict (all 4 modes), dedicated dict search,
//! prefetchCDictTables, dictionary vs prefix, trained vs raw content, and
//! refMultipleDDicts.

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int, c_int) -> size_t;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    size_t,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;
type FnRef = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnDictID = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDictIDBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnSizeofOpaque = unsafe extern "C" fn(*const c_void) -> size_t;
type FnUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
) -> size_t;
type FnUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    ZSTD_frameParameters,
) -> size_t;
type FnUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    size_t,
) -> size_t;
type FnUsingDictLvl = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
) -> size_t;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub alloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}
const NULL_MEM: ZSTD_customMem = ZSTD_customMem {
    alloc: None,
    free: None,
    opaque: std::ptr::null_mut(),
};

// ZSTD_dictAttachPref_e
const ZSTD_dictDefaultAttach: c_int = 0;
const ZSTD_dictForceAttach: c_int = 1;
const ZSTD_dictForceCopy: c_int = 2;
const ZSTD_dictForceLoad: c_int = 3;

struct D {
    ccctx: (FnCreate, FnCreate),
    fcctx: (FnFree, FnFree),
    cdctx: (FnCreate, FnCreate),
    fdctx: (FnFree, FnFree),
    setp: (FnSetParam, FnSetParam),
    reset_c: (FnReset, FnReset),
    c2: (FnCompress2, FnCompress2),
    dd: (FnDecompressDCtx, FnDecompressDCtx),
    bound: (FnSizeSize, FnSizeSize),
    is_err: (FnIsError, FnIsError),
    ecode: (FnGetErrorCode, FnGetErrorCode),
}

fn d() -> D {
    D {
        ccctx: fnpair!("ZSTD_createCCtx", FnCreate),
        fcctx: fnpair!("ZSTD_freeCCtx", FnFree),
        cdctx: fnpair!("ZSTD_createDCtx", FnCreate),
        fdctx: fnpair!("ZSTD_freeDCtx", FnFree),
        setp: fnpair!("ZSTD_CCtx_setParameter", FnSetParam),
        reset_c: fnpair!("ZSTD_CCtx_reset", FnReset),
        c2: fnpair!("ZSTD_compress2", FnCompress2),
        dd: fnpair!("ZSTD_decompressDCtx", FnDecompressDCtx),
        bound: fnpair!("ZSTD_compressBound", FnSizeSize),
        is_err: fnpair!("ZSTD_isError", FnIsError),
        ecode: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
    }
}

fn nn(v: &[u8]) -> *const c_void {
    if v.is_empty() {
        std::ptr::null()
    } else {
        v.as_ptr() as *const c_void
    }
}
fn nnsrc(v: &[u8]) -> *const c_void {
    if v.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
    } else {
        v.as_ptr() as *const c_void
    }
}

/// Build a real trained dictionary. `phaseb_dictbuilder.rs` already proves both
/// libraries train identically; here we only need the bytes, but we assert the
/// equality anyway so this file is self-contained.
fn trained_dict(rng: &mut Rng, cap: usize) -> Vec<u8> {
    type FnTrain =
        unsafe extern "C" fn(*mut c_void, size_t, *const c_void, *const size_t, c_uint) -> size_t;
    let (ct, rt) = fnpair!("ZDICT_trainFromBuffer", FnTrain);
    let mut samples: Vec<u8> = Vec::new();
    let mut sizes: Vec<size_t> = Vec::new();
    for _ in 0..150 {
        let n = 300 + rng.below(900);
        let s = gen(Shape::Text, n, rng);
        samples.extend_from_slice(&s);
        sizes.push(n);
    }
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    unsafe {
        let n1 = ct(
            a.as_mut_ptr() as *mut c_void,
            cap,
            samples.as_ptr() as *const c_void,
            sizes.as_ptr(),
            sizes.len() as c_uint,
        );
        let n2 = rt(
            b.as_mut_ptr() as *mut c_void,
            cap,
            samples.as_ptr() as *const c_void,
            sizes.as_ptr(),
            sizes.len() as c_uint,
        );
        assert_eq!(n1, n2, "ZDICT_trainFromBuffer size");
        if (0usize.wrapping_sub(n1)) <= 200 {
            // training failed identically on both sides -> use raw content
            return gen(Shape::Text, cap, rng);
        }
        assert_bytes_eq("ZDICT_trainFromBuffer", &a[..n1], &b[..n2]);
        a.truncate(n1);
    }
    a
}

/// Apply `setup` to a fresh CCtx on each library, compress, compare bytes,
/// then cross-decompress with `dict_for_decode` if given.
#[track_caller]
unsafe fn diff_with_setup(
    d: &D,
    src: &[u8],
    ctx: &str,
    setup: &dyn Fn(*mut c_void, bool) -> size_t,
    dict_for_decode: Option<&[u8]>,
) {
    let cc = (d.ccctx.0)();
    let rc = (d.ccctx.1)();
    let a = setup(cc, false);
    let b = setup(rc, true);
    assert_eq!(a, b, "{ctx}: setup rc (C={a:#x} R={b:#x})");
    assert_eq!((d.ecode.0)(a), (d.ecode.1)(b), "{ctx}: setup ecode");
    if (d.is_err.0)(a) == 0 {
        let cap = (d.bound.0)(src.len()).max(64);
        let mut o1 = vec![0xAAu8; cap];
        let mut o2 = vec![0xAAu8; cap];
        let sp = nnsrc(src);
        let n1 = (d.c2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, src.len());
        let n2 = (d.c2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, src.len());
        assert_eq!(n1, n2, "{ctx}: compress2 rc");
        assert_eq!((d.ecode.0)(n1), (d.ecode.1)(n2), "{ctx}: compress2 ecode");
        if (d.is_err.0)(n1) == 0 {
            assert_bytes_eq(&format!("{ctx}: frame"), &o1[..n1], &o2[..n2]);
            if let Some(dict) = dict_for_decode {
                let (c_ud, r_ud) = fnpair!("ZSTD_decompress_usingDict", FnUsingDict);
                let dx = (d.cdctx.0)();
                let dy = (d.cdctx.1)();
                let mut p1 = vec![0xAAu8; src.len() + 8];
                let mut p2 = vec![0xAAu8; src.len() + 8];
                // C decoder on the RUST frame, Rust decoder on the C frame
                let r1 = c_ud(
                    dx,
                    p1.as_mut_ptr() as *mut c_void,
                    p1.len(),
                    o2.as_ptr() as *const c_void,
                    n2,
                    nn(dict),
                    dict.len(),
                );
                let r2 = r_ud(
                    dy,
                    p2.as_mut_ptr() as *mut c_void,
                    p2.len(),
                    o1.as_ptr() as *const c_void,
                    n1,
                    nn(dict),
                    dict.len(),
                );
                assert_eq!(r1, r2, "{ctx}: cross-decode rc");
                assert_eq!((d.ecode.0)(r1), (d.ecode.1)(r2), "{ctx}: cross-decode ecode");
                assert_bytes_eq(&format!("{ctx}: cross-decode"), &p1, &p2);
                if (d.is_err.0)(r1) == 0 {
                    assert_eq!(r1, src.len(), "{ctx}: cross-decode size");
                    assert_bytes_eq(&format!("{ctx}: roundtrip"), src, &p1[..r1]);
                }
                (d.fdctx.0)(dx);
                (d.fdctx.1)(dy);
            }
        }
    }
    (d.fcctx.0)(cc);
    (d.fcctx.1)(rc);
}

// ============== CONFIGS: loadDictionary x contentType x loadMethod =========

#[test]
fn b_load_dictionary_all_modes() {
    let d = d();
    let (c_ld, r_ld) = fnpair!("ZSTD_CCtx_loadDictionary", FnLoadDict);
    let (c_lb, r_lb) = fnpair!("ZSTD_CCtx_loadDictionary_byReference", FnLoadDict);
    let (c_la, r_la) = fnpair!("ZSTD_CCtx_loadDictionary_advanced", FnLoadDictAdv);

    let mut rng = Rng::new(0xD1C7);
    let trained = trained_dict(&mut rng, 8192);
    let raw = gen(Shape::Text, 8192, &mut rng);
    let tiny = gen(Shape::Random, 3, &mut rng);
    let empty: Vec<u8> = Vec::new();

    unsafe {
        for (dname, dict) in [
            ("trained", &trained),
            ("raw", &raw),
            ("tiny", &tiny),
            ("empty", &empty),
        ] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Mixed] {
                for &len in &[0usize, 1, 4000, 150_000] {
                    let src = gen(shape, len, &mut rng);
                    let dp = nn(dict);
                    let dl = dict.len();

                    let tag = format!("loadDictionary d={dname} {shape:?} len={len}");
                    diff_with_setup(
                        &d,
                        &src,
                        &tag,
                        &|c, r| if r { r_ld(c, dp, dl) } else { c_ld(c, dp, dl) },
                        Some(dict),
                    );

                    let tag = format!("loadDictionary_byRef d={dname} {shape:?} len={len}");
                    diff_with_setup(
                        &d,
                        &src,
                        &tag,
                        &|c, r| if r { r_lb(c, dp, dl) } else { c_lb(c, dp, dl) },
                        Some(dict),
                    );

                    // advanced: loadMethod x contentType (incl. out-of-range 2 / 3)
                    for lm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef, 2] {
                        for ct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict, 3] {
                            for attach in [
                                ZSTD_dictDefaultAttach,
                                ZSTD_dictForceAttach,
                                ZSTD_dictForceCopy,
                                ZSTD_dictForceLoad,
                            ] {
                                let tag = format!(
                                    "loadDict_adv d={dname} lm={lm} ct={ct} attach={attach} {shape:?} len={len}"
                                );
                                diff_with_setup(
                                    &d,
                                    &src,
                                    &tag,
                                    &|c, r| {
                                        let sp = if r {
                                            (d.setp.1)(c, ZSTD_c_forceAttachDict, attach)
                                        } else {
                                            (d.setp.0)(c, ZSTD_c_forceAttachDict, attach)
                                        };
                                        if (d.is_err.0)(sp) != 0 {
                                            return sp;
                                        }
                                        if r {
                                            r_la(c, dp, dl, lm, ct, 3)
                                        } else {
                                            c_la(c, dp, dl, lm, ct, 3)
                                        }
                                    },
                                    Some(dict),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================== CONFIGS: refPrefix (compress + decompress) =============

#[test]
fn b_ref_prefix() {
    let d = d();
    let (c_rp, r_rp) = fnpair!("ZSTD_CCtx_refPrefix", FnLoadDict);
    let (c_ra, r_ra) = fnpair!("ZSTD_CCtx_refPrefix_advanced", FnRefPrefixAdv);
    let (c_drp, r_drp) = fnpair!("ZSTD_DCtx_refPrefix", FnLoadDict);
    let (c_dra, r_dra) = fnpair!("ZSTD_DCtx_refPrefix_advanced", FnRefPrefixAdv);

    let mut rng = Rng::new(0x9EF1);
    unsafe {
        for &plen in &[0usize, 1, 100, 8192, 200_000] {
            let prefix = gen(Shape::Text, plen, &mut rng);
            let pp = nn(&prefix);
            for &shape in &[Shape::Text, Shape::Random, Shape::LongRange] {
                for &len in &[0usize, 1, 5000, 150_000] {
                    let src = gen(shape, len, &mut rng);
                    let sp = nnsrc(&src);
                    let cap = (d.bound.0)(len).max(64);

                    for ct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict, 3] {
                        let tag = format!("refPrefix_adv plen={plen} ct={ct} {shape:?} len={len}");
                        let cc = (d.ccctx.0)();
                        let rc = (d.ccctx.1)();
                        let a = c_ra(cc, pp, plen, ct);
                        let b = r_ra(rc, pp, plen, ct);
                        assert_eq!(a, b, "{tag}: refPrefix_advanced rc");
                        assert_eq!((d.ecode.0)(a), (d.ecode.1)(b), "{tag}: ecode");
                        if (d.is_err.0)(a) == 0 {
                            let mut o1 = vec![0xAAu8; cap];
                            let mut o2 = vec![0xAAu8; cap];
                            let n1 = (d.c2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, len);
                            let n2 = (d.c2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, len);
                            assert_eq!(n1, n2, "{tag}: compress2 rc");
                            assert_eq!((d.ecode.0)(n1), (d.ecode.1)(n2), "{tag}: c2 ecode");
                            if (d.is_err.0)(n1) == 0 {
                                assert_bytes_eq(&format!("{tag}: frame"), &o1[..n1], &o2[..n2]);
                                let dx = (d.cdctx.0)();
                                let dy = (d.cdctx.1)();
                                let x = c_dra(dx, pp, plen, ct);
                                let y = r_dra(dy, pp, plen, ct);
                                assert_eq!(x, y, "{tag}: DCtx_refPrefix_advanced rc");
                                assert_eq!((d.ecode.0)(x), (d.ecode.1)(y), "{tag}: dctx ecode");
                                if (d.is_err.0)(x) == 0 {
                                    let mut p1 = vec![0xAAu8; len + 8];
                                    let mut p2 = vec![0xAAu8; len + 8];
                                    let r1 = (d.dd.0)(
                                        dx,
                                        p1.as_mut_ptr() as *mut c_void,
                                        p1.len(),
                                        o2.as_ptr() as *const c_void,
                                        n2,
                                    );
                                    let r2 = (d.dd.1)(
                                        dy,
                                        p2.as_mut_ptr() as *mut c_void,
                                        p2.len(),
                                        o1.as_ptr() as *const c_void,
                                        n1,
                                    );
                                    assert_eq!(r1, r2, "{tag}: cross decode rc");
                                    assert_eq!(
                                        (d.ecode.0)(r1),
                                        (d.ecode.1)(r2),
                                        "{tag}: cross decode ecode"
                                    );
                                    assert_bytes_eq(&format!("{tag}: cross decode"), &p1, &p2);
                                    if (d.is_err.0)(r1) == 0 {
                                        assert_bytes_eq(
                                            &format!("{tag}: roundtrip"),
                                            &src,
                                            &p1[..r1],
                                        );
                                    }
                                }
                                (d.fdctx.0)(dx);
                                (d.fdctx.1)(dy);
                            }
                        }
                        (d.fcctx.0)(cc);
                        (d.fcctx.1)(rc);
                    }

                    // plain refPrefix / DCtx_refPrefix
                    let tag = format!("refPrefix plen={plen} {shape:?} len={len}");
                    let cc = (d.ccctx.0)();
                    let rc = (d.ccctx.1)();
                    assert_eq!(c_rp(cc, pp, plen), r_rp(rc, pp, plen), "{tag}: refPrefix rc");
                    let mut o1 = vec![0xAAu8; cap];
                    let mut o2 = vec![0xAAu8; cap];
                    let n1 = (d.c2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, len);
                    let n2 = (d.c2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, len);
                    assert_eq!(n1, n2, "{tag}: compress rc");
                    if (d.is_err.0)(n1) == 0 {
                        assert_bytes_eq(&format!("{tag}: frame"), &o1[..n1], &o2[..n2]);
                        let dx = (d.cdctx.0)();
                        let dy = (d.cdctx.1)();
                        assert_eq!(
                            c_drp(dx, pp, plen),
                            r_drp(dy, pp, plen),
                            "{tag}: DCtx_refPrefix rc"
                        );
                        let mut p1 = vec![0xAAu8; len + 8];
                        let mut p2 = vec![0xAAu8; len + 8];
                        let r1 = (d.dd.0)(
                            dx,
                            p1.as_mut_ptr() as *mut c_void,
                            p1.len(),
                            o2.as_ptr() as *const c_void,
                            n2,
                        );
                        let r2 = (d.dd.1)(
                            dy,
                            p2.as_mut_ptr() as *mut c_void,
                            p2.len(),
                            o1.as_ptr() as *const c_void,
                            n1,
                        );
                        assert_eq!(r1, r2, "{tag}: cross decode rc");
                        assert_bytes_eq(&format!("{tag}: cross decode"), &p1, &p2);
                        (d.fdctx.0)(dx);
                        (d.fdctx.1)(dy);
                    }
                    (d.fcctx.0)(cc);
                    (d.fcctx.1)(rc);
                }
            }
        }
    }
}

// ============== CONFIGS: CDict creation variants x refCDict ================

#[test]
fn b_cdict_variants() {
    let d = d();
    let (c_cc, r_cc) = fnpair!("ZSTD_createCDict", FnCreateCDict);
    let (c_cb, r_cb) = fnpair!("ZSTD_createCDict_byReference", FnCreateCDict);
    let (c_ca, r_ca) = fnpair!("ZSTD_createCDict_advanced", FnCreateCDictAdv);
    let (c_fc, r_fc) = fnpair!("ZSTD_freeCDict", FnFree);
    let (c_rf, r_rf) = fnpair!("ZSTD_CCtx_refCDict", FnRef);
    let (c_id, r_id) = fnpair!("ZSTD_getDictID_fromCDict", FnDictID);
    let (c_sz, r_sz) = fnpair!("ZSTD_sizeof_CDict", FnSizeofOpaque);
    let (c_gc, r_gc) = fnpair!(
        "ZSTD_getCParams",
        unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_compressionParameters
    );
    let (c_uc, r_uc) = fnpair!("ZSTD_compress_usingCDict", FnUsingCDict);
    let (c_ua, r_ua) = fnpair!("ZSTD_compress_usingCDict_advanced", FnUsingCDictAdv);

    let mut rng = Rng::new(0xCD1C);
    let trained = trained_dict(&mut rng, 16384);
    let raw = gen(Shape::Text, 16384, &mut rng);

    unsafe {
        for (dname, dict) in [("trained", &trained), ("raw", &raw)] {
            let dp = nn(dict);
            for lvl in [1, 6, 19] {
                let cd1 = c_cc(dp, dict.len(), lvl);
                let rd1 = r_cc(dp, dict.len(), lvl);
                assert_eq!(cd1.is_null(), rd1.is_null(), "createCDict null d={dname}");
                let cd2 = c_cb(dp, dict.len(), lvl);
                let rd2 = r_cb(dp, dict.len(), lvl);
                assert_eq!(cd2.is_null(), rd2.is_null(), "createCDict_byReference null");
                assert_eq!(c_id(cd1), r_id(rd1), "getDictID_fromCDict d={dname} lvl={lvl}");
                assert_eq!(c_sz(cd1), r_sz(rd1), "sizeof_CDict d={dname} lvl={lvl}");
                assert_eq!(c_sz(cd2), r_sz(rd2), "sizeof_CDict byRef d={dname} lvl={lvl}");

                for lm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    for ct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                        let cp1 = c_gc(lvl, 0, dict.len());
                        let cp2 = r_gc(lvl, 0, dict.len());
                        assert_eq!(cp1, cp2, "getCParams lvl={lvl}");
                        let a = c_ca(dp, dict.len(), lm, ct, cp1, NULL_MEM);
                        let b = r_ca(dp, dict.len(), lm, ct, cp2, NULL_MEM);
                        assert_eq!(
                            a.is_null(),
                            b.is_null(),
                            "createCDict_advanced null d={dname} lm={lm} ct={ct} lvl={lvl}"
                        );
                        if a.is_null() {
                            continue;
                        }
                        assert_eq!(c_id(a), r_id(b), "adv dictID lm={lm} ct={ct}");
                        assert_eq!(c_sz(a), r_sz(b), "adv sizeof_CDict lm={lm} ct={ct}");
                        for &shape in &[Shape::Text, Shape::Random] {
                            for &len in &[0usize, 1, 6000, 150_000] {
                                let src = gen(shape, len, &mut rng);
                                let cap = (d.bound.0)(len).max(64);
                                let mut o1 = vec![0xAAu8; cap];
                                let mut o2 = vec![0xAAu8; cap];
                                let sp = nnsrc(&src);
                                let cx = (d.ccctx.0)();
                                let rx = (d.ccctx.1)();
                                let n1 =
                                    c_uc(cx, o1.as_mut_ptr() as *mut c_void, cap, sp, len, a);
                                let n2 =
                                    r_uc(rx, o2.as_mut_ptr() as *mut c_void, cap, sp, len, b);
                                let tag = format!(
                                    "usingCDict d={dname} lvl={lvl} lm={lm} ct={ct} {shape:?} len={len}"
                                );
                                assert_eq!(n1, n2, "{tag}: rc");
                                assert_eq!((d.ecode.0)(n1), (d.ecode.1)(n2), "{tag}: ecode");
                                if (d.is_err.0)(n1) == 0 {
                                    assert_bytes_eq(&tag, &o1[..n1], &o2[..n2]);
                                }
                                for fp in [
                                    ZSTD_frameParameters {
                                        contentSizeFlag: 0,
                                        checksumFlag: 0,
                                        noDictIDFlag: 0,
                                    },
                                    ZSTD_frameParameters {
                                        contentSizeFlag: 1,
                                        checksumFlag: 1,
                                        noDictIDFlag: 1,
                                    },
                                    ZSTD_frameParameters {
                                        contentSizeFlag: 1,
                                        checksumFlag: 0,
                                        noDictIDFlag: 1,
                                    },
                                ] {
                                    let n1 = c_ua(
                                        cx,
                                        o1.as_mut_ptr() as *mut c_void,
                                        cap,
                                        sp,
                                        len,
                                        a,
                                        fp,
                                    );
                                    let n2 = r_ua(
                                        rx,
                                        o2.as_mut_ptr() as *mut c_void,
                                        cap,
                                        sp,
                                        len,
                                        b,
                                        fp,
                                    );
                                    let tag = format!(
                                        "usingCDict_adv fp={fp:?} d={dname} lvl={lvl} lm={lm} ct={ct} {shape:?} len={len}"
                                    );
                                    assert_eq!(n1, n2, "{tag}: rc");
                                    assert_eq!((d.ecode.0)(n1), (d.ecode.1)(n2), "{tag}: ecode");
                                    if (d.is_err.0)(n1) == 0 {
                                        assert_bytes_eq(&tag, &o1[..n1], &o2[..n2]);
                                    }
                                }
                                (d.fcctx.0)(cx);
                                (d.fcctx.1)(rx);
                            }
                        }
                        c_fc(a);
                        r_fc(b);
                    }
                }

                // refCDict x forceAttachDict x dedicatedDictSearch x prefetch
                for attach in [
                    ZSTD_dictDefaultAttach,
                    ZSTD_dictForceAttach,
                    ZSTD_dictForceCopy,
                    ZSTD_dictForceLoad,
                ] {
                    for dds in [0, 1] {
                        for pf in [0, 1] {
                            for &shape in &[Shape::Text, Shape::Mixed] {
                                for &len in &[0usize, 1, 9000, 150_000] {
                                    let src = gen(shape, len, &mut rng);
                                    let tag = format!(
                                        "refCDict d={dname} lvl={lvl} attach={attach} dds={dds} pf={pf} {shape:?} len={len}"
                                    );
                                    diff_with_setup(
                                        &d,
                                        &src,
                                        &tag,
                                        &|c, r| {
                                            for &(p, v) in &[
                                                (ZSTD_c_forceAttachDict, attach),
                                                (ZSTD_c_enableDedicatedDictSearch, dds),
                                                (ZSTD_c_prefetchCDictTables, pf),
                                            ] {
                                                let rc = if r {
                                                    (d.setp.1)(c, p, v)
                                                } else {
                                                    (d.setp.0)(c, p, v)
                                                };
                                                if (d.is_err.0)(rc) != 0 {
                                                    return rc;
                                                }
                                            }
                                            if r {
                                                r_rf(c, rd1 as *const c_void)
                                            } else {
                                                c_rf(c, cd1 as *const c_void)
                                            }
                                        },
                                        Some(dict),
                                    );
                                }
                            }
                        }
                    }
                }

                c_fc(cd1);
                r_fc(rd1);
                c_fc(cd2);
                r_fc(rd2);
            }
        }
        // NULL CDict is documented as "clear the dictionary"
        let cx = (d.ccctx.0)();
        let rx = (d.ccctx.1)();
        assert_eq!(
            c_rf(cx, std::ptr::null()),
            r_rf(rx, std::ptr::null()),
            "refCDict(NULL)"
        );
        (d.fcctx.0)(cx);
        (d.fcctx.1)(rx);
    }
}

// ============== CONFIGS: DDict variants x refDDict x load paths ============

#[test]
fn b_ddict_variants() {
    let d = d();
    let (c_cd, r_cd) = fnpair!("ZSTD_createDDict", FnCreateDDict);
    let (c_cb, r_cb) = fnpair!("ZSTD_createDDict_byReference", FnCreateDDict);
    let (c_ca, r_ca) = fnpair!(
        "ZSTD_createDDict_advanced",
        unsafe extern "C" fn(*const c_void, size_t, c_int, c_int, ZSTD_customMem) -> *mut c_void
    );
    let (c_fd, r_fd) = fnpair!("ZSTD_freeDDict", FnFree);
    let (c_rf, r_rf) = fnpair!("ZSTD_DCtx_refDDict", FnRef);
    let (c_id, r_id) = fnpair!("ZSTD_getDictID_fromDDict", FnDictID);
    let (c_idd, r_idd) = fnpair!("ZSTD_getDictID_fromDict", FnDictIDBuf);
    let (c_sz, r_sz) = fnpair!("ZSTD_sizeof_DDict", FnSizeofOpaque);
    let (c_dc, r_dc) = fnpair!(
        "ZSTD_DDict_dictContent",
        unsafe extern "C" fn(*const c_void) -> *const c_void
    );
    let (c_ds, r_ds) = fnpair!("ZSTD_DDict_dictSize", FnSizeofOpaque);
    let (c_cpp, r_cpp) = fnpair!(
        "ZSTD_copyDDictParameters",
        unsafe extern "C" fn(*mut c_void, *const c_void)
    );
    let (c_ud, r_ud) = fnpair!(
        "ZSTD_decompress_usingDDict",
        unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            size_t,
            *const c_void,
            size_t,
            *const c_void,
        ) -> size_t
    );
    let (c_ldd, r_ldd) = fnpair!("ZSTD_DCtx_loadDictionary", FnLoadDict);
    let (c_ldb, r_ldb) = fnpair!("ZSTD_DCtx_loadDictionary_byReference", FnLoadDict);
    let (c_lda, r_lda) = fnpair!("ZSTD_DCtx_loadDictionary_advanced", FnLoadDictAdv);
    let (c_cud, r_cud) = fnpair!("ZSTD_compress_usingDict", FnUsingDictLvl);
    let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);

    let mut rng = Rng::new(0xDD1C);
    let trained = trained_dict(&mut rng, 16384);
    let raw = gen(Shape::Text, 16384, &mut rng);

    unsafe {
        for (dname, dict) in [("trained", &trained), ("raw", &raw)] {
            let dp = nn(dict);
            assert_eq!(
                c_idd(dp, dict.len()),
                r_idd(dp, dict.len()),
                "getDictID_fromDict d={dname}"
            );
            let a = c_cd(dp, dict.len());
            let b = r_cd(dp, dict.len());
            assert_eq!(a.is_null(), b.is_null(), "createDDict null d={dname}");
            assert_eq!(c_id(a), r_id(b), "getDictID_fromDDict d={dname}");
            assert_eq!(c_sz(a), r_sz(b), "sizeof_DDict d={dname}");
            assert_eq!(c_ds(a), r_ds(b), "DDict_dictSize d={dname}");
            let (ca, cb, n) = (c_dc(a), r_dc(b), c_ds(a));
            if !ca.is_null() && !cb.is_null() && n > 0 {
                assert_bytes_eq(
                    &format!("DDict_dictContent d={dname}"),
                    std::slice::from_raw_parts(ca as *const u8, n),
                    std::slice::from_raw_parts(cb as *const u8, n),
                );
            }
            let a2 = c_cb(dp, dict.len());
            let b2 = r_cb(dp, dict.len());
            assert_eq!(a2.is_null(), b2.is_null(), "createDDict_byReference null");
            assert_eq!(c_sz(a2), r_sz(b2), "sizeof_DDict byRef d={dname}");
            // ZSTD_copyDDictParameters(dctx, ddict) — first arg is a DCtx. Prime a
            // DCtx from the DDict, then check it decodes a dictionary frame the
            // same way on both sides (observable effect of the copied state).
            {
                let dx = (d.cdctx.0)();
                let dy = (d.cdctx.1)();
                c_cpp(dx, a as *const c_void);
                r_cpp(dy, b as *const c_void);
                let mut o1 = vec![0xAAu8; 64];
                let mut o2 = vec![0xAAu8; 64];
                // decompressDCtx resets the session, so this only asserts the
                // copied parameters left both contexts in an equivalent state.
                let x = (d.dd.0)(
                    dx,
                    o1.as_mut_ptr() as *mut c_void,
                    o1.len(),
                    dp,
                    dict.len().min(32),
                );
                let y = (d.dd.1)(
                    dy,
                    o2.as_mut_ptr() as *mut c_void,
                    o2.len(),
                    dp,
                    dict.len().min(32),
                );
                assert_eq!(x, y, "copyDDictParameters then decode rc d={dname}");
                assert_eq!(
                    (d.ecode.0)(x),
                    (d.ecode.1)(y),
                    "copyDDictParameters then decode ecode d={dname}"
                );
                assert_bytes_eq(&format!("copyDDictParameters out d={dname}"), &o1, &o2);
                (d.fdctx.0)(dx);
                (d.fdctx.1)(dy);
            }

            for lm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for ct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let x = c_ca(dp, dict.len(), lm, ct, NULL_MEM);
                    let y = r_ca(dp, dict.len(), lm, ct, NULL_MEM);
                    assert_eq!(
                        x.is_null(),
                        y.is_null(),
                        "createDDict_advanced null lm={lm} ct={ct}"
                    );
                    if !x.is_null() {
                        assert_eq!(c_id(x), r_id(y), "adv DDict dictID lm={lm} ct={ct}");
                        assert_eq!(c_sz(x), r_sz(y), "adv DDict sizeof lm={lm} ct={ct}");
                        assert_eq!(c_ds(x), r_ds(y), "adv DDict dictSize lm={lm} ct={ct}");
                        c_fd(x);
                        r_fd(y);
                    }
                }
            }

            let cx = (d.ccctx.0)();
            let rx = (d.ccctx.1)();
            for lvl in [1, 9, 19] {
                for &shape in &[Shape::Text, Shape::Random] {
                    for &len in &[0usize, 1, 7000, 150_000] {
                        let src = gen(shape, len, &mut rng);
                        let sp = nnsrc(&src);
                        let cap = (d.bound.0)(len).max(64);
                        let mut f1 = vec![0u8; cap];
                        let mut f2 = vec![0u8; cap];
                        let n1 = c_cud(
                            cx,
                            f1.as_mut_ptr() as *mut c_void,
                            cap,
                            sp,
                            len,
                            dp,
                            dict.len(),
                            lvl,
                        );
                        let n2 = r_cud(
                            rx,
                            f2.as_mut_ptr() as *mut c_void,
                            cap,
                            sp,
                            len,
                            dp,
                            dict.len(),
                            lvl,
                        );
                        assert_eq!(
                            n1, n2,
                            "compress_usingDict d={dname} lvl={lvl} {shape:?} len={len}"
                        );
                        if (d.is_err.0)(n1) != 0 {
                            continue;
                        }
                        assert_bytes_eq("compress_usingDict frame", &f1[..n1], &f2[..n2]);

                        let dx = (d.cdctx.0)();
                        let dy = (d.cdctx.1)();
                        let mut o1 = vec![0xAAu8; len + 8];
                        let mut o2 = vec![0xAAu8; len + 8];
                        let r1 = c_ud(
                            dx,
                            o1.as_mut_ptr() as *mut c_void,
                            o1.len(),
                            f2.as_ptr() as *const c_void,
                            n2,
                            a as *const c_void,
                        );
                        let r2 = r_ud(
                            dy,
                            o2.as_mut_ptr() as *mut c_void,
                            o2.len(),
                            f1.as_ptr() as *const c_void,
                            n1,
                            b as *const c_void,
                        );
                        let tag = format!("usingDDict d={dname} lvl={lvl} {shape:?} len={len}");
                        assert_eq!(r1, r2, "{tag}: rc");
                        assert_eq!((d.ecode.0)(r1), (d.ecode.1)(r2), "{tag}: ecode");
                        assert_bytes_eq(&tag, &o1, &o2);
                        if (d.is_err.0)(r1) == 0 {
                            assert_bytes_eq(&format!("{tag} roundtrip"), &src, &o1[..r1]);
                        }
                        (d.fdctx.0)(dx);
                        (d.fdctx.1)(dy);

                        for rmd in [ZSTD_rmd_refSingleDDict, ZSTD_rmd_refMultipleDDicts] {
                            for path in 0..5 {
                                let dx = (d.cdctx.0)();
                                let dy = (d.cdctx.1)();
                                let sa = c_setdp(dx, ZSTD_d_refMultipleDDicts, rmd);
                                let sb = r_setdp(dy, ZSTD_d_refMultipleDDicts, rmd);
                                assert_eq!(sa, sb, "d_refMultipleDDicts({rmd}) rc");
                                let (ra, rb) = match path {
                                    0 => (
                                        c_rf(dx, a as *const c_void),
                                        r_rf(dy, b as *const c_void),
                                    ),
                                    1 => (c_ldd(dx, dp, dict.len()), r_ldd(dy, dp, dict.len())),
                                    2 => (c_ldb(dx, dp, dict.len()), r_ldb(dy, dp, dict.len())),
                                    3 => (
                                        c_lda(
                                            dx,
                                            dp,
                                            dict.len(),
                                            ZSTD_dlm_byRef,
                                            ZSTD_dct_rawContent,
                                            3,
                                        ),
                                        r_lda(
                                            dy,
                                            dp,
                                            dict.len(),
                                            ZSTD_dlm_byRef,
                                            ZSTD_dct_rawContent,
                                            3,
                                        ),
                                    ),
                                    _ => (
                                        c_lda(
                                            dx,
                                            dp,
                                            dict.len(),
                                            ZSTD_dlm_byCopy,
                                            ZSTD_dct_fullDict,
                                            3,
                                        ),
                                        r_lda(
                                            dy,
                                            dp,
                                            dict.len(),
                                            ZSTD_dlm_byCopy,
                                            ZSTD_dct_fullDict,
                                            3,
                                        ),
                                    ),
                                };
                                let tag = format!(
                                    "ddict-path{path} rmd={rmd} d={dname} lvl={lvl} {shape:?} len={len}"
                                );
                                assert_eq!(ra, rb, "{tag}: setup rc");
                                assert_eq!((d.ecode.0)(ra), (d.ecode.1)(rb), "{tag}: setup ecode");
                                if (d.is_err.0)(ra) == 0 {
                                    let mut o1 = vec![0xAAu8; len + 8];
                                    let mut o2 = vec![0xAAu8; len + 8];
                                    let r1 = (d.dd.0)(
                                        dx,
                                        o1.as_mut_ptr() as *mut c_void,
                                        o1.len(),
                                        f2.as_ptr() as *const c_void,
                                        n2,
                                    );
                                    let r2 = (d.dd.1)(
                                        dy,
                                        o2.as_mut_ptr() as *mut c_void,
                                        o2.len(),
                                        f1.as_ptr() as *const c_void,
                                        n1,
                                    );
                                    assert_eq!(r1, r2, "{tag}: decode rc");
                                    assert_eq!((d.ecode.0)(r1), (d.ecode.1)(r2), "{tag}: ecode");
                                    assert_bytes_eq(&tag, &o1, &o2);
                                }
                                (d.fdctx.0)(dx);
                                (d.fdctx.1)(dy);
                            }
                        }
                    }
                }
            }
            (d.fcctx.0)(cx);
            (d.fcctx.1)(rx);
            c_fd(a);
            r_fd(b);
            c_fd(a2);
            r_fd(b2);
        }
        let dx = (d.cdctx.0)();
        let dy = (d.cdctx.1)();
        assert_eq!(
            c_rf(dx, std::ptr::null()),
            r_rf(dy, std::ptr::null()),
            "refDDict(NULL)"
        );
        (d.fdctx.0)(dx);
        (d.fdctx.1)(dy);
    }
}

// ============== CONFIGS: setCParams / setFParams / setParams ===============

#[test]
fn b_set_cparams_fparams() {
    let d = d();
    let (c_sc, r_sc) = fnpair!(
        "ZSTD_CCtx_setCParams",
        unsafe extern "C" fn(*mut c_void, ZSTD_compressionParameters) -> size_t
    );
    let (c_sf, r_sf) = fnpair!(
        "ZSTD_CCtx_setFParams",
        unsafe extern "C" fn(*mut c_void, ZSTD_frameParameters) -> size_t
    );
    let (c_sp, r_sp) = fnpair!(
        "ZSTD_CCtx_setParams",
        unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> size_t
    );
    let (c_gp, r_gp) = fnpair!(
        "ZSTD_getParams",
        unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_parameters
    );
    let mut rng = Rng::new(0xCF11);
    unsafe {
        for lvl in [-3, 1, 6, 12, 19, 22] {
            for &srch in &[0u64, 1000, 1_000_000, ZSTD_CONTENTSIZE_UNKNOWN] {
                let p1 = c_gp(lvl, srch, 0);
                let p2 = r_gp(lvl, srch, 0);
                assert_eq!(p1, p2, "getParams lvl={lvl} srch={srch}");
                for &shape in &[Shape::Text, Shape::Random] {
                    for &len in &[0usize, 1, 8000, 150_000] {
                        let src = gen(shape, len, &mut rng);
                        for which in 0..3 {
                            let tag = format!(
                                "setParams{which} lvl={lvl} srch={srch} {shape:?} len={len}"
                            );
                            diff_with_setup(
                                &d,
                                &src,
                                &tag,
                                &|c, r| match which {
                                    0 => {
                                        if r {
                                            r_sc(c, p2.cParams)
                                        } else {
                                            c_sc(c, p1.cParams)
                                        }
                                    }
                                    1 => {
                                        if r {
                                            r_sf(c, p2.fParams)
                                        } else {
                                            c_sf(c, p1.fParams)
                                        }
                                    }
                                    _ => {
                                        if r {
                                            r_sp(c, p2)
                                        } else {
                                            c_sp(c, p1)
                                        }
                                    }
                                },
                                None,
                            );
                        }
                    }
                }
            }
        }
        // out-of-range cParams must be rejected identically
        for bad in [
            ZSTD_compressionParameters { windowLog: 99, chainLog: 0, hashLog: 0, searchLog: 0, minMatch: 0, targetLength: 0, strategy: 0 },
            ZSTD_compressionParameters { windowLog: 0, chainLog: 99, hashLog: 0, searchLog: 0, minMatch: 0, targetLength: 0, strategy: 0 },
            ZSTD_compressionParameters { windowLog: 0, chainLog: 0, hashLog: 99, searchLog: 0, minMatch: 0, targetLength: 0, strategy: 0 },
            ZSTD_compressionParameters { windowLog: 0, chainLog: 0, hashLog: 0, searchLog: 99, minMatch: 0, targetLength: 0, strategy: 0 },
            ZSTD_compressionParameters { windowLog: 0, chainLog: 0, hashLog: 0, searchLog: 0, minMatch: 99, targetLength: 0, strategy: 0 },
            ZSTD_compressionParameters { windowLog: 0, chainLog: 0, hashLog: 0, searchLog: 0, minMatch: 0, targetLength: 0, strategy: 99 },
        ] {
            let cc = (d.ccctx.0)();
            let rc = (d.ccctx.1)();
            let a = c_sc(cc, bad);
            let b = r_sc(rc, bad);
            assert_eq!(a, b, "setCParams(bad {bad:?}) rc");
            assert_eq!((d.ecode.0)(a), (d.ecode.1)(b), "setCParams(bad) ecode");
            (d.fcctx.0)(cc);
            (d.fcctx.1)(rc);
        }
    }
}

// ============== CONFIGS: copyCCtx / copyDCtx with dictionaries =============

#[test]
fn b_copy_contexts_with_dict() {
    let d = d();
    let (c_cb, r_cb) = fnpair!(
        "ZSTD_compressBegin_usingDict",
        unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t
    );
    let (c_cp, r_cp) = fnpair!(
        "ZSTD_copyCCtx",
        unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> size_t
    );
    let (c_cc, r_cc) = fnpair!(
        "ZSTD_compressContinue",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );
    let (c_ce, r_ce) = fnpair!(
        "ZSTD_compressEnd",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );
    let (c_db, r_db) = fnpair!(
        "ZSTD_decompressBegin_usingDict",
        unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t
    );
    let (c_dcp, r_dcp) =
        fnpair!("ZSTD_copyDCtx", unsafe extern "C" fn(*mut c_void, *const c_void));

    let mut rng = Rng::new(0xC0B1);
    let dict = trained_dict(&mut rng, 8192);
    unsafe {
        for lvl in [1, 6, 19] {
            for &shape in &[Shape::Text, Shape::Mixed, Shape::Random] {
                for &len in &[0usize, 1, 5000, 200_000] {
                    let src = gen(shape, len, &mut rng);
                    let cap = (d.bound.0)(len).max(1024) + 8192;
                    let tag = format!("copyCCtx lvl={lvl} {shape:?} len={len}");

                    let a1 = (d.ccctx.0)();
                    let b1 = (d.ccctx.1)();
                    let a2 = (d.ccctx.0)();
                    let b2 = (d.ccctx.1)();
                    let ra = c_cb(a1, nn(&dict), dict.len(), lvl);
                    let rb = r_cb(b1, nn(&dict), dict.len(), lvl);
                    assert_eq!(ra, rb, "{tag}: compressBegin_usingDict rc");
                    let ra = c_cp(a2, a1 as *const c_void, len as u64);
                    let rb = r_cp(b2, b1 as *const c_void, len as u64);
                    assert_eq!(ra, rb, "{tag}: copyCCtx rc");
                    assert_eq!((d.ecode.0)(ra), (d.ecode.1)(rb), "{tag}: copyCCtx ecode");
                    if (d.is_err.0)(ra) == 0 {
                        let mut o1 = vec![0xAAu8; cap];
                        let mut o2 = vec![0xAAu8; cap];
                        let mut p1 = 0usize;
                        let mut p2 = 0usize;
                        if len == 0 {
                            let x = c_ce(
                                a2,
                                o1.as_mut_ptr() as *mut c_void,
                                cap,
                                std::ptr::null(),
                                0,
                            );
                            let y = r_ce(
                                b2,
                                o2.as_mut_ptr() as *mut c_void,
                                cap,
                                std::ptr::null(),
                                0,
                            );
                            assert_eq!(x, y, "{tag}: compressEnd(empty) rc");
                            if (d.is_err.0)(x) == 0 {
                                p1 = x;
                                p2 = y;
                            }
                        } else {
                            let mut pos = 0usize;
                            let chunk = 40_000usize;
                            while pos < len {
                                let n = chunk.min(len - pos);
                                let last = pos + n == len;
                                let f1 = if last { c_ce } else { c_cc };
                                let f2 = if last { r_ce } else { r_cc };
                                let x = f1(
                                    a2,
                                    o1.as_mut_ptr().add(p1) as *mut c_void,
                                    cap - p1,
                                    src.as_ptr().add(pos) as *const c_void,
                                    n,
                                );
                                let y = f2(
                                    b2,
                                    o2.as_mut_ptr().add(p2) as *mut c_void,
                                    cap - p2,
                                    src.as_ptr().add(pos) as *const c_void,
                                    n,
                                );
                                assert_eq!(x, y, "{tag}: step at {pos}");
                                assert_eq!(
                                    (d.ecode.0)(x),
                                    (d.ecode.1)(y),
                                    "{tag}: step ecode at {pos}"
                                );
                                if (d.is_err.0)(x) != 0 {
                                    break;
                                }
                                p1 += x;
                                p2 += y;
                                pos += n;
                            }
                        }
                        assert_bytes_eq(&tag, &o1[..p1], &o2[..p2]);

                        if p1 > 0 {
                            let da = (d.cdctx.0)();
                            let db = (d.cdctx.1)();
                            let da2 = (d.cdctx.0)();
                            let db2 = (d.cdctx.1)();
                            assert_eq!(
                                c_db(da, nn(&dict), dict.len()),
                                r_db(db, nn(&dict), dict.len()),
                                "{tag}: decompressBegin_usingDict rc"
                            );
                            c_dcp(da2, da as *const c_void);
                            r_dcp(db2, db as *const c_void);
                            let mut q1 = vec![0xAAu8; len + 8];
                            let mut q2 = vec![0xAAu8; len + 8];
                            let x = (d.dd.0)(
                                da2,
                                q1.as_mut_ptr() as *mut c_void,
                                q1.len(),
                                o2.as_ptr() as *const c_void,
                                p2,
                            );
                            let y = (d.dd.1)(
                                db2,
                                q2.as_mut_ptr() as *mut c_void,
                                q2.len(),
                                o1.as_ptr() as *const c_void,
                                p1,
                            );
                            assert_eq!(x, y, "{tag}: copyDCtx decode rc");
                            assert_eq!((d.ecode.0)(x), (d.ecode.1)(y), "{tag}: copyDCtx ecode");
                            assert_bytes_eq(&format!("{tag}: copyDCtx out"), &q1, &q2);
                            (d.fdctx.0)(da);
                            (d.fdctx.1)(db);
                            (d.fdctx.0)(da2);
                            (d.fdctx.1)(db2);
                        }
                    }
                    (d.fcctx.0)(a1);
                    (d.fcctx.1)(b1);
                    (d.fcctx.0)(a2);
                    (d.fcctx.1)(b2);
                }
            }
        }
    }
}

// ============== CONFIGS: dictionary across resets and streaming ============

#[test]
fn b_dict_across_resets_and_streams() {
    let d = d();
    let (c_ld, r_ld) = fnpair!("ZSTD_CCtx_loadDictionary", FnLoadDict);
    let (c_cs, r_cs) = fnpair!("ZSTD_compressStream2", FnStream);
    let mut rng = Rng::new(0xA5E7);
    let dict = trained_dict(&mut rng, 8192);
    unsafe {
        for reset in [
            ZSTD_reset_session_only,
            ZSTD_reset_parameters,
            ZSTD_reset_session_and_parameters,
        ] {
            for &shape in &[Shape::Text, Shape::Random] {
                for &len in &[0usize, 1, 30_000, 150_000] {
                    let src = gen(shape, len, &mut rng);
                    let sp = nnsrc(&src);
                    let cap = (d.bound.0)(len).max(64);
                    let cc = (d.ccctx.0)();
                    let rc = (d.ccctx.1)();
                    assert_eq!(
                        c_ld(cc, nn(&dict), dict.len()),
                        r_ld(rc, nn(&dict), dict.len()),
                        "loadDictionary rc"
                    );
                    let mut o1 = vec![0xAAu8; cap];
                    let mut o2 = vec![0xAAu8; cap];
                    let n1 = (d.c2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, len);
                    let n2 = (d.c2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, len);
                    assert_eq!(n1, n2, "first frame rc");
                    assert_bytes_eq("first frame", &o1[..n1], &o2[..n2]);

                    let a = (d.reset_c.0)(cc, reset);
                    let b = (d.reset_c.1)(rc, reset);
                    assert_eq!(a, b, "reset({reset}) rc");
                    assert_eq!((d.ecode.0)(a), (d.ecode.1)(b), "reset({reset}) ecode");
                    let mut q1 = vec![0xAAu8; cap];
                    let mut q2 = vec![0xAAu8; cap];
                    let m1 = (d.c2.0)(cc, q1.as_mut_ptr() as *mut c_void, cap, sp, len);
                    let m2 = (d.c2.1)(rc, q2.as_mut_ptr() as *mut c_void, cap, sp, len);
                    let tag = format!("post-reset({reset}) {shape:?} len={len}");
                    assert_eq!(m1, m2, "{tag}: frame rc");
                    if (d.is_err.0)(m1) == 0 {
                        assert_bytes_eq(&tag, &q1[..m1], &q2[..m2]);
                    }

                    // and through the streaming path with a tiny output buffer
                    assert_eq!(
                        (d.reset_c.0)(cc, ZSTD_reset_session_only),
                        (d.reset_c.1)(rc, ZSTD_reset_session_only)
                    );
                    let mut s1 = Vec::new();
                    let mut s2 = Vec::new();
                    for (which, ctx) in [(0usize, cc), (1usize, rc)] {
                        let f = if which == 0 { c_cs } else { r_cs };
                        let out: &mut Vec<u8> = if which == 0 { &mut s1 } else { &mut s2 };
                        let mut buf = vec![0xAAu8; 97];
                        let mut ib = ZSTD_inBuffer {
                            src: sp,
                            size: len,
                            pos: 0,
                        };
                        loop {
                            let mut ob = ZSTD_outBuffer {
                                dst: buf.as_mut_ptr() as *mut c_void,
                                size: buf.len(),
                                pos: 0,
                            };
                            let r = f(ctx, &mut ob, &mut ib, ZSTD_e_end);
                            out.extend_from_slice(&buf[..ob.pos]);
                            if (0usize.wrapping_sub(r)) <= 120 || r == 0 {
                                break;
                            }
                        }
                    }
                    assert_bytes_eq(&format!("{tag}: streamed"), &s1, &s2);
                    (d.fcctx.0)(cc);
                    (d.fcctx.1)(rc);
                }
            }
        }
    }
}
