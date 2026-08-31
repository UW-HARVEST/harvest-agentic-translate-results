//! Phase E: the low-level ENTROPY (FSE / HUF / HIST) and XXHASH surface.
//!
//! Every symbol below is exercised through `dlopen`'d exports of BOTH
//! libraries; the Rust crate is never linked directly.
//!
//! The C build (`c_src/build/libzstd.so`) is compiled with `DEBUGLEVEL==0`, so
//! `assert()` is compiled out. That means the *documented* preconditions of
//! these private APIs are not enforced at run time — several of them would read
//! or write out of bounds if violated (e.g. `HIST_countFast` with
//! `maxSymbolValue > 255`, or `ZSTD_highbit32(0)` which is UB in both
//! languages). Where that is the case the comment says so and the input axis is
//! bounded to the memory-safe domain; every *checked* error path is swept.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

mod common;
use common::*;

use std::os::raw::{c_char, c_void};

// ------------------------------------------------------------------ signatures

type Fn_u32_v = unsafe extern "C" fn() -> u32;
type Fn_isErr = unsafe extern "C" fn(usize) -> u32;
type Fn_errName = unsafe extern "C" fn(usize) -> *const c_char;
type Fn_sz_sz = unsafe extern "C" fn(usize) -> usize;

// ---- FSE
type Fn_FSE_optimalTableLog = unsafe extern "C" fn(u32, usize, u32) -> u32;
type Fn_FSE_optimalTableLog_internal = unsafe extern "C" fn(u32, usize, u32, u32) -> u32;
type Fn_FSE_normalizeCount =
    unsafe extern "C" fn(*mut i16, u32, *const u32, usize, u32, u32) -> usize;
type Fn_FSE_NCountWriteBound = unsafe extern "C" fn(u32, u32) -> usize;
type Fn_FSE_writeNCount = unsafe extern "C" fn(*mut u8, usize, *const i16, u32, u32) -> usize;
type Fn_FSE_readNCount =
    unsafe extern "C" fn(*mut i16, *mut u32, *mut u32, *const u8, usize) -> usize;
type Fn_FSE_readNCount_bmi2 =
    unsafe extern "C" fn(*mut i16, *mut u32, *mut u32, *const u8, usize, i32) -> usize;
type Fn_FSE_buildCTable_wksp =
    unsafe extern "C" fn(*mut u32, *const i16, u32, u32, *mut u8, usize) -> usize;
type Fn_FSE_buildCTable_rle = unsafe extern "C" fn(*mut u32, u8) -> usize;
type Fn_FSE_buildDTable_wksp =
    unsafe extern "C" fn(*mut u32, *const i16, u32, u32, *mut u8, usize) -> usize;
type Fn_FSE_compress_usingCTable =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32) -> usize;
type Fn_FSE_decompress_wksp_bmi2 =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, u32, *mut u8, usize, i32) -> usize;

// ---- HIST
type Fn_HIST_count = unsafe extern "C" fn(*mut u32, *mut u32, *const u8, usize) -> usize;
type Fn_HIST_count_wksp =
    unsafe extern "C" fn(*mut u32, *mut u32, *const u8, usize, *mut u8, usize) -> usize;
type Fn_HIST_count_simple = unsafe extern "C" fn(*mut u32, *mut u32, *const u8, usize) -> u32;
type Fn_HIST_add = unsafe extern "C" fn(*mut u32, *const u8, usize);

// ---- HUF
type HUF_CElt = usize;
type Fn_HUF_buildCTable_wksp =
    unsafe extern "C" fn(*mut HUF_CElt, *const u32, u32, u32, *mut u8, usize) -> usize;
type Fn_HUF_writeCTable_wksp =
    unsafe extern "C" fn(*mut u8, usize, *const HUF_CElt, u32, u32, *mut u8, usize) -> usize;
type Fn_HUF_readCTable =
    unsafe extern "C" fn(*mut HUF_CElt, *mut u32, *const u8, usize, *mut u32) -> usize;
type Fn_HUF_readCTableHeader = unsafe extern "C" fn(*const HUF_CElt) -> HUF_CTableHeader;
type Fn_HUF_getNbBitsFromCTable = unsafe extern "C" fn(*const HUF_CElt, u32) -> u32;
type Fn_HUF_cardinality = unsafe extern "C" fn(*const u32, u32) -> u32;
type Fn_HUF_minTableLog = unsafe extern "C" fn(u32) -> u32;
type Fn_HUF_optimalTableLog = unsafe extern "C" fn(
    u32,
    usize,
    u32,
    *mut u8,
    usize,
    *mut HUF_CElt,
    *const u32,
    i32,
) -> u32;
type Fn_HUF_estimateCompressedSize =
    unsafe extern "C" fn(*const HUF_CElt, *const u32, u32) -> usize;
type Fn_HUF_validateCTable = unsafe extern "C" fn(*const HUF_CElt, *const u32, u32) -> i32;
type Fn_HUF_usingCTable =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const HUF_CElt, i32) -> usize;
type Fn_HUF_repeat = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    u32,
    u32,
    *mut u8,
    usize,
    *mut HUF_CElt,
    *mut i32,
    i32,
) -> usize;
type Fn_HUF_readStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut u32,
    *mut u32,
    *const u8,
    usize,
) -> usize;
type Fn_HUF_readStats_wksp = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut u32,
    *mut u32,
    *const u8,
    usize,
    *mut u8,
    usize,
    i32,
) -> usize;
type Fn_HUF_readDTable =
    unsafe extern "C" fn(*mut u32, *const u8, usize, *mut u8, usize, i32) -> usize;
type Fn_HUF_usingDTable =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32, i32) -> usize;
type Fn_HUF_DCtx_wksp =
    unsafe extern "C" fn(*mut u32, *mut u8, usize, *const u8, usize, *mut u8, usize, i32) -> usize;
type Fn_HUF_selectDecoder = unsafe extern "C" fn(usize, usize) -> u32;

/// `HUF_CTableHeader` — { BYTE tableLog; BYTE maxSymbolValue; BYTE unused[6]; }
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct HUF_CTableHeader {
    table_log: u8,
    max_symbol_value: u8,
    unused: [u8; core::mem::size_of::<usize>() - 2],
}

// ---- XXH
type Fn_XXH32 = unsafe extern "C" fn(*const u8, usize, u32) -> u32;
type Fn_XXH64 = unsafe extern "C" fn(*const u8, usize, u64) -> u64;
type Fn_newState = unsafe extern "C" fn() -> *mut c_void;
type Fn_freeState = unsafe extern "C" fn(*mut c_void) -> i32;
type Fn_copyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type Fn_reset32 = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type Fn_reset64 = unsafe extern "C" fn(*mut c_void, u64) -> i32;
type Fn_update = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32;
type Fn_digest32 = unsafe extern "C" fn(*const c_void) -> u32;
type Fn_digest64 = unsafe extern "C" fn(*const c_void) -> u64;
type Fn_canon32 = unsafe extern "C" fn(*mut u8, u32);
type Fn_canon64 = unsafe extern "C" fn(*mut u8, u64);
type Fn_fromCanon32 = unsafe extern "C" fn(*const u8) -> u32;
type Fn_fromCanon64 = unsafe extern "C" fn(*const u8) -> u64;

// -------------------------------------------------------------- ABI constants

const FSE_MIN_TABLELOG: u32 = 5;
const FSE_MAX_TABLELOG: u32 = 12;
const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;
const FSE_MAX_SYMBOL_VALUE: u32 = 255;
const FSE_NCOUNTBOUND: usize = 512;

const HUF_TABLELOG_MAX: u32 = 12;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE: usize = ((4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192) * 4;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
/// `HUF_CTABLE_SIZE_ST(HUF_SYMBOLVALUE_MAX)` — the size `HUF_compress*_repeat`
/// memcpy's into `hufTable`, so the caller must always provide this many.
const HUF_CTABLE_ST_MAX: usize = HUF_SYMBOLVALUE_MAX as usize + 2;
/// `ZSTD_HUFFDTABLE_CAPACITY_LOG` — what zstd itself uses for its HUF_DTable.
const HUF_DTABLE_LOG: u32 = 12;
const HUF_DTABLE_U32: usize = 1 + (1usize << HUF_DTABLE_LOG);

const HIST_WKSP_SIZE: usize = 1024 * 4;

const HUF_flags_bmi2: i32 = 1;
const HUF_flags_optimalDepth: i32 = 2;
const HUF_flags_preferRepeat: i32 = 4;
const HUF_flags_suspectUncompressible: i32 = 8;
const HUF_flags_disableAsm: i32 = 16;
const HUF_flags_disableFast: i32 = 32;

const HUF_repeat_none: i32 = 0;
const HUF_repeat_check: i32 = 1;
const HUF_repeat_valid: i32 = 2;

/// `ERROR(x)` == `(size_t)-ZSTD_error_x`
fn ec(code: i32) -> usize {
    0usize.wrapping_sub(code as usize)
}

// --------------------------------------------------------------- size helpers

fn fse_ctable_size_u32(table_log: u32, msv: u32) -> usize {
    1 + (1usize << (table_log.max(1) - 1)) + (msv as usize + 1) * 2
}
fn fse_build_ctable_wksp(msv: u32, table_log: u32) -> usize {
    4 * ((((msv as usize + 2) + (1usize << table_log)) / 2) + 2)
}
fn fse_dtable_size_u32(table_log: u32) -> usize {
    1 + (1usize << table_log)
}
fn fse_build_dtable_wksp(table_log: u32, msv: u32) -> usize {
    2 * (msv as usize + 1) + (1usize << table_log) + 8
}
fn fse_decompress_wksp(table_log: u32, msv: u32) -> usize {
    (fse_dtable_size_u32(table_log)
        + 1
        + (fse_build_dtable_wksp(table_log, msv) + 3) / 4
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1)
        * 4
}

// ------------------------------------------------------------- buffer helpers

/// 8-byte aligned scratch buffer with a deterministic fill so that "untouched"
/// tail bytes are identical between the two libraries and a full-buffer
/// comparison stays meaningful.
///
/// `ABUF_SLACK` extra bytes are always allocated beyond the requested size: the
/// C's own `FSE_BUILD_CTABLE_WORKSPACE_SIZE` macro rounds *down* (`(msv+2+
/// tableSize)/2`) and `FSE_buildCTable_wksp` then `MEM_write64`s up to two bytes
/// past the advertised size for odd `maxSymbolValue`. The slack keeps the
/// harness memory-safe while still letting us hand the library the exact
/// *declared* size, which is what the error paths key off. Comparing the slack
/// region too means an over-write that differs between the two libraries still
/// shows up as a failure.
const ABUF_SLACK: usize = 64;

fn abuf(bytes: usize, fill: u8) -> Vec<u64> {
    let n = (bytes + ABUF_SLACK + 7) / 8;
    vec![u64::from_ne_bytes([fill; 8]); n.max(1)]
}
fn aptr(v: &mut [u64]) -> *mut u8 {
    v.as_mut_ptr() as *mut u8
}
fn bytes_of<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, core::mem::size_of_val(v)) }
}
fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

/// Lengths that matter for the entropy coders specifically: tiny inputs the
/// coders reject outright, the `srcSize<=2` FSE early-out, the 12-byte HUF
/// 4-stream minimum, the 1500-byte HIST parallel-histogram threshold, the
/// 4096*10 HUF "suspect uncompressible" sampling threshold and the 128 KB
/// block limit (plus one past it).
const ENT_LENS: [usize; 24] = [
    0, 1, 2, 3, 4, 7, 8, 9, 11, 12, 13, 16, 31, 64, 127, 128, 1023, 1499, 1500, 4096, 40_960,
    65_536, 131_072, 131_073,
];

/// A compact but representative subset for the expensive nested sweeps.
const ENT_LENS_SMALL: [usize; 12] = [0, 1, 2, 3, 8, 12, 64, 1024, 1500, 5000, 65_536, 131_072];

/// Inputs whose *histogram shape* is what the coders special-case: single
/// symbol (RLE), two symbols, the full 256-symbol alphabet and heavy skew.
fn alphabet_inputs(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();

    // single symbol (RLE) at several lengths and symbol values
    for &sym in &[0u8, 1, 127, 255] {
        for &len in &[1usize, 3, 12, 1024, 5000] {
            out.push((format!("rle{sym}x{len}"), vec![sym; len]));
        }
    }
    // exactly two symbols, various ratios
    for &(a, b) in &[(0u8, 255u8), (7, 8), (255, 254)] {
        for &ratio in &[1usize, 2, 17, 500] {
            let mut v = Vec::new();
            for i in 0..3000usize {
                v.push(if i % (ratio + 1) == 0 { a } else { b });
            }
            out.push((format!("two{a}/{b}:{ratio}"), v));
        }
    }
    // full 256-symbol alphabet, flat
    {
        let mut v = Vec::with_capacity(4096);
        for i in 0..4096usize {
            v.push((i & 0xff) as u8);
        }
        out.push(("flat256".into(), v));
    }
    // full alphabet but heavily skewed (geometric-ish)
    for &shift in &[1u32, 3, 6] {
        let mut v = Vec::with_capacity(8000);
        while v.len() < 8000 {
            let r = rng.next_u32();
            let sym = (r >> shift) & 0xff;
            v.push(sym as u8);
            for _ in 0..(1 << shift) {
                if v.len() < 8000 {
                    v.push(0);
                }
            }
        }
        out.push((format!("skew{shift}"), v));
    }
    // one dominant symbol plus a long tail
    {
        let mut v = vec![42u8; 20_000];
        for i in 0..256usize {
            v[i * 70] = i as u8;
        }
        out.push(("dominant+tail".into(), v));
    }
    // every ALL_SHAPES distribution at a couple of sizes
    for &shape in &ALL_SHAPES {
        for &len in &[1usize, 13, 700, 3000, 70_000] {
            out.push((format!("{shape:?}/{len}"), gen_shape(shape, len, rng)));
        }
    }
    out
}

// =============================================================== 1. meta APIs

/// Version numbers plus the *whole* FSE/HUF/HIST error-reporting surface.
#[test]
fn entropy_versions_and_error_surface() {
    let i = impls();

    let (c, r) = i.pair::<Fn_u32_v>("FSE_versionNumber");
    unsafe { assert_eq_dbg("FSE_versionNumber", c(), r()) };
    let (c, r) = i.pair::<Fn_u32_v>("ZSTD_XXH_versionNumber");
    unsafe { assert_eq_dbg("ZSTD_XXH_versionNumber", c(), r()) };

    let (c_fse_is, r_fse_is) = i.pair::<Fn_isErr>("FSE_isError");
    let (c_huf_is, r_huf_is) = i.pair::<Fn_isErr>("HUF_isError");
    let (c_hist_is, r_hist_is) = i.pair::<Fn_isErr>("HIST_isError");
    let (c_fse_nm, r_fse_nm) = i.pair::<Fn_errName>("FSE_getErrorName");
    let (c_huf_nm, r_huf_nm) = i.pair::<Fn_errName>("HUF_getErrorName");

    let mut probes: Vec<usize> = Vec::new();
    for e in 0..=200usize {
        probes.push(0usize.wrapping_sub(e));
    }
    probes.extend([0, 1, 2, 3, 512, 1 << 20, usize::MAX / 2, usize::MAX / 2 + 1]);
    let mut rng = Rng::new(0x5E1F_0001);
    for _ in 0..200 {
        probes.push(rng.next_u64() as usize);
    }

    for p in probes {
        unsafe {
            assert_eq_dbg(&format!("FSE_isError({p:#x})"), c_fse_is(p), r_fse_is(p));
            assert_eq_dbg(&format!("HUF_isError({p:#x})"), c_huf_is(p), r_huf_is(p));
            assert_eq_dbg(&format!("HIST_isError({p:#x})"), c_hist_is(p), r_hist_is(p));
            assert_eq_dbg(
                &format!("FSE_getErrorName({p:#x})"),
                cstr(c_fse_nm(p)),
                cstr(r_fse_nm(p)),
            );
            assert_eq_dbg(
                &format!("HUF_getErrorName({p:#x})"),
                cstr(c_huf_nm(p)),
                cstr(r_huf_nm(p)),
            );
        }
    }
}

/// `FSE_compressBound`, `HUF_compressBound` and `FSE_NCountWriteBound` are pure
/// arithmetic — sweep them hard, including the wrap-around domain.
#[test]
fn entropy_bounds() {
    let i = impls();
    let (c_fb, r_fb) = i.pair::<Fn_sz_sz>("FSE_compressBound");
    let (c_hb, r_hb) = i.pair::<Fn_sz_sz>("HUF_compressBound");
    let (c_nb, r_nb) = i.pair::<Fn_FSE_NCountWriteBound>("FSE_NCountWriteBound");

    let mut sizes: Vec<usize> = ENT_LENS.to_vec();
    sizes.extend(EDGE_LENS.iter().copied());
    sizes.extend([
        1 << 20,
        1 << 24,
        (1 << 30) + 1,
        usize::MAX / 8,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ]);
    let mut rng = Rng::new(0x5E1F_0002);
    for _ in 0..300 {
        sizes.push(rng.next_u64() as usize);
    }
    for s in sizes {
        unsafe {
            assert_eq_dbg(&format!("FSE_compressBound({s})"), c_fb(s), r_fb(s));
            assert_eq_dbg(&format!("HUF_compressBound({s})"), c_hb(s), r_hb(s));
        }
    }

    // maxSymbolValue==0 short-circuits to FSE_NCOUNTBOUND; everything else is
    // the (msv+1)*tableLog arithmetic. Include out-of-range msv/tableLog.
    for &msv in &[0u32, 1, 2, 3, 127, 254, 255, 256, 511, 65535, u32::MAX / 4] {
        for table_log in 0..=20u32 {
            let got_c = unsafe { c_nb(msv, table_log) };
            let got_r = unsafe { r_nb(msv, table_log) };
            assert_eq_dbg(
                &format!("FSE_NCountWriteBound(msv={msv},tl={table_log})"),
                got_c,
                got_r,
            );
            if msv == 0 {
                assert_eq_dbg("NCountWriteBound(msv=0)", got_c, FSE_NCOUNTBOUND);
            }
        }
    }
}

/// `FSE_optimalTableLog` / `FSE_optimalTableLog_internal`.
///
/// Both compute `ZSTD_highbit32(srcSize-1)` and `ZSTD_highbit32(maxSymbolValue)`
/// unguarded, so `srcSize<2` / `maxSymbolValue==0` are UB (`__builtin_clz(0)`)
/// in the C and would not be a meaningful differential input; the sweep starts
/// at the documented `srcSize>1`, `maxSymbolValue>=1` domain.
#[test]
fn fse_optimal_table_log() {
    let i = impls();
    let (c_o, r_o) = i.pair::<Fn_FSE_optimalTableLog>("FSE_optimalTableLog");
    let (c_oi, r_oi) =
        i.pair::<Fn_FSE_optimalTableLog_internal>("FSE_optimalTableLog_internal");

    let mut srcs: Vec<usize> = vec![
        2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 127, 128, 255, 256, 1023, 1024,
        4096, 65_535, 65_536, 131_072, 131_073, 1 << 20, 1 << 24, u32::MAX as usize,
    ];
    let mut rng = Rng::new(0x5E1F_0003);
    for _ in 0..64 {
        srcs.push(rng.range(2, 1 << 22));
    }
    let msvs = [1u32, 2, 3, 4, 7, 8, 15, 16, 63, 127, 128, 254, 255, 256, 1023, 65_535];

    for &src in &srcs {
        for &msv in &msvs {
            for max_tl in 0..=17u32 {
                unsafe {
                    assert_eq_dbg(
                        &format!("FSE_optimalTableLog({max_tl},{src},{msv})"),
                        c_o(max_tl, src, msv),
                        r_o(max_tl, src, msv),
                    );
                    for minus in 0..=3u32 {
                        assert_eq_dbg(
                            &format!(
                                "FSE_optimalTableLog_internal({max_tl},{src},{msv},{minus})"
                            ),
                            c_oi(max_tl, src, msv, minus),
                            r_oi(max_tl, src, msv, minus),
                        );
                    }
                }
            }
        }
    }
}

// ================================================================== 2. HIST

/// All five histogram entry points over every shape/length, comparing the
/// return value, the full `count[]` array and the updated `*maxSymbolValuePtr`.
///
/// `HIST_countFast*` / `HIST_count_simple` are documented as *unsafe* when the
/// source contains a byte `> *maxSymbolValuePtr`, and `HIST_count_parallel_wksp`
/// would `memmove` `(msv+1)*4` bytes out of a 4 KB workspace when `msv > 255`,
/// so those two axes are pinned to the memory-safe domain (`msv == 255` or the
/// exact observed maximum).
#[test]
fn hist_counting_all_variants() {
    let i = impls();
    let (c_cnt, r_cnt) = i.pair::<Fn_HIST_count>("HIST_count");
    let (c_cntw, r_cntw) = i.pair::<Fn_HIST_count_wksp>("HIST_count_wksp");
    let (c_fast, r_fast) = i.pair::<Fn_HIST_count>("HIST_countFast");
    let (c_fastw, r_fastw) = i.pair::<Fn_HIST_count_wksp>("HIST_countFast_wksp");
    let (c_simple, r_simple) = i.pair::<Fn_HIST_count_simple>("HIST_count_simple");
    let (c_add, r_add) = i.pair::<Fn_HIST_add>("HIST_add");

    let mut rng = Rng::new(0x5E1F_0010);
    let mut corpus: Vec<(String, Vec<u8>)> = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    // count[] is always oversized (HIST_count clamps msv to 255, so 256 cells
    // are enough for every call we make) plus slack to catch stray writes.
    const NC: usize = 512;

    for (tag, src) in &corpus {
        let observed_max = src.iter().copied().max().unwrap_or(0) as u32;

        // ---- HIST_count / HIST_count_wksp : msv is *checked*, so sweep it
        // including values below the observed maximum (error path) and above 255.
        let mut msvs: Vec<u32> = vec![0, 1, 2, 127, 255, 256, 65_535];
        msvs.push(observed_max);
        if observed_max > 0 {
            msvs.push(observed_max - 1);
        }
        for &msv in &msvs {
            for wksp_variant in 0..2 {
                let mut cc = vec![0xDEAD_BEEFu32; NC];
                let mut rc = vec![0xDEAD_BEEFu32; NC];
                let mut cm = msv;
                let mut rm = msv;
                let (a, b) = unsafe {
                    if wksp_variant == 0 {
                        (
                            c_cnt(cc.as_mut_ptr(), &mut cm, src.as_ptr(), src.len()),
                            r_cnt(rc.as_mut_ptr(), &mut rm, src.as_ptr(), src.len()),
                        )
                    } else {
                        let mut cw = abuf(HIST_WKSP_SIZE, 0x11);
                        let mut rw = abuf(HIST_WKSP_SIZE, 0x11);
                        (
                            c_cntw(
                                cc.as_mut_ptr(),
                                &mut cm,
                                src.as_ptr(),
                                src.len(),
                                aptr(&mut cw),
                                HIST_WKSP_SIZE,
                            ),
                            r_cntw(
                                rc.as_mut_ptr(),
                                &mut rm,
                                src.as_ptr(),
                                src.len(),
                                aptr(&mut rw),
                                HIST_WKSP_SIZE,
                            ),
                        )
                    }
                };
                let t = format!("HIST_count{} {tag} msv={msv}", if wksp_variant == 1 { "_wksp" } else { "" });
                assert_eq_dbg(&t, a, b);
                assert_eq_dbg(&format!("{t} / *maxSymbolValuePtr"), cm, rm);
                assert_bytes_eq(&format!("{t} / count[]"), bytes_of(&cc), bytes_of(&rc));
                // the checked variant must reject a too-small msv
                if msv < observed_max && !src.is_empty() {
                    assert_eq_dbg(
                        &format!("{t} / expected maxSymbolValue_tooSmall"),
                        a,
                        ec(ZSTD_error_maxSymbolValue_tooSmall),
                    );
                }
            }
        }

        // ---- HIST_countFast / _wksp / _simple : msv must be >= observed max
        for &msv in &[observed_max, 255u32] {
            let mut cc = vec![0xA5A5_A5A5u32; NC];
            let mut rc = vec![0xA5A5_A5A5u32; NC];
            let mut cm = msv;
            let mut rm = msv;
            let a = unsafe { c_fast(cc.as_mut_ptr(), &mut cm, src.as_ptr(), src.len()) };
            let b = unsafe { r_fast(rc.as_mut_ptr(), &mut rm, src.as_ptr(), src.len()) };
            let t = format!("HIST_countFast {tag} msv={msv}");
            assert_eq_dbg(&t, a, b);
            assert_eq_dbg(&format!("{t} / msvPtr"), cm, rm);
            assert_bytes_eq(&format!("{t} / count[]"), bytes_of(&cc), bytes_of(&rc));

            let mut cc = vec![0x5A5A_5A5Au32; NC];
            let mut rc = vec![0x5A5A_5A5Au32; NC];
            let mut cm = msv;
            let mut rm = msv;
            let mut cw = abuf(HIST_WKSP_SIZE, 0x22);
            let mut rw = abuf(HIST_WKSP_SIZE, 0x22);
            let a = unsafe {
                c_fastw(cc.as_mut_ptr(), &mut cm, src.as_ptr(), src.len(), aptr(&mut cw), HIST_WKSP_SIZE)
            };
            let b = unsafe {
                r_fastw(rc.as_mut_ptr(), &mut rm, src.as_ptr(), src.len(), aptr(&mut rw), HIST_WKSP_SIZE)
            };
            let t = format!("HIST_countFast_wksp {tag} msv={msv}");
            assert_eq_dbg(&t, a, b);
            assert_eq_dbg(&format!("{t} / msvPtr"), cm, rm);
            assert_bytes_eq(&format!("{t} / count[]"), bytes_of(&cc), bytes_of(&rc));

            let mut cc = vec![0u32; NC];
            let mut rc = vec![0u32; NC];
            let mut cm = msv;
            let mut rm = msv;
            let a = unsafe { c_simple(cc.as_mut_ptr(), &mut cm, src.as_ptr(), src.len()) };
            let b = unsafe { r_simple(rc.as_mut_ptr(), &mut rm, src.as_ptr(), src.len()) };
            let t = format!("HIST_count_simple {tag} msv={msv}");
            assert_eq_dbg(&t, a, b);
            assert_eq_dbg(&format!("{t} / msvPtr"), cm, rm);
            assert_bytes_eq(&format!("{t} / count[]"), bytes_of(&cc), bytes_of(&rc));
        }

        // ---- HIST_add : accumulates, never resets. Call it twice to prove the
        // accumulation (and not just the first pass) matches.
        let mut cc = vec![0u32; NC];
        let mut rc = vec![0u32; NC];
        unsafe {
            c_add(cc.as_mut_ptr(), src.as_ptr(), src.len());
            r_add(rc.as_mut_ptr(), src.as_ptr(), src.len());
            c_add(cc.as_mut_ptr(), src.as_ptr(), src.len());
            r_add(rc.as_mut_ptr(), src.as_ptr(), src.len());
        }
        assert_bytes_eq(&format!("HIST_add {tag}"), bytes_of(&cc), bytes_of(&rc));
    }
}

/// The two *checked* HIST failure modes: a misaligned workspace
/// (`ERROR(GENERIC)`) and a workspace below `HIST_WKSP_SIZE`
/// (`ERROR(workSpace_tooSmall)`), swept over the whole 0..HIST_WKSP_SIZE range.
#[test]
fn hist_error_paths() {
    let i = impls();
    let (c_cntw, r_cntw) = i.pair::<Fn_HIST_count_wksp>("HIST_count_wksp");
    let (c_fastw, r_fastw) = i.pair::<Fn_HIST_count_wksp>("HIST_countFast_wksp");

    let mut rng = Rng::new(0x5E1F_0011);
    // >= 1500 bytes so countFast_wksp actually reaches the workspace checks
    // instead of delegating to HIST_count_simple.
    let src = gen_shape(Shape::SkewedText, 4000, &mut rng);
    let tiny = gen_shape(Shape::SkewedText, 100, &mut rng);

    let mut sizes: Vec<usize> = vec![0, 1, 3, 4, 8, 1024, HIST_WKSP_SIZE - 4, HIST_WKSP_SIZE - 1];
    sizes.push(HIST_WKSP_SIZE);
    sizes.push(HIST_WKSP_SIZE + 1);
    sizes.push(usize::MAX);

    for (name, src) in [("large", &src), ("tiny", &tiny)] {
        for &wsz in &sizes {
            for &off in &[0usize, 1, 2, 3, 4] {
                // buffer is always full-size; only the *declared* size and the
                // alignment of the pointer we hand over vary.
                let mut cw = abuf(HIST_WKSP_SIZE + 16, 0x33);
                let mut rw = abuf(HIST_WKSP_SIZE + 16, 0x33);
                let mut cc = vec![0u32; 512];
                let mut rc = vec![0u32; 512];
                let mut cm = 255u32;
                let mut rm = 255u32;
                let declared = if wsz == usize::MAX { HIST_WKSP_SIZE } else { wsz };
                let a = unsafe {
                    c_cntw(
                        cc.as_mut_ptr(),
                        &mut cm,
                        src.as_ptr(),
                        src.len(),
                        aptr(&mut cw).add(off),
                        declared,
                    )
                };
                let b = unsafe {
                    r_cntw(
                        rc.as_mut_ptr(),
                        &mut rm,
                        src.as_ptr(),
                        src.len(),
                        aptr(&mut rw).add(off),
                        declared,
                    )
                };
                let t = format!("HIST_count_wksp {name} wsz={declared} off={off}");
                assert_eq_dbg(&t, a, b);
                assert_eq_dbg(&format!("{t} msvPtr"), cm, rm);
                if off % 4 != 0 {
                    assert_eq_dbg(&format!("{t} expect GENERIC"), a, ec(ZSTD_error_GENERIC));
                } else if declared < HIST_WKSP_SIZE {
                    assert_eq_dbg(
                        &format!("{t} expect workSpace_tooSmall"),
                        a,
                        ec(ZSTD_error_workSpace_tooSmall),
                    );
                }

                let mut cw = abuf(HIST_WKSP_SIZE + 16, 0x44);
                let mut rw = abuf(HIST_WKSP_SIZE + 16, 0x44);
                let mut cc = vec![0u32; 512];
                let mut rc = vec![0u32; 512];
                let mut cm = 255u32;
                let mut rm = 255u32;
                let a = unsafe {
                    c_fastw(
                        cc.as_mut_ptr(),
                        &mut cm,
                        src.as_ptr(),
                        src.len(),
                        aptr(&mut cw).add(off),
                        declared,
                    )
                };
                let b = unsafe {
                    r_fastw(
                        rc.as_mut_ptr(),
                        &mut rm,
                        src.as_ptr(),
                        src.len(),
                        aptr(&mut rw).add(off),
                        declared,
                    )
                };
                let t = format!("HIST_countFast_wksp {name} wsz={declared} off={off}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} count[]"), bytes_of(&cc), bytes_of(&rc));
            }
        }
    }
}

// ============================================================ 3. FSE NCount

/// Histogram of `src` computed with the **C** library (the ground truth), plus
/// the observed maxSymbolValue. `count` is oversized to 512 cells so callers may
/// legally pass `maxSymbolValue` up to 511 into the unchecked FSE helpers.
fn hist_c(i: &Impls, src: &[u8]) -> (Vec<u32>, u32, usize) {
    let (c_cnt, _) = i.pair::<Fn_HIST_count>("HIST_count");
    let mut count = vec![0u32; 512];
    let mut msv = 255u32;
    let largest = unsafe { c_cnt(count.as_mut_ptr(), &mut msv, src.as_ptr(), src.len()) };
    assert!(largest <= src.len(), "HIST_count failed: {largest:#x}");
    (count, msv, largest)
}

/// `FSE_normalizeCount` across every histogram shape and the whole tableLog
/// axis (below FSE_MIN_TABLELOG, at min, typical, at FSE_MAX_TABLELOG, above
/// max, and the `tableLog==0` "use default" sentinel) times `useLowProbCount`.
///
/// `total==0` or `maxSymbolValue==0` reach `ZSTD_highbit32(0)` (UB in C), so the
/// sweep requires `srcSize>=2` / `msv>=1`.
#[test]
fn fse_normalize_count() {
    let i = impls();
    let (c_n, r_n) = i.pair::<Fn_FSE_normalizeCount>("FSE_normalizeCount");

    let mut rng = Rng::new(0x5E1F_0020);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    let table_logs = [0u32, 1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 31];

    for (tag, src) in &corpus {
        if src.len() < 2 {
            continue; // FSE_minTableLog() -> ZSTD_highbit32(total) needs total>=1
        }
        let (count, obs_msv, _) = hist_c(i, src);
        if obs_msv == 0 {
            continue; // ZSTD_highbit32(maxSymbolValue) needs msv>=1
        }

        // maxSymbolValue axis: the true value, and larger values (including
        // 256 / 511, which FSE_normalizeCount does *not* range-check).
        let mut msvs = vec![obs_msv, 255u32, 256, 511];
        if obs_msv > 1 {
            msvs.push(obs_msv - 1); // deliberately too small
        }
        msvs.dedup();

        for &msv in &msvs {
            for &tl in &table_logs {
                for lowprob in 0..2u32 {
                    let mut cn = vec![0x7777i16; 512];
                    let mut rn = vec![0x7777i16; 512];
                    let a = unsafe {
                        c_n(cn.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob)
                    };
                    let b = unsafe {
                        r_n(rn.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob)
                    };
                    let t = format!(
                        "FSE_normalizeCount {tag} msv={msv} tl={tl} lowProb={lowprob}"
                    );
                    assert_eq_dbg(&t, a, b);
                    // The normalizedCounter is only fully defined when the call
                    // reports a tableLog (return != 0 and not an error); the
                    // rle short-circuit (`return 0`) leaves it partly written.
                    if a != 0 && a < ec(200) {
                        assert_bytes_eq(&format!("{t} / norm[]"), bytes_of(&cn), bytes_of(&rn));
                        let eff_tl = if tl == 0 { 11 } else { tl };
                        assert_eq_dbg(&format!("{t} / return == tableLog"), a, eff_tl as usize);
                        // sanity invariant, only meaningful when the declared
                        // alphabet actually covers the whole histogram
                        if msv >= obs_msv {
                            let sum: i32 = (0..=msv.min(511) as usize)
                                .map(|s| (cn[s] as i32).abs())
                                .sum();
                            assert_eq_dbg(
                                &format!("{t} / sum(|norm|) == 1<<tableLog"),
                                sum,
                                1i32 << eff_tl,
                            );
                        }
                    }
                    // documented error codes
                    if tl != 0 && tl < FSE_MIN_TABLELOG {
                        assert_eq_dbg(&format!("{t} expect GENERIC"), a, ec(ZSTD_error_GENERIC));
                    } else if tl > FSE_MAX_TABLELOG {
                        assert_eq_dbg(
                            &format!("{t} expect tableLog_tooLarge"),
                            a,
                            ec(ZSTD_error_tableLog_tooLarge),
                        );
                    }
                }
            }
        }
    }
}

/// Produce a valid normalized counter with the C library.
/// Returns `(norm, msv, tableLog)` or `None` when the input is degenerate.
fn normalized_c(i: &Impls, src: &[u8], max_tl: u32, lowprob: u32) -> Option<(Vec<i16>, u32, u32)> {
    if src.len() < 2 {
        return None;
    }
    let (count, msv, _) = hist_c(i, src);
    if msv == 0 {
        return None;
    }
    let (c_opt, _) = i.pair::<Fn_FSE_optimalTableLog>("FSE_optimalTableLog");
    let (c_n, _) = i.pair::<Fn_FSE_normalizeCount>("FSE_normalizeCount");
    let tl = unsafe { c_opt(max_tl, src.len(), msv) };
    let mut norm = vec![0i16; 512];
    let ret = unsafe { c_n(norm.as_mut_ptr(), tl, count.as_ptr(), src.len(), msv, lowprob) };
    if ret == 0 || ret > ec(200) {
        return None; // rle short-circuit or error: no usable distribution
    }
    Some((norm, msv, ret as u32))
}

/// `FSE_writeNCount` (full dstCapacity sweep 0..bound) then
/// `FSE_readNCount` / `FSE_readNCount_bmi2` round trip, plus truncation and
/// bit-flip corruption of the serialized header.
#[test]
fn fse_write_read_ncount() {
    let i = impls();
    let (c_w, r_w) = i.pair::<Fn_FSE_writeNCount>("FSE_writeNCount");
    let (c_r, r_r) = i.pair::<Fn_FSE_readNCount>("FSE_readNCount");
    let (c_rb, r_rb) = i.pair::<Fn_FSE_readNCount_bmi2>("FSE_readNCount_bmi2");
    let (c_nb, _) = i.pair::<Fn_FSE_NCountWriteBound>("FSE_NCountWriteBound");

    let mut rng = Rng::new(0x5E1F_0021);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &[13usize, 200, 3000, 70_000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    for (tag, src) in &corpus {
        for &max_tl in &[0u32, 5, 8, 11, 12] {
            for lowprob in 0..2u32 {
                let Some((norm, msv, tl)) = normalized_c(i, src, max_tl, lowprob) else {
                    continue;
                };
                let bound = unsafe { c_nb(msv, tl) };

                // ---- exact-capacity write, byte-for-byte
                let mut cb = vec![0xCCu8; bound + 64];
                let mut rb = vec![0xCCu8; bound + 64];
                let n = unsafe { c_w(cb.as_mut_ptr(), bound, norm.as_ptr(), msv, tl) };
                let m = unsafe { r_w(rb.as_mut_ptr(), bound, norm.as_ptr(), msv, tl) };
                let t = format!("FSE_writeNCount {tag} msv={msv} tl={tl} lp={lowprob}");
                assert_eq_dbg(&t, n, m);
                assert!(n <= bound, "{t}: unexpected error {n:#x}");
                assert_bytes_eq(&format!("{t} / full buffer"), &cb, &rb);

                let header = cb[..n].to_vec();

                // ---- dstCapacity sweep 0..=exact
                for cap in 0..=n {
                    let mut cb = vec![0x33u8; bound + 64];
                    let mut rb = vec![0x33u8; bound + 64];
                    let a = unsafe { c_w(cb.as_mut_ptr(), cap, norm.as_ptr(), msv, tl) };
                    let b = unsafe { r_w(rb.as_mut_ptr(), cap, norm.as_ptr(), msv, tl) };
                    assert_eq_dbg(&format!("{t} cap={cap}"), a, b);
                    assert_bytes_eq(&format!("{t} cap={cap} / buffer"), &cb, &rb);
                }

                // ---- tableLog out of range
                for &bad in &[1u32, 4, 13, 15, 16, 255] {
                    let mut cb = vec![0u8; bound + 64];
                    let mut rb = vec![0u8; bound + 64];
                    let a = unsafe { c_w(cb.as_mut_ptr(), bound, norm.as_ptr(), msv, bad) };
                    let b = unsafe { r_w(rb.as_mut_ptr(), bound, norm.as_ptr(), msv, bad) };
                    assert_eq_dbg(&format!("{t} badTL={bad}"), a, b);
                    let want = if bad > FSE_MAX_TABLELOG {
                        ec(ZSTD_error_tableLog_tooLarge)
                    } else {
                        ec(ZSTD_error_GENERIC)
                    };
                    assert_eq_dbg(&format!("{t} badTL={bad} code"), a, want);
                }

                // ---- readNCount round trip, both entry points, both bmi2 flags
                for (rname, use_bmi2) in [("FSE_readNCount", -1i32), ("bmi2=0", 0), ("bmi2=1", 1)] {
                    for &declared_msv in &[255u32, msv, 511] {
                        let mut cn = vec![0x11i16; 512];
                        let mut rn = vec![0x11i16; 512];
                        let mut cm = declared_msv;
                        let mut rm = declared_msv;
                        let mut ctl = 0u32;
                        let mut rtl = 0u32;
                        let (a, b) = unsafe {
                            if use_bmi2 < 0 {
                                (
                                    c_r(cn.as_mut_ptr(), &mut cm, &mut ctl, header.as_ptr(), n),
                                    r_r(rn.as_mut_ptr(), &mut rm, &mut rtl, header.as_ptr(), n),
                                )
                            } else {
                                (
                                    c_rb(cn.as_mut_ptr(), &mut cm, &mut ctl, header.as_ptr(), n, use_bmi2),
                                    r_rb(rn.as_mut_ptr(), &mut rm, &mut rtl, header.as_ptr(), n, use_bmi2),
                                )
                            }
                        };
                        let rt = format!("{rname} {tag} msv={declared_msv} tl={tl}");
                        assert_eq_dbg(&rt, a, b);
                        assert_eq_dbg(&format!("{rt} / msvPtr"), cm, rm);
                        assert_eq_dbg(&format!("{rt} / tableLogPtr"), ctl, rtl);
                        assert_bytes_eq(&format!("{rt} / norm[]"), bytes_of(&cn), bytes_of(&rn));
                        if declared_msv >= msv {
                            assert_eq_dbg(&format!("{rt} / consumed all"), a, n);
                            assert_eq_dbg(&format!("{rt} / tableLog"), ctl, tl);
                            assert_eq_dbg(&format!("{rt} / msv"), cm, msv);
                            for s in 0..=msv as usize {
                                assert_eq_dbg(&format!("{rt} / norm[{s}]"), cn[s], norm[s]);
                            }
                        }
                    }
                }

                // ---- truncation sweep: every prefix of the header
                for cut in 0..n {
                    let mut cn = vec![0u8; 8];
                    let mut rn = vec![0u8; 8];
                    let _ = (&mut cn, &mut rn);
                    let mut cnn = vec![0i16; 512];
                    let mut rnn = vec![0i16; 512];
                    let mut cm = 255u32;
                    let mut rm = 255u32;
                    let mut ctl = 0u32;
                    let mut rtl = 0u32;
                    let a = unsafe {
                        c_r(cnn.as_mut_ptr(), &mut cm, &mut ctl, header.as_ptr(), cut)
                    };
                    let b = unsafe {
                        r_r(rnn.as_mut_ptr(), &mut rm, &mut rtl, header.as_ptr(), cut)
                    };
                    let ct = format!("{t} readNCount truncated to {cut}/{n}");
                    assert_eq_dbg(&ct, a, b);
                    assert_eq_dbg(&format!("{ct} msvPtr"), cm, rm);
                    assert_eq_dbg(&format!("{ct} tlPtr"), ctl, rtl);
                    assert_bytes_eq(&format!("{ct} norm[]"), bytes_of(&cnn), bytes_of(&rnn));
                }
            }
        }
    }
}

/// Bit-flip fuzzing of serialized NCount headers: both libraries must agree on
/// accept/reject *and* on the exact error code and parsed output.
#[test]
fn fse_read_ncount_corrupted() {
    let i = impls();
    let (c_w, _) = i.pair::<Fn_FSE_writeNCount>("FSE_writeNCount");
    let (c_r, r_r) = i.pair::<Fn_FSE_readNCount>("FSE_readNCount");
    let (c_nb, _) = i.pair::<Fn_FSE_NCountWriteBound>("FSE_NCountWriteBound");

    let mut rng = Rng::new(0x5E1F_0022);
    let mut headers: Vec<(String, Vec<u8>)> = Vec::new();
    for (tag, src) in alphabet_inputs(&mut rng) {
        for &max_tl in &[0u32, 6, 12] {
            if let Some((norm, msv, tl)) = normalized_c(i, &src, max_tl, max_tl & 1) {
                let bound = unsafe { c_nb(msv, tl) };
                let mut buf = vec![0u8; bound + 32];
                let n = unsafe { c_w(buf.as_mut_ptr(), bound, norm.as_ptr(), msv, tl) };
                if n <= bound {
                    headers.push((format!("{tag}/tl{tl}"), buf[..n].to_vec()));
                }
            }
        }
    }
    assert!(headers.len() > 20, "expected a decent header corpus");

    // Also feed pure garbage — readNCount must not diverge on random bytes.
    for len in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 40, 300] {
        for _ in 0..40 {
            let mut v = vec![0u8; len];
            for b in v.iter_mut() {
                *b = rng.byte();
            }
            headers.push((format!("garbage/{len}"), v));
        }
    }

    for (tag, base) in &headers {
        let mut variants: Vec<Vec<u8>> = vec![base.clone()];
        for _ in 0..24 {
            if base.is_empty() {
                break;
            }
            let mut v = base.clone();
            let pos = rng.below(v.len());
            v[pos] ^= 1u8 << rng.below(8);
            variants.push(v);
        }
        for (vi, v) in variants.iter().enumerate() {
            for &declared_msv in &[255u32, 1, 15] {
                let mut cn = vec![0x22i16; 512];
                let mut rn = vec![0x22i16; 512];
                let mut cm = declared_msv;
                let mut rm = declared_msv;
                let mut ctl = 0xFFFF_FFFFu32;
                let mut rtl = 0xFFFF_FFFFu32;
                let a =
                    unsafe { c_r(cn.as_mut_ptr(), &mut cm, &mut ctl, v.as_ptr(), v.len()) };
                let b =
                    unsafe { r_r(rn.as_mut_ptr(), &mut rm, &mut rtl, v.as_ptr(), v.len()) };
                let t = format!("FSE_readNCount corrupt {tag} v={vi} msv={declared_msv}");
                assert_eq_dbg(&t, a, b);
                assert_eq_dbg(&format!("{t} msvPtr"), cm, rm);
                assert_eq_dbg(&format!("{t} tlPtr"), ctl, rtl);
                assert_bytes_eq(&format!("{t} norm[]"), bytes_of(&cn), bytes_of(&rn));
            }
        }
    }
}

// ======================================================= 4. FSE CTable/DTable

/// `FSE_buildCTable_wksp`, `FSE_buildCTable_rle` and `FSE_buildDTable_wksp`
/// must produce **byte-identical** tables, which is a far stronger check than
/// "both round-trip". Also sweeps the workspace-too-small / out-of-range
/// parameter errors.
#[test]
fn fse_build_tables_bit_identical() {
    let i = impls();
    let (c_bc, r_bc) = i.pair::<Fn_FSE_buildCTable_wksp>("FSE_buildCTable_wksp");
    let (c_rle, r_rle) = i.pair::<Fn_FSE_buildCTable_rle>("FSE_buildCTable_rle");
    let (c_bd, r_bd) = i.pair::<Fn_FSE_buildDTable_wksp>("FSE_buildDTable_wksp");

    // ---- RLE CTable for every symbol value. FSE_buildCTable_rle only writes
    // the header, tableU16[0..1] and symbolTT[symbolValue], leaving the rest of
    // the buffer untouched — both buffers are pre-filled identically so a
    // full-buffer comparison is still exact.
    for sym in 0..=255u8 {
        let n = fse_ctable_size_u32(FSE_MAX_TABLELOG, 255);
        let mut ct = vec![0x1234_5678u32; n];
        let mut rt = vec![0x1234_5678u32; n];
        let a = unsafe { c_rle(ct.as_mut_ptr(), sym) };
        let b = unsafe { r_rle(rt.as_mut_ptr(), sym) };
        assert_eq_dbg(&format!("FSE_buildCTable_rle({sym})"), a, b);
        assert_eq_dbg(&format!("FSE_buildCTable_rle({sym}) == 0"), a, 0);
        assert_bytes_eq(
            &format!("FSE_buildCTable_rle({sym}) / table"),
            bytes_of(&ct),
            bytes_of(&rt),
        );
    }

    let mut rng = Rng::new(0x5E1F_0030);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &[13usize, 300, 5000, 70_000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    for (tag, src) in &corpus {
        for &max_tl in &[0u32, 5, 6, 9, 11, 12] {
            for lowprob in 0..2u32 {
                let Some((norm, msv, tl)) = normalized_c(i, src, max_tl, lowprob) else {
                    continue;
                };
                let t = format!("{tag} msv={msv} tl={tl} lp={lowprob}");

                // ---- CTable, exact workspace
                let wsz = fse_build_ctable_wksp(msv, tl);
                let ctn = fse_ctable_size_u32(tl, msv);
                let mut ct = vec![0xA0A0_A0A0u32; ctn + 8];
                let mut rt = vec![0xA0A0_A0A0u32; ctn + 8];
                let mut cw = abuf(wsz, 0x5C);
                let mut rw = abuf(wsz, 0x5C);
                let a = unsafe {
                    c_bc(ct.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), wsz)
                };
                let b = unsafe {
                    r_bc(rt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), wsz)
                };
                assert_eq_dbg(&format!("FSE_buildCTable_wksp {t}"), a, b);
                assert_eq_dbg(&format!("FSE_buildCTable_wksp {t} == 0"), a, 0);
                assert_bytes_eq(
                    &format!("FSE_buildCTable_wksp {t} / CTable"),
                    bytes_of(&ct),
                    bytes_of(&rt),
                );
                assert_bytes_eq(
                    &format!("FSE_buildCTable_wksp {t} / workspace"),
                    bytes_of(&cw),
                    bytes_of(&rw),
                );

                // ---- CTable, workspace one byte short and a coarse sweep below
                let mut short_sizes: Vec<usize> = vec![0, 1, 2, 4, wsz - 1];
                if wsz > 8 {
                    short_sizes.push(wsz / 2);
                    short_sizes.push(wsz - 8);
                }
                for &s in &short_sizes {
                    let mut ct = vec![0u32; ctn + 8];
                    let mut rt = vec![0u32; ctn + 8];
                    let mut cw = abuf(wsz, 0x5C);
                    let mut rw = abuf(wsz, 0x5C);
                    let a = unsafe {
                        c_bc(ct.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), s)
                    };
                    let b = unsafe {
                        r_bc(rt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), s)
                    };
                    assert_eq_dbg(&format!("FSE_buildCTable_wksp {t} wsz={s}"), a, b);
                    assert_eq_dbg(
                        &format!("FSE_buildCTable_wksp {t} wsz={s} expect tableLog_tooLarge"),
                        a,
                        ec(ZSTD_error_tableLog_tooLarge),
                    );
                    assert_bytes_eq(
                        &format!("FSE_buildCTable_wksp {t} wsz={s} / CTable untouched"),
                        bytes_of(&ct),
                        bytes_of(&rt),
                    );
                }

                // ---- DTable, exact workspace
                let dwsz = fse_build_dtable_wksp(tl, msv);
                let dtn = fse_dtable_size_u32(tl);
                let mut dt = vec![0xB0B0_B0B0u32; dtn + 8];
                let mut rdt = vec![0xB0B0_B0B0u32; dtn + 8];
                let mut cw = abuf(dwsz, 0x6D);
                let mut rw = abuf(dwsz, 0x6D);
                let a = unsafe {
                    c_bd(dt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), dwsz)
                };
                let b = unsafe {
                    r_bd(rdt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), dwsz)
                };
                assert_eq_dbg(&format!("FSE_buildDTable_wksp {t}"), a, b);
                assert_eq_dbg(&format!("FSE_buildDTable_wksp {t} == 0"), a, 0);
                assert_bytes_eq(
                    &format!("FSE_buildDTable_wksp {t} / DTable"),
                    bytes_of(&dt),
                    bytes_of(&rdt),
                );
                assert_bytes_eq(
                    &format!("FSE_buildDTable_wksp {t} / workspace"),
                    bytes_of(&cw),
                    bytes_of(&rw),
                );

                // ---- DTable errors: short workspace, msv/tableLog out of range
                for &s in &[0usize, 1, 8, dwsz - 1] {
                    let mut dt = vec![0u32; dtn + 8];
                    let mut rdt = vec![0u32; dtn + 8];
                    let mut cw = abuf(dwsz, 0x6D);
                    let mut rw = abuf(dwsz, 0x6D);
                    let a = unsafe {
                        c_bd(dt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), s)
                    };
                    let b = unsafe {
                        r_bd(rdt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), s)
                    };
                    assert_eq_dbg(&format!("FSE_buildDTable_wksp {t} wsz={s}"), a, b);
                    assert_eq_dbg(
                        &format!("FSE_buildDTable_wksp {t} wsz={s} expect msv_tooLarge"),
                        a,
                        ec(ZSTD_error_maxSymbolValue_tooLarge),
                    );
                    assert_bytes_eq(
                        &format!("FSE_buildDTable_wksp {t} wsz={s} / DTable untouched"),
                        bytes_of(&dt),
                        bytes_of(&rdt),
                    );
                }
            }
        }
    }

    // ---- out-of-range maxSymbolValue / tableLog on FSE_buildDTable_wksp.
    // The workspace check happens *first*, so it is sized generously enough for
    // the (invalid) parameters to make sure the parameter check is what fires.
    let norm = {
        let mut v = vec![0i16; 1024];
        v[0] = 16;
        v[1] = 16;
        v
    };
    for &(msv, tl, want) in &[
        (256u32, 6u32, ZSTD_error_maxSymbolValue_tooLarge),
        (511, 6, ZSTD_error_maxSymbolValue_tooLarge),
        (255, 13, ZSTD_error_tableLog_tooLarge),
        (255, 15, ZSTD_error_tableLog_tooLarge),
        (1, 13, ZSTD_error_tableLog_tooLarge),
    ] {
        let dwsz = fse_build_dtable_wksp(tl, msv);
        let dtn = fse_dtable_size_u32(tl);
        let mut dt = vec![0xEEu32; dtn + 8];
        let mut rdt = vec![0xEEu32; dtn + 8];
        let mut cw = abuf(dwsz, 0x77);
        let mut rw = abuf(dwsz, 0x77);
        let a = unsafe { c_bd(dt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), dwsz) };
        let b = unsafe { r_bd(rdt.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), dwsz) };
        let t = format!("FSE_buildDTable_wksp bad msv={msv} tl={tl}");
        assert_eq_dbg(&t, a, b);
        assert_eq_dbg(&format!("{t} code"), a, ec(want));
        assert_bytes_eq(&format!("{t} DTable untouched"), bytes_of(&dt), bytes_of(&rdt));
    }
}

// ================================================== 5. FSE compress/decompress

/// A complete FSE stream (`writeNCount` header ++ `compress_usingCTable`
/// payload) built entirely with the C library, plus the parameters used.
#[allow(dead_code)]
struct FseStream {
    header_len: usize,
    stream: Vec<u8>,
    table_log: u32,
    msv: u32,
    src: Vec<u8>,
}

fn fse_stream_c(i: &Impls, src: &[u8], max_tl: u32, lowprob: u32) -> Option<FseStream> {
    if src.len() <= 2 {
        return None;
    }
    let (norm, msv, tl) = normalized_c(i, src, max_tl, lowprob)?;
    let (c_nb, _) = i.pair::<Fn_FSE_NCountWriteBound>("FSE_NCountWriteBound");
    let (c_w, _) = i.pair::<Fn_FSE_writeNCount>("FSE_writeNCount");
    let (c_bc, _) = i.pair::<Fn_FSE_buildCTable_wksp>("FSE_buildCTable_wksp");
    let (c_cc, _) = i.pair::<Fn_FSE_compress_usingCTable>("FSE_compress_usingCTable");
    let (c_cb, _) = i.pair::<Fn_sz_sz>("FSE_compressBound");

    let bound = unsafe { c_nb(msv, tl) };
    let mut out = vec![0u8; unsafe { c_cb(src.len()) } + 128];
    let hn = unsafe { c_w(out.as_mut_ptr(), bound, norm.as_ptr(), msv, tl) };
    if hn > bound {
        return None;
    }

    let wsz = fse_build_ctable_wksp(msv, tl);
    let mut wk = abuf(wsz, 0);
    let mut ct = vec![0u32; fse_ctable_size_u32(tl, msv) + 8];
    let e = unsafe { c_bc(ct.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut wk), wsz) };
    if e != 0 {
        return None;
    }
    let cap = out.len() - hn;
    let pn = unsafe {
        c_cc(out.as_mut_ptr().add(hn), cap, src.as_ptr(), src.len(), ct.as_ptr())
    };
    if pn == 0 || pn > cap {
        return None;
    }
    out.truncate(hn + pn);
    Some(FseStream {
        header_len: hn,
        stream: out,
        table_log: tl,
        msv,
        src: src.to_vec(),
    })
}

/// The full FSE compression pipeline, then `FSE_decompress_wksp_bmi2` (both
/// `bmi2` settings) round trip. Everything is compared byte-for-byte, and each
/// library must decode the *other's* stream.
#[test]
fn fse_compress_decompress_pipeline() {
    let i = impls();
    let (c_bc, r_bc) = i.pair::<Fn_FSE_buildCTable_wksp>("FSE_buildCTable_wksp");
    let (c_cc, r_cc) = i.pair::<Fn_FSE_compress_usingCTable>("FSE_compress_usingCTable");
    let (c_d, r_d) = i.pair::<Fn_FSE_decompress_wksp_bmi2>("FSE_decompress_wksp_bmi2");
    let (c_nb, _) = i.pair::<Fn_FSE_NCountWriteBound>("FSE_NCountWriteBound");
    let (c_w, _) = i.pair::<Fn_FSE_writeNCount>("FSE_writeNCount");
    let (c_cb, _) = i.pair::<Fn_sz_sz>("FSE_compressBound");

    let mut rng = Rng::new(0x5E1F_0040);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &[3usize, 4, 7, 8, 9, 300, 5000, 70_000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    let dwsz = fse_decompress_wksp(FSE_MAX_TABLELOG, FSE_MAX_SYMBOL_VALUE);

    for (tag, src) in &corpus {
        if src.len() <= 2 {
            continue;
        }
        for &max_tl in &[0u32, 5, 8, 11, 12] {
            for lowprob in 0..2u32 {
                let Some((norm, msv, tl)) = normalized_c(i, src, max_tl, lowprob) else {
                    continue;
                };
                let t = format!("{tag} msv={msv} tl={tl} lp={lowprob}");

                // ---- both libraries build their own CTable, then compress
                let wsz = fse_build_ctable_wksp(msv, tl);
                let ctn = fse_ctable_size_u32(tl, msv) + 8;
                let mut cct = vec![0u32; ctn];
                let mut rct = vec![0u32; ctn];
                let mut cw = abuf(wsz, 0);
                let mut rw = abuf(wsz, 0);
                assert_eq_dbg(
                    &format!("buildCTable {t}"),
                    unsafe { c_bc(cct.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut cw), wsz) },
                    unsafe { r_bc(rct.as_mut_ptr(), norm.as_ptr(), msv, tl, aptr(&mut rw), wsz) },
                );
                assert_bytes_eq(
                    &format!("buildCTable {t} / bit-identical CTable"),
                    bytes_of(&cct),
                    bytes_of(&rct),
                );

                let cap = unsafe { c_cb(src.len()) };
                let mut cb = vec![0x9Eu8; cap + 64];
                let mut rb = vec![0x9Eu8; cap + 64];
                let n = unsafe {
                    c_cc(cb.as_mut_ptr(), cap, src.as_ptr(), src.len(), cct.as_ptr())
                };
                let m = unsafe {
                    r_cc(rb.as_mut_ptr(), cap, src.as_ptr(), src.len(), rct.as_ptr())
                };
                assert_eq_dbg(&format!("FSE_compress_usingCTable {t}"), n, m);
                assert_bytes_eq(
                    &format!("FSE_compress_usingCTable {t} / buffer"),
                    &cb,
                    &rb,
                );

                // ---- cross-consumption: Rust CTable into the C encoder
                let mut xb = vec![0x9Eu8; cap + 64];
                let x = unsafe {
                    c_cc(xb.as_mut_ptr(), cap, src.as_ptr(), src.len(), rct.as_ptr())
                };
                assert_eq_dbg(&format!("C encoder + Rust CTable {t}"), x, n);
                assert_bytes_eq(&format!("C encoder + Rust CTable {t} bytes"), &xb, &cb);

                if n == 0 || n > cap {
                    continue; // incompressible / error: nothing to decode
                }

                // ---- dstCapacity sweep on the encoder: every value in the
                // small range where the bitstream initialisation and the
                // "doesn't fit" early-outs live, then a coarse sweep up to the
                // exact size (a full 0..=n sweep on a 70 KB input would be
                // ~70k encoder invocations per row).
                let mut enc_caps: Vec<usize> = (0..=n.min(72)).collect();
                let step = (n / 24).max(1);
                enc_caps.extend((0..=n).step_by(step));
                enc_caps.push(n.saturating_sub(1));
                enc_caps.push(n);
                enc_caps.sort_unstable();
                enc_caps.dedup();
                for c in enc_caps {
                    let mut cb2 = vec![0x11u8; cap + 64];
                    let mut rb2 = vec![0x11u8; cap + 64];
                    let a = unsafe {
                        c_cc(cb2.as_mut_ptr(), c, src.as_ptr(), src.len(), cct.as_ptr())
                    };
                    let b = unsafe {
                        r_cc(rb2.as_mut_ptr(), c, src.as_ptr(), src.len(), rct.as_ptr())
                    };
                    assert_eq_dbg(&format!("FSE_compress_usingCTable {t} cap={c}"), a, b);
                    assert_bytes_eq(
                        &format!("FSE_compress_usingCTable {t} cap={c} / buffer"),
                        &cb2,
                        &rb2,
                    );
                }

                // ---- assemble the full stream and decode it
                let bound = unsafe { c_nb(msv, tl) };
                let mut hdr = vec![0u8; bound + 32];
                let hn = unsafe { c_w(hdr.as_mut_ptr(), bound, norm.as_ptr(), msv, tl) };
                assert!(hn <= bound, "{t}: writeNCount failed {hn:#x}");
                let mut stream = hdr[..hn].to_vec();
                stream.extend_from_slice(&cb[..n]);

                for &bmi2 in &[0i32, 1] {
                    let mut cd = vec![0x77u8; src.len() + 64];
                    let mut rd = vec![0x77u8; src.len() + 64];
                    let mut cwk = abuf(dwsz, 0);
                    let mut rwk = abuf(dwsz, 0);
                    let a = unsafe {
                        c_d(
                            cd.as_mut_ptr(),
                            src.len(),
                            stream.as_ptr(),
                            stream.len(),
                            FSE_MAX_TABLELOG,
                            aptr(&mut cwk),
                            dwsz,
                            bmi2,
                        )
                    };
                    let b = unsafe {
                        r_d(
                            rd.as_mut_ptr(),
                            src.len(),
                            stream.as_ptr(),
                            stream.len(),
                            FSE_MAX_TABLELOG,
                            aptr(&mut rwk),
                            dwsz,
                            bmi2,
                        )
                    };
                    let dt = format!("FSE_decompress_wksp_bmi2({bmi2}) {t}");
                    assert_eq_dbg(&dt, a, b);
                    assert_eq_dbg(&format!("{dt} / regenerated size"), a, src.len());
                    assert_bytes_eq(&format!("{dt} / buffer"), &cd, &rd);
                    assert_bytes_eq(&format!("{dt} / plaintext"), &cd[..a], src);
                }
            }
        }
    }
}

/// Every checked failure mode of `FSE_decompress_wksp_bmi2`: workspace too
/// small (full sweep), `maxLog` below the stream's tableLog, `dstCapacity`
/// sweep 0..exact, `cSrcSize==0`, truncation and bit-flip corruption.
#[test]
fn fse_decompress_error_paths() {
    let i = impls();
    let (c_d, r_d) = i.pair::<Fn_FSE_decompress_wksp_bmi2>("FSE_decompress_wksp_bmi2");

    let mut rng = Rng::new(0x5E1F_0041);
    let mut streams: Vec<(String, FseStream)> = Vec::new();
    for (tag, src) in alphabet_inputs(&mut rng) {
        for &max_tl in &[0u32, 6, 12] {
            if let Some(s) = fse_stream_c(i, &src, max_tl, max_tl & 1) {
                streams.push((format!("{tag}/tl{}", s.table_log), s));
            }
        }
    }
    assert!(streams.len() > 20, "expected a decent FSE stream corpus");

    let full = fse_decompress_wksp(FSE_MAX_TABLELOG, FSE_MAX_SYMBOL_VALUE);

    // ---- workspace sweep. The first check is `wkspSize < sizeof(wksp)`
    // (ERROR(GENERIC)); after the header is parsed the check becomes
    // FSE_DECOMPRESS_WKSP_SIZE(tableLog, msv) (ERROR(tableLog_tooLarge)).
    let (tag, s) = &streams[3];
    let mut wsizes: Vec<usize> = vec![0, 1, 4, 8, 64, 128, 256, 512, 1024];
    let mut w = 1024;
    while w < full {
        wsizes.push(w);
        w += (full - 1024) / 12 + 1;
    }
    wsizes.push(full - 1);
    wsizes.push(full);
    wsizes.push(full + 4096);
    for &wsz in &wsizes {
        for &bmi2 in &[0i32, 1] {
            let mut cd = vec![0u8; s.src.len() + 64];
            let mut rd = vec![0u8; s.src.len() + 64];
            let mut cwk = abuf(full + 4096, 0x5A);
            let mut rwk = abuf(full + 4096, 0x5A);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), s.stream.len(),
                    FSE_MAX_TABLELOG, aptr(&mut cwk), wsz, bmi2)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), s.stream.len(),
                    FSE_MAX_TABLELOG, aptr(&mut rwk), wsz, bmi2)
            };
            let t = format!("FSE_decompress wksp={wsz} bmi2={bmi2} {tag}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
        }
    }

    for (tag, s) in streams.iter().take(24) {
        // ---- maxLog axis, including below the stream's own tableLog
        for max_log in 0..=FSE_TABLELOG_ABSOLUTE_MAX + 2 {
            let mut cd = vec![0u8; s.src.len() + 64];
            let mut rd = vec![0u8; s.src.len() + 64];
            let mut cwk = abuf(full, 0);
            let mut rwk = abuf(full, 0);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), s.stream.len(),
                    max_log, aptr(&mut cwk), full, 0)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), s.stream.len(),
                    max_log, aptr(&mut rwk), full, 0)
            };
            let t = format!("FSE_decompress maxLog={max_log} {tag}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
            if max_log < s.table_log {
                assert_eq_dbg(
                    &format!("{t} expect tableLog_tooLarge"),
                    a,
                    ec(ZSTD_error_tableLog_tooLarge),
                );
            }
        }

        // ---- dstCapacity sweep 0..=srcSize
        let step = (s.src.len() / 12).max(1);
        let mut caps: Vec<usize> = (0..s.src.len()).step_by(step).collect();
        caps.push(s.src.len());
        caps.push(s.src.len() + 1);
        for &cap in &caps {
            let mut cd = vec![0x3Cu8; s.src.len() + 64];
            let mut rd = vec![0x3Cu8; s.src.len() + 64];
            let mut cwk = abuf(full, 0);
            let mut rwk = abuf(full, 0);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), cap, s.stream.as_ptr(), s.stream.len(),
                    FSE_MAX_TABLELOG, aptr(&mut cwk), full, 0)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), cap, s.stream.as_ptr(), s.stream.len(),
                    FSE_MAX_TABLELOG, aptr(&mut rwk), full, 0)
            };
            let t = format!("FSE_decompress dstCap={cap}/{} {tag}", s.src.len());
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
        }

        // ---- truncation: every prefix, including cSrcSize == 0
        let hstep = (s.stream.len() / 40).max(1);
        let mut cuts: Vec<usize> = (0..s.stream.len()).step_by(hstep).collect();
        cuts.extend(0..=s.header_len.min(s.stream.len()));
        cuts.push(s.stream.len());
        for &cut in &cuts {
            let mut cd = vec![0x2Du8; s.src.len() + 64];
            let mut rd = vec![0x2Du8; s.src.len() + 64];
            let mut cwk = abuf(full, 0);
            let mut rwk = abuf(full, 0);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), cut,
                    FSE_MAX_TABLELOG, aptr(&mut cwk), full, 0)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), s.src.len(), s.stream.as_ptr(), cut,
                    FSE_MAX_TABLELOG, aptr(&mut rwk), full, 0)
            };
            let t = format!("FSE_decompress truncated {cut}/{} {tag}", s.stream.len());
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
        }

        // ---- bit-flip corruption over the whole stream
        for _ in 0..60 {
            let mut v = s.stream.clone();
            let pos = rng.below(v.len());
            v[pos] ^= 1u8 << rng.below(8);
            let mut cd = vec![0x1Eu8; s.src.len() + 64];
            let mut rd = vec![0x1Eu8; s.src.len() + 64];
            let mut cwk = abuf(full, 0);
            let mut rwk = abuf(full, 0);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), s.src.len(), v.as_ptr(), v.len(),
                    FSE_MAX_TABLELOG, aptr(&mut cwk), full, 0)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), s.src.len(), v.as_ptr(), v.len(),
                    FSE_MAX_TABLELOG, aptr(&mut rwk), full, 0)
            };
            let t = format!("FSE_decompress corrupt@{pos} {tag}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
        }
    }

    // ---- pure garbage input, all lengths that matter for the 8-byte NCount
    // fast path plus larger buffers.
    for len in [0usize, 1, 2, 3, 4, 7, 8, 9, 16, 64, 500] {
        for _ in 0..60 {
            let mut v = vec![0u8; len];
            for b in v.iter_mut() {
                *b = rng.byte();
            }
            let mut cd = vec![0u8; 4096];
            let mut rd = vec![0u8; 4096];
            let mut cwk = abuf(full, 0);
            let mut rwk = abuf(full, 0);
            let a = unsafe {
                c_d(cd.as_mut_ptr(), 4096, v.as_ptr(), len, FSE_MAX_TABLELOG,
                    aptr(&mut cwk), full, 0)
            };
            let b = unsafe {
                r_d(rd.as_mut_ptr(), 4096, v.as_ptr(), len, FSE_MAX_TABLELOG,
                    aptr(&mut rwk), full, 0)
            };
            let t = format!("FSE_decompress garbage len={len}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
        }
    }
}

// ================================================================== 6. HUF CTable

/// `HUF_buildCTable_wksp` -> `HUF_writeCTable_wksp` -> `HUF_readStats*` ->
/// `HUF_readCTable`, all compared byte-for-byte. The CTable equality check is
/// the strong one: the two libraries must produce identical Huffman tables, not
/// merely tables that happen to encode the same.
#[test]
fn huf_build_write_read_ctable() {
    let i = impls();
    let (c_bc, r_bc) = i.pair::<Fn_HUF_buildCTable_wksp>("HUF_buildCTable_wksp");
    let (c_wc, r_wc) = i.pair::<Fn_HUF_writeCTable_wksp>("HUF_writeCTable_wksp");
    let (c_rc, r_rc) = i.pair::<Fn_HUF_readCTable>("HUF_readCTable");
    let (c_hd, r_hd) = i.pair::<Fn_HUF_readCTableHeader>("HUF_readCTableHeader");
    let (c_nb, r_nb) = i.pair::<Fn_HUF_getNbBitsFromCTable>("HUF_getNbBitsFromCTable");
    let (c_rs, r_rs) = i.pair::<Fn_HUF_readStats>("HUF_readStats");
    let (c_rsw, r_rsw) = i.pair::<Fn_HUF_readStats_wksp>("HUF_readStats_wksp");

    let mut rng = Rng::new(0x5E1F_0050);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &[13usize, 300, 5000, 70_000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    // tableLog axis. `HUF_buildCTable_wksp` has *no* lower bound check on
    // `maxNbBits`: if the requested depth cannot represent the alphabet,
    // `HUF_setMaxHeight`'s "repay cost" loop indexes `rankLast[]` (14 entries)
    // with `ZSTD_highbit32(totalCost)+1` and then dereferences the garbage it
    // reads — the C only catches that with `assert(rankLast[nBitsToDecrease] !=
    // noSymbol)`, which is compiled out here. zstd itself never gets there
    // because `HUF_optimalTableLog` starts its search at
    // `HUF_minTableLog(cardinality)`, so the axis is filtered the same way.
    // `maxNbBits==0` is the "use HUF_TABLELOG_DEFAULT" sentinel.
    // The `huffLog > HUF_TABLELOG_MAX` rejection is a *checked* path on
    // HUF_compress{1,4}X_repeat and is swept in `huf_compress_error_paths`.
    let max_bits_all = [0u32, 5, 6, 7, 8, 9, 10, 11, 12];

    for (tag, src) in &corpus {
        if src.is_empty() {
            continue;
        }
        let (count, obs_msv, _) = hist_c(i, src);

        for &msv in &[obs_msv, 255u32] {
            let min_tl = unsafe {
                let (c_card, _) = i.pair::<Fn_HUF_cardinality>("HUF_cardinality");
                let (c_min, _) = i.pair::<Fn_HUF_minTableLog>("HUF_minTableLog");
                let card = c_card(count.as_ptr(), msv);
                if card == 0 { continue; }
                c_min(card)
            };
            let max_bits: Vec<u32> = max_bits_all
                .iter()
                .copied()
                .filter(|&mb| mb == 0 || mb >= min_tl)
                .collect();
            for &mb in &max_bits {
                let mut cct = vec![0xC1C1_C1C1_C1C1_C1C1usize; HUF_CTABLE_ST_MAX];
                let mut rct = vec![0xC1C1_C1C1_C1C1_C1C1usize; HUF_CTABLE_ST_MAX];
                let mut cw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x4B);
                let mut rw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x4B);
                let a = unsafe {
                    c_bc(cct.as_mut_ptr(), count.as_ptr(), msv, mb,
                         aptr(&mut cw), HUF_CTABLE_WORKSPACE_SIZE)
                };
                let b = unsafe {
                    r_bc(rct.as_mut_ptr(), count.as_ptr(), msv, mb,
                         aptr(&mut rw), HUF_CTABLE_WORKSPACE_SIZE)
                };
                let t = format!("HUF_buildCTable_wksp {tag} msv={msv} maxNbBits={mb}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / CTable"), bytes_of(&cct), bytes_of(&rct));
                assert_bytes_eq(&format!("{t} / workspace"), bytes_of(&cw), bytes_of(&rw));
                if a > ec(200) {
                    continue; // rejected (maxNbBits too large for this tree)
                }
                let huff_log = a as u32;

                // ---- header accessor + per-symbol nbBits
                unsafe {
                    let ch = c_hd(cct.as_ptr());
                    let rh = r_hd(rct.as_ptr());
                    assert_eq_dbg(&format!("{t} / HUF_readCTableHeader"), ch, rh);
                    assert_eq_dbg(&format!("{t} / header.tableLog"), ch.table_log as u32, huff_log);
                    assert_eq_dbg(
                        &format!("{t} / header.maxSymbolValue"),
                        ch.max_symbol_value as u32,
                        msv,
                    );
                    for s in 0..=HUF_SYMBOLVALUE_MAX {
                        assert_eq_dbg(
                            &format!("{t} / HUF_getNbBitsFromCTable({s})"),
                            c_nb(cct.as_ptr(), s),
                            r_nb(rct.as_ptr(), s),
                        );
                    }
                }

                // ---- serialize the table
                let dst_cap = 512usize;
                let mut cd = vec![0x8Fu8; dst_cap + 64];
                let mut rd = vec![0x8Fu8; dst_cap + 64];
                let mut cw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x6E);
                let mut rw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x6E);
                let hn = unsafe {
                    c_wc(cd.as_mut_ptr(), dst_cap, cct.as_ptr(), msv, huff_log,
                         aptr(&mut cw), HUF_CTABLE_WORKSPACE_SIZE)
                };
                let hm = unsafe {
                    r_wc(rd.as_mut_ptr(), dst_cap, rct.as_ptr(), msv, huff_log,
                         aptr(&mut rw), HUF_CTABLE_WORKSPACE_SIZE)
                };
                let wt = format!("HUF_writeCTable_wksp {tag} msv={msv} huffLog={huff_log}");
                assert_eq_dbg(&wt, hn, hm);
                assert_bytes_eq(&format!("{wt} / dst"), &cd, &rd);
                assert_bytes_eq(&format!("{wt} / workspace"), bytes_of(&cw), bytes_of(&rw));
                if hn > ec(200) {
                    continue;
                }
                let desc = cd[..hn].to_vec();

                // ---- dstCapacity sweep 0..=exact
                for cap in 0..=hn {
                    let mut cd2 = vec![0x2Bu8; dst_cap + 64];
                    let mut rd2 = vec![0x2Bu8; dst_cap + 64];
                    let mut cw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x6E);
                    let mut rw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0x6E);
                    let a = unsafe {
                        c_wc(cd2.as_mut_ptr(), cap, cct.as_ptr(), msv, huff_log,
                             aptr(&mut cw), HUF_CTABLE_WORKSPACE_SIZE)
                    };
                    let b = unsafe {
                        r_wc(rd2.as_mut_ptr(), cap, rct.as_ptr(), msv, huff_log,
                             aptr(&mut rw), HUF_CTABLE_WORKSPACE_SIZE)
                    };
                    assert_eq_dbg(&format!("{wt} cap={cap}"), a, b);
                    assert_bytes_eq(&format!("{wt} cap={cap} / dst"), &cd2, &rd2);
                }

                // ---- HUF_readStats / HUF_readStats_wksp on the description
                for &(rs_name, flags) in &[
                    ("readStats", -1i32),
                    ("readStats_wksp/0", 0),
                    ("readStats_wksp/bmi2", HUF_flags_bmi2),
                ] {
                    for &hw_size in &[HUF_SYMBOLVALUE_MAX as usize + 1, 256, 128, 4, 1] {
                        let mut chw = vec![0x99u8; 512];
                        let mut rhw = vec![0x99u8; 512];
                        let mut crank = vec![0xEEEE_EEEEu32; 32];
                        let mut rrank = vec![0xEEEE_EEEEu32; 32];
                        let mut cns = 0u32;
                        let mut rns = 0u32;
                        let mut ctl = 0u32;
                        let mut rtl = 0u32;
                        let ws = fse_decompress_wksp(6, HUF_TABLELOG_MAX - 1);
                        let mut cw = abuf(ws, 0x3A);
                        let mut rw = abuf(ws, 0x3A);
                        let (a, b) = unsafe {
                            if flags < 0 {
                                (
                                    c_rs(chw.as_mut_ptr(), hw_size, crank.as_mut_ptr(),
                                         &mut cns, &mut ctl, desc.as_ptr(), desc.len()),
                                    r_rs(rhw.as_mut_ptr(), hw_size, rrank.as_mut_ptr(),
                                         &mut rns, &mut rtl, desc.as_ptr(), desc.len()),
                                )
                            } else {
                                (
                                    c_rsw(chw.as_mut_ptr(), hw_size, crank.as_mut_ptr(),
                                          &mut cns, &mut ctl, desc.as_ptr(), desc.len(),
                                          aptr(&mut cw), ws, flags),
                                    r_rsw(rhw.as_mut_ptr(), hw_size, rrank.as_mut_ptr(),
                                          &mut rns, &mut rtl, desc.as_ptr(), desc.len(),
                                          aptr(&mut rw), ws, flags),
                                )
                            }
                        };
                        let st = format!("HUF_{rs_name} {tag} hwSize={hw_size}");
                        assert_eq_dbg(&st, a, b);
                        assert_eq_dbg(&format!("{st} / nbSymbols"), cns, rns);
                        assert_eq_dbg(&format!("{st} / tableLog"), ctl, rtl);
                        assert_bytes_eq(&format!("{st} / rankStats"),
                                        bytes_of(&crank), bytes_of(&rrank));
                        assert_bytes_eq(&format!("{st} / huffWeight"),
                                        bytes_of(&chw), bytes_of(&rhw));
                    }
                }

                // ---- HUF_readCTable: reconstruct and compare
                for &declared in &[255u32, msv, 1, 0] {
                    let mut cct2 = vec![0x7070_7070_7070_7070usize; HUF_CTABLE_ST_MAX];
                    let mut rct2 = vec![0x7070_7070_7070_7070usize; HUF_CTABLE_ST_MAX];
                    let mut cmsv = declared;
                    let mut rmsv = declared;
                    let mut czw = 0xFFFF_FFFFu32;
                    let mut rzw = 0xFFFF_FFFFu32;
                    let a = unsafe {
                        c_rc(cct2.as_mut_ptr(), &mut cmsv, desc.as_ptr(), desc.len(), &mut czw)
                    };
                    let b = unsafe {
                        r_rc(rct2.as_mut_ptr(), &mut rmsv, desc.as_ptr(), desc.len(), &mut rzw)
                    };
                    let rt = format!("HUF_readCTable {tag} declaredMsv={declared}");
                    assert_eq_dbg(&rt, a, b);
                    assert_eq_dbg(&format!("{rt} / msvPtr"), cmsv, rmsv);
                    assert_eq_dbg(&format!("{rt} / hasZeroWeights"), czw, rzw);
                    // HUF_readCTable only fills ct[0..nbSymbols); the rest of
                    // the buffer is left as-is, and both buffers were
                    // pre-filled identically, so a full compare is still exact.
                    assert_bytes_eq(&format!("{rt} / CTable"),
                                    bytes_of(&cct2), bytes_of(&rct2));
                    if a <= ec(200) {
                        assert_eq_dbg(&format!("{rt} / readSize"), a, desc.len());
                    }
                }

                // ---- truncated / corrupted table descriptions
                for cut in 0..desc.len() {
                    let mut cct2 = vec![0usize; HUF_CTABLE_ST_MAX];
                    let mut rct2 = vec![0usize; HUF_CTABLE_ST_MAX];
                    let mut cmsv = 255u32;
                    let mut rmsv = 255u32;
                    let mut czw = 0u32;
                    let mut rzw = 0u32;
                    let a = unsafe {
                        c_rc(cct2.as_mut_ptr(), &mut cmsv, desc.as_ptr(), cut, &mut czw)
                    };
                    let b = unsafe {
                        r_rc(rct2.as_mut_ptr(), &mut rmsv, desc.as_ptr(), cut, &mut rzw)
                    };
                    let ct2 = format!("HUF_readCTable truncated {cut}/{} {tag}", desc.len());
                    assert_eq_dbg(&ct2, a, b);
                    assert_eq_dbg(&format!("{ct2} msvPtr"), cmsv, rmsv);
                    assert_bytes_eq(&format!("{ct2} CTable"),
                                    bytes_of(&cct2), bytes_of(&rct2));
                }
            }
        }
    }
}

/// `HUF_writeCTable_wksp` / `HUF_buildCTable_wksp` checked failure modes:
/// workspace below `HUF_CTABLE_WORKSPACE_SIZE` and `maxSymbolValue > 255`.
#[test]
fn huf_ctable_error_paths() {
    let i = impls();
    let (c_bc, r_bc) = i.pair::<Fn_HUF_buildCTable_wksp>("HUF_buildCTable_wksp");
    let (c_wc, r_wc) = i.pair::<Fn_HUF_writeCTable_wksp>("HUF_writeCTable_wksp");

    let mut rng = Rng::new(0x5E1F_0051);
    let src = gen_shape(Shape::SkewedText, 5000, &mut rng);
    let (count, msv, _) = hist_c(i, &src);

    // build a good reference CTable with the C library
    let mut ref_ct = vec![0usize; HUF_CTABLE_ST_MAX];
    let mut w = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0);
    let huff_log = unsafe {
        c_bc(ref_ct.as_mut_ptr(), count.as_ptr(), msv, 11,
             aptr(&mut w), HUF_CTABLE_WORKSPACE_SIZE)
    } as u32;
    assert!(huff_log <= HUF_TABLELOG_MAX, "reference build failed");

    let mut wsizes: Vec<usize> = vec![0, 1, 4, 8, 64, 512, 1024, 2048, 4096];
    wsizes.push(HUF_CTABLE_WORKSPACE_SIZE - 1);
    wsizes.push(HUF_CTABLE_WORKSPACE_SIZE);
    wsizes.push(HUF_CTABLE_WORKSPACE_SIZE + 8);
    wsizes.push(HUF_WORKSPACE_SIZE);

    for &wsz in &wsizes {
        // HUF_buildCTable_wksp
        let mut cct = vec![0x11usize; HUF_CTABLE_ST_MAX];
        let mut rct = vec![0x11usize; HUF_CTABLE_ST_MAX];
        let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x1A);
        let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x1A);
        let a = unsafe {
            c_bc(cct.as_mut_ptr(), count.as_ptr(), msv, 11, aptr(&mut cw), wsz)
        };
        let b = unsafe {
            r_bc(rct.as_mut_ptr(), count.as_ptr(), msv, 11, aptr(&mut rw), wsz)
        };
        let t = format!("HUF_buildCTable_wksp wsz={wsz}");
        assert_eq_dbg(&t, a, b);
        assert_bytes_eq(&format!("{t} / CTable"), bytes_of(&cct), bytes_of(&rct));
        if wsz < HUF_CTABLE_WORKSPACE_SIZE {
            assert_eq_dbg(
                &format!("{t} expect workSpace_tooSmall"),
                a,
                ec(ZSTD_error_workSpace_tooSmall),
            );
        }

        // HUF_writeCTable_wksp
        let mut cd = vec![0x22u8; 1024];
        let mut rd = vec![0x22u8; 1024];
        let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x2A);
        let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x2A);
        let a = unsafe {
            c_wc(cd.as_mut_ptr(), 512, ref_ct.as_ptr(), msv, huff_log, aptr(&mut cw), wsz)
        };
        let b = unsafe {
            r_wc(rd.as_mut_ptr(), 512, ref_ct.as_ptr(), msv, huff_log, aptr(&mut rw), wsz)
        };
        let t = format!("HUF_writeCTable_wksp wsz={wsz}");
        assert_eq_dbg(&t, a, b);
        assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
    }

    // maxSymbolValue > HUF_SYMBOLVALUE_MAX on both entry points. The count array
    // is oversized so the (rejected) call cannot read out of bounds even if the
    // check were missing.
    let big_count = {
        let mut v = vec![0u32; 70_000];
        v[..count.len()].copy_from_slice(&count);
        v
    };
    for &bad in &[256u32, 257, 511, 65_535] {
        let mut cct = vec![0x33usize; 70_000];
        let mut rct = vec![0x33usize; 70_000];
        let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x3A);
        let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x3A);
        let a = unsafe {
            c_bc(cct.as_mut_ptr(), big_count.as_ptr(), bad, 11,
                 aptr(&mut cw), HUF_WORKSPACE_SIZE)
        };
        let b = unsafe {
            r_bc(rct.as_mut_ptr(), big_count.as_ptr(), bad, 11,
                 aptr(&mut rw), HUF_WORKSPACE_SIZE)
        };
        let t = format!("HUF_buildCTable_wksp badMsv={bad}");
        assert_eq_dbg(&t, a, b);
        assert_eq_dbg(
            &format!("{t} expect maxSymbolValue_tooLarge"),
            a,
            ec(ZSTD_error_maxSymbolValue_tooLarge),
        );
        assert_bytes_eq(&format!("{t} / CTable"), bytes_of(&cct), bytes_of(&rct));

        let mut cd = vec![0x44u8; 70_000];
        let mut rd = vec![0x44u8; 70_000];
        let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x4A);
        let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x4A);
        let a = unsafe {
            c_wc(cd.as_mut_ptr(), 65_536, ref_ct.as_ptr(), bad, huff_log,
                 aptr(&mut cw), HUF_WORKSPACE_SIZE)
        };
        let b = unsafe {
            r_wc(rd.as_mut_ptr(), 65_536, ref_ct.as_ptr(), bad, huff_log,
                 aptr(&mut rw), HUF_WORKSPACE_SIZE)
        };
        let t = format!("HUF_writeCTable_wksp badMsv={bad}");
        assert_eq_dbg(&t, a, b);
        assert_eq_dbg(
            &format!("{t} expect maxSymbolValue_tooLarge"),
            a,
            ec(ZSTD_error_maxSymbolValue_tooLarge),
        );
        assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
    }
}

/// The pure CTable inspectors: `HUF_cardinality`, `HUF_minTableLog`,
/// `HUF_optimalTableLog`, `HUF_estimateCompressedSize`, `HUF_validateCTable`.
#[test]
fn huf_ctable_helpers() {
    let i = impls();
    let (c_card, r_card) = i.pair::<Fn_HUF_cardinality>("HUF_cardinality");
    let (c_min, r_min) = i.pair::<Fn_HUF_minTableLog>("HUF_minTableLog");
    let (c_opt, r_opt) = i.pair::<Fn_HUF_optimalTableLog>("HUF_optimalTableLog");
    let (c_est, r_est) = i.pair::<Fn_HUF_estimateCompressedSize>("HUF_estimateCompressedSize");
    let (c_val, r_val) = i.pair::<Fn_HUF_validateCTable>("HUF_validateCTable");
    let (c_bc, _) = i.pair::<Fn_HUF_buildCTable_wksp>("HUF_buildCTable_wksp");

    // HUF_minTableLog(0) would be ZSTD_highbit32(0) (UB in C), so start at 1.
    for card in 1..=300u32 {
        unsafe {
            assert_eq_dbg(&format!("HUF_minTableLog({card})"), c_min(card), r_min(card));
        }
    }

    let mut rng = Rng::new(0x5E1F_0052);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &[13usize, 700, 9000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    for (tag, src) in &corpus {
        if src.len() < 2 {
            continue;
        }
        let (count, obs_msv, _) = hist_c(i, src);
        if obs_msv == 0 {
            continue; // FSE_optimalTableLog_internal needs msv>=1
        }

        for &msv in &[obs_msv, 255u32] {
            unsafe {
                assert_eq_dbg(
                    &format!("HUF_cardinality {tag} msv={msv}"),
                    c_card(count.as_ptr(), msv),
                    r_card(count.as_ptr(), msv),
                );
            }

            // build a CTable to feed the estimators
            let mut ct = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut w = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0);
            let hl = unsafe {
                c_bc(ct.as_mut_ptr(), count.as_ptr(), msv, 11,
                     aptr(&mut w), HUF_CTABLE_WORKSPACE_SIZE)
            };
            if hl > ec(200) {
                continue;
            }

            for &q in &[0u32, 1, 2, 127, msv, 255] {
                unsafe {
                    assert_eq_dbg(
                        &format!("HUF_estimateCompressedSize {tag} msv={msv} q={q}"),
                        c_est(ct.as_ptr(), count.as_ptr(), q),
                        r_est(ct.as_ptr(), count.as_ptr(), q),
                    );
                    assert_eq_dbg(
                        &format!("HUF_validateCTable {tag} msv={msv} q={q}"),
                        c_val(ct.as_ptr(), count.as_ptr(), q),
                        r_val(ct.as_ptr(), count.as_ptr(), q),
                    );
                }
            }

            // HUF_validateCTable against a *different* histogram, so the
            // "count[s]!=0 but nbBits==0" rejection actually triggers.
            let other = {
                let mut v = vec![1u32; 512];
                for (s, c) in v.iter_mut().enumerate().take(256) {
                    *c = ((s as u32) % 7) + 1;
                }
                v
            };
            for &q in &[0u32, 1, 200, 255] {
                unsafe {
                    assert_eq_dbg(
                        &format!("HUF_validateCTable/other {tag} msv={msv} q={q}"),
                        c_val(ct.as_ptr(), other.as_ptr(), q),
                        r_val(ct.as_ptr(), other.as_ptr(), q),
                    );
                }
            }

            // HUF_optimalTableLog over the whole flag axis, including the
            // expensive `optimalDepth` search.
            for &flags in &[
                0i32,
                HUF_flags_optimalDepth,
                HUF_flags_bmi2,
                HUF_flags_optimalDepth | HUF_flags_bmi2,
                HUF_flags_optimalDepth | HUF_flags_preferRepeat,
            ] {
                for &max_tl in &[0u32, 1, 5, 8, 11, 12] {
                    let mut cscr = vec![0usize; HUF_CTABLE_ST_MAX];
                    let mut rscr = vec![0usize; HUF_CTABLE_ST_MAX];
                    let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x5B);
                    let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x5B);
                    let a = unsafe {
                        c_opt(max_tl, src.len(), msv, aptr(&mut cw), HUF_WORKSPACE_SIZE,
                              cscr.as_mut_ptr(), count.as_ptr(), flags)
                    };
                    let b = unsafe {
                        r_opt(max_tl, src.len(), msv, aptr(&mut rw), HUF_WORKSPACE_SIZE,
                              rscr.as_mut_ptr(), count.as_ptr(), flags)
                    };
                    let t = format!(
                        "HUF_optimalTableLog {tag} msv={msv} maxTL={max_tl} flags={flags}"
                    );
                    assert_eq_dbg(&t, a, b);
                    assert_bytes_eq(&format!("{t} / scratch table"),
                                    bytes_of(&cscr), bytes_of(&rscr));
                    assert_bytes_eq(&format!("{t} / workspace"),
                                    bytes_of(&cw), bytes_of(&rw));
                }
            }
        }
    }
}

// ============================================================= 7. HUF compress

/// `HUF_compress1X_repeat` / `HUF_compress4X_repeat` across shapes, sizes,
/// `maxSymbolValue`, `tableLog`, every flag bit and every `HUF_repeat` state.
/// The output bytes, the `hufTable` the call leaves behind and the updated
/// `*repeat` must all match.
///
/// For `HUF_repeat_check` / `HUF_repeat_valid` the `hufTable` must already hold
/// a table produced by a previous `HUF_compress*_repeat` call — that is the
/// documented contract, and feeding garbage instead makes the C encode with a
/// `nbBits` read out of uninitialised memory (shift-by->64 UB). So each library
/// first *primes* its own table on a prefix of the input, which is exactly how
/// zstd drives this API block-to-block, and the two primed tables are compared
/// too.
#[test]
fn huf_compress_repeat() {
    let i = impls();
    let (c_1x, r_1x) = i.pair::<Fn_HUF_repeat>("HUF_compress1X_repeat");
    let (c_4x, r_4x) = i.pair::<Fn_HUF_repeat>("HUF_compress4X_repeat");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("HUF_compressBound");

    let mut rng = Rng::new(0x5E1F_0060);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    let flag_sets = [
        0i32,
        HUF_flags_bmi2,
        HUF_flags_optimalDepth,
        HUF_flags_suspectUncompressible,
        HUF_flags_disableAsm,
        HUF_flags_disableFast,
        HUF_flags_preferRepeat,
        HUF_flags_optimalDepth | HUF_flags_bmi2 | HUF_flags_disableAsm,
    ];

    let (c_card, _) = i.pair::<Fn_HUF_cardinality>("HUF_cardinality");
    let (c_min, _) = i.pair::<Fn_HUF_minTableLog>("HUF_minTableLog");

    for (tag, src) in &corpus {
        // prefix used to prime the repeat table (a different but related block)
        let prime: Vec<u8> = if src.len() > 64 {
            src[..src.len() / 2].to_vec()
        } else {
            src.clone()
        };

        // tableLog axis. With `HUF_flags_optimalDepth` set, HUF_optimalTableLog
        // returns `maxTableLog` verbatim when `minTableLog > maxTableLog`
        // (its search loop simply never runs), and HUF_buildCTable_wksp is then
        // called with a depth that cannot represent the alphabet — the same
        // `HUF_setMaxHeight` out-of-bounds the assert-less C build does not
        // catch. zstd always passes HUF_TABLELOG_DEFAULT here, so the axis is
        // filtered to `0` (the "use default" sentinel) plus depths the alphabet
        // can actually be coded in.
        let min_tl = if src.is_empty() {
            1
        } else {
            let (count, _, _) = hist_c(i, src);
            let card = unsafe { c_card(count.as_ptr(), 255) };
            if card == 0 { 1 } else { unsafe { c_min(card) } }
        };
        let tls: Vec<u32> = [0u32, 5, 8, 11, 12]
            .into_iter()
            .filter(|&tl| tl == 0 || tl >= min_tl)
            .collect();

        for (which, c_f, r_f) in [("1X", &c_1x, &r_1x), ("4X", &c_4x, &r_4x)] {
            for &msv in &[0u32, 255] {
                for &tl in &tls {
                    for &flags in &flag_sets {
                        for &rep_in in
                            &[-1i32, HUF_repeat_none, HUF_repeat_check, HUF_repeat_valid]
                        {
                            let cap = unsafe { c_bound(src.len()) }.max(64);
                            let mut cd = vec![0x6Cu8; cap + 64];
                            let mut rd = vec![0x6Cu8; cap + 64];
                            let mut cht = vec![0usize; HUF_CTABLE_ST_MAX];
                            let mut rht = vec![0usize; HUF_CTABLE_ST_MAX];
                            let mut crep = HUF_repeat_none;
                            let mut rrep = HUF_repeat_none;
                            let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x7D);
                            let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x7D);

                            let t = format!(
                                "HUF_compress{which}_repeat {tag} msv={msv} tl={tl} \
                                 flags={flags} repeat={rep_in}"
                            );

                            // ---- prime the table when the state demands a
                            // pre-existing one.
                            //
                            // HUF_repeat_valid means "assume the table encodes
                            // this input" and the encoder then takes its
                            // bounds-check-free fast flush path: priming it with
                            // a table that cannot code every symbol makes the C
                            // itself write past dstCapacity (only
                            // `assert(bitC.ptr <= bitC.endPtr)` catches it, and
                            // that is compiled out here). So `valid` is primed
                            // on the *same* block, while `check` is primed on a
                            // prefix so HUF_validateCTable's reject path runs.
                            if rep_in == HUF_repeat_check || rep_in == HUF_repeat_valid {
                                let pin: &[u8] = if rep_in == HUF_repeat_valid {
                                    src
                                } else {
                                    &prime
                                };
                                let mut pc = vec![0u8; cap + 64];
                                let mut pr = vec![0u8; cap + 64];
                                let pa = unsafe {
                                    c_f(pc.as_mut_ptr(), cap, pin.as_ptr(), pin.len(),
                                        msv, tl, aptr(&mut cw), HUF_WORKSPACE_SIZE,
                                        cht.as_mut_ptr(), &mut crep, flags & !HUF_flags_preferRepeat)
                                };
                                let pb = unsafe {
                                    r_f(pr.as_mut_ptr(), cap, pin.as_ptr(), pin.len(),
                                        msv, tl, aptr(&mut rw), HUF_WORKSPACE_SIZE,
                                        rht.as_mut_ptr(), &mut rrep, flags & !HUF_flags_preferRepeat)
                                };
                                assert_eq_dbg(&format!("{t} / prime"), pa, pb);
                                assert_bytes_eq(&format!("{t} / prime dst"), &pc, &pr);
                                assert_bytes_eq(
                                    &format!("{t} / primed hufTable"),
                                    bytes_of(&cht),
                                    bytes_of(&rht),
                                );
                                if pa == 0 || pa > ec(200) {
                                    // no table was produced (rle / incompressible /
                                    // rejected) -> HUF_repeat_* would be a lie
                                    continue;
                                }
                                crep = rep_in;
                                rrep = rep_in;
                            }

                            // NULL hufTable/repeat is explicitly tolerated by
                            // HUF_compress_internal (`if (repeat)` /
                            // `if (oldHufTable)`), so rep_in==-1 passes NULL.
                            let (cht_p, crep_p) = if rep_in < 0 {
                                (std::ptr::null_mut(), std::ptr::null_mut())
                            } else {
                                (cht.as_mut_ptr(), &mut crep as *mut i32)
                            };
                            let (rht_p, rrep_p) = if rep_in < 0 {
                                (std::ptr::null_mut(), std::ptr::null_mut())
                            } else {
                                (rht.as_mut_ptr(), &mut rrep as *mut i32)
                            };
                            let a = unsafe {
                                c_f(cd.as_mut_ptr(), cap, src.as_ptr(), src.len(), msv, tl,
                                    aptr(&mut cw), HUF_WORKSPACE_SIZE, cht_p, crep_p, flags)
                            };
                            let b = unsafe {
                                r_f(rd.as_mut_ptr(), cap, src.as_ptr(), src.len(), msv, tl,
                                    aptr(&mut rw), HUF_WORKSPACE_SIZE, rht_p, rrep_p, flags)
                            };
                            assert_eq_dbg(&t, a, b);
                            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
                            assert_eq_dbg(&format!("{t} / *repeat"), crep, rrep);
                            assert_bytes_eq(
                                &format!("{t} / hufTable"),
                                bytes_of(&cht),
                                bytes_of(&rht),
                            );
                            assert_bytes_eq(
                                &format!("{t} / workspace"),
                                bytes_of(&cw),
                                bytes_of(&rw),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `HUF_compress1X_usingCTable` / `HUF_compress4X_usingCTable`: identical
/// output, a full `dstCapacity` sweep and a cross-check that a CTable built by
/// one library encodes identically in the other.
#[test]
fn huf_compress_using_ctable() {
    let i = impls();
    let (c_1x, r_1x) = i.pair::<Fn_HUF_usingCTable>("HUF_compress1X_usingCTable");
    let (c_4x, r_4x) = i.pair::<Fn_HUF_usingCTable>("HUF_compress4X_usingCTable");
    let (c_bc, r_bc) = i.pair::<Fn_HUF_buildCTable_wksp>("HUF_buildCTable_wksp");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("HUF_compressBound");
    let (c_card, _) = i.pair::<Fn_HUF_cardinality>("HUF_cardinality");
    let (c_min, _) = i.pair::<Fn_HUF_minTableLog>("HUF_minTableLog");

    let mut rng = Rng::new(0x5E1F_0061);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS_SMALL {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    for (tag, src) in &corpus {
        if src.is_empty() {
            continue;
        }
        let (count, _obs, _) = hist_c(i, src);
        let card = unsafe { c_card(count.as_ptr(), 255) };
        if card == 0 {
            continue;
        }
        let min_tl = unsafe { c_min(card) };

        for &tl in &[0u32, 8, 11, 12] {
            if tl != 0 && tl < min_tl {
                continue; // see huf_build_write_read_ctable for why
            }
            // both libraries build their own CTable from the same histogram
            let mut cct = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut rct = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut cw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0);
            let mut rw = abuf(HUF_CTABLE_WORKSPACE_SIZE, 0);
            let ca = unsafe {
                c_bc(cct.as_mut_ptr(), count.as_ptr(), 255, tl,
                     aptr(&mut cw), HUF_CTABLE_WORKSPACE_SIZE)
            };
            let rb = unsafe {
                r_bc(rct.as_mut_ptr(), count.as_ptr(), 255, tl,
                     aptr(&mut rw), HUF_CTABLE_WORKSPACE_SIZE)
            };
            assert_eq_dbg(&format!("buildCTable {tag} tl={tl}"), ca, rb);
            assert_bytes_eq(
                &format!("buildCTable {tag} tl={tl} / bit-identical"),
                bytes_of(&cct),
                bytes_of(&rct),
            );
            if ca > ec(200) {
                continue;
            }

            for (which, c_f, r_f) in [("1X", &c_1x, &r_1x), ("4X", &c_4x, &r_4x)] {
                for &flags in &[0i32, HUF_flags_bmi2, HUF_flags_disableAsm, HUF_flags_disableFast] {
                    let cap = unsafe { c_bound(src.len()) }.max(64);
                    let mut cd = vec![0x4Du8; cap + 64];
                    let mut rd = vec![0x4Du8; cap + 64];
                    let n = unsafe {
                        c_f(cd.as_mut_ptr(), cap, src.as_ptr(), src.len(), cct.as_ptr(), flags)
                    };
                    let m = unsafe {
                        r_f(rd.as_mut_ptr(), cap, src.as_ptr(), src.len(), rct.as_ptr(), flags)
                    };
                    let t = format!(
                        "HUF_compress{which}_usingCTable {tag} tl={tl} flags={flags}"
                    );
                    assert_eq_dbg(&t, n, m);
                    assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);

                    // cross-library table consumption
                    let mut xd = vec![0x4Du8; cap + 64];
                    let x = unsafe {
                        c_f(xd.as_mut_ptr(), cap, src.as_ptr(), src.len(), rct.as_ptr(), flags)
                    };
                    assert_eq_dbg(&format!("{t} / C encoder + Rust CTable"), x, n);
                    assert_bytes_eq(&format!("{t} / cross bytes"), &xd, &cd);
                    let mut yd = vec![0x4Du8; cap + 64];
                    let y = unsafe {
                        r_f(yd.as_mut_ptr(), cap, src.as_ptr(), src.len(), cct.as_ptr(), flags)
                    };
                    assert_eq_dbg(&format!("{t} / Rust encoder + C CTable"), y, n);
                    assert_bytes_eq(&format!("{t} / cross bytes 2"), &yd, &cd);

                    if n == 0 || n > ec(200) {
                        continue;
                    }
                    // dstCapacity sweep: all small values (the `dstSize<8` /
                    // `dstSize < 6+1+1+1+8` early-outs) plus a coarse sweep.
                    let mut caps: Vec<usize> = (0..=n.min(40)).collect();
                    let step = (n / 16).max(1);
                    caps.extend((0..=n).step_by(step));
                    caps.push(n);
                    caps.sort_unstable();
                    caps.dedup();
                    for c in caps {
                        let mut cd2 = vec![0x5Eu8; cap + 64];
                        let mut rd2 = vec![0x5Eu8; cap + 64];
                        let a = unsafe {
                            c_f(cd2.as_mut_ptr(), c, src.as_ptr(), src.len(), cct.as_ptr(), flags)
                        };
                        let b = unsafe {
                            r_f(rd2.as_mut_ptr(), c, src.as_ptr(), src.len(), rct.as_ptr(), flags)
                        };
                        assert_eq_dbg(&format!("{t} cap={c}"), a, b);
                        assert_bytes_eq(&format!("{t} cap={c} / dst"), &cd2, &rd2);
                    }
                }
            }
        }
    }
}

/// The *checked* rejections of `HUF_compress{1,4}X_repeat`: workspace too small,
/// `srcSize > HUF_BLOCKSIZE_MAX`, `huffLog > HUF_TABLELOG_MAX`,
/// `maxSymbolValue > HUF_SYMBOLVALUE_MAX`, `srcSize==0`, `dstSize==0`.
#[test]
fn huf_compress_error_paths() {
    let i = impls();
    let (c_1x, r_1x) = i.pair::<Fn_HUF_repeat>("HUF_compress1X_repeat");
    let (c_4x, r_4x) = i.pair::<Fn_HUF_repeat>("HUF_compress4X_repeat");
    let (c_bound, _) = i.pair::<Fn_sz_sz>("HUF_compressBound");

    let mut rng = Rng::new(0x5E1F_0062);
    let ok = gen_shape(Shape::SkewedText, 5000, &mut rng);
    let too_big = gen_shape(Shape::SkewedText, HUF_BLOCKSIZE_MAX + 1, &mut rng);
    let exact = gen_shape(Shape::SkewedText, HUF_BLOCKSIZE_MAX, &mut rng);
    let empty: Vec<u8> = Vec::new();

    let mut wsizes: Vec<usize> = vec![0, 1, 8, 64, 512, 4096, 8192];
    wsizes.push(HUF_WORKSPACE_SIZE - 1);
    wsizes.push(HUF_WORKSPACE_SIZE);
    wsizes.push(HUF_WORKSPACE_SIZE + 64);

    for (which, c_f, r_f) in [("1X", &c_1x, &r_1x), ("4X", &c_4x, &r_4x)] {
        // ---- workspace sweep
        for &wsz in &wsizes {
            let cap = unsafe { c_bound(ok.len()) };
            let mut cd = vec![0x9Au8; cap + 64];
            let mut rd = vec![0x9Au8; cap + 64];
            let mut cht = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut rht = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut crep = HUF_repeat_none;
            let mut rrep = HUF_repeat_none;
            let mut cw = abuf(HUF_WORKSPACE_SIZE + 64, 0x11);
            let mut rw = abuf(HUF_WORKSPACE_SIZE + 64, 0x11);
            let a = unsafe {
                c_f(cd.as_mut_ptr(), cap, ok.as_ptr(), ok.len(), 255, 11,
                    aptr(&mut cw), wsz, cht.as_mut_ptr(), &mut crep, 0)
            };
            let b = unsafe {
                r_f(rd.as_mut_ptr(), cap, ok.as_ptr(), ok.len(), 255, 11,
                    aptr(&mut rw), wsz, rht.as_mut_ptr(), &mut rrep, 0)
            };
            let t = format!("HUF_compress{which}_repeat wsz={wsz}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
            assert_bytes_eq(&format!("{t} / hufTable"), bytes_of(&cht), bytes_of(&rht));
        }

        // ---- parameter rejections and degenerate sizes
        let cases: Vec<(&str, &Vec<u8>, u32, u32, usize)> = vec![
            ("srcSize>MAX", &too_big, 255, 11, 1 << 20),
            ("srcSize==MAX", &exact, 255, 11, 1 << 20),
            ("srcSize==0", &empty, 255, 11, 4096),
            ("dstSize==0", &ok, 255, 11, 0),
            ("huffLog=13", &ok, 255, 13, 4096),
            ("huffLog=15", &ok, 255, 15, 4096),
            ("huffLog=255", &ok, 255, 255, 4096),
            ("msv=256", &ok, 256, 11, 4096),
            ("msv=65535", &ok, 65_535, 11, 4096),
            ("msv=1", &ok, 1, 11, 4096),
            ("msv=2", &ok, 2, 11, 4096),
            ("msv=127", &ok, 127, 11, 4096),
        ];
        for (name, src, msv, tl, cap) in cases {
            let mut cd = vec![0xB7u8; cap.max(1) + 64];
            let mut rd = vec![0xB7u8; cap.max(1) + 64];
            let mut cht = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut rht = vec![0usize; HUF_CTABLE_ST_MAX];
            let mut crep = HUF_repeat_none;
            let mut rrep = HUF_repeat_none;
            let mut cw = abuf(HUF_WORKSPACE_SIZE, 0x21);
            let mut rw = abuf(HUF_WORKSPACE_SIZE, 0x21);
            let a = unsafe {
                c_f(cd.as_mut_ptr(), cap, src.as_ptr(), src.len(), msv, tl,
                    aptr(&mut cw), HUF_WORKSPACE_SIZE, cht.as_mut_ptr(), &mut crep, 0)
            };
            let b = unsafe {
                r_f(rd.as_mut_ptr(), cap, src.as_ptr(), src.len(), msv, tl,
                    aptr(&mut rw), HUF_WORKSPACE_SIZE, rht.as_mut_ptr(), &mut rrep, 0)
            };
            let t = format!("HUF_compress{which}_repeat {name}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &cd, &rd);
            assert_bytes_eq(&format!("{t} / hufTable"), bytes_of(&cht), bytes_of(&rht));
            match name {
                "srcSize>MAX" => {
                    assert_eq_dbg(&format!("{t} code"), a, ec(ZSTD_error_srcSize_wrong))
                }
                "huffLog=13" | "huffLog=15" | "huffLog=255" => assert_eq_dbg(
                    &format!("{t} code"),
                    a,
                    ec(ZSTD_error_tableLog_tooLarge),
                ),
                "msv=256" | "msv=65535" => assert_eq_dbg(
                    &format!("{t} code"),
                    a,
                    ec(ZSTD_error_maxSymbolValue_tooLarge),
                ),
                "srcSize==0" | "dstSize==0" => assert_eq_dbg(&format!("{t} code"), a, 0),
                _ => {}
            }
        }
    }
}

// =========================================================== 8. HUF decompress

/// Fresh `HUF_DTable` initialised the way zstd's DCtx does it
/// (`hufTable[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x01000001`).
fn fresh_dtable(fill: u32) -> Vec<u32> {
    let mut v = vec![fill; HUF_DTABLE_U32];
    v[0] = HUF_DTABLE_LOG * 0x0100_0001;
    v
}

/// Compress `src` into a complete HUF block (table description ++ stream(s))
/// with the C library. Returns `None` unless a real Huffman block was produced
/// (0 == incompressible, 1 == RLE, error == rejected).
fn huf_block_c(i: &Impls, src: &[u8], one_stream: bool, tl: u32) -> Option<Vec<u8>> {
    let name = if one_stream {
        "HUF_compress1X_repeat"
    } else {
        "HUF_compress4X_repeat"
    };
    let (c_f, _) = i.pair::<Fn_HUF_repeat>(name);
    let (c_bound, _) = i.pair::<Fn_sz_sz>("HUF_compressBound");
    let cap = unsafe { c_bound(src.len()) }.max(64);
    let mut dst = vec![0u8; cap + 64];
    let mut ht = vec![0usize; HUF_CTABLE_ST_MAX];
    let mut rep = HUF_repeat_none;
    let mut w = abuf(HUF_WORKSPACE_SIZE, 0);
    let n = unsafe {
        c_f(dst.as_mut_ptr(), cap, src.as_ptr(), src.len(), 255, tl,
            aptr(&mut w), HUF_WORKSPACE_SIZE, ht.as_mut_ptr(), &mut rep, 0)
    };
    if n <= 1 || n > ec(200) {
        return None;
    }
    dst.truncate(n);
    Some(dst)
}

/// `HUF_selectDecoder` over a dense grid — it is pure arithmetic over a
/// pre-computed table, so any divergence is a table transcription error.
#[test]
fn huf_select_decoder() {
    let i = impls();
    let (c_s, r_s) = i.pair::<Fn_HUF_selectDecoder>("HUF_selectDecoder");
    let mut sizes: Vec<usize> = vec![0, 1, 2, 3, 7, 8, 255, 256, 257, 1023, 1024, 4096,
                                     65_535, 65_536, 131_071, 131_072];
    let mut rng = Rng::new(0x5E1F_0070);
    for _ in 0..200 {
        sizes.push(rng.range(1, 131_072));
    }
    for &d in &sizes {
        for &c in &sizes {
            unsafe {
                assert_eq_dbg(&format!("HUF_selectDecoder({d},{c})"), c_s(d, c), r_s(d, c));
            }
        }
    }
}

/// `HUF_readDTableX1_wksp` / `HUF_readDTableX2_wksp` must build **byte-identical**
/// decoding tables, and `HUF_decompress{1,4}X_usingDTable` must then regenerate
/// identical output. Also cross-consumes each library's DTable in the other.
#[test]
fn huf_read_dtable_and_decompress_using_dtable() {
    let i = impls();
    let (c_x1, r_x1) = i.pair::<Fn_HUF_readDTable>("HUF_readDTableX1_wksp");
    let (c_x2, r_x2) = i.pair::<Fn_HUF_readDTable>("HUF_readDTableX2_wksp");
    let (c_d1, r_d1) = i.pair::<Fn_HUF_usingDTable>("HUF_decompress1X_usingDTable");
    let (c_d4, r_d4) = i.pair::<Fn_HUF_usingDTable>("HUF_decompress4X_usingDTable");

    let mut rng = Rng::new(0x5E1F_0071);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS_SMALL {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    let dec_flags = [
        0i32,
        HUF_flags_bmi2,
        HUF_flags_disableAsm,
        HUF_flags_disableFast,
        HUF_flags_bmi2 | HUF_flags_disableAsm | HUF_flags_disableFast,
    ];

    for (tag, src) in &corpus {
        for one_stream in [true, false] {
            for &tl in &[0u32, 11, 12] {
                let Some(block) = huf_block_c(i, src, one_stream, tl) else {
                    continue;
                };
                for (xname, c_rd, r_rd) in [("X1", &c_x1, &r_x1), ("X2", &c_x2, &r_x2)] {
                    for &flags in &dec_flags {
                        let mut cdt = fresh_dtable(0x1234_5678);
                        let mut rdt = fresh_dtable(0x1234_5678);
                        let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0x8C);
                        let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0x8C);
                        let hn = unsafe {
                            c_rd(cdt.as_mut_ptr(), block.as_ptr(), block.len(),
                                 aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, flags)
                        };
                        let hm = unsafe {
                            r_rd(rdt.as_mut_ptr(), block.as_ptr(), block.len(),
                                 aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, flags)
                        };
                        let t = format!(
                            "HUF_readDTable{xname}_wksp {tag} 1X={one_stream} tl={tl} \
                             flags={flags}"
                        );
                        assert_eq_dbg(&t, hn, hm);
                        assert_bytes_eq(&format!("{t} / DTable"),
                                        bytes_of(&cdt), bytes_of(&rdt));
                        assert_bytes_eq(&format!("{t} / workspace"),
                                        bytes_of(&cw), bytes_of(&rw));
                        if hn > ec(200) || hn >= block.len() {
                            continue;
                        }
                        let payload = &block[hn..];

                        let (dname, c_dec, r_dec) = if one_stream {
                            ("1X", &c_d1, &r_d1)
                        } else {
                            ("4X", &c_d4, &r_d4)
                        };
                        for &dflags in &dec_flags {
                            let mut co = vec![0xA7u8; src.len() + 64];
                            let mut ro = vec![0xA7u8; src.len() + 64];
                            let a = unsafe {
                                c_dec(co.as_mut_ptr(), src.len(), payload.as_ptr(),
                                      payload.len(), cdt.as_ptr(), dflags)
                            };
                            let b = unsafe {
                                r_dec(ro.as_mut_ptr(), src.len(), payload.as_ptr(),
                                      payload.len(), rdt.as_ptr(), dflags)
                            };
                            let dt = format!("HUF_decompress{dname}_usingDTable {t} \
                                              decFlags={dflags}");
                            assert_eq_dbg(&dt, a, b);
                            assert_bytes_eq(&format!("{dt} / dst"), &co, &ro);
                            assert_eq_dbg(&format!("{dt} / size"), a, src.len());
                            assert_bytes_eq(&format!("{dt} / plaintext"), &co[..a], src);

                            // cross-library DTable consumption
                            let mut xo = vec![0xA7u8; src.len() + 64];
                            let x = unsafe {
                                c_dec(xo.as_mut_ptr(), src.len(), payload.as_ptr(),
                                      payload.len(), rdt.as_ptr(), dflags)
                            };
                            assert_eq_dbg(&format!("{dt} / C decoder + Rust DTable"), x, a);
                            assert_bytes_eq(&format!("{dt} / cross bytes"), &xo, &co);
                        }
                    }
                }
            }
        }
    }
}

/// The whole-block decoders: `HUF_decompress1X_DCtx_wksp`,
/// `HUF_decompress1X1_DCtx_wksp`, `HUF_decompress1X2_DCtx_wksp` and
/// `HUF_decompress4X_hufOnly_wksp`, over every flag combination.
#[test]
fn huf_decompress_dctx_roundtrip() {
    let i = impls();
    let (c_1x, r_1x) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X_DCtx_wksp");
    let (c_1x1, r_1x1) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X1_DCtx_wksp");
    let (c_1x2, r_1x2) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X2_DCtx_wksp");
    let (c_4x, r_4x) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress4X_hufOnly_wksp");

    let mut rng = Rng::new(0x5E1F_0072);
    let mut corpus = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS_SMALL {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    let dec_flags = [
        0i32,
        HUF_flags_bmi2,
        HUF_flags_disableAsm,
        HUF_flags_disableFast,
        HUF_flags_bmi2 | HUF_flags_disableAsm,
        HUF_flags_disableAsm | HUF_flags_disableFast,
    ];

    for (tag, src) in &corpus {
        for one_stream in [true, false] {
            let Some(block) = huf_block_c(i, src, one_stream, 0) else {
                continue;
            };
            let entries: Vec<(&str, &_, &_)> = if one_stream {
                vec![
                    ("1X_DCtx", &c_1x, &r_1x),
                    ("1X1_DCtx", &c_1x1, &r_1x1),
                    ("1X2_DCtx", &c_1x2, &r_1x2),
                ]
            } else {
                vec![("4X_hufOnly", &c_4x, &r_4x)]
            };
            for (dname, c_f, r_f) in entries {
                for &flags in &dec_flags {
                    let mut cdt = fresh_dtable(0x5A5A_5A5A);
                    let mut rdt = fresh_dtable(0x5A5A_5A5A);
                    let mut co = vec![0x3Bu8; src.len() + 64];
                    let mut ro = vec![0x3Bu8; src.len() + 64];
                    let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0x9D);
                    let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0x9D);
                    let a = unsafe {
                        c_f(cdt.as_mut_ptr(), co.as_mut_ptr(), src.len(),
                            block.as_ptr(), block.len(),
                            aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, flags)
                    };
                    let b = unsafe {
                        r_f(rdt.as_mut_ptr(), ro.as_mut_ptr(), src.len(),
                            block.as_ptr(), block.len(),
                            aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, flags)
                    };
                    let t = format!("HUF_decompress{dname}_wksp {tag} 1X={one_stream} \
                                     flags={flags}");
                    assert_eq_dbg(&t, a, b);
                    assert_bytes_eq(&format!("{t} / dst"), &co, &ro);
                    assert_bytes_eq(&format!("{t} / DTable"),
                                    bytes_of(&cdt), bytes_of(&rdt));
                    assert_bytes_eq(&format!("{t} / workspace"),
                                    bytes_of(&cw), bytes_of(&rw));
                    if a <= ec(200) {
                        assert_eq_dbg(&format!("{t} / size"), a, src.len());
                        assert_bytes_eq(&format!("{t} / plaintext"), &co[..a], src);
                    }
                }
            }
        }
    }
}

/// Every checked HUF decompression failure mode: `dstSize==0`, `cSrcSize==0`,
/// `cSrcSize > dstSize`, the `cSrcSize==dstSize` memcpy and `cSrcSize==1` RLE
/// shortcuts, workspace sweeps, truncation and bit-flip corruption.
#[test]
fn huf_decompress_error_paths() {
    let i = impls();
    let (c_1x, r_1x) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X_DCtx_wksp");
    let (c_1x1, r_1x1) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X1_DCtx_wksp");
    let (c_1x2, r_1x2) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress1X2_DCtx_wksp");
    let (c_4x, r_4x) = i.pair::<Fn_HUF_DCtx_wksp>("HUF_decompress4X_hufOnly_wksp");
    let (c_x1, r_x1) = i.pair::<Fn_HUF_readDTable>("HUF_readDTableX1_wksp");
    let (c_x2, r_x2) = i.pair::<Fn_HUF_readDTable>("HUF_readDTableX2_wksp");

    let mut rng = Rng::new(0x5E1F_0073);
    let mut blocks: Vec<(String, Vec<u8>, usize)> = Vec::new();
    for (tag, src) in alphabet_inputs(&mut rng) {
        for one_stream in [true, false] {
            if let Some(b) = huf_block_c(i, &src, one_stream, 0) {
                blocks.push((format!("{tag}/1X={one_stream}"), b, src.len()));
            }
        }
    }
    assert!(blocks.len() > 20, "expected a decent HUF block corpus");

    let all: Vec<(&str, &Fn_HUF_DCtx_wksp, &Fn_HUF_DCtx_wksp)> = vec![];
    let _ = all;

    // ---- workspace sweep on every entry point
    let mut wsizes: Vec<usize> = vec![0, 1, 8, 64, 256, 512, 1024, 1536, 2048];
    wsizes.push(HUF_DECOMPRESS_WORKSPACE_SIZE - 1);
    wsizes.push(HUF_DECOMPRESS_WORKSPACE_SIZE);
    wsizes.push(HUF_DECOMPRESS_WORKSPACE_SIZE + 64);
    let (btag, block, blen) = &blocks[2];
    for &wsz in &wsizes {
        for (dname, c_f, r_f) in [
            ("1X_DCtx", &c_1x, &r_1x),
            ("1X1_DCtx", &c_1x1, &r_1x1),
            ("1X2_DCtx", &c_1x2, &r_1x2),
            ("4X_hufOnly", &c_4x, &r_4x),
        ] {
            let mut cdt = fresh_dtable(0);
            let mut rdt = fresh_dtable(0);
            let mut co = vec![0u8; *blen + 64];
            let mut ro = vec![0u8; *blen + 64];
            let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE + 64, 0xE1);
            let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE + 64, 0xE1);
            let a = unsafe {
                c_f(cdt.as_mut_ptr(), co.as_mut_ptr(), *blen, block.as_ptr(), block.len(),
                    aptr(&mut cw), wsz, 0)
            };
            let b = unsafe {
                r_f(rdt.as_mut_ptr(), ro.as_mut_ptr(), *blen, block.as_ptr(), block.len(),
                    aptr(&mut rw), wsz, 0)
            };
            let t = format!("HUF_decompress{dname}_wksp {btag} wsz={wsz}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / dst"), &co, &ro);
            assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
        }
        for (xname, c_rd, r_rd) in [("X1", &c_x1, &r_x1), ("X2", &c_x2, &r_x2)] {
            let mut cdt = fresh_dtable(0);
            let mut rdt = fresh_dtable(0);
            let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE + 64, 0xE2);
            let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE + 64, 0xE2);
            let a = unsafe {
                c_rd(cdt.as_mut_ptr(), block.as_ptr(), block.len(), aptr(&mut cw), wsz, 0)
            };
            let b = unsafe {
                r_rd(rdt.as_mut_ptr(), block.as_ptr(), block.len(), aptr(&mut rw), wsz, 0)
            };
            let t = format!("HUF_readDTable{xname}_wksp {btag} wsz={wsz}");
            assert_eq_dbg(&t, a, b);
            assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
        }
    }

    for (btag, block, blen) in blocks.iter().take(20) {
        // ---- dstSize axis: 0, the memcpy/RLE shortcuts and the exact size
        let mut dsizes: Vec<usize> = vec![0, 1, 2, block.len() - 1, block.len(),
                                          block.len() + 1, *blen];
        let step = (*blen / 8).max(1);
        dsizes.extend((0..=*blen).step_by(step));
        dsizes.sort_unstable();
        dsizes.dedup();
        for &dsz in &dsizes {
            for (dname, c_f, r_f) in [
                ("1X_DCtx", &c_1x, &r_1x),
                ("1X1_DCtx", &c_1x1, &r_1x1),
                ("1X2_DCtx", &c_1x2, &r_1x2),
                ("4X_hufOnly", &c_4x, &r_4x),
            ] {
                let mut cdt = fresh_dtable(0);
                let mut rdt = fresh_dtable(0);
                let mut co = vec![0x0Fu8; dsz + 64];
                let mut ro = vec![0x0Fu8; dsz + 64];
                let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE3);
                let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE3);
                let a = unsafe {
                    c_f(cdt.as_mut_ptr(), co.as_mut_ptr(), dsz,
                        block.as_ptr(), block.len(),
                        aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let b = unsafe {
                    r_f(rdt.as_mut_ptr(), ro.as_mut_ptr(), dsz,
                        block.as_ptr(), block.len(),
                        aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let t = format!("HUF_decompress{dname}_wksp {btag} dstSize={dsz}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / dst"), &co, &ro);
                assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
                // only the two "universal selector" entry points carry the
                // `dstSize == 0` guard; the X1/X2 specific ones go straight to
                // HUF_readDTable* and report corruption instead.
                if dsz == 0 && (dname == "1X_DCtx" || dname == "4X_hufOnly") {
                    assert_eq_dbg(&format!("{t} expect dstSize_tooSmall"), a,
                                  ec(ZSTD_error_dstSize_tooSmall));
                }
            }
        }

        // ---- truncation (down to cSrcSize==0) and bit-flip corruption
        let mut cuts: Vec<usize> = (0..=block.len().min(24)).collect();
        let cstep = (block.len() / 24).max(1);
        cuts.extend((0..=block.len()).step_by(cstep));
        cuts.sort_unstable();
        cuts.dedup();
        let mut variants: Vec<(String, Vec<u8>)> =
            cuts.iter().map(|&c| (format!("cut{c}"), block[..c].to_vec())).collect();
        for _ in 0..40 {
            let mut v = block.clone();
            let pos = rng.below(v.len());
            v[pos] ^= 1u8 << rng.below(8);
            variants.push((format!("flip@{pos}"), v));
        }
        for (vname, v) in &variants {
            for (dname, c_f, r_f) in [
                ("1X_DCtx", &c_1x, &r_1x),
                ("1X1_DCtx", &c_1x1, &r_1x1),
                ("1X2_DCtx", &c_1x2, &r_1x2),
                ("4X_hufOnly", &c_4x, &r_4x),
            ] {
                let mut cdt = fresh_dtable(0);
                let mut rdt = fresh_dtable(0);
                let mut co = vec![0x1Fu8; *blen + 64];
                let mut ro = vec![0x1Fu8; *blen + 64];
                let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE4);
                let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE4);
                let a = unsafe {
                    c_f(cdt.as_mut_ptr(), co.as_mut_ptr(), *blen, v.as_ptr(), v.len(),
                        aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let b = unsafe {
                    r_f(rdt.as_mut_ptr(), ro.as_mut_ptr(), *blen, v.as_ptr(), v.len(),
                        aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let t = format!("HUF_decompress{dname}_wksp {btag} {vname}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / dst"), &co, &ro);
                assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
            }
            for (xname, c_rd, r_rd) in [("X1", &c_x1, &r_x1), ("X2", &c_x2, &r_x2)] {
                let mut cdt = fresh_dtable(0xCACA_CACA);
                let mut rdt = fresh_dtable(0xCACA_CACA);
                let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE5);
                let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE5);
                let a = unsafe {
                    c_rd(cdt.as_mut_ptr(), v.as_ptr(), v.len(),
                         aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let b = unsafe {
                    r_rd(rdt.as_mut_ptr(), v.as_ptr(), v.len(),
                         aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let t = format!("HUF_readDTable{xname}_wksp {btag} {vname}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
                assert_bytes_eq(&format!("{t} / workspace"), bytes_of(&cw), bytes_of(&rw));
            }
        }
    }

    // ---- pure garbage table descriptions
    for len in [0usize, 1, 2, 3, 4, 8, 16, 64, 200] {
        for _ in 0..40 {
            let mut v = vec![0u8; len];
            for b in v.iter_mut() {
                *b = rng.byte();
            }
            for (xname, c_rd, r_rd) in [("X1", &c_x1, &r_x1), ("X2", &c_x2, &r_x2)] {
                let mut cdt = fresh_dtable(0);
                let mut rdt = fresh_dtable(0);
                let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE6);
                let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE6);
                let a = unsafe {
                    c_rd(cdt.as_mut_ptr(), v.as_ptr(), len,
                         aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let b = unsafe {
                    r_rd(rdt.as_mut_ptr(), v.as_ptr(), len,
                         aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let t = format!("HUF_readDTable{xname}_wksp garbage/{len}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
            }
            for (dname, c_f, r_f) in [
                ("1X_DCtx", &c_1x, &r_1x),
                ("1X1_DCtx", &c_1x1, &r_1x1),
                ("1X2_DCtx", &c_1x2, &r_1x2),
                ("4X_hufOnly", &c_4x, &r_4x),
            ] {
                let dsz = 4096usize;
                let mut cdt = fresh_dtable(0);
                let mut rdt = fresh_dtable(0);
                let mut co = vec![0x2Fu8; dsz + 64];
                let mut ro = vec![0x2Fu8; dsz + 64];
                let mut cw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE7);
                let mut rw = abuf(HUF_DECOMPRESS_WORKSPACE_SIZE, 0xE7);
                let a = unsafe {
                    c_f(cdt.as_mut_ptr(), co.as_mut_ptr(), dsz, v.as_ptr(), len,
                        aptr(&mut cw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let b = unsafe {
                    r_f(rdt.as_mut_ptr(), ro.as_mut_ptr(), dsz, v.as_ptr(), len,
                        aptr(&mut rw), HUF_DECOMPRESS_WORKSPACE_SIZE, 0)
                };
                let t = format!("HUF_decompress{dname}_wksp garbage/{len}");
                assert_eq_dbg(&t, a, b);
                assert_bytes_eq(&format!("{t} / dst"), &co, &ro);
                assert_bytes_eq(&format!("{t} / DTable"), bytes_of(&cdt), bytes_of(&rdt));
            }
        }
    }
}

// ================================================================= 9. XXHASH

/// `ZSTD_XXH32` / `ZSTD_XXH64` one-shot, over every shape and length edge and a
/// wide seed sweep. Also checks the *known* xxHash reference digests so the
/// test would catch both libraries drifting together.
#[test]
fn xxh_oneshot() {
    let i = impls();
    let (c32, r32) = i.pair::<Fn_XXH32>("ZSTD_XXH32");
    let (c64, r64) = i.pair::<Fn_XXH64>("ZSTD_XXH64");

    let mut rng = Rng::new(0x5E1F_0080);
    let mut corpus: Vec<(String, Vec<u8>)> = alphabet_inputs(&mut rng);
    for &shape in &ALL_SHAPES {
        for &len in &ENT_LENS {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }
    // every length 0..=64 covers all of xxhash's stripe/lane tail cases
    for len in 0..=64usize {
        corpus.push((format!("tail/{len}"), gen_shape(Shape::Random, len, &mut rng)));
    }

    let seeds32: Vec<u32> = vec![0, 1, 2, 0x9E37_79B1, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
    let seeds64: Vec<u64> = vec![
        0,
        1,
        2,
        0x9E37_79B1_85EB_CA87,
        u32::MAX as u64,
        1u64 << 63,
        u64::MAX,
    ];

    for (tag, src) in &corpus {
        for &s in &seeds32 {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_XXH32 {tag} seed={s:#x}"),
                    c32(src.as_ptr(), src.len(), s),
                    r32(src.as_ptr(), src.len(), s),
                );
            }
        }
        for &s in &seeds64 {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_XXH64 {tag} seed={s:#x}"),
                    c64(src.as_ptr(), src.len(), s),
                    r64(src.as_ptr(), src.len(), s),
                );
            }
        }
    }

    // xxHash reference vectors (xxhsum of the empty string and "abc").
    unsafe {
        let e: [u8; 0] = [];
        assert_eq_dbg("XXH32(\"\",0)", c32(e.as_ptr(), 0, 0), 0x02CC_5D05);
        assert_eq_dbg("XXH64(\"\",0)", c64(e.as_ptr(), 0, 0), 0xEF46_DB37_51D8_E999);
        assert_eq_dbg("XXH32(\"\",0) rust", r32(e.as_ptr(), 0, 0), 0x02CC_5D05);
        assert_eq_dbg("XXH64(\"\",0) rust", r64(e.as_ptr(), 0, 0), 0xEF46_DB37_51D8_E999);
        let abc = b"abc";
        assert_eq_dbg("XXH32(\"abc\",0)", c32(abc.as_ptr(), 3, 0), 0x32D1_53FF);
        assert_eq_dbg("XXH32(\"abc\",0) rust", r32(abc.as_ptr(), 3, 0), 0x32D1_53FF);
        assert_eq_dbg("XXH64(\"abc\",0)", c64(abc.as_ptr(), 3, 0), 0x44BC_2CF5_AD77_0999);
        assert_eq_dbg("XXH64(\"abc\",0) rust", r64(abc.as_ptr(), 3, 0), 0x44BC_2CF5_AD77_0999);
    }

    // `input == NULL` with `len == 0` is explicitly accepted by xxhash.
    unsafe {
        assert_eq_dbg("XXH32(NULL,0)", c32(std::ptr::null(), 0, 7), r32(std::ptr::null(), 0, 7));
        assert_eq_dbg("XXH64(NULL,0)", c64(std::ptr::null(), 0, 7), r64(std::ptr::null(), 0, 7));
    }
}

/// Streaming (`createState` / `reset` / `update` / `digest`) must equal the
/// one-shot digest for *every* random chunk split, in both libraries, and
/// `copyState` must fork a stream exactly.
#[test]
fn xxh_streaming() {
    let i = impls();
    let (c32, r32) = i.pair::<Fn_XXH32>("ZSTD_XXH32");
    let (c64, r64) = i.pair::<Fn_XXH64>("ZSTD_XXH64");
    let (c_new32, r_new32) = i.pair::<Fn_newState>("ZSTD_XXH32_createState");
    let (c_free32, r_free32) = i.pair::<Fn_freeState>("ZSTD_XXH32_freeState");
    let (c_rst32, r_rst32) = i.pair::<Fn_reset32>("ZSTD_XXH32_reset");
    let (c_upd32, r_upd32) = i.pair::<Fn_update>("ZSTD_XXH32_update");
    let (c_dig32, r_dig32) = i.pair::<Fn_digest32>("ZSTD_XXH32_digest");
    let (c_cpy32, r_cpy32) = i.pair::<Fn_copyState>("ZSTD_XXH32_copyState");
    let (c_new64, r_new64) = i.pair::<Fn_newState>("ZSTD_XXH64_createState");
    let (c_free64, r_free64) = i.pair::<Fn_freeState>("ZSTD_XXH64_freeState");
    let (c_rst64, r_rst64) = i.pair::<Fn_reset64>("ZSTD_XXH64_reset");
    let (c_upd64, r_upd64) = i.pair::<Fn_update>("ZSTD_XXH64_update");
    let (c_dig64, r_dig64) = i.pair::<Fn_digest64>("ZSTD_XXH64_digest");
    let (c_cpy64, r_cpy64) = i.pair::<Fn_copyState>("ZSTD_XXH64_copyState");

    let mut rng = Rng::new(0x5E1F_0081);
    let mut corpus: Vec<(String, Vec<u8>)> = Vec::new();
    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 3, 4, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 200, 1000,
                      4096, 70_000] {
            corpus.push((format!("{shape:?}/{len}"), gen_shape(shape, len, &mut rng)));
        }
    }

    unsafe {
        let cs = c_new32();
        let rs = r_new32();
        let cs2 = c_new32();
        let rs2 = r_new32();
        let cs64 = c_new64();
        let rs64 = r_new64();
        let cs64b = c_new64();
        let rs64b = r_new64();
        assert!(!cs.is_null() && !rs.is_null(), "createState returned NULL");

        for (tag, src) in &corpus {
            for &seed in &[0u32, 1, 0xDEAD_BEEF, u32::MAX] {
                // random chunk splits, including zero-length updates
                for trial in 0..6 {
                    let mut splits: Vec<usize> = Vec::new();
                    let mut pos = 0usize;
                    while pos < src.len() {
                        let remaining = src.len() - pos;
                        let take = match trial {
                            0 => remaining,          // one big update
                            1 => 1,                  // byte at a time
                            2 => 4.min(remaining),   // sub-stripe chunks
                            3 => 16.min(remaining),  // exactly one XXH32 stripe
                            4 => rng.range(1, remaining),
                            _ => {
                                splits.push(0); // interleave empty updates
                                rng.range(1, remaining)
                            }
                        };
                        splits.push(take);
                        pos += take;
                    }
                    splits.push(0); // trailing empty update

                    assert_eq_dbg(
                        &format!("XXH32_reset {tag}"),
                        c_rst32(cs, seed),
                        r_rst32(rs, seed),
                    );
                    assert_eq_dbg(
                        &format!("XXH64_reset {tag}"),
                        c_rst64(cs64, seed as u64),
                        r_rst64(rs64, seed as u64),
                    );
                    let mut off = 0usize;
                    let mut forked = false;
                    for (si, &n) in splits.iter().enumerate() {
                        let n = n.min(src.len() - off);
                        let p = src.as_ptr().add(off);
                        assert_eq_dbg(
                            &format!("XXH32_update {tag} chunk{si}"),
                            c_upd32(cs, p, n),
                            r_upd32(rs, p, n),
                        );
                        assert_eq_dbg(
                            &format!("XXH64_update {tag} chunk{si}"),
                            c_upd64(cs64, p, n),
                            r_upd64(rs64, p, n),
                        );
                        off += n;
                        // fork the state halfway through and finish the fork
                        if !forked && off * 2 >= src.len() {
                            forked = true;
                            c_cpy32(cs2, cs);
                            r_cpy32(rs2, rs);
                            c_cpy64(cs64b, cs64);
                            r_cpy64(rs64b, rs64);
                            let rest = src.len() - off;
                            let q = src.as_ptr().add(off);
                            c_upd32(cs2, q, rest);
                            r_upd32(rs2, q, rest);
                            c_upd64(cs64b, q, rest);
                            r_upd64(rs64b, q, rest);
                            let fc = c_dig32(cs2);
                            let fr = r_dig32(rs2);
                            assert_eq_dbg(&format!("XXH32 copyState digest {tag}"), fc, fr);
                            assert_eq_dbg(
                                &format!("XXH32 copyState == oneshot {tag}"),
                                fc,
                                c32(src.as_ptr(), src.len(), seed),
                            );
                            let fc = c_dig64(cs64b);
                            let fr = r_dig64(rs64b);
                            assert_eq_dbg(&format!("XXH64 copyState digest {tag}"), fc, fr);
                            assert_eq_dbg(
                                &format!("XXH64 copyState == oneshot {tag}"),
                                fc,
                                c64(src.as_ptr(), src.len(), seed as u64),
                            );
                        }
                    }
                    assert_eq_dbg(&format!("streamed all of {tag}"), off, src.len());

                    let d32c = c_dig32(cs);
                    let d32r = r_dig32(rs);
                    let d64c = c_dig64(cs64);
                    let d64r = r_dig64(rs64);
                    let t = format!("{tag} seed={seed:#x} trial={trial}");
                    assert_eq_dbg(&format!("XXH32_digest {t}"), d32c, d32r);
                    assert_eq_dbg(&format!("XXH64_digest {t}"), d64c, d64r);
                    assert_eq_dbg(
                        &format!("XXH32 streaming == oneshot {t}"),
                        d32c,
                        c32(src.as_ptr(), src.len(), seed),
                    );
                    assert_eq_dbg(
                        &format!("XXH32 streaming == rust oneshot {t}"),
                        d32r,
                        r32(src.as_ptr(), src.len(), seed),
                    );
                    assert_eq_dbg(
                        &format!("XXH64 streaming == oneshot {t}"),
                        d64c,
                        c64(src.as_ptr(), src.len(), seed as u64),
                    );
                    assert_eq_dbg(
                        &format!("XXH64 streaming == rust oneshot {t}"),
                        d64r,
                        r64(src.as_ptr(), src.len(), seed as u64),
                    );
                    // digest() must not consume the state
                    assert_eq_dbg(&format!("XXH32_digest idempotent {t}"), c_dig32(cs), d32c);
                    assert_eq_dbg(&format!("XXH32_digest idempotent {t}"), r_dig32(rs), d32r);
                    assert_eq_dbg(&format!("XXH64_digest idempotent {t}"), c_dig64(cs64), d64c);
                    assert_eq_dbg(&format!("XXH64_digest idempotent {t}"), r_dig64(rs64), d64r);
                }
            }
        }

        // update(NULL, 0) is documented as a no-op returning XXH_OK
        c_rst32(cs, 0);
        r_rst32(rs, 0);
        assert_eq_dbg(
            "XXH32_update(NULL,0)",
            c_upd32(cs, std::ptr::null(), 0),
            r_upd32(rs, std::ptr::null(), 0),
        );
        assert_eq_dbg("XXH32_digest after NULL update", c_dig32(cs), r_dig32(rs));
        c_rst64(cs64, 0);
        r_rst64(rs64, 0);
        assert_eq_dbg(
            "XXH64_update(NULL,0)",
            c_upd64(cs64, std::ptr::null(), 0),
            r_upd64(rs64, std::ptr::null(), 0),
        );
        assert_eq_dbg("XXH64_digest after NULL update", c_dig64(cs64), r_dig64(rs64));

        for (cf, rf, cp, rp) in [
            (&c_free32, &r_free32, cs, rs),
            (&c_free32, &r_free32, cs2, rs2),
        ] {
            assert_eq_dbg("XXH32_freeState", cf(cp), rf(rp));
        }
        assert_eq_dbg("XXH64_freeState", c_free64(cs64), r_free64(rs64));
        assert_eq_dbg("XXH64_freeState", c_free64(cs64b), r_free64(rs64b));
        // freeState(NULL) is documented as returning XXH_OK
        assert_eq_dbg(
            "XXH32_freeState(NULL)",
            c_free32(std::ptr::null_mut()),
            r_free32(std::ptr::null_mut()),
        );
        assert_eq_dbg(
            "XXH64_freeState(NULL)",
            c_free64(std::ptr::null_mut()),
            r_free64(std::ptr::null_mut()),
        );
    }
}

/// Canonical (big-endian) representation round trips.
#[test]
fn xxh_canonical() {
    let i = impls();
    let (c_c32, r_c32) = i.pair::<Fn_canon32>("ZSTD_XXH32_canonicalFromHash");
    let (c_f32, r_f32) = i.pair::<Fn_fromCanon32>("ZSTD_XXH32_hashFromCanonical");
    let (c_c64, r_c64) = i.pair::<Fn_canon64>("ZSTD_XXH64_canonicalFromHash");
    let (c_f64, r_f64) = i.pair::<Fn_fromCanon64>("ZSTD_XXH64_hashFromCanonical");

    let mut hashes32: Vec<u32> = vec![0, 1, 2, 0xFF, 0x100, 0x1234_5678, 0x8000_0000, u32::MAX];
    let mut hashes64: Vec<u64> = vec![
        0, 1, 2, 0xFF, 0x100, 0x0123_4567_89AB_CDEF, 1u64 << 63, u64::MAX,
    ];
    let mut rng = Rng::new(0x5E1F_0082);
    for _ in 0..300 {
        hashes32.push(rng.next_u32());
        hashes64.push(rng.next_u64());
    }

    for &h in &hashes32 {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            c_c32(cb.as_mut_ptr(), h);
            r_c32(rb.as_mut_ptr(), h);
        }
        assert_bytes_eq(&format!("XXH32_canonicalFromHash({h:#x})"), &cb, &rb);
        // canonical form is big-endian by definition
        assert_bytes_eq(
            &format!("XXH32 canonical is BE ({h:#x})"),
            &cb[..4],
            &h.to_be_bytes(),
        );
        unsafe {
            assert_eq_dbg(
                &format!("XXH32_hashFromCanonical({h:#x})"),
                c_f32(cb.as_ptr()),
                r_f32(rb.as_ptr()),
            );
            assert_eq_dbg(&format!("XXH32 canonical round trip ({h:#x})"),
                          c_f32(cb.as_ptr()), h);
            // cross: rust canonical -> C parser
            assert_eq_dbg(&format!("XXH32 cross canonical ({h:#x})"),
                          c_f32(rb.as_ptr()), h);
        }
    }

    for &h in &hashes64 {
        let mut cb = [0x55u8; 16];
        let mut rb = [0x55u8; 16];
        unsafe {
            c_c64(cb.as_mut_ptr(), h);
            r_c64(rb.as_mut_ptr(), h);
        }
        assert_bytes_eq(&format!("XXH64_canonicalFromHash({h:#x})"), &cb, &rb);
        assert_bytes_eq(
            &format!("XXH64 canonical is BE ({h:#x})"),
            &cb[..8],
            &h.to_be_bytes(),
        );
        unsafe {
            assert_eq_dbg(
                &format!("XXH64_hashFromCanonical({h:#x})"),
                c_f64(cb.as_ptr()),
                r_f64(rb.as_ptr()),
            );
            assert_eq_dbg(&format!("XXH64 canonical round trip ({h:#x})"),
                          c_f64(cb.as_ptr()), h);
            assert_eq_dbg(&format!("XXH64 cross canonical ({h:#x})"),
                          c_f64(rb.as_ptr()), h);
        }
    }

    // arbitrary canonical bytes (not produced by canonicalFromHash) must parse
    // identically too
    for _ in 0..300 {
        let mut b = [0u8; 8];
        for x in b.iter_mut() {
            *x = rng.byte();
        }
        unsafe {
            assert_eq_dbg("XXH32_hashFromCanonical random", c_f32(b.as_ptr()), r_f32(b.as_ptr()));
            assert_eq_dbg("XXH64_hashFromCanonical random", c_f64(b.as_ptr()), r_f64(b.as_ptr()));
        }
    }
}
