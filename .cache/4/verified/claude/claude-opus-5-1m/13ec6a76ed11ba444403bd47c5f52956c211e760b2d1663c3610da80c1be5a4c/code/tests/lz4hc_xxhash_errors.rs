//! Phase C — error-path differential tests for `lz4hc.c` (ERRORS.md rows
//! 94-147) and `xxhash.c` (ERRORS.md rows 148-167).
//!
//! One `#[test]` (or one clearly-labelled block inside a grouped test) per row.
//! Every case constructs the exact invalid input / condition described by the
//! row, calls BOTH the C `.so` and the Rust `.so`, and asserts they return the
//! SAME error code / sentinel — exactly `0`, exactly `1`, exactly `NULL`,
//! exactly `XXH_ERROR`, or the exact clamped value for the silent-clamp rows.
//!
//! Facts about this build that drive the test design:
//!   * ASSERT LIVENESS IS PER TRANSLATION UNIT — verified mechanically with
//!     `nm -u` on each object file in `c_src/build/CMakeFiles/lz4.dir/src/`
//!     (a reference to `__assert_fail` means live asserts):
//!
//!         lz4.c       -> no    lz4frame.c -> no    lz4hc.c -> no
//!         lz4file.c   -> YES   xxhash.c   -> YES
//!
//!     `-DNDEBUG` is indeed absent, but that is NOT what decides it: `lz4.c`
//!     (lines 268-274), `lz4frame.c` (143-149) and, through them, `lz4hc.c`
//!     define their own `#define assert(condition) ((void)0)` whenever
//!     `LZ4_DEBUG` is undefined — and `c_src/CMakeLists.txt` never defines it.
//!     `lz4file.c` (line 36) and `xxhash.c` (line 114) include `<assert.h>`
//!     UNCONDITIONALLY, so only those two have live asserts.
//!
//!     Consequence for the rows below: an `assert`-guarded trigger in **lz4hc.c**
//!     does NOT abort — the assert is compiled out and execution continues into
//!     undefined behaviour (an out-of-bounds read/write, or a wrapped size_t).
//!     That is still not a comparable behaviour, so such rows remain documented
//!     rather than executed, but the REASON is "assert compiled out => UB in this
//!     build", not "a live assert aborts the C". Rows 118, 137, 141 and 147 below
//!     are annotated accordingly. In **xxhash.c** the asserts really are live.
//!   * `LZ4HC_HEAPMODE` is 1 (lz4hc.c:47-49; the CMake file only sets
//!     `LZ4_HEAPMODE=0`/`LZ4F_HEAPMODE=0`), so `LZ4_compress_HC` and
//!     `LZ4HC_compress_optimal` heap-allocate.
//!   * `LZ4_ALIGN_TEST` is 1 (lz4.c:185-187) and `LZ4_streamHC_t`'s alignment is
//!     8 (it contains pointers), so the alignment rejections are live.
//!   * `XXH_ACCEPT_NULL_INPUT_POINTER` is 0 (xxhash.c:70-72).
//!
//! HARNESS RULE observed throughout: the C dst buffer and the Rust dst buffer
//! are pre-filled with the SAME sentinel byte (0xAA) and the FULL buffer is
//! compared, so a divergence in untouched bytes cannot hide.
//!
//! A complete row-by-row coverage map is at the END of this file.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

const SENTINEL: u8 = 0xAA;

/// `#define LZ4_minLength (MFLIMIT+1)` — inputs below this skip the main loop
/// and go straight to the "encode last literals" tail in every strategy.
const LZ4_MIN_LENGTH: usize = MFLIMIT + 1; // 13

// ===========================================================================
// Signature aliases (verbatim from lz4hc.h / lz4hc.c / xxhash.h)
// ===========================================================================

type FnSizeof = unsafe extern "C" fn() -> c_int;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
/// `int LZ4_compress_HC(const char*, char*, int, int, int)`
type FnCompressHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_extStateHC(void*, const char*, char*, int, int, int)`
type FnExtState =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_destSize(void*, const char*, char*, int*, int, int)`
type FnDestSize = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    *mut c_int,
    c_int,
    c_int,
) -> c_int;
/// `LZ4_streamHC_t* LZ4_createStreamHC(void)`
type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
/// `int LZ4_freeStreamHC(LZ4_streamHC_t*)`
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
/// `LZ4_streamHC_t* LZ4_initStreamHC(void*, size_t)`
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
/// `void LZ4_resetStreamHC(LZ4_streamHC_t*, int)` / `_fast` /
/// `LZ4_setCompressionLevel`
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
/// `int LZ4_loadDictHC(LZ4_streamHC_t*, const char*, int)`
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
/// `int LZ4_saveDictHC(LZ4_streamHC_t*, char*, int)`
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
/// `void LZ4_attach_HC_dictionary(LZ4_streamHC_t*, const LZ4_streamHC_t*)`
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
/// `int LZ4_compress_HC_continue(LZ4_streamHC_t*, const char*, char*, int, int)`
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compress_HC_continue_destSize(LZ4_streamHC_t*, const char*, char*, int*, int)`
type FnContinueDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
/// `int LZ4_compressHC(const char*, char*, int)`
type FnDep3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `int LZ4_compressHC_limitedOutput(const char*, char*, int, int)` /
/// `int LZ4_compressHC2(const char*, char*, int, int)`
type FnDep4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compressHC2_limitedOutput(const char*, char*, int, int, int)`
type FnDep5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_compressHC2_continue(void*, const char*, char*, int, int)`
type FnDepS5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_compressHC2_limitedOutput_continue(void*, const char*, char*, int, int, int)`
type FnDepS6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `void* LZ4_createHC(const char*)`
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
/// `int LZ4_freeHC(void*)`
type FnFreeHC = unsafe extern "C" fn(*mut c_void) -> c_int;
/// `int LZ4_resetStreamStateHC(void*, char*)`
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
// xxhash
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;
type FnXXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnReset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;

// ===========================================================================
// Resolved-symbol table
// ===========================================================================

struct Api {
    bound: (FnBound, FnBound),
    sizeof_state: (FnSizeof, FnSizeof),
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
    load_dict: (FnLoadDict, FnLoadDict),
    save_dict: (FnSaveDict, FnSaveDict),
    attach: (FnAttach, FnAttach),
    cont: (FnContinue, FnContinue),
    cont_dest_size: (FnContinueDestSize, FnContinueDestSize),
    d_hc: (FnDep3, FnDep3),
    d_hc_lim: (FnDep4, FnDep4),
    d_hc2: (FnDep4, FnDep4),
    d_hc2_lim: (FnDep5, FnDep5),
    d_hc2_cont: (FnDepS5, FnDepS5),
    d_hc2_lim_cont: (FnDepS6, FnDepS6),
    d_create_hc: (FnCreateHC, FnCreateHC),
    d_free_hc: (FnFreeHC, FnFreeHC),
    d_reset_stream_state: (FnResetStreamState, FnResetStreamState),
}

fn api() -> &'static Api {
    static A: std::sync::OnceLock<Api> = std::sync::OnceLock::new();
    A.get_or_init(|| Api {
        bound: both("LZ4_compressBound"),
        sizeof_state: both("LZ4_sizeofStateHC"),
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
        load_dict: both("LZ4_loadDictHC"),
        save_dict: both("LZ4_saveDictHC"),
        attach: both("LZ4_attach_HC_dictionary"),
        cont: both("LZ4_compress_HC_continue"),
        cont_dest_size: both("LZ4_compress_HC_continue_destSize"),
        d_hc: both("LZ4_compressHC"),
        d_hc_lim: both("LZ4_compressHC_limitedOutput"),
        d_hc2: both("LZ4_compressHC2"),
        d_hc2_lim: both("LZ4_compressHC2_limitedOutput"),
        d_hc2_cont: both("LZ4_compressHC2_continue"),
        d_hc2_lim_cont: both("LZ4_compressHC2_limitedOutput_continue"),
        d_create_hc: both("LZ4_createHC"),
        d_free_hc: both("LZ4_freeHC"),
        d_reset_stream_state: both("LZ4_resetStreamStateHC"),
    })
}

// ===========================================================================
// LZ4HC_CCtx_internal mirror — lets a test read the exact field a "silent"
// row mutates (compressionLevel, dirty, dictLimit, dictCtx, ...).
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

/// Scalar view with pointers reduced to offsets relative to `prefixStart`, so
/// two contexts that reference the SAME input buffers but live at different
/// addresses still compare equal.
#[derive(Debug, PartialEq, Eq)]
struct HcView {
    end_off: isize,
    dict_start_off: isize,
    prefix_null: bool,
    end_null: bool,
    dict_limit: u32,
    low_limit: u32,
    next_to_update: u32,
    compression_level: i16,
    favor_dec_speed: i8,
    dirty: i8,
    dict_ctx_null: bool,
}

unsafe fn view(p: *const c_void) -> HcView {
    let c = p as *const HcCtx;
    let prefix = (*c).prefix_start;
    HcView {
        end_off: ((*c).end as isize) - (prefix as isize),
        dict_start_off: ((*c).dict_start as isize) - (prefix as isize),
        prefix_null: prefix.is_null(),
        end_null: (*c).end.is_null(),
        dict_limit: (*c).dict_limit,
        low_limit: (*c).low_limit,
        next_to_update: (*c).next_to_update,
        compression_level: (*c).compression_level,
        favor_dec_speed: (*c).favor_dec_speed,
        dirty: (*c).dirty,
        dict_ctx_null: (*c).dict_ctx.is_null(),
    }
}

unsafe fn level_of(p: *const c_void) -> i16 {
    (*(p as *const HcCtx)).compression_level
}

unsafe fn dirty_of(p: *const c_void) -> i8 {
    (*(p as *const HcCtx)).dirty
}

/// Byte-exact comparison of the two big index tables plus the scalar view.
unsafe fn assert_state_eq(label: &str, cp: *const c_void, rp: *const c_void) {
    let ct = std::slice::from_raw_parts(cp as *const u8, HC_TABLES_BYTES);
    let rt = std::slice::from_raw_parts(rp as *const u8, HC_TABLES_BYTES);
    assert_bytes_eq(&format!("{}: hashTable+chainTable", label), ct, rt);
    assert_eq!(view(cp), view(rp), "{}: LZ4HC_CCtx_internal scalars", label);
}

/// Strict byte comparison of the whole context blob — valid whenever both
/// libraries were handed the SAME src/dict pointers.
unsafe fn assert_state_blob_eq(label: &str, cp: *const c_void, rp: *const c_void) {
    let ct = std::slice::from_raw_parts(cp as *const u8, HC_CCTX_BYTES);
    let rt = std::slice::from_raw_parts(rp as *const u8, HC_CCTX_BYTES);
    if ct != rt {
        assert_eq!(view(cp), view(rp), "{}: state scalars", label);
        assert_bytes_eq(&format!("{}: raw state blob", label), ct, rt);
    }
}

// ===========================================================================
// Small helpers
// ===========================================================================

fn bound_of(len: usize) -> usize {
    let (c, r) = api().bound;
    let cb = unsafe { c(len as c_int) };
    assert_eq!(cb, unsafe { r(len as c_int) }, "LZ4_compressBound({})", len);
    cb.max(1) as usize
}

/// The exact expected clamp performed by `LZ4HC_getCLevelParams`
/// (lz4hc.c:110-115) and by `LZ4_setCompressionLevel` (lz4hc.c:1613-1616):
/// `< 1` becomes `LZ4HC_CLEVEL_DEFAULT` (9), `> 12` becomes 12.
fn clamped_level(l: c_int) -> c_int {
    if l < 1 {
        LZ4HC_CLEVEL_DEFAULT
    } else if l > LZ4HC_CLEVEL_MAX {
        LZ4HC_CLEVEL_MAX
    } else {
        l
    }
}

fn src_buf(rng: &mut Rng, shape: usize, len: usize) -> Vec<u8> {
    let mut v = gen_shape(rng, shape, len);
    v.reserve(64);
    v
}

/// A fresh, correctly aligned `LZ4_streamHC_t`-sized scratch block.
fn state_buf() -> AlignedBuf {
    AlignedBuf::new(HC_STREAM_BYTES, 64)
}

/// Run `LZ4_compress_HC` on both libraries and return the C output, asserting
/// the two agree on the return value and on the FULL sentinel-filled buffer.
fn diff_compress_hc(src: &[u8], cap: usize, level: c_int, label: &str) -> Vec<u8> {
    let a = api();
    let mut cd = vec![SENTINEL; cap + 32];
    let mut rd = vec![SENTINEL; cap + 32];
    let (cn, rn) = unsafe {
        (
            (a.compress_hc.0)(
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                level,
            ),
            (a.compress_hc.1)(
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                level,
            ),
        )
    };
    assert_eq!(cn, rn, "{}: LZ4_compress_HC return", label);
    assert_bytes_eq(&format!("{}: LZ4_compress_HC dst", label), &cd, &rd);
    cd.truncate(cn.max(0) as usize);
    cd
}

// ===========================================================================
// ERRORS.md rows 94, 95 — HC compressionLevel is silently CLAMPED, never
// rejected: `< 1` -> LZ4HC_CLEVEL_DEFAULT (9), `> 12` -> 12
// (`LZ4HC_getCLevelParams`, lz4hc.c:110-115).
//
// Asserted as a CLAMP: the output for an out-of-range level must be
// byte-identical to the output for the exact clamp target, in BOTH libraries.
// ===========================================================================

#[test]
fn row_94_95_hc_level_silently_clamped_to_9_and_12() {
    let a = api();
    let mut rng = Rng::new(0x94_95_C1A3);

    let below: [c_int; 4] = [c_int::MIN, -1000, -1, 0];
    let above: [c_int; 4] = [13, 100, 0x7FFF_FFFE, c_int::MAX];

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 12, 13, 1000, 9000] {
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            let label = |l: c_int| {
                format!(
                    "rows 94/95 shape={} len={} level={}",
                    shape_name(shape),
                    len,
                    l
                )
            };

            // Reference outputs at the two clamp targets.
            let ref9 = diff_compress_hc(&src, bound, LZ4HC_CLEVEL_DEFAULT, &label(9));
            let ref12 = diff_compress_hc(&src, bound, LZ4HC_CLEVEL_MAX, &label(12));

            // row 94: any level < 1 must behave exactly like level 9.
            for &l in &below {
                let got = diff_compress_hc(&src, bound, l, &label(l));
                assert_bytes_eq(
                    &format!("row 94: level {} must equal level 9 output", l),
                    &ref9,
                    &got,
                );
            }
            // row 95: any level > 12 must behave exactly like level 12.
            for &l in &above {
                let got = diff_compress_hc(&src, bound, l, &label(l));
                assert_bytes_eq(
                    &format!("row 95: level {} must equal level 12 output", l),
                    &ref12,
                    &got,
                );
            }

            // Same clamp through LZ4_compress_HC_extStateHC (rows 94/95 list it
            // explicitly as an affected entry point).
            let mut cst = state_buf();
            let mut rst = state_buf();
            let mut ext = |l: c_int| -> Vec<u8> {
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let (cn, rn) = unsafe {
                    (
                        (a.ext_state.0)(
                            cst.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            l,
                        ),
                        (a.ext_state.1)(
                            rst.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            l,
                        ),
                    )
                };
                assert_eq!(cn, rn, "rows 94/95 extStateHC level={}", l);
                assert_bytes_eq(&format!("rows 94/95 extStateHC dst level={}", l), &cd, &rd);
                // The clamp is also observable in the stored level field.
                assert_eq!(
                    unsafe { level_of(cst.as_ptr() as *const c_void) },
                    clamped_level(l) as i16,
                    "rows 94/95: extStateHC stored level for {}",
                    l
                );
                assert_eq!(
                    unsafe { level_of(cst.as_ptr() as *const c_void) },
                    unsafe { level_of(rst.as_ptr() as *const c_void) },
                    "rows 94/95: extStateHC stored level C vs Rust for {}",
                    l
                );
                cd.truncate(cn.max(0) as usize);
                cd
            };
            let e9 = ext(LZ4HC_CLEVEL_DEFAULT);
            let e12 = ext(LZ4HC_CLEVEL_MAX);
            for &l in &below {
                assert_bytes_eq("row 94: extStateHC clamp to 9", &e9, &ext(l));
            }
            for &l in &above {
                assert_bytes_eq("row 95: extStateHC clamp to 12", &e12, &ext(l));
            }
        }
    }
}

#[test]
fn row_94_95_deprecated_wrappers_hardcode_level_zero() {
    // `LZ4_compressHC` / `LZ4_compressHC_limitedOutput` hard-code
    // compressionLevel 0 (lz4hc.c:2133-2134), which row 94 says is clamped to
    // LZ4HC_CLEVEL_DEFAULT. `LZ4_compressHC2*` forward the caller's level.
    let a = api();
    let mut rng = Rng::new(0x94_95_DEE0);

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 13, 1000, 9000] {
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            let ref9 = diff_compress_hc(
                &src,
                bound,
                LZ4HC_CLEVEL_DEFAULT,
                &format!("dep ref shape={} len={}", shape_name(shape), len),
            );
            let ref12 = diff_compress_hc(&src, bound, LZ4HC_CLEVEL_MAX, "dep ref 12");

            unsafe {
                // LZ4_compressHC(src, dst, srcSize) — capacity == compressBound
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.d_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                let rn = (a.d_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    len as c_int,
                );
                assert_eq!(cn, rn, "row 94: LZ4_compressHC return");
                assert_bytes_eq("row 94: LZ4_compressHC dst", &cd, &rd);
                assert_bytes_eq("row 94: LZ4_compressHC == level 9", &ref9, &cd[..cn as usize]);

                // LZ4_compressHC_limitedOutput(src, dst, srcSize, maxDstSize)
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.d_hc_lim.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                let rn = (a.d_hc_lim.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    bound as c_int,
                );
                assert_eq!(cn, rn, "row 94: LZ4_compressHC_limitedOutput return");
                assert_bytes_eq("row 94: LZ4_compressHC_limitedOutput dst", &cd, &rd);
                assert_bytes_eq(
                    "row 94: LZ4_compressHC_limitedOutput == level 9",
                    &ref9,
                    &cd[..cn as usize],
                );

                // LZ4_compressHC2 / LZ4_compressHC2_limitedOutput with out-of-range levels
                for &l in &[c_int::MIN, -1, 0, 13, c_int::MAX] {
                    let want: &[u8] = if l < 1 { &ref9 } else { &ref12 };

                    let mut cd = vec![SENTINEL; bound + 32];
                    let mut rd = vec![SENTINEL; bound + 32];
                    let cn = (a.d_hc2.0)(
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        l,
                    );
                    let rn = (a.d_hc2.1)(
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        l,
                    );
                    assert_eq!(cn, rn, "rows 94/95: LZ4_compressHC2({})", l);
                    assert_bytes_eq(&format!("rows 94/95: LZ4_compressHC2({}) dst", l), &cd, &rd);
                    assert_bytes_eq(
                        &format!("rows 94/95: LZ4_compressHC2({}) clamped output", l),
                        want,
                        &cd[..cn as usize],
                    );

                    let mut cd = vec![SENTINEL; bound + 32];
                    let mut rd = vec![SENTINEL; bound + 32];
                    let cn = (a.d_hc2_lim.0)(
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        l,
                    );
                    let rn = (a.d_hc2_lim.1)(
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        l,
                    );
                    assert_eq!(cn, rn, "rows 94/95: LZ4_compressHC2_limitedOutput({})", l);
                    assert_bytes_eq(
                        &format!("rows 94/95: LZ4_compressHC2_limitedOutput({}) dst", l),
                        &cd,
                        &rd,
                    );
                    assert_bytes_eq(
                        &format!(
                            "rows 94/95: LZ4_compressHC2_limitedOutput({}) clamped output",
                            l
                        ),
                        want,
                        &cd[..cn as usize],
                    );
                }
            }
        }
    }
}

#[test]
fn row_94_95_destsize_level_clamp() {
    // `LZ4_compress_HC_destSize` is listed by rows 94/95 too. It uses the
    // `fillOutput` directive, so its output differs from LZ4_compress_HC's;
    // the clamp is therefore asserted against destSize's OWN level-9 / level-12
    // output, plus the stored `compressionLevel` field which
    // `LZ4_setCompressionLevel` (lz4hc.c:1541) writes.
    let a = api();
    let mut rng = Rng::new(0x94_95_D5);
    let src = src_buf(&mut rng, 4, 20000);

    let run = |l: c_int, target: usize| -> (c_int, c_int, Vec<u8>) {
        let mut cst = state_buf();
        let mut rst = state_buf();
        let mut cd = vec![SENTINEL; target + 32];
        let mut rd = vec![SENTINEL; target + 32];
        let mut css = src.len() as c_int;
        let mut rss = src.len() as c_int;
        let (cn, rn) = unsafe {
            (
                (a.dest_size.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut css,
                    target as c_int,
                    l,
                ),
                (a.dest_size.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut rss,
                    target as c_int,
                    l,
                ),
            )
        };
        assert_eq!(cn, rn, "rows 94/95 destSize level={} return", l);
        assert_eq!(css, rss, "rows 94/95 destSize level={} *srcSizePtr", l);
        assert_bytes_eq(&format!("rows 94/95 destSize level={} dst", l), &cd, &rd);
        assert_eq!(
            unsafe { level_of(cst.as_ptr() as *const c_void) },
            clamped_level(l) as i16,
            "rows 94/95: destSize stored level for {}",
            l
        );
        assert_eq!(
            unsafe { level_of(cst.as_ptr() as *const c_void) },
            unsafe { level_of(rst.as_ptr() as *const c_void) },
            "rows 94/95: destSize stored level C vs Rust for {}",
            l
        );
        cd.truncate(cn.max(0) as usize);
        (cn, css, cd)
    };

    for &target in &[100usize, 1000, 8000] {
        let r9 = run(LZ4HC_CLEVEL_DEFAULT, target);
        let r12 = run(LZ4HC_CLEVEL_MAX, target);
        for &l in &[c_int::MIN, -1, 0] {
            let got = run(l, target);
            assert_eq!(got.0, r9.0, "row 94: destSize({}) return == level 9", l);
            assert_eq!(got.1, r9.1, "row 94: destSize({}) consumed == level 9", l);
            assert_bytes_eq("row 94: destSize clamp to 9", &r9.2, &got.2);
        }
        for &l in &[13, 100, c_int::MAX] {
            let got = run(l, target);
            assert_eq!(got.0, r12.0, "row 95: destSize({}) return == level 12", l);
            assert_eq!(got.1, r12.1, "row 95: destSize({}) consumed == level 12", l);
            assert_bytes_eq("row 95: destSize clamp to 12", &r12.2, &got.2);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 96, 97, 98 — LZ4_setCompressionLevel / LZ4_resetStreamHC /
// LZ4_resetStreamHC_fast are `void`; the out-of-range level is silently
// replaced (9 if < 1, 12 if > 12) in `internal_donotuse.compressionLevel`
// (lz4hc.c:1613-1616, reached from 1592 and 1608).
//
// The stored field is read back through the state mirror, so the exact clamped
// VALUE is asserted, not just C==Rust.
//
// (`tests/lz4hc_diff.rs::hc_set_compression_level_out_of_range` exercises the
// same three functions from the compression-output side.)
// ===========================================================================

#[test]
fn row_96_97_98_stored_level_clamped() {
    let a = api();
    let levels: Vec<c_int> = vec![
        c_int::MIN,
        -1_000_000,
        -2,
        -1,
        0, // rows 96 / 98 lower half
        1,
        2,
        3,
        6,
        9,
        10,
        11,
        12, // in range: stored verbatim
        13,
        14,
        100,
        0x7FFF_FFFE,
        c_int::MAX, // rows 97 / 98 upper half
    ];
    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        assert!(!cs.is_null() && !rs.is_null(), "LZ4_createStreamHC");

        for &l in &levels {
            let want = clamped_level(l) as i16;

            // row 96/97 — LZ4_setCompressionLevel
            (a.set_level.0)(cs, l);
            (a.set_level.1)(rs, l);
            assert_eq!(
                level_of(cs),
                want,
                "row 96/97: LZ4_setCompressionLevel({}) stored {}",
                l,
                level_of(cs)
            );
            assert_eq!(
                level_of(cs),
                level_of(rs),
                "row 96/97: LZ4_setCompressionLevel({}) C vs Rust",
                l
            );

            // row 98 — LZ4_resetStreamHC (full init + setCompressionLevel)
            (a.reset_stream.0)(cs, l);
            (a.reset_stream.1)(rs, l);
            assert_eq!(
                level_of(cs),
                want,
                "row 98: LZ4_resetStreamHC({}) stored {}",
                l,
                level_of(cs)
            );
            assert_eq!(
                level_of(cs),
                level_of(rs),
                "row 98: LZ4_resetStreamHC({}) C vs Rust",
                l
            );
            assert_state_blob_eq(&format!("row 98: resetStreamHC({})", l), cs, rs);

            // row 98 — LZ4_resetStreamHC_fast (same clamp via line 1608)
            (a.reset_stream_fast.0)(cs, l);
            (a.reset_stream_fast.1)(rs, l);
            assert_eq!(
                level_of(cs),
                want,
                "row 98: LZ4_resetStreamHC_fast({}) stored {}",
                l,
                level_of(cs)
            );
            assert_eq!(
                level_of(cs),
                level_of(rs),
                "row 98: LZ4_resetStreamHC_fast({}) C vs Rust",
                l
            );
            assert_state_blob_eq(&format!("row 98: resetStreamHC_fast({})", l), cs, rs);
        }
        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
    }
}

// ===========================================================================
// ERRORS.md rows 99, 100 — `(U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE`
// (lz4hc.c:1389) returns 0. The comparison is UNSIGNED, so it rejects negative
// sizes too, and it runs BEFORE `ctx->end += *srcSizePtr` and before any buffer
// access — so a LYING size can be passed with a small real buffer.
//
// (`tests/lz4hc_diff.rs::hc_error_srcsize_out_of_range` covers the one-shot
// entry points; this test adds every remaining entry point named by row 99.)
// ===========================================================================

fn bad_src_sizes() -> Vec<c_int> {
    vec![
        c_int::MIN,                      // row 99
        -1_000_000,                      // row 99
        -2,                              // row 99
        -1,                              // row 99
        LZ4_MAX_INPUT_SIZE as c_int + 1, // row 100
        0x7EFF_FFFF,                     // row 100
        0x7FFF_FFFE,                     // row 100
        c_int::MAX,                      // row 100
    ]
}

#[test]
fn row_99_100_srcsize_out_of_range_every_entry_point() {
    let a = api();
    let mut rng = Rng::new(0x99_100_5125);
    let src = src_buf(&mut rng, 3, 8192);
    // Levels spanning all three strategies. The size check fires before
    // `LZ4HC_getCLevelParams` dispatches, so levels 1/2 are safe here even
    // though LZ4MID_compress itself is never entered.
    let levels: [c_int; 7] = [c_int::MIN, 0, 1, 2, 9, 12, c_int::MAX];

    unsafe {
        for &n in &bad_src_sizes() {
            for &level in &levels {
                // --- LZ4_compress_HC
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let cn = (a.compress_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                    level,
                );
                let rn = (a.compress_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                    level,
                );
                assert_eq!(cn, rn, "rows 99/100 compress_HC n={} level={}", n, level);
                assert_eq!(cn, 0, "rows 99/100 compress_HC n={} must be 0", n);
                assert_bytes_eq("rows 99/100 compress_HC dst untouched", &cd, &rd);

                // --- LZ4_compress_HC_extStateHC and _fastReset
                let mut cst = state_buf();
                let mut rst = state_buf();
                for (tag, cf, rf) in [
                    ("extStateHC", a.ext_state.0, a.ext_state.1),
                    ("extStateHC_fastReset", a.ext_state_fast.0, a.ext_state_fast.1),
                ] {
                    let mut cd = vec![SENTINEL; 256];
                    let mut rd = vec![SENTINEL; 256];
                    let cn = cf(
                        cst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        n,
                        256,
                        level,
                    );
                    let rn = rf(
                        rst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        n,
                        256,
                        level,
                    );
                    assert_eq!(cn, rn, "rows 99/100 {} n={} level={}", tag, n, level);
                    assert_eq!(cn, 0, "rows 99/100 {} n={} must be 0", tag, n);
                    assert_bytes_eq(&format!("rows 99/100 {} dst untouched", tag), &cd, &rd);
                }

                // --- LZ4_compress_HC_continue
                let cs = (a.create_stream.0)();
                let rs = (a.create_stream.1)();
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let cn = (a.cont.0)(
                    cs,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                );
                let rn = (a.cont.1)(
                    rs,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                );
                assert_eq!(cn, rn, "rows 99/100 continue n={} level={}", n, level);
                assert_eq!(cn, 0, "rows 99/100 continue n={} must be 0", n);
                assert_bytes_eq("rows 99/100 continue dst untouched", &cd, &rd);
                assert_state_blob_eq("rows 99/100 continue state", cs, rs);
                assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));

                // --- LZ4_compressHC2_continue (dstCapacity hard-coded 0,
                //     notLimited) and LZ4_compressHC2_limitedOutput_continue.
                //     Both bypass auto-init, so a LZ4_createHC-initialised
                //     state is used. The size check still fires first, so no
                //     write can happen and levels 1/2 cannot reach the live
                //     `assert(op <= oend)` at lz4hc.c:743.
                let cdata = (a.d_create_hc.0)(src.as_ptr() as *const c_char);
                let rdata = (a.d_create_hc.1)(src.as_ptr() as *const c_char);
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let cn = (a.d_hc2_cont.0)(
                    cdata,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    level,
                );
                let rn = (a.d_hc2_cont.1)(
                    rdata,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    level,
                );
                assert_eq!(cn, rn, "rows 99/100 compressHC2_continue n={}", n);
                assert_eq!(cn, 0, "rows 99/100 compressHC2_continue n={} must be 0", n);
                assert_bytes_eq("rows 99/100 compressHC2_continue dst", &cd, &rd);

                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let cn = (a.d_hc2_lim_cont.0)(
                    cdata,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                    level,
                );
                let rn = (a.d_hc2_lim_cont.1)(
                    rdata,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    n,
                    256,
                    level,
                );
                assert_eq!(cn, rn, "rows 99/100 compressHC2_limitedOutput_continue n={}", n);
                assert_eq!(cn, 0, "rows 99/100 compressHC2_lim_continue n={} must be 0", n);
                assert_bytes_eq("rows 99/100 compressHC2_lim_continue dst", &cd, &rd);
                assert_state_blob_eq("rows 99/100 legacy state", cdata, rdata);
                assert_eq!((a.d_free_hc.0)(cdata), (a.d_free_hc.1)(rdata));
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 101, 102, 103 — the two destSize entry points
//   * row 101: `(U32)*sourceSizePtr > (U32)LZ4_MAX_INPUT_SIZE` -> 0
//   * row 102: `LZ4_compress_HC_destSize` with `targetDestSize < 1` -> 0
//              (`limit == fillOutput && dstCapacity < 1`, lz4hc.c:1388)
//   * row 103: same check reached through
//              `LZ4_compress_HC_continue_destSize` (lz4hc.c:1733)
// ===========================================================================

#[test]
fn row_101_102_103_destsize_rejections() {
    let a = api();
    let mut rng = Rng::new(0x101_102_103);
    let src = src_buf(&mut rng, 4, 8192);

    unsafe {
        // ---- row 101: bad *sourceSizePtr (targetDestSize is valid) ----------
        for &n in &bad_src_sizes() {
            for &level in &[c_int::MIN, 1, 2, 9, 12, c_int::MAX] {
                let mut cst = state_buf();
                let mut rst = state_buf();
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let mut css = n;
                let mut rss = n;
                let cn = (a.dest_size.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut css,
                    256,
                    level,
                );
                let rn = (a.dest_size.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut rss,
                    256,
                    level,
                );
                assert_eq!(cn, rn, "row 101: destSize srcSize={} level={}", n, level);
                assert_eq!(cn, 0, "row 101: destSize srcSize={} must be 0", n);
                assert_eq!(css, n, "row 101: *sourceSizePtr must be left untouched");
                assert_eq!(css, rss, "row 101: *sourceSizePtr C vs Rust");
                assert_bytes_eq("row 101: destSize dst untouched", &cd, &rd);
                assert_state_blob_eq("row 101: destSize state", cst.as_ptr() as *const c_void, rst.as_ptr() as *const c_void);
            }
        }

        // ---- row 102: targetDestSize < 1 ------------------------------------
        for &target in &[c_int::MIN, -1_000_000, -1, 0] {
            for &level in &[c_int::MIN, 1, 2, 9, 12, c_int::MAX] {
                let mut cst = state_buf();
                let mut rst = state_buf();
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = (a.dest_size.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut css,
                    target,
                    level,
                );
                let rn = (a.dest_size.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut rss,
                    target,
                    level,
                );
                assert_eq!(cn, rn, "row 102: destSize target={} level={}", target, level);
                assert_eq!(cn, 0, "row 102: destSize target={} must be 0", target);
                assert_eq!(
                    css,
                    src.len() as c_int,
                    "row 102: *sourceSizePtr must be left untouched"
                );
                assert_eq!(css, rss, "row 102: *sourceSizePtr C vs Rust");
                assert_bytes_eq("row 102: destSize dst untouched", &cd, &rd);
            }
        }

        // ---- row 103: LZ4_compress_HC_continue_destSize, targetDestSize < 1 -
        for &target in &[c_int::MIN, -1_000_000, -1, 0] {
            for &level in &[c_int::MIN, 1, 2, 9, 12, c_int::MAX] {
                let cs = (a.create_stream.0)();
                let rs = (a.create_stream.1)();
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let mut cd = vec![SENTINEL; 256];
                let mut rd = vec![SENTINEL; 256];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = (a.cont_dest_size.0)(
                    cs,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut css,
                    target,
                );
                let rn = (a.cont_dest_size.1)(
                    rs,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut rss,
                    target,
                );
                assert_eq!(
                    cn, rn,
                    "row 103: continue_destSize target={} level={}",
                    target, level
                );
                assert_eq!(cn, 0, "row 103: continue_destSize target={} must be 0", target);
                assert_eq!(css, rss, "row 103: *srcSizePtr C vs Rust");
                assert_eq!(
                    css,
                    src.len() as c_int,
                    "row 103: *srcSizePtr must be left untouched"
                );
                assert_bytes_eq("row 103: continue_destSize dst untouched", &cd, &rd);
                assert_state_blob_eq("row 103: continue_destSize state", cs, rs);
                assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 104, 105, 106, 108 — NULL / misaligned `state`
//   * row 104: `LZ4_compress_HC_extStateHC_fastReset` misaligned -> 0
//              (`if (!LZ4_isAligned(...)) return 0;`, lz4hc.c:1503)
//   * row 105: `LZ4_compress_HC_extStateHC` state == NULL -> 0 (1514-1515)
//   * row 106: `LZ4_compress_HC_extStateHC` misaligned -> 0 (1515 via 1580)
//   * row 108: `LZ4_compress_HC_destSize` NULL or misaligned -> 0 (1540-1541)
//
// NOTE: `LZ4_compress_HC_extStateHC_fastReset(NULL, ...)` is NOT probed:
// `LZ4_isAligned(NULL, 8)` is `((size_t)0 & 7) == 0` -> TRUE, so the C accepts
// the NULL pointer and then dereferences it inside `LZ4_resetStreamHC_fast`.
// That is an unconditional NULL dereference in both libraries.
//
// (`tests/lz4hc_diff.rs::hc_extstatehc_misaligned_state_returns_zero` covers
// the misaligned half of rows 104/106 from the same angle.)
// ===========================================================================

#[test]
fn row_104_105_106_108_extstatehc_and_destsize_bad_state() {
    let a = api();
    let ssz = unsafe { (a.sizeof_state.0)() } as usize;
    assert_eq!(ssz, HC_STREAM_BYTES, "LZ4_sizeofStateHC");
    assert_eq!(ssz, unsafe { (a.sizeof_state.1)() } as usize, "sizeofStateHC parity");

    let mut rng = Rng::new(0x104_108);
    let src = src_buf(&mut rng, 5, 6000);
    let bound = bound_of(src.len());
    let levels: [c_int; 6] = [c_int::MIN, 1, 2, 9, 12, c_int::MAX];

    unsafe {
        // ---- row 105: extStateHC with state == NULL -------------------------
        for &level in &levels {
            let mut cd = vec![SENTINEL; bound + 32];
            let mut rd = vec![SENTINEL; bound + 32];
            let cn = (a.ext_state.0)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                level,
            );
            let rn = (a.ext_state.1)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                level,
            );
            assert_eq!(cn, rn, "row 105: extStateHC(NULL) level={}", level);
            assert_eq!(cn, 0, "row 105: extStateHC(NULL) must return exactly 0");
            assert_bytes_eq("row 105: extStateHC(NULL) dst untouched", &cd, &rd);
        }

        // ---- row 108: destSize with state == NULL ---------------------------
        for &level in &levels {
            let mut cd = vec![SENTINEL; bound + 32];
            let mut rd = vec![SENTINEL; bound + 32];
            let mut css = src.len() as c_int;
            let mut rss = src.len() as c_int;
            let cn = (a.dest_size.0)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                &mut css,
                bound as c_int,
                level,
            );
            let rn = (a.dest_size.1)(
                std::ptr::null_mut(),
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                &mut rss,
                bound as c_int,
                level,
            );
            assert_eq!(cn, rn, "row 108: destSize(NULL) level={}", level);
            assert_eq!(cn, 0, "row 108: destSize(NULL) must return exactly 0");
            assert_eq!(css, rss, "row 108: *sourceSizePtr C vs Rust");
            assert_eq!(css, src.len() as c_int, "row 108: *sourceSizePtr untouched");
            assert_bytes_eq("row 108: destSize(NULL) dst untouched", &cd, &rd);
        }

        // ---- rows 104 / 106 / 108: misaligned state ------------------------
        for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
            let mut cst = AlignedBuf::with_offset(ssz, 8, off);
            let mut rst = AlignedBuf::with_offset(ssz, 8, off);
            for &level in &levels {
                // row 106 — LZ4_compress_HC_extStateHC
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.ext_state.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                let rn = (a.ext_state.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                assert_eq!(cn, rn, "row 106: extStateHC(+{}) level={}", off, level);
                assert_eq!(cn, 0, "row 106: extStateHC(+{}) must be exactly 0", off);
                assert_bytes_eq("row 106: extStateHC misaligned dst untouched", &cd, &rd);

                // row 104 — LZ4_compress_HC_extStateHC_fastReset
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.ext_state_fast.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                let rn = (a.ext_state_fast.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                assert_eq!(cn, rn, "row 104: fastReset(+{}) level={}", off, level);
                assert_eq!(cn, 0, "row 104: fastReset(+{}) must be exactly 0", off);
                assert_bytes_eq("row 104: fastReset misaligned dst untouched", &cd, &rd);

                // row 108 — LZ4_compress_HC_destSize
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = (a.dest_size.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    &mut css,
                    bound as c_int,
                    level,
                );
                let rn = (a.dest_size.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    &mut rss,
                    bound as c_int,
                    level,
                );
                assert_eq!(cn, rn, "row 108: destSize(+{}) level={}", off, level);
                assert_eq!(cn, 0, "row 108: destSize(+{}) must be exactly 0", off);
                assert_eq!(css, rss, "row 108: *sourceSizePtr C vs Rust");
                assert_bytes_eq("row 108: destSize misaligned dst untouched", &cd, &rd);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 109, 110, 111 — LZ4_initStreamHC returns NULL for
//   * row 109: `buffer == NULL`                        (lz4hc.c:1578)
//   * row 110: `size < sizeof(LZ4_streamHC_t)` (262200) (lz4hc.c:1579)
//   * row 111: `buffer` not 8-byte aligned             (lz4hc.c:1580)
//
// (`tests/lz4hc_diff.rs::hc_init_stream_invalid_buffers` covers the same three.)
// ===========================================================================

#[test]
fn row_109_110_111_init_stream_hc_returns_null() {
    let a = api();
    unsafe {
        // row 109 — NULL buffer, for every size including the valid one.
        for &size in &[0usize, 1, HC_STREAM_BYTES - 1, HC_STREAM_BYTES, usize::MAX] {
            let cp = (a.init_stream.0)(std::ptr::null_mut(), size);
            let rp = (a.init_stream.1)(std::ptr::null_mut(), size);
            assert!(cp.is_null(), "row 109: initStreamHC(NULL,{}) must be NULL", size);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 109: initStreamHC(NULL,{}) C vs Rust",
                size
            );
        }

        // row 110 — undersized `size` with a perfectly valid, aligned buffer.
        let mut cb = state_buf();
        let mut rb = state_buf();
        for &size in &[
            0usize,
            1,
            7,
            8,
            64,
            HC_TABLES_BYTES,
            HC_CCTX_BYTES,
            HC_STREAM_BYTES - 8,
            HC_STREAM_BYTES - 2,
            HC_STREAM_BYTES - 1,
        ] {
            let cp = (a.init_stream.0)(cb.as_mut_ptr() as *mut c_void, size);
            let rp = (a.init_stream.1)(rb.as_mut_ptr() as *mut c_void, size);
            assert!(cp.is_null(), "row 110: initStreamHC(size={}) must be NULL", size);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 110: initStreamHC(size={}) C vs Rust",
                size
            );
        }
        // The boundary must be accepted, pinning the exact threshold at 262200.
        for &size in &[HC_STREAM_BYTES, HC_STREAM_BYTES + 1, HC_STREAM_BYTES * 2] {
            let cp = (a.init_stream.0)(cb.as_mut_ptr() as *mut c_void, size);
            let rp = (a.init_stream.1)(rb.as_mut_ptr() as *mut c_void, size);
            assert!(
                !cp.is_null() && !rp.is_null(),
                "row 110: initStreamHC(size={}) must succeed",
                size
            );
            assert_state_blob_eq(&format!("row 110: initStreamHC(size={})", size), cp, rp);
        }

        // row 111 — misaligned buffer (LZ4_streamHC_t alignment is 8).
        for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
            let mut mc = AlignedBuf::with_offset(HC_STREAM_BYTES, 8, off);
            let mut mr = AlignedBuf::with_offset(HC_STREAM_BYTES, 8, off);
            let cp = (a.init_stream.0)(mc.as_mut_ptr() as *mut c_void, HC_STREAM_BYTES);
            let rp = (a.init_stream.1)(mr.as_mut_ptr() as *mut c_void, HC_STREAM_BYTES);
            assert!(cp.is_null(), "row 111: initStreamHC(+{}) must be NULL", off);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 111: initStreamHC(+{}) C vs Rust",
                off
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 113, 114, 116 — the free / legacy-reset sentinels
//   * row 113: `LZ4_freeStreamHC(NULL)` -> 0        (lz4hc.c:1566)
//   * row 114: `LZ4_freeHC(NULL)`       -> 0        (lz4hc.c:2169)
//   * row 116: `LZ4_resetStreamStateHC` -> **1** on init failure (lz4hc.c:2153)
//              — INVERTED relative to every other function (success is 0).
// ===========================================================================

#[test]
fn row_113_114_116_free_null_and_reset_stream_state_hc() {
    let a = api();
    unsafe {
        // row 113
        let cv = (a.free_stream.0)(std::ptr::null_mut());
        let rv = (a.free_stream.1)(std::ptr::null_mut());
        assert_eq!(cv, 0, "row 113: LZ4_freeStreamHC(NULL) must return exactly 0");
        assert_eq!(cv, rv, "row 113: LZ4_freeStreamHC(NULL) C vs Rust");

        // row 114
        let cv = (a.d_free_hc.0)(std::ptr::null_mut());
        let rv = (a.d_free_hc.1)(std::ptr::null_mut());
        assert_eq!(cv, 0, "row 114: LZ4_freeHC(NULL) must return exactly 0");
        assert_eq!(cv, rv, "row 114: LZ4_freeHC(NULL) C vs Rust");

        // row 116 — NULL state. `LZ4_resetStreamStateHC` has no `size`
        // parameter (it hard-codes `sizeof(*hc4)`), so the "too small" trigger
        // of row 116 is not expressible through the API; NULL and misaligned
        // are.
        let mut inbuf = vec![0u8; 64];
        for pass_buf in [false, true] {
            let ib = if pass_buf {
                inbuf.as_mut_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            };
            let cv = (a.d_reset_stream_state.0)(std::ptr::null_mut(), ib);
            let rv = (a.d_reset_stream_state.1)(std::ptr::null_mut(), ib);
            assert_eq!(
                cv, 1,
                "row 116: resetStreamStateHC(NULL) must return exactly 1 (error)"
            );
            assert_eq!(cv, rv, "row 116: resetStreamStateHC(NULL) C vs Rust");

            // misaligned state -> LZ4_initStreamHC NULL -> 1
            for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
                let mut mc = AlignedBuf::with_offset(HC_STREAM_BYTES, 8, off);
                let mut mr = AlignedBuf::with_offset(HC_STREAM_BYTES, 8, off);
                let cv = (a.d_reset_stream_state.0)(mc.as_mut_ptr() as *mut c_void, ib);
                let rv = (a.d_reset_stream_state.1)(mr.as_mut_ptr() as *mut c_void, ib);
                assert_eq!(
                    cv, 1,
                    "row 116: resetStreamStateHC(+{}) must return exactly 1",
                    off
                );
                assert_eq!(cv, rv, "row 116: resetStreamStateHC(+{}) C vs Rust", off);
            }

            // ... and the success case really is 0, pinning the inversion.
            let mut cok = state_buf();
            let mut rok = state_buf();
            let cv = (a.d_reset_stream_state.0)(cok.as_mut_ptr() as *mut c_void, ib);
            let rv = (a.d_reset_stream_state.1)(rok.as_mut_ptr() as *mut c_void, ib);
            assert_eq!(cv, 0, "row 116: success must return exactly 0");
            assert_eq!(cv, rv, "row 116: success C vs Rust");
            assert_state_blob_eq(
                "row 116: resetStreamStateHC success state",
                cok.as_ptr() as *const c_void,
                rok.as_ptr() as *const c_void,
            );
        }
    }
}

// ===========================================================================
// ERRORS.md row 117 — `LZ4MID_compress` (levels 1-2) rejects a negative
// `maxOutputSize`: `if (maxOutputSize < 0) return 0;` (lz4hc.c:560).
//
// Reachable through the public API: `LZ4_compress_HC` with dstCapacity < 0
// selects `limitedOutput` (dstCapacity < LZ4_compressBound), and the guards at
// lz4hc.c:559/561 do not fire first for a valid srcSize.
// The other two strategies reject a negative capacity through their output-limit
// arithmetic, so all levels are swept and all must return exactly 0.
// ===========================================================================

#[test]
fn row_117_negative_dstcapacity_returns_zero() {
    let a = api();
    let mut rng = Rng::new(0x117_0A11);
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 12, 13, 100, 5000] {
            let src = src_buf(&mut rng, shape, len);
            for &cap in &[c_int::MIN, -1_000_000, -2, -1] {
                for &level in &[c_int::MIN, 0, 1, 2, 3, 9, 10, 12, c_int::MAX] {
                    let mut cd = vec![SENTINEL; 64];
                    let mut rd = vec![SENTINEL; 64];
                    let cn = unsafe {
                        (a.compress_hc.0)(
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap,
                            level,
                        )
                    };
                    let rn = unsafe {
                        (a.compress_hc.1)(
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap,
                            level,
                        )
                    };
                    let label = format!(
                        "row 117 shape={} len={} cap={} level={}",
                        shape_name(shape),
                        len,
                        cap,
                        level
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_eq!(cn, 0, "{}: must return exactly 0", label);
                    assert_bytes_eq(&format!("{}: dst untouched", label), &cd, &rd);
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 120, 121 — the two output-overflow exits of `LZ4MID_compress`
// (levels 1-2, the lz4mid strategy)
//   * row 120: `limit == limitedOutput` and the FINAL literal run does not fit
//              (`op + totalSize > oend` -> `return 0`, lz4hc.c:713-714).
//              Constructed with an input shorter than LZ4_minLength (13), which
//              jumps straight to `_lz4mid_last_literals`.
//   * row 121: mid-stream overflow — `LZ4HC_encodeSequence` returns 1, control
//              falls through `_lz4mid_dest_overflow` and, because
//              `limit != fillOutput`, `return 0` (lz4hc.c:684-689, 771-772).
//              Constructed with a highly repetitive 5000-byte input, which
//              produces one huge sequence, and a capacity far below its cost.
// ===========================================================================

#[test]
fn row_120_lz4mid_last_literals_do_not_fit() {
    let a = api();
    let mut rng = Rng::new(0x120_11AA);
    for shape in 0..N_SHAPES {
        for len in 0..LZ4_MIN_LENGTH {
            let src = src_buf(&mut rng, shape, len);
            // Encoding `len` literals costs 1 token + ceil-extension + len bytes.
            let need = 1 + len;
            for cap in 0..=(need + 2) {
                for &level in &[1i32, 2] {
                    let label = format!(
                        "row 120 shape={} len={} cap={} level={}",
                        shape_name(shape),
                        len,
                        cap,
                        level
                    );
                    let mut cd = vec![SENTINEL; cap + 32];
                    let mut rd = vec![SENTINEL; cap + 32];
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
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    if cap < need {
                        assert_eq!(cn, 0, "{}: last-literal run must not fit -> 0", label);
                    } else {
                        assert_eq!(cn as usize, need, "{}: exact literal-run size", label);
                    }
                }
            }
        }
    }
}

#[test]
fn row_121_lz4mid_midstream_dest_overflow() {
    let a = api();
    // A 5000-byte run of one byte: the encoder finds a single ~4995-byte match
    // at offset 1 after one literal, so the whole block costs ~30 bytes. Any
    // capacity <= 20 therefore fails inside LZ4HC_encodeSequence (either the
    // literal check at lz4hc.c:305 or the match-length check at 331) and lands
    // on `_lz4mid_dest_overflow` with limit == limitedOutput.
    let src = gen_constant(5000, 0x5A);
    for &level in &[1i32, 2] {
        for cap in 0..=64usize {
            let label = format!("row 121 cap={} level={}", cap, level);
            let mut cd = vec![SENTINEL; cap + 32];
            let mut rd = vec![SENTINEL; cap + 32];
            let cn = unsafe {
                (a.compress_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.compress_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "{}: return", label);
            assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
            if cap <= 20 {
                assert_eq!(cn, 0, "{}: must return exactly 0", label);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 122, 123, 124, 125 — the hashChain strategy (levels 3-9)
//   * row 122: literal-length output check inside `LZ4HC_encodeSequence`
//              (`limit && ((op + length/255 + length + (2+1+LASTLITERALS)) > oend)`,
//              lz4hc.c:305-309) -> helper returns 1 -> `_dest_overflow`.
//   * row 123: match-length output check (lz4hc.c:331-334) -> helper returns 1.
//   * row 124: final literal run does not fit (lz4hc.c:1314-1315) -> 0.
//   * row 125: `_dest_overflow` with `limit != fillOutput` -> 0
//              (lz4hc.c:1340-1341, 1360-1361).
//
// Construction for 122/123/125: a 5000-byte constant run encodes as
// [1 literal][match ~4995 @ offset 1]. The literal check needs 1+0+8 = 9 bytes,
// so capacities 0..8 exercise row 122 and capacities 9..~28 get past the
// literals and then fail the match-length check, exercising row 123. Both then
// reach `_dest_overflow`, which returns 0 (row 125) because the directive is
// `limitedOutput`.
// ===========================================================================

#[test]
fn row_122_123_125_hashchain_encode_sequence_overflow() {
    let a = api();
    let src = gen_constant(5000, 0x37);
    for &level in &[3i32, 4, 5, 6, 7, 8, 9] {
        for cap in 0..=64usize {
            let label = format!("rows 122/123/125 cap={} level={}", cap, level);
            let mut cd = vec![SENTINEL; cap + 32];
            let mut rd = vec![SENTINEL; cap + 32];
            let cn = unsafe {
                (a.compress_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.compress_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "{}: return", label);
            assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
            if cap <= 20 {
                // row 122 for cap <= 8, row 123 for 9..20; row 125 in both.
                assert_eq!(cn, 0, "{}: must return exactly 0", label);
            }
        }
    }

    // A literal-heavy input drives row 122 with a LONG literal run, so the
    // `length/255` term of the check is non-zero.
    let mut rng = Rng::new(0x122_1123);
    let mut lit = gen_random(&mut rng, 600);
    lit.extend_from_slice(&lit.clone()[..400]); // trailing long match
    for &level in &[3i32, 9] {
        for cap in 0..=40usize {
            let label = format!("row 122 long-literals cap={} level={}", cap, level);
            let mut cd = vec![SENTINEL; cap + 32];
            let mut rd = vec![SENTINEL; cap + 32];
            let cn = unsafe {
                (a.compress_hc.0)(
                    lit.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    lit.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.compress_hc.1)(
                    lit.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    lit.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "{}: return", label);
            assert_eq!(cn, 0, "{}: must return exactly 0", label);
            assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
        }
    }
}

#[test]
fn row_124_hashchain_last_literals_do_not_fit() {
    let a = api();
    let mut rng = Rng::new(0x124_11AA);
    for shape in 0..N_SHAPES {
        for len in 0..LZ4_MIN_LENGTH {
            let src = src_buf(&mut rng, shape, len);
            let need = 1 + len;
            for cap in 0..=(need + 2) {
                for &level in &[3i32, 6, 9] {
                    let label = format!(
                        "row 124 shape={} len={} cap={} level={}",
                        shape_name(shape),
                        len,
                        cap,
                        level
                    );
                    let mut cd = vec![SENTINEL; cap + 32];
                    let mut rd = vec![SENTINEL; cap + 32];
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
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    if cap < need {
                        assert_eq!(cn, 0, "{}: must return exactly 0", label);
                    } else {
                        assert_eq!(cn as usize, need, "{}: exact literal-run size", label);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 126, 127 — the optimal parser (levels 10-12)
//   * row 126: `limit == limitedOutput` and the final literal run does not fit:
//              `retval = 0; goto _return_label;` (lz4hc.c:2065-2069).
//   * row 127: `_dest_overflow` with `limit != fillOutput`: `retval` keeps its
//              initial 0 (lz4hc.c:1835, 2095-2117, 2122).
// ===========================================================================

#[test]
fn row_126_127_optimal_parser_overflow() {
    let a = api();
    let mut rng = Rng::new(0x126_127);

    // row 126 — input below LZ4_minLength skips the main loop entirely.
    for shape in 0..N_SHAPES {
        for len in 0..LZ4_MIN_LENGTH {
            let src = src_buf(&mut rng, shape, len);
            let need = 1 + len;
            for cap in 0..=(need + 2) {
                for &level in &[10i32, 11, 12] {
                    let label = format!(
                        "row 126 shape={} len={} cap={} level={}",
                        shape_name(shape),
                        len,
                        cap,
                        level
                    );
                    let mut cd = vec![SENTINEL; cap + 32];
                    let mut rd = vec![SENTINEL; cap + 32];
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
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                    if cap < need {
                        assert_eq!(cn, 0, "{}: must return exactly 0", label);
                    } else {
                        assert_eq!(cn as usize, need, "{}: exact literal-run size", label);
                    }
                }
            }
        }
    }

    // row 127 — one huge "good enough" match, capacity far too small, so
    // LZ4HC_encodeSequence fails and `_dest_overflow` returns retval == 0.
    // A 5000-byte constant run costs ~30 bytes to encode (token + offset + the
    // 0xFF match-length extension bytes + the 5-byte trailing literal run), so
    // any capacity <= 20 provably cannot hold it and must reach `_dest_overflow`.
    let src = gen_constant(5000, 0x91);
    for &level in &[10i32, 11, 12] {
        for cap in 0..=40usize {
            let label = format!("row 127 cap={} level={}", cap, level);
            let mut cd = vec![SENTINEL; cap + 32];
            let mut rd = vec![SENTINEL; cap + 32];
            let cn = unsafe {
                (a.compress_hc.0)(
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            let rn = unsafe {
                (a.compress_hc.1)(
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    cap as c_int,
                    level,
                )
            };
            assert_eq!(cn, rn, "{}: return", label);
            assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
            if cap <= 20 {
                assert_eq!(cn, 0, "{}: must return exactly 0", label);
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 129 — level 12's `targetLength` is `LZ4_OPT_NUM` (4096), and
// `LZ4HC_compress_optimal` silently clamps it:
// `if (sufficient_len >= LZ4_OPT_NUM) sufficient_len = LZ4_OPT_NUM-1;`
// (lz4hc.c:1861). The clamp changes the `firstMatch.len > sufficient_len`
// decision (immediate encoding vs. full parse) for a match of EXACTLY 4096.
//
// Asserted by driving level 12 with inputs whose longest match is exactly
// 4093..4098 — i.e. straddling both the clamped (4095) and unclamped (4096)
// thresholds — and requiring byte-identical output from both libraries.
// Levels 10/11 (targetLength 64/128, never clamped) are included as controls.
// ===========================================================================

#[test]
fn row_129_optimal_sufficient_len_clamped_to_opt_num_minus_one() {
    let mut rng = Rng::new(0x129_0F70);
    // Incompressible base so that the ONLY long match is the one we plant.
    let base = gen_random(&mut rng, 5000);

    for &m in &[4093usize, 4094, 4095, 4096, 4097, 4098] {
        let mut src = base.clone();
        src.extend_from_slice(&base[..m]);
        // A fresh random tail guarantees the planted match stops at exactly `m`.
        src.extend_from_slice(&gen_random(&mut rng, 128));
        let bound = bound_of(src.len());
        for &level in &[10i32, 11, 12] {
            let label = format!("row 129 matchLen={} level={}", m, level);
            let out = diff_compress_hc(&src, bound, level, &label);
            assert!(!out.is_empty(), "{}: compression must succeed", label);
        }
    }
}

// ===========================================================================
// ERRORS.md row 130 — `fillOutput`: when `targetDestSize` is smaller than the
// full block needs, the INPUT is silently truncated instead of failing. The
// function returns > 0 and rewrites `*srcSizePtr` / `*sourceSizePtr` with the
// number of bytes actually consumed (lz4hc.c:712-719 mid, 1313-1320 /
// 1341-1358 hashChain, 2064-2073 / 2096-2117 optimal).
//
// (`tests/lz4hc_diff.rs::hc_compress_hc_destsize_target_sweep` and
// `hc_continue_destsize` sweep the same functions for round-trip validity.)
// ===========================================================================

#[test]
fn row_130_destsize_truncates_input_instead_of_failing() {
    let a = api();
    let mut rng = Rng::new(0x130_7AD1);
    for shape in 0..N_SHAPES {
        let src = src_buf(&mut rng, shape, 30000);
        let bound = bound_of(src.len());
        for &level in &[1i32, 2, 3, 9, 10, 12] {
            // How many bytes `fillOutput` needs for the WHOLE block at this
            // level. Only targets strictly below this force a truncation; a
            // larger budget legitimately consumes the entire input.
            let full = {
                let mut cst = state_buf();
                let mut rst = state_buf();
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = unsafe {
                    (a.dest_size.0)(
                        cst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        &mut css,
                        bound as c_int,
                        level,
                    )
                };
                let rn = unsafe {
                    (a.dest_size.1)(
                        rst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        &mut rss,
                        bound as c_int,
                        level,
                    )
                };
                assert_eq!(cn, rn, "row 130: full-budget destSize return");
                assert_eq!(css, rss, "row 130: full-budget destSize *sourceSizePtr");
                assert_bytes_eq("row 130: full-budget destSize dst", &cd, &rd);
                assert_eq!(
                    css,
                    src.len() as c_int,
                    "row 130: a compressBound-sized budget must consume everything"
                );
                cn as usize
            };

            for &target in &[1usize, 2, 3, 8, 20, 100, 1000, 5000] {
                let must_truncate = target < full;
                let label = format!(
                    "row 130 shape={} target={} level={} (full={})",
                    shape_name(shape),
                    target,
                    level,
                    full
                );

                // ---- LZ4_compress_HC_destSize -------------------------------
                let mut cst = state_buf();
                let mut rst = state_buf();
                let mut cd = vec![SENTINEL; target + 32];
                let mut rd = vec![SENTINEL; target + 32];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = unsafe {
                    (a.dest_size.0)(
                        cst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        &mut css,
                        target as c_int,
                        level,
                    )
                };
                let rn = unsafe {
                    (a.dest_size.1)(
                        rst.as_mut_ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        &mut rss,
                        target as c_int,
                        level,
                    )
                };
                assert_eq!(cn, rn, "{}: destSize return", label);
                assert_eq!(css, rss, "{}: destSize *sourceSizePtr", label);
                assert_bytes_eq(&format!("{}: destSize dst", label), &cd, &rd);
                assert!(cn > 0, "{}: destSize must succeed, not fail", label);
                assert!(
                    (cn as usize) <= target,
                    "{}: destSize wrote past targetDestSize",
                    label
                );
                if must_truncate {
                    assert!(
                        css >= 0 && (css as usize) < src.len(),
                        "{}: destSize must report a TRUNCATED input ({} of {})",
                        label,
                        css,
                        src.len()
                    );
                } else {
                    assert_eq!(
                        css,
                        src.len() as c_int,
                        "{}: a big-enough budget must consume the whole input",
                        label
                    );
                }

                // ---- LZ4_compress_HC_continue_destSize ----------------------
                let cs = unsafe { (a.create_stream.0)() };
                let rs = unsafe { (a.create_stream.1)() };
                unsafe {
                    (a.reset_stream.0)(cs, level);
                    (a.reset_stream.1)(rs, level);
                }
                let mut cd = vec![SENTINEL; target + 32];
                let mut rd = vec![SENTINEL; target + 32];
                let mut css = src.len() as c_int;
                let mut rss = src.len() as c_int;
                let cn = unsafe {
                    (a.cont_dest_size.0)(
                        cs,
                        src.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        &mut css,
                        target as c_int,
                    )
                };
                let rn = unsafe {
                    (a.cont_dest_size.1)(
                        rs,
                        src.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        &mut rss,
                        target as c_int,
                    )
                };
                assert_eq!(cn, rn, "{}: continue_destSize return", label);
                assert_eq!(css, rss, "{}: continue_destSize *srcSizePtr", label);
                assert_bytes_eq(&format!("{}: continue_destSize dst", label), &cd, &rd);
                assert!(cn > 0, "{}: continue_destSize must succeed", label);
                assert!(
                    (cn as usize) <= target,
                    "{}: continue_destSize wrote past targetDestSize",
                    label
                );
                if must_truncate {
                    assert!(
                        css >= 0 && (css as usize) < src.len(),
                        "{}: continue_destSize must report a TRUNCATED input ({} of {})",
                        label,
                        css,
                        src.len()
                    );
                } else {
                    assert_eq!(
                        css,
                        src.len() as c_int,
                        "{}: a big-enough budget must consume the whole input",
                        label
                    );
                }
                unsafe {
                    assert_state_blob_eq(&label, cs, rs);
                    assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 131 — `LZ4_compress_HC_continue` picks the `notLimited`
// directive (no output-bound check at all) as soon as
// `dstCapacity >= LZ4_compressBound(srcSize)` (lz4hc.c:1725-1728).
//
// The consequence documented by the row — that an OVERSTATED capacity is not
// caught — cannot be exercised without writing out of bounds in BOTH libraries
// (same situation as rows 13/14 of lz4.c). What IS asserted here is that both
// libraries flip the directive at EXACTLY `LZ4_compressBound(srcSize)`:
// capacities bound-1 / bound / bound+1 must produce identical return values and
// identical bytes in C and Rust, and the notLimited side must never fail.
// ===========================================================================

#[test]
fn row_131_continue_notlimited_threshold_is_compressbound() {
    let a = api();
    let mut rng = Rng::new(0x131_B0DD);
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 13, 1000, 20000] {
            let src = src_buf(&mut rng, shape, len);
            let bound = bound_of(len);
            for &level in &[1i32, 2, 3, 9, 10, 12] {
                for cap in [bound - 1, bound, bound + 1] {
                    let label = format!(
                        "row 131 shape={} len={} level={} cap={} (bound={})",
                        shape_name(shape),
                        len,
                        level,
                        cap,
                        bound
                    );
                    unsafe {
                        let cs = (a.create_stream.0)();
                        let rs = (a.create_stream.1)();
                        (a.reset_stream.0)(cs, level);
                        (a.reset_stream.1)(rs, level);
                        // The buffer is really `cap + 32` bytes, so even the
                        // notLimited path stays inside owned memory.
                        let mut cd = vec![SENTINEL; cap + 32];
                        let mut rd = vec![SENTINEL; cap + 32];
                        let cn = (a.cont.0)(
                            cs,
                            src.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            src.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        );
                        assert_eq!(cn, rn, "{}: return", label);
                        assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                        assert!(
                            cn > 0,
                            "{}: an honestly-sized buffer must never fail",
                            label
                        );
                        assert_state_blob_eq(&label, cs, rs);
                        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md row 132 — `LZ4_compressHC2_continue` hard-codes `dstCapacity = 0`
// together with the `notLimited` directive (lz4hc.c:2177), so it performs NO
// output-bound check and can never return 0 for an overflow.
//
// Asserted by giving it a real compressBound-sized buffer while it believes the
// capacity is 0: the return must be > 0 (never the 0 sentinel) and identical in
// both libraries, for a `dst` that is a hundred times smaller than
// `LZ4_compressBound`.
//
// Levels 1 and 2 are EXCLUDED: `LZ4MID_compress` ends with a live
// `assert(op <= oend)` (lz4hc.c:743) and `oend == dst + 0` here, so that
// combination aborts the non-NDEBUG C build. `LZ4_compressHC2_limitedOutput_continue`
// does NOT hard-code the capacity (it forwards the caller's `dstCapacity` with
// the `limitedOutput` directive, lz4hc.c:2182), so it is used as the contrasting
// control that CAN return 0.
// ===========================================================================

#[test]
fn row_132_compresshc2_continue_has_no_output_bound_check() {
    let a = api();
    let mut rng = Rng::new(0x132_C0DE);
    let src = src_buf(&mut rng, 4, 20000);
    let bound = bound_of(src.len());

    for &level in &[c_int::MIN, 0, 3, 9, 10, 12, c_int::MAX] {
        unsafe {
            let cdata = (a.d_create_hc.0)(src.as_ptr() as *const c_char);
            let rdata = (a.d_create_hc.1)(src.as_ptr() as *const c_char);
            assert!(!cdata.is_null() && !rdata.is_null(), "LZ4_createHC");

            // notLimited with a hard-coded capacity of 0 -> must still succeed.
            let mut cd = vec![SENTINEL; bound + 32];
            let mut rd = vec![SENTINEL; bound + 32];
            let cn = (a.d_hc2_cont.0)(
                cdata,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                level,
            );
            let rn = (a.d_hc2_cont.1)(
                rdata,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                level,
            );
            assert_eq!(cn, rn, "row 132: compressHC2_continue level={}", level);
            assert!(
                cn > 0,
                "row 132: compressHC2_continue must NOT be able to return 0 (level={})",
                level
            );
            assert_bytes_eq(&format!("row 132: dst level={}", level), &cd, &rd);
            assert_state_blob_eq(&format!("row 132: state level={}", level), cdata, rdata);
            assert_eq!((a.d_free_hc.0)(cdata), (a.d_free_hc.1)(rdata));
        }
    }

    // Control: the *limitedOutput* legacy variant DOES check and returns 0.
    for &level in &[3i32, 9, 12] {
        unsafe {
            let cdata = (a.d_create_hc.0)(src.as_ptr() as *const c_char);
            let rdata = (a.d_create_hc.1)(src.as_ptr() as *const c_char);
            let mut cd = vec![SENTINEL; 40];
            let mut rd = vec![SENTINEL; 40];
            let cn = (a.d_hc2_lim_cont.0)(
                cdata,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                8,
                level,
            );
            let rn = (a.d_hc2_lim_cont.1)(
                rdata,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                8,
                level,
            );
            assert_eq!(cn, rn, "row 132 control: limitedOutput_continue level={}", level);
            assert_eq!(
                cn, 0,
                "row 132 control: limitedOutput_continue must return exactly 0"
            );
            assert_bytes_eq("row 132 control: dst", &cd, &rd);
            assert_eq!((a.d_free_hc.0)(cdata), (a.d_free_hc.1)(rdata));
        }
    }
}

// ===========================================================================
// ERRORS.md row 133 — `if (result <= 0) ctx->dirty = 1;` (lz4hc.c:1412). The
// documented consequence is that a later `LZ4_resetStreamHC_fast` performs a
// FULL re-init (lz4hc.c:1599-1600) instead of the cheap reset.
//
// Both halves are asserted on the exact field values:
//   * dirty == 1 in BOTH libraries after a failed compression;
//   * after `LZ4_resetStreamHC_fast`, the dirty stream's `dictLimit` is 0 (the
//     signature of the full `LZ4_initStreamHC` re-init) whereas a clean stream's
//     `dictLimit` has grown by the prefix size (the cheap path).
// ===========================================================================

#[test]
fn row_133_failed_compression_marks_stream_dirty() {
    let a = api();
    let mut rng = Rng::new(0x133_D127);
    let src = src_buf(&mut rng, 4, 8000);
    const BLOCK: usize = 3000;

    unsafe {
        for &level in &[1i32, 2, 3, 9, 10, 12] {
            // ---------- dirty path ----------
            let cs = (a.create_stream.0)();
            let rs = (a.create_stream.1)();
            (a.reset_stream.0)(cs, level);
            (a.reset_stream.1)(rs, level);

            // A successful first block establishes some history.
            let bound = bound_of(BLOCK);
            let mut cd = vec![SENTINEL; bound + 32];
            let mut rd = vec![SENTINEL; bound + 32];
            let cn = (a.cont.0)(
                cs,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                BLOCK as c_int,
                bound as c_int,
            );
            let rn = (a.cont.1)(
                rs,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                BLOCK as c_int,
                bound as c_int,
            );
            assert_eq!(cn, rn, "row 133: first block level={}", level);
            assert!(cn > 0, "row 133: first block must succeed");
            assert_bytes_eq("row 133: first block dst", &cd, &rd);
            assert_eq!(dirty_of(cs), 0, "row 133: success must NOT set dirty");
            assert_eq!(dirty_of(cs), dirty_of(rs), "row 133: dirty C vs Rust");
            let clean_prefix = view(cs).end_off;
            assert_eq!(clean_prefix, BLOCK as isize, "row 133: prefix size");

            // Now force a failure with a 3-byte capacity on the next block.
            let mut cd = vec![SENTINEL; 35];
            let mut rd = vec![SENTINEL; 35];
            let cn = (a.cont.0)(
                cs,
                src.as_ptr().add(BLOCK) as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                2000,
                3,
            );
            let rn = (a.cont.1)(
                rs,
                src.as_ptr().add(BLOCK) as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                2000,
                3,
            );
            assert_eq!(cn, rn, "row 133: failing block level={}", level);
            assert_eq!(cn, 0, "row 133: failing block must return exactly 0");
            assert_bytes_eq("row 133: failing block dst", &cd, &rd);
            assert_eq!(
                dirty_of(cs),
                1,
                "row 133: result <= 0 must set dirty to exactly 1 (level={})",
                level
            );
            assert_eq!(dirty_of(cs), dirty_of(rs), "row 133: dirty C vs Rust");
            assert_state_blob_eq("row 133: state after failure", cs, rs);

            // resetStreamHC_fast on a DIRTY stream -> full LZ4_initStreamHC.
            (a.reset_stream_fast.0)(cs, level);
            (a.reset_stream_fast.1)(rs, level);
            let vc = view(cs);
            assert_eq!(vc, view(rs), "row 133: state after dirty fast-reset");
            assert_eq!(
                vc.dirty, 0,
                "row 133: full re-init must clear dirty (level={})",
                level
            );
            assert_eq!(
                vc.dict_limit, 0,
                "row 133: dirty fast-reset must FULLY re-init (dictLimit 0), got {}",
                vc.dict_limit
            );
            assert!(vc.prefix_null, "row 133: full re-init nulls prefixStart");
            assert_state_blob_eq("row 133: blob after dirty fast-reset", cs, rs);
            assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));

            // ---------- clean contrast ----------
            let cs = (a.create_stream.0)();
            let rs = (a.create_stream.1)();
            (a.reset_stream.0)(cs, level);
            (a.reset_stream.1)(rs, level);
            let mut cd = vec![SENTINEL; bound + 32];
            let mut rd = vec![SENTINEL; bound + 32];
            let cn = (a.cont.0)(
                cs,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                BLOCK as c_int,
                bound as c_int,
            );
            let rn = (a.cont.1)(
                rs,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                BLOCK as c_int,
                bound as c_int,
            );
            assert_eq!(cn, rn);
            assert!(cn > 0);
            assert_bytes_eq("row 133 clean: dst", &cd, &rd);
            (a.reset_stream_fast.0)(cs, level);
            (a.reset_stream_fast.1)(rs, level);
            let vc = view(cs);
            assert_eq!(vc, view(rs), "row 133 clean: state after fast-reset");
            assert_eq!(
                vc.dict_limit,
                65536 + BLOCK as u32,
                "row 133 clean: cheap fast-reset must ADD the prefix size to dictLimit"
            );
            assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
        }
    }
}

// ===========================================================================
// ERRORS.md rows 134, 135, 136 — LZ4_loadDictHC's silent behaviours
//   * row 134: `dictSize > 64 KB` -> the pointer is advanced by
//              `dictSize - 64 KB` and the size becomes 65536; return is 65536
//              (lz4hc.c:1634-1637).
//   * row 135: `dictSize < LZ4HC_HASHSIZE (4)` on a non-lz4mid level skips
//              `LZ4HC_Insert` (lz4hc.c:1649) — returns `dictSize` but the index
//              tables stay empty.
//   * row 136: `dictSize <= LZ4MID_HASHSIZE (8)` on levels <= 2 makes
//              `LZ4MID_fillHTable` return immediately (lz4hc.c:498-499).
// ===========================================================================

#[test]
fn row_134_load_dict_hc_truncates_to_last_64kb() {
    let a = api();
    let mut rng = Rng::new(0x134_64B0);
    let dict = src_buf(&mut rng, 5, 200_000);

    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        for &dsz in &[65537usize, 70000, 131072, 200_000] {
            for &level in &[1i32, 2, 3, 9, 12] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let cl = (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, dsz as c_int);
                let rl = (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, dsz as c_int);
                assert_eq!(
                    cl, 65536,
                    "row 134: loadDictHC({}) must return exactly 65536",
                    dsz
                );
                assert_eq!(cl, rl, "row 134: loadDictHC({}) C vs Rust", dsz);
                // The kept window is the LAST 64 KB: prefixStart == dict+dsz-64KB.
                let want = dict.as_ptr().add(dsz - 65536);
                let cpx = (*(cs as *const HcCtx)).prefix_start;
                let rpx = (*(rs as *const HcCtx)).prefix_start;
                assert_eq!(
                    cpx, want,
                    "row 134: prefixStart must be dict + dictSize - 64 KB"
                );
                assert_eq!(cpx, rpx, "row 134: prefixStart C vs Rust");
                assert_eq!(
                    view(cs).end_off,
                    65536,
                    "row 134: end - prefixStart must be exactly 65536"
                );
                assert_state_blob_eq(&format!("row 134: dsz={} level={}", dsz, level), cs, rs);
            }
        }
        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
    }
}

#[test]
fn row_135_136_load_dict_hc_too_small_leaves_tables_empty() {
    let a = api();
    let mut rng = Rng::new(0x135_136);
    let dict = src_buf(&mut rng, 0, 64);

    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();
        // row 135: non-lz4mid levels skip LZ4HC_Insert below LZ4HC_HASHSIZE (4).
        // row 136: lz4mid levels (1-2) skip LZ4MID_fillHTable at or below
        //          LZ4MID_HASHSIZE (8).
        for &dsz in &[0usize, 1, 2, 3, 4, 5, 8, 9, 16] {
            for &level in &[1i32, 2, 3, 9, 12] {
                let mid = level <= 2;
                let inserts = if mid { dsz > 8 } else { dsz >= 4 };
                let label = format!("rows 135/136 dictSize={} level={}", dsz, level);

                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                let cl = (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, dsz as c_int);
                let rl = (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, dsz as c_int);
                assert_eq!(
                    cl, dsz as c_int,
                    "{}: must return dictSize unchanged",
                    label
                );
                assert_eq!(cl, rl, "{}: C vs Rust return", label);
                assert_state_blob_eq(&label, cs, rs);

                // The documented consequence: the index tables stayed all-zero.
                let ct = std::slice::from_raw_parts(cs as *const u8, HC_TABLES_BYTES);
                let all_zero = ct.iter().all(|&b| b == 0);
                if !inserts {
                    assert!(
                        all_zero,
                        "{}: no reference may be inserted (dictionary unusable)",
                        label
                    );
                } else {
                    assert!(
                        !all_zero,
                        "{}: a large-enough dictionary MUST insert references",
                        label
                    );
                }
            }
        }
        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
    }
}

// ===========================================================================
// ERRORS.md rows 138, 139, 140, 141 — LZ4_saveDictHC's three silent clamps
//   * row 138: `if (dictSize > 64 KB) dictSize = 64 KB;`   (lz4hc.c:1748)
//   * row 139: `if (dictSize < 4) dictSize = 0;`           (lz4hc.c:1749)
//   * row 140: `if (dictSize > prefixSize) dictSize = prefixSize;` (lz4hc.c:1750)
//   * row 141: `if (safeBuffer == NULL) assert(dictSize == 0);` (lz4hc.c:1751)
//              — the WELL-DEFINED half (a NULL buffer whose dictSize clamps to
//              0) is exercised; see the coverage map for why the other half is
//              not.
//
// (`tests/lz4hc_diff.rs::hc_save_dict_hc` sweeps the same function for
// round-trip validity.)
// ===========================================================================

#[test]
fn row_138_139_140_141_save_dict_hc_clamps() {
    let a = api();
    let mut rng = Rng::new(0x138_141);
    let dict = src_buf(&mut rng, 5, 200_000);

    unsafe {
        let cs = (a.create_stream.0)();
        let rs = (a.create_stream.1)();

        // ---- row 138: dictSize > 64 KB with a 64 KB prefix available --------
        for &level in &[1i32, 2, 9, 12] {
            (a.reset_stream.0)(cs, level);
            (a.reset_stream.1)(rs, level);
            assert_eq!(
                (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, 200_000),
                (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, 200_000)
            );
            for &ask in &[65537i32, 100_000, 200_000, c_int::MAX] {
                let mut cb = vec![SENTINEL; 65536 + 32];
                let mut rb = vec![SENTINEL; 65536 + 32];
                let cv = (a.save_dict.0)(cs, cb.as_mut_ptr() as *mut c_char, ask);
                let rv = (a.save_dict.1)(rs, rb.as_mut_ptr() as *mut c_char, ask);
                assert_eq!(cv, 65536, "row 138: saveDictHC({}) must return 65536", ask);
                assert_eq!(cv, rv, "row 138: saveDictHC({}) C vs Rust", ask);
                assert_bytes_eq(&format!("row 138: saved bytes ask={}", ask), &cb, &rb);
                assert_bytes_eq(
                    "row 138: saved window must be the LAST 64 KB of the prefix",
                    &dict[200_000 - 65536..],
                    &cb[..65536],
                );
                // Re-establish the prefix for the next iteration.
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                assert_eq!(
                    (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, 200_000),
                    (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, 200_000)
                );
            }
        }

        // ---- row 139: dictSize < 4 (incl. 0 and negative) -> 0, nothing copied
        for &level in &[1i32, 9, 12] {
            for &ask in &[c_int::MIN, -1_000_000, -1, 0, 1, 2, 3] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                assert_eq!(
                    (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, 4096),
                    (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, 4096)
                );
                let mut cb = vec![SENTINEL; 128];
                let mut rb = vec![SENTINEL; 128];
                let cv = (a.save_dict.0)(cs, cb.as_mut_ptr() as *mut c_char, ask);
                let rv = (a.save_dict.1)(rs, rb.as_mut_ptr() as *mut c_char, ask);
                assert_eq!(cv, 0, "row 139: saveDictHC({}) must return exactly 0", ask);
                assert_eq!(cv, rv, "row 139: saveDictHC({}) C vs Rust", ask);
                assert!(
                    cb.iter().all(|&b| b == SENTINEL),
                    "row 139: nothing may be copied for dictSize={}",
                    ask
                );
                assert_bytes_eq(&format!("row 139: buffer ask={}", ask), &cb, &rb);
                assert_eq!(
                    view(cs),
                    view(rs),
                    "row 139: state after saveDictHC({})",
                    ask
                );
            }
        }

        // ---- row 140: dictSize > prefixSize -> clamps to prefixSize ---------
        for &prefix in &[4usize, 5, 100, 1000, 4096] {
            for &level in &[1i32, 9, 12] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                assert_eq!(
                    (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, prefix as c_int),
                    (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, prefix as c_int)
                );
                for &ask in &[prefix as c_int + 1, prefix as c_int + 1000, 65536, 200_000] {
                    let mut cb = vec![SENTINEL; 65536 + 32];
                    let mut rb = vec![SENTINEL; 65536 + 32];
                    let cv = (a.save_dict.0)(cs, cb.as_mut_ptr() as *mut c_char, ask);
                    let rv = (a.save_dict.1)(rs, rb.as_mut_ptr() as *mut c_char, ask);
                    assert_eq!(
                        cv, prefix as c_int,
                        "row 140: saveDictHC(ask={}) must clamp to prefixSize {}",
                        ask, prefix
                    );
                    assert_eq!(cv, rv, "row 140: C vs Rust");
                    assert_bytes_eq(
                        "row 140: saved bytes are the whole prefix",
                        &dict[..prefix],
                        &cb[..prefix],
                    );
                    assert_bytes_eq(&format!("row 140: buffer ask={}", ask), &cb, &rb);
                    // saveDictHC re-points the stream at safeBuffer, so reload.
                    (a.reset_stream.0)(cs, level);
                    (a.reset_stream.1)(rs, level);
                    assert_eq!(
                        (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, prefix as c_int),
                        (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, prefix as c_int)
                    );
                }
            }
        }

        // ---- row 141 (well-defined half): safeBuffer == NULL on a stream with
        //      NO prefix. `dictSize` clamps to 0 via row 139/140, so the live
        //      `assert(dictSize == 0)` at lz4hc.c:1751 holds and the call is
        //      fully defined: it returns 0 and copies nothing.
        for &level in &[1i32, 9, 12] {
            for &ask in &[c_int::MIN, -1, 0, 3, 4, 100, 65536, c_int::MAX] {
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                // A freshly reset stream has end == prefixStart == NULL, so
                // prefixSize == 0 and every `ask` clamps to 0.
                let cv = (a.save_dict.0)(cs, std::ptr::null_mut(), ask);
                let rv = (a.save_dict.1)(rs, std::ptr::null_mut(), ask);
                assert_eq!(
                    cv, 0,
                    "row 141: saveDictHC(NULL, {}) on an empty prefix must return exactly 0",
                    ask
                );
                assert_eq!(cv, rv, "row 141: saveDictHC(NULL, {}) C vs Rust", ask);
                assert_eq!(view(cs), view(rs), "row 141: state after saveDictHC(NULL)");
                assert!(
                    view(cs).end_null && view(cs).prefix_null,
                    "row 141: a NULL safeBuffer must leave end/prefixStart NULL"
                );
            }
        }

        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
    }
}

// ===========================================================================
// ERRORS.md row 142 — `LZ4_attach_HC_dictionary(ws, NULL)` is not an error: it
// silently DETACHES by storing NULL in `dictCtx` (lz4hc.c:1655).
// ===========================================================================

#[test]
fn row_142_attach_hc_dictionary_null_detaches() {
    let a = api();
    let mut rng = Rng::new(0x142_A77A);
    let dict = src_buf(&mut rng, 3, 65536);
    let src = src_buf(&mut rng, 3, 8000);
    let bound = bound_of(src.len());

    unsafe {
        let cd_s = (a.create_stream.0)();
        let rd_s = (a.create_stream.1)();
        let cw = (a.create_stream.0)();
        let rw = (a.create_stream.1)();

        for &level in &[1i32, 2, 9, 12] {
            (a.reset_stream.0)(cd_s, level);
            (a.reset_stream.1)(rd_s, level);
            assert_eq!(
                (a.load_dict.0)(cd_s, dict.as_ptr() as *const c_char, 65536),
                (a.load_dict.1)(rd_s, dict.as_ptr() as *const c_char, 65536)
            );

            (a.reset_stream.0)(cw, level);
            (a.reset_stream.1)(rw, level);
            assert!(view(cw).dict_ctx_null, "row 142: fresh stream has no dictCtx");

            // attach -> non-NULL
            (a.attach.0)(cw, cd_s);
            (a.attach.1)(rw, rd_s);
            assert!(!view(cw).dict_ctx_null, "row 142: attach must set dictCtx");
            assert_eq!(
                view(cw).dict_ctx_null,
                view(rw).dict_ctx_null,
                "row 142: dictCtx nullness C vs Rust after attach"
            );

            // attach(NULL) -> back to NULL, no error signalling (void return)
            (a.attach.0)(cw, std::ptr::null());
            (a.attach.1)(rw, std::ptr::null());
            assert!(
                view(cw).dict_ctx_null,
                "row 142: attach(NULL) must clear dictCtx (level={})",
                level
            );
            assert_eq!(
                view(cw).dict_ctx_null,
                view(rw).dict_ctx_null,
                "row 142: dictCtx nullness C vs Rust after detach"
            );
            // Nothing else may have changed.
            assert_eq!(view(cw), view(rw), "row 142: state after detach");

            // ... and compression afterwards behaves as if never attached.
            let mut cdst = vec![SENTINEL; bound + 32];
            let mut rdst = vec![SENTINEL; bound + 32];
            let cn = (a.cont.0)(
                cw,
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
            );
            let rn = (a.cont.1)(
                rw,
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
            );
            assert_eq!(cn, rn, "row 142: post-detach compression level={}", level);
            assert!(cn > 0, "row 142: post-detach compression must succeed");
            assert_bytes_eq("row 142: post-detach dst", &cdst, &rdst);
            assert_state_blob_eq("row 142: post-detach state", cw, rw);
        }

        assert_eq!((a.free_stream.0)(cd_s), (a.free_stream.1)(rd_s));
        assert_eq!((a.free_stream.0)(cw), (a.free_stream.1)(rw));
    }
}

// ===========================================================================
// ERRORS.md row 143 — `LZ4_compress_HC_continue*` with a `src` range that
// OVERLAPS the context's extDict (`sourceEnd > dictBegin && src < dictEnd`,
// lz4hc.c:1706-1717): the dictionary is silently shrunk, and fully invalidated
// when `dictLimit - lowLimit < LZ4HC_HASHSIZE (4)`.
//
// Both sub-cases are constructed inside ONE owned buffer:
//   * block 1 = B[0..2048] establishes a 2048-byte prefix;
//   * block 2 starts at B[off] for off in 0..2048, which is NOT contiguous with
//     `end`, so `LZ4HC_setExternalDict` turns the prefix into an extDict that
//     the new source overlaps. Small `off` shrinks it (dictLimit-lowLimit >= 4),
//     large `off` invalidates it entirely.
// ===========================================================================

#[test]
fn row_143_continue_src_overlaps_extdict() {
    let a = api();
    let mut rng = Rng::new(0x143_0FA1);
    for shape in 0..N_SHAPES {
        let b = src_buf(&mut rng, shape, 4096);
        for &off in &[0usize, 1, 512, 1024, 2040, 2044, 2047, 2048] {
            for &n2 in &[100usize, 1000] {
                for &level in &[1i32, 2, 3, 9, 12] {
                    let label = format!(
                        "row 143 shape={} off={} n2={} level={}",
                        shape_name(shape),
                        off,
                        n2,
                        level
                    );
                    unsafe {
                        let cs = (a.create_stream.0)();
                        let rs = (a.create_stream.1)();
                        (a.reset_stream.0)(cs, level);
                        (a.reset_stream.1)(rs, level);

                        // block 1: B[0..2048]
                        let bound1 = bound_of(2048);
                        let mut cd = vec![SENTINEL; bound1 + 32];
                        let mut rd = vec![SENTINEL; bound1 + 32];
                        let cn = (a.cont.0)(
                            cs,
                            b.as_ptr() as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            2048,
                            bound1 as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            b.as_ptr() as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            2048,
                            bound1 as c_int,
                        );
                        assert_eq!(cn, rn, "{}: block1 return", label);
                        assert!(cn > 0, "{}: block1 must succeed", label);
                        assert_bytes_eq(&format!("{}: block1 dst", label), &cd, &rd);

                        // block 2: overlapping (or exactly abutting) source
                        let bound2 = bound_of(n2);
                        let mut cd = vec![SENTINEL; bound2 + 32];
                        let mut rd = vec![SENTINEL; bound2 + 32];
                        let cn = (a.cont.0)(
                            cs,
                            b.as_ptr().add(off) as *const c_char,
                            cd.as_mut_ptr() as *mut c_char,
                            n2 as c_int,
                            bound2 as c_int,
                        );
                        let rn = (a.cont.1)(
                            rs,
                            b.as_ptr().add(off) as *const c_char,
                            rd.as_mut_ptr() as *mut c_char,
                            n2 as c_int,
                            bound2 as c_int,
                        );
                        assert_eq!(cn, rn, "{}: block2 return", label);
                        assert!(cn > 0, "{}: block2 must succeed (no error path)", label);
                        assert_bytes_eq(&format!("{}: block2 dst", label), &cd, &rd);

                        // The silent shrink / invalidation must be IDENTICAL:
                        // lowLimit, dictStart and dictLimit are the fields the
                        // overlap handler rewrites.
                        let vc = view(cs);
                        assert_eq!(vc, view(rs), "{}: state after overlap handling", label);
                        assert_state_blob_eq(&label, cs, rs);
                        // dictLimit - lowLimit is the surviving dictionary size;
                        // it must be either >= 4 (shrunk) or exactly 0
                        // (invalidated) — never 1..3.
                        let surviving = vc.dict_limit.wrapping_sub(vc.low_limit);
                        assert!(
                            surviving == 0 || surviving >= LZ4HC_HASHSIZE_U32,
                            "{}: dictionary left in the forbidden 1..3 range ({})",
                            label,
                            surviving
                        );
                        assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
                    }
                }
            }
        }
    }
}

/// `LZ4HC_HASHSIZE` (lz4hc.c:281) as a u32, used by the row-143 invariant.
const LZ4HC_HASHSIZE_U32: u32 = 4;

// ===========================================================================
// ERRORS.md row 144 — the accumulated stream position overflow check
// `(size_t)(end - prefixStart) + dictLimit > 2 GB` (lz4hc.c:1695-1699) silently
// reloads the dictionary from the last <= 64 KB via `LZ4_loadDictHC`.
//
// The condition is reached WITHOUT a 2 GB allocation by writing the `dictLimit`
// (and `lowLimit` / `nextToUpdate`) index fields of the context directly — they
// are plain U32 counters, and BOTH libraries are given the identical bytes, so
// the comparison stays honest.
// ===========================================================================

#[test]
fn row_144_continue_two_gigabyte_position_overflow_reloads_dict() {
    let a = api();
    let mut rng = Rng::new(0x144_2_6B);
    let dict = src_buf(&mut rng, 5, 4096);
    let src = src_buf(&mut rng, 4, 6000);

    // Values straddling the exact `> 2 GB` boundary.
    const TWO_GB: u32 = 2 * 1024 * 1024 * 1024;
    for &(dict_size, base, expect_reload) in &[
        (1000usize, TWO_GB - 2000, false), // 2 GB - 1000 -> NOT over
        (1000usize, TWO_GB - 1000, false), // exactly 2 GB -> NOT over (`>`)
        (1000usize, TWO_GB - 999, true),   // 2 GB + 1 -> over
        (1000usize, TWO_GB, true),
        (4096usize, TWO_GB, true),
    ] {
        for &level in &[1i32, 2, 3, 9, 12] {
            let label = format!(
                "row 144 dictSize={} dictLimit={:#x} level={}",
                dict_size, base, level
            );
            unsafe {
                let cs = (a.create_stream.0)();
                let rs = (a.create_stream.1)();
                (a.reset_stream.0)(cs, level);
                (a.reset_stream.1)(rs, level);
                assert_eq!(
                    (a.load_dict.0)(cs, dict.as_ptr() as *const c_char, dict_size as c_int),
                    (a.load_dict.1)(rs, dict.as_ptr() as *const c_char, dict_size as c_int),
                    "{}: loadDictHC",
                    label
                );

                // Force the accumulated position into (or just below) overflow.
                for st in [cs, rs] {
                    let c = st as *mut HcCtx;
                    (*c).dict_limit = base;
                    (*c).low_limit = base;
                    (*c).next_to_update = base;
                }
                assert_state_blob_eq(&format!("{}: doctored state parity", label), cs, rs);

                let bound = bound_of(src.len());
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
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
                assert_eq!(cn, rn, "{}: return", label);
                assert!(cn > 0, "{}: must succeed, this is not an error path", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                let vc = view(cs);
                assert_eq!(vc, view(rs), "{}: state", label);
                assert_state_blob_eq(&label, cs, rs);

                if expect_reload {
                    // `LZ4_loadDictHC` re-inits from scratch, so the huge index
                    // base is gone and dictLimit is back near 64 KB + dictSize.
                    assert_eq!(
                        vc.dict_limit,
                        65536 + dict_size as u32,
                        "{}: the >2 GB check must RELOAD the dictionary",
                        label
                    );
                } else {
                    // Not over the limit: the doctored base survives.
                    assert_eq!(
                        vc.dict_limit,
                        base + dict_size as u32,
                        "{}: at or below 2 GB nothing may be reloaded",
                        label
                    );
                }
                assert_eq!((a.free_stream.0)(cs), (a.free_stream.1)(rs));
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 145, 146 — `LZ4HC_compress_generic_dictCtx` (lz4hc.c:1450-1462)
//   * row 145: `position >= 64 KB` silently DROPS the attached dictionary
//              (`ctx->dictCtx = NULL`, lz4hc.c:1452-1456).
//   * row 146: `isStateCompatible` false (one side lz4mid, the other not,
//              lz4hc.c:1434-1439 / 1457) falls back to the `usingDictCtxHc`
//              path instead of the `LZ4_memcpy` fast-table-copy path.
//
// Both are asserted on the exact observable the branch produces: the fast-copy
// path ends in `LZ4HC_setExternalDict`, which NULLs `dictCtx`, whereas the
// `usingDictCtxHc` path leaves it set.
// ===========================================================================

#[test]
fn row_145_attached_dictctx_dropped_past_64kb() {
    let a = api();
    let mut rng = Rng::new(0x145_64B0);
    let dict = src_buf(&mut rng, 3, 65536);
    // Contiguous blocks: 2000 (position 0, <= 4 KB -> usingDictCtxHc),
    // 64000 (position 2000 -> still usingDictCtxHc),
    // 1000 (position 66000 >= 64 KB -> dictCtx dropped).
    let sizes = [2000usize, 64000, 1000];
    let total: usize = sizes.iter().sum();
    let src = src_buf(&mut rng, 4, total);

    unsafe {
        let cds = (a.create_stream.0)();
        let rds = (a.create_stream.1)();
        let cw = (a.create_stream.0)();
        let rw = (a.create_stream.1)();

        for &level in &[3i32, 9, 12] {
            (a.reset_stream.0)(cds, level);
            (a.reset_stream.1)(rds, level);
            assert_eq!(
                (a.load_dict.0)(cds, dict.as_ptr() as *const c_char, 65536),
                (a.load_dict.1)(rds, dict.as_ptr() as *const c_char, 65536)
            );
            (a.reset_stream.0)(cw, level);
            (a.reset_stream.1)(rw, level);
            (a.attach.0)(cw, cds);
            (a.attach.1)(rw, rds);
            assert!(!view(cw).dict_ctx_null, "row 145: dictCtx must start attached");

            let mut off = 0usize;
            for (bi, &bsz) in sizes.iter().enumerate() {
                let label = format!("row 145 level={} block={} size={}", level, bi, bsz);
                let bound = bound_of(bsz);
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.cont.0)(
                    cw,
                    src.as_ptr().add(off) as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    bsz as c_int,
                    bound as c_int,
                );
                let rn = (a.cont.1)(
                    rw,
                    src.as_ptr().add(off) as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    bsz as c_int,
                    bound as c_int,
                );
                assert_eq!(cn, rn, "{}: return", label);
                assert!(cn > 0, "{}: must succeed", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                assert_state_eq(&label, cw, rw);

                let dropped = view(cw).dict_ctx_null;
                assert_eq!(
                    dropped,
                    view(rw).dict_ctx_null,
                    "{}: dictCtx nullness C vs Rust",
                    label
                );
                if bi < 2 {
                    assert!(
                        !dropped,
                        "{}: position < 64 KB must KEEP the attached dictionary",
                        label
                    );
                } else {
                    assert!(
                        dropped,
                        "{}: position >= 64 KB must DROP the attached dictionary",
                        label
                    );
                }
                off += bsz;
            }
        }

        assert_eq!((a.free_stream.0)(cds), (a.free_stream.1)(rds));
        assert_eq!((a.free_stream.0)(cw), (a.free_stream.1)(rw));
    }
}

#[test]
fn row_146_incompatible_dictctx_strategy_uses_slow_path() {
    let a = api();
    let mut rng = Rng::new(0x146_1C00);
    let dict = src_buf(&mut rng, 3, 65536);
    // > 4 KB so the `position == 0 && *srcSizePtr > 4 KB` fast-copy branch is a
    // candidate; only `isStateCompatible` then decides.
    let src = src_buf(&mut rng, 4, 20000);
    let bound = bound_of(src.len());

    unsafe {
        let cds = (a.create_stream.0)();
        let rds = (a.create_stream.1)();
        let cw = (a.create_stream.0)();
        let rw = (a.create_stream.1)();

        // work_level is always lz4hc/lz4opt; dict_level flips the strategy.
        for &work_level in &[3i32, 9, 12] {
            for &(dict_level, compatible) in &[
                (1i32, false), // lz4mid dict vs non-mid work -> INCOMPATIBLE
                (2i32, false),
                (3i32, true), // lz4hc dict -> compatible
                (9i32, true),
                (12i32, true), // lz4opt is also "not lz4mid" -> compatible
            ] {
                let label = format!(
                    "row 146 dict_level={} work_level={} compatible={}",
                    dict_level, work_level, compatible
                );
                (a.reset_stream.0)(cds, dict_level);
                (a.reset_stream.1)(rds, dict_level);
                assert_eq!(
                    (a.load_dict.0)(cds, dict.as_ptr() as *const c_char, 65536),
                    (a.load_dict.1)(rds, dict.as_ptr() as *const c_char, 65536),
                    "{}: loadDictHC",
                    label
                );
                (a.reset_stream.0)(cw, work_level);
                (a.reset_stream.1)(rw, work_level);
                (a.attach.0)(cw, cds);
                (a.attach.1)(rw, rds);

                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.cont.0)(
                    cw,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                );
                let rn = (a.cont.1)(
                    rw,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                );
                assert_eq!(cn, rn, "{}: return", label);
                assert!(cn > 0, "{}: must succeed", label);
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                assert_state_eq(&label, cw, rw);

                // The branch taken is observable: the fast table-copy path runs
                // LZ4HC_setExternalDict, which NULLs dictCtx; the fallback
                // usingDictCtxHc path leaves it attached.
                let dropped = view(cw).dict_ctx_null;
                assert_eq!(
                    dropped,
                    view(rw).dict_ctx_null,
                    "{}: dictCtx nullness C vs Rust",
                    label
                );
                if compatible {
                    assert!(
                        dropped,
                        "{}: compatible states must take the LZ4_memcpy fast path",
                        label
                    );
                } else {
                    assert!(
                        !dropped,
                        "{}: incompatible states must fall back to usingDictCtxHc",
                        label
                    );
                }
            }
        }

        assert_eq!((a.free_stream.0)(cds), (a.free_stream.1)(rds));
        assert_eq!((a.free_stream.0)(cw), (a.free_stream.1)(rw));
    }
}

// ===========================================================================
// ERRORS.md row 147 — `LZ4_compress_HC_extStateHC_fastReset` performs NO check
// beyond the alignment test at lz4hc.c:1503, so a state that was never
// initialised is used as-is.
//
// Two DETERMINISTIC "garbage" fills are exercised, because for both of them the
// C's behaviour is fully defined:
//   * all-zero  -> `s->dirty == 0`, `end == prefixStart == NULL`, so the cheap
//                  reset runs (`dictLimit += 0`) and the call must succeed;
//   * all-0xAA  -> `s->dirty != 0`, so `LZ4_resetStreamHC_fast` takes the
//                  `LZ4_initStreamHC` full-reset branch and the call must also
//                  succeed.
// The third sub-case (dirty == 0 with an incoherent end/prefixStart pair) is
// NOT exercised — see the coverage map.
// ===========================================================================

#[test]
fn row_147_fastreset_accepts_uninitialised_state() {
    let a = api();
    let mut rng = Rng::new(0x147_6A26);
    let src = src_buf(&mut rng, 4, 6000);
    let bound = bound_of(src.len());

    for &fill in &[0x00u8, 0xAAu8] {
        for &level in &[c_int::MIN, 1, 2, 3, 9, 12, c_int::MAX] {
            let label = format!("row 147 fill={:#04x} level={}", fill, level);
            let mut cst = state_buf();
            let mut rst = state_buf();
            unsafe {
                std::ptr::write_bytes(cst.as_mut_ptr(), fill, HC_STREAM_BYTES);
                std::ptr::write_bytes(rst.as_mut_ptr(), fill, HC_STREAM_BYTES);
                let mut cd = vec![SENTINEL; bound + 32];
                let mut rd = vec![SENTINEL; bound + 32];
                let cn = (a.ext_state_fast.0)(
                    cst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                let rn = (a.ext_state_fast.1)(
                    rst.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rd.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    level,
                );
                assert_eq!(cn, rn, "{}: return", label);
                assert!(
                    cn > 0,
                    "{}: fastReset must NOT reject an uninitialised state",
                    label
                );
                assert_bytes_eq(&format!("{}: dst", label), &cd, &rd);
                assert_state_blob_eq(
                    &label,
                    cst.as_ptr() as *const c_void,
                    rst.as_ptr() as *const c_void,
                );
                assert_eq!(
                    level_of(cst.as_ptr() as *const c_void),
                    clamped_level(level) as i16,
                    "{}: stored level",
                    label
                );
            }
        }
    }
}

// ###########################################################################
// #                              xxhash.c                                   #
// #  ERRORS.md rows 148-167. `XXH_errorcode` is `{ XXH_OK = 0, XXH_ERROR }`  #
// #  (xxhash.h:79) and every symbol carries the `LZ4_` namespace prefix.     #
// ###########################################################################

/// `sizeof(XXH32_state_t)` on this target (total_len_32, large_len, v1..v4,
/// mem32[4], memsize, reserved — all uint32_t).
const XXH32_STATE_SIZE: usize = 48;
/// `sizeof(XXH64_state_t)` on this target (total_len, v1..v4, mem64[4] as
/// uint64_t, memsize + reserved[2] as uint32_t, plus 4 bytes tail padding).
const XXH64_STATE_SIZE: usize = 88;

struct XApi {
    h32: (FnXXH32, FnXXH32),
    h64: (FnXXH64, FnXXH64),
    create32: (FnCreateState, FnCreateState),
    free32: (FnFreeState, FnFreeState),
    reset32: (FnReset32, FnReset32),
    update32: (FnUpdate, FnUpdate),
    digest32: (FnDigest32, FnDigest32),
    create64: (FnCreateState, FnCreateState),
    free64: (FnFreeState, FnFreeState),
    reset64: (FnReset64, FnReset64),
    update64: (FnUpdate, FnUpdate),
    digest64: (FnDigest64, FnDigest64),
}

fn xapi() -> &'static XApi {
    static X: std::sync::OnceLock<XApi> = std::sync::OnceLock::new();
    X.get_or_init(|| XApi {
        h32: both("LZ4_XXH32"),
        h64: both("LZ4_XXH64"),
        create32: both("LZ4_XXH32_createState"),
        free32: both("LZ4_XXH32_freeState"),
        reset32: both("LZ4_XXH32_reset"),
        update32: both("LZ4_XXH32_update"),
        digest32: both("LZ4_XXH32_digest"),
        create64: both("LZ4_XXH64_createState"),
        free64: both("LZ4_XXH64_freeState"),
        reset64: both("LZ4_XXH64_reset"),
        update64: both("LZ4_XXH64_update"),
        digest64: both("LZ4_XXH64_digest"),
    })
}

/// A never-empty static buffer, for whenever a *valid, non-NULL* pointer is
/// needed with length 0 (an empty `Vec`'s pointer is dangling).
static ZERO_PAD: [u8; 64] = [0u8; 64];

/// Allocate a state through the library's own `createState` and pre-fill it with
/// the SAME sentinel in both libraries, so the `reserved` tail that `reset()`
/// deliberately does not write is still byte-comparable.
unsafe fn new_state(create: FnCreateState, size: usize) -> *mut c_void {
    let p = create();
    assert!(!p.is_null(), "createState returned NULL");
    std::ptr::write_bytes(p as *mut u8, SENTINEL, size);
    p
}

unsafe fn state_bytes(p: *const c_void, size: usize) -> &'static [u8] {
    std::slice::from_raw_parts(p as *const u8, size)
}

// ===========================================================================
// ERRORS.md rows 148, 149, 150 — the ONLY validating xxhash functions
//   * row 148: `LZ4_XXH32_update(state, NULL, len)` -> XXH_ERROR (1) for ANY
//              len, including 0, because `XXH_ACCEPT_NULL_INPUT_POINTER` is 0
//              (xxhash.c:454-459). The state must be left untouched.
//   * row 149: the same for `LZ4_XXH64_update` (xxhash.c:914-919).
//   * row 150: with a valid pointer every other path returns XXH_OK (0)
//              (xxhash.c:470, 511) — there is NO length/overflow rejection.
//
// (`tests/xxhash_diff.rs::xxh_error_paths_null_pointers` covers the same NULL
// rejection from the digest-continuity angle.)
// ===========================================================================

#[test]
fn row_148_149_150_update_null_input_is_the_only_rejection() {
    let x = xapi();
    let mut rng = Rng::new(0x148_150);
    let payload = gen_random(&mut rng, 8192);

    unsafe {
        // ---------------- row 148: XXH32_update(NULL) ----------------------
        let cs = new_state(x.create32.0, XXH32_STATE_SIZE);
        let rs = new_state(x.create32.1, XXH32_STATE_SIZE);
        assert_eq!((x.reset32.0)(cs, 0x1234_5678), XXH_OK);
        assert_eq!((x.reset32.1)(rs, 0x1234_5678), XXH_OK);

        for &len in &[
            0usize,
            1,
            2,
            15,
            16,
            17,
            31,
            32,
            1024,
            usize::MAX / 2,
            usize::MAX,
        ] {
            let before_c = state_bytes(cs, XXH32_STATE_SIZE).to_vec();
            let before_r = state_bytes(rs, XXH32_STATE_SIZE).to_vec();
            let cv = (x.update32.0)(cs, std::ptr::null(), len);
            let rv = (x.update32.1)(rs, std::ptr::null(), len);
            assert_eq!(
                cv, XXH_ERROR,
                "row 148: XXH32_update(state, NULL, {}) must return exactly XXH_ERROR (1)",
                len
            );
            assert_eq!(cv, rv, "row 148: XXH32_update(NULL,{}) C vs Rust", len);
            assert_bytes_eq(
                &format!("row 148: C state modified by update(NULL,{})", len),
                &before_c,
                state_bytes(cs, XXH32_STATE_SIZE),
            );
            assert_bytes_eq(
                &format!("row 148: Rust state modified by update(NULL,{})", len),
                &before_r,
                state_bytes(rs, XXH32_STATE_SIZE),
            );
            assert_bytes_eq(
                &format!("row 148: state parity after update(NULL,{})", len),
                state_bytes(cs, XXH32_STATE_SIZE),
                state_bytes(rs, XXH32_STATE_SIZE),
            );
        }

        // ---------------- row 150: valid pointer -> always XXH_OK -----------
        for &len in &[0usize, 1, 2, 3, 15, 16, 17, 31, 32, 33, 255, 4096, 8192] {
            let p = if len == 0 {
                ZERO_PAD.as_ptr()
            } else {
                payload.as_ptr()
            };
            let cv = (x.update32.0)(cs, p as *const c_void, len);
            let rv = (x.update32.1)(rs, p as *const c_void, len);
            assert_eq!(
                cv, XXH_OK,
                "row 150: XXH32_update(state, valid, {}) must return exactly XXH_OK (0)",
                len
            );
            assert_eq!(cv, rv, "row 150: XXH32_update(valid,{}) C vs Rust", len);
            assert_bytes_eq(
                &format!("row 150: state parity after update(valid,{})", len),
                state_bytes(cs, XXH32_STATE_SIZE),
                state_bytes(rs, XXH32_STATE_SIZE),
            );
        }
        let cd = (x.digest32.0)(cs);
        let rd = (x.digest32.1)(rs);
        assert_eq!(cd, rd, "row 150: digest after the mixed sequence");
        assert_eq!((x.free32.0)(cs), XXH_OK);
        assert_eq!((x.free32.1)(rs), XXH_OK);

        // ---------------- row 149: XXH64_update(NULL) ----------------------
        let cs = new_state(x.create64.0, XXH64_STATE_SIZE);
        let rs = new_state(x.create64.1, XXH64_STATE_SIZE);
        assert_eq!((x.reset64.0)(cs, 0x0123_4567_89AB_CDEF), XXH_OK);
        assert_eq!((x.reset64.1)(rs, 0x0123_4567_89AB_CDEF), XXH_OK);

        for &len in &[
            0usize,
            1,
            2,
            31,
            32,
            33,
            63,
            64,
            1024,
            usize::MAX / 2,
            usize::MAX,
        ] {
            let before_c = state_bytes(cs, XXH64_STATE_SIZE).to_vec();
            let before_r = state_bytes(rs, XXH64_STATE_SIZE).to_vec();
            let cv = (x.update64.0)(cs, std::ptr::null(), len);
            let rv = (x.update64.1)(rs, std::ptr::null(), len);
            assert_eq!(
                cv, XXH_ERROR,
                "row 149: XXH64_update(state, NULL, {}) must return exactly XXH_ERROR (1)",
                len
            );
            assert_eq!(cv, rv, "row 149: XXH64_update(NULL,{}) C vs Rust", len);
            assert_bytes_eq(
                &format!("row 149: C state modified by update(NULL,{})", len),
                &before_c,
                state_bytes(cs, XXH64_STATE_SIZE),
            );
            assert_bytes_eq(
                &format!("row 149: Rust state modified by update(NULL,{})", len),
                &before_r,
                state_bytes(rs, XXH64_STATE_SIZE),
            );
        }
        // row 150's counterpart for XXH64: valid pointer -> always XXH_OK.
        for &len in &[0usize, 1, 31, 32, 33, 4096, 8192] {
            let p = if len == 0 {
                ZERO_PAD.as_ptr()
            } else {
                payload.as_ptr()
            };
            let cv = (x.update64.0)(cs, p as *const c_void, len);
            let rv = (x.update64.1)(rs, p as *const c_void, len);
            assert_eq!(cv, XXH_OK, "XXH64_update(valid,{}) must be XXH_OK", len);
            assert_eq!(cv, rv, "XXH64_update(valid,{}) C vs Rust", len);
        }
        assert_bytes_eq(
            "rows 149/150: XXH64 state parity",
            state_bytes(cs, XXH64_STATE_SIZE),
            state_bytes(rs, XXH64_STATE_SIZE),
        );
        assert_eq!((x.digest64.0)(cs), (x.digest64.1)(rs));
        assert_eq!((x.free64.0)(cs), XXH_OK);
        assert_eq!((x.free64.1)(rs), XXH_OK);
    }
}

// ===========================================================================
// ERRORS.md row 151 — `state->total_len_32 += (unsigned)len` is a 32-bit
// accumulator (xxhash.c:464): past 4 GiB of input the length silently WRAPS mod
// 2^32 and the digest reflects the wrapped value. There is no error.
//
// 4 GiB is not hashed here. The condition is constructed exactly by writing the
// `total_len_32` counter (offset 0 of XXH32_state_t) and the sticky `large_len`
// flag (offset 4) directly, with the IDENTICAL bytes in both libraries, and then
// performing a normal 32-byte update. Asserted: the counter really wrapped to
// the exact expected value in BOTH libraries, and the digests agree.
//
// XXH64 has no counterpart: `total_len` is a 64-bit field (xxhash.c:917).
// ===========================================================================

#[test]
fn row_151_xxh32_total_len_wraparound() {
    let x = xapi();
    let mut rng = Rng::new(0x151_2AF0);
    let chunk = gen_random(&mut rng, 32);

    unsafe {
        for &(pre, add) in &[
            (0xFFFF_FFFFu32, 32usize), // wraps to 31
            (0xFFFF_FFF0u32, 32usize), // wraps to 16
            (0xFFFF_FFE0u32, 32usize), // wraps to exactly 0
            (0xFFFF_FFE1u32, 32usize), // wraps to 1
            (0x8000_0000u32, 32usize), // no wrap (control)
        ] {
            let want = pre.wrapping_add(add as u32);
            let cs = new_state(x.create32.0, XXH32_STATE_SIZE);
            let rs = new_state(x.create32.1, XXH32_STATE_SIZE);
            assert_eq!((x.reset32.0)(cs, 0), XXH_OK);
            assert_eq!((x.reset32.1)(rs, 0), XXH_OK);

            // Pre-load the 32-bit length accumulator and the sticky large_len
            // flag exactly as >= 4 GiB of prior input would have left them.
            for st in [cs, rs] {
                let f = st as *mut u32;
                *f.add(0) = pre; // total_len_32
                *f.add(1) = 1; // large_len (sticky |=)
                *f.add(10) = 0; // memsize -> empty accumulation buffer
            }
            assert_bytes_eq(
                "row 151: doctored state parity",
                state_bytes(cs, XXH32_STATE_SIZE),
                state_bytes(rs, XXH32_STATE_SIZE),
            );

            let cv = (x.update32.0)(cs, chunk.as_ptr() as *const c_void, add);
            let rv = (x.update32.1)(rs, chunk.as_ptr() as *const c_void, add);
            assert_eq!(cv, XXH_OK, "row 151: no error may be reported on wrap");
            assert_eq!(cv, rv, "row 151: update return C vs Rust");

            let got_c = *(cs as *const u32);
            let got_r = *(rs as *const u32);
            assert_eq!(
                got_c, want,
                "row 151: total_len_32 must wrap mod 2^32 ({:#010x} + {} -> {:#010x}), got {:#010x}",
                pre, add, want, got_c
            );
            assert_eq!(got_c, got_r, "row 151: total_len_32 C vs Rust");
            assert_bytes_eq(
                "row 151: full state parity after the wrapping update",
                state_bytes(cs, XXH32_STATE_SIZE),
                state_bytes(rs, XXH32_STATE_SIZE),
            );

            // The digest folds in the WRAPPED length, so it must also agree.
            let dc = (x.digest32.0)(cs);
            let dr = (x.digest32.1)(rs);
            assert_eq!(
                dc, dr,
                "row 151: digest after wrap ({:#010x} vs {:#010x})",
                dc, dr
            );
            assert_eq!((x.free32.0)(cs), XXH_OK);
            assert_eq!((x.free32.1)(rs), XXH_OK);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 152, 153, 155, 156 — the remaining `XXH_errorcode` returns
//   * row 152: `LZ4_XXH32_reset` has NO NULL check; for ANY non-NULL state it
//              unconditionally returns XXH_OK (xxhash.c:437-448).
//   * row 153: the same for `LZ4_XXH64_reset` (xxhash.c:907).
//   * row 155: `LZ4_XXH32_freeState(NULL)` -> XXH_OK (`XXH_free(NULL)` is a
//              no-op, xxhash.c:426-430).
//   * row 156: `LZ4_XXH64_freeState(NULL)` -> XXH_OK (xxhash.c:887-891).
//
// (`tests/xxhash_diff.rs::xxh_error_paths_null_pointers` also asserts the two
// freeState(NULL) sentinels.)
// ===========================================================================

#[test]
fn row_152_153_155_156_reset_always_ok_and_freestate_null() {
    let x = xapi();
    unsafe {
        // rows 155 / 156 — freeState(NULL) is XXH_OK, repeatedly.
        for i in 0..4 {
            let cv = (x.free32.0)(std::ptr::null_mut());
            let rv = (x.free32.1)(std::ptr::null_mut());
            assert_eq!(
                cv, XXH_OK,
                "row 155: XXH32_freeState(NULL) must return exactly XXH_OK (0) [{}]",
                i
            );
            assert_eq!(cv, rv, "row 155: XXH32_freeState(NULL) C vs Rust");

            let cv = (x.free64.0)(std::ptr::null_mut());
            let rv = (x.free64.1)(std::ptr::null_mut());
            assert_eq!(
                cv, XXH_OK,
                "row 156: XXH64_freeState(NULL) must return exactly XXH_OK (0) [{}]",
                i
            );
            assert_eq!(cv, rv, "row 156: XXH64_freeState(NULL) C vs Rust");
        }

        // row 152 — XXH32_reset can only ever return XXH_OK, and it must write
        // exactly `sizeof(state) - sizeof(state.reserved)` == 44 bytes, leaving
        // the sentinel-filled tail alone. Both libraries get the same fill.
        let cs = new_state(x.create32.0, XXH32_STATE_SIZE);
        let rs = new_state(x.create32.1, XXH32_STATE_SIZE);
        for seed in [
            0u32,
            1,
            0x8000_0000,
            0xFFFF_FFFF,
            0x9E37_79B1,
            0x1EDE_1234,
        ] {
            let cv = (x.reset32.0)(cs, seed);
            let rv = (x.reset32.1)(rs, seed);
            assert_eq!(
                cv, XXH_OK,
                "row 152: XXH32_reset(state, {:#010x}) must return exactly XXH_OK (0)",
                seed
            );
            assert_eq!(cv, rv, "row 152: XXH32_reset C vs Rust");
            assert_bytes_eq(
                &format!("row 152: state after reset({:#010x})", seed),
                state_bytes(cs, XXH32_STATE_SIZE),
                state_bytes(rs, XXH32_STATE_SIZE),
            );
            // The last 4 bytes are `reserved` and must still hold the sentinel.
            assert_eq!(
                &state_bytes(cs, XXH32_STATE_SIZE)[44..],
                &[SENTINEL; 4],
                "row 152: reset must not write `reserved`"
            );
        }
        assert_eq!((x.free32.0)(cs), XXH_OK);
        assert_eq!((x.free32.1)(rs), XXH_OK);

        // row 153 — XXH64_reset, same argument (80 of 88 bytes written).
        let cs = new_state(x.create64.0, XXH64_STATE_SIZE);
        let rs = new_state(x.create64.1, XXH64_STATE_SIZE);
        for seed in [
            0u64,
            1,
            0xFFFF_FFFF,
            0xFFFF_FFFF_FFFF_FFFF,
            0x8000_0000_0000_0000,
            0x9E37_79B1_85EB_CA87,
        ] {
            let cv = (x.reset64.0)(cs, seed);
            let rv = (x.reset64.1)(rs, seed);
            assert_eq!(
                cv, XXH_OK,
                "row 153: XXH64_reset(state, {:#018x}) must return exactly XXH_OK (0)",
                seed
            );
            assert_eq!(cv, rv, "row 153: XXH64_reset C vs Rust");
            assert_bytes_eq(
                &format!("row 153: state after reset({:#018x})", seed),
                state_bytes(cs, XXH64_STATE_SIZE),
                state_bytes(rs, XXH64_STATE_SIZE),
            );
            assert_eq!(
                &state_bytes(cs, XXH64_STATE_SIZE)[80..],
                &[SENTINEL; 8],
                "row 153: reset must not write `reserved`"
            );
        }
        assert_eq!((x.free64.0)(cs), XXH_OK);
        assert_eq!((x.free64.1)(rs), XXH_OK);
    }
}

// ===========================================================================
// ERRORS.md row 160 — `LZ4_XXH32(NULL, 0, seed)` does NOT crash and does NOT
// signal: `len >= 16` is false and `XXH32_finalize` with `(len & 15) == 0` hits
// `case 0: return XXH32_avalanche(h32)` before touching `p` (xxhash.c:366-388,
// 344). The result is the seed-only hash.
//
// Row 161's counterpart boundary (`LZ4_XXH64(NULL, 0, seed)`) is asserted the
// same way: `XXH64_endian_align` never dereferences `p` at len 0 either
// (xxhash.c:806-830). Only `len > 0` with a NULL pointer is a dereference, and
// that is what rows 159/161 describe — see the coverage map.
// ===========================================================================

#[test]
fn row_160_161_oneshot_null_pointer_zero_length() {
    let x = xapi();
    unsafe {
        for seed in [0u32, 1, 0x8000_0000, 0xFFFF_FFFF, 0x9E37_79B1] {
            let cv = (x.h32.0)(std::ptr::null(), 0, seed);
            let rv = (x.h32.1)(std::ptr::null(), 0, seed);
            assert_eq!(
                cv, rv,
                "row 160: LZ4_XXH32(NULL,0,{:#010x}) C={:#010x} Rust={:#010x}",
                seed, cv, rv
            );
            // No sentinel exists (the return type is `unsigned`): the documented
            // result is exactly the hash of a zero-length buffer.
            let cz = (x.h32.0)(ZERO_PAD.as_ptr() as *const c_void, 0, seed);
            let rz = (x.h32.1)(ZERO_PAD.as_ptr() as *const c_void, 0, seed);
            assert_eq!(cz, rz, "row 160: LZ4_XXH32(valid,0,..) C vs Rust");
            assert_eq!(
                cv, cz,
                "row 160: LZ4_XXH32(NULL,0,{:#010x}) must equal the empty-input hash",
                seed
            );
        }
        for seed in [0u64, 1, 0xFFFF_FFFF_FFFF_FFFF, 0x9E37_79B1_85EB_CA87] {
            let cv = (x.h64.0)(std::ptr::null(), 0, seed);
            let rv = (x.h64.1)(std::ptr::null(), 0, seed);
            assert_eq!(
                cv, rv,
                "row 161 boundary: LZ4_XXH64(NULL,0,{:#018x}) C={:#018x} Rust={:#018x}",
                seed, cv, rv
            );
            let cz = (x.h64.0)(ZERO_PAD.as_ptr() as *const c_void, 0, seed);
            let rz = (x.h64.1)(ZERO_PAD.as_ptr() as *const c_void, 0, seed);
            assert_eq!(cz, rz, "row 161 boundary: LZ4_XXH64(valid,0,..) C vs Rust");
            assert_eq!(
                cv, cz,
                "row 161 boundary: LZ4_XXH64(NULL,0,..) must equal the empty-input hash"
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 165, 166 — the `assert(0)` fall-through of `XXH32_finalize`
// (xxhash.c:346-347) and `XXH64_finalize` (xxhash.c:805-807).
//
// Both switches enumerate EVERY possible value of `len & 15` / `len & 31`, so
// the fall-through is unreachable. This test proves it operationally: every
// residue class is driven through BOTH the one-shot and the streaming entry
// points. NOTE: asserts are compiled OUT in lz4hc.c (see the file header), so a
// reachable `assert(0)`
// would abort the process instead of returning. All values must also produce
// identical digests in C and Rust.
// ===========================================================================

#[test]
fn row_165_166_finalize_switch_covers_every_residue() {
    let x = xapi();
    let mut rng = Rng::new(0x165_166);
    let data = gen_random(&mut rng, 1024);

    unsafe {
        // row 165 — every value of len & 15 for XXH32, one-shot and streaming.
        for len in 0..=64usize {
            for seed in [0u32, 0x9E37_79B1] {
                let cv = (x.h32.0)(data.as_ptr() as *const c_void, len, seed);
                let rv = (x.h32.1)(data.as_ptr() as *const c_void, len, seed);
                assert_eq!(
                    cv, rv,
                    "row 165: LZ4_XXH32(len={} -> len&15={}) C={:#010x} Rust={:#010x}",
                    len,
                    len & 15,
                    cv,
                    rv
                );

                let cs = new_state(x.create32.0, XXH32_STATE_SIZE);
                let rs = new_state(x.create32.1, XXH32_STATE_SIZE);
                assert_eq!((x.reset32.0)(cs, seed), XXH_OK);
                assert_eq!((x.reset32.1)(rs, seed), XXH_OK);
                assert_eq!(
                    (x.update32.0)(cs, data.as_ptr() as *const c_void, len),
                    XXH_OK
                );
                assert_eq!(
                    (x.update32.1)(rs, data.as_ptr() as *const c_void, len),
                    XXH_OK
                );
                let dc = (x.digest32.0)(cs);
                let dr = (x.digest32.1)(rs);
                assert_eq!(dc, dr, "row 165: XXH32_digest(len={}) C vs Rust", len);
                assert_eq!(dc, cv, "row 165: streaming must match one-shot (len={})", len);
                assert_eq!((x.free32.0)(cs), XXH_OK);
                assert_eq!((x.free32.1)(rs), XXH_OK);
            }
        }

        // row 166 — every value of len & 31 for XXH64.
        for len in 0..=96usize {
            for seed in [0u64, 0x9E37_79B1_85EB_CA87] {
                let cv = (x.h64.0)(data.as_ptr() as *const c_void, len, seed);
                let rv = (x.h64.1)(data.as_ptr() as *const c_void, len, seed);
                assert_eq!(
                    cv, rv,
                    "row 166: LZ4_XXH64(len={} -> len&31={}) C={:#018x} Rust={:#018x}",
                    len,
                    len & 31,
                    cv,
                    rv
                );

                let cs = new_state(x.create64.0, XXH64_STATE_SIZE);
                let rs = new_state(x.create64.1, XXH64_STATE_SIZE);
                assert_eq!((x.reset64.0)(cs, seed), XXH_OK);
                assert_eq!((x.reset64.1)(rs, seed), XXH_OK);
                assert_eq!(
                    (x.update64.0)(cs, data.as_ptr() as *const c_void, len),
                    XXH_OK
                );
                assert_eq!(
                    (x.update64.1)(rs, data.as_ptr() as *const c_void, len),
                    XXH_OK
                );
                let dc = (x.digest64.0)(cs);
                let dr = (x.digest64.1)(rs);
                assert_eq!(dc, dr, "row 166: XXH64_digest(len={}) C vs Rust", len);
                assert_eq!(dc, cv, "row 166: streaming must match one-shot (len={})", len);
                assert_eq!((x.free64.0)(cs), XXH_OK);
                assert_eq!((x.free64.1)(rs), XXH_OK);
            }
        }
    }
}

// ===========================================================================
// ROW-BY-ROW COVERAGE MAP — ERRORS.md rows 94-167
//
// lz4hc.c
// -------
//  94 -> row_94_95_hc_level_silently_clamped_to_9_and_12
//        + row_94_95_deprecated_wrappers_hardcode_level_zero
//        + row_94_95_destsize_level_clamp
//  95 -> row_94_95_hc_level_silently_clamped_to_9_and_12
//        + row_94_95_deprecated_wrappers_hardcode_level_zero
//        + row_94_95_destsize_level_clamp
//  96 -> row_96_97_98_stored_level_clamped
//  97 -> row_96_97_98_stored_level_clamped
//  98 -> row_96_97_98_stored_level_clamped
//  99 -> row_99_100_srcsize_out_of_range_every_entry_point
// 100 -> row_99_100_srcsize_out_of_range_every_entry_point
// 101 -> row_101_102_103_destsize_rejections
// 102 -> row_101_102_103_destsize_rejections
// 103 -> row_101_102_103_destsize_rejections
// 104 -> row_104_105_106_108_extstatehc_and_destsize_bad_state
// 105 -> row_104_105_106_108_extstatehc_and_destsize_bad_state
// 106 -> row_104_105_106_108_extstatehc_and_destsize_bad_state
// 107 -> NOT TESTED. `LZ4HC_HEAPMODE == 1` (lz4hc.c:47-49), so
//        `LZ4_compress_HC` does `ALLOC(sizeof(LZ4_streamHC_t))` (262200 bytes)
//        at lz4hc.c:1523 and returns 0 if it fails (1524). Forcing that malloc
//        to fail requires an allocator hook, which the API does not provide;
//        the branch cannot be reached from a caller. (This is the direct
//        analogue of lz4.c rows 21/22 and 29, documented the same way in
//        tests/lz4_errors.rs.)
// 108 -> row_104_105_106_108_extstatehc_and_destsize_bad_state
// 109 -> row_109_110_111_init_stream_hc_returns_null
// 110 -> row_109_110_111_init_stream_hc_returns_null
// 111 -> row_109_110_111_init_stream_hc_returns_null
// 112 -> NOT TESTED. `ALLOC_AND_ZERO(sizeof(LZ4_streamHC_t))` failure inside
//        `LZ4_createStreamHC` (lz4hc.c:1556-1558). Same reason as row 107: the
//        allocation cannot be made to fail through the public API.
// 113 -> row_113_114_116_free_null_and_reset_stream_state_hc
// 114 -> row_113_114_116_free_null_and_reset_stream_state_hc
// 115 -> NOT TESTED. `LZ4_createHC` returns NULL only when
//        `LZ4_createStreamHC()` returns NULL (lz4hc.c:2161-2162), i.e. row 112's
//        un-forceable allocation failure.
// 116 -> row_113_114_116_free_null_and_reset_stream_state_hc
//        (returns exactly 1 on error — the inverted convention — and exactly 0
//        on success; the "smaller than sizeof(LZ4_streamHC_t)" trigger is not
//        expressible because the function hard-codes `sizeof(*hc4)`)
// 117 -> row_117_negative_dstcapacity_returns_zero
// 118 -> NOT TESTED. `if (*srcSizePtr < 0) return 0;` inside `LZ4MID_compress`
//        (lz4hc.c:559) is unreachable through every public entry point: the
//        unsigned check at lz4hc.c:1389 rejects negative sizes first (row 99).
//        In addition the live `assert(*srcSizePtr >= 0)` at lz4hc.c:556 sits
//        BEFORE it, so if the branch were somehow reached the C build (compiled
//        is compiled out in lz4hc.c, so this is UB rather than an abort) —
//        either way it is not a comparable behaviour.
// 119 -> NOT TESTED. `if (*srcSizePtr > LZ4_MAX_INPUT_SIZE) return 0;` at
//        lz4hc.c:561-564 is an exact duplicate of the lz4hc.c:1389 guard, which
//        every public entry point hits first (row 100), so the duplicate is
//        unreachable.
// 120 -> row_120_lz4mid_last_literals_do_not_fit
// 121 -> row_121_lz4mid_midstream_dest_overflow
// 122 -> row_122_123_125_hashchain_encode_sequence_overflow
// 123 -> row_122_123_125_hashchain_encode_sequence_overflow
// 124 -> row_124_hashchain_last_literals_do_not_fit
// 125 -> row_122_123_125_hashchain_encode_sequence_overflow
// 126 -> row_126_127_optimal_parser_overflow
// 127 -> row_126_127_optimal_parser_overflow
// 128 -> NOT TESTED. `LZ4HC_HEAPMODE == 1` makes
//        `ALLOC(sizeof(LZ4HC_optimal_t) * (LZ4_OPT_NUM+3))` (~64 KB) at
//        lz4hc.c:1838 the only way to reach `if (opt == NULL) goto
//        _return_label;` (1855-1856). Same un-forceable allocation failure as
//        rows 107/112.
// 129 -> row_129_optimal_sufficient_len_clamped_to_opt_num_minus_one
// 130 -> row_130_destsize_truncates_input_instead_of_failing
// 131 -> row_131_continue_notlimited_threshold_is_compressbound
// 132 -> row_132_compresshc2_continue_has_no_output_bound_check
//        (levels 1-2 excluded: `LZ4MID_compress` ends with the live
//        `assert(op <= oend)` at lz4hc.c:743 while `oend == dst + 0`, which
//        aborts the non-NDEBUG C build. `LZ4_compressHC2_limitedOutput_continue`
//        does NOT hard-code the capacity — lz4hc.c:2182 forwards the caller's
//        `dstCapacity` with `limitedOutput` — and is used as the control.)
// 133 -> row_133_failed_compression_marks_stream_dirty
// 134 -> row_134_load_dict_hc_truncates_to_last_64kb
// 135 -> row_135_136_load_dict_hc_too_small_leaves_tables_empty
// 136 -> row_135_136_load_dict_hc_too_small_leaves_tables_empty
// 137 -> NOT TESTED. `LZ4_loadDictHC` with `dictSize < 0` is guarded ONLY by the
//        live `assert(dictSize >= 0)` at lz4hc.c:1632, which fires and aborts
//        the C into undefined behaviour (the assert is compiled out in lz4hc.c,
//        so the negative size is cast to a huge size_t and read out of bounds).
//        Row 137 itself documents the
//        release behaviour as UB (`ctxPtr->end` set from a negative size).
// 138 -> row_138_139_140_141_save_dict_hc_clamps
// 139 -> row_138_139_140_141_save_dict_hc_clamps
// 140 -> row_138_139_140_141_save_dict_hc_clamps
// 141 -> row_138_139_140_141_save_dict_hc_clamps covers the WELL-DEFINED half
//        (a NULL `safeBuffer` whose `dictSize` clamps to 0, so the assert
//        holds). The other half — a NULL `safeBuffer` with a non-zero clamped
//        `dictSize` — is guarded only by the live `assert(dictSize == 0)` at
//        lz4hc.c:1751, which aborts the C build; row 141 documents the release
//        behaviour as UB (`LZ4_memmove` to NULL).
// 142 -> row_142_attach_hc_dictionary_null_detaches
// 143 -> row_143_continue_src_overlaps_extdict
// 144 -> row_144_continue_two_gigabyte_position_overflow_reloads_dict
// 145 -> row_145_attached_dictctx_dropped_past_64kb
// 146 -> row_146_incompatible_dictctx_strategy_uses_slow_path
// 147 -> row_147_fastreset_accepts_uninitialised_state covers the two
//        deterministic "not correctly initialized" fills (all-zero -> cheap
//        reset, all-0xAA -> `dirty != 0` -> full re-init). The remaining
//        sub-case, a state whose `dirty` byte happens to be 0 while
//        `end`/`prefixStart` hold incoherent garbage, is guarded only by the
//        live `assert(s->end >= s->prefixStart)` at lz4hc.c:1597 and would abort
//        the C build; row 147 itself classifies it as UB.
//
// xxhash.c
// --------
// 148 -> row_148_149_150_update_null_input_is_the_only_rejection
// 149 -> row_148_149_150_update_null_input_is_the_only_rejection
// 150 -> row_148_149_150_update_null_input_is_the_only_rejection
//        (the "huge len" half of row 150 is covered up to 8192; a genuinely
//        huge length with a valid pointer would need a matching allocation,
//        which is what row 151 documents instead)
// 151 -> row_151_xxh32_total_len_wraparound
// 152 -> row_152_153_155_156_reset_always_ok_and_freestate_null asserts the
//        testable half (any non-NULL state returns exactly XXH_OK). The
//        `statePtr == NULL` half is NOT tested: xxhash.c:437-448 has no NULL
//        check at all and `memcpy(statePtr, ...)` writes through the pointer, so
//        the call is an unconditional NULL store — it faults both libraries
//        identically and proves nothing.
// 153 -> row_152_153_155_156_reset_always_ok_and_freestate_null (same split;
//        the NULL half is the `memcpy` at xxhash.c:907)
// 154 -> NOT TESTED. `LZ4_XXH32_copyState` / `LZ4_XXH64_copyState` are a bare
//        `memcpy` with no checks (xxhash.c:432-435, 893-896), so a NULL `dst`
//        or `src` is an unconditional NULL access in both libraries.
// 155 -> row_152_153_155_156_reset_always_ok_and_freestate_null
// 156 -> row_152_153_155_156_reset_always_ok_and_freestate_null
// 157 -> NOT TESTED. `XXH_malloc(sizeof(XXH32_state_t))` failure inside
//        `LZ4_XXH32_createState` (xxhash.c:422-425). Not forceable without an
//        allocator hook, which xxhash 0.6.5 does not expose.
// 158 -> NOT TESTED. Same for `LZ4_XXH64_createState` (xxhash.c:883-886).
// 159 -> NOT TESTED. With `XXH_ACCEPT_NULL_INPUT_POINTER == 0` the NULL guard at
//        xxhash.c:359-364 is compiled out, so `LZ4_XXH32(NULL, len>0, seed)`
//        dereferences NULL in the main loop / `XXH32_finalize`. There is no
//        sentinel to compare (the return type is `unsigned`) and the call faults
//        both libraries identically. The defined boundary (`len == 0`) is
//        asserted by row_160_161_oneshot_null_pointer_zero_length.
// 160 -> row_160_161_oneshot_null_pointer_zero_length
// 161 -> NOT TESTED for `len > 0` — the guard at xxhash.c:818-823 is compiled
//        out and the call dereferences NULL, exactly as for row 159. The
//        `len == 0` boundary is asserted by
//        row_160_161_oneshot_null_pointer_zero_length.
// 162 -> NOT TESTED. `LZ4_XXH32_digest` / `LZ4_XXH64_digest` read
//        `state_in->...` with no NULL check (xxhash.c:531-542, 985-1002) and
//        have no error value in their return type, so a NULL state is an
//        unconditional NULL read in both libraries.
// 163 -> NOT TESTED. `LZ4_XXH32_hashFromCanonical` /
//        `LZ4_XXH64_hashFromCanonical` do a direct `XXH_readBE32/64(src)` with
//        no check (xxhash.c:572-575, 1025-1028) — NULL read, no sentinel.
// 164 -> NOT TESTED. `LZ4_XXH32_canonicalFromHash` /
//        `LZ4_XXH64_canonicalFromHash` do a direct `memcpy(dst, ...)` with no
//        check (xxhash.c:565-570, 1018-1023) — NULL store, `void` return.
// 165 -> row_165_166_finalize_switch_covers_every_residue proves the
//        `assert(0)` at xxhash.c:346-347 is unreachable: all 16 values of
//        `len & 15` are enumerated by the switch and every one of them is
//        driven here. Because asserts are compiled out in lz4hc.c, a reachable
//        fall-through would abort instead of returning.
// 166 -> row_165_166_finalize_switch_covers_every_residue, same argument for
//        the 32 values of `len & 31` and the `assert(0)` at xxhash.c:805-807.
// 167 -> NOT TESTED (compile-time). `XXH_STATIC_ASSERT(sizeof(XXH32_canonical_t)
//        == sizeof(XXH32_hash_t))` at xxhash.c:567 (and the 64-bit counterpart
//        at 1020) is a compile-time division by zero. Both libraries built
//        successfully, which is itself the proof the assertion holds.
// ===========================================================================
