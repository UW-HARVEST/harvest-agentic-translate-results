//! Phase B: dictBuilder API — differential tests (C vs Rust).
//!
//! Every entry point of `zdict.h` (public + static/experimental) plus the
//! internal COVER_* helpers and divsufsort/divbwt are called with identical
//! inputs in both libraries; the produced dictionaries must be BYTE-identical
//! and the returned `size_t` codes must match exactly.
//!
//! Determinism notes:
//!   * `ZDICT_params_t.dictID == 0` makes zdict derive the ID from
//!     `XXH64(content)`, so it is in fact deterministic, but every
//!     byte-comparing configuration below pins a non-zero dictID anyway.
//!   * `notificationLevel` is always 0 so nothing is written to stderr and the
//!     `g_displayLevel` global cannot influence the result.
//!   * The libraries are built without `ZSTD_MULTITHREAD`, so `nbThreads > 1`
//!     still runs the POOL jobs synchronously => reproducible.
#![allow(non_snake_case, non_camel_case_types, dead_code)]

mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// ------------------------------------------------------------ error codes ---

const E_GENERIC: usize = 1;
const E_DICTIONARY_CORRUPTED: usize = 30;
const E_DICTIONARYCREATION_FAILED: usize = 34;
const E_PARAMETER_OUTOFBOUND: usize = 42;
const E_MEMORY_ALLOCATION: usize = 64;
const E_DSTSIZE_TOOSMALL: usize = 70;
const E_SRCSIZE_WRONG: usize = 72;

/// ZSTD_MAGIC_DICTIONARY = 0xEC30A437, little endian.
const DICT_MAGIC: [u8; 4] = [0x37, 0xA4, 0x30, 0xEC];

/// Build the `size_t` return value that corresponds to error `code`.
fn as_err(code: usize) -> usize {
    0usize.wrapping_sub(code)
}

/// Serializes the two multi-gigabyte configurations (fastCover f=31 and the
/// 2 GB `ZDICT_finalizeDictionary` content) so that a concurrent test cannot
/// change how much memory is available between the C and the Rust call, which
/// would make the comparison non-deterministic.
static HEAVY: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Fallible zeroed allocation: `vec![0u8; huge]` aborts the process when the
/// allocator says no, which would take the whole test binary down.
struct RawBuf {
    ptr: *mut u8,
    len: usize,
}

impl RawBuf {
    fn layout(len: usize) -> std::alloc::Layout {
        std::alloc::Layout::from_size_align(len, 16).unwrap()
    }
    fn zeroed(len: usize) -> Option<RawBuf> {
        assert!(len > 0);
        let ptr = unsafe { std::alloc::alloc_zeroed(Self::layout(len)) };
        if ptr.is_null() { None } else { Some(RawBuf { ptr, len }) }
    }
}

impl Drop for RawBuf {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, Self::layout(self.len)) }
    }
}

fn fmt_ret(r: usize) -> String {
    if is_error(r) {
        format!("error({})", err_code(r))
    } else {
        format!("{r}")
    }
}

// -------------------------------------------------------------- structs -----
//
// Layouts verified against c_src/src/include/zdict.h + dictBuilder/cover.h with
// a C program (gcc, x86_64):
//   ZDICT_params_t           size 12
//   ZDICT_cover_params_t     size 48  k0 d4 steps8 nbThreads12 splitPoint16
//                                     shrinkDict24 shrinkDictMaxRegression28
//                                     zParams32
//   ZDICT_fastCover_params_t size 56  k0 d4 f8 steps12 nbThreads16
//                                     splitPoint24 accel32 shrinkDict36
//                                     shrinkDictMaxRegression40 zParams44
//   ZDICT_legacy_params_t    size 16  selectivityLevel0 zParams4
//   COVER_best_t             size 88  liveJobs8 dict16 dictSize24
//                                     parameters32 compressedSize80
//                                     (ZSTD_pthread_mutex_t/cond_t == int,
//                                      because ZSTD_MULTITHREAD is not defined)
//   COVER_dictSelection_t    size 24, COVER_epoch_info_t size 8

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct ZDICT_params_t {
    compressionLevel: c_int,
    notificationLevel: c_uint,
    dictID: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct ZDICT_cover_params_t {
    k: c_uint,
    d: c_uint,
    steps: c_uint,
    nbThreads: c_uint,
    splitPoint: f64,
    shrinkDict: c_uint,
    shrinkDictMaxRegression: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct ZDICT_fastCover_params_t {
    k: c_uint,
    d: c_uint,
    f: c_uint,
    steps: c_uint,
    nbThreads: c_uint,
    splitPoint: f64,
    accel: c_uint,
    shrinkDict: c_uint,
    shrinkDictMaxRegression: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
struct ZDICT_legacy_params_t {
    selectivityLevel: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct COVER_epoch_info_t {
    num: u32,
    size: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct COVER_dictSelection_t {
    dictContent: *mut u8,
    dictSize: usize,
    totalCompressedSize: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct COVER_best_t {
    mutex: c_int,
    cond: c_int,
    liveJobs: usize,
    dict: *mut c_void,
    dictSize: usize,
    parameters: ZDICT_cover_params_t,
    compressedSize: usize,
}

const _: () = assert!(core::mem::size_of::<ZDICT_params_t>() == 12);
const _: () = assert!(core::mem::size_of::<ZDICT_cover_params_t>() == 48);
const _: () = assert!(core::mem::size_of::<ZDICT_fastCover_params_t>() == 56);
const _: () = assert!(core::mem::size_of::<ZDICT_legacy_params_t>() == 16);
const _: () = assert!(core::mem::size_of::<COVER_best_t>() == 88);
const _: () = assert!(core::mem::size_of::<COVER_dictSelection_t>() == 24);
const _: () = assert!(core::mem::size_of::<COVER_epoch_info_t>() == 8);

// ------------------------------------------------------------- fn types -----

type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, c_uint) -> usize;
type FnTrainCover = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    ZDICT_cover_params_t,
) -> usize;
type FnOptCover = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> usize;
type FnTrainFast = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    ZDICT_fastCover_params_t,
) -> usize;
type FnOptFast = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> usize;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    ZDICT_legacy_params_t,
) -> usize;
type FnFinalize = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const usize,
    c_uint,
    ZDICT_params_t,
) -> usize;
type FnAddEntropy =
    unsafe extern "C" fn(*mut u8, usize, usize, *const u8, *const usize, c_uint) -> usize;
type FnGetDictID = unsafe extern "C" fn(*const u8, usize) -> c_uint;
type FnGetHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrorName = unsafe extern "C" fn(usize) -> *const c_char;

type FnComputeEpochs = unsafe extern "C" fn(u32, u32, u32, u32) -> COVER_epoch_info_t;
type FnCoverSum = unsafe extern "C" fn(*const usize, c_uint) -> usize;
type FnBest1 = unsafe extern "C" fn(*mut COVER_best_t);
type FnBestFinish =
    unsafe extern "C" fn(*mut COVER_best_t, ZDICT_cover_params_t, COVER_dictSelection_t);
type FnSelectDict = unsafe extern "C" fn(
    *mut u8,
    usize,
    usize,
    *const u8,
    *const usize,
    c_uint,
    usize,
    usize,
    ZDICT_cover_params_t,
    *mut usize,
    usize,
) -> COVER_dictSelection_t;
type FnCheckTotal = unsafe extern "C" fn(
    ZDICT_cover_params_t,
    *const usize,
    *const u8,
    *mut usize,
    usize,
    usize,
    *mut u8,
    usize,
) -> usize;
type FnDictSelErr = unsafe extern "C" fn(usize) -> COVER_dictSelection_t;
type FnDictSelIsErr = unsafe extern "C" fn(COVER_dictSelection_t) -> c_uint;
type FnDictSelFree = unsafe extern "C" fn(COVER_dictSelection_t);
type FnWarnSmall = unsafe extern "C" fn(usize, usize, c_int);
type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut i32, i32, i32) -> i32;
type FnDivbwt =
    unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32, *mut u8, *mut i32, i32) -> i32;

// --------------------------------------------------------------- corpora ----

struct Corpus {
    buf: Vec<u8>,
    sizes: Vec<usize>,
}

impl Corpus {
    fn new() -> Corpus {
        Corpus { buf: Vec::new(), sizes: Vec::new() }
    }
    fn push(&mut self, s: &[u8]) {
        self.sizes.push(s.len());
        self.buf.extend_from_slice(s);
    }
    fn nb(&self) -> c_uint {
        self.sizes.len() as c_uint
    }
    fn ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }
    fn sptr(&self) -> *const usize {
        self.sizes.as_ptr()
    }
    fn total(&self) -> usize {
        self.buf.len()
    }
    /// cumulative offsets, `nbSamples + 1` entries (as COVER builds them)
    fn offsets(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.sizes.len() + 1);
        let mut acc = 0usize;
        v.push(0usize);
        for &s in &self.sizes {
            acc += s;
            v.push(acc);
        }
        v
    }
}

/// `nb` samples of random length in `[lo, hi]` of the given shape.
fn corpus(shape: Shape, seed: u64, nb: usize, lo: usize, hi: usize) -> Corpus {
    let mut rng = Rng::new(seed);
    let mut c = Corpus::new();
    for _ in 0..nb {
        let n = rng.range(lo, hi + 1);
        let s = gen(shape, n, &mut rng);
        c.push(&s);
    }
    c
}

/// `nb` copies of the very same sample.
fn identical_corpus(seed: u64, nb: usize, len: usize) -> Corpus {
    let mut rng = Rng::new(seed);
    let s = gen(Shape::Text, len, &mut rng);
    let mut c = Corpus::new();
    for _ in 0..nb {
        c.push(&s);
    }
    c
}

/// Corpus whose total size is exactly `total` bytes, spread over `nb` samples.
fn sized_corpus(seed: u64, nb: usize, total: usize) -> Corpus {
    let mut rng = Rng::new(seed);
    let big = gen(Shape::Text, total, &mut rng);
    let mut c = Corpus::new();
    let each = total / nb;
    let mut off = 0usize;
    for i in 0..nb {
        let n = if i + 1 == nb { total - off } else { each };
        c.push(&big[off..off + n]);
        off += n;
    }
    assert_eq!(c.total(), total);
    c
}

// -------------------------------------------------------------- helpers -----

/// Run `f` against both libraries with a private output buffer (filled with
/// different sentinels so only genuinely produced bytes can match), assert that
/// the returned `size_t` is identical and, on success, that the produced
/// dictionary is byte-identical.  Returns (retval, C dictionary bytes).
fn diff_dict<F>(cap: usize, ctx: &str, f: F) -> (usize, Vec<u8>)
where
    F: Fn(bool, &mut [u8]) -> usize,
{
    let mut cbuf = vec![0xAAu8; cap];
    let mut rbuf = vec![0x55u8; cap];
    let a = f(false, &mut cbuf);
    let b = f(true, &mut rbuf);
    assert_eq!(a, b, "{ctx}: return mismatch: C={} Rust={}", fmt_ret(a), fmt_ret(b));
    if is_error(a) {
        return (a, Vec::new());
    }
    assert!(a <= cap, "{ctx}: returned {a} > capacity {cap}");
    assert_bytes_eq(&format!("{ctx}: dictionary bytes"), &cbuf[..a], &rbuf[..a]);
    (a, cbuf[..a].to_vec())
}

fn zparams(level: c_int, dictID: c_uint) -> ZDICT_params_t {
    ZDICT_params_t { compressionLevel: level, notificationLevel: 0, dictID }
}

fn cover_params(k: c_uint, d: c_uint, level: c_int, dictID: c_uint) -> ZDICT_cover_params_t {
    ZDICT_cover_params_t {
        k,
        d,
        steps: 0,
        nbThreads: 1,
        splitPoint: 1.0,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: zparams(level, dictID),
    }
}

fn fast_params(k: c_uint, d: c_uint, level: c_int, dictID: c_uint) -> ZDICT_fastCover_params_t {
    ZDICT_fastCover_params_t {
        k,
        d,
        f: 0,
        steps: 0,
        nbThreads: 1,
        splitPoint: 1.0,
        accel: 0,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: zparams(level, dictID),
    }
}

fn train_cover(cap: usize, corp: &Corpus, p: ZDICT_cover_params_t, ctx: &str) -> (usize, Vec<u8>) {
    let (cf, rf) = unsafe { pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover") };
    diff_dict(cap, ctx, |rust, buf| {
        let ptr = buf.as_mut_ptr();
        unsafe {
            if rust {
                rf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            } else {
                cf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            }
        }
    })
}

fn train_fast(
    cap: usize,
    corp: &Corpus,
    p: ZDICT_fastCover_params_t,
    ctx: &str,
) -> (usize, Vec<u8>) {
    let (cf, rf) = unsafe { pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover") };
    diff_dict(cap, ctx, |rust, buf| {
        let ptr = buf.as_mut_ptr();
        unsafe {
            if rust {
                rf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            } else {
                cf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            }
        }
    })
}

fn train_legacy(
    cap: usize,
    corp: &Corpus,
    p: ZDICT_legacy_params_t,
    ctx: &str,
) -> (usize, Vec<u8>) {
    let (cf, rf) = unsafe { pair::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy") };
    diff_dict(cap, ctx, |rust, buf| {
        let ptr = buf.as_mut_ptr();
        unsafe {
            if rust {
                rf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            } else {
                cf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb(), p)
            }
        }
    })
}

fn train_plain(cap: usize, corp: &Corpus, ctx: &str) -> (usize, Vec<u8>) {
    let (cf, rf) = unsafe { pair::<FnTrain>("ZDICT_trainFromBuffer") };
    diff_dict(cap, ctx, |rust, buf| {
        let ptr = buf.as_mut_ptr();
        unsafe {
            if rust {
                rf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb())
            } else {
                cf(ptr, cap, corp.ptr(), corp.sptr(), corp.nb())
            }
        }
    })
}

/// `ZDICT_optimizeTrainFromBuffer_cover` — also compares the parameters that
/// the two libraries write back into `*parameters`.
fn opt_cover(
    cap: usize,
    corp: &Corpus,
    base: ZDICT_cover_params_t,
    ctx: &str,
) -> (usize, ZDICT_cover_params_t) {
    let (cf, rf) = unsafe { pair::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover") };
    let mut cp = base;
    let mut rp = base;
    let mut cbuf = vec![0xAAu8; cap];
    let mut rbuf = vec![0x55u8; cap];
    let a = unsafe { cf(cbuf.as_mut_ptr(), cap, corp.ptr(), corp.sptr(), corp.nb(), &mut cp) };
    let b = unsafe { rf(rbuf.as_mut_ptr(), cap, corp.ptr(), corp.sptr(), corp.nb(), &mut rp) };
    assert_eq!(a, b, "{ctx}: return mismatch C={} Rust={}", fmt_ret(a), fmt_ret(b));
    assert_eq!(cp, rp, "{ctx}: written-back cover params differ");
    if !is_error(a) {
        assert!(a <= cap, "{ctx}: returned {a} > capacity {cap}");
        assert_bytes_eq(&format!("{ctx}: dictionary bytes"), &cbuf[..a], &rbuf[..a]);
    }
    (a, cp)
}

fn opt_fast(
    cap: usize,
    corp: &Corpus,
    base: ZDICT_fastCover_params_t,
    ctx: &str,
) -> (usize, ZDICT_fastCover_params_t) {
    let (cf, rf) = unsafe { pair::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover") };
    let mut cp = base;
    let mut rp = base;
    let mut cbuf = vec![0xAAu8; cap];
    let mut rbuf = vec![0x55u8; cap];
    let a = unsafe { cf(cbuf.as_mut_ptr(), cap, corp.ptr(), corp.sptr(), corp.nb(), &mut cp) };
    let b = unsafe { rf(rbuf.as_mut_ptr(), cap, corp.ptr(), corp.sptr(), corp.nb(), &mut rp) };
    assert_eq!(a, b, "{ctx}: return mismatch C={} Rust={}", fmt_ret(a), fmt_ret(b));
    assert_eq!(cp, rp, "{ctx}: written-back fastCover params differ");
    if !is_error(a) {
        assert!(a <= cap, "{ctx}: returned {a} > capacity {cap}");
        assert_bytes_eq(&format!("{ctx}: dictionary bytes"), &cbuf[..a], &rbuf[..a]);
    }
    (a, cp)
}

fn finalize(
    cap: usize,
    content: &[u8],
    corp: &Corpus,
    p: ZDICT_params_t,
    ctx: &str,
) -> (usize, Vec<u8>) {
    let (cf, rf) = unsafe { pair::<FnFinalize>("ZDICT_finalizeDictionary") };
    diff_dict(cap, ctx, |rust, buf| {
        let ptr = buf.as_mut_ptr();
        unsafe {
            if rust {
                rf(ptr, cap, content.as_ptr(), content.len(), corp.ptr(), corp.sptr(), corp.nb(), p)
            } else {
                cf(ptr, cap, content.as_ptr(), content.len(), corp.ptr(), corp.sptr(), corp.nb(), p)
            }
        }
    })
}

// ============================================================== CONFIGS =====

/// CONFIGS 185 — ZDICT_trainFromBuffer, 200 text samples, several capacities.
#[test]
fn dictb_train_basic() {
    for &seed in &[0xB0A1u64, 0x1234, 0xFEED] {
        let corp = corpus(Shape::Text, seed, 200, 256, 512);
        assert!(corp.total() <= 300 * 1024);
        for &cap in &[256usize, 1024, 4096, 16384, 112640] {
            let ctx = format!("trainFromBuffer seed={seed:#x} cap={cap}");
            let (n, dict) = train_plain(cap, &corp, &ctx);
            assert!(!is_error(n), "{ctx}: unexpected error {}", err_code(n));
            assert!(n >= 8, "{ctx}: dictionary too short ({n})");
            assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: dictionary magic");
        }
    }
}

/// CONFIGS 186 — ZDICT_trainFromBuffer edge corpora.
#[test]
fn dictb_train_edge() {
    // 0 samples
    let empty = Corpus::new();
    let (n, _) = train_plain(4096, &empty, "trainFromBuffer 0 samples");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "0 samples => srcSize_wrong");

    // a single sample
    let one = corpus(Shape::Text, 0x5101, 1, 4096, 4096);
    train_plain(4096, &one, "trainFromBuffer 1 sample");

    // total corpus size 511 B (below the 512 B ZDICT_MIN_SAMPLES_SIZE)
    for &nb in &[1usize, 5, 8, 64] {
        let small = sized_corpus(0x511 + nb as u64, nb, 511);
        assert_eq!(small.total(), 511);
        train_plain(4096, &small, &format!("trainFromBuffer 511 B nb={nb}"));
        train_plain(256, &small, &format!("trainFromBuffer 511 B nb={nb} cap=256"));
    }

    // all samples identical
    for &nb in &[8usize, 200] {
        let ident = identical_corpus(0x1DEA, nb, 128);
        train_plain(4096, &ident, &format!("trainFromBuffer identical nb={nb}"));
    }

    // incompressible corpus
    for &seed in &[0x8AD1u64, 0xA5D2] {
        let rnd = corpus(Shape::Random, seed, 200, 64, 256);
        train_plain(4096, &rnd, &format!("trainFromBuffer random seed={seed:#x}"));
        train_plain(112640, &rnd, &format!("trainFromBuffer random big seed={seed:#x}"));
    }

    // every shape the harness can generate
    for &shape in ALL_SHAPES {
        let c = corpus(shape, 0xA5A5, 120, 64, 320);
        train_plain(8192, &c, &format!("trainFromBuffer shape={shape:?}"));
    }
}

/// CONFIGS 187 + 188 — ZDICT_trainFromBuffer_cover parameter grid.
#[test]
fn dictb_cover() {
    // note: ZDICT_trainFromBuffer_cover() overwrites splitPoint with 1.0, so
    // the value passed in can never influence the result — we still sweep it.
    for &seed in &[0xC0FEu64, 0x2222] {
        let corp = corpus(Shape::Text, seed, 250, 64, 384);
        // full k x d grid (d <= k) x capacity
        for &k in &[16u32, 50, 200, 1000, 2048] {
            for &d in &[6u32, 8, 16] {
                if d > k {
                    continue; // d > k is an error path (see err_dictb_cover_params)
                }
                for &cap in &[256usize, 1024, 4096, 16384, 112640] {
                    if k as usize > cap {
                        continue; // k > maxDictSize is an error path (see err test)
                    }
                    let p = cover_params(k, d, 3, 12345);
                    let ctx = format!("cover seed={seed:#x} k={k} d={d} cap={cap}");
                    let (n, dict) = train_cover(cap, &corp, p, &ctx);
                    assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                    assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: magic");
                    // dictID was pinned
                    assert_eq!(&dict[4..8], &12345u32.to_le_bytes(), "{ctx}: dictID");
                }
            }
        }
        // splitPoint sweep (overwritten with 1.0 by this entry point)
        for &(k, d) in &[(50u32, 6u32), (200, 8)] {
            for &sp in &[0.0f64, 0.5, 0.75, 1.0] {
                let mut p = cover_params(k, d, 3, 12345);
                p.splitPoint = sp;
                let ctx = format!("cover seed={seed:#x} k={k} d={d} sp={sp}");
                let (n, dict) = train_cover(8192, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert_eq!(&dict[4..8], &12345u32.to_le_bytes(), "{ctx}: dictID");
            }
        }
        // CONFIGS 188: shrinkDict x shrinkDictMaxRegression (no effect on the
        // non-optimizing entry point, but must still match byte for byte)
        for &shrink in &[0u32, 1] {
            for &reg in &[0u32, 1, 5] {
                for &sp in &[0.5f64, 0.75] {
                    let mut p = cover_params(200, 8, 3, 1);
                    p.shrinkDict = shrink;
                    p.shrinkDictMaxRegression = reg;
                    p.splitPoint = sp;
                    let ctx = format!(
                        "cover seed={seed:#x} shrink={shrink} reg={reg} sp={sp}"
                    );
                    let (n, _) = train_cover(8192, &corp, p, &ctx);
                    assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                }
            }
        }
        // compression levels and dictIDs
        for &lvl in &[0i32, 1, 3, 9, 19] {
            for &id in &[1u32, 12345, 0x8000_0001] {
                let p = cover_params(200, 8, lvl, id);
                let ctx = format!("cover seed={seed:#x} lvl={lvl} id={id:#x}");
                let (n, dict) = train_cover(8192, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert_eq!(&dict[4..8], &id.to_le_bytes(), "{ctx}: dictID");
            }
        }
        // dictID == 0 => derived from XXH64(content): deterministic, but the
        // value is not pinned, so only the return code is asserted here.
        let p = cover_params(200, 8, 3, 0);
        let (cf, rf) = unsafe { pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover") };
        let mut cbuf = vec![0u8; 8192];
        let mut rbuf = vec![0u8; 8192];
        let a = unsafe {
            cf(cbuf.as_mut_ptr(), 8192, corp.ptr(), corp.sptr(), corp.nb(), p)
        };
        let b = unsafe {
            rf(rbuf.as_mut_ptr(), 8192, corp.ptr(), corp.sptr(), corp.nb(), p)
        };
        assert_eq!(a, b, "cover dictID=0 return");
        assert!(!is_error(a));
        // shapes other than text
        for &shape in &[Shape::Random, Shape::Rle, Shape::Repetitive, Shape::Sparse] {
            let c = corpus(shape, seed ^ 0x99, 120, 64, 256);
            let p = cover_params(200, 8, 3, 7);
            train_cover(4096, &c, p, &format!("cover shape={shape:?} seed={seed:#x}"));
        }
    }
}

/// ERRORS 290..299 + CONFIGS 189 — invalid cover parameters.
#[test]
fn err_dictb_cover_params() {
    let corp = corpus(Shape::Text, 0xBAD0, 40, 64, 256);
    let empty = Corpus::new();

    // --- 290: parameter_outOfBound from COVER_checkParameters -------------
    let bad: &[(&str, u32, u32, usize)] = &[
        ("d=0", 200, 0, 4096),
        ("k=0", 0, 8, 4096),
        ("k=0,d=0", 0, 0, 4096),
        ("d>k", 10, 16, 4096),
        ("k>capacity", 1000, 8, 256),
    ];
    for &(name, k, d, cap) in bad {
        let p = cover_params(k, d, 3, 1);
        let ctx = format!("cover invalid {name}");
        let (n, _) = train_cover(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}: expected parameter_outOfBound");
    }
    // splitPoint is *overwritten* with 1.0 by ZDICT_trainFromBuffer_cover, so
    // an out-of-range value is NOT an error there; both libs must agree.
    for &sp in &[0.0f64, -1.0, 1.5, f64::NAN] {
        let mut p = cover_params(200, 8, 3, 1);
        p.splitPoint = sp;
        let ctx = format!("cover splitPoint={sp} (overwritten)");
        train_cover(4096, &corp, p, &ctx);
    }

    // --- 291: nbSamples == 0 ---------------------------------------------
    let (n, _) = train_cover(4096, &empty, cover_params(200, 8, 3, 1), "cover 0 samples");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "cover 0 samples");

    // --- 292: dictBufferCapacity < ZDICT_DICTSIZE_MIN --------------------
    // note: `k > maxDictSize` is checked *before* the capacity, so with k=50
    // capacities below 50 report parameter_outOfBound instead.
    for &cap in &[100usize, 255] {
        let p = cover_params(50, 8, 3, 1);
        let ctx = format!("cover cap={cap}");
        let (n, _) = train_cover(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
    for &cap in &[1usize, 8, 50] {
        let p = cover_params(1, 1, 3, 1);
        let ctx = format!("cover k=1 cap={cap}");
        let (n, _) = train_cover(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
    for &cap in &[0usize, 1, 49] {
        let p = cover_params(50, 8, 3, 1);
        let ctx = format!("cover k=50 > cap={cap}");
        let (n, _) = train_cover(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 293: totalSamplesSize < MAX(d, 8) -------------------------------
    for &(nb, each) in &[(5usize, 1usize), (7, 1)] {
        let mut tiny = Corpus::new();
        for _ in 0..nb {
            tiny.push(&vec![b'a'; each]);
        }
        let ctx = format!("cover tiny total={} d=16", tiny.total());
        let (n, _) = train_cover(4096, &tiny, cover_params(200, 16, 3, 1), &ctx);
        assert_eq!(n, as_err(E_SRCSIZE_WRONG), "{ctx}");
    }

    // --- 294: nbTrainSamples < 5 ----------------------------------------
    for &nb in &[1usize, 2, 4] {
        let few = corpus(Shape::Text, 0x1111 + nb as u64, nb, 100, 200);
        let ctx = format!("cover nbSamples={nb}");
        let (n, _) = train_cover(4096, &few, cover_params(50, 8, 3, 1), &ctx);
        assert_eq!(n, as_err(E_SRCSIZE_WRONG), "{ctx}");
    }

    // --- 295: nbTestSamples < 1 -----------------------------------------
    // `nbTestSamples = nbSamples - (unsigned)(nbSamples * splitPoint)` can only
    // reach 0 if the truncated product equals nbSamples, which never happens
    // for splitPoint < 1.0 (even for the largest double below 1.0 the product
    // stays strictly below nbSamples — verified for every nbSamples < 20000).
    // The check is therefore defensive; what is tested here is the whole
    // splitPoint boundary region, where the *other* two checks fire.
    {
        let sp_max = 1.0f64 - f64::EPSILON / 2.0;
        assert!(sp_max < 1.0);
        for &nb in &[5usize, 6, 7, 10, 100] {
            for &sp in &[1e-12f64, 0.001, 0.1, 0.5, 0.75, 0.9, sp_max] {
                let c = corpus(Shape::Text, 0x4242 + nb as u64, nb, 64, 128);
                let mut p = cover_params(50, 6, 3, 1);
                p.splitPoint = sp;
                p.steps = 1;
                let ctx = format!("optimize cover splitPoint boundary nb={nb} sp={sp}");
                let (n, _) = opt_cover(4096, &c, p, &ctx);
                let ntrain = (nb as f64 * sp) as usize;
                if ntrain < 5 {
                    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "{ctx}: nbTrainSamples={ntrain}");
                }
                assert!(nb - ntrain >= 1, "{ctx}: nbTestSamples reached 0");
            }
        }
    }

    // --- 296: optimize splitPoint out of range --------------------------
    for &sp in &[1.5f64, 2.0, 100.0] {
        let mut p = cover_params(50, 6, 3, 1);
        p.steps = 1;
        p.splitPoint = sp;
        let ctx = format!("optimize cover splitPoint={sp}");
        let (n, _) = opt_cover(4096, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }
    // splitPoint <= 0 means "default" for the optimizing entry point
    for &sp in &[0.0f64, -1.0] {
        let mut p = cover_params(50, 6, 3, 1);
        p.steps = 1;
        p.splitPoint = sp;
        let ctx = format!("optimize cover splitPoint={sp} (default)");
        let (n, _) = opt_cover(4096, &corp, p, &ctx);
        assert!(!is_error(n), "{ctx}: unexpected error {}", err_code(n));
    }

    // --- 297: kMinK < kMaxD -------------------------------------------
    for &(k, d) in &[(6u32, 8u32), (1, 6), (7, 16)] {
        let mut p = cover_params(k, d, 3, 1);
        p.steps = 1;
        let ctx = format!("optimize cover k={k} d={d}");
        let (n, _) = opt_cover(4096, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 298: optimize nbSamples == 0 ---------------------------------
    let mut p = cover_params(50, 6, 3, 1);
    p.steps = 1;
    let (n, _) = opt_cover(4096, &empty, p, "optimize cover 0 samples");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "optimize cover 0 samples");

    // --- 299: optimize capacity < 256 --------------------------------
    for &cap in &[0usize, 100, 255] {
        let mut p = cover_params(50, 6, 3, 1);
        p.steps = 1;
        let ctx = format!("optimize cover cap={cap}");
        let (n, _) = opt_cover(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
}

/// CONFIGS 190 + 191 — ZDICT_optimizeTrainFromBuffer_cover.
#[test]
fn dictb_cover_optimize() {
    for &seed in &[0x09B1u64, 0xB33F] {
        let corp = corpus(Shape::Text, seed, 120, 64, 192);
        // 190: k=0,d=0 with several `steps`, two levels; check written-back params
        for &steps in &[1u32, 4, 8] {
            for &lvl in &[3i32, 9] {
                let mut p = cover_params(0, 0, lvl, 12345);
                p.steps = steps;
                let ctx = format!("optimize cover seed={seed:#x} steps={steps} lvl={lvl}");
                let (n, out) = opt_cover(4096, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert!(out.k >= 50 && out.k <= 2000, "{ctx}: selected k={}", out.k);
                assert!(out.d == 6 || out.d == 8, "{ctx}: selected d={}", out.d);
                assert_eq!(out.zParams.dictID, 12345, "{ctx}: dictID preserved");
            }
        }
        // 191: d fixed, k auto, splitPoint 0.75, nbThreads 1 vs 4
        for &threads in &[1u32, 4] {
            let mut p = cover_params(0, 6, 3, 1);
            p.steps = 4;
            p.splitPoint = 0.75;
            p.nbThreads = threads;
            let ctx = format!("optimize cover seed={seed:#x} d=6 threads={threads}");
            let (n, _) = opt_cover(4096, &corp, p, &ctx);
            assert!(!is_error(n), "{ctx}: error {}", err_code(n));
        }
        // shrinkDict has no effect on the optimizing entry point (shrinkDict is
        // forced to 0 internally) but must still agree.
        for &shrink in &[0u32, 1] {
            for &reg in &[0u32, 5] {
                let mut p = cover_params(0, 8, 3, 1);
                p.steps = 2;
                p.shrinkDict = shrink;
                p.shrinkDictMaxRegression = reg;
                let ctx = format!("optimize cover shrink={shrink} reg={reg} seed={seed:#x}");
                opt_cover(4096, &corp, p, &ctx);
            }
        }
    }
    // steps = 0 => 40 steps: expensive, so use a small corpus
    let small = corpus(Shape::Text, 0x5A11, 60, 48, 160);
    let mut p = cover_params(0, 8, 3, 1);
    p.steps = 0;
    let (n, out) = opt_cover(2048, &small, p, "optimize cover steps=0(40)");
    assert!(!is_error(n), "steps=0: error {}", err_code(n));
    assert!(out.k >= 50 && out.k <= 2000);
    // fixed k and d: a single iteration
    let mut p = cover_params(200, 8, 3, 1);
    p.steps = 4;
    let (n, out) = opt_cover(4096, &small, p, "optimize cover fixed k/d");
    assert!(!is_error(n));
    assert_eq!((out.k, out.d), (200, 8), "single iteration keeps k/d");
}

/// CONFIGS 192 + 193 — ZDICT_trainFromBuffer_fastCover.
#[test]
fn dictb_fastcover() {
    for &seed in &[0xFA57u64, 0x7777] {
        let corp = corpus(Shape::Text, seed, 200, 64, 320);
        // 192: f sweep
        for &f in &[0u32, 1, 6, 20, 25] {
            for &(k, d) in &[(50u32, 6u32), (200, 8), (1000, 8), (2048, 6)] {
                let mut p = fast_params(k, d, 3, 12345);
                p.f = f;
                let ctx = format!("fastCover seed={seed:#x} k={k} d={d} f={f}");
                let (n, dict) = train_fast(8192, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: magic");
                assert_eq!(&dict[4..8], &12345u32.to_le_bytes(), "{ctx}: dictID");
            }
        }
        // 193: accel sweep
        for &accel in &[0u32, 1, 2, 5, 10] {
            let mut p = fast_params(200, 8, 3, 1);
            p.f = 20;
            p.accel = accel;
            let ctx = format!("fastCover seed={seed:#x} accel={accel}");
            let (n, _) = train_fast(8192, &corp, p, &ctx);
            assert!(!is_error(n), "{ctx}: error {}", err_code(n));
        }
        // splitPoint is overwritten with 1.0; shrinkDict/regression are inert
        for &sp in &[0.0f64, 0.5, 0.75, 1.0, 1.5] {
            for &shrink in &[0u32, 1] {
                let mut p = fast_params(200, 8, 3, 1);
                p.splitPoint = sp;
                p.shrinkDict = shrink;
                p.shrinkDictMaxRegression = 5;
                let ctx = format!("fastCover sp={sp} shrink={shrink} seed={seed:#x}");
                let (n, _) = train_fast(4096, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
            }
        }
        // levels, dictIDs and capacities
        for &lvl in &[0i32, 1, 3, 9, 19] {
            for &cap in &[256usize, 1024, 4096, 16384, 112640] {
                let p = fast_params(200, 8, lvl, 0x8000_0001);
                let ctx = format!("fastCover lvl={lvl} cap={cap} seed={seed:#x}");
                let (n, dict) = train_fast(cap, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert_eq!(&dict[4..8], &0x8000_0001u32.to_le_bytes(), "{ctx}: dictID");
            }
        }
        for &shape in &[Shape::Random, Shape::Rle, Shape::Mixed, Shape::LongMatches] {
            let c = corpus(shape, seed ^ 0x1234, 100, 64, 256);
            let p = fast_params(200, 8, 3, 9);
            train_fast(4096, &c, p, &format!("fastCover shape={shape:?}"));
        }
    }
    // f = 25 and f = 31 need 6 * 2^f bytes of (lazily zeroed) memory — run them
    // once on a small corpus.  f = 31 asks for 12 GB, which exceeds the
    // RLIMIT_DATA of the sandbox; both libraries must then agree on
    // memory_allocation.
    let small = corpus(Shape::Text, 0x1F1F, 40, 48, 128);
    let _heavy = HEAVY.lock().unwrap();
    for &f in &[25u32, 31] {
        let mut p = fast_params(200, 8, 3, 1);
        p.f = f;
        let ctx = format!("fastCover f={f}");
        let (n, _) = train_fast(4096, &small, p, &ctx);
        if is_error(n) {
            assert_eq!(n, as_err(E_MEMORY_ALLOCATION), "{ctx}: unexpected error");
        }
    }
}

/// ERRORS 300..308 + CONFIGS 194 — invalid fastCover parameters.
#[test]
fn err_dictb_fastcover_params() {
    let corp = corpus(Shape::Text, 0xBAD1, 40, 64, 256);
    let empty = Corpus::new();

    // --- 300: FASTCOVER_checkParameters ---------------------------------
    let bad: &[(&str, u32, u32, u32, u32, usize)] = &[
        // name, k, d, f, accel, cap
        ("d=0", 200, 0, 20, 1, 4096),
        ("k=0", 0, 8, 20, 1, 4096),
        ("d=7", 200, 7, 20, 1, 4096),
        ("d=16", 200, 16, 20, 1, 4096),
        ("d>k", 4, 6, 20, 1, 4096),
        ("k>capacity", 1000, 8, 20, 1, 256),
        ("f=32", 200, 8, 32, 1, 4096),
        ("f=33", 200, 8, 33, 1, 4096),
        ("accel=11", 200, 8, 20, 11, 4096),
        ("accel=255", 200, 8, 20, 255, 4096),
    ];
    for &(name, k, d, f, accel, cap) in bad {
        let mut p = fast_params(k, d, 3, 1);
        p.f = f;
        p.accel = accel;
        let ctx = format!("fastCover invalid {name}");
        let (n, _) = train_fast(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }
    // f == 0 / accel == 0 mean "default", splitPoint is overwritten with 1.0
    for &sp in &[0.0f64, 1.5, -3.0] {
        let mut p = fast_params(200, 8, 3, 1);
        p.f = 0;
        p.accel = 0;
        p.splitPoint = sp;
        let ctx = format!("fastCover defaults sp={sp}");
        let (n, _) = train_fast(4096, &corp, p, &ctx);
        assert!(!is_error(n), "{ctx}: unexpected error {}", err_code(n));
    }

    // --- 301: nbSamples == 0 -------------------------------------------
    let (n, _) = train_fast(4096, &empty, fast_params(200, 8, 3, 1), "fastCover 0 samples");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "fastCover 0 samples");

    // --- 302: capacity < 256 -------------------------------------------
    // (as for COVER, `k > maxDictSize` is checked first)
    for &cap in &[100usize, 255] {
        let ctx = format!("fastCover cap={cap}");
        let (n, _) = train_fast(cap, &corp, fast_params(50, 8, 3, 1), &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
    for &cap in &[6usize, 8, 50] {
        let ctx = format!("fastCover k=6 cap={cap}");
        let (n, _) = train_fast(cap, &corp, fast_params(6, 6, 3, 1), &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
    for &cap in &[0usize, 1, 49] {
        let ctx = format!("fastCover k=50 > cap={cap}");
        let (n, _) = train_fast(cap, &corp, fast_params(50, 8, 3, 1), &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 303: corpus too small / too few train samples -----------------
    {
        let mut tiny = Corpus::new();
        for _ in 0..5 {
            tiny.push(b"a");
        }
        let (n, _) = train_fast(4096, &tiny, fast_params(200, 8, 3, 1), "fastCover 5 B corpus");
        assert_eq!(n, as_err(E_SRCSIZE_WRONG), "fastCover 5 B corpus");
    }
    for &nb in &[1usize, 4] {
        let few = corpus(Shape::Text, 0x303 + nb as u64, nb, 100, 200);
        let ctx = format!("fastCover nbSamples={nb}");
        let (n, _) = train_fast(4096, &few, fast_params(50, 8, 3, 1), &ctx);
        assert_eq!(n, as_err(E_SRCSIZE_WRONG), "{ctx}");
    }

    // --- 304: optimize splitPoint --------------------------------------
    for &sp in &[1.5f64, 3.0] {
        let mut p = fast_params(50, 6, 3, 1);
        p.steps = 1;
        p.splitPoint = sp;
        let ctx = format!("optimize fastCover splitPoint={sp}");
        let (n, _) = opt_fast(4096, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 305: optimize accel -------------------------------------------
    for &accel in &[11u32, 100] {
        let mut p = fast_params(50, 6, 3, 1);
        p.steps = 1;
        p.accel = accel;
        let ctx = format!("optimize fastCover accel={accel}");
        let (n, _) = opt_fast(4096, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 306: kMinK < kMaxD --------------------------------------------
    for &(k, d) in &[(6u32, 8u32), (1, 6)] {
        let mut p = fast_params(k, d, 3, 1);
        p.steps = 1;
        let ctx = format!("optimize fastCover k={k} d={d}");
        let (n, _) = opt_fast(4096, &corp, p, &ctx);
        assert_eq!(n, as_err(E_PARAMETER_OUTOFBOUND), "{ctx}");
    }

    // --- 307: optimize nbSamples == 0 ----------------------------------
    let mut p = fast_params(50, 6, 3, 1);
    p.steps = 1;
    let (n, _) = opt_fast(4096, &empty, p, "optimize fastCover 0 samples");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "optimize fastCover 0 samples");

    // --- 308: optimize capacity < 256 ---------------------------------
    for &cap in &[0usize, 100, 255] {
        let mut p = fast_params(50, 6, 3, 1);
        p.steps = 1;
        let ctx = format!("optimize fastCover cap={cap}");
        let (n, _) = opt_fast(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }
}

/// CONFIGS 195 — ZDICT_optimizeTrainFromBuffer_fastCover.
#[test]
fn dictb_fastcover_optimize() {
    for &seed in &[0x0F51u64, 0xDEAD] {
        let corp = corpus(Shape::Text, seed, 150, 64, 192);
        for &steps in &[1u32, 4, 8] {
            for &sp in &[0.0f64, 0.5, 1.0] {
                let mut p = fast_params(0, 0, 3, 12345);
                p.f = 0;
                p.accel = 0;
                p.steps = steps;
                p.splitPoint = sp;
                let ctx = format!("optimize fastCover seed={seed:#x} steps={steps} sp={sp}");
                let (n, out) = opt_fast(4096, &corp, p, &ctx);
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert!(out.k >= 50 && out.k <= 2000, "{ctx}: k={}", out.k);
                assert!(out.d == 6 || out.d == 8, "{ctx}: d={}", out.d);
                assert_eq!(out.zParams.dictID, 12345, "{ctx}: dictID");
            }
        }
        // explicit f / accel / nbThreads / levels
        for &f in &[6u32, 20] {
            for &accel in &[1u32, 5, 10] {
                for &threads in &[1u32, 4] {
                    let mut p = fast_params(0, 8, 9, 1);
                    p.f = f;
                    p.accel = accel;
                    p.steps = 2;
                    p.nbThreads = threads;
                    let ctx = format!(
                        "optimize fastCover f={f} accel={accel} threads={threads} seed={seed:#x}"
                    );
                    let (n, _) = opt_fast(4096, &corp, p, &ctx);
                    assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                }
            }
        }
    }
    // steps = 0 => 40 steps on a small corpus
    let small = corpus(Shape::Text, 0x5A22, 60, 48, 160);
    let mut p = fast_params(0, 8, 3, 1);
    p.steps = 0;
    let (n, out) = opt_fast(2048, &small, p, "optimize fastCover steps=0(40)");
    assert!(!is_error(n), "steps=0: error {}", err_code(n));
    assert!(out.k >= 50 && out.k <= 2000);
}

/// CONFIGS 196 — ZDICT_trainFromBuffer_legacy.
#[test]
fn dictb_legacy() {
    for &seed in &[0x1E6Au64, 0x5555] {
        let corp = corpus(Shape::Text, seed, 250, 64, 384);
        let mut built = 0usize;
        for &sel in &[0u32, 1, 5, 9, 12, 31] {
            for &cap in &[256usize, 1024, 4096, 16384, 112640] {
                for &lvl in &[0i32, 1, 3, 9, 19] {
                    let p = ZDICT_legacy_params_t {
                        selectivityLevel: sel,
                        zParams: zparams(lvl, 12345),
                    };
                    let ctx =
                        format!("legacy seed={seed:#x} sel={sel} cap={cap} lvl={lvl}");
                    let (n, dict) = train_legacy(cap, &corp, p, &ctx);
                    if is_error(n) || n == 0 {
                        // both libraries agreed on the error / empty result
                        continue;
                    }
                    assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: magic");
                    assert_eq!(&dict[4..8], &12345u32.to_le_bytes(), "{ctx}: dictID");
                    built += 1;
                }
            }
        }
        assert!(built > 20, "legacy seed={seed:#x}: only {built} dictionaries built");
        // identical samples and mixed shapes
        for &shape in &[Shape::Repetitive, Shape::Mixed, Shape::Sparse, Shape::LongMatches] {
            let c = corpus(shape, seed ^ 0x77, 150, 64, 256);
            let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 1) };
            train_legacy(16384, &c, p, &format!("legacy shape={shape:?} seed={seed:#x}"));
        }
        let ident = identical_corpus(seed, 200, 256);
        let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 1) };
        train_legacy(16384, &ident, p, &format!("legacy identical seed={seed:#x}"));
    }
}

/// ERRORS 309..312 — ZDICT_trainFromBuffer_legacy failure modes.
#[test]
fn err_dictb_legacy() {
    let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3, 1) };

    // --- 309: total sample size < ZDICT_MIN_SAMPLES_SIZE (512) => 0 ------
    for &total in &[0usize, 1, 100, 511] {
        let corp = if total == 0 {
            Corpus::new()
        } else {
            sized_corpus(0x309 + total as u64, 1.max(total / 64), total)
        };
        let ctx = format!("legacy total={total}");
        let (n, _) = train_legacy(4096, &corp, p, &ctx);
        assert_eq!(n, 0, "{ctx}: expected 0 (no dictionary)");
    }
    // exactly 512 bytes is enough to get past the wrapper
    let ok = sized_corpus(0x512, 8, 512);
    let (n, _) = train_legacy(4096, &ok, p, "legacy total=512");
    // may or may not produce a dictionary, but both libraries must agree
    let _ = n;

    // --- 310: dictBufferCapacity < ZDICT_DICTSIZE_MIN -------------------
    let corp = corpus(Shape::Text, 0x310, 200, 64, 256);
    assert!(corp.total() >= 512);
    for &cap in &[0usize, 1, 100, 255] {
        let ctx = format!("legacy cap={cap}");
        let (n, _) = train_legacy(cap, &corp, p, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }

    // --- 311: ZDICT_MIN_SAMPLES_SIZE check inside the unsafe variant is
    // unreachable through the public wrapper (it returns 0 first, see 309).

    // --- 312: selected content < ZDICT_CONTENTSIZE_MIN (128) ------------
    for &seed in &[0x312u64, 0x313, 0x314] {
        let rnd = corpus(Shape::Random, seed, 8, 128, 128);
        assert!(rnd.total() >= 512);
        let ctx = format!("legacy incompressible seed={seed:#x}");
        let (n, _) = train_legacy(4096, &rnd, p, &ctx);
        assert_eq!(
            n,
            as_err(E_DICTIONARYCREATION_FAILED),
            "{ctx}: expected dictionaryCreation_failed"
        );
    }
}

/// CONFIGS 197 + 199 — ZDICT_finalizeDictionary and
/// ZDICT_addEntropyTablesFromBuffer.
#[test]
fn dictb_finalize() {
    let (c_ae, r_ae) = unsafe { pair::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer") };

    for &seed in &[0xF1DAu64, 0xABCD] {
        let corp = corpus(Shape::Text, seed, 200, 64, 384);
        let mut rng = Rng::new(seed ^ 0xFF);
        let mut built = 0usize;
        for &clen in &[0usize, 8, 64, 1024, 8192] {
            let content = gen(Shape::Text, clen, &mut rng);
            for &cap in &[256usize, 1024, 8192, 12288] {
                for &lvl in &[0i32, 1, 3, 19] {
                    for &id in &[1u32, 12345, 0x8000_0001] {
                        let ctx = format!(
                            "finalize seed={seed:#x} clen={clen} cap={cap} lvl={lvl} id={id:#x}"
                        );
                        let (n, dict) =
                            finalize(cap, &content, &corp, zparams(lvl, id), &ctx);
                        if is_error(n) {
                            // capacity < content size etc. — both agreed
                            assert!(cap < clen.max(256), "{ctx}: unexpected error");
                            continue;
                        }
                        assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: magic");
                        assert_eq!(&dict[4..8], &id.to_le_bytes(), "{ctx}: dictID");
                        assert!(n <= cap);
                        built += 1;
                    }
                }
            }
        }
        assert!(built > 100, "finalize seed={seed:#x}: only {built} dictionaries built");
        // dictID = 0 => XXH64(content)-derived, identical in both libs
        let content = gen(Shape::Text, 4096, &mut rng);
        let (n, dict) = finalize(8192, &content, &corp, zparams(3, 0), "finalize dictID=0");
        assert!(!is_error(n));
        assert_ne!(&dict[4..8], &[0u8; 4], "auto dictID must be non-zero");

        // incompressible / identical / few samples
        for (name, c) in [
            ("random", corpus(Shape::Random, seed, 100, 64, 256)),
            ("identical", identical_corpus(seed, 100, 128)),
            ("single", corpus(Shape::Text, seed, 1, 512, 512)),
        ] {
            let ctx = format!("finalize samples={name} seed={seed:#x}");
            finalize(8192, &content, &c, zparams(3, 1), &ctx);
        }

        // CONFIGS 199: ZDICT_addEntropyTablesFromBuffer — the content has to
        // live at the END of the buffer.
        for &clen in &[64usize, 1024, 8192] {
            for &cap in &[1024usize, 16384] {
                if clen + 8 > cap {
                    continue;
                }
                let content = gen(Shape::Text, clen, &mut rng);
                let ctx = format!("addEntropyTables clen={clen} cap={cap} seed={seed:#x}");
                let (n, dict) = diff_dict(cap, &ctx, |rust, buf| {
                    buf[cap - clen..].copy_from_slice(&content);
                    let ptr = buf.as_mut_ptr();
                    unsafe {
                        if rust {
                            r_ae(ptr, clen, cap, corp.ptr(), corp.sptr(), corp.nb())
                        } else {
                            c_ae(ptr, clen, cap, corp.ptr(), corp.sptr(), corp.nb())
                        }
                    }
                });
                assert!(!is_error(n), "{ctx}: error {}", err_code(n));
                assert_eq!(&dict[..4], &DICT_MAGIC, "{ctx}: magic");
            }
        }
    }
}

/// ERRORS 282..286 + 313 and CONFIGS 198 — finalizeDictionary failures.
#[test]
fn err_dictb_finalize() {
    let (c_ae, r_ae) = unsafe { pair::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer") };
    let corp = corpus(Shape::Text, 0xE1AA, 200, 64, 256);
    let mut rng = Rng::new(0xE1AB);

    // --- 282: dictBufferCapacity < dictContentSize ----------------------
    for &(cap, clen) in &[(256usize, 1024usize), (1024, 4096), (4096, 8192)] {
        let content = gen(Shape::Text, clen, &mut rng);
        let ctx = format!("finalize cap={cap} < clen={clen}");
        let (n, _) = finalize(cap, &content, &corp, zparams(3, 1), &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }

    // --- 283: dictBufferCapacity < ZDICT_DICTSIZE_MIN (256) -------------
    for &cap in &[0usize, 1, 100, 255] {
        let content = gen(Shape::Text, cap.min(64), &mut rng);
        let ctx = format!("finalize cap={cap} (< 256)");
        let (n, _) = finalize(cap, &content, &corp, zparams(3, 1), &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }

    // --- 284: hSize + minContentSize > capacity ------------------------
    // Needs the entropy header to grow to nearly HBUFFSIZE; try the smallest
    // legal capacity with several corpora — both libraries must agree whatever
    // the outcome is.
    for &seed in &[0x284u64, 0x285, 0x286] {
        for &clen in &[0usize, 1, 4] {
            let content = gen(Shape::Random, clen, &mut rng);
            let c = corpus(Shape::Random, seed, 50, 64, 256);
            let ctx = format!("finalize cap=256 clen={clen} seed={seed:#x}");
            finalize(256, &content, &c, zparams(19, 1), &ctx);
        }
    }

    // --- 285 / 313: ZDICT_analyzeEntropy has < 12 bytes left for the
    // repcodes; reachable through addEntropyTablesFromBuffer with a tiny
    // capacity, whose errors are forwarded verbatim.
    for &cap in &[9usize, 12, 16, 20, 40, 80, 120] {
        let clen = 0usize;
        let ctx = format!("addEntropyTables tiny cap={cap}");
        let (n, _) = diff_dict(cap, &ctx, |rust, buf| {
            let ptr = buf.as_mut_ptr();
            unsafe {
                if rust {
                    r_ae(ptr, clen, cap, corp.ptr(), corp.sptr(), corp.nb())
                } else {
                    c_ae(ptr, clen, cap, corp.ptr(), corp.sptr(), corp.nb())
                }
            }
        });
        assert!(is_error(n), "addEntropyTables cap={cap} should fail");
    }

    // --- 286: offcodeMax > OFFCODE_MAX, i.e. dictContentSize + 128 KB
    // >= 2^31.  `dstDictBuffer` and `dictContent` are explicitly allowed to
    // overlap, so a single 2 GB buffer serves as both (and is reused for the
    // two libraries): the allocation is zeroed and therefore lazily mapped, so
    // this costs address space rather than RAM, and stays inside RLIMIT_DATA.
    {
        let clen = (1usize << 31) - (128 << 10);
        let cap = clen + 256;
        let _heavy = HEAVY.lock().unwrap();
        let buf = RawBuf::zeroed(cap).expect("2 GB zeroed allocation");
        let (cf, rf) = unsafe { pair::<FnFinalize>("ZDICT_finalizeDictionary") };
        let p = zparams(3, 1);
        let dst = buf.ptr;
        let a =
            unsafe { cf(dst, cap, dst as *const u8, clen, corp.ptr(), corp.sptr(), corp.nb(), p) };
        let b =
            unsafe { rf(dst, cap, dst as *const u8, clen, corp.ptr(), corp.sptr(), corp.nb(), p) };
        assert_eq!(a, b, "finalize huge content: C={} Rust={}", fmt_ret(a), fmt_ret(b));
        assert_eq!(
            a,
            as_err(E_DICTIONARYCREATION_FAILED),
            "finalize huge content => dictionaryCreation_failed"
        );
    }

    // --- CONFIGS 198: 0 samples, identical samples, random samples ------
    let content = gen(Shape::Text, 4096, &mut rng);
    let empty = Corpus::new();
    let (n, _) = finalize(8192, &content, &empty, zparams(3, 1), "finalize 0 samples");
    let _ = n; // whatever it is, both libraries returned the same value
    for &lvl in &[1i32, 3, 19] {
        let ident = identical_corpus(0x198, 100, 128);
        finalize(8192, &content, &ident, zparams(lvl, 1), &format!("finalize identical lvl={lvl}"));
        let rnd = corpus(Shape::Random, 0x199, 100, 64, 256);
        finalize(8192, &content, &rnd, zparams(lvl, 1), &format!("finalize random lvl={lvl}"));
    }
    // maxDictSize = 100 (CONFIGS 198)
    let (n, _) = finalize(100, &content, &corp, zparams(3, 1), "finalize cap=100");
    assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "finalize cap=100");
}

/// CONFIGS 200 + ERRORS 278..280 — ZDICT_getDictID / isError / getErrorName.
#[test]
fn dictb_queries() {
    let (c_id, r_id) = unsafe { pair::<FnGetDictID>("ZDICT_getDictID") };
    let (c_hs, r_hs) = unsafe { pair::<FnGetHeaderSize>("ZDICT_getDictHeaderSize") };
    let (c_ie, r_ie) = unsafe { pair::<FnIsError>("ZDICT_isError") };
    let (c_en, r_en) = unsafe { pair::<FnErrorName>("ZDICT_getErrorName") };

    // ERRORS 278: isError on non-error values, and on every error code
    let mut codes: Vec<usize> = vec![0, 1, 2, 8, 100, 1000, 100000, usize::MAX / 2];
    for c in 0..=126usize {
        codes.push(as_err(c));
    }
    codes.push(usize::MAX);
    codes.push(usize::MAX - 1);
    for &code in &codes {
        let a = unsafe { c_ie(code) };
        let b = unsafe { r_ie(code) };
        assert_eq!(a, b, "ZDICT_isError({code:#x})");
        let sa = unsafe { CStr::from_ptr(c_en(code)) };
        let sb = unsafe { CStr::from_ptr(r_en(code)) };
        assert_eq!(sa, sb, "ZDICT_getErrorName({code:#x})");
    }

    // a real trained dictionary with a pinned dictID
    let corp = corpus(Shape::Text, 0xC301, 200, 64, 320);
    let (n, trained) = train_cover(8192, &corp, cover_params(200, 8, 3, 0x8000_0001), "queries dict");
    assert!(!is_error(n));

    let raw = gen(Shape::Random, 4096, &mut Rng::new(0xC302));
    let mut trunc_magic = trained.clone();
    trunc_magic.truncate(6);
    let mut bad_magic = trained.clone();
    bad_magic[0] ^= 0xFF;

    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("trained", trained.clone()),
        ("raw", raw.clone()),
        ("3-byte", vec![1, 2, 3]),
        ("empty", Vec::new()),
        ("truncated magic", trunc_magic),
        ("bad magic", bad_magic),
        ("header only", trained[..8].to_vec()),
        ("header+1", trained[..9].to_vec()),
    ];
    for (name, buf) in &cases {
        for &len in &[buf.len(), buf.len() / 2, 0] {
            let a = unsafe { c_id(buf.as_ptr(), len) };
            let b = unsafe { r_id(buf.as_ptr(), len) };
            assert_eq!(a, b, "getDictID {name} len={len}");
            let ha = unsafe { c_hs(buf.as_ptr(), len) };
            let hb = unsafe { r_hs(buf.as_ptr(), len) };
            assert_eq!(ha, hb, "getDictHeaderSize {name} len={len}");
        }
    }
    // ERRORS 279 / 280
    assert_eq!(unsafe { c_id(trained.as_ptr(), 7) }, 0, "dictSize < 8 => 0");
    assert_eq!(unsafe { r_id(trained.as_ptr(), 7) }, 0, "dictSize < 8 => 0");
    assert_eq!(
        unsafe { c_id(trained.as_ptr(), trained.len()) },
        0x8000_0001,
        "pinned dictID is reported"
    );
    assert_eq!(
        unsafe { r_id(trained.as_ptr(), trained.len()) },
        0x8000_0001,
        "pinned dictID is reported (Rust)"
    );
    assert_eq!(unsafe { c_id(raw.as_ptr(), raw.len()) }, 0, "raw dict => 0");
    assert_eq!(unsafe { r_id(raw.as_ptr(), raw.len()) }, 0, "raw dict => 0");
    // a valid dictionary has a well-defined header size
    let ha = unsafe { c_hs(trained.as_ptr(), trained.len()) };
    let hb = unsafe { r_hs(trained.as_ptr(), trained.len()) };
    assert_eq!(ha, hb);
    assert!(!is_error(ha) && ha >= 8 && ha <= trained.len());
}

/// ERRORS 281 — ZDICT_getDictHeaderSize error paths.
#[test]
fn err_dictb_headersize() {
    let (c_hs, r_hs) = unsafe { pair::<FnGetHeaderSize>("ZDICT_getDictHeaderSize") };
    let corp = corpus(Shape::Text, 0x281, 200, 64, 320);
    let (n, trained) = train_cover(8192, &corp, cover_params(200, 8, 3, 1), "headersize dict");
    assert!(!is_error(n));

    // dictSize <= 8
    for len in 0..=8usize {
        let a = unsafe { c_hs(trained.as_ptr(), len) };
        let b = unsafe { r_hs(trained.as_ptr(), len) };
        assert_eq!(a, b, "getDictHeaderSize len={len}");
        assert_eq!(a, as_err(E_DICTIONARY_CORRUPTED), "len={len}");
    }
    // wrong magic
    for i in 0..4usize {
        let mut bad = trained.clone();
        bad[i] ^= 0x5A;
        let a = unsafe { c_hs(bad.as_ptr(), bad.len()) };
        let b = unsafe { r_hs(bad.as_ptr(), bad.len()) };
        assert_eq!(a, b, "getDictHeaderSize bad magic byte {i}");
        assert_eq!(a, as_err(E_DICTIONARY_CORRUPTED), "bad magic byte {i}");
    }
    // truncated entropy tables => forwarded ZSTD_loadCEntropy error
    for len in [9usize, 12, 20, 40, 60].iter().copied() {
        if len >= trained.len() {
            continue;
        }
        let a = unsafe { c_hs(trained.as_ptr(), len) };
        let b = unsafe { r_hs(trained.as_ptr(), len) };
        assert_eq!(a, b, "getDictHeaderSize truncated len={len}");
        assert!(is_error(a), "truncated len={len} should fail");
    }
    // random buffers with a patched-in magic
    let mut rng = Rng::new(0x2811);
    for _ in 0..32 {
        let mut buf = gen(Shape::Random, rng.range(8, 200), &mut rng);
        buf[..4].copy_from_slice(&DICT_MAGIC);
        let a = unsafe { c_hs(buf.as_ptr(), buf.len()) };
        let b = unsafe { r_hs(buf.as_ptr(), buf.len()) };
        assert_eq!(a, b, "getDictHeaderSize fuzz len={}", buf.len());
    }
}

/// ERRORS 287..289 — ZDICT_trainFromBuffer failure modes.
#[test]
fn err_dictb_train() {
    let corp = corpus(Shape::Text, 0x287, 200, 64, 256);

    // --- 287: dictBufferCapacity < 256 --------------------------------
    for &cap in &[0usize, 1, 100, 255] {
        let ctx = format!("trainFromBuffer cap={cap}");
        let (n, _) = train_plain(cap, &corp, &ctx);
        assert_eq!(n, as_err(E_DSTSIZE_TOOSMALL), "{ctx}");
    }

    // --- 288: nbSamples == 0 -----------------------------------------
    let empty = Corpus::new();
    let (n, _) = train_plain(4096, &empty, "trainFromBuffer nbSamples=0");
    assert_eq!(n, as_err(E_SRCSIZE_WRONG), "nbSamples=0");

    // --- 289: fewer than 5 training samples (splitPoint defaults to 0.75,
    // so 6 samples only yield 4 training samples).
    for &nb in &[1usize, 2, 4, 6] {
        let few = corpus(Shape::Text, 0x289 + nb as u64, nb, 100, 300);
        let ctx = format!("trainFromBuffer nbSamples={nb}");
        let (n, _) = train_plain(4096, &few, &ctx);
        assert_eq!(n, as_err(E_SRCSIZE_WRONG), "{ctx}");
    }
    // 7 samples => 5 training samples => no longer an error
    let seven = corpus(Shape::Text, 0x2897, 7, 100, 300);
    let (n, _) = train_plain(4096, &seven, "trainFromBuffer nbSamples=7");
    assert!(!is_error(n), "7 samples: unexpected error {}", err_code(n));
}

/// CONFIGS 201 — the internal COVER_* helpers, divsufsort and divbwt.
#[test]
fn dictb_cover_internals() {
    // ---------------------------------------------- COVER_computeEpochs ---
    let (c_ce, r_ce) = unsafe { pair::<FnComputeEpochs>("COVER_computeEpochs") };
    for &maxDictSize in &[256u32, 8192, 112640] {
        for &nbDmers in &[1u32, 7, 100, 1000, 100_000, 1 << 20] {
            for &k in &[16u32, 50, 200, 1000, 2048] {
                for &passes in &[1u32, 4] {
                    let a = unsafe { c_ce(maxDictSize, nbDmers, k, passes) };
                    let b = unsafe { r_ce(maxDictSize, nbDmers, k, passes) };
                    assert_eq!(
                        a, b,
                        "COVER_computeEpochs({maxDictSize},{nbDmers},{k},{passes})"
                    );
                    assert!(a.num >= 1);
                }
            }
        }
    }

    // ----------------------------------------------------- COVER_sum ------
    let (c_sum, r_sum) = unsafe { pair::<FnCoverSum>("COVER_sum") };
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..64 {
        let n = rng.below(300);
        let sizes: Vec<usize> = (0..n).map(|_| rng.below(4096)).collect();
        let a = unsafe { c_sum(sizes.as_ptr(), n as c_uint) };
        let b = unsafe { r_sum(sizes.as_ptr(), n as c_uint) };
        assert_eq!(a, b, "COVER_sum n={n}");
        assert_eq!(a, sizes.iter().sum::<usize>(), "COVER_sum value n={n}");
    }
    // 0 samples
    let none: [usize; 0] = [];
    assert_eq!(unsafe { c_sum(none.as_ptr(), 0) }, unsafe { r_sum(none.as_ptr(), 0) });

    // -------------------------------------------- COVER_warnOnSmallCorpus -
    let (c_warn, r_warn) = unsafe { pair::<FnWarnSmall>("COVER_warnOnSmallCorpus") };
    for &(md, nd) in &[(256usize, 10usize), (8192, 100000), (1, 1), (112640, 0)] {
        unsafe {
            c_warn(md, nd, 0);
            r_warn(md, nd, 0);
        }
    }

    // ------------------------------- COVER_dictSelectionError/IsError/Free
    let (c_dse, r_dse) = unsafe { pair::<FnDictSelErr>("COVER_dictSelectionError") };
    let (c_dsi, r_dsi) = unsafe { pair::<FnDictSelIsErr>("COVER_dictSelectionIsError") };
    let (c_dsf, r_dsf) = unsafe { pair::<FnDictSelFree>("COVER_dictSelectionFree") };
    for &code in &[0usize, 1, 1234, as_err(E_GENERIC), as_err(E_DSTSIZE_TOOSMALL)] {
        let a = unsafe { c_dse(code) };
        let b = unsafe { r_dse(code) };
        assert!(a.dictContent.is_null() && b.dictContent.is_null());
        assert_eq!(a.dictSize, b.dictSize);
        assert_eq!(a.totalCompressedSize, b.totalCompressedSize);
        assert_eq!(a.totalCompressedSize, code);
        assert_eq!(unsafe { c_dsi(a) }, unsafe { r_dsi(b) }, "IsError({code:#x})");
        // freeing a NULL dictContent is a no-op
        unsafe {
            c_dsf(a);
            r_dsf(b);
        }
    }
    // a non-error selection with a non-NULL (non-owned) content pointer
    let mut probe = vec![7u8; 64];
    let sel = COVER_dictSelection_t {
        dictContent: probe.as_mut_ptr(),
        dictSize: probe.len(),
        totalCompressedSize: 4096,
    };
    assert_eq!(unsafe { c_dsi(sel) }, unsafe { r_dsi(sel) }, "IsError(valid)");
    assert_eq!(unsafe { c_dsi(sel) }, 0, "valid selection is not an error");

    // ------------------------------------------------- COVER_best_* -------
    let (c_bi, r_bi) = unsafe { pair::<FnBest1>("COVER_best_init") };
    let (c_bs, r_bs) = unsafe { pair::<FnBest1>("COVER_best_start") };
    let (c_bw, r_bw) = unsafe { pair::<FnBest1>("COVER_best_wait") };
    let (c_bd, r_bd) = unsafe { pair::<FnBest1>("COVER_best_destroy") };
    let (c_bf, r_bf) = unsafe { pair::<FnBestFinish>("COVER_best_finish") };
    {
        let mut cb = COVER_best_t {
            mutex: -1,
            cond: -1,
            liveJobs: 99,
            dict: std::ptr::null_mut(),
            dictSize: 99,
            parameters: cover_params(9, 9, 9, 9),
            compressedSize: 99,
        };
        let mut rb = cb;
        unsafe {
            c_bi(&mut cb);
            r_bi(&mut rb);
        }
        assert_eq!(cb.liveJobs, rb.liveJobs, "best_init liveJobs");
        assert_eq!(cb.dictSize, rb.dictSize, "best_init dictSize");
        assert_eq!(cb.compressedSize, rb.compressedSize, "best_init compressedSize");
        assert_eq!(cb.compressedSize, usize::MAX, "best_init compressedSize = -1");
        assert_eq!(cb.parameters, rb.parameters, "best_init parameters");
        assert_eq!(cb.parameters, ZDICT_cover_params_t::default());
        assert!(cb.dict.is_null() && rb.dict.is_null());

        // three "jobs": a good one, a worse one, then an error one
        let mut contents: Vec<Vec<u8>> = vec![vec![0xE1; 100], vec![0xE2; 200], vec![0xE3; 50]];
        let sizes = [5000usize, 9000, 4000];
        for (i, csz) in sizes.iter().copied().enumerate() {
            let p = cover_params(100 + i as u32, 6, 3, 1 + i as u32);
            let ptr = contents[i].as_mut_ptr();
            let len = contents[i].len();
            let sel =
                COVER_dictSelection_t { dictContent: ptr, dictSize: len, totalCompressedSize: csz };
            unsafe {
                c_bs(&mut cb);
                r_bs(&mut rb);
                assert_eq!(cb.liveJobs, rb.liveJobs, "best_start liveJobs");
                c_bf(&mut cb, p, sel);
                r_bf(&mut rb, p, sel);
            }
            assert_eq!(cb.liveJobs, rb.liveJobs, "best_finish liveJobs job={i}");
            assert_eq!(cb.dictSize, rb.dictSize, "best_finish dictSize job={i}");
            assert_eq!(
                cb.compressedSize, rb.compressedSize,
                "best_finish compressedSize job={i}"
            );
            assert_eq!(cb.parameters, rb.parameters, "best_finish parameters job={i}");
            assert_eq!(cb.dict.is_null(), rb.dict.is_null());
            if !cb.dict.is_null() {
                let ca = unsafe { std::slice::from_raw_parts(cb.dict as *const u8, cb.dictSize) };
                let ra = unsafe { std::slice::from_raw_parts(rb.dict as *const u8, rb.dictSize) };
                assert_bytes_eq(&format!("best_finish dict job={i}"), ca, ra);
            }
        }
        // best selection was the 4000-byte one with k=102
        assert_eq!(cb.compressedSize, 4000);
        assert_eq!(cb.parameters.k, 102);
        unsafe {
            c_bw(&mut cb);
            r_bw(&mut rb);
            c_bd(&mut cb);
            r_bd(&mut rb);
        }
        // NULL is accepted everywhere
        unsafe {
            c_bi(std::ptr::null_mut());
            r_bi(std::ptr::null_mut());
            c_bs(std::ptr::null_mut());
            r_bs(std::ptr::null_mut());
            c_bw(std::ptr::null_mut());
            r_bw(std::ptr::null_mut());
            c_bd(std::ptr::null_mut());
            r_bd(std::ptr::null_mut());
        }
    }

    // --------------------------- COVER_checkTotalCompressedSize / selectDict
    let (c_ct, r_ct) = unsafe { pair::<FnCheckTotal>("COVER_checkTotalCompressedSize") };
    let (c_sd, r_sd) = unsafe { pair::<FnSelectDict>("COVER_selectDict") };
    let corp = corpus(Shape::Text, 0xC0DE, 60, 64, 256);
    let mut offsets = corp.offsets();
    let nb = corp.sizes.len();
    let (dn, dict) = train_cover(2048, &corp, cover_params(200, 8, 3, 1), "internals dict");
    assert!(!is_error(dn));
    for &lvl in &[1i32, 3, 9] {
        for &sp in &[1.0f64, 0.75] {
            let mut p = cover_params(200, 8, lvl, 1);
            p.splitPoint = sp;
            let mut cdict = dict.clone();
            let mut rdict = dict.clone();
            let a = unsafe {
                c_ct(
                    p,
                    corp.sptr(),
                    corp.ptr(),
                    offsets.as_mut_ptr(),
                    nb / 2,
                    nb,
                    cdict.as_mut_ptr(),
                    cdict.len(),
                )
            };
            let b = unsafe {
                r_ct(
                    p,
                    corp.sptr(),
                    corp.ptr(),
                    offsets.as_mut_ptr(),
                    nb / 2,
                    nb,
                    rdict.as_mut_ptr(),
                    rdict.len(),
                )
            };
            assert_eq!(a, b, "COVER_checkTotalCompressedSize lvl={lvl} sp={sp}");
            assert!(!is_error(a), "checkTotalCompressedSize error {}", err_code(a));
        }
    }
    // COVER_selectDict, with and without shrinking
    let mut rng2 = Rng::new(0xC0DF);
    let content = gen(Shape::Text, 1024, &mut rng2);
    for &shrink in &[0u32, 1] {
        for &reg in &[0u32, 1, 5] {
            let mut p = cover_params(200, 8, 3, 1);
            p.shrinkDict = shrink;
            p.shrinkDictMaxRegression = reg;
            let mut ccontent = content.clone();
            let mut rcontent = content.clone();
            let a = unsafe {
                c_sd(
                    ccontent.as_mut_ptr(),
                    4096,
                    ccontent.len(),
                    corp.ptr(),
                    corp.sptr(),
                    nb as c_uint,
                    0,
                    nb,
                    p,
                    offsets.as_mut_ptr(),
                    0,
                )
            };
            let b = unsafe {
                r_sd(
                    rcontent.as_mut_ptr(),
                    4096,
                    rcontent.len(),
                    corp.ptr(),
                    corp.sptr(),
                    nb as c_uint,
                    0,
                    nb,
                    p,
                    offsets.as_mut_ptr(),
                    0,
                )
            };
            let ctx = format!("COVER_selectDict shrink={shrink} reg={reg}");
            assert_eq!(a.dictSize, b.dictSize, "{ctx}: dictSize");
            assert_eq!(a.totalCompressedSize, b.totalCompressedSize, "{ctx}: totalCompressedSize");
            assert_eq!(a.dictContent.is_null(), b.dictContent.is_null(), "{ctx}: NULL-ness");
            assert_eq!(unsafe { c_dsi(a) }, unsafe { r_dsi(b) }, "{ctx}: IsError");
            if !a.dictContent.is_null() && !is_error(a.totalCompressedSize) {
                let ca = unsafe { std::slice::from_raw_parts(a.dictContent, a.dictSize) };
                let ra = unsafe { std::slice::from_raw_parts(b.dictContent, b.dictSize) };
                assert_bytes_eq(&format!("{ctx}: dictionary"), ca, ra);
            }
            unsafe {
                c_dsf(a);
                r_dsf(b);
            }
        }
    }

    // ----------------------------------------------- divsufsort / divbwt --
    let (c_ds, r_ds) = unsafe { pair::<FnDivsufsort>("divsufsort") };
    let (c_bwt, r_bwt) = unsafe { pair::<FnDivbwt>("divbwt") };
    let mut rng3 = Rng::new(0x50A7);
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    inputs.push(("text 4096".into(), gen(Shape::Text, 4096, &mut rng3)));
    inputs.push(("zeros 4096".into(), vec![0u8; 4096]));
    for &shape in ALL_SHAPES {
        inputs.push((format!("{shape:?} 4096"), gen(shape, 4096, &mut rng3)));
    }
    for &n in &[0usize, 1, 2, 3, 7, 64, 1000] {
        inputs.push((format!("text {n}"), gen(Shape::Text, n, &mut rng3)));
        inputs.push((format!("zeros {n}"), vec![0u8; n]));
    }
    inputs.push(("all 255 2048".into(), vec![255u8; 2048]));
    inputs.push((
        "ramp 1024".into(),
        (0..1024u32).map(|i| (i % 256) as u8).collect(),
    ));

    for (name, t) in &inputs {
        let n = t.len();
        // divsufsort: compare the whole suffix array
        // identical sentinels: entries the algorithm never writes must stay
        // equal on both sides, so the whole array can be compared
        let mut csa = vec![-7i32; n.max(1)];
        let mut rsa = vec![-7i32; n.max(1)];
        let a = unsafe { c_ds(t.as_ptr(), csa.as_mut_ptr(), n as i32, 0) };
        let b = unsafe { r_ds(t.as_ptr(), rsa.as_mut_ptr(), n as i32, 0) };
        assert_eq!(a, b, "divsufsort({name}) return");
        assert_eq!(a, 0, "divsufsort({name}) should succeed");
        assert_eq!(csa[..n], rsa[..n], "divsufsort({name}) suffix array");

        // divbwt without secondary indexes
        let mut cu = vec![0xAAu8; n.max(1)];
        let mut ru = vec![0x55u8; n.max(1)];
        let mut ca = vec![-7i32; n + 1];
        let mut ra = vec![-7i32; n + 1];
        let a = unsafe {
            c_bwt(
                t.as_ptr(),
                cu.as_mut_ptr(),
                ca.as_mut_ptr(),
                n as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        let b = unsafe {
            r_bwt(
                t.as_ptr(),
                ru.as_mut_ptr(),
                ra.as_mut_ptr(),
                n as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(a, b, "divbwt({name}) primary index");
        assert_bytes_eq(&format!("divbwt({name}) output"), &cu[..n], &ru[..n]);
        assert_eq!(ca, ra, "divbwt({name}) scratch array");

        // divbwt with secondary indexes, and A == NULL (internally malloc'ed)
        let mut cnum = 0u8;
        let mut rnum = 0u8;
        let mut cidx = vec![-1i32; 256];
        let mut ridx = vec![-1i32; 256];
        let mut cu2 = vec![0xAAu8; n.max(1)];
        let mut ru2 = vec![0x55u8; n.max(1)];
        let a = unsafe {
            c_bwt(
                t.as_ptr(),
                cu2.as_mut_ptr(),
                std::ptr::null_mut(),
                n as i32,
                &mut cnum,
                cidx.as_mut_ptr(),
                0,
            )
        };
        let b = unsafe {
            r_bwt(
                t.as_ptr(),
                ru2.as_mut_ptr(),
                std::ptr::null_mut(),
                n as i32,
                &mut rnum,
                ridx.as_mut_ptr(),
                0,
            )
        };
        assert_eq!(a, b, "divbwt-idx({name}) primary index");
        assert_eq!(cnum, rnum, "divbwt-idx({name}) num_indexes");
        assert_eq!(cidx, ridx, "divbwt-idx({name}) indexes");
        assert_bytes_eq(&format!("divbwt-idx({name}) output"), &cu2[..n], &ru2[..n]);
    }

    // argument validation
    let t = vec![1u8, 2, 3, 4];
    let mut sa = vec![0i32; 4];
    assert_eq!(
        unsafe { c_ds(std::ptr::null(), sa.as_mut_ptr(), 4, 0) },
        unsafe { r_ds(std::ptr::null(), sa.as_mut_ptr(), 4, 0) },
        "divsufsort(NULL)"
    );
    assert_eq!(
        unsafe { c_ds(t.as_ptr(), std::ptr::null_mut(), 4, 0) },
        unsafe { r_ds(t.as_ptr(), std::ptr::null_mut(), 4, 0) },
        "divsufsort(SA=NULL)"
    );
    assert_eq!(
        unsafe { c_ds(t.as_ptr(), sa.as_mut_ptr(), -1, 0) },
        unsafe { r_ds(t.as_ptr(), sa.as_mut_ptr(), -1, 0) },
        "divsufsort(n<0)"
    );
    let mut u = vec![0u8; 4];
    assert_eq!(
        unsafe {
            c_bwt(
                std::ptr::null(),
                u.as_mut_ptr(),
                std::ptr::null_mut(),
                4,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        },
        unsafe {
            r_bwt(
                std::ptr::null(),
                u.as_mut_ptr(),
                std::ptr::null_mut(),
                4,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        },
        "divbwt(NULL)"
    );
    assert_eq!(
        unsafe {
            c_bwt(
                t.as_ptr(),
                u.as_mut_ptr(),
                std::ptr::null_mut(),
                -3,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        },
        unsafe {
            r_bwt(
                t.as_ptr(),
                u.as_mut_ptr(),
                std::ptr::null_mut(),
                -3,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        },
        "divbwt(n<0)"
    );
}
