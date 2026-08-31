//! Phase D, final gap closure: the last exported symbols that are still
//! callable across the FFI boundary using only public/plain types.
//!
//! Everything left uncovered after this file takes a *private* struct type by
//! pointer (`SeqStore_t*`, `ZSTD_MatchState_t*`, `RawSeqStore_t*`,
//! `ZSTD_entropyCTables_t*`, `ZSTD_CCtx_params*`, `rawSeq*`, `ZSTDMT_CCtx*`,
//! `ZSTD_hufCTables_t*`, ...). Those have no public layout, so no external
//! consumer can construct a valid argument for them; they are reached only
//! through the public API and are therefore covered indirectly by the rest of
//! the suite. See `SYMBOLS.md` for the full accounting.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;

type Fn_errCode = unsafe extern "C" fn(usize) -> i32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;

/// `BlockSummary` — { size_t nbSequences; size_t blockSize; size_t litSize; }
/// (compress/zstd_compress_internal.h)
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct BlockSummary {
    nb_sequences: usize,
    block_size: usize,
    lit_size: usize,
}

/// Compare two `BlockSummary` results.
///
/// IMPORTANT: on its failure path the C does
/// ```c
/// BlockSummary bs;
/// bs.nbSequences = ERROR(externalSequences_invalid);
/// return bs;            /* blockSize and litSize NEVER ASSIGNED */
/// ```
/// (zstd_compress.c:7465-7469), so when `nbSequences` is an error code the other
/// two fields are indeterminate stack garbage in the C — reading them is UB and
/// they were observed to hold unrelated values. Only `nbSequences` carries
/// meaning there, so only that field is compared in the error case. On the
/// success path all three fields are compared.
fn cmp_summary(tag: &str, c: BlockSummary, r: BlockSummary) {
    // error codes are small negated values, i.e. very large usize
    let is_err = c.nb_sequences > usize::MAX - 256;
    assert_eq_dbg(&format!("{tag} / nbSequences"), c.nb_sequences, r.nb_sequences);
    if !is_err {
        assert_eq_dbg(&format!("{tag} / blockSize"), c.block_size, r.block_size);
        assert_eq_dbg(&format!("{tag} / litSize"), c.lit_size, r.lit_size);
    }
}

/// `ZSTD_get1BlockSummary(const ZSTD_Sequence* seqs, size_t nbSeqs)` — returns a
/// 3-word struct by value; summarises one block's worth of a sequence array.
#[test]
fn get1_block_summary_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(*const ZSTD_Sequence, usize) -> BlockSummary>(
        "ZSTD_get1BlockSummary",
    );

    let mut rng = Rng::new(0xB105_0001);

    // (a) real sequence arrays produced by ZSTD_generateSequences
    let (c_new, _) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_free, _) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_gen, _) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *mut ZSTD_Sequence,
        usize,
        *const u8,
        usize,
    ) -> usize>("ZSTD_generateSequences");
    let (c_sb, _) = i.pair::<Fn_bound>("ZSTD_sequenceBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    for &shape in &ALL_SHAPES {
        for &len in &[1usize, 100, 5000, 60_000] {
            let src = gen_shape(shape, len, &mut rng);
            let cap = unsafe { c_sb(len) };
            let mut seqs = vec![ZSTD_Sequence::default(); cap.max(1)];
            let cctx = unsafe { c_new() };
            let n = unsafe { c_gen(cctx, seqs.as_mut_ptr(), cap, src.as_ptr(), len) };
            unsafe { c_free(cctx) };
            if unsafe { c_isE(n) } != 0 {
                continue;
            }
            // summarise every prefix length, so the terminator handling is hit
            for take in 0..=n {
                let a = unsafe { c(seqs.as_ptr(), take) };
                let b = unsafe { r(seqs.as_ptr(), take) };
                cmp_summary(
                    &format!("get1BlockSummary shape={shape:?} len={len} take={take}/{n}"),
                    a,
                    b,
                );
            }
        }
    }

    // (b) randomized (often nonsensical) sequence arrays — the C has no
    // validation here, so any array is a legal input
    for _ in 0..4000 {
        let n = rng.range(1, 40);
        let mut seqs = Vec::with_capacity(n);
        for _ in 0..n {
            seqs.push(ZSTD_Sequence {
                offset: rng.next_u32() % 70_000,
                lit_length: rng.next_u32() % 5000,
                match_length: rng.next_u32() % 5000,
                rep: rng.next_u32() % 5,
            });
        }
        // ensure a terminator variant is exercised too
        if rng.bool() {
            let k = rng.below(n);
            seqs[k].offset = 0;
        }
        for take in [0usize, 1, n / 2, n] {
            let a = unsafe { c(seqs.as_ptr(), take) };
            let b = unsafe { r(seqs.as_ptr(), take) };
            cmp_summary(&format!("get1BlockSummary rand n={n} take={take}"), a, b);
        }
    }
}

/// `ZSTD_convertBlockSequences(cctx, inSeqs, nbSequences, repcodeResolution)` —
/// converts a public `ZSTD_Sequence` array into the cctx's internal seq store.
#[test]
fn convert_block_sequences_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_CCtx_reset");
    let (c_set, r_set) =
        i.pair::<unsafe extern "C" fn(CCtx, i32, i32) -> usize>("ZSTD_CCtx_setParameter");
    let (c_cv, r_cv) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *const ZSTD_Sequence,
        usize,
        i32,
    ) -> usize>("ZSTD_convertBlockSequences");
    let (c_gen, _) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *mut ZSTD_Sequence,
        usize,
        *const u8,
        usize,
    ) -> usize>("ZSTD_generateSequences");
    let (c_sb, _) = i.pair::<Fn_bound>("ZSTD_sequenceBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0xC0B5_0001);

    for &shape in &[Shape::SkewedText, Shape::Tabular, Shape::Repetitive, Shape::Random] {
        for &len in &[1usize, 500, 20_000] {
            let src = gen_shape(shape, len, &mut rng);
            let cap = unsafe { c_sb(len) };
            let mut seqs = vec![ZSTD_Sequence::default(); cap.max(1)];
            let gctx = unsafe { c_new() };
            let n = unsafe { c_gen(gctx, seqs.as_mut_ptr(), cap, src.as_ptr(), len) };
            unsafe { c_free(gctx) };
            if unsafe { c_isE(n) } != 0 {
                continue;
            }

            for &rcres in &[0i32, 1] {
                // the cctx must be in a started state for the internal seq store
                // to exist, so drive a real begin first
                for &lvl in &[1i32, 9] {
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                        c_set(cc, ZSTD_c_compressionLevel, lvl);
                        r_set(rc, ZSTD_c_compressionLevel, lvl);
                    }
                    let a = unsafe { c_cv(cc, seqs.as_ptr(), n, rcres) };
                    let b = unsafe { r_cv(rc, seqs.as_ptr(), n, rcres) };
                    let tag = format!(
                        "convertBlockSequences shape={shape:?} len={len} n={n} rcres={rcres} lvl={lvl}"
                    );
                    assert_eq_dbg(&tag, a, b);
                    unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
                }
            }

            // truncated / empty sequence arrays
            for take in [0usize, 1, n / 2] {
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                }
                let a = unsafe { c_cv(cc, seqs.as_ptr(), take, 0) };
                let b = unsafe { r_cv(rc, seqs.as_ptr(), take, 0) };
                let tag = format!("convertBlockSequences take={take}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// `ZSTD_checkContinuity(dctx, dst, dstSize)` — updates the dctx's notion of
/// output buffer continuity. Observable through subsequent decoding.
#[test]
fn check_continuity_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> DCtx>("ZSTD_createDCtx");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_freeDCtx");
    let (c_cc, r_cc) =
        i.pair::<unsafe extern "C" fn(DCtx, *const u8, usize)>("ZSTD_checkContinuity");
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");
    let (c_db, r_db) =
        i.pair::<unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_decompressBlock",
        );
    let (c_cbeg, _) = i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_compressBegin");
    let (c_cnew, _) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_cfree, _) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_cblk, _) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compressBlock",
        );
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cd = unsafe { c_new() };
    let rd = unsafe { r_new() };
    let mut rng = Rng::new(0xC047_0001);

    for &shape in &[Shape::SkewedText, Shape::Repetitive, Shape::Random] {
        for &len in &[1usize, 500, 40_000] {
            let src = gen_shape(shape, len, &mut rng);
            // produce a raw block with the C
            let cctx = unsafe { c_cnew() };
            unsafe { c_cbeg(cctx, 3) };
            let mut blk = vec![0u8; len + 1024];
            let bn = unsafe { c_cblk(cctx, blk.as_mut_ptr(), blk.len(), src.as_ptr(), len) };
            unsafe { c_cfree(cctx) };
            if unsafe { c_isE(bn) } != 0 || bn == 0 {
                continue;
            }

            // decode with an explicit checkContinuity announcement first, at
            // several output offsets (contiguous vs discontiguous)
            for off in [0usize, 1, 64, 1000] {
                let mut d1 = vec![0u8; len + off + 64];
                let mut d2 = vec![0u8; len + off + 64];
                unsafe {
                    c_dbeg(cd);
                    r_dbeg(rd);
                    c_cc(cd, d1.as_ptr().add(off), len);
                    r_cc(rd, d2.as_ptr().add(off), len);
                }
                let a = unsafe {
                    c_db(cd, d1.as_mut_ptr().add(off), len + 64, blk.as_ptr(), bn)
                };
                let b = unsafe {
                    r_db(rd, d2.as_mut_ptr().add(off), len + 64, blk.as_ptr(), bn)
                };
                let tag =
                    format!("checkContinuity shape={shape:?} len={len} off={off}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
                if unsafe { c_isE(a) } == 0 {
                    assert_bytes_eq(
                        &format!("{tag} payload"),
                        &d1[off..off + a],
                        &d2[off..off + b],
                    );
                }
            }

            // checkContinuity with a zero size and with NULL dst
            unsafe {
                c_dbeg(cd);
                r_dbeg(rd);
                c_cc(cd, std::ptr::null(), 0);
                r_cc(rd, std::ptr::null(), 0);
            }
        }
    }

    unsafe {
        c_free(cd);
        r_free(rd);
    }
}

/// `ZSTD_decodeLiteralsBlock_wrapper(dctx, src, srcSize, dst, dstCapacity)` —
/// parses a literals section. Driven with real literal blocks and with fuzz.
#[test]
fn decode_literals_block_wrapper_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> DCtx>("ZSTD_createDCtx");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_freeDCtx");
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");
    let (c_dl, r_dl) = i.pair::<unsafe extern "C" fn(
        DCtx,
        *const u8,
        usize,
        *mut u8,
        usize,
    ) -> usize>("ZSTD_decodeLiteralsBlock_wrapper");
    let (c_nc, _) = i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize>(
        "ZSTD_noCompressLiterals",
    );
    let (c_rle, _) = i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize>(
        "ZSTD_compressRleLiteralsBlock",
    );
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cd = unsafe { c_new() };
    let rd = unsafe { r_new() };
    let mut rng = Rng::new(0xD117_0001);

    // (a) well-formed raw and RLE literals sections
    let mut blocks: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [1usize, 2, 31, 32, 4095, 4096, 40_000] {
        let src = gen_shape(Shape::Random, n, &mut rng);
        let mut b = vec![0u8; n + 8];
        let k = unsafe { c_nc(b.as_mut_ptr(), b.len(), src.as_ptr(), n) };
        if unsafe { c_isE(k) } == 0 {
            b.truncate(k);
            blocks.push((format!("raw{n}"), b));
        }
        let cst = vec![0xA7u8; n];
        let mut b = vec![0u8; 8];
        let k = unsafe { c_rle(b.as_mut_ptr(), b.len(), cst.as_ptr(), n) };
        if unsafe { c_isE(k) } == 0 {
            b.truncate(k);
            blocks.push((format!("rle{n}"), b));
        }
    }

    for (name, blk) in &blocks {
        // every truncation, and a range of dst capacities
        for take in 0..=blk.len() {
            for dcap in [0usize, 1, 64, 1 << 18] {
                let mut d1 = vec![0u8; dcap.max(1)];
                let mut d2 = vec![0u8; dcap.max(1)];
                unsafe {
                    c_dbeg(cd);
                    r_dbeg(rd);
                }
                let a =
                    unsafe { c_dl(cd, blk.as_ptr(), take, d1.as_mut_ptr(), dcap) };
                let b =
                    unsafe { r_dl(rd, blk.as_ptr(), take, d2.as_mut_ptr(), dcap) };
                let tag = format!("decodeLiteralsBlock[{name}] take={take} dcap={dcap}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
            }
        }
    }

    // (b) fuzz: random literal-section headers
    for _ in 0..20_000 {
        let n = rng.range(0, 24);
        let mut buf = vec![0u8; n.max(1)];
        for x in buf.iter_mut() {
            *x = rng.byte();
        }
        let dcap = 1usize << 18;
        let mut d1 = vec![0u8; dcap];
        let mut d2 = vec![0u8; dcap];
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
        }
        let a = unsafe { c_dl(cd, buf.as_ptr(), n, d1.as_mut_ptr(), dcap) };
        let b = unsafe { r_dl(rd, buf.as_ptr(), n, d2.as_mut_ptr(), dcap) };
        let tag = format!("decodeLiteralsBlock fuzz n={n} {:02x?}", &buf[..n.min(6)]);
        assert_eq_dbg(&tag, a, b);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
    }

    unsafe {
        c_free(cd);
        r_free(rd);
    }
}

/// `ZSTD_crossEntropyCost(norm, accuracyLog, count, max)` — pure arithmetic over
/// a normalized-count table and a histogram.
#[test]
fn cross_entropy_cost_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(*const i16, u32, *const u32, u32) -> usize>(
        "ZSTD_crossEntropyCost",
    );

    let mut rng = Rng::new(0xCE05_0001);

    // PRECONDITIONS taken from the C body (zstd_compress_sequences.c:139-154):
    //   unsigned const shift = 8 - accuracyLog;      => accuracyLog <= 8
    //   normAcc = (norm[s] != -1) ? norm[s] : 1;
    //   norm256 = normAcc << shift;  assert(0 < norm256 < 256);
    //   cost += count[s] * kInverseProbabilityLog256[norm256];
    // `kInverseProbabilityLog256` has exactly 256 entries, so `norm256 == 256`
    // (i.e. a single symbol taking the whole table) reads one past the end.
    // The generator below therefore keeps accuracyLog in 1..=8 and every entry
    // strictly below `1 << accuracyLog`, which is what a real normalized count
    // with >= 2 used symbols always satisfies.
    for _ in 0..20_000 {
        // `max` is the highest symbol value; both arrays must have max+1 entries
        let max = rng.range(1, 63) as u32; // >= 1 so at least two symbols exist
        let n = (max + 1) as usize;
        let alog = rng.range(1, 8) as u32; // accuracyLog: tableLog of `norm`
        let total = 1u32 << alog;

        // build a VALID normalized count: entries sum to 2^alog, every entry
        // strictly below 2^alog, and -1 allowed as the low-probability marker.
        let cap_per_entry = (total - 1).max(1) as i32;
        let mut norm = vec![0i16; n];
        let mut left = total as i32;
        for k in 0..n {
            if left <= 0 {
                break;
            }
            let room = cap_per_entry.min(left);
            let v = if k + 1 == n {
                room
            } else {
                (rng.below(room as usize + 1)) as i32
            };
            norm[k] = v as i16;
            left -= v;
        }
        // distribute any remainder without exceeding the per-entry cap
        let mut k = 0usize;
        while left > 0 {
            if (norm[k % n] as i32) < cap_per_entry {
                norm[k % n] += 1;
                left -= 1;
            }
            k += 1;
            if k > 64 * n {
                break;
            }
        }
        // sprinkle -1 "low probability" markers (normAcc becomes 1, still valid)
        for k in 0..n {
            if norm[k] == 0 && rng.bool() {
                norm[k] = -1;
            }
        }

        let mut count = vec![0u32; n];
        for k in 0..n {
            count[k] = rng.next_u32() % 10_000;
        }

        let a = unsafe { c(norm.as_ptr(), alog, count.as_ptr(), max) };
        let b = unsafe { r(norm.as_ptr(), alog, count.as_ptr(), max) };
        assert_eq_dbg(
            &format!("crossEntropyCost max={max} alog={alog} norm={norm:?} count={count:?}"),
            a,
            b,
        );
    }
}

/// `ZSTD_splitBlock(blockStart, blockSize, level, workspace, wkspSize)` — the
/// pre-split heuristic.
///
/// The header states: "For the time being, this function only accepts full
/// 128 KB blocks. Therefore, @blockSize must be == 128 KB." That precondition is
/// respected here; the workspace must be `ZSTD_SLIPBLOCK_WORKSPACESIZE` (8208).
#[test]
fn split_block_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(*const u8, usize, i32, *mut u8, usize) -> usize>(
        "ZSTD_splitBlock",
    );
    const WKSP: usize = 8208; // ZSTD_SLIPBLOCK_WORKSPACESIZE
    const BLK: usize = 128 * 1024;

    let mut rng = Rng::new(0x5911_0001);

    for &shape in &ALL_SHAPES {
        for _ in 0..3 {
            let src = gen_shape(shape, BLK, &mut rng);
            // `level` selects the split aggressiveness (ZSTD_c_blockSplitterLevel)
            // `assert(0<=level && level<=4)` (zstd_preSplit.c:233): level 5/6
            // would reach ZSTD_splitBlock_byChunks(level-1 = 4/5) and index its
            // internal tables out of range — UB in the C, verified to crash.
            for level in 0i32..=4 {
                // the workspace is cast to `FPStats*` (which contains `unsigned`
                // arrays), so give it real word alignment rather than a
                // byte-aligned Vec<u8>.
                let mut w1 = vec![0u64; WKSP / 8 + 1];
                let mut w2 = vec![0u64; WKSP / 8 + 1];
                let a = unsafe { c(src.as_ptr(), BLK, level, w1.as_mut_ptr() as *mut u8, WKSP) };
                let b = unsafe { r(src.as_ptr(), BLK, level, w2.as_mut_ptr() as *mut u8, WKSP) };
                assert_eq_dbg(
                    &format!("splitBlock shape={shape:?} level={level}"),
                    a,
                    b,
                );
            }
        }
    }

    // a two-phase block (a hard boundary in the middle) is exactly what the
    // splitter is meant to find — pin that the found split point agrees
    for level in 0i32..=4 {
        let mut src = vec![b'A'; BLK / 2];
        src.extend(std::iter::repeat(b'Z').take(BLK - BLK / 2));
        let mut w1 = vec![0u64; WKSP / 8 + 1];
        let mut w2 = vec![0u64; WKSP / 8 + 1];
        let a = unsafe { c(src.as_ptr(), BLK, level, w1.as_mut_ptr() as *mut u8, WKSP) };
        let b = unsafe { r(src.as_ptr(), BLK, level, w2.as_mut_ptr() as *mut u8, WKSP) };
        assert_eq_dbg(&format!("splitBlock two-phase level={level}"), a, b);
    }
}

/// `ZSTD_selectEncodingType(...)` — picks basic/rle/compressed/repeat for a
/// symbol stream.
///
/// PRECONDITIONS read out of the C body (zstd_compress_sequences.c:156-215):
///   * `assert(defaultNormLog >= 5 && defaultNormLog <= 6)` — so only 5 and 6.
///   * `assert(mult <= 9 && mult >= 7)` where `mult = 10 - strategy`, so the
///     `strategy < ZSTD_lazy` branch requires `strategy` in 1..=3. (All 9
///     strategies are still swept; 4..=9 take the other branch.)
///   * `defaultNorm` is indexed up to `max`, so `max <= MaxLL (35)` for the real
///     `LL_defaultNorm` table used here.
///   * `ZSTD_crossEntropyCost` indexes `kInverseProbabilityLog256[normAcc << (8 -
///     defaultNormLog)]`, a 256-entry table, so no `defaultNorm` entry may reach
///     `1 << defaultNormLog`.
///   * when `*repeatMode != FSE_repeat_none` the C calls
///     `ZSTD_fseBitCost(prevCTable, ...)`, which walks a REAL `FSE_CTable`. An
///     all-zero buffer is not a valid table and makes the C read out of bounds,
///     so the table below is built with `FSE_buildCTable_wksp` from the same
///     normalized counts.
///
/// `LL_defaultNorm` / `LL_DEFAULTNORMLOG` are copied verbatim from
/// `c_src/src/common/zstd_internal.h:126-132`.
#[test]
fn select_encoding_type_matches() {
    let i = impls();
    let (c, r) = i.pair::<unsafe extern "C" fn(
        *mut i32,   // FSE_repeat* repeatMode (enum, int-sized)
        *const u32, // count
        u32,        // max
        usize,      // mostFrequent
        usize,      // nbSeq
        u32,        // FSELog
        *const u32, // prevCTable (FSE_CTable*)
        *const i16, // defaultNorm
        u32,        // defaultNormLog
        i32,        // ZSTD_DefaultPolicy_e
        i32,        // ZSTD_strategy
    ) -> i32>("ZSTD_selectEncodingType");

    // FSE_buildCTable_wksp(ct, normalizedCounter, maxSymbolValue, tableLog, wksp, wkspSize)
    let (c_bct, r_bct) = i.pair::<unsafe extern "C" fn(
        *mut u32,
        *const i16,
        u32,
        u32,
        *mut u32,
        usize,
    ) -> usize>("FSE_buildCTable_wksp");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    // c_src/src/common/zstd_internal.h:126 — LL_defaultNorm[MaxLL+1], MaxLL = 35
    const LL_DEFAULT_NORM: [i16; 36] = [
        4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1,
        1, 1, 1, 1, -1, -1, -1, -1,
    ];
    const MAX_LL: u32 = 35;
    const LL_DEFAULTNORMLOG: u32 = 6;

    // sanity: the table must satisfy the crossEntropyCost bound
    for &v in LL_DEFAULT_NORM.iter() {
        let acc = if v != -1 { v as i32 } else { 1 };
        assert!(
            (acc << (8 - LL_DEFAULTNORMLOG)) < 256,
            "LL_defaultNorm entry {v} violates the kInverseProbabilityLog256 bound"
        );
    }

    // Build a REAL FSE_CTable from LL_defaultNorm, once per library, so
    // ZSTD_fseBitCost() has something valid to walk.
    // FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue)
    //   = 1 + (1 << (maxTableLog-1)) + ((maxSymbolValue+1)*2)
    let ct_u32 = 1 + (1usize << (LL_DEFAULTNORMLOG - 1)) + ((MAX_LL as usize + 1) * 2);
    // FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)
    //   = ((maxSymbolValue + 2) + (1 << tableLog))/2 + 2
    let wksp_u32 =
        ((MAX_LL as usize + 2) + (1usize << LL_DEFAULTNORMLOG)) / 2 + 2 + 8;
    let mut c_ct = vec![0u32; ct_u32];
    let mut r_ct = vec![0u32; ct_u32];
    {
        let mut w1 = vec![0u32; wksp_u32];
        let mut w2 = vec![0u32; wksp_u32];
        let a = unsafe {
            c_bct(
                c_ct.as_mut_ptr(),
                LL_DEFAULT_NORM.as_ptr(),
                MAX_LL,
                LL_DEFAULTNORMLOG,
                w1.as_mut_ptr(),
                wksp_u32 * 4,
            )
        };
        let b = unsafe {
            r_bct(
                r_ct.as_mut_ptr(),
                LL_DEFAULT_NORM.as_ptr(),
                MAX_LL,
                LL_DEFAULTNORMLOG,
                w2.as_mut_ptr(),
                wksp_u32 * 4,
            )
        };
        assert_eq_dbg("FSE_buildCTable_wksp(LL_defaultNorm)", a, b);
        assert!(unsafe { c_isE(a) } == 0, "could not build the reference CTable");
        // the two libraries must produce byte-identical tables — a strong check
        // in its own right, and a prerequisite for comparing fseBitCost below
        assert_eq_dbg("reference FSE_CTable contents", c_ct.clone(), r_ct.clone());
    }

    let mut rng = Rng::new(0x5E1E_0001);

    for _ in 0..8000 {
        let max = rng.range(1, MAX_LL as usize) as u32;
        let n = (max + 1) as usize;
        let mut count = vec![0u32; n];
        let mut total = 0usize;
        let mut most = 0usize;
        for k in 0..n {
            let v = rng.next_u32() % 2000;
            count[k] = v;
            total += v as usize;
            if v as usize > most {
                most = v as usize;
            }
        }
        // INVARIANT: `nbSeq` is the TOTAL of `count[]`. Real callers always pass
        // the true sum, and the C relies on it: `ZSTD_entropyCost` computes
        // `norm = 256 * count[s] / total` and indexes
        // `kInverseProbabilityLog256[norm]`, a 256-entry table. Any `count[s] >=
        // total` makes `norm >= 256` and the C reads PAST the table (undefined
        // behaviour, silently returning garbage; the Rust panics on the same
        // index). `total == 0` would additionally divide by zero. So nbSeq is
        // always the real sum here, and the single-symbol early-exit branch
        // (`mostFrequent == nbSeq`) is hit by *constructing* single-symbol
        // histograms rather than by lying about nbSeq.
        if total == 0 {
            continue;
        }
        let single_symbol = rng.below(4) == 0;
        if single_symbol {
            // collapse everything onto one symbol so mostFrequent == nbSeq
            for v in count.iter_mut() {
                *v = 0;
            }
            let k = rng.below(n);
            count[k] = 1 + (rng.next_u32() % 2000);
            total = count[k] as usize;
            most = total;
        }
        let nb_seq = total;
        let fselog = rng.range(5, 9) as u32;

        for &policy in &[0i32, 1] {
            for &strat in &ALL_STRATEGIES {
                for &start_mode in &[0i32, 1, 2] {
                    let mut m1 = start_mode;
                    let mut m2 = start_mode;
                    let a = unsafe {
                        c(
                            &mut m1,
                            count.as_ptr(),
                            max,
                            most,
                            nb_seq,
                            fselog,
                            c_ct.as_ptr(),
                            LL_DEFAULT_NORM.as_ptr(),
                            LL_DEFAULTNORMLOG,
                            policy,
                            strat,
                        )
                    };
                    let b = unsafe {
                        r(
                            &mut m2,
                            count.as_ptr(),
                            max,
                            most,
                            nb_seq,
                            fselog,
                            r_ct.as_ptr(),
                            LL_DEFAULT_NORM.as_ptr(),
                            LL_DEFAULTNORMLOG,
                            policy,
                            strat,
                        )
                    };
                    let tag = format!(
                        "selectEncodingType max={max} most={most} nbSeq={nb_seq} FSELog={fselog} \
                         policy={policy} strat={strat} mode={start_mode}"
                    );
                    assert_eq_dbg(&tag, a, b);
                    // the in/out repeatMode write-back must match too
                    assert_eq_dbg(&format!("{tag} / repeatMode out"), m1, m2);
                }
            }
        }
    }
}
