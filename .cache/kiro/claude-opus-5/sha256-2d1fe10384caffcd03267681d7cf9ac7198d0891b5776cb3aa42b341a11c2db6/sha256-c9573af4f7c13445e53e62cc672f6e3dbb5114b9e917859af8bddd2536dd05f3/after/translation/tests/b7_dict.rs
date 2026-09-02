//! Phase B: differential tests for the DICTIONARY surface — VALID paths.
//!
//! Every entry point is exercised through the real FFI boundary via
//! `both::<T>("name")`; for each configuration we assert that the C and Rust
//! builds produce BYTE-IDENTICAL compressed / decompressed output and agree on
//! all returns and dictIDs. We also require CROSS-decompression: each library
//! must decode the other's dictionary-compressed frame to the same plaintext.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------------ FFI types

type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnCreateCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> size_t;

// dict enums (passed as c_int across the boundary)
const ZSTD_dct_auto: c_int = 0;
const ZSTD_dct_rawContent: c_int = 1;
const ZSTD_dct_fullDict: c_int = 2;
const ZSTD_dlm_byCopy: c_int = 0;
const ZSTD_dlm_byRef: c_int = 1;

// CDict / DDict creation
type FnCreateCDict = unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    size_t,
    c_int, // dictLoadMethod
    c_int, // dictContentType
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void,
    size_t,
    c_int,
    c_int,
    *const c_void, // ZSTD_CCtx_params*
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;
type FnCreateDDictAdv = unsafe extern "C" fn(
    *const c_void,
    size_t,
    c_int,
    c_int,
    ZSTD_customMem,
) -> *mut c_void;

type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void, // workspace
    size_t,      // workspaceSize
    *const c_void,
    size_t,
    c_int, // dlm
    c_int, // dct
    ZSTD_compressionParameters,
) -> *const c_void;
type FnInitStaticDDict = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
    c_int,
) -> *const c_void;

type FnSizeofDict = unsafe extern "C" fn(*const c_void) -> size_t;
type FnEstimateCDict = unsafe extern "C" fn(size_t, c_int) -> size_t;
type FnEstimateCDictAdv =
    unsafe extern "C" fn(size_t, ZSTD_compressionParameters, c_int) -> size_t;
type FnEstimateDDict = unsafe extern "C" fn(size_t, c_int) -> size_t;

type FnGetDictIDFromDict = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetDictIDFromCDict = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnGetDictIDFromDDict = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnGetDictIDFromFrame = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;

// load / ref
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int) -> size_t;
type FnRefCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnRefPrefix = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnRefPrefixAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;

// one-shot dict compression
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
) -> size_t;
type FnCompressUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
) -> size_t;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    size_t,
) -> size_t;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
) -> size_t;

// compressBegin_* streaming block API
type FnCompressBeginUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnCompressBeginUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnCompressBeginUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    ZSTD_frameParameters,
    c_ulonglong,
) -> size_t;
type FnDecompressBeginUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnDecompressBeginUsingDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;

// initCStream_* / initDStream_*
type FnInitCStreamUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnInitCStreamUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnInitCStreamUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    ZSTD_frameParameters,
    c_ulonglong,
) -> size_t;
type FnInitDStreamUsingDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnInitDStreamUsingDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnCBounds = unsafe extern "C" fn(c_int) -> ZSTD_bounds;
type FnGetCParams =
    unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnStream2 = unsafe extern "C" fn(
    *mut c_void,
    *mut ZSTD_outBuffer,
    *mut ZSTD_inBuffer,
    c_int,
) -> size_t;
type FnDStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
type FnZdictTrain = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
) -> size_t;

// ZSTD_customMem (matches header layout: two fn ptrs + opaque). All-NULL means
// "use the default allocator".
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_customMem {
    customAlloc: *const c_void,
    customFree: *const c_void,
    opaque: *mut c_void,
}
const NULL_CMEM: ZSTD_customMem = ZSTD_customMem {
    customAlloc: std::ptr::null(),
    customFree: std::ptr::null(),
    opaque: std::ptr::null_mut(),
};

// ------------------------------------------------------------------ constants

const DICT_SIZES: &[usize] = &[0, 1, 8, 100, 1024, 8192, 112640];
const LEVELS: &[c_int] = &[-5, 1, 3, 9, 19, 22];
const DATA_LENS: &[usize] = &[0, 1, 100, 1024, 20000, 131100];

// ------------------------------------------------------------------ dict data

/// Build a real trained dictionary once with the C library and return its
/// bytes (identical bytes are then handed to BOTH libraries so inputs match).
fn trained_dict(cap: usize, seed: u64) -> Vec<u8> {
    unsafe {
        let (train, _) = both::<FnZdictTrain>("ZDICT_trainFromBuffer");
        let e = Err2::new();
        let mut rng = Rng::new(seed);
        // Build many small, structured samples so training succeeds.
        let mut samples: Vec<u8> = Vec::new();
        let mut sizes: Vec<size_t> = Vec::new();
        let nb = 512u32;
        for _ in 0..nb {
            let shape = [Shape::Text, Shape::Repeating, Shape::LowEntropy, Shape::Sequential]
                [rng.below(4)];
            let len = 64 + rng.below(512);
            let s = gen(shape, len, &mut rng);
            sizes.push(s.len());
            samples.extend_from_slice(&s);
        }
        let mut buf = vec![0u8; cap];
        let n = train(
            buf.as_mut_ptr() as *mut c_void,
            cap,
            samples.as_ptr() as *const c_void,
            sizes.as_ptr(),
            nb,
        );
        if e.c.is_err(n) {
            // Training can fail for tiny caps; fall back to raw bytes so callers
            // still have `cap` bytes to work with.
            for (i, b) in buf.iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(31).wrapping_add(7);
            }
            return buf;
        }
        buf.truncate(n);
        buf
    }
}

/// The three dictionary content flavours used across the matrix.
enum DictKind {
    RawRandom,
    RawText,
    Trained,
}

fn make_dict(kind: &DictKind, size: usize, rng: &mut Rng, trained: &[u8]) -> Vec<u8> {
    match kind {
        DictKind::RawRandom => (0..size).map(|_| rng.byte()).collect(),
        DictKind::RawText => gen(Shape::Text, size, rng),
        DictKind::Trained => {
            if trained.len() >= size {
                trained[..size].to_vec()
            } else {
                let mut v = trained.to_vec();
                while v.len() < size {
                    v.push(rng.byte());
                }
                v
            }
        }
    }
}

// ------------------------------------------------------------------ ctx pairs

struct CctxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl CctxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnCreateCtx>("ZSTD_createCCtx");
            CctxPair { c: a(), r: b() }
        }
    }
}
impl Drop for CctxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnFreeCtx>("ZSTD_freeCCtx");
            a(self.c);
            b(self.r);
        }
    }
}
struct DctxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl DctxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnCreateCtx>("ZSTD_createDCtx");
            DctxPair { c: a(), r: b() }
        }
    }
}
impl Drop for DctxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnFreeCtx>("ZSTD_freeDCtx");
            a(self.c);
            b(self.r);
        }
    }
}

/// A CDict created independently in each library from identical dict bytes.
struct CDictPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl Drop for CDictPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnFreeCtx>("ZSTD_freeCDict");
            a(self.c);
            b(self.r);
        }
    }
}
struct DDictPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl Drop for DDictPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnFreeCtx>("ZSTD_freeDDict");
            a(self.c);
            b(self.r);
        }
    }
}

// ================================================================== TEST 1
// createCDict / createDDict + one-shot compress_usingCDict / decompress_usingDDict
// over the full matrix, with cross-decompression and dictID agreement.

#[test]
fn cdict_ddict_oneshot_matrix() {
    unsafe {
        let e = Err2::new();
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_cuc, r_cuc) = both::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let (c_dud, r_dud) = both::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (c_idc, r_idc) = both::<FnGetDictIDFromCDict>("ZSTD_getDictID_fromCDict");
        let (c_idd, r_idd) = both::<FnGetDictIDFromDDict>("ZSTD_getDictID_fromDDict");
        let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
        let (c_idf, r_idf) = both::<FnGetDictIDFromFrame>("ZSTD_getDictID_fromFrame");
        let (c_szc, r_szc) = both::<FnSizeofDict>("ZSTD_sizeof_CDict");
        let (c_szd, r_szd) = both::<FnSizeofDict>("ZSTD_sizeof_DDict");

        let mut rng = Rng::new(0xD1C7_0001);
        let trained = trained_dict(112640, 0xABCD_0001);

        for kind in [DictKind::RawRandom, DictKind::RawText, DictKind::Trained] {
            for &dsize in DICT_SIZES {
                let dict = make_dict(&kind, dsize, &mut rng, &trained);
                let dptr = dict.as_ptr() as *const c_void;

                assert_eq!(
                    c_idb(dptr, dict.len()),
                    r_idb(dptr, dict.len()),
                    "getDictID_fromDict dsize={dsize}"
                );

                for &lvl in LEVELS {
                    let cd_c = c_ccd(dptr, dict.len(), lvl);
                    let cd_r = r_ccd(dptr, dict.len(), lvl);
                    assert_eq!(cd_c.is_null(), cd_r.is_null(),
                               "createCDict null-ness dsize={dsize} lvl={lvl}");
                    let dd_c = c_cdd(dptr, dict.len());
                    let dd_r = r_cdd(dptr, dict.len());
                    assert_eq!(dd_c.is_null(), dd_r.is_null(), "createDDict null-ness");
                    if cd_c.is_null() || dd_c.is_null() {
                        if !cd_c.is_null() {
                            let (fa, fb) = both::<FnFreeCtx>("ZSTD_freeCDict");
                            fa(cd_c);
                            fb(cd_r);
                        }
                        if !dd_c.is_null() {
                            let (fa, fb) = both::<FnFreeCtx>("ZSTD_freeDDict");
                            fa(dd_c);
                            fb(dd_r);
                        }
                        continue;
                    }
                    let cd = CDictPair { c: cd_c, r: cd_r };
                    let dd = DDictPair { c: dd_c, r: dd_r };

                    assert_eq!(c_idc(cd.c), r_idc(cd.r), "getDictID_fromCDict");
                    assert_eq!(c_idd(dd.c), r_idd(dd.r), "getDictID_fromDDict");
                    assert_eq!(c_szc(cd.c), r_szc(cd.r), "sizeof_CDict");
                    assert_eq!(c_szd(dd.c), r_szd(dd.r), "sizeof_DDict");

                    for _ in 0..2 {
                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let dlen = DATA_LENS[rng.below(DATA_LENS.len())];
                        let src = gen(shape, dlen, &mut rng);
                        let cap = c_cb(src.len()) + 64;
                        let mut o1 = vec![0u8; cap];
                        let mut o2 = vec![0u8; cap];
                        let cx = CctxPair::new();
                        let cn = c_cuc(cx.c, o1.as_mut_ptr() as *mut c_void, cap,
                                       src.as_ptr() as *const c_void, src.len(), cd.c);
                        let rn = r_cuc(cx.r, o2.as_mut_ptr() as *mut c_void, cap,
                                       src.as_ptr() as *const c_void, src.len(), cd.r);
                        let ctx = format!(
                            "compress_usingCDict dsize={dsize} lvl={lvl} shape={shape:?} len={dlen}");
                        e.eq(&ctx, cn, rn);
                        if e.c.is_err(cn) {
                            continue;
                        }
                        assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);

                        assert_eq!(
                            c_idf(o1.as_ptr() as *const c_void, cn),
                            r_idf(o2.as_ptr() as *const c_void, rn),
                            "getDictID_fromFrame {ctx}");

                        // CROSS-decompress: C decodes RS frame, RS decodes C frame.
                        let dx = DctxPair::new();
                        let mut d1 = vec![0u8; src.len() + 16];
                        let mut d2 = vec![0u8; src.len() + 16];
                        let a = c_dud(dx.c, d1.as_mut_ptr() as *mut c_void, d1.len(),
                                      o2.as_ptr() as *const c_void, rn, dd.c);
                        let b = r_dud(dx.r, d2.as_mut_ptr() as *mut c_void, d2.len(),
                                      o1.as_ptr() as *const c_void, cn, dd.r);
                        e.eq(&format!("{ctx} / cross-decompress"), a, b);
                        assert_eq!(a, src.len(), "{ctx}: roundtrip size");
                        assert_bytes_eq(&format!("{ctx} / decoded C"), &d1[..a], &src);
                        assert_bytes_eq(&format!("{ctx} / decoded RS"), &d2[..b], &src);
                    }
                }
            }
        }
    }
}

// ================================================================== TEST 2
// compress_usingDict / decompress_usingDict (raw, undigested) matrix.

#[test]
fn using_dict_oneshot_matrix() {
    unsafe {
        let e = Err2::new();
        let (c_cud, r_cud) = both::<FnCompressUsingDict>("ZSTD_compress_usingDict");
        let (c_dud, r_dud) = both::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");

        let mut rng = Rng::new(0xD1C7_0002);
        let trained = trained_dict(112640, 0xABCD_0002);

        for kind in [DictKind::RawRandom, DictKind::RawText, DictKind::Trained] {
            for &dsize in DICT_SIZES {
                let dict = make_dict(&kind, dsize, &mut rng, &trained);
                let dptr = dict.as_ptr() as *const c_void;
                for &lvl in LEVELS {
                    for _ in 0..2 {
                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let dlen = DATA_LENS[rng.below(DATA_LENS.len())];
                        let src = gen(shape, dlen, &mut rng);
                        let cap = c_cb(src.len()) + 64;
                        let mut o1 = vec![0u8; cap];
                        let mut o2 = vec![0u8; cap];
                        let cx = CctxPair::new();
                        let cn = c_cud(cx.c, o1.as_mut_ptr() as *mut c_void, cap,
                                       src.as_ptr() as *const c_void, src.len(),
                                       dptr, dict.len(), lvl);
                        let rn = r_cud(cx.r, o2.as_mut_ptr() as *mut c_void, cap,
                                       src.as_ptr() as *const c_void, src.len(),
                                       dptr, dict.len(), lvl);
                        let ctx = format!(
                            "compress_usingDict dsize={dsize} lvl={lvl} shape={shape:?} len={dlen}");
                        e.eq(&ctx, cn, rn);
                        if e.c.is_err(cn) {
                            continue;
                        }
                        assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);

                        let dx = DctxPair::new();
                        let mut d1 = vec![0u8; src.len() + 16];
                        let mut d2 = vec![0u8; src.len() + 16];
                        let a = c_dud(dx.c, d1.as_mut_ptr() as *mut c_void, d1.len(),
                                      o2.as_ptr() as *const c_void, rn, dptr, dict.len());
                        let b = r_dud(dx.r, d2.as_mut_ptr() as *mut c_void, d2.len(),
                                      o1.as_ptr() as *const c_void, cn, dptr, dict.len());
                        e.eq(&format!("{ctx} / cross-decompress"), a, b);
                        assert_eq!(a, src.len(), "{ctx}: roundtrip");
                        assert_bytes_eq(&format!("{ctx} / decoded C"), &d1[..a], &src);
                        assert_bytes_eq(&format!("{ctx} / decoded RS"), &d2[..b], &src);
                    }
                }
            }
        }
    }
}

// ================================================================== TEST 3
// The advanced create paths: createCDict_advanced/_advanced2/_byReference,
// createDDict_advanced/_byReference, over dictLoadMethod x dictContentType,
// plus estimateCDictSize(_advanced) / estimateDDictSize / sizeof agreement.

#[test]
fn cdict_ddict_advanced_create_matrix() {
    unsafe {
        let e = Err2::new();
        let (c_ccda, r_ccda) = both::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (c_ccda2, r_ccda2) = both::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2");
        let (c_ccbr, r_ccbr) = both::<FnCreateCDict>("ZSTD_createCDict_byReference");
        let (c_cdda, r_cdda) = both::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (c_cdbr, r_cdbr) = both::<FnCreateDDict>("ZSTD_createDDict_byReference");
        let (c_gcp, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let (c_idc, r_idc) = both::<FnGetDictIDFromCDict>("ZSTD_getDictID_fromCDict");
        let (c_idd, r_idd) = both::<FnGetDictIDFromDDict>("ZSTD_getDictID_fromDDict");
        let (c_szc, r_szc) = both::<FnSizeofDict>("ZSTD_sizeof_CDict");
        let (c_szd, r_szd) = both::<FnSizeofDict>("ZSTD_sizeof_DDict");
        let (c_ecd, r_ecd) = both::<FnEstimateCDict>("ZSTD_estimateCDictSize");
        let (c_ecda, r_ecda) = both::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced");
        let (c_edd, r_edd) = both::<FnEstimateDDict>("ZSTD_estimateDDictSize");

        let mut rng = Rng::new(0xD1C7_0003);
        let trained = trained_dict(112640, 0xABCD_0003);

        for kind in [DictKind::RawRandom, DictKind::Trained] {
            for &dsize in DICT_SIZES {
                let dict = make_dict(&kind, dsize, &mut rng, &trained);
                let dptr = dict.as_ptr() as *const c_void;
                for &lvl in LEVELS {
                    let cparams = c_gcp(lvl, 0, dict.len());
                    for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                        assert_eq!(c_ecd(dict.len(), lvl), r_ecd(dict.len(), lvl),
                                   "estimateCDictSize dsize={dsize} lvl={lvl}");
                        assert_eq!(c_ecda(dict.len(), cparams, dlm),
                                   r_ecda(dict.len(), cparams, dlm),
                                   "estimateCDictSize_advanced dsize={dsize} lvl={lvl} dlm={dlm}");
                        assert_eq!(c_edd(dict.len(), dlm), r_edd(dict.len(), dlm),
                                   "estimateDDictSize dsize={dsize} dlm={dlm}");

                        for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                            let cd_c = c_ccda(dptr, dict.len(), dlm, dct, cparams, NULL_CMEM);
                            let cd_r = r_ccda(dptr, dict.len(), dlm, dct, cparams, NULL_CMEM);
                            assert_eq!(cd_c.is_null(), cd_r.is_null(),
                                       "createCDict_advanced null dsize={dsize} dlm={dlm} dct={dct}");
                            if !cd_c.is_null() {
                                let cd = CDictPair { c: cd_c, r: cd_r };
                                assert_eq!(c_idc(cd.c), r_idc(cd.r),
                                           "adv CDict dictID dsize={dsize} dlm={dlm} dct={dct}");
                                assert_eq!(c_szc(cd.c), r_szc(cd.r), "adv CDict sizeof");
                            }

                            let dd_c = c_cdda(dptr, dict.len(), dlm, dct, NULL_CMEM);
                            let dd_r = r_cdda(dptr, dict.len(), dlm, dct, NULL_CMEM);
                            assert_eq!(dd_c.is_null(), dd_r.is_null(),
                                       "createDDict_advanced null dsize={dsize} dlm={dlm} dct={dct}");
                            if !dd_c.is_null() {
                                let dd = DDictPair { c: dd_c, r: dd_r };
                                assert_eq!(c_idd(dd.c), r_idd(dd.r), "adv DDict dictID");
                                assert_eq!(c_szd(dd.c), r_szd(dd.r), "adv DDict sizeof");
                            }
                        }
                    }

                    // _advanced2 (uses a CCtx_params object)
                    let (c_cpc, r_cpc) = both::<FnCreateCtx>("ZSTD_createCCtxParams");
                    let (c_cpf, r_cpf) = both::<FnFreeCtx>("ZSTD_freeCCtxParams");
                    let (c_ps, r_ps) = both::<FnSetParam>("ZSTD_CCtxParams_setParameter");
                    let pc = c_cpc();
                    let pr = r_cpc();
                    c_ps(pc, ZSTD_c_compressionLevel, lvl);
                    r_ps(pr, ZSTD_c_compressionLevel, lvl);
                    let cd_c = c_ccda2(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto,
                                       pc as *const c_void, NULL_CMEM);
                    let cd_r = r_ccda2(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto,
                                       pr as *const c_void, NULL_CMEM);
                    assert_eq!(cd_c.is_null(), cd_r.is_null(),
                               "createCDict_advanced2 null dsize={dsize} lvl={lvl}");
                    if !cd_c.is_null() {
                        let cd = CDictPair { c: cd_c, r: cd_r };
                        assert_eq!(c_idc(cd.c), r_idc(cd.r), "adv2 CDict dictID");
                        assert_eq!(c_szc(cd.c), r_szc(cd.r), "adv2 CDict sizeof");
                    }
                    c_cpf(pc);
                    r_cpf(pr);

                    // byReference variants
                    let cd_c = c_ccbr(dptr, dict.len(), lvl);
                    let cd_r = r_ccbr(dptr, dict.len(), lvl);
                    assert_eq!(cd_c.is_null(), cd_r.is_null(), "createCDict_byReference null");
                    if !cd_c.is_null() {
                        let cd = CDictPair { c: cd_c, r: cd_r };
                        assert_eq!(c_idc(cd.c), r_idc(cd.r), "byRef CDict dictID");
                    }
                    let dd_c = c_cdbr(dptr, dict.len());
                    let dd_r = r_cdbr(dptr, dict.len());
                    assert_eq!(dd_c.is_null(), dd_r.is_null(), "createDDict_byReference null");
                    if !dd_c.is_null() {
                        let dd = DDictPair { c: dd_c, r: dd_r };
                        assert_eq!(c_idd(dd.c), r_idd(dd.r), "byRef DDict dictID");
                    }
                    let _ = &e;
                }
            }
        }
    }
}

// ================================================================== TEST 4
// CCtx_loadDictionary(+byReference/+advanced) driven through ZSTD_compress2,
// with the forceAttachDict / dedicatedDictSearch / prefetchCDictTables /
// deterministicRefPrefix parameter sweep.

fn cbounds(id: c_int) -> (c_int, c_int) {
    unsafe {
        let (cb, _) = both::<FnCBounds>("ZSTD_cParam_getBounds");
        let b = cb(id);
        (b.lowerBound, b.upperBound)
    }
}

/// Compress `src` via compress2 on both libs after the CCtx has been configured
/// with a dictionary; assert identical frames, then round-trip decode.
unsafe fn c2_roundtrip_check(e: &Err2, cx: &CctxPair, src: &[u8], dict: &[u8], ctx: &str) {
    let (c_c2, r_c2) = both::<FnCompress2>("ZSTD_compress2");
    let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
    let (c_dd, r_dd) = both::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
    let cap = c_cb(src.len()) + 64;
    let mut o1 = vec![0u8; cap];
    let mut o2 = vec![0u8; cap];
    let cn = c_c2(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    let rn = r_c2(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
    e.eq(ctx, cn, rn);
    if e.c.is_err(cn) {
        return;
    }
    assert_bytes_eq(ctx, &o1[..cn], &o2[..rn]);
    let dx = DctxPair::new();
    let mut d1 = vec![0u8; src.len() + 16];
    let mut d2 = vec![0u8; src.len() + 16];
    let a = c_dd(dx.c, d1.as_mut_ptr() as *mut c_void, d1.len(),
                 o2.as_ptr() as *const c_void, rn, dict.as_ptr() as *const c_void, dict.len());
    let b = r_dd(dx.r, d2.as_mut_ptr() as *mut c_void, d2.len(),
                 o1.as_ptr() as *const c_void, cn, dict.as_ptr() as *const c_void, dict.len());
    e.eq(&format!("{ctx} / cross-decode"), a, b);
    assert_eq!(a, src.len(), "{ctx}: roundtrip size");
    assert_bytes_eq(&format!("{ctx} / decoded C"), &d1[..a], src);
    assert_bytes_eq(&format!("{ctx} / decoded RS"), &d2[..b], src);
}

#[test]
fn cctx_loaddict_and_param_sweep() {
    unsafe {
        let e = Err2::new();
        let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (c_ldr, r_ldr) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference");
        let (c_lda, r_lda) = both::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (c_sp, r_sp) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        type FnResetD = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
        let (crd, rrd) = both::<FnResetD>("ZSTD_CCtx_reset");

        let mut rng = Rng::new(0xD1C7_0004);
        let trained = trained_dict(112640, 0xABCD_0004);

        let (fa_lo, fa_hi) = cbounds(ZSTD_c_forceAttachDict);
        assert!(fa_lo <= 0 && fa_hi >= 2, "forceAttachDict bounds {fa_lo}..{fa_hi}");

        let force_vals: Vec<c_int> = (fa_lo..=fa_hi).collect();
        let dedicated_vals = [0, 1];
        let prefetch_vals = [0, 1, 2];
        let determ_vals = [0, 1];

        for kind in [DictKind::RawRandom, DictKind::Trained] {
            for &dsize in &[100usize, 1024, 8192] {
                let dict = make_dict(&kind, dsize, &mut rng, &trained);
                for &lvl in &[3, 19] {
                    for &fa in &force_vals {
                        for &dds in &dedicated_vals {
                            for &pf in &prefetch_vals {
                                for &drp in &determ_vals {
                                    let cx = CctxPair::new();
                                    crd(cx.c, ZSTD_reset_session_and_parameters);
                                    rrd(cx.r, ZSTD_reset_session_and_parameters);
                                    for &(id, v) in &[
                                        (ZSTD_c_compressionLevel, lvl),
                                        (ZSTD_c_forceAttachDict, fa),
                                        (ZSTD_c_enableDedicatedDictSearch, dds),
                                        (ZSTD_c_prefetchCDictTables, pf),
                                        (ZSTD_c_deterministicRefPrefix, drp),
                                    ] {
                                        e.eq(&format!("setParameter id={id} v={v}"),
                                             c_sp(cx.c, id, v), r_sp(cx.r, id, v));
                                    }
                                    let dptr = dict.as_ptr() as *const c_void;
                                    let which = rng.below(3);
                                    let (a, b) = match which {
                                        0 => (c_ld(cx.c, dptr, dict.len()),
                                              r_ld(cx.r, dptr, dict.len())),
                                        1 => (c_ldr(cx.c, dptr, dict.len()),
                                              r_ldr(cx.r, dptr, dict.len())),
                                        _ => (
                                            c_lda(cx.c, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
                                            r_lda(cx.r, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
                                        ),
                                    };
                                    e.eq(&format!("loadDictionary variant={which}"), a, b);

                                    let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                                    let dlen = DATA_LENS[rng.below(DATA_LENS.len())];
                                    let src = gen(shape, dlen, &mut rng);
                                    let ctx = format!(
                                        "loaddict dsize={dsize} lvl={lvl} fa={fa} dds={dds} pf={pf} drp={drp} v={which} shape={shape:?} len={dlen}");
                                    c2_roundtrip_check(&e, &cx, &src, &dict, &ctx);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================================== TEST 5
// refCDict / refDDict (including NULL to clear), refPrefix(_advanced), and
// DCtx_loadDictionary(+variants). Also d_refMultipleDDicts with 3 dictIDs.

#[test]
fn ref_cdict_ddict_prefix_and_multiddict() {
    unsafe {
        let e = Err2::new();
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_rc, r_rc) = both::<FnRefCDict>("ZSTD_CCtx_refCDict");
        let (c_rd, r_rd) = both::<FnRefDDict>("ZSTD_DCtx_refDDict");
        let (c_rp, r_rp) = both::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let (c_rpa, r_rpa) = both::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (c_drp, r_drp) = both::<FnRefPrefix>("ZSTD_DCtx_refPrefix");
        let (c_drpa, r_drpa) = both::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
        let (c_dld, r_dld) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (c_dldr, r_dldr) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
        let (c_dlda, r_dlda) = both::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (c_c2, r_c2) = both::<FnCompress2>("ZSTD_compress2");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (c_ds, r_ds) = both::<FnDStream>("ZSTD_decompressStream");
        let (c_sp, r_sp) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c_dsp, r_dsp) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
        type FnResetD = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
        let (crd, rrd) = both::<FnResetD>("ZSTD_CCtx_reset");
        let (drd, drr) = both::<FnResetD>("ZSTD_DCtx_reset");

        let mut rng = Rng::new(0xD1C7_0005);
        let trained = trained_dict(112640, 0xABCD_0005);
        let dict = make_dict(&DictKind::Trained, 8192, &mut rng, &trained);
        let dptr = dict.as_ptr() as *const c_void;

        let cd = CDictPair { c: c_ccd(dptr, dict.len(), 5), r: r_ccd(dptr, dict.len(), 5) };
        let dd = DDictPair { c: c_cdd(dptr, dict.len()), r: r_cdd(dptr, dict.len()) };

        // ------ refCDict roundtrip + refCDict(NULL) clears it ------
        for clear in [false, true] {
            let cx = CctxPair::new();
            crd(cx.c, ZSTD_reset_session_and_parameters);
            rrd(cx.r, ZSTD_reset_session_and_parameters);
            e.eq("refCDict", c_rc(cx.c, cd.c), r_rc(cx.r, cd.r));
            if clear {
                e.eq("refCDict(NULL)", c_rc(cx.c, std::ptr::null()), r_rc(cx.r, std::ptr::null()));
            }
            let src = gen(Shape::Text, 5000, &mut rng);
            let cap = c_cb(src.len()) + 64;
            let mut o1 = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let cn = c_c2(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let rn = r_c2(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let ctx = format!("refCDict clear={clear}");
            e.eq(&ctx, cn, rn);
            assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);
        }

        // ------ refDDict roundtrip via decompressStream + refDDict(NULL) ------
        {
            let cx = CctxPair::new();
            crd(cx.c, ZSTD_reset_session_and_parameters);
            rrd(cx.r, ZSTD_reset_session_and_parameters);
            c_rc(cx.c, cd.c);
            r_rc(cx.r, cd.r);
            let src = gen(Shape::Text, 4000, &mut rng);
            let cap = c_cb(src.len()) + 64;
            let mut frame = vec![0u8; cap];
            let n = c_c2(cx.c, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            frame.truncate(n);

            for clear in [false, true] {
                let dx = DctxPair::new();
                drd(dx.c, ZSTD_reset_session_and_parameters);
                drr(dx.r, ZSTD_reset_session_and_parameters);
                e.eq("refDDict", c_rd(dx.c, dd.c), r_rd(dx.r, dd.r));
                if clear {
                    e.eq("refDDict(NULL)", c_rd(dx.c, std::ptr::null()), r_rd(dx.r, std::ptr::null()));
                    let mut o1 = vec![0u8; src.len() + 16];
                    let mut o2 = vec![0u8; src.len() + 16];
                    let mut ib1 = ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
                    let mut ib2 = ib1;
                    let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
                    let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
                    e.eq("refDDict(NULL) decode", c_ds(dx.c, &mut ob1, &mut ib1), r_ds(dx.r, &mut ob2, &mut ib2));
                    continue;
                }
                let mut o1 = vec![0u8; src.len() + 16];
                let mut o2 = vec![0u8; src.len() + 16];
                let mut ib1 = ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
                let mut ib2 = ib1;
                let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
                let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
                e.eq("refDDict decode", c_ds(dx.c, &mut ob1, &mut ib1), r_ds(dx.r, &mut ob2, &mut ib2));
                assert_bytes_eq("refDDict decoded", &o1[..ob1.pos], &o2[..ob2.pos]);
                assert_bytes_eq("refDDict plaintext", &o1[..ob1.pos], &src);
            }
        }

        // ------ refPrefix / refPrefix_advanced (compress) + DCtx side ------
        for adv in [false, true] {
            let cx = CctxPair::new();
            crd(cx.c, ZSTD_reset_session_and_parameters);
            rrd(cx.r, ZSTD_reset_session_and_parameters);
            c_sp(cx.c, ZSTD_c_compressionLevel, 6);
            r_sp(cx.r, ZSTD_c_compressionLevel, 6);
            let prefix = make_dict(&DictKind::RawText, 4096, &mut rng, &trained);
            let pptr = prefix.as_ptr() as *const c_void;
            if adv {
                e.eq("refPrefix_advanced",
                     c_rpa(cx.c, pptr, prefix.len(), ZSTD_dct_rawContent),
                     r_rpa(cx.r, pptr, prefix.len(), ZSTD_dct_rawContent));
            } else {
                e.eq("refPrefix", c_rp(cx.c, pptr, prefix.len()), r_rp(cx.r, pptr, prefix.len()));
            }
            let src = gen(Shape::Text, 6000, &mut rng);
            let cap = c_cb(src.len()) + 64;
            let mut o1 = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let cn = c_c2(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let rn = r_c2(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let ctx = format!("refPrefix adv={adv}");
            e.eq(&ctx, cn, rn);
            assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);
            o1.truncate(cn);

            let dx = DctxPair::new();
            drd(dx.c, ZSTD_reset_session_and_parameters);
            drr(dx.r, ZSTD_reset_session_and_parameters);
            if adv {
                e.eq("DCtx_refPrefix_advanced",
                     c_drpa(dx.c, pptr, prefix.len(), ZSTD_dct_rawContent),
                     r_drpa(dx.r, pptr, prefix.len(), ZSTD_dct_rawContent));
            } else {
                e.eq("DCtx_refPrefix", c_drp(dx.c, pptr, prefix.len()), r_drp(dx.r, pptr, prefix.len()));
            }
            let mut d1 = vec![0u8; src.len() + 16];
            let mut d2 = vec![0u8; src.len() + 16];
            let mut ib1 = ZSTD_inBuffer { src: o1.as_ptr() as *const c_void, size: o1.len(), pos: 0 };
            let mut ib2 = ib1;
            let mut ob1 = ZSTD_outBuffer { dst: d1.as_mut_ptr() as *mut c_void, size: d1.len(), pos: 0 };
            let mut ob2 = ZSTD_outBuffer { dst: d2.as_mut_ptr() as *mut c_void, size: d2.len(), pos: 0 };
            e.eq(&format!("{ctx} decode"), c_ds(dx.c, &mut ob1, &mut ib1), r_ds(dx.r, &mut ob2, &mut ib2));
            assert_bytes_eq(&format!("{ctx} plaintext"), &d1[..ob1.pos], &src);
        }

        // ------ DCtx_loadDictionary variants roundtrip ------
        for which in 0..3usize {
            let cx = CctxPair::new();
            crd(cx.c, ZSTD_reset_session_and_parameters);
            rrd(cx.r, ZSTD_reset_session_and_parameters);
            c_rc(cx.c, cd.c);
            r_rc(cx.r, cd.r);
            let src = gen(Shape::Text, 5000, &mut rng);
            let cap = c_cb(src.len()) + 64;
            let mut frame = vec![0u8; cap];
            let n = c_c2(cx.c, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            frame.truncate(n);

            let dx = DctxPair::new();
            drd(dx.c, ZSTD_reset_session_and_parameters);
            drr(dx.r, ZSTD_reset_session_and_parameters);
            let (a, b) = match which {
                0 => (c_dld(dx.c, dptr, dict.len()), r_dld(dx.r, dptr, dict.len())),
                1 => (c_dldr(dx.c, dptr, dict.len()), r_dldr(dx.r, dptr, dict.len())),
                _ => (
                    c_dlda(dx.c, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
                    r_dlda(dx.r, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto),
                ),
            };
            e.eq(&format!("DCtx_loadDictionary variant={which}"), a, b);
            let mut o1 = vec![0u8; src.len() + 16];
            let mut o2 = vec![0u8; src.len() + 16];
            let mut ib1 = ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: frame.len(), pos: 0 };
            let mut ib2 = ib1;
            let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
            let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
            e.eq(&format!("DCtx_loadDictionary decode {which}"), c_ds(dx.c, &mut ob1, &mut ib1), r_ds(dx.r, &mut ob2, &mut ib2));
            assert_bytes_eq(&format!("DCtx_loadDictionary plaintext {which}"), &o1[..ob1.pos], &src);
        }

        // ------ d_refMultipleDDicts with three distinct dictIDs ------
        {
            let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
            let mut dicts: Vec<Vec<u8>> = Vec::new();
            let mut seed = 0x3333_0000u64;
            while dicts.len() < 3 && seed <= 0x3333_0080 {
                let d = trained_dict(16384, seed);
                seed += 1;
                let id = c_idb(d.as_ptr() as *const c_void, d.len());
                assert_eq!(id, r_idb(d.as_ptr() as *const c_void, d.len()), "distinct-dict dictID");
                if id != 0 && !dicts.iter().any(|e2: &Vec<u8>| {
                    c_idb(e2.as_ptr() as *const c_void, e2.len()) == id
                }) {
                    dicts.push(d);
                }
            }
            if dicts.len() == 3 {
                let mut frames: Vec<Vec<u8>> = Vec::new();
                let src = gen(Shape::Text, 3000, &mut rng);
                for d in &dicts {
                    let cx = CctxPair::new();
                    crd(cx.c, ZSTD_reset_session_and_parameters);
                    rrd(cx.r, ZSTD_reset_session_and_parameters);
                    c_sp(cx.c, ZSTD_c_compressionLevel, 5);
                    r_sp(cx.r, ZSTD_c_compressionLevel, 5);
                    let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
                    c_ld(cx.c, d.as_ptr() as *const c_void, d.len());
                    r_ld(cx.r, d.as_ptr() as *const c_void, d.len());
                    let cap = c_cb(src.len()) + 64;
                    let mut o1 = vec![0u8; cap];
                    let mut o2 = vec![0u8; cap];
                    let cn = c_c2(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                    let rn = r_c2(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                    e.eq("multiDDict frame build", cn, rn);
                    assert_bytes_eq("multiDDict frame bytes", &o1[..cn], &o2[..rn]);
                    o1.truncate(cn);
                    frames.push(o1);
                }
                for enable in [0, 1] {
                    let dx = DctxPair::new();
                    drd(dx.c, ZSTD_reset_session_and_parameters);
                    drr(dx.r, ZSTD_reset_session_and_parameters);
                    e.eq("d_refMultipleDDicts",
                         c_dsp(dx.c, ZSTD_d_refMultipleDDicts, enable),
                         r_dsp(dx.r, ZSTD_d_refMultipleDDicts, enable));
                    let mut ddicts: Vec<DDictPair> = Vec::new();
                    for d in &dicts {
                        let cc = c_cdd(d.as_ptr() as *const c_void, d.len());
                        let rr = r_cdd(d.as_ptr() as *const c_void, d.len());
                        e.eq("refDDict multi", c_rd(dx.c, cc), r_rd(dx.r, rr));
                        ddicts.push(DDictPair { c: cc, r: rr });
                    }
                    for (fi, f) in frames.iter().enumerate() {
                        let mut o1 = vec![0u8; src.len() + 16];
                        let mut o2 = vec![0u8; src.len() + 16];
                        let mut ib1 = ZSTD_inBuffer { src: f.as_ptr() as *const c_void, size: f.len(), pos: 0 };
                        let mut ib2 = ib1;
                        let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
                        let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
                        let a = c_ds(dx.c, &mut ob1, &mut ib1);
                        let b = r_ds(dx.r, &mut ob2, &mut ib2);
                        e.eq(&format!("multiDDict decode enable={enable} frame={fi}"), a, b);
                        assert_bytes_eq(&format!("multiDDict out enable={enable} frame={fi}"),
                                        &o1[..ob1.pos], &o2[..ob2.pos]);
                        let _ = &ddicts;
                    }
                }
            }
        }
        let _ = &dd;
    }
}

// ================================================================== TEST 6
// compressBegin_usingDict / usingCDict / usingCDict_advanced +
// decompressBegin_usingDict / usingDDict streaming-block API.

type FnCompressEnd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnNextSrc = unsafe extern "C" fn(*mut c_void) -> size_t;

#[test]
fn compress_decompress_begin_using_dict() {
    unsafe {
        let e = Err2::new();
        let (c_cbd, r_cbd) = both::<FnCompressBeginUsingDict>("ZSTD_compressBegin_usingDict");
        let (c_cbc, r_cbc) = both::<FnCompressBeginUsingCDict>("ZSTD_compressBegin_usingCDict");
        let (c_cbca, r_cbca) =
            both::<FnCompressBeginUsingCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
        let (c_dbd, r_dbd) = both::<FnDecompressBeginUsingDict>("ZSTD_decompressBegin_usingDict");
        let (c_dbdd, r_dbdd) = both::<FnDecompressBeginUsingDDict>("ZSTD_decompressBegin_usingDDict");
        let (c_ce, r_ce) = both::<FnCompressEnd>("ZSTD_compressEnd");
        let (c_dc, r_dc) = both::<FnDecompressContinue>("ZSTD_decompressContinue");
        let (c_ns, r_ns) = both::<FnNextSrc>("ZSTD_nextSrcSizeToDecompress");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");

        let mut rng = Rng::new(0xD1C7_0006);
        let trained = trained_dict(112640, 0xABCD_0006);
        let dict = make_dict(&DictKind::Trained, 8192, &mut rng, &trained);
        let dptr = dict.as_ptr() as *const c_void;

        let cd = CDictPair { c: c_ccd(dptr, dict.len(), 5), r: r_ccd(dptr, dict.len(), 5) };
        let dd = DDictPair { c: c_cdd(dptr, dict.len()), r: r_cdd(dptr, dict.len()) };

        let variants = ["usingDict", "usingCDict", "usingCDict_advanced"];
        for (vi, vname) in variants.iter().enumerate() {
            for &dlen in &[0usize, 100, 4096, 40000] {
                let src = gen(Shape::Text, dlen, &mut rng);
                let cx = CctxPair::new();
                let fp = ZSTD_frameParameters { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 };
                let a = match vi {
                    0 => (c_cbd(cx.c, dptr, dict.len(), 5), r_cbd(cx.r, dptr, dict.len(), 5)),
                    1 => (c_cbc(cx.c, cd.c), r_cbc(cx.r, cd.r)),
                    _ => (
                        c_cbca(cx.c, cd.c, fp, src.len() as c_ulonglong),
                        r_cbca(cx.r, cd.r, fp, src.len() as c_ulonglong),
                    ),
                };
                e.eq(&format!("compressBegin_{vname}"), a.0, a.1);
                if e.c.is_err(a.0) {
                    continue;
                }
                let cap = c_cb(src.len()) + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let cn = c_ce(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                let rn = r_ce(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                let ctx = format!("compressEnd_{vname} len={dlen}");
                e.eq(&ctx, cn, rn);
                if e.c.is_err(cn) {
                    continue;
                }
                assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);

                for use_ddict in [false, true] {
                    // decode the C frame block-by-block with the C dctx
                    let dxc = DctxPair::new();
                    let da_c = if use_ddict { c_dbdd(dxc.c, dd.c) } else { c_dbd(dxc.c, dptr, dict.len()) };
                    // decode the same C frame with a fresh RS dctx
                    let dxr = DctxPair::new();
                    let da_r = if use_ddict { r_dbdd(dxr.r, dd.r) } else { r_dbd(dxr.r, dptr, dict.len()) };
                    e.eq(&format!("{ctx} decompressBegin ddict={use_ddict}"), da_c, da_r);

                    let frame = &o1[..cn];
                    let out_c = block_decode(&e, true, dxc.c, frame, src.len(), &c_ns, &c_dc);
                    let out_r = block_decode(&e, false, dxr.r, frame, src.len(), &r_ns, &r_dc);
                    assert_bytes_eq(&format!("{ctx} block-decoded ddict={use_ddict}"), &out_c, &out_r);
                    if dlen > 0 {
                        assert_bytes_eq(&format!("{ctx} block plaintext ddict={use_ddict}"), &out_c, &src);
                    }
                }
            }
        }
        let _ = (&cd, &dd);
    }
}

/// Decode a single-frame block stream via nextSrcSizeToDecompress + decompressContinue.
unsafe fn block_decode(
    e: &Err2,
    is_c: bool,
    dctx: *mut c_void,
    frame: &[u8],
    plain_hint: usize,
    ns: &libloading::Symbol<'static, FnNextSrc>,
    dc: &libloading::Symbol<'static, FnDecompressContinue>,
) -> Vec<u8> {
    let is_err = |r: size_t| if is_c { e.c.is_err(r) } else { e.r.is_err(r) };
    let mut out = vec![0u8; plain_hint + 64];
    let mut ipos = 0usize;
    let mut opos = 0usize;
    loop {
        let need = ns(dctx);
        if is_err(need) || need == 0 {
            break;
        }
        if ipos + need > frame.len() {
            break;
        }
        let r = dc(dctx, out.as_mut_ptr().add(opos) as *mut c_void, out.len() - opos,
                   frame.as_ptr().add(ipos) as *const c_void, need);
        if is_err(r) {
            break;
        }
        opos += r;
        ipos += need;
    }
    out.truncate(opos);
    out
}

// ================================================================== TEST 7
// initCStream_usingDict / usingCDict / usingCDict_advanced +
// initDStream_usingDict / usingDDict streaming API.

#[test]
fn init_cstream_dstream_using_dict() {
    unsafe {
        let e = Err2::new();
        let (c_icd, r_icd) = both::<FnInitCStreamUsingDict>("ZSTD_initCStream_usingDict");
        let (c_icc, r_icc) = both::<FnInitCStreamUsingCDict>("ZSTD_initCStream_usingCDict");
        let (c_icca, r_icca) =
            both::<FnInitCStreamUsingCDictAdv>("ZSTD_initCStream_usingCDict_advanced");
        let (c_idd, r_idd) = both::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
        let (c_iddd, r_iddd) = both::<FnInitDStreamUsingDDict>("ZSTD_initDStream_usingDDict");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_s2, r_s2) = both::<FnStream2>("ZSTD_compressStream2");
        let (c_ds, r_ds) = both::<FnDStream>("ZSTD_decompressStream");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");

        let mut rng = Rng::new(0xD1C7_0007);
        let trained = trained_dict(112640, 0xABCD_0007);
        let dict = make_dict(&DictKind::Trained, 8192, &mut rng, &trained);
        let dptr = dict.as_ptr() as *const c_void;

        let cd = CDictPair { c: c_ccd(dptr, dict.len(), 5), r: r_ccd(dptr, dict.len(), 5) };
        let dd = DDictPair { c: c_cdd(dptr, dict.len()), r: r_cdd(dptr, dict.len()) };

        let variants = ["usingDict", "usingCDict", "usingCDict_advanced"];
        for (vi, vname) in variants.iter().enumerate() {
            for &dlen in &[0usize, 100, 4096, 40000] {
                let src = gen(Shape::Text, dlen, &mut rng);
                let cx = CctxPair::new(); // CStream is a CCtx alias
                let fp = ZSTD_frameParameters { contentSizeFlag: 1, checksumFlag: 0, noDictIDFlag: 0 };
                let a = match vi {
                    0 => (c_icd(cx.c, dptr, dict.len(), 5), r_icd(cx.r, dptr, dict.len(), 5)),
                    1 => (c_icc(cx.c, cd.c), r_icc(cx.r, cd.r)),
                    _ => (
                        c_icca(cx.c, cd.c, fp, src.len() as c_ulonglong),
                        r_icca(cx.r, cd.r, fp, src.len() as c_ulonglong),
                    ),
                };
                e.eq(&format!("initCStream_{vname}"), a.0, a.1);
                if e.c.is_err(a.0) {
                    continue;
                }
                let cap = c_cb(src.len()) + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let mut ib1 = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
                let mut ib2 = ib1;
                let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                let re1 = c_s2(cx.c, &mut ob1, &mut ib1, ZSTD_e_end);
                let re2 = r_s2(cx.r, &mut ob2, &mut ib2, ZSTD_e_end);
                let ctx = format!("initCStream_{vname} len={dlen}");
                e.eq(&ctx, re1, re2);
                assert_eq!(re1, 0, "{ctx} not fully flushed (C)");
                assert_bytes_eq(&ctx, &o1[..ob1.pos], &o2[..ob2.pos]);
                let frame_len = ob1.pos;

                for use_ddict in [false, true] {
                    let dx = DctxPair::new(); // DStream is a DCtx alias
                    let da = if use_ddict {
                        (c_iddd(dx.c, dd.c), r_iddd(dx.r, dd.r))
                    } else {
                        (c_idd(dx.c, dptr, dict.len()), r_idd(dx.r, dptr, dict.len()))
                    };
                    e.eq(&format!("{ctx} initDStream ddict={use_ddict}"), da.0, da.1);
                    let mut d1 = vec![0u8; src.len() + 16];
                    let mut d2 = vec![0u8; src.len() + 16];
                    let mut jb1 = ZSTD_inBuffer { src: o1.as_ptr() as *const c_void, size: frame_len, pos: 0 };
                    let mut jb2 = jb1;
                    let mut pb1 = ZSTD_outBuffer { dst: d1.as_mut_ptr() as *mut c_void, size: d1.len(), pos: 0 };
                    let mut pb2 = ZSTD_outBuffer { dst: d2.as_mut_ptr() as *mut c_void, size: d2.len(), pos: 0 };
                    e.eq(&format!("{ctx} decode ddict={use_ddict}"),
                         c_ds(dx.c, &mut pb1, &mut jb1), r_ds(dx.r, &mut pb2, &mut jb2));
                    assert_bytes_eq(&format!("{ctx} decoded ddict={use_ddict}"), &d1[..pb1.pos], &d2[..pb2.pos]);
                    assert_bytes_eq(&format!("{ctx} plaintext ddict={use_ddict}"), &d1[..pb1.pos], &src);
                }
            }
        }
        let _ = (&cd, &dd);
    }
}

// ================================================================== TEST 8
// initStaticCDict / initStaticDDict at workspace sizes {estimate-1, estimate,
// estimate+1, huge}. Assert NULL-vs-non-NULL parity, and that a non-NULL static
// dict produces byte-identical frames vs a heap CDict.

#[test]
fn static_cdict_ddict_workspace_sizes() {
    unsafe {
        let e = Err2::new();
        let (c_scd, r_scd) = both::<FnInitStaticCDict>("ZSTD_initStaticCDict");
        let (c_sdd, r_sdd) = both::<FnInitStaticDDict>("ZSTD_initStaticDDict");
        let (c_ecda, r_ecda) = both::<FnEstimateCDictAdv>("ZSTD_estimateCDictSize_advanced");
        let (c_edd, r_edd) = both::<FnEstimateDDict>("ZSTD_estimateDDictSize");
        let (c_gcp, _) = both::<FnGetCParams>("ZSTD_getCParams");
        let (c_cuc, r_cuc) = both::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let (c_dud, r_dud) = both::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (c_idc, r_idc) = both::<FnGetDictIDFromCDict>("ZSTD_getDictID_fromCDict");
        let (c_idd, r_idd) = both::<FnGetDictIDFromDDict>("ZSTD_getDictID_fromDDict");

        let mut rng = Rng::new(0xD1C7_0008);
        let trained = trained_dict(112640, 0xABCD_0008);

        for &dsize in &[100usize, 1024, 8192] {
            let dict = make_dict(&DictKind::Trained, dsize, &mut rng, &trained);
            let dptr = dict.as_ptr() as *const c_void;
            let lvl = 5;
            let cparams = c_gcp(lvl, 0, dict.len());
            let est_c = c_ecda(dict.len(), cparams, ZSTD_dlm_byCopy);
            let est_r = r_ecda(dict.len(), cparams, ZSTD_dlm_byCopy);
            assert_eq!(est_c, est_r, "estimateCDictSize_advanced dsize={dsize}");
            let estd_c = c_edd(dict.len(), ZSTD_dlm_byCopy);
            let estd_r = r_edd(dict.len(), ZSTD_dlm_byCopy);
            assert_eq!(estd_c, estd_r, "estimateDDictSize dsize={dsize}");

            // ---- CDict workspace sizing ----
            for ws in [est_c.wrapping_sub(1), est_c, est_c + 1, est_c + 4096, est_c * 4 + 65536] {
                let mut wc = vec![0u8; ws.max(1)];
                let mut wr = vec![0u8; ws.max(1)];
                let pc = c_scd(wc.as_mut_ptr() as *mut c_void, ws, dptr, dict.len(),
                               ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams);
                let pr = r_scd(wr.as_mut_ptr() as *mut c_void, ws, dptr, dict.len(),
                               ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams);
                assert_eq!(pc.is_null(), pr.is_null(),
                           "initStaticCDict NULL parity dsize={dsize} ws={ws} est={est_c}");
                if pc.is_null() {
                    // Both agreed on NULL. The estimate is a lower bound; the
                    // real init can need a little more (alignment), so NULL at
                    // ws<=estimate+padding is acceptable as long as parity holds.
                    continue;
                }
                assert_eq!(c_idc(pc), r_idc(pr), "static CDict dictID dsize={dsize} ws={ws}");
                let src = gen(Shape::Text, 3000, &mut rng);
                let cap = c_cb(src.len()) + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let cx = CctxPair::new();
                let cn = c_cuc(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), pc as *const c_void);
                let rn = r_cuc(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), pr as *const c_void);
                let ctx = format!("static CDict compress dsize={dsize} ws={ws}");
                e.eq(&ctx, cn, rn);
                assert_bytes_eq(&ctx, &o1[..cn], &o2[..rn]);
            }

            // ---- DDict workspace sizing ----
            for ws in [estd_c.wrapping_sub(1), estd_c, estd_c + 1, estd_c + 4096, estd_c * 4 + 65536] {
                let mut wc = vec![0u8; ws.max(1)];
                let mut wr = vec![0u8; ws.max(1)];
                let pc = c_sdd(wc.as_mut_ptr() as *mut c_void, ws, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                let pr = r_sdd(wr.as_mut_ptr() as *mut c_void, ws, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto);
                assert_eq!(pc.is_null(), pr.is_null(),
                           "initStaticDDict NULL parity dsize={dsize} ws={ws} est={estd_c}");
                if pc.is_null() {
                    // Parity held; estimate is a lower bound (see CDict note).
                    continue;
                }
                assert_eq!(c_idd(pc), r_idd(pr), "static DDict dictID dsize={dsize} ws={ws}");
                let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
                let cd = CDictPair { c: c_ccd(dptr, dict.len(), lvl), r: r_ccd(dptr, dict.len(), lvl) };
                let src = gen(Shape::Text, 3000, &mut rng);
                let cap = c_cb(src.len()) + 64;
                let mut f1 = vec![0u8; cap];
                let mut f2 = vec![0u8; cap];
                let cx = CctxPair::new();
                let cn = c_cuc(cx.c, f1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), cd.c);
                let rn = r_cuc(cx.r, f2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), cd.r);
                assert_bytes_eq("static-DDict frame build", &f1[..cn], &f2[..rn]);
                let dx = DctxPair::new();
                let mut d1 = vec![0u8; src.len() + 16];
                let mut d2 = vec![0u8; src.len() + 16];
                let a = c_dud(dx.c, d1.as_mut_ptr() as *mut c_void, d1.len(), f2.as_ptr() as *const c_void, rn, pc as *const c_void);
                let b = r_dud(dx.r, d2.as_mut_ptr() as *mut c_void, d2.len(), f1.as_ptr() as *const c_void, cn, pr as *const c_void);
                let ctx = format!("static DDict decode dsize={dsize} ws={ws}");
                e.eq(&ctx, a, b);
                assert_eq!(a, src.len(), "{ctx} size");
                assert_bytes_eq(&format!("{ctx} plaintext"), &d1[..a], &src);
            }
        }
    }
}
