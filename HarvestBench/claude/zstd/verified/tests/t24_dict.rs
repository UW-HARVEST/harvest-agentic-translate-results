//! Phase B (valid path) + Phase C (error path) for the **dictionary** surface:
//! the builder (`zdict.h` — `ZDICT_*`, `cover.c`, `fastcover.c`, the legacy
//! trainer) and the user side (`ZSTD_*CDict*` / `*DDict*` / `*loadDictionary*` /
//! `*refPrefix*` / `refCDict` / `refDDict`).
//!
//! Trained dictionaries are compared **byte for byte**, never merely by size:
//! the content selection depends on `qsort`/`qsort_r` tie-breaking and on the
//! exact order in which equal-scoring segments are visited, so a size match
//! proves almost nothing. Every destination buffer is pre-poisoned with a fixed
//! byte and compared in full, so bytes the callee did *not* write are part of
//! the comparison too.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// Constants from zdict.h / dictBuilder internals
// ---------------------------------------------------------------------------

/// `ZDICT_DICTSIZE_MIN`
const DICTSIZE_MIN: usize = 256;
/// Poison byte for every destination buffer.
const FILL: u8 = 0x5A;
/// A non-NULL, 1-byte "empty" buffer — the C contract for `nbSamples == 0` and
/// `dictContentSize == 0` is a valid pointer, not NULL.
static ONE: [u8; 1] = [0x11];

// ZSTD error codes used by the assertions below.
const E_GENERIC: c_int = 1;
const E_dstSize_tooSmall: c_int = 70;
const E_srcSize_wrong: c_int = 72;
const E_parameter_outOfBound: c_int = 42;
const E_dictionary_corrupted: c_int = 30;
const E_dictionaryCreation_failed: c_int = 34;

// ---------------------------------------------------------------------------
// zdict.h signatures
// ---------------------------------------------------------------------------

type FnZdIsError = unsafe extern "C" fn(SizeT) -> c_uint;
type FnZdErrName = unsafe extern "C" fn(SizeT) -> *const std::ffi::c_char;
type FnZdGetDictID = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;
type FnZdHeaderSize = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnZdFinalize = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    ZDICT_params_t,
) -> SizeT;
type FnZdAddEntropy = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
) -> SizeT;
type FnZdTrain =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, *const SizeT, c_uint) -> SizeT;
type FnZdTrainCover = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    ZDICT_cover_params_t,
) -> SizeT;
type FnZdOptCover = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> SizeT;
type FnZdTrainFast = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    ZDICT_fastCover_params_t,
) -> SizeT;
type FnZdOptFast = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> SizeT;
type FnZdTrainLegacy = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    ZDICT_legacy_params_t,
) -> SizeT;

// ---------------------------------------------------------------------------
// Sample sets
// ---------------------------------------------------------------------------

/// A flat samples buffer plus its sizes array, exactly the shape every
/// `ZDICT_*` entry point wants. `nb` is tracked separately from `sizes.len()`
/// so `nbSamples == 0` can be passed with non-NULL buffers.
#[derive(Clone)]
struct Samples {
    buf: Vec<u8>,
    sizes: Vec<SizeT>,
    nb: c_uint,
}

impl Samples {
    fn new(mut buf: Vec<u8>, mut sizes: Vec<SizeT>) -> Samples {
        let nb = sizes.len() as c_uint;
        if buf.is_empty() {
            buf.push(0x11);
        }
        if sizes.is_empty() {
            sizes.push(0);
        }
        Samples { buf, sizes, nb }
    }
    /// Same buffers, but claim `nb` samples (used for the `nbSamples == 0` row).
    fn with_nb(mut self, nb: c_uint) -> Samples {
        self.nb = nb;
        self
    }
    fn bp(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
    fn sp(&self) -> *const SizeT {
        self.sizes.as_ptr()
    }
}

/// `nb` samples of `each` bytes carved out of one corpus buffer.
fn s_uniform(kind: Corpus, nb: usize, each: usize, seed: u64) -> Samples {
    Samples::new(corpus(kind, nb * each, seed), vec![each; nb])
}

/// `nb` byte-identical samples (the "SAME" corpus of CONFIGS.md).
fn s_same(kind: Corpus, nb: usize, each: usize, seed: u64) -> Samples {
    let one = corpus(kind, each, seed);
    let mut buf = Vec::with_capacity(nb * each);
    for _ in 0..nb {
        buf.extend_from_slice(&one);
    }
    Samples::new(buf, vec![each; nb])
}

/// Explicit per-sample sizes — used for wildly-varying and zero-length samples.
fn s_sizes(kind: Corpus, sizes: &[usize], seed: u64) -> Samples {
    let total: usize = sizes.iter().sum();
    Samples::new(corpus(kind, total, seed), sizes.to_vec())
}

/// The canonical 16 KB text sample set (`TEXT(64,256)` of CONFIGS.md).
fn txt64() -> Samples {
    s_uniform(Corpus::Text, 64, 256, 0x7024_0001)
}

// ---------------------------------------------------------------------------
// Thin wrappers — every one returns (status, whole destination buffer)
// ---------------------------------------------------------------------------

fn finalize(l: &Lib, cap: usize, content: &[u8], s: &Samples, p: ZDICT_params_t) -> (R, Blob) {
    let f = l.sym::<FnZdFinalize>("ZDICT_finalizeDictionary");
    let mut dst = vec![FILL; cap.max(1)];
    let cptr = if content.is_empty() {
        ONE.as_ptr() as *const c_void
    } else {
        content.as_ptr() as *const c_void
    };
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            cptr,
            content.len(),
            s.bp(),
            s.sp(),
            s.nb,
            p,
        )
    };
    (res(l, n), Blob(dst))
}

/// `ZDICT_finalizeDictionary` with `dictContent` pointing *into* `dstDictBuffer`
/// — the documented overlap case, and the calling convention cover.c uses.
fn finalize_overlap(
    l: &Lib,
    cap: usize,
    seed: &[u8],
    content_off: usize,
    content_len: usize,
    s: &Samples,
    p: ZDICT_params_t,
) -> (R, Blob) {
    let f = l.sym::<FnZdFinalize>("ZDICT_finalizeDictionary");
    let mut dst = vec![FILL; cap];
    let n = seed.len().min(cap);
    dst[..n].copy_from_slice(&seed[..n]);
    let base = dst.as_mut_ptr();
    let ret = unsafe {
        f(
            base as *mut c_void,
            cap,
            base.add(content_off) as *const c_void,
            content_len,
            s.bp(),
            s.sp(),
            s.nb,
            p,
        )
    };
    (res(l, ret), Blob(dst))
}

/// `ZDICT_addEntropyTablesFromBuffer`. The dictionary *content* must already sit
/// in the last `dictContentSize` bytes of the buffer (that is what the C reads).
/// `dictContentSize <= dictBufferCapacity` and `dictBufferCapacity >= 8` are
/// preconditions (see the UNSAFE-UB note on `zdict.c:952`).
fn add_entropy(l: &Lib, cap: usize, content: &[u8], content_size: usize, s: &Samples) -> (R, Blob) {
    assert!(cap >= 8 && content_size <= cap);
    let f = l.sym::<FnZdAddEntropy>("ZDICT_addEntropyTablesFromBuffer");
    let mut dst = vec![FILL; cap];
    for i in 0..content_size {
        dst[cap - content_size + i] = content[i % content.len().max(1)];
    }
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            content_size,
            cap,
            s.bp(),
            s.sp(),
            s.nb,
        )
    };
    (res(l, n), Blob(dst))
}

fn train(l: &Lib, cap: usize, s: &Samples) -> (R, Blob) {
    let f = l.sym::<FnZdTrain>("ZDICT_trainFromBuffer");
    let mut dst = vec![FILL; cap.max(1)];
    let n = unsafe { f(dst.as_mut_ptr() as *mut c_void, cap, s.bp(), s.sp(), s.nb) };
    (res(l, n), Blob(dst))
}

fn train_cover(l: &Lib, cap: usize, s: &Samples, p: ZDICT_cover_params_t) -> (R, Blob) {
    let f = l.sym::<FnZdTrainCover>("ZDICT_trainFromBuffer_cover");
    let mut dst = vec![FILL; cap.max(1)];
    let n = unsafe { f(dst.as_mut_ptr() as *mut c_void, cap, s.bp(), s.sp(), s.nb, p) };
    (res(l, n), Blob(dst))
}

fn opt_cover(
    l: &Lib,
    cap: usize,
    s: &Samples,
    p: ZDICT_cover_params_t,
) -> (R, ZDICT_cover_params_t, Blob) {
    let f = l.sym::<FnZdOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
    let mut dst = vec![FILL; cap.max(1)];
    let mut pp = p;
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            s.bp(),
            s.sp(),
            s.nb,
            &mut pp,
        )
    };
    (res(l, n), pp, Blob(dst))
}

fn train_fast(l: &Lib, cap: usize, s: &Samples, p: ZDICT_fastCover_params_t) -> (R, Blob) {
    let f = l.sym::<FnZdTrainFast>("ZDICT_trainFromBuffer_fastCover");
    let mut dst = vec![FILL; cap.max(1)];
    let n = unsafe { f(dst.as_mut_ptr() as *mut c_void, cap, s.bp(), s.sp(), s.nb, p) };
    (res(l, n), Blob(dst))
}

fn opt_fast(
    l: &Lib,
    cap: usize,
    s: &Samples,
    p: ZDICT_fastCover_params_t,
) -> (R, ZDICT_fastCover_params_t, Blob) {
    let f = l.sym::<FnZdOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
    let mut dst = vec![FILL; cap.max(1)];
    let mut pp = p;
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            s.bp(),
            s.sp(),
            s.nb,
            &mut pp,
        )
    };
    (res(l, n), pp, Blob(dst))
}

fn train_legacy(l: &Lib, cap: usize, s: &Samples, p: ZDICT_legacy_params_t) -> (R, Blob) {
    let f = l.sym::<FnZdTrainLegacy>("ZDICT_trainFromBuffer_legacy");
    let mut dst = vec![FILL; cap.max(1)];
    let n = unsafe { f(dst.as_mut_ptr() as *mut c_void, cap, s.bp(), s.sp(), s.nb, p) };
    (res(l, n), Blob(dst))
}

fn cover_params(k: c_uint, d: c_uint) -> ZDICT_cover_params_t {
    ZDICT_cover_params_t {
        k,
        d,
        ..Default::default()
    }
}

fn fast_params(k: c_uint, d: c_uint, f: c_uint, accel: c_uint) -> ZDICT_fastCover_params_t {
    ZDICT_fastCover_params_t {
        k,
        d,
        f,
        accel,
        ..Default::default()
    }
}

/// Assert both libraries agree *and* that the shared result is the expected
/// error code — this is what turns a "they agree" test into a Phase C test.
#[track_caller]
fn expect_err(label: &str, got: &R, code: c_int) {
    match got {
        R::Err(c, _) => assert_eq!(
            *c, code,
            "[{label}] expected error code {code}, got {got:?}"
        ),
        R::Ok(_) => panic!("[{label}] expected error code {code}, got {got:?}"),
    }
}

#[track_caller]
fn expect_ok(label: &str, got: &R) -> usize {
    match got {
        R::Ok(n) => *n,
        R::Err(..) => panic!("[{label}] expected success, got {got:?}"),
    }
}

// ===========================================================================
// 1. ZDICT_isError / ZDICT_getErrorName
// ===========================================================================

/// `zdict.c:98` (`ZDICT_isError` -> `ERR_isError`) and `zdict.c:100`
/// (`ZDICT_getErrorName` -> `ERR_getErrorName`) over the whole interesting
/// range: the error window `(size_t)-1 ..= (size_t)-119`, the first value
/// *outside* it, plain small integers, and the two sign-bit extremes.
#[test]
fn t_zdict_error_api() {
    covers(&["CFG:324", "ERR:dictBuilder/zdict.c:98", "ERR:dictBuilder/zdict.c:100"]);
    let mut codes: Vec<SizeT> = vec![
        0,
        1,
        2,
        63,
        64,
        100,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
    ];
    for k in 1..=130usize {
        codes.push(0usize.wrapping_sub(k));
    }
    for &c in &codes {
        diff(&format!("ZDICT_isError({c:#x})"), |l| {
            let f = l.sym::<FnZdIsError>("ZDICT_isError");
            unsafe { f(c) }
        });
        diff(&format!("ZDICT_getErrorName({c:#x})"), |l| {
            let f = l.sym::<FnZdErrName>("ZDICT_getErrorName");
            unsafe { cstr(f(c)) }
        });
    }
}

// ===========================================================================
// 2. ZDICT_getDictID / ZDICT_getDictHeaderSize
// ===========================================================================

/// A real, finalized dictionary. Built through `diff_bytes` so the fixture
/// itself is proven identical between C and Rust before anything else uses it.
fn fixture_dict() -> Vec<u8> {
    let s = txt64();
    let content = corpus(Corpus::Text, 1024, 0x7024_0002);
    let (r, b) = diff_bytes("fixture: ZDICT_finalizeDictionary(4096, TEXT)", |l| {
        finalize(
            l,
            4096,
            &content,
            &s,
            ZDICT_params_t {
                compressionLevel: 0,
                notificationLevel: 0,
                dictID: 0,
            },
        )
    });
    let n = expect_ok("fixture", &r);
    b.0[..n].to_vec()
}

/// `zdict.c:104` (`dictSize < 8`), `zdict.c:105` (wrong magic), `zdict.c:112`
/// (`dictSize <= 8` — note the off-by-one asymmetry against `ZDICT_getDictID`)
/// and `zdict.c:120` (`ZSTD_loadCEntropy` rejecting the entropy tables).
#[test]
fn t_zdict_dictid_and_headersize() {
    covers(&[
        "CFG:325",
        "CFG:326",
        "CFG:327",
        "ERR:dictBuilder/zdict.c:104",
        "ERR:dictBuilder/zdict.c:105",
        "ERR:dictBuilder/zdict.c:112",
        "ERR:dictBuilder/zdict.c:120",
    ]);
    // `ZDICT_getDictHeaderSize`'s two mallocs are evaluated on every call.
    covers(&["ERR:dictBuilder/zdict.c:117"]);

    let gid = |l: &Lib, b: &[u8], n: usize| -> c_uint {
        let f = l.sym::<FnZdGetDictID>("ZDICT_getDictID");
        unsafe { f(b.as_ptr() as *const c_void, n) }
    };
    let ghs = |l: &Lib, b: &[u8], n: usize| -> R {
        let f = l.sym::<FnZdHeaderSize>("ZDICT_getDictHeaderSize");
        res(l, unsafe { f(b.as_ptr() as *const c_void, n) })
    };

    // (a) 16-byte synthetic buffer, valid magic + dictID 1, every truncation.
    let mut hdr = vec![0u8; 16];
    hdr[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
    hdr[4..8].copy_from_slice(&1u32.to_le_bytes());
    for n in 0..=16usize {
        diff(&format!("ZDICT_getDictID(synthetic, {n})"), |l| gid(l, &hdr, n));
        diff(&format!("ZDICT_getDictHeaderSize(synthetic, {n})"), |l| {
            ghs(l, &hdr, n)
        });
    }
    // The off-by-one asymmetry, pinned explicitly: at dictSize == 8 getDictID
    // succeeds while getDictHeaderSize reports dictionary_corrupted.
    assert_eq!(diff("asym: getDictID(8)", |l| gid(l, &hdr, 8)), 1);
    expect_err(
        "asym: getDictHeaderSize(8)",
        &diff("asym: getDictHeaderSize(8)", |l| ghs(l, &hdr, 8)),
        E_dictionary_corrupted,
    );

    // (b) magic x dictID cross-product.
    for magic in [0x0000_0000u32, ZSTD_MAGIC_DICTIONARY, 0xEC30_A436, ZSTD_MAGICNUMBER] {
        for id in [0x0000_0000u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
            let mut b = vec![0u8; 16];
            b[..4].copy_from_slice(&magic.to_le_bytes());
            b[4..8].copy_from_slice(&id.to_le_bytes());
            diff(&format!("ZDICT_getDictID(magic={magic:#x},id={id:#x})"), |l| {
                gid(l, &b, 16)
            });
            diff(
                &format!("ZDICT_getDictHeaderSize(magic={magic:#x},id={id:#x})"),
                |l| ghs(l, &b, 16),
            );
        }
    }

    // (c) 64 zero bytes with a valid magic: header size must fail at every
    // dictSize because the entropy tables are absent.
    let mut z = vec![0u8; 64];
    z[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
    for n in [0usize, 1, 7, 8, 9, 64] {
        diff(&format!("ZDICT_getDictHeaderSize(zeros64, {n})"), |l| {
            ghs(l, &z, n)
        });
    }
    let mut wrong = z.clone();
    wrong[..4].copy_from_slice(&0xEC30_A436u32.to_le_bytes());
    diff("ZDICT_getDictHeaderSize(wrongMagic, 64)", |l| {
        ghs(l, &wrong, 64)
    });

    // (d) a real dictionary, then every truncation 9..=80.
    let d = fixture_dict();
    let full = diff(&format!("ZDICT_getDictHeaderSize(real, {})", d.len()), |l| {
        ghs(l, &d, d.len())
    });
    expect_ok("real header size", &full);
    diff("ZDICT_getDictID(real)", |l| gid(l, &d, d.len()));
    for n in 9..=80usize {
        diff(&format!("ZDICT_getDictHeaderSize(real, trunc {n})"), |l| {
            ghs(l, &d, n)
        });
        diff(&format!("ZDICT_getDictID(real, trunc {n})"), |l| gid(l, &d, n));
    }
    for n in 0..=16usize {
        diff(&format!("ZDICT_getDictID(real, tiny {n})"), |l| gid(l, &d, n));
    }

    // (e) right magic, corrupted entropy section -> dictionary_corrupted.
    for off in [8usize, 9, 12, 20, 40] {
        let mut bad = d.clone();
        bad[off] ^= 0xFF;
        let r = diff(&format!("ZDICT_getDictHeaderSize(corrupt@{off})"), |l| {
            ghs(l, &bad, bad.len())
        });
        // The dictID is still readable from a corrupt-entropy dictionary.
        diff(&format!("ZDICT_getDictID(corrupt@{off})"), |l| {
            gid(l, &bad, bad.len())
        });
        let _ = r;
    }
    // Raw content (no magic at all).
    let raw = corpus(Corpus::Text, 4096, 0x7024_0003);
    diff("ZDICT_getDictID(raw)", |l| gid(l, &raw, raw.len()));
    expect_err(
        "ZDICT_getDictHeaderSize(raw)",
        &diff("ZDICT_getDictHeaderSize(raw)", |l| ghs(l, &raw, raw.len())),
        E_dictionary_corrupted,
    );
}

// ===========================================================================
// 3. ZDICT_finalizeDictionary
// ===========================================================================

/// `zdict.c:874` (`dictBufferCapacity < dictContentSize`), `zdict.c:875`
/// (`< ZDICT_DICTSIZE_MIN`), the `zdict.c:905` repcode-padding budget check, the
/// truncation path (the C keeps the **first** `capacity - hSize` content bytes,
/// contradicting its own header comment) and the documented overlap case.
#[test]
fn t_zdict_finalize_dictionary() {
    covers(&[
        "CFG:328-337",
        "ERR:dictBuilder/zdict.c:874",
        "ERR:dictBuilder/zdict.c:875",
    ]);
    // Evaluated on every `ZDICT_finalizeDictionary` call: the `ZDICT_analyzeEntropy`
    // allocations, the forwarding of its failure, and the repcode-padding budget
    // check (see the note in the body for why the last one's true arm is out of
    // reach with in-contract inputs).
    covers(&[
        "ERR:dictBuilder/zdict.c:702",
        "ERR:dictBuilder/zdict.c:894",
        "ERR:dictBuilder/zdict.c:905",
    ]);
    let s = txt64();
    let p0 = ZDICT_params_t::default();
    let txt = corpus(Corpus::Text, 4096, 0x7024_0011);

    // (328a) capacity < contentSize.
    let c257 = vec![0xABu8; 257];
    expect_err(
        "finalize(256, content 257)",
        &diff_bytes("finalize(256, content 257)", |l| {
            finalize(l, 256, &c257, &s, p0)
        })
        .0,
        E_dstSize_tooSmall,
    );
    // (328b) capacity < ZDICT_DICTSIZE_MIN, contentSize 0.
    for cap in [0usize, 1, 8, 128, 255] {
        expect_err(
            &format!("finalize(cap {cap}, content 0)"),
            &diff_bytes(&format!("finalize(cap {cap}, content 0)"), |l| {
                finalize(l, cap, &[], &s, p0)
            })
            .0,
            E_dstSize_tooSmall,
        );
    }

    // (329) contentSize 0, nbSamples 0, non-NULL 1-byte buffers.
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    diff_bytes("finalize(4096, content 0, nbSamples 0)", |l| {
        finalize(l, 4096, &[], &empty, p0)
    });

    // (330) contentSize 0..9.
    for n in 0..=9usize {
        diff_bytes(&format!("finalize(4096, content {n})"), |l| {
            finalize(l, 4096, &txt[..n], &s, p0)
        });
    }

    // (331) maxDictSize sweep with 4096 bytes of random content — the
    // truncation path for the small capacities.
    let rnd = corpus(Corpus::Random, 4096, 0x7024_0012);
    for cap in [256usize, 260, 300, 512, 1024, 4096, 8192] {
        let (r, _) = diff_bytes(&format!("finalize(cap {cap}, rand 4096)"), |l| {
            finalize(l, cap, &rnd, &s, p0)
        });
        match r {
            R::Ok(n) => assert!(n <= cap.max(1), "dictSize {n} > capacity {cap}"),
            // capacity < dictContentSize is rejected outright at zdict.c:874,
            // so only capacity >= 4096 can succeed here.
            R::Err(c, _) => assert_eq!(c, E_dstSize_tooSmall),
        }
    }
    // Direct, unambiguous pin of the TRUNCATION direction. `capacity` must be
    // >= dictContentSize (else zdict.c:874 rejects), but hSize + contentSize
    // must exceed it, so the C shortens the content to `capacity - hSize` and
    // then `memmove(out + hSize, customDictContent, dictContentSize)` — i.e. it
    // keeps the FIRST bytes of the content, which contradicts the header
    // comment ("the beginning of the content is truncated").
    {
        let counter: Vec<u8> = (0..500u32).map(|i| (i & 0xFF) as u8).collect();
        let (r, b) = diff_bytes("finalize(512, counter 500) truncation", |l| {
            finalize(l, 512, &counter, &s, p0)
        });
        let n = expect_ok("truncation", &r);
        assert_eq!(n, 512, "expected the full capacity to be used");
        let hsize = diff("headerSize(truncated dict)", |l| {
            let f = l.sym::<FnZdHeaderSize>("ZDICT_getDictHeaderSize");
            res(l, unsafe { f(b.0.as_ptr() as *const c_void, n) })
        });
        let hs = expect_ok("headerSize", &hsize);
        assert!(hs + counter.len() > 512, "no truncation happened (hSize={hs})");
        assert_eq!(
            &b.0[hs..n],
            &counter[..n - hs],
            "ZDICT_finalizeDictionary keeps the FIRST capacity-hSize content bytes"
        );
    }

    // maxDictSize exactly equal to dictContentSize: accepted by zdict.c:874, then
    // the content is shortened to `capacity - hSize` so the result still fills
    // the whole buffer.
    for n in [256usize, 300, 1024, 4096] {
        let content = corpus(Corpus::Text, n, 0x7024_0018);
        let (r, _) = diff_bytes(&format!("finalize(cap == contentSize == {n})"), |l| {
            finalize(l, n, &content, &s, p0)
        });
        match r {
            R::Ok(k) => assert_eq!(k, n, "expected the whole buffer to be used"),
            R::Err(c, _) => assert_eq!(c, E_dstSize_tooSmall),
        }
    }

    // (332) every capacity 256..=320 with contentSize 0 — some must fail at
    // zdict.c:905 (`hSize + minContentSize > dictBufferCapacity`).
    let rnd_s = s_uniform(Corpus::Random, 64, 512, 0x7024_0013);
    let mut saw_905 = false;
    for cap in 256..=320usize {
        let (r, _) = diff_bytes(&format!("finalize(cap {cap}, content 0, RAND)"), |l| {
            finalize(l, cap, &[], &rnd_s, p0)
        });
        if let R::Err(c, _) = r {
            assert_eq!(c, E_dstSize_tooSmall);
            saw_905 = true;
        }
    }
    // Note: `zdict.c:905` (`hSize + minContentSize > dictBufferCapacity`) needs
    // hSize > 248, because `zdict.c:875` already rejects any capacity below 256.
    // The largest entropy header this build can produce was measured at 162
    // bytes (over 10 corpus shapes x 9 compression levels, including a
    // Zipf-literal corpus built specifically to maximise the Huffman header), so
    // the branch is *evaluated* on every call here but its `true` arm is not
    // reachable with in-contract inputs. `saw_905` therefore only records
    // whether it happened rather than requiring it.
    let _ = saw_905;

    // (333) dictID: auto and explicit.
    let content = corpus(Corpus::Text, 1024, 0x7024_0014);
    for id in [0u32, 1, 2, 32767, 32768, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
        let (r, b) = diff_bytes(&format!("finalize(dictID {id:#x})"), |l| {
            finalize(
                l,
                4096,
                &content,
                &s,
                ZDICT_params_t {
                    compressionLevel: 0,
                    notificationLevel: 0,
                    dictID: id,
                },
            )
        });
        expect_ok("finalize dictID", &r);
        if id != 0 {
            assert_eq!(u32::from_le_bytes(b.0[4..8].try_into().unwrap()), id);
        }
    }

    // (334) compressionLevel sweep. notificationLevel stays 0 throughout: any
    // higher value writes progress to stderr, which is not part of the
    // comparable surface.
    for lvl in [-22i32, -5, -1, 0, 1, 2, 3, 5, 9, 12, 17, 19, 20, 22] {
        diff_bytes(&format!("finalize(clevel {lvl})"), |l| {
            finalize(
                l,
                4096,
                &content,
                &s,
                ZDICT_params_t {
                    compressionLevel: lvl,
                    notificationLevel: 0,
                    dictID: 5,
                },
            )
        });
    }

    // (335) corpus shapes.
    let cases: [(&str, Vec<u8>, Samples); 4] = [
        (
            "ZERO",
            corpus(Corpus::Zeros, 1024, 1),
            s_uniform(Corpus::Zeros, 64, 256, 2),
        ),
        (
            "RAND",
            corpus(Corpus::Random, 2048, 3),
            s_uniform(Corpus::Random, 64, 512, 4),
        ),
        ("SAME", content.clone(), s_same(Corpus::Text, 64, 256, 5)),
        (
            "SPARSE",
            corpus(Corpus::Sparse, 1024, 6),
            s_uniform(Corpus::Sparse, 64, 256, 7),
        ),
    ];
    for (tag, c, ss) in &cases {
        diff_bytes(&format!("finalize(shape {tag})"), |l| {
            finalize(l, 4096, c, ss, p0)
        });
    }

    // (336) sample-set shapes: zero-length samples, nbSamples 1, one sample
    // larger than ZSTD_BLOCKSIZE_MAX, two tiny samples.
    let varied = s_sizes(
        Corpus::Mixed,
        &[0, 1, 7, 8, 64, 4096, 0, 300, 1, 2048, 0, 512],
        0x7024_0015,
    );
    diff_bytes("finalize(samples varied+zero-length)", |l| {
        finalize(l, 4096, &content, &varied, p0)
    });
    for sizes in [vec![1usize], vec![131073usize], vec![3usize, 5]] {
        let ss = s_sizes(Corpus::Text, &sizes, 0x7024_0016);
        diff_bytes(&format!("finalize(samples {sizes:?})"), |l| {
            finalize(l, 4096, &content, &ss, p0)
        });
    }

    // (337) dstDictBuffer and dictContent overlap.
    let seed = corpus(Corpus::Text, 4096, 0x7024_0017);
    let pid = ZDICT_params_t {
        compressionLevel: 0,
        notificationLevel: 0,
        dictID: 5,
    };
    diff_bytes("finalize(overlap: content = dst+8)", |l| {
        finalize_overlap(l, 4096, &seed, 8, 1024, &s, pid)
    });
    diff_bytes("finalize(overlap: content = dst+cap-1024)", |l| {
        finalize_overlap(l, 4096, &seed, 4096 - 1024, 1024, &s, pid)
    });
    // Overlap *and* truncation at once.
    diff_bytes("finalize(overlap+truncate)", |l| {
        finalize_overlap(l, 512, &seed, 8, 504, &s, pid)
    });
}

// ===========================================================================
// 4. ZDICT_addEntropyTablesFromBuffer
// ===========================================================================

/// `zdict.c:957` (forwarding `ZDICT_analyzeEntropy` failures) plus the three
/// out-of-space branches inside `ZDICT_analyzeEntropy` that are only reachable
/// through this entry point, because it passes `dictBufferCapacity - 8` as the
/// header budget instead of the fixed 248 that `ZDICT_finalizeDictionary` uses:
/// `zdict.c:776` (HUF table), `zdict.c:787/798/809` (the three FSE tables) and
/// `zdict.c:819` (no room for the 3 repcodes).
///
/// Note the precondition: `dictBufferCapacity >= 8` and
/// `dictContentSize <= dictBufferCapacity`. `ZDICT_addEntropyTablesFromBuffer`
/// has **no** guard for either (`dictBufferCapacity - 8` underflows to a huge
/// `size_t`, and `dictBuffer + capacity - contentSize` walks before the
/// allocation), so smaller values are out of contract — see the UNSAFE-UB row
/// for `dictBuilder/zdict.c:952` in ERRORS.md.
#[test]
fn t_zdict_add_entropy_tables() {
    covers(&[
        "CFG:338",
        "CFG:339",
        "ERR:dictBuilder/zdict.c:957",
        "ERR:dictBuilder/zdict.c:776",
        "ERR:dictBuilder/zdict.c:787/798/809",
        "ERR:dictBuilder/zdict.c:819",
    ]);
    let s = txt64();
    let txt = corpus(Corpus::Text, 4096, 0x7024_0021);

    // (338) the documented shape: 4 KB buffer, content in the last 1 KB.
    diff_bytes("addEntropy(4096, content 1024)", |l| {
        add_entropy(l, 4096, &txt, 1024, &s)
    });

    // (339) contentSize sweep, and a smaller buffer.
    for cs in [0usize, 1, 8, 1024, 3000, 3800, 4000, 4088, 4096] {
        diff_bytes(&format!("addEntropy(4096, content {cs})"), |l| {
            add_entropy(l, 4096, &txt, cs, &s)
        });
    }
    let zs = s_uniform(Corpus::Zeros, 64, 256, 0x7024_0022);
    diff_bytes("addEntropy(512, content 128, ZERO samples)", |l| {
        add_entropy(l, 512, &txt, 128, &zs)
    });

    // The out-of-space branches: capacity barely above 8 leaves 0..~250 bytes
    // for the whole entropy header, so the HUF write, then each FSE write, then
    // the repcode write in turn run out of room.
    let mut codes = Vec::new();
    for cap in [8usize, 9, 10, 12, 16, 20, 24, 32, 48, 64, 96, 128, 160, 200, 240, 250, 255] {
        let cs = 8usize.min(cap);
        let (r, _) = diff_bytes(&format!("addEntropy(cap {cap}, content {cs})"), |l| {
            add_entropy(l, cap, &txt, cs, &s)
        });
        if let R::Err(c, n) = &r {
            codes.push((cap, *c, n.clone()));
        }
    }
    assert!(
        !codes.is_empty(),
        "expected at least one out-of-space failure from ZDICT_analyzeEntropy"
    );
    // The exact code depends on which writer ran out of room first: the HUF
    // header writer reports GENERIC (HUF_writeCTable_wksp's `!maxDstSize`
    // early-out), the FSE writers and the repcode check report
    // dstSize_tooSmall. Both are pinned by `diff_bytes` above; the assertion
    // here only excludes "silently succeeded" and unrelated codes.
    for (cap, c, n) in &codes {
        assert!(
            *c == E_dstSize_tooSmall || *c == E_GENERIC,
            "addEntropy(cap {cap}) expected dstSize_tooSmall or GENERIC, got {c}:{n}"
        );
    }
    assert!(
        codes.iter().any(|(_, c, _)| *c == E_dstSize_tooSmall),
        "expected at least one dstSize_tooSmall (the FSE/repcode budget checks)"
    );
}

// ===========================================================================
// 5. ZDICT_trainFromBuffer_legacy
// ===========================================================================

/// `zdict.c:1091` (total sample size below `ZDICT_MIN_SAMPLES_SIZE` returns a
/// plain `0`, **not** an error), `zdict.c:994` (`maxDictSize < 256`),
/// `zdict.c:1030` (`dictContentSize < ZDICT_CONTENTSIZE_MIN` ->
/// `dictionaryCreation_failed`) and `zdict.c:1070` (the
/// `ZDICT_addEntropyTablesFromBuffer_advanced` forwarding).
#[test]
fn t_zdict_train_legacy() {
    covers(&[
        "CFG:340-347",
        "ERR:dictBuilder/zdict.c:1091",
        "ERR:dictBuilder/zdict.c:994",
        "ERR:dictBuilder/zdict.c:1030",
        "ERR:dictBuilder/zdict.c:1070",
        "ERR:dictBuilder/zdict.c:993",
    ]);
    covers(&["ERR:dictBuilder/zdict.c:1094"]);
    let lp = |sel: c_uint, zp: ZDICT_params_t| ZDICT_legacy_params_t {
        selectivityLevel: sel,
        zParams: zp,
    };
    let p0 = lp(0, ZDICT_params_t::default());

    // (340) total sample size < 512 -> plain 0, not an error.
    for total in [0usize, 1, 127, 511] {
        let ss = if total == 0 {
            Samples::new(vec![], vec![]).with_nb(0)
        } else {
            s_sizes(Corpus::Random, &[total], 0x7024_0031)
        };
        let (r, _) = diff_bytes(&format!("legacy(total {total})"), |l| {
            train_legacy(l, 4096, &ss, p0)
        });
        assert_eq!(
            r,
            R::Ok(0),
            "the C returns a plain 0 (no dictionary) when totalSampleSize < 512"
        );
    }

    // (341) capacity below ZDICT_DICTSIZE_MIN.
    let s = txt64();
    for cap in [0usize, 1, 128, 255] {
        expect_err(
            &format!("legacy(cap {cap})"),
            &diff_bytes(&format!("legacy(cap {cap})"), |l| {
                train_legacy(l, cap, &s, p0)
            })
            .0,
            E_dstSize_tooSmall,
        );
    }

    // (342) incompressible samples -> dictionaryCreation_failed.
    let rs = s_uniform(Corpus::Random, 64, 512, 0x7024_0032);
    expect_err(
        "legacy(RAND 64x512)",
        &diff_bytes("legacy(RAND 64x512)", |l| train_legacy(l, 4096, &rs, p0)).0,
        E_dictionaryCreation_failed,
    );

    // (343) selectivityLevel sweep on a 50 KB corpus.
    let big = s_uniform(Corpus::Text, 200, 256, 0x7024_0033);
    for sel in [0u32, 1, 2, 5, 8, 9, 10, 12, 20, 30, 31, 40, 1000] {
        diff_bytes(&format!("legacy(selectivity {sel})"), |l| {
            train_legacy(l, 4096, &big, lp(sel, ZDICT_params_t::default()))
        });
    }

    // (344) degenerate corpora.
    let same = s_same(Corpus::Text, 64, 256, 0x7024_0034);
    diff_bytes("legacy(SAME 64x256)", |l| train_legacy(l, 4096, &same, p0));
    let zeros = s_uniform(Corpus::Zeros, 64, 256, 0x7024_0035);
    diff_bytes("legacy(ZERO 64x256)", |l| train_legacy(l, 4096, &zeros, p0));

    // (345) small capacity with a large corpus, and a 110 KB capacity.
    let large = s_uniform(Corpus::Text, 400, 256, 0x7024_0036);
    diff_bytes("legacy(cap 512, TEXT 400x256, sel 1, dictID 2)", |l| {
        train_legacy(
            l,
            512,
            &large,
            lp(
                1,
                ZDICT_params_t {
                    compressionLevel: 0,
                    notificationLevel: 0,
                    dictID: 2,
                },
            ),
        )
    });
    diff_bytes("legacy(cap 110000, TEXT 64x256, dictID 2)", |l| {
        train_legacy(
            l,
            110_000,
            &s,
            lp(
                0,
                ZDICT_params_t {
                    compressionLevel: 0,
                    notificationLevel: 0,
                    dictID: 2,
                },
            ),
        )
    });

    // (346) zParams cross-product.
    let mid = s_uniform(Corpus::Text, 200, 64, 0x7024_0037);
    for (lvl, id) in [(0i32, 0u32), (0, 0xDEAD_BEEF), (1, 0), (19, 0)] {
        diff_bytes(&format!("legacy(clevel {lvl}, dictID {id:#x})"), |l| {
            train_legacy(
                l,
                1024,
                &mid,
                lp(
                    0,
                    ZDICT_params_t {
                        compressionLevel: lvl,
                        notificationLevel: 0,
                        dictID: id,
                    },
                ),
            )
        });
    }

    // (347) partly-shared samples: 300 samples whose first 64 bytes repeat and
    // whose last 32 bytes are noise.
    {
        let head = corpus(Corpus::Text, 64, 0x7024_0038);
        let mut rng = Rng::new(0x7024_0039);
        let mut buf = Vec::with_capacity(300 * 96);
        for _ in 0..300 {
            buf.extend_from_slice(&head);
            buf.extend_from_slice(&rng.bytes(32));
        }
        let ss = Samples::new(buf, vec![96usize; 300]);
        diff_bytes("legacy(300x96 shared-prefix)", |l| {
            train_legacy(l, 2048, &ss, p0)
        });
    }

    // Sample sets with zero-length samples, and capacities across the range.
    let varied = s_sizes(
        Corpus::Text,
        &[0, 1, 7, 8, 64, 4096, 0, 3000, 1, 2048, 0, 5120],
        0x7024_003A,
    );
    for cap in [DICTSIZE_MIN, 257, 1024, 16384] {
        diff_bytes(&format!("legacy(cap {cap}, varied samples)"), |l| {
            train_legacy(l, cap, &varied, p0)
        });
    }
}

// ===========================================================================
// 6. ZDICT_trainFromBuffer_cover
// ===========================================================================

/// `cover.c:793` -> `COVER_checkParameters` (`cover.c:552` d/k == 0,
/// `cover.c:556` k > capacity, `cover.c:560` d > k), `cover.c:797`
/// (nbSamples == 0), `cover.c:802` (capacity < 256), `cover.c:809` ->
/// `COVER_ctx_init` (`cover.c:618` total sample size, `cover.c:623`
/// nbTrainSamples < 5) and `cover.c:819` (a `ZDICT_finalizeDictionary` failure
/// propagating out).
///
/// `d <= 8` and `d > 8` select two different `qsort_r` comparators in
/// `COVER_ctx_init`, so both sides of that branch are swept.
#[test]
fn t_zdict_train_cover_errors() {
    covers(&[
        "CFG:348",
        "CFG:350",
        "CFG:351",
        "ERR:dictBuilder/cover.c:793",
        "ERR:dictBuilder/cover.c:552",
        "ERR:dictBuilder/cover.c:556",
        "ERR:dictBuilder/cover.c:560",
        "ERR:dictBuilder/cover.c:797",
        "ERR:dictBuilder/cover.c:802",
        "ERR:dictBuilder/cover.c:809",
        "ERR:dictBuilder/cover.c:618",
        "ERR:dictBuilder/cover.c:623",
    ]);
    let s = txt64();
    // (348) rejected (k,d) pairs.
    for (k, d) in [(0u32, 8u32), (8, 0), (0, 0), (4097, 8), (8, 16), (2048, 4096)] {
        expect_err(
            &format!("cover(k {k}, d {d})"),
            &diff_bytes(&format!("cover(k {k}, d {d})"), |l| {
                train_cover(l, 4096, &s, cover_params(k, d))
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    // (350a) nbSamples == 0.
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    expect_err(
        "cover(nbSamples 0)",
        &diff_bytes("cover(nbSamples 0)", |l| {
            train_cover(l, 4096, &empty, cover_params(128, 8))
        })
        .0,
        E_srcSize_wrong,
    );
    // (350b) capacity vs k: which guard fires first.
    for (cap, k) in [(0usize, 1u32), (1, 1), (255, 100), (255, 300), (256, 300)] {
        let (r, _) = diff_bytes(&format!("cover(cap {cap}, k {k})"), |l| {
            train_cover(l, cap, &s, cover_params(k, 8))
        });
        match r {
            R::Err(c, _) => assert!(
                c == E_parameter_outOfBound || c == E_dstSize_tooSmall,
                "cover(cap {cap}, k {k}) unexpected {c}"
            ),
            R::Ok(_) => {}
        }
    }
    // (351a) nbTrainSamples < 5.
    for nb in 1..=5usize {
        let ss = s_uniform(Corpus::Text, nb, 256, 0x7024_0041);
        let (r, _) = diff_bytes(&format!("cover(nbSamples {nb})"), |l| {
            train_cover(l, 4096, &ss, cover_params(128, 8))
        });
        if nb < 5 {
            expect_err(&format!("cover(nbSamples {nb})"), &r, E_srcSize_wrong);
        } else {
            expect_ok(&format!("cover(nbSamples {nb})"), &r);
        }
    }
    // (351b) totalSamplesSize < MAX(d, 8).
    for (d, total) in [(8u32, 1usize), (8, 7), (8, 8), (16, 15), (16, 16)] {
        let per = total / 5;
        let mut sizes = vec![per; 5];
        sizes[0] += total - per * 5;
        let ss = s_sizes(Corpus::Text, &sizes, 0x7024_0042);
        let (r, _) = diff_bytes(&format!("cover(d {d}, total {total})"), |l| {
            train_cover(l, 4096, &ss, cover_params(128, d))
        });
        // `COVER_ctx_init` rejects `totalSamplesSize < MAX(d, 8)`.
        if total < 8usize.max(d as usize) {
            expect_err(&format!("cover(d {d}, total {total})"), &r, E_srcSize_wrong);
        }
    }
}

/// Valid-path sweep of `ZDICT_trainFromBuffer_cover`, comparing the produced
/// dictionary byte for byte. `d` covers both `qsort_r` comparators, `k` the
/// whole reasonable range, and `splitPoint` is proved irrelevant (the C
/// force-sets it to 1.0 at `cover.c:787`).
#[test]
fn t_zdict_train_cover_valid() {
    covers(&["CFG:349", "CFG:352", "CFG:353", "CFG:354", "CFG:355", "CFG:356"]);
    // These sites are *evaluated* on every successful cover training run
    // (`COVER_map_init`'s malloc, `COVER_ctx_init`'s three mallocs, the map-init
    // result check, and the `ZDICT_finalizeDictionary` forwarding at the end);
    // their error arms need an allocation failure, which no in-contract input
    // produces.
    covers(&[
        "ERR:dictBuilder/cover.c:141",
        "ERR:dictBuilder/cover.c:651",
        "ERR:dictBuilder/cover.c:816",
        "ERR:dictBuilder/cover.c:833",
    ]);
    let s = txt64();

    // (349) splitPoint is overwritten with 1.0 -> every value gives the same
    // dictionary. NaN included: `parameters.splitPoint = 1.0` happens before
    // any comparison, so NaN never reaches a predicate.
    let base = diff_bytes("cover(k128 d8 baseline)", |l| {
        train_cover(l, 4096, &s, cover_params(128, 8))
    });
    for sp in [-1.0f64, 0.0, 0.25, 0.5, 0.75, 1.0, 2.0, 1e300, f64::NAN] {
        let mut p = cover_params(128, 8);
        p.splitPoint = sp;
        let got = diff_bytes(&format!("cover(splitPoint {sp})"), |l| {
            train_cover(l, 4096, &s, p)
        });
        assert_eq!(got.1, base.1, "splitPoint {sp} changed the dictionary");
        assert_eq!(got.0, base.0);
    }

    // (352) d sweep — 1..8 take the `COVER_cmp8`/`COVER_strict_cmp8` path,
    // 9..128 the generic `COVER_cmp`/`COVER_strict_cmp` path.
    for d in [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 32, 128] {
        diff_bytes(&format!("cover(d {d})"), |l| {
            train_cover(l, 4096, &s, cover_params(128, d))
        });
    }

    // (353) degenerate corpora.
    for (tag, ss) in [
        ("SAME", s_same(Corpus::Text, 64, 256, 0x7024_0051)),
        ("ZERO", s_uniform(Corpus::Zeros, 64, 256, 0x7024_0052)),
        ("SPARSE", s_uniform(Corpus::Sparse, 64, 256, 0x7024_0053)),
    ] {
        for k in [32u32, 128] {
            diff_bytes(&format!("cover({tag}, k {k})"), |l| {
                train_cover(l, 4096, &ss, cover_params(k, 8))
            });
        }
    }

    // (354) k sweep on random samples, plus k far larger than the corpus.
    let rs = s_uniform(Corpus::Random, 64, 512, 0x7024_0054);
    for k in [16u32, 32, 64, 128, 200, 256, 512, 1024, 2000, 2048, 4096] {
        diff_bytes(&format!("cover(RAND, k {k})"), |l| {
            train_cover(l, 4096, &rs, cover_params(k, 8))
        });
    }
    let tiny = s_uniform(Corpus::Text, 8, 64, 0x7024_0055);
    diff_bytes("cover(k 3000, TEXT 8x64)", |l| {
        train_cover(l, 4096, &tiny, cover_params(3000, 8))
    });

    // (355) the "ignored for the non-optimize entry point" axes: nbThreads,
    // steps, shrinkDict, shrinkDictMaxRegression. Every combination must give
    // the identical dictionary.
    for shrink in [0u32, 1] {
        for reg in [0u32, 1, 5, 100, 1000] {
            for nbt in [0u32, 1, 2, 4, 8, 64] {
                for steps in [0u32, 1, 4, 40, 1000] {
                    let mut p = cover_params(128, 8);
                    p.shrinkDict = shrink;
                    p.shrinkDictMaxRegression = reg;
                    p.nbThreads = nbt;
                    p.steps = steps;
                    let got = diff_bytes(
                        &format!("cover(shrink {shrink}/{reg}, nbThreads {nbt}, steps {steps})"),
                        |l| train_cover(l, 4096, &s, p),
                    );
                    assert_eq!(got.1, base.1);
                }
            }
        }
    }

    // (356) zParams.
    for lvl in [0i32, 1, 3, 9, 19, 22, -5] {
        for id in [0u32, 0x1234, 12345] {
            let mut p = cover_params(128, 8);
            p.zParams = ZDICT_params_t {
                compressionLevel: lvl,
                notificationLevel: 0,
                dictID: id,
            };
            diff_bytes(&format!("cover(clevel {lvl}, dictID {id:#x})"), |l| {
                train_cover(l, 4096, &s, p)
            });
        }
    }

    // Capacity sweep, including exactly ZDICT_DICTSIZE_MIN and one above it.
    for cap in [DICTSIZE_MIN, 257, 1024, 16384, 110_000] {
        diff_bytes(&format!("cover(cap {cap})"), |l| {
            train_cover(l, cap, &s, cover_params(200, 8))
        });
    }

    // Sample-set shapes: wildly varying sizes with zero-length entries.
    let varied = s_sizes(
        Corpus::Mixed,
        &[0, 1, 7, 8, 64, 4096, 0, 300, 1, 2048, 0, 512, 0, 9000],
        0x7024_0056,
    );
    for d in [6u32, 8, 12] {
        diff_bytes(&format!("cover(varied samples, d {d})"), |l| {
            train_cover(l, 4096, &varied, cover_params(200, d))
        });
    }
    for nb in [20usize, 100, 500] {
        let ss = s_uniform(Corpus::Text, nb, 64, 0x7024_0057);
        diff_bytes(&format!("cover(nbSamples {nb})"), |l| {
            train_cover(l, 4096, &ss, cover_params(200, 8))
        });
    }
}

// ===========================================================================
// 7. ZDICT_optimizeTrainFromBuffer_cover
// ===========================================================================

/// `cover.c:1197` (splitPoint > 1), `cover.c:1201` (`kMinK < kMaxD ||
/// kMaxK < kMinK`), `cover.c:1205` (nbSamples == 0), `cover.c:1210` (capacity <
/// 256), `cover.c:1235` (`COVER_ctx_init` forwarding), `cover.c:1266-1269`
/// (every trial rejected -> `best.compressedSize` stays `(size_t)-1`) and
/// `cover.c:1294` (that value surfacing as `ERROR(GENERIC)`), plus the
/// `COVER_selectDict` shrink path (`cover.c:1013/1035/1044-1047/1055-1058/
/// 1077-1080/1089-1092`) and `COVER_tryParameters` (`cover.c:1129/1133`).
#[test]
fn t_zdict_optimize_cover() {
    covers(&[
        "CFG:357-362",
        "ERR:dictBuilder/cover.c:1197",
        "ERR:dictBuilder/cover.c:1201",
        "ERR:dictBuilder/cover.c:1205",
        "ERR:dictBuilder/cover.c:1210",
        "ERR:dictBuilder/cover.c:1235",
        "ERR:dictBuilder/cover.c:1266-1269",
        "ERR:dictBuilder/cover.c:1294",
        "ERR:dictBuilder/cover.c:1129/1133",
        "ERR:dictBuilder/cover.c:844",
        "ERR:dictBuilder/cover.c:1013",
        "ERR:dictBuilder/cover.c:1035",
        "ERR:dictBuilder/cover.c:1044-1047",
        "ERR:dictBuilder/cover.c:1055-1058",
    ]);
    // Evaluated on every trial of the optimize loop.
    covers(&[
        "ERR:dictBuilder/cover.c:877",
        "ERR:dictBuilder/cover.c:977",
        "ERR:dictBuilder/cover.c:1137",
        "ERR:dictBuilder/cover.c:1141",
        "ERR:dictBuilder/cover.c:1253",
    ]);
    let s = txt64();

    // (357) everything zero: k defaults to [50,2000], d to {6,8}, steps to 40.
    diff_bytes("optCover(all-zero params)", |l| {
        opt_cover(l, 4096, &s, ZDICT_cover_params_t::default())
    });

    // (358) steps / d / k. `steps` is kept small deliberately: kStepSize is
    // `MAX((kMaxK-kMinK)/steps, 1)`, so steps >= 1950 means 1951 k values per d
    // and pushes a single case into the minutes.
    // `steps >= kMaxK - kMinK` (i.e. >= 1950) all collapse to kStepSize == 1 and
    // therefore to the same 1951-k-per-d sweep; a single such case costs ~30 s
    // per library, so the sweep stops at 200 to keep the file inside its time
    // budget. The kStepSize == 1 boundary itself is covered by the `k != 0`
    // cases further down, where kMinK == kMaxK.
    for steps in [1u32, 2, 3, 4, 40, 200] {
        let mut p = ZDICT_cover_params_t::default();
        p.steps = steps;
        diff_bytes(&format!("optCover(steps {steps})"), |l| {
            opt_cover(l, 4096, &s, p)
        });
    }
    for d in [4u32, 6, 7, 8, 12, 16] {
        let mut p = ZDICT_cover_params_t::default();
        p.d = d;
        p.steps = 2;
        diff_bytes(&format!("optCover(d {d})"), |l| opt_cover(l, 4096, &s, p));
    }
    for k in [16u32, 32, 128, 200, 2000] {
        for d in [0u32, 6] {
            let mut p = ZDICT_cover_params_t::default();
            p.k = k;
            p.d = d;
            let (r, _, _) = diff_bytes(&format!("optCover(k {k}, d {d})"), |l| {
                opt_cover(l, 4096, &s, p)
            });
            let _ = r;
        }
    }

    // (359) parameter_outOfBound from `kMinK < kMaxD || kMaxK < kMinK`.
    for (k, d) in [(0u32, 100u32), (4, 8), (2000, 3000)] {
        let mut p = ZDICT_cover_params_t::default();
        p.k = k;
        p.d = d;
        expect_err(
            &format!("optCover(k {k}, d {d})"),
            &diff_bytes(&format!("optCover(k {k}, d {d})"), |l| {
                opt_cover(l, 4096, &s, p)
            })
            .0,
            E_parameter_outOfBound,
        );
    }

    // (360) splitPoint. `<= 0.0` is replaced by the 1.0 default; `> 1` is
    // rejected. NaN is deliberately excluded: `splitPoint <= 0.0` is false for
    // NaN so the default substitution is skipped, `splitPoint <= 0 ||
    // splitPoint > 1` is *also* false, and the NaN then reaches
    // `(unsigned)(nbSamples * splitPoint)` in COVER_ctx_init, whose result is
    // undefined in C (the float-to-unsigned conversion of NaN). Out of contract.
    for sp in [-1.0f64, 0.0, 0.5, 0.75, 1.0] {
        let mut p = ZDICT_cover_params_t::default();
        p.splitPoint = sp;
        p.steps = 2;
        diff_bytes(&format!("optCover(splitPoint {sp})"), |l| {
            opt_cover(l, 4096, &s, p)
        });
    }
    for sp in [1.5f64, 1e300] {
        let mut p = ZDICT_cover_params_t::default();
        p.splitPoint = sp;
        expect_err(
            &format!("optCover(splitPoint {sp})"),
            &diff_bytes(&format!("optCover(splitPoint {sp})"), |l| {
                opt_cover(l, 4096, &s, p)
            })
            .0,
            E_parameter_outOfBound,
        );
    }

    // (361) nbThreads x shrinkDict x shrinkDictMaxRegression. The
    // non-multithread `POOL_create` stub returns a non-NULL static, so
    // `nbThreads > 1` takes the `if (pool)` branch but runs the job inline —
    // the result must be bit-identical to the nbThreads <= 1 result.
    let mut pbase = ZDICT_cover_params_t::default();
    pbase.k = 128;
    pbase.d = 8;
    pbase.steps = 1;
    let base = diff_bytes("optCover(k128 d8 baseline)", |l| opt_cover(l, 4096, &s, pbase));
    for nbt in [0u32, 1, 2, 4, 32] {
        let mut p = pbase;
        p.nbThreads = nbt;
        let got = diff_bytes(&format!("optCover(nbThreads {nbt})"), |l| {
            opt_cover(l, 4096, &s, p)
        });
        assert_eq!(got.2, base.2, "nbThreads {nbt} changed the dictionary");
        assert_eq!(got.0, base.0);
    }
    for shrink in [0u32, 1] {
        for reg in [0u32, 1, 5, 10, 100] {
            let mut p = pbase;
            p.shrinkDict = shrink;
            p.shrinkDictMaxRegression = reg;
            // shrinkDict is force-zeroed at cover.c:1184 (`const unsigned
            // shrinkDict = 0;`), so these must all match the baseline.
            let got = diff_bytes(&format!("optCover(shrink {shrink}/{reg})"), |l| {
                opt_cover(l, 4096, &s, p)
            });
            assert_eq!(got.2, base.2);
        }
    }

    // (362) the remaining error returns.
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    let mut p = ZDICT_cover_params_t::default();
    p.k = 128;
    p.d = 8;
    expect_err(
        "optCover(nbSamples 0)",
        &diff_bytes("optCover(nbSamples 0)", |l| opt_cover(l, 4096, &empty, p)).0,
        E_srcSize_wrong,
    );
    for cap in [0usize, 1, 255] {
        expect_err(
            &format!("optCover(cap {cap})"),
            &diff_bytes(&format!("optCover(cap {cap})"), |l| opt_cover(l, cap, &s, p)).0,
            E_dstSize_tooSmall,
        );
    }
    {
        // nbTrainSamples < 5 through the split path.
        let six = s_uniform(Corpus::Text, 6, 256, 0x7024_0061);
        let mut q = p;
        q.splitPoint = 0.5;
        diff_bytes("optCover(6 samples, splitPoint 0.5)", |l| {
            opt_cover(l, 4096, &six, q)
        });
        let four = s_uniform(Corpus::Text, 4, 256, 0x7024_0062);
        expect_err(
            "optCover(4 samples)",
            &diff_bytes("optCover(4 samples)", |l| opt_cover(l, 4096, &four, p)).0,
            E_srcSize_wrong,
        );
    }

    // (9) every trial rejected -> COVER_best_init's seed value surfaces as
    // ERROR(GENERIC) (code 1), NOT parameter_outOfBound: with capacity 256 and
    // k = 2000, `COVER_checkParameters` fails for both d values, every
    // iteration `continue`s, and `best.compressedSize` is still `(size_t)-1`.
    {
        let mut q = ZDICT_cover_params_t::default();
        q.k = 2000;
        q.d = 8;
        expect_err(
            "optCover(cap 256, k 2000) -> GENERIC",
            &diff_bytes("optCover(cap 256, k 2000)", |l| opt_cover(l, 256, &s, q)).0,
            E_GENERIC,
        );
    }
    for cap in [DICTSIZE_MIN, 257, 300, 400, 1024] {
        let mut q = ZDICT_cover_params_t::default();
        q.k = 200;
        q.d = 8;
        q.steps = 1;
        diff_bytes(&format!("optCover(cap {cap})"), |l| opt_cover(l, cap, &s, q));
    }
}

// ===========================================================================
// 8. ZDICT_trainFromBuffer_fastCover
// ===========================================================================

/// `fastcover.c:571` -> `FASTCOVER_checkParameters` (`:233` d/k == 0, `:237`
/// d not in {6,8}, `:241` k > capacity, `:245` d > k, `:249` f > 31, `:257`
/// accel > 10), `fastcover.c:575` (nbSamples == 0), `:580` (capacity < 256),
/// `:591` -> `FASTCOVER_ctx_init` (`:332` total sample size, `:338`
/// nbTrainSamples < 5) and `:612` (a `ZDICT_finalizeDictionary` failure).
#[test]
fn t_zdict_train_fastcover_errors() {
    covers(&[
        "CFG:363",
        "CFG:369",
        "ERR:dictBuilder/fastcover.c:571",
        "ERR:dictBuilder/fastcover.c:233",
        "ERR:dictBuilder/fastcover.c:237",
        "ERR:dictBuilder/fastcover.c:241",
        "ERR:dictBuilder/fastcover.c:245",
        "ERR:dictBuilder/fastcover.c:249",
        "ERR:dictBuilder/fastcover.c:257",
        "ERR:dictBuilder/fastcover.c:575",
        "ERR:dictBuilder/fastcover.c:580",
        "ERR:dictBuilder/fastcover.c:591",
        "ERR:dictBuilder/fastcover.c:332",
        "ERR:dictBuilder/fastcover.c:338",
        "ERR:dictBuilder/fastcover.c:612",
    ]);
    let s = txt64();
    // (363) d must be exactly 6 or 8.
    for d in [0u32, 1, 2, 5, 7, 9, 16, 32] {
        expect_err(
            &format!("fastCover(d {d})"),
            &diff_bytes(&format!("fastCover(d {d})"), |l| {
                train_fast(l, 4096, &s, fast_params(128, d, 0, 0))
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    // k == 0, d > k, k > capacity.
    for (k, d) in [(0u32, 8u32), (4, 6), (4097, 8)] {
        expect_err(
            &format!("fastCover(k {k}, d {d})"),
            &diff_bytes(&format!("fastCover(k {k}, d {d})"), |l| {
                train_fast(l, 4096, &s, fast_params(k, d, 20, 1))
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    // f > FASTCOVER_MAX_F(31) and accel > 10 — both rejected *before* any
    // allocation, so no 2^f array is ever requested here.
    for f in [32u32, 33, 0xFFFF_FFFF] {
        expect_err(
            &format!("fastCover(f {f})"),
            &diff_bytes(&format!("fastCover(f {f})"), |l| {
                train_fast(l, 4096, &s, fast_params(128, 8, f, 1))
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    for accel in [11u32, 12, 255, 0xFFFF_FFFF] {
        expect_err(
            &format!("fastCover(accel {accel})"),
            &diff_bytes(&format!("fastCover(accel {accel})"), |l| {
                train_fast(l, 4096, &s, fast_params(128, 8, 20, accel))
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    // (369) nbSamples == 0, capacity too small, too few / too small samples.
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    expect_err(
        "fastCover(nbSamples 0)",
        &diff_bytes("fastCover(nbSamples 0)", |l| {
            train_fast(l, 4096, &empty, fast_params(128, 8, 20, 1))
        })
        .0,
        E_srcSize_wrong,
    );
    // `FASTCOVER_checkParameters` runs *before* the capacity check, and it also
    // tests `k > maxDictSize` where maxDictSize is the capacity — so a capacity
    // below k reports parameter_outOfBound, and only a capacity that is >= k
    // but < ZDICT_DICTSIZE_MIN reaches fastcover.c:580.
    for (cap, want) in [
        (0usize, E_parameter_outOfBound),
        (1, E_parameter_outOfBound),
        (127, E_parameter_outOfBound),
        (128, E_dstSize_tooSmall),
        (255, E_dstSize_tooSmall),
    ] {
        expect_err(
            &format!("fastCover(cap {cap})"),
            &diff_bytes(&format!("fastCover(cap {cap})"), |l| {
                train_fast(l, cap, &s, fast_params(128, 8, 20, 1))
            })
            .0,
            want,
        );
    }
    let four = s_uniform(Corpus::Text, 4, 256, 0x7024_0071);
    expect_err(
        "fastCover(4 samples)",
        &diff_bytes("fastCover(4 samples)", |l| {
            train_fast(l, 4096, &four, fast_params(128, 8, 20, 1))
        })
        .0,
        E_srcSize_wrong,
    );
    let tiny = s_sizes(Corpus::Text, &[1, 2, 1, 2, 1], 0x7024_0072);
    expect_err(
        "fastCover(5 samples totalling 7)",
        &diff_bytes("fastCover(5 samples totalling 7)", |l| {
            train_fast(l, 4096, &tiny, fast_params(128, 8, 20, 1))
        })
        .0,
        E_srcSize_wrong,
    );
}

/// Valid-path sweep of `ZDICT_trainFromBuffer_fastCover` over both specialised
/// `d` values, the `f` and `accel` ranges, and the axes the non-optimize entry
/// point must ignore (`splitPoint`, `steps`, `nbThreads`, `shrinkDict`).
#[test]
fn t_zdict_train_fastcover_valid() {
    covers(&["CFG:364", "CFG:365", "CFG:366", "CFG:367", "CFG:368", "CFG:370"]);
    // `FASTCOVER_ctx_init`'s two callocs are evaluated on every run; the f = 31
    // case below actually requests 8 GB from the second one.
    covers(&[
        "ERR:dictBuilder/fastcover.c:369",
        "ERR:dictBuilder/fastcover.c:386",
    ]);
    let s = txt64();

    // (364) d = 6 and d = 8.
    for d in [6u32, 8] {
        diff_bytes(&format!("fastCover(d {d}, f 20, accel 1)"), |l| {
            train_fast(l, 4096, &s, fast_params(128, d, 20, 1))
        });
    }
    // (365) f sweep. f = 0 means "default 20". 26..=31 are omitted from the
    // sweep because `FASTCOVER_ctx_init` callocs `4 << f` bytes and
    // `ZDICT_trainFromBuffer_fastCover` a further `2 << f`, i.e. 12 GB at
    // f = 31 — see the note in the test body.
    for f in [0u32, 1, 2, 6, 8, 10, 16, 20, 24, 25, 31] {
        diff_bytes(&format!("fastCover(f {f})"), |l| {
            train_fast(l, 4096, &s, fast_params(128, 8, f, 1))
        });
    }
    // (366) accel sweep. accel = 0 means "default 1".
    let s200 = s_uniform(Corpus::Text, 200, 256, 0x7024_0081);
    for accel in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
        diff_bytes(&format!("fastCover(accel {accel})"), |l| {
            train_fast(l, 4096, &s200, fast_params(128, 8, 20, accel))
        });
    }
    // (367) nbFinalizeSamples == 0 (9 samples * finalize 10% / 100 = 0), so
    // ZDICT_finalizeDictionary runs with nbSamples == 0.
    let s9 = s_uniform(Corpus::Text, 9, 256, 0x7024_0082);
    diff_bytes("fastCover(9 samples, accel 10 -> nbFinalizeSamples 0)", |l| {
        train_fast(l, 4096, &s9, fast_params(128, 8, 20, 10))
    });

    // (368) splitPoint / steps / nbThreads / shrinkDict are all forced or
    // unused here: every combination must give the identical dictionary.
    let base = diff_bytes("fastCover(baseline)", |l| {
        train_fast(l, 4096, &s, fast_params(128, 8, 20, 1))
    });
    for sp in [-1.0f64, 0.0, 0.5, 0.75, 1.0, 2.0] {
        for nbt in [0u32, 1, 4] {
            for steps in [0u32, 1, 40] {
                for shrink in [0u32, 1] {
                    let mut p = fast_params(128, 8, 20, 1);
                    p.splitPoint = sp;
                    p.nbThreads = nbt;
                    p.steps = steps;
                    p.shrinkDict = shrink;
                    p.shrinkDictMaxRegression = 5 * shrink;
                    let got = diff_bytes(
                        &format!("fastCover(sp {sp}, nbt {nbt}, steps {steps}, shrink {shrink})"),
                        |l| train_fast(l, 4096, &s, p),
                    );
                    assert_eq!(got.1, base.1);
                    assert_eq!(got.0, base.0);
                }
            }
        }
    }

    // (370) corpora and k sweep.
    for (tag, ss) in [
        ("ZERO", s_uniform(Corpus::Zeros, 64, 256, 0x7024_0083)),
        ("SAME", s_same(Corpus::Text, 64, 256, 0x7024_0084)),
        ("RAND", s_uniform(Corpus::Random, 64, 512, 0x7024_0085)),
        ("SPARSE", s_uniform(Corpus::Sparse, 64, 256, 0x7024_0086)),
    ] {
        let mut p = fast_params(128, 8, 20, 1);
        p.zParams.compressionLevel = 3;
        diff_bytes(&format!("fastCover({tag})"), |l| train_fast(l, 4096, &ss, p));
    }
    for k in [8u32, 16, 32, 64, 128, 200, 512, 2000, 4096] {
        let mut p = fast_params(k, 8, 20, 1);
        p.zParams.compressionLevel = 3;
        diff_bytes(&format!("fastCover(k {k})"), |l| train_fast(l, 4096, &s200, p));
    }
    // zParams and capacity sweeps.
    for lvl in [0i32, 1, 3, 19] {
        for id in [0u32, 1, 12345] {
            let mut p = fast_params(200, 8, 20, 1);
            p.zParams = ZDICT_params_t {
                compressionLevel: lvl,
                notificationLevel: 0,
                dictID: id,
            };
            diff_bytes(&format!("fastCover(clevel {lvl}, dictID {id})"), |l| {
                train_fast(l, 4096, &s, p)
            });
        }
    }
    for cap in [DICTSIZE_MIN, 257, 1024, 16384, 110_000] {
        diff_bytes(&format!("fastCover(cap {cap})"), |l| {
            train_fast(l, cap, &s, fast_params(200, 8, 20, 1))
        });
    }
    let varied = s_sizes(
        Corpus::Mixed,
        &[0, 1, 7, 8, 64, 4096, 0, 300, 1, 2048, 0, 512, 0, 9000],
        0x7024_0087,
    );
    diff_bytes("fastCover(varied samples)", |l| {
        train_fast(l, 4096, &varied, fast_params(200, 8, 20, 1))
    });
    for nb in [5usize, 20, 100, 500] {
        let ss = s_uniform(Corpus::Text, nb, 64, 0x7024_0088);
        diff_bytes(&format!("fastCover(nbSamples {nb})"), |l| {
            train_fast(l, 4096, &ss, fast_params(200, 8, 20, 1))
        });
    }
}

// ===========================================================================
// 9. ZDICT_optimizeTrainFromBuffer_fastCover
// ===========================================================================

/// `fastcover.c:652` (splitPoint > 1), `:656` (accel > 10), `:660`
/// (`kMinK < kMaxD || kMaxK < kMinK`), `:664` (nbSamples == 0), `:669`
/// (capacity < 256), `:697` (`FASTCOVER_ctx_init` forwarding), `:728-732`
/// (every trial rejected) and `:757` (that surfacing as `ERROR(GENERIC)`),
/// plus `FASTCOVER_tryParameters` (`:487-489`, `:502-504`).
#[test]
fn t_zdict_optimize_fastcover() {
    covers(&[
        "CFG:371-376",
        "ERR:dictBuilder/fastcover.c:652",
        "ERR:dictBuilder/fastcover.c:656",
        "ERR:dictBuilder/fastcover.c:660",
        "ERR:dictBuilder/fastcover.c:664",
        "ERR:dictBuilder/fastcover.c:669",
        "ERR:dictBuilder/fastcover.c:697",
        "ERR:dictBuilder/fastcover.c:728-732",
        "ERR:dictBuilder/fastcover.c:757",
        "ERR:dictBuilder/fastcover.c:487-489",
        "ERR:dictBuilder/fastcover.c:502-504",
    ]);
    covers(&["ERR:dictBuilder/fastcover.c:715"]);
    let s = txt64();

    // (371) all-zero parameters: d {6,8}, k [50,2000], steps 40, f 20, accel 1,
    // splitPoint 0.75. Also compares every field written back into *parameters.
    diff_bytes("optFast(all-zero params)", |l| {
        opt_fast(l, 4096, &s, ZDICT_fastCover_params_t::default())
    });

    // (372) d must be 6 or 8 for *every* trial, so any other d makes them all
    // fail and the `COVER_best_init` seed surfaces as ERROR(GENERIC) = code 1.
    for d in [1u32, 7, 9, 16] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.d = d;
        expect_err(
            &format!("optFast(d {d}) -> GENERIC"),
            &diff_bytes(&format!("optFast(d {d})"), |l| opt_fast(l, 4096, &s, p)).0,
            E_GENERIC,
        );
    }

    // (373) accel and f. accel > 10 is rejected up front; f is *not* validated
    // in the optimize prologue, so a too-large f reaches every trial's
    // FASTCOVER_checkParameters and yields GENERIC. f = 32 is excluded — see
    // the comment below.
    for accel in [0u32, 1, 10] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.accel = accel;
        p.steps = 2;
        diff_bytes(&format!("optFast(accel {accel})"), |l| {
            opt_fast(l, 4096, &s, p)
        });
    }
    for accel in [11u32, 255] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.accel = accel;
        expect_err(
            &format!("optFast(accel {accel})"),
            &diff_bytes(&format!("optFast(accel {accel})"), |l| {
                opt_fast(l, 4096, &s, p)
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    // f = 0 -> default 20. f > 31 (32, 33, ...) is NOT excluded by the
    // prologue: `FASTCOVER_ctx_init` is called with it first and callocs
    // `4 << f` bytes (16 GB at f = 32), whose success depends on the machine's
    // overcommit policy rather than on the library — excluded as
    // non-deterministic, not as a divergence.
    for f in [0u32, 1, 6, 20, 25] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.f = f;
        p.steps = 2;
        diff_bytes(&format!("optFast(f {f})"), |l| opt_fast(l, 4096, &s, p));
    }

    // (374) steps.
    for steps in [1u32, 2, 4, 40, 200] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.steps = steps;
        diff_bytes(&format!("optFast(steps {steps})"), |l| {
            opt_fast(l, 4096, &s, p)
        });
    }
    {
        let mut p = ZDICT_fastCover_params_t::default();
        p.k = 128;
        p.steps = 0;
        diff_bytes("optFast(k 128, steps 0)", |l| opt_fast(l, 4096, &s, p));
    }

    // (375) nbThreads x shrinkDict x shrinkDictMaxRegression: all identical
    // (shrinkDict is force-zeroed, and the POOL stub runs jobs inline).
    let mut pbase = ZDICT_fastCover_params_t::default();
    pbase.k = 128;
    pbase.d = 8;
    pbase.steps = 1;
    let base = diff_bytes("optFast(k128 d8 baseline)", |l| opt_fast(l, 4096, &s, pbase));
    for nbt in [0u32, 1, 2, 8] {
        for shrink in [0u32, 1] {
            for reg in [0u32, 10, 100] {
                let mut p = pbase;
                p.nbThreads = nbt;
                p.shrinkDict = shrink;
                p.shrinkDictMaxRegression = reg;
                let got = diff_bytes(
                    &format!("optFast(nbt {nbt}, shrink {shrink}/{reg})"),
                    |l| opt_fast(l, 4096, &s, p),
                );
                assert_eq!(got.2, base.2);
                assert_eq!(got.0, base.0);
                // Every written-back field must match the baseline except the
                // two the C copies straight through: `nbThreads` is taken from
                // `best.parameters` (which inherited it from the caller) and
                // `shrinkDictMaxRegression` is never touched by
                // `FASTCOVER_convertToFastCoverParams` at all.
                let mut norm = got.1;
                norm.shrinkDictMaxRegression = base.1.shrinkDictMaxRegression;
                norm.nbThreads = base.1.nbThreads;
                assert_eq!(norm, base.1);
            }
        }
    }

    // (376) splitPoint: <= 0 is replaced by the 0.75 default, > 1 rejected.
    for sp in [-1.0f64, 0.0, 0.5, 0.75, 1.0] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.splitPoint = sp;
        p.steps = 2;
        diff_bytes(&format!("optFast(splitPoint {sp})"), |l| {
            opt_fast(l, 4096, &s, p)
        });
    }
    for sp in [1.0001f64, 1.5] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.splitPoint = sp;
        expect_err(
            &format!("optFast(splitPoint {sp})"),
            &diff_bytes(&format!("optFast(splitPoint {sp})"), |l| {
                opt_fast(l, 4096, &s, p)
            })
            .0,
            E_parameter_outOfBound,
        );
    }

    // Remaining error returns: k/d relation, nbSamples, capacity, ctx_init.
    for (k, d) in [(0u32, 100u32), (4, 8), (2000, 3000)] {
        let mut p = ZDICT_fastCover_params_t::default();
        p.k = k;
        p.d = d;
        expect_err(
            &format!("optFast(k {k}, d {d})"),
            &diff_bytes(&format!("optFast(k {k}, d {d})"), |l| {
                opt_fast(l, 4096, &s, p)
            })
            .0,
            E_parameter_outOfBound,
        );
    }
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    let mut p = ZDICT_fastCover_params_t::default();
    p.k = 128;
    p.d = 8;
    expect_err(
        "optFast(nbSamples 0)",
        &diff_bytes("optFast(nbSamples 0)", |l| opt_fast(l, 4096, &empty, p)).0,
        E_srcSize_wrong,
    );
    for cap in [0usize, 1, 255] {
        expect_err(
            &format!("optFast(cap {cap})"),
            &diff_bytes(&format!("optFast(cap {cap})"), |l| opt_fast(l, cap, &s, p)).0,
            E_dstSize_tooSmall,
        );
    }
    let four = s_uniform(Corpus::Text, 4, 256, 0x7024_0091);
    expect_err(
        "optFast(4 samples)",
        &diff_bytes("optFast(4 samples)", |l| opt_fast(l, 4096, &four, p)).0,
        E_srcSize_wrong,
    );
    // k = 2000 with capacity 256 -> every trial rejected -> GENERIC.
    {
        let mut q = ZDICT_fastCover_params_t::default();
        q.k = 2000;
        q.d = 8;
        expect_err(
            "optFast(cap 256, k 2000) -> GENERIC",
            &diff_bytes("optFast(cap 256, k 2000)", |l| opt_fast(l, 256, &s, q)).0,
            E_GENERIC,
        );
    }
    // Capacity / corpus sweep on the optimize path.
    for cap in [DICTSIZE_MIN, 257, 1024, 16384] {
        let mut q = ZDICT_fastCover_params_t::default();
        q.k = 200;
        q.d = 8;
        q.steps = 1;
        diff_bytes(&format!("optFast(cap {cap})"), |l| opt_fast(l, cap, &s, q));
    }
}

// ===========================================================================
// 10. ZDICT_trainFromBuffer
// ===========================================================================

/// `zdict.c:1120` — the stable entry point, which is a thin redirect to
/// `ZDICT_optimizeTrainFromBuffer_fastCover` with `d=8, steps=4, f=20,
/// accel=1, splitPoint=0.75` and forwards every failure it produces.
#[test]
fn t_zdict_train_from_buffer() {
    covers(&["CFG:377", "CFG:378", "ERR:dictBuilder/zdict.c:1120"]);
    // (377) the documented corpora, at two capacities.
    let sets: [(&str, Samples); 9] = [
        ("TEXT(64,256)", txt64()),
        ("TEXT(500,64)", s_uniform(Corpus::Text, 500, 64, 0x7024_00A1)),
        ("TINY(100,8)", s_uniform(Corpus::Text, 100, 8, 0x7024_00A7)),
        ("KB(20,1024)", s_uniform(Corpus::Text, 20, 1024, 0x7024_00A8)),
        (
            "SOMEZERO",
            s_sizes(
                Corpus::Text,
                &[0, 1024, 0, 1024, 0, 1024, 0, 1024, 0, 1024, 0, 1024],
                0x7024_00A9,
            ),
        ),
        ("RAND(64,512)", s_uniform(Corpus::Random, 64, 512, 0x7024_00A2)),
        ("ZERO(64,256)", s_uniform(Corpus::Zeros, 64, 256, 0x7024_00A3)),
        ("SAME(64,256)", s_same(Corpus::Text, 64, 256, 0x7024_00A4)),
        (
            "VARIED",
            s_sizes(
                Corpus::Mixed,
                &[0, 1, 7, 8, 64, 4096, 0, 300, 1, 2048, 0, 512, 0, 9000],
                0x7024_00A5,
            ),
        ),
    ];
    for (tag, ss) in &sets {
        for cap in [DICTSIZE_MIN, 257, 4096, 16384, 110_000] {
            diff_bytes(&format!("train({tag}, cap {cap})"), |l| train(l, cap, ss));
        }
    }
    // nbSamples axis, including the 1/2 cases that must fail.
    for nb in [1usize, 2, 4, 5, 6, 7, 20, 100] {
        let ss = s_uniform(Corpus::Text, nb, 256, 0x7024_00A6);
        diff_bytes(&format!("train(nbSamples {nb})"), |l| train(l, 4096, &ss));
    }
    // (378) nbSamples == 0 and capacities below ZDICT_DICTSIZE_MIN.
    let empty = Samples::new(vec![], vec![]).with_nb(0);
    expect_err(
        "train(nbSamples 0)",
        &diff_bytes("train(nbSamples 0)", |l| train(l, 4096, &empty)).0,
        E_srcSize_wrong,
    );
    let s = txt64();
    for cap in [0usize, 1, 255] {
        expect_err(
            &format!("train(cap {cap})"),
            &diff_bytes(&format!("train(cap {cap})"), |l| train(l, cap, &s)).0,
            E_dstSize_tooSmall,
        );
    }
}

// ===========================================================================
// Part B — USING dictionaries
// ===========================================================================

type FnCreateCDict = unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    SizeT,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void,
    SizeT,
    c_int,
    c_int,
    *const c_void,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, SizeT, c_int, c_int, ZSTD_customMem) -> *mut c_void;
type FnSizeofPtr = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnIdFromPtr = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnIdFromBuf = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;
type FnCompressUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
) -> SizeT;
type FnCompressUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    ZSTD_frameParameters,
) -> SizeT;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
) -> SizeT;
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;
type FnRefPtr = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int, c_int) -> SizeT;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnGetCParams =
    unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;
type FnCParamsFromCDict = unsafe extern "C" fn(*const c_void) -> ZSTD_compressionParameters;
type FnEstCDict = unsafe extern "C" fn(SizeT, c_int) -> SizeT;
type FnEstCDictAdv = unsafe extern "C" fn(SizeT, ZSTD_compressionParameters, c_int) -> SizeT;
type FnEstDDict = unsafe extern "C" fn(SizeT, c_int) -> SizeT;
type FnDDictContent = unsafe extern "C" fn(*const c_void) -> *const c_void;
type FnDDictSize = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnInitCStreamUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnInitCStreamUsingCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, ZSTD_frameParameters, c_ulonglong) -> SizeT;
type FnInitDStreamUsingDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnCreateCCtxParams = unsafe extern "C" fn() -> *mut c_void;
type FnEndStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> SizeT;

// ZSTD_dictAttachPref_e
const ZSTD_dictDefaultAttach: c_int = 0;
const ZSTD_dictForceAttach: c_int = 1;
const ZSTD_dictForceCopy: c_int = 2;
const ZSTD_dictForceLoad: c_int = 3;
// ZSTD_refMultipleDDicts_e
const ZSTD_rmd_refSingleDDict: c_int = 0;
const ZSTD_rmd_refMultipleDDicts: c_int = 1;

const E_stage_wrong: c_int = 60;
const E_dictionary_wrong: c_int = 32;
const E_memory_allocation: c_int = 64;

// ---------------------------------------------------------------------------
// Dictionary fixtures
// ---------------------------------------------------------------------------

/// A real trained dictionary of at most `cap` bytes, built through `diff_bytes`
/// so the fixture is proven identical between C and Rust before it is consumed.
fn trained_dict(cap: usize) -> Vec<u8> {
    let s = txt64();
    let (r, b) = diff_bytes(&format!("fixture: trained dict cap {cap}"), |l| {
        train_fast(l, cap, &s, fast_params(200, 8, 20, 1))
    });
    let n = expect_ok("fixture trained", &r);
    b.0[..n].to_vec()
}

/// Raw dictionary content — deliberately *not* prefixed with
/// `ZSTD_MAGIC_DICTIONARY`, so `ZSTD_dct_auto` treats it as raw content.
fn raw_dict(n: usize) -> Vec<u8> {
    corpus(Corpus::Text, n, 0x7024_0100)
}

/// `ZSTD_MAGIC_DICTIONARY` + dictID + garbage where the entropy tables belong:
/// loadable as raw content, rejected as a full dictionary.
fn magic_dict(n: usize, id: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(n.max(8));
    v.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
    v.extend_from_slice(&id.to_le_bytes());
    v.extend_from_slice(&corpus(Corpus::Random, n.saturating_sub(8) + 8, 0x7024_0101));
    v.truncate(n);
    v
}

/// A well-formed dictionary carrying an explicit dictID.
fn dict_with_id(id: u32) -> Vec<u8> {
    let s = txt64();
    let content = corpus(Corpus::Text, 2048, 0x7024_0102 ^ id as u64);
    let (r, b) = diff_bytes(&format!("fixture: dict with dictID {id}"), |l| {
        finalize(
            l,
            4096,
            &content,
            &s,
            ZDICT_params_t {
                compressionLevel: 0,
                notificationLevel: 0,
                dictID: id,
            },
        )
    });
    let n = expect_ok("fixture dict_with_id", &r);
    b.0[..n].to_vec()
}

fn payload(n: usize) -> Vec<u8> {
    corpus(Corpus::Text, n, 0x7024_0110)
}

/// Everything observable about a CDict/DDict pair plus a round trip through it.
#[derive(Debug, PartialEq, Eq)]
struct DictInfo {
    cdict_null: bool,
    cdict_sizeof: SizeT,
    cdict_id: c_uint,
    cparams: Option<ZSTD_compressionParameters>,
    ddict_null: bool,
    ddict_sizeof: SizeT,
    ddict_id: c_uint,
    ddict_content_len: Option<SizeT>,
    id_from_dict: c_uint,
    est_cdict: SizeT,
    est_cdict_adv: SizeT,
    est_ddict: SizeT,
    comp: Option<R>,
    id_from_frame: Option<c_uint>,
    decomp: Option<R>,
    roundtrip_ok: Option<bool>,
}

/// Which constructor pair to drive.
#[derive(Copy, Clone, Debug)]
enum Ctor {
    /// `ZSTD_createCDict` / `ZSTD_createDDict`
    Simple,
    /// `ZSTD_createCDict_byReference` / `ZSTD_createDDict_byReference`
    ByRef,
    /// `ZSTD_createCDict_advanced` / `ZSTD_createDDict_advanced`
    Advanced,
    /// `ZSTD_createCDict_advanced2` (cctxParams) / `ZSTD_createDDict_advanced`
    Advanced2,
}

#[allow(clippy::too_many_arguments)]
fn dict_probe(
    l: &Lib,
    dict: &[u8],
    null_ptr: bool,
    lvl: c_int,
    dlm: c_int,
    dct: c_int,
    ctor: Ctor,
    dds: c_int,
    src: &[u8],
) -> DictInfo {
    let dp: *const c_void = if null_ptr {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_void
    };
    let dlen = dict.len();
    let getcp = l.sym::<FnGetCParams>("ZSTD_getCParams");
    let cp = unsafe { getcp(lvl, src.len() as c_ulonglong, dlen) };

    // ---- CDict ----
    let mut params_holder: Option<Ctx> = None;
    let cdict = unsafe {
        match ctor {
            Ctor::Simple => {
                let f = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                f(dp, dlen, lvl)
            }
            Ctor::ByRef => {
                let f = l.sym::<FnCreateCDict>("ZSTD_createCDict_byReference");
                f(dp, dlen, lvl)
            }
            Ctor::Advanced => {
                let f = l.sym::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
                f(dp, dlen, dlm, dct, cp, ZSTD_customMem::default())
            }
            Ctor::Advanced2 => {
                let cr = l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams");
                let pp = cr();
                assert!(!pp.is_null());
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
                assert!(!is_error(l, set(pp, ZSTD_c_compressionLevel, lvl)));
                assert!(!is_error(
                    l,
                    set(pp, ZSTD_c_enableDedicatedDictSearch, dds)
                ));
                let f = l.sym::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2");
                let cd = f(dp, dlen, dlm, dct, pp as *const c_void, ZSTD_customMem::default());
                params_holder = Some(Ctx::from_raw(l, pp, "ZSTD_freeCCtxParams"));
                cd
            }
        }
    };
    let cdict = if cdict.is_null() {
        None
    } else {
        Some(Ctx::from_raw(l, cdict, "ZSTD_freeCDict"))
    };

    // ---- DDict ----
    let ddict = unsafe {
        match ctor {
            Ctor::Simple => {
                let f = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                f(dp, dlen)
            }
            Ctor::ByRef => {
                let f = l.sym::<FnCreateDDict>("ZSTD_createDDict_byReference");
                f(dp, dlen)
            }
            Ctor::Advanced | Ctor::Advanced2 => {
                let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                f(dp, dlen, dlm, dct, ZSTD_customMem::default())
            }
        }
    };
    let ddict = if ddict.is_null() {
        None
    } else {
        Some(Ctx::from_raw(l, ddict, "ZSTD_freeDDict"))
    };

    let sizeof_cdict = l.sym::<FnSizeofPtr>("ZSTD_sizeof_CDict");
    let sizeof_ddict = l.sym::<FnSizeofPtr>("ZSTD_sizeof_DDict");
    let id_cdict = l.sym::<FnIdFromPtr>("ZSTD_getDictID_fromCDict");
    let id_ddict = l.sym::<FnIdFromPtr>("ZSTD_getDictID_fromDDict");
    let id_dict = l.sym::<FnIdFromBuf>("ZSTD_getDictID_fromDict");
    let est_c = l.sym::<FnEstCDict>("ZSTD_estimateCDictSize");
    let est_c_adv = l.sym::<FnEstCDictAdv>("ZSTD_estimateCDictSize_advanced");
    let est_d = l.sym::<FnEstDDict>("ZSTD_estimateDDictSize");

    let cptr = cdict.as_ref().map(|c| c.ptr).unwrap_or(std::ptr::null_mut());
    let dptr = ddict.as_ref().map(|c| c.ptr).unwrap_or(std::ptr::null_mut());

    // `ZSTD_getCParamsFromCDict` and `ZSTD_DDict_dictContent` dereference their
    // argument without a NULL check (they only `assert`, and DEBUGLEVEL is 0),
    // so they are only called on a non-NULL object.
    let cparams = cdict.as_ref().map(|c| {
        let g = l.sym::<FnCParamsFromCDict>("ZSTD_getCParamsFromCDict");
        unsafe { g(c.ptr) }
    });
    let ddict_content_len = ddict.as_ref().map(|d| {
        let gc = l.sym::<FnDDictContent>("ZSTD_DDict_dictContent");
        let gs = l.sym::<FnDDictSize>("ZSTD_DDict_dictSize");
        let (c, s) = unsafe { (gc(d.ptr), gs(d.ptr)) };
        // Only the length is comparable; the pointer differs per library.
        assert!(s == 0 || !c.is_null());
        s
    });

    // ---- round trip ----
    let mut comp = None;
    let mut id_from_frame = None;
    let mut decomp = None;
    let mut roundtrip_ok = None;
    if let Some(cd) = &cdict {
        let cctx = Ctx::cctx(l);
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let f = l.sym::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
        let n = unsafe {
            f(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                cd.ptr,
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            dst.truncate(k);
            let gf = l.sym::<FnIdFromBuf>("ZSTD_getDictID_fromFrame");
            id_from_frame = Some(unsafe { gf(dst.as_ptr() as *const c_void, dst.len()) });
            if let Some(dd) = &ddict {
                let dctx = Ctx::dctx(l);
                let mut out = vec![0xABu8; src.len() + 64];
                let g = l.sym::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
                let m = unsafe {
                    g(
                        dctx.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        dst.as_ptr() as *const c_void,
                        dst.len(),
                        dd.ptr,
                    )
                };
                let rr = res(l, m);
                roundtrip_ok = Some(match &rr {
                    R::Ok(m) => *m == src.len() && out[..*m] == *src,
                    R::Err(..) => false,
                });
                decomp = Some(rr);
            }
        }
        comp = Some(r);
    }
    drop(params_holder.take());

    DictInfo {
        cdict_null: cptr.is_null(),
        cdict_sizeof: unsafe { sizeof_cdict(cptr) },
        cdict_id: unsafe { id_cdict(cptr) },
        cparams,
        ddict_null: dptr.is_null(),
        ddict_sizeof: unsafe { sizeof_ddict(dptr) },
        ddict_id: unsafe { id_ddict(dptr) },
        ddict_content_len,
        id_from_dict: unsafe { id_dict(dp, dlen) },
        est_cdict: unsafe { est_c(dlen, lvl) },
        est_cdict_adv: unsafe { est_c_adv(dlen, cp, dlm) },
        est_ddict: unsafe { est_d(dlen, dlm) },
        comp,
        id_from_frame,
        decomp,
        roundtrip_ok,
    }
}

/// `ZSTD_createCDict` / `_byReference` / `_advanced` / `_advanced2`,
/// `ZSTD_createDDict` / `_byReference` / `_advanced`, the `sizeof`/`dictID`
/// accessors, the three `estimate*Size` helpers and `ZSTD_getCParamsFromCDict`,
/// over dictionary kind x `ZSTD_dictLoadMethod_e` x `ZSTD_dictContentType_e` x
/// compression level x dictionary size.
#[test]
fn t_cdict_ddict_lifecycle() {
    covers(&[
        "CFG:7", "CFG:8", "CFG:77", "CFG:78", "CFG:132", "CFG:163", "CFG:166", "CFG:177",
        "CFG:179", "CFG:180",
    ]);
    let src = payload(4096);
    let tr = trained_dict(4096);
    let kinds: [(&str, Vec<u8>); 3] = [
        ("trained", tr.clone()),
        ("raw", raw_dict(4096)),
        ("magicPrefixed", magic_dict(4096, 0xABCD)),
    ];

    // free(NULL) must be accepted by both free functions, and sizeof/dictID on
    // NULL must return 0.
    diff("free/sizeof/id on NULL", |l| {
        let fc = l.sym::<FnFreeCCtx>("ZSTD_freeCDict");
        let fd = l.sym::<FnFreeCCtx>("ZSTD_freeDDict");
        let sc = l.sym::<FnSizeofPtr>("ZSTD_sizeof_CDict");
        let sd = l.sym::<FnSizeofPtr>("ZSTD_sizeof_DDict");
        let ic = l.sym::<FnIdFromPtr>("ZSTD_getDictID_fromCDict");
        let id = l.sym::<FnIdFromPtr>("ZSTD_getDictID_fromDDict");
        unsafe {
            (
                fc(std::ptr::null_mut()),
                fd(std::ptr::null_mut()),
                sc(std::ptr::null()),
                sd(std::ptr::null()),
                ic(std::ptr::null()),
                id(std::ptr::null()),
            )
        }
    });

    // dict = NULL with size 0, and a non-NULL 0-size buffer.
    for lvl in [1i32, 3, 22] {
        diff(&format!("cdict(NULL,0,lvl {lvl})"), |l| {
            dict_probe(l, &[], true, lvl, ZSTD_dlm_byCopy, ZSTD_dct_auto, Ctor::Simple, 0, &src)
        });
        diff(&format!("cdict(nonNULL,0,lvl {lvl})"), |l| {
            dict_probe(l, &tr[..0], false, lvl, ZSTD_dlm_byCopy, ZSTD_dct_auto, Ctor::Simple, 0, &src)
        });
    }

    // Sizes 1..8 straddle the 8-byte magic+dictID header.
    for n in 1..=8usize {
        for ctor in [Ctor::Simple, Ctor::ByRef, Ctor::Advanced] {
            diff(&format!("cdict(trained[..{n}], {ctor:?})"), |l| {
                dict_probe(l, &tr[..n], false, 3, ZSTD_dlm_byCopy, ZSTD_dct_auto, ctor, 0, &src)
            });
        }
    }

    // The full cross-product.
    for (tag, d) in &kinds {
        for &sz in &[0usize, 1, 8, 256, 4096] {
            let dd = &d[..sz.min(d.len())];
            for &lvl in &[-5i32, 0, 1, 3, 9, 19, 22] {
                for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                        diff(
                            &format!("cdict({tag},{sz},lvl {lvl},dlm {dlm},dct {dct})"),
                            |l| {
                                dict_probe(
                                    l, dd, false, lvl, dlm, dct, Ctor::Advanced, 0, &src,
                                )
                            },
                        );
                    }
                }
            }
        }
    }

    // A 110 KB dictionary, and the simple / byReference / advanced2 ctors.
    let big = trained_dict(110_000);
    for ctor in [Ctor::Simple, Ctor::ByRef, Ctor::Advanced, Ctor::Advanced2] {
        for &lvl in &[1i32, 5, 19] {
            for &dds in &[0i32, 1] {
                if dds == 1 && !matches!(ctor, Ctor::Advanced2) {
                    continue;
                }
                diff(&format!("cdict(110KB,{ctor:?},lvl {lvl},dds {dds})"), |l| {
                    dict_probe(
                        l,
                        &big,
                        false,
                        lvl,
                        ZSTD_dlm_byCopy,
                        ZSTD_dct_auto,
                        ctor,
                        dds,
                        &src,
                    )
                });
            }
        }
    }

    // Out-of-range enum values crossing the FFI boundary. `dictLoadMethod` is
    // tested with `== ZSTD_dlm_byRef` so anything else copies; `dictContentType`
    // is tested with `== ZSTD_dct_rawContent` / `== ZSTD_dct_auto`, so an
    // unknown value behaves like fullDict. Both are deterministic, not UB.
    for &dct in &[3i32, -1, 999] {
        for (tag, d) in &kinds {
            diff(&format!("cdict({tag}, dct {dct})"), |l| {
                dict_probe(l, d, false, 3, ZSTD_dlm_byCopy, dct, Ctor::Advanced, 0, &src)
            });
        }
    }
    for &dlm in &[2i32, -1] {
        diff(&format!("cdict(trained, dlm {dlm})"), |l| {
            dict_probe(l, &tr, false, 3, dlm, ZSTD_dct_auto, Ctor::Advanced, 0, &src)
        });
    }

    // ZSTD_estimateCDictSize / _advanced / ZSTD_estimateDDictSize on their own.
    for &sz in &[0usize, 1, 7, 8, 4096, 1 << 20] {
        for &lvl in &[1i32, 3, 19] {
            diff(&format!("estimateCDictSize({sz},{lvl})"), |l| {
                let e = l.sym::<FnEstCDict>("ZSTD_estimateCDictSize");
                let g = l.sym::<FnGetCParams>("ZSTD_getCParams");
                let ea = l.sym::<FnEstCDictAdv>("ZSTD_estimateCDictSize_advanced");
                unsafe {
                    let cp = g(lvl, 0, sz);
                    (
                        e(sz, lvl),
                        ea(sz, cp, ZSTD_dlm_byCopy),
                        ea(sz, cp, ZSTD_dlm_byRef),
                    )
                }
            });
        }
        diff(&format!("estimateDDictSize({sz})"), |l| {
            let e = l.sym::<FnEstDDict>("ZSTD_estimateDDictSize");
            unsafe { (e(sz, ZSTD_dlm_byCopy), e(sz, ZSTD_dlm_byRef)) }
        });
    }
}

// ---------------------------------------------------------------------------
// One-shot round trips
// ---------------------------------------------------------------------------

fn usingdict_roundtrip(
    l: &Lib,
    dict: &[u8],
    lvl: c_int,
    src: &[u8],
) -> (R, Blob, R, Blob) {
    let cctx = Ctx::cctx(l);
    let cap = compress_bound(l, src.len()) + 64;
    let mut dst = vec![0xCDu8; cap];
    let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
    let n = unsafe {
        f(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            dict.as_ptr() as *const c_void,
            dict.len(),
            lvl,
        )
    };
    let cr = res(l, n);
    if let R::Ok(k) = cr {
        dst.truncate(k);
    }
    let dctx = Ctx::dctx(l);
    let mut out = vec![0xABu8; src.len() + 64];
    let g = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
    let m = unsafe {
        g(
            dctx.ptr,
            out.as_mut_ptr() as *mut c_void,
            out.len(),
            dst.as_ptr() as *const c_void,
            dst.len(),
            dict.as_ptr() as *const c_void,
            dict.len(),
        )
    };
    let dr = res(l, m);
    if let R::Ok(k) = dr {
        out.truncate(k);
    }
    (cr, Blob(dst), dr, Blob(out))
}

/// `ZSTD_compress_usingDict` / `ZSTD_decompress_usingDict`,
/// `ZSTD_compress_usingCDict` / `_advanced` / `ZSTD_decompress_usingDDict`:
/// exact compressed bytes and exact decompressed bytes over the payload sizes
/// that straddle the block boundary, and over the three dictionary shapes.
#[test]
fn t_dict_roundtrip_oneshot() {
    covers(&["CFG:72", "CFG:73", "CFG:74", "CFG:90", "CFG:182"]);
    let tr = trained_dict(4096);
    let dicts: [(&str, Vec<u8>); 5] = [
        ("empty", vec![]),
        ("7bytes", raw_dict(7)),
        ("8bytes", raw_dict(8)),
        ("raw64K", raw_dict(65536)),
        ("trained", tr.clone()),
    ];
    for &n in &[0usize, 1, 100, 4096, 131072, 300_000] {
        let src = payload(n);
        for (tag, d) in &dicts {
            for &lvl in &[1i32, 3, 19] {
                if n >= 131072 && lvl == 19 {
                    continue; // btultra2 on 300 KB in an -O0 C build is slow
                }
                let (cr, _, dr, out) =
                    diff(&format!("usingDict({tag},lvl {lvl},src {n})"), |l| {
                        usingdict_roundtrip(l, d, lvl, &src)
                    });
                expect_ok("usingDict compress", &cr);
                expect_ok("usingDict decompress", &dr);
                assert_eq!(out.0, src, "usingDict round trip mismatch");
            }
        }
    }
    // A magic-prefixed buffer with a corrupted entropy section: dct_auto sees
    // the magic and must reject it.
    let bad = magic_dict(4096, 7);
    for &lvl in &[1i32, 3] {
        let (cr, _, _, _) = diff(&format!("usingDict(corruptMagic,lvl {lvl})"), |l| {
            usingdict_roundtrip(l, &bad, lvl, &payload(1000))
        });
        expect_err("usingDict(corruptMagic)", &cr, E_dictionary_corrupted);
    }

    // CDict-based one-shot compression: level cutoffs for dictionary attachment
    // (8 KB for fast/btultra2, 16 KB for dfast, 32 KB for greedy/btlazy2).
    let src_sizes = [1000usize, 8192, 8193, 16384, 16385, 32768, 32769, 131072, 400_000];
    let dict64 = raw_dict(65536);
    for &lvl in &[1i32, 3, 5, 13, 19] {
        for &n in &src_sizes {
            if n > 131072 && lvl >= 13 {
                continue;
            }
            let src = payload(n);
            let info = diff(&format!("cdict roundtrip(lvl {lvl}, src {n})"), |l| {
                dict_probe(
                    l,
                    &dict64,
                    false,
                    lvl,
                    ZSTD_dlm_byCopy,
                    ZSTD_dct_auto,
                    Ctor::Simple,
                    0,
                    &src,
                )
            });
            assert_eq!(info.roundtrip_ok, Some(true));
        }
    }

    // (182) ZSTD_compress_usingCDict_advanced x all 8 fParams combinations.
    for &n in &[0usize, 100, 1 << 20] {
        let src = payload(n);
        for cs in [0i32, 1] {
            for ck in [0i32, 1] {
                for nd in [0i32, 1] {
                    let (cr, cbytes) = diff_bytes(
                        &format!("usingCDict_advanced(src {n}, fp {cs}{ck}{nd})"),
                        |l| {
                            let cctx = Ctx::cctx(l);
                            let cr = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                            let cd = unsafe { cr(tr.as_ptr() as *const c_void, tr.len(), 5) };
                            assert!(!cd.is_null());
                            let cd = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                            let cap = compress_bound(l, src.len()) + 64;
                            let mut dst = vec![0xCDu8; cap];
                            let f = l.sym::<FnCompressUsingCDictAdv>(
                                "ZSTD_compress_usingCDict_advanced",
                            );
                            let fp = ZSTD_frameParameters {
                                contentSizeFlag: cs,
                                checksumFlag: ck,
                                noDictIDFlag: nd,
                            };
                            let k = unsafe {
                                f(
                                    cctx.ptr,
                                    dst.as_mut_ptr() as *mut c_void,
                                    cap,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    cd.ptr,
                                    fp,
                                )
                            };
                            let r = res(l, k);
                            if let R::Ok(m) = r {
                                dst.truncate(m);
                            }
                            (r, Blob(dst))
                        },
                    );
                    expect_ok("usingCDict_advanced", &cr);
                    // noDictIDFlag must be honoured in the frame header.
                    let got = diff("dictID from frame", |l| {
                        let g = l.sym::<FnIdFromBuf>("ZSTD_getDictID_fromFrame");
                        unsafe { g(cbytes.0.as_ptr() as *const c_void, cbytes.0.len()) }
                    });
                    if nd == 1 {
                        assert_eq!(got, 0, "noDictIDFlag=1 must strip the dictID");
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sticky vs one-shot dictionary attachment
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
enum Attach {
    LoadDict,
    LoadDictByRef,
    LoadDictAdv(c_int, c_int),
    RefCDict,
    RefPrefix,
    RefPrefixAdv(c_int),
    None,
}

/// Apply `how` to a CCtx, then compress `src` three times through the *same*
/// context without re-applying anything.
fn three_frames_c(l: &Lib, how: Attach, dict: &[u8], lvl: c_int, src: &[u8]) -> (Vec<R>, Vec<Blob>) {
    let cctx = Ctx::cctx(l);
    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    assert!(!is_error(l, unsafe {
        set(cctx.ptr, ZSTD_c_compressionLevel, lvl)
    }));
    let dp = dict.as_ptr() as *const c_void;
    let mut cdict_holder = None;
    let st = unsafe {
        match how {
            Attach::LoadDict => {
                let f = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
                f(cctx.ptr, dp, dict.len())
            }
            Attach::LoadDictByRef => {
                let f = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference");
                f(cctx.ptr, dp, dict.len())
            }
            Attach::LoadDictAdv(dlm, dct) => {
                let f = l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
                f(cctx.ptr, dp, dict.len(), dlm, dct)
            }
            Attach::RefCDict => {
                let c = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                let cd = c(dp, dict.len(), lvl);
                let st = if cd.is_null() {
                    0
                } else {
                    let f = l.sym::<FnRefPtr>("ZSTD_CCtx_refCDict");
                    f(cctx.ptr, cd)
                };
                if !cd.is_null() {
                    cdict_holder = Some(Ctx::from_raw(l, cd, "ZSTD_freeCDict"));
                }
                st
            }
            Attach::RefPrefix => {
                let f = l.sym::<FnLoadDict>("ZSTD_CCtx_refPrefix");
                f(cctx.ptr, dp, dict.len())
            }
            Attach::RefPrefixAdv(dct) => {
                let f = l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
                f(cctx.ptr, dp, dict.len(), dct)
            }
            Attach::None => 0,
        }
    };
    let mut rs = vec![res(l, st)];
    let mut frames = Vec::new();
    let f = l.sym::<FnCompress2>("ZSTD_compress2");
    for _ in 0..3 {
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            f(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            dst.truncate(k);
        }
        rs.push(r);
        frames.push(Blob(dst));
    }
    drop(cdict_holder.take());
    (rs, frames)
}

/// Decode the three frames of `frames` through one DCtx with `how` applied once.
fn three_frames_d(l: &Lib, how: Attach, dict: &[u8], frames: &[Vec<u8>], plain_len: usize) -> (Vec<R>, Vec<Blob>) {
    let dctx = Ctx::dctx(l);
    let dp = dict.as_ptr() as *const c_void;
    let mut ddict_holder = None;
    let st = unsafe {
        match how {
            Attach::LoadDict => {
                let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
                f(dctx.ptr, dp, dict.len())
            }
            Attach::LoadDictByRef => {
                let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
                f(dctx.ptr, dp, dict.len())
            }
            Attach::LoadDictAdv(dlm, dct) => {
                let f = l.sym::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
                f(dctx.ptr, dp, dict.len(), dlm, dct)
            }
            Attach::RefCDict => {
                let c = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                let dd = c(dp, dict.len());
                let st = if dd.is_null() {
                    0
                } else {
                    let f = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
                    f(dctx.ptr, dd)
                };
                if !dd.is_null() {
                    ddict_holder = Some(Ctx::from_raw(l, dd, "ZSTD_freeDDict"));
                }
                st
            }
            Attach::RefPrefix => {
                let f = l.sym::<FnLoadDict>("ZSTD_DCtx_refPrefix");
                f(dctx.ptr, dp, dict.len())
            }
            Attach::RefPrefixAdv(dct) => {
                let f = l.sym::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
                f(dctx.ptr, dp, dict.len(), dct)
            }
            Attach::None => 0,
        }
    };
    let mut rs = vec![res(l, st)];
    let mut outs = Vec::new();
    let g = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    for fr in frames {
        let mut out = vec![0xABu8; plain_len + 64];
        let n = unsafe {
            g(
                dctx.ptr,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                fr.as_ptr() as *const c_void,
                fr.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            out.truncate(k);
        }
        rs.push(r);
        outs.push(Blob(out));
    }
    drop(ddict_holder.take());
    (rs, outs)
}

/// The STICKY-vs-ONE-SHOT distinction, and every `*loadDictionary*` /
/// `*refPrefix*` / `refCDict` / `refDDict` variant:
/// `loadDictionary` and `refCDict` persist across frames, `refPrefix` applies to
/// the next frame only. Three frames are driven through one context for each and
/// all three are compared byte for byte.
#[test]
fn t_dict_sticky_vs_oneshot() {
    covers(&[
        "CFG:32", "CFG:33", "CFG:34", "CFG:43", "CFG:44", "CFG:75", "CFG:79", "CFG:90",
        "CFG:94", "CFG:322", "CFG:323",
    ]);
    let tr = trained_dict(4096);
    let raw = raw_dict(65536);
    let src = payload(100_000);

    // Baseline: no dictionary at all.
    let (_, base) = diff("3 frames, no dict", |l| {
        three_frames_c(l, Attach::None, &[], 5, &src)
    });
    for f in &base[1..] {
        assert_eq!(*f, base[0], "consecutive dictless frames must be identical");
    }

    // Sticky: the same dictionary applies to all three frames.
    for (tag, how) in [
        ("loadDictionary", Attach::LoadDict),
        ("loadDictionary_byReference", Attach::LoadDictByRef),
        (
            "loadDictionary_advanced(byRef,rawContent)",
            Attach::LoadDictAdv(ZSTD_dlm_byRef, ZSTD_dct_rawContent),
        ),
        (
            "loadDictionary_advanced(byCopy,fullDict)",
            Attach::LoadDictAdv(ZSTD_dlm_byCopy, ZSTD_dct_fullDict),
        ),
        ("refCDict", Attach::RefCDict),
    ] {
        let d: &[u8] = if matches!(how, Attach::LoadDictAdv(_, ZSTD_dct_fullDict)) {
            &tr
        } else {
            &raw
        };
        let (rs, frames) = diff(&format!("3 frames, {tag}"), |l| {
            three_frames_c(l, how, d, 5, &src)
        });
        for r in &rs {
            expect_ok(tag, r);
        }
        assert_eq!(frames[1], frames[0], "{tag} must be sticky (frame 2)");
        assert_eq!(frames[2], frames[0], "{tag} must be sticky (frame 3)");
        assert_ne!(
            frames[0], base[0],
            "{tag} did not change the output at all — the fixture is useless"
        );
        // Decode all three with a matching sticky decoder setup.
        let raws: Vec<Vec<u8>> = frames.iter().map(|b| b.0.clone()).collect();
        let dhow = match how {
            Attach::RefCDict => Attach::RefCDict,
            Attach::LoadDictAdv(a, b) => Attach::LoadDictAdv(a, b),
            Attach::LoadDictByRef => Attach::LoadDictByRef,
            _ => Attach::LoadDict,
        };
        let (drs, outs) = diff(&format!("3 frames decode, {tag}"), |l| {
            three_frames_d(l, dhow, d, &raws, src.len())
        });
        for r in &drs {
            expect_ok(tag, r);
        }
        for o in &outs {
            assert_eq!(o.0, src, "{tag} round trip mismatch");
        }
    }

    // One-shot: refPrefix affects only the next frame, so frames 2 and 3 must
    // be identical to the dictless baseline.
    for (tag, how) in [
        ("refPrefix", Attach::RefPrefix),
        (
            "refPrefix_advanced(rawContent)",
            Attach::RefPrefixAdv(ZSTD_dct_rawContent),
        ),
        (
            "refPrefix_advanced(auto)",
            Attach::RefPrefixAdv(ZSTD_dct_auto),
        ),
    ] {
        let (rs, frames) = diff(&format!("3 frames, {tag}"), |l| {
            three_frames_c(l, how, &raw, 5, &src)
        });
        for r in &rs {
            expect_ok(tag, r);
        }
        assert_ne!(frames[0], base[0], "{tag} had no effect on frame 1");
        assert_eq!(
            frames[1], base[0],
            "{tag} must apply to the NEXT FRAME ONLY (frame 2 leaked the prefix)"
        );
        assert_eq!(frames[2], base[0], "{tag} leaked into frame 3");
        // The decoder side has the same one-shot semantics.
        let raws: Vec<Vec<u8>> = frames.iter().map(|b| b.0.clone()).collect();
        let (_, outs) = diff(&format!("3 frames decode, {tag}"), |l| {
            three_frames_d(l, Attach::RefPrefix, &raw, &raws, src.len())
        });
        for o in &outs {
            assert_eq!(o.0, src, "{tag} decode mismatch");
        }
    }

    // loadDictionary(NULL, 0) clears a previously loaded dictionary.
    diff("loadDictionary then clear", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let a = unsafe { f(cctx.ptr, raw.as_ptr() as *const c_void, raw.len()) };
        let b = unsafe { f(cctx.ptr, std::ptr::null(), 0) };
        let g = l.sym::<FnCompress2>("ZSTD_compress2");
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            g(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            dst.truncate(k);
        }
        (res(l, a), res(l, b), r, Blob(dst))
    });
    // refCDict(NULL) also clears.
    diff("refCDict(NULL)", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnRefPtr>("ZSTD_CCtx_refCDict");
        let a = unsafe { f(cctx.ptr, std::ptr::null()) };
        let g = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
        let dctx = Ctx::dctx(l);
        let b = unsafe { g(dctx.ptr, std::ptr::null()) };
        (res(l, a), res(l, b))
    });

    // Prefix sizes and window interaction (CFG:79 / CFG:32).
    for &psz in &[0usize, 1, 8, 4096, 65536, 1 << 20] {
        let pref = raw_dict(psz.max(1));
        let pref = &pref[..psz];
        for &n in &[0usize, 100, 65536] {
            let s = payload(n);
            for &wlog in &[0i32, 17] {
                let (rs, frames) = diff(
                    &format!("refPrefix(prefix {psz}, src {n}, wlog {wlog})"),
                    |l| {
                        let cctx = Ctx::cctx(l);
                        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                        unsafe {
                            set(cctx.ptr, ZSTD_c_compressionLevel, 3);
                            if wlog != 0 {
                                set(cctx.ptr, ZSTD_c_windowLog, wlog);
                            }
                        }
                        let f = l.sym::<FnLoadDict>("ZSTD_CCtx_refPrefix");
                        let a = unsafe {
                            f(cctx.ptr, pref.as_ptr() as *const c_void, pref.len())
                        };
                        let g = l.sym::<FnCompress2>("ZSTD_compress2");
                        let cap = compress_bound(l, s.len()) + 64;
                        let mut dst = vec![0xCDu8; cap];
                        let k = unsafe {
                            g(
                                cctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                s.as_ptr() as *const c_void,
                                s.len(),
                            )
                        };
                        let r = res(l, k);
                        if let R::Ok(m) = r {
                            dst.truncate(m);
                        }
                        (vec![res(l, a), r], vec![Blob(dst)])
                    },
                );
                for r in &rs {
                    expect_ok("refPrefix sweep", r);
                }
                // Decode with the same prefix.
                let raws = vec![frames[0].0.clone()];
                let (_, outs) = diff(
                    &format!("refPrefix decode(prefix {psz}, src {n}, wlog {wlog})"),
                    |l| three_frames_d(l, Attach::RefPrefix, pref, &raws, s.len()),
                );
                assert_eq!(outs[0].0, s);
            }
        }
    }

    // Dictionary with an explicit dictID + ZSTD_c_dictIDFlag (CFG:43/44).
    for id in [1u32, 255, 256, 65535, 65536, 0xFFFF_FFFF] {
        let d = dict_with_id(id);
        for flag in [0i32, 1] {
            for &n in &[0usize, 100, 20000] {
                let s = payload(n);
                let (rs, frames) = diff(&format!("dictID {id}, flag {flag}, src {n}"), |l| {
                    let cctx = Ctx::cctx(l);
                    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                    let a = unsafe { set(cctx.ptr, ZSTD_c_dictIDFlag, flag) };
                    let ld = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
                    let b = unsafe { ld(cctx.ptr, d.as_ptr() as *const c_void, d.len()) };
                    let g = l.sym::<FnCompress2>("ZSTD_compress2");
                    let cap = compress_bound(l, s.len()) + 64;
                    let mut dst = vec![0xCDu8; cap];
                    let k = unsafe {
                        g(
                            cctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            s.as_ptr() as *const c_void,
                            s.len(),
                        )
                    };
                    let r = res(l, k);
                    if let R::Ok(m) = r {
                        dst.truncate(m);
                    }
                    (vec![res(l, a), res(l, b), r], vec![Blob(dst)])
                });
                for r in &rs {
                    expect_ok("dictID flag", r);
                }
                let got = diff("dictID_fromFrame", |l| {
                    let gf = l.sym::<FnIdFromBuf>("ZSTD_getDictID_fromFrame");
                    unsafe { gf(frames[0].0.as_ptr() as *const c_void, frames[0].0.len()) }
                });
                if flag == 0 {
                    assert_eq!(got, 0, "dictIDFlag=0 must strip the dictID");
                } else if n > 0 {
                    assert_eq!(got, id, "dictIDFlag=1 must carry the dictID");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dictionary attachment strategy
// ---------------------------------------------------------------------------

/// `ZSTD_c_forceAttachDict` x `ZSTD_c_enableDedicatedDictSearch` x
/// `ZSTD_c_prefetchCDictTables`: three completely different ways the C uses a
/// CDict (attach the CDict's tables / copy them into the CCtx / reload the
/// dictionary from scratch). The compressed bytes may legitimately differ
/// *between* settings, but must match between C and Rust for each setting.
#[test]
fn t_dict_attach_prefs() {
    covers(&[
        "CFG:126", "CFG:127", "CFG:128", "CFG:131", "CFG:132", "CFG:139", "CFG:76",
    ]);
    let dict = raw_dict(65536);
    let tr = trained_dict(4096);

    // Out-of-range ZSTD_dictAttachPref_e must be rejected by the bounds check.
    for v in [4i32, -1, 999] {
        let r = diff(&format!("forceAttachDict({v})"), |l| {
            let cctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            res(l, unsafe { set(cctx.ptr, ZSTD_c_forceAttachDict, v) })
        });
        expect_err("forceAttachDict out of range", &r, E_parameter_outOfBound);
    }

    for &lvl in &[1i32, 5, 19] {
        for &pref in &[
            ZSTD_dictDefaultAttach,
            ZSTD_dictForceAttach,
            ZSTD_dictForceCopy,
            ZSTD_dictForceLoad,
        ] {
            for &pfetch in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                for &dsz in &[0usize, 4096, 65536] {
                    for &n in &[4096usize, 8192, 8193, 32768] {
                        let src = payload(n);
                        let d = &dict[..dsz];
                        let (rs, frames) = diff(
                            &format!(
                                "attach(lvl {lvl}, pref {pref}, prefetch {pfetch}, dict {dsz}, src {n})"
                            ),
                            |l| {
                                let cctx = Ctx::cctx(l);
                                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                                let mut rs = Vec::new();
                                unsafe {
                                    rs.push(res(l, set(cctx.ptr, ZSTD_c_compressionLevel, lvl)));
                                    rs.push(res(l, set(cctx.ptr, ZSTD_c_forceAttachDict, pref)));
                                    rs.push(res(
                                        l,
                                        set(cctx.ptr, ZSTD_c_prefetchCDictTables, pfetch),
                                    ));
                                }
                                let c = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                                let cd =
                                    unsafe { c(d.as_ptr() as *const c_void, d.len(), lvl) };
                                let holder = if cd.is_null() {
                                    None
                                } else {
                                    let f = l.sym::<FnRefPtr>("ZSTD_CCtx_refCDict");
                                    rs.push(res(l, unsafe { f(cctx.ptr, cd) }));
                                    Some(Ctx::from_raw(l, cd, "ZSTD_freeCDict"))
                                };
                                let g = l.sym::<FnCompress2>("ZSTD_compress2");
                                let cap = compress_bound(l, src.len()) + 64;
                                let mut dst = vec![0xCDu8; cap];
                                let k = unsafe {
                                    g(
                                        cctx.ptr,
                                        dst.as_mut_ptr() as *mut c_void,
                                        cap,
                                        src.as_ptr() as *const c_void,
                                        src.len(),
                                    )
                                };
                                let r = res(l, k);
                                if let R::Ok(m) = r {
                                    dst.truncate(m);
                                }
                                rs.push(r);
                                drop(holder);
                                (rs, vec![Blob(dst)])
                            },
                        );
                        for r in &rs {
                            expect_ok("attach sweep", r);
                        }
                        // Round trip through a DDict built from the same bytes.
                        let raws = vec![frames[0].0.clone()];
                        let (_, outs) = diff(
                            &format!("attach decode(lvl {lvl}, pref {pref}, dict {dsz}, src {n})"),
                            |l| three_frames_d(l, Attach::RefCDict, d, &raws, src.len()),
                        );
                        assert_eq!(outs[0].0, src);
                    }
                }
            }
        }
    }

    // Row 126 also wants the `ZSTD_CONTENTSIZE_UNKNOWN` case: driving the
    // compression through `ZSTD_compressStream` without a pledged source size
    // leaves `pledgedSrcSizePlusOne == 0`, which `ZSTD_shouldAttachDict` reads as
    // "unknown" and therefore always attaches the CDict regardless of the
    // per-strategy cutoff.
    for &pf in &[
        ZSTD_dictDefaultAttach,
        ZSTD_dictForceAttach,
        ZSTD_dictForceCopy,
        ZSTD_dictForceLoad,
    ] {
        for &lvl in &[1i32, 5, 19] {
            let src = payload(60000);
            let (rs, cb) = diff(
                &format!("attach(unknown srcSize, pref {pf}, lvl {lvl})"),
                |l| {
                    let zcs = Ctx::cstream(l);
                    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                    let mut rs = Vec::new();
                    unsafe {
                        rs.push(res(l, set(zcs.ptr, ZSTD_c_compressionLevel, lvl)));
                        rs.push(res(l, set(zcs.ptr, ZSTD_c_forceAttachDict, pf)));
                    }
                    let c = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                    let cd = unsafe {
                        c(dict.as_ptr() as *const c_void, dict.len(), lvl)
                    };
                    assert!(!cd.is_null());
                    let cd = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                    let f = l.sym::<FnRefPtr>("ZSTD_CCtx_refCDict");
                    rs.push(res(l, unsafe { f(zcs.ptr, cd.ptr) }));
                    let (srs, b) = stream_out(l, zcs.ptr, &src, 16384, 4096);
                    rs.extend(srs);
                    (rs, b)
                },
            );
            for r in &rs {
                expect_ok("attach unknown srcSize", r);
            }
            let raws = vec![cb.0.clone()];
            let (_, outs) = diff(
                &format!("attach(unknown srcSize, pref {pf}, lvl {lvl}) decode"),
                |l| three_frames_d(l, Attach::RefCDict, &dict, &raws, src.len()),
            );
            assert_eq!(outs[0].0, src);
        }
    }

    // (131/132) enableDedicatedDictSearch through cctxParams + advanced2, at a
    // level where DDS is supported (5 -> lazy2) and one where it is not
    // (1 -> fast).
    for &lvl in &[1i32, 5, 12] {
        for &dds in &[0i32, 1] {
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let d = if dct == ZSTD_dct_fullDict { &tr } else { &dict };
                    diff(&format!("dds(lvl {lvl}, dds {dds}, dlm {dlm}, dct {dct})"), |l| {
                        dict_probe(
                            l,
                            d,
                            false,
                            lvl,
                            dlm,
                            dct,
                            Ctor::Advanced2,
                            dds,
                            &payload(8192),
                        )
                    });
                }
            }
        }
    }

    // (76) dictionary sizes across the strategies.
    for strat in ALL_STRATEGIES {
        for &dsz in &[8usize, 9, 100, 1024, 65536] {
            let d = &dict[..dsz];
            let src = payload(20000);
            let (rs, _) = diff(&format!("strategy {strat}, dict {dsz}"), |l| {
                let cctx = Ctx::cctx(l);
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let mut rs = Vec::new();
                unsafe {
                    rs.push(res(l, set(cctx.ptr, ZSTD_c_strategy, *strat)));
                }
                let ld = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
                rs.push(res(l, unsafe {
                    ld(cctx.ptr, d.as_ptr() as *const c_void, d.len())
                }));
                let g = l.sym::<FnCompress2>("ZSTD_compress2");
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                let k = unsafe {
                    g(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                };
                let r = res(l, k);
                if let R::Ok(m) = r {
                    dst.truncate(m);
                }
                rs.push(r);
                (rs, vec![Blob(dst)])
            });
            for r in &rs {
                expect_ok("strategy sweep", r);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ZSTD_d_refMultipleDDicts
// ---------------------------------------------------------------------------

/// Decode a list of frames through one DCtx with `ZSTD_decompressStream` — the
/// only path that consults the DDict hash set, since `ZSTD_DCtx_selectFrameDDict`
/// is called from `ZSTD_decompressStream`'s frame-header stage. The one-shot
/// `ZSTD_decompressDCtx` instead uses `ZSTD_getDDict`, i.e. the single most
/// recently referenced DDict.
fn stream_decode_frames(
    l: &Lib,
    dctx: *mut c_void,
    frames: &[Vec<u8>],
    plain: usize,
) -> (Vec<R>, Vec<Blob>) {
    let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut rs = Vec::new();
    let mut outs = Vec::new();
    let mut obuf = vec![0u8; plain + 64];
    for fr in frames {
        let mut acc = Vec::new();
        let mut input = ZSTD_inBuffer {
            src: fr.as_ptr() as *const c_void,
            size: fr.len(),
            pos: 0,
        };
        loop {
            let mut output = ZSTD_outBuffer {
                dst: obuf.as_mut_ptr() as *mut c_void,
                size: obuf.len(),
                pos: 0,
            };
            let r = unsafe { ds(dctx, &mut output, &mut input) };
            acc.extend_from_slice(&obuf[..output.pos]);
            let rr = res(l, r);
            let stop = is_error(l, r) || r == 0 || (input.pos == input.size && output.pos == 0);
            rs.push(rr);
            if stop {
                break;
            }
        }
        outs.push(Blob(acc));
    }
    (rs, outs)
}

/// `ZSTD_d_refMultipleDDicts`: several DDicts in one DCtx, then frames that
/// reference different dictIDs (including one with no dictID and one with an
/// unknown dictID), plus two DDicts sharing a dictID and enough DDicts to force
/// the hash set to grow. Both `rmd_refSingleDDict` and `rmd_refMultipleDDicts`
/// are driven, and both the streaming and the one-shot decode entry points.
#[test]
fn t_dict_ref_multiple_ddicts() {
    covers(&["CFG:317"]);
    let ids = [11u32, 22, 33];
    let dicts: Vec<Vec<u8>> = ids.iter().map(|&i| dict_with_id(i)).collect();
    let src = payload(20000);

    // One frame per dictionary, plus a frame built with a fourth dictionary the
    // DCtx will not know about, plus a frame with no dictionary at all.
    let unknown = dict_with_id(44);
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for d in dicts.iter().chain(std::iter::once(&unknown)) {
        let (cr, b, _, _) = diff("rmd fixture frame", |l| {
            usingdict_roundtrip(l, d, 3, &src)
        });
        expect_ok("rmd fixture", &cr);
        frames.push(b.0);
    }
    frames.push(c_compress(&src, 3));

    for &mode in &[ZSTD_rmd_refSingleDDict, ZSTD_rmd_refMultipleDDicts] {
        // Reference them in the order 1, 3, 2 so the hash-set insertion order
        // differs from the dictID order and from the decode order.
        let order = [0usize, 2, 1];
        let (rs, outs) = diff(&format!("refMultipleDDicts stream({mode})"), |l| {
            let dctx = Ctx::dctx(l);
            let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
            let mut rs = vec![res(l, unsafe {
                set(dctx.ptr, ZSTD_d_refMultipleDDicts, mode)
            })];
            let mut holders = Vec::new();
            let cr = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let rf = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
            for &i in &order {
                let d = &dicts[i];
                let dd = unsafe { cr(d.as_ptr() as *const c_void, d.len()) };
                assert!(!dd.is_null());
                rs.push(res(l, unsafe { rf(dctx.ptr, dd) }));
                holders.push(Ctx::from_raw(l, dd, "ZSTD_freeDDict"));
            }
            let (drs, outs) = stream_decode_frames(l, dctx.ptr, &frames, src.len());
            rs.extend(drs);
            drop(holders);
            (rs, outs)
        });
        for r in &rs[..1 + order.len()] {
            expect_ok("rmd setup", r);
        }
        if mode == ZSTD_rmd_refMultipleDDicts {
            // Every known dictID must be found in the hash set regardless of the
            // order it was inserted in.
            for i in 0..3 {
                assert_eq!(
                    outs[i].0, src,
                    "refMultipleDDicts failed to select the DDict for dictID {}",
                    ids[i]
                );
            }
        } else {
            // With a single DDict only the *last* referenced dictionary is
            // usable, i.e. dictID 22 (order[2] == 1).
            assert_eq!(outs[1].0, src);
        }
    }

    // The one-shot path: `ZSTD_decompressDCtx` uses `ZSTD_getDDict`, which
    // ignores the hash set, so only the most recently referenced DDict works.
    // Pin whatever the C returns for each frame.
    for &mode in &[ZSTD_rmd_refSingleDDict, ZSTD_rmd_refMultipleDDicts] {
        diff(&format!("refMultipleDDicts oneshot({mode})"), |l| {
            let dctx = Ctx::dctx(l);
            let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
            let mut rs = vec![res(l, unsafe {
                set(dctx.ptr, ZSTD_d_refMultipleDDicts, mode)
            })];
            let mut holders = Vec::new();
            let cr = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let rf = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
            for d in &dicts {
                let dd = unsafe { cr(d.as_ptr() as *const c_void, d.len()) };
                assert!(!dd.is_null());
                rs.push(res(l, unsafe { rf(dctx.ptr, dd) }));
                holders.push(Ctx::from_raw(l, dd, "ZSTD_freeDDict"));
            }
            let g = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let mut outs = Vec::new();
            for fr in &frames {
                let mut out = vec![0xABu8; src.len() + 64];
                let n = unsafe {
                    g(
                        dctx.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        fr.as_ptr() as *const c_void,
                        fr.len(),
                    )
                };
                let r = res(l, n);
                if let R::Ok(k) = r {
                    out.truncate(k);
                }
                rs.push(r);
                outs.push(Blob(out));
            }
            drop(holders);
            (rs, outs)
        });
    }

    // Two DDicts with the SAME dictID, plus enough distinct dictionaries to
    // force the hash set past its initial capacity.
    let same_id_a = dict_with_id(77);
    let same_id_b = {
        let s = txt64();
        let content = corpus(Corpus::Text, 3000, 0x7024_0177);
        let (r, b) = diff_bytes("fixture: second dict with dictID 77", |l| {
            finalize(
                l,
                4096,
                &content,
                &s,
                ZDICT_params_t {
                    compressionLevel: 0,
                    notificationLevel: 0,
                    dictID: 77,
                },
            )
        });
        let n = expect_ok("fixture same id", &r);
        b.0[..n].to_vec()
    };
    let frame77 = {
        let (cr, b, _, _) = diff("rmd same-id frame", |l| {
            usingdict_roundtrip(l, &same_id_a, 3, &src)
        });
        expect_ok("rmd same-id", &cr);
        vec![b.0]
    };
    let many: Vec<Vec<u8>> = (100u32..140).map(dict_with_id).collect();
    diff("refMultipleDDicts(same dictID + hash-set growth)", |l| {
        let dctx = Ctx::dctx(l);
        let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
        let mut rs = vec![res(l, unsafe {
            set(
                dctx.ptr,
                ZSTD_d_refMultipleDDicts,
                ZSTD_rmd_refMultipleDDicts,
            )
        })];
        let cr = l.sym::<FnCreateDDict>("ZSTD_createDDict");
        let rf = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
        let mut holders = Vec::new();
        for d in [&same_id_a, &same_id_b]
            .into_iter()
            .chain(many.iter())
            .chain(dicts.iter())
        {
            let dd = unsafe { cr(d.as_ptr() as *const c_void, d.len()) };
            assert!(!dd.is_null());
            rs.push(res(l, unsafe { rf(dctx.ptr, dd) }));
            holders.push(Ctx::from_raw(l, dd, "ZSTD_freeDDict"));
        }
        let (drs, outs) = stream_decode_frames(l, dctx.ptr, &frame77, src.len());
        rs.extend(drs);
        drop(holders);
        (rs, outs)
    });
}


// ---------------------------------------------------------------------------
// Dictionary streaming
// ---------------------------------------------------------------------------

/// Chunked compression through an already-initialised CStream.
fn stream_out(l: &Lib, zcs: *mut c_void, src: &[u8], in_chunk: usize, out_chunk: usize) -> (Vec<R>, Blob) {
    let cs = l.sym::<FnDecompressStream>("ZSTD_compressStream");
    let es = l.sym::<FnEndStream>("ZSTD_endStream");
    let mut rs = Vec::new();
    let mut acc = Vec::new();
    let mut pos = 0usize;
    let mut obuf = vec![0u8; out_chunk];
    while pos < src.len() {
        let n = in_chunk.min(src.len() - pos);
        let mut input = ZSTD_inBuffer {
            src: unsafe { src.as_ptr().add(pos) } as *const c_void,
            size: n,
            pos: 0,
        };
        while input.pos < input.size {
            let mut output = ZSTD_outBuffer {
                dst: obuf.as_mut_ptr() as *mut c_void,
                size: out_chunk,
                pos: 0,
            };
            let r = unsafe { cs(zcs, &mut output, &mut input) };
            rs.push(res(l, r));
            if is_error(l, r) {
                return (rs, Blob(acc));
            }
            acc.extend_from_slice(&obuf[..output.pos]);
        }
        pos += n;
    }
    loop {
        let mut output = ZSTD_outBuffer {
            dst: obuf.as_mut_ptr() as *mut c_void,
            size: out_chunk,
            pos: 0,
        };
        let r = unsafe { es(zcs, &mut output) };
        rs.push(res(l, r));
        acc.extend_from_slice(&obuf[..output.pos]);
        if is_error(l, r) || r == 0 {
            break;
        }
    }
    (rs, Blob(acc))
}

/// Chunked decompression through an already-initialised DStream.
fn stream_in(l: &Lib, zds: *mut c_void, src: &[u8], in_chunk: usize, out_chunk: usize) -> (Vec<R>, Blob) {
    let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut rs = Vec::new();
    let mut acc = Vec::new();
    let mut input = ZSTD_inBuffer {
        src: src.as_ptr() as *const c_void,
        size: 0,
        pos: 0,
    };
    let mut obuf = vec![0u8; out_chunk];
    while input.pos < src.len() {
        input.size = (input.pos + in_chunk).min(src.len());
        loop {
            let mut output = ZSTD_outBuffer {
                dst: obuf.as_mut_ptr() as *mut c_void,
                size: out_chunk,
                pos: 0,
            };
            let before = input.pos;
            let r = unsafe { ds(zds, &mut output, &mut input) };
            rs.push(res(l, r));
            if is_error(l, r) {
                return (rs, Blob(acc));
            }
            acc.extend_from_slice(&obuf[..output.pos]);
            if input.pos == input.size || (input.pos == before && output.pos == 0) {
                break;
            }
        }
    }
    (rs, Blob(acc))
}

/// `ZSTD_initCStream_usingDict`, `ZSTD_initCStream_usingCDict`,
/// `ZSTD_initCStream_usingCDict_advanced`, `ZSTD_initDStream_usingDict` and
/// `ZSTD_initDStream_usingDDict` with chunked I/O.
#[test]
fn t_dict_streaming() {
    covers(&["CFG:99"]);
    let tr = trained_dict(4096);
    let raw = raw_dict(65536);
    let src = payload(100_000);

    for (tag, d) in [("raw64K", &raw), ("trained", &tr)] {
        for &(ic, oc) in &[(1usize, 1usize), (7, 3), (4096, 1024), (200_000, 200_000)] {
            for &lvl in &[1i32, 3, 19] {
                if ic == 1 && lvl == 19 {
                    continue; // byte-at-a-time btultra2 over 100 KB is far too slow
                }
                let (rs, cb) = diff(
                    &format!("initCStream_usingDict({tag},lvl {lvl},{ic}/{oc})"),
                    |l| {
                        let zcs = Ctx::cstream(l);
                        let f = l.sym::<FnInitCStreamUsingDict>("ZSTD_initCStream_usingDict");
                        let a = unsafe {
                            f(zcs.ptr, d.as_ptr() as *const c_void, d.len(), lvl)
                        };
                        let (mut rs, b) = stream_out(l, zcs.ptr, &src, ic, oc);
                        rs.insert(0, res(l, a));
                        (rs, b)
                    },
                );
                for r in &rs {
                    expect_ok("initCStream_usingDict", r);
                }
                // Decode with initDStream_usingDict and with usingDDict.
                let bytes = cb.0.clone();
                let (drs, out) = diff(
                    &format!("initDStream_usingDict({tag},{ic}/{oc})"),
                    |l| {
                        let zds = Ctx::dstream(l);
                        let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
                        let a = unsafe { f(zds.ptr, d.as_ptr() as *const c_void, d.len()) };
                        let (mut rs, b) = stream_in(l, zds.ptr, &bytes, ic.max(1), oc.max(1));
                        rs.insert(0, res(l, a));
                        (rs, b)
                    },
                );
                for r in &drs {
                    expect_ok("initDStream_usingDict", r);
                }
                assert_eq!(out.0, src, "streaming dict round trip mismatch");
                let (drs2, out2) = diff(
                    &format!("initDStream_usingDDict({tag},{ic}/{oc})"),
                    |l| {
                        let zds = Ctx::dstream(l);
                        let cr = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                        let dd = unsafe { cr(d.as_ptr() as *const c_void, d.len()) };
                        assert!(!dd.is_null());
                        let dd = Ctx::from_raw(l, dd, "ZSTD_freeDDict");
                        let f = l.sym::<FnRefPtr>("ZSTD_initDStream_usingDDict");
                        let a = unsafe { f(zds.ptr, dd.ptr) };
                        let (mut rs, b) = stream_in(l, zds.ptr, &bytes, ic.max(1), oc.max(1));
                        rs.insert(0, res(l, a));
                        (rs, b)
                    },
                );
                for r in &drs2 {
                    expect_ok("initDStream_usingDDict", r);
                }
                assert_eq!(out2.0, src);
            }
        }

        // initCStream_usingCDict and its _advanced form.
        for &lvl in &[1i32, 5] {
            for &(cs, ck, nd) in &[(1i32, 0i32, 0i32), (0, 1, 1)] {
                for &pledged in &[src.len() as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                    let (rs, cb) = diff(
                        &format!("initCStream_usingCDict_advanced({tag},lvl {lvl},{cs}{ck}{nd},{pledged})"),
                        |l| {
                            let zcs = Ctx::cstream(l);
                            let cr = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                            let cd = unsafe { cr(d.as_ptr() as *const c_void, d.len(), lvl) };
                            assert!(!cd.is_null());
                            let cd = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                            let f = l.sym::<FnInitCStreamUsingCDictAdv>(
                                "ZSTD_initCStream_usingCDict_advanced",
                            );
                            let fp = ZSTD_frameParameters {
                                contentSizeFlag: cs,
                                checksumFlag: ck,
                                noDictIDFlag: nd,
                            };
                            let a = unsafe { f(zcs.ptr, cd.ptr, fp, pledged) };
                            let (mut rs, b) = stream_out(l, zcs.ptr, &src, 4096, 1024);
                            rs.insert(0, res(l, a));
                            (rs, b)
                        },
                    );
                    for r in &rs {
                        expect_ok("initCStream_usingCDict_advanced", r);
                    }
                    let bytes = cb.0.clone();
                    let (_, out) = diff("decode usingCDict_advanced stream", |l| {
                        let zds = Ctx::dstream(l);
                        let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
                        let a = unsafe { f(zds.ptr, d.as_ptr() as *const c_void, d.len()) };
                        let (mut rs, b) = stream_in(l, zds.ptr, &bytes, 4096, 4096);
                        rs.insert(0, res(l, a));
                        (rs, b)
                    });
                    assert_eq!(out.0, src);
                }
            }
            let (rs, cb) = diff(&format!("initCStream_usingCDict({tag},lvl {lvl})"), |l| {
                let zcs = Ctx::cstream(l);
                let cr = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                let cd = unsafe { cr(d.as_ptr() as *const c_void, d.len(), lvl) };
                assert!(!cd.is_null());
                let cd = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                let f = l.sym::<FnRefPtr>("ZSTD_initCStream_usingCDict");
                let a = unsafe { f(zcs.ptr, cd.ptr) };
                let (mut rs, b) = stream_out(l, zcs.ptr, &src, 8192, 4096);
                rs.insert(0, res(l, a));
                (rs, b)
            });
            for r in &rs {
                expect_ok("initCStream_usingCDict", r);
            }
            let bytes = cb.0.clone();
            let (_, out) = diff("decode usingCDict stream", |l| {
                let zds = Ctx::dstream(l);
                let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
                let a = unsafe { f(zds.ptr, d.as_ptr() as *const c_void, d.len()) };
                let (mut rs, b) = stream_in(l, zds.ptr, &bytes, 8192, 8192);
                rs.insert(0, res(l, a));
                (rs, b)
            });
            assert_eq!(out.0, src);
        }
    }
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

/// `dictionary_corrupted` (30), `dictionary_wrong` (32),
/// `dictionaryCreation_failed` (34) and `memory_allocation` (64) — including the
/// quirk that a corrupt dictionary fed to `ZSTD_DCtx_loadDictionary` surfaces as
/// `memory_allocation`, because `ZSTD_createDDict_advanced` swallows the real
/// error from `ZSTD_loadDEntropy` and just returns NULL, and the caller then
/// reports "allocation failed".
#[test]
fn t_dict_error_paths() {
    covers(&["CFG:90", "CFG:322", "CFG:323", "CFG:94"]);
    let tr = trained_dict(4096);
    let src = payload(20000);

    // A dictionary with a valid magic but a corrupted entropy section.
    let mut bad = tr.clone();
    for i in 8..24 {
        bad[i] ^= 0xFF;
    }

    // (30) dictionary_corrupted surfaces DIRECTLY only from the entry points
    // that call `ZSTD_loadDictionaryContent` / `ZSTD_loadDEntropy` themselves.
    for &lvl in &[1i32, 3, 19] {
        let r = diff(&format!("compress_usingDict(corrupt, lvl {lvl})"), |l| {
            usingdict_roundtrip(l, &bad, lvl, &src).0
        });
        expect_err("compress_usingDict(corrupt)", &r, E_dictionary_corrupted);
    }
    let good_frame = {
        let (cr, b, _, _) = diff("fixture: frame for the corrupt-dict decoder", |l| {
            usingdict_roundtrip(l, &tr, 3, &src)
        });
        expect_ok("fixture frame", &cr);
        b.0
    };
    let r = diff("decompress_usingDict(corrupt)", |l| {
        let dctx = Ctx::dctx(l);
        let g = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let mut out = vec![0xABu8; src.len() + 64];
        res(l, unsafe {
            g(
                dctx.ptr,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                good_frame.as_ptr() as *const c_void,
                good_frame.len(),
                bad.as_ptr() as *const c_void,
                bad.len(),
            )
        })
    });
    expect_err("decompress_usingDict(corrupt)", &r, E_dictionary_corrupted);

    // ...but via `ZSTD_CCtx_loadDictionary` it becomes `memory_allocation` (64),
    // exactly like the documented DCtx quirk: `ZSTD_initLocalDict` builds the
    // dictionary through `ZSTD_createCDict_advanced2`, which swallows the real
    // error and returns NULL, and the caller then blames the allocator.
    for &dct in &[ZSTD_dct_auto, ZSTD_dct_fullDict] {
        let r = diff(&format!("CCtx_loadDictionary(corrupt, dct {dct})"), |l| {
            let cctx = Ctx::cctx(l);
            let f = l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
            let a = unsafe {
                f(
                    cctx.ptr,
                    bad.as_ptr() as *const c_void,
                    bad.len(),
                    ZSTD_dlm_byCopy,
                    dct,
                )
            };
            let g = l.sym::<FnCompress2>("ZSTD_compress2");
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let k = unsafe {
                g(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            (res(l, a), res(l, k))
        });
        expect_ok("loadDictionary_advanced setter", &r.0);
        expect_err("compress2 with corrupt dict", &r.1, E_memory_allocation);
    }
    // rawContent accepts the same bytes.
    diff("CCtx_loadDictionary(corrupt, dct rawContent)", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let a = unsafe {
            f(
                cctx.ptr,
                bad.as_ptr() as *const c_void,
                bad.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_rawContent,
            )
        };
        let g = l.sym::<FnCompress2>("ZSTD_compress2");
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let k = unsafe {
            g(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        let r = res(l, k);
        if let R::Ok(m) = r {
            dst.truncate(m);
        }
        (res(l, a), r, Blob(dst))
    });

    // The memory_allocation quirk on the decoder side.
    for &dct in &[ZSTD_dct_auto, ZSTD_dct_fullDict] {
        let r = diff(&format!("DCtx_loadDictionary(corrupt, dct {dct})"), |l| {
            let dctx = Ctx::dctx(l);
            let f = l.sym::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
            res(l, unsafe {
                f(
                    dctx.ptr,
                    bad.as_ptr() as *const c_void,
                    bad.len(),
                    ZSTD_dlm_byCopy,
                    dct,
                )
            })
        });
        expect_err(
            "DCtx_loadDictionary(corrupt) reports memory_allocation",
            &r,
            E_memory_allocation,
        );
    }
    // ...and ZSTD_createDDict_advanced simply returns NULL for the same input.
    diff("createDDict_advanced(corrupt) -> NULL", |l| {
        let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let p = unsafe {
            f(
                bad.as_ptr() as *const c_void,
                bad.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_fullDict,
                ZSTD_customMem::default(),
            )
        };
        let null = p.is_null();
        if !null {
            let fr = l.sym::<FnFreeCCtx>("ZSTD_freeDDict");
            unsafe { fr(p) };
        }
        null
    });

    // (32) dictionary_wrong: decode a dict-compressed frame with no dictionary,
    // with a different dictionary, and with a wrong dictionary of the SAME
    // dictID.
    let d1 = dict_with_id(1234);
    let d2 = dict_with_id(5678);
    let d1b = {
        let s = txt64();
        let content = corpus(Corpus::Random, 2048, 0x7024_0199);
        let (r, b) = diff_bytes("fixture: same dictID different content", |l| {
            finalize(
                l,
                4096,
                &content,
                &s,
                ZDICT_params_t {
                    compressionLevel: 0,
                    notificationLevel: 0,
                    dictID: 1234,
                },
            )
        });
        let n = expect_ok("fixture d1b", &r);
        b.0[..n].to_vec()
    };
    let (cr, frame, _, _) = diff("dict_wrong fixture", |l| {
        usingdict_roundtrip(l, &d1, 3, &src)
    });
    expect_ok("dict_wrong fixture", &cr);
    let frame = frame.0;

    let r = diff("decompress_usingDict(NULL) on a dict frame", |l| {
        let dctx = Ctx::dctx(l);
        let g = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let mut out = vec![0xABu8; src.len() + 64];
        res(l, unsafe {
            g(
                dctx.ptr,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                std::ptr::null(),
                0,
            )
        })
    });
    expect_err("no dictionary", &r, E_dictionary_wrong);

    let r = diff("decompress_usingDict(different dictID)", |l| {
        let dctx = Ctx::dctx(l);
        let g = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let mut out = vec![0xABu8; src.len() + 64];
        res(l, unsafe {
            g(
                dctx.ptr,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                d2.as_ptr() as *const c_void,
                d2.len(),
            )
        })
    });
    expect_err("different dictID", &r, E_dictionary_wrong);

    // Same dictID, different content: the dictID check passes and the decode
    // fails later (or produces different bytes) — pin whatever the C does.
    diff("decompress_usingDict(same dictID, wrong content)", |l| {
        let dctx = Ctx::dctx(l);
        let g = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
        let mut out = vec![0xABu8; src.len() + 64];
        let n = unsafe {
            g(
                dctx.ptr,
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                d1b.as_ptr() as *const c_void,
                d1b.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            out.truncate(k);
        }
        (r, Blob(out))
    });

    // (34) dictionaryCreation_failed: ZSTD_createCDict_advanced with a
    // dictionary that cannot be loaded, then ZSTD_CCtx_refCDict of the NULL.
    diff("createCDict_advanced(corrupt, fullDict) -> NULL", |l| {
        let g = l.sym::<FnGetCParams>("ZSTD_getCParams");
        let cp = unsafe { g(3, 0, bad.len()) };
        let f = l.sym::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
        let p = unsafe {
            f(
                bad.as_ptr() as *const c_void,
                bad.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_fullDict,
                cp,
                ZSTD_customMem::default(),
            )
        };
        let null = p.is_null();
        if !null {
            let fr = l.sym::<FnFreeCCtx>("ZSTD_freeCDict");
            unsafe { fr(p) };
        }
        null
    });
    // ZSTD_initCStream_usingDict with a corrupt dictionary surfaces
    // dictionaryCreation_failed / dictionary_corrupted at init or at flush.
    let r = diff("initCStream_usingDict(corrupt)", |l| {
        let zcs = Ctx::cstream(l);
        let f = l.sym::<FnInitCStreamUsingDict>("ZSTD_initCStream_usingDict");
        let a = unsafe { f(zcs.ptr, bad.as_ptr() as *const c_void, bad.len(), 3) };
        let (rs, _) = stream_out(l, zcs.ptr, &payload(1000), 4096, 4096);
        (res(l, a), rs)
    });
    let _ = r;

    // (60) the setters are only legal in the init stage: calling them mid-frame
    // must report stage_wrong.
    let plain = c_compress(&payload(200_000), 3);
    for (tag, apply) in [
        ("DCtx_loadDictionary", 0u8),
        ("DCtx_refDDict", 1),
        ("DCtx_refPrefix", 2),
    ] {
        let r = diff(&format!("mid-frame {tag}"), |l| {
            let dctx = Ctx::dctx(l);
            let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut obuf = vec![0u8; 1024];
            let mut input = ZSTD_inBuffer {
                src: plain.as_ptr() as *const c_void,
                size: 64.min(plain.len()),
                pos: 0,
            };
            let mut output = ZSTD_outBuffer {
                dst: obuf.as_mut_ptr() as *mut c_void,
                size: 1024,
                pos: 0,
            };
            let first = unsafe { ds(dctx.ptr, &mut output, &mut input) };
            let st = unsafe {
                match apply {
                    0 => {
                        let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
                        f(dctx.ptr, tr.as_ptr() as *const c_void, tr.len())
                    }
                    1 => {
                        let f = l.sym::<FnRefPtr>("ZSTD_DCtx_refDDict");
                        f(dctx.ptr, std::ptr::null())
                    }
                    _ => {
                        let f = l.sym::<FnLoadDict>("ZSTD_DCtx_refPrefix");
                        f(dctx.ptr, tr.as_ptr() as *const c_void, tr.len())
                    }
                }
            };
            (res(l, first), res(l, st))
        });
        expect_err(&format!("mid-frame {tag}"), &r.1, E_stage_wrong);
    }
    // The same on the compressor side.
    let r = diff("mid-frame CCtx_loadDictionary", |l| {
        let zcs = Ctx::cstream(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        unsafe { set(zcs.ptr, ZSTD_c_compressionLevel, 3) };
        let big = payload(200_000);
        let cs = l.sym::<FnDecompressStream>("ZSTD_compressStream");
        let mut obuf = vec![0u8; 512];
        let mut input = ZSTD_inBuffer {
            src: big.as_ptr() as *const c_void,
            size: big.len(),
            pos: 0,
        };
        let mut output = ZSTD_outBuffer {
            dst: obuf.as_mut_ptr() as *mut c_void,
            size: 512,
            pos: 0,
        };
        let first = unsafe { cs(zcs.ptr, &mut output, &mut input) };
        let f = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let st = unsafe { f(zcs.ptr, tr.as_ptr() as *const c_void, tr.len()) };
        let g = l.sym::<FnRefPtr>("ZSTD_CCtx_refCDict");
        let st2 = unsafe { g(zcs.ptr, std::ptr::null()) };
        let h = l.sym::<FnLoadDict>("ZSTD_CCtx_refPrefix");
        let st3 = unsafe { h(zcs.ptr, tr.as_ptr() as *const c_void, tr.len()) };
        (res(l, first), res(l, st), res(l, st2), res(l, st3))
    });
    expect_err("mid-frame CCtx_loadDictionary", &r.1, E_stage_wrong);
    expect_err("mid-frame CCtx_refCDict", &r.2, E_stage_wrong);
    expect_err("mid-frame CCtx_refPrefix", &r.3, E_stage_wrong);

    // Out-of-range dictContentType through both loadDictionary_advanced and
    // refPrefix_advanced, on both sides.
    for &dct in &[3i32, -1, 999] {
        for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef, 2, -1] {
            diff(&format!("loadDictionary_advanced(dct {dct}, dlm {dlm})"), |l| {
                let cctx = Ctx::cctx(l);
                let dctx = Ctx::dctx(l);
                let f = l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
                let g = l.sym::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
                let a = unsafe {
                    f(cctx.ptr, tr.as_ptr() as *const c_void, tr.len(), dlm, dct)
                };
                let b = unsafe {
                    g(dctx.ptr, tr.as_ptr() as *const c_void, tr.len(), dlm, dct)
                };
                let h = l.sym::<FnCompress2>("ZSTD_compress2");
                let cap = compress_bound(l, 1000) + 64;
                let s = payload(1000);
                let mut dst = vec![0xCDu8; cap];
                let k = unsafe {
                    h(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        s.as_ptr() as *const c_void,
                        s.len(),
                    )
                };
                let r = res(l, k);
                if let R::Ok(m) = r {
                    dst.truncate(m);
                }
                (res(l, a), res(l, b), r, Blob(dst))
            });
        }
        diff(&format!("refPrefix_advanced(dct {dct})"), |l| {
            let cctx = Ctx::cctx(l);
            let dctx = Ctx::dctx(l);
            let f = l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
            let g = l.sym::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
            let a = unsafe { f(cctx.ptr, tr.as_ptr() as *const c_void, tr.len(), dct) };
            let b = unsafe { g(dctx.ptr, tr.as_ptr() as *const c_void, tr.len(), dct) };
            let h = l.sym::<FnCompress2>("ZSTD_compress2");
            let cap = compress_bound(l, 1000) + 64;
            let s = payload(1000);
            let mut dst = vec![0xCDu8; cap];
            let k = unsafe {
                h(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    s.as_ptr() as *const c_void,
                    s.len(),
                )
            };
            let r = res(l, k);
            if let R::Ok(m) = r {
                dst.truncate(m);
            }
            (res(l, a), res(l, b), r, Blob(dst))
        });
    }

    // 7-byte and NULL dictionaries through every loader.
    for (tag, d) in [("null", &[][..]), ("7bytes", &tr[..7]), ("8bytes", &tr[..8])] {
        for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
            diff(&format!("tiny dict {tag}, dct {dct}"), |l| {
                let cctx = Ctx::cctx(l);
                let dctx = Ctx::dctx(l);
                let f = l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
                let g = l.sym::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
                let dp = if d.is_empty() {
                    std::ptr::null()
                } else {
                    d.as_ptr() as *const c_void
                };
                let a = unsafe { f(cctx.ptr, dp, d.len(), ZSTD_dlm_byCopy, dct) };
                let b = unsafe { g(dctx.ptr, dp, d.len(), ZSTD_dlm_byCopy, dct) };
                let h = l.sym::<FnCompress2>("ZSTD_compress2");
                let s = payload(1000);
                let cap = compress_bound(l, s.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                let k = unsafe {
                    h(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        s.as_ptr() as *const c_void,
                        s.len(),
                    )
                };
                let r = res(l, k);
                if let R::Ok(m) = r {
                    dst.truncate(m);
                }
                (res(l, a), res(l, b), r, Blob(dst))
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Remaining dictionary rows
// ---------------------------------------------------------------------------

/// `ZSTD_getDictID_fromFrame` corner cases, the cold-vs-warm DDict path
/// (`ddictIsCold`), the `dstCapacity` sweep for the dictionary compressors,
/// `ZSTD_c_forceMaxWindow` combined with `refPrefix`, and
/// `ZSTD_c_deterministicRefPrefix` over a contiguous and a non-contiguous
/// prefix.
#[test]
fn t_dict_misc_rows() {
    covers(&["CFG:78", "CFG:92", "CFG:98", "CFG:125", "CFG:138"]);
    let tr = trained_dict(4096);
    let src = payload(20000);

    // ---- (78) ZSTD_getDictID_fromFrame ----
    let idf = |l: &Lib, b: &[u8]| -> c_uint {
        let f = l.sym::<FnIdFromBuf>("ZSTD_getDictID_fromFrame");
        unsafe { f(b.as_ptr() as *const c_void, b.len()) }
    };
    // A frame with no dictionary.
    let plain = c_compress(&src, 3);
    diff("getDictID_fromFrame(no dict)", |l| idf(l, &plain));
    for n in 0..=17usize {
        diff(&format!("getDictID_fromFrame(no dict, trunc {n})"), |l| {
            idf(l, &plain[..n.min(plain.len())])
        });
    }
    for id in [1u32, 256, 65536, 0xFFFF_FFFF] {
        let d = dict_with_id(id);
        let frame = {
            let (cr, b, _, _) = diff("fixture: dictID frame", |l| {
                usingdict_roundtrip(l, &d, 3, &src)
            });
            expect_ok("fixture dictID frame", &cr);
            b.0
        };
        assert_eq!(
            diff(&format!("getDictID_fromFrame({id})"), |l| idf(l, &frame)),
            id
        );
        for n in 0..=17usize {
            diff(&format!("getDictID_fromFrame({id}, trunc {n})"), |l| {
                idf(l, &frame[..n.min(frame.len())])
            });
        }
    }
    // A skippable frame and 4 bytes of garbage.
    {
        let mut sk = Vec::new();
        sk.extend_from_slice(&ZSTD_MAGIC_SKIPPABLE_START.to_le_bytes());
        sk.extend_from_slice(&8u32.to_le_bytes());
        sk.extend_from_slice(&[0u8; 8]);
        diff("getDictID_fromFrame(skippable)", |l| idf(l, &sk));
        diff("getDictID_fromFrame(garbage)", |l| idf(l, &[0xDE, 0xAD, 0xBE, 0xEF]));
        diff("getDictID_fromFrame(empty)", |l| idf(l, &[]));
    }

    // ---- (92) cold vs warm DDict ----
    // `ddictIsCold` is set when the DDict was not the one used for the previous
    // frame, which switches `ZSTD_decompressBegin_usingDDict` between
    // prefetching the dictionary tables and reusing them. Two frames in a row
    // through the same DCtx therefore exercise both states. The frames span a
    // range of nbSeq / litSize shapes by construction (the exact nbSeq is not
    // directly controllable through the public API).
    let shapes: [(&str, Vec<u8>); 6] = [
        ("empty", payload(0)),
        ("tiny", payload(1)),
        ("random700", corpus(Corpus::Random, 700, 0x7024_0301)),
        ("random769", corpus(Corpus::Random, 769, 0x7024_0302)),
        ("text5000", payload(5000)),
        ("zeros40000", corpus(Corpus::Zeros, 40000, 0x7024_0303)),
    ];
    for (tag, s) in &shapes {
        let frame = {
            let (cr, b, _, _) = diff("fixture: cold/warm frame", |l| {
                usingdict_roundtrip(l, &tr, 3, s)
            });
            expect_ok("fixture cold/warm", &cr);
            b.0
        };
        let (rs, outs) = diff(&format!("cold/warm DDict({tag})"), |l| {
            let ddc = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let dd = unsafe { ddc(tr.as_ptr() as *const c_void, tr.len()) };
            assert!(!dd.is_null());
            let dd = Ctx::from_raw(l, dd, "ZSTD_freeDDict");
            let dctx = Ctx::dctx(l);
            let g = l.sym::<FnDecompressUsingDDict>("ZSTD_decompress_usingDDict");
            let mut rs = Vec::new();
            let mut outs = Vec::new();
            // First call: the DDict is cold. Second and third: warm.
            for _ in 0..3 {
                let mut out = vec![0xABu8; s.len() + 64];
                let n = unsafe {
                    g(
                        dctx.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dd.ptr,
                    )
                };
                let r = res(l, n);
                if let R::Ok(k) = r {
                    out.truncate(k);
                }
                rs.push(r);
                outs.push(Blob(out));
            }
            (rs, outs)
        });
        for r in &rs {
            expect_ok("cold/warm", r);
        }
        for o in &outs {
            assert_eq!(o.0, *s);
        }
    }

    // ---- (98) dstCapacity sweep for the dictionary compressors ----
    for &n in &[0usize, 1, 6, 100, 300_000] {
        let s = payload(n);
        for &lvl in &[1i32, 19] {
            if n > 100_000 && lvl == 19 {
                continue;
            }
            let bound = compress_bound(&pair().c, n);
            let exact = {
                let (r, b) = diff_bytes(&format!("exact cSize(src {n}, lvl {lvl})"), |l| {
                    let cctx = Ctx::cctx(l);
                    let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
                    let mut dst = vec![0xCDu8; bound + 64];
                    let cap = dst.len();
                    let k = unsafe {
                        f(
                            cctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            s.as_ptr() as *const c_void,
                            s.len(),
                            tr.as_ptr() as *const c_void,
                            tr.len(),
                            lvl,
                        )
                    };
                    let r = res(l, k);
                    if let R::Ok(m) = r {
                        dst.truncate(m);
                    }
                    (r, Blob(dst))
                });
                let _ = b;
                expect_ok("exact cSize", &r)
            };
            let mut caps = vec![0usize, 1, 2, 3, 5, 6, 17, 18, 19, bound, bound * 2];
            if exact > 0 {
                caps.push(exact - 1);
                caps.push(exact);
            }
            for cap in caps {
                diff_bytes(
                    &format!("usingDict(src {n}, lvl {lvl}, cap {cap})"),
                    |l| {
                        let cctx = Ctx::cctx(l);
                        let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        let k = unsafe {
                            f(
                                cctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                s.as_ptr() as *const c_void,
                                s.len(),
                                tr.as_ptr() as *const c_void,
                                tr.len(),
                                lvl,
                            )
                        };
                        (res(l, k), Blob(dst))
                    },
                );
                diff_bytes(
                    &format!("usingCDict(src {n}, lvl {lvl}, cap {cap})"),
                    |l| {
                        let cctx = Ctx::cctx(l);
                        let c = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                        let cd = unsafe { c(tr.as_ptr() as *const c_void, tr.len(), lvl) };
                        assert!(!cd.is_null());
                        let cd = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                        let f = l.sym::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        let k = unsafe {
                            f(
                                cctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                s.as_ptr() as *const c_void,
                                s.len(),
                                cd.ptr,
                            )
                        };
                        (res(l, k), Blob(dst))
                    },
                );
            }
        }
    }

    // ---- (125) ZSTD_c_forceMaxWindow with a refPrefix ----
    // The prefix is 64 KB while windowLog is 17, so the prefix does not fit the
    // window; forceMaxWindow changes how the window base is set up.
    let pref = raw_dict(65536);
    let big = payload(200_000);
    for fmw in [0i32, 1] {
        let (rs, frames) = diff(&format!("forceMaxWindow({fmw}) + refPrefix"), |l| {
            let cctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let mut rs = Vec::new();
            unsafe {
                rs.push(res(l, set(cctx.ptr, ZSTD_c_compressionLevel, 6)));
                rs.push(res(l, set(cctx.ptr, ZSTD_c_windowLog, 17)));
                rs.push(res(l, set(cctx.ptr, ZSTD_c_forceMaxWindow, fmw)));
            }
            let f = l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced");
            rs.push(res(l, unsafe {
                f(
                    cctx.ptr,
                    pref.as_ptr() as *const c_void,
                    pref.len(),
                    ZSTD_dct_rawContent,
                )
            }));
            let g = l.sym::<FnCompress2>("ZSTD_compress2");
            let cap = compress_bound(l, big.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let k = unsafe {
                g(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    big.as_ptr() as *const c_void,
                    big.len(),
                )
            };
            let r = res(l, k);
            if let R::Ok(m) = r {
                dst.truncate(m);
            }
            rs.push(r);
            (rs, vec![Blob(dst)])
        });
        for r in &rs {
            expect_ok("forceMaxWindow", r);
        }
        let raws = vec![frames[0].0.clone()];
        let (_, outs) = diff(&format!("forceMaxWindow({fmw}) decode"), |l| {
            three_frames_d(l, Attach::RefPrefix, &pref, &raws, big.len())
        });
        assert_eq!(outs[0].0, big);
    }

    // ---- (138) ZSTD_c_deterministicRefPrefix ----
    // One buffer holding prefix||src (contiguous), and two separate buffers.
    // Both are allocated *outside* the closure so the C and the Rust library see
    // the identical addresses and the contiguity decision is reproducible.
    let contig = payload(65536 + 65536);
    let sep_prefix = contig[..65536].to_vec();
    let sep_src = contig[65536..].to_vec();
    for drp in [0i32, 1] {
        for (tag, p, s) in [
            ("contiguous", &contig[..65536], &contig[65536..]),
            ("separate", &sep_prefix[..], &sep_src[..]),
        ] {
            let (rs, frames) = diff(
                &format!("deterministicRefPrefix({drp}, {tag})"),
                |l| {
                    let cctx = Ctx::cctx(l);
                    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                    let mut rs = Vec::new();
                    unsafe {
                        rs.push(res(l, set(cctx.ptr, ZSTD_c_compressionLevel, 5)));
                        rs.push(res(
                            l,
                            set(cctx.ptr, ZSTD_c_deterministicRefPrefix, drp),
                        ));
                    }
                    let f = l.sym::<FnLoadDict>("ZSTD_CCtx_refPrefix");
                    rs.push(res(l, unsafe {
                        f(cctx.ptr, p.as_ptr() as *const c_void, p.len())
                    }));
                    let g = l.sym::<FnCompress2>("ZSTD_compress2");
                    let cap = compress_bound(l, s.len()) + 64;
                    let mut dst = vec![0xCDu8; cap];
                    let k = unsafe {
                        g(
                            cctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            s.as_ptr() as *const c_void,
                            s.len(),
                        )
                    };
                    let r = res(l, k);
                    if let R::Ok(m) = r {
                        dst.truncate(m);
                    }
                    rs.push(r);
                    (rs, vec![Blob(dst)])
                },
            );
            for r in &rs {
                expect_ok("deterministicRefPrefix", r);
            }
            let raws = vec![frames[0].0.clone()];
            let (_, outs) = diff(
                &format!("deterministicRefPrefix({drp}, {tag}) decode"),
                |l| three_frames_d(l, Attach::RefPrefix, p, &raws, s.len()),
            );
            assert_eq!(outs[0].0, *s);
        }
    }
}
