//! Phase D: the dictionary BUILDER surface (`zdict.h`, `cover.c`, `fastcover.c`).
//!
//! Every row drives the C and the Rust `.so` through the identical FFI entry
//! point with byte-identical `samplesBuffer` / `samplesSizes[]` inputs and then
//! requires:
//!
//!   * the same numeric return value (so the same `ZSTD_ErrorCode` on failure),
//!   * the same `ZDICT_isError()` / `ZDICT_getErrorName()` verdict,
//!   * on success, a byte-identical dictionary, and
//!   * that the dictionary actually works: `ZSTD_compress_usingDict` must emit
//!     identical frames from both libraries and round trip through both
//!     decoders (including C-dict -> Rust-decoder cross checks).
//!
//! Determinism
//! -----------
//! Nothing in this API is timing dependent: the only `clock()` uses are the
//! `DISPLAYUPDATE` throttles which affect *stderr text only*, never the
//! dictionary. `dictID` "auto" mode is `XXH64(dictContent)`, not a PRNG
//! (zdict.c:879), so it is reproducible. This build has no `ZSTD_MULTITHREAD`
//! (no such define in c_src/CMakeLists.txt), so `nbThreads<=1` keeps the
//! `optimize*` search strictly sequential. Consequently *every* assertion here
//! is full byte equality — nothing has been weakened.
//!
//! `notificationLevel > 0` makes the C write progress text to stderr; that is
//! expected and harmless (the Rust port writes the same text), so a couple of
//! rows exercise levels 1..4 on deliberately tiny corpora.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod common;
use common::*;

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `cover.c:67` and `fastcover.c:54` keep a *file-static* `g_displayLevel` that
/// every entry point overwrites from `zParams.notificationLevel`, and that
/// `ZDICT_optimizeTrainFromBuffer_*` copies back into the caller's
/// `parameters->zParams.notificationLevel` (cover.c:1264, fastcover.c:726).
/// That makes those functions process-global-stateful. `cargo test` runs
/// `#[test]` functions on parallel threads, so every test that reaches the
/// COVER/FASTCOVER trainers takes this lock for its whole duration; otherwise a
/// concurrent test's `notificationLevel` leaks into this one's output and the
/// comparison becomes nondeterministic *within a single library*, not just
/// between the two.
fn display_level_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------- zdict types

/// `ZDICT_params_t` (zdict.h:214) — 12 bytes, passed in registers on SysV.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct ZDICT_params_t {
    compressionLevel: i32,
    notificationLevel: u32,
    dictID: u32,
}

/// `ZDICT_cover_params_t` (zdict.h:313) — 48 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct ZDICT_cover_params_t {
    k: u32,
    d: u32,
    steps: u32,
    nbThreads: u32,
    splitPoint: f64,
    shrinkDict: u32,
    shrinkDictMaxRegression: u32,
    zParams: ZDICT_params_t,
}

/// `ZDICT_fastCover_params_t` (zdict.h:324) — 56 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct ZDICT_fastCover_params_t {
    k: u32,
    d: u32,
    f: u32,
    steps: u32,
    nbThreads: u32,
    splitPoint: f64,
    accel: u32,
    shrinkDict: u32,
    shrinkDictMaxRegression: u32,
    zParams: ZDICT_params_t,
}

/// `ZDICT_legacy_params_t` (zdict.h:423) — 16 bytes, passed in two registers.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct ZDICT_legacy_params_t {
    selectivityLevel: u32,
    zParams: ZDICT_params_t,
}

/// `COVER_epoch_info_t` (cover.h:48) — 8 bytes, returned in RAX.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct COVER_epoch_info_t {
    num: u32,
    size: u32,
}

/// `COVER_dictSelection_t` (cover.h:58) — 24 bytes, returned via sret.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct COVER_dictSelection_t {
    dictContent: *mut u8,
    dictSize: usize,
    totalCompressedSize: usize,
}

/// `COVER_best_t` (cover.h:29). `ZSTD_pthread_mutex_t` / `ZSTD_pthread_cond_t`
/// are plain `int` because this build has no `ZSTD_MULTITHREAD`
/// (common/threading.h:124).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct COVER_best_t {
    mutex: i32,
    cond: i32,
    liveJobs: usize,
    dict: *mut c_void,
    dictSize: usize,
    parameters: ZDICT_cover_params_t,
    compressedSize: usize,
}

const ZDICT_DICTSIZE_MIN: usize = 256;
/// `ZDICT_MIN_SAMPLES_SIZE` = `ZDICT_CONTENTSIZE_MIN(128) * MINRATIO(4)` (zdict.c:17).
const ZDICT_MIN_SAMPLES_SIZE: usize = 512;
/// `FASTCOVER_MAX_ACCEL` / `DEFAULT_ACCEL` (fastcover.c:44,47).
const FASTCOVER_MAX_ACCEL: u32 = 10;
const DEFAULT_ACCEL: u32 = 1;

/// `size_t` encoding of a `ZSTD_ErrorCode`: `0 - code`.
fn err(code: i32) -> usize {
    (0usize).wrapping_sub(code as usize)
}

// ------------------------------------------------------------- fn signatures

type CCtx = *mut c_void;
type DCtx = *mut c_void;

type FnTrain = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, u32) -> usize;
type FnTrainCover =
    unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, u32, ZDICT_cover_params_t) -> usize;
type FnOptCover = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    u32,
    *mut ZDICT_cover_params_t,
) -> usize;
type FnTrainFast = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    u32,
    ZDICT_fastCover_params_t,
) -> usize;
type FnOptFast = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    u32,
    *mut ZDICT_fastCover_params_t,
) -> usize;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    *const usize,
    u32,
    ZDICT_legacy_params_t,
) -> usize;
type FnFinalize = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const usize,
    u32,
    ZDICT_params_t,
) -> usize;
type FnAddEntropy =
    unsafe extern "C" fn(*mut u8, usize, usize, *const u8, *const usize, u32) -> usize;

type FnGetDictID = unsafe extern "C" fn(*const u8, usize) -> u32;
type FnHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> u32;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

type FnCoverSum = unsafe extern "C" fn(*const usize, u32) -> usize;
type FnComputeEpochs = unsafe extern "C" fn(u32, u32, u32, u32) -> COVER_epoch_info_t;
type FnWarnSmall = unsafe extern "C" fn(usize, usize, i32);
type FnCheckTcs = unsafe extern "C" fn(
    ZDICT_cover_params_t,
    *const usize,
    *const u8,
    *mut usize,
    usize,
    usize,
    *mut u8,
    usize,
) -> usize;
type FnSelectDict = unsafe extern "C" fn(
    *mut u8,
    usize,
    usize,
    *const u8,
    *const usize,
    u32,
    usize,
    usize,
    ZDICT_cover_params_t,
    *mut usize,
    usize,
) -> COVER_dictSelection_t;
type FnDsError = unsafe extern "C" fn(usize) -> COVER_dictSelection_t;
type FnDsIsError = unsafe extern "C" fn(COVER_dictSelection_t) -> u32;
type FnDsFree = unsafe extern "C" fn(COVER_dictSelection_t);
type FnBest1 = unsafe extern "C" fn(*mut COVER_best_t);
type FnBestFinish =
    unsafe extern "C" fn(*mut COVER_best_t, ZDICT_cover_params_t, COVER_dictSelection_t);

type FnCreateCCtx = unsafe extern "C" fn() -> CCtx;
type FnFreeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type FnCreateDCtx = unsafe extern "C" fn() -> DCtx;
type FnFreeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type FnCompressUsingDict =
    unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, *const u8, usize, i32) -> usize;
type FnDecompressUsingDict =
    unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;

// ------------------------------------------------------------------- corpora

/// A realistic training corpus: the flat `samplesBuffer` plus `samplesSizes[]`.
struct Corpus {
    buf: Vec<u8>,
    sizes: Vec<usize>,
}

impl Corpus {
    fn nb(&self) -> u32 {
        self.sizes.len() as u32
    }
    fn buf_ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }
    fn sizes_ptr(&self) -> *const usize {
        self.sizes.as_ptr()
    }
    fn total(&self) -> usize {
        self.buf.len()
    }
    /// `offsets[i]` = start of sample i; `offsets[nb]` = total (cover.c:659).
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
    /// A few payloads to compression-test a produced dictionary with.
    fn probes(&self) -> Vec<&[u8]> {
        let mut out = Vec::new();
        let mut acc = 0usize;
        for (idx, &s) in self.sizes.iter().enumerate() {
            if idx % 7 == 0 && s > 0 && out.len() < 3 {
                out.push(&self.buf[acc..acc + s]);
            }
            acc += s;
        }
        if out.is_empty() && !self.buf.is_empty() {
            out.push(&self.buf[..]);
        }
        out
    }
}

/// Samples that share long substrings drawn from a common base blob — the case
/// a dictionary is *supposed* to help with, so the trainers find real segments.
fn corpus_shared(shape: Shape, sizes: &[usize], seed: u64) -> Corpus {
    let mut rng = Rng::new(seed);
    let base_len = 6144usize;
    let base = gen_shape(shape, base_len, &mut rng);
    let mut buf = Vec::with_capacity(sizes.iter().sum());
    for &sz in sizes {
        let off = rng.below(base_len);
        for j in 0..sz {
            if rng.below(96) == 0 {
                buf.push(rng.byte());
            } else {
                buf.push(base[(off + j) % base_len]);
            }
        }
    }
    Corpus {
        buf,
        sizes: sizes.to_vec(),
    }
}

/// Samples with nothing in common — a dictionary cannot help; exercises the
/// "no useful segment" / "pathological dataset" branches.
fn corpus_independent(shape: Shape, sizes: &[usize], seed: u64) -> Corpus {
    let mut rng = Rng::new(seed);
    let mut buf = Vec::with_capacity(sizes.iter().sum());
    for &sz in sizes {
        buf.extend_from_slice(&gen_shape(shape, sz, &mut rng));
    }
    Corpus {
        buf,
        sizes: sizes.to_vec(),
    }
}

fn sizes_uniform(n: usize, sz: usize) -> Vec<usize> {
    vec![sz; n]
}

/// Wildly varying sample sizes, including 0-byte and 1-byte samples.
fn sizes_varying(n: usize, lo: usize, hi: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|i| match i % 9 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => rng.range(lo, hi),
        })
        .collect()
}

// ------------------------------------------------------------------ checkers

/// Compare a `size_t` return that may encode an error: numeric equality plus
/// `ZDICT_isError` / `ZDICT_getErrorName` agreement.
fn assert_rc(tag: &str, cn: usize, rn: usize) -> bool {
    let i = impls();
    let (c_ise, r_ise) = i.pair::<FnIsError>("ZDICT_isError");
    let (c_nm, r_nm) = i.pair::<FnErrName>("ZDICT_getErrorName");
    assert_eq_dbg(&format!("{tag} / return"), cn, rn);
    let (ce, re) = unsafe { (c_ise(cn), r_ise(rn)) };
    assert_eq_dbg(&format!("{tag} / ZDICT_isError"), ce, re);
    unsafe {
        let a = CStr::from_ptr(c_nm(cn));
        let b = CStr::from_ptr(r_nm(rn));
        assert_eq_dbg(&format!("{tag} / ZDICT_getErrorName"), a, b);
    }
    ce == 0
}

/// The dictionary must be byte identical AND usable: identical frames out of
/// both libraries and a correct round trip through both decoders.
fn verify_dict(tag: &str, dict: &[u8], probes: &[&[u8]]) {
    if dict.is_empty() {
        return;
    }
    let i = impls();
    // ZDICT_getDictID must agree with the compressor's own reader
    // (ZSTD_getDictID_fromDict) about the dictionary just produced.
    {
        let (c_zid, r_zid) = i.pair::<FnGetDictID>("ZDICT_getDictID");
        let (c_sid, r_sid) = i.pair::<FnGetDictID>("ZSTD_getDictID_fromDict");
        unsafe {
            let a = c_zid(dict.as_ptr(), dict.len());
            let b = r_zid(dict.as_ptr(), dict.len());
            assert_eq_dbg(&format!("{tag} / ZDICT_getDictID"), a, b);
            let x = c_sid(dict.as_ptr(), dict.len());
            let y = r_sid(dict.as_ptr(), dict.len());
            assert_eq_dbg(&format!("{tag} / ZSTD_getDictID_fromDict"), x, y);
            assert_eq_dbg(&format!("{tag} / dictID agreement"), a, x);
        }
        let (c_hs, r_hs) = i.pair::<FnHeaderSize>("ZDICT_getDictHeaderSize");
        unsafe {
            let (p, q) = (c_hs(dict.as_ptr(), dict.len()), r_hs(dict.as_ptr(), dict.len()));
            assert_rc(&format!("{tag} / ZDICT_getDictHeaderSize"), p, q);
        }
    }
    let (c_cn, r_cn) = i.pair::<FnCreateCCtx>("ZSTD_createCCtx");
    let (c_cf, r_cf) = i.pair::<FnFreeCCtx>("ZSTD_freeCCtx");
    let (c_dn, r_dn) = i.pair::<FnCreateDCtx>("ZSTD_createDCtx");
    let (c_df, r_df) = i.pair::<FnFreeDCtx>("ZSTD_freeDCtx");
    let (c_cud, r_cud) = i.pair::<FnCompressUsingDict>("ZSTD_compress_usingDict");
    let (c_dud, r_dud) = i.pair::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
    let (c_bnd, _) = i.pair::<FnBound>("ZSTD_compressBound");

    unsafe {
        let cc = c_cn();
        let rc = r_cn();
        let cd = c_dn();
        let rd = r_dn();
        for (pi, p) in probes.iter().enumerate() {
            for &lvl in &[1i32, 3, 12] {
                let cap = c_bnd(p.len()) + 64;
                let mut cb = vec![0xA5u8; cap];
                let mut rb = vec![0x5Au8; cap];
                let cn = c_cud(
                    cc,
                    cb.as_mut_ptr(),
                    cap,
                    p.as_ptr(),
                    p.len(),
                    dict.as_ptr(),
                    dict.len(),
                    lvl,
                );
                let rn = r_cud(
                    rc,
                    rb.as_mut_ptr(),
                    cap,
                    p.as_ptr(),
                    p.len(),
                    dict.as_ptr(),
                    dict.len(),
                    lvl,
                );
                let t = format!("{tag} / compress_usingDict probe{pi} lvl{lvl}");
                assert_eq_dbg(&t, cn, rn);
                if cn > usize::MAX - 200 {
                    continue;
                }
                assert_bytes_eq(&t, &cb[..cn], &rb[..rn]);

                // cross round trip: C frame -> Rust decoder and vice versa
                let mut d1 = vec![0u8; p.len() + 64];
                let mut d2 = vec![0u8; p.len() + 64];
                let a = c_dud(
                    cd,
                    d1.as_mut_ptr(),
                    d1.len(),
                    rb.as_ptr(),
                    rn,
                    dict.as_ptr(),
                    dict.len(),
                );
                let b = r_dud(
                    rd,
                    d2.as_mut_ptr(),
                    d2.len(),
                    cb.as_ptr(),
                    cn,
                    dict.as_ptr(),
                    dict.len(),
                );
                assert_eq_dbg(&format!("{t} / decode rc"), a, b);
                assert_eq_dbg(&format!("{t} / decode len"), a, p.len());
                assert_bytes_eq(&format!("{t} / payload C-dec"), p, &d1[..a]);
                assert_bytes_eq(&format!("{t} / payload R-dec"), p, &d2[..b]);
            }
        }
        c_cf(cc);
        r_cf(rc);
        c_df(cd);
        r_df(rd);
    }
}

/// Bytes in the training subset that `COVER_ctx_init` / `FASTCOVER_ctx_init`
/// will actually look at: `splitPoint < 1` keeps only the first
/// `(unsigned)(nbSamples * splitPoint)` samples (cover.c:609, fastcover.c:322).
fn training_bytes(sizes: &[usize], split: f64) -> usize {
    let nb = sizes.len();
    let nb_train = if split < 1.0 {
        (nb as f64 * split) as usize
    } else {
        nb
    };
    sizes[..nb_train.min(nb)].iter().sum()
}

/// `false` for inputs that make **the C itself** die with SIGFPE, which this
/// suite must therefore never generate.
///
/// `COVER_ctx_init`/`FASTCOVER_ctx_init` validate the *total* sample size
/// against `MAX(d, sizeof(U64))` (cover.c:614, fastcover.c:328) but then derive
/// the dmer count from the *training* subset:
/// `suffixSize = trainingSamplesSize - MAX(d, 8) + 1` (cover.c:642) /
/// `nbDmers = ...` (fastcover.c:359). When `splitPoint < 1` shrinks the training
/// subset to exactly `MAX(d,8) - 1` bytes that count becomes 0, and
/// `COVER_computeEpochs` then evaluates `nbDmers / epochs.size` with
/// `epochs.size == 0` (cover.c:719) — an integer division by zero. Verified
/// empirically: `ZDICT_trainFromBuffer` with 10 one-byte samples aborts the
/// process with SIGFPE in `c_src/build/libzstd.so`. The Rust port reproduces the
/// C faithfully, so the fix is to not ask either library the question.
fn cover_input_is_safe(sizes: &[usize], split: f64, d: u32) -> bool {
    let need = core::cmp::max(d as usize, 8);
    let nb = sizes.len();
    let (nb_train, nb_test) = if split < 1.0 {
        let t = ((nb as f64 * split) as usize).min(nb);
        (t, nb - t)
    } else {
        (nb, nb)
    };
    let total: usize = sizes.iter().sum();
    // these are all rejected before the dmer arithmetic runs
    if nb_train < 5 || nb_test < 1 || total < need {
        return true;
    }
    training_bytes(sizes, split) >= need
}

/// `dictBufferCapacity` slack so that any (identical) over-write by either
/// library lands in scratch space instead of corrupting the allocator.
const SLACK: usize = 8192;

fn dict_buf(cap: usize, fill: u8) -> Vec<u8> {
    vec![fill; cap + SLACK]
}

// ============================================================== 1. helpers

/// `ZDICT_isError`, `ZDICT_getErrorName`, `ZDICT_getDictID` and
/// `ZDICT_getDictHeaderSize` over valid, truncated, malformed and
/// non-dictionary buffers.
#[test]
fn dict_helpers_match() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_ise, r_ise) = i.pair::<FnIsError>("ZDICT_isError");
    let (c_nm, r_nm) = i.pair::<FnErrName>("ZDICT_getErrorName");
    let (c_id, r_id) = i.pair::<FnGetDictID>("ZDICT_getDictID");
    let (c_hs, r_hs) = i.pair::<FnHeaderSize>("ZDICT_getDictHeaderSize");

    // every plausible error code plus the boundary values
    let mut codes: Vec<usize> = (0usize..=200).collect();
    for c in 0..=200i32 {
        codes.push(err(c));
    }
    codes.extend([usize::MAX, usize::MAX - 1, usize::MAX / 2, 1 << 40]);
    for rc in codes {
        unsafe {
            assert_eq_dbg(&format!("ZDICT_isError({rc})"), c_ise(rc), r_ise(rc));
            let a = CStr::from_ptr(c_nm(rc));
            let b = CStr::from_ptr(r_nm(rc));
            assert_eq_dbg(&format!("ZDICT_getErrorName({rc})"), a, b);
        }
    }

    // ---- getDictID / getDictHeaderSize on synthetic buffers
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(Vec::new());
    for n in 1..=16usize {
        // magic + dictID prefix, truncated at every length
        let mut v = Vec::new();
        v.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        v.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        v.extend_from_slice(&[0u8; 16]);
        v.truncate(n);
        cases.push(v);
    }
    // wrong magic, right length
    let mut wrong = vec![0u8; 64];
    wrong[..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
    cases.push(wrong);
    // right magic, garbage entropy tables
    let mut rng = Rng::new(0x11FE_0001);
    for len in [9usize, 16, 64, 256] {
        let mut v = Vec::with_capacity(len);
        v.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        v.extend_from_slice(&1234u32.to_le_bytes());
        while v.len() < len {
            v.push(rng.byte());
        }
        cases.push(v);
    }
    // dictID edge values (0, 1, 32767, 32768, 2^31, u32::MAX)
    for id in [0u32, 1, 32767, 32768, 1 << 31, u32::MAX] {
        let mut v = Vec::new();
        v.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        v.extend_from_slice(&id.to_le_bytes());
        v.resize(40, 0);
        cases.push(v);
    }
    // a genuinely valid dictionary, plus every truncation of its first 40 bytes
    let corpus = corpus_shared(Shape::Tabular, &sizes_uniform(24, 1500), 0xD1C7_0001);
    {
        let (c_tr, _) = i.pair::<FnTrain>("ZDICT_trainFromBuffer");
        let cap = 4096;
        let mut db = dict_buf(cap, 0);
        let n = unsafe {
            c_tr(
                db.as_mut_ptr(),
                cap,
                corpus.buf_ptr(),
                corpus.sizes_ptr(),
                corpus.nb(),
            )
        };
        assert!(n < usize::MAX - 200, "training the fixture dict failed: {n}");
        let full = db[..n].to_vec();
        for t in [0usize, 1, 7, 8, 9, 12, 40, 64, n / 2, n - 1, n] {
            if t <= full.len() {
                cases.push(full[..t].to_vec());
            }
        }
    }

    for (ci, buf) in cases.iter().enumerate() {
        for &sz in &[0usize, 1, 4, 7, 8, 9, buf.len()] {
            if sz > buf.len() {
                continue;
            }
            let p = buf.as_ptr();
            unsafe {
                assert_eq_dbg(
                    &format!("ZDICT_getDictID(case{ci}, size={sz})"),
                    c_id(p, sz),
                    r_id(p, sz),
                );
                let (a, b) = (c_hs(p, sz), r_hs(p, sz));
                assert_rc(&format!("ZDICT_getDictHeaderSize(case{ci}, size={sz})"), a, b);
            }
        }
    }
}

// ========================================================== 2. COVER helpers

/// `COVER_sum`, `COVER_computeEpochs` and `COVER_warnOnSmallCorpus`.
///
/// `COVER_computeEpochs` divides by `k`, `passes` and by `epochs.size`, so
/// `k >= 1`, `passes >= 1` and `nbDmers >= 1` are the only inputs the C can
/// survive; those are the only ones the callers ever produce (cover.c:734).
#[test]
fn cover_sum_and_epochs_match() {
    let i = impls();
    let (c_sum, r_sum) = i.pair::<FnCoverSum>("COVER_sum");
    let (c_ep, r_ep) = i.pair::<FnComputeEpochs>("COVER_computeEpochs");
    let (c_warn, r_warn) = i.pair::<FnWarnSmall>("COVER_warnOnSmallCorpus");

    // COVER_sum, including nbSamples == 0 and huge values that overflow
    let tables: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![1],
        vec![0, 0, 0, 0, 0],
        vec![1, 2, 3, 4, 5],
        vec![usize::MAX, 1],
        vec![usize::MAX / 2, usize::MAX / 2, 4],
        (0..500usize).map(|x| x * 3).collect(),
    ];
    for (ti, t) in tables.iter().enumerate() {
        for nb in [0u32, 1, 2, t.len() as u32] {
            if nb as usize > t.len() {
                continue;
            }
            unsafe {
                assert_eq_dbg(
                    &format!("COVER_sum(table{ti}, nb={nb})"),
                    c_sum(t.as_ptr(), nb),
                    r_sum(t.as_ptr(), nb),
                );
            }
        }
    }

    for &maxDictSize in &[0u32, 1, 255, 256, 1024, 110 * 1024, u32::MAX] {
        for &nbDmers in &[1u32, 2, 9, 10, 1000, 100_000, u32::MAX] {
            for &k in &[1u32, 6, 8, 16, 50, 1000, 2000, u32::MAX] {
                for &passes in &[1u32, 4, 40] {
                    let (a, b) = unsafe {
                        (
                            c_ep(maxDictSize, nbDmers, k, passes),
                            r_ep(maxDictSize, nbDmers, k, passes),
                        )
                    };
                    assert_eq_dbg(
                        &format!("COVER_computeEpochs({maxDictSize},{nbDmers},{k},{passes})"),
                        a,
                        b,
                    );
                }
            }
        }
    }

    // void function: just require both survive the same inputs. displayLevel 0
    // keeps stderr clean; level 1 is exercised too (it prints the warning).
    for &(md, nd) in &[
        (0usize, 0usize),
        (1, 0),
        (1, 10),
        (256, 100),
        (256, 2560),
        (110 * 1024, 1),
        (usize::MAX, usize::MAX),
    ] {
        unsafe {
            c_warn(md, nd, 0);
            r_warn(md, nd, 0);
        }
    }
    unsafe {
        c_warn(1024, 100, 1);
        r_warn(1024, 100, 1);
    }
}

// ================================================== 3. ZDICT_trainFromBuffer

/// The headline entry point. It forwards to
/// `ZDICT_optimizeTrainFromBuffer_fastCover(d=8, steps=4, f=20, accel=1)` with
/// `splitPoint` defaulting to 0.75, so it needs >= 7 samples (5 training + 1
/// testing after the split).
#[test]
fn train_from_buffer_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrain>("ZDICT_trainFromBuffer");

    struct Row {
        name: &'static str,
        corpus: Corpus,
        caps: Vec<usize>,
    }
    let rows = vec![
        Row {
            name: "shared-tabular-50x1500",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(50, 1500), 0x7A01),
            caps: vec![256, 1024, 8192, 32768],
        },
        Row {
            // 500 samples / 150 KB, plus the zstd CLI's default 110 KB dict size
            name: "shared-skewed-500x300",
            corpus: corpus_shared(Shape::SkewedText, &sizes_uniform(500, 300), 0x7A02),
            caps: vec![256, 4096, 112_640],
        },
        Row {
            // 50 samples / 200 KB — the "large sample" end of the range
            name: "shared-tabular-50x4000",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(50, 4000), 0x7A20),
            caps: vec![256, 16384, 112_640],
        },
        Row {
            name: "shared-repetitive-varying",
            corpus: corpus_shared(Shape::Repetitive, &sizes_varying(60, 10, 3000, 0x7A03), 0x7A13),
            caps: vec![256, 2048],
        },
        Row {
            name: "independent-random-30x2000",
            corpus: corpus_independent(Shape::Random, &sizes_uniform(30, 2000), 0x7A04),
            caps: vec![256, 4096],
        },
        Row {
            name: "constant-20x1000",
            corpus: corpus_independent(Shape::Constant, &sizes_uniform(20, 1000), 0x7A05),
            caps: vec![256, 1024],
        },
        Row {
            name: "counter-8x600",
            corpus: corpus_independent(Shape::Counter, &sizes_uniform(8, 600), 0x7A06),
            caps: vec![256, 1024],
        },
        Row {
            name: "two-phase-12x900",
            corpus: corpus_shared(Shape::TwoPhase, &sizes_uniform(12, 900), 0x7A07),
            caps: vec![256, 900],
        },
        Row {
            name: "sparse-10x2000",
            corpus: corpus_shared(Shape::Sparse, &sizes_uniform(10, 2000), 0x7A08),
            caps: vec![256, 1500],
        },
        // exactly at / just below the >= 7 sample requirement
        Row {
            name: "7-samples",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(7, 800), 0x7A09),
            caps: vec![256, 1024],
        },
        Row {
            name: "6-samples",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(6, 800), 0x7A0A),
            caps: vec![256],
        },
        Row {
            name: "5-samples",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(5, 800), 0x7A0B),
            caps: vec![256],
        },
        Row {
            name: "1-sample",
            corpus: corpus_shared(Shape::Tabular, &sizes_uniform(1, 4000), 0x7A0C),
            caps: vec![256],
        },
        Row {
            name: "0-samples",
            corpus: Corpus {
                buf: Vec::new(),
                sizes: Vec::new(),
            },
            caps: vec![0, 256],
        },
        Row {
            name: "all-empty-samples",
            corpus: Corpus {
                buf: Vec::new(),
                sizes: vec![0; 20],
            },
            caps: vec![256],
        },
        Row {
            name: "one-byte-samples",
            corpus: corpus_shared(Shape::SkewedText, &sizes_uniform(20, 1), 0x7A0D),
            caps: vec![256],
        },
        // total corpus below / above ZDICT_MIN_SAMPLES_SIZE (512)
        Row {
            name: "total-below-min",
            corpus: corpus_shared(Shape::SkewedText, &sizes_uniform(10, 40), 0x7A0E),
            caps: vec![256],
        },
        Row {
            name: "total-above-min",
            corpus: corpus_shared(Shape::SkewedText, &sizes_uniform(10, 60), 0x7A0F),
            caps: vec![256],
        },
    ];

    for row in &rows {
        // dictBufferCapacity sweep including everything below ZDICT_DICTSIZE_MIN
        let mut caps = vec![0usize, 1, 8, 64, 128, 255];
        caps.extend(row.caps.iter().copied());
        for cap in caps {
            let mut cb = dict_buf(cap, 0xC1);
            let mut rb = dict_buf(cap, 0xC1);
            let cn = unsafe {
                c_tr(
                    cb.as_mut_ptr(),
                    cap,
                    row.corpus.buf_ptr(),
                    row.corpus.sizes_ptr(),
                    row.corpus.nb(),
                )
            };
            let rn = unsafe {
                r_tr(
                    rb.as_mut_ptr(),
                    cap,
                    row.corpus.buf_ptr(),
                    row.corpus.sizes_ptr(),
                    row.corpus.nb(),
                )
            };
            let tag = format!(
                "ZDICT_trainFromBuffer[{}] nb={} total={} cap={cap}",
                row.name,
                row.corpus.nb(),
                row.corpus.total()
            );
            if !assert_rc(&tag, cn, rn) {
                // documented rejections, asserted against the C ground truth
                if row.corpus.nb() == 0 {
                    assert_eq_dbg(&format!("{tag} / srcSize_wrong"), cn, err(72));
                } else if cap < ZDICT_DICTSIZE_MIN {
                    assert_eq_dbg(&format!("{tag} / dstSize_tooSmall"), cn, err(70));
                }
                continue;
            }
            assert!(cn <= cap, "{tag}: dict {cn} exceeds capacity {cap}");
            assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            verify_dict(&tag, &cb[..cn], &row.corpus.probes());
        }
    }
}

// ============================================ 4. ZDICT_trainFromBuffer_legacy

/// The pre-COVER trainer (divsufsort + `ZDICT_analyzePos`). Note the C returns
/// **0, not an error**, when the corpus is below `ZDICT_MIN_SAMPLES_SIZE`
/// (zdict.c:1091) — replicated verbatim.
#[test]
fn train_from_buffer_legacy_matches() {
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");

    let corpora = vec![
        (
            "shared-tabular",
            corpus_shared(Shape::Tabular, &sizes_uniform(40, 1200), 0x1E01),
        ),
        (
            "shared-skewed-500",
            corpus_shared(Shape::SkewedText, &sizes_uniform(500, 120), 0x1E02),
        ),
        (
            "shared-varying",
            corpus_shared(Shape::Repetitive, &sizes_varying(50, 5, 2000, 0x1E03), 0x1E13),
        ),
        (
            "independent-random",
            corpus_independent(Shape::Random, &sizes_uniform(20, 1500), 0x1E04),
        ),
        (
            "constant",
            corpus_independent(Shape::Constant, &sizes_uniform(10, 2000), 0x1E05),
        ),
        (
            "below-min-samples",
            corpus_shared(Shape::Tabular, &sizes_uniform(5, 100), 0x1E06),
        ),
        (
            "exactly-min-samples",
            corpus_shared(Shape::Tabular, &sizes_uniform(8, 64), 0x1E07),
        ),
        (
            "single-sample",
            corpus_shared(Shape::Tabular, &sizes_uniform(1, 20000), 0x1E08),
        ),
        (
            "zero-samples",
            Corpus {
                buf: Vec::new(),
                sizes: Vec::new(),
            },
        ),
        (
            "all-zero-sized",
            Corpus {
                buf: Vec::new(),
                sizes: vec![0; 30],
            },
        ),
    ];

    let mut cfgs: Vec<ZDICT_legacy_params_t> = Vec::new();
    for &sel in &[0u32, 1, 2, 5, 9, 10, 30, 31, 40] {
        cfgs.push(ZDICT_legacy_params_t {
            selectivityLevel: sel,
            zParams: ZDICT_params_t::default(),
        });
    }
    for &lvl in &[-5i32, 0, 1, 3, 19] {
        cfgs.push(ZDICT_legacy_params_t {
            selectivityLevel: 9,
            zParams: ZDICT_params_t {
                compressionLevel: lvl,
                notificationLevel: 0,
                dictID: 0,
            },
        });
    }
    for &id in &[0u32, 1, 32767, 32768, 0x8000_0000, u32::MAX] {
        cfgs.push(ZDICT_legacy_params_t {
            selectivityLevel: 9,
            zParams: ZDICT_params_t {
                compressionLevel: 0,
                notificationLevel: 0,
                dictID: id,
            },
        });
    }

    for (cname, corpus) in &corpora {
        for (ci, p) in cfgs.iter().enumerate() {
            // only sweep the capacity axis on the first config to bound runtime
            let caps: Vec<usize> = if ci == 0 {
                vec![0, 1, 128, 255, 256, 1024, 16384]
            } else {
                vec![256, 4096]
            };
            for cap in caps {
                let mut cb = dict_buf(cap, 0x3C);
                let mut rb = dict_buf(cap, 0x3C);
                let cn = unsafe {
                    c_tr(
                        cb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let rn = unsafe {
                    r_tr(
                        rb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let tag = format!("ZDICT_trainFromBuffer_legacy[{cname}] p={p:?} cap={cap}");
                if !assert_rc(&tag, cn, rn) {
                    if cap < ZDICT_DICTSIZE_MIN && corpus.total() >= ZDICT_MIN_SAMPLES_SIZE {
                        assert_eq_dbg(&format!("{tag} / dstSize_tooSmall"), cn, err(70));
                    }
                    continue;
                }
                if corpus.total() < ZDICT_MIN_SAMPLES_SIZE {
                    // zdict.c:1091 "not enough content => no dictionary"
                    assert_eq_dbg(&format!("{tag} / no dictionary"), cn, 0);
                    continue;
                }
                assert!(cn <= cap, "{tag}: dict {cn} > cap {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cn > 0 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

// ============================================= 5/6. ZDICT_*_cover

fn cover_params(k: u32, d: u32) -> ZDICT_cover_params_t {
    ZDICT_cover_params_t {
        k,
        d,
        steps: 0,
        nbThreads: 1,
        splitPoint: 1.0,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: ZDICT_params_t::default(),
    }
}

/// `ZDICT_trainFromBuffer_cover` over the whole `ZDICT_cover_params_t` axis.
/// Note the C *overwrites* `splitPoint` with 1.0 before validating
/// (cover.c:787), so out-of-range splitPoints are silently accepted here.
#[test]
fn train_from_buffer_cover_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");

    let corpora = vec![
        (
            "shared-tabular-40x1200",
            corpus_shared(Shape::Tabular, &sizes_uniform(40, 1200), 0xC001),
        ),
        (
            "shared-skewed-200x250",
            corpus_shared(Shape::SkewedText, &sizes_uniform(200, 250), 0xC002),
        ),
        (
            "shared-varying",
            corpus_shared(Shape::Repetitive, &sizes_varying(40, 8, 2500, 0xC003), 0xC013),
        ),
        (
            "independent-random-20x1200",
            corpus_independent(Shape::Random, &sizes_uniform(20, 1200), 0xC004),
        ),
        (
            "constant-10x1500",
            corpus_independent(Shape::Constant, &sizes_uniform(10, 1500), 0xC005),
        ),
        (
            "sparse-10x1500",
            corpus_shared(Shape::Sparse, &sizes_uniform(10, 1500), 0xC006),
        ),
    ];

    let mut cfgs: Vec<ZDICT_cover_params_t> = Vec::new();
    // k / d grid; d must be <= k, cover.c also allows d values other than 6/8
    for &d in &[6u32, 8] {
        for &k in &[8u32, 16, 50, 200, 1024] {
            cfgs.push(cover_params(k, d));
        }
    }
    // d values COVER (unlike FASTCOVER) does not restrict
    for &d in &[1u32, 2, 7, 16, 32] {
        cfgs.push(cover_params(64, d));
    }
    // splitPoint is force-overwritten -> even out-of-range values must succeed
    for &sp in &[0.0f64, 0.5, 1.0, 1.5, -1.0] {
        let mut p = cover_params(64, 8);
        p.splitPoint = sp;
        cfgs.push(p);
    }
    // steps / nbThreads are only read by the optimize* variant
    for &(steps, nb) in &[(0u32, 0u32), (1, 1), (40, 1)] {
        let mut p = cover_params(64, 8);
        p.steps = steps;
        p.nbThreads = nb;
        cfgs.push(p);
    }
    // shrinkDict / shrinkDictMaxRegression are ignored by the non-optimize path
    for &(sd, mr) in &[(0u32, 0u32), (1, 0), (1, 1), (1, 100)] {
        let mut p = cover_params(64, 8);
        p.shrinkDict = sd;
        p.shrinkDictMaxRegression = mr;
        cfgs.push(p);
    }
    // zParams: compressionLevel / dictID
    for &lvl in &[-5i32, 0, 1, 3, 19] {
        let mut p = cover_params(64, 8);
        p.zParams.compressionLevel = lvl;
        cfgs.push(p);
    }
    for &id in &[0u32, 1, 32767, 32768, 0x8000_0000, u32::MAX] {
        let mut p = cover_params(64, 8);
        p.zParams.dictID = id;
        cfgs.push(p);
    }

    for (cname, corpus) in &corpora {
        for p in &cfgs {
            for cap in [256usize, 2048, 40_960] {
                let mut cb = dict_buf(cap, 0x5A);
                let mut rb = dict_buf(cap, 0x5A);
                let cn = unsafe {
                    c_tr(
                        cb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let rn = unsafe {
                    r_tr(
                        rb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let tag = format!("ZDICT_trainFromBuffer_cover[{cname}] p={p:?} cap={cap}");
                if !assert_rc(&tag, cn, rn) {
                    continue;
                }
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cap >= 2048 && p.zParams.dictID == 0 && p.zParams.compressionLevel == 0 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

/// Every documented COVER rejection path, checked for identical numeric error
/// codes: `COVER_checkParameters` (k=0, d=0, d>k, k>maxDictSize),
/// `nbSamples == 0`, `dictBufferCapacity < ZDICT_DICTSIZE_MIN`, and the
/// `COVER_ctx_init` sample-count / sample-size minimums.
#[test]
fn train_from_buffer_cover_errors() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");

    let good = corpus_shared(Shape::Tabular, &sizes_uniform(20, 800), 0xE001);
    let empty = Corpus {
        buf: Vec::new(),
        sizes: Vec::new(),
    };
    let tiny_total = corpus_shared(Shape::Tabular, &sizes_uniform(8, 1), 0xE002); // total 8
    let sub8_total = corpus_shared(Shape::Tabular, &sizes_uniform(7, 1), 0xE003); // total 7 < 8
    let four_samples = corpus_shared(Shape::Tabular, &sizes_uniform(4, 500), 0xE004);
    let five_samples = corpus_shared(Shape::Tabular, &sizes_uniform(5, 500), 0xE005);
    let zero_sized = Corpus {
        buf: Vec::new(),
        sizes: vec![0; 10],
    };

    struct Case<'a> {
        name: &'a str,
        corpus: &'a Corpus,
        cap: usize,
        p: ZDICT_cover_params_t,
        expect: Option<usize>,
    }

    let mut cases: Vec<Case> = Vec::new();

    // ---- COVER_checkParameters rejections -> parameter_outOfBound (42)
    for (nm, k, d) in [
        ("k=0", 0u32, 8u32),
        ("d=0", 64, 0),
        ("k=0,d=0", 0, 0),
        ("d>k", 8, 9),
        ("k>maxDictSize", 4096, 8),
        ("k=maxDictSize+1", 257, 8),
    ] {
        cases.push(Case {
            name: nm,
            corpus: &good,
            cap: 256,
            p: cover_params(k, d),
            expect: Some(err(42)),
        });
    }
    // k == maxDictSize is *accepted* (the check is `k > maxDictSize`)
    cases.push(Case {
        name: "k==maxDictSize",
        corpus: &good,
        cap: 256,
        p: cover_params(256, 8),
        expect: None,
    });

    // ---- nbSamples == 0 -> srcSize_wrong (72). Checked *after* the param
    // check but *before* the capacity check (cover.c:795).
    cases.push(Case {
        name: "nbSamples=0",
        corpus: &empty,
        cap: 4096,
        p: cover_params(64, 8),
        expect: Some(err(72)),
    });
    cases.push(Case {
        name: "nbSamples=0,cap=0",
        corpus: &empty,
        cap: 0,
        p: cover_params(0, 0),
        // params are validated first, and k=0 with maxDictSize=0 fails there
        expect: Some(err(42)),
    });

    // ---- capacity below ZDICT_DICTSIZE_MIN -> dstSize_tooSmall (70).
    // k must be <= cap to get past COVER_checkParameters first.
    for cap in [8usize, 16, 64, 128, 255] {
        cases.push(Case {
            name: "cap<DICTSIZE_MIN",
            corpus: &good,
            cap,
            p: cover_params(8, 8),
            expect: Some(err(70)),
        });
    }

    // ---- COVER_ctx_init minimums -> srcSize_wrong (72)
    cases.push(Case {
        name: "total<8",
        corpus: &sub8_total,
        cap: 256,
        p: cover_params(8, 8),
        expect: Some(err(72)),
    });
    cases.push(Case {
        name: "total==8",
        corpus: &tiny_total,
        cap: 256,
        p: cover_params(8, 8),
        expect: None,
    });
    cases.push(Case {
        name: "4-samples",
        corpus: &four_samples,
        cap: 256,
        p: cover_params(64, 8),
        expect: Some(err(72)),
    });
    cases.push(Case {
        name: "5-samples",
        corpus: &five_samples,
        cap: 256,
        p: cover_params(64, 8),
        expect: None,
    });
    cases.push(Case {
        name: "all-zero-sized",
        corpus: &zero_sized,
        cap: 256,
        p: cover_params(64, 8),
        expect: Some(err(72)),
    });

    for c in &cases {
        let mut cb = dict_buf(c.cap, 0x77);
        let mut rb = dict_buf(c.cap, 0x77);
        let cn = unsafe {
            c_tr(
                cb.as_mut_ptr(),
                c.cap,
                c.corpus.buf_ptr(),
                c.corpus.sizes_ptr(),
                c.corpus.nb(),
                c.p,
            )
        };
        let rn = unsafe {
            r_tr(
                rb.as_mut_ptr(),
                c.cap,
                c.corpus.buf_ptr(),
                c.corpus.sizes_ptr(),
                c.corpus.nb(),
                c.p,
            )
        };
        let tag = format!(
            "cover-error[{}] cap={} k={} d={}",
            c.name, c.cap, c.p.k, c.p.d
        );
        let ok = assert_rc(&tag, cn, rn);
        match c.expect {
            Some(e) => {
                assert!(!ok, "{tag}: expected error {e:#x}, got success {cn}");
                assert_eq_dbg(&format!("{tag} / exact code"), cn, e);
            }
            None => assert!(ok, "{tag}: expected success, got error {cn:#x}"),
        }
        if ok {
            assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
        }
    }
}

/// `ZDICT_optimizeTrainFromBuffer_cover`: the dictionary *and* the
/// `*parameters` written back must match. `steps` is kept tiny — the default
/// (40) would run 80 full dictionary builds per row.
#[test]
fn optimize_train_from_buffer_cover_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");

    let corpora = vec![
        (
            "shared-tabular-20x900",
            corpus_shared(Shape::Tabular, &sizes_uniform(20, 900), 0xF001),
        ),
        (
            "shared-skewed-60x200",
            corpus_shared(Shape::SkewedText, &sizes_uniform(60, 200), 0xF002),
        ),
        (
            "independent-random-12x800",
            corpus_independent(Shape::Random, &sizes_uniform(12, 800), 0xF003),
        ),
        (
            "shared-varying-20",
            corpus_shared(Shape::Repetitive, &sizes_varying(20, 20, 1200, 0xF004), 0xF014),
        ),
    ];

    let mut cfgs: Vec<ZDICT_cover_params_t> = Vec::new();
    // d fixed (single context) / d auto (checks both 6 and 8)
    for &d in &[0u32, 6, 8] {
        let mut p = cover_params(0, d);
        p.steps = 2;
        cfgs.push(p);
    }
    // explicit k pins the k loop to a single value
    for &k in &[16u32, 50, 200, 2000] {
        let mut p = cover_params(k, 8);
        p.steps = 2;
        cfgs.push(p);
    }
    // splitPoint axis: <=0 -> default 1.0, >1 -> parameter_outOfBound, NaN
    // slips past both comparisons and therefore behaves like 1.0.
    for &sp in &[0.0f64, -1.0, 0.25, 0.5, 1.0, 1.5, f64::NAN] {
        let mut p = cover_params(64, 8);
        p.steps = 1;
        p.splitPoint = sp;
        cfgs.push(p);
    }
    // steps edge values
    for &steps in &[0u32, 1, 3] {
        let mut p = cover_params(1000, 8);
        p.steps = steps;
        cfgs.push(p);
    }
    // nbThreads 0 and 1 are both single-threaded in a non-ZSTD_MULTITHREAD build
    for &nbt in &[0u32, 1] {
        let mut p = cover_params(64, 8);
        p.steps = 1;
        p.nbThreads = nbt;
        cfgs.push(p);
    }
    // shrinkDict is force-zeroed by the optimizer (cover.c:1184) -> no effect
    for &(sd, mr) in &[(1u32, 0u32), (1, 50)] {
        let mut p = cover_params(64, 8);
        p.steps = 1;
        p.shrinkDict = sd;
        p.shrinkDictMaxRegression = mr;
        cfgs.push(p);
    }
    // zParams
    for &lvl in &[-3i32, 0, 1, 19] {
        let mut p = cover_params(64, 8);
        p.steps = 1;
        p.zParams.compressionLevel = lvl;
        cfgs.push(p);
    }
    for &id in &[0u32, 7, 32768, u32::MAX] {
        let mut p = cover_params(64, 8);
        p.steps = 1;
        p.zParams.dictID = id;
        cfgs.push(p);
    }
    // k below d -> "Incorrect parameters" before anything is allocated
    {
        let mut p = cover_params(4, 0);
        p.steps = 1;
        cfgs.push(p);
    }

    for (cname, corpus) in &corpora {
        for p in &cfgs {
            for cap in [256usize, 1024] {
                let mut cp = *p;
                let mut rp = *p;
                let mut cb = dict_buf(cap, 0x11);
                let mut rb = dict_buf(cap, 0x11);
                let cn = unsafe {
                    c_tr(
                        cb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        &mut cp,
                    )
                };
                let rn = unsafe {
                    r_tr(
                        rb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        &mut rp,
                    )
                };
                let tag =
                    format!("ZDICT_optimizeTrainFromBuffer_cover[{cname}] p={p:?} cap={cap}");
                if !assert_rc(&tag, cn, rn) {
                    continue;
                }
                // NaN splitPoint round trips as NaN; compare bitwise so the
                // out-parameter is still fully checked.
                assert_eq_dbg(&format!("{tag} / out k"), cp.k, rp.k);
                assert_eq_dbg(&format!("{tag} / out d"), cp.d, rp.d);
                assert_eq_dbg(&format!("{tag} / out steps"), cp.steps, rp.steps);
                assert_eq_dbg(&format!("{tag} / out nbThreads"), cp.nbThreads, rp.nbThreads);
                assert_eq_dbg(
                    &format!("{tag} / out splitPoint bits"),
                    cp.splitPoint.to_bits(),
                    rp.splitPoint.to_bits(),
                );
                assert_eq_dbg(
                    &format!("{tag} / out shrinkDict"),
                    (cp.shrinkDict, cp.shrinkDictMaxRegression),
                    (rp.shrinkDict, rp.shrinkDictMaxRegression),
                );
                assert_eq_dbg(&format!("{tag} / out zParams"), cp.zParams, rp.zParams);
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cap == 1024 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

// ========================================== 8/9/10. ZDICT_*_fastCover

fn fast_params(k: u32, d: u32) -> ZDICT_fastCover_params_t {
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
        zParams: ZDICT_params_t::default(),
    }
}

/// `ZDICT_trainFromBuffer_fastCover` over `k`, `d` (must be 6 or 8), `f`,
/// `accel` and the `zParams`. As with cover, `splitPoint` is force-set to 1.0
/// before validation (fastcover.c:561).
#[test]
fn train_from_buffer_fastcover_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");

    let corpora = vec![
        (
            "shared-tabular-40x1000",
            corpus_shared(Shape::Tabular, &sizes_uniform(40, 1000), 0xA001),
        ),
        (
            "shared-skewed-200x200",
            corpus_shared(Shape::SkewedText, &sizes_uniform(200, 200), 0xA002),
        ),
        (
            "shared-varying",
            corpus_shared(Shape::Repetitive, &sizes_varying(40, 8, 2000, 0xA003), 0xA013),
        ),
        (
            "independent-random-20x1000",
            corpus_independent(Shape::Random, &sizes_uniform(20, 1000), 0xA004),
        ),
        (
            "constant-10x1200",
            corpus_independent(Shape::Constant, &sizes_uniform(10, 1200), 0xA005),
        ),
        (
            "two-phase-15x800",
            corpus_shared(Shape::TwoPhase, &sizes_uniform(15, 800), 0xA006),
        ),
    ];

    let mut cfgs: Vec<ZDICT_fastCover_params_t> = Vec::new();
    for &d in &[6u32, 8] {
        for &k in &[8u32, 16, 50, 200, 1024] {
            cfgs.push(fast_params(k, d));
        }
    }
    // f: 0 -> DEFAULT_F(20); small f makes hash collisions dominate
    for &f in &[0u32, 1, 6, 8, 15, 20, 23] {
        let mut p = fast_params(64, 8);
        p.f = f;
        cfgs.push(p);
    }
    // accel: 0 -> DEFAULT_ACCEL(1); 1..10 valid
    for &a in &[0u32, DEFAULT_ACCEL, 2, 5, 10] {
        let mut p = fast_params(64, 8);
        p.accel = a;
        cfgs.push(p);
    }
    // splitPoint is overwritten -> even out-of-range values must succeed
    for &sp in &[0.0f64, 0.5, 1.0, 2.0, -0.5] {
        let mut p = fast_params(64, 8);
        p.splitPoint = sp;
        cfgs.push(p);
    }
    // steps / nbThreads / shrinkDict are ignored on this path
    for &(steps, nbt, sd, mr) in &[(0u32, 0u32, 0u32, 0u32), (5, 1, 1, 25), (40, 1, 1, 100)] {
        let mut p = fast_params(64, 8);
        p.steps = steps;
        p.nbThreads = nbt;
        p.shrinkDict = sd;
        p.shrinkDictMaxRegression = mr;
        cfgs.push(p);
    }
    for &lvl in &[-5i32, 0, 1, 3, 19] {
        let mut p = fast_params(64, 8);
        p.zParams.compressionLevel = lvl;
        cfgs.push(p);
    }
    for &id in &[0u32, 1, 32767, 32768, 0x8000_0000, u32::MAX] {
        let mut p = fast_params(64, 8);
        p.zParams.dictID = id;
        cfgs.push(p);
    }

    for (cname, corpus) in &corpora {
        for p in &cfgs {
            for cap in [256usize, 2048, 40_960] {
                let mut cb = dict_buf(cap, 0x2E);
                let mut rb = dict_buf(cap, 0x2E);
                let cn = unsafe {
                    c_tr(
                        cb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let rn = unsafe {
                    r_tr(
                        rb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        *p,
                    )
                };
                let tag = format!("ZDICT_trainFromBuffer_fastCover[{cname}] p={p:?} cap={cap}");
                if !assert_rc(&tag, cn, rn) {
                    continue;
                }
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cap >= 2048 && p.zParams.dictID == 0 && p.zParams.compressionLevel == 0 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

/// `FASTCOVER_checkParameters` rejections plus the shared `nbSamples == 0` /
/// capacity / `FASTCOVER_ctx_init` minimums.
#[test]
fn train_from_buffer_fastcover_errors() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");

    let good = corpus_shared(Shape::Tabular, &sizes_uniform(20, 700), 0xB001);
    let empty = Corpus {
        buf: Vec::new(),
        sizes: Vec::new(),
    };
    let sub8 = corpus_shared(Shape::Tabular, &sizes_uniform(7, 1), 0xB002);
    let four = corpus_shared(Shape::Tabular, &sizes_uniform(4, 400), 0xB003);

    struct Case<'a> {
        name: &'a str,
        corpus: &'a Corpus,
        cap: usize,
        p: ZDICT_fastCover_params_t,
        expect: Option<usize>,
    }
    let mut cases: Vec<Case> = Vec::new();

    // d must be exactly 6 or 8 (fastcover.c:237) -> parameter_outOfBound
    for &d in &[0u32, 1, 2, 4, 5, 7, 9, 12, 16, 32, u32::MAX] {
        cases.push(Case {
            name: "bad-d",
            corpus: &good,
            cap: 1024,
            p: fast_params(64, d),
            expect: Some(err(42)),
        });
    }
    for &d in &[6u32, 8] {
        cases.push(Case {
            name: "good-d",
            corpus: &good,
            cap: 1024,
            p: fast_params(64, d),
            expect: None,
        });
    }
    // k
    for &k in &[0u32, 4, 2048, u32::MAX] {
        cases.push(Case {
            name: "bad-k",
            corpus: &good,
            cap: 1024,
            // k=0 -> rejected; k=4 -> d(8) > k; k>cap -> rejected
            p: fast_params(k, 8),
            expect: Some(err(42)),
        });
    }
    cases.push(Case {
        name: "k==cap",
        corpus: &good,
        cap: 1024,
        p: fast_params(1024, 8),
        expect: None,
    });
    // f: 0 -> default(20) so valid; f > 31 rejected
    for &f in &[32u32, 33, 64, u32::MAX] {
        let mut p = fast_params(64, 8);
        p.f = f;
        cases.push(Case {
            name: "bad-f",
            corpus: &good,
            cap: 1024,
            p,
            expect: Some(err(42)),
        });
    }
    // accel: 0 -> default(1); > FASTCOVER_MAX_ACCEL rejected
    for &a in &[FASTCOVER_MAX_ACCEL + 1, 12, 100, u32::MAX] {
        let mut p = fast_params(64, 8);
        p.accel = a;
        cases.push(Case {
            name: "bad-accel",
            corpus: &good,
            cap: 1024,
            p,
            expect: Some(err(42)),
        });
    }
    // nbSamples == 0 is checked after the parameter check
    cases.push(Case {
        name: "nbSamples=0",
        corpus: &empty,
        cap: 1024,
        p: fast_params(64, 8),
        expect: Some(err(72)),
    });
    // capacity below ZDICT_DICTSIZE_MIN (k must be <= cap to reach the check)
    for cap in [8usize, 64, 128, 255] {
        cases.push(Case {
            name: "cap<DICTSIZE_MIN",
            corpus: &good,
            cap,
            p: fast_params(8, 8),
            expect: Some(err(70)),
        });
    }
    // ctx_init minimums
    cases.push(Case {
        name: "total<8",
        corpus: &sub8,
        cap: 1024,
        p: fast_params(8, 8),
        expect: Some(err(72)),
    });
    cases.push(Case {
        name: "4-samples",
        corpus: &four,
        cap: 1024,
        p: fast_params(64, 8),
        expect: Some(err(72)),
    });

    for c in &cases {
        let mut cb = dict_buf(c.cap, 0x63);
        let mut rb = dict_buf(c.cap, 0x63);
        let cn = unsafe {
            c_tr(
                cb.as_mut_ptr(),
                c.cap,
                c.corpus.buf_ptr(),
                c.corpus.sizes_ptr(),
                c.corpus.nb(),
                c.p,
            )
        };
        let rn = unsafe {
            r_tr(
                rb.as_mut_ptr(),
                c.cap,
                c.corpus.buf_ptr(),
                c.corpus.sizes_ptr(),
                c.corpus.nb(),
                c.p,
            )
        };
        let tag = format!(
            "fastcover-error[{}] cap={} k={} d={} f={} accel={}",
            c.name, c.cap, c.p.k, c.p.d, c.p.f, c.p.accel
        );
        let ok = assert_rc(&tag, cn, rn);
        match c.expect {
            Some(e) => {
                assert!(!ok, "{tag}: expected error {e:#x}, got {cn}");
                assert_eq_dbg(&format!("{tag} / exact code"), cn, e);
            }
            None => assert!(ok, "{tag}: expected success, got {cn:#x}"),
        }
        if ok {
            assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
        }
    }
}

/// `ZDICT_optimizeTrainFromBuffer_fastCover`, including the `*parameters`
/// write-back (`FASTCOVER_convertToFastCoverParams`). Default `splitPoint`
/// here is 0.75, so >= 7 samples are needed for the train/test split.
#[test]
fn optimize_train_from_buffer_fastcover_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");

    let corpora = vec![
        (
            "shared-tabular-24x900",
            corpus_shared(Shape::Tabular, &sizes_uniform(24, 900), 0x9001),
        ),
        (
            "shared-skewed-80x200",
            corpus_shared(Shape::SkewedText, &sizes_uniform(80, 200), 0x9002),
        ),
        (
            "independent-random-16x700",
            corpus_independent(Shape::Random, &sizes_uniform(16, 700), 0x9003),
        ),
        (
            "shared-varying-24",
            corpus_shared(Shape::Repetitive, &sizes_varying(24, 20, 1000, 0x9004), 0x9014),
        ),
        (
            "7-samples",
            corpus_shared(Shape::Tabular, &sizes_uniform(7, 900), 0x9005),
        ),
        (
            "6-samples",
            corpus_shared(Shape::Tabular, &sizes_uniform(6, 900), 0x9006),
        ),
    ];

    let mut cfgs: Vec<ZDICT_fastCover_params_t> = Vec::new();
    for &d in &[0u32, 6, 8] {
        let mut p = fast_params(0, d);
        p.steps = 2;
        cfgs.push(p);
    }
    for &k in &[16u32, 50, 200, 2000] {
        let mut p = fast_params(k, 8);
        p.steps = 2;
        cfgs.push(p);
    }
    for &f in &[0u32, 8, 15, 20] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.f = f;
        cfgs.push(p);
    }
    for &a in &[0u32, 1, 3, 10, 11] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.accel = a;
        cfgs.push(p);
    }
    for &sp in &[0.0f64, -1.0, 0.25, 0.5, 0.75, 1.0, 1.25, f64::NAN] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.splitPoint = sp;
        cfgs.push(p);
    }
    for &steps in &[0u32, 1, 3] {
        let mut p = fast_params(1000, 8);
        p.steps = steps;
        cfgs.push(p);
    }
    for &nbt in &[0u32, 1] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.nbThreads = nbt;
        cfgs.push(p);
    }
    for &(sd, mr) in &[(1u32, 0u32), (1, 50)] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.shrinkDict = sd;
        p.shrinkDictMaxRegression = mr;
        cfgs.push(p);
    }
    for &lvl in &[-3i32, 0, 1, 19] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.zParams.compressionLevel = lvl;
        cfgs.push(p);
    }
    for &id in &[0u32, 7, 32768, u32::MAX] {
        let mut p = fast_params(64, 8);
        p.steps = 1;
        p.zParams.dictID = id;
        cfgs.push(p);
    }

    for (cname, corpus) in &corpora {
        for p in &cfgs {
            for cap in [256usize, 1024] {
                let mut cp = *p;
                let mut rp = *p;
                let mut cb = dict_buf(cap, 0x44);
                let mut rb = dict_buf(cap, 0x44);
                let cn = unsafe {
                    c_tr(
                        cb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        &mut cp,
                    )
                };
                let rn = unsafe {
                    r_tr(
                        rb.as_mut_ptr(),
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                        &mut rp,
                    )
                };
                let tag = format!(
                    "ZDICT_optimizeTrainFromBuffer_fastCover[{cname}] p={p:?} cap={cap}"
                );
                if !assert_rc(&tag, cn, rn) {
                    continue;
                }
                assert_eq_dbg(&format!("{tag} / out k"), cp.k, rp.k);
                assert_eq_dbg(&format!("{tag} / out d"), cp.d, rp.d);
                assert_eq_dbg(&format!("{tag} / out f"), cp.f, rp.f);
                assert_eq_dbg(&format!("{tag} / out steps"), cp.steps, rp.steps);
                assert_eq_dbg(&format!("{tag} / out nbThreads"), cp.nbThreads, rp.nbThreads);
                assert_eq_dbg(
                    &format!("{tag} / out splitPoint bits"),
                    cp.splitPoint.to_bits(),
                    rp.splitPoint.to_bits(),
                );
                assert_eq_dbg(&format!("{tag} / out accel"), cp.accel, rp.accel);
                assert_eq_dbg(
                    &format!("{tag} / out shrinkDict"),
                    (cp.shrinkDict, cp.shrinkDictMaxRegression),
                    (rp.shrinkDict, rp.shrinkDictMaxRegression),
                );
                assert_eq_dbg(&format!("{tag} / out zParams"), cp.zParams, rp.zParams);
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cap == 1024 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

// ================================================ 11/12. finalizeDictionary

/// `ZDICT_finalizeDictionary` on hand-made "custom content", including the
/// documented overlapping-buffer usage (content living at the tail of the
/// output buffer, which is exactly how cover.c calls it).
#[test]
fn finalize_dictionary_matches() {
    let i = impls();
    let (c_fin, r_fin) = i.pair::<FnFinalize>("ZDICT_finalizeDictionary");

    let corpora = vec![
        (
            "shared-tabular",
            corpus_shared(Shape::Tabular, &sizes_uniform(30, 900), 0x5001),
        ),
        (
            "shared-skewed",
            corpus_shared(Shape::SkewedText, &sizes_uniform(100, 200), 0x5002),
        ),
        (
            "independent-random",
            corpus_independent(Shape::Random, &sizes_uniform(15, 900), 0x5003),
        ),
        (
            "all-identical",
            corpus_independent(Shape::Constant, &sizes_uniform(12, 700), 0x5004),
        ),
        (
            "varying",
            corpus_shared(Shape::Repetitive, &sizes_varying(30, 5, 1500, 0x5005), 0x5015),
        ),
        (
            "no-samples",
            Corpus {
                buf: Vec::new(),
                sizes: Vec::new(),
            },
        ),
        (
            "one-sample",
            corpus_shared(Shape::Tabular, &sizes_uniform(1, 600), 0x5006),
        ),
        (
            "empty-samples",
            Corpus {
                buf: Vec::new(),
                sizes: vec![0; 8],
            },
        ),
    ];

    let mut contents: Vec<(&'static str, Vec<u8>)> = Vec::new();
    {
        let mut rng = Rng::new(0x5EED_5001);
        contents.push(("empty", Vec::new()));
        contents.push(("one-byte", vec![0x42]));
        contents.push(("seven-bytes", vec![1, 2, 3, 4, 5, 6, 7]));
        contents.push(("eight-bytes", vec![1, 2, 3, 4, 5, 6, 7, 8]));
        contents.push(("zeros-300", vec![0u8; 300]));
        contents.push(("random-300", gen_shape(Shape::Random, 300, &mut rng)));
        contents.push(("text-1000", gen_shape(Shape::SkewedText, 1000, &mut rng)));
        contents.push(("tabular-2000", gen_shape(Shape::Tabular, 2000, &mut rng)));
        // "bogus" content: a buffer that looks like a zstd dictionary already
        let mut bogus = Vec::new();
        bogus.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        bogus.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        bogus.extend(gen_shape(Shape::Random, 400, &mut rng));
        contents.push(("looks-like-a-dict", bogus));
    }

    let mut params: Vec<ZDICT_params_t> = Vec::new();
    for &lvl in &[-131_072i32, -5, 0, 1, 3, 19, 22] {
        params.push(ZDICT_params_t {
            compressionLevel: lvl,
            notificationLevel: 0,
            dictID: 0,
        });
    }
    for &id in &[0u32, 1, 32767, 32768, 0x8000_0000, u32::MAX] {
        params.push(ZDICT_params_t {
            compressionLevel: 0,
            notificationLevel: 0,
            dictID: id,
        });
    }

    for (cname, corpus) in &corpora {
        for (dname, content) in &contents {
            for p in &params {
                for &cap in &[256usize, 512, 4096] {
                    // ---- non-overlapping content buffer
                    let mut cb = dict_buf(cap, 0x99);
                    let mut rb = dict_buf(cap, 0x99);
                    let cn = unsafe {
                        c_fin(
                            cb.as_mut_ptr(),
                            cap,
                            content.as_ptr(),
                            content.len(),
                            corpus.buf_ptr(),
                            corpus.sizes_ptr(),
                            corpus.nb(),
                            *p,
                        )
                    };
                    let rn = unsafe {
                        r_fin(
                            rb.as_mut_ptr(),
                            cap,
                            content.as_ptr(),
                            content.len(),
                            corpus.buf_ptr(),
                            corpus.sizes_ptr(),
                            corpus.nb(),
                            *p,
                        )
                    };
                    let tag = format!(
                        "ZDICT_finalizeDictionary[{cname}/{dname}] p={p:?} cap={cap} disjoint"
                    );
                    if assert_rc(&tag, cn, rn) {
                        assert!(cn <= cap, "{tag}: {cn} > {cap}");
                        assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                        if cap == 4096 && p.dictID == 0 {
                            verify_dict(&tag, &cb[..cn], &corpus.probes());
                        }
                    }

                    // ---- overlapping: content sits at the tail of dictBuffer
                    if content.len() <= cap {
                        let tail = cap - content.len();
                        let mut cb = dict_buf(cap, 0x99);
                        let mut rb = dict_buf(cap, 0x99);
                        cb[tail..cap].copy_from_slice(content);
                        rb[tail..cap].copy_from_slice(content);
                        let cn = unsafe {
                            let base = cb.as_mut_ptr();
                            c_fin(
                                base,
                                cap,
                                base.add(tail),
                                content.len(),
                                corpus.buf_ptr(),
                                corpus.sizes_ptr(),
                                corpus.nb(),
                                *p,
                            )
                        };
                        let rn = unsafe {
                            let base = rb.as_mut_ptr();
                            r_fin(
                                base,
                                cap,
                                base.add(tail),
                                content.len(),
                                corpus.buf_ptr(),
                                corpus.sizes_ptr(),
                                corpus.nb(),
                                *p,
                            )
                        };
                        let tag = format!(
                            "ZDICT_finalizeDictionary[{cname}/{dname}] p={p:?} cap={cap} overlap"
                        );
                        if assert_rc(&tag, cn, rn) {
                            assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                        }
                    }
                }
            }
        }
    }
}

/// `ZDICT_finalizeDictionary` error paths: `maxDictSize < dictContentSize`,
/// `maxDictSize < ZDICT_DICTSIZE_MIN`, and the "too small to fit max repcode"
/// padding failure.
#[test]
fn finalize_dictionary_errors() {
    let i = impls();
    let (c_fin, r_fin) = i.pair::<FnFinalize>("ZDICT_finalizeDictionary");
    let corpus = corpus_shared(Shape::Tabular, &sizes_uniform(16, 700), 0x6001);
    let mut rng = Rng::new(0x6002);
    let content = gen_shape(Shape::SkewedText, 4096, &mut rng);

    for &cap in &[0usize, 1, 7, 8, 9, 16, 64, 128, 255, 256, 257, 300, 512] {
        for &clen in &[0usize, 1, 8, 100, 255, 256, 257, 512, 4096] {
            if clen > content.len() {
                continue;
            }
            let mut cb = dict_buf(cap, 0x21);
            let mut rb = dict_buf(cap, 0x21);
            let p = ZDICT_params_t::default();
            let cn = unsafe {
                c_fin(
                    cb.as_mut_ptr(),
                    cap,
                    content.as_ptr(),
                    clen,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    p,
                )
            };
            let rn = unsafe {
                r_fin(
                    rb.as_mut_ptr(),
                    cap,
                    content.as_ptr(),
                    clen,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    p,
                )
            };
            let tag = format!("finalize-error cap={cap} contentSize={clen}");
            let ok = assert_rc(&tag, cn, rn);
            if cap < clen || cap < ZDICT_DICTSIZE_MIN {
                assert!(!ok, "{tag}: expected dstSize_tooSmall, got {cn}");
                assert_eq_dbg(&format!("{tag} / exact code"), cn, err(70));
            } else if ok {
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            }
        }
    }
}

// ============================================ 13. addEntropyTablesFromBuffer

/// The deprecated in-place variant: content is expected at the *end* of
/// `dictBuffer` and the header is prepended (zdict.c:940). `dictBufferCapacity`
/// is kept >= 64 because the C computes `dictBufferCapacity - 8` unchecked.
#[test]
fn add_entropy_tables_matches() {
    let i = impls();
    let (c_ae, r_ae) = i.pair::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer");

    let corpora = vec![
        (
            "shared-tabular",
            corpus_shared(Shape::Tabular, &sizes_uniform(24, 800), 0x8001),
        ),
        (
            "independent-random",
            corpus_independent(Shape::Random, &sizes_uniform(12, 800), 0x8002),
        ),
        (
            "constant",
            corpus_independent(Shape::Constant, &sizes_uniform(10, 600), 0x8003),
        ),
        (
            "no-samples",
            Corpus {
                buf: Vec::new(),
                sizes: Vec::new(),
            },
        ),
        (
            "varying",
            corpus_shared(Shape::SkewedText, &sizes_varying(20, 3, 1200, 0x8004), 0x8014),
        ),
    ];

    let mut rng = Rng::new(0x8EED);
    for (cname, corpus) in &corpora {
        for &cap in &[64usize, 128, 256, 300, 1024, 4096] {
            for &clen in &[0usize, 1, 8, 64, 200, 1000] {
                if clen > cap {
                    continue;
                }
                let content = gen_shape(Shape::SkewedText, clen, &mut rng);
                let mut cb = dict_buf(cap, 0xB7);
                let mut rb = dict_buf(cap, 0xB7);
                cb[cap - clen..cap].copy_from_slice(&content);
                rb[cap - clen..cap].copy_from_slice(&content);
                let cn = unsafe {
                    c_ae(
                        cb.as_mut_ptr(),
                        clen,
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                    )
                };
                let rn = unsafe {
                    r_ae(
                        rb.as_mut_ptr(),
                        clen,
                        cap,
                        corpus.buf_ptr(),
                        corpus.sizes_ptr(),
                        corpus.nb(),
                    )
                };
                let tag = format!(
                    "ZDICT_addEntropyTablesFromBuffer[{cname}] cap={cap} contentSize={clen}"
                );
                if !assert_rc(&tag, cn, rn) {
                    continue;
                }
                assert!(cn <= cap, "{tag}: {cn} > {cap}");
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                if cap >= 1024 {
                    verify_dict(&tag, &cb[..cn], &corpus.probes());
                }
            }
        }
    }
}

// ================================= 14/15/16. exported COVER_* internals

/// `COVER_checkTotalCompressedSize` — compresses every (test) sample with a
/// CDict built from the candidate dictionary and sums the sizes.
#[test]
fn cover_check_total_compressed_size_matches() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_f, r_f) = i.pair::<FnCheckTcs>("COVER_checkTotalCompressedSize");
    let (c_tr, _) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");

    let corpora = vec![
        (
            "shared-tabular",
            corpus_shared(Shape::Tabular, &sizes_uniform(20, 700), 0x4001),
        ),
        (
            "independent-random",
            corpus_independent(Shape::Random, &sizes_uniform(12, 500), 0x4002),
        ),
        (
            "varying",
            corpus_shared(Shape::SkewedText, &sizes_varying(20, 0, 900, 0x4003), 0x4013),
        ),
        (
            "no-samples",
            Corpus {
                buf: Vec::new(),
                sizes: Vec::new(),
            },
        ),
    ];

    // three flavours of "dictionary": a real one, raw content, and a corrupt
    // one (magic present, garbage body) which makes ZSTD_createCDict fail.
    let mut rng = Rng::new(0x4EED);
    let real = {
        let corpus = corpus_shared(Shape::Tabular, &sizes_uniform(20, 700), 0x4001);
        let cap = 1024;
        let mut db = dict_buf(cap, 0);
        let n = unsafe {
            c_tr(
                db.as_mut_ptr(),
                cap,
                corpus.buf_ptr(),
                corpus.sizes_ptr(),
                corpus.nb(),
                cover_params(64, 8),
            )
        };
        assert!(n < usize::MAX - 200, "fixture cover dict failed: {n:#x}");
        db[..n].to_vec()
    };
    let raw = gen_shape(Shape::SkewedText, 700, &mut rng);
    let corrupt = {
        let mut v = Vec::new();
        v.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        v.extend_from_slice(&12345u32.to_le_bytes());
        v.extend(gen_shape(Shape::Random, 600, &mut rng));
        v
    };
    let dicts: Vec<(&str, Vec<u8>)> = vec![
        ("real", real),
        ("raw", raw),
        ("corrupt", corrupt),
        ("empty", Vec::new()),
    ];

    for (cname, corpus) in &corpora {
        let mut offsets = corpus.offsets();
        for (dname, dict) in &dicts {
            for &sp in &[1.0f64, 0.5] {
                for &lvl in &[0i32, 1, 3, 9] {
                    let nb = corpus.nb() as usize;
                    let nb_train = if sp < 1.0 { nb / 2 } else { nb };
                    let mut p = cover_params(64, 8);
                    p.splitPoint = sp;
                    p.zParams.compressionLevel = lvl;
                    let mut cd = dict.clone();
                    let mut rd = dict.clone();
                    let (a, b) = unsafe {
                        (
                            c_f(
                                p,
                                corpus.sizes_ptr(),
                                corpus.buf_ptr(),
                                offsets.as_mut_ptr(),
                                nb_train,
                                nb,
                                cd.as_mut_ptr(),
                                cd.len(),
                            ),
                            r_f(
                                p,
                                corpus.sizes_ptr(),
                                corpus.buf_ptr(),
                                offsets.as_mut_ptr(),
                                nb_train,
                                nb,
                                rd.as_mut_ptr(),
                                rd.len(),
                            ),
                        )
                    };
                    let tag = format!(
                        "COVER_checkTotalCompressedSize[{cname}/{dname}] sp={sp} lvl={lvl}"
                    );
                    assert_rc(&tag, a, b);
                    assert_bytes_eq(&format!("{tag} / dict untouched"), &cd, &rd);
                }
            }
        }
    }
}

/// `COVER_selectDict` + `COVER_dictSelectionError` / `..IsError` / `..Free`.
///
/// The custom content is placed at the tail of a `dictBufferCapacity`-sized
/// allocation, exactly as `COVER_tryParameters` does, because the shrink path
/// walks backwards from `customDictContent + dictContentSize`.
#[test]
fn cover_select_dict_matches() {
    let i = impls();
    let (c_sel, r_sel) = i.pair::<FnSelectDict>("COVER_selectDict");
    let (c_dse, r_dse) = i.pair::<FnDsError>("COVER_dictSelectionError");
    let (c_dsi, r_dsi) = i.pair::<FnDsIsError>("COVER_dictSelectionIsError");
    let (c_dsf, r_dsf) = i.pair::<FnDsFree>("COVER_dictSelectionFree");

    // ---- COVER_dictSelectionError / IsError on a range of codes
    for &code in &[0usize, 1, err(1), err(42), err(70), err(72), usize::MAX] {
        unsafe {
            let a = c_dse(code);
            let b = r_dse(code);
            assert_eq_dbg(
                &format!("COVER_dictSelectionError({code:#x}) / dictSize"),
                a.dictSize,
                b.dictSize,
            );
            assert_eq_dbg(
                &format!("COVER_dictSelectionError({code:#x}) / totalCompressedSize"),
                a.totalCompressedSize,
                b.totalCompressedSize,
            );
            assert!(a.dictContent.is_null() && b.dictContent.is_null());
            assert_eq_dbg(
                &format!("COVER_dictSelectionIsError({code:#x})"),
                c_dsi(a),
                r_dsi(b),
            );
            // free(NULL) must be a no-op in both
            c_dsf(a);
            r_dsf(b);
        }
    }
    // non-null dictContent with a non-error size => not an error
    {
        let mut probe = vec![7u8; 32];
        let s = COVER_dictSelection_t {
            dictContent: probe.as_mut_ptr(),
            dictSize: 32,
            totalCompressedSize: 1234,
        };
        unsafe {
            assert_eq_dbg("COVER_dictSelectionIsError(valid)", c_dsi(s), r_dsi(s));
        }
    }

    let corpora = vec![
        (
            "shared-tabular",
            corpus_shared(Shape::Tabular, &sizes_uniform(20, 700), 0x3001),
        ),
        (
            "independent-random",
            corpus_independent(Shape::Random, &sizes_uniform(12, 600), 0x3002),
        ),
        (
            "varying",
            corpus_shared(Shape::SkewedText, &sizes_varying(20, 4, 800, 0x3003), 0x3013),
        ),
    ];

    let mut rng = Rng::new(0x3EED);
    for (cname, corpus) in &corpora {
        let mut offsets = corpus.offsets();
        let nb = corpus.nb() as usize;
        for &cap in &[256usize, 1024, 4096] {
            for &clen in &[0usize, 8, 200, 256, 700] {
                if clen > cap {
                    continue;
                }
                for &(sd, mr) in &[(0u32, 0u32), (1, 0), (1, 25), (1, 1000)] {
                    let content = gen_shape(Shape::Tabular, clen, &mut rng);
                    let tail = cap - clen;
                    let mut cbuf = vec![0x5Cu8; cap];
                    let mut rbuf = vec![0x5Cu8; cap];
                    cbuf[tail..].copy_from_slice(&content);
                    rbuf[tail..].copy_from_slice(&content);

                    let mut p = cover_params(64, 8);
                    p.shrinkDict = sd;
                    p.shrinkDictMaxRegression = mr;

                    let (cs, rs) = unsafe {
                        (
                            c_sel(
                                cbuf.as_mut_ptr().add(tail),
                                cap,
                                clen,
                                corpus.buf_ptr(),
                                corpus.sizes_ptr(),
                                nb as u32,
                                nb,
                                nb,
                                p,
                                offsets.as_mut_ptr(),
                                0,
                            ),
                            r_sel(
                                rbuf.as_mut_ptr().add(tail),
                                cap,
                                clen,
                                corpus.buf_ptr(),
                                corpus.sizes_ptr(),
                                nb as u32,
                                nb,
                                nb,
                                p,
                                offsets.as_mut_ptr(),
                                0,
                            ),
                        )
                    };
                    let tag = format!(
                        "COVER_selectDict[{cname}] cap={cap} contentSize={clen} shrink={sd}/{mr}"
                    );
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / isError"), c_dsi(cs), r_dsi(rs));
                    }
                    assert_eq_dbg(&format!("{tag} / dictSize"), cs.dictSize, rs.dictSize);
                    assert_eq_dbg(
                        &format!("{tag} / totalCompressedSize"),
                        cs.totalCompressedSize,
                        rs.totalCompressedSize,
                    );
                    assert_eq_dbg(
                        &format!("{tag} / null-ness"),
                        cs.dictContent.is_null(),
                        rs.dictContent.is_null(),
                    );
                    if !cs.dictContent.is_null() && cs.dictSize > 0 {
                        let a = unsafe { std::slice::from_raw_parts(cs.dictContent, cs.dictSize) };
                        let b = unsafe { std::slice::from_raw_parts(rs.dictContent, rs.dictSize) };
                        assert_bytes_eq(&format!("{tag} / dictContent"), a, b);
                        verify_dict(&tag, a, &corpus.probes());
                    }
                    unsafe {
                        c_dsf(cs);
                        r_dsf(rs);
                    }
                }
            }
        }
    }
}

/// `COVER_best_init` / `_start` / `_finish` / `_wait` / `_destroy`. With no
/// `ZSTD_MULTITHREAD` the mutex/cond are inert `int`s, so this exercises the
/// "keep the best dictionary" bookkeeping only — which is what the optimizers
/// depend on.
#[test]
fn cover_best_lifecycle_matches() {
    let i = impls();
    let (c_init, r_init) = i.pair::<FnBest1>("COVER_best_init");
    let (c_start, r_start) = i.pair::<FnBest1>("COVER_best_start");
    let (c_wait, r_wait) = i.pair::<FnBest1>("COVER_best_wait");
    let (c_destroy, r_destroy) = i.pair::<FnBest1>("COVER_best_destroy");
    let (c_finish, r_finish) = i.pair::<FnBestFinish>("COVER_best_finish");

    // NULL tolerance (cover.c:896, 910, 924, 940, 960)
    unsafe {
        c_init(std::ptr::null_mut());
        r_init(std::ptr::null_mut());
        c_start(std::ptr::null_mut());
        r_start(std::ptr::null_mut());
        c_wait(std::ptr::null_mut());
        r_wait(std::ptr::null_mut());
        c_destroy(std::ptr::null_mut());
        r_destroy(std::ptr::null_mut());
        c_finish(std::ptr::null_mut(), cover_params(64, 8), {
            COVER_dictSelection_t {
                dictContent: std::ptr::null_mut(),
                dictSize: 0,
                totalCompressedSize: 0,
            }
        });
        r_finish(std::ptr::null_mut(), cover_params(64, 8), {
            COVER_dictSelection_t {
                dictContent: std::ptr::null_mut(),
                dictSize: 0,
                totalCompressedSize: 0,
            }
        });
    }

    let zero = COVER_best_t {
        mutex: 0,
        cond: 0,
        liveJobs: 0,
        dict: std::ptr::null_mut(),
        dictSize: 0,
        parameters: ZDICT_cover_params_t::default(),
        compressedSize: 0,
    };
    let mut cbest = zero;
    let mut rbest = zero;

    let cmp = |tag: &str, a: &COVER_best_t, b: &COVER_best_t| {
        assert_eq_dbg(&format!("{tag} / liveJobs"), a.liveJobs, b.liveJobs);
        assert_eq_dbg(&format!("{tag} / dictSize"), a.dictSize, b.dictSize);
        assert_eq_dbg(
            &format!("{tag} / compressedSize"),
            a.compressedSize,
            b.compressedSize,
        );
        assert_eq_dbg(&format!("{tag} / parameters"), a.parameters, b.parameters);
        assert_eq_dbg(
            &format!("{tag} / dict null-ness"),
            a.dict.is_null(),
            b.dict.is_null(),
        );
        if !a.dict.is_null() && a.dictSize > 0 {
            let x = unsafe { std::slice::from_raw_parts(a.dict as *const u8, a.dictSize) };
            let y = unsafe { std::slice::from_raw_parts(b.dict as *const u8, b.dictSize) };
            assert_bytes_eq(&format!("{tag} / dict bytes"), x, y);
        }
    };

    unsafe {
        c_init(&mut cbest);
        r_init(&mut rbest);
    }
    cmp("after init", &cbest, &rbest);
    // documented post-init state
    assert_eq_dbg("init compressedSize", cbest.compressedSize, usize::MAX);

    // a sequence of candidate results: improving, worsening, equal, NULL dict,
    // and an error-coded compressedSize
    let mut payloads: Vec<Vec<u8>> = Vec::new();
    let mut rng = Rng::new(0x2EED);
    for n in [64usize, 300, 128, 300, 512] {
        payloads.push(gen_shape(Shape::Tabular, n, &mut rng));
    }

    let steps: Vec<(&str, usize, bool, u32)> = vec![
        ("first", 5000, true, 11),
        ("worse", 9000, true, 22),
        ("better", 2500, true, 33),
        ("equal", 2500, true, 44),
        ("null-dict-better", 1000, false, 55),
        ("error-size", err(1), true, 66),
    ];
    for (si, (nm, csize, with_dict, k)) in steps.iter().enumerate() {
        let mut payload = payloads[si % payloads.len()].clone();
        let plen = payload.len();
        let mut p = cover_params(*k, 8);
        p.zParams.dictID = *k;
        let sel = COVER_dictSelection_t {
            dictContent: if *with_dict {
                payload.as_mut_ptr()
            } else {
                std::ptr::null_mut()
            },
            dictSize: plen,
            totalCompressedSize: *csize,
        };
        unsafe {
            c_start(&mut cbest);
            r_start(&mut rbest);
            cmp(&format!("after start {nm}"), &cbest, &rbest);
            c_finish(&mut cbest, p, sel);
            r_finish(&mut rbest, p, sel);
        }
        cmp(&format!("after finish {nm}"), &cbest, &rbest);
    }

    unsafe {
        c_wait(&mut cbest);
        r_wait(&mut rbest);
        cmp("after wait", &cbest, &rbest);
        c_destroy(&mut cbest);
        r_destroy(&mut rbest);
    }
}

// ============================================= 17. notificationLevel sweep

/// `notificationLevel` 0..4 changes only stderr text, never the dictionary.
/// Tiny corpora keep the (expected) stderr noise small.
#[test]
fn notification_levels_match() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_cov, r_cov) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
    let (c_fast, r_fast) = i.pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
    let (c_leg, r_leg) = i.pair::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
    let (c_fin, r_fin) = i.pair::<FnFinalize>("ZDICT_finalizeDictionary");
    let (c_ocov, r_ocov) = i.pair::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
    let (c_ofast, r_ofast) = i.pair::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");

    let corpus = corpus_shared(Shape::Tabular, &sizes_uniform(10, 200), 0x1234);
    let cap = 256usize;
    let mut rng = Rng::new(0x1235);
    let content = gen_shape(Shape::Tabular, 200, &mut rng);

    for nl in 0..=4u32 {
        let mut cp = cover_params(64, 8);
        cp.zParams.notificationLevel = nl;
        let mut fp = fast_params(64, 8);
        fp.zParams.notificationLevel = nl;
        let lp = ZDICT_legacy_params_t {
            selectivityLevel: 9,
            zParams: ZDICT_params_t {
                compressionLevel: 0,
                notificationLevel: nl,
                dictID: 0,
            },
        };
        let zp = ZDICT_params_t {
            compressionLevel: 0,
            notificationLevel: nl,
            dictID: 0,
        };

        macro_rules! run {
            ($tag:expr, $cf:expr, $rf:expr) => {{
                let mut cb = dict_buf(cap, 0x1F);
                let mut rb = dict_buf(cap, 0x1F);
                let cn = unsafe { $cf(cb.as_mut_ptr()) };
                let rn = unsafe { $rf(rb.as_mut_ptr()) };
                let tag = format!("{}(notificationLevel={nl})", $tag);
                if assert_rc(&tag, cn, rn) {
                    assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
                }
            }};
        }

        run!(
            "ZDICT_trainFromBuffer_cover",
            |d: *mut u8| c_cov(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), cp),
            |d: *mut u8| r_cov(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), cp)
        );
        run!(
            "ZDICT_trainFromBuffer_fastCover",
            |d: *mut u8| c_fast(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), fp),
            |d: *mut u8| r_fast(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), fp)
        );
        run!(
            "ZDICT_trainFromBuffer_legacy",
            |d: *mut u8| c_leg(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), lp),
            |d: *mut u8| r_leg(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), lp)
        );
        run!(
            "ZDICT_finalizeDictionary",
            |d: *mut u8| c_fin(
                d,
                cap,
                content.as_ptr(),
                content.len(),
                corpus.buf_ptr(),
                corpus.sizes_ptr(),
                corpus.nb(),
                zp
            ),
            |d: *mut u8| r_fin(
                d,
                cap,
                content.as_ptr(),
                content.len(),
                corpus.buf_ptr(),
                corpus.sizes_ptr(),
                corpus.nb(),
                zp
            )
        );

        // the optimizers additionally shift g_displayLevel down by one
        {
            let mut c1 = cp;
            c1.steps = 1;
            let mut r1 = c1;
            let mut cb = dict_buf(cap, 0x1F);
            let mut rb = dict_buf(cap, 0x1F);
            let cn = unsafe {
                c_ocov(
                    cb.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut c1,
                )
            };
            let rn = unsafe {
                r_ocov(
                    rb.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut r1,
                )
            };
            let tag = format!("ZDICT_optimizeTrainFromBuffer_cover(notificationLevel={nl})");
            if assert_rc(&tag, cn, rn) {
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            }
            assert_eq_dbg(&format!("{tag} / out params"), c1, r1);
        }
        {
            let mut c1 = fp;
            c1.steps = 1;
            let mut r1 = c1;
            let mut cb = dict_buf(cap, 0x1F);
            let mut rb = dict_buf(cap, 0x1F);
            let cn = unsafe {
                c_ofast(
                    cb.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut c1,
                )
            };
            let rn = unsafe {
                r_ofast(
                    rb.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut r1,
                )
            };
            let tag = format!("ZDICT_optimizeTrainFromBuffer_fastCover(notificationLevel={nl})");
            if assert_rc(&tag, cn, rn) {
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            }
            assert_eq_dbg(&format!("{tag} / out params"), c1, r1);
        }
    }
}

// ====================================== 18. determinism (evidence, not parity)

/// Empirical justification for asserting *byte* equality everywhere above.
///
/// Every trainer is invoked twice **inside the same library** (C twice, Rust
/// twice) on identical inputs; the outputs must be bit-identical. If any of
/// these functions were genuinely nondeterministic — e.g. because
/// `ZDICT_finalizeDictionary`'s "auto" dictID used a clock/PRNG rather than
/// `XXH64(dictContent)`, or because `optimize*` raced worker threads — this
/// test would fail and the cross-library assertions would have to be weakened
/// to structural checks. It passes, so they are not weakened.
#[test]
fn same_library_results_are_deterministic() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrain>("ZDICT_trainFromBuffer");
    let (c_cov, r_cov) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
    let (c_fast, r_fast) = i.pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
    let (c_leg, r_leg) = i.pair::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
    let (c_ocov, r_ocov) = i.pair::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
    let (c_ofast, r_ofast) = i.pair::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
    let (c_fin, r_fin) = i.pair::<FnFinalize>("ZDICT_finalizeDictionary");

    let corpus = corpus_shared(Shape::Tabular, &sizes_uniform(24, 1500), 0xDE7E_0001);
    let cap = 2048usize;
    let mut rng = Rng::new(0xDE7E_0002);
    let content = gen_shape(Shape::SkewedText, 900, &mut rng);

    /// Run `f` twice and require identical (rc, bytes).
    macro_rules! twice {
        ($tag:expr, $f:expr) => {{
            let mut a = dict_buf(cap, 0);
            let mut b = dict_buf(cap, 0);
            let n1 = unsafe { $f(a.as_mut_ptr()) };
            let n2 = unsafe { $f(b.as_mut_ptr()) };
            assert_eq_dbg(&format!("{} / repeat rc", $tag), n1, n2);
            if n1 < usize::MAX - 200 {
                assert!(n1 > 0, "{}: expected a dictionary", $tag);
                assert_bytes_eq(&format!("{} / repeat bytes", $tag), &a[..n1], &b[..n2]);
            }
        }};
    }

    let cp = cover_params(64, 8);
    let mut fp = fast_params(64, 8);
    fp.steps = 2;
    let lp = ZDICT_legacy_params_t {
        selectivityLevel: 9,
        zParams: ZDICT_params_t::default(),
    };
    let zp = ZDICT_params_t::default();

    for (lib, tr, cov, fast, leg, ocov, ofast, fin) in [
        (
            "C", &c_tr, &c_cov, &c_fast, &c_leg, &c_ocov, &c_ofast, &c_fin,
        ),
        (
            "Rust", &r_tr, &r_cov, &r_fast, &r_leg, &r_ocov, &r_ofast, &r_fin,
        ),
    ] {
        twice!(format!("{lib} ZDICT_trainFromBuffer"), |d: *mut u8| tr(
            d,
            cap,
            corpus.buf_ptr(),
            corpus.sizes_ptr(),
            corpus.nb()
        ));
        twice!(
            format!("{lib} ZDICT_trainFromBuffer_cover"),
            |d: *mut u8| cov(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), cp)
        );
        twice!(
            format!("{lib} ZDICT_trainFromBuffer_fastCover"),
            |d: *mut u8| fast(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), fp)
        );
        twice!(
            format!("{lib} ZDICT_trainFromBuffer_legacy"),
            |d: *mut u8| leg(d, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), lp)
        );
        twice!(format!("{lib} ZDICT_finalizeDictionary"), |d: *mut u8| fin(
            d,
            cap,
            content.as_ptr(),
            content.len(),
            corpus.buf_ptr(),
            corpus.sizes_ptr(),
            corpus.nb(),
            zp
        ));

        // the optimizers also write back *parameters, which must repeat too
        {
            let mut p1 = {
                let mut p = cover_params(0, 0);
                p.steps = 2;
                p
            };
            let mut p2 = p1;
            let mut a = dict_buf(cap, 0);
            let mut b = dict_buf(cap, 0);
            let n1 = unsafe {
                ocov(
                    a.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut p1,
                )
            };
            let n2 = unsafe {
                ocov(
                    b.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut p2,
                )
            };
            let tag = format!("{lib} ZDICT_optimizeTrainFromBuffer_cover");
            assert_eq_dbg(&format!("{tag} / repeat rc"), n1, n2);
            assert_eq_dbg(&format!("{tag} / repeat params"), p1, p2);
            assert!(n1 < usize::MAX - 200 && n1 > 0, "{tag}: {n1:#x}");
            assert_bytes_eq(&format!("{tag} / repeat bytes"), &a[..n1], &b[..n2]);
        }
        {
            let mut p1 = {
                let mut p = fast_params(0, 0);
                p.steps = 2;
                p
            };
            let mut p2 = p1;
            let mut a = dict_buf(cap, 0);
            let mut b = dict_buf(cap, 0);
            let n1 = unsafe {
                ofast(
                    a.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut p1,
                )
            };
            let n2 = unsafe {
                ofast(
                    b.as_mut_ptr(),
                    cap,
                    corpus.buf_ptr(),
                    corpus.sizes_ptr(),
                    corpus.nb(),
                    &mut p2,
                )
            };
            let tag = format!("{lib} ZDICT_optimizeTrainFromBuffer_fastCover");
            assert_eq_dbg(&format!("{tag} / repeat rc"), n1, n2);
            assert_eq_dbg(&format!("{tag} / repeat params"), p1, p2);
            assert!(n1 < usize::MAX - 200 && n1 > 0, "{tag}: {n1:#x}");
            assert_bytes_eq(&format!("{tag} / repeat bytes"), &a[..n1], &b[..n2]);
        }
    }
}

// ================================== 19. randomized cross-product fuzz

/// A fixed-seed randomized sweep over the whole `(corpus x parameters x
/// capacity x entry point)` cross product. The named tests above pin the
/// interesting boundaries; this one hunts for divergences in combinations
/// nobody thought to enumerate, while staying fully reproducible.
#[test]
fn randomized_dictbuilder_fuzz() {
    let _serialize = display_level_lock();
    let i = impls();
    let (c_tr, r_tr) = i.pair::<FnTrain>("ZDICT_trainFromBuffer");
    let (c_cov, r_cov) = i.pair::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
    let (c_fast, r_fast) = i.pair::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
    let (c_leg, r_leg) = i.pair::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");
    let (c_ocov, r_ocov) = i.pair::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
    let (c_ofast, r_ofast) = i.pair::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
    let (c_fin, r_fin) = i.pair::<FnFinalize>("ZDICT_finalizeDictionary");
    let (c_ae, r_ae) = i.pair::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer");

    const NB_CHOICES: [usize; 9] = [0, 1, 2, 5, 7, 10, 50, 200, 500];
    const CAP_CHOICES: [usize; 10] = [0, 1, 8, 255, 256, 300, 512, 1024, 8192, 40_960];
    const LEVELS: [i32; 6] = [-5, 0, 1, 3, 9, 19];
    const IDS: [u32; 6] = [0, 1, 32767, 32768, 0x8000_0000, u32::MAX];
    const SPLITS: [f64; 7] = [0.0, -1.0, 0.25, 0.5, 0.75, 1.0, 1.5];

    let mut rng = Rng::new(0xF0FF_1234);
    for iter in 0..5000u32 {
        // ---- corpus
        let nb = NB_CHOICES[rng.below(NB_CHOICES.len())];
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        // keep the total under ~120 KB so the slow trainers stay cheap
        let budget = 200_000usize;
        let per = if nb == 0 { 0 } else { (budget / nb).max(1) };
        let sizes: Vec<usize> = match rng.below(4) {
            0 => sizes_uniform(nb, rng.range(1, per)),
            1 => sizes_varying(nb, 0, per, rng.next_u64()),
            2 => sizes_uniform(nb, 1),
            _ => (0..nb).map(|j| if j % 3 == 0 { 0 } else { per }).collect(),
        };
        let corpus = if rng.bool() {
            corpus_shared(shape, &sizes, rng.next_u64())
        } else {
            corpus_independent(shape, &sizes, rng.next_u64())
        };

        let cap = CAP_CHOICES[rng.below(CAP_CHOICES.len())];
        let zp = ZDICT_params_t {
            compressionLevel: LEVELS[rng.below(LEVELS.len())],
            notificationLevel: 0, // keep stderr quiet across 320 iterations
            dictID: IDS[rng.below(IDS.len())],
        };

        let mut cb = dict_buf(cap, 0x6D);
        let mut rb = dict_buf(cap, 0x6D);
        let which = rng.below(8);
        let mut tag = format!(
            "fuzz#{iter} fn={which} shape={shape:?} nb={nb} total={} cap={cap} zp={zp:?}",
            corpus.total()
        );

        if std::env::var_os("FUZZ_TRACE").is_some() {
            eprintln!("TRACE {tag}");
        }
        let (cn, rn) = unsafe {
            match which {
                0 => {
                    // internally: optimize_fastCover(d=8, splitPoint=0.75)
                    if !cover_input_is_safe(&corpus.sizes, 0.75, 8) {
                        continue;
                    }
                    (
                        c_tr(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb()),
                        r_tr(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb()),
                    )
                }
                1 | 2 => {
                    let mut p = ZDICT_cover_params_t {
                        k: [0u32, 1, 8, 16, 50, 64, 200, 1024, 2048][rng.below(9)],
                        d: [0u32, 1, 6, 8, 9, 16][rng.below(6)],
                        steps: [0u32, 1, 2, 3][rng.below(4)],
                        nbThreads: rng.below(2) as u32,
                        splitPoint: SPLITS[rng.below(SPLITS.len())],
                        shrinkDict: rng.below(2) as u32,
                        shrinkDictMaxRegression: [0u32, 1, 25, 100][rng.below(4)],
                        zParams: zp,
                    };
                    let mut q = p;
                    tag = format!("{tag} p={p:?}");
                    // trainFromBuffer_cover forces splitPoint=1.0 (cover.c:787);
                    // the optimizer defaults <=0 to 1.0 and iterates d in {6,8}
                    // when d==0 (cover.c:1174).
                    let eff_split = if which == 1 {
                        1.0
                    } else if p.splitPoint <= 0.0 {
                        1.0
                    } else {
                        p.splitPoint
                    };
                    let ds: &[u32] = if which == 2 && p.d == 0 { &[6, 8] } else { &[p.d] };
                    if ds.iter().any(|&d| !cover_input_is_safe(&corpus.sizes, eff_split, d)) {
                        continue;
                    }
                    if which == 1 {
                        (
                            c_cov(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                            r_cov(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                        )
                    } else {
                        let a = c_ocov(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut p);
                        let b = r_ocov(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut q);
                        assert_eq_dbg(&format!("{tag} / out k,d,steps"), (p.k, p.d, p.steps), (q.k, q.d, q.steps));
                        assert_eq_dbg(
                            &format!("{tag} / out splitPoint bits"),
                            p.splitPoint.to_bits(),
                            q.splitPoint.to_bits(),
                        );
                        assert_eq_dbg(&format!("{tag} / out zParams"), p.zParams, q.zParams);
                        (a, b)
                    }
                }
                3 | 4 => {
                    let mut p = ZDICT_fastCover_params_t {
                        k: [0u32, 1, 8, 16, 50, 64, 200, 1024, 2048][rng.below(9)],
                        d: [0u32, 5, 6, 7, 8, 9][rng.below(6)],
                        f: [0u32, 1, 8, 15, 20, 31, 32][rng.below(7)],
                        steps: [0u32, 1, 2, 3][rng.below(4)],
                        nbThreads: rng.below(2) as u32,
                        splitPoint: SPLITS[rng.below(SPLITS.len())],
                        accel: [0u32, 1, 2, 5, 10, 11][rng.below(6)],
                        shrinkDict: rng.below(2) as u32,
                        shrinkDictMaxRegression: [0u32, 1, 25, 100][rng.below(4)],
                        zParams: zp,
                    };
                    let mut q = p;
                    tag = format!("{tag} p={p:?}");
                    // trainFromBuffer_fastCover forces splitPoint=1.0
                    // (fastcover.c:561); the optimizer defaults <=0 to 0.75.
                    let eff_split = if which == 3 {
                        1.0
                    } else if p.splitPoint <= 0.0 {
                        0.75
                    } else {
                        p.splitPoint
                    };
                    let ds: &[u32] = if which == 4 && p.d == 0 { &[6, 8] } else { &[p.d] };
                    if ds.iter().any(|&d| !cover_input_is_safe(&corpus.sizes, eff_split, d)) {
                        continue;
                    }
                    if which == 3 {
                        (
                            c_fast(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                            r_fast(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                        )
                    } else {
                        let a = c_ofast(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut p);
                        let b = r_ofast(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), &mut q);
                        assert_eq_dbg(
                            &format!("{tag} / out k,d,f,accel"),
                            (p.k, p.d, p.f, p.accel),
                            (q.k, q.d, q.f, q.accel),
                        );
                        assert_eq_dbg(
                            &format!("{tag} / out splitPoint bits"),
                            p.splitPoint.to_bits(),
                            q.splitPoint.to_bits(),
                        );
                        assert_eq_dbg(&format!("{tag} / out zParams"), p.zParams, q.zParams);
                        (a, b)
                    }
                }
                5 => {
                    let p = ZDICT_legacy_params_t {
                        selectivityLevel: [0u32, 1, 2, 5, 9, 10, 31, 40][rng.below(8)],
                        zParams: zp,
                    };
                    tag = format!("{tag} p={p:?}");
                    (
                        c_leg(cb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                        r_leg(rb.as_mut_ptr(), cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), p),
                    )
                }
                6 => {
                    // finalizeDictionary with content drawn from the corpus itself
                    let clen = if corpus.total() == 0 {
                        0
                    } else {
                        rng.below(corpus.total().min(cap.max(1) + 16) + 1)
                    };
                    tag = format!("{tag} contentSize={clen}");
                    (
                        c_fin(
                            cb.as_mut_ptr(), cap, corpus.buf_ptr(), clen,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), zp,
                        ),
                        r_fin(
                            rb.as_mut_ptr(), cap, corpus.buf_ptr(), clen,
                            corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb(), zp,
                        ),
                    )
                }
                _ => {
                    // addEntropyTablesFromBuffer needs cap >= 8 (it computes
                    // `dictBufferCapacity - 8` without checking) and expects the
                    // content to already live at the tail of dictBuffer.
                    if cap < 8 {
                        continue;
                    }
                    let clen = rng.below(cap + 1);
                    let mut content = vec![0u8; clen];
                    for (j, b) in content.iter_mut().enumerate() {
                        *b = if corpus.total() == 0 {
                            (j & 0xff) as u8
                        } else {
                            corpus.buf[j % corpus.total()]
                        };
                    }
                    cb[cap - clen..cap].copy_from_slice(&content);
                    rb[cap - clen..cap].copy_from_slice(&content);
                    tag = format!("{tag} contentSize={clen}");
                    (
                        c_ae(cb.as_mut_ptr(), clen, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb()),
                        r_ae(rb.as_mut_ptr(), clen, cap, corpus.buf_ptr(), corpus.sizes_ptr(), corpus.nb()),
                    )
                }
            }
        };

        if !assert_rc(&tag, cn, rn) {
            continue;
        }
        assert!(cn <= cap, "{tag}: dict {cn} exceeds capacity {cap}");
        assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
        if cn > 0 && iter % 2 == 0 {
            verify_dict(&tag, &cb[..cn], &corpus.probes());
        }
    }
}
