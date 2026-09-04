//! Group 12 — legacy decoders v0.1 … v0.7  (`CONFIGS.md` rows 157-165).
//!
//! The legacy code paths are **decode only** — this library contains no legacy
//! encoder — so genuine v0.x frames cannot be produced here.  Instead every
//! legacy symbol is driven, in *both* shared libraries, with
//!
//!   * hundreds of pseudo-random buffers prefixed with each legacy magic
//!     number (exercises header parse + all the early error paths),
//!   * hand-built pseudo-frames assembled from the real v0.x on-disk layout
//!     (magic + frame-header byte(s) + 3-byte block headers) so that RAW / RLE
//!     / END blocks really are decoded, plus truncated and bit-flipped
//!     variants of those,
//!   * the buffered `ZBUFFv0x_*` streaming API over a grid of in/out chunk
//!     sizes,
//!   * the direct `nextSrcSizeToDecompress` / `decompressContinue` loop,
//!   * the low-level `FSEv0x_*` / `HUFv0x_*` entropy entry points,
//!   * and the public dispatch surface (`ZSTD_decompress`,
//!     `ZSTD_decompressStream`, `ZSTD_isFrame`, …) on all 7 magics — where
//!     v0.1 … v0.4 **must** be rejected because the build sets
//!     `ZSTD_LEGACY_SUPPORT=5`.
//!
//! Return values, destination buffers (including a generous slack region so
//! that any over-write is caught too) and every out-parameter are compared.

#![allow(non_snake_case)]
#![allow(dead_code)]

mod common;
use common::*;

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

// ===========================================================================
// symbol table
// ===========================================================================

/// Every legacy symbol exported by the reference C build.
const LEGACY_SYMBOLS: &[&str] = &[
    "FSEv05_buildDTable",
    "FSEv05_buildDTable_raw",
    "FSEv05_buildDTable_rle",
    "FSEv05_createDTable",
    "FSEv05_decompress",
    "FSEv05_decompress_usingDTable",
    "FSEv05_freeDTable",
    "FSEv05_getErrorName",
    "FSEv05_isError",
    "FSEv05_readNCount",
    "FSEv06_buildDTable",
    "FSEv06_buildDTable_raw",
    "FSEv06_buildDTable_rle",
    "FSEv06_createDTable",
    "FSEv06_decompress",
    "FSEv06_decompress_usingDTable",
    "FSEv06_freeDTable",
    "FSEv06_getErrorName",
    "FSEv06_isError",
    "FSEv06_readNCount",
    "FSEv07_buildDTable",
    "FSEv07_buildDTable_raw",
    "FSEv07_buildDTable_rle",
    "FSEv07_createDTable",
    "FSEv07_decompress",
    "FSEv07_decompress_usingDTable",
    "FSEv07_freeDTable",
    "FSEv07_getErrorName",
    "FSEv07_isError",
    "FSEv07_readNCount",
    "HUFv05_decompress",
    "HUFv05_decompress1X2",
    "HUFv05_decompress1X2_usingDTable",
    "HUFv05_decompress1X4",
    "HUFv05_decompress1X4_usingDTable",
    "HUFv05_decompress4X2",
    "HUFv05_decompress4X2_usingDTable",
    "HUFv05_decompress4X4",
    "HUFv05_decompress4X4_usingDTable",
    "HUFv05_getErrorName",
    "HUFv05_isError",
    "HUFv05_readDTableX2",
    "HUFv05_readDTableX4",
    "HUFv06_decompress",
    "HUFv06_decompress1X2",
    "HUFv06_decompress1X2_usingDTable",
    "HUFv06_decompress1X4",
    "HUFv06_decompress1X4_usingDTable",
    "HUFv06_decompress4X2",
    "HUFv06_decompress4X2_usingDTable",
    "HUFv06_decompress4X4",
    "HUFv06_decompress4X4_usingDTable",
    "HUFv06_readDTableX2",
    "HUFv06_readDTableX4",
    "HUFv07_decompress",
    "HUFv07_decompress1X2",
    "HUFv07_decompress1X2_DCtx",
    "HUFv07_decompress1X2_usingDTable",
    "HUFv07_decompress1X4",
    "HUFv07_decompress1X4_DCtx",
    "HUFv07_decompress1X4_usingDTable",
    "HUFv07_decompress1X_DCtx",
    "HUFv07_decompress1X_usingDTable",
    "HUFv07_decompress4X2",
    "HUFv07_decompress4X2_DCtx",
    "HUFv07_decompress4X2_usingDTable",
    "HUFv07_decompress4X4",
    "HUFv07_decompress4X4_DCtx",
    "HUFv07_decompress4X4_usingDTable",
    "HUFv07_decompress4X_DCtx",
    "HUFv07_decompress4X_hufOnly",
    "HUFv07_decompress4X_usingDTable",
    "HUFv07_getErrorName",
    "HUFv07_isError",
    "HUFv07_readDTableX2",
    "HUFv07_readDTableX4",
    "HUFv07_readStats",
    "HUFv07_selectDecoder",
    "ZBUFFv04_createDCtx",
    "ZBUFFv04_decompressContinue",
    "ZBUFFv04_decompressInit",
    "ZBUFFv04_decompressWithDictionary",
    "ZBUFFv04_freeDCtx",
    "ZBUFFv04_getErrorName",
    "ZBUFFv04_isError",
    "ZBUFFv04_recommendedDInSize",
    "ZBUFFv04_recommendedDOutSize",
    "ZBUFFv05_createDCtx",
    "ZBUFFv05_decompressContinue",
    "ZBUFFv05_decompressInit",
    "ZBUFFv05_decompressInitDictionary",
    "ZBUFFv05_freeDCtx",
    "ZBUFFv05_getErrorName",
    "ZBUFFv05_isError",
    "ZBUFFv05_recommendedDInSize",
    "ZBUFFv05_recommendedDOutSize",
    "ZBUFFv06_createDCtx",
    "ZBUFFv06_decompressContinue",
    "ZBUFFv06_decompressInit",
    "ZBUFFv06_decompressInitDictionary",
    "ZBUFFv06_freeDCtx",
    "ZBUFFv06_getErrorName",
    "ZBUFFv06_isError",
    "ZBUFFv06_recommendedDInSize",
    "ZBUFFv06_recommendedDOutSize",
    "ZBUFFv07_createDCtx",
    "ZBUFFv07_createDCtx_advanced",
    "ZBUFFv07_decompressContinue",
    "ZBUFFv07_decompressInit",
    "ZBUFFv07_decompressInitDictionary",
    "ZBUFFv07_freeDCtx",
    "ZBUFFv07_getErrorName",
    "ZBUFFv07_isError",
    "ZBUFFv07_recommendedDInSize",
    "ZBUFFv07_recommendedDOutSize",
    "ZSTDv01_createDCtx",
    "ZSTDv01_decompress",
    "ZSTDv01_decompressContinue",
    "ZSTDv01_decompressDCtx",
    "ZSTDv01_findFrameSizeInfoLegacy",
    "ZSTDv01_freeDCtx",
    "ZSTDv01_isError",
    "ZSTDv01_nextSrcSizeToDecompress",
    "ZSTDv01_resetDCtx",
    "ZSTDv02_createDCtx",
    "ZSTDv02_decompress",
    "ZSTDv02_decompressContinue",
    "ZSTDv02_findFrameSizeInfoLegacy",
    "ZSTDv02_freeDCtx",
    "ZSTDv02_isError",
    "ZSTDv02_nextSrcSizeToDecompress",
    "ZSTDv02_resetDCtx",
    "ZSTDv03_createDCtx",
    "ZSTDv03_decompress",
    "ZSTDv03_decompressContinue",
    "ZSTDv03_findFrameSizeInfoLegacy",
    "ZSTDv03_freeDCtx",
    "ZSTDv03_isError",
    "ZSTDv03_nextSrcSizeToDecompress",
    "ZSTDv03_resetDCtx",
    "ZSTDv04_createDCtx",
    "ZSTDv04_decompress",
    "ZSTDv04_decompressContinue",
    "ZSTDv04_decompressDCtx",
    "ZSTDv04_findFrameSizeInfoLegacy",
    "ZSTDv04_freeDCtx",
    "ZSTDv04_nextSrcSizeToDecompress",
    "ZSTDv04_resetDCtx",
    "ZSTDv05_copyDCtx",
    "ZSTDv05_createDCtx",
    "ZSTDv05_decompress",
    "ZSTDv05_decompressBegin",
    "ZSTDv05_decompressBegin_usingDict",
    "ZSTDv05_decompressBlock",
    "ZSTDv05_decompressContinue",
    "ZSTDv05_decompressDCtx",
    "ZSTDv05_decompress_usingDict",
    "ZSTDv05_decompress_usingPreparedDCtx",
    "ZSTDv05_findFrameSizeInfoLegacy",
    "ZSTDv05_freeDCtx",
    "ZSTDv05_getErrorName",
    "ZSTDv05_getFrameParams",
    "ZSTDv05_isError",
    "ZSTDv05_nextSrcSizeToDecompress",
    "ZSTDv05_sizeofDCtx",
    "ZSTDv06_copyDCtx",
    "ZSTDv06_createDCtx",
    "ZSTDv06_decompress",
    "ZSTDv06_decompressBegin",
    "ZSTDv06_decompressBegin_usingDict",
    "ZSTDv06_decompressBlock",
    "ZSTDv06_decompressContinue",
    "ZSTDv06_decompressDCtx",
    "ZSTDv06_decompress_usingDict",
    "ZSTDv06_decompress_usingPreparedDCtx",
    "ZSTDv06_findFrameSizeInfoLegacy",
    "ZSTDv06_freeDCtx",
    "ZSTDv06_getErrorName",
    "ZSTDv06_getFrameParams",
    "ZSTDv06_isError",
    "ZSTDv06_nextSrcSizeToDecompress",
    "ZSTDv06_sizeofDCtx",
    "ZSTDv07_copyDCtx",
    "ZSTDv07_createDCtx",
    "ZSTDv07_createDCtx_advanced",
    "ZSTDv07_createDDict",
    "ZSTDv07_decompress",
    "ZSTDv07_decompressBegin",
    "ZSTDv07_decompressBegin_usingDict",
    "ZSTDv07_decompressBlock",
    "ZSTDv07_decompressContinue",
    "ZSTDv07_decompressDCtx",
    "ZSTDv07_decompress_usingDDict",
    "ZSTDv07_decompress_usingDict",
    "ZSTDv07_estimateDCtxSize",
    "ZSTDv07_findFrameSizeInfoLegacy",
    "ZSTDv07_freeDCtx",
    "ZSTDv07_freeDDict",
    "ZSTDv07_getDecompressedSize",
    "ZSTDv07_getErrorName",
    "ZSTDv07_getFrameParams",
    "ZSTDv07_insertBlock",
    "ZSTDv07_isError",
    "ZSTDv07_isSkipFrame",
    "ZSTDv07_nextSrcSizeToDecompress",
    "ZSTDv07_sizeofDCtx",
];

// ===========================================================================
// fn types
// ===========================================================================

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnVoidPtr1 = unsafe extern "C" fn(*mut c_void);
type FnSz0 = unsafe extern "C" fn() -> usize;
type FnSz1 = unsafe extern "C" fn(*mut c_void) -> usize;
type FnU32Sz = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

/// `(dst, dstCapacity, src, srcSize) -> size_t`
type FnDec = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
/// `(ctx, dst, dstCapacity, src, srcSize) -> size_t`
type FnDecCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
/// `(ctx, dst, cap, src, size, dict, dictSize) -> size_t`
type FnDecDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    usize,
) -> usize;
/// `(dctx, refDCtx, dst, cap, src, size) -> size_t`
type FnDecPrepared =
    unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void, usize, *const c_void, usize) -> usize;
/// `(dctx, dst, cap, src, size, ddict) -> size_t`
type FnDecDDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, *const c_void) -> usize;
/// `(src, srcSize, *cSize, *dBound)`
type FnFsil = unsafe extern "C" fn(*const c_void, usize, *mut usize, *mut c_ulonglong);
/// `(paramsPtr, src, srcSize) -> size_t`
type FnGetFP = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
/// `(dst, src)`
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
/// `(ctx, dict, dictSize) -> size_t`
type FnInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
/// `(ctx, dst, *dstCapacity, src, *srcSize) -> size_t`
type FnZbCont =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize, *const c_void, *mut usize) -> usize;
type FnU64Src = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
type FnIsSkip = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnSizeofPtr = unsafe extern "C" fn(*const c_void) -> usize;
type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;

// FSE
type FnFseCreateDT = unsafe extern "C" fn(c_uint) -> *mut c_uint;
type FnFseFreeDT = unsafe extern "C" fn(*mut c_uint);
type FnFseBuildDT = unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint) -> usize;
type FnFseBuildRaw = unsafe extern "C" fn(*mut c_uint, c_uint) -> usize;
type FnFseBuildRle = unsafe extern "C" fn(*mut c_uint, u8) -> usize;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
type FnFseDecDT =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const c_uint) -> usize;

// HUF
type FnHufReadU16 = unsafe extern "C" fn(*mut u16, *const c_void, usize) -> usize;
type FnHufReadU32 = unsafe extern "C" fn(*mut c_uint, *const c_void, usize) -> usize;
type FnHufDecU16 =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const u16) -> usize;
type FnHufDecU32 =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const c_uint) -> usize;
type FnHufDCtx = unsafe extern "C" fn(*mut c_uint, *mut c_void, usize, *const c_void, usize) -> usize;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut c_uint,
    *mut c_uint,
    *mut c_uint,
    *const c_void,
    usize,
) -> usize;
type FnHufSelect = unsafe extern "C" fn(usize, usize) -> c_uint;

// dispatch surface
type FnIsFrame = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnFindCSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnDictID = unsafe extern "C" fn(*const c_void, usize) -> c_uint;

// ===========================================================================
// helpers
// ===========================================================================

/// Slack appended to every source buffer (so that an out-of-bounds *read* by
/// the legacy C decoder stays inside a mapped allocation) and to every
/// destination buffer (so that an out-of-bounds *write* is still compared).
const SLACK: usize = 1 << 16;

/// Source buffer with trailing slack; `len` is the logical size handed to the
/// library.
struct Src {
    v: Vec<u8>,
    len: usize,
}

impl Src {
    fn new(b: &[u8]) -> Src {
        let mut v = Vec::with_capacity(b.len() + SLACK);
        v.extend_from_slice(b);
        v.resize(b.len() + SLACK, 0);
        Src { v, len: b.len() }
    }
    fn ptr(&self) -> *const c_void {
        self.v.as_ptr() as *const c_void
    }
    fn at(&self, off: usize) -> *const c_void {
        unsafe { self.v.as_ptr().add(off) as *const c_void }
    }
}

/// Destination buffer pair (C / Rust) with slack, compared in full.
struct Dst {
    c: Vec<u8>,
    r: Vec<u8>,
    cap: usize,
}

impl Dst {
    fn new(cap: usize) -> Dst {
        Dst { c: vec![0xA5u8; cap + SLACK], r: vec![0xA5u8; cap + SLACK], cap }
    }
    fn cp(&mut self) -> *mut c_void {
        self.c.as_mut_ptr() as *mut c_void
    }
    fn rp(&mut self) -> *mut c_void {
        self.r.as_mut_ptr() as *mut c_void
    }
    #[track_caller]
    fn check(&self, what: &str) {
        eqbuf(what, &self.c, &self.r);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ver {
    V01,
    V02,
    V03,
    V04,
    V05,
    V06,
    V07,
}

const ALL_VERS: [Ver; 7] = [Ver::V01, Ver::V02, Ver::V03, Ver::V04, Ver::V05, Ver::V06, Ver::V07];

impl Ver {
    fn n(self) -> u32 {
        match self {
            Ver::V01 => 1,
            Ver::V02 => 2,
            Ver::V03 => 3,
            Ver::V04 => 4,
            Ver::V05 => 5,
            Ver::V06 => 6,
            Ver::V07 => 7,
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Ver::V01 => "v01",
            Ver::V02 => "v02",
            Ver::V03 => "v03",
            Ver::V04 => "v04",
            Ver::V05 => "v05",
            Ver::V06 => "v06",
            Ver::V07 => "v07",
        }
    }
    /// The 4 magic bytes as they appear on disk.
    fn magic_bytes(self) -> [u8; 4] {
        match self {
            // v0.1 reads its magic big-endian: 0xFD2FB51E
            Ver::V01 => [0xFD, 0x2F, 0xB5, 0x1E],
            Ver::V02 => 0xFD2FB522u32.to_le_bytes(),
            Ver::V03 => 0xFD2FB523u32.to_le_bytes(),
            Ver::V04 => 0xFD2FB524u32.to_le_bytes(),
            Ver::V05 => 0xFD2FB525u32.to_le_bytes(),
            Ver::V06 => 0xFD2FB526u32.to_le_bytes(),
            Ver::V07 => 0xFD2FB527u32.to_le_bytes(),
        }
    }
}

/// 3-byte legacy block header (identical layout in v0.1 … v0.7).
/// `btype`: 0 = compressed, 1 = raw, 2 = rle, 3 = end.
fn push_block_header(v: &mut Vec<u8>, btype: u8, size: u32) {
    v.push((btype << 6) | ((size >> 16) & 7) as u8);
    v.push(((size >> 8) & 0xFF) as u8);
    v.push((size & 0xFF) as u8);
}

/// Build a syntactically well-formed frame header for `v`.
fn frame_header(v: Ver, rng: &mut Rng) -> Vec<u8> {
    let mut h = v.magic_bytes().to_vec();
    match v {
        // v0.1 / v0.2 / v0.3: frame header is the 4 magic bytes only.
        Ver::V01 | Ver::V02 | Ver::V03 => {}
        // v0.4 / v0.5: one descriptor byte, low nibble = windowLog - 11,
        // high nibble reserved (must be 0).
        Ver::V04 | Ver::V05 => {
            let wl = rng.below(15) as u8; // windowLog 11..25
            h.push(wl);
        }
        // v0.6: descriptor byte = fcsId<<6 | (windowLog-12); bit5 reserved.
        Ver::V06 => {
            let fcs = rng.below(4) as u8;
            let wl = rng.below(13) as u8; // windowLog 12..24
            h.push((fcs << 6) | wl);
            let n = [0usize, 1, 2, 8][fcs as usize];
            for _ in 0..n {
                h.push(rng.byte());
            }
        }
        // v0.7: full frame-header descriptor.
        Ver::V07 => {
            let mut fhd = rng.byte();
            fhd &= !0x08; // reserved bit 3 must be zero
            h.push(fhd);
            let did_code = (fhd & 3) as usize;
            let direct = ((fhd >> 5) & 1) != 0;
            let fcs_id = (fhd >> 6) as usize;
            if !direct {
                let wl = (rng.below(18) as u8) << 3;
                h.push(wl | (rng.byte() & 7));
            }
            for _ in 0..[0usize, 1, 2, 4][did_code] {
                h.push(rng.byte());
            }
            let mut n = [0usize, 2, 4, 8][fcs_id];
            if direct && n == 0 {
                n = 1;
            }
            for _ in 0..n {
                h.push(rng.byte());
            }
        }
    }
    h
}

/// A frame header that every version is guaranteed to *accept*: no dictID, no
/// checksum, no single-segment flag, no frame-content-size field.
fn frame_header_canonical(v: Ver, rng: &mut Rng) -> Vec<u8> {
    let mut h = v.magic_bytes().to_vec();
    match v {
        Ver::V01 | Ver::V02 | Ver::V03 => {}
        // low nibble = windowLog - 11 (v0.4/v0.5) / - 12 (v0.6); high bits reserved 0
        Ver::V04 | Ver::V05 => h.push(rng.below(15) as u8),
        Ver::V06 => h.push(rng.below(13) as u8),
        Ver::V07 => {
            h.push(0x00);
            let wl = (rng.below(18) as u8) << 3;
            h.push(wl);
        }
    }
    h
}

/// A frame made only of RAW and RLE blocks — decodable by v0.4 … v0.7 and
/// (for RAW blocks) by v0.1 … v0.3.
fn craft_raw_rle_frame(v: Ver, rng: &mut Rng, nblocks: usize, raw_only: bool) -> Vec<u8> {
    let mut b = frame_header(v, rng);
    for _ in 0..nblocks {
        let rle = !raw_only && rng.below(2) == 0;
        if rle {
            let n = rng.below(4000) as u32;
            push_block_header(&mut b, 2, n);
            b.push(rng.byte());
        } else {
            let n = rng.below(400);
            let d = rng.bytes(n);
            push_block_header(&mut b, 1, n as u32);
            b.extend_from_slice(&d);
        }
    }
    push_block_header(&mut b, 3, 0);
    b
}

/// A *valid* v0.5 / v0.6 / v0.7 compressed-block payload: a RAW or RLE
/// literals section followed by a `nbSeq == 0` sequences section.  The three
/// versions share this layout exactly (`IS_RAW == 2`, `IS_RLE == 3`,
/// `MIN_CBLOCK_SIZE == 3`, `MIN_SEQUENCES_SIZE == 1`).
fn v567_compressed_block(rng: &mut Rng, rle_lits: bool) -> Vec<u8> {
    let mut p = Vec::new();
    if rle_lits {
        let lit = rng.below(32);
        p.push(0xC0 | lit as u8); // IS_RLE << 6 | lhSize-code 0/1 | litSize
        p.push(rng.byte());
    } else {
        let lit = 1 + rng.below(31);
        p.push(0x80 | lit as u8); // IS_RAW << 6 | lhSize-code 0/1 | litSize
        let d = rng.bytes(lit);
        p.extend_from_slice(&d);
    }
    p.push(0x00); // nbSeq == 0
    p
}

/// The per-version corpus: random-after-magic, crafted, truncated, mutated.
fn corpus(v: Ver, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed ^ (v.n() as u64) << 32);
    let mut out: Vec<Vec<u8>> = Vec::new();

    // ---- garbage of many lengths behind a valid magic ---------------------
    const NS: [usize; 22] =
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 16, 23, 32, 64, 127, 128, 255, 256, 1000, 4096];
    for &n in NS.iter() {
        let magic = v.magic_bytes();
        let mut b = magic.to_vec();
        let extra = rng.bytes(n);
        b.extend_from_slice(&extra);
        out.push(b);
        // magic truncated to n bytes
        if n <= 4 {
            out.push(magic[..n].to_vec());
        }
        // a full syntactic header followed by garbage
        let h = frame_header(v, &mut rng);
        let mut b2 = h.clone();
        let extra2 = rng.bytes(n);
        b2.extend_from_slice(&extra2);
        out.push(b2);
        // header truncated
        if n < h.len() {
            out.push(h[..n].to_vec());
        }
    }

    // ---- clean RAW / RLE frames -------------------------------------------
    for i in 0..120 {
        let nb = 1 + (i % 6);
        out.push(craft_raw_rle_frame(v, &mut rng, nb, false));
        out.push(craft_raw_rle_frame(v, &mut rng, nb, true));
    }
    // block sizes straddling the 128 KB block limit
    for &n in &[
        128usize * 1024 - 1,
        128 * 1024,
        128 * 1024 + 1,
        (1 << 21) - 1, // > 3-byte block-size field can express (21 bits)
    ] {
        let mut b = frame_header(v, &mut rng);
        let d = gen_class(n % N_CLASSES, n.min(1 << 18), 0x51_2E00 + n as u64);
        push_block_header(&mut b, 1, n as u32);
        b.extend_from_slice(&d);
        push_block_header(&mut b, 3, 0);
        out.push(b);
        let mut b2 = frame_header(v, &mut rng);
        push_block_header(&mut b2, 2, n as u32);
        b2.push(rng.byte());
        push_block_header(&mut b2, 3, 0);
        out.push(b2);
    }
    // empty frame: header + end block
    for _ in 0..3 {
        let mut b = frame_header(v, &mut rng);
        push_block_header(&mut b, 3, 0);
        out.push(b);
    }

    // ---- frames including compressed blocks, plus mutations --------------
    for i in 0..120 {
        let mut b = frame_header(v, &mut rng);
        let nb = 1 + (i % 5);
        for _ in 0..nb {
            match rng.below(3) {
                0 => {
                    let n = 1 + rng.below(160);
                    let d = rng.bytes(n);
                    push_block_header(&mut b, 0, n as u32);
                    b.extend_from_slice(&d);
                }
                1 => {
                    let n = rng.below(200);
                    let d = rng.bytes(n);
                    push_block_header(&mut b, 1, n as u32);
                    b.extend_from_slice(&d);
                }
                _ => {
                    let n = rng.below(2000) as u32;
                    push_block_header(&mut b, 2, n);
                    b.push(rng.byte());
                }
            }
        }
        push_block_header(&mut b, 3, 0);
        out.push(b.clone());

        // truncation
        let tl = rng.below(b.len() + 1);
        out.push(b[..tl].to_vec());

        // single-bit mutation
        let mut m = b.clone();
        if !m.is_empty() {
            let idx = rng.below(m.len());
            let bit = rng.below(8);
            m[idx] ^= 1u8 << bit;
            out.push(m);
        }
    }

    // ---- genuinely valid *compressed* blocks (v0.5 / v0.6 / v0.7) --------
    if matches!(v, Ver::V05 | Ver::V06 | Ver::V07) {
        for k in 0..100 {
            let mut b = frame_header(v, &mut rng);
            for j in 0..(1 + k % 3) {
                let payload = v567_compressed_block(&mut rng, (k + j) % 2 == 0);
                push_block_header(&mut b, 0, payload.len() as u32);
                b.extend_from_slice(&payload);
            }
            push_block_header(&mut b, 3, 0);
            out.push(b.clone());
            let tl = rng.below(b.len() + 1);
            out.push(b[..tl].to_vec());
        }
        // mixed frames: raw + rle + compressed blocks together
        for k in 0..50 {
            let mut b = frame_header(v, &mut rng);
            let n = rng.below(300);
            let d = rng.bytes(n);
            push_block_header(&mut b, 1, n as u32);
            b.extend_from_slice(&d);
            let rl = rng.below(1000) as u32;
            push_block_header(&mut b, 2, rl);
            b.push(rng.byte());
            let payload = v567_compressed_block(&mut rng, k % 2 == 0);
            push_block_header(&mut b, 0, payload.len() as u32);
            b.extend_from_slice(&payload);
            push_block_header(&mut b, 3, 0);
            out.push(b);
        }
    }

    // ---- structured content behind the magic (from gen_class) ------------
    for class in 0..N_CLASSES {
        for &sz in &[16usize, 200, 3000, 70_000] {
            let mut b = v.magic_bytes().to_vec();
            b.extend_from_slice(&gen_class(class, sz, seed ^ sz as u64));
            out.push(b);
        }
    }

    // ---- two frames back to back (multi-frame decoding) ------------------
    for k in 0..24 {
        let a = craft_raw_rle_frame(v, &mut rng, 1 + k % 3, true);
        let b = craft_raw_rle_frame(v, &mut rng, 1 + (k + 1) % 3, true);
        let mut both = a.clone();
        both.extend_from_slice(&b);
        out.push(both);
    }

    out
}

const CAPS: [usize; 4] = [0, 1, 300, 1 << 18];

// ---------------------------------------------------------------------------
// custom allocator shared by both libraries (for the `_advanced` ctors)
// ---------------------------------------------------------------------------

unsafe extern "C" fn test_alloc(_opaque: *mut c_void, size: usize) -> *mut c_void {
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    (p as *mut usize).write(size);
    p.add(16) as *mut c_void
}

unsafe extern "C" fn test_free(_opaque: *mut c_void, addr: *mut c_void) {
    if addr.is_null() {
        return;
    }
    let base = (addr as *mut u8).sub(16);
    let size = (base as *mut usize).read();
    let layout = std::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    std::alloc::dealloc(base, layout);
}

fn custom_mem() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(test_alloc),
        customFree: Some(test_free),
        opaque: std::ptr::null_mut(),
    }
}

// ===========================================================================
// row 158-164 : the per-version one-shot decompressors
// ===========================================================================

/// `ZSTDv0x_decompress` / `ZSTDv0x_decompressDCtx` over the whole corpus.
unsafe fn drive_oneshot(v: Ver, one: &str, dctx_variant: Option<(&str, &str, &str)>) {
    let (dc, dr) = duo::<FnDec>(one);
    let bufs = corpus(v, 0xB1_0000 + v.n() as u64);
    // reusable dctx pair for the *_decompressDCtx variant
    let ctxpair = dctx_variant.map(|(create, free, dcname)| {
        let (cc, cr) = duo::<FnCreate>(create);
        let (fc, fr) = duo::<FnFree>(free);
        let (xc, xr) = duo::<FnDecCtx>(dcname);
        (cc(), cr(), fc, fr, xc, xr)
    });

    let mut ok_nonempty = 0usize;
    for (i, b) in bufs.iter().enumerate() {
        let s = Src::new(b);
        for (ci, &cap) in CAPS.iter().enumerate() {
            let mut d = Dst::new(cap);
            let rc = dc(d.cp(), cap, s.ptr(), s.len);
            let rr = dr(d.rp(), cap, s.ptr(), s.len);
            eqv(&format!("{one}[buf{i} cap{ci}]"), rc, rr);
            d.check(&format!("{one}[buf{i} cap{ci}] dst"));
            if !is_err(rc) && rc > 0 {
                ok_nonempty += 1;
            }

            if let Some((cc, cr, _, _, xc, xr)) = ctxpair {
                let name = dctx_variant.unwrap().2;
                let mut d2 = Dst::new(cap);
                let rc2 = xc(cc, d2.cp(), cap, s.ptr(), s.len);
                let rr2 = xr(cr, d2.rp(), cap, s.ptr(), s.len);
                eqv(&format!("{name}[buf{i} cap{ci}]"), rc2, rr2);
                d2.check(&format!("{name}[buf{i} cap{ci}] dst"));
            }
        }
    }

    if let Some((cc, cr, fc, fr, _, _)) = ctxpair {
        eqv("freeDCtx", fc(cc), fr(cr));
    }
    // the corpus must actually reach block decoding, not just the error paths
    assert!(
        ok_nonempty > 0,
        "{}: no crafted {} frame decoded to a non-empty output — corpus is not \
         exercising the block decoder",
        one,
        v.tag()
    );
}

/// `ZSTDv0x_findFrameSizeInfoLegacy` over the whole corpus.
unsafe fn drive_fsil(v: Ver, name: &str) {
    let (fc, fr) = duo::<FnFsil>(name);
    for (i, b) in corpus(v, 0xF5_0000 + v.n() as u64).iter().enumerate() {
        let s = Src::new(b);
        let mut cs_c: usize = 0x5A5A;
        let mut cs_r: usize = 0x5A5A;
        let mut db_c: c_ulonglong = 0x1234;
        let mut db_r: c_ulonglong = 0x1234;
        fc(s.ptr(), s.len, &mut cs_c, &mut db_c);
        fr(s.ptr(), s.len, &mut cs_r, &mut db_r);
        eqv(&format!("{name}[buf{i}] cSize"), cs_c, cs_r);
        eqv(&format!("{name}[buf{i}] dBound"), db_c, db_r);
    }
}

/// direct streaming: `nextSrcSizeToDecompress` + `decompressContinue`
unsafe fn drive_direct_stream(
    v: Ver,
    create: &str,
    free: &str,
    next: &str,
    cont: &str,
    begin: Option<&str>,
) {
    let (cc, cr) = duo::<FnCreate>(create);
    let (fc, fr) = duo::<FnFree>(free);
    let (nc, nr) = duo::<FnSz1>(next);
    let (dc, dr) = duo::<FnDecCtx>(cont);
    let beg = begin.map(|b| duo::<FnSz1>(b));

    let bufs = corpus(v, 0x57_0000 + v.n() as u64);
    let mut productive = 0usize;
    for (i, b) in bufs.iter().enumerate() {
        let s = Src::new(b);
        for &cap in &[0usize, 300, 1 << 18] {
            let mut trace_c: Vec<(usize, usize)> = Vec::new();
            let mut trace_r: Vec<(usize, usize)> = Vec::new();
            let mut beg_c = 0usize;
            let mut beg_r = 0usize;
            let mut d = Dst::new(cap);

            for lib in 0..2 {
                let ctx = if lib == 0 { cc() } else { cr() };
                assert!(!ctx.is_null());
                if let Some((bc, br)) = beg {
                    // resetDCtx / decompressBegin
                    let rb = if lib == 0 { bc(ctx) } else { br(ctx) };
                    if lib == 0 {
                        beg_c = rb;
                    } else {
                        beg_r = rb;
                    }
                }
                let base = if lib == 0 { d.cp() } else { d.rp() };
                let mut op = 0usize;
                let mut ip = 0usize;
                let mut steps = 0;
                loop {
                    steps += 1;
                    if steps > 300 {
                        break;
                    }
                    let exp = if lib == 0 { nc(ctx) } else { nr(ctx) };
                    if exp == 0 || is_err(exp) {
                        if lib == 0 {
                            trace_c.push((exp, 0));
                        } else {
                            trace_r.push((exp, 0));
                        }
                        break;
                    }
                    if ip + exp > s.len {
                        if lib == 0 {
                            trace_c.push((exp, usize::MAX));
                        } else {
                            trace_r.push((exp, usize::MAX));
                        }
                        break;
                    }
                    let ret = if lib == 0 {
                        dc((ctx) as *mut c_void, (base as *mut u8).add(op) as *mut c_void, cap - op, s.at(ip), exp)
                    } else {
                        dr((ctx) as *mut c_void, (base as *mut u8).add(op) as *mut c_void, cap - op, s.at(ip), exp)
                    };
                    if lib == 0 {
                        trace_c.push((exp, ret));
                    } else {
                        trace_r.push((exp, ret));
                    }
                    if is_err(ret) {
                        break;
                    }
                    assert!(ret <= cap - op, "decompressContinue overflowed dst");
                    op += ret;
                    ip += exp;
                }
                if lib == 0 {
                    fc(ctx);
                } else {
                    fr(ctx);
                }
            }
            if trace_c.iter().any(|&(_, r)| r != usize::MAX && !is_err(r) && r > 0) {
                productive += 1;
            }
            eqv(&format!("{cont}[buf{i} cap{cap}] begin"), beg_c, beg_r);
            eqv(&format!("{cont}[buf{i} cap{cap}] trace"), trace_c, trace_r);
            d.check(&format!("{cont}[buf{i} cap{cap}] dst"));
        }
    }
    assert!(
        productive > 0,
        "{cont}: the streaming loop never regenerated any data — corpus too weak"
    );
}

// ---------------------------------------------------------------------------
// buffered (ZBUFF) streaming
// ---------------------------------------------------------------------------

struct ZbApi {
    create: FnCreate,
    free: FnFree,
    init: FnSz1,
    init_dict: FnInitDict,
    cont: FnZbCont,
    /// v0.4 needs `decompressInit` followed by `decompressWithDictionary`.
    v04: bool,
}

unsafe fn zb_run(
    api: &ZbApi,
    dict: &Src,
    src: &Src,
    in_chunk: usize,
    out_chunk: usize,
) -> (Vec<(usize, usize, usize)>, Vec<u8>) {
    let ctx = (api.create)();
    assert!(!ctx.is_null(), "ZBUFF createDCtx returned NULL");
    let init_ret = if api.v04 {
        let a = (api.init)(ctx);
        let b = (api.init_dict)(ctx, dict.ptr(), dict.len);
        a ^ b
    } else if dict.len == 0 {
        (api.init)(ctx)
    } else {
        (api.init_dict)(ctx, dict.ptr(), dict.len)
    };

    // the init result is compared as the first pseudo-step of the trace
    let mut trace = vec![(init_ret, usize::MAX, usize::MAX)];
    let mut outall = Vec::new();
    let mut obuf = vec![0xC3u8; out_chunk + SLACK];
    let mut ip = 0usize;
    let mut steps = 0;
    loop {
        steps += 1;
        if steps > 400 {
            break;
        }
        let avail = (src.len - ip).min(in_chunk);
        let mut ssz = avail;
        let mut dsz = out_chunk;
        let r = (api.cont)(
            ctx,
            obuf.as_mut_ptr() as *mut c_void,
            &mut dsz,
            src.at(ip),
            &mut ssz,
        );
        trace.push((r, ssz, dsz));
        if is_err(r) {
            break;
        }
        assert!(dsz <= out_chunk, "ZBUFF wrote past dst");
        assert!(ssz <= avail, "ZBUFF consumed past src");
        outall.extend_from_slice(&obuf[..dsz]);
        ip += ssz;
        if r == 0 {
            break; // frame complete
        }
        if ssz == 0 && dsz == 0 {
            break; // no progress
        }
    }
    (api.free)(ctx);
    (trace, outall)
}

unsafe fn zb_api(prefix: &str, v04: bool) -> (ZbApi, ZbApi) {
    let (cc, cr) = duo::<FnCreate>(&format!("{prefix}_createDCtx"));
    let (fc, fr) = duo::<FnFree>(&format!("{prefix}_freeDCtx"));
    let (ic, ir) = duo::<FnSz1>(&format!("{prefix}_decompressInit"));
    let dictname = if v04 {
        format!("{prefix}_decompressWithDictionary")
    } else {
        format!("{prefix}_decompressInitDictionary")
    };
    let (kc, kr) = duo::<FnInitDict>(&dictname);
    let (tc, tr) = duo::<FnZbCont>(&format!("{prefix}_decompressContinue"));
    (
        ZbApi { create: cc, free: fc, init: ic, init_dict: kc, cont: tc, v04 },
        ZbApi { create: cr, free: fr, init: ir, init_dict: kr, cont: tr, v04 },
    )
}

unsafe fn drive_zbuff(v: Ver, prefix: &str, v04: bool) {
    let (ac, ar) = zb_api(prefix, v04);
    let bufs = corpus(v, 0x2B_0000 + v.n() as u64);
    let mut rng = Rng::new(0x2B_1000 + v.n() as u64);
    let chunks: [(usize, usize); 10] = [
        (1, 1),
        (1, 1 << 17),
        (3, 7),
        (17, 129),
        (1 << 17, 1 << 17),
        (5, 1 << 17),
        (1 << 17, 3),
        (1024, 1024),
        (131_075, 131_072),
        (1 << 20, 1 << 20),
    ];

    let nodict = Src::new(&[]);
    let mut produced = 0usize;
    let mut completed = 0usize;
    for (i, b) in bufs.iter().enumerate().filter(|(i, _)| i % 2 == 0) {
        let s = Src::new(b);
        for (ci, &(ic, oc)) in chunks.iter().enumerate() {
            let (t1, o1) = zb_run(&ac, &nodict, &s, ic, oc);
            let (t2, o2) = zb_run(&ar, &nodict, &s, ic, oc);
            if !o1.is_empty() {
                produced += 1;
            }
            if t1.last().map(|&(r, _, _)| r == 0).unwrap_or(false) {
                completed += 1;
            }
            eqv(&format!("{prefix}[buf{i} chunk{ci}] trace"), t1, t2);
            eqbuf(&format!("{prefix}[buf{i} chunk{ci}] out"), &o1, &o2);
        }
    }
    assert!(produced > 0, "{prefix}: streaming never produced output");
    assert!(completed > 0, "{prefix}: streaming never completed a frame");

    // with dictionaries
    for di in 0..6 {
        let dn = [0usize, 1, 8, 64, 4096, 200_000][di];
        let dict = Src::new(&gen_class(di, dn, 0x5EED_0000 + di as u64));
        for (i, b) in bufs.iter().enumerate().filter(|(i, _)| i % 7 == 0) {
            let s = Src::new(b);
            let ic = 1 + rng.below(1 << 17);
            let oc = 1 + rng.below(1 << 17);
            let (t1, o1) = zb_run(&ac, &dict, &s, ic, oc);
            let (t2, o2) = zb_run(&ar, &dict, &s, ic, oc);
            eqv(&format!("{prefix}[dict{di} buf{i}] trace"), t1, t2);
            eqbuf(&format!("{prefix}[dict{di} buf{i}] out"), &o1, &o2);
        }
    }
}

// ===========================================================================
// tests — inventory & scalar helpers
// ===========================================================================

#[test]
fn legacy_all_symbols_resolve_in_both_libraries() {
    unsafe {
        for name in LEGACY_SYMBOLS {
            let (c, r) = duo::<FnSz0>(name);
            assert!(!(c as usize == 0), "{name} resolved to NULL in C");
            assert!(!(r as usize == 0), "{name} resolved to NULL in Rust");
        }
    }
    assert_eq!(LEGACY_SYMBOLS.len(), 206);
}

/// Guards the size of the generated corpora (so a future edit cannot silently
/// shrink the differential coverage).
#[test]
fn legacy_corpus_is_large() {
    let mut total = 0usize;
    for v in ALL_VERS {
        let n = corpus(v, 1).len();
        eprintln!("corpus {} -> {n} buffers", v.tag());
        assert!(n > 400, "{} corpus shrank to {n} buffers", v.tag());
        total += n;
    }
    assert!(total > 3000, "total legacy corpus only {total} buffers");
    let d = dispatch_corpus().len();
    eprintln!("dispatch corpus -> {d} buffers");
    assert!(d > 1500, "dispatch corpus only {d} buffers");
}

#[test]
fn legacy_is_error_and_get_error_name() {
    let is_err_syms = [
        "ZSTDv01_isError",
        "ZSTDv02_isError",
        "ZSTDv03_isError",
        "ZSTDv05_isError",
        "ZSTDv06_isError",
        "ZSTDv07_isError",
        "ZBUFFv04_isError",
        "ZBUFFv05_isError",
        "ZBUFFv06_isError",
        "ZBUFFv07_isError",
        "FSEv05_isError",
        "FSEv06_isError",
        "FSEv07_isError",
        "HUFv05_isError",
        "HUFv07_isError",
    ];
    let name_syms = [
        "ZSTDv05_getErrorName",
        "ZSTDv06_getErrorName",
        "ZSTDv07_getErrorName",
        "ZBUFFv04_getErrorName",
        "ZBUFFv05_getErrorName",
        "ZBUFFv06_getErrorName",
        "ZBUFFv07_getErrorName",
        "FSEv05_getErrorName",
        "FSEv06_getErrorName",
        "FSEv07_getErrorName",
        "HUFv05_getErrorName",
        "HUFv07_getErrorName",
    ];

    let mut codes: Vec<usize> = Vec::new();
    for i in 0..=200usize {
        codes.push(i);
        codes.push(0usize.wrapping_sub(i));
    }
    codes.push(usize::MAX);
    codes.push(usize::MAX / 2);
    codes.push(1 << 20);
    let mut rng = Rng::new(0xE770);
    for _ in 0..200 {
        codes.push(rng.next_u64() as usize);
    }

    unsafe {
        for s in is_err_syms {
            let (c, r) = duo::<FnU32Sz>(s);
            for &code in &codes {
                eqv(&format!("{s}({code})"), c(code), r(code));
            }
        }
        for s in name_syms {
            let (c, r) = duo::<FnErrName>(s);
            for &code in &codes {
                eqv(&format!("{s}({code})"), cstr(c(code)), cstr(r(code)));
            }
        }
    }
}

#[test]
fn legacy_scalar_constants_and_ctx_sizes() {
    unsafe {
        for s in [
            "ZBUFFv04_recommendedDInSize",
            "ZBUFFv04_recommendedDOutSize",
            "ZBUFFv05_recommendedDInSize",
            "ZBUFFv05_recommendedDOutSize",
            "ZBUFFv06_recommendedDInSize",
            "ZBUFFv06_recommendedDOutSize",
            "ZBUFFv07_recommendedDInSize",
            "ZBUFFv07_recommendedDOutSize",
            "ZSTDv05_sizeofDCtx",
            "ZSTDv06_sizeofDCtx",
            "ZSTDv07_estimateDCtxSize",
        ] {
            let (c, r) = duo::<FnSz0>(s);
            eqv(s, c(), r());
        }

        // ZSTDv07_sizeofDCtx takes the context.
        let (cc, cr) = duo::<FnCreate>("ZSTDv07_createDCtx");
        let (fc, fr) = duo::<FnFree>("ZSTDv07_freeDCtx");
        let (sc, sr) = duo::<FnSizeofPtr>("ZSTDv07_sizeofDCtx");
        let c = cc();
        let r = cr();
        assert!(!c.is_null() && !r.is_null());
        eqv("ZSTDv07_sizeofDCtx", sc(c), sr(r));
        eqv("ZSTDv07_freeDCtx", fc(c), fr(r));
        // free(NULL) is documented to be a no-op returning 0
        eqv("ZSTDv07_freeDCtx(NULL)", fc(std::ptr::null_mut()), fr(std::ptr::null_mut()));

        // HUFv07_selectDecoder is a pure function of two sizes.  Its documented
        // contract is `0 < cSrcSize < dstSize <= 128 KB`; outside that range the
        // C divides by `dstSize` (SIGFPE at dstSize==0) and indexes
        // `algoTime[cSrcSize*16/dstSize]` out of bounds, so the domain below is
        // deliberately restricted (see the report notes).
        let (hc, hr) = duo::<FnHufSelect>("HUFv07_selectDecoder");
        let mut rng = Rng::new(0x5E1EC7);
        for dst in [2usize, 3, 10, 100, 1000, 10_000, 100_000, 128 * 1024] {
            for csrc in [1usize, 2, 3, 10, 100, 1000, 10_000, 100_000] {
                if csrc >= dst {
                    continue;
                }
                eqv(&format!("HUFv07_selectDecoder({dst},{csrc})"), hc(dst, csrc), hr(dst, csrc));
            }
        }
        for _ in 0..1000 {
            let d = 2 + rng.below(128 * 1024 - 1);
            let c2 = 1 + rng.below(d - 1);
            eqv(&format!("HUFv07_selectDecoder({d},{c2})"), hc(d, c2), hr(d, c2));
        }
    }
}

// ===========================================================================
// row 158 — v0.1
// ===========================================================================

#[test]
fn legacy_v01_decoders() {
    unsafe {
        drive_oneshot(
            Ver::V01,
            "ZSTDv01_decompress",
            Some(("ZSTDv01_createDCtx", "ZSTDv01_freeDCtx", "ZSTDv01_decompressDCtx")),
        );
        drive_fsil(Ver::V01, "ZSTDv01_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V01,
            "ZSTDv01_createDCtx",
            "ZSTDv01_freeDCtx",
            "ZSTDv01_nextSrcSizeToDecompress",
            "ZSTDv01_decompressContinue",
            Some("ZSTDv01_resetDCtx"),
        );
    }
}

// ===========================================================================
// row 159 — v0.2
// ===========================================================================

#[test]
fn legacy_v02_decoders() {
    unsafe {
        drive_oneshot(Ver::V02, "ZSTDv02_decompress", None);
        drive_fsil(Ver::V02, "ZSTDv02_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V02,
            "ZSTDv02_createDCtx",
            "ZSTDv02_freeDCtx",
            "ZSTDv02_nextSrcSizeToDecompress",
            "ZSTDv02_decompressContinue",
            Some("ZSTDv02_resetDCtx"),
        );
    }
}

// ===========================================================================
// row 160 — v0.3
// ===========================================================================

#[test]
fn legacy_v03_decoders() {
    unsafe {
        drive_oneshot(Ver::V03, "ZSTDv03_decompress", None);
        drive_fsil(Ver::V03, "ZSTDv03_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V03,
            "ZSTDv03_createDCtx",
            "ZSTDv03_freeDCtx",
            "ZSTDv03_nextSrcSizeToDecompress",
            "ZSTDv03_decompressContinue",
            Some("ZSTDv03_resetDCtx"),
        );
    }
}

// ===========================================================================
// row 161 — v0.4 (+ ZBUFFv04)
// ===========================================================================

#[test]
fn legacy_v04_decoders() {
    unsafe {
        drive_oneshot(
            Ver::V04,
            "ZSTDv04_decompress",
            Some(("ZSTDv04_createDCtx", "ZSTDv04_freeDCtx", "ZSTDv04_decompressDCtx")),
        );
        drive_fsil(Ver::V04, "ZSTDv04_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V04,
            "ZSTDv04_createDCtx",
            "ZSTDv04_freeDCtx",
            "ZSTDv04_nextSrcSizeToDecompress",
            "ZSTDv04_decompressContinue",
            Some("ZSTDv04_resetDCtx"),
        );
    }
}

#[test]
fn legacy_v04_zbuff_streaming() {
    unsafe {
        drive_zbuff(Ver::V04, "ZBUFFv04", true);
    }
}

// ===========================================================================
// rows 162-164 — v0.5 / v0.6 / v0.7 one-shot + direct streaming
// ===========================================================================

#[test]
fn legacy_v05_decoders() {
    unsafe {
        drive_oneshot(
            Ver::V05,
            "ZSTDv05_decompress",
            Some(("ZSTDv05_createDCtx", "ZSTDv05_freeDCtx", "ZSTDv05_decompressDCtx")),
        );
        drive_fsil(Ver::V05, "ZSTDv05_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V05,
            "ZSTDv05_createDCtx",
            "ZSTDv05_freeDCtx",
            "ZSTDv05_nextSrcSizeToDecompress",
            "ZSTDv05_decompressContinue",
            Some("ZSTDv05_decompressBegin"),
        );
    }
}

#[test]
fn legacy_v06_decoders() {
    unsafe {
        drive_oneshot(
            Ver::V06,
            "ZSTDv06_decompress",
            Some(("ZSTDv06_createDCtx", "ZSTDv06_freeDCtx", "ZSTDv06_decompressDCtx")),
        );
        drive_fsil(Ver::V06, "ZSTDv06_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V06,
            "ZSTDv06_createDCtx",
            "ZSTDv06_freeDCtx",
            "ZSTDv06_nextSrcSizeToDecompress",
            "ZSTDv06_decompressContinue",
            Some("ZSTDv06_decompressBegin"),
        );
    }
}

#[test]
fn legacy_v07_decoders() {
    unsafe {
        drive_oneshot(
            Ver::V07,
            "ZSTDv07_decompress",
            Some(("ZSTDv07_createDCtx", "ZSTDv07_freeDCtx", "ZSTDv07_decompressDCtx")),
        );
        drive_fsil(Ver::V07, "ZSTDv07_findFrameSizeInfoLegacy");
        drive_direct_stream(
            Ver::V07,
            "ZSTDv07_createDCtx",
            "ZSTDv07_freeDCtx",
            "ZSTDv07_nextSrcSizeToDecompress",
            "ZSTDv07_decompressContinue",
            Some("ZSTDv07_decompressBegin"),
        );
    }
}

/// Ground-truth check: hand-built v0.4 … v0.7 frames whose *expected* output is
/// known exactly.  Confirms both libraries really run the block decoder (RAW,
/// RLE and — for v0.5/0.6/0.7 — a compressed block with a RAW/RLE literals
/// section and `nbSeq == 0`) and agree with the reference semantics.
#[test]
fn legacy_crafted_frames_decode_to_known_output() {
    unsafe {
        for v in [Ver::V05, Ver::V06, Ver::V07] {
            let (dc, dr) = duo::<FnDec>(&format!("ZSTD{}_decompress", v.tag()));
            let mut rng = Rng::new(0x60_0D00 + v.n() as u64);
            let mut ok = 0usize;
            for k in 0..300 {
                let mut frame = frame_header_canonical(v, &mut rng);
                let mut expect: Vec<u8> = Vec::new();
                let nblocks = 1 + (k % 4);
                for j in 0..nblocks {
                    match (k + j) % 3 {
                        // NOTE: n >= 1 — a *zero-length* RAW block makes
                        // `ZSTD_getcBlockSize()` return 0, and v0.1 … v0.6 end the
                        // frame on `cBlockSize == 0` rather than on `bt_end`, so
                        // such a block silently truncates the frame.  (The random
                        // corpus does generate them; both libraries agree.)
                        0 => {
                            let n = 1 + rng.below(500);
                            let d = rng.bytes(n);
                            push_block_header(&mut frame, 1, n as u32);
                            frame.extend_from_slice(&d);
                            expect.extend_from_slice(&d);
                        }
                        1 if v == Ver::V07 => {
                            // `bt_rle` is only implemented by v0.7 — v0.1 … v0.6
                            // all `return ERROR(GENERIC) /* not yet supported */`.
                            let n = 1 + rng.below(3000);
                            let byte = rng.byte();
                            push_block_header(&mut frame, 2, n as u32);
                            frame.push(byte);
                            expect.extend(std::iter::repeat(byte).take(n));
                        }
                        1 => {
                            let n = 1 + rng.below(600);
                            let d = rng.bytes(n);
                            push_block_header(&mut frame, 1, n as u32);
                            frame.extend_from_slice(&d);
                            expect.extend_from_slice(&d);
                        }
                        _ => {
                            let rle = (k + j) % 2 == 0;
                            let payload = v567_compressed_block(&mut rng, rle);
                            push_block_header(&mut frame, 0, payload.len() as u32);
                            frame.extend_from_slice(&payload);
                            let lit = (payload[0] & 31) as usize;
                            if rle {
                                expect.extend(std::iter::repeat(payload[1]).take(lit));
                            } else {
                                expect.extend_from_slice(&payload[1..1 + lit]);
                            }
                        }
                    }
                }
                push_block_header(&mut frame, 3, 0);

                let s = Src::new(&frame);
                let cap = expect.len() + 1024;
                let mut d = Dst::new(cap);
                let rc = dc(d.cp(), cap, s.ptr(), s.len);
                let rr = dr(d.rp(), cap, s.ptr(), s.len);
                eqv(&format!("ZSTD{}_decompress[known#{k}]", v.tag()), rc, rr);
                d.check(&format!("ZSTD{}_decompress[known#{k}] dst", v.tag()));
                assert!(
                    !is_err(rc),
                    "hand-built {} frame #{k} failed to decode: {rc}",
                    v.tag()
                );
                assert_eq!(rc, expect.len(), "{} frame #{k}: wrong regenerated size", v.tag());
                eqbuf(&format!("ZSTD{}[known#{k}] content", v.tag()), &d.c[..rc], &expect);
                ok += 1;

                // exact-size destination, and one byte too small
                let mut d2 = Dst::new(expect.len());
                let rc2 = dc(d2.cp(), expect.len(), s.ptr(), s.len);
                let rr2 = dr(d2.rp(), expect.len(), s.ptr(), s.len);
                eqv(&format!("ZSTD{}_decompress[known#{k} exact]", v.tag()), rc2, rr2);
                d2.check(&format!("ZSTD{}_decompress[known#{k} exact] dst", v.tag()));
                if !expect.is_empty() {
                    let small = expect.len() - 1;
                    let mut d3 = Dst::new(small);
                    let rc3 = dc(d3.cp(), small, s.ptr(), s.len);
                    let rr3 = dr(d3.rp(), small, s.ptr(), s.len);
                    eqv(&format!("ZSTD{}_decompress[known#{k} small]", v.tag()), rc3, rr3);
                    d3.check(&format!("ZSTD{}_decompress[known#{k} small] dst", v.tag()));
                }
            }
            assert_eq!(ok, 300);
        }

        // v0.1 … v0.4: RAW blocks only — all four reject `bt_rle` outright
        // (`return ERROR(GENERIC); /* not yet supported */`).
        for v in [Ver::V01, Ver::V02, Ver::V03, Ver::V04] {
            let (dc, dr) = duo::<FnDec>(&format!("ZSTD{}_decompress", v.tag()));
            let mut rng = Rng::new(0x60_0E00 + v.n() as u64);
            for k in 0..200 {
                let mut frame = frame_header_canonical(v, &mut rng);
                let mut expect: Vec<u8> = Vec::new();
                for _ in 0..(1 + k % 3) {
                    let n = 1 + rng.below(500);
                    let d = rng.bytes(n);
                    push_block_header(&mut frame, 1, n as u32);
                    frame.extend_from_slice(&d);
                    expect.extend_from_slice(&d);
                }
                push_block_header(&mut frame, 3, 0);
                let s = Src::new(&frame);
                let cap = expect.len() + 1024;
                let mut d = Dst::new(cap);
                let rc = dc(d.cp(), cap, s.ptr(), s.len);
                let rr = dr(d.rp(), cap, s.ptr(), s.len);
                eqv(&format!("ZSTD{}_decompress[raw#{k}]", v.tag()), rc, rr);
                d.check(&format!("ZSTD{}_decompress[raw#{k}] dst", v.tag()));
                assert!(!is_err(rc), "hand-built {} RAW frame #{k} failed: {rc}", v.tag());
                assert_eq!(rc, expect.len(), "{} RAW frame #{k}: wrong size", v.tag());
                eqbuf(&format!("ZSTD{}[raw#{k}] content", v.tag()), &d.c[..rc], &expect);
            }
        }
    }
}

type FnXxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;

/// v0.7 is the only legacy version that carries a frame checksum, and it hides
/// it in the low 22 bits of the *end-block header*
/// (`check32 == ip[2] + (ip[1]<<8) + ((ip[0]&0x3F)<<16)` vs
/// `(XXH64(content,0) >> 11) & 0x3FFFFF`).  Only the *streaming* decoders
/// (`ZSTDv07_decompressContinue`, `ZBUFFv07`, `ZSTD_decompressStream`) verify
/// it, so this test drives those.
#[test]
fn legacy_v07_checksummed_frames_streaming() {
    unsafe {
        let (xxh, _) = duo::<FnXxh64>("ZSTD_XXH64");
        let (cc, cr) = duo::<FnCreate>("ZSTDv07_createDCtx");
        let (fc, fr) = duo::<FnFree>("ZSTDv07_freeDCtx");
        let (nc, nr) = duo::<FnSz1>("ZSTDv07_nextSrcSizeToDecompress");
        let (kc, kr) = duo::<FnDecCtx>("ZSTDv07_decompressContinue");
        let (zc, zr) = zb_api("ZBUFFv07", false);
        let nodict = Src::new(&[]);

        let mut rng = Rng::new(0xC0DEC5);
        let mut good = 0usize;
        let mut bad = 0usize;
        let mut zb_completed = 0usize;

        for k in 0..150 {
            // canonical header + checksumFlag (bit 2 of the descriptor byte)
            let mut frame = Ver::V07.magic_bytes().to_vec();
            frame.push(0x04);
            frame.push((rng.below(18) as u8) << 3);
            let mut content: Vec<u8> = Vec::new();
            for _ in 0..(1 + k % 4) {
                // the streaming path rejects `bt_rle`, so RAW + compressed only
                if k % 2 == 0 {
                    let n = 1 + rng.below(700);
                    let d = rng.bytes(n);
                    push_block_header(&mut frame, 1, n as u32);
                    frame.extend_from_slice(&d);
                    content.extend_from_slice(&d);
                } else {
                    let payload = v567_compressed_block(&mut rng, k % 4 == 1);
                    push_block_header(&mut frame, 0, payload.len() as u32);
                    frame.extend_from_slice(&payload);
                    let lit = (payload[0] & 31) as usize;
                    if k % 4 == 1 {
                        content.extend(std::iter::repeat(payload[1]).take(lit));
                    } else {
                        content.extend_from_slice(&payload[1..1 + lit]);
                    }
                }
            }
            let h64 = xxh(content.as_ptr() as *const c_void, content.len(), 0);
            let h32 = ((h64 >> 11) & ((1 << 22) - 1)) as u32;
            let mut ok_frame = frame.clone();
            ok_frame.push(0xC0 | ((h32 >> 16) & 0x3F) as u8);
            ok_frame.push((h32 >> 8) as u8);
            ok_frame.push(h32 as u8);
            let mut bad_frame = frame.clone();
            let wrong = h32 ^ 1;
            bad_frame.push(0xC0 | ((wrong >> 16) & 0x3F) as u8);
            bad_frame.push((wrong >> 8) as u8);
            bad_frame.push(wrong as u8);

            for (which, f) in [("good", &ok_frame), ("bad", &bad_frame)] {
                let s = Src::new(f);
                // --- direct streaming
                let cap = content.len() + 4096;
                let mut d = Dst::new(cap);
                let mut tc: Vec<(usize, usize)> = Vec::new();
                let mut tr: Vec<(usize, usize)> = Vec::new();
                for lib in 0..2 {
                    let ctx = if lib == 0 { cc() } else { cr() };
                    let base = if lib == 0 { d.cp() } else { d.rp() };
                    let mut op = 0usize;
                    let mut ip = 0usize;
                    let mut steps = 0;
                    loop {
                        steps += 1;
                        if steps > 300 {
                            break;
                        }
                        let exp = if lib == 0 { nc(ctx) } else { nr(ctx) };
                        if exp == 0 || is_err(exp) || ip + exp > s.len {
                            if lib == 0 {
                                tc.push((exp, usize::MAX));
                            } else {
                                tr.push((exp, usize::MAX));
                            }
                            break;
                        }
                        let ret = if lib == 0 {
                            kc(ctx, (base as *mut u8).add(op) as *mut c_void, cap - op, s.at(ip), exp)
                        } else {
                            kr(ctx, (base as *mut u8).add(op) as *mut c_void, cap - op, s.at(ip), exp)
                        };
                        if lib == 0 {
                            tc.push((exp, ret));
                        } else {
                            tr.push((exp, ret));
                        }
                        if is_err(ret) {
                            break;
                        }
                        op += ret;
                        ip += exp;
                    }
                    if lib == 0 {
                        fc(ctx);
                    } else {
                        fr(ctx);
                    }
                }
                eqv(&format!("v07 checksum {which}#{k} continue trace"), tc.clone(), tr);
                d.check(&format!("v07 checksum {which}#{k} continue dst"));
                if which == "good" {
                    assert!(
                        !tc.iter().any(|&(_, r)| r != usize::MAX && is_err(r)),
                        "correctly-checksummed v0.7 frame #{k} was rejected: {tc:?}"
                    );
                    assert_eq!(
                        tc.last().copied(),
                        Some((0usize, usize::MAX)),
                        "good frame #{k} did not run to completion: {tc:?}"
                    );
                    eqbuf(
                        &format!("v07 checksum good#{k} content"),
                        &d.c[..content.len()],
                        &content,
                    );
                    good += 1;
                } else {
                    assert!(
                        tc.iter().any(|&(_, r)| r != usize::MAX && is_err(r)),
                        "wrong checksum must be detected (frame #{k})"
                    );
                    bad += 1;
                }

                // --- buffered streaming, several chunk shapes
                let mut bad_rejected = false;
                for &(inc, outc) in &[(1usize, 1usize), (5, 97), (1 << 17, 1 << 17)] {
                    let (t1, o1) = zb_run(&zc, &nodict, &s, inc, outc);
                    let (t2, o2) = zb_run(&zr, &nodict, &s, inc, outc);
                    eqv(&format!("ZBUFFv07 checksum {which}#{k} ({inc},{outc}) trace"), t1.clone(), t2);
                    eqbuf(&format!("ZBUFFv07 checksum {which}#{k} ({inc},{outc}) out"), &o1, &o2);
                    if which == "good" {
                        // the (1,1) shape can hit the step limit before the frame
                        // ends, so only fully-drained runs are content-checked
                        if t1.last().map(|&(r, _, _)| r == 0).unwrap_or(false) {
                            eqbuf(&format!("ZBUFFv07 checksum good#{k} content"), &o1, &content);
                            zb_completed += 1;
                        }
                        assert!(
                            !t1.iter().any(|&(r, _, _)| is_err(r)),
                            "good frame #{k} errored via ZBUFFv07: {t1:?}"
                        );
                    } else if t1.iter().any(|&(r, _, _)| is_err(r)) {
                        bad_rejected = true;
                    }
                }
                if which == "bad" {
                    assert!(
                        bad_rejected,
                        "ZBUFFv07 must reject a wrong checksum (frame #{k})"
                    );
                }
            }
        }
        assert_eq!(good, 150);
        assert_eq!(bad, 150);
        assert!(zb_completed > 0, "ZBUFFv07 never fully drained a checksummed frame");
    }
}

// ---------------------------------------------------------------------------
// v0.5 / v0.6 advanced surface
// ---------------------------------------------------------------------------

/// v0.5 `ZSTDv05_parameters` (U64 + 7×U32)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct V05Params {
    srcSize: u64,
    windowLog: c_uint,
    contentLog: c_uint,
    hashLog: c_uint,
    searchLog: c_uint,
    searchLength: c_uint,
    targetLength: c_uint,
    strategy: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct V06FrameParams {
    frameContentSize: c_ulonglong,
    windowLog: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct V07FrameParams {
    frameContentSize: c_ulonglong,
    windowSize: c_uint,
    dictID: c_uint,
    checksumFlag: c_uint,
}

#[test]
fn legacy_v05_get_frame_params() {
    unsafe {
        let (gc, gr) = duo::<FnGetFP>("ZSTDv05_getFrameParams");
        for (i, b) in corpus(Ver::V05, 0x9051).iter().enumerate() {
            let s = Src::new(b);
            let mut pc = V05Params { srcSize: 0xAAAA, windowLog: 0xBBBB, ..Default::default() };
            let mut pr = pc;
            let rc = gc(&mut pc as *mut _ as *mut c_void, s.ptr(), s.len);
            let rr = gr(&mut pr as *mut _ as *mut c_void, s.ptr(), s.len);
            eqv(&format!("ZSTDv05_getFrameParams[buf{i}] ret"), rc, rr);
            eqv(&format!("ZSTDv05_getFrameParams[buf{i}] params"), pc, pr);
        }
    }
}

#[test]
fn legacy_v06_get_frame_params() {
    unsafe {
        let (gc, gr) = duo::<FnGetFP>("ZSTDv06_getFrameParams");
        for (i, b) in corpus(Ver::V06, 0x9061).iter().enumerate() {
            let s = Src::new(b);
            let mut pc = V06FrameParams { frameContentSize: 0xAAAA, windowLog: 0xBBBB };
            let mut pr = pc;
            let rc = gc(&mut pc as *mut _ as *mut c_void, s.ptr(), s.len);
            let rr = gr(&mut pr as *mut _ as *mut c_void, s.ptr(), s.len);
            eqv(&format!("ZSTDv06_getFrameParams[buf{i}] ret"), rc, rr);
            eqv(&format!("ZSTDv06_getFrameParams[buf{i}] params"), pc, pr);
        }
    }
}

#[test]
fn legacy_v07_get_frame_params_and_decompressed_size() {
    unsafe {
        let (gc, gr) = duo::<FnGetFP>("ZSTDv07_getFrameParams");
        let (sc, sr) = duo::<FnU64Src>("ZSTDv07_getDecompressedSize");
        let mut bufs = corpus(Ver::V07, 0x9071);
        // also feed the skippable-frame magics that v0.7 recognises
        let mut rng = Rng::new(0x9072);
        for lo in 0..16u32 {
            for &n in &[3usize, 4, 7, 8, 16, 100] {
                let mut b = (0x184D2A50u32 + lo).to_le_bytes().to_vec();
                let extra = rng.bytes(n);
                b.extend_from_slice(&extra);
                bufs.push(b);
            }
        }
        for (i, b) in bufs.iter().enumerate() {
            let s = Src::new(b);
            let mut pc = V07FrameParams {
                frameContentSize: 0xAAAA,
                windowSize: 0xBBBB,
                dictID: 0xCCCC,
                checksumFlag: 0xDDDD,
            };
            let mut pr = pc;
            let rc = gc(&mut pc as *mut _ as *mut c_void, s.ptr(), s.len);
            let rr = gr(&mut pr as *mut _ as *mut c_void, s.ptr(), s.len);
            eqv(&format!("ZSTDv07_getFrameParams[buf{i}] ret"), rc, rr);
            eqv(&format!("ZSTDv07_getFrameParams[buf{i}] params"), pc, pr);
            eqv(&format!("ZSTDv07_getDecompressedSize[buf{i}]"), sc(s.ptr(), s.len), sr(s.ptr(), s.len));
        }
    }
}

/// `decompress_usingDict`, `decompressBegin_usingDict`, `copyDCtx`,
/// `decompressBlock`, `decompress_usingPreparedDCtx` for v0.5 / v0.6.
unsafe fn drive_v05_v06_advanced(v: Ver, p: &str) {
    let (cc, cr) = duo::<FnCreate>(&format!("{p}_createDCtx"));
    let (fc, fr) = duo::<FnFree>(&format!("{p}_freeDCtx"));
    let (udc, udr) = duo::<FnDecDict>(&format!("{p}_decompress_usingDict"));
    let (bdc, bdr) = duo::<FnInitDict>(&format!("{p}_decompressBegin_usingDict"));
    let (blc, blr) = duo::<FnDecCtx>(&format!("{p}_decompressBlock"));
    let (cpc, cpr) = duo::<FnCopyDCtx>(&format!("{p}_copyDCtx"));
    let (ppc, ppr) = duo::<FnDecPrepared>(&format!("{p}_decompress_usingPreparedDCtx"));

    let bufs = corpus(v, 0xAD_0000 + v.n() as u64);
    let dicts: Vec<Src> = (0..5)
        .map(|i| Src::new(&gen_class(i, [0usize, 1, 100, 8192, 70_000][i], 0xD1C7 + i as u64)))
        .collect();

    let c1 = cc();
    let r1 = cr();
    let c2 = cc();
    let r2 = cr();
    assert!(!c1.is_null() && !r1.is_null() && !c2.is_null() && !r2.is_null());

    for (i, b) in bufs.iter().enumerate() {
        let s = Src::new(b);
        let dict = &dicts[i % dicts.len()];
        for &cap in &[0usize, 300, 1 << 18] {
            let mut d = Dst::new(cap);
            let rc = udc(c1, d.cp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
            let rr = udr(r1, d.rp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
            eqv(&format!("{p}_decompress_usingDict[buf{i} cap{cap}]"), rc, rr);
            d.check(&format!("{p}_decompress_usingDict[buf{i} cap{cap}] dst"));
        }

        // prepared-DCtx path: prepare a reference context, then copy it.
        eqv(
            &format!("{p}_decompressBegin_usingDict[buf{i}]"),
            bdc(c2, dict.ptr(), dict.len),
            bdr(r2, dict.ptr(), dict.len),
        );
        {
            let cap = 1usize << 18;
            let mut d = Dst::new(cap);
            let rc = ppc(c1, c2 as *const c_void, d.cp(), cap, s.ptr(), s.len);
            let rr = ppr(r1, r2 as *const c_void, d.rp(), cap, s.ptr(), s.len);
            eqv(&format!("{p}_decompress_usingPreparedDCtx[buf{i}]"), rc, rr);
            d.check(&format!("{p}_decompress_usingPreparedDCtx[buf{i}] dst"));
        }

        // explicit copyDCtx then decompressBlock on the *raw* block payload
        cpc(c1, c2 as *const c_void);
        cpr(r1, r2 as *const c_void);
        if i % 2 == 0 {
            let cap = 1usize << 18;
            let mut d = Dst::new(cap);
            // skip the frame header, feed what follows as a "block"
            let off = (4 + (i % 3)).min(s.len);
            let rc = blc(c1, d.cp(), cap, s.at(off), s.len - off);
            let rr = blr(r1, d.rp(), cap, s.at(off), s.len - off);
            eqv(&format!("{p}_decompressBlock[buf{i}]"), rc, rr);
            d.check(&format!("{p}_decompressBlock[buf{i}] dst"));
        }
    }

    eqv(&format!("{p}_freeDCtx#1"), fc(c1), fr(r1));
    eqv(&format!("{p}_freeDCtx#2"), fc(c2), fr(r2));
}

#[test]
fn legacy_v05_advanced() {
    unsafe { drive_v05_v06_advanced(Ver::V05, "ZSTDv05") }
}

#[test]
fn legacy_v06_advanced() {
    unsafe { drive_v05_v06_advanced(Ver::V06, "ZSTDv06") }
}

#[test]
fn legacy_v07_advanced() {
    unsafe {
        let (cc, cr) = duo::<FnCreate>("ZSTDv07_createDCtx");
        let (cac, car) = duo::<FnCreateAdv>("ZSTDv07_createDCtx_advanced");
        let (fc, fr) = duo::<FnFree>("ZSTDv07_freeDCtx");
        let (udc, udr) = duo::<FnDecDict>("ZSTDv07_decompress_usingDict");
        let (bdc, bdr) = duo::<FnInitDict>("ZSTDv07_decompressBegin_usingDict");
        let (bgc, bgr) = duo::<FnSz1>("ZSTDv07_decompressBegin");
        let (blc, blr) = duo::<FnDecCtx>("ZSTDv07_decompressBlock");
        let (cpc, cpr) = duo::<FnCopyDCtx>("ZSTDv07_copyDCtx");
        let (skc, skr) = duo::<FnIsSkip>("ZSTDv07_isSkipFrame");
        let (ibc, ibr) = duo::<FnInitDict>("ZSTDv07_insertBlock");
        let (ddc, ddr) = duo::<FnCreateDDict>("ZSTDv07_createDDict");
        let (dfc, dfr) = duo::<FnFree>("ZSTDv07_freeDDict");
        let (uwc, uwr) = duo::<FnDecDDict>("ZSTDv07_decompress_usingDDict");

        // createDCtx_advanced with the *same* custom allocator in both libs
        let cm = custom_mem();
        let ac = cac(cm);
        let ar = car(cm);
        assert!(!ac.is_null(), "ZSTDv07_createDCtx_advanced NULL in C");
        assert!(!ar.is_null(), "ZSTDv07_createDCtx_advanced NULL in Rust");
        // an allocator with only one of the two hooks must be rejected
        let half = ZSTD_customMem {
            customAlloc: Some(test_alloc),
            customFree: None,
            opaque: std::ptr::null_mut(),
        };
        eqv("createDCtx_advanced(half).is_null", cac(half).is_null(), car(half).is_null());

        let c1 = cc();
        let r1 = cr();
        let c2 = cc();
        let r2 = cr();
        assert!(!c1.is_null() && !r1.is_null() && !c2.is_null() && !r2.is_null());

        let bufs = corpus(Ver::V07, 0xAD_0007);
        let dicts: Vec<Src> = (0..5)
            .map(|i| Src::new(&gen_class(i, [0usize, 1, 100, 8192, 70_000][i], 0xD1C8 + i as u64)))
            .collect();
        let ddicts: Vec<(*mut c_void, *mut c_void)> =
            dicts.iter().map(|d| (ddc(d.ptr(), d.len), ddr(d.ptr(), d.len))).collect();
        for (i, (a, b)) in ddicts.iter().enumerate() {
            eqv(&format!("createDDict[{i}].is_null"), a.is_null(), b.is_null());
        }

        for (i, b) in bufs.iter().enumerate() {
            let s = Src::new(b);
            let di = i % dicts.len();
            let dict = &dicts[di];
            for &cap in &[0usize, 300, 1 << 18] {
                let mut d = Dst::new(cap);
                let rc = udc(c1, d.cp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
                let rr = udr(r1, d.rp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
                eqv(&format!("ZSTDv07_decompress_usingDict[buf{i} cap{cap}]"), rc, rr);
                d.check(&format!("ZSTDv07_decompress_usingDict[buf{i} cap{cap}] dst"));
            }
            // same through the DDict object.
            // NOTE: the DDict path goes through `ZSTDv07_decompress_usingPreparedDCtx`,
            // which `memcpy`s the *reference* context over `dctx` — including its
            // `customMem` — so a context built with a non-default allocator would
            // afterwards be released with the wrong `free`.  Hence a plain
            // `createDCtx` context is used here.
            {
                let cap = 1usize << 18;
                let mut d = Dst::new(cap);
                let (a, bb) = ddicts[di];
                let rc = uwc(c1, d.cp(), cap, s.ptr(), s.len, a as *const c_void);
                let rr = uwr(r1, d.rp(), cap, s.ptr(), s.len, bb as *const c_void);
                eqv(&format!("ZSTDv07_decompress_usingDDict[buf{i}]"), rc, rr);
                d.check(&format!("ZSTDv07_decompress_usingDDict[buf{i}] dst"));
            }
            // the custom-allocator context: `decompress_usingDict` does *not*
            // copy a reference context, so its allocator survives.
            if i % 5 == 0 {
                let cap = 1usize << 18;
                let mut d = Dst::new(cap);
                let rc = udc(ac, d.cp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
                let rr = udr(ar, d.rp(), cap, s.ptr(), s.len, dict.ptr(), dict.len);
                eqv(&format!("ZSTDv07_decompress_usingDict(adv)[buf{i}]"), rc, rr);
                d.check(&format!("ZSTDv07_decompress_usingDict(adv)[buf{i}] dst"));
                eqv(&format!("ZSTDv07_isSkipFrame(adv)[buf{i}]"), skc(ac), skr(ar));
            }

            eqv(
                &format!("ZSTDv07_decompressBegin_usingDict[buf{i}]"),
                bdc(c2, dict.ptr(), dict.len),
                bdr(r2, dict.ptr(), dict.len),
            );
            eqv(&format!("ZSTDv07_isSkipFrame[buf{i}]"), skc(c2), skr(r2));
            cpc(c1, c2 as *const c_void);
            cpr(r1, r2 as *const c_void);
            eqv(&format!("ZSTDv07_isSkipFrame(copy)[buf{i}]"), skc(c1), skr(r1));

            if i % 2 == 0 {
                let cap = 1usize << 18;
                let mut d = Dst::new(cap);
                let off = (4 + (i % 3)).min(s.len);
                let rc = blc(c1, d.cp(), cap, s.at(off), s.len - off);
                let rr = blr(r1, d.rp(), cap, s.at(off), s.len - off);
                eqv(&format!("ZSTDv07_decompressBlock[buf{i}]"), rc, rr);
                d.check(&format!("ZSTDv07_decompressBlock[buf{i}] dst"));
            }

            if i % 3 == 0 {
                eqv(&format!("ZSTDv07_decompressBegin[buf{i}]"), bgc(c1), bgr(r1));
                let n = s.len.min(1000);
                eqv(
                    &format!("ZSTDv07_insertBlock[buf{i}]"),
                    ibc(c1, s.ptr(), n),
                    ibr(r1, s.ptr(), n),
                );
            }
        }

        for (i, (a, b)) in ddicts.iter().enumerate() {
            eqv(&format!("ZSTDv07_freeDDict[{i}]"), dfc(*a), dfr(*b));
        }
        // NOTE: `ZSTDv07_freeDDict(NULL)` dereferences `ddict->refContext`
        // unconditionally and segfaults in the reference C build too, so NULL is
        // deliberately not exercised here (see report).
        eqv("ZSTDv07_freeDCtx#1", fc(c1), fr(r1));
        eqv("ZSTDv07_freeDCtx#2", fc(c2), fr(r2));
        eqv("ZSTDv07_freeDCtx(adv)", fc(ac), fr(ar));
    }
}

// ---------------------------------------------------------------------------
// ZBUFF streaming for v0.5 / v0.6 / v0.7
// ---------------------------------------------------------------------------

#[test]
fn legacy_v05_zbuff_streaming() {
    unsafe { drive_zbuff(Ver::V05, "ZBUFFv05", false) }
}

#[test]
fn legacy_v06_zbuff_streaming() {
    unsafe { drive_zbuff(Ver::V06, "ZBUFFv06", false) }
}

#[test]
fn legacy_v07_zbuff_streaming() {
    unsafe {
        drive_zbuff(Ver::V07, "ZBUFFv07", false);

        // ZBUFFv07_createDCtx_advanced with a shared custom allocator
        let (cac, car) = duo::<FnCreateAdv>("ZBUFFv07_createDCtx_advanced");
        let (fc, fr) = duo::<FnFree>("ZBUFFv07_freeDCtx");
        let (ic, ir) = duo::<FnSz1>("ZBUFFv07_decompressInit");
        let (tc, tr) = duo::<FnZbCont>("ZBUFFv07_decompressContinue");

        let cm = custom_mem();
        let c = cac(cm);
        let r = car(cm);
        assert!(!c.is_null() && !r.is_null());
        eqv("ZBUFFv07_decompressInit(adv)", ic(c), ir(r));

        let bufs = corpus(Ver::V07, 0x2B_0107);
        for (i, b) in bufs.iter().enumerate().filter(|(i, _)| i % 7 == 0) {
            let s = Src::new(b);
            let cap = 1usize << 17;
            let mut d = Dst::new(cap);
            let mut dc = cap;
            let mut dr = cap;
            let mut sc = s.len;
            let mut sr = s.len;
            eqv(&format!("ZBUFFv07_decompressInit(adv)[buf{i}]"), ic(c), ir(r));
            let rc = tc(c, d.cp(), &mut dc, s.ptr(), &mut sc);
            let rr = tr(r, d.rp(), &mut dr, s.ptr(), &mut sr);
            eqv(&format!("ZBUFFv07(adv)[buf{i}] ret"), rc, rr);
            eqv(&format!("ZBUFFv07(adv)[buf{i}] dstSize"), dc, dr);
            eqv(&format!("ZBUFFv07(adv)[buf{i}] srcSize"), sc, sr);
            d.check(&format!("ZBUFFv07(adv)[buf{i}] dst"));
        }
        eqv("ZBUFFv07_freeDCtx(adv)", fc(c), fr(r));
        eqv("ZBUFFv07_freeDCtx(NULL)", fc(std::ptr::null_mut()), fr(std::ptr::null_mut()));
    }
}

// ===========================================================================
// FSEv05 / FSEv06 / FSEv07
// ===========================================================================

/// Random but *valid* normalized counter (sums to `1<<tableLog`).
fn norm_counts(rng: &mut Rng, max_sv: usize, table_log: u32) -> Option<Vec<i16>> {
    let total = 1i32 << table_log;
    let n = max_sv + 1;
    if n as i32 > total {
        return None;
    }
    let mut c = vec![1i16; n];
    let mut rem = total - n as i32;
    while rem > 0 {
        let i = rng.below(n);
        let add = 1 + rng.below(rem as usize);
        let add = add.min(30_000 - c[i] as usize);
        if add == 0 {
            continue;
        }
        c[i] += add as i16;
        rem -= add as i32;
    }
    Some(c)
}

unsafe fn drive_fse(p: &str) {
    let (crc, crr) = duo::<FnFseCreateDT>(&format!("{p}_createDTable"));
    let (frc, frr) = duo::<FnFseFreeDT>(&format!("{p}_freeDTable"));
    let (bdc, bdr) = duo::<FnFseBuildDT>(&format!("{p}_buildDTable"));
    let (brc, brr) = duo::<FnFseBuildRaw>(&format!("{p}_buildDTable_raw"));
    let (blc, blr) = duo::<FnFseBuildRle>(&format!("{p}_buildDTable_rle"));
    let (rnc, rnr) = duo::<FnFseReadNCount>(&format!("{p}_readNCount"));
    let (dc, dr) = duo::<FnDec>(&format!("{p}_decompress"));
    let (udc, udr) = duo::<FnFseDecDT>(&format!("{p}_decompress_usingDTable"));

    // createDTable / freeDTable round trip
    for tl in 0..=16u32 {
        let c = crc(tl);
        let r = crr(tl);
        eqv(&format!("{p}_createDTable({tl}).is_null"), c.is_null(), r.is_null());
        frc(c);
        frr(r);
    }
    frc(std::ptr::null_mut());
    frr(std::ptr::null_mut());

    // ---- buildDTable_raw / _rle: compare the full table image -------------
    const DTW: usize = 1 + (1 << 16) + 64; // over-allocated, in u32 units
    for nb in 0..=16u32 {
        let mut tc = vec![0u32; DTW];
        let mut tr = vec![0u32; DTW];
        let rc = brc(tc.as_mut_ptr(), nb);
        let rr = brr(tr.as_mut_ptr(), nb);
        eqv(&format!("{p}_buildDTable_raw({nb})"), rc, rr);
        eqv(&format!("{p}_buildDTable_raw({nb}) table"), tc, tr);
    }
    for sym in [0u8, 1, 2, 42, 127, 128, 200, 254, 255] {
        let mut tc = vec![0u32; DTW];
        let mut tr = vec![0u32; DTW];
        let rc = blc(tc.as_mut_ptr(), sym);
        let rr = blr(tr.as_mut_ptr(), sym);
        eqv(&format!("{p}_buildDTable_rle({sym})"), rc, rr);
        eqv(&format!("{p}_buildDTable_rle({sym}) table"), tc, tr);
    }

    // ---- buildDTable with valid normalized counters ----------------------
    let mut rng = Rng::new(0xF5E0 + p.len() as u64);
    let mut good_tables: Vec<Vec<u32>> = Vec::new();
    for _ in 0..1200 {
        let tl = 5 + rng.below(11) as u32; // 5..15
        let msv = rng.below(256);
        let counts = match norm_counts(&mut rng, msv, tl) {
            Some(c) => c,
            None => continue,
        };
        let mut tc = vec![0u32; DTW];
        let mut tr = vec![0u32; DTW];
        let rc = bdc(tc.as_mut_ptr(), counts.as_ptr(), msv as c_uint, tl);
        let rr = bdr(tr.as_mut_ptr(), counts.as_ptr(), msv as c_uint, tl);
        eqv(&format!("{p}_buildDTable(msv={msv},tl={tl})"), rc, rr);
        eqv(&format!("{p}_buildDTable(msv={msv},tl={tl}) table"), tc.clone(), tr);
        if !is_err(rc) && good_tables.len() < 12 {
            good_tables.push(tc);
        }
    }
    // out-of-range tableLog must be rejected identically
    for tl in [16u32, 17, 20, 31] {
        let counts = vec![1i16; 4];
        let mut tc = vec![0u32; DTW];
        let mut tr = vec![0u32; DTW];
        eqv(
            &format!("{p}_buildDTable(tl={tl})"),
            bdc(tc.as_mut_ptr(), counts.as_ptr(), 3, tl),
            bdr(tr.as_mut_ptr(), counts.as_ptr(), 3, tl),
        );
        eqv(&format!("{p}_buildDTable(tl={tl}) table"), tc, tr);
    }

    // ---- readNCount over random buffers ---------------------------------
    for i in 0..2000 {
        let n = if i < 40 { i } else { 1 + rng.below(600) };
        let buf = if i % 3 == 0 {
            rng.bytes(n)
        } else {
            gen_class(i % N_CLASSES, n, 0x5EE0 + i as u64)
        };
        let s = Src::new(&buf);
        let mut nc_c = vec![0i16; 512];
        let mut nc_r = vec![0i16; 512];
        for &start_msv in &[0u32, 3, 63, 255] {
            for &start_tl in &[0u32, 9, 12, 15] {
                let mut msv_c = start_msv;
                let mut msv_r = start_msv;
                let mut tl_c = start_tl;
                let mut tl_r = start_tl;
                for x in nc_c.iter_mut() {
                    *x = 0;
                }
                for x in nc_r.iter_mut() {
                    *x = 0;
                }
                let rc = rnc(nc_c.as_mut_ptr(), &mut msv_c, &mut tl_c, s.ptr(), s.len);
                let rr = rnr(nc_r.as_mut_ptr(), &mut msv_r, &mut tl_r, s.ptr(), s.len);
                let lbl = format!("{p}_readNCount[buf{i} msv{start_msv} tl{start_tl}]");
                eqv(&format!("{lbl} ret"), rc, rr);
                eqv(&format!("{lbl} maxSV"), msv_c, msv_r);
                eqv(&format!("{lbl} tableLog"), tl_c, tl_r);
                eqv(&format!("{lbl} counts"), nc_c.clone(), nc_r.clone());
            }
        }
    }

    // ---- decompress / decompress_usingDTable ----------------------------
    for i in 0..2500 {
        let n = if i < 40 { i } else { 1 + rng.below(2000) };
        let buf = if i % 2 == 0 {
            rng.bytes(n)
        } else {
            gen_class(i % N_CLASSES, n, 0x5EE1 + i as u64)
        };
        let s = Src::new(&buf);
        for &cap in &[0usize, 1, 17, 4096] {
            let mut d = Dst::new(cap);
            let rc = dc(d.cp(), cap, s.ptr(), s.len);
            let rr = dr(d.rp(), cap, s.ptr(), s.len);
            eqv(&format!("{p}_decompress[buf{i} cap{cap}]"), rc, rr);
            d.check(&format!("{p}_decompress[buf{i} cap{cap}] dst"));
        }
        if !good_tables.is_empty() {
            let dt = &good_tables[i % good_tables.len()];
            for &cap in &[0usize, 1, 300] {
                let mut d = Dst::new(cap);
                let rc = udc(d.cp(), cap, s.ptr(), s.len, dt.as_ptr());
                let rr = udr(d.rp(), cap, s.ptr(), s.len, dt.as_ptr());
                eqv(&format!("{p}_decompress_usingDTable[buf{i} cap{cap}]"), rc, rr);
                d.check(&format!("{p}_decompress_usingDTable[buf{i} cap{cap}] dst"));
            }
        }
    }
}

#[test]
fn legacy_fse_v05() {
    unsafe { drive_fse("FSEv05") }
}

#[test]
fn legacy_fse_v06() {
    unsafe { drive_fse("FSEv06") }
}

#[test]
fn legacy_fse_v07() {
    unsafe { drive_fse("FSEv07") }
}

// ===========================================================================
// HUFv05 / HUFv06 / HUFv07
// ===========================================================================

/// Replicates the validity test inside `HUF*_readStats` so that we can build
/// weight vectors that actually produce a usable DTable.
fn weights_valid(w: &[u8]) -> bool {
    if w.is_empty() || w.len() > 114 {
        return false;
    }
    let mut rank = [0u32; 20];
    let mut total: u32 = 0;
    for &x in w {
        if x >= 16 {
            return false;
        }
        rank[x as usize] += 1;
        total += (1u32 << x) >> 1;
    }
    if total == 0 {
        return false;
    }
    let tl = 32 - total.leading_zeros(); // highbit32(total)+1
    if tl > 16 {
        return false;
    }
    let t = 1u32 << tl;
    if total >= t {
        return false;
    }
    let rest = t - total;
    if rest.count_ones() != 1 {
        return false;
    }
    let last = 32 - rest.leading_zeros();
    if last >= 16 {
        return false;
    }
    rank[last as usize] += 1;
    if rank[1] < 2 || (rank[1] & 1) == 1 {
        return false;
    }
    true
}

/// Encode a weight list using the "incompressible" (direct nibble) header
/// form accepted by every `HUF*_readStats` implementation.
fn huf_weight_header(w: &[u8]) -> Vec<u8> {
    let mut h = vec![(127 + w.len()) as u8];
    let mut i = 0;
    while i < w.len() {
        let hi = w[i];
        let lo = if i + 1 < w.len() { w[i + 1] } else { 0 };
        h.push((hi << 4) | lo);
        i += 2;
    }
    h
}

fn valid_huf_headers(seed: u64, want: usize) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    // deterministic seeds first
    for base in [
        vec![1u8, 1],
        vec![1, 1, 2],
        vec![1, 1, 1, 1],
        vec![1, 1, 2, 3],
        vec![2, 2, 1, 1, 1, 1],
        vec![1, 1, 1, 1, 1, 1, 2, 2],
    ] {
        if weights_valid(&base) {
            out.push(huf_weight_header(&base));
        }
    }
    let mut tries = 0;
    while out.len() < want && tries < 200_000 {
        tries += 1;
        let n = 2 + rng.below(40);
        let maxw = 1 + rng.below(8) as u8;
        let w: Vec<u8> = (0..n).map(|_| (rng.below(maxw as usize + 1)) as u8).collect();
        if weights_valid(&w) {
            out.push(huf_weight_header(&w));
        }
    }
    out
}

unsafe fn drive_huf_v05_v06(p: &str) {
    let (r2c, r2r) = duo::<FnHufReadU16>(&format!("{p}_readDTableX2"));
    let (r4c, r4r) = duo::<FnHufReadU32>(&format!("{p}_readDTableX4"));
    let (u12c, u12r) = duo::<FnHufDecU16>(&format!("{p}_decompress1X2_usingDTable"));
    let (u42c, u42r) = duo::<FnHufDecU16>(&format!("{p}_decompress4X2_usingDTable"));
    let (u14c, u14r) = duo::<FnHufDecU32>(&format!("{p}_decompress1X4_usingDTable"));
    let (u44c, u44r) = duo::<FnHufDecU32>(&format!("{p}_decompress4X4_usingDTable"));

    let plain: Vec<(&str, (FnDec, FnDec))> = vec![
        ("decompress", duo::<FnDec>(&format!("{p}_decompress"))),
        ("decompress1X2", duo::<FnDec>(&format!("{p}_decompress1X2"))),
        ("decompress1X4", duo::<FnDec>(&format!("{p}_decompress1X4"))),
        ("decompress4X2", duo::<FnDec>(&format!("{p}_decompress4X2"))),
        ("decompress4X4", duo::<FnDec>(&format!("{p}_decompress4X4"))),
    ];

    // over-allocated DTables so that a `tableLog == maxTableLog+1` table
    // (which the C accepts) cannot run off the end of the buffer.
    const NU16: usize = 1 + (1 << 17);
    const NU32: usize = 1 + (1 << 17);
    let mut rng = Rng::new(0x8F0 + p.len() as u64);

    // ---- readDTableX2 / X4 over random and crafted headers ---------------
    let mut headers = valid_huf_headers(0xC0FFEE + p.len() as u64, 96);
    let nvalid = headers.len();
    for i in 0..700 {
        let n = if i < 30 { i } else { 1 + rng.below(400) };
        headers.push(rng.bytes(n));
    }
    for i in 0..24 {
        // RLE-header form (iSize >= 242) and FSE-header form
        headers.push(vec![242 + (i % 14) as u8]);
        let mut v = vec![(1 + rng.below(120)) as u8];
        let n = v[0] as usize;
        let extra = rng.bytes(n);
        v.extend_from_slice(&extra);
        headers.push(v);
    }

    let mut good_x2: Vec<Vec<u16>> = Vec::new();
    let mut good_x4: Vec<Vec<u32>> = Vec::new();

    for (i, h) in headers.iter().enumerate() {
        let s = Src::new(h);
        for &ml in &[0u32, 1, 8, 11, 12, 16] {
            let mut tc = vec![0u16; NU16];
            let mut tr = vec![0u16; NU16];
            tc[0] = ml as u16;
            tr[0] = ml as u16;
            let rc = r2c(tc.as_mut_ptr(), s.ptr(), s.len);
            let rr = r2r(tr.as_mut_ptr(), s.ptr(), s.len);
            eqv(&format!("{p}_readDTableX2[h{i} ml{ml}]"), rc, rr);
            eqv(&format!("{p}_readDTableX2[h{i} ml{ml}] table"), tc.clone(), tr);
            if !is_err(rc) && ml == 12 && i < nvalid && good_x2.len() < 8 {
                good_x2.push(tc);
            }

            let mut qc = vec![0u32; NU32];
            let mut qr = vec![0u32; NU32];
            qc[0] = ml;
            qr[0] = ml;
            let rc4 = r4c(qc.as_mut_ptr(), s.ptr(), s.len);
            let rr4 = r4r(qr.as_mut_ptr(), s.ptr(), s.len);
            eqv(&format!("{p}_readDTableX4[h{i} ml{ml}]"), rc4, rr4);
            eqv(&format!("{p}_readDTableX4[h{i} ml{ml}] table"), qc.clone(), qr);
            if !is_err(rc4) && ml == 12 && i < nvalid && good_x4.len() < 8 {
                good_x4.push(qc);
            }
        }
    }

    // ---- whole-block decoders over random bitstreams ---------------------
    for i in 0..1200 {
        let n = if i < 30 { i } else { 1 + rng.below(3000) };
        let buf = if i % 2 == 0 {
            rng.bytes(n)
        } else {
            gen_class(i % N_CLASSES, n, 0x8F01 + i as u64)
        };
        let s = Src::new(&buf);
        for &cap in &[0usize, 1, 40, 4096] {
            for (nm, (fc, fr)) in plain.iter() {
                let mut d = Dst::new(cap);
                let rc = fc(d.cp(), cap, s.ptr(), s.len);
                let rr = fr(d.rp(), cap, s.ptr(), s.len);
                eqv(&format!("{p}_{nm}[buf{i} cap{cap}]"), rc, rr);
                d.check(&format!("{p}_{nm}[buf{i} cap{cap}] dst"));
            }
        }
        // usingDTable variants with the tables we managed to build
        if !good_x2.is_empty() {
            let dt = &good_x2[i % good_x2.len()];
            for &cap in &[0usize, 7, 512] {
                let mut d = Dst::new(cap);
                eqv(
                    &format!("{p}_decompress1X2_usingDTable[buf{i} cap{cap}]"),
                    u12c(d.cp(), cap, s.ptr(), s.len, dt.as_ptr()),
                    u12r(d.rp(), cap, s.ptr(), s.len, dt.as_ptr()),
                );
                d.check(&format!("{p}_decompress1X2_usingDTable[buf{i} cap{cap}] dst"));
                let mut d = Dst::new(cap);
                eqv(
                    &format!("{p}_decompress4X2_usingDTable[buf{i} cap{cap}]"),
                    u42c(d.cp(), cap, s.ptr(), s.len, dt.as_ptr()),
                    u42r(d.rp(), cap, s.ptr(), s.len, dt.as_ptr()),
                );
                d.check(&format!("{p}_decompress4X2_usingDTable[buf{i} cap{cap}] dst"));
            }
        }
        if !good_x4.is_empty() {
            let dt = &good_x4[i % good_x4.len()];
            for &cap in &[0usize, 7, 512] {
                let mut d = Dst::new(cap);
                eqv(
                    &format!("{p}_decompress1X4_usingDTable[buf{i} cap{cap}]"),
                    u14c(d.cp(), cap, s.ptr(), s.len, dt.as_ptr()),
                    u14r(d.rp(), cap, s.ptr(), s.len, dt.as_ptr()),
                );
                d.check(&format!("{p}_decompress1X4_usingDTable[buf{i} cap{cap}] dst"));
                let mut d = Dst::new(cap);
                eqv(
                    &format!("{p}_decompress4X4_usingDTable[buf{i} cap{cap}]"),
                    u44c(d.cp(), cap, s.ptr(), s.len, dt.as_ptr()),
                    u44r(d.rp(), cap, s.ptr(), s.len, dt.as_ptr()),
                );
                d.check(&format!("{p}_decompress4X4_usingDTable[buf{i} cap{cap}] dst"));
            }
        }
    }
}

#[test]
fn legacy_huf_v05() {
    unsafe { drive_huf_v05_v06("HUFv05") }
}

#[test]
fn legacy_huf_v06() {
    unsafe { drive_huf_v05_v06("HUFv06") }
}

#[test]
fn legacy_huf_v07() {
    unsafe {
        let p = "HUFv07";
        let (rsc, rsr) = duo::<FnHufReadStats>("HUFv07_readStats");
        let (r2c, r2r) = duo::<FnHufReadU32>("HUFv07_readDTableX2");
        let (r4c, r4r) = duo::<FnHufReadU32>("HUFv07_readDTableX4");

        let plain: Vec<(&str, (FnDec, FnDec))> = vec![
            ("decompress", duo::<FnDec>("HUFv07_decompress")),
            ("decompress1X2", duo::<FnDec>("HUFv07_decompress1X2")),
            ("decompress1X4", duo::<FnDec>("HUFv07_decompress1X4")),
            ("decompress4X2", duo::<FnDec>("HUFv07_decompress4X2")),
            ("decompress4X4", duo::<FnDec>("HUFv07_decompress4X4")),
        ];
        let dctxs: Vec<(&str, (FnHufDCtx, FnHufDCtx))> = vec![
            ("decompress1X_DCtx", duo::<FnHufDCtx>("HUFv07_decompress1X_DCtx")),
            ("decompress1X2_DCtx", duo::<FnHufDCtx>("HUFv07_decompress1X2_DCtx")),
            ("decompress1X4_DCtx", duo::<FnHufDCtx>("HUFv07_decompress1X4_DCtx")),
            ("decompress4X_DCtx", duo::<FnHufDCtx>("HUFv07_decompress4X_DCtx")),
            ("decompress4X_hufOnly", duo::<FnHufDCtx>("HUFv07_decompress4X_hufOnly")),
            ("decompress4X2_DCtx", duo::<FnHufDCtx>("HUFv07_decompress4X2_DCtx")),
            ("decompress4X4_DCtx", duo::<FnHufDCtx>("HUFv07_decompress4X4_DCtx")),
        ];
        let usings: Vec<(&str, (FnHufDecU32, FnHufDecU32))> = vec![
            ("decompress1X_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress1X_usingDTable")),
            ("decompress1X2_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress1X2_usingDTable")),
            ("decompress1X4_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress1X4_usingDTable")),
            ("decompress4X_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress4X_usingDTable")),
            ("decompress4X2_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress4X2_usingDTable")),
            ("decompress4X4_usingDTable", duo::<FnHufDecU32>("HUFv07_decompress4X4_usingDTable")),
        ];

        const NU32: usize = 1 + (1 << 17);
        let mut rng = Rng::new(0x8F07);

        let mut headers = valid_huf_headers(0xC0FFEF, 96);
        let nvalid = headers.len();
        for i in 0..700 {
            let n = if i < 30 { i } else { 1 + rng.below(400) };
            headers.push(rng.bytes(n));
        }
        for i in 0..24 {
            headers.push(vec![242 + (i % 14) as u8]);
            let mut v = vec![(1 + rng.below(120)) as u8];
            let n = v[0] as usize;
            let extra = rng.bytes(n);
            v.extend_from_slice(&extra);
            headers.push(v);
        }

        // ---- readStats ---------------------------------------------------
        for (i, h) in headers.iter().enumerate() {
            let s = Src::new(h);
            for &hw in &[1usize, 2, 32, 256] {
                let mut wc = vec![0u8; 512];
                let mut wr = vec![0u8; 512];
                let mut rkc = vec![0u32; 64];
                let mut rkr = vec![0u32; 64];
                let mut nbc: c_uint = 0xDEAD;
                let mut nbr: c_uint = 0xDEAD;
                let mut tlc: c_uint = 0xBEEF;
                let mut tlr: c_uint = 0xBEEF;
                let rc = rsc(
                    wc.as_mut_ptr(),
                    hw,
                    rkc.as_mut_ptr(),
                    &mut nbc,
                    &mut tlc,
                    s.ptr(),
                    s.len,
                );
                let rr = rsr(
                    wr.as_mut_ptr(),
                    hw,
                    rkr.as_mut_ptr(),
                    &mut nbr,
                    &mut tlr,
                    s.ptr(),
                    s.len,
                );
                let lbl = format!("HUFv07_readStats[h{i} hw{hw}]");
                eqv(&format!("{lbl} ret"), rc, rr);
                eqv(&format!("{lbl} weights"), wc, wr);
                eqv(&format!("{lbl} rankStats"), rkc, rkr);
                eqv(&format!("{lbl} nbSymbols"), nbc, nbr);
                eqv(&format!("{lbl} tableLog"), tlc, tlr);
            }
        }

        // ---- readDTableX2 / X4 ------------------------------------------
        let mut good: Vec<Vec<u32>> = Vec::new();
        for (i, h) in headers.iter().enumerate() {
            let s = Src::new(h);
            for &ml in &[0u32, 1, 8, 11, 12, 16] {
                let desc = ml * 0x0100_0001;
                let mut tc = vec![0u32; NU32];
                let mut tr = vec![0u32; NU32];
                tc[0] = desc;
                tr[0] = desc;
                let rc = r2c(tc.as_mut_ptr(), s.ptr(), s.len);
                let rr = r2r(tr.as_mut_ptr(), s.ptr(), s.len);
                eqv(&format!("{p}_readDTableX2[h{i} ml{ml}]"), rc, rr);
                eqv(&format!("{p}_readDTableX2[h{i} ml{ml}] table"), tc.clone(), tr);
                if !is_err(rc) && ml == 12 && i < nvalid && good.len() < 12 {
                    good.push(tc);
                }

                let mut qc = vec![0u32; NU32];
                let mut qr = vec![0u32; NU32];
                qc[0] = desc;
                qr[0] = desc;
                let rc4 = r4c(qc.as_mut_ptr(), s.ptr(), s.len);
                let rr4 = r4r(qr.as_mut_ptr(), s.ptr(), s.len);
                eqv(&format!("{p}_readDTableX4[h{i} ml{ml}]"), rc4, rr4);
                eqv(&format!("{p}_readDTableX4[h{i} ml{ml}] table"), qc.clone(), qr);
                if !is_err(rc4) && ml == 12 && i < nvalid && good.len() < 24 {
                    good.push(qc);
                }
            }
        }

        // ---- block decoders --------------------------------------------
        for i in 0..900 {
            let n = if i < 30 { i } else { 1 + rng.below(3000) };
            let buf = if i % 2 == 0 {
                rng.bytes(n)
            } else {
                gen_class(i % N_CLASSES, n, 0x8F08 + i as u64)
            };
            let s = Src::new(&buf);
            for &cap in &[0usize, 1, 40, 4096] {
                for (nm, (fc, fr)) in plain.iter() {
                    let mut d = Dst::new(cap);
                    eqv(
                        &format!("{p}_{nm}[buf{i} cap{cap}]"),
                        fc(d.cp(), cap, s.ptr(), s.len),
                        fr(d.rp(), cap, s.ptr(), s.len),
                    );
                    d.check(&format!("{p}_{nm}[buf{i} cap{cap}] dst"));
                }
                for (nm, (fc, fr)) in dctxs.iter() {
                    let mut tc = vec![0u32; NU32];
                    let mut tr = vec![0u32; NU32];
                    tc[0] = 12 * 0x0100_0001;
                    tr[0] = 12 * 0x0100_0001;
                    let mut d = Dst::new(cap);
                    let rc = fc(tc.as_mut_ptr(), d.cp(), cap, s.ptr(), s.len);
                    let rr = fr(tr.as_mut_ptr(), d.rp(), cap, s.ptr(), s.len);
                    eqv(&format!("{p}_{nm}[buf{i} cap{cap}]"), rc, rr);
                    eqv(&format!("{p}_{nm}[buf{i} cap{cap}] dtable"), tc, tr);
                    d.check(&format!("{p}_{nm}[buf{i} cap{cap}] dst"));
                }
            }
            if !good.is_empty() {
                let dt = &good[i % good.len()];
                for &cap in &[0usize, 7, 512] {
                    for (nm, (fc, fr)) in usings.iter() {
                        let mut d = Dst::new(cap);
                        eqv(
                            &format!("{p}_{nm}[buf{i} cap{cap}]"),
                            fc(d.cp(), cap, s.ptr(), s.len, dt.as_ptr()),
                            fr(d.rp(), cap, s.ptr(), s.len, dt.as_ptr()),
                        );
                        d.check(&format!("{p}_{nm}[buf{i} cap{cap}] dst"));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// rows 157 / 165 — the public dispatch surface on legacy magics
// ===========================================================================

/// Buffers that begin with every legacy magic (the 7 real ones plus the 7
/// from `common::LEGACY_MAGICS`).
fn dispatch_corpus() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for v in ALL_VERS {
        for (i, b) in corpus(v, 0xD15_0000 + v.n() as u64).into_iter().enumerate() {
            if i % 2 != 0 {
                continue;
            }
            out.push((format!("{}#{i}", v.tag()), b));
        }
    }
    // the (differently-derived) magic list published by the shared harness
    let mut rng = Rng::new(0xD15C);
    for (k, &m) in LEGACY_MAGICS.iter().enumerate() {
        for &n in &[0usize, 1, 3, 4, 5, 8, 16, 64, 300, 4096] {
            let mut b = m.to_le_bytes().to_vec();
            let extra = rng.bytes(n);
            b.extend_from_slice(&extra);
            out.push((format!("harnessmagic{k}#{n}"), b));
        }
    }
    out
}

#[test]
fn dispatch_oneshot_on_legacy_magics() {
    unsafe {
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let (fc, fr) = duo::<FnFindCSize>("ZSTD_findFrameCompressedSize");
        let (gc, gr) = duo::<FnU64Src>("ZSTD_getFrameContentSize");
        let (bc, br) = duo::<FnU64Src>("ZSTD_decompressBound");
        let (ic, ir) = duo::<FnIsFrame>("ZSTD_isFrame");
        let (kc, kr) = duo::<FnDictID>("ZSTD_getDictID_fromFrame");
        let (sc, sr) = duo::<FnU64Src>("ZSTD_getDecompressedSize");
        let ctx = CtxPair::dctx();
        let (xc, xr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");

        let mut decoded = 0usize;
        for (lbl, b) in dispatch_corpus() {
            let s = Src::new(&b);
            eqv(&format!("ZSTD_isFrame[{lbl}]"), ic(s.ptr(), s.len), ir(s.ptr(), s.len));
            eqv(
                &format!("ZSTD_findFrameCompressedSize[{lbl}]"),
                fc(s.ptr(), s.len),
                fr(s.ptr(), s.len),
            );
            eqv(
                &format!("ZSTD_getFrameContentSize[{lbl}]"),
                gc(s.ptr(), s.len),
                gr(s.ptr(), s.len),
            );
            eqv(&format!("ZSTD_decompressBound[{lbl}]"), bc(s.ptr(), s.len), br(s.ptr(), s.len));
            eqv(
                &format!("ZSTD_getDictID_fromFrame[{lbl}]"),
                kc(s.ptr(), s.len),
                kr(s.ptr(), s.len),
            );
            eqv(
                &format!("ZSTD_getDecompressedSize[{lbl}]"),
                sc(s.ptr(), s.len),
                sr(s.ptr(), s.len),
            );
            for &cap in &[0usize, 300, 1 << 18] {
                let mut d = Dst::new(cap);
                let rc0 = dc(d.cp(), cap, s.ptr(), s.len);
                eqv(&format!("ZSTD_decompress[{lbl} cap{cap}]"), rc0, dr(d.rp(), cap, s.ptr(), s.len));
                if !is_err(rc0) && rc0 > 0 {
                    decoded += 1;
                }
                d.check(&format!("ZSTD_decompress[{lbl} cap{cap}] dst"));
                let mut d = Dst::new(cap);
                eqv(
                    &format!("ZSTD_decompressDCtx[{lbl} cap{cap}]"),
                    xc(ctx.c, d.cp(), cap, s.ptr(), s.len),
                    xr(ctx.r, d.rp(), cap, s.ptr(), s.len),
                );
                d.check(&format!("ZSTD_decompressDCtx[{lbl} cap{cap}] dst"));
            }
        }
        assert!(decoded > 0, "ZSTD_decompress never decoded a legacy frame");
    }
}

#[test]
fn dispatch_stream_on_legacy_magics() {
    unsafe {
        let (initc, initr) = duo::<FnSz1>("ZSTD_initDStream");
        let (dsc, dsr) = duo::<FnDStream>("ZSTD_decompressStream");
        let (cc, cr) = duo::<FnPtr0>("ZSTD_createDStream");
        let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeDStream");

        let chunks: [(usize, usize); 5] =
            [(1, 1), (3, 11), (1 << 17, 1 << 17), (7, 1 << 17), (1 << 17, 5)];

        let mut produced = 0usize;
        let mut completed = 0usize;
        for (lbl, b) in dispatch_corpus().into_iter().step_by(2) {
            let s = Src::new(&b);
            for (ci, &(inc, outc)) in chunks.iter().enumerate() {
                let mut trace_c: Vec<(usize, usize, usize)> = Vec::new();
                let mut trace_r: Vec<(usize, usize, usize)> = Vec::new();
                let mut out_c: Vec<u8> = Vec::new();
                let mut out_r: Vec<u8> = Vec::new();

                for lib in 0..2 {
                    let ds = if lib == 0 { cc() } else { cr() };
                    assert!(!ds.is_null());
                    let ini = if lib == 0 { initc } else { initr };
                    let ir0 = ini(ds);
                    assert!(!is_err(ir0), "ZSTD_initDStream failed");
                    let mut obuf = vec![0xC7u8; outc + SLACK];
                    let mut ipos = 0usize;
                    let mut steps = 0;
                    loop {
                        steps += 1;
                        if steps > 400 {
                            break;
                        }
                        let avail = (s.len - ipos).min(inc);
                        let mut inb = ZSTD_inBuffer { src: s.at(ipos), size: avail, pos: 0 };
                        let mut outb = ZSTD_outBuffer {
                            dst: obuf.as_mut_ptr() as *mut c_void,
                            size: outc,
                            pos: 0,
                        };
                        let r = if lib == 0 {
                            dsc(ds, &mut outb, &mut inb)
                        } else {
                            dsr(ds, &mut outb, &mut inb)
                        };
                        if lib == 0 {
                            trace_c.push((r, inb.pos, outb.pos));
                            out_c.extend_from_slice(&obuf[..outb.pos]);
                        } else {
                            trace_r.push((r, inb.pos, outb.pos));
                            out_r.extend_from_slice(&obuf[..outb.pos]);
                        }
                        if is_err(r) {
                            break;
                        }
                        ipos += inb.pos;
                        if r == 0 {
                            break;
                        }
                        if inb.pos == 0 && outb.pos == 0 {
                            break;
                        }
                    }
                    if lib == 0 {
                        fc(ds);
                    } else {
                        fr(ds);
                    }
                }
                if !out_c.is_empty() {
                    produced += 1;
                }
                if trace_c.last().map(|&(r, _, _)| r == 0).unwrap_or(false) {
                    completed += 1;
                }
                eqv(&format!("ZSTD_decompressStream[{lbl} chunk{ci}] trace"), trace_c, trace_r);
                eqbuf(&format!("ZSTD_decompressStream[{lbl} chunk{ci}] out"), &out_c, &out_r);
            }
        }
        assert!(produced > 0, "ZSTD_decompressStream never regenerated legacy data");
        assert!(completed > 0, "ZSTD_decompressStream never completed a legacy frame");
    }
}

/// `ZSTD_LEGACY_SUPPORT=5`: v0.1 … v0.4 frames must be *rejected* — and by
/// both libraries in exactly the same way.
#[test]
fn dispatch_rejects_legacy_versions_below_five() {
    unsafe {
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let (fc, fr) = duo::<FnFindCSize>("ZSTD_findFrameCompressedSize");
        let (ic, ir) = duo::<FnIsFrame>("ZSTD_isFrame");
        let (gc, gr) = duo::<FnU64Src>("ZSTD_getFrameContentSize");

        for v in [Ver::V01, Ver::V02, Ver::V03, Ver::V04] {
            // a frame that the matching legacy decoder *would* accept
            let mut rng = Rng::new(0xBAD5 + v.n() as u64);
            for k in 0..12 {
                let b = craft_raw_rle_frame(v, &mut rng, 1 + (k % 3), true);
                let s = Src::new(&b);
                let cap = 1usize << 18;
                let mut d = Dst::new(cap);
                let rc = dc(d.cp(), cap, s.ptr(), s.len);
                let rr = dr(d.rp(), cap, s.ptr(), s.len);
                eqv(&format!("ZSTD_decompress[{} reject #{k}]", v.tag()), rc, rr);
                d.check(&format!("ZSTD_decompress[{} reject #{k}] dst", v.tag()));
                assert!(
                    is_err(rc),
                    "{} frame must be rejected with ZSTD_LEGACY_SUPPORT=5, got {rc}",
                    v.tag()
                );
                eqv(&format!("ZSTD_isFrame[{} #{k}]", v.tag()), ic(s.ptr(), s.len), ir(s.ptr(), s.len));
                assert_eq!(ic(s.ptr(), s.len), 0, "{} must not be reported as a frame", v.tag());
                let cs_c = fc(s.ptr(), s.len);
                eqv(
                    &format!("ZSTD_findFrameCompressedSize[{} #{k}]", v.tag()),
                    cs_c,
                    fr(s.ptr(), s.len),
                );
                assert!(is_err(cs_c), "{} findFrameCompressedSize must fail", v.tag());
                let g = gc(s.ptr(), s.len);
                eqv(&format!("ZSTD_getFrameContentSize[{} #{k}]", v.tag()), g, gr(s.ptr(), s.len));
                assert_eq!(
                    g,
                    ZSTD_CONTENTSIZE_ERROR,
                    "{} getFrameContentSize must be CONTENTSIZE_ERROR",
                    v.tag()
                );
            }
        }

        // and v0.5 / v0.6 / v0.7 must be *accepted* by the dispatcher
        for v in [Ver::V05, Ver::V06, Ver::V07] {
            let mut rng = Rng::new(0x600D + v.n() as u64);
            let mut accepted = 0;
            for _ in 0..40 {
                let b = craft_raw_rle_frame(v, &mut rng, 2, true);
                let s = Src::new(&b);
                let cap = 1usize << 19;
                let mut d = Dst::new(cap);
                let rc = dc(d.cp(), cap, s.ptr(), s.len);
                let rr = dr(d.rp(), cap, s.ptr(), s.len);
                eqv(&format!("ZSTD_decompress[{} accept]", v.tag()), rc, rr);
                d.check(&format!("ZSTD_decompress[{} accept] dst", v.tag()));
                if !is_err(rc) {
                    accepted += 1;
                }
            }
            assert!(
                accepted > 0,
                "at least one hand-built {} RAW frame should decode through the dispatcher",
                v.tag()
            );
        }
    }
}
