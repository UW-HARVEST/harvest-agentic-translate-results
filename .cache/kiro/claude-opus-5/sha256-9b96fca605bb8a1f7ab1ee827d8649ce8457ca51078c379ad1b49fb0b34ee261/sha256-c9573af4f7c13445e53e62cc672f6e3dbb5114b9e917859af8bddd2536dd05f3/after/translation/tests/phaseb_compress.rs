//! Phase B — valid-path differential tests for the COMPRESSION surface.
//!
//! Covers `CONFIGS.md` sections:
//!   * Compression parameters (all stable cParameters, all 9 strategies)
//!   * Experimental compression parameters
//!   * Input SHAPE special-cases
//!   * One-shot compression entry points
//!
//! Every call goes through `dlsym` on both `.so`s; outputs are compared
//! byte-for-byte. Inputs are randomized with a fixed seed.

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, u64) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;

struct Api {
    create_cctx: (FnCreate, FnCreate),
    free_cctx: (FnFree, FnFree),
    set_param: (FnSetParam, FnSetParam),
    get_param: (FnGetParam, FnGetParam),
    reset: (FnReset, FnReset),
    set_pledged: (FnSetPledged, FnSetPledged),
    compress2: (FnCompress2, FnCompress2),
    bound: (FnSizeSize, FnSizeSize),
    is_error: (FnIsError, FnIsError),
    err_code: (FnGetErrorCode, FnGetErrorCode),
    decompress: (FnDecompress, FnDecompress),
}

fn api() -> Api {
    Api {
        create_cctx: fnpair!("ZSTD_createCCtx", FnCreate),
        free_cctx: fnpair!("ZSTD_freeCCtx", FnFree),
        set_param: fnpair!("ZSTD_CCtx_setParameter", FnSetParam),
        get_param: fnpair!("ZSTD_CCtx_getParameter", FnGetParam),
        reset: fnpair!("ZSTD_CCtx_reset", FnReset),
        set_pledged: fnpair!("ZSTD_CCtx_setPledgedSrcSize", FnSetPledged),
        compress2: fnpair!("ZSTD_compress2", FnCompress2),
        bound: fnpair!("ZSTD_compressBound", FnSizeSize),
        is_error: fnpair!("ZSTD_isError", FnIsError),
        err_code: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        decompress: fnpair!("ZSTD_decompress", FnDecompress),
    }
}

/// Run `ZSTD_compress2` on both libraries with the given parameter list and
/// assert the compressed bytes are identical. Also verifies the frame
/// round-trips through the *other* implementation's decompressor.
#[track_caller]
fn diff_compress2(a: &Api, params: &[(c_int, c_int)], pledged: Option<u64>, src: &[u8], ctx: &str) {
    unsafe {
        let cap = (a.bound.0)(src.len()).max(64);
        assert_eq!(cap, (a.bound.1)(src.len()), "{ctx}: compressBound differs");

        let cctx_c = (a.create_cctx.0)();
        let cctx_r = (a.create_cctx.1)();
        assert!(!cctx_c.is_null() && !cctx_r.is_null(), "{ctx}: createCCtx null");

        for &(p, v) in params {
            let rc = (a.set_param.0)(cctx_c, p, v);
            let rr = (a.set_param.1)(cctx_r, p, v);
            assert_eq!(
                (a.is_error.0)(rc),
                (a.is_error.1)(rr),
                "{ctx}: setParameter({p},{v}) isError differs (C={rc:#x} R={rr:#x})"
            );
            assert_eq!(
                (a.err_code.0)(rc),
                (a.err_code.1)(rr),
                "{ctx}: setParameter({p},{v}) error code differs"
            );
            if (a.is_error.0)(rc) == 0 {
                assert_eq!(rc, rr, "{ctx}: setParameter({p},{v}) return differs");
                // read-back must agree too
                let mut gc: c_int = 0;
                let mut gr: c_int = 0;
                let ec = (a.get_param.0)(cctx_c, p, &mut gc);
                let er = (a.get_param.1)(cctx_r, p, &mut gr);
                assert_eq!(ec, er, "{ctx}: getParameter({p}) rc differs");
                assert_eq!(gc, gr, "{ctx}: getParameter({p}) value differs");
            }
        }

        if let Some(n) = pledged {
            let rc = (a.set_pledged.0)(cctx_c, n);
            let rr = (a.set_pledged.1)(cctx_r, n);
            assert_eq!(rc, rr, "{ctx}: setPledgedSrcSize differs");
        }

        let mut ob_c = vec![0u8; cap];
        let mut ob_r = vec![0u8; cap];
        let sp = if src.is_empty() {
            std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
        } else {
            src.as_ptr() as *const c_void
        };
        let nc = (a.compress2.0)(cctx_c, ob_c.as_mut_ptr() as *mut c_void, cap, sp, src.len());
        let nr = (a.compress2.1)(cctx_r, ob_r.as_mut_ptr() as *mut c_void, cap, sp, src.len());

        assert_eq!(
            (a.is_error.0)(nc),
            (a.is_error.1)(nr),
            "{ctx}: compress2 isError differs (C={nc:#x} R={nr:#x})"
        );
        if (a.is_error.0)(nc) != 0 {
            assert_eq!(
                (a.err_code.0)(nc),
                (a.err_code.1)(nr),
                "{ctx}: compress2 error code differs"
            );
        } else {
            assert_eq!(nc, nr, "{ctx}: compress2 size differs");
            assert_bytes_eq(&format!("{ctx}: compress2 bytes"), &ob_c[..nc], &ob_r[..nr]);

            // Cross round-trip (skip magicless / non-standard formats).
            let magicless = params
                .iter()
                .any(|&(p, v)| p == ZSTD_c_format && v == ZSTD_f_zstd1_magicless);
            if !magicless {
                let mut o = vec![0u8; src.len() + 1];
                let d = (a.decompress.1)(
                    o.as_mut_ptr() as *mut c_void,
                    src.len(),
                    ob_c.as_ptr() as *const c_void,
                    nc,
                );
                assert_eq!((a.is_error.1)(d), 0, "{ctx}: Rust failed to decode C frame");
                assert_eq!(d, src.len(), "{ctx}: rust-decode size");
                assert_bytes_eq(&format!("{ctx}: rust decodes C frame"), src, &o[..d]);

                let d = (a.decompress.0)(
                    o.as_mut_ptr() as *mut c_void,
                    src.len(),
                    ob_r.as_ptr() as *const c_void,
                    nr,
                );
                assert_eq!((a.is_error.0)(d), 0, "{ctx}: C failed to decode Rust frame");
                assert_bytes_eq(&format!("{ctx}: C decodes Rust frame"), src, &o[..d]);
            }
        }

        (a.free_cctx.0)(cctx_c);
        (a.free_cctx.1)(cctx_r);
    }
}

/// Size classes that hit the documented boundaries (see CONFIGS.md "Input
/// SHAPE special-cases"): empty, 1, < MINMATCH, small, block-size boundary,
/// multi-block, > default window for low levels.
fn size_classes() -> Vec<usize> {
    vec![
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        16,
        63,
        64,
        127,
        128,
        1024,
        4095,
        4096,
        65535,
        65536,
        131071,
        131072,
        131073,
        200_000,
        300_000,
    ]
}

// =========================== CONFIGS rows: strategies ======================

#[test]
fn b_all_strategies_all_shapes() {
    let a = api();
    let mut rng = Rng::new(0xC0FFEE);
    for strat in ZSTD_fast..=ZSTD_btultra2 {
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 3, 64, 1024, 40_000, 140_000] {
                let src = gen(shape, len, &mut rng);
                diff_compress2(
                    &a,
                    &[(ZSTD_c_strategy, strat)],
                    None,
                    &src,
                    &format!("strategy={strat} shape={shape:?} len={len}"),
                );
            }
        }
    }
}

#[test]
fn b_all_compression_levels() {
    let a = api();
    let mut rng = Rng::new(0xBEEF);
    // full stable range + negative (fast) levels
    let levels: Vec<c_int> = (-7..=22).collect();
    for lvl in levels {
        for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive, Shape::Zeros] {
            let len = 1 + rng.below(60_000);
            let src = gen(shape, len, &mut rng);
            diff_compress2(
                &a,
                &[(ZSTD_c_compressionLevel, lvl)],
                None,
                &src,
                &format!("level={lvl} shape={shape:?} len={len}"),
            );
        }
    }
}

#[test]
fn b_size_classes_all_shapes() {
    let a = api();
    let mut rng = Rng::new(0x5121);
    for &len in &size_classes() {
        for &shape in &ALL_SHAPES {
            let src = gen(shape, len, &mut rng);
            for lvl in [1, 3, 9, 19] {
                diff_compress2(
                    &a,
                    &[(ZSTD_c_compressionLevel, lvl)],
                    None,
                    &src,
                    &format!("size={len} shape={shape:?} lvl={lvl}"),
                );
            }
        }
    }
}

// ================== CONFIGS rows: window / hash / chain / search ===========

#[test]
fn b_window_hash_chain_search_minmatch() {
    let a = api();
    let mut rng = Rng::new(0x777);
    // Values chosen at the documented MIN/MAX boundaries and inside.
    let cases: Vec<Vec<(c_int, c_int)>> = vec![
        vec![(ZSTD_c_windowLog, 10)],
        vec![(ZSTD_c_windowLog, 15)],
        vec![(ZSTD_c_windowLog, 20)],
        vec![(ZSTD_c_windowLog, 27)],
        vec![(ZSTD_c_hashLog, 6)],
        vec![(ZSTD_c_hashLog, 12)],
        vec![(ZSTD_c_hashLog, 22)],
        vec![(ZSTD_c_chainLog, 6)],
        vec![(ZSTD_c_chainLog, 16)],
        vec![(ZSTD_c_chainLog, 24)],
        vec![(ZSTD_c_searchLog, 1)],
        vec![(ZSTD_c_searchLog, 5)],
        vec![(ZSTD_c_searchLog, 10)],
        vec![(ZSTD_c_minMatch, 3)],
        vec![(ZSTD_c_minMatch, 4)],
        vec![(ZSTD_c_minMatch, 5)],
        vec![(ZSTD_c_minMatch, 6)],
        vec![(ZSTD_c_minMatch, 7)],
        vec![(ZSTD_c_targetLength, 0)],
        vec![(ZSTD_c_targetLength, 1)],
        vec![(ZSTD_c_targetLength, 64)],
        vec![(ZSTD_c_targetLength, 999)],
        vec![(ZSTD_c_targetLength, 131_072)],
        vec![(ZSTD_c_targetCBlockSize, 0)],
        vec![(ZSTD_c_targetCBlockSize, 1340)],
        vec![(ZSTD_c_targetCBlockSize, 65536)],
        // interactions: window + chain + strategy
        vec![
            (ZSTD_c_strategy, ZSTD_btultra2),
            (ZSTD_c_windowLog, 18),
            (ZSTD_c_chainLog, 17),
            (ZSTD_c_hashLog, 18),
            (ZSTD_c_searchLog, 8),
            (ZSTD_c_minMatch, 3),
            (ZSTD_c_targetLength, 256),
        ],
        vec![
            (ZSTD_c_strategy, ZSTD_dfast),
            (ZSTD_c_windowLog, 12),
            (ZSTD_c_chainLog, 12),
            (ZSTD_c_hashLog, 12),
            (ZSTD_c_minMatch, 6),
        ],
        vec![
            (ZSTD_c_strategy, ZSTD_lazy2),
            (ZSTD_c_windowLog, 23),
            (ZSTD_c_searchLog, 3),
            (ZSTD_c_minMatch, 5),
        ],
    ];
    for (i, p) in cases.iter().enumerate() {
        for &shape in &[Shape::Text, Shape::Mixed, Shape::LongRange, Shape::Random] {
            for &len in &[100usize, 5000, 70_000, 150_000] {
                let src = gen(shape, len, &mut rng);
                diff_compress2(&a, p, None, &src, &format!("cparams#{i} {shape:?} len={len}"));
            }
        }
    }
}

// ============ CONFIGS rows: row match finder / LDM / block splitter ========

#[test]
fn b_row_match_finder_and_ldm() {
    let a = api();
    let mut rng = Rng::new(0x1234_5678);
    let mut cases: Vec<Vec<(c_int, c_int)>> = Vec::new();
    for urm in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for strat in [ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_btlazy2] {
            cases.push(vec![(ZSTD_c_useRowMatchFinder, urm), (ZSTD_c_strategy, strat)]);
        }
    }
    for ldm in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        cases.push(vec![(ZSTD_c_enableLongDistanceMatching, ldm)]);
        cases.push(vec![
            (ZSTD_c_enableLongDistanceMatching, ldm),
            (ZSTD_c_ldmHashLog, 10),
            (ZSTD_c_ldmMinMatch, 16),
            (ZSTD_c_ldmBucketSizeLog, 1),
            (ZSTD_c_ldmHashRateLog, 4),
        ]);
        cases.push(vec![
            (ZSTD_c_enableLongDistanceMatching, ldm),
            (ZSTD_c_ldmHashLog, 20),
            (ZSTD_c_ldmMinMatch, 64),
            (ZSTD_c_ldmBucketSizeLog, 4),
            (ZSTD_c_ldmHashRateLog, 7),
            (ZSTD_c_windowLog, 27),
        ]);
    }
    for lvl in [0, 4, 8] {
        for bsl in [0, 1, 2, 3, 4] {
            cases.push(vec![
                (ZSTD_c_blockSplitterLevel, bsl),
                (ZSTD_c_compressionLevel, lvl),
            ]);
        }
    }
    for sas in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        cases.push(vec![(ZSTD_c_splitAfterSequences, sas)]);
    }

    for (i, p) in cases.iter().enumerate() {
        for &shape in &[Shape::LongRange, Shape::Text, Shape::Mixed] {
            for &len in &[1000usize, 80_000, 260_000] {
                let src = gen(shape, len, &mut rng);
                diff_compress2(&a, p, None, &src, &format!("rmf/ldm#{i} {shape:?} len={len}"));
            }
        }
    }
}

// ================= CONFIGS rows: frame flags & literal modes ===============

#[test]
fn b_frame_flags_and_literal_modes() {
    let a = api();
    let mut rng = Rng::new(0xABCD);
    let mut cases: Vec<Vec<(c_int, c_int)>> = Vec::new();
    for cs in [0, 1] {
        for ck in [0, 1] {
            for did in [0, 1] {
                cases.push(vec![
                    (ZSTD_c_contentSizeFlag, cs),
                    (ZSTD_c_checksumFlag, ck),
                    (ZSTD_c_dictIDFlag, did),
                ]);
            }
        }
    }
    for lcm in [ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed] {
        cases.push(vec![(ZSTD_c_literalCompressionMode, lcm)]);
        for strat in [ZSTD_fast, ZSTD_lazy2, ZSTD_btultra2] {
            cases.push(vec![
                (ZSTD_c_literalCompressionMode, lcm),
                (ZSTD_c_strategy, strat),
            ]);
        }
    }
    for f in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
        cases.push(vec![(ZSTD_c_format, f)]);
        cases.push(vec![(ZSTD_c_format, f), (ZSTD_c_checksumFlag, 1)]);
    }
    for v in [0, 1] {
        cases.push(vec![(ZSTD_c_forceMaxWindow, v)]);
        cases.push(vec![(ZSTD_c_deterministicRefPrefix, v)]);
        cases.push(vec![(ZSTD_c_rsyncable, v)]);
    }
    for mbs in [0, 1024, 65536, 131_072] {
        cases.push(vec![(ZSTD_c_maxBlockSize, mbs)]);
    }
    for rr in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        cases.push(vec![(ZSTD_c_repcodeResolution, rr)]);
    }
    for h in [0, 1, 1000, 100_000, 10_000_000] {
        cases.push(vec![(ZSTD_c_srcSizeHint, h)]);
    }

    for (i, p) in cases.iter().enumerate() {
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 5, 3000, 150_000] {
                let src = gen(shape, len, &mut rng);
                diff_compress2(&a, p, None, &src, &format!("flags#{i} {shape:?} len={len}"));
            }
        }
    }
}

// ===================== CONFIGS rows: pledgedSrcSize =======================

#[test]
fn b_pledged_src_size() {
    let a = api();
    let mut rng = Rng::new(0x9999);
    for &shape in &[Shape::Text, Shape::Random, Shape::Zeros] {
        for &len in &[0usize, 1, 1000, 140_000] {
            let src = gen(shape, len, &mut rng);
            for pledged in [Some(0u64), Some(len as u64), Some(u64::MAX), None] {
                for cs in [0, 1] {
                    // pledged=0 with non-empty src is a legitimate "unknown" in the C API
                    diff_compress2(
                        &a,
                        &[(ZSTD_c_contentSizeFlag, cs), (ZSTD_c_compressionLevel, 3)],
                        pledged,
                        &src,
                        &format!("pledged={pledged:?} cs={cs} {shape:?} len={len}"),
                    );
                }
            }
        }
    }
}

// ============== CONFIGS rows: one-shot entry points (all of them) ==========

#[test]
fn b_oneshot_entry_points() {
    let a = api();
    type FnCompressCCtx =
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
    type FnCompressAdvanced = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        size_t,
        *const c_void,
        size_t,
        *const c_void,
        size_t,
        ZSTD_parameters,
    ) -> size_t;
    type FnCompressUsingDict = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        size_t,
        *const c_void,
        size_t,
        *const c_void,
        size_t,
        c_int,
    ) -> size_t;
    type FnGetParams = unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_parameters;

    let (c_cctx, r_cctx) = fnpair!("ZSTD_compressCCtx", FnCompressCCtx);
    let (c_adv, r_adv) = fnpair!("ZSTD_compress_advanced", FnCompressAdvanced);
    let (c_ud, r_ud) = fnpair!("ZSTD_compress_usingDict", FnCompressUsingDict);
    let (c_gp, r_gp) = fnpair!("ZSTD_getParams", FnGetParams);
    let (c_cc, r_cc) = fnpair!("ZSTD_createCCtx", FnCreate);
    let (c_fc, r_fc) = fnpair!("ZSTD_freeCCtx", FnFree);
    let (c_one, r_one) = fnpair!("ZSTD_compress", FnCompress);

    let mut rng = Rng::new(0x2468);
    unsafe {
        let cc = c_cc();
        let rc = r_cc();
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 100, 5000, 140_000] {
                let src = gen(shape, len, &mut rng);
                let dict = gen(Shape::Text, 2048, &mut rng);
                let cap = (a.bound.0)(len).max(64);
                let mut o1 = vec![0u8; cap];
                let mut o2 = vec![0u8; cap];
                let sp = src.as_ptr() as *const c_void;
                for lvl in [1, 5, 12, 19] {
                    let tag = format!("oneshot {shape:?} len={len} lvl={lvl}");
                    // ZSTD_compress
                    let n1 = c_one(o1.as_mut_ptr() as *mut c_void, cap, sp, len, lvl);
                    let n2 = r_one(o2.as_mut_ptr() as *mut c_void, cap, sp, len, lvl);
                    assert_eq!(n1, n2, "{tag}: ZSTD_compress");
                    assert_bytes_eq(&format!("{tag} ZSTD_compress"), &o1[..n1], &o2[..n2]);
                    // ZSTD_compressCCtx
                    let n1 = c_cctx(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, len, lvl);
                    let n2 = r_cctx(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, len, lvl);
                    assert_eq!(n1, n2, "{tag}: compressCCtx");
                    assert_bytes_eq(&format!("{tag} compressCCtx"), &o1[..n1], &o2[..n2]);
                    // ZSTD_getParams must agree, then ZSTD_compress_advanced
                    let p1 = c_gp(lvl, len as u64, dict.len());
                    let p2 = r_gp(lvl, len as u64, dict.len());
                    assert_eq!(p1, p2, "{tag}: ZSTD_getParams");
                    let n1 = c_adv(
                        cc,
                        o1.as_mut_ptr() as *mut c_void,
                        cap,
                        sp,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        p1,
                    );
                    let n2 = r_adv(
                        rc,
                        o2.as_mut_ptr() as *mut c_void,
                        cap,
                        sp,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        p2,
                    );
                    assert_eq!(n1, n2, "{tag}: compress_advanced");
                    if (a.is_error.0)(n1) == 0 {
                        assert_bytes_eq(&format!("{tag} compress_advanced"), &o1[..n1], &o2[..n2]);
                    }
                    // ZSTD_compress_usingDict
                    let n1 = c_ud(
                        cc,
                        o1.as_mut_ptr() as *mut c_void,
                        cap,
                        sp,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    );
                    let n2 = r_ud(
                        rc,
                        o2.as_mut_ptr() as *mut c_void,
                        cap,
                        sp,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    );
                    assert_eq!(n1, n2, "{tag}: compress_usingDict");
                    if (a.is_error.0)(n1) == 0 {
                        assert_bytes_eq(&format!("{tag} compress_usingDict"), &o1[..n1], &o2[..n2]);
                    }
                }
            }
        }
        c_fc(cc);
        r_fc(rc);
    }
}

// ============== CONFIGS rows: CCtxParams object (separate API) =============

#[test]
fn b_cctx_params_object() {
    type FnCreateP = unsafe extern "C" fn() -> *mut c_void;
    type FnFreeP = unsafe extern "C" fn(*mut c_void) -> size_t;
    type FnPSet = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
    type FnPGet = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> size_t;
    type FnPInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
    type FnPReset = unsafe extern "C" fn(*mut c_void) -> size_t;
    type FnSetParams = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;

    let a = api();
    let (c_cp, r_cp) = fnpair!("ZSTD_createCCtxParams", FnCreateP);
    let (c_fp, r_fp) = fnpair!("ZSTD_freeCCtxParams", FnFreeP);
    let (c_ps, r_ps) = fnpair!("ZSTD_CCtxParams_setParameter", FnPSet);
    let (c_pg, r_pg) = fnpair!("ZSTD_CCtxParams_getParameter", FnPGet);
    let (c_pi, r_pi) = fnpair!("ZSTD_CCtxParams_init", FnPInit);
    let (c_pr, r_pr) = fnpair!("ZSTD_CCtxParams_reset", FnPReset);
    let (c_sp, r_sp) = fnpair!("ZSTD_CCtx_setParametersUsingCCtxParams", FnSetParams);
    type FnCompress2 =
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
    let (c_c2, r_c2) = fnpair!("ZSTD_compress2", FnCompress2);

    let mut rng = Rng::new(0x13579);
    unsafe {
        for lvl in [1, 6, 17] {
            for extra in [
                vec![],
                vec![(ZSTD_c_checksumFlag, 1), (ZSTD_c_strategy, ZSTD_btopt)],
                vec![(ZSTD_c_windowLog, 17), (ZSTD_c_enableLongDistanceMatching, 1)],
                vec![(ZSTD_c_literalCompressionMode, ZSTD_lcm_uncompressed)],
            ] {
                let pc = c_cp();
                let pr = r_cp();
                assert!(!pc.is_null() && !pr.is_null());
                assert_eq!(c_pr(pc), r_pr(pr), "CCtxParams_reset");
                assert_eq!(c_pi(pc, lvl), r_pi(pr, lvl), "CCtxParams_init({lvl})");
                for &(p, v) in &extra {
                    assert_eq!(c_ps(pc, p, v), r_ps(pr, p, v), "CCtxParams_set({p},{v})");
                    let mut gc = 0;
                    let mut gr = 0;
                    assert_eq!(c_pg(pc, p, &mut gc), r_pg(pr, p, &mut gr), "get rc");
                    assert_eq!(gc, gr, "CCtxParams_get({p})");
                }
                let cc = (a.create_cctx.0)();
                let rc2 = (a.create_cctx.1)();
                assert_eq!(c_sp(cc, pc), r_sp(rc2, pr), "setParametersUsingCCtxParams");
                for &shape in &[Shape::Text, Shape::Random, Shape::Mixed] {
                    let len = 1 + rng.below(80_000);
                    let src = gen(shape, len, &mut rng);
                    let cap = (a.bound.0)(len).max(64);
                    let mut o1 = vec![0u8; cap];
                    let mut o2 = vec![0u8; cap];
                    let n1 = c_c2(
                        cc,
                        o1.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                    );
                    let n2 = r_c2(
                        rc2,
                        o2.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                    );
                    assert_eq!(n1, n2, "params-object compress2 {shape:?} len={len}");
                    assert_bytes_eq("params-object compress2", &o1[..n1], &o2[..n2]);
                }
                (a.free_cctx.0)(cc);
                (a.free_cctx.1)(rc2);
                c_fp(pc);
                r_fp(pr);
            }
        }
    }
}

// ============== CONFIGS: bounds + defaults for every parameter =============

#[test]
fn b_all_parameter_bounds_and_defaults() {
    let (cb, rb) = fnpair!("ZSTD_cParam_getBounds", FnBounds);
    let (cdb, rdb) = fnpair!("ZSTD_dParam_getBounds", FnBounds);
    let all_c = [
        ZSTD_c_compressionLevel,
        ZSTD_c_windowLog,
        ZSTD_c_hashLog,
        ZSTD_c_chainLog,
        ZSTD_c_searchLog,
        ZSTD_c_minMatch,
        ZSTD_c_targetLength,
        ZSTD_c_strategy,
        ZSTD_c_targetCBlockSize,
        ZSTD_c_enableLongDistanceMatching,
        ZSTD_c_ldmHashLog,
        ZSTD_c_ldmMinMatch,
        ZSTD_c_ldmBucketSizeLog,
        ZSTD_c_ldmHashRateLog,
        ZSTD_c_contentSizeFlag,
        ZSTD_c_checksumFlag,
        ZSTD_c_dictIDFlag,
        ZSTD_c_nbWorkers,
        ZSTD_c_jobSize,
        ZSTD_c_overlapLog,
        ZSTD_c_rsyncable,
        ZSTD_c_format,
        ZSTD_c_forceMaxWindow,
        ZSTD_c_forceAttachDict,
        ZSTD_c_literalCompressionMode,
        ZSTD_c_srcSizeHint,
        ZSTD_c_enableDedicatedDictSearch,
        ZSTD_c_stableInBuffer,
        ZSTD_c_stableOutBuffer,
        ZSTD_c_blockDelimiters,
        ZSTD_c_validateSequences,
        ZSTD_c_splitAfterSequences,
        ZSTD_c_useRowMatchFinder,
        ZSTD_c_deterministicRefPrefix,
        ZSTD_c_prefetchCDictTables,
        ZSTD_c_enableSeqProducerFallback,
        ZSTD_c_maxBlockSize,
        ZSTD_c_repcodeResolution,
        ZSTD_c_blockSplitterLevel,
    ];
    unsafe {
        for p in all_c {
            assert_eq!(cb(p), rb(p), "cParam_getBounds({p})");
        }
        for p in [
            ZSTD_d_windowLogMax,
            ZSTD_d_format,
            ZSTD_d_stableOutBuffer,
            ZSTD_d_forceIgnoreChecksum,
            ZSTD_d_refMultipleDDicts,
            ZSTD_d_disableHuffmanAssembly,
            ZSTD_d_maxBlockSize,
        ] {
            assert_eq!(cdb(p), rdb(p), "dParam_getBounds({p})");
        }
    }

    // Also: every parameter set to every value inside its own bounds must be
    // accepted identically and read back identically.
    let a = api();
    let mut rng = Rng::new(0xFEED);
    unsafe {
        for p in all_c {
            let b = cb(p);
            if (a.is_error.0)(b.error) != 0 {
                continue;
            }
            let mut vals = vec![b.lowerBound, b.upperBound, 0];
            for _ in 0..6 {
                vals.push(rng.range(b.lowerBound, b.upperBound));
            }
            for v in vals {
                let src = gen(Shape::Mixed, 20_000, &mut rng);
                diff_compress2(&a, &[(p, v)], None, &src, &format!("param {p} = {v}"));
            }
        }
    }
}

// ================= CONFIGS: getFrameProgression / sizeof ==================

#[test]
fn b_cctx_introspection() {
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    struct ZSTD_frameProgression {
        ingested: c_ulonglong_,
        consumed: c_ulonglong_,
        produced: c_ulonglong_,
        flushed: c_ulonglong_,
        currentJobID: c_uint,
        nbActiveWorkers: c_uint,
    }
    type c_ulonglong_ = u64;
    type FnProg = unsafe extern "C" fn(*const c_void) -> ZSTD_frameProgression;
    type FnSizeof = unsafe extern "C" fn(*const c_void) -> size_t;
    type FnToFlush = unsafe extern "C" fn(*const c_void) -> size_t;

    let a = api();
    let (cp, rp) = fnpair!("ZSTD_getFrameProgression", FnProg);
    let (cs, rs) = fnpair!("ZSTD_sizeof_CCtx", FnSizeof);
    let (ct, rt) = fnpair!("ZSTD_toFlushNow", FnToFlush);
    unsafe {
        for lvl in [1, 3, 9, 19] {
            let cc = (a.create_cctx.0)();
            let rc = (a.create_cctx.1)();
            assert_eq!((a.set_param.0)(cc, ZSTD_c_compressionLevel, lvl), (a.set_param.1)(rc, ZSTD_c_compressionLevel, lvl));
            assert_eq!(cp(cc), rp(rc), "frameProgression fresh lvl={lvl}");
            assert_eq!(ct(cc), rt(rc), "toFlushNow lvl={lvl}");
            let mut rng = Rng::new(lvl as u64 + 1);
            let src = gen(Shape::Text, 50_000, &mut rng);
            let cap = (a.bound.0)(src.len());
            let mut o1 = vec![0u8; cap];
            let mut o2 = vec![0u8; cap];
            let n1 = (a.compress2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let n2 = (a.compress2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            assert_eq!(n1, n2);
            assert_eq!(cp(cc), rp(rc), "frameProgression after compress lvl={lvl}");
            // sizeof must be identical too — the workspace layout is part of the ABI
            assert_eq!(cs(cc), rs(rc), "sizeof_CCtx lvl={lvl}");
            (a.free_cctx.0)(cc);
            (a.free_cctx.1)(rc);
        }
    }
}
