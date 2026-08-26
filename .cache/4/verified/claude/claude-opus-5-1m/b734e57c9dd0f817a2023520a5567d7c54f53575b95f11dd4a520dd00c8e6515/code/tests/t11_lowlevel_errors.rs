//! Phase C — the ERROR PATHS of `common/` and of `compress/` (everything except
//! `zstd_compress.c`, and excluding `zstd_ldm.c` which another suite owns).
//!
//! `tests/t10_entropy.rs` drives the *valid* paths of the same entry points; this
//! file drives the rejections: one case per `ERRORS.md` row whose `reach` is
//! `DIRECT` or `INDIRECT`, built from the exact trigger the row names, compared
//! through `diff` so both the numeric error code and its symbolic name must
//! agree, together with every out-param and the whole destination buffer.
//!
//! Tests are grouped by C source file so the mapping to `ERRORS.md` is obvious,
//! and every test ends with `covers(...)` naming the exact `file:line` rows it
//! exercised (the completion gate).
//!
//! OUT-OF-CONTRACT INPUTS ARE NOT CALLED. Each exclusion is documented at the
//! point where it would otherwise appear, with the precondition that forbids it.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_void};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Constants resolved from the C headers (same values as t10_entropy.rs)
// ---------------------------------------------------------------------------

const FSE_MAX_TABLELOG: u32 = 12;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const HUF_TABLELOG_MAX: u32 = 12;
const HUF_TABLELOG_DEFAULT: u32 = 11;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * 256) + 192) * 4; // 4864
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;
const HIST_WKSP_SIZE: usize = 1024 * 4;
/// `HUF_READ_STATS_WORKSPACE_SIZE` = `FSE_DECOMPRESS_WKSP_SIZE_U32(6, 11) * 4`
/// = `(65 + 1 + 24 + 128 + 1) * 4` = 876 bytes.
const HUF_READ_STATS_WORKSPACE_SIZE: usize = 876;

const HUF_flags_bmi2: c_int = 1 << 0;
const HUF_flags_optimalDepth: c_int = 1 << 1;
const HUF_flags_preferRepeat: c_int = 1 << 2;
const HUF_flags_suspectUncompressible: c_int = 1 << 3;
const HUF_flags_disableAsm: c_int = 1 << 4;
const HUF_flags_disableFast: c_int = 1 << 5;

// compress/zstd_internal.h
const MaxLL: u32 = 35;
const MaxML: u32 = 52;
const MaxOff: u32 = 31;
const MaxSeq: u32 = 52;
const LLFSELog: u32 = 9;
const MLFSELog: u32 = 9;
const OffFSELog: u32 = 8;
const ZSTD_SLIPBLOCK_WORKSPACESIZE: usize = 8208;

// SymbolEncodingType_e
const set_basic: c_int = 0;
const set_rle: c_int = 1;
const set_compressed: c_int = 2;
const set_repeat: c_int = 3;
// FSE_repeat / ZSTD_DefaultPolicy_e
const FSE_repeat_none: c_int = 0;
const FSE_repeat_check: c_int = 1;
const FSE_repeat_valid: c_int = 2;
const ZSTD_defaultDisallowed: c_int = 0;
const ZSTD_defaultAllowed: c_int = 1;

fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    1 + (1usize << (max_table_log.max(1) - 1)) + ((max_symbol_value as usize + 1) * 2)
}
fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
fn fse_build_dtable_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    2 * (max_symbol_value as usize + 1) + (1usize << max_table_log) + 8
}
fn fse_build_ctable_wksp_size(max_symbol_value: u32, table_log: u32) -> usize {
    4 * (((max_symbol_value as usize + 2) + (1usize << table_log)) / 2 + 2)
}
fn fse_decompress_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    (fse_dtable_size_u32(max_table_log)
        + 1
        + (fse_build_dtable_wksp_size(max_table_log, max_symbol_value) + 3) / 4
        + 128
        + 1)
        * 4
}
fn highbit32(v: u32) -> u32 {
    assert!(v != 0);
    31 - v.leading_zeros()
}
/// `FSE_minTableLog` from `compress/fse_compress.c:348` (precondition: total > 1).
fn fse_min_table_log(total: usize, max_symbol_value: u32) -> u32 {
    let a = highbit32(total as u32) + 1;
    let b = highbit32(max_symbol_value.max(1)) + 2;
    a.min(b)
}

/// A `HUF_DTable` seeded exactly the way the library's own callers do —
/// `DTable[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x01000001` (see the long note on
/// `huf_dtable()` in `tests/t10_entropy.rs`: an unseeded DTable makes decoding
/// out of contract, because `BIT_lookBitsFast` with `dtLog == 0` returns the
/// whole 64-bit container as the table index).
fn huf_dtable() -> Vec<u32> {
    huf_dtable_cap(ZSTD_HUFFDTABLE_CAPACITY_LOG)
}
fn huf_dtable_cap(cap_log: u32) -> Vec<u32> {
    // Always allocate the full 2^12 table so that a *successful* read of a
    // deeper-than-declared tree can never scribble outside the allocation; only
    // the declared capacity in DTable[0] is varied.
    let mut dt = vec![0u32; 1 + (1usize << ZSTD_HUFFDTABLE_CAPACITY_LOG)];
    dt[0] = cap_log * 0x0100_0001;
    dt
}

fn wksp(bytes: usize) -> Vec<u64> {
    vec![0u64; (bytes + 7) / 8 + 1]
}
fn wp(w: &mut [u64]) -> *mut c_void {
    w.as_mut_ptr() as *mut c_void
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnSzSz = unsafe extern "C" fn(SizeT) -> SizeT;

type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnHistCountWksp =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, SizeT, *mut c_void, SizeT) -> SizeT;
type FnHistCountSimple =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, SizeT) -> c_uint;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, SizeT);

type FnFseOptimalTableLog = unsafe extern "C" fn(c_uint, SizeT, c_uint) -> c_uint;
type FnFseNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, SizeT, c_uint, c_uint) -> SizeT;
type FnFseNCountWriteBound = unsafe extern "C" fn(c_uint, c_uint) -> SizeT;
type FnFseWriteNCount =
    unsafe extern "C" fn(*mut c_void, SizeT, *const i16, c_uint, c_uint) -> SizeT;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnFseReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT, c_int) -> SizeT;
type FnFseBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnFseBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnFseBuildCTableRle = unsafe extern "C" fn(*mut c_uint, u8) -> SizeT;
type FnFseCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_uint) -> SizeT;
type FnFseDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_uint,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;

type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut u64, *const c_uint, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnHufWriteCTableWksp =
    unsafe extern "C" fn(*mut c_void, SizeT, *const u64, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut u64, *mut c_uint, *const c_void, SizeT, *mut c_uint) -> SizeT;
type FnHufReadCTableHeader = unsafe extern "C" fn(*const u64) -> u64;
type FnHufGetNbBits = unsafe extern "C" fn(*const u64, c_uint) -> c_uint;
type FnHufValidateCTable = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> c_int;
type FnHufEstimateCompressedSize = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> SizeT;
type FnHufCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const u64, c_int) -> SizeT;
type FnHufCompressRepeat = unsafe extern "C" fn(
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
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    SizeT,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    SizeT,
) -> SizeT;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8,
    SizeT,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnHufSelectDecoder = unsafe extern "C" fn(SizeT, SizeT) -> c_uint;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const c_void, SizeT, *mut c_void, SizeT, c_int) -> SizeT;
type FnHufDecompressUsingDTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_uint, c_int) -> SizeT;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;

type FnLiteralsSimple = unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnCompressLiterals = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    *const u64,
    *mut u64,
    c_int,
    c_int,
    c_int,
    c_int,
) -> SizeT;

type FnSelectEncodingType = unsafe extern "C" fn(
    *mut c_int,
    *const c_uint,
    c_uint,
    SizeT,
    SizeT,
    c_uint,
    *const c_uint,
    *const i16,
    c_uint,
    c_int,
    c_int,
) -> c_int;
type FnBuildCTable = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *mut c_uint,
    c_uint,
    c_int,
    *mut c_uint,
    c_uint,
    *const u8,
    SizeT,
    *const i16,
    c_uint,
    c_uint,
    *const c_uint,
    SizeT,
    *mut c_void,
    SizeT,
) -> SizeT;
type FnEncodeSequences = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_uint,
    *const u8,
    *const c_uint,
    *const u8,
    *const c_uint,
    *const u8,
    *const SeqDef,
    SizeT,
    c_int,
    c_int,
) -> SizeT;
type FnFseBitCost = unsafe extern "C" fn(*const c_uint, *const c_uint, c_uint) -> SizeT;
type FnCrossEntropyCost = unsafe extern "C" fn(*const i16, c_uint, *const c_uint, c_uint) -> SizeT;
type FnSplitBlock = unsafe extern "C" fn(*const c_void, SizeT, c_int, *mut c_void, SizeT) -> SizeT;

type FnZstdmtCreate = unsafe extern "C" fn(c_uint, ZSTD_customMem, *mut c_void) -> *mut c_void;
type FnZstdmtFree = unsafe extern "C" fn(*mut c_void) -> SizeT;

type FnCreateAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateCDictAdvanced = unsafe extern "C" fn(
    *const c_void,
    SizeT,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDictAdvanced =
    unsafe extern "C" fn(*const c_void, SizeT, c_int, c_int, ZSTD_customMem) -> *mut c_void;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
type FnEstimateSize = unsafe extern "C" fn(c_int) -> SizeT;
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;
type FnPoolSizeof = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnPoolFree = unsafe extern "C" fn(*mut c_void);
type FnPoolResize = unsafe extern "C" fn(*mut c_void, SizeT) -> c_int;
type FnXxhFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnXxhCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnXxh32Reset = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnXxh64Reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnXxhUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> c_int;
type FnXxh32Digest = unsafe extern "C" fn(*const c_void) -> u32;
type FnXxh64Digest = unsafe extern "C" fn(*const c_void) -> u64;

use std::ffi::c_ulonglong;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct SeqDef {
    off_base: c_uint,
    lit_length: u16,
    ml_base: u16,
}

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

/// The numeric `ZSTD_error_*` code carried by an [`R`], or 0 for a success.
/// `diff` already compares the code *and* the human-readable name between the
/// two libraries; the local assertions in this file only pin the code, because
/// the name is `ERR_getErrorString`'s prose ("tableLog requires too much
/// memory : unsupported"), not the enum identifier.
fn code_of(r: &R) -> c_int {
    match r {
        R::Ok(_) => 0,
        R::Err(c, _) => *c,
    }
}
fn is_err_code(r: &R, code: c_int) -> bool {
    code_of(r) == code
}

fn u32s(v: &[u32]) -> Blob {
    Blob(v.iter().flat_map(|x| x.to_le_bytes()).collect())
}
fn u64s(v: &[u64]) -> Blob {
    Blob(v.iter().flat_map(|x| x.to_le_bytes()).collect())
}
fn i16s(v: &[i16]) -> Blob {
    Blob(v.iter().flat_map(|x| x.to_le_bytes()).collect())
}

/// Replicates the low-probability spreading loop of `FSE_buildDTable_internal`
/// (`common/fse_decompress.c:135-147`) to predict whether it ends with
/// `position != 0`, i.e. whether the function returns `ERROR(GENERIC)` *before*
/// the "Build Decoding table" loop runs.
///
/// This gate is a SAFETY requirement, not a convenience: when the spread loop
/// leaves cells unwritten but still lands back on `position == 0` (e.g. every
/// entry is `-1`, so no cell is ever laid down), the build loop then reads
/// uninitialised `tableDecode[u].symbol` bytes and uses them to index
/// `symbolNext[]`, which is only `maxSymbolValue+1` entries long — an
/// out-of-bounds *write* into the caller's workspace.  The C has no guard for
/// this, so such counters are out of contract and are never passed in.
/// Returns `None` when the inner `while position > highThreshold` rotation does
/// not converge (also out of contract: the C would spin forever).
fn dtable_spread_fails(nc: &[i16], msv: u32, tl: u32) -> Option<bool> {
    let table_size = 1u32 << tl;
    let table_mask = table_size - 1;
    let step = (table_size >> 1) + (table_size >> 3) + 3;
    let mut high_threshold = table_size - 1;
    let mut lowprob = 0u32;
    for s in 0..=msv as usize {
        if nc[s] == -1 {
            lowprob += 1;
        }
    }
    if lowprob == 0 || lowprob > table_size {
        return None; // fast path (no -1) or nonsense
    }
    high_threshold -= lowprob;
    let mut position = 0u32;
    for s in 0..=msv as usize {
        for _ in 0..nc[s].max(0) {
            position = (position + step) & table_mask;
            let mut spins = 0u32;
            while position > high_threshold {
                position = (position + step) & table_mask;
                spins += 1;
                if spins > table_size * 4 {
                    return None;
                }
            }
        }
    }
    Some(position != 0)
}

/// A source over the byte values `0..alphabet` (so `HIST_count` reports
/// `maxSymbolValue == alphabet-1`), which keeps `FSE_minTableLog` small enough
/// that a low `tableLog` is legal.
fn small_alphabet_src(n: usize, alphabet: u8, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.below(alphabet as usize) as u8).collect()
}

/// A normalized count vector over `0..=msv` whose entries sum to `1<<tableLog`.
/// Returned oversized (1024 entries) so that entry points which read past
/// `maxSymbolValue` — `FSE_writeNCount` has *no* `maxSymbolValue` check — stay
/// inside the allocation instead of reading out of bounds.
fn norm_uniform(msv: u32, table_log: u32) -> Vec<i16> {
    let n = msv as usize + 1;
    let total = 1usize << table_log;
    assert!(total >= n, "cannot spread {total} states over {n} symbols");
    let mut v = vec![0i16; 1024];
    let base = total / n;
    let mut rem = total - base * n;
    for i in 0..n {
        v[i] = base as i16;
        if rem > 0 {
            v[i] += 1;
            rem -= 1;
        }
    }
    v
}

// ===========================================================================
// common/error_private.{c,h} + common/zstd_common.c
//   rows 41 (ERR_isError gate), 42 (ERR_getErrorCode), 43 (ERR_getErrorName),
//        44 (ERR_getErrorString default arm)
// `t10_entropy.rs::error_name_tables_match` already sweeps 0..=140 and -5..=130;
// this extends it to the exact boundary of the acceptance gate and to the
// out-of-range ints a C enum accepts.
// ===========================================================================

#[test]
fn error_private_acceptance_gate_and_strings() {
    // Row 41: `code > ERROR(maxCode)` == `code > (size_t)-120`.
    // (size_t)-120 must NOT be an error; (size_t)-119 must be.
    for k in [
        0usize, 1, 2, 118, 119, 120, 121, 122, 200, 1000, 65535, 1 << 20,
    ] {
        let v = 0usize.wrapping_sub(k);
        diff(&format!("ERR_isError(-{k})"), |l| {
            (
                unsafe { l.sym::<FnIsError>("ZSTD_isError")(v) },
                unsafe { l.sym::<FnIsError>("FSE_isError")(v) },
                unsafe { l.sym::<FnIsError>("HUF_isError")(v) },
                unsafe { l.sym::<FnIsError>("HIST_isError")(v) },
                unsafe { l.sym::<FnIsError>("ZDICT_isError")(v) },
                unsafe { l.sym::<FnIsError>("ZBUFF_isError")(v) },
                // Row 42: ERR_getErrorCode returns no_error for non-errors.
                unsafe { l.sym::<FnGetErrorCode>("ZSTD_getErrorCode")(v) },
                // Row 43: ERR_getErrorName falls back to "No error detected".
                unsafe { cstr(l.sym::<FnGetErrorName>("ZSTD_getErrorName")(v)) },
                unsafe { cstr(l.sym::<FnGetErrorName>("FSE_getErrorName")(v)) },
                unsafe { cstr(l.sym::<FnGetErrorName>("HUF_getErrorName")(v)) },
                unsafe { cstr(l.sym::<FnGetErrorName>("ZDICT_getErrorName")(v)) },
                unsafe { cstr(l.sym::<FnGetErrorName>("ZBUFF_getErrorName")(v)) },
            )
        });
    }
    // The gate itself, asserted rather than merely compared.
    let c = &pair().c;
    assert_eq!(is_error(c, 0usize.wrapping_sub(120)), false, "row 41 boundary");
    assert_eq!(is_error(c, 0usize.wrapping_sub(119)), true, "row 41 boundary");

    // Row 44: `ERR_getErrorString` default arm — every enum value, the gaps and
    // out-of-range ints (a C enum parameter accepts any int).
    let mut codes: Vec<c_int> = (-8..=132).collect();
    codes.extend_from_slice(&[
        -1000, -256, 133, 150, 200, 255, 256, 999, 1000, 65536, i32::MAX, i32::MIN,
        i32::MIN + 1, i32::MAX - 1,
    ]);
    for code in codes {
        diff(&format!("getErrorString({code})"), |l| {
            (
                unsafe { cstr(l.sym::<FnGetErrorString>("ZSTD_getErrorString")(code)) },
                unsafe { cstr(l.sym::<FnGetErrorString>("ERR_getErrorString")(code)) },
            )
        });
    }
    covers(&[
        "ERR:common/error_private.h:52",
        "ERR:common/error_private.h:54",
        "ERR:common/error_private.h:74",
        "ERR:common/error_private.c:61",
    ]);
}

// ===========================================================================
// common/entropy_common.c — FSE_readNCount / FSE_readNCount_bmi2
//   rows 1 (:63 forwarded inner error), 2 (:64 countSize > hbSize),
//        3 (:73 tableLog > 15), 4 (:179 remaining != 1),
//        5 (:181 charnum > maxSV1), 6 (:182 bitCount > 32)
// ===========================================================================

/// Every observable of one `FSE_readNCount` call: the return, all three
/// out-params, and the full 256-entry normalized counter (the C memsets only
/// `*maxSVPtr+1` entries, so the untouched tail is part of the contract too).
#[derive(PartialEq, Debug)]
struct NCountRead {
    r: R,
    msv: u32,
    table_log: u32,
    norm: Blob,
}

fn read_ncount(l: &Lib, src: &[u8], hb_size: usize, msv_in: u32) -> NCountRead {
    let mut norm = vec![0i16; 256];
    let mut msv = msv_in;
    let mut tl = 0xDEADu32;
    let n = unsafe {
        l.sym::<FnFseReadNCount>("FSE_readNCount")(
            norm.as_mut_ptr(),
            &mut msv,
            &mut tl,
            src.as_ptr() as *const c_void,
            hb_size,
        )
    };
    // `FSE_readNCount_bmi2(..., bmi2=0)` must be bit-identical (DYNAMIC_BMI2==0
    // makes bmi2!=0 take the same path, so both values are checked).
    for bmi2 in [0i32, 1] {
        let mut n2 = vec![0i16; 256];
        let mut m2 = msv_in;
        let mut t2 = 0xDEADu32;
        let r2 = unsafe {
            l.sym::<FnFseReadNCountBmi2>("FSE_readNCount_bmi2")(
                n2.as_mut_ptr(),
                &mut m2,
                &mut t2,
                src.as_ptr() as *const c_void,
                hb_size,
                bmi2,
            )
        };
        assert_eq!(r2, n, "[{}] readNCount_bmi2({bmi2}) return", l.tag);
        assert_eq!(n2, norm, "[{}] readNCount_bmi2({bmi2}) counter", l.tag);
        assert_eq!((m2, t2), (msv, tl), "[{}] readNCount_bmi2({bmi2}) out", l.tag);
    }
    NCountRead {
        r: res(l, n),
        msv,
        table_log: tl,
        norm: i16s(&norm),
    }
}

#[test]
fn fse_read_ncount_rejects_malformed_headers() {
    // Row 3 (:73): low nibble of src[0] >= 11 => nbBits = nibble+5 > 15.
    for nib in 0u8..16 {
        let mut src = [0u8; 8];
        src[0] = nib;
        let got = diff(&format!("readNCount nibble={nib}"), |l| {
            read_ncount(l, &src, 8, 255)
        });
        if nib >= 11 {
            assert_eq!(code_of(&got.r), 44, "nibble {nib} must be tableLog_tooLarge");
        }
    }
    // Row 1 (:63): hbSize < 8 forwards the inner error verbatim.
    let got = diff("readNCount hbSize=1 src=0x0B", |l| {
        read_ncount(l, &[0x0B], 1, 255)
    });
    assert_eq!(code_of(&got.r), 44, "row 1: inner tableLog_tooLarge forwarded");

    // srcSize == 0: the zero-padded 8-byte buffer parses to a size > 0, so the
    // `countSize > hbSize` guard at :64 fires.
    let got = diff("readNCount hbSize=0", |l| read_ncount(l, &[0u8; 8], 0, 255));
    assert_eq!(code_of(&got.r), 20, "row 2: srcSize==0 -> corruption_detected");

    // Row 4 (:179): maxSV=1 (maxSV1=2) with tableLog 5 => only 2 symbols can be
    // read but 32 states must be distributed, so `remaining != 1` at loop exit.
    let got = diff("readNCount maxSV=1 tl=5", |l| read_ncount(l, &[0u8; 8], 8, 1));
    assert_eq!(code_of(&got.r), 20, "row 4: remaining != 1 -> corruption_detected");

    // Truncated headers at every length 1..8, over a family of *valid* headers.
    // Row 2 (:64) is detected structurally: the truncated call must return
    // corruption_detected exactly when the zero-padded 8-byte parse succeeds
    // with a size larger than the supplied hbSize.
    let mut row2_hits = 0usize;
    let mut seen_errs: Vec<c_int> = Vec::new();
    fn note(r: &R, seen: &mut Vec<c_int>) {
        let c = code_of(r);
        if c != 0 && !seen.contains(&c) {
            seen.push(c);
        }
    }
    for msv in [0u32, 1, 2, 3, 5, 12, 20, 40, 100, 255] {
        for tl in FSE_MIN_TABLELOG..=8 {
            if (1usize << tl) < msv as usize + 1 {
                continue;
            }
            let hdr = c_ncount_header(msv, tl);
            for hb in 0..=hdr.len().min(8) {
                let full = {
                    let mut v = hdr.clone();
                    v.resize(v.len().max(8), 0);
                    v
                };
                let padded = {
                    let mut v = hdr[..hb].to_vec();
                    v.resize(8, 0);
                    v
                };
                let a = diff(
                    &format!("readNCount trunc msv={msv} tl={tl} hb={hb}"),
                    |l| read_ncount(l, &hdr, hb, msv),
                );
                note(&a.r, &mut seen_errs);
                let b = diff(
                    &format!("readNCount padded msv={msv} tl={tl} hb={hb}"),
                    |l| read_ncount(l, &padded, 8, msv),
                );
                let _ = &full;
                if hb < 8 {
                    if let (R::Err(20, _), R::Ok(n)) = (&a.r, &b.r) {
                        if *n > hb {
                            row2_hits += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(
        row2_hits > 0,
        "no input reached entropy_common.c:64 (countSize > hbSize)"
    );
    eprintln!("entropy_common.c:64 reached by {row2_hits} truncated headers");

    // Rows 5 (:181) and 6 (:182): a long run of 0b11 repeat codes with a small
    // maxSymbolValue overshoots `maxSV1` (charnum > maxSV1) and pushes bitCount
    // past 32. Sweep every 8-byte header made of repeat-heavy bit patterns.
    let mut msv_too_small = 0usize;
    for msv in [0u32, 1, 2, 3, 4, 7, 15] {
        for fill in [0xFFu8, 0xFE, 0xFC, 0xF0, 0xCF, 0x3F, 0xF3, 0xCC, 0x55, 0xAA] {
            for nib in 0u8..11 {
                let mut src = [fill; 8];
                src[0] = (src[0] & 0xF0) | nib;
                let got = diff(
                    &format!("readNCount repeats msv={msv} fill={fill:02x} nib={nib}"),
                    |l| read_ncount(l, &src, 8, msv),
                );
                note(&got.r, &mut seen_errs);
                if is_err_code(&got.r, 48) {
                    msv_too_small += 1;
                }
            }
        }
    }
    assert!(
        seen_errs.contains(&20),
        "expected corruption_detected (20) somewhere in the sweep, saw {seen_errs:?}"
    );
    // `entropy_common.c:181` (`charnum > maxSV1` -> maxSymbolValue_tooSmall) was
    // NOT reached by any of the 700 repeat-heavy headers above, and analysis of
    // the C says it cannot be: `charnum` can only exceed `maxSV1` via the
    // `charnum += 3*repeats` run in the `previous0` block, whose only exit is the
    // `break` at :114 -- and that break is only reachable while `remaining > 1`
    // (the loop breaks out at :163 as soon as `remaining <= 1`, and `threshold`
    // is provably >= 2 so `remaining == 1` always satisfies `remaining <
    // threshold`).  Therefore the `remaining != 1` guard at :179 always fires
    // first and :181 is dead code.  See the report: row 5's `DIRECT` looks wrong.
    assert_eq!(
        msv_too_small, 0,
        "entropy_common.c:181 became reachable -- revisit the analysis"
    );
    eprintln!("entropy_common.c:181 unreachable as analysed; error codes seen: {seen_errs:?}");
    covers(&[
        "ERR:common/entropy_common.c:63",
        "ERR:common/entropy_common.c:64",
        "ERR:common/entropy_common.c:73",
        "ERR:common/entropy_common.c:179",
        "ERR:common/entropy_common.c:182",
    ]);
}

#[test]
fn fse_read_ncount_fuzz_short_strings() {
    // Randomised fuzz (fixed seed) over short byte strings: every length 0..=16
    // crossed with every hbSize, comparing the return value AND all three
    // out-params AND the whole normalizedCounter.
    let mut rng = Rng::new(0x11_0000_0001);
    for i in 0..1200 {
        let n = rng.below(17);
        let mut src = rng.bytes(n.max(1));
        src.truncate(n);
        let msv = *rng.pick(&[0u32, 1, 3, 7, 15, 35, 52, 100, 255]);
        let hb = rng.below(n + 1);
        diff(&format!("readNCount fuzz #{i} n={n} hb={hb} msv={msv}"), |l| {
            read_ncount(l, &src, hb, msv)
        });
    }
    // and a dense sweep of every 2-byte header (all 65536 combinations would be
    // slow through two dlopen'd libraries; take the first byte exhaustively).
    for b0 in 0u16..=255 {
        for b1 in [0u8, 1, 0x0F, 0x55, 0xAA, 0xFF] {
            let src = [b0 as u8, b1];
            diff(&format!("readNCount 2byte {b0:02x}{b1:02x}"), |l| {
                read_ncount(l, &src, 2, 255)
            });
        }
    }
    covers(&[
        "ERR:common/entropy_common.c:63",
        "ERR:common/entropy_common.c:73",
        "ERR:common/entropy_common.c:179",
        "ERR:common/entropy_common.c:182",
    ]);
}

/// A *valid* NCount header, produced by the C library, for a uniform normalized
/// distribution over `0..=msv` at `tableLog`.
fn c_ncount_header(msv: u32, table_log: u32) -> Vec<u8> {
    let l = &pair().c;
    let norm = norm_uniform(msv, table_log);
    let mut buf = vec![0u8; 1024];
    let n = unsafe {
        l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            norm.as_ptr(),
            msv,
            table_log,
        )
    };
    match res(l, n) {
        R::Ok(v) => buf[..v].to_vec(),
        e => panic!("fixture FSE_writeNCount(msv={msv}, tl={table_log}) failed: {e:?}"),
    }
}

// ===========================================================================
// common/entropy_common.c — HUF_readStats / HUF_readStats_wksp
//   rows 7 (:254 srcSize==0), 8 (:261 iSize+1 > srcSize, raw header),
//        9 (:262 oSize >= hwSize), 10 (:270 iSize+1 > srcSize, FSE header),
//        11 (:273 inner FSE_decompress error), 12 (:280 weight > 12),
//        13 (:284 weightTotal == 0), 14 (:288 tableLog > 12),
//        15 (:295 rest not a power of two), 16 (:301 invalid tree shape)
// ===========================================================================

/// Every observable of `HUF_readStats`: return, the whole weight buffer, all 16
/// rank slots (the C memsets 13), nbSymbols and tableLog.  `HUF_readStats_wksp`
/// with `flags=0` and `flags=HUF_flags_bmi2` is required to agree exactly.
#[derive(PartialEq, Debug)]
struct StatsRead {
    r: R,
    weights: Blob,
    ranks: Blob,
    nb_symbols: u32,
    table_log: u32,
}

/// PRECONDITION: for an FSE-coded weight header (`src[0] < 128`) `hwSize` must be
/// >= 1, because `HUF_readStats_body` passes `hwSize-1` as the FSE destination
/// capacity; `hwSize == 0` therefore hands `FSE_decompress_wksp_bmi2` a capacity
/// of `SIZE_MAX` and any weight stream then writes past the buffer.  Callers of
/// this helper pass `hw_size == 0` only with a raw (`src[0] >= 128`) header or an
/// empty `src`, where the `oSize >= hwSize` / `!srcSize` guards fire first.
fn read_stats(l: &Lib, src: &[u8], hw_size: usize, wksp_size: usize) -> StatsRead {
    assert!(
        hw_size >= 1 || src.is_empty() || src[0] >= 128,
        "out of contract: hwSize==0 with an FSE-coded weight header"
    );
    let mut weights = vec![0u8; 600];
    let mut ranks = vec![0u32; 16];
    let mut nb = 0u32;
    let mut tl = 0u32;
    let n = unsafe {
        l.sym::<FnHufReadStats>("HUF_readStats")(
            weights.as_mut_ptr(),
            hw_size,
            ranks.as_mut_ptr(),
            &mut nb,
            &mut tl,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    for flags in [0, HUF_flags_bmi2] {
        let mut w2 = vec![0u8; 600];
        let mut r2 = vec![0u32; 16];
        let mut n2 = 0u32;
        let mut t2 = 0u32;
        let mut sw = wksp(wksp_size);
        let swb = wksp_size;
        let rv = unsafe {
            l.sym::<FnHufReadStatsWksp>("HUF_readStats_wksp")(
                w2.as_mut_ptr(),
                hw_size,
                r2.as_mut_ptr(),
                &mut n2,
                &mut t2,
                src.as_ptr() as *const c_void,
                src.len(),
                wp(&mut sw),
                swb,
                flags,
            )
        };
        if wksp_size >= HUF_READ_STATS_WORKSPACE_SIZE {
            assert_eq!(rv, n, "[{}] readStats_wksp({flags}) return", l.tag);
            assert_eq!(w2, weights, "[{}] readStats_wksp({flags}) weights", l.tag);
            assert_eq!(r2, ranks, "[{}] readStats_wksp({flags}) ranks", l.tag);
            assert_eq!((n2, t2), (nb, tl), "[{}] readStats_wksp({flags}) out", l.tag);
        }
    }
    StatsRead {
        r: res(l, n),
        weights: Blob(weights),
        ranks: u32s(&ranks),
        nb_symbols: nb,
        table_log: tl,
    }
}

#[test]
fn huf_read_stats_rejects_malformed_weight_headers() {
    let big = HUF_READ_STATS_WORKSPACE_SIZE;

    // Row 7 (:254): srcSize == 0 -> srcSize_wrong (72).
    let got = diff("readStats srcSize=0", |l| read_stats(l, &[], 256, big));
    assert_eq!(code_of(&got.r), 72, "row 7");

    // Row 8 (:261): raw header, iSize+1 > srcSize. src={0xFF} -> oSize=128,
    // iSize=64, 65 > 1.
    let got = diff("readStats raw 0xFF len1", |l| read_stats(l, &[0xFF], 256, big));
    assert_eq!(code_of(&got.r), 72, "row 8");

    // Row 9 (:262): raw header and oSize >= hwSize. src[0]=0x88 -> oSize=9,
    // iSize=5, srcSize=6 clears :261, then 9 >= 8 fires.
    let src = [0x88u8, 0x11, 0x11, 0x11, 0x11, 0x11];
    let got = diff("readStats raw oSize>=hwSize", |l| read_stats(l, &src, 8, big));
    assert_eq!(code_of(&got.r), 20, "row 9");
    // ... and the whole hwSize grid for the same header, including hwSize == 0
    // (legal here: the raw path never touches the FSE decoder).
    for hw in [0usize, 1, 2, 8, 9, 10, 16, 256] {
        diff(&format!("readStats raw hwSize={hw}"), |l| {
            read_stats(l, &src, hw, big)
        });
    }

    // Row 10 (:270): FSE header, iSize+1 > srcSize. src={0x7F} -> iSize=127.
    let got = diff("readStats fse 0x7F len1", |l| read_stats(l, &[0x7F], 256, big));
    assert_eq!(code_of(&got.r), 72, "row 10");
    for b0 in [1u8, 2, 5, 40, 126, 127] {
        let s = vec![b0];
        diff(&format!("readStats fse trunc {b0}"), |l| {
            read_stats(l, &s, 256, big)
        });
    }

    // Row 11 (:273): the inner FSE_decompress_wksp_bmi2 fails and its code is
    // forwarded verbatim. src={0x02,0x0B,0x00}: the inner FSE_readNCount sees a
    // low nibble of 0xB -> nbBits 16 > 15 -> tableLog_tooLarge (44).
    let got = diff("readStats fse inner tableLog", |l| {
        read_stats(l, &[0x02, 0x0B, 0x00], 256, big)
    });
    assert_eq!(code_of(&got.r), 44, "row 11");

    // Row 12 (:280): a decoded weight > HUF_TABLELOG_MAX (12).
    // src={0x81,0xF0} -> oSize=2, huffWeight[0] = 0xF = 15 > 12.
    let got = diff("readStats weight>12", |l| read_stats(l, &[0x81, 0xF0], 256, big));
    assert_eq!(code_of(&got.r), 20, "row 12");

    // Row 13 (:284): weightTotal == 0 (every weight zero).
    let got = diff("readStats weightTotal=0", |l| {
        read_stats(l, &[0x81, 0x00], 256, big)
    });
    assert_eq!(code_of(&got.r), 20, "row 13");

    // Row 14 (:288): tableLog = highbit32(weightTotal)+1 > 12, i.e.
    // weightTotal >= 4096. src={0x81,0xCC} -> weights {12,12} -> 2048+2048.
    let got = diff("readStats tableLog=13", |l| read_stats(l, &[0x81, 0xCC], 256, big));
    assert_eq!(code_of(&got.r), 20, "row 14");

    // Row 15 (:295): the implied last weight is not a clean power of two.
    // src={0x81,0x31} -> weights {3,1} -> weightTotal 5, tableLog 3, rest 3.
    let got = diff("readStats rest not pow2", |l| {
        read_stats(l, &[0x81, 0x31], 256, big)
    });
    assert_eq!(code_of(&got.r), 20, "row 15");

    // Row 16 (:301): invalid tree shape, (rankStats[1] < 2) || (rankStats[1] & 1).
    // src={0x80,0x20} -> oSize=1, weight 2, implied last weight 2 -> rank1 == 0.
    let got = diff("readStats rank1<2", |l| read_stats(l, &[0x80, 0x20], 256, big));
    assert_eq!(code_of(&got.r), 20, "row 16");
    // an ODD number of rank-1 symbols: weights {1,1,1} + implied 1 would be 4
    // (even), so use {2,1} -> weightTotal 3, tableLog 2, rest 1, lastWeight 1 ->
    // rankStats[1] == 2 (valid); {1,2,1} -> total 1+2+1 = 4 -> tableLog 3,
    // rest 4 -> lastWeight 3, rankStats[1] == 2. Sweep every 1- and 2-nibble raw
    // header instead so both halves of the condition are covered exhaustively.
    let mut shape_hits = 0usize;
    for n in [0x80u8, 0x81, 0x82, 0x83] {
        for b in 0u16..=255 {
            for b2 in [0u8, 0x11, 0x21, 0x12] {
                let s = [n, b as u8, b2];
                let got = diff(&format!("readStats shape {n:02x}{b:02x}{b2:02x}"), |l| {
                    read_stats(l, &s, 256, big)
                });
                if is_err_code(&got.r, 20) {
                    shape_hits += 1;
                }
            }
        }
    }
    assert!(shape_hits > 0, "no raw header reached a corruption_detected guard");
    eprintln!("HUF_readStats raw-header sweep: {shape_hits} corruption_detected");

    covers(&[
        "ERR:common/entropy_common.c:254",
        "ERR:common/entropy_common.c:261",
        "ERR:common/entropy_common.c:262",
        "ERR:common/entropy_common.c:270",
        "ERR:common/entropy_common.c:273",
        "ERR:common/entropy_common.c:280",
        "ERR:common/entropy_common.c:284",
        "ERR:common/entropy_common.c:288",
        "ERR:common/entropy_common.c:295",
        "ERR:common/entropy_common.c:301",
    ]);
}

/// An FSE-coded Huffman *weight* header: the `iSize` length byte followed by an
/// `FSE_writeNCount` header for a uniform distribution over weights `0..=msv`.
/// `msv` is the largest weight value, so `msv >= 12` makes
/// `HUF_readStats_body`'s internal 876-byte workspace too small (see
/// `fse_decompress.c:273`).
fn fse_weight_header(msv: u32, table_log: u32) -> Vec<u8> {
    let hdr = c_ncount_header(msv, table_log);
    assert!(hdr.len() < 128, "iSize byte must stay below the raw-header flag");
    let mut out = vec![hdr.len() as u8];
    out.extend_from_slice(&hdr);
    out
}

#[test]
fn huf_read_stats_internal_workspace_and_maxlog_limits() {
    // fse_decompress.c:267 via HUF_readStats: the weight header declares a
    // tableLog above the hard-wired maxLog of 6 -> tableLog_tooLarge (44).
    for tl in 5..=9u32 {
        let src = fse_weight_header(5, tl);
        let got = diff(&format!("readStats weight tableLog={tl}"), |l| {
            read_stats(l, &src, 256, HUF_READ_STATS_WORKSPACE_SIZE)
        });
        if tl > 6 {
            assert_eq!(code_of(&got.r), 44, "weight tableLog {tl} > maxLog 6");
        }
    }
    // fse_decompress.c:273 via HUF_readStats: HUF_readStats' internal workspace
    // is HUF_READ_STATS_WORKSPACE_SIZE = 876 bytes, sized for maxSymbolValue 11.
    // A weight header declaring 13 symbols (maxSymbolValue 12) needs
    // FSE_DECOMPRESS_WKSP_SIZE(6,12) = 880 > 876 -> tableLog_tooLarge (44).
    for msv in [9u32, 10, 11, 12, 13, 20, 40] {
        if (1usize << 6) < msv as usize + 1 {
            continue;
        }
        let src = fse_weight_header(msv, 6);
        let got = diff(&format!("readStats weight msv={msv}"), |l| {
            read_stats(l, &src, 256, HUF_READ_STATS_WORKSPACE_SIZE)
        });
        if msv >= 12 {
            assert_eq!(
                code_of(&got.r),
                44,
                "weight alphabet of {} symbols must exceed the 876-byte wksp",
                msv + 1
            );
        }
    }
    // fse_decompress.c:258 / :273 through the *explicit* workspace of
    // HUF_readStats_wksp: below 512 bytes -> GENERIC (1), 512..879 -> 44.
    let src = fse_weight_header(11, 6);
    for ws in [0usize, 1, 8, 256, 511, 512, 600, 875, 876, 1024, 4096] {
        diff(&format!("readStats_wksp wkspSize={ws}"), |l| {
            let mut w = vec![0u8; 600];
            let mut r = vec![0u32; 16];
            let mut nb = 0u32;
            let mut tl = 0u32;
            let mut sw = wksp(ws);
            let n = unsafe {
                l.sym::<FnHufReadStatsWksp>("HUF_readStats_wksp")(
                    w.as_mut_ptr(),
                    256,
                    r.as_mut_ptr(),
                    &mut nb,
                    &mut tl,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    wp(&mut sw),
                    ws,
                    0,
                )
            };
            (res(l, n), Blob(w), u32s(&r), nb, tl)
        });
    }
    covers(&[
        "ERR:common/entropy_common.c:273",
        "ERR:common/fse_decompress.c:258",
        "ERR:common/fse_decompress.c:266",
        "ERR:common/fse_decompress.c:267",
        "ERR:common/fse_decompress.c:273",
    ]);
}

#[test]
fn huf_read_stats_fuzz_short_headers() {
    let mut rng = Rng::new(0x11_0000_0002);
    for i in 0..1500 {
        let n = 1 + rng.below(12);
        let mut src = rng.bytes(n);
        // Keep hwSize >= 1 (see the precondition on `read_stats`).
        if rng.below(3) == 0 {
            src[0] |= 0x80; // force the raw-nibble path
        } else {
            src[0] &= 0x7F; // force the FSE path
        }
        let hw = *rng.pick(&[1usize, 2, 4, 8, 13, 16, 32, 100, 256, 512]);
        diff(&format!("readStats fuzz #{i} n={n} hw={hw}"), |l| {
            read_stats(l, &src, hw, HUF_READ_STATS_WORKSPACE_SIZE)
        });
    }
    covers(&[
        "ERR:common/entropy_common.c:254",
        "ERR:common/entropy_common.c:261",
        "ERR:common/entropy_common.c:262",
        "ERR:common/entropy_common.c:270",
        "ERR:common/entropy_common.c:273",
        "ERR:common/entropy_common.c:280",
        "ERR:common/entropy_common.c:284",
        "ERR:common/entropy_common.c:288",
        "ERR:common/entropy_common.c:295",
        "ERR:common/entropy_common.c:301",
    ]);
}

// ===========================================================================
// common/fse_decompress.c + common/bitstream.h
//   rows 17 (:70), 18 (:71), 19 (:72), 20 (:146) — FSE_buildDTable_wksp
//   rows 21 (:188), 22 (:193), 23 (:220), 24 (:227) — the decode loops
//   rows 25 (:258), 26 (:266), 27 (:267), 28 (:273), 29 (:278) — the wksp body
//   rows 33 (bitstream.h:256), 34 (:266), 35 (:294), 37 (:415), 38 (:429),
//        39 (:435) — BIT_initDStream / BIT_reloadDStream
// ===========================================================================

#[test]
fn fse_build_dtable_wksp_rejects_bad_sizes_and_distributions() {
    let dt_cap = fse_dtable_size_u32(FSE_TABLELOG_ABSOLUTE_MAX);
    fn run<'a>(
        nc: &'a Vec<i16>,
        msv: u32,
        tl: u32,
        ws: usize,
        dt_cap: usize,
    ) -> impl Fn(&Lib) -> (R, Blob) + 'a {
        move |l: &Lib| {
            let mut dt = vec![0xA5A5_A5A5u32; dt_cap];
            let mut w = wksp(ws.max(1));
            let n = unsafe {
                l.sym::<FnFseBuildDTableWksp>("FSE_buildDTable_wksp")(
                    dt.as_mut_ptr(),
                    nc.as_ptr(),
                    msv,
                    tl,
                    wp(&mut w),
                    ws,
                )
            };
            (res(l, n), u32s(&dt))
        }
    }

    // Row 17 (:70): wkspSize one byte below FSE_BUILD_DTABLE_WKSP_SIZE.
    for (msv, tl) in [(255u32, 8u32), (255, 12), (0, 5), (31, 9), (52, 7)] {
        let need = fse_build_dtable_wksp_size(tl, msv);
        let nc = norm_uniform(msv, tl);
        for ws in [0usize, 1, need - 1, need, need + 1] {
            let got = diff_bytes(
                &format!("buildDTable msv={msv} tl={tl} wksp={ws} (need {need})"),
                run(&nc, msv, tl, ws, dt_cap),
            );
            if ws < need {
                assert_eq!(code_of(&got.0), 46, "row 17: maxSymbolValue_tooLarge");
            }
        }
    }
    // Row 18 (:71): maxSymbolValue > FSE_MAX_SYMBOL_VALUE, with a workspace big
    // enough to clear :70 first.  The counter is all-zero: :71 fires before it is
    // ever read, so no out-of-range access happens.
    for msv in [256u32, 257, 300, 1000] {
        let need = fse_build_dtable_wksp_size(6, msv);
        let nc = vec![0i16; 2048];
        let got = diff_bytes(
            &format!("buildDTable msv={msv} (oob)"),
            run(&nc, msv, 6, need + 64, dt_cap),
        );
        assert_eq!(code_of(&got.0), 46, "row 18: maxSymbolValue_tooLarge");
    }
    // Row 19 (:72): tableLog > FSE_MAX_TABLELOG (12).
    for tl in 13..=15u32 {
        let need = fse_build_dtable_wksp_size(tl, 255);
        let nc = vec![0i16; 2048];
        let got = diff_bytes(
            &format!("buildDTable tl={tl}"),
            run(&nc, 255, tl, need + 64, dt_cap),
        );
        assert_eq!(code_of(&got.0), 44, "row 19: tableLog_tooLarge");
    }
    // Row 20 (:146): the low-probability branch ends with `position != 0` because
    // the counts do not sum to 1<<tableLog. nc={-1,4} needs 32 cells but lays
    // down 1 lowprob + 4 -> GENERIC (1).
    {
        let mut nc = vec![0i16; 2048];
        nc[0] = -1;
        nc[1] = 4;
        let got = diff_bytes("buildDTable position!=0", run(&nc, 1, 5, 1024, dt_cap));
        assert_eq!(code_of(&got.0), 1, "row 20: GENERIC");
    }
    // and a randomised sweep of counter vectors that contain a -1 (so the
    // low-probability branch is taken) and generally do NOT sum to 1<<tableLog.
    let mut rng = Rng::new(0x11_0000_0003);
    let mut generic = 0usize;
    for i in 0..300 {
        let msv = rng.range(1, 40) as u32;
        let tl = rng.range(FSE_MIN_TABLELOG as i64, FSE_MAX_TABLELOG as i64) as u32;
        let mut nc = vec![0i16; 2048];
        nc[0] = -1;
        for s in 1..=msv as usize {
            nc[s] = rng.range(-1, 8) as i16;
        }
        // Only counters that make the spread loop end on `position != 0` are in
        // contract here (see `dtable_spread_fails`); the others would make the C
        // read uninitialised table bytes and write out of bounds.
        if dtable_spread_fails(&nc, msv, tl) != Some(true) {
            continue;
        }
        let got = diff_bytes(
            &format!("buildDTable rand #{i} msv={msv} tl={tl}"),
            run(&nc, msv, tl, fse_build_dtable_wksp_size(tl, msv) + 64, dt_cap),
        );
        assert_eq!(code_of(&got.0), 1, "predicted position!=0 must yield GENERIC");
        generic += 1;
    }
    assert!(generic > 0, "no random counter reached fse_decompress.c:146");
    eprintln!("fse_decompress.c:146 reached {generic} times");
    covers(&[
        "ERR:common/fse_decompress.c:70",
        "ERR:common/fse_decompress.c:71",
        "ERR:common/fse_decompress.c:72",
        "ERR:common/fse_decompress.c:146",
    ]);
}

/// A complete FSE frame (NCount header ++ bitstream), produced by the C library.
struct FseFix {
    frame: Vec<u8>,
    hdr_len: usize,
    table_log: u32,
    msv: u32,
}

fn c_fse_frame(src: &[u8], table_log_req: u32) -> FseFix {
    assert!(src.len() > 1, "FSE_minTableLog asserts srcSize > 1");
    let l = &pair().c;
    let mut count = vec![0u32; 256];
    let mut msv = FSE_MAX_SYMBOL_VALUE;
    unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let tl = if table_log_req == 0 {
        unsafe { l.sym::<FnFseOptimalTableLog>("FSE_optimalTableLog")(0, src.len(), msv) }
    } else {
        table_log_req
    };
    let mut norm = vec![0i16; 1024];
    let nn = unsafe {
        l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
            norm.as_mut_ptr(),
            tl,
            count.as_ptr(),
            src.len(),
            msv,
            0,
        )
    };
    let etl = match res(l, nn) {
        R::Ok(v) if v >= FSE_MIN_TABLELOG as usize => v as u32,
        other => panic!("fixture FSE_normalizeCount gave {other:?} (RLE or error)"),
    };
    let mut hdr = vec![0u8; 1024];
    let hn = unsafe {
        l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
            hdr.as_mut_ptr() as *mut c_void,
            hdr.len(),
            norm.as_ptr(),
            msv,
            etl,
        )
    };
    let hdr_len = match res(l, hn) {
        R::Ok(v) => v,
        e => panic!("fixture FSE_writeNCount failed: {e:?}"),
    };
    let mut ct = vec![0u32; fse_ctable_size_u32(etl, msv)];
    let mut cw = wksp(1 << 17);
    let bc = unsafe {
        l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
            ct.as_mut_ptr(),
            norm.as_ptr(),
            msv,
            etl,
            wp(&mut cw),
            (1usize << 17) as SizeT,
        )
    };
    assert!(matches!(res(l, bc), R::Ok(_)), "fixture buildCTable failed");
    let cbound = unsafe { l.sym::<FnSzSz>("FSE_compressBound")(src.len()) };
    let mut body = vec![0u8; cbound + 64];
    let cn = unsafe {
        l.sym::<FnFseCompressUsingCTable>("FSE_compress_usingCTable")(
            body.as_mut_ptr() as *mut c_void,
            body.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            ct.as_ptr(),
        )
    };
    let body_len = match res(l, cn) {
        R::Ok(v) => v,
        e => panic!("fixture FSE_compress_usingCTable failed: {e:?}"),
    };
    assert!(body_len > 0, "fixture bitstream did not fit");
    let mut frame = hdr[..hdr_len].to_vec();
    frame.extend_from_slice(&body[..body_len]);
    FseFix {
        frame,
        hdr_len,
        table_log: etl,
        msv,
    }
}

/// `FSE_decompress_wksp_bmi2` with every observable compared: return value and
/// the *whole* destination buffer including bytes the callee never wrote.
fn fse_decompress(
    l: &Lib,
    frame: &[u8],
    c_src_size: usize,
    dst_cap: usize,
    dst_alloc: usize,
    max_log: u32,
    wksp_size: usize,
) -> (R, Blob) {
    let mut dst = vec![0xBBu8; dst_alloc.max(1)];
    let mut w = wksp(wksp_size.max(1));
    let mut out = 0usize;
    for bmi2 in [0i32, 1] {
        for b in dst.iter_mut() {
            *b = 0xBB;
        }
        let n = unsafe {
            l.sym::<FnFseDecompressWkspBmi2>("FSE_decompress_wksp_bmi2")(
                dst.as_mut_ptr() as *mut c_void,
                dst_cap,
                frame.as_ptr() as *const c_void,
                c_src_size,
                max_log,
                wp(&mut w),
                wksp_size,
                bmi2,
            )
        };
        if bmi2 == 0 {
            out = n;
        } else {
            assert_eq!(n, out, "[{}] decompress_wksp bmi2 mismatch", l.tag);
        }
    }
    (res(l, out), Blob(dst))
}

#[test]
fn fse_decompress_wksp_rejects_bad_sizes_and_truncation() {
    let src = small_alphabet_src(4096, 16, 0x11_0004);
    let fix = c_fse_frame(&src, 6);
    assert_eq!((fix.table_log, fix.msv), (6, 15), "fixture shape");
    let need = fse_decompress_wksp_size(fix.table_log, fix.msv);
    let big = need + 4096;

    // Row 25 (:258): wkspSize < sizeof(FSE_DecompressWksp) == 512 -> GENERIC (1).
    for ws in [0usize, 1, 8, 256, 511] {
        let got = diff_bytes(&format!("decompress wksp={ws}"), |l| {
            fse_decompress(l, &fix.frame, fix.frame.len(), src.len(), src.len() + 64, 12, ws)
        });
        assert_eq!(code_of(&got.0), 1, "row 25: GENERIC at wkspSize={ws}");
    }
    // Row 28 (:273): clears :258 but below FSE_DECOMPRESS_WKSP_SIZE -> 44.
    for ws in [512usize, 600, 1024, need - 1, need, big] {
        let got = diff_bytes(&format!("decompress wksp={ws} (need {need})"), |l| {
            fse_decompress(l, &fix.frame, fix.frame.len(), src.len(), src.len() + 64, 12, ws)
        });
        if ws < need {
            assert_eq!(code_of(&got.0), 44, "row 28: tableLog_tooLarge at {ws}");
        } else {
            assert_eq!(got.0, R::Ok(src.len()), "valid decode at wkspSize={ws}");
        }
    }
    // Row 26 (:266): the inner FSE_readNCount fails and its code is forwarded.
    {
        let mut bad = fix.frame.clone();
        bad[0] = (bad[0] & 0xF0) | 0x0B; // nibble 11 -> nbBits 16 > 15
        let got = diff_bytes("decompress inner readNCount", |l| {
            fse_decompress(l, &bad, bad.len(), src.len(), src.len() + 64, 12, big)
        });
        assert_eq!(code_of(&got.0), 44, "row 26");
    }
    // Row 27 (:267): the header-declared tableLog exceeds the caller's maxLog.
    for max_log in 0..=FSE_MAX_TABLELOG {
        let got = diff_bytes(&format!("decompress maxLog={max_log}"), |l| {
            fse_decompress(
                l,
                &fix.frame,
                fix.frame.len(),
                src.len(),
                src.len() + 64,
                max_log,
                big,
            )
        });
        if max_log < fix.table_log {
            assert_eq!(code_of(&got.0), 44, "row 27 at maxLog={max_log}");
        }
    }
    // Row 29 (:278): FSE_buildDTable_internal fails and CHECK_F forwards it.
    // A hand-built NCount header declaring tableLog 13 (low nibble 8) with a
    // single symbol taking all 8192 states: FSE_readNCount accepts it, :267
    // accepts it when maxLog >= 13, :273 is cleared by a large workspace, and
    // then fse_decompress.c:72 rejects tableLog 13 > FSE_MAX_TABLELOG.
    {
        // bitStream low 4 bits = 8 (tableLog 5+8 = 13), next 14 bits all ones,
        // which decodes to raw count 16383-8190 = 8193 -> norm[0] = 8192.
        let hdr = [0xF8u8, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00];
        let probe = diff("decompress tl13 header parse", |l| {
            read_ncount(l, &hdr, 8, 255)
        });
        assert_eq!(probe.r, R::Ok(3), "hand-built tableLog-13 header must parse");
        assert_eq!(probe.table_log, 13);
        assert_eq!(probe.msv, 0);
        for max_log in [12u32, 13, 15] {
            let got = diff_bytes(&format!("decompress tl13 maxLog={max_log}"), |l| {
                fse_decompress(l, &hdr, 8, 64, 128, max_log, 1 << 17)
            });
            assert_eq!(
                code_of(&got.0),
                44,
                "row 29/19: tableLog 13 must be rejected (maxLog={max_log})"
            );
        }
    }
    covers(&[
        "ERR:common/fse_decompress.c:258",
        "ERR:common/fse_decompress.c:266",
        "ERR:common/fse_decompress.c:267",
        "ERR:common/fse_decompress.c:273",
        "ERR:common/fse_decompress.c:278",
        "ERR:common/fse_decompress.c:72",
    ]);
}

#[test]
fn fse_decompress_dst_and_bitstream_limits() {
    let src = small_alphabet_src(512, 16, 0x11_0005);
    let fix = c_fse_frame(&src, 6);
    let big = fse_decompress_wksp_size(FSE_MAX_TABLELOG, 255) + 4096;

    // Rows 23 (:220) and 24 (:227): the tail loop refuses to emit past
    // `omax-2`; dstCapacity 0 fails on state1, 1 on state2.
    for cap in [0usize, 1, 2, 3, 4, 7, 8, src.len() - 1, src.len()] {
        let got = diff_bytes(&format!("decompress dstCapacity={cap}"), |l| {
            fse_decompress(l, &fix.frame, fix.frame.len(), cap, src.len() + 64, 12, big)
        });
        if cap < src.len() {
            assert_eq!(code_of(&got.0), 70, "row 23/24: dstSize_tooSmall at {cap}");
        } else {
            assert_eq!(got.0, R::Ok(src.len()));
        }
    }

    // Rows 21 (:188) / 33 (bitstream.h:256): the NCount header consumes all of
    // `cSrcSize`, so BIT_initDStream sees srcSize == 0 -> srcSize_wrong (72).
    let got = diff_bytes("decompress empty body", |l| {
        fse_decompress(l, &fix.frame, fix.hdr_len, src.len(), src.len() + 64, 12, big)
    });
    assert_eq!(code_of(&got.0), 72, "row 21/33: srcSize_wrong");

    // Row 35 (bitstream.h:294): a 1..7-byte body whose last byte is 0.
    for body in 1..=7usize {
        let mut f = fix.frame[..fix.hdr_len + body].to_vec();
        *f.last_mut().unwrap() = 0;
        let got = diff_bytes(&format!("decompress short body={body} lastByte=0"), |l| {
            fse_decompress(l, &f, f.len(), src.len(), src.len() + 64, 12, big)
        });
        assert_eq!(code_of(&got.0), 20, "row 35: corruption_detected");
    }
    // Row 34 (bitstream.h:266): a >= 8-byte body whose last byte is 0 -> GENERIC.
    for body in [8usize, 9, 12, 16, 32] {
        if fix.hdr_len + body > fix.frame.len() {
            continue;
        }
        let mut f = fix.frame[..fix.hdr_len + body].to_vec();
        *f.last_mut().unwrap() = 0;
        let got = diff_bytes(&format!("decompress body={body} lastByte=0"), |l| {
            fse_decompress(l, &f, f.len(), src.len(), src.len() + 64, 12, big)
        });
        assert_eq!(code_of(&got.0), 1, "row 34: GENERIC (no endMark)");
    }
    // Row 22 (:193) / 37 (bitstream.h:415): the body is shorter than the two
    // initial states (2*tableLog bits) -> BIT_DStream_overflow -> 20.
    let mut overflow_hits = 0usize;
    for body in 1..=9usize {
        for last in [0x01u8, 0x02, 0x40, 0x80, 0xFF] {
            let mut f = fix.frame[..fix.hdr_len + body].to_vec();
            *f.last_mut().unwrap() = last; // a valid endMark, but too few bits
            let got = diff_bytes(
                &format!("decompress states body={body} last={last:02x}"),
                |l| fse_decompress(l, &f, f.len(), src.len(), src.len() + 64, 12, big),
            );
            if is_err_code(&got.0, 20) {
                overflow_hits += 1;
            }
        }
    }
    assert!(
        overflow_hits > 0,
        "no short body reached fse_decompress.c:193 / bitstream.h:415"
    );
    eprintln!("fse_decompress.c:193 reached {overflow_hits} times");
    // Rows 38 (bitstream.h:429) and 39 (:435): every truncation length, so the
    // 1..8-byte end-of-buffer path and the 9..15-byte cautious-refill path are
    // both walked to exhaustion.
    for n in 0..=fix.frame.len().min(fix.hdr_len + 40) {
        diff_bytes(&format!("decompress truncated cSrcSize={n}"), |l| {
            fse_decompress(l, &fix.frame, n, src.len(), src.len() + 64, 12, big)
        });
    }
    covers(&[
        "ERR:common/fse_decompress.c:188",
        "ERR:common/fse_decompress.c:193",
        "ERR:common/fse_decompress.c:220",
        "ERR:common/fse_decompress.c:227",
        "ERR:common/bitstream.h:256",
        "ERR:common/bitstream.h:266",
        "ERR:common/bitstream.h:294",
        "ERR:common/bitstream.h:415",
        "ERR:common/bitstream.h:429",
        "ERR:common/bitstream.h:435",
    ]);
}

#[test]
fn fse_decompress_wksp_fuzz_short_inputs() {
    let mut rng = Rng::new(0x11_0000_0006);
    let big = fse_decompress_wksp_size(FSE_MAX_TABLELOG, 255) + 4096;
    for i in 0..900 {
        let n = rng.below(24);
        let mut buf = rng.bytes(n.max(1));
        buf.truncate(n);
        let cap = *rng.pick(&[0usize, 1, 2, 8, 64, 256]);
        let max_log = rng.range(0, FSE_TABLELOG_ABSOLUTE_MAX as i64) as u32;
        let ws = *rng.pick(&[0usize, 511, 512, 1024, big]);
        diff_bytes(
            &format!("decompress fuzz #{i} n={n} cap={cap} maxLog={max_log} ws={ws}"),
            |l| fse_decompress(l, &buf, n, cap, 512, max_log, ws),
        );
    }
    covers(&[
        "ERR:common/fse_decompress.c:258",
        "ERR:common/fse_decompress.c:266",
        "ERR:common/fse_decompress.c:267",
        "ERR:common/fse_decompress.c:273",
        "ERR:common/bitstream.h:256",
    ]);
}

// ===========================================================================
// compress/fse_compress.c + the compression half of common/bitstream.h
//   rows 301 (:87), 302 (:269), 303 (:284), 304 (:301), 305 (:306),
//        306 (:315), 307 (:320), 308 (:333), 309 (:334), 310 (:457),
//        311 (:471), 312 (:472), 313 (:473), 314 (:487), 315 (:563),
//        316 (:565), 317 (:607)
//   rows 30 (bitstream.h:158), 31 (:228), 32 (:240)
// ===========================================================================

#[test]
fn fse_build_ctable_wksp_rejects_small_workspace() {
    // Row 301 (:87): wkspSize < FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue,
    // tableLog) -> tableLog_tooLarge (44). For (255, 12) the bound is 8712.
    for (msv, tl) in [(255u32, 12u32), (255, 8), (0, 5), (31, 9), (52, 7), (12, 6)] {
        let need = fse_build_ctable_wksp_size(msv, tl);
        let nc = norm_uniform(msv, tl);
        for ws in [0usize, 1, 8, need - 1, need, need + 8] {
            let got = diff_bytes(
                &format!("buildCTable msv={msv} tl={tl} wksp={ws} (need {need})"),
                |l| {
                    let mut ct = vec![0x5A5A_5A5Au32; fse_ctable_size_u32(tl, msv) + 8];
                    let mut w = wksp(ws.max(1));
                    let n = unsafe {
                        l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
                            ct.as_mut_ptr(),
                            nc.as_ptr(),
                            msv,
                            tl,
                            wp(&mut w),
                            ws,
                        )
                    };
                    (res(l, n), u32s(&ct))
                },
            );
            if ws < need {
                assert_eq!(code_of(&got.0), 44, "row 301: tableLog_tooLarge");
            } else {
                assert_eq!(got.0, R::Ok(0), "valid buildCTable at wkspSize={ws}");
            }
        }
    }
    // `FSE_buildCTable_wksp` has NO `maxSymbolValue > FSE_MAX_SYMBOL_VALUE` check
    // (unlike `FSE_buildDTable_wksp`, fse_decompress.c:71) — only the workspace
    // bound at :87, which scales with maxSymbolValue. Confirm both libraries
    // agree on that absence for a 257-symbol alphabet with a *valid* 512-state
    // distribution (256 symbols of weight 2, one of weight 0).
    {
        let msv = 256u32;
        let tl = 9u32;
        let mut nc = vec![0i16; 2048];
        for s in 0..256 {
            nc[s] = 2;
        }
        let need = fse_build_ctable_wksp_size(msv, tl);
        diff_bytes("buildCTable msv=256 valid", |l| {
            let mut ct = vec![0x5A5A_5A5Au32; fse_ctable_size_u32(tl, msv) + 8];
            let mut w = wksp(need + 64);
            let n = unsafe {
                l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
                    ct.as_mut_ptr(),
                    nc.as_ptr(),
                    msv,
                    tl,
                    wp(&mut w),
                    need + 64,
                )
            };
            (res(l, n), u32s(&ct))
        });
    }
    covers(&["ERR:compress/fse_compress.c:87"]);
}

/// `FSE_writeNCount` with the whole destination buffer compared. The buffer is
/// always allocated far larger than `buffer_size`, so if either library were to
/// write past the declared capacity the extra bytes would show up in the diff
/// rather than corrupting the heap.
fn write_ncount(l: &Lib, nc: &[i16], msv: u32, tl: u32, buffer_size: usize) -> (R, SizeT, Blob) {
    let mut buf = vec![0xEEu8; buffer_size + 64];
    let bound = unsafe { l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, tl) };
    let n = unsafe {
        l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
            buf.as_mut_ptr() as *mut c_void,
            buffer_size,
            nc.as_ptr(),
            msv,
            tl,
        )
    };
    (res(l, n), bound, Blob(buf))
}

#[test]
fn fse_write_ncount_rejects_bad_tablelog_and_small_buffers() {
    // Rows 308 (:333) and 309 (:334): tableLog out of [5,12]. Note 0 is NOT
    // remapped here (unlike FSE_normalizeCount) -> GENERIC.
    let nc = norm_uniform(255, 12);
    for tl in 0..=FSE_TABLELOG_ABSOLUTE_MAX + 3 {
        let got = diff_bytes(&format!("writeNCount tableLog={tl}"), |l| {
            write_ncount(l, &nc, 255, tl, 4096)
        });
        if tl > FSE_MAX_TABLELOG {
            assert_eq!(code_of(&got.0), 44, "row 308 at tableLog={tl}");
        } else if tl < FSE_MIN_TABLELOG {
            assert_eq!(code_of(&got.0), 1, "row 309 at tableLog={tl}");
        }
    }
    // Rows 302 (:269), 303 (:284), 305 (:306), 307 (:320): every
    // `!writeIsSafe && out > oend-2` flush site. Each norm shape below routes
    // through a different one (a >= 24-symbol zero run, a 3..23 zero run, the
    // per-symbol flush, and the final flush), and every bufferSize from 0 to the
    // bound is tried so all four are hit.
    let mut zero_run_24 = norm_uniform(0, 5); // norm[0] = 32
    zero_run_24[0] = 31;
    zero_run_24[1] = 1; // then 254 zeros -> the run-of-24 loop
    let mut zero_run_small = vec![0i16; 1024];
    zero_run_small[0] = 28;
    zero_run_small[1] = 1;
    zero_run_small[5] = 1;
    zero_run_small[6] = 1;
    zero_run_small[7] = 1; // 3-symbol gaps
    let shapes: [(&str, Vec<i16>, u32, u32); 4] = [
        ("run24", zero_run_24, 255, 5),
        ("run3", zero_run_small, 7, 5),
        ("uniform255x12", norm_uniform(255, 12), 255, 12),
        ("two", {
            let mut v = vec![0i16; 1024];
            v[0] = 31;
            v[1] = 1;
            v
        }, 1, 5),
    ];
    let mut dst_too_small = 0usize;
    for (name, nc, msv, tl) in shapes.iter() {
        let bound = {
            let l = &pair().c;
            unsafe { l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(*msv, *tl) }
        };
        for bs in 0..=bound + 2 {
            let got = diff_bytes(&format!("writeNCount {name} bufferSize={bs}"), |l| {
                write_ncount(l, nc, *msv, *tl, bs)
            });
            if is_err_code(&got.0, 70) {
                dst_too_small += 1;
            }
        }
    }
    assert!(dst_too_small > 0, "no bufferSize reached a dstSize_tooSmall flush guard");
    eprintln!("FSE_writeNCount dstSize_tooSmall reached {dst_too_small} times");

    // Row 304 (:301): sum(abs(norm)) overshoots 1<<tableLog -> GENERIC.
    {
        let mut nc = vec![0i16; 1024];
        nc[0] = 40; // 40 > 32
        let got = diff_bytes("writeNCount overshoot", |l| write_ncount(l, &nc, 1, 5, 512));
        assert_eq!(code_of(&got.0), 1, "row 304: GENERIC");
    }
    // Row 306 (:315): undershoot, including the all-zero counter.
    {
        let nc = vec![0i16; 1024];
        let got = diff_bytes("writeNCount all-zero", |l| write_ncount(l, &nc, 3, 5, 512));
        assert_eq!(code_of(&got.0), 1, "row 306: GENERIC");
        let mut nc2 = vec![0i16; 1024];
        nc2[0] = 8;
        nc2[1] = 8;
        let got = diff_bytes("writeNCount undershoot", |l| write_ncount(l, &nc2, 3, 5, 512));
        assert_eq!(code_of(&got.0), 1, "row 306: GENERIC (undershoot)");
    }
    // `FSE_writeNCount` has NO maxSymbolValue check either. Values above 255 just
    // read further into `normalizedCounter` — which is why every counter in this
    // file is allocated with 1024 entries. Compare the (identical) outcome.
    for msv in [255u32, 256, 300, 511] {
        let mut nc = vec![0i16; 1024];
        nc[0] = 32;
        diff_bytes(&format!("writeNCount msv={msv}"), |l| {
            write_ncount(l, &nc, msv, 5, 4096)
        });
        diff(&format!("NCountWriteBound msv={msv}"), |l| unsafe {
            (
                l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, 5),
                l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, 12),
                l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, 0),
            )
        });
    }
    covers(&[
        "ERR:compress/fse_compress.c:269",
        "ERR:compress/fse_compress.c:284",
        "ERR:compress/fse_compress.c:301",
        "ERR:compress/fse_compress.c:306",
        "ERR:compress/fse_compress.c:315",
        "ERR:compress/fse_compress.c:320",
        "ERR:compress/fse_compress.c:333",
        "ERR:compress/fse_compress.c:334",
    ]);
}

#[test]
fn fse_normalize_count_rejects_bad_tablelog() {
    // CRITICAL PRECONDITION, documented in tests/t10_entropy.rs: `total == 0`
    // makes `FSE_normalizeCount` compute `ZSTD_div64((U64)1<<62, (U32)0)` at
    // fse_compress.c:479 and the reference C build takes SIGFPE (and
    // `FSE_minTableLog` additionally calls `ZSTD_highbit32(0)`, whose
    // `__builtin_clz(0)` is undefined). `total == 0` is therefore OUT OF
    // CONTRACT and is never passed below; every case uses `total >= 2`.
    let mut count = vec![0u32; 1024];
    count[0] = 600;
    count[1] = 300;
    count[2] = 100;
    let total = 1000usize;
    let msv = 2u32;
    let min_tl = fse_min_table_log(total, msv);
    for tl in 0..=FSE_TABLELOG_ABSOLUTE_MAX + 3 {
        let got = diff_bytes(&format!("normalizeCount tl={tl}"), |l| {
            let mut norm = vec![0i16; 1024];
            let n = unsafe {
                l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
                    norm.as_mut_ptr(),
                    tl,
                    count.as_ptr(),
                    total,
                    msv,
                    0,
                )
            };
            (res(l, n), i16s(&norm))
        });
        // tableLog == 0 is remapped to FSE_DEFAULT_TABLELOG (11) at :470.
        let eff = if tl == 0 { 11 } else { tl };
        if eff > FSE_MAX_TABLELOG {
            assert_eq!(code_of(&got.0), 44, "row 312 at tableLog={tl}");
        } else if eff < FSE_MIN_TABLELOG {
            assert_eq!(code_of(&got.0), 1, "row 311 at tableLog={tl}");
        } else if eff < min_tl {
            assert_eq!(code_of(&got.0), 1, "row 313 at tableLog={tl}");
        }
    }
    // Row 313 (:473): tableLog < FSE_minTableLog(total, maxSymbolValue).
    // total = 1_000_000, msv = 255 -> minTableLog = MIN(20, 9) = 9.
    {
        let mut count = vec![0u32; 1024];
        for s in 0..=255usize {
            count[s] = 3906;
        }
        count[0] += 1_000_000 - 3906 * 256;
        let total = 1_000_000usize;
        assert_eq!(fse_min_table_log(total, 255), 9);
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            let got = diff_bytes(&format!("normalizeCount minTableLog tl={tl}"), |l| {
                let mut norm = vec![0i16; 1024];
                let n = unsafe {
                    l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
                        norm.as_mut_ptr(),
                        tl,
                        count.as_ptr(),
                        total,
                        255,
                        0,
                    )
                };
                (res(l, n), i16s(&norm))
            });
            if tl < 9 {
                assert_eq!(code_of(&got.0), 1, "row 313 at tableLog={tl}");
            }
        }
    }
    // Row 314 (:487): count[s] == total for some s -> returns 0, meaning "RLE
    // special case, no normalized table" — NOT an error.
    for (s, msv) in [(0usize, 3u32), (1, 3), (7, 15), (255, 255), (0, 255)] {
        let mut count = vec![0u32; 1024];
        count[s] = 100;
        // tableLog must clear the :473 guard first, so pick the smallest legal one.
        let tl = fse_min_table_log(100, msv).max(FSE_MIN_TABLELOG);
        let got = diff_bytes(&format!("normalizeCount rle s={s} msv={msv} tl={tl}"), |l| {
            let mut norm = vec![0i16; 1024];
            let n = unsafe {
                l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
                    norm.as_mut_ptr(),
                    tl,
                    count.as_ptr(),
                    100,
                    msv,
                    0,
                )
            };
            (res(l, n), i16s(&norm))
        });
        assert_eq!(got.0, R::Ok(0), "row 314: RLE special case returns 0");
    }
    covers(&[
        "ERR:compress/fse_compress.c:471",
        "ERR:compress/fse_compress.c:472",
        "ERR:compress/fse_compress.c:473",
        "ERR:compress/fse_compress.c:487",
    ]);
}

#[test]
fn fse_normalize_m2_secondary_normalization_failure() {
    // Row 310 (:457): inside `FSE_normalizeM2` a NOT_YET_ASSIGNED symbol receives
    // `weight = sEnd - sStart < 1` -> GENERIC. This is INDIRECT: it requires the
    // primary normalization to overshoot first (`-stillToDistribute >=
    // norm[largest]>>1` at :502).
    //
    // Because `FSE_normalizeCount` also returns GENERIC from :471 and :473, the
    // search below only accepts hits where `tableLog` is inside [5,12] AND
    // >= FSE_minTableLog(total, maxSymbolValue) — then GENERIC can only come from
    // :457. The C library alone is used to *find* candidates (cheap); every hit
    // is then compared across both libraries.
    let c = &pair().c;
    let f = c.sym::<FnFseNormalizeCount>("FSE_normalizeCount");
    let mut rng = Rng::new(0x11_0000_0310);
    let mut hits: Vec<(Vec<u32>, usize, u32, u32, u32)> = Vec::new();
    for _ in 0..250000 {
        if hits.len() >= 12 {
            break;
        }
        let msv = rng.range(2, 255) as u32;
        let mut count = vec![0u32; 1024];
        let mut total = 0usize;
        // Heavily skewed: one dominant symbol plus a long tail just above the
        // low-probability threshold, which is what forces the M2 path.
        let shape = rng.below(3);
        for s in 0..=msv as usize {
            let v = match shape {
                0 => (rng.below(3) + 1) as u32,
                1 => {
                    if s == 0 {
                        rng.below(50000) as u32 + 1000
                    } else {
                        (rng.below(20) + 1) as u32
                    }
                }
                _ => {
                    let k = 1 + rng.below(12);
                    (rng.below(1 << k) + 1) as u32
                }
            };
            count[s] = v;
            total += v as usize;
        }
        if total < 2 {
            continue;
        }
        let min_tl = fse_min_table_log(total, msv);
        let lo = min_tl.max(FSE_MIN_TABLELOG);
        if lo > FSE_MAX_TABLELOG {
            continue;
        }
        for tl in lo..=FSE_MAX_TABLELOG {
            for ulp in [0u32, 1] {
                let mut norm = vec![0i16; 1024];
                let n = unsafe { f(norm.as_mut_ptr(), tl, count.as_ptr(), total, msv, ulp) };
                if is_err_code(&res(c, n), 1) {
                    hits.push((count.clone(), total, msv, tl, ulp));
                    break;
                }
            }
        }
    }
    // `fse_compress.c:457` was NOT reached by any of the 250 000 adversarial
    // histograms above, and the arithmetic makes it very hard to reach: a symbol
    // is still `NOT_YET_ASSIGNED` at :453 only when `count[s] > lowOne`, and
    // `lowOne` is essentially the threshold that makes
    // `count[s] * rStep >= 1 << vStepLog` (so `weight >= 1`):
    //   * if the re-scaling branch at :414 was NOT taken then
    //     `total/ToDistribute <= lowOne`, so `count[s] > lowOne >=
    //     total/ToDistribute`, i.e. `count[s]*ToDistribute > total` — weight >= 1
    //     unconditionally;
    //   * if it WAS taken, `lowOne` is re-set to `3*total/(2*ToDistribute)` and
    //     the survivors satisfy `count[s]*ToDistribute > 1.5*total`, which still
    //     leaves weight >= 1 for every ratio the second pass can produce unless
    //     it re-classifies more than a third of the remaining budget — a window
    //     the search above did not manage to enter;
    //   * and `total` is *decremented* for every symbol classified in the first
    //     loop, while `rStep` additionally carries a `+mid` upward bias.
    // See the report: row 310 is reported as not-reached rather than covered.
    assert!(
        hits.is_empty(),
        "fse_compress.c:457 became reachable ({} histograms) -- revisit the analysis",
        hits.len()
    );
    eprintln!("fse_compress.c:457 unreachable as analysed (250000 histograms tried)");
    for (i, (count, total, msv, tl, ulp)) in hits.iter().enumerate() {
        let got = diff_bytes(
            &format!("normalizeM2 #{i} msv={msv} total={total} tl={tl} ulp={ulp}"),
            |l| {
                let mut norm = vec![0i16; 1024];
                let n = unsafe {
                    l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
                        norm.as_mut_ptr(),
                        *tl,
                        count.as_ptr(),
                        *total,
                        *msv,
                        *ulp,
                    )
                };
                (res(l, n), i16s(&norm))
            },
        );
        assert_eq!(code_of(&got.0), 1, "row 310: GENERIC from FSE_normalizeM2");
    }
    // deliberately NOT tagged: the site was never executed.
}

/// A valid `FSE_CTable` built by the C library from `src`'s own histogram.
fn c_fse_ctable(src: &[u8], table_log: u32) -> (Vec<u32>, u32, u32) {
    let l = &pair().c;
    let mut count = vec![0u32; 256];
    let mut msv = FSE_MAX_SYMBOL_VALUE;
    unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let mut norm = vec![0i16; 1024];
    let nn = unsafe {
        l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
            norm.as_mut_ptr(),
            table_log,
            count.as_ptr(),
            src.len(),
            msv,
            0,
        )
    };
    let etl = match res(l, nn) {
        R::Ok(v) if v >= FSE_MIN_TABLELOG as usize => v as u32,
        other => panic!("fixture normalizeCount gave {other:?}"),
    };
    let mut ct = vec![0u32; fse_ctable_size_u32(etl, msv) + 8];
    let mut w = wksp(1 << 17);
    let bc = unsafe {
        l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
            ct.as_mut_ptr(),
            norm.as_ptr(),
            msv,
            etl,
            wp(&mut w),
            (1usize << 17) as SizeT,
        )
    };
    assert!(matches!(res(l, bc), R::Ok(_)), "fixture buildCTable failed");
    (ct, msv, etl)
}

#[test]
fn fse_compress_using_ctable_returns_zero_when_it_cannot_fit() {
    let src = small_alphabet_src(4096, 16, 0x11_0317);
    let (ct, _msv, _tl) = c_fse_ctable(&src, 6);
    let bound = {
        let l = &pair().c;
        unsafe { l.sym::<FnSzSz>("FSE_compressBound")(src.len()) }
    };

    fn run<'a>(
        ct: &'a [u32],
        src: &'a [u8],
        n: usize,
        dst_size: usize,
    ) -> impl Fn(&Lib) -> (R, Blob) + 'a {
        move |l: &Lib| {
            let mut dst = vec![0xDDu8; dst_size + 64];
            let r = unsafe {
                l.sym::<FnFseCompressUsingCTable>("FSE_compress_usingCTable")(
                    dst.as_mut_ptr() as *mut c_void,
                    dst_size,
                    src.as_ptr() as *const c_void,
                    n,
                    ct.as_ptr(),
                )
            };
            (res(l, r), Blob(dst))
        }
    }

    // Row 315 (:563): srcSize <= 2 -> 0 (not an error, "not compressible").
    for n in [0usize, 1, 2, 3] {
        let got = diff_bytes(&format!("compress_usingCTable srcSize={n}"), run(&ct, &src, n, 512));
        if n <= 2 {
            assert_eq!(got.0, R::Ok(0), "row 315: srcSize {n} -> 0");
        }
    }
    // Rows 316 (:565) and 30 (bitstream.h:158): BIT_initCStream fails when
    // dstSize <= sizeof(size_t) == 8, and FSE_compress_usingCTable turns the
    // dstSize_tooSmall into a plain 0.
    for dst_size in 0..=9usize {
        let got = diff_bytes(
            &format!("compress_usingCTable dstSize={dst_size}"),
            run(&ct, &src, src.len(), dst_size),
        );
        if dst_size <= 8 {
            assert_eq!(got.0, R::Ok(0), "row 316/30: dstSize {dst_size} -> 0");
        }
    }
    // Rows 317 (:607), 31 (bitstream.h:228) and 32 (:240): the bitstream
    // overflows dstSize mid-encoding, BIT_flushBits clamps the write pointer to
    // endPtr and BIT_closeCStream then reports 0.
    let mut zero_returns = 0usize;
    for dst_size in 9..=(bound + 8) {
        let got = diff_bytes(
            &format!("compress_usingCTable overflow dstSize={dst_size}"),
            run(&ct, &src, src.len(), dst_size),
        );
        if got.0 == R::Ok(0) {
            zero_returns += 1;
        }
    }
    assert!(
        zero_returns > 0,
        "no dstSize between 9 and the bound made BIT_closeCStream return 0"
    );
    eprintln!("BIT_closeCStream returned 0 for {zero_returns} capacities");
    covers(&[
        "ERR:compress/fse_compress.c:563",
        "ERR:compress/fse_compress.c:565",
        "ERR:compress/fse_compress.c:607",
        "ERR:common/bitstream.h:158",
        "ERR:common/bitstream.h:228",
        "ERR:common/bitstream.h:240",
    ]);
}

// ===========================================================================
// compress/hist.c
//   rows 318 (:48 srcSize==0), 319 (:52 unchecked byte > maxSymbolValue),
//        320 (:138 maxSymbolValue_tooSmall), 321 (:156 misaligned wksp),
//        322 (:157 workSpaceSize too small), 323 (:154 sourceSize < 1500
//        bypasses both checks), 324 (:168), 325 (:169)
// ===========================================================================

/// Drive `HIST_count_wksp` with a workspace whose alignment and size are chosen
/// by the caller. `misalign` shifts the base pointer by 1 byte inside a larger
/// allocation, so the C never touches memory outside the buffer.
fn hist_count_wksp(
    l: &Lib,
    name: &str,
    src: &[u8],
    msv_in: u32,
    wksp_size: usize,
    misalign: usize,
) -> (R, u32, Blob) {
    let mut count = vec![0u32; 256];
    let mut msv = msv_in;
    let mut w = vec![0u8; wksp_size + 64];
    let base = unsafe { w.as_mut_ptr().add(misalign) };
    let n = unsafe {
        l.sym::<FnHistCountWksp>(name)(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
            base as *mut c_void,
            wksp_size,
        )
    };
    (res(l, n), msv, u32s(&count))
}

#[test]
fn hist_count_wksp_rejects_alignment_and_size() {
    let small = small_alphabet_src(10, 4, 0x11_0318);
    let big = small_alphabet_src(2000, 4, 0x11_0319);
    assert!(big.len() >= 1500, "row 321/322 need sourceSize >= 1500");

    // Rows 324 (:168) and 325 (:169): HIST_count_wksp checks alignment and size
    // UNCONDITIONALLY, for every sourceSize.
    for src in [&small, &big] {
        for misalign in [0usize, 1, 2, 3, 4, 8] {
            for ws in [0usize, 1, 4095, 4096, 8192] {
                let got = diff_bytes(
                    &format!(
                        "HIST_count_wksp n={} misalign={misalign} ws={ws}",
                        src.len()
                    ),
                    |l| hist_count_wksp(l, "HIST_count_wksp", src, 255, ws, misalign),
                );
                if misalign % 4 != 0 {
                    assert_eq!(code_of(&got.0), 1, "row 324: GENERIC (misaligned)");
                } else if ws < HIST_WKSP_SIZE {
                    assert_eq!(code_of(&got.0), 66, "row 325: workSpace_tooSmall");
                } else {
                    assert_eq!(code_of(&got.0), 0, "valid HIST_count_wksp");
                }
            }
        }
    }
    // Rows 321 (:156), 322 (:157) and 323 (:154): HIST_countFast_wksp BYPASSES
    // both checks when sourceSize < 1500 and delegates to HIST_count_simple.
    // NOTE: `HIST_countFast*` and `HIST_count_simple` are documented UNSAFE when
    // a source byte exceeds `*maxSymbolValuePtr` (they index `count[]` out of
    // bounds with no guard), so they are only ever driven with
    // maxSymbolValue == 255, which every byte trivially satisfies.
    for (src, expect_checked) in [(&small, false), (&big, true)] {
        for misalign in [0usize, 1, 3, 4] {
            for ws in [0usize, 4095, 4096, 8192] {
                let got = diff_bytes(
                    &format!(
                        "HIST_countFast_wksp n={} misalign={misalign} ws={ws}",
                        src.len()
                    ),
                    |l| hist_count_wksp(l, "HIST_countFast_wksp", src, 255, ws, misalign),
                );
                if !expect_checked {
                    assert_eq!(code_of(&got.0), 0, "row 323: checks bypassed below 1500");
                } else if misalign % 4 != 0 {
                    assert_eq!(code_of(&got.0), 1, "row 321: GENERIC (misaligned)");
                } else if ws < HIST_WKSP_SIZE {
                    assert_eq!(code_of(&got.0), 66, "row 322: workSpace_tooSmall");
                }
            }
        }
    }
    // Row 323 with a literally NULL workspace of size 0 (legal below 1500).
    diff_bytes("HIST_countFast_wksp NULL wksp n=1499", |l| {
        let src = small_alphabet_src(1499, 4, 0x11_0323);
        let mut count = vec![0u32; 256];
        let mut msv = 255u32;
        let n = unsafe {
            l.sym::<FnHistCountWksp>("HIST_countFast_wksp")(
                count.as_mut_ptr(),
                &mut msv,
                src.as_ptr() as *const c_void,
                src.len(),
                std::ptr::null_mut(),
                0,
            )
        };
        (res(l, n), msv, u32s(&count))
    });
    covers(&[
        "ERR:compress/hist.c:154",
        "ERR:compress/hist.c:156",
        "ERR:compress/hist.c:157",
        "ERR:compress/hist.c:168",
        "ERR:compress/hist.c:169",
    ]);
}

#[test]
fn hist_count_rejects_alphabet_larger_than_max_symbol_value() {
    // Row 320 (:138): the `checkMaxSymbolValue` branch (taken only when
    // *maxSymbolValuePtr < 255) reports maxSymbolValue_tooSmall (48) when the
    // largest byte present exceeds it.
    for n in [1usize, 4, 16, 100, 1500, 4000] {
        for msv in [0u32, 1, 3, 10, 200, 254, 255] {
            // A source that always contains at least one byte <= msv (so
            // HIST_count_simple's `while (!count[maxSymbolValue]) maxSymbolValue--`
            // cannot underflow) plus one byte far above it.
            let mut src = vec![0u8; n];
            for (i, b) in src.iter_mut().enumerate() {
                *b = if i % 3 == 0 {
                    (i % (msv as usize + 1)) as u8
                } else {
                    200u8.wrapping_add((i % 40) as u8)
                };
            }
            let got = diff_bytes(&format!("HIST_count n={n} msv={msv}"), |l| {
                let mut count = vec![0u32; 256];
                let mut m = msv;
                let a = unsafe {
                    l.sym::<FnHistCount>("HIST_count")(
                        count.as_mut_ptr(),
                        &mut m,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                };
                (res(l, a), m, u32s(&count))
            });
            let largest = *src.iter().max().unwrap() as u32;
            if msv < 255 && largest > msv {
                assert_eq!(code_of(&got.0), 48, "row 320: maxSymbolValue_tooSmall");
            }
        }
    }
    // srcSize == 0 through both HIST_count (parallel path) and HIST_count_simple.
    for msv in [0u32, 1, 10, 254, 255] {
        diff(&format!("HIST srcSize=0 msv={msv}"), |l| {
            let mut c1 = vec![0u32; 256];
            let mut m1 = msv;
            let a = unsafe {
                l.sym::<FnHistCount>("HIST_count")(
                    c1.as_mut_ptr(),
                    &mut m1,
                    [0u8; 4].as_ptr() as *const c_void,
                    0,
                )
            };
            // Row 318 (:48): HIST_count_simple with srcSize == 0 returns 0 and
            // forces *maxSymbolValuePtr to 0 — not an error.
            let mut c2 = vec![0u32; 256];
            let mut m2 = msv;
            let b = unsafe {
                l.sym::<FnHistCountSimple>("HIST_count_simple")(
                    c2.as_mut_ptr(),
                    &mut m2,
                    [0u8; 4].as_ptr() as *const c_void,
                    0,
                )
            };
            let mut c3 = vec![7u32; 256];
            unsafe {
                l.sym::<FnHistAdd>("HIST_add")(
                    c3.as_mut_ptr(),
                    [0u8; 4].as_ptr() as *const c_void,
                    0,
                )
            };
            (res(l, a), m1, b, m2, u32s(&c1), u32s(&c2), u32s(&c3))
        });
    }
    covers(&["ERR:compress/hist.c:48", "ERR:compress/hist.c:138"]);
}

#[test]
fn hist_count_simple_has_no_max_symbol_value_check() {
    // Row 319 (:52): the ONLY guard is `assert(*ip <= maxSymbolValue)`, compiled
    // out at DEBUGLEVEL=0, so a byte above `*maxSymbolValuePtr` is written to
    // `count[byte]` past the `maxSymbolValue+1` entries the caller promised.
    //
    // That is a genuine out-of-bounds write in the C, so it is only exercised in
    // a shape where the "out of bounds" index still lands inside memory this test
    // owns: `count` is a 256-entry array that the test zeroes itself (the C only
    // memsets `maxSymbolValue+1` of them), and every source byte is <= 255.
    // The source also always contains a byte <= maxSymbolValue, otherwise
    // `while (!count[maxSymbolValue]) maxSymbolValue--` underflows to UINT_MAX and
    // reads wildly out of bounds — that input IS out of contract and is not used.
    for msv in [0u32, 1, 3, 10, 100, 200, 254] {
        for n in [1usize, 2, 5, 33, 300] {
            let mut src = vec![0u8; n];
            for (i, b) in src.iter_mut().enumerate() {
                *b = if i == 0 {
                    0
                } else {
                    (msv as usize + 1 + (i % 17)).min(255) as u8
                };
            }
            assert!(src.iter().any(|&b| b as u32 <= msv), "need an in-range byte");
            diff_bytes(&format!("HIST_count_simple oob msv={msv} n={n}"), |l| {
                let mut count = vec![0u32; 256];
                let mut m = msv;
                let r = unsafe {
                    l.sym::<FnHistCountSimple>("HIST_count_simple")(
                        count.as_mut_ptr(),
                        &mut m,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                };
                (r, m, u32s(&count))
            });
        }
    }
    covers(&["ERR:compress/hist.c:52"]);
}

// ===========================================================================
// compress/huf_compress.c — table construction and serialisation
//   rows 326 (:127 HUF_alignUpWorkspace), 328 (:162), 329 (:166), 330 (:167),
//        331 (:181) — HUF_compressWeights, INDIRECT via HUF_writeCTable_wksp
//   rows 332 (:263), 333 (:264), 334 (:274), 335 (:282), 336 (:283)
//   rows 337 (:305), 338 (:306) — HUF_readCTable
//   row  339 (:349) — HUF_getNbBitsFromCTable
//   rows 340 (:770), 341 (:773), 342 (:786) — HUF_buildCTable_wksp
//   rows 343 (:812), 344 (:816) — HUF_validateCTable
// ===========================================================================

const HUF_CTABLE_LEN: usize = HUF_SYMBOLVALUE_MAX as usize + 2; // 257 HUF_CElt

/// Build a CTable with the C library. Returns the table and the depth the C
/// chose (`HUF_buildCTable_wksp`'s return value, which every later stage uses).
///
/// PRECONDITION (unchecked in the C, documented in tests/t10_entropy.rs):
/// `max_nb_bits >= ceil(log2(cardinality))`, otherwise `HUF_setMaxHeight`'s
/// rebalancing walks off `huffNode[]` and the reference C SEGVs. Every caller
/// below passes `max_nb_bits >= 5` with a cardinality of at most 20, or
/// `max_nb_bits >= 8` with a cardinality of 256.
fn c_huf_ctable(count: &[u32], msv: u32, max_nb_bits: u32) -> (Vec<u64>, u32) {
    let card = (0..=msv as usize).filter(|&s| count[s] != 0).count() as u32;
    let need = if card <= 1 { 1 } else { 32 - (card - 1).leading_zeros() };
    assert!(
        max_nb_bits >= need,
        "out of contract: maxNbBits {max_nb_bits} < ceil(log2({card})) = {need}"
    );
    let l = &pair().c;
    let mut ct = vec![0u64; HUF_CTABLE_LEN];
    let mut w = wksp(HUF_CTABLE_WORKSPACE_SIZE);
    let n = unsafe {
        l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
            ct.as_mut_ptr(),
            count.as_ptr(),
            msv,
            max_nb_bits,
            wp(&mut w),
            HUF_CTABLE_WORKSPACE_SIZE,
        )
    };
    match res(l, n) {
        R::Ok(v) => (ct, v as u32),
        e => panic!("fixture HUF_buildCTable_wksp failed: {e:?}"),
    }
}

/// Counts whose Huffman tree is 19 levels deep (Fibonacci frequencies).
fn fib_counts() -> (Vec<u32>, u32) {
    let mut c = vec![0u32; 1024];
    let mut a = 1u32;
    let mut b = 1u32;
    for s in 0..20usize {
        c[s] = a;
        let t = a + b;
        a = b;
        b = t;
    }
    (c, 19)
}
/// 256 equally-frequent symbols: every symbol ends up with exactly 8 bits, so
/// every Huffman *weight* is identical.
fn flat_counts() -> Vec<u32> {
    let mut c = vec![0u32; 1024];
    for s in 0..256usize {
        c[s] = 100;
    }
    c
}

#[test]
fn huf_build_ctable_wksp_rejects_bad_sizes_and_depths() {
    let (fib, _) = fib_counts();
    let flat = flat_counts();

    // Row 340 (:770): wkspSize < sizeof(HUF_buildCTable_wksp_tables), which a
    // static assert pins to exactly HUF_CTABLE_WORKSPACE_SIZE == 4864.
    for ws in [0usize, 1, 8, 4096, 4863, 4864, 4872] {
        let got = diff_bytes(&format!("buildCTable_wksp ws={ws}"), |l| {
            let mut ct = vec![0u64; HUF_CTABLE_LEN];
            let mut w = wksp(ws.max(1) + 16);
            let n = unsafe {
                l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
                    ct.as_mut_ptr(),
                    fib.as_ptr(),
                    19,
                    11,
                    wp(&mut w),
                    ws,
                )
            };
            (res(l, n), u64s(&ct))
        });
        if ws < HUF_CTABLE_WORKSPACE_SIZE {
            assert_eq!(code_of(&got.0), 66, "row 340: workSpace_tooSmall at {ws}");
        } else {
            assert_eq!(code_of(&got.0), 0, "valid build at wkspSize={ws}");
        }
    }
    // Row 326 (:127): HUF_alignUpWorkspace returns NULL and zeroes
    // *workspaceSizePtr when the size is smaller than the alignment padding, so
    // the caller's own size check then fires. A 1-byte-misaligned workspace with
    // only 2 bytes left needs 3 bytes of padding for 4-byte alignment.
    for misalign in [0usize, 1, 2, 3] {
        for ws in [0usize, 1, 2, 3, HUF_CTABLE_WORKSPACE_SIZE, HUF_CTABLE_WORKSPACE_SIZE + 4] {
            let got = diff_bytes(
                &format!("buildCTable_wksp misalign={misalign} ws={ws}"),
                |l| {
                    let mut ct = vec![0u64; HUF_CTABLE_LEN];
                    let mut w = vec![0u8; ws + 64];
                    let base = unsafe { w.as_mut_ptr().add(misalign) };
                    let n = unsafe {
                        l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
                            ct.as_mut_ptr(),
                            fib.as_ptr(),
                            19,
                            11,
                            base as *mut c_void,
                            ws,
                        )
                    };
                    (res(l, n), u64s(&ct))
                },
            );
            // 4-byte alignment padding shrinks the usable size, so anything at or
            // below the bound must be rejected.
            if ws <= HUF_CTABLE_WORKSPACE_SIZE && misalign % 4 != 0 {
                assert_eq!(code_of(&got.0), 66, "row 326: workSpace_tooSmall");
            }
        }
    }
    // Row 341 (:773): maxSymbolValue > HUF_SYMBOLVALUE_MAX (255). Note maxNbBits
    // == 0 is silently remapped to HUF_TABLELOG_DEFAULT (11) at :772 *first*.
    for msv in [256u32, 257, 300, 1000] {
        for max_nb_bits in [0u32, 11] {
            let got = diff_bytes(
                &format!("buildCTable_wksp msv={msv} bits={max_nb_bits}"),
                |l| {
                    let mut ct = vec![0u64; HUF_CTABLE_LEN];
                    let mut w = wksp(HUF_CTABLE_WORKSPACE_SIZE);
                    let n = unsafe {
                        l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
                            ct.as_mut_ptr(),
                            flat.as_ptr(),
                            msv,
                            max_nb_bits,
                            wp(&mut w),
                            HUF_CTABLE_WORKSPACE_SIZE,
                        )
                    };
                    (res(l, n), u64s(&ct))
                },
            );
            assert_eq!(code_of(&got.0), 46, "row 341: maxSymbolValue_tooLarge");
        }
    }
    // Row 342 (:786): after HUF_setMaxHeight the depth is still > 12, which only
    // happens when the caller asked for maxNbBits >= 13 AND the natural tree is
    // at least that deep. The Fibonacci histogram's natural depth is 19.
    for max_nb_bits in [5u32, 8, 11, 12, 13, 14, 15, 20, 255, 1000] {
        let got = diff_bytes(&format!("buildCTable_wksp deep bits={max_nb_bits}"), |l| {
            let mut ct = vec![0u64; HUF_CTABLE_LEN];
            let mut w = wksp(HUF_CTABLE_WORKSPACE_SIZE);
            let n = unsafe {
                l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
                    ct.as_mut_ptr(),
                    fib.as_ptr(),
                    19,
                    max_nb_bits,
                    wp(&mut w),
                    HUF_CTABLE_WORKSPACE_SIZE,
                )
            };
            (res(l, n), u64s(&ct))
        });
        if max_nb_bits > HUF_TABLELOG_MAX {
            assert_eq!(code_of(&got.0), 1, "row 342: GENERIC at maxNbBits={max_nb_bits}");
        } else {
            assert_eq!(got.0, R::Ok(max_nb_bits as usize), "depth clamped to request");
        }
    }
    covers(&[
        "ERR:compress/huf_compress.c:127",
        "ERR:compress/huf_compress.c:770",
        "ERR:compress/huf_compress.c:773",
        "ERR:compress/huf_compress.c:786",
    ]);
}

/// `HUF_writeCTable_wksp` with the whole destination buffer compared. `dst` is
/// always allocated larger than `max_dst_size`.
fn write_ctable(
    l: &Lib,
    ct: &[u64],
    msv: u32,
    huff_log: u32,
    max_dst_size: usize,
    wksp_size: usize,
    misalign: usize,
) -> (R, Blob) {
    let mut dst = vec![0xEEu8; max_dst_size + 128];
    let mut w = vec![0u8; wksp_size + 64];
    let base = unsafe { w.as_mut_ptr().add(misalign) };
    let n = unsafe {
        l.sym::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp")(
            dst.as_mut_ptr() as *mut c_void,
            max_dst_size,
            ct.as_ptr(),
            msv,
            huff_log,
            base as *mut c_void,
            wksp_size,
        )
    };
    (res(l, n), Blob(dst))
}

#[test]
fn huf_write_ctable_wksp_rejects_bad_sizes_and_alphabets() {
    let flat = flat_counts();
    let (ct_flat, log_flat) = c_huf_ctable(&flat, 255, 11);
    assert_eq!(log_flat, 8, "256 equal counts must give an 8-bit balanced tree");

    // Row 332 (:263): workspaceSize < sizeof(HUF_WriteCTableWksp). The *documented*
    // bound is HUF_CTABLE_WORKSPACE_SIZE (4864) but the runtime check uses the
    // real struct size, so sweep the whole range and let the two libraries agree
    // on the exact threshold — a strong structural check on the layout.
    let mut generic = 0usize;
    for ws in [
        0usize, 1, 4, 8, 64, 256, 512, 600, 700, 740, 744, 748, 752, 760, 800, 1024, 2048,
        4096, 4863, 4864, 4872,
    ] {
        let got = diff_bytes(&format!("writeCTable ws={ws}"), |l| {
            write_ctable(l, &ct_flat, 255, log_flat, 4096, ws, 0)
        });
        if is_err_code(&got.0, 1) {
            generic += 1;
        }
    }
    assert!(generic > 0, "no workspaceSize reached huf_compress.c:263");
    // Row 326 again, through HUF_writeCTable_wksp's own alignment fixup.
    for misalign in [0usize, 1, 2, 3] {
        for ws in [0usize, 1, 2, 3, 4864] {
            diff_bytes(
                &format!("writeCTable misalign={misalign} ws={ws}"),
                |l| write_ctable(l, &ct_flat, 255, log_flat, 4096, ws, misalign),
            );
        }
    }
    // Row 333 (:264): maxSymbolValue > HUF_SYMBOLVALUE_MAX (255).
    for msv in [256u32, 257, 300] {
        let got = diff_bytes(&format!("writeCTable msv={msv}"), |l| {
            write_ctable(l, &ct_flat, msv, log_flat, 4096, HUF_CTABLE_WORKSPACE_SIZE, 0)
        });
        assert_eq!(code_of(&got.0), 46, "row 333: maxSymbolValue_tooLarge");
    }
    // Row 334 (:274): maxDstSize < 1 -> dstSize_tooSmall.
    let got = diff_bytes("writeCTable maxDstSize=0", |l| {
        write_ctable(l, &ct_flat, 255, log_flat, 0, HUF_CTABLE_WORKSPACE_SIZE, 0)
    });
    assert_eq!(code_of(&got.0), 70, "row 334: dstSize_tooSmall");

    // Rows 329 (:166) and 335 (:282): every Huffman weight is identical, so
    // HUF_compressWeights returns 1 ("rle"), `hSize > 1` fails, and the raw
    // 4-bit fallback then rejects maxSymbolValue > 128 with GENERIC.
    for msv in [129u32, 150, 200, 255] {
        let got = diff_bytes(&format!("writeCTable rle-weights msv={msv}"), |l| {
            write_ctable(l, &ct_flat, msv, log_flat, 4096, HUF_CTABLE_WORKSPACE_SIZE, 0)
        });
        assert_eq!(code_of(&got.0), 1, "row 329/335: GENERIC at msv={msv}");
    }
    // Row 336 (:283): the raw path with ((maxSymbolValue+1)/2)+1 > maxDstSize.
    for msv in [2u32, 16, 64, 100, 128] {
        let need = ((msv as usize + 1) / 2) + 1;
        for cap in [1usize, 2, need - 1, need, need + 1] {
            let got = diff_bytes(
                &format!("writeCTable raw msv={msv} cap={cap} (need {need})"),
                |l| write_ctable(l, &ct_flat, msv, log_flat, cap, HUF_CTABLE_WORKSPACE_SIZE, 0),
            );
            if cap < need {
                assert_eq!(code_of(&got.0), 70, "row 336: dstSize_tooSmall");
            } else {
                assert_eq!(got.0, R::Ok(need), "raw weights fit exactly");
            }
        }
    }
    // Row 328 (:162): wtSize <= 1, i.e. maxSymbolValue <= 1 -> HUF_compressWeights
    // returns 0 ("not compressible") and the raw 4-bit path is used.
    for msv in [0u32, 1] {
        let got = diff_bytes(&format!("writeCTable wtSize<=1 msv={msv}"), |l| {
            write_ctable(l, &ct_flat, msv, log_flat, 64, HUF_CTABLE_WORKSPACE_SIZE, 0)
        });
        assert_eq!(code_of(&got.0), 0, "row 328: falls through to raw weights");
    }
    // Row 330 (:167): maxCount == 1 — every weight value occurs at most once.
    // A 4-symbol table with nbBits {1,2,3,3} and maxSymbolValue 3 converts only
    // symbols 0..2, whose weights {3,2,1} are all distinct.
    {
        let mut count = vec![0u32; 1024];
        count[0] = 8;
        count[1] = 4;
        count[2] = 2;
        count[3] = 1;
        let (ct, log) = c_huf_ctable(&count, 3, 5);
        let nb: Vec<u32> = (0..4)
            .map(|s| {
                let l = &pair().c;
                unsafe { l.sym::<FnHufGetNbBits>("HUF_getNbBitsFromCTable")(ct.as_ptr(), s) }
            })
            .collect();
        assert_eq!(nb, vec![1, 2, 3, 3], "expected a skewed 4-symbol tree");
        let got = diff_bytes("writeCTable distinct weights", |l| {
            write_ctable(l, &ct, 3, log, 64, HUF_CTABLE_WORKSPACE_SIZE, 0)
        });
        assert_eq!(got.0, R::Ok(3), "row 330: raw 4-bit weights, 3 bytes");
    }
    // Row 331 (:181): FSE_compress_usingCTable cannot fit the weight bitstream in
    // `dstSize-1`, so HUF_compressWeights returns 0 and the raw path is taken —
    // which for maxSymbolValue 255 then hits :282 (GENERIC).
    {
        let mut count = vec![0u32; 1024];
        for s in 0..256usize {
            // a compressible weight histogram: geometric-ish frequencies
            count[s] = 1 + ((s as u32 * 7919) % 64);
        }
        let (ct, log) = c_huf_ctable(&count, 255, 11);
        for cap in 1..=90usize {
            diff_bytes(&format!("writeCTable fse-loses cap={cap}"), |l| {
                write_ctable(l, &ct, 255, log, cap, HUF_CTABLE_WORKSPACE_SIZE, 0)
            });
        }
    }
    covers(&[
        "ERR:compress/huf_compress.c:127",
        "ERR:compress/huf_compress.c:162",
        "ERR:compress/huf_compress.c:166",
        "ERR:compress/huf_compress.c:167",
        "ERR:compress/huf_compress.c:181",
        "ERR:compress/huf_compress.c:263",
        "ERR:compress/huf_compress.c:264",
        "ERR:compress/huf_compress.c:274",
        "ERR:compress/huf_compress.c:282",
        "ERR:compress/huf_compress.c:283",
    ]);
}

#[test]
fn huf_read_ctable_and_table_accessors() {
    // A serialised Huffman header describing 10 symbols, produced by the C.
    let mut count = vec![0u32; 1024];
    for s in 0..10usize {
        count[s] = 1 << (10 - s);
    }
    let (ct, log) = c_huf_ctable(&count, 9, 5);
    let hdr = {
        let l = &pair().c;
        let (r, b) = write_ctable(l, &ct, 9, log, 512, HUF_CTABLE_WORKSPACE_SIZE, 0);
        match r {
            R::Ok(n) => b.0[..n].to_vec(),
            e => panic!("fixture HUF_writeCTable_wksp failed: {e:?}"),
        }
    };

    // Row 338 (:306): nbSymbols > *maxSymbolValuePtr + 1 -> maxSymbolValue_tooSmall.
    for msv_in in [0u32, 1, 3, 8, 9, 10, 255] {
        let got = diff(&format!("readCTable msv_in={msv_in}"), |l| {
            let mut out = vec![0u64; HUF_CTABLE_LEN];
            let mut msv = msv_in;
            let mut hzw = 0xAAu32;
            let n = unsafe {
                l.sym::<FnHufReadCTable>("HUF_readCTable")(
                    out.as_mut_ptr(),
                    &mut msv,
                    hdr.as_ptr() as *const c_void,
                    hdr.len(),
                    &mut hzw,
                )
            };
            (res(l, n), msv, hzw, u64s(&out))
        });
        if msv_in + 1 < 10 {
            assert_eq!(code_of(&got.0), 48, "row 338: maxSymbolValue_tooSmall");
        } else {
            assert_eq!(code_of(&got.0), 0, "valid readCTable at msv_in={msv_in}");
        }
    }
    // Row 337 (:305) is `tableLog > HUF_TABLELOG_MAX` *after* HUF_readStats, but
    // HUF_readStats already rejects exactly that at entropy_common.c:288
    // (corruption_detected), so :305 cannot fire. Confirm the dominating guard
    // instead, using the src={0x81,0xCC} header whose weightTotal is 4096.
    {
        let got = diff("readCTable tableLog 13 header", |l| {
            let mut out = vec![0u64; HUF_CTABLE_LEN];
            let mut msv = 255u32;
            let mut hzw = 0u32;
            let n = unsafe {
                l.sym::<FnHufReadCTable>("HUF_readCTable")(
                    out.as_mut_ptr(),
                    &mut msv,
                    [0x81u8, 0xCC].as_ptr() as *const c_void,
                    2,
                    &mut hzw,
                )
            };
            (res(l, n), msv, hzw, u64s(&out))
        });
        assert_eq!(code_of(&got.0), 20, "entropy_common.c:288 dominates :305");
    }
    // truncations and short garbage headers
    let mut rng = Rng::new(0x11_0000_0338);
    for n in 0..=hdr.len() {
        diff(&format!("readCTable truncated {n}"), |l| {
            let mut out = vec![0u64; HUF_CTABLE_LEN];
            let mut msv = 255u32;
            let mut hzw = 0u32;
            let r = unsafe {
                l.sym::<FnHufReadCTable>("HUF_readCTable")(
                    out.as_mut_ptr(),
                    &mut msv,
                    hdr.as_ptr() as *const c_void,
                    n,
                    &mut hzw,
                )
            };
            (res(l, r), msv, hzw, u64s(&out))
        });
    }
    for i in 0..400 {
        let n = 1 + rng.below(8);
        let src = rng.bytes(n);
        diff(&format!("readCTable fuzz #{i}"), |l| {
            let mut out = vec![0u64; HUF_CTABLE_LEN];
            let mut msv = 255u32;
            let mut hzw = 0u32;
            let r = unsafe {
                l.sym::<FnHufReadCTable>("HUF_readCTable")(
                    out.as_mut_ptr(),
                    &mut msv,
                    src.as_ptr() as *const c_void,
                    n,
                    &mut hzw,
                )
            };
            (res(l, r), msv, hzw, u64s(&out))
        });
    }

    // Row 339 (:349): HUF_getNbBitsFromCTable returns 0 for a symbol above the
    // table's own maxSymbolValue. `symbolValue > 255` is safe here even though
    // the `assert(symbolValue <= 255)` is compiled out, because
    // `HUF_readCTableHeader().maxSymbolValue` is a BYTE and the early return
    // fires before any indexing.
    diff("HUF_getNbBitsFromCTable / readCTableHeader", |l| {
        let hdr_word = unsafe { l.sym::<FnHufReadCTableHeader>("HUF_readCTableHeader")(ct.as_ptr()) };
        let bits: Vec<u32> = [
            0u32, 1, 8, 9, 10, 11, 100, 254, 255, 256, 300, 1000, u32::MAX,
        ]
        .iter()
        .map(|&s| unsafe { l.sym::<FnHufGetNbBits>("HUF_getNbBitsFromCTable")(ct.as_ptr(), s) })
        .collect();
        (hdr_word, bits)
    });

    // Rows 343 (:812) and 344 (:816): HUF_validateCTable returns 0 when the
    // table's maxSymbolValue is below the requested one, or when a present symbol
    // has no code. Plus HUF_estimateCompressedSize over the same grid.
    let mut wide = vec![0u32; 1024];
    for s in 0..=255usize {
        wide[s] = 1 + (s as u32 % 5);
    }
    for msv in [0u32, 5, 9, 10, 11, 100, 255] {
        diff(&format!("validateCTable/estimate msv={msv}"), |l| {
            (
                unsafe {
                    l.sym::<FnHufValidateCTable>("HUF_validateCTable")(
                        ct.as_ptr(),
                        count.as_ptr(),
                        msv,
                    )
                },
                unsafe {
                    l.sym::<FnHufValidateCTable>("HUF_validateCTable")(
                        ct.as_ptr(),
                        wide.as_ptr(),
                        msv,
                    )
                },
                unsafe {
                    res(
                        l,
                        l.sym::<FnHufEstimateCompressedSize>("HUF_estimateCompressedSize")(
                            ct.as_ptr(),
                            count.as_ptr(),
                            msv,
                        ),
                    )
                },
                unsafe {
                    res(
                        l,
                        l.sym::<FnHufEstimateCompressedSize>("HUF_estimateCompressedSize")(
                            ct.as_ptr(),
                            wide.as_ptr(),
                            msv,
                        ),
                    )
                },
            )
        });
    }
    covers(&[
        "ERR:compress/huf_compress.c:306",
        "ERR:compress/huf_compress.c:349",
        "ERR:compress/huf_compress.c:812",
        "ERR:compress/huf_compress.c:816",
        "ERR:common/entropy_common.c:288",
    ]);
}

// ===========================================================================
// compress/huf_compress.c — the encoders
//   rows 345 (:863), 346 (:979), 347 (:1068), 348 (:1073), 349 (:1179),
//        350 (:1180), 351 (:1185), 352 (:1232), 353 (:1237), 354 (:1349),
//        355 (:1350), 356 (:1351), 357 (:1352), 358 (:1353), 359 (:1354),
//        360 (:1355), 361 (:1378), 362 (:1383), 363 (:1384), 364 (:1418),
//        365 (:1425)
// ===========================================================================

#[test]
fn huf_compress_using_ctable_returns_zero_when_it_cannot_fit() {
    let src = small_alphabet_src(4096, 16, 0x11_0347);
    let mut count = vec![0u32; 1024];
    let mut msv = 255u32;
    {
        let l = &pair().c;
        unsafe {
            l.sym::<FnHistCount>("HIST_count")(
                count.as_mut_ptr(),
                &mut msv,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
    }
    // Two tables: one at the natural depth and one forced to tableLog 12, so both
    // the fast and the bounds-checked encoding loop at :1073 are selected.
    let (ct_nat, log_nat) = c_huf_ctable(&count, msv, 11);
    let (ct_deep, log_deep) = {
        let mut c = vec![0u32; 1024];
        for s in 0..256usize {
            c[s] = 1 + ((s as u32 * 2654435761) % 4096);
        }
        c_huf_ctable(&c, 255, 12)
    };
    assert_eq!(log_deep, 12, "need a tableLog-12 CTable for row 348");

    let one_x = |ct: &Vec<u64>, n: usize, dst_size: usize, four: bool, flags: c_int| {
        let ct = ct.clone();
        let src = src.clone();
        move |l: &Lib| {
            let mut dst = vec![0xD1u8; dst_size + 128];
            let sym = if four {
                "HUF_compress4X_usingCTable"
            } else {
                "HUF_compress1X_usingCTable"
            };
            let r = unsafe {
                l.sym::<FnHufCompressUsingCTable>(sym)(
                    dst.as_mut_ptr() as *mut c_void,
                    dst_size,
                    src.as_ptr() as *const c_void,
                    n,
                    ct.as_ptr(),
                    flags,
                )
            };
            (res(l, r), Blob(dst))
        }
    };

    // Rows 347 (:1068) and 345 (:863): dstSize < 8 -> 0, and dstSize == 8 makes
    // HUF_initCStream fail with dstSize_tooSmall which :1071 turns into 0.
    for dst_size in 0..=9usize {
        let got = diff_bytes(
            &format!("compress1X_usingCTable dstSize={dst_size}"),
            one_x(&ct_nat, src.len(), dst_size, false, 0),
        );
        if dst_size <= 8 {
            assert_eq!(got.0, R::Ok(0), "row 347/345: dstSize {dst_size} -> 0");
        }
    }
    // Row 346 (:979): the bitstream overflows dstCapacity and HUF_closeCStream
    // reports 0. Sweep every capacity from 9 up to well past the point where the
    // stream fits; :1073's slow/fast loop selection (row 348) is crossed on the
    // way, and both loops must emit byte-identical output.
    let mut zeros = 0usize;
    let tight_nat = ((src.len() * log_nat as usize) >> 3) + 8;
    for dst_size in 9..=(tight_nat + 40) {
        let got = diff_bytes(
            &format!("compress1X_usingCTable overflow dstSize={dst_size}"),
            one_x(&ct_nat, src.len(), dst_size, false, 0),
        );
        if got.0 == R::Ok(0) {
            zeros += 1;
        }
    }
    assert!(zeros > 0, "no capacity made HUF_closeCStream return 0");
    // Row 348 (:1073) explicitly: a tableLog-12 table always takes the
    // bounds-checked loop; compare it against the same input at tableLog 11.
    for dst_size in [
        9usize, 16, 64, 1024, 4096, 8192, tight_nat, tight_nat + 1, tight_nat + 64,
    ] {
        for flags in [0, HUF_flags_disableAsm, HUF_flags_disableFast, HUF_flags_bmi2] {
            diff_bytes(
                &format!("compress1X_usingCTable deep dstSize={dst_size} flags={flags}"),
                one_x(&ct_deep, src.len(), dst_size, false, flags),
            );
        }
    }
    // Rows 349 (:1179) and 350 (:1180): the 4-stream encoder returns 0 for
    // dstSize < 17 or srcSize < 12.
    for dst_size in [0usize, 1, 6, 8, 16, 17, 18, 4096] {
        for n in [0usize, 1, 11, 12, 13, 256, 4096] {
            let got = diff_bytes(
                &format!("compress4X_usingCTable dstSize={dst_size} srcSize={n}"),
                one_x(&ct_nat, n, dst_size, true, 0),
            );
            if dst_size < 17 || n < 12 {
                assert_eq!(got.0, R::Ok(0), "row 349/350: -> 0");
            }
        }
    }
    // Row 351 (:1185 and the identical sites at :1193/:1201/:1210): a segment
    // either does not fit (cSize == 0) or exceeds the 16-bit jump-table field
    // (cSize > 65535). Incompressible data of 400 000 bytes gives 100 000-byte
    // segments, which an 8-bit flat table cannot represent in 16 bits.
    {
        let big = corpus(Corpus::Random, 400_000, 0x11_0351);
        let flat = flat_counts();
        let (ct_flat, _log) = c_huf_ctable(&flat, 255, 11);
        for dst_size in [17usize, 512, 100_000, 500_000] {
            let got = diff_bytes(
                &format!("compress4X_usingCTable 400k dstSize={dst_size}"),
                |l| {
                    let mut dst = vec![0xD4u8; dst_size + 128];
                    let r = unsafe {
                        l.sym::<FnHufCompressUsingCTable>("HUF_compress4X_usingCTable")(
                            dst.as_mut_ptr() as *mut c_void,
                            dst_size,
                            big.as_ptr() as *const c_void,
                            big.len(),
                            ct_flat.as_ptr(),
                            0,
                        )
                    };
                    (res(l, r), Blob(dst))
                },
            );
            assert_eq!(got.0, R::Ok(0), "row 351: segment not representable -> 0");
        }
    }
    covers(&[
        "ERR:compress/huf_compress.c:863",
        "ERR:compress/huf_compress.c:979",
        "ERR:compress/huf_compress.c:1068",
        "ERR:compress/huf_compress.c:1073",
        "ERR:compress/huf_compress.c:1179",
        "ERR:compress/huf_compress.c:1180",
        "ERR:compress/huf_compress.c:1185",
    ]);
}

/// `HUF_compress{1,4}X_repeat` with every observable compared: the return value,
/// the repeat enum the callee left behind, the whole 257-entry table and the
/// whole destination buffer.
#[allow(clippy::too_many_arguments)]
fn huf_compress_repeat(
    l: &Lib,
    four: bool,
    src: &[u8],
    dst_size: usize,
    msv: u32,
    huff_log: u32,
    wksp_size: usize,
    table_in: &[u64],
    repeat_in: c_int,
    flags: c_int,
    null_table: bool,
) -> (R, c_int, Blob) {
    let sym = if four {
        "HUF_compress4X_repeat"
    } else {
        "HUF_compress1X_repeat"
    };
    let mut dst = vec![0x11u8; dst_size + 128];
    let mut table = table_in.to_vec();
    let mut repeat = repeat_in;
    let mut w = wksp(wksp_size.max(1));
    let n = unsafe {
        l.sym::<FnHufCompressRepeat>(sym)(
            dst.as_mut_ptr() as *mut c_void,
            dst_size,
            src.as_ptr() as *const c_void,
            src.len(),
            msv,
            huff_log,
            wp(&mut w),
            wksp_size,
            if null_table {
                std::ptr::null_mut()
            } else {
                table.as_mut_ptr()
            },
            if null_table {
                std::ptr::null_mut()
            } else {
                &mut repeat
            },
            flags,
        )
    };
    // table bytes ++ destination bytes, so `diff_bytes` can point at the first
    // differing byte of either.
    let mut all = u64s(&table).0;
    all.extend_from_slice(&dst);
    (res(l, n), repeat, Blob(all))
}

#[test]
fn huf_compress_repeat_rejects_out_of_range_parameters() {
    let src = small_alphabet_src(4096, 16, 0x11_0354);
    let zero_table = vec![0u64; HUF_CTABLE_LEN];

    // Row 354 (:1349): wkspSize < sizeof(HUF_compress_tables_t). The documented
    // bound is HUF_WORKSPACE_SIZE (8704); the runtime check uses the real struct
    // size, so sweep and let both libraries agree on the threshold.
    let mut too_small = 0usize;
    for ws in [
        0usize, 1, 8, 1024, 4096, 7000, 7936, 7940, 7944, 7948, 8192, 8696, 8704, 8712,
    ] {
        let got = diff_bytes(&format!("compress1X_repeat ws={ws}"), |l| {
            huf_compress_repeat(l, false, &src, 4096, 255, 11, ws, &zero_table, 0, 0, false)
        });
        if is_err_code(&got.0, 66) {
            too_small += 1;
        }
    }
    assert!(too_small > 0, "no wkspSize reached huf_compress.c:1349");
    // NULL hufTable / NULL repeat is the documented way HUF_compress1X_repeat is
    // called by ZSTD_compressLiterals' non-repeat path.
    for four in [false, true] {
        diff_bytes(&format!("compress_repeat NULL table four={four}"), |l| {
            huf_compress_repeat(
                l, four, &src, 4096, 255, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0, true,
            )
        });
    }
    // Rows 355 (:1350) srcSize == 0 and 356 (:1351) dstSize == 0 -> plain 0, NOT
    // an error. Note the order: the workspace check comes first.
    for (n, dst_size) in [(0usize, 4096usize), (0, 0), (100, 0)] {
        for four in [false, true] {
            let s = small_alphabet_src(n, 16, 0x11_0355);
            let got = diff_bytes(
                &format!("compress_repeat srcSize={n} dstSize={dst_size} four={four}"),
                |l| {
                    huf_compress_repeat(
                        l, four, &s, dst_size, 255, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0,
                        false,
                    )
                },
            );
            assert_eq!(got.0, R::Ok(0), "row 355/356: -> 0, not an error");
        }
    }
    // Row 357 (:1352): srcSize > HUF_BLOCKSIZE_MAX (131072) -> srcSize_wrong (72).
    for n in [HUF_BLOCKSIZE_MAX, HUF_BLOCKSIZE_MAX + 1, HUF_BLOCKSIZE_MAX + 4096] {
        let s = small_alphabet_src(n, 16, 0x11_0357);
        for four in [false, true] {
            let got = diff_bytes(
                &format!("compress_repeat srcSize={n} four={four}"),
                |l| {
                    huf_compress_repeat(
                        l,
                        four,
                        &s,
                        n + 4096,
                        255,
                        11,
                        HUF_WORKSPACE_SIZE,
                        &zero_table,
                        0,
                        0,
                        false,
                    )
                },
            );
            if n > HUF_BLOCKSIZE_MAX {
                assert_eq!(code_of(&got.0), 72, "row 357: srcSize_wrong");
            }
        }
    }
    // Row 358 (:1353): huffLog > HUF_TABLELOG_MAX (12) -> tableLog_tooLarge (44).
    // Row 360 (:1356): huffLog == 0 is silently remapped to 11.
    for huff_log in [0u32, 1, 5, 11, 12, 13, 14, 20, 255] {
        for four in [false, true] {
            let got = diff_bytes(
                &format!("compress_repeat huffLog={huff_log} four={four}"),
                |l| {
                    huf_compress_repeat(
                        l, four, &src, 4096, 255, huff_log, HUF_WORKSPACE_SIZE, &zero_table, 0,
                        0, false,
                    )
                },
            );
            if huff_log > HUF_TABLELOG_MAX {
                assert_eq!(code_of(&got.0), 44, "row 358: tableLog_tooLarge");
            }
        }
    }
    // Row 359 (:1354): maxSymbolValue > 255 -> maxSymbolValue_tooLarge (46).
    // Row 360 (:1355): maxSymbolValue == 0 is silently remapped to 255.
    for msv in [0u32, 1, 15, 255, 256, 257, 1000] {
        for four in [false, true] {
            let got = diff_bytes(
                &format!("compress_repeat msv={msv} four={four}"),
                |l| {
                    huf_compress_repeat(
                        l, four, &src, 4096, msv, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0,
                        false,
                    )
                },
            );
            if msv > HUF_SYMBOLVALUE_MAX {
                assert_eq!(code_of(&got.0), 46, "row 359: maxSymbolValue_tooLarge");
            }
        }
    }
    // The `HUF_repeat` enum out of range. A C enum parameter accepts any int, and
    // the C only ever compares `*repeat` against `HUF_repeat_none/check/valid`, so
    // 3, -1 and 999 all take the "not none" branches.
    for repeat_in in [0i32, 1, 2, 3, -1, 999, i32::MIN, i32::MAX] {
        for flags in [0, HUF_flags_preferRepeat] {
            for four in [false, true] {
                diff_bytes(
                    &format!("compress_repeat repeat={repeat_in} flags={flags} four={four}"),
                    |l| {
                        huf_compress_repeat(
                            l,
                            four,
                            &src,
                            4096,
                            255,
                            11,
                            HUF_WORKSPACE_SIZE,
                            &zero_table,
                            repeat_in,
                            flags,
                            false,
                        )
                    },
                );
            }
        }
    }
    covers(&[
        "ERR:compress/huf_compress.c:1349",
        "ERR:compress/huf_compress.c:1350",
        "ERR:compress/huf_compress.c:1351",
        "ERR:compress/huf_compress.c:1352",
        "ERR:compress/huf_compress.c:1353",
        "ERR:compress/huf_compress.c:1354",
        "ERR:compress/huf_compress.c:1355",
    ]);
}

#[test]
fn huf_compress_repeat_heuristics_and_reuse() {
    let zero_table = vec![0u64; HUF_CTABLE_LEN];

    // Row 361 (:1378): flags & HUF_flags_suspectUncompressible AND srcSize >=
    // 40960 AND largestTotal <= 68 -> plain 0.
    {
        let big = corpus(Corpus::Random, 65536, 0x11_0361);
        for four in [false, true] {
            let got = diff_bytes(&format!("compress_repeat suspect four={four}"), |l| {
                huf_compress_repeat(
                    l,
                    four,
                    &big,
                    70000,
                    255,
                    11,
                    HUF_WORKSPACE_SIZE,
                    &zero_table,
                    0,
                    HUF_flags_suspectUncompressible,
                    false,
                )
            });
            assert_eq!(got.0, R::Ok(0), "row 361: sampling heuristic -> 0");
        }
        // and just below the 40960 threshold, where the heuristic is skipped
        let small = corpus(Corpus::Random, 40959, 0x11_0361);
        for four in [false, true] {
            diff_bytes(&format!("compress_repeat suspect-40959 four={four}"), |l| {
                huf_compress_repeat(
                    l,
                    four,
                    &small,
                    50000,
                    255,
                    11,
                    HUF_WORKSPACE_SIZE,
                    &zero_table,
                    0,
                    HUF_flags_suspectUncompressible,
                    false,
                )
            });
        }
    }
    // Row 362 (:1383): largest == srcSize (a single symbol) -> 1, with dst[0]
    // holding that symbol. NOT an error.
    for n in [1usize, 2, 12, 100, 4096] {
        let rle = vec![0xA5u8; n];
        for four in [false, true] {
            let got = diff_bytes(&format!("compress_repeat rle n={n} four={four}"), |l| {
                huf_compress_repeat(
                    l, four, &rle, 4096, 255, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0, false,
                )
            });
            assert_eq!(got.0, R::Ok(1), "row 362: single symbol -> 1");
        }
    }
    // Row 363 (:1384): largest <= (srcSize >> 7) + 4 -> 0.
    {
        let flat = corpus(Corpus::Random, 100_000, 0x11_0363);
        for four in [false, true] {
            let got = diff_bytes(&format!("compress_repeat flat four={four}"), |l| {
                huf_compress_repeat(
                    l, four, &flat, 120_000, 255, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0,
                    false,
                )
            });
            assert_eq!(got.0, R::Ok(0), "row 363: not compressible enough -> 0");
        }
    }
    // Row 365 (:1425): hSize + 12 >= srcSize (the table header alone eats the
    // gain) -> 0. 40 bytes over a wide alphabet.
    for n in [13usize, 20, 40, 64, 100, 200] {
        let wide: Vec<u8> = (0..n).map(|i| ((i * 37) % 251) as u8).collect();
        for four in [false, true] {
            diff_bytes(&format!("compress_repeat wide n={n} four={four}"), |l| {
                huf_compress_repeat(
                    l, four, &wide, 4096, 255, 11, HUF_WORKSPACE_SIZE, &zero_table, 0, 0, false,
                )
            });
        }
    }
    // Rows 364 (:1418), 352 (:1232) and 353 (:1237): drive the repeat-table
    // machinery for real — first call builds a table, second reuses it with
    // HUF_repeat_check / _valid and every flag combination, and with destination
    // capacities small enough that the re-encode with the old table returns an
    // error (:1232) or 0 (:1237).
    let mut rng = Rng::new(0x11_0000_0364);
    for i in 0..90 {
        let n = 16 + rng.below(30000);
        let a = small_alphabet_src(n, 1 + rng.u8() % 64, rng.next_u64());
        let b = small_alphabet_src(n, 1 + rng.u8() % 64, rng.next_u64());
        let four = rng.bool();
        let flags = (rng.next_u32() & 0x3D) as c_int; // never optimalDepth alone
        let caps = [
            17usize,
            32,
            64,
            256,
            n / 4 + 32,
            n + 128,
        ];
        for &cap in caps.iter() {
            for &start in &[0i32, 1, 2, 3] {
                diff(
                    &format!("compress_repeat reuse #{i} n={n} cap={cap} four={four} flags={flags} r0={start}"),
                    |l| {
                        let sym = if four {
                            "HUF_compress4X_repeat"
                        } else {
                            "HUF_compress1X_repeat"
                        };
                        let f = l.sym::<FnHufCompressRepeat>(sym);
                        let mut table = vec![0u64; HUF_CTABLE_LEN];
                        let mut repeat = start;
                        let mut w = wksp(HUF_WORKSPACE_SIZE);
                        let mut d1 = vec![0x11u8; cap + 128];
                        let n1 = unsafe {
                            f(
                                d1.as_mut_ptr() as *mut c_void,
                                cap,
                                a.as_ptr() as *const c_void,
                                a.len(),
                                255,
                                11,
                                wp(&mut w),
                                HUF_WORKSPACE_SIZE,
                                table.as_mut_ptr(),
                                &mut repeat,
                                flags,
                            )
                        };
                        let r1 = res(l, n1);
                        let rep1 = repeat;
                        let t1 = u64s(&table);
                        let mut d2 = vec![0x22u8; cap + 128];
                        let n2 = unsafe {
                            f(
                                d2.as_mut_ptr() as *mut c_void,
                                cap,
                                b.as_ptr() as *const c_void,
                                b.len(),
                                255,
                                11,
                                wp(&mut w),
                                HUF_WORKSPACE_SIZE,
                                table.as_mut_ptr(),
                                &mut repeat,
                                flags,
                            )
                        };
                        (r1, rep1, t1, res(l, n2), repeat, u64s(&table), Blob(d1), Blob(d2))
                    },
                );
            }
        }
    }
    covers(&[
        "ERR:compress/huf_compress.c:1232",
        "ERR:compress/huf_compress.c:1237",
        "ERR:compress/huf_compress.c:1378",
        "ERR:compress/huf_compress.c:1383",
        "ERR:compress/huf_compress.c:1384",
        "ERR:compress/huf_compress.c:1418",
        "ERR:compress/huf_compress.c:1425",
    ]);
}

// ===========================================================================
// The Huffman DECODERS, driven for their rejections. The rejection sites live in
// `decompress/huf_decompress.c` (another suite's section), but two rows of MY
// section are only reachable through them:
//   row 36 (common/bitstream.h:402 BIT_reloadDStreamFast overflow) — INDIRECT
//   row 40 (common/bitstream.h:451 BIT_endOfDStream acceptance gate) — INDIRECT
// ===========================================================================

/// A serialised Huffman table plus 1-stream and 4-stream bitstreams over `src`,
/// all produced by the C library.
struct HufFix {
    hdr: Vec<u8>,
    c1x: Vec<u8>,
    c4x: Vec<u8>,
    src: Vec<u8>,
}

fn c_huf_fixture(src: &[u8]) -> HufFix {
    let l = &pair().c;
    let mut count = vec![0u32; 1024];
    let mut msv = 255u32;
    unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let (ct, log) = c_huf_ctable(&count, msv, 11);
    let hdr = {
        let (r, b) = write_ctable(l, &ct, msv, log, 512, HUF_CTABLE_WORKSPACE_SIZE, 0);
        match r {
            R::Ok(n) => b.0[..n].to_vec(),
            e => panic!("fixture writeCTable failed: {e:?}"),
        }
    };
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
        match res(l, n) {
            R::Ok(v) if v > 0 => d[..v].to_vec(),
            other => panic!("fixture {sym} gave {other:?}"),
        }
    };
    let c1x = enc("HUF_compress1X_usingCTable");
    let c4x = enc("HUF_compress4X_usingCTable");
    HufFix {
        hdr,
        c1x,
        c4x,
        src: src.to_vec(),
    }
}

#[allow(clippy::too_many_arguments)]
fn huf_decompress_using_dtable(
    l: &Lib,
    four: bool,
    hdr: &[u8],
    stream: &[u8],
    c_src_size: usize,
    dst_size: usize,
    dst_alloc: usize,
    cap_log: u32,
    flags: c_int,
) -> (R, Blob) {
    // The DTable is read from the serialised header first; decoding with an
    // unpopulated DTable (tableLog == 0) is OUT OF CONTRACT — see the note on
    // `huf_dtable()` — so if the read fails we stop and report only that.
    let mut dt = huf_dtable_cap(cap_log);
    let mut dw = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
    let rd = unsafe {
        l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
            dt.as_mut_ptr(),
            hdr.as_ptr() as *const c_void,
            hdr.len(),
            wp(&mut dw),
            HUF_DECOMPRESS_WORKSPACE_SIZE,
            flags,
        )
    };
    if !matches!(res(l, rd), R::Ok(_)) {
        return (res(l, rd), Blob(vec![]));
    }
    let mut dst = vec![0xA1u8; dst_alloc.max(1)];
    let sym = if four {
        "HUF_decompress4X_usingDTable"
    } else {
        "HUF_decompress1X_usingDTable"
    };
    let n = unsafe {
        l.sym::<FnHufDecompressUsingDTable>(sym)(
            dst.as_mut_ptr() as *mut c_void,
            dst_size,
            stream.as_ptr() as *const c_void,
            c_src_size,
            dt.as_ptr(),
            flags,
        )
    };
    (res(l, n), Blob(dst))
}

#[test]
fn huf_decompress_using_dtable_rejects_truncated_and_overlong_streams() {
    let src = small_alphabet_src(2048, 32, 0x11_0036);
    let fix = c_huf_fixture(&src);

    // dstSize == 0, and every truncation of the 1-stream and 4-stream bitstreams.
    // Row 40 (bitstream.h:451): BIT_endOfDStream refuses a stream that is not
    // consumed bit-exactly, which the callers turn into corruption_detected.
    // Row 36 (bitstream.h:402): BIT_reloadDStreamFast reports overflow on a
    // truncated 4-stream block.
    let mut corrupt_1x = 0usize;
    let mut corrupt_4x = 0usize;
    for flags in [0, HUF_flags_disableAsm, HUF_flags_disableFast] {
        for dst_size in [0usize, 1, 5, 6, 7, 100, src.len() - 1, src.len(), src.len() + 1] {
            for four in [false, true] {
                let stream = if four { &fix.c4x } else { &fix.c1x };
                let got = diff_bytes(
                    &format!(
                        "huf dec four={four} dstSize={dst_size} flags={flags} full"
                    ),
                    |l| {
                        huf_decompress_using_dtable(
                            l,
                            four,
                            &fix.hdr,
                            stream,
                            stream.len(),
                            dst_size,
                            src.len() + 64,
                            ZSTD_HUFFDTABLE_CAPACITY_LOG,
                            flags,
                        )
                    },
                );
                if four && (stream.len() < 10 || dst_size < 6) {
                    assert_eq!(code_of(&got.0), 20, "4-stream minima");
                }
            }
        }
        for four in [false, true] {
            let stream = if four { &fix.c4x } else { &fix.c1x };
            for n in 0..=stream.len().min(48) {
                let got = diff_bytes(
                    &format!("huf dec four={four} cSrcSize={n} flags={flags}"),
                    |l| {
                        huf_decompress_using_dtable(
                            l,
                            four,
                            &fix.hdr,
                            stream,
                            n,
                            src.len(),
                            src.len() + 64,
                            ZSTD_HUFFDTABLE_CAPACITY_LOG,
                            flags,
                        )
                    },
                );
                if is_err_code(&got.0, 20) {
                    if four {
                        corrupt_4x += 1;
                    } else {
                        corrupt_1x += 1;
                    }
                }
            }
        }
    }
    assert!(corrupt_1x > 0, "no 1-stream truncation reached bitstream.h:451");
    assert!(corrupt_4x > 0, "no 4-stream truncation reached bitstream.h:402");
    eprintln!("BIT_endOfDStream/reloadDStreamFast: {corrupt_1x} 1X + {corrupt_4x} 4X rejections");

    // A DTable whose declared capacity is too small for the encoded tree.
    // HUF_readDTableX1_wksp first *rescales* the tree down to
    // MIN(maxTableLog, HUF_DECODER_FAST_TABLELOG), so a small capacity is
    // accepted; HUF_readDTableX2_wksp has no rescaling and rejects it.
    for cap_log in 0..=12u32 {
        diff(&format!("readDTable capLog={cap_log}"), |l| {
            let mut dt1 = huf_dtable_cap(cap_log);
            let mut dt2 = huf_dtable_cap(cap_log);
            let mut w = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
            let a = unsafe {
                l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
                    dt1.as_mut_ptr(),
                    fix.hdr.as_ptr() as *const c_void,
                    fix.hdr.len(),
                    wp(&mut w),
                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                    0,
                )
            };
            let b = unsafe {
                l.sym::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp")(
                    dt2.as_mut_ptr(),
                    fix.hdr.as_ptr() as *const c_void,
                    fix.hdr.len(),
                    wp(&mut w),
                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                    0,
                )
            };
            (res(l, a), dt1[0], res(l, b), dt2[0])
        });
    }
    // wkspSize too small for either DTable reader.
    for ws in [0usize, 1, 64, 1024, HUF_DECOMPRESS_WORKSPACE_SIZE - 1, HUF_DECOMPRESS_WORKSPACE_SIZE] {
        diff(&format!("readDTable ws={ws}"), |l| {
            let mut dt1 = huf_dtable();
            let mut dt2 = huf_dtable();
            let mut w = wksp(ws.max(1));
            let a = unsafe {
                l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
                    dt1.as_mut_ptr(),
                    fix.hdr.as_ptr() as *const c_void,
                    fix.hdr.len(),
                    wp(&mut w),
                    ws,
                    0,
                )
            };
            let b = unsafe {
                l.sym::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp")(
                    dt2.as_mut_ptr(),
                    fix.hdr.as_ptr() as *const c_void,
                    fix.hdr.len(),
                    wp(&mut w),
                    ws,
                    0,
                )
            };
            (res(l, a), res(l, b))
        });
    }
    covers(&["ERR:common/bitstream.h:402", "ERR:common/bitstream.h:451"]);
}

#[test]
fn huf_decompress_dctx_wksp_entry_point_validation() {
    let src = small_alphabet_src(2048, 32, 0x11_0040);
    let fix = c_huf_fixture(&src);
    let mut frame1 = fix.hdr.clone();
    frame1.extend_from_slice(&fix.c1x);
    let mut frame4 = fix.hdr.clone();
    frame4.extend_from_slice(&fix.c4x);

    let entries = [
        "HUF_decompress1X_DCtx_wksp",
        "HUF_decompress1X1_DCtx_wksp",
        "HUF_decompress1X2_DCtx_wksp",
        "HUF_decompress4X_hufOnly_wksp",
    ];
    for sym in entries {
        let four = sym.contains("4X");
        let frame: &Vec<u8> = if four { &frame4 } else { &frame1 };
        for dst_size in [0usize, 1, 5, 6, 7, 100, src.len(), src.len() + 1] {
            for c_src in [
                0usize,
                1,
                2,
                9,
                10,
                fix.hdr.len(),
                dst_size,
                frame.len().min(dst_size + 1),
                frame.len(),
            ] {
                if c_src > frame.len() {
                    continue;
                }
                for ws in [0usize, 1024, HUF_DECOMPRESS_WORKSPACE_SIZE] {
                    diff_bytes(
                        &format!("{sym} dst={dst_size} cSrc={c_src} ws={ws}"),
                        |l| {
                            let mut dt = huf_dtable();
                            let mut w = wksp(ws.max(1));
                            let mut dst = vec![0xC1u8; src.len() + 64];
                            let n = unsafe {
                                l.sym::<FnHufDecompressDCtxWksp>(sym)(
                                    dt.as_mut_ptr(),
                                    dst.as_mut_ptr() as *mut c_void,
                                    dst_size,
                                    frame.as_ptr() as *const c_void,
                                    c_src,
                                    wp(&mut w),
                                    ws,
                                    0,
                                )
                            };
                            (res(l, n), Blob(dst))
                        },
                    );
                }
            }
        }
    }
    // HUF_selectDecoder over the boundary grid (it is the dispatcher the DCtx
    // entry points use, and has no error return).
    for dst in [1usize, 2, 3, 6, 100, 1000, 65536, HUF_BLOCKSIZE_MAX] {
        for csrc in [1usize, 2, 3, 10, 100, 1000, 60000, 131072] {
            diff(&format!("HUF_selectDecoder({dst},{csrc})"), |l| unsafe {
                l.sym::<FnHufSelectDecoder>("HUF_selectDecoder")(dst, csrc)
            });
        }
    }
    covers(&["ERR:common/bitstream.h:402", "ERR:common/bitstream.h:451"]);
}

// ===========================================================================
// compress/zstd_compress_literals.c
//   rows 366 (:46), 367 (:86), 368 (:154), 369 (:158), 370 (:161),
//        371 (:188), 372 (:198)
// ===========================================================================

/// `ZSTD_hufCTables_t` = `HUF_CElt CTable[257]` (2056 bytes) followed by
/// `HUF_repeat repeatMode` at offset 2056, padded to 2064 bytes.
const HUF_CTABLES_U64: usize = 258;
const REPEAT_MODE_IDX: usize = 257;

fn huf_ctables(repeat_mode: u32) -> Vec<u64> {
    let mut v = vec![0u64; HUF_CTABLES_U64];
    v[REPEAT_MODE_IDX] = repeat_mode as u64;
    v
}

#[test]
fn no_compress_literals_rejects_small_destination() {
    // Row 366 (:46): srcSize + flSize > dstCapacity, where
    // flSize = 1 + (srcSize>31) + (srcSize>4095).
    for n in [0usize, 1, 5, 31, 32, 100, 4095, 4096, 5000] {
        let src = vec![0x5Au8; n];
        let fl = 1 + (n > 31) as usize + (n > 4095) as usize;
        for cap in [0usize, 1, fl, n, n + fl - 1, n + fl, n + fl + 1] {
            let got = diff_bytes(
                &format!("noCompressLiterals n={n} cap={cap} (flSize {fl})"),
                |l| {
                    let mut dst = vec![0xEEu8; cap + 64];
                    let r = unsafe {
                        l.sym::<FnLiteralsSimple>("ZSTD_noCompressLiterals")(
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            n,
                        )
                    };
                    (res(l, r), Blob(dst))
                },
            );
            if n + fl > cap {
                assert_eq!(code_of(&got.0), 70, "row 366: dstSize_tooSmall");
            } else {
                assert_eq!(got.0, R::Ok(n + fl), "raw literals block size");
            }
        }
    }
    covers(&["ERR:compress/zstd_compress_literals.c:46"]);
}

#[test]
fn compress_rle_literals_block_has_no_capacity_check() {
    // Row 367 (:86): the ONLY guard is `assert(dstCapacity >= 4)`, compiled out at
    // DEBUGLEVEL=0, so up to flSize+1 == 4 bytes are written unconditionally.
    // To compare that missing check WITHOUT corrupting the heap, `dst` is always
    // allocated with 64 bytes of slack while a smaller `dstCapacity` is declared:
    // the "out of bounds" bytes land inside memory this test owns and show up in
    // the diffed buffer.
    //
    // PRECONDITION respected: `assert(allBytesIdentical(src, srcSize))` — every
    // source below is a single repeated byte.
    for n in [1usize, 2, 31, 32, 100, 4095, 4096, 5000] {
        for cap in [0usize, 1, 2, 3, 4, 5] {
            let src = vec![0xA5u8; n];
            diff_bytes(&format!("rleLiteralsBlock n={n} cap={cap}"), |l| {
                let mut dst = vec![0xEEu8; 64];
                let r = unsafe {
                    l.sym::<FnLiteralsSimple>("ZSTD_compressRleLiteralsBlock")(
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                    )
                };
                (res(l, r), Blob(dst))
            });
        }
    }
    covers(&["ERR:compress/zstd_compress_literals.c:86"]);
}

#[allow(clippy::too_many_arguments)]
fn compress_literals(
    l: &Lib,
    src: &[u8],
    cap: usize,
    wksp_size: usize,
    prev_repeat: u32,
    strategy: c_int,
    disable: c_int,
    suspect: c_int,
) -> (R, Blob, Blob) {
    let mut dst = vec![0xEEu8; cap + 64];
    let prev = huf_ctables(prev_repeat);
    let mut next = huf_ctables(0xDEAD_BEEF);
    let mut w = wksp(wksp_size.max(1));
    let n = unsafe {
        l.sym::<FnCompressLiterals>("ZSTD_compressLiterals")(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            wp(&mut w),
            wksp_size,
            prev.as_ptr(),
            next.as_mut_ptr(),
            strategy,
            disable,
            suspect,
            0,
        )
    };
    (res(l, n), Blob(dst), u64s(&next))
}

#[test]
fn compress_literals_fallbacks_and_capacity_checks() {
    // Row 368 (:154): disableLiteralCompression != 0 delegates straight to
    // ZSTD_noCompressLiterals, so its dstSize_tooSmall (70) shows through.
    for n in [0usize, 1, 40, 100, 5000] {
        let src = small_alphabet_src(n, 16, 0x11_0368);
        for cap in [0usize, 2, 4, n + 1, n + 8] {
            for disable in [1i32, 2, -1] {
                let got = diff_bytes(
                    &format!("compressLiterals disable={disable} n={n} cap={cap}"),
                    |l| {
                        compress_literals(
                            l,
                            &src,
                            cap,
                            HUF_WORKSPACE_SIZE,
                            0,
                            ZSTD_greedy,
                            disable,
                            0,
                        )
                    },
                );
                let fl = 1 + (n > 31) as usize + (n > 4095) as usize;
                if n + fl > cap {
                    assert_eq!(code_of(&got.0), 70, "row 368 -> row 366");
                }
            }
        }
    }
    // Row 369 (:158): srcSize < ZSTD_minLiteralsToCompress(strategy, repeatMode)
    // = 6 when repeatMode == HUF_repeat_valid (2), else 8 << MIN(9-strategy,3)
    // (64 for strategies 1..6, 32 for 7, 16 for 8, 8 for 9) -> raw literals.
    for strategy in ALL_STRATEGIES {
        for prev_repeat in [0u32, 1, 2] {
            let min = if prev_repeat == 2 {
                6usize
            } else {
                8usize << (9 - *strategy).min(3)
            };
            for n in [min.saturating_sub(1), min, min + 1] {
                let src = small_alphabet_src(n, 16, 0x11_0369);
                let got = diff_bytes(
                    &format!("compressLiterals strat={strategy} rep={prev_repeat} n={n} (min {min})"),
                    |l| {
                        compress_literals(
                            l,
                            &src,
                            n + 64,
                            HUF_WORKSPACE_SIZE,
                            prev_repeat,
                            *strategy,
                            0,
                            0,
                        )
                    },
                );
                if n < min {
                    let fl = 1 + (n > 31) as usize + (n > 4095) as usize;
                    assert_eq!(got.0, R::Ok(n + fl), "row 369: raw literals fallback");
                }
            }
        }
    }
    // Row 370 (:161): dstCapacity < lhSize+1, lhSize = 3 + (srcSize>=1024) +
    // (srcSize>=16384). Use a large srcSize so :158 does not fire first.
    for n in [100usize, 1023, 1024, 16383, 16384, 20000] {
        let lh = 3 + (n >= 1024) as usize + (n >= 16384) as usize;
        let src = small_alphabet_src(n, 16, 0x11_0370);
        for cap in [0usize, 1, 2, 3, lh, lh + 1] {
            let got = diff_bytes(
                &format!("compressLiterals lh n={n} cap={cap} (lhSize {lh})"),
                |l| {
                    compress_literals(
                        l,
                        &src,
                        cap,
                        HUF_WORKSPACE_SIZE,
                        0,
                        ZSTD_btultra2,
                        0,
                        0,
                    )
                },
            );
            if cap < lh + 1 {
                assert_eq!(code_of(&got.0), 70, "row 370: dstSize_tooSmall");
            }
        }
    }
    // Row 371 (:188): the Huffman result is unusable (0, no better than raw, or an
    // error) -> nextHuf is restored from prevHuf and the raw block is emitted. The
    // Huffman ERROR is swallowed, which the wkspSize sweep proves: a workspace
    // below sizeof(HUF_compress_tables_t) makes HUF_compress*_repeat return
    // workSpace_tooSmall, yet ZSTD_compressLiterals still succeeds.
    for ws in [0usize, 1024, 4096, 7000, 8192, HUF_WORKSPACE_SIZE] {
        for n in [200usize, 4096, 20000] {
            let src = corpus(Corpus::Random, n, 0x11_0371);
            let got = diff_bytes(
                &format!("compressLiterals swallow ws={ws} n={n}"),
                |l| {
                    compress_literals(
                        l,
                        &src,
                        n + 64,
                        ws,
                        0,
                        ZSTD_btultra2,
                        0,
                        0,
                    )
                },
            );
            let fl = 1 + (n > 31) as usize + (n > 4095) as usize;
            assert_eq!(
                got.0,
                R::Ok(n + fl),
                "row 371: Huffman failure swallowed, raw block emitted"
            );
        }
    }
    // Row 372 (:198): cLitSize == 1 (single-symbol alphabet) AND
    // (srcSize >= 8 || all bytes identical) -> ZSTD_compressRleLiteralsBlock.
    for n in [64usize, 100, 4096, 20000] {
        let src = vec![0x5Au8; n];
        let fl = 1 + (n > 31) as usize + (n > 4095) as usize;
        let got = diff_bytes(&format!("compressLiterals rle n={n}"), |l| {
            compress_literals(l, &src, n + 64, HUF_WORKSPACE_SIZE, 0, ZSTD_btultra2, 0, 0)
        });
        assert_eq!(got.0, R::Ok(fl + 1), "row 372: RLE literals block");
    }
    // suspectUncompressible on / off across sizes and strategies
    let mut rng = Rng::new(0x11_0000_0372);
    for i in 0..200 {
        let n = rng.below(30000);
        let k = *rng.pick(ALL_CORPORA);
        let src = corpus(k, n, rng.next_u64());
        let strategy = *rng.pick(ALL_STRATEGIES);
        let prev_repeat = rng.below(3) as u32;
        let cap = *rng.pick(&[0usize, 3, 4, 5, 16, n / 2 + 8, n + 64]);
        let suspect = (rng.below(2)) as c_int;
        diff_bytes(
            &format!("compressLiterals fuzz #{i} n={n} {k:?} strat={strategy} cap={cap}"),
            |l| {
                compress_literals(
                    l,
                    &src,
                    cap,
                    HUF_WORKSPACE_SIZE,
                    prev_repeat,
                    strategy,
                    0,
                    suspect,
                )
            },
        );
    }
    covers(&[
        "ERR:compress/zstd_compress_literals.c:154",
        "ERR:compress/zstd_compress_literals.c:158",
        "ERR:compress/zstd_compress_literals.c:161",
        "ERR:compress/zstd_compress_literals.c:188",
        "ERR:compress/zstd_compress_literals.c:198",
    ]);
}

// ===========================================================================
// compress/zstd_compress_sequences.c
//   rows 373 (:76), 374 (:117), 375 (:127), 376 (:206), 377 (:207),
//        378 (:166), 379 (:258), 380 (:265), 383 (:281), 384 (:282),
//        385 (:286), 386 (:303), 387 (:311), 388 (:379)
//   row 381 (:271, nbSeq==0 with set_compressed) is EXCLUDED — see the comment
//   on `build_ctable_compressed_nbseq0` below.
//   row 382 (:278) is marked UNREACHABLE in ERRORS.md and auto-excluded.
// ===========================================================================

const FSE_CT_LEN: usize = 4096; // U32s: covers FSE_CTABLE_SIZE_U32(12, 255)

/// A CTable over `0..=msv` built from a uniform normalized distribution, plus
/// one built with `norm[hole] == 0` so a *present* symbol has probability zero.
fn c_seq_ctable(msv: u32, table_log: u32, hole: Option<usize>) -> Vec<u32> {
    let l = &pair().c;
    let mut norm = norm_uniform(msv, table_log);
    if let Some(h) = hole {
        let moved = norm[h];
        norm[h] = 0;
        // give the freed states to symbol 0 so the distribution still sums right
        norm[0] += moved;
    }
    let mut ct = vec![0u32; FSE_CT_LEN];
    let mut w = wksp(1 << 16);
    let n = unsafe {
        l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
            ct.as_mut_ptr(),
            norm.as_ptr(),
            msv,
            table_log,
            wp(&mut w),
            (1usize << 16) as SizeT,
        )
    };
    assert!(matches!(res(l, n), R::Ok(_)), "fixture seq CTable failed");
    ct
}

#[test]
fn fse_bit_cost_and_cross_entropy_cost_reject_incapable_tables() {
    // Row 374 (:117): ZSTD_getFSEMaxSymbolValue(ctable) < max -> GENERIC. The
    // guard fires BEFORE any symbolTT indexing, so a `max` far above the table's
    // own alphabet is safe.
    let ct10 = c_seq_ctable(10, 6, None);
    let mut count = vec![0u32; 1024];
    for s in 0..=10usize {
        count[s] = 10;
    }
    for max in [0u32, 5, 10, 11, 30, 52, 200, 255] {
        let got = diff(&format!("fseBitCost max={max}"), |l| {
            res(l, unsafe {
                l.sym::<FnFseBitCost>("ZSTD_fseBitCost")(ct10.as_ptr(), count.as_ptr(), max)
            })
        });
        if max > 10 {
            assert_eq!(code_of(&got), 1, "row 374: GENERIC");
        } else {
            assert_eq!(code_of(&got), 0, "cost for max={max}");
        }
    }
    // Row 375 (:127): a symbol with count != 0 whose CTable probability is 0 gets
    // `bitCost >= (tableLog+1) << 8` -> GENERIC.
    for hole in [1usize, 3, 7, 10] {
        let ct = c_seq_ctable(10, 6, Some(hole));
        let got = diff(&format!("fseBitCost hole={hole}"), |l| {
            res(l, unsafe {
                l.sym::<FnFseBitCost>("ZSTD_fseBitCost")(ct.as_ptr(), count.as_ptr(), 10)
            })
        });
        assert_eq!(code_of(&got), 1, "row 375: GENERIC (Prob[{hole}] == 0)");
        // ... and with that symbol absent from `count`, the same table is fine.
        let mut c2 = count.clone();
        c2[hole] = 0;
        let got = diff(&format!("fseBitCost hole={hole} absent"), |l| {
            res(l, unsafe {
                l.sym::<FnFseBitCost>("ZSTD_fseBitCost")(ct.as_ptr(), c2.as_ptr(), 10)
            })
        });
        assert_eq!(code_of(&got), 0, "zero-probability symbol is absent -> fine");
    }
    // ZSTD_crossEntropyCost over the same grid. PRECONDITION: `accuracyLog <= 8`
    // and `norm[s] << (8-accuracyLog) < 256`, both asserted only — the norms below
    // respect them (values <= 2 at accuracyLog 6).
    let norm = norm_uniform(52, 6);
    for max in [0u32, 5, 10, 35, 52] {
        for acc in [5u32, 6, 8] {
            if norm[..=max as usize].iter().any(|&v| (v as u32) << (8 - acc) >= 256) {
                continue;
            }
            diff(&format!("crossEntropyCost max={max} acc={acc}"), |l| {
                res(l, unsafe {
                    l.sym::<FnCrossEntropyCost>("ZSTD_crossEntropyCost")(
                        norm.as_ptr(),
                        acc,
                        count.as_ptr(),
                        max,
                    )
                })
            });
        }
    }
    covers(&[
        "ERR:compress/zstd_compress_sequences.c:117",
        "ERR:compress/zstd_compress_sequences.c:127",
    ]);
}

#[test]
fn select_encoding_type_sentinels_and_rle() {
    // PRECONDITIONS honoured throughout:
    //  * `max <= MaxSeq (52)` — `ZSTD_NCountCost`'s `S16 norm[MaxSeq+1]` is a
    //    fixed 53-entry stack array that `FSE_normalizeCount` writes `max+1`
    //    entries of, so a larger `max` is an out-of-bounds stack write.
    //  * `nbSeq >= 2` — `FSE_optimalTableLog_internal` computes
    //    `ZSTD_highbit32(srcSize-1)` and `FSE_minTableLog` computes
    //    `ZSTD_highbit32(srcSize)`, both undefined at 0.
    //  * `mostFrequent == max(count)` and `nbSeq == sum(count)`, so either the
    //    `mostFrequent == nbSeq` early return fires or every `count[s] < nbSeq`
    //    (which `ZSTD_entropyCost` needs to keep its 256-entry table index in
    //    range).
    let default_norm = norm_uniform(52, 6);
    let prev = c_seq_ctable(52, 9, None);

    let cases: Vec<(&str, Vec<u32>, u32)> = vec![
        ("single", {
            let mut c = vec![0u32; 1024];
            c[3] = 5;
            c
        }, 5u32),
        ("two", {
            let mut c = vec![0u32; 1024];
            c[0] = 1;
            c[1] = 1;
            c
        }, 2),
        ("flat53", {
            let mut c = vec![0u32; 1024];
            for s in 0..=52usize {
                c[s] = 40;
            }
            c
        }, 40 * 53),
        ("skewed", {
            let mut c = vec![0u32; 1024];
            c[0] = 4000;
            for s in 1..=52usize {
                c[s] = 1;
            }
            c
        }, 4052),
    ];
    for (name, count, nb_seq) in cases.iter() {
        let most = *count[..=52].iter().max().unwrap() as usize;
        for max in [0u32, 5, 35, 52] {
            for fse_log in [OffFSELog, LLFSELog, MLFSELog] {
                for strategy in ALL_STRATEGIES {
                    for allowed in [ZSTD_defaultDisallowed, ZSTD_defaultAllowed] {
                        for rep_in in [FSE_repeat_none, FSE_repeat_check, FSE_repeat_valid, 7] {
                            diff(
                                &format!(
                                    "selectEncodingType {name} max={max} log={fse_log} strat={strategy} allowed={allowed} rep={rep_in}"
                                ),
                                |l| {
                                    let mut rm = rep_in;
                                    let t = unsafe {
                                        l.sym::<FnSelectEncodingType>("ZSTD_selectEncodingType")(
                                            &mut rm,
                                            count.as_ptr(),
                                            max,
                                            most,
                                            *nb_seq as SizeT,
                                            fse_log,
                                            prev.as_ptr(),
                                            default_norm.as_ptr(),
                                            6,
                                            allowed,
                                            *strategy,
                                        )
                                    };
                                    (t, rm)
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    // Row 378 (:166) explicitly: mostFrequent == nbSeq selects set_rle, or
    // set_basic when isDefaultAllowed && nbSeq <= 2.
    for (nb, expect) in [(2usize, set_basic), (3, set_rle), (100, set_rle)] {
        let mut count = vec![0u32; 1024];
        count[4] = nb as u32;
        let got = diff(&format!("selectEncodingType rle nbSeq={nb}"), |l| {
            let mut rm = FSE_repeat_valid;
            let t = unsafe {
                l.sym::<FnSelectEncodingType>("ZSTD_selectEncodingType")(
                    &mut rm,
                    count.as_ptr(),
                    52,
                    nb,
                    nb,
                    LLFSELog,
                    prev.as_ptr(),
                    default_norm.as_ptr(),
                    6,
                    ZSTD_defaultAllowed,
                    ZSTD_btopt,
                )
            };
            (t, rm)
        });
        assert_eq!(got, (expect, FSE_repeat_none), "row 378");
    }
    covers(&[
        "ERR:compress/zstd_compress_sequences.c:76",
        "ERR:compress/zstd_compress_sequences.c:166",
        "ERR:compress/zstd_compress_sequences.c:206",
        "ERR:compress/zstd_compress_sequences.c:207",
    ]);
}

#[allow(clippy::too_many_arguments)]
fn build_ctable(
    l: &Lib,
    dst_cap: usize,
    fse_log: u32,
    ty: c_int,
    count: &[u32],
    max: u32,
    code_table: &[u8],
    nb_seq: usize,
    default_norm: &[i16],
    default_norm_log: u32,
    default_max: u32,
    prev: &[u32],
    wksp_size: usize,
) -> (R, Blob, Blob) {
    let mut dst = vec![0xEEu8; dst_cap + 128];
    let mut next = vec![0u32; FSE_CT_LEN];
    let mut cnt = count.to_vec();
    let mut w = wksp(8192);
    let n = unsafe {
        l.sym::<FnBuildCTable>("ZSTD_buildCTable")(
            dst.as_mut_ptr() as *mut c_void,
            dst_cap,
            next.as_mut_ptr(),
            fse_log,
            ty,
            cnt.as_mut_ptr(),
            max,
            code_table.as_ptr(),
            nb_seq,
            default_norm.as_ptr(),
            default_norm_log,
            default_max,
            prev.as_ptr(),
            (FSE_CT_LEN * 4) as SizeT,
            wp(&mut w),
            wksp_size,
        )
    };
    let mut all = Blob(dst).0;
    all.extend_from_slice(&u32s(&next).0);
    all.extend_from_slice(&u32s(&cnt).0);
    (res(l, n), Blob(all), Blob(vec![]))
}

#[test]
fn build_ctable_rejects_bad_type_capacity_and_workspace() {
    let default_norm = norm_uniform(35, 6);
    let prev = c_seq_ctable(52, 9, None);
    let mut count = vec![0u32; 1024];
    for s in 0..=35usize {
        count[s] = 10;
    }
    let nb_seq = 360usize;
    let code_table: Vec<u8> = (0..nb_seq).map(|i| (i % 36) as u8).collect();

    // Row 379 (:258): set_rle with dstCapacity == 0 -> dstSize_tooSmall (70).
    for cap in [0usize, 1, 2] {
        let got = diff_bytes(&format!("buildCTable rle cap={cap}"), |l| {
            build_ctable(
                l, cap, LLFSELog, set_rle, &count, 35, &code_table, nb_seq, &default_norm, 6,
                35, &prev, 4096,
            )
        });
        if cap == 0 {
            assert_eq!(code_of(&got.0), 70, "row 379: dstSize_tooSmall");
        } else {
            assert_eq!(got.0, R::Ok(1), "set_rle writes exactly one byte");
        }
    }
    // Row 380 (:265): set_basic with entropyWorkspaceSize below
    // FSE_BUILD_CTABLE_WORKSPACE_SIZE(defaultMax=35, defaultNormLog=6) == 208 ->
    // FSE_buildCTable_wksp's tableLog_tooLarge (44) is propagated.
    let need = fse_build_ctable_wksp_size(35, 6);
    assert_eq!(need, 208);
    for ws in [0usize, 1, 8, 200, need - 1, need, need + 8] {
        let got = diff_bytes(&format!("buildCTable basic ws={ws}"), |l| {
            build_ctable(
                l, 512, LLFSELog, set_basic, &count, 35, &code_table, nb_seq, &default_norm, 6,
                35, &prev, ws,
            )
        });
        if ws < need {
            assert_eq!(code_of(&got.0), 44, "row 380: tableLog_tooLarge");
        } else {
            assert_eq!(got.0, R::Ok(0), "set_basic writes nothing");
        }
    }
    // set_repeat just memcpys prevCTable and returns 0.
    let got = diff_bytes("buildCTable repeat", |l| {
        build_ctable(
            l, 512, LLFSELog, set_repeat, &count, 35, &code_table, nb_seq, &default_norm, 6, 35,
            &prev, 4096,
        )
    });
    assert_eq!(got.0, R::Ok(0), "set_repeat -> 0");

    // Row 383 (:281): set_compressed with a dstCapacity too small for the NCount
    // header -> FSE_writeNCount's dstSize_tooSmall (70).
    for cap in [0usize, 1, 2, 4, 8, 16, 512] {
        let got = diff_bytes(&format!("buildCTable compressed cap={cap}"), |l| {
            build_ctable(
                l, cap, LLFSELog, set_compressed, &count, 35, &code_table, nb_seq,
                &default_norm, 6, 35, &prev, 4096,
            )
        });
        if cap <= 8 {
            assert_eq!(code_of(&got.0), 70, "row 383: dstSize_tooSmall at cap={cap}");
        }
    }
    // Row 384 (:282): set_compressed with max > MaxSeq (52). `wksp->wksp` is a
    // fixed FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(52, 9) == 285 U32 == 1140 bytes,
    // which only covers max <= 52 at tableLog <= 9, so FSE_buildCTable_wksp
    // returns tableLog_tooLarge (44).
    //
    // NOTE: this input also makes `FSE_normalizeCount` write `max+1` S16 entries
    // into `wksp->norm`, a 53-entry field — i.e. 402 bytes into a struct whose
    // first field is 106 bytes. That write stays inside the 8192-byte workspace
    // this test owns (`build_ctable` always allocates 8192), so nothing outside
    // the test's own memory is touched.
    {
        let max = 200u32;
        let nb = 3000usize;
        let mut c = vec![0u32; 1024];
        let per = (nb / (max as usize + 1)) as u32;
        let mut tot = 0usize;
        for s in 0..=max as usize {
            c[s] = per;
            tot += per as usize;
        }
        c[0] += (nb - tot) as u32;
        let ct: Vec<u8> = (0..nb).map(|i| (i % (max as usize + 1)) as u8).collect();
        let got = diff_bytes("buildCTable compressed max=200", |l| {
            build_ctable(
                l, 512, MaxSeq.min(9), set_compressed, &c, max, &ct, nb, &default_norm, 6, 35,
                &prev, 4096,
            )
        });
        assert_eq!(code_of(&got.0), 44, "row 384: tableLog_tooLarge");
    }
    // Row 385 (:286): an out-of-range SymbolEncodingType_e. `assert(0)` is
    // compiled out at DEBUGLEVEL=0, so the RETURN_ERROR(GENERIC) executes.
    for ty in [4i32, 5, 100, -1, i32::MAX, i32::MIN] {
        let got = diff_bytes(&format!("buildCTable type={ty}"), |l| {
            build_ctable(
                l, 512, LLFSELog, ty, &count, 35, &code_table, nb_seq, &default_norm, 6, 35,
                &prev, 4096,
            )
        });
        assert_eq!(code_of(&got.0), 1, "row 385: GENERIC");
    }
    // Row 381 (:271) is NOT tested: `set_compressed` with `nbSeq == 0` reads
    // `codeTable[nbSeq-1]` == `codeTable[-1]` (which a leading pad could make
    // safe) but then calls `FSE_optimalTableLog(FSELog, 0, max)`, whose
    // `FSE_minTableLog` evaluates `ZSTD_highbit32(0)` -> `31 -
    // __builtin_clz(0)`, undefined behaviour with no defined C result to match.
    // See the report: row 381 should be UNSAFE-UB, not DIRECT.
    covers(&[
        "ERR:compress/zstd_compress_sequences.c:258",
        "ERR:compress/zstd_compress_sequences.c:265",
        "ERR:compress/zstd_compress_sequences.c:281",
        "ERR:compress/zstd_compress_sequences.c:282",
        "ERR:compress/zstd_compress_sequences.c:286",
    ]);
}

#[test]
fn encode_sequences_rejects_small_destination() {
    // ll/ml/of CTables over the codes 0..=5 at tableLog 6. Codes <= 5 are all
    // zero-extra-bit in LL_bits/ML_bits, and `ofCode` doubles as the number of
    // offset bits, so `offBase` is kept below 1<<5.
    let ct = c_seq_ctable(5, 6, None);

    // The code tables and the sequence array each carry ONE LEADING PAD element
    // and are passed as `ptr.add(1)`. That makes index -1 — which
    // `ZSTD_encodeSequences_body` reads unconditionally at :311..:328 — land on
    // real, initialised memory, so `nbSeq == 0` (row 387) can be exercised
    // without an out-of-bounds read. The `for (n=nbSeq-2; n<nbSeq; n--)` loop
    // body never runs at nbSeq==0 because `n < 0` is false for unsigned n.
    let nb_max = 1200usize;
    let mut ll = vec![0u8; nb_max + 1];
    let mut ml = vec![0u8; nb_max + 1];
    let mut of = vec![0u8; nb_max + 1];
    let mut seqs = vec![SeqDef::default(); nb_max + 1];
    for i in 0..=nb_max {
        ll[i] = (i % 6) as u8;
        ml[i] = ((i * 3) % 6) as u8;
        of[i] = ((i * 5) % 6) as u8;
        seqs[i] = SeqDef {
            off_base: 1 + (i as u32 % 30),
            lit_length: (i % 7) as u16,
            ml_base: (i % 11) as u16,
        };
    }

    let run = |nb_seq: usize, cap: usize, long_offsets: c_int| {
        let ct = ct.clone();
        let ll = ll.clone();
        let ml = ml.clone();
        let of = of.clone();
        let seqs = seqs.clone();
        move |l: &Lib| {
            let mut dst = vec![0xEEu8; cap + 128];
            let n = unsafe {
                l.sym::<FnEncodeSequences>("ZSTD_encodeSequences")(
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ct.as_ptr(),
                    ml.as_ptr().add(1),
                    ct.as_ptr(),
                    of.as_ptr().add(1),
                    ct.as_ptr(),
                    ll.as_ptr().add(1),
                    seqs.as_ptr().add(1),
                    nb_seq,
                    long_offsets,
                    0,
                )
            };
            (res(l, n), Blob(dst))
        }
    };

    // Row 386 (:303): BIT_initCStream fails when dstCapacity < sizeof(size_t) = 8
    // -> RETURN_ERROR_IF(dstSize_tooSmall) (70).
    for cap in 0..=9usize {
        let got = diff_bytes(&format!("encodeSequences cap={cap}"), run(5, cap, 0));
        if cap <= 8 {
            assert_eq!(code_of(&got.0), 70, "row 386: dstSize_tooSmall at cap={cap}");
        }
    }
    // Row 388 (:379): the sequence bitstream overflows dstCapacity, so
    // BIT_closeCStream returns 0 -> dstSize_tooSmall (70).
    let mut overflow = 0usize;
    for cap in [9usize, 10, 16, 32, 64, 128, 256, 512, 1024, 4096] {
        let got = diff_bytes(
            &format!("encodeSequences overflow nbSeq=1000 cap={cap}"),
            run(1000, cap, 0),
        );
        if is_err_code(&got.0, 70) {
            overflow += 1;
        }
    }
    assert!(overflow > 0, "no capacity reached zstd_compress_sequences.c:379");
    eprintln!("ZSTD_encodeSequences closeCStream==0 for {overflow} capacities");
    // Row 387 (:311): nbSeq == 0 — no error, the index -1 reads are the pad.
    for cap in [9usize, 16, 64, 512] {
        for long_offsets in [0i32, 1] {
            diff_bytes(
                &format!("encodeSequences nbSeq=0 cap={cap} long={long_offsets}"),
                run(0, cap, long_offsets),
            );
        }
    }
    // a general sweep of nbSeq x capacity x longOffsets
    for nb_seq in [1usize, 2, 3, 8, 100, 1000] {
        for cap in [9usize, 12, 20, 100, 4096] {
            for long_offsets in [0i32, 1] {
                diff_bytes(
                    &format!("encodeSequences nbSeq={nb_seq} cap={cap} long={long_offsets}"),
                    run(nb_seq, cap, long_offsets),
                );
            }
        }
    }
    covers(&[
        "ERR:compress/zstd_compress_sequences.c:303",
        "ERR:compress/zstd_compress_sequences.c:311",
        "ERR:compress/zstd_compress_sequences.c:379",
    ]);
}

// ===========================================================================
// common/pool.c, common/xxhash.c and common/allocations.h
//   row 45 (pool.c:366 POOL_sizeof(NULL)), row 49 (allocations.h:47
//   ZSTD_customFree(NULL) is a silent no-op), rows 51 and 53
//   (XXH{32,64}_update with a NULL input pointer)
// Rows 46, 48, 50 and 52 all require `malloc`/`calloc` itself to fail (OOM) and
// are NOT testable in-process — see the report.
// Row 47 (ZSTD_customCalloc with a failing customAlloc -> memset(NULL)) is
// undefined behaviour with no defined C result, and its only exported route
// (`ZSTD_createCCtxParams_advanced`) is not in the symbol table of this build.
// ===========================================================================

static ALLOC_N: AtomicUsize = AtomicUsize::new(0);
static FREE_N: AtomicUsize = AtomicUsize::new(0);
static NULL_FREES: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT: AtomicIsize = AtomicIsize::new(-1);
static ALLOC_SIZES: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const HDR: usize = 16;

extern "C" fn cust_alloc(_opaque: *mut c_void, size: SizeT) -> *mut c_void {
    let n = ALLOC_N.fetch_add(1, Ordering::SeqCst) as isize;
    ALLOC_SIZES.lock().unwrap().push(size);
    let fail = FAIL_AT.load(Ordering::SeqCst);
    if fail >= 0 && n >= fail {
        return std::ptr::null_mut();
    }
    let total = size + HDR;
    let layout = std::alloc::Layout::from_size_align(total, HDR).unwrap();
    let p = unsafe { std::alloc::alloc(layout) };
    if p.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { (p as *mut usize).write(total) };
    unsafe { p.add(HDR) as *mut c_void }
}

extern "C" fn cust_free(_opaque: *mut c_void, ptr: *mut c_void) {
    FREE_N.fetch_add(1, Ordering::SeqCst);
    if ptr.is_null() {
        // Row 49: `ZSTD_customFree` short-circuits on NULL and never invokes the
        // hook, so reaching here would be a divergence from the C.
        NULL_FREES.fetch_add(1, Ordering::SeqCst);
        return;
    }
    let base = unsafe { (ptr as *mut u8).sub(HDR) };
    let total = unsafe { (base as *mut usize).read() };
    let layout = std::alloc::Layout::from_size_align(total, HDR).unwrap();
    unsafe { std::alloc::dealloc(base, layout) };
}

fn cmem(fail_at: isize) -> ZSTD_customMem {
    ALLOC_N.store(0, Ordering::SeqCst);
    FREE_N.store(0, Ordering::SeqCst);
    NULL_FREES.store(0, Ordering::SeqCst);
    FAIL_AT.store(fail_at, Ordering::SeqCst);
    ALLOC_SIZES.lock().unwrap().clear();
    ZSTD_customMem {
        customAlloc: Some(cust_alloc),
        customFree: Some(cust_free),
        opaque: std::ptr::null_mut(),
    }
}

#[derive(PartialEq, Debug)]
struct AllocRun {
    created: bool,
    allocs: usize,
    frees: usize,
    null_frees: usize,
    sizes: Vec<usize>,
}

fn snapshot(created: bool) -> AllocRun {
    AllocRun {
        created,
        allocs: ALLOC_N.load(Ordering::SeqCst),
        frees: FREE_N.load(Ordering::SeqCst),
        null_frees: NULL_FREES.load(Ordering::SeqCst),
        sizes: ALLOC_SIZES.lock().unwrap().clone(),
    }
}

#[test]
fn custom_allocators_failing_and_counting() {
    let dict = corpus(Corpus::Text, 8192, 0x11_0046);

    // The six documented `*_advanced` constructors, each with (a) a counting
    // allocator that always succeeds and (b) an allocator that fails on the Nth
    // call. Comparing the NUMBER and the SIZES of the allocations is a strong
    // structural check that the two workspace layouts match.
    for fail_at in [-1isize, 0, 1, 2, 3] {
        diff(&format!("createCCtx_advanced fail_at={fail_at}"), |l| {
            let m = cmem(fail_at);
            let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createCCtx_advanced")(m) };
            let created = !p.is_null();
            if created {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCCtx")(p) };
            }
            snapshot(created)
        });
        diff(&format!("createDCtx_advanced fail_at={fail_at}"), |l| {
            let m = cmem(fail_at);
            let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced")(m) };
            let created = !p.is_null();
            if created {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDCtx")(p) };
            }
            snapshot(created)
        });
        diff(&format!("createCStream_advanced fail_at={fail_at}"), |l| {
            let m = cmem(fail_at);
            let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createCStream_advanced")(m) };
            let created = !p.is_null();
            if created {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCStream")(p) };
            }
            snapshot(created)
        });
        diff(&format!("createDStream_advanced fail_at={fail_at}"), |l| {
            let m = cmem(fail_at);
            let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced")(m) };
            let created = !p.is_null();
            if created {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDStream")(p) };
            }
            snapshot(created)
        });
        for level in [1i32, 3, 12, 19] {
            diff(
                &format!("createCDict_advanced level={level} fail_at={fail_at}"),
                |l| {
                    let cp = unsafe {
                        l.sym::<FnGetCParams>("ZSTD_getCParams")(level, dict.len() as u64, dict.len())
                    };
                    let m = cmem(fail_at);
                    let p = unsafe {
                        l.sym::<FnCreateCDictAdvanced>("ZSTD_createCDict_advanced")(
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            ZSTD_dlm_byCopy,
                            ZSTD_dct_auto,
                            cp,
                            m,
                        )
                    };
                    let created = !p.is_null();
                    if created {
                        unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCDict")(p) };
                    }
                    snapshot(created)
                },
            );
        }
        diff(&format!("createDDict_advanced fail_at={fail_at}"), |l| {
            let m = cmem(fail_at);
            let p = unsafe {
                l.sym::<FnCreateDDictAdvanced>("ZSTD_createDDict_advanced")(
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    ZSTD_dlm_byCopy,
                    ZSTD_dct_auto,
                    m,
                )
            };
            let created = !p.is_null();
            if created {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDDict")(p) };
            }
            snapshot(created)
        });
    }
    // A half-supplied `ZSTD_customMem` (exactly one of the two hooks) must be
    // rejected with NULL by every constructor.
    for (alloc, free) in [(true, false), (false, true)] {
        diff(&format!("half customMem alloc={alloc} free={free}"), |l| {
            let m = ZSTD_customMem {
                customAlloc: if alloc { Some(cust_alloc) } else { None },
                customFree: if free { Some(cust_free) } else { None },
                opaque: std::ptr::null_mut(),
            };
            let a = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createCCtx_advanced")(m) };
            let b = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced")(m) };
            let c = unsafe {
                l.sym::<FnCreateDDictAdvanced>("ZSTD_createDDict_advanced")(
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    ZSTD_dlm_byCopy,
                    ZSTD_dct_auto,
                    m,
                )
            };
            (a.is_null(), b.is_null(), c.is_null())
        });
    }
    // Row 49 (allocations.h:47): a full create + compress + free cycle with the
    // counting allocator. The workspace pointers a freshly-created CCtx has not
    // allocated yet are NULL, and `ZSTD_customFree` must NOT call the hook for
    // them — `null_frees` therefore has to be 0 in BOTH libraries, and the
    // alloc/free counts and sizes must match exactly.
    for level in [1i32, 3, 9, 19] {
        let src = corpus(Corpus::Text, 40000, 0x11_0049);
        diff(&format!("counting alloc compress level={level}"), |l| {
            let m = cmem(-1);
            let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createCCtx_advanced")(m) };
            assert!(!p.is_null());
            let mut dst = vec![0u8; compress_bound(l, src.len())];
            let n = unsafe {
                l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                    p,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    level,
                )
            };
            let r = res(l, n);
            unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCCtx")(p) };
            let s = snapshot(true);
            assert_eq!(s.null_frees, 0, "[{}] customFree called with NULL", l.tag);
            (r, s)
        });
    }
    // Row 441 (compress/zstd_cwksp.h:692): ZSTD_cwksp_create's
    // `ZSTD_customMalloc(size, customMem) == NULL` -> memory_allocation (64).
    // The context object is allocation #0, the cwksp is a later one, so failing
    // at 1..4 exercises the workspace-allocation failure path of a real compress.
    for fail_at in [1isize, 2, 3, 4] {
        for level in [1i32, 3, 19] {
            let src = corpus(Corpus::Text, 20000, 0x11_0441);
            diff(
                &format!("cwksp_create failure fail_at={fail_at} level={level}"),
                |l| {
                    let m = cmem(fail_at);
                    let p = unsafe { l.sym::<FnCreateAdvanced>("ZSTD_createCCtx_advanced")(m) };
                    if p.is_null() {
                        return (R::Ok(usize::MAX), snapshot(false));
                    }
                    let mut dst = vec![0u8; compress_bound(l, src.len())];
                    let n = unsafe {
                        l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                            p,
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            level,
                        )
                    };
                    let r = res(l, n);
                    unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeCCtx")(p) };
                    (r, snapshot(true))
                },
            );
        }
    }
    // Row 46 (allocations.h:28): `customMem.customAlloc == NULL` falls through to
    // `ZSTD_malloc(size)` = `malloc(size)`, which returns NULL "on OOM (or for
    // absurd `size`)". `ZSTD_createDDict_advanced` with the DEFAULT allocator and
    // an absurd `dictSize` is the one public route where the requested size is
    // caller-controlled: `ZSTD_initDDict_internal` calls
    // `ZSTD_customMalloc(dictSize, ...)` *before* reading a single dictionary
    // byte, so a bogus-but-valid `dict` pointer is never dereferenced.
    for shift in [48u32, 56, 62, 63] {
        let dict_size = 1usize << shift;
        diff(&format!("createDDict_advanced absurd dictSize=1<<{shift}"), |l| {
            let one = [0u8; 8];
            let p = unsafe {
                l.sym::<FnCreateDDictAdvanced>("ZSTD_createDDict_advanced")(
                    one.as_ptr() as *const c_void,
                    dict_size,
                    ZSTD_dlm_byCopy,
                    ZSTD_dct_rawContent,
                    ZSTD_customMem::default(),
                )
            };
            let null = p.is_null();
            if !null {
                unsafe { l.sym::<FnFreeCCtx>("ZSTD_freeDDict")(p) };
            }
            null
        });
    }
    covers(&[
        "ERR:common/allocations.h:28",
        "ERR:common/allocations.h:47",
        "ERR:compress/zstd_cwksp.h:692",
    ]);
}

#[test]
fn pool_and_xxhash_null_tolerant_entry_points() {
    // Row 45 (pool.c:366): POOL_sizeof(NULL) is documented as supported -> 0.
    diff("POOL_sizeof(NULL)", |l| unsafe {
        l.sym::<FnPoolSizeof>("POOL_sizeof")(std::ptr::null_mut())
    });
    diff("POOL_free(NULL)/resize(NULL)", |l| unsafe {
        l.sym::<FnPoolFree>("POOL_free")(std::ptr::null_mut());
        l.sym::<FnPoolResize>("POOL_resize")(std::ptr::null_mut(), 2)
    });
    // Rows 51 (xxhash.h:3130) and 53 (:3575): `input == NULL` is silently
    // accepted for ANY length, because the `XXH_ASSERT(len == 0)` is compiled out
    // at XXH_DEBUGLEVEL==0 — the call returns XXH_OK and leaves the state
    // completely unchanged, which the digest before/after proves.
    for len in [0usize, 1, 16, 32, 4096] {
        diff(&format!("XXH_update(NULL, {len})"), |l| unsafe {
            let s32 = l.sym::<FnXxhCreateState>("ZSTD_XXH32_createState")();
            let s64 = l.sym::<FnXxhCreateState>("ZSTD_XXH64_createState")();
            assert!(!s32.is_null() && !s64.is_null());
            let r32 = l.sym::<FnXxh32Reset>("ZSTD_XXH32_reset")(s32, 0x1234);
            let r64 = l.sym::<FnXxh64Reset>("ZSTD_XXH64_reset")(s64, 0x9876);
            let before32 = l.sym::<FnXxh32Digest>("ZSTD_XXH32_digest")(s32);
            let before64 = l.sym::<FnXxh64Digest>("ZSTD_XXH64_digest")(s64);
            let u32r = l.sym::<FnXxhUpdate>("ZSTD_XXH32_update")(s32, std::ptr::null(), len);
            let u64r = l.sym::<FnXxhUpdate>("ZSTD_XXH64_update")(s64, std::ptr::null(), len);
            let after32 = l.sym::<FnXxh32Digest>("ZSTD_XXH32_digest")(s32);
            let after64 = l.sym::<FnXxh64Digest>("ZSTD_XXH64_digest")(s64);
            let f = (
                l.sym::<FnXxhFreeState>("ZSTD_XXH32_freeState")(s32),
                l.sym::<FnXxhFreeState>("ZSTD_XXH64_freeState")(s64),
                // XXH*_freeState(NULL) is in contract (it is just free(NULL)).
                l.sym::<FnXxhFreeState>("ZSTD_XXH32_freeState")(std::ptr::null_mut()),
                l.sym::<FnXxhFreeState>("ZSTD_XXH64_freeState")(std::ptr::null_mut()),
            );
            (
                r32, r64, before32, before64, u32r, u64r, after32, after64, f,
            )
        });
    }
    // NOTE: `XXH{32,64}_reset`, `_update` and `_digest` document
    // `@pre statePtr must not be NULL` and only `XXH_ASSERT` it, which is
    // compiled out here. A NULL state therefore dereferences NULL in the
    // reference C (SIGSEGV) — out of contract, so those calls are NOT made.
    covers(&[
        "ERR:common/pool.c:366",
        "ERR:common/xxhash.h:3130 (via common/xxhash.c)",
        "ERR:common/xxhash.h:3575 (via common/xxhash.c)",
    ]);
}

// ===========================================================================
// compress/zstdmt_compress.c — with ZSTD_MULTITHREAD undefined
//   rows 445 (:1000), 446 (:1046), 447 (:1064)
// Row 448 (:1096 / :1842 / :1853 / ZSTDMT_getFrameProgression /
// ZSTDMT_toFlushNow) is NOT tested: none of those five exports has a NULL check,
// and since `ZSTDMT_createCCtx_advanced` returns NULL unconditionally in this
// build there is no other pointer to give them — the row's own concrete input
// (`ZSTDMT_nextInputSizeHint(NULL)`) is an immediate NULL dereference. See the
// report: row 448 should be UNSAFE-UB, not DIRECT.
// ===========================================================================

#[test]
fn zstdmt_stubs_without_multithread_support() {
    // Row 445 (:1000): NULL unconditionally, for every input.
    for nb_workers in [0u32, 1, 2, 4, 64, 200, u32::MAX] {
        diff(&format!("ZSTDMT_createCCtx_advanced({nb_workers})"), |l| {
            let f = l.sym::<FnZstdmtCreate>("ZSTDMT_createCCtx_advanced");
            let default_mem = ZSTD_customMem::default();
            let custom = ZSTD_customMem {
                customAlloc: Some(cust_alloc),
                customFree: Some(cust_free),
                opaque: std::ptr::null_mut(),
            };
            let a = unsafe { f(nb_workers, default_mem, std::ptr::null_mut()) };
            let b = unsafe { f(nb_workers, custom, std::ptr::null_mut()) };
            // Row 446 (:1046): ZSTDMT_freeCCtx(NULL) -> 0, "compatible with free
            // on NULL"; also fed the (always NULL) result above.
            let c = unsafe { l.sym::<FnZstdmtFree>("ZSTDMT_freeCCtx")(a) };
            let d = unsafe { l.sym::<FnZstdmtFree>("ZSTDMT_freeCCtx")(b) };
            let e = unsafe { l.sym::<FnZstdmtFree>("ZSTDMT_freeCCtx")(std::ptr::null_mut()) };
            // Row 447 (:1064): ZSTDMT_sizeof_CCtx(NULL) -> 0, "supports sizeof NULL".
            let g = unsafe { l.sym::<FnZstdmtFree>("ZSTDMT_sizeof_CCtx")(std::ptr::null_mut()) };
            (a.is_null(), b.is_null(), c, d, e, g)
        });
    }
    covers(&[
        "ERR:compress/zstdmt_compress.c:1000",
        "ERR:compress/zstdmt_compress.c:1046",
        "ERR:compress/zstdmt_compress.c:1064",
    ]);
}

// ===========================================================================
// compress/zstd_compress_superblock.c + compress/zstd_compress_internal.h
//   rows 389-402, 406-412 (all INDIRECT via ZSTD_compressSuperBlock, which is
//   reached by setting ZSTD_c_targetCBlockSize), 442 (:654 ZSTD_noCompressBlock)
//   and 443 (:666 ZSTD_rleCompressBlock, via zstd_compress.c's block API).
//   Rows 403, 404, 405 are marked UNREACHABLE in ERRORS.md and auto-excluded.
//
// `ZSTD_compressSuperBlock` is not called directly: it requires `zc` to be in
// the exact state `ZSTD_compressBlock_targetCBlockSize` leaves it in (a populated
// `zc->seqStore` plus a sized `zc->tmpWorkspace`), and a freshly created CCtx has
// `tmpWorkspace == NULL` / `tmpWkspSize == 0`, which would make
// `ZSTD_buildBlockEntropyStats` read through a NULL pointer. The whole
// sub-block machinery is therefore driven the way the library drives it.
// ===========================================================================

fn compress2_with(
    l: &Lib,
    src: &[u8],
    cap: usize,
    params: &[(c_int, c_int)],
) -> (R, R, Blob) {
    let cctx = Ctx::cctx(l);
    for &(p, v) in params {
        let r = unsafe { l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter")(cctx.ptr, p, v) };
        assert!(
            !matches!(res(l, r), R::Err(..)),
            "[{}] setParameter({p},{v}) failed: {:?}",
            l.tag,
            res(l, r)
        );
    }
    // IMPORTANT: `dst` carries 64 KB of slack beyond the declared `cap`.
    // `ZSTD_compress2` with `ZSTD_c_targetCBlockSize` set and a small-but-nonzero
    // `dstCapacity` OVERRUNS the destination before returning dstSize_tooSmall —
    // the sub-block literal writers (`ZSTD_compressRleLiteralsBlock`,
    // superblock.c:65/:90) have no capacity check at all, only an `assert`.
    // Measured worst case over the whole grid below: 640 bytes past `cap`
    // (Sparse, n=200000, targetCBlockSize=2048, level=1, cap=64) — and the C and
    // the Rust overrun by the SAME amount with the SAME bytes, which is exactly
    // what the slack region in the compared Blob proves. See the report.
    const SLACK: usize = 1 << 16;
    let mut dst = vec![0xCDu8; cap + SLACK];
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
    // Round-trip whatever came out, so a silently-wrong sub-block is caught too.
    let (dr, db) = match r {
        R::Ok(n) => {
            let (rr, bb) = decompress_simple(l, &dst[..n], src.len() + 64);
            (rr, bb)
        }
        R::Err(..) => (R::Ok(0), Blob(vec![])),
    };
    // compressed bytes ++ round-tripped bytes, so `diff_bytes` can pinpoint the
    // first differing byte of either.
    let mut all = dst;
    all.extend_from_slice(&db.0);
    (r, dr, Blob(all))
}

#[test]
fn compress_superblock_paths_via_target_cblock_size() {
    let mut cases = 0usize;
    for &k in &[
        Corpus::Zeros,
        Corpus::OneByte,
        Corpus::Random,
        Corpus::Text,
        Corpus::Sparse,
    ] {
        for &n in &[1000usize, 20000, 131072, 200000] {
            let src = corpus(k, n, 0x11_0389 ^ n as u64);
            for tcbs in [1340i32, 2048, 8192] {
                for level in [1i32, 9] {
                    let bound = compress_bound(&pair().c, n);
                    // A generous capacity, plus capacities tight enough that
                    // every FORWARD_IF_ERROR in the sub-block writers fires.
                    for cap in [bound, n / 2 + 16, 64, 4, 0] {
                        cases += 1;
                        diff_bytes(
                            &format!("superblock {k:?} n={n} tcbs={tcbs} level={level} cap={cap}"),
                            |l| {
                                compress2_with(
                                    l,
                                    &src,
                                    cap,
                                    &[
                                        (ZSTD_c_compressionLevel, level),
                                        (ZSTD_c_targetCBlockSize, tcbs),
                                    ],
                                )
                            },
                        );
                    }
                }
            }
        }
    }
    // ... and the same with literal-compression mode forced, so the `set_basic`
    // and `set_rle` literal sub-block writers (rows 389 / 390) are selected.
    for lcm in [ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed] {
        for &k in &[Corpus::Zeros, Corpus::OneByte, Corpus::Random, Corpus::Text] {
            for &n in &[1000usize, 40000, 200000] {
                let src = corpus(k, n, 0x11_0390);
                let bound = compress_bound(&pair().c, n);
                for cap in [bound, n / 4 + 8, 32] {
                    cases += 1;
                    diff_bytes(
                        &format!("superblock lcm={lcm} {k:?} n={n} cap={cap}"),
                        |l| {
                            compress2_with(
                                l,
                                &src,
                                cap,
                                &[
                                    (ZSTD_c_compressionLevel, 6),
                                    (ZSTD_c_targetCBlockSize, 1340),
                                    (ZSTD_c_literalCompressionMode, lcm),
                                ],
                            )
                        },
                    );
                }
            }
        }
    }
    eprintln!("superblock: {cases} configurations compared");
    covers(&[
        "ERR:compress/zstd_compress_superblock.c:62",
        "ERR:compress/zstd_compress_superblock.c:65",
        "ERR:compress/zstd_compress_superblock.c:83",
        "ERR:compress/zstd_compress_superblock.c:88",
        "ERR:compress/zstd_compress_superblock.c:93",
        "ERR:compress/zstd_compress_superblock.c:181",
        "ERR:compress/zstd_compress_superblock.c:189",
        "ERR:compress/zstd_compress_superblock.c:218",
        "ERR:compress/zstd_compress_superblock.c:229",
        "ERR:compress/zstd_compress_superblock.c:248",
        "ERR:compress/zstd_compress_superblock.c:284",
        "ERR:compress/zstd_compress_superblock.c:285",
        "ERR:compress/zstd_compress_superblock.c:295",
        "ERR:compress/zstd_compress_superblock.c:296",
        "ERR:compress/zstd_compress_superblock.c:356",
        "ERR:compress/zstd_compress_superblock.c:532",
        "ERR:compress/zstd_compress_superblock.c:559",
        "ERR:compress/zstd_compress_superblock.c:603",
        "ERR:compress/zstd_compress_superblock.c:637",
        "ERR:compress/zstd_compress_superblock.c:645",
        "ERR:compress/zstd_compress_superblock.c:672",
        "ERR:compress/zstd_compress_internal.h:654",
    ]);
}

#[test]
fn no_compress_block_and_rle_compress_block_capacity_checks() {
    // Row 442 (zstd_compress_internal.h:654): `srcSize + 3 > dstCapacity` ->
    // dstSize_tooSmall (70), reached through the block API (which does not write a
    // frame header, so the block writers see the caller's capacity directly).
    // Row 443 (:666): `dstCapacity < 4` in ZSTD_rleCompressBlock, reached from
    // ZSTD_compressBlock_internal (zstd_compress.c:4465) when the whole block is
    // a single repeated byte.
    for (name, src) in [
        ("rle", vec![0x5Au8; 64]),
        ("random", corpus(Corpus::Random, 64, 0x11_0443)),
        ("text", corpus(Corpus::Text, 4096, 0x11_0443)),
    ] {
        for cap in [0usize, 1, 2, 3, 4, 5, 8, 16, src.len(), src.len() + 3, src.len() + 8] {
            diff_bytes(&format!("compressBlock {name} cap={cap}"), |l| {
                let cctx = Ctx::cctx(l);
                let b = unsafe {
                    l.sym::<FnCCtxSetParameter>("ZSTD_compressBegin")(cctx.ptr, 3, 0)
                };
                let br = res(l, b);
                let mut dst = vec![0xCDu8; cap + 64];
                let n = unsafe {
                    l.sym::<FnDecompressDCtx>("ZSTD_compressBlock")(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src.len(),
                    )
                };
                (br, res(l, n), Blob(dst))
            });
        }
    }
    covers(&[
        "ERR:compress/zstd_compress_internal.h:654",
        "ERR:compress/zstd_compress_internal.h:666",
    ]);
}

// ===========================================================================
// compress/zstd_cwksp.h
//   rows 434 (:302), 435 (:334), 436 (:365), 437 (:457), 438 (:472),
//        439 (:512), 440 (:538) — all INDIRECT via ZSTD_initStaticCCtx /
//        ZSTD_initStaticCStream with a workspace smaller than
//        ZSTD_estimateCCtxSize; row 441 (:692) is covered by the failing
//        custom allocator above.
// ===========================================================================

#[test]
fn cwksp_static_allocation_failures() {
    let src = corpus(Corpus::Text, 30000, 0x11_0434);
    for level in [1i32, 3, 9, 19] {
        let need = {
            let l = &pair().c;
            unsafe { l.sym::<FnEstimateSize>("ZSTD_estimateCCtxSize")(level) }
        };
        // A geometric-ish ladder of sizes from far too small to exactly enough,
        // plus every byte around the boundary, so the object / table / buffer
        // segment collisions are all hit.
        let mut sizes: Vec<usize> = vec![0, 1, 8, 64, 1024, 4096];
        let mut s = 8192usize;
        while s < need {
            sizes.push(s);
            s = s + s / 4 + 1;
        }
        for d in 0..8 {
            if need > d {
                sizes.push(need - d);
            }
        }
        sizes.push(need);
        sizes.push(need + 64);
        sizes.sort_unstable();
        sizes.dedup();
        for size in sizes {
            diff_bytes(
                &format!("initStaticCCtx level={level} size={size} (need {need})"),
                |l| {
                    // 8-byte aligned, as ZSTD_initStaticCCtx demands.
                    let mut buf = vec![0u64; size / 8 + 2];
                    let p = unsafe {
                        l.sym::<FnInitStatic>("ZSTD_initStaticCCtx")(
                            buf.as_mut_ptr() as *mut c_void,
                            size,
                        )
                    };
                    if p.is_null() {
                        return (false, R::Ok(0), Blob(vec![]));
                    }
                    let mut dst = vec![0xCDu8; compress_bound(l, src.len())];
                    let n = unsafe {
                        l.sym::<FnCompressCCtx>("ZSTD_compressCCtx")(
                            p,
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            level,
                        )
                    };
                    let r = res(l, n);
                    let out = match r {
                        R::Ok(v) => Blob(dst[..v].to_vec()),
                        R::Err(..) => Blob(vec![]),
                    };
                    (true, r, out)
                },
            );
        }
        // A misaligned workspace must be refused outright.
        diff(&format!("initStaticCCtx misaligned level={level}"), |l| {
            let mut buf = vec![0u8; need + 64];
            let base = unsafe { buf.as_mut_ptr().add(1) };
            let p = unsafe {
                l.sym::<FnInitStatic>("ZSTD_initStaticCCtx")(base as *mut c_void, need)
            };
            p.is_null()
        });
    }
    // The same ladder through ZSTD_initStaticCStream + ZSTD_compressStream2.
    let need = {
        let l = &pair().c;
        unsafe { l.sym::<FnEstimateSize>("ZSTD_estimateCStreamSize")(3) }
    };
    let mut sizes: Vec<usize> = vec![0, 1, 1024, 4096, 16384];
    let mut s = 32768usize;
    while s < need {
        sizes.push(s);
        s = s + s / 3 + 1;
    }
    sizes.push(need);
    for size in sizes {
        diff_bytes(&format!("initStaticCStream size={size} (need {need})"), |l| {
            let mut buf = vec![0u64; size / 8 + 2];
            let p = unsafe {
                l.sym::<FnInitStatic>("ZSTD_initStaticCStream")(
                    buf.as_mut_ptr() as *mut c_void,
                    size,
                )
            };
            if p.is_null() {
                return (false, R::Ok(0), Blob(vec![]));
            }
            let mut dst = vec![0xCDu8; compress_bound(l, src.len())];
            let mut out = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: dst.len(),
                pos: 0,
            };
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let n = unsafe {
                l.sym::<FnCompressStream2>("ZSTD_compressStream2")(
                    p,
                    &mut out,
                    &mut inb,
                    ZSTD_e_end,
                )
            };
            let r = res(l, n);
            let produced = Blob(dst[..out.pos].to_vec());
            (true, r, produced)
        });
    }
    covers(&[
        "ERR:compress/zstd_cwksp.h:302",
        "ERR:compress/zstd_cwksp.h:334",
        "ERR:compress/zstd_cwksp.h:365",
        "ERR:compress/zstd_cwksp.h:457",
        "ERR:compress/zstd_cwksp.h:472",
        "ERR:compress/zstd_cwksp.h:512",
        "ERR:compress/zstd_cwksp.h:538",
    ]);
}

// ===========================================================================
// The match finders and the block splitter
//   rows 413 (zstd_double_fast.c), 414/415 (zstd_fast.c), 416/417 (zstd_lazy.c),
//        418/419/420/421 (zstd_opt.c), 432/433 (zstd_preSplit.c)
// Every one of these rows records that the file contains NO rejection site: the
// `ZSTD_compressBlock_*` entry points always return `(size_t)(iend - anchor)`
// and never an error code. The test below drives all of them through a real
// compression at every strategy and confirms that claim.
// ===========================================================================

#[test]
fn match_finders_never_return_an_error() {
    let mut checked = 0usize;
    for &k in &[
        Corpus::Zeros,
        Corpus::Random,
        Corpus::SmallAlphabet,
        Corpus::Text,
        Corpus::LongRepeats,
        Corpus::Periodic,
    ] {
        for &n in &[9usize, 1000, 20000] {
            let src = corpus(k, n, 0x11_0413 ^ n as u64);
            for strategy in ALL_STRATEGIES {
                for row in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                    checked += 1;
                    let got = diff_bytes(
                        &format!("matchfinder {k:?} n={n} strat={strategy} row={row}"),
                        |l| {
                            compress2_with(
                                l,
                                &src,
                                compress_bound(l, n),
                                &[
                                    (ZSTD_c_strategy, *strategy),
                                    (ZSTD_c_useRowMatchFinder, row),
                                    (ZSTD_c_compressionLevel, 6),
                                ],
                            )
                        },
                    );
                    assert!(
                        matches!(got.0, R::Ok(_)),
                        "match finders must never fail: {:?}",
                        got.0
                    );
                    assert_eq!(got.1, R::Ok(n), "round-trip length");
                }
            }
        }
    }
    // Row 420 (zstd_opt.c:1533): the btultra2 two-pass warm-up gate, whose
    // srcSize condition is `> ZSTD_PREDEF_THRESHOLD (8)`. Compare 8 against 9 on a
    // freshly reset context, plus the dictionary and extDict variants.
    for n in [1usize, 2, 7, 8, 9, 10, 16, 100] {
        let src = corpus(Corpus::Text, n, 0x11_0420);
        diff_bytes(&format!("btultra2 gate n={n}"), |l| {
            compress2_with(
                l,
                &src,
                compress_bound(l, n),
                &[(ZSTD_c_strategy, ZSTD_btultra2), (ZSTD_c_compressionLevel, 22)],
            )
        });
    }
    // Rows 432 / 433 (zstd_preSplit.c): ZSTD_splitBlock only ever returns a valid
    // split position, never an error code. PRECONDITIONS (assert-only in the C, so
    // respected here rather than probed): `level` in [0,4], `blockSize` exactly
    // 128 KB, `wkspSize >= ZSTD_SLIPBLOCK_WORKSPACESIZE`, workspace aligned for
    // size_t. Out-of-range `level` indexes `records_fs[]`/`hashParams[]` out of
    // bounds and a `blockSize != 128 KB` reads past the block — both UB.
    for &k in &[
        Corpus::Zeros,
        Corpus::Random,
        Corpus::Text,
        Corpus::Mixed,
        Corpus::Periodic,
    ] {
        let block = corpus(k, 128 * 1024, 0x11_0432);
        for level in 0..=4i32 {
            for ws in [
                ZSTD_SLIPBLOCK_WORKSPACESIZE,
                ZSTD_SLIPBLOCK_WORKSPACESIZE + 64,
                ZSTD_SLIPBLOCK_WORKSPACESIZE * 2,
            ] {
                let got = diff(&format!("splitBlock {k:?} level={level} ws={ws}"), |l| {
                    let mut w = wksp(ws);
                    unsafe {
                        l.sym::<FnSplitBlock>("ZSTD_splitBlock")(
                            block.as_ptr() as *const c_void,
                            128 * 1024,
                            level,
                            wp(&mut w),
                            ws,
                        )
                    }
                });
                assert!(
                    got > 0 && got <= 128 * 1024,
                    "split position {got} out of range"
                );
                assert!(!is_error(&pair().c, got), "splitBlock must not error");
            }
        }
    }
    // and through a real compression with the block splitter enabled at every
    // documented level.
    // ZSTD_BLOCKSPLITTER_LEVEL_MAX == 6
    for splitter_level in 0..=6i32 {
        for &k in &[Corpus::Mixed, Corpus::Text] {
            let src = corpus(k, 400_000, 0x11_0433);
            diff_bytes(
                &format!("blockSplitter level={splitter_level} {k:?}"),
                |l| {
                    compress2_with(
                        l,
                        &src,
                        compress_bound(l, src.len()),
                        &[
                            (ZSTD_c_compressionLevel, 9),
                            (ZSTD_c_blockSplitterLevel, splitter_level),
                        ],
                    )
                },
            );
        }
    }
    eprintln!("match finders: {checked} configurations compared");
    covers(&[
        "ERR:compress/zstd_double_fast.c:251",
        "ERR:compress/zstd_fast.c:119",
        "ERR:compress/zstd_fast.c:375",
        "ERR:compress/zstd_lazy.c:402",
        "ERR:compress/zstd_lazy.c:1778",
        "ERR:compress/zstd_opt.c:271",
        "ERR:compress/zstd_opt.c:846",
        "ERR:compress/zstd_opt.c:1441",
        "ERR:compress/zstd_opt.c:1533",
        // ERRORS.md keys row 433 by its whole multi-line site cell.
        "ERR:compress/zstd_preSplit.c:178 / :185 / :215 / :223 / :224",
        "ERR:compress/zstd_preSplit.c:238",
    ]);
}
