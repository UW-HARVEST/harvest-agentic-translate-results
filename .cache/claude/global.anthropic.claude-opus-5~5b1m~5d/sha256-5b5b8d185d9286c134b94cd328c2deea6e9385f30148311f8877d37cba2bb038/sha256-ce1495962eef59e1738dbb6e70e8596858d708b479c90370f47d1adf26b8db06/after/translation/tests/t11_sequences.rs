//! Phase C: the SEQUENCE-level API.
//!
//! Covered symbols: `ZSTD_sequenceBound`, `ZSTD_generateSequences`,
//! `ZSTD_mergeBlockDelimiters`, `ZSTD_compressSequences`,
//! `ZSTD_compressSequencesAndLiterals`.
//!
//! For every configuration we require identical `ZSTD_Sequence` arrays
//! (element by element), identical counts, identical compressed frames, and
//! identical error codes — plus the frames must decode back to the original
//! input.
//!
//! ### Deliberate UB avoidance
//!
//! Two C behaviours are *undefined* rather than diagnosed, so this file stays
//! away from them (a differential test cannot compare memory corruption):
//!
//! * `ZSTD_compressSequences{,AndLiterals}` call `ZSTD_writeFrameHeader()` and
//!   only `assert()` its result, so a `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX`
//!   (18) makes the C advance `op` by a negated error code and write out of
//!   bounds. Every dst sweep therefore starts at 18.
//! * With `ZSTD_c_validateSequences == 0` the header documents invalid
//!   sequences as UB, and `matchLength < MINMATCH` genuinely indexes the ML
//!   FSE table out of bounds (`ZSTD_MLcode()` on an underflowed `mlBase`).
//!   Structural errors (block delimiters, block/frame size agreement) *are*
//!   checked unconditionally, so those are exercised with validation both off
//!   and on; the validation-only rejections are exercised with validation on,
//!   plus one *representable* case (`matchLength == 3` while `minMatch == 4`)
//!   that validation rejects and non-validation legitimately accepts.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod common;
use common::*;

use libloading::Symbol;
use std::ffi::c_void;

// ------------------------------------------------------------------ FFI types

type CCtx = *mut c_void;
type DCtx = *mut c_void;

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_decompressDCtx = unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_getErrorCode = unsafe extern "C" fn(usize) -> i32;
type Fn_loadDict = unsafe extern "C" fn(CCtx, *const u8, usize) -> usize;
type Fn_decompress_usingDict =
    unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;
type Fn_train = unsafe extern "C" fn(*mut u8, usize, *const u8, *const usize, u32) -> usize;
type Fn_dictID_fromFrame = unsafe extern "C" fn(*const u8, usize) -> u32;

type Fn_sequenceBound = unsafe extern "C" fn(usize) -> usize;
type Fn_generateSequences =
    unsafe extern "C" fn(CCtx, *mut ZSTD_Sequence, usize, *const u8, usize) -> usize;
type Fn_mergeBlockDelimiters = unsafe extern "C" fn(*mut ZSTD_Sequence, usize) -> usize;
type Fn_compressSequences = unsafe extern "C" fn(
    CCtx,
    *mut u8,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const u8,
    usize,
) -> usize;
type Fn_compressSequencesAndLiterals = unsafe extern "C" fn(
    CCtx,
    *mut u8,
    usize,
    *const ZSTD_Sequence,
    usize,
    *const u8,
    usize,
    usize,
    usize,
) -> usize;

// -------------------------------------------------------------------- helpers

fn is_err(v: usize) -> bool {
    v > usize::MAX - 200
}

/// Compares a numeric result and — when it is an error — the decoded
/// `ZSTD_ErrorCode`. Returns `true` when the (identical) result is an error.
struct ErrCmp {
    c: Symbol<'static, Fn_getErrorCode>,
    r: Symbol<'static, Fn_getErrorCode>,
}

impl ErrCmp {
    fn new() -> Self {
        let (c, r) = impls().pair::<Fn_getErrorCode>("ZSTD_getErrorCode");
        ErrCmp { c, r }
    }
    fn check(&self, tag: &str, a: usize, b: usize) -> bool {
        assert_eq_dbg(tag, a, b);
        if is_err(a) {
            unsafe {
                assert_eq_dbg(&format!("{tag} / ZSTD_getErrorCode"), (self.c)(a), (self.r)(b));
            }
            true
        } else {
            false
        }
    }
    fn code(&self, v: usize) -> i32 {
        unsafe { (self.c)(v) }
    }
}

fn assert_seqs_eq(tag: &str, c: &[ZSTD_Sequence], r: &[ZSTD_Sequence]) {
    if c.len() != r.len() {
        panic!("{tag}: sequence count mismatch C={} Rust={}", c.len(), r.len());
    }
    for (k, (a, b)) in c.iter().zip(r).enumerate() {
        assert!(
            a == b,
            "{tag}: sequence[{k}] differs\n  C   ={a:?}\n  Rust={b:?}"
        );
    }
}

/// Applies a parameter list to both cctxs, asserting identical returns.
/// Returns `false` when either setter rejected a value (identically).
fn apply_params(
    ec: &ErrCmp,
    set: (Fn_setParam, Fn_setParam),
    cc: CCtx,
    rc: CCtx,
    params: &[(i32, i32)],
) -> bool {
    let mut ok = true;
    for &(id, v) in params {
        let (a, b) = unsafe { (set.0(cc, id, v), set.1(rc, id, v)) };
        if ec.check(&format!("setParameter({id},{v})"), a, b) {
            ok = false;
        }
    }
    ok
}

/// A compressible, structured payload — dense in matches, so the generated
/// sequence arrays are long and interesting.
fn gen_logish(rng: &mut Rng, len: usize) -> Vec<u8> {
    const TOK: [&str; 8] = [
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    ];
    let mut v = Vec::with_capacity(len + 64);
    while v.len() < len {
        v.extend_from_slice(TOK[rng.below(TOK.len())].as_bytes());
        v.push(b'=');
        v.extend_from_slice(format!("{}", rng.below(1000)).as_bytes());
        v.push(b';');
        if rng.below(9) == 0 {
            v.push(b'\n');
        }
    }
    v.truncate(len);
    v
}

/// Result of a `ZSTD_generateSequences` comparison: the (identical) arrays.
struct Generated {
    seqs: Vec<ZSTD_Sequence>,
}

/// Runs `ZSTD_generateSequences` on BOTH libraries with the given parameters
/// and asserts full parity. Returns the produced array on success.
///
/// A *fresh* cctx is used for each call and freed immediately afterwards,
/// because the C never clears `cctx->seqCollector` — reusing such a context for
/// a normal compression would keep writing into `outSeqs`.
fn generate_both(
    ec: &ErrCmp,
    params: &[(i32, i32)],
    src: &[u8],
    capacity: usize,
    tag: &str,
) -> Option<Generated> {
    generate_both_dict(ec, params, &[], src, capacity, tag)
}

/// Same, with an optional dictionary loaded onto the generating context
/// (`ZSTD_CCtx_loadDictionary`); an empty `dict` means "no dictionary".
fn generate_both_dict(
    ec: &ErrCmp,
    params: &[(i32, i32)],
    dict: &[u8],
    src: &[u8],
    capacity: usize,
    tag: &str,
) -> Option<Generated> {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_gen, r_gen) = i.pair::<Fn_generateSequences>("ZSTD_generateSequences");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    assert!(!cc.is_null() && !rc.is_null());

    let dict_ok = if dict.is_empty() {
        true
    } else {
        let (a, b) = unsafe {
            (
                c_ld(cc, dict.as_ptr(), dict.len()),
                r_ld(rc, dict.as_ptr(), dict.len()),
            )
        };
        !ec.check(&format!("{tag} / loadDictionary"), a, b)
    };

    let out = if !dict_ok || !apply_params(ec, (*c_set, *r_set), cc, rc, params) {
        None
    } else {
        // zeroed buffers: `ZSTD_copyBlockSequences` never writes the `rep`
        // field of the block-delimiter entry, so the caller's initial bytes are
        // observable and must start out identical.
        let mut cs = vec![ZSTD_Sequence::default(); capacity.max(1)];
        let mut rs = vec![ZSTD_Sequence::default(); capacity.max(1)];
        let a = unsafe { c_gen(cc, cs.as_mut_ptr(), capacity, src.as_ptr(), src.len()) };
        let b = unsafe { r_gen(rc, rs.as_mut_ptr(), capacity, src.as_ptr(), src.len()) };
        if ec.check(&format!("{tag} / ZSTD_generateSequences"), a, b) {
            None
        } else {
            assert!(
                a <= capacity,
                "{tag}: generateSequences returned {a} > capacity {capacity}"
            );
            assert_seqs_eq(
                &format!("{tag} / generated sequences"),
                &cs[..a],
                &rs[..a],
            );
            Some(Generated {
                seqs: cs[..a].to_vec(),
            })
        }
    };

    unsafe {
        c_free(cc);
        r_free(rc);
    }
    out
}

/// Extracts the literals described by an explicit-delimiter sequence array,
/// exactly as `ZSTD_compressSequencesAndLiterals` expects them.
fn extract_literals(src: &[u8], seqs: &[ZSTD_Sequence]) -> Option<Vec<u8>> {
    let mut lits = Vec::new();
    let mut pos = 0usize;
    for s in seqs {
        let ll = s.lit_length as usize;
        let ml = s.match_length as usize;
        if pos + ll > src.len() {
            return None;
        }
        lits.extend_from_slice(&src[pos..pos + ll]);
        pos += ll;
        if pos + ml > src.len() {
            return None;
        }
        pos += ml;
    }
    if pos != src.len() {
        return None;
    }
    Some(lits)
}

// ============================================================ 1. sequenceBound

/// `ZSTD_sequenceBound` is pure arithmetic on `srcSize`; sweep the whole range
/// including the boundaries of both divisors and randomized giant values.
#[test]
fn sequence_bound_matches() {
    let i = impls();
    let (c, r) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");

    let mut sizes: Vec<usize> = EDGE_LENS.to_vec();
    for base in [0usize, 3, 1024, 131_072, 1 << 20, 1 << 30] {
        for d in 0..=6usize {
            sizes.push(base.saturating_add(d));
            sizes.push(base.saturating_sub(d));
        }
    }
    sizes.extend([
        1 << 24,
        (1 << 30) + 1,
        usize::MAX / 8,
        usize::MAX / 4,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ]);
    let mut rng = Rng::new(0x5E9B_0001);
    for _ in 0..500 {
        sizes.push(rng.next_u64() as usize);
        sizes.push(rng.below(4_000_000));
    }

    for s in sizes {
        unsafe {
            assert_eq_dbg(&format!("ZSTD_sequenceBound({s})"), c(s), r(s));
        }
    }
}

// ======================================================== 2. generateSequences

/// `ZSTD_generateSequences` over every input shape, a wide size sweep and a
/// level sweep. The produced `ZSTD_Sequence` arrays must be identical element
/// by element, and the returned count must match.
#[test]
fn generate_sequences_shapes_levels_match() {
    let i = impls();
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let ec = ErrCmp::new();

    let mut rng = Rng::new(0x6E_5E_0001);
    let lens = [
        0usize, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 100, 127, 128, 1_000, 1_024, 2_000, 5_000, 30_000,
        60_000, 131_071, 131_072, 131_073, 200_000, 300_000,
    ];

    for &shape in &ALL_SHAPES {
        for &len in &lens {
            let src = gen_shape(shape, len, &mut rng);
            let bound = unsafe { c_sb(len) };
            for &lvl in &[
                -131_072i32, -1000, -20, -5, -1, 0, 1, 2, 3, 6, 9, 12, 15, 17, 19, 22,
            ] {
                let tag = format!("generateSequences shape={shape:?} len={len} lvl={lvl}");
                generate_both(
                    &ec,
                    &[(ZSTD_c_compressionLevel, lvl)],
                    &src,
                    bound,
                    &tag,
                );
            }
        }
        // and the same on log-like (highly matchable) input
        for &len in &[7usize, 500, 4_000, 20_000, 131_072, 150_000, 400_000] {
            let src = gen_logish(&mut rng, len);
            let bound = unsafe { c_sb(len) };
            for &lvl in &[-5i32, 1, 3, 6, 9, 12, 15, 19, 22] {
                let tag = format!("generateSequences logish len={len} lvl={lvl}");
                generate_both(&ec, &[(ZSTD_c_compressionLevel, lvl)], &src, bound, &tag);
            }
        }
    }
}

/// Every strategy, plus the row match finder / LDM / minMatch / windowLog knobs
/// that change which match finder produces the sequences.
#[test]
fn generate_sequences_strategies_match() {
    let i = impls();
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let ec = ErrCmp::new();
    let mut rng = Rng::new(0x57_9A_0001);

    let mut rows: Vec<Vec<(i32, i32)>> = Vec::new();
    for &s in &ALL_STRATEGIES {
        rows.push(vec![(ZSTD_c_strategy, s), (ZSTD_c_compressionLevel, 6)]);
        rows.push(vec![(ZSTD_c_strategy, s), (ZSTD_c_compressionLevel, 1)]);
    }
    for &rmf in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &s in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
            rows.push(vec![
                (ZSTD_c_useRowMatchFinder, rmf),
                (ZSTD_c_strategy, s),
            ]);
        }
    }
    for &mm in &[3i32, 4, 5, 6, 7] {
        rows.push(vec![
            (ZSTD_c_minMatch, mm),
            (ZSTD_c_strategy, if mm == 7 { ZSTD_fast } else { ZSTD_btopt }),
        ]);
    }
    for &wl in &[10i32, 12, 17, 20] {
        rows.push(vec![(ZSTD_c_windowLog, wl)]);
    }
    for &ldm in &[0i32, 1] {
        rows.push(vec![
            (ZSTD_c_enableLongDistanceMatching, ldm),
            (ZSTD_c_windowLog, 20),
        ]);
    }
    for &lvl in &[0i32, 2, 5, 8, 11, 14, 17, 20, 22] {
        rows.push(vec![(ZSTD_c_compressionLevel, lvl)]);
    }
    for &bs in &[0i32, 1, 3, 6] {
        rows.push(vec![(ZSTD_c_blockSplitterLevel, bs)]);
    }
    for &sas in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        rows.push(vec![
            (ZSTD_c_splitAfterSequences, sas),
            (ZSTD_c_compressionLevel, 9),
        ]);
    }
    for &tl in &[0i32, 1, 32, 999, 131_072] {
        rows.push(vec![
            (ZSTD_c_targetLength, tl),
            (ZSTD_c_strategy, ZSTD_btultra2),
        ]);
    }
    for &sl in &[1i32, 3, 6, 9] {
        rows.push(vec![
            (ZSTD_c_searchLog, sl),
            (ZSTD_c_strategy, ZSTD_lazy2),
            (ZSTD_c_compressionLevel, 9),
        ]);
    }
    rows.push(vec![
        (ZSTD_c_strategy, ZSTD_fast),
        (ZSTD_c_windowLog, 17),
        (ZSTD_c_hashLog, 16),
        (ZSTD_c_searchLog, 1),
        (ZSTD_c_minMatch, 5),
        (ZSTD_c_targetLength, 0),
    ]);
    rows.push(vec![
        (ZSTD_c_strategy, ZSTD_btultra2),
        (ZSTD_c_windowLog, 18),
        (ZSTD_c_hashLog, 17),
        (ZSTD_c_chainLog, 17),
        (ZSTD_c_searchLog, 6),
        (ZSTD_c_minMatch, 3),
        (ZSTD_c_targetLength, 999),
    ]);
    rows.push(vec![
        (ZSTD_c_enableLongDistanceMatching, 1),
        (ZSTD_c_ldmHashLog, 17),
        (ZSTD_c_ldmMinMatch, 32),
        (ZSTD_c_ldmBucketSizeLog, 3),
        (ZSTD_c_ldmHashRateLog, 4),
        (ZSTD_c_windowLog, 21),
    ]);
    for &lcm in &[ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed] {
        rows.push(vec![(ZSTD_c_literalCompressionMode, lcm)]);
    }

    for row in &rows {
        for trial in 0..6 {
            let len = match trial {
                0 => 0,
                1 => rng.range(1, 40),
                2 => rng.range(40, 400),
                3 => rng.range(4_000, 40_000),
                4 => rng.range(120_000, 140_000),
                _ => rng.range(140_000, 260_000),
            };
            let src = if rng.bool() {
                gen_logish(&mut rng, len)
            } else {
                let s = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                gen_shape(s, len, &mut rng)
            };
            let bound = unsafe { c_sb(len) };
            let tag = format!("generateSequences row={row:?} len={len}");
            generate_both(&ec, row, &src, bound, &tag);
        }
    }
}

// ==================================================== 3. mergeBlockDelimiters

/// `ZSTD_mergeBlockDelimiters` is a pure in-place array transform: sweep it on
/// real generated arrays *and* on fully randomized arrays (including arrays
/// made only of delimiters and arrays with none at all).
#[test]
fn merge_block_delimiters_matches() {
    let i = impls();
    let (c_mg, r_mg) = i.pair::<Fn_mergeBlockDelimiters>("ZSTD_mergeBlockDelimiters");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let ec = ErrCmp::new();
    let mut rng = Rng::new(0x_4E_46_0001);

    // ---- real arrays produced by ZSTD_generateSequences
    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 300, 9_000, 140_000] {
            let src = gen_shape(shape, len, &mut rng);
            let bound = unsafe { c_sb(len) };
            for &lvl in &[1i32, 5, 19] {
                let tag = format!("merge(real) shape={shape:?} len={len} lvl={lvl}");
                if let Some(g) =
                    generate_both(&ec, &[(ZSTD_c_compressionLevel, lvl)], &src, bound, &tag)
                {
                    let mut cs = g.seqs.clone();
                    let mut rs = g.seqs.clone();
                    let a = unsafe { c_mg(cs.as_mut_ptr(), cs.len()) };
                    let b = unsafe { r_mg(rs.as_mut_ptr(), rs.len()) };
                    assert_eq_dbg(&format!("{tag} / merged count"), a, b);
                    // the whole buffer must be identical, not just the prefix
                    assert_seqs_eq(&format!("{tag} / merged array"), &cs, &rs);
                }
            }
        }
    }

    // ---- randomized arrays, incl. all-delimiter and delimiter-free shapes
    for case in 0..1500 {
        let n = rng.range(0, 40);
        let mut base: Vec<ZSTD_Sequence> = Vec::with_capacity(n);
        for _ in 0..n {
            let kind = rng.below(4);
            let s = match kind {
                0 => ZSTD_Sequence {
                    // block delimiter / last literals
                    offset: 0,
                    lit_length: rng.below(500) as u32,
                    match_length: 0,
                    rep: rng.below(4) as u32,
                },
                1 => ZSTD_Sequence {
                    // pure marker
                    offset: 0,
                    lit_length: 0,
                    match_length: 0,
                    rep: 0,
                },
                2 => ZSTD_Sequence {
                    // "offset == 0 but matchLength != 0" — merge only checks both
                    offset: 0,
                    lit_length: rng.below(50) as u32,
                    match_length: rng.range(1, 100) as u32,
                    rep: rng.below(4) as u32,
                },
                _ => ZSTD_Sequence {
                    offset: rng.range(1, 100_000) as u32,
                    lit_length: rng.below(200) as u32,
                    match_length: rng.range(3, 300) as u32,
                    rep: rng.below(4) as u32,
                },
            };
            base.push(s);
        }
        // sweep every prefix length so the `in == seqsSize - 1` branch is hit
        for take in 0..=base.len() {
            let mut cs = base[..take].to_vec();
            let mut rs = base[..take].to_vec();
            let a = unsafe { c_mg(cs.as_mut_ptr(), take) };
            let b = unsafe { r_mg(rs.as_mut_ptr(), take) };
            let tag = format!("mergeBlockDelimiters(case={case}, n={take})");
            assert_eq_dbg(&tag, a, b);
            assert_seqs_eq(&format!("{tag} / array"), &cs, &rs);
        }
    }
}

// ====================================================== 4. compressSequences

/// The core round trip: generate -> (optionally merge) -> compressSequences ->
/// decompress. Both `ZSTD_c_blockDelimiters` modes and both
/// `ZSTD_c_validateSequences` settings, crossed with `ZSTD_c_repcodeResolution`
/// and frame flags. Frames must be byte identical and decode to the input.
#[test]
fn compress_sequences_roundtrip_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs, r_cs) = i.pair::<Fn_compressSequences>("ZSTD_compressSequences");
    let (c_mg, r_mg) = i.pair::<Fn_mergeBlockDelimiters>("ZSTD_mergeBlockDelimiters");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dec, r_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0xC5_E9_0001);

    // (name, extra cctx params applied on top of the level)
    let mut rows: Vec<(&'static str, Vec<(i32, i32)>)> = Vec::new();
    for &rr in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        rows.push(("repcodeResolution", vec![(ZSTD_c_repcodeResolution, rr)]));
    }
    for &(cs, ck) in &[(1i32, 0i32), (0, 1), (1, 1), (0, 0)] {
        rows.push((
            "frame-flags",
            vec![(ZSTD_c_contentSizeFlag, cs), (ZSTD_c_checksumFlag, ck)],
        ));
    }
    for &mb in &[0i32, 1024, 8192, 65_536, 131_072] {
        rows.push(("maxBlockSize", vec![(ZSTD_c_maxBlockSize, mb)]));
    }
    rows.push(("plain", vec![]));
    for &s in &ALL_STRATEGIES {
        rows.push(("strategy", vec![(ZSTD_c_strategy, s)]));
    }
    for &mm in &[3i32, 4, 5, 6] {
        rows.push(("minMatch", vec![(ZSTD_c_minMatch, mm)]));
    }
    for &wl in &[15i32, 18, 21] {
        rows.push(("windowLog", vec![(ZSTD_c_windowLog, wl)]));
    }
    for &lcm in &[ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed] {
        rows.push(("literalCompressionMode", vec![(ZSTD_c_literalCompressionMode, lcm)]));
    }
    for &f in &[ZSTD_f_zstd1] {
        rows.push(("format", vec![(ZSTD_c_format, f)]));
    }

    for &shape_logish in &[true, false] {
        for &len in &[0usize, 1, 9, 300, 4_000, 40_000, 131_072, 200_000, 400_000] {
            let src = if shape_logish {
                gen_logish(&mut rng, len)
            } else {
                gen_shape(Shape::Tabular, len, &mut rng)
            };
            let bound = unsafe { c_sb(len) };
            for &lvl in &[-3i32, 1, 5, 9, 11, 15, 19, 22] {
                let gtag = format!("len={len} lvl={lvl} logish={shape_logish}");
                let gen = match generate_both(
                    &ec,
                    &[(ZSTD_c_compressionLevel, lvl)],
                    &src,
                    bound,
                    &gtag,
                ) {
                    Some(g) => g,
                    None => continue,
                };

                // merged variant, verified for parity in its own right
                let merged = {
                    let mut cs2 = gen.seqs.clone();
                    let mut rs2 = gen.seqs.clone();
                    let a = unsafe { c_mg(cs2.as_mut_ptr(), cs2.len()) };
                    let b = unsafe { r_mg(rs2.as_mut_ptr(), rs2.len()) };
                    assert_eq_dbg(&format!("{gtag} / merge count"), a, b);
                    assert_seqs_eq(&format!("{gtag} / merge array"), &cs2, &rs2);
                    cs2[..a].to_vec()
                };

                for row in &rows {
                    for &delim in &[ZSTD_sf_explicitBlockDelimiters, ZSTD_sf_noBlockDelimiters] {
                        let seqs: &[ZSTD_Sequence] = if delim == ZSTD_sf_explicitBlockDelimiters {
                            &gen.seqs
                        } else {
                            &merged
                        };
                        for &validate in &[0i32, 1] {
                            unsafe {
                                c_rst(cc, ZSTD_reset_session_and_parameters);
                                r_rst(rc, ZSTD_reset_session_and_parameters);
                            }
                            let mut params: Vec<(i32, i32)> = vec![
                                (ZSTD_c_compressionLevel, lvl),
                                (ZSTD_c_blockDelimiters, delim),
                                (ZSTD_c_validateSequences, validate),
                            ];
                            params.extend(row.1.iter().copied());
                            if !apply_params(&ec, (*c_set, *r_set), cc, rc, &params) {
                                continue;
                            }

                            let cap = unsafe { c_bound(len) } + 128;
                            let mut cb = vec![0xA5u8; cap];
                            let mut rb = vec![0x5Au8; cap];
                            let a = unsafe {
                                c_cs(
                                    cc,
                                    cb.as_mut_ptr(),
                                    cap,
                                    seqs.as_ptr(),
                                    seqs.len(),
                                    src.as_ptr(),
                                    len,
                                )
                            };
                            let b = unsafe {
                                r_cs(
                                    rc,
                                    rb.as_mut_ptr(),
                                    cap,
                                    seqs.as_ptr(),
                                    seqs.len(),
                                    src.as_ptr(),
                                    len,
                                )
                            };
                            let tag = format!(
                                "compressSequences [{}] {gtag} delim={delim} validate={validate} params={:?}",
                                row.0, row.1
                            );
                            if ec.check(&tag, a, b) {
                                continue;
                            }
                            assert_bytes_eq(&tag, &cb[..a], &rb[..b]);

                            // must decode back to the original input, both ways
                            let mut o1 = vec![0u8; len + 16];
                            let mut o2 = vec![0u8; len + 16];
                            let n1 = unsafe {
                                r_dec(rd, o1.as_mut_ptr(), o1.len(), cb.as_ptr(), a)
                            };
                            let n2 = unsafe {
                                c_dec(cd, o2.as_mut_ptr(), o2.len(), rb.as_ptr(), b)
                            };
                            assert_eq_dbg(&format!("{tag} / rust decodes C frame"), n1, len);
                            assert_eq_dbg(&format!("{tag} / C decodes rust frame"), n2, len);
                            assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                            assert_bytes_eq(&format!("{tag} / payload"), &src, &o2[..n2]);
                        }
                    }
                }
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

// ================================================= 5. compressSequences dst sweep

/// Undersized destination buffers. The sweep starts at
/// `ZSTD_FRAMEHEADERSIZE_MAX` (18) because a smaller capacity makes the C write
/// out of bounds (see the module comment).
#[test]
fn compress_sequences_dst_too_small_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs, r_cs) = i.pair::<Fn_compressSequences>("ZSTD_compressSequences");
    let (c_mg, _) = i.pair::<Fn_mergeBlockDelimiters>("ZSTD_mergeBlockDelimiters");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    const FRAMEHEADERSIZE_MAX: usize = 18;
    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let mut rng = Rng::new(0xD57_0001);

    for &len in &[0usize, 5, 700, 6_000, 150_000] {
        let src = gen_logish(&mut rng, len);
        let bound = unsafe { c_sb(len) };
        let gtag = format!("dst-sweep len={len}");
        let gen = match generate_both(&ec, &[(ZSTD_c_compressionLevel, 3)], &src, bound, &gtag) {
            Some(g) => g,
            None => continue,
        };
        let merged = {
            let mut v = gen.seqs.clone();
            let n = unsafe { c_mg(v.as_mut_ptr(), v.len()) };
            v[..n].to_vec()
        };

        for &delim in &[ZSTD_sf_explicitBlockDelimiters, ZSTD_sf_noBlockDelimiters] {
            let seqs: &[ZSTD_Sequence] = if delim == ZSTD_sf_explicitBlockDelimiters {
                &gen.seqs
            } else {
                &merged
            };
            // full size first
            let full = {
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    c_set(cc, ZSTD_c_compressionLevel, 3);
                    c_set(cc, ZSTD_c_blockDelimiters, delim);
                }
                let cap = unsafe { c_bound(len) } + 128;
                let mut b = vec![0u8; cap];
                unsafe {
                    c_cs(
                        cc,
                        b.as_mut_ptr(),
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        src.as_ptr(),
                        len,
                    )
                }
            };
            if is_err(full) {
                continue;
            }

            // a dense sweep near the frame-header floor plus a coarse sweep up
            // to (and one past) the exact frame size
            let mut caps: Vec<usize> = (FRAMEHEADERSIZE_MAX..=(FRAMEHEADERSIZE_MAX + 40).min(full))
                .collect();
            let step = (full / 40).max(1);
            let mut c = FRAMEHEADERSIZE_MAX;
            while c <= full {
                caps.push(c);
                c += step;
            }
            caps.push(full);
            caps.push(full + 1);
            if full > 0 {
                caps.push(full - 1);
            }
            caps.retain(|&c| c >= FRAMEHEADERSIZE_MAX);
            caps.sort_unstable();
            caps.dedup();

            for cap in caps {
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                }
                if !apply_params(
                    &ec,
                    (*c_set, *r_set),
                    cc,
                    rc,
                    &[
                        (ZSTD_c_compressionLevel, 3),
                        (ZSTD_c_blockDelimiters, delim),
                    ],
                ) {
                    continue;
                }
                let mut cb = vec![0u8; cap + 8];
                let mut rb = vec![0u8; cap + 8];
                let a = unsafe {
                    c_cs(
                        cc,
                        cb.as_mut_ptr(),
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        src.as_ptr(),
                        len,
                    )
                };
                let b = unsafe {
                    r_cs(
                        rc,
                        rb.as_mut_ptr(),
                        cap,
                        seqs.as_ptr(),
                        seqs.len(),
                        src.as_ptr(),
                        len,
                    )
                };
                let tag = format!("{gtag} delim={delim} dstCapacity={cap} (full={full})");
                if !ec.check(&tag, a, b) {
                    assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

// ====================================================== 6. invalid sequences

/// Invalid sequence arrays must be rejected with the *same* error code.
///
/// Structural violations (missing / malformed block delimiter, block size vs
/// srcSize disagreement, `nbSequences == 0`) are validated unconditionally by
/// the C, so they are checked with `validateSequences` both off and on.
/// Validation-only violations (offset beyond the window, matchLength below the
/// minMatch floor) are checked with validation on.
#[test]
fn compress_sequences_invalid_sequences_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs, r_cs) = i.pair::<Fn_compressSequences>("ZSTD_compressSequences");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let mut rng = Rng::new(0x1_BAD_5E9);

    // --- a valid baseline (explicit delimiters) for several sizes
    for &len in &[600usize, 5_000, 40_000] {
        let src = gen_logish(&mut rng, len);
        let bound = unsafe { c_sb(len) };
        let gtag = format!("invalid-base len={len}");
        let gen = match generate_both(&ec, &[(ZSTD_c_compressionLevel, 3)], &src, bound, &gtag) {
            Some(g) => g,
            None => continue,
        };
        let base = gen.seqs;
        assert!(base.len() >= 2, "{gtag}: need a non-trivial sequence array");

        // index of the first real (non-delimiter) sequence
        let real = base
            .iter()
            .position(|s| s.offset != 0 || s.match_length != 0)
            .expect("at least one real sequence");

        // (name, mutated seqs, nbSeqs, srcSize, validate settings to try)
        let mut cases: Vec<(String, Vec<ZSTD_Sequence>, usize, usize, Vec<i32>)> = Vec::new();

        // a. offset == 0 on a real sequence -> malformed delimiter
        {
            let mut v = base.clone();
            v[real].offset = 0;
            let n = v.len();
            cases.push(("offset0-on-real".into(), v, n, len, vec![0, 1]));
        }
        // b. offset beyond the window / history (validation only)
        {
            let mut v = base.clone();
            v[real].offset = 1 << 30;
            let n = v.len();
            cases.push(("offset-too-large".into(), v, n, len, vec![1]));
        }
        // c. matchLength == 3 while minMatch == 4: representable in the format,
        //    so validate=0 accepts it and validate=1 must reject it.
        {
            let mut v = base.clone();
            let delta = v[real].match_length as i64 - 3;
            if delta > 0 {
                v[real].match_length = 3;
                // keep the block size intact by moving the difference into the
                // literals of the same sequence
                v[real].lit_length += delta as u32;
                let n = v.len();
                cases.push((
                    "matchLength-below-minMatch".into(),
                    v,
                    n,
                    len,
                    vec![0, 1],
                ));
            }
        }
        // d. litLength beyond the available literals -> block size mismatch
        {
            let mut v = base.clone();
            v[real].lit_length = v[real].lit_length.saturating_add(1_000_000);
            let n = v.len();
            cases.push(("litLength-too-large".into(), v, n, len, vec![0, 1]));
        }
        // e. array not ending in a terminal delimiter
        {
            let mut v = base.clone();
            v.pop();
            let n = v.len();
            cases.push(("no-terminal-delimiter".into(), v, n, len, vec![0, 1]));
        }
        // f. delimiter with matchLength != 0 -> "delimiter format error"
        {
            let mut v = base.clone();
            let last = v.len() - 1;
            v[last].match_length = 7;
            let n = v.len();
            cases.push(("delimiter-with-matchLength".into(), v, n, len, vec![0, 1]));
        }
        // g. srcSize disagreeing with sum(litLength + matchLength)
        for delta in [-1i64, -17, 1, 33] {
            let s = (len as i64 + delta) as usize;
            cases.push((
                format!("srcSize-mismatch({delta:+})"),
                base.clone(),
                base.len(),
                s,
                vec![0, 1],
            ));
        }
        // h. nbSequences == 0 with a non-empty src
        cases.push(("nbSequences0".into(), base.clone(), 0, len, vec![0, 1]));
        // i. truncated arrays: every prefix of the array
        for take in [1usize, real, base.len() / 2, base.len() - 1] {
            if take < base.len() {
                cases.push((
                    format!("truncated-to-{take}"),
                    base.clone(),
                    take,
                    len,
                    vec![0, 1],
                ));
            }
        }

        for (name, seqs, nb, src_size, validates) in cases {
            for &validate in &validates {
                for &delim in &[ZSTD_sf_explicitBlockDelimiters] {
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                    }
                    if !apply_params(
                        &ec,
                        (*c_set, *r_set),
                        cc,
                        rc,
                        &[
                            (ZSTD_c_compressionLevel, 3),
                            (ZSTD_c_blockDelimiters, delim),
                            (ZSTD_c_validateSequences, validate),
                            // windowLog 20 bounds the legal offsets, so case (b)
                            // is unambiguously out of range
                            (ZSTD_c_windowLog, 20),
                            (ZSTD_c_minMatch, 4),
                        ],
                    ) {
                        continue;
                    }
                    let cap = unsafe { c_bound(src_size.max(len)) } + 128;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let use_size = src_size.min(src.len());
                    let a = unsafe {
                        c_cs(
                            cc,
                            cb.as_mut_ptr(),
                            cap,
                            seqs.as_ptr(),
                            nb,
                            src.as_ptr(),
                            use_size,
                        )
                    };
                    let b = unsafe {
                        r_cs(
                            rc,
                            rb.as_mut_ptr(),
                            cap,
                            seqs.as_ptr(),
                            nb,
                            src.as_ptr(),
                            use_size,
                        )
                    };
                    let tag = format!(
                        "invalid[{name}] len={len} nb={nb} srcSize={use_size} validate={validate}"
                    );
                    if !ec.check(&tag, a, b) {
                        assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
                    }
                }
            }
        }

        // --- validateSequences=1 must reject exactly what =0 accepts for the
        //     "matchLength below minMatch" case.
        {
            let mut v = base.clone();
            let delta = v[real].match_length as i64 - 3;
            if delta > 0 {
                v[real].match_length = 3;
                v[real].lit_length += delta as u32;
                let mut results = [0usize; 2];
                for (k, &validate) in [0i32, 1].iter().enumerate() {
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                    }
                    apply_params(
                        &ec,
                        (*c_set, *r_set),
                        cc,
                        rc,
                        &[
                            (ZSTD_c_compressionLevel, 3),
                            (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                            (ZSTD_c_validateSequences, validate),
                            (ZSTD_c_minMatch, 4),
                        ],
                    );
                    let cap = unsafe { c_bound(len) } + 128;
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let a = unsafe {
                        c_cs(cc, cb.as_mut_ptr(), cap, v.as_ptr(), v.len(), src.as_ptr(), len)
                    };
                    let b = unsafe {
                        r_cs(rc, rb.as_mut_ptr(), cap, v.as_ptr(), v.len(), src.as_ptr(), len)
                    };
                    let tag = format!("minMatch-asymmetry len={len} validate={validate}");
                    ec.check(&tag, a, b);
                    results[k] = a;
                }
                // if the C accepted with validation off it must reject with it on
                if !is_err(results[0]) {
                    assert!(
                        is_err(results[1]),
                        "validateSequences=1 must reject the short match (len={len})"
                    );
                    assert_eq_dbg(
                        "short match must be externalSequences_invalid",
                        ec.code(results[1]),
                        ZSTD_error_externalSequences_invalid,
                    );
                }
            }
        }
    }

    // --- nbSequences == 0 / NULL sequence pointer.
    // `ZSTD_compressSequences` never dereferences `inSeqs` when
    // `nbSequences == 0` in no-delimiter mode (nor when srcSize == 0), so this
    // is a well-defined API misuse the two libraries must handle identically.
    for &delim in &[ZSTD_sf_noBlockDelimiters, ZSTD_sf_explicitBlockDelimiters] {
        for &len in &[0usize, 1, 400, 200_000] {
            let src = gen_logish(&mut rng, len);
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
            }
            if !apply_params(
                &ec,
                (*c_set, *r_set),
                cc,
                rc,
                &[
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_blockDelimiters, delim),
                ],
            ) {
                continue;
            }
            let cap = unsafe { c_bound(len) } + 128;
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe {
                c_cs(
                    cc,
                    cb.as_mut_ptr(),
                    cap,
                    std::ptr::null(),
                    0,
                    src.as_ptr(),
                    len,
                )
            };
            let b = unsafe {
                r_cs(
                    rc,
                    rb.as_mut_ptr(),
                    cap,
                    std::ptr::null(),
                    0,
                    src.as_ptr(),
                    len,
                )
            };
            let tag = format!("compressSequences(NULL, 0) delim={delim} len={len}");
            if !ec.check(&tag, a, b) {
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

// =============================================== 7. generateSequences errors

/// `ZSTD_generateSequences` error paths: an undersized output array
/// (`dstSize_tooSmall`), the parameters it refuses outright
/// (`targetCBlockSize != 0`) and incompressible input (`sequenceProducer_failed`
/// via the "Uncompressible block" guard).
#[test]
fn generate_sequences_error_paths_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_gen, r_gen) = i.pair::<Fn_generateSequences>("ZSTD_generateSequences");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let ec = ErrCmp::new();
    let mut rng = Rng::new(0x9E_E9_0001);

    // ---- undersized output capacity, swept from 0 up to the exact count
    for &len in &[0usize, 1, 200, 4_000, 140_000] {
        let src = gen_logish(&mut rng, len);
        let bound = unsafe { c_sb(len) };
        let need = match generate_both(&ec, &[(ZSTD_c_compressionLevel, 3)], &src, bound, "need") {
            Some(g) => g.seqs.len(),
            None => continue,
        };
        let mut caps: Vec<usize> = (0..=need.min(24)).collect();
        let step = (need / 12).max(1);
        let mut c = 0usize;
        while c <= need + 2 {
            caps.push(c);
            c += step;
        }
        caps.push(need);
        caps.push(need + 1);
        caps.sort_unstable();
        caps.dedup();
        for cap in caps {
            let tag = format!("generateSequences len={len} outCapacity={cap} (need={need})");
            // parity of the return (and of the array prefix) is asserted inside
            generate_both(&ec, &[(ZSTD_c_compressionLevel, 3)], &src, cap, &tag);
        }
    }

    // ---- parameters generateSequences refuses
    for &(id, v, what) in &[
        (ZSTD_c_targetCBlockSize, 1340, "targetCBlockSize=1340"),
        (ZSTD_c_targetCBlockSize, 65_536, "targetCBlockSize=65536"),
        (ZSTD_c_targetCBlockSize, 0, "targetCBlockSize=0 (allowed)"),
    ] {
        let src = gen_logish(&mut rng, 20_000);
        let bound = unsafe { c_sb(src.len()) };
        let tag = format!("generateSequences with {what}");
        generate_both(
            &ec,
            &[(ZSTD_c_compressionLevel, 3), (id, v)],
            &src,
            bound,
            &tag,
        );
    }

    // ---- incompressible input: the C bails out with sequenceProducer_failed
    {
        let (cc, rc) = unsafe { (c_new(), r_new()) };
        for &len in &[2_000usize, 200_000] {
            let src = gen_shape(Shape::Random, len, &mut rng);
            let bound = unsafe { c_sb(len) };
            let mut cs = vec![ZSTD_Sequence::default(); bound];
            let mut rs = vec![ZSTD_Sequence::default(); bound];
            unsafe {
                c_set(cc, ZSTD_c_compressionLevel, 3);
                r_set(rc, ZSTD_c_compressionLevel, 3);
            }
            let a = unsafe { c_gen(cc, cs.as_mut_ptr(), bound, src.as_ptr(), len) };
            let b = unsafe { r_gen(rc, rs.as_mut_ptr(), bound, src.as_ptr(), len) };
            let tag = format!("generateSequences(incompressible len={len})");
            if ec.check(&tag, a, b) {
                assert_eq_dbg(
                    &format!("{tag} / code"),
                    ec.code(a),
                    ZSTD_error_sequenceProducer_failed,
                );
            } else {
                assert_seqs_eq(&tag, &cs[..a], &rs[..b]);
            }
        }
        // these contexts still carry a live seqCollector: do not reuse them
        unsafe {
            c_free(cc);
            r_free(rc);
        }
    }
}

// ==================================== 8. compressSequencesAndLiterals variant

/// `ZSTD_compressSequencesAndLiterals`: explicit delimiters only, validation
/// and checksum must be off, `litBufCapacity >= litSize + 8`, and
/// `decompressedSize` must be exact. Covers the happy path (identical frames +
/// round trip) and each documented rejection.
#[test]
fn compress_sequences_and_literals_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_csl, r_csl) =
        i.pair::<Fn_compressSequencesAndLiterals>("ZSTD_compressSequencesAndLiterals");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dec, r_dec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
    let ec = ErrCmp::new();

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };
    let mut rng = Rng::new(0xCA_11_0001);

    for &len in &[0usize, 1, 700, 9_000, 60_000, 200_000] {
        let src = gen_logish(&mut rng, len);
        let bound = unsafe { c_sb(len) };
        for &lvl in &[1i32, 5, 13, 19] {
            let gtag = format!("csl len={len} lvl={lvl}");
            let gen = match generate_both(
                &ec,
                &[(ZSTD_c_compressionLevel, lvl)],
                &src,
                bound,
                &gtag,
            ) {
                Some(g) => g,
                None => continue,
            };
            let lits = match extract_literals(&src, &gen.seqs) {
                Some(l) => l,
                None => continue,
            };
            // the API requires >= 8 bytes of slack behind the literals
            let mut litbuf = lits.clone();
            litbuf.extend_from_slice(&[0u8; 16]);
            let litcap = litbuf.len();

            // (name, nbSeqs, litSize, litCapacity, decompressedSize, extra params)
            let variants: Vec<(String, usize, usize, usize, usize, Vec<(i32, i32)>)> = vec![
                (
                    "valid".into(),
                    gen.seqs.len(),
                    lits.len(),
                    litcap,
                    len,
                    vec![],
                ),
                (
                    "noBlockDelimiters (unsupported)".into(),
                    gen.seqs.len(),
                    lits.len(),
                    litcap,
                    len,
                    vec![(ZSTD_c_blockDelimiters, ZSTD_sf_noBlockDelimiters)],
                ),
                (
                    "validateSequences (unsupported)".into(),
                    gen.seqs.len(),
                    lits.len(),
                    litcap,
                    len,
                    vec![(ZSTD_c_validateSequences, 1)],
                ),
                (
                    "checksum (unsupported)".into(),
                    gen.seqs.len(),
                    lits.len(),
                    litcap,
                    len,
                    vec![(ZSTD_c_checksumFlag, 1)],
                ),
                (
                    "litCapacity < litSize".into(),
                    gen.seqs.len(),
                    lits.len(),
                    lits.len().saturating_sub(1),
                    len,
                    vec![],
                ),
                (
                    "nbSequences == 0".into(),
                    0,
                    lits.len(),
                    litcap,
                    len,
                    vec![],
                ),
                (
                    "wrong decompressedSize".into(),
                    gen.seqs.len(),
                    lits.len(),
                    litcap,
                    len + 7,
                    vec![],
                ),
                (
                    "litSize too small".into(),
                    gen.seqs.len(),
                    lits.len() / 2,
                    litcap,
                    len,
                    vec![],
                ),
            ];

            for (name, nb, lit_size, lit_cap, dsize, extra) in variants {
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                }
                let mut params: Vec<(i32, i32)> = vec![
                    (ZSTD_c_compressionLevel, lvl),
                    (ZSTD_c_blockDelimiters, ZSTD_sf_explicitBlockDelimiters),
                    (ZSTD_c_checksumFlag, 0),
                    (ZSTD_c_validateSequences, 0),
                ];
                params.extend(extra.iter().copied());
                if !apply_params(&ec, (*c_set, *r_set), cc, rc, &params) {
                    continue;
                }
                let cap = unsafe { c_bound(len) } + 256;
                let mut cb = vec![0xC3u8; cap];
                let mut rb = vec![0x3Cu8; cap];
                let a = unsafe {
                    c_csl(
                        cc,
                        cb.as_mut_ptr(),
                        cap,
                        gen.seqs.as_ptr(),
                        nb,
                        litbuf.as_ptr(),
                        lit_size,
                        lit_cap,
                        dsize,
                    )
                };
                let b = unsafe {
                    r_csl(
                        rc,
                        rb.as_mut_ptr(),
                        cap,
                        gen.seqs.as_ptr(),
                        nb,
                        litbuf.as_ptr(),
                        lit_size,
                        lit_cap,
                        dsize,
                    )
                };
                let tag = format!("compressSequencesAndLiterals[{name}] {gtag}");
                if ec.check(&tag, a, b) {
                    continue;
                }
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
                if name == "valid" {
                    let mut o1 = vec![0u8; len + 16];
                    let mut o2 = vec![0u8; len + 16];
                    let n1 = unsafe { r_dec(rd, o1.as_mut_ptr(), o1.len(), cb.as_ptr(), a) };
                    let n2 = unsafe { c_dec(cd, o2.as_mut_ptr(), o2.len(), rb.as_ptr(), b) };
                    assert_eq_dbg(&format!("{tag} / rust decodes C"), n1, len);
                    assert_eq_dbg(&format!("{tag} / C decodes rust"), n2, len);
                    assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                    assert_bytes_eq(&format!("{tag} / payload"), &src, &o2[..n2]);
                }
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

// ================================================= 9. sequences + dictionaries

/// Training samples for the dictionaries used below.
fn make_samples(seed: u64, n: usize) -> (Vec<u8>, Vec<usize>) {
    let mut rng = Rng::new(seed);
    let mut buf = Vec::new();
    let mut sizes = Vec::with_capacity(n);
    for _ in 0..n {
        let len = rng.range(48, 200);
        let one = gen_logish(&mut rng, len);
        sizes.push(one.len());
        buf.extend_from_slice(&one);
    }
    (buf, sizes)
}

/// The header documents that `ZSTD_compressSequences` honours a dictionary
/// referenced on the cctx: the dictionary supplies the starting entropy tables
/// and repcodes, contributes `dictSize` to the offset validation bound and puts
/// a dictID in the frame header. Cross that with both delimiter modes, both
/// validation settings and a raw / trained / empty dictionary.
#[test]
fn compress_sequences_with_dictionary_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_ld, r_ld) = i.pair::<Fn_loadDict>("ZSTD_CCtx_loadDictionary");
    let (c_cs, r_cs) = i.pair::<Fn_compressSequences>("ZSTD_compressSequences");
    let (c_mg, r_mg) = i.pair::<Fn_mergeBlockDelimiters>("ZSTD_mergeBlockDelimiters");
    let (c_sb, _) = i.pair::<Fn_sequenceBound>("ZSTD_sequenceBound");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (cd_new, rd_new) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (cd_free, rd_free) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dud, r_dud) = i.pair::<Fn_decompress_usingDict>("ZSTD_decompress_usingDict");
    let (c_ff, r_ff) = i.pair::<Fn_dictID_fromFrame>("ZSTD_getDictID_fromFrame");
    let (c_tr, _) = i.pair::<Fn_train>("ZDICT_trainFromBuffer");
    let ec = ErrCmp::new();

    let mut rng = Rng::new(0xD1C7_5E90);

    // dictionary corpus: none, raw prefix content, a real trained dictionary
    let raw_small = gen_logish(&mut rng, 1_024);
    let raw_big = gen_logish(&mut rng, 100_000);
    let trained = {
        let (buf, sizes) = make_samples(0xF00D, 900);
        let mut d = vec![0u8; 8 * 1024];
        let n = unsafe {
            c_tr(
                d.as_mut_ptr(),
                d.len(),
                buf.as_ptr(),
                sizes.as_ptr(),
                sizes.len() as u32,
            )
        };
        assert!(!is_err(n), "ZDICT_trainFromBuffer failed: {n:#x}");
        d.truncate(n);
        d
    };
    let dicts: [(&str, &[u8]); 4] = [
        ("none", &[]),
        ("raw-1k", &raw_small),
        ("raw-100k", &raw_big),
        ("trained-8k", &trained),
    ];

    let (cc, rc) = unsafe { (c_new(), r_new()) };
    let (cd, rd) = unsafe { (cd_new(), rd_new()) };

    for (dname, dict) in dicts {
        for &gen_with_dict in &[false, true] {
            for &len in &[0usize, 1, 400, 7_000, 60_000, 200_000] {
                let src = gen_logish(&mut rng, len);
                let bound = unsafe { c_sb(len) };
                for &lvl in &[1i32, 6, 12, 19] {
                    let gtag =
                        format!("seq+dict[{dname}] genWithDict={gen_with_dict} len={len} lvl={lvl}");
                    let gdict: &[u8] = if gen_with_dict { dict } else { &[] };
                    let gen = match generate_both_dict(
                        &ec,
                        &[(ZSTD_c_compressionLevel, lvl)],
                        gdict,
                        &src,
                        bound,
                        &gtag,
                    ) {
                        Some(g) => g,
                        None => continue,
                    };
                    let merged = {
                        let mut a = gen.seqs.clone();
                        let mut b = gen.seqs.clone();
                        let x = unsafe { c_mg(a.as_mut_ptr(), a.len()) };
                        let y = unsafe { r_mg(b.as_mut_ptr(), b.len()) };
                        assert_eq_dbg(&format!("{gtag} / merge"), x, y);
                        assert_seqs_eq(&format!("{gtag} / merge array"), &a, &b);
                        a[..x].to_vec()
                    };

                    for &delim in
                        &[ZSTD_sf_explicitBlockDelimiters, ZSTD_sf_noBlockDelimiters]
                    {
                        let seqs: &[ZSTD_Sequence] =
                            if delim == ZSTD_sf_explicitBlockDelimiters {
                                &gen.seqs
                            } else {
                                &merged
                            };
                        for &validate in &[0i32, 1] {
                            for &dict_id_flag in &[0i32, 1] {
                                unsafe {
                                    c_rst(cc, ZSTD_reset_session_and_parameters);
                                    r_rst(rc, ZSTD_reset_session_and_parameters);
                                }
                                if !dict.is_empty() {
                                    let (a, b) = unsafe {
                                        (
                                            c_ld(cc, dict.as_ptr(), dict.len()),
                                            r_ld(rc, dict.as_ptr(), dict.len()),
                                        )
                                    };
                                    if ec.check(&format!("{gtag} / loadDictionary"), a, b) {
                                        continue;
                                    }
                                }
                                if !apply_params(
                                    &ec,
                                    (*c_set, *r_set),
                                    cc,
                                    rc,
                                    &[
                                        (ZSTD_c_compressionLevel, lvl),
                                        (ZSTD_c_blockDelimiters, delim),
                                        (ZSTD_c_validateSequences, validate),
                                        (ZSTD_c_dictIDFlag, dict_id_flag),
                                    ],
                                ) {
                                    continue;
                                }
                                let cap = unsafe { c_bound(len) } + 128;
                                let mut cb = vec![0xA5u8; cap];
                                let mut rb = vec![0x5Au8; cap];
                                let a = unsafe {
                                    c_cs(
                                        cc,
                                        cb.as_mut_ptr(),
                                        cap,
                                        seqs.as_ptr(),
                                        seqs.len(),
                                        src.as_ptr(),
                                        len,
                                    )
                                };
                                let b = unsafe {
                                    r_cs(
                                        rc,
                                        rb.as_mut_ptr(),
                                        cap,
                                        seqs.as_ptr(),
                                        seqs.len(),
                                        src.as_ptr(),
                                        len,
                                    )
                                };
                                let tag = format!(
                                    "{gtag} delim={delim} validate={validate} dictID={dict_id_flag}"
                                );
                                if ec.check(&tag, a, b) {
                                    continue;
                                }
                                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
                                unsafe {
                                    assert_eq_dbg(
                                        &format!("{tag} / dictID_fromFrame"),
                                        c_ff(cb.as_ptr(), a),
                                        r_ff(rb.as_ptr(), b),
                                    );
                                }
                                // cross round trip with the same dictionary
                                let mut o1 = vec![0u8; len + 16];
                                let mut o2 = vec![0u8; len + 16];
                                let n1 = unsafe {
                                    r_dud(
                                        rd,
                                        o1.as_mut_ptr(),
                                        o1.len(),
                                        cb.as_ptr(),
                                        a,
                                        dict.as_ptr(),
                                        dict.len(),
                                    )
                                };
                                let n2 = unsafe {
                                    c_dud(
                                        cd,
                                        o2.as_mut_ptr(),
                                        o2.len(),
                                        rb.as_ptr(),
                                        b,
                                        dict.as_ptr(),
                                        dict.len(),
                                    )
                                };
                                assert_eq_dbg(&format!("{tag} / rust decodes C"), n1, len);
                                assert_eq_dbg(&format!("{tag} / C decodes rust"), n2, len);
                                assert_bytes_eq(&format!("{tag} / payload"), &src, &o1[..n1]);
                                assert_bytes_eq(&format!("{tag} / payload"), &src, &o2[..n2]);
                            }
                        }
                    }
                }
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
