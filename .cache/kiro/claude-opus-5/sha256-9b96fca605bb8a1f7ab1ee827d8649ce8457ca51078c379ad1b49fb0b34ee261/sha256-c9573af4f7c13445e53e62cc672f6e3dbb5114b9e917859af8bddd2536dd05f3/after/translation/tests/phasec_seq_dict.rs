//! Phase C — differential ERROR-PATH tests, part 3 of 4.
//!
//! Covers three ERRORS.md sections, one #[test] group cluster each:
//!   * Sequence / external-sequence API              (rows 101-129)
//!   * Dictionary load / CDict / DDict               (rows 220-230)
//!   * Dictionary builder (ZDICT / COVER / fastCover) (rows 231-262)
//!
//! Every invalid input is fed to BOTH the C `libzstd.so` and the Rust
//! `libzstd.so` through their exported symbols (via `fnpair!` => dlsym only —
//! no Rust function is ever called directly).  We assert exact parity:
//!   - `ZSTD_isError`/`ZDICT_isError` agree,
//!   - the raw return `size_t` is identical,
//!   - `ZSTD_getErrorCode` (for ZSTD_*) / `ZDICT_getErrorName` (for ZDICT_*)
//!     match, and
//!   - where a call is *defined*, the FULL output buffer (pre-filled 0xAA
//!     identically) matches.
//! Sentinel rows (NULL / 0) assert the exact sentinel on both sides.
//!
//! Where the C documents an input as UNDEFINED BEHAVIOUR (e.g. invalid
//! sequences with `ZSTD_c_validateSequences=0`), we do NOT assert equality on
//! the undefined output; we `eprintln!` that the row is UB-by-contract and
//! only assert what is defined (typically that both sides do not *crash* and
//! that when validation is enabled they reject identically).
//!
//! All randomness uses a FIXED seed so runs are reproducible.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// --------------------------------------------------------------- FFI types ---

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
#[allow(dead_code)]
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, u64) -> size_t;

type FnCompress2 =
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

// ZSTD_writeSkippableFrame(dst, dstCapacity, src, srcSize, magicVariant)
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_uint) -> size_t;

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

// ZSTD_registerSequenceProducer(cctx, state, fn) -> void
type FnRegisterSeqProd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, Option<ZSTD_sequenceProducer_F>);

// --- dictionary / CDict / DDict ---
type FnCreateCDict =
    unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void; // ZSTD_createCDict
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void; // ZSTD_createDDict
type FnFreeCDict = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnFreeDDict = unsafe extern "C" fn(*mut c_void) -> size_t;
#[allow(dead_code)]
type FnEstimateCDictSize = unsafe extern "C" fn(size_t, c_int) -> size_t;
#[allow(dead_code)]
type FnEstimateDDictSize = unsafe extern "C" fn(size_t, c_int) -> size_t;
// ZSTD_initStaticCDict(workspace, wkspSize, dict, dictSize, dlm, dct, cParams)
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
    c_int,
    ZSTD_compressionParameters,
) -> *const c_void;
// ZSTD_initStaticDDict(workspace, wkspSize, dict, dictSize, dlm, dct)
type FnInitStaticDDict = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
    c_int,
) -> *const c_void;
// ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dictSize, dlm, dct)
type FnLoadDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int, c_int) -> size_t;
type FnGetCParams =
    unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_compressionParameters; // ZSTD_getCParams

// --- ZDICT ---
type FnTrain = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
) -> size_t;
type FnTrainCover = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_cover_params_t,
) -> size_t;
type FnOptCover = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    *mut ZDICT_cover_params_t,
) -> size_t;
type FnTrainFast = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_fastCover_params_t,
) -> size_t;
type FnOptFast = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    *mut ZDICT_fastCover_params_t,
) -> size_t;
type FnTrainLegacy = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_legacy_params_t,
) -> size_t;
// ZDICT_finalizeDictionary(dst, maxDictSize, dictContent, dictContentSize,
//                          samplesBuffer, samplesSizes, nbSamples, ZDICT_params_t)
type FnFinalize = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    *const size_t,
    c_uint,
    ZDICT_params_t,
) -> size_t;
type FnGetDictID = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetHdrSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;

// ------------------------------------------------------------------ helpers ---

fn ptr_or_dangling(b: &[u8]) -> *const c_void {
    if b.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
    } else {
        b.as_ptr() as *const c_void
    }
}

/// Shared handles for the ZSTD_* error inspectors.
struct Zerr {
    is_error: (FnIsError, FnIsError),
    err_code: (FnGetErrorCode, FnGetErrorCode),
    err_name: (FnErrName, FnErrName),
}
fn zerr() -> Zerr {
    Zerr {
        is_error: fnpair!("ZSTD_isError", FnIsError),
        err_code: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        err_name: fnpair!("ZSTD_getErrorName", FnErrName),
    }
}

/// Full ZSTD error parity: isError agreement, exact raw return, error code and
/// error-name string parity when both errored.
#[track_caller]
fn assert_zstd_parity(z: &Zerr, ctx: &str, rc: size_t, rr: size_t) {
    unsafe {
        let ec = (z.is_error.0)(rc);
        let er = (z.is_error.1)(rr);
        assert_eq!(ec != 0, er != 0, "{ctx}: isError differs (C={rc:#x} R={rr:#x})");
        if ec != 0 {
            assert_eq!(
                (z.err_code.0)(rc),
                (z.err_code.1)(rr),
                "{ctx}: error code differs (C={} R={})",
                (z.err_code.0)(rc),
                (z.err_code.1)(rr)
            );
            assert_eq!(
                cstr((z.err_name.0)(rc)),
                cstr((z.err_name.1)(rr)),
                "{ctx}: error-name differs"
            );
        }
        // Raw return must be bit-identical whether error or success sentinel.
        assert_eq!(rc, rr, "{ctx}: raw return differs (C={rc:#x} R={rr:#x})");
    }
}

/// ZDICT error parity: isError agreement + error-name string parity + raw return.
#[track_caller]
fn assert_zdict_parity(
    is_err: &(FnIsError, FnIsError),
    err_name: &(FnErrName, FnErrName),
    ctx: &str,
    rc: size_t,
    rr: size_t,
) {
    unsafe {
        let ec = (is_err.0)(rc);
        let er = (is_err.1)(rr);
        assert_eq!(ec != 0, er != 0, "{ctx}: ZDICT isError differs (C={rc:#x} R={rr:#x})");
        if ec != 0 {
            assert_eq!(
                cstr((err_name.0)(rc)),
                cstr((err_name.1)(rr)),
                "{ctx}: ZDICT error-name differs"
            );
        }
        assert_eq!(rc, rr, "{ctx}: ZDICT raw return differs (C={rc:#x} R={rr:#x})");
    }
}

/// Build a valid explicit-delimiter sequence array for `src` at the given level,
/// returning `(count, seqs)`; panics if generation errors (it is used only to
/// obtain a valid baseline to then mutate into invalid inputs).
fn gen_valid_seqs(
    gen_seqs: &(FnGenerateSequences, FnGenerateSequences),
    seq_bound: &(FnSizeSize, FnSizeSize),
    create: &(FnCreate, FnCreate),
    free: &(FnFree, FnFree),
    set_param: &(FnSetParam, FnSetParam),
    src: &[u8],
    level: c_int,
) -> (usize, Vec<ZSTD_Sequence>) {
    unsafe {
        let cap = (seq_bound.0)(src.len()).max(1);
        let cctx = (create.0)();
        assert!(!cctx.is_null());
        let _ = (set_param.0)(cctx, ZSTD_c_compressionLevel, level);
        let mut sc = vec![ZSTD_Sequence::default(); cap];
        let n = (gen_seqs.0)(cctx, sc.as_mut_ptr(), cap, ptr_or_dangling(src), src.len());
        (free.0)(cctx);
        let is_err = fnpair!("ZSTD_isError", FnIsError);
        assert!((is_err.0)(n) == 0, "gen_valid_seqs: generateSequences errored");
        sc.truncate(n);
        (n, sc)
    }
}

// ============================================================================
//  SEQUENCE / EXTERNAL-SEQUENCE API — ERRORS.md rows 101-129
// ============================================================================

/// ERRORS rows 108-109, 114, 116, 122: `ZSTD_compressSequences` fed sequences
/// that are structurally invalid (empty, total != srcSize, block too large,
/// frame longer than source).  With `ZSTD_c_validateSequences=1` the C library
/// must reject; both libraries must reject identically.  With `validate=0` the
/// contract documents the behaviour as UNDEFINED for malformed sequences, so we
/// report those as UB-by-contract and do not assert on the (possibly garbage)
/// output — only that neither side crashes the process.
#[test]
fn seq_compress_sequences_invalid() {
    let z = zerr();
    let create = fnpair!("ZSTD_createCCtx", FnCreate);
    let free = fnpair!("ZSTD_freeCCtx", FnFree);
    let set_param = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let comp_bound = fnpair!("ZSTD_compressBound", FnSizeSize);
    let comp_seqs = fnpair!("ZSTD_compressSequences", FnCompressSequences);
    let gen_seqs = fnpair!("ZSTD_generateSequences", FnGenerateSequences);
    let seq_bound = fnpair!("ZSTD_sequenceBound", FnSizeSize);

    let mut rng = Rng::new(0xC0DE_0101);
    let src = gen(Shape::Text, 4096, &mut rng);
    let (count, base) = gen_valid_seqs(&gen_seqs, &seq_bound, &create, &free, &set_param, &src, 6);
    assert!(count >= 2, "need a multi-sequence baseline");

    // Each entry: (row, description, mutator, is_ub_when_validate0)
    // The mutator produces the invalid sequence array from the valid baseline.
    struct Case {
        rows: &'static str,
        desc: &'static str,
        seqs: Vec<ZSTD_Sequence>,
        nb: usize,
        // Whether validate=0 is documented UB (skip output assert then).
        ub_when_novalidate: bool,
    }

    let mut cases: Vec<Case> = Vec::new();

    // seqCount == 0  (rows 116 for AndLiterals mirror; for compressSequences an
    // empty seq array is rejected as it cannot terminate a block).
    cases.push(Case {
        rows: "116",
        desc: "seqCount=0 (no terminating end-of-block sequence)",
        seqs: vec![],
        nb: 0,
        ub_when_novalidate: false,
    });

    // offset == 0 in a real sequence (row 101 validateSequence rejects offBase).
    {
        let mut s = base.clone();
        // first non-delimiter sequence -> set offset 0
        for e in s.iter_mut() {
            if e.matchLength != 0 || e.offset != 0 {
                e.offset = 0;
                break;
            }
        }
        cases.push(Case { rows: "101", desc: "offset=0", seqs: s, nb: count, ub_when_novalidate: true });
    }

    // offset larger than window / data seen so far (row 101).
    {
        let mut s = base.clone();
        for e in s.iter_mut() {
            if e.matchLength != 0 {
                e.offset = 0xFFFF_FFF0;
                break;
            }
        }
        cases.push(Case {
            rows: "101",
            desc: "offset > window",
            seqs: s,
            nb: count,
            ub_when_novalidate: true,
        });
    }

    // matchLength < ZSTD_MINMATCH_MIN (row 102).
    {
        let mut s = base.clone();
        for e in s.iter_mut() {
            if e.matchLength >= 3 {
                e.matchLength = 1;
                break;
            }
        }
        cases.push(Case {
            rows: "102",
            desc: "matchLength < MINMATCH",
            seqs: s,
            nb: count,
            ub_when_novalidate: true,
        });
    }

    // litLength+matchLength summing past srcSize; frame longer than source
    // (rows 108/109/114/122).
    {
        let mut s = base.clone();
        // inflate the first sequence's matchLength massively.
        s[0].matchLength = s[0].matchLength.wrapping_add(0x0010_0000);
        cases.push(Case {
            rows: "108/109/114/122",
            desc: "sequences overrun srcSize",
            seqs: s,
            nb: count,
            ub_when_novalidate: true,
        });
    }

    // Missing terminal block-delimiter sequence in explicit-delimiter mode
    // (rows 104/107): drop the trailing (0,0,0)-style delimiter by truncating.
    {
        let mut s = base.clone();
        // Remove the final entry (which in explicit mode is the block delimiter).
        s.pop();
        let nb = s.len();
        cases.push(Case {
            rows: "104/107",
            desc: "missing terminal block delimiter (explicit mode)",
            seqs: s,
            nb,
            ub_when_novalidate: false,
        });
    }

    unsafe {
        let cap = (comp_bound.0)(src.len()).max(64);
        for case in &cases {
            for &validate in &[1i32, 0i32] {
                let cctx_c = (create.0)();
                let cctx_r = (create.1)();
                for &(p, v) in &[
                    (ZSTD_c_compressionLevel, 6),
                    (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                    (ZSTD_c_validateSequences, validate),
                ] {
                    let rc = (set_param.0)(cctx_c, p, v);
                    let rr = (set_param.1)(cctx_r, p, v);
                    assert_zstd_parity(&z, &format!("row {}: setParam", case.rows), rc, rr);
                }
                let mut oc = vec![0xAAu8; cap];
                let mut or = vec![0xAAu8; cap];
                let sp = if case.seqs.is_empty() {
                    std::ptr::null()
                } else {
                    case.seqs.as_ptr()
                };
                let nc = (comp_seqs.0)(
                    cctx_c,
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    sp,
                    case.nb,
                    ptr_or_dangling(&src),
                    src.len(),
                );
                let nr = (comp_seqs.1)(
                    cctx_r,
                    or.as_mut_ptr() as *mut c_void,
                    cap,
                    sp,
                    case.nb,
                    ptr_or_dangling(&src),
                    src.len(),
                );
                (free.0)(cctx_c);
                (free.1)(cctx_r);

                let ctx = format!("ERRORS row {}: {} (validate={validate})", case.rows, case.desc);
                if validate == 0 && case.ub_when_novalidate {
                    // The contract documents malformed sequences with
                    // validateSequences=0 as UNDEFINED BEHAVIOUR.  We only
                    // require that neither side aborts the process; we do NOT
                    // assert equality of the (undefined) result or output.
                    eprintln!(
                        "ERRORS row {}: {} is UB-by-contract with validateSequences=0 \
                         (C rc={nc:#x}, R rc={nr:#x}) — output not asserted",
                        case.rows, case.desc
                    );
                    let _ = (&oc, &or);
                    continue;
                }
                // Defined path: both must agree on error/return exactly.
                assert_zstd_parity(&z, &ctx, nc, nr);
                if (z.is_error.0)(nc) == 0 {
                    assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
                }
            }
        }
    }
}

/// ERRORS rows 116-126: `ZSTD_compressSequencesAndLiterals` error paths:
/// seqCount==0 (116), dstCapacity too small (117/119), litSize mismatch (118),
/// literals-not-consumed / remaining!=0 (121/122), litBuf too small (123),
/// noBlockDelimiters unsupported (124), incompatible with validateSequences
/// (125), incompatible with checksum (126), pledgedSrcSize wrong.
#[test]
fn seq_compress_sequences_and_literals_invalid() {
    let z = zerr();
    let create = fnpair!("ZSTD_createCCtx", FnCreate);
    let free = fnpair!("ZSTD_freeCCtx", FnFree);
    let set_param = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let set_pledged = fnpair!("ZSTD_CCtx_setPledgedSrcSize", FnSetPledged);
    let comp_bound = fnpair!("ZSTD_compressBound", FnSizeSize);
    let comp_seqs_lit = fnpair!("ZSTD_compressSequencesAndLiterals", FnCompressSequencesAndLiterals);
    let gen_seqs = fnpair!("ZSTD_generateSequences", FnGenerateSequences);
    let seq_bound = fnpair!("ZSTD_sequenceBound", FnSizeSize);

    let mut rng = Rng::new(0xC0DE_0116);
    let src = gen(Shape::Text, 4096, &mut rng);
    let (count, seqs) = gen_valid_seqs(&gen_seqs, &seq_bound, &create, &free, &set_param, &src, 6);

    // Assemble the literals buffer per the sequence litLengths (explicit mode).
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
    let lit_cap = lit_size + 8;
    let mut lit_buf = vec![0u8; lit_cap];
    lit_buf[..lit_size].copy_from_slice(&lits);

    // (row, desc, params-override, nb, litSize, litCap, decompressed, dstCapOverride)
    struct C {
        rows: &'static str,
        desc: &'static str,
        params: Vec<(c_int, c_int)>,
        nb: usize,
        lit_size: usize,
        lit_cap: usize,
        decompressed: usize,
        dst_cap: Option<usize>,
        pledged: Option<u64>,
    }
    let base_params = vec![
        (ZSTD_c_compressionLevel, 6),
        (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
    ];
    let mut cases: Vec<C> = Vec::new();

    cases.push(C {
        rows: "116",
        desc: "nbSequences=0",
        params: base_params.clone(),
        nb: 0,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    // ERRORS row 117 (`dstCapacity < 3`) is NOT safely reachable as a clean
    // differential: ZSTD_compressSequencesAndLiterals calls ZSTD_writeFrameHeader
    // WITHOUT checking its return value (compress/zstd_compress.c:7591 — only an
    // `assert`, compiled out in release). For any `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX`
    // (18) the header write returns an error code that is then used as a byte
    // count (`op += frameHeaderSize`), corrupting the heap on the C side too.
    // This is a C-library defect / UNDEFINED BEHAVIOUR, faithfully mirrored by
    // the Rust translation, so we do NOT construct it. Reported below.
    cases.push(C {
        rows: "118",
        desc: "litSize too small for sequences",
        params: base_params.clone(),
        nb: count,
        lit_size: lit_size.saturating_sub(1),
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    cases.push(C {
        rows: "123",
        desc: "litBufCapacity < litSize+8 (OOB-read risk)",
        params: base_params.clone(),
        nb: count,
        lit_size,
        lit_cap: lit_size, // no +8 slack
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    cases.push(C {
        rows: "124",
        desc: "noBlockDelimiters unsupported",
        params: vec![
            (ZSTD_c_compressionLevel, 6),
            (ZSTD_c_blockDelimiters, ZSTD_sf_noBlockDelimiters),
        ],
        nb: count,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    cases.push(C {
        rows: "125",
        desc: "incompatible with validateSequences=1",
        params: vec![
            (ZSTD_c_compressionLevel, 6),
            (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
            (ZSTD_c_validateSequences, 1),
        ],
        nb: count,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    cases.push(C {
        rows: "126",
        desc: "incompatible with frame checksum",
        params: vec![
            (ZSTD_c_compressionLevel, 6),
            (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
            (ZSTD_c_checksumFlag, 1),
        ],
        nb: count,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64),
    });
    // pledgedSrcSize not set / wrong (rows 121/122 total-mismatch surface).
    cases.push(C {
        rows: "122",
        desc: "pledgedSrcSize wrong (too large)",
        params: base_params.clone(),
        nb: count,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: Some(decompressed as u64 + 4096),
    });
    cases.push(C {
        rows: "121/122",
        desc: "pledgedSrcSize not set",
        params: base_params.clone(),
        nb: count,
        lit_size,
        lit_cap,
        decompressed,
        dst_cap: None,
        pledged: None,
    });

    unsafe {
        let full_cap = (comp_bound.0)(src.len()).max(64);
        for case in &cases {
            let cap = case.dst_cap.unwrap_or(full_cap);
            let cctx_c = (create.0)();
            let cctx_r = (create.1)();
            if let Some(p) = case.pledged {
                let rc = (set_pledged.0)(cctx_c, p);
                let rr = (set_pledged.1)(cctx_r, p);
                assert_zstd_parity(&z, &format!("row {}: setPledged", case.rows), rc, rr);
            }
            for &(p, v) in &case.params {
                let rc = (set_param.0)(cctx_c, p, v);
                let rr = (set_param.1)(cctx_r, p, v);
                assert_zstd_parity(&z, &format!("row {}: setParam({p},{v})", case.rows), rc, rr);
            }
            let mut oc = vec![0xAAu8; cap.max(1)];
            let mut or = vec![0xAAu8; cap.max(1)];
            let odst_c = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                oc.as_mut_ptr() as *mut c_void
            };
            let odst_r = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                or.as_mut_ptr() as *mut c_void
            };
            let lit_ptr = lit_buf.as_ptr() as *const c_void;
            let sp = if case.nb == 0 { std::ptr::null() } else { seqs.as_ptr() };

            let nc = (comp_seqs_lit.0)(
                cctx_c, odst_c, cap, sp, case.nb, lit_ptr, case.lit_size, case.lit_cap,
                case.decompressed,
            );
            let nr = (comp_seqs_lit.1)(
                cctx_r, odst_r, cap, sp, case.nb, lit_ptr, case.lit_size, case.lit_cap,
                case.decompressed,
            );
            (free.0)(cctx_c);
            (free.1)(cctx_r);

            let ctx = format!("ERRORS row {}: {}", case.rows, case.desc);
            assert_zstd_parity(&z, &ctx, nc, nr);
            if cap > 0 && (z.is_error.0)(nc) == 0 {
                assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
            }
        }
    }
    eprintln!(
        "ERRORS row 117 (ZSTD_compressSequencesAndLiterals dstCapacity < 3): UB-by-contract — \
         the caller does not check ZSTD_writeFrameHeader's return, so dstCapacity < 18 corrupts \
         the heap on the C side too (assert compiled out in release). Not constructed."
    );
}

/// ERRORS rows 110-113 (sequence-producer surface) and generateSequences /
/// mergeBlockDelimiters / sequenceBound edge inputs.
///
/// A deterministic Rust `extern "C"` callback is registered into BOTH libraries
/// per behaviour and we assert identical error/fallback outcomes.
#[test]
fn seq_producer_and_generate_edges() {
    let z = zerr();
    let create = fnpair!("ZSTD_createCCtx", FnCreate);
    let free = fnpair!("ZSTD_freeCCtx", FnFree);
    let set_param = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let compress2 = fnpair!("ZSTD_compress2", FnCompress2);
    let comp_bound = fnpair!("ZSTD_compressBound", FnSizeSize);
    let reg = fnpair!("ZSTD_registerSequenceProducer", FnRegisterSeqProd);
    let gen_seqs = fnpair!("ZSTD_generateSequences", FnGenerateSequences);
    let seq_bound = fnpair!("ZSTD_sequenceBound", FnSizeSize);
    let merge = fnpair!("ZSTD_mergeBlockDelimiters", FnMergeBlockDelimiters);

    // --- rows 110/111/112: sequence-producer failure behaviours. -----------
    // Callback #1: returns an error code (any value > outCapacity) -> row 110.
    unsafe extern "C" fn cb_error(
        _s: *mut c_void,
        _o: *mut ZSTD_Sequence,
        cap: size_t,
        _src: *const c_void,
        _ss: size_t,
        _d: *const c_void,
        _ds: size_t,
        _l: c_int,
        _w: size_t,
    ) -> size_t {
        cap.wrapping_add(1)
    }
    // Callback #2: returns 0 sequences for non-empty src -> row 111.
    unsafe extern "C" fn cb_empty(
        _s: *mut c_void,
        _o: *mut ZSTD_Sequence,
        _cap: size_t,
        _src: *const c_void,
        _ss: size_t,
        _d: *const c_void,
        _ds: size_t,
        _l: c_int,
        _w: size_t,
    ) -> size_t {
        0
    }
    // Callback #3: returns MORE sequences than the window allows / final seq is
    // not a delimiter (fills capacity with non-delimiter entries) -> row 112.
    unsafe extern "C" fn cb_toomany(
        _s: *mut c_void,
        o: *mut ZSTD_Sequence,
        cap: size_t,
        _src: *const c_void,
        _ss: size_t,
        _d: *const c_void,
        _ds: size_t,
        _l: c_int,
        _w: size_t,
    ) -> size_t {
        // fill the whole capacity with non-delimiter sequences (matchLength!=0)
        if cap > 0 && !o.is_null() {
            for i in 0..cap {
                *o.add(i) = ZSTD_Sequence { offset: 1, litLength: 0, matchLength: 3, rep: 0 };
            }
        }
        cap
    }

    let mut rng = Rng::new(0xC0DE_0110);
    let src = gen(Shape::Text, 8192, &mut rng);

    let variants: [(&str, &str, ZSTD_sequenceProducer_F); 3] = [
        ("110", "producer returns error code", cb_error as ZSTD_sequenceProducer_F),
        ("111", "producer returns 0 seqs for non-empty src", cb_empty as ZSTD_sequenceProducer_F),
        (
            "112",
            "producer fills capacity w/ non-delimiter final seq",
            cb_toomany as ZSTD_sequenceProducer_F,
        ),
    ];

    unsafe {
        let cap = (comp_bound.0)(src.len()).max(64);
        for (rows, desc, cb) in variants {
            for &fallback in &[0i32, 1i32] {
                let cctx_c = (create.0)();
                let cctx_r = (create.1)();
                for &(p, v) in &[
                    (ZSTD_c_compressionLevel, 6),
                    (ZSTD_c_enableSeqProducerFallback, fallback),
                    (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_disable),
                ] {
                    let rc = (set_param.0)(cctx_c, p, v);
                    let rr = (set_param.1)(cctx_r, p, v);
                    assert_zstd_parity(&z, &format!("row {rows}: setParam"), rc, rr);
                }
                // Register the SAME callback into BOTH libraries.
                (reg.0)(cctx_c, std::ptr::null_mut(), Some(cb));
                (reg.1)(cctx_r, std::ptr::null_mut(), Some(cb));

                let mut oc = vec![0xAAu8; cap];
                let mut or = vec![0xAAu8; cap];
                let nc = (compress2.0)(
                    cctx_c, oc.as_mut_ptr() as *mut c_void, cap, ptr_or_dangling(&src), src.len(),
                );
                let nr = (compress2.1)(
                    cctx_r, or.as_mut_ptr() as *mut c_void, cap, ptr_or_dangling(&src), src.len(),
                );
                (free.0)(cctx_c);
                (free.1)(cctx_r);

                let ctx = format!("ERRORS row {rows}: {desc} (fallback={fallback})");
                assert_zstd_parity(&z, &ctx, nc, nr);
                // When fallback==0 an error is expected; when fallback==1 the
                // internal producer takes over.  Either way both libs agree,
                // and on success the full buffer matches.
                if (z.is_error.0)(nc) == 0 {
                    assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
                }
            }
        }

        // --- ZSTD_generateSequences edge inputs (rows in the 101-129 band). --
        // srcSize 0.
        {
            let cctx_c = (create.0)();
            let cctx_r = (create.1)();
            let _ = (set_param.0)(cctx_c, ZSTD_c_compressionLevel, 6);
            let _ = (set_param.1)(cctx_r, ZSTD_c_compressionLevel, 6);
            let cap = (seq_bound.0)(0).max(1);
            let mut sc = vec![ZSTD_Sequence::default(); cap];
            let mut sr = vec![ZSTD_Sequence::default(); cap];
            let empty: [u8; 0] = [];
            let nc = (gen_seqs.0)(cctx_c, sc.as_mut_ptr(), cap, ptr_or_dangling(&empty), 0);
            let nr = (gen_seqs.1)(cctx_r, sr.as_mut_ptr(), cap, ptr_or_dangling(&empty), 0);
            (free.0)(cctx_c);
            (free.1)(cctx_r);
            assert_zstd_parity(&z, "ERRORS row 113: generateSequences srcSize=0", nc, nr);
        }

        // outSeqs capacity 0, 1, and below sequenceBound(srcSize) (row 113).
        {
            let src2 = gen(Shape::Text, 4096, &mut rng);
            let full = (seq_bound.0)(src2.len()).max(1);
            for &cap in &[0usize, 1, full / 2, full.saturating_sub(1)] {
                let cctx_c = (create.0)();
                let cctx_r = (create.1)();
                let _ = (set_param.0)(cctx_c, ZSTD_c_compressionLevel, 6);
                let _ = (set_param.1)(cctx_r, ZSTD_c_compressionLevel, 6);
                let mut sc = vec![ZSTD_Sequence::default(); cap.max(1)];
                let mut sr = vec![ZSTD_Sequence::default(); cap.max(1)];
                let scp = if cap == 0 { std::ptr::null_mut() } else { sc.as_mut_ptr() };
                let srp = if cap == 0 { std::ptr::null_mut() } else { sr.as_mut_ptr() };
                let nc = (gen_seqs.0)(cctx_c, scp, cap, ptr_or_dangling(&src2), src2.len());
                let nr = (gen_seqs.1)(cctx_r, srp, cap, ptr_or_dangling(&src2), src2.len());
                (free.0)(cctx_c);
                (free.1)(cctx_r);
                assert_zstd_parity(
                    &z,
                    &format!("ERRORS row 113: generateSequences outCap={cap}"),
                    nc,
                    nr,
                );
            }
        }

        // NULL outSeqs with capacity 0 (row 113 / sentinel).
        {
            let cctx_c = (create.0)();
            let cctx_r = (create.1)();
            let _ = (set_param.0)(cctx_c, ZSTD_c_compressionLevel, 6);
            let _ = (set_param.1)(cctx_r, ZSTD_c_compressionLevel, 6);
            let s = gen(Shape::Text, 512, &mut rng);
            let nc = (gen_seqs.0)(cctx_c, std::ptr::null_mut(), 0, ptr_or_dangling(&s), s.len());
            let nr = (gen_seqs.1)(cctx_r, std::ptr::null_mut(), 0, ptr_or_dangling(&s), s.len());
            (free.0)(cctx_c);
            (free.1)(cctx_r);
            assert_zstd_parity(&z, "ERRORS row 113: generateSequences NULL outSeqs cap0", nc, nr);
        }

        // ZSTD_mergeBlockDelimiters on an empty array and an array with no
        // delimiters (band 101-129 helper).
        {
            let nc = (merge.0)(std::ptr::null_mut(), 0);
            let nr = (merge.1)(std::ptr::null_mut(), 0);
            assert_eq!(nc, nr, "ERRORS row ~103: mergeBlockDelimiters(empty) differs");

            let mut ac = vec![
                ZSTD_Sequence { offset: 1, litLength: 2, matchLength: 3, rep: 0 },
                ZSTD_Sequence { offset: 1, litLength: 0, matchLength: 4, rep: 0 },
            ];
            let mut ar = ac.clone();
            let mc = (merge.0)(ac.as_mut_ptr(), ac.len());
            let mr = (merge.1)(ar.as_mut_ptr(), ar.len());
            assert_eq!(mc, mr, "ERRORS row ~103: mergeBlockDelimiters(no-delim) count differs");
            assert_eq!(ac, ar, "ERRORS row ~103: mergeBlockDelimiters(no-delim) array differs");
        }

        // ZSTD_sequenceBound with 0 and huge sizes.
        for &s in &[0usize, 1, usize::MAX / 2, usize::MAX] {
            let bc = (seq_bound.0)(s);
            let br = (seq_bound.1)(s);
            assert_eq!(bc, br, "ERRORS row ~127: sequenceBound({s:#x}) differs C={bc} R={br}");
        }
    }
}

/// ERRORS rows 127-129: `ZSTD_writeSkippableFrame` — dstCapacity too small
/// (127), srcSize > 0xFFFFFFFF (128, unreachable on this platform if usize is
/// 64-bit but we still attempt it and report), magicVariant > 15 (129).
#[test]
fn seq_write_skippable_frame_invalid() {
    let z = zerr();
    let write_skip = fnpair!("ZSTD_writeSkippableFrame", FnWriteSkippable);

    let mut rng = Rng::new(0xC0DE_0127);
    let payload = gen(Shape::Random, 64, &mut rng);

    unsafe {
        // row 127: dstCapacity < srcSize + 8.
        for &cap in &[0usize, 1, 7, payload.len() + 7] {
            let mut oc = vec![0xAAu8; cap.max(1)];
            let mut or = vec![0xAAu8; cap.max(1)];
            let dc = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                oc.as_mut_ptr() as *mut c_void
            };
            let dr = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                or.as_mut_ptr() as *mut c_void
            };
            let nc = (write_skip.0)(dc, cap, ptr_or_dangling(&payload), payload.len(), 0);
            let nr = (write_skip.1)(dr, cap, ptr_or_dangling(&payload), payload.len(), 0);
            let ctx = format!("ERRORS row 127: writeSkippableFrame dstCap={cap}");
            assert_zstd_parity(&z, &ctx, nc, nr);
            if cap > 0 && (z.is_error.0)(nc) == 0 {
                assert_bytes_eq(&format!("{ctx}: full output"), &oc, &or);
            }
        }

        // row 129: magicVariant > 15.
        for &mv in &[16u32, 100, 0xFFFF_FFFF] {
            let cap = payload.len() + 8;
            let mut oc = vec![0xAAu8; cap];
            let mut or = vec![0xAAu8; cap];
            let nc = (write_skip.0)(
                oc.as_mut_ptr() as *mut c_void, cap, ptr_or_dangling(&payload), payload.len(), mv,
            );
            let nr = (write_skip.1)(
                or.as_mut_ptr() as *mut c_void, cap, ptr_or_dangling(&payload), payload.len(), mv,
            );
            let ctx = format!("ERRORS row 129: writeSkippableFrame magicVariant={mv}");
            assert_zstd_parity(&z, &ctx, nc, nr);
        }

        // row 128: srcSize > 0xFFFFFFFF. On a 64-bit platform we cannot
        // materialise a >4GiB buffer, but the check reads only srcSize, so we
        // pass a dangling ptr with an oversized length and small dstCapacity.
        // Both libs read the length field before touching src, so this is
        // exercised safely.
        {
            let huge: size_t = 0x1_0000_0000; // 4 GiB + 0
            // dstCapacity is deliberately small; the srcSize check (128) is the
            // first branch, so no OOB read occurs.
            let mut oc = vec![0xAAu8; 16];
            let mut or = vec![0xAAu8; 16];
            let dummy = std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void;
            let nc = (write_skip.0)(oc.as_mut_ptr() as *mut c_void, 16, dummy, huge, 0);
            let nr = (write_skip.1)(or.as_mut_ptr() as *mut c_void, 16, dummy, huge, 0);
            assert_zstd_parity(&z, "ERRORS row 128: writeSkippableFrame srcSize>4GiB", nc, nr);
        }
    }
}

// ============================================================================
//  DICTIONARY LOAD / CDict / DDict — ERRORS.md rows 220-230
// ============================================================================

/// Train a small real dictionary to use as valid input we can then corrupt.
fn train_real_dict(rng: &mut Rng) -> Vec<u8> {
    let train = fnpair!("ZDICT_trainFromBuffer", FnTrain);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    // build a modest corpus with shared motif so training succeeds
    const MOTIF: &[u8] =
        b"{\"id\":12345,\"name\":\"zstd-dictionary-training-shared-motif\",\"kind\":\"record\"}";
    let mut buf = Vec::new();
    let mut sizes = Vec::new();
    for i in 0..200usize {
        let mut s = gen(if i % 2 == 0 { Shape::Text } else { Shape::Repetitive }, 128 + rng.below(256), rng);
        if s.len() > MOTIF.len() + 4 {
            let off = rng.below(s.len() - MOTIF.len());
            s[off..off + MOTIF.len()].copy_from_slice(MOTIF);
        }
        sizes.push(s.len());
        buf.extend_from_slice(&s);
    }
    let cap = 8192usize;
    let mut d = vec![0u8; cap];
    unsafe {
        let n = (train.0)(
            d.as_mut_ptr() as *mut c_void,
            cap,
            buf.as_ptr() as *const c_void,
            sizes.as_ptr(),
            sizes.len() as c_uint,
        );
        assert!((is_err.0)(n) == 0, "train_real_dict failed");
        d.truncate(n);
    }
    d
}

/// ERRORS rows 220-230: CDict/DDict creation and dictionary loading error paths.
///
/// * createCDict/createDDict with NULL dict & non-zero size, size 0, a 4-byte
///   truncated dict-magic buffer, and a corrupted-entropy real dict under
///   dctType fullDict (must reject 220-228) vs rawContent (must accept, 229/230
///   raw path).
/// * loadDictionary_advanced with fullDict on non-dictionary data (62/220-228).
/// * decoding a dictID-tagged frame with WRONG dict / NO dict (144/229 wrong).
#[test]
fn dict_cdict_ddict_invalid() {
    let z = zerr();
    let create_cd = fnpair!("ZSTD_createCDict", FnCreateCDict);
    let free_cd = fnpair!("ZSTD_freeCDict", FnFreeCDict);
    let create_dd = fnpair!("ZSTD_createDDict", FnCreateDDict);
    let free_dd = fnpair!("ZSTD_freeDDict", FnFreeDDict);
    let create_cctx = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_cctx = fnpair!("ZSTD_freeCCtx", FnFree);
    let create_dctx = fnpair!("ZSTD_createDCtx", FnCreate);
    let free_dctx = fnpair!("ZSTD_freeDCtx", FnFree);
    let load_c = fnpair!("ZSTD_CCtx_loadDictionary_advanced", FnLoadDictAdv);
    let load_d = fnpair!("ZSTD_DCtx_loadDictionary_advanced", FnLoadDictAdv);
    let set_param = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);

    let mut rng = Rng::new(0xD1C7_0220);

    unsafe {
        // --- createCDict / createDDict NULL + size!=0 (sentinel: NULL) ------
        {
            let cc = (create_cd.0)(std::ptr::null(), 32, 6);
            let cr = (create_cd.1)(std::ptr::null(), 32, 6);
            assert_eq!(
                cc.is_null(),
                cr.is_null(),
                "ERRORS row 220: createCDict(NULL,32) null-ness differs (C={cc:?} R={cr:?})"
            );
            if !cc.is_null() {
                (free_cd.0)(cc);
            }
            if !cr.is_null() {
                (free_cd.1)(cr);
            }
            let dc = (create_dd.0)(std::ptr::null(), 32);
            let dr = (create_dd.1)(std::ptr::null(), 32);
            assert_eq!(
                dc.is_null(),
                dr.is_null(),
                "ERRORS row 220: createDDict(NULL,32) null-ness differs"
            );
            if !dc.is_null() {
                (free_dd.0)(dc);
            }
            if !dr.is_null() {
                (free_dd.1)(dr);
            }
        }

        // --- createCDict / createDDict size 0 (empty dict is valid) ---------
        {
            let empty: [u8; 0] = [];
            let cc = (create_cd.0)(ptr_or_dangling(&empty), 0, 6);
            let cr = (create_cd.1)(ptr_or_dangling(&empty), 0, 6);
            assert_eq!(cc.is_null(), cr.is_null(), "ERRORS row 220: createCDict(empty) differs");
            if !cc.is_null() {
                (free_cd.0)(cc);
            }
            if !cr.is_null() {
                (free_cd.1)(cr);
            }
            let dc = (create_dd.0)(ptr_or_dangling(&empty), 0);
            let dr = (create_dd.1)(ptr_or_dangling(&empty), 0);
            assert_eq!(dc.is_null(), dr.is_null(), "ERRORS row 220: createDDict(empty) differs");
            if !dc.is_null() {
                (free_dd.0)(dc);
            }
            if !dr.is_null() {
                (free_dd.1)(dr);
            }
        }

        // --- 4-byte buffer starting with ZSTD_MAGIC_DICTIONARY, truncated ----
        // (row 220/230: dictSize <= 8 -> dictionary_corrupted for the fullDict
        // path; createCDict/DDict use dct_auto so a too-short magic dict yields
        // a NULL object). Assert null-ness parity + load parity below.
        {
            let mut trunc = 0xEC30A437u32.to_le_bytes().to_vec(); // exactly 4 bytes
            // createCDict with dct_auto: too-short "dict" -> treated as raw or
            // rejected; either way both libs must agree.
            let cc = (create_cd.0)(trunc.as_ptr() as *const c_void, trunc.len(), 6);
            let cr = (create_cd.1)(trunc.as_ptr() as *const c_void, trunc.len(), 6);
            assert_eq!(
                cc.is_null(),
                cr.is_null(),
                "ERRORS row 230: createCDict(truncated-magic) null-ness differs"
            );
            if !cc.is_null() {
                (free_cd.0)(cc);
            }
            if !cr.is_null() {
                (free_cd.1)(cr);
            }
            let dc = (create_dd.0)(trunc.as_ptr() as *const c_void, trunc.len());
            let dr = (create_dd.1)(trunc.as_ptr() as *const c_void, trunc.len());
            assert_eq!(
                dc.is_null(),
                dr.is_null(),
                "ERRORS row 230: createDDict(truncated-magic) null-ness differs"
            );
            if !dc.is_null() {
                (free_dd.0)(dc);
            }
            if !dr.is_null() {
                (free_dd.1)(dr);
            }

            // loadDictionary_advanced fullDict on this truncated magic buffer
            // (rows 220-228): must reject identically as dictionary_corrupted.
            let cctx_c = (create_cctx.0)();
            let cctx_r = (create_cctx.1)();
            let rc = (load_c.0)(
                cctx_c, trunc.as_ptr() as *const c_void, trunc.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
            );
            let rr = (load_c.1)(
                cctx_r, trunc.as_ptr() as *const c_void, trunc.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
            );
            (free_cctx.0)(cctx_c);
            (free_cctx.1)(cctx_r);
            assert_zstd_parity(&z, "ERRORS row 220-228: CCtx load fullDict truncated-magic", rc, rr);

            let dctx_c = (create_dctx.0)();
            let dctx_r = (create_dctx.1)();
            let rc = (load_d.0)(
                dctx_c, trunc.as_ptr() as *const c_void, trunc.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
            );
            let rr = (load_d.1)(
                dctx_r, trunc.as_ptr() as *const c_void, trunc.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
            );
            (free_dctx.0)(dctx_c);
            (free_dctx.1)(dctx_r);
            assert_zstd_parity(&z, "ERRORS row 220-228: DCtx load fullDict truncated-magic", rc, rr);
            let _ = &mut trunc;
        }

        // --- corrupt entropy tables of a real trained dict ------------------
        // fullDict must reject (220-228 dictionary_corrupted); rawContent must
        // accept (229/230 raw path). We flip bytes just after the magic+dictID
        // (offset ~8) where the entropy tables live.
        {
            let good = train_real_dict(&mut rng);
            assert!(good.len() > 32, "trained dict too small to corrupt");
            let mut bad = good.clone();
            for i in 8..24 {
                bad[i] ^= 0xFF;
            }

            // fullDict on both CCtx and DCtx must reject identically.
            for (name, loader, create_ctx, free_ctx) in [
                ("CCtx", &load_c, &create_cctx, &free_cctx),
                ("DCtx", &load_d, &create_dctx, &free_dctx),
            ] {
                let ctx_c = (create_ctx.0)();
                let ctx_r = (create_ctx.1)();
                let rc = (loader.0)(
                    ctx_c, bad.as_ptr() as *const c_void, bad.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
                );
                let rr = (loader.1)(
                    ctx_r, bad.as_ptr() as *const c_void, bad.len(), ZSTD_dlm_byRef, ZSTD_dct_fullDict,
                );
                (free_ctx.0)(ctx_c);
                (free_ctx.1)(ctx_r);
                assert_zstd_parity(
                    &z,
                    &format!("ERRORS row 220-228: {name} load fullDict corrupt-entropy"),
                    rc,
                    rr,
                );
            }

            // rawContent must be accepted on both (no entropy interpretation).
            for (name, loader, create_ctx, free_ctx) in [
                ("CCtx", &load_c, &create_cctx, &free_cctx),
                ("DCtx", &load_d, &create_dctx, &free_dctx),
            ] {
                let ctx_c = (create_ctx.0)();
                let ctx_r = (create_ctx.1)();
                let rc = (loader.0)(
                    ctx_c, bad.as_ptr() as *const c_void, bad.len(), ZSTD_dlm_byRef, ZSTD_dct_rawContent,
                );
                let rr = (loader.1)(
                    ctx_r, bad.as_ptr() as *const c_void, bad.len(), ZSTD_dlm_byRef, ZSTD_dct_rawContent,
                );
                (free_ctx.0)(ctx_c);
                (free_ctx.1)(ctx_r);
                assert_zstd_parity(
                    &z,
                    &format!("ERRORS row 229/230: {name} load rawContent corrupt-entropy (accepted)"),
                    rc,
                    rr,
                );
            }
        }

        // --- loadDictionary_advanced fullDict on non-dictionary data (row 62/220) ---
        {
            let junk = gen(Shape::Random, 1024, &mut rng);
            let cctx_c = (create_cctx.0)();
            let cctx_r = (create_cctx.1)();
            let rc = (load_c.0)(
                cctx_c, junk.as_ptr() as *const c_void, junk.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
            );
            let rr = (load_c.1)(
                cctx_r, junk.as_ptr() as *const c_void, junk.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
            );
            (free_cctx.0)(cctx_c);
            (free_cctx.1)(cctx_r);
            assert_zstd_parity(&z, "ERRORS row 220-228: CCtx fullDict on random data", rc, rr);

            let dctx_c = (create_dctx.0)();
            let dctx_r = (create_dctx.1)();
            let rc = (load_d.0)(
                dctx_c, junk.as_ptr() as *const c_void, junk.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
            );
            let rr = (load_d.1)(
                dctx_r, junk.as_ptr() as *const c_void, junk.len(), ZSTD_dlm_byCopy, ZSTD_dct_fullDict,
            );
            (free_dctx.0)(dctx_c);
            (free_dctx.1)(dctx_r);
            assert_zstd_parity(&z, "ERRORS row 220-228: DCtx fullDict on random data", rc, rr);
        }

        // --- loading a dictionary while CCtx is mid-stream (row 50 stage_wrong) ---
        {
            let compress2 = fnpair!("ZSTD_compress2", FnCompress2);
            let comp_bound = fnpair!("ZSTD_compressBound", FnSizeSize);
            let stream_fn = fnpair!("ZSTD_compressStream2", FnCompress2Stream);
            let dict = train_real_dict(&mut rng);
            let src = gen(Shape::Text, 4096, &mut rng);
            let cctx_c = (create_cctx.0)();
            let cctx_r = (create_cctx.1)();
            let _ = (set_param.0)(cctx_c, ZSTD_c_compressionLevel, 6);
            let _ = (set_param.1)(cctx_r, ZSTD_c_compressionLevel, 6);
            let cap = (comp_bound.0)(src.len()).max(64);
            let mut oc = vec![0u8; cap];
            let mut or = vec![0u8; cap];
            // start a stream so streamStage != zcss_init
            let mut inb_c = ZSTD_inBuffer { src: src.as_ptr() as *const c_void, size: src.len(), pos: 0 };
            let mut inb_r = inb_c;
            let mut outb_c = ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let mut outb_r = ZSTD_outBuffer { dst: or.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
            let _ = (stream_fn.0)(cctx_c, &mut outb_c, &mut inb_c, ZSTD_e_continue);
            let _ = (stream_fn.1)(cctx_r, &mut outb_r, &mut inb_r, ZSTD_e_continue);
            // now load a dictionary mid-stream -> stage_wrong on both
            let rc = (load_c.0)(
                cctx_c, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byRef, ZSTD_dct_auto,
            );
            let rr = (load_c.1)(
                cctx_r, dict.as_ptr() as *const c_void, dict.len(), ZSTD_dlm_byRef, ZSTD_dct_auto,
            );
            (free_cctx.0)(cctx_c);
            (free_cctx.1)(cctx_r);
            assert_zstd_parity(&z, "ERRORS row 50: loadDictionary mid-stream (stage_wrong)", rc, rr);
            let _ = &compress2;
        }
    }
}

// ZSTD_compressStream2(cctx, out*, in*, endOp)
type FnCompress2Stream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int) -> size_t;

/// ERRORS rows 144/229: decode a dictID-tagged frame with the WRONG dictionary
/// and with NO dictionary — both must fail as `dictionary_wrong` identically.
/// Also covers static CDict/DDict init with undersized + misaligned workspace
/// (must return NULL identically).
#[test]
fn dict_wrong_and_static_init() {
    let z = zerr();
    let create_cctx = fnpair!("ZSTD_createCCtx", FnCreate);
    let free_cctx = fnpair!("ZSTD_freeCCtx", FnFree);
    let create_dctx = fnpair!("ZSTD_createDCtx", FnCreate);
    let free_dctx = fnpair!("ZSTD_freeDCtx", FnFree);
    let set_param = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
    let comp_usingdict = fnpair!("ZSTD_compress_usingDict", FnCompressUsingDict);
    let decomp_usingdict = fnpair!("ZSTD_decompress_usingDict", FnDecompressUsingDict);
    let comp_bound = fnpair!("ZSTD_compressBound", FnSizeSize);
    let init_static_cd = fnpair!("ZSTD_initStaticCDict", FnInitStaticCDict);
    let init_static_dd = fnpair!("ZSTD_initStaticDDict", FnInitStaticDDict);
    let get_cparams = fnpair!("ZSTD_getCParams", FnGetCParams);

    let mut rng = Rng::new(0xD1C7_0144);

    unsafe {
        // Build a frame compressed WITH a dictionary that carries a dictID.
        let dict = train_real_dict(&mut rng);
        let src = gen(Shape::Text, 8192, &mut rng);
        let cap = (comp_bound.0)(src.len()).max(64);
        let mut frame = vec![0u8; cap];
        let cctx = (create_cctx.0)();
        let _ = (set_param.0)(cctx, ZSTD_c_compressionLevel, 6);
        // ensure dictID is written into the frame (default keeps it)
        let fsize = (comp_usingdict.0)(
            cctx,
            frame.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            dict.as_ptr() as *const c_void,
            dict.len(),
            6,
        );
        (free_cctx.0)(cctx);
        assert!((z.is_error.0)(fsize) == 0, "setup: compress_usingDict failed");
        frame.truncate(fsize);

        // A *different* dictionary (wrong dictID).
        let wrong = train_real_dict(&mut rng);

        // decode with WRONG dict (row 144 dictionary_wrong).
        {
            let mut oc = vec![0xAAu8; src.len() + 64];
            let mut or = vec![0xAAu8; src.len() + 64];
            let dctx_c = (create_dctx.0)();
            let dctx_r = (create_dctx.1)();
            let rc = (decomp_usingdict.0)(
                dctx_c, oc.as_mut_ptr() as *mut c_void, oc.len(),
                frame.as_ptr() as *const c_void, frame.len(),
                wrong.as_ptr() as *const c_void, wrong.len(),
            );
            let rr = (decomp_usingdict.1)(
                dctx_r, or.as_mut_ptr() as *mut c_void, or.len(),
                frame.as_ptr() as *const c_void, frame.len(),
                wrong.as_ptr() as *const c_void, wrong.len(),
            );
            (free_dctx.0)(dctx_c);
            (free_dctx.1)(dctx_r);
            assert_zstd_parity(&z, "ERRORS row 144: decode with WRONG dictionary", rc, rr);
        }

        // decode with NO dict (row 144/229 dictionary_wrong).
        {
            let mut oc = vec![0xAAu8; src.len() + 64];
            let mut or = vec![0xAAu8; src.len() + 64];
            let dctx_c = (create_dctx.0)();
            let dctx_r = (create_dctx.1)();
            let rc = (decomp_usingdict.0)(
                dctx_c, oc.as_mut_ptr() as *mut c_void, oc.len(),
                frame.as_ptr() as *const c_void, frame.len(),
                std::ptr::null(), 0,
            );
            let rr = (decomp_usingdict.1)(
                dctx_r, or.as_mut_ptr() as *mut c_void, or.len(),
                frame.as_ptr() as *const c_void, frame.len(),
                std::ptr::null(), 0,
            );
            (free_dctx.0)(dctx_c);
            (free_dctx.1)(dctx_r);
            assert_zstd_parity(&z, "ERRORS row 144/229: decode with NO dictionary", rc, rr);
        }

        // --- static CDict/DDict: undersized + misaligned workspace ----------
        // Must return NULL identically on both libraries.
        let cparams = (get_cparams.0)(6, src.len() as u64, dict.len());
        // Undersized workspace.
        {
            let mut ws = vec![0u8; 64]; // far too small
            let cc = (init_static_cd.0)(
                ws.as_mut_ptr() as *mut c_void, ws.len(),
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams,
            );
            let cr = (init_static_cd.1)(
                ws.as_mut_ptr() as *mut c_void, ws.len(),
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams,
            );
            assert_eq!(
                cc.is_null(),
                cr.is_null(),
                "ERRORS row (static CDict undersized): null-ness differs"
            );
            let dc = (init_static_dd.0)(
                ws.as_mut_ptr() as *mut c_void, ws.len(),
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto,
            );
            let dr = (init_static_dd.1)(
                ws.as_mut_ptr() as *mut c_void, ws.len(),
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto,
            );
            assert_eq!(
                dc.is_null(),
                dr.is_null(),
                "ERRORS row (static DDict undersized): null-ness differs"
            );
        }
        // Misaligned workspace (offset the base pointer by 1 byte) — even with
        // a generous size, both libs must handle alignment identically.
        {
            let mut ws = vec![0u8; 256 * 1024];
            let base = ws.as_mut_ptr().add(1); // deliberately misaligned
            let len = ws.len() - 1;
            let cc = (init_static_cd.0)(
                base as *mut c_void, len,
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams,
            );
            let cr = (init_static_cd.1)(
                base as *mut c_void, len,
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto, cparams,
            );
            assert_eq!(
                cc.is_null(),
                cr.is_null(),
                "ERRORS row (static CDict misaligned): null-ness differs (C={cc:?} R={cr:?})"
            );
            let dc = (init_static_dd.0)(
                base as *mut c_void, len,
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto,
            );
            let dr = (init_static_dd.1)(
                base as *mut c_void, len,
                dict.as_ptr() as *const c_void, dict.len(),
                ZSTD_dlm_byCopy, ZSTD_dct_auto,
            );
            assert_eq!(
                dc.is_null(),
                dr.is_null(),
                "ERRORS row (static DDict misaligned): null-ness differs"
            );
        }
    }
}

// ZSTD_compress_usingDict(cctx, dst, dstCap, src, srcSize, dict, dictSize, level)
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t, c_int,
) -> size_t;
// ZSTD_decompress_usingDict(dctx, dst, dstCap, src, srcSize, dict, dictSize)
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t,
) -> size_t;

// ============================================================================
//  DICTIONARY BUILDER (ZDICT / COVER / fastCover) — ERRORS.md rows 231-262
// ============================================================================

const FILL: u8 = 0xAA;

/// Build a small training corpus (kept small so training stays fast).
fn small_corpus(rng: &mut Rng, n: usize, lo: usize, hi: usize) -> (Vec<u8>, Vec<size_t>) {
    const MOTIF: &[u8] = b"shared-motif-for-dictionary-training-1234567890";
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(n);
    for i in 0..n {
        let len = (lo + rng.below(hi - lo + 1)).max(8);
        let mut s = gen(if i % 2 == 0 { Shape::Text } else { Shape::Repetitive }, len, rng);
        if s.len() > MOTIF.len() + 2 {
            let off = rng.below(s.len() - MOTIF.len());
            s[off..off + MOTIF.len()].copy_from_slice(MOTIF);
        }
        sizes.push(s.len());
        buf.extend_from_slice(&s);
    }
    (buf, sizes)
}

/// ERRORS rows 231, 236, 239, 248, 249, 260, 261 and related: capacity- and
/// sample-count-based rejections of the basic trainer and the getDictID /
/// getDictHeaderSize helpers on empty/short/wrong-magic/truncated buffers.
#[test]
fn zdict_basic_and_headers_invalid() {
    let train = fnpair!("ZDICT_trainFromBuffer", FnTrain);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);
    let get_id = fnpair!("ZDICT_getDictID", FnGetDictID);
    let get_hdr = fnpair!("ZDICT_getDictHeaderSize", FnGetHdrSize);

    let mut rng = Rng::new(0xD1C7_0231);
    let (buf, sizes) = small_corpus(&mut rng, 40, 24, 200);

    unsafe {
        // (row, desc, dictCap, nbSamples override, samplesPtr null, sizesPtr null, sizesMutator)
        struct C {
            rows: &'static str,
            desc: &'static str,
            cap: usize,
            nb: c_uint,
            null_buf: bool,
            null_sizes: bool,
            bad_sizes: bool, // make sizes sum exceed buffer
        }
        let nb_all = sizes.len() as c_uint;
        let cases = [
            C { rows: "236", desc: "dictBufferCapacity < ZDICT_DICTSIZE_MIN(256)", cap: 64, nb: nb_all, null_buf: false, null_sizes: false, bad_sizes: false },
            C { rows: "240/248", desc: "nbSamples=0", cap: 4096, nb: 0, null_buf: false, null_sizes: false, bad_sizes: false },
            C { rows: "240/244", desc: "nbSamples=1", cap: 4096, nb: 1, null_buf: false, null_sizes: false, bad_sizes: false },
            C { rows: "240", desc: "samplesSizes sum > buffer (total too large)", cap: 4096, nb: nb_all, null_buf: false, null_sizes: false, bad_sizes: true },
        ];

        for case in &cases {
            let mut dc = vec![FILL; case.cap];
            let mut dr = vec![FILL; case.cap];
            let mut sizes2 = sizes.clone();
            if case.bad_sizes {
                // inflate last size so the sum exceeds the buffer length
                *sizes2.last_mut().unwrap() = buf.len() + 1024;
            }
            let buf_ptr = if case.null_buf {
                std::ptr::null()
            } else {
                buf.as_ptr() as *const c_void
            };
            let sizes_ptr = if case.null_sizes { std::ptr::null() } else { sizes2.as_ptr() };
            let rc = (train.0)(dc.as_mut_ptr() as *mut c_void, case.cap, buf_ptr, sizes_ptr, case.nb);
            let rr = (train.1)(dr.as_mut_ptr() as *mut c_void, case.cap, buf_ptr, sizes_ptr, case.nb);
            let ctx = format!("ERRORS row {}: trainFromBuffer {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // ERRORS row 240 (total sample size < ZDICT_MIN_SAMPLES_SIZE): a corpus
        // whose concatenated samples are far below the minimum training size.
        {
            let tiny_buf = vec![0x42u8; 40];
            let tiny_sizes: Vec<size_t> = vec![8, 8, 8, 8, 8]; // 5 tiny samples, 40 bytes total
            let cap = 4096usize;
            let mut dc = vec![FILL; cap];
            let mut dr = vec![FILL; cap];
            let rc = (train.0)(
                dc.as_mut_ptr() as *mut c_void, cap, tiny_buf.as_ptr() as *const c_void,
                tiny_sizes.as_ptr(), tiny_sizes.len() as c_uint,
            );
            let rr = (train.1)(
                dr.as_mut_ptr() as *mut c_void, cap, tiny_buf.as_ptr() as *const c_void,
                tiny_sizes.as_ptr(), tiny_sizes.len() as c_uint,
            );
            let ctx = "ERRORS row 240: total sample size too small";
            assert_zdict_parity(&is_err, &err_name, ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // NULL samplesBuffer / NULL samplesSizes are UNDEFINED BEHAVIOUR: the
        // C trainer dereferences these pointers without a NULL guard (the API
        // contract requires valid non-NULL buffers). Passing NULL crashes the
        // C library itself, so this is UB-by-contract, not a Rust divergence.
        eprintln!(
            "ERRORS row 240: NULL samplesBuffer / NULL samplesSizes are UB-by-contract \
             (C dereferences them without a NULL check) — not constructed."
        );

        // --- getDictID / getDictHeaderSize on empty/short/wrong-magic/trunc --
        // (rows 230/231 dictionary_corrupted for header size).
        let mut buffers: Vec<(String, Vec<u8>)> = Vec::new();
        buffers.push(("empty".into(), vec![]));
        buffers.push(("1byte".into(), vec![0x11]));
        buffers.push(("4byte".into(), vec![1, 2, 3, 4]));
        buffers.push(("8byte-wrongmagic".into(), vec![0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4]));
        {
            // right magic but truncated header (only magic + a few bytes)
            let mut m = 0xEC30A437u32.to_le_bytes().to_vec();
            m.extend_from_slice(&[0u8; 6]); // total 10 bytes, still too short for a full header
            buffers.push(("magic-trunc".into(), m));
        }
        for (name, b) in &buffers {
            let ctx = format!("ERRORS row 230/231: getDict* /{name} len={}", b.len());
            let idc = (get_id.0)(b.as_ptr() as *const c_void, b.len());
            let idr = (get_id.1)(b.as_ptr() as *const c_void, b.len());
            assert_eq!(idc, idr, "{ctx}: getDictID differs (C={idc} R={idr})");
            let hc = (get_hdr.0)(b.as_ptr() as *const c_void, b.len());
            let hr = (get_hdr.1)(b.as_ptr() as *const c_void, b.len());
            assert_zdict_parity(&is_err, &err_name, &format!("{ctx}: getDictHeaderSize"), hc, hr);
        }
    }
}

/// ERRORS rows 243-254 (COVER) : parameter and sample-count rejections for
/// `ZDICT_trainFromBuffer_cover` and the optimize variant, plus field-for-field
/// comparison of the MUTATED params struct after a failed optimize* call.
#[test]
fn zdict_cover_invalid() {
    let train = fnpair!("ZDICT_trainFromBuffer_cover", FnTrainCover);
    let opt = fnpair!("ZDICT_optimizeTrainFromBuffer_cover", FnOptCover);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);

    let mut rng = Rng::new(0xD1C7_0243);
    let (buf, sizes) = small_corpus(&mut rng, 30, 24, 160);
    let nb = sizes.len() as c_uint;

    // helper to make a baseline valid-ish params struct
    let mk = |k: c_uint, d: c_uint, sp: f64, nt: c_uint| {
        let mut p = ZDICT_cover_params_t::default();
        p.k = k;
        p.d = d;
        p.steps = 0;
        p.nbThreads = nt;
        p.splitPoint = sp;
        p.zParams.compressionLevel = 3;
        p
    };

    unsafe {
        // Non-optimize: COVER_checkParameters failures (row 247) + capacity
        // (249) + nbSamples=0 (248).
        struct C {
            rows: &'static str,
            desc: &'static str,
            p: ZDICT_cover_params_t,
            cap: usize,
            nb: c_uint,
        }
        let cases = [
            C { rows: "247", desc: "d=0", p: mk(200, 0, 0.0, 0), cap: 4096, nb },
            C { rows: "247", desc: "k=0", p: mk(0, 8, 0.0, 0), cap: 4096, nb },
            C { rows: "247", desc: "d>k (d=8,k=6)", p: mk(6, 8, 0.0, 0), cap: 4096, nb },
            C { rows: "249", desc: "dictBufferCapacity < 256", p: mk(200, 8, 0.0, 0), cap: 64, nb },
            C { rows: "248", desc: "nbSamples=0", p: mk(200, 8, 0.0, 0), cap: 4096, nb: 0 },
            C { rows: "254", desc: "nbThreads>1 (no ZSTD_MULTITHREAD)", p: mk(200, 8, 0.0, 4), cap: 4096, nb },
        ];
        for case in &cases {
            let mut dc = vec![FILL; case.cap];
            let mut dr = vec![FILL; case.cap];
            let rc = (train.0)(
                dc.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, case.p,
            );
            let rr = (train.1)(
                dr.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, case.p,
            );
            let ctx = format!("ERRORS row {}: cover {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // Optimize variant: splitPoint invalid (250), bad k/d range (251),
        // nbSamples=0 (252), capacity (253). Compare MUTATED params too.
        struct O {
            rows: &'static str,
            desc: &'static str,
            p: ZDICT_cover_params_t,
            cap: usize,
            nb: c_uint,
        }
        let opt_cases = [
            O { rows: "250", desc: "splitPoint <= 0", p: mk(200, 8, -0.5, 1), cap: 4096, nb },
            O { rows: "250", desc: "splitPoint > 1", p: mk(200, 8, 1.5, 1), cap: 4096, nb },
            O { rows: "252", desc: "nbSamples=0", p: mk(0, 0, 0.0, 1), cap: 4096, nb: 0 },
            O { rows: "253", desc: "dictBufferCapacity < 256", p: mk(0, 0, 0.0, 1), cap: 64, nb },
        ];
        for case in &opt_cases {
            let mut dc = vec![FILL; case.cap];
            let mut dr = vec![FILL; case.cap];
            let mut pc = case.p;
            let mut pr = case.p;
            let rc = (opt.0)(
                dc.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, &mut pc,
            );
            let rr = (opt.1)(
                dr.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, &mut pr,
            );
            let ctx = format!("ERRORS row {}: optimizeCover {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            // MUTATED params struct must match field-for-field after the call.
            assert_eq!(pc, pr, "{ctx}: mutated cover_params differ (C={pc:?} R={pr:?})");
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }
    }
}

/// ERRORS rows 255-261 (fastCover): parameter rejections for
/// `ZDICT_trainFromBuffer_fastCover` + optimize variant (adds f, accel),
/// plus mutated-params comparison.
#[test]
fn zdict_fastcover_invalid() {
    let train = fnpair!("ZDICT_trainFromBuffer_fastCover", FnTrainFast);
    let opt = fnpair!("ZDICT_optimizeTrainFromBuffer_fastCover", FnOptFast);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);

    let mut rng = Rng::new(0xD1C7_0255);
    let (buf, sizes) = small_corpus(&mut rng, 30, 24, 160);
    let nb = sizes.len() as c_uint;

    let mk = |k: c_uint, d: c_uint, f: c_uint, accel: c_uint, sp: f64, nt: c_uint| {
        let mut p = ZDICT_fastCover_params_t::default();
        p.k = k;
        p.d = d;
        p.f = f;
        p.steps = 0;
        p.nbThreads = nt;
        p.splitPoint = sp;
        p.accel = accel;
        p.zParams.compressionLevel = 3;
        p
    };

    unsafe {
        // Non-optimize (row 259 FASTCOVER_checkParameters, 261 capacity, 260 nb=0).
        struct C {
            rows: &'static str,
            desc: &'static str,
            p: ZDICT_fastCover_params_t,
            cap: usize,
            nb: c_uint,
        }
        let cases = [
            C { rows: "259", desc: "d not in {6,8} (d=7)", p: mk(200, 7, 20, 1, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "k=0", p: mk(0, 8, 20, 1, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "d>k", p: mk(6, 8, 20, 1, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "f=0", p: mk(200, 8, 0, 1, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "f>MAX_F (f=32)", p: mk(200, 8, 32, 1, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "accel=0", p: mk(200, 8, 20, 0, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "accel=11 (>10)", p: mk(200, 8, 20, 11, 0.0, 0), cap: 4096, nb },
            C { rows: "259", desc: "splitPoint > 1", p: mk(200, 8, 20, 1, 1.5, 0), cap: 4096, nb },
            C { rows: "261", desc: "dictBufferCapacity < 256", p: mk(200, 8, 20, 1, 0.0, 0), cap: 64, nb },
            C { rows: "260", desc: "nbSamples=0", p: mk(200, 8, 20, 1, 0.0, 0), cap: 4096, nb: 0 },
        ];
        for case in &cases {
            let mut dc = vec![FILL; case.cap];
            let mut dr = vec![FILL; case.cap];
            let rc = (train.0)(
                dc.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, case.p,
            );
            let rr = (train.1)(
                dr.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, case.p,
            );
            let ctx = format!("ERRORS row {}: fastCover {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // Optimize variant + mutated params comparison.
        struct O {
            rows: &'static str,
            desc: &'static str,
            p: ZDICT_fastCover_params_t,
            cap: usize,
            nb: c_uint,
        }
        let opt_cases = [
            O { rows: "259", desc: "splitPoint <= 0", p: mk(200, 8, 20, 1, -0.5, 1), cap: 4096, nb },
            O { rows: "260", desc: "nbSamples=0", p: mk(200, 8, 20, 1, 0.0, 1), cap: 4096, nb: 0 },
            O { rows: "261", desc: "dictBufferCapacity < 256", p: mk(200, 8, 20, 1, 0.0, 1), cap: 64, nb },
            O { rows: "259", desc: "accel=11", p: mk(200, 8, 20, 11, 0.0, 1), cap: 4096, nb },
        ];
        for case in &opt_cases {
            let mut dc = vec![FILL; case.cap];
            let mut dr = vec![FILL; case.cap];
            let mut pc = case.p;
            let mut pr = case.p;
            let rc = (opt.0)(
                dc.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, &mut pc,
            );
            let rr = (opt.1)(
                dr.as_mut_ptr() as *mut c_void, case.cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), case.nb, &mut pr,
            );
            let ctx = format!("ERRORS row {}: optimizeFastCover {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            assert_eq!(pc, pr, "{ctx}: mutated fastCover_params differ (C={pc:?} R={pr:?})");
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }
    }
}

/// ERRORS rows 232-242 (legacy trainer) and 235-237 (finalizeDictionary).
/// legacy: out-of-range selectivityLevel + capacity/too-few-samples.
/// finalize: undersized dictBuffer, customDictContent larger than buffer,
/// zero samples.
#[test]
fn zdict_legacy_and_finalize_invalid() {
    let train_legacy = fnpair!("ZDICT_trainFromBuffer_legacy", FnTrainLegacy);
    let finalize = fnpair!("ZDICT_finalizeDictionary", FnFinalize);
    let is_err = fnpair!("ZDICT_isError", FnIsError);
    let err_name = fnpair!("ZDICT_getErrorName", FnErrName);

    let mut rng = Rng::new(0xD1C7_0232);
    let (buf, sizes) = small_corpus(&mut rng, 40, 24, 200);
    let nb = sizes.len() as c_uint;

    unsafe {
        // --- legacy trainer: out-of-range selectivityLevel + tiny capacity ---
        for &(rows, desc, sel, cap, nbs) in &[
            ("232/240", "selectivityLevel huge", 0xFFFF_FFFFu32, 4096usize, nb),
            ("239", "maxDictSize < 256", 1u32, 64usize, nb),
            ("240", "nbSamples=0", 1u32, 4096usize, 0u32),
        ] {
            let mut p = ZDICT_legacy_params_t::default();
            p.selectivityLevel = sel;
            p.zParams.compressionLevel = 3;
            let mut dc = vec![FILL; cap];
            let mut dr = vec![FILL; cap];
            let rc = (train_legacy.0)(
                dc.as_mut_ptr() as *mut c_void, cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), nbs, p,
            );
            let rr = (train_legacy.1)(
                dr.as_mut_ptr() as *mut c_void, cap, buf.as_ptr() as *const c_void,
                sizes.as_ptr(), nbs, p,
            );
            let ctx = format!("ERRORS row {rows}: legacy {desc}");
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // --- finalizeDictionary error paths ---------------------------------
        let content = gen(Shape::Text, 2048, &mut rng);
        // row 236: dictBufferCapacity < ZDICT_DICTSIZE_MIN(256)
        // row 235: dictBufferCapacity < dictContentSize (customDictContent bigger than buffer)
        // zero samples: nbSamples=0
        struct F {
            rows: &'static str,
            desc: &'static str,
            max_dict: usize,
            content_len: usize,
            nb: c_uint,
        }
        let fcases = [
            F { rows: "236", desc: "maxDictSize < 256", max_dict: 128, content_len: content.len(), nb },
            F { rows: "235", desc: "customDictContent > buffer", max_dict: 512, content_len: 2048, nb },
            F { rows: "235/237", desc: "zero samples", max_dict: 4096, content_len: content.len(), nb: 0 },
        ];
        for case in &fcases {
            let mut p = ZDICT_params_t::default();
            p.compressionLevel = 3;
            let mut dc = vec![FILL; case.max_dict];
            let mut dr = vec![FILL; case.max_dict];
            let rc = (finalize.0)(
                dc.as_mut_ptr() as *mut c_void, case.max_dict,
                content.as_ptr() as *const c_void, case.content_len,
                buf.as_ptr() as *const c_void, sizes.as_ptr(), case.nb, p,
            );
            let rr = (finalize.1)(
                dr.as_mut_ptr() as *mut c_void, case.max_dict,
                content.as_ptr() as *const c_void, case.content_len,
                buf.as_ptr() as *const c_void, sizes.as_ptr(), case.nb, p,
            );
            let ctx = format!("ERRORS row {}: finalizeDictionary {}", case.rows, case.desc);
            assert_zdict_parity(&is_err, &err_name, &ctx, rc, rr);
            if (is_err.0)(rc) == 0 {
                assert_bytes_eq(&format!("{ctx}: dict buffer"), &dc, &dr);
            }
        }

        // ERRORS row 262 (divsufsort failure) and rows 232-234/238/242/246/258
        // (internal allocation failures) are unreachable from the public API
        // with valid-shaped inputs — they require malloc to fail or an internal
        // suffix-sort to fail, which cannot be deterministically triggered
        // across the FFI boundary. Report them.
        eprintln!(
            "ERRORS rows 232,233,234,238,242,246,258,262: unreachable — require internal \
             malloc/divsufsort failure that cannot be forced across the public FFI boundary"
        );
    }
}
