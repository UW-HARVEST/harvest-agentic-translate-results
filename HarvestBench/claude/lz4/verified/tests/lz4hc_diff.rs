//! Differential tests for the `lz4hc.c` high-compression API.
//!
//! Every call is dispatched through BOTH shared libraries' export tables
//! (`c_src/build/liblz4.so` and `target/release/liblz4.so`) via `libloading`,
//! so the `#[no_mangle]` Rust wrappers are exercised exactly as an external C
//! caller would. Rust functions are never called directly.
//!
//! The dominating axis is the compression level: `lz4hc.c` dispatches to three
//! completely different parsers,
//!   * `lz4mid`  for levels 1..2                    (`LZ4MID_compress`)
//!   * `lz4hc`   for levels 3..9                    (`LZ4HC_compress_hashChain`)
//!   * `lz4opt`  for levels >= LZ4HC_CLEVEL_OPT_MIN (`LZ4HC_compress_optimal`)
//! and all three are swept over every input shape / size. Levels < 1 are
//! remapped to `LZ4HC_CLEVEL_DEFAULT` and levels > 12 clamped to
//! `LZ4HC_CLEVEL_MAX` by `LZ4HC_getCLevelParams` / `LZ4_setCompressionLevel`.
//!
//! Buffer discipline: the C destination buffer and the Rust destination buffer
//! are always pre-filled with the SAME 0xAA sentinel and the FULL buffer is
//! compared, so untouched tail bytes cannot produce a false positive and a
//! write past the reported compressed length is detected.
//!
//! Note: the C library in `c_src/build` is compiled WITHOUT `-DNDEBUG`, so its
//! `assert()`s are live. Cases where an assert would fire for an out-of-contract
//! argument combination are called out where they are avoided.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ===========================================================================
// Signature aliases (taken verbatim from lz4hc.h / lz4hc.c)
// ===========================================================================

type FnSizeof = unsafe extern "C" fn() -> c_int;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;

/// `int LZ4_compress_HC(const char*, char*, int, int, int)`
type FnCompressHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_extStateHC(void*, const char*, char*, int, int, int)`
type FnExtState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_destSize(void*, const char*, char*, int*, int, int)`
type FnDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;

/// `LZ4_streamHC_t* LZ4_createStreamHC(void)`
type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
/// `int LZ4_freeStreamHC(LZ4_streamHC_t*)`
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
/// `LZ4_streamHC_t* LZ4_initStreamHC(void*, size_t)`
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
/// `void LZ4_resetStreamHC(LZ4_streamHC_t*, int)`, `LZ4_resetStreamHC_fast`,
/// `LZ4_setCompressionLevel`, `LZ4_favorDecompressionSpeed`
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
/// `int LZ4_loadDictHC(LZ4_streamHC_t*, const char*, int)`
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
/// `int LZ4_compress_HC_continue(LZ4_streamHC_t*, const char*, char*, int, int)`
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_continue_destSize(LZ4_streamHC_t*, const char*, char*, int*, int)`
type FnContinueDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
/// `int LZ4_saveDictHC(LZ4_streamHC_t*, char*, int)`
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
/// `void LZ4_attach_HC_dictionary(LZ4_streamHC_t*, const LZ4_streamHC_t*)`
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);

// deprecated / legacy
/// `int LZ4_compressHC(const char*, char*, int)`
type FnDep3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `int LZ4_compressHC_limitedOutput(const char*, char*, int, int)` /
/// `int LZ4_compressHC2(const char*, char*, int, int)`
type FnDep4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compressHC2_limitedOutput(const char*, char*, int, int, int)`
type FnDep5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compressHC_withStateHC(void*, const char*, char*, int)` /
/// `int LZ4_compressHC_continue(LZ4_streamHC_t*, const char*, char*, int)`
type FnDepS4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `int LZ4_compressHC_limitedOutput_withStateHC(void*, const char*, char*, int, int)` /
/// `int LZ4_compressHC2_withStateHC(void*, const char*, char*, int, int)` /
/// `int LZ4_compressHC_limitedOutput_continue(...)` /
/// `int LZ4_compressHC2_continue(void*, const char*, char*, int, int)`
type FnDepS5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compressHC2_limitedOutput_withStateHC(void*, const char*, char*, int, int, int)` /
/// `int LZ4_compressHC2_limitedOutput_continue(void*, const char*, char*, int, int, int)`
type FnDepS6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `void* LZ4_createHC(const char*)`
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
/// `int LZ4_freeHC(void*)`
type FnFreeHC = unsafe extern "C" fn(*mut c_void) -> c_int;
/// `char* LZ4_slideInputBufferHC(void*)`
type FnSlide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
/// `int LZ4_resetStreamStateHC(void*, char*)`
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

// decompression (round-trip validation)
type FnDecompSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;

/// `LZ4HC_match_t` — `{ int off; int len; int back; }` (lz4hc.c:357).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct LZ4HC_match_t {
    off: c_int,
    len: c_int,
    back: c_int,
}

/// ```c
/// LZ4HC_match_t LZ4HC_searchExtDict(const BYTE* ip, U32 ipIndex,
///         const BYTE* iLowLimit, const BYTE* iHighLimit,
///         const LZ4HC_CCtx_internal* dictCtx, U32 gDictEndIndex,
///         int currentBestML, int nbAttempts)
/// ```
type FnSearchExtDict = unsafe extern "C" fn(
    *const u8,
    u32,
    *const u8,
    *const u8,
    *const c_void,
    u32,
    c_int,
    c_int,
) -> LZ4HC_match_t;

// ===========================================================================
// Resolved-symbol table (one dlsym pass, reused by every test)
// ===========================================================================

struct Api {
    bound: (FnBound, FnBound),
    sizeof_state: (FnSizeof, FnSizeof),
    sizeof_stream_state: (FnSizeof, FnSizeof),
    compress_hc: (FnCompressHC, FnCompressHC),
    ext_state: (FnExtState, FnExtState),
    ext_state_fast: (FnExtState, FnExtState),
    dest_size: (FnDestSize, FnDestSize),
    create_stream: (FnCreateStream, FnCreateStream),
    free_stream: (FnFreeStream, FnFreeStream),
    init_stream: (FnInitStream, FnInitStream),
    reset_stream: (FnStreamInt, FnStreamInt),
    reset_stream_fast: (FnStreamInt, FnStreamInt),
    set_level: (FnStreamInt, FnStreamInt),
    favor: (FnStreamInt, FnStreamInt),
    load_dict: (FnLoadDict, FnLoadDict),
    cont: (FnContinue, FnContinue),
    cont_dest_size: (FnContinueDestSize, FnContinueDestSize),
    save_dict: (FnSaveDict, FnSaveDict),
    attach: (FnAttach, FnAttach),
    search_ext_dict: (FnSearchExtDict, FnSearchExtDict),
    d_hc: (FnDep3, FnDep3),
    d_hc_lim: (FnDep4, FnDep4),
    d_hc2: (FnDep4, FnDep4),
    d_hc2_lim: (FnDep5, FnDep5),
    d_hc_st: (FnDepS4, FnDepS4),
    d_hc_lim_st: (FnDepS5, FnDepS5),
    d_hc2_st: (FnDepS5, FnDepS5),
    d_hc2_lim_st: (FnDepS6, FnDepS6),
    d_hc_cont: (FnDepS4, FnDepS4),
    d_hc_lim_cont: (FnDepS5, FnDepS5),
    d_hc2_cont: (FnDepS5, FnDepS5),
    d_hc2_lim_cont: (FnDepS6, FnDepS6),
    d_create_hc: (FnCreateHC, FnCreateHC),
    d_free_hc: (FnFreeHC, FnFreeHC),
    d_slide: (FnSlide, FnSlide),
    d_reset_stream_state: (FnResetStreamState, FnResetStreamState),
    decomp: (FnDecompSafe, FnDecompSafe),
    decomp_dict: (FnDecompDict, FnDecompDict),
}

fn api() -> &'static Api {
    static A: std::sync::OnceLock<Api> = std::sync::OnceLock::new();
    A.get_or_init(|| Api {
        bound: both("LZ4_compressBound"),
        sizeof_state: both("LZ4_sizeofStateHC"),
        sizeof_stream_state: both("LZ4_sizeofStreamStateHC"),
        compress_hc: both("LZ4_compress_HC"),
        ext_state: both("LZ4_compress_HC_extStateHC"),
        ext_state_fast: both("LZ4_compress_HC_extStateHC_fastReset"),
        dest_size: both("LZ4_compress_HC_destSize"),
        create_stream: both("LZ4_createStreamHC"),
        free_stream: both("LZ4_freeStreamHC"),
        init_stream: both("LZ4_initStreamHC"),
        reset_stream: both("LZ4_resetStreamHC"),
        reset_stream_fast: both("LZ4_resetStreamHC_fast"),
        set_level: both("LZ4_setCompressionLevel"),
        favor: both("LZ4_favorDecompressionSpeed"),
        load_dict: both("LZ4_loadDictHC"),
        cont: both("LZ4_compress_HC_continue"),
        cont_dest_size: both("LZ4_compress_HC_continue_destSize"),
        save_dict: both("LZ4_saveDictHC"),
        attach: both("LZ4_attach_HC_dictionary"),
        search_ext_dict: both("LZ4HC_searchExtDict"),
        d_hc: both("LZ4_compressHC"),
        d_hc_lim: both("LZ4_compressHC_limitedOutput"),
        d_hc2: both("LZ4_compressHC2"),
        d_hc2_lim: both("LZ4_compressHC2_limitedOutput"),
        d_hc_st: both("LZ4_compressHC_withStateHC"),
        d_hc_lim_st: both("LZ4_compressHC_limitedOutput_withStateHC"),
        d_hc2_st: both("LZ4_compressHC2_withStateHC"),
        d_hc2_lim_st: both("LZ4_compressHC2_limitedOutput_withStateHC"),
        d_hc_cont: both("LZ4_compressHC_continue"),
        d_hc_lim_cont: both("LZ4_compressHC_limitedOutput_continue"),
        d_hc2_cont: both("LZ4_compressHC2_continue"),
        d_hc2_lim_cont: both("LZ4_compressHC2_limitedOutput_continue"),
        d_create_hc: both("LZ4_createHC"),
        d_free_hc: both("LZ4_freeHC"),
        d_slide: both("LZ4_slideInputBufferHC"),
        d_reset_stream_state: both("LZ4_resetStreamStateHC"),
        decomp: both("LZ4_decompress_safe"),
        decomp_dict: both("LZ4_decompress_safe_usingDict"),
    })
}

// ===========================================================================
// LZ4HC_CCtx_internal mirror — used to compare stream/state contents
// ===========================================================================

const HC_HASHTABLESIZE: usize = 1 << 15; // LZ4HC_HASH_LOG == 15
const HC_MAXD: usize = 1 << 16; // LZ4HC_DICTIONARY_LOGSIZE == 16
/// hashTable + chainTable = 32768*4 + 65536*2
const HC_TABLES_BYTES: usize = HC_HASHTABLESIZE * 4 + HC_MAXD * 2;
/// `sizeof(LZ4HC_CCtx_internal)` on LP64.
const HC_CCTX_BYTES: usize = 262192;
/// `LZ4_STREAMHC_MINSIZE` == `sizeof(LZ4_streamHC_t)`.
const HC_STREAM_BYTES: usize = 262200;

#[repr(C)]
struct HcCtx {
    hash_table: [u32; HC_HASHTABLESIZE],
    chain_table: [u16; HC_MAXD],
    end: *const u8,
    prefix_start: *const u8,
    dict_start: *const u8,
    dict_limit: u32,
    low_limit: u32,
    next_to_update: u32,
    compression_level: i16,
    favor_dec_speed: i8,
    dirty: i8,
    dict_ctx: *const HcCtx,
}

/// Everything in `LZ4HC_CCtx_internal` except the two big tables, with absolute
/// pointers reduced to offsets relative to `prefixStart`, so that two contexts
/// referencing *different but identically laid out* buffers compare equal.
#[derive(Debug, PartialEq, Eq)]
struct HcScalars {
    end_off: isize,
    dict_start_off: isize,
    prefix_null: bool,
    end_null: bool,
    dict_start_null: bool,
    dict_limit: u32,
    low_limit: u32,
    next_to_update: u32,
    compression_level: i16,
    favor_dec_speed: i8,
    dirty: i8,
    dict_ctx_null: bool,
}

unsafe fn hc_scalars(p: *const c_void) -> HcScalars {
    let c = p as *const HcCtx;
    let prefix = (*c).prefix_start;
    let end = (*c).end;
    let ds = (*c).dict_start;
    HcScalars {
        end_off: (end as isize) - (prefix as isize),
        dict_start_off: (ds as isize) - (prefix as isize),
        prefix_null: prefix.is_null(),
        end_null: end.is_null(),
        dict_start_null: ds.is_null(),
        dict_limit: (*c).dict_limit,
        low_limit: (*c).low_limit,
        next_to_update: (*c).next_to_update,
        compression_level: (*c).compression_level,
        favor_dec_speed: (*c).favor_dec_speed,
        dirty: (*c).dirty,
        dict_ctx_null: (*c).dict_ctx.is_null(),
    }
}

/// Structural comparison: raw tables byte-for-byte plus pointer-relative scalars.
unsafe fn assert_state_eq(label: &str, cp: *const c_void, rp: *const c_void) {
    let ct = std::slice::from_raw_parts(cp as *const u8, HC_TABLES_BYTES);
    let rt = std::slice::from_raw_parts(rp as *const u8, HC_TABLES_BYTES);
    assert_bytes_eq(&format!("{}: hashTable+chainTable", label), ct, rt);
    assert_eq!(
        hc_scalars(cp),
        hc_scalars(rp),
        "{}: LZ4HC_CCtx_internal scalar fields",
        label
    );
}

/// Strict byte comparison of the whole `LZ4HC_CCtx_internal` blob. Valid
/// whenever both contexts were handed the *same* source/dict pointers (the
/// normal case here, because input buffers are shared between the two
/// libraries) — then even the raw pointer fields must be identical.
unsafe fn assert_state_blob_eq(label: &str, cp: *const c_void, rp: *const c_void) {
    let ct = std::slice::from_raw_parts(cp as *const u8, HC_CCTX_BYTES);
    let rt = std::slice::from_raw_parts(rp as *const u8, HC_CCTX_BYTES);
    if ct != rt {
        // Produce the most specific message available.
        assert_eq!(hc_scalars(cp), hc_scalars(rp), "{}: state scalars", label);
        assert_bytes_eq(&format!("{}: raw state blob", label), ct, rt);
    }
}

// ===========================================================================
// Axes
// ===========================================================================

/// Every compression level worth probing: below `LZ4HC_CLEVEL_MIN`, the whole
/// documented range, and above `LZ4HC_CLEVEL_MAX` (clamped by the C).
fn all_levels() -> Vec<c_int> {
    vec![
        c_int::MIN,
        -1,
        0,
        1,
        2, // LZ4HC_CLEVEL_MIN
        3,
        4,
        5,
        6,
        7,
        8,
        9,  // LZ4HC_CLEVEL_DEFAULT
        10, // LZ4HC_CLEVEL_OPT_MIN
        11,
        12, // LZ4HC_CLEVEL_MAX
        13,
        100,
        c_int::MAX,
    ]
}

/// Input sizes straddling every threshold lz4 / lz4hc branch on.
fn hc_sizes() -> Vec<usize> {
    vec![
        0,
        1,
        4,  // MINMATCH
        5,  // LASTLITERALS
        12, // MFLIMIT
        13,
        16,
        63,
        64,
        65,
        255,
        256,
        1000,
        4096,
        65535, // LZ4_DISTANCE_MAX
        65536,
        65546, // LZ4_64Klimit - 1
        65547, // LZ4_64Klimit
        65548,
        100000,
        200000,
    ]
}

/// Shaped source data with guaranteed spare capacity so that `as_ptr()` is
/// always a real allocation (even for len 0) and any speculative over-read by
/// the compressor stays inside owned memory.
fn src_buf(rng: &mut Rng, shape: usize, len: usize) -> Vec<u8> {
    let mut v = gen_shape(rng, shape, len);
    v.reserve(64);
    v
}

// ===========================================================================
// Shared helpers
// ===========================================================================

fn bound_of(len: usize) -> usize {
    let a = api();
    unsafe { (a.bound.0)(len as c_int) }.max(1) as usize
}

/// Round-trip: HC output must be valid LZ4 block data for BOTH decompressors.
fn assert_roundtrip(label: &str, comp: &[u8], orig: &[u8]) {
    let a = api();
    for (f, tag) in [(a.decomp.0, "C"), (a.decomp.1, "Rust")] {
        let mut out = vec![0xAAu8; orig.len() + 16];
        let n = unsafe {
            f(
                comp.as_ptr() as *const c_char,
                out.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                orig.len() as c_int,
            )
        };
        assert_eq!(
            n,
            orig.len() as c_int,
            "{}: {} LZ4_decompress_safe returned {} (want {})",
            label,
            tag,
            n,
            orig.len()
        );
        assert_bytes_eq(
            &format!("{}: {} round-trip payload", label, tag),
            orig,
            &out[..orig.len()],
        );
    }
}

/// Round-trip of a streamed block against its (<= 64 KB) history.
fn assert_roundtrip_dict(label: &str, comp: &[u8], orig: &[u8], hist: &[u8]) {
    let a = api();
    for (f, tag) in [(a.decomp_dict.0, "C"), (a.decomp_dict.1, "Rust")] {
        let mut out = vec![0xAAu8; orig.len() + 16];
        let n = unsafe {
            f(
                comp.as_ptr() as *const c_char,
                out.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                orig.len() as c_int,
                hist.as_ptr() as *const c_char,
                hist.len() as c_int,
            )
        };
        assert_eq!(
            n,
            orig.len() as c_int,
            "{}: {} LZ4_decompress_safe_usingDict returned {} (want {})",
            label,
            tag,
            n,
            orig.len()
        );
        assert_bytes_eq(
            &format!("{}: {} streamed round-trip payload", label, tag),
            orig,
            &out[..orig.len()],
        );
    }
}

/// Keep only the trailing `LZ4_DISTANCE_MAX + 1` bytes — everything the encoder
/// could still reference.
fn trim_hist(hist: &mut Vec<u8>) {
    if hist.len() > 65536 {
        let cut = hist.len() - 65536;
        hist.drain(..cut);
    }
}

/// `LZ4_compress_HC` on both libraries; asserts identical return value and
/// identical FULL destination buffer, then round-trips the result.
fn diff_compress_hc(src: &[u8], level: c_int, label: &str) -> Vec<u8> {
    let a = api();
    let bound = bound_of(src.len());
    let mut cdst = vec![0xAAu8; bound + 32];
    let mut rdst = vec![0xAAu8; bound + 32];
    let cn = unsafe {
        (a.compress_hc.0)(
            src.as_ptr() as *const c_char,
            cdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            level,
        )
    };
    let rn = unsafe {
        (a.compress_hc.1)(
            src.as_ptr() as *const c_char,
            rdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            level,
        )
    };
    assert_eq!(cn, rn, "{}: LZ4_compress_HC return value", label);
    assert!(cn > 0, "{}: LZ4_compress_HC unexpectedly failed", label);
    assert_bytes_eq(&format!("{}: LZ4_compress_HC dst", label), &cdst, &rdst);
    cdst.truncate(cn as usize);
    assert_roundtrip(label, &cdst, src);
    cdst
}

/// Full `(shape x size)` sweep of `LZ4_compress_HC` for the given levels.
fn sweep_levels(levels: &[c_int], seed: u64) {
    for &level in levels {
        let mut rng = Rng::new(seed ^ (level as u32 as u64).wrapping_mul(0x9E37_79B9));
        for shape in 0..N_SHAPES {
            for &len in &hc_sizes() {
                let src = src_buf(&mut rng, shape, len);
                diff_compress_hc(
                    &src,
                    level,
                    &format!("HC level={} shape={} len={}", level, shape_name(shape), len),
                );
            }
        }
    }
}

// ===========================================================================
// Metadata
// ===========================================================================

#[test]
fn hc_sizeof_state_and_stream_state() {
    let a = api();
    unsafe {
        let cs = (a.sizeof_state.0)();
        let rs = (a.sizeof_state.1)();
        assert_eq!(cs, rs, "LZ4_sizeofStateHC");
        assert_eq!(
            cs as usize, HC_STREAM_BYTES,
            "LZ4_sizeofStateHC must be LZ4_STREAMHC_MINSIZE"
        );

        let css = (a.sizeof_stream_state.0)();
        let rss = (a.sizeof_stream_state.1)();
        assert_eq!(css, rss, "LZ4_sizeofStreamStateHC");
        assert_eq!(css, cs, "LZ4_sizeofStreamStateHC == LZ4_sizeofStateHC");
    }
    // The local mirror of LZ4HC_CCtx_internal must match the C layout,
    // otherwise every state comparison below would be meaningless.
    assert_eq!(
        std::mem::size_of::<HcCtx>(),
        HC_CCTX_BYTES,
        "HcCtx mirror layout"
    );
    assert_eq!(std::mem::align_of::<HcCtx>(), 8);
}

// ===========================================================================
// LZ4_compress_HC : the compression-level axis (split for test parallelism)
// ===========================================================================

/// Levels < 1 are remapped to `LZ4HC_CLEVEL_DEFAULT`; levels 1..2 select the
/// `lz4mid` strategy.
#[test]
fn hc_levels_belowmin_and_lz4mid() {
    sweep_levels(&[c_int::MIN, -1, 0, 1, 2], 0xA1);
}

/// `LZ4HC_compress_hashChain`, low search budgets.
#[test]
fn hc_levels_hashchain_low() {
    sweep_levels(&[3, 4, 5, 6], 0xA2);
}

/// `LZ4HC_compress_hashChain`, high search budgets (incl. the default, 9).
#[test]
fn hc_levels_hashchain_high() {
    sweep_levels(&[7, 8, 9], 0xA3);
}

/// `LZ4HC_compress_optimal` below "ultra" mode.
#[test]
fn hc_levels_optimal() {
    sweep_levels(&[10, 11], 0xA4);
}

/// `LZ4HC_compress_optimal` in "ultra" mode (`cLevel >= LZ4HC_CLEVEL_MAX`),
/// including out-of-range levels that the C clamps down to 12.
#[test]
fn hc_levels_optimal_ultra_and_above_max() {
    sweep_levels(&[12, 13, 100, c_int::MAX], 0xA5);
}

// ===========================================================================
// extStateHC / extStateHC_fastReset
// ===========================================================================

#[test]
fn hc_extstatehc_and_fastreset_state_reuse() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;

    // The SAME state block is reused across successive calls, which is exactly
    // what distinguishes the fastReset variant from the full-reset variant.
    let mut c_full = AlignedBuf::new(ssz, 64);
    let mut r_full = AlignedBuf::new(ssz, 64);
    let mut c_fast = AlignedBuf::new(ssz, 64);
    let mut r_fast = AlignedBuf::new(ssz, 64);

    // fastReset presumes an already-initialised state.
    unsafe {
        (a.init_stream.0)(c_fast.as_mut_ptr() as *mut c_void, ssz);
        (a.init_stream.1)(r_fast.as_mut_ptr() as *mut c_void, ssz);
    }

    let mut rng = Rng::new(0xE5_7A7E);
    let levels = [c_int::MIN, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13, 100, c_int::MAX];
    for round in 0..3 {
        for shape in 0..N_SHAPES {
            for &len in &[0usize, 1, 13, 64, 1000, 4096, 65546, 65547, 100000] {
                let src = src_buf(&mut rng, shape, len);
                let bound = bound_of(len);
                for &level in &levels {
                    let label = format!(
                        "extStateHC round={} shape={} len={} level={}",
                        round,
                        shape_name(shape),
                        len,
                        level
                    );
                    // ---- full-reset variant ----
                    let mut cdst = vec![0xAAu8; bound + 32];
                    let mut rdst = vec![0xAAu8; bound + 32];
                    let cn = unsafe {
                        (a.ext_state.0)(
                            c_full.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.ext_state.1)(
                            r_full.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: unexpected failure", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cdst, &rdst);
                    unsafe {
                        assert_state_blob_eq(
                            &label,
                            c_full.as_ptr() as *const c_void,
                            r_full.as_ptr() as *const c_void,
                        )
                    };
                    assert_roundtrip(&label, &cdst[..cn as usize], &src);

                    // ---- fastReset variant, same state carried forward ----
                    let flabel = format!("{} [fastReset]", label);
                    let mut cdst = vec![0xAAu8; bound + 32];
                    let mut rdst = vec![0xAAu8; bound + 32];
                    let cn = unsafe {
                        (a.ext_state_fast.0)(
                            c_fast.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.ext_state_fast.1)(
                            r_fast.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", flabel);
                    assert_bytes_eq(&format!("{}: dst", flabel), &cdst, &rdst);
                    unsafe {
                        assert_state_blob_eq(
                            &flabel,
                            c_fast.as_ptr() as *const c_void,
                            r_fast.as_ptr() as *const c_void,
                        )
                    };
                    if cn > 0 {
                        assert_roundtrip(&flabel, &cdst[..cn as usize], &src);
                    }
                }
            }
        }
    }
}

/// `LZ4_compress_HC_extStateHC` funnels through `LZ4_initStreamHC`, and
/// `LZ4_compress_HC_extStateHC_fastReset` does its own `LZ4_isAligned` check:
/// both must return 0 for a misaligned state block.
#[test]
fn hc_extstatehc_misaligned_state_returns_zero() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut rng = Rng::new(0x8115);
    let src = src_buf(&mut rng, 4, 5000);
    let bound = bound_of(src.len());

    for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
        let mut cst = AlignedBuf::with_offset(ssz, 8, off);
        let mut rst = AlignedBuf::with_offset(ssz, 8, off);
        for &level in &[2i32, 9, 12] {
            let mut cdst = vec![0xAAu8; bound + 32];
            let mut rdst = vec![0xAAu8; bound + 32];
            let cn = unsafe {
                (a.ext_state.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.ext_state.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "extStateHC misaligned(+{}) level={}", off, level);
            assert_eq!(cn, 0, "extStateHC misaligned(+{}) must fail", off);
            assert_bytes_eq("extStateHC misaligned dst untouched", &cdst, &rdst);

            let cn = unsafe {
                (a.ext_state_fast.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.ext_state_fast.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "fastReset misaligned(+{}) level={}", off, level);
            assert_eq!(cn, 0, "fastReset misaligned(+{}) must fail", off);
        }
    }
}

// ===========================================================================
// LZ4_compress_HC_destSize
// ===========================================================================

#[test]
fn hc_compress_hc_destsize_target_sweep() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut cst = AlignedBuf::new(ssz, 64);
    let mut rst = AlignedBuf::new(ssz, 64);
    let mut rng = Rng::new(0xD357_5123);

    let levels = [c_int::MIN, -1, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13, 100, c_int::MAX];
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 5, 13, 64, 300, 4096, 65547, 100000] {
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            let mut targets: Vec<usize> = vec![
                0, 1, 2, 3, 4, 5, 6, 8, 11, 12, 16, 20, 32, 64, 128, 256, 1024, 4096,
            ];
            targets.push(bound / 2);
            targets.push(bound.saturating_sub(1));
            targets.push(bound);
            targets.push(bound + 100);
            targets.sort_unstable();
            targets.dedup();

            for &level in &levels {
                for &target in &targets {
                    let label = format!(
                        "destSize shape={} len={} level={} target={}",
                        shape_name(shape),
                        len,
                        level,
                        target
                    );
                    let mut cdst = vec![0xAAu8; target + 64];
                    let mut rdst = vec![0xAAu8; target + 64];
                    let mut c_ssz: c_int = len as c_int;
                    let mut r_ssz: c_int = len as c_int;
                    let cn = unsafe {
                        (a.dest_size.0)(
                            cst.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            &mut c_ssz,
                            target as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.dest_size.1)(
                            rst.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            &mut r_ssz,
                            target as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return value", label);
                    assert_eq!(c_ssz, r_ssz, "{}: mutated *srcSizePtr", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cdst, &rdst);
                    unsafe {
                        assert_state_blob_eq(
                            &label,
                            cst.as_ptr() as *const c_void,
                            rst.as_ptr() as *const c_void,
                        )
                    };
                    if cn > 0 {
                        assert!(
                            cn as usize <= target,
                            "{}: wrote {} > target {}",
                            label,
                            cn,
                            target
                        );
                        assert!(
                            c_ssz >= 0 && c_ssz as usize <= len,
                            "{}: consumed {} of {}",
                            label,
                            c_ssz,
                            len
                        );
                        assert_roundtrip(&label, &cdst[..cn as usize], &src[..c_ssz as usize]);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Stream lifecycle
// ===========================================================================

#[test]
fn hc_create_free_init_and_reset_stream() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;

    unsafe {
        // createStreamHC: freshly allocated + level = LZ4HC_CLEVEL_DEFAULT.
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        assert!(!cs.is_null() && !rs.is_null(), "LZ4_createStreamHC");
        assert_state_blob_eq("LZ4_createStreamHC fresh state", cs, rs);
        assert_eq!(
            hc_scalars(cs).compression_level,
            LZ4HC_CLEVEL_DEFAULT as i16
        );

        // resetStreamHC / resetStreamHC_fast over the whole level axis.
        for &level in &all_levels() {
            (a.reset_stream.0)(cs, level);
            (a.reset_stream.1)(rs, level);
            assert_state_blob_eq(&format!("LZ4_resetStreamHC({})", level), cs, rs);
            (a.reset_stream_fast.0)(cs, level);
            (a.reset_stream_fast.1)(rs, level);
            assert_state_blob_eq(&format!("LZ4_resetStreamHC_fast({})", level), cs, rs);
        }

        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs), "freeStreamHC");
        // free on NULL is explicitly supported and returns 0.
        assert_eq!(
            (a.free_stream.0)(std::ptr::null_mut()),
            (a.free_stream.1)(std::ptr::null_mut()),
            "LZ4_freeStreamHC(NULL)"
        );
        assert_eq!((a.free_stream.0)(std::ptr::null_mut()), 0);

        // initStreamHC on a user buffer: returns the buffer, level = default.
        let mut cb = AlignedBuf::new(ssz, 64);
        let mut rb = AlignedBuf::new(ssz, 64);
        let cp = (a.init_stream.0)(cb.as_mut_ptr() as *mut c_void, ssz);
        let rp = (a.init_stream.1)(rb.as_mut_ptr() as *mut c_void, ssz);
        assert_eq!(cp as usize, cb.as_mut_ptr() as usize, "initStreamHC C ret");
        assert_eq!(rp as usize, rb.as_mut_ptr() as usize, "initStreamHC Rust ret");
        assert_state_blob_eq("LZ4_initStreamHC", cp, rp);

        // resetStreamHC_fast on a *dirty* stream must fall back to a full init:
        // force `dirty` by making a compression fail (dstCapacity == 0).
        let mut rng = Rng::new(0x0D1247);
        let src = src_buf(&mut rng, 5, 40000);
        for &level in &all_levels() {
            let mut cd = vec![0xAAu8; 8];
            let mut rd = vec![0xAAu8; 8];
            (a.reset_stream.0)(cp, level);
            (a.reset_stream.1)(rp, level);
            let cn = (a.cont.0)(
                cp,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                0,
            );
            let rn = (a.cont.1)(
                rp,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                0,
            );
            assert_eq!(cn, rn, "continue with dstCapacity 0 (level {})", level);
            assert_eq!(cn, 0, "continue with dstCapacity 0 must fail");
            assert_bytes_eq("dirty-forcing dst untouched", &cd, &rd);
            assert_eq!(hc_scalars(cp).dirty, 1, "C stream should be dirty");
            assert_state_blob_eq(&format!("dirty stream level={}", level), cp, rp);
            // Now the fast reset must take the "full init" branch.
            (a.reset_stream_fast.0)(cp, level);
            (a.reset_stream_fast.1)(rp, level);
            assert_state_blob_eq(&format!("fast reset of dirty stream level={}", level), cp, rp);
            assert_eq!(hc_scalars(cp).dirty, 0);
        }
    }
}

/// `LZ4_initStreamHC` must return NULL for NULL, undersized or misaligned
/// buffers (lz4hc.c:1573-1580).
#[test]
fn hc_init_stream_invalid_buffers() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    unsafe {
        // NULL buffer
        let cp = (a.init_stream.0)(std::ptr::null_mut(), ssz);
        let rp = (a.init_stream.1)(std::ptr::null_mut(), ssz);
        assert!(cp.is_null() && rp.is_null(), "initStreamHC(NULL) must be NULL");

        // Too small.
        let mut buf_c = AlignedBuf::new(ssz, 64);
        let mut buf_r = AlignedBuf::new(ssz, 64);
        for &size in &[0usize, 1, 7, 8, 1024, ssz - 8, ssz - 2, ssz - 1] {
            let cp = (a.init_stream.0)(buf_c.as_mut_ptr() as *mut c_void, size);
            let rp = (a.init_stream.1)(buf_r.as_mut_ptr() as *mut c_void, size);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "initStreamHC(size={}) nullness must agree",
                size
            );
            assert!(cp.is_null(), "initStreamHC(size={}) must be NULL", size);
        }
        // Exactly big enough, and bigger, succeed.
        for &size in &[ssz, ssz + 1, ssz * 2] {
            let cp = (a.init_stream.0)(buf_c.as_mut_ptr() as *mut c_void, size);
            let rp = (a.init_stream.1)(buf_r.as_mut_ptr() as *mut c_void, size);
            assert!(!cp.is_null() && !rp.is_null(), "initStreamHC(size={})", size);
            assert_state_blob_eq(&format!("initStreamHC(size={})", size), cp, rp);
        }

        // Misaligned buffer (LZ4_ALIGN_TEST is on -> 8-byte requirement).
        for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
            let mut mc = AlignedBuf::with_offset(ssz, 8, off);
            let mut mr = AlignedBuf::with_offset(ssz, 8, off);
            let cp = (a.init_stream.0)(mc.as_mut_ptr() as *mut c_void, ssz);
            let rp = (a.init_stream.1)(mr.as_mut_ptr() as *mut c_void, ssz);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "initStreamHC misaligned(+{}) nullness",
                off
            );
            assert!(cp.is_null(), "initStreamHC misaligned(+{}) must be NULL", off);
        }
    }
}

// ===========================================================================
// LZ4_setCompressionLevel / LZ4_favorDecompressionSpeed
// ===========================================================================

#[test]
fn hc_set_compression_level_out_of_range() {
    let a = api();
    let mut rng = Rng::new(0x5C1E_7E11);
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();

        // Pure setter behaviour over the whole int range.
        let mut probes = all_levels();
        probes.extend_from_slice(&[-2, -1000000, 14, 32767, 32768, 65535, 65536, 0x7FFF_FFFE]);
        for _ in 0..500 {
            probes.push(rng.next_u32() as c_int);
        }
        for &level in &probes {
            (a.set_level.0)(cs, level);
            (a.set_level.1)(rs, level);
            let c = hc_scalars(cs);
            let r = hc_scalars(rs);
            assert_eq!(c, r, "LZ4_setCompressionLevel({}) state", level);
            let expect: i16 = if level < 1 {
                LZ4HC_CLEVEL_DEFAULT as i16
            } else if level > LZ4HC_CLEVEL_MAX {
                LZ4HC_CLEVEL_MAX as i16
            } else {
                level as i16
            };
            assert_eq!(
                c.compression_level, expect,
                "LZ4_setCompressionLevel({}) clamping",
                level
            );
        }

        // Then actually compress with the out-of-range level in effect.
        let src = src_buf(&mut rng, 5, 30000);
        let bound = bound_of(src.len());
        for &level in &all_levels() {
            (a.reset_stream.0)(cs, LZ4HC_CLEVEL_DEFAULT);
            (a.reset_stream.1)(rs, LZ4HC_CLEVEL_DEFAULT);
            (a.set_level.0)(cs, level);
            (a.set_level.1)(rs, level);
            let mut cd = vec![0xAAu8; bound + 32];
            let mut rd = vec![0xAAu8; bound + 32];
            let cn = (a.cont.0)(
                cs,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
            );
            let rn = (a.cont.1)(
                rs,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
            );
            let label = format!("setCompressionLevel({}) + continue", level);
            assert_eq!(cn, rn, "{}: return", label);
            assert!(cn > 0, "{}: failed", label);
            assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
            assert_state_blob_eq(&label, cs, rs);
            assert_roundtrip(&label, &cd[..cn as usize], &src);
        }

        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

/// `favorDecSpeed` only changes the optimal parser, but the flag itself is set
/// unconditionally, so it is probed at low levels too. Favor values 2 and -1
/// must normalise to 1 (`favor != 0`).
#[test]
fn hc_favor_decompression_speed_axis() {
    let a = api();
    let mut rng = Rng::new(0xFA_B0_0D);
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            for &len in &[13usize, 1000, 4096, 65547, 100000] {
                let src = src_buf(&mut rng, shape, len);
                let bound = bound_of(len);
                for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                    for &fav in &[0i32, 1, 2, -1] {
                        (a.reset_stream.0)(cs, level);
                        (a.reset_stream.1)(rs, level);
                        (a.favor.0)(cs, fav);
                        (a.favor.1)(rs, fav);
                        let label = format!(
                            "favorDecSpeed({}) level={} shape={} len={}",
                            fav,
                            level,
                            shape_name(shape),
                            len
                        );
                        assert_eq!(
                            hc_scalars(cs).favor_dec_speed,
                            hc_scalars(rs).favor_dec_speed,
                            "{}: flag",
                            label
                        );
                        assert_eq!(
                            hc_scalars(cs).favor_dec_speed,
                            (fav != 0) as i8,
                            "{}: flag value",
                            label
                        );
                        // 2 / -1 normalise to the same flag as 1, so only run
                        // the (expensive) compression for the two distinct
                        // behaviours.
                        if fav != 0 && fav != 1 {
                            continue;
                        }
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let cn = (a.cont.0)(
                            cs,
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        assert_state_blob_eq(&label, cs, rs);
                        if cn > 0 {
                            assert_roundtrip(&label, &cd[..cn as usize], &src);
                        }
                    }
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// LZ4_loadDictHC
// ===========================================================================

#[test]
fn hc_load_dict_size_axis() {
    let a = api();
    let mut rng = Rng::new(0x10AD_D1C7);
    // 0 / 1 / 4 (== LZ4HC_HASHSIZE) / small / the 64 KB boundary / >64 KB
    // (truncated to the trailing 64 KB by lz4hc.c:1636).
    let dict_sizes = [0usize, 1, 2, 3, 4, 5, 100, 4096, 65535, 65536, 65537, 70000];
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let dict = src_buf(&mut rng, shape, 70000);
            let src = src_buf(&mut rng, shape, 30000);
            let bound = bound_of(src.len());
            for &dsz in &dict_sizes {
                for &level in &[c_int::MIN, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13, 100, c_int::MAX] {
                    let label = format!(
                        "loadDictHC shape={} dictSize={} level={}",
                        shape_name(shape),
                        dsz,
                        level
                    );
                    // The level must be set BEFORE loading the dictionary.
                    (a.reset_stream.0)(cs, level);
                    (a.reset_stream.1)(rs, level);
                    let cl = (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, dsz as c_int);
                    let rl = (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, dsz as c_int);
                    assert_eq!(cl, rl, "{}: LZ4_loadDictHC return", label);
                    assert_eq!(
                        cl as usize,
                        dsz.min(65536),
                        "{}: dict truncation to trailing 64 KB",
                        label
                    );
                    assert_state_blob_eq(&label, cs, rs);

                    // ... then compress a block against it.
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = (a.cont.0)(
                        cs,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        src.len() as c_int,
                        bound as c_int,
                    );
                    let rn = (a.cont.1)(
                        rs,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        src.len() as c_int,
                        bound as c_int,
                    );
                    assert_eq!(cn, rn, "{}: continue return", label);
                    assert!(cn > 0, "{}: continue failed", label);
                    assert_bytes_eq(&format!("{}: continue dst", label), &cd, &rd);
                    assert_state_blob_eq(&format!("{} after continue", label), cs, rs);

                    // loadDictHC keeps `dict[dsz-used .. dsz]` (it only drops a
                    // leading part when dsz > 64 KB).
                    let used = cl as usize;
                    let hist = &dict[dsz - used..dsz];
                    assert_roundtrip_dict(&label, &cd[..cn as usize], &src, hist);
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// LZ4_compress_HC_continue : multi-block streaming
// ===========================================================================

/// Blocks laid out contiguously in one shared buffer -> the "prefix" path
/// (`src == ctxPtr->end`, no `LZ4HC_setExternalDict`).
#[test]
fn hc_continue_multiblock_prefix() {
    let a = api();
    let mut rng = Rng::new(0xB10C_C047);
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            for nblocks in 2..=8usize {
                // Block sizes deliberately include < MINMATCH and > 64 KB.
                let mut sizes: Vec<usize> = Vec::new();
                for i in 0..nblocks {
                    sizes.push(match (i + nblocks) % 6 {
                        0 => 0,
                        1 => 3,
                        2 => 13,
                        3 => rng.range(1, 5000),
                        4 => rng.range(60000, 70000),
                        _ => rng.range(1, 40000),
                    });
                }
                let total: usize = sizes.iter().sum();
                let src = src_buf(&mut rng, shape, total);
                for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                    (a.reset_stream_fast.0)(cs, level);
                    (a.reset_stream_fast.1)(rs, level);
                    let mut hist: Vec<u8> = Vec::new();
                    let mut off = 0usize;
                    for (bi, &bsz) in sizes.iter().enumerate() {
                        let label = format!(
                            "continue prefix shape={} nblocks={} block={} size={} level={}",
                            shape_name(shape),
                            nblocks,
                            bi,
                            bsz,
                            level
                        );
                        let bound = bound_of(bsz);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let sp = src.as_ptr().add(off) as *const c_char;
                        let cn = (a.cont.0)(
                            cs,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert!(cn > 0, "{}: failed", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        assert_state_blob_eq(&label, cs, rs);

                        let block = &src[off..off + bsz];
                        assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                        hist.extend_from_slice(block);
                        trim_hist(&mut hist);
                        off += bsz;
                    }
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

/// Blocks scattered (with gaps) inside ONE shared arena -> the extDict path
/// (`src != ctxPtr->end` triggers `LZ4HC_setExternalDict`). Sharing the arena
/// between the two libraries keeps the pointer comparisons in
/// `LZ4_compressHC_continue_generic`'s overlap detection identical.
#[test]
fn hc_continue_multiblock_extdict() {
    let a = api();
    let mut rng = Rng::new(0xE47D_1C7);
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let arena = src_buf(&mut rng, shape, 400_000);
            for nblocks in 2..=8usize {
                let mut offs: Vec<(usize, usize)> = Vec::new();
                let mut cur = 1024usize;
                for i in 0..nblocks {
                    let bsz = match (i + nblocks) % 5 {
                        0 => 3,
                        1 => 13,
                        2 => rng.range(1, 3000),
                        3 => rng.range(60000, 68000),
                        _ => rng.range(1, 20000),
                    };
                    if cur + bsz + 4096 > arena.len() {
                        break;
                    }
                    offs.push((cur, bsz));
                    cur += bsz + rng.range(1, 4096);
                }
                for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                    (a.reset_stream_fast.0)(cs, level);
                    (a.reset_stream_fast.1)(rs, level);
                    let mut hist: Vec<u8> = Vec::new();
                    for (bi, &(o, bsz)) in offs.iter().enumerate() {
                        let label = format!(
                            "continue extDict shape={} nblocks={} block={} size={} level={}",
                            shape_name(shape),
                            nblocks,
                            bi,
                            bsz,
                            level
                        );
                        let bound = bound_of(bsz);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let sp = arena.as_ptr().add(o) as *const c_char;
                        let cn = (a.cont.0)(
                            cs,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert!(cn > 0, "{}: failed", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        assert_state_blob_eq(&label, cs, rs);

                        let block = &arena[o..o + bsz];
                        assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                        hist.extend_from_slice(block);
                        trim_hist(&mut hist);
                    }
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

/// `LZ4_setCompressionLevel` between blocks of the same stream, including
/// crossing the hashChain <-> optimal boundary and the lz4mid <-> lz4hc
/// boundary (which changes the meaning of the hash tables) mid-stream.
#[test]
fn hc_continue_level_change_midstream() {
    let a = api();
    let mut rng = Rng::new(0x1E7E_1C46);
    let level_walks: [&[c_int]; 6] = [
        &[2, 9, 12, 2, 10, 1],
        &[12, 11, 10, 9, 3, 2],
        &[1, 12, 1, 12, 1, 12],
        &[9, 9, 10, 10, 2, 2],
        &[c_int::MIN, 100, 1, c_int::MAX, 0, 5],
        &[3, 4, 5, 6, 7, 8],
    ];
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let sizes = [7000usize, 3, 40000, 13, 66000, 5000];
            let total: usize = sizes.iter().sum();
            let src = src_buf(&mut rng, shape, total);
            for walk in level_walks.iter() {
                (a.reset_stream_fast.0)(cs, walk[0]);
                (a.reset_stream_fast.1)(rs, walk[0]);
                let mut hist: Vec<u8> = Vec::new();
                let mut off = 0usize;
                for (bi, &bsz) in sizes.iter().enumerate() {
                    let level = walk[bi % walk.len()];
                    (a.set_level.0)(cs, level);
                    (a.set_level.1)(rs, level);
                    let label = format!(
                        "midstream level change shape={} block={} size={} level={}",
                        shape_name(shape),
                        bi,
                        bsz,
                        level
                    );
                    let bound = bound_of(bsz);
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let sp = src.as_ptr().add(off) as *const c_char;
                    let cn = (a.cont.0)(
                        cs,
                        sp,
                        cd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        bound as c_int,
                    );
                    let rn = (a.cont.1)(
                        rs,
                        sp,
                        rd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        bound as c_int,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(&label, cs, rs);
                    let block = &src[off..off + bsz];
                    assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                    hist.extend_from_slice(block);
                    trim_hist(&mut hist);
                    off += bsz;
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// LZ4_compress_HC_continue_destSize
// ===========================================================================

#[test]
fn hc_continue_destsize() {
    let a = api();
    let mut rng = Rng::new(0xC0_D357);
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let sizes = [5000usize, 3, 30000, 13, 66000];
            let total: usize = sizes.iter().sum();
            let src = src_buf(&mut rng, shape, total);
            for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                for &target_kind in &[0usize, 1, 2, 3, 4, 5] {
                    (a.reset_stream_fast.0)(cs, level);
                    (a.reset_stream_fast.1)(rs, level);
                    let mut hist: Vec<u8> = Vec::new();
                    let mut off = 0usize;
                    for (bi, &bsz) in sizes.iter().enumerate() {
                        let bound = bound_of(bsz);
                        let target = match target_kind {
                            0 => 0,
                            1 => 1,
                            2 => 16,
                            3 => bsz / 4 + 1,
                            4 => bound / 2,
                            _ => bound,
                        };
                        let label = format!(
                            "continue_destSize shape={} level={} block={} size={} target={}",
                            shape_name(shape),
                            level,
                            bi,
                            bsz,
                            target
                        );
                        let mut cd = vec![0xAAu8; target + 64];
                        let mut rd = vec![0xAAu8; target + 64];
                        let mut c_ssz = bsz as c_int;
                        let mut r_ssz = bsz as c_int;
                        let sp = src.as_ptr().add(off) as *const c_char;
                        let cn = (a.cont_dest_size.0)(
                            cs,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            &mut c_ssz,
                            target as c_int,
                        );
                        let rn = (a.cont_dest_size.1)(
                            rs,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            &mut r_ssz,
                            target as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert_eq!(c_ssz, r_ssz, "{}: mutated *srcSizePtr", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        assert_state_blob_eq(&label, cs, rs);
                        if cn <= 0 {
                            // Compression failed -> the stream is now dirty and
                            // must be reset before further use.
                            break;
                        }
                        assert!(cn as usize <= target, "{}: overflow", label);
                        let consumed = c_ssz as usize;
                        assert!(consumed <= bsz, "{}: consumed too much", label);
                        let block = &src[off..off + consumed];
                        assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                        hist.extend_from_slice(block);
                        trim_hist(&mut hist);
                        off += consumed;
                        if consumed < bsz {
                            // Only a prefix was consumed; the rest of this
                            // sequence would no longer be contiguous.
                            break;
                        }
                    }
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// LZ4_saveDictHC
// ===========================================================================

#[test]
fn hc_save_dict_hc() {
    let a = api();
    let mut rng = Rng::new(0x5A7E_D1C7);
    // saveDictHC writes into a caller buffer and then re-points the stream at
    // it, so each library needs its own arena. The arenas have IDENTICAL
    // layouts (save area first, source blocks later) so that the pointer
    // comparisons inside LZ4_compressHC_continue_generic behave identically.
    const SAVE_AREA: usize = 80_000;
    const SRC_OFF: usize = 131_072;
    const ARENA: usize = 400_000;

    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let payload = src_buf(&mut rng, shape, ARENA - SRC_OFF);
            let mut arena_c = vec![0xAAu8; ARENA];
            let mut arena_r = vec![0xAAu8; ARENA];
            arena_c[SRC_OFF..].copy_from_slice(&payload);
            arena_r[SRC_OFF..].copy_from_slice(&payload);

            // Large first blocks are expensive at level 12, so only pair them
            // with a few interesting maxDictSize values.
            let cases: [(usize, &[usize]); 3] = [
                (13, &[0, 1, 3, 4, 100, 65535, 65536, 70000]),
                (5000, &[0, 1, 3, 4, 100, 65535, 65536, 70000]),
                (70000, &[0, 1, 3, 4, 100, 65535, 65536, 70000]),
            ];
            for (b0, max_dicts) in cases.iter() {
                let b0 = *b0;
                for &max_dict in max_dicts.iter() {
                    for &level in &[1i32, 2, 3, 9, 10, 11, 12] {
                        let label = format!(
                            "saveDictHC shape={} maxDict={} level={} b0={}",
                            shape_name(shape),
                            max_dict,
                            level,
                            b0
                        );
                        // Reset both save areas to the sentinel.
                        for x in &mut arena_c[..SAVE_AREA] {
                            *x = 0xAA;
                        }
                        for x in &mut arena_r[..SAVE_AREA] {
                            *x = 0xAA;
                        }
                        (a.reset_stream.0)(cs, level);
                        (a.reset_stream.1)(rs, level);

                        // Block 0 out of each arena.
                        let bound = bound_of(b0);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let cn = (a.cont.0)(
                            cs,
                            arena_c.as_ptr().add(SRC_OFF) as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            b0 as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            arena_r.as_ptr().add(SRC_OFF) as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            b0 as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: block0 return", label);
                        assert!(cn > 0, "{}: block0 failed", label);
                        assert_bytes_eq(&format!("{}: block0 dst", label), &cd, &rd);
                        assert_state_eq(&format!("{} after block0", label), cs, rs);
                        assert_roundtrip(&label, &cd[..cn as usize], &payload[..b0]);

                        // Save the history.
                        let csv = (a.save_dict.0)(
                            cs,
                            arena_c.as_mut_ptr() as *mut c_char,
                            max_dict as c_int,
                        );
                        let rsv = (a.save_dict.1)(
                            rs,
                            arena_r.as_mut_ptr() as *mut c_char,
                            max_dict as c_int,
                        );
                        assert_eq!(csv, rsv, "{}: LZ4_saveDictHC return", label);
                        let expect = {
                            let mut d = max_dict.min(65536);
                            if d < 4 {
                                d = 0;
                            }
                            d.min(b0)
                        };
                        assert_eq!(csv as usize, expect, "{}: saved size", label);
                        assert_bytes_eq(
                            &format!("{}: safeBuffer contents", label),
                            &arena_c[..SAVE_AREA],
                            &arena_r[..SAVE_AREA],
                        );
                        if csv > 0 {
                            assert_bytes_eq(
                                &format!("{}: safeBuffer == tail of block0", label),
                                &payload[b0 - csv as usize..b0],
                                &arena_c[..csv as usize],
                            );
                        }
                        assert_state_eq(&format!("{} after saveDictHC", label), cs, rs);

                        // Continue from the saved dictionary with a new block.
                        let b1 = 20_000usize;
                        let b1_off = SRC_OFF + b0;
                        let bound = bound_of(b1);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let cn = (a.cont.0)(
                            cs,
                            arena_c.as_ptr().add(b1_off) as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            b1 as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            arena_r.as_ptr().add(b1_off) as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            b1 as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: block1 return", label);
                        assert!(cn > 0, "{}: block1 failed", label);
                        assert_bytes_eq(&format!("{}: block1 dst", label), &cd, &rd);
                        assert_state_eq(&format!("{} after block1", label), cs, rs);
                        let block1 = &payload[b0..b0 + b1];
                        assert_roundtrip_dict(
                            &label,
                            &cd[..cn as usize],
                            block1,
                            &arena_c[..csv as usize],
                        );
                    }
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// LZ4_attach_HC_dictionary
// ===========================================================================

#[test]
fn hc_attach_hc_dictionary() {
    let a = api();
    let mut rng = Rng::new(0xA77A_C4ED);
    let src_lens = [13usize, 3000, 4097, 30000, 70000];
    unsafe {
        let c_dict = (a.create_stream.0)();
        let r_dict = (a.create_stream.1)();
        let c_work = (a.create_stream.0)();
        let r_work = (a.create_stream.1)();

        for shape in 0..N_SHAPES {
            let dict = src_buf(&mut rng, shape, 65536);
            let srcs: Vec<Vec<u8>> = src_lens
                .iter()
                .map(|&n| src_buf(&mut rng, shape, n))
                .collect();
            for &dsz in &[0usize, 4, 100, 4096, 65535, 65536] {
                for &level in &[1i32, 2, 3, 9, 10, 12, c_int::MAX] {
                    // Build the dictionary stream with loadDictHC ...
                    (a.reset_stream.0)(c_dict, level);
                    (a.reset_stream.1)(r_dict, level);
                    let cl = (a.load_dict.0)(c_dict, dict.as_ptr() as *const c_char, dsz as c_int);
                    let rl = (a.load_dict.1)(r_dict, dict.as_ptr() as *const c_char, dsz as c_int);
                    assert_eq!(cl, rl, "attach: loadDictHC return");

                    // ... then attach it to a freshly reset working stream.
                    for src in srcs.iter() {
                        let srclen = src.len();
                        let label = format!(
                            "attach shape={} dictSize={} level={} srcLen={}",
                            shape_name(shape),
                            dsz,
                            level,
                            srclen
                        );
                        (a.reset_stream_fast.0)(c_work, level);
                        (a.reset_stream_fast.1)(r_work, level);
                        (a.attach.0)(c_work, c_dict);
                        (a.attach.1)(r_work, r_dict);
                        assert_eq!(
                            hc_scalars(c_work).dict_ctx_null,
                            hc_scalars(r_work).dict_ctx_null
                        );
                        assert!(!hc_scalars(c_work).dict_ctx_null, "dictCtx must be set");

                        let bound = bound_of(srclen);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let cn = (a.cont.0)(
                            c_work,
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            srclen as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            r_work,
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            srclen as c_int,
                            bound as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert!(cn > 0, "{}: failed", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        // The working contexts reference different dictionary
                        // contexts, so compare structurally.
                        assert_state_eq(&label, c_work, r_work);
                        let used = cl as usize;
                        let hist = &dict[dsz - used..dsz];
                        assert_roundtrip_dict(&label, &cd[..cn as usize], src, hist);
                    }

                    // Detaching with NULL must clear the association.
                    (a.attach.0)(c_work, std::ptr::null());
                    (a.attach.1)(r_work, std::ptr::null());
                    assert!(hc_scalars(c_work).dict_ctx_null);
                    assert!(hc_scalars(r_work).dict_ctx_null);
                }
            }
        }
        (a.free_stream.0)(c_dict);
        (a.free_stream.1)(r_dict);
        (a.free_stream.0)(c_work);
        (a.free_stream.1)(r_work);
    }
}

// ===========================================================================
// LZ4HC_searchExtDict — direct FFI call
// ===========================================================================

/// `LZ4HC_searchExtDict` is exported, so it is called directly here with the
/// exact argument shape its internal callers use
/// (`LZ4HC_InsertAndGetWiderMatch` / `LZ4MID_searchHCDict`):
///   * `dictCtx` is a stream prepared with `LZ4_loadDictHC` (offset 0 of
///     `LZ4_streamHC_t` is `internal_donotuse`),
///   * `gDictEndIndex` is the working context's `dictLimit`, which is exactly
///     64 KB right after `LZ4HC_init_internal`,
///   * `ipIndex` is `(ip - prefixStart) + dictLimit`.
/// The dictionary lives in the middle of a 3x64 KB arena so that any transient
/// out-of-window `matchPtr` the chain walk computes still points into mapped
/// memory (the `LZ4_DISTANCE_MAX` check rejects those before dereferencing).
///
/// The extDict *streaming* path is additionally covered indirectly by
/// `hc_continue_multiblock_extdict` and `hc_attach_hc_dictionary`.
#[test]
fn hc_search_ext_dict_direct() {
    let a = api();
    let mut rng = Rng::new(0x5EA2_C4ED);
    const PAD: usize = 65536;
    // Guard against a vacuous pass: the search must actually report matches.
    let mut nonempty = 0usize;

    unsafe {
        let cstream = (a.create_stream.0)();
        let rstream = (a.create_stream.1)();

        for shape in 0..N_SHAPES {
            for &dsz in &[16usize, 100, 4096, 40000, 65000] {
                // arena: [PAD guard][dict of dsz][PAD guard]
                let mut arena = vec![0u8; PAD * 2 + dsz + 64];
                let body = gen_shape(&mut rng, shape, dsz + PAD * 2);
                arena[..body.len()].copy_from_slice(&body);
                let dict_ptr = arena.as_ptr().add(PAD);

                // Source sharing content with the dictionary so matches exist.
                let mut src = arena[PAD..PAD + dsz].to_vec();
                src.extend_from_slice(&gen_shape(&mut rng, shape, 4096));
                src.reserve(64);
                let src_ptr = src.as_ptr();
                let i_high = src_ptr.add(src.len() - 16);

                for &level in &[1i32, 2, 3, 6, 9, 10, 11, 12, 100] {
                    (a.reset_stream.0)(cstream, level);
                    (a.reset_stream.1)(rstream, level);
                    let cl = (a.load_dict.0)(cstream, dict_ptr as *const c_char, dsz as c_int);
                    let rl = (a.load_dict.1)(rstream, dict_ptr as *const c_char, dsz as c_int);
                    assert_eq!(cl, rl);
                    let g_dict_end_index: u32 = 65536; // working ctx dictLimit

                    for &ip_off in &[0usize, 1, 4, 37, 500, 2000] {
                        if ip_off + 32 >= src.len() {
                            continue;
                        }
                        let ip = src_ptr.add(ip_off);
                        let ip_index = g_dict_end_index + ip_off as u32;
                        for &nb in &[0i32, 1, 2, 4, 16] {
                            for &best in &[MINMATCH as c_int - 1, 8, 1000] {
                                for &low_back in &[0usize, 3] {
                                    let low = if low_back <= ip_off {
                                        ip.sub(low_back)
                                    } else {
                                        ip
                                    };
                                    let cm = (a.search_ext_dict.0)(
                                        ip,
                                        ip_index,
                                        low,
                                        i_high,
                                        cstream as *const c_void,
                                        g_dict_end_index,
                                        best,
                                        nb,
                                    );
                                    let rm = (a.search_ext_dict.1)(
                                        ip,
                                        ip_index,
                                        low,
                                        i_high,
                                        rstream as *const c_void,
                                        g_dict_end_index,
                                        best,
                                        nb,
                                    );
                                    assert_eq!(
                                        cm, rm,
                                        "LZ4HC_searchExtDict shape={} dsz={} level={} ipOff={} nb={} best={} lowBack={}",
                                        shape_name(shape), dsz, level, ip_off, nb, best, low_back
                                    );
                                    if cm.off != 0 {
                                        nonempty += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                std::hint::black_box(&arena);
            }
        }
        (a.free_stream.0)(cstream);
        (a.free_stream.1)(rstream);
    }
    assert!(
        nonempty > 0,
        "LZ4HC_searchExtDict never reported a match — the test would be vacuous"
    );
}

// ===========================================================================
// Deprecated / legacy entry points
// ===========================================================================

#[test]
fn hc_deprecated_oneshot_and_withstate() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut cst = AlignedBuf::new(ssz, 64);
    let mut rst = AlignedBuf::new(ssz, 64);
    let mut rng = Rng::new(0xDE12_0000);
    let levels = [c_int::MIN, -1, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13, 100, c_int::MAX];

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 4, 13, 64, 1000, 4096, 65547, 100000] {
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            let sp = src.as_ptr() as *const c_char;

            // ---- LZ4_compressHC (implicit level 0 -> default 9) ----
            {
                let label = format!("LZ4_compressHC shape={} len={}", shape_name(shape), len);
                let mut cd = vec![0xAAu8; bound + 32];
                let mut rd = vec![0xAAu8; bound + 32];
                let cn = unsafe { (a.d_hc.0)(sp, cd.as_mut_ptr() as *mut c_char, len as c_int) };
                let rn = unsafe { (a.d_hc.1)(sp, rd.as_mut_ptr() as *mut c_char, len as c_int) };
                assert_eq!(cn, rn, "{}: return", label);
                assert!(cn > 0, "{}: failed", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                assert_roundtrip(&label, &cd[..cn as usize], &src);
            }

            // ---- LZ4_compressHC_limitedOutput ----
            for &cap in &[0usize, 1, bound / 2, bound] {
                let label = format!(
                    "LZ4_compressHC_limitedOutput shape={} len={} cap={}",
                    shape_name(shape),
                    len,
                    cap
                );
                let mut cd = vec![0xAAu8; cap + 32];
                let mut rd = vec![0xAAu8; cap + 32];
                let cn = unsafe {
                    (a.d_hc_lim.0)(sp, cd.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int)
                };
                let rn = unsafe {
                    (a.d_hc_lim.1)(sp, rd.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int)
                };
                assert_eq!(cn, rn, "{}: return", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                if cn > 0 {
                    assert_roundtrip(&label, &cd[..cn as usize], &src);
                }
            }

            // ---- LZ4_compressHC_withStateHC ----
            {
                let label = format!(
                    "LZ4_compressHC_withStateHC shape={} len={}",
                    shape_name(shape),
                    len
                );
                let mut cd = vec![0xAAu8; bound + 32];
                let mut rd = vec![0xAAu8; bound + 32];
                let cn = unsafe {
                    (a.d_hc_st.0)(
                        cst.as_mut_ptr() as *mut c_void,
                        sp,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    )
                };
                let rn = unsafe {
                    (a.d_hc_st.1)(
                        rst.as_mut_ptr() as *mut c_void,
                        sp,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    )
                };
                assert_eq!(cn, rn, "{}: return", label);
                assert!(cn > 0, "{}: failed", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                unsafe {
                    assert_state_blob_eq(
                        &label,
                        cst.as_ptr() as *const c_void,
                        rst.as_ptr() as *const c_void,
                    )
                };
                assert_roundtrip(&label, &cd[..cn as usize], &src);
            }

            // ---- LZ4_compressHC_limitedOutput_withStateHC ----
            for &cap in &[0usize, 1, bound / 2, bound] {
                let label = format!(
                    "LZ4_compressHC_limitedOutput_withStateHC shape={} len={} cap={}",
                    shape_name(shape),
                    len,
                    cap
                );
                let mut cd = vec![0xAAu8; cap + 32];
                let mut rd = vec![0xAAu8; cap + 32];
                let cn = unsafe {
                    (a.d_hc_lim_st.0)(
                        cst.as_mut_ptr() as *mut c_void,
                        sp,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    )
                };
                let rn = unsafe {
                    (a.d_hc_lim_st.1)(
                        rst.as_mut_ptr() as *mut c_void,
                        sp,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    )
                };
                assert_eq!(cn, rn, "{}: return", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                unsafe {
                    assert_state_blob_eq(
                        &label,
                        cst.as_ptr() as *const c_void,
                        rst.as_ptr() as *const c_void,
                    )
                };
                if cn > 0 {
                    assert_roundtrip(&label, &cd[..cn as usize], &src);
                }
            }

            for &level in &levels {
                // ---- LZ4_compressHC2 ----
                {
                    let label = format!(
                        "LZ4_compressHC2 shape={} len={} level={}",
                        shape_name(shape),
                        len,
                        level
                    );
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = unsafe {
                        (a.d_hc2.0)(sp, cd.as_mut_ptr() as *mut c_char, len as c_int, level)
                    };
                    let rn = unsafe {
                        (a.d_hc2.1)(sp, rd.as_mut_ptr() as *mut c_char, len as c_int, level)
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_roundtrip(&label, &cd[..cn as usize], &src);
                }

                // ---- LZ4_compressHC2_limitedOutput ----
                {
                    let cap = bound / 2;
                    let label = format!(
                        "LZ4_compressHC2_limitedOutput shape={} len={} level={} cap={}",
                        shape_name(shape),
                        len,
                        level,
                        cap
                    );
                    let mut cd = vec![0xAAu8; cap + 32];
                    let mut rd = vec![0xAAu8; cap + 32];
                    let cn = unsafe {
                        (a.d_hc2_lim.0)(
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.d_hc2_lim.1)(
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    if cn > 0 {
                        assert_roundtrip(&label, &cd[..cn as usize], &src);
                    }
                }

                // ---- LZ4_compressHC2_withStateHC ----
                {
                    let label = format!(
                        "LZ4_compressHC2_withStateHC shape={} len={} level={}",
                        shape_name(shape),
                        len,
                        level
                    );
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = unsafe {
                        (a.d_hc2_st.0)(
                            cst.as_mut_ptr() as *mut c_void,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.d_hc2_st.1)(
                            rst.as_mut_ptr() as *mut c_void,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    unsafe {
                        assert_state_blob_eq(
                            &label,
                            cst.as_ptr() as *const c_void,
                            rst.as_ptr() as *const c_void,
                        )
                    };
                    assert_roundtrip(&label, &cd[..cn as usize], &src);
                }

                // ---- LZ4_compressHC2_limitedOutput_withStateHC ----
                {
                    let cap = bound / 3;
                    let label = format!(
                        "LZ4_compressHC2_limitedOutput_withStateHC shape={} len={} level={} cap={}",
                        shape_name(shape),
                        len,
                        level,
                        cap
                    );
                    let mut cd = vec![0xAAu8; cap + 32];
                    let mut rd = vec![0xAAu8; cap + 32];
                    let cn = unsafe {
                        (a.d_hc2_lim_st.0)(
                            cst.as_mut_ptr() as *mut c_void,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.d_hc2_lim_st.1)(
                            rst.as_mut_ptr() as *mut c_void,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    unsafe {
                        assert_state_blob_eq(
                            &label,
                            cst.as_ptr() as *const c_void,
                            rst.as_ptr() as *const c_void,
                        )
                    };
                    if cn > 0 {
                        assert_roundtrip(&label, &cd[..cn as usize], &src);
                    }
                }
            }
        }
    }
}

#[test]
fn hc_deprecated_streaming_and_legacy_state() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut rng = Rng::new(0xDE12_0001);

    unsafe {
        // ---- LZ4_compressHC_continue / LZ4_compressHC_limitedOutput_continue
        // (both take a real LZ4_streamHC_t*) ----
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for shape in 0..N_SHAPES {
            let sizes = [4000usize, 3, 25000, 13, 66000];
            let total: usize = sizes.iter().sum();
            let src = src_buf(&mut rng, shape, total);
            for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let mut hist: Vec<u8> = Vec::new();
                let mut off = 0usize;
                for (bi, &bsz) in sizes.iter().enumerate() {
                    let bound = bound_of(bsz);
                    let sp = src.as_ptr().add(off) as *const c_char;
                    let label = format!(
                        "LZ4_compressHC_continue shape={} level={} block={} size={}",
                        shape_name(shape),
                        level,
                        bi,
                        bsz
                    );
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    // alternate between the two deprecated continue wrappers
                    let (cn, rn) = if bi % 2 == 0 {
                        (
                            (a.d_hc_cont.0)(cs, sp, cd.as_mut_ptr() as *mut c_char, bsz as c_int),
                            (a.d_hc_cont.1)(rs, sp, rd.as_mut_ptr() as *mut c_char, bsz as c_int),
                        )
                    } else {
                        (
                            (a.d_hc_lim_cont.0)(
                                cs,
                                sp,
                                cd.as_mut_ptr() as *mut c_char,
                                bsz as c_int,
                                bound as c_int,
                            ),
                            (a.d_hc_lim_cont.1)(
                                rs,
                                sp,
                                rd.as_mut_ptr() as *mut c_char,
                                bsz as c_int,
                                bound as c_int,
                            ),
                        )
                    };
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(&label, cs, rs);
                    let block = &src[off..off + bsz];
                    assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                    hist.extend_from_slice(block);
                    trim_hist(&mut hist);
                    off += bsz;
                }
            }
        }
        // LZ4_compressHC_limitedOutput_continue with a too-small budget.
        {
            let src = src_buf(&mut rng, 4, 20000);
            for &level in &[1i32, 2, 9, 12] {
                for &cap in &[0usize, 1, 8, 64] {
                    (a.reset_stream.0)(cs, level);
                    (a.reset_stream.1)(rs, level);
                    let mut cd = vec![0xAAu8; cap + 32];
                    let mut rd = vec![0xAAu8; cap + 32];
                    let cn = (a.d_hc_lim_cont.0)(
                        cs,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        src.len() as c_int,
                        cap as c_int,
                    );
                    let rn = (a.d_hc_lim_cont.1)(
                        rs,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        src.len() as c_int,
                        cap as c_int,
                    );
                    assert_eq!(
                        cn, rn,
                        "LZ4_compressHC_limitedOutput_continue cap={} level={}",
                        cap, level
                    );
                    assert_eq!(cn, 0, "cap={} must fail", cap);
                    assert_bytes_eq("deprecated continue overflow dst", &cd, &rd);
                    assert_state_blob_eq("deprecated continue overflow state", cs, rs);
                }
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);

        // ---- LZ4_createHC / LZ4_compressHC2_continue /
        // LZ4_compressHC2_limitedOutput_continue / LZ4_slideInputBufferHC /
        // LZ4_freeHC ----
        //
        // LZ4_compressHC2_continue calls LZ4HC_compress_generic() directly with
        // dstCapacity==0 and `notLimited`, so `dst` must be >= compressBound()
        // and `src` must start exactly at the context's `end`.
        //
        // NOTE: for the lz4mid strategy (levels 1 and 2) LZ4MID_compress ends
        // with a live `assert(op <= oend)` (lz4hc.c:743) and `oend == dst + 0`
        // here, so that combination aborts a non-NDEBUG C build. Levels 1/2 are
        // therefore only driven through the *limitedOutput* variant, which
        // receives a real capacity.
        let non_mid_levels = [c_int::MIN, 0, 3, 4, 9, 10, 11, 12, 100, c_int::MAX];
        let mid_levels = [1i32, 2];
        for shape in 0..N_SHAPES {
            let sizes = [3000usize, 12000, 40000];
            let total: usize = sizes.iter().sum();
            let src = src_buf(&mut rng, shape, total);

            // (a) notLimited variant, non-lz4mid levels only
            for &level in &non_mid_levels {
                let cdata = (a.d_create_hc.0)(src.as_ptr() as *const c_char);
                let rdata = (a.d_create_hc.1)(src.as_ptr() as *const c_char);
                assert!(!cdata.is_null() && !rdata.is_null(), "LZ4_createHC");
                assert_state_blob_eq("LZ4_createHC fresh", cdata, rdata);

                let mut hist: Vec<u8> = Vec::new();
                let mut off = 0usize;
                for (bi, &bsz) in sizes.iter().enumerate() {
                    let bound = bound_of(bsz);
                    let sp = src.as_ptr().add(off) as *const c_char;
                    let label = format!(
                        "LZ4_compressHC2_continue shape={} level={} block={}",
                        shape_name(shape),
                        level,
                        bi
                    );
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = (a.d_hc2_cont.0)(
                        cdata,
                        sp,
                        cd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        level,
                    );
                    let rn = (a.d_hc2_cont.1)(
                        rdata,
                        sp,
                        rd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        level,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(&label, cdata, rdata);
                    let block = &src[off..off + bsz];
                    assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                    hist.extend_from_slice(block);
                    trim_hist(&mut hist);
                    off += bsz;
                }

                // LZ4_slideInputBufferHC returns prefixStart-dictLimit+lowLimit
                // and resets the stream; both sides were given the SAME input
                // buffer, so the returned pointers must be identical.
                let cp = (a.d_slide.0)(cdata);
                let rp = (a.d_slide.1)(rdata);
                assert_eq!(
                    cp as usize,
                    rp as usize,
                    "LZ4_slideInputBufferHC pointer (shape={} level={})",
                    shape_name(shape),
                    level
                );
                assert_eq!(
                    cp as usize,
                    src.as_ptr() as usize,
                    "LZ4_slideInputBufferHC should point at the input buffer"
                );
                assert_state_blob_eq("after LZ4_slideInputBufferHC", cdata, rdata);

                assert_eq!((a.d_free_hc.0)(cdata), (a.d_free_hc.1)(rdata), "LZ4_freeHC");
            }

            // (b) limitedOutput variant, all levels (incl. lz4mid)
            for &level in mid_levels.iter().chain(non_mid_levels.iter()) {
                let cdata = (a.d_create_hc.0)(src.as_ptr() as *const c_char);
                let rdata = (a.d_create_hc.1)(src.as_ptr() as *const c_char);
                let mut hist: Vec<u8> = Vec::new();
                let mut off = 0usize;
                for (bi, &bsz) in sizes.iter().enumerate() {
                    let bound = bound_of(bsz);
                    let sp = src.as_ptr().add(off) as *const c_char;
                    let label = format!(
                        "LZ4_compressHC2_limitedOutput_continue shape={} level={} block={}",
                        shape_name(shape),
                        level,
                        bi
                    );
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = (a.d_hc2_lim_cont.0)(
                        cdata,
                        sp,
                        cd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        bound as c_int,
                        level,
                    );
                    let rn = (a.d_hc2_lim_cont.1)(
                        rdata,
                        sp,
                        rd.as_mut_ptr() as *mut c_char,
                        bsz as c_int,
                        bound as c_int,
                        level,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert!(cn > 0, "{}: failed", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(&label, cdata, rdata);
                    let block = &src[off..off + bsz];
                    assert_roundtrip_dict(&label, &cd[..cn as usize], block, &hist);
                    hist.extend_from_slice(block);
                    trim_hist(&mut hist);
                    off += bsz;
                }
                assert_eq!((a.d_free_hc.0)(cdata), (a.d_free_hc.1)(rdata), "LZ4_freeHC");
            }
        }
        // LZ4_freeHC(NULL) is explicitly supported.
        assert_eq!(
            (a.d_free_hc.0)(std::ptr::null_mut()),
            (a.d_free_hc.1)(std::ptr::null_mut()),
            "LZ4_freeHC(NULL)"
        );
        assert_eq!((a.d_free_hc.0)(std::ptr::null_mut()), 0);

        // ---- LZ4_resetStreamStateHC ----
        let src = src_buf(&mut rng, 5, 30000);
        let mut cst = AlignedBuf::new(ssz, 64);
        let mut rst = AlignedBuf::new(ssz, 64);
        let cr = (a.d_reset_stream_state.0)(
            cst.as_mut_ptr() as *mut c_void,
            src.as_ptr() as *mut c_char,
        );
        let rr = (a.d_reset_stream_state.1)(
            rst.as_mut_ptr() as *mut c_void,
            src.as_ptr() as *mut c_char,
        );
        assert_eq!(cr, rr, "LZ4_resetStreamStateHC return");
        assert_eq!(cr, 0, "LZ4_resetStreamStateHC on a valid state");
        assert_state_blob_eq(
            "LZ4_resetStreamStateHC",
            cst.as_ptr() as *const c_void,
            rst.as_ptr() as *const c_void,
        );
        // ... and the state is usable afterwards via the legacy wrapper.
        {
            let bound = bound_of(src.len());
            let mut cd = vec![0xAAu8; bound + 32];
            let mut rd = vec![0xAAu8; bound + 32];
            let cn = (a.d_hc2_cont.0)(
                cst.as_mut_ptr() as *mut c_void,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                9,
            );
            let rn = (a.d_hc2_cont.1)(
                rst.as_mut_ptr() as *mut c_void,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                9,
            );
            assert_eq!(cn, rn, "resetStreamStateHC + compressHC2_continue");
            assert!(cn > 0);
            assert_bytes_eq("resetStreamStateHC + continue dst", &cd, &rd);
            assert_state_blob_eq(
                "resetStreamStateHC + continue state",
                cst.as_ptr() as *const c_void,
                rst.as_ptr() as *const c_void,
            );
            assert_roundtrip("resetStreamStateHC + continue", &cd[..cn as usize], &src);
        }
        // Misaligned state -> LZ4_initStreamHC fails -> returns 1.
        for &off in &[1usize, 3, 7] {
            let mut mc = AlignedBuf::with_offset(ssz, 8, off);
            let mut mr = AlignedBuf::with_offset(ssz, 8, off);
            let cr = (a.d_reset_stream_state.0)(
                mc.as_mut_ptr() as *mut c_void,
                src.as_ptr() as *mut c_char,
            );
            let rr = (a.d_reset_stream_state.1)(
                mr.as_mut_ptr() as *mut c_void,
                src.as_ptr() as *mut c_char,
            );
            assert_eq!(cr, rr, "LZ4_resetStreamStateHC misaligned(+{})", off);
            assert_eq!(cr, 1, "LZ4_resetStreamStateHC misaligned(+{}) must fail", off);
        }
    }
}

// ===========================================================================
// Error paths
// ===========================================================================

/// `dstCapacity` too small: 0, 1, and every size up to just past the minimum
/// needed. The limitedOutput path must return 0 identically, and whatever
/// partial output was produced before the overflow was detected must be
/// byte-identical too.
#[test]
fn hc_error_dst_capacity_too_small() {
    let a = api();
    let mut rng = Rng::new(0x0F10_FA11);
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 13, 100, 700, 1000] {
            let src = src_buf(&mut rng, shape, len);
            let full = diff_compress_hc(&src, 9, &format!("min-size probe len={}", len));
            let m = full.len();
            for &level in &[c_int::MIN, 1i32, 2, 3, 9, 10, 11, 12, 100] {
                for cap in 0..=(m + 2) {
                    let label = format!(
                        "dstCapacity shape={} len={} level={} cap={}",
                        shape_name(shape),
                        len,
                        level,
                        cap
                    );
                    let mut cd = vec![0xAAu8; cap + 32];
                    let mut rd = vec![0xAAu8; cap + 32];
                    let cn = unsafe {
                        (a.compress_hc.0)(
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.compress_hc.1)(
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            level,
                        )
                    };
                    assert_eq!(cn, rn, "{}: return value", label);
                    assert_bytes_eq(&format!("{}: dst (incl. partial output)", label), &cd, &rd);
                    if cn > 0 {
                        assert!(cn as usize <= cap, "{}: wrote past dstCapacity", label);
                        assert_roundtrip(&label, &cd[..cn as usize], &src);
                    }
                }
            }
        }
    }

    // dstCapacity == 0 on a large input, for every level.
    let src = src_buf(&mut rng, 4, 100_000);
    for &level in &all_levels() {
        let mut cd = vec![0xAAu8; 32];
        let mut rd = vec![0xAAu8; 32];
        let cn = unsafe {
            (a.compress_hc.0)(
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                0,
                level,
            )
        };
        let rn = unsafe {
            (a.compress_hc.1)(
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                0,
                level,
            )
        };
        assert_eq!(cn, rn, "LZ4_compress_HC(dstCapacity=0, level={})", level);
        assert_eq!(cn, 0, "dstCapacity 0 must fail (level={})", level);
        assert_bytes_eq("dstCapacity 0 dst untouched", &cd, &rd);
    }
}

/// `LZ4HC_compress_generic_internal` rejects `(U32)srcSize > LZ4_MAX_INPUT_SIZE`
/// *before* touching either buffer (lz4hc.c:1390), and because the comparison is
/// unsigned, negative sizes are rejected by the same test. That makes it safe to
/// probe oversized/negative `srcSize` with tiny buffers — no 2 GB allocation and
/// no out-of-bounds access.
#[test]
fn hc_error_srcsize_out_of_range() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut cst = AlignedBuf::new(ssz, 64);
    let mut rst = AlignedBuf::new(ssz, 64);
    let mut rng = Rng::new(0xBAD_512E);
    let src = src_buf(&mut rng, 3, 4096);

    let bad_sizes: Vec<c_int> = vec![
        c_int::MIN,
        -1_000_000,
        -2,
        -1,
        LZ4_MAX_INPUT_SIZE as c_int + 1,
        0x7EFF_FFFF,
        0x7FFF_FFFE,
        c_int::MAX,
    ];

    unsafe {
        for &n in &bad_sizes {
            for &level in &[c_int::MIN, 1, 2, 9, 10, 12, c_int::MAX] {
                // LZ4_compress_HC: compressBound(n)==0 selects notLimited, then
                // the size check bails out with 0.
                let mut cd = vec![0xAAu8; 64];
                let mut rd = vec![0xAAu8; 64];
                let cn = (a.compress_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                    level,
                );
                let rn = (a.compress_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                    level,
                );
                assert_eq!(cn, rn, "LZ4_compress_HC(srcSize={}, level={})", n, level);
                assert_eq!(cn, 0, "srcSize={} must be rejected", n);
                assert_bytes_eq("oversized srcSize dst untouched", &cd, &rd);

                // extStateHC
                let cn = (a.ext_state.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                    level,
                );
                let rn = (a.ext_state.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                    level,
                );
                assert_eq!(cn, rn, "extStateHC(srcSize={})", n);
                assert_eq!(cn, 0);
                assert_state_blob_eq(
                    &format!("extStateHC(srcSize={}) state", n),
                    cst.as_ptr() as *const c_void,
                    rst.as_ptr() as *const c_void,
                );

                // destSize: *srcSizePtr must be left identical too.
                let mut c_ssz = n;
                let mut r_ssz = n;
                let cn = (a.dest_size.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut c_ssz,
                    16,
                    level,
                );
                let rn = (a.dest_size.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut r_ssz,
                    16,
                    level,
                );
                assert_eq!(cn, rn, "destSize(srcSize={})", n);
                assert_eq!(c_ssz, r_ssz, "destSize(srcSize={}) *srcSizePtr", n);
                assert_eq!(cn, 0);

                // deprecated wrappers funnel into the same check
                let cn = (a.d_hc2.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    level,
                );
                let rn = (a.d_hc2.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    level,
                );
                assert_eq!(cn, rn, "LZ4_compressHC2(srcSize={})", n);
                assert_eq!(cn, 0);
            }
        }

        // Streaming variant: same check, and the stream must end up dirty in
        // both libraries.
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for &n in &bad_sizes {
            for &level in &[2i32, 9, 12] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let mut cd = vec![0xAAu8; 64];
                let mut rd = vec![0xAAu8; 64];
                let cn = (a.cont.0)(
                    cs,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                );
                let rn = (a.cont.1)(
                    rs,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    16,
                );
                assert_eq!(cn, rn, "continue(srcSize={}, level={})", n, level);
                assert_eq!(cn, 0);
                assert_bytes_eq("continue oversized dst untouched", &cd, &rd);
                assert_state_blob_eq(&format!("continue(srcSize={}) state", n), cs, rs);
            }
        }
        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}

// ===========================================================================
// Randomized property loop (fixed seed)
// ===========================================================================

#[test]
fn hc_randomized_property_loop() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    let mut cst = AlignedBuf::new(ssz, 64);
    let mut rst = AlignedBuf::new(ssz, 64);
    let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
    let levels = all_levels();

    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();

        for iter in 0..900 {
            let shape = rng.below(N_SHAPES);
            let len = match rng.below(5) {
                0 => rng.range(0, 32),
                1 => rng.range(0, 1024),
                2 => rng.range(60000, 70000), // straddle LZ4_64Klimit
                3 => rng.range(0, 20000),
                _ => rng.range(0, 90000),
            };
            let level = levels[rng.below(levels.len())];
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            let which = rng.below(4);
            let label = format!(
                "random iter={} which={} shape={} len={} level={}",
                iter,
                which,
                shape_name(shape),
                len,
                level
            );

            match which {
                0 => {
                    // one-shot, random capacity around the true size
                    let cap = match rng.below(3) {
                        0 => bound,
                        1 => rng.range(0, bound),
                        _ => rng.range(0, 64),
                    };
                    let mut cd = vec![0xAAu8; cap + 32];
                    let mut rd = vec![0xAAu8; cap + 32];
                    let cn = (a.compress_hc.0)(
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                        level,
                    );
                    let rn = (a.compress_hc.1)(
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                        level,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    if cn > 0 {
                        assert_roundtrip(&label, &cd[..cn as usize], &src);
                    }
                }
                1 => {
                    // extState / fastReset, alternating, state carried forward
                    let f = if rng.bool() {
                        (a.ext_state.0, a.ext_state.1)
                    } else {
                        (a.ext_state_fast.0, a.ext_state_fast.1)
                    };
                    let mut cd = vec![0xAAu8; bound + 32];
                    let mut rd = vec![0xAAu8; bound + 32];
                    let cn = f.0(
                        cst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        level,
                    );
                    let rn = f.1(
                        rst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        level,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(
                        &label,
                        cst.as_ptr() as *const c_void,
                        rst.as_ptr() as *const c_void,
                    );
                    if cn > 0 {
                        assert_roundtrip(&label, &cd[..cn as usize], &src);
                    }
                }
                2 => {
                    // destSize with a random budget
                    let target = match rng.below(3) {
                        0 => rng.range(0, 32),
                        1 => rng.range(0, bound),
                        _ => bound,
                    };
                    let mut cd = vec![0xAAu8; target + 64];
                    let mut rd = vec![0xAAu8; target + 64];
                    let mut c_ssz = len as c_int;
                    let mut r_ssz = len as c_int;
                    let cn = (a.dest_size.0)(
                        cst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        &mut c_ssz,
                        target as c_int,
                        level,
                    );
                    let rn = (a.dest_size.1)(
                        rst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        &mut r_ssz,
                        target as c_int,
                        level,
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_eq!(c_ssz, r_ssz, "{}: *srcSizePtr", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    assert_state_blob_eq(
                        &label,
                        cst.as_ptr() as *const c_void,
                        rst.as_ptr() as *const c_void,
                    );
                    if cn > 0 {
                        assert_roundtrip(&label, &cd[..cn as usize], &src[..c_ssz as usize]);
                    }
                }
                _ => {
                    // streaming: dict + a couple of blocks, random favor flag
                    let dsz = rng.range(0, 70000);
                    let dshape = rng.below(N_SHAPES);
                    let dict = src_buf(&mut rng, dshape, dsz);
                    (a.reset_stream.0)(cs, level);
                    (a.reset_stream.1)(rs, level);
                    let fav = rng.below(2) as c_int;
                    (a.favor.0)(cs, fav);
                    (a.favor.1)(rs, fav);
                    let cl = (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, dsz as c_int);
                    let rl = (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, dsz as c_int);
                    assert_eq!(cl, rl, "{}: loadDictHC", label);
                    assert_state_blob_eq(&format!("{} after loadDictHC", label), cs, rs);

                    let mut hist: Vec<u8> = dict[dict.len() - cl as usize..].to_vec();
                    let nblocks = rng.range(1, 4);
                    let mut off = 0usize;
                    for bi in 0..nblocks {
                        let remaining = src.len() - off;
                        let bsz = if remaining == 0 { 0 } else { rng.range(0, remaining) };
                        let bound = bound_of(bsz);
                        let mut cd = vec![0xAAu8; bound + 32];
                        let mut rd = vec![0xAAu8; bound + 32];
                        let sp = src.as_ptr().add(off) as *const c_char;
                        let cn = (a.cont.0)(
                            cs,
                            sp,
                            cd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            sp,
                            rd.as_mut_ptr() as *mut c_char,
                            bsz as c_int,
                            bound as c_int,
                        );
                        let blabel = format!("{} block={} size={}", label, bi, bsz);
                        assert_eq!(cn, rn, "{}: return", blabel);
                        assert!(cn > 0, "{}: failed", blabel);
                        assert_bytes_eq(&format!("{}: dst", blabel), &cd, &rd);
                        assert_state_blob_eq(&blabel, cs, rs);
                        let block = &src[off..off + bsz];
                        assert_roundtrip_dict(&blabel, &cd[..cn as usize], block, &hist);
                        hist.extend_from_slice(block);
                        trim_hist(&mut hist);
                        off += bsz;
                    }
                }
            }
        }

        (a.free_stream.0)(cs);
        (a.free_stream.1)(rs);
    }
}
