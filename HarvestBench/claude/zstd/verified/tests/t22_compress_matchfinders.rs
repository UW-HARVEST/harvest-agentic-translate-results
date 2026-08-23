//! Phase B — the MATCH-FINDER and BLOCK-SHAPING option matrix.
//!
//! Every `ZSTD_strategy` selects a different match finder (`zstd_fast.c`,
//! `zstd_double_fast.c`, `zstd_lazy.c`, `zstd_opt.c`); every `dictMode`
//! (noDict / extDict / dictMatchState) selects a different *variant* of it; and
//! the row-based finder in `zstd_lazy.c` is a whole separate implementation that
//! is compiled with `ZSTD_ARCH_X86_SSE2` live (`ZSTD_row_getSSEMask`) in this
//! build. On top of the finder sit the block shapers: the pre-splitter
//! (`zstd_preSplit.c`, via `ZSTD_optimalBlockSize`), the post-splitter
//! (`ZSTD_compressBlock_splitBlock`), the superblock encoder
//! (`zstd_compress_superblock.c`, reached only via `ZSTD_c_targetCBlockSize`),
//! and the long-distance matcher (`zstd_ldm.c`).
//!
//! Everything here is driven through `ZSTD_compress2` on a `ZSTD_CCtx` whose
//! parameters were set explicitly with `ZSTD_CCtx_setParameter`. A few rows are
//! *only* reachable with an unknown pledged source size and additionally use
//! `ZSTD_compressStream2`; those are called out where they occur. For every case
//! the suite compares
//!
//!   * every `ZSTD_CCtx_setParameter` return value (many of them echo the value
//!     the library actually stored, e.g. the `targetCBlockSize` clamp),
//!   * the `ZSTD_compress2` return value, and
//!   * the EXACT compressed bytes,
//!
//! and then decompresses the frame with both libraries and requires the plain
//! text back. Byte-exactness is the point: two match finders that merely
//! "compress about as well" are a translation bug.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// Constants from `zstd.h` (the ranges `ZSTD_cParam_getBounds` reports)
// ---------------------------------------------------------------------------

const ZSTD_WINDOWLOG_MIN: c_int = 10;
const ZSTD_WINDOWLOG_MAX: c_int = 31;
const ZSTD_HASHLOG_MIN: c_int = 6;
const ZSTD_HASHLOG_MAX: c_int = 30;
const ZSTD_CHAINLOG_MIN: c_int = 6;
const ZSTD_CHAINLOG_MAX: c_int = 30;
const ZSTD_SEARCHLOG_MIN: c_int = 1;
const ZSTD_SEARCHLOG_MAX: c_int = 30;
const ZSTD_MINMATCH_MIN: c_int = 3;
const ZSTD_MINMATCH_MAX: c_int = 7;
const ZSTD_TARGETLENGTH_MAX: c_int = 131072;
const ZSTD_TARGETCBLOCKSIZE_MIN: c_int = 1340;
const ZSTD_TARGETCBLOCKSIZE_MAX: c_int = 131072;
const ZSTD_BLOCKSIZE_MAX_MIN: c_int = 1024;
const ZSTD_BLOCKSPLITTER_LEVEL_MAX: c_int = 6;
const ZSTD_LDM_HASHLOG_MIN: c_int = 6;
const ZSTD_LDM_MINMATCH_MIN: c_int = 4;
const ZSTD_LDM_MINMATCH_MAX: c_int = 4096;
const ZSTD_LDM_BUCKETSIZELOG_MIN: c_int = 1;
const ZSTD_LDM_BUCKETSIZELOG_MAX: c_int = 8;
const ZSTD_LDM_HASHRATELOG_MAX: c_int = 25;

/// `parameter_unsupported`, the code `ZSTD_c_rsyncable != 0` must return in a
/// build without `ZSTD_MULTITHREAD`.
const E_parameter_unsupported: c_int = 40;
/// `parameter_outOfBound`.
const E_parameter_outOfBound: c_int = 42;

/// Every low-level block compressor `ZSTD_selectBlockCompressor` can dispatch
/// to, plus the shapers and helpers, as named in `SYMBOLS.md` / `CONFIGS.md`.
const BLOCK_COMPRESSORS: &[&str] = &[
    "ZSTD_compressBlock_fast",
    "ZSTD_compressBlock_fast_dictMatchState",
    "ZSTD_compressBlock_fast_extDict",
    "ZSTD_compressBlock_doubleFast",
    "ZSTD_compressBlock_doubleFast_dictMatchState",
    "ZSTD_compressBlock_doubleFast_extDict",
    "ZSTD_compressBlock_greedy",
    "ZSTD_compressBlock_greedy_row",
    "ZSTD_compressBlock_greedy_dictMatchState",
    "ZSTD_compressBlock_greedy_dictMatchState_row",
    "ZSTD_compressBlock_greedy_dedicatedDictSearch",
    "ZSTD_compressBlock_greedy_dedicatedDictSearch_row",
    "ZSTD_compressBlock_greedy_extDict",
    "ZSTD_compressBlock_greedy_extDict_row",
    "ZSTD_compressBlock_lazy",
    "ZSTD_compressBlock_lazy_row",
    "ZSTD_compressBlock_lazy_dictMatchState",
    "ZSTD_compressBlock_lazy_dictMatchState_row",
    "ZSTD_compressBlock_lazy_dedicatedDictSearch",
    "ZSTD_compressBlock_lazy_dedicatedDictSearch_row",
    "ZSTD_compressBlock_lazy_extDict",
    "ZSTD_compressBlock_lazy_extDict_row",
    "ZSTD_compressBlock_lazy2",
    "ZSTD_compressBlock_lazy2_row",
    "ZSTD_compressBlock_lazy2_dictMatchState",
    "ZSTD_compressBlock_lazy2_dictMatchState_row",
    "ZSTD_compressBlock_lazy2_dedicatedDictSearch",
    "ZSTD_compressBlock_lazy2_dedicatedDictSearch_row",
    "ZSTD_compressBlock_lazy2_extDict",
    "ZSTD_compressBlock_lazy2_extDict_row",
    "ZSTD_compressBlock_btlazy2",
    "ZSTD_compressBlock_btlazy2_dictMatchState",
    "ZSTD_compressBlock_btlazy2_extDict",
    "ZSTD_compressBlock_btopt",
    "ZSTD_compressBlock_btopt_dictMatchState",
    "ZSTD_compressBlock_btopt_extDict",
    "ZSTD_compressBlock_btultra",
    "ZSTD_compressBlock_btultra2",
    "ZSTD_compressBlock_btultra_dictMatchState",
    "ZSTD_compressBlock_btultra_extDict",
    "ZSTD_compressBlock_deprecated",
    "ZSTD_selectBlockCompressor",
    "ZSTD_splitBlock",
    "ZSTD_row_update",
    "ZSTD_cycleLog",
    "ZSTD_checkCParams",
];

// ---------------------------------------------------------------------------
// Extra signatures
// ---------------------------------------------------------------------------

/// `size_t ZSTD_checkCParams(ZSTD_compressionParameters)` — the 28-byte struct
/// is MEMORY-class on x86-64 SysV, i.e. passed on the stack.
type FnCheckCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> SizeT;
type FnCycleLog = unsafe extern "C" fn(c_uint, c_int) -> c_uint;
type FnRefPrefix = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnCreateCCtxParams = unsafe extern "C" fn() -> *mut c_void;
type FnParamsSet = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> SizeT;
type FnParamsGet = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> SizeT;

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

/// Full observable result of one parameterised compression: the return value of
/// **every** `ZSTD_CCtx_setParameter` call (several of them echo the stored
/// value, which is how the `targetCBlockSize` / `maxBlockSize` clamps become
/// observable), then the `ZSTD_compress2` status, then the exact output bytes.
type Out = (Vec<R>, R, Blob);

/// Sentinel used in place of a compression status when an earlier
/// `setParameter` already failed, so the two libraries are still compared on the
/// same shape of result.
const NOT_RUN: R = R::Ok(usize::MAX);

fn set_all(l: &Lib, ctx: &Ctx<'_>, params: &[(c_int, c_int)]) -> (Vec<R>, bool) {
    let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    let mut sets = Vec::with_capacity(params.len());
    for (p, v) in params {
        let r = res(l, unsafe { set(ctx.ptr, *p, *v) });
        let failed = matches!(r, R::Err(..));
        sets.push(r);
        if failed {
            return (sets, false);
        }
    }
    (sets, true)
}

fn drive(l: &Lib, params: &[(c_int, c_int)], src: &[u8]) -> Out {
    let ctx = Ctx::cctx(l);
    let (sets, ok) = set_all(l, &ctx, params);
    if !ok {
        return (sets, NOT_RUN, Blob(Vec::new()));
    }
    let cap = compress_bound(l, src.len()).max(64);
    let mut dst = vec![0xCDu8; cap];
    let f = l.sym::<FnCompress2>("ZSTD_compress2");
    let n = unsafe {
        f(
            ctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    let r = res(l, n);
    dst.truncate(if let R::Ok(n) = r { n } else { 0 });
    (sets, r, Blob(dst))
}

/// `ZSTD_compressStream2`, used only for the rows that are unreachable through
/// `ZSTD_compress2`: `ZSTD_CCtx_init_compressStream2` sets
/// `pledgedSrcSizePlusOne = inSize+1` whenever `endOp == ZSTD_e_end`
/// (`zstd_compress.c:6366`), so a one-shot call always knows the source size and
/// therefore (a) always shrinks `windowLog` to `srcLog` in
/// `ZSTD_adjustCParams_internal` and (b) never consults `ZSTD_c_srcSizeHint`
/// (`zstd_compress.c:1641`). Feeding the input with `ZSTD_e_continue` first
/// keeps the pledged size unknown.
fn drive_stream(l: &Lib, params: &[(c_int, c_int)], src: &[u8], chunk: usize) -> Out {
    let ctx = Ctx::cctx(l);
    let (sets, ok) = set_all(l, &ctx, params);
    if !ok {
        return (sets, NOT_RUN, Blob(Vec::new()));
    }
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let cap = compress_bound(l, src.len()).max(64) + 4096;
    let mut out = vec![0xCDu8; cap];
    let mut ob = ZSTD_outBuffer {
        dst: out.as_mut_ptr() as *mut c_void,
        size: cap,
        pos: 0,
    };
    let end = unsafe { src.as_ptr().add(src.len()) } as *const c_void;
    let mut pos = 0usize;
    while pos < src.len() {
        let n = (src.len() - pos).min(chunk.max(1));
        let mut ib = ZSTD_inBuffer {
            src: unsafe { src.as_ptr().add(pos) } as *const c_void,
            size: n,
            pos: 0,
        };
        while ib.pos < ib.size {
            let r = unsafe { f(ctx.ptr, &mut ob, &mut ib, ZSTD_e_continue) };
            if is_error(l, r) {
                out.truncate(ob.pos);
                return (sets, res(l, r), Blob(out));
            }
        }
        pos += n;
    }
    loop {
        let mut ib = ZSTD_inBuffer {
            src: end,
            size: 0,
            pos: 0,
        };
        let r = res(l, unsafe { f(ctx.ptr, &mut ob, &mut ib, ZSTD_e_end) });
        match r {
            R::Ok(0) => break,
            R::Ok(_) => continue,
            R::Err(..) => {
                out.truncate(ob.pos);
                return (sets, r, Blob(out));
            }
        }
    }
    out.truncate(ob.pos);
    (sets, R::Ok(0), Blob(out))
}

/// Decompress with a fresh `ZSTD_DCtx`, optionally after setting dParams.
fn undrive(l: &Lib, dparams: &[(c_int, c_int)], comp: &[u8], cap: usize) -> Out {
    let ctx = Ctx::dctx(l);
    let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    let mut sets = Vec::with_capacity(dparams.len());
    for (p, v) in dparams {
        let r = res(l, unsafe { set(ctx.ptr, *p, *v) });
        let failed = matches!(r, R::Err(..));
        sets.push(r);
        if failed {
            return (sets, NOT_RUN, Blob(Vec::new()));
        }
    }
    let mut dst = vec![0xCDu8; cap.max(1)];
    let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    let n = unsafe {
        f(
            ctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            comp.as_ptr() as *const c_void,
            comp.len(),
        )
    };
    let r = res(l, n);
    dst.truncate(if let R::Ok(n) = r { n } else { 0 });
    (sets, r, Blob(dst))
}

/// Decompress with `ZSTD_decompressStream`. Unlike the one-shot entry point
/// this one honours `ZSTD_d_windowLogMax`: the
/// `fParams.windowSize > maxWindowSize -> frameParameter_windowTooLarge` gate
/// lives in `ZSTD_decompressStream` (`zstd_decompress.c:2231`) only, because the
/// one-shot decoder writes straight into `dst` and never allocates a window.
fn undrive_stream(l: &Lib, dparams: &[(c_int, c_int)], comp: &[u8], cap: usize) -> Out {
    let ctx = Ctx::dstream(l);
    let set = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    let mut sets = Vec::with_capacity(dparams.len());
    for (p, v) in dparams {
        let r = res(l, unsafe { set(ctx.ptr, *p, *v) });
        let failed = matches!(r, R::Err(..));
        sets.push(r);
        if failed {
            return (sets, NOT_RUN, Blob(Vec::new()));
        }
    }
    let mut dst = vec![0xCDu8; cap.max(1)];
    let mut ob = ZSTD_outBuffer {
        dst: dst.as_mut_ptr() as *mut c_void,
        size: cap,
        pos: 0,
    };
    let mut ib = ZSTD_inBuffer {
        src: comp.as_ptr() as *const c_void,
        size: comp.len(),
        pos: 0,
    };
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut last;
    loop {
        let before = (ib.pos, ob.pos);
        last = res(l, unsafe { f(ctx.ptr, &mut ob, &mut ib) });
        match last {
            R::Err(..) => break,
            R::Ok(0) => break,
            R::Ok(_) => {
                if ib.pos == comp.len() && (ib.pos, ob.pos) == before {
                    break; // no progress possible
                }
            }
        }
    }
    dst.truncate(ob.pos);
    (sets, last, Blob(dst))
}

/// Round-trip a frame through both libraries and require the plaintext back.
#[track_caller]
fn rt(label: &str, comp: &[u8], src: &[u8]) {
    let (_, r, plain) = diff_bytes(&format!("{label}|rt"), |l| undrive(l, &[], comp, src.len()));
    match &r {
        R::Ok(n) => assert_eq!(*n, src.len(), "[{label}] round-trip length"),
        e => panic!("[{label}] round-trip failed: {e:?}"),
    }
    assert!(
        plain.0.as_slice() == src,
        "[{label}] round-trip content mismatch (len {})",
        src.len()
    );
}

/// Compare compressed bytes across the two libraries, require success, then
/// round-trip.
#[track_caller]
fn ok(label: &str, params: &[(c_int, c_int)], src: &[u8]) -> Blob {
    let (sets, r, comp) = diff_bytes(label, |l| drive(l, params, src));
    for (i, s) in sets.iter().enumerate() {
        assert!(
            matches!(s, R::Ok(_)),
            "[{label}] ZSTD_CCtx_setParameter{:?} unexpectedly rejected: {s:?}",
            params[i]
        );
    }
    assert!(matches!(r, R::Ok(_)), "[{label}] ZSTD_compress2 failed: {r:?}");
    rt(label, &comp.0, src);
    comp
}

/// Same as [`ok`] but through `ZSTD_compressStream2`.
#[track_caller]
fn ok_stream(label: &str, params: &[(c_int, c_int)], src: &[u8], chunk: usize) -> Blob {
    let (sets, r, comp) = diff_bytes(label, |l| drive_stream(l, params, src, chunk));
    for (i, s) in sets.iter().enumerate() {
        assert!(
            matches!(s, R::Ok(_)),
            "[{label}] ZSTD_CCtx_setParameter{:?} unexpectedly rejected: {s:?}",
            params[i]
        );
    }
    assert!(
        matches!(r, R::Ok(0)),
        "[{label}] ZSTD_compressStream2(e_end) did not finish: {r:?}"
    );
    rt(label, &comp.0, src);
    comp
}

/// Like [`ok`] but tolerating a *matching* failure from both libraries. Some
/// valid parameter combinations make the C compute an out-of-range derived
/// value — most notably `ZSTD_ldm_adjustParameters` deriving
/// `ldmHashLog = BOUNDED(6, windowLog - hashRateLog, 30)` with unsigned
/// arithmetic, so any `ldmHashRateLog > windowLog` underflows and lands on
/// `ldmHashLog = 30`, i.e. an 8 GB hash table and `memory_allocation`. The
/// differential comparison is unchanged (return value *and* bytes); only the
/// "must succeed" assertion is relaxed, and the round-trip runs when it did.
#[track_caller]
fn ok_or_same(label: &str, params: &[(c_int, c_int)], src: &[u8]) -> R {
    let (sets, r, comp) = diff_bytes(label, |l| drive(l, params, src));
    for (i, s) in sets.iter().enumerate() {
        assert!(
            matches!(s, R::Ok(_)),
            "[{label}] ZSTD_CCtx_setParameter{:?} unexpectedly rejected: {s:?}",
            params[i]
        );
    }
    if matches!(r, R::Ok(_)) {
        rt(label, &comp.0, src);
    }
    r
}

/// A parameter value both libraries must refuse with the same error.
#[track_caller]
fn rejected(label: &str, params: &[(c_int, c_int)], code: c_int) {
    let (sets, _, _) = diff_bytes(label, |l| drive(l, params, b"0123456789abcdef"));
    let last = sets.last().unwrap_or_else(|| panic!("[{label}] no params"));
    match last {
        R::Err(c, _) => assert_eq!(*c, code, "[{label}] wrong error code: {last:?}"),
        R::Ok(v) => panic!("[{label}] expected rejection, got Ok({v})"),
    }
}

// ---------------------------------------------------------------------------
// Corpora derived from the shared ones
// ---------------------------------------------------------------------------

/// Concatenate 32 KB runs of *different* corpora. Both block splitters only
/// ever fire when neighbouring regions have visibly different byte
/// distributions, and `ZSTD_optimalBlockSize` additionally requires
/// `savings >= 3` (so a purely incompressible input is never pre-split, and the
/// first full block is never pre-split at all).
fn hetero(len: usize, seed: u64) -> Vec<u8> {
    const KINDS: &[Corpus] = &[
        Corpus::Text,
        Corpus::Random,
        Corpus::Zeros,
        Corpus::SmallAlphabet,
        Corpus::Counter,
        Corpus::Periodic,
        Corpus::LongRepeats,
        Corpus::Sparse,
    ];
    let mut out = Vec::with_capacity(len);
    let mut i = 0usize;
    while out.len() < len {
        let n = (32usize << 10).min(len - out.len());
        out.extend_from_slice(&corpus(KINDS[i % KINDS.len()], n, seed ^ (i as u64 * 7919)));
        i += 1;
    }
    out
}

/// One sharp distribution border at `at`: `ZSTD_splitBlock_fromBorders`
/// compares only the first and the last 512-byte segment of a block, so where
/// the border falls selects between its {blockSize, 65536, 32768, 98304} exits.
fn sharp(len: usize, at: usize, seed: u64) -> Vec<u8> {
    let mut out = corpus(Corpus::Text, at.min(len), seed);
    out.extend_from_slice(&corpus(Corpus::Random, len - out.len(), seed ^ 0xABCD));
    out
}

/// Chunks alternating between two distributions — the geometry
/// `ZSTD_splitBlock_byChunks` (CHUNKSIZE = 8192) samples at levels 1..4.
fn alternating(len: usize, chunk: usize, seed: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0usize;
    while out.len() < len {
        let n = chunk.min(len - out.len());
        let k = if i % 2 == 0 {
            Corpus::Text
        } else {
            Corpus::SmallAlphabet
        };
        out.extend_from_slice(&corpus(k, n, seed ^ (i as u64)));
        i += 1;
    }
    out
}

// ===========================================================================
// 1. The core match-finder dispatch matrix
// ===========================================================================

/// All nine strategies x all ten corpus shapes x nine sizes.
///
/// Targets `ZSTD_selectBlockCompressor(strat, ZSTD_noDict)` — row 0 of
/// `blockCompressor[3][10]` — and hence `ZSTD_compressBlock_fast`,
/// `_doubleFast`, `_greedy(_row)`, `_lazy(_row)`, `_lazy2(_row)`, `_btlazy2`,
/// `_btopt`, `_btultra`, `_btultra2`. The size set straddles every shape
/// boundary the encoder branches on: below `HASH_READ_SIZE` (7/8), the
/// `MIN_CBLOCK_SIZE` "too small to compress" cut-off, the four
/// `ZSTD_defaultCParameters` table rows (16 KB / 128 KB / 256 KB), and the
/// 128 KB block boundary in both directions (131072 / 131073). The corpora pin
/// `ZSTD_compressBlock_internal`'s three exits: RLE (`Zeros`/`OneByte`, but
/// never on the first block, since the shortcut needs `!isFirstBlock`),
/// raw/`ZSTD_noCompressBlock` (`Random`), and compressed.
#[test]
fn t1_strategy_corpus_size_matrix() {
    covers(&[
        "CFG:18", "CFG:19", "CFG:20", "CFG:21", "CFG:22", "CFG:23", "CFG:24", "CFG:25", "CFG:26",
        "CFG:27", "CFG:47", "CFG:48", "CFG:49", "CFG:100",
    ]);
    const SZ: &[usize] = &[7, 8, 100, 1024, 8192, 65536, 131072, 131073, 400_000];
    for &strat in ALL_STRATEGIES {
        for &k in ALL_CORPORA {
            for &n in SZ {
                let src = corpus(k, n, 0x5151 ^ n as u64);
                ok(
                    &format!("strategy={strat} corpus={k:?} size={n}"),
                    &[(ZSTD_c_strategy, strat)],
                    &src,
                );
            }
        }
    }
}

/// The same dispatch matrix with `windowLog` pinned to 17 (the configuration
/// `CONFIGS.md` rows 18/19 name) so the level table's window choice cannot mask
/// a finder difference, plus `targetLength` at `ZSTD_fast` — where
/// `targetLength` is not a match length at all but the
/// `stepSize = targetLength + !targetLength + 1` acceleration of
/// `ZSTD_compressBlock_fast_generic`, which negative compression levels reach
/// through `cp.targetLength = -MAX(ZSTD_minCLevel(), level)`.
#[test]
fn t2_strategy_windowlog17_and_fast_acceleration() {
    covers(&["CFG:11", "CFG:15", "CFG:16", "CFG:18", "CFG:19", "CFG:100"]);
    for &strat in ALL_STRATEGIES {
        for &k in ALL_CORPORA {
            for &n in &[1024usize, 131_072, 400_000] {
                let src = corpus(k, n, 0x7272 ^ n as u64);
                ok(
                    &format!("wlog17 strategy={strat} corpus={k:?} size={n}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, 17)],
                    &src,
                );
            }
        }
    }
    for &tl in &[0, 1, 2, 8, 64, 4096, ZSTD_TARGETLENGTH_MAX] {
        for &k in &[
            Corpus::Text,
            Corpus::LongRepeats,
            Corpus::Random,
            Corpus::Periodic,
        ] {
            let src = corpus(k, 200_000, 0x9191);
            ok(
                &format!("fast targetLength={tl} corpus={k:?}"),
                &[
                    (ZSTD_c_strategy, ZSTD_fast),
                    (ZSTD_c_windowLog, 17),
                    (ZSTD_c_targetLength, tl),
                ],
                &src,
            );
        }
    }
    for &lvl in &[-1, -2, -3, -5, -10, -100, -1000, -131_072, i32::MIN] {
        for &k in &[Corpus::Text, Corpus::Random, Corpus::LongRepeats] {
            let src = corpus(k, 200_000, 0x2828);
            ok(
                &format!("negative level={lvl} corpus={k:?}"),
                &[(ZSTD_c_compressionLevel, lvl)],
                &src,
            );
        }
    }
    // CONFIGS row 16: the sizes around the frame-header / singleSegment
    // decisions, with and without an explicit windowLog=27 (which
    // `ZSTD_adjustCParams_internal` then shrinks all the way back to
    // `ZSTD_WINDOWLOG_ABSOLUTEMIN` for the smallest inputs).
    for &n in &[1usize, 2, 63, 64, 65, 512, 513, 514, 4096, 65536] {
        for &k in &[Corpus::Zeros, Corpus::Random] {
            for &lvl in &[1, 19] {
                let src = corpus(k, n, 0x2829);
                ok(
                    &format!("hdrsize level={lvl} corpus={k:?} size={n}"),
                    &[(ZSTD_c_compressionLevel, lvl)],
                    &src,
                );
                ok(
                    &format!("hdrsize level={lvl} wlog27 corpus={k:?} size={n}"),
                    &[(ZSTD_c_compressionLevel, lvl), (ZSTD_c_windowLog, 27)],
                    &src,
                );
            }
        }
    }
    // CONFIGS row 47: incompressible input forces the `cSize == 0` arm of
    // `ZSTD_compressBlock_internal` / `ZSTD_entropyCompressSeqStore`
    // (`cSize >= maxCSize` with `maxCSize = blockSize - ZSTD_minGain(blockSize,
    // strategy)`, and `ZSTD_minGain`'s `minlog` differs for btultra+), hence
    // `ZSTD_noCompressBlock` and bt_raw blocks.
    for &n in &[100usize, 131_072, 300_000, 600_000] {
        for &lvl in &[-5, 1, 3, 9, 19, 22] {
            let src = corpus(Corpus::Random, n, 0x282A);
            ok(
                &format!("incompressible level={lvl} size={n}"),
                &[(ZSTD_c_compressionLevel, lvl)],
                &src,
            );
        }
    }
    // CONFIGS row 48: the RLE shortcut needs `frame && !isFirstBlock &&
    // cSize < rleMaxLength(25) && ZSTD_isRLE(...)`, so a multi-block
    // all-one-byte input mixes bt_compressed (first block) with bt_rle.
    for &n in &[26usize, 27, 100, 131_072, 262_144, 400_000] {
        for &lvl in &[1, 3, 9, 19] {
            for &k in &[Corpus::Zeros, Corpus::OneByte] {
                let src = corpus(k, n, 0x282B);
                ok(
                    &format!("rle level={lvl} corpus={k:?} size={n}"),
                    &[(ZSTD_c_compressionLevel, lvl)],
                    &src,
                );
            }
        }
    }
}

// ===========================================================================
// 2. The row-based match finder
// ===========================================================================

/// `ZSTD_c_useRowMatchFinder` x the three strategies that have a row variant
/// x `windowLog` on both sides of the auto threshold.
///
/// `ZSTD_resolveRowMatchFinderMode` (`zstd_compress.c:238`) passes a non-auto
/// value straight through — `ps_enable` is honoured even where the strategy has
/// no row variant — and for `ps_auto` returns `ps_enable` iff
/// `ZSTD_rowMatchFinderSupported(strategy)` (greedy/lazy/lazy2 only) **and**
/// `windowLog > 14`. So `windowLog` 14 vs 15 flips the finder for the same
/// strategy. Inputs of 300 KB fill the tag table so the SSE2
/// `ZSTD_row_getSSEMask` compare and the `ZSTD_row_update` hash-cache path
/// really run; `windowLog=10` on 300 KB additionally slides the window, which
/// switches the row finder to its `_extDict_row` variant.
#[test]
fn t3_row_match_finder() {
    covers(&[
        "CFG:20", "CFG:21", "CFG:22", "CFG:23", "CFG:35", "CFG:36", "CFG:137",
    ]);
    for &row in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &strat in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
            for &wlog in &[10, 14, 15, 17, 20] {
                for &k in &[Corpus::Text, Corpus::LongRepeats, Corpus::SmallAlphabet] {
                    for &n in &[1024usize, 300_000] {
                        let src = corpus(k, n, 0x1234 ^ n as u64);
                        ok(
                            &format!(
                                "row={row} strategy={strat} wlog={wlog} corpus={k:?} size={n}"
                            ),
                            &[
                                (ZSTD_c_strategy, strat),
                                (ZSTD_c_windowLog, wlog),
                                (ZSTD_c_useRowMatchFinder, row),
                            ],
                            &src,
                        );
                    }
                }
            }
        }
    }
    // `ps_enable` on strategies that have no row variant at all
    // (`ZSTD_rowMatchFinderUsed` gates on `ZSTD_rowMatchFinderSupported`, so
    // this must be a no-op for the finder but still suppresses the chainTable
    // for `ZSTD_allocateChainTable`). CONFIGS row 137 names fast and btopt.
    for &row in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra2] {
            let src = corpus(Corpus::Text, 512 << 10, 0x4321);
            ok(
                &format!("row={row} unsupported strategy={strat} wlog=20"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, 20),
                    (ZSTD_c_useRowMatchFinder, row),
                ],
                &src,
            );
        }
    }
    // `rowLog = BOUNDED(4, searchLog, 6)` selects 16- / 32- / 64-entry rows in
    // `ZSTD_row_prefetch` / `ZSTD_row_update` / `ZSTD_RowFindBestMatch`, and
    // `ZSTD_adjustCParams_internal` then clamps hashLog to
    // `(32 - ZSTD_ROW_HASH_TAG_BITS) + rowLog`.
    for &strat in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
        for &slog in &[1, 2, 3, 4, 5, 6, 7, 10, ZSTD_SEARCHLOG_MAX] {
            for &row in &[ZSTD_ps_enable, ZSTD_ps_disable] {
                let src = corpus(Corpus::Text, 300_000, 0x8642);
                ok(
                    &format!("rowLog strategy={strat} searchLog={slog} row={row}"),
                    &[
                        (ZSTD_c_strategy, strat),
                        (ZSTD_c_windowLog, 20),
                        (ZSTD_c_searchLog, slog),
                        (ZSTD_c_useRowMatchFinder, row),
                    ],
                    &src,
                );
            }
        }
        for &hlog in &[ZSTD_HASHLOG_MIN, 12, 24, ZSTD_HASHLOG_MAX] {
            let src = corpus(Corpus::LongRepeats, 300_000, 0x1357);
            ok(
                &format!("rowHash strategy={strat} hashLog={hlog}"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, 20),
                    (ZSTD_c_hashLog, hlog),
                    (ZSTD_c_useRowMatchFinder, ZSTD_ps_enable),
                ],
                &src,
            );
        }
    }
    // The row finder also changes the *workspace layout*: it suppresses the
    // chainTable (`ZSTD_allocateChainTable`) and adds an `hSize`-byte tagTable,
    // so `ZSTD_sizeof_CCtx` after the compression is a second, independent
    // observation of `ZSTD_resolveRowMatchFinderMode`'s answer (CONFIGS row 36).
    for &row in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &strat in ALL_STRATEGIES {
            for &wlog in &[14, 15, 20] {
                for &slog in &[3, 5, 6] {
                    diff(
                        &format!("sizeof_CCtx row={row} strategy={strat} wlog={wlog} slog={slog}"),
                        |l| {
                            let ctx = Ctx::cctx(l);
                            let (sets, okk) = set_all(
                                l,
                                &ctx,
                                &[
                                    (ZSTD_c_strategy, strat),
                                    (ZSTD_c_windowLog, wlog),
                                    (ZSTD_c_searchLog, slog),
                                    (ZSTD_c_useRowMatchFinder, row),
                                ],
                            );
                            assert!(okk, "{sets:?}");
                            let src = corpus(Corpus::Text, 300_000, 0x2468);
                            let cap = compress_bound(l, src.len());
                            let mut dst = vec![0u8; cap];
                            let f = l.sym::<FnCompress2>("ZSTD_compress2");
                            let n = unsafe {
                                f(
                                    ctx.ptr,
                                    dst.as_mut_ptr() as *mut c_void,
                                    cap,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                )
                            };
                            let sz = l.sym::<FnFreeCCtx>("ZSTD_sizeof_CCtx");
                            (res(l, n), unsafe { sz(ctx.ptr) })
                        },
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 3. Window sliding / extDict / index rebasing
// ===========================================================================

/// `windowLog` far below the input size, so `ZSTD_window_update` reports a
/// non-contiguous window and `dictLimit`/`lowLimit` diverge: every finder then
/// switches to its `_extDict` variant, and `ZSTD_overflowCorrectIfNeeded` /
/// `ZSTD_reduceIndex` rebase the tables.
///
/// `windowLog` 27 and 31 are included even though
/// `ZSTD_adjustCParams_internal` shrinks them to `srcLog` for these sizes —
/// that clamp is itself under test (a translation that forgot it would try to
/// allocate a 2 GB window).
#[test]
fn t4_window_sliding_extdict() {
    covers(&["CFG:15", "CFG:16", "CFG:28", "CFG:100"]);
    for &wlog in &[
        ZSTD_WINDOWLOG_MIN,
        11,
        15,
        17,
        20,
        23,
        27,
        ZSTD_WINDOWLOG_MAX,
    ] {
        for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_lazy2] {
            for &k in &[Corpus::LongRepeats, Corpus::Text] {
                let src = corpus(k, 1 << 20, 0xB0B0);
                ok(
                    &format!("slide wlog={wlog} strategy={strat} corpus={k:?} size=1MB"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, wlog)],
                    &src,
                );
            }
        }
    }
    // Several times the window at multi-megabyte sizes, where the 32-bit index
    // space is genuinely re-based more than once.
    for &(wlog, strat) in &[
        (10, ZSTD_fast),
        (11, ZSTD_dfast),
        (15, ZSTD_greedy),
        (20, ZSTD_lazy2),
        (23, ZSTD_dfast),
    ] {
        for &k in &[Corpus::LongRepeats, Corpus::Text] {
            let src = corpus(k, 6 << 20, 0xC0C0);
            ok(
                &format!("slide6MB wlog={wlog} strategy={strat} corpus={k:?}"),
                &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, wlog)],
                &src,
            );
        }
    }
    // extDict for the binary-tree strategies. `ZSTD_selectBlockCompressor` maps
    // BOTH btultra and btultra2 onto `ZSTD_compressBlock_btultra_extDict`, so
    // strategies 8 and 9 must agree from the second block onwards.
    for &strat in &[ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2] {
        for &wlog in &[10, 11] {
            for &k in &[Corpus::LongRepeats, Corpus::Text] {
                let src = corpus(k, 262_144, 0xD0D0);
                ok(
                    &format!("bt-extDict wlog={wlog} strategy={strat} corpus={k:?}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, wlog)],
                    &src,
                );
            }
        }
    }
}

/// The same window geometry through `ZSTD_compressStream2` in small chunks:
/// with the pledged source size unknown `ZSTD_adjustCParams_internal` does not
/// shrink `windowLog`, and the input reaches `ZSTD_compress_frameChunk` in
/// pieces, so the block boundaries — and hence the `extDict` transitions — fall
/// elsewhere than in the one-shot case. Decoding is additionally checked
/// against `ZSTD_d_windowLogMax` at the exact window and one below it (which
/// must be refused identically by both libraries).
#[test]
fn t5_window_sliding_streaming() {
    covers(&["CFG:28", "CFG:29", "CFG:30", "CFG:31", "CFG:67"]);
    for &wlog in &[10, 11, 14, 15, 17] {
        for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
            for &k in &[Corpus::LongRepeats, Corpus::Text] {
                let src = corpus(k, 262_144, 0xE0E0);
                ok_stream(
                    &format!("stream wlog={wlog} strategy={strat} corpus={k:?}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, wlog)],
                    &src,
                    16 << 10,
                );
            }
        }
    }
    for &strat in &[ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2] {
        for &k in &[Corpus::LongRepeats, Corpus::Text] {
            let src = corpus(k, 262_144, 0xE1E1);
            ok_stream(
                &format!("stream bt wlog=11 strategy={strat} corpus={k:?}"),
                &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, 11)],
                &src,
                16 << 10,
            );
        }
    }
    for &wlog in &[10, 11, 17] {
        let src = corpus(Corpus::LongRepeats, 1 << 20, 0xE2E2);
        let label = format!("stream4k wlog={wlog}");
        let comp = ok_stream(&label, &[(ZSTD_c_windowLog, wlog)], &src, 4096);
        // `ZSTD_d_windowLogMax` is itself bounded below by
        // ZSTD_WINDOWLOG_ABSOLUTEMIN, so `wlog-1` is only a legal *value* when
        // wlog > 10; 0 means "the ZSTD_WINDOWLOG_LIMIT_DEFAULT of 27".
        let mut wmaxes = vec![0, wlog];
        if wlog - 1 >= ZSTD_WINDOWLOG_MIN {
            wmaxes.push(wlog - 1);
        }
        for wmax in wmaxes {
            // One-shot: `ZSTD_decompressDCtx` writes straight into `dst` and
            // never allocates a window, so it ignores windowLogMax entirely.
            let (_, r1, plain1) = diff_bytes(&format!("{label}|oneshot wmax={wmax}"), |l| {
                undrive(l, &[(ZSTD_d_windowLogMax, wmax)], &comp.0, src.len())
            });
            assert!(matches!(r1, R::Ok(_)), "[{label}] one-shot wmax={wmax}: {r1:?}");
            assert!(plain1.0.as_slice() == src.as_slice());
            // Streaming: here the gate is live.
            let (_, r2, plain2) = diff_bytes(&format!("{label}|stream wmax={wmax}"), |l| {
                undrive_stream(l, &[(ZSTD_d_windowLogMax, wmax)], &comp.0, src.len())
            });
            if wmax != 0 && wmax < wlog {
                assert!(
                    matches!(r2, R::Err(..)),
                    "[{label}] streaming windowLogMax={wmax} < frame windowLog={wlog} must be refused, got {r2:?}"
                );
            } else {
                assert!(matches!(r2, R::Ok(0)), "[{label}] streaming wmax={wmax}: {r2:?}");
                assert!(plain2.0.as_slice() == src.as_slice());
            }
        }
    }
}

// ===========================================================================
// 4. Long-distance matching
// ===========================================================================

/// `ZSTD_c_enableLongDistanceMatching` and the four `ldm*` knobs.
///
/// `ZSTD_resolveEnableLdm` (`zstd_compress.c:269`) only auto-enables LDM when
/// `strategy >= btopt && windowLog >= 27`; `ZSTD_ldm_adjustParameters`
/// (`zstd_ldm.c:135`) then derives whichever of `hashRateLog` / `hashLog` /
/// `minMatchLength` / `bucketSizeLog` were left at 0, in that order and with
/// cross-dependencies (`hashRateLog` from `hashLog` or from the strategy,
/// `hashLog` from `windowLog - hashRateLog`, `bucketSizeLog` finally clamped by
/// `MIN(bucketSizeLog, hashLog)`), so a one-at-a-time sweep and a joint sweep
/// exercise different code. Also note that enabling LDM forces
/// `cParams.windowLog = ZSTD_LDM_DEFAULT_WINDOW_LOG` (27) *before*
/// `ZSTD_overrideCParams`, so an explicit `windowLog` still wins.
/// The input has 64 KB-scale duplication far beyond any window used here.
#[test]
fn t6_long_distance_matching() {
    covers(&["CFG:57", "CFG:58", "CFG:59"]);
    let src = corpus(Corpus::LongRepeats, 2 << 20, 0x1D11);

    // (a) CONFIGS row 57: enable with every ldm param left at 0 (auto).
    for &lvl in &[1, 3, 9, 19] {
        for &n in &[131_072usize, 1 << 20] {
            let s = corpus(Corpus::LongRepeats, n, 0x1D12);
            ok(
                &format!("ldm auto-params level={lvl} size={n}"),
                &[
                    (ZSTD_c_compressionLevel, lvl),
                    (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                ],
                &s,
            );
        }
    }
    let big = corpus(Corpus::LongRepeats, 4 << 20, 0x1D13);
    for &lvl in &[1, 19] {
        ok(
            &format!("ldm auto-params level={lvl} size=4MB"),
            &[
                (ZSTD_c_compressionLevel, lvl),
                (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            ],
            &big,
        );
    }

    // (b) CONFIGS row 58: one knob at a time, at a fast and a slow strategy.
    let sweeps: &[(&str, c_int, &[c_int])] = &[
        ("ldmHashLog", ZSTD_c_ldmHashLog, &[0, ZSTD_LDM_HASHLOG_MIN, 10, 20]),
        (
            "ldmMinMatch",
            ZSTD_c_ldmMinMatch,
            &[0, ZSTD_LDM_MINMATCH_MIN, 32, 64, ZSTD_LDM_MINMATCH_MAX],
        ),
        (
            "ldmBucketSizeLog",
            ZSTD_c_ldmBucketSizeLog,
            &[0, ZSTD_LDM_BUCKETSIZELOG_MIN, 3, ZSTD_LDM_BUCKETSIZELOG_MAX],
        ),
        // ldmHashRateLog is swept separately: values above the (already
        // srcSize-clamped) windowLog underflow the derived ldmHashLog.
        ("ldmHashRateLog", ZSTD_c_ldmHashRateLog, &[0, 1, 4, 7]),
    ];
    for &lvl in &[1, 19] {
        for (name, p, vals) in sweeps {
            for &v in *vals {
                ok(
                    &format!("ldm {name}={v} level={lvl}"),
                    &[
                        (ZSTD_c_compressionLevel, lvl),
                        (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                        (*p, v),
                    ],
                    &src,
                );
            }
            // The whole ldmHashRateLog range, with ldmHashLog pinned so that
            // `ZSTD_ldm_adjustParameters` does not derive it (and so cannot
            // underflow): here hashRateLog is purely the sampling step of
            // `ZSTD_ldm_generateSequences`.
            if *p == ZSTD_c_ldmHashRateLog {
                for &v in &[0, 1, 4, 7, 8, 16, 20, ZSTD_LDM_HASHRATELOG_MAX] {
                    ok(
                        &format!("ldm hashRateLog={v} hashLog=20 level={lvl}"),
                        &[
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                            (ZSTD_c_ldmHashLog, 20),
                            (ZSTD_c_ldmHashRateLog, v),
                        ],
                        &src,
                    );
                }
            }
        }
    }
    // `ZSTD_ldm_adjustParameters` (zstd_ldm.c:155) computes
    // `hashLog = BOUNDED(6, windowLog - hashRateLog, 30)` in U32 arithmetic.
    // For a 2 MB source `ZSTD_adjustCParams_internal` has already reduced
    // windowLog to 21, so hashRateLog 22..25 underflows and the clamp lands on
    // 30 — an 8 GB LDM hash table. Both libraries must reproduce that exactly
    // (on this host both report `memory_allocation`); this is the reference C's
    // documented-by-code behaviour, not a rejection site, so it is compared
    // rather than asserted to succeed.
    for &v in &[22, 25] {
        ok_or_same(
            &format!("ldm hashRateLog={v} derived-hashLog underflow"),
            &[
                (ZSTD_c_compressionLevel, 1),
                (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                (ZSTD_c_ldmHashRateLog, v),
            ],
            &src,
        );
    }

    // (c) A pruned joint cross-product (fixed seed) over all five knobs.
    let mut rng = Rng::new(0x1D_C0FFEE);
    let enable = [ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable];
    let hashlog = [0, ZSTD_LDM_HASHLOG_MIN, 10, 20];
    let minmatch = [0, 4, 32, 64, 4096];
    let bucket = [0, 1, 3, 8];
    let rate = [0, 4, ZSTD_LDM_HASHRATELOG_MAX];
    let mut seen = std::collections::HashSet::new();
    let mut n_done = 0;
    while n_done < 60 {
        let e = *rng.pick(&enable);
        let hl = *rng.pick(&hashlog);
        let mm = *rng.pick(&minmatch);
        let bs = *rng.pick(&bucket);
        let hr = *rng.pick(&rate);
        let strat = *rng.pick(ALL_STRATEGIES);
        if !seen.insert((e, hl, mm, bs, hr, strat)) {
            continue;
        }
        n_done += 1;
        // `ok_or_same` rather than `ok`: the draw can pair `ldmHashRateLog=25`
        // with an unset `ldmHashLog`, which underflows as described above.
        ok_or_same(
            &format!(
                "ldm joint enable={e} hashLog={hl} minMatch={mm} bucket={bs} rate={hr} strategy={strat}"
            ),
            &[
                (ZSTD_c_strategy, strat),
                (ZSTD_c_enableLongDistanceMatching, e),
                (ZSTD_c_ldmHashLog, hl),
                (ZSTD_c_ldmMinMatch, mm),
                (ZSTD_c_ldmBucketSizeLog, bs),
                (ZSTD_c_ldmHashRateLog, hr),
            ],
            &src,
        );
    }

    // (d) CONFIGS row 59: level 22 (windowLog 27 from the level table) with and
    // without LDM explicitly disabled.
    for &n in &[1usize << 20, 4 << 20] {
        let s = corpus(Corpus::LongRepeats, n, 0x1D14);
        for &e in &[ZSTD_ps_auto, ZSTD_ps_disable] {
            ok(
                &format!("ldm level22 enable={e} size={n}"),
                &[(ZSTD_c_compressionLevel, 22), (ZSTD_c_enableLongDistanceMatching, e)],
                &s,
            );
        }
    }

    // (e) The largest LDM hash tables actually usable here.
    // `ZSTD_ldm_getTableSize` is `(1 << ldmHashLog) * sizeof(ldmEntry_t)` and is
    // NOT clamped by `windowLog` anywhere, so ldmHashLog 24/26/28 ask for
    // 128 MB / 512 MB / 2 GB of hash table (all of it memset in
    // `ZSTD_resetCCtx_internal`); one case each.
    let small = corpus(Corpus::LongRepeats, 256 << 10, 0x1D15);
    for &hl in &[24, 26, 28] {
        ok(
            &format!("ldm huge hashLog={hl}"),
            &[
                (ZSTD_c_compressionLevel, 1),
                (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                (ZSTD_c_ldmHashLog, hl),
            ],
            &small,
        );
    }
    // The upper bound `ZSTD_LDM_HASHLOG_MAX` (== ZSTD_HASHLOG_MAX == 30) is
    // pinned through the setter's echo only. Actually compressing with it would
    // request an 8 GB LDM hash table, i.e. the outcome would depend on how much
    // memory the host happens to have free rather than on the translation.
    diff("ldmHashLog bound echoes", |l| {
        let ctx = Ctx::cctx(l);
        let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        let mut v = Vec::new();
        for hl in [0, ZSTD_LDM_HASHLOG_MIN - 1, ZSTD_LDM_HASHLOG_MIN, 29, 30, 31] {
            v.push(res(l, unsafe { set(ctx.ptr, ZSTD_c_ldmHashLog, hl) }));
        }
        v
    });
}

/// The `ZSTD_resolveEnableLdm` *auto* rule. It is evaluated on the already
/// adjusted cParams, and `ZSTD_adjustCParams_internal` shrinks `windowLog` to
/// `srcLog` whenever the source size is known — so `strategy >= btopt &&
/// windowLog >= 27` is unreachable through `ZSTD_compress2` for any input
/// smaller than 64 MB. Streaming with `ZSTD_e_continue` keeps the pledged size
/// unknown, which keeps `windowLog` at the requested value; `windowLog` 26 vs
/// 27 then flips LDM on for the same strategy.
#[test]
fn t7_ldm_auto_rule_and_streaming() {
    covers(&["CFG:57", "CFG:59", "CFG:60"]);
    let src = corpus(Corpus::LongRepeats, 256 << 10, 0x1D20);
    for &strat in &[ZSTD_lazy2, ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2] {
        for &wlog in &[26, 27] {
            ok_stream(
                &format!("ldm auto strategy={strat} wlog={wlog}"),
                &[(ZSTD_c_strategy, strat), (ZSTD_c_windowLog, wlog)],
                &src,
                64 << 10,
            );
        }
    }
    // CONFIGS row 60: 4 MB of long-range duplication in 128 KB chunks, and the
    // same with the window forced small so the LDM window itself slides.
    let big = corpus(Corpus::LongRepeats, 4 << 20, 0x1D21);
    ok_stream(
        "ldm stream 4MB",
        &[(ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable)],
        &big,
        128 << 10,
    );
    ok_stream(
        "ldm stream 4MB wlog20",
        &[
            (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            (ZSTD_c_windowLog, 20),
        ],
        &big,
        128 << 10,
    );
}

// ===========================================================================
// 5. targetCBlockSize — the superblock encoder
// ===========================================================================

/// `ZSTD_useTargetCBlockSize(params)` is just `targetCBlockSize != 0`, and it
/// swaps `ZSTD_compress_frameChunk`'s per-block call from
/// `ZSTD_compressBlock_internal` to `ZSTD_compressBlock_targetCBlockSize` ->
/// `ZSTD_compressSuperBlock` (`zstd_compress_superblock.c`), an entirely
/// separate block encoder that re-emits the literal and sequence sections in
/// sub-blocks. `ZSTD_compressBlock_targetCBlockSize_body` has three exits: the
/// RLE shortcut, the "superblock was profitable" test
/// `cSize != 0 && cSize < maxCSize + ZSTD_blockHeaderSize`, and the
/// `ZSTD_noCompressBlock` fallback. Note the setter clamps *up*:
/// `value = MAX(value, ZSTD_TARGETCBLOCKSIZE_MIN)`, so 1 and 1339 become 1340
/// and only values above the maximum are rejected.
#[test]
fn t8_target_cblock_size_superblock() {
    covers(&["CFG:52", "CFG:53"]);
    for &tcb in &[
        0,
        ZSTD_TARGETCBLOCKSIZE_MIN,
        2048,
        8192,
        65536,
        ZSTD_TARGETCBLOCKSIZE_MAX,
    ] {
        for &strat in &[ZSTD_fast, ZSTD_greedy, ZSTD_lazy2, ZSTD_btultra2] {
            for &k in &[
                Corpus::Text,
                Corpus::Random,
                Corpus::Zeros,
                Corpus::Mixed,
                Corpus::Sparse,
            ] {
                let src = corpus(k, 131_072, 0x7C00);
                ok(
                    &format!("tcb={tcb} strategy={strat} corpus={k:?} size=131072"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_targetCBlockSize, tcb)],
                    &src,
                );
            }
        }
    }
    for &tcb in &[ZSTD_TARGETCBLOCKSIZE_MIN, 8192] {
        for &strat in &[ZSTD_fast, ZSTD_btopt] {
            for &k in &[
                Corpus::Text,
                Corpus::Random,
                Corpus::Zeros,
                Corpus::OneByte,
                Corpus::LongRepeats,
                Corpus::Counter,
            ] {
                for &n in &[6usize, 1000, 262_144, 400_000] {
                    let src = corpus(k, n, 0x7C01);
                    let label = format!("tcb={tcb} strategy={strat} corpus={k:?} size={n}");
                    let comp = ok(
                        &label,
                        &[(ZSTD_c_strategy, strat), (ZSTD_c_targetCBlockSize, tcb)],
                        &src,
                    );
                    // The superblock encoder re-writes the block headers, so
                    // `ZSTD_findFrameCompressedSize` walking them back is a
                    // second, independent check on the shape it produced
                    // (CONFIGS row 52 names it explicitly).
                    let ffcs = diff(&format!("{label}|findFrameCompressedSize"), |l| {
                        let f = l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize");
                        res(l, unsafe {
                            f(comp.0.as_ptr() as *const c_void, comp.0.len())
                        })
                    });
                    assert_eq!(
                        ffcs,
                        R::Ok(comp.0.len()),
                        "[{label}] findFrameCompressedSize must equal the frame length"
                    );
                }
            }
        }
    }
    // The clamp and the upper bound.
    for &v in &[1, 2, 1339] {
        let src = corpus(Corpus::Text, 200_000, 0x7C02);
        let a = ok(
            &format!("tcb={v} (clamps up to 1340)"),
            &[(ZSTD_c_targetCBlockSize, v)],
            &src,
        );
        let b = ok(
            "tcb=1340 reference",
            &[(ZSTD_c_targetCBlockSize, ZSTD_TARGETCBLOCKSIZE_MIN)],
            &src,
        );
        assert_eq!(a, b, "targetCBlockSize={v} must clamp up to 1340");
    }
    rejected(
        "tcb=131073 (above ZSTD_TARGETCBLOCKSIZE_MAX)",
        &[(ZSTD_c_targetCBlockSize, ZSTD_TARGETCBLOCKSIZE_MAX + 1)],
        E_parameter_outOfBound,
    );
    // Combined with the two block splitters, which the superblock encoder must
    // cooperate with rather than replace.
    let het = hetero(512 << 10, 0x7C03);
    for &split in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &lvl in &[0, 4] {
            ok(
                &format!("tcb=2048 splitAfterSequences={split} blockSplitterLevel={lvl}"),
                &[
                    (ZSTD_c_compressionLevel, 19),
                    (ZSTD_c_targetCBlockSize, 2048),
                    (ZSTD_c_splitAfterSequences, split),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &het,
            );
        }
    }
    // Combined with `maxBlockSize`, including the case where the target
    // compressed block size is *larger* than the uncompressed block size (so
    // `ZSTD_compressSuperBlock` never has to split at all) and the case where it
    // is much smaller.
    for &mbs in &[ZSTD_BLOCKSIZE_MAX_MIN, 4096, 65536] {
        for &tcb in &[ZSTD_TARGETCBLOCKSIZE_MIN, 8192, ZSTD_TARGETCBLOCKSIZE_MAX] {
            for &k in &[Corpus::Text, Corpus::Random, Corpus::Zeros] {
                let src = corpus(k, 262_144, 0x7C04);
                ok(
                    &format!("tcb={tcb} maxBlockSize={mbs} corpus={k:?}"),
                    &[
                        (ZSTD_c_maxBlockSize, mbs),
                        (ZSTD_c_targetCBlockSize, tcb),
                        (ZSTD_c_compressionLevel, 5),
                    ],
                    &src,
                );
            }
        }
    }
    // And through the streaming front end, where the superblock encoder sees
    // partial blocks handed over by `ZSTD_compressStream2`'s buffering.
    for &tcb in &[ZSTD_TARGETCBLOCKSIZE_MIN, 8192] {
        for &k in &[Corpus::Text, Corpus::Random, Corpus::Mixed] {
            let src = corpus(k, 262_144, 0x7C05);
            ok_stream(
                &format!("tcb={tcb} streaming corpus={k:?}"),
                &[(ZSTD_c_targetCBlockSize, tcb), (ZSTD_c_compressionLevel, 5)],
                &src,
                20_000,
            );
        }
    }
}

// ===========================================================================
// 6. Literal compression mode
// ===========================================================================

/// `ZSTD_c_literalCompressionMode` and the size thresholds around it.
///
/// `ZSTD_literalsCompressionIsDisabled` resolves `lcm_auto` to "disabled" iff
/// the compression level is negative, so a negative level with `lcm_huffman`
/// and a positive level with `lcm_uncompressed` are both meaningful. Inside
/// `ZSTD_compressLiterals` the interesting boundaries are
/// `ZSTD_minLiteralsToCompress(strategy, huf_repeat)`
/// `= (repeat_valid ? 6 : 8 << MIN(9-strategy, 3))`, i.e. 64 bytes up to
/// btlazy2, then 32 / 16 / 8 for btopt / btultra / btultra2 (6 once a previous
/// block left a reusable table), and `singleStream = litSize < 256`, which
/// decides 1-stream vs 4-stream Huffman. `Random` makes Huffman unprofitable
/// (`set_basic`), `Zeros`/`OneByte` produce `set_rle` literals.
#[test]
fn t9_literal_compression_mode() {
    covers(&["CFG:11", "CFG:49", "CFG:50", "CFG:129"]);
    const LCM: &[c_int] = &[ZSTD_lcm_auto, ZSTD_lcm_huffman, ZSTD_lcm_uncompressed];
    for &lcm in LCM {
        for &lvl in &[-5, -1, 1, 3, 19] {
            for &k in &[
                Corpus::Random,
                Corpus::Zeros,
                Corpus::OneByte,
                Corpus::Text,
                Corpus::Sparse,
                Corpus::SmallAlphabet,
            ] {
                for &n in &[5usize, 6, 7, 8, 16, 32, 64, 65, 255, 256, 257, 1024, 65536] {
                    let src = corpus(k, n, 0x11CE ^ n as u64);
                    ok(
                        &format!("lcm={lcm} level={lvl} corpus={k:?} size={n}"),
                        &[
                            (ZSTD_c_compressionLevel, lvl),
                            (ZSTD_c_literalCompressionMode, lcm),
                        ],
                        &src,
                    );
                }
            }
        }
    }
    // The per-strategy `ZSTD_minLiteralsToCompress` shift, right around the
    // threshold each strategy uses.
    for &lcm in LCM {
        for &strat in &[ZSTD_fast, ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2] {
            for &k in &[Corpus::Random, Corpus::Text, Corpus::Periodic] {
                for &n in &[
                    5usize, 6, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 255, 256, 257,
                ] {
                    let src = corpus(k, n, 0x11CF ^ n as u64);
                    ok(
                        &format!("minLit lcm={lcm} strategy={strat} corpus={k:?} size={n}"),
                        &[
                            (ZSTD_c_strategy, strat),
                            (ZSTD_c_literalCompressionMode, lcm),
                        ],
                        &src,
                    );
                }
            }
        }
    }
    // Multi-block inputs, where the second block's literals may reuse the first
    // block's Huffman table (`huf_repeat == HUF_repeat_valid`, threshold 6, and
    // `singleStream = 1` whenever `lhSize == 3`).
    for &lcm in LCM {
        for &strat in &[ZSTD_fast, ZSTD_lazy2, ZSTD_btultra2] {
            for &k in &[Corpus::Text, Corpus::Random, Corpus::Mixed, Corpus::Sparse] {
                for &n in &[131_073usize, 200_000, 400_000] {
                    let src = corpus(k, n, 0x11D0);
                    ok(
                        &format!("litRepeat lcm={lcm} strategy={strat} corpus={k:?} size={n}"),
                        &[
                            (ZSTD_c_strategy, strat),
                            (ZSTD_c_literalCompressionMode, lcm),
                        ],
                        &src,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 7. The two block splitters
// ===========================================================================

/// `ZSTD_c_blockSplitterLevel` (the *pre*-splitter, `zstd_preSplit.c`) and
/// `ZSTD_c_splitAfterSequences` (the *post*-splitter,
/// `ZSTD_compressBlock_splitBlock` -> `ZSTD_deriveBlockSplits`).
///
/// `ZSTD_optimalBlockSize` (`zstd_compress.c:4552`) returns
/// `MIN(srcSize, blockSizeMax)` while `srcSize < 128 KB || blockSizeMax < 128
/// KB`, returns 128 KB while `savings < 3` (so incompressible data is never
/// pre-split and the FIRST full block is never pre-split), returns 128 KB for
/// `splitLevel == 1`, maps `splitLevel == 0` through
/// `splitLevels[strategy] = {0,0,1,2,2,3,3,4,4,4}`, and otherwise calls
/// `ZSTD_splitBlock(src, blockSizeMax, splitLevel-2, ...)`. Internal level 0 is
/// `ZSTD_splitBlock_fromBorders` (first/last 512-byte segment only); levels 1..4
/// are `ZSTD_splitBlock_byChunks` with sampling rates {43,11,5,1} and hashLogs
/// {8,9,10,10}. `ZSTD_resolveBlockSplitterMode` auto-enables the post-splitter
/// iff `strategy >= btopt && windowLog >= 17`.
#[test]
fn t10_block_splitters() {
    covers(&[
        "CFG:54", "CFG:55", "CFG:56", "CFG:136", "CFG:147", "CFG:148", "CFG:149", "CFG:150",
    ]);
    // (a) The pre-splitter across its whole level range, on data with real
    // borders. > 256 KB, because the first full block never splits.
    let het512 = hetero(512 << 10, 0x5911);
    let het1m = hetero(1 << 20, 0x5912);
    for lvl in 0..=ZSTD_BLOCKSPLITTER_LEVEL_MAX {
        for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_greedy, ZSTD_lazy2, ZSTD_btopt, ZSTD_btultra2] {
            ok(
                &format!("preSplit level={lvl} strategy={strat} hetero=512K"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, 20),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &het512,
            );
        }
        for &k in &[Corpus::Text, Corpus::Random] {
            for &n in &[131_072usize, 131_073, 262_144] {
                let src = corpus(k, n, 0x5913);
                ok(
                    &format!("preSplit level={lvl} corpus={k:?} size={n}"),
                    &[
                        (ZSTD_c_compressionLevel, 3),
                        (ZSTD_c_blockSplitterLevel, lvl),
                    ],
                    &src,
                );
            }
        }
        // CONFIGS row 56's "two halves of different entropy" shape.
        for &n in &[262_144usize, 393_216] {
            let src = sharp(n, n / 2, 0x591B);
            ok(
                &format!("preSplit level={lvl} halves size={n}"),
                &[
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &src,
            );
        }
    }
    // CONFIGS row 147 verbatim: every splitter level x six strategies on 1 MB of
    // mixed-entropy data with windowLog >= 17.
    for lvl in 0..=ZSTD_BLOCKSPLITTER_LEVEL_MAX {
        for &strat in &[
            ZSTD_fast,
            ZSTD_dfast,
            ZSTD_greedy,
            ZSTD_lazy2,
            ZSTD_btopt,
            ZSTD_btultra,
            ZSTD_btultra2,
        ] {
            ok(
                &format!("preSplit level={lvl} strategy={strat} hetero=1M"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, 20),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &het1m,
            );
        }
    }
    rejected(
        "blockSplitterLevel=7",
        &[(ZSTD_c_blockSplitterLevel, ZSTD_BLOCKSPLITTER_LEVEL_MAX + 1)],
        E_parameter_outOfBound,
    );

    // (b) CONFIGS row 148: internal level 0 (`ZSTD_splitBlock_fromBorders`) with
    // the border at 32 KB / 64 KB / 96 KB of the second 128 KB block.
    for &at in &[32usize << 10, 64 << 10, 96 << 10] {
        for &pre in &[0usize, 128 << 10] {
            let mut src = corpus(Corpus::Text, pre, 0x5914);
            src.extend_from_slice(&sharp(128 << 10, at, 0x5915));
            src.extend_from_slice(&corpus(Corpus::Text, 64 << 10, 0x5916));
            ok(
                &format!("fromBorders at={at} prefix={pre}"),
                &[
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_blockSplitterLevel, 2),
                ],
                &src,
            );
        }
    }

    // (c) CONFIGS row 149: internal levels 1..4 (`ZSTD_splitBlock_byChunks`) on
    // 1 MB built from chunks alternating between two distributions. The chunk
    // size straddles CHUNKSIZE = 8192.
    for lvl in 3..=6 {
        for &chunk in &[4096usize, 8192, 16384] {
            let src = alternating(1 << 20, chunk, 0x5917);
            ok(
                &format!("byChunks level={lvl} chunk={chunk}"),
                &[
                    (ZSTD_c_compressionLevel, 3),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &src,
            );
        }
    }

    // (d) The post-splitter: `ZSTD_c_splitAfterSequences` x strategy, plus the
    // auto rule's `windowLog >= 17` boundary (16 vs 17).
    for &split in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for &strat in &[ZSTD_fast, ZSTD_lazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2] {
            for &wlog in &[16, 17, 20] {
                for &k in &[Corpus::Text, Corpus::Random] {
                    for &n in &[131_072usize, 300_000] {
                        let src = corpus(k, n, 0x5918);
                        ok(
                            &format!(
                                "postSplit={split} strategy={strat} wlog={wlog} corpus={k:?} size={n}"
                            ),
                            &[
                                (ZSTD_c_strategy, strat),
                                (ZSTD_c_windowLog, wlog),
                                (ZSTD_c_splitAfterSequences, split),
                            ],
                            &src,
                        );
                    }
                }
            }
        }
        ok(
            &format!("postSplit={split} hetero=512K btultra2"),
            &[
                (ZSTD_c_strategy, ZSTD_btultra2),
                (ZSTD_c_windowLog, 20),
                (ZSTD_c_splitAfterSequences, split),
            ],
            &het512,
        );
        for &lvl in &[1, 3, 9, 19] {
            let src = hetero(300_000, 0x5919);
            ok(
                &format!("postSplit={split} level={lvl} hetero=300K"),
                &[
                    (ZSTD_c_compressionLevel, lvl),
                    (ZSTD_c_splitAfterSequences, split),
                ],
                &src,
            );
        }
        // CONFIGS row 136 verbatim: 1 MB with (btopt, wlog 17), (btopt, wlog 16)
        // — the two sides of the `ZSTD_resolveBlockSplitterMode` auto rule — and
        // (lazy2, wlog 20), where the rule would never auto-enable.
        for &(strat, wlog) in &[
            (ZSTD_btopt, 17),
            (ZSTD_btopt, 16),
            (ZSTD_lazy2, 20),
        ] {
            ok(
                &format!("postSplit={split} strategy={strat} wlog={wlog} hetero=1M"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, wlog),
                    (ZSTD_c_splitAfterSequences, split),
                ],
                &het1m,
            );
        }
    }

    // (e) CONFIGS row 54: the levels whose table rows land on btopt+ with
    // windowLog >= 17, so the post-splitter auto-enables; contrasted with the
    // same levels plus an explicit windowLog=16.
    for &lvl in &[16, 17, 18, 19, 20, 21, 22] {
        for &k in &[Corpus::Text, Corpus::SmallAlphabet, Corpus::Random] {
            for &n in &[131_072usize, 300_000] {
                let src = corpus(k, n, 0x591A);
                ok(
                    &format!("splitAuto level={lvl} corpus={k:?} size={n}"),
                    &[(ZSTD_c_compressionLevel, lvl)],
                    &src,
                );
                ok(
                    &format!("splitAuto level={lvl} wlog16 corpus={k:?} size={n}"),
                    &[(ZSTD_c_compressionLevel, lvl), (ZSTD_c_windowLog, 16)],
                    &src,
                );
            }
        }
    }

    // (f) CONFIGS row 150: both splitters at once.
    for &split in &[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable] {
        for lvl in 0..=ZSTD_BLOCKSPLITTER_LEVEL_MAX {
            ok(
                &format!("bothSplitters post={split} pre={lvl} level19 hetero=1M"),
                &[
                    (ZSTD_c_compressionLevel, 19),
                    (ZSTD_c_splitAfterSequences, split),
                    (ZSTD_c_blockSplitterLevel, lvl),
                ],
                &het1m,
            );
        }
    }
}

// ===========================================================================
// 8. maxBlockSize
// ===========================================================================

/// `ZSTD_c_maxBlockSize` shrinks `cctx->blockSizeMax`, which changes the number
/// of blocks, disables the pre-splitter entirely (`blockSizeMax < 128 KB` in
/// `ZSTD_optimalBlockSize`), and feeds `ZSTD_compressBound`. `0` means
/// "default"; the valid range is `[ZSTD_BLOCKSIZE_MAX_MIN, ZSTD_BLOCKSIZE_MAX]`
/// so 1023 and 131073 must be refused.
#[test]
fn t11_max_block_size() {
    covers(&["CFG:144", "CFG:145"]);
    for &mbs in &[0, ZSTD_BLOCKSIZE_MAX_MIN, 2048, 4096, 65536, 131_072] {
        for &strat in &[ZSTD_fast, ZSTD_lazy2, ZSTD_btultra2] {
            for &k in &[Corpus::Text, Corpus::Random, Corpus::Zeros, Corpus::LongRepeats] {
                for &n in &[1024usize, 65536, 300_000] {
                    let src = corpus(k, n, 0xB105 ^ n as u64);
                    ok(
                        &format!("maxBlockSize={mbs} strategy={strat} corpus={k:?} size={n}"),
                        &[(ZSTD_c_strategy, strat), (ZSTD_c_maxBlockSize, mbs)],
                        &src,
                    );
                }
            }
        }
        // With a window smaller than the block size, so both limits bind.
        let src = corpus(Corpus::LongRepeats, 1 << 20, 0xB106);
        ok(
            &format!("maxBlockSize={mbs} wlog11 size=1MB"),
            &[(ZSTD_c_maxBlockSize, mbs), (ZSTD_c_windowLog, 11)],
            &src,
        );
        // `ZSTD_getBlockSize` reports `cctx->blockSizeMax` from the *applied*
        // parameters, i.e. `MIN(ZSTD_BLOCKSIZE_MAX, windowSize, maxBlockSize)`
        // after `ZSTD_resolveMaxBlockSize` — the resolution CONFIGS row 144
        // names alongside the compression itself.
        for &wlog in &[0, 11, 17, 20] {
            diff(&format!("getBlockSize maxBlockSize={mbs} wlog={wlog}"), |l| {
                let ctx = Ctx::cctx(l);
                let mut ps = vec![(ZSTD_c_maxBlockSize, mbs)];
                if wlog != 0 {
                    ps.push((ZSTD_c_windowLog, wlog));
                }
                let (sets, okk) = set_all(l, &ctx, &ps);
                assert!(okk, "{sets:?}");
                let s = corpus(Corpus::Text, 300_000, 0xB108);
                let cap = compress_bound(l, s.len());
                let mut dst = vec![0u8; cap];
                let f = l.sym::<FnCompress2>("ZSTD_compress2");
                let n = unsafe {
                    f(
                        ctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        s.as_ptr() as *const c_void,
                        s.len(),
                    )
                };
                let gb = l.sym::<FnFreeCCtx>("ZSTD_getBlockSize");
                (res(l, n), res(l, unsafe { gb(ctx.ptr) }))
            });
        }
    }
    for &bad in &[ZSTD_BLOCKSIZE_MAX_MIN - 1, 131_073] {
        rejected(
            &format!("maxBlockSize={bad}"),
            &[(ZSTD_c_maxBlockSize, bad)],
            E_parameter_outOfBound,
        );
    }
    // CONFIGS row 145: maxBlockSize at its minimum with incompressible data and
    // dstCapacity exactly `ZSTD_compressBound(srcSize)` — the small-block
    // margin term of the bound is what makes this fit at all.
    let rnd = corpus(Corpus::Random, 1 << 20, 0xB107);
    let (sets, r, comp) = diff_bytes("maxBlockSize=1024 rand 1MB exact bound", |l| {
        let ctx = Ctx::cctx(l);
        let (sets, ok) = set_all(
            l,
            &ctx,
            &[
                (ZSTD_c_maxBlockSize, ZSTD_BLOCKSIZE_MAX_MIN),
                (ZSTD_c_compressionLevel, 1),
            ],
        );
        if !ok {
            return (sets, NOT_RUN, Blob(Vec::new()));
        }
        let cap = compress_bound(l, rnd.len());
        let mut dst = vec![0xCDu8; cap];
        let f = l.sym::<FnCompress2>("ZSTD_compress2");
        let n = unsafe {
            f(
                ctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                rnd.as_ptr() as *const c_void,
                rnd.len(),
            )
        };
        let rr = res(l, n);
        dst.truncate(if let R::Ok(n) = rr { n } else { 0 });
        (sets, rr, Blob(dst))
    });
    for s in &sets {
        assert!(matches!(s, R::Ok(_)), "{s:?}");
    }
    assert!(matches!(r, R::Ok(_)), "exact-bound compression failed: {r:?}");
    rt("maxBlockSize=1024 rand 1MB exact bound", &comp.0, &rnd);
}

// ===========================================================================
// 9. Independent cParam sweeps
// ===========================================================================

/// Each of `hashLog` / `chainLog` / `searchLog` / `minMatch` / `targetLength`
/// swept across its full `ZSTD_cParam_getBounds` range (plus the "0 means
/// default" value) at several strategies.
///
/// These interact: `ZSTD_adjustCParams_internal` clamps `hashLog` to
/// `dictAndWindowLog + 1` and shrinks `chainLog` by
/// `ZSTD_cycleLog(chainLog, strategy) - dictAndWindowLog` (with
/// `ZSTD_cycleLog`'s `btScale` making bt* strategies differ by one), `minMatch
/// == 3` is the only value that allocates the hashLog3 table, `minMatch` is
/// then re-clamped to `MAX(4, MIN(6, minMatch))` for every lazy/bt search
/// function while `zstd_fast.c` accepts 3..7 directly, and `targetLength` means
/// "acceptable match length" for btopt+ but a step size for `ZSTD_fast`.
#[test]
fn t12_cparam_sweeps() {
    covers(&["CFG:15", "CFG:17", "CFG:36", "CFG:158"]);
    let text = corpus(Corpus::Text, 200_000, 0x3131);
    let reps = corpus(Corpus::LongRepeats, 200_000, 0x3132);
    let srcs: [(&str, &Vec<u8>); 2] = [("text", &text), ("repeats", &reps)];

    for (sname, src) in &srcs {
        for &hlog in &[0, ZSTD_HASHLOG_MIN, 7, 12, 17, 24, 25, ZSTD_HASHLOG_MAX] {
            for &strat in &[ZSTD_fast, ZSTD_dfast, ZSTD_lazy2, ZSTD_btopt] {
                ok(
                    &format!("hashLog={hlog} strategy={strat} src={sname}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_hashLog, hlog)],
                    src,
                );
            }
        }
        for &clog in &[0, ZSTD_CHAINLOG_MIN, 7, 16, 24, 29, ZSTD_CHAINLOG_MAX] {
            for &strat in &[ZSTD_dfast, ZSTD_lazy, ZSTD_btlazy2, ZSTD_btopt] {
                ok(
                    &format!("chainLog={clog} strategy={strat} src={sname}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_chainLog, clog)],
                    src,
                );
            }
        }
        for &slog in &[0, ZSTD_SEARCHLOG_MIN, 2, 3, 4, 5, 6, 7, 10, ZSTD_SEARCHLOG_MAX] {
            for &strat in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2, ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra] {
                ok(
                    &format!("searchLog={slog} strategy={strat} src={sname}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_searchLog, slog)],
                    src,
                );
            }
        }
        for mm in 0..=ZSTD_MINMATCH_MAX {
            if mm != 0 && mm < ZSTD_MINMATCH_MIN {
                continue;
            }
            for &strat in ALL_STRATEGIES {
                ok(
                    &format!("minMatch={mm} strategy={strat} src={sname}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_minMatch, mm)],
                    src,
                );
            }
        }
        for &tl in &[0, 1, 2, 32, 64, 999, 1024, 65536, ZSTD_TARGETLENGTH_MAX] {
            for &strat in &[ZSTD_fast, ZSTD_greedy, ZSTD_lazy2, ZSTD_btopt, ZSTD_btultra2] {
                ok(
                    &format!("targetLength={tl} strategy={strat} src={sname}"),
                    &[(ZSTD_c_strategy, strat), (ZSTD_c_targetLength, tl)],
                    src,
                );
            }
        }
        for &wlog in &[0, ZSTD_WINDOWLOG_MIN, 11, 14, 15, 17, 20, 27, ZSTD_WINDOWLOG_MAX] {
            ok(
                &format!("windowLog={wlog} level19 src={sname}"),
                &[(ZSTD_c_compressionLevel, 19), (ZSTD_c_windowLog, wlog)],
                src,
            );
        }
    }

    // CONFIGS row 17: the btlazy2 / btopt+ levels, and chainLog at its maximum
    // with strategy btlazy2 (btScale) versus lazy2 (no btScale).
    for &lvl in &[13, 14, 15, 16, 17, 18, 19, 20, 21, 22] {
        for &n in &[1000usize, 500_000] {
            let src = corpus(Corpus::Text, n, 0x3133);
            ok(&format!("level={lvl} size={n}"), &[(ZSTD_c_compressionLevel, lvl)], &src);
        }
    }
    for &strat in &[ZSTD_lazy2, ZSTD_btlazy2] {
        for &clog in &[24, 29, ZSTD_CHAINLOG_MAX] {
            ok(
                &format!("btScale strategy={strat} chainLog={clog}"),
                &[(ZSTD_c_strategy, strat), (ZSTD_c_chainLog, clog)],
                &text,
            );
        }
    }

    // CONFIGS row 15: each cParam set *individually* on top of level 3 and
    // level 19, so the other six fields come from the level table rather than
    // from a strategy override — a different starting point, and therefore a
    // different `ZSTD_adjustCParams_internal` outcome, than the sweeps above.
    let indiv: &[(&str, c_int, &[c_int])] = &[
        (
            "windowLog",
            ZSTD_c_windowLog,
            &[0, 10, 11, 14, 15, 17, 20, 27, ZSTD_WINDOWLOG_MAX],
        ),
        ("hashLog", ZSTD_c_hashLog, &[0, 6, 12, 25, 30]),
        ("chainLog", ZSTD_c_chainLog, &[0, 6, 16, 30]),
        ("searchLog", ZSTD_c_searchLog, &[0, 1, 4, 5, 6, 30]),
        ("minMatch", ZSTD_c_minMatch, &[0, 3, 4, 5, 6, 7]),
        ("targetLength", ZSTD_c_targetLength, &[0, 1, 64, 999, 131_072]),
        ("strategy", ZSTD_c_strategy, &[0, 1, 9]),
    ];
    let big = corpus(Corpus::Text, 300_000, 0x3134);
    for &lvl in &[3, 19] {
        for &(name, p, vals) in indiv {
            for &v in vals {
                ok(
                    &format!("individual {name}={v} on level {lvl}"),
                    &[(ZSTD_c_compressionLevel, lvl), (p, v)],
                    &big,
                );
            }
        }
    }

    // `ZSTD_cycleLog` itself: hashLog x every strategy.
    let cl = diff("ZSTD_cycleLog matrix", |l| {
        let f = l.sym::<FnCycleLog>("ZSTD_cycleLog");
        let mut v = Vec::new();
        for hl in [0u32, 1, 2, 6, 12, 24, 30] {
            for s in 0..=9 {
                v.push(unsafe { f(hl, s) });
            }
        }
        v
    });
    assert_eq!(cl.len(), 7 * 10);
}

// ===========================================================================
// 10. Randomised joint cParam sweep
// ===========================================================================

/// A fixed-seed joint draw of all seven compression parameters, filtered
/// through `ZSTD_checkCParams` (which is compared across the two libraries
/// first). Independent sweeps can never catch a bug that needs two parameters
/// to disagree — e.g. `hashLog < chainLog` with a bt strategy, or a `searchLog`
/// that makes the row finder pick a 64-entry row while `hashLog` is at its
/// minimum.
#[test]
fn t13_random_joint_cparams() {
    covers(&["CFG:15", "CFG:154"]);
    // First the systematic `ZSTD_checkCParams` matrix CONFIGS row 154 names:
    // each field at min-1 / min / max / max+1 with the others valid, an
    // all-zero struct, and hashLog < chainLog.
    let base = ZSTD_compressionParameters {
        windowLog: 17,
        chainLog: 16,
        hashLog: 17,
        searchLog: 4,
        minMatch: 4,
        targetLength: 16,
        strategy: ZSTD_lazy2,
    };
    let ranges: &[(&str, fn(&mut ZSTD_compressionParameters, c_int), c_int, c_int)] = &[
        ("windowLog", |p, v| p.windowLog = v as c_uint, ZSTD_WINDOWLOG_MIN, ZSTD_WINDOWLOG_MAX),
        ("chainLog", |p, v| p.chainLog = v as c_uint, ZSTD_CHAINLOG_MIN, ZSTD_CHAINLOG_MAX),
        ("hashLog", |p, v| p.hashLog = v as c_uint, ZSTD_HASHLOG_MIN, ZSTD_HASHLOG_MAX),
        ("searchLog", |p, v| p.searchLog = v as c_uint, ZSTD_SEARCHLOG_MIN, ZSTD_SEARCHLOG_MAX),
        ("minMatch", |p, v| p.minMatch = v as c_uint, ZSTD_MINMATCH_MIN, ZSTD_MINMATCH_MAX),
        ("targetLength", |p, v| p.targetLength = v as c_uint, 0, ZSTD_TARGETLENGTH_MAX),
        ("strategy", |p, v| p.strategy = v, ZSTD_fast, ZSTD_btultra2),
    ];
    for &(name, setf, lo, hi) in ranges {
        for &v in &[lo - 1, lo, hi, hi + 1] {
            let mut p = base;
            setf(&mut p, v);
            diff(&format!("checkCParams {name}={v}"), |l| {
                let f = l.sym::<FnCheckCParams>("ZSTD_checkCParams");
                res(l, unsafe { f(p) })
            });
        }
    }
    diff("checkCParams all-zero", |l| {
        let f = l.sym::<FnCheckCParams>("ZSTD_checkCParams");
        res(l, unsafe { f(ZSTD_compressionParameters::default()) })
    });
    {
        let mut p = base;
        p.hashLog = 8;
        p.chainLog = 20;
        diff("checkCParams hashLog<chainLog", |l| {
            let f = l.sym::<FnCheckCParams>("ZSTD_checkCParams");
            res(l, unsafe { f(p) })
        });
    }

    // Then the randomised joint draw.
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    let sizes = [1024usize, 8192, 65536, 131_072, 131_073, 200_000];
    let mut cases = 0usize;
    let mut skipped = 0usize;
    for i in 0..200u32 {
        let p = ZSTD_compressionParameters {
            windowLog: rng.range((ZSTD_WINDOWLOG_MIN - 1) as i64, ZSTD_WINDOWLOG_MAX as i64) as c_uint,
            chainLog: rng.range((ZSTD_CHAINLOG_MIN - 1) as i64, ZSTD_CHAINLOG_MAX as i64) as c_uint,
            hashLog: rng.range((ZSTD_HASHLOG_MIN - 1) as i64, ZSTD_HASHLOG_MAX as i64) as c_uint,
            searchLog: rng.range((ZSTD_SEARCHLOG_MIN - 1) as i64, ZSTD_SEARCHLOG_MAX as i64) as c_uint,
            minMatch: rng.range((ZSTD_MINMATCH_MIN - 1) as i64, (ZSTD_MINMATCH_MAX + 1) as i64) as c_uint,
            targetLength: rng.range(0, ZSTD_TARGETLENGTH_MAX as i64) as c_uint,
            strategy: rng.range(0, (ZSTD_btultra2 + 1) as i64) as c_int,
        };
        let chk = diff(&format!("joint#{i} checkCParams {p:?}"), |l| {
            let f = l.sym::<FnCheckCParams>("ZSTD_checkCParams");
            res(l, unsafe { f(p) })
        });
        if matches!(chk, R::Err(..)) {
            // Out of range for at least one field — `ZSTD_CCtx_setParameter`
            // would refuse it too, which is Phase C's business.
            skipped += 1;
            continue;
        }
        let n = *rng.pick(&sizes);
        let k = *rng.pick(ALL_CORPORA);
        let src = corpus(k, n, 0x4A01 ^ n as u64);
        let row = *rng.pick(&[ZSTD_ps_auto, ZSTD_ps_enable, ZSTD_ps_disable]);
        ok(
            &format!("joint#{i} {p:?} corpus={k:?} size={n} row={row}"),
            &[
                (ZSTD_c_windowLog, p.windowLog as c_int),
                (ZSTD_c_chainLog, p.chainLog as c_int),
                (ZSTD_c_hashLog, p.hashLog as c_int),
                (ZSTD_c_searchLog, p.searchLog as c_int),
                (ZSTD_c_minMatch, p.minMatch as c_int),
                (ZSTD_c_targetLength, p.targetLength as c_int),
                (ZSTD_c_strategy, p.strategy),
                (ZSTD_c_useRowMatchFinder, row),
            ],
            &src,
        );
        cases += 1;
    }
    assert!(cases > 40, "joint sweep degenerated: only {cases} usable of 200 ({skipped} rejected)");
}

// ===========================================================================
// 11. forceMaxWindow / srcSizeHint / deterministicRefPrefix / rsyncable
// ===========================================================================

/// The remaining block-shaping flags.
///
/// * `ZSTD_c_forceMaxWindow` is read exactly once, in
///   `ZSTD_loadDictionaryContent` (`zstd_compress.c:4957/4972`) where it forces
///   `loadedDictEnd = 0`, so it is observable only with a dictionary/prefix.
/// * `ZSTD_c_deterministicRefPrefix` sets `ms->forceNonContiguous`
///   (`zstd_compress.c:4973`), which makes a prefix that happens to sit
///   immediately before `src` behave like one that does not.
/// * `ZSTD_c_srcSizeHint` is consulted only when the pledged source size is
///   unknown (`zstd_compress.c:1641`), where it shifts the
///   `ZSTD_defaultCParameters` table row via
///   `tableID = (rSize<=256KB) + (rSize<=128KB) + (rSize<=16KB)`.
/// * `ZSTD_c_rsyncable` must reject every non-zero value with
///   `parameter_unsupported` because `ZSTD_MULTITHREAD` is not defined
///   (`zstd_compress.c:900-903`), and `ZSTD_CCtxParams_getParameter` must
///   return the same error.
#[test]
fn t14_misc_shaping_flags() {
    covers(&["CFG:108", "CFG:125", "CFG:130", "CFG:138"]);
    let src = corpus(Corpus::Text, 200 << 10, 0xF00D);
    let prefix = corpus(Corpus::Text, 64 << 10, 0xF00E);

    // forceMaxWindow / deterministicRefPrefix without any dictionary: both are
    // pure no-ops, which is itself worth pinning byte-for-byte.
    for &fmw in &[0, 1] {
        for &drp in &[0, 1] {
            for &strat in &[ZSTD_fast, ZSTD_lazy2, ZSTD_btopt] {
                ok(
                    &format!("forceMaxWindow={fmw} detRefPrefix={drp} strategy={strat} nodict"),
                    &[
                        (ZSTD_c_strategy, strat),
                        (ZSTD_c_windowLog, 17),
                        (ZSTD_c_forceMaxWindow, fmw),
                        (ZSTD_c_deterministicRefPrefix, drp),
                    ],
                    &src,
                );
            }
        }
    }

    // With a prefix, so `ZSTD_loadDictionaryContent` actually sees the flags.
    // The prefix is passed both as its own allocation and as the 64 KB that
    // immediately precedes `src` inside one allocation (the case
    // `deterministicRefPrefix` exists for).
    let mut joined = prefix.clone();
    joined.extend_from_slice(&src);
    for &fmw in &[0, 1] {
        for &drp in &[0, 1] {
            for contiguous in [false, true] {
                let label = format!(
                    "prefix forceMaxWindow={fmw} detRefPrefix={drp} contiguous={contiguous}"
                );
                let (sets, r, comp) = diff_bytes(&label, |l| {
                    let ctx = Ctx::cctx(l);
                    let (sets, okk) = set_all(
                        l,
                        &ctx,
                        &[
                            (ZSTD_c_compressionLevel, 6),
                            (ZSTD_c_windowLog, 17),
                            (ZSTD_c_forceMaxWindow, fmw),
                            (ZSTD_c_deterministicRefPrefix, drp),
                        ],
                    );
                    if !okk {
                        return (sets, NOT_RUN, Blob(Vec::new()));
                    }
                    let rp = l.sym::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
                    let (pptr, sptr) = if contiguous {
                        (joined.as_ptr(), unsafe { joined.as_ptr().add(prefix.len()) })
                    } else {
                        (prefix.as_ptr(), src.as_ptr())
                    };
                    let e = unsafe { rp(ctx.ptr, pptr as *const c_void, prefix.len()) };
                    if is_error(l, e) {
                        return (sets, res(l, e), Blob(Vec::new()));
                    }
                    let cap = compress_bound(l, src.len()).max(64);
                    let mut dst = vec![0xCDu8; cap];
                    let f = l.sym::<FnCompress2>("ZSTD_compress2");
                    let n = unsafe {
                        f(
                            ctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            sptr as *const c_void,
                            src.len(),
                        )
                    };
                    let rr = res(l, n);
                    dst.truncate(if let R::Ok(n) = rr { n } else { 0 });
                    (sets, rr, Blob(dst))
                });
                for s in &sets {
                    assert!(matches!(s, R::Ok(_)), "[{label}] {s:?}");
                }
                assert!(matches!(r, R::Ok(_)), "[{label}] compress2: {r:?}");
                // Decode needs the same prefix.
                let (_, dr, plain) = diff_bytes(&format!("{label}|rt"), |l| {
                    let ctx = Ctx::dctx(l);
                    let rp = l.sym::<FnRefPrefix>("ZSTD_DCtx_refPrefix");
                    let e = unsafe { rp(ctx.ptr, prefix.as_ptr() as *const c_void, prefix.len()) };
                    if is_error(l, e) {
                        return (vec![res(l, e)], NOT_RUN, Blob(Vec::new()));
                    }
                    let mut dst = vec![0xCDu8; src.len()];
                    let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
                    let n = unsafe {
                        f(
                            ctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            src.len(),
                            comp.0.as_ptr() as *const c_void,
                            comp.0.len(),
                        )
                    };
                    let rr = res(l, n);
                    dst.truncate(if let R::Ok(n) = rr { n } else { 0 });
                    (Vec::new(), rr, Blob(dst))
                });
                assert!(matches!(dr, R::Ok(_)), "[{label}] prefix round-trip: {dr:?}");
                assert!(plain.0.as_slice() == src.as_slice(), "[{label}] prefix round-trip content");
            }
        }
    }

    // srcSizeHint. Through ZSTD_compress2 it can have no effect at all (the
    // pledged size is always known); through streaming it selects a different
    // ZSTD_defaultCParameters row.
    let hint_src = corpus(Corpus::Text, 1 << 20, 0xF010);
    let mut oneshot: Option<Blob> = None;
    for &h in &[0, 1, 1024, 16384, 16385, 131_072, 131_073, 262_144, 1 << 20, i32::MAX] {
        let b = ok(
            &format!("srcSizeHint={h} compress2"),
            &[(ZSTD_c_compressionLevel, 3), (ZSTD_c_srcSizeHint, h)],
            &hint_src,
        );
        match &oneshot {
            None => oneshot = Some(b),
            Some(first) => assert_eq!(
                first, &b,
                "ZSTD_c_srcSizeHint={h} must not change ZSTD_compress2 output: \
                 ZSTD_CCtx_init_compressStream2 always pledges inSize for ZSTD_e_end"
            ),
        }
        ok_stream(
            &format!("srcSizeHint={h} stream"),
            &[(ZSTD_c_compressionLevel, 3), (ZSTD_c_srcSizeHint, h)],
            &hint_src,
            32 << 10,
        );
        diff(&format!("srcSizeHint={h} getParameter"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            let s = res(l, unsafe { set(ctx.ptr, ZSTD_c_srcSizeHint, h) });
            let get = l.sym::<FnCCtxGetParameter>("ZSTD_CCtx_getParameter");
            let mut v: c_int = -777;
            let g = res(l, unsafe { get(ctx.ptr, ZSTD_c_srcSizeHint, &mut v) });
            (s, g, v)
        });
    }

    // rsyncable: unsupported for every non-zero value, on a CCtx and on a
    // ZSTD_CCtx_params, and unsupported to read back at all.
    for &v in &[0, 1, 2, -1, i32::MAX] {
        diff(&format!("rsyncable={v} on CCtx"), |l| {
            let ctx = Ctx::cctx(l);
            let set = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
            res(l, unsafe { set(ctx.ptr, ZSTD_c_rsyncable, v) })
        });
        let r = diff(&format!("rsyncable={v} on CCtxParams"), |l| {
            let create = l.sym::<FnCreateCCtxParams>("ZSTD_createCCtxParams");
            let p = unsafe { create() };
            assert!(!p.is_null());
            let ctx = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
            let set = l.sym::<FnParamsSet>("ZSTD_CCtxParams_setParameter");
            let s = res(l, unsafe { set(ctx.ptr, ZSTD_c_rsyncable, v) });
            let get = l.sym::<FnParamsGet>("ZSTD_CCtxParams_getParameter");
            let mut out: c_int = -777;
            let g = res(l, unsafe { get(ctx.ptr, ZSTD_c_rsyncable, &mut out) });
            (s, g, out)
        });
        if v == 0 {
            assert_eq!(r.0, R::Ok(0), "rsyncable=0 must be accepted");
        } else {
            assert_eq!(
                r.0,
                R::Err(E_parameter_unsupported, "Unsupported parameter".to_string()),
                "rsyncable={v} must be parameter_unsupported without ZSTD_MULTITHREAD"
            );
        }
        assert!(
            matches!(r.1, R::Err(E_parameter_unsupported, _)),
            "reading ZSTD_c_rsyncable back must be parameter_unsupported, got {:?}",
            r.1
        );
    }
    // Compression still works after rsyncable=0.
    ok(
        "rsyncable=0 then compress",
        &[(ZSTD_c_rsyncable, 0), (ZSTD_c_compressionLevel, 3)],
        &src,
    );
}

// ===========================================================================
// 12. Exported block-compressor entry points
// ===========================================================================

/// Every `ZSTD_compressBlock_*` entry point named in `CONFIGS.md` /
/// `SYMBOLS.md`, plus `ZSTD_selectBlockCompressor`, `ZSTD_splitBlock`,
/// `ZSTD_row_update`, `ZSTD_cycleLog` and `ZSTD_checkCParams`, must exist in
/// both shared objects: `ZSTD_selectBlockCompressor` dispatches to them by
/// table index, so a missing one is a completeness failure that no
/// higher-level test can distinguish from "this configuration was never
/// reached". Their *behaviour* is covered transitively by the tests above (the
/// `ERRORS.md` rows for `zstd_fast.c` / `zstd_double_fast.c` / `zstd_lazy.c` /
/// `zstd_opt.c` record that those files contain no rejection sites at all).
#[test]
fn t15_block_compressor_symbols() {
    covers(&["CFG:18", "CFG:19", "CFG:24", "CFG:25", "CFG:26", "CFG:27"]);
    let p = pair();
    // Drive one real compression per strategy first, so the symbols are known
    // to be reachable and not merely present.
    let src = corpus(Corpus::Text, 300_000, 0x5B10);
    for &strat in ALL_STRATEGIES {
        for &row in &[ZSTD_ps_enable, ZSTD_ps_disable] {
            ok(
                &format!("dispatch strategy={strat} row={row}"),
                &[
                    (ZSTD_c_strategy, strat),
                    (ZSTD_c_windowLog, 20),
                    (ZSTD_c_useRowMatchFinder, row),
                ],
                &src,
            );
        }
    }
    let mut missing = Vec::new();
    for name in BLOCK_COMPRESSORS {
        if !p.c.has(name) {
            missing.push(format!("C:{name}"));
        }
        if !p.r.has(name) {
            missing.push(format!("RUST:{name}"));
        }
    }
    assert!(missing.is_empty(), "missing block-compressor symbols: {missing:?}");
}

// ===========================================================================
// 13. The two shaping helpers called directly
// ===========================================================================

/// `ZSTD_splitBlock` and `ZSTD_adjustCParams`, driven straight through the FFI.
///
/// `ZSTD_optimalBlockSize` only ever reaches `ZSTD_splitBlock` for *full*
/// 128 KB blocks and only once `savings >= 3`, so calling it directly is the
/// only way to pin every one of its return values: internal level 0 is
/// `ZSTD_splitBlock_fromBorders`, whose three exits are `blockSize`, `64 KB`,
/// and `32 KB`/`96 KB` depending on which end the middle fingerprint is closer
/// to (`minDistance = SEGMENT_SIZE*SEGMENT_SIZE/3`); internal levels 1..4 are
/// `ZSTD_splitBlock_byChunks` with per-level sampling rates {43,11,5,1} and
/// hashLogs {8,9,10,10}.
///
/// `ZSTD_adjustCParams` is the clamp every test above depends on: the
/// `srcSize <= maxWindowResize` window shrink, `hashLog <= dictAndWindowLog+1`,
/// `chainLog -= cycleLog - dictAndWindowLog`, the
/// `ZSTD_WINDOWLOG_ABSOLUTEMIN` floor, and the row-matchfinder
/// `hashLog <= (32 - ZSTD_ROW_HASH_TAG_BITS) + BOUNDED(4, searchLog, 6)` cap.
/// Note it runs `ZSTD_clampCParams` first, so it accepts deliberately invalid
/// input where `ZSTD_checkCParams` would not.
#[test]
fn t16_split_block_and_adjust_cparams() {
    covers(&["CFG:148", "CFG:149", "CFG:155", "CFG:156", "CFG:157", "CFG:158"]);
    type FnSplitBlock =
        unsafe extern "C" fn(*const c_void, SizeT, c_int, *mut c_void, SizeT) -> SizeT;
    /// `ZSTD_SLIPBLOCK_WORKSPACESIZE` from `compress/zstd_preSplit.h`.
    const WKSP: usize = 8208;
    // `ZSTD_splitBlock` documents (and asserts) `blockSize == 128 KB`; it reads
    // `blockStart + blockSize - SEGMENT_SIZE`, so no other size is in contract.
    const BLK: usize = 128 << 10;

    let mut blocks: Vec<(String, Vec<u8>)> = Vec::new();
    for &k in ALL_CORPORA {
        blocks.push((format!("{k:?}"), corpus(k, BLK, 0x5B20)));
    }
    for &at in &[0usize, 512, 1024, 32 << 10, 64 << 10, 96 << 10, BLK - 512, BLK] {
        blocks.push((format!("sharp@{at}"), sharp(BLK, at, 0x5B21)));
    }
    for &chunk in &[512usize, 4096, 8192, 16384, 32768] {
        blocks.push((
            format!("alternating/{chunk}"),
            alternating(BLK, chunk, 0x5B22),
        ));
    }
    for (name, blk) in &blocks {
        assert_eq!(blk.len(), BLK);
        for level in 0..=4 {
            let got = diff(&format!("ZSTD_splitBlock level={level} {name}"), |l| {
                let f = l.sym::<FnSplitBlock>("ZSTD_splitBlock");
                let mut w = vec![0u64; WKSP / 8 + 1];
                res(l, unsafe {
                    f(
                        blk.as_ptr() as *const c_void,
                        BLK,
                        level,
                        w.as_mut_ptr() as *mut c_void,
                        w.len() * 8,
                    )
                })
            });
            if let R::Ok(n) = got {
                assert!(n > 0 && n <= BLK, "[{name}] level={level} split={n}");
            } else {
                panic!("[{name}] level={level}: {got:?}");
            }
        }
    }

    // ZSTD_adjustCParams. CONFIGS row 155: a valid level-19 cParams against a
    // wide srcSize x dictSize grid; row 156: a deliberately invalid cParams
    // (ZSTD_clampCParams has to fix it up first); row 157: greedy/lazy/lazy2
    // with searchLog 3..7 and hashLog 30, which is precisely the row-hash cap.
    type FnAdjust = unsafe extern "C" fn(
        ZSTD_compressionParameters,
        c_ulonglong,
        SizeT,
    ) -> ZSTD_compressionParameters;
    type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;
    let base19 = diff("ZSTD_getCParams(19,0,0)", |l| {
        let f = l.sym::<FnGetCParams>("ZSTD_getCParams");
        unsafe { f(19, 0, 0) }
    });
    for &srcSize in &[
        0u64,
        1,
        63,
        64,
        65,
        512,
        513,
        1 << 20,
        1 << 30,
        (1u64 << 30) + 1,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ] {
        for &dictSize in &[0usize, 1000, (1usize << 30) + 1] {
            diff(
                &format!("adjustCParams(level19, src={srcSize}, dict={dictSize})"),
                |l| {
                    let f = l.sym::<FnAdjust>("ZSTD_adjustCParams");
                    unsafe { f(base19, srcSize, dictSize) }
                },
            );
        }
    }
    {
        let bad = ZSTD_compressionParameters {
            windowLog: 40,
            chainLog: 0,
            hashLog: 99,
            searchLog: 0,
            minMatch: 1,
            targetLength: 1 << 20,
            strategy: 77,
        };
        diff("adjustCParams(deliberately invalid)", |l| {
            let f = l.sym::<FnAdjust>("ZSTD_adjustCParams");
            unsafe { f(bad, 1 << 20, 0) }
        });
    }
    for &strat in &[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2] {
        for slog in 3..=7 {
            let p = ZSTD_compressionParameters {
                windowLog: 27,
                chainLog: 24,
                hashLog: 30,
                searchLog: slog,
                minMatch: 4,
                targetLength: 0,
                strategy: strat,
            };
            diff(
                &format!("adjustCParams(rowHashCap strategy={strat} searchLog={slog})"),
                |l| {
                    let f = l.sym::<FnAdjust>("ZSTD_adjustCParams");
                    unsafe { f(p, 1 << 20, 0) }
                },
            );
        }
    }
}
