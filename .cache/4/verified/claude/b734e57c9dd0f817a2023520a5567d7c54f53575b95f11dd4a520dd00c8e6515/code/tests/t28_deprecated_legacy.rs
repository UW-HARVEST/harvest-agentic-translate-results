//! Phase B (valid path) + Phase C (error path) for the two "attic" areas of the
//! library:
//!
//!  * **A. the deprecated `ZBUFF_*` streaming API** (`c_src/src/deprecated/`).
//!    Every `ZBUFF_*` entry point is a thin forward to the modern
//!    `ZSTD_*CStream` / `ZSTD_*DStream` API, so the interesting part is (a) the
//!    exact forwarding, (b) the `size_t*` in/out parameters that are written
//!    back with `outBuff.pos` / `inBuff.pos`, and (c) two documented quirks:
//!    `ZBUFF_compressInit_advanced` maps `pledgedSrcSize == 0` to
//!    `ZSTD_CONTENTSIZE_UNKNOWN` and passes `fParams.noDictIDFlag` straight
//!    into `ZSTD_c_dictIDFlag` (an inverted polarity).
//!
//!  * **B. the legacy `ZSTDv0x_*` decoders** (`c_src/src/legacy/`, built with
//!    `ZSTD_LEGACY_SUPPORT=5`).
//!
//! # SAFETY POLICY FOR THE LEGACY DECODERS
//!
//! The v0.1..v0.7 decoders predate zstd's fuzzing hardening. Feeding them
//! arbitrary bytes is **undefined behaviour in the reference C**: the HUF/FSE
//! bitstream readers do unchecked pointer arithmetic and the reference build
//! segfaults. There is therefore no C behaviour for the Rust to match and such
//! a test would be worthless as well as flaky.
//!
//! Every legacy call in this file is restricted to an input that has been
//! verified *by reading the C* to be rejected (or fully consumed) by an
//! explicit size/magic check **before** any bitstream walking. Each test
//! doc-comment names the guard and the `c_src` line it lives on. The complete
//! list of excluded entry points, with the evidence, is at the bottom of this
//! file (`mod exclusions`).
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// Constants resolved from the C sources
// ---------------------------------------------------------------------------

/// `ZSTD_BLOCKSIZE_MAX` == `ZBUFF_recommendedCInSize()` == `ZSTD_CStreamInSize()`.
const BLOCKSIZE_MAX: usize = 128 * 1024;
/// `ZBUFF_recommendedDInSize()` == `ZSTD_DStreamInSize()` == `128 KB + 3`.
const DSTREAM_IN_SIZE: usize = BLOCKSIZE_MAX + 3;

// Legacy magic numbers, as the little-endian U32 that `ZSTD_isLegacy` switches
// on (`c_src/src/legacy/zstd_legacy.h:60`).  Note v0.1 is the odd one out: the
// frame stores the magic **big-endian**, so the LE32 read of the first four
// bytes is `0x1EB52FFD` (`zstd_v01.h:86-87`).
const MAGIC_V01_LE: u32 = 0x1EB5_2FFD;
const MAGIC_V02: u32 = 0xFD2F_B522;
const MAGIC_V03: u32 = 0xFD2F_B523;
const MAGIC_V04: u32 = 0xFD2F_B524;
const MAGIC_V05: u32 = 0xFD2F_B525;
const MAGIC_V06: u32 = 0xFD2F_B526;
const MAGIC_V07: u32 = 0xFD2F_B527;

/// `(label, LE32 first word)` for every prefix the dispatch tests sweep.
const MAGICS: &[(&str, u32)] = &[
    ("v01", MAGIC_V01_LE),
    ("v02", MAGIC_V02),
    ("v03", MAGIC_V03),
    ("v04", MAGIC_V04),
    ("v05", MAGIC_V05),
    ("v06", MAGIC_V06),
    ("v07", MAGIC_V07),
    ("zero", 0x0000_0000),
    ("ones", 0xFFFF_FFFF),
    ("zstd1", ZSTD_MAGICNUMBER),
    ("dict", ZSTD_MAGIC_DICTIONARY),
    ("skip0", ZSTD_MAGIC_SKIPPABLE_START),
];

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

type FnS0 = unsafe extern "C" fn() -> SizeT;
type FnP0 = unsafe extern "C" fn() -> *mut c_void;
type FnPCustom = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnS1p = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnV1p = unsafe extern "C" fn(*mut c_void);
type FnI1p = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnS1pc = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnSpi = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnSpdi = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnSpd = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnZbAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, ZSTD_parameters, c_ulonglong) -> SizeT;
type FnZbCont =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut SizeT, *const c_void, *mut SizeT) -> SizeT;
type FnZbFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut SizeT) -> SizeT;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnFsi = unsafe extern "C" fn(*const c_void, SizeT, *mut SizeT, *mut c_ulonglong);
type FnGfp = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnDec4 = unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnDecDCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnDecDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;
type FnDecPrepared =
    unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnDecDDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT, *const c_void) -> SizeT;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnCreateDTable = unsafe extern "C" fn(c_uint) -> *mut c_void;
type FnBuildRaw = unsafe extern "C" fn(*mut c_void, c_uint) -> SizeT;
type FnBuildRle = unsafe extern "C" fn(*mut c_void, u8) -> SizeT;
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnReadStats = unsafe extern "C" fn(
    *mut u8,
    SizeT,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    SizeT,
) -> SizeT;
type FnSelectDecoder = unsafe extern "C" fn(SizeT, SizeT) -> c_uint;
type FnHufDCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnHufReadDTable = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnUll2 = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;
type FnU0Buf = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn poison(n: usize) -> Vec<u8> {
    vec![0x5Au8; n]
}

/// Append one fixed-width (8 bytes/value) record to a call trace.  Fixed width
/// is deliberate: when `diff_bytes` reports "first differing byte at index i",
/// `i / (8 * fields)` is the index of the diverging call.
fn rec(t: &mut Vec<u8>, vals: &[usize]) {
    for v in vals {
        t.extend_from_slice(&(*v as u64).to_le_bytes());
    }
}

fn zbuff_is_err(l: &Lib, code: SizeT) -> bool {
    let f = l.sym::<FnIsError>("ZBUFF_isError");
    unsafe { f(code) != 0 }
}

/// Every code an `isError`-style predicate must classify: the whole
/// `1..=maxCode` neighbourhood on the negative side plus the extremes.
fn error_code_sweep() -> Vec<SizeT> {
    let mut v: Vec<SizeT> = Vec::new();
    for i in 0..=130usize {
        v.push(i);
    }
    for i in 0..=130usize {
        v.push(0usize.wrapping_sub(i));
    }
    v.extend_from_slice(&[
        4096,
        65535,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
        0x7fff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0usize.wrapping_sub(119),
        0usize.wrapping_sub(120),
        0usize.wrapping_sub(121),
    ]);
    v
}

/// `isError` + `getErrorName` for one per-translation-unit copy of
/// `ERR_isError`/`ERR_getErrorName`.  `name` may be absent (v01..v04 export
/// only `isError`), in which case only the predicate is compared.
fn check_err_family(label: &str, is_error: &str, get_name: Option<&str>) {
    let codes = error_code_sweep();
    diff_bytes(label, |l| {
        let f = l.sym::<FnIsError>(is_error);
        let g = get_name.map(|n| l.sym::<FnGetErrorName>(n));
        let mut out: Vec<u8> = Vec::new();
        for &c in &codes {
            out.push(unsafe { f(c) } as u8);
            out.push((unsafe { f(c) } >> 8) as u8);
            if let Some(g) = &g {
                let s = unsafe { cstr(g(c)) };
                out.extend_from_slice(s.as_bytes());
            }
            out.push(0);
        }
        Blob(out)
    });
}

/// A 4-byte little-endian prefix followed by `n - 4` zero bytes (or a short
/// truncation of the prefix itself when `n < 4`).
fn magic_buf(magic: u32, n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    let m = magic.to_le_bytes();
    for i in 0..n.min(4) {
        v[i] = m[i];
    }
    v
}

// ---------------------------------------------------------------------------
// A counting allocator, so the `*_advanced` customMem paths are observable
// ---------------------------------------------------------------------------

/// `(0 = alloc, 1 = free, requested size)` in call order.
static ALLOC_LOG: std::sync::Mutex<Vec<(u8, usize)>> = std::sync::Mutex::new(Vec::new());
/// Header prepended to every block so `counting_free` can recover the size
/// (Rust's global allocator needs the layout back).
const AHDR: usize = 16;

extern "C" fn counting_alloc(_opaque: *mut c_void, size: SizeT) -> *mut c_void {
    ALLOC_LOG.lock().unwrap().push((0, size));
    unsafe {
        let layout = std::alloc::Layout::from_size_align(size + AHDR, AHDR).unwrap();
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut usize).write(size);
        p.add(AHDR) as *mut c_void
    }
}

extern "C" fn counting_free(_opaque: *mut c_void, addr: *mut c_void) {
    if addr.is_null() {
        ALLOC_LOG.lock().unwrap().push((1, usize::MAX));
        return;
    }
    unsafe {
        let p = (addr as *mut u8).sub(AHDR);
        let size = (p as *mut usize).read();
        ALLOC_LOG.lock().unwrap().push((1, size));
        let layout = std::alloc::Layout::from_size_align(size + AHDR, AHDR).unwrap();
        std::alloc::dealloc(p, layout);
    }
}

fn counting_mem() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(counting_alloc),
        customFree: Some(counting_free),
        opaque: std::ptr::null_mut(),
    }
}

fn take_alloc_log() -> Vec<(u8, usize)> {
    let mut g = ALLOC_LOG.lock().unwrap();
    std::mem::take(&mut *g)
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A genuine zstd dictionary (with `ZSTD_MAGIC_DICTIONARY` and a dictID),
/// produced once by the **C** library so both libraries are handed the exact
/// same bytes.
fn c_dict() -> &'static Vec<u8> {
    static D: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        type FnFinalize = unsafe extern "C" fn(
            *mut c_void,
            SizeT,
            *const c_void,
            SizeT,
            *const c_void,
            *const SizeT,
            c_uint,
            ZDICT_params_t,
        ) -> SizeT;
        let l = &pair().c;
        let f = l.sym::<FnFinalize>("ZDICT_finalizeDictionary");
        let content = corpus(Corpus::Text, 4096, 0xD1C7);
        let nb = 64usize;
        let each = 256usize;
        let samples = corpus(Corpus::Text, nb * each, 0x5A_9101);
        let sizes: Vec<SizeT> = vec![each; nb];
        let mut out = vec![0u8; 4096];
        let n = unsafe {
            f(
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                content.as_ptr() as *const c_void,
                content.len(),
                samples.as_ptr() as *const c_void,
                sizes.as_ptr(),
                nb as c_uint,
                ZDICT_params_t { compressionLevel: 3, notificationLevel: 0, dictID: 0x1234 },
            )
        };
        assert!(
            !is_error(l, n),
            "C-side dictionary fixture failed: {:?}",
            res(l, n)
        );
        out.truncate(n);
        assert_eq!(
            u32::from_le_bytes(out[..4].try_into().unwrap()),
            ZSTD_MAGIC_DICTIONARY
        );
        out
    })
}

/// 4096 bytes of raw dictionary content (no magic), so `loadDictionary` takes
/// its `ZSTD_dct_rawContent` branch.
fn raw_dict() -> Vec<u8> {
    corpus(Corpus::Random, 4096, 0x0BAD_0D1C)
}

// ---------------------------------------------------------------------------
// ZBUFF compression / decompression drivers
// ---------------------------------------------------------------------------

/// Bound on the number of `*Continue`/`*Flush`/`*End` calls per case; recorded
/// in the trace so hitting it is itself a compared observation.
const MAX_CALLS: usize = 4_000_000;

#[derive(Copy, Clone, Debug)]
enum Dict<'a> {
    None,
    /// `ZBUFF_compressInitDictionary` / `ZBUFF_decompressInitDictionary`.
    Buf(&'a [u8]),
    /// `ZBUFF_compressInit_advanced(zbc, dict, dictSize, params, pledged)`.
    Adv(&'a [u8], ZSTD_parameters, c_ulonglong),
}

/// Full `ZBUFF_compressInit* / compressContinue* / compressFlush* / compressEnd*`
/// cycle.  Returns `(nb_calls, trace ++ 0xFF sentinel ++ compressed bytes)`.
fn zbuff_compress(
    l: &Lib,
    src: &[u8],
    level: c_int,
    dict: Dict<'_>,
    in_chunk: usize,
    out_chunk: usize,
) -> (usize, Blob) {
    let create = l.sym::<FnP0>("ZBUFF_createCCtx");
    let free = l.sym::<FnS1p>("ZBUFF_freeCCtx");
    let cont = l.sym::<FnZbCont>("ZBUFF_compressContinue");
    let flush = l.sym::<FnZbFlush>("ZBUFF_compressFlush");
    let endf = l.sym::<FnZbFlush>("ZBUFF_compressEnd");

    let zbc = unsafe { create() };
    assert!(!zbc.is_null(), "[{}] ZBUFF_createCCtx returned NULL", l.tag);

    let mut trace: Vec<u8> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut calls = 0usize;

    let init = match dict {
        Dict::None => {
            let f = l.sym::<FnSpi>("ZBUFF_compressInit");
            unsafe { f(zbc, level) }
        }
        Dict::Buf(d) => {
            let f = l.sym::<FnSpdi>("ZBUFF_compressInitDictionary");
            unsafe { f(zbc, d.as_ptr() as *const c_void, d.len(), level) }
        }
        Dict::Adv(d, params, pledged) => {
            let f = l.sym::<FnZbAdv>("ZBUFF_compressInit_advanced");
            unsafe { f(zbc, d.as_ptr() as *const c_void, d.len(), params, pledged) }
        }
    };
    rec(&mut trace, &[init, 0, 0]);

    if !zbuff_is_err(l, init) {
        let mut dst = vec![0xCDu8; out_chunk.max(1)];
        // ---- Continue: consume `src` in `in_chunk`-sized pieces ------------
        let mut pos = 0usize;
        let mut first = true;
        while pos < src.len() || first {
            first = false;
            let mut got = in_chunk.min(src.len() - pos);
            let mut cap = out_chunk;
            dst.iter_mut().for_each(|b| *b = 0xCD);
            let r = unsafe {
                cont(
                    zbc,
                    dst.as_mut_ptr() as *mut c_void,
                    &mut cap,
                    src[pos..].as_ptr() as *const c_void,
                    &mut got,
                )
            };
            calls += 1;
            rec(&mut trace, &[r, cap, got]);
            if zbuff_is_err(l, r) {
                break;
            }
            out.extend_from_slice(&dst[..cap]);
            pos += got;
            if got == 0 && cap == 0 {
                break; // no forward progress possible
            }
            if calls > MAX_CALLS {
                break;
            }
        }
        // ---- Flush until the internal buffer reports empty ----------------
        for phase in 0..2 {
            let f: &libloading::Symbol<'_, FnZbFlush> = if phase == 0 { &flush } else { &endf };
            loop {
                let mut cap = out_chunk;
                dst.iter_mut().for_each(|b| *b = 0xCD);
                let r = unsafe { f(zbc, dst.as_mut_ptr() as *mut c_void, &mut cap) };
                calls += 1;
                rec(&mut trace, &[r, cap, phase]);
                if zbuff_is_err(l, r) {
                    break;
                }
                out.extend_from_slice(&dst[..cap]);
                if r == 0 {
                    break;
                }
                if cap == 0 {
                    break; // dst too small to make progress
                }
                if calls > MAX_CALLS {
                    break;
                }
            }
        }
    }
    let fr = unsafe { free(zbc) };
    rec(&mut trace, &[fr, 0, 0]);

    trace.extend_from_slice(&[0xFFu8; 8]);
    trace.extend_from_slice(&out);
    (calls, Blob(trace))
}

/// Full `ZBUFF_decompressInit* / decompressContinue*` cycle.
fn zbuff_decompress(
    l: &Lib,
    cframe: &[u8],
    dict: Dict<'_>,
    in_chunk: usize,
    out_chunk: usize,
) -> (usize, Blob) {
    let create = l.sym::<FnP0>("ZBUFF_createDCtx");
    let free = l.sym::<FnS1p>("ZBUFF_freeDCtx");
    let cont = l.sym::<FnZbCont>("ZBUFF_decompressContinue");

    let zbd = unsafe { create() };
    assert!(!zbd.is_null(), "[{}] ZBUFF_createDCtx returned NULL", l.tag);

    let mut trace: Vec<u8> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut calls = 0usize;

    let init = match dict {
        Dict::None => {
            let f = l.sym::<FnS1p>("ZBUFF_decompressInit");
            unsafe { f(zbd) }
        }
        Dict::Buf(d) => {
            let f = l.sym::<FnSpd>("ZBUFF_decompressInitDictionary");
            unsafe { f(zbd, d.as_ptr() as *const c_void, d.len()) }
        }
        Dict::Adv(..) => unreachable!("no advanced init on the ZBUFF decompression side"),
    };
    rec(&mut trace, &[init, 0, 0]);

    if !zbuff_is_err(l, init) {
        let mut dst = vec![0xCDu8; out_chunk.max(1)];
        let mut pos = 0usize;
        let mut first = true;
        loop {
            if pos >= cframe.len() && !first {
                break;
            }
            first = false;
            let mut got = in_chunk.min(cframe.len() - pos);
            let mut cap = out_chunk;
            dst.iter_mut().for_each(|b| *b = 0xCD);
            let r = unsafe {
                cont(
                    zbd,
                    dst.as_mut_ptr() as *mut c_void,
                    &mut cap,
                    cframe[pos..].as_ptr() as *const c_void,
                    &mut got,
                )
            };
            calls += 1;
            rec(&mut trace, &[r, cap, got]);
            if zbuff_is_err(l, r) {
                break;
            }
            out.extend_from_slice(&dst[..cap]);
            pos += got;
            if r == 0 {
                break; // frame decoded and fully flushed
            }
            if got == 0 && cap == 0 {
                break;
            }
            if calls > MAX_CALLS {
                break;
            }
        }
    }
    let fr = unsafe { free(zbd) };
    rec(&mut trace, &[fr, 0, 0]);

    trace.extend_from_slice(&[0xFFu8; 8]);
    trace.extend_from_slice(&out);
    (calls, Blob(trace))
}

// ===========================================================================
// A. THE DEPRECATED ZBUFF API
// ===========================================================================

/// `deprecated/zbuff.h` declares exactly 21 entry points in zstd 1.5.7.
/// `ZBUFF_maxCLevel` was removed in an earlier release: it is in neither
/// `SYMBOLS.md`, `c_symbols.txt` nor `zbuff.h`, so its *absence* from both
/// shared objects is the assertion (CONFIGS row 400).
#[test]
fn zbuff_symbol_surface() {
    covers(&["CFG:400"]);
    const PRESENT: &[&str] = &[
        "ZBUFF_isError",
        "ZBUFF_getErrorName",
        "ZBUFF_createCCtx",
        "ZBUFF_createCCtx_advanced",
        "ZBUFF_freeCCtx",
        "ZBUFF_compressInit",
        "ZBUFF_compressInit_advanced",
        "ZBUFF_compressInitDictionary",
        "ZBUFF_compressContinue",
        "ZBUFF_compressFlush",
        "ZBUFF_compressEnd",
        "ZBUFF_recommendedCInSize",
        "ZBUFF_recommendedCOutSize",
        "ZBUFF_createDCtx",
        "ZBUFF_createDCtx_advanced",
        "ZBUFF_freeDCtx",
        "ZBUFF_decompressInit",
        "ZBUFF_decompressInitDictionary",
        "ZBUFF_decompressContinue",
        "ZBUFF_recommendedDInSize",
        "ZBUFF_recommendedDOutSize",
    ];
    const ABSENT: &[&str] = &[
        "ZBUFF_maxCLevel",
        "ZBUFF_compressInitDictionary_advanced",
        "ZBUFF_decompressInit_advanced",
        "ZBUFF_freeCDict",
        "ZBUFF_compressBound",
    ];
    let p = pair();
    for n in PRESENT {
        assert!(p.c.has(n), "C .so is missing `{n}`");
        assert!(p.r.has(n), "Rust .so is missing `{n}`");
    }
    for n in ABSENT {
        assert!(!p.c.has(n), "C .so unexpectedly exports `{n}`");
        assert!(!p.r.has(n), "Rust .so unexpectedly exports `{n}`");
    }
    // `ZBUFF_maxCLevel` never existed; the replacement is `ZSTD_maxCLevel()`.
    let m = diff("ZSTD_maxCLevel", |l| unsafe { l.sym::<FnMaxCLevel>("ZSTD_maxCLevel")() });
    assert_eq!(m, 22, "zstd 1.5.7 caps the compression level at 22");
}

/// `ZBUFF_recommended{C,D}{In,Out}Size` — direct forwards to
/// `ZSTD_{C,D}Stream{In,Out}Size` (`zbuff_compress.c:166-167`,
/// `zbuff_decompress.c:76-77`).  CONFIGS row 388.
#[test]
fn zbuff_recommended_sizes() {
    covers(&["CFG:388"]);
    let pairs: &[(&str, &str)] = &[
        ("ZBUFF_recommendedCInSize", "ZSTD_CStreamInSize"),
        ("ZBUFF_recommendedCOutSize", "ZSTD_CStreamOutSize"),
        ("ZBUFF_recommendedDInSize", "ZSTD_DStreamInSize"),
        ("ZBUFF_recommendedDOutSize", "ZSTD_DStreamOutSize"),
    ];
    for (zb, zs) in pairs {
        let v = diff(&format!("{zb} value"), |l| unsafe { l.sym::<FnS0>(zb)() });
        // the forward must be exact, in each library independently
        diff(&format!("{zb} == {zs}"), |l| unsafe {
            (l.sym::<FnS0>(zb)(), l.sym::<FnS0>(zs)())
        });
        match *zb {
            "ZBUFF_recommendedCInSize" => assert_eq!(v, BLOCKSIZE_MAX),
            "ZBUFF_recommendedCOutSize" => {
                let bound = compress_bound(&pair().c, BLOCKSIZE_MAX);
                assert_eq!(v, bound + 3 + 4, "compressBound(128K) + blockHeader + 4");
            }
            "ZBUFF_recommendedDInSize" => assert_eq!(v, DSTREAM_IN_SIZE),
            "ZBUFF_recommendedDOutSize" => assert_eq!(v, BLOCKSIZE_MAX),
            _ => unreachable!(),
        }
    }
}

/// `ZBUFF_isError` / `ZBUFF_getErrorName` — a *separate translation unit's*
/// copy of `ERR_isError`/`ERR_getErrorName` (`zbuff_common.c:23`, `:26`).  The
/// sweep straddles the `code > ERROR(maxCode)` boundary (120) and the
/// "Unspecified error code" fallback.  CONFIGS row 389.
#[test]
fn zbuff_error_helpers() {
    covers(&[
        "CFG:389",
        "ERR:deprecated/zbuff_common.c:23,ERR:deprecated/zbuff_common.c:26",
    ]);
    check_err_family("ZBUFF err family", "ZBUFF_isError", Some("ZBUFF_getErrorName"));
    // The wrapper must agree with the ZSTD_* copy for every code.
    diff_bytes("ZBUFF vs ZSTD error names", |l| {
        let zb = l.sym::<FnGetErrorName>("ZBUFF_getErrorName");
        let zs = l.sym::<FnGetErrorName>("ZSTD_getErrorName");
        let mut out = Vec::new();
        for c in error_code_sweep() {
            let a = unsafe { cstr(zb(c)) };
            let b = unsafe { cstr(zs(c)) };
            assert_eq!(a, b, "[{}] ZBUFF/ZSTD error name mismatch for {c}", l.tag);
            out.extend_from_slice(a.as_bytes());
            out.push(0);
        }
        Blob(out)
    });
}

/// Context lifecycle for both sides, including the `customMem` paths:
/// `{NULL,NULL,NULL}` must behave like the default allocator, a *half*-set
/// `customMem` must yield NULL (`ZSTD_createCCtx_advanced`'s
/// `(!customAlloc) ^ (!customFree)` test), free-on-NULL must return 0, and a
/// counting allocator makes the number of allocations and the free order
/// observable.  CONFIGS row 390.
#[test]
fn zbuff_ctx_lifecycle() {
    covers(&[
        "CFG:390",
        "ERR:deprecated/zbuff_compress.c:56,ERR:deprecated/zbuff_compress.c:61,ERR:deprecated/zbuff_compress.c:66",
        "ERR:deprecated/zbuff_decompress.c:24,ERR:deprecated/zbuff_decompress.c:29,ERR:deprecated/zbuff_decompress.c:34",
    ]);
    for (create, create_adv, free) in [
        ("ZBUFF_createCCtx", "ZBUFF_createCCtx_advanced", "ZBUFF_freeCCtx"),
        ("ZBUFF_createDCtx", "ZBUFF_createDCtx_advanced", "ZBUFF_freeDCtx"),
    ] {
        // (a) plain create/free, (b) free(NULL)
        diff(&format!("{create}/{free}"), |l| {
            let c = l.sym::<FnP0>(create);
            let f = l.sym::<FnS1p>(free);
            let p = unsafe { c() };
            let nonnull = !p.is_null();
            let r = unsafe { f(p) };
            let rnull = unsafe { f(std::ptr::null_mut()) };
            (nonnull, r, rnull)
        });
        // (c) customMem = {NULL,NULL,NULL} -> defaults
        diff(&format!("{create_adv} default mem"), |l| {
            let c = l.sym::<FnPCustom>(create_adv);
            let f = l.sym::<FnS1p>(free);
            let p = unsafe { c(ZSTD_customMem::default()) };
            let nonnull = !p.is_null();
            let r = unsafe { f(p) };
            (nonnull, r)
        });
        // (c') half-set customMem -> NULL, both ways round
        diff(&format!("{create_adv} half mem"), |l| {
            let c = l.sym::<FnPCustom>(create_adv);
            let only_alloc = ZSTD_customMem {
                customAlloc: Some(counting_alloc),
                customFree: None,
                opaque: std::ptr::null_mut(),
            };
            let only_free = ZSTD_customMem {
                customAlloc: None,
                customFree: Some(counting_free),
                opaque: std::ptr::null_mut(),
            };
            let a = unsafe { c(only_alloc) };
            let b = unsafe { c(only_free) };
            (a.is_null(), b.is_null())
        });
        // (d) counting allocator: number of allocations and free order
        diff(&format!("{create_adv} counting mem"), |l| {
            let _ = take_alloc_log();
            let c = l.sym::<FnPCustom>(create_adv);
            let f = l.sym::<FnS1p>(free);
            let p = unsafe { c(counting_mem()) };
            assert!(!p.is_null(), "[{}] {create_adv} with a working allocator returned NULL", l.tag);
            let r = unsafe { f(p) };
            let log = take_alloc_log();
            let nb_alloc = log.iter().filter(|(k, _)| *k == 0).count();
            let nb_free = log.iter().filter(|(k, _)| *k == 1).count();
            let kinds: Vec<u8> = log.iter().map(|(k, _)| *k).collect();
            let sizes: Vec<usize> = log.iter().map(|(_, s)| *s).collect();
            assert_eq!(nb_alloc, nb_free, "[{}] {create_adv} leaked", l.tag);
            (r, nb_alloc, kinds, sizes)
        });
    }
}

/// `ZBUFF_compressInit` == `ZSTD_initCStream` (`zbuff_compress.c:107`): the
/// level is *clamped*, never rejected, so 23 / 100 / INT_MAX / INT_MIN must all
/// succeed and produce the level-22 / level-minCLevel byte stream.  CONFIGS
/// row 391.
#[test]
fn zbuff_compress_levels() {
    covers(&[
        "CFG:391",
        "ERR:deprecated/zbuff_compress.c:107,ERR:deprecated/zbuff_compress.c:126",
        "ERR:deprecated/zbuff_compress.c:143,ERR:deprecated/zbuff_compress.c:156",
    ]);
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    let src = corpus(Corpus::Mixed, 200_000, 0x391);
    for &lvl in &[-22i32, -5, -1, 0, 1, 3, 9, 19, 22, 23, 100, i32::MAX, i32::MIN] {
        diff_bytes(&format!("compressInit level={lvl}"), |l| {
            zbuff_compress(l, &src, lvl, Dict::None, BLOCKSIZE_MAX, cout)
        });
    }
}

/// The chunking sweep for *small* inputs: 10 corpora x {0,1,100,4096} bytes x
/// input chunk {1, 7, 100} with an output buffer big enough that the internal
/// buffer is always drained.  Walks every `zcss_load`/`zcss_flush` transition
/// of `ZSTD_compressStream` one byte at a time and pins both `size_t*`
/// out-params after every single call.  CONFIGS row 392.
#[test]
fn zbuff_compress_chunked_small() {
    covers(&["CFG:392"]);
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    let cin = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCInSize")() };
    for &k in ALL_CORPORA {
        for &n in &[0usize, 1, 100, 4096] {
            let src = corpus(k, n, 0x392);
            for &ic in &[1usize, 7, 100, 65536, cin] {
                diff_bytes(&format!("zbuff cc {k:?}/{n}/in={ic}"), |l| {
                    zbuff_compress(l, &src, 3, Dict::None, ic, cout)
                });
            }
        }
    }
}

/// The chunking sweep for *large* inputs: 10 corpora x {131072, 300000} bytes
/// x input chunk {100, `ZBUFF_recommendedCInSize()`, 65536, 0}.  The
/// recommended-size boundary is exactly where `ZSTD_compressStream` may bypass
/// its internal buffer; `in_chunk == 0` is the documented zero-length Continue.
/// CONFIGS row 392.
#[test]
fn zbuff_compress_chunked_large() {
    covers(&["CFG:392"]);
    let cin = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCInSize")() };
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    assert_eq!(cin, BLOCKSIZE_MAX);
    for &k in ALL_CORPORA {
        for &n in &[131_072usize, 300_000] {
            let src = corpus(k, n, 0x392B);
            for &ic in &[100usize, cin, 65536, 0, 200_000] {
                diff_bytes(&format!("zbuff cc {k:?}/{n}/in={ic}"), |l| {
                    zbuff_compress(l, &src, 3, Dict::None, ic, cout)
                });
            }
        }
    }
    // The 1- and 7-byte input chunkings are the expensive ones (one FFI call per
    // byte), so they run over 131072/300000-byte inputs for a representative
    // three corpora rather than all ten.
    for &k in &[Corpus::Zeros, Corpus::Random, Corpus::LongRepeats] {
        for &n in &[131_072usize, 300_000] {
            let src = corpus(k, n, 0x392B);
            for &ic in &[1usize, 7] {
                diff_bytes(&format!("zbuff cc-1by {k:?}/{n}/in={ic}"), |l| {
                    zbuff_compress(l, &src, 3, Dict::None, ic, cout)
                });
            }
        }
    }
}

/// The *output*-starved side of the same cycle: `*dstCapacityPtr` far smaller
/// than one block, so `ZBUFF_compressFlush`/`ZBUFF_compressEnd` must be called
/// repeatedly and their "nb of bytes still present in the internal buffer"
/// return value becomes observable (it is 0 whenever `dst` is large enough).
/// CONFIGS rows 392, 393.
#[test]
fn zbuff_compress_small_out_buffer() {
    covers(&["CFG:392,CFG:393"]);
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    for &k in &[Corpus::Zeros, Corpus::Random, Corpus::Text] {
        for &n in &[0usize, 1, 100, 4096] {
            let src = corpus(k, n, 0x393);
            for &(ic, oc) in &[(1usize, 1usize), (7, 3), (128, 17), (131_073, 1)] {
                diff_bytes(&format!("zbuff cc-tight {k:?}/{n}/{ic},{oc}"), |l| {
                    zbuff_compress(l, &src, 3, Dict::None, ic, oc)
                });
            }
        }
    }
    // CONFIGS 393: compressEnd with no Continue at all (empty frame epilogue),
    // with a roomy dst and then one byte at a time.
    for &oc in &[cout, 1usize] {
        diff_bytes(&format!("zbuff empty frame out={oc}"), |l| {
            zbuff_compress(l, &[], 3, Dict::None, BLOCKSIZE_MAX, oc)
        });
    }
}

/// `ZBUFF_compressInitDictionary` (`zbuff_compress.c:97-103`) = session reset +
/// `ZSTD_c_compressionLevel` + `ZSTD_CCtx_loadDictionary`, which auto-detects
/// raw content vs `ZSTD_MAGIC_DICTIONARY` and decides whether a dictID lands in
/// the frame header.  Crossed with the levels the task asks for.
/// CONFIGS row 394.
#[test]
fn zbuff_compress_dictionary() {
    covers(&[
        "CFG:394",
        "ERR:deprecated/zbuff_compress.c:100,ERR:deprecated/zbuff_compress.c:101",
    ]);
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    let rawd = raw_dict();
    let realdict = c_dict();
    let variants: &[(&str, Dict<'_>)] = &[
        ("nodict", Dict::None),
        ("empty", Dict::Buf(&[])),
        ("raw4096", Dict::Buf(&rawd)),
        ("zdict", Dict::Buf(realdict)),
    ];
    for &lvl in &[-1i32, 0, 1, 3, 9, 19, 22] {
        for (dn, d) in variants {
            for &k in &[Corpus::Text, Corpus::Random, Corpus::Zeros, Corpus::Periodic] {
                for &n in &[0usize, 1, 100, 4096, 131_072] {
                    let src = corpus(k, n, 0x394);
                    diff_bytes(&format!("zbuff dict {dn}/lvl={lvl}/{k:?}/{n}"), |l| {
                        zbuff_compress(l, &src, lvl, *d, BLOCKSIZE_MAX, cout)
                    });
                }
            }
        }
    }
}

/// `ZBUFF_compressInit_advanced` (`zbuff_compress.c:72-95`).  Two documented
/// quirks must be reproduced verbatim: `pledgedSrcSize == 0` is remapped to
/// `ZSTD_CONTENTSIZE_UNKNOWN` (line 76), and `fParams.noDictIDFlag` is passed
/// into `ZSTD_c_dictIDFlag` (line 91) — an *inverted* polarity versus the modern
/// API.  CONFIGS row 395.
#[test]
fn zbuff_compress_init_advanced() {
    covers(&[
        "CFG:395",
        "ERR:deprecated/zbuff_compress.c:93",
    ]);
    let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
    let rawd = raw_dict();
    // The parameter block itself comes from the C library so both libraries are
    // handed identical bytes; ZSTD_getParams is compared separately.
    diff("ZSTD_getParams(3,65536,0)", |l| unsafe {
        l.sym::<FnGetParams>("ZSTD_getParams")(3, 65536, 0)
    });
    let base = unsafe { pair().c.sym::<FnGetParams>("ZSTD_getParams")(3, 65536, 0) };
    let src = corpus(Corpus::Text, 65536, 0x395);
    for &pledged in &[0u64, 1, 65535, 65536, 65537, ZSTD_CONTENTSIZE_UNKNOWN] {
        for &csf in &[0i32, 1] {
            for &ckf in &[0i32, 1] {
                for &ndf in &[0i32, 1] {
                    let mut p = base;
                    p.fParams.contentSizeFlag = csf;
                    p.fParams.checksumFlag = ckf;
                    p.fParams.noDictIDFlag = ndf;
                    for (dn, d) in [("nodict", &[][..]), ("raw4096", &rawd[..])] {
                        diff_bytes(
                            &format!("zbuff adv p={pledged}/cs={csf}/ck={ckf}/nd={ndf}/{dn}"),
                            |l| {
                                zbuff_compress(
                                    l,
                                    &src,
                                    0,
                                    Dict::Adv(d, p, pledged),
                                    BLOCKSIZE_MAX,
                                    cout,
                                )
                            },
                        );
                    }
                }
            }
        }
    }
}

/// `ZBUFF_compressInit_advanced` rejects bad `cParams` at
/// `FORWARD_IF_ERROR(ZSTD_checkCParams(...))` (`zbuff_compress.c:80`) — i.e.
/// *before* any `ZSTD_CCtx_setParameter`, so the first failing bound in
/// `ZSTD_checkCParams` decides the code.  CONFIGS row 396.
#[test]
fn zbuff_compress_init_advanced_bad_params() {
    covers(&["CFG:396", "ERR:deprecated/zbuff_compress.c:80"]);
    let base = unsafe { pair().c.sym::<FnGetParams>("ZSTD_getParams")(3, 65536, 0) };
    let mutations: &[(&str, fn(&mut ZSTD_parameters))] = &[
        ("windowLog=5", |p| p.cParams.windowLog = 5),
        ("windowLog=32", |p| p.cParams.windowLog = 32),
        ("windowLog=0", |p| p.cParams.windowLog = 0),
        ("hashLog=1", |p| p.cParams.hashLog = 1),
        ("hashLog=40", |p| p.cParams.hashLog = 40),
        ("chainLog=0+btlazy2", |p| {
            p.cParams.chainLog = 0;
            p.cParams.strategy = ZSTD_btlazy2;
        }),
        ("minMatch=2", |p| p.cParams.minMatch = 2),
        ("minMatch=8", |p| p.cParams.minMatch = 8),
        ("searchLog=64", |p| p.cParams.searchLog = 64),
        ("targetLength=1<<20", |p| p.cParams.targetLength = 1 << 20),
        ("strategy=10", |p| p.cParams.strategy = 10),
        ("strategy=0", |p| p.cParams.strategy = 0),
        ("strategy=-1", |p| p.cParams.strategy = -1),
    ];
    for (name, m) in mutations {
        let mut p = base;
        m(&mut p);
        diff(&format!("zbuff adv bad {name}"), |l| {
            let create = l.sym::<FnP0>("ZBUFF_createCCtx");
            let free = l.sym::<FnS1p>("ZBUFF_freeCCtx");
            let init = l.sym::<FnZbAdv>("ZBUFF_compressInit_advanced");
            let zbc = unsafe { create() };
            let r = unsafe { init(zbc, std::ptr::null(), 0, p, 100) };
            let out = res(l, r);
            unsafe { free(zbc) };
            out
        });
    }
}

/// `ZBUFF_compressContinue` / `Flush` / `End` forward whatever
/// `ZSTD_compressStream`/`flushStream`/`endStream` return, and still write the
/// (zero) `outBuff.pos` / `inBuff.pos` back into the caller's `size_t`s
/// (`zbuff_compress.c:126-128`, `:143-144`, `:156-157`).  `dst == NULL` with a
/// non-zero capacity is the reachable error; a NULL *size pointer* is
/// `UNSAFE-UB` in the reference C and is therefore not tested (see
/// `mod exclusions`).
#[test]
fn zbuff_compress_error_paths() {
    covers(&[
        "ERR:deprecated/zbuff_compress.c:126,ERR:deprecated/zbuff_compress.c:143",
        "ERR:deprecated/zbuff_compress.c:156",
    ]);
    let src = corpus(Corpus::Text, 4096, 0x3C0);
    // (a) dst == NULL with *dstCapacityPtr != 0  -> dstBuffer_null
    diff("zbuff cc dst=NULL", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createCCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeCCtx");
        let init = l.sym::<FnSpi>("ZBUFF_compressInit");
        let cont = l.sym::<FnZbCont>("ZBUFF_compressContinue");
        let flush = l.sym::<FnZbFlush>("ZBUFF_compressFlush");
        let endf = l.sym::<FnZbFlush>("ZBUFF_compressEnd");
        let zbc = unsafe { create() };
        let i = unsafe { init(zbc, 3) };
        let mut cap = 64usize;
        let mut got = src.len();
        let a = unsafe {
            cont(zbc, std::ptr::null_mut(), &mut cap, src.as_ptr() as *const c_void, &mut got)
        };
        let mut cap2 = 64usize;
        let b = unsafe { flush(zbc, std::ptr::null_mut(), &mut cap2) };
        let mut cap3 = 64usize;
        let c = unsafe { endf(zbc, std::ptr::null_mut(), &mut cap3) };
        let fr = unsafe { free(zbc) };
        (res(l, i), res(l, a), cap, got, res(l, b), cap2, res(l, c), cap3, fr)
    });
    // (b) src == NULL with *srcSizePtr != 0 -> srcBuffer_wrong
    diff("zbuff cc src=NULL", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createCCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeCCtx");
        let init = l.sym::<FnSpi>("ZBUFF_compressInit");
        let cont = l.sym::<FnZbCont>("ZBUFF_compressContinue");
        let zbc = unsafe { create() };
        let i = unsafe { init(zbc, 3) };
        let mut dst = poison(4096);
        let mut cap = dst.len();
        let mut got = 100usize;
        let a = unsafe {
            cont(zbc, dst.as_mut_ptr() as *mut c_void, &mut cap, std::ptr::null(), &mut got)
        };
        let fr = unsafe { free(zbc) };
        (res(l, i), res(l, a), cap, got, fr, Blob(dst))
    });
    // (c) Continue/Flush/End on a context that was never initialised.
    diff("zbuff cc no-init", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createCCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeCCtx");
        let cont = l.sym::<FnZbCont>("ZBUFF_compressContinue");
        let flush = l.sym::<FnZbFlush>("ZBUFF_compressFlush");
        let endf = l.sym::<FnZbFlush>("ZBUFF_compressEnd");
        let zbc = unsafe { create() };
        let mut dst = poison(4096);
        let mut cap = dst.len();
        let mut got = src.len();
        let a = unsafe {
            cont(
                zbc,
                dst.as_mut_ptr() as *mut c_void,
                &mut cap,
                src.as_ptr() as *const c_void,
                &mut got,
            )
        };
        let mut cap2 = 4096usize;
        let b = unsafe { flush(zbc, dst.as_mut_ptr() as *mut c_void, &mut cap2) };
        let mut cap3 = 4096usize;
        let c = unsafe { endf(zbc, dst.as_mut_ptr() as *mut c_void, &mut cap3) };
        let fr = unsafe { free(zbc) };
        (res(l, a), cap, got, res(l, b), cap2, res(l, c), cap3, fr)
    });
}

/// `ZBUFF_decompressInit` == `ZSTD_initDStream`, `ZBUFF_decompressContinue`
/// wraps `ZSTD_decompressStream` and its documented return contract (0 = frame
/// decoded and flushed, 1 = data still buffered, >1 = suggested next input
/// size).  Round-trips real C-produced frames at several chunkings.
/// CONFIGS row 397.
#[test]
fn zbuff_decompress_cycle() {
    covers(&[
        "CFG:397",
        "ERR:deprecated/zbuff_decompress.c:47,ERR:deprecated/zbuff_decompress.c:66",
    ]);
    let din = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedDInSize")() };
    let dout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedDOutSize")() };
    for &k in ALL_CORPORA {
        for &n in &[0usize, 1, 100, 4096, 131_072] {
            let plain = corpus(k, n, 0x397);
            let frame = c_compress(&plain, 3);
            for &(ic, oc) in &[
                (1usize, 1usize),
                (3, 7),
                (100, 100),
                (din, dout),
                (frame.len().max(1), plain.len().max(1)),
            ] {
                let got = diff_bytes(&format!("zbuff dd {k:?}/{n}/{ic},{oc}"), |l| {
                    zbuff_decompress(l, &frame, Dict::None, ic, oc)
                });
                // the plaintext must actually come back out
                let sep = got.1 .0.windows(8).position(|w| w == [0xFFu8; 8]).unwrap();
                assert_eq!(
                    &got.1 .0[sep + 8..],
                    &plain[..],
                    "ZBUFF round-trip lost data for {k:?}/{n}/{ic},{oc}"
                );
            }
        }
    }
    // larger inputs, recommended chunking only (keeps the runtime bounded)
    for &k in &[Corpus::Zeros, Corpus::Random, Corpus::Text, Corpus::LongRepeats] {
        for &n in &[131_072usize, 300_000] {
            let plain = corpus(k, n, 0x397B);
            let frame = c_compress(&plain, 3);
            for &(ic, oc) in &[(din, dout), (7, 65536), (65536, 3)] {
                diff_bytes(&format!("zbuff dd-big {k:?}/{n}/{ic},{oc}"), |l| {
                    zbuff_decompress(l, &frame, Dict::None, ic, oc)
                });
            }
        }
    }
}

/// `ZBUFF_decompressInitDictionary` == `ZSTD_initDStream_usingDict`
/// (`zbuff_decompress.c:42`).  A dictID mismatch surfaces as
/// `dictionary_wrong` from `ZSTD_decompressStream`.  CONFIGS row 398.
#[test]
fn zbuff_decompress_dictionary() {
    covers(&["CFG:398", "ERR:deprecated/zbuff_decompress.c:42"]);
    let din = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedDInSize")() };
    let dout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedDOutSize")() };
    let rawd = raw_dict();
    let otherd = corpus(Corpus::Text, 4096, 0x0F1F);
    let realdict = c_dict();
    let plain = corpus(Corpus::Text, 65536, 0x398);

    // Frames produced with each dictionary, by the C library.
    let mut frames: Vec<(&str, Vec<u8>)> = Vec::new();
    for (n, d) in [("raw", &rawd[..]), ("zdict", &realdict[..])] {
        let cout = unsafe { pair().c.sym::<FnS0>("ZBUFF_recommendedCOutSize")() };
        let (_calls, blob) = zbuff_compress(&pair().c, &plain, 3, Dict::Buf(d), BLOCKSIZE_MAX, cout);
        let sep = blob.0.windows(8).position(|w| w == [0xFFu8; 8]).unwrap();
        frames.push((n, blob.0[sep + 8..].to_vec()));
    }
    for (fname, frame) in &frames {
        for (dn, d) in [
            ("match-raw", &rawd[..]),
            ("match-zdict", &realdict[..]),
            ("none", &[][..]),
            ("other", &otherd[..]),
        ] {
            for &(ic, oc) in &[(1usize, 1usize), (din, dout)] {
                diff_bytes(&format!("zbuff dd dict {fname}/{dn}/{ic},{oc}"), |l| {
                    zbuff_decompress(l, frame, Dict::Buf(d), ic, oc)
                });
            }
        }
    }
    // dict == NULL with a non-zero dictSize
    diff("zbuff decompressInitDictionary NULL/100", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createDCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeDCtx");
        let init = l.sym::<FnSpd>("ZBUFF_decompressInitDictionary");
        let d = unsafe { create() };
        let a = unsafe { init(d, std::ptr::null(), 100) };
        let b = unsafe { init(d, std::ptr::null(), 0) };
        let fr = unsafe { free(d) };
        (res(l, a), res(l, b), fr)
    });
}

/// The `ZBUFF_decompressContinue` error surface (`zbuff_decompress.c:66`):
/// no-init, truncated frames, corrupted headers, a broken XXH64 checksum, and
/// `dst == NULL`.  Every one of these is handled by the *modern*, fuzz-hardened
/// `ZSTD_decompressStream`, so arbitrary bytes are safe here (unlike the
/// `ZSTDv0x_*` decoders).  CONFIGS row 399.
#[test]
fn zbuff_decompress_error_paths() {
    covers(&["CFG:399", "ERR:deprecated/zbuff_decompress.c:66"]);
    let plain = corpus(Corpus::Text, 4096, 0x399);
    let good = c_compress(&plain, 3);

    // (a) no decompressInit at all
    diff("zbuff dd no-init", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createDCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeDCtx");
        let cont = l.sym::<FnZbCont>("ZBUFF_decompressContinue");
        let zbd = unsafe { create() };
        let mut dst = poison(4096);
        let one = [0u8; 4];
        let mut out: Vec<(R, usize, usize)> = Vec::new();
        for &sn in &[0usize, 4] {
            let mut cap = dst.len();
            let mut got = sn;
            let r = unsafe {
                cont(
                    zbd,
                    dst.as_mut_ptr() as *mut c_void,
                    &mut cap,
                    one.as_ptr() as *const c_void,
                    &mut got,
                )
            };
            out.push((res(l, r), cap, got));
        }
        let fr = unsafe { free(zbd) };
        (out, fr, Blob(dst))
    });

    // (b) truncated / corrupted frames after a proper init
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), Vec::new()));
    cases.push(("3-byte-prefix".into(), good[..3].to_vec()));
    cases.push(("4-zeros".into(), vec![0u8; 4]));
    cases.push(("magic-only".into(), good[..4].to_vec()));
    cases.push(("half".into(), good[..good.len() / 2].to_vec()));
    {
        let mut f = good.clone();
        f[5] ^= 0xFF;
        cases.push(("byte5-flipped".into(), f));
    }
    {
        // a checksummed frame with the trailing XXH64 digest corrupted
        type FnCompress2 = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            SizeT,
            *const c_void,
            SizeT,
        ) -> SizeT;
        let l = &pair().c;
        let cctx = Ctx::cctx(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        unsafe { set(cctx.ptr, ZSTD_c_checksumFlag, 1) };
        let mut dst = vec![0u8; compress_bound(l, plain.len()) + 64];
        let f = l.sym::<FnCompress2>("ZSTD_compress2");
        let n = unsafe {
            f(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                plain.as_ptr() as *const c_void,
                plain.len(),
            )
        };
        assert!(!is_error(l, n));
        dst.truncate(n);
        let mut bad = dst.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0x01;
        cases.push(("checksum-ok".into(), dst));
        cases.push(("checksum-broken".into(), bad));
    }
    for (name, frame) in &cases {
        for &(ic, oc) in &[(frame.len().max(1), 131_072usize), (1, 1)] {
            diff_bytes(&format!("zbuff dd bad {name}/{ic},{oc}"), |l| {
                zbuff_decompress(l, frame, Dict::None, ic, oc)
            });
        }
    }

    // (c) dst == NULL with *dstCapacityPtr != 0
    diff("zbuff dd dst=NULL", |l| {
        let create = l.sym::<FnP0>("ZBUFF_createDCtx");
        let free = l.sym::<FnS1p>("ZBUFF_freeDCtx");
        let init = l.sym::<FnS1p>("ZBUFF_decompressInit");
        let cont = l.sym::<FnZbCont>("ZBUFF_decompressContinue");
        let zbd = unsafe { create() };
        let i = unsafe { init(zbd) };
        let mut cap = 4096usize;
        let mut got = good.len();
        let r = unsafe {
            cont(
                zbd,
                std::ptr::null_mut(),
                &mut cap,
                good.as_ptr() as *const c_void,
                &mut got,
            )
        };
        let fr = unsafe { free(zbd) };
        (res(l, i), res(l, r), cap, got, fr)
    });
}

// ===========================================================================
// B. THE LEGACY DECODERS  (ZSTD_LEGACY_SUPPORT == 5)
// ===========================================================================

/// The exact per-file export lists from `SYMBOLS.md`.  v0.1..v0.4 are still
/// compiled and still export their symbols even though `ZSTD_isLegacy` no
/// longer dispatches to them, so their presence is part of the contract.
const SYM_V01: &[&str] = &[
    "ZSTDv01_createDCtx",
    "ZSTDv01_decompress",
    "ZSTDv01_decompressContinue",
    "ZSTDv01_decompressDCtx",
    "ZSTDv01_findFrameSizeInfoLegacy",
    "ZSTDv01_freeDCtx",
    "ZSTDv01_isError",
    "ZSTDv01_nextSrcSizeToDecompress",
    "ZSTDv01_resetDCtx",
];
const SYM_V02: &[&str] = &[
    "ZSTDv02_createDCtx",
    "ZSTDv02_decompress",
    "ZSTDv02_decompressContinue",
    "ZSTDv02_findFrameSizeInfoLegacy",
    "ZSTDv02_freeDCtx",
    "ZSTDv02_isError",
    "ZSTDv02_nextSrcSizeToDecompress",
    "ZSTDv02_resetDCtx",
];
const SYM_V03: &[&str] = &[
    "ZSTDv03_createDCtx",
    "ZSTDv03_decompress",
    "ZSTDv03_decompressContinue",
    "ZSTDv03_findFrameSizeInfoLegacy",
    "ZSTDv03_freeDCtx",
    "ZSTDv03_isError",
    "ZSTDv03_nextSrcSizeToDecompress",
    "ZSTDv03_resetDCtx",
];
const SYM_V04: &[&str] = &[
    "ZBUFFv04_createDCtx",
    "ZBUFFv04_decompressContinue",
    "ZBUFFv04_decompressInit",
    "ZBUFFv04_decompressWithDictionary",
    "ZBUFFv04_freeDCtx",
    "ZBUFFv04_getErrorName",
    "ZBUFFv04_isError",
    "ZBUFFv04_recommendedDInSize",
    "ZBUFFv04_recommendedDOutSize",
    "ZSTDv04_createDCtx",
    "ZSTDv04_decompress",
    "ZSTDv04_decompressContinue",
    "ZSTDv04_decompressDCtx",
    "ZSTDv04_findFrameSizeInfoLegacy",
    "ZSTDv04_freeDCtx",
    "ZSTDv04_nextSrcSizeToDecompress",
    "ZSTDv04_resetDCtx",
];
const SYM_V05: &[&str] = &[
    "FSEv05_buildDTable", "FSEv05_buildDTable_raw", "FSEv05_buildDTable_rle",
    "FSEv05_createDTable", "FSEv05_decompress", "FSEv05_decompress_usingDTable",
    "FSEv05_freeDTable", "FSEv05_getErrorName", "FSEv05_isError", "FSEv05_readNCount",
    "HUFv05_decompress", "HUFv05_decompress1X2", "HUFv05_decompress1X2_usingDTable",
    "HUFv05_decompress1X4", "HUFv05_decompress1X4_usingDTable", "HUFv05_decompress4X2",
    "HUFv05_decompress4X2_usingDTable", "HUFv05_decompress4X4",
    "HUFv05_decompress4X4_usingDTable", "HUFv05_getErrorName", "HUFv05_isError",
    "HUFv05_readDTableX2", "HUFv05_readDTableX4",
    "ZBUFFv05_createDCtx", "ZBUFFv05_decompressContinue", "ZBUFFv05_decompressInit",
    "ZBUFFv05_decompressInitDictionary", "ZBUFFv05_freeDCtx", "ZBUFFv05_getErrorName",
    "ZBUFFv05_isError", "ZBUFFv05_recommendedDInSize", "ZBUFFv05_recommendedDOutSize",
    "ZSTDv05_copyDCtx", "ZSTDv05_createDCtx", "ZSTDv05_decompress",
    "ZSTDv05_decompressBegin", "ZSTDv05_decompressBegin_usingDict",
    "ZSTDv05_decompressBlock", "ZSTDv05_decompressContinue", "ZSTDv05_decompressDCtx",
    "ZSTDv05_decompress_usingDict", "ZSTDv05_decompress_usingPreparedDCtx",
    "ZSTDv05_findFrameSizeInfoLegacy", "ZSTDv05_freeDCtx", "ZSTDv05_getErrorName",
    "ZSTDv05_getFrameParams", "ZSTDv05_isError", "ZSTDv05_nextSrcSizeToDecompress",
    "ZSTDv05_sizeofDCtx",
];
const SYM_V06: &[&str] = &[
    "FSEv06_buildDTable", "FSEv06_buildDTable_raw", "FSEv06_buildDTable_rle",
    "FSEv06_createDTable", "FSEv06_decompress", "FSEv06_decompress_usingDTable",
    "FSEv06_freeDTable", "FSEv06_getErrorName", "FSEv06_isError", "FSEv06_readNCount",
    "HUFv06_decompress", "HUFv06_decompress1X2", "HUFv06_decompress1X2_usingDTable",
    "HUFv06_decompress1X4", "HUFv06_decompress1X4_usingDTable", "HUFv06_decompress4X2",
    "HUFv06_decompress4X2_usingDTable", "HUFv06_decompress4X4",
    "HUFv06_decompress4X4_usingDTable", "HUFv06_readDTableX2", "HUFv06_readDTableX4",
    "ZBUFFv06_createDCtx", "ZBUFFv06_decompressContinue", "ZBUFFv06_decompressInit",
    "ZBUFFv06_decompressInitDictionary", "ZBUFFv06_freeDCtx", "ZBUFFv06_getErrorName",
    "ZBUFFv06_isError", "ZBUFFv06_recommendedDInSize", "ZBUFFv06_recommendedDOutSize",
    "ZSTDv06_copyDCtx", "ZSTDv06_createDCtx", "ZSTDv06_decompress",
    "ZSTDv06_decompressBegin", "ZSTDv06_decompressBegin_usingDict",
    "ZSTDv06_decompressBlock", "ZSTDv06_decompressContinue", "ZSTDv06_decompressDCtx",
    "ZSTDv06_decompress_usingDict", "ZSTDv06_decompress_usingPreparedDCtx",
    "ZSTDv06_findFrameSizeInfoLegacy", "ZSTDv06_freeDCtx", "ZSTDv06_getErrorName",
    "ZSTDv06_getFrameParams", "ZSTDv06_isError", "ZSTDv06_nextSrcSizeToDecompress",
    "ZSTDv06_sizeofDCtx",
];
const SYM_V07: &[&str] = &[
    "FSEv07_buildDTable", "FSEv07_buildDTable_raw", "FSEv07_buildDTable_rle",
    "FSEv07_createDTable", "FSEv07_decompress", "FSEv07_decompress_usingDTable",
    "FSEv07_freeDTable", "FSEv07_getErrorName", "FSEv07_isError", "FSEv07_readNCount",
    "HUFv07_decompress", "HUFv07_decompress1X2", "HUFv07_decompress1X2_DCtx",
    "HUFv07_decompress1X2_usingDTable", "HUFv07_decompress1X4",
    "HUFv07_decompress1X4_DCtx", "HUFv07_decompress1X4_usingDTable",
    "HUFv07_decompress1X_DCtx", "HUFv07_decompress1X_usingDTable",
    "HUFv07_decompress4X2", "HUFv07_decompress4X2_DCtx",
    "HUFv07_decompress4X2_usingDTable", "HUFv07_decompress4X4",
    "HUFv07_decompress4X4_DCtx", "HUFv07_decompress4X4_usingDTable",
    "HUFv07_decompress4X_DCtx", "HUFv07_decompress4X_hufOnly",
    "HUFv07_decompress4X_usingDTable", "HUFv07_getErrorName", "HUFv07_isError",
    "HUFv07_readDTableX2", "HUFv07_readDTableX4", "HUFv07_readStats",
    "HUFv07_selectDecoder",
    "ZBUFFv07_createDCtx", "ZBUFFv07_createDCtx_advanced", "ZBUFFv07_decompressContinue",
    "ZBUFFv07_decompressInit", "ZBUFFv07_decompressInitDictionary", "ZBUFFv07_freeDCtx",
    "ZBUFFv07_getErrorName", "ZBUFFv07_isError", "ZBUFFv07_recommendedDInSize",
    "ZBUFFv07_recommendedDOutSize",
    "ZSTDv07_copyDCtx", "ZSTDv07_createDCtx", "ZSTDv07_createDCtx_advanced",
    "ZSTDv07_createDDict", "ZSTDv07_decompress", "ZSTDv07_decompressBegin",
    "ZSTDv07_decompressBegin_usingDict", "ZSTDv07_decompressBlock",
    "ZSTDv07_decompressContinue", "ZSTDv07_decompressDCtx",
    "ZSTDv07_decompress_usingDDict", "ZSTDv07_decompress_usingDict",
    "ZSTDv07_estimateDCtxSize", "ZSTDv07_findFrameSizeInfoLegacy", "ZSTDv07_freeDCtx",
    "ZSTDv07_freeDDict", "ZSTDv07_getDecompressedSize", "ZSTDv07_getErrorName",
    "ZSTDv07_getFrameParams", "ZSTDv07_insertBlock", "ZSTDv07_isError",
    "ZSTDv07_isSkipFrame", "ZSTDv07_nextSrcSizeToDecompress", "ZSTDv07_sizeofDCtx",
];

/// Export parity for the seven legacy translation units, taken verbatim from
/// `SYMBOLS.md`'s per-file tables.  The *absences* matter as much as the
/// presences and are not symmetric across versions: v0.1..v0.4 export
/// `ZSTDv0x_isError` but no `getErrorName` (and v0.4 exports neither), v0.6
/// exports the `FSEv06` error helpers but not the `HUFv06` ones, and there is no
/// `FSEv01`..`FSEv04` family at all (those files keep FSE internal).
#[test]
fn legacy_symbol_surface() {
    covers(&["CFG:401,CFG:402,CFG:432"]);
    let p = pair();
    let all: &[&[&str]] = &[SYM_V01, SYM_V02, SYM_V03, SYM_V04, SYM_V05, SYM_V06, SYM_V07];
    let mut n = 0;
    for list in all {
        for s in *list {
            assert!(p.c.has(s), "C .so is missing `{s}`");
            assert!(p.r.has(s), "Rust .so is missing `{s}`");
            n += 1;
        }
    }
    assert_eq!(n, 9 + 8 + 8 + 17 + 49 + 47 + 68, "SYMBOLS.md legacy counts");

    const ABSENT: &[&str] = &[
        // v01..v04 have no getErrorName; v04 has no isError either.
        "ZSTDv01_getErrorName",
        "ZSTDv02_getErrorName",
        "ZSTDv03_getErrorName",
        "ZSTDv04_getErrorName",
        "ZSTDv04_isError",
        // v06 does not export the HUF error helpers although v05 and v07 do.
        "HUFv06_isError",
        "HUFv06_getErrorName",
        // no FSE family is exported from v01..v04.
        "FSEv01_isError", "FSEv01_decompress", "FSEv01_createDTable",
        "FSEv02_isError", "FSEv02_decompress",
        "FSEv03_isError", "FSEv03_decompress",
        "FSEv04_isError", "FSEv04_decompress",
        // no version-number entry points anywhere in the legacy families.
        "FSEv05_versionNumber", "FSEv06_versionNumber", "FSEv07_versionNumber",
        "HUFv05_versionNumber", "HUFv07_versionNumber",
        "ZSTDv05_versionNumber", "ZSTDv06_versionNumber", "ZSTDv07_versionNumber",
        // isSkipFrame only exists in v07.
        "ZSTDv05_isSkipFrame", "ZSTDv06_isSkipFrame",
        // *_advanced constructors only exist where v07 declares them.
        "ZBUFFv04_createDCtx_advanced",
        "ZBUFFv05_createDCtx_advanced",
        "ZBUFFv06_createDCtx_advanced",
        "ZSTDv05_createDCtx_advanced",
        "ZSTDv06_createDCtx_advanced",
        // v05/v06 have no DDict API.
        "ZSTDv05_createDDict", "ZSTDv06_createDDict",
        "ZSTDv05_estimateDCtxSize", "ZSTDv06_estimateDCtxSize",
        // no legacy compressor is shipped at all.
        "ZSTDv05_compress", "ZSTDv06_compress", "ZSTDv07_compress",
        "ZBUFFv07_compressInit", "ZSTDv07_createCCtx",
    ];
    for s in ABSENT {
        assert!(!p.c.has(s), "C .so unexpectedly exports `{s}`");
        assert!(!p.r.has(s), "Rust .so unexpectedly exports `{s}`");
    }
}

/// Every exported per-translation-unit copy of `ERR_isError` /
/// `ERR_getErrorName`.  Pure functions of a `size_t`, so the whole code range is
/// safe to sweep.  CONFIGS row 401.
#[test]
fn legacy_error_helpers() {
    covers(&["CFG:401"]);
    let fams: &[(&str, Option<&str>)] = &[
        ("ZSTDv01_isError", None),
        ("ZSTDv02_isError", None),
        ("ZSTDv03_isError", None),
        ("ZSTDv05_isError", Some("ZSTDv05_getErrorName")),
        ("ZSTDv06_isError", Some("ZSTDv06_getErrorName")),
        ("ZSTDv07_isError", Some("ZSTDv07_getErrorName")),
        ("ZBUFFv04_isError", Some("ZBUFFv04_getErrorName")),
        ("ZBUFFv05_isError", Some("ZBUFFv05_getErrorName")),
        ("ZBUFFv06_isError", Some("ZBUFFv06_getErrorName")),
        ("ZBUFFv07_isError", Some("ZBUFFv07_getErrorName")),
        ("FSEv05_isError", Some("FSEv05_getErrorName")),
        ("FSEv06_isError", Some("FSEv06_getErrorName")),
        ("FSEv07_isError", Some("FSEv07_getErrorName")),
        ("HUFv05_isError", Some("HUFv05_getErrorName")),
        ("HUFv07_isError", Some("HUFv07_getErrorName")),
    ];
    for (is_err, name) in fams {
        check_err_family(&format!("{is_err} family"), is_err, *name);
    }
}

/// DCtx lifecycle for all seven versions: `sizeofDCtx`/`estimateDCtxSize` (which
/// pin the struct layout as part of the ABI), `createDCtx` non-NULL,
/// `nextSrcSizeToDecompress` (4 for v01..v03's `ZSTD_frameHeaderSize`, 5 for
/// v04..v07's `frameHeaderSize_min`), `resetDCtx`/`decompressBegin`,
/// `ZSTDv07_isSkipFrame`, `freeDCtx` and `freeDCtx(NULL)`.  Pure state
/// inspection: nothing reads a caller buffer.  CONFIGS row 402.
#[test]
fn legacy_dctx_lifecycle() {
    covers(&[
        "CFG:402",
        "ERR:legacy/zstd_v01.c:2043,ERR:legacy/zstd_v02.c:3344,ERR:legacy/zstd_v03.c:2984",
        "ERR:legacy/zstd_v05.c:2632,ERR:legacy/zstd_v06.c:2789",
        "ERR:legacy/zstd_v07.c:2930/2933,ERR:legacy/zstd_v07.c:2939,ERR:legacy/zstd_v07.c:2946",
    ]);
    // ---- v01..v04 : create / nextSrcSize / reset / nextSrcSize / free ------
    for (v, expected) in [("ZSTDv01", 4usize), ("ZSTDv02", 4), ("ZSTDv03", 4), ("ZSTDv04", 5)] {
        diff(&format!("{v} dctx lifecycle"), |l| {
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let reset = l.sym::<FnS1p>(&format!("{v}_resetDCtx"));
            let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
            let d = unsafe { create() };
            assert!(!d.is_null(), "[{}] {v}_createDCtx returned NULL", l.tag);
            let n0 = unsafe { next(d) };
            let r = unsafe { reset(d) };
            let n1 = unsafe { next(d) };
            let f0 = unsafe { free(d) };
            let f1 = unsafe { free(std::ptr::null_mut()) };
            (n0, r, n1, f0, f1)
        });
        let got = diff(&format!("{v} nextSrcSize after create"), |l| {
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
            let d = unsafe { create() };
            let n = unsafe { next(d) };
            unsafe { free(d) };
            n
        });
        assert_eq!(got, expected, "{v} initial `expected`");
    }
    // ---- v05 / v06 : sizeofDCtx() takes no argument -----------------------
    for v in ["ZSTDv05", "ZSTDv06"] {
        diff(&format!("{v} dctx lifecycle"), |l| {
            let size = l.sym::<FnS0>(&format!("{v}_sizeofDCtx"));
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let begin = l.sym::<FnS1p>(&format!("{v}_decompressBegin"));
            let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
            let sz = unsafe { size() };
            let d = unsafe { create() };
            assert!(!d.is_null());
            let n0 = unsafe { next(d) };
            let b = unsafe { begin(d) };
            let n1 = unsafe { next(d) };
            let f0 = unsafe { free(d) };
            let f1 = unsafe { free(std::ptr::null_mut()) };
            (sz, n0, b, n1, f0, f1)
        });
    }
    // ---- v07 : sizeofDCtx(dctx), estimateDCtxSize(), isSkipFrame ----------
    diff("ZSTDv07 dctx lifecycle", |l| {
        let est = l.sym::<FnS0>("ZSTDv07_estimateDCtxSize");
        let size = l.sym::<FnS1pc>("ZSTDv07_sizeofDCtx");
        let create = l.sym::<FnP0>("ZSTDv07_createDCtx");
        let free = l.sym::<FnS1p>("ZSTDv07_freeDCtx");
        let begin = l.sym::<FnS1p>("ZSTDv07_decompressBegin");
        let next = l.sym::<FnS1p>("ZSTDv07_nextSrcSizeToDecompress");
        let skip = l.sym::<FnI1p>("ZSTDv07_isSkipFrame");
        let e = unsafe { est() };
        let d = unsafe { create() };
        assert!(!d.is_null());
        let sz = unsafe { size(d) };
        let n0 = unsafe { next(d) };
        let s0 = unsafe { skip(d) };
        let b = unsafe { begin(d) };
        let n1 = unsafe { next(d) };
        let s1 = unsafe { skip(d) };
        let f0 = unsafe { free(d) };
        let f1 = unsafe { free(std::ptr::null_mut()) };
        (e, sz, n0, s0, b, n1, s1, f0, f1)
    });
    // `ZSTDv07_createDCtx_advanced`'s two-step customMem validation
    // (`zstd_v07.c:2925-2931`): `!alloc && !free` -> defaults, then
    // `!alloc || !free` -> NULL.
    diff("ZSTDv07_createDCtx_advanced customMem", |l| {
        let c = l.sym::<FnPCustom>("ZSTDv07_createDCtx_advanced");
        let free = l.sym::<FnS1p>("ZSTDv07_freeDCtx");
        let def = unsafe { c(ZSTD_customMem::default()) };
        let def_ok = !def.is_null();
        let fd = unsafe { free(def) };
        let only_a = unsafe {
            c(ZSTD_customMem {
                customAlloc: Some(counting_alloc),
                customFree: None,
                opaque: std::ptr::null_mut(),
            })
        };
        let only_f = unsafe {
            c(ZSTD_customMem {
                customAlloc: None,
                customFree: Some(counting_free),
                opaque: std::ptr::null_mut(),
            })
        };
        let _ = take_alloc_log();
        let cnt = unsafe { c(counting_mem()) };
        let cnt_ok = !cnt.is_null();
        let fc = unsafe { free(cnt) };
        let log = take_alloc_log();
        (def_ok, fd, only_a.is_null(), only_f.is_null(), cnt_ok, fc, log)
    });
    // Same two-step in `ZBUFFv07_createDCtx_advanced` (`zstd_v07.c:4287-4303`),
    // which additionally unwinds with `ZBUFFv07_freeDCtx(zbd); return NULL;`.
    diff("ZBUFFv07_createDCtx_advanced customMem", |l| {
        let c = l.sym::<FnPCustom>("ZBUFFv07_createDCtx_advanced");
        let free = l.sym::<FnS1p>("ZBUFFv07_freeDCtx");
        let def = unsafe { c(ZSTD_customMem::default()) };
        let def_ok = !def.is_null();
        let fd = unsafe { free(def) };
        let only_a = unsafe {
            c(ZSTD_customMem {
                customAlloc: Some(counting_alloc),
                customFree: None,
                opaque: std::ptr::null_mut(),
            })
        };
        let only_f = unsafe {
            c(ZSTD_customMem {
                customAlloc: None,
                customFree: Some(counting_free),
                opaque: std::ptr::null_mut(),
            })
        };
        let _ = take_alloc_log();
        let cnt = unsafe { c(counting_mem()) };
        let cnt_ok = !cnt.is_null();
        let fc = unsafe { free(cnt) };
        let log = take_alloc_log();
        (def_ok, fd, only_a.is_null(), only_f.is_null(), cnt_ok, fc, log)
    });
}

/// `ZSTDv0x_copyDCtx` copies `sizeof(DCtx) - (BLOCKSIZE + WILDCOPY_OVERLENGTH +
/// frameHeaderSize_max)` bytes — the deliberately short "no need to copy
/// workspace" length, only observable when the destination tail is
/// pre-poisoned.  Both contexts are poisoned first so every byte of the
/// comparison is deterministic (`litPtr`/`litSize` are *not* initialised by
/// `decompressBegin`, so without the poison the copied region would contain
/// uninitialised heap).  CONFIGS row 403.
#[test]
fn legacy_copy_dctx() {
    covers(&["CFG:403"]);
    for v in ["ZSTDv05", "ZSTDv06"] {
        for pre_begin_b in [false, true] {
            diff_bytes(&format!("{v}_copyDCtx preB={pre_begin_b}"), |l| {
                let size = l.sym::<FnS0>(&format!("{v}_sizeofDCtx"));
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let begin = l.sym::<FnS1p>(&format!("{v}_decompressBegin"));
                let copy = l.sym::<FnCopyDCtx>(&format!("{v}_copyDCtx"));
                let sz = unsafe { size() };
                let a = unsafe { create() };
                let b = unsafe { create() };
                unsafe {
                    std::ptr::write_bytes(a as *mut u8, 0x5A, sz);
                    std::ptr::write_bytes(b as *mut u8, 0x5A, sz);
                    begin(a);
                    if pre_begin_b {
                        begin(b);
                    }
                    copy(b, a as *const c_void);
                }
                let out = unsafe { std::slice::from_raw_parts(b as *const u8, sz).to_vec() };
                // v05/v06 free with plain free(3): poisoning is harmless.
                unsafe {
                    free(a);
                    free(b);
                }
                (sz, Blob(out))
            });
        }
    }
    // v07's DCtx embeds `customMem`, and `ZSTDv07_freeDCtx` calls through it, so
    // the contexts are allocated with *our* allocator and released directly
    // rather than through the (poisoned) function pointers.
    for pre_begin_b in [false, true] {
        diff_bytes(&format!("ZSTDv07_copyDCtx preB={pre_begin_b}"), |l| {
            let create = l.sym::<FnPCustom>("ZSTDv07_createDCtx_advanced");
            let size = l.sym::<FnS1pc>("ZSTDv07_sizeofDCtx");
            let begin = l.sym::<FnS1p>("ZSTDv07_decompressBegin");
            let copy = l.sym::<FnCopyDCtx>("ZSTDv07_copyDCtx");
            let _ = take_alloc_log();
            let a = unsafe { create(counting_mem()) };
            let b = unsafe { create(counting_mem()) };
            assert!(!a.is_null() && !b.is_null());
            let sz = unsafe { size(a) };
            unsafe {
                std::ptr::write_bytes(a as *mut u8, 0x5A, sz);
                std::ptr::write_bytes(b as *mut u8, 0x5A, sz);
                begin(a);
                if pre_begin_b {
                    begin(b);
                }
                copy(b, a as *const c_void);
            }
            let out = unsafe { std::slice::from_raw_parts(b as *const u8, sz).to_vec() };
            counting_free(std::ptr::null_mut(), a);
            counting_free(std::ptr::null_mut(), b);
            let _ = take_alloc_log();
            (sz, Blob(out))
        });
    }
}

/// `ZSTDv05/06/07_getFrameParams` and `ZSTDv07_getDecompressedSize`.
///
/// SAFE because all three check `srcSize < frameHeaderSize_min` **before** any
/// `MEM_readLE32` (`zstd_v05.c:2754`, `zstd_v06.c:2928`, `zstd_v07.c:3099`) and
/// then reject a foreign magic before reading anything else. The three
/// short-input returns come from three *different* constants — v05 returns
/// `ZSTDv05_frameHeaderSize_max` (5), v06/v07 their `frameHeaderSize_min` (5) —
/// and only v07 has the `0x184D2A5x` skippable branch.
///
/// The out-param is a 64-byte poisoned buffer: bigger than any of the three
/// structs (v05 40, v06 16, v07 24 bytes), so an over-write by either library is
/// caught rather than corrupting the test. This also pins v05's documented
/// quirk that `params->windowLog` is stored *before* the reserved-bit check, so
/// a *rejected* header still mutates the caller's struct
/// (`zstd_v05.c:2758-2759`).  CONFIGS rows 404-407.
#[test]
fn legacy_get_frame_params() {
    covers(&[
        "CFG:404,CFG:405,CFG:406,CFG:407",
        "ERR:legacy/zstd_v05.c:2754,ERR:legacy/zstd_v05.c:2756,ERR:legacy/zstd_v05.c:2759",
        "ERR:legacy/zstd_v06.c:2913,ERR:legacy/zstd_v06.c:2928,ERR:legacy/zstd_v06.c:2929",
        "ERR:legacy/zstd_v06.c:2933,ERR:legacy/zstd_v06.c:2938",
        "ERR:legacy/zstd_v07.c:3099,ERR:legacy/zstd_v07.c:3103,ERR:legacy/zstd_v07.c:3126",
        "ERR:legacy/zstd_v07.c:3131,ERR:legacy/zstd_v07.c:3154,ERR:legacy/zstd_v07.c:3175",
    ]);
    const OUT: usize = 64;
    // (a) short inputs: nothing may be read and nothing written.
    for v in ["ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        diff_bytes(&format!("{v}_getFrameParams short"), |l| {
            let f = l.sym::<FnGfp>(&format!("{v}_getFrameParams"));
            let src = vec![0xA5u8; 32];
            let mut rets: Vec<SizeT> = Vec::new();
            let mut params: Vec<u8> = Vec::new();
            for n in 0..=4usize {
                let mut p = poison(OUT);
                let r = unsafe { f(p.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, n) };
                rets.push(r);
                params.append(&mut p);
            }
            // src == NULL with srcSize == 0 must take the same early return
            let mut p = poison(OUT);
            let r = unsafe { f(p.as_mut_ptr() as *mut c_void, std::ptr::null(), 0) };
            rets.push(r);
            params.append(&mut p);
            (rets, Blob(params))
        });
    }
    // (b) every magic (including the 16 skippable ones) x several sizes.
    let mut prefixes: Vec<(String, u32)> =
        MAGICS.iter().map(|(n, m)| ((*n).to_string(), *m)).collect();
    for i in 0..16u32 {
        prefixes.push((format!("skip{i}"), ZSTD_MAGIC_SKIPPABLE_START + i));
    }
    for v in ["ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        for (pn, magic) in &prefixes {
            diff_bytes(&format!("{v}_getFrameParams magic={pn}"), |l| {
                let f = l.sym::<FnGfp>(&format!("{v}_getFrameParams"));
                let gds = if v == "ZSTDv07" {
                    Some(l.sym::<FnUll2>("ZSTDv07_getDecompressedSize"))
                } else {
                    None
                };
                let mut rets: Vec<SizeT> = Vec::new();
                let mut sizes: Vec<c_ulonglong> = Vec::new();
                let mut params: Vec<u8> = Vec::new();
                for &n in &[5usize, 6, 7, 8, 9, 13, 14, 16, 18, 32] {
                    let src = magic_buf(*magic, n);
                    let mut p = poison(OUT);
                    let r = unsafe {
                        f(p.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, n)
                    };
                    rets.push(r);
                    params.append(&mut p);
                    if let Some(g) = &gds {
                        sizes.push(unsafe { g(src.as_ptr() as *const c_void, n) });
                    }
                }
                (rets, sizes, Blob(params))
            });
        }
    }
    // (c) the frame-descriptor byte sweep, with each version's own magic.
    for (v, magic) in [("ZSTDv05", MAGIC_V05), ("ZSTDv06", MAGIC_V06), ("ZSTDv07", MAGIC_V07)] {
        diff_bytes(&format!("{v}_getFrameParams fhd sweep"), |l| {
            let f = l.sym::<FnGfp>(&format!("{v}_getFrameParams"));
            let mut rets: Vec<SizeT> = Vec::new();
            let mut params: Vec<u8> = Vec::new();
            for fhd in 0..=255u8 {
                for &n in &[5usize, 6, 7, 8, 9, 12, 13, 14, 18, 32] {
                    let mut src = magic_buf(magic, n.max(5));
                    src[4] = fhd;
                    let mut p = poison(OUT);
                    let r = unsafe {
                        f(p.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, n)
                    };
                    rets.push(r);
                    params.append(&mut p);
                }
            }
            (rets.len(), Blob(rets.iter().flat_map(|r| (*r as u64).to_le_bytes()).collect()), Blob(params))
        });
    }
}

/// `ZSTDv0x_findFrameSizeInfoLegacy` for all seven versions.
///
/// SAFE for *any* payload: the whole function is a walk over 3-byte block
/// headers via `ZSTD_getcBlockSize`, whose own `srcSize < 3` guard
/// (`zstd_v01.c:1430`, `zstd_v05.c:2785`, `zstd_v06.c:2975`, `zstd_v07.c:3205`)
/// bounds every read; no literal/sequence bitstream is ever touched. The three
/// front guards differ per version and this test pins all three orderings:
///  * v01/v02/v03 guard with `srcSize < ZSTD_frameHeaderSize + blockHeaderSize`
///    (4+3 == 7) and v01 reads the magic **big-endian**;
///  * v04/v05 guard with `frameHeaderSize_min` (5) only;
///  * v06 calls `ZSTDv06_frameHeaderSize()` *before* the magic test, so a short
///    input reports `srcSize_wrong` while a 5..7-byte foreign-magic input
///    reports `prefix_unknown`;
///  * v07 has an explicit up-front `srcSize < 5 + 3` check before *both*, a
///    third distinct ordering, and its block loop tests `bt_end` **before**
///    `cBlockSize > remainingSize` (`zstd_v07.c:3899-3901`) unlike v05/v06.
/// CONFIGS rows 408, 409.
#[test]
fn legacy_find_frame_size_info() {
    covers(&[
        "CFG:408,CFG:409",
        "ERR:legacy/zstd_v01.c:1989/1994,ERR:legacy/zstd_v01.c:1430",
        "ERR:legacy/zstd_v02.c:3290/3295,ERR:legacy/zstd_v02.c:2762",
        "ERR:legacy/zstd_v03.c:2929/2934,ERR:legacy/zstd_v03.c:2402",
        "ERR:legacy/zstd_v04.c:3101/3105/3122,ERR:legacy/zstd_v04.c:2534",
        "ERR:legacy/zstd_v05.c:3493/3497/3514,ERR:legacy/zstd_v05.c:2785",
        "ERR:legacy/zstd_v06.c:3626-3628/3630/3634/3652,ERR:legacy/zstd_v06.c:2975",
        "ERR:legacy/zstd_v07.c:3867/3878/3882/3903,ERR:legacy/zstd_v07.c:3205",
    ]);
    let versions = ["ZSTDv01", "ZSTDv02", "ZSTDv03", "ZSTDv04", "ZSTDv05", "ZSTDv06", "ZSTDv07"];
    let mut prefixes: Vec<(String, u32)> =
        MAGICS.iter().map(|(n, m)| ((*n).to_string(), *m)).collect();
    prefixes.push(("skip5".into(), ZSTD_MAGIC_SKIPPABLE_START + 5));
    for v in versions {
        for (pn, magic) in &prefixes {
            diff(&format!("{v}_findFrameSizeInfoLegacy magic={pn}"), |l| {
                let f = l.sym::<FnFsi>(&format!("{v}_findFrameSizeInfoLegacy"));
                let mut out: Vec<(usize, R, c_ulonglong)> = Vec::new();
                for n in 0..=32usize {
                    let src = magic_buf(*magic, n);
                    // pre-poison both out-params so an untouched write is visible
                    let mut c: SizeT = 0x5A5A_5A5A_5A5A_5A5A;
                    let mut d: c_ulonglong = 0x5A5A_5A5A_5A5A_5A5A;
                    unsafe { f(src.as_ptr() as *const c_void, n, &mut c, &mut d) };
                    out.push((n, res(l, c), d));
                }
                out
            });
        }
        // A frame descriptor sweep with the *correct* magic. Still safe: every
        // payload byte is 0 except src[4], and the loop cannot leave the block
        // header walk.
        for fhd in [0x00u8, 0x01, 0x20, 0x3F, 0x40, 0x80, 0xC0, 0xE0, 0xFF] {
            diff(&format!("{v}_findFrameSizeInfoLegacy own-magic fhd={fhd:#02x}"), |l| {
                let f = l.sym::<FnFsi>(&format!("{v}_findFrameSizeInfoLegacy"));
                let magic = match v {
                    "ZSTDv01" => MAGIC_V01_LE,
                    "ZSTDv02" => MAGIC_V02,
                    "ZSTDv03" => MAGIC_V03,
                    "ZSTDv04" => MAGIC_V04,
                    "ZSTDv05" => MAGIC_V05,
                    "ZSTDv06" => MAGIC_V06,
                    _ => MAGIC_V07,
                };
                let mut out: Vec<(usize, R, c_ulonglong)> = Vec::new();
                for n in 5..=32usize {
                    let mut src = magic_buf(magic, n);
                    src[4] = fhd;
                    let mut c: SizeT = 0x5A5A_5A5A_5A5A_5A5A;
                    let mut d: c_ulonglong = 0x5A5A_5A5A_5A5A_5A5A;
                    unsafe { f(src.as_ptr() as *const c_void, n, &mut c, &mut d) };
                    out.push((n, res(l, c), d));
                }
                out
            });
        }
    }
    // Pin the v05/v07 asymmetry explicitly: a v0.5 magic + three zero block
    // header bytes is a *complete* (empty) frame, while the identical shape with
    // a v0.7 magic runs out of block headers and reports srcSize_wrong.
    let c = &pair().c;
    let f5 = c.sym::<FnFsi>("ZSTDv05_findFrameSizeInfoLegacy");
    let mut cs: SizeT = 0;
    let mut db: c_ulonglong = 0;
    let src = magic_buf(MAGIC_V05, 8);
    unsafe { f5(src.as_ptr() as *const c_void, 8, &mut cs, &mut db) };
    assert_eq!((cs, db), (8, 0), "v0.5: bt_compressed with cSize 0 ends the frame");
}

/// The front guards of the one-shot legacy decompressors.
///
/// SAFE: for every version the size check and the magic test both run **before**
/// `getcBlockSize` and before any literal/sequence decoding
/// (`zstd_v01.c:1921/1923`, `zstd_v02.c:3221/3223`, `zstd_v03.c:2860/2862`,
/// `zstd_v04.c:3036` + `2496`, `zstd_v05.c:3385/3388`, `zstd_v06.c:3517/3520`,
/// `zstd_v07.c:3752/3757`). Each version's **own** magic is deliberately
/// excluded from the sweep, because with a matching magic the header is accepted
/// and the block loop starts walking caller bytes; likewise no `0x184D2A5x`
/// skippable magic is used, which for v0.7 would make `decodeFrameHeader`
/// succeed. `dst` is poisoned and compared in full: none of these paths may
/// write a byte.
///
/// The v05 vs v06/v07 behavioural split is the point: v05 propagates
/// `prefix_unknown` unchanged, whereas `ZSTDv06/07_decompressFrame` collapse
/// *every* non-zero `getFrameParams` result into `corruption_detected`
/// (`zstd_v06.c:3521`, `zstd_v07.c:3757`).  CONFIGS rows 410, 411, 412.
#[test]
fn legacy_decompress_front_guards() {
    covers(&[
        "CFG:410,CFG:411,CFG:412",
        "ERR:legacy/zstd_v01.c:1921,ERR:legacy/zstd_v01.c:1923",
        "ERR:legacy/zstd_v02.c:3221,ERR:legacy/zstd_v02.c:3223",
        "ERR:legacy/zstd_v03.c:2860,ERR:legacy/zstd_v03.c:2862",
        "ERR:legacy/zstd_v04.c:3036,ERR:legacy/zstd_v04.c:2496,ERR:legacy/zstd_v04.c:2510",
        "ERR:legacy/zstd_v04.c:3039/3054/3069,ERR:legacy/zstd_v04.c:3560",
        "ERR:legacy/zstd_v05.c:3385/3388/3403/3418,ERR:legacy/zstd_v05.c:3466",
        "ERR:legacy/zstd_v06.c:3517/3522/3535/3550,ERR:legacy/zstd_v06.c:3523",
        "ERR:legacy/zstd_v06.c:3599,ERR:legacy/zstd_v07.c:3752/3757/3771/3786",
        "ERR:legacy/zstd_v07.c:3758,ERR:legacy/zstd_v07.c:3842,ERR:legacy/zstd_v07.c:3186",
    ]);
    const CAP: usize = 65536;
    let sizes: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 32];
    let own = |v: &str| match v {
        "ZSTDv01" => MAGIC_V01_LE,
        "ZSTDv02" => MAGIC_V02,
        "ZSTDv03" => MAGIC_V03,
        "ZSTDv04" => MAGIC_V04,
        "ZSTDv05" => MAGIC_V05,
        "ZSTDv06" => MAGIC_V06,
        _ => MAGIC_V07,
    };

    // ---- the plain `ZSTDv0x_decompress(dst, cap, src, n)` form -------------
    for v in ["ZSTDv01", "ZSTDv02", "ZSTDv03", "ZSTDv04", "ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        let mine = own(v);
        for (pn, magic) in MAGICS {
            if *magic == mine || (*magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                continue; // see the doc-comment: not a safe input
            }
            diff_bytes(&format!("{v}_decompress magic={pn}"), |l| {
                let f = l.sym::<FnDec4>(&format!("{v}_decompress"));
                let mut rets: Vec<R> = Vec::new();
                let mut dsts: Vec<u8> = Vec::new();
                for &n in sizes {
                    let src = magic_buf(*magic, n);
                    let mut dst = poison(CAP);
                    let r = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            CAP,
                            src.as_ptr() as *const c_void,
                            n,
                        )
                    };
                    rets.push(res(l, r));
                    dsts.push(if dst.iter().all(|&b| b == 0x5A) { 1 } else { 0 });
                    assert!(
                        dst.iter().all(|&b| b == 0x5A),
                        "[{}] {v}_decompress wrote into dst for magic={pn} n={n}",
                        l.tag
                    );
                }
                (rets, Blob(dsts))
            });
        }
    }

    // ---- the DCtx forms, incl. usingDict / usingPreparedDCtx / usingDDict --
    let rawd = raw_dict();
    for v in ["ZSTDv01", "ZSTDv04", "ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        let mine = own(v);
        for (pn, magic) in MAGICS {
            if *magic == mine || (*magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                continue;
            }
            diff(&format!("{v}_decompressDCtx magic={pn}"), |l| {
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let f = l.sym::<FnDecDCtx>(&format!("{v}_decompressDCtx"));
                let mut rets: Vec<R> = Vec::new();
                let mut untouched = true;
                for &n in sizes {
                    let src = magic_buf(*magic, n);
                    let mut dst = poison(CAP);
                    let d = unsafe { create() };
                    let r = unsafe {
                        f(
                            d,
                            dst.as_mut_ptr() as *mut c_void,
                            CAP,
                            src.as_ptr() as *const c_void,
                            n,
                        )
                    };
                    unsafe { free(d) };
                    rets.push(res(l, r));
                    untouched &= dst.iter().all(|&b| b == 0x5A);
                }
                (rets, untouched)
            });
            if v == "ZSTDv01" || v == "ZSTDv04" {
                continue; // no usingDict family below v0.5
            }
            diff(&format!("{v}_decompress_usingDict magic={pn}"), |l| {
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let f = l.sym::<FnDecDict>(&format!("{v}_decompress_usingDict"));
                let mut rets: Vec<R> = Vec::new();
                for &n in sizes {
                    let src = magic_buf(*magic, n);
                    for dict in [&[][..], &rawd[..]] {
                        let mut dst = poison(CAP);
                        let d = unsafe { create() };
                        let r = unsafe {
                            f(
                                d,
                                dst.as_mut_ptr() as *mut c_void,
                                CAP,
                                src.as_ptr() as *const c_void,
                                n,
                                if dict.is_empty() {
                                    std::ptr::null()
                                } else {
                                    dict.as_ptr() as *const c_void
                                },
                                dict.len(),
                            )
                        };
                        unsafe { free(d) };
                        rets.push(res(l, r));
                        assert!(dst.iter().all(|&b| b == 0x5A));
                    }
                }
                rets
            });
        }
    }
    // v05/v06 `_usingPreparedDCtx` (reference DCtx prepared with and without a
    // raw dictionary) and v07 `_usingDDict`.
    for v in ["ZSTDv05", "ZSTDv06"] {
        let mine = own(v);
        for (pn, magic) in MAGICS {
            if *magic == mine || (*magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START {
                continue;
            }
            diff(&format!("{v}_decompress_usingPreparedDCtx magic={pn}"), |l| {
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let begind = l.sym::<FnSpd>(&format!("{v}_decompressBegin_usingDict"));
                let f = l.sym::<FnDecPrepared>(&format!("{v}_decompress_usingPreparedDCtx"));
                let mut rets: Vec<R> = Vec::new();
                for dict in [&[][..], &rawd[..]] {
                    let refd = unsafe { create() };
                    let bd = unsafe {
                        begind(
                            refd,
                            if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void },
                            dict.len(),
                        )
                    };
                    rets.push(res(l, bd));
                    for &n in sizes {
                        let src = magic_buf(*magic, n);
                        let mut dst = poison(CAP);
                        let d = unsafe { create() };
                        let r = unsafe {
                            f(
                                d,
                                refd as *const c_void,
                                dst.as_mut_ptr() as *mut c_void,
                                CAP,
                                src.as_ptr() as *const c_void,
                                n,
                            )
                        };
                        unsafe { free(d) };
                        rets.push(res(l, r));
                        assert!(dst.iter().all(|&b| b == 0x5A));
                    }
                    unsafe { free(refd) };
                }
                rets
            });
        }
    }
    for (pn, magic) in MAGICS {
        if *magic == MAGIC_V07 || (*magic & ZSTD_MAGIC_SKIPPABLE_MASK) == ZSTD_MAGIC_SKIPPABLE_START
        {
            continue;
        }
        diff(&format!("ZSTDv07_decompress_usingDDict magic={pn}"), |l| {
            let create = l.sym::<FnP0>("ZSTDv07_createDCtx");
            let free = l.sym::<FnS1p>("ZSTDv07_freeDCtx");
            let cdd = l.sym::<FnCreateDDict>("ZSTDv07_createDDict");
            let fdd = l.sym::<FnS1p>("ZSTDv07_freeDDict");
            let f = l.sym::<FnDecDDict>("ZSTDv07_decompress_usingDDict");
            let ddict = unsafe { cdd(rawd.as_ptr() as *const c_void, rawd.len()) };
            assert!(!ddict.is_null());
            let mut rets: Vec<R> = Vec::new();
            for &n in sizes {
                let src = magic_buf(*magic, n);
                let mut dst = poison(CAP);
                let d = unsafe { create() };
                let r = unsafe {
                    f(
                        d,
                        dst.as_mut_ptr() as *mut c_void,
                        CAP,
                        src.as_ptr() as *const c_void,
                        n,
                        ddict as *const c_void,
                    )
                };
                unsafe { free(d) };
                rets.push(res(l, r));
                assert!(dst.iter().all(|&b| b == 0x5A));
            }
            let fr = unsafe { fdd(ddict) };
            rets.push(res(l, fr));
            rets
        });
    }
}

/// `ZSTDv0x_decompressBlock`'s only pre-bitstream guard is
/// `srcSize >= BLOCKSIZE` (all three constants are 131072:
/// `zstd_v05.c:3347`, `zstd_v06.c:3481`, `zstd_v07.c:3694`).  Feeding exactly
/// that or more is therefore the *whole* safe input set for these three entry
/// points; anything smaller reaches `decodeLiteralsBlock` and is `UNSAFE-UB`.
/// CONFIGS row 413.
#[test]
fn legacy_decompress_block_size_cap() {
    // CFG:414 / CFG:420 are the CONFIGS rows whose *content* is the exclusion
    // statement for `ZSTDv0x_decompressContinue` / `ZSTDv0x_decompressBlock` /
    // `ZSTDv07_insertBlock` / `ZBUFFv0x_decompressContinue`-past-the-header; they
    // are discharged by this test (the one safe guard) plus `mod exclusions`.
    covers(&[
        "CFG:413,CFG:414,CFG:420",
        "ERR:legacy/zstd_v05.c:3347,ERR:legacy/zstd_v06.c:3481,ERR:legacy/zstd_v07.c:3694",
    ]);
    const CAP: usize = 262_144;
    let src = vec![0u8; CAP];
    for v in ["ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        diff(&format!("{v}_decompressBlock size cap"), |l| {
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let begin = l.sym::<FnS1p>(&format!("{v}_decompressBegin"));
            let f = l.sym::<FnDecDCtx>(&format!("{v}_decompressBlock"));
            let mut rets: Vec<R> = Vec::new();
            for &n in &[131_072usize, 131_073, 200_000, 262_144] {
                let d = unsafe { create() };
                unsafe { begin(d) };
                let mut dst = poison(CAP);
                let r = unsafe {
                    f(
                        d,
                        dst.as_mut_ptr() as *mut c_void,
                        CAP,
                        src.as_ptr() as *const c_void,
                        n,
                    )
                };
                unsafe { free(d) };
                rets.push(res(l, r));
                assert!(
                    dst.iter().all(|&b| b == 0x5A),
                    "[{}] {v}_decompressBlock wrote to dst at n={n}",
                    l.tag
                );
            }
            rets
        });
    }
}

/// `ZSTDv0x_decompressContinue`'s first statement is
/// `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);`
/// (`zstd_v01.c:2064`, `zstd_v02.c:3363`, `zstd_v03.c:3003`, `zstd_v04.c:3149`,
/// `zstd_v05.c:3540`, `zstd_v06.c:3678`, `zstd_v07.c:3936`).  Feeding *any*
/// srcSize other than the freshly-reset `expected` (4 for v01..v03, 5 for
/// v04..v07) is therefore rejected before a single byte of `src` is read — the
/// only safe way to exercise these entry points with arbitrary sizes.
///
/// Additionally, the first *matching*-size call is safe because it does nothing
/// but a magic/frame-header parse of exactly those 4-5 bytes: v01/v02/v03 read a
/// single BE32/LE32 magic, v04..v07 call `decodeFrameHeader_Part1` /
/// `frameHeaderSize`. A foreign magic therefore yields `prefix_unknown` with no
/// bitstream walking.
#[test]
fn legacy_decompress_continue_guards() {
    covers(&[
        "ERR:legacy/zstd_v01.c:2064,ERR:legacy/zstd_v01.c:2073",
        "ERR:legacy/zstd_v02.c:3363/3372,ERR:legacy/zstd_v03.c:3003/3012",
        "ERR:legacy/zstd_v04.c:3149/3157/3161,ERR:legacy/zstd_v04.c:2494",
        "ERR:legacy/zstd_v05.c:3540/3548/3552,ERR:legacy/zstd_v06.c:3678/3685",
        "ERR:legacy/zstd_v07.c:3936/3942",
    ]);
    let own = |v: &str| match v {
        "ZSTDv01" => MAGIC_V01_LE,
        "ZSTDv02" => MAGIC_V02,
        "ZSTDv03" => MAGIC_V03,
        "ZSTDv04" => MAGIC_V04,
        "ZSTDv05" => MAGIC_V05,
        "ZSTDv06" => MAGIC_V06,
        _ => MAGIC_V07,
    };
    for v in ["ZSTDv01", "ZSTDv02", "ZSTDv03", "ZSTDv04", "ZSTDv05", "ZSTDv06", "ZSTDv07"] {
        let reset = if v == "ZSTDv05" || v == "ZSTDv06" || v == "ZSTDv07" {
            format!("{v}_decompressBegin")
        } else {
            format!("{v}_resetDCtx")
        };
        // (a) srcSize != expected -> srcSize_wrong, nothing read.
        diff(&format!("{v}_decompressContinue wrong size"), |l| {
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let rst = l.sym::<FnS1p>(&reset);
            let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
            let f = l.sym::<FnDecDCtx>(&format!("{v}_decompressContinue"));
            let d = unsafe { create() };
            unsafe { rst(d) };
            let expected = unsafe { next(d) };
            let mut rets: Vec<(usize, R, SizeT)> = Vec::new();
            for &n in &[0usize, 1, 2, 3, 4, 5, 6, 7, 100] {
                if n == expected {
                    continue;
                }
                let mut dst = poison(4096);
                // src is a zero-filled 128-byte buffer; it is never read because
                // the size check fires first.
                let src = vec![0u8; 128];
                let r = unsafe {
                    f(
                        d,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        n,
                    )
                };
                assert!(dst.iter().all(|&b| b == 0x5A));
                rets.push((n, res(l, r), unsafe { next(d) }));
            }
            let fr = unsafe { free(d) };
            (expected, rets, fr)
        });
        // (b) the matching-size first call: pure magic / frame-header parse.
        for (pn, magic) in MAGICS {
            if *magic == own(v) {
                continue; // a matching magic advances into the block stages
            }
            diff(&format!("{v}_decompressContinue hdr magic={pn}"), |l| {
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let rst = l.sym::<FnS1p>(&reset);
                let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
                let f = l.sym::<FnDecDCtx>(&format!("{v}_decompressContinue"));
                let d = unsafe { create() };
                unsafe { rst(d) };
                let expected = unsafe { next(d) };
                let src = magic_buf(*magic, expected);
                let mut dst = poison(4096);
                let r = unsafe {
                    f(
                        d,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        src.as_ptr() as *const c_void,
                        expected,
                    )
                };
                let n2 = unsafe { next(d) };
                let fr = unsafe { free(d) };
                (res(l, r), n2, fr, Blob(dst))
            });
        }
    }
}

/// `FSEv0x_createDTable` / `freeDTable` (CONFIGS row 421) plus
/// `buildDTable_raw` / `buildDTable_rle` (CONFIGS row 422).
///
/// SAFE: `createDTable` silently clamps `tableLog` to
/// `FSEv0x_TABLELOG_ABSOLUTE_MAX` (15) before the malloc
/// (`zstd_v05.c:1148`, `zstd_v06.c:1393`, `zstd_v07.c:1414`), so 16 / 31 /
/// 0xFFFFFFFF all allocate the tableLog-15 size; `buildDTable_raw`'s
/// `nbBits < 1` test precedes every store (`zstd_v05.c:1358`) and
/// `buildDTable_rle` writes exactly one header plus one cell. Neither reads a
/// caller buffer. The whole allocation is poisoned first so the in-memory layout
/// of `FSEv0x_DTableHeader`/`FSEv0x_decode_t` is pinned byte for byte.
#[test]
fn legacy_fse_dtable() {
    covers(&[
        "CFG:421,CFG:422",
        "ERR:legacy/zstd_v05.c:1148,ERR:legacy/zstd_v06.c:1394,ERR:legacy/zstd_v07.c:1415",
        "ERR:legacy/zstd_v05.c:1358,ERR:legacy/zstd_v06.c:1497,ERR:legacy/zstd_v07.c:1518",
        "ERR:legacy/zstd_v05.c:1328,ERR:legacy/zstd_v06.c:1467,ERR:legacy/zstd_v07.c:1488",
    ]);
    /// `FSEv0x_DTABLE_SIZE_U32(15) * sizeof(U32)`.
    const T15: usize = (1 + (1usize << 15)) * 4;
    for v in ["FSEv05", "FSEv06", "FSEv07"] {
        // (a) createDTable clamp: the requested byte size per tableLog.
        diff(&format!("{v}_createDTable clamp"), |l| {
            let _ = take_alloc_log();
            let c = l.sym::<FnCreateDTable>(&format!("{v}_createDTable"));
            let f = l.sym::<FnV1p>(&format!("{v}_freeDTable"));
            let mut ok: Vec<bool> = Vec::new();
            for &tl in &[0u32, 1, 5, 8, 12, 15, 16, 31, 0xFFFF_FFFF] {
                let p = unsafe { c(tl) };
                ok.push(!p.is_null());
                unsafe { f(p) };
            }
            unsafe { f(std::ptr::null_mut()) }; // free(NULL)
            ok
        });
        // (b) buildDTable_raw / _rle over a fully poisoned tableLog-15 table.
        diff_bytes(&format!("{v}_buildDTable_raw/_rle"), |l| {
            let c = l.sym::<FnCreateDTable>(&format!("{v}_createDTable"));
            let fr = l.sym::<FnV1p>(&format!("{v}_freeDTable"));
            let raw = l.sym::<FnBuildRaw>(&format!("{v}_buildDTable_raw"));
            let rle = l.sym::<FnBuildRle>(&format!("{v}_buildDTable_rle"));
            let mut rets: Vec<R> = Vec::new();
            let mut bytes: Vec<u8> = Vec::new();
            for &nb in &[0u32, 1, 2, 3, 4, 8, 15] {
                let dt = unsafe { c(15) };
                assert!(!dt.is_null());
                unsafe { std::ptr::write_bytes(dt as *mut u8, 0x5A, T15) };
                let r = unsafe { raw(dt, nb) };
                rets.push(res(l, r));
                bytes.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(dt as *const u8, T15)
                });
                unsafe { fr(dt) };
            }
            for &sv in &[0u8, 1, 127, 128, 255] {
                let dt = unsafe { c(15) };
                unsafe { std::ptr::write_bytes(dt as *mut u8, 0x5A, T15) };
                let r = unsafe { rle(dt, sv) };
                rets.push(res(l, r));
                bytes.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(dt as *const u8, T15)
                });
                unsafe { fr(dt) };
            }
            (rets, Blob(bytes))
        });
    }
}

/// `FSEv0x_readNCount` (CONFIGS row 423) and `FSEv0x_decompress` (row 424) —
/// only the two up-front checks, which are the *entire* safe input set.
///
/// SAFE: `if (hbSize < 4) return ERROR(srcSize_wrong);` precedes the
/// `MEM_readLE32` (`zstd_v05.c:1244`), and
/// `nbBits = (bitStream & 0xF) + 5; if (nbBits > 15) return tableLog_tooLarge;`
/// (`zstd_v05.c:1247`) fires before the normalized-count bit walk. Byte 0 low
/// nibble >= 11 therefore never enters the loop whose `iend`-relative pointer
/// guards under-flow. `FSEv0x_decompress` adds `if (cSrcSize < 2) return
/// srcSize_wrong;` (`zstd_v05.c:1464`) and then forwards `readNCount`'s error.
#[test]
fn legacy_fse_readncount_decompress() {
    covers(&[
        "CFG:423,CFG:424",
        "ERR:legacy/zstd_v05.c:1244,ERR:legacy/zstd_v05.c:1247,ERR:legacy/zstd_v05.c:1464",
        "ERR:legacy/zstd_v05.c:1469",
        "ERR:legacy/zstd_v06.c:1221/1224/1251/1291/1295,ERR:legacy/zstd_v06.c:1602/1607",
        "ERR:legacy/zstd_v07.c:1166/1169/1196/1236/1240,ERR:legacy/zstd_v07.c:1623/1628",
        // CFG:425 is the CONFIGS row that *states* the FSE exclusion; discharged
        // by the two safe front guards above plus `mod exclusions`.
        "CFG:425",
    ]);
    for v in ["FSEv05", "FSEv06", "FSEv07"] {
        diff_bytes(&format!("{v}_readNCount guards"), |l| {
            let f = l.sym::<FnReadNCount>(&format!("{v}_readNCount"));
            let mut rets: Vec<R> = Vec::new();
            let mut outs: Vec<u8> = Vec::new();
            // hbSize 0..3 : srcSize_wrong before any read
            for hb in 0..=3usize {
                let mut nc = vec![0x5A5Ai16; 256];
                let mut msv: c_uint = 255;
                let mut tl: c_uint = 0x5A5A_5A5A;
                let hdr = [0u8; 8];
                let r = unsafe {
                    f(nc.as_mut_ptr(), &mut msv, &mut tl, hdr.as_ptr() as *const c_void, hb)
                };
                rets.push(res(l, r));
                outs.extend_from_slice(&msv.to_le_bytes());
                outs.extend_from_slice(&tl.to_le_bytes());
                outs.extend(nc.iter().flat_map(|x| x.to_le_bytes()));
            }
            // low nibble >= 11 : tableLog_tooLarge before the count walk
            for b0 in [0x0Bu8, 0x0C, 0x0D, 0x0E, 0x0F, 0x1B, 0xFB, 0xFF] {
                for hb in [4usize, 5, 8] {
                    let mut nc = vec![0x5A5Ai16; 256];
                    let mut msv: c_uint = 255;
                    let mut tl: c_uint = 0x5A5A_5A5A;
                    let mut hdr = [0u8; 8];
                    hdr[0] = b0;
                    let r = unsafe {
                        f(nc.as_mut_ptr(), &mut msv, &mut tl, hdr.as_ptr() as *const c_void, hb)
                    };
                    rets.push(res(l, r));
                    outs.extend_from_slice(&msv.to_le_bytes());
                    outs.extend_from_slice(&tl.to_le_bytes());
                    outs.extend(nc.iter().flat_map(|x| x.to_le_bytes()));
                }
            }
            (rets, Blob(outs))
        });
        diff(&format!("{v}_decompress guards"), |l| {
            let f = l.sym::<FnDec4>(&format!("{v}_decompress"));
            let mut rets: Vec<R> = Vec::new();
            let mut untouched = true;
            for cs in [0usize, 1] {
                let src = [0u8; 8];
                let mut dst = poison(4096);
                let r = unsafe {
                    f(dst.as_mut_ptr() as *mut c_void, dst.len(), src.as_ptr() as *const c_void, cs)
                };
                rets.push(res(l, r));
                untouched &= dst.iter().all(|&b| b == 0x5A);
            }
            for b0 in [0x0Bu8, 0x0C, 0x0D, 0x0E, 0x0F] {
                let mut src = [0u8; 8];
                src[0] = b0;
                let mut dst = poison(4096);
                let r = unsafe {
                    f(dst.as_mut_ptr() as *mut c_void, dst.len(), src.as_ptr() as *const c_void, 4)
                };
                rets.push(res(l, r));
                untouched &= dst.iter().all(|&b| b == 0x5A);
            }
            assert!(untouched, "[{}] {v}_decompress wrote into dst", l.tag);
            (rets, untouched)
        });
    }
}

/// The `HUFv0x_decompress` family's up-front short-circuits — the only branches
/// that never touch a Huffman table.
///
/// SAFE and behaviourally *asymmetric*, which is the point:
///  * v0.5 guards with `cSrcSize >= dstSize -> corruption_detected`
///    (`zstd_v05.c:2476`), making the "not compressed" case
///    `cSrcSize == dstSize` an **error**, and has no raw-copy short-circuit;
///  * v0.6/v0.7 use a strict `cSrcSize > dstSize`, then `== dstSize` is a raw
///    `memcpy`, then `cSrcSize == 1` is an RLE `memset`
///    (`zstd_v06.c:2595`, `zstd_v07.c:2469`);
///  * `HUFv07_decompress4X_hufOnly` rejects *both* RLE and uncompressed with
///    `(cSrcSize >= dstSize) || (cSrcSize <= 1)` (`zstd_v07.c:2499`).
/// CONFIGS rows 426, 427, 428.
#[test]
fn legacy_huf_decompress_shortcircuits() {
    covers(&[
        "CFG:426,CFG:427,CFG:428",
        "ERR:legacy/zstd_v05.c:2475,ERR:legacy/zstd_v05.c:2476",
        "ERR:legacy/zstd_v06.c:2595/2596",
        "ERR:legacy/zstd_v07.c:2469/2470,ERR:legacy/zstd_v07.c:2485/2486",
        "ERR:legacy/zstd_v07.c:2499/2500,ERR:legacy/zstd_v07.c:2511/2512",
    ]);
    const CAP: usize = 4096;
    /// `HUFv07_DTABLE_SIZE(12)` U32s with `DTable[0]` seeded exactly the way
    /// `HUFv07_CREATE_STATIC_DTABLEX2` does.
    fn huf07_dtable() -> Vec<u32> {
        let mut dt = vec![0u32; 1 + (1usize << 12)];
        dt[0] = 12 * 0x0100_0001;
        dt
    }
    let payload = corpus(Corpus::Random, 128, 0x427);

    // ---- the plain 3-arg entry points -------------------------------------
    for v in ["HUFv05", "HUFv06", "HUFv07"] {
        diff_bytes(&format!("{v}_decompress short-circuits"), |l| {
            let f = l.sym::<FnDec4>(&format!("{v}_decompress"));
            let mut rets: Vec<R> = Vec::new();
            let mut dsts: Vec<u8> = Vec::new();
            let cases: &[(usize, usize)] = &[(0, 0), (0, 1), (64, 64), (64, 65), (64, 100), (64, 1)];
            for &(dsize, csize) in cases {
                let mut dst = poison(CAP);
                let r = unsafe {
                    f(
                        dst.as_mut_ptr() as *mut c_void,
                        dsize,
                        payload.as_ptr() as *const c_void,
                        csize,
                    )
                };
                rets.push(res(l, r));
                dsts.append(&mut dst);
            }
            // the RLE short-circuit for three distinct fill bytes
            for &b in &[0x00u8, 0x41, 0xFF] {
                let src = [b; 1];
                let mut dst = poison(CAP);
                let r = unsafe {
                    f(dst.as_mut_ptr() as *mut c_void, 64, src.as_ptr() as *const c_void, 1)
                };
                rets.push(res(l, r));
                dsts.append(&mut dst);
            }
            (rets, Blob(dsts))
        });
    }
    // ---- the v0.7 `_DCtx` / `_hufOnly` variants ---------------------------
    for name in [
        "HUFv07_decompress4X_DCtx",
        "HUFv07_decompress1X_DCtx",
        "HUFv07_decompress4X_hufOnly",
    ] {
        diff_bytes(&format!("{name} short-circuits"), |l| {
            let f = l.sym::<FnHufDCtx>(name);
            let mut rets: Vec<R> = Vec::new();
            let mut dsts: Vec<u8> = Vec::new();
            let mut tables: Vec<u8> = Vec::new();
            let cases: &[(usize, usize)] = &[(0, 0), (64, 0), (64, 1), (64, 64), (64, 65), (64, 100)];
            for &(dsize, csize) in cases {
                let mut dt = huf07_dtable();
                let mut dst = poison(CAP);
                let r = unsafe {
                    f(
                        dt.as_mut_ptr() as *mut c_void,
                        dst.as_mut_ptr() as *mut c_void,
                        dsize,
                        payload.as_ptr() as *const c_void,
                        csize,
                    )
                };
                rets.push(res(l, r));
                dsts.append(&mut dst);
                tables.extend(dt.iter().flat_map(|x| x.to_le_bytes()));
            }
            (rets, Blob(dsts), Blob(tables))
        });
    }
}

/// `HUFv07_selectDecoder` is a pure function of two sizes over the 16-row
/// `algoTime` constant table (`zstd_v07.c:2449-2459`).
///
/// SAFE only within its *documented but unchecked* precondition
/// `0 < cSrcSize < dstSize`: `dstSize == 0` divides by zero (SIGFPE) and
/// `cSrcSize >= dstSize` makes `Q >= 16` and reads `algoTime[Q]` out of bounds.
/// Every pair below satisfies `1 <= cSrcSize < dstSize`, which forces
/// `Q = cSrcSize * 16 / dstSize <= 15`.  CONFIGS row 429.
#[test]
fn legacy_huf_select_decoder() {
    covers(&["CFG:429"]);
    diff_bytes("HUFv07_selectDecoder grid", |l| {
        let f = l.sym::<FnSelectDecoder>("HUFv07_selectDecoder");
        let mut out: Vec<u8> = Vec::new();
        for &dst in &[16usize, 256, 1024, 4096, 65536, 131_072] {
            let mut cs: Vec<usize> = vec![1, 2, 16, 64, 255];
            cs.extend_from_slice(&[dst / 16, dst / 8, dst / 4, dst / 2, dst - 1]);
            // sweep the exact Q boundaries too: ceil(q*dst/16) for q = 0..15
            for q in 0..16usize {
                cs.push(q * dst / 16);
                cs.push(q * dst / 16 + 1);
            }
            for c in cs {
                if c == 0 || c >= dst {
                    continue; // outside the documented precondition
                }
                let r = unsafe { f(dst, c) };
                out.extend_from_slice(&r.to_le_bytes());
                assert!(r <= 1, "selectDecoder must return 0 or 1, got {r}");
            }
        }
        Blob(out)
    });
}

/// `HUFv0x_readDTableX2` / `X4` and `HUFv07_readStats` with `srcSize == 0`.
///
/// SAFE: `HUFv0x_readStats`'s first statement is
/// `if (!srcSize) return ERROR(srcSize_wrong);` before `iSize = ip[0]`
/// (`zstd_v05.c:1753`, `zstd_v07.c:1260`), and `readDTableX2`/`X4` forward it
/// immediately. Any `srcSize > 0` of arbitrary bytes enters the `iSize >= 242`
/// RLE branch that indexes `l[iSize-242]` and memsets `hwSize` weights without
/// re-checking `srcSize` — `UNSAFE-UB`, hence excluded.
///
/// Note v0.5/v0.6 `readDTableX4` read `memLog = DTable[0]` and reject
/// `memLog > 16` *before* calling `readStats`, so `DTable[0]` is seeded to the
/// version's static max tableLog (12) rather than left poisoned.
/// CONFIGS row 430.
#[test]
fn legacy_huf_read_dtable_empty() {
    covers(&[
        "CFG:430",
        "ERR:legacy/zstd_v05.c:1753/1767/1775,ERR:legacy/zstd_v05.c:1836",
        "ERR:legacy/zstd_v05.c:2160/2167,ERR:legacy/zstd_v06.c:1967",
        "ERR:legacy/zstd_v06.c:2286/2293,ERR:legacy/zstd_v07.c:1739",
        "ERR:legacy/zstd_v07.c:2095/2102,ERR:legacy/zstd_v07.c:1260/1274/1283",
        // CFG:431 is the CONFIGS row that *states* the HUF exclusion; discharged
        // by the srcSize == 0 case above plus `mod exclusions`.
        "CFG:431",
    ]);
    let src = [0xA5u8; 32];
    // X2 tables are U16* in v05/v06, U32-based HUFv07_DTable in v07.
    for v in ["HUFv05", "HUFv06"] {
        diff_bytes(&format!("{v}_readDTableX2 srcSize=0"), |l| {
            let f = l.sym::<FnHufReadDTable>(&format!("{v}_readDTableX2"));
            let mut dt = vec![0x5A5Au16; 1 + (1usize << 12)];
            dt[0] = 12;
            let r = unsafe { f(dt.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 0) };
            (res(l, r), Blob(dt.iter().flat_map(|x| x.to_le_bytes()).collect()))
        });
        diff_bytes(&format!("{v}_readDTableX4 srcSize=0"), |l| {
            let f = l.sym::<FnHufReadDTable>(&format!("{v}_readDTableX4"));
            let mut dt = vec![0x5A5A_5A5Au32; 1 + (1usize << 12)];
            dt[0] = 12;
            let r = unsafe { f(dt.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 0) };
            (res(l, r), Blob(dt.iter().flat_map(|x| x.to_le_bytes()).collect()))
        });
    }
    for name in ["HUFv07_readDTableX2", "HUFv07_readDTableX4"] {
        diff_bytes(&format!("{name} srcSize=0"), |l| {
            let f = l.sym::<FnHufReadDTable>(name);
            let mut dt = vec![0x5A5A_5A5Au32; 1 + (1usize << 12)];
            dt[0] = 12 * 0x0100_0001;
            let r = unsafe { f(dt.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 0) };
            (res(l, r), Blob(dt.iter().flat_map(|x| x.to_le_bytes()).collect()))
        });
    }
    diff_bytes("HUFv07_readStats srcSize=0", |l| {
        let f = l.sym::<FnReadStats>("HUFv07_readStats");
        let mut hw = poison(256);
        let mut rank = vec![0x5A5A_5A5Au32; 17];
        let mut nbs: c_uint = 0x5A5A_5A5A;
        let mut tl: c_uint = 0x5A5A_5A5A;
        let r = unsafe {
            f(
                hw.as_mut_ptr(),
                hw.len(),
                rank.as_mut_ptr(),
                &mut nbs,
                &mut tl,
                src.as_ptr() as *const c_void,
                0,
            )
        };
        let mut out = hw.clone();
        out.extend(rank.iter().flat_map(|x| x.to_le_bytes()));
        out.extend_from_slice(&nbs.to_le_bytes());
        out.extend_from_slice(&tl.to_le_bytes());
        (res(l, r), Blob(out))
    });
}

/// `ZBUFFv0x` size helpers and context lifecycle: four *independent* per-version
/// definitions of DInSize/DOutSize (`BLOCKSIZE + 3`,
/// `BLOCKSIZE + ZBUFFv05_blockHeaderSize`,
/// `ZSTDv06_BLOCKSIZE_MAX + ZSTDv06_blockHeaderSize`,
/// `ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + ZSTDv07_blockHeaderSize`) that all have to
/// come out at 131075 / 131072.  CONFIGS row 416.
#[test]
fn legacy_zbuff_lifecycle() {
    covers(&[
        "CFG:416",
        "ERR:legacy/zstd_v04.c:3336,ERR:legacy/zstd_v05.c:3816",
        "ERR:legacy/zstd_v06.c:3932,ERR:legacy/zstd_v07.c:4307",
        "ERR:legacy/zstd_v06.c:3919/3924,ERR:legacy/zstd_v07.c:4293/4296/4300",
    ]);
    for v in ["ZBUFFv04", "ZBUFFv05", "ZBUFFv06", "ZBUFFv07"] {
        let sizes = diff(&format!("{v} recommended sizes"), |l| {
            let i = l.sym::<FnS0>(&format!("{v}_recommendedDInSize"));
            let o = l.sym::<FnS0>(&format!("{v}_recommendedDOutSize"));
            unsafe { (i(), o()) }
        });
        assert_eq!(sizes, (DSTREAM_IN_SIZE, BLOCKSIZE_MAX), "{v} size helpers");
        diff(&format!("{v} create/free"), |l| {
            let c = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let f = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let d = unsafe { c() };
            let ok = !d.is_null();
            let a = unsafe { f(d) };
            let b = unsafe { f(std::ptr::null_mut()) };
            (ok, a, b)
        });
    }
}

/// `case ZBUFFds_init : return ERROR(init_missing);` is the first case of every
/// `ZBUFFv0x_decompressContinue` switch (`zstd_v04.c:3391`, `zstd_v05.c:3856`,
/// `zstd_v06.c:3985`, `zstd_v07.c:4360`), reached because `createDCtx` leaves
/// `stage == ZBUFFds_init`.
///
/// SAFE: nothing is read from `src`, and both `size_t*` out-params are left
/// untouched (they are only written on the way out of the state machine).
/// CONFIGS row 417.
#[test]
fn legacy_zbuff_init_missing() {
    covers(&[
        "CFG:417",
        "ERR:legacy/zstd_v04.c:3391,ERR:legacy/zstd_v05.c:3856",
        "ERR:legacy/zstd_v06.c:3985,ERR:legacy/zstd_v07.c:4360",
    ]);
    for v in ["ZBUFFv04", "ZBUFFv05", "ZBUFFv06", "ZBUFFv07"] {
        diff_bytes(&format!("{v}_decompressContinue no-init"), |l| {
            let c = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let f = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let cont = l.sym::<FnZbCont>(&format!("{v}_decompressContinue"));
            let d = unsafe { c() };
            let src = [0u8; 32];
            let mut rets: Vec<(R, SizeT, SizeT)> = Vec::new();
            let mut dsts: Vec<u8> = Vec::new();
            for &sn in &[0usize, 8] {
                let mut dst = poison(4096);
                let mut cap = dst.len();
                let mut got = sn;
                let r = unsafe {
                    cont(
                        d,
                        dst.as_mut_ptr() as *mut c_void,
                        &mut cap,
                        src.as_ptr() as *const c_void,
                        &mut got,
                    )
                };
                rets.push((res(l, r), cap, got));
                dsts.append(&mut dst);
            }
            let fr = unsafe { f(d) };
            (rets, fr, Blob(dsts))
        });
    }
}

/// The **pre-header** configurations of `ZBUFFv0x_decompressContinue`, which are
/// the only safe ones once a context has been initialised.
///
/// SAFE because everything below stops inside the header state machine:
///  * v04/v05 set `stage = ZBUFFds_readHeader`, whose first act is
///    `ZSTD_getFrameParams(&params, src, *srcSizePtr)`; for `*srcSizePtr == 0`
///    that returns 5 without reading, does a zero-length `memcpy`, sets
///    `*maxDstSizePtr = 0` and returns `headerSize - hPos == 5`
///    (`zstd_v04.c:3396`, `zstd_v05.c:3862`).
///  * v06/v07 set `stage = ZBUFFds_loadHeader` (there is no readHeader stage) and
///    their hint adds `blockHeaderSize` (3) to the missing header bytes, so the
///    zero-input return is 8, not 5 (`zstd_v06.c:3990`, `zstd_v07.c:4365`).
///  * a 5-byte foreign magic makes `getFrameParams` return `prefix_unknown`,
///    which v06/v07 return **directly** — no `corruption_detected` collapse,
///    unlike the `ZSTDv0x_decompressFrame` path.
///  * with each version's own magic and a zero frame descriptor the header is
///    accepted, the in/out buffers are allocated, and the first block header
///    (`00 00 00`) yields `expected == 0`, which the `ZBUFFds_read` stage treats
///    as end-of-frame. No literal or sequence bitstream is ever entered.
/// CONFIGS rows 418, 419.
#[test]
fn legacy_zbuff_pre_header() {
    covers(&[
        "CFG:418,CFG:419",
        "ERR:legacy/zstd_v04.c:3396/3417,ERR:legacy/zstd_v04.c:2505",
        "ERR:legacy/zstd_v05.c:3862,ERR:legacy/zstd_v05.c:3694",
        "ERR:legacy/zstd_v06.c:3833,ERR:legacy/zstd_v07.c:4121",
    ]);
    let rawd = raw_dict();
    let short7 = corpus(Corpus::Counter, 7, 1);
    let versions: &[(&str, u32, &str)] = &[
        ("ZBUFFv04", MAGIC_V04, "ZBUFFv04_decompressWithDictionary"),
        ("ZBUFFv05", MAGIC_V05, "ZBUFFv05_decompressInitDictionary"),
        ("ZBUFFv06", MAGIC_V06, "ZBUFFv06_decompressInitDictionary"),
        ("ZBUFFv07", MAGIC_V07, "ZBUFFv07_decompressInitDictionary"),
    ];
    for (v, mine, initdict) in versions {
        // (a) init -> zero-length Continue -> 5-byte foreign magic
        for (pn, magic) in MAGICS {
            if magic == mine {
                continue; // handled by (c)
            }
            diff_bytes(&format!("{v} pre-header magic={pn}"), |l| {
                let c = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let f = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let init = l.sym::<FnS1p>(&format!("{v}_decompressInit"));
                let cont = l.sym::<FnZbCont>(&format!("{v}_decompressContinue"));
                let d = unsafe { c() };
                let i = unsafe { init(d) };
                let mut trace: Vec<u8> = Vec::new();
                rec(&mut trace, &[i, 0, 0]);
                let src32 = magic_buf(*magic, 32);
                for &sn in &[0usize, 5] {
                    let mut dst = poison(4096);
                    let mut cap = dst.len();
                    let mut got = sn;
                    let r = unsafe {
                        cont(
                            d,
                            dst.as_mut_ptr() as *mut c_void,
                            &mut cap,
                            src32.as_ptr() as *const c_void,
                            &mut got,
                        )
                    };
                    rec(&mut trace, &[r, cap, got]);
                    trace.extend_from_slice(&dst[..64]);
                }
                let fr = unsafe { f(d) };
                rec(&mut trace, &[fr, 0, 0]);
                Blob(trace)
            });
        }
        // (b) initDictionary variants
        diff(&format!("{v} initDictionary"), |l| {
            let c = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let f = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let init = l.sym::<FnS1p>(&format!("{v}_decompressInit"));
            let idict = l.sym::<FnSpd>(initdict);
            let d = unsafe { c() };
            let i0 = unsafe { init(d) };
            let a = unsafe { idict(d, std::ptr::null(), 0) };
            let b = unsafe { idict(d, short7.as_ptr() as *const c_void, short7.len()) };
            let cc = unsafe { idict(d, rawd.as_ptr() as *const c_void, rawd.len()) };
            let fr = unsafe { f(d) };
            (res(l, i0), res(l, a), res(l, b), res(l, cc), fr)
        });
        // (c) own magic + reserved-bit frame descriptors, then the accepted
        //     zero descriptor whose first block header ends the frame.
        for fhd in [0x00u8, 0x08, 0x20, 0xC0, 0xFF] {
            diff_bytes(&format!("{v} own-magic fhd={fhd:#02x}"), |l| {
                let c = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let f = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let init = l.sym::<FnS1p>(&format!("{v}_decompressInit"));
                let cont = l.sym::<FnZbCont>(&format!("{v}_decompressContinue"));
                let d = unsafe { c() };
                let i = unsafe { init(d) };
                let mut trace: Vec<u8> = Vec::new();
                rec(&mut trace, &[i, 0, 0]);
                let mut src = magic_buf(*mine, 32);
                src[4] = fhd;
                let mut pos = 0usize;
                for _ in 0..6 {
                    let mut dst = poison(131_072);
                    let mut cap = dst.len();
                    let mut got = (src.len() - pos).min(8);
                    let r = unsafe {
                        cont(
                            d,
                            dst.as_mut_ptr() as *mut c_void,
                            &mut cap,
                            src[pos..].as_ptr() as *const c_void,
                            &mut got,
                        )
                    };
                    rec(&mut trace, &[r, cap, got]);
                    trace.extend_from_slice(&dst[..64]);
                    if zbuff_is_err(l, r) || r == 0 {
                        break;
                    }
                    pos += got;
                    if got == 0 && cap == 0 {
                        break;
                    }
                }
                let fr = unsafe { f(d) };
                rec(&mut trace, &[fr, 0, 0]);
                Blob(trace)
            });
        }
    }
}

/// `ZSTDv07_createDDict` / `freeDDict` and `ZSTDv0x_decompressBegin_usingDict`.
///
/// SAFE: `ZSTDv07_decompress_insertDictionary` short-circuits with
/// `if (dictSize < 8) return ZSTDv07_refDictContent(...)` and then
/// `if (MEM_readLE32(dict) != ZSTDv07_DICT_MAGIC) return refDictContent(...)`
/// (`zstd_v07.c:4093-4098`); the raw-content path is pure pointer bookkeeping
/// and cannot fault. Only a buffer that *does* start with the version's DICT
/// magic reaches `ZSTDv07_loadEntropy`, whose `HUFv07_readDTableX4` /
/// `FSEv07_readNCount` failures become `dictionary_corrupted` — and a magic
/// followed by garbage is exactly what that path is designed to reject, so it is
/// the one entropy-loading input that is in-contract.
/// `decompressBegin_usingDict` gates everything on `if (dict && dictSize)`.
/// CONFIGS row 415.
#[test]
fn legacy_ddict_and_begin_using_dict() {
    covers(&[
        "CFG:415",
        "ERR:legacy/zstd_v07.c:4140/4150/4159,ERR:legacy/zstd_v07.c:4081-4084",
        "ERR:legacy/zstd_v05.c:3694,ERR:legacy/zstd_v06.c:3833,ERR:legacy/zstd_v07.c:4121",
    ]);
    let rawd = raw_dict();
    let one = [0xABu8; 1];
    let seven = [0xABu8; 7];
    // A buffer whose first four bytes are each version's DICT magic followed by
    // a dictID and then garbage.
    let magic_dict = |m: u32| -> Vec<u8> {
        let mut v = magic_buf(m, 4096);
        v[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        for (i, b) in v[8..].iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        v
    };
    let dict_magic_v05: u32 = 0xEC30_A435;
    let dict_magic_v06: u32 = 0xEC30_A436;
    let dict_magic_v07: u32 = 0xEC30_A437;

    diff("ZSTDv07_createDDict", |l| {
        let cdd = l.sym::<FnCreateDDict>("ZSTDv07_createDDict");
        let fdd = l.sym::<FnS1p>("ZSTDv07_freeDDict");
        let mut out: Vec<(bool, SizeT)> = Vec::new();
        let cases: Vec<(&str, &[u8])> = vec![
            ("empty", &one[..0]),
            ("one", &one[..]),
            ("seven", &seven[..]),
            ("raw4096", &rawd[..]),
        ];
        for (_n, d) in cases {
            let dd = unsafe { cdd(d.as_ptr() as *const c_void, d.len()) };
            let ok = !dd.is_null();
            let r = if ok { unsafe { fdd(dd) } } else { 0 };
            out.push((ok, r));
        }
        // dictSize == 0 with a non-NULL buffer
        let dd = unsafe { cdd(one.as_ptr() as *const c_void, 0) };
        let ok = !dd.is_null();
        let r = if ok { unsafe { fdd(dd) } } else { 0 };
        out.push((ok, r));
        // the DICT-magic-then-garbage path (must be rejected -> NULL)
        let md = magic_dict(dict_magic_v07);
        let dd = unsafe { cdd(md.as_ptr() as *const c_void, md.len()) };
        let ok = !dd.is_null();
        let r = if ok { unsafe { fdd(dd) } } else { 0 };
        out.push((ok, r));
        out
    });

    for (v, dm) in [
        ("ZSTDv05", dict_magic_v05),
        ("ZSTDv06", dict_magic_v06),
        ("ZSTDv07", dict_magic_v07),
    ] {
        let md = magic_dict(dm);
        diff(&format!("{v}_decompressBegin_usingDict"), |l| {
            let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
            let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
            let bd = l.sym::<FnSpd>(&format!("{v}_decompressBegin_usingDict"));
            let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
            let skip = if v == "ZSTDv07" {
                Some(l.sym::<FnI1p>("ZSTDv07_isSkipFrame"))
            } else {
                None
            };
            let mut out: Vec<(R, SizeT, c_int)> = Vec::new();
            let cases: Vec<(*const c_void, usize)> = vec![
                (std::ptr::null(), 0),
                (one.as_ptr() as *const c_void, 0),
                (seven.as_ptr() as *const c_void, 7),
                (rawd.as_ptr() as *const c_void, rawd.len()),
                (md.as_ptr() as *const c_void, md.len()),
            ];
            for (p, n) in cases {
                let d = unsafe { create() };
                let r = unsafe { bd(d, p, n) };
                let e = unsafe { next(d) };
                let s = skip.as_ref().map(|f| unsafe { f(d) }).unwrap_or(-1);
                unsafe { free(d) };
                out.push((res(l, r), e, s));
            }
            out
        });
    }
}

// ---------------------------------------------------------------------------
// The legacy DISPATCH  (this is the part that is reachable from the public API)
// ---------------------------------------------------------------------------

/// With `ZSTD_LEGACY_SUPPORT == 5`, `zstd_legacy.h` compiles out the `case`
/// labels for v0.1..v0.4 (`#if (ZSTD_LEGACY_SUPPORT <= N)`), so `ZSTD_isLegacy`
/// recognises **only** the v05/v06/v07 magic numbers and returns 0 for
/// `0x1EB52FFD` / `0xFD2FB522` / `0xFD2FB523` / `0xFD2FB524`.  Those four
/// therefore fall to `ZSTD_decompressLegacy`'s `default : return
/// ERROR(prefix_unknown);` (`zstd_legacy.h:190-191`) — code **10**, *not*
/// `version_unsupported` (12) — and to `ZSTD_findFrameSizeInfoLegacy`'s
/// `compressedSize = ERROR(prefix_unknown)` / `decompressedBound =
/// ZSTD_CONTENTSIZE_ERROR` (`:251-252`).  This test pins that through every
/// public entry point that consults `ZSTD_isLegacy`.
///
/// SAFE: for the v05/v06/v07 magics the payload is all zeros, and the routing
/// stops inside the header/block-header walk. `ZSTD_findFrameCompressedSizeLegacy`
/// runs *first* (`zstd_decompress.c:1092`) and is a pure block-header walk (see
/// `legacy_find_frame_size_info`); when it does succeed (v05/v06 report an
/// 8-byte empty frame) the subsequent `ZSTD_decompressLegacy` reaches
/// `ZSTDv0x_decompressBlock_internal` with `srcSize == 0`, and
/// `decodeLiteralsBlock`'s `srcSize < MIN_CBLOCK_SIZE (3)` guard
/// (`zstd_v05.c:2816`, `zstd_v06.c:3004`, `zstd_v07.c:3234`) rejects it before
/// any Huffman/FSE read.  CONFIGS row 432.
#[test]
fn legacy_dispatch_oneshot() {
    covers(&[
        "CFG:432",
        "ERR:legacy/zstd_legacy.h:59,ERR:legacy/zstd_legacy.h:84",
        "ERR:legacy/zstd_legacy.h:191,ERR:legacy/zstd_legacy.h:251-252",
        "ERR:legacy/zstd_legacy.h:256-257",
        "ERR:decompress/zstd_decompress.c:395",
    ]);
    const CAP: usize = 65536;
    let sizes: &[usize] = &[0, 1, 3, 4, 5, 6, 7, 8, 9, 12, 16, 32];
    for (pn, magic) in MAGICS {
        diff(&format!("legacy dispatch magic={pn}"), |l| {
            let dec = l.sym::<FnDecompress>("ZSTD_decompress");
            let gfcs = l.sym::<FnGetFrameContentSize>("ZSTD_getFrameContentSize");
            let ffcs = l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize");
            let dbound = l.sym::<FnUll2>("ZSTD_decompressBound");
            let isframe = l.sym::<FnU0Buf>("ZSTD_isFrame");
            let mut out: Vec<(usize, R, c_ulonglong, R, c_ulonglong, c_uint)> = Vec::new();
            for &n in sizes {
                let src = magic_buf(*magic, n);
                let mut dst = poison(CAP);
                let a = unsafe {
                    dec(
                        dst.as_mut_ptr() as *mut c_void,
                        CAP,
                        src.as_ptr() as *const c_void,
                        n,
                    )
                };
                let b = unsafe { gfcs(src.as_ptr() as *const c_void, n) };
                let c = unsafe { ffcs(src.as_ptr() as *const c_void, n) };
                let d = unsafe { dbound(src.as_ptr() as *const c_void, n) };
                let e = unsafe { isframe(src.as_ptr() as *const c_void, n) };
                out.push((n, res(l, a), b, res(l, c), d, e));
            }
            out
        });
        // the same through an explicit DCtx
        diff(&format!("legacy dispatch DCtx magic={pn}"), |l| {
            let dctx = Ctx::dctx(l);
            let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let mut out: Vec<(usize, R)> = Vec::new();
            for &n in sizes {
                let src = magic_buf(*magic, n);
                let mut dst = poison(CAP);
                let r = unsafe {
                    f(
                        dctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        CAP,
                        src.as_ptr() as *const c_void,
                        n,
                    )
                };
                out.push((n, res(l, r)));
            }
            out
        });
    }
    // Pin the exact codes the build configuration implies, on the C side.
    let c = &pair().c;
    let dec = c.sym::<FnDecompress>("ZSTD_decompress");
    let mut dst = vec![0u8; CAP];
    for (pn, magic, want) in [
        ("v01", MAGIC_V01_LE, 10),
        ("v02", MAGIC_V02, 10),
        ("v03", MAGIC_V03, 10),
        ("v04", MAGIC_V04, 10),
    ] {
        let src = magic_buf(magic, 12);
        let n = unsafe {
            dec(dst.as_mut_ptr() as *mut c_void, CAP, src.as_ptr() as *const c_void, 12)
        };
        assert_eq!(
            err_code(c, n),
            want,
            "{pn} magic must be prefix_unknown (10), not version_unsupported (12)"
        );
    }
    // ... and that the v05/v06/v07 magics really are routed into their *own*
    // decoders, which is provable from the error code: a reserved frame
    // descriptor bit produces `frameParameter_unsupported` (14) in v0.5, which
    // v0.6/v0.7 collapse into `corruption_detected` (20)
    // (`zstd_v06.c:3521`, `zstd_v07.c:3757`).  The modern decoder can produce
    // neither for these inputs.
    for (pn, magic, fhd, want) in [
        ("v05", MAGIC_V05, 0x10u8, 14),
        ("v06", MAGIC_V06, 0x20u8, 20),
        ("v07", MAGIC_V07, 0x08u8, 20),
    ] {
        // magic + reserved-bit descriptor + a bt_end block header: a structurally
        // complete, minimal frame, so `findFrameCompressedSizeLegacy` succeeds
        // and the per-version header decoder is what rejects it.
        let mut src = magic_buf(magic, if pn == "v07" { 9 } else { 8 });
        src[4] = fhd;
        let bh = src.len() - 3;
        src[bh] = 0xC0;
        let n = unsafe {
            dec(
                dst.as_mut_ptr() as *mut c_void,
                CAP,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        assert_eq!(
            err_code(c, n),
            want,
            "{pn} magic must be decoded by the v0.{} header parser, got {:?}",
            &pn[2..],
            res(c, n)
        );
    }
}

/// A legacy `bt_raw` / `bt_rle` / `bt_end` block header (`zstd_v05.c:2776-2801`
/// and the v06/v07 equivalents): a 2-bit block type in the top of byte 0 and a
/// 19-bit size spread over `(in[0] & 7) << 16 | in[1] << 8 | in[2]`.
fn legacy_block_header(block_type: u8, size: usize) -> [u8; 3] {
    [
        (block_type << 6) | ((size >> 16) & 7) as u8,
        ((size >> 8) & 0xFF) as u8,
        (size & 0xFF) as u8,
    ]
}

/// Build a structurally *valid* v0.5 / v0.6 / v0.7 frame out of uncompressed
/// (`bt_raw`) blocks — plus `bt_rle` blocks for v0.7, which is the only version
/// that implements them (v0.5/v0.6 return `ERROR(GENERIC)` "not yet supported",
/// `zstd_v05.c:3414`, `zstd_v06.c:3546`).
///
/// The frame headers are the minimal legal ones:
///  * v0.5: `magic || windowDescriptor`, low nibble = `windowLog - 11`, high
///    nibble reserved (`zstd_v05.c:2751-2760`);
///  * v0.6: `magic || frameDesc`, low nibble = `windowLog - 12`, bit 5 reserved,
///    bits 6-7 = fcsId (0 => no content-size field) (`zstd_v06.c:2924-2950`);
///  * v0.7: `magic || fhdByte || wlByte`, fhdByte 0 => no dictID, no checksum,
///    `directMode == 0` so a window byte follows and fcsID 0 means *no*
///    content-size field (`zstd_v07.c:3095-3168`).
///
/// Returns `(frame, expected_plaintext)`.
fn legacy_raw_frame(ver: u8, blocks: &[(u8, Vec<u8>, usize)]) -> (Vec<u8>, Vec<u8>) {
    let mut f: Vec<u8> = Vec::new();
    let mut plain: Vec<u8> = Vec::new();
    match ver {
        5 => {
            f.extend_from_slice(&MAGIC_V05.to_le_bytes());
            f.push(0x00); // windowLog 11
        }
        6 => {
            f.extend_from_slice(&MAGIC_V06.to_le_bytes());
            f.push(0x00); // windowLog 12, fcsId 0
        }
        _ => {
            f.extend_from_slice(&MAGIC_V07.to_le_bytes());
            f.push(0x00); // dictID 0, no checksum, directMode 0, fcsID 0
            f.push(0x10); // windowLog (0x10>>3)+10 == 12
        }
    }
    for (bt, data, rep) in blocks {
        match *bt {
            1 => {
                f.extend_from_slice(&legacy_block_header(1, data.len()));
                f.extend_from_slice(data);
                plain.extend_from_slice(data);
            }
            2 => {
                // bt_rle : the header carries the *regenerated* size, the payload
                // is a single byte (`ZSTDv07_generateNxBytes`).
                f.extend_from_slice(&legacy_block_header(2, *rep));
                f.push(data[0]);
                plain.extend(std::iter::repeat(data[0]).take(*rep));
            }
            _ => unreachable!(),
        }
    }
    f.extend_from_slice(&legacy_block_header(3, 0)); // bt_end
    (f, plain)
}

/// PHASE B for the legacy dispatch: real, structurally valid v0.5/v0.6/v0.7
/// frames really do round-trip through the public `ZSTD_decompress*` API and
/// through each version's own `ZSTDv0x_decompress*`.
///
/// SAFE: every block is `bt_raw` (a bounds-checked `memcpy` in
/// `ZSTDv0x_copyRawBlock`, `zstd_v05.c:2808`, `zstd_v06.c:2987`,
/// `zstd_v07.c:3217`) or, for v0.7 only, `bt_rle` (a bounds-checked `memset` in
/// `ZSTDv07_generateNxBytes`, `zstd_v07.c:3728`), plus a terminating `bt_end`.
/// No Huffman or FSE bitstream is ever entered, so no unhardened read can occur —
/// and this is the *only* shape of legacy frame that can be constructed without
/// a legacy encoder (none is shipped).
#[test]
fn legacy_valid_raw_frames() {
    covers(&[
        "CFG:432",
        "ERR:legacy/zstd_v05.c:3414/3421,ERR:legacy/zstd_v06.c:3546/3553,ERR:legacy/zstd_v07.c:3219",
        "ERR:legacy/zstd_v07.c:3730",
        "ERR:legacy/zstd_legacy.h:191",
    ]);
    const CAP: usize = 262_144;
    let d1 = corpus(Corpus::Text, 4096, 0x432);
    let d2 = corpus(Corpus::Random, 777, 0x433);
    let d3 = corpus(Corpus::Counter, 1, 0x434);

    let mut cases: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
    for ver in [5u8, 6, 7] {
        for (name, blocks) in [
            ("empty", vec![]),
            ("one-raw", vec![(1u8, d1.clone(), 0usize)]),
            ("one-byte", vec![(1u8, d3.clone(), 0usize)]),
            (
                "three-raw",
                vec![
                    (1u8, d1.clone(), 0usize),
                    (1u8, d2.clone(), 0usize),
                    (1u8, d1[..17].to_vec(), 0usize),
                ],
            ),
        ] {
            let (f, p) = legacy_raw_frame(ver, &blocks);
            cases.push((format!("v0{ver}/{name}"), f, p));
        }
        // bt_rle: implemented only in v0.7, `ERROR(GENERIC)` in v0.5/v0.6.
        let (f, p) = legacy_raw_frame(ver, &[(2u8, vec![0xA7u8], 300usize)]);
        cases.push((format!("v0{ver}/rle300"), f, p));
        let (f, p) = legacy_raw_frame(
            ver,
            &[(1u8, d2.clone(), 0), (2u8, vec![0x00u8], 65_000), (1u8, d3.clone(), 0)],
        );
        cases.push((format!("v0{ver}/mixed"), f, p));
    }
    // two concatenated frames, to exercise the `continue` in
    // `ZSTD_decompressMultiFrame`'s legacy branch
    {
        let (mut a, mut pa) = legacy_raw_frame(5, &[(1u8, d1.clone(), 0)]);
        let (b, pb) = legacy_raw_frame(6, &[(1u8, d2.clone(), 0)]);
        a.extend_from_slice(&b);
        pa.extend_from_slice(&pb);
        cases.push(("v05+v06 concat".into(), a, pa));
    }

    let mut nb_decoded = 0usize;
    let mut nb_bytes = 0usize;
    for (name, frame, plain) in &cases {
        // (a) through the public dispatch, one-shot and DCtx
        let out = diff_bytes(&format!("legacy valid {name} ZSTD_decompress"), |l| {
            let (r, b) = decompress_simple(l, frame, CAP);
            (r, b)
        });
        if let R::Ok(n) = out.0 {
            nb_decoded += 1;
            nb_bytes += n;
            // v0.5/v0.6 reject bt_rle, so only compare when the frame decoded
            assert_eq!(n, plain.len(), "{name}: decoded size");
            assert_eq!(&out.1 .0[..], &plain[..], "{name}: decoded bytes");
        }
        diff(&format!("legacy valid {name} frame queries"), |l| {
            let gfcs = l.sym::<FnGetFrameContentSize>("ZSTD_getFrameContentSize");
            let ffcs = l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize");
            let dbound = l.sym::<FnUll2>("ZSTD_decompressBound");
            let isframe = l.sym::<FnU0Buf>("ZSTD_isFrame");
            let p = frame.as_ptr() as *const c_void;
            unsafe {
                (
                    gfcs(p, frame.len()),
                    res(l, ffcs(p, frame.len())),
                    dbound(p, frame.len()),
                    isframe(p, frame.len()),
                )
            }
        });
        // (b) through the version's own entry point, for the truncation sweep
        let ver = name.as_bytes()[2] - b'0';
        let v = format!("ZSTDv0{ver}");
        diff_bytes(&format!("legacy valid {name} {v}_decompress"), |l| {
            let f = l.sym::<FnDec4>(&format!("{v}_decompress"));
            let mut dst = poison(CAP);
            let r = unsafe {
                f(
                    dst.as_mut_ptr() as *mut c_void,
                    CAP,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                )
            };
            let rr = res(l, r);
            if let R::Ok(n) = rr {
                dst.truncate(n);
            }
            (rr, Blob(dst))
        });
        // (c) truncations: every prefix of the frame must be rejected by one of
        //     the size checks, never decoded.  Safe for the same reason as the
        //     full frame: the block *payloads* are raw/rle, so the only reads
        //     are the bounds-checked memcpy/memset and the header walk.
        diff(&format!("legacy valid {name} truncated"), |l| {
            let mut out: Vec<(usize, R)> = Vec::new();
            for n in 0..frame.len().min(40) {
                let (r, _) = decompress_simple(l, &frame[..n], CAP);
                out.push((n, r));
            }
            // and a few larger truncations
            for &n in &[frame.len() / 2, frame.len() - 1] {
                if n < frame.len() {
                    let (r, _) = decompress_simple(l, &frame[..n], CAP);
                    out.push((n, r));
                }
            }
            out
        });
        // (d) an output buffer that is too small must hit dstSize_tooSmall in
        //     copyRawBlock / generateNxBytes rather than overflow.
        if !plain.is_empty() {
            diff(&format!("legacy valid {name} tight dst"), |l| {
                let mut out: Vec<(usize, R)> = Vec::new();
                for &cap in &[0usize, 1, plain.len() / 2, plain.len() - 1, plain.len()] {
                    let (r, _) = decompress_simple(l, frame, cap);
                    out.push((cap, r));
                }
                out
            });
        }
    }
    // Guard against a vacuous pass: the raw/rle frames really must decode.
    assert!(
        nb_decoded >= 15 && nb_bytes >= 97_912,
        "legacy raw frames did not actually decode ({nb_decoded} frames, {nb_bytes} bytes)"
    );
}

/// The legacy *streaming* dispatch (`ZSTD_initLegacyStream` /
/// `ZSTD_decompressLegacyStream` / `ZSTD_freeLegacyStreamContext`,
/// `zstd_legacy.h:301-445`).  With `ZSTD_LEGACY_SUPPORT == 5` cases 1/2/3 are
/// folded into `default` and case 4 is compiled out, so a v0.4 magic never
/// reaches `ZBUFFv04` through this path; only 5/6/7 create a `ZBUFFv0x_DCtx`.
/// The `if (prevVersion != newVersion) ZSTD_freeLegacyStreamContext(...)` test
/// (`:311`) is exercised by re-feeding a *different* legacy magic after a
/// `ZSTD_DCtx_reset`, and the teardown-with-a-live-legacy-context path by
/// dropping the DStream mid-frame.
///
/// SAFE: see `legacy_zbuff_pre_header` — the all-zero payload keeps every
/// version inside its header state machine and the first `00 00 00` block header
/// makes `nextSrcSizeToDecompress` return 0, which the `ZBUFFds_read` stage
/// treats as end-of-frame.  CONFIGS row 433.
#[test]
fn legacy_dispatch_stream() {
    covers(&[
        "CFG:433",
        "ERR:legacy/zstd_legacy.h:284,ERR:legacy/zstd_legacy.h:319,ERR:legacy/zstd_legacy.h:387",
        "ERR:legacy/zstd_legacy.h:164/174/184,ERR:legacy/zstd_legacy.h:335/345/355",
    ]);
    const OUT: usize = 65536;
    let feed = |l: &Lib, magic: u32, n: usize, chunk: usize| -> Blob {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let src = magic_buf(magic, n);
        let mut dst = poison(OUT);
        let mut trace: Vec<u8> = Vec::new();
        let mut inb = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: 0,
            pos: 0,
        };
        let mut outb = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: OUT,
            pos: 0,
        };
        let mut fed = 0usize;
        for _ in 0..(n / chunk.max(1) + 4) {
            fed = (fed + chunk).min(n);
            inb.size = fed;
            let r = unsafe { f(ds.ptr, &mut outb, &mut inb) };
            rec(&mut trace, &[r, inb.pos, outb.pos]);
            if is_error(l, r) || r == 0 {
                break;
            }
            if inb.pos == fed && fed == n {
                break;
            }
        }
        trace.extend_from_slice(&dst[..256]);
        Blob(trace)
    };
    for (pn, magic) in MAGICS {
        for &chunk in &[1usize, 4, 8, 32] {
            diff_bytes(&format!("legacy stream magic={pn} chunk={chunk}"), |l| {
                feed(l, *magic, 32, chunk)
            });
        }
    }
    // The `prevVersion != newVersion` context swap: reset the DCtx between two
    // *different* legacy magics on the same DStream, then free it while a legacy
    // context is live.
    diff_bytes("legacy stream version swap", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let reset = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
        let mut trace: Vec<u8> = Vec::new();
        let mut dst = poison(OUT);
        for magic in [MAGIC_V05, MAGIC_V06, MAGIC_V07, MAGIC_V05, MAGIC_V04, MAGIC_V07] {
            let rr = unsafe { reset(ds.ptr, ZSTD_reset_session_only) };
            rec(&mut trace, &[rr, 0, 0]);
            let src = magic_buf(magic, 32);
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let mut outb = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: OUT,
                pos: 0,
            };
            for _ in 0..4 {
                let r = unsafe { f(ds.ptr, &mut outb, &mut inb) };
                rec(&mut trace, &[r, inb.pos, outb.pos]);
                if is_error(l, r) || r == 0 || inb.pos == inb.size {
                    break;
                }
            }
        }
        // ds is dropped here (ZSTD_freeDStream) with a live legacy context
        Blob(trace)
    });
    // `ZSTD_initStaticDCtx` + a legacy frame must be rejected with
    // memory_allocation (`zstd_decompress.c:1094`): legacy support is not
    // compatible with a static DCtx.
    diff("legacy + static DCtx", |l| {
        type FnInitStatic = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
        type FnEstimate = unsafe extern "C" fn() -> SizeT;
        let est = l.sym::<FnEstimate>("ZSTD_estimateDCtxSize");
        let sz = unsafe { est() };
        let mut ws = vec![0u8; sz + 64];
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticDCtx");
        let d = unsafe { init(ws.as_mut_ptr() as *mut c_void, ws.len()) };
        assert!(!d.is_null());
        let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let mut out: Vec<R> = Vec::new();
        for magic in [MAGIC_V05, MAGIC_V06, MAGIC_V07, MAGIC_V04] {
            let src = magic_buf(magic, 32);
            let mut dst = poison(4096);
            let r = unsafe {
                f(
                    d,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            out.push(res(l, r));
        }
        out
    });
}

// ---------------------------------------------------------------------------
// EXCLUSIONS
// ---------------------------------------------------------------------------

/// Legacy entry points that are **deliberately not exercised with arbitrary
/// input**, with the evidence from `c_src/`.  Kept in the source so the reason
/// travels with the suite.
///
/// ## Undefined behaviour on arbitrary bytes (no C behaviour to match)
///
/// * `ZSTDv01_decompressContinue`, `ZSTDv02_decompressContinue`,
///   `ZSTDv03_decompressContinue`, `ZSTDv04_decompressContinue`,
///   `ZSTDv05_decompressContinue`, `ZSTDv06_decompressContinue`,
///   `ZSTDv07_decompressContinue` — *only* the `srcSize != dctx->expected`
///   rejection and the first (header-parsing) call are tested
///   (`legacy_decompress_continue_guards`). Once the header has been accepted
///   the `ZSTDds_decompressBlock` stage calls
///   `ZSTDv0x_decompressBlock_internal` on caller bytes with no validation
///   beyond the 131072 size cap.
/// * `ZSTDv05_decompressBlock`, `ZSTDv06_decompressBlock`,
///   `ZSTDv07_decompressBlock` — only `srcSize >= 131072` is tested
///   (`legacy_decompress_block_size_cap`); that is the single check that runs
///   before `decodeLiteralsBlock` starts walking the bitstream
///   (`zstd_v05.c:3347`, `zstd_v06.c:3481`, `zstd_v07.c:3694`).
/// * `ZSTDv07_insertBlock` — blindly records `blockStart + blockSize` as decoder
///   history (`zstd_v07.c:3712-3716`); there is no validation at all, so any
///   call poisons the DCtx for every later call. Not called.
/// * `ZBUFFv04/05/06/07_decompressContinue` past the header — `case ZBUFFds_read`
///   / `case ZBUFFds_load` invoke `ZSTDv0x_decompressContinue` on the block
///   payload with no validation (`zstd_v05.c:3930`, `zstd_v06.c:4041`,
///   `zstd_v07.c:4416`). Only the pre-header configurations are used
///   (`legacy_zbuff_pre_header`).
/// * `FSEv05/06/07_buildDTable` — trusts that `normalizedCounter` sums to
///   `1 << tableLog`; the `position = (position + step) & tableMask` spreading
///   loop has no bound on caller counts (`zstd_v05.c:1197`). Not called.
/// * `FSEv05/06/07_decompress_usingDTable` — walks a `BITv0x_DStream` *backwards*
///   from `cSrc + cSrcSize` and trusts a caller-supplied DTable. Not called.
/// * `FSEv05/06/07_readNCount` beyond the two front guards — with
///   `byte0 & 0xF <= 10` it enters the count loop whose `if (ip < iend-5)` and
///   `if ((ip <= iend-7) || (ip + (bitCount>>3) <= iend-4))` pointer guards
///   under-flow for small `hbSize`. Only `hbSize < 4` and `nbBits > 15` are used.
/// * `HUFv05/06/07_readDTableX2` / `X4`, `HUFv07_readStats` beyond
///   `srcSize == 0` — `readStats` reads `iSize = ip[0]` and, in the
///   `iSize >= 242` RLE branch, indexes `l[iSize-242]` and memsets `hwSize`
///   weights without re-checking `srcSize` (`zstd_v07.c:1268-1273`).
/// * every `HUFv0x_decompress{1,4}X{2,4}[_DCtx][_usingDTable]` beyond the
///   `dstSize`/`cSrcSize` short-circuits — the 4-stream decoders derive four
///   `BITv0x_DStream_t` cursors from unvalidated 16-bit lengths at `istart`,
///   `istart+2`, `istart+4` and only bound the last
///   (`if (length4 > cSrcSize) return ERROR(corruption_detected);`), leaving the
///   first three unbounded.
/// * `HUFv07_decompress1X2_usingDTable` / `HUFv07_decompress1X4_usingDTable`
///   with a *matching* `DTable[0].tableType` — unlike the `4X` forms, the
///   `_internal` they tail-call has **no** minimum-`cSrcSize` guard, so it hands
///   `cSrc`/`cSrcSize` straight to `BITv07_initDStream` +
///   `HUFv07_decodeStreamX{2,4}`. Verified empirically: the reference C `.so`
///   SIGSEGVs on `dstSize = 4096`, `cSrcSize = 10` of `0xFF`. Only the
///   *mismatching* table type (which returns `ERROR(GENERIC)` after reading
///   `DTable[0]` alone, `zstd_v07.c:1842`/`:2254`) is exercised.
/// * `HUFv07_selectDecoder` outside `0 < cSrcSize < dstSize` — `dstSize == 0`
///   is a division by zero (SIGFPE) and `cSrcSize >= dstSize` gives `Q >= 16`,
///   an out-of-bounds `algoTime[Q]` read (`zstd_v07.c:2452`). The grid in
///   `legacy_huf_select_decoder` stays strictly inside the precondition.
///
/// ## Missing NULL checks (`UNSAFE-UB`)
///
/// * `ZSTDv07_freeDDict(NULL)` — dereferences `ddict->refContext` unconditionally
///   (`zstd_v07.c:4180`), unlike `ZSTDv07_freeDCtx` which does support
///   free-on-NULL. Only non-NULL DDicts are freed.
/// * `ZSTDv07_decompress_usingDDict(..., NULL)` — no NULL check on `ddict`
///   (`zstd_v07.c:4196`).
/// * the inner `ZSTD_createDCtx()` in `ZBUFFv04_createDCtx`
///   (`zstd_v04.c:3329`) and `ZBUFFv05_createDCtx` (`zstd_v05.c:3809`) is not
///   NULL-checked, so an allocation failure there is UB. Allocation failure is
///   not induced.
/// * `ZBUFF_compressContinue` / `ZBUFF_compressFlush` / `ZBUFF_compressEnd` /
///   `ZBUFF_decompressContinue` with a NULL `size_t*` — all four dereference
///   `dstCapacityPtr`/`srcSizePtr` with no check
///   (`zbuff_compress.c:122/125/142`, `zbuff_decompress.c:62/65`). Never passed.
///
/// ## Unreachable in this build configuration
///
/// * `ZSTD_freeLegacyStreamContext` / `ZSTD_initLegacyStream` /
///   `ZSTD_decompressLegacyStream` versions 1-4 (`zstd_legacy.h:284`, `:319`,
///   `:387`) — `ZSTD_isLegacy` can only ever yield 0, 5, 6 or 7 at
///   `ZSTD_LEGACY_SUPPORT == 5`, so the v0.1-v0.4 `case` labels are dead. Their
///   *existence* is instead pinned indirectly by `legacy_dispatch_stream`, which
///   shows a v0.4 magic never reaches `ZBUFFv04`.
/// * every `ERROR(memory_allocation)` on a legacy `malloc` failure
///   (`zstd_legacy.h:164/174/184`, `:335/345/355`, `zstd_v0x.c` `createDCtx`) —
///   allocation failure is not induced anywhere in this suite.
#[allow(dead_code)]
mod exclusions {}

/// The minimal legal frame header for every legacy version, as a byte vector.
///  * v0.1/v0.2/v0.3: `ZSTD_frameHeaderSize == 4`, i.e. the magic and nothing
///    else (`zstd_v01.c:1921`, `zstd_v02.c:3221`, `zstd_v03.c:2860`);
///  * v0.4/v0.5: magic + a window descriptor whose high nibble is reserved
///    (`zstd_v04.c:2510`, `zstd_v05.c:2758`);
///  * v0.6: magic + a frame descriptor (bit 5 reserved, bits 6-7 fcsId);
///  * v0.7: magic + fhdByte + a window byte (fhdByte 0 => `directMode == 0`).
fn legacy_header(ver: u8) -> Vec<u8> {
    match ver {
        1 => MAGIC_V01_LE.to_le_bytes().to_vec(),
        2 => MAGIC_V02.to_le_bytes().to_vec(),
        3 => MAGIC_V03.to_le_bytes().to_vec(),
        4 => {
            let mut v = MAGIC_V04.to_le_bytes().to_vec();
            v.push(0x00);
            v
        }
        5 => {
            let mut v = MAGIC_V05.to_le_bytes().to_vec();
            v.push(0x00);
            v
        }
        6 => {
            let mut v = MAGIC_V06.to_le_bytes().to_vec();
            v.push(0x00);
            v
        }
        _ => {
            let mut v = MAGIC_V07.to_le_bytes().to_vec();
            v.push(0x00);
            v.push(0x10);
            v
        }
    }
}

/// PHASE C for the legacy block-header walk, all seven versions.
///
/// SAFE: every case is rejected by an arithmetic/size check inside the
/// `getcBlockSize` loop, or by the bounds-checked `copyRawBlock`, before any
/// entropy decoding. Concretely this covers, per version:
///  * `blockSize > remainingSize` — a `bt_raw` header declaring more bytes than
///    the frame contains (`zstd_v01.c:1934`, `zstd_v02.c:3235`,
///    `zstd_v03.c:2874`, `zstd_v04.c:3054`, `zstd_v05.c:3403`,
///    `zstd_v06.c:3535`, `zstd_v07.c:3771`);
///  * `bt_rle` -> `ERROR(GENERIC)` "not yet supported" in v0.1..v0.6
///    (`zstd_v01.c:1945`, `zstd_v05.c:3414`, `zstd_v06.c:3546`) — v0.7 is the
///    only version that implements it;
///  * `bt_end` with `remainingSize != 0` — trailing bytes after the end-of-frame
///    block (`zstd_v01.c:1949`, `zstd_v05.c:3418`, `zstd_v07.c:3786`);
///  * a truncated block header (fewer than 3 bytes left) -> `srcSize_wrong` from
///    `getcBlockSize`;
///  * `dstCapacity` smaller than the raw block -> `dstSize_tooSmall` from
///    `copyRawBlock` (`zstd_v05.c:2801/2802`, `zstd_v06.c:2989/2990`,
///    `zstd_v07.c:3219`).
#[test]
fn legacy_malformed_block_headers() {
    covers(&[
        "ERR:legacy/zstd_v01.c:1934,ERR:legacy/zstd_v01.c:1945,ERR:legacy/zstd_v01.c:1949",
        "ERR:legacy/zstd_v02.c:3235/3250,ERR:legacy/zstd_v02.c:3246/3253",
        "ERR:legacy/zstd_v03.c:2874/2889,ERR:legacy/zstd_v03.c:2885/2892",
        "ERR:legacy/zstd_v04.c:3065/3072,ERR:legacy/zstd_v05.c:3414/3421",
        "ERR:legacy/zstd_v05.c:2801/2802,ERR:legacy/zstd_v06.c:2989/2990",
        "ERR:legacy/zstd_v06.c:3546/3553,ERR:legacy/zstd_v07.c:3219",
    ]);
    const CAP: usize = 262_144;
    let data = corpus(Corpus::Text, 512, 0x4B);
    for ver in 1u8..=7 {
        let hdr = legacy_header(ver);
        let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
        // bt_raw declaring more bytes than the frame contains
        for declared in [data.len() + 1, data.len() + 1000, 0x7_FFFF] {
            let mut f = hdr.clone();
            f.extend_from_slice(&legacy_block_header(1, declared));
            f.extend_from_slice(&data);
            f.extend_from_slice(&legacy_block_header(3, 0));
            cases.push((format!("raw-oversized-{declared}"), f));
        }
        // bt_rle
        for n in [1usize, 300, 70_000] {
            let mut f = hdr.clone();
            f.extend_from_slice(&legacy_block_header(2, n));
            f.push(0x5A);
            f.extend_from_slice(&legacy_block_header(3, 0));
            cases.push((format!("rle-{n}"), f));
        }
        // trailing bytes after bt_end
        for extra in [1usize, 3, 17] {
            let mut f = hdr.clone();
            f.extend_from_slice(&legacy_block_header(1, 16));
            f.extend_from_slice(&data[..16]);
            f.extend_from_slice(&legacy_block_header(3, 0));
            f.extend(std::iter::repeat(0x00u8).take(extra));
            cases.push((format!("trailing-{extra}"), f));
        }
        // a truncated final block header
        for keep in [0usize, 1, 2] {
            let mut f = hdr.clone();
            f.extend_from_slice(&legacy_block_header(1, 16));
            f.extend_from_slice(&data[..16]);
            f.extend_from_slice(&legacy_block_header(3, 0)[..keep]);
            cases.push((format!("short-bh-{keep}"), f));
        }
        // a valid frame, so the same sweep also has a positive control
        {
            let mut f = hdr.clone();
            f.extend_from_slice(&legacy_block_header(1, data.len()));
            f.extend_from_slice(&data);
            f.extend_from_slice(&legacy_block_header(3, 0));
            cases.push(("valid".into(), f));
        }
        let v = format!("ZSTDv0{ver}");
        for (name, frame) in &cases {
            diff_bytes(&format!("{v} block-walk {name}"), |l| {
                let f = l.sym::<FnDec4>(&format!("{v}_decompress"));
                let mut out: Vec<(usize, R)> = Vec::new();
                let mut last = Blob(Vec::new());
                // full capacity, then capacities too small for the raw block
                for &cap in &[CAP, 0usize, 1, 8, 15, 16, 511, 512] {
                    let mut dst = poison(cap.min(CAP).max(1));
                    let r = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            frame.as_ptr() as *const c_void,
                            frame.len(),
                        )
                    };
                    let rr = res(l, r);
                    if let R::Ok(n) = rr {
                        dst.truncate(n);
                    }
                    out.push((cap, rr));
                    if cap == CAP {
                        last = Blob(dst);
                    }
                }
                (out, last)
            });
        }
    }
}

/// The `bt_rle` / out-of-range `bType` arms of `ZSTDv01..v04_decompressContinue`
/// (`zstd_v01.c:2112`, `zstd_v02.c:3411`, `zstd_v03.c:3051`, `zstd_v04.c:3203`).
///
/// SAFE: `getcBlockSize` returns **1** for `bt_rle`, so the third call is made
/// with `srcSize == 1 == dctx->expected` and the `case bt_rle:` arm returns
/// `ERROR(GENERIC)` *before* reading the payload byte. The first two calls are a
/// 4/5-byte magic parse and a 3-byte block-header parse, both fully bounded.
#[test]
fn legacy_decompress_continue_rle_arm() {
    covers(&[
        "ERR:legacy/zstd_v01.c:2112/2118,ERR:legacy/zstd_v02.c:3411/3417",
        "ERR:legacy/zstd_v03.c:3051/3057,ERR:legacy/zstd_v04.c:3203/3209/3218",
        "ERR:legacy/zstd_v05.c:3593/3599/3608,ERR:legacy/zstd_v06.c:3730/3736/3745",
        "ERR:legacy/zstd_v07.c:4000/4006/4027",
    ]);
    for ver in 1u8..=7 {
        let v = format!("ZSTDv0{ver}");
        let reset = if ver >= 5 {
            format!("{v}_decompressBegin")
        } else {
            format!("{v}_resetDCtx")
        };
        for (bt, label) in [(2u8, "rle"), (1u8, "raw")] {
            diff_bytes(&format!("{v}_decompressContinue {label} arm"), |l| {
                let create = l.sym::<FnP0>(&format!("{v}_createDCtx"));
                let free = l.sym::<FnS1p>(&format!("{v}_freeDCtx"));
                let rst = l.sym::<FnS1p>(&reset);
                let next = l.sym::<FnS1p>(&format!("{v}_nextSrcSizeToDecompress"));
                let cont = l.sym::<FnDecDCtx>(&format!("{v}_decompressContinue"));
                let d = unsafe { create() };
                unsafe { rst(d) };
                // frame header, then a bt_rle/bt_raw block header, then the payload
                let hdr = legacy_header(ver);
                let mut stream = hdr.clone();
                stream.extend_from_slice(&legacy_block_header(bt, 4));
                stream.extend_from_slice(&[0xA5u8; 4]);
                stream.extend_from_slice(&legacy_block_header(3, 0));
                let mut trace: Vec<u8> = Vec::new();
                let mut pos = 0usize;
                for _ in 0..6 {
                    let want = unsafe { next(d) };
                    if want == 0 || pos + want > stream.len() {
                        rec(&mut trace, &[want, usize::MAX, 0]);
                        break;
                    }
                    let mut dst = poison(4096);
                    let r = unsafe {
                        cont(
                            d,
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            stream[pos..].as_ptr() as *const c_void,
                            want,
                        )
                    };
                    rec(&mut trace, &[want, r, 0]);
                    trace.extend_from_slice(&dst[..32]);
                    if is_error(l, r) {
                        break;
                    }
                    pos += want;
                }
                let fr = unsafe { free(d) };
                rec(&mut trace, &[fr, 0, 0]);
                Blob(trace)
            });
        }
    }
}

/// `FSEv0x_buildDTable`'s two front bound checks — the *only* part of that
/// function that is safe to reach: `maxSymbolValue > FSEv0x_MAX_SYMBOL_VALUE`
/// (255) and `tableLog > FSEv0x_MAX_TABLELOG` (12) both return before the
/// `normalizedCounter` array is touched (`zstd_v05.c:1173-1174`,
/// `zstd_v06.c:1413-1414`, `zstd_v07.c:1434-1435`).
///
/// `tableLog == 0` is *not* used: `const S16 largeLimit = (S16)(1 << (tableLog-1))`
/// is evaluated in the declaration block, before the checks, so a zero tableLog
/// is a negative shift — undefined behaviour in the reference C.
#[test]
fn legacy_fse_build_dtable_bounds() {
    covers(&[
        "ERR:legacy/zstd_v05.c:1173,ERR:legacy/zstd_v05.c:1174",
        "ERR:legacy/zstd_v06.c:1413/1414/1445,ERR:legacy/zstd_v07.c:1434/1435/1466",
    ]);
    type FnBuildDTable =
        unsafe extern "C" fn(*mut c_void, *const i16, c_uint, c_uint) -> SizeT;
    const T15: usize = (1 + (1usize << 15)) * 4;
    for v in ["FSEv05", "FSEv06", "FSEv07"] {
        diff_bytes(&format!("{v}_buildDTable bounds"), |l| {
            let c = l.sym::<FnCreateDTable>(&format!("{v}_createDTable"));
            let fr = l.sym::<FnV1p>(&format!("{v}_freeDTable"));
            let bd = l.sym::<FnBuildDTable>(&format!("{v}_buildDTable"));
            let nc = vec![0i16; 512];
            let mut rets: Vec<R> = Vec::new();
            let mut bytes: Vec<u8> = Vec::new();
            for &(msv, tl) in &[
                (256u32, 11u32),
                (257, 11),
                (300, 5),
                (0xFFFF_FFFF, 11),
                (255, 13),
                (255, 15),
                (255, 16),
                (255, 31),
                (100, 13),
            ] {
                let dt = unsafe { c(15) };
                assert!(!dt.is_null());
                unsafe { std::ptr::write_bytes(dt as *mut u8, 0x5A, T15) };
                let r = unsafe { bd(dt, nc.as_ptr(), msv, tl) };
                rets.push(res(l, r));
                bytes.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(dt as *const u8, T15)
                });
                unsafe { fr(dt) };
            }
            (rets, Blob(bytes))
        });
    }
}

/// The remaining *pre-bitstream* guards of the HUF stream decoders.
///
/// SAFE, and each is the first statement of its function:
///  * `HUFv05_decompress1X2_usingDTable`: `dstSize <= cSrcSize ->
///    dstSize_tooSmall` (`zstd_v05.c:1916`);
///  * `HUFv0x_decompress4X{2,4}_usingDTable`: `cSrcSize < 10 ->
///    corruption_detected` ("jump table + 1 byte per stream",
///    `zstd_v05.c:1950`/`2331`, `zstd_v06.c:2080`/`2455`,
///    `zstd_v07.c:1871`/`2281`);
///  * the same functions' `length4 = cSrcSize - (length1+length2+length3+6);
///    if (length4 > cSrcSize) return corruption_detected;` overflow test
///    (`zstd_v05.c:1984`/`2366`, `zstd_v06.c:2114`/`2489`,
///    `zstd_v07.c:1904`/`2314`).  Exactly ten `0xFF` bytes make
///    `length1..3 == 0xFFFF`, so the subtraction under-flows and the guard fires
///    *before* any `BITv0x_initDStream`; the only reads are the three
///    `MEM_readLE16`s at offsets 0/2/4, all inside the 10-byte buffer;
///  * v0.7's `if (dtd.tableType != 0) return ERROR(GENERIC)` /
///    `!= 1` table-type gates (`zstd_v07.c:1842`, `:1964`, `:2254`, `:2375`),
///    which read only `DTable[0]`.
#[test]
fn legacy_huf_stream_guards() {
    covers(&[
        "ERR:legacy/zstd_v05.c:1916,ERR:legacy/zstd_v05.c:1950",
        "ERR:legacy/zstd_v05.c:1984/2017-2019/2030",
        "ERR:legacy/zstd_v05.c:2331/2366/2400-2402/2413",
        "ERR:legacy/zstd_v06.c:2080/2455,ERR:legacy/zstd_v06.c:2114/2489",
        "ERR:legacy/zstd_v07.c:1871/2281,ERR:legacy/zstd_v07.c:1904/2314",
        "ERR:legacy/zstd_v07.c:1842/1964/2254/2375",
    ]);
    type FnUsingDTable =
        unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_void) -> SizeT;
    const CAP: usize = 4096;
    let ff = [0xFFu8; 16];

    // ---- v0.5 / v0.6 : X2 tables are U16*, X4 tables are U32* --------------
    for v in ["HUFv05", "HUFv06"] {
        diff_bytes(&format!("{v} 4X jump-table guards"), |l| {
            let mut rets: Vec<R> = Vec::new();
            let mut dsts: Vec<u8> = Vec::new();
            for x4 in [false, true] {
                let name = if x4 {
                    format!("{v}_decompress4X4_usingDTable")
                } else {
                    format!("{v}_decompress4X2_usingDTable")
                };
                let f = l.sym::<FnUsingDTable>(&name);
                let mut dt16 = vec![0u16; 1 + (1usize << 12)];
                dt16[0] = 12;
                let mut dt32 = vec![0u32; 1 + (1usize << 12)];
                dt32[0] = 12;
                let dtp = if x4 {
                    dt32.as_mut_ptr() as *const c_void
                } else {
                    dt16.as_mut_ptr() as *const c_void
                };
                for &cs in &[0usize, 1, 5, 9, 10] {
                    let mut dst = poison(CAP);
                    let r = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            CAP,
                            ff.as_ptr() as *const c_void,
                            cs,
                            dtp,
                        )
                    };
                    rets.push(res(l, r));
                    dsts.push(if dst.iter().all(|&b| b == 0x5A) { 1 } else { 0 });
                }
            }
            (rets, Blob(dsts))
        });
    }
    // v0.5's single-stream X2 `dstSize <= cSrcSize` gate
    diff("HUFv05_decompress1X2_usingDTable dstSize gate", |l| {
        let f = l.sym::<FnUsingDTable>("HUFv05_decompress1X2_usingDTable");
        let mut dt = vec![0u16; 1 + (1usize << 12)];
        dt[0] = 12;
        let mut rets: Vec<R> = Vec::new();
        for &(ds, cs) in &[(0usize, 0usize), (0, 1), (4, 4), (4, 5), (4, 100)] {
            let mut dst = poison(CAP);
            let r = unsafe {
                f(
                    dst.as_mut_ptr() as *mut c_void,
                    ds,
                    ff.as_ptr() as *const c_void,
                    cs,
                    dt.as_ptr() as *const c_void,
                )
            };
            rets.push(res(l, r));
            assert!(dst.iter().all(|&b| b == 0x5A));
        }
        rets
    });
    // ---- v0.7 : one `HUFv07_DTable` type, plus the tableType gates ---------
    diff_bytes("HUFv07 usingDTable guards", |l| {
        let mut rets: Vec<R> = Vec::new();
        let mut dsts: Vec<u8> = Vec::new();
        // `(name, wants_tableType, is_4x)`.  For the single-stream (`1X`) forms
        // only the *mismatching* table type is exercised: with a matching type
        // they hand `cSrcSize` straight to `HUFv07_decompress1X{2,4}_usingDTable_internal`,
        // which has no minimum-size guard at all and walks the bitstream — the
        // reference C SIGSEGVs on 10 bytes of 0xFF, so there is no behaviour to
        // compare.  The `4X` forms are safe for both types because their
        // `_internal` starts with `cSrcSize < 10 -> corruption_detected`.
        for (name, is_4x) in [
            ("HUFv07_decompress1X2_usingDTable", false),
            ("HUFv07_decompress1X4_usingDTable", false),
            ("HUFv07_decompress4X2_usingDTable", true),
            ("HUFv07_decompress4X4_usingDTable", true),
        ] {
            let f = l.sym::<FnUsingDTable>(name);
            let wants: u32 = if name.contains("X4") { 1 } else { 0 };
            let types: &[u32] = if is_4x { &[0, 1] } else { &[1 - wants] };
            for &table_type in types {
                let mut dt = vec![0u32; 1 + (1usize << 12)];
                // DTableDesc { maxTableLog, tableType, tableLog, reserved }
                dt[0] = 12 * 0x0100_0001 | (table_type << 8);
                for &cs in &[0usize, 1, 9, 10] {
                    let mut dst = poison(CAP);
                    let r = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            CAP,
                            ff.as_ptr() as *const c_void,
                            cs,
                            dt.as_ptr() as *const c_void,
                        )
                    };
                    rets.push(res(l, r));
                    dsts.push(if dst.iter().all(|&b| b == 0x5A) { 1 } else { 0 });
                }
            }
        }
        (rets, Blob(dsts))
    });
}

/// `ZSTDv07_getFrameParams`'s window-descriptor arithmetic:
/// `windowLog = (wlByte >> 3) + ZSTDv07_WINDOWLOG_ABSOLUTEMIN (10)`,
/// `if (windowLog > ZSTDv07_WINDOWLOG_MAX) return frameParameter_unsupported`
/// (`zstd_v07.c:3108`), then
/// `windowSize = (1 << windowLog); windowSize += (windowSize >> 3) * (wlByte & 7);`
/// and `if (windowSize > windowSizeMax) return frameParameter_unsupported`
/// (`zstd_v07.c:3113`).  `WINDOWLOG_MAX` is 27 on a 64-bit build, so
/// `wlByte >= 136` starts to overshoot.  Pure arithmetic over two header bytes.
#[test]
fn legacy_v07_window_descriptor() {
    covers(&["ERR:legacy/zstd_v07.c:3108,ERR:legacy/zstd_v07.c:3113"]);
    diff_bytes("ZSTDv07_getFrameParams wlByte sweep", |l| {
        let f = l.sym::<FnGfp>("ZSTDv07_getFrameParams");
        let mut rets: Vec<u8> = Vec::new();
        let mut params: Vec<u8> = Vec::new();
        for fhd in [0x00u8, 0x01, 0x02, 0x03, 0x04, 0x40, 0x80, 0xC0] {
            for wl in 0..=255u8 {
                let mut src = magic_buf(MAGIC_V07, 24);
                src[4] = fhd;
                src[5] = wl;
                let mut p = poison(64);
                let r = unsafe {
                    f(p.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void, 24)
                };
                rets.extend_from_slice(&(r as u64).to_le_bytes());
                params.append(&mut p);
            }
        }
        (Blob(rets), Blob(params))
    });
}

/// `ZSTDv07_decompressContinue` driven over a *valid* v0.7 frame, including the
/// checksum arm `if (check32 != h32) return ERROR(checksum_wrong);`
/// (`zstd_v07.c:3978`) — which the one-shot `ZSTDv07_decompressFrame` path never
/// reaches, because it only `XXH64_update`s and breaks at `bt_end`.
///
/// SAFE: every block is `bt_raw` or `bt_rle`, so `ZSTDv07_decompressContinue`'s
/// `ZSTDds_decompressBlock` stage takes the `copyRawBlock` / `generateNxBytes`
/// arm, both of which are bounds-checked `memcpy`/`memset`; the Huffman/FSE
/// decoders are never entered.
#[test]
fn legacy_v07_decompress_continue_valid() {
    covers(&["ERR:legacy/zstd_v07.c:3978"]);
    let d1 = corpus(Corpus::Text, 300, 0x3978);
    for checksum in [false, true] {
        for blocks in [
            vec![(1u8, d1.clone(), 0usize)],
            vec![(2u8, vec![0x5Au8], 100usize)],
            vec![(1u8, d1.clone(), 0), (2u8, vec![0x00u8], 50), (1u8, d1[..7].to_vec(), 0)],
            vec![],
        ] {
            let (mut frame, plain) = legacy_raw_frame(7, &blocks);
            if checksum {
                frame[4] |= 0x04; // fhdByte bit 2 == checksumFlag
            }
            diff_bytes(
                &format!("ZSTDv07_decompressContinue valid cs={checksum} nb={}", blocks.len()),
                |l| {
                    let create = l.sym::<FnP0>("ZSTDv07_createDCtx");
                    let free = l.sym::<FnS1p>("ZSTDv07_freeDCtx");
                    let begin = l.sym::<FnS1p>("ZSTDv07_decompressBegin");
                    let next = l.sym::<FnS1p>("ZSTDv07_nextSrcSizeToDecompress");
                    let skip = l.sym::<FnI1p>("ZSTDv07_isSkipFrame");
                    let cont = l.sym::<FnDecDCtx>("ZSTDv07_decompressContinue");
                    let d = unsafe { create() };
                    unsafe { begin(d) };
                    let mut trace: Vec<u8> = Vec::new();
                    let mut out: Vec<u8> = Vec::new();
                    let mut pos = 0usize;
                    for _ in 0..32 {
                        let want = unsafe { next(d) };
                        rec(&mut trace, &[want, unsafe { skip(d) } as usize, pos]);
                        if want == 0 || pos + want > frame.len() {
                            break;
                        }
                        let mut dst = poison(262_144);
                        let r = unsafe {
                            cont(
                                d,
                                dst.as_mut_ptr() as *mut c_void,
                                dst.len(),
                                frame[pos..].as_ptr() as *const c_void,
                                want,
                            )
                        };
                        rec(&mut trace, &[r, 0, 0]);
                        if is_error(l, r) {
                            break;
                        }
                        out.extend_from_slice(&dst[..r]);
                        pos += want;
                    }
                    let fr = unsafe { free(d) };
                    rec(&mut trace, &[fr, 0, 0]);
                    trace.extend_from_slice(&[0xFFu8; 8]);
                    trace.extend_from_slice(&out);
                    (plain.len(), Blob(trace))
                },
            );
        }
    }
}

/// `FSEv0x_decompress_usingDTable`'s output-full guard
/// `if (op > omax-2) return ERROR(dstSize_tooSmall);` (`zstd_v05.c:1434/1436`,
/// `zstd_v06.c:1557/1566`, `zstd_v07.c:1578/1587`).
///
/// SAFE with a *valid* DTable and a tiny `originalSize`: the table comes from
/// `FSEv0x_buildDTable_raw` (a complete, self-consistent 1-bit table), and with
/// `maxDstSize <= 3` the `olimit = omax-3` main loop is skipped entirely and the
/// tail loop's very first statement returns `dstSize_tooSmall` — so no
/// `FSEv0x_GETSYMBOL` ever runs. The only bitstream contact is
/// `BITv0x_initDStream` + `FSEv0x_initDState`, whose byte-wise fill is bounded by
/// `cSrcSize` (and `cSrcSize == 0` is rejected outright). Larger `maxDstSize`
/// would enter the symbol loops and is excluded.
#[test]
fn legacy_fse_decompress_using_dtable_dst_full() {
    covers(&[
        "ERR:legacy/zstd_v05.c:1434/1436,ERR:legacy/zstd_v06.c:1557/1566",
        "ERR:legacy/zstd_v07.c:1578/1587",
    ]);
    type FnFseUsingDTable =
        unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_void) -> SizeT;
    let cs = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    for v in ["FSEv05", "FSEv06", "FSEv07"] {
        diff_bytes(&format!("{v}_decompress_usingDTable dst-full"), |l| {
            let c = l.sym::<FnCreateDTable>(&format!("{v}_createDTable"));
            let fr = l.sym::<FnV1p>(&format!("{v}_freeDTable"));
            let raw = l.sym::<FnBuildRaw>(&format!("{v}_buildDTable_raw"));
            let f = l.sym::<FnFseUsingDTable>(&format!("{v}_decompress_usingDTable"));
            let mut rets: Vec<R> = Vec::new();
            let mut dsts: Vec<u8> = Vec::new();
            for &nb in &[1u32, 2, 4] {
                let dt = unsafe { c(15) };
                let br = unsafe { raw(dt, nb) };
                assert!(!is_error(l, br));
                for &orig in &[0usize, 1, 2, 3] {
                    for &csn in &[0usize, 1, 2, 8] {
                        let mut dst = poison(64);
                        let r = unsafe {
                            f(
                                dst.as_mut_ptr() as *mut c_void,
                                orig,
                                cs.as_ptr() as *const c_void,
                                csn,
                                dt as *const c_void,
                            )
                        };
                        rets.push(res(l, r));
                        dsts.append(&mut dst);
                    }
                }
                unsafe { fr(dt) };
            }
            (rets, Blob(dsts))
        });
    }
}
