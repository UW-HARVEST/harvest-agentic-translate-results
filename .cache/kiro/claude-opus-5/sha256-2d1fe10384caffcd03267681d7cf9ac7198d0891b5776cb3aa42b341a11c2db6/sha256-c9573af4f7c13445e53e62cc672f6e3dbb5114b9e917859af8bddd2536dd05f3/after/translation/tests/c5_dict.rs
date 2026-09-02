//! Phase C: differential tests for the DICTIONARY surface — ERROR paths.
//!
//! Every invalid condition is constructed on BOTH libraries and we assert the
//! two libraries return IDENTICAL error codes (via `Err2::eq`, which compares
//! `ZSTD_getErrorCode`). Fixed RNG seeds, many randomized inputs.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------------ FFI types

type FnCreateCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;

const ZSTD_dct_auto: c_int = 0;
const ZSTD_dct_rawContent: c_int = 1;
const ZSTD_dct_fullDict: c_int = 2;
const ZSTD_dlm_byCopy: c_int = 0;
const ZSTD_dlm_byRef: c_int = 1;
const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30A437;

type FnCreateCDict = unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void, size_t, c_int, c_int, ZSTD_compressionParameters, ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, size_t, c_int, c_int, ZSTD_customMem) -> *mut c_void;

type FnGetDictIDFromDict = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetDictIDFromFrame = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetDictIDFromCDict = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnGetDictIDFromDDict = unsafe extern "C" fn(*const c_void) -> c_uint;

type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int) -> size_t;
type FnRefPrefix = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnRefPrefixAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnRefCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;

type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t,
) -> size_t;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void,
) -> size_t;
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t, c_int,
) -> size_t;

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnResetD = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnStream2 =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int) -> size_t;
type FnDStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
type FnZdictTrain = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint,
) -> size_t;

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

// ------------------------------------------------------------------ dict data

fn trained_dict(cap: usize, seed: u64) -> Vec<u8> {
    unsafe {
        let (train, _) = both::<FnZdictTrain>("ZDICT_trainFromBuffer");
        let e = Err2::new();
        let mut rng = Rng::new(seed);
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
        let n = train(buf.as_mut_ptr() as *mut c_void, cap,
                      samples.as_ptr() as *const c_void, sizes.as_ptr(), nb);
        assert!(!e.c.is_err(n), "trained_dict: training failed for cap={cap}");
        buf.truncate(n);
        buf
    }
}

// ================================================================== TEST 1
// dictSize > 0 with dict == NULL, for every load/create/ref entry point.
// C and Rust must agree (whatever the code is).

#[test]
fn nonzero_size_null_dict() {
    unsafe {
        let e = Err2::new();
        let null = std::ptr::null::<c_void>();
        let sizes: &[usize] = &[1, 8, 100, 1024, 8192];

        let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (c_ldr, r_ldr) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference");
        let (c_lda, r_lda) = both::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (c_dld, r_dld) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (c_dldr, r_dldr) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
        let (c_dlda, r_dlda) = both::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (c_rp, r_rp) = both::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let (c_rpa, r_rpa) = both::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (c_drp, r_drp) = both::<FnRefPrefix>("ZSTD_DCtx_refPrefix");
        let (c_drpa, r_drpa) = both::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");

        for &sz in sizes {
            let cx = CctxPair::new();
            let dx = DctxPair::new();
            e.eq(&format!("CCtx_loadDictionary null sz={sz}"), c_ld(cx.c, null, sz), r_ld(cx.r, null, sz));
            e.eq(&format!("CCtx_loadDictionary_byReference null sz={sz}"), c_ldr(cx.c, null, sz), r_ldr(cx.r, null, sz));
            e.eq(&format!("CCtx_loadDictionary_advanced null sz={sz}"),
                 c_lda(cx.c, null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto),
                 r_lda(cx.r, null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto));
            e.eq(&format!("DCtx_loadDictionary null sz={sz}"), c_dld(dx.c, null, sz), r_dld(dx.r, null, sz));
            e.eq(&format!("DCtx_loadDictionary_byReference null sz={sz}"), c_dldr(dx.c, null, sz), r_dldr(dx.r, null, sz));
            e.eq(&format!("DCtx_loadDictionary_advanced null sz={sz}"),
                 c_dlda(dx.c, null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto),
                 r_dlda(dx.r, null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto));
            e.eq(&format!("CCtx_refPrefix null sz={sz}"), c_rp(cx.c, null, sz), r_rp(cx.r, null, sz));
            e.eq(&format!("CCtx_refPrefix_advanced null sz={sz}"),
                 c_rpa(cx.c, null, sz, ZSTD_dct_rawContent), r_rpa(cx.r, null, sz, ZSTD_dct_rawContent));
            e.eq(&format!("DCtx_refPrefix null sz={sz}"), c_drp(dx.c, null, sz), r_drp(dx.r, null, sz));
            e.eq(&format!("DCtx_refPrefix_advanced null sz={sz}"),
                 c_drpa(dx.c, null, sz, ZSTD_dct_rawContent), r_drpa(dx.r, null, sz, ZSTD_dct_rawContent));
        }

        // create* with NULL dict + nonzero size: null-ness must agree
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_ccda, r_ccda) = both::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (c_cdda, r_cdda) = both::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (c_gcp, _) = both::<unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters>("ZSTD_getCParams");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");
        let cparams = c_gcp(3, 0, 0);
        for &sz in sizes {
            let a = c_ccd(null, sz, 3);
            let b = r_ccd(null, sz, 3);
            assert_eq!(a.is_null(), b.is_null(), "createCDict(NULL,{sz}) null parity");
            c_free_cd(a); r_free_cd(b);
            let a = c_cdd(null, sz);
            let b = r_cdd(null, sz);
            assert_eq!(a.is_null(), b.is_null(), "createDDict(NULL,{sz}) null parity");
            c_free_dd(a); r_free_dd(b);
            let a = c_ccda(null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams, NULL_CMEM);
            let b = r_ccda(null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams, NULL_CMEM);
            assert_eq!(a.is_null(), b.is_null(), "createCDict_advanced(NULL,{sz}) null parity");
            c_free_cd(a); r_free_cd(b);
            let a = c_cdda(null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto, NULL_CMEM);
            let b = r_cdda(null, sz, ZSTD_dlm_byCopy, ZSTD_dct_auto, NULL_CMEM);
            assert_eq!(a.is_null(), b.is_null(), "createDDict_advanced(NULL,{sz}) null parity");
            c_free_dd(a); r_free_dd(b);
        }
    }
}

// ================================================================== TEST 2
// ZSTD_dct_fullDict on a buffer lacking ZSTD_MAGIC_DICTIONARY -> dictionary_wrong.

#[test]
fn fulldict_without_magic() {
    unsafe {
        let e = Err2::new();
        let (c_lda, r_lda) = both::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (c_dlda, r_dlda) = both::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (c_ccda, r_ccda) = both::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (c_cdda, r_cdda) = both::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (c_gcp, _) = both::<unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters>("ZSTD_getCParams");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");
        let cparams = c_gcp(3, 0, 0);

        let mut rng = Rng::new(0xE550_0002);
        for &sz in &[8usize, 100, 1024, 8192] {
            // raw bytes guaranteed NOT to start with the dictionary magic
            let mut dict: Vec<u8> = (0..sz).map(|_| rng.byte()).collect();
            // clobber the first 4 bytes to something != magic
            if sz >= 4 {
                dict[..4].copy_from_slice(&0x11223344u32.to_le_bytes());
            }
            let dptr = dict.as_ptr() as *const c_void;

            let cx = CctxPair::new();
            let dx = DctxPair::new();
            // Loading with dct_fullDict defers validation: the load itself
            // succeeds; the dictionary_wrong error surfaces when the dict is
            // actually used. Assert C/Rust agree at load time...
            let a = c_lda(cx.c, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
            let b = r_lda(cx.r, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
            e.eq(&format!("CCtx_loadDictionary_advanced fullDict no-magic sz={sz}"), a, b);

            let a = c_dlda(dx.c, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
            let b = r_dlda(dx.r, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
            e.eq(&format!("DCtx_loadDictionary_advanced fullDict no-magic sz={sz}"), a, b);

            // ...then drive a compression: the deferred dictionary_wrong must
            // surface identically on both libraries.
            {
                let (c_sp, r_sp) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
                let (c_lda2, r_lda2) = (c_lda.clone(), r_lda.clone());
                let (c_c2, r_c2) =
                    both::<FnCompress2>("ZSTD_compress2");
                let (crd, rrd) = both::<FnResetD>("ZSTD_CCtx_reset");
                let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
                let cx2 = CctxPair::new();
                crd(cx2.c, ZSTD_reset_session_and_parameters);
                rrd(cx2.r, ZSTD_reset_session_and_parameters);
                c_sp(cx2.c, ZSTD_c_compressionLevel, 3);
                r_sp(cx2.r, ZSTD_c_compressionLevel, 3);
                // reload the fullDict/no-magic dictionary on the fresh contexts
                c_lda2(cx2.c, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
                r_lda2(cx2.r, dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict);
                let mut src = vec![0u8; 2000];
                for (i, b) in src.iter_mut().enumerate() { *b = (i as u8).wrapping_mul(3); }
                let cap = c_cb(src.len()) + 64;
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let ca = c_c2(cx2.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                let cb = r_c2(cx2.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
                e.eq(&format!("compress2 after fullDict no-magic sz={sz}"), ca, cb);
                // The deferred failure surfaces identically on both libraries
                // (the exact code is build-dependent); it must be an error.
                assert!(e.c.is_err(ca), "fullDict no-magic compress must fail sz={sz}");
            }

            // create*_advanced with fullDict + no magic -> NULL for both
            let pc = c_ccda(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict, cparams, NULL_CMEM);
            let pr = r_ccda(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict, cparams, NULL_CMEM);
            assert_eq!(pc.is_null(), pr.is_null(), "createCDict_advanced fullDict no-magic null parity sz={sz}");
            assert!(pc.is_null(), "createCDict_advanced fullDict no-magic should be NULL sz={sz}");
            c_free_cd(pc); r_free_cd(pr);
            let pc = c_cdda(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict, NULL_CMEM);
            let pr = r_cdda(dptr, dict.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict, NULL_CMEM);
            assert_eq!(pc.is_null(), pr.is_null(), "createDDict_advanced fullDict no-magic null parity sz={sz}");
            assert!(pc.is_null(), "createDDict_advanced fullDict no-magic should be NULL sz={sz}");
            c_free_dd(pc); r_free_dd(pr);
        }
        let _ = ZSTD_MAGIC_DICTIONARY;
    }
}

fn err_name(e: &Err2, r: size_t, is_c: bool) -> String {
    let cl = if is_c { e.c.classify(r) } else { e.r.classify(r) };
    match cl {
        Ret::Err { name, .. } => name,
        Ret::Ok(_) => String::new(),
    }
}

// ================================================================== TEST 3
// Out-of-range dictContentType / dictLoadMethod enum values across the FFI
// boundary. C enums accept any int; assert C and Rust agree for every value.

#[test]
fn out_of_range_enum_values() {
    unsafe {
        let e = Err2::new();
        let (c_lda, r_lda) = both::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (c_dlda, r_dlda) = both::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (c_rpa, r_rpa) = both::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (c_drpa, r_drpa) = both::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
        let (c_ccda, r_ccda) = both::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let (c_cdda, r_cdda) = both::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (c_gcp, _) = both::<unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters>("ZSTD_getCParams");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");
        let cparams = c_gcp(3, 0, 0);

        let bad: &[c_int] = &[-1, 3, 4, 99, i32::MIN, i32::MAX];
        let mut rng = Rng::new(0xE550_0003);
        let dict = trained_dict(4096, 0xBEEF_0003);
        let dptr = dict.as_ptr() as *const c_void;

        for &dct in bad {
            for &dlm in bad {
                let cx = CctxPair::new();
                let dx = DctxPair::new();
                e.eq(&format!("CCtx_loadDictionary_advanced dct={dct} dlm={dlm}"),
                     c_lda(cx.c, dptr, dict.len(), dlm, dct),
                     r_lda(cx.r, dptr, dict.len(), dlm, dct));
                e.eq(&format!("DCtx_loadDictionary_advanced dct={dct} dlm={dlm}"),
                     c_dlda(dx.c, dptr, dict.len(), dlm, dct),
                     r_dlda(dx.r, dptr, dict.len(), dlm, dct));
                e.eq(&format!("CCtx_refPrefix_advanced dct={dct}"),
                     c_rpa(cx.c, dptr, dict.len(), dct), r_rpa(cx.r, dptr, dict.len(), dct));
                e.eq(&format!("DCtx_refPrefix_advanced dct={dct}"),
                     c_drpa(dx.c, dptr, dict.len(), dct), r_drpa(dx.r, dptr, dict.len(), dct));

                let pc = c_ccda(dptr, dict.len(), dlm, dct, cparams, NULL_CMEM);
                let pr = r_ccda(dptr, dict.len(), dlm, dct, cparams, NULL_CMEM);
                assert_eq!(pc.is_null(), pr.is_null(),
                           "createCDict_advanced dct={dct} dlm={dlm} null parity");
                c_free_cd(pc); r_free_cd(pr);
                let pc = c_cdda(dptr, dict.len(), dlm, dct, NULL_CMEM);
                let pr = r_cdda(dptr, dict.len(), dlm, dct, NULL_CMEM);
                assert_eq!(pc.is_null(), pr.is_null(),
                           "createDDict_advanced dct={dct} dlm={dlm} null parity");
                c_free_dd(pc); r_free_dd(pr);
            }
        }
        let _ = &mut rng;
    }
}

// ================================================================== TEST 4
// Corrupted trained dictionary: sweep every single-byte position of the first
// 256 bytes, set each to 0x00 and 0xFF and flip each of the 8 bits, calling
// several entry points on each mutant and asserting identical results.

#[test]
fn corrupted_trained_dict_byte_sweep() {
    unsafe {
        let e = Err2::new();
        let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (c_dld, r_dld) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");

        let base = trained_dict(8192, 0xBEEF_0004);
        let sweep = base.len().min(256);

        // mutation set per position: 0x00, 0xFF, and 8 single-bit flips
        for pos in 0..sweep {
            let orig = base[pos];
            let mut mutants: Vec<u8> = vec![0x00, 0xFF];
            for bit in 0..8 {
                mutants.push(orig ^ (1u8 << bit));
            }
            for m in mutants {
                if m == orig {
                    continue;
                }
                let mut d = base.clone();
                d[pos] = m;
                let dptr = d.as_ptr() as *const c_void;
                let tag = format!("pos={pos} mut={m:#x}");

                let cx = CctxPair::new();
                let dx = DctxPair::new();
                e.eq(&format!("CCtx_loadDictionary corrupt {tag}"),
                     c_ld(cx.c, dptr, d.len()), r_ld(cx.r, dptr, d.len()));
                e.eq(&format!("DCtx_loadDictionary corrupt {tag}"),
                     c_dld(dx.c, dptr, d.len()), r_dld(dx.r, dptr, d.len()));

                let pc = c_ccd(dptr, d.len(), 3);
                let pr = r_ccd(dptr, d.len(), 3);
                assert_eq!(pc.is_null(), pr.is_null(), "createCDict corrupt null parity {tag}");
                c_free_cd(pc); r_free_cd(pr);
                let pc = c_cdd(dptr, d.len());
                let pr = r_cdd(dptr, d.len());
                assert_eq!(pc.is_null(), pr.is_null(), "createDDict corrupt null parity {tag}");
                c_free_dd(pc); r_free_dd(pr);

                assert_eq!(c_idb(dptr, d.len()), r_idb(dptr, d.len()),
                           "getDictID_fromDict corrupt {tag}");
            }
        }
    }
}

// ================================================================== TEST 5
// Truncated dictionaries: every length from 0 to 64 of a real trained dict.

#[test]
fn truncated_dict_lengths() {
    unsafe {
        let e = Err2::new();
        let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (c_dld, r_dld) = both::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (c_lda, r_lda) = both::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");

        let base = trained_dict(8192, 0xBEEF_0005);
        for len in 0..=64usize {
            let d = &base[..len.min(base.len())];
            let dptr = d.as_ptr() as *const c_void;
            let cx = CctxPair::new();
            let dx = DctxPair::new();
            e.eq(&format!("CCtx_loadDictionary trunc len={len}"), c_ld(cx.c, dptr, d.len()), r_ld(cx.r, dptr, d.len()));
            e.eq(&format!("DCtx_loadDictionary trunc len={len}"), c_dld(dx.c, dptr, d.len()), r_dld(dx.r, dptr, d.len()));
            // fullDict content type on a truncated dict must also agree
            e.eq(&format!("CCtx_loadDictionary_advanced fullDict trunc len={len}"),
                 c_lda(cx.c, dptr, d.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict),
                 r_lda(cx.r, dptr, d.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict));
            let pc = c_ccd(dptr, d.len(), 3);
            let pr = r_ccd(dptr, d.len(), 3);
            assert_eq!(pc.is_null(), pr.is_null(), "createCDict trunc null parity len={len}");
            c_free_cd(pc); r_free_cd(pr);
            let pc = c_cdd(dptr, d.len());
            let pr = r_cdd(dptr, d.len());
            assert_eq!(pc.is_null(), pr.is_null(), "createDDict trunc null parity len={len}");
            c_free_dd(pc); r_free_dd(pr);
            assert_eq!(c_idb(dptr, d.len()), r_idb(dptr, d.len()), "getDictID_fromDict trunc len={len}");
        }
    }
}

// ================================================================== TEST 6
// Decompress a dictionary-compressed frame with NO dictionary -> dictionary_wrong.
// Decompress with the WRONG dictionary (different dictID) -> dictionary_wrong.

#[test]
fn decompress_no_dict_and_wrong_dict() {
    unsafe {
        let e = Err2::new();
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cuc, r_cuc) = both::<unsafe extern "C" fn(*mut c_void,*mut c_void,size_t,*const c_void,size_t,*const c_void)->size_t>("ZSTD_compress_usingCDict");
        let (c_dud, r_dud) = both::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");

        let mut rng = Rng::new(0xE550_0006);
        // two distinct trained dictionaries with distinct dictIDs
        let dictA = trained_dict(8192, 0xAAAA_0006);
        let mut dictB = trained_dict(8192, 0xBBBB_0006);
        let ida = c_idb(dictA.as_ptr() as *const c_void, dictA.len());
        // ensure dictB has a different id
        let mut s = 0xBBBB_0007u64;
        while c_idb(dictB.as_ptr() as *const c_void, dictB.len()) == ida && s < 0xBBBB_0040 {
            dictB = trained_dict(8192, s);
            s += 1;
        }
        assert_eq!(ida, r_idb(dictA.as_ptr() as *const c_void, dictA.len()), "dictA id parity");

        let cd = c_ccd(dictA.as_ptr() as *const c_void, dictA.len(), 5);
        let cdr = r_ccd(dictA.as_ptr() as *const c_void, dictA.len(), 5);

        for &dlen in &[100usize, 5000, 40000] {
            let src = gen(Shape::Text, dlen, &mut rng);
            let cap = c_cb(src.len()) + 64;
            let mut frame = vec![0u8; cap];
            let cx = CctxPair::new();
            let n = c_cuc(cx.c, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), cd);
            frame.truncate(n);

            // 1) no dictionary
            let dx = DctxPair::new();
            let mut o1 = vec![0u8; src.len() + 16];
            let mut o2 = vec![0u8; src.len() + 16];
            let a = c_dud(dx.c, o1.as_mut_ptr() as *mut c_void, o1.len(), frame.as_ptr() as *const c_void, frame.len(), std::ptr::null(), 0);
            let b = r_dud(dx.r, o2.as_mut_ptr() as *mut c_void, o2.len(), frame.as_ptr() as *const c_void, frame.len(), std::ptr::null(), 0);
            e.eq(&format!("decompress no-dict dlen={dlen}"), a, b);
            assert_eq!(e.c.classify(a),
                       Ret::Err { code: E_dictionary_wrong, name: err_name(&e, a, true) },
                       "no-dict decode must be dictionary_wrong dlen={dlen}");

            // 2) wrong dictionary
            let dx2 = DctxPair::new();
            let a = c_dud(dx2.c, o1.as_mut_ptr() as *mut c_void, o1.len(), frame.as_ptr() as *const c_void, frame.len(),
                          dictB.as_ptr() as *const c_void, dictB.len());
            let b = r_dud(dx2.r, o2.as_mut_ptr() as *mut c_void, o2.len(), frame.as_ptr() as *const c_void, frame.len(),
                          dictB.as_ptr() as *const c_void, dictB.len());
            e.eq(&format!("decompress wrong-dict dlen={dlen}"), a, b);
            assert_eq!(e.c.classify(a),
                       Ret::Err { code: E_dictionary_wrong, name: err_name(&e, a, true) },
                       "wrong-dict decode must be dictionary_wrong dlen={dlen}");
        }
        c_free_cd(cd); r_free_cd(cdr);
    }
}

// ================================================================== TEST 7
// createCDict / createDDict with invalid compression level and NULL dict.
// prefixSize > 0 with prefix == NULL for refPrefix and refPrefix_advanced.

#[test]
fn create_bad_level_and_null_prefix() {
    unsafe {
        let e = Err2::new();
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");
        let (c_rp, r_rp) = both::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let (c_rpa, r_rpa) = both::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
        let (c_drp, r_drp) = both::<FnRefPrefix>("ZSTD_DCtx_refPrefix");
        let (c_drpa, r_drpa) = both::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");

        let dict = trained_dict(4096, 0xBEEF_0007);
        let dptr = dict.as_ptr() as *const c_void;
        let null = std::ptr::null::<c_void>();

        // createCDict with a variety of levels (incl. invalid) + NULL dict
        for lvl in [i32::MIN, -131073, -131072, -1000, 0, 23, 100, 1000, i32::MAX] {
            let a = c_ccd(dptr, dict.len(), lvl);
            let b = r_ccd(dptr, dict.len(), lvl);
            assert_eq!(a.is_null(), b.is_null(), "createCDict lvl={lvl} null parity");
            c_free_cd(a); r_free_cd(b);
            // NULL dict, nonzero size
            let a = c_ccd(null, 100, lvl);
            let b = r_ccd(null, 100, lvl);
            assert_eq!(a.is_null(), b.is_null(), "createCDict NULL lvl={lvl} null parity");
            c_free_cd(a); r_free_cd(b);
        }
        // createDDict NULL dict
        let a = c_cdd(null, 100);
        let b = r_cdd(null, 100);
        assert_eq!(a.is_null(), b.is_null(), "createDDict NULL null parity");
        c_free_dd(a); r_free_dd(b);

        // prefixSize > 0 with prefix == NULL
        for &sz in &[1usize, 8, 100, 1024] {
            let cx = CctxPair::new();
            let dx = DctxPair::new();
            e.eq(&format!("CCtx_refPrefix NULL sz={sz}"), c_rp(cx.c, null, sz), r_rp(cx.r, null, sz));
            e.eq(&format!("CCtx_refPrefix_advanced NULL sz={sz}"),
                 c_rpa(cx.c, null, sz, ZSTD_dct_rawContent), r_rpa(cx.r, null, sz, ZSTD_dct_rawContent));
            e.eq(&format!("DCtx_refPrefix NULL sz={sz}"), c_drp(dx.c, null, sz), r_drp(dx.r, null, sz));
            e.eq(&format!("DCtx_refPrefix_advanced NULL sz={sz}"),
                 c_drpa(dx.c, null, sz, ZSTD_dct_rawContent), r_drpa(dx.r, null, sz, ZSTD_dct_rawContent));
        }
    }
}

// ================================================================== TEST 8
// loadDictionary called mid-frame (after compressStream2 has started) ->
// stage_wrong. Same for refCDict / refPrefix mid-frame.

#[test]
fn load_dictionary_mid_frame() {
    unsafe {
        let e = Err2::new();
        let (c_ld, r_ld) = both::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (c_rc, r_rc) = both::<FnRefCDict>("ZSTD_CCtx_refCDict");
        let (c_rp, r_rp) = both::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_s2, r_s2) = both::<FnStream2>("ZSTD_compressStream2");
        let (crd, rrd) = both::<FnResetD>("ZSTD_CCtx_reset");
        let (c_sp, r_sp) = both::<FnSetParam>("ZSTD_CCtx_setParameter");

        let mut rng = Rng::new(0xE550_0008);
        let dict = trained_dict(8192, 0xBEEF_0008);
        let dptr = dict.as_ptr() as *const c_void;
        let cd = c_ccd(dptr, dict.len(), 5);
        let cdr = r_ccd(dptr, dict.len(), 5);

        let cx = CctxPair::new();
        crd(cx.c, ZSTD_reset_session_and_parameters);
        rrd(cx.r, ZSTD_reset_session_and_parameters);
        c_sp(cx.c, ZSTD_c_compressionLevel, 5);
        r_sp(cx.r, ZSTD_c_compressionLevel, 5);

        // start a frame (warm-up), consuming some input but not ending it
        let src = gen(Shape::Text, 60000, &mut rng);
        let mut o1 = vec![0u8; 4096];
        let mut o2 = vec![0u8; 4096];
        let mut ib1 = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
        let mut ib2 = ib1;
        let mut ob1 = ZSTD_outBuffer { dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0 };
        let mut ob2 = ZSTD_outBuffer { dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0 };
        e.eq("warm-up", c_s2(cx.c, &mut ob1, &mut ib1, ZSTD_e_continue), r_s2(cx.r, &mut ob2, &mut ib2, ZSTD_e_continue));

        // now these must fail with stage_wrong on both
        let a = c_ld(cx.c, dptr, dict.len());
        let b = r_ld(cx.r, dptr, dict.len());
        e.eq("loadDictionary mid-frame", a, b);
        assert_eq!(e.c.classify(a), Ret::Err { code: E_stage_wrong, name: err_name(&e, a, true) },
                   "loadDictionary mid-frame must be stage_wrong");

        let a = c_rc(cx.c, cd);
        let b = r_rc(cx.r, cdr);
        e.eq("refCDict mid-frame", a, b);
        assert_eq!(e.c.classify(a), Ret::Err { code: E_stage_wrong, name: err_name(&e, a, true) },
                   "refCDict mid-frame must be stage_wrong");

        let a = c_rp(cx.c, dptr, dict.len());
        let b = r_rp(cx.r, dptr, dict.len());
        e.eq("refPrefix mid-frame", a, b);
        assert_eq!(e.c.classify(a), Ret::Err { code: E_stage_wrong, name: err_name(&e, a, true) },
                   "refPrefix mid-frame must be stage_wrong");

        c_free_cd(cd); r_free_cd(cdr);
    }
}

// ================================================================== TEST 9
// ZSTD_getDictID_fromDict / _fromFrame / _fromCDict / _fromDDict on buffers that
// are too small, wrong magic, random garbage, or truncated frames. Thousands of
// random cases, fixed seed.

#[test]
fn getdictid_fuzz() {
    unsafe {
        let e = Err2::new();
        let (c_idb, r_idb) = both::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
        let (c_idf, r_idf) = both::<FnGetDictIDFromFrame>("ZSTD_getDictID_fromFrame");
        let (c_ccd, r_ccd) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (c_cdd, r_cdd) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (c_idc, r_idc) = both::<FnGetDictIDFromCDict>("ZSTD_getDictID_fromCDict");
        let (c_idd, r_idd) = both::<FnGetDictIDFromDDict>("ZSTD_getDictID_fromDDict");
        let (c_free_cd, r_free_cd) = both::<FnFreeCtx>("ZSTD_freeCDict");
        let (c_free_dd, r_free_dd) = both::<FnFreeCtx>("ZSTD_freeDDict");

        let mut rng = Rng::new(0xE550_0009);

        // fromCDict/fromDDict on NULL must agree
        assert_eq!(c_idc(std::ptr::null()), r_idc(std::ptr::null()), "getDictID_fromCDict(NULL)");
        assert_eq!(c_idd(std::ptr::null()), r_idd(std::ptr::null()), "getDictID_fromDDict(NULL)");

        for i in 0..4000 {
            let n = rng.below(40);
            let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            // sometimes plant a dictionary magic prefix
            match i % 5 {
                0 if n >= 4 => buf[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes()),
                1 if n >= 4 => buf[..4].copy_from_slice(&0xFD2FB528u32.to_le_bytes()), // frame magic
                2 if n >= 8 => {
                    buf[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
                    let id = rng.next_u32();
                    buf[4..8].copy_from_slice(&id.to_le_bytes());
                }
                _ => {}
            }
            let p = buf.as_ptr() as *const c_void;
            assert_eq!(c_idb(p, buf.len()), r_idb(p, buf.len()),
                       "getDictID_fromDict fuzz #{i} n={n}");
            assert_eq!(c_idf(p, buf.len()), r_idf(p, buf.len()),
                       "getDictID_fromFrame fuzz #{i} n={n}");
        }

        // real frames truncated at every offset, plus dictID checks
        let (c_cuc, r_cuc) = both::<unsafe extern "C" fn(*mut c_void,*mut c_void,size_t,*const c_void,size_t,*const c_void)->size_t>("ZSTD_compress_usingCDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dict = trained_dict(8192, 0xBEEF_0009);
        let cd = c_ccd(dict.as_ptr() as *const c_void, dict.len(), 5);
        let cdr = r_ccd(dict.as_ptr() as *const c_void, dict.len(), 5);
        let dd = c_cdd(dict.as_ptr() as *const c_void, dict.len());
        let ddr = r_cdd(dict.as_ptr() as *const c_void, dict.len());
        assert_eq!(c_idc(cd), r_idc(cdr), "getDictID_fromCDict real");
        assert_eq!(c_idd(dd), r_idd(ddr), "getDictID_fromDDict real");

        let src = gen(Shape::Text, 20000, &mut rng);
        let cap = c_cb(src.len()) + 64;
        let mut frame = vec![0u8; cap];
        let cx = CctxPair::new();
        let n = c_cuc(cx.c, frame.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), cd);
        frame.truncate(n);
        for cut in 0..=frame.len().min(64) {
            let p = frame.as_ptr() as *const c_void;
            assert_eq!(c_idf(p, cut), r_idf(p, cut), "getDictID_fromFrame truncated cut={cut}");
        }
        // a few larger cuts
        for &cut in &[frame.len()/2, frame.len().saturating_sub(1), frame.len()] {
            let p = frame.as_ptr() as *const c_void;
            assert_eq!(c_idf(p, cut), r_idf(p, cut), "getDictID_fromFrame big cut={cut}");
        }
        let _ = e;
        c_free_cd(cd); r_free_cd(cdr);
        c_free_dd(dd); r_free_dd(ddr);
    }
}

// ================================================================== TEST 10
// compress_usingDict with invalid level (compression must clamp/reject
// identically) and NULL dict at various sizes.

#[test]
fn compress_using_dict_bad_level_and_null() {
    unsafe {
        let e = Err2::new();
        let (c_cud, r_cud) = both::<FnCompressUsingDict>("ZSTD_compress_usingDict");
        let (c_cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xE550_000A);
        let dict = trained_dict(4096, 0xBEEF_000A);
        let dptr = dict.as_ptr() as *const c_void;
        let null = std::ptr::null::<c_void>();

        let src = gen(Shape::Text, 5000, &mut rng);
        let cap = c_cb(src.len()) + 64;
        for lvl in [i32::MIN, -131073, 0, 23, 1000, i32::MAX] {
            for (dp, ds, tag) in [(dptr, dict.len(), "dict"), (null, 100usize, "null100"), (null, 0usize, "null0")] {
                let cx = CctxPair::new();
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let a = c_cud(cx.c, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), dp, ds, lvl);
                let b = r_cud(cx.r, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), dp, ds, lvl);
                let ctx = format!("compress_usingDict lvl={lvl} {tag}");
                e.eq(&ctx, a, b);
                if !e.c.is_err(a) {
                    assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
                }
            }
        }
    }
}
