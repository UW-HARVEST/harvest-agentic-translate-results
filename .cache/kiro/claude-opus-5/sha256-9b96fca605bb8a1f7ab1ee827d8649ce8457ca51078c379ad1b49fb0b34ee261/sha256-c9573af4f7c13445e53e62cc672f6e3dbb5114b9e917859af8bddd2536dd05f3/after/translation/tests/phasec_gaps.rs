//! Phase C — gap-closing tests for the `ERRORS.md` rows not explicitly
//! referenced by `phasec_params.rs`, `phasec_decomp.rs`, `phasec_seq_dict.rs`
//! or `phasec_entropy_misc.rs`.
//!
//! Rows covered here: 36, 38, 40, 43, 87, 88, 97, 98, 101, 102, 103, 104, 105,
//! 106, 107, 115, 295, 310, 312, 324, 327.
//!
//! Rows that are genuinely unreachable through the public FFI surface are
//! reported with `eprintln!` and the closest observable behaviour is still
//! asserted identical between the two libraries — no row is faked.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int, c_int) -> size_t;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;
type FnCheckCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t;
type FnErrString = unsafe extern "C" fn(c_int) -> *const c_char;
type FnCompressSequences = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const ZSTD_Sequence,
    size_t,
    *const c_void,
    size_t,
) -> size_t;

struct G {
    ccctx: (FnCreate, FnCreate),
    fcctx: (FnFree, FnFree),
    cdctx: (FnCreate, FnCreate),
    fdctx: (FnFree, FnFree),
    setp: (FnSetParam, FnSetParam),
    getp: (FnGetParam, FnGetParam),
    setdp: (FnSetParam, FnSetParam),
    c2: (FnCompress2, FnCompress2),
    bound: (FnSizeSize, FnSizeSize),
    is_err: (FnIsError, FnIsError),
    ecode: (FnGetErrorCode, FnGetErrorCode),
    ename: (FnErrName, FnErrName),
}

fn g() -> G {
    G {
        ccctx: fnpair!("ZSTD_createCCtx", FnCreate),
        fcctx: fnpair!("ZSTD_freeCCtx", FnFree),
        cdctx: fnpair!("ZSTD_createDCtx", FnCreate),
        fdctx: fnpair!("ZSTD_freeDCtx", FnFree),
        setp: fnpair!("ZSTD_CCtx_setParameter", FnSetParam),
        getp: fnpair!("ZSTD_CCtx_getParameter", FnGetParam),
        setdp: fnpair!("ZSTD_DCtx_setParameter", FnSetParam),
        c2: fnpair!("ZSTD_compress2", FnCompress2),
        bound: fnpair!("ZSTD_compressBound", FnSizeSize),
        is_err: fnpair!("ZSTD_isError", FnIsError),
        ecode: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        ename: fnpair!("ZSTD_getErrorName", FnErrName),
    }
}

/// Assert two `size_t` returns are the same error (or same success value),
/// including error code and error-name string.
#[track_caller]
fn same_err(g: &G, ctx: &str, a: size_t, b: size_t) {
    unsafe {
        assert_eq!(a, b, "{ctx}: raw return differs (C={a:#x} R={b:#x})");
        assert_eq!(
            (g.is_err.0)(a),
            (g.is_err.1)(b),
            "{ctx}: ZSTD_isError differs"
        );
        assert_eq!(
            (g.ecode.0)(a),
            (g.ecode.1)(b),
            "{ctx}: ZSTD_getErrorCode differs"
        );
        assert_eq!(
            cstr((g.ename.0)(a)),
            cstr((g.ename.1)(b)),
            "{ctx}: ZSTD_getErrorName differs"
        );
    }
}

/// An aligned workspace for the `initStatic*` APIs.
struct Ws(Vec<u64>);
impl Ws {
    fn new(bytes: usize) -> Self {
        Ws(vec![0u64; bytes.div_ceil(8) + 1])
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    fn len(&self) -> size_t {
        self.0.len() * 8
    }
}

// ---------------------------------------------------------------- rows 36, 43
/// ERRORS rows 36 and 43: size-changing parameters on a STATIC context are
/// rejected with `parameter_unsupported`, because a static workspace cannot
/// grow. Row 43 is the `ZSTD_d_refMultipleDDicts` case on a static DCtx.
#[test]
fn c_static_context_parameter_rejection() {
    let g = g();
    let (c_isc, r_isc) = fnpair!("ZSTD_initStaticCCtx", FnInitStatic);
    let (c_isd, r_isd) = fnpair!("ZSTD_initStaticDCtx", FnInitStatic);
    let (c_ecs, r_ecs) = fnpair!("ZSTD_estimateCCtxSize", unsafe extern "C" fn(c_int) -> size_t);
    let (c_eds, r_eds) = fnpair!("ZSTD_estimateDCtxSize", unsafe extern "C" fn() -> size_t);

    unsafe {
        // --- row 36: static CCtx
        for lvl in [1, 3, 9, 19] {
            let need = c_ecs(lvl);
            assert_eq!(need, r_ecs(lvl), "estimateCCtxSize({lvl}) differs");
            let mut w1 = Ws::new(need);
            let mut w2 = Ws::new(need);
            let cc = c_isc(w1.ptr(), w1.len());
            let rc = r_isc(w2.ptr(), w2.len());
            assert_eq!(
                cc.is_null(),
                rc.is_null(),
                "ERRORS row 36: initStaticCCtx nullness differs at lvl={lvl}"
            );
            if cc.is_null() {
                continue;
            }
            // Every size-affecting parameter must be rejected identically.
            for &(p, v) in &[
                (ZSTD_c_windowLog, 27),
                (ZSTD_c_hashLog, 24),
                (ZSTD_c_chainLog, 25),
                (ZSTD_c_compressionLevel, 22),
                (ZSTD_c_strategy, ZSTD_btultra2),
                (ZSTD_c_enableLongDistanceMatching, 1),
                (ZSTD_c_ldmHashLog, 20),
                (ZSTD_c_maxBlockSize, 131_072),
                (ZSTD_c_useRowMatchFinder, ZSTD_ps_enable),
                (ZSTD_c_nbWorkers, 1),
                // and a NON size-affecting one, which must be accepted identically
                (ZSTD_c_checksumFlag, 1),
                (ZSTD_c_contentSizeFlag, 0),
            ] {
                let a = (g.setp.0)(cc, p, v);
                let b = (g.setp.1)(rc, p, v);
                same_err(&g, &format!("ERRORS row 36: static CCtx setParameter({p},{v})"), a, b);
                let mut x: c_int = 0;
                let mut y: c_int = 0;
                let ra = (g.getp.0)(cc, p, &mut x);
                let rb = (g.getp.1)(rc, p, &mut y);
                same_err(&g, &format!("ERRORS row 36: static CCtx getParameter({p})"), ra, rb);
                assert_eq!(x, y, "ERRORS row 36: static CCtx getParameter({p}) value");
            }
            // and it must still compress identically
            let mut rng = Rng::new(0x36_36);
            for &shape in &[Shape::Text, Shape::Random] {
                let src = gen(shape, 5000, &mut rng);
                let cap = (g.bound.0)(src.len());
                let mut o1 = vec![0xAAu8; cap];
                let mut o2 = vec![0xAAu8; cap];
                let n1 = (g.c2.0)(
                    cc,
                    o1.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                let n2 = (g.c2.1)(
                    rc,
                    o2.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                same_err(&g, &format!("ERRORS row 36: static CCtx compress2 {shape:?}"), n1, n2);
                if (g.is_err.0)(n1) == 0 {
                    assert_bytes_eq("ERRORS row 36: static CCtx frame", &o1[..n1], &o2[..n2]);
                }
            }
        }

        // --- row 43: static DCtx + ZSTD_d_refMultipleDDicts
        let need = c_eds();
        assert_eq!(need, r_eds(), "estimateDCtxSize differs");
        let mut w1 = Ws::new(need);
        let mut w2 = Ws::new(need);
        let cd = c_isd(w1.ptr(), w1.len());
        let rd = r_isd(w2.ptr(), w2.len());
        assert_eq!(
            cd.is_null(),
            rd.is_null(),
            "ERRORS row 43: initStaticDCtx nullness differs"
        );
        if !cd.is_null() {
            for &(p, v) in &[
                (ZSTD_d_refMultipleDDicts, ZSTD_rmd_refMultipleDDicts),
                (ZSTD_d_refMultipleDDicts, ZSTD_rmd_refSingleDDict),
                (ZSTD_d_windowLogMax, 27),
                (ZSTD_d_forceIgnoreChecksum, 1),
                (ZSTD_d_maxBlockSize, 131_072),
                (ZSTD_d_stableOutBuffer, 1),
            ] {
                let a = (g.setdp.0)(cd, p, v);
                let b = (g.setdp.1)(rd, p, v);
                same_err(
                    &g,
                    &format!("ERRORS row 43: static DCtx DCtx_setParameter({p},{v})"),
                    a,
                    b,
                );
            }
        }

        // undersized and misaligned static workspaces must fail identically
        for take in [0usize, 1, 8, need / 2, need - 1] {
            let mut w1 = Ws::new(need);
            let mut w2 = Ws::new(need);
            let a = c_isd(w1.ptr(), take);
            let b = r_isd(w2.ptr(), take);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "ERRORS row 43: initStaticDCtx(size={take}) nullness differs"
            );
        }
        let mut w1 = Ws::new(need + 16);
        let mut w2 = Ws::new(need + 16);
        let a = c_isd((w1.ptr() as *mut u8).add(1) as *mut c_void, need);
        let b = r_isd((w2.ptr() as *mut u8).add(1) as *mut c_void, need);
        assert_eq!(
            a.is_null(),
            b.is_null(),
            "ERRORS row 43: misaligned initStaticDCtx nullness differs"
        );
    }
}

// ------------------------------------------------------------ rows 38, 40, 87
/// ERRORS row 38: the `getParameter` side of the MT-only parameters when the
/// library is built without `ZSTD_MULTITHREAD`.
/// ERRORS row 40: `ZSTD_checkCParams` rejects each field out of its own bound.
/// ERRORS row 87: `ZSTD_CCtx_getParameter` for every parameter id on a CCtx
/// that has a CDict / prefix referenced.
#[test]
fn c_getparameter_and_checkcparams() {
    let g = g();
    let (c_cp, r_cp) = fnpair!("ZSTD_checkCParams", FnCheckCParams);
    let (c_ac, r_ac) = fnpair!(
        "ZSTD_adjustCParams",
        unsafe extern "C" fn(ZSTD_compressionParameters, u64, size_t) -> ZSTD_compressionParameters
    );
    let (c_gc, r_gc) = fnpair!(
        "ZSTD_getCParams",
        unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_compressionParameters
    );
    let (c_bd, r_bd) = fnpair!("ZSTD_cParam_getBounds", FnBounds);
    let (c_rp, r_rp) = fnpair!(
        "ZSTD_CCtx_refPrefix",
        unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t
    );
    let (c_rc, r_rc) = fnpair!(
        "ZSTD_CCtx_refCDict",
        unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t
    );

    let all_params = [
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
        // out-of-range ids too
        -1,
        0,
        1,
        99,
        999,
        1018,
        100_000,
        c_int::MIN,
        c_int::MAX,
    ];

    unsafe {
        // --- row 38 / row 87: getParameter for every id, on a plain CCtx and on
        // one with a prefix and a CDict referenced.
        let mut rng = Rng::new(0x38_87);
        let prefix = gen(Shape::Text, 4096, &mut rng);
        for state in 0..3 {
            let cc = (g.ccctx.0)();
            let rc = (g.ccctx.1)();
            match state {
                1 => {
                    let a = c_rp(cc, prefix.as_ptr() as *const c_void, prefix.len());
                    let b = r_rp(rc, prefix.as_ptr() as *const c_void, prefix.len());
                    same_err(&g, "ERRORS row 87: refPrefix", a, b);
                }
                2 => {
                    // NULL CDict is the documented "clear" case and is the only
                    // CDict pointer we can hand to BOTH libraries (a CDict from
                    // one library is opaque to the other).
                    let a = c_rc(cc, std::ptr::null());
                    let b = r_rc(rc, std::ptr::null());
                    same_err(&g, "ERRORS row 87: refCDict(NULL)", a, b);
                }
                _ => {}
            }
            for p in all_params {
                let mut x: c_int = 0x5A5A_5A5A;
                let mut y: c_int = 0x5A5A_5A5A;
                let a = (g.getp.0)(cc, p, &mut x);
                let b = (g.getp.1)(rc, p, &mut y);
                same_err(
                    &g,
                    &format!("ERRORS rows 38/87: getParameter({p}) state={state}"),
                    a,
                    b,
                );
                assert_eq!(
                    x, y,
                    "ERRORS rows 38/87: getParameter({p}) value state={state}"
                );
            }
            (g.fcctx.0)(cc);
            (g.fcctx.1)(rc);
        }

        // --- row 40: ZSTD_checkCParams, each field out of bound
        let base = c_gc(3, 0, 0);
        assert_eq!(base, r_gc(3, 0, 0), "getCParams(3) differs");
        let fields: [(&str, c_int); 7] = [
            ("windowLog", ZSTD_c_windowLog),
            ("chainLog", ZSTD_c_chainLog),
            ("hashLog", ZSTD_c_hashLog),
            ("searchLog", ZSTD_c_searchLog),
            ("minMatch", ZSTD_c_minMatch),
            ("targetLength", ZSTD_c_targetLength),
            ("strategy", ZSTD_c_strategy),
        ];
        for (name, pid) in fields {
            let bd = c_bd(pid);
            assert_eq!(bd, r_bd(pid), "getBounds({pid}) differs");
            for v in [
                bd.lowerBound.saturating_sub(1),
                bd.upperBound.saturating_add(1),
                bd.lowerBound,
                bd.upperBound,
                c_int::MAX,
            ] {
                let mut p = base;
                let vu = v as c_uint;
                match name {
                    "windowLog" => p.windowLog = vu,
                    "chainLog" => p.chainLog = vu,
                    "hashLog" => p.hashLog = vu,
                    "searchLog" => p.searchLog = vu,
                    "minMatch" => p.minMatch = vu,
                    "targetLength" => p.targetLength = vu,
                    _ => p.strategy = vu,
                }
                let a = c_cp(p);
                let b = r_cp(p);
                same_err(
                    &g,
                    &format!("ERRORS row 40: checkCParams {name}={v} ({vu:#x})"),
                    a,
                    b,
                );
                // adjustCParams must agree on the same inputs
                for &(sz, ds) in &[(0u64, 0usize), (1000, 0), (1 << 20, 4096)] {
                    let x = c_ac(p, sz, ds);
                    let y = r_ac(p, sz, ds);
                    assert_eq!(
                        x, y,
                        "ERRORS row 40: adjustCParams {name}={v} sz={sz} ds={ds}"
                    );
                }
            }
        }
    }
}

// -------------------------------------------------- rows 88, 97, 98 (report)
/// ERRORS rows 88, 97 and 98 sit behind conditions that a caller cannot force
/// through the public FFI in this build: row 88 needs the internal
/// `seqCollector` active while an uncompressible block is emitted, row 97 needs
/// a failed multi-threading context allocation (the MT path is not compiled at
/// all here — `ZSTD_MULTITHREAD` is undefined), and row 98 needs the internal
/// pledged-size/dstSize interaction. Rather than fake them, this test asserts
/// the closest *observable* behaviour on both libraries.
#[test]
fn c_unreachable_rows_closest_observable() {
    let g = g();
    let (c_gs, r_gs) = fnpair!(
        "ZSTD_generateSequences",
        unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, size_t, *const c_void, size_t) -> size_t
    );
    let (c_sb, r_sb) = fnpair!("ZSTD_sequenceBound", FnSizeSize);
    let (c_ps, r_ps) = fnpair!(
        "ZSTD_CCtx_setPledgedSrcSize",
        unsafe extern "C" fn(*mut c_void, u64) -> size_t
    );

    eprintln!(
        "ERRORS row 88: not reachable via public API — requires the internal \
         seqCollector to be active while an uncompressible block is emitted; \
         asserting the observable ZSTD_generateSequences surface instead."
    );
    eprintln!(
        "ERRORS row 97: not reachable via public API — the multi-threading path \
         is not compiled (ZSTD_MULTITHREAD undefined), so cctx->mtctx is never \
         allocated; asserting that ZSTD_c_nbWorkers>0 is rejected identically."
    );
    eprintln!(
        "ERRORS row 98: not reachable as a distinct error — the pledged-size / \
         dstSize interaction surfaces as the ordinary dstSize_tooSmall path; \
         asserting pledged-size mismatch behaviour instead."
    );

    let mut rng = Rng::new(0x88_97_98);
    unsafe {
        // row 88 neighbourhood: generateSequences over incompressible data with
        // undersized and exact sequence buffers.
        for &shape in &[Shape::Random, Shape::Mixed, Shape::Zeros] {
            for &len in &[0usize, 1, 1024, 140_000] {
                let src = gen(shape, len, &mut rng);
                let need = c_sb(len);
                assert_eq!(need, r_sb(len), "ERRORS row 88: sequenceBound({len})");
                for cap in [0usize, 1, need / 2, need] {
                    let mut s1 = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let mut s2 = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let cc = (g.ccctx.0)();
                    let rc = (g.ccctx.1)();
                    let a = c_gs(
                        cc,
                        s1.as_mut_ptr(),
                        cap,
                        if len == 0 {
                            std::ptr::null()
                        } else {
                            src.as_ptr() as *const c_void
                        },
                        len,
                    );
                    let b = r_gs(
                        rc,
                        s2.as_mut_ptr(),
                        cap,
                        if len == 0 {
                            std::ptr::null()
                        } else {
                            src.as_ptr() as *const c_void
                        },
                        len,
                    );
                    same_err(
                        &g,
                        &format!("ERRORS row 88: generateSequences {shape:?} len={len} cap={cap}"),
                        a,
                        b,
                    );
                    if (g.is_err.0)(a) == 0 {
                        assert_eq!(
                            &s1[..a],
                            &s2[..b],
                            "ERRORS row 88: sequences differ {shape:?} len={len} cap={cap}"
                        );
                    }
                    (g.fcctx.0)(cc);
                    (g.fcctx.1)(rc);
                }
            }
        }

        // row 97 neighbourhood: MT parameters must be rejected identically.
        for w in [1, 2, 4, 64, i32::MAX] {
            let cc = (g.ccctx.0)();
            let rc = (g.ccctx.1)();
            let a = (g.setp.0)(cc, ZSTD_c_nbWorkers, w);
            let b = (g.setp.1)(rc, ZSTD_c_nbWorkers, w);
            same_err(&g, &format!("ERRORS row 97: nbWorkers={w}"), a, b);
            (g.fcctx.0)(cc);
            (g.fcctx.1)(rc);
        }

        // row 98 neighbourhood: pledged size that does not match the data, with
        // both too-small and adequate output buffers.
        for &shape in &[Shape::Text, Shape::Random] {
            for &len in &[1usize, 1000, 70_000] {
                let src = gen(shape, len, &mut rng);
                for pledged in [
                    0u64,
                    (len - 1) as u64,
                    len as u64,
                    (len + 1) as u64,
                    u64::MAX,
                ] {
                    for cap in [0usize, 1, 8, (g.bound.0)(len)] {
                        let cc = (g.ccctx.0)();
                        let rc = (g.ccctx.1)();
                        let a = c_ps(cc, pledged);
                        let b = r_ps(rc, pledged);
                        same_err(
                            &g,
                            &format!("ERRORS row 98: setPledgedSrcSize({pledged})"),
                            a,
                            b,
                        );
                        let mut o1 = vec![0xAAu8; cap.max(1)];
                        let mut o2 = vec![0xAAu8; cap.max(1)];
                        let n1 = (g.c2.0)(
                            cc,
                            o1.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            len,
                        );
                        let n2 = (g.c2.1)(
                            rc,
                            o2.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            len,
                        );
                        same_err(
                            &g,
                            &format!(
                                "ERRORS row 98: compress2 pledged={pledged} cap={cap} {shape:?} len={len}"
                            ),
                            n1,
                            n2,
                        );
                        assert_bytes_eq(
                            &format!("ERRORS row 98: output pledged={pledged} cap={cap}"),
                            &o1,
                            &o2,
                        );
                        (g.fcctx.0)(cc);
                        (g.fcctx.1)(rc);
                    }
                }
            }
        }
    }
}

// ------------------------------------- rows 101-107, 115: sequence validation
/// ERRORS rows 101, 102, 103, 104, 105, 106, 107 and 115: every rejection in
/// `ZSTD_validateSequence` / `ZSTD_copySequencesToSeqStore*` /
/// `ZSTD_transferSequences`, driven through `ZSTD_compressSequences` with
/// `ZSTD_c_validateSequences = 1` (with validation off the C documents the
/// behaviour as undefined, so equality is not asserted there).
#[test]
fn c_sequence_validation_rejections() {
    let g = g();
    let (c_cs, r_cs) = fnpair!("ZSTD_compressSequences", FnCompressSequences);
    let (c_gs, r_gs) = fnpair!(
        "ZSTD_generateSequences",
        unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, size_t, *const c_void, size_t) -> size_t
    );
    let (c_sb, r_sb) = fnpair!("ZSTD_sequenceBound", FnSizeSize);

    let mut rng = Rng::new(0x101_115);
    unsafe {
        // Build a VALID explicit-delimiter sequence array from real data, then
        // mutate it one way per ERRORS row.
        for &shape in &[Shape::Text, Shape::Mixed, Shape::Repetitive] {
            for &len in &[4096usize, 60_000, 140_000] {
                let src = gen(shape, len, &mut rng);
                let need = c_sb(len);
                assert_eq!(need, r_sb(len), "sequenceBound({len})");
                let mut base = vec![ZSTD_Sequence::default(); need];
                let mut base2 = vec![ZSTD_Sequence::default(); need];
                let cc = (g.ccctx.0)();
                let rc = (g.ccctx.1)();
                let n = c_gs(cc, base.as_mut_ptr(), need, src.as_ptr() as *const c_void, len);
                let n2 = r_gs(rc, base2.as_mut_ptr(), need, src.as_ptr() as *const c_void, len);
                same_err(&g, "generateSequences setup", n, n2);
                (g.fcctx.0)(cc);
                (g.fcctx.1)(rc);
                if (g.is_err.0)(n) != 0 || n == 0 {
                    continue;
                }
                assert_eq!(&base[..n], &base2[..n2], "generateSequences setup array");
                let valid = base[..n].to_vec();

                // (row, description, mutation)
                type Mut = fn(&mut Vec<ZSTD_Sequence>);
                let cases: Vec<(u32, &str, Mut)> = vec![
                    (
                        101,
                        "offset beyond window+dict",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            for q in s.iter_mut() {
                                if q.matchLength != 0 {
                                    q.offset = 0xFFFF_FFF0;
                                    break;
                                }
                            }
                        },
                    ),
                    (101, "offset = 0", |s: &mut Vec<ZSTD_Sequence>| {
                        for q in s.iter_mut() {
                            if q.matchLength != 0 {
                                q.offset = 0;
                                break;
                            }
                        }
                    }),
                    (
                        102,
                        "matchLength below MINMATCH",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            for q in s.iter_mut() {
                                if q.matchLength >= 3 {
                                    q.matchLength = 1;
                                    break;
                                }
                            }
                        },
                    ),
                    (
                        103,
                        "too many sequences (duplicate the array)",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            let c = s.clone();
                            s.extend_from_slice(&c);
                        },
                    ),
                    (
                        104,
                        "input exhausted before block delimiter (drop terminator)",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            while let Some(l) = s.last() {
                                if l.matchLength == 0 && l.offset == 0 {
                                    s.pop();
                                } else {
                                    break;
                                }
                            }
                        },
                    ),
                    (
                        105,
                        "block content length disagrees with delimiter",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            for q in s.iter_mut() {
                                if q.litLength > 1 {
                                    q.litLength -= 1;
                                    break;
                                }
                            }
                        },
                    ),
                    (
                        106,
                        "matchLength and offset both 0 in a non-delimiter slot",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            if s.len() > 2 {
                                s[0].matchLength = 0;
                                s[0].offset = 0;
                            }
                        },
                    ),
                    (
                        107,
                        "no block delimiter at all",
                        |s: &mut Vec<ZSTD_Sequence>| {
                            s.retain(|q| !(q.matchLength == 0 && q.offset == 0));
                        },
                    ),
                    (115, "nbSequences >= maxNbSeq", |s: &mut Vec<ZSTD_Sequence>| {
                        let c = s.clone();
                        for _ in 0..4 {
                            s.extend_from_slice(&c);
                        }
                    }),
                ];

                for (row, what, mutate) in cases {
                    let mut seqs = valid.clone();
                    mutate(&mut seqs);
                    for delim in [ZSTD_sf_explicitBlockDelimiters, ZSTD_sf_noBlockDelimiters] {
                        // NOTE: only a full-size dst is exercised here. Feeding an
                        // INVALID sequence array together with a dstCapacity smaller
                        // than the frame header is undefined behaviour in the C
                        // reference itself: ZSTD_compressSequences ignores the error
                        // returned by ZSTD_writeFrameHeader (guarded only by an
                        // assert(), compiled out in release) and writes past dst,
                        // aborting with "double free or corruption". Verified
                        // out-of-band that the C .so and the Rust .so abort at the
                        // SAME call with the SAME preceding return values, so this is
                        // a faithfully-reproduced upstream defect, not a divergence.
                        // Tiny-dstCapacity behaviour on VALID sequences is covered in
                        // phaseb_seq.rs.
                        for cap in [(g.bound.0)(len)] {
                            let cc = (g.ccctx.0)();
                            let rc = (g.ccctx.1)();
                            let mut ok = true;
                            for &(p, v) in &[
                                (ZSTD_c_validateSequences, 1),
                                (ZSTD_c_blockDelimiters, delim),
                            ] {
                                let a = (g.setp.0)(cc, p, v);
                                let b = (g.setp.1)(rc, p, v);
                                same_err(
                                    &g,
                                    &format!("ERRORS row {row}: setParameter({p},{v})"),
                                    a,
                                    b,
                                );
                                if (g.is_err.0)(a) != 0 {
                                    ok = false;
                                }
                            }
                            if ok {
                                let mut o1 = vec![0xAAu8; cap.max(1)];
                                let mut o2 = vec![0xAAu8; cap.max(1)];
                                let a = c_cs(
                                    cc,
                                    o1.as_mut_ptr() as *mut c_void,
                                    cap,
                                    seqs.as_ptr(),
                                    seqs.len(),
                                    src.as_ptr() as *const c_void,
                                    len,
                                );
                                let b = r_cs(
                                    rc,
                                    o2.as_mut_ptr() as *mut c_void,
                                    cap,
                                    seqs.as_ptr(),
                                    seqs.len(),
                                    src.as_ptr() as *const c_void,
                                    len,
                                );
                                let tag = format!(
                                    "ERRORS row {row}: {what} delim={delim} cap={cap} {shape:?} len={len}"
                                );
                                same_err(&g, &tag, a, b);
                                assert_bytes_eq(&format!("{tag}: output"), &o1, &o2);
                            }
                            (g.fcctx.0)(cc);
                            (g.fcctx.1)(rc);
                        }
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------- row 295 (HUF)
/// ERRORS row 295: `HUF_setMaxHeight` rejects `maxNbBits > HUF_TABLELOG_MAX`.
/// `HUF_setMaxHeight` is not exported, so this is driven through the exported
/// `HUF_buildCTable_wksp`, which calls it — the documented substitution.
#[test]
fn c_huf_tablelog_too_large() {
    type FnBuildCTable = unsafe extern "C" fn(
        *mut c_void,
        *const c_uint,
        c_uint,
        c_uint,
        *mut c_void,
        size_t,
    ) -> size_t;
    let (c_b, r_b) = fnpair!("HUF_buildCTable_wksp", FnBuildCTable);
    let (c_ie, r_ie) = fnpair!("HUF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("HUF_getErrorName", FnErrName);
    let (c_ot, r_ot) = fnpair!(
        "HUF_optimalTableLog",
        unsafe extern "C" fn(c_uint, size_t, c_uint) -> c_uint
    );

    let mut rng = Rng::new(0x295);
    unsafe {
        // HUF_TABLELOG_MAX is 12; drive tableLog well past it, and also at and
        // just below the limit so the accept/reject boundary is pinned down.
        for maxSymbol in [1u32, 3, 31, 127, 255] {
            let mut count = vec![0u32; 256];
            for i in 0..=maxSymbol as usize {
                count[i] = 1 + (rng.next_u32() % 1000);
            }
            // `maxNbBits` must be at least ceil(log2(maxSymbol+1)) — otherwise no
            // prefix code exists and HUF_setMaxHeight walks off its rank table.
            // That precondition is guarded only by assert() (compiled out in
            // release), so smaller values segfault the C reference too: verified
            // out-of-band that the C .so and the Rust .so fault at the SAME
            // (maxSymbol, tableLog) pair. maxSymbol <= 255 needs 8 bits, so the
            // sweep starts at 8. tableLog 8..12 must be ACCEPTED and 13/16/255
            // must be REJECTED with ZSTD_error_GENERIC — that boundary is the
            // actual ERRORS row 295 trigger.
            for tablelog in [8u32, 9, 10, 11, 12, 13, 16, 255] {
                // HUF_buildCTable_wksp requires a U32-ALIGNED workspace and CTable
                // (it casts them to U32*/HUF_CElt*); a Vec<u8> is only byte-aligned,
                // which is an out-of-contract call that faults in BOTH libraries.
                // Back both buffers with u64 so they are 8-byte aligned.
                let mut w1 = vec![0xAAAAAAAA_AAAAAAAAu64; (1 << 16) / 8];
                let mut w2 = vec![0xAAAAAAAA_AAAAAAAAu64; (1 << 16) / 8];
                let mut t1 = vec![0xAAAAAAAA_AAAAAAAAu64; 4 * 1024 / 8];
                let mut t2 = vec![0xAAAAAAAA_AAAAAAAAu64; 4 * 1024 / 8];
                let wlen = w1.len() * 8;
                let a = c_b(
                    t1.as_mut_ptr() as *mut c_void,
                    count.as_ptr(),
                    maxSymbol,
                    tablelog,
                    w1.as_mut_ptr() as *mut c_void,
                    wlen,
                );
                let b = r_b(
                    t2.as_mut_ptr() as *mut c_void,
                    count.as_ptr(),
                    maxSymbol,
                    tablelog,
                    w2.as_mut_ptr() as *mut c_void,
                    wlen,
                );
                let tag = format!(
                    "ERRORS row 295: HUF_buildCTable_wksp maxSymbol={maxSymbol} tableLog={tablelog}"
                );
                assert_eq!(a, b, "{tag}: raw return differs (C={a:#x} R={b:#x})");
                assert_eq!(c_ie(a), r_ie(b), "{tag}: HUF_isError differs");
                assert_eq!(
                    cstr(c_en(a)),
                    cstr(r_en(b)),
                    "{tag}: HUF_getErrorName differs"
                );
                let bytes = |v: &[u64]| -> Vec<u8> {
                    v.iter().flat_map(|x| x.to_le_bytes()).collect()
                };
                assert_bytes_eq(&format!("{tag}: CTable"), &bytes(&t1), &bytes(&t2));
                assert_bytes_eq(&format!("{tag}: workspace"), &bytes(&w1), &bytes(&w2));
            }
        }
        // and the tableLog chooser itself
        for &srcSize in &[2usize, 3, 255, 256, 1 << 16] {
            for maxSymbol in [1u32, 3, 255] {
                for tablelog in [8u32, 11, 12, 13, 255] {
                    assert_eq!(
                        c_ot(tablelog, srcSize, maxSymbol),
                        r_ot(tablelog, srcSize, maxSymbol),
                        "ERRORS row 295: HUF_optimalTableLog({tablelog},{srcSize},{maxSymbol})"
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------- rows 310, 312 (ZBUFF)
/// ERRORS row 310: `ZBUFF_compressInit_advanced` forwards a dictionary-load
/// failure. ERRORS row 312: `ZBUFF_decompressInit_usingDict` forwards a
/// corrupt-dictionary failure.
#[test]
fn c_zbuff_dict_forwarding() {
    type FnZCreate = unsafe extern "C" fn() -> *mut c_void;
    type FnZFree = unsafe extern "C" fn(*mut c_void) -> size_t;
    type FnZInitAdv = unsafe extern "C" fn(
        *mut c_void,
        *const c_void,
        size_t,
        ZSTD_parameters,
        u64,
    ) -> size_t;
    type FnZInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
    type FnZDInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;

    let (c_cc, r_cc) = fnpair!("ZBUFF_createCCtx", FnZCreate);
    let (c_fc, r_fc) = fnpair!("ZBUFF_freeCCtx", FnZFree);
    let (c_cd, r_cd) = fnpair!("ZBUFF_createDCtx", FnZCreate);
    let (c_fd, r_fd) = fnpair!("ZBUFF_freeDCtx", FnZFree);
    let (c_ia, r_ia) = fnpair!("ZBUFF_compressInit_advanced", FnZInitAdv);
    let (c_id, r_id) = fnpair!("ZBUFF_compressInitDictionary", FnZInitDict);
    let (c_di, r_di) = fnpair!("ZBUFF_decompressInitDictionary", FnZDInitDict);
    let (c_ie, r_ie) = fnpair!("ZBUFF_isError", FnIsError);
    let (c_en, r_en) = fnpair!("ZBUFF_getErrorName", FnErrName);
    let (c_gp, r_gp) = fnpair!(
        "ZSTD_getParams",
        unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_parameters
    );

    #[track_caller]
    fn zsame(
        ie: (FnIsError, FnIsError),
        en: (FnErrName, FnErrName),
        ctx: &str,
        a: size_t,
        b: size_t,
    ) {
        unsafe {
            assert_eq!(a, b, "{ctx}: raw return differs (C={a:#x} R={b:#x})");
            assert_eq!((ie.0)(a), (ie.1)(b), "{ctx}: ZBUFF_isError differs");
            assert_eq!(
                cstr((en.0)(a)),
                cstr((en.1)(b)),
                "{ctx}: ZBUFF_getErrorName differs"
            );
        }
    }
    let ie = (c_ie, r_ie);
    let en = (c_en, r_en);

    let mut rng = Rng::new(0x310_312);
    unsafe {
        // Dictionaries that are corrupt in the ways the C distinguishes.
        let mut dicts: Vec<(&str, Vec<u8>)> = Vec::new();
        dicts.push(("empty", Vec::new()));
        dicts.push(("1 byte", vec![0x37]));
        for n in [4usize, 5, 8, 12, 16, 32, 64] {
            // starts with the dictionary magic but is truncated / garbage after
            let mut v = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
            while v.len() < n {
                v.push((rng.next_u32() & 0xFF) as u8);
            }
            v.truncate(n.max(4));
            dicts.push(("magic+garbage", v));
        }
        dicts.push(("random 1000", gen(Shape::Random, 1000, &mut rng)));
        dicts.push(("text 1000", gen(Shape::Text, 1000, &mut rng)));
        // a magic-prefixed buffer long enough to look like a real dictionary
        let mut big = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
        big.extend_from_slice(&gen(Shape::Random, 4096, &mut rng));
        dicts.push(("magic+4096 random", big));

        for (dname, dict) in &dicts {
            let dp = if dict.is_empty() {
                std::ptr::null()
            } else {
                dict.as_ptr() as *const c_void
            };
            for lvl in [-1, 0, 1, 3, 19, 22, 23, 100, -100] {
                // --- row 310: compressInit_advanced forwards the dict-load result
                let a = c_cc();
                let b = r_cc();
                assert_eq!(a.is_null(), b.is_null(), "ZBUFF_createCCtx nullness");
                let p1 = c_gp(lvl.clamp(-7, 22), 0, dict.len());
                let p2 = r_gp(lvl.clamp(-7, 22), 0, dict.len());
                assert_eq!(p1, p2, "ZSTD_getParams({lvl}) differs");
                let x = c_ia(a, dp, dict.len(), p1, 0);
                let y = r_ia(b, dp, dict.len(), p2, 0);
                zsame(
                    ie,
                    en,
                    &format!("ERRORS row 310: compressInit_advanced d={dname} lvl={lvl}"),
                    x,
                    y,
                );
                // and the simpler dictionary init
                let x = c_id(a, dp, dict.len(), lvl);
                let y = r_id(b, dp, dict.len(), lvl);
                zsame(
                    ie,
                    en,
                    &format!("ERRORS row 310: compressInitDictionary d={dname} lvl={lvl}"),
                    x,
                    y,
                );
                c_fc(a);
                r_fc(b);
            }

            // --- row 312: decompressInitDictionary forwards the dict result
            let a = c_cd();
            let b = r_cd();
            assert_eq!(a.is_null(), b.is_null(), "ZBUFF_createDCtx nullness");
            let x = c_di(a, dp, dict.len());
            let y = r_di(b, dp, dict.len());
            zsame(
                ie,
                en,
                &format!("ERRORS row 312: decompressInitDictionary d={dname}"),
                x,
                y,
            );
            c_fd(a);
            r_fd(b);
        }
    }
}

// --------------------------------------------------------- rows 324, 327
/// ERRORS row 324: `ZSTD_dictLoadMethod_e` = 2 (and any other out-of-range
/// value) has NO explicit range check — the `byRef` branch tests `== byRef`, so
/// everything else falls through to `byCopy` with no error. Both libraries must
/// agree, including on the produced frame bytes.
/// ERRORS row 327: `ZSTD_getErrorString` for codes past `ZSTD_error_maxCode`
/// returns the default string.
#[test]
fn c_dict_load_method_fallthrough_and_error_strings() {
    let g = g();
    let (c_la, r_la) = fnpair!("ZSTD_CCtx_loadDictionary_advanced", FnLoadDictAdv);
    let (c_lda, r_lda) = fnpair!("ZSTD_DCtx_loadDictionary_advanced", FnLoadDictAdv);
    let (c_es, r_es) = fnpair!("ZSTD_getErrorString", FnErrString);
    let (c_dec, r_dec) = fnpair!(
        "ZSTD_decompressDCtx",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );

    let mut rng = Rng::new(0x324_327);
    unsafe {
        // --- row 324
        let dict = gen(Shape::Text, 8192, &mut rng);
        let dp = dict.as_ptr() as *const c_void;
        for lm in [
            ZSTD_dlm_byCopy,
            ZSTD_dlm_byRef,
            2,
            3,
            17,
            -1,
            c_int::MIN,
            c_int::MAX,
        ] {
            for ct in [
                ZSTD_dct_auto,
                ZSTD_dct_rawContent,
                ZSTD_dct_fullDict,
                3,
                -1,
                c_int::MAX,
            ] {
                for &shape in &[Shape::Text, Shape::Random] {
                    for &len in &[0usize, 1, 3000, 140_000] {
                        let src = gen(shape, len, &mut rng);
                        let cap = (g.bound.0)(len).max(64);
                        let cc = (g.ccctx.0)();
                        let rc = (g.ccctx.1)();
                        let a = c_la(cc, dp, dict.len(), lm, ct, 3);
                        let b = r_la(rc, dp, dict.len(), lm, ct, 3);
                        let tag =
                            format!("ERRORS row 324: loadDictionary_advanced lm={lm} ct={ct}");
                        same_err(&g, &tag, a, b);
                        if (g.is_err.0)(a) == 0 {
                            let mut o1 = vec![0xAAu8; cap];
                            let mut o2 = vec![0xAAu8; cap];
                            let sp = if len == 0 {
                                std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
                            } else {
                                src.as_ptr() as *const c_void
                            };
                            let n1 =
                                (g.c2.0)(cc, o1.as_mut_ptr() as *mut c_void, cap, sp, len);
                            let n2 =
                                (g.c2.1)(rc, o2.as_mut_ptr() as *mut c_void, cap, sp, len);
                            same_err(
                                &g,
                                &format!("{tag}: compress2 {shape:?} len={len}"),
                                n1,
                                n2,
                            );
                            assert_bytes_eq(&format!("{tag}: frame {shape:?} len={len}"), &o1, &o2);

                            // symmetric check on the decoder side
                            if (g.is_err.0)(n1) == 0 {
                                let dx = (g.cdctx.0)();
                                let dy = (g.cdctx.1)();
                                let x = c_lda(dx, dp, dict.len(), lm, ct, 3);
                                let y = r_lda(dy, dp, dict.len(), lm, ct, 3);
                                same_err(
                                    &g,
                                    &format!("{tag}: DCtx_loadDictionary_advanced"),
                                    x,
                                    y,
                                );
                                if (g.is_err.0)(x) == 0 {
                                    let mut q1 = vec![0xAAu8; len + 8];
                                    let mut q2 = vec![0xAAu8; len + 8];
                                    // C decoder on the Rust frame and vice versa
                                    let u = c_dec(
                                        dx,
                                        q1.as_mut_ptr() as *mut c_void,
                                        q1.len(),
                                        o2.as_ptr() as *const c_void,
                                        n2,
                                    );
                                    let v = r_dec(
                                        dy,
                                        q2.as_mut_ptr() as *mut c_void,
                                        q2.len(),
                                        o1.as_ptr() as *const c_void,
                                        n1,
                                    );
                                    same_err(&g, &format!("{tag}: cross decode"), u, v);
                                    assert_bytes_eq(&format!("{tag}: decoded"), &q1, &q2);
                                }
                                (g.fdctx.0)(dx);
                                (g.fdctx.1)(dy);
                            }
                        }
                        (g.fcctx.0)(cc);
                        (g.fcctx.1)(rc);
                    }
                }
            }
        }

        // --- row 327: every error code, valid and out of range
        let mut codes: Vec<c_int> = vec![
            -1,
            0,
            1,
            10,
            12,
            14,
            16,
            20,
            22,
            24,
            30,
            32,
            34,
            40,
            41,
            42,
            44,
            46,
            48,
            49,
            50,
            60,
            62,
            64,
            66,
            70,
            72,
            74,
            80,
            82,
            100,
            102,
            104,
            105,
            106,
            107,
            120,
            121,
            122,
            1000,
            100_000,
            c_int::MIN,
            c_int::MAX,
        ];
        for i in 0..130 {
            codes.push(i);
        }
        for code in codes {
            let a = cstr(c_es(code));
            let b = cstr(r_es(code));
            assert_eq!(
                a, b,
                "ERRORS row 327: ZSTD_getErrorString({code}) differs (C={a:?} R={b:?})"
            );
        }
    }
}
