//! Phase B — CONFIGS.md rows 120..125: explicit sequences, sequence
//! conversion, and the long-distance-matcher's public entry points.
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------- types

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct BlockSummary {
    nbSequences: usize,
    blockSize: usize,
    litSize: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct ldmParams_t {
    enableLdm: c_int,
    hashLog: c_uint,
    bucketSizeLog: c_uint,
    minMatchLength: c_uint,
    hashRateLog: c_uint,
    windowLog: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct rawSeq {
    offset: c_uint,
    litLength: c_uint,
    matchLength: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RawSeqStore_t {
    seq: *mut rawSeq,
    pos: usize,
    posInSequence: usize,
    size: usize,
    capacity: usize,
}

type FnGenSeq =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, usize, *const c_void, usize) -> usize;
type FnMerge = unsafe extern "C" fn(*mut ZSTD_Sequence, usize) -> usize;
type FnCompressSeq = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
) -> usize;
type FnCompressSeqLit = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
    usize,
    usize,
) -> usize;
type FnConvert =
    unsafe extern "C" fn(*mut c_void, *const ZSTD_Sequence, usize, c_int) -> usize;
type FnSummary = unsafe extern "C" fn(*const ZSTD_Sequence, usize) -> BlockSummary;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

/// Ask the *C* library for the sequences of `src` at `level`; used as the
/// fixture for the ZSTD_compressSequences* rows.
unsafe fn c_sequences(src: &[u8], level: c_int, block_delims: c_int) -> Vec<ZSTD_Sequence> {
    let (gsc, _) = duo::<FnGenSeq>("ZSTD_generateSequences");
    let (bnd, _) = duo::<FnSizeT1>("ZSTD_sequenceBound");
    let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (rst, _) = duo::<FnReset>("ZSTD_CCtx_reset");
    let (mrg, _) = duo::<FnMerge>("ZSTD_mergeBlockDelimiters");
    let cctx = CtxPair::cctx();
    rst(cctx.c, ZSTD_reset_session_and_parameters);
    sp(cctx.c, ZSTD_c_compressionLevel, level);
    let cap = bnd(src.len()).max(1);
    let mut seqs = vec![ZSTD_Sequence::default(); cap];
    let n = gsc(
        cctx.c,
        seqs.as_mut_ptr(),
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
    );
    if is_err(n) {
        return Vec::new();
    }
    seqs.truncate(n);
    if block_delims == 0 && !seqs.is_empty() {
        let m = mrg(seqs.as_mut_ptr(), seqs.len());
        if is_err(m) {
            return Vec::new();
        }
        seqs.truncate(m);
    }
    seqs
}

// ---------------------------------------------------------------- row 120, 121

#[test]
fn row120_generate_sequences() {
    unsafe {
        let (gsc, gsr) = duo::<FnGenSeq>("ZSTD_generateSequences");
        let (bnd, _) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let cctx = CtxPair::cctx();
        let mut rng = Rng::new(120);
        for lvl in [1, 3, 5, 9, 13, 19, 22] {
            for cls in 0..N_CLASSES {
                for &sz in &[0usize, 1, 7, 300, 5000, 60_000, 140_000] {
                    eqv(
                        "row120 reset",
                        rc(cctx.c, ZSTD_reset_session_and_parameters),
                        rr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    eqv(
                        "row120 set level",
                        sp_c(cctx.c, ZSTD_c_compressionLevel, lvl),
                        sp_r(cctx.r, ZSTD_c_compressionLevel, lvl),
                    );
                    let src = gen_class(cls, sz, lvl as u64);
                    let full = bnd(sz).max(1);
                    for cap in [full, full / 2, 4, 1, 0] {
                        let mut sc = vec![ZSTD_Sequence::default(); cap.max(1)];
                        let mut sr = vec![ZSTD_Sequence::default(); cap.max(1)];
                        let a = gsc(
                            cctx.c,
                            sc.as_mut_ptr(),
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let b = gsr(
                            cctx.r,
                            sr.as_mut_ptr(),
                            cap,
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let w = format!("row120 lvl={lvl} cls={cls} sz={sz} cap={cap}");
                        eqv(&format!("{w} generateSequences"), a, b);
                        eqv(&format!("{w} sequences"), &sc[..], &sr[..]);
                        if is_err(a) {
                            continue;
                        }
                        // row 121: mergeBlockDelimiters over the same array
                        let (mc, mr) = duo::<FnMerge>("ZSTD_mergeBlockDelimiters");
                        let mut m1 = sc.clone();
                        let mut m2 = sr.clone();
                        let x = mc(m1.as_mut_ptr(), a);
                        let y = mr(m2.as_mut_ptr(), b);
                        eqv(&format!("{w} mergeBlockDelimiters"), x, y);
                        eqv(&format!("{w} merged sequences"), &m1[..], &m2[..]);
                    }
                }
            }
        }
        // randomized
        for i in 0..80 {
            eqv(
                "row120r reset",
                rc(cctx.c, ZSTD_reset_session_and_parameters),
                rr(cctx.r, ZSTD_reset_session_and_parameters),
            );
            let lvl = rng.range(1, 22);
            eqv(
                "row120r set level",
                sp_c(cctx.c, ZSTD_c_compressionLevel, lvl),
                sp_r(cctx.r, ZSTD_c_compressionLevel, lvl),
            );
            let sz = rng.below(80_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            let cap = bnd(sz).max(1);
            let mut sc = vec![ZSTD_Sequence::default(); cap];
            let mut sr = vec![ZSTD_Sequence::default(); cap];
            let a = gsc(cctx.c, sc.as_mut_ptr(), cap, src.as_ptr() as *const c_void, sz);
            let b = gsr(cctx.r, sr.as_mut_ptr(), cap, src.as_ptr() as *const c_void, sz);
            eqv(&format!("row120r i={i} generateSequences"), a, b);
            eqv(&format!("row120r i={i} sequences"), &sc[..], &sr[..]);
        }
    }
}

// ---------------------------------------------------------------- row 122

#[test]
fn row122_compress_sequences() {
    unsafe {
        let (csc, csr) = duo::<FnCompressSeq>("ZSTD_compressSequences");
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let (pl_c, pl_r) =
            duo::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize>("ZSTD_CCtx_setPledgedSrcSize");
        let cctx = CtxPair::cctx();
        let mut rng = Rng::new(122);

        for delim in [0, 1] {
            for vs in [0, 1] {
                for rr_ in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                    for i in 0..8 {
                        let sz = 200 + rng.below(70_000);
                        let cls = rng.below(N_CLASSES);
                        let src = gen_class(cls, sz, i);
                        let lvl = rng.range(1, 19);
                        let seqs = c_sequences(&src, lvl, delim);
                        if seqs.is_empty() {
                            continue;
                        }
                        eqv(
                            "row122 reset",
                            rc(cctx.c, ZSTD_reset_session_and_parameters),
                            rr(cctx.r, ZSTD_reset_session_and_parameters),
                        );
                        for (p, v) in [
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_blockDelimiters, delim),
                            (ZSTD_c_validateSequences, vs),
                            (ZSTD_c_repcodeResolution, rr_),
                        ] {
                            eqv(
                                &format!("row122 set({p},{v})"),
                                sp_c(cctx.c, p, v),
                                sp_r(cctx.r, p, v),
                            );
                        }
                        eqv(
                            "row122 pledged",
                            pl_c(cctx.c, sz as c_ulonglong),
                            pl_r(cctx.r, sz as c_ulonglong),
                        );
                        let cap = bd(sz) + 64;
                        let mut oc = vec![0x3Bu8; cap];
                        let mut or_ = vec![0x3Bu8; cap];
                        let a = csc(
                            cctx.c,
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let b = csr(
                            cctx.r,
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            src.as_ptr() as *const c_void,
                            sz,
                        );
                        let w = format!(
                            "row122 delim={delim} vs={vs} rr={rr_} i={i} sz={sz} nseq={}",
                            seqs.len()
                        );
                        eqv(&format!("{w} compressSequences"), a, b);
                        eqbuf(&format!("{w} dst"), &oc, &or_);
                        if is_err(a) {
                            continue;
                        }
                        let mut p1 = vec![0u8; sz + 8];
                        let mut p2 = vec![0u8; sz + 8];
                        let x = dc(
                            p1.as_mut_ptr() as *mut c_void,
                            p1.len(),
                            oc.as_ptr() as *const c_void,
                            a,
                        );
                        let y = dr(
                            p2.as_mut_ptr() as *mut c_void,
                            p2.len(),
                            or_.as_ptr() as *const c_void,
                            b,
                        );
                        eqv(&format!("{w} roundtrip"), x, y);
                        eqbuf(&format!("{w} roundtrip dst"), &p1, &p2);
                        // Truncated dst capacities.
                        //
                        // UPSTREAM C MEMORY-SAFETY BUG: with a dstCapacity that
                        // is too small, ZSTD_compressSequences writes *before*
                        // dst (measured: 70 bytes below dst for dstCapacity=10)
                        // before returning dstSize_tooSmall. The Rust port
                        // transliterates the same pointer arithmetic, so both
                        // libraries scribble the same bytes in the same place.
                        // To keep this row differential *and* not corrupt the
                        // test process' heap, both destinations get a 64 KiB
                        // canary guard band on each side and the WHOLE padded
                        // region is compared. See "Upstream C out-of-bounds
                        // writes" in CONFIGS.md.
                        const GUARD: usize = 64 * 1024;
                        for tcap in [a, a - 1, a / 2, 1, 0] {
                            eqv(
                                "row122 reset(trunc)",
                                rc(cctx.c, ZSTD_reset_session_and_parameters),
                                rr(cctx.r, ZSTD_reset_session_and_parameters),
                            );
                            for (p, v) in [
                                (ZSTD_c_compressionLevel, lvl),
                                (ZSTD_c_blockDelimiters, delim),
                                (ZSTD_c_validateSequences, vs),
                                (ZSTD_c_repcodeResolution, rr_),
                            ] {
                                eqv(
                                    &format!("row122 set(trunc)({p},{v})"),
                                    sp_c(cctx.c, p, v),
                                    sp_r(cctx.r, p, v),
                                );
                            }
                            eqv(
                                "row122 pledged(trunc)",
                                pl_c(cctx.c, sz as c_ulonglong),
                                pl_r(cctx.r, sz as c_ulonglong),
                            );
                            let mut q1 = vec![0xAAu8; GUARD + tcap + GUARD];
                            let mut q2 = vec![0xAAu8; GUARD + tcap + GUARD];
                            let x = csc(
                                cctx.c,
                                q1.as_mut_ptr().add(GUARD) as *mut c_void,
                                tcap,
                                seqs.as_ptr(),
                                seqs.len(),
                                src.as_ptr() as *const c_void,
                                sz,
                            );
                            let y = csr(
                                cctx.r,
                                q2.as_mut_ptr().add(GUARD) as *mut c_void,
                                tcap,
                                seqs.as_ptr(),
                                seqs.len(),
                                src.as_ptr() as *const c_void,
                                sz,
                            );
                            eqv(&format!("{w} tcap={tcap}"), x, y);
                            eqbuf(&format!("{w} tcap={tcap} padded dst"), &q1, &q2);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 123

#[test]
fn row123_compress_sequences_and_literals() {
    unsafe {
        let (csc, csr) = duo::<FnCompressSeqLit>("ZSTD_compressSequencesAndLiterals");
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();
        let mut rng = Rng::new(123);
        for delim in [0, 1] {
            for mbs in [0, 1024, 65536, 131072] {
                for i in 0..8 {
                    let sz = 200 + rng.below(60_000);
                    let src = gen_class(rng.below(N_CLASSES), sz, i);
                    let lvl = rng.range(1, 12);
                    let seqs = c_sequences(&src, lvl, delim);
                    if seqs.is_empty() {
                        continue;
                    }
                    // literals = all bytes not covered by matches, in order
                    let mut lits: Vec<u8> = Vec::new();
                    let mut pos = 0usize;
                    for s in &seqs {
                        let ll = s.litLength as usize;
                        if pos + ll > src.len() {
                            break;
                        }
                        lits.extend_from_slice(&src[pos..pos + ll]);
                        pos += ll + s.matchLength as usize;
                        if pos > src.len() {
                            break;
                        }
                    }
                    eqv(
                        "row123 reset",
                        rc(cctx.c, ZSTD_reset_session_and_parameters),
                        rr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    for (p, v) in [
                        (ZSTD_c_compressionLevel, lvl),
                        (ZSTD_c_blockDelimiters, delim),
                        (ZSTD_c_maxBlockSize, mbs),
                    ] {
                        eqv(
                            &format!("row123 set({p},{v})"),
                            sp_c(cctx.c, p, v),
                            sp_r(cctx.r, p, v),
                        );
                    }
                    let cap = bd(sz) + 64;
                    let mut oc = vec![0x7Eu8; cap];
                    let mut or_ = vec![0x7Eu8; cap];
                    for litcap in [lits.len(), lits.len() + 64, lits.len() / 2] {
                        let mut lc = lits.clone();
                        let mut lr = lits.clone();
                        lc.resize(litcap.max(lits.len()), 0);
                        lr.resize(litcap.max(lits.len()), 0);
                        let a = csc(
                            cctx.c,
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            lc.as_ptr() as *const c_void,
                            lits.len(),
                            litcap,
                            sz,
                        );
                        let b = csr(
                            cctx.r,
                            or_.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            lr.as_ptr() as *const c_void,
                            lits.len(),
                            litcap,
                            sz,
                        );
                        let w = format!(
                            "row123 delim={delim} mbs={mbs} i={i} sz={sz} nseq={} litcap={litcap}",
                            seqs.len()
                        );
                        eqv(&format!("{w} compressSequencesAndLiterals"), a, b);
                        eqbuf(&format!("{w} dst"), &oc, &or_);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 124

#[test]
fn row124_bound_convert_summary() {
    unsafe {
        let (bc, br) = duo::<FnSizeT1>("ZSTD_sequenceBound");
        let mut rng = Rng::new(124);
        let mut cases: Vec<usize> = vec![0, 1, 2, 3, 127, 128, 129, 1 << 17, 1 << 20, usize::MAX / 8];
        for _ in 0..500 {
            cases.push(rng.next_u64() as usize >> rng.below(40) as u32);
        }
        for s in cases {
            eqv(&format!("row124 sequenceBound({s})"), bc(s), br(s));
        }

        let (cvc, cvr) = duo::<FnConvert>("ZSTD_convertBlockSequences");
        let (smc, smr) = duo::<FnSummary>("ZSTD_get1BlockSummary");
        let cctx = CtxPair::cctx();
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");

        for i in 0..40 {
            let sz = 500 + rng.below(40_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            for delim in [0, 1] {
                let seqs = c_sequences(&src, 5, delim);
                if seqs.is_empty() {
                    continue;
                }
                let w = format!("row124 i={i} delim={delim} nseq={}", seqs.len());
                // ZSTD_get1BlockSummary is a pure function over the array
                for take in [seqs.len(), seqs.len() / 2, 1, 0] {
                    let a = smc(seqs.as_ptr(), take);
                    let b = smr(seqs.as_ptr(), take);
                    eqv(
                        &format!("{w} get1BlockSummary take={take} nbSequences"),
                        a.nbSequences,
                        b.nbSequences,
                    );
                    // On the error path the C leaves `blockSize` and `litSize`
                    // UNINITIALISED (`BlockSummary bs; bs.nbSequences =
                    // ERROR(externalSequences_invalid); return bs;` -- see
                    // zstd_compress.c:7462). Those two fields are unspecified
                    // in the C and cannot be compared; the Rust zeroes them.
                    if !is_err(a.nbSequences) {
                        eqv(
                            &format!("{w} get1BlockSummary take={take} blockSize"),
                            a.blockSize,
                            b.blockSize,
                        );
                        eqv(
                            &format!("{w} get1BlockSummary take={take} litSize"),
                            a.litSize,
                            b.litSize,
                        );
                    }
                }
                // ZSTD_convertBlockSequences needs a CCtx that has been
                // initialised for a compression session, so run a compression
                // first, then convert the same sequences.
                for rcr in [0, 1] {
                    eqv(
                        "row124 reset",
                        rc(cctx.c, ZSTD_reset_session_and_parameters),
                        rr(cctx.r, ZSTD_reset_session_and_parameters),
                    );
                    eqv(
                        "row124 set delim",
                        sp_c(cctx.c, ZSTD_c_blockDelimiters, delim),
                        sp_r(cctx.r, ZSTD_c_blockDelimiters, delim),
                    );
                    let cap = bd(sz) + 64;
                    let mut oc = vec![0u8; cap];
                    let mut or_ = vec![0u8; cap];
                    let a = c2c(
                        cctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    let b = c2r(
                        cctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    eqv(&format!("{w} warmup compress2"), a, b);
                    eqbuf(&format!("{w} warmup dst"), &oc, &or_);
                    let nb = ZSTD_get1BlockSummary_nb(smc(seqs.as_ptr(), seqs.len()));
                    let x = cvc(cctx.c, seqs.as_ptr(), nb, rcr);
                    let y = cvr(cctx.r, seqs.as_ptr(), nb, rcr);
                    eqv(&format!("{w} convertBlockSequences rcr={rcr} nb={nb}"), x, y);
                }
            }
        }
    }
}

fn ZSTD_get1BlockSummary_nb(s: BlockSummary) -> usize {
    s.nbSequences
}

// ---------------------------------------------------------------- row 125

#[test]
fn row125_ldm_entry_points() {
    unsafe {
        let (tsc, tsr) = duo::<unsafe extern "C" fn(ldmParams_t) -> usize>("ZSTD_ldm_getTableSize");
        let (msc, msr) =
            duo::<unsafe extern "C" fn(ldmParams_t, usize) -> usize>("ZSTD_ldm_getMaxNbSeq");
        let (apc, apr) = duo::<
            unsafe extern "C" fn(*mut ldmParams_t, *const ZSTD_compressionParameters),
        >("ZSTD_ldm_adjustParameters");
        let (skc, skr) = duo::<unsafe extern "C" fn(*mut RawSeqStore_t, usize)>(
            "ZSTD_ldm_skipRawSeqStoreBytes",
        );
        let (ssc, ssr) = duo::<unsafe extern "C" fn(*mut RawSeqStore_t, usize, *const ZSTD_compressionParameters, c_uint)>(
            "ZSTD_ldm_skipSequences",
        );
        let (gcp, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let mut rng = Rng::new(125);

        // pure functions over ldmParams_t
        let mut params: Vec<ldmParams_t> = Vec::new();
        for e in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
            for hl in [0u32, 1, 6, 10, 20, 26, 30] {
                for bsl in [0u32, 1, 4, 8, 9] {
                    params.push(ldmParams_t {
                        enableLdm: e,
                        hashLog: hl,
                        bucketSizeLog: bsl,
                        minMatchLength: 64,
                        hashRateLog: 7,
                        windowLog: 27,
                    });
                }
            }
        }
        for _ in 0..400 {
            params.push(ldmParams_t {
                enableLdm: rng.range(0, 2),
                hashLog: rng.range(0, 30) as u32,
                bucketSizeLog: rng.range(0, 10) as u32,
                // NOTE: minMatchLength must stay > 0; ZSTD_ldm_getMaxNbSeq
                // divides by it and the C library dies with SIGFPE at 0 (see
                // "Upstream C crashes" in CONFIGS.md).
                minMatchLength: rng.range(1, 4096) as u32,
                hashRateLog: rng.range(0, 24) as u32,
                windowLog: rng.range(10, 31) as u32,
            });
        }
        for p in &params {
            eqv(&format!("row125 getTableSize({p:?})"), tsc(*p), tsr(*p));
            for chunk in [0usize, 1, 1024, 1 << 17, 1 << 20] {
                eqv(
                    &format!("row125 getMaxNbSeq({p:?},{chunk})"),
                    msc(*p, chunk),
                    msr(*p, chunk),
                );
            }
        }
        // adjustParameters against every cParams row
        for lvl in [-5, 1, 3, 9, 15, 19, 22] {
            for ss in [0u64, 1024, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                let cp = gcp(lvl, ss, 0);
                for p in params.iter().take(80) {
                    let mut a = *p;
                    let mut b = *p;
                    apc(&mut a, &cp);
                    apr(&mut b, &cp);
                    eqv(
                        &format!("row125 adjustParameters({p:?},{cp:?})"),
                        a,
                        b,
                    );
                }
            }
        }

        // RawSeqStore_t skipping
        for i in 0..60 {
            let n = 1 + rng.below(40);
            let mut seqs: Vec<rawSeq> = (0..n)
                .map(|_| rawSeq {
                    offset: rng.range(1, 100_000) as u32,
                    litLength: rng.range(0, 200) as u32,
                    matchLength: rng.range(3, 500) as u32,
                })
                .collect();
            let mut sc = seqs.clone();
            let mut sr = seqs.clone();
            let mut a = RawSeqStore_t {
                seq: sc.as_mut_ptr(),
                pos: 0,
                posInSequence: 0,
                size: n,
                capacity: n,
            };
            let mut b = RawSeqStore_t {
                seq: sr.as_mut_ptr(),
                pos: 0,
                posInSequence: 0,
                size: n,
                capacity: n,
            };
            for _ in 0..6 {
                let nb = rng.below(300);
                skc(&mut a, nb);
                skr(&mut b, nb);
                eqv(&format!("row125 skipRawSeqStoreBytes i={i} nb={nb} pos"), a.pos, b.pos);
                eqv(
                    &format!("row125 skipRawSeqStoreBytes i={i} nb={nb} posInSequence"),
                    a.posInSequence,
                    b.posInSequence,
                );
                eqv(&format!("row125 seq array i={i}"), &sc[..], &sr[..]);
            }
            // ZSTD_ldm_skipSequences
            let cp = gcp(5, 0, 0);
            let mut sc2 = seqs.clone();
            let mut sr2 = seqs.clone();
            let mut a = RawSeqStore_t {
                seq: sc2.as_mut_ptr(),
                pos: 0,
                posInSequence: 0,
                size: n,
                capacity: n,
            };
            let mut b = RawSeqStore_t {
                seq: sr2.as_mut_ptr(),
                pos: 0,
                posInSequence: 0,
                size: n,
                capacity: n,
            };
            for mls in [3u32, 4, 5, 6, 7] {
                let sz = rng.below(3000);
                ssc(&mut a, sz, &cp, mls);
                ssr(&mut b, sz, &cp, mls);
                eqv(&format!("row125 skipSequences i={i} mls={mls} pos"), a.pos, b.pos);
                eqv(
                    &format!("row125 skipSequences i={i} mls={mls} posInSequence"),
                    a.posInSequence,
                    b.posInSequence,
                );
                eqv(&format!("row125 skipSequences seq array i={i}"), &sc2[..], &sr2[..]);
            }
            let _ = &mut seqs;
        }

        // ZSTD_ldm_generateSequences / blockCompress / fillHashTable are driven
        // end-to-end (they need a fully initialised ldmState_t / MatchState_t,
        // whose layout is private); the LDM parameter grid in
        // tests/phase_b_params.rs::row38_ldm_grid exercises them through
        // ZSTD_compress2 with LDM enabled.
        let (sp_c, sp_r) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = duo::<FnReset>("ZSTD_CCtx_reset");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let cctx = CtxPair::cctx();
        let bait = gen_class(5, 1_500_000, 125);
        for lvl in [1, 5, 12, 19] {
            for mml in [4, 16, 64, 200, 4096] {
                eqv(
                    "row125 reset",
                    rc(cctx.c, ZSTD_reset_session_and_parameters),
                    rr(cctx.r, ZSTD_reset_session_and_parameters),
                );
                for (p, v) in [
                    (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                    (ZSTD_c_ldmMinMatch, mml),
                    (ZSTD_c_compressionLevel, lvl),
                ] {
                    eqv(
                        &format!("row125 set({p},{v})"),
                        sp_c(cctx.c, p, v),
                        sp_r(cctx.r, p, v),
                    );
                }
                let cap = bd(bait.len()) + 64;
                let mut oc = vec![0u8; cap];
                let mut or_ = vec![0u8; cap];
                let a = c2c(
                    cctx.c,
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    bait.as_ptr() as *const c_void,
                    bait.len(),
                );
                let b = c2r(
                    cctx.r,
                    or_.as_mut_ptr() as *mut c_void,
                    cap,
                    bait.as_ptr() as *const c_void,
                    bait.len(),
                );
                let w = format!("row125 ldm e2e lvl={lvl} mml={mml}");
                eqv(&format!("{w} compress2"), a, b);
                eqbuf(&format!("{w} dst"), &oc, &or_);
                assert!(!is_err(a));
                let mut p1 = vec![0u8; bait.len() + 8];
                let mut p2 = vec![0u8; bait.len() + 8];
                let x = dc(
                    p1.as_mut_ptr() as *mut c_void,
                    p1.len(),
                    oc.as_ptr() as *const c_void,
                    a,
                );
                let y = dr(
                    p2.as_mut_ptr() as *mut c_void,
                    p2.len(),
                    or_.as_ptr() as *const c_void,
                    b,
                );
                eqv(&format!("{w} roundtrip"), x, y);
                eqbuf(&format!("{w} roundtrip dst"), &p1, &p2);
                assert_eq!(&p1[..x], &bait[..]);
            }
        }
    }
}
