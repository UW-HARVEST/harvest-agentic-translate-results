//! Phase B — valid-path differential tests, part 2:
//! `CONFIGS.md` rows 18, 20, 21, 22, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
//! 43, 44, 46.
//!
//! Includes the LOWEST-LEVEL entry points: the `ZSTD_compressBegin` /
//! `ZSTD_compressContinue` / `ZSTD_compressEnd` frame API and the raw
//! `ZSTD_compressBlock` / `ZSTD_decompressBlock` block API.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};

const P_LEVEL: c_int = 100;
const P_WINDOWLOG: c_int = 101;
const P_STRATEGY: c_int = 107;
const P_TARGETCBLOCKSIZE: c_int = 130;
const P_LDM: c_int = 160;
const P_LDMHASHLOG: c_int = 161;
const P_LDMMINMATCH: c_int = 162;
const P_LDMBUCKETSIZELOG: c_int = 163;
const P_LDMHASHRATELOG: c_int = 164;
const P_CONTENTSIZEFLAG: c_int = 200;
const P_CHECKSUMFLAG: c_int = 201;
const P_FORCEMAXWINDOW: c_int = 1000;
const P_SRCSIZEHINT: c_int = 1004;
const P_STABLEIN: c_int = 1006;
const P_STABLEOUT: c_int = 1007;
const P_SPLITAFTERSEQ: c_int = 1010;
const P_DETERMINISTICREFPREFIX: c_int = 1012;
const P_MAXBLOCKSIZE: c_int = 1015;
const P_BLOCKSPLITTERLEVEL: c_int = 1017;

const DP_WINDOWLOGMAX: c_int = 100;
const DP_STABLEOUT: c_int = 1001;
const DP_IGNORECHECKSUM: c_int = 1002;
const DP_NOHUFASM: c_int = 1004;
const DP_MAXBLOCKSIZE: c_int = 1005;

type F2 = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
type FD = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;

fn diff_compress2(p: &Pair, src: &[u8], opts: &[(c_int, c_int)], tag: &str) -> Option<Vec<u8>> {
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");
    unsafe {
        let cc = c_new();
        let rc = r_new();
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
        let cap = c_cb(src.len()) + 4096;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cn = c_c2(cc, cbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
        let rn = r_c2(rc, rbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
        eq(&format!("{tag}: compress2 ret"), cn, rn);
        c_free(cc);
        r_free(rc);
        if c_ie(cn) != 0 {
            return None;
        }
        eq_bytes(&format!("{tag}: frame"), &cbuf[..cn], &rbuf[..rn]);
        cbuf.truncate(cn);
        Some(cbuf)
    }
}

fn diff_decompress(p: &Pair, frame: &[u8], expect: &[u8], tag: &str) {
    let (c_de, r_de) = p.sym::<FnDecompress>("ZSTD_decompress");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    unsafe {
        let cap = expect.len() + 64;
        let mut cb = vec![0xA5u8; cap];
        let mut rb = vec![0xA5u8; cap];
        let cn = c_de(cb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        let rn = r_de(rb.as_mut_ptr() as *mut c_void, cap, frame.as_ptr() as *const c_void, frame.len());
        eq(&format!("{tag}: decompress ret"), cn, rn);
        if c_ie(cn) != 0 {
            return;
        }
        eq_bytes(&format!("{tag}: decoded"), &cb[..cn], &rb[..rn]);
        eq_bytes(&format!("{tag}: round trip"), expect, &cb[..cn]);
    }
}

// ==================================================== row 18 — long distance matching

#[test]
fn row18_long_distance_matching() {
    let p = libs();
    let (c_gb, _) = p.sym::<FnGetBounds>("ZSTD_cParam_getBounds");
    let mut rng = Rng::new(SEED ^ 0x18);
    unsafe {
        let hl = c_gb(P_LDMHASHLOG);
        let mm = c_gb(P_LDMMINMATCH);
        let bs = c_gb(P_LDMBUCKETSIZELOG);
        let hr = c_gb(P_LDMHASHRATELOG);
        for ldm in [0, 1, 2] {
            for &(a, b, c, d) in &[
                (hl.lower, mm.lower, bs.lower, hr.lower),
                (20, 64, 3, 4),
                (hl.upper, mm.upper, bs.upper, hr.upper),
                (0, 0, 0, 0), // 0 = "auto" for these
            ] {
                for wl in [17, 20, 27] {
                    for shape in [Shape::Repetitive, Shape::TextLike, Shape::MixedEntropy, Shape::Zeros] {
                        // LDM needs long inputs to trigger
                        let len = 200_000 + rng.below(400_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag = format!(
                            "row18 ldm={ldm} hl={a} mm={b} bs={c} hr={d} wl={wl} shape={shape:?} len={len}"
                        );
                        if let Some(f) = diff_compress2(
                            p,
                            &src,
                            &[
                                (P_LDM, ldm),
                                (P_LDMHASHLOG, a),
                                (P_LDMMINMATCH, b),
                                (P_LDMBUCKETSIZELOG, c),
                                (P_LDMHASHRATELOG, d),
                                (P_WINDOWLOG, wl),
                            ],
                            &tag,
                        ) {
                            diff_decompress(p, &f, &src, &tag);
                        }
                    }
                }
            }
        }
    }
}

// ==================================================== row 20 — superblock path

#[test]
fn row20_target_cblock_size_superblock() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x20);
    for tcbs in [0, 1340, 2000, 8192, 65_536, 131_072] {
        for strat in STRATEGIES {
            for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive, Shape::MixedEntropy, Shape::TwoSymbol] {
                for len in [1usize, 1339, 1340, 1341, 65_536, 131_072, 300_000] {
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row20 tcbs={tcbs} strat={strat} shape={shape:?} len={len}");
                    if let Some(f) = diff_compress2(
                        p,
                        &src,
                        &[(P_TARGETCBLOCKSIZE, tcbs), (P_STRATEGY, strat)],
                        &tag,
                    ) {
                        diff_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

// =========================================== row 21 — block splitter / splitAfterSeq

#[test]
fn row21_block_splitter_levels() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x21);
    for lvl in 0..=6 {
        for sas in [0, 1, 2] {
            for shape in [Shape::MixedEntropy, Shape::TextLike, Shape::Incompressible, Shape::TwoSymbol] {
                for _ in 0..3 {
                    let len = 50_000 + rng.below(400_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row21 split={lvl} sas={sas} shape={shape:?} len={len}");
                    if let Some(f) = diff_compress2(
                        p,
                        &src,
                        &[(P_BLOCKSPLITTERLEVEL, lvl), (P_SPLITAFTERSEQ, sas)],
                        &tag,
                    ) {
                        diff_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

// ==================================================== row 22 — maxBlockSize

#[test]
fn row22_max_block_size() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");
    let mut rng = Rng::new(SEED ^ 0x22);
    unsafe {
        for mbs in [1024, 4096, 16_384, 65_536, 131_072] {
            for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive] {
                for len in [1usize, 1023, 1024, 1025, 65_536, 300_000] {
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row22 mbs={mbs} shape={shape:?} len={len}");
                    let Some(f) = diff_compress2(p, &src, &[(P_MAXBLOCKSIZE, mbs)], &tag) else {
                        continue;
                    };
                    // decode with a matching (and a maximal) decoder maxBlockSize
                    for dmbs in [mbs, 131_072] {
                        let cd = c_dnew();
                        let rd = r_dnew();
                        eq(
                            &format!("{tag}: d-maxBlockSize={dmbs}"),
                            c_dset(cd, DP_MAXBLOCKSIZE, dmbs),
                            r_dset(rd, DP_MAXBLOCKSIZE, dmbs),
                        );
                        let cap = len + 64;
                        let mut cb = vec![0u8; cap];
                        let mut rb = vec![0u8; cap];
                        let cn = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                        let rn = r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                        eq(&format!("{tag}: decode(dmbs={dmbs}) ret"), cn, rn);
                        if c_ie(cn) == 0 {
                            eq_bytes(&format!("{tag}: decode(dmbs={dmbs})"), &cb[..cn], &rb[..rn]);
                        }
                        c_dfree(cd);
                        r_dfree(rd);
                    }
                }
            }
        }
    }
}

// ==================================================== row 25/26 — window, hints

#[test]
fn row25_window_log_force_max_window() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");
    let mut rng = Rng::new(SEED ^ 0x25);
    unsafe {
        for wl in [10, 11, 15, 17, 20, 23, 25, 27] {
            for fmw in [0, 1] {
                for shape in [Shape::Repetitive, Shape::TextLike, Shape::Incompressible] {
                    for len in [1usize, 1024, 70_000, 400_000] {
                        let src = gen(shape, len, &mut rng);
                        let tag = format!("row25 wl={wl} fmw={fmw} shape={shape:?} len={len}");
                        let Some(f) = diff_compress2(
                            p,
                            &src,
                            &[(P_WINDOWLOG, wl), (P_FORCEMAXWINDOW, fmw)],
                            &tag,
                        ) else {
                            continue;
                        };
                        for dwl in [10, wl, 27, 31] {
                            let cd = c_dnew();
                            let rd = r_dnew();
                            eq(
                                &format!("{tag}: d-windowLogMax={dwl}"),
                                c_dset(cd, DP_WINDOWLOGMAX, dwl),
                                r_dset(rd, DP_WINDOWLOGMAX, dwl),
                            );
                            let cap = len + 64;
                            let mut cb = vec![0u8; cap];
                            let mut rb = vec![0u8; cap];
                            let cn = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            let rn = r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            eq(&format!("{tag}: decode(dwl={dwl}) ret"), cn, rn);
                            if c_ie(cn) == 0 {
                                eq_bytes(&format!("{tag}: decode(dwl={dwl})"), &cb[..cn], &rb[..rn]);
                            }
                            c_dfree(cd);
                            r_dfree(rd);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row26_src_size_hint_and_deterministic_ref_prefix() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x26);
    for shape in ALL_SHAPES {
        for _ in 0..3 {
            let len = 1 + rng.below(200_000) as usize;
            let src = gen(shape, len, &mut rng);
            for hint in [
                0i32,
                len as i32,
                (len as i32).saturating_mul(10),
                (len as i32) / 10,
                1,
                i32::MAX,
            ] {
                for drp in [0, 1] {
                    let tag =
                        format!("row26 hint={hint} drp={drp} shape={shape:?} len={len}");
                    if let Some(f) = diff_compress2(
                        p,
                        &src,
                        &[(P_SRCSIZEHINT, hint), (P_DETERMINISTICREFPREFIX, drp)],
                        &tag,
                    ) {
                        diff_decompress(p, &f, &src, &tag);
                    }
                }
            }
        }
    }
}

// ============================================ rows 27-31 — streaming, all chunkings

/// Chunk patterns the state machine distinguishes.
fn chunk_sizes(pattern: u32, total: usize, rng: &mut Rng) -> Vec<usize> {
    let mut out = Vec::new();
    let mut left = total;
    while left > 0 {
        let n = match pattern {
            0 => 1,
            1 => 7,
            2 => 128,
            3 => 1 << 14,
            4 => total,
            _ => 1 + rng.below(4096) as usize,
        }
        .min(left);
        out.push(n);
        left -= n;
    }
    if out.is_empty() {
        out.push(0);
    }
    out
}

/// Row 28/29/31 — `ZSTD_compressStream2` with all `ZSTD_EndDirective` values,
/// restricted output buffers, then `ZSTD_decompressStream` with chunked output.
#[test]
fn row27_row31_streaming_round_trip() {
    let p = libs();
    let (c_cnew, r_cnew) = p.sym::<FnCreateCtx>("ZSTD_createCStream");
    let (c_cfree, r_cfree) = p.sym::<FnFreeCtx>("ZSTD_freeCStream");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDStream");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDStream");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    type FDS = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> Sz;
    type FInit = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FInitD = unsafe extern "C" fn(*mut c_void) -> Sz;
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let (c_ds, r_ds) = p.sym::<FDS>("ZSTD_decompressStream");
    let (c_ci, r_ci) = p.sym::<FInit>("ZSTD_initCStream");
    let (c_di, r_di) = p.sym::<FInitD>("ZSTD_initDStream");

    let mut rng = Rng::new(SEED ^ 0x27);

    // one streaming pass; returns the produced stream (asserted identical)
    unsafe fn stream_compress(
        p: &Pair,
        cs2: (&FCS2, &FCS2),
        ctxs: (*mut c_void, *mut c_void),
        src: &[u8],
        in_chunks: &[usize],
        out_chunk: usize,
        endop_pattern: u32,
        tag: &str,
    ) -> Option<Vec<u8>> {
        let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
        let mut cout: Vec<u8> = Vec::new();
        let mut rout: Vec<u8> = Vec::new();
        let mut cscratch = vec![0u8; out_chunk.max(1)];
        let mut rscratch = vec![0u8; out_chunk.max(1)];
        let mut off = 0usize;
        for (i, &n) in in_chunks.iter().enumerate() {
            let last = i + 1 == in_chunks.len();
            let endop: c_int = if last {
                2 // ZSTD_e_end
            } else {
                match endop_pattern {
                    0 => 0,                                  // continue
                    1 => 1,                                  // flush
                    _ => if i % 3 == 2 { 1 } else { 0 },      // mixed
                }
            };
            let slice = &src[off..off + n];
            off += n;
            let mut cin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
            let mut rin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
            loop {
                let mut cob = OutBuffer { dst: cscratch.as_mut_ptr() as *mut c_void, size: cscratch.len(), pos: 0 };
                let mut rob = OutBuffer { dst: rscratch.as_mut_ptr() as *mut c_void, size: rscratch.len(), pos: 0 };
                let cr = (cs2.0)(ctxs.0, &mut cob, &mut cin, endop);
                let rr = (cs2.1)(ctxs.1, &mut rob, &mut rin, endop);
                eq(&format!("{tag}: compressStream2 ret"), cr, rr);
                if c_ie(cr) != 0 {
                    return None;
                }
                eq(&format!("{tag}: compressStream2 in.pos"), cin.pos, rin.pos);
                eq(&format!("{tag}: compressStream2 out.pos"), cob.pos, rob.pos);
                eq_bytes(
                    &format!("{tag}: compressStream2 out bytes"),
                    &cscratch[..cob.pos],
                    &rscratch[..rob.pos],
                );
                cout.extend_from_slice(&cscratch[..cob.pos]);
                rout.extend_from_slice(&rscratch[..rob.pos]);
                let done = if last { cr == 0 } else { cin.pos == cin.size && cr == 0 };
                if done {
                    break;
                }
                if cob.pos == 0 && cin.pos == cin.size && !last {
                    break;
                }
            }
        }
        eq_bytes(&format!("{tag}: full stream"), &cout, &rout);
        Some(cout)
    }

    unsafe fn stream_decompress(
        p: &Pair,
        ds: (&FDS, &FDS),
        ctxs: (*mut c_void, *mut c_void),
        frame: &[u8],
        in_chunk: usize,
        out_chunk: usize,
        expect: &[u8],
        tag: &str,
    ) {
        let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
        let mut cout: Vec<u8> = Vec::new();
        let mut rout: Vec<u8> = Vec::new();
        let mut cs = vec![0u8; out_chunk.max(1)];
        let mut rs = vec![0u8; out_chunk.max(1)];
        let mut off = 0usize;
        while off < frame.len() {
            let n = in_chunk.min(frame.len() - off);
            let slice = &frame[off..off + n];
            off += n;
            let mut cin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
            let mut rin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
            while cin.pos < cin.size {
                let mut cob = OutBuffer { dst: cs.as_mut_ptr() as *mut c_void, size: cs.len(), pos: 0 };
                let mut rob = OutBuffer { dst: rs.as_mut_ptr() as *mut c_void, size: rs.len(), pos: 0 };
                let cr = (ds.0)(ctxs.0, &mut cob, &mut cin, );
                let rr = (ds.1)(ctxs.1, &mut rob, &mut rin, );
                eq(&format!("{tag}: decompressStream ret"), cr, rr);
                if c_ie(cr) != 0 {
                    return;
                }
                eq(&format!("{tag}: decompressStream in.pos"), cin.pos, rin.pos);
                eq(&format!("{tag}: decompressStream out.pos"), cob.pos, rob.pos);
                eq_bytes(&format!("{tag}: decode chunk"), &cs[..cob.pos], &rs[..rob.pos]);
                cout.extend_from_slice(&cs[..cob.pos]);
                rout.extend_from_slice(&rs[..rob.pos]);
                if cob.pos == 0 && cin.pos == cin.size {
                    break;
                }
            }
        }
        eq_bytes(&format!("{tag}: full decode"), &cout, &rout);
        eq_bytes(&format!("{tag}: round trip"), expect, &cout);
    }

    unsafe {
        for level in [1, 3, 9, 19] {
            for in_pat in 0..6u32 {
                for &out_chunk in &[1usize, 7, 1024, 1 << 16] {
                    for endop_pattern in 0..3u32 {
                        for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive, Shape::Zeros] {
                            let len = 1 + rng.below(60_000) as usize;
                            let src = gen(shape, len, &mut rng);
                            let chunks = chunk_sizes(in_pat, len, &mut rng);
                            let tag = format!(
                                "row27 lvl={level} inpat={in_pat} out={out_chunk} eop={endop_pattern} shape={shape:?} len={len}"
                            );
                            let cc = c_cnew();
                            let rc = r_cnew();
                            eq(&format!("{tag}: initCStream"), c_ci(cc, level), r_ci(rc, level));
                            eq(
                                &format!("{tag}: set checksum"),
                                c_set(cc, P_CHECKSUMFLAG, 1),
                                r_set(rc, P_CHECKSUMFLAG, 1),
                            );
                            let frame = stream_compress(
                                p,
                                (&c_cs2, &r_cs2),
                                (cc, rc),
                                &src,
                                &chunks,
                                out_chunk,
                                endop_pattern,
                                &tag,
                            );
                            c_cfree(cc);
                            r_cfree(rc);
                            let Some(frame) = frame else { continue };
                            if c_ie(frame.len()) != 0 {
                                continue;
                            }
                            let cd = c_dnew();
                            let rd = r_dnew();
                            eq(&format!("{tag}: initDStream"), c_di(cd), r_di(rd));
                            stream_decompress(
                                p,
                                (&c_ds, &r_ds),
                                (cd, rd),
                                &frame,
                                match in_pat { 0 => 1, 1 => 7, 2 => 128, 3 => 1 << 14, _ => frame.len().max(1) },
                                out_chunk,
                                &src,
                                &tag,
                            );
                            c_dfree(cd);
                            r_dfree(rd);
                        }
                    }
                }
            }
        }
    }
}

/// Row 27b — the deprecated-but-exported `ZSTD_compressStream` /
/// `ZSTD_flushStream` / `ZSTD_endStream` trio and `ZSTD_resetCStream`.
#[test]
fn row27b_legacy_stream_api() {
    let p = libs();
    let (c_cnew, r_cnew) = p.sym::<FnCreateCtx>("ZSTD_createCStream");
    let (c_cfree, r_cfree) = p.sym::<FnFreeCtx>("ZSTD_freeCStream");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FInit = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FCS = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> Sz;
    type FFl = unsafe extern "C" fn(*mut c_void, *mut OutBuffer) -> Sz;
    type FRst = unsafe extern "C" fn(*mut c_void, u64) -> Sz;
    let (c_ci, r_ci) = p.sym::<FInit>("ZSTD_initCStream");
    let (c_cs, r_cs) = p.sym::<FCS>("ZSTD_compressStream");
    let (c_fl, r_fl) = p.sym::<FFl>("ZSTD_flushStream");
    let (c_en, r_en) = p.sym::<FFl>("ZSTD_endStream");
    let (c_rs, r_rs) = p.sym::<FRst>("ZSTD_resetCStream");
    let (c_cis, r_cis) = p.sym::<FnVoidSz>("ZSTD_CStreamInSize");
    let (c_cos, r_cos) = p.sym::<FnVoidSz>("ZSTD_CStreamOutSize");

    let mut rng = Rng::new(SEED ^ 0x27B);
    unsafe {
        eq("CStreamInSize", c_cis(), r_cis());
        eq("CStreamOutSize", c_cos(), r_cos());
        let cc = c_cnew();
        let rc = r_cnew();
        for level in [1, 5, 12, 19] {
            for in_pat in 0..6u32 {
                for shape in [Shape::TextLike, Shape::Incompressible, Shape::Repetitive] {
                    let len = 1 + rng.below(80_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row27b lvl={level} inpat={in_pat} shape={shape:?} len={len}");
                    eq(&format!("{tag}: initCStream"), c_ci(cc, level), r_ci(rc, level));
                    eq(
                        &format!("{tag}: resetCStream"),
                        c_rs(cc, len as u64),
                        r_rs(rc, len as u64),
                    );
                    let mut cout = Vec::new();
                    let mut rout = Vec::new();
                    let mut cs = vec![0u8; c_cos()];
                    let mut rs = vec![0u8; c_cos()];
                    let mut off = 0;
                    let chunks = chunk_sizes(in_pat, len, &mut rng);
                    let mut failed = false;
                    for &n in &chunks {
                        let slice = &src[off..off + n];
                        off += n;
                        let mut cin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
                        let mut rin = InBuffer { src: slice.as_ptr() as *const c_void, size: n, pos: 0 };
                        while cin.pos < cin.size {
                            let mut cob = OutBuffer { dst: cs.as_mut_ptr() as *mut c_void, size: cs.len(), pos: 0 };
                            let mut rob = OutBuffer { dst: rs.as_mut_ptr() as *mut c_void, size: rs.len(), pos: 0 };
                            let cr = c_cs(cc, &mut cob, &mut cin);
                            let rr = r_cs(rc, &mut rob, &mut rin);
                            eq(&format!("{tag}: compressStream ret"), cr, rr);
                            if c_ie(cr) != 0 { failed = true; break; }
                            eq(&format!("{tag}: in.pos"), cin.pos, rin.pos);
                            eq_bytes(&format!("{tag}: out"), &cs[..cob.pos], &rs[..rob.pos]);
                            cout.extend_from_slice(&cs[..cob.pos]);
                            rout.extend_from_slice(&rs[..rob.pos]);
                        }
                        if failed { break; }
                        // interleave an explicit flush
                        loop {
                            let mut cob = OutBuffer { dst: cs.as_mut_ptr() as *mut c_void, size: cs.len(), pos: 0 };
                            let mut rob = OutBuffer { dst: rs.as_mut_ptr() as *mut c_void, size: rs.len(), pos: 0 };
                            let cr = c_fl(cc, &mut cob);
                            let rr = r_fl(rc, &mut rob);
                            eq(&format!("{tag}: flushStream ret"), cr, rr);
                            if c_ie(cr) != 0 { failed = true; break; }
                            eq_bytes(&format!("{tag}: flush out"), &cs[..cob.pos], &rs[..rob.pos]);
                            cout.extend_from_slice(&cs[..cob.pos]);
                            rout.extend_from_slice(&rs[..rob.pos]);
                            if cr == 0 { break; }
                        }
                        if failed { break; }
                    }
                    if failed { continue; }
                    loop {
                        let mut cob = OutBuffer { dst: cs.as_mut_ptr() as *mut c_void, size: cs.len(), pos: 0 };
                        let mut rob = OutBuffer { dst: rs.as_mut_ptr() as *mut c_void, size: rs.len(), pos: 0 };
                        let cr = c_en(cc, &mut cob);
                        let rr = r_en(rc, &mut rob);
                        eq(&format!("{tag}: endStream ret"), cr, rr);
                        if c_ie(cr) != 0 { break; }
                        eq_bytes(&format!("{tag}: end out"), &cs[..cob.pos], &rs[..rob.pos]);
                        cout.extend_from_slice(&cs[..cob.pos]);
                        rout.extend_from_slice(&rs[..rob.pos]);
                        if cr == 0 { break; }
                    }
                    eq_bytes(&format!("{tag}: whole stream"), &cout, &rout);
                    diff_decompress(p, &cout, &src, &tag);
                }
            }
        }
        c_cfree(cc);
        r_cfree(rc);
    }
}

/// Row 29 — `*_simpleArgs` variants, verifying the written-back positions.
#[test]
fn row29_simple_args() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FCSA = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *mut Sz, *const c_void, Sz, *mut Sz, c_int,
    ) -> Sz;
    type FDSA = unsafe extern "C" fn(
        *mut c_void, *mut c_void, Sz, *mut Sz, *const c_void, Sz, *mut Sz,
    ) -> Sz;
    let (c_csa, r_csa) = p.sym::<FCSA>("ZSTD_compressStream2_simpleArgs");
    let (c_dsa, r_dsa) = p.sym::<FDSA>("ZSTD_decompressStream_simpleArgs");
    let mut rng = Rng::new(SEED ^ 0x29);
    unsafe {
        let cc = c_new();
        let rc = r_new();
        let cd = c_dnew();
        let rd = r_dnew();
        for endop in [0, 1, 2, -1, 3, 99] {
            for shape in [Shape::TextLike, Shape::Incompressible] {
                for len in [0usize, 1, 1024, 70_000] {
                    let src = gen(shape, len, &mut rng);
                    let cap = c_cb(len) + 64;
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let (mut cdp, mut rdp, mut csp, mut rsp) = (0usize, 0usize, 0usize, 0usize);
                    let tag = format!("row29 eop={endop} shape={shape:?} len={len}");
                    let cr = c_csa(cc, cbuf.as_mut_ptr() as *mut c_void, cap, &mut cdp,
                                   src.as_ptr() as *const c_void, len, &mut csp, endop);
                    let rr = r_csa(rc, rbuf.as_mut_ptr() as *mut c_void, cap, &mut rdp,
                                   src.as_ptr() as *const c_void, len, &mut rsp, endop);
                    eq(&format!("{tag}: csa ret"), cr, rr);
                    eq(&format!("{tag}: csa dstPos"), cdp, rdp);
                    eq(&format!("{tag}: csa srcPos"), csp, rsp);
                    if c_ie(cr) != 0 { continue; }
                    eq_bytes(&format!("{tag}: csa out"), &cbuf[..cdp], &rbuf[..rdp]);
                    if endop != 2 || cr != 0 { continue; }
                    let dcap = len + 64;
                    let mut cdb = vec![0u8; dcap];
                    let mut rdb = vec![0u8; dcap];
                    let (mut cdp2, mut rdp2, mut csp2, mut rsp2) = (0usize, 0usize, 0usize, 0usize);
                    let cr = c_dsa(cd, cdb.as_mut_ptr() as *mut c_void, dcap, &mut cdp2,
                                   cbuf.as_ptr() as *const c_void, cdp, &mut csp2);
                    let rr = r_dsa(rd, rdb.as_mut_ptr() as *mut c_void, dcap, &mut rdp2,
                                   rbuf.as_ptr() as *const c_void, rdp, &mut rsp2);
                    eq(&format!("{tag}: dsa ret"), cr, rr);
                    eq(&format!("{tag}: dsa dstPos"), cdp2, rdp2);
                    eq(&format!("{tag}: dsa srcPos"), csp2, rsp2);
                    if c_ie(cr) == 0 {
                        eq_bytes(&format!("{tag}: dsa out"), &cdb[..cdp2], &rdb[..rdp2]);
                    }
                }
            }
        }
        c_free(cc); r_free(rc); c_dfree(cd); r_dfree(rd);
    }
}

/// Row 30 — stableInBuffer / stableOutBuffer, contract respected.
#[test]
fn row30_stable_buffers() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = p.sym::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    type FCS2 = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> Sz;
    let (c_cs2, r_cs2) = p.sym::<FCS2>("ZSTD_compressStream2");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let mut rng = Rng::new(SEED ^ 0x30);
    unsafe {
        let cc = c_new();
        let rc = r_new();
        for si in [0, 1] {
            for so in [0, 1] {
                for shape in [Shape::TextLike, Shape::Incompressible, Shape::Zeros] {
                    for len in [0usize, 1, 4096, 70_000, 200_000] {
                        let src = gen(shape, len, &mut rng);
                        let tag = format!("row30 si={si} so={so} shape={shape:?} len={len}");
                        c_reset(cc, 2); r_reset(rc, 2);
                        eq(&format!("{tag}: set stableIn"), c_set(cc, P_STABLEIN, si), r_set(rc, P_STABLEIN, si));
                        eq(&format!("{tag}: set stableOut"), c_set(cc, P_STABLEOUT, so), r_set(rc, P_STABLEOUT, so));
                        let cap = c_cb(len) + 64;
                        let mut cbuf = vec![0u8; cap];
                        let mut rbuf = vec![0u8; cap];
                        let mut cin = InBuffer { src: src.as_ptr() as *const c_void, size: len, pos: 0 };
                        let mut rin = InBuffer { src: src.as_ptr() as *const c_void, size: len, pos: 0 };
                        let mut cob = OutBuffer { dst: cbuf.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                        let mut rob = OutBuffer { dst: rbuf.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                        loop {
                            let cr = c_cs2(cc, &mut cob, &mut cin, 2);
                            let rr = r_cs2(rc, &mut rob, &mut rin, 2);
                            eq(&format!("{tag}: cs2 ret"), cr, rr);
                            if c_ie(cr) != 0 { break; }
                            eq(&format!("{tag}: in.pos"), cin.pos, rin.pos);
                            eq(&format!("{tag}: out.pos"), cob.pos, rob.pos);
                            if cr == 0 { break; }
                        }
                        eq_bytes(&format!("{tag}: frame"), &cbuf[..cob.pos], &rbuf[..rob.pos]);
                        if cob.pos > 0 {
                            diff_decompress(p, &cbuf[..cob.pos], &src, &tag);
                        }
                    }
                }
            }
        }
        c_free(cc); r_free(rc);
    }
}

/// Row 32 — decoder-side option cross-product.
#[test]
fn row32_decoder_options() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_dset, r_dset) = p.sym::<FnSetParam>("ZSTD_DCtx_setParameter");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_dd, r_dd) = p.sym::<FD>("ZSTD_decompressDCtx");
    let mut rng = Rng::new(SEED ^ 0x32);
    unsafe {
        for ck in [0, 1] {
            for ic in [0, 1] {
                for noasm in [0, 1] {
                    for mbs in [1024, 65_536, 131_072] {
                        for shape in ALL_SHAPES {
                            let len = 1 + rng.below(150_000) as usize;
                            let src = gen(shape, len, &mut rng);
                            let tag = format!(
                                "row32 ck={ck} ic={ic} noasm={noasm} mbs={mbs} shape={shape:?} len={len}"
                            );
                            let Some(f) = diff_compress2(p, &src, &[(P_CHECKSUMFLAG, ck)], &tag) else { continue };
                            let cd = c_dnew();
                            let rd = r_dnew();
                            for (prm, val) in [(DP_IGNORECHECKSUM, ic), (DP_NOHUFASM, noasm), (DP_MAXBLOCKSIZE, mbs)] {
                                eq(&format!("{tag}: dset({prm},{val})"), c_dset(cd, prm, val), r_dset(rd, prm, val));
                            }
                            let cap = len + 64;
                            let mut cb = vec![0u8; cap];
                            let mut rb = vec![0u8; cap];
                            let cn = c_dd(cd, cb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            let rn = r_dd(rd, rb.as_mut_ptr() as *mut c_void, cap, f.as_ptr() as *const c_void, f.len());
                            eq(&format!("{tag}: decode ret"), cn, rn);
                            if c_ie(cn) == 0 {
                                eq_bytes(&format!("{tag}: decode"), &cb[..cn], &rb[..rn]);
                                eq_bytes(&format!("{tag}: round trip"), &src, &cb[..cn]);
                            }
                            c_dfree(cd); r_dfree(rd);
                        }
                    }
                }
            }
        }
    }
}

/// Row 33/34 — reset directives and the CCtxParams object driving a real
/// compression via `ZSTD_CCtx_setParametersUsingCCtxParams`.
#[test]
fn row33_row34_reset_and_params_object() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_pnew, r_pnew) = p.sym::<FnCreateCtx>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = p.sym::<FnFreeCtx>("ZSTD_freeCCtxParams");
    let (c_pset, r_pset) = p.sym::<FnSetParam>("ZSTD_CCtxParams_setParameter");
    let (c_reset, r_reset) = p.sym::<FnReset>("ZSTD_CCtx_reset");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FApply = unsafe extern "C" fn(*mut c_void, *const c_void) -> Sz;
    type FInitP = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    let (c_apply, r_apply) = p.sym::<FApply>("ZSTD_CCtx_setParametersUsingCCtxParams");
    let (c_initp, r_initp) = p.sym::<FInitP>("ZSTD_CCtxParams_init");
    let (c_c2, r_c2) = p.sym::<F2>("ZSTD_compress2");
    let mut rng = Rng::new(SEED ^ 0x34);
    unsafe {
        let cc = c_new(); let rc = r_new();
        let cp = c_pnew(); let rp = r_pnew();
        for reset_dir in [0, 1, 2] {
            for level in [1, 6, 19] {
                for strat in STRATEGIES {
                    for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible] {
                        let len = 1 + rng.below(100_000) as usize;
                        let src = gen(shape, len, &mut rng);
                        let tag = format!("row34 rd={reset_dir} lvl={level} strat={strat} shape={shape:?} len={len}");
                        eq(&format!("{tag}: reset"), c_reset(cc, reset_dir), r_reset(rc, reset_dir));
                        eq(&format!("{tag}: CCtxParams_init"), c_initp(cp, level), r_initp(rp, level));
                        eq(&format!("{tag}: pset strategy"), c_pset(cp, P_STRATEGY, strat), r_pset(rp, P_STRATEGY, strat));
                        eq(&format!("{tag}: pset checksum"), c_pset(cp, P_CHECKSUMFLAG, 1), r_pset(rp, P_CHECKSUMFLAG, 1));
                        let ca = c_apply(cc, cp);
                        let ra = r_apply(rc, rp);
                        eq(&format!("{tag}: apply"), ca, ra);
                        if c_ie(ca) != 0 { continue; }
                        let cap = c_cb(len) + 64;
                        let mut cbuf = vec![0u8; cap];
                        let mut rbuf = vec![0u8; cap];
                        let cn = c_c2(cc, cbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        let rn = r_c2(rc, rbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        eq(&format!("{tag}: compress2 ret"), cn, rn);
                        if c_ie(cn) == 0 {
                            eq_bytes(&format!("{tag}: frame"), &cbuf[..cn], &rbuf[..rn]);
                            diff_decompress(p, &cbuf[..cn], &src, &tag);
                        }
                    }
                }
            }
        }
        c_pfree(cp); r_pfree(rp); c_free(cc); r_free(rc);
    }
}

// ================= row 43/44 — LOWEST-LEVEL frame and block APIs =================

/// Row 43 — manual frame construction: `ZSTD_compressBegin` +
/// `ZSTD_compressContinue`* + `ZSTD_compressEnd`, plus `ZSTD_copyCCtx`.
#[test]
fn row43_low_level_frame_api() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    let (c_cb, _) = p.sym::<FnCompressBound>("ZSTD_compressBound");
    type FBegin = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FBeginAdv =
        unsafe extern "C" fn(*mut c_void, *const c_void, Sz, Params, u64) -> Sz;
    type FGP = unsafe extern "C" fn(c_int, u64, Sz) -> Params;
    type FCont = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FBlk = unsafe extern "C" fn(*const c_void) -> Sz;
    type FCopy = unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> Sz;
    let (c_begin, r_begin) = p.sym::<FBegin>("ZSTD_compressBegin");
    let (c_beginadv, r_beginadv) = p.sym::<FBeginAdv>("ZSTD_compressBegin_advanced");
    let (c_gp, r_gp) = p.sym::<FGP>("ZSTD_getParams");
    let (c_cont, r_cont) = p.sym::<FCont>("ZSTD_compressContinue");
    let (c_end, r_end) = p.sym::<FCont>("ZSTD_compressEnd");
    let (c_bs, r_bs) = p.sym::<FBlk>("ZSTD_getBlockSize");
    let (c_cpy, r_cpy) = p.sym::<FCopy>("ZSTD_copyCCtx");

    let mut rng = Rng::new(SEED ^ 0x43);
    unsafe {
        let cc = c_new(); let rc = r_new();
        let cc2 = c_new(); let rc2 = r_new();
        for level in [1, 3, 6, 12, 19] {
            for use_srcsize in [false, true] {
                for shape in [Shape::TextLike, Shape::Repetitive, Shape::Incompressible, Shape::Zeros, Shape::MixedEntropy] {
                    let len = 1 + rng.below(400_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row43 lvl={level} adv={use_srcsize} shape={shape:?} len={len}");
                    if use_srcsize {
                        // ZSTD_compressBegin_advanced with an explicit pledgedSrcSize
                        let cprm = c_gp(level, len as u64, 0);
                        let rprm = r_gp(level, len as u64, 0);
                        eq(&format!("{tag}: getParams"), cprm, rprm);
                        eq(
                            &format!("{tag}: compressBegin_advanced"),
                            c_beginadv(cc, std::ptr::null(), 0, cprm, len as u64),
                            r_beginadv(rc, std::ptr::null(), 0, rprm, len as u64),
                        );
                    } else {
                        eq(&format!("{tag}: begin"), c_begin(cc, level), r_begin(rc, level));
                    }
                    let cbsz = c_bs(cc);
                    let rbsz = r_bs(rc);
                    eq(&format!("{tag}: getBlockSize"), cbsz, rbsz);
                    assert!(cbsz > 0);

                    let cap = c_cb(len) + 4096;
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let mut cpos = 0usize;
                    let mut rpos = 0usize;
                    let mut off = 0usize;
                    let mut copied = false;
                    let mut failed = false;
                    while off < len {
                        let n = (1 + rng.below(cbsz as u32) as usize).min(len - off).min(cbsz);
                        let last = off + n >= len;
                        let f = if last { (&c_end, &r_end) } else { (&c_cont, &r_cont) };
                        let cr = (f.0)(cc, cbuf[cpos..].as_mut_ptr() as *mut c_void, cap - cpos,
                                       src[off..].as_ptr() as *const c_void, n);
                        let rr = (f.1)(rc, rbuf[rpos..].as_mut_ptr() as *mut c_void, cap - rpos,
                                       src[off..].as_ptr() as *const c_void, n);
                        eq(&format!("{tag}: {} ret @off={off} n={n}", if last {"compressEnd"} else {"compressContinue"}), cr, rr);
                        if c_ie(cr) != 0 { failed = true; break; }
                        eq_bytes(&format!("{tag}: chunk bytes @off={off}"),
                                 &cbuf[cpos..cpos + cr], &rbuf[rpos..rpos + rr]);
                        cpos += cr; rpos += rr; off += n;
                        // exercise ZSTD_copyCCtx mid-stream exactly once
                        if !copied && off > 0 && off < len {
                            copied = true;
                            eq(&format!("{tag}: copyCCtx"),
                               c_cpy(cc2, cc, len as u64), r_cpy(rc2, rc, len as u64));
                        }
                    }
                    if failed { continue; }
                    eq_bytes(&format!("{tag}: whole frame"), &cbuf[..cpos], &rbuf[..rpos]);
                    diff_decompress(p, &cbuf[..cpos], &src, &tag);
                }
            }
        }
        c_free(cc); r_free(rc); c_free(cc2); r_free(rc2);
    }
}

/// Row 44 — the RAW block API: no frame header at all.
/// `ZSTD_compressBlock` / `ZSTD_decompressBlock` / `ZSTD_insertBlock`.
#[test]
fn row44_raw_block_api() {
    let p = libs();
    let (c_new, r_new) = p.sym::<FnCreateCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = p.sym::<FnFreeCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FBegin = unsafe extern "C" fn(*mut c_void, c_int) -> Sz;
    type FDBegin = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FBlkOp = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FIns = unsafe extern "C" fn(*mut c_void, *const c_void, Sz) -> Sz;
    type FBlk = unsafe extern "C" fn(*const c_void) -> Sz;
    let (c_begin, r_begin) = p.sym::<FBegin>("ZSTD_compressBegin");
    let (c_dbegin, r_dbegin) = p.sym::<FDBegin>("ZSTD_decompressBegin");
    let (c_cblk, r_cblk) = p.sym::<FBlkOp>("ZSTD_compressBlock");
    let (c_dblk, r_dblk) = p.sym::<FBlkOp>("ZSTD_decompressBlock");
    let (c_ins, r_ins) = p.sym::<FIns>("ZSTD_insertBlock");
    let (c_bs, r_bs) = p.sym::<FBlk>("ZSTD_getBlockSize");

    let mut rng = Rng::new(SEED ^ 0x44);
    unsafe {
        let cc = c_new(); let rc = r_new();
        let cd = c_dnew(); let rd = r_dnew();
        for level in [1, 3, 9, 19] {
            for shape in ALL_SHAPES {
                eq("row44: compressBegin", c_begin(cc, level), r_begin(rc, level));
                eq("row44: decompressBegin", c_dbegin(cd), r_dbegin(rd));
                let cbsz = c_bs(cc);
                eq("row44: getBlockSize", cbsz, r_bs(rc));
                // A contiguous window buffer, as the block API requires.
                let nblocks = 6;
                let total = cbsz * nblocks;
                let src = gen(shape, total, &mut rng);
                let mut cwin = vec![0u8; total];
                let mut rwin = vec![0u8; total];
                for b in 0..nblocks {
                    let bs = if b == nblocks - 1 {
                        // last block: exercise odd sizes at the boundary
                        (1 + rng.below(cbsz as u32) as usize).min(cbsz)
                    } else {
                        cbsz
                    };
                    let off = b * cbsz;
                    let blk = &src[off..off + bs];
                    let cap = cbsz + 1024;
                    let mut cbuf = vec![0u8; cap];
                    let mut rbuf = vec![0u8; cap];
                    let cn = c_cblk(cc, cbuf.as_mut_ptr() as *mut c_void, cap,
                                    blk.as_ptr() as *const c_void, bs);
                    let rn = r_cblk(rc, rbuf.as_mut_ptr() as *mut c_void, cap,
                                    blk.as_ptr() as *const c_void, bs);
                    let tag = format!("row44 lvl={level} shape={shape:?} blk={b} bs={bs}");
                    eq(&format!("{tag}: compressBlock ret"), cn, rn);
                    if c_ie(cn) != 0 { break; }
                    eq_bytes(&format!("{tag}: compressBlock out"), &cbuf[..cn], &rbuf[..rn]);
                    if cn == 0 {
                        // incompressible: C requires ZSTD_insertBlock on the decoder side
                        cwin[off..off + bs].copy_from_slice(blk);
                        rwin[off..off + bs].copy_from_slice(blk);
                        eq(&format!("{tag}: insertBlock"),
                           c_ins(cd, cwin[off..].as_ptr() as *const c_void, bs),
                           r_ins(rd, rwin[off..].as_ptr() as *const c_void, bs));
                        continue;
                    }
                    let cdn = c_dblk(cd, cwin[off..].as_mut_ptr() as *mut c_void, total - off,
                                     cbuf.as_ptr() as *const c_void, cn);
                    let rdn = r_dblk(rd, rwin[off..].as_mut_ptr() as *mut c_void, total - off,
                                     rbuf.as_ptr() as *const c_void, rn);
                    eq(&format!("{tag}: decompressBlock ret"), cdn, rdn);
                    if c_ie(cdn) != 0 { break; }
                    eq_bytes(&format!("{tag}: decompressBlock out"),
                             &cwin[off..off + cdn], &rwin[off..off + rdn]);
                    eq_bytes(&format!("{tag}: block round trip"), blk, &cwin[off..off + cdn]);
                }
            }
        }
        c_free(cc); r_free(rc); c_dfree(cd); r_dfree(rd);
    }
}

/// Row 44b — the low-level DECODER driver: `ZSTD_decompressContinue` fed
/// exactly `ZSTD_nextSrcSizeToDecompress` bytes at a time, plus
/// `ZSTD_nextInputType` and `ZSTD_copyDCtx`.
#[test]
fn row44b_decompress_continue_driver() {
    let p = libs();
    let (c_dnew, r_dnew) = p.sym::<FnCreateCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = p.sym::<FnFreeCtx>("ZSTD_freeDCtx");
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FDBegin = unsafe extern "C" fn(*mut c_void) -> Sz;
    type FNext = unsafe extern "C" fn(*const c_void) -> Sz;
    type FType = unsafe extern "C" fn(*const c_void) -> c_int;
    type FCont = unsafe extern "C" fn(*mut c_void, *mut c_void, Sz, *const c_void, Sz) -> Sz;
    type FCopyD = unsafe extern "C" fn(*mut c_void, *const c_void);
    let (c_dbegin, r_dbegin) = p.sym::<FDBegin>("ZSTD_decompressBegin");
    let (c_next, r_next) = p.sym::<FNext>("ZSTD_nextSrcSizeToDecompress");
    let (c_ty, r_ty) = p.sym::<FType>("ZSTD_nextInputType");
    let (c_cont, r_cont) = p.sym::<FCont>("ZSTD_decompressContinue");
    let (c_cpy, r_cpy) = p.sym::<FCopyD>("ZSTD_copyDCtx");

    let mut rng = Rng::new(SEED ^ 0x44B);
    unsafe {
        let cd = c_dnew(); let rd = r_dnew();
        let cd2 = c_dnew(); let rd2 = r_dnew();
        for level in [1, 3, 9, 19] {
            for ck in [0, 1] {
                for shape in ALL_SHAPES {
                    let len = 1 + rng.below(300_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row44b lvl={level} ck={ck} shape={shape:?} len={len}");
                    let Some(frame) = diff_compress2(
                        p, &src,
                        &[(P_LEVEL, level), (P_CHECKSUMFLAG, ck), (P_CONTENTSIZEFLAG, 1)],
                        &tag,
                    ) else { continue };

                    eq(&format!("{tag}: decompressBegin"), c_dbegin(cd), r_dbegin(rd));
                    let mut cout = vec![0u8; len + 1024];
                    let mut rout = vec![0u8; len + 1024];
                    let mut cpos = 0usize;
                    let mut rpos = 0usize;
                    let mut off = 0usize;
                    let mut copied = false;
                    loop {
                        let cn = c_next(cd);
                        let rn = r_next(rd);
                        eq(&format!("{tag}: nextSrcSizeToDecompress @{off}"), cn, rn);
                        eq(&format!("{tag}: nextInputType @{off}"), c_ty(cd), r_ty(rd));
                        if cn == 0 { break; }
                        if c_ie(cn) != 0 { break; }
                        if off + cn > frame.len() { break; }
                        let cr = c_cont(cd, cout[cpos..].as_mut_ptr() as *mut c_void, cout.len() - cpos,
                                        frame[off..].as_ptr() as *const c_void, cn);
                        let rr = r_cont(rd, rout[rpos..].as_mut_ptr() as *mut c_void, rout.len() - rpos,
                                        frame[off..].as_ptr() as *const c_void, cn);
                        eq(&format!("{tag}: decompressContinue @{off} n={cn}"), cr, rr);
                        if c_ie(cr) != 0 { break; }
                        eq_bytes(&format!("{tag}: continue out @{off}"),
                                 &cout[cpos..cpos + cr], &rout[rpos..rpos + rr]);
                        cpos += cr; rpos += rr; off += cn;
                        if !copied && off < frame.len() {
                            copied = true;
                            c_cpy(cd2, cd);
                            r_cpy(rd2, rd);
                            eq(&format!("{tag}: after copyDCtx next"), c_next(cd2), r_next(rd2));
                            eq(&format!("{tag}: after copyDCtx type"), c_ty(cd2), r_ty(rd2));
                        }
                    }
                    eq_bytes(&format!("{tag}: continue whole output"), &cout[..cpos], &rout[..rpos]);
                    if cpos == len {
                        eq_bytes(&format!("{tag}: continue round trip"), &src, &cout[..cpos]);
                    }
                }
            }
        }
        c_dfree(cd); r_dfree(rd); c_dfree(cd2); r_dfree(rd2);
    }
}

// ============================== row 46 — multi-frame and skippable frames

#[test]
fn row46_multi_frame_and_skippable() {
    let p = libs();
    let (c_ie, _) = p.sym::<FnIsError>("ZSTD_isError");
    type FWS = unsafe extern "C" fn(*mut c_void, Sz, *const c_void, Sz, c_uint) -> Sz;
    type FRS = unsafe extern "C" fn(*mut c_void, Sz, *mut c_uint, *const c_void, Sz) -> Sz;
    type FU = unsafe extern "C" fn(*const c_void, Sz) -> c_uint;
    type FU64 = unsafe extern "C" fn(*const c_void, Sz) -> u64;
    let (c_ws, r_ws) = p.sym::<FWS>("ZSTD_writeSkippableFrame");
    let (c_rs, r_rs) = p.sym::<FRS>("ZSTD_readSkippableFrame");
    let (c_isk, r_isk) = p.sym::<FU>("ZSTD_isSkippableFrame");
    let (c_fds, r_fds) = p.sym::<FU64>("ZSTD_findDecompressedSize");
    let (c_db, r_db) = p.sym::<FU64>("ZSTD_decompressBound");

    let mut rng = Rng::new(SEED ^ 0x46);
    unsafe {
        // skippable frames, all 16 magic variants
        for mv in 0..16u32 {
            for len in [0usize, 1, 7, 8, 1024, 65_536] {
                let payload = gen(Shape::Incompressible, len, &mut rng);
                let cap = len + 16;
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let cn = c_ws(cb.as_mut_ptr() as *mut c_void, cap, payload.as_ptr() as *const c_void, len, mv);
                let rn = r_ws(rb.as_mut_ptr() as *mut c_void, cap, payload.as_ptr() as *const c_void, len, mv);
                let tag = format!("row46 skippable mv={mv} len={len}");
                eq(&format!("{tag}: writeSkippableFrame ret"), cn, rn);
                if c_ie(cn) != 0 { continue; }
                eq_bytes(&format!("{tag}: skippable bytes"), &cb[..cn], &rb[..rn]);
                eq(&format!("{tag}: isSkippableFrame"),
                   c_isk(cb.as_ptr() as *const c_void, cn), r_isk(rb.as_ptr() as *const c_void, rn));
                let mut cmv = 0u32; let mut rmv = 0u32;
                let mut co = vec![0u8; len + 16];
                let mut ro = vec![0u8; len + 16];
                let cr = c_rs(co.as_mut_ptr() as *mut c_void, co.len(), &mut cmv, cb.as_ptr() as *const c_void, cn);
                let rr = r_rs(ro.as_mut_ptr() as *mut c_void, ro.len(), &mut rmv, rb.as_ptr() as *const c_void, rn);
                eq(&format!("{tag}: readSkippableFrame ret"), cr, rr);
                eq(&format!("{tag}: readSkippableFrame magicVariant"), cmv, rmv);
                if c_ie(cr) == 0 {
                    eq_bytes(&format!("{tag}: skippable payload"), &co[..cr], &ro[..rr]);
                }
            }
        }

        // concatenated frames, with skippable frames interleaved
        for nframes in [1usize, 2, 5] {
            for interleave in [false, true] {
                let mut blob: Vec<u8> = Vec::new();
                let mut expect: Vec<u8> = Vec::new();
                for i in 0..nframes {
                    let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len() as u32) as usize];
                    let len = rng.below(60_000) as usize;
                    let src = gen(shape, len, &mut rng);
                    let tag = format!("row46 multi n={nframes} i={i} shape={shape:?} len={len}");
                    let Some(f) = diff_compress2(p, &src, &[(P_LEVEL, 3), (P_CHECKSUMFLAG, (i % 2) as c_int)], &tag) else { continue };
                    blob.extend_from_slice(&f);
                    expect.extend_from_slice(&src);
                    if interleave {
                        let pay = gen(Shape::TextLike, 32, &mut rng);
                        let mut sk = vec![0u8; 64];
                        let n = c_ws(sk.as_mut_ptr() as *mut c_void, 64, pay.as_ptr() as *const c_void, 32, (i % 16) as u32);
                        if c_ie(n) == 0 { blob.extend_from_slice(&sk[..n]); }
                    }
                }
                if blob.is_empty() { continue; }
                let tag = format!("row46 multiframe n={nframes} il={interleave}");
                eq(&format!("{tag}: findDecompressedSize"),
                   c_fds(blob.as_ptr() as *const c_void, blob.len()),
                   r_fds(blob.as_ptr() as *const c_void, blob.len()));
                eq(&format!("{tag}: decompressBound"),
                   c_db(blob.as_ptr() as *const c_void, blob.len()),
                   r_db(blob.as_ptr() as *const c_void, blob.len()));
                diff_decompress(p, &blob, &expect, &tag);
            }
        }
    }
}
