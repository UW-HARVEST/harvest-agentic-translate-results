//! Phase B — differential tests for the ZSTD **SEQUENCE** API surface.
//!
//! Covers the "Block level Sequence Producer API" and the frame-level
//! `ZSTD_compressSequences*` family, plus the sequence generation / merge
//! helpers.  Every call is dispatched through `dlsym` on BOTH the C and the
//! Rust `libzstd.so` (via the `fnpair!` macro) so we exercise the
//! `#[no_mangle]` FFI wrappers only — no Rust function is ever called directly.
//!
//! Entry points differentially tested here:
//!   1. `ZSTD_sequenceBound`
//!   2. `ZSTD_generateSequences`
//!   3. `ZSTD_mergeBlockDelimiters`
//!   4. `ZSTD_compressSequences`
//!   5. `ZSTD_compressSequencesAndLiterals`
//!   6. `ZSTD_referenceExternalSequences`
//!   7. `ZSTD_registerSequenceProducer` / `ZSTD_CCtxParams_registerSequenceProducer`
//!
//! All randomized inputs use a FIXED seed so runs are reproducible.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// --------------------------------------------------------------- FFI types ---

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, u64) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDCtxDecompress =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

// ZSTD_generateSequences(zc, outSeqs, outSeqsCapacity, src, srcSize)
type FnGenerateSequences =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, size_t, *const c_void, size_t) -> size_t;

// ZSTD_mergeBlockDelimiters(sequences, seqsSize)
type FnMergeBlockDelimiters = unsafe extern "C" fn(*mut ZSTD_Sequence, size_t) -> size_t;

// ZSTD_compressSequences(cctx, dst, dstCapacity, inSeqs, inSeqsSize, src, srcSize)
type FnCompressSequences = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const ZSTD_Sequence,
    size_t,
    *const c_void,
    size_t,
) -> size_t;

// ZSTD_compressSequencesAndLiterals(cctx, dst, dstCapacity, inSeqs, nbSequences,
//                                   literals, litSize, litBufCapacity, decompressedSize)
type FnCompressSequencesAndLiterals = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const ZSTD_Sequence,
    size_t,
    *const c_void,
    size_t,
    size_t,
    size_t,
) -> size_t;

// The block-level sequence producer callback type (from zstd.h).
type ZSTD_sequenceProducer_F = unsafe extern "C" fn(
    *mut c_void,        // sequenceProducerState
    *mut ZSTD_Sequence, // outSeqs
    size_t,             // outSeqsCapacity
    *const c_void,      // src
    size_t,             // srcSize
    *const c_void,      // dict
    size_t,             // dictSize
    c_int,              // compressionLevel
    size_t,             // windowSize
) -> size_t;

// ZSTD_registerSequenceProducer(cctx, state, fn)  -> void
type FnRegisterSeqProd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, Option<ZSTD_sequenceProducer_F>);

// ZSTD_referenceExternalSequences(cctx, rawSeq*, nbSeq) -> void.
// NOTE: `rawSeq` (from zstd_compress_internal.h) is a *different* struct than
// the public `ZSTD_Sequence` — it has NO `rep` field:
//     typedef struct { U32 offset; U32 litLength; U32 matchLength; } rawSeq;
// (verified against c_src/src/compress/zstd_compress_internal.h line ~198).
type FnReferenceExternalSequences = unsafe extern "C" fn(*mut c_void, *mut RawSeq, size_t);

// Buffer-less compression API used to drive referenceExternalSequences.
type FnCompressBegin = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCCtxContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

// ------------------------------------------------------------ local structs ---

/// Mirror of the internal `rawSeq` struct.  Do NOT confuse with `ZSTD_Sequence`
/// (defined in tests/common/mod.rs, which has an extra `rep` field).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct RawSeq {
    offset: c_uint,
    litLength: c_uint,
    matchLength: c_uint,
}

// --------------------------------------------------------------------- API ---

struct Api {
    create_cctx: (FnCreate, FnCreate),
    free_cctx: (FnFree, FnFree),
    create_dctx: (FnCreate, FnCreate),
    free_dctx: (FnFree, FnFree),
    #[allow(dead_code)]
    reset: (FnReset, FnReset),
    set_param: (FnSetParam, FnSetParam),
    set_pledged: (FnSetPledged, FnSetPledged),
    compress2: (FnCompress2, FnCompress2),
    seq_bound: (FnSizeSize, FnSizeSize),
    gen_seqs: (FnGenerateSequences, FnGenerateSequences),
    merge_delims: (FnMergeBlockDelimiters, FnMergeBlockDelimiters),
    comp_seqs: (FnCompressSequences, FnCompressSequences),
    comp_seqs_lit: (FnCompressSequencesAndLiterals, FnCompressSequencesAndLiterals),
    ref_ext_seqs: (FnReferenceExternalSequences, FnReferenceExternalSequences),
    reg_seqprod: (FnRegisterSeqProd, FnRegisterSeqProd),
    comp_begin: (FnCompressBegin, FnCompressBegin),
    comp_end: (FnCCtxContinue, FnCCtxContinue),
    comp_bound: (FnSizeSize, FnSizeSize),
    is_error: (FnIsError, FnIsError),
    err_code: (FnGetErrorCode, FnGetErrorCode),
    decompress_dctx: (FnDCtxDecompress, FnDCtxDecompress),
}

fn api() -> Api {
    Api {
        create_cctx: fnpair!("ZSTD_createCCtx", FnCreate),
        free_cctx: fnpair!("ZSTD_freeCCtx", FnFree),
        create_dctx: fnpair!("ZSTD_createDCtx", FnCreate),
        free_dctx: fnpair!("ZSTD_freeDCtx", FnFree),
        reset: fnpair!("ZSTD_CCtx_reset", FnReset),
        set_param: fnpair!("ZSTD_CCtx_setParameter", FnSetParam),
        set_pledged: fnpair!("ZSTD_CCtx_setPledgedSrcSize", FnSetPledged),
        compress2: fnpair!("ZSTD_compress2", FnCompress2),
        seq_bound: fnpair!("ZSTD_sequenceBound", FnSizeSize),
        gen_seqs: fnpair!("ZSTD_generateSequences", FnGenerateSequences),
        merge_delims: fnpair!("ZSTD_mergeBlockDelimiters", FnMergeBlockDelimiters),
        comp_seqs: fnpair!("ZSTD_compressSequences", FnCompressSequences),
        comp_seqs_lit: fnpair!("ZSTD_compressSequencesAndLiterals", FnCompressSequencesAndLiterals),
        ref_ext_seqs: fnpair!("ZSTD_referenceExternalSequences", FnReferenceExternalSequences),
        reg_seqprod: fnpair!("ZSTD_registerSequenceProducer", FnRegisterSeqProd),
        comp_begin: fnpair!("ZSTD_compressBegin", FnCompressBegin),
        comp_end: fnpair!("ZSTD_compressEnd", FnCCtxContinue),
        comp_bound: fnpair!("ZSTD_compressBound", FnSizeSize),
        is_error: fnpair!("ZSTD_isError", FnIsError),
        err_code: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        decompress_dctx: fnpair!("ZSTD_decompressDCtx", FnDCtxDecompress),
    }
}

// ---------------------------------------------------------------- helpers ---

fn ptr_or_dangling(b: &[u8]) -> *const c_void {
    if b.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
    } else {
        b.as_ptr() as *const c_void
    }
}

/// Assert two `ZSTD_Sequence` arrays (the first `count` entries) match exactly,
/// field for field, with a descriptive context.
#[track_caller]
fn assert_seqs_eq(ctx: &str, c: &[ZSTD_Sequence], r: &[ZSTD_Sequence], count: usize) {
    assert!(count <= c.len() && count <= r.len(), "{ctx}: count {count} out of range");
    for i in 0..count {
        let sc = c[i];
        let sr = r[i];
        assert_eq!(
            sc, sr,
            "{ctx}: sequence[{i}] differs C={{off:{},ll:{},ml:{},rep:{}}} R={{off:{},ll:{},ml:{},rep:{}}}",
            sc.offset, sc.litLength, sc.matchLength, sc.rep,
            sr.offset, sr.litLength, sr.matchLength, sr.rep,
        );
    }
}

/// Assert exact error parity for two return codes.
#[track_caller]
fn assert_err_parity(a: &Api, ctx: &str, rc: size_t, rr: size_t) {
    unsafe {
        assert_eq!(
            (a.is_error.0)(rc),
            (a.is_error.1)(rr),
            "{ctx}: isError differs (C={rc:#x} R={rr:#x})"
        );
        if (a.is_error.0)(rc) != 0 {
            assert_eq!(
                (a.err_code.0)(rc),
                (a.err_code.1)(rr),
                "{ctx}: error code differs (C={} R={})",
                (a.err_code.0)(rc),
                (a.err_code.1)(rr)
            );
        }
    }
}

/// Run `ZSTD_generateSequences` on both libraries with a fresh CCtx configured
/// via `params`, into a buffer pre-filled with a recognizable sentinel pattern.
/// Returns `(count, seqs_c, seqs_r)` on success (parity already asserted), or
/// `None` if BOTH errored identically.
#[track_caller]
fn gen_both(
    a: &Api,
    params: &[(c_int, c_int)],
    src: &[u8],
    cap: usize,
    ctx: &str,
) -> Option<(usize, Vec<ZSTD_Sequence>, Vec<ZSTD_Sequence>)> {
    unsafe {
        let cctx_c = (a.create_cctx.0)();
        let cctx_r = (a.create_cctx.1)();
        assert!(!cctx_c.is_null() && !cctx_r.is_null(), "{ctx}: createCCtx null");
        for &(p, v) in params {
            let rc = (a.set_param.0)(cctx_c, p, v);
            let rr = (a.set_param.1)(cctx_r, p, v);
            assert_err_parity(a, &format!("{ctx}: setParam({p},{v})"), rc, rr);
        }

        // Recognizable fill pattern so the tail beyond `count` is deterministic.
        let sentinel = ZSTD_Sequence {
            offset: 0xDEAD_BEEF,
            litLength: 0xCAFE_BABE,
            matchLength: 0x1234_5678,
            rep: 0xA5A5_A5A5,
        };
        let mut sc = vec![sentinel; cap];
        let mut sr = vec![sentinel; cap];

        let nc = (a.gen_seqs.0)(cctx_c, sc.as_mut_ptr(), cap, ptr_or_dangling(src), src.len());
        let nr = (a.gen_seqs.1)(cctx_r, sr.as_mut_ptr(), cap, ptr_or_dangling(src), src.len());

        (a.free_cctx.0)(cctx_c);
        (a.free_cctx.1)(cctx_r);

        assert_err_parity(a, &format!("{ctx}: generateSequences"), nc, nr);
        if (a.is_error.0)(nc) != 0 {
            return None;
        }
        assert_eq!(nc, nr, "{ctx}: generateSequences count differs (C={nc} R={nr})");
        // The tail beyond `count` is genuinely undefined per the API contract
        // (only the first `count` entries are meaningful), so compare only the
        // returned prefix, field-for-field.
        assert_seqs_eq(&format!("{ctx}: generated seqs"), &sc, &sr, nc);
        Some((nc, sc, sr))
    }
}

/// Decompress `frame` with the given library's DCtx and confirm it equals `src`.
#[track_caller]
fn roundtrip_check(a: &Api, which_c: bool, frame: &[u8], src: &[u8], ctx: &str) {
    unsafe {
        let (create, free, decomp, is_err) = if which_c {
            (a.create_dctx.0, a.free_dctx.0, a.decompress_dctx.0, a.is_error.0)
        } else {
            (a.create_dctx.1, a.free_dctx.1, a.decompress_dctx.1, a.is_error.1)
        };
        let dctx = create();
        assert!(!dctx.is_null(), "{ctx}: createDCtx null");
        let mut out = vec![0u8; src.len() + 64];
        let outp = if out.is_empty() {
            std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
        } else {
            out.as_mut_ptr() as *mut c_void
        };
        let d = decomp(dctx, outp, out.len(), frame.as_ptr() as *const c_void, frame.len());
        free(dctx);
        assert_eq!(is_err(d), 0, "{ctx}: decompress failed (rc={d:#x})");
        assert_eq!(d, src.len(), "{ctx}: decompressed size differs");
        assert_bytes_eq(&format!("{ctx}: decompressed bytes"), src, &out[..d]);
    }
}

// ============================================================================
//  1. ZSTD_sequenceBound
// ============================================================================

#[test]
fn seq_bound_exact() {
    let a = api();
    let mut rng = Rng::new(0x5E9B0007);

    let mut sizes: Vec<usize> = vec![
        0,
        1,
        2,
        3,
        63,
        64,
        1024,
        65535,
        65536,
        131071, // 128KB - 1
        131072, // 128KB
        131073, // 128KB + 1
        1 << 20,
        (1 << 20) + 1,
        3 * (1 << 20),
        7 * (1 << 20) + 12345,
    ];
    for _ in 0..2000 {
        sizes.push(rng.below(8 * (1 << 20)));
    }

    unsafe {
        for &s in &sizes {
            let bc = (a.seq_bound.0)(s);
            let br = (a.seq_bound.1)(s);
            assert_eq!(bc, br, "sequenceBound({s}) differs: C={bc} R={br}");
        }
    }
}

// ============================================================================
//  2. ZSTD_generateSequences (+ undersized outSeqs capacity)
// ============================================================================

/// Parameter matrix rows for generateSequences.  generateSequences forbids
/// targetCBlockSize != 0 and nbWorkers != 0, so those stay out of this matrix.
fn gen_cparam_matrix() -> Vec<Vec<(c_int, c_int)>> {
    let strategies = [
        ZSTD_fast, ZSTD_dfast, ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_btlazy2, ZSTD_btopt,
        ZSTD_btultra, ZSTD_btultra2,
    ];
    let mut out = Vec::new();
    for lvl in [-5, 1, 3, 9, 12, 19] {
        out.push(vec![(ZSTD_c_compressionLevel, lvl)]);
    }
    for &st in &strategies {
        out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_strategy, st)]);
    }
    out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_windowLog, 10)]);
    out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_windowLog, 18)]);
    out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_minMatch, 3)]);
    out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_minMatch, 6)]);
    out.push(vec![
        (ZSTD_c_compressionLevel, 9),
        (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
    ]);
    out.push(vec![
        (ZSTD_c_compressionLevel, 9),
        (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_disable),
    ]);
    for urm in [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        out.push(vec![(ZSTD_c_compressionLevel, 6), (ZSTD_c_useRowMatchFinder, urm)]);
    }
    out
}

#[test]
fn generate_sequences_shapes_and_params() {
    let a = api();
    let mut rng = Rng::new(0x9E10);
    let lens = [0usize, 1, 3, 64, 1024, 65536, 131072, 131073, 300000];
    let matrix = gen_cparam_matrix();

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                let cap = (a.seq_bound.0)(len).max(1);
                assert_eq!(cap, (a.seq_bound.1)(len), "seqBound mismatch len={len}");

                for params in &matrix {
                    let ctx = format!("gen shape={shape:?} len={len} params={params:?}");
                    gen_both(&a, params, &src, cap, &ctx);
                }
            }
        }

        // Undersized outSeqs capacity: pick a shape/len that produces several
        // sequences, then request a capacity smaller than needed and confirm
        // both libraries agree (both error, or both truncate identically).
        let src = gen(Shape::Text, 4096, &mut rng);
        let full_cap = (a.seq_bound.0)(src.len()).max(1);
        let params = vec![(ZSTD_c_compressionLevel, 6)];
        if let Some((count, _, _)) = gen_both(&a, &params, &src, full_cap, "undersized-baseline") {
            if count > 1 {
                for undersized in [1usize, count / 2, count.saturating_sub(1)] {
                    let undersized = undersized.max(1);
                    let ctx = format!("gen undersized cap={undersized} (needs {count})");
                    gen_both(&a, &params, &src, undersized, &ctx);
                }
            }
        }
    }
}

// ============================================================================
//  3. ZSTD_mergeBlockDelimiters
// ============================================================================

#[test]
fn merge_block_delimiters() {
    let a = api();
    let mut rng = Rng::new(0x0EDE_1157);
    let lens = [1usize, 3, 64, 1024, 65536, 131073];
    let params = vec![(ZSTD_c_compressionLevel, 7)];

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                let cap = (a.seq_bound.0)(len).max(1);
                let ctx = format!("merge shape={shape:?} len={len}");
                let (count, sc, sr) = match gen_both(&a, &params, &src, cap, &ctx) {
                    Some(t) => t,
                    None => continue,
                };
                // Feed the EXACT generated arrays (explicit block delimiters).
                let mut mc = sc.clone();
                let mut mr = sr.clone();
                let nc = (a.merge_delims.0)(mc.as_mut_ptr(), count);
                let nr = (a.merge_delims.1)(mr.as_mut_ptr(), count);
                assert_eq!(nc, nr, "{ctx}: mergeBlockDelimiters count differs (C={nc} R={nr})");
                // Compare the mutated array field-for-field over the returned count.
                assert_seqs_eq(&format!("{ctx}: merged seqs"), &mc, &mr, nc);
            }
        }
    }
}

// ============================================================================
//  4. ZSTD_compressSequences
// ============================================================================

/// compressSequences differential driver for one config.
#[track_caller]
fn diff_compress_sequences(
    a: &Api,
    src: &[u8],
    seqs: &[ZSTD_Sequence],
    nb_seqs: usize,
    params: &[(c_int, c_int)],
    ctx: &str,
) {
    unsafe {
        let cap = (a.comp_bound.0)(src.len()).max(64);
        let cctx_c = (a.create_cctx.0)();
        let cctx_r = (a.create_cctx.1)();
        for &(p, v) in params {
            let rc = (a.set_param.0)(cctx_c, p, v);
            let rr = (a.set_param.1)(cctx_r, p, v);
            assert_err_parity(a, &format!("{ctx}: setParam({p},{v})"), rc, rr);
        }

        // Pre-fill output with 0xAA identically on both sides.
        let mut oc = vec![0xAAu8; cap];
        let mut or = vec![0xAAu8; cap];

        let nc = (a.comp_seqs.0)(
            cctx_c,
            oc.as_mut_ptr() as *mut c_void,
            cap,
            seqs.as_ptr(),
            nb_seqs,
            ptr_or_dangling(src),
            src.len(),
        );
        let nr = (a.comp_seqs.1)(
            cctx_r,
            or.as_mut_ptr() as *mut c_void,
            cap,
            seqs.as_ptr(),
            nb_seqs,
            ptr_or_dangling(src),
            src.len(),
        );
        (a.free_cctx.0)(cctx_c);
        (a.free_cctx.1)(cctx_r);

        assert_err_parity(a, &format!("{ctx}: compressSequences"), nc, nr);
        if (a.is_error.0)(nc) != 0 {
            return;
        }
        assert_eq!(nc, nr, "{ctx}: compressSequences size differs (C={nc} R={nr})");
        // Compare the FULL output buffer (including the 0xAA tail beyond nc).
        assert_bytes_eq(&format!("{ctx}: compressSequences full buffer"), &oc, &or);

        // Frame decompresses to the original on BOTH libs, and cross-wise.
        roundtrip_check(a, true, &oc[..nc], src, &format!("{ctx}: C-decode-C"));
        roundtrip_check(a, false, &or[..nr], src, &format!("{ctx}: R-decode-R"));
        roundtrip_check(a, false, &oc[..nc], src, &format!("{ctx}: R-decode-C"));
        roundtrip_check(a, true, &or[..nr], src, &format!("{ctx}: C-decode-R"));
    }
}

#[test]
fn compress_sequences_matrix() {
    let a = api();
    let mut rng = Rng::new(0xC0FFEE11);
    let lens = [1usize, 64, 1024, 65536, 131073, 300000];
    let gen_params = vec![(ZSTD_c_compressionLevel, 6)];

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                let cap = (a.seq_bound.0)(len).max(1);
                let ctx0 = format!("compSeq shape={shape:?} len={len}");
                // Raw generated seqs (explicit block delimiters).
                let (count, sc, _sr) = match gen_both(&a, &gen_params, &src, cap, &ctx0) {
                    Some(t) => t,
                    None => continue,
                };
                let explicit_seqs = sc.clone();

                // Merged version (no block delimiters).
                let mut merged = sc.clone();
                let merged_count = (a.merge_delims.0)(merged.as_mut_ptr(), count);

                for &(delim, seqs, nb) in &[
                    (ZSTD_sf_explicitBlockDelimiters, &explicit_seqs, count),
                    (ZSTD_sf_noBlockDelimiters, &merged, merged_count),
                ] {
                    for &validate in &[0, 1] {
                        for &repcode in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
                            let combos: Vec<Vec<(c_int, c_int)>> = vec![
                                vec![
                                    (ZSTD_c_blockDelimiters, delim),
                                    (ZSTD_c_validateSequences, validate),
                                    (ZSTD_c_repcodeResolution, repcode),
                                    (ZSTD_c_compressionLevel, 6),
                                ],
                                vec![
                                    (ZSTD_c_blockDelimiters, delim),
                                    (ZSTD_c_validateSequences, validate),
                                    (ZSTD_c_repcodeResolution, repcode),
                                    (ZSTD_c_compressionLevel, 3),
                                    (ZSTD_c_checksumFlag, 1),
                                    (ZSTD_c_contentSizeFlag, 1),
                                ],
                                vec![
                                    (ZSTD_c_blockDelimiters, delim),
                                    (ZSTD_c_validateSequences, validate),
                                    (ZSTD_c_repcodeResolution, repcode),
                                    (ZSTD_c_compressionLevel, 9),
                                    (ZSTD_c_windowLog, 18),
                                    (ZSTD_c_maxBlockSize, 1 << 16),
                                ],
                                vec![
                                    (ZSTD_c_blockDelimiters, delim),
                                    (ZSTD_c_validateSequences, validate),
                                    (ZSTD_c_repcodeResolution, repcode),
                                    (ZSTD_c_compressionLevel, 12),
                                    (ZSTD_c_targetCBlockSize, 2048),
                                ],
                            ];
                            for params in &combos {
                                let ctx = format!(
                                    "{ctx0} delim={delim} validate={validate} rep={repcode} params={params:?}"
                                );
                                diff_compress_sequences(&a, &src, seqs, nb, params, &ctx);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
//  5. ZSTD_compressSequencesAndLiterals
// ============================================================================

/// Assemble the literals buffer from `src` per the sequence array's litLength
/// fields (explicit delimiter mode).  Returns (literals, litSize,
/// decompressedSize).  `src` is consumed left-to-right: each sequence eats
/// `litLength` literals then `matchLength` match bytes.
fn assemble_literals(src: &[u8], seqs: &[ZSTD_Sequence], count: usize) -> (Vec<u8>, usize, usize) {
    let mut lits = Vec::new();
    let mut pos = 0usize;
    let mut decompressed = 0usize;
    for s in seqs.iter().take(count) {
        let ll = s.litLength as usize;
        let ml = s.matchLength as usize;
        let end = (pos + ll).min(src.len());
        lits.extend_from_slice(&src[pos..end]);
        pos = (pos + ll).min(src.len());
        pos = (pos + ml).min(src.len());
        decompressed += ll + ml;
    }
    let lit_size = lits.len();
    (lits, lit_size, decompressed)
}

#[test]
fn compress_sequences_and_literals_matrix() {
    let a = api();
    let mut rng = Rng::new(0x117E2A15);
    let lens = [1usize, 64, 1024, 65536, 131073];
    // Must be explicit-delimiter mode; checksum must be disabled; validation
    // unsupported (must be 0).  A known pledged src size == decompressedSize.
    let gen_params = vec![(ZSTD_c_compressionLevel, 6)];

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                let cap = (a.seq_bound.0)(len).max(1);
                let ctx0 = format!("compSeqLit shape={shape:?} len={len}");
                let (count, sc, _sr) = match gen_both(&a, &gen_params, &src, cap, &ctx0) {
                    Some(t) => t,
                    None => continue,
                };
                let (lits, lit_size, decompressed) = assemble_literals(&src, &sc, count);
                // litBufCapacity must be >= litSize + 8.
                let lit_cap = lit_size + 8;
                let mut lit_buf = vec![0u8; lit_cap];
                lit_buf[..lit_size].copy_from_slice(&lits);

                let combos: Vec<Vec<(c_int, c_int)>> = vec![
                    vec![
                        (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                        (ZSTD_c_compressionLevel, 6),
                    ],
                    vec![
                        (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                        (ZSTD_c_compressionLevel, 9),
                        (ZSTD_c_windowLog, 18),
                    ],
                    vec![
                        (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                        (ZSTD_c_compressionLevel, 3),
                        (ZSTD_c_repcodeResolution, ZSTD_ps_enable),
                    ],
                    vec![
                        (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                        (ZSTD_c_compressionLevel, 6),
                        (ZSTD_c_maxBlockSize, 1 << 16),
                    ],
                ];

                for params in &combos {
                    let ctx = format!("{ctx0} params={params:?}");
                    let cap_out = (a.comp_bound.0)(src.len()).max(64);
                    let cctx_c = (a.create_cctx.0)();
                    let cctx_r = (a.create_cctx.1)();
                    // pledged src size must equal decompressedSize.
                    let _ = (a.set_pledged.0)(cctx_c, decompressed as u64);
                    let _ = (a.set_pledged.1)(cctx_r, decompressed as u64);
                    for &(p, v) in params {
                        let rc = (a.set_param.0)(cctx_c, p, v);
                        let rr = (a.set_param.1)(cctx_r, p, v);
                        assert_err_parity(&a, &format!("{ctx}: setParam({p},{v})"), rc, rr);
                    }

                    let mut oc = vec![0xAAu8; cap_out];
                    let mut or = vec![0xAAu8; cap_out];

                    let lit_ptr = lit_buf.as_ptr() as *const c_void;

                    let nc = (a.comp_seqs_lit.0)(
                        cctx_c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap_out,
                        sc.as_ptr(),
                        count,
                        lit_ptr,
                        lit_size,
                        lit_cap,
                        decompressed,
                    );
                    let nr = (a.comp_seqs_lit.1)(
                        cctx_r,
                        or.as_mut_ptr() as *mut c_void,
                        cap_out,
                        sc.as_ptr(),
                        count,
                        lit_ptr,
                        lit_size,
                        lit_cap,
                        decompressed,
                    );
                    (a.free_cctx.0)(cctx_c);
                    (a.free_cctx.1)(cctx_r);

                    // Compare error codes exactly (C may reject incompressible input).
                    assert_err_parity(&a, &format!("{ctx}: compressSequencesAndLiterals"), nc, nr);
                    if (a.is_error.0)(nc) != 0 {
                        continue;
                    }
                    assert_eq!(nc, nr, "{ctx}: size differs (C={nc} R={nr})");
                    assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
                    // decompressedSize corresponds to the sum of sequences, which
                    // may be < len when the trailing literals are not represented.
                    if decompressed > 0 {
                        roundtrip_check(&a, true, &oc[..nc], &src[..decompressed], &format!("{ctx}: C"));
                        roundtrip_check(&a, false, &or[..nr], &src[..decompressed], &format!("{ctx}: R"));
                    }
                }
            }
        }
    }
}

// ============================================================================
//  6. ZSTD_referenceExternalSequences
// ============================================================================

// This is a low-level, void-returning API whose docstring warns that
// "seqs are not verified! Invalid sequences can cause out-of-bounds memory
// access and data corruption." To avoid UB we only reference the always-valid
// empty store (NULL, 0) — exactly how the library itself calls it to clear the
// store (see ZSTD_compress.c). We then drive a buffer-less compression on both
// libraries and compare the produced frame bytes exactly. This exercises the
// FFI symbol and its state-setting effect on the compress path in both
// directions. (The C prototype returns void, so there is no return value to
// compare; we compare the observable effect — the compressed frame.)
#[test]
fn reference_external_sequences_bufferless() {
    let a = api();
    let mut rng = Rng::new(0x5EEDEF);
    let lens = [1usize, 64, 1024, 65536, 200000];

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                for level in [1i32, 6, 12] {
                    let ctx = format!("refExtSeq shape={shape:?} len={len} lvl={level}");
                    let cap = (a.comp_bound.0)(src.len()).max(64);

                    let cctx_c = (a.create_cctx.0)();
                    let cctx_r = (a.create_cctx.1)();

                    // Begin a buffer-less compression, then clear the external
                    // sequence store (NULL, 0) — always valid.
                    let bc = (a.comp_begin.0)(cctx_c, level);
                    let br = (a.comp_begin.1)(cctx_r, level);
                    assert_err_parity(&a, &format!("{ctx}: compressBegin"), bc, br);

                    (a.ref_ext_seqs.0)(cctx_c, std::ptr::null_mut(), 0);
                    (a.ref_ext_seqs.1)(cctx_r, std::ptr::null_mut(), 0);

                    let mut oc = vec![0xAAu8; cap];
                    let mut or = vec![0xAAu8; cap];

                    let ec = (a.comp_end.0)(
                        cctx_c,
                        oc.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr_or_dangling(&src),
                        src.len(),
                    );
                    let er = (a.comp_end.1)(
                        cctx_r,
                        or.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr_or_dangling(&src),
                        src.len(),
                    );
                    (a.free_cctx.0)(cctx_c);
                    (a.free_cctx.1)(cctx_r);

                    assert_err_parity(&a, &format!("{ctx}: compressEnd"), ec, er);
                    if (a.is_error.0)(ec) != 0 {
                        continue;
                    }
                    assert_eq!(ec, er, "{ctx}: compressEnd size differs (C={ec} R={er})");
                    assert_bytes_eq(&format!("{ctx}: frame bytes"), &oc, &or);
                    // Confirm both frames decode back to src, cross-wise.
                    roundtrip_check(&a, true, &oc[..ec], &src, &format!("{ctx}: C-decode-C"));
                    roundtrip_check(&a, false, &or[..er], &src, &format!("{ctx}: R-decode-R"));
                    roundtrip_check(&a, false, &oc[..ec], &src, &format!("{ctx}: R-decode-C"));
                }
            }
        }
    }
}

// ============================================================================
//  7. ZSTD_registerSequenceProducer / ZSTD_CCtxParams_registerSequenceProducer
// ============================================================================

// A deterministic external sequence producer callback, registered into BOTH
// libraries. It ALWAYS returns an error code (via the ZSTD_SEQUENCE_PRODUCER_ERROR
// convention: any value > outSeqsCapacity is treated as an error). Because it is
// identical code registered into both C and Rust CCtxs, both libraries must
// observe the same behavior — either falling back to the internal producer
// (fallback == 1) or failing compression (fallback == 0), producing identical
// output/errors.
//
// This exercises the C-calls-back-into-caller path across the FFI boundary in
// BOTH directions: the C libzstd calling our Rust callback, and the Rust
// libzstd calling the very same callback.
unsafe extern "C" fn seqprod_always_error(
    _state: *mut c_void,
    _out: *mut ZSTD_Sequence,
    out_cap: size_t,
    _src: *const c_void,
    _src_size: size_t,
    _dict: *const c_void,
    _dict_size: size_t,
    _level: c_int,
    _window: size_t,
) -> size_t {
    // Any value strictly greater than out_cap is an error code.
    out_cap.wrapping_add(1)
}

#[test]
fn register_sequence_producer_null_and_real() {
    let a = api();
    let mut rng = Rng::new(0x5E9_0009);
    let lens = [1usize, 64, 1024, 65536];

    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, &mut rng);
                let cap = (a.comp_bound.0)(src.len()).max(64);

                // Two callback variants: NULL (clears) and the real error-callback.
                let variants: [(&str, Option<ZSTD_sequenceProducer_F>); 2] = [
                    ("null", None),
                    ("error_cb", Some(seqprod_always_error as ZSTD_sequenceProducer_F)),
                ];

                for (vname, cb) in variants {
                    for fallback in [0i32, 1] {
                        let ctx = format!(
                            "regSeqProd shape={shape:?} len={len} cb={vname} fallback={fallback}"
                        );

                        let cctx_c = (a.create_cctx.0)();
                        let cctx_r = (a.create_cctx.1)();

                        // Fresh, deterministic parameters. External seq producer
                        // forbids LDM, so disable it explicitly.
                        for &(p, v) in &[
                            (ZSTD_c_compressionLevel, 6),
                            (ZSTD_c_enableSeqProducerFallback, fallback),
                            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_disable),
                        ] {
                            let rc = (a.set_param.0)(cctx_c, p, v);
                            let rr = (a.set_param.1)(cctx_r, p, v);
                            assert_err_parity(&a, &format!("{ctx}: setParam({p},{v})"), rc, rr);
                        }

                        // Register the SAME callback into BOTH libraries.
                        (a.reg_seqprod.0)(cctx_c, std::ptr::null_mut(), cb);
                        (a.reg_seqprod.1)(cctx_r, std::ptr::null_mut(), cb);

                        let mut oc = vec![0xAAu8; cap];
                        let mut or = vec![0xAAu8; cap];

                        let nc = (a.compress2.0)(
                            cctx_c,
                            oc.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr_or_dangling(&src),
                            src.len(),
                        );
                        let nr = (a.compress2.1)(
                            cctx_r,
                            or.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr_or_dangling(&src),
                            src.len(),
                        );
                        (a.free_cctx.0)(cctx_c);
                        (a.free_cctx.1)(cctx_r);

                        assert_err_parity(&a, &format!("{ctx}: compress2"), nc, nr);
                        if (a.is_error.0)(nc) != 0 {
                            continue;
                        }
                        assert_eq!(nc, nr, "{ctx}: compress2 size differs (C={nc} R={nr})");
                        assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
                        roundtrip_check(&a, true, &oc[..nc], &src, &format!("{ctx}: C"));
                        roundtrip_check(&a, false, &or[..nr], &src, &format!("{ctx}: R"));
                    }
                }
            }
        }
    }
}
