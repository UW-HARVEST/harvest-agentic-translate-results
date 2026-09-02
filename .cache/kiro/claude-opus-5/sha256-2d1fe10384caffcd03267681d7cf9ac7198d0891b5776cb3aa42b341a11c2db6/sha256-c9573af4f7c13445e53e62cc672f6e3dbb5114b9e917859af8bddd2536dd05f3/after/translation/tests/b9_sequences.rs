//! Phase B: differential tests for the EXTERNAL-SEQUENCE API — VALID paths.
//!
//! Covers ZSTD_sequenceBound, ZSTD_generateSequences, ZSTD_mergeBlockDelimiters,
//! ZSTD_compressSequences and ZSTD_compressSequencesAndLiterals over the full
//! cross-product of the relevant experimental cParams, asserting byte-identical
//! compressed output between the C and Rust libraries plus cross-decompression.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------- FFI typedefs

type FnSeqBound = unsafe extern "C" fn(size_t) -> size_t;
type FnGenSeq =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_Sequence, size_t, *const c_void, size_t) -> size_t;
type FnMergeDelims = unsafe extern "C" fn(*mut ZSTD_Sequence, size_t) -> size_t;
type FnCompressSeq = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const ZSTD_Sequence,
    size_t,
    *const c_void,
    size_t,
) -> size_t;
type FnCompressSeqAndLits = unsafe extern "C" fn(
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
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;

// ---------------------------------------------------------------- CCtx wrapper

/// A pair of compression contexts — one per library. Pointers are NEVER crossed
/// between libraries.
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

/// Compare two `ZSTD_Sequence` arrays element-by-element, printing the first
/// differing index.
#[track_caller]
fn assert_seqs_eq(ctx: &str, a: &[ZSTD_Sequence], b: &[ZSTD_Sequence]) {
    assert_eq!(a.len(), b.len(), "{ctx}: sequence count differs C={} RS={}", a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert_eq!(
            x, y,
            "{ctx}: first differing sequence at index {i}: C={x:?} RS={y:?}"
        );
    }
}

/// Compression levels swept everywhere.
const LEVELS: &[c_int] = &[1, 3, 9, 19];

// ================================================================ sequenceBound

/// ZSTD_sequenceBound over LENS + boundary + 500 random usize values.
#[test]
fn seq_bound_all() {
    unsafe {
        let (cb, rb) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let mut cases: Vec<usize> = LENS.to_vec();
        cases.extend([0usize, 1, usize::MAX, usize::MAX - 1, 1 << 30, 1 << 40]);
        let mut rng = Rng::new(0xB9_5EED_0001);
        for _ in 0..500 {
            cases.push(rng.next_u64() as usize);
        }
        for &n in &cases {
            assert_eq!(cb(n), rb(n), "ZSTD_sequenceBound({n})");
        }
    }
}

// ============================================================= generateSequences

/// Generate sequences on BOTH libraries (each with its own CCtx) and assert the
/// produced arrays are byte-identical and the returned counts match.
fn gen_seqs_on(
    cx: &Ctx,
    gen_c: &FnGenSeq,
    gen_r: &FnGenSeq,
    set_c: &FnSetParam,
    set_r: &FnSetParam,
    rst_c: &FnReset,
    rst_r: &FnReset,
    seq_bound: &FnSeqBound,
    e: &Err2,
    src: &[u8],
    level: c_int,
    ctx: &str,
) -> Option<Vec<ZSTD_Sequence>> {
    unsafe {
        rst_c(cx.cctx_c, ZSTD_reset_session_and_parameters);
        rst_r(cx.cctx_r, ZSTD_reset_session_and_parameters);
        e.eq(
            &format!("{ctx}: set level C"),
            set_c(cx.cctx_c, ZSTD_c_compressionLevel, level),
            set_r(cx.cctx_r, ZSTD_c_compressionLevel, level),
        );
        let bound = seq_bound(src.len());
        // over-allocate a touch to be safe against any off-by-one in bound.
        let cap = bound + 1;
        let mut cbuf = vec![ZSTD_Sequence::default(); cap];
        let mut rbuf = vec![ZSTD_Sequence::default(); cap];
        let cn = gen_c(
            cx.cctx_c,
            cbuf.as_mut_ptr(),
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        let rn = gen_r(
            cx.cctx_r,
            rbuf.as_mut_ptr(),
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        );
        e.eq(&format!("{ctx}: generateSequences return"), cn, rn);
        if e.c.is_err(cn) {
            return None;
        }
        assert_seqs_eq(&format!("{ctx}: generateSequences array"), &cbuf[..cn], &rbuf[..rn]);
        Some(cbuf[..cn].to_vec())
    }
}

#[test]
fn generate_sequences_all_shapes() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let cx = new_cctx();
        let mut rng = Rng::new(0xB9_5EED_0002);
        let lens: &[usize] = &[0, 1, 100, 1024, 20000, 131100, 200000];
        for &shape in ALL_SHAPES {
            for &len in lens {
                let src = gen(shape, len, &mut rng);
                for &lvl in LEVELS {
                    let ctx = format!("generateSequences shape={shape:?} len={} lvl={lvl}", src.len());
                    let _ = gen_seqs_on(
                        &cx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, lvl, &ctx,
                    );
                }
            }
        }
    }
}

// ========================================================== mergeBlockDelimiters

#[test]
fn merge_block_delimiters() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let (mc, mr) = both::<FnMergeDelims>("ZSTD_mergeBlockDelimiters");
        let cx = new_cctx();
        let mut rng = Rng::new(0xB9_5EED_0003);

        // Part 1: merge the arrays produced by generateSequences.
        let lens: &[usize] = &[0, 1, 100, 1024, 20000, 131100];
        for &shape in ALL_SHAPES {
            for &len in lens {
                let src = gen(shape, len, &mut rng);
                for &lvl in LEVELS {
                    let ctx = format!("merge(gen) shape={shape:?} len={} lvl={lvl}", src.len());
                    let seqs = match gen_seqs_on(
                        &cx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, lvl, &ctx,
                    ) {
                        Some(s) => s,
                        None => continue,
                    };
                    let mut ca = seqs.clone();
                    let mut ra = seqs.clone();
                    let cn = mc(ca.as_mut_ptr(), ca.len());
                    let rn = mr(ra.as_mut_ptr(), ra.len());
                    e.eq(&format!("{ctx}: merge return"), cn, rn);
                    if e.c.is_err(cn) {
                        continue;
                    }
                    assert_seqs_eq(&format!("{ctx}: merge array"), &ca[..cn], &ra[..rn]);
                }
            }
        }

        // Part 2: hand-built arrays with explicit block delimiters.
        for i in 0..400 {
            let nseq = 1 + rng.below(40);
            let mut seqs: Vec<ZSTD_Sequence> = Vec::with_capacity(nseq);
            for _ in 0..nseq {
                if rng.below(4) == 0 {
                    // explicit delimiter
                    seqs.push(ZSTD_Sequence::default());
                } else {
                    seqs.push(ZSTD_Sequence {
                        offset: 1 + rng.below(1000) as c_uint,
                        litLength: rng.below(50) as c_uint,
                        matchLength: 3 + rng.below(50) as c_uint,
                        rep: rng.below(4) as c_uint,
                    });
                }
            }
            // ensure at least one delimiter at the end (well-formed shape)
            seqs.push(ZSTD_Sequence::default());
            let mut ca = seqs.clone();
            let mut ra = seqs.clone();
            let cn = mc(ca.as_mut_ptr(), ca.len());
            let rn = mr(ra.as_mut_ptr(), ra.len());
            let ctx = format!("merge(handbuilt #{i})");
            e.eq(&format!("{ctx}: return"), cn, rn);
            if e.c.is_err(cn) {
                continue;
            }
            assert_seqs_eq(&format!("{ctx}: array"), &ca[..cn], &ra[..rn]);
        }
    }
}

// ============================================================ compressSequences

/// Drive compressSequences with a given param combination and assert
/// byte-identical output + cross-decompression.
#[allow(clippy::too_many_arguments)]
unsafe fn drive_compress_seqs(
    cx: &Ctx,
    csq_c: &FnCompressSeq,
    csq_r: &FnCompressSeq,
    set_c: &FnSetParam,
    set_r: &FnSetParam,
    rst_c: &FnReset,
    rst_r: &FnReset,
    dec_c: &FnDecompress,
    dec_r: &FnDecompress,
    cbound: &FnCompressBound,
    e: &Err2,
    seqs: &[ZSTD_Sequence],
    src: &[u8],
    bd: c_int,
    validate: c_int,
    repres: c_int,
    level: c_int,
    ctx: &str,
) {
    rst_c(cx.cctx_c, ZSTD_reset_session_and_parameters);
    rst_r(cx.cctx_r, ZSTD_reset_session_and_parameters);
    for (cctx, is_c) in [(cx.cctx_c, true), (cx.cctx_r, false)] {
        let set = if is_c { set_c } else { set_r };
        set(cctx, ZSTD_c_compressionLevel, level);
        set(cctx, ZSTD_c_blockDelimiters, bd);
        set(cctx, ZSTD_c_validateSequences, validate);
        set(cctx, ZSTD_c_repcodeResolution, repres);
    }
    let cap = cbound(src.len()) + 64;
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
    if e.c.is_err(cn) {
        return;
    }
    assert_bytes_eq(ctx, &cbuf[..cn], &rbuf[..rn]);

    // cross-decompress: C decodes RS output and vice versa.
    let mut d1 = vec![0u8; src.len() + 16];
    let mut d2 = vec![0u8; src.len() + 16];
    let a = dec_c(
        d1.as_mut_ptr() as *mut c_void,
        d1.len(),
        rbuf.as_ptr() as *const c_void,
        rn,
    );
    let b = dec_r(
        d2.as_mut_ptr() as *mut c_void,
        d2.len(),
        cbuf.as_ptr() as *const c_void,
        cn,
    );
    e.eq(&format!("{ctx}/cross-decompress"), a, b);
    if e.c.is_err(a) {
        return;
    }
    assert_eq!(a, src.len(), "{ctx}: roundtrip size (C decode of RS)");
    assert_bytes_eq(&format!("{ctx}/decoded C-of-RS"), &d1[..a], src);
    assert_bytes_eq(&format!("{ctx}/decoded RS-of-C"), &d2[..b], src);
}

/// Full cross-product of blockDelimiters/validateSequences/repcodeResolution/
/// level over generated sequence arrays for all shapes and lengths.
#[test]
fn compress_sequences_cross_product() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (mc, mr) = both::<FnMergeDelims>("ZSTD_mergeBlockDelimiters");
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let (dc, dr) = both::<FnDecompress>("ZSTD_decompress");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let gcx = new_cctx(); // separate context for sequence generation
        let mut rng = Rng::new(0xB9_5EED_0004);

        let lens: &[usize] = &[1, 100, 1024, 20000, 131100];
        for &shape in ALL_SHAPES {
            for &len in lens {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                // Build both an explicit-delimiter array and a merged (no-delim) array.
                let ctxg = format!("csq-gen shape={shape:?} len={}", src.len());
                let explicit = match gen_seqs_on(
                    &gcx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, 3, &ctxg,
                ) {
                    Some(s) => s,
                    None => continue,
                };
                let _ = &mr; // merge on both is verified in merge_block_delimiters
                let mut merged = explicit.clone();
                let mn = mc(merged.as_mut_ptr(), merged.len());
                if e.c.is_err(mn) {
                    continue;
                }
                merged.truncate(mn);

                for &lvl in LEVELS {
                    for &repres in &[0i32, 1, 2] {
                        // bd == 1 (explicit) uses the delimiter-containing array.
                        drive_compress_seqs(
                            &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &dc, &dr, &cbnd, &e,
                            &explicit, &src, 1, 0, repres, lvl,
                            &format!("csq shape={shape:?} len={} bd=1 val=0 rep={repres} lvl={lvl}", src.len()),
                        );
                        drive_compress_seqs(
                            &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &dc, &dr, &cbnd, &e,
                            &explicit, &src, 1, 1, repres, lvl,
                            &format!("csq shape={shape:?} len={} bd=1 val=1 rep={repres} lvl={lvl}", src.len()),
                        );
                        // bd == 0 (no delimiters) uses the merged array.
                        drive_compress_seqs(
                            &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &dc, &dr, &cbnd, &e,
                            &merged, &src, 0, 0, repres, lvl,
                            &format!("csq shape={shape:?} len={} bd=0 val=0 rep={repres} lvl={lvl}", src.len()),
                        );
                        drive_compress_seqs(
                            &cx, &csq_c, &csq_r, &sc, &sr, &rc, &rr, &dc, &dr, &cbnd, &e,
                            &merged, &src, 0, 1, repres, lvl,
                            &format!("csq shape={shape:?} len={} bd=0 val=1 rep={repres} lvl={lvl}", src.len()),
                        );
                    }
                }
            }
        }
    }
}

/// Sweep dst capacity from 0 up to compressBound+1 for compressSequences.
#[test]
fn compress_sequences_dst_capacity_sweep() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (csq_c, csq_r) = both::<FnCompressSeq>("ZSTD_compressSequences");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let gcx = new_cctx();
        let mut rng = Rng::new(0xB9_5EED_0005);

        for &shape in ALL_SHAPES {
            for &len in &[1usize, 100, 1024, 20000] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                let ctxg = format!("csq-cap-gen shape={shape:?} len={}", src.len());
                let explicit = match gen_seqs_on(
                    &gcx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, 3, &ctxg,
                ) {
                    Some(s) => s,
                    None => continue,
                };
                // Find the size zstd needs. NOTE: ZSTD_compressSequences uses
                // the destination buffer as scratch and requires roughly
                // ZSTD_compressBound(srcSize) of space, NOT merely the final
                // compressed size. Passing 0 < dstCapacity < that requirement is
                // documented undefined behavior: the C reference underflows an
                // internal (dstCapacity - headerSize) computation and runs away
                // writing far past the buffer (confirmed to segfault even with a
                // 128 KiB slack buffer for a 100-byte input). We therefore sweep
                // dstCapacity only at/above compressBound(srcSize) up through
                // compressBound+1; caps below that (including 0) fall inside the
                // UB window — the Rust translation returns dstSize_tooSmall
                // cleanly there, but the C ground truth corrupts memory, so a
                // differential comparison is impossible. Every buffer is sized
                // exactly to its reported capacity to honor the real API contract.
                rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                let bound = cbnd(src.len());
                let mut tmp = vec![0u8; bound + 64];
                let need = csq_c(
                    cx.cctx_c,
                    tmp.as_mut_ptr() as *mut c_void,
                    tmp.len(),
                    explicit.as_ptr(),
                    explicit.len(),
                    src.as_ptr() as *const c_void,
                    src.len(),
                );
                if e.c.is_err(need) {
                    continue;
                }
                // Safe capacities: at/above the scratch requirement
                // (~compressBound). Any 0 <= dstCapacity < that requirement is
                // documented UB in the C reference (integer-underflow runaway
                // write, confirmed to segfault), so it cannot be differentially
                // tested; the Rust side returns dstSize_tooSmall there instead.
                let caps = [bound, bound + 1, bound + 64];
                for &cap in &caps {
                    rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                    rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                    sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                    sr(cx.cctx_r, ZSTD_c_blockDelimiters, 1);
                    // Physical buffer is exactly `cap` bytes: the true API contract.
                    let mut cbuf = vec![0u8; cap.max(1)];
                    let mut rbuf = vec![0u8; cap.max(1)];
                    let cn = csq_c(
                        cx.cctx_c,
                        cbuf.as_mut_ptr() as *mut c_void,
                        cap,
                        explicit.as_ptr(),
                        explicit.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    let rn = csq_r(
                        cx.cctx_r,
                        rbuf.as_mut_ptr() as *mut c_void,
                        cap,
                        explicit.as_ptr(),
                        explicit.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                    );
                    let ctx = format!("csq-cap shape={shape:?} len={} cap={cap}", src.len());
                    e.eq(&ctx, cn, rn);
                    if !e.c.is_err(cn) {
                        assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }
    }
}

// =================================================== compressSequencesAndLiterals

/// Build a literals buffer from the sum of litLengths in `seqs`, taking the
/// literals from `src` in order. Returns (literals, decompressedSize).
fn extract_literals(seqs: &[ZSTD_Sequence], src: &[u8]) -> (Vec<u8>, usize) {
    let mut lits: Vec<u8> = Vec::new();
    let mut decompressed: usize = 0;
    let mut pos = 0usize;
    for s in seqs {
        let ll = s.litLength as usize;
        let ml = s.matchLength as usize;
        // take ll literals starting at current match/lit boundary in src
        let end = (pos + ll).min(src.len());
        if pos < src.len() {
            lits.extend_from_slice(&src[pos..end]);
        }
        // pad if src ran short
        for _ in end..(pos + ll) {
            lits.push(0);
        }
        pos += ll + ml;
        decompressed += ll + ml;
    }
    (lits, decompressed)
}

#[test]
fn compress_sequences_and_literals_cross_product() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (csl_c, csl_r) = both::<FnCompressSeqAndLits>("ZSTD_compressSequencesAndLiterals");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let gcx = new_cctx();
        let mut rng = Rng::new(0xB9_5EED_0006);

        let lens: &[usize] = &[1, 100, 1024, 20000, 131100];
        for &shape in ALL_SHAPES {
            for &len in lens {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                let ctxg = format!("csl-gen shape={shape:?} len={}", src.len());
                // generateSequences yields explicit-delimiter sequences, which is
                // exactly what compressSequencesAndLiterals requires.
                let seqs = match gen_seqs_on(
                    &gcx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, 3, &ctxg,
                ) {
                    Some(s) => s,
                    None => continue,
                };
                let (lits, decompressed) = extract_literals(&seqs, &src);
                let lit_cap = lits.len() + 8; // must be >= litSize + 8
                let mut litbuf = vec![0u8; lit_cap];
                litbuf[..lits.len()].copy_from_slice(&lits);

                for &lvl in LEVELS {
                    for &repres in &[0i32, 1, 2] {
                        rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                        rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                        for (cctx, is_c) in [(cx.cctx_c, true), (cx.cctx_r, false)] {
                            let set = if is_c { &sc } else { &sr };
                            set(cctx, ZSTD_c_compressionLevel, lvl);
                            // explicit delimiters only; checksum must be off (default off)
                            set(cctx, ZSTD_c_blockDelimiters, 1);
                            set(cctx, ZSTD_c_repcodeResolution, repres);
                        }
                        let cap = cbnd(decompressed) + 64;
                        let mut cbuf = vec![0u8; cap];
                        let mut rbuf = vec![0u8; cap];
                        let cn = csl_c(
                            cx.cctx_c,
                            cbuf.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            litbuf.as_ptr() as *const c_void,
                            lits.len(),
                            lit_cap,
                            decompressed,
                        );
                        let rn = csl_r(
                            cx.cctx_r,
                            rbuf.as_mut_ptr() as *mut c_void,
                            cap,
                            seqs.as_ptr(),
                            seqs.len(),
                            litbuf.as_ptr() as *const c_void,
                            lits.len(),
                            lit_cap,
                            decompressed,
                        );
                        let ctx = format!(
                            "csl shape={shape:?} len={} rep={repres} lvl={lvl}",
                            src.len()
                        );
                        e.eq(&ctx, cn, rn);
                        if !e.c.is_err(cn) {
                            assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                        }
                    }
                }
            }
        }
    }
}

/// Sweep dst capacity from 0 up to compressBound+1 for compressSequencesAndLiterals.
#[test]
fn compress_sequences_and_literals_dst_capacity_sweep() {
    unsafe {
        let e = Err2::new();
        let (gc, gr) = both::<FnGenSeq>("ZSTD_generateSequences");
        let (csl_c, csl_r) = both::<FnCompressSeqAndLits>("ZSTD_compressSequencesAndLiterals");
        let (sc, sr) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (rc, rr) = both::<FnReset>("ZSTD_CCtx_reset");
        let (sb, _) = both::<FnSeqBound>("ZSTD_sequenceBound");
        let (cbnd, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let cx = new_cctx();
        let gcx = new_cctx();
        let mut rng = Rng::new(0xB9_5EED_0007);

        for &shape in ALL_SHAPES {
            for &len in &[1usize, 100, 1024, 20000] {
                let src = gen(shape, len, &mut rng);
                if src.is_empty() {
                    continue;
                }
                let ctxg = format!("csl-cap-gen shape={shape:?} len={}", src.len());
                let seqs = match gen_seqs_on(
                    &gcx, &gc, &gr, &sc, &sr, &rc, &rr, &sb, &e, &src, 3, &ctxg,
                ) {
                    Some(s) => s,
                    None => continue,
                };
                let (lits, decompressed) = extract_literals(&seqs, &src);
                let lit_cap = lits.len() + 8;
                let mut litbuf = vec![0u8; lit_cap];
                litbuf[..lits.len()].copy_from_slice(&lits);

                // Same UB caveat as compressSequences: 0 < dstCapacity below the
                // scratch requirement is undefined and the C reference runs away.
                // Sweep 0 and caps at/above compressBound.
                rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                let bound = cbnd(decompressed);
                // ZSTD_compressSequencesAndLiterals exhibits the same C-reference
                // undefined behavior for undersized dst as ZSTD_compressSequences,
                // including for dstCapacity == 0 (unlike compressSequences it does
                // not short-circuit at 0). Sweep only caps at/above compressBound.
                let caps = [bound, bound + 1, bound + 64];
                for &cap in &caps {
                    rc(cx.cctx_c, ZSTD_reset_session_and_parameters);
                    rr(cx.cctx_r, ZSTD_reset_session_and_parameters);
                    sc(cx.cctx_c, ZSTD_c_blockDelimiters, 1);
                    sr(cx.cctx_r, ZSTD_c_blockDelimiters, 1);
                    let mut cbuf = vec![0u8; cap.max(1)];
                    let mut rbuf = vec![0u8; cap.max(1)];
                    let cn = csl_c(
                        cx.cctx_c,
                        cbuf.as_mut_ptr() as *mut c_void,
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        litbuf.as_ptr() as *const c_void,
                        lits.len(),
                        lit_cap,
                        decompressed,
                    );
                    let rn = csl_r(
                        cx.cctx_r,
                        rbuf.as_mut_ptr() as *mut c_void,
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        litbuf.as_ptr() as *const c_void,
                        lits.len(),
                        lit_cap,
                        decompressed,
                    );
                    let ctx = format!("csl-cap shape={shape:?} len={} cap={cap}", src.len());
                    e.eq(&ctx, cn, rn);
                    if !e.c.is_err(cn) {
                        assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }
    }
}
