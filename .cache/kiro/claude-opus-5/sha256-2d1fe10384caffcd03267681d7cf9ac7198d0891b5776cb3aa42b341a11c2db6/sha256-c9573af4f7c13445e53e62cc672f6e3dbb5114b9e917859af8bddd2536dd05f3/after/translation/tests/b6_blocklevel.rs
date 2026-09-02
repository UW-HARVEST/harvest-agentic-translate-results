//! Phase B6: differential tests for the low-level "buffer-less" streaming
//! API and the raw block-level API — VALID paths.
//!
//! Every call crosses the FFI boundary through `both::<T>(name)`; the C and
//! Rust libraries are driven in lock-step and their emitted bytes / return
//! values are asserted byte-identical after every step.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------- FFI typedefs

type FnBeginLevel = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnBeginDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnBeginAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    size_t,
    ZSTD_parameters,
    c_ulonglong,
) -> size_t;
type FnBeginUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnBeginUsingCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, ZSTD_frameParameters, c_ulonglong) -> size_t;
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> size_t;
type FnCctxToSize = unsafe extern "C" fn(*const c_void) -> size_t;
type FnBlock =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

type FnDecBegin = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnDecBeginDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnNextSrcSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnNextInputType = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;

type FnVoidPtr = unsafe extern "C" fn() -> *mut c_void;
type FnPtrSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters;
type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnTrain =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, *const size_t, c_uint) -> size_t;

const ZSTD_BLOCKSIZE_MAX: usize = 1 << 17; // 131072

// ---------------------------------------------------------------- CCtx/DCtx pairs

/// A pair of C and Rust CCtx pointers, freed on drop.
struct CCtxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl CCtxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnVoidPtr>("ZSTD_createCCtx");
            let (x, y) = (a(), b());
            assert!(!x.is_null() && !y.is_null());
            CCtxPair { c: x, r: y }
        }
    }
}
impl Drop for CCtxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrSize>("ZSTD_freeCCtx");
            a(self.c);
            b(self.r);
        }
    }
}

struct DCtxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl DCtxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnVoidPtr>("ZSTD_createDCtx");
            let (x, y) = (a(), b());
            assert!(!x.is_null() && !y.is_null());
            DCtxPair { c: x, r: y }
        }
    }
}
impl Drop for DCtxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrSize>("ZSTD_freeDCtx");
            a(self.c);
            b(self.r);
        }
    }
}

// ---------------------------------------------------------------- shared helpers

/// Train a real dictionary once with the C library and return the identical
/// bytes for both libraries to reuse. Falls back to `None` if training fails
/// (e.g. not enough sample data), so callers can skip the trained-dict variant.
fn trained_dict() -> Option<Vec<u8>> {
    unsafe {
        let (ctrain, _) = both::<FnTrain>("ZDICT_trainFromBuffer");
        let e = Err2::new();
        let mut rng = Rng::new(0xD1C7_0001);
        // Build many small correlated samples so training has signal.
        let mut samples: Vec<u8> = Vec::new();
        let mut sizes: Vec<size_t> = Vec::new();
        for _ in 0..2048 {
            let shape = [Shape::Text, Shape::Repeating, Shape::LongMatches, Shape::LowEntropy]
                [rng.below(4)];
            let len = 32 + rng.below(256);
            let s = gen(shape, len, &mut rng);
            sizes.push(s.len());
            samples.extend_from_slice(&s);
        }
        let mut dict = vec![0u8; 16 * 1024];
        let n = ctrain(
            dict.as_mut_ptr() as *mut c_void,
            dict.len(),
            samples.as_ptr() as *const c_void,
            sizes.as_ptr(),
            sizes.len() as c_uint,
        );
        if e.c.is_err(n) {
            return None;
        }
        dict.truncate(n);
        Some(dict)
    }
}

/// A raw (untrained) dictionary — arbitrary bytes usable via the raw-content
/// dictionary path.
fn raw_dict(rng: &mut Rng) -> Vec<u8> {
    gen(Shape::Text, 4096, rng)
}

/// Full buffer-less roundtrip check: drive compressBegin(applied via `begin`)
/// + compressContinue(chunks) + compressEnd in lock-step across C and Rust,
/// asserting byte-identical output after EVERY step, then verify the frame
/// decompresses on both libraries.
///
/// `begin` receives (cctx_c, cctx_r) and must initialise both, returning the
/// per-library begin return codes for comparison.
fn drive_bufferless<F>(
    ctx_label: &str,
    src: &[u8],
    chunk: usize,
    e: &Err2,
    cont: &(libloading::Symbol<'static, FnContinue>, libloading::Symbol<'static, FnContinue>),
    end: &(libloading::Symbol<'static, FnContinue>, libloading::Symbol<'static, FnContinue>),
    cbound: &libloading::Symbol<'static, FnCompressBound>,
    dec: &(libloading::Symbol<'static, FnDecompress>, libloading::Symbol<'static, FnDecompress>),
    cp: &CCtxPair,
    mut begin: F,
) where
    F: FnMut(&CCtxPair) -> (size_t, size_t),
{
    unsafe {
        let (br_c, br_r) = begin(cp);
        e.eq(&format!("{ctx_label}: begin"), br_c, br_r);
        if e.c.is_err(br_c) {
            return;
        }

        let chunk = chunk.max(1);
        let cap = (*cbound)(src.len()) + 1024;
        let mut out_c = vec![0u8; cap];
        let mut out_r = vec![0u8; cap];
        let mut off_c = 0usize;
        let mut off_r = 0usize;

        let mut pos = 0usize;
        while pos < src.len() {
            let n = chunk.min(src.len() - pos);
            let sc = &src[pos..pos + n];
            let rc_c = (cont.0)(
                cp.c,
                out_c.as_mut_ptr().add(off_c) as *mut c_void,
                cap - off_c,
                sc.as_ptr() as *const c_void,
                n,
            );
            let rc_r = (cont.1)(
                cp.r,
                out_r.as_mut_ptr().add(off_r) as *mut c_void,
                cap - off_r,
                sc.as_ptr() as *const c_void,
                n,
            );
            e.eq(&format!("{ctx_label}: continue@{pos}"), rc_c, rc_r);
            if e.c.is_err(rc_c) {
                return;
            }
            off_c += rc_c;
            off_r += rc_r;
            // Byte-identical after EVERY step.
            assert_bytes_eq(
                &format!("{ctx_label}: bytes after continue@{pos}"),
                &out_c[..off_c],
                &out_r[..off_r],
            );
            pos += n;
        }

        // compressEnd flushes the last block(s) + optional checksum. srcSize 0.
        let rc_c = (end.0)(
            cp.c,
            out_c.as_mut_ptr().add(off_c) as *mut c_void,
            cap - off_c,
            std::ptr::null(),
            0,
        );
        let rc_r = (end.1)(
            cp.r,
            out_r.as_mut_ptr().add(off_r) as *mut c_void,
            cap - off_r,
            std::ptr::null(),
            0,
        );
        e.eq(&format!("{ctx_label}: end"), rc_c, rc_r);
        if e.c.is_err(rc_c) {
            return;
        }
        off_c += rc_c;
        off_r += rc_r;
        assert_bytes_eq(
            &format!("{ctx_label}: final frame"),
            &out_c[..off_c],
            &out_r[..off_r],
        );

        // Verify the frame decompresses on BOTH libraries to the original.
        let frame = &out_c[..off_c];
        let mut d_c = vec![0u8; src.len() + 16];
        let mut d_r = vec![0u8; src.len() + 16];
        let dc = (dec.0)(
            d_c.as_mut_ptr() as *mut c_void,
            d_c.len(),
            frame.as_ptr() as *const c_void,
            frame.len(),
        );
        let dr = (dec.1)(
            d_r.as_mut_ptr() as *mut c_void,
            d_r.len(),
            frame.as_ptr() as *const c_void,
            frame.len(),
        );
        e.eq(&format!("{ctx_label}: decompress"), dc, dr);
        if e.c.is_err(dc) {
            return;
        }
        assert_eq!(dc, src.len(), "{ctx_label}: roundtrip size");
        assert_bytes_eq(&format!("{ctx_label}: decoded C"), &d_c[..dc], src);
        assert_bytes_eq(&format!("{ctx_label}: decoded RS"), &d_r[..dr], src);
    }
}

/// Full buffer-less decompression: decompressBegin (via `begin`) then the
/// nextSrcSizeToDecompress / decompressContinue loop, sampling nextInputType
/// each iteration and asserting C and Rust agree on everything.
fn drive_bufferless_decode<F>(
    ctx_label: &str,
    frame: &[u8],
    expected: &[u8],
    e: &Err2,
    nsstd: &(libloading::Symbol<'static, FnNextSrcSize>, libloading::Symbol<'static, FnNextSrcSize>),
    dcont: &(libloading::Symbol<'static, FnDecContinue>, libloading::Symbol<'static, FnDecContinue>),
    nit: &(libloading::Symbol<'static, FnNextInputType>, libloading::Symbol<'static, FnNextInputType>),
    dp: &DCtxPair,
    mut begin: F,
) where
    F: FnMut(&DCtxPair) -> (size_t, size_t),
{
    unsafe {
        let (rc, rr) = begin(dp);
        e.eq(&format!("{ctx_label}: decompressBegin"), rc, rr);
        if e.c.is_err(rc) {
            return;
        }

        let mut out_c = vec![0u8; expected.len() + 16];
        let mut out_r = vec![0u8; expected.len() + 16];
        let mut oc = 0usize;
        let mut or = 0usize;
        let mut ic = 0usize; // input consumed
        let mut iter = 0usize;
        loop {
            let nc = (nsstd.0)(dp.c);
            let nr = (nsstd.1)(dp.r);
            assert_eq!(
                nc, nr,
                "{ctx_label}: nextSrcSizeToDecompress mismatch @iter{iter} (C={nc} RS={nr})"
            );
            // nextInputType must agree every iteration.
            let tc = (nit.0)(dp.c);
            let tr = (nit.1)(dp.r);
            assert_eq!(tc, tr, "{ctx_label}: nextInputType mismatch @iter{iter}");
            if nc == 0 {
                break;
            }
            assert!(ic + nc <= frame.len(), "{ctx_label}: frame underflow @iter{iter}");
            let inp = &frame[ic..ic + nc];
            let rc = (dcont.0)(
                dp.c,
                out_c.as_mut_ptr().add(oc) as *mut c_void,
                out_c.len() - oc,
                inp.as_ptr() as *const c_void,
                nc,
            );
            let rr = (dcont.1)(
                dp.r,
                out_r.as_mut_ptr().add(or) as *mut c_void,
                out_r.len() - or,
                inp.as_ptr() as *const c_void,
                nc,
            );
            e.eq(&format!("{ctx_label}: decompressContinue @iter{iter}"), rc, rr);
            if e.c.is_err(rc) {
                return;
            }
            oc += rc;
            or += rr;
            ic += nc;
            iter += 1;
            assert!(iter < 10_000_000, "{ctx_label}: runaway loop");
        }
        assert_eq!(oc, or, "{ctx_label}: output size mismatch");
        assert_bytes_eq(&format!("{ctx_label}: decoded C vs expected"), &out_c[..oc], expected);
        assert_bytes_eq(&format!("{ctx_label}: decoded RS"), &out_r[..or], &out_c[..oc]);
    }
}

// A compact list of shapes/lengths kept small enough to stay well under the
// 300s per-test budget while still covering every shape and the required
// boundary lengths.
const SHAPE_SET: &[Shape] = ALL_SHAPES;
const LEN_SET: &[usize] = &[0, 1, 100, 1024, 20000, 131072, 131073, 200000];

// -------------------------------------------------------------------- tests

/// compressBegin(level) + compressContinue(chunks) + compressEnd, over every
/// level, chunk size, shape, and length. Byte-identical after every step and
/// verified decompressible. Split across two tests to bound runtime.
fn run_begin_level(levels: &[c_int], seed: u64) {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let cont = both::<FnContinue>("ZSTD_compressContinue");
        let end = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dec = both::<FnDecompress>("ZSTD_decompress");
        let (gbs_c, _) = both::<FnCctxToSize>("ZSTD_getBlockSize");
        let mut rng = Rng::new(seed);

        for &shape in SHAPE_SET {
            for &len in LEN_SET {
                let src = gen(shape, len, &mut rng);
                for &lvl in levels {
                    let cp = CCtxPair::new();
                    // block size depends on the level's window; query on C ctx
                    // AFTER a begin, so compute it once via a scratch begin.
                    begin_c(cp.c, lvl);
                    let bs = gbs_c(cp.c);
                    // fresh cctx for the actual drive
                    let cp2 = CCtxPair::new();
                    let chunks: [usize; 9] =
                        [1, 2, 7, 100, 1024, 65535, 65536, bs.max(1), 131072];
                    // pick a couple of chunk sizes per (shape,len,lvl) at random
                    // plus always exercise 1 and the block size, to bound cost.
                    let picks = [1usize, chunks[rng.below(chunks.len())], bs.max(1)];
                    for &chunk in &picks {
                        let cp3 = CCtxPair::new();
                        drive_bufferless(
                            &format!("begin lvl={lvl} shape={shape:?} len={len} chunk={chunk}"),
                            &src,
                            chunk,
                            &e,
                            &cont,
                            &end,
                            &cbound,
                            &dec,
                            &cp3,
                            |cp| (begin_c(cp.c, lvl), begin_r(cp.r, lvl)),
                        );
                    }
                    let _ = &cp;
                    let _ = &cp2;
                }
            }
        }
    }
}

#[test]
fn begin_level_low() {
    run_begin_level(&[-5, -1, 1], 0xB6_0001);
}

#[test]
fn begin_level_mid() {
    run_begin_level(&[3, 9], 0xB6_0002);
}

#[test]
fn begin_level_high() {
    run_begin_level(&[19, 22], 0xB6_0003);
}

/// Exhaustively exercise every required chunk size at a single level/shape mix
/// (the chunk-size dimension the per-config tests sample randomly).
#[test]
fn begin_all_chunk_sizes() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let cont = both::<FnContinue>("ZSTD_compressContinue");
        let end = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dec = both::<FnDecompress>("ZSTD_decompress");
        let (gbs_c, _) = both::<FnCctxToSize>("ZSTD_getBlockSize");
        let mut rng = Rng::new(0xB6_0004);
        let lvl = 3;
        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating, Shape::Sequential] {
            for &len in &[0usize, 1, 100, 1024, 65535, 65536, 131072, 200000] {
                let src = gen(shape, len, &mut rng);
                let scratch = CCtxPair::new();
                begin_c(scratch.c, lvl);
                let bs = gbs_c(scratch.c);
                let chunks: [usize; 9] =
                    [1, 2, 7, 100, 1024, 65535, 65536, bs.max(1), 131072];
                for &chunk in &chunks {
                    let cp = CCtxPair::new();
                    drive_bufferless(
                        &format!("chunk sweep chunk={chunk} shape={shape:?} len={len}"),
                        &src,
                        chunk,
                        &e,
                        &cont,
                        &end,
                        &cbound,
                        &dec,
                        &cp,
                        |cp| (begin_c(cp.c, lvl), begin_r(cp.r, lvl)),
                    );
                }
            }
        }
    }
}

/// compressBegin_usingDict / _advanced with raw and trained dictionaries.
#[test]
fn begin_using_dict_and_advanced() {
    unsafe {
        let e = Err2::new();
        let (bd_c, bd_r) = both::<FnBeginDict>("ZSTD_compressBegin_usingDict");
        let (ba_c, ba_r) = both::<FnBeginAdvanced>("ZSTD_compressBegin_advanced");
        let cont = both::<FnContinue>("ZSTD_compressContinue");
        let end = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dec = both::<FnDecompress>("ZSTD_decompress");
        let (gp_c, _) = both::<FnGetParams>("ZSTD_getParams");
        let mut rng = Rng::new(0xB6_0010);

        let raw = raw_dict(&mut rng);
        let trained = trained_dict();
        assert!(trained.is_some(), "dictionary training should succeed for coverage");

        let mut dicts: Vec<(&str, &[u8])> = vec![("raw", &raw[..])];
        if let Some(ref t) = trained {
            dicts.push(("trained", &t[..]));
        }

        for (dname, dict) in &dicts {
            for &shape in &[Shape::Text, Shape::Repeating, Shape::LongMatches, Shape::Random] {
                for &len in &[0usize, 1, 100, 1024, 20000, 131073] {
                    let src = gen(shape, len, &mut rng);
                    for &lvl in &[-1i32, 1, 3, 9, 19] {
                        // _usingDict
                        let cp = CCtxPair::new();
                        drive_bufferless(
                            &format!("usingDict[{dname}] lvl={lvl} shape={shape:?} len={len}"),
                            &src,
                            1024,
                            &e,
                            &cont,
                            &end,
                            &cbound,
                            &dec,
                            &cp,
                            |cp| {
                                (
                                    bd_c(cp.c, dict.as_ptr() as *const c_void, dict.len(), lvl),
                                    bd_r(cp.r, dict.as_ptr() as *const c_void, dict.len(), lvl),
                                )
                            },
                        );

                        // _advanced: params from getParams, both pledged known
                        // and CONTENTSIZE_UNKNOWN.
                        for &pledged in
                            &[src.len() as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN]
                        {
                            let params = gp_c(lvl, src.len() as c_ulonglong, dict.len());
                            let cp = CCtxPair::new();
                            drive_bufferless(
                                &format!(
                                    "advanced[{dname}] lvl={lvl} shape={shape:?} len={len} pledged={pledged}"
                                ),
                                &src,
                                1024,
                                &e,
                                &cont,
                                &end,
                                &cbound,
                                &dec,
                                &cp,
                                |cp| {
                                    (
                                        ba_c(
                                            cp.c,
                                            dict.as_ptr() as *const c_void,
                                            dict.len(),
                                            params,
                                            pledged,
                                        ),
                                        ba_r(
                                            cp.r,
                                            dict.as_ptr() as *const c_void,
                                            dict.len(),
                                            params,
                                            pledged,
                                        ),
                                    )
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

/// compressBegin_usingCDict / _usingCDict_advanced. The CDict is built per
/// library from identical bytes (never shared across libraries).
#[test]
fn begin_using_cdict() {
    unsafe {
        let e = Err2::new();
        let (bc_c, bc_r) = both::<FnBeginUsingCDict>("ZSTD_compressBegin_usingCDict");
        let (bca_c, bca_r) =
            both::<FnBeginUsingCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
        let (cd_c, cd_r) = both::<FnCreateCDict>("ZSTD_createCDict");
        let (fcd_c, fcd_r) = both::<FnFreeDict>("ZSTD_freeCDict");
        let cont = both::<FnContinue>("ZSTD_compressContinue");
        let end = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dec = both::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xB6_0020);

        let raw = raw_dict(&mut rng);
        let trained = trained_dict();
        let mut dicts: Vec<(&str, &[u8])> = vec![("raw", &raw[..])];
        if let Some(ref t) = trained {
            dicts.push(("trained", &t[..]));
        }

        for (dname, dict) in &dicts {
            for &lvl in &[1i32, 3, 9, 19] {
                // Build one CDict per library from identical bytes.
                let cdc = cd_c(dict.as_ptr() as *const c_void, dict.len(), lvl);
                let cdr = cd_r(dict.as_ptr() as *const c_void, dict.len(), lvl);
                assert!(!cdc.is_null() && !cdr.is_null(), "createCDict null");

                for &shape in &[Shape::Text, Shape::Repeating, Shape::Random] {
                    for &len in &[0usize, 1, 1024, 20000, 131073] {
                        let src = gen(shape, len, &mut rng);
                        // _usingCDict
                        let cp = CCtxPair::new();
                        drive_bufferless(
                            &format!("usingCDict[{dname}] lvl={lvl} shape={shape:?} len={len}"),
                            &src,
                            1024,
                            &e,
                            &cont,
                            &end,
                            &cbound,
                            &dec,
                            &cp,
                            |cp| (bc_c(cp.c, cdc), bc_r(cp.r, cdr)),
                        );
                        // _usingCDict_advanced with fParams and pledged variants
                        for &pledged in
                            &[src.len() as c_ulonglong, ZSTD_CONTENTSIZE_UNKNOWN]
                        {
                            for cs in [0i32, 1] {
                                for ck in [0i32, 1] {
                                    let fp = ZSTD_frameParameters {
                                        contentSizeFlag: cs,
                                        checksumFlag: ck,
                                        noDictIDFlag: 0,
                                    };
                                    // _advanced requires correct pledged for known-size.
                                    let cp = CCtxPair::new();
                                    drive_bufferless(
                                        &format!(
                                            "usingCDictAdv[{dname}] lvl={lvl} shape={shape:?} len={len} pledged={pledged} cs={cs} ck={ck}"
                                        ),
                                        &src,
                                        1024,
                                        &e,
                                        &cont,
                                        &end,
                                        &cbound,
                                        &dec,
                                        &cp,
                                        |cp| {
                                            (
                                                bca_c(cp.c, cdc, fp, pledged),
                                                bca_r(cp.r, cdr, fp, pledged),
                                            )
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                fcd_c(cdc);
                fcd_r(cdr);
            }
        }
    }
}

/// ZSTD_copyCCtx: after compressBegin, copy the context and continue on the
/// COPY. The copy's output must be byte-identical between C and Rust and
/// identical to the non-copied path.
#[test]
fn copy_cctx() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (copy_c, copy_r) = both::<FnCopyCCtx>("ZSTD_copyCCtx");
        let cont = both::<FnContinue>("ZSTD_compressContinue");
        let end = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let dec = both::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xB6_0030);

        for &shape in &[Shape::Text, Shape::Repeating, Shape::Random, Shape::Zeros] {
            for &len in &[0usize, 1, 100, 1024, 20000, 131073] {
                let src = gen(shape, len, &mut rng);
                for &lvl in &[-1i32, 1, 3, 9, 19] {
                    // Reference: non-copied path bytes.
                    let cap = cbound(src.len()) + 1024;
                    let mut ref_out = vec![0u8; cap];
                    let cpref = CCtxPair::new();
                    begin_c(cpref.c, lvl);
                    let mut off = 0usize;
                    let rc = cont.0(
                        cpref.c,
                        ref_out.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    assert!(!e.c.is_err(rc), "reference continue");
                    off += rc;
                    let rc = end.0(
                        cpref.c,
                        ref_out.as_mut_ptr().add(off) as *mut c_void,
                        cap - off,
                        std::ptr::null(),
                        0,
                    );
                    assert!(!e.c.is_err(rc), "reference end");
                    off += rc;
                    let reference = ref_out[..off].to_vec();

                    // Copy path: begin prepared cctx, copy into fresh cctx, drive on copy.
                    let prepared = CCtxPair::new();
                    e.eq(
                        &format!("copyCCtx begin lvl={lvl}"),
                        begin_c(prepared.c, lvl),
                        begin_r(prepared.r, lvl),
                    );
                    let dest = CCtxPair::new();
                    // compressBegin (the reference path) leaves pledgedSrcSize
                    // unknown, so the copy must use UNKNOWN to produce a
                    // byte-identical frame header.
                    let pledged = ZSTD_CONTENTSIZE_UNKNOWN;
                    e.eq(
                        &format!("copyCCtx copy lvl={lvl} len={len}"),
                        copy_c(dest.c, prepared.c, pledged),
                        copy_r(dest.r, prepared.r, pledged),
                    );

                    // Drive on the copy (single continue+end so bytes must match
                    // the reference which used the same pledged/single chunk).
                    drive_bufferless(
                        &format!("copyCCtx drive lvl={lvl} shape={shape:?} len={len}"),
                        &src,
                        src.len().max(1),
                        &e,
                        &cont,
                        &end,
                        &cbound,
                        &dec,
                        &dest,
                        |_cp| (0, 0), // already begun via copy
                    );

                    // Re-run explicitly to compare against the reference bytes.
                    let dest2 = CCtxPair::new();
                    copy_c(dest2.c, prepared.c, pledged);
                    copy_r(dest2.r, prepared.r, pledged);
                    let mut oc = vec![0u8; cap];
                    let mut or = vec![0u8; cap];
                    let mut offc = 0usize;
                    let mut offr = 0usize;
                    let rc = cont.0(dest2.c, oc.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, src.len());
                    let rr = cont.1(dest2.r, or.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, src.len());
                    e.eq("copyCCtx continue on copy", rc, rr);
                    offc += rc; offr += rr;
                    let rc = end.0(dest2.c, oc.as_mut_ptr().add(offc) as *mut c_void, cap - offc,
                        std::ptr::null(), 0);
                    let rr = end.1(dest2.r, or.as_mut_ptr().add(offr) as *mut c_void, cap - offr,
                        std::ptr::null(), 0);
                    e.eq("copyCCtx end on copy", rc, rr);
                    offc += rc; offr += rr;
                    assert_bytes_eq("copyCCtx C vs RS", &oc[..offc], &or[..offr]);
                    assert_bytes_eq("copyCCtx C vs non-copied reference", &oc[..offc], &reference);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- decompression

/// Build a frame with specific frame parameters via the advanced streaming API
/// (ZSTD_compress2 after setting parameters) using the C library, for feeding
/// into the buffer-less decode loop.
unsafe fn make_frame(
    src: &[u8],
    checksum: c_int,
    content_size: c_int,
    window_log: c_int,
) -> Vec<u8> {
    let (create_c, _) = both::<FnVoidPtr>("ZSTD_createCCtx");
    let (free_c, _) = both::<FnPtrSize>("ZSTD_freeCCtx");
    let (setp_c, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c2_c, _) = both::<FnCompress2>("ZSTD_compress2");
    let (cbound_c, _) = both::<FnCompressBound>("ZSTD_compressBound");
    let (reset_c, _) = both::<FnReset>("ZSTD_CCtx_reset");
    let cctx = create_c();
    reset_c(cctx, ZSTD_reset_session_and_parameters);
    setp_c(cctx, ZSTD_c_compressionLevel, 3);
    setp_c(cctx, ZSTD_c_checksumFlag, checksum);
    setp_c(cctx, ZSTD_c_contentSizeFlag, content_size);
    if window_log > 0 {
        setp_c(cctx, ZSTD_c_windowLog, window_log);
    }
    let cap = cbound_c(src.len()) + 64;
    let mut out = vec![0u8; cap];
    let n = c2_c(
        cctx,
        out.as_mut_ptr() as *mut c_void,
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
    );
    free_c(cctx);
    assert!(!Err2::new().c.is_err(n), "make_frame compress2 failed");
    out.truncate(n);
    out
}

/// Full buffer-less decompression loop over frames with all combinations of
/// checksumFlag/contentSizeFlag/windowLog and shapes/lengths.
#[test]
fn decompress_bufferless_loop() {
    unsafe {
        let e = Err2::new();
        let nsstd = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let dcont = both::<FnDecContinue>("ZSTD_decompressContinue");
        let nit = both::<FnNextInputType>("ZSTD_nextInputType");
        let (db_c, db_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let mut rng = Rng::new(0xB6_0040);

        for &shape in SHAPE_SET {
            for &len in &[0usize, 1, 100, 1024, 20000, 131073] {
                let src = gen(shape, len, &mut rng);
                for &checksum in &[0i32, 1] {
                    for &csize in &[0i32, 1] {
                        for &wlog in &[0i32, 10, 17, 20] {
                            let frame = make_frame(&src, checksum, csize, wlog);
                            let dp = DCtxPair::new();
                            drive_bufferless_decode(
                                &format!(
                                    "decode shape={shape:?} len={len} ck={checksum} cs={csize} wl={wlog}"
                                ),
                                &frame,
                                &src,
                                &e,
                                &nsstd,
                                &dcont,
                                &nit,
                                &dp,
                                |dp| (db_c(dp.c), db_r(dp.r)),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// decompressBegin_usingDict / _usingDDict with raw and trained dictionaries.
#[test]
fn decompress_bufferless_with_dict() {
    unsafe {
        let e = Err2::new();
        let nsstd = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let dcont = both::<FnDecContinue>("ZSTD_decompressContinue");
        let nit = both::<FnNextInputType>("ZSTD_nextInputType");
        let (dbd_c, dbd_r) = both::<FnDecBeginDict>("ZSTD_decompressBegin_usingDict");
        let (dbdd_c, dbdd_r) = both::<FnDecBeginDDict>("ZSTD_decompressBegin_usingDDict");
        let (dd_c, dd_r) = both::<FnCreateDDict>("ZSTD_createDDict");
        let (fdd_c, fdd_r) = both::<FnFreeDict>("ZSTD_freeDDict");
        // Compression side helpers to build dict-compressed frames with C lib.
        let (create_c, _) = both::<FnVoidPtr>("ZSTD_createCCtx");
        let (free_c, _) = both::<FnPtrSize>("ZSTD_freeCCtx");
        let (cbd_c, _) = both::<FnBeginDict>("ZSTD_compressBegin_usingDict");
        let (cont_c, _) = both::<FnContinue>("ZSTD_compressContinue");
        let (end_c, _) = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound_c, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xB6_0050);

        let raw = raw_dict(&mut rng);
        let trained = trained_dict();
        let mut dicts: Vec<(&str, &[u8])> = vec![("raw", &raw[..])];
        if let Some(ref t) = trained {
            dicts.push(("trained", &t[..]));
        }

        for (dname, dict) in &dicts {
            for &shape in &[Shape::Text, Shape::Repeating, Shape::Random] {
                for &len in &[0usize, 1, 1024, 20000, 131073] {
                    let src = gen(shape, len, &mut rng);
                    // Build a dict-compressed frame with the C library.
                    let cctx = create_c();
                    cbd_c(cctx, dict.as_ptr() as *const c_void, dict.len(), 3);
                    let cap = cbound_c(src.len()) + 1024;
                    let mut frame = vec![0u8; cap];
                    let mut off = 0usize;
                    let rc = cont_c(cctx, frame.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, src.len());
                    assert!(!e.c.is_err(rc));
                    off += rc;
                    let rc = end_c(cctx, frame.as_mut_ptr().add(off) as *mut c_void, cap - off,
                        std::ptr::null(), 0);
                    assert!(!e.c.is_err(rc));
                    off += rc;
                    free_c(cctx);
                    frame.truncate(off);

                    // _usingDict
                    let dp = DCtxPair::new();
                    drive_bufferless_decode(
                        &format!("decode usingDict[{dname}] shape={shape:?} len={len}"),
                        &frame,
                        &src,
                        &e,
                        &nsstd,
                        &dcont,
                        &nit,
                        &dp,
                        |dp| {
                            (
                                dbd_c(dp.c, dict.as_ptr() as *const c_void, dict.len()),
                                dbd_r(dp.r, dict.as_ptr() as *const c_void, dict.len()),
                            )
                        },
                    );

                    // _usingDDict (build one DDict per library from same bytes)
                    let ddc = dd_c(dict.as_ptr() as *const c_void, dict.len());
                    let ddr = dd_r(dict.as_ptr() as *const c_void, dict.len());
                    assert!(!ddc.is_null() && !ddr.is_null());
                    let dp = DCtxPair::new();
                    drive_bufferless_decode(
                        &format!("decode usingDDict[{dname}] shape={shape:?} len={len}"),
                        &frame,
                        &src,
                        &e,
                        &nsstd,
                        &dcont,
                        &nit,
                        &dp,
                        |dp| (dbdd_c(dp.c, ddc), dbdd_r(dp.r, ddr)),
                    );
                    fdd_c(ddc);
                    fdd_r(ddr);
                }
            }
        }
    }
}

/// ZSTD_copyDCtx: copy after decompressBegin and continue the decode loop on
/// the copy.
#[test]
fn copy_dctx() {
    unsafe {
        let e = Err2::new();
        let nsstd = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let dcont = both::<FnDecContinue>("ZSTD_decompressContinue");
        let nit = both::<FnNextInputType>("ZSTD_nextInputType");
        let (db_c, db_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (copy_c, copy_r) = both::<FnCopyDCtx>("ZSTD_copyDCtx");
        let mut rng = Rng::new(0xB6_0060);

        for &shape in &[Shape::Text, Shape::Repeating, Shape::Random, Shape::Zeros] {
            for &len in &[0usize, 1, 1024, 20000, 131073] {
                let src = gen(shape, len, &mut rng);
                for &checksum in &[0i32, 1] {
                    let frame = make_frame(&src, checksum, 1, 0);
                    let dp = DCtxPair::new();
                    drive_bufferless_decode(
                        &format!("copyDCtx shape={shape:?} len={len} ck={checksum}"),
                        &frame,
                        &src,
                        &e,
                        &nsstd,
                        &dcont,
                        &nit,
                        &dp,
                        |dp| {
                            let rc = db_c(dp.c);
                            let rr = db_r(dp.r);
                            // Now copy the freshly-begun state into dp itself's
                            // pointers is not possible; instead copy into scratch
                            // and drive there. We drive on the ORIGINAL after
                            // copying FROM a prepared ctx.
                            (rc, rr)
                        },
                    );

                    // Explicit copy path: prepare, copy into dest, then decode
                    // loop on dest.
                    let prepared = DCtxPair::new();
                    db_c(prepared.c);
                    db_r(prepared.r);
                    let dest = DCtxPair::new();
                    copy_c(dest.c, prepared.c);
                    copy_r(dest.r, prepared.r);
                    drive_bufferless_decode(
                        &format!("copyDCtx-on-copy shape={shape:?} len={len} ck={checksum}"),
                        &frame,
                        &src,
                        &e,
                        &nsstd,
                        &dcont,
                        &nit,
                        &dest,
                        |_dp| (0, 0), // already prepared via copy
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------- raw blocks

/// Raw block API: getBlockSize, compressBlock + decompressBlock roundtrip and
/// insertBlock for uncompressed blocks.
#[test]
fn raw_block_roundtrip() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (gbs_c, gbs_r) = both::<FnCctxToSize>("ZSTD_getBlockSize");
        let (cblk_c, cblk_r) = both::<FnBlock>("ZSTD_compressBlock");
        let (dbegin_c, dbegin_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (dblk_c, dblk_r) = both::<FnBlock>("ZSTD_decompressBlock");
        let (insert_c, insert_r) = both::<FnInsertBlock>("ZSTD_insertBlock");
        let mut rng = Rng::new(0xB6_0070);

        let block_lens = [1usize, 2, 100, 1024, 65535, 65536, 131071, 131072];

        for &shape in SHAPE_SET {
            for &blen in &block_lens {
                let src = gen(shape, blen, &mut rng);
                let n = src.len(); // Shape::Empty may return empty
                for &lvl in &[1i32, 3, 9] {
                    let cp = CCtxPair::new();
                    e.eq(
                        &format!("block begin lvl={lvl}"),
                        begin_c(cp.c, lvl),
                        begin_r(cp.r, lvl),
                    );
                    let bs_c = gbs_c(cp.c);
                    let bs_r = gbs_r(cp.r);
                    assert_eq!(bs_c, bs_r, "getBlockSize mismatch lvl={lvl}");

                    if n == 0 || n > bs_c {
                        continue; // block must be <= block size and non-trivial
                    }

                    let cap = ZSTD_BLOCKSIZE_MAX + 1024;
                    let mut oc = vec![0u8; cap];
                    let mut or = vec![0u8; cap];
                    let rc = cblk_c(cp.c, oc.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, n);
                    let rr = cblk_r(cp.r, or.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, n);
                    let ctx = format!("compressBlock shape={shape:?} blen={blen} lvl={lvl}");
                    e.eq(&ctx, rc, rr);
                    if e.c.is_err(rc) {
                        continue;
                    }
                    // Byte-identical compressed block.
                    assert_bytes_eq(&ctx, &oc[..rc], &or[..rr]);

                    if rc == 0 {
                        // Incompressible: compressBlock produced nothing. The
                        // decoder must be told via insertBlock (raw block).
                        let dp = DCtxPair::new();
                        dbegin_c(dp.c);
                        dbegin_r(dp.r);
                        let ir = insert_c(dp.c, src.as_ptr() as *const c_void, n);
                        let irr = insert_r(dp.r, src.as_ptr() as *const c_void, n);
                        e.eq(&format!("insertBlock {ctx}"), ir, irr);
                        assert_eq!(ir, n, "insertBlock returns blockSize");
                        continue;
                    }

                    // decompressBlock roundtrip.
                    let dp = DCtxPair::new();
                    e.eq(
                        "decompressBegin for block",
                        dbegin_c(dp.c),
                        dbegin_r(dp.r),
                    );
                    let mut dc = vec![0u8; n + 16];
                    let mut dr = vec![0u8; n + 16];
                    let drc = dblk_c(dp.c, dc.as_mut_ptr() as *mut c_void, dc.len(),
                        oc.as_ptr() as *const c_void, rc);
                    let drr = dblk_r(dp.r, dr.as_mut_ptr() as *mut c_void, dr.len(),
                        or.as_ptr() as *const c_void, rr);
                    e.eq(&format!("decompressBlock {ctx}"), drc, drr);
                    if e.c.is_err(drc) {
                        continue;
                    }
                    assert_eq!(drc, n, "decompressBlock size {ctx}");
                    assert_bytes_eq(&format!("decoded block {ctx}"), &dc[..drc], &src);
                    assert_bytes_eq(&format!("decoded block RS {ctx}"), &dr[..drr], &dc[..drc]);
                }
            }
        }
    }
}

/// getBlockSize returns the same value for both libraries at every level.
#[test]
fn get_block_size_matches() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (gbs_c, gbs_r) = both::<FnCctxToSize>("ZSTD_getBlockSize");
        for lvl in [-5i32, -1, 1, 3, 9, 19, 22] {
            let cp = CCtxPair::new();
            e.eq("gbs begin", begin_c(cp.c, lvl), begin_r(cp.r, lvl));
            assert_eq!(gbs_c(cp.c), gbs_r(cp.r), "getBlockSize lvl={lvl}");
        }
    }
}
