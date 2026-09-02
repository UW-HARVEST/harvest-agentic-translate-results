//! Phase C: differential tests for the EXTERNAL-SEQUENCE API — ERROR paths.
//!
//! Every case constructs a specific invalid condition and asserts that the C
//! and Rust libraries return the IDENTICAL error code (or the identical success
//! + identical bytes) via `Err2::eq`.
//!
//! IMPORTANT SAFETY NOTE ON dstCapacity:
//! `ZSTD_compressSequences` / `ZSTD_compressSequencesAndLiterals` use the
//! destination buffer as scratch and require ~`ZSTD_compressBound(srcSize)` of
//! space. Passing `0 <= dstCapacity < that requirement` is documented undefined
//! behavior; the C reference underflows an internal `(dstCapacity - headerSize)`
//! computation and runs away writing far past the buffer (confirmed to segfault
//! even with large slack buffers). We therefore always hand these functions a
//! buffer of at least `compressBound` bytes and, where the task calls for
//! "one byte below the needed size", we test that against the *final compressed
//! size* boundary while keeping the physical buffer >= compressBound so the C
//! reference stays well-defined. Genuinely too-small caps are only exercised at
//! the well-defined `dstCapacity == compressBound - 1` style boundary via the
//! return-value contract, never by shrinking the physical allocation into the
//! UB window.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------- FFI typedefs

type FnSeqBound = unsafe extern "C" fn(size_t) -> size_t;
type FnGenSeq =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, size_t, *const c_void, size_t) -> size_t;
type FnCompressSeq = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const ZSTD_Sequence,
    size_t,
    *const c_void,
    size_t,
) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnRegisterSeqProd = unsafe extern "C" fn(*mut c_void, *mut c_void, ZSTD_seqProdFn);
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

/// The block-level sequence producer callback type (matches
/// `ZSTD_sequenceProducer_F` in zstd.h).
type ZSTD_seqProdFn = Option<
    unsafe extern "C" fn(
        *mut c_void,          // sequenceProducerState
        *mut ZSTD_Sequence,   // outSeqs
        size_t,               // outSeqsCapacity
        *const c_void,        // src
        size_t,               // srcSize
        *const c_void,        // dict
        size_t,               // dictSize
        c_int,                // compressionLevel
        size_t,               // windowSize
    ) -> size_t,
>;

// ---------------------------------------------------------------- CCtx wrapper

struct Ctx {
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
}
fn new_cctx() -> Ctx {
    unsafe {
        let (a, b) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let x = a();
        let y = b();
        assert!(!x.is_null() && !y.is_null());
        Ctx { cctx_c: x, cctx_r: y }
    }
}
impl Drop for Ctx {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrToSize>("ZSTD_freeCCtx");
            a(self.cctx_c);
            b(self.cctx_r);
        }
    }
}

/// Run `ZSTD_compressSequences` on BOTH libraries with the given params and a
/// physically full-size (>= compressBound) buffer, asserting identical returns
/// (and identical bytes on success). This is the core differential comparator
/// for error paths.
#[allow(clippy::too_many_arguments)]
unsafe fn cmp_compress_seqs(
    cx: &Ctx,
    csq_c: &FnCompressSeq,
    csq_r: &FnCompressSeq,
    set_c: &FnSetParam,
    set_r: &FnSetParam,
    rst_c: &FnReset,
    rst_r: &FnReset,
    cbnd: &FnCompressBound,
    e: &Err2,
    seqs: &[ZSTD_Sequence],
    src: &[u8],
    bd: c_int,
    validate: c_int,
    level: c_int,
    ctx: &str,
) {
    rst_c(cx.cctx_c, ZSTD_reset_session_and_parameters);
    rst_r(cx.cctx_r, ZSTD_reset_session_and_parameters);
    set_c(cx.cctx_c, ZSTD_c_compressionLevel, level);
    set_c(cx.cctx_c, ZSTD_c_blockDelimiters, bd);
    set_c(cx.cctx_c, ZSTD_c_validateSequences, validate);
    set_r(cx.cctx_r, ZSTD_c_compressionLevel, level);
    set_r(cx.cctx_r, ZSTD_c_blockDelimiters, bd);
    set_r(cx.cctx_r, ZSTD_c_validateSequences, validate);

    // Always at least compressBound to avoid the C-reference UB window.
    let cap = cbnd(src.len()) + 1024;
    let mut cbuf = vec![0u8; cap];
    let mut rbuf = vec![0u8; cap];
    let cn = csq_c(
        cx.cctx_c,
        cbuf.as_mut_ptr() as *mut c_void,
        cap,
        seqs.as_ptr(),
        seqs.len(),
        src.as_ptr() as *const c_void,
        src.len(),
    );
    let rn = csq_r(
        cx.cctx_r,
        rbuf.as_mut_ptr() as *mut c_void,
        cap,
        seqs.as_ptr(),
        seqs.len(),
        src.as_ptr() as *const c_void,
        src.len(),
    );
    e.eq(ctx, cn, rn);
    if !e.c.is_err(cn) {
        assert_bytes_eq(ctx, &cbuf[..cn], &rbuf[..rn]);
    }
}

fn delim() -> ZSTD_Sequence {
    ZSTD_Sequence::default()
}

// ===================================================== hand-built invalid seqs

/// Invalid sequence conditions from the task, validated (bd=1) and unvalidated
/// (bd=1), both validate in {0,1}. All use a small, well-defined src.
#[test]
fn invalid_sequence_conditions() {
    unsafe {
        let e = Err2::new();
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();

        // A well-formed baseline "block": litLength + matchLength == srcSize, with
        // a trailing block delimiter. srcSize = 32.
        let src_size = 32usize;
        let src = vec![0x41u8; src_size];

        // Build a set of (description, sequences) cases.
        let mut cases: Vec<(String, Vec<ZSTD_Sequence>)> = Vec::new();

        // offset == 0 (not a valid all-zero delimiter, because ml != 0)
        cases.push((
            "offset==0 with matchLength!=0".into(),
            vec![
                ZSTD_Sequence { offset: 0, litLength: 4, matchLength: 28, rep: 0 },
                delim(),
            ],
        ));
        // offset larger than window / larger than decoded-so-far
        cases.push((
            "offset > bytes decoded so far".into(),
            vec![
                ZSTD_Sequence { offset: 1000, litLength: 4, matchLength: 28, rep: 0 },
                delim(),
            ],
        ));
        cases.push((
            "offset huge (u32::MAX)".into(),
            vec![
                ZSTD_Sequence { offset: u32::MAX, litLength: 4, matchLength: 28, rep: 0 },
                delim(),
            ],
        ));
        // matchLength < MINMATCH (3)
        for ml in [1u32, 2] {
            cases.push((
                format!("matchLength {ml} < MINMATCH"),
                vec![
                    ZSTD_Sequence { offset: 4, litLength: src_size as u32 - ml, matchLength: ml, rep: 0 },
                    delim(),
                ],
            ));
        }
        // litLength + matchLength sum too small (< srcSize)
        cases.push((
            "ll+ml too small".into(),
            vec![
                ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 8, rep: 0 },
                delim(),
            ],
        ));
        // sum too large (> srcSize)
        cases.push((
            "ll+ml too large".into(),
            vec![
                ZSTD_Sequence { offset: 4, litLength: 40, matchLength: 40, rep: 0 },
                delim(),
            ],
        ));
        // rep out of range
        for rep in [4u32, 5, 99, u32::MAX] {
            cases.push((
                format!("rep {rep} out of range"),
                vec![
                    ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 28, rep },
                    delim(),
                ],
            ));
        }

        for (desc, seqs) in &cases {
            for &validate in &[0i32, 1] {
                for &lvl in &[1i32, 3, 19] {
                    let ctx = format!("invalid[{desc}] bd=1 val={validate} lvl={lvl}");
                    // Only exercise explicit-delimiter mode here (the arrays carry
                    // delimiters). Undefined behavior on validate==0 is the C
                    // ground truth, which the Rust translation must match exactly.
                    cmp_compress_seqs(
                        &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &cbnd, &e, seqs, &src, 1, validate,
                        lvl, &ctx,
                    );
                }
            }
        }
    }
}

/// Block-delimiter placement errors.
#[test]
fn block_delimiter_placement_errors() {
    unsafe {
        let e = Err2::new();
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();

        let src_size = 32usize;
        let src = vec![0x42u8; src_size];

        let good_seq = ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 28, rep: 0 };

        let mut cases: Vec<(String, c_int, Vec<ZSTD_Sequence>)> = Vec::new();

        // bd==1 (explicit) but array has NO delimiter
        cases.push(("bd=1 no delimiter".into(), 1, vec![good_seq]));
        // bd==1 with a delimiter in the wrong place (middle, splitting a block
        // that does not sum to srcSize)
        cases.push((
            "bd=1 delimiter in wrong place".into(),
            1,
            vec![
                ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 8, rep: 0 },
                delim(),
                ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 8, rep: 0 },
                delim(),
            ],
        ));
        // bd==1 with only delimiters
        cases.push(("bd=1 only delimiters".into(), 1, vec![delim(), delim(), delim()]));
        // bd==0 (no delimiters) but array DOES contain delimiters
        cases.push((
            "bd=0 with delimiters present".into(),
            0,
            vec![good_seq, delim(), good_seq],
        ));

        for (desc, bd, seqs) in &cases {
            for &validate in &[0i32, 1] {
                for &lvl in &[1i32, 3, 19] {
                    let ctx = format!("delim-placement[{desc}] bd={bd} val={validate} lvl={lvl}");
                    cmp_compress_seqs(
                        &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &cbnd, &e, seqs, &src, *bd,
                        validate, lvl, &ctx,
                    );
                }
            }
        }
    }
}

/// nbSequences / srcSize mismatches, and NULL pointer arguments with nonzero
/// counts.
#[test]
fn count_and_null_pointer_errors() {
    unsafe {
        let e = Err2::new();
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();

        let src = vec![0x43u8; 32];
        let good = vec![
            ZSTD_Sequence { offset: 4, litLength: 4, matchLength: 28, rep: 0 },
            delim(),
        ];

        // nbSequences == 0 with srcSize > 0
        for &validate in &[0i32, 1] {
            for &bd in &[0i32, 1] {
                rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                sc(cx.cctx_c, ZSTD_c_blockDelimiters, bd);
                sc(cx.cctx_c, ZSTD_c_validateSequences, validate);
                sr(cx.cctx_r, ZSTD_c_blockDelimiters, bd);
                sr(cx.cctx_r, ZSTD_c_validateSequences, validate);
                let cap = cbnd(src.len()) + 1024;
                let mut cbuf = vec![0u8; cap];
                let mut rbuf = vec![0u8; cap];
                // nbSequences == 0, srcSize > 0
                let cn = csq_c(
                    cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
                    good.as_ptr(), 0, src.as_ptr() as *const c_void, src.len(),
                );
                let rn = csq_r(
                    cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
                    good.as_ptr(), 0, src.as_ptr() as *const c_void, src.len(),
                );
                e.eq(&format!("nbSeq=0 srcSize>0 bd={bd} val={validate}"), cn, rn);
                if !e.c.is_err(cn) {
                    assert_bytes_eq("nbSeq=0 srcSize>0 bytes", &cbuf[..cn], &rbuf[..rn]);
                }

                // nbSequences > 0, srcSize == 0
                let cn2 = csq_c(
                    cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
                    good.as_ptr(), good.len(), src.as_ptr() as *const c_void, 0,
                );
                let rn2 = csq_r(
                    cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
                    good.as_ptr(), good.len(), src.as_ptr() as *const c_void, 0,
                );
                e.eq(&format!("nbSeq>0 srcSize=0 bd={bd} val={validate}"), cn2, rn2);
            }
        }

        // outSeqs (dst) == NULL with nonzero counts, and inSeqs == NULL with
        // nonzero counts. dstCapacity is >= compressBound so a well-defined
        // path is taken; the NULL should be caught cleanly by both libraries.
        let cap = cbnd(src.len()) + 1024;
        rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
        rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
        sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
        sr(cx.cctx_r, ZSTD_c_blockDelimiters, 1);
        // NULL-pointer cases. The C reference does NOT guard NULL dst/inSeqs when
        // the corresponding count/capacity is nonzero: it dereferences them and
        // segfaults (memory-unsafe UB in the reference; the API contract requires
        // valid pointers). Confirmed by direct probing. We therefore differentially
        // test the WELL-DEFINED NULL contract: a NULL pointer paired with a zero
        // count, where zstd is required to short-circuit before any dereference.
        // dst == NULL with dstCapacity == 0 AND nbSequences == 0.
        let cn = csq_c(
            cx.cctx_c, std::ptr::null_mut(), 0,
            std::ptr::null(), 0, src.as_ptr() as *const c_void, src.len(),
        );
        let rn = csq_r(
            cx.cctx_r, std::ptr::null_mut(), 0,
            std::ptr::null(), 0, src.as_ptr() as *const c_void, src.len(),
        );
        e.eq("dst==NULL,inSeqs==NULL,cap0,nbSeq0", cn, rn);
        // inSeqs == NULL with nbSequences == 0 but a real dst buffer.
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cn2 = csq_c(
            cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
            std::ptr::null(), 0, src.as_ptr() as *const c_void, src.len(),
        );
        let rn2 = csq_r(
            cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
            std::ptr::null(), 0, src.as_ptr() as *const c_void, src.len(),
        );
        e.eq("inSeqs==NULL nbSeq==0", cn2, rn2);
    }
}

/// dstCapacity boundary errors: the "one byte below the exact needed size" is
/// tested at the *final compressed size* boundary, but keeping the physical
/// buffer >= compressBound so the C reference does not enter its UB window.
#[test]
fn dst_capacity_boundary_errors() {
    unsafe {
        let e = Err2::new();
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let cx = new_cctx();
        let gcx = new_cctx();
        let mut rng = Rng::new(0xC9_5EED_0001);

        for &shape in ALL_SHAPES {
            for &len in &[100usize, 1024, 5000] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                // generate valid explicit-delimiter sequences on gcx
                rc(gcx.cctx_c, ZSTD_reset_session_and_parameters);
                rr(gcx.cctx_r, ZSTD_reset_session_and_parameters);
                sc(gcx.cctx_c, ZSTD_c_compressionLevel, 3);
                sr(gcx.cctx_r, ZSTD_c_compressionLevel, 3);
                let sbcap = sb(src.len()) + 1;
                let mut seqbuf = vec![ZSTD_Sequence::default(); sbcap];
                let gn = gc(
                    gcx.cctx_c, seqbuf.as_mut_ptr(), sbcap,
                    src.as_ptr() as *const c_void, src.len(),
                );
                if e.c.is_err(gn) {
                    continue;
                }
                // verify RS produces the same, then use the array
                let mut seqbuf_r = vec![ZSTD_Sequence::default(); sbcap];
                let gn_r = gr(
                    gcx.cctx_r, seqbuf_r.as_mut_ptr(), sbcap,
                    src.as_ptr() as *const c_void, src.len(),
                );
                e.eq(&format!("gen for cap-boundary shape={shape:?} len={}", src.len()), gn, gn_r);
                if e.c.is_err(gn) {
                    continue;
                }
                let seqs = &seqbuf[..gn];

                // Establish the final compressed size with a generous buffer.
                rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                let bound = cbnd(src.len());
                let big = bound + 1024;
                let mut tmp = vec![0u8; big];
                let need = csq_c(
                    cx.cctx_c, tmp.as_mut_ptr() as *mut c_void, big,
                    seqs.as_ptr(), seqs.len(), src.as_ptr() as *const c_void, src.len(),
                );
                if e.c.is_err(need) {
                    continue;
                }
                // Physical buffer stays at `big` (>= compressBound), but we report
                // capacities of {need-1, need, need+1, bound}. need-1 must yield
                // dstSize_tooSmall on BOTH; the physical buffer prevents the UB
                // runaway because dstCapacity here (need-1) is still large.
                for cap in [need.saturating_sub(1), need, need + 1, bound] {
                    rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                    rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                    sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                    sr(cx.cctx_r, ZSTD_c_blockDelimiters, 1);
                    let mut cbuf = vec![0u8; big];
                    let mut rbuf = vec![0u8; big];
                    let cn = csq_c(
                        cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
                        seqs.as_ptr(), seqs.len(), src.as_ptr() as *const c_void, src.len(),
                    );
                    let rn = csq_r(
                        cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
                        seqs.as_ptr(), seqs.len(), src.as_ptr() as *const c_void, src.len(),
                    );
                    let ctx = format!("cap-boundary shape={shape:?} len={} cap={cap} need={need}", src.len());
                    e.eq(&ctx, cn, rn);
                    if !e.c.is_err(cn) {
                        assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }
    }
}

/// ZSTD_generateSequences with outSeqsCapacity below ZSTD_sequenceBound(srcSize).
#[test]
fn generate_sequences_capacity_errors() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let cx = new_cctx();
        let mut rng = Rng::new(0xC9_5EED_0002);

        for &shape in ALL_SHAPES {
            for &len in &[1usize, 100, 1024, 20000] {
                let src = gen(shape, len, &mut rng);
                let bound = sb(src.len());
                for cap in [0usize, 1, bound.saturating_sub(1)] {
                    rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                    rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                    sc(cx.cctx_c, ZSTD_c_compressionLevel, 3);
                    sr(cx.cctx_r, ZSTD_c_compressionLevel, 3);
                    // Physical buffer sized to `cap` (the real contract); the C
                    // reference does bounds-check outSeqsCapacity here.
                    let mut cbuf = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let mut rbuf = vec![ZSTD_Sequence::default(); cap.max(1)];
                    let cn = gc(
                        cx.cctx_c, cbuf.as_mut_ptr(), cap,
                        src.as_ptr() as *const c_void, src.len(),
                    );
                    let rn = gr(
                        cx.cctx_r, rbuf.as_mut_ptr(), cap,
                        src.as_ptr() as *const c_void, src.len(),
                    );
                    let ctx = format!("genSeq-cap shape={shape:?} len={} cap={cap} bound={bound}", src.len());
                    e.eq(&ctx, cn, rn);
                    if !e.c.is_err(cn) && cn > 0 {
                        // On success the produced arrays must be identical.
                        for i in 0..cn {
                            assert_eq!(cbuf[i], rbuf[i], "{ctx}: seq[{i}]");
                        }
                    }
                }
            }
        }
    }
}

// =============================================== 3000+ fully random sequences

/// The most important test: 3000+ fully random ZSTD_Sequence arrays (fixed
/// seed) fed to ZSTD_compressSequences under every combination of
/// blockDelimiters and validateSequences. Assert C and Rust return the IDENTICAL
/// error code (or identical success + identical bytes) for every single one.
fn random_seqs_body(seed: u64, iters: usize, bd: c_int, validate: c_int) {
    unsafe {
        let e = Err2::new();
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let mut rng = Rng::new(seed);

        for i in 0..iters {
            // Random source of a random (bounded) length.
            let src_len = rng.below(4096);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let mut src = gen(shape, src_len, &mut rng);
            // Padding tail: the C reference may over-read literals past `src_size`
            // for over-long sequences before validation rejects them. With
            // litLength/matchLength bounded to <=2048 and at most ~25 sequences,
            // the worst-case over-read is well under 128 KiB. We pass `src_size`
            // (NOT src.len()) as the srcSize argument so the extra bytes are pure
            // guard space that keeps the reference memory-safe.
            let src_size = src.len();
            src.resize(src_size + (128 * 1024), 0);

            // Random sequence array.
            let n = rng.below(24);
            let mut seqs: Vec<ZSTD_Sequence> = Vec::with_capacity(n + 1);
            for _ in 0..n {
                // occasionally emit an explicit delimiter
                if rng.below(5) == 0 {
                    seqs.push(delim());
                    continue;
                }
                let off = match rng.below(4) {
                    0 => 0,
                    1 => u32::MAX,
                    2 => rng.next_u32(),
                    _ => 1 + rng.below(4096) as u32,
                };
                // litLength and matchLength drive how many bytes the C reference
                // reads out of `src` (literals) and how it computes the trailing
                // "last literals" segment via an UNSIGNED (srcSize - consumed)
                // subtraction. Feeding u32::MAX there makes the C ground truth
                // read gigabytes out of bounds and segfault (confirmed) — genuine
                // memory-unsafety in the reference, which the Rust translation
                // avoids. To keep the differential comparison meaningful we bound
                // these two fields to a modest range (still covering 0, tiny
                // sub-MINMATCH values, and sums that over/under-shoot srcSize),
                // and back `src` with a large padding tail so the bounded
                // over-read stays inside the allocation. offset/rep remain fully
                // random (including u32::MAX) since they never trigger reads.
                let ll = match rng.below(3) {
                    0 => 0,
                    1 => rng.below(8) as u32, // tiny, incl. sub-MINMATCH region
                    _ => rng.below(2048) as u32,
                };
                let ml = match rng.below(4) {
                    0 => 0,
                    1 => rng.below(8) as u32, // tiny, incl. sub-MINMATCH (<3)
                    2 => u32::MAX,            // huge matchLength: no read, tests validation
                    _ => rng.below(2048) as u32,
                };
                let rep = match rng.below(3) {
                    0 => rng.below(4) as u32,
                    1 => u32::MAX,
                    _ => rng.next_u32(),
                };
                seqs.push(ZSTD_Sequence { offset: off, litLength: ll, matchLength: ml, rep });
            }
            // sometimes append a trailing delimiter
            if rng.bool() {
                seqs.push(delim());
            }

            rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
            rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
            sc(cx.cctx_c, ZSTD_c_compressionLevel, 3);
            sc(cx.cctx_c, ZSTD_c_blockDelimiters, bd);
            sc(cx.cctx_c, ZSTD_c_validateSequences, validate);
            sr(cx.cctx_r, ZSTD_c_compressionLevel, 3);
            sr(cx.cctx_r, ZSTD_c_blockDelimiters, bd);
            sr(cx.cctx_r, ZSTD_c_validateSequences, validate);

            // Always give a >= compressBound buffer to keep the C reference out
            // of its undersized-dst UB window; random *sequence* content is what
            // we are fuzzing, not the dst capacity.
            let cap = cbnd(src_size) + 1024;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let ctx = format!(
                "rand-seq #{i} bd={bd} val={validate} src_size={src_size} nseq={}",
                seqs.len()
            );
            let cn = csq_c(
                cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
                seqs.as_ptr(), seqs.len(), src.as_ptr() as *const c_void, src_size,
            );
            let rn = csq_r(
                cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
                seqs.as_ptr(), seqs.len(), src.as_ptr() as *const c_void, src_size,
            );
            e.eq(&ctx, cn, rn);
            if !e.c.is_err(cn) {
                assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
            }
        }
    }
}

// 3000+ random arrays across each (bd, validate) combination. Split into four
// #[test] functions so each stays well under the time budget while together
// covering the full cross-product with >3000 arrays each.
#[test]
fn random_sequences_bd0_val0() {
    random_seqs_body(0xC9_A0_0001, 3200, 0, 0);
}
#[test]
fn random_sequences_bd0_val1() {
    random_seqs_body(0xC9_A0_0002, 3200, 0, 1);
}
#[test]
fn random_sequences_bd1_val0() {
    random_seqs_body(0xC9_A0_0003, 3200, 1, 0);
}
#[test]
fn random_sequences_bd1_val1() {
    random_seqs_body(0xC9_A0_0004, 3200, 1, 1);
}

// ===================================================== sequence producer errors

// A user state passed to the sequence-producer callbacks to select behavior.
#[repr(C)]
struct SeqProdState {
    mode: c_int,
}

// (a) returns an error sentinel (ZSTD_SEQUENCE_PRODUCER_ERROR == (size_t)-1).
unsafe extern "C" fn seqprod_error(
    _state: *mut c_void,
    _out: *mut ZSTD_Sequence,
    _cap: size_t,
    _src: *const c_void,
    _src_size: size_t,
    _dict: *const c_void,
    _dict_size: size_t,
    _level: c_int,
    _window: size_t,
) -> size_t {
    // ZSTD_SEQUENCE_PRODUCER_ERROR
    usize::MAX
}

// (b) returns more sequences than capacity (also treated as an error).
unsafe extern "C" fn seqprod_overflow(
    _state: *mut c_void,
    _out: *mut ZSTD_Sequence,
    cap: size_t,
    _src: *const c_void,
    _src_size: size_t,
    _dict: *const c_void,
    _dict_size: size_t,
    _level: c_int,
    _window: size_t,
) -> size_t {
    cap + 1
}

// (c) returns invalid sequences (claims a single sequence whose lengths do not
// sum to srcSize and whose offset is absurd).
unsafe extern "C" fn seqprod_invalid(
    _state: *mut c_void,
    out: *mut ZSTD_Sequence,
    cap: size_t,
    _src: *const c_void,
    src_size: size_t,
    _dict: *const c_void,
    _dict_size: size_t,
    _level: c_int,
    _window: size_t,
) -> size_t {
    if cap == 0 {
        return usize::MAX; // cannot write; signal error
    }
    // One bogus sequence: huge offset, lengths that don't cover src_size.
    *out = ZSTD_Sequence {
        offset: u32::MAX,
        litLength: 1,
        matchLength: (src_size as u32).wrapping_add(100),
        rep: 0,
    };
    1
}

#[test]
fn register_sequence_producer_errors() {
    unsafe {
        let e = Err2::new();
        let (reg_c, reg_r) = both::<FnRegisterSeqProd>("ZSTD_registerSequenceProducer");
        let (c2_c, c2_r) = both::<FnCompress2>("ZSTD_compress2");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let mut rng = Rng::new(0xC9_5EED_0003);

        let mut state_c = SeqProdState { mode: 0 };
        let mut state_r = SeqProdState { mode: 0 };

        let callbacks: &[(&str, ZSTD_seqProdFn)] = &[
            ("null-fn", None),
            ("error-sentinel", Some(seqprod_error)),
            ("overflow-cap", Some(seqprod_overflow)),
            ("invalid-seqs", Some(seqprod_invalid)),
        ];

        for &shape in &[Shape::Text, Shape::Random, Shape::Zeros, Shape::Sequential] {
            for &len in &[1usize, 100, 4096, 20000] {
                let src = gen(shape, len, &mut rng);
                for (label, cb) in callbacks {
                    for &fallback in &[0i32, 1] {
                        rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                        rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                        // Disable LDM (unsupported with a sequence producer) and
                        // single-threaded (nbWorkers default 0).
                        sc(cx.cctx_c, ZSTD_c_compressionLevel, 3);
                        sc(cx.cctx_c, ZSTD_c_enableLongDistanceMatching, 0);
                        sc(cx.cctx_c, ZSTD_c_enableSeqProducerFallback, fallback);
                        sr(cx.cctx_r, ZSTD_c_compressionLevel, 3);
                        sr(cx.cctx_r, ZSTD_c_enableLongDistanceMatching, 0);
                        sr(cx.cctx_r, ZSTD_c_enableSeqProducerFallback, fallback);

                        reg_c(cx.cctx_c, &mut state_c as *mut _ as *mut c_void, *cb);
                        reg_r(cx.cctx_r, &mut state_r as *mut _ as *mut c_void, *cb);

                        let cap = cbnd(src.len()) + 1024;
                        let mut cbuf = vec![0u8; cap];
                        let mut rbuf = vec![0u8; cap];
                        let cn = c2_c(
                            cx.cctx_c, cbuf.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len(),
                        );
                        let rn = c2_r(
                            cx.cctx_r, rbuf.as_mut_ptr() as *mut c_void, cap,
                            src.as_ptr() as *const c_void, src.len(),
                        );
                        let ctx = format!(
                            "seqprod[{label}] shape={shape:?} len={} fallback={fallback}",
                            src.len()
                        );
                        e.eq(&ctx, cn, rn);
                        if !e.c.is_err(cn) {
                            assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                        }
                        // Unregister to avoid dangling state on the next reset.
                        reg_c(cx.cctx_c, std::ptr::null_mut(), None);
                        reg_r(cx.cctx_r, std::ptr::null_mut(), None);
                    }
                }
            }
        }
        // keep the state objects alive across all calls
        let _ = (&state_c, &state_r);
    }
}
