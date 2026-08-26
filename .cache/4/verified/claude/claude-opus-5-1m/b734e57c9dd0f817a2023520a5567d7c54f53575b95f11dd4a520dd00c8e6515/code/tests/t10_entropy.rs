//! Phase B — the LOWEST-LEVEL exported entry points: FSE, HUF, HIST, xxhash,
//! POOL, divsufsort. These are driven directly (not through `ZSTD_compress`)
//! because a bug in e.g. `FSE_normalizeCount`'s tie-breaking is invisible once
//! the composed pipeline has rounded it away.
//!
//! Every case runs the identical call sequence against the C `.so` and the Rust
//! `.so` and compares *all* observable outputs: return value, every out-param,
//! and the full destination buffer including bytes the callee did not write.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// Constants resolved from the C headers
// ---------------------------------------------------------------------------

const FSE_MAX_TABLELOG: u32 = 12; // FSE_MAX_MEMORY_USAGE(14) - 2
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const FSE_DEFAULT_TABLELOG: u32 = 11; // FSE_DEFAULT_MEMORY_USAGE(13) - 2
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const FSE_NCOUNTBOUND: usize = 512;

const HUF_TABLELOG_MAX: u32 = 12;
const HUF_TABLELOG_DEFAULT: u32 = 11;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192) * 4;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
/// `ZSTD_HUFFDTABLE_CAPACITY_LOG` from `decompress/zstd_decompress_internal.h`.
const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;
const HIST_WKSP_SIZE: usize = 1024 * 4;

const HUF_flags_bmi2: c_int = 1 << 0;
const HUF_flags_optimalDepth: c_int = 1 << 1;
const HUF_flags_preferRepeat: c_int = 1 << 2;
const HUF_flags_suspectUncompressible: c_int = 1 << 3;
const HUF_flags_disableAsm: c_int = 1 << 4;
const HUF_flags_disableFast: c_int = 1 << 5;

fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    1 + (1usize << (max_table_log - 1)) + ((max_symbol_value as usize + 1) * 2)
}
fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
fn fse_build_dtable_wksp_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    let bytes = 2 * (max_symbol_value as usize + 1) + (1usize << max_table_log) + 8;
    (bytes + 3) / 4
}
fn fse_decompress_wksp_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_dtable_size_u32(max_table_log)
        + 1
        + fse_build_dtable_wksp_size_u32(max_table_log, max_symbol_value)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}

/// 4-byte (in fact 8-byte) aligned scratch buffer, as the C API demands.
/// A `HUF_DTable` allocated and *seeded* exactly the way the library's own
/// callers do it. `DTable[0]` is a `DTableDesc { BYTE maxTableLog; BYTE
/// tableType; BYTE tableLog; BYTE reserved; }`, and both `zstd_decompress.c`
/// and `zstd_ddict.c` initialise it as
/// `hufTable[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x1000001` — the multiply
/// replicates the capacity into the low and high bytes so the same constant
/// works on little- and big-endian.
///
/// Getting this wrong is silently catastrophic rather than an error: with
/// `maxTableLog == 0`, `HUF_readDTableX1_wksp` *succeeds* because
/// `HUF_rescaleStats` rescales the tree down to `targetTableLog == 1`, and the
/// resulting 2-entry table then decodes a stream that was encoded with an
/// 11-bit table — walking off the end of `dst` (SIGSEGV in the reference C).
fn huf_dtable() -> Vec<u32> {
    let mut dt = vec![0u32; 1 + (1usize << ZSTD_HUFFDTABLE_CAPACITY_LOG)];
    dt[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x0100_0001;
    dt
}

fn wksp(bytes: usize) -> Vec<u64> {
    vec![0u64; (bytes + 7) / 8]
}
fn wksp_ptr(w: &mut [u64]) -> *mut c_void {
    w.as_mut_ptr() as *mut c_void
}
fn wksp_bytes(w: &[u64]) -> usize {
    w.len() * 8
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnU32Void = unsafe extern "C" fn() -> c_uint;
type FnSzSz = unsafe extern "C" fn(SizeT) -> SizeT;

type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnHistCountWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
) -> SizeT;
type FnHistCountSimple =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, SizeT) -> c_uint;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, SizeT);

type FnFseOptimalTableLog = unsafe extern "C" fn(c_uint, SizeT, c_uint) -> c_uint;
type FnFseOptimalTableLogInternal = unsafe extern "C" fn(c_uint, SizeT, c_uint, c_uint) -> c_uint;
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
type FnFseBuildCTableRle = unsafe extern "C" fn(*mut c_uint, u8) -> SizeT;
type FnFseCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_uint) -> SizeT;
type FnFseBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
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

type FnHufCardinality = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
type FnHufMinTableLog = unsafe extern "C" fn(c_uint) -> c_uint;
type FnHufOptimalTableLog = unsafe extern "C" fn(
    c_uint,
    SizeT,
    c_uint,
    *mut c_void,
    SizeT,
    *mut u64,
    *const c_uint,
    c_int,
) -> c_uint;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut u64, *const c_uint, c_uint, c_uint, *mut c_void, SizeT) -> SizeT;
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
type FnHufEstimateCompressedSize = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> SizeT;
type FnHufValidateCTable = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> c_int;
type FnHufGetNbBits = unsafe extern "C" fn(*const u64, c_uint) -> c_uint;
type FnHufReadCTableHeader = unsafe extern "C" fn(*const u64) -> u64;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut u64, *mut c_uint, *const c_void, SizeT, *mut c_uint) -> SizeT;
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

type FnXxh32 = unsafe extern "C" fn(*const c_void, SizeT, u32) -> u32;
type FnXxh64 = unsafe extern "C" fn(*const c_void, SizeT, u64) -> u64;
type FnXxhCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnXxhFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnXxh32Reset = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnXxh64Reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnXxhUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> c_int;
type FnXxh32Digest = unsafe extern "C" fn(*const c_void) -> u32;
type FnXxh64Digest = unsafe extern "C" fn(*const c_void) -> u64;
type FnXxhCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnXxh32Canon = unsafe extern "C" fn(*mut c_void, u32);
type FnXxh64Canon = unsafe extern "C" fn(*mut c_void, u64);
type FnXxh32FromCanon = unsafe extern "C" fn(*const c_void) -> u32;
type FnXxh64FromCanon = unsafe extern "C" fn(*const c_void) -> u64;

type FnDivsufsort = unsafe extern "C" fn(*const u8, *mut i32, i32, i32) -> i32;
type FnDivbwt = unsafe extern "C" fn(*mut u8, *mut u8, *mut i32, i32, *mut u8, i32) -> i32;

type FnPoolCreate = unsafe extern "C" fn(SizeT, SizeT) -> *mut c_void;
type FnPoolFree = unsafe extern "C" fn(*mut c_void);
type FnPoolSizeof = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnPoolResize = unsafe extern "C" fn(*mut c_void, SizeT) -> c_int;
type FnPoolJoinJobs = unsafe extern "C" fn(*mut c_void);
type FnPoolAdd = unsafe extern "C" fn(*mut c_void, Option<extern "C" fn(*mut c_void)>, *mut c_void);
type FnPoolTryAdd =
    unsafe extern "C" fn(*mut c_void, Option<extern "C" fn(*mut c_void)>, *mut c_void) -> c_int;

// ---------------------------------------------------------------------------
// CONFIGS row: version / bound / error-name helpers of every low-level family
// ---------------------------------------------------------------------------

#[test]
fn low_level_versions_and_bounds() {
    covers(&["CFG:247", "CFG:263", "CFG:294"]);
    diff("FSE_versionNumber", |l| unsafe {
        l.sym::<FnU32Void>("FSE_versionNumber")()
    });
    diff("ZSTD_XXH_versionNumber", |l| unsafe {
        l.sym::<FnU32Void>("ZSTD_XXH_versionNumber")()
    });
    for n in [
        0usize, 1, 2, 3, 7, 8, 255, 256, 1024, 65535, 65536, 131072, 1 << 20, usize::MAX / 4,
    ] {
        diff(&format!("FSE_compressBound({n})"), |l| unsafe {
            l.sym::<FnSzSz>("FSE_compressBound")(n)
        });
        diff(&format!("HUF_compressBound({n})"), |l| unsafe {
            l.sym::<FnSzSz>("HUF_compressBound")(n)
        });
        diff(&format!("ZSTD_compressBound({n})"), |l| unsafe {
            l.sym::<FnSzSz>("ZSTD_compressBound")(n)
        });
    }
    // FSE_NCountWriteBound over the whole (maxSymbolValue, tableLog) grid, and a
    // check that it never exceeds the static `FSE_NCOUNTBOUND` the header
    // promises callers can allocate.
    for msv in [0u32, 1, 2, 15, 16, 63, 127, 128, 254, 255] {
        for tl in 0..=FSE_TABLELOG_ABSOLUTE_MAX {
            let b = diff(&format!("FSE_NCountWriteBound({msv},{tl})"), |l| unsafe {
                l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, tl)
            });
            assert!(
                b <= FSE_NCOUNTBOUND,
                "FSE_NCountWriteBound({msv},{tl}) = {b} exceeds FSE_NCOUNTBOUND ({FSE_NCOUNTBOUND})"
            );
        }
    }
}

#[test]
fn error_name_tables_match() {
    covers(&["CFG:263", "CFG:294", "CFG:300",
             "ERR:common/error_private.c:61", "ERR:common/error_private.h:52",
             "ERR:common/error_private.h:54", "ERR:common/error_private.h:74"]);
    // Every family's isError/getErrorName over the full plausible code range.
    let fams = [
        ("FSE_isError", "FSE_getErrorName"),
        ("HUF_isError", "HUF_getErrorName"),
        ("HIST_isError", ""),
        ("ZSTD_isError", "ZSTD_getErrorName"),
        ("ZDICT_isError", "ZDICT_getErrorName"),
        ("ZBUFF_isError", "ZBUFF_getErrorName"),
    ];
    for (is_err, get_name) in fams {
        for code in 0..=140usize {
            let v = 0usize.wrapping_sub(code);
            diff(&format!("{is_err}({code})"), |l| unsafe {
                l.sym::<FnIsError>(is_err)(v)
            });
            if !get_name.is_empty() {
                diff(&format!("{get_name}({code})"), |l| unsafe {
                    cstr(l.sym::<FnGetErrorName>(get_name)(v))
                });
            }
        }
        // and a few non-error values
        for v in [0usize, 1, 2, 1000, usize::MAX / 2] {
            diff(&format!("{is_err}(ok {v})"), |l| unsafe {
                l.sym::<FnIsError>(is_err)(v)
            });
        }
    }
    // ERR_getErrorString / ZSTD_getErrorString over every enum value incl. gaps
    // and out-of-range values (a C enum accepts any int).
    for code in -5i32..=130 {
        diff(&format!("ZSTD_getErrorString({code})"), |l| unsafe {
            cstr(l.sym::<FnGetErrorString>("ZSTD_getErrorString")(code))
        });
        diff(&format!("ERR_getErrorString({code})"), |l| unsafe {
            cstr(l.sym::<FnGetErrorString>("ERR_getErrorString")(code))
        });
    }
    for code in [-1000i32, -1, 121, 200, 1000, i32::MAX, i32::MIN] {
        diff(&format!("ZSTD_getErrorString(oob {code})"), |l| unsafe {
            cstr(l.sym::<FnGetErrorString>("ZSTD_getErrorString")(code))
        });
    }
    // ZSTD_getErrorCode for every encoded error and for non-errors.
    for code in 0..=140usize {
        let v = 0usize.wrapping_sub(code);
        diff(&format!("ZSTD_getErrorCode({code})"), |l| unsafe {
            l.sym::<FnGetErrorCode>("ZSTD_getErrorCode")(v)
        });
    }
}

// ---------------------------------------------------------------------------
// HIST_* — the histogram primitives every entropy stage starts from
// ---------------------------------------------------------------------------

#[derive(PartialEq, Debug)]
struct HistRun {
    count: R,
    msv_out: u32,
    counts: Blob,
    count_wksp: R,
    msv_wksp: u32,
    simple: u32,
    fast: R,
    fast_wksp: R,
    msv_fast: u32,
    other_counts: Blob,
}

/// Run every HIST entry point and return every observable.
fn hist_all(l: &Lib, src: &[u8], msv_in: u32) -> HistRun {
    let mut count = vec![0u32; 256];
    let mut msv = msv_in;
    let n = unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let r1 = res(l, n);
    let msv1 = msv;

    // HIST_count_wksp with the documented workspace
    let mut count2 = vec![0u32; 256];
    let mut msv2 = msv_in;
    let mut w = wksp(HIST_WKSP_SIZE);
    let n2 = unsafe {
        l.sym::<FnHistCountWksp>("HIST_count_wksp")(
            count2.as_mut_ptr(),
            &mut msv2,
            src.as_ptr() as *const c_void,
            src.len(),
            wksp_ptr(&mut w),
            wksp_bytes(&w),
        )
    };
    let r2 = res(l, n2);

    // HIST_count_simple / HIST_countFast / HIST_countFast_wksp are documented as
    // UNSAFE unless every byte is <= *maxSymbolValuePtr, so only drive them with
    // maxSymbolValue = 255 (always true) — the same restriction the C documents.
    let mut count3 = vec![0u32; 256];
    let mut msv3 = 255u32;
    let s3 = unsafe {
        l.sym::<FnHistCountSimple>("HIST_count_simple")(
            count3.as_mut_ptr(),
            &mut msv3,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let mut count4 = vec![0u32; 256];
    let mut msv4 = 255u32;
    let s4 = unsafe {
        l.sym::<FnHistCount>("HIST_countFast")(
            count4.as_mut_ptr(),
            &mut msv4,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let mut count5 = vec![0u32; 256];
    let mut msv5 = 255u32;
    let mut w5 = wksp(HIST_WKSP_SIZE);
    let s5 = unsafe {
        l.sym::<FnHistCountWksp>("HIST_countFast_wksp")(
            count5.as_mut_ptr(),
            &mut msv5,
            src.as_ptr() as *const c_void,
            src.len(),
            wksp_ptr(&mut w5),
            wksp_bytes(&w5),
        )
    };

    // HIST_add accumulates into a pre-seeded array — verify it does not reset.
    let mut count6 = vec![7u32; 256];
    unsafe {
        l.sym::<FnHistAdd>("HIST_add")(
            count6.as_mut_ptr(),
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };

    HistRun {
        count: r1,
        msv_out: msv1,
        counts: Blob(count.iter().flat_map(|v| v.to_le_bytes()).collect()),
        count_wksp: r2,
        msv_wksp: msv2,
        simple: s3,
        fast: res(l, s4),
        fast_wksp: res(l, s5),
        msv_fast: msv4 ^ msv5 ^ msv3,
        other_counts: Blob(
            count2
                .iter()
                .chain(count3.iter())
                .chain(count4.iter())
                .chain(count5.iter())
                .chain(count6.iter())
                .flat_map(|v| v.to_le_bytes())
                .collect(),
        ),
    }
}

#[test]
fn hist_family_matches_over_all_corpora_and_sizes() {
    covers(&["CFG:295", "CFG:298", "CFG:299", "CFG:300",
             "ERR:compress/hist.c:48", "ERR:compress/hist.c:154"]);
    for &k in ALL_CORPORA {
        for &n in SIZES {
            if n > 200000 {
                continue;
            }
            let src = corpus(k, n, 0x4831_5354);
            for msv in [0u32, 1, 3, 15, 63, 127, 255] {
                diff(&format!("HIST {k:?} n={n} msv={msv}"), |l| {
                    hist_all(l, &src, msv)
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// FSE_* — the full low-level FSE pipeline, driven stage by stage
// ---------------------------------------------------------------------------

/// Result of the complete manual FSE encode+decode pipeline.
#[derive(PartialEq, Debug)]
struct FseRun {
    hist: R,
    max_symbol: u32,
    opt_table_log: u32,
    normalize: R,
    norm_counter: Vec<i16>,
    ncount_written: R,
    ncount_bytes: Blob,
    ncount_read: R,
    read_msv: u32,
    read_tl: u32,
    read_counter: Vec<i16>,
    build_ctable: R,
    ctable: Blob,
    compressed: R,
    cblob: Blob,
    build_dtable: R,
    dtable: Blob,
    decompressed: R,
    dblob: Blob,
}

/// Drive the whole low-level FSE encode/decode chain the way the C's own
/// callers do (see `ZSTD_buildCTable` in `compress/zstd_compress_sequences.c`):
/// each stage runs only if the previous one succeeded, because the later stages
/// document a *precondition* (a valid normalised distribution) rather than
/// validating their input.
///
/// PRECONDITION, taken from the C: `srcSize > 1`.
/// `FSE_minTableLog`, `FSE_optimalTableLog_internal` and `HUF_optimalTableLog`
/// all carry `assert(srcSize > 1) /* Not supported, RLE should be used instead */`,
/// and `FSE_normalizeCount` computes `ZSTD_div64((U64)1<<62, (U32)total)` —
/// with `total == 0` the reference C build divides by zero and takes SIGFPE.
/// Callers therefore never pass 0 or 1 here; the exported entry points are
/// probed with those values separately in the error-path suite, where the C's
/// *checked* rejections are compared and the crashing ones are documented.
fn fse_pipeline(
    l: &Lib,
    src: &[u8],
    table_log_req: u32,
    use_low_prob: u32,
    dst_cap_bias: isize,
) -> FseRun {
    assert!(
        src.len() > 1,
        "fse_pipeline precondition: the C asserts srcSize > 1"
    );
    // 1. histogram
    let mut count = vec![0u32; 256];
    let mut msv = FSE_MAX_SYMBOL_VALUE;
    let hn = unsafe {
        l.sym::<FnHistCount>("HIST_count")(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let hist = res(l, hn);

    // 2. optimal table log
    let otl = unsafe {
        l.sym::<FnFseOptimalTableLog>("FSE_optimalTableLog")(table_log_req, src.len(), msv)
    };
    let table_log = if table_log_req == 0 { otl } else { table_log_req };

    // 3. normalize
    let mut norm = vec![0i16; 256];
    let nn = unsafe {
        l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
            norm.as_mut_ptr(),
            table_log,
            count.as_ptr(),
            src.len(),
            msv,
            use_low_prob,
        )
    };
    let normalize = res(l, nn);
    // `FSE_normalizeCount` returns 0 to mean "RLE special case" (one symbol
    // covers the whole input); in that case `norm` is NOT a usable distribution
    // and the C's callers switch to `set_rle` instead of building a table.
    let usable_norm = matches!(normalize, R::Ok(v) if v >= FSE_MIN_TABLELOG as usize);
    let eff_table_log = match normalize {
        R::Ok(v) if v >= FSE_MIN_TABLELOG as usize => v as u32,
        _ => table_log.clamp(FSE_MIN_TABLELOG, FSE_MAX_TABLELOG),
    };

    // 4. writeNCount
    let bound = unsafe { l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, eff_table_log) };
    let cap = (bound as isize + dst_cap_bias).max(0) as usize;
    let mut nc = vec![0xEEu8; cap];
    let wn = if usable_norm {
        unsafe {
            l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
                nc.as_mut_ptr() as *mut c_void,
                cap,
                norm.as_ptr(),
                msv,
                eff_table_log,
            )
        }
    } else {
        0
    };
    let ncount_written = res(l, wn);
    let nc_len = match ncount_written {
        R::Ok(v) => v,
        R::Err(..) => 0,
    };

    // 5. readNCount round-trip (both the plain and the _bmi2 entry point)
    let mut rnorm = vec![0i16; 256];
    let mut rmsv = FSE_MAX_SYMBOL_VALUE;
    let mut rtl = FSE_TABLELOG_ABSOLUTE_MAX;
    let rn = unsafe {
        l.sym::<FnFseReadNCount>("FSE_readNCount")(
            rnorm.as_mut_ptr(),
            &mut rmsv,
            &mut rtl,
            nc.as_ptr() as *const c_void,
            nc_len,
        )
    };
    let ncount_read = res(l, rn);
    {
        // FSE_readNCount_bmi2 with bmi2=0 must agree with FSE_readNCount exactly.
        let mut n2 = vec![0i16; 256];
        let mut m2 = FSE_MAX_SYMBOL_VALUE;
        let mut t2 = FSE_TABLELOG_ABSOLUTE_MAX;
        let r2 = unsafe {
            l.sym::<FnFseReadNCountBmi2>("FSE_readNCount_bmi2")(
                n2.as_mut_ptr(),
                &mut m2,
                &mut t2,
                nc.as_ptr() as *const c_void,
                nc_len,
                0,
            )
        };
        assert_eq!(r2, rn, "[{}] readNCount_bmi2 != readNCount", l.tag);
        assert_eq!(n2, rnorm, "[{}] readNCount_bmi2 counters differ", l.tag);
        assert_eq!((m2, t2), (rmsv, rtl), "[{}] bmi2 out-params", l.tag);
    }

    // 6. buildCTable + compress
    let ct_u32 = fse_ctable_size_u32(eff_table_log.max(1), msv);
    let mut ctable = vec![0u32; ct_u32];
    let mut cw = wksp(1 << 16);
    let bc = if usable_norm {
        unsafe {
            l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
                ctable.as_mut_ptr(),
                norm.as_ptr(),
                msv,
                eff_table_log,
                wksp_ptr(&mut cw),
                wksp_bytes(&cw),
            )
        }
    } else {
        0
    };
    let build_ctable = res(l, bc);

    let cbound = unsafe { l.sym::<FnSzSz>("FSE_compressBound")(src.len()) };
    let mut cbuf = vec![0xDDu8; cbound + 64];
    let cn = if usable_norm && matches!(build_ctable, R::Ok(_)) {
        unsafe {
            l.sym::<FnFseCompressUsingCTable>("FSE_compress_usingCTable")(
                cbuf.as_mut_ptr() as *mut c_void,
                cbuf.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ctable.as_ptr(),
            )
        }
    } else {
        0
    };
    let compressed = res(l, cn);
    let c_len = match compressed {
        R::Ok(v) => v,
        R::Err(..) => 0,
    };

    // 7. buildDTable
    let dt_u32 = fse_dtable_size_u32(eff_table_log.max(1));
    let mut dtable = vec![0u32; dt_u32];
    let mut dw = wksp(1 << 16);
    let bd = if usable_norm {
        unsafe {
            l.sym::<FnFseBuildDTableWksp>("FSE_buildDTable_wksp")(
                dtable.as_mut_ptr(),
                norm.as_ptr(),
                msv,
                eff_table_log,
                wksp_ptr(&mut dw),
                wksp_bytes(&dw),
            )
        }
    } else {
        0
    };
    let build_dtable = res(l, bd);

    // 8. full-frame decompress (NCount header ++ bitstream), as FSE_decompress does
    let mut frame = Vec::new();
    frame.extend_from_slice(&nc[..nc_len]);
    frame.extend_from_slice(&cbuf[..c_len]);
    let mut dbuf = vec![0xBBu8; src.len() + 64];
    let mut dwk = wksp(fse_decompress_wksp_size_u32(FSE_MAX_TABLELOG, 255) * 4 + 64);
    let dn = if c_len > 0 {
        unsafe {
            l.sym::<FnFseDecompressWkspBmi2>("FSE_decompress_wksp_bmi2")(
                dbuf.as_mut_ptr() as *mut c_void,
                src.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                FSE_MAX_TABLELOG,
                wksp_ptr(&mut dwk),
                wksp_bytes(&dwk),
                0,
            )
        }
    } else {
        0
    };
    let decompressed = res(l, dn);
    let dlen = match decompressed {
        R::Ok(v) => v,
        R::Err(..) => 0,
    };

    FseRun {
        hist,
        max_symbol: msv,
        opt_table_log: otl,
        normalize,
        norm_counter: norm,
        ncount_written,
        ncount_bytes: Blob(nc),
        ncount_read,
        read_msv: rmsv,
        read_tl: rtl,
        read_counter: rnorm,
        build_ctable,
        ctable: Blob(ctable.iter().flat_map(|v| v.to_le_bytes()).collect()),
        compressed,
        cblob: Blob(cbuf[..c_len].to_vec()),
        build_dtable,
        dtable: Blob(dtable.iter().flat_map(|v| v.to_le_bytes()).collect()),
        decompressed,
        dblob: Blob(dbuf[..dlen].to_vec()),
    }
}

#[test]
fn fse_pipeline_matches_over_corpora_sizes_and_tablelogs() {
    covers(&["CFG:243", "CFG:244", "CFG:251", "CFG:255", "CFG:256", "CFG:259",
             "CFG:260", "CFG:261",
             "ERR:compress/fse_compress.c:471", "ERR:compress/fse_compress.c:472",
             "ERR:compress/fse_compress.c:473", "ERR:compress/fse_compress.c:487",
             "ERR:compress/fse_compress.c:563", "ERR:compress/fse_compress.c:333",
             "ERR:compress/fse_compress.c:334",
             "ERR:common/entropy_common.c:63", "ERR:common/entropy_common.c:73",
             "ERR:common/fse_decompress.c:267"]);
    let mut cases = 0usize;
    for &k in ALL_CORPORA {
        for &n in &[2usize, 3, 4, 5, 8, 11, 12, 17, 64, 255, 256, 1000, 4096, 65535] {
            let src = corpus(k, n, 0x5EED_0001 ^ n as u64);
            for tl in [0u32, FSE_MIN_TABLELOG, 6, 9, FSE_DEFAULT_TABLELOG, FSE_MAX_TABLELOG] {
                for ulp in [0u32, 1] {
                    cases += 1;
                    diff(
                        &format!("FSE pipeline {k:?} n={n} tableLog={tl} useLowProb={ulp}"),
                        |l| fse_pipeline(l, &src, tl, ulp, 0),
                    );
                }
            }
        }
    }
    assert!(cases > 500, "expected a wide sweep, ran {cases}");
    eprintln!("FSE pipeline: {cases} configurations compared");
}

#[test]
fn fse_pipeline_matches_on_randomised_inputs() {
    covers(&["CFG:243", "CFG:251", "CFG:255", "CFG:259", "CFG:261",
             "ERR:common/entropy_common.c:179", "ERR:common/entropy_common.c:181",
             "ERR:common/entropy_common.c:182"]);
    let mut rng = Rng::new(0xF5E_0000);
    for i in 0..400 {
        let k = *rng.pick(ALL_CORPORA);
        let n = 2 + rng.below(9000);
        let src = corpus(k, n, rng.next_u64());
        let tl = rng.range(0, FSE_TABLELOG_ABSOLUTE_MAX as i64) as u32;
        let ulp = rng.next_u32() & 1;
        diff(
            &format!("FSE random #{i} {k:?} n={n} tl={tl} ulp={ulp}"),
            |l| fse_pipeline(l, &src, tl, ulp, 0),
        );
    }
}

#[test]
fn fse_optimal_table_log_matches_over_full_grid() {
    covers(&["CFG:241", "CFG:242"]);
    for max_tl in 0..=FSE_TABLELOG_ABSOLUTE_MAX + 2 {
        for &src_size in &[
            0usize, 1, 2, 3, 4, 5, 8, 16, 100, 255, 256, 1000, 65536, 1 << 20,
        ] {
            for msv in [0u32, 1, 2, 3, 15, 16, 100, 254, 255] {
                diff(
                    &format!("FSE_optimalTableLog({max_tl},{src_size},{msv})"),
                    |l| unsafe {
                        l.sym::<FnFseOptimalTableLog>("FSE_optimalTableLog")(
                            max_tl, src_size, msv,
                        )
                    },
                );
                for minus in [0u32, 1, 2, 3] {
                    diff(
                        &format!("FSE_optimalTableLog_internal({max_tl},{src_size},{msv},{minus})"),
                        |l| unsafe {
                            l.sym::<FnFseOptimalTableLogInternal>("FSE_optimalTableLog_internal")(
                                max_tl, src_size, msv, minus,
                            )
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn fse_build_ctable_rle_matches() {
    covers(&["CFG:258"]);
    for sym in [0u8, 1, 42, 127, 128, 254, 255] {
        diff(&format!("FSE_buildCTable_rle({sym})"), |l| {
            let mut ct = vec![0u32; fse_ctable_size_u32(1, 255)];
            let n = unsafe { l.sym::<FnFseBuildCTableRle>("FSE_buildCTable_rle")(ct.as_mut_ptr(), sym) };
            (
                res(l, n),
                Blob(ct.iter().flat_map(|v| v.to_le_bytes()).collect()),
            )
        });
    }
}

/// `FSE_normalizeCount` on synthetic histograms — the interesting inputs are
/// *not* real corpora but adversarial count vectors (single symbol, one huge
/// count plus many ones, counts summing to exactly `srcSize`, etc.).
#[test]
fn fse_normalize_count_on_synthetic_histograms() {
    covers(&["CFG:243", "CFG:244", "CFG:245", "CFG:246", "CFG:248", "CFG:250",
             "ERR:compress/fse_compress.c:471", "ERR:compress/fse_compress.c:472",
             "ERR:compress/fse_compress.c:473", "ERR:compress/fse_compress.c:487",
             "ERR:compress/fse_compress.c:301", "ERR:compress/fse_compress.c:315"]);
    let mut rng = Rng::new(0x1DEA_5EED);
    let mut cases = Vec::new();

    // degenerate: single symbol
    for msv in [0u32, 1, 7, 255] {
        let mut c = vec![0u32; 256];
        c[msv as usize] = 1000;
        cases.push((c, 1000usize, msv));
    }
    // uniform
    for msv in [1u32, 3, 15, 255] {
        let mut c = vec![0u32; 256];
        let per = 4u32;
        for i in 0..=msv as usize {
            c[i] = per;
        }
        cases.push((c, per as usize * (msv as usize + 1), msv));
    }
    // one dominant plus many singletons (forces the low-prob path)
    for msv in [15u32, 100, 255] {
        let mut c = vec![0u32; 256];
        c[0] = 100000;
        for i in 1..=msv as usize {
            c[i] = 1;
        }
        cases.push((c, 100000 + msv as usize, msv));
    }
    // randomised histograms
    for _ in 0..200 {
        let msv = rng.range(0, 255) as u32;
        let mut c = vec![0u32; 256];
        let mut total = 0usize;
        for i in 0..=msv as usize {
            let v = if rng.below(4) == 0 {
                0
            } else {
                rng.below(5000) as u32
            };
            c[i] = v;
            total += v as usize;
        }
        if total == 0 {
            c[0] = 1;
            total = 1;
        }
        cases.push((c, total, msv));
    }

    for (idx, (count, total, msv)) in cases.iter().enumerate() {
        for tl in [0u32, 1, 4, FSE_MIN_TABLELOG, 8, FSE_MAX_TABLELOG, FSE_TABLELOG_ABSOLUTE_MAX, 16] {
            for ulp in [0u32, 1] {
                diff(
                    &format!("FSE_normalizeCount #{idx} tl={tl} msv={msv} total={total} ulp={ulp}"),
                    |l| {
                        let mut norm = vec![0i16; 256];
                        let n = unsafe {
                            l.sym::<FnFseNormalizeCount>("FSE_normalizeCount")(
                                norm.as_mut_ptr(),
                                tl,
                                count.as_ptr(),
                                *total,
                                *msv,
                                ulp,
                            )
                        };
                        let r = res(l, n);
                        // writeNCount / readNCount round-trip of the result
                        let (wr, wbytes, rr, rmsv, rtl, rnorm) = if let R::Ok(etl) = r {
                            let etl = etl as u32;
                            let bound = unsafe {
                                l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(*msv, etl)
                            };
                            let mut buf = vec![0xEEu8; bound + 16];
                            let w = unsafe {
                                l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
                                    buf.as_mut_ptr() as *mut c_void,
                                    buf.len(),
                                    norm.as_ptr(),
                                    *msv,
                                    etl,
                                )
                            };
                            let wr = res(l, w);
                            let wl = match wr {
                                R::Ok(v) => v,
                                _ => 0,
                            };
                            let mut rn = vec![0i16; 256];
                            let mut rm = 255u32;
                            let mut rt = FSE_TABLELOG_ABSOLUTE_MAX;
                            let rv = unsafe {
                                l.sym::<FnFseReadNCount>("FSE_readNCount")(
                                    rn.as_mut_ptr(),
                                    &mut rm,
                                    &mut rt,
                                    buf.as_ptr() as *const c_void,
                                    wl,
                                )
                            };
                            (wr, Blob(buf[..wl].to_vec()), res(l, rv), rm, rt, rn)
                        } else {
                            (R::Ok(0), Blob(vec![]), R::Ok(0), 0, 0, vec![])
                        };
                        (r, norm, wr, wbytes, rr, rmsv, rtl, rnorm)
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HUF_* — the full low-level Huffman pipeline
// ---------------------------------------------------------------------------

/// `HUF_buildCTable_wksp` has an **unchecked** precondition: `maxNbBits` must be
/// large enough to give every present symbol a distinct code, i.e.
/// `maxNbBits >= ceil(log2(cardinality))`. Below that, `HUF_setMaxHeight`'s
/// rebalancing walks off `huffNode[]` and the reference C build takes SIGSEGV
/// (verified directly: cardinality 27 crashes for `maxNbBits <= 4` and succeeds
/// from 5; cardinality 60 crashes at <= 5; cardinality 256 crashes at <= 6).
///
/// The library never violates it, because `HUF_optimalTableLog` is always
/// interposed:
///   * without `HUF_flags_optimalDepth` it returns
///     `FSE_optimalTableLog_internal(...)`, which floors the result at
///     `FSE_minTableLog(srcSize, maxSymbolValue)` — always >= ceil(log2(card));
///   * with `HUF_flags_optimalDepth` it searches upward from
///     `HUF_minTableLog(cardinality)`, so every value it *tries* is legal —
///     BUT when `maxTableLog < HUF_minTableLog(cardinality)` the loop body never
///     runs and it returns `maxTableLog` unchanged, which is then too small.
///
/// `cardinality <= 256`, so `HUF_minTableLog(cardinality) <= highbit32(256)+1 == 9`.
/// Requesting `tableLog == 0` ("use the default", 11) or `tableLog >= 9` is
/// therefore safe for *any* input, and that is what the sweeps below request
/// whenever `HUF_flags_optimalDepth` is set.
/// Stage tracer for localising crashes inside either `.so` (no Rust backtrace
/// survives a SIGSEGV). Enabled by `ZSTD_DIFF_TRACE=stages`.
fn stg(l: &Lib, name: &str) {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var("ZSTD_DIFF_TRACE").as_deref() == Ok("stages")) {
        eprintln!("    [{}] stage {}", l.tag, name);
    }
}

fn safe_table_log(tl: u32, flags: c_int) -> u32 {
    if flags & HUF_flags_optimalDepth != 0 && tl != 0 {
        tl.max(9)
    } else {
        tl
    }
}

#[derive(PartialEq, Debug)]
struct HufRun {
    cardinality: u32,
    min_table_log: u32,
    optimal_table_log: u32,
    build_ctable: R,
    ctable: Blob,
    header: u64,
    nb_bits: Vec<u32>,
    estimate: R,
    validate: c_int,
    write_ctable: R,
    ctable_bytes: Blob,
    read_ctable: R,
    read_msv: u32,
    has_zero_weights: u32,
    read_stats: R,
    stats_weights: Blob,
    stats_ranks: Vec<u32>,
    stats_nb_symbols: u32,
    stats_table_log: u32,
    comp1x: R,
    c1x: Blob,
    comp4x: R,
    c4x: Blob,
    select_decoder_1x: u32,
    select_decoder_4x: u32,
    read_dtable_x1: R,
    read_dtable_x2: R,
    dec1x_using: R,
    d1x_using: Blob,
    dec4x_using: R,
    d4x_using: Blob,
    dec1x_dctx: R,
    d1x_dctx: Blob,
    dec1x1_dctx: R,
    dec1x2_dctx: R,
    dec4x_hufonly: R,
    d4x_hufonly: Blob,
}

/// PRECONDITION, taken from the C: `srcSize > 1` — `HUF_optimalTableLog` opens
/// with `assert(srcSize > 1) /* Not supported, RLE should be used instead */`
/// and delegates to `FSE_optimalTableLog_internal`, whose `FSE_minTableLog`
/// carries the same assertion. Note `HUF_compress4X_usingCTable` additionally
/// returns 0 (not an error) for `srcSize < 12` or `dstSize < 17`, which the
/// sweep below covers deliberately by including sizes on both sides of 12.
fn huf_pipeline(l: &Lib, src: &[u8], table_log_req: u32, flags: c_int) -> HufRun {
    assert!(
        src.len() > 1,
        "huf_pipeline precondition: the C asserts srcSize > 1"
    );
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

    stg(l, "cardinality");
let cardinality =
        unsafe { l.sym::<FnHufCardinality>("HUF_cardinality")(count.as_ptr(), msv) };
    let min_table_log = unsafe { l.sym::<FnHufMinTableLog>("HUF_minTableLog")(cardinality) };

    let ct_len = HUF_SYMBOLVALUE_MAX as usize + 2;
    let mut ctable = vec![0u64; ct_len];
    let mut hw = wksp(HUF_WORKSPACE_SIZE);
    stg(l, "optimalTableLog");
let optimal_table_log = unsafe {
        l.sym::<FnHufOptimalTableLog>("HUF_optimalTableLog")(
            table_log_req,
            src.len(),
            msv,
            wksp_ptr(&mut hw),
            wksp_bytes(&hw),
            ctable.as_mut_ptr(),
            count.as_ptr(),
            flags,
        )
    };

    // HUF_optimalTableLog scribbles on `ctable` as scratch — rebuild from scratch.
    let mut ctable = vec![0u64; ct_len];
    let mut cw = wksp(HUF_CTABLE_WORKSPACE_SIZE);
stg(l, "buildCTable");
    let bc = unsafe {
        l.sym::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp")(
            ctable.as_mut_ptr(),
            count.as_ptr(),
            msv,
            optimal_table_log,
            wksp_ptr(&mut cw),
            wksp_bytes(&cw),
        )
    };
    let build_ctable = res(l, bc);
    // `HUF_compress_internal` does `CHECK_F(maxBits); huffLog = (U32)maxBits;`
    // — every later stage uses the *returned* depth, not the requested one, and
    // is skipped entirely if the build failed.
    let ok_ct = matches!(build_ctable, R::Ok(_));
    let huff_log = match build_ctable {
        R::Ok(v) => v as u32,
        R::Err(..) => optimal_table_log,
    };

    stg(l, "readCTableHeader");
let header = unsafe { l.sym::<FnHufReadCTableHeader>("HUF_readCTableHeader")(ctable.as_ptr()) };
    let nb_bits: Vec<u32> = if ok_ct {
        (0..=256u32)
            .map(|s| unsafe {
                l.sym::<FnHufGetNbBits>("HUF_getNbBitsFromCTable")(ctable.as_ptr(), s)
            })
            .collect()
    } else {
        Vec::new()
    };
    let est = if ok_ct {
        unsafe {
            l.sym::<FnHufEstimateCompressedSize>("HUF_estimateCompressedSize")(
                ctable.as_ptr(),
                count.as_ptr(),
                msv,
            )
        }
    } else {
        0
    };
    let estimate = res(l, est);
    let validate = if ok_ct {
        unsafe {
            l.sym::<FnHufValidateCTable>("HUF_validateCTable")(ctable.as_ptr(), count.as_ptr(), msv)
        }
    } else {
        -1
    };

    // serialise the table
    let mut ctbuf = vec![0xEEu8; 1024];
    let mut ww = wksp(HUF_WORKSPACE_SIZE);
stg(l, "writeCTable");
    let wc = if ok_ct {
        unsafe {
            l.sym::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp")(
                ctbuf.as_mut_ptr() as *mut c_void,
                ctbuf.len(),
                ctable.as_ptr(),
                msv,
                huff_log,
                wksp_ptr(&mut ww),
                wksp_bytes(&ww),
            )
        }
    } else {
        0
    };
    let write_ctable = res(l, wc);
    let ct_bytes = match write_ctable {
        R::Ok(v) => v,
        R::Err(..) => 0,
    };

    // read it back
    let mut ctable2 = vec![0u64; ct_len];
    let mut rmsv = 255u32;
    let mut hzw = 0u32;
stg(l, "readCTable");
    let rc = if ct_bytes > 0 {
        unsafe {
        l.sym::<FnHufReadCTable>("HUF_readCTable")(
            ctable2.as_mut_ptr(),
            &mut rmsv,
            ctbuf.as_ptr() as *const c_void,
            ct_bytes,
            &mut hzw,
        )
        }
    } else {
        0
    };
    let read_ctable = res(l, rc);

    // HUF_readStats / HUF_readStats_wksp on the same serialised table
    let mut weights = vec![0u8; 256];
    let mut ranks = vec![0u32; 16];
    let mut nb_symbols = 0u32;
    let mut tl_out = 0u32;
stg(l, "readStats");
    let rs = if ct_bytes > 0 {
        unsafe {
        l.sym::<FnHufReadStats>("HUF_readStats")(
            weights.as_mut_ptr(),
            weights.len(),
            ranks.as_mut_ptr(),
            &mut nb_symbols,
            &mut tl_out,
            ctbuf.as_ptr() as *const c_void,
            ct_bytes,
        )
        }
    } else {
        0
    };
    let read_stats = res(l, rs);
    if ct_bytes > 0 {
        let mut w2 = vec![0u8; 256];
        let mut r2 = vec![0u32; 16];
        let mut n2 = 0u32;
        let mut t2 = 0u32;
        let mut sw = wksp(4096);
        let rv = unsafe {
            l.sym::<FnHufReadStatsWksp>("HUF_readStats_wksp")(
                w2.as_mut_ptr(),
                w2.len(),
                r2.as_mut_ptr(),
                &mut n2,
                &mut t2,
                ctbuf.as_ptr() as *const c_void,
                ct_bytes,
                wksp_ptr(&mut sw),
                wksp_bytes(&sw),
                0,
            )
        };
        assert_eq!(rv, rs, "[{}] readStats_wksp != readStats", l.tag);
        assert_eq!(w2, weights, "[{}] readStats_wksp weights", l.tag);
        assert_eq!((n2, t2), (nb_symbols, tl_out), "[{}] readStats_wksp out", l.tag);
    }

    // encode
    let cbound = unsafe { l.sym::<FnSzSz>("HUF_compressBound")(src.len()) };
    let mut b1 = vec![0xD1u8; cbound + 64];
stg(l, "compress1X");
    let c1 = if ok_ct && !src.is_empty() {
        unsafe {
            l.sym::<FnHufCompressUsingCTable>("HUF_compress1X_usingCTable")(
                b1.as_mut_ptr() as *mut c_void,
                b1.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ctable.as_ptr(),
                flags,
            )
        }
    } else {
        0
    };
    let comp1x = res(l, c1);
    let l1 = match comp1x {
        R::Ok(v) => v,
        _ => 0,
    };
    let mut b4 = vec![0xD4u8; cbound + 64];
stg(l, "compress4X");
    let c4 = if ok_ct && !src.is_empty() {
        unsafe {
            l.sym::<FnHufCompressUsingCTable>("HUF_compress4X_usingCTable")(
                b4.as_mut_ptr() as *mut c_void,
                b4.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ctable.as_ptr(),
                flags,
            )
        }
    } else {
        0
    };
    let comp4x = res(l, c4);
    let l4 = match comp4x {
        R::Ok(v) => v,
        _ => 0,
    };

    let select_decoder_1x =
        unsafe { l.sym::<FnHufSelectDecoder>("HUF_selectDecoder")(src.len().max(1), l1.max(1)) };
    let select_decoder_4x =
        unsafe { l.sym::<FnHufSelectDecoder>("HUF_selectDecoder")(src.len().max(1), l4.max(1)) };

    // decode: build both DTables from the serialised CTable
    let mut dt1 = huf_dtable();
    let mut dw = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
stg(l, "readDTableX1");
    let rd1 = if ct_bytes > 0 {
        unsafe {
        l.sym::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp")(
            dt1.as_mut_ptr(),
            ctbuf.as_ptr() as *const c_void,
            ct_bytes,
            wksp_ptr(&mut dw),
            wksp_bytes(&dw),
            flags,
        )
        }
    } else {
        0
    };
    let read_dtable_x1 = res(l, rd1);

    let mut dt2 = huf_dtable();
    let mut dw2 = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
stg(l, "readDTableX2");
    let rd2 = if ct_bytes > 0 {
        unsafe {
        l.sym::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp")(
            dt2.as_mut_ptr(),
            ctbuf.as_ptr() as *const c_void,
            ct_bytes,
            wksp_ptr(&mut dw2),
            wksp_bytes(&dw2),
            flags,
        )
        }
    } else {
        0
    };
    let read_dtable_x2 = res(l, rd2);

    // Decoding with a DTable that was never populated is out of contract, not
    // just useless: `HUF_decodeSymbolX1` is documented `note : dtLog >= 1`, and
    // with `dtLog == 0` `BIT_lookBitsFast` shifts by `(64 - 0) & 63 == 0` and
    // returns the whole 64-bit bit container as the table index — an immediate
    // wild read (SIGSEGV in the reference C). So only decode once the table has
    // actually been read successfully.
    let ok_dt1 = ct_bytes > 0 && matches!(read_dtable_x1, R::Ok(_));

    let mut o1 = vec![0xA1u8; src.len() + 64];
stg(l, "decompress1X_usingDTable");
    let d1 = if l1 > 0 && ok_dt1 {
        unsafe {
            l.sym::<FnHufDecompressUsingDTable>("HUF_decompress1X_usingDTable")(
                o1.as_mut_ptr() as *mut c_void,
                src.len(),
                b1.as_ptr() as *const c_void,
                l1,
                dt1.as_ptr(),
                flags,
            )
        }
    } else {
        0
    };
    let dec1x_using = res(l, d1);
    let d1len = match dec1x_using {
        R::Ok(v) => v,
        _ => 0,
    };

    let mut o4 = vec![0xA4u8; src.len() + 64];
stg(l, "decompress4X_usingDTable");
    let d4 = if l4 > 0 && ok_dt1 {
        unsafe {
            l.sym::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable")(
                o4.as_mut_ptr() as *mut c_void,
                src.len(),
                b4.as_ptr() as *const c_void,
                l4,
                dt1.as_ptr(),
                flags,
            )
        }
    } else {
        0
    };
    let dec4x_using = res(l, d4);
    let d4len = match dec4x_using {
        R::Ok(v) => v,
        _ => 0,
    };

    // full frames: serialised table ++ bitstream, via the *_DCtx_wksp entries
    let mut frame1 = Vec::new();
    frame1.extend_from_slice(&ctbuf[..ct_bytes]);
    frame1.extend_from_slice(&b1[..l1]);
    let mut frame4 = Vec::new();
    frame4.extend_from_slice(&ctbuf[..ct_bytes]);
    frame4.extend_from_slice(&b4[..l4]);

    let mut dctx = huf_dtable();
    let mut dcw = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
    let mut od = vec![0xC1u8; src.len() + 64];
stg(l, "decompress1X_DCtx");
    let dd = if ct_bytes > 0 && l1 > 0 {
        unsafe {
            l.sym::<FnHufDecompressDCtxWksp>("HUF_decompress1X_DCtx_wksp")(
                dctx.as_mut_ptr(),
                od.as_mut_ptr() as *mut c_void,
                src.len(),
                frame1.as_ptr() as *const c_void,
                frame1.len(),
                wksp_ptr(&mut dcw),
                wksp_bytes(&dcw),
                flags,
            )
        }
    } else {
        0
    };
    let dec1x_dctx = res(l, dd);
    let ddlen = match dec1x_dctx {
        R::Ok(v) => v,
        _ => 0,
    };

    let mut dctx1 = huf_dtable();
    let mut dcw1 = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
    let mut od1 = vec![0xC2u8; src.len() + 64];
stg(l, "decompress1X1_DCtx");
    let dd1 = if ct_bytes > 0 && l1 > 0 {
        unsafe {
            l.sym::<FnHufDecompressDCtxWksp>("HUF_decompress1X1_DCtx_wksp")(
                dctx1.as_mut_ptr(),
                od1.as_mut_ptr() as *mut c_void,
                src.len(),
                frame1.as_ptr() as *const c_void,
                frame1.len(),
                wksp_ptr(&mut dcw1),
                wksp_bytes(&dcw1),
                flags,
            )
        }
    } else {
        0
    };
    let dec1x1_dctx = res(l, dd1);

    let mut dctx2 = huf_dtable();
    let mut dcw2 = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
    let mut od2 = vec![0xC3u8; src.len() + 64];
stg(l, "decompress1X2_DCtx");
    let dd2 = if ct_bytes > 0 && l1 > 0 {
        unsafe {
            l.sym::<FnHufDecompressDCtxWksp>("HUF_decompress1X2_DCtx_wksp")(
                dctx2.as_mut_ptr(),
                od2.as_mut_ptr() as *mut c_void,
                src.len(),
                frame1.as_ptr() as *const c_void,
                frame1.len(),
                wksp_ptr(&mut dcw2),
                wksp_bytes(&dcw2),
                flags,
            )
        }
    } else {
        0
    };
    let dec1x2_dctx = res(l, dd2);

    let mut dctx4 = huf_dtable();
    let mut dcw4 = wksp(HUF_DECOMPRESS_WORKSPACE_SIZE);
    let mut od4 = vec![0xC4u8; src.len() + 64];
stg(l, "decompress4X_hufOnly");
    let dd4 = if ct_bytes > 0 && l4 > 0 {
        unsafe {
            l.sym::<FnHufDecompressDCtxWksp>("HUF_decompress4X_hufOnly_wksp")(
                dctx4.as_mut_ptr(),
                od4.as_mut_ptr() as *mut c_void,
                src.len(),
                frame4.as_ptr() as *const c_void,
                frame4.len(),
                wksp_ptr(&mut dcw4),
                wksp_bytes(&dcw4),
                flags,
            )
        }
    } else {
        0
    };
    let dec4x_hufonly = res(l, dd4);
    let dd4len = match dec4x_hufonly {
        R::Ok(v) => v,
        _ => 0,
    };

    HufRun {
        cardinality,
        min_table_log,
        optimal_table_log,
        build_ctable,
        ctable: Blob(ctable.iter().flat_map(|v| v.to_le_bytes()).collect()),
        header,
        nb_bits,
        estimate,
        validate,
        write_ctable,
        ctable_bytes: Blob(ctbuf[..ct_bytes].to_vec()),
        read_ctable,
        read_msv: rmsv,
        has_zero_weights: hzw,
        read_stats,
        stats_weights: Blob(weights),
        stats_ranks: ranks,
        stats_nb_symbols: nb_symbols,
        stats_table_log: tl_out,
        comp1x,
        c1x: Blob(b1[..l1].to_vec()),
        comp4x,
        c4x: Blob(b4[..l4].to_vec()),
        select_decoder_1x,
        select_decoder_4x,
        read_dtable_x1,
        read_dtable_x2,
        dec1x_using,
        d1x_using: Blob(o1[..d1len].to_vec()),
        dec4x_using,
        d4x_using: Blob(o4[..d4len].to_vec()),
        dec1x_dctx,
        d1x_dctx: Blob(od[..ddlen].to_vec()),
        dec1x1_dctx,
        dec1x2_dctx,
        dec4x_hufonly,
        d4x_hufonly: Blob(od4[..dd4len].to_vec()),
    }
}

#[test]
fn huf_pipeline_matches_over_corpora_sizes_tablelogs_and_flags() {
    covers(&["CFG:264", "CFG:265", "CFG:266", "CFG:267", "CFG:268", "CFG:269",
             "CFG:270", "CFG:271", "CFG:272", "CFG:273", "CFG:275", "CFG:284",
             "CFG:286", "CFG:287", "CFG:288", "CFG:291", "CFG:292",
             "ERR:common/entropy_common.c:254", "ERR:common/entropy_common.c:261",
             "ERR:common/entropy_common.c:280", "ERR:common/entropy_common.c:284",
             "ERR:common/entropy_common.c:288", "ERR:common/entropy_common.c:295"]);
    let mut cases = 0usize;
    for &k in ALL_CORPORA {
        for &n in &[2usize, 3, 5, 11, 12, 13, 16, 64, 255, 256, 1000, 4096, 65536] {
            let src = corpus(k, n, 0x8055_0000 ^ n as u64);
            for tl in [0u32, 5, 8, HUF_TABLELOG_DEFAULT, HUF_TABLELOG_MAX] {
                for flags in [
                    0,
                    // `DYNAMIC_BMI2=0` in this build, so `HUF_flags_bmi2` must be
                    // a no-op: every `_bmi2` dispatcher takes the `_default`
                    // path. Including it pins that.
                    HUF_flags_bmi2,
                    HUF_flags_optimalDepth,
                    HUF_flags_preferRepeat,
                    HUF_flags_disableAsm,
                    HUF_flags_disableFast,
                    HUF_flags_disableAsm | HUF_flags_disableFast,
                    HUF_flags_optimalDepth | HUF_flags_suspectUncompressible,
                ] {
                    let tl = safe_table_log(tl, flags);
                    cases += 1;
                    diff(
                        &format!("HUF pipeline {k:?} n={n} tl={tl} flags={flags}"),
                        |l| huf_pipeline(l, &src, tl, flags),
                    );
                }
            }
        }
    }
    assert!(cases > 500, "expected a wide sweep, ran {cases}");
    eprintln!("HUF pipeline: {cases} configurations compared");
}

#[test]
fn huf_pipeline_matches_on_randomised_inputs() {
    covers(&["CFG:266", "CFG:267", "CFG:271", "CFG:284", "CFG:288", "CFG:291"]);
    let mut rng = Rng::new(0x8055_F00D);
    for i in 0..300 {
        let k = *rng.pick(ALL_CORPORA);
        let n = 2 + rng.below(9000);
        let src = corpus(k, n, rng.next_u64());
        let flags = (rng.next_u32() & 0x3F) as c_int;
        let tl = safe_table_log(rng.range(0, HUF_TABLELOG_MAX as i64) as u32, flags);
        diff(
            &format!("HUF random #{i} {k:?} n={n} tl={tl} flags={flags}"),
            |l| huf_pipeline(l, &src, tl, flags),
        );
    }
}

/// `HUF_compress{1,4}X_repeat` drives the repeat-table logic that the block
/// compressor relies on: first call builds a table, second call reuses it.
#[test]
fn huf_compress_repeat_matches() {
    covers(&["CFG:277", "CFG:278", "CFG:279", "CFG:280", "CFG:281", "CFG:282",
             "CFG:283"]);
    let mut rng = Rng::new(0x8E9E_A700);
    for i in 0..120 {
        let k = *rng.pick(ALL_CORPORA);
        let n = 2 + rng.below(20000);
        let a = corpus(k, n, rng.next_u64());
        let b = corpus(k, n, rng.next_u64());
        let msv = *rng.pick(&[0u32, 15, 63, 255]);
        let flags = (rng.next_u32() & 0x3F) as c_int;
        let tl = safe_table_log(rng.range(0, HUF_TABLELOG_MAX as i64) as u32, flags);
        let four = rng.bool();
        let sym = if four {
            "HUF_compress4X_repeat"
        } else {
            "HUF_compress1X_repeat"
        };
        for &start_repeat in &[0i32, 1, 2] {
            diff(
                &format!("{sym} #{i} {k:?} n={n} tl={tl} msv={msv} flags={flags} repeat0={start_repeat}"),
                |l| {
                    let f = l.sym::<FnHufCompressRepeat>(sym);
                    let bound = unsafe { l.sym::<FnSzSz>("HUF_compressBound")(n) };
                    let mut table = vec![0u64; HUF_SYMBOLVALUE_MAX as usize + 2];
                    let mut repeat = start_repeat;
                    let mut w = wksp(HUF_WORKSPACE_SIZE);
                    let mut d1 = vec![0x11u8; bound + 64];
                    let n1 = unsafe {
                        f(
                            d1.as_mut_ptr() as *mut c_void,
                            d1.len(),
                            a.as_ptr() as *const c_void,
                            a.len(),
                            msv,
                            tl,
                            wksp_ptr(&mut w),
                            wksp_bytes(&w),
                            table.as_mut_ptr(),
                            &mut repeat,
                            flags,
                        )
                    };
                    let r1 = res(l, n1);
                    let rep1 = repeat;
                    let t1 = Blob(table.iter().flat_map(|v| v.to_le_bytes()).collect());
                    // second call reuses the table produced by the first
                    let mut d2 = vec![0x22u8; bound + 64];
                    let n2 = unsafe {
                        f(
                            d2.as_mut_ptr() as *mut c_void,
                            d2.len(),
                            b.as_ptr() as *const c_void,
                            b.len(),
                            msv,
                            tl,
                            wksp_ptr(&mut w),
                            wksp_bytes(&w),
                            table.as_mut_ptr(),
                            &mut repeat,
                            flags,
                        )
                    };
                    let r2 = res(l, n2);
                    let l1 = match r1 {
                        R::Ok(v) => v,
                        _ => 0,
                    };
                    let l2 = match r2 {
                        R::Ok(v) => v,
                        _ => 0,
                    };
                    (
                        r1,
                        rep1,
                        t1,
                        Blob(d1[..l1].to_vec()),
                        r2,
                        repeat,
                        Blob(table.iter().flat_map(|v| v.to_le_bytes()).collect()),
                        Blob(d2[..l2].to_vec()),
                    )
                },
            );
        }
    }
}

/// `DYNAMIC_BMI2=0` and no `__BMI2__` in this build, so `ZSTD_ENABLE_ASM_X86_64_BMI2`
/// is 0 and `HUF_flags_bmi2` selects nothing: setting it must leave every output
/// bit-identical. Verified against the C, not assumed.
#[test]
fn huf_flags_bmi2_is_a_no_op_in_this_build() {
    covers(&["CFG:275", "CFG:288"]);
    let mut rng = Rng::new(0xB312_0000);
    for i in 0..40 {
        let k = *rng.pick(ALL_CORPORA);
        let n = 2 + rng.below(20000);
        let src = corpus(k, n, rng.next_u64());
        let base = *rng.pick(&[0, HUF_flags_disableFast, HUF_flags_optimalDepth]);
        let tl = safe_table_log(rng.range(0, HUF_TABLELOG_MAX as i64) as u32, base);
        for l in [&pair().c, &pair().r] {
            let without = huf_pipeline(l, &src, tl, base);
            let with = huf_pipeline(l, &src, tl, base | HUF_flags_bmi2);
            assert_eq!(
                without, with,
                "[{}] HUF_flags_bmi2 changed the result (case #{i} {k:?} n={n} tl={tl})",
                l.tag
            );
        }
    }
}

#[test]
fn huf_select_decoder_matches_over_grid() {
    covers(&["CFG:285"]);
    for dst in [
        1usize, 2, 3, 100, 1000, 1024, 16384, 65536, 131072, HUF_BLOCKSIZE_MAX,
    ] {
        for csrc in [1usize, 2, 3, 10, 100, 1000, 5000, 60000, 131072] {
            diff(&format!("HUF_selectDecoder({dst},{csrc})"), |l| unsafe {
                l.sym::<FnHufSelectDecoder>("HUF_selectDecoder")(dst, csrc)
            });
        }
    }
}

#[test]
fn huf_cardinality_and_min_table_log_match() {
    covers(&["CFG:264", "CFG:265"]);
    let mut rng = Rng::new(0xCA4D_0001);
    for _ in 0..300 {
        let msv = rng.range(0, 255) as u32;
        let mut count = vec![0u32; 256];
        for i in 0..=msv as usize {
            count[i] = if rng.below(3) == 0 {
                0
            } else {
                rng.below(1000) as u32
            };
        }
        diff("HUF_cardinality", |l| unsafe {
            l.sym::<FnHufCardinality>("HUF_cardinality")(count.as_ptr(), msv)
        });
    }
    for c in 0..=300u32 {
        diff(&format!("HUF_minTableLog({c})"), |l| unsafe {
            l.sym::<FnHufMinTableLog>("HUF_minTableLog")(c)
        });
    }
}

// ---------------------------------------------------------------------------
// xxhash — the checksum the frame format depends on
// ---------------------------------------------------------------------------

#[test]
fn xxhash_one_shot_matches_over_all_lengths_and_seeds() {
    covers(&["CFG:301", "CFG:304"]);
    let mut rng = Rng::new(0x8888_0001);
    let big = rng.bytes(4096);
    for len in 0..=600usize {
        for seed in [0u32, 1, 0xDEADBEEF, u32::MAX] {
            diff(&format!("ZSTD_XXH32 len={len} seed={seed}"), |l| unsafe {
                l.sym::<FnXxh32>("ZSTD_XXH32")(big.as_ptr() as *const c_void, len, seed)
            });
        }
        for seed in [0u64, 1, 0x0123_4567_89AB_CDEF, u64::MAX] {
            diff(&format!("ZSTD_XXH64 len={len} seed={seed}"), |l| unsafe {
                l.sym::<FnXxh64>("ZSTD_XXH64")(big.as_ptr() as *const c_void, len, seed)
            });
        }
    }
    // long inputs: cross the 16/32-byte stripe and the 4 KB accumulator paths
    for len in [1000usize, 2048, 4096] {
        diff(&format!("ZSTD_XXH32 long {len}"), |l| unsafe {
            l.sym::<FnXxh32>("ZSTD_XXH32")(big.as_ptr() as *const c_void, len, 7)
        });
        diff(&format!("ZSTD_XXH64 long {len}"), |l| unsafe {
            l.sym::<FnXxh64>("ZSTD_XXH64")(big.as_ptr() as *const c_void, len, 7)
        });
    }
}

#[derive(PartialEq, Debug)]
struct XxhRun {
    reset32: c_int,
    reset64: c_int,
    h32: u32,
    h64: u64,
    canon32: [u8; 4],
    canon64: [u8; 8],
    back32: u32,
    back64: u64,
    mids32: Vec<u32>,
    mids64: Vec<u64>,
    frees: [c_int; 4],
}

#[test]
fn xxhash_streaming_matches_with_arbitrary_chunking() {
    covers(&["CFG:302", "CFG:303", "CFG:305", "CFG:306"]);
    let mut rng = Rng::new(0x8888_0002);
    let data = rng.bytes(9000);
    for trial in 0..40 {
        // build a random chunk schedule, identical for both libraries
        let mut chunks = Vec::new();
        let mut pos = 0usize;
        let mut r2 = Rng::new(0xC401 + trial);
        while pos < data.len() {
            let n = (1 + r2.below(700)).min(data.len() - pos);
            chunks.push((pos, n));
            pos += n;
        }
        diff(&format!("XXH streaming trial {trial}"), |l| {
            let cs32 = l.sym::<FnXxhCreateState>("ZSTD_XXH32_createState");
            let fs32 = l.sym::<FnXxhFreeState>("ZSTD_XXH32_freeState");
            let rs32 = l.sym::<FnXxh32Reset>("ZSTD_XXH32_reset");
            let up32 = l.sym::<FnXxhUpdate>("ZSTD_XXH32_update");
            let dg32 = l.sym::<FnXxh32Digest>("ZSTD_XXH32_digest");
            let cp32 = l.sym::<FnXxhCopyState>("ZSTD_XXH32_copyState");
            let cs64 = l.sym::<FnXxhCreateState>("ZSTD_XXH64_createState");
            let fs64 = l.sym::<FnXxhFreeState>("ZSTD_XXH64_freeState");
            let rs64 = l.sym::<FnXxh64Reset>("ZSTD_XXH64_reset");
            let up64 = l.sym::<FnXxhUpdate>("ZSTD_XXH64_update");
            let dg64 = l.sym::<FnXxh64Digest>("ZSTD_XXH64_digest");
            let cp64 = l.sym::<FnXxhCopyState>("ZSTD_XXH64_copyState");
            unsafe {
                let s32 = cs32();
                let t32 = cs32();
                let s64 = cs64();
                let t64 = cs64();
                assert!(!s32.is_null() && !s64.is_null() && !t32.is_null() && !t64.is_null());
                let e1 = rs32(s32, 0x1234);
                let e2 = rs64(s64, 0x9876);
                let mut mids32 = Vec::new();
                let mut mids64 = Vec::new();
                for (i, &(off, n)) in chunks.iter().enumerate() {
                    let p = data[off..].as_ptr() as *const c_void;
                    up32(s32, p, n);
                    up64(s64, p, n);
                    if i % 3 == 0 {
                        // copyState must produce a state that digests identically
                        cp32(t32, s32);
                        cp64(t64, s64);
                        mids32.push(dg32(t32));
                        mids64.push(dg64(t64));
                    }
                }
                let h32 = dg32(s32);
                let h64 = dg64(s64);
                // canonical round-trip
                let mut can32 = [0u8; 4];
                let mut can64 = [0u8; 8];
                l.sym::<FnXxh32Canon>("ZSTD_XXH32_canonicalFromHash")(
                    can32.as_mut_ptr() as *mut c_void,
                    h32,
                );
                l.sym::<FnXxh64Canon>("ZSTD_XXH64_canonicalFromHash")(
                    can64.as_mut_ptr() as *mut c_void,
                    h64,
                );
                let b32 = l.sym::<FnXxh32FromCanon>("ZSTD_XXH32_hashFromCanonical")(
                    can32.as_ptr() as *const c_void,
                );
                let b64 = l.sym::<FnXxh64FromCanon>("ZSTD_XXH64_hashFromCanonical")(
                    can64.as_ptr() as *const c_void,
                );
                XxhRun {
                    reset32: e1,
                    reset64: e2,
                    h32,
                    h64,
                    canon32: can32,
                    canon64: can64,
                    back32: b32,
                    back64: b64,
                    mids32,
                    mids64,
                    frees: [fs32(s32), fs32(t32), fs64(s64), fs64(t64)],
                }
            }
        });
    }
}

#[test]
fn xxhash_free_state_null_matches() {
    covers(&["CFG:302", "CFG:305"]);
    diff("ZSTD_XXH32_freeState(NULL)", |l| unsafe {
        l.sym::<FnXxhFreeState>("ZSTD_XXH32_freeState")(std::ptr::null_mut())
    });
    diff("ZSTD_XXH64_freeState(NULL)", |l| unsafe {
        l.sym::<FnXxhFreeState>("ZSTD_XXH64_freeState")(std::ptr::null_mut())
    });
    // NOTE: `XXH32_reset`, `XXH64_reset`, `XXH*_update` and `XXH*_digest`
    // document `@pre statePtr must not be NULL` and enforce it only with
    // `XXH_ASSERT`, which is compiled out here. Passing NULL makes the
    // reference C build dereference it and take SIGSEGV, so there is no C
    // behaviour to match and those calls are deliberately NOT made.
    // `XXH*_freeState(NULL)` above IS in contract (it is just `free(NULL)`).
    diff("ZSTD_XXH32(NULL,0,seed)", |l| unsafe {
        l.sym::<FnXxh32>("ZSTD_XXH32")(std::ptr::null(), 0, 12345)
    });
    diff("ZSTD_XXH64(NULL,0,seed)", |l| unsafe {
        l.sym::<FnXxh64>("ZSTD_XXH64")(std::ptr::null(), 0, 12345)
    });
    diff("live state update(NULL,0)", |l| unsafe {
        let s = l.sym::<FnXxhCreateState>("ZSTD_XXH64_createState")();
        let a = l.sym::<FnXxh64Reset>("ZSTD_XXH64_reset")(s, 0);
        let b = l.sym::<FnXxhUpdate>("ZSTD_XXH64_update")(s, std::ptr::null(), 0);
        let d = l.sym::<FnXxh64Digest>("ZSTD_XXH64_digest")(s);
        let f = l.sym::<FnXxhFreeState>("ZSTD_XXH64_freeState")(s);
        (a, b, d, f)
    });
}

// ---------------------------------------------------------------------------
// divsufsort / divbwt — used by the dictionary builder, exported directly
// ---------------------------------------------------------------------------

#[test]
fn divsufsort_matches_over_shapes_and_sizes() {
    covers(&["CFG:309", "CFG:382", "CFG:383"]);
    let mut rng = Rng::new(0x0D1F_5001);
    for &k in ALL_CORPORA {
        for &n in &[0usize, 1, 2, 3, 5, 16, 100, 257, 1000, 4000] {
            let src = corpus(k, n, 0x0D1F ^ n as u64);
            for openMP in [0i32, 1] {
                diff(
                    &format!("divsufsort {k:?} n={n} openMP={openMP}"),
                    |l| {
                        let mut sa = vec![-1i32; n.max(1) + 8];
                        let rc = unsafe {
                            l.sym::<FnDivsufsort>("divsufsort")(
                                src.as_ptr(),
                                sa.as_mut_ptr(),
                                n as i32,
                                openMP,
                            )
                        };
                        (rc, sa)
                    },
                );
            }
        }
    }
    // randomized
    for i in 0..60 {
        let n = rng.below(3000);
        let k = *rng.pick(ALL_CORPORA);
        let src = corpus(k, n, rng.next_u64());
        diff(&format!("divsufsort random #{i} n={n}"), |l| {
            let mut sa = vec![-1i32; n.max(1) + 8];
            let rc = unsafe {
                l.sym::<FnDivsufsort>("divsufsort")(src.as_ptr(), sa.as_mut_ptr(), n as i32, 0)
            };
            (rc, sa)
        });
    }
}

#[test]
fn divbwt_matches_over_shapes_and_sizes() {
    covers(&["CFG:309", "CFG:386"]);
    for &k in ALL_CORPORA {
        for &n in &[0usize, 1, 2, 3, 5, 16, 100, 257, 1000] {
            let src = corpus(k, n, 0xB07 ^ n as u64);
            diff(&format!("divbwt {k:?} n={n}"), |l| {
                let mut t = src.clone();
                t.resize(n.max(1) + 8, 0);
                let mut u = vec![0u8; n.max(1) + 8];
                let mut a = vec![0i32; n.max(1) + 8];
                let rc = unsafe {
                    l.sym::<FnDivbwt>("divbwt")(
                        t.as_mut_ptr(),
                        u.as_mut_ptr(),
                        a.as_mut_ptr(),
                        n as i32,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                (rc, t, u, a)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// POOL_* — with ZSTD_MULTITHREAD undefined these are the synchronous stubs
// ---------------------------------------------------------------------------

extern "C" fn noop_job(_: *mut c_void) {}

#[test]
fn pool_family_matches_single_threaded_build() {
    covers(&["CFG:239", "CFG:240", "ERR:common/pool.c:366"]);
    for nb_threads in [0usize, 1, 2, 4] {
        for queue in [0usize, 1, 4] {
            diff(&format!("POOL nbThreads={nb_threads} queue={queue}"), |l| {
                unsafe {
                    let p = l.sym::<FnPoolCreate>("POOL_create")(nb_threads, queue);
                    let sz = if p.is_null() {
                        0
                    } else {
                        l.sym::<FnPoolSizeof>("POOL_sizeof")(p)
                    };
                    let rs = if p.is_null() {
                        -999
                    } else {
                        l.sym::<FnPoolResize>("POOL_resize")(p, 3)
                    };
                    let ta = if p.is_null() {
                        -999
                    } else {
                        l.sym::<FnPoolTryAdd>("POOL_tryAdd")(p, Some(noop_job), std::ptr::null_mut())
                    };
                    if !p.is_null() {
                        l.sym::<FnPoolAdd>("POOL_add")(p, Some(noop_job), std::ptr::null_mut());
                        l.sym::<FnPoolJoinJobs>("POOL_joinJobs")(p);
                        l.sym::<FnPoolFree>("POOL_free")(p);
                    }
                    (p.is_null(), sz, rs, ta)
                }
            });
        }
    }
    // POOL_sizeof / POOL_free / POOL_resize on NULL
    diff("POOL_sizeof(NULL)", |l| unsafe {
        l.sym::<FnPoolSizeof>("POOL_sizeof")(std::ptr::null_mut())
    });
    diff("POOL_free(NULL)", |l| unsafe {
        l.sym::<FnPoolFree>("POOL_free")(std::ptr::null_mut());
        0
    });
    diff("POOL_resize(NULL)", |l| unsafe {
        l.sym::<FnPoolResize>("POOL_resize")(std::ptr::null_mut(), 2)
    });
    diff("POOL_joinJobs(NULL)", |l| unsafe {
        l.sym::<FnPoolJoinJobs>("POOL_joinJobs")(std::ptr::null_mut());
        0
    });
}

#[test]
fn threading_useless_symbol_is_present_in_both() {
    let p = pair();
    assert!(p.c.has("g_ZSTD_threading_useless_symbol"));
    assert!(p.r.has("g_ZSTD_threading_useless_symbol"));
    assert!(p.c.has("g_debuglevel"));
    assert!(p.r.has("g_debuglevel"));
}

// keep the c_char import used
const _: Option<*const c_char> = None;

// ===========================================================================
// Remaining CONFIGS.md rows for the low-level surface
// ===========================================================================

/// CONFIGS 239/240 — `POOL_*` with a real job that records its execution order.
/// With `ZSTD_MULTITHREAD` undefined these are the synchronous stubs, so a job
/// must run inline on `POOL_add`/`POOL_tryAdd` and the log order is part of the
/// observable behaviour.
#[test]
fn pool_jobs_execute_inline_in_the_same_order() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static LOG: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn job_a(_: *mut c_void) {
        LOG.fetch_add(1, Ordering::SeqCst);
    }
    extern "C" fn job_b(_: *mut c_void) {
        LOG.fetch_add(100, Ordering::SeqCst);
    }
    covers(&["CFG:239", "CFG:240"]);
    for nb in [0usize, 1, 2, 8] {
        for q in [0usize, 1, 8] {
            diff(&format!("POOL job log nb={nb} q={q}"), |l| unsafe {
                LOG.store(0, Ordering::SeqCst);
                let p = l.sym::<FnPoolCreate>("POOL_create")(nb, q);
                if p.is_null() {
                    return (true, 0usize, 0usize, -1, -1);
                }
                let sz0 = l.sym::<FnPoolSizeof>("POOL_sizeof")(p);
                l.sym::<FnPoolAdd>("POOL_add")(p, Some(job_a), std::ptr::null_mut());
                let t1 = l.sym::<FnPoolTryAdd>("POOL_tryAdd")(p, Some(job_b), std::ptr::null_mut());
                l.sym::<FnPoolAdd>("POOL_add")(p, Some(job_a), std::ptr::null_mut());
                l.sym::<FnPoolJoinJobs>("POOL_joinJobs")(p);
                let r = l.sym::<FnPoolResize>("POOL_resize")(p, nb + 1);
                let sz1 = l.sym::<FnPoolSizeof>("POOL_sizeof")(p);
                l.sym::<FnPoolFree>("POOL_free")(p);
                (false, sz0, sz1, t1, r)
            });
            // the job log must also match; read it after both runs finished
            diff(&format!("POOL job count nb={nb} q={q}"), |l| unsafe {
                LOG.store(0, Ordering::SeqCst);
                let p = l.sym::<FnPoolCreate>("POOL_create")(nb, q);
                if !p.is_null() {
                    l.sym::<FnPoolAdd>("POOL_add")(p, Some(job_a), std::ptr::null_mut());
                    l.sym::<FnPoolTryAdd>("POOL_tryAdd")(p, Some(job_b), std::ptr::null_mut());
                    l.sym::<FnPoolJoinJobs>("POOL_joinJobs")(p);
                    l.sym::<FnPoolFree>("POOL_free")(p);
                }
                LOG.load(Ordering::SeqCst)
            });
        }
    }
}

/// CONFIGS 249/250/252/253/254/257 — `FSE_writeNCount` / `FSE_readNCount` /
/// `FSE_buildCTable_wksp` on hand-built normalised distributions that hit the
/// header encoder's special cases: runs of zero-probability symbols (the
/// "repeat zero" escape is emitted in groups of 24, so run lengths 2, 3, 23, 24,
/// 25, 26 and 48 all take different paths), the `-1` low-probability marker, and
/// a distribution whose absolute values do not sum to `1 << tableLog`.
#[test]
fn fse_ncount_header_special_cases() {
    covers(&["CFG:249", "CFG:250", "CFG:252", "CFG:253", "CFG:254", "CFG:257"]);

    /// Build a normalised counter with `zero_run` zero-probability symbols
    /// between each pair of live symbols, summing to exactly `1 << table_log`.
    fn build(table_log: u32, zero_run: usize, use_low_prob: bool) -> (Vec<i16>, u32) {
        let total = 1i32 << table_log;
        let mut norm = vec![0i16; 256];
        let mut idx = 0usize;
        let mut live: Vec<usize> = Vec::new();
        while idx < 256 && live.len() < 12 {
            live.push(idx);
            idx += 1 + zero_run;
        }
        let msv = *live.last().unwrap() as u32;
        let mut left = total;
        for (i, &s) in live.iter().enumerate() {
            let v = if use_low_prob && i % 3 == 1 {
                -1
            } else if i + 1 == live.len() {
                left.max(1) as i16
            } else {
                let share = (total / (live.len() as i32 * 2)).max(1);
                share as i16
            };
            norm[s] = v;
            left -= v.max(1) as i32;
        }
        // fix up the last live symbol so the magnitudes sum to `total`
        let sum: i32 = norm.iter().map(|&v| if v == -1 { 1 } else { v as i32 }).sum();
        let last = *live.last().unwrap();
        let adj = norm[last] as i32 + (total - sum);
        norm[last] = adj.clamp(1, total) as i16;
        (norm, msv)
    }

    for table_log in [5u32, 6, 9, 11, 12] {
        for zero_run in [0usize, 1, 2, 3, 22, 23, 24, 25, 26, 48] {
            for low in [false, true] {
                let (norm, msv) = build(table_log, zero_run, low);
                diff(
                    &format!("writeNCount tl={table_log} zeroRun={zero_run} lowProb={low}"),
                    |l| {
                        let bound = unsafe {
                            l.sym::<FnFseNCountWriteBound>("FSE_NCountWriteBound")(msv, table_log)
                        };
                        // exact bound, bound-1 (too small) and a generous buffer
                        let mut outs = Vec::new();
                        for cap in [bound, bound.saturating_sub(1), bound + 32] {
                            let mut buf = vec![0x5Au8; cap.max(1)];
                            let n = unsafe {
                                l.sym::<FnFseWriteNCount>("FSE_writeNCount")(
                                    buf.as_mut_ptr() as *mut c_void,
                                    cap,
                                    norm.as_ptr(),
                                    msv,
                                    table_log,
                                )
                            };
                            let r = res(l, n);
                            let len = match r {
                                R::Ok(v) => v,
                                _ => 0,
                            };
                            // read the header back, including with a *smaller*
                            // maxSymbolValuePtr than the header encodes
                            let mut reads = Vec::new();
                            for req_msv in [255u32, msv, msv.saturating_sub(1), 0] {
                                let mut rn = vec![0i16; 256];
                                let mut rm = req_msv;
                                let mut rt = FSE_TABLELOG_ABSOLUTE_MAX;
                                let rv = unsafe {
                                    l.sym::<FnFseReadNCount>("FSE_readNCount")(
                                        rn.as_mut_ptr(),
                                        &mut rm,
                                        &mut rt,
                                        buf.as_ptr() as *const c_void,
                                        len,
                                    )
                                };
                                reads.push((res(l, rv), rm, rt, rn));
                            }
                            outs.push((r, Blob(buf), reads));
                        }
                        // and build both tables from the same distribution
                        let mut ct = vec![0u32; fse_ctable_size_u32(table_log, msv)];
                        let mut cw = wksp(1 << 16);
                        let bc = unsafe {
                            l.sym::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp")(
                                ct.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                table_log,
                                wksp_ptr(&mut cw),
                                wksp_bytes(&cw),
                            )
                        };
                        let mut dt = vec![0u32; fse_dtable_size_u32(table_log)];
                        let mut dw = wksp(1 << 16);
                        let bd = unsafe {
                            l.sym::<FnFseBuildDTableWksp>("FSE_buildDTable_wksp")(
                                dt.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                table_log,
                                wksp_ptr(&mut dw),
                                wksp_bytes(&dw),
                            )
                        };
                        (
                            outs,
                            res(l, bc),
                            Blob(ct.iter().flat_map(|v| v.to_le_bytes()).collect()),
                            res(l, bd),
                            Blob(dt.iter().flat_map(|v| v.to_le_bytes()).collect()),
                        )
                    },
                );
            }
        }
    }
}

/// CONFIGS 296 — `HIST_count` / `HIST_count_wksp` must report
/// `maxSymbolValue_tooSmall` (48) when the source contains a byte greater than
/// `*maxSymbolValuePtr`, and must leave the out-param at the value the C leaves.
#[test]
fn hist_count_rejects_symbol_above_max() {
    covers(&["CFG:296"]);
    let mut rng = Rng::new(0x4831_0296);
    for _ in 0..200 {
        let n = 1 + rng.below(3000);
        let mut src = vec![0u8; n];
        rng.fill(&mut src);
        let max = rng.range(0, 255) as u32;
        // guarantee at least one byte above `max` for half the cases
        if rng.bool() && max < 255 {
            let i = rng.below(n);
            src[i] = (max + 1 + rng.below((255 - max) as usize) as u32) as u8;
        } else {
            for b in src.iter_mut() {
                *b %= (max + 1) as u8;
            }
        }
        diff(&format!("HIST_count max={max} n={n}"), |l| {
            let mut c1 = vec![0u32; 256];
            let mut m1 = max;
            let r1 = unsafe {
                l.sym::<FnHistCount>("HIST_count")(
                    c1.as_mut_ptr(),
                    &mut m1,
                    src.as_ptr() as *const c_void,
                    src.len(),
                )
            };
            let mut c2 = vec![0u32; 256];
            let mut m2 = max;
            let mut w = wksp(HIST_WKSP_SIZE);
            let r2 = unsafe {
                l.sym::<FnHistCountWksp>("HIST_count_wksp")(
                    c2.as_mut_ptr(),
                    &mut m2,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    wksp_ptr(&mut w),
                    wksp_bytes(&w),
                )
            };
            (
                res(l, r1),
                m1,
                Blob(c1.iter().flat_map(|v| v.to_le_bytes()).collect()),
                res(l, r2),
                m2,
                Blob(c2.iter().flat_map(|v| v.to_le_bytes()).collect()),
            )
        });
    }
}

/// CONFIGS 297 — `HIST_countFast_wksp` / `HIST_count_wksp` with the workspace
/// MISALIGNED by 1..7 bytes and with the workspace exactly `HIST_WKSP_SIZE`.
/// The C aligns internally and shrinks the usable size, so the alignment slack
/// changes whether the "too small" branch fires.
#[test]
fn hist_workspace_alignment_and_exact_size() {
    covers(&["CFG:297"]);
    let src = corpus(Corpus::Text, 4096, 0x0297);
    for off in 0..8usize {
        for size in [
            HIST_WKSP_SIZE,
            HIST_WKSP_SIZE + 8,
            HIST_WKSP_SIZE - 1,
            HIST_WKSP_SIZE / 2,
            0,
        ] {
            diff(&format!("HIST wksp off={off} size={size}"), |l| {
                let mut backing = vec![0u8; HIST_WKSP_SIZE + 64];
                let p = unsafe { backing.as_mut_ptr().add(off) } as *mut c_void;
                let mut c1 = vec![0u32; 256];
                let mut m1 = 255u32;
                let r1 = unsafe {
                    l.sym::<FnHistCountWksp>("HIST_count_wksp")(
                        c1.as_mut_ptr(),
                        &mut m1,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        p,
                        size,
                    )
                };
                let mut c2 = vec![0u32; 256];
                let mut m2 = 255u32;
                let r2 = unsafe {
                    l.sym::<FnHistCountWksp>("HIST_countFast_wksp")(
                        c2.as_mut_ptr(),
                        &mut m2,
                        src.as_ptr() as *const c_void,
                        src.len(),
                        p,
                        size,
                    )
                };
                (
                    res(l, r1),
                    m1,
                    Blob(c1.iter().flat_map(|v| v.to_le_bytes()).collect()),
                    res(l, r2),
                    m2,
                    Blob(c2.iter().flat_map(|v| v.to_le_bytes()).collect()),
                )
            });
        }
    }
}

/// CONFIGS 307/308/379-387 — `divsufsort` / `divbwt` argument validation and the
/// exhaustively enumerable small-`n` cases.
///
/// Both functions DO validate their arguments:
/// `if ((T == NULL) || (SA == NULL) || (n < 0)) return -1;` (divsufsort.c:1853)
/// and `if ((T == NULL) || (U == NULL) || (n < 0)) return -1;` (divsufsort.c:1882),
/// so NULL and negative `n` are in contract here — unlike most of this library.
#[test]
fn divsufsort_argument_validation_and_small_n() {
    covers(&["CFG:307", "CFG:308", "CFG:379", "CFG:380", "CFG:384"]);
    let data = corpus(Corpus::Text, 64, 0x0307);
    // NULL / negative-n rejections
    for n in [-1i32, -1000, i32::MIN, 0, 1, 10] {
        diff(&format!("divsufsort NULL T n={n}"), |l| {
            let mut sa = vec![-1i32; 32];
            unsafe {
                l.sym::<FnDivsufsort>("divsufsort")(std::ptr::null(), sa.as_mut_ptr(), n, 0)
            }
        });
        diff(&format!("divsufsort NULL SA n={n}"), |l| unsafe {
            l.sym::<FnDivsufsort>("divsufsort")(data.as_ptr(), std::ptr::null_mut(), n, 0)
        });
        diff(&format!("divbwt NULL T n={n}"), |l| {
            let mut u = vec![0u8; 32];
            let mut a = vec![0i32; 40];
            unsafe {
                l.sym::<FnDivbwt>("divbwt")(
                    std::ptr::null_mut(),
                    u.as_mut_ptr(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    0,
                )
            }
        });
        diff(&format!("divbwt NULL U n={n}"), |l| {
            let mut t = data.clone();
            let mut a = vec![0i32; 40];
            unsafe {
                l.sym::<FnDivbwt>("divbwt")(
                    t.as_mut_ptr(),
                    std::ptr::null_mut(),
                    a.as_mut_ptr(),
                    n,
                    std::ptr::null_mut(),
                    0,
                )
            }
        });
    }
    // exhaustive binary strings for n = 1..=10 (the n<=2 shortcuts plus the
    // general sorter's smallest real inputs)
    for n in 1..=10usize {
        for bits in 0u32..(1u32 << n) {
            let t: Vec<u8> = (0..n)
                .map(|i| if bits >> i & 1 == 1 { 0xFFu8 } else { 0x00 })
                .collect();
            diff(&format!("divsufsort exhaustive n={n} bits={bits}"), |l| {
                let mut sa = vec![-1i32; n + 8];
                let rc = unsafe {
                    l.sym::<FnDivsufsort>("divsufsort")(t.as_ptr(), sa.as_mut_ptr(), n as i32, 0)
                };
                (rc, sa)
            });
            if n <= 8 {
                diff(&format!("divbwt exhaustive n={n} bits={bits}"), |l| {
                    let mut tt = t.clone();
                    tt.resize(n + 8, 0);
                    let mut u = vec![0u8; n + 8];
                    let mut a = vec![0i32; n + 9];
                    let rc = unsafe {
                        l.sym::<FnDivbwt>("divbwt")(
                            tt.as_mut_ptr(),
                            u.as_mut_ptr(),
                            a.as_mut_ptr(),
                            n as i32,
                            std::ptr::null_mut(),
                            0,
                        )
                    };
                    (rc, tt, u, a)
                });
            }
        }
    }
}

/// CONFIGS 381/382/383/385/386/387 — `divsufsort` / `divbwt` on the degenerate
/// shapes (all-equal input, sizes straddling the bucket-sort thresholds), with
/// `openMP` ignored (`LIBBSC_OPENMP` is undefined in this build so 0/1/12345 must
/// all behave identically), with `A == NULL` (internal malloc) and with `U == T`
/// (`divbwt` documents that the buffers may be the same).
#[test]
fn divsufsort_degenerate_shapes_and_aliasing() {
    covers(&["CFG:381", "CFG:382", "CFG:383", "CFG:385", "CFG:386", "CFG:387"]);
    for n in [1000usize, 1024, 1025, 2048, 4096] {
        for shape in [0u8, 1, 2] {
            let t: Vec<u8> = match shape {
                0 => vec![0x41u8; n],                          // all equal
                1 => corpus(Corpus::Text, n, 0x381),            // text
                _ => corpus(Corpus::Counter, n, 0x381),         // strictly cyclic
            };
            for open_mp in [0i32, 1, 12345] {
                diff(
                    &format!("divsufsort n={n} shape={shape} openMP={open_mp}"),
                    |l| {
                        let mut sa = vec![-1i32; n + 8];
                        let rc = unsafe {
                            l.sym::<FnDivsufsort>("divsufsort")(
                                t.as_ptr(),
                                sa.as_mut_ptr(),
                                n as i32,
                                open_mp,
                            )
                        };
                        (rc, Blob(sa.iter().flat_map(|v| v.to_le_bytes()).collect()))
                    },
                );
            }
            // divbwt with a caller-provided A, with A == NULL, and with U == T
            diff(&format!("divbwt A given n={n} shape={shape}"), |l| {
                let mut tt = t.clone();
                tt.resize(n + 8, 0);
                let mut u = vec![0u8; n + 8];
                let mut a = vec![0i32; n + 9];
                let rc = unsafe {
                    l.sym::<FnDivbwt>("divbwt")(
                        tt.as_mut_ptr(),
                        u.as_mut_ptr(),
                        a.as_mut_ptr(),
                        n as i32,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                (rc, Blob(u), Blob(a.iter().flat_map(|v| v.to_le_bytes()).collect()))
            });
            diff(&format!("divbwt A=NULL n={n} shape={shape}"), |l| {
                let mut tt = t.clone();
                tt.resize(n + 8, 0);
                let mut u = vec![0u8; n + 8];
                let rc = unsafe {
                    l.sym::<FnDivbwt>("divbwt")(
                        tt.as_mut_ptr(),
                        u.as_mut_ptr(),
                        std::ptr::null_mut(),
                        n as i32,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                (rc, Blob(u))
            });
            diff(&format!("divbwt U==T n={n} shape={shape}"), |l| {
                let mut tt = t.clone();
                tt.resize(n + 8, 0);
                let mut a = vec![0i32; n + 9];
                let p = tt.as_mut_ptr();
                let rc = unsafe {
                    l.sym::<FnDivbwt>("divbwt")(
                        p,
                        p,
                        a.as_mut_ptr(),
                        n as i32,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                (rc, Blob(tt))
            });
        }
    }
}
