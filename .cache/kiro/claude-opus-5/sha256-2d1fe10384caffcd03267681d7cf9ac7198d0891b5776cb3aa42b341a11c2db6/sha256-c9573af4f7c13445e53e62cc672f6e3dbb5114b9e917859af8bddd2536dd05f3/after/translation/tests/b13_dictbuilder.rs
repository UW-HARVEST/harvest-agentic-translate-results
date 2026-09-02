//! Phase B row 13: differential tests for the ZDICT dictionary-builder API
//! (valid paths). Every training entry point is exercised over a swept space
//! of corpora and parameters; the produced dictionary bytes, returned size,
//! `ZDICT_getDictID` and `ZDICT_getDictHeaderSize` must all agree
//! byte-for-byte between the C build and the Rust translation. Produced
//! dictionaries are also round-tripped through
//! `ZSTD_compress_usingDict` / `ZSTD_decompress_usingDict` and the compressed
//! output must be byte-identical.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

// ----------------------------------------------------------------- FFI structs

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZDICT_cover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZDICT_fastCover_params_t {
    pub k: c_uint,
    pub d: c_uint,
    pub f: c_uint,
    pub steps: c_uint,
    pub nbThreads: c_uint,
    pub splitPoint: f64,
    pub accel: c_uint,
    pub shrinkDict: c_uint,
    pub shrinkDictMaxRegression: c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: c_uint,
    pub zParams: ZDICT_params_t,
}

// cover.h shared types. The lib is built WITHOUT ZSTD_MULTITHREAD, so
// ZSTD_pthread_mutex_t / ZSTD_pthread_cond_t are plain `int`.
pub type ZSTD_pthread_mutex_t = c_int;
pub type ZSTD_pthread_cond_t = c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_best_t {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub liveJobs: size_t,
    pub dict: *mut c_void,
    pub dictSize: size_t,
    pub parameters: ZDICT_cover_params_t,
    pub compressedSize: size_t,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct COVER_epoch_info_t {
    pub num: c_uint,
    pub size: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct COVER_dictSelection_t {
    pub dictContent: *mut u8,
    pub dictSize: size_t,
    pub totalCompressedSize: size_t,
}

// ------------------------------------------------------------------- fn types

type FnTrain = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, *const size_t, c_uint) -> size_t;
type FnTrainCover = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint, ZDICT_cover_params_t,
) -> size_t;
type FnOptCover = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint, *mut ZDICT_cover_params_t,
) -> size_t;
type FnTrainFast = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint, ZDICT_fastCover_params_t,
) -> size_t;
type FnOptFast = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint, *mut ZDICT_fastCover_params_t,
) -> size_t;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, *const size_t, c_uint, ZDICT_legacy_params_t,
) -> size_t;
type FnFinalize = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, *const c_void, *const size_t, c_uint, ZDICT_params_t,
) -> size_t;
type FnAddEntropy = unsafe extern "C" fn(
    *mut c_void, size_t, size_t, *const c_void, *const size_t, c_uint,
) -> size_t;
type FnGetDictID = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetDictHeaderSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;

type FnZdIsError = unsafe extern "C" fn(size_t) -> c_uint;
type FnZdErrName = unsafe extern "C" fn(size_t) -> *const std::os::raw::c_char;

// COVER helpers
type FnCoverSum = unsafe extern "C" fn(*const size_t, c_uint) -> size_t;
type FnComputeEpochs = unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> COVER_epoch_info_t;
type FnWarn = unsafe extern "C" fn(size_t, size_t, c_int);
type FnBestVoid = unsafe extern "C" fn(*mut COVER_best_t);
type FnBestFinish = unsafe extern "C" fn(*mut COVER_best_t, ZDICT_cover_params_t, COVER_dictSelection_t);
type FnDictSelErr = unsafe extern "C" fn(size_t) -> COVER_dictSelection_t;
type FnDictSelIsErr = unsafe extern "C" fn(COVER_dictSelection_t) -> c_uint;
type FnDictSelFree = unsafe extern "C" fn(COVER_dictSelection_t);
type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut c_int, c_int, c_int) -> c_int;
type FnDivbwt = unsafe extern "C" fn(*const u8, *mut u8, *mut c_int, c_int, *mut u8, *mut c_int, c_int) -> c_int;

// compress-using-dict roundtrip
type FnCreateCCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeCCtx = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCreateDCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeDCtx = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t, c_int,
) -> size_t;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t,
) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;

// -------------------------------------------------------------------- helpers

const ZDICT_DICTSIZE_MIN: size_t = 256;

/// A concatenated sample corpus + its sizes array.
struct Corpus {
    buf: Vec<u8>,
    sizes: Vec<size_t>,
}
impl Corpus {
    fn nb(&self) -> c_uint {
        self.sizes.len() as c_uint
    }
    fn buf_ptr(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
    fn sizes_ptr(&self) -> *const size_t {
        self.sizes.as_ptr()
    }
}

/// Build a corpus of `nb` samples, each of size `sample_len`, mixing shapes so
/// the corpus has cross-sample repetition (needed for training to succeed).
/// NOTE: gen() is always called with `v.len()` as the size argument.
fn build_corpus(nb: usize, sample_len: usize, rng: &mut Rng, shapes: &[Shape]) -> Corpus {
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(nb);
    // A shared "backbone" gives cross-sample structure the trainer can find.
    let backbone = gen(Shape::Text, 96, rng);
    for i in 0..nb {
        let shape = shapes[i % shapes.len()];
        let mut v = gen(shape, sample_len, rng);
        // splice the backbone in so samples share content
        if !v.is_empty() && !backbone.is_empty() {
            let n = backbone.len().min(v.len());
            v[..n].copy_from_slice(&backbone[..n]);
        }
        sizes.push(v.len());
        buf.extend_from_slice(&v);
    }
    Corpus { buf, sizes }
}

/// Build a corpus with random per-sample sizes drawn from `size_choices`.
fn build_corpus_mixed(nb: usize, size_choices: &[usize], rng: &mut Rng, shapes: &[Shape]) -> Corpus {
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(nb);
    let backbone = gen(Shape::Text, 128, rng);
    for i in 0..nb {
        let shape = shapes[rng.below(shapes.len())];
        let sl = size_choices[rng.below(size_choices.len())];
        let mut v = gen(shape, sl, rng);
        if !v.is_empty() && !backbone.is_empty() {
            let n = backbone.len().min(v.len());
            v[..n].copy_from_slice(&backbone[..n]);
        }
        sizes.push(v.len());
        buf.extend_from_slice(&v);
    }
    Corpus { buf, sizes }
}

struct DictApi {
    getid: (
        libloading::Symbol<'static, FnGetDictID>,
        libloading::Symbol<'static, FnGetDictID>,
    ),
    gethdr: (
        libloading::Symbol<'static, FnGetDictHeaderSize>,
        libloading::Symbol<'static, FnGetDictHeaderSize>,
    ),
    is_err: (
        libloading::Symbol<'static, FnZdIsError>,
        libloading::Symbol<'static, FnZdIsError>,
    ),
    err_name: (
        libloading::Symbol<'static, FnZdErrName>,
        libloading::Symbol<'static, FnZdErrName>,
    ),
}
impl DictApi {
    unsafe fn new() -> Self {
        DictApi {
            getid: both::<FnGetDictID>("ZDICT_getDictID"),
            gethdr: both::<FnGetDictHeaderSize>("ZDICT_getDictHeaderSize"),
            is_err: both::<FnZdIsError>("ZDICT_isError"),
            err_name: both::<FnZdErrName>("ZDICT_getErrorName"),
        }
    }
    unsafe fn c_is_err(&self, r: size_t) -> bool {
        (self.is_err.0)(r) != 0
    }
    /// Assert the ZDICT return codes match (both OK with same size, or both
    /// errors with the same isError bool and error-name string).
    unsafe fn assert_ret_eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let ce = (self.is_err.0)(cr) != 0;
        let re = (self.is_err.1)(rr) != 0;
        assert_eq!(ce, re, "{ctx}: ZDICT_isError bool differs C={ce} RS={re} (C={cr:#x} RS={rr:#x})");
        if ce {
            let cn = cstr((self.err_name.0)(cr));
            let rn = cstr((self.err_name.1)(rr));
            assert_eq!(cn, rn, "{ctx}: error name differs C={cn:?} RS={rn:?}");
        } else {
            assert_eq!(cr, rr, "{ctx}: OK size differs C={cr} RS={rr}");
        }
    }
    /// When both produced a valid dictionary of identical bytes, also assert
    /// getDictID / getDictHeaderSize agree across libs and across the two
    /// dictionary buffers.
    unsafe fn assert_dict_meta(&self, ctx: &str, cdict: &[u8], rdict: &[u8]) {
        assert_bytes_eq(ctx, cdict, rdict);
        let cp = cdict.as_ptr() as *const c_void;
        let rp = rdict.as_ptr() as *const c_void;
        let id_cc = (self.getid.0)(cp, cdict.len());
        let id_rc = (self.getid.1)(cp, cdict.len());
        let id_rr = (self.getid.1)(rp, rdict.len());
        assert_eq!(id_cc, id_rc, "{ctx}: getDictID(C-lib vs RS-lib on C dict) {id_cc} {id_rc}");
        assert_eq!(id_cc, id_rr, "{ctx}: getDictID differs on RS dict {id_cc} {id_rr}");
        let h_cc = (self.gethdr.0)(cp, cdict.len());
        let h_rc = (self.gethdr.1)(cp, cdict.len());
        let h_rr = (self.gethdr.1)(rp, rdict.len());
        self.assert_ret_eq(&format!("{ctx}: getDictHeaderSize C-lib vs RS-lib"), h_cc, h_rc);
        self.assert_ret_eq(&format!("{ctx}: getDictHeaderSize on RS dict"), h_cc, h_rr);
    }
}

/// After both libs return a valid dictionary, verify the dictionary actually
/// works: compress+decompress a sample with it, in each library, and assert
/// the compressed output is byte-identical and round-trips.
unsafe fn verify_dict_roundtrip(ctx: &str, dict: &[u8], sample: &[u8], level: c_int) {
    let (ccc, rcc) = both::<FnCreateCCtx>("ZSTD_createCCtx");
    let (cfc, rfc) = both::<FnFreeCCtx>("ZSTD_freeCCtx");
    let (cdc, rdc) = both::<FnCreateDCtx>("ZSTD_createDCtx");
    let (cfd, rfd) = both::<FnFreeDCtx>("ZSTD_freeDCtx");
    let (ccu, rcu) = both::<FnCompressUsingDict>("ZSTD_compress_usingDict");
    let (cdu, rdu) = both::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
    let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
    let e = Err2::new();

    let cap = cb(sample.len()) + 64;
    let mut c_out = vec![0u8; cap];
    let mut r_out = vec![0u8; cap];

    let cctx_c = ccc();
    let cctx_r = rcc();
    let cn = ccu(cctx_c, c_out.as_mut_ptr() as *mut c_void, cap,
                 sample.as_ptr() as *const c_void, sample.len(),
                 dict.as_ptr() as *const c_void, dict.len(), level);
    let rn = rcu(cctx_r, r_out.as_mut_ptr() as *mut c_void, cap,
                 sample.as_ptr() as *const c_void, sample.len(),
                 dict.as_ptr() as *const c_void, dict.len(), level);
    e.eq(&format!("{ctx}: compress_usingDict"), cn, rn);
    if !e.c.is_err(cn) {
        assert_bytes_eq(&format!("{ctx}: compress_usingDict bytes"), &c_out[..cn], &r_out[..rn]);
        // round-trip decode each library's frame with each library
        let mut d1 = vec![0u8; sample.len() + 16];
        let mut d2 = vec![0u8; sample.len() + 16];
        let dctx_c = cdc();
        let dctx_r = rdc();
        let a = cdu(dctx_c, d1.as_mut_ptr() as *mut c_void, d1.len(),
                    c_out.as_ptr() as *const c_void, cn,
                    dict.as_ptr() as *const c_void, dict.len());
        let b = rdu(dctx_r, d2.as_mut_ptr() as *mut c_void, d2.len(),
                    r_out.as_ptr() as *const c_void, rn,
                    dict.as_ptr() as *const c_void, dict.len());
        e.eq(&format!("{ctx}: decompress_usingDict"), a, b);
        if !e.c.is_err(a) {
            assert_eq!(a, sample.len(), "{ctx}: roundtrip size");
            assert_bytes_eq(&format!("{ctx}: decoded"), &d1[..a], sample);
            assert_bytes_eq(&format!("{ctx}: decoded rs"), &d2[..b], sample);
        }
        cfd(dctx_c);
        rfd(dctx_r);
    }
    cfc(cctx_c);
    rfc(cctx_r);
}

fn zp(compressionLevel: c_int, notificationLevel: c_uint, dictID: c_uint) -> ZDICT_params_t {
    ZDICT_params_t { compressionLevel, notificationLevel, dictID }
}

// ============================================================================
// Tests
// ============================================================================

/// ZDICT_trainFromBuffer (the simple API): sweep corpora sizes/shapes and
/// dictBufferCapacity. Assert dict bytes, size, dictID, headerSize agree, and
/// the produced dict round-trips identically.
#[test]
fn train_from_buffer_simple() {
    unsafe {
        let (ct, rt) = both::<FnTrain>("ZDICT_trainFromBuffer");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0001);

        let nb_samples = [16usize, 64, 256, 1000];
        let sample_sizes = [64usize, 512, 1024];
        let caps = [256usize, 1024, 4096, 16384, 112640];

        for &nb in &nb_samples {
            for &sl in &sample_sizes {
                // keep total corpus modest
                if nb * sl > 1_500_000 {
                    continue;
                }
                let corpus = build_corpus(nb, sl, &mut rng, ALL_SHAPES);
                for &cap in &caps {
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                    let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                    let ctx = format!("trainFromBuffer nb={nb} sl={sl} cap={cap}");
                    api.assert_ret_eq(&ctx, cn, rn);
                    if !api.c_is_err(cn) {
                        api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                        // roundtrip against one of the samples
                        let s0 = &corpus.buf[..corpus.sizes[0].min(corpus.buf.len())];
                        verify_dict_roundtrip(&ctx, &cbuf[..cn], s0, 3);
                    }
                }
            }
        }

        // mixed / random sizes and tiny corpora
        for &nb in &[1usize, 2, 4] {
            let corpus = build_corpus_mixed(nb, &[8, 64, 512, 1024, 8192], &mut rng, ALL_SHAPES);
            for &cap in &[256usize, 4096, 16384] {
                let mut cbuf = vec![0u8; cap];
                let mut rbuf = vec![0u8; cap];
                let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                let ctx = format!("trainFromBuffer mixed nb={nb} cap={cap}");
                api.assert_ret_eq(&ctx, cn, rn);
                if !api.c_is_err(cn) {
                    api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                }
            }
        }
    }
}

/// ZDICT_trainFromBuffer_cover: sweep k, d, steps=0 (no optimization),
/// nbThreads, splitPoint, shrinkDict, shrinkDictMaxRegression + zParams.
#[test]
fn train_from_buffer_cover() {
    unsafe {
        let (ct, rt) = both::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0002);

        // one moderately sized corpus reused across parameter combos
        let corpus = build_corpus(256, 512, &mut rng, ALL_SHAPES);
        let cap = 16384usize;

        let ks = [16u32, 32, 50, 64, 200, 256, 1024, 2048];
        let ds = [6u32, 8];
        let split = [0.0f64, 0.5, 0.75, 1.0];
        let shrink = [0u32, 1];
        let regress = [0u32, 1, 5];
        let zparams = [
            zp(0, 0, 0),
            zp(3, 0, 12345),
            zp(9, 0, 1),
            zp(19, 0, u32::MAX),
        ];

        let mut count = 0usize;
        for &k in &ks {
            for &d in &ds {
                for &sp in &split {
                    for &sd in &shrink {
                        for &reg in &regress {
                            let z = zparams[count % zparams.len()];
                            count += 1;
                            let params = ZDICT_cover_params_t {
                                k, d, steps: 0, nbThreads: 1, splitPoint: sp,
                                shrinkDict: sd, shrinkDictMaxRegression: reg, zParams: z,
                            };
                            let mut cbuf = vec![0u8; cap];
                            let mut rbuf = vec![0u8; cap];
                            let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                            let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                            let ctx = format!("trainFromBuffer_cover k={k} d={d} sp={sp} sd={sd} reg={reg}");
                            api.assert_ret_eq(&ctx, cn, rn);
                            if !api.c_is_err(cn) {
                                api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                            }
                        }
                    }
                }
            }
        }

        // nbThreads variation (0 and 1) and a roundtrip check
        for &nt in &[0u32, 1] {
            let params = ZDICT_cover_params_t {
                k: 64, d: 8, steps: 0, nbThreads: nt, splitPoint: 1.0,
                shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 7),
            };
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
            let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
            let ctx = format!("trainFromBuffer_cover nbThreads={nt}");
            api.assert_ret_eq(&ctx, cn, rn);
            if !api.c_is_err(cn) {
                api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                let s0 = &corpus.buf[..corpus.sizes[0]];
                verify_dict_roundtrip(&ctx, &cbuf[..cn], s0, 3);
            }
        }
    }
}

/// ZDICT_optimizeTrainFromBuffer_cover: steps sweep. Optimization is slow, so
/// use small steps and one small corpus. Also assert the out-param
/// (`*parameters`) selected values agree.
#[test]
fn optimize_train_cover() {
    unsafe {
        let (ct, rt) = both::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0003);
        let corpus = build_corpus(128, 256, &mut rng, ALL_SHAPES);
        let cap = 8192usize;

        for &steps in &[0u32, 1, 4] {
            for &d in &[6u32, 8] {
                for &sp in &[0.0f64, 0.75, 1.0] {
                    // Fix k so we don't explore the huge [50,2000] range (slow).
                    let base = ZDICT_cover_params_t {
                        k: 200, d, steps, nbThreads: 1, splitPoint: sp,
                        shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
                    };
                    let mut cp = base;
                    let mut rp = base;
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
                    let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
                    let ctx = format!("optimizeTrain_cover steps={steps} d={d} sp={sp}");
                    api.assert_ret_eq(&ctx, cn, rn);
                    if !api.c_is_err(cn) {
                        api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                        assert_eq!(cp, rp, "{ctx}: selected params differ");
                    }
                }
            }
        }
    }
}

/// ZDICT_trainFromBuffer_fastCover: k, d, f, accel, split, shrink sweep.
#[test]
fn train_from_buffer_fastcover() {
    unsafe {
        let (ct, rt) = both::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0004);
        let corpus = build_corpus(256, 512, &mut rng, ALL_SHAPES);
        let cap = 16384usize;

        let ks = [16u32, 32, 50, 64, 200, 256, 1024, 2048];
        let ds = [6u32, 8];
        let fs = [6u32, 8, 15, 20, 23, 24, 25];
        let accels = [0u32, 1, 2, 10, 15, 16];
        let split = [0.0f64, 0.5, 0.75, 1.0];
        let shrink = [0u32, 1];
        let regress = [0u32, 1, 5];
        let zparams = [zp(0, 0, 0), zp(3, 0, 12345), zp(9, 0, 1), zp(19, 0, u32::MAX)];

        let mut count = 0usize;
        // full k×d×f is large; iterate f/accel with a rotating k to bound cost.
        for (fi, &f) in fs.iter().enumerate() {
            for &d in &ds {
                let k = ks[fi % ks.len()];
                for &accel in &accels {
                    let sp = split[count % split.len()];
                    let sd = shrink[count % shrink.len()];
                    let reg = regress[count % regress.len()];
                    let z = zparams[count % zparams.len()];
                    count += 1;
                    let params = ZDICT_fastCover_params_t {
                        k, d, f, steps: 0, nbThreads: 1, splitPoint: sp, accel,
                        shrinkDict: sd, shrinkDictMaxRegression: reg, zParams: z,
                    };
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                    let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                    let ctx = format!("trainFromBuffer_fastCover k={k} d={d} f={f} accel={accel} sp={sp}");
                    api.assert_ret_eq(&ctx, cn, rn);
                    if !api.c_is_err(cn) {
                        api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }

        // explicit full k sweep at default f/accel
        for &k in &ks {
            let params = ZDICT_fastCover_params_t {
                k, d: 8, f: 20, steps: 0, nbThreads: 1, splitPoint: 1.0, accel: 1,
                shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
            };
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
            let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
            let ctx = format!("trainFromBuffer_fastCover k-sweep k={k}");
            api.assert_ret_eq(&ctx, cn, rn);
            if !api.c_is_err(cn) {
                api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                let s0 = &corpus.buf[..corpus.sizes[0]];
                verify_dict_roundtrip(&ctx, &cbuf[..cn], s0, 3);
            }
        }
    }
}

/// ZDICT_optimizeTrainFromBuffer_fastCover: bounded steps/f/accel sweep with
/// out-param comparison.
#[test]
fn optimize_train_fastcover() {
    unsafe {
        let (ct, rt) = both::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0005);
        let corpus = build_corpus(128, 256, &mut rng, ALL_SHAPES);
        let cap = 8192usize;

        for &steps in &[0u32, 1, 4] {
            for &f in &[6u32, 15, 20] {
                for &accel in &[0u32, 1, 2] {
                    for &sp in &[0.0f64, 0.75] {
                        let base = ZDICT_fastCover_params_t {
                            k: 200, d: 8, f, steps, nbThreads: 1, splitPoint: sp, accel,
                            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
                        };
                        let mut cp = base;
                        let mut rp = base;
                        let mut cbuf = vec![0u8; cap];
                        let mut rbuf = vec![0u8; cap];
                        let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
                        let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
                        let ctx = format!("optimizeTrain_fastCover steps={steps} f={f} accel={accel} sp={sp}");
                        api.assert_ret_eq(&ctx, cn, rn);
                        if !api.c_is_err(cn) {
                            api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                            assert_eq!(cp, rp, "{ctx}: selected params differ");
                        }
                    }
                }
            }
        }
    }
}

/// ZDICT_trainFromBuffer_legacy: selectivityLevel + zParams sweep.
#[test]
fn train_from_buffer_legacy() {
    unsafe {
        let (ct, rt) = both::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0006);
        let corpus = build_corpus(256, 512, &mut rng, ALL_SHAPES);

        for &sel in &[0u32, 1, 9, 20] {
            for &cap in &[256usize, 1024, 4096, 16384, 112640] {
                for &z in &[zp(0, 0, 0), zp(3, 0, 12345), zp(9, 0, 1), zp(19, 0, u32::MAX)] {
                    let params = ZDICT_legacy_params_t { selectivityLevel: sel, zParams: z };
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                    let rn = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                    let ctx = format!("trainFromBuffer_legacy sel={sel} cap={cap} lvl={}", z.compressionLevel);
                    api.assert_ret_eq(&ctx, cn, rn);
                    if !api.c_is_err(cn) {
                        api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }
    }
}

/// ZDICT_finalizeDictionary + ZDICT_addEntropyTablesFromBuffer over custom
/// dictionary content of several sizes.
#[test]
fn finalize_and_add_entropy() {
    unsafe {
        let (cf, rf) = both::<FnFinalize>("ZDICT_finalizeDictionary");
        let (ca, ra) = both::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer");
        let api = DictApi::new();
        let mut rng = Rng::new(0xD1C7_0007);
        let corpus = build_corpus(256, 256, &mut rng, ALL_SHAPES);

        for &content_size in &[0usize, 1, 256, 4096] {
            // custom dict content: use text so it's meaningful
            let content = gen(Shape::Text, content_size, &mut rng);
            for &cap in &[256usize, 1024, 4096, 16384] {
                for &z in &[zp(0, 0, 0), zp(3, 0, 12345), zp(9, 0, 1), zp(19, 0, u32::MAX)] {
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = cf(cbuf.as_mut_ptr() as *mut c_void, cap,
                                content.as_ptr() as *const c_void, content.len(),
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z);
                    let rn = rf(rbuf.as_mut_ptr() as *mut c_void, cap,
                                content.as_ptr() as *const c_void, content.len(),
                                corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z);
                    let ctx = format!("finalizeDictionary content={content_size} cap={cap} lvl={}", z.compressionLevel);
                    api.assert_ret_eq(&ctx, cn, rn);
                    if !api.c_is_err(cn) {
                        api.assert_dict_meta(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }

        // ZDICT_addEntropyTablesFromBuffer: dictBuffer holds content of
        // dictContentSize, cap is the total buffer, result appended.
        for &content_size in &[256usize, 1024, 4096] {
            for &cap in &[content_size + 512, content_size + 4096, 16384] {
                let content = gen(Shape::Text, content_size, &mut rng);
                let mut cbuf = vec![0u8; cap];
                let mut rbuf = vec![0u8; cap];
                cbuf[..content.len()].copy_from_slice(&content);
                rbuf[..content.len()].copy_from_slice(&content);
                let cn = ca(cbuf.as_mut_ptr() as *mut c_void, content_size, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                let rn = ra(rbuf.as_mut_ptr() as *mut c_void, content_size, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
                let ctx = format!("addEntropyTables content={content_size} cap={cap}");
                api.assert_ret_eq(&ctx, cn, rn);
                if !api.c_is_err(cn) {
                    assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                }
            }
        }
    }
}

/// COVER standalone helpers: COVER_sum, COVER_computeEpochs — pure functions,
/// swept with many random inputs.
#[test]
fn cover_pure_helpers() {
    unsafe {
        let (cs, rs_) = both::<FnCoverSum>("COVER_sum");
        let (cce, rce) = both::<FnComputeEpochs>("COVER_computeEpochs");
        let (cw, rw) = both::<FnWarn>("COVER_warnOnSmallCorpus");
        let mut rng = Rng::new(0xD1C7_0008);

        // COVER_sum over random size arrays
        for _ in 0..2000 {
            let n = rng.below(64);
            let sizes: Vec<size_t> = (0..n).map(|_| rng.below(1_000_000)).collect();
            let sp = if sizes.is_empty() { std::ptr::null() } else { sizes.as_ptr() };
            assert_eq!(cs(sp, n as c_uint), rs_(sp, n as c_uint), "COVER_sum n={n}");
        }

        // COVER_computeEpochs — pure arithmetic; sweep incl. edge values
        let vals = [1u32, 2, 4, 8, 10, 16, 32, 50, 64, 100, 256, 1000, 4096, 16384, 100000, u32::MAX / 8];
        for &maxd in &vals {
            for &nbd in &vals {
                for &k in &[1u32, 8, 16, 50, 64, 256] {
                    for &p in &[1u32, 4, 40] {
                        assert_eq!(
                            cce(maxd, nbd, k, p), rce(maxd, nbd, k, p),
                            "COVER_computeEpochs({maxd},{nbd},{k},{p})"
                        );
                    }
                }
            }
        }
        // random computeEpochs (guard against div-by-zero: k>=1, p>=1)
        for _ in 0..3000 {
            let maxd = rng.next_u32() >> 4;
            let nbd = rng.next_u32() >> 4;
            let k = 1 + (rng.next_u32() % 4096);
            let p = 1 + (rng.next_u32() % 64);
            assert_eq!(cce(maxd, nbd, k, p), rce(maxd, nbd, k, p),
                       "COVER_computeEpochs rand({maxd},{nbd},{k},{p})");
        }

        // COVER_warnOnSmallCorpus prints to stderr; with displayLevel 0 it is
        // silent. Just exercise both to confirm no divergence/crash.
        for &(m, n) in &[(1000usize, 100usize), (1000, 20000), (256, 256), (100000, 500000)] {
            cw(m, n, 0);
            rw(m, n, 0);
        }
    }
}

/// COVER dictSelection helpers + COVER_best lifecycle. Uses the COVER_best_t
/// and COVER_dictSelection_t struct layouts directly.
#[test]
fn cover_selection_and_best() {
    unsafe {
        let (cse, rse) = both::<FnDictSelErr>("COVER_dictSelectionError");
        let (cie, rie) = both::<FnDictSelIsErr>("COVER_dictSelectionIsError");
        let (_cfr, _rfr) = both::<FnDictSelFree>("COVER_dictSelectionFree");
        let (cbi, rbi) = both::<FnBestVoid>("COVER_best_init");
        let (cbs, rbs) = both::<FnBestVoid>("COVER_best_start");
        let (cbw, rbw) = both::<FnBestVoid>("COVER_best_wait");
        let (cbd, rbd) = both::<FnBestVoid>("COVER_best_destroy");
        let (cbf, rbf) = both::<FnBestFinish>("COVER_best_finish");
        let e = Err2::new();

        // COVER_dictSelectionError / IsError over many error codes
        for code in [0usize, 1, 10, 20, 30, 40, 64, 70, 0usize.wrapping_sub(1), 0usize.wrapping_sub(64)] {
            let cs = cse(code);
            let rs2 = rse(code);
            assert_eq!(cs.totalCompressedSize, rs2.totalCompressedSize,
                       "dictSelectionError({code}) totalCompressedSize");
            assert_eq!(cs.dictSize, rs2.dictSize, "dictSelectionError({code}) dictSize");
            // IsError agrees
            assert_eq!(cie(cs), rie(rs2), "dictSelectionIsError({code})");
        }

        // A non-error selection (dictContent null, csize small) should not be
        // an error, and both libs must agree.
        let ok_sel = COVER_dictSelection_t { dictContent: std::ptr::null_mut(), dictSize: 100, totalCompressedSize: 42 };
        assert_eq!(cie(ok_sel), rie(ok_sel), "dictSelectionIsError(ok)");

        // COVER_best lifecycle: init, start, finish (with a heap dict), wait,
        // destroy. finish() copies the dict into best->dict via malloc; destroy
        // frees it. We drive both libraries in lockstep and compare the visible
        // state (dictSize, compressedSize) after finish.
        let base_params = ZDICT_cover_params_t {
            k: 64, d: 8, steps: 0, nbThreads: 1, splitPoint: 1.0,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };
        let mut cbest: COVER_best_t = std::mem::zeroed();
        let mut rbest: COVER_best_t = std::mem::zeroed();
        cbi(&mut cbest);
        rbi(&mut rbest);
        // after init, compressedSize is (size_t)-1
        assert_eq!(cbest.compressedSize, rbest.compressedSize, "best_init compressedSize");
        assert_eq!(cbest.dictSize, rbest.dictSize, "best_init dictSize");

        cbs(&mut cbest);
        rbs(&mut rbest);
        assert_eq!(cbest.liveJobs, rbest.liveJobs, "best_start liveJobs");

        // Provide a dict content buffer to finish(). finish copies it, so a
        // stack buffer is fine.
        let dict_content = gen(Shape::Text, 300, &mut Rng::new(0x1234));
        let sel = COVER_dictSelection_t {
            dictContent: dict_content.as_ptr() as *mut u8,
            dictSize: dict_content.len(),
            totalCompressedSize: 1000,
        };
        cbf(&mut cbest, base_params, sel);
        rbf(&mut rbest, base_params, sel);
        assert_eq!(cbest.liveJobs, rbest.liveJobs, "best_finish liveJobs");
        assert_eq!(cbest.dictSize, rbest.dictSize, "best_finish dictSize");
        assert_eq!(cbest.compressedSize, rbest.compressedSize, "best_finish compressedSize");
        // dict bytes copied identically
        if !cbest.dict.is_null() && !rbest.dict.is_null() {
            let cd = std::slice::from_raw_parts(cbest.dict as *const u8, cbest.dictSize);
            let rd = std::slice::from_raw_parts(rbest.dict as *const u8, rbest.dictSize);
            assert_bytes_eq("best_finish dict content", cd, rd);
        }

        cbw(&mut cbest);
        rbw(&mut rbest);
        cbd(&mut cbest);
        rbd(&mut rbest);
        let _ = &e;
    }
}

type FnCheckTotal = unsafe extern "C" fn(
    ZDICT_cover_params_t, *const size_t, *const u8, *mut size_t, size_t, size_t, *mut u8, size_t,
) -> size_t;
type FnSelectDict = unsafe extern "C" fn(
    *mut u8, size_t, size_t, *const u8, *const size_t, c_uint, size_t, size_t,
    ZDICT_cover_params_t, *mut size_t, size_t,
) -> COVER_dictSelection_t;

/// COVER_checkTotalCompressedSize + COVER_selectDict. Both take an `offsets`
/// array (cumulative byte offset of each sample) plus a candidate dictionary,
/// and internally compress the training samples. We build a self-consistent
/// (samples, sizes, offsets) triple and a small valid dictionary and assert
/// the returned compressed size / selection agree between the two libraries.
#[test]
fn cover_check_and_select() {
    unsafe {
        let (cct, rct) = both::<FnCheckTotal>("COVER_checkTotalCompressedSize");
        let (csd, rsd) = both::<FnSelectDict>("COVER_selectDict");
        let (cfr, rfr) = both::<FnDictSelFree>("COVER_dictSelectionFree");
        let mut rng = Rng::new(0xD1C7_000A);

        // Build a corpus and a matching offsets array.
        let corpus = build_corpus(64, 512, &mut rng, ALL_SHAPES);
        let nb = corpus.sizes.len();
        let mut offsets = vec![0usize; nb + 1];
        for i in 0..nb {
            offsets[i + 1] = offsets[i] + corpus.sizes[i];
        }

        // A valid dictionary to check, produced by the (shared) trainer so it
        // is a real zstd dictionary. Use the C trainer; both libs read it.
        let (ct_cover, _) = both::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let dcap = 4096usize;
        let mut dictbuf = vec![0u8; dcap];
        let train_params = ZDICT_cover_params_t {
            k: 200, d: 8, steps: 0, nbThreads: 1, splitPoint: 1.0,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };
        let dsz = ct_cover(dictbuf.as_mut_ptr() as *mut c_void, dcap,
                           corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), train_params);
        {
            let e = Err2::new();
            assert!(!e.c.is_err(dsz), "setup: cover training failed");
        }

        for &lvl in &[0i32, 3, 9] {
            for &sp in &[0.5f64, 1.0] {
                let params = ZDICT_cover_params_t {
                    k: 200, d: 8, steps: 0, nbThreads: 1, splitPoint: sp,
                    shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(lvl, 0, 0),
                };
                // COVER_checkTotalCompressedSize: dict is read-only here.
                let nb_train = if sp < 1.0 { (nb as f64 * sp) as size_t } else { nb };
                let mut dc = dictbuf[..dsz].to_vec();
                let mut dr = dictbuf[..dsz].to_vec();
                let mut off_c = offsets.clone();
                let mut off_r = offsets.clone();
                let a = cct(params, corpus.sizes_ptr(), corpus.buf.as_ptr(),
                            off_c.as_mut_ptr(), nb_train, nb, dc.as_mut_ptr(), dsz);
                let b = rct(params, corpus.sizes_ptr(), corpus.buf.as_ptr(),
                            off_r.as_mut_ptr(), nb_train, nb, dr.as_mut_ptr(), dsz);
                assert_eq!(a, b, "COVER_checkTotalCompressedSize lvl={lvl} sp={sp}");

                // COVER_selectDict: customDictContent must be a heap buffer of
                // capacity dictBufferCapacity holding dictContentSize bytes.
                for &shrink in &[0u32, 1] {
                    for &reg in &[0u32, 5] {
                        let mut sp_params = params;
                        sp_params.shrinkDict = shrink;
                        sp_params.shrinkDictMaxRegression = reg;
                        // content = the trained dict content region (bytes).
                        let content_size = dsz;
                        let mut cust_c = vec![0u8; dcap];
                        let mut cust_r = vec![0u8; dcap];
                        cust_c[..content_size].copy_from_slice(&dictbuf[..content_size]);
                        cust_r[..content_size].copy_from_slice(&dictbuf[..content_size]);
                        let mut oc = offsets.clone();
                        let mut or = offsets.clone();
                        let sel_c = csd(cust_c.as_mut_ptr(), dcap, content_size,
                                        corpus.buf.as_ptr(), corpus.sizes_ptr(), corpus.nb(),
                                        nb, nb, sp_params, oc.as_mut_ptr(), 0);
                        let sel_r = rsd(cust_r.as_mut_ptr(), dcap, content_size,
                                        corpus.buf.as_ptr(), corpus.sizes_ptr(), corpus.nb(),
                                        nb, nb, sp_params, or.as_mut_ptr(), 0);
                        let ctx = format!("COVER_selectDict lvl={lvl} sp={sp} shrink={shrink} reg={reg}");
                        assert_eq!(sel_c.dictSize, sel_r.dictSize, "{ctx}: dictSize");
                        assert_eq!(sel_c.totalCompressedSize, sel_r.totalCompressedSize,
                                   "{ctx}: totalCompressedSize");
                        assert_eq!(cie_isnull(sel_c), cie_isnull(sel_r), "{ctx}: null-ness");
                        if !sel_c.dictContent.is_null() && !sel_r.dictContent.is_null()
                            && sel_c.dictSize == sel_r.dictSize {
                            let a = std::slice::from_raw_parts(sel_c.dictContent, sel_c.dictSize);
                            let b = std::slice::from_raw_parts(sel_r.dictContent, sel_r.dictSize);
                            assert_bytes_eq(&format!("{ctx}: dict content"), a, b);
                        }
                        // free the selection buffers via each library
                        cfr(sel_c);
                        rfr(sel_r);
                    }
                }
            }
        }
    }
}

fn cie_isnull(s: COVER_dictSelection_t) -> bool {
    s.dictContent.is_null()
}

/// divsufsort / divbwt: suffix-array and BWT primitives. Compare the full
/// output arrays byte/int-for-int over many shapes and lengths.
#[test]
fn divsufsort_and_divbwt() {
    unsafe {
        let (cds, rds) = both::<FnDivsufsort>("divsufsort");
        let (cdb, rdb) = both::<FnDivbwt>("divbwt");
        let mut rng = Rng::new(0xD1C7_0009);

        let lens = [0usize, 1, 2, 3, 4, 7, 8, 15, 16, 32, 64, 100, 256, 512, 1000, 4096];
        for &shape in ALL_SHAPES {
            for &len in &lens {
                let t = gen(shape, len, &mut rng);
                let n = t.len() as c_int;
                let tp = t.as_ptr();

                // divsufsort: SA has n entries
                let mut sa_c = vec![0i32; t.len().max(1)];
                let mut sa_r = vec![0i32; t.len().max(1)];
                let rc = cds(tp, sa_c.as_mut_ptr(), n, 0);
                let rr = rds(tp, sa_r.as_mut_ptr(), n, 0);
                let ctx = format!("divsufsort shape={shape:?} len={len}");
                assert_eq!(rc, rr, "{ctx}: return code");
                assert_eq!(sa_c, sa_r, "{ctx}: suffix array");

                // divbwt: U has n bytes, A is workspace of n ints, indexes small
                let mut u_c = vec![0u8; t.len().max(1)];
                let mut u_r = vec![0u8; t.len().max(1)];
                let mut a_c = vec![0i32; t.len().max(1)];
                let mut a_r = vec![0i32; t.len().max(1)];
                let bc = cdb(tp, u_c.as_mut_ptr(), a_c.as_mut_ptr(), n,
                             std::ptr::null_mut(), std::ptr::null_mut(), 0);
                let br = rdb(tp, u_r.as_mut_ptr(), a_r.as_mut_ptr(), n,
                             std::ptr::null_mut(), std::ptr::null_mut(), 0);
                let ctx2 = format!("divbwt shape={shape:?} len={len}");
                assert_eq!(bc, br, "{ctx2}: return code");
                assert_eq!(u_c, u_r, "{ctx2}: BWT output");
            }
        }

        // error / edge inputs shared by both
        let mut sa = vec![0i32; 4];
        assert_eq!(cds(std::ptr::null(), sa.as_mut_ptr(), 4, 0),
                   rds(std::ptr::null(), sa.as_mut_ptr(), 4, 0), "divsufsort null T");
        let t = gen(Shape::Text, 8, &mut rng);
        assert_eq!(cds(t.as_ptr(), std::ptr::null_mut(), 8, 0),
                   rds(t.as_ptr(), std::ptr::null_mut(), 8, 0), "divsufsort null SA");
        assert_eq!(cds(t.as_ptr(), sa.as_mut_ptr(), -1, 0),
                   rds(t.as_ptr(), sa.as_mut_ptr(), -1, 0), "divsufsort negative n");
    }
}
