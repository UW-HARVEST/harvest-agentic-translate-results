//! Phase C — ERROR-PATH differential tests for the entropy layer.
//!
//! Covers every rejection site in
//!   * `common/entropy_common.c`      (ERRORS.md rows 5-18)
//!   * `common/fse_decompress.c`      (rows 24-33)
//!   * `common/bitstream.h`           (rows 1-4)
//!   * `compress/fse_compress.c`      (rows 46-58)
//!   * `compress/huf_compress.c`      (rows 64-80)
//!   * `compress/hist.c`              (rows 59-63)
//!   * `decompress/huf_decompress.c`  (rows 268-304)
//!
//! Every call goes through `dlsym` in BOTH shared libraries; the returned
//! error *code* (not merely "both failed") plus every output buffer and
//! out-parameter is compared in full.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_void};

// ===================================================================== consts

const FSE_MIN_TABLELOG: u32 = 5;
const FSE_MAX_TABLELOG: u32 = 12;
const FSE_DEFAULT_TABLELOG: u32 = 11;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const HUF_TABLELOG_MAX: u32 = 12;
const HUF_TABLELOG_DEFAULT: u32 = 11;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE: usize = (4 * (HUF_SYMBOLVALUE_MAX as usize + 1) + 192) * 4;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
const HUF_DECODER_FAST_TABLELOG: u32 = 11;

const HIST_WKSP_SIZE: usize = 1024 * 4;

const HUF_flags_bmi2: c_int = 1 << 0;
const HUF_flags_optimalDepth: c_int = 1 << 1;
const HUF_flags_preferRepeat: c_int = 1 << 2;
const HUF_flags_suspectUncompressible: c_int = 1 << 3;
const HUF_flags_disableAsm: c_int = 1 << 4;
const HUF_flags_disableFast: c_int = 1 << 5;

const HUF_repeat_none: c_int = 0;
const HUF_repeat_check: c_int = 1;
const HUF_repeat_valid: c_int = 2;

// `FSE_DecompressWksp` == `short ncount[FSE_MAX_SYMBOL_VALUE+1]`
const FSE_DECOMPRESS_WKSP_MIN: usize = 2 * 256;

const fn fse_dtable_size_u32(tl: u32) -> usize {
    1 + (1usize << tl)
}
const fn fse_ctable_size_u32(tl: u32, msv: u32) -> usize {
    1 + (1usize << (tl - 1)) + ((msv as usize + 1) * 2)
}
/// `FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)`
fn fse_build_ctable_wksp_u32(msv: u32, tl: u32) -> usize {
    ((msv as usize + 2) + (1usize << tl)) / 2 + 2
}
/// `FSE_BUILD_DTABLE_WKSP_SIZE(maxTableLog, maxSymbolValue)` — in BYTES.
fn fse_build_dtable_wksp_bytes(tl: u32, msv: u32) -> usize {
    2 * (msv as usize + 1) + (1usize << tl) + 8
}
fn fse_build_dtable_wksp_u32(tl: u32, msv: u32) -> usize {
    (fse_build_dtable_wksp_bytes(tl, msv) + 3) / 4
}
/// `FSE_DECOMPRESS_WKSP_SIZE(maxTableLog, maxSymbolValue)` — in BYTES.
fn fse_decompress_wksp_bytes(tl: u32, msv: u32) -> usize {
    4 * (fse_dtable_size_u32(tl) + 1 + fse_build_dtable_wksp_u32(tl, msv) + 128 + 1)
}

// ================================================================= fn types

type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnHistCountWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    usize,
    *mut c_void,
    usize,
) -> usize;
type FnHistSimple = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> c_uint;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, usize);

type FnOptimalTableLog = unsafe extern "C" fn(c_uint, usize, c_uint) -> c_uint;
type FnOptimalTableLogInt = unsafe extern "C" fn(c_uint, usize, c_uint, c_uint) -> c_uint;
type FnNormalizeCount =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, usize, c_uint, c_uint) -> usize;
type FnNCountWriteBound = unsafe extern "C" fn(c_uint, c_uint) -> usize;
type FnWriteNCount = unsafe extern "C" fn(*mut c_void, usize, *const i16, c_uint, c_uint) -> usize;
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnReadNCountBmi2 = unsafe extern "C" fn(
    *mut i16,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    usize,
    c_int,
) -> usize;
type FnBuildCTableWksp =
    unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnBuildCTableRle = unsafe extern "C" fn(*mut u32, u8) -> usize;
type FnCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const u32) -> usize;
type FnBuildDTableWksp =
    unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_uint,
    *mut c_void,
    usize,
    c_int,
) -> usize;

type FnHufRepeat = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_uint,
    c_uint,
    *mut c_void,
    usize,
    *mut u64,
    *mut c_int,
    c_int,
) -> usize;
type FnHufUsingCTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const u64, c_int) -> usize;
type FnHufBuildCTable =
    unsafe extern "C" fn(*mut u64, *const c_uint, u32, u32, *mut c_void, usize) -> usize;
type FnHufWriteCTable = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const u64,
    c_uint,
    c_uint,
    *mut c_void,
    usize,
) -> usize;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut u64, *mut c_uint, *const c_void, usize, *mut c_uint) -> usize;
type FnHufOptimalTableLog = unsafe extern "C" fn(
    c_uint,
    usize,
    c_uint,
    *mut c_void,
    usize,
    *mut u64,
    *const c_uint,
    c_int,
) -> c_uint;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut u32,
    *mut u32,
    *const c_void,
    usize,
) -> usize;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut u32,
    *mut u32,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    c_int,
) -> usize;
type FnHufReadDTable =
    unsafe extern "C" fn(*mut u32, *const c_void, usize, *mut c_void, usize, c_int) -> usize;
type FnHufDecUsingDTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const u32, c_int) -> usize;
type FnHufDecDCtxWksp = unsafe extern "C" fn(
    *mut u32,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    c_int,
) -> usize;

// ================================================================== helpers

/// Compare an error/return `size_t` between the two libraries by *code*, and
/// cross-check the three error-name accessors as well.
#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        let (fic, fir) = duo::<FnIsError>("FSE_isError");
        let (fnc, fnr) = duo::<FnErrName>("FSE_getErrorName");
        let (hic, hir) = duo::<FnIsError>("HUF_isError");
        let (hnc, hnr) = duo::<FnErrName>("HUF_getErrorName");
        let (zic, zir) = duo::<FnIsError>("ZSTD_isError");
        assert_eq!(
            c, r,
            "{what}: C={c:#x} (code {} = {}), Rust={r:#x} (code {} = {})",
            gcc(c),
            cstr(nc(c)),
            gcr(r),
            cstr(nr(r))
        );
        assert_eq!(gcc(c), gcr(r), "{what}: ZSTD_getErrorCode mismatch");
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: ZSTD_getErrorName mismatch");
        assert_eq!(fic(c), fir(r), "{what}: FSE_isError mismatch");
        assert_eq!(cstr(fnc(c)), cstr(fnr(r)), "{what}: FSE_getErrorName mismatch");
        assert_eq!(hic(c), hir(r), "{what}: HUF_isError mismatch");
        assert_eq!(cstr(hnc(c)), cstr(hnr(r)), "{what}: HUF_getErrorName mismatch");
        assert_eq!(zic(c), zir(r), "{what}: ZSTD_isError mismatch");
    }
}

/// Assert `n` is an error whose `ZSTD_getErrorCode` equals `code` (per the C lib).
#[track_caller]
fn assert_code(what: &str, n: usize, code: c_uint) {
    unsafe {
        let (gcc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, _) = duo::<FnErrName>("ZSTD_getErrorName");
        assert!(is_err(n), "{what}: expected an error, got {n}");
        assert_eq!(
            gcc(n),
            code,
            "{what}: expected code {code}, got {} ({})",
            gcc(n),
            cstr(nc(n))
        );
    }
}

// ZSTD_ErrorCode values used below (zstd_errors.h).
const E_GENERIC: c_uint = 1;
const E_corruption_detected: c_uint = 20;
const E_tableLog_tooLarge: c_uint = 44;
const E_maxSymbolValue_tooLarge: c_uint = 46;
const E_maxSymbolValue_tooSmall: c_uint = 48;
const E_workSpace_tooSmall: c_uint = 66;
const E_dstSize_tooSmall: c_uint = 70;
const E_srcSize_wrong: c_uint = 72;

fn twin(n: usize) -> (Vec<u8>, Vec<u8>) {
    (vec![0xA5u8; n], vec![0xA5u8; n])
}
fn twin32(n: usize) -> (Vec<u32>, Vec<u32>) {
    (vec![0xA5A5_A5A5u32; n], vec![0xA5A5_A5A5u32; n])
}
fn twin64(n: usize) -> (Vec<u64>, Vec<u64>) {
    (vec![0xA5A5_A5A5_A5A5_A5A5u64; n], vec![0xA5A5_A5A5_A5A5_A5A5u64; n])
}
fn twin16(n: usize) -> (Vec<i16>, Vec<i16>) {
    (vec![0x5A5Ai16; n], vec![0x5A5Ai16; n])
}
fn as_bytes32(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}
fn as_bytes64(v: &[u64]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) }
}
fn as_bytes16(v: &[i16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) }
}

/// A `HUF_DTable` array sized for `HUF_TABLELOG_MAX`, with a hand-written
/// header exactly as `HUF_CREATE_STATIC_DTABLEX{1,2}` writes it.
fn new_dtable_x1(max_table_log: u32) -> Vec<u32> {
    let mut d = vec![0u32; 1 + (1 << HUF_TABLELOG_MAX)];
    d[0] = (max_table_log - 1).wrapping_mul(0x0100_0001);
    d
}
fn new_dtable_x2(max_table_log: u32) -> Vec<u32> {
    let mut d = vec![0u32; 1 + (1 << HUF_TABLELOG_MAX)];
    d[0] = max_table_log.wrapping_mul(0x0100_0001);
    d
}
/// A raw DTable descriptor with an explicit `maxTableLog` byte (byte 0).
fn dtable_with_desc(max_table_log: u32) -> Vec<u32> {
    let mut d = vec![0u32; 1 + (1 << HUF_TABLELOG_MAX)];
    d[0] = max_table_log & 0xFF;
    d
}

/// `HUF_readStats()` "special" (raw 4-bit) weight header for `weights`.
fn raw_weight_header(weights: &[u8]) -> Vec<u8> {
    let osize = weights.len();
    assert!(osize >= 1 && osize <= 128);
    let mut v = vec![(127 + osize) as u8];
    let n = (osize + 1) / 2;
    for i in 0..n {
        let hi = weights[2 * i] & 15;
        let lo = if 2 * i + 1 < osize { weights[2 * i + 1] & 15 } else { 0 };
        v.push((hi << 4) | lo);
    }
    v
}

/// A valid FSE-coded stream: normalized-count header followed by the bitstream.
struct FseStream {
    blob: Vec<u8>,
    hdr_len: usize,
    table_log: u32,
    msv: u32,
    dec_size: usize,
}

/// Build a complete FSE stream with the **C** library only.
unsafe fn c_fse_stream(src: &[u8], want_table_log: u32) -> Option<FseStream> {
    assert!(src.len() > 2);
    let (hist, _) = duo::<FnHistCountWksp>("HIST_count_wksp");
    let (otl, _) = duo::<FnOptimalTableLog>("FSE_optimalTableLog");
    let (norm, _) = duo::<FnNormalizeCount>("FSE_normalizeCount");
    let (wnc, _) = duo::<FnWriteNCount>("FSE_writeNCount");
    let (bct, _) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
    let (cuc, _) = duo::<FnCompressUsingCTable>("FSE_compress_usingCTable");

    let mut count = vec![0u32; 256];
    let mut msv: c_uint = 255;
    let mut hwksp = vec![0u32; 1024];
    let mx = hist(
        count.as_mut_ptr(),
        &mut msv,
        src.as_ptr() as *const c_void,
        src.len(),
        hwksp.as_mut_ptr() as *mut c_void,
        hwksp.len() * 4,
    );
    if is_err(mx) || msv < 1 || mx == src.len() {
        return None; // RLE or degenerate
    }
    let tl = otl(want_table_log, src.len(), msv);
    let mut nrm = vec![0i16; 256];
    let r = norm(nrm.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, 1);
    if is_err(r) || r == 0 {
        return None;
    }
    let tl = r as u32;
    let mut blob = vec![0u8; src.len() * 2 + 1024];
    let hdr = wnc(blob.as_mut_ptr() as *mut c_void, blob.len(), nrm.as_ptr(), msv, tl);
    if is_err(hdr) {
        return None;
    }
    let mut ct = vec![0u32; fse_ctable_size_u32(FSE_MAX_TABLELOG, 255) + 8];
    let mut cw = vec![0u32; fse_build_ctable_wksp_u32(255, FSE_MAX_TABLELOG) + 8];
    let e = bct(
        ct.as_mut_ptr(),
        nrm.as_ptr(),
        msv,
        tl,
        cw.as_mut_ptr() as *mut c_void,
        cw.len() * 4,
    );
    if is_err(e) {
        return None;
    }
    let body = cuc(
        blob.as_mut_ptr().add(hdr) as *mut c_void,
        blob.len() - hdr,
        src.as_ptr() as *const c_void,
        src.len(),
        ct.as_ptr(),
    );
    if is_err(body) || body == 0 {
        return None;
    }
    blob.truncate(hdr + body);
    Some(FseStream { blob, hdr_len: hdr, table_log: tl, msv, dec_size: src.len() })
}

/// A valid HUF blob (table description + bitstream) plus the description length.
struct HufBlob {
    blob: Vec<u8>,
    desc_len: usize,
    dec_size: usize,
}

unsafe fn c_huf_blob(src: &[u8], msv: c_uint, tl: c_uint, four: bool) -> Option<HufBlob> {
    let name = if four { "HUF_compress4X_repeat" } else { "HUF_compress1X_repeat" };
    let (fc, _) = duo::<FnHufRepeat>(name);
    let (bd, _) = duo::<FnSizeT1>("HUF_compressBound");
    let (rs, _) = duo::<FnHufReadStats>("HUF_readStats");
    let cap = bd(src.len()).max(64);
    let mut out = vec![0u8; cap];
    let mut ct = vec![0u64; 258];
    let mut w = vec![0u64; HUF_WORKSPACE_SIZE / 8];
    let mut rep = HUF_repeat_none;
    let n = fc(
        out.as_mut_ptr() as *mut c_void,
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
        msv,
        tl,
        w.as_mut_ptr() as *mut c_void,
        HUF_WORKSPACE_SIZE,
        ct.as_mut_ptr(),
        &mut rep,
        0,
    );
    if is_err(n) || n <= 1 {
        return None;
    }
    out.truncate(n);
    let mut hw = vec![0u8; 256];
    let mut rk = vec![0u32; 16];
    let mut ns = 0u32;
    let mut tlog = 0u32;
    let desc = rs(
        hw.as_mut_ptr(),
        256,
        rk.as_mut_ptr(),
        &mut ns,
        &mut tlog,
        out.as_ptr() as *const c_void,
        out.len(),
    );
    if is_err(desc) || desc >= out.len() {
        return None;
    }
    Some(HufBlob { blob: out, desc_len: desc, dec_size: src.len() })
}

/// Source data that reliably HUF-compresses (skewed byte distribution).
fn skewed(n: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let x = rng.next_u32();
            if x % 4 == 0 {
                (x >> 8) as u8 % 32
            } else if x % 4 == 1 {
                7
            } else {
                (x >> 16) as u8 % 8
            }
        })
        .collect()
}

// =================================================== bitstream.h  (rows 1-4)

/// Row 1: `BIT_initCStream` -> `dstSize_tooSmall`.
///
/// `FSE_compress_usingCTable_generic` *swallows* the error
/// (`if (FSE_isError(initError)) return 0;`, fse_compress.c L565), so the only
/// observable behaviour is "returns 0"; both libraries must agree.
#[test]
fn err_bit_initcstream_dstsize_toosmall() {
    unsafe {
        let (cc, cr) = duo::<FnCompressUsingCTable>("FSE_compress_usingCTable");
        let (norm, _) = duo::<FnNormalizeCount>("FSE_normalizeCount");
        let (bct, _) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
        let (hist, _) = duo::<FnHistCountWksp>("HIST_count_wksp");

        let src = skewed(4096, 0xC0FFEE);
        let mut count = vec![0u32; 256];
        let mut msv: c_uint = 255;
        let mut hw = vec![0u32; 1024];
        let mx = hist(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
            hw.as_mut_ptr() as *mut c_void,
            hw.len() * 4,
        );
        assert!(!is_err(mx));
        let mut nrm = vec![0i16; 256];
        let tl = norm(nrm.as_mut_ptr(), 11, count.as_ptr(), src.len(), msv, 1) as u32;
        assert!(!is_err(tl as usize));
        let mut ct = vec![0u32; fse_ctable_size_u32(FSE_MAX_TABLELOG, 255) + 8];
        let mut cw = vec![0u32; fse_build_ctable_wksp_u32(255, FSE_MAX_TABLELOG) + 8];
        assert!(!is_err(bct(
            ct.as_mut_ptr(),
            nrm.as_ptr(),
            msv,
            tl,
            cw.as_mut_ptr() as *mut c_void,
            cw.len() * 4
        )));

        // `sizeof(bitC->bitContainer)` == 8 on this target: any dstCapacity <= 8
        // makes BIT_initCStream fail. srcSize <= 2 short-circuits to 0 first.
        for cap in 0usize..=12 {
            for srclen in [0usize, 1, 2, 3, 4, 17, 4096] {
                if srclen > src.len() {
                    continue;
                }
                let (mut dc, mut dr) = twin(cap.max(1));
                let a = cc(
                    dc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    srclen,
                    ct.as_ptr(),
                );
                let b = cr(
                    dr.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    srclen,
                    ct.as_ptr(),
                );
                eqcode(&format!("FSE_compress_usingCTable(cap={cap},src={srclen})"), a, b);
                eqbuf(&format!("FSE_compress_usingCTable dst cap={cap} src={srclen}"), &dc, &dr);
                if srclen > 2 && cap <= 8 {
                    assert_eq!(a, 0, "cap={cap}: BIT_initCStream failure must surface as 0");
                }
            }
        }
    }
}

/// Rows 2-4: `BIT_initDStream` — `srcSize_wrong` (srcSize < 1),
/// `GENERIC` (srcSize >= 8 and last byte == 0),
/// `corruption_detected` (srcSize < 8 and last byte == 0).
#[test]
fn err_bit_initdstream_all_three() {
    unsafe {
        let (u1c, u1r) = duo::<FnHufDecUsingDTable>("HUF_decompress1X_usingDTable");
        let (d1c, _) = duo::<FnHufReadDTable>("HUF_readDTableX1_wksp");

        // Build a real X1 DTable so that BIT_initDStream is the first thing
        // that can fail inside the decoder.
        let src = skewed(8192, 0x1D5);
        let hb = c_huf_blob(&src, 0, 0, false).expect("huf blob");
        let mut dt = new_dtable_x1(HUF_TABLELOG_MAX);
        let mut dws = vec![0u8; HUF_DECOMPRESS_WORKSPACE_SIZE];
        let hs = d1c(
            dt.as_mut_ptr(),
            hb.blob.as_ptr() as *const c_void,
            hb.blob.len(),
            dws.as_mut_ptr() as *mut c_void,
            HUF_DECOMPRESS_WORKSPACE_SIZE,
            0,
        );
        assert!(!is_err(hs));

        let body = &hb.blob[hb.desc_len..];
        assert!(body.len() >= 16);

        // --- row 2: srcSize < 1
        for dst in [1usize, 8, 64] {
            let (mut dc, mut dr) = twin(dst);
            let a = u1c(
                dc.as_mut_ptr() as *mut c_void,
                dst,
                body.as_ptr() as *const c_void,
                0,
                dt.as_ptr(),
                0,
            );
            let b = u1r(
                dr.as_mut_ptr() as *mut c_void,
                dst,
                body.as_ptr() as *const c_void,
                0,
                dt.as_ptr(),
                0,
            );
            eqcode(&format!("BIT_initDStream srcSize=0 dst={dst}"), a, b);
            eqbuf("BIT_initDStream srcSize=0 dst", &dc, &dr);
            assert_code("BIT_initDStream srcSize=0", a, E_srcSize_wrong);
        }

        // --- row 3: srcSize >= 8, last byte == 0 -> GENERIC
        for n in [8usize, 9, 16, 33] {
            let mut s = body[..n.min(body.len())].to_vec();
            let ln = s.len();
            s[ln - 1] = 0;
            let (mut dc, mut dr) = twin(64);
            let a = u1c(
                dc.as_mut_ptr() as *mut c_void,
                64,
                s.as_ptr() as *const c_void,
                s.len(),
                dt.as_ptr(),
                0,
            );
            let b = u1r(
                dr.as_mut_ptr() as *mut c_void,
                64,
                s.as_ptr() as *const c_void,
                s.len(),
                dt.as_ptr(),
                0,
            );
            eqcode(&format!("BIT_initDStream endMark0 n={n}"), a, b);
            eqbuf("BIT_initDStream endMark0 dst", &dc, &dr);
            assert_code("BIT_initDStream endMark0 (>=8)", a, E_GENERIC);
        }

        // --- row 4: srcSize < 8, last byte == 0 -> corruption_detected
        for n in 1usize..8 {
            let mut s = body[..n].to_vec();
            s[n - 1] = 0;
            let (mut dc, mut dr) = twin(64);
            let a = u1c(
                dc.as_mut_ptr() as *mut c_void,
                64,
                s.as_ptr() as *const c_void,
                n,
                dt.as_ptr(),
                0,
            );
            let b = u1r(
                dr.as_mut_ptr() as *mut c_void,
                64,
                s.as_ptr() as *const c_void,
                n,
                dt.as_ptr(),
                0,
            );
            eqcode(&format!("BIT_initDStream short endMark0 n={n}"), a, b);
            eqbuf("BIT_initDStream short endMark0 dst", &dc, &dr);
            assert_code("BIT_initDStream endMark0 (<8)", a, E_corruption_detected);
        }
    }
}

// ============================== entropy_common.c : FSE_readNCount (rows 5-9)

/// Call `FSE_readNCount` / `FSE_readNCount_bmi2` on both libraries and compare
/// the return code plus every out-parameter and the whole `normalizedCounter`.
#[track_caller]
unsafe fn diff_readncount(what: &str, hdr: &[u8], hb_size: usize, msv_in: c_uint) {
    let (fc, fr) = duo::<FnReadNCount>("FSE_readNCount");
    let (bc, br) = duo::<FnReadNCountBmi2>("FSE_readNCount_bmi2");
    // 4096 shorts: enough for any msv_in used here (incl. 0xFFFFFFFF, whose
    // memset length is (msv+1)==0).
    let (mut nc, mut nr) = twin16(4096);
    let mut mc: c_uint = msv_in;
    let mut mr: c_uint = msv_in;
    let mut tc: c_uint = 0xDEAD_BEEF;
    let mut tr: c_uint = 0xDEAD_BEEF;
    let a = fc(nc.as_mut_ptr(), &mut mc, &mut tc, hdr.as_ptr() as *const c_void, hb_size);
    let b = fr(nr.as_mut_ptr(), &mut mr, &mut tr, hdr.as_ptr() as *const c_void, hb_size);
    eqcode(&format!("FSE_readNCount {what}"), a, b);
    eqv(&format!("FSE_readNCount {what} *maxSVPtr"), mc, mr);
    eqv(&format!("FSE_readNCount {what} *tableLogPtr"), tc, tr);
    eqbuf(&format!("FSE_readNCount {what} ncount"), as_bytes16(&nc), as_bytes16(&nr));

    for bmi2 in [0i32, 1, -1, 2, 0x7FFF_FFFF, i32::MIN] {
        let (mut nc, mut nr) = twin16(4096);
        let mut mc: c_uint = msv_in;
        let mut mr: c_uint = msv_in;
        let mut tc: c_uint = 0xDEAD_BEEF;
        let mut tr: c_uint = 0xDEAD_BEEF;
        let a = bc(
            nc.as_mut_ptr(),
            &mut mc,
            &mut tc,
            hdr.as_ptr() as *const c_void,
            hb_size,
            bmi2,
        );
        let b = br(
            nr.as_mut_ptr(),
            &mut mr,
            &mut tr,
            hdr.as_ptr() as *const c_void,
            hb_size,
            bmi2,
        );
        eqcode(&format!("FSE_readNCount_bmi2({bmi2}) {what}"), a, b);
        eqv(&format!("FSE_readNCount_bmi2({bmi2}) {what} *maxSVPtr"), mc, mr);
        eqv(&format!("FSE_readNCount_bmi2({bmi2}) {what} *tableLogPtr"), tc, tr);
        eqbuf(
            &format!("FSE_readNCount_bmi2({bmi2}) {what} ncount"),
            as_bytes16(&nc),
            as_bytes16(&nr),
        );
    }
}

/// Row 6: `nbBits > FSE_TABLELOG_ABSOLUTE_MAX` -> `tableLog_tooLarge`
/// (low nibble of byte 0 >= 11, so tableLog >= 16).
#[test]
fn err_fse_readncount_tablelog_toolarge() {
    unsafe {
        for nib in 11u8..16 {
            for extra in [0u8, 0xFF, 0x5A] {
                let mut hdr = vec![extra; 16];
                hdr[0] = (extra & 0xF0) | nib;
                for hb in [8usize, 9, 12, 16] {
                    diff_readncount(&format!("tableLog nib={nib} extra={extra} hb={hb}"), &hdr, hb, 255);
                }
                // the exact code, from the C side
                let (fc, _) = duo::<FnReadNCount>("FSE_readNCount");
                let mut n = vec![0i16; 4096];
                let mut m: c_uint = 255;
                let mut t: c_uint = 0;
                let r = fc(n.as_mut_ptr(), &mut m, &mut t, hdr.as_ptr() as *const c_void, 16);
                assert_code(&format!("FSE_readNCount nib={nib}"), r, E_tableLog_tooLarge);
            }
        }
        // hbSize < 8 takes the padded-buffer path but must produce the same code
        for nib in 11u8..16 {
            let mut hdr = vec![0u8; 8];
            hdr[0] = nib;
            for hb in 1usize..8 {
                diff_readncount(&format!("short tableLog nib={nib} hb={hb}"), &hdr, hb, 255);
            }
        }
    }
}

/// Row 5: `hbSize < 8` and the (zero-padded) decode consumed more than
/// `hbSize` bytes -> `corruption_detected`.
#[test]
fn err_fse_readncount_short_header_corruption() {
    unsafe {
        // A *valid* header of length L, truncated to < L, is decoded out of the
        // zero-padded 8-byte buffer and reports countSize > hbSize.
        let src = skewed(4096, 0x5EED);
        let st = c_fse_stream(&src, 11).expect("fse stream");
        let full = &st.blob[..st.hdr_len];
        assert!(full.len() >= 3, "header too short to truncate meaningfully");
        let mut saw = false;
        for hb in 1..full.len().min(8) {
            let mut hdr = full[..hb].to_vec();
            hdr.resize(8, 0);
            diff_readncount(&format!("truncated hdr hb={hb}"), &hdr, hb, 255);
            let (fc, _) = duo::<FnReadNCount>("FSE_readNCount");
            let mut n = vec![0i16; 4096];
            let mut m: c_uint = 255;
            let mut t: c_uint = 0;
            let r = fc(n.as_mut_ptr(), &mut m, &mut t, hdr.as_ptr() as *const c_void, hb);
            if is_err(r) {
                let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
                if gc(r) == E_corruption_detected {
                    saw = true;
                }
            }
        }
        // Also: a tiny header claiming a large distribution.
        for hb in 1usize..8 {
            for pat in [0x00u8, 0x01, 0x0A, 0x08, 0x55, 0xAA, 0xFE] {
                let mut hdr = vec![pat; 8];
                hdr[0] = pat & 0x0A; // valid tableLog nibble
                diff_readncount(&format!("tiny hdr pat={pat} hb={hb}"), &hdr, hb, 255);
            }
        }
        assert!(saw, "row 5 (countSize > hbSize) was never reached");
    }
}

/// Row 7: `remaining != 1` -> `corruption_detected`.
/// Row 9: `bitCount > 32` -> `corruption_detected`.
#[test]
fn err_fse_readncount_remaining_corruption() {
    unsafe {
        // A header whose very first count consumes far more than the table.
        // nibble 0 => nbBits 5, threshold 32, remaining 33.
        // 0xF0,0x00.. => bitStream>>4 low bits give count 15 -> remaining 33-14
        // (not 1) and the stream then runs out of symbols.
        let cases: [&[u8]; 8] = [
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            &[0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            &[0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            &[0x03, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77],
            &[0x08, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80],
            &[0x01, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0],
            &[0x06, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA],
            &[0x0A, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ];
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (fc, _) = duo::<FnReadNCount>("FSE_readNCount");
        let mut saw_corruption = false;
        for (i, c) in cases.iter().enumerate() {
            for msv in [0u32, 1, 3, 15, 63, 255] {
                diff_readncount(&format!("case{i} msv={msv}"), c, c.len(), msv);
                let mut n = vec![0i16; 4096];
                let mut m: c_uint = msv;
                let mut t: c_uint = 0;
                let r = fc(n.as_mut_ptr(), &mut m, &mut t, c.as_ptr() as *const c_void, c.len());
                if is_err(r) && gc(r) == E_corruption_detected {
                    saw_corruption = true;
                }
            }
        }
        assert!(saw_corruption, "no corruption_detected reached (rows 7/9)");
    }
}

/// Row 8: `charnum > maxSV1` -> `maxSymbolValue_tooSmall`.
///
/// The only way to leave the decode loop with `remaining == 1` *and*
/// `charnum > maxSV1` is `maxSV1 == 0` (i.e. `*maxSVPtr == UINT_MAX`, whose
/// `memset` length `(*maxSVPtr+1)` is 0 — no out-of-bounds write).
/// Byte 0 low nibble 0 => nbBits 5; bits 4..9 all set => raw count 63 -> 33,
/// so `remaining` drops from 33 to exactly 1 on the very first symbol.
#[test]
fn err_fse_readncount_maxsymbolvalue_toosmall() {
    unsafe {
        let hdr: [u8; 8] = [0xF0, 0x03, 0, 0, 0, 0, 0, 0];
        diff_readncount("row8 maxSV1==0", &hdr, 8, 0xFFFF_FFFF);
        let (fc, _) = duo::<FnReadNCount>("FSE_readNCount");
        let mut n = vec![0i16; 4096];
        let mut m: c_uint = 0xFFFF_FFFF;
        let mut t: c_uint = 0;
        let r = fc(n.as_mut_ptr(), &mut m, &mut t, hdr.as_ptr() as *const c_void, 8);
        assert_code("FSE_readNCount row8", r, E_maxSymbolValue_tooSmall);
        // longer buffers and trailing garbage must behave identically
        for extra in [0u8, 0xFF, 0x5A] {
            for hb in [8usize, 9, 16, 24] {
                let mut h = vec![extra; hb];
                h[0] = 0xF0;
                h[1] = 0x03 | (extra & 0xFC);
                diff_readncount(&format!("row8 extra={extra} hb={hb}"), &h, hb, 0xFFFF_FFFF);
            }
        }
    }
}

/// Randomised corruption of *valid* normalized-count headers, plus fully
/// random headers, over the whole `hbSize` ladder. Fixed seed.
#[test]
fn err_fse_readncount_fuzz() {
    unsafe {
        let mut rng = Rng::new(0xF5E_0001);
        // pool of valid headers at every table log
        let mut valid: Vec<Vec<u8>> = Vec::new();
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            for cls in 0..N_CLASSES {
                let d = gen_class(cls, 3000, 0x1234 + tl as u64);
                if let Some(st) = c_fse_stream(&d, tl) {
                    valid.push(st.blob[..st.hdr_len].to_vec());
                }
            }
        }
        assert!(valid.len() >= 4, "not enough valid headers ({})", valid.len());

        for h in &valid {
            for _ in 0..120 {
                let mut v = h.clone();
                let nmut = 1 + rng.below(3);
                for _ in 0..nmut {
                    let i = rng.below(v.len());
                    let b = rng.byte();
                    v[i] = b;
                }
                let hb = if rng.below(4) == 0 {
                    1 + rng.below(v.len())
                } else {
                    v.len()
                };
                let msv = [0u32, 1, 7, 31, 63, 127, 200, 255][rng.below(8)];
                if v.len() < 8 {
                    v.resize(8, 0);
                }
                diff_readncount("fuzz-mutated", &v, hb, msv);
            }
        }
        for _ in 0..600 {
            let n = 1 + rng.below(24);
            let mut v = rng.bytes(n.max(8));
            if rng.below(2) == 0 {
                let nib = rng.below(11) as u8;
                v[0] = (v[0] & 0xF0) | nib;
            }
            let msv = [0u32, 1, 7, 31, 63, 127, 200, 255][rng.below(8)];
            diff_readncount("fuzz-random", &v, n, msv);
        }
    }
}

// ====================== fse_decompress.c : FSE_buildDTable_wksp (rows 24-27)

#[track_caller]
unsafe fn diff_build_dtable(
    what: &str,
    norm: &[i16],
    msv: c_uint,
    tl: c_uint,
    wksp_bytes: usize,
    dt_u32: usize,
) -> usize {
    let (fc, fr) = duo::<FnBuildDTableWksp>("FSE_buildDTable_wksp");
    let (mut dc, mut dr) = twin32(dt_u32);
    let mut wc = vec![0u32; (wksp_bytes + 3) / 4 + 4];
    let mut wr = vec![0u32; (wksp_bytes + 3) / 4 + 4];
    let a = fc(
        dc.as_mut_ptr(),
        norm.as_ptr(),
        msv,
        tl,
        wc.as_mut_ptr() as *mut c_void,
        wksp_bytes,
    );
    let b = fr(
        dr.as_mut_ptr(),
        norm.as_ptr(),
        msv,
        tl,
        wr.as_mut_ptr() as *mut c_void,
        wksp_bytes,
    );
    eqcode(&format!("FSE_buildDTable_wksp {what}"), a, b);
    eqbuf(&format!("FSE_buildDTable_wksp {what} dtable"), as_bytes32(&dc), as_bytes32(&dr));
    eqbuf(&format!("FSE_buildDTable_wksp {what} wksp"), as_bytes32(&wc), as_bytes32(&wr));
    a
}

/// Rows 24-26: workspace one byte short -> `maxSymbolValue_tooLarge`;
/// `maxSymbolValue > FSE_MAX_SYMBOL_VALUE` -> `maxSymbolValue_tooLarge`;
/// `tableLog > FSE_MAX_TABLELOG` -> `tableLog_tooLarge`.
#[test]
fn err_fse_builddtable_wksp_guards() {
    unsafe {
        // A genuinely valid normalized distribution to isolate the guards.
        let src = skewed(4096, 0xB1D);
        let st = c_fse_stream(&src, 11).expect("fse stream");
        let (rnc, _) = duo::<FnReadNCount>("FSE_readNCount");
        let mut norm = vec![0i16; 256];
        let mut msv: c_uint = 255;
        let mut tl: c_uint = 0;
        let rd = rnc(
            norm.as_mut_ptr(),
            &mut msv,
            &mut tl,
            st.blob.as_ptr() as *const c_void,
            st.blob.len(),
        );
        assert!(!is_err(rd));

        let dt_u32 = fse_dtable_size_u32(FSE_MAX_TABLELOG) + 8;
        let need = fse_build_dtable_wksp_bytes(tl, msv);

        // sanity: exact size works, identically in both
        let ok = diff_build_dtable("exact wksp", &norm, msv, tl, need, dt_u32);
        assert!(!is_err(ok), "exact workspace size should succeed");

        // --- row 24: one byte short (and further short sizes)
        for short in [1usize, 2, 4, 8, 64, need / 2] {
            if short > need {
                continue;
            }
            let n = need - short;
            let r = diff_build_dtable(&format!("wksp short by {short}"), &norm, msv, tl, n, dt_u32);
            assert_code("FSE_buildDTable_wksp short wksp", r, E_maxSymbolValue_tooLarge);
        }
        let r = diff_build_dtable("wksp 0", &norm, msv, tl, 0, dt_u32);
        assert_code("FSE_buildDTable_wksp wksp=0", r, E_maxSymbolValue_tooLarge);

        // --- row 25: maxSymbolValue > 255 (workspace deliberately huge)
        for bad in [256u32, 257, 511, 1000, 0xFFFF] {
            let big = fse_build_dtable_wksp_bytes(tl, bad) + 64;
            let r = diff_build_dtable(&format!("msv={bad}"), &norm, bad, tl, big, dt_u32);
            assert_code("FSE_buildDTable_wksp msv too large", r, E_maxSymbolValue_tooLarge);
        }

        // --- row 26: tableLog > FSE_MAX_TABLELOG (workspace deliberately huge)
        for bad in [13u32, 14, 15, 16, 20] {
            let big = fse_build_dtable_wksp_bytes(bad, 255) + 64;
            let r = diff_build_dtable(&format!("tl={bad}"), &norm, 255, bad, big, dt_u32);
            assert_code("FSE_buildDTable_wksp tableLog too large", r, E_tableLog_tooLarge);
        }
        // ordering: msv check comes after the workspace check
        for bad in [256u32, 300] {
            let r = diff_build_dtable(&format!("msv={bad} tiny wksp"), &norm, bad, tl, 8, dt_u32);
            assert_code("FSE_buildDTable_wksp msv+tiny wksp", r, E_maxSymbolValue_tooLarge);
        }
    }
}

/// Row 27: `position != 0` after spreading a distribution whose counts do not
/// sum to `1 << tableLog` (the `highThreshold != tableSize-1` branch) ->
/// `GENERIC`.
#[test]
fn err_fse_builddtable_wksp_generic() {
    unsafe {
        let dt_u32 = fse_dtable_size_u32(FSE_MAX_TABLELOG) + 8;
        // At least one -1 ("lowprob") entry forces the slow spreading path,
        // and the total is far below 1<<tableLog, so `position` cannot return
        // to 0.
        let mut saw = false;
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            for tail in [1i16, 2, 3, 4, 5, 7] {
                let mut norm = vec![0i16; 256];
                norm[0] = -1;
                norm[1] = tail;
                let need = fse_build_dtable_wksp_bytes(tl, 1) + 64;
                let r = diff_build_dtable(
                    &format!("bad distribution tl={tl} tail={tail}"),
                    &norm,
                    1,
                    tl,
                    need,
                    dt_u32,
                );
                if is_err(r) {
                    let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
                    if gc(r) == E_GENERIC {
                        saw = true;
                    }
                }
            }
        }
        assert!(saw, "row 27 (FSE_buildDTable_wksp GENERIC) was never reached");
    }
}

// ============ fse_decompress.c : FSE_decompress_wksp_bmi2 (rows 28-33, 29/30)

#[track_caller]
unsafe fn diff_fse_decompress(
    what: &str,
    dst_cap: usize,
    csrc: &[u8],
    csrc_size: usize,
    max_log: c_uint,
    wksp_bytes: usize,
) -> usize {
    let (fc, fr) = duo::<FnDecompressWkspBmi2>("FSE_decompress_wksp_bmi2");
    let mut out = usize::MAX;
    for bmi2 in [0i32, 1] {
        let (mut dc, mut dr) = twin(dst_cap.max(1));
        let mut wc = vec![0u32; wksp_bytes / 4 + 8];
        let mut wr = vec![0u32; wksp_bytes / 4 + 8];
        let a = fc(
            dc.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            max_log,
            wc.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            bmi2,
        );
        let b = fr(
            dr.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            max_log,
            wr.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            bmi2,
        );
        eqcode(&format!("FSE_decompress_wksp_bmi2({bmi2}) {what}"), a, b);
        eqbuf(&format!("FSE_decompress_wksp_bmi2({bmi2}) {what} dst"), &dc, &dr);
        eqbuf(
            &format!("FSE_decompress_wksp_bmi2({bmi2}) {what} wksp"),
            as_bytes32(&wc),
            as_bytes32(&wr),
        );
        out = a;
    }
    out
}

/// Rows 31-33 + 29/30: workspace / maxLog / dstCapacity rejections.
#[test]
fn err_fse_decompress_wksp_guards() {
    unsafe {
        let src = skewed(3000, 0xDEC0);
        let st = c_fse_stream(&src, 11).expect("fse stream");
        let full = fse_decompress_wksp_bytes(FSE_MAX_TABLELOG, 255);

        // sanity: succeeds with a full workspace and enough dst
        let ok = diff_fse_decompress(
            "valid",
            src.len(),
            &st.blob,
            st.blob.len(),
            FSE_MAX_TABLELOG,
            full,
        );
        assert_eq!(ok, src.len(), "valid FSE stream should round-trip");

        // --- row 31: wkspSize < sizeof(FSE_DecompressWksp) (== 512) -> GENERIC
        for w in [0usize, 4, 64, 256, FSE_DECOMPRESS_WKSP_MIN - 4] {
            let r = diff_fse_decompress(
                &format!("wksp={w}"),
                src.len(),
                &st.blob,
                st.blob.len(),
                FSE_MAX_TABLELOG,
                w,
            );
            assert_code("FSE_decompress_wksp tiny wksp", r, E_GENERIC);
        }

        // --- row 32: tableLog > maxLog -> tableLog_tooLarge
        for ml in 0..st.table_log {
            let r = diff_fse_decompress(
                &format!("maxLog={ml} (tableLog={})", st.table_log),
                src.len(),
                &st.blob,
                st.blob.len(),
                ml,
                full,
            );
            assert_code("FSE_decompress_wksp maxLog too small", r, E_tableLog_tooLarge);
        }

        // --- row 33: FSE_DECOMPRESS_WKSP_SIZE(tableLog, msv) > wkspSize
        let need = fse_decompress_wksp_bytes(st.table_log, st.msv);
        for short in [4usize, 8, 64, 256] {
            if need < FSE_DECOMPRESS_WKSP_MIN + short {
                continue;
            }
            let w = need - short;
            let r = diff_fse_decompress(
                &format!("wksp={w} need={need}"),
                src.len(),
                &st.blob,
                st.blob.len(),
                FSE_MAX_TABLELOG,
                w,
            );
            assert_code("FSE_decompress_wksp wksp short for tableLog", r, E_tableLog_tooLarge);
        }

        // --- rows 29/30: dstCapacity too small -> dstSize_tooSmall
        let mut saw_dst = false;
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        for cap in [
            0usize,
            1,
            2,
            3,
            4,
            5,
            8,
            16,
            src.len() / 4,
            src.len() / 2,
            src.len() - 1,
            src.len() - 2,
        ] {
            let r = diff_fse_decompress(
                &format!("dstCap={cap}"),
                cap,
                &st.blob,
                st.blob.len(),
                FSE_MAX_TABLELOG,
                full,
            );
            if is_err(r) && gc(r) == E_dstSize_tooSmall {
                saw_dst = true;
            }
        }
        assert!(saw_dst, "rows 29/30 (dstSize_tooSmall) never reached");

        // --- row 28: overflow right after the two state inits.
        // Truncate the bitstream to fewer bytes than the two initial states need.
        let mut saw_corrupt = false;
        for keep in 1..=12usize {
            if st.hdr_len + keep > st.blob.len() {
                break;
            }
            let r = diff_fse_decompress(
                &format!("truncated body keep={keep}"),
                src.len(),
                &st.blob,
                st.hdr_len + keep,
                FSE_MAX_TABLELOG,
                full,
            );
            if is_err(r) && gc(r) == E_corruption_detected {
                saw_corrupt = true;
            }
        }
        // srcSize == header length exactly -> BIT_initDStream(srcSize 0)
        let r = diff_fse_decompress(
            "no body",
            src.len(),
            &st.blob,
            st.hdr_len,
            FSE_MAX_TABLELOG,
            full,
        );
        assert_code("FSE_decompress_wksp empty body", r, E_srcSize_wrong);
        assert!(saw_corrupt, "row 28 (post-init BIT_DStream_overflow) never reached");

        // srcSize 0 / 1
        for n in [0usize, 1, 2, 3] {
            let r = diff_fse_decompress(
                &format!("cSrcSize={n}"),
                src.len(),
                &st.blob,
                n,
                FSE_MAX_TABLELOG,
                full,
            );
            assert!(is_err(r), "cSrcSize={n} must fail");
        }
    }
}

/// Randomised corruption of complete FSE streams across every table log,
/// every `maxLog`, both `bmi2` paths and a ladder of `dstCapacity` values.
#[test]
fn err_fse_decompress_wksp_fuzz() {
    unsafe {
        let mut rng = Rng::new(0xF5E_0002);
        let full = fse_decompress_wksp_bytes(FSE_MAX_TABLELOG, 255);
        let mut streams: Vec<(FseStream, usize)> = Vec::new();
        for cls in 0..N_CLASSES {
            for tl in [5u32, 7, 9, 11, 12] {
                let d = gen_class(cls, 2500, 0xABCD ^ tl as u64);
                if let Some(st) = c_fse_stream(&d, tl) {
                    let n = d.len();
                    streams.push((st, n));
                }
            }
        }
        assert!(streams.len() >= 6, "not enough FSE streams: {}", streams.len());

        for (st, dec) in &streams {
            for _ in 0..40 {
                let mut v = st.blob.clone();
                let nmut = 1 + rng.below(4);
                for _ in 0..nmut {
                    let i = rng.below(v.len());
                    let b = rng.byte();
                    v[i] = b;
                }
                let ml = [0u32, 5, 6, 9, 11, 12, 13, 20][rng.below(8)];
                let cap = match rng.below(5) {
                    0 => 0,
                    1 => 1 + rng.below(16),
                    2 => *dec / 2,
                    3 => *dec,
                    _ => *dec + 64,
                };
                let n = if rng.below(4) == 0 { 1 + rng.below(v.len()) } else { v.len() };
                let w = match rng.below(4) {
                    0 => full,
                    1 => FSE_DECOMPRESS_WKSP_MIN,
                    2 => FSE_DECOMPRESS_WKSP_MIN + 64,
                    _ => full / 2,
                };
                diff_fse_decompress("fuzz", cap, &v, n, ml, w);
            }
        }
        // fully random blobs
        for _ in 0..500 {
            let n = 1 + rng.below(40);
            let v = rng.bytes(n);
            let ml = [0u32, 5, 9, 12, 13][rng.below(5)];
            let cap = [0usize, 1, 7, 64, 1024][rng.below(5)];
            diff_fse_decompress("fuzz-random", cap, &v, n, ml, full);
        }
    }
}

// ================= fse_compress.c : FSE_buildCTable_wksp (row 46)

/// Row 46: `FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize`
/// -> `tableLog_tooLarge`.
#[test]
fn err_fse_buildctable_wksp_tablelog_toolarge() {
    unsafe {
        let (fc, fr) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
        let src = skewed(4096, 0xC7B);
        let st = c_fse_stream(&src, 11).expect("fse stream");
        let (rnc, _) = duo::<FnReadNCount>("FSE_readNCount");
        let mut norm = vec![0i16; 256];
        let mut msv: c_uint = 255;
        let mut tl: c_uint = 0;
        assert!(!is_err(rnc(
            norm.as_mut_ptr(),
            &mut msv,
            &mut tl,
            st.blob.as_ptr() as *const c_void,
            st.blob.len()
        )));

        let ct_u32 = fse_ctable_size_u32(FSE_MAX_TABLELOG, 255) + 8;
        let need_u32 = fse_build_ctable_wksp_u32(msv, tl);
        for (label, w_u32) in [
            ("exact", need_u32),
            ("short1", need_u32 - 1),
            ("short2", need_u32 - 2),
            ("half", need_u32 / 2),
            ("zero", 0),
        ] {
            let (mut cc, mut cr) = twin32(ct_u32);
            let mut wc = vec![0u32; need_u32 + 8];
            let mut wr = vec![0u32; need_u32 + 8];
            let a = fc(
                cc.as_mut_ptr(),
                norm.as_ptr(),
                msv,
                tl,
                wc.as_mut_ptr() as *mut c_void,
                w_u32 * 4,
            );
            let b = fr(
                cr.as_mut_ptr(),
                norm.as_ptr(),
                msv,
                tl,
                wr.as_mut_ptr() as *mut c_void,
                w_u32 * 4,
            );
            eqcode(&format!("FSE_buildCTable_wksp {label}"), a, b);
            eqbuf(&format!("FSE_buildCTable_wksp {label} ctable"), as_bytes32(&cc), as_bytes32(&cr));
            eqbuf(&format!("FSE_buildCTable_wksp {label} wksp"), as_bytes32(&wc), as_bytes32(&wr));
            if label == "exact" {
                assert!(!is_err(a), "exact workspace should succeed");
            } else {
                assert_code(
                    &format!("FSE_buildCTable_wksp {label}"),
                    a,
                    E_tableLog_tooLarge,
                );
            }
        }
        // sweep every (msv, tableLog) with a workspace one U32 short
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            for m in [1u32, 15, 63, 128, 255] {
                let need = fse_build_ctable_wksp_u32(m, tl);
                let (mut cc, mut cr) = twin32(ct_u32);
                let mut wc = vec![0u32; need + 8];
                let mut wr = vec![0u32; need + 8];
                let a = fc(
                    cc.as_mut_ptr(),
                    norm.as_ptr(),
                    m,
                    tl,
                    wc.as_mut_ptr() as *mut c_void,
                    (need - 1) * 4,
                );
                let b = fr(
                    cr.as_mut_ptr(),
                    norm.as_ptr(),
                    m,
                    tl,
                    wr.as_mut_ptr() as *mut c_void,
                    (need - 1) * 4,
                );
                eqcode(&format!("FSE_buildCTable_wksp msv={m} tl={tl} short"), a, b);
                eqbuf("FSE_buildCTable_wksp sweep ctable", as_bytes32(&cc), as_bytes32(&cr));
                assert_code("FSE_buildCTable_wksp sweep", a, E_tableLog_tooLarge);
            }
        }
    }
}

// ============ fse_compress.c : FSE_writeNCount / _generic (rows 47-54)

#[track_caller]
unsafe fn diff_write_ncount(
    what: &str,
    cap: usize,
    norm: &[i16],
    msv: c_uint,
    tl: c_uint,
) -> usize {
    let (fc, fr) = duo::<FnWriteNCount>("FSE_writeNCount");
    let (mut dc, mut dr) = twin(cap.max(1) + 16);
    let a = fc(dc.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl);
    let b = fr(dr.as_mut_ptr() as *mut c_void, cap, norm.as_ptr(), msv, tl);
    eqcode(&format!("FSE_writeNCount {what}"), a, b);
    eqbuf(&format!("FSE_writeNCount {what} dst"), &dc, &dr);
    a
}

/// Rows 53/54: `tableLog > FSE_MAX_TABLELOG` -> `tableLog_tooLarge`;
/// `tableLog < FSE_MIN_TABLELOG` -> `GENERIC`.
/// Rows 47/48/50/52: the four `out > oend-2` buffer-overflow guards.
/// Rows 49/51: `remaining < 1` / `remaining != 1` -> `GENERIC`.
#[test]
fn err_fse_writencount_all() {
    unsafe {
        let (nb_c, nb_r) = duo::<FnNCountWriteBound>("FSE_NCountWriteBound");
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");

        // FSE_NCountWriteBound is a pure function; compare it everywhere first.
        for tl in 0u32..=20 {
            for msv in [0u32, 1, 15, 63, 127, 128, 255, 256, 1000] {
                eqv(
                    &format!("FSE_NCountWriteBound({msv},{tl})"),
                    nb_c(msv, tl),
                    nb_r(msv, tl),
                );
            }
        }

        // --- rows 53/54: tableLog out of range (checked before anything else)
        let zero = vec![0i16; 512];
        for tl in [0u32, 1, 2, 3, 4] {
            // tableLog == 0 is remapped? No: FSE_writeNCount rejects < FSE_MIN_TABLELOG.
            let r = diff_write_ncount(&format!("tl={tl}"), 4096, &zero, 255, tl);
            assert_code(&format!("FSE_writeNCount tl={tl}"), r, E_GENERIC);
        }
        for tl in [13u32, 14, 15, 16, 31, 255] {
            let r = diff_write_ncount(&format!("tl={tl}"), 4096, &zero, 255, tl);
            assert_code(&format!("FSE_writeNCount tl={tl}"), r, E_tableLog_tooLarge);
        }

        // --- row 49: `remaining < 1` -> GENERIC.
        // norm[0] == tableSize+1 consumes the whole table plus the accuracy slot.
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            let mut n = vec![0i16; 512];
            n[0] = (1i32 << tl) as i16 + 1;
            let r = diff_write_ncount(&format!("remaining<1 tl={tl}"), 65536, &n, 1, tl);
            assert_code(&format!("FSE_writeNCount remaining<1 tl={tl}"), r, E_GENERIC);
        }

        // --- row 51: `remaining != 1` at the end -> GENERIC (all-zero counts)
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            for msv in [1u32, 2, 15, 255] {
                let r = diff_write_ncount(
                    &format!("remaining!=1 tl={tl} msv={msv}"),
                    65536,
                    &zero,
                    msv,
                    tl,
                );
                assert_code("FSE_writeNCount all-zero counts", r, E_GENERIC);
            }
        }

        // --- rows 47/48/50/52: dstSize_tooSmall.
        // Use a *valid* distribution with a long run of zero counts (symbols 1..254)
        // so the `symbol >= start+24` fast-skip loop (row 47) is exercised too.
        let mut data = Vec::new();
        for i in 0..4096 {
            data.push(if i % 5 == 0 { 255u8 } else { 0u8 });
        }
        let st = c_fse_stream(&data, 11).expect("sparse fse stream");
        let (rnc, _) = duo::<FnReadNCount>("FSE_readNCount");
        let mut norm = vec![0i16; 512];
        let mut msv: c_uint = 255;
        let mut tl: c_uint = 0;
        assert!(!is_err(rnc(
            norm.as_mut_ptr(),
            &mut msv,
            &mut tl,
            st.blob.as_ptr() as *const c_void,
            st.blob.len()
        )));
        assert_eq!(msv, 255, "expected symbol 255 to be the top symbol");
        let bound = nb_c(msv, tl);
        let full = diff_write_ncount("sparse full", bound + 64, &norm, msv, tl);
        assert!(!is_err(full));
        let mut saw_dst = false;
        for cap in 0..=bound {
            let r = diff_write_ncount(&format!("sparse cap={cap}"), cap, &norm, msv, tl);
            if is_err(r) {
                assert_eq!(gc(r), E_dstSize_tooSmall, "cap={cap}: unexpected code");
                saw_dst = true;
            }
        }
        assert!(saw_dst, "rows 47/48/50/52 (dstSize_tooSmall) never reached");

        // A dense distribution as well (row 50 = the main-body bitCount>16 flush)
        let dense = skewed(4096, 0xD3E5A);
        let st2 = c_fse_stream(&dense, 12).expect("dense fse stream");
        let mut n2 = vec![0i16; 512];
        let mut m2: c_uint = 255;
        let mut t2: c_uint = 0;
        assert!(!is_err(rnc(
            n2.as_mut_ptr(),
            &mut m2,
            &mut t2,
            st2.blob.as_ptr() as *const c_void,
            st2.blob.len()
        )));
        let b2 = nb_c(m2, t2);
        for cap in 0..=b2 {
            diff_write_ncount(&format!("dense cap={cap}"), cap, &n2, m2, t2);
        }
    }
}

// ============ fse_compress.c : FSE_normalizeCount / M2 (rows 55-58)

#[track_caller]
unsafe fn diff_normalize(
    what: &str,
    tl: c_uint,
    count: &[c_uint],
    total: usize,
    msv: c_uint,
    lowprob: c_uint,
) -> usize {
    let (fc, fr) = duo::<FnNormalizeCount>("FSE_normalizeCount");
    let (mut nc, mut nr) = twin16(8192);
    let a = fc(nc.as_mut_ptr(), tl, count.as_ptr(), total, msv, lowprob);
    let b = fr(nr.as_mut_ptr(), tl, count.as_ptr(), total, msv, lowprob);
    eqcode(&format!("FSE_normalizeCount {what}"), a, b);
    eqbuf(&format!("FSE_normalizeCount {what} norm"), as_bytes16(&nc), as_bytes16(&nr));
    a
}

/// Rows 56-58: `tableLog < FSE_MIN_TABLELOG` -> `GENERIC`;
/// `tableLog > FSE_MAX_TABLELOG` -> `tableLog_tooLarge`;
/// `tableLog < FSE_minTableLog(total, maxSymbolValue)` -> `GENERIC`.
///
/// NOTE the documented precondition `total > 1` (`assert(srcSize > 1)`,
/// fse_compress.c L351/L362) and `maxSymbolValue >= 1`: violating either
/// reaches `ZSTD_highbit32(0)`, which is UB in both libraries, so those inputs
/// are deliberately out of scope.
#[test]
fn err_fse_normalizecount_guards() {
    unsafe {
        let mut count = vec![0u32; 4096];
        for i in 0..4096usize {
            count[i] = (i as u32 % 7) + 1;
        }
        let total: usize = count[..256].iter().map(|&x| x as usize).sum();

        // --- row 56: tableLog < FSE_MIN_TABLELOG (tableLog 0 is remapped to
        //     FSE_DEFAULT_TABLELOG *before* the check, so it must NOT error).
        for tl in [1u32, 2, 3, 4] {
            let r = diff_normalize(&format!("tl={tl}"), tl, &count, total, 255, 1);
            assert_code(&format!("FSE_normalizeCount tl={tl}"), r, E_GENERIC);
        }
        let r0 = diff_normalize("tl=0 (default)", 0, &count, total, 255, 1);
        assert!(!is_err(r0), "tableLog 0 must be remapped to the default");
        eqv("FSE_normalizeCount tl=0 result", r0, FSE_DEFAULT_TABLELOG as usize);

        // --- row 57: tableLog > FSE_MAX_TABLELOG
        for tl in [13u32, 14, 15, 16, 31, 63, 255, 0xFFFF] {
            let r = diff_normalize(&format!("tl={tl}"), tl, &count, total, 255, 1);
            assert_code(&format!("FSE_normalizeCount tl={tl}"), r, E_tableLog_tooLarge);
        }

        // --- row 58: tableLog < FSE_minTableLog(total, maxSymbolValue)
        // minBits = min(highbit32(total)+1, highbit32(msv)+2); both must exceed
        // tableLog, so pick a large total and a large maxSymbolValue.
        for (tl, tot, msv) in [
            (5u32, 1000usize, 200u32),
            (5, 100_000, 255),
            (6, 100_000, 255),
            (7, 1_000_000, 255),
            (8, 1_000_000, 255),
            (9, 1_000_000, 511),
            (10, 1_000_000, 1023),
            (11, 1_000_000, 2047),
            (12, 1_000_000, 4095),
        ] {
            let r = diff_normalize(
                &format!("minTableLog tl={tl} total={tot} msv={msv}"),
                tl,
                &count,
                tot,
                msv,
                1,
            );
            assert_code("FSE_normalizeCount minTableLog", r, E_GENERIC);
        }
        // and the boundary, which must *not* error
        for (tl, tot, msv) in [
            (9u32, 1000usize, 200u32),
            (10, 100_000, 255),
            (9, 1_000_000, 255),
            (10, 1_000_000, 511),
            (12, 1_000_000, 2047),
        ] {
            diff_normalize(
                &format!("minTableLog ok tl={tl} total={tot} msv={msv}"),
                tl,
                &count,
                tot,
                msv,
                1,
            );
        }
    }
}

/// Row 55: `FSE_normalizeM2` -> `GENERIC` (`weight < 1`).
///
/// Only reachable because `FSE_normalizeCount` truncates `total` to `U32` when
/// dividing (`ZSTD_div64(..., (U32)total)`, fse_compress.c L447/L479): with
/// `total == 2^32 + 545` the scaled step `rStep` is computed against 545 while
/// the residual `total` is ~2^32, so `count[s] * rStep` wraps around 2^64 and
/// `sEnd - sStart` collapses to 0. `total == 2^32` exactly would divide by zero
/// (UB in both libraries) and is excluded.
#[test]
fn err_fse_normalizem2_generic() {
    unsafe {
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        // brute-forced minimal witness (msv == 1)
        let mut count = vec![0u32; 4096];
        count[0] = 2_482_120_768;
        count[1] = 3_880_001_444;
        let total: usize = (1usize << 32) + 545;
        let r = diff_normalize("M2 witness", 6, &count, total, 1, 1);
        assert_code("FSE_normalizeM2 witness", r, E_GENERIC);

        // a wider randomised sweep in the same regime; every result must match
        let mut rng = Rng::new(0xF5E_0055);
        let mut hits = 0;
        for _ in 0..4000 {
            let msv = 1 + rng.below(64) as u32;
            let tl = 5 + rng.below(8) as u32;
            let mut c = vec![0u32; 4096];
            for i in 0..=msv as usize {
                c[i] = rng.next_u32();
            }
            // never let (U32)total be 0 (that is a division by zero in the C)
            let lo = 1 + (rng.next_u32() % 0xFFFF_FFFE);
            let total = (1usize << 32) + lo as usize;
            let lp = (rng.next_u32() & 1) as c_uint;
            let r = diff_normalize("M2 sweep", tl, &c, total, msv, lp);
            if is_err(r) && gc(r) == E_GENERIC {
                hits += 1;
            }
        }
        assert!(hits > 0, "randomised M2 sweep produced no GENERIC");
    }
}

/// `FSE_optimalTableLog` / `_internal` inside their documented domain
/// (`srcSize > 1`, `maxSymbolValue >= 1`): pure functions, must agree exactly.
#[test]
fn err_fse_optimaltablelog_domain() {
    unsafe {
        let (oc, or) = duo::<FnOptimalTableLog>("FSE_optimalTableLog");
        let (ic, ir) = duo::<FnOptimalTableLogInt>("FSE_optimalTableLog_internal");
        for mtl in [0u32, 1, 4, 5, 6, 9, 11, 12, 13, 20, 0xFFFF] {
            for n in [2usize, 3, 7, 100, 1000, 1 << 16, 1 << 20, usize::MAX / 2] {
                for msv in [1u32, 2, 15, 63, 255, 256, 1000] {
                    eqv(
                        &format!("FSE_optimalTableLog({mtl},{n},{msv})"),
                        oc(mtl, n, msv),
                        or(mtl, n, msv),
                    );
                    for minus in [0u32, 1, 2, 3] {
                        eqv(
                            &format!("FSE_optimalTableLog_internal({mtl},{n},{msv},{minus})"),
                            ic(mtl, n, msv, minus),
                            ir(mtl, n, msv, minus),
                        );
                    }
                }
            }
        }
    }
}

// ================================== hist.c (rows 59-63)

/// `HIST_count*_wksp` with a deliberately misaligned / undersized workspace,
/// and `HIST_count*` with a `maxSymbolValue` smaller than the data alphabet.
///
/// The workspace pointer is offset inside an over-allocated buffer so that both
/// libraries see the *same* `(size_t)workSpace & 3` value.
#[track_caller]
unsafe fn diff_hist_wksp(
    name: &str,
    what: &str,
    src: &[u8],
    msv_in: c_uint,
    off: usize,
    wksp_bytes: usize,
) -> usize {
    let (fc, fr) = duo::<FnHistCountWksp>(name);
    let (mut cc, mut cr) = twin32(256);
    // 8-byte aligned base (Vec<u64>) + explicit byte offset
    let mut wc = vec![0u64; wksp_bytes / 8 + 8];
    let mut wr = vec![0u64; wksp_bytes / 8 + 8];
    let pc = (wc.as_mut_ptr() as *mut u8).add(off);
    let pr = (wr.as_mut_ptr() as *mut u8).add(off);
    assert_eq!(
        (pc as usize) & 7,
        (pr as usize) & 7,
        "workspace alignment differs between libraries"
    );
    let mut mc: c_uint = msv_in;
    let mut mr: c_uint = msv_in;
    let a = fc(
        cc.as_mut_ptr(),
        &mut mc,
        src.as_ptr() as *const c_void,
        src.len(),
        pc as *mut c_void,
        wksp_bytes,
    );
    let b = fr(
        cr.as_mut_ptr(),
        &mut mr,
        src.as_ptr() as *const c_void,
        src.len(),
        pr as *mut c_void,
        wksp_bytes,
    );
    eqcode(&format!("{name} {what}"), a, b);
    eqv(&format!("{name} {what} *maxSymbolValuePtr"), mc, mr);
    eqbuf(&format!("{name} {what} count"), as_bytes32(&cc), as_bytes32(&cr));
    a
}

/// Rows 60/61/62/63: `GENERIC` (workspace not 4-byte aligned) and
/// `workSpace_tooSmall` (`workSpaceSize < HIST_WKSP_SIZE`).
#[test]
fn err_hist_wksp_alignment_and_size() {
    unsafe {
        // `HIST_countFast_wksp` only reaches the guards when sourceSize >= 1500
        // (below that it delegates to HIST_count_simple).
        let big = gen_class(4, 4096, 0x11157);
        let small = gen_class(4, 700, 0x11158);

        for off in [1usize, 2, 3, 5, 6, 7] {
            // --- rows 60 / 62: GENERIC
            let r = diff_hist_wksp(
                "HIST_countFast_wksp",
                &format!("misaligned off={off}"),
                &big,
                255,
                off,
                HIST_WKSP_SIZE,
            );
            assert_code("HIST_countFast_wksp misaligned", r, E_GENERIC);
            let r = diff_hist_wksp(
                "HIST_count_wksp",
                &format!("misaligned off={off}"),
                &big,
                255,
                off,
                HIST_WKSP_SIZE,
            );
            assert_code("HIST_count_wksp misaligned", r, E_GENERIC);
            // sourceSize < 1500: countFast_wksp short-circuits, count_wksp does not
            let r = diff_hist_wksp(
                "HIST_countFast_wksp",
                &format!("misaligned small off={off}"),
                &small,
                255,
                off,
                HIST_WKSP_SIZE,
            );
            assert!(!is_err(r), "countFast_wksp(<1500) must ignore the workspace");
            let r = diff_hist_wksp(
                "HIST_count_wksp",
                &format!("misaligned small off={off}"),
                &small,
                255,
                off,
                HIST_WKSP_SIZE,
            );
            assert_code("HIST_count_wksp misaligned small", r, E_GENERIC);
        }

        // --- rows 61 / 63: workSpace_tooSmall (aligned but short)
        for w in [0usize, 4, 256, 1024, HIST_WKSP_SIZE - 4, HIST_WKSP_SIZE - 1] {
            let r = diff_hist_wksp(
                "HIST_countFast_wksp",
                &format!("wksp={w}"),
                &big,
                255,
                0,
                w,
            );
            assert_code("HIST_countFast_wksp short wksp", r, E_workSpace_tooSmall);
            let r = diff_hist_wksp("HIST_count_wksp", &format!("wksp={w}"), &big, 255, 0, w);
            assert_code("HIST_count_wksp short wksp", r, E_workSpace_tooSmall);
        }
        // exact size succeeds identically
        for name in ["HIST_countFast_wksp", "HIST_count_wksp"] {
            let r = diff_hist_wksp(name, "exact", &big, 255, 0, HIST_WKSP_SIZE);
            assert!(!is_err(r), "{name}: exact HIST_WKSP_SIZE must succeed");
        }
        // ordering: misaligned *and* short -> GENERIC wins
        for name in ["HIST_countFast_wksp", "HIST_count_wksp"] {
            let r = diff_hist_wksp(name, "misaligned+short", &big, 255, 1, 16);
            assert_code(&format!("{name} misaligned+short"), r, E_GENERIC);
        }
    }
}

/// Row 59: `HIST_count_parallel_wksp` -> `maxSymbolValue_tooSmall`.
#[test]
fn err_hist_maxsymbolvalue_toosmall() {
    unsafe {
        let (hc, hr) = duo::<FnHistCount>("HIST_count");
        // Data whose alphabet is exactly [0, top].
        for top in [1u8, 3, 15, 63, 127, 200, 255] {
            let mut src = Vec::new();
            for i in 0..4096usize {
                src.push(((i as u32) % (top as u32 + 1)) as u8);
            }
            for msv in [0u32, 1, 2, 7, 31, 63, 100, 127, 200, 254] {
                if msv >= top as u32 {
                    continue;
                }
                // HIST_count (internal workspace)
                let (mut cc, mut cr) = twin32(256);
                let mut mc: c_uint = msv;
                let mut mr: c_uint = msv;
                let a = hc(
                    cc.as_mut_ptr(),
                    &mut mc,
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                let b = hr(
                    cr.as_mut_ptr(),
                    &mut mr,
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                eqcode(&format!("HIST_count top={top} msv={msv}"), a, b);
                eqv(&format!("HIST_count top={top} msv={msv} *msvPtr"), mc, mr);
                eqbuf("HIST_count count", as_bytes32(&cc), as_bytes32(&cr));
                assert_code("HIST_count msv too small", a, E_maxSymbolValue_tooSmall);

                // HIST_count_wksp (external workspace, well-formed)
                let r = diff_hist_wksp(
                    "HIST_count_wksp",
                    &format!("top={top} msv={msv}"),
                    &src,
                    msv,
                    0,
                    HIST_WKSP_SIZE,
                );
                assert_code("HIST_count_wksp msv too small", r, E_maxSymbolValue_tooSmall);
            }
            // msv == top must succeed, and msv == 255 too
            for msv in [top as u32, 255] {
                let r = diff_hist_wksp(
                    "HIST_count_wksp",
                    &format!("ok top={top} msv={msv}"),
                    &src,
                    msv,
                    0,
                    HIST_WKSP_SIZE,
                );
                assert!(!is_err(r), "top={top} msv={msv} should succeed");
            }
        }
        // empty source: no symbol can be out of range
        for msv in [0u32, 1, 255] {
            let r = diff_hist_wksp(
                "HIST_count_wksp",
                &format!("empty msv={msv}"),
                &[],
                msv,
                0,
                HIST_WKSP_SIZE,
            );
            assert!(!is_err(r));
        }
        // HIST_isError agrees
        let (iec, ier) = duo::<unsafe extern "C" fn(usize) -> c_uint>("HIST_isError");
        for v in [0usize, 1, 100, usize::MAX, usize::MAX - 1, usize::MAX - 47, usize::MAX - 129, usize::MAX - 130] {
            eqv(&format!("HIST_isError({v:#x})"), iec(v), ier(v));
        }
    }
}

// ========================= entropy_common.c : HUF_readStats (rows 10-18)

#[track_caller]
unsafe fn diff_read_stats(what: &str, src: &[u8], src_size: usize, hw_size: usize) -> usize {
    let (fc, fr) = duo::<FnHufReadStats>("HUF_readStats");
    let (mut hwc, mut hwr) = twin(512);
    let (mut rkc, mut rkr) = twin32(32);
    let mut nsc = 0xDEAD_BEEFu32;
    let mut nsr = 0xDEAD_BEEFu32;
    let mut tlc = 0xDEAD_BEEFu32;
    let mut tlr = 0xDEAD_BEEFu32;
    let a = fc(
        hwc.as_mut_ptr(),
        hw_size,
        rkc.as_mut_ptr(),
        &mut nsc,
        &mut tlc,
        src.as_ptr() as *const c_void,
        src_size,
    );
    let b = fr(
        hwr.as_mut_ptr(),
        hw_size,
        rkr.as_mut_ptr(),
        &mut nsr,
        &mut tlr,
        src.as_ptr() as *const c_void,
        src_size,
    );
    eqcode(&format!("HUF_readStats {what}"), a, b);
    eqv(&format!("HUF_readStats {what} *nbSymbolsPtr"), nsc, nsr);
    eqv(&format!("HUF_readStats {what} *tableLogPtr"), tlc, tlr);
    eqbuf(&format!("HUF_readStats {what} huffWeight"), &hwc, &hwr);
    eqbuf(&format!("HUF_readStats {what} rankStats"), as_bytes32(&rkc), as_bytes32(&rkr));
    a
}

const HUF_FLAG_SET: [c_int; 14] = [
    0,
    HUF_flags_bmi2,
    HUF_flags_optimalDepth,
    HUF_flags_preferRepeat,
    HUF_flags_suspectUncompressible,
    HUF_flags_disableAsm,
    HUF_flags_disableFast,
    HUF_flags_disableAsm | HUF_flags_disableFast,
    HUF_flags_bmi2 | HUF_flags_optimalDepth,
    0x3F,
    // out-of-range bitmasks crossing the FFI boundary
    0x40,
    1 << 30,
    -1,
    i32::MIN,
];

#[track_caller]
unsafe fn diff_read_stats_wksp(
    what: &str,
    src: &[u8],
    src_size: usize,
    hw_size: usize,
    wksp_bytes: usize,
) -> usize {
    let (fc, fr) = duo::<FnHufReadStatsWksp>("HUF_readStats_wksp");
    let mut out = usize::MAX;
    for flags in HUF_FLAG_SET {
        let (mut hwc, mut hwr) = twin(512);
        let (mut rkc, mut rkr) = twin32(32);
        let mut wc = vec![0u32; wksp_bytes / 4 + 8];
        let mut wr = vec![0u32; wksp_bytes / 4 + 8];
        let mut nsc = 0xDEAD_BEEFu32;
        let mut nsr = 0xDEAD_BEEFu32;
        let mut tlc = 0xDEAD_BEEFu32;
        let mut tlr = 0xDEAD_BEEFu32;
        let a = fc(
            hwc.as_mut_ptr(),
            hw_size,
            rkc.as_mut_ptr(),
            &mut nsc,
            &mut tlc,
            src.as_ptr() as *const c_void,
            src_size,
            wc.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        let b = fr(
            hwr.as_mut_ptr(),
            hw_size,
            rkr.as_mut_ptr(),
            &mut nsr,
            &mut tlr,
            src.as_ptr() as *const c_void,
            src_size,
            wr.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        eqcode(&format!("HUF_readStats_wksp(flags={flags:#x}) {what}"), a, b);
        eqv(&format!("HUF_readStats_wksp(flags={flags:#x}) {what} nbSymbols"), nsc, nsr);
        eqv(&format!("HUF_readStats_wksp(flags={flags:#x}) {what} tableLog"), tlc, tlr);
        eqbuf(&format!("HUF_readStats_wksp(flags={flags:#x}) {what} hw"), &hwc, &hwr);
        eqbuf(
            &format!("HUF_readStats_wksp(flags={flags:#x}) {what} rank"),
            as_bytes32(&rkc),
            as_bytes32(&rkr),
        );
        eqbuf(
            &format!("HUF_readStats_wksp(flags={flags:#x}) {what} wksp"),
            as_bytes32(&wc),
            as_bytes32(&wr),
        );
        out = a;
    }
    out
}

/// Rows 10-18, every `HUF_readStats` rejection, one targeted input each.
#[test]
fn err_huf_readstats_targeted() {
    unsafe {
        // --- row 10: srcSize == 0 -> srcSize_wrong
        let r = diff_read_stats("srcSize=0", &[0u8; 4], 0, 512);
        assert_code("HUF_readStats srcSize=0", r, E_srcSize_wrong);
        let r = diff_read_stats_wksp("srcSize=0", &[0u8; 4], 0, 512, 4096);
        assert_code("HUF_readStats_wksp srcSize=0", r, E_srcSize_wrong);

        // --- row 11: special header, iSize+1 > srcSize -> srcSize_wrong
        for osize in [2usize, 4, 10, 40, 128] {
            let w: Vec<u8> = (0..osize).map(|i| (i % 5) as u8 + 1).collect();
            let hdr = raw_weight_header(&w);
            for n in 1..hdr.len() {
                let r = diff_read_stats(&format!("special truncated osize={osize} n={n}"), &hdr, n, 512);
                assert_code("HUF_readStats special truncated", r, E_srcSize_wrong);
            }
        }

        // --- row 12: special header, oSize >= hwSize -> corruption_detected
        for osize in [4usize, 10, 40, 128] {
            let w: Vec<u8> = (0..osize).map(|i| (i % 5) as u8 + 1).collect();
            let hdr = raw_weight_header(&w);
            for hw in [0usize, 1, 2, osize - 1, osize] {
                let r = diff_read_stats(
                    &format!("special hwSize={hw} osize={osize}"),
                    &hdr,
                    hdr.len(),
                    hw,
                );
                assert_code("HUF_readStats oSize >= hwSize", r, E_corruption_detected);
            }
        }

        // --- row 13: FSE-coded header, iSize+1 > srcSize -> srcSize_wrong
        for isize_ in [1u8, 2, 5, 40, 127] {
            let mut hdr = vec![0u8; isize_ as usize + 8];
            hdr[0] = isize_;
            for n in 1..=(isize_ as usize) {
                let r = diff_read_stats(&format!("fse truncated iSize={isize_} n={n}"), &hdr, n, 512);
                assert_code("HUF_readStats fse truncated", r, E_srcSize_wrong);
            }
        }
        // iSize == 0 means "0 bytes of FSE payload": FSE_decompress_wksp(srcSize 0)
        let r = diff_read_stats("iSize=0", &[0u8, 0, 0, 0], 4, 512);
        assert!(is_err(r), "iSize == 0 must fail");

        // --- row 14: a weight > HUF_TABLELOG_MAX -> corruption_detected
        for bad in 13u8..16 {
            for pos in [0usize, 1, 3] {
                let mut w = vec![1u8, 1, 2, 2, 3, 3];
                w[pos] = bad;
                let hdr = raw_weight_header(&w);
                let r = diff_read_stats(&format!("weight {bad} at {pos}"), &hdr, hdr.len(), 512);
                assert_code("HUF_readStats weight too large", r, E_corruption_detected);
            }
        }

        // --- row 15: weightTotal == 0 -> corruption_detected
        for osize in [1usize, 2, 5, 16, 128] {
            let w = vec![0u8; osize];
            let hdr = raw_weight_header(&w);
            let r = diff_read_stats(&format!("all-zero weights osize={osize}"), &hdr, hdr.len(), 512);
            assert_code("HUF_readStats weightTotal==0", r, E_corruption_detected);
        }

        // --- row 16: tableLog > HUF_TABLELOG_MAX -> corruption_detected
        for w in [vec![12u8, 12], vec![12u8, 12, 12], vec![12u8, 12, 12, 12]] {
            let hdr = raw_weight_header(&w);
            let r = diff_read_stats(&format!("weightTotal overflow {w:?}"), &hdr, hdr.len(), 512);
            assert_code("HUF_readStats tableLog>12", r, E_corruption_detected);
        }

        // --- row 17: `rest` is not a clean power of two -> corruption_detected
        for w in [vec![12u8, 1], vec![12u8, 2], vec![11u8, 1, 2], vec![10u8, 3, 1]] {
            let hdr = raw_weight_header(&w);
            let r = diff_read_stats(&format!("dirty rest {w:?}"), &hdr, hdr.len(), 512);
            assert_code("HUF_readStats rest not pow2", r, E_corruption_detected);
        }

        // --- row 18: rankStats[1] < 2 or odd -> corruption_detected
        for w in [
            vec![2u8, 2],       // rankStats[1] == 0
            vec![3u8, 3, 2],    // rankStats[1] == 0
            vec![2u8, 1, 1, 1], // rankStats[1] odd (3)
        ] {
            let hdr = raw_weight_header(&w);
            let r = diff_read_stats(&format!("rank1 invalid {w:?}"), &hdr, hdr.len(), 512);
            assert_code("HUF_readStats rank1 invalid", r, E_corruption_detected);
        }

        // control: a well-formed raw header must succeed
        let hdr = raw_weight_header(&[1u8, 1]);
        let r = diff_read_stats("valid [1,1]", &hdr, hdr.len(), 512);
        assert!(!is_err(r), "valid raw header rejected: {r:#x}");
    }
}

/// Workspace one byte too small for `HUF_readStats_wksp` -> the `GENERIC`
/// returned by `FSE_decompress_wksp` (the FSE-coded header path).
#[test]
fn err_huf_readstats_wksp_toosmall() {
    unsafe {
        // A real HUF table description whose weights are FSE-coded.
        let src = skewed(16384, 0xA1B2);
        let hb = c_huf_blob(&src, 0, 0, false).expect("huf blob");
        assert!(hb.blob[0] < 128, "expected an FSE-coded weight header");
        let desc = &hb.blob[..hb.desc_len];

        // sanity: succeeds with the documented workspace size
        let r = diff_read_stats_wksp("full wksp", desc, desc.len(), 512, 4096);
        assert!(!is_err(r), "full workspace must succeed");

        for w in [0usize, 4, 128, FSE_DECOMPRESS_WKSP_MIN - 4, FSE_DECOMPRESS_WKSP_MIN - 1] {
            let r = diff_read_stats_wksp(&format!("wksp={w}"), desc, desc.len(), 512, w);
            assert_code("HUF_readStats_wksp short wksp", r, E_GENERIC);
        }
        // just enough for FSE_DecompressWksp but not for the DTable
        let mut saw_tl = false;
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        for w in [
            FSE_DECOMPRESS_WKSP_MIN,
            FSE_DECOMPRESS_WKSP_MIN + 4,
            FSE_DECOMPRESS_WKSP_MIN + 64,
            FSE_DECOMPRESS_WKSP_MIN + 256,
        ] {
            let r = diff_read_stats_wksp(&format!("wksp={w}"), desc, desc.len(), 512, w);
            if is_err(r) && gc(r) == E_tableLog_tooLarge {
                saw_tl = true;
            }
        }
        assert!(saw_tl, "workspace just short of the DTable never gave tableLog_tooLarge");

        // the special (raw) header path ignores the workspace entirely
        let raw = raw_weight_header(&[1u8, 1, 2, 2]);
        for w in [0usize, 4, 64] {
            diff_read_stats_wksp(&format!("raw hdr wksp={w}"), &raw, raw.len(), 512, w);
        }
    }
}

/// Randomised corruption of real HUF table descriptions, plus fully random
/// weight headers. Fixed seed.
#[test]
fn err_huf_readstats_fuzz() {
    unsafe {
        let mut rng = Rng::new(0x4055_0001);
        let mut descs: Vec<Vec<u8>> = Vec::new();
        for cls in 0..N_CLASSES {
            for tl in [0u32, 8, 10, 11, 12] {
                let d = to_alphabet(&gen_class(cls, 6000, 0x77 ^ tl as u64), 0);
                if let Some(hb) = c_huf_blob(&d, 0, tl, false) {
                    descs.push(hb.blob[..hb.desc_len].to_vec());
                }
            }
        }
        assert!(descs.len() >= 4, "not enough descriptions: {}", descs.len());

        for d in &descs {
            for _ in 0..60 {
                let mut v = d.clone();
                let nmut = 1 + rng.below(3);
                for _ in 0..nmut {
                    let i = rng.below(v.len());
                    let b = rng.byte();
                    v[i] = b;
                }
                let n = if rng.below(4) == 0 { 1 + rng.below(v.len()) } else { v.len() };
                let hw = [0usize, 1, 2, 16, 128, 255, 256, 512][rng.below(8)];
                diff_read_stats("fuzz-desc", &v, n, hw);
            }
        }
        for _ in 0..500 {
            let n = 1 + rng.below(48);
            let v = rng.bytes(n);
            let hw = [0usize, 1, 2, 16, 128, 256, 512][rng.below(7)];
            diff_read_stats("fuzz-random", &v, n, hw);
        }
        // random raw (special) headers with random nibble weights
        for _ in 0..400 {
            let osize = 1 + rng.below(128);
            let mut w = Vec::with_capacity(osize);
            for _ in 0..osize {
                w.push(rng.byte() & 15);
            }
            let hdr = raw_weight_header(&w);
            let hw = [0usize, 1, 2, 129, 256, 512][rng.below(6)];
            diff_read_stats("fuzz-raw", &hdr, hdr.len(), hw);
        }
    }
}

/// Restrict `data` to the alphabet `[0, msv]` (msv==0 means "auto" = 255).
fn to_alphabet(data: &[u8], msv: u32) -> Vec<u8> {
    let m = if msv == 0 { 255 } else { msv };
    data.iter().map(|&b| (b as u32 % (m + 1)) as u8).collect()
}

// ============================== huf_compress.c (rows 64-80)

/// Build a valid `HUF_CElt` table with the **C** library. `flat` makes every
/// symbol use the same number of bits (so its weight table is RLE and
/// `HUF_compressWeights` returns 1 without touching `dst`).
unsafe fn c_ctable(msv: c_uint, max_nb_bits: u32, flat: bool, seed: u64) -> (Vec<u64>, u32) {
    let (bc, _) = duo::<FnHufBuildCTable>("HUF_buildCTable_wksp");
    let mut count = vec![0u32; 256];
    let mut rng = Rng::new(seed);
    for i in 0..=msv as usize {
        count[i] = if flat { 1 } else { 1 + (rng.next_u32() % 1000) };
    }
    let mut ct = vec![0u64; 258];
    let mut w = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE / 4 + 8];
    let r = bc(
        ct.as_mut_ptr(),
        count.as_ptr(),
        msv,
        max_nb_bits,
        w.as_mut_ptr() as *mut c_void,
        HUF_CTABLE_WORKSPACE_SIZE,
    );
    assert!(!is_err(r), "helper c_ctable failed: {r:#x}");
    (ct, r as u32)
}

#[track_caller]
unsafe fn diff_write_ctable(
    what: &str,
    cap: usize,
    ct: &[u64],
    msv: c_uint,
    hufLog: c_uint,
    wksp_bytes: usize,
    off: usize,
) -> usize {
    let (fc, fr) = duo::<FnHufWriteCTable>("HUF_writeCTable_wksp");
    let (mut dc, mut dr) = twin(cap.max(1) + 16);
    let mut wc = vec![0u64; wksp_bytes / 8 + 8];
    let mut wr = vec![0u64; wksp_bytes / 8 + 8];
    let pc = (wc.as_mut_ptr() as *mut u8).add(off);
    let pr = (wr.as_mut_ptr() as *mut u8).add(off);
    let a = fc(
        dc.as_mut_ptr() as *mut c_void,
        cap,
        ct.as_ptr(),
        msv,
        hufLog,
        pc as *mut c_void,
        wksp_bytes,
    );
    let b = fr(
        dr.as_mut_ptr() as *mut c_void,
        cap,
        ct.as_ptr(),
        msv,
        hufLog,
        pr as *mut c_void,
        wksp_bytes,
    );
    eqcode(&format!("HUF_writeCTable_wksp {what}"), a, b);
    eqbuf(&format!("HUF_writeCTable_wksp {what} dst"), &dc, &dr);
    eqbuf(
        &format!("HUF_writeCTable_wksp {what} wksp"),
        as_bytes64(&wc),
        as_bytes64(&wr),
    );
    a
}

/// Rows 64/66/67/68/69/70 — every `HUF_writeCTable_wksp` rejection, plus the
/// `HUF_alignUpWorkspace` NULL return (row 64), which surfaces as the `GENERIC`
/// of row 66 because `*workspaceSizePtr` is zeroed.
#[test]
fn err_huf_writectable_wksp_all() {
    unsafe {
        // `sizeof(HUF_WriteCTableWksp)` is not exported; derive the exact
        // threshold by bisecting the C library's own answer.
        let (ct, tl) = c_ctable(63, 11, false, 0x9001);
        let mut lo = 0usize;
        let mut hi = HUF_CTABLE_WORKSPACE_SIZE;
        let (fc, _) = duo::<FnHufWriteCTable>("HUF_writeCTable_wksp");
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let mut probe = |n: usize| -> bool {
            let mut d = vec![0u8; 4096];
            let mut w = vec![0u64; HUF_CTABLE_WORKSPACE_SIZE / 8 + 8];
            let r = fc(
                d.as_mut_ptr() as *mut c_void,
                d.len(),
                ct.as_ptr(),
                63,
                tl,
                w.as_mut_ptr() as *mut c_void,
                n,
            );
            !(is_err(r) && gc(r) == E_GENERIC)
        };
        assert!(probe(hi), "full HUF_CTABLE_WORKSPACE_SIZE should be enough");
        assert!(!probe(lo));
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if probe(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let need = hi; // smallest workspace that is not rejected
        assert!(need > 8 && need <= HUF_CTABLE_WORKSPACE_SIZE, "need={need}");

        // --- row 66: workspaceSize < sizeof(HUF_WriteCTableWksp) -> GENERIC
        for w in [0usize, 1, 8, 64, need / 2, need - 1] {
            let r = diff_write_ctable(&format!("wksp={w}"), 4096, &ct, 63, tl, w, 0);
            assert_code("HUF_writeCTable_wksp short wksp", r, E_GENERIC);
        }
        // --- row 64: misaligned workspace with too little slack -> NULL -> GENERIC
        for off in [1usize, 2, 3] {
            for w in [0usize, 1, 2] {
                if w >= 4 - off {
                    continue;
                }
                let r = diff_write_ctable(
                    &format!("alignUp NULL off={off} wksp={w}"),
                    4096,
                    &ct,
                    63,
                    tl,
                    w,
                    off,
                );
                assert_code("HUF_alignUpWorkspace NULL", r, E_GENERIC);
            }
            // misaligned but large: the padding is charged against wkspSize
            let r = diff_write_ctable(
                &format!("alignUp pad off={off} exact"),
                4096,
                &ct,
                63,
                tl,
                need,
                off,
            );
            assert_code("HUF_writeCTable_wksp misaligned exact", r, E_GENERIC);
            let r = diff_write_ctable(
                &format!("alignUp pad off={off} exact+4"),
                4096,
                &ct,
                63,
                tl,
                need + 4,
                off,
            );
            assert!(!is_err(r), "off={off}: need+4 should be enough");
        }

        // --- row 67: maxSymbolValue > HUF_SYMBOLVALUE_MAX -> maxSymbolValue_tooLarge
        for bad in [256u32, 257, 1000, 0xFFFF, 0xFFFF_FFFF] {
            let r = diff_write_ctable(
                &format!("msv={bad}"),
                4096,
                &ct,
                bad,
                tl,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            assert_code("HUF_writeCTable_wksp msv too large", r, E_maxSymbolValue_tooLarge);
        }
        // ordering: the workspace check comes first
        let r = diff_write_ctable("msv=300 + tiny wksp", 4096, &ct, 300, tl, 8, 0);
        assert_code("HUF_writeCTable_wksp msv+tiny wksp", r, E_GENERIC);

        // --- row 68: maxDstSize < 1 -> dstSize_tooSmall
        let r = diff_write_ctable("cap=0", 0, &ct, 63, tl, HUF_CTABLE_WORKSPACE_SIZE, 0);
        assert_code("HUF_writeCTable_wksp cap=0", r, E_dstSize_tooSmall);

        // --- row 69: maxSymbolValue > 128 and the weights did not FSE-compress
        // (a flat CTable makes every weight identical -> HUF_compressWeights
        // returns 1 == "rle", so the raw-nibble fallback is taken).
        // Only a power-of-two alphabet yields a *uniform* code length for every
        // symbol, which is what makes HUF_compressWeights report "rle" (1).
        for msv in [255u32] {
            let (flat, ftl) = c_ctable(msv, 8, true, 0x9002);
            let r = diff_write_ctable(
                &format!("flat msv={msv}"),
                4096,
                &flat,
                msv,
                ftl,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            assert_code("HUF_writeCTable_wksp msv>128 raw fallback", r, E_GENERIC);
        }
        // non-power-of-two alphabets above 128 exercise the same call shape and
        // must still agree exactly
        for msv in [129u32, 150, 200, 254] {
            let (flat, ftl) = c_ctable(msv, 8, true, 0x9002);
            for cap in [1usize, 2, 8, 40, 130, 4096] {
                diff_write_ctable(
                    &format!("flat msv={msv} cap={cap}"),
                    cap,
                    &flat,
                    msv,
                    ftl,
                    HUF_CTABLE_WORKSPACE_SIZE,
                    0,
                );
            }
        }

        // --- row 70: ((maxSymbolValue+1)/2)+1 > maxDstSize -> dstSize_tooSmall
        for msv in [3u32, 7, 15, 31, 63, 127] {
            let (flat, ftl) = c_ctable(msv, 8, true, 0x9003);
            let need_dst = ((msv as usize + 1) / 2) + 1;
            for cap in 1..need_dst {
                let r = diff_write_ctable(
                    &format!("flat msv={msv} cap={cap}"),
                    cap,
                    &flat,
                    msv,
                    ftl,
                    HUF_CTABLE_WORKSPACE_SIZE,
                    0,
                );
                assert_code("HUF_writeCTable_wksp raw fallback dst", r, E_dstSize_tooSmall);
            }
            let r = diff_write_ctable(
                &format!("flat msv={msv} cap={need_dst}"),
                need_dst,
                &flat,
                msv,
                ftl,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            assert!(!is_err(r), "msv={msv}: exact dst size must succeed");
        }

        // full dstCapacity sweep on a normal (FSE-compressible) table
        for cap in 0..64usize {
            diff_write_ctable(
                &format!("sweep cap={cap}"),
                cap,
                &ct,
                63,
                tl,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
        }
    }
}

/// Row 72: `HUF_readCTable` -> `maxSymbolValue_tooSmall`.
///
/// Row 71 (`tableLog > HUF_TABLELOG_MAX` -> `tableLog_tooLarge`) is
/// **unreachable**: `HUF_readStats` already rejects `tableLog > 12` with
/// `corruption_detected` (entropy_common.c L288) before returning, so the
/// guard at huf_compress.c L305 is dead code.
#[test]
fn err_huf_readctable() {
    unsafe {
        let (fc, fr) = duo::<FnHufReadCTable>("HUF_readCTable");
        let (rs, _) = duo::<FnHufReadStats>("HUF_readStats");
        let src = to_alphabet(&gen_class(4, 20000, 0xCAFE), 0);
        let hb = c_huf_blob(&src, 0, 0, false).expect("huf blob");
        let desc = &hb.blob[..hb.desc_len];

        // how many symbols does this description describe?
        let mut hw = vec![0u8; 512];
        let mut rk = vec![0u32; 32];
        let mut ns = 0u32;
        let mut tl = 0u32;
        assert!(!is_err(rs(
            hw.as_mut_ptr(),
            512,
            rk.as_mut_ptr(),
            &mut ns,
            &mut tl,
            desc.as_ptr() as *const c_void,
            desc.len()
        )));
        assert!(ns >= 8, "not enough symbols in the description: {ns}");

        let mut probe = |what: &str, blob: &[u8], n: usize, msv_in: c_uint| -> usize {
            let (mut cc, mut cr) = twin64(258);
            let mut mc: c_uint = msv_in;
            let mut mr: c_uint = msv_in;
            let mut zc: c_uint = 0xDEAD_BEEF;
            let mut zr: c_uint = 0xDEAD_BEEF;
            let a = fc(
                cc.as_mut_ptr(),
                &mut mc,
                blob.as_ptr() as *const c_void,
                n,
                &mut zc,
            );
            let b = fr(
                cr.as_mut_ptr(),
                &mut mr,
                blob.as_ptr() as *const c_void,
                n,
                &mut zr,
            );
            eqcode(&format!("HUF_readCTable {what}"), a, b);
            eqv(&format!("HUF_readCTable {what} *maxSymbolValuePtr"), mc, mr);
            eqv(&format!("HUF_readCTable {what} *hasZeroWeights"), zc, zr);
            eqbuf(&format!("HUF_readCTable {what} CTable"), as_bytes64(&cc), as_bytes64(&cr));
            a
        };

        // --- row 72: nbSymbols > *maxSymbolValuePtr + 1
        for msv in 0..(ns - 1) {
            let r = probe(&format!("msv={msv} (ns={ns})"), desc, desc.len(), msv);
            assert_code("HUF_readCTable msv too small", r, E_maxSymbolValue_tooSmall);
        }
        // exactly enough -> success
        let r = probe(&format!("msv={} exact", ns - 1), desc, desc.len(), ns - 1);
        assert!(!is_err(r), "exact maxSymbolValue rejected");

        // srcSize 0/1/truncated, and corrupted descriptions
        for n in 0..desc.len() {
            probe(&format!("truncated n={n}"), desc, n, 255);
        }
        let mut rng = Rng::new(0x7C2);
        for _ in 0..400 {
            let mut v = desc.to_vec();
            let nmut = 1 + rng.below(3);
            for _ in 0..nmut {
                let i = rng.below(v.len());
                let b = rng.byte();
                v[i] = b;
            }
            let msv = [0u32, 1, 15, 63, 127, 255, 256, 1000][rng.below(8)];
            probe("fuzz", &v, v.len(), msv);
        }
        // raw (special) headers, including ones with tableLog exactly 12
        for w in [
            vec![1u8, 1],
            vec![1u8, 1, 1, 1],
            vec![12u8, 11, 11],
            vec![4u8, 4, 3, 2, 1, 1],
        ] {
            let hdr = raw_weight_header(&w);
            for msv in [0u32, 1, 2, 5, 255] {
                probe(&format!("raw {w:?} msv={msv}"), &hdr, hdr.len(), msv);
            }
        }
    }
}

/// Rows 73/74/75 — `HUF_buildCTable_wksp`.
#[test]
fn err_huf_buildctable_wksp_all() {
    unsafe {
        let (fc, fr) = duo::<FnHufBuildCTable>("HUF_buildCTable_wksp");
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");

        let mut probe = |what: &str,
                         count: &[c_uint],
                         msv: u32,
                         mnb: u32,
                         wksp_bytes: usize,
                         off: usize|
         -> usize {
            let (mut cc, mut cr) = twin64(258);
            let mut wc = vec![0u64; wksp_bytes / 8 + 8];
            let mut wr = vec![0u64; wksp_bytes / 8 + 8];
            let pc = (wc.as_mut_ptr() as *mut u8).add(off);
            let pr = (wr.as_mut_ptr() as *mut u8).add(off);
            let a = fc(
                cc.as_mut_ptr(),
                count.as_ptr(),
                msv,
                mnb,
                pc as *mut c_void,
                wksp_bytes,
            );
            let b = fr(
                cr.as_mut_ptr(),
                count.as_ptr(),
                msv,
                mnb,
                pr as *mut c_void,
                wksp_bytes,
            );
            eqcode(&format!("HUF_buildCTable_wksp {what}"), a, b);
            eqbuf(
                &format!("HUF_buildCTable_wksp {what} CTable"),
                as_bytes64(&cc),
                as_bytes64(&cr),
            );
            eqbuf(
                &format!("HUF_buildCTable_wksp {what} wksp"),
                as_bytes64(&wc),
                as_bytes64(&wr),
            );
            a
        };

        let mut count = vec![0u32; 512];
        for i in 0..256usize {
            count[i] = (i as u32 % 13) + 1;
        }

        // --- row 73: wkspSize < sizeof(HUF_buildCTable_wksp_tables)
        // == HUF_CTABLE_WORKSPACE_SIZE (statically asserted in the C).
        for w in [
            0usize,
            8,
            1024,
            HUF_CTABLE_WORKSPACE_SIZE / 2,
            HUF_CTABLE_WORKSPACE_SIZE - 8,
            HUF_CTABLE_WORKSPACE_SIZE - 1,
        ] {
            let r = probe(&format!("wksp={w}"), &count, 255, 11, w, 0);
            assert_code("HUF_buildCTable_wksp short wksp", r, E_workSpace_tooSmall);
        }
        let r = probe("wksp exact", &count, 255, 11, HUF_CTABLE_WORKSPACE_SIZE, 0);
        assert!(!is_err(r), "exact HUF_CTABLE_WORKSPACE_SIZE must succeed");
        // row 64 again: misaligned with no slack -> NULL -> workSpace_tooSmall
        for off in [1usize, 2, 3] {
            let r = probe(&format!("alignUp NULL off={off}"), &count, 255, 11, 0, off);
            assert_code("HUF_alignUpWorkspace NULL (buildCTable)", r, E_workSpace_tooSmall);
            let r = probe(
                &format!("misaligned exact off={off}"),
                &count,
                255,
                11,
                HUF_CTABLE_WORKSPACE_SIZE,
                off,
            );
            assert_code("HUF_buildCTable_wksp misaligned exact", r, E_workSpace_tooSmall);
        }

        // --- row 74: maxSymbolValue > HUF_SYMBOLVALUE_MAX
        for bad in [256u32, 257, 511] {
            let r = probe(
                &format!("msv={bad}"),
                &count,
                bad,
                11,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            assert_code("HUF_buildCTable_wksp msv too large", r, E_maxSymbolValue_tooLarge);
        }
        // ordering: workspace first
        let r = probe("msv=300 + tiny wksp", &count, 300, 11, 8, 0);
        assert_code("HUF_buildCTable_wksp msv+tiny wksp", r, E_workSpace_tooSmall);

        // --- row 75: the resulting tree is deeper than HUF_TABLELOG_MAX -> GENERIC.
        // Fibonacci counts give a maximally unbalanced tree (depth == nbSymbols-1);
        // asking for maxNbBits == 13 lets a depth-13 tree through HUF_setMaxHeight,
        // and the `maxNbBits > HUF_TABLELOG_MAX` guard then fires.
        let mut saw = false;
        for nsym in 14..=40usize {
            let mut c = vec![0u32; 512];
            let (mut a, mut b) = (1u32, 1u32);
            for i in 0..nsym {
                c[i] = a;
                let n = a + b;
                a = b;
                b = n;
            }
            let r = probe(
                &format!("fib nsym={nsym} maxNbBits=13"),
                &c,
                (nsym - 1) as u32,
                13,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            if is_err(r) && gc(r) == E_GENERIC {
                saw = true;
            }
            // maxNbBits inside the legal range must not trip it.
            // (`maxNbBits` in 1..=4 is *below* the tree depth and makes
            // HUF_setMaxHeight fall through to `huffNode[rankLast[13]]` with
            // rankLast[13] == 0xF0F0F0F0 (huf_compress.c L443) — UB that
            // segfaults both libraries, so it is excluded.)
            for mnb in [0u32, 11, 12] {
                probe(
                    &format!("fib nsym={nsym} maxNbBits={mnb}"),
                    &c,
                    (nsym - 1) as u32,
                    mnb,
                    HUF_CTABLE_WORKSPACE_SIZE,
                    0,
                );
            }
        }
        assert!(saw, "row 75 (HUF_buildCTable_wksp GENERIC) was never reached");
        // `maxNbBits > HUF_TABLELOG_MAX + 1` is only safe when the tree is
        // already shallower than `maxNbBits`: otherwise HUF_setMaxHeight indexes
        // `U32 rankLast[HUF_TABLELOG_MAX+2]` at `targetNbBits-currentNbBits`
        // (huf_compress.c L411/L420), i.e. out of bounds — UB that crashes BOTH
        // libraries. So only shallow trees are probed here.
        for mnb in [14u32, 16, 20, 32, 0xFFFF] {
            let mut c = vec![0u32; 512];
            let (mut a, mut b) = (1u32, 1u32);
            for i in 0..8usize {
                c[i] = a;
                let n = a + b;
                a = b;
                b = n;
            }
            probe(
                &format!("shallow fib maxNbBits={mnb}"),
                &c,
                7,
                mnb,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
        }
        // NOTE: an all-zero `count[]` (cardinality 0) is out of the documented
        // domain — HUF_buildTree then leaves `nonNullRank` at -1 and
        // HUF_buildCTableFromTree walks off `huffNode`, segfaulting BOTH
        // libraries. Only non-degenerate histograms are probed.
        for msv in [0u32, 1, 2, 5, 255] {
            let mut c = vec![0u32; 512];
            for i in 0..=msv as usize {
                c[i] = 1;
            }
            probe(
                &format!("uniform counts msv={msv}"),
                &c,
                msv,
                11,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
            // a single non-zero symbol
            let mut c1 = vec![0u32; 512];
            c1[msv as usize] = 7;
            probe(
                &format!("single symbol msv={msv}"),
                &c1,
                msv,
                11,
                HUF_CTABLE_WORKSPACE_SIZE,
                0,
            );
        }
    }
}

/// Row 76: `HUF_initCStream` -> `dstSize_tooSmall`
/// (`dstCapacity <= sizeof(bitContainer[0])` == 8).
///
/// `HUF_compress1X_usingCTable_internal_body` *swallows* the error
/// (`if (HUF_isError(initErr)) return 0;`, huf_compress.c L1071) — exactly as
/// `FSE_compress_usingCTable` does — so the only observable behaviour is
/// "returns 0". Both libraries must agree on that, and on the whole dst buffer.
#[test]
fn err_huf_initcstream_dstsize_toosmall() {
    unsafe {
        let (fc, fr) = duo::<FnHufUsingCTable>("HUF_compress1X_usingCTable");
        let (gc, _) = duo::<FnHufUsingCTable>("HUF_compress4X_usingCTable");
        let (gr, _) = duo::<FnHufUsingCTable>("HUF_compress4X_usingCTable");
        let (ct, _tl) = c_ctable(255, 11, false, 0x7601);
        let src = to_alphabet(&gen_class(4, 4096, 0x7602), 0);

        for flags in HUF_FLAG_SET {
            for cap in 0usize..=12 {
                let (mut dc, mut dr) = twin(cap.max(1));
                let a = fc(
                    dc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ct.as_ptr(),
                    flags,
                );
                let b = fr(
                    dr.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ct.as_ptr(),
                    flags,
                );
                eqcode(&format!("HUF_compress1X_usingCTable cap={cap} flags={flags:#x}"), a, b);
                eqbuf("HUF_compress1X_usingCTable dst", &dc, &dr);
                if cap <= 8 {
                    assert_eq!(
                        a, 0,
                        "cap={cap}: HUF_initCStream failure must surface as 0"
                    );
                }
            }
            // 4X: `dstSize < 6+1+1+1+8` returns 0, larger sizes reach the
            // per-segment HUF_initCStream.
            for cap in 0usize..=40 {
                let (mut dc, mut dr) = twin(cap.max(1));
                let a = gc(
                    dc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ct.as_ptr(),
                    flags,
                );
                let b = gr(
                    dr.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ct.as_ptr(),
                    flags,
                );
                eqcode(&format!("HUF_compress4X_usingCTable cap={cap} flags={flags:#x}"), a, b);
                eqbuf("HUF_compress4X_usingCTable dst", &dc, &dr);
            }
            // tiny srcSize as well
            for n in [0usize, 1, 2, 3, 11, 12, 13] {
                for cap in [0usize, 1, 8, 9, 16, 64, 4096] {
                    let (mut dc, mut dr) = twin(cap.max(1));
                    let a = fc(
                        dc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ct.as_ptr(),
                        flags,
                    );
                    let b = fr(
                        dr.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ct.as_ptr(),
                        flags,
                    );
                    eqcode(
                        &format!("HUF_compress1X_usingCTable n={n} cap={cap} flags={flags:#x}"),
                        a,
                        b,
                    );
                    eqbuf("HUF_compress1X_usingCTable small dst", &dc, &dr);
                    let a = gc(
                        dc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ct.as_ptr(),
                        flags,
                    );
                    let b = gr(
                        dr.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        n,
                        ct.as_ptr(),
                        flags,
                    );
                    eqcode(
                        &format!("HUF_compress4X_usingCTable n={n} cap={cap} flags={flags:#x}"),
                        a,
                        b,
                    );
                }
            }
        }
    }
}

/// Rows 77-80: the four `HUF_compress_internal` guards, reached through both
/// public entry points, together with out-of-range `flags` bitmasks and
/// out-of-range `HUF_repeat` enum values crossing the FFI boundary.
#[test]
fn err_huf_compress_repeat_guards() {
    unsafe {
        let src = to_alphabet(&gen_class(4, 8192, 0x8001), 0);

        #[allow(clippy::too_many_arguments)]
        unsafe fn probe(
            name: &str,
            what: &str,
            cap: usize,
            src: &[u8],
            srclen: usize,
            msv: c_uint,
            huf_log: c_uint,
            wksp: usize,
            rep_in: c_int,
            flags: c_int,
        ) -> usize {
            let (fc, fr) = duo::<FnHufRepeat>(name);
            let (mut dc, mut dr) = twin(cap.max(1));
            let (mut tc, mut tr) = twin64(258);
            let mut wc = vec![0u64; wksp / 8 + 8];
            let mut wr = vec![0u64; wksp / 8 + 8];
            let mut rc: c_int = rep_in;
            let mut rr: c_int = rep_in;
            let a = fc(
                dc.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                srclen,
                msv,
                huf_log,
                wc.as_mut_ptr() as *mut c_void,
                wksp,
                tc.as_mut_ptr(),
                &mut rc,
                flags,
            );
            let b = fr(
                dr.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                srclen,
                msv,
                huf_log,
                wr.as_mut_ptr() as *mut c_void,
                wksp,
                tr.as_mut_ptr(),
                &mut rr,
                flags,
            );
            eqcode(&format!("{name} {what}"), a, b);
            eqv(&format!("{name} {what} *repeat"), rc, rr);
            eqbuf(&format!("{name} {what} dst"), &dc, &dr);
            eqbuf(&format!("{name} {what} CTable"), as_bytes64(&tc), as_bytes64(&tr));
            eqbuf(&format!("{name} {what} wksp"), as_bytes64(&wc), as_bytes64(&wr));
            a
        }

        // `sizeof(HUF_compress_tables_t)` is not exported; bisect the exact
        // threshold out of the C library itself.
        let need_wksp = {
            let (fc, _) = duo::<FnHufRepeat>("HUF_compress1X_repeat");
            let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
            let mut probe_w = |n: usize| -> bool {
                let mut d = vec![0u8; 65536];
                let mut t = vec![0u64; 258];
                let mut w = vec![0u64; HUF_WORKSPACE_SIZE / 8 + 8];
                let mut rep = HUF_repeat_none;
                let r = fc(
                    d.as_mut_ptr() as *mut c_void,
                    d.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                    0,
                    0,
                    w.as_mut_ptr() as *mut c_void,
                    n,
                    t.as_mut_ptr(),
                    &mut rep,
                    0,
                );
                !(is_err(r) && gc(r) == E_workSpace_tooSmall)
            };
            let mut lo = 0usize;
            let mut hi = HUF_WORKSPACE_SIZE;
            assert!(probe_w(hi));
            assert!(!probe_w(lo));
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if probe_w(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            hi
        };
        assert!(need_wksp > 1024 && need_wksp <= HUF_WORKSPACE_SIZE, "need_wksp={need_wksp}");

        for name in ["HUF_compress1X_repeat", "HUF_compress4X_repeat"] {
            // --- row 77: wkspSize < sizeof(HUF_compress_tables_t)
            for w in [0usize, 8, 1024, need_wksp / 2, need_wksp - 8, need_wksp - 1] {
                let r = probe(name, &format!("wksp={w}"), 65536, &src, src.len(), 0, 0, w, HUF_repeat_none, 0);
                assert_code(&format!("{name} short wksp"), r, E_workSpace_tooSmall);
            }
            let r = probe(
                name,
                "wksp exact",
                65536,
                &src,
                src.len(),
                0,
                0,
                need_wksp,
                HUF_repeat_none,
                0,
            );
            assert!(!is_err(r), "{name}: exact workspace must succeed");
            let r = probe(
                name,
                "wksp full",
                65536,
                &src,
                src.len(),
                0,
                0,
                HUF_WORKSPACE_SIZE,
                HUF_repeat_none,
                0,
            );
            assert!(!is_err(r), "{name}: full workspace must succeed");

            // --- row 78: srcSize > HUF_BLOCKSIZE_MAX
            let huge = to_alphabet(&gen_class(4, HUF_BLOCKSIZE_MAX + 64, 0x8002), 0);
            for n in [HUF_BLOCKSIZE_MAX + 1, HUF_BLOCKSIZE_MAX + 2, HUF_BLOCKSIZE_MAX + 64] {
                let r = probe(
                    name,
                    &format!("srcSize={n}"),
                    1 << 20,
                    &huge,
                    n,
                    0,
                    0,
                    HUF_WORKSPACE_SIZE,
                    HUF_repeat_none,
                    0,
                );
                assert_code(&format!("{name} srcSize too large"), r, E_srcSize_wrong);
            }
            // exactly at the limit is legal
            let r = probe(
                name,
                "srcSize==HUF_BLOCKSIZE_MAX",
                1 << 20,
                &huge,
                HUF_BLOCKSIZE_MAX,
                0,
                0,
                HUF_WORKSPACE_SIZE,
                HUF_repeat_none,
                0,
            );
            assert!(!is_err(r), "{name}: srcSize == HUF_BLOCKSIZE_MAX must be accepted");

            // --- row 79: huffLog > HUF_TABLELOG_MAX
            for tl in [13u32, 14, 15, 16, 31, 255, 0xFFFF, 0xFFFF_FFFF] {
                let r = probe(
                    name,
                    &format!("huffLog={tl}"),
                    65536,
                    &src,
                    src.len(),
                    0,
                    tl,
                    HUF_WORKSPACE_SIZE,
                    HUF_repeat_none,
                    0,
                );
                assert_code(&format!("{name} huffLog too large"), r, E_tableLog_tooLarge);
            }

            // --- row 80: maxSymbolValue > HUF_SYMBOLVALUE_MAX
            for msv in [256u32, 257, 1000, 0xFFFF, 0xFFFF_FFFF] {
                let r = probe(
                    name,
                    &format!("msv={msv}"),
                    65536,
                    &src,
                    src.len(),
                    msv,
                    0,
                    HUF_WORKSPACE_SIZE,
                    HUF_repeat_none,
                    0,
                );
                assert_code(&format!("{name} msv too large"), r, E_maxSymbolValue_tooLarge);
            }

            // guard ordering: workspace first, then srcSize, then huffLog, then msv
            let r = probe(
                name,
                "all bad at once",
                65536,
                &src,
                src.len(),
                300,
                20,
                8,
                HUF_repeat_none,
                0,
            );
            assert_code(&format!("{name} all bad"), r, E_workSpace_tooSmall);
            let r = probe(
                name,
                "srcSize+huffLog+msv bad",
                1 << 20,
                &huge,
                HUF_BLOCKSIZE_MAX + 1,
                300,
                20,
                HUF_WORKSPACE_SIZE,
                HUF_repeat_none,
                0,
            );
            assert_code(&format!("{name} srcSize wins"), r, E_srcSize_wrong);
            let r = probe(
                name,
                "huffLog+msv bad",
                65536,
                &src,
                src.len(),
                300,
                20,
                HUF_WORKSPACE_SIZE,
                HUF_repeat_none,
                0,
            );
            assert_code(&format!("{name} huffLog wins"), r, E_tableLog_tooLarge);

            // --- out-of-range flags and HUF_repeat values
            for flags in HUF_FLAG_SET {
                for rep in [
                    HUF_repeat_none,
                    HUF_repeat_check,
                    HUF_repeat_valid,
                    3,
                    4,
                    99,
                    -1,
                    i32::MAX,
                    i32::MIN,
                ] {
                    for n in [0usize, 1, 2, 12, 200, 8192] {
                        probe(
                            name,
                            &format!("flags={flags:#x} rep={rep} n={n}"),
                            65536,
                            &src,
                            n,
                            0,
                            0,
                            HUF_WORKSPACE_SIZE,
                            rep,
                            flags,
                        );
                    }
                    // dstCapacity 0 / 1 / one byte short
                    for cap in [0usize, 1, 2, 8, 16] {
                        probe(
                            name,
                            &format!("flags={flags:#x} rep={rep} cap={cap}"),
                            cap,
                            &src,
                            src.len(),
                            0,
                            0,
                            HUF_WORKSPACE_SIZE,
                            rep,
                            flags,
                        );
                    }
                }
            }
        }
    }
}

/// `HUF_optimalTableLog`, `HUF_cardinality`, `HUF_minTableLog`,
/// `HUF_validateCTable`, `HUF_estimateCompressedSize`,
/// `HUF_getNbBitsFromCTable`, `HUF_readCTableHeader` under adversarial /
/// out-of-range arguments (pure or read-only functions: any difference is a bug).
#[test]
fn err_huf_misc_out_of_range() {
    unsafe {
        let (otc, otr) = duo::<FnHufOptimalTableLog>("HUF_optimalTableLog");
        let (cac, car) = duo::<unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint>("HUF_cardinality");
        let (mtc, mtr) = duo::<unsafe extern "C" fn(c_uint) -> c_uint>("HUF_minTableLog");
        let (vac, var_) =
            duo::<unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> c_int>("HUF_validateCTable");
        let (esc, esr) =
            duo::<unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> usize>("HUF_estimateCompressedSize");
        let (nbc, nbr) = duo::<unsafe extern "C" fn(*const u64, u32) -> u32>("HUF_getNbBitsFromCTable");

        let mut count = vec![0u32; 512];
        for i in 0..256usize {
            count[i] = (i as u32 % 11) + 1;
        }
        let (ct, tl) = c_ctable(255, 11, false, 0x9101);

        for card in [1u32, 2, 3, 255, 256, 1000, 0xFFFF_FFFF] {
            eqv(&format!("HUF_minTableLog({card})"), mtc(card), mtr(card));
        }
        for msv in [0u32, 1, 15, 255, 511] {
            eqv(&format!("HUF_cardinality(msv={msv})"), cac(count.as_ptr(), msv), car(count.as_ptr(), msv));
            eqv(
                &format!("HUF_validateCTable(msv={msv})"),
                vac(ct.as_ptr(), count.as_ptr(), msv),
                var_(ct.as_ptr(), count.as_ptr(), msv),
            );
            eqv(
                &format!("HUF_estimateCompressedSize(msv={msv})"),
                esc(ct.as_ptr(), count.as_ptr(), msv),
                esr(ct.as_ptr(), count.as_ptr(), msv),
            );
        }
        for sym in [0u32, 1, 127, 255] {
            eqv(
                &format!("HUF_getNbBitsFromCTable({sym})"),
                nbc(ct.as_ptr(), sym),
                nbr(ct.as_ptr(), sym),
            );
        }

        // HUF_optimalTableLog: `maxTableLog` above the limit, `srcSize` 0/1,
        // undersized workspace, every flags bitmask.
        for flags in HUF_FLAG_SET {
            for mtl in [0u32, 1, 5, 11, 12, 13, 20, 0xFFFF] {
                for n in [0usize, 1, 2, 1000, 1 << 20] {
                    for w in [0usize, 8, 1024, HUF_WORKSPACE_SIZE] {
                        let mut wc = vec![0u64; w / 8 + 8];
                        let mut wr = vec![0u64; w / 8 + 8];
                        let (mut tc, mut tr) = twin64(258);
                        let a = otc(
                            mtl,
                            n,
                            255,
                            wc.as_mut_ptr() as *mut c_void,
                            w,
                            tc.as_mut_ptr(),
                            count.as_ptr(),
                            flags,
                        );
                        let b = otr(
                            mtl,
                            n,
                            255,
                            wr.as_mut_ptr() as *mut c_void,
                            w,
                            tr.as_mut_ptr(),
                            count.as_ptr(),
                            flags,
                        );
                        eqv(
                            &format!("HUF_optimalTableLog(mtl={mtl},n={n},w={w},flags={flags:#x})"),
                            a,
                            b,
                        );
                        eqbuf("HUF_optimalTableLog CTable", as_bytes64(&tc), as_bytes64(&tr));
                        eqbuf("HUF_optimalTableLog wksp", as_bytes64(&wc), as_bytes64(&wr));
                    }
                }
            }
        }
        // HUF_readCTableHeader on a real table and on garbage
        let (rhc, rhr) = duo::<unsafe extern "C" fn(*const u64) -> u64>("HUF_readCTableHeader");
        eqv("HUF_readCTableHeader(valid)", rhc(ct.as_ptr()), rhr(ct.as_ptr()));
        let mut rng = Rng::new(0x9102);
        for _ in 0..200 {
            let mut g = vec![0u64; 258];
            for x in g.iter_mut() {
                *x = rng.next_u64();
            }
            eqv("HUF_readCTableHeader(garbage)", rhc(g.as_ptr()), rhr(g.as_ptr()));
        }
        let _ = tl;
    }
}

// ============================ huf_decompress.c (rows 268-304)

const HUF_DEC_FLAGS: [c_int; 10] = [
    0,
    HUF_flags_bmi2,
    HUF_flags_disableAsm,
    HUF_flags_disableFast,
    HUF_flags_disableAsm | HUF_flags_disableFast,
    HUF_flags_bmi2 | HUF_flags_disableAsm | HUF_flags_disableFast,
    0x3F,
    0x40,
    -1,
    i32::MIN,
];

#[track_caller]
unsafe fn diff_read_dtable(
    name: &str,
    what: &str,
    desc: u32,
    src: &[u8],
    src_size: usize,
    wksp_bytes: usize,
) -> usize {
    let (fc, fr) = duo::<FnHufReadDTable>(name);
    let mut out = usize::MAX;
    for flags in HUF_DEC_FLAGS {
        let mut dc = dtable_with_desc(desc);
        let mut dr = dtable_with_desc(desc);
        let mut wc = vec![0u32; wksp_bytes / 4 + 8];
        let mut wr = vec![0u32; wksp_bytes / 4 + 8];
        let a = fc(
            dc.as_mut_ptr(),
            src.as_ptr() as *const c_void,
            src_size,
            wc.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        let b = fr(
            dr.as_mut_ptr(),
            src.as_ptr() as *const c_void,
            src_size,
            wr.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        eqcode(&format!("{name}(flags={flags:#x}) {what}"), a, b);
        eqbuf(
            &format!("{name}(flags={flags:#x}) {what} DTable"),
            as_bytes32(&dc),
            as_bytes32(&dr),
        );
        eqbuf(
            &format!("{name}(flags={flags:#x}) {what} wksp"),
            as_bytes32(&wc),
            as_bytes32(&wr),
        );
        out = a;
    }
    out
}

/// Rows 272/273: `HUF_readDTableX1_wksp` -> `tableLog_tooLarge`
/// (workspace smaller than `HUF_ReadDTableX1_Workspace`, and a DTable whose
/// `maxTableLog` cannot hold the Huffman tree).
#[test]
fn err_huf_readdtable_x1() {
    unsafe {
        let src = to_alphabet(&gen_class(4, 30000, 0x2721), 0);
        let hb = c_huf_blob(&src, 0, 11, false).expect("huf blob");
        let desc_bytes = &hb.blob[..hb.desc_len];
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");

        // bisect sizeof(HUF_ReadDTableX1_Workspace) out of the C library
        let need = {
            let (fc, _) = duo::<FnHufReadDTable>("HUF_readDTableX1_wksp");
            let mut p = |n: usize| -> bool {
                let mut d = dtable_with_desc(11);
                let mut w = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4 + 8];
                let r = fc(
                    d.as_mut_ptr(),
                    desc_bytes.as_ptr() as *const c_void,
                    desc_bytes.len(),
                    w.as_mut_ptr() as *mut c_void,
                    n,
                    0,
                );
                !(is_err(r) && gc(r) == E_tableLog_tooLarge)
            };
            let mut lo = 0usize;
            let mut hi = HUF_DECOMPRESS_WORKSPACE_SIZE;
            assert!(p(hi));
            assert!(!p(lo));
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if p(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            hi
        };
        assert!(need > 256 && need <= HUF_DECOMPRESS_WORKSPACE_SIZE, "need={need}");

        // --- row 272
        for w in [0usize, 4, 256, need / 2, need - 4, need - 1] {
            let r = diff_read_dtable(
                "HUF_readDTableX1_wksp",
                &format!("wksp={w}"),
                11,
                desc_bytes,
                desc_bytes.len(),
                w,
            );
            assert_code("HUF_readDTableX1_wksp short wksp", r, E_tableLog_tooLarge);
        }
        let r = diff_read_dtable(
            "HUF_readDTableX1_wksp",
            "wksp exact",
            11,
            desc_bytes,
            desc_bytes.len(),
            need,
        );
        assert!(!is_err(r), "exact workspace must succeed");

        // --- row 273: DTable too small for the tree
        let mut saw = false;
        for d in 0u32..=12 {
            let r = diff_read_dtable(
                "HUF_readDTableX1_wksp",
                &format!("desc maxTableLog={d}"),
                d,
                desc_bytes,
                desc_bytes.len(),
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
            if is_err(r) && gc(r) == E_tableLog_tooLarge {
                saw = true;
            }
        }
        assert!(saw, "row 273 (DTable too small) was never reached");

        // truncated / corrupted descriptions (HUF_readStats errors propagate)
        for n in 0..desc_bytes.len() {
            diff_read_dtable(
                "HUF_readDTableX1_wksp",
                &format!("truncated n={n}"),
                11,
                desc_bytes,
                n,
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
        }
        let mut rng = Rng::new(0x2722);
        for _ in 0..200 {
            let mut v = desc_bytes.to_vec();
            let i = rng.below(v.len());
            let b = rng.byte();
            v[i] = b;
            diff_read_dtable(
                "HUF_readDTableX1_wksp",
                "fuzz",
                11,
                &v,
                v.len(),
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
        }
    }
}

/// Rows 285/286/287: `HUF_readDTableX2_wksp` -> `GENERIC` (workspace too
/// small), `tableLog_tooLarge` (`dtd.maxTableLog > HUF_TABLELOG_MAX`) and
/// `tableLog_tooLarge` (`tableLog > maxTableLog`).
#[test]
fn err_huf_readdtable_x2() {
    unsafe {
        let src = to_alphabet(&gen_class(4, 30000, 0x2851), 0);
        let hb = c_huf_blob(&src, 0, 11, false).expect("huf blob");
        let desc_bytes = &hb.blob[..hb.desc_len];
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");

        let need = {
            let (fc, _) = duo::<FnHufReadDTable>("HUF_readDTableX2_wksp");
            let mut p = |n: usize| -> bool {
                let mut d = dtable_with_desc(12);
                let mut w = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4 + 8];
                let r = fc(
                    d.as_mut_ptr(),
                    desc_bytes.as_ptr() as *const c_void,
                    desc_bytes.len(),
                    w.as_mut_ptr() as *mut c_void,
                    n,
                    0,
                );
                !(is_err(r) && gc(r) == E_GENERIC)
            };
            let mut lo = 0usize;
            let mut hi = HUF_DECOMPRESS_WORKSPACE_SIZE;
            assert!(p(hi));
            assert!(!p(lo));
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if p(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            hi
        };
        assert!(need > 256 && need <= HUF_DECOMPRESS_WORKSPACE_SIZE, "need={need}");

        // --- row 285
        for w in [0usize, 4, 256, need / 2, need - 4, need - 1] {
            let r = diff_read_dtable(
                "HUF_readDTableX2_wksp",
                &format!("wksp={w}"),
                12,
                desc_bytes,
                desc_bytes.len(),
                w,
            );
            assert_code("HUF_readDTableX2_wksp short wksp", r, E_GENERIC);
        }
        let r = diff_read_dtable(
            "HUF_readDTableX2_wksp",
            "wksp exact",
            12,
            desc_bytes,
            desc_bytes.len(),
            need,
        );
        assert!(!is_err(r), "exact workspace must succeed");

        // --- row 286: dtd.maxTableLog > HUF_TABLELOG_MAX (checked before readStats)
        for d in [13u32, 14, 20, 255] {
            let r = diff_read_dtable(
                "HUF_readDTableX2_wksp",
                &format!("desc maxTableLog={d}"),
                d,
                desc_bytes,
                desc_bytes.len(),
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
            assert_code("HUF_readDTableX2_wksp maxTableLog>12", r, E_tableLog_tooLarge);
        }
        // and it wins over a bad description
        let r = diff_read_dtable(
            "HUF_readDTableX2_wksp",
            "desc 13 + srcSize 0",
            13,
            desc_bytes,
            0,
            HUF_DECOMPRESS_WORKSPACE_SIZE,
        );
        assert_code("HUF_readDTableX2_wksp maxTableLog>12 first", r, E_tableLog_tooLarge);

        // --- row 287: tableLog > maxTableLog
        let mut saw = false;
        for d in 0u32..=12 {
            let r = diff_read_dtable(
                "HUF_readDTableX2_wksp",
                &format!("desc maxTableLog={d}"),
                d,
                desc_bytes,
                desc_bytes.len(),
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
            if is_err(r) && gc(r) == E_tableLog_tooLarge {
                saw = true;
            }
        }
        assert!(saw, "row 287 (tableLog > maxTableLog) was never reached");

        for n in 0..desc_bytes.len() {
            diff_read_dtable(
                "HUF_readDTableX2_wksp",
                &format!("truncated n={n}"),
                12,
                desc_bytes,
                n,
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
        }
        let mut rng = Rng::new(0x2852);
        for _ in 0..200 {
            let mut v = desc_bytes.to_vec();
            let i = rng.below(v.len());
            let b = rng.byte();
            v[i] = b;
            diff_read_dtable(
                "HUF_readDTableX2_wksp",
                "fuzz",
                12,
                &v,
                v.len(),
                HUF_DECOMPRESS_WORKSPACE_SIZE,
            );
        }
    }
}


/// Build a **valid** HUF table description whose length lands in
/// `[96, 126]` bytes, using only C-library primitives.
///
/// Needed for ERRORS.md row 299: `HUF_selectDecoder` only picks the X2 decoder
/// when `cSrcSize >= 96` (see huf_decompress.c L1820-1828), and
/// `HUF_decompress4X2_DCtx_wksp`'s `hSize >= cSrcSize` guard requires
/// `cSrcSize <= hSize`, so the description itself has to be at least 96 bytes.
/// `HUF_writeCTable_wksp` never emits one that big, so the weights are
/// FSE-coded here with a deliberately *mismatched* CTable (which is legal: the
/// decoder only reads the normalized-count header that accompanies it).
unsafe fn c_huf_big_desc() -> Option<Vec<u8>> {
    let (hist, _) = duo::<FnHistCountWksp>("HIST_count_wksp");
    let (norm, _) = duo::<FnNormalizeCount>("FSE_normalizeCount");
    let (wnc, _) = duo::<FnWriteNCount>("FSE_writeNCount");
    let (bct, _) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
    let (cuc, _) = duo::<FnCompressUsingCTable>("FSE_compress_usingCTable");
    let (rs, _) = duo::<FnHufReadStats>("HUF_readStats");

    for seed in 0u64..4000 {
        let mut rng = Rng::new(0x2991_0000 + seed);
        let osize = 200 + rng.below(56);
        let mut wt = vec![0u8; osize];
        let mut total: u32 = 0;
        let mut ok = true;
        for i in 0..osize {
            let r = rng.below(100);
            let w: u32 = if r < 30 {
                1
            } else if r < 55 {
                2
            } else if r < 72 {
                3
            } else if r < 84 {
                4
            } else if r < 92 {
                5
            } else if r < 97 {
                6
            } else {
                7
            };
            wt[i] = w as u8;
            total += 1u32 << (w - 1);
            if total > 4000 {
                ok = false;
                break;
            }
        }
        if !ok || total < 2048 {
            continue;
        }
        // make `rest = 4096 - total` an exact power of two
        let mut rest = 4096 - total;
        let mut p = 1u32;
        while p < rest {
            p <<= 1;
        }
        if p != rest {
            let need = total - (4096 - p);
            let mut done = false;
            for i in 0..osize {
                let v = 1u32 << (wt[i] - 1);
                if v > need {
                    let nv = v - need;
                    if nv.is_power_of_two() {
                        let k = nv.trailing_zeros();
                        if k + 1 <= 12 {
                            wt[i] = (k + 1) as u8;
                            total -= need;
                            done = true;
                            break;
                        }
                    }
                }
            }
            if !done {
                continue;
            }
            rest = 4096 - total;
            if rest == 0 || !rest.is_power_of_two() {
                continue;
            }
        }
        // rankStats[1] must be >= 2 and even (counting the implied last weight)
        let last_w = rest.trailing_zeros() + 1;
        let mut r1 = wt.iter().filter(|&&w| w == 1).count();
        if last_w == 1 {
            r1 += 1;
        }
        if r1 < 2 || r1 % 2 == 1 {
            continue;
        }

        // FSE-code the weights
        let mut count = vec![0u32; 256];
        let mut msv: c_uint = 255;
        let mut hw = vec![0u32; 1024];
        let mx = hist(
            count.as_mut_ptr(),
            &mut msv,
            wt.as_ptr() as *const c_void,
            wt.len(),
            hw.as_mut_ptr() as *mut c_void,
            hw.len() * 4,
        );
        if is_err(mx) || msv < 1 || mx == wt.len() {
            continue;
        }
        for alpha in 0u32..500 {
            let mut c2 = vec![0u32; 256];
            let mut tot2: usize = 0;
            for i in 0..=msv as usize {
                c2[i] = count[i] + alpha;
                tot2 += c2[i] as usize;
            }
            let mut nrm = vec![0i16; 256];
            let r = norm(nrm.as_mut_ptr(), 6, c2.as_ptr(), tot2, msv, 0);
            if is_err(r) || r == 0 {
                continue;
            }
            let tl = r as u32;
            let mut desc = vec![0u8; 512];
            let hdr = wnc(
                desc.as_mut_ptr().add(1) as *mut c_void,
                desc.len() - 1,
                nrm.as_ptr(),
                msv,
                tl,
            );
            if is_err(hdr) {
                continue;
            }
            let mut ct = vec![0u32; fse_ctable_size_u32(FSE_MAX_TABLELOG, 255) + 8];
            let mut cw = vec![0u32; fse_build_ctable_wksp_u32(255, FSE_MAX_TABLELOG) + 8];
            if is_err(bct(
                ct.as_mut_ptr(),
                nrm.as_ptr(),
                msv,
                tl,
                cw.as_mut_ptr() as *mut c_void,
                cw.len() * 4,
            )) {
                continue;
            }
            let body = cuc(
                desc.as_mut_ptr().add(1 + hdr) as *mut c_void,
                desc.len() - 1 - hdr,
                wt.as_ptr() as *const c_void,
                wt.len(),
                ct.as_ptr(),
            );
            if is_err(body) || body == 0 {
                continue;
            }
            let isize_ = hdr + body;
            if isize_ >= 127 || isize_ < 95 {
                continue;
            }
            desc[0] = isize_ as u8;
            desc.truncate(isize_ + 1);
            // validate with the real reader
            let mut hwo = vec![0u8; 512];
            let mut rk = vec![0u32; 32];
            let mut ns = 0u32;
            let mut tlo = 0u32;
            let v = rs(
                hwo.as_mut_ptr(),
                512,
                rk.as_mut_ptr(),
                &mut ns,
                &mut tlo,
                desc.as_ptr() as *const c_void,
                desc.len(),
            );
            if is_err(v) || v != desc.len() {
                continue;
            }
            return Some(desc);
        }
    }
    None
}

#[track_caller]
unsafe fn diff_dec_dctx(
    name: &str,
    what: &str,
    desc: u32,
    dst_cap: usize,
    csrc: &[u8],
    csrc_size: usize,
    wksp_bytes: usize,
) -> usize {
    let (fc, fr) = duo::<FnHufDecDCtxWksp>(name);
    let mut out = usize::MAX;
    for flags in HUF_DEC_FLAGS {
        let mut dtc = dtable_with_desc(desc);
        let mut dtr = dtable_with_desc(desc);
        let (mut dc, mut dr) = twin(dst_cap.max(1));
        let mut wc = vec![0u32; wksp_bytes / 4 + 8];
        let mut wr = vec![0u32; wksp_bytes / 4 + 8];
        let a = fc(
            dtc.as_mut_ptr(),
            dc.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            wc.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        let b = fr(
            dtr.as_mut_ptr(),
            dr.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            wr.as_mut_ptr() as *mut c_void,
            wksp_bytes,
            flags,
        );
        eqcode(&format!("{name}(flags={flags:#x}) {what}"), a, b);
        eqbuf(&format!("{name}(flags={flags:#x}) {what} dst"), &dc, &dr);
        eqbuf(
            &format!("{name}(flags={flags:#x}) {what} DTable"),
            as_bytes32(&dtc),
            as_bytes32(&dtr),
        );
        eqbuf(
            &format!("{name}(flags={flags:#x}) {what} wksp"),
            as_bytes32(&wc),
            as_bytes32(&wr),
        );
        if flags == HUF_flags_disableAsm | HUF_flags_disableFast {
            out = a;
        }
    }
    out
}

/// Rows 300/301: `HUF_decompress1X_DCtx_wksp` -> `dstSize_tooSmall`
/// (`dstSize == 0`) and `corruption_detected` (`cSrcSize > dstSize`).
/// Rows 302/298: `HUF_decompress1X1_DCtx_wksp` / `HUF_decompress1X2_DCtx_wksp`
/// -> `srcSize_wrong` (`hSize >= cSrcSize`, i.e. no bitstream after the table).
#[test]
fn err_huf_decompress_1x_dctx_wksp() {
    unsafe {
        let src = to_alphabet(&gen_class(4, 20000, 0x3001), 0);
        let hb = c_huf_blob(&src, 0, 11, false).expect("huf blob");
        let desc_bytes = &hb.blob[..hb.desc_len];
        let W = HUF_DECOMPRESS_WORKSPACE_SIZE;

        // sanity: the whole blob round-trips through the universal selector
        let ok = diff_dec_dctx(
            "HUF_decompress1X_DCtx_wksp",
            "valid",
            12,
            src.len(),
            &hb.blob,
            hb.blob.len(),
            W,
        );
        assert_eq!(ok, src.len(), "valid 1X blob should decode");

        // --- row 300: dstSize == 0 (checked before HUF_selectDecoder, which
        //     would otherwise divide by zero)
        for n in [0usize, 1, 2, 10, hb.blob.len()] {
            let r = diff_dec_dctx(
                "HUF_decompress1X_DCtx_wksp",
                &format!("dstSize=0 cSrcSize={n}"),
                12,
                0,
                &hb.blob,
                n,
                W,
            );
            assert_code("HUF_decompress1X_DCtx_wksp dstSize=0", r, E_dstSize_tooSmall);
        }

        // --- row 301: cSrcSize > dstSize
        for dst in [1usize, 2, 7, 64, hb.blob.len() - 1] {
            let r = diff_dec_dctx(
                "HUF_decompress1X_DCtx_wksp",
                &format!("cSrcSize>dstSize dst={dst}"),
                12,
                dst,
                &hb.blob,
                hb.blob.len(),
                W,
            );
            assert_code("HUF_decompress1X_DCtx_wksp cSrcSize>dstSize", r, E_corruption_detected);
        }
        // cSrcSize == dstSize -> raw copy; cSrcSize == 1 -> RLE
        let r = diff_dec_dctx(
            "HUF_decompress1X_DCtx_wksp",
            "cSrcSize==dstSize",
            12,
            hb.blob.len(),
            &hb.blob,
            hb.blob.len(),
            W,
        );
        eqv("HUF_decompress1X_DCtx_wksp raw copy", r, hb.blob.len());
        let r = diff_dec_dctx(
            "HUF_decompress1X_DCtx_wksp",
            "cSrcSize==1 (rle)",
            12,
            100,
            &hb.blob,
            1,
            W,
        );
        eqv("HUF_decompress1X_DCtx_wksp rle", r, 100);

        // --- rows 302 / 298: hSize >= cSrcSize
        for name in ["HUF_decompress1X1_DCtx_wksp", "HUF_decompress1X2_DCtx_wksp"] {
            let r = diff_dec_dctx(
                name,
                "table only, no bitstream",
                12,
                src.len(),
                desc_bytes,
                desc_bytes.len(),
                W,
            );
            assert_code(&format!("{name} hSize>=cSrcSize"), r, E_srcSize_wrong);
            // one byte of bitstream is enough to pass the guard
            let r = diff_dec_dctx(
                name,
                "table + 1 byte",
                12,
                src.len(),
                &hb.blob,
                hb.desc_len + 1,
                W,
            );
            assert!(is_err(r), "{name}: a 1-byte bitstream cannot decode {} bytes", src.len());
            // workspace too small propagates from HUF_readDTableX*
            for w in [0usize, 4, 256, 1024] {
                let r = diff_dec_dctx(name, &format!("wksp={w}"), 12, src.len(), &hb.blob, hb.blob.len(), w);
                assert!(is_err(r), "{name}: wksp={w} must fail");
            }
            // dstSize 0 / 1 / one byte short
            for dst in [0usize, 1, src.len() - 1, src.len(), src.len() + 1] {
                diff_dec_dctx(
                    name,
                    &format!("dst={dst}"),
                    12,
                    dst,
                    &hb.blob,
                    hb.blob.len(),
                    W,
                );
            }
            // cSrcSize 0 / 1 / truncated
            for n in [0usize, 1, 2, 5, hb.desc_len, hb.blob.len() - 1] {
                diff_dec_dctx(
                    name,
                    &format!("cSrcSize={n}"),
                    12,
                    src.len(),
                    &hb.blob,
                    n,
                    W,
                );
            }
        }
    }
}

/// Rows 303/304 + 284/299: `HUF_decompress4X_hufOnly_wksp` ->
/// `dstSize_tooSmall` (`dstSize == 0`), `corruption_detected`
/// (`cSrcSize == 0`), and the `srcSize_wrong` of
/// `HUF_decompress4X1_DCtx_wksp` / `HUF_decompress4X2_DCtx_wksp`
/// (`hSize >= cSrcSize`) — both selector branches.
#[test]
fn err_huf_decompress_4x_hufonly_wksp() {
    unsafe {
        let (sel, selr) = duo::<unsafe extern "C" fn(usize, usize) -> u32>("HUF_selectDecoder");
        let W = HUF_DECOMPRESS_WORKSPACE_SIZE;
        let src = to_alphabet(&gen_class(4, 40000, 0x3041), 0);
        let hb = c_huf_blob(&src, 0, 11, true).expect("huf 4X blob");
        let desc_bytes = &hb.blob[..hb.desc_len];

        // HUF_selectDecoder is a pure function (dstSize == 0 divides by zero, so
        // it is only probed for dstSize >= 1 — the two callers guard it too).
        for dst in [1usize, 2, 7, 100, 1 << 10, 1 << 16, 1 << 20] {
            for cs in [0usize, 1, 2, 10, 100, 1000, 1 << 16, 1 << 20] {
                eqv(&format!("HUF_selectDecoder({dst},{cs})"), sel(dst, cs), selr(dst, cs));
            }
        }

        // sanity
        let ok = diff_dec_dctx(
            "HUF_decompress4X_hufOnly_wksp",
            "valid",
            12,
            src.len(),
            &hb.blob,
            hb.blob.len(),
            W,
        );
        assert_eq!(ok, src.len(), "valid 4X blob should decode");

        // --- row 303: dstSize == 0
        for n in [0usize, 1, 10, hb.blob.len()] {
            let r = diff_dec_dctx(
                "HUF_decompress4X_hufOnly_wksp",
                &format!("dstSize=0 cSrcSize={n}"),
                12,
                0,
                &hb.blob,
                n,
                W,
            );
            assert_code("HUF_decompress4X_hufOnly_wksp dstSize=0", r, E_dstSize_tooSmall);
        }
        // --- row 304: cSrcSize == 0
        for dst in [1usize, 2, 100, src.len()] {
            let r = diff_dec_dctx(
                "HUF_decompress4X_hufOnly_wksp",
                &format!("cSrcSize=0 dst={dst}"),
                12,
                dst,
                &hb.blob,
                0,
                W,
            );
            assert_code("HUF_decompress4X_hufOnly_wksp cSrcSize=0", r, E_corruption_detected);
        }

        // --- rows 284 / 299: hSize >= cSrcSize, in both selector branches.
        // The selector is driven by the (dstSize, cSrcSize) ratio, so sweep
        // dstSize until each branch has been taken.
        let big = c_huf_big_desc().expect("could not build a >=96-byte table description");
        let mut seen = [false, false];
        for tbl in [desc_bytes, &big[..]] {
            for dst in [
                tbl.len() + 1,
                tbl.len() * 2,
                tbl.len() * 8,
                768,
                1 << 10,
                1 << 12,
                1 << 14,
                1 << 16,
                1 << 18,
            ] {
                let algo = sel(dst, tbl.len()) as usize;
                let r = diff_dec_dctx(
                    "HUF_decompress4X_hufOnly_wksp",
                    &format!("table only len={} dst={dst} (algo {algo})", tbl.len()),
                    12,
                    dst,
                    tbl,
                    tbl.len(),
                    W,
                );
                assert_code(
                    &format!("HUF_decompress4X{}_DCtx_wksp hSize>=cSrcSize", algo + 1),
                    r,
                    E_srcSize_wrong,
                );
                seen[algo] = true;
            }
        }
        assert!(seen[0], "row 284 (X1 branch) was never reached");
        assert!(seen[1], "row 299 (X2 branch) was never reached");

        // truncated bitstreams, tiny dst, tiny workspace
        for dst in [1usize, 2, 5, 6, 7, 64, src.len() / 2, src.len(), src.len() + 1] {
            for n in [1usize, 2, 5, hb.desc_len, hb.desc_len + 1, hb.desc_len + 9, hb.blob.len() - 1, hb.blob.len()] {
                diff_dec_dctx(
                    "HUF_decompress4X_hufOnly_wksp",
                    &format!("dst={dst} cSrcSize={n}"),
                    12,
                    dst,
                    &hb.blob,
                    n,
                    W,
                );
            }
        }
        for w in [0usize, 4, 256, 1024, W - 4] {
            diff_dec_dctx(
                "HUF_decompress4X_hufOnly_wksp",
                &format!("wksp={w}"),
                12,
                src.len(),
                &hb.blob,
                hb.blob.len(),
                w,
            );
        }
        // every DTable descriptor value
        for d in 0u32..=13 {
            diff_dec_dctx(
                "HUF_decompress4X_hufOnly_wksp",
                &format!("desc={d}"),
                d,
                src.len(),
                &hb.blob,
                hb.blob.len(),
                W,
            );
        }
    }
}

/// A valid DTable built by the C library from `desc`.
unsafe fn c_dtable(desc: &[u8], x2: bool, max_table_log: u32) -> Vec<u32> {
    let name = if x2 { "HUF_readDTableX2_wksp" } else { "HUF_readDTableX1_wksp" };
    let (fc, _) = duo::<FnHufReadDTable>(name);
    let mut dt = dtable_with_desc(max_table_log);
    let mut w = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4 + 8];
    let r = fc(
        dt.as_mut_ptr(),
        desc.as_ptr() as *const c_void,
        desc.len(),
        w.as_mut_ptr() as *mut c_void,
        HUF_DECOMPRESS_WORKSPACE_SIZE,
        0,
    );
    assert!(!is_err(r), "helper c_dtable({name}) failed: {r:#x}");
    dt
}

#[track_caller]
unsafe fn diff_dec_usingdtable(
    name: &str,
    what: &str,
    dt: &[u32],
    dst_cap: usize,
    csrc: &[u8],
    csrc_size: usize,
) -> usize {
    let (fc, fr) = duo::<FnHufDecUsingDTable>(name);
    let mut out = usize::MAX;
    for flags in HUF_DEC_FLAGS {
        let (mut dc, mut dr) = twin(dst_cap.max(1));
        let a = fc(
            dc.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            dt.as_ptr(),
            flags,
        );
        let b = fr(
            dr.as_mut_ptr() as *mut c_void,
            dst_cap,
            csrc.as_ptr() as *const c_void,
            csrc_size,
            dt.as_ptr(),
            flags,
        );
        eqcode(&format!("{name}(flags={flags:#x}) {what}"), a, b);
        eqbuf(&format!("{name}(flags={flags:#x}) {what} dst"), &dc, &dr);
        // report the plain fallback-body result: the fast C/asm loop can accept
        // shapes the body rejects (e.g. dstSize < 6), so targeted assertions
        // must look at the body.
        if flags == HUF_flags_disableAsm | HUF_flags_disableFast {
            out = a;
        }
    }
    out
}

/// Rows 274/275/276/277/282 (X1) and 288/289/290/291/296 (X2): the
/// `HUF_decompress{1,4}X{1,2}_usingDTable_internal_body` rejections
/// (`cSrcSize < 10`, `dstSize < 6`, `length4 > cSrcSize` and the end-of-stream
/// checks), for X1 *and* X2, via both public `usingDTable` entry points.
///
/// Rows 278 (L644) and 292 (L1425) — `opStart4 > oend` — are **unreachable**:
/// the body already rejects `dstSize < 6`, and `3*((n+3)/4) <= n` holds for
/// every `n >= 6`, so `opStart4` can never exceed `oend`.
/// Rows 279/280/281 (L680-682) and 293/294/295 (L1483-1485) —
/// `op1 > opStart2` / `op2 > opStart3` / `op3 > opStart4` — are the defensive
/// checks the C itself annotates "should not be necessary : op# advance in lock
/// step, and we control op4"; they are only reachable, if at all, through the
/// randomised corruption in `err_huf_decompress_fuzz`.
#[test]
fn err_huf_decompress_usingdtable_bodies() {
    unsafe {
        let src = to_alphabet(&gen_class(4, 60000, 0x4001), 0);
        let hb1 = c_huf_blob(&src, 0, 11, false).expect("1X blob");
        let hb4 = c_huf_blob(&src, 0, 11, true).expect("4X blob");
        let dt_x1 = c_dtable(&hb1.blob[..hb1.desc_len], false, 12);
        let dt_x2 = c_dtable(&hb1.blob[..hb1.desc_len], true, 12);
        let body1 = hb1.blob[hb1.desc_len..].to_vec();
        let body4 = hb4.blob[hb4.desc_len..].to_vec();
        assert!(body4.len() > 64, "4X body too short: {}", body4.len());
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");

        // sanity: both round-trip
        let r = diff_dec_usingdtable(
            "HUF_decompress1X_usingDTable",
            "valid X1",
            &dt_x1,
            src.len(),
            &body1,
            body1.len(),
        );
        eqv("valid 1X1 decode", r, src.len());
        let r = diff_dec_usingdtable(
            "HUF_decompress4X_usingDTable",
            "valid X1",
            &dt_x1,
            src.len(),
            &body4,
            body4.len(),
        );
        eqv("valid 4X1 decode", r, src.len());

        for (dtname, dt) in [("X1", &dt_x1), ("X2", &dt_x2)] {
            // --- cSrcSize < 10 (4X only; the 1X path goes through BIT_initDStream)
            for n in 0usize..10 {
                let r = diff_dec_usingdtable(
                    "HUF_decompress4X_usingDTable",
                    &format!("{dtname} cSrcSize={n}"),
                    dt,
                    src.len(),
                    &body4,
                    n,
                );
                assert_code(
                    &format!("4X {dtname} cSrcSize<10"),
                    r,
                    E_corruption_detected,
                );
            }
            // --- dstSize < 6
            for d in 0usize..6 {
                let r = diff_dec_usingdtable(
                    "HUF_decompress4X_usingDTable",
                    &format!("{dtname} dstSize={d}"),
                    dt,
                    d,
                    &body4,
                    body4.len(),
                );
                assert_code(&format!("4X {dtname} dstSize<6"), r, E_corruption_detected);
            }
            // --- length4 > cSrcSize (jump-table overflow)
            for (l1, l2, l3) in [
                (0xFFFFu16, 0u16, 0u16),
                (0u16, 0xFFFFu16, 0u16),
                (0u16, 0u16, 0xFFFFu16),
                (0xFFFFu16, 0xFFFFu16, 0xFFFFu16),
                (0x7FFFu16, 0x7FFFu16, 0x7FFFu16),
            ] {
                let mut v = body4.clone();
                v[0..2].copy_from_slice(&l1.to_le_bytes());
                v[2..4].copy_from_slice(&l2.to_le_bytes());
                v[4..6].copy_from_slice(&l3.to_le_bytes());
                let r = diff_dec_usingdtable(
                    "HUF_decompress4X_usingDTable",
                    &format!("{dtname} jumptable {l1}/{l2}/{l3}"),
                    dt,
                    src.len(),
                    &v,
                    v.len(),
                );
                assert_code(
                    &format!("4X {dtname} length4 overflow"),
                    r,
                    E_corruption_detected,
                );
            }
            // --- end-of-stream checks: truncate / corrupt the bitstream
            let mut saw1 = false;
            let mut saw4 = false;
            for n in [
                10usize,
                11,
                16,
                32,
                body4.len() / 4,
                body4.len() / 2,
                body4.len() - 1,
            ] {
                if n >= body4.len() {
                    continue;
                }
                let r = diff_dec_usingdtable(
                    "HUF_decompress4X_usingDTable",
                    &format!("{dtname} truncated n={n}"),
                    dt,
                    src.len(),
                    &body4,
                    n,
                );
                if is_err(r) && gc(r) == E_corruption_detected {
                    saw4 = true;
                }
            }
            for n in [1usize, 2, 8, 9, body1.len() / 2, body1.len() - 1] {
                if n >= body1.len() {
                    continue;
                }
                let r = diff_dec_usingdtable(
                    "HUF_decompress1X_usingDTable",
                    &format!("{dtname} truncated n={n}"),
                    dt,
                    src.len(),
                    &body1,
                    n,
                );
                if is_err(r) && gc(r) == E_corruption_detected {
                    saw1 = true;
                }
            }
            assert!(saw1, "1X {dtname}: end-of-stream check never fired");
            assert!(saw4, "4X {dtname}: end-of-stream check never fired");

            // dstSize larger/smaller than the true decoded size
            for d in [
                6usize,
                7,
                16,
                src.len() / 4,
                src.len() / 2,
                src.len() - 1,
                src.len(),
                src.len() + 1,
                src.len() + 1024,
            ] {
                diff_dec_usingdtable(
                    "HUF_decompress1X_usingDTable",
                    &format!("{dtname} dst={d}"),
                    dt,
                    d,
                    &body1,
                    body1.len(),
                );
                diff_dec_usingdtable(
                    "HUF_decompress4X_usingDTable",
                    &format!("{dtname} dst={d}"),
                    dt,
                    d,
                    &body4,
                    body4.len(),
                );
            }
        }
    }
}

/// Rows 268/269/270/271/283/297 plus everything else in the bodies: randomised
/// corruption of real 1X and 4X bitstreams, over X1 and X2 DTables, every
/// `flags` bitmask (including out-of-range ones) and a `dstSize` ladder.
/// The fast (assembly / C fast-loop) path requires `dtLog == 11`, `cSrcSize >= 10`
/// and each of the four segments >= 8 bytes, which the blobs below satisfy.
#[test]
fn err_huf_decompress_fuzz() {
    unsafe {
        let mut rng = Rng::new(0x4002_0001);
        let mut shapes: Vec<(&'static str, Vec<u8>, Vec<u32>, usize)> = Vec::new();
        for (cls, tl) in [(4usize, 11u32), (6, 11), (3, 11), (4, 8), (4, 12)] {
            let src = to_alphabet(&gen_class(cls, 60000, 0x4003 ^ tl as u64), 0);
            if let Some(hb) = c_huf_blob(&src, 0, tl, false) {
                let d1 = c_dtable(&hb.blob[..hb.desc_len], false, 12);
                let d2 = c_dtable(&hb.blob[..hb.desc_len], true, 12);
                let body = hb.blob[hb.desc_len..].to_vec();
                shapes.push(("HUF_decompress1X_usingDTable", body.clone(), d1, src.len()));
                shapes.push(("HUF_decompress1X_usingDTable", body, d2, src.len()));
            }
            if let Some(hb) = c_huf_blob(&src, 0, tl, true) {
                let d1 = c_dtable(&hb.blob[..hb.desc_len], false, 12);
                let d2 = c_dtable(&hb.blob[..hb.desc_len], true, 12);
                let body = hb.blob[hb.desc_len..].to_vec();
                shapes.push(("HUF_decompress4X_usingDTable", body.clone(), d1, src.len()));
                shapes.push(("HUF_decompress4X_usingDTable", body, d2, src.len()));
            }
        }
        assert!(shapes.len() >= 8, "not enough shapes: {}", shapes.len());

        for (name, body, dt, dec) in &shapes {
            for _ in 0..180 {
                let mut v = body.clone();
                match rng.below(4) {
                    0 => {
                        // single-byte flips
                        let k = 1 + rng.below(4);
                        for _ in 0..k {
                            let i = rng.below(v.len());
                            let b = rng.byte();
                            v[i] = b;
                        }
                    }
                    1 => {
                        // corrupt the 4X jump table
                        for i in 0..6.min(v.len()) {
                            let b = rng.byte();
                            v[i] = b;
                        }
                    }
                    2 => {
                        // zero out the end marker / tail bytes
                        let k = 1 + rng.below(8);
                        for j in 0..k {
                            if v.len() > j {
                                let l = v.len();
                                v[l - 1 - j] = 0;
                            }
                        }
                    }
                    _ => {
                        // splice in a random run
                        let off = rng.below(v.len());
                        let len = 1 + rng.below(16);
                        let bs = rng.bytes(len);
                        for (k, b) in bs.iter().enumerate() {
                            if off + k < v.len() {
                                v[off + k] = *b;
                            }
                        }
                    }
                }
                let n = if rng.below(4) == 0 { rng.below(v.len() + 1) } else { v.len() };
                let cap = match rng.below(6) {
                    0 => 0,
                    1 => 1 + rng.below(6),
                    2 => 6 + rng.below(32),
                    3 => *dec / 2,
                    4 => *dec,
                    _ => *dec + 128,
                };
                diff_dec_usingdtable(name, "fuzz", dt, cap, &v, n);
            }
        }
        // fully random bitstreams
        for (name, _body, dt, dec) in &shapes {
            for _ in 0..60 {
                let n = rng.below(64);
                let v = rng.bytes(n.max(1));
                let cap = [0usize, 1, 5, 6, 40, *dec][rng.below(6)];
                diff_dec_usingdtable(name, "fuzz-random", dt, cap, &v, n);
            }
        }
    }
}

/// Randomised corruption of *whole* HUF blobs (table description + bitstream)
/// through every public `HUF_decompress*` wrapper, with the workspace, the
/// `dstSize` and the `flags` all varied. Fixed seed.
#[test]
fn err_huf_decompress_wrappers_fuzz() {
    unsafe {
        let mut rng = Rng::new(0x4004_0001);
        let W = HUF_DECOMPRESS_WORKSPACE_SIZE;
        let names = [
            "HUF_decompress1X1_DCtx_wksp",
            "HUF_decompress1X2_DCtx_wksp",
            "HUF_decompress1X_DCtx_wksp",
            "HUF_decompress4X_hufOnly_wksp",
        ];
        let mut blobs: Vec<(Vec<u8>, usize)> = Vec::new();
        for (cls, tl, four) in [
            (4usize, 11u32, false),
            (4, 11, true),
            (6, 11, true),
            (3, 12, false),
            (4, 8, true),
        ] {
            let src = to_alphabet(&gen_class(cls, 40000, 0x4005 ^ tl as u64), 0);
            if let Some(hb) = c_huf_blob(&src, 0, tl, four) {
                blobs.push((hb.blob, src.len()));
            }
        }
        assert!(blobs.len() >= 4);

        for name in names {
            for (blob, dec) in &blobs {
                for _ in 0..45 {
                    let mut v = blob.clone();
                    let k = 1 + rng.below(4);
                    for _ in 0..k {
                        let i = rng.below(v.len());
                        let b = rng.byte();
                        v[i] = b;
                    }
                    let n = if rng.below(4) == 0 { rng.below(v.len() + 1) } else { v.len() };
                    let cap = match rng.below(6) {
                        0 => 0,
                        1 => 1,
                        2 => 1 + rng.below(16),
                        3 => *dec / 2,
                        4 => *dec,
                        _ => *dec + 256,
                    };
                    let w = [0usize, 256, 1024, W / 2, W, W - 4][rng.below(6)];
                    let desc = [0u32, 5, 11, 12, 13][rng.below(5)];
                    diff_dec_dctx(name, "fuzz", desc, cap, &v, n, w);
                }
            }
        }
    }
}

/// Rows 268/269/270/271/283/297: the *fast* decode path
/// (`HUF_DecompressFastArgs_init`, `HUF_initRemainingDStream` and the
/// `args.op[i] != segmentEnd` check in both fast C loops).
///
/// The fast path is only entered when `dtLog == HUF_DECODER_FAST_TABLELOG`
/// (11), `cSrcSize >= 10`, every one of the four segments is >= 8 bytes and
/// `op[3] < oend`. This test builds such a blob, verifies that the fast path is
/// really taken (by showing that `HUF_flags_disableFast` changes the outcome for
/// at least one crafted input) and compares C vs Rust for every mutation.
#[test]
fn err_huf_decompress_fast_path() {
    unsafe {
        let (fc, fr) = duo::<FnHufDecUsingDTable>("HUF_decompress4X_usingDTable");
        let (gc, _) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let src = to_alphabet(&gen_class(4, 120_000, 0x5001), 0);

        let mut differed = false;
        let mut saw_corruption = false;
        let mut rng = Rng::new(0x5002);

        for x2 in [false, true] {
            let hb = c_huf_blob(&src, 0, 11, true).expect("4X blob");
            let dt = c_dtable(&hb.blob[..hb.desc_len], x2, 12);
            let body = hb.blob[hb.desc_len..].to_vec();
            // jump table: all four segments must be >= 8 bytes for the fast loop
            let l1 = u16::from_le_bytes([body[0], body[1]]) as usize;
            let l2 = u16::from_le_bytes([body[2], body[3]]) as usize;
            let l3 = u16::from_le_bytes([body[4], body[5]]) as usize;
            let l4 = body.len() - (l1 + l2 + l3 + 6);
            assert!(
                l1 >= 8 && l2 >= 8 && l3 >= 8 && l4 >= 8,
                "segments too short: {l1}/{l2}/{l3}/{l4}"
            );

            let mut probe = |what: &str, v: &[u8], n: usize, cap: usize| {
                let mut res = [0usize; 2];
                for (k, flags) in [0 as c_int, HUF_flags_disableAsm | HUF_flags_disableFast]
                    .into_iter()
                    .enumerate()
                {
                    let (mut dc, mut dr) = twin(cap.max(1));
                    let a = fc(
                        dc.as_mut_ptr() as *mut c_void,
                        cap,
                        v.as_ptr() as *const c_void,
                        n,
                        dt.as_ptr(),
                        flags,
                    );
                    let b = fr(
                        dr.as_mut_ptr() as *mut c_void,
                        cap,
                        v.as_ptr() as *const c_void,
                        n,
                        dt.as_ptr(),
                        flags,
                    );
                    eqcode(&format!("fast-path {what} (flags={flags:#x})"), a, b);
                    eqbuf(&format!("fast-path {what} (flags={flags:#x}) dst"), &dc, &dr);
                    res[k] = a;
                }
                res
            };

            // sanity
            let r = probe("valid", &body, body.len(), src.len());
            assert_eq!(r[0], src.len());
            assert_eq!(r[1], src.len());

            // row 269 (length4 overflow) inside the fast init
            for l in [0xFFFFu16, 0xF000, 0x8000] {
                let mut v = body.clone();
                v[0..2].copy_from_slice(&l.to_le_bytes());
                let r = probe(&format!("length4 overflow {l}"), &v, v.len(), src.len());
                if is_err(r[0]) && gc(r[0]) == E_corruption_detected {
                    saw_corruption = true;
                }
                if r[0] != r[1] {
                    differed = true;
                }
            }

            // rows 270/271/283/297: corrupt payload bytes inside the segments
            for _ in 0..400 {
                let mut v = body.clone();
                let k = 1 + rng.below(4);
                for _ in 0..k {
                    let i = 6 + rng.below(v.len() - 6);
                    let b = rng.byte();
                    v[i] = b;
                }
                let cap = match rng.below(4) {
                    0 => src.len(),
                    1 => src.len() + 64,
                    2 => src.len() - 1,
                    _ => src.len() / 2,
                };
                let r = probe("fuzz", &v, v.len(), cap);
                if is_err(r[0]) && gc(r[0]) == E_corruption_detected {
                    saw_corruption = true;
                }
                if r[0] != r[1] {
                    differed = true;
                }
            }

            // row 268: srcSize < 10 seen by the fast init
            for n in 0usize..10 {
                probe(&format!("fast srcSize={n}"), &body, n, src.len());
            }
            // dstSize small enough that `op[3] >= oend` and the fast path bails
            for cap in [1usize, 2, 5, 6, 7, 8, 16, 32] {
                probe(&format!("fast dst={cap}"), &body, body.len(), cap);
            }
        }
        assert!(saw_corruption, "the fast path never reported corruption_detected");
        assert!(
            differed,
            "HUF_flags_disableFast never changed the result — the fast loop was not exercised"
        );
    }
}

/// `FSE_buildCTable_rle`, `FSE_compressBound`, `HUF_compressBound`,
/// `HIST_add`, `HIST_count_simple` and `HIST_countFast` inside their documented
/// domains. `HIST_count_simple` / `HIST_countFast` are documented "unsafe :
/// won't check if src contains values beyond count[] limit" (hist.c L104/L172),
/// so they are only called with data that *is* inside `[0, maxSymbolValue]`.
#[test]
fn err_entropy_pure_helpers() {
    unsafe {
        let (rc, rr) = duo::<FnBuildCTableRle>("FSE_buildCTable_rle");
        let (fbc, fbr) = duo::<FnSizeT1>("FSE_compressBound");
        let (hbc, hbr) = duo::<FnSizeT1>("HUF_compressBound");
        let (asc, asr) = duo::<FnHistAdd>("HIST_add");
        let (smc, smr) = duo::<FnHistSimple>("HIST_count_simple");
        let (ffc, ffr) = duo::<FnHistCount>("HIST_countFast");
        let (vc, vr) = duo::<FnUint0>("FSE_versionNumber");

        eqv("FSE_versionNumber", vc(), vr());

        for n in [0usize, 1, 2, 7, 128, 1 << 10, 1 << 20, 1 << 30, usize::MAX / 4] {
            eqv(&format!("FSE_compressBound({n})"), fbc(n), fbr(n));
            eqv(&format!("HUF_compressBound({n})"), hbc(n), hbr(n));
        }

        for sym in [0u8, 1, 7, 128, 254, 255] {
            let (mut cc, mut cr) = twin32(fse_ctable_size_u32(FSE_MAX_TABLELOG, 255) + 8);
            let a = rc(cc.as_mut_ptr(), sym);
            let b = rr(cr.as_mut_ptr(), sym);
            eqcode(&format!("FSE_buildCTable_rle({sym})"), a, b);
            eqbuf(
                &format!("FSE_buildCTable_rle({sym}) ctable"),
                as_bytes32(&cc),
                as_bytes32(&cr),
            );
        }

        let mut rng = Rng::new(0x6001);
        for n in [0usize, 1, 2, 3, 4, 15, 16, 17, 1499, 1500, 1501, 4096] {
            let data = rng.bytes(n.max(1));
            let data = &data[..n];
            // HIST_add: pure accumulation
            let (mut cc, mut cr) = twin32(256);
            asc(cc.as_mut_ptr(), data.as_ptr() as *const c_void, n);
            asr(cr.as_mut_ptr(), data.as_ptr() as *const c_void, n);
            eqbuf(&format!("HIST_add(n={n})"), as_bytes32(&cc), as_bytes32(&cr));

            // HIST_count_simple / HIST_countFast with maxSymbolValue == 255
            let (mut cc, mut cr) = twin32(256);
            let mut mc: c_uint = 255;
            let mut mr: c_uint = 255;
            let a = smc(cc.as_mut_ptr(), &mut mc, data.as_ptr() as *const c_void, n);
            let b = smr(cr.as_mut_ptr(), &mut mr, data.as_ptr() as *const c_void, n);
            eqv(&format!("HIST_count_simple(n={n})"), a, b);
            eqv(&format!("HIST_count_simple(n={n}) *msvPtr"), mc, mr);
            eqbuf(&format!("HIST_count_simple(n={n}) count"), as_bytes32(&cc), as_bytes32(&cr));

            let (mut cc, mut cr) = twin32(256);
            let mut mc: c_uint = 255;
            let mut mr: c_uint = 255;
            let a = ffc(cc.as_mut_ptr(), &mut mc, data.as_ptr() as *const c_void, n);
            let b = ffr(cr.as_mut_ptr(), &mut mr, data.as_ptr() as *const c_void, n);
            eqcode(&format!("HIST_countFast(n={n})"), a, b);
            eqv(&format!("HIST_countFast(n={n}) *msvPtr"), mc, mr);
            eqbuf(&format!("HIST_countFast(n={n}) count"), as_bytes32(&cc), as_bytes32(&cr));

            // restricted alphabets, still inside the documented domain
            for top in [1u32, 3, 15, 63, 127] {
                let d2 = to_alphabet(data, top);
                let (mut cc, mut cr) = twin32(256);
                let mut mc: c_uint = top;
                let mut mr: c_uint = top;
                let a = smc(cc.as_mut_ptr(), &mut mc, d2.as_ptr() as *const c_void, n);
                let b = smr(cr.as_mut_ptr(), &mut mr, d2.as_ptr() as *const c_void, n);
                eqv(&format!("HIST_count_simple(n={n},top={top})"), a, b);
                eqv(&format!("HIST_count_simple(n={n},top={top}) *msvPtr"), mc, mr);
                eqbuf("HIST_count_simple restricted count", as_bytes32(&cc), as_bytes32(&cr));
            }
        }
    }
}
