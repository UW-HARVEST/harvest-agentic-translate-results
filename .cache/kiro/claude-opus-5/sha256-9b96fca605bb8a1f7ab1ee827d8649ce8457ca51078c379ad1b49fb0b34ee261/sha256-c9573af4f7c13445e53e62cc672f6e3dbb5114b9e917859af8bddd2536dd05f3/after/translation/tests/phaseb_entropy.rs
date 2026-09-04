//! Differential tests for the low-level ENTROPY API of zstd:
//! FSE_*, HUF_*, HIST_*, and the (namespaced) ZSTD_XXH* hash functions.
//!
//! Every symbol is fetched from BOTH the C `libzstd.so` and the Rust
//! `libzstd.so` via `fnpair!` and exercised only through FFI. For every call
//! we compare the return value exactly, and — when the call succeeds — the full
//! output buffer bytes (including untouched tail, initialised to 0xAA). For the
//! table-building functions we compare the entire produced CTable / DTable /
//! normalizedCounter arrays byte-for-byte. On error we require both sides to
//! agree that it is an error (via FSE_isError / HUF_isError) and produce the
//! same error-name string.
//!
//! NOTE ON EXPORTED SURFACE: this build only exports the `_wksp` / `_usingCTable`
//! / `_repeat` family plus the tool functions (see
//! `nm -D --defined-only c_src/build/libzstd.so | grep -E 'FSE_|HUF_|HIST_|XXH'`).
//! Symbols like `FSE_compress`, `FSE_decompress`, `HUF_compress`,
//! `HUF_decompress1X1` etc. are static/inlined in this single-translation-unit
//! build and are therefore NOT exported; the tests below drive the exported
//! primitives directly (which collectively cover the same code paths).

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

const FILL: u8 = 0xAA;

// ---- FSE constants (from fse.h) ----
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const FSE_MAX_TABLELOG: u32 = 12; // FSE_MAX_MEMORY_USAGE(14) - 2
const FSE_MIN_TABLELOG: u32 = 5;

// FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue) = 1 + (1<<(maxTableLog-1)) + (maxSymbolValue+1)*2
fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    1 + (1usize << (max_table_log - 1)) + ((max_symbol_value as usize + 1) * 2)
}
// FSE_DTABLE_SIZE_U32(maxTableLog) = 1 + (1<<maxTableLog)
fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
// FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32
fn fse_build_ctable_wksp_u32(max_symbol_value: u32, table_log: u32) -> usize {
    (((max_symbol_value as usize + 2) + (1usize << table_log)) / 2) + (8 / 4)
}
// FSE_BUILD_DTABLE_WKSP_SIZE (bytes)
fn fse_build_dtable_wksp_bytes(max_table_log: u32, max_symbol_value: u32) -> usize {
    2 * (max_symbol_value as usize + 1) + (1usize << max_table_log) + 8
}
// FSE_DECOMPRESS_WKSP_SIZE_U32
fn fse_decompress_wksp_u32(max_log: u32, max_symbol_value: u32) -> usize {
    let dtable = fse_dtable_size_u32(max_log);
    let build_dtable_u32 = fse_build_dtable_wksp_bytes(max_log, max_symbol_value).div_ceil(4);
    dtable + 1 + build_dtable_u32 + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2 + 1
}

// ---- HUF constants (from huf.h) ----
const HUF_TABLELOG_MAX: u32 = 12;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192) * 4;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
// HUF_CTABLE_SIZE_ST(maxSymbolValue) = maxSymbolValue + 2  (units of size_t)
fn huf_ctable_size_st(max_symbol_value: u32) -> usize {
    max_symbol_value as usize + 2
}
// HUF_DTABLE_SIZE(maxTableLog) = 1 + (1<<maxTableLog)  (units of U32)
fn huf_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
// HUF_READ_STATS_WORKSPACE_SIZE_U32 = FSE_DECOMPRESS_WKSP_SIZE_U32(6, HUF_TABLELOG_MAX-1)
fn huf_read_stats_wksp_u32() -> usize {
    fse_decompress_wksp_u32(6, HUF_TABLELOG_MAX - 1)
}

// -------------------------------------------------------------- fn types ----

type FnFseCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnFseOptimalTableLog = unsafe extern "C" fn(c_uint, size_t, c_uint) -> c_uint;
type FnFseOptimalTableLogInternal =
    unsafe extern "C" fn(c_uint, size_t, c_uint, c_uint) -> c_uint;
type FnFseNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, size_t, c_uint, c_uint) -> size_t;
type FnFseNCountWriteBound = unsafe extern "C" fn(c_uint, c_uint) -> size_t;
type FnFseWriteNCount =
    unsafe extern "C" fn(*mut c_void, size_t, *const i16, c_uint, c_uint) -> size_t;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnFseReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t, c_int) -> size_t;
type FnFseBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnFseBuildCTableRle = unsafe extern "C" fn(*mut c_uint, u8) -> size_t;
type FnFseBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnFseCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint) -> size_t;
type FnFseDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint,
    *mut c_void,
    size_t,
    c_int,
) -> size_t;
type FnVersion = unsafe extern "C" fn() -> c_uint;

// HIST
type FnHistCount =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnHistCountWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    size_t,
    *mut c_void,
    size_t,
) -> size_t;
type FnHistCountSimple =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> c_uint;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, size_t);

// HUF
type FnHufCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnHufOptimalTableLog = unsafe extern "C" fn(
    c_uint,
    size_t,
    c_uint,
    *mut c_void,
    size_t,
    *mut c_void,
    *const c_uint,
    c_int,
) -> c_uint;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut c_void, *const c_uint, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufWriteCTableWksp = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    c_uint,
    c_uint,
    *mut c_void,
    size_t,
) -> size_t;
type FnHufEstimateCompressedSize =
    unsafe extern "C" fn(*const c_void, *const c_uint, c_uint) -> size_t;
type FnHufValidateCTable = unsafe extern "C" fn(*const c_void, *const c_uint, c_uint) -> c_int;
type FnHufGetNbBits = unsafe extern "C" fn(*const c_void, c_uint) -> c_uint;
type FnHufCardinality = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
type FnHufMinTableLog = unsafe extern "C" fn(c_uint) -> c_uint;
type FnHufCompressUsingCTable = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    c_int,
) -> size_t;
type FnHufCompressRepeat = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint,
    c_uint,
    *mut c_void,
    size_t,
    *mut c_void,
    *mut c_int,
    c_int,
) -> size_t;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    size_t,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    size_t,
) -> size_t;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8,
    size_t,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    size_t,
    *mut c_void,
    size_t,
    c_int,
) -> size_t;
type FnHufReadCTable = unsafe extern "C" fn(
    *mut c_void,
    *mut c_uint,
    *const c_void,
    size_t,
    *mut c_uint,
) -> size_t;
type FnHufSelectDecoder = unsafe extern "C" fn(size_t, size_t) -> c_uint;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, *mut c_void, size_t, c_int) -> size_t;
type FnHufDecompressUsingDTable = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    c_int,
) -> size_t;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *mut c_void,
    size_t,
    c_int,
) -> size_t;

// xxhash
type FnXXH32 = unsafe extern "C" fn(*const c_void, size_t, u32) -> u32;
type FnXXH64 = unsafe extern "C" fn(*const c_void, size_t, u64) -> u64;
type FnXXHCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnXXHFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnXXH32Reset = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnXXH64Reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnXXHUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> c_int;
type FnXXH32Digest = unsafe extern "C" fn(*const c_void) -> u32;
type FnXXH64Digest = unsafe extern "C" fn(*const c_void) -> u64;
type FnXXHCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnXXH32CanonicalFromHash = unsafe extern "C" fn(*mut c_void, u32);
type FnXXH64CanonicalFromHash = unsafe extern "C" fn(*mut c_void, u64);
type FnXXH32HashFromCanonical = unsafe extern "C" fn(*const c_void) -> u32;
type FnXXH64HashFromCanonical = unsafe extern "C" fn(*const c_void) -> u64;

// ------------------------------------------------------------ helpers ------

fn length_classes() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100, 127, 128, 129, 200, 255, 256,
        257, 300, 511, 512, 513, 1000, 1024, 4096, 65535, 65536, 65537, 131072,
    ]
}

/// Compare two size_t return values that may be error codes, using the given
/// isError/getErrorName pairs to check error-status + error-name agreement.
/// Returns true when both succeeded (caller may then compare output buffers).
#[allow(clippy::too_many_arguments)]
fn cmp_ret_status(
    ctx: &str,
    cr: size_t,
    rr: size_t,
    c_iserr: &FnIsError,
    r_iserr: &FnIsError,
    c_ername: &FnErrName,
    r_ername: &FnErrName,
) -> bool {
    unsafe {
        let ce = (c_iserr)(cr) != 0;
        let re = (r_iserr)(rr) != 0;
        assert_eq!(
            ce, re,
            "{ctx}: error-status mismatch C_ret={cr} (err={ce}) Rust_ret={rr} (err={re})"
        );
        if ce {
            let cn = cstr((c_ername)(cr));
            let rn = cstr((r_ername)(rr));
            assert_eq!(cn, rn, "{ctx}: error-name mismatch C='{cn}' Rust='{rn}'");
            return false;
        }
    }
    assert_eq!(cr, rr, "{ctx}: return value mismatch C={cr} Rust={rr}");
    true
}

// =========================================================================
//                              FSE TESTS
// =========================================================================

#[test]
fn fse_tools_and_bounds() {
    let (c_bound, r_bound) = fnpair!("FSE_compressBound", FnFseCompressBound);
    let (c_ncwb, r_ncwb) = fnpair!("FSE_NCountWriteBound", FnFseNCountWriteBound);
    let (c_ver, r_ver) = fnpair!("FSE_versionNumber", FnVersion);
    let (c_otl, r_otl) = fnpair!("FSE_optimalTableLog", FnFseOptimalTableLog);
    let (c_otli, r_otli) = fnpair!("FSE_optimalTableLog_internal", FnFseOptimalTableLogInternal);

    unsafe {
        assert_eq!((c_ver)(), (r_ver)(), "FSE_versionNumber");
        for s in [0usize, 1, 2, 3, 15, 16, 64, 128, 255, 256, 512, 1024, 65536, 131072, 1 << 20] {
            assert_eq!((c_bound)(s), (r_bound)(s), "FSE_compressBound(len={s})");
        }
        for &msv in &[1u32, 2, 3, 15, 63, 127, 255] {
            for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
                assert_eq!(
                    (c_ncwb)(msv, tl),
                    (r_ncwb)(msv, tl),
                    "FSE_NCountWriteBound(msv={msv},tl={tl})"
                );
            }
        }
        let mut rng = Rng::new(0xF5E00001);
        for _ in 0..5000 {
            let max_tl = rng.range(0, 15) as u32;
            let src = (rng.next_u64() % (1 << 20)) as usize;
            let msv = rng.range(0, 255) as u32;
            assert_eq!(
                (c_otl)(max_tl, src, msv),
                (r_otl)(max_tl, src, msv),
                "FSE_optimalTableLog(max_tl={max_tl},src={src},msv={msv})"
            );
            let minus = rng.range(0, 3) as u32;
            assert_eq!(
                (c_otli)(max_tl, src, msv, minus),
                (r_otli)(max_tl, src, msv, minus),
                "FSE_optimalTableLog_internal(max_tl={max_tl},src={src},msv={msv},minus={minus})"
            );
        }
    }
}

/// Full FSE pipeline: HIST_count -> normalizeCount -> writeNCount -> buildCTable
/// -> compress_usingCTable -> readNCount -> buildDTable -> decompress_wksp_bmi2.
#[test]
fn fse_pipeline_and_tables() {
    let (c_iserr, r_iserr) = fnpair!("FSE_isError", FnIsError);
    let (c_ername, r_ername) = fnpair!("FSE_getErrorName", FnErrName);
    let (c_bound, r_bound) = fnpair!("FSE_compressBound", FnFseCompressBound);
    let (c_hist, r_hist) = fnpair!("HIST_count", FnHistCount);
    let (c_norm, r_norm) = fnpair!("FSE_normalizeCount", FnFseNormalizeCount);
    let (c_wnc, r_wnc) = fnpair!("FSE_writeNCount", FnFseWriteNCount);
    let (c_rnc, r_rnc) = fnpair!("FSE_readNCount", FnFseReadNCount);
    let (c_rnc2, r_rnc2) = fnpair!("FSE_readNCount_bmi2", FnFseReadNCountBmi2);
    let (c_bct, r_bct) = fnpair!("FSE_buildCTable_wksp", FnFseBuildCTableWksp);
    let (c_bdt, r_bdt) = fnpair!("FSE_buildDTable_wksp", FnFseBuildDTableWksp);
    let (c_cuc, r_cuc) = fnpair!("FSE_compress_usingCTable", FnFseCompressUsingCTable);
    let (c_dwb, r_dwb) = fnpair!("FSE_decompress_wksp_bmi2", FnFseDecompressWkspBmi2);
    let (c_otl, r_otl) = fnpair!("FSE_optimalTableLog", FnFseOptimalTableLog);
    let (c_ncwb, _r_ncwb) = fnpair!("FSE_NCountWriteBound", FnFseNCountWriteBound);

    let mut rng = Rng::new(0xF5E12345);
    let lens = length_classes();

    for &shape in ALL_SHAPES.iter() {
        for &len in &lens {
            if len > 65536 && !(shape == Shape::Text || shape == Shape::Random) {
                continue;
            }
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("FSE[shape={shape:?},len={len}]");

            unsafe {
                let mut c_count = vec![0u32; 256];
                let mut r_count = vec![0u32; 256];
                let mut c_msv: c_uint = 255;
                let mut r_msv: c_uint = 255;
                let ch = (c_hist)(c_count.as_mut_ptr(), &mut c_msv, src.as_ptr() as *const c_void, len);
                let rh = (r_hist)(r_count.as_mut_ptr(), &mut r_msv, src.as_ptr() as *const c_void, len);
                let ctx = format!("{ctx0} HIST_count");
                if cmp_ret_status(&ctx, ch, rh, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                    assert_eq!(c_msv, r_msv, "{ctx}: maxSymbolValue");
                    assert_bytes_eq(&ctx, bytes_u32(&c_count), bytes_u32(&r_count));
                }
                if len < 2 {
                    continue;
                }
                let max_symbol = c_msv;
                if max_symbol == 0 {
                    continue; // single-symbol -> RLE path
                }

                let tl_c = (c_otl)(0, len, max_symbol);
                let tl_r = (r_otl)(0, len, max_symbol);
                assert_eq!(tl_c, tl_r, "{ctx0}: optimalTableLog");
                let table_log = tl_c;

                for &use_low in &[0u32, 1u32] {
                    let mut c_norm_arr = vec![0i16; 256];
                    let mut r_norm_arr = vec![0i16; 256];
                    let cn = (c_norm)(c_norm_arr.as_mut_ptr(), table_log, c_count.as_ptr(), len, max_symbol, use_low);
                    let rn = (r_norm)(r_norm_arr.as_mut_ptr(), table_log, r_count.as_ptr(), len, max_symbol, use_low);
                    let ctx = format!("{ctx0} FSE_normalizeCount(tl={table_log},low={use_low})");
                    if !cmp_ret_status(&ctx, cn, rn, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        continue;
                    }
                    assert_bytes_eq(
                        &format!("{ctx} normalizedCounter"),
                        bytes_i16(&c_norm_arr),
                        bytes_i16(&r_norm_arr),
                    );
                    let actual_tl = cn as u32;

                    let nc_bound = (c_ncwb)(max_symbol, actual_tl).max(512);
                    let mut c_ncbuf = vec![FILL; nc_bound + 16];
                    let mut r_ncbuf = vec![FILL; nc_bound + 16];
                    let cw = (c_wnc)(c_ncbuf.as_mut_ptr() as *mut c_void, c_ncbuf.len(), c_norm_arr.as_ptr(), max_symbol, actual_tl);
                    let rw = (r_wnc)(r_ncbuf.as_mut_ptr() as *mut c_void, r_ncbuf.len(), r_norm_arr.as_ptr(), max_symbol, actual_tl);
                    let ctx = format!("{ctx0} FSE_writeNCount(tl={actual_tl},low={use_low})");
                    if !cmp_ret_status(&ctx, cw, rw, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        continue;
                    }
                    assert_bytes_eq(&format!("{ctx} full buffer"), &c_ncbuf, &r_ncbuf);
                    let nc_size = cw;

                    // readNCount_bmi2 (both flags) + readNCount
                    for bmi2 in [0i32, 1i32] {
                        let mut c_rd = vec![0i16; 256];
                        let mut r_rd = vec![0i16; 256];
                        let mut c_msv2: c_uint = 255;
                        let mut r_msv2: c_uint = 255;
                        let mut c_tl2: c_uint = 0;
                        let mut r_tl2: c_uint = 0;
                        let cr = (c_rnc2)(c_rd.as_mut_ptr(), &mut c_msv2, &mut c_tl2, c_ncbuf.as_ptr() as *const c_void, nc_size, bmi2);
                        let rr = (r_rnc2)(r_rd.as_mut_ptr(), &mut r_msv2, &mut r_tl2, r_ncbuf.as_ptr() as *const c_void, nc_size, bmi2);
                        let ctx = format!("{ctx0} FSE_readNCount_bmi2(bmi2={bmi2})");
                        if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                            assert_eq!(c_msv2, r_msv2, "{ctx}: msv");
                            assert_eq!(c_tl2, r_tl2, "{ctx}: tableLog");
                            assert_bytes_eq(&format!("{ctx} counts"), bytes_i16(&c_rd), bytes_i16(&r_rd));
                        }
                    }
                    {
                        let mut c_rd = vec![0i16; 256];
                        let mut r_rd = vec![0i16; 256];
                        let mut c_msv2: c_uint = 255;
                        let mut r_msv2: c_uint = 255;
                        let mut c_tl2: c_uint = 0;
                        let mut r_tl2: c_uint = 0;
                        let cr = (c_rnc)(c_rd.as_mut_ptr(), &mut c_msv2, &mut c_tl2, c_ncbuf.as_ptr() as *const c_void, nc_size);
                        let rr = (r_rnc)(r_rd.as_mut_ptr(), &mut r_msv2, &mut r_tl2, r_ncbuf.as_ptr() as *const c_void, nc_size);
                        let ctx = format!("{ctx0} FSE_readNCount");
                        if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                            assert_eq!(c_msv2, r_msv2, "{ctx}: msv");
                            assert_eq!(c_tl2, r_tl2, "{ctx}: tableLog");
                            assert_bytes_eq(&format!("{ctx} counts"), bytes_i16(&c_rd), bytes_i16(&r_rd));
                        }
                    }

                    if use_low != 1 {
                        continue; // run full compress/decompress once per (shape,len)
                    }

                    // buildCTable_wksp — allocate at the maximum possible size
                    // (tableLog=FSE_MAX_TABLELOG, maxSymbol=255) plus generous slack so a
                    // benign over-write on either side cannot corrupt the heap; we then
                    // compare the full identically-sized buffers byte-for-byte.
                    let ct_u32 = fse_ctable_size_u32(FSE_MAX_TABLELOG, FSE_MAX_SYMBOL_VALUE) + 64;
                    let wksp_u32 =
                        fse_build_ctable_wksp_u32(FSE_MAX_SYMBOL_VALUE, FSE_MAX_TABLELOG) + 64;
                    let mut c_ct = vec![0xAAAA_AAAAu32; ct_u32];
                    let mut r_ct = vec![0xAAAA_AAAAu32; ct_u32];
                    let mut c_ws = vec![0u32; wksp_u32];
                    let mut r_ws = vec![0u32; wksp_u32];
                    let cb = (c_bct)(c_ct.as_mut_ptr(), c_norm_arr.as_ptr(), max_symbol, actual_tl, c_ws.as_mut_ptr() as *mut c_void, (wksp_u32 * 4) as size_t);
                    let rb = (r_bct)(r_ct.as_mut_ptr(), r_norm_arr.as_ptr(), max_symbol, actual_tl, r_ws.as_mut_ptr() as *mut c_void, (wksp_u32 * 4) as size_t);
                    let ctx = format!("{ctx0} FSE_buildCTable_wksp(tl={actual_tl})");
                    if !cmp_ret_status(&ctx, cb, rb, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        continue;
                    }
                    assert_bytes_eq(&format!("{ctx} CTable"), bytes_u32(&c_ct), bytes_u32(&r_ct));

                    // compress_usingCTable
                    let cap = (c_bound)(len);
                    assert_eq!(cap, (r_bound)(len), "{ctx0}: compressBound");
                    let mut c_dst = vec![FILL; cap + 16];
                    let mut r_dst = vec![FILL; cap + 16];
                    let cc = (c_cuc)(c_dst.as_mut_ptr() as *mut c_void, c_dst.len(), src.as_ptr() as *const c_void, len, c_ct.as_ptr());
                    let rc = (r_cuc)(r_dst.as_mut_ptr() as *mut c_void, r_dst.len(), src.as_ptr() as *const c_void, len, r_ct.as_ptr());
                    let ctx = format!("{ctx0} FSE_compress_usingCTable");
                    let ce = (c_iserr)(cc) != 0;
                    let re = (r_iserr)(rc) != 0;
                    assert_eq!(ce, re, "{ctx}: err-status C={cc} R={rc}");
                    if ce {
                        assert_eq!(cstr((c_ername)(cc)), cstr((r_ername)(rc)), "{ctx}: err-name");
                        continue;
                    }
                    assert_eq!(cc, rc, "{ctx}: compressed size");
                    assert_bytes_eq(&format!("{ctx} full dst"), &c_dst, &r_dst);
                    let comp_size = cc;
                    if comp_size == 0 {
                        continue; // incompressible (0 == not compressible)
                    }

                    // buildDTable_wksp from re-read NCount + decompress_wksp_bmi2.
                    // FSE_decompress_wksp() consumes a full FSE frame: the NCount
                    // header followed by the payload from compress_usingCTable. Build
                    // that frame so the roundtrip actually reconstructs the source.
                    let mut c_frame = Vec::with_capacity(nc_size + comp_size);
                    c_frame.extend_from_slice(&c_ncbuf[..nc_size]);
                    c_frame.extend_from_slice(&c_dst[..comp_size]);
                    let mut r_frame = Vec::with_capacity(nc_size + comp_size);
                    r_frame.extend_from_slice(&r_ncbuf[..nc_size]);
                    r_frame.extend_from_slice(&r_dst[..comp_size]);
                    assert_bytes_eq(&format!("{ctx0} FSE frame"), &c_frame, &r_frame);

                    let mut c_rd = vec![0i16; 256];
                    let mut c_msv2: c_uint = 255;
                    let mut c_tl2: c_uint = 0;
                    let _ = (c_rnc)(c_rd.as_mut_ptr(), &mut c_msv2, &mut c_tl2, c_ncbuf.as_ptr() as *const c_void, nc_size);
                    let dt_u32 = fse_dtable_size_u32(FSE_MAX_TABLELOG) + 64;
                    let dwksp =
                        fse_build_dtable_wksp_bytes(FSE_MAX_TABLELOG, FSE_MAX_SYMBOL_VALUE) + 256;
                    let mut c_dt = vec![0xAAAA_AAAAu32; dt_u32];
                    let mut r_dt = vec![0xAAAA_AAAAu32; dt_u32];
                    let mut c_dws = vec![0u8; dwksp];
                    let mut r_dws = vec![0u8; dwksp];
                    let cd = (c_bdt)(c_dt.as_mut_ptr(), c_rd.as_ptr(), c_msv2, c_tl2, c_dws.as_mut_ptr() as *mut c_void, c_dws.len());
                    let rd = (r_bdt)(r_dt.as_mut_ptr(), c_rd.as_ptr(), c_msv2, c_tl2, r_dws.as_mut_ptr() as *mut c_void, r_dws.len());
                    let ctx = format!("{ctx0} FSE_buildDTable_wksp");
                    if cmp_ret_status(&ctx, cd, rd, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_bytes_eq(&format!("{ctx} DTable"), bytes_u32(&c_dt), bytes_u32(&r_dt));
                    }

                    for bmi2 in [0i32, 1i32] {
                        let dwu = fse_decompress_wksp_u32(FSE_MAX_TABLELOG, FSE_MAX_SYMBOL_VALUE);
                        let mut c_out = vec![FILL; len + 16];
                        let mut r_out = vec![FILL; len + 16];
                        let mut c_dw = vec![0u32; dwu];
                        let mut r_dw = vec![0u32; dwu];
                        let cdec = (c_dwb)(c_out.as_mut_ptr() as *mut c_void, len, c_frame.as_ptr() as *const c_void, c_frame.len(), FSE_MAX_TABLELOG, c_dw.as_mut_ptr() as *mut c_void, (dwu * 4) as size_t, bmi2);
                        let rdec = (r_dwb)(r_out.as_mut_ptr() as *mut c_void, len, r_frame.as_ptr() as *const c_void, r_frame.len(), FSE_MAX_TABLELOG, r_dw.as_mut_ptr() as *mut c_void, (dwu * 4) as size_t, bmi2);
                        let ctx = format!("{ctx0} FSE_decompress_wksp_bmi2(bmi2={bmi2})");
                        if cmp_ret_status(&ctx, cdec, rdec, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                            assert_bytes_eq(&format!("{ctx} decoded"), &c_out, &r_out);
                            assert_eq!(&c_out[..len], &src[..], "{ctx}: roundtrip mismatch vs source");
                        }
                    }
                }
            }
        }
    }
}

/// FSE_buildCTable_rle across all symbol values.
#[test]
fn fse_build_ctable_rle() {
    let (c_rle, r_rle) = fnpair!("FSE_buildCTable_rle", FnFseBuildCTableRle);
    let (c_iserr, r_iserr) = fnpair!("FSE_isError", FnIsError);
    let (c_ername, r_ername) = fnpair!("FSE_getErrorName", FnErrName);
    unsafe {
        // FSE_buildCTable_rle writes a 2-u32 header followed by symbolTT[symbolValue]
        // where each symbolTT entry is 8 bytes; for symbolValue==255 that reaches
        // u32 index 2 + 255*2 + 1. Size the buffer to hold the full worst case.
        let rle_u32 = 2 + (255 * 2 + 2);
        for sym in 0u16..=255 {
            let sym = sym as u8;
            let mut c_ct = vec![0xAAAA_AAAAu32; rle_u32];
            let mut r_ct = vec![0xAAAA_AAAAu32; rle_u32];
            let cr = (c_rle)(c_ct.as_mut_ptr(), sym);
            let rr = (r_rle)(r_ct.as_mut_ptr(), sym);
            let ctx = format!("FSE_buildCTable_rle(sym={sym})");
            if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                assert_bytes_eq(&format!("{ctx} CTable"), bytes_u32(&c_ct), bytes_u32(&r_ct));
            }
        }
    }
}

// =========================================================================
//                              HIST TESTS
// =========================================================================

#[test]
fn hist_all_variants() {
    let (c_count, r_count) = fnpair!("HIST_count", FnHistCount);
    let (c_cw, r_cw) = fnpair!("HIST_count_wksp", FnHistCountWksp);
    let (c_cf, r_cf) = fnpair!("HIST_countFast", FnHistCount);
    let (c_cfw, r_cfw) = fnpair!("HIST_countFast_wksp", FnHistCountWksp);
    let (c_cs, r_cs) = fnpair!("HIST_count_simple", FnHistCountSimple);
    let (c_add, r_add) = fnpair!("HIST_add", FnHistAdd);
    let (c_iserr, r_iserr) = fnpair!("HIST_isError", FnIsError);
    let (c_ername, r_ername) = fnpair!("FSE_getErrorName", FnErrName); // HIST errors are FSE error codes

    let mut rng = Rng::new(0x415700FF);
    let lens = length_classes();
    let wksp_bytes = 1024 * 4; // HIST_WKSP_SIZE

    for &shape in ALL_SHAPES.iter() {
        for &len in &lens {
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("HIST[shape={shape:?},len={len}]");
            unsafe {
                // safe (full 255) + clamped msv to exercise the "value > max" branch
                for &start_msv in &[255u32, 15u32, 1u32] {
                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = start_msv;
                    let mut rm = start_msv;
                    let cr = (c_count)(cc.as_mut_ptr(), &mut cm, src.as_ptr() as *const c_void, len);
                    let rr = (r_count)(rc.as_mut_ptr(), &mut rm, src.as_ptr() as *const c_void, len);
                    let ctx = format!("{ctx0} HIST_count(msv0={start_msv})");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(cm, rm, "{ctx}: msv");
                        assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));
                    }

                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = start_msv;
                    let mut rm = start_msv;
                    let mut cws = vec![0u8; wksp_bytes];
                    let mut rws = vec![0u8; wksp_bytes];
                    let cr = (c_cw)(cc.as_mut_ptr(), &mut cm, src.as_ptr() as *const c_void, len, cws.as_mut_ptr() as *mut c_void, wksp_bytes);
                    let rr = (r_cw)(rc.as_mut_ptr(), &mut rm, src.as_ptr() as *const c_void, len, rws.as_mut_ptr() as *mut c_void, wksp_bytes);
                    let ctx = format!("{ctx0} HIST_count_wksp(msv0={start_msv})");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(cm, rm, "{ctx}: msv");
                        assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));
                    }
                }

                // Fast + simple variants require all bytes <= msv; use full 255.
                {
                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = 255u32;
                    let mut rm = 255u32;
                    let cr = (c_cf)(cc.as_mut_ptr(), &mut cm, src.as_ptr() as *const c_void, len);
                    let rr = (r_cf)(rc.as_mut_ptr(), &mut rm, src.as_ptr() as *const c_void, len);
                    let ctx = format!("{ctx0} HIST_countFast");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(cm, rm, "{ctx}: msv");
                        assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));
                    }

                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = 255u32;
                    let mut rm = 255u32;
                    let mut cws = vec![0u8; wksp_bytes];
                    let mut rws = vec![0u8; wksp_bytes];
                    let cr = (c_cfw)(cc.as_mut_ptr(), &mut cm, src.as_ptr() as *const c_void, len, cws.as_mut_ptr() as *mut c_void, wksp_bytes);
                    let rr = (r_cfw)(rc.as_mut_ptr(), &mut rm, src.as_ptr() as *const c_void, len, rws.as_mut_ptr() as *mut c_void, wksp_bytes);
                    let ctx = format!("{ctx0} HIST_countFast_wksp");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(cm, rm, "{ctx}: msv");
                        assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));
                    }

                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = 255u32;
                    let mut rm = 255u32;
                    let cr = (c_cs)(cc.as_mut_ptr(), &mut cm, src.as_ptr() as *const c_void, len);
                    let rr = (r_cs)(rc.as_mut_ptr(), &mut rm, src.as_ptr() as *const c_void, len);
                    let ctx = format!("{ctx0} HIST_count_simple");
                    assert_eq!(cr, rr, "{ctx}: return (max count)");
                    assert_eq!(cm, rm, "{ctx}: msv");
                    assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));

                    let mut cc = vec![7u32; 256];
                    let mut rc = vec![7u32; 256];
                    (c_add)(cc.as_mut_ptr(), src.as_ptr() as *const c_void, len);
                    (r_add)(rc.as_mut_ptr(), src.as_ptr() as *const c_void, len);
                    let ctx = format!("{ctx0} HIST_add");
                    assert_bytes_eq(&format!("{ctx} count"), bytes_u32(&cc), bytes_u32(&rc));
                }
            }
        }
    }
}

// =========================================================================
//                              HUF TESTS
// =========================================================================

#[test]
fn huf_tools() {
    let (c_bound, r_bound) = fnpair!("HUF_compressBound", FnHufCompressBound);
    let (c_min, r_min) = fnpair!("HUF_minTableLog", FnHufMinTableLog);
    let (c_card, r_card) = fnpair!("HUF_cardinality", FnHufCardinality);
    let (c_sel, r_sel) = fnpair!("HUF_selectDecoder", FnHufSelectDecoder);
    unsafe {
        for s in [0usize, 1, 2, 3, 128, 255, 256, 512, 1024, 65536, 131072, 1 << 20] {
            assert_eq!((c_bound)(s), (r_bound)(s), "HUF_compressBound(len={s})");
        }
        // NOTE: cardinality 0 is out of contract — HUF_minTableLog calls
        // ZSTD_highbit32 which asserts val != 0 (UB in release). Real callers
        // always pass cardinality >= 1, so we start at 1.
        for card in 1u32..=257 {
            assert_eq!((c_min)(card), (r_min)(card), "HUF_minTableLog(card={card})");
        }
        let mut rng = Rng::new(0x40F00001);
        for _ in 0..4000 {
            let dst = (rng.next_u64() % (256 * 1024)) as usize;
            let csrc = (rng.next_u64() % (256 * 1024)) as usize;
            assert_eq!((c_sel)(dst, csrc), (r_sel)(dst, csrc), "HUF_selectDecoder(dst={dst},cSrc={csrc})");
        }
        for _ in 0..2000 {
            let msv = rng.range(0, 255) as u32;
            let mut count = vec![0u32; 256];
            for c in count.iter_mut().take(msv as usize + 1) {
                *c = (rng.next_u64() % 1000) as u32;
            }
            assert_eq!(
                (c_card)(count.as_ptr(), msv),
                (r_card)(count.as_ptr(), msv),
                "HUF_cardinality(msv={msv})"
            );
        }
    }
}

/// Full HUF pipeline: build/write/read CTable, compress (1X/4X usingCTable &
/// repeat), readStats, read DTable (X1/X2), decompress (DCtx_wksp variants).
#[test]
fn huf_pipeline_and_tables() {
    let (c_iserr, r_iserr) = fnpair!("HUF_isError", FnIsError);
    let (c_ername, r_ername) = fnpair!("HUF_getErrorName", FnErrName);
    let (c_bound, _r_bound) = fnpair!("HUF_compressBound", FnHufCompressBound);
    let (c_hist, _r_hist) = fnpair!("HIST_count", FnHistCount);

    let (c_bct, r_bct) = fnpair!("HUF_buildCTable_wksp", FnHufBuildCTableWksp);
    let (c_wct, r_wct) = fnpair!("HUF_writeCTable_wksp", FnHufWriteCTableWksp);
    let (c_rct, r_rct) = fnpair!("HUF_readCTable", FnHufReadCTable);
    let (c_est, r_est) = fnpair!("HUF_estimateCompressedSize", FnHufEstimateCompressedSize);
    let (c_val, r_val) = fnpair!("HUF_validateCTable", FnHufValidateCTable);
    let (c_nbb, r_nbb) = fnpair!("HUF_getNbBitsFromCTable", FnHufGetNbBits);
    let (c_otl, r_otl) = fnpair!("HUF_optimalTableLog", FnHufOptimalTableLog);

    let (c_c1u, r_c1u) = fnpair!("HUF_compress1X_usingCTable", FnHufCompressUsingCTable);
    let (c_c4u, r_c4u) = fnpair!("HUF_compress4X_usingCTable", FnHufCompressUsingCTable);
    let (c_c1r, r_c1r) = fnpair!("HUF_compress1X_repeat", FnHufCompressRepeat);
    let (c_c4r, r_c4r) = fnpair!("HUF_compress4X_repeat", FnHufCompressRepeat);

    let (c_rs, r_rs) = fnpair!("HUF_readStats", FnHufReadStats);
    let (c_rsw, r_rsw) = fnpair!("HUF_readStats_wksp", FnHufReadStatsWksp);
    let (c_rd1, r_rd1) = fnpair!("HUF_readDTableX1_wksp", FnHufReadDTableWksp);
    let (c_rd2, r_rd2) = fnpair!("HUF_readDTableX2_wksp", FnHufReadDTableWksp);
    let (c_dc1, r_dc1) = fnpair!("HUF_decompress1X1_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_dc12, r_dc12) = fnpair!("HUF_decompress1X2_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_dc1x, r_dc1x) = fnpair!("HUF_decompress1X_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_d4h, r_d4h) = fnpair!("HUF_decompress4X_hufOnly_wksp", FnHufDecompressDCtxWksp);
    let (c_d1u, r_d1u) = fnpair!("HUF_decompress1X_usingDTable", FnHufDecompressUsingDTable);
    let (c_d4u, r_d4u) = fnpair!("HUF_decompress4X_usingDTable", FnHufDecompressUsingDTable);

    let mut rng = Rng::new(0x40F12345);
    let lens: Vec<usize> = vec![
        2, 3, 4, 8, 16, 32, 64, 100, 127, 128, 129, 200, 255, 256, 257, 300, 512, 1000, 1024,
        4096, 65535, 65536, 131072,
    ];
    let flags = 0i32; // DYNAMIC_BMI2=0 build: bmi2 flag ignored

    for &shape in ALL_SHAPES.iter() {
        for &len in &lens {
            let src = gen(shape, len, &mut rng);
            let ctx0 = format!("HUF[shape={shape:?},len={len}]");
            unsafe {
                let mut count = vec![0u32; 256];
                let mut msv: c_uint = 255;
                let _ = (c_hist)(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, len);
                if msv == 0 {
                    continue; // single symbol
                }

                // Always size CTable buffers to the maximum symbol range
                // (HUF_readCTable is handed maxSymbolValuePtr=255 and may populate up
                // to symbol 255) plus generous slack, so no benign over-write can
                // corrupt the heap. Full identically-sized buffers are compared.
                let ct_st = huf_ctable_size_st(HUF_SYMBOLVALUE_MAX) + 8;
                let mut c_scratch = vec![0usize; ct_st + 8];
                let mut r_scratch = vec![0usize; ct_st + 8];
                let mut wsc = vec![0u8; HUF_WORKSPACE_SIZE];
                let mut wsr = vec![0u8; HUF_WORKSPACE_SIZE];
                let c_tl = (c_otl)(HUF_TABLELOG_MAX, len, msv, wsc.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE, c_scratch.as_mut_ptr() as *mut c_void, count.as_ptr(), flags);
                let r_tl = (r_otl)(HUF_TABLELOG_MAX, len, msv, wsr.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE, r_scratch.as_mut_ptr() as *mut c_void, count.as_ptr(), flags);
                assert_eq!(c_tl, r_tl, "{ctx0}: HUF_optimalTableLog");
                let table_log = if c_tl == 0 { HUF_TABLELOG_MAX } else { c_tl };

                let mut c_ct = vec![0usize; ct_st];
                let mut r_ct = vec![0usize; ct_st];
                let mut cws = vec![0u8; HUF_CTABLE_WORKSPACE_SIZE];
                let mut rws = vec![0u8; HUF_CTABLE_WORKSPACE_SIZE];
                let cb = (c_bct)(c_ct.as_mut_ptr() as *mut c_void, count.as_ptr(), msv, table_log, cws.as_mut_ptr() as *mut c_void, HUF_CTABLE_WORKSPACE_SIZE);
                let rb = (r_bct)(r_ct.as_mut_ptr() as *mut c_void, count.as_ptr(), msv, table_log, rws.as_mut_ptr() as *mut c_void, HUF_CTABLE_WORKSPACE_SIZE);
                let ctx = format!("{ctx0} HUF_buildCTable_wksp(tl={table_log})");
                if !cmp_ret_status(&ctx, cb, rb, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                    continue;
                }
                assert_bytes_eq(&format!("{ctx} CTable"), bytes_usize(&c_ct), bytes_usize(&r_ct));
                let max_bits = cb as u32;

                for sym in 0u32..=255 {
                    assert_eq!(
                        (c_nbb)(c_ct.as_ptr() as *const c_void, sym),
                        (r_nbb)(r_ct.as_ptr() as *const c_void, sym),
                        "{ctx0} HUF_getNbBitsFromCTable(sym={sym})"
                    );
                }
                assert_eq!(
                    (c_est)(c_ct.as_ptr() as *const c_void, count.as_ptr(), msv),
                    (r_est)(r_ct.as_ptr() as *const c_void, count.as_ptr(), msv),
                    "{ctx0} HUF_estimateCompressedSize"
                );
                assert_eq!(
                    (c_val)(c_ct.as_ptr() as *const c_void, count.as_ptr(), msv),
                    (r_val)(r_ct.as_ptr() as *const c_void, count.as_ptr(), msv),
                    "{ctx0} HUF_validateCTable"
                );

                let mut c_hdr = vec![FILL; 512];
                let mut r_hdr = vec![FILL; 512];
                let mut cws2 = vec![0u8; HUF_WORKSPACE_SIZE];
                let mut rws2 = vec![0u8; HUF_WORKSPACE_SIZE];
                let cw = (c_wct)(c_hdr.as_mut_ptr() as *mut c_void, c_hdr.len(), c_ct.as_ptr() as *const c_void, msv, max_bits, cws2.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE);
                let rw = (r_wct)(r_hdr.as_mut_ptr() as *mut c_void, r_hdr.len(), r_ct.as_ptr() as *const c_void, msv, max_bits, rws2.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE);
                let ctx = format!("{ctx0} HUF_writeCTable_wksp");
                if !cmp_ret_status(&ctx, cw, rw, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                    continue;
                }
                assert_bytes_eq(&format!("{ctx} full hdr"), &c_hdr, &r_hdr);
                let hdr_size = cw;

                // readStats + readStats_wksp
                {
                    let mut c_hw = vec![FILL; HUF_SYMBOLVALUE_MAX as usize + 2];
                    let mut r_hw = vec![FILL; HUF_SYMBOLVALUE_MAX as usize + 2];
                    let mut c_rank = vec![0u32; 16];
                    let mut r_rank = vec![0u32; 16];
                    let mut c_ns = 0u32; let mut r_ns = 0u32;
                    let mut c_tl2 = 0u32; let mut r_tl2 = 0u32;
                    let cr = (c_rs)(c_hw.as_mut_ptr(), c_hw.len(), c_rank.as_mut_ptr(), &mut c_ns, &mut c_tl2, c_hdr.as_ptr() as *const c_void, hdr_size);
                    let rr = (r_rs)(r_hw.as_mut_ptr(), r_hw.len(), r_rank.as_mut_ptr(), &mut r_ns, &mut r_tl2, r_hdr.as_ptr() as *const c_void, hdr_size);
                    let ctx = format!("{ctx0} HUF_readStats");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(c_ns, r_ns, "{ctx}: nbSymbols");
                        assert_eq!(c_tl2, r_tl2, "{ctx}: tableLog");
                        assert_bytes_eq(&format!("{ctx} huffWeight"), &c_hw, &r_hw);
                        assert_bytes_eq(&format!("{ctx} rankStats"), bytes_u32(&c_rank), bytes_u32(&r_rank));
                    }

                    let rswu = huf_read_stats_wksp_u32() * 4;
                    let mut c_hw = vec![FILL; HUF_SYMBOLVALUE_MAX as usize + 2];
                    let mut r_hw = vec![FILL; HUF_SYMBOLVALUE_MAX as usize + 2];
                    let mut c_rank = vec![0u32; 16];
                    let mut r_rank = vec![0u32; 16];
                    let mut c_ns = 0u32; let mut r_ns = 0u32;
                    let mut c_tl2 = 0u32; let mut r_tl2 = 0u32;
                    let mut cwsp = vec![0u8; rswu];
                    let mut rwsp = vec![0u8; rswu];
                    let cr = (c_rsw)(c_hw.as_mut_ptr(), c_hw.len(), c_rank.as_mut_ptr(), &mut c_ns, &mut c_tl2, c_hdr.as_ptr() as *const c_void, hdr_size, cwsp.as_mut_ptr() as *mut c_void, rswu, flags);
                    let rr = (r_rsw)(r_hw.as_mut_ptr(), r_hw.len(), r_rank.as_mut_ptr(), &mut r_ns, &mut r_tl2, r_hdr.as_ptr() as *const c_void, hdr_size, rwsp.as_mut_ptr() as *mut c_void, rswu, flags);
                    let ctx = format!("{ctx0} HUF_readStats_wksp");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(c_ns, r_ns, "{ctx}: nbSymbols");
                        assert_eq!(c_tl2, r_tl2, "{ctx}: tableLog");
                        assert_bytes_eq(&format!("{ctx} huffWeight"), &c_hw, &r_hw);
                        assert_bytes_eq(&format!("{ctx} rankStats"), bytes_u32(&c_rank), bytes_u32(&r_rank));
                    }
                }

                // readCTable round-trip
                {
                    let mut c_ct2 = vec![0usize; ct_st];
                    let mut r_ct2 = vec![0usize; ct_st];
                    let mut c_msv2 = HUF_SYMBOLVALUE_MAX;
                    let mut r_msv2 = HUF_SYMBOLVALUE_MAX;
                    let mut c_zw = 0u32; let mut r_zw = 0u32;
                    let cr = (c_rct)(c_ct2.as_mut_ptr() as *mut c_void, &mut c_msv2, c_hdr.as_ptr() as *const c_void, hdr_size, &mut c_zw);
                    let rr = (r_rct)(r_ct2.as_mut_ptr() as *mut c_void, &mut r_msv2, r_hdr.as_ptr() as *const c_void, hdr_size, &mut r_zw);
                    let ctx = format!("{ctx0} HUF_readCTable");
                    if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_eq!(c_msv2, r_msv2, "{ctx}: msv");
                        assert_eq!(c_zw, r_zw, "{ctx}: hasZeroWeights");
                        assert_bytes_eq(&format!("{ctx} CTable"), bytes_usize(&c_ct2), bytes_usize(&r_ct2));
                    }
                }

                let cap = (c_bound)(len);

                // compress1X/4X usingCTable (compares raw encoded payload + DTable build)
                for (which, cf, rf) in [(1u8, &c_c1u, &r_c1u), (4u8, &c_c4u, &r_c4u)] {
                    let mut c_dst = vec![FILL; cap + 16];
                    let mut r_dst = vec![FILL; cap + 16];
                    let cc = (cf)(c_dst.as_mut_ptr() as *mut c_void, c_dst.len(), src.as_ptr() as *const c_void, len, c_ct.as_ptr() as *const c_void, flags);
                    let rc = (rf)(r_dst.as_mut_ptr() as *mut c_void, r_dst.len(), src.as_ptr() as *const c_void, len, r_ct.as_ptr() as *const c_void, flags);
                    let ctx = format!("{ctx0} HUF_compress{which}X_usingCTable");
                    let ce = (c_iserr)(cc) != 0;
                    let re = (r_iserr)(rc) != 0;
                    assert_eq!(ce, re, "{ctx}: err-status C={cc} R={rc}");
                    if ce {
                        assert_eq!(cstr((c_ername)(cc)), cstr((r_ername)(rc)), "{ctx}: err-name");
                        continue;
                    }
                    assert_eq!(cc, rc, "{ctx}: size");
                    assert_bytes_eq(&format!("{ctx} full dst"), &c_dst, &r_dst);
                }

                // read DTable X1/X2 from the header, compare full DTable memory
                for (dv, c_rdt, r_rdt) in [(1u8, &c_rd1, &r_rd1), (2u8, &c_rd2, &r_rd2)] {
                    let dt_u32 = huf_dtable_size_u32(HUF_TABLELOG_MAX) + 64;
                    let mut c_dt = vec![0u32; dt_u32];
                    let mut r_dt = vec![0u32; dt_u32];
                    let mut cwd = vec![0u8; HUF_DECOMPRESS_WORKSPACE_SIZE];
                    let mut rwd = vec![0u8; HUF_DECOMPRESS_WORKSPACE_SIZE];
                    let crd = (c_rdt)(c_dt.as_mut_ptr() as *mut c_void, c_hdr.as_ptr() as *const c_void, hdr_size, cwd.as_mut_ptr() as *mut c_void, HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
                    let rrd = (r_rdt)(r_dt.as_mut_ptr() as *mut c_void, r_hdr.as_ptr() as *const c_void, hdr_size, rwd.as_mut_ptr() as *mut c_void, HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
                    let ctx = format!("{ctx0} HUF_readDTableX{dv}_wksp");
                    if cmp_ret_status(&ctx, crd, rrd, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                        assert_bytes_eq(&format!("{ctx} DTable"), bytes_u32(&c_dt), bytes_u32(&r_dt));
                    }
                }

                // 1X/4X repeat writes a self-describing stream (header + payload) that
                // the DCtx_wksp decoders can consume end-to-end.
                for (which, cf, rf) in [(1u8, &c_c1r, &r_c1r), (4u8, &c_c4r, &r_c4r)] {
                    let mut c_dst = vec![FILL; cap + 16];
                    let mut r_dst = vec![FILL; cap + 16];
                    let mut c_ht = vec![0usize; ct_st];
                    let mut r_ht = vec![0usize; ct_st];
                    let mut c_hw = vec![0u8; HUF_WORKSPACE_SIZE];
                    let mut r_hw = vec![0u8; HUF_WORKSPACE_SIZE];
                    let mut c_rep: c_int = 0; // HUF_repeat_none
                    let mut r_rep: c_int = 0;
                    let cc = (cf)(c_dst.as_mut_ptr() as *mut c_void, c_dst.len(), src.as_ptr() as *const c_void, len, msv, table_log, c_hw.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE, c_ht.as_mut_ptr() as *mut c_void, &mut c_rep, flags);
                    let rc = (rf)(r_dst.as_mut_ptr() as *mut c_void, r_dst.len(), src.as_ptr() as *const c_void, len, msv, table_log, r_hw.as_mut_ptr() as *mut c_void, HUF_WORKSPACE_SIZE, r_ht.as_mut_ptr() as *mut c_void, &mut r_rep, flags);
                    let ctx = format!("{ctx0} HUF_compress{which}X_repeat");
                    let ce = (c_iserr)(cc) != 0;
                    let re = (r_iserr)(rc) != 0;
                    assert_eq!(ce, re, "{ctx}: err-status C={cc} R={rc}");
                    if ce {
                        assert_eq!(cstr((c_ername)(cc)), cstr((r_ername)(rc)), "{ctx}: err-name");
                        continue;
                    }
                    assert_eq!(cc, rc, "{ctx}: size");
                    assert_bytes_eq(&format!("{ctx} full dst"), &c_dst, &r_dst);
                    assert_eq!(c_rep, r_rep, "{ctx}: repeat state");
                    assert_bytes_eq(&format!("{ctx} hufTable"), bytes_usize(&c_ht), bytes_usize(&r_ht));
                    let comp = cc;
                    if comp == 0 {
                        continue; // not compressible (raw block); nothing huffman to decode
                    }

                    let dt_u32 = huf_dtable_size_u32(HUF_TABLELOG_MAX) + 64;
                    let decode = |df: &FnHufDecompressDCtxWksp, srcbuf: &[u8]| -> (size_t, Vec<u8>) {
                        let mut dt = vec![0u32; dt_u32];
                        let mut out = vec![FILL; len + 16];
                        let mut ws = vec![0u8; HUF_DECOMPRESS_WORKSPACE_SIZE];
                        let r = (df)(dt.as_mut_ptr() as *mut c_void, out.as_mut_ptr() as *mut c_void, len, srcbuf.as_ptr() as *const c_void, comp, ws.as_mut_ptr() as *mut c_void, HUF_DECOMPRESS_WORKSPACE_SIZE, flags);
                        (r, out)
                    };

                    if which == 1 {
                        for (name, cdf, rdf) in [
                            ("HUF_decompress1X1_DCtx_wksp", &c_dc1, &r_dc1),
                            ("HUF_decompress1X2_DCtx_wksp", &c_dc12, &r_dc12),
                            ("HUF_decompress1X_DCtx_wksp", &c_dc1x, &r_dc1x),
                        ] {
                            let (cr, cout) = decode(cdf, &c_dst);
                            let (rr, rout) = decode(rdf, &r_dst);
                            let ctx = format!("{ctx0} {name}");
                            if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                                assert_bytes_eq(&format!("{ctx} decoded"), &cout, &rout);
                                assert_eq!(&cout[..len], &src[..], "{ctx}: roundtrip vs source");
                            }
                        }
                    } else {
                        let (cr, cout) = decode(&c_d4h, &c_dst);
                        let (rr, rout) = decode(&r_d4h, &r_dst);
                        let ctx = format!("{ctx0} HUF_decompress4X_hufOnly_wksp");
                        if cmp_ret_status(&ctx, cr, rr, &c_iserr, &r_iserr, &c_ername, &r_ername) {
                            assert_bytes_eq(&format!("{ctx} decoded"), &cout, &rout);
                            assert_eq!(&cout[..len], &src[..], "{ctx}: roundtrip vs source");
                        }
                    }
                }
                let _ = (&c_d1u, &r_d1u, &c_d4u, &r_d4u); // exercised for symbol presence
            }
        }
    }
}

// =========================================================================
//                              XXHASH TESTS
// =========================================================================

#[test]
fn xxhash_oneshot() {
    let (c32, r32) = fnpair!("ZSTD_XXH32", FnXXH32);
    let (c64, r64) = fnpair!("ZSTD_XXH64", FnXXH64);
    let (cver, rver) = fnpair!("ZSTD_XXH_versionNumber", FnVersion);
    unsafe {
        assert_eq!((cver)(), (rver)(), "ZSTD_XXH_versionNumber");
    }
    let mut rng = Rng::new(0x58480001);
    let mut lens: Vec<usize> = (0..=300).collect();
    lens.extend_from_slice(&[511, 512, 513, 1023, 1024, 4096, 65535, 65536, 131072]);
    unsafe {
        for &shape in ALL_SHAPES.iter() {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                for &seed32 in &[0u32, 1, rng.next_u32(), rng.next_u32(), 0xFFFF_FFFF] {
                    let c = (c32)(src.as_ptr() as *const c_void, len, seed32);
                    let r = (r32)(src.as_ptr() as *const c_void, len, seed32);
                    assert_eq!(c, r, "ZSTD_XXH32(shape={shape:?},len={len},seed={seed32:#x})");
                }
                for &seed64 in &[0u64, 1, rng.next_u64(), rng.next_u64(), u64::MAX] {
                    let c = (c64)(src.as_ptr() as *const c_void, len, seed64);
                    let r = (r64)(src.as_ptr() as *const c_void, len, seed64);
                    assert_eq!(c, r, "ZSTD_XXH64(shape={shape:?},len={len},seed={seed64:#x})");
                }
            }
        }
    }
}

#[test]
fn xxhash_streaming_and_canonical() {
    let (c_cs32, r_cs32) = fnpair!("ZSTD_XXH32_createState", FnXXHCreateState);
    let (c_fs32, r_fs32) = fnpair!("ZSTD_XXH32_freeState", FnXXHFreeState);
    let (c_rs32, r_rs32) = fnpair!("ZSTD_XXH32_reset", FnXXH32Reset);
    let (c_up32, r_up32) = fnpair!("ZSTD_XXH32_update", FnXXHUpdate);
    let (c_dg32, r_dg32) = fnpair!("ZSTD_XXH32_digest", FnXXH32Digest);
    let (c_cp32, r_cp32) = fnpair!("ZSTD_XXH32_copyState", FnXXHCopyState);
    let (c_cs64, r_cs64) = fnpair!("ZSTD_XXH64_createState", FnXXHCreateState);
    let (c_fs64, r_fs64) = fnpair!("ZSTD_XXH64_freeState", FnXXHFreeState);
    let (c_rs64, r_rs64) = fnpair!("ZSTD_XXH64_reset", FnXXH64Reset);
    let (c_up64, r_up64) = fnpair!("ZSTD_XXH64_update", FnXXHUpdate);
    let (c_dg64, r_dg64) = fnpair!("ZSTD_XXH64_digest", FnXXH64Digest);
    let (c_cp64, r_cp64) = fnpair!("ZSTD_XXH64_copyState", FnXXHCopyState);
    let (c_c32c, r_c32c) = fnpair!("ZSTD_XXH32_canonicalFromHash", FnXXH32CanonicalFromHash);
    let (c_c32h, r_c32h) = fnpair!("ZSTD_XXH32_hashFromCanonical", FnXXH32HashFromCanonical);
    let (c_c64c, r_c64c) = fnpair!("ZSTD_XXH64_canonicalFromHash", FnXXH64CanonicalFromHash);
    let (c_c64h, r_c64h) = fnpair!("ZSTD_XXH64_hashFromCanonical", FnXXH64HashFromCanonical);

    let mut rng = Rng::new(0x58481234);
    let lens: Vec<usize> = vec![0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 100, 127, 128, 200, 255, 256, 512, 1000, 4096, 65536];

    unsafe {
        for &shape in ALL_SHAPES.iter() {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                for &seed in &[0u64, 1, rng.next_u64(), 0x1234_5678_9ABC_DEF0] {
                    // 64-bit streaming with randomized chunk splits
                    let cst = (c_cs64)();
                    let rst = (r_cs64)();
                    assert!(!cst.is_null() && !rst.is_null(), "XXH64 createState");
                    assert_eq!((c_rs64)(cst, seed), (r_rs64)(rst, seed), "XXH64_reset err");
                    let mut pos = 0usize;
                    while pos < len {
                        let remain = len - pos;
                        let chunk = (1 + rng.below(remain.max(1))).min(remain);
                        let ce = (c_up64)(cst, src[pos..].as_ptr() as *const c_void, chunk);
                        let re = (r_up64)(rst, src[pos..].as_ptr() as *const c_void, chunk);
                        assert_eq!(ce, re, "XXH64_update err (shape={shape:?},len={len},pos={pos},chunk={chunk})");
                        pos += chunk;
                    }
                    let cst2 = (c_cs64)();
                    let rst2 = (r_cs64)();
                    (c_cp64)(cst2, cst);
                    (r_cp64)(rst2, rst);
                    let cd = (c_dg64)(cst);
                    let rd = (r_dg64)(rst);
                    assert_eq!(cd, rd, "XXH64_digest(shape={shape:?},len={len},seed={seed:#x})");
                    let cd2 = (c_dg64)(cst2);
                    let rd2 = (r_dg64)(rst2);
                    assert_eq!(cd2, rd2, "XXH64_digest(copied)(shape={shape:?},len={len})");
                    assert_eq!(cd, cd2, "XXH64 copyState digest equivalence");
                    assert_eq!((c_fs64)(cst), (r_fs64)(rst), "XXH64_freeState err");
                    let _ = (c_fs64)(cst2);
                    let _ = (r_fs64)(rst2);

                    let mut c_can = [FILL; 8];
                    let mut r_can = [FILL; 8];
                    (c_c64c)(c_can.as_mut_ptr() as *mut c_void, cd);
                    (r_c64c)(r_can.as_mut_ptr() as *mut c_void, rd);
                    assert_bytes_eq(&format!("XXH64_canonicalFromHash(len={len})"), &c_can, &r_can);
                    let ch = (c_c64h)(c_can.as_ptr() as *const c_void);
                    let rh = (r_c64h)(r_can.as_ptr() as *const c_void);
                    assert_eq!(ch, rh, "XXH64_hashFromCanonical(len={len})");
                    assert_eq!(ch, cd, "XXH64 canonical roundtrip");

                    // 32-bit streaming
                    let seed32 = seed as u32;
                    let cst = (c_cs32)();
                    let rst = (r_cs32)();
                    assert_eq!((c_rs32)(cst, seed32), (r_rs32)(rst, seed32), "XXH32_reset err");
                    let mut pos = 0usize;
                    while pos < len {
                        let remain = len - pos;
                        let chunk = (1 + rng.below(remain.max(1))).min(remain);
                        let ce = (c_up32)(cst, src[pos..].as_ptr() as *const c_void, chunk);
                        let re = (r_up32)(rst, src[pos..].as_ptr() as *const c_void, chunk);
                        assert_eq!(ce, re, "XXH32_update err (len={len},pos={pos})");
                        pos += chunk;
                    }
                    let cst2 = (c_cs32)();
                    let rst2 = (r_cs32)();
                    (c_cp32)(cst2, cst);
                    (r_cp32)(rst2, rst);
                    let cd = (c_dg32)(cst);
                    let rd = (r_dg32)(rst);
                    assert_eq!(cd, rd, "XXH32_digest(shape={shape:?},len={len},seed={seed32:#x})");
                    let cd2 = (c_dg32)(cst2);
                    let rd2 = (r_dg32)(rst2);
                    assert_eq!(cd2, rd2, "XXH32_digest(copied)(len={len})");
                    assert_eq!((c_fs32)(cst), (r_fs32)(rst), "XXH32_freeState err");
                    let _ = (c_fs32)(cst2);
                    let _ = (r_fs32)(rst2);

                    let mut c_can = [FILL; 4];
                    let mut r_can = [FILL; 4];
                    (c_c32c)(c_can.as_mut_ptr() as *mut c_void, cd);
                    (r_c32c)(r_can.as_mut_ptr() as *mut c_void, rd);
                    assert_bytes_eq(&format!("XXH32_canonicalFromHash(len={len})"), &c_can, &r_can);
                    let ch = (c_c32h)(c_can.as_ptr() as *const c_void);
                    let rh = (r_c32h)(r_can.as_ptr() as *const c_void);
                    assert_eq!(ch, rh, "XXH32_hashFromCanonical(len={len})");
                    assert_eq!(ch, cd, "XXH32 canonical roundtrip");
                }
            }
        }
    }
}

// -------------------------------------------------- byte-view helpers ------

fn bytes_u32(s: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}
fn bytes_i16(s: &[i16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}
fn bytes_usize(s: &[usize]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}
