//! Phase B: differential tests for the DEPRECATED ZBUFF streaming API —
//! VALID paths.
//!
//! The ZBUFF layer is a thin shim over the modern `ZSTD_*Stream` API (see
//! `c_src/src/deprecated/zbuff_*.c`). We drive full streaming compression and
//! decompression round-trips through the ZBUFF entry points of BOTH libraries
//! and assert:
//!   * every `ZBUFF_compress*` call returns byte-identical output at each step,
//!   * the `*dstCapacityPtr` / `*srcSizePtr` counters advance identically,
//!   * the hint / remaining-bytes return values are equivalent,
//!   * a ZBUFF-produced frame decodes to the original via the ZBUFF decoder,
//!   * the recommended-size helpers agree.
//!
//! Every call crosses the FFI boundary via `both::<T>(name)`. A context handle
//! obtained from library X is NEVER passed to library Y.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

// ---------------------------------------------------------------- FFI typedefs

// ZSTD_customMem = { allocFn, freeFn, opaque } — three pointers.
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_customMem {
    alloc: *mut c_void,
    free: *mut c_void,
    opaque: *mut c_void,
}
impl ZSTD_customMem {
    fn null() -> Self {
        ZSTD_customMem { alloc: std::ptr::null_mut(), free: std::ptr::null_mut(), opaque: std::ptr::null_mut() }
    }
}

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompressInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompressInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnCompressInitAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, ZSTD_parameters, c_ulonglong) -> size_t;
type FnCompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnCompressFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t) -> size_t;
type FnDecompressInit = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecompressInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnVoidSize = unsafe extern "C" fn() -> size_t;

use std::os::raw::c_ulonglong;

// -------------------------------------------------------------- ZBUFF err API

/// The ZBUFF error API (`ZBUFF_isError` / `ZBUFF_getErrorName`) delegates to
/// the shared `ERR_*` implementation, so it is a valid classifier for every
/// `size_t` result the ZBUFF functions return. We compare BOTH the boolean and
/// the string across libraries.
struct ZbuffErr {
    is_err: (libloading::Symbol<'static, FnIsError>, libloading::Symbol<'static, FnIsError>),
    name: (libloading::Symbol<'static, FnGetErrorName>, libloading::Symbol<'static, FnGetErrorName>),
}
impl ZbuffErr {
    unsafe fn new() -> Self {
        ZbuffErr {
            is_err: both::<FnIsError>("ZBUFF_isError"),
            name: both::<FnGetErrorName>("ZBUFF_getErrorName"),
        }
    }
    /// Assert C and Rust return values are equivalent, using ZBUFF's own error
    /// classifier (boolean + string).
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let c_is = (self.is_err.0)(cr) != 0;
        let r_is = (self.is_err.1)(rr) != 0;
        assert_eq!(c_is, r_is, "{ctx}: isError mismatch C={c_is} RS={r_is} (raw C={cr:#x} RS={rr:#x})");
        if c_is {
            let cn = cstr((self.name.0)(cr));
            let rn = cstr((self.name.1)(rr));
            assert_eq!(cn, rn, "{ctx}: error name mismatch (raw C={cr:#x} RS={rr:#x})");
        } else {
            // both OK: the hint / remaining-bytes value must match exactly
            assert_eq!(cr, rr, "{ctx}: OK return value differs C={cr:#x} RS={rr:#x}");
        }
    }
    unsafe fn is_c_err(&self, r: size_t) -> bool {
        (self.is_err.0)(r) != 0
    }
}

// ----------------------------------------------------------------- CCtx guard

struct CCtx {
    c: *mut c_void,
    r: *mut c_void,
    free: (libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>),
}
impl CCtx {
    unsafe fn new() -> Self {
        let (cc, rc) = both::<FnCreate>("ZBUFF_createCCtx");
        let c = cc();
        let r = rc();
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createCCtx returned null");
        CCtx { c, r, free: both::<FnFree>("ZBUFF_freeCCtx") }
    }
    unsafe fn new_advanced() -> Self {
        let (cc, rc) = both::<FnCreateAdv>("ZBUFF_createCCtx_advanced");
        let c = cc(ZSTD_customMem::null());
        let r = rc(ZSTD_customMem::null());
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createCCtx_advanced returned null");
        CCtx { c, r, free: both::<FnFree>("ZBUFF_freeCCtx") }
    }
}
impl Drop for CCtx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}

struct DCtx {
    c: *mut c_void,
    r: *mut c_void,
    free: (libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>),
}
impl DCtx {
    unsafe fn new() -> Self {
        let (cc, rc) = both::<FnCreate>("ZBUFF_createDCtx");
        let c = cc();
        let r = rc();
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createDCtx returned null");
        DCtx { c, r, free: both::<FnFree>("ZBUFF_freeDCtx") }
    }
    unsafe fn new_advanced() -> Self {
        let (cc, rc) = both::<FnCreateAdv>("ZBUFF_createDCtx_advanced");
        let c = cc(ZSTD_customMem::null());
        let r = rc(ZSTD_customMem::null());
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createDCtx_advanced returned null");
        DCtx { c, r, free: both::<FnFree>("ZBUFF_freeDCtx") }
    }
}
impl Drop for DCtx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}

// ------------------------------------------------------------------- helpers

const LEVELS: &[c_int] = &[-5, 1, 3, 9, 19, 22];

/// One full streaming compression of `src` through ZBUFF, returning the frame.
/// Drives C and Rust lock-step and asserts byte-identical output and identical
/// counter advancement at EVERY compressContinue / compressFlush / compressEnd
/// call. `in_chunk` / `out_chunk` control the granularity.
unsafe fn zbuff_compress_lockstep(
    ze: &ZbuffErr,
    cc: &CCtx,
    cont: &(libloading::Symbol<'static, FnCompressContinue>, libloading::Symbol<'static, FnCompressContinue>),
    flush: &(libloading::Symbol<'static, FnCompressFlush>, libloading::Symbol<'static, FnCompressFlush>),
    end: &(libloading::Symbol<'static, FnCompressFlush>, libloading::Symbol<'static, FnCompressFlush>),
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    ctx: &str,
) -> Vec<u8> {
    let in_chunk = in_chunk.max(1);
    let out_chunk = out_chunk.max(1);
    let mut cout: Vec<u8> = Vec::new();
    let mut rout: Vec<u8> = Vec::new();
    let mut ipos = 0usize;

    let mut cscratch = vec![0u8; out_chunk];
    let mut rscratch = vec![0u8; out_chunk];

    let mut step = 0usize;
    // consume input
    while ipos < src.len() {
        let take = in_chunk.min(src.len() - ipos);
        let mut c_src_sz = take;
        let mut r_src_sz = take;
        let mut c_dst_cap = out_chunk;
        let mut r_dst_cap = out_chunk;
        let cr = (cont.0)(
            cc.c,
            cscratch.as_mut_ptr() as *mut c_void,
            &mut c_dst_cap,
            src[ipos..].as_ptr() as *const c_void,
            &mut c_src_sz,
        );
        let rr = (cont.1)(
            cc.r,
            rscratch.as_mut_ptr() as *mut c_void,
            &mut r_dst_cap,
            src[ipos..].as_ptr() as *const c_void,
            &mut r_src_sz,
        );
        let sctx = format!("{ctx} continue step={step}");
        ze.eq(&sctx, cr, rr);
        assert_eq!(c_src_sz, r_src_sz, "{sctx}: srcConsumed differs C={c_src_sz} RS={r_src_sz}");
        assert_eq!(c_dst_cap, r_dst_cap, "{sctx}: dstWritten differs C={c_dst_cap} RS={r_dst_cap}");
        assert_bytes_eq(&format!("{sctx} bytes"), &cscratch[..c_dst_cap], &rscratch[..r_dst_cap]);
        cout.extend_from_slice(&cscratch[..c_dst_cap]);
        rout.extend_from_slice(&rscratch[..r_dst_cap]);
        // both consumed the same amount; ZBUFF may consume < take
        ipos += c_src_sz;
        step += 1;
        assert!(step < 20_000_000, "{ctx}: compressContinue not converging");
    }

    // end (flush + epilogue). Loop until the internal buffer is empty (return 0).
    step = 0;
    loop {
        let mut c_dst_cap = out_chunk;
        let mut r_dst_cap = out_chunk;
        let cr = (end.0)(cc.c, cscratch.as_mut_ptr() as *mut c_void, &mut c_dst_cap);
        let rr = (end.1)(cc.r, rscratch.as_mut_ptr() as *mut c_void, &mut r_dst_cap);
        let sctx = format!("{ctx} end step={step}");
        ze.eq(&sctx, cr, rr);
        assert_eq!(c_dst_cap, r_dst_cap, "{sctx}: dstWritten differs C={c_dst_cap} RS={r_dst_cap}");
        assert_bytes_eq(&format!("{sctx} bytes"), &cscratch[..c_dst_cap], &rscratch[..r_dst_cap]);
        cout.extend_from_slice(&cscratch[..c_dst_cap]);
        rout.extend_from_slice(&rscratch[..r_dst_cap]);
        step += 1;
        if ze.is_c_err(cr) {
            break;
        }
        if cr == 0 {
            break;
        }
        assert!(step < 20_000_000, "{ctx}: compressEnd not converging");
    }
    // the two libraries produced identical frames
    assert_bytes_eq(&format!("{ctx} full frame"), &cout, &rout);
    let _ = flush; // flush exercised separately
    cout
}

/// Full streaming decompression of `frame` through ZBUFF; asserts C and Rust
/// advance in lock-step and produce byte-identical output. Returns C output.
unsafe fn zbuff_decompress_lockstep(
    ze: &ZbuffErr,
    dc: &DCtx,
    cont: &(libloading::Symbol<'static, FnDecompressContinue>, libloading::Symbol<'static, FnDecompressContinue>),
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    ctx: &str,
) -> Vec<u8> {
    let in_chunk = in_chunk.max(1);
    let out_chunk = out_chunk.max(1);
    let mut cout: Vec<u8> = Vec::new();
    let mut rout: Vec<u8> = Vec::new();
    let mut ipos = 0usize;
    let mut cscratch = vec![0u8; out_chunk];
    let mut rscratch = vec![0u8; out_chunk];
    let mut step = 0usize;

    loop {
        let avail = frame.len() - ipos;
        let take = in_chunk.min(avail);
        let mut c_src_sz = take;
        let mut r_src_sz = take;
        let mut c_dst_cap = out_chunk;
        let mut r_dst_cap = out_chunk;
        let src_ptr = if avail == 0 {
            std::ptr::null()
        } else {
            frame[ipos..].as_ptr() as *const c_void
        };
        let cr = (cont.0)(dc.c, cscratch.as_mut_ptr() as *mut c_void, &mut c_dst_cap, src_ptr, &mut c_src_sz);
        let rr = (cont.1)(dc.r, rscratch.as_mut_ptr() as *mut c_void, &mut r_dst_cap, src_ptr, &mut r_src_sz);
        let sctx = format!("{ctx} decompress step={step}");
        ze.eq(&sctx, cr, rr);
        assert_eq!(c_src_sz, r_src_sz, "{sctx}: srcConsumed differs C={c_src_sz} RS={r_src_sz}");
        assert_eq!(c_dst_cap, r_dst_cap, "{sctx}: dstWritten differs C={c_dst_cap} RS={r_dst_cap}");
        assert_bytes_eq(&format!("{sctx} bytes"), &cscratch[..c_dst_cap], &rscratch[..r_dst_cap]);
        cout.extend_from_slice(&cscratch[..c_dst_cap]);
        rout.extend_from_slice(&rscratch[..r_dst_cap]);
        ipos += c_src_sz;
        step += 1;
        if ze.is_c_err(cr) {
            break;
        }
        if cr == 0 {
            break; // frame fully decoded
        }
        if c_src_sz == 0 && c_dst_cap == 0 && avail == 0 {
            break; // no more input, no progress
        }
        assert!(step < 1_000_000, "{ctx}: decompress not converging");
    }
    assert_bytes_eq(&format!("{ctx} full output"), &cout, &rout);
    cout
}

// ------------------------------------------------------------------- tests

/// Recommended buffer size helpers must agree between C and Rust.
#[test]
fn zbuff_recommended_sizes() {
    unsafe {
        for name in [
            "ZBUFF_recommendedCInSize",
            "ZBUFF_recommendedCOutSize",
            "ZBUFF_recommendedDInSize",
            "ZBUFF_recommendedDOutSize",
        ] {
            let (a, b) = both::<FnVoidSize>(name);
            assert_eq!(a(), b(), "{name}");
        }
    }
}

/// create / free lifecycle (plain + advanced) for both CCtx and DCtx.
#[test]
fn zbuff_create_free_lifecycle() {
    unsafe {
        let ze = ZbuffErr::new();
        for _ in 0..64 {
            let cc = CCtx::new();
            let cca = CCtx::new_advanced();
            let dc = DCtx::new();
            let dca = DCtx::new_advanced();
            // dropping frees; nothing else to assert beyond non-null (checked in new()).
            drop((cc, cca, dc, dca));
        }
        // free(NULL) must be a no-op returning identical results.
        let (cf, rf) = both::<FnFree>("ZBUFF_freeCCtx");
        ze.eq("freeCCtx(NULL)", cf(std::ptr::null_mut()), rf(std::ptr::null_mut()));
        let (cdf, rdf) = both::<FnFree>("ZBUFF_freeDCtx");
        ze.eq("freeDCtx(NULL)", cdf(std::ptr::null_mut()), rdf(std::ptr::null_mut()));
    }
}

/// Full ZBUFF round-trips: compress with ZBUFF, decompress with ZBUFF, across
/// ALL_SHAPES, a spread of lengths, every level, and every in/out chunk size
/// (including the recommended sizes). Asserts byte-identical compressed frames
/// and identical decoded output at every step.
#[test]
fn zbuff_compress_decompress_roundtrip() {
    unsafe {
        let ze = ZbuffErr::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let cinit = both::<FnCompressInit>("ZBUFF_compressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        let (crci, _) = both::<FnVoidSize>("ZBUFF_recommendedCInSize");
        let (crco, _) = both::<FnVoidSize>("ZBUFF_recommendedCOutSize");
        let (crdi, _) = both::<FnVoidSize>("ZBUFF_recommendedDInSize");
        let (crdo, _) = both::<FnVoidSize>("ZBUFF_recommendedDOutSize");
        let rec_ci = crci();
        let rec_co = crco();
        let rec_di = crdi();
        let rec_do = crdo();

        let mut rng = Rng::new(0xB14_0001);
        // A curated subset of lengths keeps the (shape x len x level x chunk)
        // matrix tractable while still hitting the boundaries.
        let lens: &[usize] = &[0, 1, 2, 7, 64, 100, 1024, 1025, 8192, 20000, 65536, 131_072];

        for &shape in ALL_SHAPES {
            for &len in lens {
                let src = gen(shape, len, &mut rng);
                for &lvl in LEVELS {
                    // Cover every requested chunk size {1,2,7,64,1024,recommended}
                    // but bound the number of streaming iterations: tiny output
                    // chunks are only paired with small inputs (otherwise a 1-byte
                    // output buffer on a 128 KB input needs >128k calls). Small
                    // lengths still exercise oc=1/2 exhaustively.
                    let small_ok = len <= 2048;
                    let oc_pool: &[usize] = if small_ok {
                        &[1, 2, 7, 64, 1024]
                    } else {
                        &[64, 1024]
                    };
                    let ic_pool: &[usize] = if small_ok {
                        &[1, 2, 7, 64, 1024]
                    } else {
                        &[7, 64, 1024]
                    };
                    let combos: [(usize, usize); 3] = [
                        (ic_pool[rng.below(ic_pool.len())], oc_pool[rng.below(oc_pool.len())]),
                        (rec_ci, rec_co),
                        if small_ok { (1, 1) } else { (7, 64) },
                    ];
                    for (ic, oc) in combos {
                        let ctx = format!("rt shape={shape:?} len={len} lvl={lvl} ic={ic} oc={oc}");
                        // fresh init each round
                        let cc = CCtx::new();
                        ze.eq(&format!("{ctx} cinit"), (cinit.0)(cc.c, lvl), (cinit.1)(cc.r, lvl));
                        let frame = zbuff_compress_lockstep(
                            &ze, &cc, &cont, &flush, &end, &src, ic, oc, &ctx,
                        );

                        // decompress via ZBUFF, using (possibly different) chunking
                        let dc = DCtx::new();
                        ze.eq(&format!("{ctx} dinit"), (dinit.0)(dc.c), (dinit.1)(dc.r));
                        let dic = *[rec_di, 1, 64, 1024].get(rng.below(4)).unwrap();
                        let doc = if small_ok {
                            *[rec_do, 1, 64, 4096].get(rng.below(4)).unwrap()
                        } else {
                            *[rec_do, 64, 4096].get(rng.below(3)).unwrap()
                        };
                        let decoded = zbuff_decompress_lockstep(
                            &ze, &dc, &dcont, &frame, dic, doc,
                            &format!("{ctx} dic={dic} doc={doc}"),
                        );
                        assert_bytes_eq(&format!("{ctx} decoded==src"), &decoded, &src);
                    }
                }
            }
        }
    }
}

/// Exercise `ZBUFF_compressFlush` explicitly: compress input in chunks,
/// interleaving flushes, then end. Asserts identical output + counters.
#[test]
fn zbuff_compress_flush_interleaved() {
    unsafe {
        let ze = ZbuffErr::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let cinit = both::<FnCompressInit>("ZBUFF_compressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");

        let mut rng = Rng::new(0xB14_0002);
        let shapes = [Shape::Text, Shape::Random, Shape::Repeating, Shape::Zeros, Shape::LongMatches];
        for &shape in &shapes {
            for &len in &[0usize, 1, 100, 1024, 5000, 40_000, 131_072] {
                let src = gen(shape, len, &mut rng);
                for &lvl in &[1, 9, 19] {
                    let ctx = format!("flush shape={shape:?} len={len} lvl={lvl}");
                    let cc = CCtx::new();
                    ze.eq(&format!("{ctx} cinit"), (cinit.0)(cc.c, lvl), (cinit.1)(cc.r, lvl));

                    let out_chunk = 1 + rng.below(200);
                    let in_chunk = 1 + rng.below(500);
                    let mut cout: Vec<u8> = Vec::new();
                    let mut rout: Vec<u8> = Vec::new();
                    let mut cscratch = vec![0u8; out_chunk];
                    let mut rscratch = vec![0u8; out_chunk];
                    let mut ipos = 0usize;
                    let mut step = 0usize;

                    while ipos < src.len() {
                        let take = in_chunk.min(src.len() - ipos);
                        let mut c_ss = take;
                        let mut r_ss = take;
                        let mut c_dc = out_chunk;
                        let mut r_dc = out_chunk;
                        let cr = (cont.0)(cc.c, cscratch.as_mut_ptr() as *mut c_void, &mut c_dc,
                            src[ipos..].as_ptr() as *const c_void, &mut c_ss);
                        let rr = (cont.1)(cc.r, rscratch.as_mut_ptr() as *mut c_void, &mut r_dc,
                            src[ipos..].as_ptr() as *const c_void, &mut r_ss);
                        let sctx = format!("{ctx} continue step={step}");
                        ze.eq(&sctx, cr, rr);
                        assert_eq!(c_ss, r_ss, "{sctx} srcConsumed");
                        assert_eq!(c_dc, r_dc, "{sctx} dstWritten");
                        assert_bytes_eq(&sctx, &cscratch[..c_dc], &rscratch[..r_dc]);
                        cout.extend_from_slice(&cscratch[..c_dc]);
                        rout.extend_from_slice(&rscratch[..r_dc]);
                        ipos += c_ss;
                        step += 1;
                        // occasionally flush
                        if rng.bool() {
                            loop {
                                let mut c_dc = out_chunk;
                                let mut r_dc = out_chunk;
                                let cf = (flush.0)(cc.c, cscratch.as_mut_ptr() as *mut c_void, &mut c_dc);
                                let rf = (flush.1)(cc.r, rscratch.as_mut_ptr() as *mut c_void, &mut r_dc);
                                let fctx = format!("{ctx} flush step={step}");
                                ze.eq(&fctx, cf, rf);
                                assert_eq!(c_dc, r_dc, "{fctx} dstWritten");
                                assert_bytes_eq(&fctx, &cscratch[..c_dc], &rscratch[..r_dc]);
                                cout.extend_from_slice(&cscratch[..c_dc]);
                                rout.extend_from_slice(&rscratch[..r_dc]);
                                if ze.is_c_err(cf) || cf == 0 { break; }
                            }
                        }
                        if c_ss == 0 && c_dc == 0 { break; }
                    }
                    // end
                    loop {
                        let mut c_dc = out_chunk;
                        let mut r_dc = out_chunk;
                        let ce = (end.0)(cc.c, cscratch.as_mut_ptr() as *mut c_void, &mut c_dc);
                        let re = (end.1)(cc.r, rscratch.as_mut_ptr() as *mut c_void, &mut r_dc);
                        let ectx = format!("{ctx} end");
                        ze.eq(&ectx, ce, re);
                        assert_eq!(c_dc, r_dc, "{ectx} dstWritten");
                        assert_bytes_eq(&ectx, &cscratch[..c_dc], &rscratch[..r_dc]);
                        cout.extend_from_slice(&cscratch[..c_dc]);
                        rout.extend_from_slice(&rscratch[..r_dc]);
                        if ze.is_c_err(ce) || ce == 0 { break; }
                    }
                    assert_bytes_eq(&format!("{ctx} frame"), &cout, &rout);

                    // verify it decodes back to src through ZBUFF
                    let dc = DCtx::new();
                    ze.eq(&format!("{ctx} dinit"), (dinit.0)(dc.c), (dinit.1)(dc.r));
                    let decoded = zbuff_decompress_lockstep(&ze, &dc, &dcont, &cout, 64, 4096,
                        &format!("{ctx} decode"));
                    assert_bytes_eq(&format!("{ctx} decoded==src"), &decoded, &src);
                }
            }
        }
    }
}

/// `ZBUFF_compressInitDictionary` + `ZBUFF_decompressInitDictionary`:
/// dictionary round-trips must be byte-identical between C and Rust.
#[test]
fn zbuff_dictionary_roundtrip() {
    unsafe {
        let ze = ZbuffErr::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let cinitd = both::<FnCompressInitDict>("ZBUFF_compressInitDictionary");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let dinitd = both::<FnDecompressInitDict>("ZBUFF_decompressInitDictionary");

        let mut rng = Rng::new(0xB14_0003);
        for &dlen in &[0usize, 1, 64, 1024, 8192] {
            let dict = gen(Shape::Text, dlen, &mut rng);
            let dptr = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
            for &shape in &[Shape::Text, Shape::Random, Shape::Repeating] {
                for &len in &[0usize, 100, 1024, 20_000] {
                    let src = gen(shape, len, &mut rng);
                    for &lvl in &[1, 3, 9, 19] {
                        let ctx = format!("dict dlen={dlen} shape={shape:?} len={len} lvl={lvl}");
                        let cc = CCtx::new();
                        ze.eq(&format!("{ctx} cinitd"),
                            (cinitd.0)(cc.c, dptr, dict.len(), lvl),
                            (cinitd.1)(cc.r, dptr, dict.len(), lvl));
                        let frame = zbuff_compress_lockstep(&ze, &cc, &cont, &flush, &end, &src, 1024, 1024, &ctx);

                        let dc = DCtx::new();
                        ze.eq(&format!("{ctx} dinitd"),
                            (dinitd.0)(dc.c, dptr, dict.len()),
                            (dinitd.1)(dc.r, dptr, dict.len()));
                        let decoded = zbuff_decompress_lockstep(&ze, &dc, &dcont, &frame, 1024, 4096, &ctx);
                        assert_bytes_eq(&format!("{ctx} decoded==src"), &decoded, &src);
                    }
                }
            }
        }
    }
}

/// `ZBUFF_compressInit_advanced` (static-linking API) with derived
/// `ZSTD_parameters` and a pledged source size. Round-trips must match.
#[test]
fn zbuff_compress_init_advanced_roundtrip() {
    unsafe {
        let ze = ZbuffErr::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let cinita = both::<FnCompressInitAdv>("ZBUFF_compressInit_advanced");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        // derive valid parameters through the modern API
        type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters;
        let (cgp, _) = both::<FnGetParams>("ZSTD_getParams");

        let mut rng = Rng::new(0xB14_0004);
        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating, Shape::Sequential] {
            for &len in &[0usize, 100, 1024, 20_000, 131_072] {
                let src = gen(shape, len, &mut rng);
                for &lvl in &[1, 3, 9, 19, 22] {
                    let params = cgp(lvl, src.len() as c_ulonglong, 0);
                    for &pledged in &[0u64, src.len() as u64] {
                        let ctx = format!("adv shape={shape:?} len={len} lvl={lvl} pledged={pledged}");
                        let cc = CCtx::new_advanced();
                        ze.eq(&format!("{ctx} cinita"),
                            (cinita.0)(cc.c, std::ptr::null(), 0, params, pledged),
                            (cinita.1)(cc.r, std::ptr::null(), 0, params, pledged));
                        let frame = zbuff_compress_lockstep(&ze, &cc, &cont, &flush, &end, &src, 1024, 1024, &ctx);

                        let dc = DCtx::new_advanced();
                        ze.eq(&format!("{ctx} dinit"), (dinit.0)(dc.c), (dinit.1)(dc.r));
                        let decoded = zbuff_decompress_lockstep(&ze, &dc, &dcont, &frame, 1024, 4096, &ctx);
                        assert_bytes_eq(&format!("{ctx} decoded==src"), &decoded, &src);
                    }
                }
            }
        }
    }
}

/// A ZBUFF frame must be decodable by the modern `ZSTD_decompress`, and a
/// modern frame must be decodable by `ZBUFF_decompressContinue` — proving the
/// "100% interoperable" contract holds identically in both libraries.
#[test]
fn zbuff_interop_with_modern_api() {
    unsafe {
        let ze = ZbuffErr::new();
        let e = Err2::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let cinit = both::<FnCompressInit>("ZBUFF_compressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
        type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
        type FnBound = unsafe extern "C" fn(size_t) -> size_t;
        let (cc_mod, _rc_mod) = both::<FnCompress>("ZSTD_compress");
        let (cd_mod, rd_mod) = both::<FnDecompress>("ZSTD_decompress");
        let (cb, _) = both::<FnBound>("ZSTD_compressBound");

        let mut rng = Rng::new(0xB14_0005);
        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating] {
            for &len in &[0usize, 100, 1024, 20_000] {
                let src = gen(shape, len, &mut rng);
                let ctx = format!("interop shape={shape:?} len={len}");

                // (1) ZBUFF-produced frame -> modern ZSTD_decompress
                let cc = CCtx::new();
                ze.eq(&format!("{ctx} cinit"), (cinit.0)(cc.c, 5), (cinit.1)(cc.r, 5));
                let frame = zbuff_compress_lockstep(&ze, &cc, &cont, &flush, &end, &src, 1024, 1024, &ctx);
                let mut d1 = vec![0u8; src.len() + 16];
                let mut d2 = vec![0u8; src.len() + 16];
                let a = cd_mod(d1.as_mut_ptr() as *mut c_void, d1.len(), frame.as_ptr() as *const c_void, frame.len());
                let b = rd_mod(d2.as_mut_ptr() as *mut c_void, d2.len(), frame.as_ptr() as *const c_void, frame.len());
                e.eq(&format!("{ctx} modern-decode zbuff frame"), a, b);
                assert_eq!(a, src.len(), "{ctx}: modern decode size");
                assert_bytes_eq(&format!("{ctx} modern decoded"), &d1[..a], &src);

                // (2) modern frame -> ZBUFF_decompressContinue
                let cap = cb(src.len()) + 64;
                let mut mframe = vec![0u8; cap];
                let mn = cc_mod(mframe.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), 7);
                assert!(!e.c.is_err(mn), "{ctx}: modern compress failed");
                mframe.truncate(mn);
                let dc = DCtx::new();
                ze.eq(&format!("{ctx} dinit"), (dinit.0)(dc.c), (dinit.1)(dc.r));
                let decoded = zbuff_decompress_lockstep(&ze, &dc, &dcont, &mframe, 1024, 4096, &format!("{ctx} zbuff-decode modern"));
                assert_bytes_eq(&format!("{ctx} zbuff decoded modern"), &decoded, &src);
            }
        }
    }
}
