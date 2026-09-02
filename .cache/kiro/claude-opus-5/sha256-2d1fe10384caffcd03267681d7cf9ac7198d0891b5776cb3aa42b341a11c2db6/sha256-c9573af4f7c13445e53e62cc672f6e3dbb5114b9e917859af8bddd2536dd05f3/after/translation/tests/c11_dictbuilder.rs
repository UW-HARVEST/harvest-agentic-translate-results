//! Phase C row 11: differential tests for the ZDICT dictionary-builder API
//! (error paths). Every documented failure mode is constructed for both the C
//! build and the Rust translation, and the resulting error must be IDENTICAL:
//! the `ZDICT_isError` boolean AND the `ZDICT_getErrorName` string must match
//! (and, when both succeed, the returned size must match).
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
type FnGetDictID = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetDictHeaderSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnZdIsError = unsafe extern "C" fn(size_t) -> c_uint;
type FnZdErrName = unsafe extern "C" fn(size_t) -> *const std::os::raw::c_char;

// ------------------------------------------------------------------- helpers

const ZDICT_DICTSIZE_MIN: size_t = 256;

struct ZdErr {
    is_err: (
        libloading::Symbol<'static, FnZdIsError>,
        libloading::Symbol<'static, FnZdIsError>,
    ),
    err_name: (
        libloading::Symbol<'static, FnZdErrName>,
        libloading::Symbol<'static, FnZdErrName>,
    ),
}
impl ZdErr {
    unsafe fn new() -> Self {
        ZdErr {
            is_err: both::<FnZdIsError>("ZDICT_isError"),
            err_name: both::<FnZdErrName>("ZDICT_getErrorName"),
        }
    }
    unsafe fn c_is_err(&self, r: size_t) -> bool {
        (self.is_err.0)(r) != 0
    }
    /// Assert identical error disposition: same isError bool AND same error
    /// name string; when both OK, same size.
    unsafe fn assert_eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let ce = (self.is_err.0)(cr) != 0;
        let re = (self.is_err.1)(rr) != 0;
        let cn = cstr((self.err_name.0)(cr));
        let rn = cstr((self.err_name.1)(rr));
        assert_eq!(ce, re, "{ctx}: isError bool C={ce} RS={re} (C name={cn:?} RS name={rn:?}, raw C={cr:#x} RS={rr:#x})");
        assert_eq!(cn, rn, "{ctx}: error name C={cn:?} RS={rn:?}");
        if !ce {
            assert_eq!(cr, rr, "{ctx}: OK size differs C={cr} RS={rr}");
        }
    }
}

fn zp(compressionLevel: c_int, notificationLevel: c_uint, dictID: c_uint) -> ZDICT_params_t {
    ZDICT_params_t { compressionLevel, notificationLevel, dictID }
}

/// Concatenated corpus + sizes.
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

fn build_corpus(nb: usize, sample_len: usize, rng: &mut Rng) -> Corpus {
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(nb);
    let backbone = gen(Shape::Text, 96, rng);
    for i in 0..nb {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let mut v = gen(shape, sample_len, rng);
        if !v.is_empty() && !backbone.is_empty() {
            let n = backbone.len().min(v.len());
            v[..n].copy_from_slice(&backbone[..n]);
        }
        sizes.push(v.len());
        buf.extend_from_slice(&v);
    }
    Corpus { buf, sizes }
}

// ============================================================================
// Tests
// ============================================================================

/// Simple-API error paths: tiny/empty capacity, zero samples, all-zero sizes,
/// too-small corpus, single tiny sample, NULL samplesBuffer.
#[test]
fn simple_api_errors() {
    unsafe {
        let (ct, rt) = both::<FnTrain>("ZDICT_trainFromBuffer");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0001);

        let corpus = build_corpus(64, 256, &mut rng);

        // dictBufferCapacity below ZDICT_DICTSIZE_MIN and zero
        for &cap in &[0usize, 1, 2, 8, 64, 100, 128, 200, 255, ZDICT_DICTSIZE_MIN - 1] {
            let mut cbuf = vec![0u8; cap.max(1)];
            let mut rbuf = vec![0u8; cap.max(1)];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
            e.assert_eq(&format!("trainFromBuffer tiny cap={cap}"), cr, rr);
        }

        let cap = 4096usize;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];

        // nbSamples == 0
        e.assert_eq(
            "trainFromBuffer nbSamples=0",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0),
        );

        // all-zero sample sizes
        let zeros = vec![0usize; 64];
        e.assert_eq(
            "trainFromBuffer all-zero sizes",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), zeros.as_ptr(), 64),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), zeros.as_ptr(), 64),
        );

        // total corpus smaller than minimum (a few tiny samples)
        let tiny = build_corpus(3, 8, &mut rng);
        e.assert_eq(
            "trainFromBuffer tiny corpus",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, tiny.buf_ptr(), tiny.sizes_ptr(), tiny.nb()),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, tiny.buf_ptr(), tiny.sizes_ptr(), tiny.nb()),
        );

        // single tiny sample
        let one = build_corpus(1, 4, &mut rng);
        e.assert_eq(
            "trainFromBuffer single tiny",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, one.buf_ptr(), one.sizes_ptr(), 1),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, one.buf_ptr(), one.sizes_ptr(), 1),
        );

        // samplesBuffer NULL with ZERO samples: the reference C library does
        // NOT defend against NULL samplesBuffer when nbSamples>0 (it reads the
        // buffer unconditionally and segfaults — genuine C-side UB, so that is
        // not a differential case). With nbSamples==0 both must agree.
        e.assert_eq(
            "trainFromBuffer NULL samplesBuffer, 0 samples",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), std::ptr::null(), 0),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), std::ptr::null(), 0),
        );
    }
}

/// COVER error paths: k out of range, d not in {6,8}, splitPoint out of (0,1],
/// nbThreads huge, steps huge, plus the corpus-shape errors.
#[test]
fn cover_errors() {
    unsafe {
        let (ct, rt) = both::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0002);
        let corpus = build_corpus(64, 256, &mut rng);
        let cap = 4096usize;

        let base = ZDICT_cover_params_t {
            k: 200, d: 8, steps: 0, nbThreads: 1, splitPoint: 1.0,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };

        // k out of range
        for &k in &[0u32, 1, 5, 2, 3, 4, 1_000_000, u32::MAX] {
            let mut p = base;
            p.k = k;
            run_cover(&e, &ct, &rt, &corpus, cap, p, &format!("cover k={k}"));
        }
        // d not in {6,8}
        for &d in &[0u32, 1, 7, 9, 16, 999] {
            let mut p = base;
            p.d = d;
            run_cover(&e, &ct, &rt, &corpus, cap, p, &format!("cover d={d}"));
        }
        // splitPoint out of (0,1]: 0.0 is default(1.0); negative, >1, NaN
        for &sp in &[0.0f64, -0.5, -1.0, 1.5, 2.0, 100.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut p = base;
            p.splitPoint = sp;
            run_cover(&e, &ct, &rt, &corpus, cap, p, &format!("cover splitPoint={sp}"));
        }
        // nbThreads huge
        for &nt in &[0u32, 2, 16, 1_000_000, u32::MAX] {
            let mut p = base;
            p.nbThreads = nt;
            run_cover(&e, &ct, &rt, &corpus, cap, p, &format!("cover nbThreads={nt}"));
        }
        // steps huge (only relevant for optimize; here just ensure parity)
        for &st in &[u32::MAX, 1_000_000] {
            let mut p = base;
            p.steps = st;
            run_cover(&e, &ct, &rt, &corpus, cap, p, &format!("cover steps={st}"));
        }
        // tiny capacity
        for &c in &[0usize, 1, 255] {
            let mut cbuf = vec![0u8; c.max(1)];
            let mut rbuf = vec![0u8; c.max(1)];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), base);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), base);
            e.assert_eq(&format!("cover tiny cap={c}"), cr, rr);
        }
        // nbSamples 0 / NULL buffer / tiny corpus
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        e.assert_eq("cover nbSamples=0",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0, base),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0, base));
    }
}

fn run_cover(
    e: &ZdErr,
    ct: &libloading::Symbol<'static, FnTrainCover>,
    rt: &libloading::Symbol<'static, FnTrainCover>,
    corpus: &Corpus,
    cap: usize,
    p: ZDICT_cover_params_t,
    ctx: &str,
) {
    unsafe {
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p);
        let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p);
        e.assert_eq(ctx, cr, rr);
        if !e.c_is_err(cr) {
            assert_bytes_eq(&format!("{ctx}: bytes"), &cbuf[..cr], &rbuf[..rr]);
        }
    }
}

/// optimizeTrainFromBuffer_cover error paths (bad d/k/steps).
#[test]
fn optimize_cover_errors() {
    unsafe {
        let (ct, rt) = both::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0003);
        let corpus = build_corpus(48, 128, &mut rng);
        let cap = 4096usize;
        let base = ZDICT_cover_params_t {
            k: 200, d: 8, steps: 1, nbThreads: 1, splitPoint: 1.0,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };
        for &d in &[1u32, 7, 9, 16] {
            let mut p = base;
            p.d = d;
            let mut cp = p;
            let mut rp = p;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
            e.assert_eq(&format!("optimize_cover d={d}"), cr, rr);
        }
        // tiny cap
        for &c in &[0usize, 255] {
            let mut cp = base;
            let mut rp = base;
            let mut cbuf = vec![0u8; c.max(1)];
            let mut rbuf = vec![0u8; c.max(1)];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
            e.assert_eq(&format!("optimize_cover tiny cap={c}"), cr, rr);
        }
    }
}

/// fastCover error paths: f out of range, accel out of range, plus k/d/split.
#[test]
fn fastcover_errors() {
    unsafe {
        let (ct, rt) = both::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0004);
        let corpus = build_corpus(64, 256, &mut rng);
        let cap = 4096usize;
        let base = ZDICT_fastCover_params_t {
            k: 200, d: 8, f: 20, steps: 0, nbThreads: 1, splitPoint: 1.0, accel: 1,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };

        // f out of range: 0 is default(20); 31 boundary; 32,99 invalid
        for &f in &[0u32, 1, 31, 32, 99, 1000, u32::MAX] {
            let mut p = base;
            p.f = f;
            run_fast(&e, &ct, &rt, &corpus, cap, p, &format!("fast f={f}"));
        }
        // accel out of range: 0 is default(1); 11..16, 17, 999 invalid
        for &accel in &[0u32, 1, 2, 10, 11, 12, 15, 16, 17, 999, u32::MAX] {
            let mut p = base;
            p.accel = accel;
            run_fast(&e, &ct, &rt, &corpus, cap, p, &format!("fast accel={accel}"));
        }
        // k out of range
        for &k in &[0u32, 1, 5, u32::MAX] {
            let mut p = base;
            p.k = k;
            run_fast(&e, &ct, &rt, &corpus, cap, p, &format!("fast k={k}"));
        }
        // d not in {6,8}
        for &d in &[0u32, 1, 7, 9, 16, 999] {
            let mut p = base;
            p.d = d;
            run_fast(&e, &ct, &rt, &corpus, cap, p, &format!("fast d={d}"));
        }
        // splitPoint out of (0,1]
        for &sp in &[-0.5f64, 1.5, f64::NAN, f64::INFINITY] {
            let mut p = base;
            p.splitPoint = sp;
            run_fast(&e, &ct, &rt, &corpus, cap, p, &format!("fast sp={sp}"));
        }
        // tiny cap
        for &c in &[0usize, 255] {
            let mut cbuf = vec![0u8; c.max(1)];
            let mut rbuf = vec![0u8; c.max(1)];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), base);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, c,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), base);
            e.assert_eq(&format!("fast tiny cap={c}"), cr, rr);
        }
    }
}

fn run_fast(
    e: &ZdErr,
    ct: &libloading::Symbol<'static, FnTrainFast>,
    rt: &libloading::Symbol<'static, FnTrainFast>,
    corpus: &Corpus,
    cap: usize,
    p: ZDICT_fastCover_params_t,
    ctx: &str,
) {
    unsafe {
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p);
        let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                    corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p);
        e.assert_eq(ctx, cr, rr);
        if !e.c_is_err(cr) {
            assert_bytes_eq(&format!("{ctx}: bytes"), &cbuf[..cr], &rbuf[..rr]);
        }
    }
}

/// optimizeTrainFromBuffer_fastCover error paths (bad f/accel/d).
#[test]
fn optimize_fastcover_errors() {
    unsafe {
        let (ct, rt) = both::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0005);
        let corpus = build_corpus(48, 128, &mut rng);
        let cap = 4096usize;
        let base = ZDICT_fastCover_params_t {
            k: 200, d: 8, f: 20, steps: 1, nbThreads: 1, splitPoint: 1.0, accel: 1,
            shrinkDict: 0, shrinkDictMaxRegression: 0, zParams: zp(3, 0, 0),
        };
        for &f in &[1u32, 32, 99] {
            let mut p = base;
            p.f = f;
            let mut cp = p;
            let mut rp = p;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
            e.assert_eq(&format!("optimize_fast f={f}"), cr, rr);
        }
        for &accel in &[11u32, 16, 17, 999] {
            let mut p = base;
            p.accel = accel;
            let mut cp = p;
            let mut rp = p;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut cp);
            let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut rp);
            e.assert_eq(&format!("optimize_fast accel={accel}"), cr, rr);
        }
    }
}

/// legacy error paths: absurd selectivityLevel, tiny cap, bad corpus.
#[test]
fn legacy_errors() {
    unsafe {
        let (ct, rt) = both::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0006);
        let corpus = build_corpus(64, 256, &mut rng);

        for &sel in &[0u32, 1, 9, 20, 100, 1000, 1_000_000, u32::MAX] {
            let params = ZDICT_legacy_params_t { selectivityLevel: sel, zParams: zp(3, 0, 0) };
            for &cap in &[0usize, 255, 4096] {
                let mut cbuf = vec![0u8; cap.max(1)];
                let mut rbuf = vec![0u8; cap.max(1)];
                let cr = ct(cbuf.as_mut_ptr() as *mut c_void, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                let rr = rt(rbuf.as_mut_ptr() as *mut c_void, cap,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), params);
                e.assert_eq(&format!("legacy sel={sel} cap={cap}"), cr, rr);
                if !e.c_is_err(cr) {
                    assert_bytes_eq(&format!("legacy sel={sel} cap={cap} bytes"), &cbuf[..cr], &rbuf[..rr]);
                }
            }
        }
        // nbSamples 0 and NULL buffer
        let cap = 4096usize;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let params = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zp(3, 0, 0) };
        e.assert_eq("legacy nbSamples=0",
            ct(cbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0, params),
            rt(rbuf.as_mut_ptr() as *mut c_void, cap, corpus.buf_ptr(), corpus.sizes_ptr(), 0, params));
    }
}

/// ZDICT_finalizeDictionary error paths: cap too small, dictContentSize 0,
/// NULL content, bad corpus.
#[test]
fn finalize_errors() {
    unsafe {
        let (cf, rf) = both::<FnFinalize>("ZDICT_finalizeDictionary");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0007);
        let corpus = build_corpus(64, 256, &mut rng);
        let content = gen(Shape::Text, 512, &mut rng);
        let z = zp(3, 0, 0);

        // maxDictSize too small (< max(dictContentSize, ZDICT_DICTSIZE_MIN))
        for &cap in &[0usize, 1, 100, 128, 255, ZDICT_DICTSIZE_MIN - 1] {
            let mut cbuf = vec![0u8; (cap + content.len()).max(1)];
            let mut rbuf = vec![0u8; (cap + content.len()).max(1)];
            let cr = cf(cbuf.as_mut_ptr() as *mut c_void, cap,
                        content.as_ptr() as *const c_void, content.len(),
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z);
            let rr = rf(rbuf.as_mut_ptr() as *mut c_void, cap,
                        content.as_ptr() as *const c_void, content.len(),
                        corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z);
            e.assert_eq(&format!("finalize cap-too-small cap={cap}"), cr, rr);
        }
        // maxDictSize smaller than dictContentSize (content bigger than cap)
        let big_content = gen(Shape::Text, 8192, &mut rng);
        let mut cbuf = vec![0u8; 8192];
        let mut rbuf = vec![0u8; 8192];
        e.assert_eq("finalize content>cap",
            cf(cbuf.as_mut_ptr() as *mut c_void, 1024,
               big_content.as_ptr() as *const c_void, big_content.len(),
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z),
            rf(rbuf.as_mut_ptr() as *mut c_void, 1024,
               big_content.as_ptr() as *const c_void, big_content.len(),
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z));

        // dictContentSize 0
        let cap = 4096usize;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        e.assert_eq("finalize content=0",
            cf(cbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, 0,
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z),
            rf(rbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, 0,
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z));

        // NULL content with nonzero dictContentSize is genuine C-side UB (the
        // reference library dereferences it and segfaults), so it is not a
        // differential case. NULL content with dictContentSize==0 is safe and
        // both libraries must agree.
        e.assert_eq("finalize NULL content, size=0",
            cf(cbuf.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0,
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z),
            rf(rbuf.as_mut_ptr() as *mut c_void, cap, std::ptr::null(), 0,
               corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), z));

        // nbSamples 0
        e.assert_eq("finalize nbSamples=0",
            cf(cbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, content.len(),
               corpus.buf_ptr(), corpus.sizes_ptr(), 0, z),
            rf(rbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, content.len(),
               corpus.buf_ptr(), corpus.sizes_ptr(), 0, z));

        // all-zero sizes
        let zeros = vec![0usize; 64];
        e.assert_eq("finalize all-zero sizes",
            cf(cbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, content.len(),
               corpus.buf_ptr(), zeros.as_ptr(), 64, z),
            rf(rbuf.as_mut_ptr() as *mut c_void, cap, content.as_ptr() as *const c_void, content.len(),
               corpus.buf_ptr(), zeros.as_ptr(), 64, z));
    }
}

/// ZDICT_getDictHeaderSize / ZDICT_getDictID on malformed dictionaries:
/// too-small buffers, wrong magic, and every single-byte mutation of a real
/// dictionary header (corrupted entropy tables).
#[test]
fn header_and_id_on_malformed() {
    unsafe {
        let (cid, rid) = both::<FnGetDictID>("ZDICT_getDictID");
        let (chs, rhs) = both::<FnGetDictHeaderSize>("ZDICT_getDictHeaderSize");
        let (ct, _) = both::<FnTrain>("ZDICT_trainFromBuffer");
        let e = ZdErr::new();
        let mut rng = Rng::new(0xE770_0008);

        // build a real dictionary to mutate
        let corpus = build_corpus(256, 512, &mut rng);
        let cap = 8192usize;
        let mut dbuf = vec![0u8; cap];
        let dsz = ct(dbuf.as_mut_ptr() as *mut c_void, cap,
                     corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb());
        assert!(!e.c_is_err(dsz), "setup: training a base dictionary failed");
        let real = dbuf[..dsz].to_vec();

        // getDictID / getDictHeaderSize on buffers too small
        for &n in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 16, 32] {
            let n = n.min(real.len());
            let p = real.as_ptr() as *const c_void;
            assert_eq!(cid(p, n), rid(p, n), "getDictID small n={n}");
            e.assert_eq(&format!("getDictHeaderSize small n={n}"), chs(p, n), rhs(p, n));
        }

        // wrong magic: mutate the first 4 bytes to various values
        for &magic in &[0u32, 0xFFFFFFFF, 0x12345678, 0xEC30A437u32.wrapping_add(1), 0xEC30A436] {
            let mut m = real.clone();
            m[..4].copy_from_slice(&magic.to_le_bytes());
            let p = m.as_ptr() as *const c_void;
            assert_eq!(cid(p, m.len()), rid(p, m.len()), "getDictID wrong magic {magic:#x}");
            e.assert_eq(&format!("getDictHeaderSize wrong magic {magic:#x}"),
                        chs(p, m.len()), rhs(p, m.len()));
        }

        // every single-byte mutation of the header region (bound the region so
        // the test stays fast; entropy tables live in the first ~256 bytes).
        let hdr_region = real.len().min(256);
        for i in 0..hdr_region {
            for delta in [1u8, 0x55, 0xAA, 0xFF] {
                let mut m = real.clone();
                m[i] = m[i].wrapping_add(delta);
                let p = m.as_ptr() as *const c_void;
                assert_eq!(cid(p, m.len()), rid(p, m.len()),
                           "getDictID mutate byte {i} +{delta}");
                e.assert_eq(&format!("getDictHeaderSize mutate byte {i} +{delta}"),
                            chs(p, m.len()), rhs(p, m.len()));
            }
        }

        // truncations of the real dictionary at many lengths
        for cut in 0..real.len().min(300) {
            let p = real.as_ptr() as *const c_void;
            assert_eq!(cid(p, cut), rid(p, cut), "getDictID truncate {cut}");
            e.assert_eq(&format!("getDictHeaderSize truncate {cut}"), chs(p, cut), rhs(p, cut));
        }

        // NULL buffer with size 0: both defend against this. NULL with a
        // nonzero size is genuine C-side UB (the reference library reads the
        // header without a NULL guard once dictSize>=8 and segfaults), so it is
        // not a differential case.
        assert_eq!(cid(std::ptr::null(), 0), rid(std::ptr::null(), 0), "getDictID null");
        e.assert_eq("getDictHeaderSize null size=0",
            chs(std::ptr::null(), 0), rhs(std::ptr::null(), 0));
    }
}

/// ZDICT_getErrorName over every int from -200 to 400: exhaustively covers
/// out-of-range enum values that map to no valid error variant. Also
/// cross-checks ZDICT_isError parity.
#[test]
fn error_name_full_range() {
    unsafe {
        let (cin, rin) = both::<FnZdIsError>("ZDICT_isError");
        let (cen, ren) = both::<FnZdErrName>("ZDICT_getErrorName");
        for code in -200i64..=400 {
            // ZDICT error codes are size_t; negative "codes" here map to the
            // (size_t)(-x) wrap that the error space uses.
            let c = code as isize as size_t;
            assert_eq!(cin(c), rin(c), "ZDICT_isError({code}) [{c:#x}]");
            let a = cstr(cen(c));
            let b = cstr(ren(c));
            assert_eq!(a, b, "ZDICT_getErrorName({code}) [{c:#x}] C={a:?} RS={b:?}");
        }
        // also sweep the low error-code space directly (0..=200) and the top of
        // the size_t range (where real error codes live: (size_t)-N).
        for n in 0u64..=200 {
            let c = 0usize.wrapping_sub(n as usize);
            assert_eq!(cin(c), rin(c), "ZDICT_isError(-{n}) [{c:#x}]");
            assert_eq!(cstr(cen(c)), cstr(ren(c)), "ZDICT_getErrorName(-{n}) [{c:#x}]");
        }
    }
}
