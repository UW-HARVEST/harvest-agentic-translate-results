//! Phase C — ERROR-PATH coverage for the dictionary builder and `ZSTD_DDict`.
//!
//! ERRORS.md rows covered here:
//!   * `dictBuilder/zdict.c`      rows 510..526  (11 sites)
//!   * `dictBuilder/cover.c`      rows 474..491  (15 sites)
//!   * `dictBuilder/fastcover.c`  rows 494..509  (15 sites)
//!   * `dictBuilder/divsufsort.c` rows 492..493  (2 `return -1` sites)
//!   * `decompress/zstd_ddict.c`  rows 305..314  (10 sites)
//!
//! Every rejection is reached by *constructing* the exact invalid input and
//! asserting the C and the Rust `.so` return the SAME error **code** (compared
//! through `ZSTD_getErrorCode` + `ZSTD_getErrorName` + `ZDICT_isError` +
//! `ZDICT_getErrorName`), plus the full destination buffer and the full in/out
//! `parameters` struct.
//!
//! `notificationLevel` is kept at 0 for the COVER / FASTCOVER families: those
//! two trainers copy it into the process-global `g_displayLevel`, which leaks
//! across libtest's parallel threads.
//!
//! ---------------------------------------------------------------------------
//! Sites that cannot be reached, with the C evidence (see the FINAL REPORT):
//!
//! * every `ERROR(memory_allocation)` in `dictBuilder/**` (rows 478, 482, 490,
//!   491, 497, 498, 508, 509, 511, 512, 515, 520, 526) — these fire only when
//!   `malloc`/`calloc` returns NULL, and the dictBuilder API offers no custom
//!   allocator hook (unlike `ZSTD_createDDict_advanced`, whose
//!   `memory_allocation` path *is* covered below via a failing `ZSTD_customMem`).
//!   `POOL_create()` additionally *cannot* fail in this build: `common/pool.c`
//!   L326-336 (`#else /* ZSTD_MULTITHREAD not defined */`) unconditionally
//!   returns `&g_poolCtx`, so `nbThreads > 1` never yields `memory_allocation`
//!   (a differential probe with `nbThreads > 1` is included instead).
//! * row 477 / row 496 (`nbTestSamples < 1` in `COVER_ctx_init` /
//!   `FASTCOVER_ctx_init`): `nbTestSamples = nbSamples - nbTrainSamples` with
//!   `nbTrainSamples = (unsigned)((double)nbSamples * splitPoint)` and the whole
//!   expression guarded by `splitPoint < 1.0`.  For every `splitPoint` strictly
//!   below 1.0 the exact product `N*splitPoint` is more than half an ULP below
//!   `N`, so it never rounds up to `N` and `nbTrainSamples < nbSamples` always
//!   holds; `nbTrainSamples < 5` is checked first for the small-splitPoint case.
//! * row 519 (`hSize + minContentSize > dictBufferCapacity` in
//!   `ZDICT_finalizeDictionary`): needs an entropy header of >= 249 bytes while
//!   `HBUFFSIZE` caps it at 256 and `dictBufferCapacity >= 256` is enforced one
//!   line earlier.  A ~5000-configuration randomized search over sample
//!   contents, sample counts and compression levels driven against the C build
//!   never produced a header larger than **198** bytes.  The *same* statement
//!   family in `ZDICT_analyzeEntropy` (row 516) IS reached, through
//!   `ZDICT_addEntropyTablesFromBuffer`, where `maxDstSize` is caller-controlled.
//! * `samplesSizes == NULL` together with `nbSamples > 0` is an unchecked
//!   precondition in every trainer (`COVER_sum` / `ZDICT_totalSampleSize`
//!   dereference it immediately), so only the `nbSamples == 0` + NULL shape is
//!   exercised; `samplesBuffer == NULL` and `dictBuffer == NULL` are exercised
//!   wherever the C returns before touching them.
//! * rows 523 / 525 (`ERROR(GENERIC)` "should never happen" in
//!   `ZDICT_trainFromBuffer_unsafe_legacy`) and row 513 (`divsufsort` failure in
//!   `ZDICT_trainBuffer_legacy`): unreachable by construction — `divsufsort`
//!   only fails with `-2` on `malloc` failure here (`T`/`SA` are non-NULL and
//!   `n >= 0`), and the two GENERIC guards are dominated by earlier invariants.
//! * row 522 (`samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE` inside the *static*
//!   `ZDICT_trainFromBuffer_unsafe_legacy`): the only exported entry point,
//!   `ZDICT_trainFromBuffer_legacy` (zdict.c L1091), already `return 0;` for that
//!   condition before calling it, so the inner check is dead.  The `return 0`
//!   sentinel is covered instead.
#![allow(dead_code)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------- structs
// Layouts copied verbatim from c_src/src/include/zdict.h and
// c_src/src/dictBuilder/cover.h.  The C build has ZSTD_MULTITHREAD *undefined*
// (no pthread_* relocations in c_src/build/libzstd.so), so
// ZSTD_pthread_mutex_t / ZSTD_pthread_cond_t are plain `int`.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_params_t {
    compressionLevel: c_int,
    notificationLevel: c_uint,
    dictID: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
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
#[derive(Clone, Copy, Debug, Default)]
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
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct ZDICT_legacy_params_t {
    selectivityLevel: c_uint,
    zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct COVER_dictSelection_t {
    dictContent: *mut u8,
    dictSize: usize,
    totalCompressedSize: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct COVER_best_t {
    mutex: c_int,
    cond: c_int,
    liveJobs: usize,
    dict: *mut c_void,
    dictSize: usize,
    parameters: ZDICT_cover_params_t,
    compressedSize: usize,
}

// Guard the ABI assumptions (`python3 ctypes` on the C build reports 12/48/56/16/24/88).
const _: () = assert!(std::mem::size_of::<ZDICT_params_t>() == 12);
const _: () = assert!(std::mem::size_of::<ZDICT_cover_params_t>() == 48);
const _: () = assert!(std::mem::size_of::<ZDICT_fastCover_params_t>() == 56);
const _: () = assert!(std::mem::size_of::<ZDICT_legacy_params_t>() == 16);
const _: () = assert!(std::mem::size_of::<COVER_dictSelection_t>() == 24);
const _: () = assert!(std::mem::size_of::<COVER_best_t>() == 88);

const ZDICT_DICTSIZE_MIN: usize = 256;
const ZDICT_CONTENTSIZE_MIN: usize = 128;
/// `ZDICT_CONTENTSIZE_MIN * MINRATIO` (zdict.c L15-17).
const ZDICT_MIN_SAMPLES_SIZE: usize = 128 * 4;
const FASTCOVER_MAX_F: c_uint = 31;
const FASTCOVER_MAX_ACCEL: c_uint = 10;

// ---------------------------------------------------------------- fn types

type FnTrain =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize;
type FnTrainCover = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_cover_params_t,
) -> usize;
type FnOptCover = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> usize;
type FnTrainFast = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_fastCover_params_t,
) -> usize;
type FnOptFast = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> usize;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_legacy_params_t,
) -> usize;
type FnFinalize = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const usize,
    c_uint,
    ZDICT_params_t,
) -> usize;
type FnAddEntropy =
    unsafe extern "C" fn(*mut c_void, usize, usize, *const c_void, *const usize, c_uint) -> usize;
type FnGetDictID = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnGetHdrSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

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
type FnBestVoid = unsafe extern "C" fn(*mut COVER_best_t);
type FnBestFinish =
    unsafe extern "C" fn(*mut COVER_best_t, ZDICT_cover_params_t, COVER_dictSelection_t);
type FnDsIsError = unsafe extern "C" fn(COVER_dictSelection_t) -> c_uint;
type FnDsFree = unsafe extern "C" fn(COVER_dictSelection_t);
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

type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut c_int, c_int, c_int) -> c_int;
type FnDivbwt = unsafe extern "C" fn(
    *const u8,
    *mut u8,
    *mut c_int,
    c_int,
    *mut u8,
    *mut c_int,
    c_int,
) -> c_int;

type FnCreateDDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, usize, c_int, c_int, ZSTD_customMem) -> *mut c_void;
type FnInitStaticDDict =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int, c_int) -> *const c_void;
type FnDDictContent = unsafe extern "C" fn(*const c_void) -> *const c_void;
type FnDDictSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnCopyDDictParams = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnDDictU32 = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnBeginUsingDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnEstimateDDictSize = unsafe extern "C" fn(usize, c_int) -> usize;
type FnPtrToSize = unsafe extern "C" fn(*mut c_void) -> usize;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int, c_int) -> usize;
type FnGetDictIDFromDict = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnDecompressDCtxSimple =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

// ---------------------------------------------------------------- comparisons

/// Compare the *error code*, not just "both failed".
#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        let (zn, znr) = duo::<FnErrName>("ZDICT_getErrorName");
        let (zi, zir) = duo::<FnIsError>("ZDICT_isError");
        if c != r {
            panic!(
                "{what}: C returned {c:#x} (code {} = {}), Rust returned {r:#x} (code {} = {})",
                gcc(c),
                cstr(nc(c)),
                gcr(r),
                cstr(nr(r))
            );
        }
        assert_eq!(gcc(c), gcr(r), "{what}: ZSTD_getErrorCode mismatch");
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: ZSTD_getErrorName mismatch");
        assert_eq!(cstr(zn(c)), cstr(znr(r)), "{what}: ZDICT_getErrorName mismatch");
        assert_eq!(zi(c), zir(r), "{what}: ZDICT_isError mismatch");
    }
}

/// Assert `n` is exactly the error whose name is `want` (C is ground truth, so
/// this pins the *site*, not merely "an error").
#[track_caller]
fn expect_err(what: &str, n: usize, want: &str) {
    unsafe {
        let (nc, _) = duo::<FnErrName>("ZSTD_getErrorName");
        assert!(is_err(n), "{what}: expected {want}, got success ({n})");
        assert_eq!(cstr(nc(n)), want, "{what}: wrong error");
    }
}

/// `f64` fields are compared bit-for-bit so a NaN `splitPoint` round-trip is
/// still checked exactly.
#[track_caller]
fn eq_cover_params(what: &str, a: ZDICT_cover_params_t, b: ZDICT_cover_params_t) {
    assert_eq!(a.k, b.k, "{what}: k");
    assert_eq!(a.d, b.d, "{what}: d");
    assert_eq!(a.steps, b.steps, "{what}: steps");
    assert_eq!(a.nbThreads, b.nbThreads, "{what}: nbThreads");
    assert_eq!(a.splitPoint.to_bits(), b.splitPoint.to_bits(), "{what}: splitPoint");
    assert_eq!(a.shrinkDict, b.shrinkDict, "{what}: shrinkDict");
    assert_eq!(
        a.shrinkDictMaxRegression, b.shrinkDictMaxRegression,
        "{what}: shrinkDictMaxRegression"
    );
    assert_eq!(a.zParams, b.zParams, "{what}: zParams");
}

#[track_caller]
fn eq_fast_params(what: &str, a: ZDICT_fastCover_params_t, b: ZDICT_fastCover_params_t) {
    assert_eq!(a.k, b.k, "{what}: k");
    assert_eq!(a.d, b.d, "{what}: d");
    assert_eq!(a.f, b.f, "{what}: f");
    assert_eq!(a.steps, b.steps, "{what}: steps");
    assert_eq!(a.nbThreads, b.nbThreads, "{what}: nbThreads");
    assert_eq!(a.splitPoint.to_bits(), b.splitPoint.to_bits(), "{what}: splitPoint");
    assert_eq!(a.accel, b.accel, "{what}: accel");
    assert_eq!(a.shrinkDict, b.shrinkDict, "{what}: shrinkDict");
    assert_eq!(
        a.shrinkDictMaxRegression, b.shrinkDictMaxRegression,
        "{what}: shrinkDictMaxRegression"
    );
    assert_eq!(a.zParams, b.zParams, "{what}: zParams");
}

// ---------------------------------------------------------------- corpora

/// Flat concatenated sample buffer + its `samplesSizes` array.
struct Corpus {
    buf: Vec<u8>,
    sizes: Vec<usize>,
}

impl Corpus {
    fn new(class: usize, sizes: Vec<usize>, seed: u64) -> Corpus {
        let maxs = sizes.iter().copied().max().unwrap_or(0);
        let master = gen_class(class, maxs * 2 + 256, seed);
        let mut buf = Vec::with_capacity(sizes.iter().sum());
        for (i, &s) in sizes.iter().enumerate() {
            let span = master.len() - s;
            let off = if span == 0 { 0 } else { (i * 4099 + 17) % span };
            buf.extend_from_slice(&master[off..off + s]);
        }
        Corpus { buf, sizes }
    }
    fn uniform(class: usize, nb: usize, size: usize, seed: u64) -> Corpus {
        Corpus::new(class, vec![size; nb], seed)
    }
    fn nb(&self) -> c_uint {
        self.sizes.len() as c_uint
    }
    fn sp(&self) -> *const c_void {
        self.buf.as_ptr() as *const c_void
    }
    fn szp(&self) -> *const usize {
        self.sizes.as_ptr()
    }
    fn total(&self) -> usize {
        self.buf.len()
    }
    /// `offsets[i]` = start of sample i, `offsets[nb]` = total (COVER convention).
    fn offsets(&self) -> Vec<usize> {
        let mut v = Vec::with_capacity(self.sizes.len() + 1);
        let mut a = 0usize;
        v.push(0usize);
        for &s in &self.sizes {
            a += s;
            v.push(a);
        }
        v
    }
}

fn zparams(level: c_int) -> ZDICT_params_t {
    ZDICT_params_t { compressionLevel: level, notificationLevel: 0, dictID: 0 }
}

/// Valid-looking COVER parameters that pass every check.
fn ok_cover() -> ZDICT_cover_params_t {
    ZDICT_cover_params_t {
        k: 50,
        d: 8,
        steps: 1,
        nbThreads: 1,
        splitPoint: 1.0,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: zparams(3),
    }
}

fn ok_fast() -> ZDICT_fastCover_params_t {
    ZDICT_fastCover_params_t {
        k: 50,
        d: 8,
        f: 15,
        steps: 1,
        nbThreads: 1,
        splitPoint: 1.0,
        accel: 1,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: zparams(3),
    }
}

// ---------------------------------------------------------------- drivers

/// Call a by-value-parameters trainer in both libraries; compare the return
/// code AND the whole destination buffer.  `cap` is what is *passed* to the
/// library; `real` is how many bytes are actually allocated (they differ only
/// for the deliberate "capacity larger than any possible allocation" probes).
#[track_caller]
unsafe fn diff_train_p<P: Copy>(
    what: &str,
    f: (
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint, P) -> usize,
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint, P) -> usize,
    ),
    cap: usize,
    real: usize,
    co: &Corpus,
    p: P,
) -> usize {
    let mut dc = vec![0xA5u8; real.max(1)];
    let mut dr = vec![0xA5u8; real.max(1)];
    let rc = (f.0)(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), p);
    let rr = (f.1)(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), p);
    eqcode(&format!("{what}: return"), rc, rr);
    eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
    rc
}

// =================================================================== zdict.c
// row 510 : ZDICT_getDictHeaderSize -> dictionary_corrupted
// plus the `return 0` sentinels of ZDICT_getDictID (zdict.c L104-105).

#[test]
fn err_zdict_dict_header_probes() {
    unsafe {
        let (idc, idr) = duo::<FnGetDictID>("ZDICT_getDictID");
        let (hc, hr) = duo::<FnGetHdrSize>("ZDICT_getDictHeaderSize");

        let magic = ZSTD_MAGIC_DICTIONARY.to_le_bytes();
        let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

        // too short: 0..=8 bytes, both with and without the right magic
        for n in 0..=8usize {
            cases.push((format!("zeros[{n}]"), vec![0u8; n]));
            let mut v = magic.to_vec();
            v.extend_from_slice(&[0x11u8; 8]);
            v.truncate(n);
            cases.push((format!("magic-prefix[{n}]"), v));
        }
        // right length, wrong magic
        for bad in [0u32, 0xFFFF_FFFF, ZSTD_MAGICNUMBER, ZSTD_MAGIC_DICTIONARY ^ 1] {
            let mut v = bad.to_le_bytes().to_vec();
            v.extend_from_slice(&[0x55u8; 64]);
            cases.push((format!("wrong-magic {bad:#x}"), v));
        }
        // right magic, corrupt header body of every length
        for n in [9usize, 10, 16, 32, 64, 128, 256, 1024] {
            let mut v = magic.to_vec();
            v.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
            let mut rng = Rng::new(0xC0FFEE ^ n as u64);
            let extra = rng.bytes(n.saturating_sub(8));
            v.extend_from_slice(&extra);
            v.truncate(n.max(9));
            cases.push((format!("magic+random[{n}]"), v));
            // all-zero body: HUF header of 0 is invalid too
            let mut z = magic.to_vec();
            z.extend_from_slice(&1u32.to_le_bytes());
            z.resize(n.max(9), 0);
            cases.push((format!("magic+zeros[{n}]"), z));
        }

        let mut corrupted = 0usize;
        for (name, d) in &cases {
            let p = d.as_ptr() as *const c_void;
            eqv(&format!("{name}: ZDICT_getDictID"), idc(p, d.len()), idr(p, d.len()));
            let a = hc(p, d.len());
            let b = hr(p, d.len());
            eqcode(&format!("{name}: ZDICT_getDictHeaderSize"), a, b);
            if d.len() <= 8 || d[..4] != magic {
                // row 510
                expect_err(
                    &format!("{name}: ZDICT_getDictHeaderSize"),
                    a,
                    "Dictionary is corrupted",
                );
                corrupted += 1;
            }
        }
        assert!(corrupted >= 20, "row 510 not exercised enough ({corrupted})");

        // NULL + size 0 is legal for getDictID (`dictSize < 8` short-circuits)
        // and for getDictHeaderSize (`dictSize <= 8` short-circuits).
        eqv("null getDictID", idc(std::ptr::null(), 0), idr(std::ptr::null(), 0));
        let a = hc(std::ptr::null(), 0);
        let b = hr(std::ptr::null(), 0);
        eqcode("null getDictHeaderSize", a, b);
        expect_err("null getDictHeaderSize", a, "Dictionary is corrupted");
    }
}

// rows 517, 518 : ZDICT_finalizeDictionary -> dstSize_tooSmall

#[test]
fn err_zdict_finalize_dictionary() {
    unsafe {
        let (fc, fr) = duo::<FnFinalize>("ZDICT_finalizeDictionary");
        let co = Corpus::uniform(4, 8, 512, 0xF1A1);
        let content = gen_class(5, 1024, 0x9E11);

        let mut hit517 = 0usize;
        let mut hit518 = 0usize;

        // ---- row 517: dictBufferCapacity < dictContentSize
        // (checked before dictBufferCapacity < ZDICT_DICTSIZE_MIN, so use a
        // capacity that is >= 256 to isolate the site)
        for &(cap, dcs) in &[
            (256usize, 257usize),
            (256, 1024),
            (512, 1024),
            (1024, 1025),
            (300, usize::MAX / 2),
        ] {
            let dcs_eff = dcs.min(content.len());
            let (cap, dcs) = if dcs > content.len() && dcs != usize::MAX / 2 {
                (cap, dcs_eff)
            } else {
                (cap, dcs)
            };
            if cap >= dcs {
                continue;
            }
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let what = format!("finalize row517 cap={cap} dictContentSize={dcs}");
            let rc = fc(
                dc.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                dcs,
                co.sp(),
                co.szp(),
                co.nb(),
                zparams(3),
            );
            let rr = fr(
                dr.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                dcs,
                co.sp(),
                co.szp(),
                co.nb(),
                zparams(3),
            );
            eqcode(&format!("{what}: return"), rc, rr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            expect_err(&what, rc, "Destination buffer is too small");
            hit517 += 1;
        }

        // ---- row 518: dictBufferCapacity < ZDICT_DICTSIZE_MIN
        for cap in [0usize, 1, 8, 100, 128, 255] {
            for dcs in [0usize, 1, 8, 64] {
                if cap < dcs {
                    continue; // that is row 517, already covered
                }
                let mut dc = vec![0xA5u8; cap.max(1)];
                let mut dr = vec![0xA5u8; cap.max(1)];
                let what = format!("finalize row518 cap={cap} dictContentSize={dcs}");
                // dictBuffer may even be NULL here: both checks precede any write
                let (pc, pr) = if cap == 0 {
                    (std::ptr::null_mut(), std::ptr::null_mut())
                } else {
                    (dc.as_mut_ptr() as *mut c_void, dr.as_mut_ptr() as *mut c_void)
                };
                let cptr = if dcs == 0 {
                    std::ptr::null()
                } else {
                    content.as_ptr() as *const c_void
                };
                let rc = fc(pc, cap, cptr, dcs, co.sp(), co.szp(), co.nb(), zparams(3));
                let rr = fr(pr, cap, cptr, dcs, co.sp(), co.szp(), co.nb(), zparams(3));
                eqcode(&format!("{what}: return"), rc, rr);
                eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                expect_err(&what, rc, "Destination buffer is too small");
                hit518 += 1;
            }
        }
        assert!(hit517 >= 3 && hit518 >= 6, "row517={hit517} row518={hit518}");

        // ---- extreme (but in-contract) ZDICT_params_t on the early-out path.
        // `notificationLevel` is only read after these two checks, so even
        // absurd values cannot write to stderr here.
        for lvl in [c_int::MIN, -99, -1, 0, 1, 22, 23, c_int::MAX] {
            for nl in [0u32, 1, 4, u32::MAX] {
                for id in [0u32, 1, u32::MAX] {
                    let p = ZDICT_params_t {
                        compressionLevel: lvl,
                        notificationLevel: nl,
                        dictID: id,
                    };
                    let mut dc = vec![0xA5u8; 16];
                    let mut dr = vec![0xA5u8; 16];
                    let what = format!("finalize corrupt-params lvl={lvl} nl={nl} id={id}");
                    let rc = fc(
                        dc.as_mut_ptr() as *mut c_void,
                        16,
                        content.as_ptr() as *const c_void,
                        1024,
                        co.sp(),
                        co.szp(),
                        co.nb(),
                        p,
                    );
                    let rr = fr(
                        dr.as_mut_ptr() as *mut c_void,
                        16,
                        content.as_ptr() as *const c_void,
                        1024,
                        co.sp(),
                        co.szp(),
                        co.nb(),
                        p,
                    );
                    eqcode(&format!("{what}: return"), rc, rr);
                    eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                    expect_err(&what, rc, "Destination buffer is too small");
                }
            }
        }

        // ---- degenerate-but-legal shapes that must behave identically.
        // dictContentSize == 0, nbSamples == 0, samples pointers NULL.
        for &(dcs, nb, nullsamples) in &[
            (0usize, 8u32, false),
            (0, 0, true),
            (8, 0, true),
            (64, 0, true),
            (0, 8, false),
        ] {
            for cap in [256usize, 512, 4096] {
                let mut dc = vec![0xA5u8; cap];
                let mut dr = vec![0xA5u8; cap];
                let (sb, sz) = if nullsamples {
                    (std::ptr::null(), std::ptr::null())
                } else {
                    (co.sp(), co.szp())
                };
                let cptr = if dcs == 0 {
                    std::ptr::null()
                } else {
                    content.as_ptr() as *const c_void
                };
                let what = format!("finalize degenerate dcs={dcs} nb={nb} cap={cap}");
                let rc = fc(dc.as_mut_ptr() as *mut c_void, cap, cptr, dcs, sb, sz, nb, zparams(3));
                let rr = fr(dr.as_mut_ptr() as *mut c_void, cap, cptr, dcs, sb, sz, nb, zparams(3));
                eqcode(&format!("{what}: return"), rc, rr);
                eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            }
        }

        // samples of size 0 mixed into a valid corpus
        let zsizes: Vec<usize> = vec![0, 0, 128, 0, 128, 0, 128, 0];
        let zbuf = gen_class(4, 384, 0x5151);
        for cap in [256usize, 1024] {
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let what = format!("finalize zero-sized-samples cap={cap}");
            let rc = fc(
                dc.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                200,
                zbuf.as_ptr() as *const c_void,
                zsizes.as_ptr(),
                zsizes.len() as c_uint,
                zparams(3),
            );
            let rr = fr(
                dr.as_mut_ptr() as *mut c_void,
                cap,
                content.as_ptr() as *const c_void,
                200,
                zbuf.as_ptr() as *const c_void,
                zsizes.as_ptr(),
                zsizes.len() as c_uint,
                zparams(3),
            );
            eqcode(&format!("{what}: return"), rc, rr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
        }
    }
}

// row 514 : ZDICT_analyzeEntropy -> dictionaryCreation_failed
//           (`offcodeMax > OFFCODE_MAX`, i.e. highbit32(dictContentSize+128KB) > 30)
// row 516 : ZDICT_analyzeEntropy -> dstSize_tooSmall  (`maxDstSize < 12`)
//
// Both are reached through the exported `ZDICT_addEntropyTablesFromBuffer`,
// which forwards `dictBufferCapacity - 8` straight to `maxDstSize` and
// `dictContentSize` straight to `dictBufferSize`.

#[test]
fn err_zdict_analyze_entropy() {
    unsafe {
        let (ac, ar) = duo::<FnAddEntropy>("ZDICT_addEntropyTablesFromBuffer");
        let co = Corpus::uniform(3, 8, 1024, 0x4242);

        // ---- row 514.  `(U32)(dictContentSize + 128 KB)` must have bit 31 set;
        // the check fires before anything dereferences the (huge) content, so a
        // small real buffer is safe.  Values whose low 32 bits do NOT set bit 31
        // would fall through to an out-of-bounds XXH64 in *both* libraries and
        // are deliberately not used.
        let mut hit514 = 0usize;
        for &dcs in &[
            0x7FFF_FFFFusize,
            0x8000_0000,
            0xFFFD_FFFF,
            (1usize << 31) - 131072,
            0x7FFF_FFFF + (1usize << 32),
            0xFFFF_FFFF - 131072 + (1usize << 40),
        ] {
            let hi = ((dcs as u32).wrapping_add(128 * 1024)) as u32;
            if hi < 0x8000_0000 {
                continue; // would proceed to read `dcs` bytes — excluded
            }
            for cap in [4096usize, 512, 260] {
                let mut dc = vec![0xA5u8; cap];
                let mut dr = vec![0xA5u8; cap];
                let what = format!("addEntropy row514 dcs={dcs:#x} cap={cap}");
                let rc = ac(dc.as_mut_ptr() as *mut c_void, dcs, cap, co.sp(), co.szp(), co.nb());
                let rr = ar(dr.as_mut_ptr() as *mut c_void, dcs, cap, co.sp(), co.szp(), co.nb());
                eqcode(&format!("{what}: return"), rc, rr);
                eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                expect_err(&what, rc, "Cannot create Dictionary from provided samples");
                hit514 += 1;
            }
        }
        assert!(hit514 >= 6, "row 514 not reached ({hit514})");

        // ---- row 516.  Measure the exact entropy-header size with a generous
        // capacity, then re-run with `maxDstSize` exactly equal to the size of
        // the four tables: every table still fits, but the trailing 12 rep-offset
        // bytes do not -> `maxDstSize < 12` -> dstSize_tooSmall.
        let mut hit516 = 0usize;
        for (ci, class) in [3usize, 4, 6].iter().enumerate() {
            let co = Corpus::uniform(*class, 8, 1024, 0x515A + ci as u64);
            const DCS: usize = 8;
            let big = 4096usize;
            let mut probe_c = vec![0u8; big];
            let mut probe_r = vec![0u8; big];
            for i in 0..DCS {
                probe_c[big - DCS + i] = 0x5A;
                probe_r[big - DCS + i] = 0x5A;
            }
            let rc =
                ac(probe_c.as_mut_ptr() as *mut c_void, DCS, big, co.sp(), co.szp(), co.nb());
            let rr =
                ar(probe_r.as_mut_ptr() as *mut c_void, DCS, big, co.sp(), co.szp(), co.nb());
            eqcode("addEntropy probe", rc, rr);
            eqbuf("addEntropy probe buffer", &probe_c, &probe_r);
            assert!(!is_err(rc), "addEntropy probe failed: {rc:#x}");
            // rc == hSize + DCS ; hSize == 8 + eSize ; eSize == tables + 12
            let h_size = rc - DCS;
            assert!(h_size > 20, "unexpected hSize {h_size}");
            let tables = h_size - 8 - 12;
            for j in 0..12usize {
                let cap = 8 + tables + j;
                assert!(cap >= DCS);
                let mut dc = vec![0u8; cap];
                let mut dr = vec![0u8; cap];
                for i in 0..DCS {
                    dc[cap - DCS + i] = 0x5A;
                    dr[cap - DCS + i] = 0x5A;
                }
                let what = format!("addEntropy row516 class={class} tables={tables} j={j}");
                let a = ac(dc.as_mut_ptr() as *mut c_void, DCS, cap, co.sp(), co.szp(), co.nb());
                let b = ar(dr.as_mut_ptr() as *mut c_void, DCS, cap, co.sp(), co.szp(), co.nb());
                eqcode(&format!("{what}: return"), a, b);
                eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
                expect_err(&what, a, "Destination buffer is too small");
                hit516 += 1;
            }
            // one more byte and it succeeds again — proves the boundary is exact
            {
                let cap = 8 + tables + 12;
                let mut dc = vec![0u8; cap];
                let mut dr = vec![0u8; cap];
                for i in 0..DCS {
                    dc[cap - DCS + i] = 0x5A;
                    dr[cap - DCS + i] = 0x5A;
                }
                let a = ac(dc.as_mut_ptr() as *mut c_void, DCS, cap, co.sp(), co.szp(), co.nb());
                let b = ar(dr.as_mut_ptr() as *mut c_void, DCS, cap, co.sp(), co.szp(), co.nb());
                eqcode("addEntropy row516 boundary+1", a, b);
                eqbuf("addEntropy row516 boundary+1 buffer", &dc, &dr);
                assert!(!is_err(a), "boundary+1 should succeed, got {a:#x}");
            }
        }
        assert!(hit516 >= 36, "row 516 not reached enough ({hit516})");

        // tiny capacities: the individual table writes fail first (HUF /
        // FSE_writeNCount dstSize_tooSmall) — still must agree exactly.
        for cap in [8usize, 9, 10, 12, 16, 24, 32, 48, 64] {
            let mut dc = vec![0xA5u8; cap];
            let mut dr = vec![0xA5u8; cap];
            let what = format!("addEntropy tiny cap={cap}");
            let a = ac(dc.as_mut_ptr() as *mut c_void, 0, cap, co.sp(), co.szp(), co.nb());
            let b = ar(dr.as_mut_ptr() as *mut c_void, 0, cap, co.sp(), co.szp(), co.nb());
            eqcode(&format!("{what}: return"), a, b);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
        }
    }
}

// rows 521, 524 : ZDICT_trainFromBuffer_unsafe_legacy
// plus the `return 0` sentinel of ZDICT_trainFromBuffer_legacy (row 522's guard)

#[test]
fn err_zdict_train_legacy() {
    unsafe {
        let f = duo::<FnTrainLegacy>("ZDICT_trainFromBuffer_legacy");

        // ---- row 521: maxDictSize < ZDICT_DICTSIZE_MIN, with enough samples
        let co = Corpus::uniform(4, 8, 256, 0x7E51); // total 2048 >= 512
        assert!(co.total() >= ZDICT_MIN_SAMPLES_SIZE);
        let mut hit521 = 0usize;
        for cap in [1usize, 8, 100, 128, 255] {
            for sel in [0u32, 1, 9, 30, 31, u32::MAX] {
                let p = ZDICT_legacy_params_t { selectivityLevel: sel, zParams: zparams(3) };
                let what = format!("legacy row521 cap={cap} sel={sel}");
                let r = diff_train_p(&what, f, cap, cap, &co, p);
                expect_err(&what, r, "Destination buffer is too small");
                hit521 += 1;
            }
        }
        assert!(hit521 >= 20, "row 521 ({hit521})");

        // ---- the `sBuffSize < ZDICT_MIN_SAMPLES_SIZE -> return 0` sentinel
        // (zdict.c L1091), which makes row 522 dead code.
        let mut hit0 = 0usize;
        for total in [0usize, 1, 8, 128, 400, 511] {
            let nb = if total == 0 { 1 } else { (total / 64).max(1) };
            let each = total / nb;
            let sizes: Vec<usize> = {
                let mut v = vec![each; nb];
                let s: usize = v.iter().sum();
                if s < total {
                    v[0] += total - s;
                }
                v
            };
            let co = Corpus::new(4, sizes, 0x11 + total as u64);
            assert_eq!(co.total(), total);
            for cap in [1usize, 255, 256, 4096] {
                let p = ZDICT_legacy_params_t { selectivityLevel: 9, zParams: zparams(3) };
                let what = format!("legacy sentinel total={total} cap={cap}");
                let r = diff_train_p(&what, f, cap, cap, &co, p);
                assert_eq!(r, 0, "{what}: expected the `return 0` sentinel, got {r:#x}");
                hit0 += 1;
            }
        }
        assert!(hit0 >= 20, "legacy `return 0` sentinel ({hit0})");

        // ---- row 524: dictContentSize < ZDICT_CONTENTSIZE_MIN
        // Incompressible, non-repeating samples produce no dictionary segment.
        let mut hit524 = 0usize;
        for class in [3usize, 7] {
            for &(nb, sz) in &[(4usize, 512usize), (8, 256), (16, 128), (2, 1024)] {
                let co = Corpus::uniform(class, nb, sz, 0xBEEF ^ (class as u64) << 8);
                assert!(co.total() >= ZDICT_MIN_SAMPLES_SIZE);
                for cap in [256usize, 1024] {
                    for sel in [1u32, 9, 31] {
                        let p =
                            ZDICT_legacy_params_t { selectivityLevel: sel, zParams: zparams(3) };
                        let what =
                            format!("legacy row524 class={class} nb={nb} sz={sz} cap={cap} sel={sel}");
                        let r = diff_train_p(&what, f, cap, cap, &co, p);
                        if is_err(r) {
                            let (nc, _) = duo::<FnErrName>("ZSTD_getErrorName");
                            if cstr(nc(r)) == "Cannot create Dictionary from provided samples" {
                                hit524 += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(hit524 >= 1, "row 524 (dictionaryCreation_failed) never reached");
    }
}

// =================================================================== cover.c
// rows 479, 480, 481 : ZDICT_trainFromBuffer_cover
// row 474            : COVER_cmp8 `return -1`  (executed by any d<=8 training)

#[test]
fn err_cover_train_rejections() {
    unsafe {
        let f = duo::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let co = Corpus::uniform(4, 8, 512, 0xC0FE);

        // ---- row 479: COVER_checkParameters fails -> parameter_outOfBound.
        // NOTE ZDICT_trainFromBuffer_cover forces `parameters.splitPoint = 1.0`
        // *before* the check, so the splitPoint clause of COVER_checkParameters
        // is dead on this entry point (it is covered through the optimize
        // entry point instead).
        let mut hit479 = 0usize;
        let bad_params: Vec<(String, ZDICT_cover_params_t)> = vec![
            ("d=0".into(), ZDICT_cover_params_t { d: 0, ..ok_cover() }),
            ("k=0".into(), ZDICT_cover_params_t { k: 0, ..ok_cover() }),
            ("k=0,d=0".into(), ZDICT_cover_params_t { k: 0, d: 0, ..ok_cover() }),
            ("d>k".into(), ZDICT_cover_params_t { k: 4, d: 8, ..ok_cover() }),
            ("d>k big".into(), ZDICT_cover_params_t { k: 100, d: 101, ..ok_cover() }),
            ("k>cap".into(), ZDICT_cover_params_t { k: 4097, d: 8, ..ok_cover() }),
            ("k=u32::MAX".into(), ZDICT_cover_params_t { k: u32::MAX, d: 8, ..ok_cover() }),
            ("d=u32::MAX".into(), ZDICT_cover_params_t { k: 50, d: u32::MAX, ..ok_cover() }),
        ];
        for (name, p) in &bad_params {
            for cap in [256usize, 4096] {
                if p.k != 0 && p.k as usize <= cap && p.d != 0 && p.d <= p.k {
                    continue;
                }
                let what = format!("cover row479 {name} cap={cap}");
                let r = diff_train_p(&what, f, cap, cap, &co, *p);
                expect_err(&what, r, "Parameter is out of bound");
                hit479 += 1;
            }
        }
        assert!(hit479 >= 10, "row 479 ({hit479})");

        // ---- row 480: nbSamples == 0 -> srcSize_wrong
        let empty = Corpus { buf: vec![0u8; 1], sizes: vec![] };
        for cap in [256usize, 1024, 4096] {
            let what = format!("cover row480 cap={cap}");
            let r = diff_train_p(&what, f, cap, cap, &empty, ok_cover());
            expect_err(&what, r, "Src size is incorrect");
        }
        // ... and with NULL sample pointers too (nothing is dereferenced)
        {
            let mut dc = vec![0xA5u8; 512];
            let mut dr = vec![0xA5u8; 512];
            let rc = (f.0)(
                dc.as_mut_ptr() as *mut c_void,
                512,
                std::ptr::null(),
                std::ptr::null(),
                0,
                ok_cover(),
            );
            let rr = (f.1)(
                dr.as_mut_ptr() as *mut c_void,
                512,
                std::ptr::null(),
                std::ptr::null(),
                0,
                ok_cover(),
            );
            eqcode("cover row480 NULL samples", rc, rr);
            eqbuf("cover row480 NULL samples dst", &dc, &dr);
            expect_err("cover row480 NULL samples", rc, "Src size is incorrect");
        }

        // ---- row 481: dictBufferCapacity < ZDICT_DICTSIZE_MIN -> dstSize_tooSmall
        // `k <= maxDictSize` must hold, so keep k small.
        let mut hit481 = 0usize;
        for cap in [1usize, 8, 100, 128, 255] {
            for d in [6u32, 8] {
                let p = ZDICT_cover_params_t { k: 1, d, ..ok_cover() };
                if p.d > p.k {
                    // d>k would be row 479 instead
                    continue;
                }
                let what = format!("cover row481 cap={cap} d={d}");
                let r = diff_train_p(&what, f, cap, cap, &co, p);
                expect_err(&what, r, "Destination buffer is too small");
                hit481 += 1;
            }
            // k == 1 with d == 1 keeps checkParameters happy for tiny caps
            let p = ZDICT_cover_params_t { k: 1, d: 1, ..ok_cover() };
            let what = format!("cover row481 k=d=1 cap={cap}");
            let r = diff_train_p(&what, f, cap, cap, &co, p);
            expect_err(&what, r, "Destination buffer is too small");
            hit481 += 1;
        }
        assert!(hit481 >= 5, "row 481 ({hit481})");

        // ---- row 474: COVER_cmp8's `return -1` is on the hot path of every
        // successful d<=8 training (`COVER_cmp8` is chosen for d<=8 in
        // COVER_ctx_init).  One tiny successful run pins it, byte-for-byte.
        let good = Corpus::uniform(4, 8, 512, 0x1234);
        for d in [6u32, 8] {
            let p = ZDICT_cover_params_t { k: 32, d, ..ok_cover() };
            let what = format!("cover row474 d={d}");
            let r = diff_train_p(&what, f, 512, 512, &good, p);
            assert!(!is_err(r), "{what}: expected success, got {r:#x}");
        }
    }
}

// rows 475, 476 : COVER_ctx_init -> srcSize_wrong

#[test]
fn err_cover_ctx_init_rejections() {
    unsafe {
        let f = duo::<FnTrainCover>("ZDICT_trainFromBuffer_cover");

        // ---- row 475: totalSamplesSize < MAX(d, sizeof(U64))
        let mut hit475 = 0usize;
        for total in [1usize, 2, 5, 7] {
            for nb in [5usize, 6, 7] {
                if total < nb {
                    continue;
                }
                let each = total / nb;
                let mut sizes = vec![each; nb];
                let s: usize = sizes.iter().sum();
                if s < total {
                    sizes[0] += total - s;
                }
                let co = Corpus::new(4, sizes, 0x9001 + total as u64);
                assert_eq!(co.total(), total);
                for d in [6u32, 8] {
                    let p = ZDICT_cover_params_t { k: 50, d, ..ok_cover() };
                    let what = format!("cover row475 total={total} nb={nb} d={d}");
                    let r = diff_train_p(&what, f, 1024, 1024, &co, p);
                    expect_err(&what, r, "Src size is incorrect");
                    hit475 += 1;
                }
            }
        }
        assert!(hit475 >= 4, "row 475 ({hit475})");

        // ---- row 476: fewer than 5 training samples (splitPoint is 1.0 here,
        // so nbTrainSamples == nbSamples)
        let mut hit476 = 0usize;
        for nb in 1..=4usize {
            for sz in [8usize, 64, 512] {
                let co = Corpus::uniform(4, nb, sz, 0xA110 + nb as u64);
                for d in [6u32, 8] {
                    let p = ZDICT_cover_params_t { k: 50, d, ..ok_cover() };
                    let what = format!("cover row476 nb={nb} sz={sz} d={d}");
                    let r = diff_train_p(&what, f, 1024, 1024, &co, p);
                    expect_err(&what, r, "Src size is incorrect");
                    hit476 += 1;
                }
            }
        }
        assert!(hit476 >= 24, "row 476 ({hit476})");
    }
}

// rows 486, 487, 488, 489 : ZDICT_optimizeTrainFromBuffer_cover
// row 485 (observable)    : COVER_tryParameters' `ERROR(GENERIC)` initialisers

#[test]
fn err_cover_optimize_rejections() {
    unsafe {
        let f = duo::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
        let co = Corpus::uniform(4, 8, 512, 0x0B71);

        #[track_caller]
        unsafe fn run(
            what: &str,
            f: (FnOptCover, FnOptCover),
            cap: usize,
            real: usize,
            co: &Corpus,
            p: ZDICT_cover_params_t,
        ) -> usize {
            let mut dc = vec![0xA5u8; real.max(1)];
            let mut dr = vec![0xA5u8; real.max(1)];
            let mut pc = p;
            let mut pr = p;
            let rc =
                (f.0)(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pc);
            let rr =
                (f.1)(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pr);
            eqcode(&format!("{what}: return"), rc, rr);
            eq_cover_params(&format!("{what}: out-params"), pc, pr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            rc
        }

        // ---- row 486: splitPoint outside (0, 1].  `<= 0` is replaced by the
        // default 0.75, so only `> 1` (and +inf) reaches the rejection.
        let mut hit486 = 0usize;
        for sp in [1.0000000000000002f64, 1.5, 2.0, 1e300, f64::INFINITY] {
            let p = ZDICT_cover_params_t { splitPoint: sp, ..ok_cover() };
            let what = format!("cover row486 splitPoint={sp}");
            let r = run(&what, f, 1024, 1024, &co, p);
            expect_err(&what, r, "Parameter is out of bound");
            hit486 += 1;
        }
        // and the values that are *silently replaced* must not be rejected
        for sp in [0.0f64, -0.0, -1.0, f64::NEG_INFINITY, -1e300] {
            let p = ZDICT_cover_params_t { splitPoint: sp, ..ok_cover() };
            let what = format!("cover splitPoint-default {sp}");
            let r = run(&what, f, 1024, 1024, &co, p);
            assert!(
                !is_err(r) || cstr(duo::<FnErrName>("ZSTD_getErrorName").0(r)) != "Parameter is out of bound",
                "{what}: splitPoint <= 0 must fall back to the default"
            );
        }
        assert!(hit486 >= 5, "row 486 ({hit486})");

        // ---- row 487: kMinK < kMaxD || kMaxK < kMinK
        let mut hit487 = 0usize;
        for &(k, d) in &[(4u32, 8u32), (1, 6), (7, 8), (5, 6), (49, 50)] {
            let p = ZDICT_cover_params_t { k, d, ..ok_cover() };
            let what = format!("cover row487 k={k} d={d}");
            let r = run(&what, f, 4096, 4096, &co, p);
            expect_err(&what, r, "Parameter is out of bound");
            hit487 += 1;
        }
        assert!(hit487 >= 5, "row 487 ({hit487})");

        // ---- row 488: nbSamples == 0
        let empty = Corpus { buf: vec![0u8; 1], sizes: vec![] };
        for cap in [256usize, 4096] {
            let what = format!("cover row488 cap={cap}");
            let r = run(&what, f, cap, cap, &empty, ok_cover());
            expect_err(&what, r, "Src size is incorrect");
        }

        // ---- row 489: dictBufferCapacity < ZDICT_DICTSIZE_MIN
        let mut hit489 = 0usize;
        for cap in [0usize, 1, 8, 100, 255] {
            let what = format!("cover row489 cap={cap}");
            let r = run(&what, f, cap, cap.max(1), &co, ok_cover());
            expect_err(&what, r, "Destination buffer is too small");
            hit489 += 1;
        }
        assert!(hit489 >= 5, "row 489 ({hit489})");

        // ---- ctx_init errors propagated out of the optimizer (rows 475/476)
        for nb in [1usize, 4] {
            let small = Corpus::uniform(4, nb, 64, 0x77 + nb as u64);
            let what = format!("cover optimize ctx_init nb={nb}");
            let r = run(&what, f, 1024, 1024, &small, ok_cover());
            expect_err(&what, r, "Src size is incorrect");
        }

        // ---- row 485 (observable): make every candidate fail.  With a
        // `dictBufferCapacity` no allocator can satisfy, `COVER_tryParameters`
        // takes `if (!dict || !freqs) goto _cleanup;` and hands
        // `COVER_dictSelectionError(ERROR(GENERIC))` to `COVER_best_finish`, so
        // `best.compressedSize` is still `(size_t)-1 == ERROR(GENERIC)` at the
        // end and the optimizer returns GENERIC.
        for &cap in &[usize::MAX / 2, usize::MAX - 4096, usize::MAX / 3 * 2] {
            let what = format!("cover row485 cap={cap:#x}");
            let r = run(&what, f, cap, 4096, &co, ok_cover());
            expect_err(&what, r, "Error (generic)");
        }

        // ---- unvalidated knobs must still agree exactly: `steps`,
        // `shrinkDict`, `shrinkDictMaxRegression`, `nbThreads`.  (`nbThreads>1`
        // cannot fail: pool.c L326-336 returns &g_poolCtx in a non-MT build.)
        for steps in [0u32, 1, 2, u32::MAX] {
            for nbt in [0u32, 1, 2, 8, u32::MAX] {
                for &(sd, sdmr) in &[(0u32, 0u32), (1, 0), (1, 1), (1, 100), (1, u32::MAX)] {
                    let p = ZDICT_cover_params_t {
                        k: 50,
                        d: 8,
                        steps,
                        nbThreads: nbt,
                        shrinkDict: sd,
                        shrinkDictMaxRegression: sdmr,
                        ..ok_cover()
                    };
                    // keep it cheap: fail fast on the capacity check
                    let what = format!(
                        "cover knobs steps={steps} nbThreads={nbt} shrink={sd}/{sdmr}"
                    );
                    let r = run(&what, f, 100, 100, &co, p);
                    expect_err(&what, r, "Destination buffer is too small");
                }
            }
        }

        // NaN splitPoint: `NaN <= 0.0` and `NaN > 1` are both false, so the C
        // accepts it and then never takes the `splitPoint < 1.0` branch.
        {
            let p = ZDICT_cover_params_t { splitPoint: f64::NAN, ..ok_cover() };
            let r = run("cover NaN splitPoint", f, 512, 512, &co, p);
            let _ = r;
        }
    }
}

// row 483 : COVER_checkTotalCompressedSize -> ERROR(GENERIC)
// row 484 : COVER_best_finish             -> ERROR(GENERIC)

#[test]
fn err_cover_internal_generic_paths() {
    unsafe {
        let (ctc, ctr) = duo::<FnCheckTotal>("COVER_checkTotalCompressedSize");
        let (bic, bir) = duo::<FnBestVoid>("COVER_best_init");
        let (bsc, bsr) = duo::<FnBestVoid>("COVER_best_start");
        let (bfc, bfr) = duo::<FnBestFinish>("COVER_best_finish");
        let (bdc, bdr) = duo::<FnBestVoid>("COVER_best_destroy");

        let co = Corpus::uniform(4, 4, 256, 0x3311);
        let mut offs = co.offsets();
        let mut dictbuf_c = vec![0u8; 512];
        let mut dictbuf_r = vec![0u8; 512];

        // ---- row 483: `dst = malloc(ZSTD_compressBound(maxSampleSize))` fails,
        // so the function returns its `ERROR(GENERIC)` initialiser.  The sample
        // *sizes* array is a caller input, so a single absurd entry is enough
        // (the huge sample is never read: the malloc fails first).
        let mut hit483 = 0usize;
        for &huge in &[1usize << 62, usize::MAX / 4, usize::MAX - 1024] {
            let mut sizes = co.sizes.clone();
            sizes[0] = huge;
            let p = ok_cover();
            let a = ctc(
                p,
                sizes.as_ptr(),
                co.buf.as_ptr(),
                offs.as_mut_ptr(),
                0,
                sizes.len(),
                dictbuf_c.as_mut_ptr(),
                dictbuf_c.len(),
            );
            let b = ctr(
                p,
                sizes.as_ptr(),
                co.buf.as_ptr(),
                offs.as_mut_ptr(),
                0,
                sizes.len(),
                dictbuf_r.as_mut_ptr(),
                dictbuf_r.len(),
            );
            eqcode(&format!("row483 huge={huge:#x}"), a, b);
            eqbuf("row483 dict buffer", &dictbuf_c, &dictbuf_r);
            expect_err(&format!("row483 huge={huge:#x}"), a, "Error (generic)");
            hit483 += 1;
        }
        assert!(hit483 >= 3, "row 483 ({hit483})");

        // sanity: the same call with sane sizes succeeds identically
        {
            let a = ctc(
                ok_cover(),
                co.szp(),
                co.buf.as_ptr(),
                offs.as_mut_ptr(),
                0,
                co.sizes.len(),
                dictbuf_c.as_mut_ptr(),
                dictbuf_c.len(),
            );
            let b = ctr(
                ok_cover(),
                co.szp(),
                co.buf.as_ptr(),
                offs.as_mut_ptr(),
                0,
                co.sizes.len(),
                dictbuf_r.as_mut_ptr(),
                dictbuf_r.len(),
            );
            eqcode("checkTotal sane", a, b);
            assert!(!is_err(a), "checkTotal sane should succeed, got {a:#x}");
        }

        // ---- row 484: COVER_best_finish's `best->dict = malloc(dictSize)`
        // fails.  `dictSize` comes straight from the COVER_dictSelection_t the
        // caller supplies, so an unsatisfiable size reaches the site.
        let mut content = [0x5Au8; 32];
        for &huge in &[1usize << 63, usize::MAX / 2, usize::MAX - 64] {
            for lib in 0..2 {
                let (init, start, finish, destroy) = if lib == 0 {
                    (bic, bsc, bfc, bdc)
                } else {
                    (bir, bsr, bfr, bdr)
                };
                let mut best = COVER_best_t {
                    mutex: 0,
                    cond: 0,
                    liveJobs: 0,
                    dict: std::ptr::null_mut(),
                    dictSize: 0,
                    parameters: ok_cover(),
                    compressedSize: 0,
                };
                init(&mut best);
                start(&mut best);
                let sel = COVER_dictSelection_t {
                    dictContent: content.as_mut_ptr(),
                    dictSize: huge,
                    totalCompressedSize: 1,
                };
                finish(&mut best, ok_cover(), sel);
                let got = (best.compressedSize, best.dictSize, best.liveJobs);
                destroy(&mut best);
                if lib == 0 {
                    RESULT_C.with(|c| *c.borrow_mut() = Some(got));
                } else {
                    let c = RESULT_C.with(|c| c.borrow().unwrap());
                    eqv(&format!("row484 huge={huge:#x} best state"), c, got);
                    expect_err(&format!("row484 huge={huge:#x}"), c.0, "Error (generic)");
                    assert_eq!(c.1, 0, "row484: dictSize must be reset to 0");
                }
            }
        }
    }
}

thread_local! {
    static RESULT_C: std::cell::RefCell<Option<(usize, usize, usize)>> =
        const { std::cell::RefCell::new(None) };
}

// COVER_selectDict error shapes (COVER_dictSelectionError / IsError / Free)

#[test]
fn err_cover_select_dict_shapes() {
    unsafe {
        let (sc, sr) = duo::<FnSelectDict>("COVER_selectDict");
        let (iec, ier) = duo::<FnDsIsError>("COVER_dictSelectionIsError");
        let (fdc, fdr) = duo::<FnDsFree>("COVER_dictSelectionFree");

        let co = Corpus::uniform(4, 8, 256, 0x5D01);
        let content = gen_class(5, 256, 0x77AA);

        // (a) dictBufferCapacity below ZDICT_DICTSIZE_MIN -> the inner
        //     ZDICT_finalizeDictionary returns dstSize_tooSmall, so
        //     COVER_selectDict returns COVER_dictSelectionError(that).
        for cap in [1usize, 8, 100, 255] {
            for shrink in [0u32, 1] {
                let p = ZDICT_cover_params_t { shrinkDict: shrink, ..ok_cover() };
                let mut cc = vec![0u8; cap.max(1)];
                let mut cr = vec![0u8; cap.max(1)];
                let n = cap.min(content.len());
                cc[..n].copy_from_slice(&content[..n]);
                cr[..n].copy_from_slice(&content[..n]);
                let mut offs = co.offsets();
                let a = sc(
                    cc.as_mut_ptr(),
                    cap,
                    n,
                    co.buf.as_ptr(),
                    co.szp(),
                    co.nb(),
                    co.sizes.len(),
                    co.sizes.len(),
                    p,
                    offs.as_mut_ptr(),
                    0,
                );
                let b = sr(
                    cr.as_mut_ptr(),
                    cap,
                    n,
                    co.buf.as_ptr(),
                    co.szp(),
                    co.nb(),
                    co.sizes.len(),
                    co.sizes.len(),
                    p,
                    offs.as_mut_ptr(),
                    0,
                );
                let what = format!("selectDict cap={cap} shrink={shrink}");
                eqcode(&format!("{what}: dictSize"), a.dictSize, b.dictSize);
                eqcode(
                    &format!("{what}: totalCompressedSize"),
                    a.totalCompressedSize,
                    b.totalCompressedSize,
                );
                eqv(&format!("{what}: isError"), iec(a), ier(b));
                assert_eq!(iec(a), 1, "{what}: expected a dictSelection error");
                expect_err(&what, a.totalCompressedSize, "Destination buffer is too small");
                assert!(a.dictContent.is_null() && b.dictContent.is_null());
                fdc(a);
                fdr(b);
            }
        }

        // (b) a sane capacity but an absurd samplesSizes entry -> the inner
        //     COVER_checkTotalCompressedSize fails with GENERIC.
        //     `nbFinalizeSamples` is 0 so the inner ZDICT_finalizeDictionary
        //     never *reads* the (fictitious) huge sample; only
        //     COVER_checkTotalCompressedSize looks at `samplesSizes`, and it
        //     fails on `malloc(ZSTD_compressBound(maxSampleSize))` before any
        //     read.
        {
            let cap = 512usize;
            let mut sizes = co.sizes.clone();
            sizes[0] = usize::MAX / 4;
            let mut offs = co.offsets();
            let mut cc = vec![0u8; cap];
            let mut cr = vec![0u8; cap];
            cc[..content.len()].copy_from_slice(&content);
            cr[..content.len()].copy_from_slice(&content);
            let a = sc(
                cc.as_mut_ptr(),
                cap,
                content.len(),
                co.buf.as_ptr(),
                sizes.as_ptr(),
                0,
                sizes.len(),
                sizes.len(),
                ok_cover(),
                offs.as_mut_ptr(),
                0,
            );
            let b = sr(
                cr.as_mut_ptr(),
                cap,
                content.len(),
                co.buf.as_ptr(),
                sizes.as_ptr(),
                0,
                sizes.len(),
                sizes.len(),
                ok_cover(),
                offs.as_mut_ptr(),
                0,
            );
            eqcode("selectDict GENERIC dictSize", a.dictSize, b.dictSize);
            eqcode(
                "selectDict GENERIC totalCompressedSize",
                a.totalCompressedSize,
                b.totalCompressedSize,
            );
            eqv("selectDict GENERIC isError", iec(a), ier(b));
            assert_eq!(iec(a), 1);
            fdc(a);
            fdr(b);
        }
    }
}

// =============================================================== fastcover.c
// rows 500, 501, 502 : ZDICT_trainFromBuffer_fastCover

#[test]
fn err_fastcover_train_rejections() {
    unsafe {
        let f = duo::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
        let co = Corpus::uniform(4, 8, 512, 0xFC01);

        // ---- row 500: FASTCOVER_checkParameters fails -> parameter_outOfBound.
        // (splitPoint is forced to 1.0 and `f`/`accel` 0 are replaced with the
        // defaults *before* the check, so those clauses are dead here.)
        let mut hit500 = 0usize;
        let bad: Vec<(String, ZDICT_fastCover_params_t)> = vec![
            ("d=0".into(), ZDICT_fastCover_params_t { d: 0, ..ok_fast() }),
            ("k=0".into(), ZDICT_fastCover_params_t { k: 0, ..ok_fast() }),
            ("d=5".into(), ZDICT_fastCover_params_t { d: 5, ..ok_fast() }),
            ("d=7".into(), ZDICT_fastCover_params_t { d: 7, ..ok_fast() }),
            ("d=9".into(), ZDICT_fastCover_params_t { d: 9, ..ok_fast() }),
            ("d=16".into(), ZDICT_fastCover_params_t { d: 16, ..ok_fast() }),
            ("d=u32::MAX".into(), ZDICT_fastCover_params_t { d: u32::MAX, ..ok_fast() }),
            ("k>cap".into(), ZDICT_fastCover_params_t { k: 5000, ..ok_fast() }),
            ("k=u32::MAX".into(), ZDICT_fastCover_params_t { k: u32::MAX, ..ok_fast() }),
            ("k<d".into(), ZDICT_fastCover_params_t { k: 6, d: 8, ..ok_fast() }),
            (
                "f=32".into(),
                ZDICT_fastCover_params_t { f: FASTCOVER_MAX_F + 1, ..ok_fast() },
            ),
            ("f=64".into(), ZDICT_fastCover_params_t { f: 64, ..ok_fast() }),
            ("f=u32::MAX".into(), ZDICT_fastCover_params_t { f: u32::MAX, ..ok_fast() }),
            (
                "accel=11".into(),
                ZDICT_fastCover_params_t { accel: FASTCOVER_MAX_ACCEL + 1, ..ok_fast() },
            ),
            ("accel=255".into(), ZDICT_fastCover_params_t { accel: 255, ..ok_fast() }),
            (
                "accel=u32::MAX".into(),
                ZDICT_fastCover_params_t { accel: u32::MAX, ..ok_fast() },
            ),
        ];
        for (name, p) in &bad {
            let what = format!("fastcover row500 {name}");
            let r = diff_train_p(&what, f, 4096, 4096, &co, *p);
            expect_err(&what, r, "Parameter is out of bound");
            hit500 += 1;
        }
        assert!(hit500 >= 16, "row 500 ({hit500})");

        // `f == 0` and `accel == 0` are *replaced* by DEFAULT_F / DEFAULT_ACCEL
        for p in [
            ZDICT_fastCover_params_t { f: 0, ..ok_fast() },
            ZDICT_fastCover_params_t { accel: 0, ..ok_fast() },
            ZDICT_fastCover_params_t { f: 0, accel: 0, ..ok_fast() },
            ZDICT_fastCover_params_t { f: FASTCOVER_MAX_F, ..ok_fast() },
            ZDICT_fastCover_params_t { accel: FASTCOVER_MAX_ACCEL, ..ok_fast() },
        ] {
            // `k == d == 8 <= maxDictSize == 8` keeps FASTCOVER_checkParameters
            // happy, so *reaching* the capacity check proves the substituted
            // default `f` / `accel` were accepted.
            let p = ZDICT_fastCover_params_t { k: 8, d: 8, ..p };
            let what = format!("fastcover defaults f={} accel={}", p.f, p.accel);
            let r = diff_train_p(&what, f, 8, 8, &co, p);
            expect_err(&what, r, "Destination buffer is too small");
        }

        // ---- row 501: nbSamples == 0 -> srcSize_wrong
        let empty = Corpus { buf: vec![0u8; 1], sizes: vec![] };
        for cap in [256usize, 4096] {
            let what = format!("fastcover row501 cap={cap}");
            let r = diff_train_p(&what, f, cap, cap, &empty, ok_fast());
            expect_err(&what, r, "Src size is incorrect");
        }
        {
            let mut dc = vec![0xA5u8; 512];
            let mut dr = vec![0xA5u8; 512];
            let rc = (f.0)(
                dc.as_mut_ptr() as *mut c_void,
                512,
                std::ptr::null(),
                std::ptr::null(),
                0,
                ok_fast(),
            );
            let rr = (f.1)(
                dr.as_mut_ptr() as *mut c_void,
                512,
                std::ptr::null(),
                std::ptr::null(),
                0,
                ok_fast(),
            );
            eqcode("fastcover row501 NULL samples", rc, rr);
            eqbuf("fastcover row501 NULL samples dst", &dc, &dr);
            expect_err("fastcover row501 NULL samples", rc, "Src size is incorrect");
        }

        // ---- row 502: dictBufferCapacity < ZDICT_DICTSIZE_MIN
        let mut hit502 = 0usize;
        for cap in [8usize, 64, 100, 200, 255] {
            for d in [6u32, 8] {
                let p = ZDICT_fastCover_params_t { k: 8, d, ..ok_fast() };
                let what = format!("fastcover row502 cap={cap} d={d}");
                let r = diff_train_p(&what, f, cap, cap, &co, p);
                expect_err(&what, r, "Destination buffer is too small");
                hit502 += 1;
            }
        }
        assert!(hit502 >= 10, "row 502 ({hit502})");
    }
}

// rows 494, 495 : FASTCOVER_ctx_init -> srcSize_wrong

#[test]
fn err_fastcover_ctx_init_rejections() {
    unsafe {
        let f = duo::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");

        // ---- row 494: totalSamplesSize < MAX(d, sizeof(U64))
        let mut hit494 = 0usize;
        for total in [1usize, 5, 7] {
            let nb = 5usize;
            if total < nb {
                let mut sizes = vec![0usize; nb];
                sizes[0] = total;
                let co = Corpus::new(4, sizes, 0x2001 + total as u64);
                assert_eq!(co.total(), total);
                for d in [6u32, 8] {
                    let p = ZDICT_fastCover_params_t { k: 50, d, ..ok_fast() };
                    let what = format!("fastcover row494 total={total} d={d}");
                    let r = diff_train_p(&what, f, 1024, 1024, &co, p);
                    expect_err(&what, r, "Src size is incorrect");
                    hit494 += 1;
                }
            } else {
                let each = total / nb;
                let mut sizes = vec![each; nb];
                let s: usize = sizes.iter().sum();
                if s < total {
                    sizes[0] += total - s;
                }
                let co = Corpus::new(4, sizes, 0x2001 + total as u64);
                for d in [6u32, 8] {
                    let p = ZDICT_fastCover_params_t { k: 50, d, ..ok_fast() };
                    let what = format!("fastcover row494 total={total} d={d}");
                    let r = diff_train_p(&what, f, 1024, 1024, &co, p);
                    expect_err(&what, r, "Src size is incorrect");
                    hit494 += 1;
                }
            }
        }
        assert!(hit494 >= 6, "row 494 ({hit494})");

        // ---- row 495: fewer than 5 training samples
        let mut hit495 = 0usize;
        for nb in 1..=4usize {
            for sz in [8usize, 64, 512] {
                let co = Corpus::uniform(4, nb, sz, 0xB110 + nb as u64);
                for d in [6u32, 8] {
                    let p = ZDICT_fastCover_params_t { k: 50, d, ..ok_fast() };
                    let what = format!("fastcover row495 nb={nb} sz={sz} d={d}");
                    let r = diff_train_p(&what, f, 1024, 1024, &co, p);
                    expect_err(&what, r, "Src size is incorrect");
                    hit495 += 1;
                }
            }
        }
        assert!(hit495 >= 24, "row 495 ({hit495})");
    }
}

// rows 503, 504, 505, 506, 507 : ZDICT_optimizeTrainFromBuffer_fastCover
// row 499 (observable)         : FASTCOVER_tryParameters' ERROR(GENERIC)

#[test]
fn err_fastcover_optimize_rejections() {
    unsafe {
        let f = duo::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
        let co = Corpus::uniform(4, 8, 512, 0x0F71);

        #[track_caller]
        unsafe fn run(
            what: &str,
            f: (FnOptFast, FnOptFast),
            cap: usize,
            real: usize,
            co: &Corpus,
            p: ZDICT_fastCover_params_t,
        ) -> usize {
            let mut dc = vec![0xA5u8; real.max(1)];
            let mut dr = vec![0xA5u8; real.max(1)];
            let mut pc = p;
            let mut pr = p;
            let rc =
                (f.0)(dc.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pc);
            let rr =
                (f.1)(dr.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb(), &mut pr);
            eqcode(&format!("{what}: return"), rc, rr);
            eq_fast_params(&format!("{what}: out-params"), pc, pr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            rc
        }

        // ---- row 503: splitPoint outside (0, 1]
        let mut hit503 = 0usize;
        for sp in [1.0000000000000002f64, 1.5, 2.0, 1e300, f64::INFINITY] {
            let p = ZDICT_fastCover_params_t { splitPoint: sp, ..ok_fast() };
            let what = format!("fastcover row503 splitPoint={sp}");
            let r = run(&what, f, 1024, 1024, &co, p);
            expect_err(&what, r, "Parameter is out of bound");
            hit503 += 1;
        }
        assert!(hit503 >= 5, "row 503 ({hit503})");

        // ---- row 504: accel == 0 || accel > FASTCOVER_MAX_ACCEL
        // (`accel == 0` is replaced by DEFAULT_ACCEL, so only `> 10` fires)
        let mut hit504 = 0usize;
        for accel in [11u32, 12, 100, u32::MAX] {
            let p = ZDICT_fastCover_params_t { accel, ..ok_fast() };
            let what = format!("fastcover row504 accel={accel}");
            let r = run(&what, f, 1024, 1024, &co, p);
            expect_err(&what, r, "Parameter is out of bound");
            hit504 += 1;
        }
        assert!(hit504 >= 4, "row 504 ({hit504})");

        // ---- row 505: kMinK < kMaxD || kMaxK < kMinK
        let mut hit505 = 0usize;
        for &(k, d) in &[(4u32, 8u32), (1, 6), (7, 8), (5, 6)] {
            let p = ZDICT_fastCover_params_t { k, d, ..ok_fast() };
            let what = format!("fastcover row505 k={k} d={d}");
            let r = run(&what, f, 4096, 4096, &co, p);
            expect_err(&what, r, "Parameter is out of bound");
            hit505 += 1;
        }
        assert!(hit505 >= 4, "row 505 ({hit505})");

        // ---- row 506: nbSamples == 0
        let empty = Corpus { buf: vec![0u8; 1], sizes: vec![] };
        for cap in [256usize, 4096] {
            let what = format!("fastcover row506 cap={cap}");
            let r = run(&what, f, cap, cap, &empty, ok_fast());
            expect_err(&what, r, "Src size is incorrect");
        }

        // ---- row 507: dictBufferCapacity < ZDICT_DICTSIZE_MIN
        let mut hit507 = 0usize;
        for cap in [0usize, 1, 8, 100, 255] {
            let what = format!("fastcover row507 cap={cap}");
            let r = run(&what, f, cap, cap.max(1), &co, ok_fast());
            expect_err(&what, r, "Destination buffer is too small");
            hit507 += 1;
        }
        assert!(hit507 >= 5, "row 507 ({hit507})");

        // ---- ctx_init errors propagated (rows 494/495)
        for nb in [1usize, 4] {
            let small = Corpus::uniform(4, nb, 64, 0x88 + nb as u64);
            let what = format!("fastcover optimize ctx_init nb={nb}");
            let r = run(&what, f, 1024, 1024, &small, ok_fast());
            expect_err(&what, r, "Src size is incorrect");
        }

        // ---- row 499 (observable): unsatisfiable dictBufferCapacity makes
        // every candidate bail out with COVER_dictSelectionError(ERROR(GENERIC)).
        for &cap in &[usize::MAX / 2, usize::MAX - 4096] {
            let what = format!("fastcover row499 cap={cap:#x}");
            let r = run(&what, f, cap, 4096, &co, ok_fast());
            expect_err(&what, r, "Error (generic)");
        }

        // ---- unvalidated knobs
        for steps in [0u32, 1, u32::MAX] {
            for nbt in [0u32, 1, 2, u32::MAX] {
                for &(sd, sdmr) in &[(0u32, 0u32), (1, 0), (1, u32::MAX)] {
                    let p = ZDICT_fastCover_params_t {
                        steps,
                        nbThreads: nbt,
                        shrinkDict: sd,
                        shrinkDictMaxRegression: sdmr,
                        ..ok_fast()
                    };
                    let what = format!(
                        "fastcover knobs steps={steps} nbThreads={nbt} shrink={sd}/{sdmr}"
                    );
                    let r = run(&what, f, 100, 100, &co, p);
                    expect_err(&what, r, "Destination buffer is too small");
                }
            }
        }
    }
}

// `ZDICT_trainFromBuffer` is a thin wrapper over the fastCover optimizer
// (d = 8, steps = 4, k = 0): every rejection above must surface through it too.

#[test]
fn err_zdict_train_from_buffer_wrapper() {
    unsafe {
        let (fc, fr) = duo::<FnTrain>("ZDICT_trainFromBuffer");

        #[track_caller]
        unsafe fn run(
            what: &str,
            f: (FnTrain, FnTrain),
            cap: usize,
            real: usize,
            sb: *const c_void,
            sz: *const usize,
            nb: c_uint,
        ) -> usize {
            let mut dc = vec![0xA5u8; real.max(1)];
            let mut dr = vec![0xA5u8; real.max(1)];
            let rc = (f.0)(dc.as_mut_ptr() as *mut c_void, cap, sb, sz, nb);
            let rr = (f.1)(dr.as_mut_ptr() as *mut c_void, cap, sb, sz, nb);
            eqcode(&format!("{what}: return"), rc, rr);
            eqbuf(&format!("{what}: dictBuffer"), &dc, &dr);
            rc
        }

        let co = Corpus::uniform(4, 8, 512, 0x7B01);

        // nbSamples == 0, including NULL sample pointers
        for cap in [0usize, 255, 256, 4096] {
            let what = format!("train nb=0 cap={cap}");
            let r =
                run(&what, (fc, fr), cap, cap.max(1), std::ptr::null(), std::ptr::null(), 0);
            expect_err(&what, r, "Src size is incorrect");
        }
        // dictBufferCapacity below the minimum (dictBuffer may even be NULL)
        for cap in [0usize, 1, 8, 100, 255] {
            let what = format!("train cap={cap}");
            let r = run(&what, (fc, fr), cap, cap.max(1), co.sp(), co.szp(), co.nb());
            expect_err(&what, r, "Destination buffer is too small");
            let mut _dc = 0u8;
            let rc = fc(std::ptr::null_mut(), cap, co.sp(), co.szp(), co.nb());
            let rr = fr(std::ptr::null_mut(), cap, co.sp(), co.szp(), co.nb());
            eqcode(&format!("{what} NULL dst"), rc, rr);
            expect_err(&format!("{what} NULL dst"), rc, "Destination buffer is too small");
        }
        // total sample size below MAX(d, 8) and fewer than 5 samples
        for total in [0usize, 1, 7] {
            let mut sizes = vec![0usize; 5];
            sizes[0] = total;
            let c2 = Corpus::new(4, sizes, 0x99 + total as u64);
            let what = format!("train total={total}");
            let r = run(&what, (fc, fr), 1024, 1024, c2.sp(), c2.szp(), c2.nb());
            expect_err(&what, r, "Src size is incorrect");
        }
        for nb in 1..=4usize {
            let c2 = Corpus::uniform(4, nb, 256, 0xC1 + nb as u64);
            let what = format!("train nb={nb}");
            let r = run(&what, (fc, fr), 1024, 1024, c2.sp(), c2.szp(), c2.nb());
            expect_err(&what, r, "Src size is incorrect");
        }
        // zero-sized individual samples inside an otherwise valid corpus
        {
            let sizes = vec![0usize, 256, 0, 256, 0, 256, 0, 256];
            let c2 = Corpus::new(4, sizes, 0xD00D);
            let r = run("train zero-sized samples", (fc, fr), 1024, 1024, c2.sp(), c2.szp(), c2.nb());
            let _ = r;
        }
        // unsatisfiable capacity -> ERROR(GENERIC) through the wrapper
        {
            let r = run(
                "train GENERIC",
                (fc, fr),
                usize::MAX / 2,
                4096,
                co.sp(),
                co.szp(),
                co.nb(),
            );
            expect_err("train GENERIC", r, "Error (generic)");
        }
    }
}

// ------------------------------------------------------- exhaustive param grid
//
// Sweeps `k`, `d`, `f`, `accel`, `splitPoint`, `steps` and `nbThreads` across
// all four COVER / FASTCOVER entry points with a capacity of 100.  Because the
// capacity check is the *last* one in every entry point, a parameter set that
// only produces `dstSize_tooSmall` is proof that it passed
// `COVER_checkParameters` / `FASTCOVER_checkParameters`, and anything else is a
// parameter rejection — so the grid pins the exact decision boundary of both
// predicates without ever running a (slow) training pass.

#[test]
fn err_cover_fastcover_param_grid() {
    unsafe {
        let tc = duo::<FnTrainCover>("ZDICT_trainFromBuffer_cover");
        let tf = duo::<FnTrainFast>("ZDICT_trainFromBuffer_fastCover");
        let oc = duo::<FnOptCover>("ZDICT_optimizeTrainFromBuffer_cover");
        let of = duo::<FnOptFast>("ZDICT_optimizeTrainFromBuffer_fastCover");
        let co = Corpus::uniform(4, 8, 512, 0x6A1D);
        const CAP: usize = 100;

        let ks: [c_uint; 12] = [0, 1, 5, 6, 7, 8, 9, 50, 99, 100, 101, u32::MAX];
        let ds: [c_uint; 10] = [0, 1, 5, 6, 7, 8, 9, 16, 100, u32::MAX];
        let fs: [c_uint; 7] = [0, 1, 15, 20, 31, 32, u32::MAX];
        let accels: [c_uint; 6] = [0, 1, 5, 10, 11, u32::MAX];
        let sps: [f64; 8] = [-1.0, 0.0, 1e-9, 0.5, 1.0, 1.0000000000000002, 2.0, f64::INFINITY];

        let mut n_param = 0usize;
        let mut n_dst = 0usize;

        // ---- COVER: k x d x splitPoint x steps x nbThreads
        for &k in &ks {
            for &d in &ds {
                for &sp in &sps {
                    let p = ZDICT_cover_params_t {
                        k,
                        d,
                        steps: 1,
                        nbThreads: 1,
                        splitPoint: sp,
                        shrinkDict: 0,
                        shrinkDictMaxRegression: 0,
                        zParams: zparams(3),
                    };
                    // by-value trainer: splitPoint is forced to 1.0 first
                    let what = format!("grid cover train k={k} d={d} sp={sp}");
                    let r = diff_train_p(&what, tc, CAP, CAP, &co, p);
                    let ok_params = k != 0 && d != 0 && (k as usize) <= CAP && d <= k;
                    if ok_params {
                        expect_err(&what, r, "Destination buffer is too small");
                        n_dst += 1;
                    } else {
                        expect_err(&what, r, "Parameter is out of bound");
                        n_param += 1;
                    }

                    // optimizer: splitPoint <= 0 -> default, > 1 -> rejected
                    let mut pc = p;
                    let mut pr = p;
                    let mut dc = vec![0xA5u8; CAP];
                    let mut dr = vec![0xA5u8; CAP];
                    let a = (oc.0)(
                        dc.as_mut_ptr() as *mut c_void,
                        CAP,
                        co.sp(),
                        co.szp(),
                        co.nb(),
                        &mut pc,
                    );
                    let b = (oc.1)(
                        dr.as_mut_ptr() as *mut c_void,
                        CAP,
                        co.sp(),
                        co.szp(),
                        co.nb(),
                        &mut pr,
                    );
                    let what = format!("grid cover optimize k={k} d={d} sp={sp}");
                    eqcode(&format!("{what}: return"), a, b);
                    eq_cover_params(&format!("{what}: out-params"), pc, pr);
                    eqbuf(&format!("{what}: dst"), &dc, &dr);
                    assert!(is_err(a), "{what}: must fail (cap < ZDICT_DICTSIZE_MIN)");
                }
            }
        }
        assert!(n_param >= 200 && n_dst >= 20, "cover grid: param={n_param} dst={n_dst}");

        // ---- FASTCOVER: k x d x f x accel
        let mut m_param = 0usize;
        let mut m_dst = 0usize;
        for &k in &ks {
            for &d in &ds {
                for &fv in &fs {
                    for &accel in &accels {
                        let p = ZDICT_fastCover_params_t {
                            k,
                            d,
                            f: fv,
                            steps: 1,
                            nbThreads: 1,
                            splitPoint: 1.0,
                            accel,
                            shrinkDict: 0,
                            shrinkDictMaxRegression: 0,
                            zParams: zparams(3),
                        };
                        let what =
                            format!("grid fastcover train k={k} d={d} f={fv} accel={accel}");
                        let r = diff_train_p(&what, tf, CAP, CAP, &co, p);
                        // `f == 0` -> DEFAULT_F(20), `accel == 0` -> DEFAULT_ACCEL(1)
                        let fe = if fv == 0 { 20 } else { fv };
                        let ae = if accel == 0 { 1 } else { accel };
                        let ok_params = k != 0
                            && d != 0
                            && (d == 6 || d == 8)
                            && (k as usize) <= CAP
                            && d <= k
                            && fe <= FASTCOVER_MAX_F
                            && ae <= FASTCOVER_MAX_ACCEL;
                        if ok_params {
                            expect_err(&what, r, "Destination buffer is too small");
                            m_dst += 1;
                        } else {
                            expect_err(&what, r, "Parameter is out of bound");
                            m_param += 1;
                        }
                    }
                }
            }
        }
        assert!(
            m_param >= 3000 && m_dst >= 10,
            "fastcover grid: param={m_param} dst={m_dst}"
        );

        // ---- FASTCOVER optimizer: the checks are ordered
        // splitPoint -> accel -> k/d -> nbSamples -> capacity.
        for &k in &ks {
            for &d in &ds {
                for &fv in &[0u32, 20, 31, 32] {
                    for &accel in &accels {
                        for &sp in &[0.0f64, 0.5, 1.0, 2.0] {
                            let p = ZDICT_fastCover_params_t {
                                k,
                                d,
                                f: fv,
                                steps: 1,
                                nbThreads: 1,
                                splitPoint: sp,
                                accel,
                                shrinkDict: 0,
                                shrinkDictMaxRegression: 0,
                                zParams: zparams(3),
                            };
                            let mut pc = p;
                            let mut pr = p;
                            let mut dc = vec![0xA5u8; CAP];
                            let mut dr = vec![0xA5u8; CAP];
                            let a = (of.0)(
                                dc.as_mut_ptr() as *mut c_void,
                                CAP,
                                co.sp(),
                                co.szp(),
                                co.nb(),
                                &mut pc,
                            );
                            let b = (of.1)(
                                dr.as_mut_ptr() as *mut c_void,
                                CAP,
                                co.sp(),
                                co.szp(),
                                co.nb(),
                                &mut pr,
                            );
                            let what = format!(
                                "grid fastcover optimize k={k} d={d} f={fv} accel={accel} sp={sp}"
                            );
                            eqcode(&format!("{what}: return"), a, b);
                            eq_fast_params(&format!("{what}: out-params"), pc, pr);
                            eqbuf(&format!("{what}: dst"), &dc, &dr);
                            assert!(is_err(a), "{what}: must fail");
                        }
                    }
                }
            }
        }
    }
}

// ============================================================= divsufsort.c
// rows 492, 493 : `return -1` on bad arguments

#[test]
fn err_divsufsort_bad_arguments() {
    unsafe {
        let (dc, dr) = duo::<FnDivsufsort>("divsufsort");
        let (bc, br) = duo::<FnDivbwt>("divbwt");

        let t = gen_class(4, 256, 0xD5D5);
        let mut sa_c = vec![0i32; t.len() + 2];
        let mut sa_r = vec![0i32; t.len() + 2];
        let mut u_c = vec![0u8; t.len() + 2];
        let mut u_r = vec![0u8; t.len() + 2];

        // ---- row 492: divsufsort(T == NULL || SA == NULL || n < 0)
        let mut hit492 = 0usize;
        for n in [-1i32, -2, -1000, i32::MIN, 0, 1, 16, 256] {
            // T == NULL
            let a = dc(std::ptr::null(), sa_c.as_mut_ptr(), n, 0);
            let b = dr(std::ptr::null(), sa_r.as_mut_ptr(), n, 0);
            eqv(&format!("divsufsort T=NULL n={n}"), a, b);
            assert_eq!(a, -1, "divsufsort T=NULL n={n} should be -1");
            eqbuf("divsufsort T=NULL SA", as_bytes(&sa_c), as_bytes(&sa_r));
            hit492 += 1;
            // SA == NULL
            let a = dc(t.as_ptr(), std::ptr::null_mut(), n, 0);
            let b = dr(t.as_ptr(), std::ptr::null_mut(), n, 0);
            eqv(&format!("divsufsort SA=NULL n={n}"), a, b);
            assert_eq!(a, -1, "divsufsort SA=NULL n={n} should be -1");
            hit492 += 1;
            // both NULL
            let a = dc(std::ptr::null(), std::ptr::null_mut(), n, 0);
            let b = dr(std::ptr::null(), std::ptr::null_mut(), n, 0);
            eqv(&format!("divsufsort both NULL n={n}"), a, b);
            assert_eq!(a, -1);
            hit492 += 1;
            // n < 0 with valid pointers
            if n < 0 {
                let a = dc(t.as_ptr(), sa_c.as_mut_ptr(), n, 0);
                let b = dr(t.as_ptr(), sa_r.as_mut_ptr(), n, 0);
                eqv(&format!("divsufsort n={n}"), a, b);
                assert_eq!(a, -1, "divsufsort n={n} should be -1");
                eqbuf("divsufsort n<0 SA", as_bytes(&sa_c), as_bytes(&sa_r));
                hit492 += 1;
            }
        }
        assert!(hit492 >= 24, "row 492 ({hit492})");

        // ---- row 493: divbwt(T == NULL || U == NULL || n < 0)
        let mut hit493 = 0usize;
        for n in [-1i32, -7, i32::MIN, 0, 1, 32, 256] {
            let a = bc(
                std::ptr::null(),
                u_c.as_mut_ptr(),
                sa_c.as_mut_ptr(),
                n,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            let b = br(
                std::ptr::null(),
                u_r.as_mut_ptr(),
                sa_r.as_mut_ptr(),
                n,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            eqv(&format!("divbwt T=NULL n={n}"), a, b);
            assert_eq!(a, -1);
            eqbuf("divbwt T=NULL U", &u_c, &u_r);
            hit493 += 1;

            let a = bc(
                t.as_ptr(),
                std::ptr::null_mut(),
                sa_c.as_mut_ptr(),
                n,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            let b = br(
                t.as_ptr(),
                std::ptr::null_mut(),
                sa_r.as_mut_ptr(),
                n,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            eqv(&format!("divbwt U=NULL n={n}"), a, b);
            assert_eq!(a, -1);
            hit493 += 1;

            if n < 0 {
                let a = bc(
                    t.as_ptr(),
                    u_c.as_mut_ptr(),
                    sa_c.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                );
                let b = br(
                    t.as_ptr(),
                    u_r.as_mut_ptr(),
                    sa_r.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                );
                eqv(&format!("divbwt n={n}"), a, b);
                assert_eq!(a, -1);
                eqbuf("divbwt n<0 U", &u_c, &u_r);
                hit493 += 1;
            }
        }
        assert!(hit493 >= 17, "row 493 ({hit493})");
    }
}

fn as_bytes(v: &[i32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

// ============================================================ zstd_ddict.c
// rows 305, 306, 307 : ZSTD_loadEntropy_intoDDict -> dictionary_corrupted
// rows 309, 310, 311 : ZSTD_createDDict_advanced  -> NULL
// row  308           : ZSTD_initDDict_internal    -> memory_allocation

/// Failure-injecting `ZSTD_customMem`: the `n`-th allocation returns NULL.
#[repr(C)]
struct AllocInj {
    remaining: i64,
    allocs: i64,
    frees: i64,
}

unsafe extern "C" fn inj_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    unsafe {
        let st = &mut *(opaque as *mut AllocInj);
        st.allocs += 1;
        if st.remaining == 0 {
            return std::ptr::null_mut();
        }
        st.remaining -= 1;
        malloc(size.max(1))
    }
}

unsafe extern "C" fn inj_free(opaque: *mut c_void, p: *mut c_void) {
    unsafe {
        let st = &mut *(opaque as *mut AllocInj);
        st.frees += 1;
        if !p.is_null() {
            free(p);
        }
    }
}

/// A real, well-formed dictionary trained by the C library.
fn real_dict(cap: usize, seed: u64) -> Vec<u8> {
    unsafe {
        let (fc, _) = duo::<FnTrain>("ZDICT_trainFromBuffer");
        let co = Corpus::uniform(4, 24, 512, seed);
        let mut d = vec![0u8; cap];
        let n = fc(d.as_mut_ptr() as *mut c_void, cap, co.sp(), co.szp(), co.nb());
        assert!(!is_err(n), "helper real_dict failed: {n:#x}");
        d.truncate(n);
        assert!(d.len() > 8, "helper real_dict produced {} bytes", d.len());
        d
    }
}

#[test]
fn err_ddict_create_advanced() {
    unsafe {
        let (cac, car) = duo::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
        let (ctc, ctr) = duo::<FnDDictContent>("ZSTD_DDict_dictContent");
        let (dsc, dsr) = duo::<FnDDictSize>("ZSTD_DDict_dictSize");
        let (gic, gir) = duo::<FnDDictU32>("ZSTD_getDictID_fromDDict");
        let (soc, sor) = duo::<FnPtrToSize>("ZSTD_sizeof_DDict");

        let good = real_dict(1024, 0xDD01);
        let magic = ZSTD_MAGIC_DICTIONARY.to_le_bytes();

        // Dictionary shapes, from "empty" to "magic + garbage".
        let mut shapes: Vec<(String, Vec<u8>)> = Vec::new();
        shapes.push(("good".into(), good.clone()));
        for n in 0..=8usize {
            shapes.push((format!("short[{n}]"), good[..n].to_vec()));
        }
        for n in [9usize, 12, 16, 32, 100, 256] {
            shapes.push((format!("truncated[{n}]"), good[..n.min(good.len())].to_vec()));
        }
        {
            let mut w = good.clone();
            w[0] ^= 0xFF;
            shapes.push(("wrong-magic".into(), w));
        }
        {
            let mut v = magic.to_vec();
            v.extend_from_slice(&12345u32.to_le_bytes());
            let mut rng = Rng::new(0xABCD);
            let n = 200usize;
            v.extend_from_slice(&rng.bytes(n));
            shapes.push(("magic+random".into(), v));
        }
        {
            let mut v = magic.to_vec();
            v.extend_from_slice(&0u32.to_le_bytes());
            v.resize(64, 0);
            shapes.push(("magic+zeros".into(), v));
        }
        {
            // valid header, then a truncated entropy section
            let mut v = good.clone();
            v.truncate(20);
            shapes.push(("good-truncated-entropy".into(), v));
        }

        let mut corrupted_hits = 0usize;
        for (name, d) in &shapes {
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let what = format!("createDDict_advanced {name} dlm={dlm} dct={dct}");
                    let a = cac(
                        d.as_ptr() as *const c_void,
                        d.len(),
                        dlm,
                        dct,
                        ZSTD_customMem::default(),
                    );
                    let b = car(
                        d.as_ptr() as *const c_void,
                        d.len(),
                        dlm,
                        dct,
                        ZSTD_customMem::default(),
                    );
                    eqv(&format!("{what}: NULL-ness"), a.is_null(), b.is_null());
                    if a.is_null() {
                        // rows 305/306/307 -> row 311.  A rejection is only
                        // legal when the caller demanded a full dictionary
                        // (rows 305/306) or when the buffer *claims* to be one
                        // (magic present, >= 8 bytes) but its entropy tables do
                        // not parse (row 307, which is content-type agnostic).
                        corrupted_hits += 1;
                        let claims_dict = d.len() >= 8 && d[..4] == magic;
                        assert!(
                            dct == ZSTD_dct_fullDict || claims_dict,
                            "{what}: unexpected rejection"
                        );
                        continue;
                    }
                    eqv(&format!("{what}: dictSize"), dsc(a), dsr(b));
                    eqv(&format!("{what}: dictID"), gic(a), gir(b));
                    eqv(&format!("{what}: sizeof"), soc(a), sor(b));
                    let (pa, pb) = (ctc(a), ctr(b));
                    let n = dsc(a);
                    if n > 0 {
                        assert!(!pa.is_null() && !pb.is_null(), "{what}: NULL dictContent");
                        let sa = std::slice::from_raw_parts(pa as *const u8, n);
                        let sb = std::slice::from_raw_parts(pb as *const u8, n);
                        eqbuf(&format!("{what}: dictContent"), sa, sb);
                        eqbuf(&format!("{what}: dictContent == input"), sa, &d[..n]);
                    }
                    eqv(&format!("{what}: freeDDict"), fdc(a), fdr(b));
                }
            }
        }
        assert!(corrupted_hits >= 10, "rows 305/306/307/311 ({corrupted_hits})");

        // NULL dict / zero size are legal (`dictContent = NULL`, `dictSize = 0`)
        for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
            for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                let what = format!("createDDict_advanced NULL dlm={dlm} dct={dct}");
                let a = cac(std::ptr::null(), 0, dlm, dct, ZSTD_customMem::default());
                let b = car(std::ptr::null(), 0, dlm, dct, ZSTD_customMem::default());
                eqv(&format!("{what}: NULL-ness"), a.is_null(), b.is_null());
                if !a.is_null() {
                    eqv(&format!("{what}: dictSize"), dsc(a), dsr(b));
                    eqv(&format!("{what}: dictID"), gic(a), gir(b));
                    eqv(&format!("{what}: content-null"), ctc(a).is_null(), ctr(b).is_null());
                    fdc(a);
                    fdr(b);
                }
                // dict == NULL with a non-zero size: `!dict` short-circuits and
                // dictSize is forced back to 0, so nothing is read.
                let a = cac(std::ptr::null(), 4096, dlm, dct, ZSTD_customMem::default());
                let b = car(std::ptr::null(), 4096, dlm, dct, ZSTD_customMem::default());
                eqv(&format!("{what}: NULL+size NULL-ness"), a.is_null(), b.is_null());
                if !a.is_null() {
                    eqv(&format!("{what}: NULL+size dictSize"), dsc(a), dsr(b));
                    fdc(a);
                    fdr(b);
                }
            }
        }

        // ---- row 309: `(!customAlloc) ^ (!customFree)` -> NULL
        for &(with_alloc, with_free) in &[(true, false), (false, true)] {
            let mut st = AllocInj { remaining: i64::MAX, allocs: 0, frees: 0 };
            let cm = ZSTD_customMem {
                customAlloc: if with_alloc { Some(inj_alloc) } else { None },
                customFree: if with_free { Some(inj_free) } else { None },
                opaque: &mut st as *mut AllocInj as *mut c_void,
            };
            let a = cac(good.as_ptr() as *const c_void, good.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cm);
            let b = car(good.as_ptr() as *const c_void, good.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cm);
            assert!(a.is_null(), "row 309: C accepted a half-set customMem");
            assert!(b.is_null(), "row 309: Rust accepted a half-set customMem");
            assert_eq!(st.allocs, 0, "row 309: allocator must not be called");
        }

        // ---- row 310: the ZSTD_DDict allocation itself fails
        for lib in 0..2 {
            let f = if lib == 0 { cac } else { car };
            let mut st = AllocInj { remaining: 0, allocs: 0, frees: 0 };
            let cm = ZSTD_customMem {
                customAlloc: Some(inj_alloc),
                customFree: Some(inj_free),
                opaque: &mut st as *mut AllocInj as *mut c_void,
            };
            let p = f(good.as_ptr() as *const c_void, good.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cm);
            assert!(p.is_null(), "row 310 (lib {lib}): expected NULL");
            assert_eq!(st.allocs, 1, "row 310 (lib {lib}): allocs={}", st.allocs);
            assert_eq!(st.frees, 0, "row 310 (lib {lib}): frees={}", st.frees);
        }

        // ---- row 308: the *internal dict buffer* allocation fails
        // (`ZSTD_dlm_byCopy` + non-empty dict) -> ERROR(memory_allocation)
        // -> row 311 frees the ddict and returns NULL.
        for lib in 0..2 {
            let f = if lib == 0 { cac } else { car };
            let mut st = AllocInj { remaining: 1, allocs: 0, frees: 0 };
            let cm = ZSTD_customMem {
                customAlloc: Some(inj_alloc),
                customFree: Some(inj_free),
                opaque: &mut st as *mut AllocInj as *mut c_void,
            };
            let p = f(good.as_ptr() as *const c_void, good.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cm);
            assert!(p.is_null(), "row 308 (lib {lib}): expected NULL");
            assert_eq!(st.allocs, 2, "row 308 (lib {lib}): allocs={}", st.allocs);
            // ZSTD_freeDDict releases the ddict itself; `dictBuffer` is NULL
            // and `ZSTD_customFree` skips NULL, so exactly one free happens.
            assert_eq!(st.frees, 1, "row 308 (lib {lib}): frees={}", st.frees);
        }

        // a *successful* custom-allocator run, for symmetry
        for lib in 0..2 {
            let (f, fr2) = if lib == 0 { (cac, fdc) } else { (car, fdr) };
            let mut st = AllocInj { remaining: i64::MAX, allocs: 0, frees: 0 };
            let cm = ZSTD_customMem {
                customAlloc: Some(inj_alloc),
                customFree: Some(inj_free),
                opaque: &mut st as *mut AllocInj as *mut c_void,
            };
            let p = f(good.as_ptr() as *const c_void, good.len(), ZSTD_dlm_byCopy, ZSTD_dct_auto, cm);
            assert!(!p.is_null(), "custom alloc success (lib {lib})");
            fr2(p);
            assert_eq!(st.allocs, 2, "custom alloc success (lib {lib}) allocs");
            assert_eq!(st.frees, 2, "custom alloc success (lib {lib}) frees");
        }

        // ZSTD_createDDict / _byReference on the same shapes
        let (c1c, c1r) = duo::<FnCreateDDict>("ZSTD_createDDict");
        let (c2c, c2r) = duo::<FnCreateDDict>("ZSTD_createDDict_byReference");
        for (name, d) in &shapes {
            for (tag, (a_fn, b_fn)) in [("createDDict", (c1c, c1r)), ("byReference", (c2c, c2r))] {
                let what = format!("{tag} {name}");
                let a = a_fn(d.as_ptr() as *const c_void, d.len());
                let b = b_fn(d.as_ptr() as *const c_void, d.len());
                eqv(&format!("{what}: NULL-ness"), a.is_null(), b.is_null());
                if !a.is_null() {
                    eqv(&format!("{what}: dictSize"), dsc(a), dsr(b));
                    eqv(&format!("{what}: dictID"), gic(a), gir(b));
                    eqv(&format!("{what}: sizeof"), soc(a), sor(b));
                    fdc(a);
                    fdr(b);
                }
            }
        }

        // NULL handling of the accessors that document it
        eqv("sizeof_DDict(NULL)", soc(std::ptr::null_mut()), sor(std::ptr::null_mut()));
        eqv(
            "getDictID_fromDDict(NULL)",
            gic(std::ptr::null()),
            gir(std::ptr::null()),
        );
        eqv("freeDDict(NULL)", fdc(std::ptr::null_mut()), fdr(std::ptr::null_mut()));

        let (edc, edr) = duo::<FnEstimateDDictSize>("ZSTD_estimateDDictSize");
        for n in [0usize, 1, 8, 1024, 1 << 20] {
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                eqv(&format!("estimateDDictSize({n},{dlm})"), edc(n, dlm), edr(n, dlm));
            }
        }
    }
}

// rows 312, 313, 314 : ZSTD_initStaticDDict -> NULL

#[test]
fn err_init_static_ddict() {
    unsafe {
        let (ic, ir) = duo::<FnInitStaticDDict>("ZSTD_initStaticDDict");
        let (edc, _) = duo::<FnEstimateDDictSize>("ZSTD_estimateDDictSize");
        let (dsc, dsr) = duo::<FnDDictSize>("ZSTD_DDict_dictSize");
        let (gic, gir) = duo::<FnDDictU32>("ZSTD_getDictID_fromDDict");
        let (ctc, ctr) = duo::<FnDDictContent>("ZSTD_DDict_dictContent");

        let good = real_dict(1024, 0xDD02);
        // over-aligned backing store so we can dial the misalignment in
        let mut ws_c = vec![0x33u8; 64 * 1024];
        let mut ws_r = vec![0x33u8; 64 * 1024];
        let base_c = ws_c.as_mut_ptr();
        let base_r = ws_r.as_mut_ptr();
        let align_c = base_c.align_offset(8);
        let align_r = base_r.align_offset(8);

        let need_copy = edc(good.len(), ZSTD_dlm_byCopy);
        let need_ref = edc(good.len(), ZSTD_dlm_byRef);

        // ---- row 312: sBuffer not 8-aligned
        let mut hit312 = 0usize;
        for off in 1..8usize {
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                let pc = base_c.add(align_c + off) as *mut c_void;
                let pr = base_r.add(align_r + off) as *mut c_void;
                let a = ic(
                    pc,
                    32 * 1024,
                    good.as_ptr() as *const c_void,
                    good.len(),
                    dlm,
                    ZSTD_dct_auto,
                );
                let b = ir(
                    pr,
                    32 * 1024,
                    good.as_ptr() as *const c_void,
                    good.len(),
                    dlm,
                    ZSTD_dct_auto,
                );
                assert!(a.is_null(), "row 312: C accepted a {off}-misaligned buffer");
                assert!(b.is_null(), "row 312: Rust accepted a {off}-misaligned buffer");
                hit312 += 1;
            }
        }
        assert!(hit312 >= 14, "row 312 ({hit312})");
        eqbuf("row 312 workspace untouched", &ws_c, &ws_r);

        // ---- row 313: sBufferSize < neededSpace
        let mut hit313 = 0usize;
        for &(dlm, need) in &[(ZSTD_dlm_byCopy, need_copy), (ZSTD_dlm_byRef, need_ref)] {
            for sz in [0usize, 1, 8, need / 2, need - 1] {
                let pc = base_c.add(align_c) as *mut c_void;
                let pr = base_r.add(align_r) as *mut c_void;
                let a = ic(pc, sz, good.as_ptr() as *const c_void, good.len(), dlm, ZSTD_dct_auto);
                let b = ir(pr, sz, good.as_ptr() as *const c_void, good.len(), dlm, ZSTD_dct_auto);
                assert!(a.is_null(), "row 313: C accepted sBufferSize={sz} < {need}");
                assert!(b.is_null(), "row 313: Rust accepted sBufferSize={sz} < {need}");
                hit313 += 1;
            }
        }
        assert!(hit313 >= 10, "row 313 ({hit313})");
        eqbuf("row 313 workspace untouched", &ws_c, &ws_r);

        // ---- row 314: ZSTD_initDDict_internal fails on a corrupt fullDict
        let magic = ZSTD_MAGIC_DICTIONARY.to_le_bytes();
        let mut bad: Vec<(String, Vec<u8>)> = Vec::new();
        for n in 0..=8usize {
            bad.push((format!("short[{n}]"), good[..n].to_vec()));
        }
        {
            let mut w = good.clone();
            w[0] ^= 0xFF;
            bad.push(("wrong-magic".into(), w));
        }
        {
            let mut v = magic.to_vec();
            v.extend_from_slice(&7u32.to_le_bytes());
            v.resize(64, 0);
            bad.push(("magic+zeros".into(), v));
        }
        {
            let mut v = good.clone();
            v.truncate(24);
            bad.push(("truncated-entropy".into(), v));
        }
        let mut hit314 = 0usize;
        for (name, d) in &bad {
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                let pc = base_c.add(align_c) as *mut c_void;
                let pr = base_r.add(align_r) as *mut c_void;
                let a = ic(
                    pc,
                    32 * 1024,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    dlm,
                    ZSTD_dct_fullDict,
                );
                let b = ir(
                    pr,
                    32 * 1024,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    dlm,
                    ZSTD_dct_fullDict,
                );
                eqv(&format!("initStaticDDict {name} dlm={dlm}: NULL-ness"), a.is_null(), b.is_null());
                if a.is_null() {
                    hit314 += 1;
                }
            }
        }
        assert!(hit314 >= 10, "row 314 ({hit314})");

        // ---- the success shapes must agree byte-for-byte
        for &(dlm, need) in &[(ZSTD_dlm_byCopy, need_copy), (ZSTD_dlm_byRef, need_ref)] {
            for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                for extra in [0usize, 1, 64] {
                    let mut wc = vec![0x77u8; need + 4096];
                    let mut wr = vec![0x77u8; need + 4096];
                    let oc = wc.as_mut_ptr().align_offset(8);
                    let or_ = wr.as_mut_ptr().align_offset(8);
                    let pc = wc.as_mut_ptr().add(oc) as *mut c_void;
                    let pr = wr.as_mut_ptr().add(or_) as *mut c_void;
                    let what = format!("initStaticDDict ok dlm={dlm} dct={dct} extra={extra}");
                    let a = ic(
                        pc,
                        need + extra,
                        good.as_ptr() as *const c_void,
                        good.len(),
                        dlm,
                        dct,
                    );
                    let b = ir(
                        pr,
                        need + extra,
                        good.as_ptr() as *const c_void,
                        good.len(),
                        dlm,
                        dct,
                    );
                    eqv(&format!("{what}: NULL-ness"), a.is_null(), b.is_null());
                    assert!(!a.is_null(), "{what}: expected success");
                    eqv(&format!("{what}: offset"), a as usize - pc as usize, b as usize - pr as usize);
                    eqv(&format!("{what}: dictSize"), dsc(a), dsr(b));
                    eqv(&format!("{what}: dictID"), gic(a), gir(b));
                    let n = dsc(a);
                    if n > 0 {
                        let sa = std::slice::from_raw_parts(ctc(a) as *const u8, n);
                        let sb = std::slice::from_raw_parts(ctr(b) as *const u8, n);
                        eqbuf(&format!("{what}: dictContent"), sa, sb);
                    }
                }
            }
        }

        // dict == NULL / dictSize == 0 (byRef only: byCopy memcpy's from `dict`).
        //
        // NOTE `ZSTD_initStaticDDict` documents `sBuffer != NULL` and
        // `dict != NULL` with plain `assert()`s (zstd_ddict.c L196-197).  This
        // build compiles at DEBUGLEVEL 0, where `common/debug.h` L69-70 turns
        // `assert` into `((void)0)`, so a NULL `sBuffer` is an unchecked
        // precondition violation (`(size_t)sBuffer & 7` passes and the function
        // then writes through it).  Only the `dict == NULL` + `ZSTD_dlm_byRef`
        // shape is legal and is exercised here.
        for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
            let need = edc(0, ZSTD_dlm_byRef);
            let mut wc = vec![0x77u8; need + 64];
            let mut wr = vec![0x77u8; need + 64];
            let oc = wc.as_mut_ptr().align_offset(8);
            let or_ = wr.as_mut_ptr().align_offset(8);
            let pc = wc.as_mut_ptr().add(oc) as *mut c_void;
            let pr = wr.as_mut_ptr().add(or_) as *mut c_void;
            let a = ic(pc, need, std::ptr::null(), 0, ZSTD_dlm_byRef, dct);
            let b = ir(pr, need, std::ptr::null(), 0, ZSTD_dlm_byRef, dct);
            let what = format!("initStaticDDict NULL dict dct={dct}");
            eqv(&format!("{what}: NULL-ness"), a.is_null(), b.is_null());
            if !a.is_null() {
                eqv(&format!("{what}: dictSize"), dsc(a), dsr(b));
                eqv(&format!("{what}: dictID"), gic(a), gir(b));
            }
        }
    }
}

// `ZSTD_copyDDictParameters` (rows 305..307 are its callees) and the
// `ZSTD_DDict_dictContent` / `_dictSize` accessors on every legal DDict shape.

#[test]
fn err_ddict_copy_parameters_and_accessors() {
    unsafe {
        let (cac, car) = duo::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
        let (cpc, cpr) = duo::<FnCopyDDictParams>("ZSTD_copyDDictParameters");
        let (bdc, bdr) = duo::<FnBeginUsingDDict>("ZSTD_decompressBegin_usingDDict");
        let (ctc, ctr) = duo::<FnDDictContent>("ZSTD_DDict_dictContent");
        let (dsc, dsr) = duo::<FnDDictSize>("ZSTD_DDict_dictSize");
        let (dcc, dcr) = duo::<FnDecompressDCtxSimple>("ZSTD_decompressDCtx");
        let (nsc, nsr) = duo::<FnPtrToSize>("ZSTD_nextSrcSizeToDecompress");
        let (cnc, cnr) =
            duo::<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize>(
                "ZSTD_decompressContinue",
            );

        let good = real_dict(1024, 0xDD03);
        let magic = ZSTD_MAGIC_DICTIONARY.to_le_bytes();

        // A frame compressed *with* `good`, so the copied dictID matters.
        let plain = gen_class(4, 4096, 0x1E1E);
        let frame = {
            let (lc, _) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
            let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
            let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
            let cctx = CtxPair::cctx();
            let n = lc(cctx.c, good.as_ptr() as *const c_void, good.len());
            assert!(!is_err(n));
            let cap = bd(plain.len());
            let mut out = vec![0u8; cap];
            let m = c2(
                cctx.c,
                out.as_mut_ptr() as *mut c_void,
                cap,
                plain.as_ptr() as *const c_void,
                plain.len(),
            );
            assert!(!is_err(m), "helper frame compression failed");
            out.truncate(m);
            out
        };

        let mut shapes: Vec<(String, Vec<u8>)> = vec![
            ("good".into(), good.clone()),
            ("empty".into(), Vec::new()),
            ("7 bytes".into(), good[..7].to_vec()),
            ("raw 200".into(), gen_class(5, 200, 0x2222)),
        ];
        {
            let mut w = good.clone();
            w[4] ^= 0xFF; // flip the dictID
            shapes.push(("flipped-dictID".into(), w));
        }
        {
            let mut v = magic.to_vec();
            v.extend_from_slice(&99u32.to_le_bytes());
            v.resize(300, 0xAB);
            shapes.push(("magic+garbage".into(), v));
        }

        for (name, d) in &shapes {
            for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent] {
                let a = cac(
                    d.as_ptr() as *const c_void,
                    d.len(),
                    ZSTD_dlm_byRef,
                    dct,
                    ZSTD_customMem::default(),
                );
                let b = car(
                    d.as_ptr() as *const c_void,
                    d.len(),
                    ZSTD_dlm_byRef,
                    dct,
                    ZSTD_customMem::default(),
                );
                eqv(&format!("copyparams {name} dct={dct} NULL-ness"), a.is_null(), b.is_null());
                if a.is_null() {
                    continue;
                }
                eqv(&format!("copyparams {name} dct={dct} dictSize"), dsc(a), dsr(b));
                eqv(
                    &format!("copyparams {name} dct={dct} content-null"),
                    ctc(a).is_null(),
                    ctr(b).is_null(),
                );

                // ZSTD_copyDDictParameters is `void`; its effect is observed by
                // decoding the frame header afterwards.
                let dctx = CtxPair::dctx();
                cpc(dctx.c, a);
                cpr(dctx.r, b);
                eqv(
                    &format!("copyparams {name} dct={dct} nextSrcSize"),
                    nsc(dctx.c),
                    nsr(dctx.r),
                );

                // ... and through the documented entry point, which calls it.
                let dctx2 = CtxPair::dctx();
                let ra = bdc(dctx2.c, a);
                let rb = bdr(dctx2.r, b);
                eqcode(&format!("beginUsingDDict {name} dct={dct}"), ra, rb);
                if !is_err(ra) {
                    // feed the frame header: a dictID mismatch must be reported
                    // identically by both libraries
                    let hn = nsc(dctx2.c);
                    let take = hn.min(frame.len());
                    let mut oa = vec![0u8; 64];
                    let mut ob = vec![0u8; 64];
                    let a2 = cnc(
                        dctx2.c,
                        oa.as_mut_ptr() as *mut c_void,
                        oa.len(),
                        frame.as_ptr() as *const c_void,
                        take,
                    );
                    let b2 = cnr(
                        dctx2.r,
                        ob.as_mut_ptr() as *mut c_void,
                        ob.len(),
                        frame.as_ptr() as *const c_void,
                        take,
                    );
                    eqcode(&format!("continue-after-DDict {name} dct={dct}"), a2, b2);
                    eqbuf(&format!("continue-after-DDict {name} dct={dct} out"), &oa, &ob);
                }

                // full decode through ZSTD_decompress_usingDDict
                let (udc, udr) = duo::<
                    unsafe extern "C" fn(
                        *mut c_void,
                        *mut c_void,
                        usize,
                        *const c_void,
                        usize,
                        *const c_void,
                    ) -> usize,
                >("ZSTD_decompress_usingDDict");
                let dctx3 = CtxPair::dctx();
                let mut oa = vec![0xA5u8; plain.len() + 16];
                let mut ob = vec![0xA5u8; plain.len() + 16];
                let a3 = udc(
                    dctx3.c,
                    oa.as_mut_ptr() as *mut c_void,
                    oa.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    a,
                );
                let b3 = udr(
                    dctx3.r,
                    ob.as_mut_ptr() as *mut c_void,
                    ob.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    b,
                );
                eqcode(&format!("decompress_usingDDict {name} dct={dct}"), a3, b3);
                eqbuf(&format!("decompress_usingDDict {name} dct={dct} out"), &oa, &ob);

                fdc(a);
                fdr(b);
            }
        }

        // a DDict over a NULL dictionary: dictContent is NULL, dictSize is 0,
        // and copyDDictParameters must still be well-behaved.
        {
            let a = cac(
                std::ptr::null(),
                0,
                ZSTD_dlm_byRef,
                ZSTD_dct_auto,
                ZSTD_customMem::default(),
            );
            let b = car(
                std::ptr::null(),
                0,
                ZSTD_dlm_byRef,
                ZSTD_dct_auto,
                ZSTD_customMem::default(),
            );
            assert!(!a.is_null() && !b.is_null());
            eqv("NULL-ddict dictSize", dsc(a), dsr(b));
            eqv("NULL-ddict content-null", ctc(a).is_null(), ctr(b).is_null());
            let dctx = CtxPair::dctx();
            cpc(dctx.c, a);
            cpr(dctx.r, b);
            eqv("NULL-ddict nextSrcSize", nsc(dctx.c), nsr(dctx.r));
            let mut oa = vec![0xA5u8; 64];
            let mut ob = vec![0xA5u8; 64];
            let ra = dcc(
                dctx.c,
                oa.as_mut_ptr() as *mut c_void,
                oa.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
            );
            let rb = dcr(
                dctx.r,
                ob.as_mut_ptr() as *mut c_void,
                ob.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
            );
            eqcode("NULL-ddict decompressDCtx", ra, rb);
            fdc(a);
            fdr(b);
        }
    }
}

// ------------------------------------------------------------- fuzzing pass

/// Randomized corruption of a real `ZDICT_trainFromBuffer` dictionary, fed to
/// every dictionary entry point.  Fixed seed, several hundred mutations.
#[test]
fn err_dictionary_corruption_fuzz() {
    unsafe {
        let (idc, idr) = duo::<FnGetDictID>("ZDICT_getDictID");
        let (hc, hr) = duo::<FnGetHdrSize>("ZDICT_getDictHeaderSize");
        let (gdc, gdr) = duo::<FnGetDictIDFromDict>("ZSTD_getDictID_fromDict");
        let (cac, car) = duo::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let (c1c, c1r) = duo::<FnCreateDDict>("ZSTD_createDDict");
        let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
        let (dsc, dsr) = duo::<FnDDictSize>("ZSTD_DDict_dictSize");
        let (gic, gir) = duo::<FnDDictU32>("ZSTD_getDictID_fromDDict");
        let (ctc, ctr) = duo::<FnDDictContent>("ZSTD_DDict_dictContent");
        let (clc, clr) = duo::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let (clac, clar) = duo::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced");
        let (dlc, dlr) = duo::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        let (dlac, dlar) = duo::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (rsc, rsr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (drc, drr) = duo::<FnReset>("ZSTD_DCtx_reset");
        let (ddc, ddr) = duo::<FnDecompressDCtxSimple>("ZSTD_decompressDCtx");
        let (ccc, ccr) =
            duo::<unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void>(
                "ZSTD_createCDict",
            );
        let (ccac, ccar) = duo::<
            unsafe extern "C" fn(
                *const c_void,
                usize,
                c_int,
                c_int,
                ZSTD_compressionParameters,
                ZSTD_customMem,
            ) -> *mut c_void,
        >("ZSTD_createCDict_advanced");
        let (fcc, fcr) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (gcc, gcr) = duo::<FnDDictU32>("ZSTD_getDictID_fromCDict");
        let (szc, szr) = duo::<FnPtrToSize>("ZSTD_sizeof_CDict");
        let (gcp, _) = duo::<
            unsafe extern "C" fn(c_int, u64, usize) -> ZSTD_compressionParameters,
        >("ZSTD_getCParams");
        let (rdc, rdr) =
            duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>("ZSTD_DCtx_refDDict");
        let (bdc, bdr) = duo::<
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize,
        >("ZSTD_decompressBegin_usingDict");
        let (nsc2, nsr2) = duo::<FnPtrToSize>("ZSTD_nextSrcSizeToDecompress");

        // three independently trained dictionaries of different sizes
        let bases: Vec<Vec<u8>> = vec![
            real_dict(2048, 0xF0F0),
            real_dict(512, 0xF1F1),
            real_dict(4096, 0xF2F2),
        ];
        for b in &bases {
            assert!(b.len() > 64);
        }

        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let src = gen_class(4, 512, 0x3141);
        let frame = c_compress(&src, 1);

        let mut rng = Rng::new(0xDEAD_BEEF_1234_5678);
        let mut n_corrupt_hdr = 0usize;
        let mut n_ddict_null = 0usize;
        let mut n_load_err = 0usize;
        let mut n_cdict_null = 0usize;

        for iter in 0..900usize {
            let base = &bases[iter % bases.len()];
            let mut d = base.clone();
            // 1..=4 mutations, occasionally a truncation
            let nmut = 1 + rng.below(4);
            for _ in 0..nmut {
                let kind = rng.below(7);
                let pos = rng.below(d.len());
                match kind {
                    0 => {
                        let b = rng.byte();
                        d[pos] = b;
                    }
                    1 => {
                        let bit = rng.below(8);
                        d[pos] ^= 1u8 << bit;
                    }
                    2 => {
                        // bias towards the header bytes, which drive the
                        // magic / dictID / entropy-table decisions
                        let p2 = rng.below(40.min(d.len()));
                        let b = rng.byte();
                        d[p2] = b;
                    }
                    3 => {
                        // zero out a run
                        let len = 1 + rng.below(32);
                        let end = (pos + len).min(d.len());
                        for x in &mut d[pos..end] {
                            *x = 0;
                        }
                    }
                    4 => {
                        // splice: duplicate a run over another
                        let len = 1 + rng.below(48);
                        let from = rng.below(d.len());
                        for i in 0..len {
                            if pos + i >= d.len() || from + i >= d.len() {
                                break;
                            }
                            d[pos + i] = d[from + i];
                        }
                    }
                    5 => {
                        // prepend garbage, shifting the whole header
                        let len = 1 + rng.below(8);
                        let g = rng.bytes(len);
                        let mut v = g;
                        v.extend_from_slice(&d);
                        d = v;
                    }
                    _ => {
                        let keep = rng.below(d.len());
                        d.truncate(keep);
                        if d.is_empty() {
                            d.push(0);
                        }
                    }
                }
            }
            if iter % 7 == 0 {
                let keep = 1 + rng.below(d.len());
                d.truncate(keep);
            }

            let p = d.as_ptr() as *const c_void;
            let n = d.len();
            let tag = format!("fuzz[{iter}] len={n}");

            // pure introspection
            eqv(&format!("{tag} ZDICT_getDictID"), idc(p, n), idr(p, n));
            eqv(&format!("{tag} ZSTD_getDictID_fromDict"), gdc(p, n), gdr(p, n));
            let a = hc(p, n);
            let b = hr(p, n);
            eqcode(&format!("{tag} ZDICT_getDictHeaderSize"), a, b);
            if is_err(a) {
                n_corrupt_hdr += 1;
            }

            // DDict construction over every load method / content type
            for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let x = cac(p, n, dlm, dct, ZSTD_customMem::default());
                    let y = car(p, n, dlm, dct, ZSTD_customMem::default());
                    eqv(&format!("{tag} createDDict dlm={dlm} dct={dct}"), x.is_null(), y.is_null());
                    if x.is_null() {
                        n_ddict_null += 1;
                        continue;
                    }
                    eqv(&format!("{tag} ddict dictSize"), dsc(x), dsr(y));
                    eqv(&format!("{tag} ddict dictID"), gic(x), gir(y));
                    let m = dsc(x);
                    if m > 0 {
                        let sa = std::slice::from_raw_parts(ctc(x) as *const u8, m);
                        let sb = std::slice::from_raw_parts(ctr(y) as *const u8, m);
                        eqbuf(&format!("{tag} ddict content"), sa, sb);
                    }
                    fdc(x);
                    fdr(y);
                }
            }
            let x = c1c(p, n);
            let y = c1r(p, n);
            eqv(&format!("{tag} ZSTD_createDDict"), x.is_null(), y.is_null());
            if !x.is_null() {
                eqv(&format!("{tag} createDDict dictID"), gic(x), gir(y));
                // ZSTD_DCtx_refDDict must accept/reject identically
                eqcode(
                    &format!("{tag} DCtx_reset before refDDict"),
                    drc(dctx.c, ZSTD_reset_session_and_parameters),
                    drr(dctx.r, ZSTD_reset_session_and_parameters),
                );
                eqcode(&format!("{tag} DCtx_refDDict"), rdc(dctx.c, x), rdr(dctx.r, y));
                fdc(x);
                fdr(y);
            }

            // CDict construction over the same shapes
            {
                let cp = gcp(3, 0, n as u64 as usize);
                for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                    let x = ccac(p, n, ZSTD_dlm_byRef, dct, cp, ZSTD_customMem::default());
                    let y = ccar(p, n, ZSTD_dlm_byRef, dct, cp, ZSTD_customMem::default());
                    eqv(&format!("{tag} createCDict_advanced dct={dct}"), x.is_null(), y.is_null());
                    if x.is_null() {
                        n_cdict_null += 1;
                        continue;
                    }
                    eqv(&format!("{tag} cdict dictID dct={dct}"), gcc(x), gcr(y));
                    eqv(&format!("{tag} cdict sizeof dct={dct}"), szc(x), szr(y));
                    fcc(x);
                    fcr(y);
                }
                let x = ccc(p, n, 3);
                let y = ccr(p, n, 3);
                eqv(&format!("{tag} createCDict"), x.is_null(), y.is_null());
                if !x.is_null() {
                    eqv(&format!("{tag} createCDict dictID"), gcc(x), gcr(y));
                    fcc(x);
                    fcr(y);
                }
            }

            // bufferless decoder dictionary load
            {
                let dctx2 = CtxPair::dctx();
                eqcode(
                    &format!("{tag} decompressBegin_usingDict"),
                    bdc(dctx2.c, p, n),
                    bdr(dctx2.r, p, n),
                );
                eqv(
                    &format!("{tag} nextSrcSize after begin_usingDict"),
                    nsc2(dctx2.c),
                    nsr2(dctx2.r),
                );
            }

            // compression side: the dictionary is digested at compress time
            for &(adv, dct) in &[
                (false, ZSTD_dct_auto),
                (true, ZSTD_dct_auto),
                (true, ZSTD_dct_rawContent),
                (true, ZSTD_dct_fullDict),
            ] {
                eqcode(
                    &format!("{tag} CCtx_reset"),
                    rsc(cctx.c, ZSTD_reset_session_and_parameters),
                    rsr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                let (la, lb) = if adv {
                    (clac(cctx.c, p, n, ZSTD_dlm_byRef, dct), clar(cctx.r, p, n, ZSTD_dlm_byRef, dct))
                } else {
                    (clc(cctx.c, p, n), clr(cctx.r, p, n))
                };
                eqcode(&format!("{tag} CCtx_loadDictionary adv={adv} dct={dct}"), la, lb);
                let mut oa = vec![0xA5u8; 4096];
                let mut ob = vec![0xA5u8; 4096];
                let ca = c2c(
                    cctx.c,
                    oa.as_mut_ptr() as *mut c_void,
                    oa.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                let cb = c2r(
                    cctx.r,
                    ob.as_mut_ptr() as *mut c_void,
                    ob.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                eqcode(&format!("{tag} compress2 adv={adv} dct={dct}"), ca, cb);
                if !is_err(ca) {
                    eqbuf(&format!("{tag} compress2 out adv={adv} dct={dct}"), &oa[..ca], &ob[..cb]);
                } else {
                    n_load_err += 1;
                }
            }

            // decompression side
            for &(adv, dct) in &[
                (false, ZSTD_dct_auto),
                (true, ZSTD_dct_auto),
                (true, ZSTD_dct_rawContent),
                (true, ZSTD_dct_fullDict),
            ] {
                eqcode(
                    &format!("{tag} DCtx_reset"),
                    drc(dctx.c, ZSTD_reset_session_and_parameters),
                    drr(dctx.r, ZSTD_reset_session_and_parameters),
                );
                let (la, lb) = if adv {
                    (dlac(dctx.c, p, n, ZSTD_dlm_byRef, dct), dlar(dctx.r, p, n, ZSTD_dlm_byRef, dct))
                } else {
                    (dlc(dctx.c, p, n), dlr(dctx.r, p, n))
                };
                eqcode(&format!("{tag} DCtx_loadDictionary adv={adv} dct={dct}"), la, lb);
                if is_err(la) {
                    n_load_err += 1;
                    continue;
                }
                let mut oa = vec![0xA5u8; src.len() + 64];
                let mut ob = vec![0xA5u8; src.len() + 64];
                let da = ddc(
                    dctx.c,
                    oa.as_mut_ptr() as *mut c_void,
                    oa.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                let db = ddr(
                    dctx.r,
                    ob.as_mut_ptr() as *mut c_void,
                    ob.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                eqcode(&format!("{tag} decompressDCtx adv={adv} dct={dct}"), da, db);
                eqbuf(&format!("{tag} decompressDCtx out adv={adv} dct={dct}"), &oa, &ob);
            }
        }

        assert!(
            n_corrupt_hdr >= 10,
            "fuzz: ZDICT_getDictHeaderSize never reported dictionary_corrupted ({n_corrupt_hdr})"
        );
        assert!(
            n_ddict_null >= 10,
            "fuzz: ZSTD_createDDict_advanced never rejected a corrupt dictionary ({n_ddict_null})"
        );
        assert!(n_load_err >= 5, "fuzz: no load/compress rejection observed ({n_load_err})");
        assert!(
            n_cdict_null >= 5,
            "fuzz: ZSTD_createCDict_advanced never rejected a corrupt dictionary ({n_cdict_null})"
        );
    }
}
