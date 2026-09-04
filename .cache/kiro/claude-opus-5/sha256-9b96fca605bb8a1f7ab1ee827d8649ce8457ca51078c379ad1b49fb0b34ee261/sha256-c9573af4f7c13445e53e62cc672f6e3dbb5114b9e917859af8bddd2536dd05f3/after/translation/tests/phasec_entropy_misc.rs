//! PHASE C error-path differential tests — part 4 of 4.
//!
//! Covers ERRORS.md rows 263-328:
//!   * FSE / HUF entropy low level              (rows 263-307)
//!   * Deprecated ZBUFF                         (rows 308-313)
//!   * Legacy v01–v07 decoders                  (rows 314-317)
//!   * Enum / out-of-range values crossing FFI  (rows 318-328)
//!
//! Every case constructs an EXACT invalid input/condition, calls BOTH the C
//! `libzstd.so` and the Rust `libzstd.so` through their exported symbols, and
//! asserts they return the SAME error (matching isError + errorName strings)
//! AND the same raw size_t return value.
//!
//! EXPORTED SURFACE (verified via `nm -D --defined-only`): this build exports
//! only a subset of FSE/HUF — the `_wksp` / `_usingCTable` / `_repeat` family
//! plus tool functions. Functions named in ERRORS.md that are NOT exported
//! (FSE_compress, FSE_decompress, HUF_compress*, HUF_writeCTable non-wksp,
//! HUF_decompress1X1 public wrappers) are reached through the nearest exported
//! entry point; each substitution is noted in a comment.
//!
//! PRECONDITION-UB NOTE: several low-level functions enforce preconditions only
//! via `assert()` (compiled out in release). Violating those is UB and can
//! segfault the C reference too — those are C precondition violations, NOT Rust
//! bugs, so we do not chase them; each avoided case is noted in a comment.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

const FILL: u8 = 0xAA;

// ---- FSE / HUF constants (from fse.h / huf.h) ----
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const FSE_MAX_TABLELOG: u32 = 12;
const FSE_MIN_TABLELOG: u32 = 5;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const HUF_TABLELOG_MAX: u32 = 12;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;

// ------------------------------------------------------------ fn types ----

type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnFseReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t, c_int) -> size_t;
type FnFseNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, size_t, c_uint, c_uint) -> size_t;
type FnFseWriteNCount =
    unsafe extern "C" fn(*mut c_void, size_t, *const i16, c_uint, c_uint) -> size_t;
type FnFseNCountWriteBound = unsafe extern "C" fn(c_uint, c_uint) -> size_t;
type FnFseBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnFseBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnFseDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, c_uint, *mut c_void, size_t, c_int,
) -> size_t;
type FnFseOptimalTableLog = unsafe extern "C" fn(c_uint, size_t, c_uint) -> c_uint;
type FnFseOptimalTableLogInternal =
    unsafe extern "C" fn(c_uint, size_t, c_uint, c_uint) -> c_uint;

// HUF
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8, size_t, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, size_t,
) -> size_t;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8, size_t, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, size_t,
    *mut c_void, size_t, c_int,
) -> size_t;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut c_void, *mut c_uint, *const c_void, size_t, *mut c_uint) -> size_t;
type FnHufReadCTableHeader =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut c_void, *const c_uint, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufWriteCTableWksp = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, c_uint, c_uint, *mut c_void, size_t,
) -> size_t;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, *mut c_void, size_t, c_int) -> size_t;
type FnHufCompressUsingCTable = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, *const c_void, c_int,
) -> size_t;
type FnHufCompressRepeat = unsafe extern "C" fn(
    *mut c_void, size_t, *const c_void, size_t, c_uint, c_uint, *mut c_void, size_t,
    *mut c_void, *mut c_int, c_int,
) -> size_t;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *mut c_void, size_t, c_int,
) -> size_t;
type FnHufValidateCTable = unsafe extern "C" fn(*const c_void, *const c_uint, c_uint) -> c_int;
type FnHufGetNbBits = unsafe extern "C" fn(*const c_void, c_uint) -> c_uint;

// ZBUFF
type FnZbuffPtr = unsafe extern "C" fn() -> *mut c_void;
type FnZbuffFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnZbuffCInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnZbuffCContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnZbuffCFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t) -> size_t;
type FnZbuffDInit = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnZbuffDContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;

// legacy
type FnLegacyDecompress =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnLegacyIsError = unsafe extern "C" fn(size_t) -> c_uint;

// ZSTD public
type FnGetFrameContentSize = unsafe extern "C" fn(*const c_void, size_t) -> u64;
type FnGetErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCtxReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int) -> size_t;

// ------------------------------------------------------------ helpers ----

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
}

/// Assert C and Rust returned the same error status + same error-name + same
/// raw return value, using the supplied isError/getErrorName pair.
#[track_caller]
fn assert_same(
    ctx: &str,
    cr: size_t,
    rr: size_t,
    c_iserr: &FnIsError,
    r_iserr: &FnIsError,
    c_ername: &FnErrName,
    r_ername: &FnErrName,
) {
    unsafe {
        let ce = (c_iserr)(cr) != 0;
        let re = (r_iserr)(rr) != 0;
        assert_eq!(ce, re, "{ctx}: error-status mismatch C_ret={cr}(err={ce}) Rust_ret={rr}(err={re})");
        if ce {
            let cn = cstr((c_ername)(cr));
            let rn = cstr((r_ername)(rr));
            assert_eq!(cn, rn, "{ctx}: error-name mismatch C='{cn}' Rust='{rn}'");
        }
    }
    assert_eq!(cr, rr, "{ctx}: raw return value mismatch C={cr} Rust={rr}");
}

// =========================================================================
//  FSE read/normalize/write/build/decompress error paths
//  ERRORS.md rows: 263,264,265,266,267,276,277,278,279,280,281,282,283,284,
//                  285,286,287,288
// =========================================================================

fn build_valid_ncount() -> (Vec<u8>, usize, u32, u32) {
    // Build a real FSE NCount header we can then truncate/mutate.
    // distribution over 4 symbols, tableLog 5.
    let (c_norm, _r) = fnpair!("FSE_normalizeCount", FnFseNormalizeCount);
    let (c_wnc, _r2) = fnpair!("FSE_writeNCount", FnFseWriteNCount);
    let (c_ncwb, _r3) = fnpair!("FSE_NCountWriteBound", FnFseNCountWriteBound);
    let count: [u32; 4] = [40, 30, 20, 10];
    let msv = 3u32;
    let tl = 5u32;
    unsafe {
        let mut norm = vec![0i16; 256];
        let cn = (c_norm)(norm.as_mut_ptr(), tl, count.as_ptr(), 100, msv, 0);
        assert!((*&fnpair!("FSE_isError", FnIsError).0)(cn) == 0, "setup normalize failed");
        let actual_tl = cn as u32;
        let bound = (c_ncwb)(msv, actual_tl).max(64);
        let mut buf = vec![0u8; bound + 16];
        let w = (c_wnc)(buf.as_mut_ptr() as *mut c_void, buf.len(), norm.as_ptr(), msv, actual_tl);
        assert!((*&fnpair!("FSE_isError", FnIsError).0)(w) == 0, "setup writeNCount failed");
        buf.truncate(w);
        (buf, w, msv, actual_tl)
    }
}

#[test]
fn fse_read_ncount_errors() {
    let (c_rnc, r_rnc) = fnpair!("FSE_readNCount", FnFseReadNCount);
    let (c_rnc2, r_rnc2) = fnpair!("FSE_readNCount_bmi2", FnFseReadNCountBmi2);
    let (c_ie, r_ie) = fnpair!("FSE_isError", FnIsError);
    let (c_en, r_en) = fnpair!("FSE_getErrorName", FnErrName);

    let call = |rnc: &FnFseReadNCount, buf: &[u8], maxsv: u32| -> (size_t, u32, u32) {
        unsafe {
            let mut norm = vec![0i16; 256];
            let mut msv = maxsv;
            let mut tl = 0u32;
            let r = (rnc)(norm.as_mut_ptr(), &mut msv, &mut tl, buf.as_ptr() as *const c_void, buf.len());
            (r, msv, tl)
        }
    };
    let call2 = |rnc: &FnFseReadNCountBmi2, buf: &[u8], maxsv: u32, bmi2: c_int| -> (size_t, u32, u32) {
        unsafe {
            let mut norm = vec![0i16; 256];
            let mut msv = maxsv;
            let mut tl = 0u32;
            let r = (rnc)(norm.as_mut_ptr(), &mut msv, &mut tl, buf.as_ptr() as *const c_void, buf.len(), bmi2);
            (r, msv, tl)
        }
    };

    // row 264/265/267: empty & 1-byte buffers (header consumes past input / bad)
    for (label, buf) in [
        ("ERRORS row 264: empty buffer", Vec::<u8>::new()),
        ("ERRORS row 264: 1-byte buffer", vec![0x00u8]),
        ("ERRORS row 264: 1-byte buffer 0xFF", vec![0xFFu8]),
    ] {
        let (cr, cm, ct) = call(&c_rnc, &buf, 255);
        let (rr, rm, rt) = call(&r_rnc, &buf, 255);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cm, ct), (rm, rt), "{label}: out params");
    }

    // valid header + every truncated prefix (row 264 countSize > hbSize)
    let (hdr, hlen, _msv, _tl) = build_valid_ncount();
    for prefix in 0..hlen {
        let buf = &hdr[..prefix];
        let label = format!("ERRORS row 264: truncated NCount prefix len={prefix}");
        let (cr, cm, ct) = call(&c_rnc, buf, 255);
        let (rr, rm, rt) = call(&r_rnc, buf, 255);
        assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cm, ct), (rm, rt), "{label}: out params");
    }

    // row 266: maxSymbolValue smaller than header's -> maxSymbolValue_tooSmall.
    // Feed the full valid header but with caller maxSV=1 (< real 3).
    {
        let label = "ERRORS row 266: maxSymbolValue smaller than header's";
        let (cr, cm, ct) = call(&c_rnc, &hdr, 1);
        let (rr, rm, rt) = call(&r_rnc, &hdr, 1);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cm, ct), (rm, rt), "{label}: out params");
    }

    // row 263: nbBits > FSE_TABLELOG_ABSOLUTE_MAX. The first NCount byte encodes
    // (tableLog - FSE_MIN_TABLELOG) in its low 4 bits. Setting nibble=0xF makes
    // nbBits = 5 + 15 = 20 > 15 -> tableLog_tooLarge.
    {
        let mut bad = hdr.clone();
        bad[0] = 0xFF; // low nibble 0xF => tableLog too large
        let label = "ERRORS row 263: tableLog > FSE_TABLELOG_ABSOLUTE_MAX";
        let (cr, ..) = call(&c_rnc, &bad, 255);
        let (rr, ..) = call(&r_rnc, &bad, 255);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        let _ = FSE_TABLELOG_ABSOLUTE_MAX;
    }

    // rows 263-267: pure random bytes at many lengths, both readNCount variants.
    let mut rng = Rng::new(0x2630_0001);
    for len in [0usize, 1, 2, 3, 4, 5, 6, 8, 12, 16, 24, 32, 48, 64, 100, 128, 200, 256] {
        for _ in 0..24 {
            let buf: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
            let label = format!("ERRORS rows 263-267: random NCount bytes len={len}");
            let (cr, cm, ct) = call(&c_rnc, &buf, 255);
            let (rr, rm, rt) = call(&r_rnc, &buf, 255);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((cm, ct), (rm, rt), "{label}: out params");
            for bmi2 in [0i32, 1] {
                let (cr, cm, ct) = call2(&c_rnc2, &buf, 255, bmi2);
                let (rr, rm, rt) = call2(&r_rnc2, &buf, 255, bmi2);
                let l2 = format!("{label} bmi2={bmi2}");
                assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                assert_eq!((cm, ct), (rm, rt), "{l2}: out params");
            }
        }
    }
}

#[test]
fn fse_normalize_and_write_errors() {
    let (c_norm, r_norm) = fnpair!("FSE_normalizeCount", FnFseNormalizeCount);
    let (c_wnc, r_wnc) = fnpair!("FSE_writeNCount", FnFseWriteNCount);
    let (c_ncwb, _r) = fnpair!("FSE_NCountWriteBound", FnFseNCountWriteBound);
    let (c_ie, r_ie) = fnpair!("FSE_isError", FnIsError);
    let (c_en, r_en) = fnpair!("FSE_getErrorName", FnErrName);

    let norm_call = |f: &FnFseNormalizeCount, tl: u32, count: &[u32], total: usize, msv: u32| -> (size_t, Vec<i16>) {
        unsafe {
            let mut norm = vec![0i16; 256];
            let r = (f)(norm.as_mut_ptr(), tl, count.as_ptr(), total, msv, 0);
            (r, norm)
        }
    };

    let count: [u32; 4] = [40, 30, 20, 10];

    // row 288: tableLog < FSE_MIN_TABLELOG -> GENERIC
    {
        let label = "ERRORS row 288: tableLog below FSE_MIN_TABLELOG";
        let (cr, cn) = norm_call(&c_norm, 1, &count, 100, 3);
        let (rr, rn) = norm_call(&r_norm, 1, &count, 100, 3);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(cn, rn, "{label}: normalizedCounter");
    }
    // row 287: tableLog > FSE_MAX_TABLELOG -> tableLog_tooLarge
    {
        let label = "ERRORS row 287: tableLog above FSE_MAX_TABLELOG";
        let (cr, cn) = norm_call(&c_norm, FSE_MAX_TABLELOG + 1, &count, 100, 3);
        let (rr, rn) = norm_call(&r_norm, FSE_MAX_TABLELOG + 1, &count, 100, 3);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(cn, rn, "{label}: normalizedCounter");
    }
    // row 286: NOTE total==0 is a C PRECONDITION VIOLATION — FSE_normalizeCount
    // does ZSTD_div64((U64)1<<62, (U32)total), dividing by total, so total==0
    // SIGFPEs the C .so itself. Real callers never normalize an empty histogram.
    // We instead reach the GENERIC "incorrect distribution" path in-contract via
    // tableLog < FSE_minTableLog(total,msv): a large alphabet at minimum tableLog.
    {
        let many: Vec<u32> = (0..256).map(|_| 1u32).collect();
        let label = "ERRORS row 286/288: tableLog below FSE_minTableLog (incorrect distribution)";
        let (cr, cn) = norm_call(&c_norm, FSE_MIN_TABLELOG, &many, 256, 255);
        let (rr, rn) = norm_call(&r_norm, FSE_MIN_TABLELOG, &many, 256, 255);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(cn, rn, "{label}: normalizedCounter");
    }
    // one symbol holds everything (degenerate but valid-ish; compare exactly)
    {
        let one: [u32; 4] = [100, 0, 0, 0];
        let label = "ERRORS row 286/288: single symbol holds all count";
        let (cr, cn) = norm_call(&c_norm, 6, &one, 100, 3);
        let (rr, rn) = norm_call(&r_norm, 6, &one, 100, 3);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(cn, rn, "{label}: normalizedCounter");
    }
    // maxSymbolValue > FSE_MAX_SYMBOL_VALUE (256): out of contract but function
    // reads count[0..=msv]; provide a 512-entry count so the C read is in-bounds
    // and we compare its actual behaviour.
    {
        let big: Vec<u32> = (0..512).map(|i| if i < 4 { count[i] } else { 0 }).collect();
        let label = "ERRORS row 288: maxSymbolValue > FSE_MAX_SYMBOL_VALUE";
        let (cr, cn) = norm_call(&c_norm, 6, &big, 100, FSE_MAX_SYMBOL_VALUE + 1);
        let (rr, rn) = norm_call(&r_norm, 6, &big, 100, FSE_MAX_SYMBOL_VALUE + 1);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(cn, rn, "{label}: normalizedCounter");
    }

    // rows 285: FSE_writeNCount with dstCapacity below FSE_NCountWriteBound.
    let mut norm = vec![0i16; 256];
    let msv = 3u32;
    let tl = unsafe {
        let r = (c_norm)(norm.as_mut_ptr(), 6, count.as_ptr(), 100, msv, 0);
        assert!((c_ie)(r) == 0);
        r as u32
    };
    let bound = unsafe { (c_ncwb)(msv, tl) };
    for cap in [0usize, 1, bound.saturating_sub(1)] {
        let label = format!("ERRORS row 285: FSE_writeNCount dstCapacity={cap} (bound={bound})");
        let mut cbuf = vec![FILL; cap.max(1)];
        let mut rbuf = vec![FILL; cap.max(1)];
        let (cr, rr) = unsafe {
            (
                (c_wnc)(cbuf.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl),
                (r_wnc)(rbuf.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl),
            )
        };
        assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_bytes_eq(&format!("{label} buffer"), &cbuf, &rbuf);
    }
}

#[test]
fn fse_optimal_table_log_degenerate() {
    // rows 287/288 tool: FSE_optimalTableLog / _internal at degenerate srcSize/msv.
    let (c_otl, r_otl) = fnpair!("FSE_optimalTableLog", FnFseOptimalTableLog);
    let (c_oti, r_oti) = fnpair!("FSE_optimalTableLog_internal", FnFseOptimalTableLogInternal);
    // NOTE: srcSize<=1 and maxSymbolValue==0 are C PRECONDITION VIOLATIONS here
    // (`assert(srcSize > 1)`; ZSTD_highbit32(srcSize-1) and highbit32(maxSymbolValue)
    // are UB when their argument is 0 — the C .so itself SIGFPEs). RLE is used
    // instead for those. We therefore keep srcSize>=2 and msv>=1 and note the
    // exclusion — rows 287/288 degenerate-but-in-contract inputs.
    unsafe {
        for &max_tl in &[0u32, 1, 5, 12, 15] {
            for &src in &[2usize, 3, 4, 16, 65536] {
                for &msv in &[1u32, 2, 255] {
                    let label = format!("ERRORS rows 287/288: FSE_optimalTableLog(max_tl={max_tl},src={src},msv={msv})");
                    assert_eq!((c_otl)(max_tl, src, msv), (r_otl)(max_tl, src, msv), "{label}");
                    for minus in [0u32, 1, 2] {
                        assert_eq!(
                            (c_oti)(max_tl, src, msv, minus),
                            (r_oti)(max_tl, src, msv, minus),
                            "ERRORS rows 287/288: FSE_optimalTableLog_internal(max_tl={max_tl},src={src},msv={msv},minus={minus})"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn fse_build_and_decompress_errors() {
    let (c_bct, r_bct) = fnpair!("FSE_buildCTable_wksp", FnFseBuildCTableWksp);
    let (c_bdt, r_bdt) = fnpair!("FSE_buildDTable_wksp", FnFseBuildDTableWksp);
    let (c_dwb, r_dwb) = fnpair!("FSE_decompress_wksp_bmi2", FnFseDecompressWkspBmi2);
    let (c_ie, r_ie) = fnpair!("FSE_isError", FnIsError);
    let (c_en, r_en) = fnpair!("FSE_getErrorName", FnErrName);

    // A valid normalized counter over 4 symbols at tableLog 6 (sums to 2^6).
    let (c_norm, _r) = fnpair!("FSE_normalizeCount", FnFseNormalizeCount);
    let count: [u32; 4] = [40, 30, 20, 10];
    let mut norm = vec![0i16; 256];
    let (msv, tl) = unsafe {
        let r = (c_norm)(norm.as_mut_ptr(), 6, count.as_ptr(), 100, 3, 0);
        assert!((c_ie)(r) == 0, "setup normalize");
        (3u32, r as u32)
    };

    // row 284: FSE_buildCTable_wksp workspace too small -> tableLog_tooLarge.
    {
        let label = "ERRORS row 284: FSE_buildCTable_wksp undersized workspace";
        let mut ct = vec![0u32; 1 + (1 << 6) + 8];
        let mut cws = vec![0u8; 4];
        let mut rws = vec![0u8; 4];
        let cr = unsafe { (c_bct)(ct.as_mut_ptr(), norm.as_ptr(), msv, tl, cws.as_mut_ptr() as *mut c_void, 4) };
        let rr = unsafe { (r_bct)(ct.as_mut_ptr(), norm.as_ptr(), msv, tl, rws.as_mut_ptr() as *mut c_void, 4) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // invalid tableLog for buildCTable (too large) with adequate workspace.
    {
        let label = "ERRORS row 284/287: FSE_buildCTable_wksp invalid tableLog";
        let mut ct = vec![0u32; 1 + (1 << 15) + 600];
        let big = 512usize;
        let mut cws = vec![0u8; big];
        let mut rws = vec![0u8; big];
        let cr = unsafe { (c_bct)(ct.as_mut_ptr(), norm.as_ptr(), msv, FSE_MAX_TABLELOG + 3, cws.as_mut_ptr() as *mut c_void, big) };
        let rr = unsafe { (r_bct)(ct.as_mut_ptr(), norm.as_ptr(), msv, FSE_MAX_TABLELOG + 3, rws.as_mut_ptr() as *mut c_void, big) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }

    // row 276: FSE_buildDTable_wksp maxSymbolValue > FSE_MAX_SYMBOL_VALUE.
    {
        let label = "ERRORS row 276: FSE_buildDTable_wksp maxSymbolValue too large";
        let mut dt = vec![0u32; 1 + (1 << 6) + 8];
        let big = 4096usize;
        let mut cws = vec![0u8; big];
        let mut rws = vec![0u8; big];
        let cr = unsafe { (c_bdt)(dt.as_mut_ptr(), norm.as_ptr(), FSE_MAX_SYMBOL_VALUE + 1, tl, cws.as_mut_ptr() as *mut c_void, big) };
        let rr = unsafe { (r_bdt)(dt.as_mut_ptr(), norm.as_ptr(), FSE_MAX_SYMBOL_VALUE + 1, tl, rws.as_mut_ptr() as *mut c_void, big) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 276: FSE_buildDTable_wksp workspace too small -> maxSymbolValue_tooLarge.
    {
        let label = "ERRORS row 276: FSE_buildDTable_wksp undersized workspace";
        let mut dt = vec![0u32; 1 + (1 << 6) + 8];
        let mut cws = vec![0u8; 4];
        let mut rws = vec![0u8; 4];
        let cr = unsafe { (c_bdt)(dt.as_mut_ptr(), norm.as_ptr(), msv, tl, cws.as_mut_ptr() as *mut c_void, 4) };
        let rr = unsafe { (r_bdt)(dt.as_mut_ptr(), norm.as_ptr(), msv, tl, rws.as_mut_ptr() as *mut c_void, 4) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 277: FSE_buildDTable_wksp tableLog > FSE_MAX_TABLELOG.
    {
        let label = "ERRORS row 277: FSE_buildDTable_wksp tableLog too large";
        let mut dt = vec![0u32; 1 + (1 << 15) + 8];
        let big = 8192usize;
        let mut cws = vec![0u8; big];
        let mut rws = vec![0u8; big];
        let cr = unsafe { (c_bdt)(dt.as_mut_ptr(), norm.as_ptr(), msv, FSE_MAX_TABLELOG + 1, cws.as_mut_ptr() as *mut c_void, big) };
        let rr = unsafe { (r_bdt)(dt.as_mut_ptr(), norm.as_ptr(), msv, FSE_MAX_TABLELOG + 1, rws.as_mut_ptr() as *mut c_void, big) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 278: normalizedCounter that does not sum to 2^tableLog -> GENERIC.
    {
        let label = "ERRORS row 278: FSE_buildDTable_wksp bad normalized distribution";
        let mut bad = norm.clone();
        bad[0] = bad[0].wrapping_add(7); // perturb so it no longer sums to 2^tl
        let mut dt = vec![0u32; 1 + (1 << 6) + 8];
        let big = 4096usize;
        let mut cws = vec![0u8; big];
        let mut rws = vec![0u8; big];
        let cr = unsafe { (c_bdt)(dt.as_mut_ptr(), bad.as_ptr(), msv, tl, cws.as_mut_ptr() as *mut c_void, big) };
        let rr = unsafe { (r_bdt)(dt.as_mut_ptr(), bad.as_ptr(), msv, tl, rws.as_mut_ptr() as *mut c_void, big) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }

    // rows 279/280/281/282/283: FSE_decompress_wksp_bmi2 error paths.
    let wksp_u32 = 4096usize; // generous but we also test the too-small case
    for bmi2 in [0i32, 1] {
        // empty / 1-byte / corrupt payloads (row 279 overflow / corruption)
        for (tag, payload) in [
            ("empty", Vec::<u8>::new()),
            ("1-byte", vec![0x20u8]),
            ("corrupt", vec![0x06u8, 0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22]),
        ] {
            let label = format!("ERRORS rows 279-283: FSE_decompress_wksp_bmi2 {tag} bmi2={bmi2}");
            let mut cout = vec![FILL; 64];
            let mut rout = vec![FILL; 64];
            let mut cws = vec![0u32; wksp_u32];
            let mut rws = vec![0u32; wksp_u32];
            let cr = unsafe { (c_dwb)(cout.as_mut_ptr() as *mut c_void, 64, payload.as_ptr() as *const c_void, payload.len(), FSE_MAX_TABLELOG, cws.as_mut_ptr() as *mut c_void, (wksp_u32 * 4) as size_t, bmi2) };
            let rr = unsafe { (r_dwb)(rout.as_mut_ptr() as *mut c_void, 64, payload.as_ptr() as *const c_void, payload.len(), FSE_MAX_TABLELOG, rws.as_mut_ptr() as *mut c_void, (wksp_u32 * 4) as size_t, bmi2) };
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_bytes_eq(&format!("{label} out"), &cout, &rout);
        }
        // row 281/283: workspace too small -> GENERIC / tableLog_tooLarge.
        {
            let label = format!("ERRORS rows 281/283: FSE_decompress_wksp_bmi2 tiny workspace bmi2={bmi2}");
            let payload = vec![0x06u8, 0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22];
            let mut cout = vec![FILL; 64];
            let mut rout = vec![FILL; 64];
            let mut cws = vec![0u32; 2];
            let mut rws = vec![0u32; 2];
            let cr = unsafe { (c_dwb)(cout.as_mut_ptr() as *mut c_void, 64, payload.as_ptr() as *const c_void, payload.len(), FSE_MAX_TABLELOG, cws.as_mut_ptr() as *mut c_void, 8, bmi2) };
            let rr = unsafe { (r_dwb)(rout.as_mut_ptr() as *mut c_void, 64, payload.as_ptr() as *const c_void, payload.len(), FSE_MAX_TABLELOG, rws.as_mut_ptr() as *mut c_void, 8, bmi2) };
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_bytes_eq(&format!("{label} out"), &cout, &rout);
        }
    }
}

// =========================================================================
//  HUF readStats / readCTable / build / write / read DTable / compress /
//  decompress error paths.
//  ERRORS.md rows: 268,269,270,271,272,273,274,275,289,290,291,292,293,294,
//                  295,296,297,298,299,300,301,302,303,304,305,306,307
// =========================================================================

/// Build a valid HUF CTable header ("weight table") we can truncate/corrupt,
/// via the exported HUF_buildCTable_wksp + HUF_writeCTable_wksp path.
/// Returns (header_bytes, ctable_st_len, msv, max_bits).
fn build_valid_huf_header() -> (Vec<u8>, usize, u32, u32) {
    type FnHist = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
    let (c_hist, _r) = fnpair!("HIST_count", FnHist);
    let (c_bct, _r2) = fnpair!("HUF_buildCTable_wksp", FnHufBuildCTableWksp);
    let (c_wct, _r3) = fnpair!("HUF_writeCTable_wksp", FnHufWriteCTableWksp);
    let (c_ie, _r4) = fnpair!("HUF_isError", FnIsError);
    // A byte distribution with several distinct symbols.
    let src: Vec<u8> = (0..4096u32).map(|i| ((i * 7 + (i >> 3)) & 0x3F) as u8).collect();
    let ct_st = HUF_SYMBOLVALUE_MAX as usize + 2 + 8;
    unsafe {
        let mut count = vec![0u32; 256];
        let mut msv = 255u32;
        let _ = (c_hist)(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
        let mut ct = vec![0usize; ct_st];
        let ctws_sz = ((4 * 256) + 192) * 4;
        let mut ctws = vec![0u8; ctws_sz];
        let mb = (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), msv, HUF_TABLELOG_MAX, ctws.as_mut_ptr() as *mut c_void, ctws_sz);
        assert!((c_ie)(mb) == 0, "setup buildCTable failed");
        let max_bits = mb as u32;
        let mut hdr = vec![0u8; 512];
        let wsz = (8 << 10) + 512;
        let mut ws = vec![0u8; wsz];
        let w = (c_wct)(hdr.as_mut_ptr() as *mut c_void, hdr.len(), ct.as_ptr() as *const c_void, msv, max_bits, ws.as_mut_ptr() as *mut c_void, wsz);
        assert!((c_ie)(w) == 0, "setup writeCTable failed");
        hdr.truncate(w);
        (hdr, ct_st, msv, max_bits)
    }
}

#[test]
fn huf_read_stats_errors() {
    let (c_rs, r_rs) = fnpair!("HUF_readStats", FnHufReadStats);
    let (c_rsw, r_rsw) = fnpair!("HUF_readStats_wksp", FnHufReadStatsWksp);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);

    let rswu = 4096usize; // >= HUF_READ_STATS_WORKSPACE_SIZE
    let run = |buf: &[u8], hwsize: usize, label: &str| {
        unsafe {
            let mut c_hw = vec![FILL; hwsize];
            let mut r_hw = vec![FILL; hwsize];
            let mut c_rank = [0u32; 16];
            let mut r_rank = [0u32; 16];
            let (mut c_ns, mut r_ns, mut c_tl, mut r_tl) = (0u32, 0u32, 0u32, 0u32);
            let cr = (c_rs)(c_hw.as_mut_ptr(), hwsize, c_rank.as_mut_ptr(), &mut c_ns, &mut c_tl, buf.as_ptr() as *const c_void, buf.len());
            let rr = (r_rs)(r_hw.as_mut_ptr(), hwsize, r_rank.as_mut_ptr(), &mut r_ns, &mut r_tl, buf.as_ptr() as *const c_void, buf.len());
            assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((c_ns, c_tl), (r_ns, r_tl), "{label}: out params");
            assert_bytes_eq(&format!("{label} huffWeight"), &c_hw, &r_hw);
            assert_bytes_eq(&format!("{label} rankStats"), bytes_of_u32(&c_rank), bytes_of_u32(&r_rank));

            let mut c_hw = vec![FILL; hwsize];
            let mut r_hw = vec![FILL; hwsize];
            let mut c_rank = [0u32; 16];
            let mut r_rank = [0u32; 16];
            let (mut c_ns, mut r_ns, mut c_tl, mut r_tl) = (0u32, 0u32, 0u32, 0u32);
            let mut cws = vec![0u8; rswu];
            let mut rws = vec![0u8; rswu];
            let cr = (c_rsw)(c_hw.as_mut_ptr(), hwsize, c_rank.as_mut_ptr(), &mut c_ns, &mut c_tl, buf.as_ptr() as *const c_void, buf.len(), cws.as_mut_ptr() as *mut c_void, rswu, 0);
            let rr = (r_rsw)(r_hw.as_mut_ptr(), hwsize, r_rank.as_mut_ptr(), &mut r_ns, &mut r_tl, buf.as_ptr() as *const c_void, buf.len(), rws.as_mut_ptr() as *mut c_void, rswu, 0);
            let l2 = format!("{label} (wksp)");
            assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((c_ns, c_tl), (r_ns, r_tl), "{l2}: out params");
        }
    };

    // row 268: empty header -> srcSize_wrong
    run(&[], 256, "ERRORS row 268: HUF_readStats empty header");
    // row 269: 1-byte + truncated (iSize+1 > srcSize)
    run(&[0x30u8], 256, "ERRORS row 269: HUF_readStats 1-byte header");

    // valid header + every truncated prefix (rows 269-275)
    let (hdr, _ct, _msv, _mb) = build_valid_huf_header();
    for prefix in 0..hdr.len().min(40) {
        run(&hdr[..prefix], 256, &format!("ERRORS rows 269-275: HUF_readStats truncated prefix len={prefix}"));
    }

    // row 270: more symbols than caller's hwSize (oSize >= hwSize).
    run(&hdr, 2, "ERRORS row 270: HUF_readStats hwSize too small (2)");
    run(&hdr, 4, "ERRORS row 270: HUF_readStats hwSize too small (4)");

    // rows 271-275: pure random bytes at many lengths.
    let mut rng = Rng::new(0x2680_0001);
    for len in [0usize, 1, 2, 3, 4, 8, 16, 32, 64, 128, 200, 256] {
        for _ in 0..24 {
            let buf: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
            run(&buf, 256, &format!("ERRORS rows 271-275: HUF_readStats random bytes len={len}"));
        }
    }

    // row 289: HUF_readStats_wksp with undersized workspace.
    unsafe {
        let label = "ERRORS row 289: HUF_readStats_wksp undersized workspace";
        let mut hw = vec![FILL; 256];
        let mut rank = [0u32; 16];
        let (mut c_ns, mut c_tl) = (0u32, 0u32);
        let mut cws = vec![0u8; 8];
        let cr = (c_rsw)(hw.as_mut_ptr(), 256, rank.as_mut_ptr(), &mut c_ns, &mut c_tl, hdr.as_ptr() as *const c_void, hdr.len(), cws.as_mut_ptr() as *mut c_void, 8, 0);
        let mut hw2 = vec![FILL; 256];
        let mut rank2 = [0u32; 16];
        let (mut r_ns, mut r_tl) = (0u32, 0u32);
        let mut rws = vec![0u8; 8];
        let rr = (r_rsw)(hw2.as_mut_ptr(), 256, rank2.as_mut_ptr(), &mut r_ns, &mut r_tl, hdr.as_ptr() as *const c_void, hdr.len(), rws.as_mut_ptr() as *mut c_void, 8, 0);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
}

#[test]
fn huf_read_ctable_errors() {
    let (c_rct, r_rct) = fnpair!("HUF_readCTable", FnHufReadCTable);
    let (c_rch, r_rch) = fnpair!("HUF_readCTableHeader", FnHufReadCTableHeader);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);

    let ct_st = HUF_SYMBOLVALUE_MAX as usize + 2 + 8;
    let read_ct = |f: &FnHufReadCTable, buf: &[u8], maxsv: u32| -> (size_t, u32, u32) {
        unsafe {
            let mut ct = vec![0usize; ct_st];
            let mut msv = maxsv;
            let mut zw = 0u32;
            let r = (f)(ct.as_mut_ptr() as *mut c_void, &mut msv, buf.as_ptr() as *const c_void, buf.len(), &mut zw);
            (r, msv, zw)
        }
    };
    let read_hdr = |f: &FnHufReadCTableHeader, buf: &[u8]| -> (size_t, u32, u32) {
        unsafe {
            let mut nsym = 0u32;
            let mut tl = 0u32;
            let r = (f)(&mut nsym, &mut tl, buf.as_ptr() as *const c_void, buf.len());
            (r, nsym, tl)
        }
    };

    let (hdr, _ct, _msv, _mb) = build_valid_huf_header();
    // truncated + corrupt tables (rows 292/293)
    for prefix in 0..hdr.len().min(40) {
        let buf = &hdr[..prefix];
        let label = format!("ERRORS rows 292/293: HUF_readCTable truncated prefix len={prefix}");
        let (cr, cm, cz) = read_ct(&c_rct, buf, 255);
        let (rr, rm, rz) = read_ct(&r_rct, buf, 255);
        assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cm, cz), (rm, rz), "{label}: out params");

        let l2 = format!("ERRORS rows 292/293: HUF_readCTableHeader truncated prefix len={prefix}");
        let (cr, cn, ct) = read_hdr(&c_rch, buf);
        let (rr, rn, rt) = read_hdr(&r_rch, buf);
        assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cn, ct), (rn, rt), "{l2}: out params");
    }
    // row 293: nbSymbols > maxSymbolValuePtr+1 (caller declares tiny maxSV).
    {
        let label = "ERRORS row 293: HUF_readCTable maxSymbolValue too small";
        let (cr, cm, cz) = read_ct(&c_rct, &hdr, 1);
        let (rr, rm, rz) = read_ct(&r_rct, &hdr, 1);
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!((cm, cz), (rm, rz), "{label}: out params");
    }
    // corrupt: random bytes at many lengths (rows 292/293)
    let mut rng = Rng::new(0x2920_0001);
    for len in [0usize, 1, 2, 3, 4, 8, 16, 32, 64, 128] {
        for _ in 0..16 {
            let buf: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
            let label = format!("ERRORS rows 292/293: HUF_readCTable random bytes len={len}");
            let (cr, ..) = read_ct(&c_rct, &buf, 255);
            let (rr, ..) = read_ct(&r_rct, &buf, 255);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        }
    }
}

#[test]
fn huf_build_write_ctable_errors() {
    let (c_bct, r_bct) = fnpair!("HUF_buildCTable_wksp", FnHufBuildCTableWksp);
    let (c_wct, r_wct) = fnpair!("HUF_writeCTable_wksp", FnHufWriteCTableWksp);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);

    let count: Vec<u32> = (0..256).map(|i| if i < 16 { (i as u32) + 1 } else { 0 }).collect();
    let ct_st = HUF_SYMBOLVALUE_MAX as usize + 2 + 8;
    let ctws_sz = ((4 * 256) + 192) * 4;

    // row 294: HUF_buildCTable_wksp maxSymbolValue > HUF_SYMBOLVALUE_MAX.
    {
        let label = "ERRORS row 294: HUF_buildCTable_wksp maxSymbolValue too large";
        let mut ct = vec![0usize; ct_st + 16];
        let mut cws = vec![0u8; ctws_sz];
        let mut rws = vec![0u8; ctws_sz];
        let cr = unsafe { (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), HUF_SYMBOLVALUE_MAX + 1, HUF_TABLELOG_MAX, cws.as_mut_ptr() as *mut c_void, ctws_sz) };
        let rr = unsafe { (r_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), HUF_SYMBOLVALUE_MAX + 1, HUF_TABLELOG_MAX, rws.as_mut_ptr() as *mut c_void, ctws_sz) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 294: HUF_buildCTable_wksp undersized workspace -> workSpace_tooSmall.
    {
        let label = "ERRORS row 294: HUF_buildCTable_wksp undersized workspace";
        let mut ct = vec![0usize; ct_st + 16];
        let mut cws = vec![0u8; 8];
        let mut rws = vec![0u8; 8];
        let cr = unsafe { (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX, cws.as_mut_ptr() as *mut c_void, 8) };
        let rr = unsafe { (r_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX, rws.as_mut_ptr() as *mut c_void, 8) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 299: tableLog > HUF_TABLELOG_MAX (buildCTable clamps/handles). Compare.
    {
        let label = "ERRORS row 299: HUF_buildCTable_wksp tableLog > HUF_TABLELOG_MAX";
        let mut ct = vec![0usize; ct_st + 16];
        let mut cws = vec![0u8; ctws_sz];
        let mut rws = vec![0u8; ctws_sz];
        let cr = unsafe { (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX + 5, cws.as_mut_ptr() as *mut c_void, ctws_sz) };
        let rr = unsafe { (r_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX + 5, rws.as_mut_ptr() as *mut c_void, ctws_sz) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }

    // Build a real CTable to exercise HUF_writeCTable_wksp error paths.
    let (hdr_src, _ct_st, msv, max_bits) = build_valid_huf_header();
    let _ = hdr_src;
    // Re-derive the CTable via buildCTable for writeCTable input.
    let mut ct = vec![0usize; ct_st];
    unsafe {
        let mut cws = vec![0u8; ctws_sz];
        let r = (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX, cws.as_mut_ptr() as *mut c_void, ctws_sz);
        assert!((c_ie)(r) == 0, "setup ct for writeCTable");
    }
    let real_mb = unsafe {
        let mut cws = vec![0u8; ctws_sz];
        (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), 15, HUF_TABLELOG_MAX, cws.as_mut_ptr() as *mut c_void, ctws_sz) as u32
    };
    let wsz = (8 << 10) + 512;

    // row 291: HUF_writeCTable_wksp dstCapacity too small.
    for cap in [0usize, 1, 2] {
        let label = format!("ERRORS row 291: HUF_writeCTable_wksp dstCapacity={cap}");
        let mut cbuf = vec![FILL; cap.max(1)];
        let mut rbuf = vec![FILL; cap.max(1)];
        let mut cws = vec![0u8; wsz];
        let mut rws = vec![0u8; wsz];
        let cr = unsafe { (c_wct)(cbuf.as_mut_ptr() as *mut c_void, cap, ct.as_ptr() as *const c_void, 15, real_mb, cws.as_mut_ptr() as *mut c_void, wsz) };
        let rr = unsafe { (r_wct)(rbuf.as_mut_ptr() as *mut c_void, cap, ct.as_ptr() as *const c_void, 15, real_mb, rws.as_mut_ptr() as *mut c_void, wsz) };
        assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_bytes_eq(&format!("{label} buf"), &cbuf, &rbuf);
    }
    // row 290: HUF_writeCTable_wksp maxSymbolValue > HUF_SYMBOLVALUE_MAX.
    {
        let label = "ERRORS row 290: HUF_writeCTable_wksp maxSymbolValue too large";
        let mut cbuf = vec![FILL; 512];
        let mut rbuf = vec![FILL; 512];
        let mut cws = vec![0u8; wsz];
        let mut rws = vec![0u8; wsz];
        let cr = unsafe { (c_wct)(cbuf.as_mut_ptr() as *mut c_void, 512, ct.as_ptr() as *const c_void, HUF_SYMBOLVALUE_MAX + 1, real_mb, cws.as_mut_ptr() as *mut c_void, wsz) };
        let rr = unsafe { (r_wct)(rbuf.as_mut_ptr() as *mut c_void, 512, ct.as_ptr() as *const c_void, HUF_SYMBOLVALUE_MAX + 1, real_mb, rws.as_mut_ptr() as *mut c_void, wsz) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_bytes_eq(&format!("{label} buf"), &cbuf, &rbuf);
    }
    // row 289: HUF_writeCTable_wksp workspace too small -> GENERIC.
    {
        let label = "ERRORS row 289: HUF_writeCTable_wksp undersized workspace";
        let mut cbuf = vec![FILL; 512];
        let mut rbuf = vec![FILL; 512];
        let mut cws = vec![0u8; 8];
        let mut rws = vec![0u8; 8];
        let cr = unsafe { (c_wct)(cbuf.as_mut_ptr() as *mut c_void, 512, ct.as_ptr() as *const c_void, 15, real_mb, cws.as_mut_ptr() as *mut c_void, 8) };
        let rr = unsafe { (r_wct)(rbuf.as_mut_ptr() as *mut c_void, 512, ct.as_ptr() as *const c_void, 15, real_mb, rws.as_mut_ptr() as *mut c_void, 8) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    let _ = (msv, max_bits);
}

// -------------------------------------------------- byte-view helpers ------
fn bytes_of_u32(s: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}

// =========================================================================
//  HUF compress (usingCTable / repeat), readDTable, decompress error paths.
//  ERRORS.md rows: 296,297,298,300,301,302,303,304,305,306,307
//  (HUF_compress4X_wksp/compress1X_wksp are NOT exported; the 4X-block-size and
//   workspace checks of rows 297/298 are reached via HUF_compress4X_repeat,
//   which forwards to HUF_compress4X_wksp internally — noted inline.)
// =========================================================================

#[test]
fn huf_compress_errors() {
    let (c_c1u, r_c1u) = fnpair!("HUF_compress1X_usingCTable", FnHufCompressUsingCTable);
    let (c_c4u, r_c4u) = fnpair!("HUF_compress4X_usingCTable", FnHufCompressUsingCTable);
    let (c_c1r, r_c1r) = fnpair!("HUF_compress1X_repeat", FnHufCompressRepeat);
    let (c_c4r, r_c4r) = fnpair!("HUF_compress4X_repeat", FnHufCompressRepeat);
    let (c_bct, _r) = fnpair!("HUF_buildCTable_wksp", FnHufBuildCTableWksp);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);

    // Build a valid CTable over the source alphabet.
    let ct_st = HUF_SYMBOLVALUE_MAX as usize + 2 + 8;
    let ctws_sz = ((4 * 256) + 192) * 4;
    let src: Vec<u8> = (0..8192u32).map(|i| ((i * 3) & 0x3F) as u8).collect();
    type FnHist = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
    let (c_hist, _r2) = fnpair!("HIST_count", FnHist);
    let mut count = vec![0u32; 256];
    let mut msv = 255u32;
    unsafe { let _ = (c_hist)(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len()); }
    let mut ct = vec![0usize; ct_st];
    let table_log = HUF_TABLELOG_MAX;
    unsafe {
        let mut ws = vec![0u8; ctws_sz];
        let r = (c_bct)(ct.as_mut_ptr() as *mut c_void, count.as_ptr(), msv, table_log, ws.as_mut_ptr() as *mut c_void, ctws_sz);
        assert!((c_ie)(r) == 0, "setup buildCTable");
    }

    // row 296: HUF_compress{1,4}X_usingCTable dstCapacity too small (0/1/small).
    for (which, cf, rf) in [(1u8, &c_c1u, &r_c1u), (4u8, &c_c4u, &r_c4u)] {
        for cap in [0usize, 1, 4, 8] {
            let label = format!("ERRORS row 296: HUF_compress{which}X_usingCTable dstCapacity={cap}");
            let mut cbuf = vec![FILL; cap.max(1)];
            let mut rbuf = vec![FILL; cap.max(1)];
            let cr = unsafe { (cf)(cbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct.as_ptr() as *const c_void, 0) };
            let rr = unsafe { (rf)(rbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct.as_ptr() as *const c_void, 0) };
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_bytes_eq(&format!("{label} buf"), &cbuf, &rbuf);
        }
        // srcSize 0: valid (produces 0 == not compressible) — compare exactly.
        {
            let label = format!("ERRORS row 296: HUF_compress{which}X_usingCTable srcSize=0");
            let mut cbuf = vec![FILL; 256];
            let mut rbuf = vec![FILL; 256];
            let cr = unsafe { (cf)(cbuf.as_mut_ptr() as *mut c_void, 256, src.as_ptr() as *const c_void, 0, ct.as_ptr() as *const c_void, 0) };
            let rr = unsafe { (rf)(rbuf.as_mut_ptr() as *mut c_void, 256, src.as_ptr() as *const c_void, 0, ct.as_ptr() as *const c_void, 0) };
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_bytes_eq(&format!("{label} buf"), &cbuf, &rbuf);
        }
    }

    // rows 297/298: HUF_compress4X_repeat forwards to compress4X_wksp:
    //   srcSize > HUF_BLOCKSIZE_MAX (128KB) -> srcSize_wrong (row 298);
    //   undersized workspace -> workSpace_tooSmall (row 297).
    let wsz = (8 << 10) + 512;
    {
        // row 298: srcSize > HUF_BLOCKSIZE_MAX
        let big: Vec<u8> = (0..(HUF_BLOCKSIZE_MAX + 1)).map(|i| (i & 0x3F) as u8).collect();
        let label = "ERRORS row 298: HUF_compress4X_repeat srcSize > HUF_BLOCKSIZE_MAX";
        let mut cbuf = vec![FILL; HUF_BLOCKSIZE_MAX + 64];
        let mut rbuf = vec![FILL; HUF_BLOCKSIZE_MAX + 64];
        let mut c_ht = vec![0usize; ct_st];
        let mut r_ht = vec![0usize; ct_st];
        let mut c_hw = vec![0u8; wsz];
        let mut r_hw = vec![0u8; wsz];
        let (mut c_rep, mut r_rep) = (0i32, 0i32);
        let cr = unsafe { (c_c4r)(cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), big.as_ptr() as *const c_void, big.len(), msv, table_log, c_hw.as_mut_ptr() as *mut c_void, wsz, c_ht.as_mut_ptr() as *mut c_void, &mut c_rep, 0) };
        let rr = unsafe { (r_c4r)(rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), big.as_ptr() as *const c_void, big.len(), msv, table_log, r_hw.as_mut_ptr() as *mut c_void, wsz, r_ht.as_mut_ptr() as *mut c_void, &mut r_rep, 0) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    {
        // row 297: undersized workspace
        let label = "ERRORS row 297: HUF_compress4X_repeat undersized workspace";
        let mut cbuf = vec![FILL; 4096];
        let mut rbuf = vec![FILL; 4096];
        let mut c_ht = vec![0usize; ct_st];
        let mut r_ht = vec![0usize; ct_st];
        let mut c_hw = vec![0u8; 16];
        let mut r_hw = vec![0u8; 16];
        let (mut c_rep, mut r_rep) = (0i32, 0i32);
        let cr = unsafe { (c_c4r)(cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), src.as_ptr() as *const c_void, src.len(), msv, table_log, c_hw.as_mut_ptr() as *mut c_void, 16, c_ht.as_mut_ptr() as *mut c_void, &mut c_rep, 0) };
        let rr = unsafe { (r_c4r)(rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), src.as_ptr() as *const c_void, src.len(), msv, table_log, r_hw.as_mut_ptr() as *mut c_void, 16, r_ht.as_mut_ptr() as *mut c_void, &mut r_rep, 0) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
    }
    // row 296/297: 1X_repeat dstCapacity too small
    {
        let label = "ERRORS row 296: HUF_compress1X_repeat dstCapacity too small";
        let mut cbuf = vec![FILL; 2];
        let mut rbuf = vec![FILL; 2];
        let mut c_ht = vec![0usize; ct_st];
        let mut r_ht = vec![0usize; ct_st];
        let mut c_hw = vec![0u8; wsz];
        let mut r_hw = vec![0u8; wsz];
        let (mut c_rep, mut r_rep) = (0i32, 0i32);
        let cr = unsafe { (c_c1r)(cbuf.as_mut_ptr() as *mut c_void, 2, src.as_ptr() as *const c_void, src.len(), msv, table_log, c_hw.as_mut_ptr() as *mut c_void, wsz, c_ht.as_mut_ptr() as *mut c_void, &mut c_rep, 0) };
        let rr = unsafe { (r_c1r)(rbuf.as_mut_ptr() as *mut c_void, 2, src.as_ptr() as *const c_void, src.len(), msv, table_log, r_hw.as_mut_ptr() as *mut c_void, wsz, r_ht.as_mut_ptr() as *mut c_void, &mut r_rep, 0) };
        assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        assert_eq!(c_rep, r_rep, "{label}: repeat state");
    }
}

#[test]
fn huf_read_dtable_and_decompress_errors() {
    let (c_rd1, r_rd1) = fnpair!("HUF_readDTableX1_wksp", FnHufReadDTableWksp);
    let (c_rd2, r_rd2) = fnpair!("HUF_readDTableX2_wksp", FnHufReadDTableWksp);
    let (c_dc1, r_dc1) = fnpair!("HUF_decompress1X1_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_dc12, r_dc12) = fnpair!("HUF_decompress1X2_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_dc1x, r_dc1x) = fnpair!("HUF_decompress1X_DCtx_wksp", FnHufDecompressDCtxWksp);
    let (c_d4h, r_d4h) = fnpair!("HUF_decompress4X_hufOnly_wksp", FnHufDecompressDCtxWksp);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);

    let (hdr, _ct, _msv, _mb) = build_valid_huf_header();
    let wksp = (2 << 10) + (1 << 9);
    let dt_u32 = 1 + (1usize << HUF_TABLELOG_MAX) + 64;

    // row 303: HUF_readDTableX* undersized workspace.
    for (dv, cf, rf) in [(1u8, &c_rd1, &r_rd1), (2u8, &c_rd2, &r_rd2)] {
        let label = format!("ERRORS row 303: HUF_readDTableX{dv}_wksp undersized workspace");
        let mut c_dt = vec![0u32; dt_u32];
        let mut r_dt = vec![0u32; dt_u32];
        let mut cws = vec![0u8; 8];
        let mut rws = vec![0u8; 8];
        let cr = unsafe { (cf)(c_dt.as_mut_ptr() as *mut c_void, hdr.as_ptr() as *const c_void, hdr.len(), cws.as_mut_ptr() as *mut c_void, 8, 0) };
        let rr = unsafe { (rf)(r_dt.as_mut_ptr() as *mut c_void, hdr.as_ptr() as *const c_void, hdr.len(), rws.as_mut_ptr() as *mut c_void, 8, 0) };
        assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);

        // rows 303/304: corrupt/truncated input to readDTable.
        for prefix in [0usize, 1, 2, 3, 5, hdr.len().min(10)] {
            let buf = &hdr[..prefix.min(hdr.len())];
            let l2 = format!("ERRORS rows 303/304: HUF_readDTableX{dv}_wksp truncated len={}", buf.len());
            let mut c_dt = vec![0u32; dt_u32];
            let mut r_dt = vec![0u32; dt_u32];
            let mut cws = vec![0u8; wksp];
            let mut rws = vec![0u8; wksp];
            let cr = unsafe { (cf)(c_dt.as_mut_ptr() as *mut c_void, buf.as_ptr() as *const c_void, buf.len(), cws.as_mut_ptr() as *mut c_void, wksp, 0) };
            let rr = unsafe { (rf)(r_dt.as_mut_ptr() as *mut c_void, buf.as_ptr() as *const c_void, buf.len(), rws.as_mut_ptr() as *mut c_void, wksp, 0) };
            assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
        }
    }

    // rows 300-307: decompress DCtx_wksp variants on empty/corrupt/inconsistent input.
    let mut rng = Rng::new(0x3000_0001);
    let decoders: [(&str, &FnHufDecompressDCtxWksp, &FnHufDecompressDCtxWksp); 4] = [
        ("HUF_decompress1X1_DCtx_wksp", &c_dc1, &r_dc1),
        ("HUF_decompress1X2_DCtx_wksp", &c_dc12, &r_dc12),
        ("HUF_decompress1X_DCtx_wksp", &c_dc1x, &r_dc1x),
        ("HUF_decompress4X_hufOnly_wksp", &c_d4h, &r_d4h),
    ];
    for (name, cf, rf) in decoders.iter() {
        // empty / tiny / corrupt cSrc
        for (tag, cbuf) in [
            ("empty", Vec::<u8>::new()),
            ("1-byte", vec![0x11u8]),
            ("tiny", vec![0x11u8, 0x22, 0x33]),
            ("corrupt", vec![0xDEu8, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]),
        ] {
            for &dstsz in &[0usize, 1, 6, 64] {
                let label = format!("ERRORS rows 300-307: {name} {tag} dstSize={dstsz}");
                let mut c_dt = vec![0u32; dt_u32];
                let mut r_dt = vec![0u32; dt_u32];
                let mut cout = vec![FILL; dstsz.max(1)];
                let mut rout = vec![FILL; dstsz.max(1)];
                let mut cws = vec![0u8; wksp];
                let mut rws = vec![0u8; wksp];
                let cr = unsafe { (cf)(c_dt.as_mut_ptr() as *mut c_void, cout.as_mut_ptr() as *mut c_void, dstsz, cbuf.as_ptr() as *const c_void, cbuf.len(), cws.as_mut_ptr() as *mut c_void, wksp, 0) };
                let rr = unsafe { (rf)(r_dt.as_mut_ptr() as *mut c_void, rout.as_mut_ptr() as *mut c_void, dstsz, cbuf.as_ptr() as *const c_void, cbuf.len(), rws.as_mut_ptr() as *mut c_void, wksp, 0) };
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                assert_bytes_eq(&format!("{label} out"), &cout, &rout);
            }
        }
        // random payloads at many lengths (rows 300-307)
        for len in [4usize, 8, 10, 16, 32, 64, 128] {
            for _ in 0..8 {
                let cbuf: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
                let dstsz = 64usize;
                let label = format!("ERRORS rows 300-307: {name} random cSrc len={len}");
                let mut c_dt = vec![0u32; dt_u32];
                let mut r_dt = vec![0u32; dt_u32];
                let mut cout = vec![FILL; dstsz];
                let mut rout = vec![FILL; dstsz];
                let mut cws = vec![0u8; wksp];
                let mut rws = vec![0u8; wksp];
                let cr = unsafe { (cf)(c_dt.as_mut_ptr() as *mut c_void, cout.as_mut_ptr() as *mut c_void, dstsz, cbuf.as_ptr() as *const c_void, cbuf.len(), cws.as_mut_ptr() as *mut c_void, wksp, 0) };
                let rr = unsafe { (rf)(r_dt.as_mut_ptr() as *mut c_void, rout.as_mut_ptr() as *mut c_void, dstsz, cbuf.as_ptr() as *const c_void, cbuf.len(), rws.as_mut_ptr() as *mut c_void, wksp, 0) };
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                assert_bytes_eq(&format!("{label} out"), &cout, &rout);
            }
        }
    }
}

#[test]
fn huf_validate_and_getnbbits() {
    // row 300 family: HUF_validateCTable on invalid CTable; HUF_getNbBitsFromCTable
    // for symbols outside the table. These are total functions (no error code);
    // we compare the raw c_int / c_uint returns exactly.
    let (c_val, r_val) = fnpair!("HUF_validateCTable", FnHufValidateCTable);
    let (c_nbb, r_nbb) = fnpair!("HUF_getNbBitsFromCTable", FnHufGetNbBits);

    let ct_st = HUF_SYMBOLVALUE_MAX as usize + 2 + 8;
    // A deliberately garbage CTable (all 0xAA bytes) plus a count array.
    let mut rng = Rng::new(0x3001_0001);
    for _ in 0..64 {
        let ct: Vec<usize> = (0..ct_st).map(|_| rng.next_u64() as usize).collect();
        let count: Vec<u32> = (0..256).map(|_| (rng.next_u32() % 500)).collect();
        let msv = (rng.next_u32() % 256) as u32;
        let label = format!("ERRORS row 300: HUF_validateCTable garbage table msv={msv}");
        let cr = unsafe { (c_val)(ct.as_ptr() as *const c_void, count.as_ptr(), msv) };
        let rr = unsafe { (r_val)(ct.as_ptr() as *const c_void, count.as_ptr(), msv) };
        assert_eq!(cr, rr, "{label}");

        for sym in [0u32, 1, 100, 255] {
            let l2 = format!("ERRORS row 300: HUF_getNbBitsFromCTable sym={sym}");
            let cr = unsafe { (c_nbb)(ct.as_ptr() as *const c_void, sym) };
            let rr = unsafe { (r_nbb)(ct.as_ptr() as *const c_void, sym) };
            assert_eq!(cr, rr, "{l2}");
        }
    }
}

// =========================================================================
//  Deprecated ZBUFF API error paths (forwarded to ZSTD_*).
//  ERRORS.md rows: 308,309,310,311,312,313
//  Compares every returned size and every in/out position on both sides.
// =========================================================================

#[test]
fn zbuff_error_paths() {
    let (c_ccreate, r_ccreate) = fnpair!("ZBUFF_createCCtx", FnZbuffPtr);
    let (c_cfree, r_cfree) = fnpair!("ZBUFF_freeCCtx", FnZbuffFree);
    let (c_cinit, r_cinit) = fnpair!("ZBUFF_compressInit", FnZbuffCInit);
    let (c_ccont, r_ccont) = fnpair!("ZBUFF_compressContinue", FnZbuffCContinue);
    let (c_cflush, r_cflush) = fnpair!("ZBUFF_compressFlush", FnZbuffCFlush);
    let (c_cend, r_cend) = fnpair!("ZBUFF_compressEnd", FnZbuffCFlush);
    let (c_dcreate, r_dcreate) = fnpair!("ZBUFF_createDCtx", FnZbuffPtr);
    let (c_dfree, r_dfree) = fnpair!("ZBUFF_freeDCtx", FnZbuffFree);
    let (c_dinit, r_dinit) = fnpair!("ZBUFF_decompressInit", FnZbuffDInit);
    let (c_dcont, r_dcont) = fnpair!("ZBUFF_decompressContinue", FnZbuffDContinue);
    let (c_ie, r_ie) = fnpair!("ZBUFF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("ZBUFF_getErrorName", FnErrName);

    unsafe {
        // row 309: ZBUFF_compressInit with an out-of-range compression level.
        // (ZSTD clamps most levels; extreme values are compared exactly.)
        for lvl in [i32::MIN, -1000, -1, 0, 1, 23, 100, 1000, i32::MAX] {
            let cc = (c_ccreate)();
            let rc = (r_ccreate)();
            assert!(!cc.is_null() && !rc.is_null(), "ZBUFF createCCtx");
            let label = format!("ERRORS row 309: ZBUFF_compressInit level={lvl}");
            let cr = (c_cinit)(cc, lvl);
            let rr = (r_cinit)(rc, lvl);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_cfree)(cc);
            let _ = (r_cfree)(rc);
        }

        // rows 308/311: compressContinue / compressFlush / compressEnd BEFORE
        // compressInit. Forwards ZSTD_compressStream2 with init missing.
        for stage in ["continue", "flush", "end"] {
            let cc = (c_ccreate)();
            let rc = (r_ccreate)();
            let mut cdst = vec![FILL; 128];
            let mut rdst = vec![FILL; 128];
            let src = [1u8, 2, 3, 4, 5, 6, 7, 8];
            let label = format!("ERRORS rows 308/311: ZBUFF_compress_{stage} before init");
            let (cr, rr, cdo, rdo, cso, rso);
            match stage {
                "continue" => {
                    let (mut cdc, mut rdc) = (128usize, 128usize);
                    let (mut csc, mut rsc) = (src.len(), src.len());
                    cr = (c_ccont)(cc, cdst.as_mut_ptr() as *mut c_void, &mut cdc, src.as_ptr() as *const c_void, &mut csc);
                    rr = (r_ccont)(rc, rdst.as_mut_ptr() as *mut c_void, &mut rdc, src.as_ptr() as *const c_void, &mut rsc);
                    cdo = cdc; rdo = rdc; cso = csc; rso = rsc;
                }
                "flush" => {
                    let (mut cdc, mut rdc) = (128usize, 128usize);
                    cr = (c_cflush)(cc, cdst.as_mut_ptr() as *mut c_void, &mut cdc);
                    rr = (r_cflush)(rc, rdst.as_mut_ptr() as *mut c_void, &mut rdc);
                    cdo = cdc; rdo = rdc; cso = 0; rso = 0;
                }
                _ => {
                    let (mut cdc, mut rdc) = (128usize, 128usize);
                    cr = (c_cend)(cc, cdst.as_mut_ptr() as *mut c_void, &mut cdc);
                    rr = (r_cend)(rc, rdst.as_mut_ptr() as *mut c_void, &mut rdc);
                    cdo = cdc; rdo = rdc; cso = 0; rso = 0;
                }
            }
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((cdo, cso), (rdo, rso), "{label}: in/out positions");
            assert_bytes_eq(&format!("{label} dst"), &cdst, &rdst);
            let _ = (c_cfree)(cc); let _ = (r_cfree)(rc);
        }

        // row 311: compressContinue with a zero-size output buffer (dstCapacity=0).
        {
            let cc = (c_ccreate)();
            let rc = (r_ccreate)();
            assert_same("setup init", (c_cinit)(cc, 3), (r_cinit)(rc, 3), &c_ie, &r_ie, &c_en, &r_en);
            let src = [9u8; 64];
            let (mut cdc, mut rdc) = (0usize, 0usize);
            let (mut csc, mut rsc) = (src.len(), src.len());
            let label = "ERRORS row 311: ZBUFF_compressContinue zero-size output";
            let cr = (c_ccont)(cc, std::ptr::null_mut(), &mut cdc, src.as_ptr() as *const c_void, &mut csc);
            let rr = (r_ccont)(rc, std::ptr::null_mut(), &mut rdc, src.as_ptr() as *const c_void, &mut rsc);
            assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((cdc, csc), (rdc, rsc), "{label}: positions");
            let _ = (c_cfree)(cc); let _ = (r_cfree)(rc);
        }

        // row 313: decompressContinue BEFORE decompressInit.
        {
            let cd = (c_dcreate)();
            let rd = (r_dcreate)();
            assert!(!cd.is_null() && !rd.is_null(), "ZBUFF createDCtx");
            let src = [0xFDu8, 0x2F, 0xB5, 0x28, 0x00, 0x00];
            let mut cdst = vec![FILL; 64];
            let mut rdst = vec![FILL; 64];
            let (mut cdc, mut rdc) = (64usize, 64usize);
            let (mut csc, mut rsc) = (src.len(), src.len());
            // Note: decompressContinue auto-inits on first call in this impl; still
            // compare exact behaviour. Feed a modern-frame prefix (truncated).
            let label = "ERRORS row 313: ZBUFF_decompressContinue truncated frame";
            let cr = (c_dcont)(cd, cdst.as_mut_ptr() as *mut c_void, &mut cdc, src.as_ptr() as *const c_void, &mut csc);
            let rr = (r_dcont)(rd, rdst.as_mut_ptr() as *mut c_void, &mut rdc, src.as_ptr() as *const c_void, &mut rsc);
            assert_same(label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((cdc, csc), (rdc, rsc), "{label}: positions");
            let _ = (c_dfree)(cd); let _ = (r_dfree)(rd);
        }

        // rows 312/313: decompressInit then feed corrupt / truncated / random
        // input to the decompressor; compare returns and positions.
        let mut rng = Rng::new(0x3120_0001);
        for len in [0usize, 1, 4, 6, 8, 16, 32, 64] {
            for _ in 0..8 {
                let src: Vec<u8> = (0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
                let cd = (c_dcreate)();
                let rd = (r_dcreate)();
                assert_same("row 312: decompressInit", (c_dinit)(cd), (r_dinit)(rd), &c_ie, &r_ie, &c_en, &r_en);
                let mut cdst = vec![FILL; 128];
                let mut rdst = vec![FILL; 128];
                let (mut cdc, mut rdc) = (128usize, 128usize);
                let (mut csc, mut rsc) = (len, len);
                let label = format!("ERRORS row 313: ZBUFF_decompressContinue random len={len}");
                let cr = (c_dcont)(cd, cdst.as_mut_ptr() as *mut c_void, &mut cdc, src.as_ptr() as *const c_void, &mut csc);
                let rr = (r_dcont)(rd, rdst.as_mut_ptr() as *mut c_void, &mut rdc, src.as_ptr() as *const c_void, &mut rsc);
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                assert_eq!((cdc, csc), (rdc, rsc), "{label}: positions");
                assert_bytes_eq(&format!("{label} dst"), &cdst, &rdst);
                let _ = (c_dfree)(cd); let _ = (r_dfree)(rd);
            }
        }
        // row 310: ZBUFF_compressInit is exercised above; dict-load failure (row
        // 310) forwards ZSTD_CCtx_loadDictionary and is covered by the ZSTD dict
        // error rows elsewhere in the suite — noted here for row check-off.
    }
}

// =========================================================================
//  Legacy v01–v07 decoders.
//  ERRORS.md rows: 314,315,316,317
//  Drives every exported ZSTDv0N_decompress on: empty, 1..8 byte inputs, each
//  legacy magic + random bytes, truncations, valid modern frames, and random
//  buffers of many lengths. Asserts identical returns and identical
//  ZSTDv0X_isError. Also drives ZSTD_decompress / ZSTD_getFrameContentSize on
//  buffers carrying each legacy magic 0xFD2FB51E..0xFD2FB527 + garbage.
// =========================================================================

// Legacy magic numbers (big/little endian as stored by each version).
const LEGACY_MAGICS: [u32; 10] = [
    0xFD2FB51E, 0xFD2FB51F, 0xFD2FB520, 0xFD2FB521, 0xFD2FB522, 0xFD2FB523,
    0xFD2FB524, 0xFD2FB525, 0xFD2FB526, 0xFD2FB527,
];
const ZSTD_MODERN_MAGIC: u32 = 0xFD2FB528;

fn le_bytes(magic: u32) -> [u8; 4] {
    magic.to_le_bytes()
}

#[test]
fn legacy_decoder_error_paths() {
    // Each exported (decompress, isError) pair for v01..v07.
    let versions: [(&str, Option<&str>); 7] = [
        ("ZSTDv01_decompress", Some("ZSTDv01_isError")),
        ("ZSTDv02_decompress", Some("ZSTDv02_isError")),
        ("ZSTDv03_decompress", Some("ZSTDv03_isError")),
        // NOTE: ZSTDv04_isError is NOT exported in this build; v04 returns codes in
        // the shared ZSTD error space, so we classify with ZSTD_isError instead
        // (substitution noted per task instructions).
        ("ZSTDv04_decompress", None),
        ("ZSTDv05_decompress", Some("ZSTDv05_isError")),
        ("ZSTDv06_decompress", Some("ZSTDv06_isError")),
        ("ZSTDv07_decompress", Some("ZSTDv07_isError")),
    ];

    let mut rng = Rng::new(0x3140_0001);

    for (decname, iename) in versions {
        let (c_dec, r_dec) = pair::<FnLegacyDecompress>(decname);
        let ie_name = iename.unwrap_or("ZSTD_isError");
        let (c_ie, r_ie) = pair::<FnLegacyIsError>(ie_name);
        let (c_dec, r_dec) = (*c_dec, *r_dec);
        let (c_ie, r_ie) = (*c_ie, *r_ie);

        // Build the input set: empty, 1..8 bytes, each legacy magic + random,
        // truncations, valid modern-frame magic + garbage, random buffers.
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        inputs.push(Vec::new());
        for n in 1..=8usize {
            inputs.push((0..n).map(|_| (rng.next_u32() & 0xFF) as u8).collect());
        }
        for &magic in LEGACY_MAGICS.iter().chain(std::iter::once(&ZSTD_MODERN_MAGIC)) {
            let mb = le_bytes(magic);
            for tail in [0usize, 1, 3, 4, 8, 16, 32, 64] {
                let mut v = mb.to_vec();
                v.extend((0..tail).map(|_| (rng.next_u32() & 0xFF) as u8));
                inputs.push(v.clone());
                // truncations of that buffer
                for cut in [1usize, 2, 3] {
                    if v.len() > cut {
                        inputs.push(v[..v.len() - cut].to_vec());
                    }
                }
            }
        }
        for len in [0usize, 1, 2, 5, 9, 16, 33, 64, 128, 200] {
            inputs.push((0..len).map(|_| (rng.next_u32() & 0xFF) as u8).collect());
        }

        for (i, inp) in inputs.iter().enumerate() {
            // row 316/317: drive vN_decompress; identical returns + isError.
            let label = format!("ERRORS rows 314-317: {decname} input#{i} (len={})", inp.len());
            let mut cdst = vec![FILL; 512];
            let mut rdst = vec![FILL; 512];
            let cr = unsafe { (c_dec)(cdst.as_mut_ptr() as *mut c_void, 512, inp.as_ptr() as *const c_void, inp.len()) };
            let rr = unsafe { (r_dec)(rdst.as_mut_ptr() as *mut c_void, 512, inp.as_ptr() as *const c_void, inp.len()) };
            unsafe {
                let ce = (c_ie)(cr) != 0;
                let re = (r_ie)(rr) != 0;
                assert_eq!(ce, re, "{label}: {ie_name} mismatch C_ret={cr}(err={ce}) R_ret={rr}(err={re})");
            }
            assert_eq!(cr, rr, "{label}: raw return mismatch C={cr} R={rr}");
            // On success the produced bytes must match exactly.
            unsafe {
                if (c_ie)(cr) == 0 {
                    assert_bytes_eq(&format!("{label} dst"), &cdst, &rdst);
                }
            }
        }
    }
}

#[test]
fn legacy_magic_via_public_api() {
    // rows 314/315/316: ZSTD_decompress and ZSTD_getFrameContentSize on buffers
    // carrying each legacy magic 0xFD2FB51E..0xFD2FB527 followed by garbage.
    let (c_dec, r_dec) = fnpair!("ZSTD_decompress", FnDecompress);
    let (c_gfcs, r_gfcs) = fnpair!("ZSTD_getFrameContentSize", FnGetFrameContentSize);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);
    let (c_en, r_en) = fnpair!("ZSTD_getErrorName", FnErrName);

    let mut rng = Rng::new(0x3150_0001);
    for &magic in LEGACY_MAGICS.iter() {
        for tail in [0usize, 4, 8, 16, 32, 64, 128] {
            let mut v = le_bytes(magic).to_vec();
            v.extend((0..tail).map(|_| (rng.next_u32() & 0xFF) as u8));
            let label = format!("ERRORS rows 314-316: ZSTD_decompress legacy magic {magic:#010x} tail={tail}");
            let mut cdst = vec![FILL; 1024];
            let mut rdst = vec![FILL; 1024];
            let cr = unsafe { (c_dec)(cdst.as_mut_ptr() as *mut c_void, 1024, v.as_ptr() as *const c_void, v.len()) };
            let rr = unsafe { (r_dec)(rdst.as_mut_ptr() as *mut c_void, 1024, v.as_ptr() as *const c_void, v.len()) };
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            unsafe {
                if (c_ie)(cr) == 0 {
                    assert_bytes_eq(&format!("{label} dst"), &cdst, &rdst);
                }
            }
            // getFrameContentSize returns u64 sentinels; compare exactly.
            let l2 = format!("ERRORS rows 314-316: ZSTD_getFrameContentSize legacy magic {magic:#010x} tail={tail}");
            let cs = unsafe { (c_gfcs)(v.as_ptr() as *const c_void, v.len()) };
            let rs = unsafe { (r_gfcs)(v.as_ptr() as *const c_void, v.len()) };
            assert_eq!(cs, rs, "{l2}: content size mismatch C={cs:#x} R={rs:#x}");
        }
    }
}

// =========================================================================
//  Enum / out-of-range values crossing FFI.
//  ERRORS.md rows: 318,319,320,321,322,323,324,325,326,327,328
//  Asserts the C's ACTUAL behaviour per ERRORS.md — several are SILENT NO-OPs
//  (return 0, no error), not errors.
// =========================================================================

// Every valid ZSTD_ErrorCode ordinal (from zstd_errors.h).
const ERROR_CODES: [c_int; 30] = [
    0, 1, 10, 12, 14, 16, 20, 22, 24, 30, 32, 34, 40, 41, 42, 44, 46, 48, 49, 50,
    60, 62, 64, 66, 70, 72, 74, 80, 82, 120,
];

#[test]
fn enum_out_of_range_params_and_reset() {
    let (c_ccnew, r_ccnew) = fnpair!("ZSTD_createCCtx", FnVoidPtr);
    let (c_ccfree, r_ccfree) = fnpair!("ZSTD_freeCCtx", FnFreeCtx);
    let (c_dcnew, r_dcnew) = fnpair!("ZSTD_createDCtx", FnVoidPtr);
    let (c_dcfree, r_dcfree) = fnpair!("ZSTD_freeDCtx", FnFreeCtx);
    let (c_csp, r_csp) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let (c_dsp, r_dsp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
    let (c_crst, r_crst) = fnpair!("ZSTD_CCtx_reset", FnCtxReset);
    let (c_drst, r_drst) = fnpair!("ZSTD_DCtx_reset", FnCtxReset);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);
    let (c_en, r_en) = fnpair!("ZSTD_getErrorName", FnErrName);

    unsafe {
        // row 318: ZSTD_CCtx_setParameter out-of-range parameter ids.
        // row 322: ZSTD_c_strategy=0 accepted(default), 10 out of bounds.
        // row 325: ZSTD_c_format=2 and -1 out of bounds.
        let bad_params = [-1, 0, 1, 99, 999, 1018, 100000, i32::MIN, i32::MAX];
        for &p in &bad_params {
            for &v in &[0, 1, -1, i32::MAX] {
                let cc = (c_ccnew)(); let rc = (r_ccnew)();
                let label = format!("ERRORS row 318: ZSTD_CCtx_setParameter param={p} value={v}");
                let cr = (c_csp)(cc, p, v);
                let rr = (r_csp)(rc, p, v);
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);
            }
        }
        // row 322: strategy specific values.
        for &v in &[0i32, 10, -1, i32::MAX] {
            let cc = (c_ccnew)(); let rc = (r_ccnew)();
            let label = format!("ERRORS row 322: ZSTD_c_strategy={v}");
            let cr = (c_csp)(cc, ZSTD_c_strategy, v);
            let rr = (r_csp)(rc, ZSTD_c_strategy, v);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);
        }
        // row 325: ZSTD_c_format = 2 and -1 (out of bounds), and ZSTD_d_format.
        for &v in &[2i32, -1, 0, 1] {
            let cc = (c_ccnew)(); let rc = (r_ccnew)();
            let label = format!("ERRORS row 325: ZSTD_c_format={v}");
            let cr = (c_csp)(cc, ZSTD_c_format, v);
            let rr = (r_csp)(rc, ZSTD_c_format, v);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);

            let cd = (c_dcnew)(); let rd = (r_dcnew)();
            let l2 = format!("ERRORS row 325: ZSTD_d_format={v}");
            let cr = (c_dsp)(cd, ZSTD_d_format, v);
            let rr = (r_dsp)(rd, ZSTD_d_format, v);
            assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_dcfree)(cd); let _ = (r_dcfree)(rd);
        }

        // row 319: ZSTD_DCtx_setParameter out-of-range parameter ids.
        for &p in &bad_params {
            for &v in &[0, 1, -1, i32::MAX] {
                let cd = (c_dcnew)(); let rd = (r_dcnew)();
                let label = format!("ERRORS row 319: ZSTD_DCtx_setParameter param={p} value={v}");
                let cr = (c_dsp)(cd, p, v);
                let rr = (r_dsp)(rd, p, v);
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                let _ = (c_dcfree)(cd); let _ = (r_dcfree)(rd);
            }
        }

        // row 320: ZSTD_CCtx_reset / ZSTD_DCtx_reset with directives 0,4,-1,i32::MAX.
        // ERRORS.md says the C silently returns 0 (no-op) for values that match no
        // branch. Verify + assert that exact behaviour on a fresh (init-stage) ctx.
        for &d in &[0i32, 4, -1, i32::MAX, 1, 2, 3] {
            let cc = (c_ccnew)(); let rc = (r_ccnew)();
            let label = format!("ERRORS row 320: ZSTD_CCtx_reset directive={d}");
            let cr = (c_crst)(cc, d);
            let rr = (r_crst)(rc, d);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);

            let cd = (c_dcnew)(); let rd = (r_dcnew)();
            let l2 = format!("ERRORS row 320: ZSTD_DCtx_reset directive={d}");
            let cr = (c_drst)(cd, d);
            let rr = (r_drst)(rd, d);
            assert_same(&l2, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            let _ = (c_dcfree)(cd); let _ = (r_dcfree)(rd);
        }
    }
}

#[test]
fn enum_compress_stream_endop_and_dict() {
    let (c_ccnew, r_ccnew) = fnpair!("ZSTD_createCCtx", FnVoidPtr);
    let (c_ccfree, r_ccfree) = fnpair!("ZSTD_freeCCtx", FnFreeCtx);
    let (c_cs2, r_cs2) = fnpair!("ZSTD_compressStream2", FnStream);
    let (c_ld, r_ld) = fnpair!("ZSTD_CCtx_loadDictionary_advanced", FnLoadDictAdv);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);
    let (c_en, r_en) = fnpair!("ZSTD_getErrorName", FnErrName);

    unsafe {
        // row 321: ZSTD_compressStream2 endOp = -1, 3, 4, i32::MAX -> outOfBound.
        let src = [1u8; 64];
        for &endop in &[-1i32, 3, 4, i32::MAX] {
            let cc = (c_ccnew)(); let rc = (r_ccnew)();
            let mut cdst = vec![FILL; 256];
            let mut rdst = vec![FILL; 256];
            let mut cout = ZSTD_outBuffer { dst: cdst.as_mut_ptr() as *mut c_void, size: 256, pos: 0 };
            let mut rout = ZSTD_outBuffer { dst: rdst.as_mut_ptr() as *mut c_void, size: 256, pos: 0 };
            let mut cin = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut rin = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let label = format!("ERRORS row 321: ZSTD_compressStream2 endOp={endop}");
            let cr = (c_cs2)(cc, &mut cout, &mut cin, endop);
            let rr = (r_cs2)(rc, &mut rout, &mut rin, endop);
            assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
            assert_eq!((cout.pos, cin.pos), (rout.pos, rin.pos), "{label}: positions");
            let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);
        }

        // rows 323/324: ZSTD_CCtx_loadDictionary_advanced with dictContentType=3
        // (out of 0..2) and dictLoadMethod=2 (out of 0..1; falls through to byCopy
        // with NO error per row 324). Feed non-dictionary data.
        let dict = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC];
        for &dlm in &[0i32, 1, 2] {
            for &dct in &[0i32, 1, 2, 3] {
                let cc = (c_ccnew)(); let rc = (r_ccnew)();
                let label = format!("ERRORS rows 323/324: loadDictionary_advanced dlm={dlm} dct={dct}");
                let cr = (c_ld)(cc, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                let rr = (r_ld)(rc, dict.as_ptr() as *const c_void, dict.len(), dlm, dct);
                assert_same(&label, cr, rr, &c_ie, &r_ie, &c_en, &r_en);
                let _ = (c_ccfree)(cc); let _ = (r_ccfree)(rc);
            }
        }
    }
}

#[test]
fn enum_error_string_and_code_roundtrip() {
    let (c_gen, r_gen) = fnpair!("ZSTD_getErrorName", FnErrName);
    let (c_gec, r_gec) = fnpair!("ZSTD_getErrorCode", FnGetErrorCode);
    let (c_ges, r_ges) = fnpair!("ZSTD_getErrorString", FnGetErrorString);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);

    unsafe {
        // rows 326/327: ZSTD_getErrorString for codes -1,0,1, every valid code,
        // 99, 120, 121, 1000, i32::MIN, i32::MAX — compare returned STRINGS.
        let mut codes: Vec<c_int> = vec![-1, 0, 1, 99, 120, 121, 1000, i32::MIN, i32::MAX];
        codes.extend_from_slice(&ERROR_CODES);
        for &code in &codes {
            let label = format!("ERRORS rows 326/327: ZSTD_getErrorString({code})");
            let cs = cstr((c_ges)(code));
            let rs = cstr((r_ges)(code));
            assert_eq!(cs, rs, "{label}: C='{cs}' Rust='{rs}'");
        }

        // row 328: ZSTD_getErrorName for a code that is not an error (returns
        // "No error detected"), and for many size_t codes — compare strings.
        // ZSTD_getErrorName takes a size_t functionResult.
        let size_codes: [size_t; 8] = [
            0,
            1,
            100,
            1000,
            0usize.wrapping_sub(1),   // (size_t)-1  -> an error code
            0usize.wrapping_sub(20),  // -ZSTD_error_corruption_detected
            0usize.wrapping_sub(72),  // -ZSTD_error_srcSize_wrong
            0usize.wrapping_sub(121), // beyond maxCode
        ];
        for &code in &size_codes {
            let label = format!("ERRORS row 328: ZSTD_getErrorName({code})");
            let cs = cstr((c_gen)(code));
            let rs = cstr((r_gen)(code));
            assert_eq!(cs, rs, "{label}: C='{cs}' Rust='{rs}'");
        }

        // ZSTD_ErrorCode round trip: ZSTD_getErrorCode(ERROR) for every valid code.
        // ERROR(x) == (size_t)(0 - x). getErrorCode should recover x for real errors.
        for &code in &ERROR_CODES {
            let function_result = 0usize.wrapping_sub(code as usize);
            let label = format!("ERRORS row 328: ZSTD_getErrorCode roundtrip code={code}");
            let cc = (c_gec)(function_result);
            let rc = (r_gec)(function_result);
            assert_eq!(cc, rc, "{label}: C={cc} R={rc}");
            // and the isError classification must agree
            assert_eq!((c_ie)(function_result), (r_ie)(function_result), "{label}: isError");
        }
    }
}
