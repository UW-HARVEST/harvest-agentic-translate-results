//! Group 10 — entropy / low-level codecs (CONFIGS.md rows 126-146).
//!
//! Every call goes through `dlsym` in BOTH `libzstd.so` builds and the results
//! (return values *and* every output buffer / table, in full) are compared
//! byte-for-byte.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};

// ===================================================================== consts

const FSE_MIN_TABLELOG: u32 = 5;
const FSE_MAX_TABLELOG: u32 = 12; // FSE_MAX_MEMORY_USAGE(14) - 2
const FSE_MAX_SYMBOL_VALUE: u32 = 255;

const HUF_TABLELOG_MAX: u32 = 12;
const HUF_TABLELOG_DEFAULT: u32 = 11;
const HUF_SYMBOLVALUE_MAX: u32 = 255;
const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = 4 * (HUF_SYMBOLVALUE_MAX as usize + 1) + 192;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);

const HUF_flags_bmi2: c_int = 1 << 0;
const HUF_flags_optimalDepth: c_int = 1 << 1;
const HUF_flags_preferRepeat: c_int = 1 << 2;
const HUF_flags_suspectUncompressible: c_int = 1 << 3;
const HUF_flags_disableAsm: c_int = 1 << 4;
const HUF_flags_disableFast: c_int = 1 << 5;

const HUF_repeat_none: c_int = 0;
const HUF_repeat_check: c_int = 1;
const HUF_repeat_valid: c_int = 2;

const FSE_repeat_none: c_int = 0;
const FSE_repeat_check: c_int = 1;
const FSE_repeat_valid: c_int = 2;

const set_basic: c_int = 0;
const set_rle: c_int = 1;
const set_compressed: c_int = 2;
const set_repeat: c_int = 3;

const MaxLL: u32 = 35;
const MaxML: u32 = 52;
const MaxOff: u32 = 31;
const MaxSeq: u32 = 52;
const LLFSELog: u32 = 9;
const MLFSELog: u32 = 9;
const OffFSELog: u32 = 8;
const MaxFSELog: u32 = 9;
const ZSTD_MAX_FSE_HEADERS_SIZE: usize = 133;
const ZSTD_MAX_HUF_HEADER_SIZE: usize = 128;

const fn fse_ctable_size_u32(tableLog: u32, maxSymbolValue: u32) -> usize {
    1 + (1usize << (tableLog - 1)) + ((maxSymbolValue as usize + 1) * 2)
}
const fn fse_dtable_size_u32(tableLog: u32) -> usize {
    1 + (1usize << tableLog)
}
fn fse_build_ctable_wksp_u32(maxSymbolValue: u32, tableLog: u32) -> usize {
    ((maxSymbolValue as usize + 2) + (1usize << tableLog)) / 2 + 2
}
fn fse_build_dtable_wksp_u32(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    let b = 2 * (maxSymbolValue as usize + 1) + (1usize << maxTableLog) + 8;
    (b + 3) / 4
}
fn fse_decompress_wksp_u32(maxLog: u32, maxSymbolValue: u32) -> usize {
    fse_dtable_size_u32(maxLog)
        + 1
        + fse_build_dtable_wksp_u32(maxLog, maxSymbolValue)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}

const OFF_CTABLE_U32: usize = fse_ctable_size_u32(OffFSELog, MaxOff);
const ML_CTABLE_U32: usize = fse_ctable_size_u32(MLFSELog, MaxML);
const LL_CTABLE_U32: usize = fse_ctable_size_u32(LLFSELog, MaxLL);

// ================================================================= fn types

type FnU32Hash = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
type FnU64Hash = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnNewState = unsafe extern "C" fn() -> *mut c_void;
type FnFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnReset32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnCanon32 = unsafe extern "C" fn(*mut c_void, u32);
type FnCanon64 = unsafe extern "C" fn(*mut c_void, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const c_void) -> u64;

type FnHistCount = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnHistCountWksp =
    unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize, *mut c_void, usize) -> usize;
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
type FnReadNCountBmi2 =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize, c_int) -> usize;
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

// ================================================================== helpers

/// 0xA5-prefilled twin output buffers.
fn twin(n: usize) -> (Vec<u8>, Vec<u8>) {
    (vec![0xA5u8; n], vec![0xA5u8; n])
}
fn twin32(n: usize) -> (Vec<u32>, Vec<u32>) {
    (vec![0xA5A5A5A5u32; n], vec![0xA5A5A5A5u32; n])
}
fn twin16(n: usize) -> (Vec<i16>, Vec<i16>) {
    (vec![0x5A5Ai16; n], vec![0x5A5Ai16; n])
}

fn as_bytes32(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}
fn as_bytes16(v: &[i16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) }
}
fn as_bytes64(v: &[u64]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) }
}
fn raw_bytes<T>(v: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// Histogram of `src` restricted to `maxSymbolValue`, computed by the *C* lib.
unsafe fn c_hist(src: &[u8], maxSymbolValue: u32) -> (Vec<c_uint>, u32, usize) {
    let (fc, _) = duo::<FnHistCountWksp>("HIST_countFast_wksp");
    let mut count = vec![0u32; 256];
    let mut msv: c_uint = maxSymbolValue;
    let mut wksp = vec![0u32; 1024];
    let r = fc(
        count.as_mut_ptr(),
        &mut msv,
        src.as_ptr() as *const c_void,
        src.len(),
        wksp.as_mut_ptr() as *mut c_void,
        wksp.len() * 4,
    );
    (count, msv, r)
}

// ============================================================ rows 126 / 127

const XXH32_STATE_SIZE: usize = 48;
const XXH64_STATE_SIZE: usize = 88;

/// Sizes covering every alignment class and the 16-byte-block boundaries.
fn xxh_sizes() -> Vec<usize> {
    let mut v: Vec<usize> = (0usize..40).collect();
    v.extend_from_slice(&[
        63, 64, 65, 127, 128, 129, 200, 255, 256, 257, 511, 512, 513, 1000, 1023, 1024,
    ]);
    v
}

#[test]
fn row126_xxh32_family() {
    unsafe {
        let (h_c, h_r) = duo::<FnU32Hash>("ZSTD_XXH32");
        let (cs_c, cs_r) = duo::<FnNewState>("ZSTD_XXH32_createState");
        let (fs_c, fs_r) = duo::<FnFreeState>("ZSTD_XXH32_freeState");
        let (rs_c, rs_r) = duo::<FnReset32>("ZSTD_XXH32_reset");
        let (up_c, up_r) = duo::<FnUpdate>("ZSTD_XXH32_update");
        let (dg_c, dg_r) = duo::<FnDigest32>("ZSTD_XXH32_digest");
        let (cp_c, cp_r) = duo::<FnCopyState>("ZSTD_XXH32_copyState");
        let (cf_c, cf_r) = duo::<FnCanon32>("ZSTD_XXH32_canonicalFromHash");
        let (fc_c, fc_r) = duo::<FnFromCanon32>("ZSTD_XXH32_hashFromCanonical");

        let seeds: [u32; 4] = [0, 1, 0xFFFF_FFFF, 0x9E37_79B9];
        // one big backing buffer so we can slice at every alignment
        let backing = gen_class(3, 1024 + 32, 0xC0FFEE);
        let mut rng = Rng::new(126);

        let sc = cs_c();
        let sr = cs_r();
        let dc = cs_c();
        let dr = cs_r();
        assert!(!sc.is_null() && !sr.is_null() && !dc.is_null() && !dr.is_null());

        for &seed in &seeds {
            for size in xxh_sizes() {
                for align in 0..8usize {
                    let src = &backing[align..align + size];
                    // one-shot
                    eqv(
                        &format!("ZSTD_XXH32(seed={seed},size={size},align={align})"),
                        h_c(src.as_ptr() as *const c_void, size, seed),
                        h_r(src.as_ptr() as *const c_void, size, seed),
                    );
                }
            }
            // streaming with many chunkings
            for class in 0..N_CLASSES {
                for &size in &[0usize, 1, 3, 4, 15, 16, 17, 31, 32, 100, 511, 1024] {
                    let data = gen_class(class, size, 0x126 ^ seed as u64);
                    for trial in 0..4 {
                        eqv(
                            &format!("XXH32_reset(seed={seed})"),
                            rs_c(sc, seed),
                            rs_r(sr, seed),
                        );
                        eqbuf(
                            "XXH32 state after reset",
                            std::slice::from_raw_parts(sc as *const u8, XXH32_STATE_SIZE),
                            std::slice::from_raw_parts(sr as *const u8, XXH32_STATE_SIZE),
                        );
                        let mut pos = 0usize;
                        let mut nchunk = 0;
                        while pos < data.len() {
                            let want = match trial {
                                0 => data.len(),
                                1 => 1,
                                2 => 1 + rng.below(7),
                                _ => 1 + rng.below(23),
                            };
                            let n = want.min(data.len() - pos);
                            let p = data[pos..].as_ptr() as *const c_void;
                            eqv(
                                &format!("XXH32_update(chunk={nchunk},n={n})"),
                                up_c(sc, p, n),
                                up_r(sr, p, n),
                            );
                            eqbuf(
                                &format!("XXH32 state after update {nchunk}"),
                                std::slice::from_raw_parts(sc as *const u8, XXH32_STATE_SIZE),
                                std::slice::from_raw_parts(sr as *const u8, XXH32_STATE_SIZE),
                            );
                            pos += n;
                            nchunk += 1;
                        }
                        let gc = dg_c(sc);
                        let gr = dg_r(sr);
                        eqv(
                            &format!("XXH32_digest(class={class},size={size},trial={trial})"),
                            gc,
                            gr,
                        );
                        eqv(
                            "XXH32 streaming == one-shot",
                            gc,
                            h_c(data.as_ptr() as *const c_void, data.len(), seed),
                        );
                        // copyState then digest again from the copy
                        cp_c(dc, sc);
                        cp_r(dr, sr);
                        eqbuf(
                            "XXH32_copyState",
                            std::slice::from_raw_parts(dc as *const u8, XXH32_STATE_SIZE),
                            std::slice::from_raw_parts(dr as *const u8, XXH32_STATE_SIZE),
                        );
                        eqv("XXH32_digest(copy)", dg_c(dc), dg_r(dr));
                        // canonical round-trip
                        let (mut kc, mut kr) = twin(4);
                        cf_c(kc.as_mut_ptr() as *mut c_void, gc);
                        cf_r(kr.as_mut_ptr() as *mut c_void, gr);
                        eqbuf("XXH32_canonicalFromHash", &kc, &kr);
                        eqv(
                            "XXH32_hashFromCanonical",
                            fc_c(kc.as_ptr() as *const c_void),
                            fc_r(kr.as_ptr() as *const c_void),
                        );
                        eqv(
                            "XXH32 canonical round-trip",
                            fc_c(kc.as_ptr() as *const c_void),
                            gc,
                        );
                    }
                }
            }
        }
        // canonicalFromHash over a wide value sweep
        for i in 0..512u32 {
            let v = i
                .wrapping_mul(0x9E37_79B1)
                .rotate_left((i % 31) as u32)
                ^ i;
            let (mut kc, mut kr) = twin(4);
            cf_c(kc.as_mut_ptr() as *mut c_void, v);
            cf_r(kr.as_mut_ptr() as *mut c_void, v);
            eqbuf(&format!("XXH32_canonicalFromHash({v})"), &kc, &kr);
            let raw = v.to_le_bytes();
            eqv(
                &format!("XXH32_hashFromCanonical(raw {v})"),
                fc_c(raw.as_ptr() as *const c_void),
                fc_r(raw.as_ptr() as *const c_void),
            );
        }
        eqv("XXH32_freeState", fs_c(sc), fs_r(sr));
        eqv("XXH32_freeState(dst)", fs_c(dc), fs_r(dr));
    }
}

#[test]
fn row127_xxh64_family() {
    unsafe {
        let (h_c, h_r) = duo::<FnU64Hash>("ZSTD_XXH64");
        let (cs_c, cs_r) = duo::<FnNewState>("ZSTD_XXH64_createState");
        let (fs_c, fs_r) = duo::<FnFreeState>("ZSTD_XXH64_freeState");
        let (rs_c, rs_r) = duo::<FnReset64>("ZSTD_XXH64_reset");
        let (up_c, up_r) = duo::<FnUpdate>("ZSTD_XXH64_update");
        let (dg_c, dg_r) = duo::<FnDigest64>("ZSTD_XXH64_digest");
        let (cp_c, cp_r) = duo::<FnCopyState>("ZSTD_XXH64_copyState");
        let (cf_c, cf_r) = duo::<FnCanon64>("ZSTD_XXH64_canonicalFromHash");
        let (fc_c, fc_r) = duo::<FnFromCanon64>("ZSTD_XXH64_hashFromCanonical");
        let (vn_c, vn_r) = duo::<FnUint0>("ZSTD_XXH_versionNumber");
        eqv("ZSTD_XXH_versionNumber", vn_c(), vn_r());

        let seeds: [u64; 4] = [0, 1, u64::MAX, 0x0123_4567_89AB_CDEF];
        let backing = gen_class(7, 1024 + 32, 0xBADF00D);
        let mut rng = Rng::new(127);

        let sc = cs_c();
        let sr = cs_r();
        let dc = cs_c();
        let dr = cs_r();
        assert!(!sc.is_null() && !sr.is_null() && !dc.is_null() && !dr.is_null());

        for &seed in &seeds {
            for size in xxh_sizes() {
                for align in 0..8usize {
                    let src = &backing[align..align + size];
                    eqv(
                        &format!("ZSTD_XXH64(seed={seed},size={size},align={align})"),
                        h_c(src.as_ptr() as *const c_void, size, seed),
                        h_r(src.as_ptr() as *const c_void, size, seed),
                    );
                }
            }
            for class in 0..N_CLASSES {
                for &size in &[0usize, 1, 7, 8, 31, 32, 33, 63, 64, 200, 1024] {
                    let data = gen_class(class, size, 0x127 ^ seed);
                    for trial in 0..4 {
                        eqv("XXH64_reset", rs_c(sc, seed), rs_r(sr, seed));
                        eqbuf(
                            "XXH64 state after reset",
                            std::slice::from_raw_parts(sc as *const u8, XXH64_STATE_SIZE),
                            std::slice::from_raw_parts(sr as *const u8, XXH64_STATE_SIZE),
                        );
                        let mut pos = 0usize;
                        let mut nchunk = 0;
                        while pos < data.len() {
                            let want = match trial {
                                0 => data.len(),
                                1 => 1,
                                2 => 1 + rng.below(11),
                                _ => 1 + rng.below(37),
                            };
                            let n = want.min(data.len() - pos);
                            let p = data[pos..].as_ptr() as *const c_void;
                            eqv(
                                &format!("XXH64_update(chunk={nchunk},n={n})"),
                                up_c(sc, p, n),
                                up_r(sr, p, n),
                            );
                            eqbuf(
                                &format!("XXH64 state after update {nchunk}"),
                                std::slice::from_raw_parts(sc as *const u8, XXH64_STATE_SIZE),
                                std::slice::from_raw_parts(sr as *const u8, XXH64_STATE_SIZE),
                            );
                            pos += n;
                            nchunk += 1;
                        }
                        let gc = dg_c(sc);
                        let gr = dg_r(sr);
                        eqv(
                            &format!("XXH64_digest(class={class},size={size},trial={trial})"),
                            gc,
                            gr,
                        );
                        eqv(
                            "XXH64 streaming == one-shot",
                            gc,
                            h_c(data.as_ptr() as *const c_void, data.len(), seed),
                        );
                        cp_c(dc, sc);
                        cp_r(dr, sr);
                        eqbuf(
                            "XXH64_copyState",
                            std::slice::from_raw_parts(dc as *const u8, XXH64_STATE_SIZE),
                            std::slice::from_raw_parts(dr as *const u8, XXH64_STATE_SIZE),
                        );
                        eqv("XXH64_digest(copy)", dg_c(dc), dg_r(dr));
                        let (mut kc, mut kr) = twin(8);
                        cf_c(kc.as_mut_ptr() as *mut c_void, gc);
                        cf_r(kr.as_mut_ptr() as *mut c_void, gr);
                        eqbuf("XXH64_canonicalFromHash", &kc, &kr);
                        eqv(
                            "XXH64_hashFromCanonical",
                            fc_c(kc.as_ptr() as *const c_void),
                            fc_r(kr.as_ptr() as *const c_void),
                        );
                        eqv(
                            "XXH64 canonical round-trip",
                            fc_c(kc.as_ptr() as *const c_void),
                            gc,
                        );
                    }
                }
            }
        }
        let mut rng2 = Rng::new(0xB16B00B5);
        for _ in 0..512 {
            let v = rng2.next_u64();
            let (mut kc, mut kr) = twin(8);
            cf_c(kc.as_mut_ptr() as *mut c_void, v);
            cf_r(kr.as_mut_ptr() as *mut c_void, v);
            eqbuf(&format!("XXH64_canonicalFromHash({v})"), &kc, &kr);
            let raw = v.to_le_bytes();
            eqv(
                &format!("XXH64_hashFromCanonical(raw {v})"),
                fc_c(raw.as_ptr() as *const c_void),
                fc_r(raw.as_ptr() as *const c_void),
            );
        }
        eqv("XXH64_freeState", fs_c(sc), fs_r(sr));
        eqv("XXH64_freeState(dst)", fs_c(dc), fs_r(dr));
    }
}

// ==================================================================== row 128

#[test]
fn row128_hist() {
    unsafe {
        let (hc_c, hc_r) = duo::<FnHistCount>("HIST_count");
        let (hf_c, hf_r) = duo::<FnHistCount>("HIST_countFast");
        let (hcw_c, hcw_r) = duo::<FnHistCountWksp>("HIST_count_wksp");
        let (hfw_c, hfw_r) = duo::<FnHistCountWksp>("HIST_countFast_wksp");
        let (hs_c, hs_r) = duo::<FnHistSimple>("HIST_count_simple");
        let (ha_c, ha_r) = duo::<FnHistAdd>("HIST_add");
        let (ie_c, ie_r) = duo::<FnIsError>("HIST_isError");

        let msvs: [u32; 7] = [0, 1, 15, 63, 127, 254, 255];
        let sizes: [usize; 9] = [0, 1, 2, 7, 15, 128, 1024, 8 * 1024, 64 * 1024];

        for class in 0..N_CLASSES {
            for &size in &sizes {
                let data = gen_class(class, size, 0x128);
                for &msv in &msvs {
                    let tag = format!("class={} size={size} msv={msv}", CLASS_NAMES[class]);
                    // `HIST_countFast*` and `HIST_count_simple` are documented as
                    // "unsafe: won't check if src contains values beyond count[]
                    // limit", so they are only fed alphabet-conforming data.
                    let masked: Vec<u8> =
                        data.iter().map(|&b| (b as u32 % (msv + 1)) as u8).collect();
                    // ---- HIST_count / HIST_count_wksp (checked variants)
                    for which in 0..2 {
                        let (mut cc, mut cr) = twin32(256);
                        let mut mc: c_uint = msv;
                        let mut mr: c_uint = msv;
                        let mut wc = vec![0u32; 1024];
                        let mut wr = vec![0u32; 1024];
                        let (rc, rr) = if which == 0 {
                            (
                                hc_c(
                                    cc.as_mut_ptr(),
                                    &mut mc,
                                    data.as_ptr() as *const c_void,
                                    size,
                                ),
                                hc_r(
                                    cr.as_mut_ptr(),
                                    &mut mr,
                                    data.as_ptr() as *const c_void,
                                    size,
                                ),
                            )
                        } else {
                            (
                                hcw_c(
                                    cc.as_mut_ptr(),
                                    &mut mc,
                                    data.as_ptr() as *const c_void,
                                    size,
                                    wc.as_mut_ptr() as *mut c_void,
                                    wc.len() * 4,
                                ),
                                hcw_r(
                                    cr.as_mut_ptr(),
                                    &mut mr,
                                    data.as_ptr() as *const c_void,
                                    size,
                                    wr.as_mut_ptr() as *mut c_void,
                                    wr.len() * 4,
                                ),
                            )
                        };
                        let n = if which == 0 { "HIST_count" } else { "HIST_count_wksp" };
                        eqv(&format!("{n} ret {tag}"), rc, rr);
                        eqv(&format!("{n} maxSymbolValue {tag}"), mc, mr);
                        eqv(&format!("{n} isError {tag}"), ie_c(rc), ie_r(rr));
                        if !is_err(rc) {
                            eqbuf(&format!("{n} count[] {tag}"), as_bytes32(&cc), as_bytes32(&cr));
                        }
                    }
                    // ---- HIST_countFast / HIST_countFast_wksp (unchecked)
                    for which in 0..2 {
                        let (mut cc, mut cr) = twin32(256);
                        let mut mc: c_uint = msv;
                        let mut mr: c_uint = msv;
                        let mut wc = vec![0u32; 1024];
                        let mut wr = vec![0u32; 1024];
                        let (rc, rr) = if which == 0 {
                            (
                                hf_c(
                                    cc.as_mut_ptr(),
                                    &mut mc,
                                    masked.as_ptr() as *const c_void,
                                    size,
                                ),
                                hf_r(
                                    cr.as_mut_ptr(),
                                    &mut mr,
                                    masked.as_ptr() as *const c_void,
                                    size,
                                ),
                            )
                        } else {
                            (
                                hfw_c(
                                    cc.as_mut_ptr(),
                                    &mut mc,
                                    masked.as_ptr() as *const c_void,
                                    size,
                                    wc.as_mut_ptr() as *mut c_void,
                                    wc.len() * 4,
                                ),
                                hfw_r(
                                    cr.as_mut_ptr(),
                                    &mut mr,
                                    masked.as_ptr() as *const c_void,
                                    size,
                                    wr.as_mut_ptr() as *mut c_void,
                                    wr.len() * 4,
                                ),
                            )
                        };
                        let n = if which == 0 {
                            "HIST_countFast"
                        } else {
                            "HIST_countFast_wksp"
                        };
                        eqv(&format!("{n} ret {tag}"), rc, rr);
                        eqv(&format!("{n} maxSymbolValue {tag}"), mc, mr);
                        eqbuf(&format!("{n} count[] {tag}"), as_bytes32(&cc), as_bytes32(&cr));
                    }
                    // ---- HIST_count_simple : all symbols must be <= msv
                    {
                        let (mut cc, mut cr) = twin32(256);
                        let mut mc: c_uint = msv;
                        let mut mr: c_uint = msv;
                        let rc = hs_c(
                            cc.as_mut_ptr(),
                            &mut mc,
                            masked.as_ptr() as *const c_void,
                            masked.len(),
                        );
                        let rr = hs_r(
                            cr.as_mut_ptr(),
                            &mut mr,
                            masked.as_ptr() as *const c_void,
                            masked.len(),
                        );
                        eqv(&format!("HIST_count_simple ret {tag}"), rc, rr);
                        eqv(&format!("HIST_count_simple msv {tag}"), mc, mr);
                        eqbuf(
                            &format!("HIST_count_simple count[] {tag}"),
                            as_bytes32(&cc),
                            as_bytes32(&cr),
                        );
                    }
                    // ---- HIST_add (accumulating, called twice)
                    {
                        let (mut cc, mut cr) = twin32(256);
                        for v in cc.iter_mut() {
                            *v = 0;
                        }
                        for v in cr.iter_mut() {
                            *v = 0;
                        }
                        ha_c(cc.as_mut_ptr(), data.as_ptr() as *const c_void, size);
                        ha_r(cr.as_mut_ptr(), data.as_ptr() as *const c_void, size);
                        ha_c(cc.as_mut_ptr(), data.as_ptr() as *const c_void, size);
                        ha_r(cr.as_mut_ptr(), data.as_ptr() as *const c_void, size);
                        eqbuf(&format!("HIST_add {tag}"), as_bytes32(&cc), as_bytes32(&cr));
                    }
                }
            }
        }
        // HIST_isError over the whole error range
        for code in 0..140usize {
            let v = usize::MAX - code;
            eqv(&format!("HIST_isError({v})"), ie_c(v), ie_r(v));
        }
        for code in 0..64usize {
            eqv(&format!("HIST_isError({code})"), ie_c(code), ie_r(code));
        }
    }
}

// ============================================================ rows 129 - 135

/// Everything the FSE public/advanced API exposes in this build, driven as a
/// chain: histogram -> normalizeCount -> writeNCount -> readNCount ->
/// buildCTable -> compress_usingCTable -> buildDTable -> decompress_wksp.
#[test]
fn row131_135_fse_pipeline() {
    unsafe {
        let (otl_c, otl_r) = duo::<FnOptimalTableLog>("FSE_optimalTableLog");
        let (oti_c, oti_r) = duo::<FnOptimalTableLogInt>("FSE_optimalTableLog_internal");
        let (nc_c, nc_r) = duo::<FnNormalizeCount>("FSE_normalizeCount");
        let (wb_c, wb_r) = duo::<FnNCountWriteBound>("FSE_NCountWriteBound");
        let (wn_c, wn_r) = duo::<FnWriteNCount>("FSE_writeNCount");
        let (rn_c, rn_r) = duo::<FnReadNCount>("FSE_readNCount");
        let (rb_c, rb_r) = duo::<FnReadNCountBmi2>("FSE_readNCount_bmi2");
        let (bc_c, bc_r) = duo::<FnBuildCTableWksp>("FSE_buildCTable_wksp");
        let (br_c, br_r) = duo::<FnBuildCTableRle>("FSE_buildCTable_rle");
        let (cu_c, cu_r) = duo::<FnCompressUsingCTable>("FSE_compress_usingCTable");
        let (bd_c, bd_r) = duo::<FnBuildDTableWksp>("FSE_buildDTable_wksp");
        let (dw_c, dw_r) = duo::<FnDecompressWkspBmi2>("FSE_decompress_wksp_bmi2");
        let (cb_c, cb_r) = duo::<FnSizeT1>("FSE_compressBound");

        // ---- FSE_optimalTableLog{,_internal} grid.
        // NOTE: the C implementation feeds `srcSize-1` and `maxSymbolValue` to
        // ZSTD_highbit32(), which is documented as `assert(val != 0)`; srcSize<2
        // or maxSymbolValue==0 is therefore undefined in C (bsr on 0) and is
        // excluded from the grid.
        for &maxTableLog in &[0u32, 1, 5, 6, 9, 11, 12, 13, 15] {
            for &srcSize in &[2usize, 3, 4, 10, 100, 1000, 65536, 1 << 20] {
                for &msv in &[1u32, 3, 15, 35, 52, 127, 255] {
                    eqv(
                        &format!("FSE_optimalTableLog({maxTableLog},{srcSize},{msv})"),
                        otl_c(maxTableLog, srcSize, msv),
                        otl_r(maxTableLog, srcSize, msv),
                    );
                    for minus in 0..4u32 {
                        eqv(
                            &format!(
                                "FSE_optimalTableLog_internal({maxTableLog},{srcSize},{msv},{minus})"
                            ),
                            oti_c(maxTableLog, srcSize, msv, minus),
                            oti_r(maxTableLog, srcSize, msv, minus),
                        );
                    }
                }
            }
        }
        // ---- FSE_NCountWriteBound / FSE_compressBound
        for msv in 0..=255u32 {
            for tl in 0..=15u32 {
                eqv(
                    &format!("FSE_NCountWriteBound({msv},{tl})"),
                    wb_c(msv, tl),
                    wb_r(msv, tl),
                );
            }
        }
        for &n in &[0usize, 1, 7, 128, 1024, 65535, 1 << 20] {
            eqv(&format!("FSE_compressBound({n})"), cb_c(n), cb_r(n));
        }

        // ---- FSE_buildCTable_rle over every symbol value
        for sym in 0..=255u8 {
            let (mut tc, mut tr) = twin32(fse_ctable_size_u32(1, 255));
            let rc = br_c(tc.as_mut_ptr(), sym);
            let rr = br_r(tr.as_mut_ptr(), sym);
            eqv(&format!("FSE_buildCTable_rle({sym}) ret"), rc, rr);
            eqbuf(
                &format!("FSE_buildCTable_rle({sym}) table"),
                as_bytes32(&tc),
                as_bytes32(&tr),
            );
        }

        // ---- the full pipeline
        let mut rng = Rng::new(131);
        for class in 0..N_CLASSES {
            for &size in &[0usize, 1, 2, 7, 63, 128, 200, 1024, 4096, 8 * 1024, 64 * 1024] {
              for dseed in 0..2u64 {
                let data = gen_class(class, size, 0x131 ^ (dseed << 24));
                for &msv_in in &[1u32, 15, 63, 127, 255] {
                    let masked: Vec<u8> = data
                        .iter()
                        .map(|&b| (b as u32 % (msv_in + 1)) as u8)
                        .collect();
                    let (count, msv, _maxCount) = c_hist(&masked, msv_in);
                    // maxSymbolValue==0 (single-symbol alphabet) reaches
                    // ZSTD_highbit32(0) inside FSE_minTableLog(), which is UB in
                    // C; real zstd uses RLE mode instead. Excluded.
                    if msv == 0 || masked.is_empty() {
                        continue;
                    }
                    for &tableLog in &[0u32, 4, 5, 6, 8, 9, 11, 12, 13] {
                        for &useLowProb in &[0u32, 1] {
                            let tag = format!(
                                "class={} size={size} msvIn={msv_in} tl={tableLog} lp={useLowProb}",
                                CLASS_NAMES[class]
                            );
                            let (mut nrmc, mut nrmr) = twin16(256);
                            let rc = nc_c(
                                nrmc.as_mut_ptr(),
                                tableLog,
                                count.as_ptr(),
                                masked.len(),
                                msv,
                                useLowProb,
                            );
                            let rr = nc_r(
                                nrmr.as_mut_ptr(),
                                tableLog,
                                count.as_ptr(),
                                masked.len(),
                                msv,
                                useLowProb,
                            );
                            eqv(&format!("FSE_normalizeCount ret {tag}"), rc, rr);
                            eqbuf(
                                &format!("FSE_normalizeCount norm[] {tag}"),
                                as_bytes16(&nrmc),
                                as_bytes16(&nrmr),
                            );
                            if is_err(rc) {
                                continue;
                            }
                            let tl = rc as u32;
                            if tl < FSE_MIN_TABLELOG || tl > FSE_MAX_TABLELOG {
                                continue;
                            }
                            let norm = &nrmc[..];

                            // ---- writeNCount
                            let bound = wb_c(msv, tl);
                            for shrink in 0..3usize {
                                let cap = match shrink {
                                    0 => bound,
                                    1 => bound + 16,
                                    _ => bound / 2,
                                };
                                let (mut hc, mut hr) = twin(cap.max(1));
                                let wc = wn_c(
                                    hc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                );
                                let wr = wn_r(
                                    hr.as_mut_ptr() as *mut c_void,
                                    cap,
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                );
                                eqv(&format!("FSE_writeNCount ret {tag} cap={cap}"), wc, wr);
                                eqbuf(&format!("FSE_writeNCount buf {tag} cap={cap}"), &hc, &hr);
                            }
                            let (mut header, _) = twin(bound + 8);
                            let hsize = wn_c(
                                header.as_mut_ptr() as *mut c_void,
                                bound,
                                norm.as_ptr(),
                                msv,
                                tl,
                            );
                            if is_err(hsize) {
                                continue;
                            }

                            // ---- readNCount / readNCount_bmi2
                            for &bmi2 in &[0i32, 1] {
                                for &extra in &[0usize, 1, 4] {
                                    let avail = (hsize + extra).min(header.len());
                                    let (mut n2c, mut n2r) = twin16(256);
                                    let mut mc: c_uint = 255;
                                    let mut mr: c_uint = 255;
                                    let mut tc: c_uint = FSE_MAX_TABLELOG;
                                    let mut tr2: c_uint = FSE_MAX_TABLELOG;
                                    let (rc2, rr2) = if bmi2 == 0 && extra == 0 {
                                        (
                                            rn_c(
                                                n2c.as_mut_ptr(),
                                                &mut mc,
                                                &mut tc,
                                                header.as_ptr() as *const c_void,
                                                avail,
                                            ),
                                            rn_r(
                                                n2r.as_mut_ptr(),
                                                &mut mr,
                                                &mut tr2,
                                                header.as_ptr() as *const c_void,
                                                avail,
                                            ),
                                        )
                                    } else {
                                        (
                                            rb_c(
                                                n2c.as_mut_ptr(),
                                                &mut mc,
                                                &mut tc,
                                                header.as_ptr() as *const c_void,
                                                avail,
                                                bmi2,
                                            ),
                                            rb_r(
                                                n2r.as_mut_ptr(),
                                                &mut mr,
                                                &mut tr2,
                                                header.as_ptr() as *const c_void,
                                                avail,
                                                bmi2,
                                            ),
                                        )
                                    };
                                    eqv(
                                        &format!("FSE_readNCount ret {tag} bmi2={bmi2} extra={extra}"),
                                        rc2,
                                        rr2,
                                    );
                                    eqv("FSE_readNCount maxSymbolValue", mc, mr);
                                    eqv("FSE_readNCount tableLog", tc, tr2);
                                    eqbuf(
                                        &format!("FSE_readNCount norm[] {tag}"),
                                        as_bytes16(&n2c),
                                        as_bytes16(&n2r),
                                    );
                                }
                            }

                            // ---- buildCTable_wksp (+ short workspace)
                            let ct_u32 = fse_ctable_size_u32(tl, msv);
                            let wk_u32 = fse_build_ctable_wksp_u32(msv, tl);
                            let (mut ctc, mut ctr) = twin32(ct_u32);
                            let mut wc = vec![0u32; wk_u32 + 64];
                            let mut wr = vec![0u32; wk_u32 + 64];
                            let bcc = bc_c(
                                ctc.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                tl,
                                wc.as_mut_ptr() as *mut c_void,
                                wk_u32 * 4,
                            );
                            let bcr = bc_r(
                                ctr.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                tl,
                                wr.as_mut_ptr() as *mut c_void,
                                wk_u32 * 4,
                            );
                            eqv(&format!("FSE_buildCTable_wksp ret {tag}"), bcc, bcr);
                            eqbuf(
                                &format!("FSE_buildCTable_wksp CTable {tag}"),
                                as_bytes32(&ctc),
                                as_bytes32(&ctr),
                            );
                            {
                                // undersized workspace -> error path, compared too
                                let (mut c2, mut r2) = twin32(ct_u32);
                                let e1 = bc_c(
                                    c2.as_mut_ptr(),
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                    wc.as_mut_ptr() as *mut c_void,
                                    4,
                                );
                                let e2 = bc_r(
                                    r2.as_mut_ptr(),
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                    wr.as_mut_ptr() as *mut c_void,
                                    4,
                                );
                                eqv(&format!("FSE_buildCTable_wksp tiny wksp {tag}"), e1, e2);
                            }
                            if is_err(bcc) {
                                continue;
                            }

                            // ---- compress_usingCTable
                            let mut blob_c: Vec<u8> = Vec::new();
                            for &capMode in &[0usize, 1, 2] {
                                let full = cb_c(masked.len()).max(16);
                                let cap = match capMode {
                                    0 => full,
                                    1 => full / 2 + 1,
                                    _ => 4,
                                };
                                let (mut oc, mut or) = twin(cap);
                                let sc = cu_c(
                                    oc.as_mut_ptr() as *mut c_void,
                                    cap,
                                    masked.as_ptr() as *const c_void,
                                    masked.len(),
                                    ctc.as_ptr(),
                                );
                                let sr = cu_r(
                                    or.as_mut_ptr() as *mut c_void,
                                    cap,
                                    masked.as_ptr() as *const c_void,
                                    masked.len(),
                                    ctr.as_ptr(),
                                );
                                eqv(
                                    &format!("FSE_compress_usingCTable ret {tag} capMode={capMode}"),
                                    sc,
                                    sr,
                                );
                                eqbuf(
                                    &format!("FSE_compress_usingCTable dst {tag} capMode={capMode}"),
                                    &oc,
                                    &or,
                                );
                                if capMode == 0 && !is_err(sc) && sc > 0 {
                                    blob_c = oc[..sc].to_vec();
                                }
                            }

                            // ---- buildDTable_wksp
                            let dt_u32 = fse_dtable_size_u32(tl);
                            let dwk_u32 = fse_build_dtable_wksp_u32(tl, msv);
                            let (mut dtc, mut dtr) = twin32(dt_u32);
                            let mut dwc = vec![0u32; dwk_u32 + 64];
                            let mut dwr = vec![0u32; dwk_u32 + 64];
                            let bdc = bd_c(
                                dtc.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                tl,
                                dwc.as_mut_ptr() as *mut c_void,
                                dwk_u32 * 4,
                            );
                            let bdr = bd_r(
                                dtr.as_mut_ptr(),
                                norm.as_ptr(),
                                msv,
                                tl,
                                dwr.as_mut_ptr() as *mut c_void,
                                dwk_u32 * 4,
                            );
                            eqv(&format!("FSE_buildDTable_wksp ret {tag}"), bdc, bdr);
                            eqbuf(
                                &format!("FSE_buildDTable_wksp DTable {tag}"),
                                as_bytes32(&dtc),
                                as_bytes32(&dtr),
                            );
                            {
                                let (mut c2, mut r2) = twin32(dt_u32);
                                let e1 = bd_c(
                                    c2.as_mut_ptr(),
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                    dwc.as_mut_ptr() as *mut c_void,
                                    4,
                                );
                                let e2 = bd_r(
                                    r2.as_mut_ptr(),
                                    norm.as_ptr(),
                                    msv,
                                    tl,
                                    dwr.as_mut_ptr() as *mut c_void,
                                    4,
                                );
                                eqv(&format!("FSE_buildDTable_wksp tiny wksp {tag}"), e1, e2);
                            }

                            // ---- decompress_wksp_bmi2 round-trip (header + payload)
                            if !blob_c.is_empty() {
                                let mut frame = header[..hsize].to_vec();
                                frame.extend_from_slice(&blob_c);
                                let dwk = fse_decompress_wksp_u32(FSE_MAX_TABLELOG, 255);
                                for &bmi2 in &[0i32, 1] {
                                    for &capMode in &[0usize, 1] {
                                        let cap = if capMode == 0 {
                                            masked.len()
                                        } else {
                                            masked.len() / 2
                                        };
                                        let (mut oc, mut or) = twin(cap.max(1));
                                        let mut w1 = vec![0u32; dwk + 64];
                                        let mut w2 = vec![0u32; dwk + 64];
                                        let a = dw_c(
                                            oc.as_mut_ptr() as *mut c_void,
                                            cap,
                                            frame.as_ptr() as *const c_void,
                                            frame.len(),
                                            FSE_MAX_TABLELOG,
                                            w1.as_mut_ptr() as *mut c_void,
                                            dwk * 4,
                                            bmi2,
                                        );
                                        let b = dw_r(
                                            or.as_mut_ptr() as *mut c_void,
                                            cap,
                                            frame.as_ptr() as *const c_void,
                                            frame.len(),
                                            FSE_MAX_TABLELOG,
                                            w2.as_mut_ptr() as *mut c_void,
                                            dwk * 4,
                                            bmi2,
                                        );
                                        eqv(
                                            &format!(
                                                "FSE_decompress_wksp_bmi2 ret {tag} bmi2={bmi2} capMode={capMode}"
                                            ),
                                            a,
                                            b,
                                        );
                                        eqbuf(
                                            &format!(
                                                "FSE_decompress_wksp_bmi2 dst {tag} bmi2={bmi2} capMode={capMode}"
                                            ),
                                            &oc,
                                            &or,
                                        );
                                        if capMode == 0 && !is_err(a) {
                                            assert_eq!(a, masked.len(), "FSE round-trip size {tag}");
                                            eqbuf(
                                                &format!("FSE round-trip content {tag}"),
                                                &masked,
                                                &oc[..a],
                                            );
                                        }
                                    }
                                }
                                // truncated / garbage payloads
                                let mut bad = frame.clone();
                                if bad.len() > 2 {
                                    let i = rng.below(bad.len());
                                    bad[i] ^= 0xFF;
                                }
                                let dwk = fse_decompress_wksp_u32(FSE_MAX_TABLELOG, 255);
                                let (mut oc, mut or) = twin(masked.len().max(1));
                                let mut w1 = vec![0u32; dwk + 64];
                                let mut w2 = vec![0u32; dwk + 64];
                                let a = dw_c(
                                    oc.as_mut_ptr() as *mut c_void,
                                    oc.len(),
                                    bad.as_ptr() as *const c_void,
                                    bad.len(),
                                    FSE_MAX_TABLELOG,
                                    w1.as_mut_ptr() as *mut c_void,
                                    dwk * 4,
                                    0,
                                );
                                let b = dw_r(
                                    or.as_mut_ptr() as *mut c_void,
                                    or.len(),
                                    bad.as_ptr() as *const c_void,
                                    bad.len(),
                                    FSE_MAX_TABLELOG,
                                    w2.as_mut_ptr() as *mut c_void,
                                    dwk * 4,
                                    0,
                                );
                                eqv(&format!("FSE_decompress corrupted ret {tag}"), a, b);
                                if !is_err(a) {
                                    eqbuf(&format!("FSE_decompress corrupted dst {tag}"), &oc, &or);
                                }
                            }
                        }
                    }
                }
              }
            }
        }
    }
}

#[test]
fn row135_fse_errors_and_version() {
    unsafe {
        let (ie_c, ie_r) = duo::<FnIsError>("FSE_isError");
        let (en_c, en_r) = duo::<FnErrName>("FSE_getErrorName");
        let (vn_c, vn_r) = duo::<FnUint0>("FSE_versionNumber");
        eqv("FSE_versionNumber", vn_c(), vn_r());
        for code in 0..200usize {
            let v = usize::MAX - code;
            eqv(&format!("FSE_isError(-{code})"), ie_c(v), ie_r(v));
            eqv(
                &format!("FSE_getErrorName(-{code})"),
                cstr(en_c(v)),
                cstr(en_r(v)),
            );
        }
        for code in [0usize, 1, 2, 100, 1000, 1 << 20, usize::MAX / 2] {
            eqv(&format!("FSE_isError({code})"), ie_c(code), ie_r(code));
            eqv(
                &format!("FSE_getErrorName({code})"),
                cstr(en_c(code)),
                cstr(en_r(code)),
            );
        }
    }
}

// ==================================================================== row 146

#[test]
fn row146_err_get_error_string() {
    unsafe {
        let (f_c, f_r) = duo::<unsafe extern "C" fn(c_int) -> *const c_char>("ERR_getErrorString");
        for code in 0..=130i32 {
            eqv(
                &format!("ERR_getErrorString({code})"),
                cstr(f_c(code)),
                cstr(f_r(code)),
            );
        }
        for &code in &[-1i32, -2, -100, 131, 132, 200, 1000, 65535, i32::MAX] {
            eqv(
                &format!("ERR_getErrorString({code}) out-of-range"),
                cstr(f_c(code)),
                cstr(f_r(code)),
            );
        }
        // and cross-check against the public name lookups
        let (zn_c, zn_r) = duo::<FnErrName>("ZSTD_getErrorName");
        for code in 0..=130usize {
            let v = usize::MAX - code + 1;
            eqv(
                &format!("ZSTD_getErrorName(-{code})"),
                cstr(zn_c(v)),
                cstr(zn_r(v)),
            );
        }
    }
}

// ============================================================ rows 136 - 138

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
type FnHufWriteCTable =
    unsafe extern "C" fn(*mut c_void, usize, *const u64, c_uint, c_uint, *mut c_void, usize) -> usize;
type FnHufReadCTable = unsafe extern "C" fn(
    *mut u64,
    *mut c_uint,
    *const c_void,
    usize,
    *mut c_uint,
) -> usize;
type FnHufEstimate = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> usize;
type FnHufValidate = unsafe extern "C" fn(*const u64, *const c_uint, c_uint) -> c_int;
type FnHufNbBits = unsafe extern "C" fn(*const u64, u32) -> u32;
type FnHufCardinality = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
type FnHufMinTableLog = unsafe extern "C" fn(c_uint) -> c_uint;
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
type FnHufSelectDecoder = unsafe extern "C" fn(usize, usize) -> u32;
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct HUF_CTableHeader {
    tableLog: u8,
    maxSymbolValue: u8,
    unused: [u8; 6],
}

/// Every interesting `flags` bitmask combination.
const HUF_FLAG_SET: [c_int; 10] = [
    0,
    HUF_flags_bmi2,
    HUF_flags_optimalDepth,
    HUF_flags_preferRepeat,
    HUF_flags_suspectUncompressible,
    HUF_flags_disableAsm,
    HUF_flags_disableFast,
    HUF_flags_disableAsm | HUF_flags_disableFast,
    HUF_flags_bmi2 | HUF_flags_optimalDepth,
    HUF_flags_bmi2
        | HUF_flags_optimalDepth
        | HUF_flags_preferRepeat
        | HUF_flags_suspectUncompressible
        | HUF_flags_disableAsm
        | HUF_flags_disableFast,
];

/// Restrict `data` to the alphabet `[0, msv]` (msv==0 means "auto" = 255).
fn to_alphabet(data: &[u8], msv: u32) -> Vec<u8> {
    let m = if msv == 0 { 255 } else { msv };
    data.iter().map(|&b| (b as u32 % (m + 1)) as u8).collect()
}

#[test]
fn row136_huf_compress() {
    unsafe {
        let (c4_c, c4_r) = duo::<FnHufRepeat>("HUF_compress4X_repeat");
        let (c1_c, c1_r) = duo::<FnHufRepeat>("HUF_compress1X_repeat");
        let (u4_c, u4_r) = duo::<FnHufUsingCTable>("HUF_compress4X_usingCTable");
        let (u1_c, u1_r) = duo::<FnHufUsingCTable>("HUF_compress1X_usingCTable");
        let (bd_c, bd_r) = duo::<FnSizeT1>("HUF_compressBound");
        let (ie_c, ie_r) = duo::<FnIsError>("HUF_isError");
        let (en_c, en_r) = duo::<FnErrName>("HUF_getErrorName");

        for &n in &[0usize, 1, 7, 128, 1024, 65535, 128 * 1024, 1 << 20] {
            eqv(&format!("HUF_compressBound({n})"), bd_c(n), bd_r(n));
        }
        for code in 0..200usize {
            let v = usize::MAX - code;
            eqv(&format!("HUF_isError(-{code})"), ie_c(v), ie_r(v));
            eqv(
                &format!("HUF_getErrorName(-{code})"),
                cstr(en_c(v)),
                cstr(en_r(v)),
            );
        }

        let sizes: [usize; 12] = [
            1,
            2,
            5,
            6,
            7,
            63,
            128,
            1024,
            8 * 1024,
            40 * 1024,
            128 * 1024,
            128 * 1024 + 1, // > HUF_BLOCKSIZE_MAX -> srcSize_wrong
        ];
        let msvs: [u32; 4] = [0, 15, 63, 255];
        let tls: [u32; 5] = [0, 5, 8, 11, 12];

        let (card_c, _) = duo::<FnHufCardinality>("HUF_cardinality");
        let (mintl_c, _) = duo::<FnHufMinTableLog>("HUF_minTableLog");

        for class in 0..N_CLASSES {
            for &size in &sizes {
                let raw = gen_class(class, size, 0x136);
                for &msv in &msvs {
                    let src = to_alphabet(&raw, msv);
                    // A too-small explicit tableLog cannot represent the alphabet;
                    // HUF_setMaxHeight() is then called with an impossible target
                    // and walks off its table in C. zstd itself always passes
                    // HUF_TABLELOG_DEFAULT, so those combinations are excluded.
                    let msv_eff = if msv == 0 { 255 } else { msv };
                    let (hcount, _, _) = c_hist(&src, msv_eff);
                    let card = card_c(hcount.as_ptr(), msv_eff);
                    let min_tl = if card == 0 { 1 } else { mintl_c(card) };
                    for &tl in &tls {
                        if tl != 0 && tl < min_tl {
                            continue;
                        }
                        for (fi, &flags) in HUF_FLAG_SET.iter().enumerate() {
                            // trim the grid a little on the biggest inputs
                            if size > 40 * 1024 && (fi % 3 != 0) {
                                continue;
                            }
                            for streams in 0..2 {
                                let (fc, fr) = if streams == 0 {
                                    (c4_c, c4_r)
                                } else {
                                    (c1_c, c1_r)
                                };
                                let name = if streams == 0 {
                                    "HUF_compress4X_repeat"
                                } else {
                                    "HUF_compress1X_repeat"
                                };
                                let tag = format!(
                                    "{name} class={} size={size} msv={msv} tl={tl} flags={flags}",
                                    CLASS_NAMES[class]
                                );
                                let cap = bd_c(src.len()).max(16);
                                for repeat0 in [HUF_repeat_none, HUF_repeat_check, HUF_repeat_valid]
                                {
                                    // build a table first so that repeat_{check,valid}
                                    // start from a *real* table (identical in both libs)
                                    let mut tc = vec![0xA5A5_A5A5_A5A5_A5A5u64; 257];
                                    let mut tr = vec![0xA5A5_A5A5_A5A5_A5A5u64; 257];
                                    let mut wc = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                                    let mut wr = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                                    if repeat0 != HUF_repeat_none {
                                        let mut r0c = HUF_repeat_none;
                                        let mut r0r = HUF_repeat_none;
                                        let (mut oc, mut or) = twin(cap);
                                        let a = fc(
                                            oc.as_mut_ptr() as *mut c_void,
                                            cap,
                                            src.as_ptr() as *const c_void,
                                            src.len(),
                                            msv,
                                            tl,
                                            wc.as_mut_ptr() as *mut c_void,
                                            HUF_WORKSPACE_SIZE,
                                            tc.as_mut_ptr(),
                                            &mut r0c,
                                            flags & !HUF_flags_preferRepeat,
                                        );
                                        let b = fr(
                                            or.as_mut_ptr() as *mut c_void,
                                            cap,
                                            src.as_ptr() as *const c_void,
                                            src.len(),
                                            msv,
                                            tl,
                                            wr.as_mut_ptr() as *mut c_void,
                                            HUF_WORKSPACE_SIZE,
                                            tr.as_mut_ptr(),
                                            &mut r0r,
                                            flags & !HUF_flags_preferRepeat,
                                        );
                                        eqv(&format!("{tag} priming ret"), a, b);
                                        eqbuf(&format!("{tag} priming dst"), &oc, &or);
                                        eqbuf(
                                            &format!("{tag} priming CTable"),
                                            as_bytes64(&tc),
                                            as_bytes64(&tr),
                                        );
                                        if is_err(a) || a <= 1 {
                                            continue; // no usable table was produced
                                        }
                                    }
                                    let mut rc = repeat0;
                                    let mut rr = repeat0;
                                    let (mut oc, mut or) = twin(cap);
                                    let a = fc(
                                        oc.as_mut_ptr() as *mut c_void,
                                        cap,
                                        src.as_ptr() as *const c_void,
                                        src.len(),
                                        msv,
                                        tl,
                                        wc.as_mut_ptr() as *mut c_void,
                                        HUF_WORKSPACE_SIZE,
                                        tc.as_mut_ptr(),
                                        &mut rc,
                                        flags,
                                    );
                                    let b = fr(
                                        or.as_mut_ptr() as *mut c_void,
                                        cap,
                                        src.as_ptr() as *const c_void,
                                        src.len(),
                                        msv,
                                        tl,
                                        wr.as_mut_ptr() as *mut c_void,
                                        HUF_WORKSPACE_SIZE,
                                        tr.as_mut_ptr(),
                                        &mut rr,
                                        flags,
                                    );
                                    eqv(&format!("{tag} repeat={repeat0} ret"), a, b);
                                    eqv(&format!("{tag} repeat={repeat0} *repeat"), rc, rr);
                                    eqbuf(&format!("{tag} repeat={repeat0} dst"), &oc, &or);
                                    eqbuf(
                                        &format!("{tag} repeat={repeat0} CTable"),
                                        as_bytes64(&tc),
                                        as_bytes64(&tr),
                                    );
                                    eqv(&format!("{tag} isError"), ie_c(a), ie_r(b));

                                    // tight / undersized dst
                                    if !is_err(a) && a > 1 {
                                        for cap2 in [a, a - 1, 1] {
                                            let mut r2c = repeat0;
                                            let mut r2r = repeat0;
                                            let (mut o2, mut o3) = twin(cap2);
                                            let x = fc(
                                                o2.as_mut_ptr() as *mut c_void,
                                                cap2,
                                                src.as_ptr() as *const c_void,
                                                src.len(),
                                                msv,
                                                tl,
                                                wc.as_mut_ptr() as *mut c_void,
                                                HUF_WORKSPACE_SIZE,
                                                tc.as_mut_ptr(),
                                                &mut r2c,
                                                flags,
                                            );
                                            let y = fr(
                                                o3.as_mut_ptr() as *mut c_void,
                                                cap2,
                                                src.as_ptr() as *const c_void,
                                                src.len(),
                                                msv,
                                                tl,
                                                wr.as_mut_ptr() as *mut c_void,
                                                HUF_WORKSPACE_SIZE,
                                                tr.as_mut_ptr(),
                                                &mut r2r,
                                                flags,
                                            );
                                            eqv(&format!("{tag} cap={cap2} ret"), x, y);
                                            eqv(&format!("{tag} cap={cap2} *repeat"), r2c, r2r);
                                            eqbuf(&format!("{tag} cap={cap2} dst"), &o2, &o3);
                                        }
                                    }

                                    // HUF_compress{1,4}X_usingCTable with the table we just built
                                    if !is_err(a) && a > 1 {
                                        for which in 0..2 {
                                            let (gc, gr) =
                                                if which == 0 { (u4_c, u4_r) } else { (u1_c, u1_r) };
                                            let nm = if which == 0 {
                                                "HUF_compress4X_usingCTable"
                                            } else {
                                                "HUF_compress1X_usingCTable"
                                            };
                                            let (mut o2, mut o3) = twin(cap);
                                            let x = gc(
                                                o2.as_mut_ptr() as *mut c_void,
                                                cap,
                                                src.as_ptr() as *const c_void,
                                                src.len(),
                                                tc.as_ptr(),
                                                flags,
                                            );
                                            let y = gr(
                                                o3.as_mut_ptr() as *mut c_void,
                                                cap,
                                                src.as_ptr() as *const c_void,
                                                src.len(),
                                                tr.as_ptr(),
                                                flags,
                                            );
                                            eqv(&format!("{nm} {tag} ret"), x, y);
                                            eqbuf(&format!("{nm} {tag} dst"), &o2, &o3);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row138_huf_tables() {
    unsafe {
        let (bc_c, bc_r) = duo::<FnHufBuildCTable>("HUF_buildCTable_wksp");
        let (wc_c, wc_r) = duo::<FnHufWriteCTable>("HUF_writeCTable_wksp");
        let (rc_c, rc_r) = duo::<FnHufReadCTable>("HUF_readCTable");
        let (rh_c, rh_r) =
            duo::<unsafe extern "C" fn(*const u64) -> HUF_CTableHeader>("HUF_readCTableHeader");
        let (es_c, es_r) = duo::<FnHufEstimate>("HUF_estimateCompressedSize");
        let (va_c, va_r) = duo::<FnHufValidate>("HUF_validateCTable");
        let (nb_c, nb_r) = duo::<FnHufNbBits>("HUF_getNbBitsFromCTable");
        let (ca_c, ca_r) = duo::<FnHufCardinality>("HUF_cardinality");
        let (mt_c, mt_r) = duo::<FnHufMinTableLog>("HUF_minTableLog");
        let (ot_c, ot_r) = duo::<FnHufOptimalTableLog>("HUF_optimalTableLog");
        let (rs_c, rs_r) = duo::<FnHufReadStats>("HUF_readStats");
        let (rw_c, rw_r) = duo::<FnHufReadStatsWksp>("HUF_readStats_wksp");
        let (d1_c, d1_r) = duo::<FnHufReadDTable>("HUF_readDTableX1_wksp");
        let (d2_c, d2_r) = duo::<FnHufReadDTable>("HUF_readDTableX2_wksp");
        let (sd_c, sd_r) = duo::<FnHufSelectDecoder>("HUF_selectDecoder");

        // ---- HUF_minTableLog / HUF_selectDecoder exhaustive-ish sweeps
        for card in 1..=256u32 {
            eqv(&format!("HUF_minTableLog({card})"), mt_c(card), mt_r(card));
        }
        for &d in &[1usize, 2, 10, 100, 1000, 10_000, 128 * 1024] {
            for &c in &[1usize, 2, 3, 10, 100, 1000, 10_000, 128 * 1024] {
                eqv(
                    &format!("HUF_selectDecoder({d},{c})"),
                    sd_c(d, c),
                    sd_r(d, c),
                );
            }
        }

        let sizes: [usize; 7] = [64, 200, 1024, 4096, 8 * 1024, 40 * 1024, 128 * 1024];
        for class in 0..N_CLASSES {
            for &size in &sizes {
                let raw = gen_class(class, size, 0x138);
                for &msv in &[15u32, 63, 127, 255] {
                    let src = to_alphabet(&raw, msv);
                    let (count, real_msv, _) = c_hist(&src, msv);
                    let card = ca_c(count.as_ptr(), msv);
                    eqv(
                        &format!("HUF_cardinality class={class} size={size} msv={msv}"),
                        card,
                        ca_r(count.as_ptr(), msv),
                    );
                    if card < 2 || real_msv == 0 {
                        continue; // RLE territory; HUF_buildTree needs >=1 symbol
                    }
                    let min_tl = mt_c(card);
                    for &tl in &[0u32, 5, 8, 11, 12] {
                        // see row136: tableLog < HUF_minTableLog(cardinality) is
                        // an impossible target for HUF_setMaxHeight() in C.
                        if tl != 0 && tl < min_tl {
                            continue;
                        }
                        let tag = format!(
                            "class={} size={size} msv={msv} tl={tl}",
                            CLASS_NAMES[class]
                        );
                        // ---- HUF_optimalTableLog
                        for &flags in &[0, HUF_flags_optimalDepth] {
                            let mut sc = vec![0u64; 257];
                            let mut sr = vec![0u64; 257];
                            let mut wc = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                            let mut wr = vec![0u64; HUF_WORKSPACE_SIZE / 8];
                            let a = ot_c(
                                if tl == 0 { HUF_TABLELOG_DEFAULT } else { tl },
                                src.len(),
                                msv,
                                wc.as_mut_ptr() as *mut c_void,
                                HUF_WORKSPACE_SIZE,
                                sc.as_mut_ptr(),
                                count.as_ptr(),
                                flags,
                            );
                            let b = ot_r(
                                if tl == 0 { HUF_TABLELOG_DEFAULT } else { tl },
                                src.len(),
                                msv,
                                wr.as_mut_ptr() as *mut c_void,
                                HUF_WORKSPACE_SIZE,
                                sr.as_mut_ptr(),
                                count.as_ptr(),
                                flags,
                            );
                            eqv(&format!("HUF_optimalTableLog {tag} flags={flags}"), a, b);
                            eqbuf(
                                &format!("HUF_optimalTableLog scratch {tag} flags={flags}"),
                                as_bytes64(&sc),
                                as_bytes64(&sr),
                            );
                        }
                        // ---- HUF_buildCTable_wksp
                        let mut ctc = vec![0xA5A5_A5A5_A5A5_A5A5u64; 257];
                        let mut ctr = vec![0xA5A5_A5A5_A5A5_A5A5u64; 257];
                        let mut wc = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32];
                        let mut wr = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32];
                        let mb_c = bc_c(
                            ctc.as_mut_ptr(),
                            count.as_ptr(),
                            msv,
                            tl,
                            wc.as_mut_ptr() as *mut c_void,
                            wc.len() * 4,
                        );
                        let mb_r = bc_r(
                            ctr.as_mut_ptr(),
                            count.as_ptr(),
                            msv,
                            tl,
                            wr.as_mut_ptr() as *mut c_void,
                            wr.len() * 4,
                        );
                        eqv(&format!("HUF_buildCTable_wksp ret {tag}"), mb_c, mb_r);
                        eqbuf(
                            &format!("HUF_buildCTable_wksp CTable {tag}"),
                            as_bytes64(&ctc),
                            as_bytes64(&ctr),
                        );
                        {
                            // undersized workspace
                            let mut c2 = vec![0u64; 257];
                            let mut r2 = vec![0u64; 257];
                            eqv(
                                &format!("HUF_buildCTable_wksp tiny wksp {tag}"),
                                bc_c(
                                    c2.as_mut_ptr(),
                                    count.as_ptr(),
                                    msv,
                                    tl,
                                    wc.as_mut_ptr() as *mut c_void,
                                    16,
                                ),
                                bc_r(
                                    r2.as_mut_ptr(),
                                    count.as_ptr(),
                                    msv,
                                    tl,
                                    wr.as_mut_ptr() as *mut c_void,
                                    16,
                                ),
                            );
                        }
                        if is_err(mb_c) {
                            continue;
                        }
                        let huffLog = mb_c as c_uint;

                        // ---- HUF_readCTableHeader
                        let hc = rh_c(ctc.as_ptr());
                        let hr = rh_r(ctr.as_ptr());
                        eqv(&format!("HUF_readCTableHeader {tag}"), hc, hr);

                        // ---- HUF_getNbBitsFromCTable over the whole alphabet
                        for s in 0..=255u32 {
                            eqv(
                                &format!("HUF_getNbBitsFromCTable({s}) {tag}"),
                                nb_c(ctc.as_ptr(), s),
                                nb_r(ctr.as_ptr(), s),
                            );
                        }
                        // ---- estimate / validate
                        for &m in &[0u32, 1, 15, msv] {
                            eqv(
                                &format!("HUF_estimateCompressedSize(msv={m}) {tag}"),
                                es_c(ctc.as_ptr(), count.as_ptr(), m),
                                es_r(ctr.as_ptr(), count.as_ptr(), m),
                            );
                            eqv(
                                &format!("HUF_validateCTable(msv={m}) {tag}"),
                                va_c(ctc.as_ptr(), count.as_ptr(), m),
                                va_r(ctr.as_ptr(), count.as_ptr(), m),
                            );
                        }

                        // ---- HUF_writeCTable_wksp (+ capacity sweep)
                        let mut desc: Vec<u8> = Vec::new();
                        for &cap in &[256usize, 128, 32, 8, 1] {
                            let (mut oc, mut or) = twin(cap);
                            let mut w1 = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32];
                            let mut w2 = vec![0u32; HUF_CTABLE_WORKSPACE_SIZE_U32];
                            let a = wc_c(
                                oc.as_mut_ptr() as *mut c_void,
                                cap,
                                ctc.as_ptr(),
                                hc.maxSymbolValue as c_uint,
                                huffLog,
                                w1.as_mut_ptr() as *mut c_void,
                                w1.len() * 4,
                            );
                            let b = wc_r(
                                or.as_mut_ptr() as *mut c_void,
                                cap,
                                ctr.as_ptr(),
                                hr.maxSymbolValue as c_uint,
                                huffLog,
                                w2.as_mut_ptr() as *mut c_void,
                                w2.len() * 4,
                            );
                            eqv(&format!("HUF_writeCTable_wksp ret {tag} cap={cap}"), a, b);
                            eqbuf(&format!("HUF_writeCTable_wksp dst {tag} cap={cap}"), &oc, &or);
                            if cap == 256 && !is_err(a) {
                                desc = oc[..a].to_vec();
                            }
                        }
                        if desc.is_empty() {
                            continue;
                        }

                        // ---- HUF_readStats / HUF_readStats_wksp
                        for &avail in &[desc.len(), desc.len().saturating_sub(1), 1] {
                            let (mut hwc, mut hwr) = twin(256);
                            let (mut rkc, mut rkr) = twin32(13);
                            let mut nsc = 0u32;
                            let mut nsr = 0u32;
                            let mut tlc = 0u32;
                            let mut tlr = 0u32;
                            let a = rs_c(
                                hwc.as_mut_ptr(),
                                256,
                                rkc.as_mut_ptr(),
                                &mut nsc,
                                &mut tlc,
                                desc.as_ptr() as *const c_void,
                                avail,
                            );
                            let b = rs_r(
                                hwr.as_mut_ptr(),
                                256,
                                rkr.as_mut_ptr(),
                                &mut nsr,
                                &mut tlr,
                                desc.as_ptr() as *const c_void,
                                avail,
                            );
                            eqv(&format!("HUF_readStats ret {tag} avail={avail}"), a, b);
                            eqv("HUF_readStats nbSymbols", nsc, nsr);
                            eqv("HUF_readStats tableLog", tlc, tlr);
                            eqbuf(&format!("HUF_readStats weights {tag}"), &hwc, &hwr);
                            eqbuf(
                                &format!("HUF_readStats rankStats {tag}"),
                                as_bytes32(&rkc),
                                as_bytes32(&rkr),
                            );
                            for &flags in &[0, HUF_flags_bmi2, HUF_flags_disableAsm] {
                                let (mut h2, mut h3) = twin(256);
                                let (mut k2, mut k3) = twin32(13);
                                let mut n2 = 0u32;
                                let mut n3 = 0u32;
                                let mut t2 = 0u32;
                                let mut t3 = 0u32;
                                let mut w1 = vec![0u32; 1024];
                                let mut w2 = vec![0u32; 1024];
                                let x = rw_c(
                                    h2.as_mut_ptr(),
                                    256,
                                    k2.as_mut_ptr(),
                                    &mut n2,
                                    &mut t2,
                                    desc.as_ptr() as *const c_void,
                                    avail,
                                    w1.as_mut_ptr() as *mut c_void,
                                    w1.len() * 4,
                                    flags,
                                );
                                let y = rw_r(
                                    h3.as_mut_ptr(),
                                    256,
                                    k3.as_mut_ptr(),
                                    &mut n3,
                                    &mut t3,
                                    desc.as_ptr() as *const c_void,
                                    avail,
                                    w2.as_mut_ptr() as *mut c_void,
                                    w2.len() * 4,
                                    flags,
                                );
                                eqv(
                                    &format!("HUF_readStats_wksp ret {tag} avail={avail} flags={flags}"),
                                    x,
                                    y,
                                );
                                eqv("HUF_readStats_wksp nbSymbols", n2, n3);
                                eqv("HUF_readStats_wksp tableLog", t2, t3);
                                eqbuf(&format!("HUF_readStats_wksp weights {tag}"), &h2, &h3);
                                eqbuf(
                                    &format!("HUF_readStats_wksp rankStats {tag}"),
                                    as_bytes32(&k2),
                                    as_bytes32(&k3),
                                );
                            }
                        }

                        // ---- HUF_readCTable (round-trip of the description)
                        for &m in &[255u32, 15] {
                            let (mut c2, mut r2) = (
                                vec![0xA5A5_A5A5_A5A5_A5A5u64; 257],
                                vec![0xA5A5_A5A5_A5A5_A5A5u64; 257],
                            );
                            let mut mc = m;
                            let mut mr = m;
                            let mut zc = 0xFFFF_FFFFu32;
                            let mut zr = 0xFFFF_FFFFu32;
                            let a = rc_c(
                                c2.as_mut_ptr(),
                                &mut mc,
                                desc.as_ptr() as *const c_void,
                                desc.len(),
                                &mut zc,
                            );
                            let b = rc_r(
                                r2.as_mut_ptr(),
                                &mut mr,
                                desc.as_ptr() as *const c_void,
                                desc.len(),
                                &mut zr,
                            );
                            eqv(&format!("HUF_readCTable ret {tag} m={m}"), a, b);
                            eqv("HUF_readCTable maxSymbolValue", mc, mr);
                            eqv("HUF_readCTable hasZeroWeights", zc, zr);
                            eqbuf(
                                &format!("HUF_readCTable CTable {tag} m={m}"),
                                as_bytes64(&c2),
                                as_bytes64(&r2),
                            );
                        }

                        // ---- HUF_readDTableX1_wksp / HUF_readDTableX2_wksp
                        for &flags in &[
                            0,
                            HUF_flags_bmi2,
                            HUF_flags_disableAsm,
                            HUF_flags_disableFast,
                        ] {
                            for x2 in 0..2 {
                                let (fc, fr) = if x2 == 0 { (d1_c, d1_r) } else { (d2_c, d2_r) };
                                let nm = if x2 == 0 {
                                    "HUF_readDTableX1_wksp"
                                } else {
                                    "HUF_readDTableX2_wksp"
                                };
                                let hdr0 = if x2 == 0 {
                                    (HUF_TABLELOG_MAX - 1) * 0x0100_0001
                                } else {
                                    HUF_TABLELOG_MAX * 0x0100_0001
                                };
                                let mut dc = vec![0xA5A5_A5A5u32; 1 + (1 << HUF_TABLELOG_MAX)];
                                let mut dr = vec![0xA5A5_A5A5u32; 1 + (1 << HUF_TABLELOG_MAX)];
                                dc[0] = hdr0;
                                dr[0] = hdr0;
                                let mut w1 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let mut w2 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let a = fc(
                                    dc.as_mut_ptr(),
                                    desc.as_ptr() as *const c_void,
                                    desc.len(),
                                    w1.as_mut_ptr() as *mut c_void,
                                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                                    flags,
                                );
                                let b = fr(
                                    dr.as_mut_ptr(),
                                    desc.as_ptr() as *const c_void,
                                    desc.len(),
                                    w2.as_mut_ptr() as *mut c_void,
                                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                                    flags,
                                );
                                eqv(&format!("{nm} ret {tag} flags={flags}"), a, b);
                                eqbuf(
                                    &format!("{nm} DTable {tag} flags={flags}"),
                                    as_bytes32(&dc),
                                    as_bytes32(&dr),
                                );
                                // undersized workspace
                                let mut e1 = vec![hdr0; 1 + (1 << HUF_TABLELOG_MAX)];
                                let mut e2 = vec![hdr0; 1 + (1 << HUF_TABLELOG_MAX)];
                                eqv(
                                    &format!("{nm} tiny wksp {tag}"),
                                    fc(
                                        e1.as_mut_ptr(),
                                        desc.as_ptr() as *const c_void,
                                        desc.len(),
                                        w1.as_mut_ptr() as *mut c_void,
                                        16,
                                        flags,
                                    ),
                                    fr(
                                        e2.as_mut_ptr(),
                                        desc.as_ptr() as *const c_void,
                                        desc.len(),
                                        w2.as_mut_ptr() as *mut c_void,
                                        16,
                                        flags,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ==================================================================== row 137

/// Compress `src` with the C library and split the blob into
/// `(tableDescription, bitstream)`.
unsafe fn c_huf_blob(src: &[u8], msv: u32, tl: u32, four: bool) -> Option<(Vec<u8>, usize)> {
    let name = if four {
        "HUF_compress4X_repeat"
    } else {
        "HUF_compress1X_repeat"
    };
    let (fc, _) = duo::<FnHufRepeat>(name);
    let (bd, _) = duo::<FnSizeT1>("HUF_compressBound");
    let (rs, _) = duo::<FnHufReadStats>("HUF_readStats");
    let cap = bd(src.len()).max(16);
    let mut out = vec![0u8; cap];
    let mut ct = vec![0u64; 257];
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
        return None; // 0 == "not compressible", 1 == RLE
    }
    out.truncate(n);
    let mut hw = vec![0u8; 256];
    let mut rk = vec![0u32; 13];
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
    Some((out, desc))
}

fn new_dtable(x1: bool) -> Vec<u32> {
    let mut d = vec![0xA5A5_A5A5u32; 1 + (1 << HUF_TABLELOG_MAX)];
    d[0] = if x1 {
        (HUF_TABLELOG_MAX - 1) * 0x0100_0001
    } else {
        HUF_TABLELOG_MAX * 0x0100_0001
    };
    d
}

const HUF_DEC_FLAGS: [c_int; 6] = [
    0,
    HUF_flags_bmi2,
    HUF_flags_disableAsm,
    HUF_flags_disableFast,
    HUF_flags_disableAsm | HUF_flags_disableFast,
    HUF_flags_bmi2 | HUF_flags_disableAsm | HUF_flags_disableFast,
];

#[test]
fn row137_huf_decompress() {
    unsafe {
        let (d1_c, d1_r) = duo::<FnHufReadDTable>("HUF_readDTableX1_wksp");
        let (d2_c, d2_r) = duo::<FnHufReadDTable>("HUF_readDTableX2_wksp");
        let (u1_c, u1_r) = duo::<FnHufDecUsingDTable>("HUF_decompress1X_usingDTable");
        let (u4_c, u4_r) = duo::<FnHufDecUsingDTable>("HUF_decompress4X_usingDTable");
        let (x1_c, x1_r) = duo::<FnHufDecDCtxWksp>("HUF_decompress1X1_DCtx_wksp");
        let (x2_c, x2_r) = duo::<FnHufDecDCtxWksp>("HUF_decompress1X2_DCtx_wksp");
        let (xa_c, xa_r) = duo::<FnHufDecDCtxWksp>("HUF_decompress1X_DCtx_wksp");
        let (ho_c, ho_r) = duo::<FnHufDecDCtxWksp>("HUF_decompress4X_hufOnly_wksp");
        let (sd_c, sd_r) = duo::<FnHufSelectDecoder>("HUF_selectDecoder");

        let sizes: [usize; 7] = [64, 200, 1024, 4096, 8 * 1024, 40 * 1024, 128 * 1024];
        for class in 0..N_CLASSES {
            for &size in &sizes {
                let raw = gen_class(class, size, 0x137);
                for &msv in &[0u32, 63, 255] {
                    let src = to_alphabet(&raw, msv);
                    for &tl in &[0u32, 11, 12] {
                        for four in [true, false] {
                            let Some((blob, descSize)) = c_huf_blob(&src, msv, tl, four) else {
                                continue;
                            };
                            let payload = &blob[descSize..];
                            let tag = format!(
                                "class={} size={size} msv={msv} tl={tl} four={four}",
                                CLASS_NAMES[class]
                            );
                            eqv(
                                &format!("HUF_selectDecoder {tag}"),
                                sd_c(src.len(), payload.len()),
                                sd_r(src.len(), payload.len()),
                            );

                            // ---- *_usingDTable with an X1 and an X2 table
                            for x1 in [true, false] {
                                let mut dc = new_dtable(x1);
                                let mut dr = new_dtable(x1);
                                let mut w1 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let mut w2 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let (rc, rr) = if x1 {
                                    (
                                        d1_c(
                                            dc.as_mut_ptr(),
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w1.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            0,
                                        ),
                                        d1_r(
                                            dr.as_mut_ptr(),
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w2.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            0,
                                        ),
                                    )
                                } else {
                                    (
                                        d2_c(
                                            dc.as_mut_ptr(),
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w1.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            0,
                                        ),
                                        d2_r(
                                            dr.as_mut_ptr(),
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w2.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            0,
                                        ),
                                    )
                                };
                                eqv(&format!("readDTable x1={x1} ret {tag}"), rc, rr);
                                eqbuf(
                                    &format!("readDTable x1={x1} DTable {tag}"),
                                    as_bytes32(&dc),
                                    as_bytes32(&dr),
                                );
                                if is_err(rc) {
                                    continue;
                                }
                                assert_eq!(rc, descSize, "table description size {tag}");
                                for &flags in &HUF_DEC_FLAGS {
                                    for dstMode in 0..2usize {
                                        let dstSize = if dstMode == 0 {
                                            src.len()
                                        } else {
                                            src.len() + 3
                                        };
                                        let (mut oc, mut or) = twin(dstSize);
                                        let (fc, fr) =
                                            if four { (u4_c, u4_r) } else { (u1_c, u1_r) };
                                        let nm = if four {
                                            "HUF_decompress4X_usingDTable"
                                        } else {
                                            "HUF_decompress1X_usingDTable"
                                        };
                                        let a = fc(
                                            oc.as_mut_ptr() as *mut c_void,
                                            dstSize,
                                            payload.as_ptr() as *const c_void,
                                            payload.len(),
                                            dc.as_ptr(),
                                            flags,
                                        );
                                        let b = fr(
                                            or.as_mut_ptr() as *mut c_void,
                                            dstSize,
                                            payload.as_ptr() as *const c_void,
                                            payload.len(),
                                            dr.as_ptr(),
                                            flags,
                                        );
                                        eqv(
                                            &format!("{nm} ret {tag} x1={x1} flags={flags} dstMode={dstMode}"),
                                            a,
                                            b,
                                        );
                                        eqbuf(
                                            &format!("{nm} dst {tag} x1={x1} flags={flags} dstMode={dstMode}"),
                                            &oc,
                                            &or,
                                        );
                                        if dstMode == 0 && !is_err(a) {
                                            assert_eq!(a, src.len(), "{nm} size {tag}");
                                            eqbuf(
                                                &format!("{nm} round-trip {tag} x1={x1}"),
                                                &src,
                                                &oc[..a],
                                            );
                                        }
                                    }
                                }
                            }

                            // ---- the DCtx_wksp entry points (full blob in)
                            let list: Vec<(&str, FnHufDecDCtxWksp, FnHufDecDCtxWksp)> = if four {
                                vec![("HUF_decompress4X_hufOnly_wksp", ho_c, ho_r)]
                            } else {
                                vec![
                                    ("HUF_decompress1X1_DCtx_wksp", x1_c, x1_r),
                                    ("HUF_decompress1X2_DCtx_wksp", x2_c, x2_r),
                                    ("HUF_decompress1X_DCtx_wksp", xa_c, xa_r),
                                ]
                            };
                            for (nm, fc, fr) in list {
                                for &flags in &HUF_DEC_FLAGS {
                                    for dstMode in 0..3usize {
                                        let dstSize = match dstMode {
                                            0 => src.len(),
                                            1 => src.len() + 5,
                                            _ => src.len() / 2,
                                        };
                                        if dstSize == 0 {
                                            continue;
                                        }
                                        let mut dc = new_dtable(false);
                                        let mut dr = new_dtable(false);
                                        let (mut oc, mut or) = twin(dstSize);
                                        let mut w1 =
                                            vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                        let mut w2 =
                                            vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                        let a = fc(
                                            dc.as_mut_ptr(),
                                            oc.as_mut_ptr() as *mut c_void,
                                            dstSize,
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w1.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            flags,
                                        );
                                        let b = fr(
                                            dr.as_mut_ptr(),
                                            or.as_mut_ptr() as *mut c_void,
                                            dstSize,
                                            blob.as_ptr() as *const c_void,
                                            blob.len(),
                                            w2.as_mut_ptr() as *mut c_void,
                                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                                            flags,
                                        );
                                        eqv(
                                            &format!("{nm} ret {tag} flags={flags} dstMode={dstMode}"),
                                            a,
                                            b,
                                        );
                                        eqbuf(
                                            &format!("{nm} dst {tag} flags={flags} dstMode={dstMode}"),
                                            &oc,
                                            &or,
                                        );
                                        eqbuf(
                                            &format!("{nm} DTable {tag} flags={flags} dstMode={dstMode}"),
                                            as_bytes32(&dc),
                                            as_bytes32(&dr),
                                        );
                                        if dstMode == 0 && !is_err(a) {
                                            eqbuf(
                                                &format!("{nm} round-trip {tag}"),
                                                &src,
                                                &oc[..a.min(oc.len())],
                                            );
                                        }
                                        // undersized workspace
                                        let mut e1 = new_dtable(false);
                                        let mut e2 = new_dtable(false);
                                        let (mut p1, mut p2) = twin(dstSize);
                                        eqv(
                                            &format!("{nm} tiny wksp {tag}"),
                                            fc(
                                                e1.as_mut_ptr(),
                                                p1.as_mut_ptr() as *mut c_void,
                                                dstSize,
                                                blob.as_ptr() as *const c_void,
                                                blob.len(),
                                                w1.as_mut_ptr() as *mut c_void,
                                                16,
                                                flags,
                                            ),
                                            fr(
                                                e2.as_mut_ptr(),
                                                p2.as_mut_ptr() as *mut c_void,
                                                dstSize,
                                                blob.as_ptr() as *const c_void,
                                                blob.len(),
                                                w2.as_mut_ptr() as *mut c_void,
                                                16,
                                                flags,
                                            ),
                                        );
                                    }
                                }
                            }

                            // ---- truncated / corrupted bitstreams
                            let mut rng = Rng::new(0x137 ^ size as u64 ^ class as u64);
                            for _ in 0..3 {
                                let mut bad = blob.clone();
                                let i = descSize + rng.below((bad.len() - descSize).max(1));
                                let last = bad.len() - 1;
                                let bit = 1u8 << rng.below(8);
                                bad[i.min(last)] ^= bit;
                                let span = bad.len() - descSize + 1;
                                let cut = descSize + rng.below(span);
                                let end = cut.max(descSize + 1).min(blob.len());
                                let bad = &bad[..end];
                                let mut dc = new_dtable(false);
                                let mut dr = new_dtable(false);
                                let (mut oc, mut or) = twin(src.len());
                                let mut w1 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let mut w2 = vec![0u32; HUF_DECOMPRESS_WORKSPACE_SIZE / 4];
                                let (fc, fr) = if four { (ho_c, ho_r) } else { (xa_c, xa_r) };
                                let a = fc(
                                    dc.as_mut_ptr(),
                                    oc.as_mut_ptr() as *mut c_void,
                                    oc.len(),
                                    bad.as_ptr() as *const c_void,
                                    bad.len(),
                                    w1.as_mut_ptr() as *mut c_void,
                                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                                    0,
                                );
                                let b = fr(
                                    dr.as_mut_ptr(),
                                    or.as_mut_ptr() as *mut c_void,
                                    or.len(),
                                    bad.as_ptr() as *const c_void,
                                    bad.len(),
                                    w2.as_mut_ptr() as *mut c_void,
                                    HUF_DECOMPRESS_WORKSPACE_SIZE,
                                    0,
                                );
                                eqv(&format!("HUF corrupted ret {tag}"), a, b);
                                eqbuf(&format!("HUF corrupted dst {tag}"), &oc, &or);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================ rows 139 / 140

const LL_BITS: [u8; 36] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
const LL_BASE: [u32; 36] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];
const LL_DEFAULT_NORM: [i16; 36] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
const LL_DEFAULT_NORM_LOG: u32 = 6;

const ML_BITS: [u8; 53] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
const ML_BASE: [u32; 53] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];
const ML_DEFAULT_NORM: [i16; 53] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
const ML_DEFAULT_NORM_LOG: u32 = 6;

const OF_BITS: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];
const OF_BASE: [u32; 32] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD, 0x1FFFFFFD, 0x3FFFFFFD, 0x7FFFFFFD,
];
const OF_DEFAULT_NORM: [i16; 29] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
const OF_DEFAULT_NORM_LOG: u32 = 5;

const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE: usize = 2 * (MaxSeq as usize + 1) + (1 << MaxFSELog) + 8;

type FnBuildFSETable = unsafe extern "C" fn(
    *mut u8,
    *const i16,
    c_uint,
    *const u32,
    *const u8,
    c_uint,
    *mut c_void,
    usize,
    c_int,
);
type FnDecodeSeqHeaders =
    unsafe extern "C" fn(*mut c_void, *mut c_int, *const c_void, usize) -> usize;

/// Produce a *valid* normalized distribution over `[0, maxSymbolValue]` that
/// sums to `1<<tableLog`, using the C FSE_normalizeCount.
unsafe fn make_norm(
    maxSymbolValue: u32,
    tableLog: u32,
    seed: u64,
    useLowProb: u32,
) -> Option<(Vec<i16>, u32)> {
    let (nc, _) = duo::<FnNormalizeCount>("FSE_normalizeCount");
    let mut rng = Rng::new(seed);
    let n = maxSymbolValue as usize + 1;
    let mut count = vec![0u32; n];
    let mut total = 0usize;
    // guarantee the top symbol is present so that maxSymbolValue is exact
    for i in 0..n {
        let c = if i == n - 1 {
            1 + rng.below(20)
        } else {
            rng.below(40)
        };
        count[i] = c as u32;
        total += c;
    }
    if total < 4 {
        return None;
    }
    let mut norm = vec![0i16; n.max(53)];
    let r = nc(
        norm.as_mut_ptr(),
        tableLog,
        count.as_ptr(),
        total,
        maxSymbolValue,
        useLowProb,
    );
    // r == 0 means "single symbol": tableLog 0 is not a usable FSE table
    if is_err(r) || (r as u32) < FSE_MIN_TABLELOG || (r as u32) > 15 {
        return None;
    }
    Some((norm, r as u32))
}

#[test]
fn row139_zstd_build_fse_table() {
    unsafe {
        let (fc, fr) = duo::<FnBuildFSETable>("ZSTD_buildFSETable");

        let table_sets: [(&str, &[u32], &[u8], u32); 3] = [
            ("LL", &LL_BASE, &LL_BITS, MaxLL),
            ("ML", &ML_BASE, &ML_BITS, MaxML),
            ("OF", &OF_BASE, &OF_BITS, MaxOff),
        ];

        // --- the three predefined ("set_basic") distributions
        let defaults: [(&str, &[i16], u32, u32, &[u32], &[u8]); 3] = [
            (
                "LL_default",
                &LL_DEFAULT_NORM,
                LL_DEFAULT_NORM_LOG,
                MaxLL,
                &LL_BASE,
                &LL_BITS,
            ),
            (
                "ML_default",
                &ML_DEFAULT_NORM,
                ML_DEFAULT_NORM_LOG,
                MaxML,
                &ML_BASE,
                &ML_BITS,
            ),
            (
                "OF_default",
                &OF_DEFAULT_NORM,
                OF_DEFAULT_NORM_LOG,
                28,
                &OF_BASE,
                &OF_BITS,
            ),
        ];
        for (nm, norm, log, max, base, bits) in defaults {
            for &bmi2 in &[0i32, 1] {
                let n = 1 + (1usize << log);
                let (mut dc, mut dr) = twin(n * 8);
                let mut w1 = vec![0u32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE];
                let mut w2 = vec![0u32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE];
                fc(
                    dc.as_mut_ptr(),
                    norm.as_ptr(),
                    max,
                    base.as_ptr(),
                    bits.as_ptr(),
                    log,
                    w1.as_mut_ptr() as *mut c_void,
                    w1.len() * 4,
                    bmi2,
                );
                fr(
                    dr.as_mut_ptr(),
                    norm.as_ptr(),
                    max,
                    base.as_ptr(),
                    bits.as_ptr(),
                    log,
                    w2.as_mut_ptr() as *mut c_void,
                    w2.len() * 4,
                    bmi2,
                );
                eqbuf(&format!("ZSTD_buildFSETable {nm} bmi2={bmi2}"), &dc, &dr);
            }
        }

        // --- randomized valid distributions over the whole (max, tableLog) grid
        let mut trials = 0u64;
        let mut built = 0u64;
        for (nm, base, bits, hardMax) in table_sets {
            for tableLog in 5..=MaxFSELog {
                for maxSymbolValue in [1u32, 2, 7, 15, 20, 28, 31, 35, 52] {
                    if maxSymbolValue > hardMax {
                        continue;
                    }
                    for useLowProb in [0u32, 1] {
                        for rep in 0..6u64 {
                            trials += 1;
                            let Some((norm, tl)) =
                                make_norm(maxSymbolValue, tableLog, 0x139 ^ trials ^ (rep << 32), useLowProb)
                            else {
                                continue;
                            };
                            if tl > MaxFSELog {
                                continue;
                            }
                            built += 1;
                            let n = 1 + (1usize << tl);
                            let (mut dc, mut dr) = twin(n * 8);
                            let mut w1 = vec![0u32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE];
                            let mut w2 = vec![0u32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE];
                            for &bmi2 in &[0i32, 1] {
                                fc(
                                    dc.as_mut_ptr(),
                                    norm.as_ptr(),
                                    maxSymbolValue,
                                    base.as_ptr(),
                                    bits.as_ptr(),
                                    tl,
                                    w1.as_mut_ptr() as *mut c_void,
                                    w1.len() * 4,
                                    bmi2,
                                );
                                fr(
                                    dr.as_mut_ptr(),
                                    norm.as_ptr(),
                                    maxSymbolValue,
                                    base.as_ptr(),
                                    bits.as_ptr(),
                                    tl,
                                    w2.as_mut_ptr() as *mut c_void,
                                    w2.len() * 4,
                                    bmi2,
                                );
                                eqbuf(
                                    &format!(
                                        "ZSTD_buildFSETable {nm} msv={maxSymbolValue} tl={tl} lp={useLowProb} bmi2={bmi2}"
                                    ),
                                    &dc,
                                    &dr,
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(trials > 100, "row139 grid unexpectedly small");
        assert!(built > 200, "row139 built only {built} tables");
    }
}

/// Assemble a valid `Sequences_Section` header.
unsafe fn make_seq_header(
    nbSeq: u32,
    llType: c_int,
    ofType: c_int,
    mlType: c_int,
    seed: u64,
) -> Option<Vec<u8>> {
    let (wn, _) = duo::<FnWriteNCount>("FSE_writeNCount");
    let mut out: Vec<u8> = Vec::new();
    // nbSeq field
    if nbSeq == 0 {
        out.push(0);
        return Some(out);
    } else if nbSeq < 0x80 {
        out.push(nbSeq as u8);
    } else if nbSeq < 0x7F00 {
        out.push(0x80 + (nbSeq >> 8) as u8);
        out.push((nbSeq & 0xFF) as u8);
    } else {
        out.push(0xFF);
        let v = (nbSeq - 0x7F00) as u16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.push(((llType as u8) << 6) | ((ofType as u8) << 4) | ((mlType as u8) << 2));
    for (i, &ty) in [llType, ofType, mlType].iter().enumerate() {
        let (max, fselog) = match i {
            0 => (MaxLL, LLFSELog),
            1 => (MaxOff, OffFSELog),
            _ => (MaxML, MLFSELog),
        };
        match ty {
            x if x == set_rle => out.push((seed % (max as u64 + 1)) as u8),
            x if x == set_compressed => {
                let msv = 1 + (seed as u32 + i as u32 * 7) % max;
                let tl = 5 + (seed as u32 + i as u32 * 3) % (fselog - 4);
                let (norm, tl) = make_norm(msv, tl, seed ^ ((i as u64) << 20), ((seed >> 3) & 1) as u32)?;
                if tl > fselog {
                    return None;
                }
                let mut buf = vec![0u8; 512];
                let n = wn(
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len(),
                    norm.as_ptr(),
                    msv,
                    tl,
                );
                if is_err(n) {
                    return None;
                }
                out.extend_from_slice(&buf[..n]);
            }
            _ => {}
        }
    }
    Some(out)
}

#[test]
fn row140_zstd_decode_seq_headers() {
    unsafe {
        let (fc, fr) = duo::<FnDecodeSeqHeaders>("ZSTD_decodeSeqHeaders");
        let (dec_c, dec_r) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");

        // Two flavours of DCtx: "cold" (fseEntropy == 0, so `set_repeat` must
        // fail) and "warm" (a real frame has been decoded, fseEntropy == 1, so
        // `set_repeat` succeeds).
        for warm in [false, true] {
            let ctx = CtxPair::dctx();
            if warm {
                let src = gen_class(4, 40_000, 0x140);
                let comp = c_compress(&src, 5);
                let mut oc = vec![0u8; src.len()];
                let mut or = vec![0u8; src.len()];
                let a = dec_c(
                    ctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    oc.len(),
                    comp.as_ptr() as *const c_void,
                    comp.len(),
                );
                let b = dec_r(
                    ctx.r,
                    or.as_mut_ptr() as *mut c_void,
                    or.len(),
                    comp.as_ptr() as *const c_void,
                    comp.len(),
                );
                eqv("warm-up decompress", a, b);
                eqbuf("warm-up decompress dst", &oc, &or);
            }

            let mut seed = 0x140u64;
            for &nbSeq in &[0u32, 1, 2, 24, 25, 127, 128, 0x7EFF, 0x7F00, 0x7F01, 0x8000] {
                for llType in 0..4 {
                    for ofType in 0..4 {
                        for mlType in 0..4 {
                            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                            let Some(hdr) = make_seq_header(nbSeq, llType, ofType, mlType, seed)
                            else {
                                continue;
                            };
                            for &extra in &[0usize, 1, 3] {
                                let mut buf = hdr.clone();
                                buf.extend(std::iter::repeat(0x5Au8).take(extra));
                                let tag = format!(
                                    "warm={warm} nbSeq={nbSeq} ll={llType} of={ofType} ml={mlType} extra={extra}"
                                );
                                let mut nc: c_int = -1;
                                let mut nr: c_int = -1;
                                let a = fc(
                                    ctx.c,
                                    &mut nc,
                                    buf.as_ptr() as *const c_void,
                                    buf.len(),
                                );
                                let b = fr(
                                    ctx.r,
                                    &mut nr,
                                    buf.as_ptr() as *const c_void,
                                    buf.len(),
                                );
                                eqv(&format!("ZSTD_decodeSeqHeaders ret {tag}"), a, b);
                                eqv(&format!("ZSTD_decodeSeqHeaders nbSeq {tag}"), nc, nr);
                            }
                            // truncations
                            for cut in 0..hdr.len().min(8) {
                                let part = &hdr[..cut];
                                let mut nc: c_int = -1;
                                let mut nr: c_int = -1;
                                let a = fc(
                                    ctx.c,
                                    &mut nc,
                                    part.as_ptr() as *const c_void,
                                    part.len(),
                                );
                                let b = fr(
                                    ctx.r,
                                    &mut nr,
                                    part.as_ptr() as *const c_void,
                                    part.len(),
                                );
                                eqv(
                                    &format!("ZSTD_decodeSeqHeaders truncated({cut}) ret"),
                                    a,
                                    b,
                                );
                                eqv(
                                    &format!("ZSTD_decodeSeqHeaders truncated({cut}) nbSeq"),
                                    nc,
                                    nr,
                                );
                            }
                        }
                    }
                }
            }
            // random garbage
            let mut rng = Rng::new(0x140 ^ warm as u64);
            for _ in 0..4000 {
                let n = 1 + rng.below(40);
                let buf = rng.bytes(n);
                let mut nc: c_int = -1;
                let mut nr: c_int = -1;
                let a = fc(ctx.c, &mut nc, buf.as_ptr() as *const c_void, buf.len());
                let b = fr(ctx.r, &mut nr, buf.as_ptr() as *const c_void, buf.len());
                eqv("ZSTD_decodeSeqHeaders(random) ret", a, b);
                eqv("ZSTD_decodeSeqHeaders(random) nbSeq", nc, nr);
            }
        }
    }
}

// ==================================================================== row 141

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct SeqDef {
    offBase: u32,
    litLength: u16,
    mlBase: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct SeqStore_t {
    sequencesStart: *mut SeqDef,
    sequences: *mut SeqDef,
    litStart: *mut u8,
    lit: *mut u8,
    llCode: *mut u8,
    mlCode: *mut u8,
    ofCode: *mut u8,
    maxNbSeq: usize,
    maxNbLit: usize,
    longLengthType: c_int,
    longLengthPos: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_hufCTables_t {
    CTable: [u64; 257],
    repeatMode: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_fseCTables_t {
    offcodeCTable: [u32; OFF_CTABLE_U32],
    matchlengthCTable: [u32; ML_CTABLE_U32],
    litlengthCTable: [u32; LL_CTABLE_U32],
    offcode_repeatMode: c_int,
    matchlength_repeatMode: c_int,
    litlength_repeatMode: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_entropyCTables_t {
    huf: ZSTD_hufCTables_t,
    fse: ZSTD_fseCTables_t,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_hufCTablesMetadata_t {
    hType: c_int,
    hufDesBuffer: [u8; ZSTD_MAX_HUF_HEADER_SIZE],
    hufDesSize: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_fseCTablesMetadata_t {
    llType: c_int,
    ofType: c_int,
    mlType: c_int,
    fseTablesBuffer: [u8; ZSTD_MAX_FSE_HEADERS_SIZE],
    fseTablesSize: usize,
    lastCountSize: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_entropyCTablesMetadata_t {
    hufMetadata: ZSTD_hufCTablesMetadata_t,
    fseMetadata: ZSTD_fseCTablesMetadata_t,
}

type FnSeqToCodes = unsafe extern "C" fn(*const SeqStore_t) -> c_int;
type FnSelectEncodingType = unsafe extern "C" fn(
    *mut c_int,
    *const c_uint,
    c_uint,
    usize,
    usize,
    c_uint,
    *const u32,
    *const i16,
    u32,
    c_int,
    c_int,
) -> c_int;
type FnZstdBuildCTable = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *mut u32,
    u32,
    c_int,
    *mut c_uint,
    u32,
    *const u8,
    usize,
    *const i16,
    u32,
    u32,
    *const u32,
    usize,
    *mut c_void,
    usize,
) -> usize;
type FnEncodeSequences = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const u32,
    *const u8,
    *const SeqDef,
    usize,
    c_int,
    c_int,
) -> usize;
type FnFseBitCost = unsafe extern "C" fn(*const u32, *const c_uint, c_uint) -> usize;
type FnCrossEntropyCost = unsafe extern "C" fn(*const i16, c_uint, *const c_uint, c_uint) -> usize;
type FnBuildBlockEntropyStats = unsafe extern "C" fn(
    *const SeqStore_t,
    *const ZSTD_entropyCTables_t,
    *mut ZSTD_entropyCTables_t,
    *const c_void,
    *mut ZSTD_entropyCTablesMetadata_t,
    *mut c_void,
    usize,
) -> usize;

/// Synthetic sequence set that exercises a broad slice of the LL/ML/OF code
/// alphabets (all `offBase >= 1`, as `ZSTD_highbit32` requires).
fn gen_seqs(nbSeq: usize, seed: u64) -> Vec<SeqDef> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::with_capacity(nbSeq);
    for i in 0..nbSeq {
        let ll = match i % 5 {
            0 => rng.below(4) as u16,
            1 => rng.below(20) as u16,
            2 => rng.below(300) as u16,
            3 => rng.below(5000) as u16,
            _ => rng.below(0xFFFF) as u16,
        };
        let ml = match i % 4 {
            0 => rng.below(8) as u16,
            1 => rng.below(64) as u16,
            2 => rng.below(1000) as u16,
            _ => rng.below(0xFFFF) as u16,
        };
        let shift = (i % 29) as u32;
        let ofb = (1u32 << shift) | (rng.next_u32() & ((1u32 << shift) - 1).max(0));
        v.push(SeqDef {
            offBase: ofb.max(1) & 0x7FFF_FFFF,
            litLength: ll,
            mlBase: ml,
        });
    }
    v
}

struct SeqStoreTwin {
    seqs: Vec<SeqDef>,
    lits: Vec<u8>,
    ll: Vec<u8>,
    ml: Vec<u8>,
    of: Vec<u8>,
}

impl SeqStoreTwin {
    fn new(seqs: Vec<SeqDef>, lits: Vec<u8>) -> Self {
        let n = seqs.len().max(1);
        SeqStoreTwin {
            seqs,
            lits,
            ll: vec![0xA5; n + 8],
            ml: vec![0xA5; n + 8],
            of: vec![0xA5; n + 8],
        }
    }
    fn store(&mut self, longLengthType: c_int, longLengthPos: u32) -> SeqStore_t {
        let n = self.seqs.len();
        SeqStore_t {
            sequencesStart: self.seqs.as_mut_ptr(),
            sequences: unsafe { self.seqs.as_mut_ptr().add(n) },
            litStart: self.lits.as_mut_ptr(),
            lit: unsafe { self.lits.as_mut_ptr().add(self.lits.len()) },
            llCode: self.ll.as_mut_ptr(),
            mlCode: self.ml.as_mut_ptr(),
            ofCode: self.of.as_mut_ptr(),
            maxNbSeq: n + 8,
            maxNbLit: self.lits.len(),
            longLengthType,
            longLengthPos,
        }
    }
}

#[test]
fn row141_zstd_compress_sequences() {
    unsafe {
        let (sc_c, sc_r) = duo::<FnSeqToCodes>("ZSTD_seqToCodes");
        let (se_c, se_r) = duo::<FnSelectEncodingType>("ZSTD_selectEncodingType");
        let (bt_c, bt_r) = duo::<FnZstdBuildCTable>("ZSTD_buildCTable");
        let (en_c, en_r) = duo::<FnEncodeSequences>("ZSTD_encodeSequences");
        let (fb_c, fb_r) = duo::<FnFseBitCost>("ZSTD_fseBitCost");
        let (ce_c, ce_r) = duo::<FnCrossEntropyCost>("ZSTD_crossEntropyCost");
        let (bs_c, bs_r) = duo::<FnBuildBlockEntropyStats>("ZSTD_buildBlockEntropyStats");
        let (hist, _) = duo::<FnHistCountWksp>("HIST_count_wksp");
        let (setp_c, setp_r) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");

        // channel descriptions: (name, FSELog, maxCode, defaultNorm, defaultNormLog, defaultMax, ctableU32)
        struct Chan {
            name: &'static str,
            fselog: u32,
            defnorm: &'static [i16],
            defnormlog: u32,
            defmax: u32,
            ct_u32: usize,
        }
        let chans = [
            Chan {
                name: "LL",
                fselog: LLFSELog,
                defnorm: &LL_DEFAULT_NORM,
                defnormlog: LL_DEFAULT_NORM_LOG,
                defmax: MaxLL,
                ct_u32: LL_CTABLE_U32,
            },
            Chan {
                name: "OF",
                fselog: OffFSELog,
                defnorm: &OF_DEFAULT_NORM,
                defnormlog: OF_DEFAULT_NORM_LOG,
                defmax: 28,
                ct_u32: OFF_CTABLE_U32,
            },
            Chan {
                name: "ML",
                fselog: MLFSELog,
                defnorm: &ML_DEFAULT_NORM,
                defnormlog: ML_DEFAULT_NORM_LOG,
                defmax: MaxML,
                ct_u32: ML_CTABLE_U32,
            },
        ];

        let params = CtxPair::cctx_params();

        for &nbSeq in &[1usize, 2, 3, 4, 8, 40, 300, 1024, 2047, 2048, 2100, 5000] {
            for trial in 0..3u64 {
                let seqs = gen_seqs(nbSeq, 0x141 ^ (nbSeq as u64) ^ (trial << 16));
                for llt in 0..3 {
                    let lpos = if nbSeq > 1 { (trial as u32) % nbSeq as u32 } else { 0 };
                    for &litClass in &[0usize, 3, 4, 6] {
                        let litSize = match trial {
                            0 => 0usize,
                            1 => 40,
                            _ => 3000,
                        };
                        let lits = gen_class(litClass, litSize, 0x141 ^ trial);
                        let tagbase = format!(
                            "nbSeq={nbSeq} trial={trial} llt={llt} litClass={litClass} litSize={litSize}"
                        );
                        let mut tw_c = SeqStoreTwin::new(seqs.clone(), lits.clone());
                        let mut tw_r = SeqStoreTwin::new(seqs.clone(), lits.clone());
                        let ss_c = tw_c.store(llt, lpos);
                        let ss_r = tw_r.store(llt, lpos);

                        // ---- ZSTD_seqToCodes
                        let a = sc_c(&ss_c);
                        let b = sc_r(&ss_r);
                        eqv(&format!("ZSTD_seqToCodes ret {tagbase}"), a, b);
                        eqbuf(&format!("ZSTD_seqToCodes llCode {tagbase}"), &tw_c.ll, &tw_r.ll);
                        eqbuf(&format!("ZSTD_seqToCodes mlCode {tagbase}"), &tw_c.ml, &tw_r.ml);
                        eqbuf(&format!("ZSTD_seqToCodes ofCode {tagbase}"), &tw_c.of, &tw_r.of);
                        let longOffsetsFromCodes = a;

                        // ---- histograms of the three code streams
                        let mut counts: Vec<Vec<c_uint>> = Vec::new();
                        let mut maxes: Vec<u32> = Vec::new();
                        let mut mosts: Vec<usize> = Vec::new();
                        for (ci, ch) in chans.iter().enumerate() {
                            let codes: &[u8] = match ci {
                                0 => &tw_c.ll[..nbSeq],
                                1 => &tw_c.of[..nbSeq],
                                _ => &tw_c.ml[..nbSeq],
                            };
                            let mut count = vec![0u32; 64];
                            let mut msv: c_uint = ch.defmax.min(63);
                            let mut wk = vec![0u32; 1024];
                            let largest = hist(
                                count.as_mut_ptr(),
                                &mut msv,
                                codes.as_ptr() as *const c_void,
                                codes.len(),
                                wk.as_mut_ptr() as *mut c_void,
                                wk.len() * 4,
                            );
                            if is_err(largest) {
                                // a code exceeded the channel's default alphabet
                                counts.push(Vec::new());
                                maxes.push(0);
                                mosts.push(0);
                                continue;
                            }
                            counts.push(count);
                            maxes.push(msv);
                            mosts.push(largest);
                        }

                        // ---- per channel: buildCTable / selectEncodingType / costs
                        let mut ctables_c: Vec<Vec<u32>> = Vec::new();
                        let mut ctables_r: Vec<Vec<u32>> = Vec::new();
                        let mut ct_ok: Vec<bool> = Vec::new();
                        for (ci, ch) in chans.iter().enumerate() {
                            if counts[ci].is_empty() {
                                ctables_c.push(Vec::new());
                                ctables_r.push(Vec::new());
                                ct_ok.push(false);
                                continue;
                            }
                            let count = &counts[ci];
                            let max = maxes[ci];
                            let most = mosts[ci];
                            let codes: &[u8] = match ci {
                                0 => &tw_c.ll[..nbSeq],
                                1 => &tw_c.of[..nbSeq],
                                _ => &tw_c.ml[..nbSeq],
                            };
                            let tag = format!("{} {tagbase}", ch.name);

                            // set_compressed table (also used as prevCTable later)
                            let mut prev_c = vec![0xA5A5_A5A5u32; ch.ct_u32];
                            let mut prev_r = vec![0xA5A5_A5A5u32; ch.ct_u32];
                            let mut have_prev = false;
                            if nbSeq >= 4 && max >= 1 && most < nbSeq {
                                let mut cc = count.clone();
                                let mut cr = count.clone();
                                let (mut oc, mut or) = twin(512);
                                let mut w1 = vec![0u64; 4096];
                                let mut w2 = vec![0u64; 4096];
                                let x = bt_c(
                                    oc.as_mut_ptr() as *mut c_void,
                                    oc.len(),
                                    prev_c.as_mut_ptr(),
                                    ch.fselog,
                                    set_compressed,
                                    cc.as_mut_ptr(),
                                    max,
                                    codes.as_ptr(),
                                    nbSeq,
                                    ch.defnorm.as_ptr(),
                                    ch.defnormlog,
                                    ch.defmax,
                                    std::ptr::null(),
                                    0,
                                    w1.as_mut_ptr() as *mut c_void,
                                    w1.len() * 8,
                                );
                                let y = bt_r(
                                    or.as_mut_ptr() as *mut c_void,
                                    or.len(),
                                    prev_r.as_mut_ptr(),
                                    ch.fselog,
                                    set_compressed,
                                    cr.as_mut_ptr(),
                                    max,
                                    codes.as_ptr(),
                                    nbSeq,
                                    ch.defnorm.as_ptr(),
                                    ch.defnormlog,
                                    ch.defmax,
                                    std::ptr::null(),
                                    0,
                                    w2.as_mut_ptr() as *mut c_void,
                                    w2.len() * 8,
                                );
                                eqv(&format!("ZSTD_buildCTable set_compressed ret {tag}"), x, y);
                                eqbuf(&format!("ZSTD_buildCTable set_compressed dst {tag}"), &oc, &or);
                                eqbuf(
                                    &format!("ZSTD_buildCTable set_compressed nextCTable {tag}"),
                                    as_bytes32(&prev_c),
                                    as_bytes32(&prev_r),
                                );
                                eqbuf(
                                    &format!("ZSTD_buildCTable set_compressed count[] {tag}"),
                                    as_bytes32(&cc),
                                    as_bytes32(&cr),
                                );
                                have_prev = !is_err(x);
                            }

                            // ---- ZSTD_fseBitCost (needs a valid CTable)
                            if have_prev {
                                for &m in &[0u32, 1, max] {
                                    eqv(
                                        &format!("ZSTD_fseBitCost(max={m}) {tag}"),
                                        fb_c(prev_c.as_ptr(), count.as_ptr(), m),
                                        fb_r(prev_r.as_ptr(), count.as_ptr(), m),
                                    );
                                }
                            }
                            // ---- ZSTD_crossEntropyCost
                            for &m in &[0u32, 1, max.min(ch.defmax)] {
                                eqv(
                                    &format!("ZSTD_crossEntropyCost(max={m}) {tag}"),
                                    ce_c(ch.defnorm.as_ptr(), ch.defnormlog, count.as_ptr(), m),
                                    ce_r(ch.defnorm.as_ptr(), ch.defnormlog, count.as_ptr(), m),
                                );
                            }

                            // ---- ZSTD_selectEncodingType over the full axis set
                            if max <= ch.defmax {
                                for &rm0 in &[FSE_repeat_none, FSE_repeat_check, FSE_repeat_valid] {
                                    for &isDefault in &[0i32, 1] {
                                        for &strategy in &ALL_STRATEGIES {
                                            if rm0 != FSE_repeat_none && !have_prev {
                                                continue;
                                            }
                                            let mut mc = rm0;
                                            let mut mr = rm0;
                                            let x = se_c(
                                                &mut mc,
                                                count.as_ptr(),
                                                max,
                                                most,
                                                nbSeq,
                                                ch.fselog,
                                                prev_c.as_ptr(),
                                                ch.defnorm.as_ptr(),
                                                ch.defnormlog,
                                                isDefault,
                                                strategy,
                                            );
                                            let y = se_r(
                                                &mut mr,
                                                count.as_ptr(),
                                                max,
                                                most,
                                                nbSeq,
                                                ch.fselog,
                                                prev_r.as_ptr(),
                                                ch.defnorm.as_ptr(),
                                                ch.defnormlog,
                                                isDefault,
                                                strategy,
                                            );
                                            eqv(
                                                &format!("ZSTD_selectEncodingType {tag} rm={rm0} def={isDefault} strat={strategy}"),
                                                x,
                                                y,
                                            );
                                            eqv(
                                                &format!("ZSTD_selectEncodingType *repeatMode {tag} rm={rm0} def={isDefault} strat={strategy}"),
                                                mc,
                                                mr,
                                            );
                                        }
                                    }
                                }
                            }

                            // ---- ZSTD_buildCTable for the other encoding types
                            for &ty in &[set_basic, set_rle, set_repeat] {
                                if ty == set_repeat && !have_prev {
                                    continue;
                                }
                                for &cap in &[512usize, 1, 0] {
                                    let mut cc = count.clone();
                                    let mut cr = count.clone();
                                    let mut nc = vec![0xA5A5_A5A5u32; ch.ct_u32];
                                    let mut nr = vec![0xA5A5_A5A5u32; ch.ct_u32];
                                    let (mut oc, mut or) = twin(cap.max(1));
                                    let mut w1 = vec![0u64; 4096];
                                    let mut w2 = vec![0u64; 4096];
                                    let x = bt_c(
                                        oc.as_mut_ptr() as *mut c_void,
                                        cap,
                                        nc.as_mut_ptr(),
                                        ch.fselog,
                                        ty,
                                        cc.as_mut_ptr(),
                                        max,
                                        codes.as_ptr(),
                                        nbSeq,
                                        ch.defnorm.as_ptr(),
                                        ch.defnormlog,
                                        ch.defmax,
                                        prev_c.as_ptr(),
                                        ch.ct_u32 * 4,
                                        w1.as_mut_ptr() as *mut c_void,
                                        w1.len() * 8,
                                    );
                                    let y = bt_r(
                                        or.as_mut_ptr() as *mut c_void,
                                        cap,
                                        nr.as_mut_ptr(),
                                        ch.fselog,
                                        ty,
                                        cr.as_mut_ptr(),
                                        max,
                                        codes.as_ptr(),
                                        nbSeq,
                                        ch.defnorm.as_ptr(),
                                        ch.defnormlog,
                                        ch.defmax,
                                        prev_r.as_ptr(),
                                        ch.ct_u32 * 4,
                                        w2.as_mut_ptr() as *mut c_void,
                                        w2.len() * 8,
                                    );
                                    eqv(&format!("ZSTD_buildCTable type={ty} cap={cap} ret {tag}"), x, y);
                                    eqbuf(
                                        &format!("ZSTD_buildCTable type={ty} cap={cap} dst {tag}"),
                                        &oc,
                                        &or,
                                    );
                                    eqbuf(
                                        &format!("ZSTD_buildCTable type={ty} cap={cap} nextCTable {tag}"),
                                        as_bytes32(&nc),
                                        as_bytes32(&nr),
                                    );
                                }
                            }
                            ctables_c.push(prev_c);
                            ctables_r.push(prev_r);
                            ct_ok.push(have_prev);
                        }

                        // ---- ZSTD_encodeSequences with the three set_compressed tables
                        if ct_ok.len() == 3 && ct_ok.iter().all(|&v| v) {
                            for &longOffsets in &[0i32, 1, longOffsetsFromCodes] {
                                for &bmi2 in &[0i32, 1] {
                                    for &capMode in &[0usize, 1, 2] {
                                        let full = nbSeq * 8 + 128;
                                        let cap = match capMode {
                                            0 => full,
                                            1 => full / 4,
                                            _ => 4,
                                        };
                                        let (mut oc, mut or) = twin(cap);
                                        let x = en_c(
                                            oc.as_mut_ptr() as *mut c_void,
                                            cap,
                                            ctables_c[2].as_ptr(),
                                            tw_c.ml.as_ptr(),
                                            ctables_c[1].as_ptr(),
                                            tw_c.of.as_ptr(),
                                            ctables_c[0].as_ptr(),
                                            tw_c.ll.as_ptr(),
                                            tw_c.seqs.as_ptr(),
                                            nbSeq,
                                            longOffsets,
                                            bmi2,
                                        );
                                        let y = en_r(
                                            or.as_mut_ptr() as *mut c_void,
                                            cap,
                                            ctables_r[2].as_ptr(),
                                            tw_r.ml.as_ptr(),
                                            ctables_r[1].as_ptr(),
                                            tw_r.of.as_ptr(),
                                            ctables_r[0].as_ptr(),
                                            tw_r.ll.as_ptr(),
                                            tw_r.seqs.as_ptr(),
                                            nbSeq,
                                            longOffsets,
                                            bmi2,
                                        );
                                        eqv(
                                            &format!("ZSTD_encodeSequences ret {tagbase} lo={longOffsets} bmi2={bmi2} capMode={capMode}"),
                                            x,
                                            y,
                                        );
                                        eqbuf(
                                            &format!("ZSTD_encodeSequences dst {tagbase} lo={longOffsets} bmi2={bmi2} capMode={capMode}"),
                                            &oc,
                                            &or,
                                        );
                                    }
                                }
                            }
                        }

                        // ---- ZSTD_buildBlockEntropyStats
                        for &strategy in &[1i32, 4, 8, 9] {
                            for &litMode in &[0i32, 1, 2] {
                                eqv(
                                    "CCtxParams_setParameter(strategy)",
                                    setp_c(params.c, ZSTD_c_strategy, strategy),
                                    setp_r(params.r, ZSTD_c_strategy, strategy),
                                );
                                eqv(
                                    "CCtxParams_setParameter(literalCompressionMode)",
                                    setp_c(params.c, ZSTD_c_literalCompressionMode, litMode),
                                    setp_r(params.r, ZSTD_c_literalCompressionMode, litMode),
                                );
                                for &hufRepeat in &[HUF_repeat_none, HUF_repeat_check] {
                                    for &fseRepeat in
                                        &[FSE_repeat_none, FSE_repeat_check, FSE_repeat_valid]
                                    {
                                        let mut prev: ZSTD_entropyCTables_t = std::mem::zeroed();
                                        prev.huf.repeatMode = hufRepeat;
                                        prev.fse.offcode_repeatMode = fseRepeat;
                                        prev.fse.matchlength_repeatMode = fseRepeat;
                                        prev.fse.litlength_repeatMode = fseRepeat;
                                        let mut nxt_c: ZSTD_entropyCTables_t = std::mem::zeroed();
                                        let mut nxt_r: ZSTD_entropyCTables_t = std::mem::zeroed();
                                        let mut md_c: ZSTD_entropyCTablesMetadata_t =
                                            std::mem::zeroed();
                                        let mut md_r: ZSTD_entropyCTablesMetadata_t =
                                            std::mem::zeroed();
                                        let mut w1 = vec![0u64; 4096];
                                        let mut w2 = vec![0u64; 4096];
                                        let x = bs_c(
                                            &ss_c,
                                            &prev,
                                            &mut nxt_c,
                                            params.c,
                                            &mut md_c,
                                            w1.as_mut_ptr() as *mut c_void,
                                            w1.len() * 8,
                                        );
                                        let y = bs_r(
                                            &ss_r,
                                            &prev,
                                            &mut nxt_r,
                                            params.r,
                                            &mut md_r,
                                            w2.as_mut_ptr() as *mut c_void,
                                            w2.len() * 8,
                                        );
                                        let t = format!(
                                            "{tagbase} strat={strategy} lit={litMode} huf={hufRepeat} fse={fseRepeat}"
                                        );
                                        eqv(&format!("ZSTD_buildBlockEntropyStats ret {t}"), x, y);
                                        eqbuf(
                                            &format!("ZSTD_buildBlockEntropyStats nextEntropy {t}"),
                                            raw_bytes(&nxt_c),
                                            raw_bytes(&nxt_r),
                                        );
                                        eqbuf(
                                            &format!("ZSTD_buildBlockEntropyStats metadata {t}"),
                                            raw_bytes(&md_c),
                                            raw_bytes(&md_r),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================ rows 142 / 143

type FnNoCompressLiterals =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
type FnCompressLiterals = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    *const ZSTD_hufCTables_t,
    *mut ZSTD_hufCTables_t,
    c_int,
    c_int,
    c_int,
    c_int,
) -> usize;
type FnCompressSuperBlock =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, c_uint) -> usize;

#[test]
fn row142_zstd_literals_blocks() {
    unsafe {
        let (nc_c, nc_r) = duo::<FnNoCompressLiterals>("ZSTD_noCompressLiterals");
        let (rl_c, rl_r) = duo::<FnNoCompressLiterals>("ZSTD_compressRleLiteralsBlock");
        let (cl_c, cl_r) = duo::<FnCompressLiterals>("ZSTD_compressLiterals");

        let sizes: [usize; 12] = [
            0,
            1,
            2,
            5,
            6,
            7,
            62,
            63,
            64,
            1024,
            8 * 1024,
            128 * 1024,
        ];
        for class in 0..N_CLASSES {
            for &size in &sizes {
                let src = gen_class(class, size, 0x142);
                let tag = format!("class={} size={size}", CLASS_NAMES[class]);
                // ---- ZSTD_noCompressLiterals over a capacity sweep
                for &cap in &[size + 8, size + 4, size + 3, size + 1, size, 1, 0] {
                    let (mut oc, mut or) = twin(cap.max(1));
                    let a = nc_c(
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    let b = nc_r(
                        or.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        size,
                    );
                    eqv(&format!("ZSTD_noCompressLiterals ret {tag} cap={cap}"), a, b);
                    eqbuf(&format!("ZSTD_noCompressLiterals dst {tag} cap={cap}"), &oc, &or);
                }
                // ---- ZSTD_compressRleLiteralsBlock (needs an all-equal buffer)
                if size > 0 {
                    let rle = vec![src[0]; size];
                    for &cap in &[8usize, 5, 4, 3, 2, 1, 0] {
                        let (mut oc, mut or) = twin(cap.max(1));
                        let a = rl_c(
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            rle.as_ptr() as *const c_void,
                            size,
                        );
                        let b = rl_r(
                            or.as_mut_ptr() as *mut c_void,
                            cap,
                            rle.as_ptr() as *const c_void,
                            size,
                        );
                        eqv(
                            &format!("ZSTD_compressRleLiteralsBlock ret {tag} cap={cap}"),
                            a,
                            b,
                        );
                        eqbuf(
                            &format!("ZSTD_compressRleLiteralsBlock dst {tag} cap={cap}"),
                            &oc,
                            &or,
                        );
                    }
                }
                // ---- ZSTD_compressLiterals
                if size == 0 {
                    continue;
                }
                for &strategy in &ALL_STRATEGIES {
                    for &disable in &[0i32, 1] {
                        for &suspect in &[0i32, 1] {
                            for &bmi2 in &[0i32, 1] {
                                for &repeat0 in
                                    &[HUF_repeat_none, HUF_repeat_check, HUF_repeat_valid]
                                {
                                    let mut prev_c: ZSTD_hufCTables_t = std::mem::zeroed();
                                    let mut prev_r: ZSTD_hufCTables_t = std::mem::zeroed();
                                    if repeat0 != HUF_repeat_none {
                                        // prime prevHuf with a real table
                                        let mut n_c: ZSTD_hufCTables_t = std::mem::zeroed();
                                        let mut n_r: ZSTD_hufCTables_t = std::mem::zeroed();
                                        let mut w1 = vec![0u64; 4096];
                                        let mut w2 = vec![0u64; 4096];
                                        let (mut oc, mut or) = twin(size + 1024);
                                        let a = cl_c(
                                            oc.as_mut_ptr() as *mut c_void,
                                            oc.len(),
                                            src.as_ptr() as *const c_void,
                                            size,
                                            w1.as_mut_ptr() as *mut c_void,
                                            w1.len() * 8,
                                            &prev_c,
                                            &mut n_c,
                                            strategy,
                                            0,
                                            0,
                                            bmi2,
                                        );
                                        let b = cl_r(
                                            or.as_mut_ptr() as *mut c_void,
                                            or.len(),
                                            src.as_ptr() as *const c_void,
                                            size,
                                            w2.as_mut_ptr() as *mut c_void,
                                            w2.len() * 8,
                                            &prev_r,
                                            &mut n_r,
                                            strategy,
                                            0,
                                            0,
                                            bmi2,
                                        );
                                        eqv(&format!("ZSTD_compressLiterals priming {tag}"), a, b);
                                        eqbuf(
                                            &format!("ZSTD_compressLiterals priming dst {tag}"),
                                            &oc,
                                            &or,
                                        );
                                        eqbuf(
                                            &format!("ZSTD_compressLiterals priming nextHuf {tag}"),
                                            raw_bytes(&n_c),
                                            raw_bytes(&n_r),
                                        );
                                        if n_c.repeatMode == HUF_repeat_none {
                                            continue; // no reusable table produced
                                        }
                                        prev_c = n_c;
                                        prev_r = n_r;
                                        prev_c.repeatMode = repeat0;
                                        prev_r.repeatMode = repeat0;
                                    }
                                    for &capMode in &[0usize, 1, 2] {
                                        let full = size + 1024;
                                        let cap = match capMode {
                                            0 => full,
                                            1 => size / 2 + 8,
                                            _ => 3,
                                        };
                                        let mut n_c: ZSTD_hufCTables_t = std::mem::zeroed();
                                        let mut n_r: ZSTD_hufCTables_t = std::mem::zeroed();
                                        let mut w1 = vec![0u64; 4096];
                                        let mut w2 = vec![0u64; 4096];
                                        let (mut oc, mut or) = twin(cap.max(1));
                                        let a = cl_c(
                                            oc.as_mut_ptr() as *mut c_void,
                                            cap,
                                            src.as_ptr() as *const c_void,
                                            size,
                                            w1.as_mut_ptr() as *mut c_void,
                                            w1.len() * 8,
                                            &prev_c,
                                            &mut n_c,
                                            strategy,
                                            disable,
                                            suspect,
                                            bmi2,
                                        );
                                        let b = cl_r(
                                            or.as_mut_ptr() as *mut c_void,
                                            cap,
                                            src.as_ptr() as *const c_void,
                                            size,
                                            w2.as_mut_ptr() as *mut c_void,
                                            w2.len() * 8,
                                            &prev_r,
                                            &mut n_r,
                                            strategy,
                                            disable,
                                            suspect,
                                            bmi2,
                                        );
                                        let t = format!(
                                            "{tag} strat={strategy} dis={disable} sus={suspect} bmi2={bmi2} rep={repeat0} capMode={capMode}"
                                        );
                                        eqv(&format!("ZSTD_compressLiterals ret {t}"), a, b);
                                        eqbuf(&format!("ZSTD_compressLiterals dst {t}"), &oc, &or);
                                        eqbuf(
                                            &format!("ZSTD_compressLiterals nextHuf {t}"),
                                            raw_bytes(&n_c),
                                            raw_bytes(&n_r),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row143_zstd_compress_super_block() {
    unsafe {
        let (sb_c, sb_r) = duo::<FnCompressSuperBlock>("ZSTD_compressSuperBlock");
        let (set_c, set_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2_c, c2_r) = duo::<FnCompressCCtx>("ZSTD_compress2");
        let (bound, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        // targetCBlockSize grid (0 == off, ZSTD_TARGETCBLOCKSIZE_MIN == 1340)
        let tcbs: [c_int; 6] = [0, 1340, 2048, 4096, 65536, 131072];
        for class in 0..N_CLASSES {
            for &size in &[1024usize, 20_000, 100_000, 131_072] {
                let src = gen_class(class, size, 0x143);
                for &t in &tcbs {
                    for &level in &[1i32, 3, 9, 19] {
                        let ctx = CtxPair::cctx();
                        for (n, f, g) in [
                            ("compressionLevel", set_c, set_r),
                            ("targetCBlockSize", set_c, set_r),
                        ] {
                            let (p, v) = if n == "compressionLevel" {
                                (ZSTD_c_compressionLevel, level)
                            } else {
                                (ZSTD_c_targetCBlockSize, t)
                            };
                            eqv(&format!("setParameter({n})"), f(ctx.c, p, v), g(ctx.r, p, v));
                        }
                        // A full compression leaves cctx->seqStore holding the last
                        // block's sequences, consistent with `src` when `src` is a
                        // single block, and initialises both blockStates.
                        let cap = bound(src.len());
                        let mut ac = vec![0u8; cap];
                        let mut ar = vec![0u8; cap];
                        let x = c2_c(
                            ctx.c,
                            ac.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            0,
                        );
                        let y = c2_r(
                            ctx.r,
                            ar.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            0,
                        );
                        let tag = format!(
                            "class={} size={size} tcbs={t} level={level}",
                            CLASS_NAMES[class]
                        );
                        eqv(&format!("ZSTD_compress2 ret {tag}"), x, y);
                        assert!(!is_err(x), "compress2 failed for {tag}");
                        eqbuf(&format!("ZSTD_compress2 dst {tag}"), &ac[..x], &ar[..y]);

                        for &lastBlock in &[0u32, 1] {
                            for &capMode in &[0usize, 1, 2] {
                                let scap = match capMode {
                                    0 => src.len() + 1024,
                                    1 => src.len() / 2 + 8,
                                    _ => 8,
                                };
                                let (mut oc, mut or) = twin(scap);
                                let a = sb_c(
                                    ctx.c,
                                    oc.as_mut_ptr() as *mut c_void,
                                    scap,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    lastBlock,
                                );
                                let b = sb_r(
                                    ctx.r,
                                    or.as_mut_ptr() as *mut c_void,
                                    scap,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    lastBlock,
                                );
                                eqv(
                                    &format!("ZSTD_compressSuperBlock ret {tag} last={lastBlock} capMode={capMode}"),
                                    a,
                                    b,
                                );
                                eqbuf(
                                    &format!("ZSTD_compressSuperBlock dst {tag} last={lastBlock} capMode={capMode}"),
                                    &oc,
                                    &or,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================ rows 144 / 145

type FnSelectBlockCompressor = unsafe extern "C" fn(c_int, c_int, c_int) -> *mut c_void;
type FnCreateCDictAdv2 = unsafe extern "C" fn(
    *const c_void,
    usize,
    c_int,
    c_int,
    *const c_void,
    ZSTD_customMem,
) -> *mut c_void;
type FnRefCDict = unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize;
type FnRefPrefix = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;

/// Row 144: the 9 x 3 x 4 dispatch table of `ZSTD_selectBlockCompressor`.
#[test]
fn row144_select_block_compressor() {
    unsafe {
        let (fc, fr) = duo::<FnSelectBlockCompressor>("ZSTD_selectBlockCompressor");
        let mut nonnull = 0usize;
        for &strat in &ALL_STRATEGIES {
            for &rmf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                for dictMode in 0..4i32 {
                    let a = fc(strat, rmf, dictMode);
                    let b = fr(strat, rmf, dictMode);
                    eqv(
                        &format!(
                            "ZSTD_selectBlockCompressor(strat={strat},rowMatchFinder={rmf},dictMode={dictMode}) non-NULL"
                        ),
                        a.is_null(),
                        b.is_null(),
                    );
                    if !a.is_null() {
                        nonnull += 1;
                    }
                }
            }
        }
        // 18 of the 108 triples have no specialised compressor (e.g. the
        // dedicatedDictSearch variants only exist for greedy/lazy/lazy2).
        assert_eq!(nonnull, 90, "block-compressor dispatch table changed");
    }
}

/// Rows 144/145: drive every exported `ZSTD_compressBlock_*` end-to-end via
/// the (strategy x dictMode x rowMatchFinder) cross-product.
///
/// `ZSTD_fillHashTable`, `ZSTD_fillDoubleHashTable`,
/// `ZSTD_insertAndFindFirstIndex`, `ZSTD_row_update` and `ZSTD_updateTree` take
/// a `ZSTD_MatchState_t*` whose internal tables live inside a `ZSTD_cwksp`
/// owned by a CCtx and are not reachable through any exported symbol, so they
/// are exercised through this cross-product rather than called directly.
#[test]
fn row145_all_block_compressors_e2e() {
    unsafe {
        let (set_c, set_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2_c, c2_r) = duo::<FnCompressCCtx>("ZSTD_compress2");
        let (bound, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (mkcd_c, mkcd_r) = duo::<FnCreateCDictAdv2>("ZSTD_createCDict_advanced2");
        let (frcd_c, frcd_r) = duo::<FnFreePtr>("ZSTD_freeCDict");
        let (refcd_c, refcd_r) = duo::<FnRefCDict>("ZSTD_CCtx_refCDict");
        let (refpx_c, refpx_r) = duo::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let (cpset_c, cpset_r) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");

        let dict = gen_class(4, 48 * 1024, 0xD1C7);

        for &size in &[8 * 1024usize, 40_000, 150_000] {
            for class in 0..N_CLASSES {
                let src = gen_class(class, size, 0x145);
                let cap = bound(src.len());
                for &strat in &ALL_STRATEGIES {
                    for &rmf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                        for dictMode in 0..4usize {
                            let tag = format!(
                                "class={} size={size} strat={strat} rmf={rmf} dictMode={dictMode}",
                                CLASS_NAMES[class]
                            );
                            let ctx = CtxPair::cctx();
                            let mut cd_c: *mut c_void = std::ptr::null_mut();
                            let mut cd_r: *mut c_void = std::ptr::null_mut();
                            let mut params: Option<CtxPair> = None;

                            for (p, v) in [
                                (ZSTD_c_compressionLevel, 3),
                                (ZSTD_c_strategy, strat),
                                (ZSTD_c_useRowMatchFinder, rmf),
                            ] {
                                eqv(
                                    &format!("setParameter({p}) {tag}"),
                                    set_c(ctx.c, p, v),
                                    set_r(ctx.r, p, v),
                                );
                            }
                            match dictMode {
                                0 => {}
                                1 => {
                                    // prefix -> ZSTD_extDict block compressors
                                    eqv(
                                        &format!("ZSTD_CCtx_refPrefix {tag}"),
                                        refpx_c(ctx.c, dict.as_ptr() as *const c_void, dict.len()),
                                        refpx_r(ctx.r, dict.as_ptr() as *const c_void, dict.len()),
                                    );
                                }
                                _ => {
                                    // CDict -> ZSTD_dictMatchState / dedicatedDictSearch
                                    let pp = CtxPair::cctx_params();
                                    let ddss = if dictMode == 3 { 1 } else { 0 };
                                    for (p, v) in [
                                        (ZSTD_c_compressionLevel, 3),
                                        (ZSTD_c_strategy, strat),
                                        (ZSTD_c_useRowMatchFinder, rmf),
                                        (ZSTD_c_enableDedicatedDictSearch, ddss),
                                    ] {
                                        eqv(
                                            &format!("CCtxParams_setParameter({p}) {tag}"),
                                            cpset_c(pp.c, p, v),
                                            cpset_r(pp.r, p, v),
                                        );
                                    }
                                    cd_c = mkcd_c(
                                        dict.as_ptr() as *const c_void,
                                        dict.len(),
                                        ZSTD_dlm_byRef,
                                        ZSTD_dct_rawContent,
                                        pp.c,
                                        ZSTD_customMem::default(),
                                    );
                                    cd_r = mkcd_r(
                                        dict.as_ptr() as *const c_void,
                                        dict.len(),
                                        ZSTD_dlm_byRef,
                                        ZSTD_dct_rawContent,
                                        pp.r,
                                        ZSTD_customMem::default(),
                                    );
                                    eqv(
                                        &format!("ZSTD_createCDict_advanced2 non-NULL {tag}"),
                                        cd_c.is_null(),
                                        cd_r.is_null(),
                                    );
                                    assert!(!cd_c.is_null(), "CDict creation failed for {tag}");
                                    eqv(
                                        &format!("ZSTD_CCtx_refCDict {tag}"),
                                        refcd_c(ctx.c, cd_c),
                                        refcd_r(ctx.r, cd_r),
                                    );
                                    if dictMode == 3 {
                                        let p = ZSTD_c_forceAttachDict;
                                        eqv(
                                            &format!("setParameter(forceAttachDict) {tag}"),
                                            set_c(ctx.c, p, 1),
                                            set_r(ctx.r, p, 1),
                                        );
                                    }
                                    params = Some(pp);
                                }
                            }

                            let mut ac = vec![0xA5u8; cap];
                            let mut ar = vec![0xA5u8; cap];
                            let x = c2_c(
                                ctx.c,
                                ac.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                src.len(),
                                0,
                            );
                            let y = c2_r(
                                ctx.r,
                                ar.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                src.len(),
                                0,
                            );
                            eqv(&format!("ZSTD_compress2 ret {tag}"), x, y);
                            assert!(!is_err(x), "ZSTD_compress2 failed for {tag}");
                            eqbuf(&format!("ZSTD_compress2 dst {tag}"), &ac[..x], &ar[..y]);

                            // round-trip (with the matching dictionary/prefix)
                            {
                                let dctx = CtxPair::dctx();
                                let mut oc = vec![0u8; src.len()];
                                let mut or = vec![0u8; src.len()];
                                let (dud_c, dud_r) = duo::<unsafe extern "C" fn(
                                    *mut c_void,
                                    *mut c_void,
                                    usize,
                                    *const c_void,
                                    usize,
                                    *const c_void,
                                    usize,
                                ) -> usize>("ZSTD_decompress_usingDict");
                                let (dref, dsize) = if dictMode == 0 {
                                    (std::ptr::null(), 0usize)
                                } else {
                                    (dict.as_ptr() as *const c_void, dict.len())
                                };
                                let a = dud_c(
                                    dctx.c,
                                    oc.as_mut_ptr() as *mut c_void,
                                    oc.len(),
                                    ac.as_ptr() as *const c_void,
                                    x,
                                    dref,
                                    dsize,
                                );
                                let b = dud_r(
                                    dctx.r,
                                    or.as_mut_ptr() as *mut c_void,
                                    or.len(),
                                    ar.as_ptr() as *const c_void,
                                    y,
                                    dref,
                                    dsize,
                                );
                                eqv(&format!("round-trip ret {tag}"), a, b);
                                assert_eq!(a, src.len(), "round-trip size {tag}");
                                eqbuf(&format!("round-trip content {tag}"), &src, &oc[..a]);
                                eqbuf(&format!("round-trip dst {tag}"), &oc, &or);
                            }

                            if !cd_c.is_null() {
                                eqv(
                                    &format!("ZSTD_freeCDict {tag}"),
                                    frcd_c(cd_c),
                                    frcd_r(cd_r),
                                );
                            }
                            drop(params);
                        }
                    }
                }
            }
        }
    }
}

// ================================================ row 144 (direct match-state)

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_window_t {
    nextSrc: *const u8,
    base: *const u8,
    dictBase: *const u8,
    dictLimit: u32,
    lowLimit: u32,
    nbOverflowCorrections: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_match_t {
    off: u32,
    len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_optimal_t {
    price: c_int,
    off: u32,
    mlen: u32,
    litlen: u32,
    rep: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct optState_t {
    litFreq: *mut c_uint,
    litLengthFreq: *mut c_uint,
    matchLengthFreq: *mut c_uint,
    offCodeFreq: *mut c_uint,
    matchTable: *mut ZSTD_match_t,
    priceTable: *mut ZSTD_optimal_t,
    litSum: u32,
    litLengthSum: u32,
    matchLengthSum: u32,
    offCodeSum: u32,
    litSumBasePrice: u32,
    litLengthSumBasePrice: u32,
    matchLengthSumBasePrice: u32,
    offCodeSumBasePrice: u32,
    priceType: c_uint,
    symbolCosts: *const c_void,
    literalCompressionMode: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_MatchState_t {
    window: ZSTD_window_t,
    loadedDictEnd: u32,
    nextToUpdate: u32,
    hashLog3: u32,
    rowHashLog: u32,
    tagTable: *mut u8,
    hashCache: [u32; 8],
    hashSalt: u64,
    hashSaltEntropy: u32,
    hashTable: *mut u32,
    hashTable3: *mut u32,
    chainTable: *mut u32,
    forceNonContiguous: c_int,
    dedicatedDictSearch: c_int,
    opt: optState_t,
    dictMatchState: *const ZSTD_MatchState_t,
    cParams: ZSTD_compressionParameters,
    ldmSeqStore: *const c_void,
    prefetchCDictTables: c_int,
    lazySkipping: c_int,
}

/// 64-byte aligned zeroed allocation, kept alive by the returned box.
struct Aligned {
    ptr: *mut u8,
    len: usize,
}
impl Aligned {
    fn new(len: usize) -> Aligned {
        let layout = std::alloc::Layout::from_size_align(len, 64).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null());
        Aligned { ptr, len }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
    fn as_u32(&self) -> *mut u32 {
        self.ptr as *mut u32
    }
}
impl Drop for Aligned {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.ptr,
                std::alloc::Layout::from_size_align(self.len, 64).unwrap(),
            )
        }
    }
}

struct MsPair {
    hash: Aligned,
    chain: Aligned,
    hash3: Aligned,
    tag: Aligned,
    ms: ZSTD_MatchState_t,
}

/// Build a `ZSTD_MatchState_t` whose tables are 4x oversized (so that a sizing
/// mistake cannot corrupt memory) over the shared buffer `data`.
unsafe fn ms_new(
    data: &[u8],
    hashLog: u32,
    chainLog: u32,
    searchLog: u32,
    minMatch: u32,
    windowLog: u32,
    strategy: u32,
) -> MsPair {
    let hsz = 4 * (1usize << hashLog) * 4;
    let csz = 4 * (1usize << chainLog) * 4;
    let tsz = 4 * (1usize << hashLog);
    let hash = Aligned::new(hsz);
    let chain = Aligned::new(csz);
    let hash3 = Aligned::new(hsz);
    let tag = Aligned::new(tsz);
    let base = data.as_ptr().wrapping_sub(2);
    let rowLog = searchLog.clamp(4, 6);
    let ms = ZSTD_MatchState_t {
        window: ZSTD_window_t {
            nextSrc: data.as_ptr().wrapping_add(data.len()),
            base,
            dictBase: base,
            dictLimit: 2,
            lowLimit: 2,
            nbOverflowCorrections: 0,
        },
        loadedDictEnd: 0,
        nextToUpdate: 2,
        hashLog3: hashLog,
        rowHashLog: hashLog - rowLog,
        tagTable: tag.ptr,
        hashCache: [0; 8],
        hashSalt: 0,
        hashSaltEntropy: 0,
        hashTable: hash.as_u32(),
        hashTable3: hash3.as_u32(),
        chainTable: chain.as_u32(),
        forceNonContiguous: 0,
        dedicatedDictSearch: 0,
        opt: std::mem::zeroed(),
        dictMatchState: std::ptr::null(),
        cParams: ZSTD_compressionParameters {
            windowLog,
            chainLog,
            hashLog,
            searchLog,
            minMatch,
            targetLength: 0,
            strategy,
        },
        ldmSeqStore: std::ptr::null(),
    prefetchCDictTables: 0,
        lazySkipping: 0,
    };
    MsPair {
        hash,
        chain,
        hash3,
        tag,
        ms,
    }
}

type FnFillHash = unsafe extern "C" fn(*mut ZSTD_MatchState_t, *const c_void, c_int, c_int);
type FnInsertIdx = unsafe extern "C" fn(*mut ZSTD_MatchState_t, *const u8) -> u32;
type FnRowUpdate = unsafe extern "C" fn(*mut ZSTD_MatchState_t, *const u8);
type FnUpdateTree = unsafe extern "C" fn(*mut ZSTD_MatchState_t, *const u8, *const u8);

#[test]
fn row144_matchstate_table_builders() {
    unsafe {
        let (fh_c, fh_r) = duo::<FnFillHash>("ZSTD_fillHashTable");
        let (fd_c, fd_r) = duo::<FnFillHash>("ZSTD_fillDoubleHashTable");
        let (ii_c, ii_r) = duo::<FnInsertIdx>("ZSTD_insertAndFindFirstIndex");
        let (ru_c, ru_r) = duo::<FnRowUpdate>("ZSTD_row_update");
        let (ut_c, ut_r) = duo::<FnUpdateTree>("ZSTD_updateTree");
        let mut nonempty = [0usize; 4];
        let nz = |b: &[u8]| b.iter().any(|&x| x != 0);

        for class in 0..N_CLASSES {
            for &size in &[64usize, 1024, 20_000] {
                let data = gen_class(class, size, 0x144);
                for &hashLog in &[10u32, 12, 14] {
                    for &chainLog in &[10u32, 13] {
                        for &minMatch in &[3u32, 4, 5, 6, 7] {
                            for &searchLog in &[1u32, 4, 5, 6] {
                                let tag = format!(
                                    "class={} size={size} hLog={hashLog} cLog={chainLog} mml={minMatch} sLog={searchLog}",
                                    CLASS_NAMES[class]
                                );
                                let end = data.as_ptr().wrapping_add(data.len()) as *const c_void;

                                // ---- ZSTD_fillHashTable
                                for &dtlm in &[0i32, 1] {
                                    for &tfp in &[0i32, 1] {
                                        let mut a =
                                            ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 1);
                                        let mut b =
                                            ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 1);
                                        fh_c(&mut a.ms, end, dtlm, tfp);
                                        fh_r(&mut b.ms, end, dtlm, tfp);
                                        eqbuf(
                                            &format!("ZSTD_fillHashTable hashTable {tag} dtlm={dtlm} tfp={tfp}"),
                                            a.hash.bytes(),
                                            b.hash.bytes(),
                                        );
                                        eqv(
                                            &format!("ZSTD_fillHashTable nextToUpdate {tag}"),
                                            a.ms.nextToUpdate,
                                            b.ms.nextToUpdate,
                                        );
                                        if nz(a.hash.bytes()) {
                                            nonempty[0] += 1;
                                        }
                                    }
                                }

                                // ---- ZSTD_fillDoubleHashTable
                                for &dtlm in &[0i32, 1] {
                                    for &tfp in &[0i32, 1] {
                                        let mut a =
                                            ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 2);
                                        let mut b =
                                            ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 2);
                                        fd_c(&mut a.ms, end, dtlm, tfp);
                                        fd_r(&mut b.ms, end, dtlm, tfp);
                                        eqbuf(
                                            &format!("ZSTD_fillDoubleHashTable hashTable {tag} dtlm={dtlm} tfp={tfp}"),
                                            a.hash.bytes(),
                                            b.hash.bytes(),
                                        );
                                        eqbuf(
                                            &format!("ZSTD_fillDoubleHashTable chainTable {tag} dtlm={dtlm} tfp={tfp}"),
                                            a.chain.bytes(),
                                            b.chain.bytes(),
                                        );
                                        if nz(a.hash.bytes()) && nz(a.chain.bytes()) {
                                            nonempty[1] += 1;
                                        }
                                    }
                                }

                                if size < 64 {
                                    continue;
                                }
                                // ---- ZSTD_insertAndFindFirstIndex (walked forward)
                                {
                                    let mut a =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 4);
                                    let mut b =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 4);
                                    let mut off = 8usize;
                                    while off + 16 < data.len() {
                                        let ip = data.as_ptr().wrapping_add(off);
                                        eqv(
                                            &format!("ZSTD_insertAndFindFirstIndex({off}) {tag}"),
                                            ii_c(&mut a.ms, ip),
                                            ii_r(&mut b.ms, ip),
                                        );
                                        off += 1 + (off % 37);
                                    }
                                    eqbuf(
                                        &format!("ZSTD_insertAndFindFirstIndex hashTable {tag}"),
                                        a.hash.bytes(),
                                        b.hash.bytes(),
                                    );
                                    eqbuf(
                                        &format!("ZSTD_insertAndFindFirstIndex chainTable {tag}"),
                                        a.chain.bytes(),
                                        b.chain.bytes(),
                                    );
                                    eqv(
                                        &format!("ZSTD_insertAndFindFirstIndex nextToUpdate {tag}"),
                                        a.ms.nextToUpdate,
                                        b.ms.nextToUpdate,
                                    );
                                }

                                // ---- ZSTD_row_update
                                if hashLog >= searchLog.clamp(4, 6) + 4 {
                                    let mut a =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 4);
                                    let mut b =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 4);
                                    let mut off = 8usize;
                                    while off + 16 < data.len() {
                                        let ip = data.as_ptr().wrapping_add(off);
                                        ru_c(&mut a.ms, ip);
                                        ru_r(&mut b.ms, ip);
                                        off += 1 + (off % 53);
                                    }
                                    eqbuf(
                                        &format!("ZSTD_row_update hashTable {tag}"),
                                        a.hash.bytes(),
                                        b.hash.bytes(),
                                    );
                                    eqbuf(
                                        &format!("ZSTD_row_update tagTable {tag}"),
                                        a.tag.bytes(),
                                        b.tag.bytes(),
                                    );
                                    if nz(a.tag.bytes()) {
                                        nonempty[2] += 1;
                                    }
                                    eqbuf(
                                        &format!("ZSTD_row_update hashCache {tag}"),
                                        as_bytes32(&a.ms.hashCache),
                                        as_bytes32(&b.ms.hashCache),
                                    );
                                    eqv(
                                        &format!("ZSTD_row_update nextToUpdate {tag}"),
                                        a.ms.nextToUpdate,
                                        b.ms.nextToUpdate,
                                    );
                                }

                                // ---- ZSTD_updateTree (binary tree match finder)
                                {
                                    let mut a =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 7);
                                    let mut b =
                                        ms_new(&data, hashLog, chainLog, searchLog, minMatch, 17, 7);
                                    let iend = data.as_ptr().wrapping_add(data.len());
                                    let mut off = 8usize;
                                    while off + 32 < data.len() {
                                        let ip = data.as_ptr().wrapping_add(off);
                                        ut_c(&mut a.ms, ip, iend);
                                        ut_r(&mut b.ms, ip, iend);
                                        off += 1 + (off % 97);
                                    }
                                    eqbuf(
                                        &format!("ZSTD_updateTree hashTable {tag}"),
                                        a.hash.bytes(),
                                        b.hash.bytes(),
                                    );
                                    eqbuf(
                                        &format!("ZSTD_updateTree chainTable {tag}"),
                                        a.chain.bytes(),
                                        b.chain.bytes(),
                                    );
                                    if nz(a.chain.bytes()) {
                                        nonempty[3] += 1;
                                    }
                                    eqbuf(
                                        &format!("ZSTD_updateTree hashTable3 {tag}"),
                                        a.hash3.bytes(),
                                        b.hash3.bytes(),
                                    );
                                    eqv(
                                        &format!("ZSTD_updateTree nextToUpdate {tag}"),
                                        a.ms.nextToUpdate,
                                        b.ms.nextToUpdate,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        // the comparisons above must not be vacuous
        assert!(nonempty[0] > 100, "fillHashTable never populated: {nonempty:?}");
        assert!(nonempty[1] > 100, "fillDoubleHashTable never populated: {nonempty:?}");
        assert!(nonempty[2] > 50, "row_update never populated: {nonempty:?}");
        assert!(nonempty[3] > 50, "updateTree never populated: {nonempty:?}");
    }
}
