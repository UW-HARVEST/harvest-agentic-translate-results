//! Phase B — valid-path differential tests, part 1:
//! `CONFIGS.md` rows 13, 14, 15, 16, 17, 19, 23, 24, 35, 45, 47, 53, 54, 72.
//!
//! Every call goes through `libloading` into BOTH `.so` files.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ------------------------------------------------------------------- helpers

struct Api<'a> {
    p: &'a Pair,
}

fn api() -> Api<'static> {
    Api { p: libs() }
}

/// Compress with the simple one-shot API in both libs; assert byte-identical.
fn diff_simple_compress(p: &Pair, src: &[u8], level: c_int, tag: &str) -> Vec<u8> {
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_co, r_co) = p.sym::<FnCompress>("ZSTD_compress");
    unsafe {
        let cap = c_cb(src.len()).max(64);
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cn = c_co(
            cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        );
        let rn = r_co(
            rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        );
        eq(&format!("{tag}: ZSTD_compress return"), cn, rn);
        let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
        if c_ie(cn) != 0 {
            return Vec::new();
        }
        eq_bytes(&format!("{tag}: compressed frame"), &cbuf[..cn], &rbuf[..rn]);
        cbuf.truncate(cn);
        cbuf
    }
}

/// Decompress a frame in both libs; assert identical return and identical bytes.
fn diff_simple_decompress(p: &Pair, frame: &[u8], expect: &[u8], tag: &str) {
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    unsafe {
        let cap = expect.len() + 64;
        let mut cbuf = vec![0xAAu8; cap];
        let mut rbuf = vec![0xAAu8; cap];
        let cn = c_de(
            cbuf.as_mut_ptr() as *mut c_void,
            cap,
            frame.as_ptr() as *const c_void,
            frame.len(),
        );
        let rn = r_de(
            rbuf.as_mut_ptr() as *mut c_void,
            cap,
            frame.as_ptr() as *const c_void,
            frame.len(),
        );
        eq(&format!("{tag}: ZSTD_decompress return"), cn, rn);
        if c_ie(cn) != 0 {
            return;
        }
        eq_bytes(&format!("{tag}: decompressed bytes"), &cbuf[..cn], &rbuf[..rn]);
        eq_bytes(&format!("{tag}: round trip vs original"), expect, &cbuf[..cn]);
    }
}

/// Full `ZSTD_compress2` run with a set of `(param, value)` pairs applied to a
/// fresh CCtx in both libs. Returns the (identical) frame.
fn diff_compress2(p: &Pair, src: &[u8], opts: &[(c_int, c_int)], tag: &str) -> Option<Vec<u8>> {
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type F2 = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");

    unsafe {
        let cc = c_new();
        let rc = r_new();
        assert!(!cc.is_null() && !rc.is_null(), "{tag}: createCCtx");
        let mut skip = false;
        for (prm, val) in opts {
            let a = c_set(cc, *prm, *val);
            let b = r_set(rc, *prm, *val);
            eq(&format!("{tag}: setParameter({prm},{val})"), a, b);
            if c_ie(a) != 0 {
                skip = true;
            }
        }
        if skip {
            c_free(cc);
            r_free(rc);
            return None;
        }
        let cap = c_cb(src.len()) + 1024;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cn = c_c2(
            cc,
            cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        let rn = r_c2(
            rc,
            rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        eq(&format!("{tag}: ZSTD_compress2 return"), cn, rn);
        c_free(cc);
        r_free(rc);
        if c_ie(cn) != 0 {
            return None;
        }
        eq_bytes(&format!("{tag}: compress2 frame"), &cbuf[..cn], &rbuf[..rn]);
        cbuf.truncate(cn);
        Some(cbuf)
    }
}

fn c_level_range(p: &Pair) -> (c_int, c_int) {
    let (c_min, r_min) = p.sym::<FnVoidInt>("ZSTD_minCLevel");
    let (c_max, r_max) = p.sym::<FnVoidInt>("ZSTD_maxCLevel");
    unsafe {
        eq("ZSTD_minCLevel", c_min(), r_min());
        eq("ZSTD_maxCLevel", c_max(), r_max());
        (c_min(), c_max())
    }
}

// ====================================================== row 13/14 — simple API

#[test]
fn row13_simple_compress_decompress_all_levels_shapes_sizes() {
    let a = api();
    let p = a.p;
    let (lo, hi) = c_level_range(p);
    // Full level axis: every level from min..=max is a distinct clevels.h row,
    // but min is -131072; sample the negative range logarithmically plus all of
    // the dense 1..=22 range and the boundaries.
    let mut levels: Vec<c_int> = Vec::new();
    levels.push(lo);
    levels.push(lo + 1);
    for k in [-131_072, -100_000, -10_000, -1000, -100, -50, -20, -10, -5, -3, -2, -1] {
        if k >= lo {
            levels.push(k);
        }
    }
    for k in 0..=hi {
        levels.push(k);
    }
    levels.sort_unstable();
    levels.dedup();

    let mut rng = Rng::new(SEED);
    for &level in &levels {
        for shape in ALL_SHAPES {
            for &len in SIZE_AXIS_SMALL.iter() {
                let src = gen(shape, len, &mut rng);
                let tag = format!("row13 lvl={level} shape={shape:?} len={len}");
                let frame = diff_simple_compress(p, &src, level, &tag);
                if !frame.is_empty() {
                    diff_simple_decompress(p, &frame, &src, &tag);
                }
            }
        }
    }
}

#[test]
fn row13b_randomized_payloads_per_level() {
    let a = api();
    let p = a.p;
    let (lo, hi) = c_level_range(p);
    let mut rng = Rng::new(SEED ^ 0xB13);
    // Many randomized inputs per level, random sizes and shapes.
    for level in [lo, -7, -1, 0, 1, 3, 6, 9, 12, 15, 17, 19, hi] {
        for _ in 0..24 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len() as u32) as usize];
            let len = rng.below(200_000) as usize;
            let src = gen(shape, len, &mut rng);
            let tag = format!("row13b lvl={level} shape={shape:?} len={len}");
            let frame = diff_simple_compress(p, &src, level, &tag);
            if !frame.is_empty() {
                diff_simple_decompress(p, &frame, &src, &tag);
            }
        }
    }
}

/// Row 14 — reuse the SAME context across many different inputs without reset,
/// which is a different code path (`ZSTD_resetCCtx_internal` continuation).
#[test]
fn row14_context_reuse_no_reset() {
    let a = api();
    let p = a.p;
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FC = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz, c_int) -> Sz;
    type FD = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    let (c_cc, r_cc) = p.sym::<FC>("ZSTD_compressCCtx");
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");

    let mut rng = Rng::new(SEED ^ 0x14);
    unsafe {
        let cc = c_new();
        let rc = r_new();
        let cd = c_dnew();
        let rd = r_dnew();
        for level in [1, 3, 6, 12, 19] {
            for shape in ALL_SHAPES {
                for _ in 0..4 {
                    let len = rng.below(70_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let cap = c_cb(len) + 64;
                    let mut cf = vec![0u8; cap];
                    let mut rf = vec![0u8; cap];
                    let cn = c_cc(
                        cc,
                        cf.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                        level,
                    );
                    let rn = r_cc(
                        rc,
                        rf.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                        level,
                    );
                    let tag = format!("row14 lvl={level} shape={shape:?} len={len}");
                    eq(&format!("{tag}: compressCCtx ret"), cn, rn);
                    if c_ie(cn) != 0 {
                        continue;
                    }
                    eq_bytes(&format!("{tag}: frame"), &cf[..cn], &rf[..rn]);
                    let dcap = len + 64;
                    let mut cdb = vec![0u8; dcap];
                    let mut rdb = vec![0u8; dcap];
                    let cdn = c_dd(
                        cd,
                        cdb.as_mut_ptr() as *mut c_void,
                        dcap,
                        cf.as_ptr() as *const c_void,
                        cn,
                    );
                    let rdn = r_dd(
                        rd,
                        rdb.as_mut_ptr() as *mut c_void,
                        dcap,
                        rf.as_ptr() as *const c_void,
                        rn,
                    );
                    eq(&format!("{tag}: decompressDCtx ret"), cdn, rdn);
                    if c_ie(cdn) == 0 {
                        eq_bytes(&format!("{tag}: decoded"), &cdb[..cdn], &rdb[..rdn]);
                        eq_bytes(&format!("{tag}: round trip"), &src, &cdb[..cdn]);
                    }
                }
            }
        }
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

// ============================================ row 15/16/17 — strategy / mls / row

const P_LEVEL: c_int = 100;
const P_WINDOWLOG: c_int = 101;
const P_HASHLOG: c_int = 102;
const P_CHAINLOG: c_int = 103;
const P_SEARCHLOG: c_int = 104;
const P_MINMATCH: c_int = 105;
const P_TARGETLENGTH: c_int = 106;
const P_STRATEGY: c_int = 107;
const P_TARGETCBLOCKSIZE: c_int = 130;
const P_LDM: c_int = 160;
const P_LDMHASHLOG: c_int = 161;
const P_LDMMINMATCH: c_int = 162;
const P_LDMBUCKETSIZELOG: c_int = 163;
const P_LDMHASHRATELOG: c_int = 164;
const P_CONTENTSIZEFLAG: c_int = 200;
const P_CHECKSUMFLAG: c_int = 201;
const P_DICTIDFLAG: c_int = 202;
const P_FORMAT: c_int = 10;
const P_FORCEMAXWINDOW: c_int = 1000;
const P_LITCOMPMODE: c_int = 1002;
const P_SRCSIZEHINT: c_int = 1004;
const P_SPLITAFTERSEQ: c_int = 1010;
const P_ROWMATCHFINDER: c_int = 1011;
const P_DETERMINISTICREFPREFIX: c_int = 1012;
const P_MAXBLOCKSIZE: c_int = 1015;
const P_BLOCKSPLITTERLEVEL: c_int = 1017;

const DP_WINDOWLOGMAX: c_int = 100;
const DP_FORMAT: c_int = 1000;
const DP_IGNORECHECKSUM: c_int = 1002;
const DP_NOHUFASM: c_int = 1004;
const DP_MAXBLOCKSIZE: c_int = 1005;

#[test]
fn row15_all_strategies() {
    let a = api();
    let p = a.p;
    let mut rng = Rng::new(SEED ^ 0x15);
    for strat in STRATEGIES {
        for shape in ALL_SHAPES {
            for _ in 0..6 {
                let len = rng.below(180_000) as usize;
                let src = gen(shape, len, &mut rng);
                let tag = format!("row15 strat={strat} shape={shape:?} len={len}");
                if let Some(f) = diff_compress2(p, &src, &[(P_STRATEGY, strat)], &tag) {
                    diff_simple_decompress(p, &f, &src, &tag);
                }
            }
        }
    }
}

/// Row 15b — each strategy driven at the bounds of its search parameters.
#[test]
fn row15b_strategy_x_search_params() {
    let a = api();
    let p = a.p;
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");
    let mut rng = Rng::new(SEED ^ 0x15B);
    unsafe {
        let wl = c_gb(P_WINDOWLOG);
        let hl = c_gb(P_HASHLOG);
        let cl = c_gb(P_CHAINLOG);
        let sl = c_gb(P_SEARCHLOG);
        let tl = c_gb(P_TARGETLENGTH);
        for strat in STRATEGIES {
            for &(w, h, ch, s, t) in &[
                (wl.lower, hl.lower, cl.lower, sl.lower, tl.lower),
                (17, 17, 17, 4, 64),
                (20, 20, 21, 6, 999),
                (wl.lower + 2, hl.lower + 1, cl.lower + 1, sl.lower + 1, 0),
                (23, 22, 23, 8, tl.upper),
            ] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible, Shape::MixedEntropy] {
                    let len = 1 + rng.below(150_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!(
                        "row15b strat={strat} wl={w} hl={h} cl={ch} sl={s} tl={t} shape={shape:?} len={len}"
                    );
                    if let Some(f) = diff_compress2(
                        p,
                        &src,
                        &[
                            (P_STRATEGY, strat),
                            (P_WINDOWLOG, w),
                            (P_HASHLOG, h),
                            (P_CHAINLOG, ch),
                            (P_SEARCHLOG, s),
                            (P_TARGETLENGTH, t),
                        ],
                        &tag,
                    ) {
                        diff_simple_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

#[test]
fn row16_min_match_all_mls_templates() {
    let a = api();
    let p = a.p;
    let mut rng = Rng::new(SEED ^ 0x16);
    for strat in STRATEGIES {
        for mm in 3..=7 {
            for shape in [Shape::TextLike, Shape::Repetitive, Shape::TwoSymbol, Shape::MixedEntropy] {
                for _ in 0..3 {
                    let len = 1 + rng.below(140_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row16 strat={strat} mm={mm} shape={shape:?} len={len}");
                    if let Some(f) =
                        diff_compress2(p, &src, &[(P_STRATEGY, strat), (P_MINMATCH, mm)], &tag)
                    {
                        diff_simple_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

#[test]
fn row17_row_match_finder() {
    let a = api();
    let p = a.p;
    let mut rng = Rng::new(SEED ^ 0x17);
    for rmf in [0, 1, 2] {
        for strat in STRATEGIES {
            for mm in [3, 4, 5, 6, 7] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible] {
                    let len = 1 + rng.below(160_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag =
                        format!("row17 rmf={rmf} strat={strat} mm={mm} shape={shape:?} len={len}");
                    if let Some(f) = diff_compress2(
                        p,
                        &src,
                        &[
                            (P_ROWMATCHFINDER, rmf),
                            (P_STRATEGY, strat),
                            (P_MINMATCH, mm),
                        ],
                        &tag,
                    ) {
                        diff_simple_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

// ================================================ row 19/23/24 — literals, flags

#[test]
fn row19_literal_compression_mode() {
    let a = api();
    let p = a.p;
    let mut rng = Rng::new(SEED ^ 0x19);
    for lcm in [0, 1, 2] {
        for shape in ALL_SHAPES {
            for _ in 0..4 {
                let len = rng.below(140_000) as usize;
                let src = gen(shape, len, &mut rng);
                let tag = format!("row19 lcm={lcm} shape={shape:?} len={len}");
                if let Some(f) = diff_compress2(p, &src, &[(P_LITCOMPMODE, lcm)], &tag) {
                    diff_simple_decompress(p, &f, &src, &tag);
                }
            }
        }
    }
}

#[test]
fn row23_frame_header_flags() {
    let a = api();
    let p = a.p;
    let mut rng = Rng::new(SEED ^ 0x23);
    for cs in [0, 1] {
        for ck in [0, 1] {
            for di in [0, 1] {
                for shape in ALL_SHAPES {
                    for _ in 0..3 {
                        let len = rng.below(140_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag = format!(
                            "row23 cs={cs} ck={ck} di={di} shape={shape:?} len={len}"
                        );
                        if let Some(f) = diff_compress2(
                            p,
                            &src,
                            &[
                                (P_CONTENTSIZEFLAG, cs),
                                (P_CHECKSUMFLAG, ck),
                                (P_DICTIDFLAG, di),
                            ],
                            &tag,
                        ) {
                            diff_simple_decompress(p, &f, &src, &tag);
                            diff_frame_introspection(p, &f, &tag);
                        }
                    }
                }
            }
        }
    }
}

/// Row 45 — every frame-introspection entry point on a given frame.
fn diff_frame_introspection(p: &Pair, frame: &[u8], tag: &str) {
    type FU64 = unsafe extern "C" fn(*const c_void, Sz) -> u64;
    type FSZ = unsafe extern "C" fn(*const c_void, Sz) -> Sz;
    type FU = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FGH = unsafe extern "C" fn(*mut FrameHeader, *const c_void, Sz) -> Sz;
    type FGHA = unsafe extern "C" fn(*mut FrameHeader, *const c_void, Sz, c_int) -> Sz;

    unsafe {
        for name in [
            "ZSTD_getFrameContentSize",
            "ZSTD_getDecompressedSize",
            "ZSTD_findDecompressedSize",
            "ZSTD_decompressBound",
        ] {
            let (c, r) = p.sym::<FU64>(name);
            for cut in [
                0usize,
                1,
                2,
                3,
                4,
                5,
                6,
                8,
                9,
                13,
                17,
                18,
                frame.len() / 2,
                frame.len().saturating_sub(1),
                frame.len(),
            ] {
                let cut = cut.min(frame.len());
                eq(
                    &format!("{tag}: {name}(len={cut})"),
                    c(frame.as_ptr() as *const c_void, cut),
                    r(frame.as_ptr() as *const c_void, cut),
                );
            }
        }
        for name in ["ZSTD_findFrameCompressedSize", "ZSTD_frameHeaderSize"] {
            let (c, r) = p.sym::<FSZ>(name);
            for cut in [0usize, 1, 4, 5, 6, 9, 18, frame.len()] {
                let cut = cut.min(frame.len());
                eq(
                    &format!("{tag}: {name}(len={cut})"),
                    c(frame.as_ptr() as *const c_void, cut),
                    r(frame.as_ptr() as *const c_void, cut),
                );
            }
        }
        {
            let (c, r) = p.sym::<FU>("ZSTD_isFrame");
            for cut in [0usize, 1, 3, 4, 5, frame.len()] {
                let cut = cut.min(frame.len());
                eq(
                    &format!("{tag}: ZSTD_isFrame(len={cut})"),
                    c(frame.as_ptr() as *const c_void, cut),
                    r(frame.as_ptr() as *const c_void, cut),
                );
            }
            let (c, r) = p.sym::<FU>("ZSTD_isSkippableFrame");
            for cut in [0usize, 4, frame.len()] {
                let cut = cut.min(frame.len());
                eq(
                    &format!("{tag}: ZSTD_isSkippableFrame(len={cut})"),
                    c(frame.as_ptr() as *const c_void, cut),
                    r(frame.as_ptr() as *const c_void, cut),
                );
            }
            let (c, r) = p.sym::<FU>("ZSTD_getDictID_fromFrame");
            eq(
                &format!("{tag}: ZSTD_getDictID_fromFrame"),
                c(frame.as_ptr() as *const c_void, frame.len()),
                r(frame.as_ptr() as *const c_void, frame.len()),
            );
        }
        {
            let (c, r) = p.sym::<FGH>("ZSTD_getFrameHeader");
            for cut in 0..=frame.len().min(20) {
                let mut ch = FrameHeader::default();
                let mut rh = FrameHeader::default();
                let cn = c(&mut ch, frame.as_ptr() as *const c_void, cut);
                let rn = r(&mut rh, frame.as_ptr() as *const c_void, cut);
                eq(&format!("{tag}: getFrameHeader(len={cut}) ret"), cn, rn);
                if cn == 0 {
                    eq(&format!("{tag}: getFrameHeader(len={cut}) hdr"), ch, rh);
                }
            }
            let (c, r) = p.sym::<FGHA>("ZSTD_getFrameHeader_advanced");
            for fmt in [0, 1, -1, 2, 99, c_int::MAX] {
                let mut ch = FrameHeader::default();
                let mut rh = FrameHeader::default();
                let cn = c(&mut ch, frame.as_ptr() as *const c_void, frame.len(), fmt);
                let rn = r(&mut rh, frame.as_ptr() as *const c_void, frame.len(), fmt);
                eq(
                    &format!("{tag}: getFrameHeader_advanced(fmt={fmt}) ret"),
                    cn,
                    rn,
                );
                if cn == 0 {
                    eq(
                        &format!("{tag}: getFrameHeader_advanced(fmt={fmt}) hdr"),
                        ch,
                        rh,
                    );
                }
            }
        }
    }
}

#[test]
fn row24_magicless_format_round_trip() {
    let a = api();
    let p = a.p;
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FD = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");

    let mut rng = Rng::new(SEED ^ 0x24);
    unsafe {
        for fmt in [0, 1] {
            for shape in ALL_SHAPES {
                for _ in 0..3 {
                    let len = rng.below(120_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row24 fmt={fmt} shape={shape:?} len={len}");
                    let Some(f) = diff_compress2(p, &src, &[(P_FORMAT, fmt)], &tag) else {
                        continue;
                    };
                    // decode with matching decoder format
                    let cd = c_dnew();
                    let rd = r_dnew();
                    eq(
                        &format!("{tag}: DCtx_setParameter(format)"),
                        c_dset(cd, DP_FORMAT, fmt),
                        r_dset(rd, DP_FORMAT, fmt),
                    );
                    let cap = len + 64;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let cn = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                    let rn = r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                    eq(&format!("{tag}: magicless decode ret"), cn, rn);
                    if c_ie(cn) == 0 {
                        eq_bytes(&format!("{tag}: magicless decode"), &cb[..cn], &rb[..rn]);
                        eq_bytes(&format!("{tag}: magicless round trip"), &src, &cb[..cn]);
                    }
                    c_dfree(cd);
                    r_dfree(rd);
                }
            }
        }
    }
}

// ==================================================== row 35 — parameter derivation

#[test]
fn row35_get_params_get_cparams_adjust_check() {
    let a = api();
    let p = a.p;
    type FGP = unsafe extern "C" fn(c_int, u64, Sz) -> Params;
    type FGC = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;
    type FADJ = unsafe extern "C" fn(CParams, u64, Sz) -> CParams;
    type FCHK = unsafe extern "C" fn(CParams) -> Sz;
    let (c_gp, r_gp) = p.sym::<FGP>("ZSTD_getParams");
    let (c_gc, r_gc) = p.sym::<FGC>("ZSTD_getCParams");
    let (c_adj, r_adj) = p.sym::<FADJ>("ZSTD_adjustCParams");
    let (c_chk, r_chk) = p.sym::<FCHK>("ZSTD_checkCParams");
    let (lo, hi) = c_level_range(p);

    let src_sizes: [u64; 12] = [
        0,
        1,
        512,
        1024,
        65_536,
        1 << 20,
        1 << 24,
        1 << 30,
        1u64 << 32,
        u64::MAX,
        u64::MAX - 1,
        123_456_789,
    ];
    let dict_sizes: [usize; 6] = [0, 1, 1024, 4096, 1 << 20, 1 << 24];

    let mut levels: Vec<c_int> = (lo..=lo + 2).collect();
    levels.extend([-10_000, -1000, -100, -20, -5, -1]);
    levels.extend(0..=hi);
    levels.extend([hi + 1, hi + 100]);
    levels.sort_unstable();
    levels.dedup();

    unsafe {
        for &lvl in &levels {
            for &ss in &src_sizes {
                for &ds in &dict_sizes {
                    eq(
                        &format!("ZSTD_getParams({lvl},{ss},{ds})"),
                        c_gp(lvl, ss, ds),
                        r_gp(lvl, ss, ds),
                    );
                    let cp = c_gc(lvl, ss, ds);
                    let rp = r_gc(lvl, ss, ds);
                    eq(&format!("ZSTD_getCParams({lvl},{ss},{ds})"), cp, rp);
                    eq(
                        &format!("ZSTD_adjustCParams({lvl},{ss},{ds})"),
                        c_adj(cp, ss, ds),
                        r_adj(cp, ss, ds),
                    );
                    eq(
                        &format!("ZSTD_checkCParams({lvl},{ss},{ds})"),
                        c_chk(cp),
                        r_chk(cp),
                    );
                }
            }
        }
        // checkCParams on deliberately out-of-range structs
        for bad in [
            CParams { window_log: 0, chain_log: 0, hash_log: 0, search_log: 0, min_match: 0, target_length: 0, strategy: 0 },
            CParams { window_log: 99, chain_log: 99, hash_log: 99, search_log: 99, min_match: 99, target_length: 99, strategy: 99 },
            CParams { window_log: 10, chain_log: 6, hash_log: 6, search_log: 1, min_match: 3, target_length: 0, strategy: 1 },
            CParams { window_log: 31, chain_log: 30, hash_log: 30, search_log: 30, min_match: 7, target_length: 131_072, strategy: 9 },
            CParams { window_log: u32::MAX, chain_log: u32::MAX, hash_log: u32::MAX, search_log: u32::MAX, min_match: u32::MAX, target_length: u32::MAX, strategy: u32::MAX },
        ] {
            eq(
                &format!("ZSTD_checkCParams({bad:?})"),
                c_chk(bad),
                r_chk(bad),
            );
            eq(
                &format!("ZSTD_adjustCParams({bad:?})"),
                c_adj(bad, 1 << 20, 0),
                r_adj(bad, 1 << 20, 0),
            );
        }
    }
}

// ==================================================== row 47 — size estimators

#[test]
fn row47_size_estimators() {
    let a = api();
    let p = a.p;
    let (lo, hi) = c_level_range(p);
    type FI = unsafe extern "C" fn(c_int) -> Sz;
    type FCP = unsafe extern "C" fn(CParams) -> Sz;
    type FDS = unsafe extern "C" fn(Sz) -> Sz;
    type FGC = unsafe extern "C" fn(c_int, u64, Sz) -> CParams;
    let (c_gc, _) = p.sym::<FGC>("ZSTD_getCParams");

    unsafe {
        for name in ["ZSTD_estimateCCtxSize", "ZSTD_estimateCStreamSize"] {
            let (c, r) = p.sym::<FI>(name);
            for lvl in (lo..=lo + 1).chain(-1000..=-995).chain(0..=hi).chain(hi + 1..=hi + 2) {
                eq(&format!("{name}({lvl})"), c(lvl), r(lvl));
            }
        }
        for name in [
            "ZSTD_estimateCCtxSize_usingCParams",
            "ZSTD_estimateCStreamSize_usingCParams",
        ] {
            let (c, r) = p.sym::<FCP>(name);
            for lvl in [lo, -100, -1, 0, 1, 3, 9, 15, 19, hi] {
                for ss in [0u64, 1024, 1 << 20, 1u64 << 32, u64::MAX] {
                    let cp = c_gc(lvl, ss, 0);
                    eq(&format!("{name}(lvl={lvl},ss={ss})"), c(cp), r(cp));
                }
            }
        }
        for name in ["ZSTD_estimateDCtxSize", "ZSTD_estimateDStreamSize"] {
            if name == "ZSTD_estimateDCtxSize" {
                let (c, r) = p.sym::<FnVoidSz>(name);
                eq(name, c(), r());
            } else {
                let (c, r) = p.sym::<FDS>(name);
                for w in [0usize, 1024, 1 << 20, 1 << 27, 1usize << 31] {
                    eq(&format!("{name}({w})"), c(w), r(w));
                }
            }
        }
        for name in ["ZSTD_estimateCDictSize", "ZSTD_estimateDDictSize"] {
            if name == "ZSTD_estimateCDictSize" {
                type F = unsafe extern "C" fn(Sz, c_int) -> Sz;
                let (c, r) = p.sym::<F>(name);
                for ds in [0usize, 1, 1024, 4096, 1 << 20] {
                    for lvl in [lo, -1, 0, 1, 9, 19, hi] {
                        eq(&format!("{name}({ds},{lvl})"), c(ds, lvl), r(ds, lvl));
                    }
                }
            } else {
                type F = unsafe extern "C" fn(Sz, c_int) -> Sz;
                let (c, r) = p.sym::<F>(name);
                for ds in [0usize, 1, 1024, 4096, 1 << 20] {
                    for dlm in [0, 1, -1, 2, 99] {
                        eq(&format!("{name}({ds},{dlm})"), c(ds, dlm), r(ds, dlm));
                    }
                }
            }
        }
        // ZSTD_estimateCDictSize_advanced(dictSize, cParams, dictLoadMethod)
        type FCDA = unsafe extern "C" fn(Sz, CParams, c_int) -> Sz;
        let (c, r) = p.sym::<FCDA>("ZSTD_estimateCDictSize_advanced");
        for ds in [0usize, 1024, 1 << 20] {
            for lvl in [lo, 1, 9, 19, hi] {
                let cp = c_gc(lvl, ds as u64, ds);
                for dlm in [0, 1, -1, 2] {
                    eq(
                        &format!("estimateCDictSize_advanced({ds},lvl={lvl},{dlm})"),
                        c(ds, cp, dlm),
                        r(ds, cp, dlm),
                    );
                }
            }
        }
    }
}

// ================================== row 53/54 — full parameter sweep + round trip

#[test]
fn row53_cparam_sweep_round_trip() {
    let a = api();
    let p = a.p;
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");
    let mut rng = Rng::new(SEED ^ 0x53);
    unsafe {
        for (prm, name) in C_PARAMS {
            let b = c_gb(prm);
            let mid = ((b.lower as i64 + b.upper as i64) / 2) as c_int;
            for val in [b.lower, mid, b.upper] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible, Shape::MixedEntropy] {
                    for _ in 0..2 {
                        let len = 1 + rng.below(120_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag =
                            format!("row53 {name}={val} shape={shape:?} len={len}");
                        if let Some(f) = diff_compress2(p, &src, &[(prm, val)], &tag) {
                            // magicless frames need a matching decoder
                            if !(prm == P_FORMAT && val != 0) {
                                diff_simple_decompress(p, &f, &src, &tag);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row54_dparam_sweep_round_trip() {
    let a = api();
    let p = a.p;
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_dParam_getBounds");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FD = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");

    let mut rng = Rng::new(SEED ^ 0x54);
    unsafe {
        for (prm, name) in D_PARAMS {
            let b = c_gb(prm);
            let mid = ((b.lower as i64 + b.upper as i64) / 2) as c_int;
            for val in [b.lower, mid, b.upper] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible] {
                    for ck in [0, 1] {
                        let len = 1 + rng.below(120_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag =
                            format!("row54 d-{name}={val} ck={ck} shape={shape:?} len={len}");
                        let Some(f) =
                            diff_compress2(p, &src, &[(P_CHECKSUMFLAG, ck)], &tag)
                        else {
                            continue;
                        };
                        let cd = c_dnew();
                        let rd = r_dnew();
                        let cs = c_dset(cd, prm, val);
                        let rs = r_dset(rd, prm, val);
                        eq(&format!("{tag}: DCtx_setParameter"), cs, rs);
                        if c_ie(cs) == 0 {
                            let cap = len + 64;
                            let mut cb = vec![0u8; cap];
                            let mut rb = vec![0u8; cap];
                            let cn = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            let rn = r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            eq(&format!("{tag}: decode ret"), cn, rn);
                            if c_ie(cn) == 0 {
                                eq_bytes(&format!("{tag}: decode"), &cb[..cn], &rb[..rn]);
                            }
                        }
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                }
            }
        }
    }
}

// ============================================================ row 72 — error API

#[test]
fn row72_error_api_valid_side() {
    let a = api();
    let p = a.p;
    let (c_gc, r_gc) = p.sym::<FnGetErrCode>("ZSTD_getErrorCode");
    let (c_gn, r_gn) = p.sym::<FnGetErrName>("ZSTD_getErrorName");
    let (c_ie, r_ie) = p.sym::<FnIsError>("ZSTD_isError");
    unsafe {
        for v in 0..=200usize {
            eq(&format!("isError({v})"), c_ie(v), r_ie(v));
            eq(&format!("getErrorCode({v})"), c_gc(v), r_gc(v));
            eq(&format!("getErrorName({v})"), cstr(c_gn(v)), cstr(r_gn(v)));
        }
    }
}
