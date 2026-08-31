//! Phase B/C: the advanced parameter surface.
//!
//! * `ZSTD_cParam_getBounds` / `ZSTD_dParam_getBounds` for EVERY parameter id,
//!   including ids with no valid variant (out-of-range enum values across FFI).
//! * `ZSTD_CCtx_setParameter` / `ZSTD_CCtxParams_setParameter` accept/reject
//!   parity at, inside and one step past each documented bound.
//! * `ZSTD_compress2` output equality for randomized valid parameter sets, so
//!   the *effect* of each accepted parameter is verified, not just its return.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type CCtxParams = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_cParamBounds = unsafe extern "C" fn(i32) -> ZSTD_bounds;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_getParam = unsafe extern "C" fn(CCtx, i32, *mut i32) -> usize;
type Fn_setPledged = unsafe extern "C" fn(CCtx, u64) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_compress2 = unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_decompressDCtx =
    unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_dSetParam = unsafe extern "C" fn(DCtx, i32, i32) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;

type Fn_createParams = unsafe extern "C" fn() -> CCtxParams;
type Fn_freeParams = unsafe extern "C" fn(CCtxParams) -> usize;
type Fn_paramsSet = unsafe extern "C" fn(CCtxParams, i32, i32) -> usize;
type Fn_paramsGet = unsafe extern "C" fn(CCtxParams, i32, *mut i32) -> usize;
type Fn_paramsInit = unsafe extern "C" fn(CCtxParams, i32) -> usize;
type Fn_paramsReset = unsafe extern "C" fn(CCtxParams) -> usize;
type Fn_setParamsUsingCCtxParams = unsafe extern "C" fn(CCtx, CCtxParams) -> usize;

/// Every parameter id the public header names, plus deliberately invalid ids.
fn all_cparam_ids() -> Vec<i32> {
    let mut v = vec![
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
        ZSTD_c_experimentalParam6,
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
    // out-of-range / no-valid-variant enum values that C must still handle
    v.extend([
        i32::MIN, -1, 0, 1, 2, 9, 11, 99, 108, 129, 131, 159, 165, 199, 203, 399, 403, 499,
        501, 999, 1018, 1019, 5000, i32::MAX,
    ]);
    v
}

fn all_dparam_ids() -> Vec<i32> {
    let mut v = vec![
        ZSTD_d_windowLogMax,
        ZSTD_d_format,
        ZSTD_d_stableOutBuffer,
        ZSTD_d_forceIgnoreChecksum,
        ZSTD_d_refMultipleDDicts,
        ZSTD_d_disableHuffmanAssembly,
        ZSTD_d_maxBlockSize,
    ];
    v.extend([i32::MIN, -1, 0, 1, 99, 101, 999, 1006, 1007, 12345, i32::MAX]);
    v
}

/// Phase C + Phase B: bounds for every id must match exactly, including the
/// `error` field for ids that are not valid parameters at all.
#[test]
fn cparam_dparam_bounds_match_including_invalid_ids() {
    let i = impls();
    let (c_cb, r_cb) = i.pair::<Fn_cParamBounds>("ZSTD_cParam_getBounds");
    let (c_db, r_db) = i.pair::<Fn_cParamBounds>("ZSTD_dParam_getBounds");

    for id in all_cparam_ids() {
        let (a, b) = unsafe { (c_cb(id), r_cb(id)) };
        assert_eq_dbg(&format!("ZSTD_cParam_getBounds({id})"), a, b);
    }
    for id in all_dparam_ids() {
        let (a, b) = unsafe { (c_db(id), r_db(id)) };
        assert_eq_dbg(&format!("ZSTD_dParam_getBounds({id})"), a, b);
    }
}

/// For every parameter, probe values at the bound, inside it, and one step past
/// it in both directions. Accept/reject and the resulting *stored* value (via
/// getParameter) must match. This is the bulk of the ERRORS.md parameter rows.
#[test]
fn setparameter_accept_reject_and_readback_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_get, r_get) = i.pair::<Fn_getParam>("ZSTD_CCtx_getParameter");
    let (c_cb, _) = i.pair::<Fn_cParamBounds>("ZSTD_cParam_getBounds");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    assert!(!cc.is_null() && !rc.is_null());

    for id in all_cparam_ids() {
        let b = unsafe { c_cb(id) };
        // probe set: bounds +- 1, extremes, and a few interior points
        let mut probes: Vec<i32> = vec![
            i32::MIN,
            i32::MIN + 1,
            -1,
            0,
            1,
            i32::MAX - 1,
            i32::MAX,
        ];
        if b.error == 0 {
            for base in [b.lower_bound, b.upper_bound] {
                for d in [-2i32, -1, 0, 1, 2] {
                    probes.push(base.saturating_add(d));
                }
            }
            let mid = (b.lower_bound as i64 + b.upper_bound as i64) / 2;
            probes.push(mid as i32);
        }

        for v in probes {
            // fresh parameter state each probe so results are independent
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let (a, bb) = unsafe { (c_set(cc, id, v), r_set(rc, id, v)) };
            let tag = format!("ZSTD_CCtx_setParameter(id={id}, value={v})");
            assert_eq_dbg(&tag, a, bb);

            // read back — must agree whether or not the set succeeded
            let mut ov1 = -12345i32;
            let mut ov2 = -12345i32;
            let (g1, g2) = unsafe { (c_get(cc, id, &mut ov1), r_get(rc, id, &mut ov2)) };
            assert_eq_dbg(&format!("{tag} / getParameter rc"), g1, g2);
            assert_eq_dbg(&format!("{tag} / getParameter value"), ov1, ov2);
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// Same sweep against the standalone `ZSTD_CCtx_params` object, which has its
/// own validation path in the C (`ZSTD_CCtxParams_setParameter`).
#[test]
fn cctxparams_accept_reject_and_readback_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createParams>("ZSTD_createCCtxParams");
    let (c_free, r_free) = i.pair::<Fn_freeParams>("ZSTD_freeCCtxParams");
    let (c_set, r_set) = i.pair::<Fn_paramsSet>("ZSTD_CCtxParams_setParameter");
    let (c_get, r_get) = i.pair::<Fn_paramsGet>("ZSTD_CCtxParams_getParameter");
    let (c_init, r_init) = i.pair::<Fn_paramsInit>("ZSTD_CCtxParams_init");
    let (c_rst, r_rst) = i.pair::<Fn_paramsReset>("ZSTD_CCtxParams_reset");
    let (c_cb, _) = i.pair::<Fn_cParamBounds>("ZSTD_cParam_getBounds");

    let cp = unsafe { c_new() };
    let rp = unsafe { r_new() };
    assert!(!cp.is_null() && !rp.is_null());

    // ZSTD_CCtxParams_init over a level sweep (incl. invalid levels)
    for lvl in [-1000i32, -1, 0, 1, 3, 19, 22, 23, 100] {
        let (a, b) = unsafe { (c_init(cp, lvl), r_init(rp, lvl)) };
        assert_eq_dbg(&format!("ZSTD_CCtxParams_init({lvl})"), a, b);
    }

    for id in all_cparam_ids() {
        let b = unsafe { c_cb(id) };
        let mut probes: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX];
        if b.error == 0 {
            for base in [b.lower_bound, b.upper_bound] {
                for d in [-1i32, 0, 1] {
                    probes.push(base.saturating_add(d));
                }
            }
        }
        for v in probes {
            unsafe {
                c_rst(cp);
                r_rst(rp);
            }
            let (a, bb) = unsafe { (c_set(cp, id, v), r_set(rp, id, v)) };
            let tag = format!("ZSTD_CCtxParams_setParameter(id={id}, value={v})");
            assert_eq_dbg(&tag, a, bb);

            let mut o1 = -999i32;
            let mut o2 = -999i32;
            let (g1, g2) = unsafe { (c_get(cp, id, &mut o1), r_get(rp, id, &mut o2)) };
            assert_eq_dbg(&format!("{tag} / get rc"), g1, g2);
            assert_eq_dbg(&format!("{tag} / get value"), o1, o2);
        }
    }

    unsafe {
        c_free(cp);
        r_free(rp);
    }
}

/// A named, reproducible parameter configuration applied through the low-level
/// `ZSTD_CCtx_setParameter` path, then run end-to-end with `ZSTD_compress2`.
struct Cfg {
    name: &'static str,
    params: Vec<(i32, i32)>,
}

fn configurations() -> Vec<Cfg> {
    let mut v: Vec<Cfg> = Vec::new();

    // one row per strategy — each selects a different match finder source file
    for &s in &ALL_STRATEGIES {
        v.push(Cfg {
            name: "strategy",
            params: vec![(ZSTD_c_strategy, s), (ZSTD_c_compressionLevel, 5)],
        });
    }
    // checksum / contentSize / dictID flag cross product
    for &ck in &[0, 1] {
        for &cs in &[0, 1] {
            for &di in &[0, 1] {
                v.push(Cfg {
                    name: "frame-flags",
                    params: vec![
                        (ZSTD_c_checksumFlag, ck),
                        (ZSTD_c_contentSizeFlag, cs),
                        (ZSTD_c_dictIDFlag, di),
                    ],
                });
            }
        }
    }
    // magicless format x checksum
    for &f in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
        for &ck in &[0, 1] {
            v.push(Cfg {
                name: "format",
                params: vec![(ZSTD_c_format, f), (ZSTD_c_checksumFlag, ck)],
            });
        }
    }
    // long distance matching, with explicit ldm tuning
    for &ldm in &[0, 1] {
        v.push(Cfg {
            name: "ldm",
            params: vec![
                (ZSTD_c_enableLongDistanceMatching, ldm),
                (ZSTD_c_windowLog, 20),
            ],
        });
    }
    v.push(Cfg {
        name: "ldm-tuned",
        params: vec![
            (ZSTD_c_enableLongDistanceMatching, 1),
            (ZSTD_c_ldmHashLog, 17),
            (ZSTD_c_ldmMinMatch, 32),
            (ZSTD_c_ldmBucketSizeLog, 3),
            (ZSTD_c_ldmHashRateLog, 4),
            (ZSTD_c_windowLog, 21),
        ],
    });
    // literal compression modes
    for &m in &[ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed] {
        v.push(Cfg {
            name: "literalCompressionMode",
            params: vec![(ZSTD_c_literalCompressionMode, m)],
        });
    }
    // row match finder toggle (auto/enable/disable) x strategy in its valid range
    for &ps in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &s in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
            v.push(Cfg {
                name: "useRowMatchFinder",
                params: vec![(ZSTD_c_useRowMatchFinder, ps), (ZSTD_c_strategy, s)],
            });
        }
    }
    // block splitter / splitAfterSequences / blockSplitterLevel
    for &ps in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        v.push(Cfg {
            name: "splitAfterSequences",
            params: vec![(ZSTD_c_splitAfterSequences, ps), (ZSTD_c_compressionLevel, 9)],
        });
    }
    for lvl in 0..=6 {
        v.push(Cfg {
            name: "blockSplitterLevel",
            params: vec![(ZSTD_c_blockSplitterLevel, lvl)],
        });
    }
    // targetCBlockSize / maxBlockSize
    for &t in &[0, 1340, 2048, 65536, 131_072] {
        v.push(Cfg {
            name: "targetCBlockSize",
            params: vec![(ZSTD_c_targetCBlockSize, t)],
        });
    }
    for &m in &[0, 1024, 8192, 65536, 131_072] {
        v.push(Cfg {
            name: "maxBlockSize",
            params: vec![(ZSTD_c_maxBlockSize, m)],
        });
    }
    // explicit window/hash/chain/search/minMatch/targetLength tuning
    v.push(Cfg {
        name: "manual-tuning-fast",
        params: vec![
            (ZSTD_c_strategy, ZSTD_fast),
            (ZSTD_c_windowLog, 17),
            (ZSTD_c_hashLog, 16),
            (ZSTD_c_searchLog, 1),
            (ZSTD_c_minMatch, 5),
            (ZSTD_c_targetLength, 0),
        ],
    });
    v.push(Cfg {
        name: "manual-tuning-btultra2",
        params: vec![
            (ZSTD_c_strategy, ZSTD_btultra2),
            (ZSTD_c_windowLog, 18),
            (ZSTD_c_hashLog, 17),
            (ZSTD_c_chainLog, 17),
            (ZSTD_c_searchLog, 6),
            (ZSTD_c_minMatch, 3),
            (ZSTD_c_targetLength, 999),
        ],
    });
    v.push(Cfg {
        name: "minMatch-sweep-3",
        params: vec![(ZSTD_c_strategy, ZSTD_lazy2), (ZSTD_c_minMatch, 3)],
    });
    v.push(Cfg {
        name: "minMatch-sweep-7",
        params: vec![(ZSTD_c_strategy, ZSTD_fast), (ZSTD_c_minMatch, 7)],
    });
    // srcSizeHint / forceMaxWindow / deterministicRefPrefix / repcodeResolution
    for &h in &[0, 1, 1000, 1 << 20] {
        v.push(Cfg {
            name: "srcSizeHint",
            params: vec![(ZSTD_c_srcSizeHint, h)],
        });
    }
    for &f in &[0, 1] {
        v.push(Cfg {
            name: "forceMaxWindow",
            params: vec![(ZSTD_c_forceMaxWindow, f)],
        });
    }
    for &d in &[0, 1] {
        v.push(Cfg {
            name: "deterministicRefPrefix",
            params: vec![(ZSTD_c_deterministicRefPrefix, d)],
        });
    }
    for &rr in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        v.push(Cfg {
            name: "repcodeResolution",
            params: vec![(ZSTD_c_repcodeResolution, rr)],
        });
    }
    for &r in &[0, 1] {
        v.push(Cfg {
            name: "rsyncable",
            params: vec![(ZSTD_c_rsyncable, r), (ZSTD_c_windowLog, 20)],
        });
    }
    // negative / fast levels and the extremes
    for &lvl in &[-131_072, -5000, -100, -10, -3, -1, 0, 1, 19, 22] {
        v.push(Cfg {
            name: "level",
            params: vec![(ZSTD_c_compressionLevel, lvl)],
        });
    }
    // window log extremes (valid range)
    for &wl in &[10, 11, 15, 20, 27] {
        v.push(Cfg {
            name: "windowLog",
            params: vec![(ZSTD_c_windowLog, wl)],
        });
    }
    v
}

/// Phase B core: for each configuration, drive `ZSTD_compress2` through the
/// low-level cctx API on many randomized inputs and require byte-identical
/// frames, then require the frame to decode back to the input in both libs.
#[test]
fn compress2_configurations_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<unsafe extern "C" fn(usize) -> u32>("ZSTD_isError");

    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (cd_dec, rd_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let (cd_set, rd_set) = i.pair::<Fn_dSetParam>("ZSTD_DCtx_setParameter");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { cd_new() };
    let rd = unsafe { rd_new() };

    let mut rng = Rng::new(0x5EED_1234);

    for cfg in configurations() {
        // several randomized inputs per configuration row
        for trial in 0..6 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = match trial {
                0 => 0,
                1 => 1,
                2 => rng.range(2, 300),
                3 => rng.range(300, 20_000),
                4 => rng.range(120_000, 140_000),
                _ => rng.range(200_000, 400_000),
            };
            let src = gen_shape(shape, len, &mut rng);

            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }

            let mut magicless = false;
            let mut skip = false;
            for &(id, val) in &cfg.params {
                let (a, b) = unsafe { (c_set(cc, id, val), r_set(rc, id, val)) };
                assert_eq_dbg(
                    &format!("[{}] setParameter({id},{val})", cfg.name),
                    a,
                    b,
                );
                if unsafe { c_isE(a) } != 0 {
                    // both rejected identically; nothing to compress for this row
                    skip = true;
                }
                if id == ZSTD_c_format && val == ZSTD_f_zstd1_magicless {
                    magicless = true;
                }
            }
            if skip {
                continue;
            }

            let cap = unsafe { c_bound(len) } + 64;
            let mut cb = vec![0xA5u8; cap];
            let mut rb = vec![0x5Au8; cap];
            let cn = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let rn = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };

            let tag = format!(
                "[{}] params={:?} shape={shape:?} len={len}",
                cfg.name, cfg.params
            );
            assert_eq_dbg(&tag, cn, rn);
            if unsafe { c_isE(cn) } != 0 {
                continue; // identical rejection, verified above
            }
            assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);

            // round trip through both decoders (magicless frames need the dctx flag)
            let mut d1 = vec![0u8; len + 64];
            let mut d2 = vec![0u8; len + 64];
            unsafe {
                cd_set(cd, ZSTD_d_format, if magicless { 1 } else { 0 });
                rd_set(rd, ZSTD_d_format, if magicless { 1 } else { 0 });
                let a = cd_dec(cd, d1.as_mut_ptr(), d1.len(), cb.as_ptr(), cn);
                let b = rd_dec(rd, d2.as_mut_ptr(), d2.len(), rb.as_ptr(), rn);
                assert_eq_dbg(&format!("{tag} / decode rc"), a, b);
                assert_eq_dbg(&format!("{tag} / decode len"), a, len);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d1[..a]);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &d2[..b]);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        cd_free(cd);
        rd_free(rd);
    }
}

/// `ZSTD_CCtx_setParametersUsingCCtxParams` — the params-object path into a cctx
/// must produce the same frames as setting the parameters directly.
#[test]
fn set_parameters_using_cctxparams_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_pnew, r_pnew) = i.pair::<Fn_createParams>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) = i.pair::<Fn_freeParams>("ZSTD_freeCCtxParams");
    let (c_pset, r_pset) = i.pair::<Fn_paramsSet>("ZSTD_CCtxParams_setParameter");
    let (c_apply, r_apply) =
        i.pair::<Fn_setParamsUsingCCtxParams>("ZSTD_CCtx_setParametersUsingCCtxParams");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cp = unsafe { c_pnew() };
    let rp = unsafe { r_pnew() };

    let mut rng = Rng::new(0xABCD_0001);
    for cfg in configurations() {
        for _ in 0..3 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = rng.range(0, 40_000);
            let src = gen_shape(shape, len, &mut rng);

            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let mut bad = false;
            for &(id, val) in &cfg.params {
                let (a, b) = unsafe { (c_pset(cp, id, val), r_pset(rp, id, val)) };
                assert_eq_dbg(&format!("params set({id},{val})"), a, b);
                if a > usize::MAX - 200 {
                    bad = true;
                }
            }
            let (a, b) = unsafe { (c_apply(cc, cp), r_apply(rc, rp)) };
            assert_eq_dbg("ZSTD_CCtx_setParametersUsingCCtxParams", a, b);
            if bad || a > usize::MAX - 200 {
                continue;
            }

            let cap = unsafe { c_bound(len) } + 64;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let cn = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let rn = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("[{}] usingCCtxParams len={len} shape={shape:?}", cfg.name);
            assert_eq_dbg(&tag, cn, rn);
            if cn <= usize::MAX - 200 {
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            }
        }
    }

    unsafe {
        c_pfree(cp);
        r_pfree(rp);
        c_free(cc);
        r_free(rc);
    }
}

/// `ZSTD_CCtx_setPledgedSrcSize` interacts with the frame header (content size
/// field) and is validated against the actual input length. Cover matching,
/// mismatching and unknown pledges — plus out-of-range reset directives.
#[test]
fn pledged_src_size_and_reset_directives() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_pl, r_pl) = i.pair::<Fn_setPledged>("ZSTD_CCtx_setPledgedSrcSize");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0x9911);

    // every reset directive, including out-of-range enum values
    for d in [-3i32, -1, 0, 1, 2, 3, 4, 7, i32::MAX, i32::MIN] {
        let (a, b) = unsafe { (c_rst(cc, d), r_rst(rc, d)) };
        assert_eq_dbg(&format!("ZSTD_CCtx_reset(directive={d})"), a, b);
    }

    for len in [0usize, 1, 100, 5000, 200_000] {
        let src = gen_shape(Shape::SkewedText, len, &mut rng);
        for pledge in [
            0u64,
            1,
            len as u64,
            len as u64 + 1,
            len.saturating_sub(1) as u64,
            1 << 40,
            ZSTD_CONTENTSIZE_UNKNOWN,
        ] {
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let (a, b) = unsafe { (c_pl(cc, pledge), r_pl(rc, pledge)) };
            assert_eq_dbg(&format!("setPledgedSrcSize({pledge})"), a, b);

            let cap = unsafe { c_bound(len) } + 64;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let cn = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let rn = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("compress2 pledge={pledge} len={len}");
            assert_eq_dbg(&tag, cn, rn);
            if cn <= usize::MAX - 200 {
                assert_bytes_eq(&tag, &cb[..cn], &rb[..rn]);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// Undersized destination buffers must produce the same `dstSize_tooSmall`
/// behaviour at every capacity from 0 up to the exact frame size.
#[test]
fn compress2_dst_too_small_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_c2, r_c2) = i.pair::<Fn_compress2>("ZSTD_compress2");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_cd, r_cd) = i.pair::<unsafe extern "C" fn(usize) -> i32>("ZSTD_getErrorCode");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0x4242);

    for &shape in &[Shape::Random, Shape::Constant, Shape::SkewedText] {
        let len = 3000;
        let src = gen_shape(shape, len, &mut rng);
        let full = {
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
            }
            let cap = unsafe { c_bound(len) };
            let mut b = vec![0u8; cap];
            unsafe { c_c2(cc, b.as_mut_ptr(), cap, src.as_ptr(), len) }
        };

        for cap in 0..=full {
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            let mut cb = vec![0u8; cap.max(1)];
            let mut rb = vec![0u8; cap.max(1)];
            let a = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let b = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("compress2 shape={shape:?} dstCapacity={cap} (full={full})");
            assert_eq_dbg(&tag, a, b);
            unsafe { assert_eq_dbg(&format!("{tag} errcode"), c_cd(a), r_cd(b)) };
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}
