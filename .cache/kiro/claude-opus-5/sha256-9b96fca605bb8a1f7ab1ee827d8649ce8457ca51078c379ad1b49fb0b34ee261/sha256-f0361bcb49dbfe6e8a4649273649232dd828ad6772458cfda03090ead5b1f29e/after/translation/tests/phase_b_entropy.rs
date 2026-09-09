//! Phase B rows 2–12 + Phase C rows H1–H21:
//! the entropy layer (`HIST_*`, `FSE_*`, `HUF_*`) and the `ZSTD_XXH*` family,
//! driven directly at their LOWEST-LEVEL exported entry points.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ============================================================== XXH (rows 2-4)

const XXH_LENS: [usize; 26] = [
    0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 15, 16, 17, 24, 31, 32, 33, 63, 64, 100, 127, 128, 129, 240,
    241, 4096,
];

#[test]
fn row2_xxh32() {
    let p = libs();
    type FH = unsafe extern "C" fn(*const c_void, Sz, u32) -> u32;
    type FNew = unsafe extern "C" fn() -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FReset = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
    type FUpd = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> c_int;
    type FDig = unsafe extern "C" fn(*const c_void) -> u32;
    type FCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
    let (c_h, r_h) = p.sym::<FH>("ZSTD_XXH32");
    let (c_new, r_new) = p.sym::<FNew>("ZSTD_XXH32_createState");
    let (c_free, r_free) = p.sym::<FFree>("ZSTD_XXH32_freeState");
    let (c_rst, r_rst) = p.sym::<FReset>("ZSTD_XXH32_reset");
    let (c_upd, r_upd) = p.sym::<FUpd>("ZSTD_XXH32_update");
    let (c_dig, r_dig) = p.sym::<FDig>("ZSTD_XXH32_digest");
    let (c_cpy, r_cpy) = p.sym::<FCopy>("ZSTD_XXH32_copyState");

    let mut rng = Rng::new(SEED ^ 0x32_32);
    unsafe {
        for seed in [0u32, 1, 0xDEAD_BEEF, u32::MAX] {
            for &len in XXH_LENS.iter() {
                let src = gen(Shape::Incompressible, len, &mut rng);
                let sp = if len == 0 { std::ptr::null() } else { src.as_ptr() as *const c_void };
                eq(
                    &format!("ZSTD_XXH32(len={len},seed={seed})"),
                    c_h(sp, len, seed),
                    r_h(sp, len, seed),
                );
                // streaming, in 1/7/len chunks
                for chunk in [1usize, 7, 64, len.max(1)] {
                    let cs = c_new();
                    let rs = r_new();
                    assert!(!cs.is_null() && !rs.is_null());
                    eq("XXH32_reset", c_rst(cs, seed), r_rst(rs, seed));
                    let mut off = 0usize;
                    let mut copied = false;
                    let cs2 = c_new();
                    let rs2 = r_new();
                    while off < len {
                        let n = chunk.min(len - off);
                        eq(
                            "XXH32_update",
                            c_upd(cs, src[off..].as_ptr() as *const c_void, n),
                            r_upd(rs, src[off..].as_ptr() as *const c_void, n),
                        );
                        off += n;
                        if !copied {
                            copied = true;
                            c_cpy(cs2, cs);
                            r_cpy(rs2, rs);
                            eq("XXH32 copyState digest", c_dig(cs2), r_dig(rs2));
                        }
                    }
                    eq(
                        &format!("XXH32 streaming digest len={len} chunk={chunk} seed={seed}"),
                        c_dig(cs),
                        r_dig(rs),
                    );
                    eq("XXH32_freeState", c_free(cs), r_free(rs));
                    c_free(cs2);
                    r_free(rs2);
                }
            }
        }
        // canonical helpers
        type FCanon = unsafe extern "C" fn(*mut [u8; 4], u32);
        type FFromCanon = unsafe extern "C" fn(*const [u8; 4]) -> u32;
        let (c_cf, r_cf) = p.sym::<FCanon>("ZSTD_XXH32_canonicalFromHash");
        let (c_hf, r_hf) = p.sym::<FFromCanon>("ZSTD_XXH32_hashFromCanonical");
        for h in [0u32, 1, 0x1234_5678, u32::MAX, 0x8000_0000] {
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            c_cf(&mut cb, h);
            r_cf(&mut rb, h);
            eq_bytes(&format!("XXH32_canonicalFromHash({h})"), &cb, &rb);
            eq(
                &format!("XXH32_hashFromCanonical({h})"),
                c_hf(&cb),
                r_hf(&rb),
            );
        }
    }
}

#[test]
fn row3_xxh64() {
    let p = libs();
    type FH = unsafe extern "C" fn(*const c_void, Sz, u64) -> u64;
    type FNew = unsafe extern "C" fn() -> *mut c_void;
    type FFree = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FReset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
    type FUpd = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> c_int;
    type FDig = unsafe extern "C" fn(*const c_void) -> u64;
    type FCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
    let (c_h, r_h) = p.sym::<FH>("ZSTD_XXH64");
    let (c_new, r_new) = p.sym::<FNew>("ZSTD_XXH64_createState");
    let (c_free, r_free) = p.sym::<FFree>("ZSTD_XXH64_freeState");
    let (c_rst, r_rst) = p.sym::<FReset>("ZSTD_XXH64_reset");
    let (c_upd, r_upd) = p.sym::<FUpd>("ZSTD_XXH64_update");
    let (c_dig, r_dig) = p.sym::<FDig>("ZSTD_XXH64_digest");
    let (c_cpy, r_cpy) = p.sym::<FCopy>("ZSTD_XXH64_copyState");
    let mut rng = Rng::new(SEED ^ 0x64_64);
    unsafe {
        for seed in [0u64, 1, 0xDEAD_BEEF_CAFE_BABE, u64::MAX] {
            for &len in XXH_LENS.iter() {
                let src = gen(Shape::Incompressible, len, &mut rng);
                let sp = if len == 0 { std::ptr::null() } else { src.as_ptr() as *const c_void };
                eq(
                    &format!("ZSTD_XXH64(len={len},seed={seed})"),
                    c_h(sp, len, seed),
                    r_h(sp, len, seed),
                );
                for chunk in [1usize, 7, 64, len.max(1)] {
                    let cs = c_new();
                    let rs = r_new();
                    eq("XXH64_reset", c_rst(cs, seed), r_rst(rs, seed));
                    let cs2 = c_new();
                    let rs2 = r_new();
                    let mut off = 0usize;
                    let mut copied = false;
                    while off < len {
                        let n = chunk.min(len - off);
                        eq(
                            "XXH64_update",
                            c_upd(cs, src[off..].as_ptr() as *const c_void, n),
                            r_upd(rs, src[off..].as_ptr() as *const c_void, n),
                        );
                        off += n;
                        if !copied {
                            copied = true;
                            c_cpy(cs2, cs);
                            r_cpy(rs2, rs);
                            eq("XXH64 copyState digest", c_dig(cs2), r_dig(rs2));
                        }
                    }
                    eq(
                        &format!("XXH64 streaming len={len} chunk={chunk} seed={seed}"),
                        c_dig(cs),
                        r_dig(rs),
                    );
                    c_free(cs);
                    r_free(rs);
                    c_free(cs2);
                    r_free(rs2);
                }
            }
        }
        type FCanon = unsafe extern "C" fn(*mut [u8; 8], u64);
        type FFromCanon = unsafe extern "C" fn(*const [u8; 8]) -> u64;
        let (c_cf, r_cf) = p.sym::<FCanon>("ZSTD_XXH64_canonicalFromHash");
        let (c_hf, r_hf) = p.sym::<FFromCanon>("ZSTD_XXH64_hashFromCanonical");
        for h in [0u64, 1, 0x1234_5678_9ABC_DEF0, u64::MAX] {
            let mut cb = [0u8; 8];
            let mut rb = [0u8; 8];
            c_cf(&mut cb, h);
            r_cf(&mut rb, h);
            eq_bytes(&format!("XXH64_canonicalFromHash({h})"), &cb, &rb);
            eq(&format!("XXH64_hashFromCanonical({h})"), c_hf(&cb), r_hf(&rb));
        }
    }
}

/// Row 4 — the XXH3 / XXH128 families are NOT exported by this build of the C
/// `.so` (`nm -D` shows only `ZSTD_XXH32*`, `ZSTD_XXH64*`, `ZSTD_XXH_versionNumber`;
/// `xxhash.c` compiles XXH3 as `static`). All that remains on this axis is the
/// version accessor, plus an assertion that neither library exports XXH3.
#[test]
fn row4_xxh_version_and_no_xxh3() {
    let p = libs();
    let (c, r) = p.sym::<FnVoidUint>("ZSTD_XXH_versionNumber");
    unsafe { eq("ZSTD_XXH_versionNumber", c(), r()) };
    for absent in [
        "ZSTD_XXH3_64bits",
        "ZSTD_XXH3_128bits",
        "ZSTD_XXH3_createState",
        "ZSTD_XXH128",
    ] {
        assert!(
            !p.has(absent),
            "{absent} unexpectedly present; CONFIGS.md row 4 needs updating"
        );
    }
}

// ================================================== row 5 / H19-H20 — HIST_*

#[test]
fn row5_hist() {
    let p = libs();
    type FH = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, Sz) -> Sz;
    type FHW = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, Sz, *mut c_void, Sz) -> Sz;
    type FHS = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, Sz) -> c_uint;
    let (c_h, r_h) = p.sym::<FH>("HIST_count");
    let (c_hf, r_hf) = p.sym::<FH>("HIST_countFast");
    let (c_hw, r_hw) = p.sym::<FHW>("HIST_count_wksp");
    let (c_hfw, r_hfw) = p.sym::<FHW>("HIST_countFast_wksp");
    let (c_hs, r_hs) = p.sym::<FHS>("HIST_count_simple");
    let (c_ie, _) = p.sym::<FnIsError>("HIST_isError");

    let mut rng = Rng::new(SEED ^ 0x5);
    // HIST workspace: HIST_WKSP_SIZE = 1024*4 (see hist.h)
    let wksp_sizes = [0usize, 8, 1024, 4096, 8192];
    unsafe {
        for shape in ALL_SHAPES {
            for &len in SIZE_AXIS.iter() {
                let src = gen(shape, len, &mut rng);
                let sp = src.as_ptr() as *const c_void;
                for &msv in &[1u32, 15, 63, 127, 255] {
                    let mut cc = vec![0u32; 256];
                    let mut rc = vec![0u32; 256];
                    let mut cm = msv;
                    let mut rm = msv;
                    let tag = format!("HIST shape={shape:?} len={len} msv={msv}");
                    let a = c_h(cc.as_mut_ptr(), &mut cm, sp, len);
                    let b = r_h(rc.as_mut_ptr(), &mut rm, sp, len);
                    eq(&format!("{tag}: HIST_count ret"), a, b);
                    eq(&format!("{tag}: HIST_count maxSymbolValue"), cm, rm);
                    if c_ie(a) == 0 {
                        eq(&format!("{tag}: HIST_count table"), &cc[..], &rc[..]);
                    }

                    // countFast requires maxSymbolValue >= real max; only call
                    // it where HIST_count agreed it is in range.
                    if c_ie(a) == 0 {
                        let mut cc = vec![0u32; 256];
                        let mut rc = vec![0u32; 256];
                        let mut cm = 255u32;
                        let mut rm = 255u32;
                        let a = c_hf(cc.as_mut_ptr(), &mut cm, sp, len);
                        let b = r_hf(rc.as_mut_ptr(), &mut rm, sp, len);
                        eq(&format!("{tag}: HIST_countFast ret"), a, b);
                        eq(&format!("{tag}: HIST_countFast msv"), cm, rm);
                        eq(&format!("{tag}: HIST_countFast table"), &cc[..], &rc[..]);
                    }

                    // simple: the C asserts `*ip <= maxSymbolValue` and then
                    // indexes count[*ip] unconditionally, and underflows
                    // `while (!count[maxSymbolValue]) maxSymbolValue--` when no
                    // symbol fits. Only msv=255 is a legal input.
                    if msv == 255 {
                        let mut cc = vec![0u32; 256];
                        let mut rc = vec![0u32; 256];
                        let mut cm = msv;
                        let mut rm = msv;
                        eq(
                            &format!("{tag}: HIST_count_simple ret"),
                            c_hs(cc.as_mut_ptr(), &mut cm, sp, len),
                            r_hs(rc.as_mut_ptr(), &mut rm, sp, len),
                        );
                        eq(&format!("{tag}: HIST_count_simple msv"), cm, rm);
                        eq(&format!("{tag}: HIST_count_simple table"), &cc[..], &rc[..]);
                    }

                    // wksp variants, incl. undersized workspaces (H20)
                    for &ws in &wksp_sizes {
                        let mut cw = vec![0u8; ws.max(1)];
                        let mut rw = vec![0u8; ws.max(1)];
                        let mut cc = vec![0u32; 256];
                        let mut rc = vec![0u32; 256];
                        let mut cm = msv;
                        let mut rm = msv;
                        let a = c_hw(cc.as_mut_ptr(), &mut cm, sp, len, cw.as_mut_ptr() as *mut c_void, ws);
                        let b = r_hw(rc.as_mut_ptr(), &mut rm, sp, len, rw.as_mut_ptr() as *mut c_void, ws);
                        eq(&format!("{tag}: HIST_count_wksp(ws={ws}) ret"), a, b);
                        eq(&format!("{tag}: HIST_count_wksp(ws={ws}) msv"), cm, rm);
                        if c_ie(a) == 0 {
                            eq(&format!("{tag}: HIST_count_wksp(ws={ws}) table"), &cc[..], &rc[..]);
                        }
                        let mut cc = vec![0u32; 256];
                        let mut rc = vec![0u32; 256];
                        let mut cm = 255u32;
                        let mut rm = 255u32;
                        let a = c_hfw(cc.as_mut_ptr(), &mut cm, sp, len, cw.as_mut_ptr() as *mut c_void, ws);
                        let b = r_hfw(rc.as_mut_ptr(), &mut rm, sp, len, rw.as_mut_ptr() as *mut c_void, ws);
                        eq(&format!("{tag}: HIST_countFast_wksp(ws={ws}) ret"), a, b);
                        eq(&format!("{tag}: HIST_countFast_wksp(ws={ws}) msv"), cm, rm);
                        if c_ie(a) == 0 {
                            eq(&format!("{tag}: HIST_countFast_wksp(ws={ws}) table"), &cc[..], &rc[..]);
                        }
                    }
                }
            }
        }
    }
}

/// Row 5b — `HIST_add`, the lowest-level histogram accumulator (accumulates
/// into a non-reset table, so it is order- and history-dependent).
#[test]
fn row5b_hist_add() {
    let p = libs();
    type FAdd = unsafe extern "C" fn(*mut c_uint, *const c_void, Sz);
    let (c_a, r_a) = p.sym::<FAdd>("HIST_add");
    let mut rng = Rng::new(SEED ^ 0x5B);
    unsafe {
        for shape in ALL_SHAPES {
            let mut cc = vec![0u32; 256];
            let mut rc = vec![0u32; 256];
            for round in 0..6 {
                for &len in &[0usize, 1, 3, 8, 63, 256, 4096, 65_536] {
                    let src = gen(shape, len, &mut rng);
                    let sp = src.as_ptr() as *const c_void;
                    c_a(cc.as_mut_ptr(), sp, len);
                    r_a(rc.as_mut_ptr(), sp, len);
                    eq(
                        &format!("HIST_add shape={shape:?} round={round} len={len}"),
                        &cc[..],
                        &rc[..],
                    );
                }
            }
        }
    }
}

// ============ rows 6-8 / H1-H11 — FSE, restricted to the EXPORTED surface ============
//
// `nm -D` on the C .so exports only: FSE_versionNumber, FSE_compressBound,
// FSE_isError, FSE_getErrorName, FSE_optimalTableLog,
// FSE_optimalTableLog_internal, FSE_normalizeCount, FSE_NCountWriteBound,
// FSE_writeNCount, FSE_readNCount, FSE_readNCount_bmi2, FSE_buildCTable_wksp,
// FSE_buildCTable_rle, FSE_compress_usingCTable, FSE_buildDTable_wksp,
// FSE_decompress_wksp_bmi2. The full pipeline is therefore composed by hand,
// exactly as FSE_compress_wksp does internally.

#[test]
fn row6_row8_fse_pipeline() {
    let p = libs();
    type FOpt = unsafe extern "C" fn(c_uint, Sz, c_uint) -> c_uint;
    type FOptI = unsafe extern "C" fn(c_uint, Sz, c_uint, c_uint) -> c_uint;
    type FNorm = unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, Sz, c_uint, c_uint) -> Sz;
    type FNCB = unsafe extern "C" fn(c_uint, c_uint) -> Sz;
    type FWrite = unsafe extern "C" fn(*mut c_void, Sz, *const i16, c_uint, c_uint) -> Sz;
    type FRead = unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, Sz) -> Sz;
    type FReadB =
        unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, Sz, c_int) -> Sz;
    type FBuildW =
        unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint, *mut c_void, Sz) -> Sz;
    type FRle = unsafe extern "C" fn(*mut c_uint, u8) -> Sz;
    type FCompCT = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, *const c_uint) -> Sz;
    type FDecWB = unsafe extern "C" fn(
        *mut c_void, Sz, *const c_void, Sz, c_uint, *mut c_void, Sz, c_int,
    ) -> Sz;
    type FHist = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, Sz) -> Sz;

    let (c_opt, r_opt) = p.sym::<FOpt>("FSE_optimalTableLog");
    let (c_opti, r_opti) = p.sym::<FOptI>("FSE_optimalTableLog_internal");
    let (c_norm, r_norm) = p.sym::<FNorm>("FSE_normalizeCount");
    let (c_ncb, r_ncb) = p.sym::<FNCB>("FSE_NCountWriteBound");
    let (c_wr, r_wr) = p.sym::<FWrite>("FSE_writeNCount");
    let (c_rd, r_rd) = p.sym::<FRead>("FSE_readNCount");
    let (c_rdb, r_rdb) = p.sym::<FReadB>("FSE_readNCount_bmi2");
    let (c_bcw, r_bcw) = p.sym::<FBuildW>("FSE_buildCTable_wksp");
    let (c_bdw, r_bdw) = p.sym::<FBuildW>("FSE_buildDTable_wksp");
    let (c_crle, r_crle) = p.sym::<FRle>("FSE_buildCTable_rle");
    let (c_cct, r_cct) = p.sym::<FCompCT>("FSE_compress_usingCTable");
    let (c_dwb, r_dwb) = p.sym::<FDecWB>("FSE_decompress_wksp_bmi2");
    let (c_hist, _) = p.sym::<FHist>("HIST_count");
    let (c_ie, _) = p.sym::<FnIsError>("FSE_isError");
    let (c_ver, r_ver) = p.sym::<FnVoidUint>("FSE_versionNumber");
    let (c_cbnd, r_cbnd) = p.sym::<FnCompressBound>("FSE_compressBound");

    let mut rng = Rng::new(SEED ^ 0x6);
    unsafe {
        eq("FSE_versionNumber", c_ver(), r_ver());
        for s in SIZE_AXIS {
            eq(&format!("FSE_compressBound({s})"), c_cbnd(s), r_cbnd(s));
        }
        for &tl in &[0u32, 1, 4, 5, 6, 8, 10, 12, 13, 15, 16, 99] {
            for &len in SIZE_AXIS.iter() {
                for &msv in &[0u32, 1, 15, 63, 255, 256] {
                    eq(
                        &format!("FSE_NCountWriteBound({msv},{tl})"),
                        c_ncb(msv, tl),
                        r_ncb(msv, tl),
                    );
                    // FSE_optimalTableLog* is only defined for srcSize > 1:
                    // the C asserts `srcSize > 1` and then evaluates
                    // ZSTD_highbit32((U32)(srcSize - 1)), which is UB
                    // (__builtin_clz(0)) at srcSize == 1. Out of contract.
                    if len < 2 {
                        continue;
                    }
                    eq(
                        &format!("FSE_optimalTableLog({tl},{len},{msv})"),
                        c_opt(tl, len, msv),
                        r_opt(tl, len, msv),
                    );
                    for minus in [0u32, 1, 2, 3] {
                        eq(
                            &format!("FSE_optimalTableLog_internal({tl},{len},{msv},{minus})"),
                            c_opti(tl, len, msv, minus),
                            r_opti(tl, len, msv, minus),
                        );
                    }
                }
            }
        }

        for shape in ALL_SHAPES {
            for &len in &[2usize, 3, 8, 63, 255, 256, 1024, 4096, 65_536, 131_072] {
                let src = gen(shape, len, &mut rng);
                let sp = src.as_ptr() as *const c_void;
                for &msv0 in &[15u32, 63, 255] {
                    let mut count = vec![0u32; 256];
                    let mut m = msv0;
                    if c_ie(c_hist(count.as_mut_ptr(), &mut m, sp, len)) != 0 {
                        continue;
                    }
                    for &tl in &[3u32, 4, 5, 6, 9, 12, 13, 16] {
                        for &ulpc in &[0u32, 1] {
                            let tag = format!(
                                "FSE shape={shape:?} len={len} msv={m} tl={tl} ulpc={ulpc}"
                            );
                            let mut cn = vec![0i16; 300];
                            let mut rn = vec![0i16; 300];
                            let a = c_norm(cn.as_mut_ptr(), tl, count.as_ptr(), len, m, ulpc);
                            let b = r_norm(rn.as_mut_ptr(), tl, count.as_ptr(), len, m, ulpc);
                            eq(&format!("{tag}: normalizeCount ret"), a, b);
                            eq(&format!("{tag}: normalizeCount table"), &cn[..], &rn[..]);
                            // C: `if (count[s] == total) return 0;` -- a return
                            // of 0 is the RLE SPECIAL CASE, not a tableLog. The
                            // C's own callers take the FSE_buildCTable_rle path
                            // there; feeding tableLog=0 to the table builders is
                            // out of contract (the C evaluates `1 << (tableLog-1)`
                            // and ZSTD_highbit32(0), both UB).
                            if c_ie(a) != 0 || a == 0 {
                                continue;
                            }
                            let realtl = a as u32;

                            let mut hdr_c: Vec<u8> = Vec::new();
                            let mut hdr_r: Vec<u8> = Vec::new();
                            for &bufsz in &[0usize, 1, 2, 8, 512, 1024] {
                                let mut cb = vec![0u8; bufsz.max(1)];
                                let mut rb = vec![0u8; bufsz.max(1)];
                                let x = c_wr(cb.as_mut_ptr() as *mut c_void, bufsz, cn.as_ptr(), m, realtl);
                                let y = r_wr(rb.as_mut_ptr() as *mut c_void, bufsz, rn.as_ptr(), m, realtl);
                                eq(&format!("{tag}: writeNCount({bufsz}) ret"), x, y);
                                if c_ie(x) != 0 {
                                    continue;
                                }
                                eq_bytes(&format!("{tag}: writeNCount out"), &cb[..x], &rb[..y]);
                                if hdr_c.is_empty() {
                                    hdr_c = cb[..x].to_vec();
                                    hdr_r = rb[..y].to_vec();
                                }
                                for cut in [0usize, 1, 2, x / 2, x.saturating_sub(1), x] {
                                    for &rmsv in &[m, 0, 1, 255] {
                                        let mut cn2 = vec![0i16; 300];
                                        let mut rn2 = vec![0i16; 300];
                                        let (mut cm, mut rm) = (rmsv, rmsv);
                                        let (mut ct, mut rt) = (0u32, 0u32);
                                        let u = c_rd(cn2.as_mut_ptr(), &mut cm, &mut ct, cb.as_ptr() as *const c_void, cut);
                                        let v = r_rd(rn2.as_mut_ptr(), &mut rm, &mut rt, rb.as_ptr() as *const c_void, cut);
                                        eq(&format!("{tag}: readNCount(cut={cut},msv={rmsv}) ret"), u, v);
                                        eq(&format!("{tag}: readNCount msv"), cm, rm);
                                        eq(&format!("{tag}: readNCount tableLog"), ct, rt);
                                        if c_ie(u) == 0 {
                                            eq(&format!("{tag}: readNCount table"), &cn2[..], &rn2[..]);
                                        }
                                        for bmi2 in [0, 1] {
                                            let mut cn3 = vec![0i16; 300];
                                            let mut rn3 = vec![0i16; 300];
                                            let (mut cm, mut rm) = (rmsv, rmsv);
                                            let (mut ct, mut rt) = (0u32, 0u32);
                                            let u = c_rdb(cn3.as_mut_ptr(), &mut cm, &mut ct, cb.as_ptr() as *const c_void, cut, bmi2);
                                            let v = r_rdb(rn3.as_mut_ptr(), &mut rm, &mut rt, rb.as_ptr() as *const c_void, cut, bmi2);
                                            eq(&format!("{tag}: readNCount_bmi2({bmi2},cut={cut}) ret"), u, v);
                                            eq(&format!("{tag}: readNCount_bmi2 msv"), cm, rm);
                                            eq(&format!("{tag}: readNCount_bmi2 tl"), ct, rt);
                                            if c_ie(u) == 0 {
                                                eq(&format!("{tag}: readNCount_bmi2 table"), &cn3[..], &rn3[..]);
                                            }
                                        }
                                    }
                                }
                            }

                            let ct_u32 = 1 + (1usize << realtl.saturating_sub(1)) + ((m as usize + 1) * 2);
                            let cws_needed = (((m as usize + 2) + (1usize << realtl)) / 2 + 2) * 4;
                            let mut cct_ok: Vec<u32> = Vec::new();
                            let mut rct_ok: Vec<u32> = Vec::new();
                            // NOTE on the workspace sizes below: the C's own
                            // FSE_BUILD_CTABLE_WORKSPACE_SIZE macro under-reports
                            // the real requirement by 2 bytes whenever
                            // (maxSymbolValue + 2 + tableSize) is odd, because
                            //   4 * ((maxSV+2 + tableSize)/2 + 2)
                            // truncates the /2. The body actually needs
                            //   2*(maxSV1+1) + tableSize (tableSymbol)
                            //                + tableSize + 8 (spread)
                            // so at exactly the macro value the C accepts the call
                            // and then writes up to 2 bytes past it. Every
                            // workspace here is therefore over-ALLOCATED by 512
                            // bytes while the smaller `ws` value is still what is
                            // passed to the function, so the C's own overshoot
                            // cannot corrupt the heap and the accept/reject
                            // boundary is still exercised faithfully.
                            for &ws in &[0usize, 8, cws_needed.saturating_sub(1), cws_needed, cws_needed + 8, 1 << 17] {
                                let mut cw = vec![0u8; ws.max(1) + 512];
                                let mut rw = vec![0u8; ws.max(1) + 512];
                                let mut cct = vec![0u32; ct_u32 + 512];
                                let mut rct = vec![0u32; ct_u32 + 512];
                                let u = c_bcw(cct.as_mut_ptr(), cn.as_ptr(), m, realtl, cw.as_mut_ptr() as *mut c_void, ws);
                                let v = r_bcw(rct.as_mut_ptr(), rn.as_ptr(), m, realtl, rw.as_mut_ptr() as *mut c_void, ws);
                        eq(&format!("{tag}: buildCTable_wksp(ws={ws}) ret"), u, v);
                                if c_ie(u) != 0 {
                                    continue;
                                }
                                // the produced CTable must be byte-identical
                                eq(&format!("{tag}: CTable(ws={ws})"), &cct[..], &rct[..]);
                                // ... and so must every byte the two libraries
                                // wrote into the workspace, including the 2-byte
                                // overshoot region the C uses.
                                eq_bytes(&format!("{tag}: wksp image(ws={ws})"), &cw, &rw);
                                if cct_ok.is_empty() {
                                    cct_ok = cct.clone();
                                    rct_ok = rct.clone();
                                }
                            }
                            if cct_ok.is_empty() {
                                continue;
                            }

                            let cap = c_cbnd(len) + 64;
                            let mut pay_c: Vec<u8> = Vec::new();
                            let mut pay_r: Vec<u8> = Vec::new();
                            for &dsz in &[0usize, 1, 4, cap / 2, cap] {
                                let mut cb = vec![0u8; dsz.max(1)];
                                let mut rb = vec![0u8; dsz.max(1)];
                                let x = c_cct(cb.as_mut_ptr() as *mut c_void, dsz, sp, len, cct_ok.as_ptr());
                                let y = r_cct(rb.as_mut_ptr() as *mut c_void, dsz, sp, len, rct_ok.as_ptr());
                                eq(&format!("{tag}: compress_usingCTable({dsz}) ret"), x, y);
                                if c_ie(x) != 0 || x == 0 {
                                    continue;
                                }
                                eq_bytes(&format!("{tag}: compress_usingCTable out"), &cb[..x], &rb[..y]);
                                if pay_c.is_empty() {
                                    pay_c = cb[..x].to_vec();
                                    pay_r = rb[..y].to_vec();
                                }
                            }

                            let dt_u32 = 1 + (1usize << realtl);
                            let dws_needed = 2 * (m as usize + 1) + (1usize << realtl) + 8;
                            for &ws in &[0usize, 8, dws_needed.saturating_sub(1), dws_needed, 1 << 17] {
                                let mut cw = vec![0u8; ws.max(1) + 512];
                                let mut rw = vec![0u8; ws.max(1) + 512];
                                let mut cdt = vec![0u32; dt_u32 + 512];
                                let mut rdt = vec![0u32; dt_u32 + 512];
                                let u = c_bdw(cdt.as_mut_ptr(), cn.as_ptr(), m, realtl, cw.as_mut_ptr() as *mut c_void, ws);
                                let v = r_bdw(rdt.as_mut_ptr(), rn.as_ptr(), m, realtl, rw.as_mut_ptr() as *mut c_void, ws);
                                eq(&format!("{tag}: buildDTable_wksp(ws={ws}) ret"), u, v);
                                if c_ie(u) == 0 {
                                    eq(&format!("{tag}: DTable(ws={ws})"), &cdt[..], &rdt[..]);
                                    eq_bytes(&format!("{tag}: DTable wksp image(ws={ws})"), &cw, &rw);
                                }
                            }

                            if !hdr_c.is_empty() && !pay_c.is_empty() {
                                let mut blob_c = hdr_c.clone();
                                blob_c.extend_from_slice(&pay_c);
                                let mut blob_r = hdr_r.clone();
                                blob_r.extend_from_slice(&pay_r);
                                eq_bytes(&format!("{tag}: FSE blob"), &blob_c, &blob_r);
                                for &maxlog in &[realtl, realtl.max(5), 12, 15] {
                                    for &ws in &[0usize, 64, 4096, 1 << 17] {
                                        for bmi2 in [0, 1] {
                                            let mut cw = vec![0u8; ws.max(1) + 512];
                                            let mut rw = vec![0u8; ws.max(1) + 512];
                                            let mut cd = vec![0u8; len + 512];
                                            let mut rd = vec![0u8; len + 512];
                                            let x = c_dwb(cd.as_mut_ptr() as *mut c_void, cd.len(), blob_c.as_ptr() as *const c_void, blob_c.len(), maxlog, cw.as_mut_ptr() as *mut c_void, ws, bmi2);
                                            let y = r_dwb(rd.as_mut_ptr() as *mut c_void, rd.len(), blob_r.as_ptr() as *const c_void, blob_r.len(), maxlog, rw.as_mut_ptr() as *mut c_void, ws, bmi2);
                                            eq(&format!("{tag}: FSE_decompress_wksp_bmi2(ml={maxlog},ws={ws},b={bmi2}) ret"), x, y);
                                            if c_ie(x) == 0 {
                                                eq_bytes(&format!("{tag}: FSE decode out"), &cd[..x], &rd[..y]);
                                                if x == len {
                                                    eq_bytes(&format!("{tag}: FSE round trip"), &src[..], &cd[..x]);
                                                }
                                            }
                                        }
                                    }
                                }
                                for cut in [0usize, 1, 2, blob_c.len() / 2, blob_c.len() - 1] {
                                    let mut cw = vec![0u8; 1 << 17];
                                    let mut rw = vec![0u8; 1 << 17];
                                    let mut cd = vec![0u8; len + 64];
                                    let mut rd = vec![0u8; len + 64];
                                    let x = c_dwb(cd.as_mut_ptr() as *mut c_void, cd.len(), blob_c.as_ptr() as *const c_void, cut, 15, cw.as_mut_ptr() as *mut c_void, cw.len(), 0);
                                    let y = r_dwb(rd.as_mut_ptr() as *mut c_void, rd.len(), blob_r.as_ptr() as *const c_void, cut, 15, rw.as_mut_ptr() as *mut c_void, rw.len(), 0);
                                    eq(&format!("{tag}: FSE decode truncated({cut}) ret"), x, y);
                                    if c_ie(x) == 0 {
                                        eq_bytes(&format!("{tag}: FSE trunc out"), &cd[..x], &rd[..y]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for sym in [0u8, 1, 127, 200, 255] {
            let mut ca = vec![0u32; 8192];
            let mut ra = vec![0u32; 8192];
            eq(
                &format!("FSE_buildCTable_rle({sym}) ret"),
                c_crle(ca.as_mut_ptr(), sym),
                r_crle(ra.as_mut_ptr(), sym),
            );
            eq(&format!("FSE_buildCTable_rle({sym}) table"), &ca[..], &ra[..]);
        }
    }
}

// ============ rows 9-12 / H12-H18 — HUF, restricted to the EXPORTED surface ============
//
// `nm -D` on the C .so exports only: HUF_compressBound, HUF_isError,
// HUF_getErrorName, HUF_minTableLog, HUF_cardinality, HUF_optimalTableLog,
// HUF_buildCTable_wksp, HUF_writeCTable_wksp, HUF_readCTable,
// HUF_readCTableHeader, HUF_getNbBitsFromCTable, HUF_estimateCompressedSize,
// HUF_validateCTable, HUF_compress1X_usingCTable, HUF_compress4X_usingCTable,
// HUF_compress1X_repeat, HUF_compress4X_repeat, HUF_readStats,
// HUF_readStats_wksp, HUF_selectDecoder, HUF_readDTableX1_wksp,
// HUF_readDTableX2_wksp, HUF_decompress1X_usingDTable,
// HUF_decompress4X_usingDTable, HUF_decompress1X_DCtx_wksp,
// HUF_decompress1X1_DCtx_wksp, HUF_decompress1X2_DCtx_wksp,
// HUF_decompress4X_hufOnly_wksp.
// The one-shot HUF_compress*/HUF_decompress* wrappers are NOT exported, so the
// pipeline is composed by hand.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct HufCTableHeader {
    table_log: u8,
    max_symbol_value: u8,
    unused: [u8; core::mem::size_of::<usize>() - 2],
}

const HUF_WKSP: usize = (8 << 10) + 512;
/// HUF_DTABLE_SIZE(HUF_TABLELOG_MAX=12) in u32 units, plus slack.
const HUF_DT_U32: usize = 1 + (1 << 12) + 512;

#[test]
fn row9_row12_huf_pipeline() {
    let p = libs();
    type FRepeat = unsafe extern "C" fn(
        *mut c_void, Sz, *const c_void, Sz, c_uint, c_uint, *mut c_void, Sz, *mut Sz, *mut c_int,
        c_int,
    ) -> Sz;
    type FUsingCT =
        unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, *const Sz, c_int) -> Sz;
    type FDecW = unsafe extern "C" fn(
        *mut c_uint, *mut c_void, Sz, *const c_void, Sz, *mut c_void, Sz, c_int,
    ) -> Sz;
    type FDecDT =
        unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, *const c_uint, c_int) -> Sz;
    type FReadDT =
        unsafe extern "C" fn(*mut c_uint, *const c_void, Sz, *mut c_void, Sz, c_int) -> Sz;
    type FSelect = unsafe extern "C" fn(Sz, Sz) -> c_uint;
    type FBuildCT =
        unsafe extern "C" fn(*mut Sz, *const c_uint, c_uint, c_uint, *mut c_void, Sz) -> Sz;
    type FWriteCT =
        unsafe extern "C" fn(*mut c_void, Sz, *const Sz, c_uint, c_uint, *mut c_void, Sz) -> Sz;
    type FReadCT =
        unsafe extern "C" fn(*mut Sz, *mut c_uint, *const c_void, Sz, *mut c_uint) -> Sz;
    type FReadCTH = unsafe extern "C" fn(*const Sz) -> HufCTableHeader;
    type FEst = unsafe extern "C" fn(*const Sz, *const c_uint, c_uint) -> Sz;
    type FValid = unsafe extern "C" fn(*const Sz, *const c_uint, c_uint) -> c_int;
    type FNbBits = unsafe extern "C" fn(*const Sz, u32) -> u32;
    type FOptTL = unsafe extern "C" fn(
        c_uint, Sz, c_uint, *mut c_void, Sz, *mut Sz, *const c_uint, c_int,
    ) -> c_uint;
    type FCard = unsafe extern "C" fn(*const c_uint, c_uint) -> c_uint;
    type FMinTL = unsafe extern "C" fn(c_uint) -> c_uint;
    type FStats = unsafe extern "C" fn(
        *mut u8, Sz, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, Sz,
    ) -> Sz;
    type FStatsW = unsafe extern "C" fn(
        *mut u8, Sz, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, Sz, *mut c_void, Sz,
        c_int,
    ) -> Sz;
    type FHist = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, Sz) -> Sz;

    let (c_c1r, r_c1r) = p.sym::<FRepeat>("HUF_compress1X_repeat");
    let (c_c4r, r_c4r) = p.sym::<FRepeat>("HUF_compress4X_repeat");
    let (c_c1ct, r_c1ct) = p.sym::<FUsingCT>("HUF_compress1X_usingCTable");
    let (c_c4ct, r_c4ct) = p.sym::<FUsingCT>("HUF_compress4X_usingCTable");
    let (c_d1w, r_d1w) = p.sym::<FDecW>("HUF_decompress1X_DCtx_wksp");
    let (c_d1w1, r_d1w1) = p.sym::<FDecW>("HUF_decompress1X1_DCtx_wksp");
    let (c_d1w2, r_d1w2) = p.sym::<FDecW>("HUF_decompress1X2_DCtx_wksp");
    let (c_d4h, r_d4h) = p.sym::<FDecW>("HUF_decompress4X_hufOnly_wksp");
    let (c_d1dt, r_d1dt) = p.sym::<FDecDT>("HUF_decompress1X_usingDTable");
    let (c_d4dt, r_d4dt) = p.sym::<FDecDT>("HUF_decompress4X_usingDTable");
    let (c_rd1, r_rd1) = p.sym::<FReadDT>("HUF_readDTableX1_wksp");
    let (c_rd2, r_rd2) = p.sym::<FReadDT>("HUF_readDTableX2_wksp");
    let (c_sel, r_sel) = p.sym::<FSelect>("HUF_selectDecoder");
    let (c_bct, r_bct) = p.sym::<FBuildCT>("HUF_buildCTable_wksp");
    let (c_wct, r_wct) = p.sym::<FWriteCT>("HUF_writeCTable_wksp");
    let (c_rct, r_rct) = p.sym::<FReadCT>("HUF_readCTable");
    let (c_rcth, r_rcth) = p.sym::<FReadCTH>("HUF_readCTableHeader");
    let (c_est, r_est) = p.sym::<FEst>("HUF_estimateCompressedSize");
    let (c_val, r_val) = p.sym::<FValid>("HUF_validateCTable");
    let (c_nb, r_nb) = p.sym::<FNbBits>("HUF_getNbBitsFromCTable");
    let (c_otl, r_otl) = p.sym::<FOptTL>("HUF_optimalTableLog");
    let (c_card, r_card) = p.sym::<FCard>("HUF_cardinality");
    let (c_mtl, r_mtl) = p.sym::<FMinTL>("HUF_minTableLog");
    let (c_st, r_st) = p.sym::<FStats>("HUF_readStats");
    let (c_stw, r_stw) = p.sym::<FStatsW>("HUF_readStats_wksp");
    let (c_hist, _) = p.sym::<FHist>("HIST_count");
    let (c_ie, _) = p.sym::<FnIsError>("HUF_isError");
    let (c_cbnd, r_cbnd) = p.sym::<FnCompressBound>("HUF_compressBound");

    let mut rng = Rng::new(SEED ^ 0x9);
    unsafe {
        for s in SIZE_AXIS {
            eq(&format!("HUF_compressBound({s})"), c_cbnd(s), r_cbnd(s));
        }
        // symbolCardinality == 0 is out of contract: HUF_minTableLog calls
        // ZSTD_highbit32(0), which the C guards with `assert(val != 0)` and
        // which evaluates `31 - __builtin_clz(0)` (UB). HUF_cardinality never
        // returns 0 for a non-empty input, so real callers never reach it.
        for n in [1u32, 2, 15, 16, 255, 256, 1000, u32::MAX] {
            eq(&format!("HUF_minTableLog({n})"), c_mtl(n), r_mtl(n));
        }

        for shape in ALL_SHAPES {
            for &len in &[
                0usize, 1, 2, 3, 8, 63, 255, 256, 1024, 4096, 65_536, 131_072, 131_073,
            ] {
                let src = gen(shape, len, &mut rng);
                let sp = src.as_ptr() as *const c_void;
                let mut count = vec![0u32; 256];
                let mut m = 255u32;
                if len > 0 && c_ie(c_hist(count.as_mut_ptr(), &mut m, sp, len)) != 0 {
                    continue;
                }
                eq(
                    &format!("HUF_cardinality(shape={shape:?},len={len},msv={m})"),
                    c_card(count.as_ptr(), m),
                    r_card(count.as_ptr(), m),
                );

                // Precondition shared by HUF_buildCTable_wksp and, through the
                // HUF_flags_optimalDepth branch of HUF_optimalTableLog, by
                // HUF_compress*_repeat: the requested huffLog must be at least
                // HUF_minTableLog(HUF_cardinality(count, maxSymbolValue)).
                // Below it HUF_setMaxHeight cannot converge and the C indexes
                // past its node table. Real callers always pass
                // HUF_TABLELOG_DEFAULT (11) or larger, which satisfies it for
                // any cardinality <= 256.
                let card0 = c_card(count.as_ptr(), m);
                let min_tl0 = if card0 == 0 { 0 } else { c_mtl(card0) };
                for &msv in &[m, 15, 63, 255] {
                    for &hl in &[0u32, 4, 5, 8, 11, 12] {
                        if hl != 0 && hl < min_tl0 {
                            continue;
                        }
                        for flags in [0, 1, 2, 3] {
                            let tag = format!(
                                "HUF shape={shape:?} len={len} msv={msv} hl={hl} fl={flags}"
                            );
                            // HUF_optimalTableLog asserts `srcSize > 1`, and its
                            // HUF_flags_optimalDepth path calls
                            // HUF_minTableLog(HUF_cardinality(count, maxSymbolValue)).
                            // With an empty/absent histogram the cardinality is 0,
                            // HUF_minTableLog(0) is UB (ZSTD_highbit32(0)), and the
                            // resulting garbage tableLog makes the C write out of
                            // bounds. Only call it inside its contract.
                            if len > 1 && c_card(count.as_ptr(), msv.min(255)) > 0 {
                                let mut cws = vec![0u8; HUF_WKSP];
                                let mut rws = vec![0u8; HUF_WKSP];
                                let mut cct = vec![0usize; 258];
                                let mut rct = vec![0usize; 258];
                                let hl_eff = if hl == 0 { 11 } else { hl };
                                eq(
                                    &format!("{tag}: optimalTableLog"),
                                    c_otl(hl_eff, len, msv, cws.as_mut_ptr() as *mut c_void, HUF_WKSP, cct.as_mut_ptr(), count.as_ptr(), flags),
                                    r_otl(hl_eff, len, msv, rws.as_mut_ptr() as *mut c_void, HUF_WKSP, rct.as_mut_ptr(), count.as_ptr(), flags),
                                );
                                eq(&format!("{tag}: optimalTableLog CTable side effect"), &cct[..], &rct[..]);
                            }
                            for repeat_in in [0, 1, 2] {
                                for (nm, cf, rf, one_stream) in [
                                    ("compress1X_repeat", &c_c1r, &r_c1r, true),
                                    ("compress4X_repeat", &c_c4r, &r_c4r, false),
                                ] {
                                    let cap = c_cbnd(len).max(1024);
                                    let mut cb = vec![0u8; cap + 512];
                                    let mut rb = vec![0u8; cap + 512];
                                    let mut cct = vec![0usize; 258];
                                    let mut rct = vec![0usize; 258];
                                    let mut cr = repeat_in;
                                    let mut rr = repeat_in;
                                    let mut cws = vec![0u8; HUF_WKSP];
                                    let mut rws = vec![0u8; HUF_WKSP];
                                    let a = cf(
                                        cb.as_mut_ptr() as *mut c_void, cap, sp, len, msv, hl,
                                        cws.as_mut_ptr() as *mut c_void, HUF_WKSP,
                                        cct.as_mut_ptr(), &mut cr, flags,
                                    );
                                    let b = rf(
                                        rb.as_mut_ptr() as *mut c_void, cap, sp, len, msv, hl,
                                        rws.as_mut_ptr() as *mut c_void, HUF_WKSP,
                                        rct.as_mut_ptr(), &mut rr, flags,
                                    );
                                    eq(&format!("{tag}: HUF_{nm}(rep={repeat_in}) ret"), a, b);
                                    eq(&format!("{tag}: HUF_{nm} repeat out"), cr, rr);
                                    eq(&format!("{tag}: HUF_{nm} CTable out"), &cct[..], &rct[..]);
                                    if c_ie(a) != 0 || a == 0 {
                                        continue;
                                    }
                                    eq_bytes(&format!("{tag}: HUF_{nm} out"), &cb[..a], &rb[..b]);
                                    eq(&format!("{tag}: selectDecoder"), c_sel(len, a), r_sel(len, b));

                                    for cut in [0usize, 1, 2, 3, 4, a.min(8), a / 2, a] {
                                        let mut cwt = vec![0u8; 512];
                                        let mut rwt = vec![0u8; 512];
                                        let (mut crk, mut rrk) = ([0u32; 32], [0u32; 32]);
                                        let (mut cns, mut rns) = (0u32, 0u32);
                                        let (mut ctl, mut rtl) = (0u32, 0u32);
                                        let u = c_st(cwt.as_mut_ptr(), 256, crk.as_mut_ptr(), &mut cns, &mut ctl, cb.as_ptr() as *const c_void, cut);
                                        let v = r_st(rwt.as_mut_ptr(), 256, rrk.as_mut_ptr(), &mut rns, &mut rtl, rb.as_ptr() as *const c_void, cut);
                                        eq(&format!("{tag}: readStats(cut={cut}) ret"), u, v);
                                        eq(&format!("{tag}: readStats ranks"), crk, rrk);
                                        eq(&format!("{tag}: readStats nbSym"), cns, rns);
                                        eq(&format!("{tag}: readStats tl"), ctl, rtl);
                                        eq_bytes(&format!("{tag}: readStats weights"), &cwt, &rwt);
                                        for &ws in &[0usize, 64, HUF_WKSP] {
                                            let mut cws2 = vec![0u8; ws.max(1) + 512];
                                            let mut rws2 = vec![0u8; ws.max(1) + 512];
                                            let mut cwt = vec![0u8; 512];
                                            let mut rwt = vec![0u8; 512];
                                            let (mut crk, mut rrk) = ([0u32; 32], [0u32; 32]);
                                            let (mut cns, mut rns) = (0u32, 0u32);
                                            let (mut ctl, mut rtl) = (0u32, 0u32);
                                            let u = c_stw(cwt.as_mut_ptr(), 256, crk.as_mut_ptr(), &mut cns, &mut ctl, cb.as_ptr() as *const c_void, cut, cws2.as_mut_ptr() as *mut c_void, ws, flags);
                                            let v = r_stw(rwt.as_mut_ptr(), 256, rrk.as_mut_ptr(), &mut rns, &mut rtl, rb.as_ptr() as *const c_void, cut, rws2.as_mut_ptr() as *mut c_void, ws, flags);
                                            eq(&format!("{tag}: readStats_wksp(cut={cut},ws={ws}) ret"), u, v);
                                            eq(&format!("{tag}: readStats_wksp ranks"), crk, rrk);
                                            eq(&format!("{tag}: readStats_wksp nbSym"), cns, rns);
                                            eq(&format!("{tag}: readStats_wksp tl"), ctl, rtl);
                                            eq_bytes(&format!("{tag}: readStats_wksp weights"), &cwt, &rwt);
                                        }
                                    }

                                    for (dnm, dcf, drf) in [
                                        ("decompress1X_DCtx_wksp", &c_d1w, &r_d1w),
                                        ("decompress1X1_DCtx_wksp", &c_d1w1, &r_d1w1),
                                        ("decompress1X2_DCtx_wksp", &c_d1w2, &r_d1w2),
                                        ("decompress4X_hufOnly_wksp", &c_d4h, &r_d4h),
                                    ] {
                                        for cut in [a, 0usize, 1, 2, a / 2] {
                                            let mut cdt = vec![0u32; HUF_DT_U32];
                                            let mut rdt = vec![0u32; HUF_DT_U32];
                                            let mut cd = vec![0u8; len + 512];
                                            let mut rd = vec![0u8; len + 512];
                                            let mut cws2 = vec![0u8; HUF_WKSP];
                                            let mut rws2 = vec![0u8; HUF_WKSP];
                                            let x = dcf(cdt.as_mut_ptr(), cd.as_mut_ptr() as *mut c_void, len, cb.as_ptr() as *const c_void, cut, cws2.as_mut_ptr() as *mut c_void, HUF_WKSP, flags);
                                            let y = drf(rdt.as_mut_ptr(), rd.as_mut_ptr() as *mut c_void, len, rb.as_ptr() as *const c_void, cut, rws2.as_mut_ptr() as *mut c_void, HUF_WKSP, flags);
                                            eq(&format!("{tag}: HUF_{dnm}(cut={cut}) ret"), x, y);
                                            if c_ie(x) != 0 { continue; }
                                            eq_bytes(&format!("{tag}: HUF_{dnm}(cut={cut}) out"), &cd[..x], &rd[..y]);
                                            // A return of 1 from HUF_compress*_repeat
                                            // is the RLE special case (a single raw
                                            // byte, no Huffman stream), and
                                            // repeat_in != HUF_repeat_none lets the
                                            // compressor reuse the caller's (here
                                            // zero-initialised) old table. Neither
                                            // is a decodable Huffman stream, so the
                                            // round trip only holds outside those.
                                            if cut == a
                                                && a > 1
                                                && repeat_in == 0
                                                && one_stream
                                                && dnm != "decompress4X_hufOnly_wksp"
                                                && x == len
                                            {
                                                eq_bytes(&format!("{tag}: HUF_{dnm} round trip"), &src[..], &cd[..x]);
                                            }
                                        }
                                    }

                                    for (rnm, rcf, rrf) in [
                                        ("X1", &c_rd1, &r_rd1),
                                        ("X2", &c_rd2, &r_rd2),
                                    ] {
                                        let mut cdt = vec![0u32; HUF_DT_U32];
                                        let mut rdt = vec![0u32; HUF_DT_U32];
                                        let mut cws2 = vec![0u8; HUF_WKSP];
                                        let mut rws2 = vec![0u8; HUF_WKSP];
                                        let u = rcf(cdt.as_mut_ptr(), cb.as_ptr() as *const c_void, a, cws2.as_mut_ptr() as *mut c_void, HUF_WKSP, flags);
                                        let v = rrf(rdt.as_mut_ptr(), rb.as_ptr() as *const c_void, b, rws2.as_mut_ptr() as *mut c_void, HUF_WKSP, flags);
                                        eq(&format!("{tag}: readDTable{rnm} ret"), u, v);
                                        if c_ie(u) != 0 { continue; }
                                        eq(&format!("{tag}: DTable{rnm}"), &cdt[..], &rdt[..]);
                                        if a <= u { continue; }
                                        for (unm, ucf, urf) in [
                                            ("1X", &c_d1dt, &r_d1dt),
                                            ("4X", &c_d4dt, &r_d4dt),
                                        ] {
                                            let mut cd = vec![0u8; len + 512];
                                            let mut rd = vec![0u8; len + 512];
                                            let x = ucf(cd.as_mut_ptr() as *mut c_void, len, cb[u..].as_ptr() as *const c_void, a - u, cdt.as_ptr(), flags);
                                            let y = urf(rd.as_mut_ptr() as *mut c_void, len, rb[v..].as_ptr() as *const c_void, b - v, rdt.as_ptr(), flags);
                                            eq(&format!("{tag}: decompress{unm}_usingDTable({rnm}) ret"), x, y);
                                            if c_ie(x) == 0 {
                                                eq_bytes(&format!("{tag}: decompress{unm}_usingDTable({rnm}) out"), &cd[..x], &rd[..y]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if len == 0 { continue; }
                // HUF_buildCTable_wksp requires maxNbBits >= the minimum table
                // log that can represent the symbol cardinality. Real callers
                // guarantee this because HUF_optimalTableLog starts its search
                // at HUF_minTableLog(HUF_cardinality(...)). Below that bound
                // HUF_setMaxHeight cannot converge and the C walks off its
                // node table, so only sweep inside the contract.
                let card = c_card(count.as_ptr(), m);
                let min_tl = if card == 0 { 5 } else { c_mtl(card) };
                for &mtl in &[5u32, 8, 11, 12] {
                    if mtl < min_tl {
                        continue;
                    }
                    let mut cws = vec![0u8; HUF_WKSP];
                    let mut rws = vec![0u8; HUF_WKSP];
                    let mut cct = vec![0usize; 258];
                    let mut rct = vec![0usize; 258];
                    let a = c_bct(cct.as_mut_ptr(), count.as_ptr(), m, mtl, cws.as_mut_ptr() as *mut c_void, HUF_WKSP);
                    let b = r_bct(rct.as_mut_ptr(), count.as_ptr(), m, mtl, rws.as_mut_ptr() as *mut c_void, HUF_WKSP);
                    let tag = format!("HUFct shape={shape:?} len={len} msv={m} mtl={mtl}");
                    eq(&format!("{tag}: buildCTable_wksp ret"), a, b);
                    if c_ie(a) != 0 { continue; }
                    eq(&format!("{tag}: CTable"), &cct[..], &rct[..]);
                    let realtl = a as u32;
                    eq(&format!("{tag}: readCTableHeader"), c_rcth(cct.as_ptr()), r_rcth(rct.as_ptr()));
                    eq(
                        &format!("{tag}: estimateCompressedSize"),
                        c_est(cct.as_ptr(), count.as_ptr(), m),
                        r_est(rct.as_ptr(), count.as_ptr(), m),
                    );
                    eq(
                        &format!("{tag}: validateCTable"),
                        c_val(cct.as_ptr(), count.as_ptr(), m),
                        r_val(rct.as_ptr(), count.as_ptr(), m),
                    );
                    for sv in [0u32, 1, m, 255] {
                        eq(
                            &format!("{tag}: getNbBitsFromCTable({sv})"),
                            c_nb(cct.as_ptr(), sv),
                            r_nb(rct.as_ptr(), sv),
                        );
                    }
                    for &ws in &[0usize, 8, 1024, HUF_WKSP] {
                        let mut cw = vec![0u8; ws.max(1) + 512];
                        let mut rw = vec![0u8; ws.max(1) + 512];
                        let mut cc2 = vec![0usize; 258];
                        let mut rc2 = vec![0usize; 258];
                        let u = c_bct(cc2.as_mut_ptr(), count.as_ptr(), m, mtl, cw.as_mut_ptr() as *mut c_void, ws);
                        let v = r_bct(rc2.as_mut_ptr(), count.as_ptr(), m, mtl, rw.as_mut_ptr() as *mut c_void, ws);
                        eq(&format!("{tag}: buildCTable_wksp(ws={ws}) ret"), u, v);
                        if c_ie(u) == 0 {
                            eq(&format!("{tag}: buildCTable_wksp(ws={ws}) table"), &cc2[..], &rc2[..]);
                        }
                    }
                    for &dsz in &[0usize, 1, 4, 64, 256, 1024] {
                        let mut cb = vec![0u8; dsz.max(1) + 512];
                        let mut rb = vec![0u8; dsz.max(1) + 512];
                        let x = c_wct(cb.as_mut_ptr() as *mut c_void, dsz, cct.as_ptr(), m, realtl, cws.as_mut_ptr() as *mut c_void, HUF_WKSP);
                        let y = r_wct(rb.as_mut_ptr() as *mut c_void, dsz, rct.as_ptr(), m, realtl, rws.as_mut_ptr() as *mut c_void, HUF_WKSP);
                        eq(&format!("{tag}: writeCTable_wksp({dsz}) ret"), x, y);
                        if c_ie(x) != 0 { continue; }
                        eq_bytes(&format!("{tag}: writeCTable out"), &cb[..x], &rb[..y]);
                        for cut in [0usize, 1, x / 2, x] {
                            for &rmsv in &[255u32, m, 15] {
                                let mut cc2 = vec![0usize; 258];
                                let mut rc2 = vec![0usize; 258];
                                let (mut cm, mut rm) = (rmsv, rmsv);
                                let (mut cz, mut rz) = (0u32, 0u32);
                                let u = c_rct(cc2.as_mut_ptr(), &mut cm, cb.as_ptr() as *const c_void, cut, &mut cz);
                                let v = r_rct(rc2.as_mut_ptr(), &mut rm, rb.as_ptr() as *const c_void, cut, &mut rz);
                                eq(&format!("{tag}: readCTable(cut={cut},msv={rmsv}) ret"), u, v);
                                eq(&format!("{tag}: readCTable msv"), cm, rm);
                                eq(&format!("{tag}: readCTable hasZeroWeights"), cz, rz);
                                if c_ie(u) == 0 {
                                    eq(&format!("{tag}: readCTable table"), &cc2[..], &rc2[..]);
                                    eq(&format!("{tag}: readCTable header"), c_rcth(cc2.as_ptr()), r_rcth(rc2.as_ptr()));
                                }
                            }
                        }
                    }
                    for (nm, cf, rf) in [
                        ("compress1X_usingCTable", &c_c1ct, &r_c1ct),
                        ("compress4X_usingCTable", &c_c4ct, &r_c4ct),
                    ] {
                        for flags in [0, 1, 2, 3] {
                            let cap = c_cbnd(len).max(1024);
                            for &dsz in &[0usize, 1, 8, cap / 2, cap] {
                                let mut cb = vec![0u8; dsz.max(1) + 512];
                                let mut rb = vec![0u8; dsz.max(1) + 512];
                                let x = cf(cb.as_mut_ptr() as *mut c_void, dsz, sp, len, cct.as_ptr(), flags);
                                let y = rf(rb.as_mut_ptr() as *mut c_void, dsz, sp, len, rct.as_ptr(), flags);
                                eq(&format!("{tag}: HUF_{nm}(fl={flags},dsz={dsz}) ret"), x, y);
                                if c_ie(x) == 0 && x > 0 {
                                    eq_bytes(&format!("{tag}: HUF_{nm} out"), &cb[..x], &rb[..y]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
