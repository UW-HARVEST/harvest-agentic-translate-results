//! Phase B: ENTROPY / HASH / POOL exports (CONFIGS.md rows 202-218).
//!
//! Every function is reached through the two loaded shared objects (C ground
//! truth first, Rust port second) — nothing in `translation/src` is called
//! directly. For each configuration the whole pipeline is replayed twice (once
//! per library) while every return code and every touched buffer is recorded;
//! the two recordings must be identical.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================ fn types =====

type FnUVoid = unsafe extern "C" fn() -> c_uint;
type FnSizeSize = unsafe extern "C" fn(usize) -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

// --- FSE helpers
type FnOptTL = unsafe extern "C" fn(c_uint, usize, c_uint) -> c_uint;
type FnOptTLI = unsafe extern "C" fn(c_uint, usize, c_uint, c_uint) -> c_uint;
type FnNCWB = unsafe extern "C" fn(c_uint, c_uint) -> usize;

// --- HIST
type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnHistCountWksp =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize, *mut c_void, usize) -> usize;
type FnHistSimple = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> c_uint;
type FnHistAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, usize);

// --- FSE pipeline
type FnFseNormalize =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, usize, c_uint, c_uint) -> usize;
type FnFseWriteNCount =
    unsafe extern "C" fn(*mut c_void, usize, *const i16, c_uint, c_uint) -> usize;
type FnFseBuildCTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnFseBuildCTableRle = unsafe extern "C" fn(*mut c_uint, u8) -> usize;
type FnFseCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const c_uint) -> usize;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnFseReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize, c_int) -> usize;
type FnFseBuildDTableWksp =
    unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnFseDecompressWkspBmi2 = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_uint,
    *mut c_void,
    usize,
    c_int,
) -> usize;

// --- HUF
type HufCElt = u64; // HUF_CElt == size_t
type HufDTable = u32; // HUF_DTable == U32

type FnHufOptimalTableLog = unsafe extern "C" fn(
    c_uint,
    usize,
    c_uint,
    *mut c_void,
    usize,
    *mut HufCElt,
    *const c_uint,
    c_int,
) -> c_uint;
type FnHufBuildCTableWksp =
    unsafe extern "C" fn(*mut HufCElt, *const c_uint, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnHufWriteCTableWksp = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const HufCElt,
    c_uint,
    c_uint,
    *mut c_void,
    usize,
) -> usize;
type FnHufCompressUsingCTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const HufCElt, c_int) -> usize;
type FnHufCompressRepeat = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_uint,
    c_uint,
    *mut c_void,
    usize,
    *mut HufCElt,
    *mut c_int,
    c_int,
) -> usize;
type FnHufReadDTableWksp =
    unsafe extern "C" fn(*mut HufDTable, *const c_void, usize, *mut c_void, usize, c_int) -> usize;
type FnHufDecompressUsingDTable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const HufDTable, c_int) -> usize;
type FnHufDecompressDCtxWksp = unsafe extern "C" fn(
    *mut HufDTable,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    c_int,
) -> usize;
type FnHufMinTableLog = unsafe extern "C" fn(c_uint) -> c_uint;
type FnHufCardinality = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
type FnHufValidateCTable = unsafe extern "C" fn(*const HufCElt, *const c_uint, c_uint) -> c_int;
type FnHufEstimate = unsafe extern "C" fn(*const HufCElt, *const c_uint, c_uint) -> usize;
type FnHufGetNbBits = unsafe extern "C" fn(*const HufCElt, c_uint) -> c_uint;
type FnHufReadCTable =
    unsafe extern "C" fn(*mut HufCElt, *mut c_uint, *const c_void, usize, *mut c_uint) -> usize;
type FnHufSelectDecoder = unsafe extern "C" fn(usize, usize) -> c_uint;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    usize,
) -> usize;
type FnHufReadStatsWksp = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    c_int,
) -> usize;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct HufCTableHeader {
    tableLog: u8,
    maxSymbolValue: u8,
    unused: [u8; 6],
}
type FnHufReadCTableHeader = unsafe extern "C" fn(*const HufCElt) -> HufCTableHeader;

// --- XXH
type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
type FnXxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnXxhCreate = unsafe extern "C" fn() -> *mut c_void;
type FnXxhFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnXxh32Reset = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnXxh64Reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnXxhUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnXxh32Digest = unsafe extern "C" fn(*const c_void) -> u32;
type FnXxh64Digest = unsafe extern "C" fn(*const c_void) -> u64;
type FnXxhCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnXxh32Canon = unsafe extern "C" fn(*mut c_void, u32);
type FnXxh64Canon = unsafe extern "C" fn(*mut c_void, u64);
type FnXxh32FromCanon = unsafe extern "C" fn(*const c_void) -> u32;
type FnXxh64FromCanon = unsafe extern "C" fn(*const c_void) -> u64;

// --- POOL
type ZstdAllocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type ZstdFreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
struct ZstdCustomMem {
    customAlloc: Option<ZstdAllocFn>,
    customFree: Option<ZstdFreeFn>,
    opaque: *mut c_void,
}

type PoolFn = unsafe extern "C" fn(*mut c_void);
type FnPoolCreate = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type FnPoolCreateAdv = unsafe extern "C" fn(usize, usize, ZstdCustomMem) -> *mut c_void;
type FnPoolVoid = unsafe extern "C" fn(*mut c_void);
type FnPoolResize = unsafe extern "C" fn(*mut c_void, usize) -> c_int;
type FnPoolSizeof = unsafe extern "C" fn(*const c_void) -> usize;
type FnPoolAdd = unsafe extern "C" fn(*mut c_void, Option<PoolFn>, *mut c_void);
type FnPoolTryAdd = unsafe extern "C" fn(*mut c_void, Option<PoolFn>, *mut c_void) -> c_int;

// ========================================================== constants =====

const FSE_MIN_TABLELOG: u32 = 5;
const FSE_MAX_TABLELOG: u32 = 12; // FSE_MAX_MEMORY_USAGE(14) - 2
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const HIST_WKSP_SIZE_U32: usize = 1024;
const HUF_TABLELOG_MAX: u32 = 12;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
/// HUF_WORKSPACE_SIZE == (8 << 10) + 512
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
/// HUF_CTABLE_WORKSPACE_SIZE == ((4 * 256) + 192) * 4
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192) * 4;
/// HUF_DECOMPRESS_WORKSPACE_SIZE == (2 << 10) + (1 << 9)
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
/// zstd initialises its huffman DTable with `ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x1000001`
const HUF_DTABLE_CAPACITY_LOG: u32 = 12;
/// Room for `1 + (1 << 13)` U32 so both X1 and X2 tables always fit.
const HUF_DTABLE_U32: usize = 1 + (1 << 13);
/// HUF_CTABLE_SIZE_ST(255) == 257, rounded up a bit.
const HUF_CTABLE_ST: usize = HUF_SYMBOLVALUE_MAX as usize + 3;

// FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue)
fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    1 + (1usize << (max_table_log.max(1) - 1)) + (max_symbol_value as usize + 1) * 2
}
// FSE_DTABLE_SIZE_U32(maxTableLog)
fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    1 + (1usize << max_table_log)
}
// FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)
fn fse_build_ctable_wksp_u32(max_symbol_value: u32, table_log: u32) -> usize {
    ((max_symbol_value as usize + 2) + (1usize << table_log)) / 2 + 2
}
// FSE_BUILD_DTABLE_WKSP_SIZE(maxTableLog, maxSymbolValue)
fn fse_build_dtable_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    2 * (max_symbol_value as usize + 1) + (1usize << max_table_log) + 8
}
fn fse_build_dtable_wksp_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    (fse_build_dtable_wksp_size(max_table_log, max_symbol_value) + 3) / 4
}
// FSE_DECOMPRESS_WKSP_SIZE_U32(maxTableLog, maxSymbolValue)
fn fse_decompress_wksp_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_dtable_size_u32(max_table_log)
        + 1
        + fse_build_dtable_wksp_u32(max_table_log, max_symbol_value)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}
// FSE_COMPRESSBOUND(size)
fn fse_compress_bound(size: usize) -> usize {
    512 + size + (size >> 7) + 4 + std::mem::size_of::<usize>()
}
// HUF_COMPRESSBOUND(size)
fn huf_compress_bound(size: usize) -> usize {
    129 + size + (size >> 8) + 8
}

// ======================================================== record/compare ===

/// Ordered log of everything one library produced for a configuration.
#[derive(Default)]
struct Rec {
    rets: Vec<(&'static str, i128)>,
    bufs: Vec<(&'static str, Vec<u8>)>,
}

impl Rec {
    fn ret(&mut self, name: &'static str, v: usize) {
        self.rets.push((name, v as i128));
    }
    fn reti(&mut self, name: &'static str, v: i64) {
        self.rets.push((name, v as i128));
    }
    fn retu(&mut self, name: &'static str, v: u64) {
        self.rets.push((name, v as i128));
    }
    fn buf(&mut self, name: &'static str, b: &[u8]) {
        self.bufs.push((name, b.to_vec()));
    }
    fn buf16(&mut self, name: &'static str, b: &[i16]) {
        self.bufs
            .push((name, b.iter().flat_map(|x| x.to_le_bytes()).collect()));
    }
    fn buf32(&mut self, name: &'static str, b: &[u32]) {
        self.bufs
            .push((name, b.iter().flat_map(|x| x.to_le_bytes()).collect()));
    }
    fn buf64(&mut self, name: &'static str, b: &[u64]) {
        self.bufs
            .push((name, b.iter().flat_map(|x| x.to_le_bytes()).collect()));
    }
}

fn cmp_rec(ctx: &str, c: &Rec, r: &Rec) {
    let cn: Vec<&str> = c.rets.iter().map(|x| x.0).collect();
    let rn: Vec<&str> = r.rets.iter().map(|x| x.0).collect();
    assert_eq!(cn, rn, "{ctx}: recorded return sequence differs (control flow)");
    for (&(n, cv), &(_, rv)) in c.rets.iter().zip(r.rets.iter()) {
        assert_eq!(
            cv, rv,
            "{ctx}: `{n}` differs: C={cv} (err {}) Rust={rv} (err {})",
            err_code(cv as usize),
            err_code(rv as usize)
        );
    }
    let cb: Vec<&str> = c.bufs.iter().map(|x| x.0).collect();
    let rb: Vec<&str> = r.bufs.iter().map(|x| x.0).collect();
    assert_eq!(cb, rb, "{ctx}: recorded buffer sequence differs");
    for ((n, cbuf), (_, rbuf)) in c.bufs.iter().zip(r.bufs.iter()) {
        assert_bytes_eq(&format!("{ctx}: buffer `{n}`"), cbuf, rbuf);
    }
}

fn cstr_of(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// Restrict every byte to `0..=max_symbol_value` (needed for the "unsafe"
/// histogram variants and to actually exercise small alphabets).
fn mask_alphabet(src: &[u8], max_symbol_value: u32) -> Vec<u8> {
    if max_symbol_value >= 255 {
        return src.to_vec();
    }
    let m = (max_symbol_value + 1) as u16;
    src.iter().map(|&b| (b as u16 % m) as u8).collect()
}

fn aligned_wksp(bytes: usize) -> Vec<u64> {
    vec![0u64; (bytes + 7) / 8]
}

// ============================================================== row 202 ====

/// Row 202: FSE version / bound / error helpers and the table-log heuristics.
#[test]
fn fse_helpers() {
    let (cver, rver) = unsafe { pair::<FnUVoid>("FSE_versionNumber") };
    let (ccb, rcb) = unsafe { pair::<FnSizeSize>("FSE_compressBound") };
    let (cie, rie) = unsafe { pair::<FnIsError>("FSE_isError") };
    let (cen, ren) = unsafe { pair::<FnErrName>("FSE_getErrorName") };
    let (cot, rot) = unsafe { pair::<FnOptTL>("FSE_optimalTableLog") };
    let (coti, roti) = unsafe { pair::<FnOptTLI>("FSE_optimalTableLog_internal") };
    let (cnb, rnb) = unsafe { pair::<FnNCWB>("FSE_NCountWriteBound") };

    assert_eq!(unsafe { cver() }, unsafe { rver() }, "FSE_versionNumber");

    // ---- FSE_compressBound
    let mut rng = Rng::new(0x0202_0001);
    let mut sizes: Vec<usize> = vec![0, 1, 2, 3, 100, 127, 128, 4096, 131072, 1 << 20];
    for _ in 0..2000 {
        sizes.push(rng.below(1 << 22));
    }
    for &s in &sizes {
        assert_eq!(
            unsafe { ccb(s) },
            unsafe { rcb(s) },
            "FSE_compressBound({s})"
        );
    }

    // ---- FSE_isError / FSE_getErrorName over the whole error range + noise
    let mut codes: Vec<usize> = vec![0, 1, 2, 100, usize::MAX, usize::MAX - 1];
    for e in 0..=200usize {
        codes.push(0usize.wrapping_sub(e));
    }
    for _ in 0..2000 {
        codes.push(rng.next_u64() as usize);
    }
    for &code in &codes {
        assert_eq!(
            unsafe { cie(code) },
            unsafe { rie(code) },
            "FSE_isError({code:#x})"
        );
        let cs = cstr_of(unsafe { cen(code) });
        let rs = cstr_of(unsafe { ren(code) });
        assert_eq!(cs, rs, "FSE_getErrorName({code:#x})");
    }

    // ---- FSE_optimalTableLog / _internal / FSE_NCountWriteBound
    let max_table_logs: [u32; 4] = [5, 9, 11, 12];
    let src_sizes: [usize; 4] = [1, 100, 4096, 131072];
    let max_symbol_values: [u32; 3] = [1, 15, 255];
    let minuses: [u32; 3] = [0, 1, 2];
    for &mtl in &max_table_logs {
        for &ss in &src_sizes {
            for &msv in &max_symbol_values {
                assert_eq!(
                    unsafe { cot(mtl, ss, msv) },
                    unsafe { rot(mtl, ss, msv) },
                    "FSE_optimalTableLog({mtl},{ss},{msv})"
                );
                for &minus in &minuses {
                    assert_eq!(
                        unsafe { coti(mtl, ss, msv, minus) },
                        unsafe { roti(mtl, ss, msv, minus) },
                        "FSE_optimalTableLog_internal({mtl},{ss},{msv},{minus})"
                    );
                }
            }
            assert_eq!(
                unsafe { cnb(255, mtl) },
                unsafe { rnb(255, mtl) },
                "FSE_NCountWriteBound(255,{mtl})"
            );
        }
        for &msv in &max_symbol_values {
            assert_eq!(
                unsafe { cnb(msv, mtl) },
                unsafe { rnb(msv, mtl) },
                "FSE_NCountWriteBound({msv},{mtl})"
            );
        }
    }
    // randomized sweep over the full (maxSymbolValue, tableLog) grid
    for msv in 0..=255u32 {
        for tl in FSE_MIN_TABLELOG..=FSE_MAX_TABLELOG {
            assert_eq!(
                unsafe { cnb(msv, tl) },
                unsafe { rnb(msv, tl) },
                "FSE_NCountWriteBound({msv},{tl})"
            );
        }
    }
    for _ in 0..4000 {
        let mtl = rng.range(0, 16) as u32;
        let ss = rng.below(1 << 20);
        let msv = rng.below(256) as u32;
        let minus = rng.below(4) as u32;
        assert_eq!(
            unsafe { cot(mtl, ss, msv) },
            unsafe { rot(mtl, ss, msv) },
            "FSE_optimalTableLog({mtl},{ss},{msv})"
        );
        assert_eq!(
            unsafe { coti(mtl, ss, msv, minus) },
            unsafe { roti(mtl, ss, msv, minus) },
            "FSE_optimalTableLog_internal({mtl},{ss},{msv},{minus})"
        );
    }
}

// ============================================================== row 203 ====

/// Row 203: the whole HIST_* family. Compares the return value, the complete
/// 256-entry `count[]` array and the value written back through
/// `maxSymbolValuePtr`.
#[test]
fn hist_functions() {
    let (c_count, r_count) = unsafe { pair::<FnHistCount>("HIST_count") };
    let (c_fast, r_fast) = unsafe { pair::<FnHistCount>("HIST_countFast") };
    let (c_cw, r_cw) = unsafe { pair::<FnHistCountWksp>("HIST_count_wksp") };
    let (c_fw, r_fw) = unsafe { pair::<FnHistCountWksp>("HIST_countFast_wksp") };
    let (c_simple, r_simple) = unsafe { pair::<FnHistSimple>("HIST_count_simple") };
    let (c_add, r_add) = unsafe { pair::<FnHistAdd>("HIST_add") };
    let (c_ie, r_ie) = unsafe { pair::<FnIsError>("HIST_isError") };

    // HIST_isError over the error range + noise
    let mut rng = Rng::new(0x0203_0001);
    for e in 0..=200usize {
        let code = 0usize.wrapping_sub(e);
        assert_eq!(
            unsafe { c_ie(code) },
            unsafe { r_ie(code) },
            "HIST_isError({code:#x})"
        );
    }
    for _ in 0..1000 {
        let code = rng.next_u64() as usize;
        assert_eq!(
            unsafe { c_ie(code) },
            unsafe { r_ie(code) },
            "HIST_isError({code:#x})"
        );
    }

    // The required grid, plus many randomized bodies per shape.
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    inputs.push(("empty".into(), Vec::new()));
    inputs.push(("one-byte".into(), vec![0xA7]));
    inputs.push(("4096-rle".into(), vec![0x5C; 4096]));
    for i in 0..8 {
        inputs.push((
            format!("4096-random#{i}"),
            gen(Shape::Random, 4096, &mut rng),
        ));
        inputs.push((format!("4096-text#{i}"), gen(Shape::Text, 4096, &mut rng)));
    }
    // extra: sizes around the internal 1500-byte heuristic threshold
    for &n in &[1usize, 15, 16, 17, 1499, 1500, 1501, 4095, 4097, 131072] {
        inputs.push((format!("{n}-random"), gen(Shape::Random, n, &mut rng)));
        inputs.push((format!("{n}-sparse"), gen(Shape::Sparse, n, &mut rng)));
    }

    for &msv in &[1u32, 15, 255] {
        for (name, raw) in &inputs {
            let masked = mask_alphabet(raw, msv);
            let ctx = format!("hist msv={msv} src={name}");

            // ---- safe variants get the raw source (may legitimately error out)
            for variant in 0..2 {
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let mut count = [0u32; 256];
                    let mut msv_io: c_uint = msv;
                    let mut wksp = vec![0u32; HIST_WKSP_SIZE_U32];
                    let ret = unsafe {
                        match (variant, which) {
                            (0, 0) => c_count(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                            ),
                            (0, _) => r_count(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                            ),
                            (_, 0) => c_cw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                                wksp.as_mut_ptr() as *mut c_void,
                                wksp.len() * 4,
                            ),
                            (_, _) => r_cw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                                wksp.as_mut_ptr() as *mut c_void,
                                wksp.len() * 4,
                            ),
                        }
                    };
                    rec.ret("ret", ret);
                    rec.retu("maxSymbolValuePtr", msv_io as u64);
                    rec.buf32("count", &count);
                    rec.buf32("wksp", &wksp);
                }
                let vname = if variant == 0 { "HIST_count" } else { "HIST_count_wksp" };
                cmp_rec(&format!("{ctx} {vname}"), &recs[0], &recs[1]);
            }

            // ---- workspace error paths of the _wksp variants
            for &wsz in &[0usize, 4, HIST_WKSP_SIZE_U32 * 4 - 4] {
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let mut count = [0u32; 256];
                    let mut msv_io: c_uint = msv;
                    let mut wksp = vec![0u32; HIST_WKSP_SIZE_U32];
                    let ret = unsafe {
                        if which == 0 {
                            c_cw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                                wksp.as_mut_ptr() as *mut c_void,
                                wsz,
                            )
                        } else {
                            r_cw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                raw.as_ptr() as *const c_void,
                                raw.len(),
                                wksp.as_mut_ptr() as *mut c_void,
                                wsz,
                            )
                        }
                    };
                    rec.ret("ret", ret);
                    rec.retu("maxSymbolValuePtr", msv_io as u64);
                    rec.buf32("count", &count);
                }
                cmp_rec(
                    &format!("{ctx} HIST_count_wksp wkspSize={wsz}"),
                    &recs[0],
                    &recs[1],
                );
            }

            // ---- unsafe variants: source must be inside the alphabet
            for variant in 0..3 {
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let mut count = [0u32; 256];
                    let mut msv_io: c_uint = msv;
                    let mut wksp = vec![0u32; HIST_WKSP_SIZE_U32];
                    let sp = masked.as_ptr() as *const c_void;
                    let sl = masked.len();
                    let ret: usize = unsafe {
                        match (variant, which) {
                            (0, 0) => c_simple(count.as_mut_ptr(), &mut msv_io, sp, sl) as usize,
                            (0, _) => r_simple(count.as_mut_ptr(), &mut msv_io, sp, sl) as usize,
                            (1, 0) => c_fast(count.as_mut_ptr(), &mut msv_io, sp, sl),
                            (1, _) => r_fast(count.as_mut_ptr(), &mut msv_io, sp, sl),
                            (_, 0) => c_fw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                sp,
                                sl,
                                wksp.as_mut_ptr() as *mut c_void,
                                wksp.len() * 4,
                            ),
                            (_, _) => r_fw(
                                count.as_mut_ptr(),
                                &mut msv_io,
                                sp,
                                sl,
                                wksp.as_mut_ptr() as *mut c_void,
                                wksp.len() * 4,
                            ),
                        }
                    };
                    rec.ret("ret", ret);
                    rec.retu("maxSymbolValuePtr", msv_io as u64);
                    rec.buf32("count", &count);
                    rec.buf32("wksp", &wksp);
                }
                let vname = match variant {
                    0 => "HIST_count_simple",
                    1 => "HIST_countFast",
                    _ => "HIST_countFast_wksp",
                };
                cmp_rec(&format!("{ctx} {vname}"), &recs[0], &recs[1]);
            }

            // ---- HIST_add accumulates into a non-zeroed table
            let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
            for which in 0..2 {
                let rec = &mut recs[which];
                // deterministic non-zero starting table
                let mut count = [0u32; 256];
                for (i, c) in count.iter_mut().enumerate() {
                    *c = (i as u32) * 3 + 1;
                }
                let f = if which == 0 { &c_add } else { &r_add };
                unsafe { f(count.as_mut_ptr(), raw.as_ptr() as *const c_void, raw.len()) };
                // add it twice to make sure accumulation is compared
                unsafe { f(count.as_mut_ptr(), raw.as_ptr() as *const c_void, raw.len()) };
                rec.buf32("count", &count);
            }
            cmp_rec(&format!("{ctx} HIST_add"), &recs[0], &recs[1]);
        }
    }
}

// ========================================================= rows 204/205 ====

/// Rows 204 + 205: the complete FSE compress/decompress pipeline, every
/// intermediate buffer and return code compared, including the degenerate
/// cases (`tableLog` 13, `FSE_buildCTable_rle`, `useLowProbCount` 0/1,
/// `FSE_readNCount_bmi2` with bmi2 0/1).
#[test]
fn fse_roundtrip() {
    let (c_hist, r_hist) = unsafe { pair::<FnHistCount>("HIST_count") };
    let (c_norm, r_norm) = unsafe { pair::<FnFseNormalize>("FSE_normalizeCount") };
    let (c_nwb, r_nwb) = unsafe { pair::<FnNCWB>("FSE_NCountWriteBound") };
    let (c_wnc, r_wnc) = unsafe { pair::<FnFseWriteNCount>("FSE_writeNCount") };
    let (c_bct, r_bct) = unsafe { pair::<FnFseBuildCTableWksp>("FSE_buildCTable_wksp") };
    let (c_rle, r_rle) = unsafe { pair::<FnFseBuildCTableRle>("FSE_buildCTable_rle") };
    let (c_cmp, r_cmp) = unsafe { pair::<FnFseCompressUsingCTable>("FSE_compress_usingCTable") };
    let (c_rnc, r_rnc) = unsafe { pair::<FnFseReadNCount>("FSE_readNCount") };
    let (c_rncb, r_rncb) = unsafe { pair::<FnFseReadNCountBmi2>("FSE_readNCount_bmi2") };
    let (c_bdt, r_bdt) = unsafe { pair::<FnFseBuildDTableWksp>("FSE_buildDTable_wksp") };
    let (c_dec, r_dec) = unsafe { pair::<FnFseDecompressWkspBmi2>("FSE_decompress_wksp_bmi2") };

    let mut rng = Rng::new(0x0204_0001);

    // -------- sources: text / random / skewed (99% one symbol), 4096 B
    let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..3 {
        sources.push((format!("text#{i}"), gen(Shape::Text, 4096, &mut rng)));
        sources.push((format!("random#{i}"), gen(Shape::Random, 4096, &mut rng)));
        let mut skewed = vec![7u8; 4096];
        for _ in 0..41 {
            let p = rng.below(4096);
            skewed[p] = rng.byte();
        }
        sources.push((format!("skewed#{i}"), skewed));
    }

    for &table_log in &[5u32, 6, 9, 11, 12, 13] {
        for &msv in &[1u32, 15, 100, 255] {
            for (sname, raw) in &sources {
                let src = mask_alphabet(raw, msv);
                for &low_prob in &[0u32, 1] {
                    for &bmi2 in &[0i32, 1] {
                        let ctx = format!(
                            "fse tableLog={table_log} maxSV={msv} src={sname} lowProb={low_prob} bmi2={bmi2}"
                        );
                        let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                        for which in 0..2 {
                            let rec = &mut recs[which];
                            let is_c = which == 0;

                            // 1. histogram
                            let mut count = [0u32; 256];
                            let mut msv_io: c_uint = msv;
                            let hret = unsafe {
                                let f = if is_c { &c_hist } else { &r_hist };
                                f(
                                    count.as_mut_ptr(),
                                    &mut msv_io,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                )
                            };
                            rec.ret("HIST_count", hret);
                            rec.retu("HIST_count.maxSV", msv_io as u64);
                            rec.buf32("count", &count);
                            if is_error(hret) {
                                continue;
                            }
                            let actual_msv = msv_io;

                            // 2. normalize
                            let mut norm = [0i16; 256];
                            let nret = unsafe {
                                let f = if is_c { &c_norm } else { &r_norm };
                                f(
                                    norm.as_mut_ptr(),
                                    table_log,
                                    count.as_ptr(),
                                    src.len(),
                                    actual_msv,
                                    low_prob,
                                )
                            };
                            rec.ret("FSE_normalizeCount", nret);
                            rec.buf16("norm", &norm);
                            if is_error(nret) || nret == 0 {
                                // error, or the "single symbol" (RLE) short-circuit
                                continue;
                            }
                            let tl = nret as u32;

                            // 3. NCountWriteBound + writeNCount (exact and too small)
                            let bound = unsafe {
                                let f = if is_c { &c_nwb } else { &r_nwb };
                                f(actual_msv, tl)
                            };
                            rec.ret("FSE_NCountWriteBound", bound);
                            let mut ncount = vec![0u8; bound + 16];
                            let wret = unsafe {
                                let f = if is_c { &c_wnc } else { &r_wnc };
                                f(
                                    ncount.as_mut_ptr() as *mut c_void,
                                    ncount.len(),
                                    norm.as_ptr(),
                                    actual_msv,
                                    tl,
                                )
                            };
                            rec.ret("FSE_writeNCount", wret);
                            rec.buf("ncount", &ncount);
                            if is_error(wret) {
                                continue;
                            }
                            // too-small destination must fail identically
                            let mut tiny = vec![0u8; 1];
                            let tret = unsafe {
                                let f = if is_c { &c_wnc } else { &r_wnc };
                                f(
                                    tiny.as_mut_ptr() as *mut c_void,
                                    tiny.len(),
                                    norm.as_ptr(),
                                    actual_msv,
                                    tl,
                                )
                            };
                            rec.ret("FSE_writeNCount(tiny)", tret);

                            // 4. buildCTable_wksp
                            let ct_u32 = fse_ctable_size_u32(tl, actual_msv);
                            let mut ctable = vec![0u32; ct_u32];
                            let ct_wksp_u32 = fse_build_ctable_wksp_u32(actual_msv, tl);
                            // The C implementation writes a "spread" table of
                            // tableSize+8 bytes right after `tableSymbol`, which
                            // FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32() under-counts by
                            // 2 bytes whenever (maxSymbolValue + 2 + tableSize) is
                            // odd. Allocate slack but still pass the macro-computed
                            // size so the internal size check behaves identically.
                            let mut ct_wksp = vec![0u32; ct_wksp_u32 + 16];
                            let bret = unsafe {
                                let f = if is_c { &c_bct } else { &r_bct };
                                f(
                                    ctable.as_mut_ptr(),
                                    norm.as_ptr(),
                                    actual_msv,
                                    tl,
                                    ct_wksp.as_mut_ptr() as *mut c_void,
                                    ct_wksp_u32 * 4,
                                )
                            };
                            rec.ret("FSE_buildCTable_wksp", bret);
                            rec.buf32("ctable", &ctable);
                            if is_error(bret) {
                                continue;
                            }
                            // workspace-too-small path
                            let sret = unsafe {
                                let f = if is_c { &c_bct } else { &r_bct };
                                f(
                                    ctable.as_mut_ptr(),
                                    norm.as_ptr(),
                                    actual_msv,
                                    tl,
                                    ct_wksp.as_mut_ptr() as *mut c_void,
                                    0,
                                )
                            };
                            rec.ret("FSE_buildCTable_wksp(wksp=0)", sret);

                            // 5. compress_usingCTable
                            let mut cbuf = vec![0u8; fse_compress_bound(src.len())];
                            let cret = unsafe {
                                let f = if is_c { &c_cmp } else { &r_cmp };
                                f(
                                    cbuf.as_mut_ptr() as *mut c_void,
                                    cbuf.len(),
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    ctable.as_ptr(),
                                )
                            };
                            rec.ret("FSE_compress_usingCTable", cret);
                            rec.buf("cbuf", &cbuf);
                            if is_error(cret) {
                                continue;
                            }
                            // small destination
                            let mut small = vec![0u8; 8];
                            let scret = unsafe {
                                let f = if is_c { &c_cmp } else { &r_cmp };
                                f(
                                    small.as_mut_ptr() as *mut c_void,
                                    small.len(),
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    ctable.as_ptr(),
                                )
                            };
                            rec.ret("FSE_compress_usingCTable(small)", scret);

                            // 6. readNCount / readNCount_bmi2
                            let mut norm2 = [0i16; 256];
                            let mut msv2: c_uint = 255;
                            let mut tl2: c_uint = 0;
                            let rret = unsafe {
                                let f = if is_c { &c_rnc } else { &r_rnc };
                                f(
                                    norm2.as_mut_ptr(),
                                    &mut msv2,
                                    &mut tl2,
                                    ncount.as_ptr() as *const c_void,
                                    ncount.len(),
                                )
                            };
                            rec.ret("FSE_readNCount", rret);
                            rec.retu("FSE_readNCount.maxSV", msv2 as u64);
                            rec.retu("FSE_readNCount.tableLog", tl2 as u64);
                            rec.buf16("norm2", &norm2);

                            let mut norm3 = [0i16; 256];
                            let mut msv3: c_uint = 255;
                            let mut tl3: c_uint = 0;
                            let rbret = unsafe {
                                let f = if is_c { &c_rncb } else { &r_rncb };
                                f(
                                    norm3.as_mut_ptr(),
                                    &mut msv3,
                                    &mut tl3,
                                    ncount.as_ptr() as *const c_void,
                                    ncount.len(),
                                    bmi2,
                                )
                            };
                            rec.ret("FSE_readNCount_bmi2", rbret);
                            rec.retu("FSE_readNCount_bmi2.maxSV", msv3 as u64);
                            rec.retu("FSE_readNCount_bmi2.tableLog", tl3 as u64);
                            rec.buf16("norm3", &norm3);
                            // maxSymbolValue too small
                            let mut norm4 = [0i16; 256];
                            let mut msv4: c_uint = 0;
                            let mut tl4: c_uint = 0;
                            let r4 = unsafe {
                                let f = if is_c { &c_rnc } else { &r_rnc };
                                f(
                                    norm4.as_mut_ptr(),
                                    &mut msv4,
                                    &mut tl4,
                                    ncount.as_ptr() as *const c_void,
                                    ncount.len(),
                                )
                            };
                            rec.ret("FSE_readNCount(msv=0)", r4);
                            if is_error(rret) {
                                continue;
                            }

                            // 7. buildDTable_wksp
                            let mut dtable = vec![0u32; fse_dtable_size_u32(FSE_MAX_TABLELOG)];
                            let mut dt_wksp =
                                vec![0u32; fse_build_dtable_wksp_u32(FSE_MAX_TABLELOG, 255)];
                            let dret = unsafe {
                                let f = if is_c { &c_bdt } else { &r_bdt };
                                f(
                                    dtable.as_mut_ptr(),
                                    norm2.as_ptr(),
                                    msv2,
                                    tl2,
                                    dt_wksp.as_mut_ptr() as *mut c_void,
                                    dt_wksp.len() * 4,
                                )
                            };
                            rec.ret("FSE_buildDTable_wksp", dret);
                            rec.buf32("dtable", &dtable);
                            let dret_small = unsafe {
                                let f = if is_c { &c_bdt } else { &r_bdt };
                                f(
                                    dtable.as_mut_ptr(),
                                    norm2.as_ptr(),
                                    msv2,
                                    tl2,
                                    dt_wksp.as_mut_ptr() as *mut c_void,
                                    8,
                                )
                            };
                            rec.ret("FSE_buildDTable_wksp(small)", dret_small);

                            // 8. decompress_wksp_bmi2 on header ++ payload
                            if cret == 0 {
                                continue;
                            }
                            let mut frame = ncount[..wret].to_vec();
                            frame.extend_from_slice(&cbuf[..cret]);
                            let mut out = vec![0u8; src.len()];
                            let mut dwksp = aligned_wksp(
                                fse_decompress_wksp_u32(FSE_MAX_TABLELOG, 255) * 4 + 64,
                            );
                            let xret = unsafe {
                                let f = if is_c { &c_dec } else { &r_dec };
                                f(
                                    out.as_mut_ptr() as *mut c_void,
                                    out.len(),
                                    frame.as_ptr() as *const c_void,
                                    frame.len(),
                                    FSE_MAX_TABLELOG,
                                    dwksp.as_mut_ptr() as *mut c_void,
                                    dwksp.len() * 8,
                                    bmi2,
                                )
                            };
                            rec.ret("FSE_decompress_wksp_bmi2", xret);
                            rec.buf("out", &out);
                            if !is_error(xret) {
                                assert_eq!(xret, src.len(), "{ctx}: regenerated size");
                                assert_bytes_eq(&format!("{ctx}: roundtrip"), &src, &out[..xret]);
                            }
                            // maxLog too small + workspace too small error paths
                            let yret = unsafe {
                                let f = if is_c { &c_dec } else { &r_dec };
                                f(
                                    out.as_mut_ptr() as *mut c_void,
                                    out.len(),
                                    frame.as_ptr() as *const c_void,
                                    frame.len(),
                                    FSE_MIN_TABLELOG - 1,
                                    dwksp.as_mut_ptr() as *mut c_void,
                                    dwksp.len() * 8,
                                    bmi2,
                                )
                            };
                            rec.ret("FSE_decompress_wksp_bmi2(maxLog=4)", yret);
                            let zret = unsafe {
                                let f = if is_c { &c_dec } else { &r_dec };
                                f(
                                    out.as_mut_ptr() as *mut c_void,
                                    out.len(),
                                    frame.as_ptr() as *const c_void,
                                    frame.len(),
                                    FSE_MAX_TABLELOG,
                                    dwksp.as_mut_ptr() as *mut c_void,
                                    16,
                                    bmi2,
                                )
                            };
                            rec.ret("FSE_decompress_wksp_bmi2(wksp=16)", zret);
                        }
                        cmp_rec(&ctx, &recs[0], &recs[1]);
                    }
                }
            }
        }
    }

    // -------- row 205: FSE_buildCTable_rle + FSE_compress_usingCTable
    for &sym in &[0u8, 127, 255] {
        for &len in &[1usize, 2, 17, 4096] {
            let src = vec![sym; len];
            let ctx = format!("fse rle sym={sym} len={len}");
            let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
            for which in 0..2 {
                let rec = &mut recs[which];
                let is_c = which == 0;
                let mut ctable = vec![0u32; fse_ctable_size_u32(6, 255)];
                let bret = unsafe {
                    let f = if is_c { &c_rle } else { &r_rle };
                    f(ctable.as_mut_ptr(), sym)
                };
                rec.ret("FSE_buildCTable_rle", bret);
                rec.buf32("ctable", &ctable);
                let mut cbuf = vec![0u8; fse_compress_bound(len)];
                let cret = unsafe {
                    let f = if is_c { &c_cmp } else { &r_cmp };
                    f(
                        cbuf.as_mut_ptr() as *mut c_void,
                        cbuf.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        ctable.as_ptr(),
                    )
                };
                rec.ret("FSE_compress_usingCTable", cret);
                rec.buf("cbuf", &cbuf);
            }
            cmp_rec(&ctx, &recs[0], &recs[1]);
        }
    }
}

// ===================================================== HUF shared helper ===

struct HufTables {
    count: [u32; 256],
    max_symbol_value: c_uint,
    table_log: u32,
    ctable: Vec<HufCElt>,
    /// full 512 B destination buffer (zero-filled beyond `header_len`)
    header: Vec<u8>,
    /// number of bytes `HUF_writeCTable_wksp()` actually wrote
    header_len: usize,
}

/// Build the huffman CTable + serialized header for `src` in ONE library
/// (`is_c` selects which). Returns `None` when the C library could not build a
/// table (the caller compares the recorded return codes anyway).
fn huf_build(
    is_c: bool,
    src: &[u8],
    huff_log: u32,
    flags: c_int,
    rec: &mut Rec,
) -> Option<HufTables> {
    let (c_hist, r_hist) = unsafe { pair::<FnHistCount>("HIST_count") };
    let (c_card, r_card) = unsafe { pair::<FnHufCardinality>("HUF_cardinality") };
    let (c_min, r_min) = unsafe { pair::<FnHufMinTableLog>("HUF_minTableLog") };
    let (c_opt, r_opt) = unsafe { pair::<FnHufOptimalTableLog>("HUF_optimalTableLog") };
    let (c_bct, r_bct) = unsafe { pair::<FnHufBuildCTableWksp>("HUF_buildCTable_wksp") };
    let (c_wct, r_wct) = unsafe { pair::<FnHufWriteCTableWksp>("HUF_writeCTable_wksp") };

    let mut count = [0u32; 256];
    let mut msv: c_uint = 255;
    let hret = unsafe {
        let f = if is_c { &c_hist } else { &r_hist };
        f(
            count.as_mut_ptr(),
            &mut msv,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    rec.ret("HIST_count", hret);
    rec.retu("HIST_count.maxSV", msv as u64);
    rec.buf32("count", &count);
    if is_error(hret) {
        return None;
    }

    let card = unsafe {
        let f = if is_c { &c_card } else { &r_card };
        f(count.as_ptr(), msv)
    };
    rec.retu("HUF_cardinality", card as u64);
    let min_log = unsafe {
        let f = if is_c { &c_min } else { &r_min };
        f(card.max(1))
    };
    rec.retu("HUF_minTableLog", min_log as u64);

    // never ask for fewer bits than the alphabet needs (the C tree builder
    // has no defence against an impossible target depth)
    let eff_log = huff_log.max(min_log).min(HUF_TABLELOG_MAX);
    rec.retu("effectiveHuffLog", eff_log as u64);

    let mut wksp = aligned_wksp(HUF_WORKSPACE_SIZE);
    let mut scratch = vec![0u64; HUF_CTABLE_ST];
    let opt = unsafe {
        let f = if is_c { &c_opt } else { &r_opt };
        f(
            eff_log,
            src.len(),
            msv,
            wksp.as_mut_ptr() as *mut c_void,
            wksp.len() * 8,
            scratch.as_mut_ptr(),
            count.as_ptr(),
            flags,
        )
    };
    rec.retu("HUF_optimalTableLog", opt as u64);
    rec.buf64("optimalTableLog.scratch", &scratch);

    let opt = opt.max(min_log).min(HUF_TABLELOG_MAX);
    let mut ctable = vec![0u64; HUF_CTABLE_ST];
    let mut ct_wksp = aligned_wksp(HUF_CTABLE_WORKSPACE_SIZE);
    let bret = unsafe {
        let f = if is_c { &c_bct } else { &r_bct };
        f(
            ctable.as_mut_ptr(),
            count.as_ptr(),
            msv,
            opt,
            ct_wksp.as_mut_ptr() as *mut c_void,
            ct_wksp.len() * 8,
        )
    };
    rec.ret("HUF_buildCTable_wksp", bret);
    rec.buf64("ctable", &ctable);
    // workspace too small
    let bsmall = unsafe {
        let f = if is_c { &c_bct } else { &r_bct };
        f(
            ctable.as_mut_ptr(),
            count.as_ptr(),
            msv,
            opt,
            ct_wksp.as_mut_ptr() as *mut c_void,
            16,
        )
    };
    rec.ret("HUF_buildCTable_wksp(wksp=16)", bsmall);
    if is_error(bret) {
        return None;
    }
    let actual_log = bret as u32;

    let mut header = vec![0u8; 512];
    let mut w_wksp = aligned_wksp(HUF_WORKSPACE_SIZE);
    let wret = unsafe {
        let f = if is_c { &c_wct } else { &r_wct };
        f(
            header.as_mut_ptr() as *mut c_void,
            header.len(),
            ctable.as_ptr(),
            msv,
            actual_log,
            w_wksp.as_mut_ptr() as *mut c_void,
            w_wksp.len() * 8,
        )
    };
    rec.ret("HUF_writeCTable_wksp", wret);
    rec.buf("header", &header);
    // destination too small + workspace too small
    let mut tiny = vec![0u8; 1];
    let tret = unsafe {
        let f = if is_c { &c_wct } else { &r_wct };
        f(
            tiny.as_mut_ptr() as *mut c_void,
            0,
            ctable.as_ptr(),
            msv,
            actual_log,
            w_wksp.as_mut_ptr() as *mut c_void,
            w_wksp.len() * 8,
        )
    };
    rec.ret("HUF_writeCTable_wksp(dst=0)", tret);
    let wsret = unsafe {
        let f = if is_c { &c_wct } else { &r_wct };
        f(
            tiny.as_mut_ptr() as *mut c_void,
            tiny.len(),
            ctable.as_ptr(),
            msv,
            actual_log,
            w_wksp.as_mut_ptr() as *mut c_void,
            8,
        )
    };
    rec.ret("HUF_writeCTable_wksp(wksp=8)", wsret);
    if is_error(wret) {
        return None;
    }
    // `header` stays at its full (zero-filled) 512 B length so that callers may
    // legitimately pass a `srcSize` larger than `header_len` without reading
    // uninitialised memory; only the first `header_len` bytes were written.
    Some(HufTables {
        count,
        max_symbol_value: msv,
        table_log: actual_log,
        ctable,
        header,
        header_len: wret,
    })
}

// ==================================================== rows 206/207/209 ====

/// Rows 206, 207 and 209: the HUF 4-stream and 1-stream round trips plus the
/// full `flags` sweep, through both `*_usingDTable` and the `*_DCtx_wksp`
/// entry points.
#[test]
fn huf_roundtrip() {
    let (c_c4, r_c4) = unsafe { pair::<FnHufCompressUsingCTable>("HUF_compress4X_usingCTable") };
    let (c_c1, r_c1) = unsafe { pair::<FnHufCompressUsingCTable>("HUF_compress1X_usingCTable") };
    let (c_rd1, r_rd1) = unsafe { pair::<FnHufReadDTableWksp>("HUF_readDTableX1_wksp") };
    let (c_rd2, r_rd2) = unsafe { pair::<FnHufReadDTableWksp>("HUF_readDTableX2_wksp") };
    let (c_d4, r_d4) =
        unsafe { pair::<FnHufDecompressUsingDTable>("HUF_decompress4X_usingDTable") };
    let (c_d1, r_d1) =
        unsafe { pair::<FnHufDecompressUsingDTable>("HUF_decompress1X_usingDTable") };
    let (c_x11, r_x11) = unsafe { pair::<FnHufDecompressDCtxWksp>("HUF_decompress1X1_DCtx_wksp") };
    let (c_x12, r_x12) = unsafe { pair::<FnHufDecompressDCtxWksp>("HUF_decompress1X2_DCtx_wksp") };
    let (c_x1, r_x1) = unsafe { pair::<FnHufDecompressDCtxWksp>("HUF_decompress1X_DCtx_wksp") };
    let (c_ho, r_ho) = unsafe { pair::<FnHufDecompressDCtxWksp>("HUF_decompress4X_hufOnly_wksp") };

    let mut rng = Rng::new(0x0206_0001);
    let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
    sources.push(("text-4096".into(), gen(Shape::Text, 4096, &mut rng)));
    sources.push(("random-4096".into(), gen(Shape::Random, 4096, &mut rng)));
    sources.push(("text-131072".into(), gen(Shape::Text, 131072, &mut rng)));
    sources.push(("random-131072".into(), gen(Shape::Random, 131072, &mut rng)));
    // small alphabet so that huffLog=5 is actually reachable
    sources.push((
        "alpha16-4096".into(),
        mask_alphabet(&gen(Shape::Random, 4096, &mut rng), 15),
    ));
    sources.push((
        "alpha16-131072".into(),
        mask_alphabet(&gen(Shape::Mixed, 131072, &mut rng), 15),
    ));
    // A flat 256-symbol alphabet cannot be serialised by HUF_writeCTable
    // (`maxSymbolValue > 128` => ERROR(GENERIC)), so also cover incompressible
    // data over a 101-symbol alphabet, plus a skewed 256-symbol alphabet.
    sources.push((
        "alpha101-4096".into(),
        mask_alphabet(&gen(Shape::Random, 4096, &mut rng), 100),
    ));
    sources.push((
        "alpha101-131072".into(),
        mask_alphabet(&gen(Shape::Random, 131072, &mut rng), 100),
    ));
    {
        let mut skew = gen(Shape::Random, 131072, &mut rng);
        for b in skew.iter_mut() {
            if rng.below(4) != 0 {
                *b = (*b as u16 % 24) as u8;
            }
        }
        sources.push(("skew-131072".into(), skew));
    }

    for &huff_log in &[5u32, 8, 11, 12] {
        for &flags in &[0i32, 1, 2, 4, 8, 16, 32, 48, 63] {
            for (sname, src) in &sources {
                let ctx = format!("huf huffLog={huff_log} flags={flags} src={sname}");
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                let mut out_check: Vec<Vec<u8>> = Vec::new();
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let is_c = which == 0;
                    let t = match huf_build(is_c, src, huff_log, flags, rec) {
                        Some(t) => t,
                        None => continue,
                    };

                    // ---- 4X + 1X compression with the freshly built table
                    let mut b4 = vec![0u8; huf_compress_bound(src.len())];
                    let c4ret = unsafe {
                        let f = if is_c { &c_c4 } else { &r_c4 };
                        f(
                            b4.as_mut_ptr() as *mut c_void,
                            b4.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            t.ctable.as_ptr(),
                            flags,
                        )
                    };
                    rec.ret("HUF_compress4X_usingCTable", c4ret);
                    rec.buf("b4", &b4);

                    let mut b1 = vec![0u8; huf_compress_bound(src.len())];
                    let c1ret = unsafe {
                        let f = if is_c { &c_c1 } else { &r_c1 };
                        f(
                            b1.as_mut_ptr() as *mut c_void,
                            b1.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            t.ctable.as_ptr(),
                            flags,
                        )
                    };
                    rec.ret("HUF_compress1X_usingCTable", c1ret);
                    rec.buf("b1", &b1);

                    // tight destination => 0 / error, must match
                    let mut tight = vec![0u8; 8];
                    let tret = unsafe {
                        let f = if is_c { &c_c4 } else { &r_c4 };
                        f(
                            tight.as_mut_ptr() as *mut c_void,
                            tight.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            t.ctable.as_ptr(),
                            flags,
                        )
                    };
                    rec.ret("HUF_compress4X_usingCTable(tight)", tret);

                    // ---- decoding tables
                    let mut dt1 = vec![0u32; HUF_DTABLE_U32];
                    dt1[0] = HUF_DTABLE_CAPACITY_LOG * 0x0100_0001;
                    let mut ddwksp = aligned_wksp(HUF_DECOMPRESS_WORKSPACE_SIZE * 8);
                    let rd1 = unsafe {
                        let f = if is_c { &c_rd1 } else { &r_rd1 };
                        f(
                            dt1.as_mut_ptr(),
                            t.header.as_ptr() as *const c_void,
                            t.header_len,
                            ddwksp.as_mut_ptr() as *mut c_void,
                            ddwksp.len() * 8,
                            flags,
                        )
                    };
                    rec.ret("HUF_readDTableX1_wksp", rd1);
                    rec.buf32("dt1", &dt1);
                    let rd1s = unsafe {
                        let f = if is_c { &c_rd1 } else { &r_rd1 };
                        f(
                            dt1.as_mut_ptr(),
                            t.header.as_ptr() as *const c_void,
                            t.header_len,
                            ddwksp.as_mut_ptr() as *mut c_void,
                            8,
                            flags,
                        )
                    };
                    rec.ret("HUF_readDTableX1_wksp(wksp=8)", rd1s);

                    let mut dt2 = vec![0u32; HUF_DTABLE_U32];
                    dt2[0] = HUF_DTABLE_CAPACITY_LOG * 0x0100_0001;
                    let rd2 = unsafe {
                        let f = if is_c { &c_rd2 } else { &r_rd2 };
                        f(
                            dt2.as_mut_ptr(),
                            t.header.as_ptr() as *const c_void,
                            t.header_len,
                            ddwksp.as_mut_ptr() as *mut c_void,
                            ddwksp.len() * 8,
                            flags,
                        )
                    };
                    rec.ret("HUF_readDTableX2_wksp", rd2);
                    rec.buf32("dt2", &dt2);
                    let rd2s = unsafe {
                        let f = if is_c { &c_rd2 } else { &r_rd2 };
                        f(
                            dt2.as_mut_ptr(),
                            t.header.as_ptr() as *const c_void,
                            t.header_len,
                            ddwksp.as_mut_ptr() as *mut c_void,
                            8,
                            flags,
                        )
                    };
                    rec.ret("HUF_readDTableX2_wksp(wksp=8)", rd2s);

                    // ---- decompress via *_usingDTable (X1 and X2 tables)
                    let mut regen: Vec<Vec<u8>> = Vec::new();
                    if !is_error(c4ret) && c4ret >= 10 && !is_error(rd1) {
                        let mut out = vec![0u8; src.len()];
                        let d = unsafe {
                            let f = if is_c { &c_d4 } else { &r_d4 };
                            f(
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                b4.as_ptr() as *const c_void,
                                c4ret,
                                dt1.as_ptr(),
                                flags,
                            )
                        };
                        rec.ret("HUF_decompress4X_usingDTable(X1)", d);
                        rec.buf("out4x1", &out);
                        if !is_error(d) {
                            regen.push(out[..d].to_vec());
                        }
                    }
                    if !is_error(c4ret) && c4ret >= 10 && !is_error(rd2) {
                        let mut out = vec![0u8; src.len()];
                        let d = unsafe {
                            let f = if is_c { &c_d4 } else { &r_d4 };
                            f(
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                b4.as_ptr() as *const c_void,
                                c4ret,
                                dt2.as_ptr(),
                                flags,
                            )
                        };
                        rec.ret("HUF_decompress4X_usingDTable(X2)", d);
                        rec.buf("out4x2", &out);
                        if !is_error(d) {
                            regen.push(out[..d].to_vec());
                        }
                    }
                    if !is_error(c1ret) && c1ret > 0 && !is_error(rd1) {
                        let mut out = vec![0u8; src.len()];
                        let d = unsafe {
                            let f = if is_c { &c_d1 } else { &r_d1 };
                            f(
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                b1.as_ptr() as *const c_void,
                                c1ret,
                                dt1.as_ptr(),
                                flags,
                            )
                        };
                        rec.ret("HUF_decompress1X_usingDTable(X1)", d);
                        rec.buf("out1x1", &out);
                        if !is_error(d) {
                            regen.push(out[..d].to_vec());
                        }
                    }
                    if !is_error(c1ret) && c1ret > 0 && !is_error(rd2) {
                        let mut out = vec![0u8; src.len()];
                        let d = unsafe {
                            let f = if is_c { &c_d1 } else { &r_d1 };
                            f(
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                b1.as_ptr() as *const c_void,
                                c1ret,
                                dt2.as_ptr(),
                                flags,
                            )
                        };
                        rec.ret("HUF_decompress1X_usingDTable(X2)", d);
                        rec.buf("out1x2", &out);
                        if !is_error(d) {
                            regen.push(out[..d].to_vec());
                        }
                    }

                    // ---- the header+payload (`hufOnly` / DCtx) entry points
                    let mut f4 = t.header[..t.header_len].to_vec();
                    if !is_error(c4ret) {
                        f4.extend_from_slice(&b4[..c4ret]);
                    }
                    let mut f1 = t.header[..t.header_len].to_vec();
                    if !is_error(c1ret) {
                        f1.extend_from_slice(&b1[..c1ret]);
                    }

                    if !is_error(c4ret) && c4ret >= 10 {
                        let mut dctx = vec![0u32; HUF_DTABLE_U32];
                        dctx[0] = HUF_DTABLE_CAPACITY_LOG * 0x0100_0001;
                        let mut out = vec![0u8; src.len()];
                        let mut w = aligned_wksp(HUF_DECOMPRESS_WORKSPACE_SIZE * 8);
                        let d = unsafe {
                            let f = if is_c { &c_ho } else { &r_ho };
                            f(
                                dctx.as_mut_ptr(),
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                f4.as_ptr() as *const c_void,
                                f4.len(),
                                w.as_mut_ptr() as *mut c_void,
                                w.len() * 8,
                                flags,
                            )
                        };
                        rec.ret("HUF_decompress4X_hufOnly_wksp", d);
                        rec.buf("outHufOnly", &out);
                        rec.buf32("dctxHufOnly", &dctx);
                        if !is_error(d) {
                            regen.push(out[..d].to_vec());
                        }
                    }
                    if !is_error(c1ret) && c1ret > 0 {
                        for variant in 0..3 {
                            let mut dctx = vec![0u32; HUF_DTABLE_U32];
                            dctx[0] = HUF_DTABLE_CAPACITY_LOG * 0x0100_0001;
                            let mut out = vec![0u8; src.len()];
                            let mut w = aligned_wksp(HUF_DECOMPRESS_WORKSPACE_SIZE * 8);
                            let d = unsafe {
                                let f = match (variant, is_c) {
                                    (0, true) => &c_x11,
                                    (0, false) => &r_x11,
                                    (1, true) => &c_x12,
                                    (1, false) => &r_x12,
                                    (_, true) => &c_x1,
                                    (_, false) => &r_x1,
                                };
                                f(
                                    dctx.as_mut_ptr(),
                                    out.as_mut_ptr() as *mut c_void,
                                    out.len(),
                                    f1.as_ptr() as *const c_void,
                                    f1.len(),
                                    w.as_mut_ptr() as *mut c_void,
                                    w.len() * 8,
                                    flags,
                                )
                            };
                            let name = match variant {
                                0 => "HUF_decompress1X1_DCtx_wksp",
                                1 => "HUF_decompress1X2_DCtx_wksp",
                                _ => "HUF_decompress1X_DCtx_wksp",
                            };
                            rec.ret(name, d);
                            rec.buf("out1XDCtx", &out);
                            rec.buf32("dctx1X", &dctx);
                            if !is_error(d) {
                                regen.push(out[..d].to_vec());
                            }
                        }
                    }
                    if is_c {
                        out_check = regen;
                    }
                }
                cmp_rec(&ctx, &recs[0], &recs[1]);
                // sanity: every successful decode must reproduce the input
                for (i, g) in out_check.iter().enumerate() {
                    assert_bytes_eq(&format!("{ctx}: regen #{i}"), src, g);
                }
                assert!(
                    !out_check.is_empty() || sname.starts_with("random-"),
                    "{ctx}: no successful decode at all"
                );
            }
        }
    }
}

// ============================================================== row 208 ====

/// Row 208: `HUF_compress{4,1}X_repeat` across three successive blocks with
/// each initial `*repeat` value. The table is primed by a first
/// `HUF_repeat_none` call (the C tree walker is not defence-hardened against a
/// zero-filled "valid" table), then the requested mode is installed.
#[test]
fn huf_repeat() {
    let (c_r4, r_r4) = unsafe { pair::<FnHufCompressRepeat>("HUF_compress4X_repeat") };
    let (c_r1, r_r1) = unsafe { pair::<FnHufCompressRepeat>("HUF_compress1X_repeat") };

    let mut rng = Rng::new(0x0208_0001);
    // blocks 0/1 similar, block 2 deliberately dissimilar
    let mut block_sets: Vec<[Vec<u8>; 3]> = Vec::new();
    for _ in 0..4 {
        let a = gen(Shape::Text, 4096, &mut rng);
        let mut b = a.clone();
        for _ in 0..64 {
            let p = rng.below(b.len());
            b[p] = a[rng.below(a.len())];
        }
        let c = gen(Shape::Random, 4096, &mut rng);
        block_sets.push([a, b, c]);
    }
    for _ in 0..2 {
        let a = mask_alphabet(&gen(Shape::Mixed, 8192, &mut rng), 63);
        let b = mask_alphabet(&gen(Shape::Mixed, 8192, &mut rng), 63);
        let c = gen(Shape::Sparse, 8192, &mut rng);
        block_sets.push([a, b, c]);
    }

    for (bi, blocks) in block_sets.iter().enumerate() {
        for &nb_streams in &[4u32, 1] {
            for &mode in &[0i32, 1, 2] {
                for &flags in &[0i32, 2, 4] {
                    for &table_log in &[0u32, 11, 12] {
                        let ctx = format!(
                            "huf_repeat blocks#{bi} {nb_streams}X mode={mode} flags={flags} tableLog={table_log}"
                        );
                        let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                        for which in 0..2 {
                            let rec = &mut recs[which];
                            let is_c = which == 0;
                            let mut huf_table = vec![0u64; HUF_CTABLE_ST];
                            let mut repeat: c_int = 0; // HUF_repeat_none
                            let mut wksp = aligned_wksp(HUF_WORKSPACE_SIZE);
                            for (i, src) in blocks.iter().enumerate() {
                                if i == 1 {
                                    repeat = mode;
                                }
                                let mut dst = vec![0u8; huf_compress_bound(src.len())];
                                let ret = unsafe {
                                    let f = match (nb_streams, is_c) {
                                        (4, true) => &c_r4,
                                        (4, false) => &r_r4,
                                        (_, true) => &c_r1,
                                        (_, false) => &r_r1,
                                    };
                                    f(
                                        dst.as_mut_ptr() as *mut c_void,
                                        dst.len(),
                                        src.as_ptr() as *const c_void,
                                        src.len(),
                                        255,
                                        table_log,
                                        wksp.as_mut_ptr() as *mut c_void,
                                        wksp.len() * 8,
                                        huf_table.as_mut_ptr(),
                                        &mut repeat,
                                        flags,
                                    )
                                };
                                rec.ret("ret", ret);
                                rec.reti("repeat", repeat as i64);
                                rec.buf("dst", &dst);
                                rec.buf64("hufTable", &huf_table);
                            }
                        }
                        cmp_rec(&ctx, &recs[0], &recs[1]);
                    }
                }
            }
        }
    }

    // NULL hufTable / NULL repeat pointer variant
    for (bi, blocks) in block_sets.iter().enumerate().take(2) {
        for &nb_streams in &[4u32, 1] {
            let ctx = format!("huf_repeat null blocks#{bi} {nb_streams}X");
            let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
            for which in 0..2 {
                let rec = &mut recs[which];
                let is_c = which == 0;
                let mut wksp = aligned_wksp(HUF_WORKSPACE_SIZE);
                for src in blocks.iter() {
                    let mut dst = vec![0u8; huf_compress_bound(src.len())];
                    let ret = unsafe {
                        let f = match (nb_streams, is_c) {
                            (4, true) => &c_r4,
                            (4, false) => &r_r4,
                            (_, true) => &c_r1,
                            (_, false) => &r_r1,
                        };
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            src.as_ptr() as *const c_void,
                            src.len(),
                            255,
                            11,
                            wksp.as_mut_ptr() as *mut c_void,
                            wksp.len() * 8,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            0,
                        )
                    };
                    rec.ret("ret", ret);
                    rec.buf("dst", &dst);
                }
            }
            cmp_rec(&ctx, &recs[0], &recs[1]);
        }
    }
}

// ============================================================== row 210 ====

/// Row 210: every HUF helper/introspection entry point, exercised on real
/// CTables built by `huf_build` and on a 128 B random input.
#[test]
fn huf_helpers() {
    let (c_min, r_min) = unsafe { pair::<FnHufMinTableLog>("HUF_minTableLog") };
    let (c_card, r_card) = unsafe { pair::<FnHufCardinality>("HUF_cardinality") };
    let (c_val, r_val) = unsafe { pair::<FnHufValidateCTable>("HUF_validateCTable") };
    let (c_est, r_est) = unsafe { pair::<FnHufEstimate>("HUF_estimateCompressedSize") };
    let (c_nb, r_nb) = unsafe { pair::<FnHufGetNbBits>("HUF_getNbBitsFromCTable") };
    let (c_rct, r_rct) = unsafe { pair::<FnHufReadCTable>("HUF_readCTable") };
    let (c_hdr, r_hdr) = unsafe { pair::<FnHufReadCTableHeader>("HUF_readCTableHeader") };
    let (c_sel, r_sel) = unsafe { pair::<FnHufSelectDecoder>("HUF_selectDecoder") };
    let (c_rs, r_rs) = unsafe { pair::<FnHufReadStats>("HUF_readStats") };
    let (c_rsw, r_rsw) = unsafe { pair::<FnHufReadStatsWksp>("HUF_readStats_wksp") };
    let (c_cb, r_cb) = unsafe { pair::<FnSizeSize>("HUF_compressBound") };
    let (c_ie, r_ie) = unsafe { pair::<FnIsError>("HUF_isError") };
    let (c_en, r_en) = unsafe { pair::<FnErrName>("HUF_getErrorName") };

    // ---- scalar helpers
    let mut rng = Rng::new(0x0210_0001);
    for card in 1..=512u32 {
        assert_eq!(
            unsafe { c_min(card) },
            unsafe { r_min(card) },
            "HUF_minTableLog({card})"
        );
    }
    let mut sizes: Vec<usize> = vec![0, 1, 100, 4096, 131072, 1 << 20];
    for _ in 0..2000 {
        sizes.push(rng.below(1 << 22));
    }
    for &s in &sizes {
        assert_eq!(
            unsafe { c_cb(s) },
            unsafe { r_cb(s) },
            "HUF_compressBound({s})"
        );
    }
    for e in 0..=200usize {
        let code = 0usize.wrapping_sub(e);
        assert_eq!(
            unsafe { c_ie(code) },
            unsafe { r_ie(code) },
            "HUF_isError({code:#x})"
        );
        assert_eq!(
            cstr_of(unsafe { c_en(code) }),
            cstr_of(unsafe { r_en(code) }),
            "HUF_getErrorName({code:#x})"
        );
    }
    for _ in 0..2000 {
        let code = rng.next_u64() as usize;
        assert_eq!(
            unsafe { c_ie(code) },
            unsafe { r_ie(code) },
            "HUF_isError({code:#x})"
        );
        assert_eq!(
            cstr_of(unsafe { c_en(code) }),
            cstr_of(unsafe { r_en(code) }),
            "HUF_getErrorName({code:#x})"
        );
    }

    // ---- HUF_selectDecoder over a grid + randomized pairs
    for &dst in &[0usize, 1, 12, 100, 1024, 4096, 65536, 131072] {
        for &csrc in &[0usize, 1, 10, 12, 64, 500, 4096, 131072] {
            assert_eq!(
                unsafe { c_sel(dst, csrc) },
                unsafe { r_sel(dst, csrc) },
                "HUF_selectDecoder({dst},{csrc})"
            );
        }
    }
    for _ in 0..5000 {
        let d = rng.below(1 << 18);
        let c = rng.below(1 << 18);
        assert_eq!(
            unsafe { c_sel(d, c) },
            unsafe { r_sel(d, c) },
            "HUF_selectDecoder({d},{c})"
        );
    }

    // ---- table-driven helpers
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    inputs.push(("random-128".into(), gen(Shape::Random, 128, &mut rng)));
    inputs.push(("text-128".into(), gen(Shape::Text, 128, &mut rng)));
    inputs.push(("text-4096".into(), gen(Shape::Text, 4096, &mut rng)));
    inputs.push(("random-4096".into(), gen(Shape::Random, 4096, &mut rng)));
    inputs.push((
        "alpha16-4096".into(),
        mask_alphabet(&gen(Shape::Random, 4096, &mut rng), 15),
    ));
    inputs.push(("rle-4096".into(), vec![0x33u8; 4096]));
    for i in 0..4 {
        inputs.push((format!("random-128#{i}"), gen(Shape::Random, 128, &mut rng)));
        inputs.push((format!("mixed-8192#{i}"), gen(Shape::Mixed, 8192, &mut rng)));
    }

    for &huff_log in &[5u32, 8, 11, 12] {
        for (sname, src) in &inputs {
            for &flags in &[0i32, 2, 63] {
                let ctx = format!("huf_helpers huffLog={huff_log} flags={flags} src={sname}");
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let is_c = which == 0;
                    let t = match huf_build(is_c, src, huff_log, flags, rec) {
                        Some(t) => t,
                        None => continue,
                    };

                    // cardinality across several maxSymbolValue caps
                    for &cap in &[0u32, 1, 15, 100, 255] {
                        let v = unsafe {
                            let f = if is_c { &c_card } else { &r_card };
                            f(t.count.as_ptr(), cap)
                        };
                        rec.retu("HUF_cardinality", v as u64);
                    }
                    // validateCTable / estimateCompressedSize
                    for &cap in &[0u32, 1, 15, 100, 255, t.max_symbol_value] {
                        let v = unsafe {
                            let f = if is_c { &c_val } else { &r_val };
                            f(t.ctable.as_ptr(), t.count.as_ptr(), cap)
                        };
                        rec.reti("HUF_validateCTable", v as i64);
                        let e = unsafe {
                            let f = if is_c { &c_est } else { &r_est };
                            f(t.ctable.as_ptr(), t.count.as_ptr(), cap)
                        };
                        rec.ret("HUF_estimateCompressedSize", e);
                    }
                    // nbBits for every symbol (including past maxSymbolValue)
                    for sym in 0..=255u32 {
                        let v = unsafe {
                            let f = if is_c { &c_nb } else { &r_nb };
                            f(t.ctable.as_ptr(), sym)
                        };
                        rec.retu("HUF_getNbBitsFromCTable", v as u64);
                    }
                    // header
                    let h = unsafe {
                        let f = if is_c { &c_hdr } else { &r_hdr };
                        f(t.ctable.as_ptr())
                    };
                    rec.retu("hdr.tableLog", h.tableLog as u64);
                    rec.retu("hdr.maxSymbolValue", h.maxSymbolValue as u64);
                    rec.buf("hdr.unused", &h.unused);
                    assert_eq!(
                        h.tableLog as u32, t.table_log,
                        "{ctx}: header tableLog vs buildCTable return"
                    );

                    // HUF_readCTable round trip
                    for &cap in &[t.max_symbol_value, 255, 1] {
                        let mut ct2 = vec![0u64; HUF_CTABLE_ST];
                        let mut msv2: c_uint = cap;
                        let mut zw: c_uint = 0xDEAD;
                        let ret = unsafe {
                            let f = if is_c { &c_rct } else { &r_rct };
                            f(
                                ct2.as_mut_ptr(),
                                &mut msv2,
                                t.header.as_ptr() as *const c_void,
                                t.header_len,
                                &mut zw,
                            )
                        };
                        rec.ret("HUF_readCTable", ret);
                        rec.retu("HUF_readCTable.maxSV", msv2 as u64);
                        rec.retu("HUF_readCTable.hasZeroWeights", zw as u64);
                        rec.buf64("HUF_readCTable.ctable", &ct2);
                    }
                    // truncated header
                    for cut in [0usize, 1, t.header_len / 2] {
                        let mut ct2 = vec![0u64; HUF_CTABLE_ST];
                        let mut msv2: c_uint = 255;
                        let mut zw: c_uint = 0;
                        let ret = unsafe {
                            let f = if is_c { &c_rct } else { &r_rct };
                            f(
                                ct2.as_mut_ptr(),
                                &mut msv2,
                                t.header.as_ptr() as *const c_void,
                                cut,
                                &mut zw,
                            )
                        };
                        rec.ret("HUF_readCTable(trunc)", ret);
                        rec.buf64("HUF_readCTable(trunc).ctable", &ct2);
                    }

                    // HUF_readStats / HUF_readStats_wksp
                    for &sz in &[
                        t.header_len,
                        t.header_len + 8,
                        t.header_len.saturating_sub(1),
                        1,
                        0,
                    ] {
                        let mut hw = [0u8; 256];
                        let mut rank = [0u32; 16];
                        let mut nb: c_uint = 0;
                        let mut tl: c_uint = 0;
                        let ret = unsafe {
                            let f = if is_c { &c_rs } else { &r_rs };
                            f(
                                hw.as_mut_ptr(),
                                hw.len(),
                                rank.as_mut_ptr(),
                                &mut nb,
                                &mut tl,
                                t.header.as_ptr() as *const c_void,
                                sz,
                            )
                        };
                        rec.ret("HUF_readStats", ret);
                        rec.retu("HUF_readStats.nbSymbols", nb as u64);
                        rec.retu("HUF_readStats.tableLog", tl as u64);
                        rec.buf("HUF_readStats.huffWeight", &hw);
                        rec.buf32("HUF_readStats.rankStats", &rank);

                        for &wflags in &[0i32, 1] {
                            let mut hw2 = [0u8; 256];
                            let mut rank2 = [0u32; 16];
                            let mut nb2: c_uint = 0;
                            let mut tl2: c_uint = 0;
                            let mut w = aligned_wksp(
                                fse_decompress_wksp_u32(6, HUF_TABLELOG_MAX - 1) * 4 + 64,
                            );
                            let ret = unsafe {
                                let f = if is_c { &c_rsw } else { &r_rsw };
                                f(
                                    hw2.as_mut_ptr(),
                                    hw2.len(),
                                    rank2.as_mut_ptr(),
                                    &mut nb2,
                                    &mut tl2,
                                    t.header.as_ptr() as *const c_void,
                                    sz,
                                    w.as_mut_ptr() as *mut c_void,
                                    w.len() * 8,
                                    wflags,
                                )
                            };
                            rec.ret("HUF_readStats_wksp", ret);
                            rec.retu("HUF_readStats_wksp.nbSymbols", nb2 as u64);
                            rec.retu("HUF_readStats_wksp.tableLog", tl2 as u64);
                            rec.buf("HUF_readStats_wksp.huffWeight", &hw2);
                            rec.buf32("HUF_readStats_wksp.rankStats", &rank2);
                        }
                        // hwSize too small / workspace too small
                        let mut hw3 = [0u8; 4];
                        let mut rank3 = [0u32; 16];
                        let mut nb3: c_uint = 0;
                        let mut tl3: c_uint = 0;
                        let ret = unsafe {
                            let f = if is_c { &c_rs } else { &r_rs };
                            f(
                                hw3.as_mut_ptr(),
                                hw3.len(),
                                rank3.as_mut_ptr(),
                                &mut nb3,
                                &mut tl3,
                                t.header.as_ptr() as *const c_void,
                                sz,
                            )
                        };
                        rec.ret("HUF_readStats(hwSize=4)", ret);
                        let mut w = aligned_wksp(64);
                        let ret = unsafe {
                            let f = if is_c { &c_rsw } else { &r_rsw };
                            f(
                                hw3.as_mut_ptr(),
                                256,
                                rank3.as_mut_ptr(),
                                &mut nb3,
                                &mut tl3,
                                t.header.as_ptr() as *const c_void,
                                sz,
                                w.as_mut_ptr() as *mut c_void,
                                8,
                                0,
                            )
                        };
                        rec.ret("HUF_readStats_wksp(wksp=8)", ret);
                    }
                }
                cmp_rec(&ctx, &recs[0], &recs[1]);
            }
        }
    }
}

// ========================================================= rows 211/212 ====

fn xxh_lengths() -> Vec<usize> {
    vec![
        0, 1, 3, 4, 7, 8, 15, 16, 31, 32, 33, 63, 64, 127, 128, 1000, 131072,
    ]
}

/// Row 211: `ZSTD_XXH32` one-shot over the required length × seed grid, on
/// every generator shape.
#[test]
fn xxh32() {
    let (c32, r32) = unsafe { pair::<FnXxh32>("ZSTD_XXH32") };
    let mut rng = Rng::new(0x0211_0001);
    let seeds: [u32; 4] = [0, 1, 0x9E37_79B1, 0xFFFF_FFFF];
    for &len in &xxh_lengths() {
        for &shape in ALL_SHAPES {
            let src = gen(shape, len, &mut rng);
            for &seed in &seeds {
                let c = unsafe { c32(src.as_ptr() as *const c_void, len, seed) };
                let r = unsafe { r32(src.as_ptr() as *const c_void, len, seed) };
                assert_eq!(c, r, "ZSTD_XXH32 len={len} shape={shape:?} seed={seed:#x}");
            }
        }
        // NULL / zero-length source
        if len == 0 {
            for &seed in &seeds {
                let c = unsafe { c32(std::ptr::null(), 0, seed) };
                let r = unsafe { r32(std::ptr::null(), 0, seed) };
                assert_eq!(c, r, "ZSTD_XXH32(NULL,0,{seed:#x})");
            }
        }
    }
    // randomized lengths + seeds, and unaligned starts
    let big = gen(Shape::Mixed, 70_000, &mut rng);
    for _ in 0..4000 {
        let off = rng.below(64);
        let len = rng.below(big.len() - 64);
        let seed = rng.next_u32();
        let p = unsafe { big.as_ptr().add(off) } as *const c_void;
        assert_eq!(
            unsafe { c32(p, len, seed) },
            unsafe { r32(p, len, seed) },
            "ZSTD_XXH32 off={off} len={len} seed={seed:#x}"
        );
    }
}

/// Row 212: `ZSTD_XXH64` one-shot over the required length × seed grid.
#[test]
fn xxh64() {
    let (c64, r64) = unsafe { pair::<FnXxh64>("ZSTD_XXH64") };
    let mut rng = Rng::new(0x0212_0001);
    let seeds: [u64; 4] = [0, 1, 0x27D4_EB2F_1656_67C5, u64::MAX];
    for &len in &xxh_lengths() {
        for &shape in ALL_SHAPES {
            let src = gen(shape, len, &mut rng);
            for &seed in &seeds {
                let c = unsafe { c64(src.as_ptr() as *const c_void, len, seed) };
                let r = unsafe { r64(src.as_ptr() as *const c_void, len, seed) };
                assert_eq!(c, r, "ZSTD_XXH64 len={len} shape={shape:?} seed={seed:#x}");
            }
        }
        if len == 0 {
            for &seed in &seeds {
                let c = unsafe { c64(std::ptr::null(), 0, seed) };
                let r = unsafe { r64(std::ptr::null(), 0, seed) };
                assert_eq!(c, r, "ZSTD_XXH64(NULL,0,{seed:#x})");
            }
        }
    }
    let big = gen(Shape::Mixed, 70_000, &mut rng);
    for _ in 0..4000 {
        let off = rng.below(64);
        let len = rng.below(big.len() - 64);
        let seed = rng.next_u64();
        let p = unsafe { big.as_ptr().add(off) } as *const c_void;
        assert_eq!(
            unsafe { c64(p, len, seed) },
            unsafe { r64(p, len, seed) },
            "ZSTD_XXH64 off={off} len={len} seed={seed:#x}"
        );
    }
}

// ========================================================= rows 213/214 ====

/// Row 213: `ZSTD_XXH32` streaming API — create/reset/update/digest/copyState/
/// freeState with the required chunk splits, `update(NULL,0)` and a mid-stream
/// `copyState` whose digest is compared against the original's.
#[test]
fn xxh32_stream() {
    let (c_new, r_new) = unsafe { pair::<FnXxhCreate>("ZSTD_XXH32_createState") };
    let (c_free, r_free) = unsafe { pair::<FnXxhFree>("ZSTD_XXH32_freeState") };
    let (c_reset, r_reset) = unsafe { pair::<FnXxh32Reset>("ZSTD_XXH32_reset") };
    let (c_upd, r_upd) = unsafe { pair::<FnXxhUpdate>("ZSTD_XXH32_update") };
    let (c_dig, r_dig) = unsafe { pair::<FnXxh32Digest>("ZSTD_XXH32_digest") };
    let (c_copy, r_copy) = unsafe { pair::<FnXxhCopyState>("ZSTD_XXH32_copyState") };
    let (c_one, r_one) = unsafe { pair::<FnXxh32>("ZSTD_XXH32") };

    let mut rng = Rng::new(0x0213_0001);
    let mut srcs: Vec<(String, Vec<u8>)> = Vec::new();
    for &shape in &[Shape::Random, Shape::Text, Shape::Mixed] {
        srcs.push((format!("{shape:?}-4096"), gen(shape, 4096, &mut rng)));
        srcs.push((format!("{shape:?}-131072"), gen(shape, 131072, &mut rng)));
    }

    for (sname, src) in &srcs {
        for &chunk in &[1usize, 3, 7, 16, 1000] {
            for &seed in &[0u32, 1, 0x9E37_79B1, 0xFFFF_FFFF] {
                let ctx = format!("xxh32_stream src={sname} chunk={chunk} seed={seed:#x}");
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let is_c = which == 0;
                    let st = unsafe {
                        let f = if is_c { &c_new } else { &r_new };
                        f()
                    };
                    assert!(!st.is_null(), "{ctx}: createState returned NULL");
                    let cp = unsafe {
                        let f = if is_c { &c_new } else { &r_new };
                        f()
                    };
                    assert!(!cp.is_null(), "{ctx}: createState returned NULL");

                    let rr = unsafe {
                        let f = if is_c { &c_reset } else { &r_reset };
                        f(st, seed)
                    };
                    rec.reti("reset", rr as i64);
                    // update(NULL, 0) must be accepted
                    let nr = unsafe {
                        let f = if is_c { &c_upd } else { &r_upd };
                        f(st, std::ptr::null(), 0)
                    };
                    rec.reti("update(NULL,0)", nr as i64);

                    let half = src.len() / 2;
                    let mut pos = 0usize;
                    let mut copied = false;
                    while pos < src.len() {
                        let n = chunk.min(src.len() - pos);
                        let ur = unsafe {
                            let f = if is_c { &c_upd } else { &r_upd };
                            f(st, src[pos..].as_ptr() as *const c_void, n)
                        };
                        rec.reti("update", ur as i64);
                        pos += n;
                        if !copied && pos >= half {
                            unsafe {
                                let f = if is_c { &c_copy } else { &r_copy };
                                f(cp, st)
                            };
                            let d = unsafe {
                                let f = if is_c { &c_dig } else { &r_dig };
                                f(cp)
                            };
                            rec.retu("digest(copy@half)", d as u64);
                            // the copy must agree with a one-shot over the same prefix
                            let expect = unsafe {
                                let f = if is_c { &c_one } else { &r_one };
                                f(src.as_ptr() as *const c_void, pos, seed)
                            };
                            assert_eq!(d, expect, "{ctx}: copyState digest vs one-shot");
                            copied = true;
                        }
                    }
                    let d = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(st)
                    };
                    rec.retu("digest", d as u64);
                    let one = unsafe {
                        let f = if is_c { &c_one } else { &r_one };
                        f(src.as_ptr() as *const c_void, src.len(), seed)
                    };
                    rec.retu("oneshot", one as u64);
                    assert_eq!(d, one, "{ctx}: streaming digest vs one-shot");

                    // continue feeding the copy and digest both again
                    let ur = unsafe {
                        let f = if is_c { &c_upd } else { &r_upd };
                        f(cp, src.as_ptr() as *const c_void, src.len())
                    };
                    rec.reti("update(copy,all)", ur as i64);
                    let d2 = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(cp)
                    };
                    rec.retu("digest(copy,after)", d2 as u64);
                    let d3 = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(st)
                    };
                    rec.retu("digest(again)", d3 as u64);

                    let f1 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(st)
                    };
                    let f2 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(cp)
                    };
                    rec.reti("freeState", f1 as i64);
                    rec.reti("freeState", f2 as i64);
                    let f3 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(std::ptr::null_mut())
                    };
                    rec.reti("freeState(NULL)", f3 as i64);
                }
                cmp_rec(&ctx, &recs[0], &recs[1]);
            }
        }
    }
}

/// Row 214: `ZSTD_XXH64` streaming API, same coverage as row 213.
#[test]
fn xxh64_stream() {
    let (c_new, r_new) = unsafe { pair::<FnXxhCreate>("ZSTD_XXH64_createState") };
    let (c_free, r_free) = unsafe { pair::<FnXxhFree>("ZSTD_XXH64_freeState") };
    let (c_reset, r_reset) = unsafe { pair::<FnXxh64Reset>("ZSTD_XXH64_reset") };
    let (c_upd, r_upd) = unsafe { pair::<FnXxhUpdate>("ZSTD_XXH64_update") };
    let (c_dig, r_dig) = unsafe { pair::<FnXxh64Digest>("ZSTD_XXH64_digest") };
    let (c_copy, r_copy) = unsafe { pair::<FnXxhCopyState>("ZSTD_XXH64_copyState") };
    let (c_one, r_one) = unsafe { pair::<FnXxh64>("ZSTD_XXH64") };

    let mut rng = Rng::new(0x0214_0001);
    let mut srcs: Vec<(String, Vec<u8>)> = Vec::new();
    for &shape in &[Shape::Random, Shape::Text, Shape::Mixed] {
        srcs.push((format!("{shape:?}-4096"), gen(shape, 4096, &mut rng)));
        srcs.push((format!("{shape:?}-131072"), gen(shape, 131072, &mut rng)));
    }

    for (sname, src) in &srcs {
        for &chunk in &[1usize, 3, 7, 16, 1000] {
            for &seed in &[0u64, 1, 0x27D4_EB2F_1656_67C5, u64::MAX] {
                let ctx = format!("xxh64_stream src={sname} chunk={chunk} seed={seed:#x}");
                let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                for which in 0..2 {
                    let rec = &mut recs[which];
                    let is_c = which == 0;
                    let st = unsafe {
                        let f = if is_c { &c_new } else { &r_new };
                        f()
                    };
                    let cp = unsafe {
                        let f = if is_c { &c_new } else { &r_new };
                        f()
                    };
                    assert!(!st.is_null() && !cp.is_null(), "{ctx}: createState NULL");

                    let rr = unsafe {
                        let f = if is_c { &c_reset } else { &r_reset };
                        f(st, seed)
                    };
                    rec.reti("reset", rr as i64);
                    let nr = unsafe {
                        let f = if is_c { &c_upd } else { &r_upd };
                        f(st, std::ptr::null(), 0)
                    };
                    rec.reti("update(NULL,0)", nr as i64);

                    let half = src.len() / 2;
                    let mut pos = 0usize;
                    let mut copied = false;
                    while pos < src.len() {
                        let n = chunk.min(src.len() - pos);
                        let ur = unsafe {
                            let f = if is_c { &c_upd } else { &r_upd };
                            f(st, src[pos..].as_ptr() as *const c_void, n)
                        };
                        rec.reti("update", ur as i64);
                        pos += n;
                        if !copied && pos >= half {
                            unsafe {
                                let f = if is_c { &c_copy } else { &r_copy };
                                f(cp, st)
                            };
                            let d = unsafe {
                                let f = if is_c { &c_dig } else { &r_dig };
                                f(cp)
                            };
                            rec.retu("digest(copy@half)", d);
                            let expect = unsafe {
                                let f = if is_c { &c_one } else { &r_one };
                                f(src.as_ptr() as *const c_void, pos, seed)
                            };
                            assert_eq!(d, expect, "{ctx}: copyState digest vs one-shot");
                            copied = true;
                        }
                    }
                    let d = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(st)
                    };
                    rec.retu("digest", d);
                    let one = unsafe {
                        let f = if is_c { &c_one } else { &r_one };
                        f(src.as_ptr() as *const c_void, src.len(), seed)
                    };
                    rec.retu("oneshot", one);
                    assert_eq!(d, one, "{ctx}: streaming digest vs one-shot");

                    let ur = unsafe {
                        let f = if is_c { &c_upd } else { &r_upd };
                        f(cp, src.as_ptr() as *const c_void, src.len())
                    };
                    rec.reti("update(copy,all)", ur as i64);
                    let d2 = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(cp)
                    };
                    rec.retu("digest(copy,after)", d2);
                    let d3 = unsafe {
                        let f = if is_c { &c_dig } else { &r_dig };
                        f(st)
                    };
                    rec.retu("digest(again)", d3);

                    let f1 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(st)
                    };
                    let f2 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(cp)
                    };
                    rec.reti("freeState", f1 as i64);
                    rec.reti("freeState", f2 as i64);
                    let f3 = unsafe {
                        let f = if is_c { &c_free } else { &r_free };
                        f(std::ptr::null_mut())
                    };
                    rec.reti("freeState(NULL)", f3 as i64);
                }
                cmp_rec(&ctx, &recs[0], &recs[1]);
            }
        }
    }
}

// ============================================================== row 215 ====

/// Row 215: canonical (big-endian) hash serialization round trips and
/// `ZSTD_XXH_versionNumber`.
#[test]
fn xxh_canonical() {
    let (c_ver, r_ver) = unsafe { pair::<FnUVoid>("ZSTD_XXH_versionNumber") };
    let (c_c32, r_c32) = unsafe { pair::<FnXxh32Canon>("ZSTD_XXH32_canonicalFromHash") };
    let (c_h32, r_h32) = unsafe { pair::<FnXxh32FromCanon>("ZSTD_XXH32_hashFromCanonical") };
    let (c_c64, r_c64) = unsafe { pair::<FnXxh64Canon>("ZSTD_XXH64_canonicalFromHash") };
    let (c_h64, r_h64) = unsafe { pair::<FnXxh64FromCanon>("ZSTD_XXH64_hashFromCanonical") };
    let (c_32, r_32) = unsafe { pair::<FnXxh32>("ZSTD_XXH32") };
    let (c_64, r_64) = unsafe { pair::<FnXxh64>("ZSTD_XXH64") };

    assert_eq!(
        unsafe { c_ver() },
        unsafe { r_ver() },
        "ZSTD_XXH_versionNumber"
    );

    let mut rng = Rng::new(0x0215_0001);
    let mut h32: Vec<u32> = vec![0, 1, 0x9E37_79B1, u32::MAX];
    let mut h64: Vec<u64> = vec![0, 1, 0x27D4_EB2F_1656_67C5, u64::MAX];
    // digests taken from real inputs (rows 211/212)
    for &len in &xxh_lengths() {
        let src = gen(Shape::Mixed, len, &mut rng);
        for &seed in &[0u32, 1, u32::MAX] {
            h32.push(unsafe { c_32(src.as_ptr() as *const c_void, len, seed) });
            assert_eq!(
                *h32.last().unwrap(),
                unsafe { r_32(src.as_ptr() as *const c_void, len, seed) },
                "ZSTD_XXH32 digest for canonical test"
            );
        }
        for &seed in &[0u64, 1, u64::MAX] {
            h64.push(unsafe { c_64(src.as_ptr() as *const c_void, len, seed) });
            assert_eq!(
                *h64.last().unwrap(),
                unsafe { r_64(src.as_ptr() as *const c_void, len, seed) },
                "ZSTD_XXH64 digest for canonical test"
            );
        }
    }
    for _ in 0..5000 {
        h32.push(rng.next_u32());
        h64.push(rng.next_u64());
    }

    for &h in &h32 {
        let mut cc = [0u8; 4];
        let mut rc = [0u8; 4];
        unsafe { c_c32(cc.as_mut_ptr() as *mut c_void, h) };
        unsafe { r_c32(rc.as_mut_ptr() as *mut c_void, h) };
        assert_bytes_eq(&format!("ZSTD_XXH32_canonicalFromHash({h:#x})"), &cc, &rc);
        assert_eq!(cc, h.to_be_bytes(), "canonical form must be big endian");
        let back_c = unsafe { c_h32(cc.as_ptr() as *const c_void) };
        let back_r = unsafe { r_h32(rc.as_ptr() as *const c_void) };
        assert_eq!(back_c, back_r, "ZSTD_XXH32_hashFromCanonical({h:#x})");
        assert_eq!(back_c, h, "XXH32 canonical round trip");
    }
    for &h in &h64 {
        let mut cc = [0u8; 8];
        let mut rc = [0u8; 8];
        unsafe { c_c64(cc.as_mut_ptr() as *mut c_void, h) };
        unsafe { r_c64(rc.as_mut_ptr() as *mut c_void, h) };
        assert_bytes_eq(&format!("ZSTD_XXH64_canonicalFromHash({h:#x})"), &cc, &rc);
        assert_eq!(cc, h.to_be_bytes(), "canonical form must be big endian");
        let back_c = unsafe { c_h64(cc.as_ptr() as *const c_void) };
        let back_r = unsafe { r_h64(rc.as_ptr() as *const c_void) };
        assert_eq!(back_c, back_r, "ZSTD_XXH64_hashFromCanonical({h:#x})");
        assert_eq!(back_c, h, "XXH64 canonical round trip");
    }
    // arbitrary canonical byte patterns (not produced by canonicalFromHash)
    for _ in 0..5000 {
        let b = rng.next_u64().to_le_bytes();
        assert_eq!(
            unsafe { c_h32(b.as_ptr() as *const c_void) },
            unsafe { r_h32(b.as_ptr() as *const c_void) },
            "ZSTD_XXH32_hashFromCanonical(raw)"
        );
        assert_eq!(
            unsafe { c_h64(b.as_ptr() as *const c_void) },
            unsafe { r_h64(b.as_ptr() as *const c_void) },
            "ZSTD_XXH64_hashFromCanonical(raw)"
        );
    }
}

// ======================================================= rows 216/217/218 ==

static JOB_COUNTER_C: AtomicUsize = AtomicUsize::new(0);
static JOB_COUNTER_R: AtomicUsize = AtomicUsize::new(0);
static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static FREE_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn inc_job(opaque: *mut c_void) {
    let c = &*(opaque as *const AtomicUsize);
    c.fetch_add(1, Ordering::SeqCst);
}

const CUSTOM_HDR: usize = 16;

unsafe extern "C" fn custom_alloc(_opaque: *mut c_void, size: usize) -> *mut c_void {
    ALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    let total = size + CUSTOM_HDR;
    let layout = std::alloc::Layout::from_size_align(total, CUSTOM_HDR).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    (p as *mut usize).write(total);
    p.add(CUSTOM_HDR) as *mut c_void
}

unsafe extern "C" fn custom_free(_opaque: *mut c_void, address: *mut c_void) {
    FREE_CALLS.fetch_add(1, Ordering::SeqCst);
    if address.is_null() {
        return;
    }
    let base = (address as *mut u8).sub(CUSTOM_HDR);
    let total = (base as *mut usize).read();
    let layout = std::alloc::Layout::from_size_align(total, CUSTOM_HDR).unwrap();
    std::alloc::dealloc(base, layout);
}

/// Row 216: `POOL_create` over the numThreads × queueSize grid, `POOL_sizeof`
/// and `POOL_free` (including `POOL_free(NULL)`).
///
/// NOTE: both libraries are built WITHOUT `ZSTD_MULTITHREAD`, so `POOL_*` are
/// synchronous stubs. The test therefore compares observable behaviour (NULL
/// vs non-NULL, sizeof, side effects) rather than assuming real threads.
#[test]
fn pool_basic() {
    let (c_new, r_new) = unsafe { pair::<FnPoolCreate>("POOL_create") };
    let (c_free, r_free) = unsafe { pair::<FnPoolVoid>("POOL_free") };
    let (c_size, r_size) = unsafe { pair::<FnPoolSizeof>("POOL_sizeof") };

    // POOL_sizeof(NULL) must be 0 in both
    assert_eq!(
        unsafe { c_size(std::ptr::null()) },
        unsafe { r_size(std::ptr::null()) },
        "POOL_sizeof(NULL)"
    );
    assert_eq!(unsafe { c_size(std::ptr::null()) }, 0, "POOL_sizeof(NULL)");
    // POOL_free(NULL) must be a no-op in both
    unsafe { c_free(std::ptr::null_mut()) };
    unsafe { r_free(std::ptr::null_mut()) };

    for &threads in &[0usize, 1, 2, 4, 16] {
        for &queue in &[0usize, 1, 4] {
            let ctx = format!("POOL_create({threads},{queue})");
            let cp = unsafe { c_new(threads, queue) };
            let rp = unsafe { r_new(threads, queue) };
            assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: NULL-ness differs");
            let cs = unsafe { c_size(cp) };
            let rs = unsafe { r_size(rp) };
            assert_eq!(cs, rs, "{ctx}: POOL_sizeof differs");
            unsafe { c_free(cp) };
            unsafe { r_free(rp) };
            // repeated create/free must stay stable
            let cp2 = unsafe { c_new(threads, queue) };
            let rp2 = unsafe { r_new(threads, queue) };
            assert_eq!(cp2.is_null(), rp2.is_null(), "{ctx}: 2nd create NULL-ness");
            assert_eq!(
                unsafe { c_size(cp2) },
                unsafe { r_size(rp2) },
                "{ctx}: 2nd POOL_sizeof"
            );
            unsafe { c_free(cp2) };
            unsafe { r_free(rp2) };
        }
    }
}

/// Row 217: `POOL_create_advanced` with the default (all-NULL) `ZSTD_customMem`,
/// with a Rust-implemented allocator pair, and with a deliberately mismatched
/// pair (alloc set / free NULL).
#[test]
fn pool_advanced() {
    let (c_adv, r_adv) = unsafe { pair::<FnPoolCreateAdv>("POOL_create_advanced") };
    let (c_free, r_free) = unsafe { pair::<FnPoolVoid>("POOL_free") };
    let (c_size, r_size) = unsafe { pair::<FnPoolSizeof>("POOL_sizeof") };
    let (c_add, r_add) = unsafe { pair::<FnPoolAdd>("POOL_add") };
    let (c_join, r_join) = unsafe { pair::<FnPoolVoid>("POOL_joinJobs") };

    let default_mem = ZstdCustomMem {
        customAlloc: None,
        customFree: None,
        opaque: std::ptr::null_mut(),
    };
    let custom_mem = ZstdCustomMem {
        customAlloc: Some(custom_alloc),
        customFree: Some(custom_free),
        opaque: std::ptr::null_mut(),
    };
    // mismatched: alloc provided, free missing => ZSTD_customMem_isValid() false
    let bad_mem = ZstdCustomMem {
        customAlloc: Some(custom_alloc),
        customFree: None,
        opaque: std::ptr::null_mut(),
    };
    let bad_mem2 = ZstdCustomMem {
        customAlloc: None,
        customFree: Some(custom_free),
        opaque: std::ptr::null_mut(),
    };

    for (mname, mem) in [
        ("default", default_mem),
        ("custom", custom_mem),
        ("alloc-only", bad_mem),
        ("free-only", bad_mem2),
    ] {
        for &threads in &[0usize, 1, 2, 4, 16] {
            for &queue in &[0usize, 1, 4] {
                let ctx = format!("POOL_create_advanced({threads},{queue},{mname})");
                let cp = unsafe { c_adv(threads, queue, mem) };
                let rp = unsafe { r_adv(threads, queue, mem) };
                assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: NULL-ness differs");
                assert_eq!(
                    unsafe { c_size(cp) },
                    unsafe { r_size(rp) },
                    "{ctx}: POOL_sizeof differs"
                );
                if !cp.is_null() {
                    // a job must still run (or not run) the same way in both
                    JOB_COUNTER_C.store(0, Ordering::SeqCst);
                    JOB_COUNTER_R.store(0, Ordering::SeqCst);
                    unsafe {
                        c_add(
                            cp,
                            Some(inc_job),
                            &JOB_COUNTER_C as *const AtomicUsize as *mut c_void,
                        )
                    };
                    unsafe {
                        r_add(
                            rp,
                            Some(inc_job),
                            &JOB_COUNTER_R as *const AtomicUsize as *mut c_void,
                        )
                    };
                    unsafe { c_join(cp) };
                    unsafe { r_join(rp) };
                    assert_eq!(
                        JOB_COUNTER_C.load(Ordering::SeqCst),
                        JOB_COUNTER_R.load(Ordering::SeqCst),
                        "{ctx}: job side effect differs"
                    );
                }
                unsafe { c_free(cp) };
                unsafe { r_free(rp) };
            }
        }
    }
}

/// Row 218: `POOL_add` / `POOL_tryAdd` / `POOL_joinJobs` / `POOL_resize` /
/// `POOL_sizeof`, with 0, 1 and 5 jobs and resizes to 0/1/4.
#[test]
fn pool_jobs() {
    let (c_new, r_new) = unsafe { pair::<FnPoolCreate>("POOL_create") };
    let (c_free, r_free) = unsafe { pair::<FnPoolVoid>("POOL_free") };
    let (c_join, r_join) = unsafe { pair::<FnPoolVoid>("POOL_joinJobs") };
    let (c_resize, r_resize) = unsafe { pair::<FnPoolResize>("POOL_resize") };
    let (c_size, r_size) = unsafe { pair::<FnPoolSizeof>("POOL_sizeof") };
    let (c_add, r_add) = unsafe { pair::<FnPoolAdd>("POOL_add") };
    let (c_try, r_try) = unsafe { pair::<FnPoolTryAdd>("POOL_tryAdd") };

    for &threads in &[1usize, 2, 4] {
        for &queue in &[0usize, 1, 4] {
            for &njobs in &[0usize, 1, 5] {
                for &use_try in &[false, true] {
                    let ctx = format!(
                        "pool_jobs threads={threads} queue={queue} jobs={njobs} tryAdd={use_try}"
                    );
                    let mut recs: [Rec; 2] = [Rec::default(), Rec::default()];
                    for which in 0..2 {
                        let rec = &mut recs[which];
                        let is_c = which == 0;
                        let counter = if is_c { &JOB_COUNTER_C } else { &JOB_COUNTER_R };
                        counter.store(0, Ordering::SeqCst);
                        let p = unsafe {
                            let f = if is_c { &c_new } else { &r_new };
                            f(threads, queue)
                        };
                        rec.reti("createdNonNull", (!p.is_null()) as i64);
                        if p.is_null() {
                            continue;
                        }
                        let opaque = counter as *const AtomicUsize as *mut c_void;
                        for _ in 0..njobs {
                            if use_try {
                                let t = unsafe {
                                    let f = if is_c { &c_try } else { &r_try };
                                    f(p, Some(inc_job), opaque)
                                };
                                rec.reti("POOL_tryAdd", t as i64);
                            } else {
                                unsafe {
                                    let f = if is_c { &c_add } else { &r_add };
                                    f(p, Some(inc_job), opaque)
                                };
                            }
                        }
                        unsafe {
                            let f = if is_c { &c_join } else { &r_join };
                            f(p)
                        };
                        rec.ret("counterAfterJoin", counter.load(Ordering::SeqCst));
                        rec.ret("POOL_sizeof", unsafe {
                            let f = if is_c { &c_size } else { &r_size };
                            f(p)
                        });
                        for &nt in &[0usize, 1, 4] {
                            let rr = unsafe {
                                let f = if is_c { &c_resize } else { &r_resize };
                                f(p, nt)
                            };
                            rec.reti("POOL_resize", rr as i64);
                            rec.ret("POOL_sizeof(after resize)", unsafe {
                                let f = if is_c { &c_size } else { &r_size };
                                f(p)
                            });
                        }
                        // add more jobs after the resizes
                        for _ in 0..njobs {
                            unsafe {
                                let f = if is_c { &c_add } else { &r_add };
                                f(p, Some(inc_job), opaque)
                            };
                        }
                        unsafe {
                            let f = if is_c { &c_join } else { &r_join };
                            f(p)
                        };
                        rec.ret("counterFinal", counter.load(Ordering::SeqCst));
                        unsafe {
                            let f = if is_c { &c_free } else { &r_free };
                            f(p)
                        };
                        // joinJobs / resize / sizeof on NULL
                        unsafe {
                            let f = if is_c { &c_join } else { &r_join };
                            f(std::ptr::null_mut())
                        };
                        rec.ret("POOL_sizeof(NULL)", unsafe {
                            let f = if is_c { &c_size } else { &r_size };
                            f(std::ptr::null())
                        });
                    }
                    cmp_rec(&ctx, &recs[0], &recs[1]);
                }
            }
        }
    }
}


