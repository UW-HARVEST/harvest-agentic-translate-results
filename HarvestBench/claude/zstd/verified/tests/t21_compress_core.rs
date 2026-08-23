//! Phase B — CORE one-shot compression, one-shot decompression and
//! frame-header *shape*.
//!
//! Everything here goes through the exported ABI of both shared objects
//! (`Lib::sym`), never through the Rust crate directly, so the `#[no_mangle]`
//! wrappers are under test too.
//!
//! The entry points covered are `ZSTD_compress`, `ZSTD_compressCCtx`,
//! `ZSTD_compress2`, `ZSTD_compress_advanced`, `ZSTD_compress_usingDict`,
//! `ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_decompress_usingDict`,
//! `ZSTD_getFrameContentSize`, `ZSTD_getDecompressedSize`,
//! `ZSTD_findFrameCompressedSize`, `ZSTD_findDecompressedSize`,
//! `ZSTD_decompressBound`, `ZSTD_getFrameHeader`, `ZSTD_getDictID_fromFrame`
//! (plus the three sibling `getDictID_from*`), `ZSTD_isFrame`,
//! `ZSTD_isSkippableFrame`, `ZSTD_getParams`, `ZSTD_CCtx_setPledgedSrcSize`,
//! `ZSTD_isError` / `ZSTD_getErrorName` / `ZSTD_getErrorCode`.
//!
//! Two things are deliberately *not* here because they are error-path or
//! dictionary-builder territory: crafted/corrupted frames (ERRORS.md rows) and
//! trained `ZSTD_MAGIC_DICTIONARY` dictionaries. All dictionaries used below are
//! raw content (`ZSTD_dct_auto` -> `ZSTD_loadDictionaryContent`).
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
mod common;
use common::*;
use std::collections::BTreeSet;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::fmt;

// ---------------------------------------------------------------------------
// ABI signatures not already declared in the harness
// ---------------------------------------------------------------------------

type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnCompressAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
    ZSTD_parameters,
) -> SizeT;
type FnCompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;
type FnU64FromBuf = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
type FnUIntFromBuf = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;
type FnSetPledgedSrcSize = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> SizeT;
type FnGetFrameHeader =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT) -> SizeT;
type FnPtrToUInt = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;
type FnWriteSkippableFrame =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_uint) -> SizeT;

// ---------------------------------------------------------------------------
// Small reporting helpers
// ---------------------------------------------------------------------------

/// `unsigned long long` returns of the frame-size queries carry two sentinels;
/// naming them makes a divergence readable instead of `18446744073709551615`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum U64R {
    Unknown,
    Error,
    N(u64),
}

impl fmt::Debug for U64R {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            U64R::Unknown => write!(f, "CONTENTSIZE_UNKNOWN"),
            U64R::Error => write!(f, "CONTENTSIZE_ERROR"),
            U64R::N(n) => write!(f, "{n}"),
        }
    }
}

fn u64r(v: c_ulonglong) -> U64R {
    if v == ZSTD_CONTENTSIZE_UNKNOWN {
        U64R::Unknown
    } else if v == ZSTD_CONTENTSIZE_ERROR {
        U64R::Error
    } else {
        U64R::N(v)
    }
}

/// `ZSTD_compressBound` from the C side; both libraries must already agree on it
/// (t00/other rows check that), and using one value keeps the two runs symmetric.
fn cbound(n: usize) -> usize {
    compress_bound(&pair().c, n).max(64)
}

/// One-shot `ZSTD_compress` returning the status **plus the entire destination
/// buffer**, so bytes the compressor never wrote are compared as well.
fn comp_full(l: &Lib, src: &[u8], level: c_int, cap: usize) -> (R, Blob) {
    comp_full_cap(l, src, level, cap, cap)
}

/// As [`comp_full`] but the allocation (`buflen`) is deliberately larger than the
/// advertised `cap`, so "did the callee stay inside the capacity it was given?"
/// is part of the compared result even when `cap` is far too small.
fn comp_full_cap(l: &Lib, src: &[u8], level: c_int, cap: usize, buflen: usize) -> (R, Blob) {
    assert!(buflen >= cap);
    let f = l.sym::<FnCompress>("ZSTD_compress");
    let mut dst = vec![0xCDu8; buflen];
    let n = unsafe {
        f(
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        )
    };
    (res(l, n), Blob(dst))
}

/// `ZSTD_compressCCtx` on a freshly created context (whole buffer compared).
fn comp_cctx_full(l: &Lib, src: &[u8], level: c_int, cap: usize) -> (R, Blob) {
    let cctx = Ctx::cctx(l);
    let f = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
    let mut dst = vec![0xCDu8; cap];
    let n = unsafe {
        f(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            level,
        )
    };
    (res(l, n), Blob(dst))
}

/// `ZSTD_compress2` on a fresh context after applying `set` (whole buffer).
fn comp2_full(
    l: &Lib,
    src: &[u8],
    cap: usize,
    set: &dyn Fn(&Lib, *mut c_void) -> Vec<R>,
) -> (Vec<R>, R, Blob) {
    let cctx = Ctx::cctx(l);
    let sets = set(l, cctx.ptr);
    let f = l.sym::<FnCompress2>("ZSTD_compress2");
    let mut dst = vec![0xCDu8; cap];
    let n = unsafe {
        f(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    (sets, res(l, n), Blob(dst))
}

fn set_param(l: &Lib, cctx: *mut c_void, p: c_int, v: c_int) -> R {
    let f = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    res(l, unsafe { f(cctx, p, v) })
}

/// `ZSTD_decompressDCtx` on a fresh DCtx, whole destination buffer compared.
fn decomp_dctx_full(l: &Lib, frame: &[u8], cap: usize) -> (R, Blob) {
    let dctx = Ctx::dctx(l);
    let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    let mut dst = vec![0xCDu8; cap];
    let n = unsafe {
        f(
            dctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            frame.as_ptr() as *const c_void,
            frame.len(),
        )
    };
    (res(l, n), Blob(dst))
}

/// The poison pattern used for every out-param struct: if a callee is documented
/// not to fill it (`ZSTD_getFrameHeader` returning "need more input"), the
/// untouched bytes are part of the compared result.
fn poison_zfh() -> ZSTD_FrameHeader {
    ZSTD_FrameHeader {
        frameContentSize: 0x5A5A_5A5A_5A5A_5A5A,
        windowSize: 0x5A5A_5A5A_5A5A_5A5A,
        blockSizeMax: 0x5A5A_5A5A,
        frameType: 0x5A5A_5A5A,
        headerSize: 0x5A5A_5A5A,
        dictID: 0x5A5A_5A5A,
        checksumFlag: 0x5A5A_5A5A,
        _reserved1: 0x5A5A_5A5A,
        _reserved2: 0x5A5A_5A5A,
    }
}

// ---------------------------------------------------------------------------
// Level and size sets
// ---------------------------------------------------------------------------

/// `ZSTD_minCLevel()..=ZSTD_maxCLevel()` is far too wide to enumerate, but every
/// *distinct behaviour* lives in `-7..=22` (each is its own row of
/// `ZSTD_defaultCParameters`) plus the clamp/extreme values that
/// `ZSTD_getCParams_internal` folds onto row 0 with a different
/// `targetLength = -MAX(ZSTD_minCLevel(), level)`, and the above-max values it
/// folds onto row 22. `ZSTD_compress` deliberately does *not* clamp the level
/// before calling `ZSTD_getCParams_internal`, so `INT_MIN` / `INT_MAX` reach it
/// raw and only the `-MAX(ZSTD_minCLevel(), level)` negation clamps them.
fn all_levels() -> Vec<c_int> {
    let l = &pair().c;
    let minc = unsafe { l.sym::<FnMinCLevel>("ZSTD_minCLevel")() };
    let maxc = unsafe { l.sym::<FnMaxCLevel>("ZSTD_maxCLevel")() };
    assert_eq!((minc, maxc), (-131072, 22), "unexpected clevel range");
    let mut v: Vec<c_int> = (-7..=22).collect();
    v.extend_from_slice(&[
        c_int::MIN,
        -1_000_000,
        minc - 1,
        minc,
        minc + 1,
        -1000,
        -100,
        -22,
        -10,
        maxc,
        maxc + 1,
        100,
        c_int::MAX,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}

/// Sizes straddling every one-shot branch point: 0 (no block at all), <7
/// (`ZSTD_buildSeqStore` -> `ZSTDbss_noCompress`), 7/8 (real match finder),
/// 255/256/257 (`fcsCode` 0 vs 1), 1023/1024 (window auto-resize / singleSegment
/// boundary) and 4096.
const SMALL_SIZES: &[usize] = &[0, 1, 2, 3, 6, 7, 8, 12, 13, 100, 255, 256, 257, 1023, 1024, 4096];

/// 65536+256 = 65792 is the `fcsCode` 1->2 boundary and 131072 is
/// `ZSTD_BLOCKSIZE_MAX`; both sides of both matter.
const LARGE_SIZES: &[usize] = &[65535, 65536, 131071, 131072, 131072 + 1, 200000, 300000];

/// Beyond the 256 KB `ZSTD_getCParams_internal` table boundary, and past the
/// level-1 window (2^19) so `singleSegment` finally becomes 0 for a default
/// compression too.
const HUGE_SIZES: &[usize] = &[600_000, 1_200_000];

/// Levels cheap enough to cross with all ten corpora on the multi-block sizes:
/// the whole negative range (row 0 + `targetLength` acceleration), the four
/// `ZSTD_defaultCParameters` tables' fast/dfast/greedy/lazy/lazy2 rows, and the
/// two btlazy2 rows.
const CHEAP_LEVELS: &[c_int] =
    &[-131072, -131071, -1000, -100, -22, -5, -1, 0, 1, 3, 6, 9, 12, 13, 15];
/// btopt / btultra / btultra2 levels. Quadratic-ish in the input, so the widest
/// corpus cross is reserved for the `ZSTD_BLOCKSIZE_MAX` boundary.
const HEAVY_LEVELS: &[c_int] = &[17, 18, 19, 20, 21, 22];
const HEAVY_LEVELS_WIDE: &[c_int] = &[17, 19, 22];

fn seed_for(kind: Corpus, len: usize) -> u64 {
    // Fixed, derived seed: reproducible run-to-run, different per cell.
    0x5DEE_CE66_D000_1234u64 ^ ((kind as u64) << 40) ^ (len as u64).wrapping_mul(0x9E37_79B9)
}

// ===========================================================================
// 1. ZSTD_compress over the whole compression-level range
// ===========================================================================

/// Every compression level (all of `-7..=22`, plus `ZSTD_minCLevel()`,
/// `ZSTD_minCLevel()+1`, -1000, -100, -22 and `ZSTD_maxCLevel()`) crossed with
/// all ten corpus shapes and every small boundary size.
///
/// Targets `ZSTD_getCParams_internal`'s table selection
/// (`tableID = (rSize<=256KB)+(rSize<=128KB)+(rSize<=16KB)`) and row selection
/// (`level==0 -> 3`, `level<0 -> row 0` with
/// `targetLength = -MAX(ZSTD_minCLevel(), level)`), plus
/// `ZSTD_adjustCParams_internal`'s window/hash/chain downsizing at the 64-byte
/// `hashSizeMin` and 513-byte `minSrcSize` constants, plus `ZSTD_buildSeqStore`'s
/// `srcSize < 7` no-compress threshold and `ZSTD_writeFrameHeader`'s `fcsCode`
/// 0/1 and `singleSegment` encodings — all in one cross-product, byte-for-byte.
#[test]
fn compress_every_level_small_sizes() {
    covers(&["CFG:9", "CFG:10", "CFG:11", "CFG:16", "CFG:37", "CFG:38", "CFG:46", "CFG:100"]);
    let levels = all_levels();
    for &n in SMALL_SIZES {
        let cap = cbound(n);
        for &k in ALL_CORPORA {
            // Three independent fixed seeds per shape: property-style, so the
            // result does not hinge on one lucky byte pattern.
            for seed_ix in 0..3u64 {
                let src = corpus(k, n, seed_for(k, n) ^ (seed_ix * 0x9E37_79B9_7F4A_7C15));
                for &lvl in &levels {
                    diff_bytes(
                        &format!("ZSTD_compress n={n} {k:?} seed={seed_ix} lvl={lvl}"),
                        |l| compress_simple(l, &src, lvl, cap),
                    );
                }
            }
        }
    }
}

/// The same sweep on the multi-block sizes, with the level set trimmed to the
/// representative subset `{-5,-1,0,1,3,6,9,12,17,19,22}`. 131072 is
/// `ZSTD_BLOCKSIZE_MAX`, so 131071/131072/131073 exercise one short block, one
/// exactly-full block and two blocks; 65792 = 65536+256 is the `fcsCode` 1->2
/// boundary and is straddled by 65535/65536 and 200000/300000.
///
/// The `Random` corpus additionally forces the `cSize == 0` "not compressible"
/// arm of `ZSTD_compressBlock_internal` (bt_raw blocks, `set_basic` literals),
/// and `Zeros`/`OneByte` force the bt_rle shortcut on every block but the first.
#[test]
fn compress_levels_large_sizes() {
    covers(&["CFG:9", "CFG:10", "CFG:11", "CFG:39", "CFG:47", "CFG:48", "CFG:54", "CFG:100"]);
    for &n in LARGE_SIZES {
        let cap = cbound(n);
        for &k in ALL_CORPORA {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in CHEAP_LEVELS {
                diff_bytes(&format!("ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                    compress_simple(l, &src, lvl, cap)
                });
            }
        }
        // btopt+ levels: every level x a 3-corpus subset at every size, plus all
        // ten corpora x {17,19,22} on the ZSTD_BLOCKSIZE_MAX boundary. Splitting
        // it this way is pure runtime budgeting.
        for &k in &[Corpus::Zeros, Corpus::Random, Corpus::Text] {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in HEAVY_LEVELS {
                diff_bytes(&format!("ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                    compress_simple(l, &src, lvl, cap)
                });
            }
        }
        if (131071..=131073).contains(&n) {
            for &k in ALL_CORPORA {
                if matches!(k, Corpus::Zeros | Corpus::Random | Corpus::Text) {
                    continue; // already done above, for every heavy level
                }
                let src = corpus(k, n, seed_for(k, n));
                for &lvl in HEAVY_LEVELS_WIDE {
                    diff_bytes(&format!("ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                        compress_simple(l, &src, lvl, cap)
                    });
                }
            }
        }
    }
}

/// The 256 KB / 512 KB region of `ZSTD_getCParams_internal`'s table selection and
/// of the level-1 window: 600000 and 1200000 bytes are 5 and 10 blocks, cross the
/// last `tableID` boundary, and exceed 2^19 so `singleSegment` becomes 0 even for
/// the default parameters. `LongRepeats` at these sizes also makes level 22 turn
/// LDM on by itself (`ZSTD_resolveEnableLdm`'s `strategy >= btopt && windowLog >=
/// 27` auto rule).
#[test]
fn compress_levels_huge_sizes() {
    covers(&["CFG:9", "CFG:39", "CFG:47", "CFG:48", "CFG:100"]);
    for &n in HUGE_SIZES {
        let cap = cbound(n);
        for &k in ALL_CORPORA {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in &[-131072, -5, -1, 0, 1, 3, 6, 9, 12] {
                diff_bytes(&format!("ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                    compress_simple(l, &src, lvl, cap)
                });
            }
        }
        for &k in &[Corpus::Zeros, Corpus::Random, Corpus::LongRepeats] {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in &[17, 19, 22] {
                diff_bytes(&format!("ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                    compress_simple(l, &src, lvl, cap)
                });
            }
        }
    }
}

/// `ZSTD_adjustCParams_internal`'s two exact constants, and the block-splitter
/// auto rule, driven from the one-shot entry points.
///
/// * window downsizing: `srcLog = (tSize < 1<<ZSTD_HASHLOG_MIN) ? 6 :
///   highbit32(tSize-1)+1` then `windowLog = MIN(windowLog, srcLog)`, followed by
///   the `windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN -> 10` floor. `tSize` 63/64/65
///   straddles the `hashSizeMin` branch and 512/513/514 the `minSrcSize`
///   constant, at level 1 and level 19 and with an explicit `windowLog=27` that
///   must be downsized right back.
/// * `ZSTD_resolveBlockSplitterMode`'s auto rule is `strategy >= ZSTD_btopt &&
///   windowLog >= 17`, so levels 16..22 with the level table's own (large)
///   windowLog take `ZSTD_compressBlock_splitBlock` while the very same levels
///   with an explicit `windowLog=16` must not — a different sequence-store
///   pipeline (`ZSTD_deriveBlockSplits` + `ZSTD_seqStore_resolveOffCodes`) for
///   otherwise identical parameters.
#[test]
fn window_and_block_splitter_boundaries() {
    covers(&["CFG:16", "CFG:54"]);
    for &n in &[1usize, 2, 63, 64, 65, 512, 513, 514, 4096, 65536] {
        let cap = cbound(n);
        for &k in &[Corpus::Zeros, Corpus::Random, Corpus::Text] {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in &[1, 19] {
                diff_bytes(&format!("wlog-adjust ZSTD_compress n={n} {k:?} lvl={lvl}"), |l| {
                    compress_simple(l, &src, lvl, cap)
                });
                for &wl in &[None, Some(10), Some(27)] {
                    let label = format!("wlog-adjust compress2 n={n} {k:?} lvl={lvl} wlog={wl:?}");
                    let (_, r, b) = diff_bytes(&label, |l| {
                        comp2_full(l, &src, cap, &|l, c| {
                            let mut v = vec![set_param(l, c, ZSTD_c_compressionLevel, lvl)];
                            if let Some(w) = wl {
                                v.push(set_param(l, c, ZSTD_c_windowLog, w));
                            }
                            v
                        })
                    });
                    let cs = match r {
                        R::Ok(v) => v,
                        R::Err(a, s) => panic!("{label}: {a}:{s}"),
                    };
                    diff_bytes(&format!("{label} :: decompress"), |l| {
                        decompress_simple(l, &b.0[..cs], n)
                    });
                }
            }
        }
    }

    for &n in &[131072usize, 300000] {
        let cap = cbound(n);
        for &k in &[Corpus::Text, Corpus::SmallAlphabet, Corpus::Random] {
            let src = corpus(k, n, seed_for(k, n));
            for &lvl in &[16, 17, 19, 22] {
                for &wl in &[None, Some(16)] {
                    let label = format!("splitter n={n} {k:?} lvl={lvl} wlog={wl:?}");
                    let (_, r, b) = diff_bytes(&label, |l| {
                        comp2_full(l, &src, cap, &|l, c| {
                            let mut v = vec![set_param(l, c, ZSTD_c_compressionLevel, lvl)];
                            if let Some(w) = wl {
                                v.push(set_param(l, c, ZSTD_c_windowLog, w));
                            }
                            v
                        })
                    });
                    let cs = match r {
                        R::Ok(v) => v,
                        R::Err(a, s) => panic!("{label}: {a}:{s}"),
                    };
                    let got = diff_bytes(&format!("{label} :: decompress"), |l| {
                        decompress_simple(l, &b.0[..cs], n)
                    });
                    assert_eq!(got.1 .0, src, "{label}: content mismatch");
                }
            }
        }
    }
}

// ===========================================================================
// 2. Round trips
// ===========================================================================

/// Round trip in both directions:
///   * compress with the **C**, decompress with **both** (pins the decoder
///     against a fixed golden frame);
///   * compress **and** decompress inside each library and compare the
///     recovered plaintext plus both status codes (pins the pair end-to-end).
///
/// `dstCapacity` is the exact `frameContentSize`, which is the case where
/// `ZSTD_decompressFrame`'s final `(op-ostart) != fParams.frameContentSize`
/// check is meaningful, and `srcSize` 0/1/6/7 keep the `ZSTDbss_noCompress`
/// bt_raw blocks in the mix.
#[test]
fn round_trip_c_frame_and_self_frame() {
    covers(&["CFG:46", "CFG:80", "CFG:100"]);
    let small: &[usize] = &[0, 1, 6, 7, 100, 1024, 4096];
    let large: &[usize] = &[65536, 131071, 131072, 131073, 300000];
    let cases: Vec<(usize, c_int)> = small
        .iter()
        .flat_map(|&n| [-5, 1, 3, 9, 19, 22].iter().map(move |&l| (n, l)))
        .chain(
            large
                .iter()
                .flat_map(|&n| [-5, 1, 3, 9].iter().map(move |&l| (n, l))),
        )
        .collect();

    for (n, lvl) in cases {
        let cap = cbound(n);
        for &k in ALL_CORPORA {
            let src = corpus(k, n, seed_for(k, n));
            // (a) golden C frame, decompressed by both.
            let frame = c_compress(&src, lvl);
            let got = diff_bytes(&format!("ZSTD_decompress(Cframe) n={n} {k:?} lvl={lvl}"), |l| {
                decompress_simple(l, &frame, n)
            });
            assert_eq!(got.1 .0, src, "C frame did not round-trip (n={n} {k:?} lvl={lvl})");
            // (b) compress and decompress entirely within each library.
            diff_bytes(&format!("self round trip n={n} {k:?} lvl={lvl}"), |l| {
                let (rc, cf) = compress_simple(l, &src, lvl, cap);
                let (rd, out) = decompress_simple(l, &cf.0, n);
                (rc, rd, out)
            });
            // (c) the same through the context-taking variants.
            diff_bytes(&format!("DCtx round trip n={n} {k:?} lvl={lvl}"), |l| {
                let (rc, cf) = comp_cctx_full(l, &src, lvl, cap);
                let (rd, out) = decomp_dctx_full(l, &cf.0[..], n);
                (rc, rd, out)
            });
        }
    }
}

// ===========================================================================
// 3. Context reuse
// ===========================================================================

/// A *reused* `ZSTD_CCtx` / `ZSTD_DCtx` is a distinct code path from a fresh
/// one: `ZSTD_resetCCtx_internal` may keep the workspace and only re-init the
/// tables (`ZSTD_indexTooCloseToMax` / `ZSTDirp_reset` vs `ZSTDirp_continue`),
/// the match-state indices continue from the previous frame, and the entropy
/// repeat-tables live in `cctx->blockState.prevCBlock`. This drives 40
/// compressions of varying level/size/shape through **one** context and compares
/// every single output, then does the same for one DCtx.
///
/// It also pins the documented interaction of `ZSTD_compressCCtx` with sticky
/// advanced parameters: `ZSTD_compressCCtx` goes through
/// `ZSTD_CCtxParams_init_internal(&cctx->simpleApiParams, ...)` and therefore
/// must ignore everything set with `ZSTD_CCtx_setParameter`, while
/// `ZSTD_compress2` on the same context must still see them.
#[test]
fn compress_cctx_reuse_sequence() {
    covers(&["CFG:9", "CFG:71"]);
    let levels = [1, 3, -3, 9, 19, 0, 22, 2];
    let sizes = [0usize, 7, 100, 4096, 40000, 131072, 131073, 1];
    let mut plan: Vec<(usize, c_int, Corpus)> = Vec::new();
    let mut i = 0usize;
    for &n in &sizes {
        for &k in &[Corpus::Text, Corpus::Random, Corpus::Zeros, Corpus::LongRepeats, Corpus::Counter] {
            plan.push((n, levels[i % levels.len()], k));
            i += 1;
        }
    }
    assert_eq!(plan.len(), 40);
    let srcs: Vec<Vec<u8>> = plan.iter().map(|&(n, _, k)| corpus(k, n, seed_for(k, n))).collect();

    // 40 compressions on one CCtx, each output compared.
    diff_bytes("ZSTD_compressCCtx reuse x40", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
        let mut all: Vec<u8> = Vec::new();
        let mut rets: Vec<R> = Vec::new();
        for (idx, &(n, lvl, _)) in plan.iter().enumerate() {
            let cap = cbound(n);
            let mut dst = vec![0xCDu8; cap];
            let ret = unsafe {
                f(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    srcs[idx].as_ptr() as *const c_void,
                    n,
                    lvl,
                )
            };
            let r = res(l, ret);
            if let R::Ok(w) = r {
                dst.truncate(w);
            }
            rets.push(r);
            all.extend_from_slice(&dst);
        }
        (rets, Blob(all))
    });

    // The same 40 frames decompressed through one reused DCtx.
    let frames: Vec<Vec<u8>> = plan
        .iter()
        .enumerate()
        .map(|(idx, &(_, lvl, _))| c_compress(&srcs[idx], lvl))
        .collect();
    diff_bytes("ZSTD_decompressDCtx reuse x40", |l| {
        let dctx = Ctx::dctx(l);
        let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let mut all: Vec<u8> = Vec::new();
        let mut rets: Vec<R> = Vec::new();
        for (idx, fr) in frames.iter().enumerate() {
            let cap = plan[idx].0;
            let mut dst = vec![0xCDu8; cap];
            let ret = unsafe {
                f(
                    dctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    fr.as_ptr() as *const c_void,
                    fr.len(),
                )
            };
            let r = res(l, ret);
            if let R::Ok(w) = r {
                dst.truncate(w);
            }
            rets.push(r);
            all.extend_from_slice(&dst);
        }
        (rets, Blob(all))
    });

    // Sticky advanced params must not leak into ZSTD_compressCCtx, and must
    // survive it for the following ZSTD_compress2.
    let src = corpus(Corpus::Text, 300000, 42);
    let cap = cbound(src.len());
    diff_bytes("compressCCtx ignores sticky params, compress2 does not", |l| {
        let cctx = Ctx::cctx(l);
        let mut rets = Vec::new();
        rets.push(set_param(l, cctx.ptr, ZSTD_c_checksumFlag, 1));
        rets.push(set_param(l, cctx.ptr, ZSTD_c_windowLog, 10));
        rets.push(set_param(l, cctx.ptr, ZSTD_c_strategy, ZSTD_btultra2));
        let fc = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
        let mut a = vec![0xCDu8; cap];
        let ra = unsafe {
            fc(
                cctx.ptr,
                a.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                1,
            )
        };
        let ra = res(l, ra);
        if let R::Ok(w) = ra {
            a.truncate(w);
        }
        let f2 = l.sym::<FnCompress2>("ZSTD_compress2");
        let mut b = vec![0xCDu8; cap];
        let rb = unsafe {
            f2(
                cctx.ptr,
                b.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
            )
        };
        let rb = res(l, rb);
        if let R::Ok(w) = rb {
            b.truncate(w);
        }
        a.extend_from_slice(&b);
        ((rets, ra, rb), Blob(a))
    });
}

// ===========================================================================
// 4. ZSTD_compress2 x frame-shape parameters
// ===========================================================================

/// The full frame-shape cross-product through `ZSTD_compress2`:
/// `ZSTD_c_contentSizeFlag` {0,1} x `ZSTD_c_checksumFlag` {0,1} x
/// `ZSTD_c_dictIDFlag` {0,1} x `ZSTD_c_format` {zstd1, zstd1_magicless} x
/// four `ZSTD_CCtx_setPledgedSrcSize` modes x levels x corpora x sizes.
///
/// * `contentSizeFlag=0` forces **both** `fcsCode=0` and `singleSegment=0`, so
///   no content-size field is emitted at all and `ZSTD_getFrameContentSize` must
///   answer `ZSTD_CONTENTSIZE_UNKNOWN` while `ZSTD_getDecompressedSize` blends
///   that to 0.
/// * `checksumFlag` sets bit 2 of the frame-header descriptor, adds
///   `XXH64_update` per block and 4 epilogue bytes, and turns on the decoder's
///   `validateChecksum` stage.
/// * `dictIDFlag=0` (`noDictIDFlag`) forces `dictIDSizeCode = 0`; with no
///   dictionary loaded the dictID is already 0, so this must be a no-op — a
///   cheap but effective check that the polarity was not inverted.
/// * `format=ZSTD_f_zstd1_magicless` suppresses the 4 magic bytes in
///   `ZSTD_writeFrameHeader`, which shifts every subsequent field; such a frame
///   must then be *rejected* by `ZSTD_getFrameContentSize`/`ZSTD_isFrame`.
/// * `ZSTD_compress2` is `ZSTD_compressStream2(..., ZSTD_e_end)`, and
///   `ZSTD_CCtx_init_compressStream2` overwrites `pledgedSrcSizePlusOne` with
///   `inSize+1` for `ZSTD_e_end`; so an explicit pledge — exact, `UNKNOWN`, or
///   deliberately wrong — must produce byte-identical output to no pledge at all.
#[test]
fn compress2_frame_shape_matrix() {
    covers(&["CFG:41", "CFG:42", "CFG:45", "CFG:97"]);
    #[derive(Copy, Clone, Debug)]
    enum Pledge {
        None,
        Exact,
        Unknown,
        Wrong,
    }
    let pledges = [Pledge::None, Pledge::Exact, Pledge::Unknown, Pledge::Wrong];
    let corpora = [Corpus::Zeros, Corpus::Random, Corpus::Text];
    let sizes = [0usize, 1, 100, 1000, 5000];
    let levels = [1, 3, 9];

    for &csf in &[0, 1] {
        for &ck in &[0, 1] {
            for &did in &[0, 1] {
                for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                    for &pl in &pledges {
                        for &lvl in &levels {
                            for &k in &corpora {
                                for &n in &sizes {
                                    let src = corpus(k, n, seed_for(k, n));
                                    let cap = cbound(n);
                                    let label = format!(
                                        "ZSTD_compress2 csf={csf} ck={ck} did={did} fmt={fmt} pledge={pl:?} lvl={lvl} {k:?} n={n}"
                                    );
                                    let out = diff_bytes(&label, |l| {
                                        comp2_full(l, &src, cap, &|l, c| {
                                            let mut v = vec![
                                                set_param(l, c, ZSTD_c_compressionLevel, lvl),
                                                set_param(l, c, ZSTD_c_contentSizeFlag, csf),
                                                set_param(l, c, ZSTD_c_checksumFlag, ck),
                                                set_param(l, c, ZSTD_c_dictIDFlag, did),
                                                set_param(l, c, ZSTD_c_format, fmt),
                                            ];
                                            let sp =
                                                l.sym::<FnSetPledgedSrcSize>("ZSTD_CCtx_setPledgedSrcSize");
                                            match pl {
                                                Pledge::None => {}
                                                Pledge::Exact => {
                                                    v.push(res(l, unsafe { sp(c, n as c_ulonglong) }))
                                                }
                                                Pledge::Unknown => v.push(res(l, unsafe {
                                                    sp(c, ZSTD_CONTENTSIZE_UNKNOWN)
                                                })),
                                                Pledge::Wrong => v.push(res(l, unsafe {
                                                    sp(c, (n as c_ulonglong) + 1000)
                                                })),
                                            }
                                            v
                                        })
                                    });
                                    let (_, ret, buf) = out;
                                    let cSize = match ret {
                                        R::Ok(v) => v,
                                        R::Err(c, s) => panic!("{label}: compress2 failed {c}:{s}"),
                                    };
                                    let frame = buf.0[..cSize].to_vec();
                                    // Frame introspection on the produced frame.
                                    diff(&format!("{label} :: getFrameContentSize"), |l| {
                                        let g = l
                                            .sym::<FnU64FromBuf>("ZSTD_getFrameContentSize");
                                        let d = l.sym::<FnU64FromBuf>("ZSTD_getDecompressedSize");
                                        let i = l.sym::<FnUIntFromBuf>("ZSTD_isFrame");
                                        unsafe {
                                            (
                                                u64r(g(frame.as_ptr() as *const c_void, cSize)),
                                                u64r(d(frame.as_ptr() as *const c_void, cSize)),
                                                i(frame.as_ptr() as *const c_void, cSize),
                                            )
                                        }
                                    });
                                    if fmt == ZSTD_f_zstd1 {
                                        // Only a magic-carrying frame is decodable by the
                                        // format-agnostic one-shot decoder.
                                        diff_bytes(&format!("{label} :: decompress"), |l| {
                                            decompress_simple(l, &frame, n.max(1))
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Multi-block phase: the checksum is an XXH64 fed one block at a time
    // (`ZSTD_compress_frameChunk` -> `XXH64_update`) and then finalised in
    // `ZSTD_writeEpilogue`, so a single-block frame does not exercise the
    // streaming accumulation at all. 131071/131072/131073 straddle
    // `ZSTD_BLOCKSIZE_MAX` exactly, which is where an off-by-one in the update
    // loop would show up first.
    for &n in &[131071usize, 131072, 131073, 300000] {
        let cap = cbound(n);
        for &k in &[Corpus::Text, Corpus::Random, Corpus::OneByte] {
            let src = corpus(k, n, seed_for(k, n));
            for &csf in &[0, 1] {
                for &ck in &[0, 1] {
                    for &lvl in &[1, 3] {
                        let label = format!(
                            "ZSTD_compress2 multiblock n={n} {k:?} csf={csf} ck={ck} lvl={lvl}"
                        );
                        let (_, ret, buf) = diff_bytes(&label, |l| {
                            comp2_full(l, &src, cap, &|l, c| {
                                vec![
                                    set_param(l, c, ZSTD_c_compressionLevel, lvl),
                                    set_param(l, c, ZSTD_c_contentSizeFlag, csf),
                                    set_param(l, c, ZSTD_c_checksumFlag, ck),
                                ]
                            })
                        });
                        let cSize = match ret {
                            R::Ok(v) => v,
                            R::Err(a, s) => panic!("{label}: {a}:{s}"),
                        };
                        let frame = buf.0[..cSize].to_vec();
                        diff(&format!("{label} :: probe"), |l| probe(l, &frame));
                        let got = diff_bytes(&format!("{label} :: decompress"), |l| {
                            decompress_simple(l, &frame, n)
                        });
                        assert_eq!(got.1 .0, src, "{label}: content mismatch");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 5. frame-content-size field widths and singleSegment
// ===========================================================================

/// `ZSTD_writeFrameHeader` picks the width of the frame-content-size field from
/// `fcsCode = contentSizeFlag ? (pledged>=256) + (pledged>=65536+256) +
/// (pledged>=0xFFFFFFFF) : 0`, and suppresses the window-descriptor byte
/// entirely when `singleSegment = contentSizeFlag && (windowSize >= pledged)`.
///
/// Sizes 255/256/257 and 65791/65792/65793 sit exactly on the two reachable
/// `fcsCode` boundaries (255->1 byte, 256->2 bytes, 65792->4 bytes), 1024/1025
/// sit exactly on the `windowSize >= pledged` boundary for `windowLog=10`, and
/// `contentSizeFlag=0` is the only way to reach `fcsCode==0 && singleSegment==0`.
/// The test asserts afterwards that all three widths and both `singleSegment`
/// values were actually produced, so it cannot silently stop covering them.
///
/// `fcsCode == 3` (the 8-byte field) needs `pledgedSrcSize >= 0xFFFFFFFF`, which
/// is unreachable through `ZSTD_compress2` (`ZSTD_e_end` overwrites the pledge
/// with the real input size) and, when forced through the streaming API, is
/// rejected by `ZSTD_compressEnd_public` — it belongs to an ERRORS.md row.
#[test]
fn frame_content_size_field_widths_and_single_segment() {
    covers(&["CFG:37", "CFG:38", "CFG:39", "CFG:40", "CFG:41", "CFG:225"]);
    let sizes: &[usize] = &[255, 256, 257, 1023, 1024, 1025, 65791, 65792, 65793, 100000];
    let wlogs: &[Option<c_int>] = &[None, Some(10), Some(17), Some(20)];
    let corpora = [Corpus::Zeros, Corpus::Text, Corpus::Random];

    let mut seen_fcs: BTreeSet<u32> = BTreeSet::new();
    let mut seen_ss: BTreeSet<u32> = BTreeSet::new();

    for &n in sizes {
        let cap = cbound(n);
        for &wl in wlogs {
            for &csf in &[0, 1] {
                for &lvl in &[1, 3] {
                    for &k in &corpora {
                        let src = corpus(k, n, seed_for(k, n));
                        let label =
                            format!("fcs n={n} wlog={wl:?} csf={csf} lvl={lvl} {k:?}");
                        let (_, ret, buf) = diff_bytes(&label, |l| {
                            comp2_full(l, &src, cap, &|l, c| {
                                let mut v = vec![
                                    set_param(l, c, ZSTD_c_compressionLevel, lvl),
                                    set_param(l, c, ZSTD_c_contentSizeFlag, csf),
                                ];
                                if let Some(w) = wl {
                                    v.push(set_param(l, c, ZSTD_c_windowLog, w));
                                }
                                v
                            })
                        });
                        let cSize = match ret {
                            R::Ok(v) => v,
                            R::Err(c, s) => panic!("{label}: {c}:{s}"),
                        };
                        // Decode the frame-header descriptor byte the C wrote and
                        // record which encodings the sweep actually reached.
                        let fhd = buf.0[4];
                        seen_fcs.insert((fhd >> 6) as u32);
                        seen_ss.insert(((fhd >> 5) & 1) as u32);
                        let frame = buf.0[..cSize].to_vec();
                        diff(&format!("{label} :: header"), |l| {
                            let g = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
                            let mut zfh = poison_zfh();
                            let r = unsafe {
                                g(&mut zfh, frame.as_ptr() as *const c_void, frame.len())
                            };
                            let fcs = l.sym::<FnU64FromBuf>("ZSTD_getFrameContentSize");
                            let dsz = l.sym::<FnU64FromBuf>("ZSTD_getDecompressedSize");
                            let cs = l.sym::<FnFindFrameCompressedSize>(
                                "ZSTD_findFrameCompressedSize",
                            );
                            unsafe {
                                (
                                    res(l, r),
                                    zfh,
                                    u64r(fcs(frame.as_ptr() as *const c_void, frame.len())),
                                    u64r(dsz(frame.as_ptr() as *const c_void, frame.len())),
                                    res(l, cs(frame.as_ptr() as *const c_void, frame.len())),
                                )
                            }
                        });
                        diff_bytes(&format!("{label} :: round trip"), |l| {
                            decompress_simple(l, &frame, n.max(1))
                        });
                        if lvl == 3 && k == Corpus::Zeros {
                            // Every prefix of a header of this exact shape: the
                            // "wanted srcSize" ladder from ZSTD_getFrameHeader is
                            // 5 -> ZSTD_frameHeaderSize_internal(6..18) -> 0, and
                            // the value depends on precisely the fcsCode /
                            // singleSegment / dictID field widths this cell built.
                            for cut in 0..=frame.len().min(20) {
                                diff(&format!("{label} :: probe cut={cut}"), |l| {
                                    probe(l, &frame[..cut])
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        seen_fcs,
        BTreeSet::from([0u32, 1, 2]),
        "the sweep no longer reaches every reachable fcsCode"
    );
    assert_eq!(
        seen_ss,
        BTreeSet::from([0u32, 1]),
        "the sweep no longer reaches both singleSegment values"
    );
}

// ===========================================================================
// 6. ZSTD_compress_advanced with explicit ZSTD_parameters
// ===========================================================================

/// `ZSTD_getParams` must agree field-for-field first (it memsets the whole
/// `ZSTD_parameters` and then sets **only** `fParams.contentSizeFlag = 1`), then
/// `ZSTD_compress_advanced` is driven with those parameters and with each field
/// individually mutated.
///
/// `ZSTD_compress_advanced` is *not* the advanced-parameter path: it runs
/// `ZSTD_checkCParams` (bounds only), `ZSTD_CCtxParams_init_internal(...,
/// ZSTD_NO_CLEVEL)` — which makes `ZSTD_resolveExternalRepcodeSearch` pick
/// `ps_disable` — and then `ZSTD_compressBegin_internal` with `ZSTD_dct_auto` +
/// `ZSTD_dtlm_fast`. Explicit `cParams` still re-enter
/// `ZSTD_adjustCParams_internal`, so windowLog/hashLog/chainLog get downsized
/// against the real `srcSize` and the row-matchfinder / block-splitter / LDM
/// auto rules are re-resolved from the mutated strategy and windowLog.
///
/// Mutations stay inside `ZSTD_checkCParams`' bounds (out-of-bounds values are
/// an ERRORS.md row) and respect the header's documented restriction that
/// `minMatch==7` is for `ZSTD_fast` and `minMatch==3` for `btopt+` only.
#[test]
fn compress_advanced_explicit_parameters() {
    covers(&["CFG:153", "CFG:181"]);

    // (a) ZSTD_getParams itself, over a level x srcSizeHint x dictSize grid.
    for &lvl in &[-131072, -22, -5, -1, 0, 1, 3, 9, 17, 19, 22, 23, 100] {
        for &hint in &[0u64, 1, 512, 513, 1024, 16384, 16385, 131072, 262144, 262145, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
            for &ds in &[0usize, 1, 1024, 65536, 1 << 20] {
                diff(&format!("ZSTD_getParams lvl={lvl} hint={hint} dict={ds}"), |l| {
                    let f = l.sym::<FnGetParams>("ZSTD_getParams");
                    unsafe { f(lvl, hint, ds) }
                });
            }
        }
    }

    // (b) ZSTD_compress_advanced with the base params and mutations of each.
    #[derive(Copy, Clone, Debug)]
    enum Mut {
        None,
        F(c_int, c_int, c_int),
        WindowLog(c_uint),
        HashLog(c_uint),
        ChainLog(c_uint),
        SearchLog(c_uint),
        MinMatch(c_uint, c_int),
        TargetLength(c_uint),
        Strategy(c_int),
    }
    let mut muts: Vec<Mut> = vec![Mut::None];
    for &a in &[0, 1] {
        for &b in &[0, 1] {
            for &c in &[0, 1] {
                muts.push(Mut::F(a, b, c));
            }
        }
    }
    for &w in &[10u32, 12, 15, 20] {
        muts.push(Mut::WindowLog(w));
    }
    for &h in &[6u32, 12, 20] {
        muts.push(Mut::HashLog(h));
    }
    for &c in &[6u32, 12, 20] {
        muts.push(Mut::ChainLog(c));
    }
    for &s in &[1u32, 4, 8] {
        muts.push(Mut::SearchLog(s));
    }
    for &m in &[4u32, 5, 6] {
        muts.push(Mut::MinMatch(m, 0));
    }
    muts.push(Mut::MinMatch(3, ZSTD_btopt));
    muts.push(Mut::MinMatch(7, ZSTD_fast));
    for &t in &[0u32, 32, 999, 131072] {
        muts.push(Mut::TargetLength(t));
    }
    for &s in ALL_STRATEGIES {
        muts.push(Mut::Strategy(s));
    }

    let dict_raw = corpus(Corpus::Text, 4096, 0xD1C7);
    assert_ne!(
        u32::from_le_bytes([dict_raw[0], dict_raw[1], dict_raw[2], dict_raw[3]]),
        ZSTD_MAGIC_DICTIONARY,
        "the raw dictionary must not look like a ZSTD_MAGIC_DICTIONARY dictionary"
    );

    for &(n, lvl) in &[(0usize, 3), (1, 3), (1000, 1), (1000, 19), (20000, 3), (20000, 19), (300000, 3)] {
        let cap = cbound(n);
        for &k in &[Corpus::Text, Corpus::Random, Corpus::LongRepeats] {
            let src = corpus(k, n, seed_for(k, n));
            for &use_dict in &[false, true] {
                let dict: &[u8] = if use_dict { &dict_raw } else { &[] };
                for &m in &muts {
                    let label = format!(
                        "ZSTD_compress_advanced n={n} lvl={lvl} {k:?} dict={} {m:?}",
                        dict.len()
                    );
                    diff_bytes(&label, |l| {
                        let gp = l.sym::<FnGetParams>("ZSTD_getParams");
                        let mut p = unsafe { gp(lvl, n as c_ulonglong, dict.len()) };
                        match m {
                            Mut::None => {}
                            Mut::F(a, b, c) => {
                                p.fParams.contentSizeFlag = a;
                                p.fParams.checksumFlag = b;
                                p.fParams.noDictIDFlag = c;
                            }
                            Mut::WindowLog(v) => p.cParams.windowLog = v,
                            Mut::HashLog(v) => p.cParams.hashLog = v,
                            Mut::ChainLog(v) => p.cParams.chainLog = v,
                            Mut::SearchLog(v) => p.cParams.searchLog = v,
                            Mut::MinMatch(v, s) => {
                                p.cParams.minMatch = v;
                                if s != 0 {
                                    p.cParams.strategy = s;
                                }
                            }
                            Mut::TargetLength(v) => p.cParams.targetLength = v,
                            Mut::Strategy(v) => p.cParams.strategy = v,
                        }
                        let cctx = Ctx::cctx(l);
                        let f = l.sym::<FnCompressAdvanced>("ZSTD_compress_advanced");
                        let mut dst = vec![0xCDu8; cap];
                        let ret = unsafe {
                            f(
                                cctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                src.len(),
                                if dict.is_empty() {
                                    std::ptr::null()
                                } else {
                                    dict.as_ptr() as *const c_void
                                },
                                dict.len(),
                                p,
                            )
                        };
                        let r = res(l, ret);
                        if let R::Ok(w) = r {
                            dst.truncate(w);
                        }
                        (r, Blob(dst))
                    });
                }
            }
        }
    }
}

// ===========================================================================
// 7. ZSTD_compress_usingDict / ZSTD_decompress_usingDict, raw content
// ===========================================================================

/// The simple dictionary API with **raw-content** dictionaries only (the
/// `ZSTD_MAGIC_DICTIONARY` branch and trained dictionaries belong to the
/// dictionary-builder rows).
///
/// `ZSTD_compress_insertDictionary` treats `dict == NULL || dictSize < 8` as
/// "no dictionary" and returns dictID 0 without loading anything, so dict sizes
/// 0/1 must produce exactly the dict-free frame while 8/1000/100000 go through
/// `ZSTD_loadDictionaryContent` — including its `srcSize <= HASH_READ_SIZE (8)`
/// early exit at exactly 8 and the per-strategy table-fill switch. Note also
/// that `ZSTD_compress_usingDict` passes `dict ? dictSize : 0` into
/// `ZSTD_getParams_internal`, so a non-NULL 0-length dictionary and a NULL one
/// take *different* argument paths to the same cParams row.
#[test]
fn compress_and_decompress_using_raw_dict() {
    covers(&["CFG:72"]);
    let dict_pool = corpus(Corpus::Text, 100_000, 0x11D1C7);
    assert_ne!(
        u32::from_le_bytes([dict_pool[0], dict_pool[1], dict_pool[2], dict_pool[3]]),
        ZSTD_MAGIC_DICTIONARY
    );
    let dict_sizes = [0usize, 1, 8, 1000, 100_000];

    for &ds in &dict_sizes {
        let dict = &dict_pool[..ds];
        for &null_dict in &[false, true] {
            if null_dict && ds != 0 {
                continue; // NULL is only meaningful with size 0
            }
            for &n in &[0usize, 1, 100, 1000, 20000] {
                let cap = cbound(n);
                for &lvl in &[1, 3, 9, 19] {
                    if lvl == 19 && ds > 1000 {
                        continue; // btultra2 over a 100 KB dict: runtime only
                    }
                    for &k in &[Corpus::Text, Corpus::Random] {
                        let src = corpus(k, n, seed_for(k, n));
                        let label = format!(
                            "ZSTD_compress_usingDict ds={ds} null={null_dict} n={n} lvl={lvl} {k:?}"
                        );
                        let (_, frame) = diff_bytes(&label, |l| {
                            let cctx = Ctx::cctx(l);
                            let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
                            let mut dst = vec![0xCDu8; cap];
                            let ret = unsafe {
                                f(
                                    cctx.ptr,
                                    dst.as_mut_ptr() as *mut c_void,
                                    cap,
                                    src.as_ptr() as *const c_void,
                                    src.len(),
                                    if null_dict {
                                        std::ptr::null()
                                    } else {
                                        dict.as_ptr() as *const c_void
                                    },
                                    ds,
                                    lvl,
                                )
                            };
                            let r = res(l, ret);
                            if let R::Ok(w) = r {
                                dst.truncate(w);
                            }
                            (r, Blob(dst))
                        });
                        // decompress_usingDict with the same dictionary, and (since
                        // the frame carries no dictID) also with none at all.
                        for &(dp, dl, tag) in &[(true, ds, "same-dict"), (false, 0usize, "no-dict")] {
                            if !dp && ds >= 8 && n > 0 {
                                continue; // referencing dict content: needs the dict
                            }
                            diff_bytes(&format!("{label} :: decompress {tag}"), |l| {
                                let dctx = Ctx::dctx(l);
                                let f =
                                    l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
                                let mut dst = vec![0xCDu8; n.max(1)];
                                let ret = unsafe {
                                    f(
                                        dctx.ptr,
                                        dst.as_mut_ptr() as *mut c_void,
                                        n.max(1),
                                        frame.0.as_ptr() as *const c_void,
                                        frame.0.len(),
                                        if dp && !null_dict {
                                            dict.as_ptr() as *const c_void
                                        } else {
                                            std::ptr::null()
                                        },
                                        dl,
                                    )
                                };
                                (res(l, ret), Blob(dst))
                            });
                        }
                    }
                }
            }
        }
    }

    // One large body with the big dictionary, so the dict actually shifts the
    // cParams table row (`rSize = srcSize + dictSize`) and the match finder has
    // real cross-dictionary matches to find.
    let src = corpus(Corpus::Text, 300_000, 0x515);
    let cap = cbound(src.len());
    for &lvl in &[1, 3, 9] {
        let label = format!("ZSTD_compress_usingDict big n=300000 lvl={lvl}");
        let (_, frame) = diff_bytes(&label, |l| {
            let cctx = Ctx::cctx(l);
            let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
            let mut dst = vec![0xCDu8; cap];
            let ret = unsafe {
                f(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    dict_pool.as_ptr() as *const c_void,
                    dict_pool.len(),
                    lvl,
                )
            };
            let r = res(l, ret);
            if let R::Ok(w) = r {
                dst.truncate(w);
            }
            (r, Blob(dst))
        });
        diff_bytes(&format!("{label} :: decompress"), |l| {
            let dctx = Ctx::dctx(l);
            let f = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
            let mut dst = vec![0xCDu8; src.len()];
            let ret = unsafe {
                f(
                    dctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    src.len(),
                    frame.0.as_ptr() as *const c_void,
                    frame.0.len(),
                    dict_pool.as_ptr() as *const c_void,
                    dict_pool.len(),
                )
            };
            (res(l, ret), Blob(dst))
        });
    }
}

// ===========================================================================
// 8. dstCapacity on the valid side
// ===========================================================================

/// The valid side of the `dstCapacity` axis: exactly `ZSTD_compressBound(n)`,
/// `bound+1`, `bound+4096` and a generously oversized buffer, for
/// `ZSTD_compress`, `ZSTD_compressCCtx` and `ZSTD_compress2`.
///
/// The whole destination buffer is compared here (`comp_full`, no truncation),
/// so a library that writes scratch bytes past the returned `cSize` — or fails
/// to leave the tail of an oversized buffer untouched — is caught. `bound+1` and
/// `bound+4096` matter because `ZSTD_compress_frameChunk` and
/// `ZSTD_writeEpilogue` compare the *remaining* capacity against their own
/// thresholds at every step, so a bigger buffer can change which branch runs.
#[test]
fn dst_capacity_valid_side() {
    covers(&["CFG:98"]);
    for &n in &[0usize, 1, 6, 7, 100, 1000, 65536, 300000] {
        let b = cbound(n);
        for &lvl in &[1, 3, 19] {
            for &k in &[Corpus::Text, Corpus::Random, Corpus::Zeros] {
                let src = corpus(k, n, seed_for(k, n));
                // `exact` is the size the frame really needs; it is the tightest
                // capacity that can possibly work, and for small inputs it is
                // *below* ZSTD_FRAMEHEADERSIZE_MAX so ZSTD_writeFrameHeader
                // refuses it up front — both libraries must agree either way.
                let exact = match compress_simple(&pair().c, &src, lvl, b).0 {
                    R::Ok(v) => v,
                    R::Err(a, s) => panic!("fixture compress failed {a}:{s}"),
                };
                let mut caps = vec![exact, b, b + 1, b + 4096];
                if n <= 1000 {
                    caps.push(b * 10);
                } else {
                    caps.push(b + 1_000_000);
                }
                for &cap in &caps {
                    let label = format!("dstCapacity n={n} cap={cap} lvl={lvl} {k:?}");
                    diff_bytes(&format!("{label} :: ZSTD_compress"), |l| {
                        comp_full(l, &src, lvl, cap)
                    });
                    diff_bytes(&format!("{label} :: ZSTD_compressCCtx"), |l| {
                        comp_cctx_full(l, &src, lvl, cap)
                    });
                    diff_bytes(&format!("{label} :: ZSTD_compress2"), |l| {
                        comp2_full(l, &src, cap, &|l, c| {
                            vec![set_param(l, c, ZSTD_c_compressionLevel, lvl)]
                        })
                    });
                }
            }
        }
    }

    // The capacities *below* the four independent one-shot guards, so that each
    // reports at a different point: `ZSTD_writeFrameHeader` refuses anything
    // under ZSTD_FRAMEHEADERSIZE_MAX (18) even for a 1-byte frame,
    // `ZSTD_compress_frameChunk` refuses < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE
    // + 1 (6), and `ZSTD_writeEpilogue` refuses < 3 for the terminal block and
    // < 4 for the checksum. Each call gets a buffer far larger than the capacity
    // it advertises, so a library that writes past `dstCapacity` before noticing
    // is caught rather than merely returning the right code.
    for &n in &[0usize, 1, 6, 100, 300000] {
        for &lvl in &[1, 19] {
            let src = corpus(Corpus::Text, n, seed_for(Corpus::Text, n));
            for &cap in &[0usize, 1, 2, 3, 5, 6, 9, 17, 18, 19, 20] {
                diff_bytes(&format!("tiny dstCapacity n={n} cap={cap} lvl={lvl}"), |l| {
                    comp_full_cap(l, &src, lvl, cap, 64)
                });
            }
        }
    }

    // The decoder side of the same axis: capacity exactly the content size, one
    // more, far more, and (for a known content size) the oversized case where
    // ZSTD_decompressFrame's `oend` is far past the real end.
    for &n in &[0usize, 1, 6, 131072, 300000] {
        let src = corpus(Corpus::Text, n, seed_for(Corpus::Text, n));
        let frame = c_compress(&src, 3);
        for &cap in &[n, n + 1, n + 4096] {
            diff_bytes(&format!("decompress n={n} cap={cap}"), |l| {
                decompress_simple(l, &frame, cap)
            });
            diff_bytes(&format!("decompressDCtx n={n} cap={cap}"), |l| {
                decomp_dctx_full(l, &frame, cap)
            });
        }
    }
}

// ===========================================================================
// 9. Frame introspection on whole frames and on every prefix
// ===========================================================================

/// Everything a caller can ask about a frame *without* decoding it, evaluated on
/// a whole frame and on **every prefix** of it (0..=32 bytes plus a few longer
/// cuts). The partial-input return values are part of the contract:
/// `ZSTD_getFrameHeader` returns 5 while `srcSize < ZSTD_startingInputLength`,
/// then the exact `ZSTD_frameHeaderSize_internal` value (6..18) while the header
/// is incomplete, then 0; `ZSTD_getFrameContentSize` collapses every one of
/// those into `ZSTD_CONTENTSIZE_ERROR`; `ZSTD_getDecompressedSize` collapses
/// both sentinels to 0; `ZSTD_getDictID_fromFrame` swallows all errors into 0;
/// `ZSTD_findFrameCompressedSize` walks every block header and only then adds
/// the 4 checksum bytes.
///
/// The out-param struct is pre-poisoned with 0x5A, so "did the callee memset
/// `zfhPtr` on this path?" is compared too — the C only memsets it *after* the
/// short-input early return.
#[test]
fn frame_introspection_whole_and_prefixes() {
    covers(&["CFG:78", "CFG:82", "CFG:83", "CFG:225", "CFG:226", "CFG:231", "CFG:232"]);
    let frames = build_frame_zoo();

    for (name, buf) in &frames {
        let mut cuts: Vec<usize> = (0..=buf.len().min(32)).collect();
        for extra in [buf.len() / 2, buf.len().saturating_sub(5), buf.len().saturating_sub(1), buf.len()] {
            cuts.push(extra);
        }
        cuts.sort_unstable();
        cuts.dedup();
        for &cut in &cuts {
            let s = &buf[..cut];
            diff(&format!("probe {name} cut={cut}/{}", buf.len()), |l| probe(l, s));
        }
    }

    // All 16 skippable magics with content sizes {0, 1, 100, 0xFFFFFFFF} at the
    // srcSizes that straddle ZSTD_SKIPPABLEHEADERSIZE: the skippable branch of
    // `ZSTD_getFrameHeader_advanced` returns 8 while srcSize < 8 and then fills
    // `frameType = ZSTD_skippableFrame`, `dictID = magic -
    // ZSTD_MAGIC_SKIPPABLE_START` (0..15), `headerSize = 8` and
    // `frameContentSize = MEM_readLE32(src+4)`, leaving windowSize / blockSizeMax
    // / checksumFlag at 0. The 0xFFFFFFFF length additionally trips
    // `readSkippableFrameSize`'s `(U32)(sizeU32+8) < sizeU32` overflow guard.
    for variant in 0..16u32 {
        for &content in &[0u32, 1, 100, 0xFFFF_FFFF] {
            let mut buf = vec![0x77u8; 8 + 100];
            buf[..4].copy_from_slice(&(ZSTD_MAGIC_SKIPPABLE_START + variant).to_le_bytes());
            buf[4..8].copy_from_slice(&content.to_le_bytes());
            for &sz in &[0usize, 4, 5, 7, 8, 9, 12, 108] {
                diff(
                    &format!("skippable magic v={variant} content={content} size={sz}"),
                    |l| probe(l, &buf[..sz]),
                );
            }
        }
    }

    // ZSTD_isFrame / ZSTD_isSkippableFrame over the magic space: the zstd magic,
    // all 16 skippable magics, the two just outside the skippable range, and the
    // legacy magics (ZSTD_LEGACY_SUPPORT==5 makes v05/v06/v07 count as frames
    // for ZSTD_isFrame but never for ZSTD_isSkippableFrame).
    let mut magics: Vec<u32> = vec![
        ZSTD_MAGICNUMBER,
        0x184D_2A4F,
        0x184D_2A60,
        0xFD2F_B51E, // v0.1 (stored big-endian in the file, so this LE value is not it)
        0xFD2F_B522,
        0xFD2F_B523,
        0xFD2F_B524,
        0xFD2F_B525,
        0xFD2F_B526,
        0xFD2F_B527,
        0x0000_0000,
        0xFFFF_FFFF,
        0xAAAA_AAAA,
    ];
    for v in 0..16u32 {
        magics.push(ZSTD_MAGIC_SKIPPABLE_START + v);
    }
    for &m in &magics {
        let mut buf = vec![0u8; 16];
        buf[..4].copy_from_slice(&m.to_le_bytes());
        for &sz in &[0usize, 1, 2, 3, 4, 5, 8, 16] {
            diff(&format!("ZSTD_isFrame magic={m:08x} size={sz}"), |l| {
                let a = l.sym::<FnUIntFromBuf>("ZSTD_isFrame");
                let b = l.sym::<FnUIntFromBuf>("ZSTD_isSkippableFrame");
                unsafe {
                    (
                        a(buf.as_ptr() as *const c_void, sz),
                        b(buf.as_ptr() as *const c_void, sz),
                    )
                }
            });
        }
    }

    // The three sibling dictID queries on the inputs their two guards branch on
    // (`dictSize < 8`, `MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY`). No entropy
    // tables are parsed by ZSTD_getDictID_fromDict, so an 8-byte magic+dictID
    // buffer is a perfectly legal input to it.
    let mut dictbuf = vec![0u8; 16];
    dictbuf[..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
    for &id in &[0u32, 1, 255, 256, 65535, 65536, 0xFFFF_FFFF] {
        dictbuf[4..8].copy_from_slice(&id.to_le_bytes());
        for &sz in &[0usize, 1, 4, 7, 8, 16] {
            diff(&format!("ZSTD_getDictID_fromDict id={id} size={sz}"), |l| {
                let f = l.sym::<FnUIntFromBuf>("ZSTD_getDictID_fromDict");
                unsafe { f(dictbuf.as_ptr() as *const c_void, sz) }
            });
        }
    }
    let zeros8 = vec![0u8; 8];
    for &sz in &[0usize, 1, 7, 8] {
        diff(&format!("ZSTD_getDictID_fromDict zeros size={sz}"), |l| {
            let f = l.sym::<FnUIntFromBuf>("ZSTD_getDictID_fromDict");
            unsafe { f(zeros8.as_ptr() as *const c_void, sz) }
        });
    }
    diff("ZSTD_getDictID_fromDict NULL/0", |l| {
        let f = l.sym::<FnUIntFromBuf>("ZSTD_getDictID_fromDict");
        unsafe { f(std::ptr::null(), 0) }
    });
    // A raw-content CDict/DDict has dictID 0; NULL is documented as 0 too.
    let raw = corpus(Corpus::Text, 4096, 0x9);
    diff("ZSTD_getDictID_fromCDict / fromDDict", |l| {
        let cc = l.sym::<FnCreateCDict>("ZSTD_createCDict");
        let cd = l.sym::<FnCreateDDict>("ZSTD_createDDict");
        let gc = l.sym::<FnPtrToUInt>("ZSTD_getDictID_fromCDict");
        let gd = l.sym::<FnPtrToUInt>("ZSTD_getDictID_fromDDict");
        unsafe {
            let c1 = cc(raw.as_ptr() as *const c_void, raw.len(), 3);
            let d1 = cd(raw.as_ptr() as *const c_void, raw.len());
            let out = (
                gc(c1),
                gd(d1),
                gc(std::ptr::null()),
                gd(std::ptr::null()),
            );
            let _ = Ctx::from_raw(l, c1, "ZSTD_freeCDict");
            let _ = Ctx::from_raw(l, d1, "ZSTD_freeDDict");
            out
        }
    });
}

/// Everything observable about a (possibly truncated) frame, in one comparable
/// bundle so a divergence names all of it at once.
#[derive(PartialEq, Debug)]
struct Probe {
    zfhRet: R,
    zfh: ZSTD_FrameHeader,
    fcs: U64R,
    decompressedSize: U64R,
    compressedSize: R,
    findDecompressedSize: U64R,
    decompressBound: U64R,
    dictID: c_uint,
    isFrame: c_uint,
    isSkippableFrame: c_uint,
}

fn probe(l: &Lib, s: &[u8]) -> Probe {
    let p = s.as_ptr() as *const c_void;
    let n = s.len();
    let mut zfh = poison_zfh();
    unsafe {
        let zfhRet = res(l, l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader")(&mut zfh, p, n));
        Probe {
            zfhRet,
            zfh,
            fcs: u64r(l.sym::<FnU64FromBuf>("ZSTD_getFrameContentSize")(p, n)),
            decompressedSize: u64r(l.sym::<FnU64FromBuf>("ZSTD_getDecompressedSize")(p, n)),
            compressedSize: res(
                l,
                l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize")(p, n),
            ),
            findDecompressedSize: u64r(l.sym::<FnU64FromBuf>("ZSTD_findDecompressedSize")(p, n)),
            decompressBound: u64r(l.sym::<FnU64FromBuf>("ZSTD_decompressBound")(p, n)),
            dictID: l.sym::<FnUIntFromBuf>("ZSTD_getDictID_fromFrame")(p, n),
            isFrame: l.sym::<FnUIntFromBuf>("ZSTD_isFrame")(p, n),
            isSkippableFrame: l.sym::<FnUIntFromBuf>("ZSTD_isSkippableFrame")(p, n),
        }
    }
}

/// A set of frames covering every reachable header shape plus the non-frames the
/// introspection functions have to reject.
fn build_frame_zoo() -> Vec<(String, Vec<u8>)> {
    let c = &pair().c;
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();

    // fcsCode 0 (1-byte, singleSegment), 1 (LE16), 2 (LE32).
    v.push(("fcs0/singleSegment".into(), c_compress(&corpus(Corpus::Text, 200, 1), 3)));
    v.push(("fcs1".into(), c_compress(&corpus(Corpus::Text, 1000, 2), 3)));
    v.push(("fcs2".into(), c_compress(&corpus(Corpus::Text, 70000, 3), 3)));
    // Empty frame.
    v.push(("empty".into(), c_compress(&[], 3)));
    // Multi-block, and an all-one-byte input whose later blocks become bt_rle.
    v.push(("multiblock".into(), c_compress(&corpus(Corpus::Text, 600_000, 4), 1)));
    v.push(("rle-blocks".into(), c_compress(&corpus(Corpus::OneByte, 262_144, 5), 3)));

    // contentSizeFlag=0 (no size field), checksumFlag=1, and windowLog=10 so
    // singleSegment is 0 and a window-descriptor byte is emitted.
    let mk = |set: &dyn Fn(&Lib, *mut c_void) -> Vec<R>, src: &[u8]| -> Vec<u8> {
        let cap = cbound(src.len());
        let (_, r, b) = comp2_full(c, src, cap, set);
        match r {
            R::Ok(k) => b.0[..k].to_vec(),
            R::Err(a, s) => panic!("fixture compress2 failed: {a}:{s}"),
        }
    };
    let body = corpus(Corpus::Text, 5000, 6);
    v.push((
        "no-contentSize".into(),
        mk(&|l, x| vec![set_param(l, x, ZSTD_c_contentSizeFlag, 0)], &body),
    ));
    v.push((
        "checksum".into(),
        mk(&|l, x| vec![set_param(l, x, ZSTD_c_checksumFlag, 1)], &body),
    ));
    v.push((
        "wlog10-multiSegment".into(),
        mk(&|l, x| vec![set_param(l, x, ZSTD_c_windowLog, 10)], &body),
    ));
    v.push((
        "wlog10+checksum+noContentSize".into(),
        mk(
            &|l, x| {
                vec![
                    set_param(l, x, ZSTD_c_windowLog, 10),
                    set_param(l, x, ZSTD_c_checksumFlag, 1),
                    set_param(l, x, ZSTD_c_contentSizeFlag, 0),
                ]
            },
            &body,
        ),
    ));
    v.push((
        "magicless".into(),
        mk(&|l, x| vec![set_param(l, x, ZSTD_c_format, ZSTD_f_zstd1_magicless)], &body),
    ));

    // Skippable frames: empty content, 100 bytes of content, and a hand-written
    // one whose length field is 0xFFFFFFFF (readSkippableFrameSize's overflow
    // guard, then its `skippableSize > srcSize` guard).
    for &(len, variant) in &[(0usize, 0u32), (100, 7), (100, 15)] {
        let payload = corpus(Corpus::Counter, len, 7);
        let mut dst = vec![0u8; len + 32];
        let w = c.sym::<FnWriteSkippableFrame>("ZSTD_writeSkippableFrame");
        let got = unsafe {
            w(
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                payload.as_ptr() as *const c_void,
                len,
                variant,
            )
        };
        assert!(!is_error(c, got), "ZSTD_writeSkippableFrame fixture failed");
        dst.truncate(got);
        v.push((format!("skippable len={len} variant={variant}"), dst));
    }
    let mut bad_skip = vec![0u8; 24];
    bad_skip[..4].copy_from_slice(&ZSTD_MAGIC_SKIPPABLE_START.to_le_bytes());
    bad_skip[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    v.push(("skippable len=0xFFFFFFFF".into(), bad_skip));

    // Non-frames and near-frames.
    v.push(("garbage-0xAA".into(), vec![0xAAu8; 24]));
    v.push(("magic-only".into(), ZSTD_MAGICNUMBER.to_le_bytes().to_vec()));
    let mut trailing = c_compress(&corpus(Corpus::Text, 1000, 8), 3);
    let cut = trailing.len();
    trailing.extend_from_slice(&[0x11u8; 100]);
    v.push((format!("frame+100 trailing (frame ends at {cut})"), trailing));
    v
}

// ===========================================================================
// 10. Multi-frame concatenation
// ===========================================================================

/// `ZSTD_decompress` loops over frames (`ZSTD_decompressMultiFrame`), so 2..5
/// frames back to back — with different levels, checksum flags, content-size
/// flags and corpora, and with a skippable frame injected before / between /
/// after them — exercise the loop condition
/// (`srcSize >= ZSTD_startingInputLength(format)`), the skippable branch with
/// `readSkippableFrameSize`, the `dstCapacity -= res` bookkeeping, and the final
/// `RETURN_ERROR_IF(srcSize, srcSize_wrong)` for unconsumed input.
///
/// The same buffers are then measured with `ZSTD_findDecompressedSize` (which
/// answers `ZSTD_CONTENTSIZE_UNKNOWN` as soon as one frame lacks the field, and
/// `ZSTD_CONTENTSIZE_ERROR` for trailing bytes) and `ZSTD_decompressBound`
/// (which instead falls back to `nbBlocks * zfh.blockSizeMax` and never returns
/// UNKNOWN) — the two functions deliberately disagree on the unknown-size case.
#[test]
fn multi_frame_concatenation() {
    covers(&["CFG:81", "CFG:229"]);
    let c = &pair().c;
    let skippable = |len: usize, variant: u32| -> Vec<u8> {
        let payload = corpus(Corpus::Counter, len, 3);
        let mut dst = vec![0u8; len + 32];
        let w = c.sym::<FnWriteSkippableFrame>("ZSTD_writeSkippableFrame");
        let got = unsafe {
            w(
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                payload.as_ptr() as *const c_void,
                len,
                variant,
            )
        };
        assert!(!is_error(c, got));
        dst.truncate(got);
        dst
    };

    // Per-frame recipes: (size, level, checksum, contentSizeFlag, corpus).
    let recipes: Vec<(usize, c_int, c_int, c_int, Corpus)> = vec![
        (1000, 3, 0, 1, Corpus::Text),
        (0, 1, 1, 1, Corpus::Zeros),
        (65536, 1, 0, 1, Corpus::Random),
        (7, 9, 1, 0, Corpus::Counter),
        (200_000, 1, 1, 1, Corpus::LongRepeats),
    ];
    let built: Vec<(Vec<u8>, Vec<u8>)> = recipes
        .iter()
        .map(|&(n, lvl, ck, csf, k)| {
            let src = corpus(k, n, seed_for(k, n));
            let cap = cbound(n);
            let (_, r, b) = comp2_full(c, &src, cap, &|l, x| {
                vec![
                    set_param(l, x, ZSTD_c_compressionLevel, lvl),
                    set_param(l, x, ZSTD_c_checksumFlag, ck),
                    set_param(l, x, ZSTD_c_contentSizeFlag, csf),
                ]
            });
            let cs = match r {
                R::Ok(k) => k,
                R::Err(a, s) => panic!("fixture failed {a}:{s}"),
            };
            (src, b.0[..cs].to_vec())
        })
        .collect();

    // Every prefix-length 2..=5 of the recipe list, and each with a skippable
    // frame at the front, in the middle and at the back.
    for nframes in 2..=5usize {
        for &skip_at in &[usize::MAX, 0, 1, nframes] {
            let mut cat: Vec<u8> = Vec::new();
            let mut plain: Vec<u8> = Vec::new();
            for i in 0..nframes {
                if skip_at == i {
                    cat.extend_from_slice(&skippable(if i % 2 == 0 { 0 } else { 100 }, i as u32));
                }
                cat.extend_from_slice(&built[i].1);
                plain.extend_from_slice(&built[i].0);
            }
            if skip_at == nframes {
                cat.extend_from_slice(&skippable(37, 3));
            }
            let label = format!("multiframe n={nframes} skip_at={skip_at}");
            let got = diff_bytes(&format!("{label} :: ZSTD_decompress"), |l| {
                decompress_simple(l, &cat, plain.len().max(1))
            });
            if let R::Ok(_) = got.0 {
                assert_eq!(got.1 .0, plain, "{label}: multi-frame content mismatch");
            }
            diff_bytes(&format!("{label} :: ZSTD_decompressDCtx"), |l| {
                decomp_dctx_full(l, &cat, plain.len().max(1))
            });
            diff(&format!("{label} :: sizes"), |l| {
                let p = cat.as_ptr() as *const c_void;
                unsafe {
                    (
                        u64r(l.sym::<FnU64FromBuf>("ZSTD_findDecompressedSize")(p, cat.len())),
                        u64r(l.sym::<FnU64FromBuf>("ZSTD_decompressBound")(p, cat.len())),
                        res(
                            l,
                            l.sym::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize")(
                                p,
                                cat.len(),
                            ),
                        ),
                        u64r(l.sym::<FnU64FromBuf>("ZSTD_getFrameContentSize")(p, cat.len())),
                    )
                }
            });
            // Trailing garbage and a truncated tail: both must be reported
            // identically by all three size queries and by the decoder.
            for &extra in &[1usize, 4, 5] {
                let mut t = cat.clone();
                t.extend_from_slice(&[0x5Au8; 5][..extra]);
                diff(&format!("{label} :: +{extra} trailing"), |l| {
                    let p = t.as_ptr() as *const c_void;
                    unsafe {
                        (
                            u64r(l.sym::<FnU64FromBuf>("ZSTD_findDecompressedSize")(p, t.len())),
                            u64r(l.sym::<FnU64FromBuf>("ZSTD_decompressBound")(p, t.len())),
                        )
                    }
                });
                diff_bytes(&format!("{label} :: +{extra} trailing decompress"), |l| {
                    decompress_simple(l, &t, plain.len().max(1))
                });
            }
            let trunc = &cat[..cat.len() - 1];
            diff(&format!("{label} :: truncated"), |l| {
                let p = trunc.as_ptr() as *const c_void;
                unsafe {
                    (
                        u64r(l.sym::<FnU64FromBuf>("ZSTD_findDecompressedSize")(p, trunc.len())),
                        u64r(l.sym::<FnU64FromBuf>("ZSTD_decompressBound")(p, trunc.len())),
                    )
                }
            });
        }
    }

    // 3 bytes total: below ZSTD_startingInputLength, so the whole loop is skipped.
    let three = vec![0x28u8, 0xB5, 0x2F];
    diff("multiframe 3 bytes", |l| {
        let p = three.as_ptr() as *const c_void;
        unsafe {
            (
                u64r(l.sym::<FnU64FromBuf>("ZSTD_findDecompressedSize")(p, 3)),
                u64r(l.sym::<FnU64FromBuf>("ZSTD_decompressBound")(p, 3)),
            )
        }
    });
}

// ===========================================================================
// 11. Empty input, and dst == NULL with dstCapacity == 0
// ===========================================================================

/// `srcSize == 0` is its own path on both sides:
/// `ZSTD_compressContinue_internal` returns right after the header
/// (`if (!srcSize) return fhSize`), `ZSTD_writeEpilogue` takes its
/// `stage == ZSTDcs_init` special case and calls
/// `ZSTD_writeFrameHeader(dst, cap, params, 0, 0)` with pledgedSrcSize **and**
/// dictID hardcoded to 0, then appends one empty `bt_raw` block and the optional
/// checksum.
///
/// `dst == NULL, dstCapacity == 0` is explicitly supported by
/// `ZSTD_decompressFrame` (`oend = dstCapacity != 0 ? ostart+dstCapacity :
/// ostart`) together with `ZSTD_copyRawBlock`'s `if (dst == NULL) { if (srcSize
/// == 0) return 0; RETURN_ERROR(dstBuffer_null); }` — so an empty frame decodes
/// to 0 and a non-empty one is a clean `dstBuffer_null`, never a crash. On the
/// compression side `ZSTD_writeFrameHeader` refuses any capacity below
/// `ZSTD_FRAMEHEADERSIZE_MAX` (18) before touching `dst`, so `NULL`/0 is safe
/// there too.
#[test]
fn empty_input_and_null_dst() {
    covers(&["CFG:80", "CFG:97"]);
    let empty: Vec<u8> = Vec::with_capacity(8);

    for &csf in &[0, 1] {
        for &ck in &[0, 1] {
            for &lvl in &[-5, 1, 3, 19, 22] {
                let label = format!("empty csf={csf} ck={ck} lvl={lvl}");
                let (_, r, b) = diff_bytes(&format!("{label} :: compress2"), |l| {
                    comp2_full(l, &empty, 64, &|l, c| {
                        vec![
                            set_param(l, c, ZSTD_c_compressionLevel, lvl),
                            set_param(l, c, ZSTD_c_contentSizeFlag, csf),
                            set_param(l, c, ZSTD_c_checksumFlag, ck),
                        ]
                    })
                });
                let cs = match r {
                    R::Ok(k) => k,
                    R::Err(a, s) => panic!("{label}: {a}:{s}"),
                };
                let frame = b.0[..cs].to_vec();
                diff(&format!("{label} :: probe"), |l| probe(l, &frame));
                for &cap in &[0usize, 1, 10] {
                    diff_bytes(&format!("{label} :: decompress cap={cap}"), |l| {
                        decompress_simple(l, &frame, cap)
                    });
                }
                // dst == NULL, dstCapacity == 0 on an empty frame -> 0.
                diff(&format!("{label} :: decompress NULL/0"), |l| {
                    let f = l.sym::<FnDecompress>("ZSTD_decompress");
                    res(l, unsafe {
                        f(
                            std::ptr::null_mut(),
                            0,
                            frame.as_ptr() as *const c_void,
                            frame.len(),
                        )
                    })
                });
            }
        }
    }

    // The same empty frame through every one-shot compressor.
    for &lvl in &[-131072, -5, 0, 1, 3, 19, 22] {
        diff_bytes(&format!("empty ZSTD_compress lvl={lvl}"), |l| {
            comp_full(l, &empty, lvl, 64)
        });
        diff_bytes(&format!("empty ZSTD_compressCCtx lvl={lvl}"), |l| {
            comp_cctx_full(l, &empty, lvl, 64)
        });
        diff_bytes(&format!("empty ZSTD_compress_usingDict lvl={lvl}"), |l| {
            let cctx = Ctx::cctx(l);
            let f = l.sym::<FnCompressUsingDict>("ZSTD_compress_usingDict");
            let mut dst = vec![0xCDu8; 64];
            let ret = unsafe {
                f(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    64,
                    empty.as_ptr() as *const c_void,
                    0,
                    std::ptr::null(),
                    0,
                    lvl,
                )
            };
            (res(l, ret), Blob(dst))
        });
    }

    // dst == NULL, dstCapacity == 0 on a NON-empty frame -> dstBuffer_null, and
    // on the compression side -> dstSize_tooSmall from ZSTD_writeFrameHeader.
    let src = corpus(Corpus::Text, 1000, 77);
    let frame = c_compress(&src, 3);
    diff("decompress NULL/0 on a non-empty frame", |l| {
        let f = l.sym::<FnDecompress>("ZSTD_decompress");
        res(l, unsafe {
            f(std::ptr::null_mut(), 0, frame.as_ptr() as *const c_void, frame.len())
        })
    });
    diff("compress into NULL/0", |l| {
        let f = l.sym::<FnCompress>("ZSTD_compress");
        res(l, unsafe {
            f(std::ptr::null_mut(), 0, src.as_ptr() as *const c_void, src.len(), 3)
        })
    });
    // A zero-length source with a NULL src pointer is what the caller of an
    // empty compress naturally passes; both must accept it.
    diff_bytes("compress NULL src, srcSize 0", |l| {
        let f = l.sym::<FnCompress>("ZSTD_compress");
        let mut dst = vec![0xCDu8; 64];
        let ret =
            unsafe { f(dst.as_mut_ptr() as *mut c_void, 64, std::ptr::null(), 0, 3) };
        (res(l, ret), Blob(dst))
    });
}

// ===========================================================================
// 12. ZSTD_isError / ZSTD_getErrorName / ZSTD_getErrorCode
// ===========================================================================

/// `ERR_isError` is the single comparison `code > ERROR(maxCode)` with
/// `maxCode == 120`, so `(size_t)-119`, `-120` and `-121` straddle the boundary,
/// and `ERR_getErrorName` is a switch over `ZSTD_ErrorCode` with an
/// "Unspecified error code" default. Every value from 0 to `(size_t)-130` that
/// the row names is compared, plus the real error returns produced by the
/// entry points in this file.
#[test]
fn is_error_and_error_name() {
    covers(&["CFG:3"]);
    let mut codes: Vec<SizeT> = vec![0, 1, 63, 100, 120, 121, 1000, SizeT::MAX / 2];
    for k in 1..=130usize {
        codes.push(0usize.wrapping_sub(k));
    }
    for &code in &codes {
        diff(&format!("ZSTD_isError({code:#x})"), |l| {
            (
                unsafe { l.sym::<FnIsError>("ZSTD_isError")(code) },
                err_name(l, code),
                err_code(l, code),
            )
        });
    }
    // Real error returns: a 0-capacity compress, a garbage decompress, and an
    // unknown parameter id.
    let src = corpus(Corpus::Text, 100, 5);
    diff("real error: ZSTD_compress cap=0", |l| {
        let f = l.sym::<FnCompress>("ZSTD_compress");
        let mut dst = [0u8; 1];
        let r = unsafe {
            f(dst.as_mut_ptr() as *mut c_void, 0, src.as_ptr() as *const c_void, src.len(), 3)
        };
        (res(l, r), err_name(l, r), err_code(l, r))
    });
    let junk = [0xAAu8; 4];
    diff("real error: ZSTD_decompress on 4x0xAA", |l| {
        let f = l.sym::<FnDecompress>("ZSTD_decompress");
        let mut dst = [0u8; 64];
        let r = unsafe {
            f(dst.as_mut_ptr() as *mut c_void, 64, junk.as_ptr() as *const c_void, 4)
        };
        (res(l, r), err_name(l, r), err_code(l, r))
    });
    diff("real error: ZSTD_CCtx_setParameter(99)", |l| {
        let cctx = Ctx::cctx(l);
        let f = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
        let r = unsafe { f(cctx.ptr, 99, 1) };
        (res(l, r), err_name(l, r), err_code(l, r))
    });
}
