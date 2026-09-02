//! Phase B: valid-path (happy) differential tests for the low-level entropy
//! primitives — FSE, HUF (huff0) and HIST — exported by both the C `libzstd.so`
//! and the Rust translation.
//!
//! Every call is made through `both::<T>("symbol")` so BOTH shared libraries are
//! exercised across the real FFI boundary. Outputs are compared byte-for-byte
//! and return values are compared through each library's own
//! `FSE_isError` / `HUF_isError` / `HIST_isError` + `getErrorName`.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

type Symbol<T> = libloading::Symbol<'static, T>;

// ---------------------------------------------------------------- signatures

type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnIsErr = unsafe extern "C" fn(size_t) -> c_uint;
type FnGetErrName = unsafe extern "C" fn(size_t) -> *const std::os::raw::c_char;

// HIST
type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnHistCountSimple = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t) -> c_uint;
type FnHistCountWksp =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, size_t, *mut c_void, size_t) -> size_t;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, size_t);

// FSE
type FnOptTableLog = unsafe extern "C" fn(c_uint, size_t, c_uint) -> c_uint;
type FnOptTableLogInternal = unsafe extern "C" fn(c_uint, size_t, c_uint, c_uint) -> c_uint;
type FnNormalize =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, size_t, c_uint, c_uint) -> size_t;
type FnNCountWriteBound = unsafe extern "C" fn(c_uint, c_uint) -> size_t;
type FnWriteNCount =
    unsafe extern "C" fn(*mut c_void, size_t, *const i16, c_uint, c_uint) -> size_t;
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;
type FnReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t, c_int) -> size_t;
type FnBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnBuildCTableRle = unsafe extern "C" fn(*mut c_uint, u8) -> size_t;
type FnBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint) -> size_t;
type FnDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint,
    *mut c_void,
    size_t,
    c_int,
) -> size_t;

// HUF
type FnHufCardinality = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
type FnHufMinTableLog = unsafe extern "C" fn(c_uint) -> c_uint;
type FnHufOptTableLog = unsafe extern "C" fn(
    c_uint,
    size_t,
    c_uint,
    *mut c_void,
    size_t,
    *mut u64,
    *const c_uint,
    c_int,
) -> c_uint;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut u64, *const c_uint, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufWriteCTableWksp =
    unsafe extern "C" fn(*mut c_void, size_t, *const u64, c_uint, c_uint, *mut c_void, size_t) -> size_t;
type FnHufEstimate = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> size_t;
type FnHufValidate = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> c_int;
type FnHufGetNbBits = unsafe extern "C" fn(*const u64, c_uint) -> c_uint;
type FnHufCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const u64, c_int) -> size_t;
type FnHufCompressRepeat = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint,
    c_uint,
    *mut c_void,
    size_t,
    *mut u64,
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
    *mut u64,
    *mut c_uint,
    *const c_void,
    size_t,
    *mut c_uint,
) -> size_t;
type FnHufReadCTableHeader = unsafe extern "C" fn(*const u64) -> HufCTableHeader;
type FnHufSelectDecoder = unsafe extern "C" fn(size_t, size_t) -> c_uint;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const c_void, size_t, *mut c_void, size_t, c_int) -> size_t;
type FnHufDecompressUsingDTable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint, c_int) -> size_t;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *mut c_void,
    size_t,
    c_int,
) -> size_t;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HufCTableHeader {
    pub tableLog: u8,
    pub maxSymbolValue: u8,
    pub unused: [u8; core::mem::size_of::<usize>() - 2],
}

// ------------------------------------------------------------ error helpers

/// A per-family error comparator (FSE_/HUF_/HIST_).
struct EntErr {
    c_is: (Symbol<FnIsErr>, Symbol<FnIsErr>),
    c_name: Option<(Symbol<FnGetErrName>, Symbol<FnGetErrName>)>,
}

impl EntErr {
    unsafe fn new(is_err: &str, get_name: Option<&str>) -> Self {
        EntErr {
            c_is: both::<FnIsErr>(is_err),
            c_name: get_name.map(|n| both::<FnGetErrName>(n)),
        }
    }
    unsafe fn c_err(&self, r: size_t) -> bool {
        (self.c_is.0)(r) != 0
    }
    /// Assert C and Rust returns are equivalent: both ok with equal value, or
    /// both errors with equal error-name string.
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let ce = (self.c_is.0)(cr) != 0;
        let re = (self.c_is.1)(rr) != 0;
        assert_eq!(ce, re, "{ctx}: isError mismatch C={ce}({cr:#x}) RS={re}({rr:#x})");
        if ce {
            if let Some((cn, rn)) = &self.c_name {
                let cs = cstr(cn(cr));
                let rs = cstr(rn(rr));
                assert_eq!(cs, rs, "{ctx}: error name mismatch C={cs:?} RS={rs:?}");
            }
        } else {
            assert_eq!(cr, rr, "{ctx}: value mismatch C={cr:#x} RS={rr:#x}");
        }
    }
}

fn fse_err() -> EntErr {
    unsafe { EntErr::new("FSE_isError", Some("FSE_getErrorName")) }
}
fn huf_err() -> EntErr {
    unsafe { EntErr::new("HUF_isError", Some("HUF_getErrorName")) }
}
fn hist_err() -> EntErr {
    unsafe { EntErr::new("HIST_isError", None) }
}

// ------------------------------------------------------------------ constants

const FSE_MAX_TABLELOG: c_uint = 12; // FSE_MAX_MEMORY_USAGE(14) - 2
const FSE_MIN_TABLELOG: c_uint = 5;
const FSE_MAX_SYMBOL_VALUE: c_uint = 255;
const HUF_TABLELOG_MAX: c_uint = 12;
const HUF_SYMBOLVALUE_MAX: c_uint = 255;

const MAXSYMS: &[c_uint] = &[0, 1, 2, 15, 127, 255];

fn HUF_flags_disableAsm() -> c_int {
    1 << 4
}
fn HUF_flags_disableFast() -> c_int {
    1 << 5
}

/// FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)
fn fse_ctable_wksp_u32(msv: c_uint, tl: c_uint) -> usize {
    ((msv as usize + 2) + (1usize << tl)) / 2 + 2
}
/// FSE_BUILD_DTABLE_WKSP_SIZE_U32(maxTableLog, maxSymbolValue)
fn fse_dtable_wksp_u32(tl: c_uint, msv: c_uint) -> usize {
    let bytes = 2 * (msv as usize + 1) + (1usize << tl) + 8;
    bytes.div_ceil(4)
}
/// FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue)
fn fse_ctable_size_u32(tl: c_uint, msv: c_uint) -> usize {
    1 + (1usize << (tl.max(1) - 1)) + (msv as usize + 1) * 2
}
/// FSE_DTABLE_SIZE_U32(maxTableLog)
fn fse_dtable_size_u32(tl: c_uint) -> usize {
    1 + (1usize << tl)
}
/// FSE_DECOMPRESS_WKSP_SIZE_U32(maxTableLog, maxSymbolValue)
fn fse_decompress_wksp_u32(tl: c_uint, msv: c_uint) -> usize {
    fse_dtable_size_u32(tl) + 1 + fse_dtable_wksp_u32(tl, msv) + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2 + 1
}

const HUF_WORKSPACE_SIZE_U64: usize = ((8 << 10) + 512) / 8;
const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = (4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192;
const HUF_CTABLE_SIZE_ST: usize = HUF_SYMBOLVALUE_MAX as usize + 2; // U64 elts, worst case
const HUF_DECOMPRESS_WORKSPACE_SIZE_U32: usize = ((2 << 10) + (1 << 9)) / 4;

// ---------------------------------------------------------------- HIST tests

#[test]
fn hist_count_all_variants() {
    unsafe {
        let e = hist_err();
        let (cc, rc) = both::<FnHistCount>("HIST_count");
        let (ccs, rcs) = both::<FnHistCountSimple>("HIST_count_simple");
        let (ccw, rcw) = both::<FnHistCountWksp>("HIST_count_wksp");
        let (ccf, rcf) = both::<FnHistCount>("HIST_countFast");
        let (ccfw, rcfw) = both::<FnHistCountWksp>("HIST_countFast_wksp");
        let (cadd, radd) = both::<FnHistAdd>("HIST_add");

        let mut rng = Rng::new(0xE0_0001);
        for &shape in ALL_SHAPES {
            for &len in LENS {
                let src = gen(shape, len, &mut rng);
                let sp = src.as_ptr() as *const c_void;
                let ssz = src.len(); // NEVER pass `len` — Shape::Empty ignores it.

                for &msv in MAXSYMS {
                    // HIST_count — safe, updates maxSymbolValue.
                    let mut cnt_c = vec![0u32; 256];
                    let mut cnt_r = vec![0u32; 256];
                    let mut m1 = msv;
                    let mut m2 = msv;
                    let a = cc(cnt_c.as_mut_ptr(), &mut m1, sp, ssz);
                    let b = rc(cnt_r.as_mut_ptr(), &mut m2, sp, ssz);
                    let ctx = format!("HIST_count shape={shape:?} len={len} msv={msv}");
                    e.eq(&ctx, a, b);
                    assert_eq!(m1, m2, "{ctx}: maxSymbolValue out");
                    if !e.c_err(a) {
                        assert_eq!(cnt_c, cnt_r, "{ctx}: counts");
                    }

                    // HIST_count_wksp
                    let mut cnt_c = vec![0u32; 256];
                    let mut cnt_r = vec![0u32; 256];
                    let mut m1 = msv;
                    let mut m2 = msv;
                    let mut wc = vec![0u32; 1024];
                    let mut wr = vec![0u32; 1024];
                    let a = ccw(cnt_c.as_mut_ptr(), &mut m1, sp, ssz, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                    let b = rcw(cnt_r.as_mut_ptr(), &mut m2, sp, ssz, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                    let ctx = format!("HIST_count_wksp shape={shape:?} len={len} msv={msv}");
                    e.eq(&ctx, a, b);
                    assert_eq!(m1, m2, "{ctx}: maxSymbolValue out");
                    if !e.c_err(a) {
                        assert_eq!(cnt_c, cnt_r, "{ctx}: counts");
                    }
                }

                // Fast variants require maxSymbolValue >= every byte present; use 255.
                let mut cnt_c = vec![0u32; 256];
                let mut cnt_r = vec![0u32; 256];
                let mut m1 = 255u32;
                let mut m2 = 255u32;
                let a = ccf(cnt_c.as_mut_ptr(), &mut m1, sp, ssz);
                let b = rcf(cnt_r.as_mut_ptr(), &mut m2, sp, ssz);
                let ctx = format!("HIST_countFast shape={shape:?} len={len}");
                e.eq(&ctx, a, b);
                assert_eq!(m1, m2, "{ctx}: maxSymbolValue out");
                if !e.c_err(a) {
                    assert_eq!(cnt_c, cnt_r, "{ctx}: counts");
                }

                let mut cnt_c = vec![0u32; 256];
                let mut cnt_r = vec![0u32; 256];
                let mut m1 = 255u32;
                let mut m2 = 255u32;
                let mut wc = vec![0u32; 1024];
                let mut wr = vec![0u32; 1024];
                let a = ccfw(cnt_c.as_mut_ptr(), &mut m1, sp, ssz, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                let b = rcfw(cnt_r.as_mut_ptr(), &mut m2, sp, ssz, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                let ctx = format!("HIST_countFast_wksp shape={shape:?} len={len}");
                e.eq(&ctx, a, b);
                assert_eq!(m1, m2, "{ctx}: maxSymbolValue out");
                if !e.c_err(a) {
                    assert_eq!(cnt_c, cnt_r, "{ctx}: counts");
                }

                // HIST_count_simple — returns most-frequent count, no error path.
                let mut cnt_c = vec![0u32; 256];
                let mut cnt_r = vec![0u32; 256];
                let mut m1 = 255u32;
                let mut m2 = 255u32;
                let a = ccs(cnt_c.as_mut_ptr(), &mut m1, sp, ssz);
                let b = rcs(cnt_r.as_mut_ptr(), &mut m2, sp, ssz);
                let ctx = format!("HIST_count_simple shape={shape:?} len={len}");
                assert_eq!(a, b, "{ctx}: return");
                assert_eq!(m1, m2, "{ctx}: maxSymbolValue out");
                assert_eq!(cnt_c, cnt_r, "{ctx}: counts");

                // HIST_add — accumulates into a 256-cell table (void return).
                let mut cnt_c = vec![7u32; 256];
                let mut cnt_r = vec![7u32; 256];
                cadd(cnt_c.as_mut_ptr(), sp, ssz);
                radd(cnt_r.as_mut_ptr(), sp, ssz);
                assert_eq!(cnt_c, cnt_r, "HIST_add shape={shape:?} len={len}");
            }
        }
    }
}

// ------------------------------------------------------ FSE optimalTableLog

#[test]
fn fse_optimal_table_log() {
    unsafe {
        let (co, ro) = both::<FnOptTableLog>("FSE_optimalTableLog");
        let (coi, roi) = both::<FnOptTableLogInternal>("FSE_optimalTableLog_internal");
        let mut rng = Rng::new(0xE0_0002);
        // srcSize must be > 1: the C code asserts this (RLE is used for <=1) and
        // its <=1 behaviour relies on ZSTD_highbit32(0) which is UB in C, so it
        // is not a meaningful differential target.
        let srcs: &[size_t] = &[2, 3, 10, 100, 1000, 65535, 100_000, 1 << 20];
        for &maxtl in &[0u32, 1, 5, 6, 9, 11, 12, 13, 20, 100] {
            for &s in srcs {
                for &msv in MAXSYMS {
                    assert_eq!(co(maxtl, s, msv), ro(maxtl, s, msv),
                        "FSE_optimalTableLog({maxtl},{s},{msv})");
                    for minus in [0u32, 1, 2, 3] {
                        assert_eq!(coi(maxtl, s, msv, minus), roi(maxtl, s, msv, minus),
                            "FSE_optimalTableLog_internal({maxtl},{s},{msv},{minus})");
                    }
                }
            }
        }
        for _ in 0..3000 {
            let maxtl = rng.range(0, 20) as c_uint;
            let s = rng.below(200_000) + 2;
            let msv = rng.range(0, 255) as c_uint;
            let minus = rng.range(0, 3) as c_uint;
            assert_eq!(co(maxtl, s, msv), ro(maxtl, s, msv), "FSE_optimalTableLog rand");
            assert_eq!(coi(maxtl, s, msv, minus), roi(maxtl, s, msv, minus),
                "FSE_optimalTableLog_internal rand");
        }
    }
}

#[test]
fn fse_ncount_write_bound() {
    unsafe {
        let (cb, rb) = both::<FnNCountWriteBound>("FSE_NCountWriteBound");
        for msv in 0u32..=256 {
            for tl in 0u32..=15 {
                assert_eq!(cb(msv, tl), rb(msv, tl), "FSE_NCountWriteBound({msv},{tl})");
            }
        }
        // FSE_compressBound over boundary + random sizes.
        let (ccb, rcb) = both::<FnCompressBound>("FSE_compressBound");
        let mut cases: Vec<usize> = LENS.to_vec();
        cases.extend([usize::MAX, usize::MAX / 2, 1 << 20, 1 << 30, 0x7fff_ffff]);
        let mut rng = Rng::new(0xE0_00B0);
        for _ in 0..500 {
            cases.push(rng.next_u64() as usize);
        }
        for &n in &cases {
            assert_eq!(ccb(n), rcb(n), "FSE_compressBound({n})");
        }
    }
}

/// Build a histogram + normalized counter for `src` via the C library and
/// return `(count[256], maxSymbolValue, tableLog, normalizedCounter[256])`.
unsafe fn build_norm(
    src: &[u8],
    tl_req: c_uint,
    lowprob: c_uint,
) -> Option<(Vec<u32>, c_uint, c_uint, Vec<i16>)> {
    let (cc, _) = both::<FnHistCount>("HIST_count");
    let (co, _) = both::<FnOptTableLog>("FSE_optimalTableLog");
    let (cn, _) = both::<FnNormalize>("FSE_normalizeCount");
    let ferr = fse_err();
    let herr = hist_err();

    let mut count = vec![0u32; 256];
    let mut msv = 255u32;
    let max = cc(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
    if herr.c_err(max) {
        return None;
    }
    if max == src.len() || src.is_empty() {
        // single symbol / empty — normalizeCount will error; skip fixture.
        return None;
    }
    let tl = if tl_req == 0 { co(0, src.len(), msv) } else { tl_req };
    let mut norm = vec![0i16; 256];
    let r = cn(norm.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob);
    if ferr.c_err(r) {
        return None;
    }
    Some((count, msv, r as c_uint, norm))
}

#[test]
fn fse_normalize_count_differential() {
    unsafe {
        let e = fse_err();
        let (cc, _) = both::<FnHistCount>("HIST_count");
        let (cn, rn) = both::<FnNormalize>("FSE_normalizeCount");
        let mut rng = Rng::new(0xE0_0003);
        for &shape in ALL_SHAPES {
            for &len in LENS {
                let src = gen(shape, len, &mut rng);
                // FSE_normalizeCount divides by srcSize and requires >=2 distinct
                // symbols; empty / single-symbol inputs are an unsupported
                // precondition that makes BOTH the C and Rust libraries divide by
                // zero (SIGFPE). Skip those — RLE is used for them instead.
                let mut count = vec![0u32; 256];
                let mut msv = 255u32;
                let max = cc(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
                if src.is_empty() || max == src.len() {
                    continue;
                }
                for tl in [0u32, FSE_MIN_TABLELOG, 6, 8, 10, FSE_MAX_TABLELOG] {
                    for lowprob in [0u32, 1] {
                        let mut nc = vec![0i16; 256];
                        let mut nr = vec![0i16; 256];
                        let a = cn(nc.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob);
                        let b = rn(nr.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob);
                        let ctx = format!("FSE_normalizeCount shape={shape:?} len={len} tl={tl} lp={lowprob}");
                        e.eq(&ctx, a, b);
                        if !e.c_err(a) {
                            assert_eq!(nc, nr, "{ctx}: normalizedCounter");
                        }
                    }
                }
            }
        }
    }
}

/// Full FSE round-trip through the exported sub-primitives:
/// writeNCount -> readNCount -> buildCTable_wksp -> compress_usingCTable ->
/// buildDTable_wksp -> decompress_wksp_bmi2. Every step compared C vs Rust.
#[test]
fn fse_writeread_build_compress_decompress() {
    unsafe {
        let e = fse_err();
        let (cw, rw) = both::<FnWriteNCount>("FSE_writeNCount");
        let (crd, rrd) = both::<FnReadNCount>("FSE_readNCount");
        let (crb, rrb) = both::<FnReadNCountBmi2>("FSE_readNCount_bmi2");
        let (cbc, rbc) = both::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
        let (cbd, rbd) = both::<FnBuildDTableWksp>("FSE_buildDTable_wksp");
        let (ccp, rcp) = both::<FnCompressUsingCTable>("FSE_compress_usingCTable");
        let (cdw, rdw) = both::<FnDecompressWkspBmi2>("FSE_decompress_wksp_bmi2");
        let (crle, rrle) = both::<FnBuildCTableRle>("FSE_buildCTable_rle");
        let (cbnd, _) = both::<FnNCountWriteBound>("FSE_NCountWriteBound");

        let mut rng = Rng::new(0xE0_0004);
        for &shape in ALL_SHAPES {
            for &len in &[13usize, 64, 100, 256, 512, 1000, 4096, 20000, 65535] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                for tl_req in [0u32, FSE_MIN_TABLELOG, 7, 9, FSE_MAX_TABLELOG] {
                    let Some((_count, msv, tl, norm)) = build_norm(&src, tl_req, 1) else { continue };

                    // --- writeNCount ---
                    let bound = cbnd(msv, tl);
                    let mut hc = vec![0u8; bound + 8];
                    let mut hr = vec![0u8; bound + 8];
                    let a = cw(hc.as_mut_ptr() as *mut c_void, hc.len(), norm.as_ptr(), msv, tl);
                    let b = rw(hr.as_mut_ptr() as *mut c_void, hr.len(), norm.as_ptr(), msv, tl);
                    let ctx = format!("FSE_writeNCount shape={shape:?} len={len} tl={tl}");
                    e.eq(&ctx, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_bytes_eq(&ctx, &hc[..a], &hr[..b]);
                    let hsize = a;

                    // --- readNCount / readNCount_bmi2 ---
                    for bmi2 in [0i32, 1] {
                        let mut n1 = vec![0i16; 256];
                        let mut n2 = vec![0i16; 256];
                        let (mut m1, mut m2) = (255u32, 255u32);
                        let (mut t1, mut t2) = (0u32, 0u32);
                        let a = crb(n1.as_mut_ptr(), &mut m1, &mut t1, hc.as_ptr() as *const c_void, hsize, bmi2);
                        let b = rrb(n2.as_mut_ptr(), &mut m2, &mut t2, hr.as_ptr() as *const c_void, hsize, bmi2);
                        let ctx = format!("FSE_readNCount_bmi2 shape={shape:?} tl={tl} bmi2={bmi2}");
                        e.eq(&ctx, a, b);
                        if !e.c_err(a) {
                            assert_eq!(m1, m2, "{ctx}: msv");
                            assert_eq!(t1, t2, "{ctx}: tableLog");
                            assert_eq!(&n1[..=m1 as usize], &n2[..=m2 as usize], "{ctx}: norm");
                        }
                    }
                    let mut n1 = vec![0i16; 256];
                    let mut n2 = vec![0i16; 256];
                    let (mut m1, mut m2) = (255u32, 255u32);
                    let (mut t1, mut t2) = (0u32, 0u32);
                    let a = crd(n1.as_mut_ptr(), &mut m1, &mut t1, hc.as_ptr() as *const c_void, hsize);
                    let b = rrd(n2.as_mut_ptr(), &mut m2, &mut t2, hr.as_ptr() as *const c_void, hsize);
                    e.eq(&format!("FSE_readNCount shape={shape:?} tl={tl}"), a, b);

                    // --- buildCTable_wksp ---
                    let ct_u32 = fse_ctable_size_u32(tl, msv);
                    let wk = fse_ctable_wksp_u32(msv, tl) + 8;
                    let mut ctc = vec![0u32; ct_u32 + 8];
                    let mut ctr = vec![0u32; ct_u32 + 8];
                    let mut wc = vec![0u32; wk];
                    let mut wr = vec![0u32; wk];
                    let a = cbc(ctc.as_mut_ptr(), norm.as_ptr(), msv, tl, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                    let b = rbc(ctr.as_mut_ptr(), norm.as_ptr(), msv, tl, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                    let ctx = format!("FSE_buildCTable_wksp shape={shape:?} tl={tl}");
                    e.eq(&ctx, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_eq!(ctc, ctr, "{ctx}: CTable bytes");

                    // --- compress_usingCTable ---
                    let cap = src.len() + 512;
                    let mut oc = vec![0u8; cap];
                    let mut or = vec![0u8; cap];
                    let a = ccp(oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ctc.as_ptr());
                    let b = rcp(or.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ctr.as_ptr());
                    let ctx = format!("FSE_compress_usingCTable shape={shape:?} tl={tl}");
                    e.eq(&ctx, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_bytes_eq(&ctx, &oc[..a], &or[..b]);
                    let csize = a;
                    if csize == 0 {
                        continue; // incompressible sentinel
                    }

                    // --- buildDTable_wksp ---
                    let dt_u32 = fse_dtable_size_u32(tl);
                    let dwk = fse_dtable_wksp_u32(tl, msv) + 8;
                    let mut dtc = vec![0u32; dt_u32 + 8];
                    let mut dtr = vec![0u32; dt_u32 + 8];
                    let mut wc = vec![0u32; dwk];
                    let mut wr = vec![0u32; dwk];
                    let a = cbd(dtc.as_mut_ptr(), norm.as_ptr(), msv, tl, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                    let b = rbd(dtr.as_mut_ptr(), norm.as_ptr(), msv, tl, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                    let ctx = format!("FSE_buildDTable_wksp shape={shape:?} tl={tl}");
                    e.eq(&ctx, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_eq!(dtc, dtr, "{ctx}: DTable bytes");

                    // --- decompress_wksp_bmi2 (regenerate original) ---
                    // FSE_decompress_wksp expects a full FSE frame: the NCount
                    // header (from FSE_writeNCount) immediately followed by the
                    // entropy payload (from FSE_compress_usingCTable). The header
                    // bytes are byte-identical between C and RS (asserted above),
                    // so build the frame once and feed it to both.
                    let mut frame = Vec::with_capacity(hsize + csize);
                    frame.extend_from_slice(&hc[..hsize]);
                    frame.extend_from_slice(&oc[..csize]);
                    for bmi2 in [0i32, 1] {
                        let dwk2 = fse_decompress_wksp_u32(tl, msv) + 16;
                        let mut d1 = vec![0u8; src.len() + 16];
                        let mut d2 = vec![0u8; src.len() + 16];
                        let mut wc = vec![0u32; dwk2];
                        let mut wr = vec![0u32; dwk2];
                        let a = cdw(d1.as_mut_ptr() as *mut c_void, d1.len(), frame.as_ptr() as *const c_void, frame.len(), tl, wc.as_mut_ptr() as *mut c_void, wc.len() * 4, bmi2);
                        let b = rdw(d2.as_mut_ptr() as *mut c_void, d2.len(), frame.as_ptr() as *const c_void, frame.len(), tl, wr.as_mut_ptr() as *mut c_void, wr.len() * 4, bmi2);
                        let ctx = format!("FSE_decompress_wksp_bmi2 shape={shape:?} tl={tl} bmi2={bmi2}");
                        e.eq(&ctx, a, b);
                        if !e.c_err(a) {
                            assert_bytes_eq(&ctx, &d1[..a], &d2[..b]);
                            assert_eq!(&d1[..a], &src[..], "{ctx}: roundtrip");
                        }
                    }
                }
            }
        }

        // FSE_buildCTable_rle — builds a fake single-symbol CTable.
        for sym in [0u8, 1, 42, 127, 200, 255] {
            let mut ctc = vec![0u32; 16];
            let mut ctr = vec![0u32; 16];
            let a = crle(ctc.as_mut_ptr(), sym);
            let b = rrle(ctr.as_mut_ptr(), sym);
            e.eq(&format!("FSE_buildCTable_rle({sym})"), a, b);
            assert_eq!(ctc, ctr, "FSE_buildCTable_rle({sym}) bytes");
        }
    }
}

// ----------------------------------------------------------------- HUF tests

#[test]
fn huf_compress_bound_and_cardinality() {
    unsafe {
        let (cb, rb) = both::<FnCompressBound>("HUF_compressBound");
        let mut cases: Vec<usize> = LENS.to_vec();
        cases.extend([usize::MAX, usize::MAX / 2, 1 << 20, 1 << 30]);
        for &n in &cases {
            assert_eq!(cb(n), rb(n), "HUF_compressBound({n})");
        }
        let (cc, rc) = both::<FnHufCardinality>("HUF_cardinality");
        let (cm, rm) = both::<FnHufMinTableLog>("HUF_minTableLog");
        let (chc, _) = both::<FnHistCount>("HIST_count");
        let mut rng = Rng::new(0xE0_0005);
        for &shape in ALL_SHAPES {
            for &len in &[1usize, 64, 1000, 20000] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                let mut count = vec![0u32; 256];
                let mut msv = 255u32;
                let _ = chc(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
                assert_eq!(cc(count.as_ptr(), msv), rc(count.as_ptr(), msv),
                    "HUF_cardinality shape={shape:?} len={len}");
                let card = cc(count.as_ptr(), msv);
                assert_eq!(cm(card), rm(card), "HUF_minTableLog({card})");
            }
        }
        // cardinality >= 1: HUF_minTableLog(0) computes ZSTD_highbit32(0),
        // which the C code guards with assert(val != 0) (UB when violated), so
        // it is not a meaningful differential target. A real histogram always
        // has cardinality >= 1.
        for card in 1u32..=256 {
            assert_eq!(cm(card), rm(card), "HUF_minTableLog({card})");
        }
    }
}

/// Build a HUF CTable through C for use as a fixture, returning
/// (count[256], maxSymbolValue, huffLog, CTable as Vec<u64>).
unsafe fn build_huf_ctable(src: &[u8], huff_log: c_uint) -> Option<(Vec<u32>, c_uint, c_uint, Vec<u64>)> {
    let (chc, _) = both::<FnHistCount>("HIST_count");
    let (cbc, _) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");
    let herr = huf_err();
    let hi = hist_err();

    let mut count = vec![0u32; 256];
    let mut msv = 255u32;
    let max = chc(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
    if hi.c_err(max) || max == src.len() || src.is_empty() {
        return None;
    }
    let mut ctable = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
    let mut wksp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
    let r = cbc(ctable.as_mut_ptr(), count.as_ptr(), msv, huff_log, wksp.as_mut_ptr() as *mut c_void, wksp.len() * 4);
    if herr.c_err(r) {
        return None;
    }
    Some((count, msv, huff_log, ctable))
}

#[test]
fn huf_build_write_read_ctable() {
    unsafe {
        let e = huf_err();
        let (cbc, rbc) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");
        let (cwc, rwc) = both::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp");
        let (crc, rrc) = both::<FnHufReadCTable>("HUF_readCTable");
        let (crh, rrh) = both::<FnHufReadCTableHeader>("HUF_readCTableHeader");
        let (cnb, rnb) = both::<FnHufGetNbBits>("HUF_getNbBitsFromCTable");
        let (cval, rval) = both::<FnHufValidate>("HUF_validateCTable");
        let (cest, rest) = both::<FnHufEstimate>("HUF_estimateCompressedSize");
        let (chc, _) = both::<FnHistCount>("HIST_count");
        let (ccard, _) = both::<FnHufCardinality>("HUF_cardinality");
        let (cmin, _) = both::<FnHufMinTableLog>("HUF_minTableLog");

        let mut rng = Rng::new(0xE0_0006);
        for &shape in ALL_SHAPES {
            for &len in &[13usize, 64, 256, 1000, 20000, 100_000] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                let mut count = vec![0u32; 256];
                let mut msv = 255u32;
                let max = chc(count.as_mut_ptr(), &mut msv, src.as_ptr() as *const c_void, src.len());
                if max == src.len() {
                    continue;
                }
                // HUF_buildCTable_wksp trusts that `huffLog` is large enough to
                // represent every symbol; too-small a huffLog is an unsupported
                // precondition that segfaults BOTH libraries. The minimum safe
                // value is HUF_minTableLog(cardinality).
                let card = ccard(count.as_ptr(), msv);
                let min_log = cmin(card).max(1);
                for huff_log in min_log..=HUF_TABLELOG_MAX {
                    // --- buildCTable_wksp ---
                    let mut ctc = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut ctr = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut wc = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                    let mut wr = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                    let a = cbc(ctc.as_mut_ptr(), count.as_ptr(), msv, huff_log, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                    let b = rbc(ctr.as_mut_ptr(), count.as_ptr(), msv, huff_log, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                    let ctx = format!("HUF_buildCTable_wksp shape={shape:?} len={len} hl={huff_log}");
                    e.eq(&ctx, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_eq!(ctc, ctr, "{ctx}: CTable bytes");

                    // The build return value is the actual tableLog used.
                    let used_log = a as c_uint;

                    // --- readCTableHeader ---
                    let h1 = crh(ctc.as_ptr());
                    let h2 = rrh(ctr.as_ptr());
                    assert_eq!(h1, h2, "{ctx}: readCTableHeader");

                    // --- getNbBitsFromCTable over all symbol values (0..=257) ---
                    for sym in 0u32..=257 {
                        assert_eq!(cnb(ctc.as_ptr(), sym), rnb(ctr.as_ptr(), sym),
                            "{ctx}: getNbBits({sym})");
                    }

                    // --- validateCTable / estimateCompressedSize ---
                    assert_eq!(cval(ctc.as_ptr(), count.as_ptr(), msv), rval(ctr.as_ptr(), count.as_ptr(), msv),
                        "{ctx}: validateCTable");
                    assert_eq!(cest(ctc.as_ptr(), count.as_ptr(), msv), rest(ctr.as_ptr(), count.as_ptr(), msv),
                        "{ctx}: estimateCompressedSize");

                    // --- writeCTable_wksp ---
                    let mut hc = vec![0u8; 512];
                    let mut hr = vec![0u8; 512];
                    let mut wc = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                    let mut wr = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                    let a = cwc(hc.as_mut_ptr() as *mut c_void, hc.len(), ctc.as_ptr(), msv, used_log, wc.as_mut_ptr() as *mut c_void, wc.len() * 4);
                    let b = rwc(hr.as_mut_ptr() as *mut c_void, hr.len(), ctr.as_ptr(), msv, used_log, wr.as_mut_ptr() as *mut c_void, wr.len() * 4);
                    let ctx2 = format!("HUF_writeCTable_wksp shape={shape:?} len={len} hl={huff_log}");
                    e.eq(&ctx2, a, b);
                    if e.c_err(a) {
                        continue;
                    }
                    assert_bytes_eq(&ctx2, &hc[..a], &hr[..b]);
                    let hsize = a;

                    // --- readCTable (round-trip the written header) ---
                    let mut ct2c = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut ct2r = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let (mut m1, mut m2) = (255u32, 255u32);
                    let (mut z1, mut z2) = (0u32, 0u32);
                    let a = crc(ct2c.as_mut_ptr(), &mut m1, hc.as_ptr() as *const c_void, hsize, &mut z1);
                    let b = rrc(ct2r.as_mut_ptr(), &mut m2, hr.as_ptr() as *const c_void, hsize, &mut z2);
                    let ctx3 = format!("HUF_readCTable shape={shape:?} len={len} hl={huff_log}");
                    e.eq(&ctx3, a, b);
                    if !e.c_err(a) {
                        assert_eq!(m1, m2, "{ctx3}: msv");
                        assert_eq!(z1, z2, "{ctx3}: hasZeroWeights");
                    }
                }
            }
        }
    }
}

/// Full HUF round-trip: optimalTableLog, compress1X/4X_usingCTable + repeat,
/// readStats, selectDecoder, readDTableX1/X2, decompress1X/4X_usingDTable,
/// and the DCtx_wksp one-shot decoders. Everything compared C vs Rust.
#[test]
fn huf_compress_decompress_roundtrip() {
    unsafe {
        let e = huf_err();
        let (cot, rot) = both::<FnHufOptTableLog>("HUF_optimalTableLog");
        let (cc1, rc1) = both::<FnHufCompressUsingCTable>("HUF_compress1X_usingCTable");
        let (cc4, rc4) = both::<FnHufCompressUsingCTable>("HUF_compress4X_usingCTable");
        let (cr1, rr1) = both::<FnHufCompressRepeat>("HUF_compress1X_repeat");
        let (cr4, rr4) = both::<FnHufCompressRepeat>("HUF_compress4X_repeat");
        let (cwc, _) = both::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp");
        let (crs, rrs) = both::<FnHufReadStats>("HUF_readStats");
        let (crsw, rrsw) = both::<FnHufReadStatsWksp>("HUF_readStats_wksp");
        let (csd, rsd) = both::<FnHufSelectDecoder>("HUF_selectDecoder");
        let (crd1, rrd1) = both::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp");
        let (crd2, rrd2) = both::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp");
        let (cdu1, rdu1) = both::<FnHufDecompressUsingDTable>("HUF_decompress1X_usingDTable");
        let (cdu4, rdu4) = both::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable");
        let (cd1w, rd1w) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X1_DCtx_wksp");
        let (cd1x2, rd1x2) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X2_DCtx_wksp");
        let (cd1g, rd1g) = both::<FnHufDecompressDCtxWksp>("HUF_decompress1X_DCtx_wksp");
        let (cd4h, rd4h) = both::<FnHufDecompressDCtxWksp>("HUF_decompress4X_hufOnly_wksp");
        let (cbc, _) = both::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp");

        let mut rng = Rng::new(0xE0_0007);
        for &shape in ALL_SHAPES {
            for &len in &[64usize, 256, 1000, 4096, 20000, 100_000] {
                let src = gen(shape, len, &mut rng);
                if src.len() < 2 {
                    continue;
                }
                let (count, msv, _hl, ct_fixture) = match build_huf_ctable(&src, HUF_TABLELOG_MAX) {
                    Some(x) => x,
                    None => continue,
                };

                // HUF_optimalTableLog needs table scratch + count.
                for maxtl in [HUF_TABLELOG_MAX, 11, 9] {
                    let mut tblc = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut tblr = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut wc = vec![0u32; HUF_WORKSPACE_SIZE_U64 * 2 + 16];
                    let mut wr = vec![0u32; HUF_WORKSPACE_SIZE_U64 * 2 + 16];
                    let a = cot(maxtl, src.len(), msv, wc.as_mut_ptr() as *mut c_void, wc.len() * 4, tblc.as_mut_ptr(), count.as_ptr(), 0);
                    let b = rot(maxtl, src.len(), msv, wr.as_mut_ptr() as *mut c_void, wr.len() * 4, tblr.as_mut_ptr(), count.as_ptr(), 0);
                    assert_eq!(a, b, "HUF_optimalTableLog shape={shape:?} len={len} maxtl={maxtl}");
                }

                // Write the header so we can round-trip through readStats & readDTable.
                let used_log = {
                    let mut ct2 = vec![0u64; HUF_CTABLE_SIZE_ST + 4];
                    let mut w2 = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                    let ul = cbc(ct2.as_mut_ptr(), count.as_ptr(), msv, HUF_TABLELOG_MAX, w2.as_mut_ptr() as *mut c_void, w2.len() * 4);
                    if e.c_err(ul) {
                        continue;
                    }
                    ul as c_uint
                };
                let mut header = vec![0u8; 512];
                let mut wtmp = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32 + 8];
                let hn = cwc(header.as_mut_ptr() as *mut c_void, header.len(), ct_fixture.as_ptr(), msv, used_log, wtmp.as_mut_ptr() as *mut c_void, wtmp.len() * 4);
                if e.c_err(hn) {
                    continue;
                }
                header.truncate(hn);

                // --- readStats / readStats_wksp ---
                {
                    let mut hw1 = vec![0u8; 256];
                    let mut hw2 = vec![0u8; 256];
                    let mut rk1 = vec![0u32; 16];
                    let mut rk2 = vec![0u32; 16];
                    let (mut ns1, mut ns2) = (0u32, 0u32);
                    let (mut tl1, mut tl2) = (0u32, 0u32);
                    let a = crs(hw1.as_mut_ptr(), 256, rk1.as_mut_ptr(), &mut ns1, &mut tl1, header.as_ptr() as *const c_void, header.len());
                    let b = rrs(hw2.as_mut_ptr(), 256, rk2.as_mut_ptr(), &mut ns2, &mut tl2, header.as_ptr() as *const c_void, header.len());
                    let ctx = format!("HUF_readStats shape={shape:?} len={len}");
                    e.eq(&ctx, a, b);
                    if !e.c_err(a) {
                        assert_eq!(hw1, hw2, "{ctx}: huffWeight");
                        assert_eq!(rk1, rk2, "{ctx}: rankStats");
                        assert_eq!(ns1, ns2, "{ctx}: nbSymbols");
                        assert_eq!(tl1, tl2, "{ctx}: tableLog");
                    }
                    for flags in [0i32, 1] {
                        let mut hw1 = vec![0u8; 256];
                        let mut hw2 = vec![0u8; 256];
                        let mut rk1 = vec![0u32; 16];
                        let mut rk2 = vec![0u32; 16];
                        let (mut ns1, mut ns2) = (0u32, 0u32);
                        let (mut tl1, mut tl2) = (0u32, 0u32);
                        let mut wc = vec![0u32; 1024];
                        let mut wr = vec![0u32; 1024];
                        let a = crsw(hw1.as_mut_ptr(), 256, rk1.as_mut_ptr(), &mut ns1, &mut tl1, header.as_ptr() as *const c_void, header.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                        let b = rrsw(hw2.as_mut_ptr(), 256, rk2.as_mut_ptr(), &mut ns2, &mut tl2, header.as_ptr() as *const c_void, header.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                        let ctx = format!("HUF_readStats_wksp shape={shape:?} len={len} flags={flags}");
                        e.eq(&ctx, a, b);
                        if !e.c_err(a) {
                            assert_eq!(hw1, hw2, "{ctx}: huffWeight");
                            assert_eq!(rk1, rk2, "{ctx}: rankStats");
                        }
                    }
                }

                let cap = {
                    let (cb, _) = both::<FnCompressBound>("HUF_compressBound");
                    cb(src.len()) + 64
                };

                // --- compress1X/4X_usingCTable + round-trip decode ---
                for flags in [0i32, HUF_flags_disableAsm(), HUF_flags_disableFast()] {
                    let mut o1c = vec![0u8; cap];
                    let mut o1r = vec![0u8; cap];
                    let a = cc1(o1c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct_fixture.as_ptr(), flags);
                    let b = rc1(o1r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct_fixture.as_ptr(), flags);
                    let ctx = format!("HUF_compress1X_usingCTable shape={shape:?} len={len} flags={flags}");
                    e.eq(&ctx, a, b);
                    if !e.c_err(a) {
                        assert_bytes_eq(&ctx, &o1c[..a], &o1r[..b]);
                        if a > 0 {
                            round_trip_1x(&e, &src, &o1c[..a], &header, flags,
                                (&csd, &rsd), (&crd1, &rrd1), (&crd2, &rrd2),
                                (&cdu1, &rdu1), (&cd1w, &rd1w), (&cd1x2, &rd1x2), (&cd1g, &rd1g), &shape, len);
                        }
                    }

                    let mut o4c = vec![0u8; cap];
                    let mut o4r = vec![0u8; cap];
                    let a = cc4(o4c.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct_fixture.as_ptr(), flags);
                    let b = rc4(o4r.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), ct_fixture.as_ptr(), flags);
                    let ctx = format!("HUF_compress4X_usingCTable shape={shape:?} len={len} flags={flags}");
                    e.eq(&ctx, a, b);
                    if !e.c_err(a) {
                        assert_bytes_eq(&ctx, &o4c[..a], &o4r[..b]);
                        if a > 0 {
                            round_trip_4x(&e, &src, &o4c[..a],
                                (&cdu4, &rdu4), (&cd4h, &rd4h), &shape, len);
                        }
                    }
                }

                // --- compress1X/4X_repeat over all HUF_repeat states ---
                for rep in [0i32, 1, 2] {
                    for name in ["1X", "4X"] {
                        let (cf, rf): (&Symbol<FnHufCompressRepeat>, &Symbol<FnHufCompressRepeat>) =
                            if name == "1X" { (&cr1, &rr1) } else { (&cr4, &rr4) };
                        let mut o1 = vec![0u8; cap];
                        let mut o2 = vec![0u8; cap];
                        let mut t1 = ct_fixture.clone();
                        let mut t2 = ct_fixture.clone();
                        let mut r1 = rep;
                        let mut r2 = rep;
                        let mut wc = vec![0u32; HUF_WORKSPACE_SIZE_U64 * 2 + 16];
                        let mut wr = vec![0u32; HUF_WORKSPACE_SIZE_U64 * 2 + 16];
                        let a = cf(o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), msv, HUF_TABLELOG_MAX, wc.as_mut_ptr() as *mut c_void, wc.len() * 4, t1.as_mut_ptr(), &mut r1, 0);
                        let b = rf(o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), msv, HUF_TABLELOG_MAX, wr.as_mut_ptr() as *mut c_void, wr.len() * 4, t2.as_mut_ptr(), &mut r2, 0);
                        let ctx = format!("HUF_compress{name}_repeat shape={shape:?} len={len} rep={rep}");
                        e.eq(&ctx, a, b);
                        assert_eq!(r1, r2, "{ctx}: repeat out");
                        if !e.c_err(a) && a > 0 {
                            assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
                            assert_eq!(t1, t2, "{ctx}: hufTable out");
                        }
                    }
                }
                let _ = cwc;
            }
        }
    }
}

/// 1X decode round-trip through both X1 and X2 decoders + DCtx wksp entry points.
#[allow(clippy::too_many_arguments)]
unsafe fn round_trip_1x(
    e: &EntErr,
    src: &[u8],
    comp: &[u8],
    header: &[u8],
    flags: c_int,
    sd: (&Symbol<FnHufSelectDecoder>, &Symbol<FnHufSelectDecoder>),
    rd1: (&Symbol<FnHufReadDTableWksp>, &Symbol<FnHufReadDTableWksp>),
    rd2: (&Symbol<FnHufReadDTableWksp>, &Symbol<FnHufReadDTableWksp>),
    du1: (&Symbol<FnHufDecompressUsingDTable>, &Symbol<FnHufDecompressUsingDTable>),
    d1w: (&Symbol<FnHufDecompressDCtxWksp>, &Symbol<FnHufDecompressDCtxWksp>),
    d1x2: (&Symbol<FnHufDecompressDCtxWksp>, &Symbol<FnHufDecompressDCtxWksp>),
    d1g: (&Symbol<FnHufDecompressDCtxWksp>, &Symbol<FnHufDecompressDCtxWksp>),
    shape: &Shape,
    len: usize,
) {
    // selectDecoder consistency
    assert_eq!((sd.0)(src.len(), comp.len()), (sd.1)(src.len(), comp.len()),
        "selectDecoder 1x shape={shape:?} len={len}");
    let _ = &rd2;

    for flavor in ["X1", "X1DCtx", "X2DCtx", "gen"] {
        let mut o1 = vec![0u8; src.len() + 16];
        let mut o2 = vec![0u8; src.len() + 16];
        let mut wc = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32 + 16];
        let mut wr = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32 + 16];
        let dsz = 1usize << (HUF_TABLELOG_MAX + 1);
        let (a, b) = match flavor {
            "X1DCtx" => {
                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                (
                    (d1w.0)(dc.as_mut_ptr(), o1.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags),
                    (d1w.1)(dr.as_mut_ptr(), o2.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags),
                )
            }
            "X2DCtx" => {
                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                (
                    (d1x2.0)(dc.as_mut_ptr(), o1.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags),
                    (d1x2.1)(dr.as_mut_ptr(), o2.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags),
                )
            }
            "gen" => {
                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                (
                    (d1g.0)(dc.as_mut_ptr(), o1.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags),
                    (d1g.1)(dr.as_mut_ptr(), o2.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags),
                )
            }
            _ => {
                // Build X1 DTable via readDTableX1_wksp from header, then usingDTable.
                let mut dc = vec![0u32; dsz];
                let mut dr = vec![0u32; dsz];
                dc[0] = (HUF_TABLELOG_MAX - 1) * 0x0100_0001;
                dr[0] = (HUF_TABLELOG_MAX - 1) * 0x0100_0001;
                let ra = (rd1.0)(dc.as_mut_ptr(), header.as_ptr() as *const c_void, header.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
                let rb = (rd1.1)(dr.as_mut_ptr(), header.as_ptr() as *const c_void, header.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
                e.eq(&format!("readDTableX1_wksp shape={shape:?} len={len}"), ra, rb);
                if e.c_err(ra) {
                    return;
                }
                assert_eq!(dc, dr, "readDTableX1 DTable bytes shape={shape:?} len={len}");
                (
                    (du1.0)(o1.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), dc.as_ptr(), flags),
                    (du1.1)(o2.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), dr.as_ptr(), flags),
                )
            }
        };
        let ctx = format!("HUF 1X decode {flavor} shape={shape:?} len={len} flags={flags}");
        e.eq(&ctx, a, b);
        if !e.c_err(a) {
            assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
            assert_eq!(&o1[..a], src, "{ctx}: roundtrip");
        }
    }
}

/// 4X decode round-trip via DCtx wksp + usingDTable.
#[allow(clippy::too_many_arguments)]
unsafe fn round_trip_4x(
    e: &EntErr,
    src: &[u8],
    comp: &[u8],
    du4: (&Symbol<FnHufDecompressUsingDTable>, &Symbol<FnHufDecompressUsingDTable>),
    d4h: (&Symbol<FnHufDecompressDCtxWksp>, &Symbol<FnHufDecompressDCtxWksp>),
    shape: &Shape,
    len: usize,
) {
    let flags = 0i32;
    let dsz = 1usize << (HUF_TABLELOG_MAX + 1);
    let mut o1 = vec![0u8; src.len() + 16];
    let mut o2 = vec![0u8; src.len() + 16];
    let mut dc = vec![0u32; dsz];
    let mut dr = vec![0u32; dsz];
    let mut wc = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32 + 16];
    let mut wr = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32 + 16];
    let a = (d4h.0)(dc.as_mut_ptr(), o1.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wc.as_mut_ptr() as *mut c_void, wc.len() * 4, flags);
    let b = (d4h.1)(dr.as_mut_ptr(), o2.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), wr.as_mut_ptr() as *mut c_void, wr.len() * 4, flags);
    let ctx = format!("HUF 4X hufOnly shape={shape:?} len={len}");
    e.eq(&ctx, a, b);
    if !e.c_err(a) {
        assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
        assert_eq!(&o1[..a], src, "{ctx}: roundtrip");
        let mut o3 = vec![0u8; src.len() + 16];
        let mut o4 = vec![0u8; src.len() + 16];
        let x = (du4.0)(o3.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), dc.as_ptr(), flags);
        let y = (du4.1)(o4.as_mut_ptr() as *mut c_void, src.len(), comp.as_ptr() as *const c_void, comp.len(), dr.as_ptr(), flags);
        e.eq(&format!("{ctx}/usingDTable"), x, y);
        if !e.c_err(x) {
            assert_bytes_eq(&format!("{ctx}/usingDTable"), &o3[..x], &o4[..y]);
        }
    }
}
