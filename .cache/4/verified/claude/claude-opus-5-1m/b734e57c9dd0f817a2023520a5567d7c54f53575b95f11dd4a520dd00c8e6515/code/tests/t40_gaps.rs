//! Final coverage-gap closure for the C-vs-Rust differential suite.
//!
//! Everything here drives *both* shared objects through the FFI boundary and
//! compares the observable result with `diff` / `diff_bytes`. Each test's doc
//! comment names the C function and branch it targets plus the `CONFIGS.md` /
//! `ERRORS.md` rows it closes.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Local FFI signatures
// ---------------------------------------------------------------------------

type FnGetErrStr = unsafe extern "C" fn(c_int) -> *const c_char;
type FnU32Ret = unsafe extern "C" fn() -> c_uint;
type FnWriteLastEmptyBlock = unsafe extern "C" fn(*mut c_void, SizeT) -> SizeT;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnRegSeqProd = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void);
type FnCompressAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
    ZSTD_parameters,
) -> SizeT;
type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut i32, i32, i32) -> i32;
type FnDivbwt =
    unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32, *mut u8, *mut i32, i32) -> i32;

/// `ZSTD_ldm_*` — mirrored from `compress/zstd_compress_internal.h`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct ldmParams_t {
    enableLdm: c_int,
    hashLog: c_uint,
    bucketSizeLog: c_uint,
    minMatchLength: c_uint,
    hashRateLog: c_uint,
    windowLog: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct rawSeq {
    offset: c_uint,
    litLength: c_uint,
    matchLength: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RawSeqStore_t {
    seq: *mut rawSeq,
    pos: SizeT,
    posInSequence: SizeT,
    size: SizeT,
    capacity: SizeT,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ZSTD_window_t {
    nextSrc: *const u8,
    base: *const u8,
    dictBase: *const u8,
    dictLimit: c_uint,
    lowLimit: c_uint,
    nbOverflowCorrections: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct ldmEntry_t {
    offset: c_uint,
    checksum: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ldmMatchCandidate_t {
    split: *const u8,
    hash: c_uint,
    checksum: c_uint,
    bucket: *mut ldmEntry_t,
}

const LDM_BATCH_SIZE: usize = 64;

#[repr(C)]
#[derive(Copy, Clone)]
struct ldmState_t {
    window: ZSTD_window_t,
    hashTable: *mut ldmEntry_t,
    loadedDictEnd: c_uint,
    bucketOffsets: *mut u8,
    splitIndices: [SizeT; LDM_BATCH_SIZE],
    matchCandidates: [ldmMatchCandidate_t; LDM_BATCH_SIZE],
}

type FnLdmAdjust = unsafe extern "C" fn(*mut ldmParams_t, *const ZSTD_compressionParameters);
type FnLdmGetTableSize = unsafe extern "C" fn(ldmParams_t) -> SizeT;
type FnLdmGetMaxNbSeq = unsafe extern "C" fn(ldmParams_t, SizeT) -> SizeT;
type FnLdmGenerate = unsafe extern "C" fn(
    *mut ldmState_t,
    *mut RawSeqStore_t,
    *const ldmParams_t,
    *const c_void,
    SizeT,
) -> SizeT;
type FnLdmSkipSequences = unsafe extern "C" fn(*mut RawSeqStore_t, SizeT, c_uint);
type FnLdmSkipRawBytes = unsafe extern "C" fn(*mut RawSeqStore_t, SizeT);
type FnLdmFillHashTable =
    unsafe extern "C" fn(*mut ldmState_t, *const u8, *const u8, *const ldmParams_t);

type FnLoadCEntropy =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, SizeT) -> SizeT;
type FnFseWriteNCount =
    unsafe extern "C" fn(*mut c_void, SizeT, *const i16, c_uint, c_uint) -> SizeT;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void,
    SizeT,
    c_int,
    c_int,
    *const c_void,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void;
type FnFreeAny = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnCreateCCtxParams = unsafe extern "C" fn() -> *mut c_void;
type FnCCtxParamsSet = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> SizeT;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int, c_int) -> SizeT;
type FnRefCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
type FnInitCStreamInternal = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    SizeT,
    *const c_void,
    *const c_void,
    c_ulonglong,
) -> SizeT;
type FnGetCParamsFromCDict = unsafe extern "C" fn(*const c_void) -> ZSTD_compressionParameters;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;
type FnDecompressionMargin = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnEstFromInt2 = unsafe extern "C" fn(c_int) -> SizeT;
type FnEstFromPtrSz = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnCreateAdvancedMem = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCompressBeginLvl = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnZbuffInit = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnZbuffContinue = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut SizeT,
    *const c_void,
    *mut SizeT,
) -> SizeT;
type FnRefCDict2 = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnCCtxParamsInitAdv = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> SizeT;

// legacy
type FnFsev05ReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnFsev05BuildDTable = unsafe extern "C" fn(*mut c_void, *const i16, c_uint, c_uint) -> SizeT;
type FnHufDecompress = unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnHufDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnReadStats7 = unsafe extern "C" fn(
    *mut u8,
    SizeT,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    SizeT,
) -> SizeT;

// low-level HUF / FSE entry points used by the CONFIGS rows below
type FnFseDecompressWksp = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_uint,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnHufC1XRepeat = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_uint,
    c_uint,
    *mut c_void,
    SizeT,
    *mut u64,
    *mut c_int,
    c_int,
) -> SizeT;
type FnHufDecompressXWksp = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnHufDecompressUsingDTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_void, c_int) -> SizeT;
type FnHufReadDTableWksp = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// A 8-byte-aligned scratch buffer.
fn wksp(bytes: usize) -> Vec<u64> {
    vec![0u64; bytes.div_ceil(8)]
}
fn wksp_ptr(w: &mut [u64]) -> *mut c_void {
    w.as_mut_ptr() as *mut c_void
}
fn wksp_bytes(w: &[u64]) -> SizeT {
    w.len() * 8
}

const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
/// `sizeof(ZSTD_compressedBlockState_t)` is ~5.6 KB; 16 KB is generous and both
/// libraries see the identical zeroed buffer, so the whole thing can be compared.
const BS_SIZE: usize = 16 * 1024;

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

// ===========================================================================
// CONFIGS 310 — the complete ZSTD_ErrorCode surface
// ===========================================================================

/// `ZSTD_getErrorCode` / `ZSTD_getErrorString` / `ZSTD_getErrorName` over every
/// documented `ZSTD_ErrorCode` enumerator, `maxCode`, `maxCode+1`, 12345, and
/// `ZSTD_getErrorCode` over 0, 1, `(size_t)-1`, `(size_t)-2` and every `ERROR(x)`.
///
/// C: `common/error_private.c` `ERR_getErrorString` (a dense switch whose
/// `maxCode` and `default` arms both yield "Unspecified error code") and
/// `ERR_getErrorCode` (a non-error `size_t` maps to `ZSTD_error_no_error`).
///
/// CONFIGS 310.
#[test]
fn error_code_enum_surface_complete() {
    covers(&["CFG:310"]);
    // Every enumerator named in the row, in header order, plus maxCode and the
    // two out-of-enum probes.
    const CODES: &[c_int] = &[
        0,   // no_error
        1,   // GENERIC
        10,  // prefix_unknown
        12,  // version_unsupported
        14,  // frameParameter_unsupported
        16,  // frameParameter_windowTooLarge
        20,  // corruption_detected
        22,  // checksum_wrong
        24,  // literals_headerWrong
        40,  // parameter_unsupported
        41,  // parameter_combination_unsupported
        42,  // parameter_outOfBound
        44,  // tableLog_tooLarge
        46,  // maxSymbolValue_tooLarge
        48,  // maxSymbolValue_tooSmall
        49,  // cannotProduce_uncompressedBlock
        50,  // stabilityCondition_notRespected
        60,  // stage_wrong
        62,  // init_missing
        64,  // memory_allocation
        66,  // workSpace_tooSmall
        70,  // dstSize_tooSmall
        72,  // srcSize_wrong
        74,  // dstBuffer_null
        80,  // noForwardProgress_destFull
        82,  // noForwardProgress_inputEmpty
        30,  // dictionary_corrupted
        32,  // dictionary_wrong
        34,  // dictionaryCreation_failed
        100, // frameIndex_tooLarge
        102, // seekableIO
        104, // dstBuffer_wrong
        105, // srcBuffer_wrong
        106, // sequenceProducer_failed
        107, // externalSequences_invalid
        120, // maxCode
        121, // maxCode + 1
        12345,
    ];
    for &code in CODES {
        diff(&format!("ZSTD_getErrorString({code})"), |l| unsafe {
            cstr(l.sym::<FnGetErrStr>("ZSTD_getErrorString")(code))
        });
        diff(&format!("ERR_getErrorString({code})"), |l| unsafe {
            cstr(l.sym::<FnGetErrStr>("ERR_getErrorString")(code))
        });
        // ERROR(code) == 0 - code, routed back through getErrorCode/Name.
        let enc = 0usize.wrapping_sub(code as usize);
        diff(&format!("roundtrip ERROR({code})"), |l| {
            (
                unsafe { l.sym::<FnIsError>("ZSTD_isError")(enc) },
                unsafe { l.sym::<FnGetErrorCode>("ZSTD_getErrorCode")(enc) },
                err_name(l, enc),
            )
        });
    }
    // getErrorCode over the non-error / boundary size_t values called out.
    for v in [0usize, 1, usize::MAX, usize::MAX - 1, 12345, usize::MAX / 2] {
        diff(&format!("ZSTD_getErrorCode(raw {v})"), |l| {
            (
                unsafe { l.sym::<FnGetErrorCode>("ZSTD_getErrorCode")(v) },
                unsafe { l.sym::<FnIsError>("ZSTD_isError")(v) },
                err_name(l, v),
            )
        });
    }
}

// ===========================================================================
// CONFIGS 70 — ZSTD_compressStream2 argument guards
// ===========================================================================

/// The three `RETURN_ERROR_IF` guards at the top of `ZSTD_compressStream2`
/// (`compress/zstd_compress.c:6449/6450/6451`) and the NULL-tolerant pointer
/// arithmetic in `ZSTD_compressStream_generic` (`istart != NULL ? istart+size
/// : istart`, `compress/zstd_compress.c:6108-6110`).
///
/// `input->src == NULL` with `input->size == 5` is **in contract for the guard**
/// (the guard only compares `pos` against `size`) but the generic loop then
/// derives `iend - ip == 0`, so nothing is read from the NULL pointer — verified
/// by running it here under both libraries.
///
/// CONFIGS 70.
#[test]
fn compress_stream2_buffer_and_enddirective_guards() {
    covers(&["CFG:70"]);
    let src = corpus(Corpus::Text, 4096, 0x70);

    // (a) output->pos > output->size  -> dstSize_tooSmall
    diff("cs2/out.pos>out.size", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut dst = vec![0u8; 256];
        let mut o = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 256,
            pos: 257,
        };
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: src.len(),
            pos: 0,
        };
        let r = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
        (r, o.pos, i.pos)
    });

    // (b) input->pos > input->size -> srcSize_wrong
    diff("cs2/in.pos>in.size", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut dst = vec![0u8; 256];
        let mut o = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 256,
            pos: 0,
        };
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: 100,
            pos: 101,
        };
        let r = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
        (r, o.pos, i.pos)
    });

    // (c) endOp out of range: 3 and -1 -> parameter_outOfBound
    for endop in [3i32, -1, i32::MAX, i32::MIN] {
        diff(&format!("cs2/endOp={endop}"), |l| {
            let cctx = Ctx::cctx(l);
            let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let mut dst = vec![0u8; 4096];
            let mut o = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: 4096,
                pos: 0,
            };
            let mut i = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let r = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, endop) });
            (r, o.pos, i.pos)
        });
    }

    // (d) NULL dst with size 0, NULL src with size 0 / size 5, each for the
    //     three legal endOp values.
    for endop in [ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end] {
        for (label, isize_, ssize) in [
            ("nulldst", 0usize, 0usize),
            ("nullsrc0", 0, 0),
            ("nullsrc5", 5, 0),
        ] {
            diff(&format!("cs2/{label}/endOp={endop}"), |l| {
                let cctx = Ctx::cctx(l);
                let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
                let mut dst = vec![0u8; 4096];
                let mut o = if label == "nulldst" {
                    ZSTD_outBuffer {
                        dst: std::ptr::null_mut(),
                        size: 0,
                        pos: 0,
                    }
                } else {
                    ZSTD_outBuffer {
                        dst: dst.as_mut_ptr() as *mut c_void,
                        size: 4096,
                        pos: 0,
                    }
                };
                let mut i = ZSTD_inBuffer {
                    src: std::ptr::null(),
                    size: isize_,
                    pos: 0,
                };
                let _ = ssize;
                let r = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, endop) });
                let mut out = Vec::new();
                if !o.dst.is_null() {
                    out.extend_from_slice(&dst[..o.pos]);
                }
                (r, o.pos, i.pos, Blob(out))
            });
        }
    }

    // (e) zero-capacity output with ZSTD_e_end: the frame cannot be finished, so
    //     the call must report a non-zero "remaining to flush".
    diff("cs2/zerocap-out/e_end", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut dst = vec![0u8; 1];
        let mut o = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 0,
            pos: 0,
        };
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: src.len(),
            pos: 0,
        };
        let r1 = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_end) });
        // and again, to check the state is still usable / consistent
        let r2 = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_end) });
        (r1, r2, o.pos, i.pos)
    });
}

// ===========================================================================
// ERRORS 195 — ZSTD_writeLastEmptyBlock
// ===========================================================================

/// `ZSTD_writeLastEmptyBlock` (`compress/zstd_compress.c:4772`):
/// `dstCapacity < ZSTD_blockHeaderSize` (3) -> `dstSize_tooSmall`; otherwise a
/// 3-byte last/raw/zero-size block header is written.
///
/// ERRORS 195 (`compress/zstd_compress.c:4772`).
#[test]
fn write_last_empty_block_capacity_guard() {
    covers(&["ERR:compress/zstd_compress.c:4772"]);
    for cap in 0..=8usize {
        diff_bytes(&format!("writeLastEmptyBlock cap={cap}"), |l| {
            let f = l.sym::<FnWriteLastEmptyBlock>("ZSTD_writeLastEmptyBlock");
            let mut dst = vec![0xCDu8; 16];
            let n = unsafe { f(dst.as_mut_ptr() as *mut c_void, cap) };
            (res(l, n), Blob(dst))
        });
    }
}

// ===========================================================================
// ERRORS 296/297/298 — the silent clamps in ZSTD_getCParams / ZSTD_getParams
// ===========================================================================

/// `ZSTD_getCParams_internal` (`compress/zstd_compress.c:7769`) silently clamps
/// `compressionLevel > ZSTD_MAX_CLEVEL` to 22 and maps `compressionLevel < 0` to
/// row 0 with `targetLength = -MAX(ZSTD_minCLevel(), level)`; `ZSTD_getCParams`
/// (`:7789`) and `ZSTD_getParams` (`:7815`) silently remap `srcSizeHint == 0` to
/// `ZSTD_CONTENTSIZE_UNKNOWN`, and `getParams` always forces
/// `fParams.contentSizeFlag = 1`. None of these has an error surface, so the
/// returned structs are compared field by field.
///
/// ERRORS 296, 297, 298.
#[test]
fn get_cparams_get_params_silent_clamps() {
    covers(&[
        "ERR:compress/zstd_compress.c:7769",
        "ERR:compress/zstd_compress.c:7789",
        "ERR:compress/zstd_compress.c:7815",
    ]);
    let levels: Vec<c_int> = {
        let mut v = vec![
            i32::MIN,
            i32::MIN + 1,
            -1000,
            -132,
            -131,
            -130,
            -100,
            -10,
            -1,
            0,
            1,
            3,
            19,
            22,
            23,
            24,
            100,
            i32::MAX,
        ];
        v.dedup();
        v
    };
    // srcSizeHint 0 is the remap case; the neighbours pin the table-ID selection.
    let hints: &[u64] = &[
        0,
        1,
        16 * 1024,
        16 * 1024 + 1,
        128 * 1024,
        128 * 1024 + 1,
        256 * 1024,
        256 * 1024 + 1,
        1 << 30,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ];
    for &lvl in &levels {
        for &hint in hints {
            for &dict in &[0usize, 1, 4096, 1 << 20] {
                diff(&format!("getCParams({lvl},{hint},{dict})"), |l| unsafe {
                    let a = l.sym::<FnGetCParams>("ZSTD_getCParams")(lvl, hint, dict);
                    let b = l.sym::<FnGetParams>("ZSTD_getParams")(lvl, hint, dict);
                    (a, b)
                });
            }
        }
    }
    // ZSTD_minCLevel()/ZSTD_maxCLevel()/ZSTD_defaultCLevel() anchor the clamp.
    diff("clevel bounds", |l| unsafe {
        (
            l.sym::<FnMinCLevel>("ZSTD_minCLevel")(),
            l.sym::<FnMaxCLevel>("ZSTD_maxCLevel")(),
            l.sym::<FnDefaultCLevel>("ZSTD_defaultCLevel")(),
        )
    });
}

// ===========================================================================
// ERRORS 299/300 — un-registering an external sequence producer
// ===========================================================================

extern "C" fn dummy_seq_prod(
    _state: *mut c_void,
    _out: *mut c_void,
    _outCap: SizeT,
    _src: *const c_void,
    _srcSize: SizeT,
    _dict: *const c_void,
    _dictSize: SizeT,
    _level: c_int,
    _windowSize: SizeT,
) -> SizeT {
    // Report "fall back to the internal sequence producer" unconditionally.
    0usize.wrapping_sub(106) /* ERROR(sequenceProducer_failed) */
}

/// `ZSTD_registerSequenceProducer` (`compress/zstd_compress.c:7819`) and
/// `ZSTD_CCtxParams_registerSequenceProducer` (`:7836`) with
/// `extSeqProdFunc == NULL`: the documented way to UNREGISTER. Both
/// `extSeqProdFunc` and `extSeqProdState` are cleared, so the supplied state is
/// discarded and compression must behave exactly as if nothing was ever
/// registered.
///
/// Passing a NULL *cctx* is out of contract (`assert(zc != NULL)` at `:7824` is
/// compiled out at `DEBUGLEVEL=0`, so it is an unconditional NULL dereference)
/// and is therefore not exercised.
///
/// ERRORS 299, 300.
#[test]
fn register_sequence_producer_null_unregisters() {
    covers(&[
        "ERR:compress/zstd_compress.c:7819",
        "ERR:compress/zstd_compress.c:7836",
    ]);
    let src = corpus(Corpus::Text, 40_000, 0x299);

    // Reference bytes with no producer ever registered.
    diff_bytes("regSeqProd/never-registered", |l| {
        let cctx = Ctx::cctx(l);
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
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
        (r, Blob(dst))
    });

    // register(non-NULL) then register(NULL) must be indistinguishable from
    // never having registered; the state pointer must be dropped as well.
    diff_bytes("regSeqProd/cctx register-then-null", |l| {
        let cctx = Ctx::cctx(l);
        let reg = l.sym::<FnRegSeqProd>("ZSTD_registerSequenceProducer");
        let mut state = [0u8; 32];
        unsafe {
            reg(
                cctx.ptr,
                state.as_mut_ptr() as *mut c_void,
                dummy_seq_prod as *const c_void,
            );
            // Now UNregister, while still handing over a non-NULL state.
            reg(
                cctx.ptr,
                state.as_mut_ptr() as *mut c_void,
                std::ptr::null(),
            );
        }
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
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
        (r, Blob(dst))
    });

    // Same via ZSTD_CCtxParams_registerSequenceProducer, then applied to a cctx.
    diff_bytes("regSeqProd/params register-then-null", |l| {
        let p = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
        assert!(!p.is_null());
        let params = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
        let reg = l.sym::<FnRegSeqProd>("ZSTD_CCtxParams_registerSequenceProducer");
        let mut state = [0u8; 32];
        unsafe {
            reg(
                params.ptr,
                state.as_mut_ptr() as *mut c_void,
                dummy_seq_prod as *const c_void,
            );
            reg(
                params.ptr,
                state.as_mut_ptr() as *mut c_void,
                std::ptr::null(),
            );
        }
        let cctx = Ctx::cctx(l);
        let applied = res(l, unsafe {
            l.sym::<unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT>(
                "ZSTD_CCtx_setParametersUsingCCtxParams",
            )(cctx.ptr, params.ptr)
        });
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
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
        (applied, r, Blob(dst))
    });
}

// ===========================================================================
// ERRORS 225 — ZSTD_compress_advanced parameter validation
// ===========================================================================

/// `ZSTD_compress_advanced` (`compress/zstd_compress.c:5448`):
/// `ZSTD_checkCParams(params.cParams)` fails -> `parameter_outOfBound` (42).
/// Every individual cParam is pushed one step outside its bound.
///
/// ERRORS 225 (`compress/zstd_compress.c:5448`).
#[test]
fn compress_advanced_rejects_out_of_bound_cparams() {
    covers(&["ERR:compress/zstd_compress.c:5448"]);
    let src = corpus(Corpus::Text, 8192, 0x225);
    let dict = corpus(Corpus::Text, 4096, 0x226);

    // A known-good baseline first, then one mutation at a time.
    let mutate: &[(&str, fn(&mut ZSTD_parameters))] = &[
        ("baseline", |_p| {}),
        ("strategy=10", |p| p.cParams.strategy = 10),
        ("strategy=0", |p| p.cParams.strategy = 0),
        ("strategy=-1", |p| p.cParams.strategy = -1),
        ("windowLog=9", |p| p.cParams.windowLog = 9),
        ("windowLog=32", |p| p.cParams.windowLog = 32),
        ("chainLog=5", |p| p.cParams.chainLog = 5),
        ("chainLog=31", |p| p.cParams.chainLog = 31),
        ("hashLog=5", |p| p.cParams.hashLog = 5),
        ("hashLog=31", |p| p.cParams.hashLog = 31),
        ("searchLog=31", |p| p.cParams.searchLog = 31),
        ("minMatch=2", |p| p.cParams.minMatch = 2),
        ("minMatch=8", |p| p.cParams.minMatch = 8),
        ("targetLength=huge", |p| p.cParams.targetLength = 1 << 21),
    ];
    for (label, f) in mutate {
        diff_bytes(&format!("compress_advanced/{label}"), |l| {
            let cctx = Ctx::cctx(l);
            let mut params =
                unsafe { l.sym::<FnGetParams>("ZSTD_getParams")(3, src.len() as u64, dict.len()) };
            f(&mut params);
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                l.sym::<FnCompressAdvanced>("ZSTD_compress_advanced")(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    params,
                )
            };
            let r = res(l, n);
            if let R::Ok(k) = r {
                dst.truncate(k);
            }
            (r, Blob(dst))
        });
    }
}

// ===========================================================================
// ERRORS 841/842/844/845 — divsufsort / divbwt argument guards
// ===========================================================================

/// `divsufsort` (`dictBuilder/divsufsort.c:1853` `T == NULL || SA == NULL ||
/// n < 0` -> `-1`; `:1854` `n == 0` -> `0` with `SA` untouched) and `divbwt`
/// (`:1882` `T == NULL || U == NULL || n < 0` -> `-1`; `:1883` `n <= 1` ->
/// `n`), called with the full 7-argument prototype from `divsufsort.h`.
///
/// ERRORS 841, 842, 844, 845.
#[test]
fn divsufsort_divbwt_argument_guards() {
    covers(&[
        "ERR:dictBuilder/divsufsort.c:1853",
        "ERR:dictBuilder/divsufsort.c:1854",
        "ERR:dictBuilder/divsufsort.c:1882",
        "ERR:dictBuilder/divsufsort.c:1883",
    ]);
    let data = corpus(Corpus::Text, 64, 0x841);
    for n in [-1i32, -1000, i32::MIN, 0, 1, 2] {
        diff(&format!("divsufsort(NULL T, n={n})"), |l| {
            let mut sa = vec![-7i32; 72];
            let rc = unsafe {
                l.sym::<FnDivsufsort>("divsufsort")(std::ptr::null(), sa.as_mut_ptr(), n, 0)
            };
            (rc, sa)
        });
        diff(&format!("divsufsort(NULL SA, n={n})"), |l| unsafe {
            l.sym::<FnDivsufsort>("divsufsort")(data.as_ptr(), std::ptr::null_mut(), n, 0)
        });
        diff(&format!("divsufsort(ok, n={n})"), |l| {
            let mut sa = vec![-7i32; 72];
            let rc = unsafe {
                l.sym::<FnDivsufsort>("divsufsort")(data.as_ptr(), sa.as_mut_ptr(), n, 0)
            };
            (rc, sa)
        });
        diff(&format!("divbwt(NULL T, n={n})"), |l| {
            let mut u = vec![0xEEu8; 72];
            let mut a = vec![0i32; 80];
            let rc = unsafe {
                l.sym::<FnDivbwt>("divbwt")(
                    std::ptr::null(),
                    u.as_mut_ptr(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            (rc, u, a)
        });
        diff(&format!("divbwt(NULL U, n={n})"), |l| {
            let mut a = vec![0i32; 80];
            let rc = unsafe {
                l.sym::<FnDivbwt>("divbwt")(
                    data.as_ptr(),
                    std::ptr::null_mut(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            (rc, a)
        });
        diff(&format!("divbwt(ok, n={n})"), |l| {
            let mut t = data.clone();
            let mut u = vec![0xEEu8; 72];
            let mut a = vec![0i32; 80];
            let rc = unsafe {
                l.sym::<FnDivbwt>("divbwt")(
                    t.as_mut_ptr(),
                    u.as_mut_ptr(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                )
            };
            (rc, t, u, a)
        });
    }
}

// ===========================================================================
// ERRORS 204..216 — ZSTD_loadCEntropy over a hand-built ZSTD_MAGIC_DICTIONARY
// ===========================================================================

/// Serialise a "raw 4-bit weights" Huffman table header, the format
/// `HUF_readStats` accepts when `src[0] >= 128` (`common/entropy_common.c`).
fn huf_raw_weights(weights: &[u8]) -> Vec<u8> {
    assert!(!weights.is_empty() && weights.len() <= 128);
    let mut v = vec![127u8 + weights.len() as u8];
    let mut n = 0;
    while n < weights.len() {
        let hi = weights[n];
        let lo = if n + 1 < weights.len() { weights[n + 1] } else { 0 };
        v.push((hi << 4) | (lo & 0xF));
        n += 2;
    }
    v
}

/// `FSE_writeNCount` run against the **C** library, so the FSE headers embedded
/// in the fixtures below are the reference serialisation.
fn fse_ncount(counts: &[i16], max_sv: u32, table_log: u32) -> Vec<u8> {
    let l = &pair().c;
    let f = l.sym::<FnFseWriteNCount>("FSE_writeNCount");
    let mut buf = vec![0u8; 1024];
    let n = unsafe {
        f(
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            counts.as_ptr(),
            max_sv,
            table_log,
        )
    };
    assert!(
        !is_error(l, n),
        "FSE_writeNCount(maxSV={max_sv}, tableLog={table_log}) failed: {}",
        err_name(l, n)
    );
    buf.truncate(n);
    buf
}

/// A flat distribution over `nb` symbols for the given `tableLog`.
fn flat_ncount(nb: usize, table_log: u32) -> Vec<u8> {
    let total = 1usize << table_log;
    assert_eq!(total % nb, 0);
    let each = (total / nb) as i16;
    let counts: Vec<i16> = vec![each; nb];
    fse_ncount(&counts, nb as u32 - 1, table_log)
}

/// Assemble a `ZSTD_MAGIC_DICTIONARY` dictionary out of explicit sections.
#[allow(clippy::too_many_arguments)]
fn assemble_dict(
    dict_id: u32,
    huf: &[u8],
    off: &[u8],
    ml: &[u8],
    ll: &[u8],
    reps: &[u8],
    content: &[u8],
) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&le32(ZSTD_MAGIC_DICTIONARY));
    d.extend_from_slice(&le32(dict_id));
    d.extend_from_slice(huf);
    d.extend_from_slice(off);
    d.extend_from_slice(ml);
    d.extend_from_slice(ll);
    d.extend_from_slice(reps);
    d.extend_from_slice(content);
    d
}

fn reps_bytes(r: [u32; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity(12);
    for x in r {
        v.extend_from_slice(&le32(x));
    }
    v
}

/// Every fixture: `(label, dictionary bytes)`.
fn load_centropy_fixtures() -> Vec<(String, Vec<u8>)> {
    let huf_ok = huf_raw_weights(&[1, 1, 1, 1]);
    // weight 13 > HUF_TABLELOG_MAX (12) -> HUF_readStats corruption_detected
    let huf_bad_weight = huf_raw_weights(&[13, 1]);
    // all-zero weights -> weightTotal == 0
    let huf_zero_total = huf_raw_weights(&[0, 0, 0, 0]);
    // no rank-1 weight at all -> rankStats[1] < 2
    let huf_no_rank1 = huf_raw_weights(&[2, 2]);
    // weightTotal's complement is not a power of two -> verif != rest
    let huf_verif = huf_raw_weights(&[1, 2, 2]);

    let off_ok = flat_ncount(8, 6);
    let ml_ok = flat_ncount(8, 6);
    let ll_ok = flat_ncount(8, 6);
    // tableLog 9 > OffFSELog (8) / tableLog 10 > MLFSELog == LLFSELog (9)
    let tl9 = flat_ncount(2, 9);
    let tl10 = flat_ncount(2, 10);

    let content = corpus(Corpus::Text, 400, 0x204);
    let reps_ok = reps_bytes([1, 2, 3]);

    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut add = |name: &str, d: Vec<u8>| out.push((name.to_string(), d));

    // The reference: everything valid.
    add(
        "valid",
        assemble_dict(0x1234_5678, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_ok, &content),
    );
    // :5081  HUF_readCTable rejects the Huffman section (3 distinct HUF_readStats
    //        rejections, all surfacing as dictionary_corrupted here).
    for (n, h) in [
        ("huf-weight13", &huf_bad_weight),
        ("huf-zerototal", &huf_zero_total),
        ("huf-norank1", &huf_no_rank1),
        ("huf-verif", &huf_verif),
    ] {
        add(
            &format!("hufbad/{n}"),
            assemble_dict(1, h, &off_ok, &ml_ok, &ll_ok, &reps_ok, &content),
        );
    }
    // :5087  offcode FSE header malformed / truncated away entirely.
    add(
        "off/truncated-1byte",
        assemble_dict(1, &huf_ok, &[0xFF], &[], &[], &[], &[]),
    );
    add(
        "off/truncated-empty",
        assemble_dict(1, &huf_ok, &[], &[], &[], &[], &[]),
    );
    add(
        "off/garbage",
        assemble_dict(1, &huf_ok, &[0x2F, 0x00, 0x00], &[], &[], &[], &[]),
    );
    // :5088  offcodeLog > OffFSELog (8)
    add(
        "off/tableLog9",
        assemble_dict(1, &huf_ok, &tl9, &ml_ok, &ll_ok, &reps_ok, &content),
    );
    // :5102  matchlength FSE header malformed / truncated
    add(
        "ml/truncated-1byte",
        assemble_dict(1, &huf_ok, &off_ok, &[0xFF], &[], &[], &[]),
    );
    add(
        "ml/truncated-empty",
        assemble_dict(1, &huf_ok, &off_ok, &[], &[], &[], &[]),
    );
    // :5103  matchlengthLog > MLFSELog (9)
    add(
        "ml/tableLog10",
        assemble_dict(1, &huf_ok, &off_ok, &tl10, &ll_ok, &reps_ok, &content),
    );
    // :5116  litlength FSE header malformed / truncated
    add(
        "ll/truncated-1byte",
        assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &[0xFF], &[], &[]),
    );
    add(
        "ll/truncated-empty",
        assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &[], &[], &[]),
    );
    // :5117  litlengthLog > LLFSELog (9)
    add(
        "ll/tableLog10",
        assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &tl10, &reps_ok, &content),
    );
    // :5127  fewer than 12 bytes left for the three repcodes
    for k in 0..12usize {
        let reps = vec![0x11u8; k];
        add(
            &format!("reps/only{k}bytes"),
            assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps, &[]),
        );
    }
    // :5145  a repcode of 0
    for z in 0..3usize {
        let mut r = [1u32, 2, 3];
        r[z] = 0;
        add(
            &format!("reps/zero{z}"),
            assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_bytes(r), &content),
        );
    }
    // :5146  a repcode larger than the dictionary content
    for z in 0..3usize {
        let mut r = [1u32, 2, 3];
        r[z] = 0x00FF_FFFF;
        add(
            &format!("reps/huge{z}"),
            assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_bytes(r), &content),
        );
        // exactly content.len() is legal, one more is not
        let mut r2 = [1u32, 2, 3];
        r2[z] = content.len() as u32;
        add(
            &format!("reps/exact{z}"),
            assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_bytes(r2), &content),
        );
        let mut r3 = [1u32, 2, 3];
        r3[z] = content.len() as u32 + 1;
        add(
            &format!("reps/over{z}"),
            assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_bytes(r3), &content),
        );
    }
    // no dictionary content at all: every non-zero repcode is then too large
    add(
        "reps/nocontent",
        assemble_dict(1, &huf_ok, &off_ok, &ml_ok, &ll_ok, &reps_ok, &[]),
    );
    out
}

/// `ZSTD_loadCEntropy` (`compress/zstd_compress.c:5060..5150`) called directly on
/// hand-built `ZSTD_MAGIC_DICTIONARY` dictionaries that trip each rejection:
///
/// * `:5081` `HUF_isError(HUF_readCTable(...))`
/// * `:5087` `FSE_isError(FSE_readNCount(offcode))`, `:5088` `offcodeLog > 8`
/// * `:5102` matchlength `FSE_readNCount`,        `:5103` `matchlengthLog > 9`
/// * `:5116` litlength `FSE_readNCount`,          `:5117` `litlengthLog > 9`
/// * `:5127` `dictPtr + 12 > dictEnd`
/// * `:5145` `bs->rep[u] == 0`, `:5146` `bs->rep[u] > dictContentSize`
///
/// The whole `ZSTD_compressedBlockState_t` scratch buffer is compared as a
/// `Blob`, so a difference in the *tables built before* the rejection shows up
/// too, not just the returned error code.
///
/// ERRORS 204, 205, 206, 208, 209, 211, 212, 214, 215, 216.
#[test]
fn load_c_entropy_rejections() {
    covers(&[
        "ERR:compress/zstd_compress.c:5081",
        "ERR:compress/zstd_compress.c:5087",
        "ERR:compress/zstd_compress.c:5088",
        "ERR:compress/zstd_compress.c:5102",
        "ERR:compress/zstd_compress.c:5103",
        "ERR:compress/zstd_compress.c:5116",
        "ERR:compress/zstd_compress.c:5117",
        "ERR:compress/zstd_compress.c:5127",
        "ERR:compress/zstd_compress.c:5145",
        "ERR:compress/zstd_compress.c:5146",
    ]);
    for (label, dict) in load_centropy_fixtures() {
        // dictSize < 8 would make `dictEnd - (dictPtr += 8)` underflow, which is
        // out of contract for this internal entry point; every fixture is >= 8.
        assert!(dict.len() >= 8);
        let (r, _) = diff_bytes(&format!("loadCEntropy/{label}"), |l| {
            let f = l.sym::<FnLoadCEntropy>("ZSTD_loadCEntropy");
            let mut bs = vec![0u64; BS_SIZE / 8];
            let mut ws = wksp(HUF_WORKSPACE_SIZE);
            let n = unsafe {
                f(
                    bs.as_mut_ptr() as *mut c_void,
                    wksp_ptr(&mut ws),
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                )
            };
            let bytes: Vec<u8> = bs.iter().flat_map(|v| v.to_le_bytes()).collect();
            (res(l, n), Blob(bytes))
        });
        // Pin the outcome, so a fixture that stopped tripping its rejection (and
        // would therefore silently stop covering the row) fails the test.
        let expect_ok = label == "valid" || label.starts_with("reps/exact");
        if expect_ok {
            assert!(matches!(r, R::Ok(_)), "{label}: expected success, got {r:?}");
        } else {
            assert!(
                matches!(r, R::Err(30, _)),
                "{label}: expected dictionary_corrupted(30), got {r:?}"
            );
        }
        // Indirectly: ZSTD_createCDict must reject the very same fixtures.
        diff(&format!("loadCEntropy/{label}/createCDict"), |l| {
            let p = unsafe {
                l.sym::<FnCreateCDict>("ZSTD_createCDict")(
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    3,
                )
            };
            let ok = !p.is_null();
            if ok {
                unsafe { l.sym::<FnFreeAny>("ZSTD_freeCDict")(p) };
            }
            ok
        });
        // ... and so must a full-dict prefix, which surfaces the raw error code
        // (ZSTD_CCtx_refPrefix_advanced -> ZSTD_compress_insertDictionary ->
        //  ZSTD_loadZstdDictionary -> ZSTD_loadCEntropy).
        let src = corpus(Corpus::Text, 2000, 0x205);
        diff_bytes(&format!("loadCEntropy/{label}/refPrefix-fullDict"), |l| {
            let cctx = Ctx::cctx(l);
            let r0 = res(l, unsafe {
                l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                    cctx.ptr,
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    ZSTD_dct_fullDict,
                )
            });
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
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
            (r0, r, Blob(dst))
        });
    }
}

// ===========================================================================
// ERRORS 217/218/219 — ZSTD_compress_insertDictionary gates
// ===========================================================================

/// `ZSTD_compress_insertDictionary` (`compress/zstd_compress.c:5203..5225`):
///
/// * `:5206` `dict == NULL || dictSize < 8` with `dictContentType != fullDict`
///   -> `0`, i.e. the dictionary is silently ignored (dictID 0);
/// * `:5207` the same shape with `dictContentType == ZSTD_dct_fullDict`
///   -> `dictionary_wrong` (32);
/// * `:5223` `dictContentType == fullDict` and `MEM_readLE32(dict) !=
///   ZSTD_MAGIC_DICTIONARY` with `dictSize >= 8` -> `dictionary_wrong` (32).
///
/// Driven through `ZSTD_CCtx_refPrefix_advanced`, which is the only public entry
/// point that reaches `ZSTD_compress_insertDictionary` with a caller-chosen
/// `dictContentType` *without* the `ZSTD_createCDict_advanced2` wrapper turning
/// the error into `memory_allocation`.
///
/// ERRORS 217, 218, 219.
#[test]
fn compress_insert_dictionary_gates() {
    covers(&[
        "ERR:compress/zstd_compress.c:5206",
        "ERR:compress/zstd_compress.c:5207",
        "ERR:compress/zstd_compress.c:5223",
    ]);
    let src = corpus(Corpus::Text, 4000, 0x217);
    let long_nonmagic = corpus(Corpus::Text, 300, 0x218);
    for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
        for n in [1usize, 2, 4, 7, 8, 9, 300] {
            let pfx: Vec<u8> = long_nonmagic[..n.min(long_nonmagic.len())].to_vec();
            let (_, r, _) = diff_bytes(&format!("insertDict/dct{dct}/n{n}"), |l| {
                let cctx = Ctx::cctx(l);
                let r0 = res(l, unsafe {
                    l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                        cctx.ptr,
                        pfx.as_ptr() as *const c_void,
                        pfx.len(),
                        dct,
                    )
                });
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                let k = unsafe {
                    l.sym::<FnCompress2>("ZSTD_compress2")(
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
                (r0, r, Blob(dst))
            });
            // fullDict never accepts this prefix: < 8 bytes hits :5207 and >= 8
            // bytes without the dictionary magic hits :5223; everything else is
            // loaded as raw content (:5206 for n < 8) and succeeds.
            if dct == ZSTD_dct_fullDict {
                assert!(
                    matches!(r, R::Err(32, _)),
                    "dct_fullDict n={n}: expected dictionary_wrong(32), got {r:?}"
                );
            } else {
                assert!(matches!(r, R::Ok(_)), "dct={dct} n={n}: got {r:?}");
            }
        }
        // A NULL prefix pointer: refPrefix_advanced stores nothing, so the CCtx
        // simply has no prefix (dict == NULL never reaches insertDictionary from
        // this entry point) — recorded for completeness.
        diff("insertDict/null-prefix", |l| {
            let cctx = Ctx::cctx(l);
            res(l, unsafe {
                l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                    cctx.ptr,
                    std::ptr::null(),
                    0,
                    dct,
                )
            })
        });
    }
}

// ===========================================================================
// A stateless custom allocator (no process-wide state -> no serialisation
// needed, unlike the counting allocators in t25/t26).
// ===========================================================================

extern "C" fn plain_alloc(_opaque: *mut c_void, size: SizeT) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    unsafe {
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut SizeT).write(size);
        p.add(16) as *mut c_void
    }
}

extern "C" fn plain_free(_opaque: *mut c_void, p: *mut c_void) {
    if p.is_null() {
        return;
    }
    unsafe {
        let base = (p as *mut u8).sub(16);
        let size = (base as *mut SizeT).read();
        let layout = std::alloc::Layout::from_size_align(size + 16, 16).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

fn plain_mem() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(plain_alloc),
        customFree: Some(plain_free),
        opaque: 0x5A5Ausize as *mut c_void,
    }
}

// ===========================================================================
// ERRORS 123/126/128/124/125 — the CCtx dictionary API guards
// ===========================================================================

/// * `ZSTD_CCtx_loadDictionary_advanced` (`compress/zstd_compress.c:1290`),
///   `ZSTD_CCtx_refCDict` (`:1330`) and `ZSTD_CCtx_refPrefix_advanced`
///   (`:1354`): `cctx->streamStage != zcss_init` -> `stage_wrong` (60).
/// * `:1300` `ZSTD_dlm_byCopy` on a **static** CCtx -> `memory_allocation` (64)
///   (a static CCtx cannot allocate an internal copy of the dictionary).
/// * `:1303` `ZSTD_customMalloc(dictSize)` returns NULL -> `memory_allocation`
///   (64); driven with `dictSize = SIZE_MAX/2`, an allocation the kernel can
///   never satisfy, so the `ZSTD_memcpy` that follows a successful malloc is
///   unreachable.
///
/// ERRORS 123, 124, 125, 126, 128.
#[test]
fn cctx_dictionary_api_guards() {
    covers(&[
        "ERR:compress/zstd_compress.c:1290",
        "ERR:compress/zstd_compress.c:1300",
        "ERR:compress/zstd_compress.c:1303",
        "ERR:compress/zstd_compress.c:1330",
        "ERR:compress/zstd_compress.c:1354",
    ]);
    let src = corpus(Corpus::Text, 300_000, 0x123);
    let dict = corpus(Corpus::Text, 4096, 0x124);

    // ---- stage_wrong: mid-stream dictionary mutation --------------------
    let (r_load, r_refcd, r_refpfx, _, _) =
        diff_bytes("dictapi/stage_wrong", |l| {
            let cctx = Ctx::cctx(l);
            let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let mut dst = vec![0u8; 1 << 16];
            let mut o = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: dst.len(),
                pos: 0,
            };
            let mut i = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            // one ZSTD_e_continue leaves streamStage == zcss_load
            let started = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
            let a = res(l, unsafe {
                l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                    cctx.ptr,
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    ZSTD_dlm_byRef,
                    ZSTD_dct_auto,
                )
            });
            let cd = unsafe {
                l.sym::<FnCreateCDict>("ZSTD_createCDict")(
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    3,
                )
            };
            assert!(!cd.is_null());
            let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
            let b = res(l, unsafe {
                l.sym::<FnRefCDict>("ZSTD_CCtx_refCDict")(cctx.ptr, cdict.ptr)
            });
            let c = res(l, unsafe {
                l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                    cctx.ptr,
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    ZSTD_dct_rawContent,
                )
            });
            // finish the frame: the rejected mutations must have changed nothing
            let mut blob = dst[..o.pos].to_vec();
            loop {
                let rem = unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_end) };
                if is_error(l, rem) {
                    break;
                }
                if rem == 0 {
                    break;
                }
                blob.extend_from_slice(&dst[..o.pos]);
                o.pos = 0;
            }
            blob.truncate(0);
            blob.extend_from_slice(&dst[..o.pos]);
            (a, b, c, started, Blob(blob))
        });
    for (what, r) in [("loadDictionary", &r_load), ("refCDict", &r_refcd), ("refPrefix", &r_refpfx)] {
        assert!(
            matches!(r, R::Err(60, _)),
            "{what} mid-stream: expected stage_wrong(60), got {r:?}"
        );
    }

    // ---- static CCtx cannot copy a dictionary (:1300) -------------------
    let (r_static_copy, r_static_ref) = diff("dictapi/static-bycopy", |l| {
        let est = unsafe { l.sym::<FnEstFromInt2>("ZSTD_estimateCStreamSize")(3) };
        let mut ws = wksp(est + 4096);
        let p = unsafe {
            l.sym::<FnInitStatic>("ZSTD_initStaticCCtx")(wksp_ptr(&mut ws), wksp_bytes(&ws))
        };
        assert!(!p.is_null());
        let a = res(l, unsafe {
            l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                p,
                dict.as_ptr() as *const c_void,
                dict.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
            )
        });
        // byRef needs no allocation, so it is accepted even on a static CCtx
        let b = res(l, unsafe {
            l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                p,
                dict.as_ptr() as *const c_void,
                dict.len(),
                ZSTD_dlm_byRef,
                ZSTD_dct_auto,
            )
        });
        let _ = &mut ws;
        (a, b)
    });
    assert!(
        matches!(r_static_copy, R::Err(64, _)),
        "static byCopy: expected memory_allocation(64), got {r_static_copy:?}"
    );
    assert!(
        matches!(r_static_ref, R::Ok(0)),
        "static byRef: expected Ok(0), got {r_static_ref:?}"
    );

    // ---- ZSTD_customMalloc(SIZE_MAX/2) fails (:1303) --------------------
    let r_huge = diff("dictapi/huge-dictSize", |l| {
        let cctx = Ctx::cctx(l);
        res(l, unsafe {
            l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                cctx.ptr,
                dict.as_ptr() as *const c_void,
                usize::MAX / 2,
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
            )
        })
    });
    assert!(
        matches!(r_huge, R::Err(64, _)),
        "SIZE_MAX/2 dictionary: expected memory_allocation(64), got {r_huge:?}"
    );
}

// ===========================================================================
// ERRORS 242/246 — ZSTD_initStaticCStream / ZSTD_initCStream_internal
// ===========================================================================

/// `ZSTD_initStaticCStream` (`compress/zstd_compress.c:5933`) delegates verbatim
/// to `ZSTD_initStaticCCtx`: `workspaceSize <= sizeof(ZSTD_CCtx)` and a
/// workspace whose address is not 8-aligned both yield NULL.
///
/// `ZSTD_initCStream_internal` (`:5998`) forwards a failing
/// `ZSTD_CCtx_loadDictionary`: on a **static** CStream the by-copy dictionary
/// cannot be allocated, so the call must report `memory_allocation` (64).
///
/// ERRORS 242, 246.
#[test]
fn init_static_cstream_and_init_cstream_internal() {
    covers(&[
        "ERR:compress/zstd_compress.c:5933",
        "ERR:compress/zstd_compress.c:5998",
    ]);
    let dict = corpus(Corpus::Text, 4096, 0x246);

    // (a) sizes around sizeof(ZSTD_CCtx) and the alignment rejection
    diff("initStaticCStream/sizes", |l| {
        let est = unsafe { l.sym::<FnEstFromInt2>("ZSTD_estimateCStreamSize")(3) };
        let mut buf = vec![0u8; est + 64];
        let base = buf.as_mut_ptr();
        let aligned = ((base as usize) + 7) & !7usize;
        let mut out = Vec::new();
        for &sz in &[0usize, 1, 8, 64, 1024, 4096] {
            out.push(unsafe {
                l.sym::<FnInitStatic>("ZSTD_initStaticCStream")(aligned as *mut c_void, sz)
                    .is_null()
            });
        }
        // unaligned workspace with an otherwise ample size
        for off in 1..8usize {
            out.push(unsafe {
                l.sym::<FnInitStatic>("ZSTD_initStaticCStream")(
                    (aligned + off) as *mut c_void,
                    est,
                )
                .is_null()
            });
        }
        // the aligned, ample case must succeed
        out.push(unsafe {
            l.sym::<FnInitStatic>("ZSTD_initStaticCStream")(aligned as *mut c_void, est).is_null()
        });
        let _ = &mut buf;
        out
    });

    // (b) initCStream_internal on a static CStream, with a dict
    let (r_dict, r_cdict, r_heap) = diff("initCStream_internal/static+dict", |l| {
        let params = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
        assert!(!params.is_null());
        let params = Ctx::from_raw(l, params, "ZSTD_freeCCtxParams");
        let p = unsafe { l.sym::<FnGetParams>("ZSTD_getParams")(3, 0, dict.len()) };
        let _ = res(l, unsafe {
            l.sym::<FnCCtxParamsInitAdv>("ZSTD_CCtxParams_init_advanced")(params.ptr, p)
        });

        let est = unsafe { l.sym::<FnEstFromInt2>("ZSTD_estimateCStreamSize")(3) };
        let mut ws = wksp(est + 8192);
        let zcs = unsafe {
            l.sym::<FnInitStatic>("ZSTD_initStaticCStream")(wksp_ptr(&mut ws), wksp_bytes(&ws))
        };
        assert!(!zcs.is_null());
        let f = l.sym::<FnInitCStreamInternal>("ZSTD_initCStream_internal");
        let a = res(l, unsafe {
            f(
                zcs,
                dict.as_ptr() as *const c_void,
                dict.len(),
                std::ptr::null(),
                params.ptr,
                dict.len() as u64,
            )
        });
        // no dict at all -> ZSTD_CCtx_refCDict(NULL) -> success
        let b = res(l, unsafe {
            f(
                zcs,
                std::ptr::null(),
                0,
                std::ptr::null(),
                params.ptr,
                ZSTD_CONTENTSIZE_UNKNOWN,
            )
        });
        // the same dict on a heap CStream succeeds
        let heap = Ctx::cstream(l);
        let c = res(l, unsafe {
            f(
                heap.ptr,
                dict.as_ptr() as *const c_void,
                dict.len(),
                std::ptr::null(),
                params.ptr,
                dict.len() as u64,
            )
        });
        let _ = &mut ws;
        (a, b, c)
    });
    assert!(
        matches!(r_dict, R::Err(64, _)),
        "static initCStream_internal+dict: expected memory_allocation(64), got {r_dict:?}"
    );
    assert!(matches!(r_cdict, R::Ok(0)), "got {r_cdict:?}");
    assert!(matches!(r_heap, R::Ok(0)), "got {r_heap:?}");
}

// ===========================================================================
// ERRORS 262/264 — error forwarding out of ZSTD_compressStream2 / ZSTD_compress2
// ===========================================================================

/// * `ZSTD_compressStream2` (`compress/zstd_compress.c:6480`) forwards a failing
///   `ZSTD_CCtx_init_compressStream2`; driven by an invalid full dictionary.
/// * `ZSTD_compress2` (`:6592`) turns a non-zero "remaining to flush" from
///   `ZSTD_compressStream2_simpleArgs(..., ZSTD_e_end)` into
///   `dstSize_tooSmall` (70).
///
/// ERRORS 262, 264.
#[test]
fn compress_stream2_and_compress2_error_forwarding() {
    covers(&[
        "ERR:compress/zstd_compress.c:6480",
        "ERR:compress/zstd_compress.c:6592",
    ]);
    let src = corpus(Corpus::Text, 60_000, 0x262);
    // 8 bytes, >= 8 but without the dictionary magic: dct_fullDict rejects it.
    let bad_full: Vec<u8> = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];

    let r_init = diff("cs2/init-failure-forwarded", |l| {
        let cctx = Ctx::cctx(l);
        let r0 = res(l, unsafe {
            l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                cctx.ptr,
                bad_full.as_ptr() as *const c_void,
                bad_full.len(),
                ZSTD_dct_fullDict,
            )
        });
        let mut dst = vec![0u8; 1 << 17];
        let mut o = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: dst.len(),
            pos: 0,
        };
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: src.len(),
            pos: 0,
        };
        let r = res(l, unsafe {
            l.sym::<FnCompressStream2>("ZSTD_compressStream2")(
                cctx.ptr,
                &mut o,
                &mut i,
                ZSTD_e_end,
            )
        });
        let _ = &mut dst;
        (r0, r, o.pos, i.pos)
    });
    assert!(
        matches!(r_init.1, R::Err(32, _)),
        "expected dictionary_wrong(32) forwarded from init, got {:?}",
        r_init.1
    );

    // ZSTD_compress2 with a dstCapacity that cannot hold the frame.
    let full = compress_bound(&pair().c, src.len());
    let real = c_compress(&src, 3).len();
    for cap in [0usize, 1, 3, 8, 64, real / 2, real - 1, real, full] {
        let (r, _) = diff_bytes(&format!("compress2/cap={cap}"), |l| {
            let cctx = Ctx::cctx(l);
            let mut dst = vec![0xCDu8; cap.max(1)];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
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
            (r, Blob(dst))
        });
        // Anything well below the real frame size must report dstSize_tooSmall;
        // `ZSTD_compressBound` must succeed. In between, ZSTD_compress2 writes
        // straight into the caller's buffer (stableOutBuffer) and may still need
        // headroom for a raw-block fallback, so no claim is made there — only
        // that the two libraries agree.
        if cap <= real / 2 {
            assert!(
                matches!(r, R::Err(70, _)),
                "compress2 cap={cap} (real={real}): expected dstSize_tooSmall(70), got {r:?}"
            );
        } else if cap == full {
            assert!(matches!(r, R::Ok(_)), "compress2 cap={cap}: got {r:?}");
        }
    }
}

// ===========================================================================
// ERRORS 201/202/203 — ZSTD_loadDictionaryContent's silent truncations
// ===========================================================================

/// `ZSTD_loadDictionaryContent` (`compress/zstd_compress.c:4900..4980`) never
/// reports an error for an over-large dictionary; it silently loads only a
/// suffix:
///
/// * `:4938` `srcSize > MIN(ZSTD_CURRENT_MAX - 2, (1<<24) - 2)`. The 3500 MB
///   clause is not testable in-process, but the `(1<<24)-2` short-cache clause
///   is: it applies when `ZSTD_CDictIndicesAreTagged(cParams)` (strategy
///   `ZSTD_fast` or `ZSTD_dfast`) and `tfp == ZSTD_tfp_forCDict`, i.e. for a
///   `ZSTD_CDict` built at those strategies from a > 16 MB dictionary.
/// * `:4964` `srcSize > 1U << MIN(MAX(hashLog+3, chainLog+1), 31)` — with
///   `hashLog == chainLog == 6` that bound is 512 bytes, so any larger
///   dictionary is truncated to its last 512 bytes for table filling.
/// * `:4975` `srcSize <= HASH_READ_SIZE` (8) -> `0`, the dictionary content is
///   ignored for table filling entirely.
///
/// ERRORS 201, 202, 203.
#[test]
fn load_dictionary_content_silent_truncation() {
    covers(&[
        "ERR:compress/zstd_compress.c:4938",
        "ERR:compress/zstd_compress.c:4964",
        "ERR:compress/zstd_compress.c:4975",
    ]);
    let src = corpus(Corpus::Text, 40_000, 0x201);

    // ---- :4964  the hashLog/chainLog bound -------------------------------
    // A CDict built with explicit hashLog = chainLog = 6 pins the bound to 512.
    let dict4k = corpus(Corpus::Text, 4096, 0x202);
    let cp = diff("loadDictContent/512-bound-cparams", |l| {
        let mut cparams =
            unsafe { l.sym::<FnGetCParams>("ZSTD_getCParams")(3, 0, dict4k.len()) };
        cparams.hashLog = 6;
        cparams.chainLog = 6;
        cparams.windowLog = 10;
        cparams.searchLog = 1;
        cparams.minMatch = 4;
        cparams.targetLength = 0;
        cparams.strategy = ZSTD_fast;
        let cd = unsafe {
            l.sym::<unsafe extern "C" fn(
                *const c_void,
                SizeT,
                c_int,
                c_int,
                ZSTD_compressionParameters,
                ZSTD_customMem,
            ) -> *mut c_void>("ZSTD_createCDict_advanced")(
                dict4k.as_ptr() as *const c_void,
                dict4k.len(),
                ZSTD_dlm_byRef,
                ZSTD_dct_rawContent,
                cparams,
                ZSTD_customMem::default(),
            )
        };
        assert!(!cd.is_null());
        let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
        let eff = unsafe {
            l.sym::<FnGetCParamsFromCDict>("ZSTD_getCParamsFromCDict")(cdict.ptr)
        };
        // compress with it, so the truncated table state is observable in bytes
        let cctx = Ctx::cctx(l);
        let _ = res(l, unsafe {
            l.sym::<FnRefCDict>("ZSTD_CCtx_refCDict")(cctx.ptr, cdict.ptr)
        });
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
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
        (eff, r, Blob(dst))
    });
    // The effective cParams prove the 512-byte bound: 1 << MIN(MAX(6+3, 6+1), 31).
    assert_eq!(cp.0.hashLog, 6, "effective hashLog: {:?}", cp.0);
    assert_eq!(cp.0.chainLog, 6, "effective chainLog: {:?}", cp.0);
    let bound = 1usize << (cp.0.hashLog + 3).max(cp.0.chainLog + 1).min(31);
    assert!(
        dict4k.len() > bound,
        "the 4 KB dictionary must exceed the {bound}-byte table bound"
    );

    // ---- :4975  a dictionary of at most HASH_READ_SIZE bytes -------------
    for n in [1usize, 4, 7, 8, 9, 16] {
        let d = corpus(Corpus::Counter, n, 0x203);
        diff_bytes(&format!("loadDictContent/tiny-{n}"), |l| {
            let cctx = Ctx::cctx(l);
            let r0 = res(l, unsafe {
                l.sym::<FnRefPrefixAdv>("ZSTD_CCtx_refPrefix_advanced")(
                    cctx.ptr,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    ZSTD_dct_rawContent,
                )
            });
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let k = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
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
            (r0, r, Blob(dst))
        });
    }

    // ---- :4938  the (1<<24)-2 short-cache clamp --------------------------
    // 17 MB > 16777214, strategy ZSTD_fast => CDict indices are tagged.
    const BIG: usize = 17 * 1024 * 1024;
    let big = corpus(Corpus::LongRepeats, BIG, 0x2938);
    assert!(big.len() > (1 << 24) - 2);
    let r_big = diff_bytes("loadDictContent/17MB-shortcache", |l| {
        let mut cparams = unsafe { l.sym::<FnGetCParams>("ZSTD_getCParams")(1, 0, big.len()) };
        cparams.strategy = ZSTD_fast;
        let cd = unsafe {
            l.sym::<unsafe extern "C" fn(
                *const c_void,
                SizeT,
                c_int,
                c_int,
                ZSTD_compressionParameters,
                ZSTD_customMem,
            ) -> *mut c_void>("ZSTD_createCDict_advanced")(
                big.as_ptr() as *const c_void,
                big.len(),
                ZSTD_dlm_byRef,
                ZSTD_dct_rawContent,
                cparams,
                ZSTD_customMem::default(),
            )
        };
        assert!(!cd.is_null());
        let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
        let eff = unsafe { l.sym::<FnGetCParamsFromCDict>("ZSTD_getCParamsFromCDict")(cdict.ptr) };
        let cctx = Ctx::cctx(l);
        let _ = res(l, unsafe {
            l.sym::<FnRefCDict>("ZSTD_CCtx_refCDict")(cctx.ptr, cdict.ptr)
        });
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
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
        (eff, r, Blob(dst))
    });
    assert_eq!(
        r_big.0.strategy, ZSTD_fast,
        "the CDict must keep strategy ZSTD_fast for indices to be tagged: {:?}",
        r_big.0
    );
    assert!(matches!(r_big.1, R::Ok(_)), "got {:?}", r_big.1);
}

// ===========================================================================
// CONFIGS 178 + ERRORS 233/234 — ZSTD_createCDict_advanced2
// ===========================================================================

/// `ZSTD_createCDict_advanced2` (`compress/zstd_compress.c:5660..5710`) derives
/// its cParams from a `ZSTD_CCtx_params` with `ZSTD_cpm_createCDict` — a
/// different path from `ZSTD_createCDict_advanced`'s direct cParams, and the only
/// way to enable dedicated dictionary search at CDict creation time.
///
/// * `:5672` `!customMem.customAlloc ^ !customMem.customFree` -> NULL.
/// * `:5704` `cdict == NULL` **or** `ZSTD_isError(ZSTD_initCDict_internal(...))`
///   -> `ZSTD_freeCDict(cdict)` then NULL, driven with a corrupt full dictionary.
///
/// CONFIGS 178; ERRORS 233, 234.
#[test]
fn create_cdict_advanced2_from_cctx_params() {
    covers(&[
        "CFG:178",
        "ERR:compress/zstd_compress.c:5672",
        "ERR:compress/zstd_compress.c:5704",
    ]);
    let raw_dict = corpus(Corpus::Text, 32 * 1024, 0x178);
    // A well-formed ZSTD_MAGIC_DICTIONARY dictionary with the same content, so
    // ZSTD_dct_fullDict has something valid to accept.
    let full_dict = assemble_dict(
        0xABCD_1234,
        &huf_raw_weights(&[1, 1, 1, 1]),
        &flat_ncount(8, 6),
        &flat_ncount(8, 6),
        &flat_ncount(8, 6),
        &reps_bytes([1, 2, 3]),
        &raw_dict,
    );
    let src = corpus(Corpus::Text, 50_000, 0x179);
    // a >= 8-byte buffer carrying the dictionary magic but a corrupt body
    let mut bad_full = le32(ZSTD_MAGIC_DICTIONARY).to_vec();
    bad_full.extend_from_slice(&le32(7));
    bad_full.extend_from_slice(&[0xFFu8; 64]);

    let only_alloc = ZSTD_customMem {
        customAlloc: Some(plain_alloc),
        customFree: None,
        opaque: std::ptr::null_mut(),
    };
    let only_free = ZSTD_customMem {
        customAlloc: None,
        customFree: Some(plain_free),
        opaque: std::ptr::null_mut(),
    };

    for dds in [0i32, 1] {
        for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
            for &dct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                for custom in [false, true] {
                    let label = format!("cdictAdv2/dds{dds}/dlm{dlm}/dct{dct}/custom{custom}");
                    let dict: &[u8] = if dct == ZSTD_dct_fullDict {
                        &full_dict
                    } else {
                        &raw_dict
                    };
                    diff_bytes(&label, |l| {
                        let p = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
                        assert!(!p.is_null());
                        let params = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
                        let set = l.sym::<FnCCtxParamsSet>("ZSTD_CCtxParams_setParameter");
                        let mut sets = Vec::new();
                        sets.push(res(l, unsafe {
                            set(params.ptr, ZSTD_c_compressionLevel, 12)
                        }));
                        sets.push(res(l, unsafe { set(params.ptr, ZSTD_c_windowLog, 22) }));
                        sets.push(res(l, unsafe {
                            set(params.ptr, ZSTD_c_enableDedicatedDictSearch, dds)
                        }));
                        let mem = if custom {
                            plain_mem()
                        } else {
                            ZSTD_customMem::default()
                        };
                        let cd = unsafe {
                            l.sym::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2")(
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                                dlm,
                                dct,
                                params.ptr,
                                mem,
                            )
                        };
                        assert!(!cd.is_null(), "[{}] createCDict_advanced2 returned NULL", l.tag);
                        let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                        let eff = unsafe {
                            l.sym::<FnGetCParamsFromCDict>("ZSTD_getCParamsFromCDict")(cdict.ptr)
                        };
                        let dictid = unsafe {
                            l.sym::<unsafe extern "C" fn(*const c_void) -> c_uint>(
                                "ZSTD_getDictID_fromCDict",
                            )(cdict.ptr)
                        };
                        let sz = unsafe {
                            l.sym::<FnEstFromPtrSz>("ZSTD_sizeof_CDict")(cdict.ptr)
                        };
                        // and use it, so the whole table state is compared
                        let cctx = Ctx::cctx(l);
                        let r0 = res(l, unsafe {
                            l.sym::<FnRefCDict>("ZSTD_CCtx_refCDict")(cctx.ptr, cdict.ptr)
                        });
                        let cap = compress_bound(l, src.len()) + 64;
                        let mut dst = vec![0xCDu8; cap];
                        let n = unsafe {
                            l.sym::<FnCompress2>("ZSTD_compress2")(
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
                        (sets, eff, dictid, sz, r0, r, Blob(dst))
                    });
                }
            }
        }
    }

    // :5672 — exactly one of the two hooks is NULL
    diff("cdictAdv2/mismatched-customMem", |l| {
        let p = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
        let params = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
        let f = l.sym::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2");
        let a = unsafe {
            f(
                raw_dict.as_ptr() as *const c_void,
                raw_dict.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
                params.ptr,
                only_alloc,
            )
        };
        let b = unsafe {
            f(
                raw_dict.as_ptr() as *const c_void,
                raw_dict.len(),
                ZSTD_dlm_byCopy,
                ZSTD_dct_auto,
                params.ptr,
                only_free,
            )
        };
        (a.is_null(), b.is_null())
    });

    // :5704 — ZSTD_initCDict_internal fails on a corrupt full dictionary
    for &dlm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
        for custom in [false, true] {
            let nulled = diff(
                &format!("cdictAdv2/corrupt-fullDict/dlm{dlm}/custom{custom}"),
                |l| {
                    let p = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
                    let params = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
                    let mem = if custom {
                        plain_mem()
                    } else {
                        ZSTD_customMem::default()
                    };
                    let cd = unsafe {
                        l.sym::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2")(
                            bad_full.as_ptr() as *const c_void,
                            bad_full.len(),
                            dlm,
                            ZSTD_dct_fullDict,
                            params.ptr,
                            mem,
                        )
                    };
                    if !cd.is_null() {
                        unsafe { l.sym::<FnFreeAny>("ZSTD_freeCDict")(cd) };
                    }
                    cd.is_null()
                },
            );
            assert!(nulled, "corrupt full dictionary must yield NULL");
        }
    }
}

// ===========================================================================
// ERRORS 422..431 — the exported ZSTD_ldm_* entry points
// ===========================================================================

/// One byte that stands in for the `(BYTE const*)" "` literal `ZSTD_window_init`
/// installs as the initial `base`/`dictBase`. It is never dereferenced here
/// because `lowLimit == dictLimit` (so `ZSTD_window_hasExtDict()` is false).
static WINDOW_DICTBASE_DUMMY: [u8; 8] = [0; 8];

/// Reproduce the window state `ZSTD_window_init()` followed by
/// `ZSTD_window_update(window, src, srcSize, 0)` produces
/// (`compress/zstd_compress_internal.h:1334` and `:1354`), which is the
/// documented precondition of `ZSTD_ldm_generateSequences`.
fn ldm_window_for(src: &[u8]) -> ZSTD_window_t {
    ZSTD_window_t {
        nextSrc: unsafe { src.as_ptr().add(src.len()) },
        base: src.as_ptr().wrapping_sub(2),
        dictBase: WINDOW_DICTBASE_DUMMY.as_ptr(),
        dictLimit: 2,
        lowLimit: 2,
        nbOverflowCorrections: 0,
    }
}

fn empty_ldm_state() -> ldmState_t {
    ldmState_t {
        window: ZSTD_window_t {
            nextSrc: std::ptr::null(),
            base: std::ptr::null(),
            dictBase: std::ptr::null(),
            dictLimit: 2,
            lowLimit: 2,
            nbOverflowCorrections: 0,
        },
        hashTable: std::ptr::null_mut(),
        loadedDictEnd: 0,
        bucketOffsets: std::ptr::null_mut(),
        splitIndices: [0; LDM_BATCH_SIZE],
        matchCandidates: [ldmMatchCandidate_t {
            split: std::ptr::null(),
            hash: 0,
            checksum: 0,
            bucket: std::ptr::null_mut(),
        }; LDM_BATCH_SIZE],
    }
}

/// `ZSTD_ldm_adjustParameters` (`compress/zstd_ldm.c:166`),
/// `ZSTD_ldm_getTableSize` (`:176`) and `ZSTD_ldm_getMaxNbSeq` (`:181`).
/// None of the three has a rejection site: the first silently clamps
/// (`hashRateLog` derivation, `hashLog = BOUNDED(6, windowLog - hashRateLog, 30)`,
/// `minMatchLength` default, `bucketSizeLog = BOUNDED(3, strategy, 8)` then
/// `MIN(bucketSizeLog, hashLog)`), the other two return a size, so the *outputs*
/// are compared instead of an error code.
///
/// `ZSTD_ldm_getMaxNbSeq` divides by `params.minMatchLength` with no zero guard,
/// so `minMatchLength == 0` raises SIGFPE inside the reference C: that input is
/// out of contract and is never passed here.
///
/// ERRORS 422, 423, 424.
#[test]
fn ldm_parameter_helpers() {
    covers(&[
        "ERR:compress/zstd_ldm.c:166",
        "ERR:compress/zstd_ldm.c:176",
        "ERR:compress/zstd_ldm.c:181",
    ]);
    let enables = [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable];
    for &en in &enables {
        for &hash_log in &[0u32, 5, 6, 10, 20, 30, 31] {
            for &bucket in &[0u32, 1, 3, 4, 8, 9, 12] {
                for &minml in &[0u32, 1, 32, 64, 100] {
                    for &rate in &[0u32, 1, 4, 7, 20] {
                        for &wlog in &[10u32, 17, 27] {
                            for &strat in ALL_STRATEGIES {
                                let p0 = ldmParams_t {
                                    enableLdm: en,
                                    hashLog: hash_log,
                                    bucketSizeLog: bucket,
                                    minMatchLength: minml,
                                    hashRateLog: rate,
                                    windowLog: wlog,
                                };
                                let cparams = ZSTD_compressionParameters {
                                    windowLog: wlog,
                                    chainLog: 16,
                                    hashLog: 17,
                                    searchLog: 1,
                                    minMatch: 4,
                                    targetLength: 0,
                                    strategy: strat,
                                };
                                let label = format!(
                                    "ldmAdjust/en{en}/h{hash_log}/b{bucket}/m{minml}/r{rate}/w{wlog}/s{strat}"
                                );
                                let adjusted = diff(&label, |l| {
                                    let mut p = p0;
                                    unsafe {
                                        l.sym::<FnLdmAdjust>("ZSTD_ldm_adjustParameters")(
                                            &mut p, &cparams,
                                        )
                                    };
                                    // getTableSize / getMaxNbSeq on the adjusted
                                    // params (minMatchLength is now non-zero)
                                    let ts = unsafe {
                                        l.sym::<FnLdmGetTableSize>("ZSTD_ldm_getTableSize")(p)
                                    };
                                    let mut seqs = Vec::new();
                                    for chunk in [0usize, 1, 1 << 10, 1 << 20, 1 << 27] {
                                        seqs.push(unsafe {
                                            l.sym::<FnLdmGetMaxNbSeq>("ZSTD_ldm_getMaxNbSeq")(
                                                p, chunk,
                                            )
                                        });
                                    }
                                    (p, ts, seqs)
                                });
                                assert!(
                                    adjusted.0.minMatchLength > 0,
                                    "adjustParameters must leave minMatchLength non-zero"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // getTableSize / getMaxNbSeq directly on un-adjusted params (minMatchLength
    // deliberately non-zero: 0 SIGFPEs in the reference C, see the doc comment).
    for &en in &enables {
        for &hash_log in &[6u32, 10, 20] {
            for &bucket in &[1u32, 3, 8, 20] {
                for &minml in &[1u32, 32, 64] {
                    let p = ldmParams_t {
                        enableLdm: en,
                        hashLog: hash_log,
                        bucketSizeLog: bucket,
                        minMatchLength: minml,
                        hashRateLog: 4,
                        windowLog: 20,
                    };
                    diff(
                        &format!("ldmSizes/en{en}/h{hash_log}/b{bucket}/m{minml}"),
                        |l| {
                            let ts = unsafe {
                                l.sym::<FnLdmGetTableSize>("ZSTD_ldm_getTableSize")(p)
                            };
                            let ms: Vec<SizeT> = [0usize, 63, 64, 1 << 20]
                                .iter()
                                .map(|&c| unsafe {
                                    l.sym::<FnLdmGetMaxNbSeq>("ZSTD_ldm_getMaxNbSeq")(p, c)
                                })
                                .collect();
                            (ts, ms)
                        },
                    );
                }
            }
        }
    }
}

/// `ZSTD_ldm_generateSequences` / `ZSTD_ldm_generateSequences_internal`
/// (`compress/zstd_ldm.c:373`, `:479`, `:548`, `:585`) driven directly on an
/// `ldmState_t` whose window is set up exactly as `ZSTD_window_init` +
/// `ZSTD_window_update` would leave it, plus `ZSTD_ldm_fillHashTable`.
///
/// * `:373` `srcSize < params->minMatchLength` -> the whole chunk is leftover
///   literals (not an error).
/// * `:479` a match was found but `rawSeqStore->size == capacity` ->
///   `dstSize_tooSmall` (70), which `:585` forwards unchanged.
/// * `:548` the chunk loop condition is already false on entry (store full) ->
///   `0`, with no sequences generated.
///
/// ERRORS 425, 426, 427, 428.
#[test]
fn ldm_generate_sequences_paths() {
    covers(&[
        "ERR:compress/zstd_ldm.c:373",
        "ERR:compress/zstd_ldm.c:479",
        "ERR:compress/zstd_ldm.c:548",
        "ERR:compress/zstd_ldm.c:585",
    ]);
    const HASH_LOG: u32 = 12;
    const BUCKET_LOG: u32 = 3;
    const MINML: u32 = 64;
    let params = ldmParams_t {
        enableLdm: ZSTD_ps_enable,
        hashLog: HASH_LOG,
        bucketSizeLog: BUCKET_LOG,
        minMatchLength: MINML,
        hashRateLog: 4,
        windowLog: 20,
    };

    // Input with several *bounded* long-range repeats: one fixed 512-byte random
    // block (>= minMatchLength) separated by distinct random filler, so each
    // repeat yields its own sequence instead of one match swallowing the tail.
    let anchor = corpus(Corpus::Random, 512, 0x425);
    let mut repeated = Vec::new();
    for i in 0..8u64 {
        repeated.extend_from_slice(&anchor);
        repeated.extend_from_slice(&corpus(Corpus::Random, 4096, 0x4250 + i));
    }

    // Runner: build the state, run generateSequences, report the status plus the
    // whole produced sequence store and the hash table.
    fn make_run<'a>(
        params: &'a ldmParams_t,
        src: &'a [u8],
        capacity: usize,
        prefill: usize,
    ) -> impl Fn(&Lib) -> (R, SizeT, SizeT, SizeT, SizeT, Blob, Blob, Blob) + 'a {
        move |l: &Lib| {
            let mut table = vec![ldmEntry_t::default(); 1usize << HASH_LOG];
            let mut buckets = vec![0u8; 1usize << (HASH_LOG - BUCKET_LOG)];
            let mut st = empty_ldm_state();
            st.window = ldm_window_for(src);
            st.hashTable = table.as_mut_ptr();
            st.bucketOffsets = buckets.as_mut_ptr();
            let mut seqs = vec![rawSeq::default(); capacity.max(1)];
            let mut store = RawSeqStore_t {
                seq: seqs.as_mut_ptr(),
                pos: 0,
                posInSequence: 0,
                size: prefill,
                capacity,
            };
            let n = unsafe {
                l.sym::<FnLdmGenerate>("ZSTD_ldm_generateSequences")(
                    &mut st,
                    &mut store,
                    params,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            let seq_bytes: Vec<u8> = seqs
                .iter()
                .flat_map(|s| {
                    let mut v = Vec::with_capacity(12);
                    v.extend_from_slice(&s.offset.to_le_bytes());
                    v.extend_from_slice(&s.litLength.to_le_bytes());
                    v.extend_from_slice(&s.matchLength.to_le_bytes());
                    v
                })
                .collect();
            let tbl_bytes: Vec<u8> = table
                .iter()
                .flat_map(|e| {
                    let mut v = Vec::with_capacity(8);
                    v.extend_from_slice(&e.offset.to_le_bytes());
                    v.extend_from_slice(&e.checksum.to_le_bytes());
                    v
                })
                .collect();
            (
                res(l, n),
                store.pos,
                store.posInSequence,
                store.size,
                store.capacity,
                Blob(seq_bytes),
                Blob(tbl_bytes),
                Blob(buckets),
            )
        }
    }

    // :373 — srcSize below minMatchLength
    for n in [0usize, 1, 8, 32, 63] {
        let src = &repeated[..n];
        let r = diff_bytes(&format!("ldmGen/short-{n}"), make_run(&params, src, 64, 0));
        assert!(matches!(r.0, R::Ok(0)), "short input: got {:?}", r.0);
        assert_eq!(r.3, 0, "no sequences may be produced for srcSize < minMatch");
    }

    // ample capacity: a normal successful run that does produce sequences
    let ok = diff_bytes("ldmGen/ample", make_run(&params, &repeated, 4096, 0));
    assert!(matches!(ok.0, R::Ok(0)), "got {:?}", ok.0);
    assert!(
        ok.3 >= 4,
        "the corpus must produce several LDM sequences (got {})",
        ok.3
    );

    // :479 / :585 — capacity exhausted while matches are still being found
    for cap in 1..=3usize {
        let r = diff_bytes(&format!("ldmGen/exhaust-cap{cap}"), make_run(&params, &repeated, cap, 0));
        assert!(
            matches!(r.0, R::Err(70, _)),
            "cap={cap}: expected dstSize_tooSmall(70), got {:?}",
            r.0
        );
    }

    // :548 — the store is already full on entry
    for cap in [0usize, 1, 4] {
        let r = diff_bytes(&format!("ldmGen/full-on-entry-{cap}"), make_run(&params, &repeated, cap, cap));
        assert!(
            matches!(r.0, R::Ok(0)),
            "cap={cap} full on entry: expected Ok(0), got {:?}",
            r.0
        );
        assert_eq!(r.3, cap, "the store must be left untouched");
    }

    // ZSTD_ldm_fillHashTable on the same state, for the table contents
    diff_bytes("ldmFillHashTable", |l| {
        let mut table = vec![ldmEntry_t::default(); 1usize << HASH_LOG];
        let mut buckets = vec![0u8; 1usize << (HASH_LOG - BUCKET_LOG)];
        let mut st = empty_ldm_state();
        st.window = ldm_window_for(&repeated);
        st.hashTable = table.as_mut_ptr();
        st.bucketOffsets = buckets.as_mut_ptr();
        unsafe {
            l.sym::<FnLdmFillHashTable>("ZSTD_ldm_fillHashTable")(
                &mut st,
                repeated.as_ptr(),
                repeated.as_ptr().add(repeated.len()),
                &params,
            )
        };
        let tbl_bytes: Vec<u8> = table
            .iter()
            .flat_map(|e| {
                let mut v = Vec::with_capacity(8);
                v.extend_from_slice(&e.offset.to_le_bytes());
                v.extend_from_slice(&e.checksum.to_le_bytes());
                v
            })
            .collect();
        (Blob(tbl_bytes), Blob(buckets))
    });
}

/// `ZSTD_ldm_skipSequences` (`compress/zstd_ldm.c:606`) and
/// `ZSTD_ldm_skipRawSeqStoreBytes` (`:664`). Neither can fail — the first is
/// guarded only by `srcSize > 0 && pos < size` and the second truncates
/// `posInSequence + nbBytes` to `U32` with no bound check — so the mutated
/// `RawSeqStore_t` (including every sequence) is what gets compared.
///
/// ERRORS 429, 430.
#[test]
fn ldm_skip_sequences_and_bytes() {
    covers(&[
        "ERR:compress/zstd_ldm.c:606",
        "ERR:compress/zstd_ldm.c:664",
    ]);
    // A deterministic set of stores: varying litLength/matchLength shapes,
    // including zero-length literals and matches shorter than minMatch.
    let shapes: Vec<Vec<rawSeq>> = vec![
        vec![],
        vec![rawSeq { offset: 5, litLength: 0, matchLength: 64 }],
        vec![rawSeq { offset: 5, litLength: 10, matchLength: 3 }],
        vec![
            rawSeq { offset: 7, litLength: 4, matchLength: 70 },
            rawSeq { offset: 9, litLength: 0, matchLength: 65 },
            rawSeq { offset: 3, litLength: 100, matchLength: 64 },
        ],
        vec![
            rawSeq { offset: 1, litLength: 1, matchLength: 1 },
            rawSeq { offset: 2, litLength: 2, matchLength: 2 },
            rawSeq { offset: 3, litLength: 3, matchLength: 3 },
            rawSeq { offset: 4, litLength: 4, matchLength: 4 },
        ],
    ];
    let skips: &[usize] = &[
        0,
        1,
        3,
        4,
        5,
        10,
        64,
        65,
        74,
        200,
        1 << 20,
        u32::MAX as usize,
        u32::MAX as usize + 1,
        usize::MAX / 4,
    ];
    for (si, shape) in shapes.iter().enumerate() {
        for &pos in &[0usize, 1] {
            if pos >= shape.len().max(1) {
                continue;
            }
            for &pis in &[0usize, 2] {
                for &skip in skips {
                    for &minmatch in &[3u32, 4, 64] {
                        diff_bytes(
                            &format!("ldmSkipSeq/s{si}/pos{pos}/pis{pis}/n{skip}/mm{minmatch}"),
                            |l| {
                                let mut seqs = shape.clone();
                                seqs.push(rawSeq::default()); // never empty
                                let n = shape.len();
                                let mut store = RawSeqStore_t {
                                    seq: seqs.as_mut_ptr(),
                                    pos: pos.min(n),
                                    posInSequence: pis,
                                    size: n,
                                    capacity: seqs.len(),
                                };
                                unsafe {
                                    l.sym::<FnLdmSkipSequences>("ZSTD_ldm_skipSequences")(
                                        &mut store, skip, minmatch,
                                    )
                                };
                                let bytes: Vec<u8> = seqs
                                    .iter()
                                    .flat_map(|s| {
                                        let mut v = Vec::with_capacity(12);
                                        v.extend_from_slice(&s.offset.to_le_bytes());
                                        v.extend_from_slice(&s.litLength.to_le_bytes());
                                        v.extend_from_slice(&s.matchLength.to_le_bytes());
                                        v
                                    })
                                    .collect();
                                (store.pos, store.posInSequence, store.size, Blob(bytes))
                            },
                        );
                        diff_bytes(
                            &format!("ldmSkipRaw/s{si}/pos{pos}/pis{pis}/n{skip}"),
                            |l| {
                                let mut seqs = shape.clone();
                                seqs.push(rawSeq::default());
                                let n = shape.len();
                                let mut store = RawSeqStore_t {
                                    seq: seqs.as_mut_ptr(),
                                    pos: pos.min(n),
                                    posInSequence: pis,
                                    size: n,
                                    capacity: seqs.len(),
                                };
                                unsafe {
                                    l.sym::<FnLdmSkipRawBytes>("ZSTD_ldm_skipRawSeqStoreBytes")(
                                        &mut store, skip,
                                    )
                                };
                                let bytes: Vec<u8> = seqs
                                    .iter()
                                    .flat_map(|s| {
                                        let mut v = Vec::with_capacity(12);
                                        v.extend_from_slice(&s.offset.to_le_bytes());
                                        v.extend_from_slice(&s.litLength.to_le_bytes());
                                        v.extend_from_slice(&s.matchLength.to_le_bytes());
                                        v
                                    })
                                    .collect();
                                (store.pos, store.posInSequence, store.size, Blob(bytes))
                            },
                        );
                    }
                }
            }
        }
    }
}

/// `ZSTD_ldm_blockCompress` (`compress/zstd_ldm.c:714`): `maybeSplitSequence`
/// returns a sequence with `offset == 0` (the rest of the block is literals),
/// which breaks out of the loop and hands the remainder to the plain block
/// compressor. This is a routine occurrence, not an error: it fires whenever an
/// LDM sequence extends past the end of the current block, so it is reached by
/// compressing multi-block input with LDM enabled at a strategy below
/// `ZSTD_btopt` (at/above `btopt` the LDM store is only a candidate source and
/// this loop is skipped entirely).
///
/// ERRORS 431.
#[test]
fn ldm_block_compress_offset_zero_break() {
    covers(&["ERR:compress/zstd_ldm.c:714"]);
    // 3 MB of long-range duplicated regions: matches routinely straddle the
    // 128 KB block boundary.
    let src = corpus(Corpus::LongRepeats, 3 * 1024 * 1024, 0x431);
    for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_greedy, ZSTD_lazy2, ZSTD_btlazy2] {
        for &minml in &[32i32, 64] {
            diff_bytes(&format!("ldmBlockCompress/s{strat}/m{minml}"), |l| {
                let cctx = Ctx::cctx(l);
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let mut sets = Vec::new();
                for (p, v) in [
                    (ZSTD_c_enableLongDistanceMatching, 1),
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_ldmMinMatch, minml),
                    (ZSTD_c_ldmHashLog, 16),
                    (ZSTD_c_ldmBucketSizeLog, 3),
                    (ZSTD_c_ldmHashRateLog, 4),
                    (ZSTD_c_windowLog, 22),
                ] {
                    sets.push(res(l, unsafe { set(cctx.ptr, p, v) }));
                }
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                let n = unsafe {
                    l.sym::<FnCompress2>("ZSTD_compress2")(
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
                (sets, r, Blob(dst))
            });
        }
    }
}

// ===========================================================================
// CONFIGS 117/118/119 — ZSTD_f_zstd1_magicless on the compression side
// ===========================================================================

type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT, c_int) -> SizeT;
type FnFrameHeaderSize = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_uint) -> SizeT;

/// Compress with the **C** library under a list of `ZSTD_CCtx_setParameter`
/// settings — used to produce fixtures both libraries then decode.
fn c_compress_with(src: &[u8], sets: &[(c_int, c_int)], dict: Option<&[u8]>) -> Vec<u8> {
    let l = &pair().c;
    let cctx = Ctx::cctx(l);
    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    for &(p, v) in sets {
        let n = unsafe { set(cctx.ptr, p, v) };
        assert!(!is_error(l, n), "C setParameter({p},{v}): {}", err_name(l, n));
    }
    if let Some(d) = dict {
        let n = unsafe {
            l.sym::<FnRefCDict2>("ZSTD_CCtx_loadDictionary")(
                cctx.ptr,
                d.as_ptr() as *const c_void,
                d.len(),
            )
        };
        assert!(!is_error(l, n), "C loadDictionary: {}", err_name(l, n));
    }
    let cap = compress_bound(l, src.len()) + 64;
    let mut dst = vec![0u8; cap];
    let n = unsafe {
        l.sym::<FnCompress2>("ZSTD_compress2")(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    assert!(!is_error(l, n), "C compress2: {}", err_name(l, n));
    dst.truncate(n);
    dst
}

type FnLoadDict2 = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;

/// `ZSTD_writeFrameHeader` (`compress/zstd_compress.c:4695`) with
/// `params->format != ZSTD_f_zstd1`: the 4-byte `ZSTD_MAGICNUMBER` is skipped.
///
/// * CONFIGS 117 — level 3, 100 KB: the magicless frame must be *exactly* 4
///   bytes shorter than the `ZSTD_f_zstd1` frame and otherwise byte-identical.
/// * CONFIGS 118 — the 2-byte frame-header corner: `contentSizeFlag=1`,
///   `checksumFlag=0`, `dictIDFlag=0`, `windowLog=10`, `srcSize=100` gives
///   `singleSegment=1` (no windowLog byte), `fcsCode==0` with `singleSegment`
///   (exactly 1 FCS byte) and a 0-byte dictID field -> `pos == 2`, i.e.
///   `ZSTD_FRAMEHEADERSIZE_MIN(magicless)`.
/// * CONFIGS 119 — the full `checksumFlag x contentSizeFlag x dictIDFlag` matrix
///   with a 4 KB dictionary over the sizes that straddle every `fcsCode` and
///   `dictIDSizeCodeLength` boundary; every combination selects a header length
///   in 2..14.
#[test]
fn magicless_frame_header_matrix() {
    covers(&["CFG:117", "CFG:118", "CFG:119"]);

    // ---- CONFIGS 117 -----------------------------------------------------
    let src = corpus(Corpus::Text, 100 * 1024, 0x117);
    let (plain, magicless) = diff_bytes("magicless/117", |l| {
        let mut out = Vec::new();
        for fmt in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
            let cctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let a = res(l, unsafe { set(cctx.ptr, ZSTD_c_format, fmt) });
            let b = res(l, unsafe { set(cctx.ptr, ZSTD_c_compressionLevel, 3) });
            assert!(matches!(a, R::Ok(_)) && matches!(b, R::Ok(_)));
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                l.sym::<FnCompress2>("ZSTD_compress2")(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            let r = res(l, n);
            assert!(matches!(r, R::Ok(_)), "fmt={fmt}: {r:?}");
            if let R::Ok(k) = r {
                dst.truncate(k);
            }
            out.push(Blob(dst));
        }
        (out.remove(0), out.remove(0))
    });
    assert_eq!(
        plain.0.len(),
        magicless.0.len() + 4,
        "the magicless frame must be exactly 4 bytes shorter"
    );
    assert_eq!(
        &plain.0[4..],
        &magicless.0[..],
        "the magicless frame must equal the zstd1 frame minus its magic"
    );
    assert_eq!(&plain.0[..4], &le32(ZSTD_MAGICNUMBER)[..]);

    // ---- CONFIGS 118 -----------------------------------------------------
    let tiny = corpus(Corpus::Text, 100, 0x118);
    let hdr118 = diff_bytes("magicless/118", |l| {
        let cctx = Ctx::cctx(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        let mut sets = Vec::new();
        for (p, v) in [
            (ZSTD_c_format, ZSTD_f_zstd1_magicless),
            (ZSTD_c_contentSizeFlag, 1),
            (ZSTD_c_checksumFlag, 0),
            (ZSTD_c_dictIDFlag, 0),
            (ZSTD_c_windowLog, 10),
        ] {
            sets.push(res(l, unsafe { set(cctx.ptr, p, v) }));
        }
        let cap = compress_bound(l, tiny.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let n = unsafe {
            l.sym::<FnCompress2>("ZSTD_compress2")(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                tiny.as_ptr() as *const c_void,
                tiny.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            dst.truncate(k);
        }
        // parse the header back with the magicless parser
        let mut h = ZSTD_FrameHeader::default();
        let hr = res(l, unsafe {
            l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced")(
                &mut h,
                dst.as_ptr() as *const c_void,
                dst.len(),
                ZSTD_f_zstd1_magicless,
            )
        });
        (sets, r, hr, h, Blob(dst))
    });
    assert_eq!(
        hdr118.3.headerSize, 2,
        "expected a 2-byte magicless frame header, got {:?}",
        hdr118.3
    );

    // ---- CONFIGS 119 -----------------------------------------------------
    let dict = corpus(Corpus::Text, 4096, 0x119);
    for &n in &[0usize, 255, 256, 65791, 65792, 1 << 20] {
        let body = corpus(Corpus::Text, n, 0x1190);
        for cs in [0i32, 1] {
            for csf in [0i32, 1] {
                for did in [0i32, 1] {
                    let label = format!("magicless/119/n{n}/chk{cs}/csf{csf}/did{did}");
                    let hdr = diff_bytes(&label, |l| {
                        let cctx = Ctx::cctx(l);
                        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                        let mut sets = Vec::new();
                        for (p, v) in [
                            (ZSTD_c_format, ZSTD_f_zstd1_magicless),
                            (ZSTD_c_checksumFlag, cs),
                            (ZSTD_c_contentSizeFlag, csf),
                            (ZSTD_c_dictIDFlag, did),
                        ] {
                            sets.push(res(l, unsafe { set(cctx.ptr, p, v) }));
                        }
                        sets.push(res(l, unsafe {
                            l.sym::<FnLoadDict2>("ZSTD_CCtx_loadDictionary")(
                                cctx.ptr,
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                            )
                        }));
                        let cap = compress_bound(l, body.len()) + 64;
                        let mut dst = vec![0xCDu8; cap];
                        let k = unsafe {
                            l.sym::<FnCompress2>("ZSTD_compress2")(
                                cctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                body.as_ptr() as *const c_void,
                                body.len(),
                            )
                        };
                        let r = res(l, k);
                        if let R::Ok(m) = r {
                            dst.truncate(m);
                        }
                        let mut h = ZSTD_FrameHeader::default();
                        let hr = res(l, unsafe {
                            l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced")(
                                &mut h,
                                dst.as_ptr() as *const c_void,
                                dst.len(),
                                ZSTD_f_zstd1_magicless,
                            )
                        });
                        // and round-trip it through a magicless DCtx
                        let dctx = Ctx::dctx(l);
                        let d0 = res(l, unsafe {
                            l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter")(
                                dctx.ptr,
                                ZSTD_d_format,
                                ZSTD_f_zstd1_magicless,
                            )
                        });
                        let mut out = vec![0xEEu8; body.len() + 64];
                        let dn = unsafe {
                            l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict")(
                                dctx.ptr,
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                dst.as_ptr() as *const c_void,
                                dst.len(),
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                            )
                        };
                        let dr = res(l, dn);
                        if let R::Ok(m) = dr {
                            out.truncate(m);
                        }
                        (sets, r, hr, h, d0, dr, Blob(out))
                    });
                    assert!(
                        (2..=14).contains(&hdr.3.headerSize),
                        "{label}: magicless header size {} outside 2..14",
                        hdr.3.headerSize
                    );
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS 123/124 — ZSTD_f_zstd1_magicless on the decompression side
// ===========================================================================

/// `ZSTD_decompressMultiFrame` loops while `srcSize >=
/// ZSTD_startingInputLength(format)` (`decompress/zstd_decompress.c:232`), which
/// is **1** for `ZSTD_f_zstd1_magicless` and 5 for `ZSTD_f_zstd1`. Trailing
/// garbage is therefore parsed as a new frame in the magicless case, whereas the
/// `zstd1` case reports `srcSize_wrong` once fewer than 5 bytes remain.
///
/// CONFIGS 123.
#[test]
fn magicless_dctx_trailing_garbage() {
    covers(&["CFG:123"]);
    let src = corpus(Corpus::Text, 8000, 0x123);
    let dict = corpus(Corpus::Text, 4096, 0x1230);
    let magicless = c_compress_with(
        &src,
        &[
            (ZSTD_c_format, ZSTD_f_zstd1_magicless),
            (ZSTD_c_compressionLevel, 5),
        ],
        Some(&dict),
    );
    let plain = c_compress_with(&src, &[(ZSTD_c_compressionLevel, 5)], Some(&dict));

    for garbage_len in 0..=6usize {
        for &fill in &[0x00u8, 0x01, 0x55, 0xFF] {
            for (name, base, fmt) in [
                ("magicless", &magicless, ZSTD_f_zstd1_magicless),
                ("zstd1", &plain, ZSTD_f_zstd1),
            ] {
                let mut frame = base.clone();
                frame.extend(std::iter::repeat(fill).take(garbage_len));
                diff_bytes(
                    &format!("magiclessTrail/{name}/g{garbage_len}/f{fill:02x}"),
                    |l| {
                        let dctx = Ctx::dctx(l);
                        let d0 = res(l, unsafe {
                            l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter")(
                                dctx.ptr,
                                ZSTD_d_format,
                                fmt,
                            )
                        });
                        let mut out = vec![0xEEu8; src.len() * 2 + 64];
                        let n = unsafe {
                            l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict")(
                                dctx.ptr,
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                frame.as_ptr() as *const c_void,
                                frame.len(),
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                            )
                        };
                        let r = res(l, n);
                        if let R::Ok(k) = r {
                            out.truncate(k);
                        } else {
                            out.truncate(0);
                        }
                        (d0, r, Blob(out))
                    },
                );
            }
        }
    }
}

/// A magicless DCtx fed a well-formed skippable frame: the skippable branch of
/// `ZSTD_decompressStream` / `ZSTD_decompressFrame` is guarded by
/// `zds->format == ZSTD_f_zstd1`, so `0x50 0x2A 0x4D 0x18` is consumed as a frame
/// header descriptor instead. The exact error code is pinned here.
///
/// CONFIGS 124.
#[test]
fn magicless_dctx_fed_skippable_frame() {
    covers(&["CFG:124"]);
    // Build the skippable frames with the C library.
    let payload = corpus(Corpus::Text, 64, 0x124);
    let mut frames: Vec<(String, Vec<u8>)> = Vec::new();
    {
        let l = &pair().c;
        let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
        for variant in [0u32, 1, 15] {
            for plen in [0usize, 1, 64] {
                let mut buf = vec![0u8; plen + 32];
                let n = unsafe {
                    f(
                        buf.as_mut_ptr() as *mut c_void,
                        buf.len(),
                        payload.as_ptr() as *const c_void,
                        plen,
                        variant,
                    )
                };
                assert!(!is_error(l, n), "writeSkippableFrame: {}", err_name(l, n));
                buf.truncate(n);
                frames.push((format!("v{variant}/p{plen}"), buf));
            }
        }
    }
    for (label, frame) in frames {
        for fmt in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
            // one-shot
            diff_bytes(&format!("skippable/{label}/fmt{fmt}/oneshot"), |l| {
                let dctx = Ctx::dctx(l);
                let d0 = res(l, unsafe {
                    l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter")(
                        dctx.ptr,
                        ZSTD_d_format,
                        fmt,
                    )
                });
                let mut out = vec![0xEEu8; 4096];
                let n = unsafe {
                    l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                        dctx.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                    )
                };
                let r = res(l, n);
                (d0, r, Blob(out))
            });
            // streaming, one byte at a time
            diff_bytes(&format!("skippable/{label}/fmt{fmt}/stream"), |l| {
                let dctx = Ctx::dstream(l);
                let d0 = res(l, unsafe {
                    l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter")(
                        dctx.ptr,
                        ZSTD_d_format,
                        fmt,
                    )
                });
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xEEu8; 4096];
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut steps = Vec::new();
                let mut fed = 0usize;
                while fed < frame.len() {
                    let mut i = ZSTD_inBuffer {
                        src: unsafe { frame.as_ptr().add(fed) } as *const c_void,
                        size: 1,
                        pos: 0,
                    };
                    let r = res(l, unsafe { f(dctx.ptr, &mut o, &mut i) });
                    let stop = matches!(r, R::Err(..));
                    steps.push(r);
                    fed += i.pos;
                    if stop {
                        break;
                    }
                    if i.pos == 0 {
                        break;
                    }
                }
                (d0, steps, o.pos, Blob(out))
            });
        }
    }
}

// ===========================================================================
// CONFIGS 262/274/276/289/290/293 — low-level FSE / HUF entry points
// ===========================================================================

const FSE_MAX_TABLELOG: u32 = 12;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_TABLELOG_MAX: u32 = 12;
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * 256) + 192) * 4;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;
const HUF_flags_bmi2: c_int = 1 << 0;
const HUF_flags_optimalDepth: c_int = 1 << 1;
const HUF_flags_preferRepeat: c_int = 1 << 2;
const HUF_flags_suspectUncompressible: c_int = 1 << 3;
const HUF_flags_disableAsm: c_int = 1 << 4;
const HUF_flags_disableFast: c_int = 1 << 5;

fn fse_ctable_size_u32(max_table_log: u32, msv: u32) -> usize {
    1 + (1usize << (max_table_log.max(1) - 1)) + ((msv as usize + 1) * 2)
}
fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
fn fse_build_dtable_wksp_size_u32(max_table_log: u32, msv: u32) -> usize {
    ((msv as usize + 2 + (1usize << (max_table_log / 2))) / 2) + 1
}
fn fse_decompress_wksp_size_u32(max_table_log: u32, msv: u32) -> usize {
    fse_dtable_size_u32(max_table_log)
        + 1
        + fse_build_dtable_wksp_size_u32(max_table_log, msv)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}

/// A `HUF_DTable` seeded the way the library's own callers do:
/// `DTable[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x01000001`.
fn huf_dtable() -> Vec<u32> {
    let mut dt = vec![0u32; 1 + (1usize << ZSTD_HUFFDTABLE_CAPACITY_LOG)];
    dt[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x0100_0001;
    dt
}

type FnHistCount = unsafe extern "C" fn(*mut u32, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnFseOptimalTableLog = unsafe extern "C" fn(c_uint, SizeT, c_uint) -> c_uint;
type FnFseNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const u32, SizeT, c_uint, c_uint) -> SizeT;
type FnFseBuildCTableWksp =
    unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnFseCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const u32) -> SizeT;
type FnSzSz = unsafe extern "C" fn(SizeT) -> SizeT;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut u64, *const u32, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnHufWriteCTableWksp = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const u64,
    c_uint,
    c_uint,
    *mut c_void,
    SizeT,
) -> SizeT;
type FnHufCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const u64, c_int) -> SizeT;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;

/// An FSE frame (normalized-count header ++ bitstream) plus the header length,
/// built entirely with the **C** library.
fn c_fse_frame(src: &[u8]) -> (Vec<u8>, usize) {
    let l = &pair().c;
    let mut count = vec![0u32; 256];
    let mut msv = 255u32;
    unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let tl = unsafe {
        l.sym::<FnFseOptimalTableLog>("FSE_optimalTableLog")(FSE_MAX_TABLELOG, src.len(), msv)
    };
    let mut norm = vec![0i16; 256];
    let n = unsafe {
        l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
            norm.as_mut_ptr(),
            tl,
            count.as_ptr(),
            src.len(),
            msv,
            0,
        )
    };
    assert!(!is_error(l, n), "FSE_normalizeCount: {}", err_name(l, n));
    let tl = n as u32;
    let mut hdr = vec![0u8; 512];
    let hn = unsafe {
        l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
            hdr.as_mut_ptr() as *mut c_void,
            hdr.len(),
            norm.as_ptr(),
            msv,
            tl,
        )
    };
    assert!(!is_error(l, hn), "FSE_writeNCount: {}", err_name(l, hn));
    hdr.truncate(hn);
    let mut ct = vec![0u32; fse_ctable_size_u32(tl, msv)];
    let mut w = wksp(1 << 16);
    let bn = unsafe {
        l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
            ct.as_mut_ptr(),
            norm.as_ptr(),
            msv,
            tl,
            wksp_ptr(&mut w),
            wksp_bytes(&w),
        )
    };
    assert!(!is_error(l, bn), "FSE_buildCTable_wksp: {}", err_name(l, bn));
    let bound = unsafe { l.sym::<FnSzSz>("FSE_compressBound")(src.len()) };
    let mut payload = vec![0u8; bound + 64];
    let cn = unsafe {
        l.sym::<FnFseCompressUsingCTable>("FSE_compress_usingCTable")(
            payload.as_mut_ptr() as *mut c_void,
            payload.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            ct.as_ptr(),
        )
    };
    assert!(
        !is_error(l, cn) && cn > 0,
        "FSE_compress_usingCTable: {} ({cn})",
        err_name(l, cn)
    );
    payload.truncate(cn);
    let hlen = hdr.len();
    let mut frame = hdr;
    frame.extend_from_slice(&payload);
    (frame, hlen)
}

/// `FSE_decompress_wksp_bmi2` (`decompress/fse_decompress.c`): `BIT_initDStream`
/// on a zero-size buffer gives `srcSize_wrong`, and a payload truncated by one
/// byte trips `BIT_reloadDStream == BIT_DStream_overflow` -> `corruption_detected`.
/// `cSrcSize` exactly equal to the NCount length leaves 0 payload bytes.
///
/// CONFIGS 262.
#[test]
fn fse_decompress_wksp_truncated_streams() {
    covers(&["CFG:262"]);
    for &kind in &[Corpus::Text, Corpus::SmallAlphabet, Corpus::Mixed] {
        for &n in &[64usize, 1000, 20_000] {
            let src = corpus(kind, n, 0x262);
            let (frame, hlen) = c_fse_frame(&src);
            let sizes: Vec<usize> = vec![
                0,
                1,
                hlen.saturating_sub(1),
                hlen,
                hlen + 1,
                frame.len() - 1,
                frame.len(),
            ];
            for cs in sizes {
                if cs > frame.len() {
                    continue;
                }
                for &maxlog in &[FSE_MAX_TABLELOG, FSE_TABLELOG_ABSOLUTE_MAX, 5] {
                    for bmi2 in [0i32, 1] {
                        diff_bytes(
                            &format!("fseDecWksp/{kind:?}/n{n}/cs{cs}/log{maxlog}/b{bmi2}"),
                            |l| {
                                let mut dst = vec![0xBBu8; src.len() + 64];
                                let mut w = wksp(
                                    fse_decompress_wksp_size_u32(
                                        FSE_TABLELOG_ABSOLUTE_MAX,
                                        FSE_MAX_SYMBOL_VALUE,
                                    ) * 4
                                        + 64,
                                );
                                let k = unsafe {
                                    l.sym::<FnFseDecompressWksp>("FSE_decompress_wksp_bmi2")(
                                        dst.as_mut_ptr() as *mut c_void,
                                        src.len(),
                                        frame.as_ptr() as *const c_void,
                                        cs,
                                        maxlog,
                                        wksp_ptr(&mut w),
                                        wksp_bytes(&w),
                                        bmi2,
                                    )
                                };
                                (res(l, k), Blob(dst))
                            },
                        );
                    }
                }
            }
        }
    }
}

/// `HUF_readStats` / `HUF_readStats_wksp` (`common/entropy_common.c:270..300`):
/// the five distinct `corruption_detected` returns.
///
/// * a weight of 13 > `HUF_TABLELOG_MAX` (12);
/// * all-zero weights so `weightTotal == 0`;
/// * a `weightTotal` whose complement is not a power of two (`verif != rest`);
/// * `rankStats[1] < 2` (no rank-1 weight at all).
///
/// The fifth listed case — `rankStats[1] & 1`, an *odd* number of rank-1
/// weights — is UNREACHABLE: each weight `w` adds `(1<<w)>>1` to `weightTotal`,
/// so only `w == 1` contributes an odd amount and `weightTotal` has the same
/// parity as `rankStats[1]`. If that parity is odd then `rest = 2^tableLog -
/// weightTotal` is odd, and `verif == rest` then forces `rest == 1`, i.e.
/// `lastWeight == 1`, which increments `rankStats[1]` to an even value. So the
/// `verif != rest` check at `:294` dominates the odd-count branch. Three
/// rank-1 weights are still driven here, to pin that behaviour.
///
/// CONFIGS 274.
#[test]
fn huf_read_stats_corruption_kinds() {
    covers(&["CFG:274"]);
    let cases: &[(&str, Vec<u8>)] = &[
        ("valid-4x1", vec![1, 1, 1, 1]),
        ("valid-2x1", vec![1, 1]),
        ("weight13", vec![13, 1]),
        ("weight15", vec![15, 1]),
        ("all-zero", vec![0, 0, 0, 0]),
        ("no-rank1", vec![2, 2]),
        ("verif-mismatch", vec![1, 2, 2]),
        ("three-rank1", vec![1, 1, 1, 2]),
        ("five-rank1", vec![1, 1, 1, 1, 1, 2]),
        ("single-weight", vec![1]),
        ("mixed", vec![4, 3, 2, 1, 1, 1, 1]),
    ];
    for (label, weights) in cases {
        let raw = huf_raw_weights(weights);
        // every truncation of the serialised header, too
        for cs in 0..=raw.len() {
            diff_bytes(&format!("hufReadStats/{label}/cs{cs}"), |l| {
                let mut hw = vec![0xEEu8; 256];
                let mut rank = vec![0u32; 16];
                let mut nb = 0u32;
                let mut tl = 0u32;
                let n = unsafe {
                    l.sym::<FnReadStats7>("HUF_readStats")(
                        hw.as_mut_ptr(),
                        hw.len(),
                        rank.as_mut_ptr(),
                        &mut nb,
                        &mut tl,
                        raw.as_ptr() as *const c_void,
                        cs,
                    )
                };
                let rank_bytes: Vec<u8> = rank.iter().flat_map(|v| v.to_le_bytes()).collect();
                (res(l, n), nb, tl, Blob(hw), Blob(rank_bytes))
            });
        }
        // hwSize smaller than the weight count -> corruption_detected
        for hwsize in [0usize, 1, 2, 4, 8] {
            diff_bytes(&format!("hufReadStats/{label}/hw{hwsize}"), |l| {
                let mut hw = vec![0xEEu8; 260];
                let mut rank = vec![0u32; 16];
                let mut nb = 0u32;
                let mut tl = 0u32;
                let n = unsafe {
                    l.sym::<FnReadStats7>("HUF_readStats")(
                        hw.as_mut_ptr(),
                        hwsize,
                        rank.as_mut_ptr(),
                        &mut nb,
                        &mut tl,
                        raw.as_ptr() as *const c_void,
                        raw.len(),
                    )
                };
                (res(l, n), nb, tl, Blob(hw))
            });
        }
    }
    // Pin the four reachable rejections.
    for (label, weights, want_err) in [
        ("weight13", vec![13u8, 1], true),
        ("all-zero", vec![0, 0, 0, 0], true),
        ("no-rank1", vec![2, 2], true),
        ("verif-mismatch", vec![1, 2, 2], true),
        ("valid-4x1", vec![1, 1, 1, 1], false),
    ] {
        let raw = huf_raw_weights(&weights);
        let got = diff(&format!("hufReadStats/pin/{label}"), |l| {
            let mut hw = vec![0u8; 256];
            let mut rank = vec![0u32; 16];
            let mut nb = 0u32;
            let mut tl = 0u32;
            res(l, unsafe {
                l.sym::<FnReadStats7>("HUF_readStats")(
                    hw.as_mut_ptr(),
                    hw.len(),
                    rank.as_mut_ptr(),
                    &mut nb,
                    &mut tl,
                    raw.as_ptr() as *const c_void,
                    raw.len(),
                )
            })
        });
        if want_err {
            assert!(
                matches!(got, R::Err(20, _)),
                "{label}: expected corruption_detected(20), got {got:?}"
            );
        } else {
            assert!(matches!(got, R::Ok(_)), "{label}: got {got:?}");
        }
    }
}

/// `HUF_compress1X_repeat` (`compress/huf_compress.c`): the six guards, in the
/// fixed order `wkspSize < sizeof(HUF_compress_tables_t)` -> `workSpace_tooSmall`,
/// `!srcSize` -> 0, `!dstSize` -> 0, `srcSize > HUF_BLOCKSIZE_MAX (131072)` ->
/// `srcSize_wrong`, `huffLog > HUF_TABLELOG_MAX (12)` -> `tableLog_tooLarge`,
/// `maxSymbolValue > HUF_SYMBOLVALUE_MAX (255)` -> `maxSymbolValue_tooLarge`.
///
/// CONFIGS 276.
#[test]
fn huf_compress1x_repeat_guards() {
    covers(&["CFG:276"]);
    let src = corpus(Corpus::Text, 4096, 0x276);
    let big = corpus(Corpus::Text, 131_073, 0x2760);
    let full_wksp = HUF_WORKSPACE_SIZE;
    struct Case<'a> {
        label: &'a str,
        src: &'a [u8],
        src_size: usize,
        dst_size: usize,
        msv: u32,
        hlog: u32,
        wksp: usize,
    }
    let cases: Vec<Case> = vec![
        Case { label: "baseline", src: &src, src_size: src.len(), dst_size: 8192, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "srcSize0", src: &src, src_size: 0, dst_size: 8192, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "dstSize0", src: &src, src_size: src.len(), dst_size: 0, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "src131073", src: &big, src_size: big.len(), dst_size: 1 << 18, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "src131072", src: &big, src_size: 131_072, dst_size: 1 << 18, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "huffLog13", src: &src, src_size: src.len(), dst_size: 8192, msv: 255, hlog: 13, wksp: full_wksp },
        Case { label: "huffLog12", src: &src, src_size: src.len(), dst_size: 8192, msv: 255, hlog: 12, wksp: full_wksp },
        Case { label: "msv256", src: &src, src_size: src.len(), dst_size: 8192, msv: 256, hlog: 11, wksp: full_wksp },
        Case { label: "wksp-1", src: &src, src_size: src.len(), dst_size: 8192, msv: 255, hlog: 11, wksp: full_wksp - 1 },
        Case { label: "wksp0", src: &src, src_size: src.len(), dst_size: 8192, msv: 255, hlog: 11, wksp: 0 },
        // several guards at once, to pin the ORDER they are applied in
        Case { label: "wksp-1+src0", src: &src, src_size: 0, dst_size: 8192, msv: 255, hlog: 11, wksp: full_wksp - 1 },
        Case { label: "src0+dst0", src: &src, src_size: 0, dst_size: 0, msv: 255, hlog: 11, wksp: full_wksp },
        Case { label: "dst0+huffLog13", src: &src, src_size: src.len(), dst_size: 0, msv: 255, hlog: 13, wksp: full_wksp },
        Case { label: "big+huffLog13", src: &big, src_size: big.len(), dst_size: 1 << 18, msv: 255, hlog: 13, wksp: full_wksp },
        Case { label: "huffLog13+msv256", src: &src, src_size: src.len(), dst_size: 8192, msv: 256, hlog: 13, wksp: full_wksp },
    ];
    for c in &cases {
        for flags in [0, HUF_flags_preferRepeat, HUF_flags_suspectUncompressible] {
            diff_bytes(&format!("hufC1XRepeat/{}/f{flags}", c.label), |l| {
                let mut dst = vec![0xCDu8; c.dst_size.max(1) + 64];
                let mut w = wksp(c.wksp.max(1));
                let mut ctable = vec![0u64; 257];
                let mut repeat: c_int = 0; // HUF_repeat_none
                let n = unsafe {
                    l.sym::<FnHufC1XRepeat>("HUF_compress1X_repeat")(
                        dst.as_mut_ptr() as *mut c_void,
                        c.dst_size,
                        c.src.as_ptr() as *const c_void,
                        c.src_size,
                        c.msv,
                        c.hlog,
                        wksp_ptr(&mut w),
                        c.wksp,
                        ctable.as_mut_ptr(),
                        &mut repeat,
                        flags,
                    )
                };
                let ct_bytes: Vec<u8> = ctable.iter().flat_map(|v| v.to_le_bytes()).collect();
                (res(l, n), repeat, Blob(dst), Blob(ct_bytes))
            });
        }
    }
}

/// A HUF fixture built with the **C** library: the serialised CTable header, the
/// 1-stream payload and the 4-stream payload (6-byte jump table ++ 4 streams).
struct HufFix {
    hdr: Vec<u8>,
    c1x: Vec<u8>,
    c4x: Vec<u8>,
    src: Vec<u8>,
}

fn c_huf_fixture(src: &[u8], max_nb_bits: u32) -> HufFix {
    let l = &pair().c;
    let mut count = vec![0u32; 256];
    let mut msv = 255u32;
    unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let mut ct = vec![0u64; 257];
    let mut cw = wksp(HUF_CTABLE_WORKSPACE_SIZE);
    let log = unsafe {
        l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
            ct.as_mut_ptr(),
            count.as_ptr(),
            msv,
            max_nb_bits,
            wksp_ptr(&mut cw),
            wksp_bytes(&cw),
        )
    };
    assert!(!is_error(l, log), "HUF_buildCTable_wksp: {}", err_name(l, log));
    let mut hdr = vec![0u8; 512];
    let mut hw = wksp(HUF_CTABLE_WORKSPACE_SIZE);
    let hn = unsafe {
        l.sym::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp")(
            hdr.as_mut_ptr() as *mut c_void,
            hdr.len(),
            ct.as_ptr(),
            msv,
            log as c_uint,
            wksp_ptr(&mut hw),
            wksp_bytes(&hw),
        )
    };
    assert!(!is_error(l, hn), "HUF_writeCTable_wksp: {}", err_name(l, hn));
    hdr.truncate(hn);
    let bound = unsafe { l.sym::<FnSzSz>("HUF_compressBound")(src.len()) };
    let enc = |sym: &str| {
        let mut d = vec![0u8; bound + 128];
        let n = unsafe {
            l.sym::<FnHufCompressUsingCTable>(sym)(
                d.as_mut_ptr() as *mut c_void,
                d.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ct.as_ptr(),
                0,
            )
        };
        assert!(
            !is_error(l, n) && n > 0,
            "{sym}: {} ({n})",
            err_name(l, n)
        );
        d[..n].to_vec()
    };
    HufFix {
        hdr,
        c1x: enc("HUF_compress1X_usingCTable"),
        c4x: enc("HUF_compress4X_usingCTable"),
        src: src.to_vec(),
    }
}

/// `HUF_decompress1X_DCtx_wksp` (`decompress/huf_decompress.c:1845..1854`): the
/// four early exits taken before any table is read — `dstSize == 0` ->
/// `dstSize_tooSmall`, `cSrcSize > dstSize` -> `corruption_detected`,
/// `cSrcSize == dstSize` -> plain `memcpy` returning `dstSize`, `cSrcSize == 1`
/// -> `memset` (RLE) returning `dstSize` — plus `cSrcSize == 0`, which falls
/// through to `HUF_readDTableX1_wksp` on an empty buffer.
///
/// CONFIGS 289.
#[test]
fn huf_decompress1x_dctx_wksp_early_exits() {
    covers(&["CFG:289"]);
    let src = corpus(Corpus::SmallAlphabet, 2048, 0x289);
    let fix = c_huf_fixture(&src, 11);
    let mut frame = fix.hdr.clone();
    frame.extend_from_slice(&fix.c1x);

    let mut expected: Vec<(&str, R)> = Vec::new();
    for dst_size in [0usize, 1, 2, 3, 8, 100, fix.hdr.len(), frame.len(), src.len()] {
        for c_src in [
            0usize,
            1,
            2,
            3,
            dst_size,
            dst_size + 1,
            fix.hdr.len(),
            frame.len(),
        ] {
            if c_src > frame.len() {
                continue;
            }
            let got = diff_bytes(
                &format!("huf1XDCtx/dst{dst_size}/cSrc{c_src}"),
                |l| {
                    let mut dt = huf_dtable();
                    let mut w = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
                    let mut dst = vec![0xA1u8; src.len() + frame.len() + 64];
                    let n = unsafe {
                        l.sym::<FnHufDecompressXWksp>("HUF_decompress1X_DCtx_wksp")(
                            dt.as_mut_ptr() as *mut c_void,
                            dst.as_mut_ptr() as *mut c_void,
                            dst_size,
                            frame.as_ptr() as *const c_void,
                            c_src,
                            wksp_ptr(&mut w),
                            wksp_bytes(&w),
                            0,
                        )
                    };
                    (res(l, n), Blob(dst))
                },
            );
            // Pin the four documented early exits.
            if dst_size == 0 {
                assert!(
                    matches!(got.0, R::Err(70, _)),
                    "dstSize==0 must be dstSize_tooSmall(70), got {:?}",
                    got.0
                );
                expected.push(("dstSize0", got.0));
            } else if c_src > dst_size {
                assert!(
                    matches!(got.0, R::Err(20, _)),
                    "cSrcSize>dstSize must be corruption_detected(20), got {:?}",
                    got.0
                );
                expected.push(("cSrc>dst", got.0));
            } else if c_src == dst_size {
                assert!(
                    matches!(got.0, R::Ok(n) if n == dst_size),
                    "cSrcSize==dstSize must memcpy and return dstSize, got {:?}",
                    got.0
                );
                expected.push(("cSrc==dst", got.0));
            } else if c_src == 1 {
                assert!(
                    matches!(got.0, R::Ok(n) if n == dst_size),
                    "cSrcSize==1 must RLE-fill and return dstSize, got {:?}",
                    got.0
                );
                expected.push(("rle", got.0));
            }
        }
    }
    for want in ["dstSize0", "cSrc>dst", "cSrc==dst", "rle"] {
        assert!(
            expected.iter().any(|(k, _)| *k == want),
            "the sweep never exercised the {want} early exit"
        );
    }
}

/// `HUF_decompress4X_hufOnly_wksp` (`decompress/huf_decompress.c:1924..1928`):
/// `dstSize == 0` -> `dstSize_tooSmall` and `cSrcSize == 0` ->
/// `corruption_detected` (unlike the 1X entry point there is no memcpy/RLE
/// shortcut), then `hSize >= cSrcSize` -> `srcSize_wrong` inside the
/// `HUF_decompress4X1/4X2_DCtx_wksp` wrappers, with `HUF_selectDecoder`
/// returning both 0 and 1 across the sweep.
///
/// CONFIGS 290.
#[test]
fn huf_decompress4x_hufonly_wksp_guards() {
    covers(&["CFG:290"]);
    // Two very different alphabets, so HUF_selectDecoder (which compares the
    // ratio dstSize/cSrcSize against a table of thresholds) picks X1 for one and
    // X2 for the other.
    for (name, src) in [
        ("smallalpha", corpus(Corpus::SmallAlphabet, 4096, 0x290)),
        ("text", corpus(Corpus::Text, 4096, 0x2901)),
        ("periodic", corpus(Corpus::Periodic, 4096, 0x2902)),
    ] {
        let fix = c_huf_fixture(&src, 11);
        let mut frame = fix.hdr.clone();
        frame.extend_from_slice(&fix.c4x);
        let mut saw_dst0 = false;
        let mut saw_csrc0 = false;
        let mut saw_hsize = false;
        for dst_size in [0usize, 1, 6, 7, 64, 1000, src.len(), src.len() * 4] {
            for c_src in [
                0usize,
                1,
                2,
                9,
                10,
                fix.hdr.len(),
                fix.hdr.len() + 1,
                frame.len() / 2,
                frame.len(),
            ] {
                if c_src > frame.len() {
                    continue;
                }
                let got = diff_bytes(
                    &format!("huf4XHufOnly/{name}/dst{dst_size}/cSrc{c_src}"),
                    |l| {
                        let mut dt = huf_dtable();
                        let mut w = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
                        let mut dst = vec![0xA4u8; src.len() * 4 + 64];
                        let n = unsafe {
                            l.sym::<FnHufDecompressXWksp>("HUF_decompress4X_hufOnly_wksp")(
                                dt.as_mut_ptr() as *mut c_void,
                                dst.as_mut_ptr() as *mut c_void,
                                dst_size,
                                frame.as_ptr() as *const c_void,
                                c_src,
                                wksp_ptr(&mut w),
                                wksp_bytes(&w),
                                0,
                            )
                        };
                        (res(l, n), Blob(dst))
                    },
                );
                if dst_size == 0 {
                    assert!(
                        matches!(got.0, R::Err(70, _)),
                        "dstSize==0: expected dstSize_tooSmall(70), got {:?}",
                        got.0
                    );
                    saw_dst0 = true;
                } else if c_src == 0 {
                    assert!(
                        matches!(got.0, R::Err(20, _)),
                        "cSrcSize==0: expected corruption_detected(20), got {:?}",
                        got.0
                    );
                    saw_csrc0 = true;
                } else if c_src == fix.hdr.len() {
                    // the table header consumes exactly cSrcSize bytes
                    assert!(
                        matches!(got.0, R::Err(72, _)),
                        "hSize==cSrcSize: expected srcSize_wrong(72), got {:?}",
                        got.0
                    );
                    saw_hsize = true;
                }
            }
        }
        assert!(saw_dst0 && saw_csrc0 && saw_hsize, "{name}: guard not reached");
    }
}

/// `HUF_decompress4X_usingDTable` (`decompress/huf_decompress.c:1907`):
/// `HUF_initRemainingDStream` (`:284`/`:291`) returns `corruption_detected` when
/// `args->op[stream] > segmentEnd` or `args->ip[stream] < args->iend[stream]-8`,
/// and the scalar tail loop reports `corruption_detected` via
/// `BIT_endOfDStream`. Driven with (a) truncations inside every stream, (b) a
/// mutated 6-byte jump table so stream 2 runs into stream 3, and (c) a `dstSize`
/// that makes a stream decode more bytes than its segment.
///
/// CONFIGS 293.
#[test]
fn huf_decompress4x_usingdtable_stream_corruption() {
    covers(&["CFG:293"]);
    let src = corpus(Corpus::SmallAlphabet, 4096, 0x293);
    let fix = c_huf_fixture(&src, 11);
    assert!(fix.c4x.len() > 10, "need a real 4-stream payload");
    let l1 = u16::from_le_bytes([fix.c4x[0], fix.c4x[1]]) as usize;
    let l2 = u16::from_le_bytes([fix.c4x[2], fix.c4x[3]]) as usize;
    let l3 = u16::from_le_bytes([fix.c4x[4], fix.c4x[5]]) as usize;

    // Read the DTable once from the serialised header; decoding with an
    // unpopulated DTable is out of contract (`tableLog == 0` makes
    // `HUF_decodeSymbolX1` index with the whole bit container).
    let payloads: Vec<(String, Vec<u8>, usize)> = {
        let mut v: Vec<(String, Vec<u8>, usize)> = Vec::new();
        v.push(("full".into(), fix.c4x.clone(), src.len()));
        // (a) truncate inside each stream, and just past the jump table
        for cut in [
            6usize,
            7,
            6 + l1 / 2,
            6 + l1,
            6 + l1 + l2 / 2,
            6 + l1 + l2,
            6 + l1 + l2 + l3 / 2,
            6 + l1 + l2 + l3,
            fix.c4x.len() - 1,
        ] {
            if cut == 0 || cut > fix.c4x.len() {
                continue;
            }
            v.push((format!("trunc{cut}"), fix.c4x[..cut].to_vec(), src.len()));
        }
        // (b) mutate the jump table so stream 2 overruns into stream 3
        for (name, d1, d2, d3) in [
            ("jt/len1+8", 8i64, 0, 0),
            ("jt/len2+8", 0, 8, 0),
            ("jt/len3+8", 0, 0, 8),
            ("jt/len1-8", -8, 0, 0),
            ("jt/len2-8", 0, -8, 0),
            ("jt/len2=all", 0, (fix.c4x.len() - 6) as i64 - l2 as i64, 0),
            ("jt/zeroed", -(l1 as i64), -(l2 as i64), -(l3 as i64)),
        ] {
            let mut p = fix.c4x.clone();
            let set = |p: &mut Vec<u8>, off: usize, base: usize, delta: i64| {
                let nv = (base as i64 + delta).clamp(0, u16::MAX as i64) as u16;
                p[off..off + 2].copy_from_slice(&nv.to_le_bytes());
            };
            set(&mut p, 0, l1, d1);
            set(&mut p, 2, l2, d2);
            set(&mut p, 4, l3, d3);
            v.push((name.into(), p, src.len()));
        }
        v
    };

    for (label, payload, dst_size) in &payloads {
        // (c) also vary dstSize so a stream decodes past its segment
        for ds in [*dst_size, dst_size / 2, dst_size + 4, 6, 7] {
            for flags in [0, HUF_flags_disableAsm, HUF_flags_disableFast] {
                diff_bytes(
                    &format!("huf4XusingDT/{label}/dst{ds}/f{flags}"),
                    |l| {
                        let mut dt = huf_dtable();
                        let mut w = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
                        let rd = unsafe {
                            l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
                                dt.as_mut_ptr() as *mut c_void,
                                fix.hdr.as_ptr() as *const c_void,
                                fix.hdr.len(),
                                wksp_ptr(&mut w),
                                wksp_bytes(&w),
                                flags,
                            )
                        };
                        let rr = res(l, rd);
                        if !matches!(rr, R::Ok(_)) {
                            return (rr, R::Ok(0), Blob(vec![]));
                        }
                        let mut dst = vec![0xA4u8; src.len() * 2 + 64];
                        let n = unsafe {
                            l.sym::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable")(
                                dst.as_mut_ptr() as *mut c_void,
                                ds,
                                payload.as_ptr() as *const c_void,
                                payload.len(),
                                dt.as_ptr() as *const c_void,
                                flags,
                            )
                        };
                        (rr, res(l, n), Blob(dst))
                    },
                );
            }
        }
    }
    // The untouched payload at the right dstSize must decode back to the source.
    let ok = diff_bytes("huf4XusingDT/roundtrip", |l| {
        let mut dt = huf_dtable();
        let mut w = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
        let rd = unsafe {
            l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
                dt.as_mut_ptr() as *mut c_void,
                fix.hdr.as_ptr() as *const c_void,
                fix.hdr.len(),
                wksp_ptr(&mut w),
                wksp_bytes(&w),
                0,
            )
        };
        assert!(matches!(res(l, rd), R::Ok(_)));
        let mut dst = vec![0xA4u8; src.len()];
        let n = unsafe {
            l.sym::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable")(
                dst.as_mut_ptr() as *mut c_void,
                src.len(),
                fix.c4x.as_ptr() as *const c_void,
                fix.c4x.len(),
                dt.as_ptr() as *const c_void,
                0,
            )
        };
        (res(l, n), Blob(dst))
    });
    assert!(matches!(ok.0, R::Ok(n) if n == src.len()), "got {:?}", ok.0);
    assert_eq!(ok.1 .0, fix.src, "4-stream round-trip mismatch");
}

// ===========================================================================
// CONFIGS 51 — the suspectUncompressible literal-ratio shortcut
// ===========================================================================

/// `ZSTD_compressLiterals` computes
/// `suspectUncompressible = (numSequences == 0) || (litSize/numSequences >=
/// SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO (20))` and passes
/// `HUF_flags_suspectUncompressible` down to `HUF_compress1X_repeat` /
/// `HUF_compress4X_repeat`, which changes the Huffman table-building shortcut
/// inside `huf_compress.c`.
///
/// Three 131072-byte inputs pin both sides of the test: random data with exactly
/// one long match at the end (one sequence, ~130 KB of literals -> ratio far
/// above 20), pure random data (no sequences at all), and text (many sequences,
/// low ratio).
///
/// CONFIGS 51.
#[test]
fn suspect_uncompressible_literal_ratio() {
    covers(&["CFG:51"]);
    const N: usize = 131072;
    let mut one_match = corpus(Corpus::Random, N, 0x51);
    {
        // make the tail an exact copy of an earlier region: one long match
        let region = one_match[1000..5096].to_vec();
        let at = N - region.len();
        one_match[at..].copy_from_slice(&region);
    }
    let pure_random = corpus(Corpus::Random, N, 0x510);
    let text = corpus(Corpus::Text, N, 0x511);
    let inputs: &[(&str, &Vec<u8>)] = &[
        ("one-long-match", &one_match),
        ("pure-random", &pure_random),
        ("text", &text),
    ];
    for (name, src) in inputs {
        for lvl in [1i32, 19] {
            for lcm in [ZSTD_lcm_auto, ZSTD_lcm_huffman] {
                diff_bytes(&format!("suspect/{name}/lvl{lvl}/lcm{lcm}"), |l| {
                    let cctx = Ctx::cctx(l);
                    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                    let mut sets = Vec::new();
                    for (p, v) in [
                        (ZSTD_c_compressionLevel, lvl),
                        (ZSTD_c_literalCompressionMode, lcm),
                    ] {
                        sets.push(res(l, unsafe { set(cctx.ptr, p, v) }));
                    }
                    let cap = compress_bound(l, src.len()) + 64;
                    let mut dst = vec![0xCDu8; cap];
                    let n = unsafe {
                        l.sym::<FnCompress2>("ZSTD_compress2")(
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
                    // round-trip, so a mis-built Huffman table is caught too
                    let mut back = vec![0xEEu8; src.len() + 64];
                    let dn = unsafe {
                        l.sym::<FnDecompress>("ZSTD_decompress")(
                            back.as_mut_ptr() as *mut c_void,
                            back.len(),
                            dst.as_ptr() as *const c_void,
                            dst.len(),
                        )
                    };
                    let dr = res(l, dn);
                    if let R::Ok(k) = dr {
                        back.truncate(k);
                    }
                    assert_eq!(&back[..], &src[..], "[{}] round-trip mismatch", l.tag);
                    (sets, r, dr, Blob(dst))
                });
            }
        }
    }
}

// ===========================================================================
// CONFIGS 69 — mid-frame ZSTD_CCtx_setParameter
// ===========================================================================

/// `ZSTD_CCtx_setParameter` (`compress/zstd_compress.c:705..716`):
/// `ZSTD_isUpdateAuthorized` (`:658`) splits the parameter set in two. The
/// authorized ones only set `cctx->cParamsChanged = 1`, which is consumed
/// *exclusively* by the `ZSTDMT` branch of `ZSTD_compressStream2`; since
/// `ZSTD_MULTITHREAD` is off in this build, an authorized mid-frame change must
/// have **no effect at all** on the output. The unauthorized ones must report
/// `stage_wrong` (60).
///
/// CONFIGS 69.
#[test]
fn mid_frame_parameter_updates() {
    covers(&["CFG:69"]);
    let src = corpus(Corpus::Text, 400_000, 0x69);

    // Baseline: the frame with no mid-stream mutation at all.
    let baseline = diff_bytes("midframe/baseline", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut dst = vec![0u8; compress_bound(l, src.len()) + 64];
        let mut o = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: dst.len(),
            pos: 0,
        };
        let mut i = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: src.len(),
            pos: 0,
        };
        let a = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
        let mut b = R::Ok(1);
        while !matches!(b, R::Ok(0)) {
            b = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_end) });
            if matches!(b, R::Err(..)) {
                break;
            }
        }
        dst.truncate(o.pos);
        (a, b, Blob(dst))
    });

    const AUTHORIZED: &[(&str, c_int, c_int)] = &[
        ("compressionLevel", ZSTD_c_compressionLevel, 9),
        ("hashLog", ZSTD_c_hashLog, 16),
        ("chainLog", ZSTD_c_chainLog, 15),
        ("searchLog", ZSTD_c_searchLog, 3),
        ("minMatch", ZSTD_c_minMatch, 5),
        ("targetLength", ZSTD_c_targetLength, 64),
        ("strategy", ZSTD_c_strategy, ZSTD_btlazy2),
        ("blockSplitterLevel", ZSTD_c_blockSplitterLevel, 3),
    ];
    const REJECTED: &[(&str, c_int, c_int)] = &[
        ("windowLog", ZSTD_c_windowLog, 20),
        ("contentSizeFlag", ZSTD_c_contentSizeFlag, 0),
        ("checksumFlag", ZSTD_c_checksumFlag, 1),
        ("dictIDFlag", ZSTD_c_dictIDFlag, 0),
        ("nbWorkers", ZSTD_c_nbWorkers, 0),
        ("enableLDM", ZSTD_c_enableLongDistanceMatching, 1),
        ("targetCBlockSize", ZSTD_c_targetCBlockSize, 2048),
        ("prefetchCDictTables", ZSTD_c_prefetchCDictTables, 1),
        ("format", ZSTD_c_format, ZSTD_f_zstd1_magicless),
        ("stableInBuffer", ZSTD_c_stableInBuffer, 1),
        ("useRowMatchFinder", ZSTD_c_useRowMatchFinder, ZSTD_ps_enable),
    ];

    for (authorized, table) in [(true, AUTHORIZED), (false, REJECTED)] {
        for &(name, param, value) in table {
            let got = diff_bytes(&format!("midframe/{name}"), |l| {
                let cctx = Ctx::cctx(l);
                let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let mut dst = vec![0u8; compress_bound(l, src.len()) + 64];
                let mut o = ZSTD_outBuffer {
                    dst: dst.as_mut_ptr() as *mut c_void,
                    size: dst.len(),
                    pos: 0,
                };
                let mut i = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: src.len(),
                    pos: 0,
                };
                let a = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_continue) });
                // now mid-frame: streamStage == zcss_load
                let sr = res(l, unsafe { set(cctx.ptr, param, value) });
                // and read it back
                let mut readback: c_int = -12345;
                let gr = res(l, unsafe {
                    l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter")(
                        cctx.ptr,
                        param,
                        &mut readback,
                    )
                });
                let mut b = R::Ok(1);
                while !matches!(b, R::Ok(0)) {
                    b = res(l, unsafe { f(cctx.ptr, &mut o, &mut i, ZSTD_e_end) });
                    if matches!(b, R::Err(..)) {
                        break;
                    }
                }
                dst.truncate(o.pos);
                (a, sr, gr, readback, b, Blob(dst))
            });
            if authorized {
                assert!(
                    matches!(got.1, R::Ok(_)),
                    "{name} must be authorized mid-frame, got {:?}",
                    got.1
                );
            } else {
                assert!(
                    matches!(got.1, R::Err(60, _)),
                    "{name} must report stage_wrong(60) mid-frame, got {:?}",
                    got.1
                );
            }
            // ZSTD_MULTITHREAD is off, so `cParamsChanged` is never consumed:
            // the frame bytes must be identical to the untouched baseline.
            assert_eq!(
                got.5, baseline.2,
                "{name}: a mid-frame parameter change altered the output"
            );
        }
    }
}

// ===========================================================================
// CONFIGS 96 — legacy frame magics
// ===========================================================================

/// `ZSTD_LEGACY_SUPPORT` is 5 in this build, so `ZSTD_isLegacy` accepts
/// v0.1..v0.5 only. `ZSTD_getFrameContentSize`, `ZSTD_findFrameSizeInfo`,
/// `ZSTD_decompressMultiFrame` and the `zdss_loadHeader` error path each have
/// their own `#if ZSTD_LEGACY_SUPPORT >= 1` branch, and the v0.6 / v0.7 magics
/// must fall through to `prefix_unknown`.
///
/// CONFIGS 96.
#[test]
fn legacy_magic_frame_fixtures() {
    covers(&["CFG:96"]);
    const MAGICS: &[(&str, u32)] = &[
        ("v01", 0x1EB5_2FFD),
        ("v02", 0xFD2F_B522),
        ("v03", 0xFD2F_B523),
        ("v04", 0xFD2F_B524),
        ("v05", 0xFD2F_B525),
        ("v06", 0xFD2F_B526),
        ("v07", 0xFD2F_B527),
        ("v08-notlegacy", 0xFD2F_B528),
    ];
    for (name, m) in MAGICS {
        for tail in [
            vec![],
            vec![0u8; 1],
            vec![0u8; 4],
            vec![0u8; 28],
            vec![0xFFu8; 28],
            (0..28u8).collect::<Vec<u8>>(),
        ] {
            let mut b = m.to_le_bytes().to_vec();
            b.extend_from_slice(&tail);
            let label = format!("legacy/{name}/tail{}", tail.len());
            // one-shot: ZSTD_decompress / getFrameContentSize / findFrameCompressedSize
            diff_bytes(&format!("{label}/oneshot"), |l| {
                let mut out = vec![0xEEu8; 4096];
                let n = unsafe {
                    l.sym::<FnDecompress>("ZSTD_decompress")(
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        b.as_ptr() as *const c_void,
                        b.len(),
                    )
                };
                let fcs = unsafe {
                    l.sym::<FnGetFrameContentSize>("ZSTD_getFrameContentSize")(
                        b.as_ptr() as *const c_void,
                        b.len(),
                    )
                };
                let fcz = res(l, unsafe {
                    l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize")(
                        b.as_ptr() as *const c_void,
                        b.len(),
                    )
                });
                (res(l, n), fcs, fcz, Blob(out))
            });
            // streaming, one byte at a time
            diff_bytes(&format!("{label}/stream1"), |l| {
                let dctx = Ctx::dstream(l);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xEEu8; 4096];
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut steps = Vec::new();
                let mut fed = 0usize;
                while fed < b.len() {
                    let mut i = ZSTD_inBuffer {
                        src: unsafe { b.as_ptr().add(fed) } as *const c_void,
                        size: 1,
                        pos: 0,
                    };
                    let r = res(l, unsafe { f(dctx.ptr, &mut o, &mut i) });
                    let stop = matches!(r, R::Err(..));
                    steps.push(r);
                    fed += i.pos;
                    if stop || i.pos == 0 {
                        break;
                    }
                }
                (steps, o.pos, Blob(out))
            });
        }
    }
}

// ===========================================================================
// CONFIGS 91 — the literals-buffer placement at the 64 KB boundary
// ===========================================================================

const ZSTD_LITBUFFEREXTRASIZE: usize = 1 << 16;
const WILDCOPY_OVERLENGTH: usize = 32;

/// The regenerated (decompressed) literals size of a frame's first block, parsed
/// from the Literals_Section_Header, plus the block type. Returns `None` when the
/// first block is not `bt_compressed` (raw/RLE blocks have no literals section).
fn first_block_lit_size(l: &Lib, frame: &[u8]) -> Option<(u32, u32, u64)> {
    let mut h = ZSTD_FrameHeader::default();
    let r = unsafe {
        l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced")(
            &mut h,
            frame.as_ptr() as *const c_void,
            frame.len(),
            ZSTD_f_zstd1,
        )
    };
    if is_error(l, r) || r != 0 {
        return None;
    }
    let mut p = h.headerSize as usize;
    if p + 3 > frame.len() {
        return None;
    }
    let bh = u32::from_le_bytes([frame[p], frame[p + 1], frame[p + 2], 0]);
    let btype = (bh >> 1) & 3;
    let bsize = bh >> 3;
    p += 3;
    if btype != 2 || p + 5 > frame.len() {
        return Some((btype, bsize, 0));
    }
    let b0 = frame[p];
    let lt = b0 & 3;
    let sf = (b0 >> 2) & 3;
    let v40 = u64::from_le_bytes([
        frame[p],
        frame[p + 1],
        frame[p + 2],
        frame[p + 3],
        frame[p + 4],
        0,
        0,
        0,
    ]);
    let regen = if lt <= 1 {
        // Raw_Literals_Block / RLE_Literals_Block
        match sf {
            0 | 2 => ((b0 >> 3) as u64) & 0x1F,
            1 => (v40 >> 4) & 0xFFF,
            _ => (v40 >> 4) & 0xFFFFF,
        }
    } else {
        // Compressed_Literals_Block / Treeless
        match sf {
            0 | 1 => (v40 >> 4) & 0x3FF,
            2 => (v40 >> 4) & 0x3FFF,
            _ => (v40 >> 4) & 0x3FFFF,
        }
    };
    Some((btype, bsize, regen))
}

/// `ZSTD_allocateLiteralsBuffer` (`decompress/zstd_decompress_block.c:80..122`)
/// picks between the three literal-buffer placements:
///
/// * `ZSTD_in_dst` when `streaming == not_streaming && dstCapacity >
///   blockSizeMax + WILDCOPY_OVERLENGTH + litSize + WILDCOPY_OVERLENGTH`;
/// * `ZSTD_not_in_dst` when `litSize <= ZSTD_LITBUFFEREXTRASIZE (65536)`;
/// * `ZSTD_split` otherwise.
///
/// `dctx->litBufferLocation` then selects `ZSTD_decompressSequencesSplitLitBuffer`
/// vs `ZSTD_decompressSequences` at the bottom of
/// `ZSTD_decompressBlock_internal` — two independently written sequence loops.
///
/// Frames are built with a single ~100000-byte block whose literals size is swept
/// across the 65536 boundary (random head + one long match tail), and each is
/// decoded (a) one-shot with a dstCapacity above the `in_dst` threshold, (b)
/// one-shot with dstCapacity exactly equal to the frame content size, and (c)
/// streaming (which is never `in_dst`).
///
/// CONFIGS 91.
#[test]
fn literals_buffer_placement_boundary() {
    covers(&["CFG:91"]);
    const TOTAL: usize = 100_000;
    let lit_targets: &[usize] = &[
        1000, 40_000, 65_000, 65_500, 65_530, 65_536, 65_540, 66_000, 70_000, 90_000, 99_000,
    ];
    let mut below = 0usize;
    let mut at_or_above = 0usize;

    for &lt in lit_targets {
        // random head of `lt` bytes, then the tail repeats an earlier region so
        // the rest of the block is a single long match (few sequences, litSize
        // ~= lt).
        let mut src = corpus(Corpus::Random, TOTAL, 0x910 ^ lt as u64);
        let tail = TOTAL - lt;
        if tail > 0 {
            let region: Vec<u8> = src[..tail.min(lt).max(1)].to_vec();
            let mut off = lt;
            while off < TOTAL {
                let n = region.len().min(TOTAL - off);
                src[off..off + n].copy_from_slice(&region[..n]);
                off += n;
            }
        }
        let frame = c_compress_with(
            &src,
            &[
                (ZSTD_c_compressionLevel, 3),
                (ZSTD_c_windowLog, 18),
                // one block, no splitting, so the parsed literals size is the
                // one ZSTD_allocateLiteralsBuffer sees
                (ZSTD_c_targetCBlockSize, 0),
                (ZSTD_c_blockSplitterLevel, 1),
            ],
            None,
        );
        let parsed = first_block_lit_size(&pair().c, &frame);
        if let Some((btype, _, regen)) = parsed {
            if btype == 2 {
                if regen as usize <= ZSTD_LITBUFFEREXTRASIZE {
                    below += 1;
                } else {
                    at_or_above += 1;
                }
            }
        }
        // (a) in_dst: dstCapacity above blockSizeMax + 32 + litSize + 32
        let big_cap = (1usize << 17) + 2 * WILDCOPY_OVERLENGTH + TOTAL + 64;
        for (which, cap) in [("in_dst", big_cap), ("exact", TOTAL)] {
            diff_bytes(&format!("litbuf/lt{lt}/{which}"), |l| {
                let mut out = vec![0xEEu8; cap];
                let n = unsafe {
                    l.sym::<FnDecompress>("ZSTD_decompress")(
                        out.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                    )
                };
                let r = res(l, n);
                if let R::Ok(k) = r {
                    out.truncate(k);
                }
                assert_eq!(&out[..], &src[..], "[{}] {which} lt={lt}", l.tag);
                (r, Blob(out))
            });
            // via ZSTD_decompressDCtx too
            diff_bytes(&format!("litbuf/lt{lt}/{which}/dctx"), |l| {
                let dctx = Ctx::dctx(l);
                let mut out = vec![0xEEu8; cap];
                let n = unsafe {
                    l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                        dctx.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                    )
                };
                let r = res(l, n);
                if let R::Ok(k) = r {
                    out.truncate(k);
                }
                (r, Blob(out))
            });
        }
        // (c) streaming: never in_dst, whatever the output capacity
        for chunk in [1usize, 7, 1024, 1 << 16] {
            diff_bytes(&format!("litbuf/lt{lt}/stream{chunk}"), |l| {
                let dctx = Ctx::dstream(l);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xEEu8; TOTAL + 4096];
                let mut o = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut fed = 0usize;
                let mut last = R::Ok(0);
                while fed < frame.len() {
                    let n = chunk.min(frame.len() - fed);
                    let mut i = ZSTD_inBuffer {
                        src: unsafe { frame.as_ptr().add(fed) } as *const c_void,
                        size: n,
                        pos: 0,
                    };
                    last = res(l, unsafe { f(dctx.ptr, &mut o, &mut i) });
                    if matches!(last, R::Err(..)) {
                        break;
                    }
                    fed += i.pos;
                    if i.pos == 0 {
                        break;
                    }
                }
                out.truncate(o.pos);
                assert_eq!(&out[..], &src[..], "[{}] stream lt={lt}", l.tag);
                (last, Blob(out))
            });
        }
    }
    assert!(
        below > 0 && at_or_above > 0,
        "the sweep must straddle ZSTD_LITBUFFEREXTRASIZE (below={below}, above={at_or_above})"
    );
}

// ===========================================================================
// CONFIGS 93 — in-place decompression
// ===========================================================================

/// `ZSTD_decompressFrame` (`decompress/zstd_decompress.c:997..1012`):
/// `if (ip >= op && ip < oBlockEnd) oBlockEnd = op + (ip - op)` clamps the
/// dstCapacity handed to `ZSTD_decompressBlock_internal` (and hence the literals
/// buffer placement), while `bt_raw` deliberately uses `oend` plus a `memmove`
/// so it is safe to overlap.
///
/// The compressed frame is placed at increasing offsets inside a single buffer
/// whose start is the destination, so `src == dst + off` for
/// `off in {0, 1, 16, ..., dSize + margin - cSize}` — the last being the
/// canonical placement `ZSTD_decompressionMargin` documents.
///
/// CONFIGS 93.
#[test]
fn in_place_decompression_margins() {
    covers(&["CFG:93"]);
    for &n in &[1000usize, 300_000] {
        for (kind, name) in [
            (Corpus::Text, "compressible"),
            (Corpus::Random, "incompressible"),
        ] {
            let src = corpus(kind, n, 0x93);
            for lvl in [1i32, 9] {
                for checksum in [0i32, 1] {
                    let frame = c_compress_with(
                        &src,
                        &[
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_checksumFlag, checksum),
                        ],
                        None,
                    );
                    let margin = {
                        let l = &pair().c;
                        let m = unsafe {
                            l.sym::<FnDecompressionMargin>("ZSTD_decompressionMargin")(
                                frame.as_ptr() as *const c_void,
                                frame.len(),
                            )
                        };
                        assert!(!is_error(l, m), "decompressionMargin: {}", err_name(l, m));
                        m
                    };
                    let canonical = n + margin - frame.len();
                    let mut offsets = vec![0usize, 1, 16, 64];
                    if canonical > 64 {
                        offsets.push(canonical / 2);
                        offsets.push(canonical - 1);
                        offsets.push(canonical);
                    }
                    offsets.push(n + margin - frame.len() + 1);
                    offsets.sort_unstable();
                    offsets.dedup();
                    for off in offsets {
                        let label =
                            format!("inplace/{name}/n{n}/lvl{lvl}/ck{checksum}/off{off}");
                        let got = diff_bytes(&label, |l| {
                            let mut buf = vec![0xEEu8; n + margin + off + 64];
                            buf[off..off + frame.len()].copy_from_slice(&frame);
                            let dst = buf.as_mut_ptr();
                            let src_ptr = unsafe { buf.as_ptr().add(off) };
                            let k = unsafe {
                                l.sym::<FnDecompress>("ZSTD_decompress")(
                                    dst as *mut c_void,
                                    n,
                                    src_ptr as *const c_void,
                                    frame.len(),
                                )
                            };
                            let r = res(l, k);
                            let out = match r {
                                R::Ok(m) => buf[..m].to_vec(),
                                R::Err(..) => Vec::new(),
                            };
                            (r, Blob(out))
                        });
                        if off >= canonical {
                            assert!(
                                matches!(got.0, R::Ok(m) if m == n),
                                "{label}: the documented margin must decode, got {:?}",
                                got.0
                            );
                            assert_eq!(got.1 .0, src, "{label}: in-place output mismatch");
                        }
                    }
                    // and via ZSTD_decompressDCtx at the canonical offset
                    diff_bytes(
                        &format!("inplace/{name}/n{n}/lvl{lvl}/ck{checksum}/dctx"),
                        |l| {
                            let dctx = Ctx::dctx(l);
                            let mut buf = vec![0xEEu8; n + margin + 64];
                            buf[canonical..canonical + frame.len()].copy_from_slice(&frame);
                            let dst = buf.as_mut_ptr();
                            let src_ptr = unsafe { buf.as_ptr().add(canonical) };
                            let k = unsafe {
                                l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                                    dctx.ptr,
                                    dst as *mut c_void,
                                    n,
                                    src_ptr as *const c_void,
                                    frame.len(),
                                )
                            };
                            let r = res(l, k);
                            let out = match r {
                                R::Ok(m) => buf[..m].to_vec(),
                                R::Err(..) => Vec::new(),
                            };
                            (r, Blob(out))
                        },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS 89 — the frame-corruption error taxonomy
// ===========================================================================

/// Build the ten documented corruptions of a valid frame. Each returns
/// `(label, bytes)`; offsets are derived from the parsed frame header so the
/// mutations land on the intended field.
fn frame_corruptions(src: &[u8], checksum: bool) -> Vec<(String, Vec<u8>)> {
    let l = &pair().c;
    let frame = c_compress_with(
        src,
        &[
            (ZSTD_c_compressionLevel, 3),
            (ZSTD_c_checksumFlag, if checksum { 1 } else { 0 }),
            (ZSTD_c_contentSizeFlag, 1),
        ],
        None,
    );
    let mut h = ZSTD_FrameHeader::default();
    let hr = unsafe {
        l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced")(
            &mut h,
            frame.as_ptr() as *const c_void,
            frame.len(),
            ZSTD_f_zstd1,
        )
    };
    assert!(!is_error(l, hr) && hr == 0, "fixture header parse failed");
    let hs = h.headerSize as usize;
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    out.push(("pristine".into(), frame.clone()));

    // 1. flip a byte of the magic -> prefix_unknown
    for i in 0..4usize {
        let mut f = frame.clone();
        f[i] ^= 0x01;
        out.push((format!("magic^{i}"), f));
    }
    // 2. set the two reserved bits of the FHD (bits 3-4) -> frameParameter_unsupported
    for bit in [3u32, 4] {
        let mut f = frame.clone();
        f[4] |= 1 << bit;
        out.push((format!("fhd-reserved{bit}"), f));
    }
    // 3. truncate by 1..3 bytes -> srcSize_wrong
    for k in 1..=3usize {
        out.push((format!("trunc{k}"), frame[..frame.len() - k].to_vec()));
    }
    // 4. corrupt the trailing 4 checksum bytes
    if checksum {
        for i in 0..4usize {
            let mut f = frame.clone();
            let n = f.len();
            f[n - 4 + i] ^= 0xFF;
            out.push((format!("checksum^{i}"), f));
        }
    }
    // block header of the first block: 3 bytes at `hs`
    let bh = u32::from_le_bytes([frame[hs], frame[hs + 1], frame[hs + 2], 0]);
    let last = bh & 1;
    let bsize = bh >> 3;
    let put_bh = |f: &mut Vec<u8>, v: u32| {
        f[hs] = (v & 0xFF) as u8;
        f[hs + 1] = ((v >> 8) & 0xFF) as u8;
        f[hs + 2] = ((v >> 16) & 0xFF) as u8;
    };
    // 5. bt_reserved (block type 3) -> corruption_detected "invalid block type"
    {
        let mut f = frame.clone();
        put_bh(&mut f, last | (3 << 1) | (bsize << 3));
        out.push(("bt_reserved".into(), f));
    }
    // 6. cSize larger than the remaining input -> srcSize_wrong
    {
        let mut f = frame.clone();
        put_bh(&mut f, last | (2 << 1) | ((bsize + 100_000) << 3));
        out.push(("cSize-too-big".into(), f));
    }
    // 7. cSize > blockSizeMax -> corruption_detected "Block Size Exceeds Maximum"
    {
        let mut f = frame.clone();
        put_bh(&mut f, last | (2 << 1) | (((1u32 << 18) - 1) << 3));
        f.resize(f.len() + (1 << 18), 0x5A);
        out.push(("cSize-over-blockmax".into(), f));
    }
    // 8. frameContentSize one byte larger than reality -> corruption_detected
    {
        let fhd = frame[4];
        let fcs_code = fhd >> 6;
        let single = (fhd >> 5) & 1;
        let fcs_len: usize = match fcs_code {
            0 => single as usize,
            1 => 2,
            2 => 4,
            _ => 8,
        };
        if fcs_len > 0 {
            let mut f = frame.clone();
            let at = hs - fcs_len;
            let mut v: u64 = 0;
            for i in 0..fcs_len {
                v |= (f[at + i] as u64) << (8 * i);
            }
            v += 1;
            for i in 0..fcs_len {
                f[at + i] = ((v >> (8 * i)) & 0xFF) as u8;
            }
            out.push(("fcs+1".into(), f));
        }
    }
    // 9. corrupt the literals-section header (first byte after the block header)
    if hs + 3 < frame.len() {
        for x in [0x01u8, 0x03, 0xFF, 0x7C] {
            let mut f = frame.clone();
            f[hs + 3] ^= x;
            out.push((format!("litheader^{x:02x}"), f));
        }
    }
    // 10. corrupt the FSE table descriptions: the bytes right after the literals
    //     section. Sweep a window so at least one lands on an FSE header.
    if hs + 3 < frame.len() {
        for d in [8usize, 12, 16, 24, 32] {
            let at = hs + 3 + d;
            if at < frame.len() - 4 {
                let mut f = frame.clone();
                f[at] ^= 0xFF;
                out.push((format!("fsetable^{d}"), f));
            }
        }
    }
    out
}

/// `ZSTD_decompress` / `ZSTD_decompressDCtx` / `ZSTD_decompressStream` over ten
/// distinct corruptions of a valid frame. The error taxonomy must match exactly
/// between the two libraries, and `ZSTD_decompressFrame` (one-shot) and the
/// `ZSTDds_*` state machine in `ZSTD_decompressContinue` must report the same
/// code for the same corruption.
///
/// CONFIGS 89.
#[test]
fn frame_corruption_taxonomy() {
    covers(&["CFG:89"]);
    for &n in &[1000usize, 200_000] {
        for checksum in [false, true] {
            let src = corpus(Corpus::Text, n, 0x89);
            for (label, bad) in frame_corruptions(&src, checksum) {
                let tag = format!("corrupt/n{n}/ck{checksum}/{label}");
                // one-shot, twice: ZSTD_decompress and ZSTD_decompressDCtx
                diff_bytes(&format!("{tag}/oneshot"), |l| {
                    let mut a = vec![0xEEu8; n + 4096];
                    let ra = res(l, unsafe {
                        l.sym::<FnDecompress>("ZSTD_decompress")(
                            a.as_mut_ptr() as *mut c_void,
                            a.len(),
                            bad.as_ptr() as *const c_void,
                            bad.len(),
                        )
                    });
                    let dctx = Ctx::dctx(l);
                    let mut b = vec![0xEEu8; n + 4096];
                    let rb = res(l, unsafe {
                        l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx")(
                            dctx.ptr,
                            b.as_mut_ptr() as *mut c_void,
                            b.len(),
                            bad.as_ptr() as *const c_void,
                            bad.len(),
                        )
                    });
                    let out = match ra {
                        R::Ok(k) => a[..k].to_vec(),
                        R::Err(..) => Vec::new(),
                    };
                    (ra, rb, Blob(out))
                });
                // streaming, in a few chunk sizes
                for chunk in [1usize, 13, 4096] {
                    diff_bytes(&format!("{tag}/stream{chunk}"), |l| {
                        let dctx = Ctx::dstream(l);
                        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                        let mut out = vec![0xEEu8; n + 4096];
                        let mut o = ZSTD_outBuffer {
                            dst: out.as_mut_ptr() as *mut c_void,
                            size: out.len(),
                            pos: 0,
                        };
                        let mut last = R::Ok(0);
                        let mut fed = 0usize;
                        let mut steps = 0usize;
                        while fed < bad.len() {
                            let k = chunk.min(bad.len() - fed);
                            let mut i = ZSTD_inBuffer {
                                src: unsafe { bad.as_ptr().add(fed) } as *const c_void,
                                size: k,
                                pos: 0,
                            };
                            last = res(l, unsafe { f(dctx.ptr, &mut o, &mut i) });
                            steps += 1;
                            if matches!(last, R::Err(..)) {
                                break;
                            }
                            fed += i.pos;
                            if i.pos == 0 {
                                break;
                            }
                        }
                        out.truncate(o.pos);
                        (last, steps, o.pos, Blob(out))
                    });
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS 122/253/142/148/149/143 — dictionary + workspace failure forwarding
// ===========================================================================

/// * `ZSTD_initLocalDict` (`compress/zstd_compress.c:1278`):
///   `ZSTD_createCDict_advanced2(...)` returned NULL -> `memory_allocation` (64).
///   No allocator injection is needed: a *corrupt full dictionary* makes
///   `ZSTD_initCDict_internal` fail, so `createCDict_advanced2` returns NULL.
/// * `ZSTD_CCtx_init_compressStream2` (`:6355`) forwards that error.
/// * `ZSTD_resetCCtx_internal` (`:2168`): the workspace must be resized and
///   `zc->staticSize != 0` -> `memory_allocation` (64); an undersized static
///   CCtx compressing at a higher level.
/// * `ZSTD_resetCCtx_byAttachingCDict` (`:2350`) and
///   `ZSTD_resetCCtx_byCopyingCDict` (`:2420`) forward that same failure when a
///   CDict is referenced (`forceAttachDict = ZSTD_dictForceCopy` selects the
///   copying variant).
///
/// ERRORS 122, 142, 148, 149, 253.
#[test]
fn local_dict_and_static_workspace_failures() {
    covers(&[
        "ERR:compress/zstd_compress.c:1278",
        "ERR:compress/zstd_compress.c:6355",
        "ERR:compress/zstd_compress.c:2168",
        "ERR:compress/zstd_compress.c:2350",
        "ERR:compress/zstd_compress.c:2420",
    ]);
    let src = corpus(Corpus::Text, 60_000, 0x122);
    // A dictionary carrying ZSTD_MAGIC_DICTIONARY whose entropy tables are
    // garbage: ZSTD_loadCEntropy rejects it, so ZSTD_initCDict_internal fails.
    let mut corrupt_full = le32(ZSTD_MAGIC_DICTIONARY).to_vec();
    corrupt_full.extend_from_slice(&le32(9));
    corrupt_full.extend_from_slice(&[0xFFu8; 200]);

    // ---- :1278 / :6355 ---------------------------------------------------
    for &dlm in &[ZSTD_dlm_byRef, ZSTD_dlm_byCopy] {
        let (r_load, r_c2, r_cs2, _) = diff_bytes(
            &format!("localDict/corrupt-fullDict/dlm{dlm}"),
            |l| {
                let cctx = Ctx::cctx(l);
                let a = res(l, unsafe {
                    l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                        cctx.ptr,
                        corrupt_full.as_ptr() as *const c_void,
                        corrupt_full.len(),
                        dlm,
                        ZSTD_dct_fullDict,
                    )
                });
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                let b = res(l, unsafe {
                    l.sym::<FnCompress2>("ZSTD_compress2")(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                });
                // and once more through ZSTD_compressStream2 directly
                let cctx2 = Ctx::cctx(l);
                let _ = res(l, unsafe {
                    l.sym::<FnLoadDictAdv>("ZSTD_CCtx_loadDictionary_advanced")(
                        cctx2.ptr,
                        corrupt_full.as_ptr() as *const c_void,
                        corrupt_full.len(),
                        dlm,
                        ZSTD_dct_fullDict,
                    )
                });
                let mut o = ZSTD_outBuffer {
                    dst: dst.as_mut_ptr() as *mut c_void,
                    size: cap,
                    pos: 0,
                };
                let mut i = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: src.len(),
                    pos: 0,
                };
                let c = res(l, unsafe {
                    l.sym::<FnCompressStream2>("ZSTD_compressStream2")(
                        cctx2.ptr,
                        &mut o,
                        &mut i,
                        ZSTD_e_end,
                    )
                });
                (a, b, c, Blob(vec![]))
            },
        );
        assert!(matches!(r_load, R::Ok(0)), "loadDictionary: {r_load:?}");
        assert!(
            matches!(r_c2, R::Err(64, _)),
            "compress2 with a corrupt local full dict: expected memory_allocation(64), got {r_c2:?}"
        );
        assert!(
            matches!(r_cs2, R::Err(64, _)),
            "compressStream2: expected memory_allocation(64), got {r_cs2:?}"
        );
    }

    // ---- :2168 / :2350 / :2420 -------------------------------------------
    let dict = corpus(Corpus::Text, 8192, 0x142);
    let mut cdict_fail_by_attach = [0usize; 4];
    for (attach, name) in [
        (0i32, "auto"),
        (1, "forceAttach"),
        (2, "forceCopy"),
        (3, "forceLoad"),
    ] {
        for lvl in [1i32, 9, 19, 22] {
            let label = format!("staticWksp/{name}/lvl{lvl}");
            let (r_plain, r_cdict) = diff(&label, |l| {
                // A static CCtx sized for level 1 only.
                let est = unsafe { l.sym::<FnEstFromInt2>("ZSTD_estimateCCtxSize")(1) };
                let mut ws = wksp(est);
                let p = unsafe {
                    l.sym::<FnInitStatic>("ZSTD_initStaticCCtx")(
                        wksp_ptr(&mut ws),
                        wksp_bytes(&ws),
                    )
                };
                assert!(!p.is_null(), "[{}] initStaticCCtx", l.tag);
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0xCDu8; cap];
                // (a) no dictionary: ZSTD_resetCCtx_internal directly
                let a = res(l, unsafe {
                    l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                        p,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        lvl,
                    )
                });
                // (b) with a CDict, so the attach / copy wrappers forward it
                let mut ws2 = wksp(est);
                let p2 = unsafe {
                    l.sym::<FnInitStatic>("ZSTD_initStaticCCtx")(
                        wksp_ptr(&mut ws2),
                        wksp_bytes(&ws2),
                    )
                };
                assert!(!p2.is_null());
                let cd = unsafe {
                    l.sym::<FnCreateCDict>("ZSTD_createCDict")(
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    )
                };
                assert!(!cd.is_null());
                let cdict = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                let _ = unsafe { set(p2, ZSTD_c_forceAttachDict, attach) };
                let _ = unsafe { set(p2, ZSTD_c_compressionLevel, lvl) };
                let _ = unsafe {
                    l.sym::<FnRefCDict>("ZSTD_CCtx_refCDict")(p2, cdict.ptr)
                };
                let b = res(l, unsafe {
                    l.sym::<FnCompress2>("ZSTD_compress2")(
                        p2,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                });
                let _ = (&mut ws, &mut ws2);
                (a, b)
            });
            if lvl > 1 {
                assert!(
                    matches!(r_plain, R::Err(64, _)),
                    "{label}: a static CCtx sized for level 1 must refuse level {lvl} with memory_allocation(64), got {r_plain:?}"
                );
            }
            if matches!(r_cdict, R::Err(64, _)) {
                cdict_fail_by_attach[attach as usize] += 1;
            }
        }
    }
    // ZSTD_dictForceAttach (1) routes through ZSTD_resetCCtx_byAttachingCDict
    // (:2350) and ZSTD_dictForceCopy (2) through
    // ZSTD_resetCCtx_byCopyingCDict (:2420); each must have forwarded the
    // undersized-static-workspace failure at least once.
    assert!(
        cdict_fail_by_attach[1] > 0,
        "the attach path never forwarded a workspace failure"
    );
    assert!(
        cdict_fail_by_attach[2] > 0,
        "the copy path never forwarded a workspace failure"
    );
}

// ---------------------------------------------------------------------------
// A "fail at the Nth allocation" custom allocator.
//
// This uses process-wide `static`s (an `extern "C"` callback has nowhere else to
// keep its counter), so every test that reads them must hold SERIAL_ALLOC for its
// whole body — otherwise tests on different `--test-threads` steal each other's
// counts and produce a phantom divergence with identical output.
// ---------------------------------------------------------------------------

static SERIAL_ALLOC: Mutex<()> = Mutex::new(());
fn serial_alloc() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL_ALLOC.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}
static ALLOC_SEEN: AtomicUsize = AtomicUsize::new(0);
static ALLOC_FAIL_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
static ALLOC_ARMED: AtomicBool = AtomicBool::new(false);

extern "C" fn nth_fail_alloc(_opaque: *mut c_void, size: SizeT) -> *mut c_void {
    if ALLOC_ARMED.load(SeqCst) {
        let n = ALLOC_SEEN.fetch_add(1, SeqCst) + 1;
        if n == ALLOC_FAIL_AT.load(SeqCst) {
            return std::ptr::null_mut();
        }
    }
    plain_alloc(std::ptr::null_mut(), size)
}

fn nth_fail_mem() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(nth_fail_alloc),
        customFree: Some(plain_free),
        opaque: std::ptr::null_mut(),
    }
}

/// `ZSTD_resetCCtx_internal` (`compress/zstd_compress.c:2173`):
/// `ZSTD_cwksp_create(ws, neededSpace, customMem)` fails -> `memory_allocation`
/// (64). `ZSTD_cwksp_create` goes through `ZSTD_customMalloc` and NULL-checks the
/// result, so refusing that specific allocation is safe (unlike
/// `ZSTD_customCalloc`, `common/allocations.h:39`, which `memset`s the returned
/// pointer with no NULL check and therefore SEGFAULTs the reference C).
///
/// The context itself is allocated by call #1 (`ZSTD_customMalloc(sizeof(
/// ZSTD_CCtx))` in `ZSTD_createCCtx_advanced`), so failing call #2 is exactly the
/// `ZSTD_cwksp_create` for the compression workspace.
///
/// ERRORS 143.
#[test]
fn cwksp_create_allocation_failure() {
    let _serial = serial_alloc();
    covers(&["ERR:compress/zstd_compress.c:2173"]);
    let src = corpus(Corpus::Text, 40_000, 0x143);

    // First, with no failure armed: count how many allocations a whole
    // compression performs, and confirm the two libraries agree.
    let total = diff("cwkspCreate/transcript", |l| {
        ALLOC_ARMED.store(false, SeqCst);
        ALLOC_SEEN.store(0, SeqCst);
        ALLOC_FAIL_AT.store(usize::MAX, SeqCst);
        ALLOC_ARMED.store(true, SeqCst);
        let p = unsafe {
            l.sym::<FnCreateAdvancedMem>("ZSTD_createCCtx_advanced")(nth_fail_mem())
        };
        assert!(!p.is_null());
        let cctx = Ctx::from_raw(l, p, "ZSTD_freeCCtx");
        let cap = compress_bound(l, src.len()) + 64;
        let mut dst = vec![0xCDu8; cap];
        let r = res(l, unsafe {
            l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                3,
            )
        });
        drop(cctx);
        let n = ALLOC_SEEN.load(SeqCst);
        ALLOC_ARMED.store(false, SeqCst);
        (r, n)
    });
    assert!(total.1 >= 2, "expected >= 2 allocations, saw {}", total.1);

    // Now fail exactly the 2nd allocation: the cwksp for the CCtx workspace.
    let got = diff("cwkspCreate/fail-at-2", |l| {
        ALLOC_ARMED.store(false, SeqCst);
        ALLOC_SEEN.store(0, SeqCst);
        ALLOC_FAIL_AT.store(2, SeqCst);
        ALLOC_ARMED.store(true, SeqCst);
        let p = unsafe {
            l.sym::<FnCreateAdvancedMem>("ZSTD_createCCtx_advanced")(nth_fail_mem())
        };
        let created = !p.is_null();
        let mut r = R::Ok(usize::MAX);
        if created {
            let cctx = Ctx::from_raw(l, p, "ZSTD_freeCCtx");
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            r = res(l, unsafe {
                l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    3,
                )
            });
        }
        ALLOC_ARMED.store(false, SeqCst);
        (created, r)
    });
    assert!(got.0, "the CCtx itself (allocation #1) must still succeed");
    assert!(
        matches!(got.1, R::Err(64, _)),
        "failing ZSTD_cwksp_create must report memory_allocation(64), got {:?}",
        got.1
    );
    ALLOC_ARMED.store(false, SeqCst);
}

// ===========================================================================
// ERRORS 999/1002/1003/1004 — legacy v0.5 FSE header reader / DTable builder
// ===========================================================================

const FSEv05_MAX_TABLELOG: u32 = 12;

/// * `FSEv05_buildDTable` (`legacy/zstd_v05.c:1197`): `position != 0` after
///   distributing symbols, i.e. `normalizedCounter` does not sum to
///   `1 << tableLog` -> `ERROR(GENERIC)` (1).
/// * `FSEv05_readNCount` (`:1274`): more symbols in the header than `*maxSVPtr`
///   allows -> `maxSymbolValue_tooSmall` (48); (`:1315`) `remaining != 1` after
///   the read loop -> `GENERIC` (1); (`:1319`) `(ip - istart) > hbSize` ->
///   `srcSize_wrong` (72).
///
/// The v0.5 normalized-count format is byte-compatible with the current one, so
/// the *valid* headers here are produced by `FSE_writeNCount`. `:1319` is reached
/// by declaring a smaller `hbSize` than the header needs; the reader walks past
/// `hbSize` before noticing, so the header always lives inside a much larger
/// allocation.
///
/// ERRORS 999, 1002, 1003, 1004.
#[test]
fn legacy_v05_fse_header_and_dtable() {
    covers(&[
        "ERR:legacy/zstd_v05.c:1197",
        "ERR:legacy/zstd_v05.c:1274",
        "ERR:legacy/zstd_v05.c:1315",
        "ERR:legacy/zstd_v05.c:1319",
    ]);

    // ---- :1197  FSEv05_buildDTable with counts that do not sum ------------
    let mut saw_build_generic = false;
    for (label, counts, msv, tl) in [
        ("sum2-log10", vec![1i16, 1], 1u32, 10u32),
        ("sum2-log5", vec![1i16, 1], 1, 5),
        ("sum31-log5", vec![16i16, 15], 1, 5),
        ("sum33-log5", vec![16i16, 17], 1, 5),
        ("sum32-log5-ok", vec![16i16, 16], 1, 5),
        // NOT an all-zero normalizedCounter: it sums to 0, which trivially
        // leaves `position == 0`, so the `:1197` guard does not fire and the
        // "Build Decoding table" loop then evaluates
        // `BITv05_highbit32(symbolNext[0]++)` with the argument 0. That expands
        // to `__builtin_clz(0) ^ 31`, and `__builtin_clz(0)` is UNDEFINED
        // BEHAVIOUR: the reference C returns nbBits 0x05 for the first cell
        // while the translation returns 0xc6, purely because the two compilers
        // realise the undefined `bsr` differently. Verified by running it once;
        // the precondition "normalizedCounter sums to 1 << tableLog" is the
        // caller's, so this input is out of contract and is not exercised.
        ("with-lowprob", vec![-1i16, 31, 1, 1], 3, 5),
    ] {
        let got = diff_bytes(&format!("FSEv05_buildDTable/{label}"), |l| {
            let mut dt = vec![0u32; 1 + (1usize << FSEv05_MAX_TABLELOG) + 8];
            let n = unsafe {
                l.sym::<FnFsev05BuildDTable>("FSEv05_buildDTable")(
                    dt.as_mut_ptr() as *mut c_void,
                    counts.as_ptr(),
                    msv,
                    tl,
                )
            };
            let bytes: Vec<u8> = dt.iter().flat_map(|v| v.to_le_bytes()).collect();
            (res_v05(l, n), Blob(bytes))
        });
        if matches!(got.0, R::Err(1, _)) {
            saw_build_generic = true;
        }
    }
    assert!(
        saw_build_generic,
        "no FSEv05_buildDTable input reached the `position != 0` GENERIC return"
    );

    // ---- :1274 / :1315 / :1319  FSEv05_readNCount -------------------------
    // Valid headers, produced by the (byte-compatible) current writer.
    let valid_small = fse_ncount(&[16, 16], 1, 5); // 2 symbols
    let valid_wide = fse_ncount(&[8, 8, 8, 8, 8, 8, 8, 8], 7, 6); // 8 symbols
    let valid_big = {
        let mut c = vec![0i16; 40];
        for (i, v) in c.iter_mut().enumerate() {
            *v = if i < 32 { 16 } else { 0 };
        }
        fse_ncount(&c, 39, 9)
    };

    let mut saw_toosmall = false;
    let mut saw_generic = false;
    let mut saw_srcsize = false;
    let mut cases: Vec<(String, Vec<u8>, u32, usize)> = Vec::new();
    for (nm, h) in [
        ("small", &valid_small),
        ("wide", &valid_wide),
        ("big", &valid_big),
    ] {
        for maxsv in [0u32, 1, 2, 5, 7, 31, 255] {
            for hb in [4usize, 5, h.len().saturating_sub(1).max(4), h.len(), h.len() + 4] {
                cases.push((format!("{nm}/msv{maxsv}/hb{hb}"), h.clone(), maxsv, hb));
            }
        }
        // corrupt the middle so the counts no longer sum to 1 << tableLog
        for i in 1..h.len() {
            let mut c = h.clone();
            c[i] ^= 0xFF;
            cases.push((format!("{nm}/flip{i}"), c, 255, h.len()));
        }
    }
    for (label, hdr, maxsv, hb) in cases {
        // The reader may walk past `hbSize` before checking, so keep the header
        // inside a much larger allocation.
        let mut buf = vec![0u8; hdr.len() + 64];
        buf[..hdr.len()].copy_from_slice(&hdr);
        let got = diff_bytes(&format!("FSEv05_readNCount/{label}"), |l| {
            let mut nc = vec![0i16; 256];
            let mut msv = maxsv;
            let mut tl = 0u32;
            let n = unsafe {
                l.sym::<FnFsev05ReadNCount>("FSEv05_readNCount")(
                    nc.as_mut_ptr(),
                    &mut msv,
                    &mut tl,
                    buf.as_ptr() as *const c_void,
                    hb,
                )
            };
            let bytes: Vec<u8> = nc.iter().flat_map(|v| v.to_le_bytes()).collect();
            (res_v05(l, n), msv, tl, Blob(bytes))
        });
        match got.0 {
            R::Err(48, _) => saw_toosmall = true,
            R::Err(1, _) => saw_generic = true,
            R::Err(72, _) => saw_srcsize = true,
            _ => {}
        }
    }
    assert!(saw_toosmall, ":1274 (maxSymbolValue_tooSmall) not reached");
    assert!(saw_generic, ":1315 (GENERIC) not reached");
    assert!(saw_srcsize, ":1319 (srcSize_wrong) not reached");
}

/// The legacy families have their own `isError`/`getErrorName`, but the codes are
/// the same `0 - code` encoding, so `res()` works. This wrapper exists purely to
/// make the intent explicit at the call sites above.
fn res_v05(l: &Lib, code: SizeT) -> R {
    res(l, code)
}

// ===========================================================================
// ERRORS 1012/1017/1020/1023/1030/1058/1088/1098 — legacy HUF / frame headers
// ===========================================================================

/// A raw ("direct") legacy Huffman weight header. The legacy `HUFv0x_readStats`
/// use the same `src[0] >= 128 -> oSize = src[0] - 127` encoding as the current
/// one, so the same construction works for v0.5, v0.6 and v0.7.
fn legacy_weight_header(weights: &[u8]) -> Vec<u8> {
    huf_raw_weights(weights)
}

/// * `HUFv05_decompress1X2` (`legacy/zstd_v05.c:1936`),
///   `HUFv05_decompress4X2` (`:2046`), `HUFv05_decompress4X4` (`:2428`),
///   `HUFv06_decompress1X2/4X2/1X4/4X4` (`:2066/2175/2443/2551`) and
///   `HUFv07_decompress1X2/4X2/1X4/4X4` (`:1852/1975/2264/2386`): the DTable
///   header consumes `>= cSrcSize` bytes -> `srcSize_wrong` (72). Driven by
///   handing each entry point *only* a valid weight header, with nothing after it.
/// * `HUFv05_readStats` (`:1768/1784/1788/1792/1798/1804`) and `HUFv07_readStats`
///   (`:1275/1292/1296/1300/1307/1313`): the six `corruption_detected` guards,
///   driven with malformed weight headers (weight >= 16 is only expressible
///   through the FSE-compressed weight path, so both encodings are swept).
///
/// ERRORS 1012, 1017, 1020, 1023, 1058, 1088, 1098.
#[test]
fn legacy_huf_header_guards() {
    covers(&[
        "ERR:legacy/zstd_v05.c:1936",
        "ERR:legacy/zstd_v05.c:2046",
        "ERR:legacy/zstd_v05.c:2428",
        "ERR:legacy/zstd_v05.c:1768/1784/1788/1792/1798/1804",
        "ERR:legacy/zstd_v06.c:2066/2175/2443/2551",
        "ERR:legacy/zstd_v07.c:1275/1292/1296/1300/1307/1313",
        "ERR:legacy/zstd_v07.c:1852/1975/2264/2386",
    ]);
    // A valid weight header: four rank-1 weights -> tableLog 3, 5 symbols.
    let valid = legacy_weight_header(&[1, 1, 1, 1]);
    // Malformed variants for the readStats guards.
    let malformed: Vec<(String, Vec<u8>)> = {
        let mut v: Vec<(String, Vec<u8>)> = Vec::new();
        v.push(("all-zero".into(), legacy_weight_header(&[0, 0, 0, 0])));
        v.push(("no-rank1".into(), legacy_weight_header(&[2, 2])));
        v.push(("verif".into(), legacy_weight_header(&[1, 2, 2])));
        v.push(("weight13".into(), legacy_weight_header(&[13, 1])));
        v.push(("weight15".into(), legacy_weight_header(&[15, 1])));
        v.push(("oSize128".into(), legacy_weight_header(&[1u8; 128])));
        // FSE-compressed weight path (src[0] < 128): the only way a decoded
        // weight can be >= 16, and also the `oSize >= hwSize` route.
        for b0 in [1u8, 2, 3, 8, 40, 127] {
            for fill in [0x00u8, 0xF0, 0xFF, 0x55] {
                let mut h = vec![b0];
                h.extend(std::iter::repeat(fill).take(b0 as usize));
                v.push((format!("fse/b{b0}/f{fill:02x}"), h));
            }
        }
        v
    };

    const V05: &[(&str, &str)] = &[
        ("1X2", "HUFv05_decompress1X2"),
        ("4X2", "HUFv05_decompress4X2"),
        ("1X4", "HUFv05_decompress1X4"),
        ("4X4", "HUFv05_decompress4X4"),
    ];
    const V06: &[(&str, &str)] = &[
        ("1X2", "HUFv06_decompress1X2"),
        ("4X2", "HUFv06_decompress4X2"),
        ("1X4", "HUFv06_decompress1X4"),
        ("4X4", "HUFv06_decompress4X4"),
    ];
    const V07: &[(&str, &str)] = &[
        ("1X2", "HUFv07_decompress1X2"),
        ("4X2", "HUFv07_decompress4X2"),
        ("1X4", "HUFv07_decompress1X4"),
        ("4X4", "HUFv07_decompress4X4"),
    ];

    let mut saw_srcsize = [false; 3];
    let mut saw_corrupt = [false; 3];
    for (fam, table) in [(0usize, V05), (1, V06), (2, V07)] {
        for (nm, sym) in table {
            // header only: hSize == cSrcSize -> srcSize_wrong
            for cs in [valid.len(), valid.len() - 1] {
                let got = diff_bytes(&format!("legacyHuf/{fam}/{nm}/headeronly{cs}"), |l| {
                    let mut dst = vec![0xEEu8; 4096];
                    let n = unsafe {
                        l.sym::<FnHufDecompress>(sym)(
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            valid.as_ptr() as *const c_void,
                            cs,
                        )
                    };
                    (res(l, n), Blob(dst))
                });
                if matches!(got.0, R::Err(72, _)) {
                    saw_srcsize[fam] = true;
                }
            }
            // malformed weight headers -> corruption_detected from readStats
            for (label, h) in &malformed {
                let mut buf = vec![0u8; h.len() + 64];
                buf[..h.len()].copy_from_slice(h);
                let got = diff_bytes(&format!("legacyHuf/{fam}/{nm}/{label}"), |l| {
                    let mut dst = vec![0xEEu8; 4096];
                    let n = unsafe {
                        l.sym::<FnHufDecompress>(sym)(
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            buf.as_ptr() as *const c_void,
                            h.len(),
                        )
                    };
                    (res(l, n), Blob(dst))
                });
                if matches!(got.0, R::Err(20, _)) {
                    saw_corrupt[fam] = true;
                }
            }
        }
    }
    for fam in 0..3 {
        assert!(saw_srcsize[fam], "family {fam}: no `hSize >= cSrcSize` rejection");
        assert!(saw_corrupt[fam], "family {fam}: no readStats corruption rejection");
    }

    // HUFv07_readStats directly (the same six guards, one call away).
    let mut saw_direct = false;
    for (label, h) in &malformed {
        let mut buf = vec![0u8; h.len() + 64];
        buf[..h.len()].copy_from_slice(h);
        // `hwSize == 0` is OUT OF CONTRACT for an FSE-coded weight header: the C
        // does `FSEv07_decompress(huffWeight, hwSize-1, ...)`
        // (`legacy/zstd_v07.c:726`), and `hwSize` is a `size_t`, so 0 wraps to
        // `SIZE_MAX` and is handed to the decoder as the DESTINATION CAPACITY —
        // defeating the bound entirely on a zero-length buffer. (The modern
        // `HUF_readStats` has the identical hazard at
        // `common/entropy_common.c:270`, and `tests/t11_lowlevel_errors.rs`
        // excludes it there for the same reason.) A raw-nibble header
        // (`src[0] >= 128`) never reaches that call, so 0 is only excluded for
        // the FSE-coded shapes.
        let fse_coded = h.first().is_some_and(|b| *b < 128);
        for hwsize in [0usize, 4, 256] {
            if hwsize == 0 && fse_coded {
                continue;
            }
            let got = diff_bytes(&format!("HUFv07_readStats/{label}/hw{hwsize}"), |l| {
                let mut hw = vec![0xEEu8; 260];
                let mut rank = vec![0u32; 20];
                let mut nb = 0u32;
                let mut tl = 0u32;
                let n = unsafe {
                    l.sym::<FnReadStats7>("HUFv07_readStats")(
                        hw.as_mut_ptr(),
                        hwsize,
                        rank.as_mut_ptr(),
                        &mut nb,
                        &mut tl,
                        buf.as_ptr() as *const c_void,
                        h.len(),
                    )
                };
                let rb: Vec<u8> = rank.iter().flat_map(|v| v.to_le_bytes()).collect();
                (res(l, n), nb, tl, Blob(hw), Blob(rb))
            });
            if matches!(got.0, R::Err(20, _)) {
                saw_direct = true;
            }
        }
    }
    assert!(saw_direct, "HUFv07_readStats never returned corruption_detected");
}

/// `ZSTDv05_decodeFrameHeader_Part1` (`legacy/zstd_v05.c:2745`): magic mismatch,
/// forwarded at `:3387` (frame path) and `:3550` (streaming path) ->
/// `prefix_unknown` (10).
///
/// ERRORS 1030.
#[test]
fn legacy_v05_frame_magic_mismatch() {
    covers(&["ERR:legacy/zstd_v05.c:2745"]);
    let mut saw = false;
    for (label, buf) in [
        ("zeros8", vec![0u8; 8]),
        ("ones8", vec![0xFFu8; 8]),
        ("v04magic", {
            let mut v = 0xFD2F_B524u32.to_le_bytes().to_vec();
            v.extend_from_slice(&[0u8; 8]);
            v
        }),
        ("v06magic", {
            let mut v = 0xFD2F_B526u32.to_le_bytes().to_vec();
            v.extend_from_slice(&[0u8; 8]);
            v
        }),
        ("v05magic", {
            let mut v = 0xFD2F_B525u32.to_le_bytes().to_vec();
            v.extend_from_slice(&[0u8; 8]);
            v
        }),
    ] {
        let got = diff_bytes(&format!("ZSTDv05_decompress/{label}"), |l| {
            let mut dst = vec![0xEEu8; 4096];
            let n = unsafe {
                l.sym::<FnDecompress>("ZSTDv05_decompress")(
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    buf.as_ptr() as *const c_void,
                    buf.len(),
                )
            };
            // and the streaming path
            let p = unsafe { l.sym::<FnCreateCCtx>("ZBUFFv05_createDCtx")() };
            assert!(!p.is_null());
            let zb = Ctx::from_raw(l, p, "ZBUFFv05_freeDCtx");
            let ir = res(l, unsafe {
                l.sym::<FnZbuffInit>("ZBUFFv05_decompressInit")(zb.ptr)
            });
            let mut out = vec![0xEEu8; 4096];
            let mut osz: SizeT = out.len();
            let mut isz: SizeT = buf.len();
            let cr = res(l, unsafe {
                l.sym::<FnZbuffContinue>("ZBUFFv05_decompressContinue")(
                    zb.ptr,
                    out.as_mut_ptr() as *mut c_void,
                    &mut osz,
                    buf.as_ptr() as *const c_void,
                    &mut isz,
                )
            });
            (res(l, n), ir, cr, osz, isz, Blob(dst))
        });
        if matches!(got.0, R::Err(10, _)) || matches!(got.2, R::Err(10, _)) {
            saw = true;
        }
    }
    assert!(saw, "no prefix_unknown from the v0.5 frame-header magic check");
}

// ===========================================================================
// ERRORS 176/177/179 — ZSTD_buildBlockEntropyStats workspace guards
// ===========================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct SeqDef {
    offBase: c_uint,
    litLength: u16,
    mlBase: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct SeqStore_t {
    sequencesStart: *mut SeqDef,
    sequences: *mut SeqDef,
    litStart: *mut u8,
    lit: *mut u8,
    llCode: *mut u8,
    mlCode: *mut u8,
    ofCode: *mut u8,
    maxNbSeq: SizeT,
    maxNbLit: SizeT,
    longLengthType: c_int,
    longLengthPos: c_uint,
}

type FnBuildBlockEntropyStats = unsafe extern "C" fn(
    *const SeqStore_t,
    *const c_void,
    *mut c_void,
    *const c_void,
    *mut c_void,
    *mut c_void,
    SizeT,
) -> SizeT;

/// `ZSTD_buildBlockEntropyStats` (`compress/zstd_compress.c:3810..3826`) called
/// directly with an undersized workspace:
///
/// * `:3678` `HIST_count_wksp(...)` fails because `wkspSize < HIST_WKSP_SIZE`
///   (4096) -> `workSpace_tooSmall` (66);
/// * `:3705` `HUF_buildCTable_wksp(...)` fails because
///   `nodeWkspSize == wkspSize - 1024 < HUF_CTABLE_WORKSPACE_SIZE` (4864);
/// * `:3817` forwards either of them.
///
/// The seqStore is deliberately built with **zero sequences**
/// (`sequencesStart == sequences`), so `ZSTD_buildBlockEntropyStats_sequences`
/// takes `ZSTD_buildDummySequencesStatistics` and never touches the workspace:
/// with `wkspSize < (MaxSeq+1)*sizeof(U32)` its `entropyWorkspaceSize`
/// subtraction would otherwise underflow, and `ZSTD_buildCTable` only
/// `assert()`s its workspace size (compiled out at `DEBUGLEVEL=0`), which would
/// be an out-of-bounds write rather than an error.
///
/// The strategy is kept below `HUF_OPTIMAL_DEPTH_THRESHOLD` (`ZSTD_btultra`) so
/// `hufFlags == 0` and `HUF_optimalTableLog` returns without touching
/// `nodeWksp` either.
///
/// ERRORS 176, 177, 179.
#[test]
fn build_block_entropy_stats_workspace_guards() {
    covers(&[
        "ERR:compress/zstd_compress.c:3678",
        "ERR:compress/zstd_compress.c:3705",
        "ERR:compress/zstd_compress.c:3817",
    ]);
    // Literals with a clearly compressible-but-not-RLE histogram: `largest` must
    // be > (srcSize >> 7) + 4 and != srcSize, otherwise the function short-cuts
    // to set_basic / set_rle before reaching HIST_count_wksp's consumers.
    let lits = corpus(Corpus::SmallAlphabet, 4096, 0x176);
    let mut saw_hist = false;
    let mut saw_hufctable = false;
    let mut saw_ok = false;
    for wkspsize in [
        0usize, 4, 64, 1024, 2048, 4092, 4096, 4100, 5000, 5880, 5888, 5892, 8192, 16384,
    ] {
        for strategy in [ZSTD_greedy, ZSTD_lazy2] {
            let got = diff_bytes(
                &format!("buildBlockEntropyStats/ws{wkspsize}/s{strategy}"),
                |l| {
                    let params = unsafe { l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams")() };
                    assert!(!params.is_null());
                    let params = Ctx::from_raw(l, params, "ZSTD_freeCCtxParams");
                    let mut p = unsafe { l.sym::<FnGetParams>("ZSTD_getParams")(3, 0, 0) };
                    p.cParams.strategy = strategy;
                    let _ = res(l, unsafe {
                        l.sym::<FnCCtxParamsInitAdv>("ZSTD_CCtxParams_init_advanced")(
                            params.ptr, p,
                        )
                    });
                    // zeroed entropy tables: prevHuf.repeatMode == HUF_repeat_none
                    let mut prev = vec![0u64; BS_SIZE / 8];
                    let mut next = vec![0u64; BS_SIZE / 8];
                    let mut meta = vec![0u64; 1024];
                    let mut ws = wksp(wkspsize.max(8));
                    let mut litbuf = lits.clone();
                    let mut seqs = vec![SeqDef { offBase: 0, litLength: 0, mlBase: 0 }; 8];
                    let mut codes = vec![0u8; 32];
                    let store = SeqStore_t {
                        sequencesStart: seqs.as_mut_ptr(),
                        sequences: seqs.as_mut_ptr(), // nbSeq == 0
                        litStart: litbuf.as_mut_ptr(),
                        lit: unsafe { litbuf.as_mut_ptr().add(litbuf.len()) },
                        llCode: codes.as_mut_ptr(),
                        mlCode: codes.as_mut_ptr(),
                        ofCode: codes.as_mut_ptr(),
                        maxNbSeq: seqs.len(),
                        maxNbLit: litbuf.len(),
                        longLengthType: 0,
                        longLengthPos: 0,
                    };
                    let n = unsafe {
                        l.sym::<FnBuildBlockEntropyStats>("ZSTD_buildBlockEntropyStats")(
                            &store,
                            prev.as_mut_ptr() as *const c_void,
                            next.as_mut_ptr() as *mut c_void,
                            params.ptr,
                            meta.as_mut_ptr() as *mut c_void,
                            wksp_ptr(&mut ws),
                            wkspsize,
                        )
                    };
                    let nb: Vec<u8> = next.iter().flat_map(|v| v.to_le_bytes()).collect();
                    let mb: Vec<u8> = meta.iter().flat_map(|v| v.to_le_bytes()).collect();
                    (res(l, n), Blob(nb), Blob(mb))
                },
            );
            match got.0 {
                R::Err(66, _) => {
                    if wkspsize < 4096 {
                        saw_hist = true;
                    } else {
                        saw_hufctable = true;
                    }
                }
                R::Ok(_) => saw_ok = true,
                _ => {}
            }
        }
    }
    assert!(saw_hist, ":3678 (HIST_count_wksp workSpace_tooSmall) not reached");
    assert!(
        saw_hufctable,
        ":3705 (HUF_buildCTable_wksp workSpace_tooSmall) not reached"
    );
    assert!(saw_ok, "the ample-workspace case must succeed");
}

// ===========================================================================
// ERRORS 151/152/153 — the three ZSTD_buildCTable call sites
// ===========================================================================

/// `ZSTD_buildSequencesStatistics` (`compress/zstd_compress.c:2806`, `:2838`,
/// `:2868`) writes the literal-length, offset-code and match-length FSE tables
/// into `dst..dstEnd` **in that order**, returning the failing `ZSTD_buildCTable`
/// error as `stats.size`.
///
/// `ZSTD_compressBlock` hands its own `dstCapacity` straight down to
/// `ZSTD_entropyCompressSeqStore`, so sweeping `dstCapacity` over *every* value
/// from 0 to the full compressed size walks the first-consumer-that-does-not-fit
/// backwards through the block: the sequences bitstream, then the match-length
/// table (`:2868`), then the offset table (`:2838`), then the literal-length
/// table (`:2806`), then the sequence-count header, then the literals section.
/// The inputs are chosen small and sequence-dense so that `full` is a few
/// thousand bytes and an exhaustive sweep is affordable — with a 128 KB block the
/// three table windows are only ~150 bytes wide out of ~60 KB and any sampled
/// sweep would step straight over them.
///
/// ERRORS 151, 152, 153.
#[test]
fn build_sequences_statistics_dst_too_small() {
    covers(&[
        "ERR:compress/zstd_compress.c:2806",
        "ERR:compress/zstd_compress.c:2838",
        "ERR:compress/zstd_compress.c:2868",
    ]);
    for &kind in &[Corpus::Periodic, Corpus::LongRepeats] {
        for &n in &[8192usize, 32768] {
            let lvl = 9;
            let src = corpus(kind, n, 0x151);
            let full = {
                let l = &pair().c;
                let cctx = Ctx::cctx(l);
                let _ = unsafe {
                    l.sym::<FnCompressBeginLvl>("ZSTD_compressBegin")(cctx.ptr, lvl)
                };
                let cap = compress_bound(l, src.len()) + 64;
                let mut dst = vec![0u8; cap];
                let k = unsafe {
                    l.sym::<FnDecompressDCtx>("ZSTD_compressBlock")(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                };
                assert!(!is_error(l, k), "fixture compressBlock: {}", err_name(l, k));
                k
            };
            assert!(full > 40, "compressed block unexpectedly tiny: {full}");
            let mut n_toosmall = 0usize;
            let mut n_ok = 0usize;
            // The sequences bitstream needs `sizeof(size_t)` bytes of slack past
            // its own length (`BIT_initCStream` refuses `dstCapacity <=
            // sizeof(bitContainer)`), so the smallest capacity that succeeds is a
            // little above `full`; sweep well past it.
            for cap in 0..=(full + 24) {
                let got = diff_bytes(
                    &format!("buildSeqStats/{kind:?}/n{n}/cap{cap}"),
                    |l| {
                        let cctx = Ctx::cctx(l);
                        let b = res(l, unsafe {
                            l.sym::<FnCompressBeginLvl>("ZSTD_compressBegin")(cctx.ptr, lvl)
                        });
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        let k = unsafe {
                            l.sym::<FnDecompressDCtx>("ZSTD_compressBlock")(
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
                        (b, r, Blob(dst))
                    },
                );
                match got.1 {
                    R::Err(70, _) => n_toosmall += 1,
                    R::Ok(_) => n_ok += 1,
                    _ => {}
                }
            }
            assert!(
                n_toosmall > 10,
                "{kind:?}/n{n}: expected many dstSize_tooSmall capacities, saw {n_toosmall}"
            );
            assert!(n_ok > 0, "{kind:?}/n{n}: no capacity succeeded (full={full})");
        }
    }
}

// ===========================================================================
// ERRORS 184/189 — block splitting and super-block emission under a tight dst
// ===========================================================================

/// * `ZSTD_compressSeqStore_singleBlock` (`compress/zstd_compress.c:4124`):
///   `dstCapacity < ZSTD_blockHeaderSize` (3) when a (possibly non-first) split
///   partition is emitted -> `dstSize_tooSmall` (70). Driven with
///   `ZSTD_c_splitAfterSequences = 1` over a 256 KB input and a fine sweep of
///   `dstCapacity`, so the residual capacity left for a later partition drops
///   below 3.
/// The `ZSTD_c_targetCBlockSize` (super-block) variant is exercised too, but only
/// from the achievable frame size upwards: see the in-body comment for the
/// unchecked `ZSTD_memcpy` in `ZSTD_compressSubBlock_literal` that makes tight
/// capacities an out-of-bounds write in the reference C. Row 189
/// (`:4487`, the swallowed `dstSize_tooSmall` from `ZSTD_compressSuperBlock`) is
/// therefore NOT claimed here.
///
/// Every call writes into a buffer with a 64-byte canary past `dstCapacity` and
/// asserts the canary is intact, so an overrun is reported instead of corrupting
/// the heap.
///
/// ERRORS 184.
#[test]
fn split_and_superblock_under_tight_dst() {
    covers(&["ERR:compress/zstd_compress.c:4124"]);
    for (name, kind, n) in [
        ("compressible", Corpus::Text, 256 * 1024),
        ("random", Corpus::Random, 200_000),
        // Periodic and Counter compress a 256 KB (two-block) input to ~3 KB and
        // ~300 bytes while still producing plenty of sequences, which makes an
        // EXHAUSTIVE dstCapacity sweep affordable. That matters for `:4124`:
        // a partition boundary leaves a residual of exactly 0/1/2 bytes for only
        // three consecutive capacities, so a sampled sweep steps over it.
        ("periodic", Corpus::Periodic, 256 * 1024),
        ("counter", Corpus::Counter, 256 * 1024),
    ] {
        let src = corpus(kind, n, 0x184);
        for (mode, sets) in [
            (
                "split",
                vec![
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_splitAfterSequences, 1),
                ],
            ),
            (
                "targetCBlock",
                vec![
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_targetCBlockSize, 1340),
                ],
            ),
        ] {
            let full = c_compress_with(&src, &sets, None).len();
            let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 8, 16];
            // fine sweep around the achievable size and around srcSize
            for base in [full, src.len(), src.len() + 3] {
                for d in -8i64..=8 {
                    let c = base as i64 + d;
                    if c >= 0 {
                        caps.push(c as usize);
                    }
                }
            }
            if mode == "targetCBlock" {
                // TIGHT CAPACITIES ARE OUT OF CONTRACT ON THIS PATH.
                // `ZSTD_compressSubBlock_literal`
                // (compress/zstd_compress_superblock.c:71) does
                //     ZSTD_memcpy(op, hufMetadata->hufDesBuffer, hufMetadata->hufDesSize);
                // with `op = dst + lhSize` (3..5) and NO check against
                // `dstSize`, so any sub-block whose remaining capacity is
                // smaller than `lhSize + hufDesSize` (up to ~128 bytes) writes
                // out of bounds. Verified: `ZSTD_compress2` on 256 KB of
                // Corpus::Periodic with ZSTD_c_targetCBlockSize = 1340 and
                // dstCapacity = 18 SIGSEGVs the reference C (and silently
                // corrupts the heap at dstCapacity = 25 when the destination is
                // exactly `dstCapacity` bytes long). Only capacities from the
                // achievable frame size upwards are exercised here.
                caps.clear();
                caps.push(full);
                caps.push(full + 1);
                caps.push(full + 64);
                caps.push(compress_bound(&pair().c, src.len()) + 64);
            } else if full <= 4096 {
                caps.extend(0..=(full + 24));
            } else {
                let step = (full / 10).max(1);
                let mut c = 0usize;
                while c < full + 4 * step {
                    caps.push(c);
                    c += step;
                }
            }
            caps.sort_unstable();
            caps.dedup();
            let mut n_toosmall = 0usize;
            let mut n_ok_raw = 0usize;
            let mut n_ok = 0usize;
            for cap in caps {
                let got = diff_bytes(
                    &format!("tightdst/{name}/{mode}/cap{cap}"),
                    |l| {
                        let cctx = Ctx::cctx(l);
                        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
                        let mut srs = Vec::new();
                        for &(p, v) in &sets {
                            srs.push(res(l, unsafe { set(cctx.ptr, p, v) }));
                        }
                        // A canary past `dstCapacity`: a library that writes
                        // beyond the capacity it was given would otherwise
                        // corrupt the heap and abort the whole run instead of
                        // being reported as a divergence.
                        const CANARY: u8 = 0xA7;
                        const CANARY_LEN: usize = 64;
                        let mut buf = vec![0xCDu8; cap + CANARY_LEN];
                        for b in &mut buf[cap..] {
                            *b = CANARY;
                        }
                        let k = unsafe {
                            l.sym::<FnCompress2>("ZSTD_compress2")(
                                cctx.ptr,
                                buf.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                src.len(),
                            )
                        };
                        let overrun = buf[cap..].iter().filter(|&&b| b != CANARY).count();
                        assert_eq!(
                            overrun, 0,
                            "[{}] wrote {overrun} bytes past dstCapacity={cap}",
                            l.tag
                        );
                        let r = res(l, k);
                        let mut dst = buf[..cap].to_vec();
                        if let R::Ok(m) = r {
                            dst.truncate(m);
                        }
                        // round-trip anything that succeeded
                        let mut rt = R::Ok(0);
                        if matches!(r, R::Ok(m) if m > 0) && overrun == 0 {
                            let mut back = vec![0xEEu8; src.len() + 64];
                            rt = res(l, unsafe {
                                l.sym::<FnDecompress>("ZSTD_decompress")(
                                    back.as_mut_ptr() as *mut c_void,
                                    back.len(),
                                    dst.as_ptr() as *const c_void,
                                    dst.len(),
                                )
                            });
                            if let R::Ok(m) = rt {
                                assert_eq!(&back[..m], &src[..], "[{}] round-trip", l.tag);
                            }
                        }
                        (srs, r, rt, overrun, Blob(dst))
                    },
                );
                match got.1 {
                    R::Err(70, _) => n_toosmall += 1,
                    R::Ok(_) => {
                        n_ok += 1;
                        // a raw first block means the compressed path was
                        // abandoned for this capacity
                        if let Some((btype, _, _)) =
                            first_block_lit_size(&pair().c, &got.4 .0)
                        {
                            if btype == 0 {
                                n_ok_raw += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if mode != "targetCBlock" {
                assert!(
                    n_toosmall > 2,
                    "{name}/{mode}: expected several dstSize_tooSmall capacities, saw {n_toosmall}"
                );
            }
            assert!(n_ok > 0, "{name}/{mode}: no capacity succeeded");
            let _ = n_ok_raw;
        }
    }
}
