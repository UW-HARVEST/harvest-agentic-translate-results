//! Phase B — CONFIGS.md rows 31..64: the `ZSTD_CCtx` parameter cross-product
//! and the `ZSTD_CCtx_params` object.
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnPledged = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize;
type FnCParamsFrom = unsafe extern "C" fn(
    *const c_void,
    u64,
    usize,
    c_int,
) -> ZSTD_compressionParameters;

/// One (C, Rust) pair of CCtx driven with the same parameter script, then the
/// same input; the compressed frame must be byte identical and must decode.
struct Rig {
    cctx: CtxPair,
    dctx: CtxPair,
    set: (FnSetParam, FnSetParam),
    get: (FnGetParam, FnGetParam),
    reset: (FnReset, FnReset),
    c2: (FnCompress2, FnCompress2),
    dec: (FnDecompressDCtx, FnDecompressDCtx),
    bound: FnSizeT1,
    pledged: (FnPledged, FnPledged),
}

impl Rig {
    unsafe fn new() -> Rig {
        Rig {
            cctx: CtxPair::cctx(),
            dctx: CtxPair::dctx(),
            set: duo::<FnSetParam>("ZSTD_CCtx_setParameter"),
            get: duo::<FnGetParam>("ZSTD_CCtx_getParameter"),
            reset: duo::<FnReset>("ZSTD_CCtx_reset"),
            c2: duo::<FnCompress2>("ZSTD_compress2"),
            dec: duo::<FnDecompressDCtx>("ZSTD_decompressDCtx"),
            bound: duo::<FnSizeT1>("ZSTD_compressBound").0,
            pledged: duo::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize"),
        }
    }
    #[track_caller]
    unsafe fn reset_all(&self) {
        let a = (self.reset.0)(self.cctx.c, ZSTD_reset_session_and_parameters);
        let b = (self.reset.1)(self.cctx.r, ZSTD_reset_session_and_parameters);
        eqv("CCtx_reset(session_and_parameters)", a, b);
    }
    /// set the parameter in both, compare the status, then read it back and
    /// compare the effective value.
    #[track_caller]
    unsafe fn set_param(&self, what: &str, p: c_int, v: c_int) -> bool {
        let a = (self.set.0)(self.cctx.c, p, v);
        let b = (self.set.1)(self.cctx.r, p, v);
        eqv(&format!("{what} setParameter({p},{v})"), a, b);
        let mut xc: c_int = -12345;
        let mut xr: c_int = -12345;
        let ga = (self.get.0)(self.cctx.c, p, &mut xc);
        let gb = (self.get.1)(self.cctx.r, p, &mut xr);
        eqv(&format!("{what} getParameter({p}) status"), ga, gb);
        eqv(&format!("{what} getParameter({p}) value"), xc, xr);
        !is_err(a)
    }
    #[track_caller]
    unsafe fn compress_and_check(&self, what: &str, src: &[u8], expect_ok: bool) {
        let cap = (self.bound)(src.len()) + 64;
        let mut oc = vec![0x5Au8; cap];
        let mut or_ = vec![0x5Au8; cap];
        let a = (self.c2.0)(
            self.cctx.c,
            oc.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        let b = (self.c2.1)(
            self.cctx.r,
            or_.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        eqv(&format!("{what} compress2 ret"), a, b);
        eqbuf(&format!("{what} compress2 dst"), &oc, &or_);
        if is_err(a) {
            assert!(!expect_ok, "{what}: unexpected compression error");
            return;
        }
        let mut pc = vec![0xA5u8; src.len() + 16];
        let mut pr = vec![0xA5u8; src.len() + 16];
        let x = (self.dec.0)(
            self.dctx.c,
            pc.as_mut_ptr() as *mut c_void,
            pc.len(),
            oc.as_ptr() as *const c_void,
            a,
        );
        let y = (self.dec.1)(
            self.dctx.r,
            pr.as_mut_ptr() as *mut c_void,
            pr.len(),
            or_.as_ptr() as *const c_void,
            b,
        );
        eqv(&format!("{what} decompressDCtx ret"), x, y);
        eqbuf(&format!("{what} decompressDCtx dst"), &pc, &pr);
    }
}

fn bounds_of(p: c_int) -> (c_int, c_int, bool) {
    unsafe {
        let (gb, _) = duo::<FnGetBounds>("ZSTD_cParam_getBounds");
        let b = gb(p);
        (b.lowerBound, b.upperBound, !is_err(b.error))
    }
}

// ------------------------------------------------------------------ row 31

#[test]
fn row31_every_param_every_boundary() {
    unsafe {
        let rig = Rig::new();
        for (name, p) in ALL_CPARAMS {
            let (lo, hi, ok) = bounds_of(*p);
            assert!(ok, "{name} has no bounds");
            let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
            let mut vals: Vec<c_int> = vec![
                lo,
                lo.saturating_add(1),
                mid,
                hi.saturating_sub(1),
                hi,
                0,
                1,
                -1,
            ];
            // one step past each end, plus extremes: also checked in Phase C
            vals.push(lo.saturating_sub(1));
            vals.push(hi.saturating_add(1));
            let mut rng = Rng::new(31 ^ (*p as u64));
            for _ in 0..12 {
                vals.push(rng.range(lo.max(i32::MIN / 2), hi.min(i32::MAX / 2)));
            }
            for v in vals {
                rig.reset_all();
                rig.set_param(&format!("row31 {name}"), *p, v);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 32-35

#[test]
fn row32_strategy_minmatch_windowlog() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(32);
        // Every (strategy, minMatch) pair selects a DIFFERENT specialised match
        // finder and a different `ZSTD_hashPtr` arm (mls 4/5/6/7 -> ZSTD_hash4 /
        // hash5 / hash6 / hash7), so each pair needs inputs that are actually
        // long enough to reach the match finder: a single random size per pair
        // can draw 0 or 1 byte and silently skip the whole matchfinder.
        for st in ALL_STRATEGIES {
            for mm in 3..=7 {
                for wl in [10, 12, 17, 20, 23] {
                    for &sz in &[0usize, 1, 300, 5000, 40_000, 140_000] {
                        for cls in 0..N_CLASSES {
                            // keep the big sizes to a couple of classes so the
                            // 9*5*5*6*8 grid stays inside the time budget
                            if sz > 40_000 && cls > 1 {
                                continue;
                            }
                            if sz == 40_000 && cls > 3 {
                                continue;
                            }
                            rig.reset_all();
                            rig.set_param("row32", ZSTD_c_strategy, st);
                            rig.set_param("row32", ZSTD_c_minMatch, mm);
                            rig.set_param("row32", ZSTD_c_windowLog, wl);
                            let src = gen_class(cls, sz, (st * 131 + mm * 17 + wl) as u64);
                            rig.compress_and_check(
                                &format!("row32 st={st} mm={mm} wl={wl} cls={cls} sz={sz}"),
                                &src,
                                true,
                            );
                        }
                    }
                }
            }
        }
        let _ = &mut rng;
    }
}

#[test]
fn row33_strategy_rowmatchfinder() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(33);
        for st in ALL_STRATEGIES {
            for rmf in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                for mm in 3..=7 {
                    for &sz in &[7usize, 1000, 30_000, 150_000] {
                        for cls in 0..N_CLASSES {
                            if sz > 30_000 && cls > 2 {
                                continue;
                            }
                            rig.reset_all();
                            rig.set_param("row33", ZSTD_c_strategy, st);
                            rig.set_param("row33", ZSTD_c_useRowMatchFinder, rmf);
                            rig.set_param("row33", ZSTD_c_minMatch, mm);
                            let src = gen_class(cls, sz, (st * 7 + rmf * 3 + mm) as u64);
                            rig.compress_and_check(
                                &format!("row33 st={st} rmf={rmf} mm={mm} cls={cls} sz={sz}"),
                                &src,
                                true,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row34_strategy_targetlength() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(34);
        for st in ALL_STRATEGIES {
            for tl in [0, 1, 16, 64, 999, 4096, 131072] {
                rig.reset_all();
                rig.set_param("row34", ZSTD_c_strategy, st);
                rig.set_param("row34", ZSTD_c_targetLength, tl);
                let cls = rng.below(N_CLASSES);
                let sz = [128usize, 9000, 60_000][rng.below(3)];
                let src = gen_class(cls, sz, rng.next_u64());
                rig.compress_and_check(&format!("row34 st={st} tl={tl} cls={cls}"), &src, true);
            }
        }
    }
}

#[test]
fn row35_hash_chain_search_grid() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(35);
        for st in ALL_STRATEGIES {
            for _ in 0..25 {
                let wl = rng.range(10, 24);
                let hl = rng.range(6, wl.min(24));
                let cl = rng.range(6, wl.min(24));
                let sl = rng.range(1, 12);
                rig.reset_all();
                rig.set_param("row35", ZSTD_c_strategy, st);
                rig.set_param("row35", ZSTD_c_windowLog, wl);
                rig.set_param("row35", ZSTD_c_hashLog, hl);
                rig.set_param("row35", ZSTD_c_chainLog, cl);
                rig.set_param("row35", ZSTD_c_searchLog, sl);
                let cls = rng.below(N_CLASSES);
                let sz = rng.below(60_000);
                let src = gen_class(cls, sz, rng.next_u64());
                // some combinations are legitimately rejected by
                // ZSTD_checkCParams; both libraries must agree, which
                // compress_and_check already asserts.
                rig.compress_and_check(
                    &format!("row35 st={st} wl={wl} hl={hl} cl={cl} sl={sl} sz={sz}"),
                    &src,
                    false,
                );
            }
        }
    }
}

// ------------------------------------------------------------------ row 36, 37

#[test]
fn row36_frame_flags_x_pledged() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(36);
        for csf in [0, 1] {
            for cks in [0, 1] {
                for did in [0, 1] {
                    for pledge in 0..3 {
                        for i in 0..3 {
                            rig.reset_all();
                            rig.set_param("row36", ZSTD_c_contentSizeFlag, csf);
                            rig.set_param("row36", ZSTD_c_checksumFlag, cks);
                            rig.set_param("row36", ZSTD_c_dictIDFlag, did);
                            let cls = rng.below(N_CLASSES);
                            let sz = [0usize, 1, 700, 40_000][rng.below(4)];
                            let src = gen_class(cls, sz, rng.next_u64());
                            let pv: c_ulonglong = match pledge {
                                0 => ZSTD_CONTENTSIZE_UNKNOWN,
                                1 => sz as c_ulonglong,
                                _ => 0,
                            };
                            let a = (rig.pledged.0)(rig.cctx.c, pv);
                            let b = (rig.pledged.1)(rig.cctx.r, pv);
                            eqv(&format!("row36 setPledgedSrcSize({pv})"), a, b);
                            rig.compress_and_check(
                                &format!("row36 csf={csf} cks={cks} did={did} pledge={pledge} i={i} sz={sz}"),
                                &src,
                                false,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row37_format_x_flags() {
    unsafe {
        let rig = Rig::new();
        let dctx = CtxPair::dctx();
        let (dsp_c, dsp_r) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (dec_c, dec_r) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let mut rng = Rng::new(37);
        for fmt in [0, 1] {
            for cks in [0, 1] {
                for csf in [0, 1] {
                    for i in 0..4 {
                        rig.reset_all();
                        rig.set_param("row37", ZSTD_c_format, fmt);
                        rig.set_param("row37", ZSTD_c_checksumFlag, cks);
                        rig.set_param("row37", ZSTD_c_contentSizeFlag, csf);
                        let cls = rng.below(N_CLASSES);
                        let sz = [0usize, 1, 2000, 70_000][rng.below(4)];
                        let src = gen_class(cls, sz, rng.next_u64());
                        // compress in both
                        let cap = (rig.bound)(sz) + 64;
                        let mut oc = vec![0u8; cap];
                        let mut or_ = vec![0u8; cap];
                        let a = (rig.c2.0)(
                            rig.cctx.c,
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let b = (rig.c2.1)(
                            rig.cctx.r,
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let w = format!("row37 fmt={fmt} cks={cks} csf={csf} i={i} sz={sz}");
                        eqv(&format!("{w} compress2"), a, b);
                        eqbuf(&format!("{w} compress2 dst"), &oc, &or_);
                        assert!(!is_err(a));
                        // decode with matching and mismatching d_format
                        for dfmt in [0, 1, 2, -1] {
                            let x = dsp_c(dctx.c, ZSTD_d_format, dfmt);
                            let y = dsp_r(dctx.r, ZSTD_d_format, dfmt);
                            eqv(&format!("{w} DCtx_setParameter(format={dfmt})"), x, y);
                            if is_err(x) {
                                continue;
                            }
                            let mut pc = vec![0u8; sz + 8];
                            let mut pr = vec![0u8; sz + 8];
                            let x = dec_c(
                                dctx.c,
                                pc.as_mut_ptr() as *mut c_void,
                                pc.len(),
                                oc.as_ptr() as *const c_void,
                                a,
                            );
                            let y = dec_r(
                                dctx.r,
                                pr.as_mut_ptr() as *mut c_void,
                                pr.len(),
                                or_.as_ptr() as *const c_void,
                                b,
                            );
                            eqv(&format!("{w} decode dfmt={dfmt} ret"), x, y);
                            eqbuf(&format!("{w} decode dfmt={dfmt} dst"), &pc, &pr);
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 38

#[test]
fn row38_ldm_grid() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(38);
        let bait = gen_class(5, 1_200_000, 38);
        let bait2 = gen_class(6, 400_000, 380);
        for ldm in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
            for _ in 0..14 {
                let hl = rng.range(6, 26);
                let mml = rng.range(4, 4096);
                let bsl = rng.range(1, 8);
                let hrl = rng.range(0, 24);
                let wl = rng.range(10, 27);
                rig.reset_all();
                rig.set_param("row38", ZSTD_c_enableLongDistanceMatching, ldm);
                rig.set_param("row38", ZSTD_c_ldmHashLog, hl);
                rig.set_param("row38", ZSTD_c_ldmMinMatch, mml);
                rig.set_param("row38", ZSTD_c_ldmBucketSizeLog, bsl);
                rig.set_param("row38", ZSTD_c_ldmHashRateLog, hrl);
                rig.set_param("row38", ZSTD_c_windowLog, wl);
                rig.set_param("row38", ZSTD_c_compressionLevel, rng.range(1, 12));
                let src: &[u8] = if rng.below(2) == 0 { &bait } else { &bait2 };
                rig.compress_and_check(
                    &format!("row38 ldm={ldm} hl={hl} mml={mml} bsl={bsl} hrl={hrl} wl={wl} n={}", src.len()),
                    src,
                    false,
                );
            }
        }
    }
}

// ------------------------------------------------------------------ rows 39-43

#[test]
fn row39_literal_compression_mode() {
    unsafe {
        let rig = Rig::new();
        for lcm in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
            for cls in 0..N_CLASSES {
                for &sz in &[0usize, 1, 5, 900, 30_000, 140_000] {
                    rig.reset_all();
                    rig.set_param("row39", ZSTD_c_literalCompressionMode, lcm);
                    let src = gen_class(cls, sz, 39);
                    rig.compress_and_check(&format!("row39 lcm={lcm} cls={cls} sz={sz}"), &src, true);
                }
            }
        }
    }
}

#[test]
fn row40_target_cblock_size() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(40);
        for tcbs in [0, 1340, 2000, 4096, 65536, 131072] {
            for lvl in [1, 3, 9, 19] {
                for cls in [0usize, 3, 4, 5, 6] {
                    rig.reset_all();
                    rig.set_param("row40", ZSTD_c_targetCBlockSize, tcbs);
                    rig.set_param("row40", ZSTD_c_compressionLevel, lvl);
                    let sz = 200_000 + rng.below(80_000);
                    let src = gen_class(cls, sz, rng.next_u64());
                    rig.compress_and_check(
                        &format!("row40 tcbs={tcbs} lvl={lvl} cls={cls} sz={sz}"),
                        &src,
                        true,
                    );
                }
            }
        }
    }
}

#[test]
fn row41_max_block_size() {
    unsafe {
        let rig = Rig::new();
        let dctx = CtxPair::dctx();
        let (dsp_c, dsp_r) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let mut rng = Rng::new(41);
        for mbs in [0, 1024, 2048, 4096, 65536, 131072] {
            for dmbs in [0, 1024, 131072] {
                for cls in [3usize, 4, 5] {
                    rig.reset_all();
                    rig.set_param("row41", ZSTD_c_maxBlockSize, mbs);
                    let x = dsp_c(dctx.c, ZSTD_d_maxBlockSize, dmbs);
                    let y = dsp_r(dctx.r, ZSTD_d_maxBlockSize, dmbs);
                    eqv(&format!("row41 DCtx maxBlockSize={dmbs}"), x, y);
                    let sz = 260_000 + rng.below(20_000);
                    let src = gen_class(cls, sz, rng.next_u64());
                    let cap = (rig.bound)(sz) + 64;
                    let mut oc = vec![0u8; cap];
                    let mut or_ = vec![0u8; cap];
                    let a = (rig.c2.0)(
                        rig.cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let b = (rig.c2.1)(
                        rig.cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let w = format!("row41 mbs={mbs} dmbs={dmbs} cls={cls}");
                    eqv(&format!("{w} compress2"), a, b);
                    eqbuf(&format!("{w} dst"), &oc, &or_);
                    if is_err(a) {
                        continue;
                    }
                    let (dec_c, dec_r) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
                    let mut pc = vec![0u8; sz + 8];
                    let mut pr = vec![0u8; sz + 8];
                    let x = dec_c(
                        dctx.c,
                        pc.as_mut_ptr() as *mut c_void,
                        pc.len(),
                        oc.as_ptr() as *const c_void,
                        a,
                    );
                    let y = dec_r(
                        dctx.r,
                        pr.as_mut_ptr() as *mut c_void,
                        pr.len(),
                        or_.as_ptr() as *const c_void,
                        b,
                    );
                    eqv(&format!("{w} decode"), x, y);
                    eqbuf(&format!("{w} decode dst"), &pc, &pr);
                }
            }
        }
    }
}

#[test]
fn row42_block_splitter() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(42);
        // mixed-entropy input: alternating incompressible / text / zeros
        let mut mixed = Vec::new();
        for i in 0..30 {
            mixed.extend_from_slice(&gen_class(i % N_CLASSES, 12_000, 42 + i as u64));
        }
        for bsl in 0..=6 {
            for sas in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                for lvl in [1, 5, 13, 19] {
                    rig.reset_all();
                    rig.set_param("row42", ZSTD_c_blockSplitterLevel, bsl);
                    rig.set_param("row42", ZSTD_c_splitAfterSequences, sas);
                    rig.set_param("row42", ZSTD_c_compressionLevel, lvl);
                    rig.compress_and_check(
                        &format!("row42 bsl={bsl} sas={sas} lvl={lvl}"),
                        &mixed,
                        true,
                    );
                }
            }
        }
        // randomized
        for i in 0..40 {
            rig.reset_all();
            rig.set_param("row42r", ZSTD_c_blockSplitterLevel, rng.range(0, 6));
            rig.set_param("row42r", ZSTD_c_splitAfterSequences, rng.range(0, 2));
            rig.set_param("row42r", ZSTD_c_compressionLevel, rng.range(-3, 19));
            let sz = rng.below(250_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            rig.compress_and_check(&format!("row42r i={i} sz={sz}"), &src, true);
        }
    }
}

#[test]
fn row43_src_size_hint() {
    unsafe {
        let rig = Rig::new();
        for hint in [0, 1, 100, 1024, 1 << 20, i32::MAX] {
            for &sz in &[0usize, 1, 1024, 60_000, 200_000] {
                for lvl in [1, 3, 9, 19] {
                    rig.reset_all();
                    rig.set_param("row43", ZSTD_c_srcSizeHint, hint);
                    rig.set_param("row43", ZSTD_c_compressionLevel, lvl);
                    let src = gen_class(4, sz, 43);
                    rig.compress_and_check(
                        &format!("row43 hint={hint} sz={sz} lvl={lvl}"),
                        &src,
                        true,
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 44

#[test]
fn row44_force_max_window_x_windowlogmax() {
    unsafe {
        let rig = Rig::new();
        let dctx = CtxPair::dctx();
        let (dsp_c, dsp_r) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (dec_c, dec_r) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let (ds_c, ds_r) = duo::<FnDStream>("ZSTD_decompressStream");
        let (idc, idr) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_initDStream");
        for fmw in [0, 1] {
            for wl in [10, 17, 21, 27] {
                for dwlm in [0, 10, 17, 21, 27, 31] {
                    rig.reset_all();
                    rig.set_param("row44", ZSTD_c_forceMaxWindow, fmw);
                    rig.set_param("row44", ZSTD_c_windowLog, wl);
                    rig.set_param("row44", ZSTD_c_compressionLevel, 3);
                    let sz = 300_000usize;
                    let src = gen_class(4, sz, 44);
                    let cap = (rig.bound)(sz) + 64;
                    let mut oc = vec![0u8; cap];
                    let mut or_ = vec![0u8; cap];
                    let a = (rig.c2.0)(
                        rig.cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let b = (rig.c2.1)(
                        rig.cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let w = format!("row44 fmw={fmw} wl={wl} dwlm={dwlm}");
                    eqv(&format!("{w} compress2"), a, b);
                    eqbuf(&format!("{w} dst"), &oc, &or_);
                    assert!(!is_err(a));
                    let x = dsp_c(dctx.c, ZSTD_d_windowLogMax, dwlm);
                    let y = dsp_r(dctx.r, ZSTD_d_windowLogMax, dwlm);
                    eqv(&format!("{w} DCtx windowLogMax"), x, y);
                    if is_err(x) {
                        continue;
                    }
                    let mut pc = vec![0u8; sz + 8];
                    let mut pr = vec![0u8; sz + 8];
                    let x = dec_c(
                        dctx.c,
                        pc.as_mut_ptr() as *mut c_void,
                        pc.len(),
                        oc.as_ptr() as *const c_void,
                        a,
                    );
                    let y = dec_r(
                        dctx.r,
                        pr.as_mut_ptr() as *mut c_void,
                        pr.len(),
                        or_.as_ptr() as *const c_void,
                        b,
                    );
                    eqv(&format!("{w} one-shot decode"), x, y);
                    eqbuf(&format!("{w} one-shot dst"), &pc, &pr);

                    // streaming decode is where windowLogMax actually bites
                    let ds = CtxPair::dstream();
                    let dsp = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
                    eqv(
                        &format!("{w} DStream windowLogMax"),
                        (dsp.0)(ds.c, ZSTD_d_windowLogMax, dwlm),
                        (dsp.1)(ds.r, ZSTD_d_windowLogMax, dwlm),
                    );
                    eqv(&format!("{w} initDStream"), idc(ds.c), idr(ds.r));
                    let mut sc_ = vec![0u8; sz + 8];
                    let mut sr_ = vec![0u8; sz + 8];
                    let mut ibc = ZSTD_inBuffer { src: oc.as_ptr() as *const c_void, size: a, pos: 0 };
                    let mut ibr = ZSTD_inBuffer { src: or_.as_ptr() as *const c_void, size: b, pos: 0 };
                    let mut obc = ZSTD_outBuffer { dst: sc_.as_mut_ptr() as *mut c_void, size: sc_.len(), pos: 0 };
                    let mut obr = ZSTD_outBuffer { dst: sr_.as_mut_ptr() as *mut c_void, size: sr_.len(), pos: 0 };
                    let mut step = 0;
                    loop {
                        step += 1;
                        let ra = ds_c(ds.c, &mut obc, &mut ibc);
                        let rb = ds_r(ds.r, &mut obr, &mut ibr);
                        eqv(&format!("{w} decompressStream step={step}"), ra, rb);
                        eqv(&format!("{w} in.pos step={step}"), ibc.pos, ibr.pos);
                        eqv(&format!("{w} out.pos step={step}"), obc.pos, obr.pos);
                        if is_err(ra) || ra == 0 || step > 200 || (ibc.pos == ibc.size && obc.pos == obc.size) {
                            break;
                        }
                    }
                    eqbuf(&format!("{w} streaming out"), &sc_, &sr_);
                }
            }
        }
        // ZSTD_DCtx_setMaxWindowSize over a grid, on a fresh DCtx each time
        let (mwc, mwr) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> usize>(
            "ZSTD_DCtx_setMaxWindowSize",
        );
        for ws in [0usize, 1, 1 << 9, 1 << 10, 1 << 17, 1 << 27, (1usize << 31) - 1, usize::MAX] {
            let d = CtxPair::dctx();
            eqv(
                &format!("row44 setMaxWindowSize({ws})"),
                mwc(d.c, ws),
                mwr(d.r, ws),
            );
        }
        // ZSTD_DCtx_setFormat over valid + invalid enum values
        let (sfc, sfr) = duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>(
            "ZSTD_DCtx_setFormat",
        );
        for f in [0, 1, 2, -1, 12345] {
            let d = CtxPair::dctx();
            eqv(&format!("row44 setFormat({f})"), sfc(d.c, f), sfr(d.r, f));
        }
    }
}

// ------------------------------------------------------------------ rows 45-51

#[test]
fn row45_mt_params_single_threaded_build() {
    unsafe {
        let rig = Rig::new();
        for rs in [0, 1] {
            for js in [0, 1024, 1 << 20, 1 << 24] {
                for ol in [0, 1, 5, 9] {
                    rig.reset_all();
                    rig.set_param("row45", ZSTD_c_rsyncable, rs);
                    rig.set_param("row45", ZSTD_c_jobSize, js);
                    rig.set_param("row45", ZSTD_c_overlapLog, ol);
                    rig.set_param("row45", ZSTD_c_nbWorkers, 0);
                    let src = gen_class(4, 90_000, 45);
                    rig.compress_and_check(&format!("row45 rs={rs} js={js} ol={ol}"), &src, true);
                }
            }
        }
        // nbWorkers > 0 in a non-MT build: both must behave identically
        for nw in [1, 2, 4, 200] {
            rig.reset_all();
            rig.set_param("row45 nbWorkers", ZSTD_c_nbWorkers, nw);
            let src = gen_class(4, 20_000, 451);
            rig.compress_and_check(&format!("row45 nbWorkers={nw}"), &src, false);
        }
    }
}

#[test]
fn row49_50_51_switches() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(4951);
        for vs in [0, 1] {
            for espf in [0, 1] {
                for rr in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                    for _ in 0..4 {
                        rig.reset_all();
                        rig.set_param("row49", ZSTD_c_validateSequences, vs);
                        rig.set_param("row50", ZSTD_c_enableSeqProducerFallback, espf);
                        rig.set_param("row51", ZSTD_c_repcodeResolution, rr);
                        rig.set_param("row51", ZSTD_c_compressionLevel, rng.range(1, 19));
                        let sz = rng.below(80_000);
                        let src = gen_class(rng.below(N_CLASSES), sz, rng.next_u64());
                        rig.compress_and_check(
                            &format!("row49-51 vs={vs} espf={espf} rr={rr} sz={sz}"),
                            &src,
                            true,
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 52

#[test]
fn row52_set_cparams_fparams_params() {
    unsafe {
        let rig = Rig::new();
        let (cpc, cpr) = duo::<unsafe extern "C" fn(*mut c_void, ZSTD_compressionParameters) -> usize>(
            "ZSTD_CCtx_setCParams",
        );
        let (fpc, fpr) = duo::<unsafe extern "C" fn(*mut c_void, ZSTD_frameParameters) -> usize>(
            "ZSTD_CCtx_setFParams",
        );
        let (apc, apr) =
            duo::<unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> usize>("ZSTD_CCtx_setParams");
        let (gc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let mut rng = Rng::new(52);
        for lvl in [1, 3, 6, 12, 19, 22] {
            let base = gc(lvl, 0, 0);
            for i in 0..8 {
                let mut cp = base;
                if i > 0 {
                    cp.windowLog = rng.range(10, 25) as c_uint;
                    cp.hashLog = rng.range(6, 24) as c_uint;
                    cp.chainLog = rng.range(6, 24) as c_uint;
                    cp.searchLog = rng.range(1, 12) as c_uint;
                    cp.minMatch = rng.range(3, 7) as c_uint;
                    cp.targetLength = rng.range(0, 5000) as c_uint;
                    cp.strategy = rng.range(1, 9) as c_uint;
                }
                rig.reset_all();
                eqv(
                    &format!("row52 setCParams({cp:?})"),
                    cpc(rig.cctx.c, cp),
                    cpr(rig.cctx.r, cp),
                );
                let fp = ZSTD_frameParameters {
                    contentSizeFlag: rng.range(0, 1),
                    checksumFlag: rng.range(0, 1),
                    noDictIDFlag: rng.range(0, 1),
                };
                eqv(
                    &format!("row52 setFParams({fp:?})"),
                    fpc(rig.cctx.c, fp),
                    fpr(rig.cctx.r, fp),
                );
                let sz = rng.below(60_000);
                let src = gen_class(rng.below(N_CLASSES), sz, rng.next_u64());
                rig.compress_and_check(&format!("row52 lvl={lvl} i={i} sz={sz}"), &src, false);

                rig.reset_all();
                let all = ZSTD_parameters { cParams: cp, fParams: fp };
                eqv(
                    &format!("row52 setParams({all:?})"),
                    apc(rig.cctx.c, all),
                    apr(rig.cctx.r, all),
                );
                rig.compress_and_check(&format!("row52b lvl={lvl} i={i} sz={sz}"), &src, false);
            }
        }
        // invalid cParams must be rejected identically
        for bad in [
            ZSTD_compressionParameters::default(),
            ZSTD_compressionParameters { windowLog: 99, ..Default::default() },
            ZSTD_compressionParameters { strategy: 42, ..Default::default() },
        ] {
            rig.reset_all();
            eqv(
                &format!("row52 setCParams bad {bad:?}"),
                cpc(rig.cctx.c, bad),
                cpr(rig.cctx.r, bad),
            );
        }
    }
}

// ------------------------------------------------------------------ row 53

#[test]
fn row53_reset_directives() {
    unsafe {
        let rig = Rig::new();
        let (c2c, c2r) = rig.c2;
        let mut rng = Rng::new(53);
        for d in [
            ZSTD_reset_session_only,
            ZSTD_reset_parameters,
            ZSTD_reset_session_and_parameters,
            0,
            4,
            -1,
        ] {
            // reset at three different points of a session
            for point in 0..3 {
                rig.reset_all();
                rig.set_param("row53", ZSTD_c_compressionLevel, 7);
                rig.set_param("row53", ZSTD_c_checksumFlag, 1);
                let sz = rng.below(30_000);
                let src = gen_class(rng.below(N_CLASSES), sz, rng.next_u64());
                let cap = (rig.bound)(sz) + 64;
                if point == 1 {
                    // reset immediately after setting parameters
                    eqv(
                        &format!("row53 reset({d}) @pre"),
                        (rig.reset.0)(rig.cctx.c, d),
                        (rig.reset.1)(rig.cctx.r, d),
                    );
                }
                let mut oc = vec![0u8; cap];
                let mut or_ = vec![0u8; cap];
                let a = c2c(
                    rig.cctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                let b = c2r(
                    rig.cctx.r,
                    or_.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                eqv(&format!("row53 d={d} point={point} compress"), a, b);
                eqbuf(&format!("row53 d={d} point={point} dst"), &oc, &or_);
                if point == 2 {
                    eqv(
                        &format!("row53 reset({d}) @post"),
                        (rig.reset.0)(rig.cctx.c, d),
                        (rig.reset.1)(rig.cctx.r, d),
                    );
                }
                // whatever the state, the next compression must still agree
                let mut oc2 = vec![0u8; cap];
                let mut or2 = vec![0u8; cap];
                let a2 = c2c(
                    rig.cctx.c,
                    oc2.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                let b2 = c2r(
                    rig.cctx.r,
                    or2.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                eqv(&format!("row53 d={d} point={point} compress#2"), a2, b2);
                eqbuf(&format!("row53 d={d} point={point} dst#2"), &oc2, &or2);
            }
        }
    }
}

// ------------------------------------------------------------------ row 54

#[test]
fn row54_pledged_src_size() {
    unsafe {
        let rig = Rig::new();
        let mut rng = Rng::new(54);
        for i in 0..80 {
            let sz = rng.below(50_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            for pv in [
                sz as c_ulonglong,
                ZSTD_CONTENTSIZE_UNKNOWN,
                0,
                1,
                (sz as c_ulonglong).wrapping_add(1),
                (sz as c_ulonglong).saturating_sub(1),
                u64::MAX / 2,
            ] {
                rig.reset_all();
                eqv(
                    &format!("row54 i={i} setPledgedSrcSize({pv})"),
                    (rig.pledged.0)(rig.cctx.c, pv),
                    (rig.pledged.1)(rig.cctx.r, pv),
                );
                rig.compress_and_check(&format!("row54 i={i} sz={sz} pv={pv}"), &src, false);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 55, 56

#[test]
fn row55_frame_progression_and_flush() {
    unsafe {
        let (fpc, fpr) = duo::<unsafe extern "C" fn(*const c_void) -> ZSTD_frameProgression>(
            "ZSTD_getFrameProgression",
        );
        let (tfc, tfr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_toFlushNow");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rs_c, rs_r) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (osz, _) = duo::<FnSizeT0>("ZSTD_CStreamOutSize");
        let cctx = CtxPair::cctx();
        let mut rng = Rng::new(55);
        for st in ALL_STRATEGIES {
            let src = gen_class(rng.below(N_CLASSES), 120_000, st as u64);
            eqv(
                "row55 reset",
                rs_c(cctx.c, ZSTD_reset_session_and_parameters),
                rs_r(cctx.r, ZSTD_reset_session_and_parameters),
            );
            eqv(
                "row55 set strategy",
                sp_c(cctx.c, ZSTD_c_strategy, st),
                sp_r(cctx.r, ZSTD_c_strategy, st),
            );
            let ocap = osz();
            let mut outc = vec![0u8; ocap];
            let mut outr = vec![0u8; ocap];
            let mut inposc = 0usize;
            let mut inposr = 0usize;
            let mut donec = false;
            let mut doner = false;
            let mut step = 0;
            while !(donec && doner) && step < 5000 {
                step += 1;
                let chunk = 7000usize;
                let inend = (inposc + chunk).min(src.len());
                let mut ibc = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: inend,
                    pos: inposc,
                };
                let mut ibr = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: inend,
                    pos: inposr,
                };
                let mut obc = ZSTD_outBuffer {
                    dst: outc.as_mut_ptr() as *mut c_void,
                    size: ocap,
                    pos: 0,
                };
                let mut obr = ZSTD_outBuffer {
                    dst: outr.as_mut_ptr() as *mut c_void,
                    size: ocap,
                    pos: 0,
                };
                let op = if inend == src.len() { ZSTD_e_end } else { ZSTD_e_continue };
                let a = s2c(cctx.c, &mut obc, &mut ibc, op);
                let b = s2r(cctx.r, &mut obr, &mut ibr, op);
                eqv(&format!("row55 st={st} step={step} compressStream2"), a, b);
                eqv(&format!("row55 st={st} step={step} in.pos"), ibc.pos, ibr.pos);
                eqv(&format!("row55 st={st} step={step} out.pos"), obc.pos, obr.pos);
                eqbuf(
                    &format!("row55 st={st} step={step} out"),
                    &outc[..obc.pos],
                    &outr[..obr.pos],
                );
                eqv(
                    &format!("row55 st={st} step={step} getFrameProgression"),
                    fpc(cctx.c),
                    fpr(cctx.r),
                );
                eqv(
                    &format!("row55 st={st} step={step} toFlushNow"),
                    tfc(cctx.c),
                    tfr(cctx.r),
                );
                inposc = ibc.pos;
                inposr = ibr.pos;
                if op == ZSTD_e_end && a == 0 {
                    donec = true;
                    doner = true;
                }
            }
        }
    }
}

#[test]
fn row56_cctx_trace() {
    unsafe {
        let (tc, tr) = duo::<unsafe extern "C" fn(*mut c_void, usize)>("ZSTD_CCtx_trace");
        let cctx = CtxPair::cctx();
        for extra in [0usize, 1, 1000, usize::MAX / 2] {
            tc(cctx.c, extra);
            tr(cctx.r, extra);
        }
    }
}

// ------------------------------------------------------------------ rows 57-64

#[test]
fn row57_58_59_60_cctxparams_object() {
    unsafe {
        let p = CtxPair::cctx_params();
        let (rc, rr) = duo::<FnFreePtr>("ZSTD_CCtxParams_reset");
        let (ic, ir) = duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
        let (iac, iar) = duo::<unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> usize>(
            "ZSTD_CCtxParams_init_advanced",
        );
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (gc, gr) = duo::<FnGetParam>("ZSTD_CCtxParams_getParameter");
        let (gpc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters>("ZSTD_getParams");

        eqv("row57 CCtxParams_reset", rc(p.c), rr(p.r));

        for lvl in [-131072, -5, -1, 0, 1, 3, 9, 19, 22, 23, 100] {
            eqv(
                &format!("row58 CCtxParams_init({lvl})"),
                ic(p.c, lvl),
                ir(p.r, lvl),
            );
            // every parameter must read back identically after init
            for (name, prm) in ALL_CPARAMS {
                let mut xc: c_int = -999;
                let mut xr: c_int = -999;
                let a = gc(p.c, *prm, &mut xc);
                let b = gr(p.r, *prm, &mut xr);
                eqv(&format!("row58 lvl={lvl} get {name} status"), a, b);
                eqv(&format!("row58 lvl={lvl} get {name} value"), xc, xr);
            }
        }

        let mut rng = Rng::new(59);
        for lvl in [1, 3, 9, 19] {
            for ss in [0u64, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                let pr_ = gpc(lvl, ss, 0);
                eqv(
                    &format!("row59 CCtxParams_init_advanced({pr_:?})"),
                    iac(p.c, pr_),
                    iar(p.r, pr_),
                );
            }
        }
        // invalid parameter structs
        for bad in [
            ZSTD_parameters::default(),
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters { windowLog: 99, ..Default::default() },
                fParams: ZSTD_frameParameters::default(),
            },
        ] {
            eqv(
                &format!("row59 init_advanced bad {bad:?}"),
                iac(p.c, bad),
                iar(p.r, bad),
            );
        }

        // row 60: every param x {min, min+1, mid, max-1, max} round-trips
        for (name, prm) in ALL_CPARAMS {
            let (lo, hi, _) = bounds_of(*prm);
            let mid = lo.wrapping_add(hi.wrapping_sub(lo) / 2);
            let mut vals = vec![lo, lo.saturating_add(1), mid, hi.saturating_sub(1), hi, 0, -1];
            for _ in 0..8 {
                vals.push(rng.range(lo.max(i32::MIN / 2), hi.min(i32::MAX / 2)));
            }
            for v in vals {
                let a = sc(p.c, *prm, v);
                let b = sr(p.r, *prm, v);
                eqv(&format!("row60 set {name}={v}"), a, b);
                let mut xc: c_int = -999;
                let mut xr: c_int = -999;
                let ga = gc(p.c, *prm, &mut xc);
                let gb = gr(p.r, *prm, &mut xr);
                eqv(&format!("row60 get {name} status"), ga, gb);
                eqv(&format!("row60 get {name} value"), xc, xr);
            }
        }
    }
}

#[test]
fn row61_62_64_use_cctxparams() {
    unsafe {
        let p = CtxPair::cctx_params();
        let rig = Rig::new();
        let (sc, sr) = duo::<FnSetParam>("ZSTD_CCtxParams_setParameter");
        let (ic, ir) = duo::<unsafe extern "C" fn(*mut c_void, c_int) -> usize>("ZSTD_CCtxParams_init");
        let (uc, ur) = duo::<unsafe extern "C" fn(*mut c_void, *const c_void) -> usize>(
            "ZSTD_CCtx_setParametersUsingCCtxParams",
        );
        let (fc, fr) = duo::<FnCParamsFrom>("ZSTD_getCParamsFromCCtxParams");
        let (ec, er) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_estimateCCtxSize_usingCCtxParams");
        let (esc, esr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>(
            "ZSTD_estimateCStreamSize_usingCCtxParams",
        );
        let mut rng = Rng::new(61);
        for i in 0..60 {
            let lvl = rng.range(-5, 22);
            eqv(
                &format!("row61 i={i} CCtxParams_init({lvl})"),
                ic(p.c, lvl),
                ir(p.r, lvl),
            );
            // random but valid parameter tweaks
            let tweaks: Vec<(c_int, c_int)> = vec![
                (ZSTD_c_strategy, rng.range(1, 9)),
                (ZSTD_c_windowLog, rng.range(10, 24)),
                (ZSTD_c_minMatch, rng.range(3, 7)),
                (ZSTD_c_checksumFlag, rng.range(0, 1)),
                (ZSTD_c_contentSizeFlag, rng.range(0, 1)),
                (ZSTD_c_dictIDFlag, rng.range(0, 1)),
                (ZSTD_c_enableLongDistanceMatching, rng.range(0, 2)),
                // NOTE: ZSTD_c_ldmMinMatch must be set explicitly whenever LDM
                // is enabled on a bare ZSTD_CCtx_params, because
                // ZSTD_estimateCCtxSize_usingCCtxParams() reaches
                // ZSTD_ldm_getMaxNbSeq() -> `maxChunkSize / minMatchLength`
                // before ZSTD_ldm_adjustParameters() has filled the default in.
                // With minMatchLength == 0 the *C* library divides by zero and
                // dies with SIGFPE; see the "Upstream C crashes" note in
                // CONFIGS.md. Both libraries transliterate that division
                // identically, so the configuration is not differentiable and
                // is excluded here.
                (ZSTD_c_ldmMinMatch, rng.range(4, 4096)),
                (ZSTD_c_literalCompressionMode, rng.range(0, 2)),
                (ZSTD_c_useRowMatchFinder, rng.range(0, 2)),
                (ZSTD_c_targetCBlockSize, if rng.below(2) == 0 { 0 } else { 2000 }),
                (ZSTD_c_blockSplitterLevel, rng.range(0, 6)),
                (ZSTD_c_maxBlockSize, if rng.below(2) == 0 { 0 } else { 1 << rng.range(10, 17) }),
            ];
            for (prm, v) in &tweaks {
                eqv(
                    &format!("row61 i={i} setParameter({prm},{v})"),
                    sc(p.c, *prm, *v),
                    sr(p.r, *prm, *v),
                );
            }
            for ss in [0u64, 1, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                for ds in [0usize, 1024, 1 << 20] {
                    for mode in [0, 1, 2, 3, 4, -1] {
                        eqv(
                            &format!("row62 i={i} getCParamsFromCCtxParams({ss},{ds},{mode})"),
                            fc(p.c, ss, ds, mode),
                            fr(p.r, ss, ds, mode),
                        );
                    }
                }
            }
            eqv(
                &format!("row64 i={i} estimateCCtxSize_usingCCtxParams"),
                ec(p.c),
                er(p.r),
            );
            eqv(
                &format!("row64 i={i} estimateCStreamSize_usingCCtxParams"),
                esc(p.c),
                esr(p.r),
            );

            rig.reset_all();
            eqv(
                &format!("row61 i={i} setParametersUsingCCtxParams"),
                uc(rig.cctx.c, p.c),
                ur(rig.cctx.r, p.r),
            );
            let sz = rng.below(90_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            rig.compress_and_check(&format!("row61 i={i} sz={sz} lvl={lvl}"), &src, false);
        }
    }
}

#[test]
fn row63_register_sequence_producer() {
    unsafe {
        type FnRegC = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void);
        let (rc, rr) = duo::<FnRegC>("ZSTD_registerSequenceProducer");
        let (pc, pr) = duo::<FnRegC>("ZSTD_CCtxParams_registerSequenceProducer");
        let rig = Rig::new();
        let params = CtxPair::cctx_params();
        let mut state: u64 = 0xDEADBEEF;
        // register NULL (documented as "clear"), on both objects, in both libs
        for round in 0..3 {
            rig.reset_all();
            rc(rig.cctx.c, &mut state as *mut u64 as *mut c_void, std::ptr::null());
            rr(rig.cctx.r, &mut state as *mut u64 as *mut c_void, std::ptr::null());
            pc(params.c, &mut state as *mut u64 as *mut c_void, std::ptr::null());
            pr(params.r, &mut state as *mut u64 as *mut c_void, std::ptr::null());
            let src = gen_class(4, 30_000 + round * 1000, 63);
            rig.compress_and_check(&format!("row63 round={round}"), &src, true);
        }
        // with fallback enabled/disabled and a NULL producer
        for espf in [0, 1] {
            rig.reset_all();
            rig.set_param("row63", ZSTD_c_enableSeqProducerFallback, espf);
            rc(rig.cctx.c, std::ptr::null_mut(), std::ptr::null());
            rr(rig.cctx.r, std::ptr::null_mut(), std::ptr::null());
            let src = gen_class(5, 60_000, 631);
            rig.compress_and_check(&format!("row63 espf={espf}"), &src, true);
        }
        // estimate sizes from a params object carrying a (NULL) producer
        let (ec, er) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_estimateCCtxSize_usingCCtxParams");
        eqv("row63 estimateCCtxSize_usingCCtxParams", ec(params.c), er(params.r));
    }
}
