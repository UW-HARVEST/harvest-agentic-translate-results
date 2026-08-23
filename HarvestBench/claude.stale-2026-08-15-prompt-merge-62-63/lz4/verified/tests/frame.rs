//! CONFIGS.md rows 100-143 — `lz4frame.c` VALID-PATH differential tests.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols only (no internal knowledge of either implementation) and compares
//! every return value AND every produced byte. Compressed frames are then
//! decoded and the result is compared against the original input.
//!
//! The axes crossed here are exactly the ones `lz4frame.c` branches on:
//! `blockSizeID` {0,4,5,6,7}, `blockMode` {linked,independent},
//! `contentChecksumFlag` {0,1}, `blockChecksumFlag` {0,1},
//! `contentSize` {0,exact}, `dictID` {0,non-zero},
//! `compressionLevel` (the fast `LZ4_stream_t` ctx below 2 / the HC ctx at >= 2),
//! `autoFlush` {0,1}, `favorDecSpeed` {0,1}, `stableSrc` {0,1},
//! `stableDst` {0,1}, `skipChecksums` {0,1}, the API shape (one-shot /
//! begin+update*+flush+end / uncompressedUpdate / mixed), the dictionary shape
//! and the decoder chunking shape.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// `lz4frame.h` / `lz4frame_static.h` structure mirrors
//
// Field-by-field from c_src/include/lz4frame.h:175-198 / :249-252 / :371-379
// and c_src/include/lz4frame_static.h (via lz4frame.h:730-735):
// every enum is a plain C `int`, `contentSize` is `unsigned long long`,
// `dictID` / the option flags are `unsigned`.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LZ4F_frameInfo_t {
    blockSizeID: c_int,         // LZ4F_blockSizeID_t
    blockMode: c_int,           // LZ4F_blockMode_t
    contentChecksumFlag: c_int, // LZ4F_contentChecksum_t
    frameType: c_int,           // LZ4F_frameType_t
    contentSize: u64,           // unsigned long long
    dictID: c_uint,             // unsigned
    blockChecksumFlag: c_int,   // LZ4F_blockChecksum_t
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LZ4F_preferences_t {
    frameInfo: LZ4F_frameInfo_t,
    compressionLevel: c_int,
    autoFlush: c_uint,
    favorDecSpeed: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct LZ4F_compressOptions_t {
    stableSrc: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct LZ4F_decompressOptions_t {
    stableDst: c_uint,
    skipChecksums: c_uint,
    reserved1: c_uint,
    reserved0: c_uint,
}

type AllocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
struct LZ4F_CustomMem {
    customAlloc: Option<AllocFn>,
    customCalloc: Option<AllocFn>,
    customFree: Option<FreeFn>,
    opaqueState: *mut c_void,
}

/// `LZ4F_defaultCMem` (lz4frame.h:740) — all-NULL, defers to stdlib.
const DEFAULT_CMEM: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: ptr::null_mut(),
};

const _: () = assert!(core::mem::size_of::<LZ4F_frameInfo_t>() == 32);
const _: () = assert!(core::mem::size_of::<LZ4F_preferences_t>() == 56);
const _: () = assert!(core::mem::align_of::<LZ4F_preferences_t>() == 8);
const _: () = assert!(core::mem::size_of::<LZ4F_compressOptions_t>() == 16);
const _: () = assert!(core::mem::size_of::<LZ4F_decompressOptions_t>() == 16);
const _: () = assert!(core::mem::size_of::<LZ4F_CustomMem>() == 32);

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------
type FnGetVersion = unsafe extern "C" fn() -> c_uint;
type FnLevelMax = unsafe extern "C" fn() -> c_int;
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCompressFrame =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateCtxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnBeginDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnBeginCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnResetDctx = unsafe extern "C" fn(*mut c_void);
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnGetFrameInfo = unsafe extern "C" fn(
    *mut c_void,
    *mut LZ4F_frameInfo_t,
    *const c_void,
    *mut usize,
) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnDecompressDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;

/// Declares `struct Api` (one plain function pointer per entry point) plus
/// `apis()`, which returns `[C, Rust]` resolved through `sym!`.
macro_rules! declare_api {
    ( $( $f:ident : $t:ty = $n:literal , )* ) => {
        #[allow(non_snake_case)]
        struct Api { $( $f: $t, )* }
        fn apis() -> [Api; 2] {
            $( sym!($f, $n, $t); )*
            [ Api { $( $f: *$f.0, )* }, Api { $( $f: *$f.1, )* } ]
        }
    };
}

declare_api! {
    getVersion: FnGetVersion = "LZ4F_getVersion",
    levelMax: FnLevelMax = "LZ4F_compressionLevel_max",
    getBlockSize: FnGetBlockSize = "LZ4F_getBlockSize",
    compressBound: FnBound = "LZ4F_compressBound",
    compressFrameBound: FnBound = "LZ4F_compressFrameBound",
    compressFrame: FnCompressFrame = "LZ4F_compressFrame",
    compressFrameUsingCDict: FnCompressFrameCDict = "LZ4F_compressFrame_usingCDict",
    createCDict: FnCreateCDict = "LZ4F_createCDict",
    createCDictAdvanced: FnCreateCDictAdv = "LZ4F_createCDict_advanced",
    freeCDict: FnFreeCDict = "LZ4F_freeCDict",
    createCctx: FnCreateCtx = "LZ4F_createCompressionContext",
    createCctxAdvanced: FnCreateCtxAdv = "LZ4F_createCompressionContext_advanced",
    freeCctx: FnFreeCtx = "LZ4F_freeCompressionContext",
    compressBegin: FnBegin = "LZ4F_compressBegin",
    compressBeginUsingDict: FnBeginDict = "LZ4F_compressBegin_usingDict",
    compressBeginUsingDictOnce: FnBeginDict = "LZ4F_compressBegin_usingDictOnce",
    compressBeginUsingCDict: FnBeginCDict = "LZ4F_compressBegin_usingCDict",
    compressUpdate: FnUpdate = "LZ4F_compressUpdate",
    uncompressedUpdate: FnUpdate = "LZ4F_uncompressedUpdate",
    flush: FnFlush = "LZ4F_flush",
    compressEnd: FnFlush = "LZ4F_compressEnd",
    createDctx: FnCreateCtx = "LZ4F_createDecompressionContext",
    createDctxAdvanced: FnCreateCtxAdv = "LZ4F_createDecompressionContext_advanced",
    freeDctx: FnFreeCtx = "LZ4F_freeDecompressionContext",
    resetDctx: FnResetDctx = "LZ4F_resetDecompressionContext",
    headerSize: FnHeaderSize = "LZ4F_headerSize",
    getFrameInfo: FnGetFrameInfo = "LZ4F_getFrameInfo",
    decompress: FnDecompress = "LZ4F_decompress",
    decompressUsingDict: FnDecompressDict = "LZ4F_decompress_usingDict",
}

// ---------------------------------------------------------------------------
// Data / preference helpers
// ---------------------------------------------------------------------------

/// `gen_data`, but ALWAYS backed by a real allocation with >= 64 readable
/// bytes past `len`. `Vec::<u8>::as_ptr()` on a zero-capacity vector returns
/// the dangling pointer `0x1`; several C paths dereference `src` (or compute
/// `src + srcSize - N`) even at size 0, so never hand a dangling pointer to
/// the API — that would be a defect of the caller, not of either library.
fn gen_src(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_data(shape, len, rng);
    if v.capacity() < len + 64 {
        v.reserve(len + 64);
    }
    v
}

fn prefs() -> LZ4F_preferences_t {
    LZ4F_preferences_t::default()
}

fn prefs_of(bsid: c_int, bmode: c_int, cc: c_int, bc: c_int, level: c_int) -> LZ4F_preferences_t {
    let mut p = prefs();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = bmode;
    p.frameInfo.contentChecksumFlag = cc;
    p.frameInfo.blockChecksumFlag = bc;
    p.compressionLevel = level;
    p
}

/// Mirror of `LZ4F_optimalBSID` (lz4frame.c:359-371). `LZ4F_compressFrame`
/// rewrites `blockSizeID` with this before writing the header.
fn optimal_bsid(requested: c_int, src_size: usize) -> c_int {
    let mut proposed: c_int = 4;
    let mut max_block: usize = 64 * 1024;
    while (requested as u32) > (proposed as u32) {
        if src_size <= max_block {
            return proposed;
        }
        proposed += 1;
        max_block <<= 2;
    }
    requested
}

/// Mirror of `LZ4F_getBlockSize` for the valid IDs (0 == default == 64 KB).
fn block_size_of(bsid: c_int) -> usize {
    match bsid {
        0 | 4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4096 * 1024,
        _ => unreachable!("invalid blockSizeID {bsid}"),
    }
}

/// The `blockSizeID` that `LZ4F_compressFrame` actually writes into the header
/// (`optimalBSID`, then `0 -> LZ4F_max64KB` inside `compressBegin_internal`).
fn header_bsid(requested: c_int, src_size: usize) -> c_int {
    let o = optimal_bsid(requested, src_size);
    if o == 0 {
        4
    } else {
        o
    }
}

/// `LZ4F_compressFrame` forces `blockIndependent` whenever the whole input
/// fits in one block (lz4frame.c:450-451).
fn header_bmode(requested_bsid: c_int, requested_bmode: c_int, src_size: usize) -> c_int {
    let o = optimal_bsid(requested_bsid, src_size);
    if src_size <= block_size_of(o) {
        1
    } else {
        requested_bmode
    }
}

const BSIDS: &[c_int] = &[0, 4, 5, 6, 7];
const REP_LEVELS: &[c_int] = &[1, 9, 12];
const ALL_LEVELS: &[c_int] = &[-5, -1, 0, 1, 2, 3, 6, 9, 10, 11, 12, 13, 100];

/// Lengths that straddle the 64 KB block boundary without being expensive.
const LENS: &[usize] = &[0, 1, 13, 64, 1000, 65535, 65536, 65537, 70000];
/// Cheap lengths for the expensive (optimal-parser) levels.
const SMALL: &[usize] = &[0, 1, 1000, 20000];

/// Length sweep appropriate for the cost of `level` (10-12 = optimal parser).
fn lens_for(level: c_int) -> &'static [usize] {
    let eff = if level < 1 {
        9
    } else if level > 12 {
        12
    } else {
        level
    };
    if eff >= 10 {
        SMALL
    } else {
        LENS
    }
}

fn cctx_new(a: &Api) -> *mut c_void {
    let mut p: *mut c_void = ptr::null_mut();
    let r = unsafe { (a.createCctx)(&mut p, LZ4F_VERSION) };
    assert_eq!(r, 0, "LZ4F_createCompressionContext failed: {r:#x}");
    assert!(!p.is_null());
    p
}

fn dctx_new(a: &Api) -> *mut c_void {
    let mut p: *mut c_void = ptr::null_mut();
    let r = unsafe { (a.createDctx)(&mut p, LZ4F_VERSION) };
    assert_eq!(r, 0, "LZ4F_createDecompressionContext failed: {r:#x}");
    assert!(!p.is_null());
    p
}

// ---------------------------------------------------------------------------
// One-shot compression
// ---------------------------------------------------------------------------

/// `LZ4F_compressFrame` through both libraries; asserts the bound, the return
/// value and every produced byte match. Returns the (identical) frame.
fn frame_both(
    ap: &[Api; 2],
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    ctx: &str,
) -> Vec<u8> {
    let p = prefs.map_or(ptr::null(), |x| x as *const LZ4F_preferences_t);
    let cb = unsafe { (ap[0].compressFrameBound)(src.len(), p) };
    let rb = unsafe { (ap[1].compressFrameBound)(src.len(), p) };
    assert_ret_eq(cb, rb, &format!("{ctx}: compressFrameBound"));
    let mut cd = vec![0xA5u8; cb + 32];
    let mut rd = vec![0xA5u8; cb + 32];
    let (cn, rn) = unsafe {
        (
            (ap[0].compressFrame)(
                cd.as_mut_ptr() as *mut c_void,
                cb,
                src.as_ptr() as *const c_void,
                src.len(),
                p,
            ),
            (ap[1].compressFrame)(
                rd.as_mut_ptr() as *mut c_void,
                cb,
                src.as_ptr() as *const c_void,
                src.len(),
                p,
            ),
        )
    };
    assert_sz_eq(cn, &cd, rn, &rd, &format!("{ctx}: compressFrame"));
    assert!(
        !is_lz4f_error(cn),
        "{ctx}: compressFrame returned error {cn:#x}"
    );
    cd.truncate(cn);
    cd
}

/// `frame_both` + a one-shot decode that must reproduce `src` exactly.
fn roundtrip(ap: &[Api; 2], src: &[u8], prefs: Option<&LZ4F_preferences_t>, ctx: &str) -> Vec<u8> {
    let frame = frame_both(ap, src, prefs, ctx);
    decode_both(ap, &frame, src, Chunk::All, Chunk::All, None, None, 1, ctx);
    frame
}

// ---------------------------------------------------------------------------
// Streaming compression
// ---------------------------------------------------------------------------

/// How `compressBegin` is performed.
#[derive(Clone, Copy)]
enum Begin<'a> {
    Plain,
    Dict(&'a [u8]),
    DictOnce(&'a [u8]),
    /// `LZ4F_compressBegin_usingCDict` with the per-library CDict pointer.
    CDict,
}

struct StreamRun {
    frame: Vec<u8>,
    rets: Vec<usize>,
}

/// One `compressBegin` .. `compressUpdate`* .. `compressEnd` frame.
///
/// `plan` gives, per call: `(srcLen, uncompressedBlock, flushAfter)`.
/// `sum(srcLen) == src.len()` must hold.
fn run_stream(
    a: &Api,
    cctx: *mut c_void,
    src: &[u8],
    prefs: *const LZ4F_preferences_t,
    plan: &[(usize, bool, bool)],
    copts: *const LZ4F_compressOptions_t,
    begin: Begin,
    cdict: *const c_void,
) -> StreamRun {
    let mut frame: Vec<u8> = Vec::new();
    let mut rets: Vec<usize> = Vec::new();
    let mut hdr = vec![0xA5u8; 64];
    let hn = unsafe {
        let hp = hdr.as_mut_ptr() as *mut c_void;
        match begin {
            Begin::Plain => (a.compressBegin)(cctx, hp, hdr.len(), prefs),
            Begin::Dict(d) => (a.compressBeginUsingDict)(
                cctx,
                hp,
                hdr.len(),
                d.as_ptr() as *const c_void,
                d.len(),
                prefs,
            ),
            Begin::DictOnce(d) => (a.compressBeginUsingDictOnce)(
                cctx,
                hp,
                hdr.len(),
                d.as_ptr() as *const c_void,
                d.len(),
                prefs,
            ),
            Begin::CDict => (a.compressBeginUsingCDict)(cctx, hp, hdr.len(), cdict, prefs),
        }
    };
    rets.push(hn);
    assert!(!is_lz4f_error(hn), "compressBegin failed: {hn:#x}");
    frame.extend_from_slice(&hdr[..hn]);

    let maxchunk = plan.iter().map(|p| p.0).max().unwrap_or(0);
    let b1 = unsafe { (a.compressBound)(maxchunk, prefs) };
    let b0 = unsafe { (a.compressBound)(0, prefs) };
    let mut scratch = vec![0xA5u8; b1.max(b0) + 64];
    let mut off = 0usize;
    for &(n, unc, do_flush) in plan {
        let f = if unc { a.uncompressedUpdate } else { a.compressUpdate };
        let r = unsafe {
            f(
                cctx,
                scratch.as_mut_ptr() as *mut c_void,
                scratch.len(),
                src.as_ptr().add(off) as *const c_void,
                n,
                copts,
            )
        };
        rets.push(r);
        assert!(!is_lz4f_error(r), "update({n}) failed: {r:#x}");
        frame.extend_from_slice(&scratch[..r]);
        off += n;
        if do_flush {
            let r = unsafe {
                (a.flush)(
                    cctx,
                    scratch.as_mut_ptr() as *mut c_void,
                    scratch.len(),
                    copts,
                )
            };
            rets.push(r);
            assert!(!is_lz4f_error(r), "flush failed: {r:#x}");
            frame.extend_from_slice(&scratch[..r]);
        }
    }
    assert_eq!(off, src.len(), "plan does not cover src");
    let r = unsafe {
        (a.compressEnd)(
            cctx,
            scratch.as_mut_ptr() as *mut c_void,
            scratch.len(),
            copts,
        )
    };
    rets.push(r);
    assert!(!is_lz4f_error(r), "compressEnd failed: {r:#x}");
    frame.extend_from_slice(&scratch[..r]);
    StreamRun { frame, rets }
}

/// `run_stream` on both libraries with fresh contexts; compares every return
/// value and the whole frame, then round-trips it.
fn stream_both(
    ap: &[Api; 2],
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    plan: &[(usize, bool, bool)],
    copts: Option<&LZ4F_compressOptions_t>,
    dict: Option<(&[u8], bool)>,
    ctx: &str,
) -> Vec<u8> {
    let p = prefs.map_or(ptr::null(), |x| x as *const LZ4F_preferences_t);
    let o = copts.map_or(ptr::null(), |x| x as *const LZ4F_compressOptions_t);
    let begin = match dict {
        None => Begin::Plain,
        Some((d, false)) => Begin::Dict(d),
        Some((d, true)) => Begin::DictOnce(d),
    };
    let c = cctx_new(&ap[0]);
    let r = cctx_new(&ap[1]);
    let cr = run_stream(&ap[0], c, src, p, plan, o, begin, ptr::null());
    let rr = run_stream(&ap[1], r, src, p, plan, o, begin, ptr::null());
    assert_ret_eq(&cr.rets, &rr.rets, &format!("{ctx}: streaming returns"));
    assert_bytes_eq(&cr.frame, &rr.frame, &format!("{ctx}: frame bytes"));
    let cf = unsafe { (ap[0].freeCctx)(c) };
    let rf = unsafe { (ap[1].freeCctx)(r) };
    assert_ret_eq(cf, rf, &format!("{ctx}: freeCompressionContext"));
    let dslice = dict.map(|(d, _)| d);
    decode_both(
        ap,
        &cr.frame,
        src,
        Chunk::All,
        Chunk::All,
        None,
        dslice,
        7,
        ctx,
    );
    cr.frame
}

// ---------------------------------------------------------------------------
// Chunked decompression
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
enum Chunk {
    /// Everything that is available.
    All,
    /// A fixed size (clamped to what is available).
    Fixed(usize),
    /// Uniform random in `1..=n`.
    Rand(usize),
    /// Exactly the `nextSrcSizeHint` returned by the previous call.
    Hint,
}

fn chunk_of(c: Chunk, rng: &mut Rng, avail: usize, hint: usize) -> usize {
    match c {
        Chunk::All => avail,
        Chunk::Fixed(n) => n.min(avail),
        Chunk::Rand(n) => rng.range(1, n).min(avail),
        Chunk::Hint => hint.max(1).min(avail),
    }
}

struct DecRun {
    out: Vec<u8>,
    rets: Vec<usize>,
    consumed: Vec<usize>,
    produced: Vec<usize>,
}

/// Feed `frame` to `LZ4F_decompress` (or `_usingDict`) in `srcc`-sized pieces,
/// offering `dstc`-sized destination windows inside ONE contiguous output
/// buffer (so a `stableDst` pledge is always honoured).
#[allow(clippy::too_many_arguments)]
fn run_decode(
    a: &Api,
    dctx: *mut c_void,
    frame: &[u8],
    out_cap: usize,
    srcc: Chunk,
    dstc: Chunk,
    dopts: *const LZ4F_decompressOptions_t,
    dict: Option<&[u8]>,
    seed: u64,
) -> DecRun {
    let mut out = vec![0xA5u8; out_cap + 1];
    let mut rng = Rng::new(seed);
    let mut consumed = 0usize;
    let mut written = 0usize;
    let mut hint = 1usize;
    let mut rets = Vec::new();
    let mut cs = Vec::new();
    let mut ds = Vec::new();
    let guard_max = 8 * frame.len() + 8 * out_cap + 4096;
    let mut guard = 0usize;
    loop {
        guard += 1;
        assert!(guard < guard_max, "decode loop did not terminate");
        let sa = frame.len() - consumed;
        let da = out.len() - written;
        let mut ss = chunk_of(srcc, &mut rng, sa, hint);
        let mut dd = chunk_of(dstc, &mut rng, da, usize::MAX);
        let r = unsafe {
            let sp = frame.as_ptr().add(consumed) as *const c_void;
            let dp = out.as_mut_ptr().add(written) as *mut c_void;
            match dict {
                None => (a.decompress)(dctx, dp, &mut dd, sp, &mut ss, dopts),
                Some(d) => (a.decompressUsingDict)(
                    dctx,
                    dp,
                    &mut dd,
                    sp,
                    &mut ss,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    dopts,
                ),
            }
        };
        rets.push(r);
        cs.push(ss);
        ds.push(dd);
        if is_lz4f_error(r) {
            break;
        }
        consumed += ss;
        written += dd;
        hint = r;
        if r == 0 && consumed == frame.len() {
            break;
        }
        if ss == 0 && dd == 0 {
            break; // no progress possible (truncated input / full destination)
        }
    }
    out.truncate(written);
    DecRun {
        out,
        rets,
        consumed: cs,
        produced: ds,
    }
}

/// `run_decode` on both libraries, comparing every return value, every
/// consumed/produced count and all output bytes; then compares the output to
/// `orig` (pass an empty slice to skip the round-trip check).
#[allow(clippy::too_many_arguments)]
fn decode_both(
    ap: &[Api; 2],
    frame: &[u8],
    orig: &[u8],
    srcc: Chunk,
    dstc: Chunk,
    dopts: Option<LZ4F_decompressOptions_t>,
    dict: Option<&[u8]>,
    seed: u64,
    ctx: &str,
) {
    let op = dopts
        .as_ref()
        .map_or(ptr::null(), |x| x as *const LZ4F_decompressOptions_t);
    let cap = orig.len() + 64;
    let cd = dctx_new(&ap[0]);
    let rd = dctx_new(&ap[1]);
    let cr = run_decode(&ap[0], cd, frame, cap, srcc, dstc, op, dict, seed);
    let rr = run_decode(&ap[1], rd, frame, cap, srcc, dstc, op, dict, seed);
    assert_ret_eq(&cr.rets, &rr.rets, &format!("{ctx}: decompress returns"));
    assert_ret_eq(
        &cr.consumed,
        &rr.consumed,
        &format!("{ctx}: *srcSizePtr sequence"),
    );
    assert_ret_eq(
        &cr.produced,
        &rr.produced,
        &format!("{ctx}: *dstSizePtr sequence"),
    );
    assert_bytes_eq(&cr.out, &rr.out, &format!("{ctx}: decoded bytes"));
    assert_bytes_eq(&cr.out, orig, &format!("{ctx}: round trip"));
    let cf = unsafe { (ap[0].freeDctx)(cd) };
    let rf = unsafe { (ap[1].freeDctx)(rd) };
    assert_ret_eq(cf, rf, &format!("{ctx}: freeDecompressionContext"));
}

// ---------------------------------------------------------------------------
// Plan builders
// ---------------------------------------------------------------------------
fn plan_uniform(len: usize, chunk: usize) -> Vec<(usize, bool, bool)> {
    let mut v = Vec::new();
    if len == 0 {
        v.push((0, false, false));
        return v;
    }
    let mut o = 0;
    while o < len {
        let n = chunk.min(len - o);
        v.push((n, false, false));
        o += n;
    }
    v
}

fn plan_random(len: usize, maxc: usize, rng: &mut Rng) -> Vec<(usize, bool, bool)> {
    let mut v = Vec::new();
    if len == 0 {
        v.push((0, false, false));
        return v;
    }
    let mut o = 0;
    while o < len {
        let n = rng.range(1, maxc).min(len - o);
        v.push((n, false, false));
        o += n;
    }
    v
}

// ---------------------------------------------------------------------------
// A reusable corpus of frames
// ---------------------------------------------------------------------------
#[allow(dead_code)]
struct Case {
    name: String,
    orig: Vec<u8>,
    frame: Vec<u8>,
    prefs: LZ4F_preferences_t,
}

/// Frames covering `blockSizeID x blockMode x contentChecksum x blockChecksum`
/// for every requested length/level, built with `LZ4F_compressFrame` (already
/// verified byte-identical between the two libraries).
fn make_corpus(
    ap: &[Api; 2],
    lens: &[usize],
    levels: &[c_int],
    bsids: &[c_int],
    seed: u64,
) -> Vec<Case> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for &len in lens {
        let shape = ALL_SHAPES[len % ALL_SHAPES.len()];
        let src = gen_src(shape, len, &mut rng);
        for &bsid in bsids {
            for &bmode in &[0, 1] {
                for &cc in &[0, 1] {
                    for &bc in &[0, 1] {
                        for &lvl in levels {
                            let p = prefs_of(bsid, bmode, cc, bc, lvl);
                            let name = format!(
                                "corpus len={len} bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl}"
                            );
                            let frame = frame_both(ap, &src, Some(&p), &name);
                            out.push(Case {
                                name,
                                orig: src.clone(),
                                frame,
                                prefs: p,
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

// ===========================================================================
// Row 100 — LZ4F_compressFrame with prefsPtr == NULL
// ===========================================================================
#[test]
fn row100_compress_frame_null_prefs() {
    let ap = apis();
    let mut rng = Rng::new(0x100);
    for &shape in ALL_SHAPES {
        for &len in LENS {
            let src = gen_src(shape, len, &mut rng);
            let ctx = format!("row100 {shape:?} len={len}");
            roundtrip(&ap, &src, None, &ctx);
        }
    }
    // ...and the explicit all-zero preferences, which must be identical to NULL.
    let p = prefs();
    for &len in LENS {
        let src = gen_src(Shape::Texty, len, &mut rng);
        let a = frame_both(&ap, &src, None, "row100 null");
        let b = frame_both(&ap, &src, Some(&p), "row100 zeroed");
        assert_bytes_eq(&a, &b, &format!("row100 NULL == zeroed prefs len={len}"));
    }
}

// ===========================================================================
// Row 101 — blockSizeID x blockMode
// ===========================================================================
#[test]
fn row101_block_size_id_x_block_mode() {
    let ap = apis();
    let mut rng = Rng::new(0x101);
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &len in &[0usize, 1, 1000, 65536, 70000, 300_000] {
                let src = gen_src(Shape::Mixed, len, &mut rng);
                let p = prefs_of(bsid, bmode, 0, 0, 0);
                roundtrip(
                    &ap,
                    &src,
                    Some(&p),
                    &format!("row101 bsid={bsid} bmode={bmode} len={len}"),
                );
            }
        }
    }
}

// ===========================================================================
// Row 102 — contentChecksumFlag x blockChecksumFlag (full 2x2)
// ===========================================================================
#[test]
fn row102_checksum_flags() {
    let ap = apis();
    let mut rng = Rng::new(0x102);
    for &cc in &[0, 1] {
        for &bc in &[0, 1] {
            for &shape in &[Shape::Random, Shape::Runs, Shape::Texty] {
                for &len in &[0usize, 1, 1000, 65536, 70000] {
                    let src = gen_src(shape, len, &mut rng);
                    let p = prefs_of(4, 0, cc, bc, 0);
                    roundtrip(
                        &ap,
                        &src,
                        Some(&p),
                        &format!("row102 cc={cc} bc={bc} {shape:?} len={len}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 103 — contentSize {0, exact} x dictID {0, 0xDEADBEEF}
// ===========================================================================
#[test]
fn row103_content_size_x_dict_id() {
    let ap = apis();
    let mut rng = Rng::new(0x103);
    for &with_cs in &[false, true] {
        for &dict_id in &[0u32, 0xDEAD_BEEF] {
            for &len in &[0usize, 1, 1000, 70000] {
                let src = gen_src(Shape::Texty, len, &mut rng);
                let mut p = prefs_of(4, 0, 1, 1, 0);
                p.frameInfo.contentSize = if with_cs { len as u64 } else { 0 };
                p.frameInfo.dictID = dict_id;
                let ctx = format!("row103 cs={with_cs} dictID={dict_id:#x} len={len}");
                let frame = roundtrip(&ap, &src, Some(&p), &ctx);
                // The header must advertise exactly what was requested.
                let want_hsize = 7
                    + if with_cs && len > 0 { 8 } else { 0 }
                    + if dict_id != 0 { 4 } else { 0 };
                let ch = unsafe {
                    (ap[0].headerSize)(frame.as_ptr() as *const c_void, frame.len().min(19))
                };
                let rh = unsafe {
                    (ap[1].headerSize)(frame.as_ptr() as *const c_void, frame.len().min(19))
                };
                assert_ret_eq(ch, rh, &format!("{ctx}: headerSize"));
                assert_eq!(ch, want_hsize, "{ctx}: unexpected header size");
                // ...and getFrameInfo must report it identically.
                let mut cfi = LZ4F_frameInfo_t::default();
                let mut rfi = LZ4F_frameInfo_t::default();
                let cd = dctx_new(&ap[0]);
                let rd = dctx_new(&ap[1]);
                let mut cs = frame.len();
                let mut rs = frame.len();
                let cr = unsafe {
                    (ap[0].getFrameInfo)(cd, &mut cfi, frame.as_ptr() as *const c_void, &mut cs)
                };
                let rr = unsafe {
                    (ap[1].getFrameInfo)(rd, &mut rfi, frame.as_ptr() as *const c_void, &mut rs)
                };
                assert_ret_eq(cr, rr, &format!("{ctx}: getFrameInfo"));
                assert_ret_eq(cs, rs, &format!("{ctx}: getFrameInfo *srcSizePtr"));
                assert_eq!(cfi, rfi, "{ctx}: frameInfo mismatch");
                assert_eq!(cfi.dictID, dict_id, "{ctx}: dictID not round-tripped");
                assert_eq!(
                    cfi.contentSize,
                    if with_cs { len as u64 } else { 0 },
                    "{ctx}: contentSize not round-tripped"
                );
                unsafe {
                    (ap[0].freeDctx)(cd);
                    (ap[1].freeDctx)(rd);
                }
            }
        }
    }
}

// ===========================================================================
// Row 104 — the whole compressionLevel list (fast ctx < 2, HC ctx >= 2)
// ===========================================================================
#[test]
fn row104_compression_levels() {
    let ap = apis();
    let mut rng = Rng::new(0x104);
    for &lvl in ALL_LEVELS {
        for &len in lens_for(lvl) {
            for &shape in &[Shape::Random, Shape::Periodic, Shape::Mixed] {
                let src = gen_src(shape, len, &mut rng);
                let p = prefs_of(4, 0, 0, 0, lvl);
                roundtrip(
                    &ap,
                    &src,
                    Some(&p),
                    &format!("row104 lvl={lvl} {shape:?} len={len}"),
                );
            }
        }
    }
}

// ===========================================================================
// Row 105 — favorDecSpeed x level {1,9,12}
// ===========================================================================
#[test]
fn row105_favor_dec_speed() {
    let ap = apis();
    let mut rng = Rng::new(0x105);
    for &favor in &[0u32, 1] {
        for &lvl in REP_LEVELS {
            for &len in &[1000usize, 8000] {
                for &shape in &[Shape::Texty, Shape::FarMatches] {
                    let src = gen_src(shape, len, &mut rng);
                    let mut p = prefs_of(4, 0, 1, 0, lvl);
                    p.favorDecSpeed = favor;
                    roundtrip(
                        &ap,
                        &src,
                        Some(&p),
                        &format!("row105 favor={favor} lvl={lvl} {shape:?} len={len}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 106 — autoFlush x blockSizeID {4,7}
// ===========================================================================
#[test]
fn row106_auto_flush() {
    let ap = apis();
    let mut rng = Rng::new(0x106);
    for &af in &[0u32, 1] {
        for &bsid in &[4, 7] {
            for &len in &[0usize, 1, 1000, 70000, 300_000] {
                let src = gen_src(Shape::Runs, len, &mut rng);
                let mut p = prefs_of(bsid, 0, 1, 1, 0);
                p.autoFlush = af;
                roundtrip(
                    &ap,
                    &src,
                    Some(&p),
                    &format!("row106 af={af} bsid={bsid} len={len}"),
                );
            }
        }
    }
    // The same axis where it actually changes the output: streaming.
    for &af in &[0u32, 1] {
        for &bsid in &[4, 7] {
            let src = gen_src(Shape::Texty, 200_000, &mut rng);
            let mut p = prefs_of(bsid, 0, 1, 1, 0);
            p.autoFlush = af;
            let plan = plan_uniform(src.len(), 20_000);
            stream_both(
                &ap,
                &src,
                Some(&p),
                &plan,
                None,
                None,
                &format!("row106 stream af={af} bsid={bsid}"),
            );
        }
    }
}

// ===========================================================================
// Row 107 — src sizes vs blockSizeID (crossing block boundaries)
// ===========================================================================
#[test]
fn row107_src_size_vs_block_size() {
    let ap = apis();
    let mut rng = Rng::new(0x107);
    for &len in &[0usize, 1, 64, 65535, 65536, 262_144, 1_048_577] {
        let src = gen_src(Shape::Periodic, len, &mut rng);
        for &bsid in BSIDS {
            let p = prefs_of(bsid, 0, 0, 0, 0);
            roundtrip(&ap, &src, Some(&p), &format!("row107 len={len} bsid={bsid}"));
        }
    }
    // ...and past the 1 MB / 4 MB block boundaries, so blockSizeID 6 and 7 also
    // produce MULTI-block frames.
    for &(len, bsid) in &[(2_100_000usize, 6), (4_194_305usize, 7)] {
        for &bmode in &[0, 1] {
            let src = gen_src(Shape::Periodic, len, &mut rng);
            let p = prefs_of(bsid, bmode, 1, 1, 0);
            roundtrip(
                &ap,
                &src,
                Some(&p),
                &format!("row107 big len={len} bsid={bsid} bmode={bmode}"),
            );
        }
    }
}

// ===========================================================================
// Row 108 — FULL cross product of every frame preference axis
// ===========================================================================
#[test]
fn row108_full_cross_product() {
    let ap = apis();
    let mut rng = Rng::new(0x108);
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &lvl in REP_LEVELS {
                        // Two lengths: one below and one well above the 64 KB
                        // block size, so every axis combination is seen with a
                        // single-block AND a multi-block frame.
                        let lens: &[usize] = if lvl >= 10 {
                            &[3000, 20000]
                        } else {
                            &[3000, 70000, 300_000]
                        };
                        for &len in lens {
                            let src = gen_src(Shape::Mixed, len, &mut rng);
                            let p = prefs_of(bsid, bmode, cc, bc, lvl);
                            roundtrip(
                                &ap,
                                &src,
                                Some(&p),
                                &format!(
                                    "row108 bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl} len={len}"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 109 — compressBegin + ONE compressUpdate + compressEnd, row-108 axes
// ===========================================================================
#[test]
fn row109_begin_one_update_end() {
    let ap = apis();
    let mut rng = Rng::new(0x109);
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &lvl in REP_LEVELS {
                        let len = if lvl >= 10 { 3000 } else { 70000 };
                        let src = gen_src(Shape::Mixed, len, &mut rng);
                        let mut p = prefs_of(bsid, bmode, cc, bc, lvl);
                        p.frameInfo.contentSize = len as u64;
                        let plan = vec![(len, false, false)];
                        stream_both(
                            &ap,
                            &src,
                            Some(&p),
                            &plan,
                            None,
                            None,
                            &format!(
                                "row109 bsid={bsid} bmode={bmode} cc={cc} bc={bc} lvl={lvl}"
                            ),
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 110 — MANY compressUpdate calls, UNIFORM chunk sizes
// ===========================================================================
#[test]
fn row110_uniform_update_chunks() {
    let ap = apis();
    let mut rng = Rng::new(0x110);
    for &chunk in &[1usize, 2, 3, 7, 64, 1000, 65536] {
        let len = if chunk < 8 { 3000 } else { 200_000 };
        let src = gen_src(Shape::Texty, len, &mut rng);
        for &bsid in &[4, 5] {
            for &bmode in &[0, 1] {
                for &af in &[0u32, 1] {
                    let mut p = prefs_of(bsid, bmode, 1, 1, 0);
                    p.autoFlush = af;
                    let plan = plan_uniform(len, chunk);
                    stream_both(
                        &ap,
                        &src,
                        Some(&p),
                        &plan,
                        None,
                        None,
                        &format!("row110 chunk={chunk} bsid={bsid} bmode={bmode} af={af}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 111 — MANY compressUpdate calls, RANDOM chunk sizes (tmpBuff filling)
// ===========================================================================
#[test]
fn row111_random_update_chunks() {
    let ap = apis();
    let mut rng = Rng::new(0x111);
    for trial in 0..12 {
        let len = 150_000;
        let src = gen_src(ALL_SHAPES[trial % ALL_SHAPES.len()], len, &mut rng);
        let bsid = [4, 5, 6][trial % 3];
        let bmode = (trial % 2) as c_int;
        let lvl = [0, 1, 9][trial % 3];
        let mut p = prefs_of(bsid, bmode, 1, 1, lvl);
        p.autoFlush = (trial % 2) as c_uint;
        let mut prng = Rng::new(0x1110 + trial as u64);
        let plan = plan_random(len, 40_000, &mut prng);
        stream_both(
            &ap,
            &src,
            Some(&p),
            &plan,
            None,
            None,
            &format!("row111 trial={trial} bsid={bsid} bmode={bmode} lvl={lvl}"),
        );
    }
}

// ===========================================================================
// Row 112 — explicit LZ4F_flush interleaved at random points
// ===========================================================================
#[test]
fn row112_explicit_flush() {
    let ap = apis();
    let mut rng = Rng::new(0x112);
    for trial in 0..12 {
        let len = 120_000;
        let src = gen_src(ALL_SHAPES[trial % ALL_SHAPES.len()], len, &mut rng);
        let bsid = [4, 5, 7][trial % 3];
        let bmode = (trial % 2) as c_int;
        let mut p = prefs_of(bsid, bmode, 1, (trial % 2) as c_int, [0, 1, 9][trial % 3]);
        p.autoFlush = ((trial / 2) % 2) as c_uint;
        let mut prng = Rng::new(0x1120 + trial as u64);
        let mut plan = plan_random(len, 30_000, &mut prng);
        for e in plan.iter_mut() {
            e.2 = prng.below(3) == 0;
        }
        stream_both(
            &ap,
            &src,
            Some(&p),
            &plan,
            None,
            None,
            &format!("row112 trial={trial}"),
        );
    }
}

// ===========================================================================
// Row 113 — LZ4F_compressOptions_t.stableSrc {0,1}
// ===========================================================================
#[test]
fn row113_stable_src() {
    let ap = apis();
    let mut rng = Rng::new(0x113);
    for &stable in &[0u32, 1] {
        let mut o = LZ4F_compressOptions_t::default();
        o.stableSrc = stable;
        for &bmode in &[0, 1] {
            for &af in &[0u32, 1] {
                for &chunk in &[3000usize, 40_000] {
                    let len = 150_000;
                    let src = gen_src(Shape::FarMatches, len, &mut rng);
                    let mut p = prefs_of(4, bmode, 1, 0, 0);
                    p.autoFlush = af;
                    let plan = plan_uniform(len, chunk);
                    stream_both(
                        &ap,
                        &src,
                        Some(&p),
                        &plan,
                        Some(&o),
                        None,
                        &format!(
                            "row113 stableSrc={stable} bmode={bmode} af={af} chunk={chunk}"
                        ),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 114 — LZ4F_uncompressedUpdate (stored blocks)
// ===========================================================================
#[test]
fn row114_uncompressed_update() {
    let ap = apis();
    let mut rng = Rng::new(0x114);
    for &chunk in &[1usize, 7, 1000, 65536, 100_000] {
        let len = if chunk < 8 { 3000 } else { 200_000 };
        let src = gen_src(Shape::Texty, len, &mut rng);
        for &bsid in &[4, 5] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &af in &[0u32, 1] {
                        let mut p = prefs_of(bsid, 1 /* blockIndependent */, cc, bc, 0);
                        p.autoFlush = af;
                        let mut plan = plan_uniform(len, chunk);
                        for e in plan.iter_mut() {
                            e.1 = true;
                        }
                        stream_both(
                            &ap,
                            &src,
                            Some(&p),
                            &plan,
                            None,
                            None,
                            &format!(
                                "row114 chunk={chunk} bsid={bsid} cc={cc} bc={bc} af={af}"
                            ),
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 115 — uncompressedUpdate and compressUpdate MIXED in one frame
//           (forces the internal LZ4F_flush at lz4frame.c:1013)
// ===========================================================================
#[test]
fn row115_mixed_updates() {
    let ap = apis();
    let mut rng = Rng::new(0x115);
    for trial in 0..12 {
        let len = 100_000;
        let src = gen_src(ALL_SHAPES[trial % ALL_SHAPES.len()], len, &mut rng);
        let mut p = prefs_of([4, 5][trial % 2], 1, (trial % 2) as c_int, ((trial / 2) % 2) as c_int, 0);
        p.autoFlush = ((trial / 4) % 2) as c_uint;
        let mut prng = Rng::new(0x1150 + trial as u64);
        let mut plan = plan_random(len, 30_000, &mut prng);
        for (i, e) in plan.iter_mut().enumerate() {
            e.1 = (i + trial) % 2 == 0; // alternate compressed / stored
        }
        stream_both(
            &ap,
            &src,
            Some(&p),
            &plan,
            None,
            None,
            &format!("row115 trial={trial}"),
        );
    }
}

// ===========================================================================
// Rows 116/117 — compressBegin_usingDict / _usingDictOnce
// ===========================================================================
const DICT_SIZES: &[usize] = &[1, 64, 1024, 65535, 65536, 70000];

fn dict_rows(ap: &[Api; 2], once: bool, tag: &str) {
    let mut rng = Rng::new(if once { 0x117 } else { 0x116 });
    for &ds in DICT_SIZES {
        // The dictionary is handed to LZ4_loadDict, which keeps a POINTER to
        // it: it must outlive the whole compression session.
        let dict = gen_src(Shape::Texty, ds, &mut rng);
        for &bmode in &[0, 1] {
            for &lvl in &[0, 1, 9] {
                for &len in &[0usize, 1, 1000, 70000] {
                    let src = gen_src(Shape::Texty, len, &mut rng);
                    let mut p = prefs_of(4, bmode, 1, 1, lvl);
                    p.frameInfo.dictID = 0x1234_5678;
                    let plan = plan_uniform(len, 20_000);
                    stream_both(
                        ap,
                        &src,
                        Some(&p),
                        &plan,
                        None,
                        Some((&dict, once)),
                        &format!("{tag} ds={ds} bmode={bmode} lvl={lvl} len={len}"),
                    );
                }
            }
        }
    }
}

#[test]
fn row116_begin_using_dict() {
    let ap = apis();
    dict_rows(&ap, false, "row116");
}

#[test]
fn row117_begin_using_dict_once() {
    let ap = apis();
    dict_rows(&ap, true, "row117");
}

// ===========================================================================
// Row 118 — LZ4F_createCDict + compressBegin_usingCDict
// ===========================================================================
#[test]
fn row118_create_cdict_begin_using_cdict() {
    let ap = apis();
    let mut rng = Rng::new(0x118);
    for &ds in DICT_SIZES {
        let dict = gen_src(Shape::Texty, ds, &mut rng);
        let cdicts = [
            unsafe { (ap[0].createCDict)(dict.as_ptr() as *const c_void, ds) },
            unsafe { (ap[1].createCDict)(dict.as_ptr() as *const c_void, ds) },
        ];
        assert!(!cdicts[0].is_null() && !cdicts[1].is_null(), "createCDict");
        for &lvl in REP_LEVELS {
            for &bmode in &[0, 1] {
                for &len in &[0usize, 1, 1000, 70000] {
                    let len = if lvl >= 10 { len.min(3000) } else { len };
                    let src = gen_src(Shape::Texty, len, &mut rng);
                    let p = prefs_of(4, bmode, 1, 1, lvl);
                    let ctx = format!("row118 ds={ds} lvl={lvl} bmode={bmode} len={len}");
                    let c = cctx_new(&ap[0]);
                    let r = cctx_new(&ap[1]);
                    let plan = plan_uniform(len, 30_000);
                    let cr = run_stream(
                        &ap[0],
                        c,
                        &src,
                        &p,
                        &plan,
                        ptr::null(),
                        Begin::CDict,
                        cdicts[0],
                    );
                    let rr = run_stream(
                        &ap[1],
                        r,
                        &src,
                        &p,
                        &plan,
                        ptr::null(),
                        Begin::CDict,
                        cdicts[1],
                    );
                    assert_ret_eq(&cr.rets, &rr.rets, &format!("{ctx}: returns"));
                    assert_bytes_eq(&cr.frame, &rr.frame, &format!("{ctx}: frame"));
                    unsafe {
                        (ap[0].freeCctx)(c);
                        (ap[1].freeCctx)(r);
                    }
                    decode_both(
                        &ap,
                        &cr.frame,
                        &src,
                        Chunk::All,
                        Chunk::All,
                        None,
                        Some(&dict),
                        9,
                        &ctx,
                    );
                }
            }
        }
        unsafe {
            (ap[0].freeCDict)(cdicts[0]);
            (ap[1].freeCDict)(cdicts[1]);
        }
    }
}

// ===========================================================================
// Row 119 — createCDict_advanced + compressFrame_usingCDict
// ===========================================================================
#[test]
fn row119_cdict_advanced_compress_frame() {
    let ap = apis();
    let mut rng = Rng::new(0x119);
    for &ds in DICT_SIZES {
        let dict = gen_src(Shape::Texty, ds, &mut rng);
        let cdicts = [
            unsafe {
                (ap[0].createCDictAdvanced)(DEFAULT_CMEM, dict.as_ptr() as *const c_void, ds)
            },
            unsafe {
                (ap[1].createCDictAdvanced)(DEFAULT_CMEM, dict.as_ptr() as *const c_void, ds)
            },
        ];
        assert!(
            !cdicts[0].is_null() && !cdicts[1].is_null(),
            "createCDict_advanced"
        );
        for &lvl in &[0, 1, 9, 12] {
            for &len in &[0usize, 1, 1000, 70000] {
                let len = if lvl >= 10 { len.min(3000) } else { len };
                let src = gen_src(Shape::Mixed, len, &mut rng);
                let mut p = prefs_of(4, 0, 1, 1, lvl);
                p.frameInfo.dictID = 0xABCD;
                let ctx = format!("row119 ds={ds} lvl={lvl} len={len}");
                let bound = unsafe { (ap[0].compressFrameBound)(len, &p) };
                let rbound = unsafe { (ap[1].compressFrameBound)(len, &p) };
                assert_ret_eq(bound, rbound, &format!("{ctx}: frameBound"));
                let c = cctx_new(&ap[0]);
                let r = cctx_new(&ap[1]);
                let mut cd = vec![0xA5u8; bound + 32];
                let mut rd = vec![0xA5u8; bound + 32];
                let cn = unsafe {
                    (ap[0].compressFrameUsingCDict)(
                        c,
                        cd.as_mut_ptr() as *mut c_void,
                        bound,
                        src.as_ptr() as *const c_void,
                        len,
                        cdicts[0],
                        &p,
                    )
                };
                let rn = unsafe {
                    (ap[1].compressFrameUsingCDict)(
                        r,
                        rd.as_mut_ptr() as *mut c_void,
                        bound,
                        src.as_ptr() as *const c_void,
                        len,
                        cdicts[1],
                        &p,
                    )
                };
                assert_sz_eq(cn, &cd, rn, &rd, &ctx);
                assert!(!is_lz4f_error(cn), "{ctx}: failed {cn:#x}");
                cd.truncate(cn);
                unsafe {
                    (ap[0].freeCctx)(c);
                    (ap[1].freeCctx)(r);
                }
                decode_both(
                    &ap,
                    &cd,
                    &src,
                    Chunk::All,
                    Chunk::All,
                    None,
                    Some(&dict),
                    11,
                    &ctx,
                );
            }
        }
        // A NULL cdict means "no dictionary" (ERRORS.md row 148 companion).
        let src = gen_src(Shape::Texty, 5000, &mut rng);
        let p = prefs_of(4, 0, 1, 0, 0);
        let bound = unsafe { (ap[0].compressFrameBound)(src.len(), &p) };
        let c = cctx_new(&ap[0]);
        let r = cctx_new(&ap[1]);
        let mut cd = vec![0xA5u8; bound + 32];
        let mut rd = vec![0xA5u8; bound + 32];
        let cn = unsafe {
            (ap[0].compressFrameUsingCDict)(
                c,
                cd.as_mut_ptr() as *mut c_void,
                bound,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
                &p,
            )
        };
        let rn = unsafe {
            (ap[1].compressFrameUsingCDict)(
                r,
                rd.as_mut_ptr() as *mut c_void,
                bound,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
                &p,
            )
        };
        assert_sz_eq(cn, &cd, rn, &rd, "row119 NULL cdict");
        cd.truncate(cn);
        unsafe {
            (ap[0].freeCctx)(c);
            (ap[1].freeCctx)(r);
        }
        decode_both(
            &ap,
            &cd,
            &src,
            Chunk::All,
            Chunk::All,
            None,
            None,
            13,
            "row119 NULL cdict",
        );
        unsafe {
            (ap[0].freeCDict)(cdicts[0]);
            (ap[1].freeCDict)(cdicts[1]);
        }
    }
}

// ===========================================================================
// Row 120 — createCompressionContext_advanced + ONE cctx reused for N frames
// ===========================================================================
#[test]
fn row120_cctx_advanced_reused() {
    let ap = apis();
    let mut rng = Rng::new(0x120);
    let c = unsafe { (ap[0].createCctxAdvanced)(DEFAULT_CMEM, LZ4F_VERSION) };
    let r = unsafe { (ap[1].createCctxAdvanced)(DEFAULT_CMEM, LZ4F_VERSION) };
    assert!(!c.is_null() && !r.is_null(), "createCompressionContext_advanced");
    for i in 0..24usize {
        let len = [0usize, 1, 1000, 70000, 100_000, 65536][i % 6];
        let src = gen_src(ALL_SHAPES[i % ALL_SHAPES.len()], len, &mut rng);
        let mut p = prefs_of(
            BSIDS[i % BSIDS.len()],
            (i % 2) as c_int,
            ((i / 2) % 2) as c_int,
            ((i / 4) % 2) as c_int,
            [0, 1, 2, 9][i % 4],
        );
        p.autoFlush = ((i / 3) % 2) as c_uint;
        p.frameInfo.contentSize = len as u64;
        let plan = plan_uniform(len, 25_000);
        let ctx = format!("row120 frame#{i} len={len}");
        let cr = run_stream(&ap[0], c, &src, &p, &plan, ptr::null(), Begin::Plain, ptr::null());
        let rr = run_stream(&ap[1], r, &src, &p, &plan, ptr::null(), Begin::Plain, ptr::null());
        assert_ret_eq(&cr.rets, &rr.rets, &format!("{ctx}: returns"));
        assert_bytes_eq(&cr.frame, &rr.frame, &format!("{ctx}: frame"));
        decode_both(
            &ap,
            &cr.frame,
            &src,
            Chunk::All,
            Chunk::All,
            None,
            None,
            15,
            &ctx,
        );
    }
    let cf = unsafe { (ap[0].freeCctx)(c) };
    let rf = unsafe { (ap[1].freeCctx)(r) };
    assert_ret_eq(cf, rf, "row120 freeCompressionContext");
}

// ===========================================================================
// Row 121 — compressBound / compressFrameBound sweeps
// ===========================================================================
#[test]
fn row121_bounds() {
    let ap = apis();
    let sizes: &[usize] = &[
        0,
        1,
        2,
        3,
        4,
        63,
        64,
        65535,
        65536,
        65537,
        262_143,
        262_144,
        1_048_576,
        4_194_303,
        4_194_304,
        4_194_305,
        10_000_000,
        1usize << 32,
        1usize << 48,
        usize::MAX / 4,
    ];
    for &n in sizes {
        // NULL preferences (worst case inside the C).
        let cb = unsafe { (ap[0].compressBound)(n, ptr::null()) };
        let rb = unsafe { (ap[1].compressBound)(n, ptr::null()) };
        assert_ret_eq(cb, rb, &format!("row121 compressBound({n}, NULL)"));
        let cf = unsafe { (ap[0].compressFrameBound)(n, ptr::null()) };
        let rf = unsafe { (ap[1].compressFrameBound)(n, ptr::null()) };
        assert_ret_eq(cf, rf, &format!("row121 compressFrameBound({n}, NULL)"));
        for &bsid in BSIDS {
            for &af in &[0u32, 1] {
                for &bc in &[0, 1, 2, 3, -1] {
                    for &cc in &[0, 1, 2, -1] {
                        let mut p = prefs_of(bsid, 0, cc, bc, 0);
                        p.autoFlush = af;
                        let ctx =
                            format!("row121 n={n} bsid={bsid} af={af} bc={bc} cc={cc}");
                        let cb = unsafe { (ap[0].compressBound)(n, &p) };
                        let rb = unsafe { (ap[1].compressBound)(n, &p) };
                        assert_ret_eq(cb, rb, &format!("{ctx}: compressBound"));
                        let cf = unsafe { (ap[0].compressFrameBound)(n, &p) };
                        let rf = unsafe { (ap[1].compressFrameBound)(n, &p) };
                        assert_ret_eq(cf, rf, &format!("{ctx}: compressFrameBound"));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 122/123 — getBlockSize, compressionLevel_max, getVersion
// ===========================================================================
#[test]
fn row122_get_block_size() {
    let ap = apis();
    for (&id, &want) in [0, 4, 5, 6, 7]
        .iter()
        .zip([65536usize, 65536, 262_144, 1_048_576, 4_194_304].iter())
    {
        let c = unsafe { (ap[0].getBlockSize)(id) };
        let r = unsafe { (ap[1].getBlockSize)(id) };
        assert_ret_eq(c, r, &format!("row122 getBlockSize({id})"));
        assert_eq!(c, want, "row122 getBlockSize({id}) unexpected");
    }
}

#[test]
fn row123_constants() {
    let ap = apis();
    let c = unsafe { (ap[0].levelMax)() };
    let r = unsafe { (ap[1].levelMax)() };
    assert_ret_eq(c, r, "row123 compressionLevel_max");
    assert_eq!(c, 12, "row123 compressionLevel_max unexpected");
    let c = unsafe { (ap[0].getVersion)() };
    let r = unsafe { (ap[1].getVersion)() };
    assert_ret_eq(c, r, "row123 getVersion");
    assert_eq!(c, LZ4F_VERSION, "row123 getVersion unexpected");
}

// ===========================================================================
// Row 124 — one-shot LZ4F_decompress of every produced frame
// ===========================================================================
#[test]
fn row124_one_shot_decompress() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[0, 1, 1000, 70000], &[0, 9], &[4, 7], 0x124);
    for c in &corpus {
        decode_both(
            &ap,
            &c.frame,
            &c.orig,
            Chunk::All,
            Chunk::All,
            None,
            None,
            17,
            &format!("row124 {}", c.name),
        );
    }
}

// ===========================================================================
// Row 125 — headerSize + getFrameInfo on every row-108 variant
// ===========================================================================
#[test]
fn row125_header_size_and_frame_info() {
    let ap = apis();
    let mut rng = Rng::new(0x125);
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &with_cs in &[false, true] {
                        for &with_id in &[false, true] {
                            let len = 5000usize;
                            let src = gen_src(Shape::Texty, len, &mut rng);
                            let mut p = prefs_of(bsid, bmode, cc, bc, 0);
                            if with_cs {
                                p.frameInfo.contentSize = len as u64;
                            }
                            if with_id {
                                p.frameInfo.dictID = 0xDEAD_BEEF;
                            }
                            let ctx = format!(
                                "row125 bsid={bsid} bmode={bmode} cc={cc} bc={bc} cs={with_cs} id={with_id}"
                            );
                            let frame = frame_both(&ap, &src, Some(&p), &ctx);
                            let want = 7
                                + if with_cs { 8 } else { 0 }
                                + if with_id { 4 } else { 0 };
                            // headerSize with every srcSize from 5 to 19.
                            for n in 5..=19usize {
                                let c = unsafe {
                                    (ap[0].headerSize)(frame.as_ptr() as *const c_void, n)
                                };
                                let r = unsafe {
                                    (ap[1].headerSize)(frame.as_ptr() as *const c_void, n)
                                };
                                assert_ret_eq(c, r, &format!("{ctx}: headerSize({n})"));
                                assert_eq!(c, want, "{ctx}: headerSize({n}) unexpected");
                            }
                            // getFrameInfo on a fresh dctx.
                            let mut cfi = LZ4F_frameInfo_t::default();
                            let mut rfi = LZ4F_frameInfo_t::default();
                            let cd = dctx_new(&ap[0]);
                            let rd = dctx_new(&ap[1]);
                            let mut cs = frame.len();
                            let mut rs = frame.len();
                            let cr = unsafe {
                                (ap[0].getFrameInfo)(
                                    cd,
                                    &mut cfi,
                                    frame.as_ptr() as *const c_void,
                                    &mut cs,
                                )
                            };
                            let rr = unsafe {
                                (ap[1].getFrameInfo)(
                                    rd,
                                    &mut rfi,
                                    frame.as_ptr() as *const c_void,
                                    &mut rs,
                                )
                            };
                            assert_ret_eq(cr, rr, &format!("{ctx}: getFrameInfo"));
                            assert_ret_eq(cs, rs, &format!("{ctx}: consumed"));
                            assert_eq!(cfi, rfi, "{ctx}: frameInfo");
                            assert_eq!(cs, want, "{ctx}: getFrameInfo consumed != header");
                            assert_eq!(
                                cfi.blockMode,
                                header_bmode(bsid, bmode, len),
                                "{ctx}: blockMode"
                            );
                            assert_eq!(cfi.contentChecksumFlag, cc, "{ctx}: cc flag");
                            assert_eq!(cfi.blockChecksumFlag, bc, "{ctx}: bc flag");
                            assert_eq!(
                                cfi.blockSizeID,
                                header_bsid(bsid, len),
                                "{ctx}: blockSizeID"
                            );
                            let cfree = unsafe { (ap[0].freeDctx)(cd) };
                            let rfree = unsafe { (ap[1].freeDctx)(rd) };
                            assert_ret_eq(cfree, rfree, &format!("{ctx}: freeDctx mid-frame"));
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 126 — getFrameInfo BEFORE any decompress, decoding continues from there
// ===========================================================================
#[test]
fn row126_frame_info_then_decode() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[0, 1000, 70000], &[0], &[4, 7], 0x126);
    for c in &corpus {
        let ctx = format!("row126 {}", c.name);
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut infos = [LZ4F_frameInfo_t::default(); 2];
        let mut hints = [0usize; 2];
        let mut consumed = [0usize; 2];
        for k in 0..2 {
            let d = dctx_new(&ap[k]);
            let mut fi = LZ4F_frameInfo_t::default();
            let mut sz = c.frame.len();
            let hint = unsafe {
                (ap[k].getFrameInfo)(d, &mut fi, c.frame.as_ptr() as *const c_void, &mut sz)
            };
            assert!(!is_lz4f_error(hint), "{ctx}: getFrameInfo failed {hint:#x}");
            infos[k] = fi;
            hints[k] = hint;
            consumed[k] = sz;
            // Decoding resumes from (srcBuffer + *srcSizePtr).
            let rest = &c.frame[sz..];
            let mut out = vec![0xA5u8; c.orig.len() + 64];
            let mut ds = out.len();
            let mut ss = rest.len();
            let r = unsafe {
                (ap[k].decompress)(
                    d,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    rest.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                )
            };
            assert_eq!(r, 0, "{ctx}: frame not fully decoded (ret {r:#x})");
            assert_eq!(ss, rest.len(), "{ctx}: not all input consumed");
            out.truncate(ds);
            outs[k] = out;
            let f = unsafe { (ap[k].freeDctx)(d) };
            assert_eq!(f, 0, "{ctx}: dctx not clean after decode");
        }
        assert_eq!(infos[0], infos[1], "{ctx}: frameInfo");
        assert_ret_eq(hints[0], hints[1], &format!("{ctx}: hint"));
        assert_ret_eq(consumed[0], consumed[1], &format!("{ctx}: consumed"));
        assert_bytes_eq(&outs[0], &outs[1], &format!("{ctx}: decoded"));
        assert_bytes_eq(&outs[0], &c.orig, &format!("{ctx}: round trip"));
    }
}

// ===========================================================================
// Row 127 — getFrameInfo mid-frame (header already consumed by decompress)
// ===========================================================================
#[test]
fn row127_frame_info_mid_frame() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[1000, 70000], &[0], &[4], 0x127);
    for c in &corpus {
        let ctx = format!("row127 {}", c.name);
        let mut res = [(0usize, 0usize, LZ4F_frameInfo_t::default()); 2];
        for k in 0..2 {
            let d = dctx_new(&ap[k]);
            // Feed exactly the frame header first, then ask again mid-frame.
            let h = unsafe { (ap[k].headerSize)(c.frame.as_ptr() as *const c_void, 19) };
            let mut out = vec![0xA5u8; c.orig.len() + 64];
            let mut ds = out.len();
            let mut ss = h + 8;
            let _ = unsafe {
                (ap[k].decompress)(
                    d,
                    out.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    c.frame.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                )
            };
            let mut fi = LZ4F_frameInfo_t::default();
            let mut sz = c.frame.len();
            let hint = unsafe {
                (ap[k].getFrameInfo)(d, &mut fi, c.frame.as_ptr() as *const c_void, &mut sz)
            };
            res[k] = (hint, sz, fi);
            unsafe {
                (ap[k].freeDctx)(d);
            }
        }
        assert_ret_eq(res[0].0, res[1].0, &format!("{ctx}: mid-frame hint"));
        assert_ret_eq(res[0].1, res[1].1, &format!("{ctx}: mid-frame consumed"));
        assert_eq!(res[0].2, res[1].2, "{ctx}: mid-frame frameInfo");
        assert_eq!(res[0].1, 0, "{ctx}: mid-frame must consume nothing");
    }
}

// ===========================================================================
// Row 128 — src fed in FIXED chunks
// ===========================================================================
#[test]
fn row128_fixed_src_chunks() {
    let ap = apis();
    let tiny = make_corpus(&ap, &[0, 1, 300, 1500], &[0], &[4], 0x1280);
    let big = make_corpus(&ap, &[0, 3000, 70000], &[0], &[4, 7], 0x1281);
    for &chunk in &[1usize, 2, 3, 4, 5, 7, 11, 15, 19, 20, 33, 64, 1000] {
        let corpus = if chunk <= 7 { &tiny } else { &big };
        for c in corpus {
            decode_both(
                &ap,
                &c.frame,
                &c.orig,
                Chunk::Fixed(chunk),
                Chunk::All,
                None,
                None,
                19,
                &format!("row128 chunk={chunk} {}", c.name),
            );
        }
    }
}

// ===========================================================================
// Row 129 — src fed in RANDOM chunk sizes
// ===========================================================================
#[test]
fn row129_random_src_chunks() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[0, 1, 3000, 70000], &[0], &[4, 7], 0x129);
    for (i, c) in corpus.iter().enumerate() {
        for &maxc in &[3usize, 40, 5000] {
            decode_both(
                &ap,
                &c.frame,
                &c.orig,
                Chunk::Rand(maxc),
                Chunk::All,
                None,
                None,
                0x1290 + i as u64,
                &format!("row129 maxc={maxc} {}", c.name),
            );
        }
    }
}

// ===========================================================================
// Row 130 — dst offered in FIXED small chunks (tmpOut buffering)
// ===========================================================================
#[test]
fn row130_fixed_dst_chunks() {
    let ap = apis();
    let tiny = make_corpus(&ap, &[1, 300, 1500], &[0], &[4], 0x1300);
    let big = make_corpus(&ap, &[3000, 70000], &[0], &[4, 7], 0x1301);
    for &chunk in &[1usize, 2, 3, 7, 64, 1000] {
        let corpus = if chunk <= 7 { &tiny } else { &big };
        for c in corpus {
            decode_both(
                &ap,
                &c.frame,
                &c.orig,
                Chunk::All,
                Chunk::Fixed(chunk),
                None,
                None,
                21,
                &format!("row130 dst={chunk} {}", c.name),
            );
        }
    }
}

// ===========================================================================
// Row 131 — dst in RANDOM chunk sizes
// ===========================================================================
#[test]
fn row131_random_dst_chunks() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[1, 3000, 70000], &[0], &[4, 7], 0x131);
    for (i, c) in corpus.iter().enumerate() {
        for &maxc in &[5usize, 100, 9000] {
            decode_both(
                &ap,
                &c.frame,
                &c.orig,
                Chunk::All,
                Chunk::Rand(maxc),
                None,
                None,
                0x1310 + i as u64,
                &format!("row131 maxdst={maxc} {}", c.name),
            );
        }
    }
}

// ===========================================================================
// Row 132 — BOTH src and dst chunked; src follows nextSrcSizeHint
// ===========================================================================
#[test]
fn row132_both_chunked() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[1, 3000, 70000], &[0, 9], &[4, 7], 0x132);
    for (i, c) in corpus.iter().enumerate() {
        decode_both(
            &ap,
            &c.frame,
            &c.orig,
            Chunk::Rand(3000),
            Chunk::Rand(3000),
            None,
            None,
            0x1320 + i as u64,
            &format!("row132 rand/rand {}", c.name),
        );
        decode_both(
            &ap,
            &c.frame,
            &c.orig,
            Chunk::Hint,
            Chunk::Rand(4000),
            None,
            None,
            0x1330 + i as u64,
            &format!("row132 hint/rand {}", c.name),
        );
        decode_both(
            &ap,
            &c.frame,
            &c.orig,
            Chunk::Hint,
            Chunk::All,
            None,
            None,
            0x1340 + i as u64,
            &format!("row132 hint/all {}", c.name),
        );
    }
}

// ===========================================================================
// Row 133 — LZ4F_decompressOptions_t.stableDst {0,1}
// ===========================================================================
#[test]
fn row133_stable_dst() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[1, 3000, 70000, 200_000], &[0], &[4, 5], 0x133);
    for (i, c) in corpus.iter().enumerate() {
        for &stable in &[0u32, 1] {
            let mut o = LZ4F_decompressOptions_t::default();
            o.stableDst = stable;
            for &dstc in &[Chunk::All, Chunk::Fixed(1000), Chunk::Rand(7000)] {
                decode_both(
                    &ap,
                    &c.frame,
                    &c.orig,
                    Chunk::Rand(20_000),
                    dstc,
                    Some(o),
                    None,
                    0x1350 + i as u64,
                    &format!("row133 stableDst={stable} dst={dstc:?} {}", c.name),
                );
            }
        }
    }
}

// ===========================================================================
// Row 134 — skipChecksums {0,1} on frames that carry checksums
// ===========================================================================
#[test]
fn row134_skip_checksums() {
    let ap = apis();
    let mut rng = Rng::new(0x134);
    for &cc in &[0, 1] {
        for &bc in &[0, 1] {
            for &len in &[0usize, 1000, 70000] {
                let src = gen_src(Shape::Texty, len, &mut rng);
                let p = prefs_of(4, 0, cc, bc, 0);
                let ctx = format!("row134 cc={cc} bc={bc} len={len}");
                let frame = frame_both(&ap, &src, Some(&p), &ctx);
                for &skip in &[0u32, 1] {
                    let mut o = LZ4F_decompressOptions_t::default();
                    o.skipChecksums = skip;
                    decode_both(
                        &ap,
                        &frame,
                        &src,
                        Chunk::Rand(5000),
                        Chunk::Rand(5000),
                        Some(o),
                        None,
                        23,
                        &format!("{ctx} skip={skip}"),
                    );
                }
            }
        }
    }
    // A stored-block frame too (skipChecksums also gates the copyDirect path).
    let src = gen_src(Shape::Random, 100_000, &mut rng);
    let mut p = prefs_of(4, 1, 1, 1, 0);
    p.autoFlush = 1;
    let mut plan = plan_uniform(src.len(), 30_000);
    for e in plan.iter_mut() {
        e.1 = true;
    }
    let frame = stream_both(&ap, &src, Some(&p), &plan, None, None, "row134 stored");
    for &skip in &[0u32, 1] {
        let mut o = LZ4F_decompressOptions_t::default();
        o.skipChecksums = skip;
        decode_both(
            &ap,
            &frame,
            &src,
            Chunk::Rand(4000),
            Chunk::Rand(4000),
            Some(o),
            None,
            29,
            &format!("row134 stored skip={skip}"),
        );
    }
}

// ===========================================================================
// Row 135 — blockLinked frames chunked so matches reach into tmpOutBuffer
// ===========================================================================
#[test]
fn row135_linked_tmp_out_history() {
    let ap = apis();
    let mut rng = Rng::new(0x135);
    for &shape in &[Shape::FarMatches, Shape::Periodic, Shape::Runs] {
        for &len in &[200_000usize, 400_000] {
            let src = gen_src(shape, len, &mut rng);
            for &bsid in &[4, 5] {
                let p = prefs_of(bsid, 0 /* blockLinked */, 1, 0, 0);
                let ctx = format!("row135 {shape:?} len={len} bsid={bsid}");
                let frame = frame_both(&ap, &src, Some(&p), &ctx);
                // Small dst windows force decoding through tmpOut, so later
                // matches must reach back into the tmpOutBuffer history.
                for &dst in &[Chunk::Fixed(1000), Chunk::Fixed(9973), Chunk::Rand(3000)] {
                    decode_both(
                        &ap,
                        &frame,
                        &src,
                        Chunk::Rand(50_000),
                        dst,
                        None,
                        None,
                        31,
                        &format!("{ctx} dst={dst:?}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 136 — frames containing STORED (uncompressed) blocks
// ===========================================================================
#[test]
fn row136_stored_blocks_decode() {
    let ap = apis();
    let mut rng = Rng::new(0x136);
    for &bsid in &[4, 5] {
        for &cc in &[0, 1] {
            for &bc in &[0, 1] {
                let src = gen_src(Shape::Random, 150_000, &mut rng);
                let mut p = prefs_of(bsid, 1, cc, bc, 0);
                p.autoFlush = 1;
                let mut plan = plan_uniform(src.len(), 20_000);
                for e in plan.iter_mut() {
                    e.1 = true;
                }
                let ctx = format!("row136 bsid={bsid} cc={cc} bc={bc}");
                let frame = stream_both(&ap, &src, Some(&p), &plan, None, None, &ctx);
                for &(s, d) in &[
                    (Chunk::Fixed(1), Chunk::All),
                    (Chunk::Fixed(7), Chunk::Fixed(3)),
                    (Chunk::Rand(6000), Chunk::Rand(700)),
                    (Chunk::Hint, Chunk::All),
                ] {
                    let sub = if matches!(s, Chunk::Fixed(n) if n <= 7) {
                        // 1-byte-at-a-time over 150 KB is needlessly slow;
                        // re-encode a small frame for those chunk sizes.
                        let small = gen_src(Shape::Random, 2000, &mut rng);
                        let mut plan = plan_uniform(small.len(), 700);
                        for e in plan.iter_mut() {
                            e.1 = true;
                        }
                        let f = stream_both(
                            &ap,
                            &small,
                            Some(&p),
                            &plan,
                            None,
                            None,
                            &format!("{ctx} small"),
                        );
                        Some((f, small))
                    } else {
                        None
                    };
                    let (f, o) = match &sub {
                        Some((f, o)) => (f.as_slice(), o.as_slice()),
                        None => (frame.as_slice(), src.as_slice()),
                    };
                    decode_both(
                        &ap,
                        f,
                        o,
                        s,
                        d,
                        None,
                        None,
                        37,
                        &format!("{ctx} s={s:?} d={d:?}"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 137 — several frames concatenated, decoded on ONE dctx
// ===========================================================================
#[test]
fn row137_concatenated_frames() {
    let ap = apis();
    let mut rng = Rng::new(0x137);
    let mut all = Vec::new();
    let mut orig = Vec::new();
    for i in 0..6usize {
        let len = [0usize, 1, 700, 70000, 1000, 30000][i];
        let src = gen_src(ALL_SHAPES[i], len, &mut rng);
        let mut p = prefs_of(
            BSIDS[i % BSIDS.len()],
            (i % 2) as c_int,
            ((i / 2) % 2) as c_int,
            (i % 2) as c_int,
            [0, 1, 9][i % 3],
        );
        if i % 2 == 0 {
            p.frameInfo.contentSize = len as u64;
        }
        let f = frame_both(&ap, &src, Some(&p), &format!("row137 frame#{i}"));
        all.extend_from_slice(&f);
        orig.extend_from_slice(&src);
    }
    for &(s, d) in &[
        (Chunk::All, Chunk::All),
        (Chunk::Fixed(13), Chunk::Fixed(7)),
        (Chunk::Rand(4000), Chunk::Rand(4000)),
        (Chunk::Hint, Chunk::All),
    ] {
        decode_both(
            &ap,
            &all,
            &orig,
            s,
            d,
            None,
            None,
            41,
            &format!("row137 s={s:?} d={d:?}"),
        );
    }
}

// ===========================================================================
// Row 138 — a skippable frame followed by a real frame
// ===========================================================================
#[test]
fn row138_skippable_frame() {
    let ap = apis();
    let mut rng = Rng::new(0x138);
    for magic_lo in 0u32..16 {
        for &payload in &[0usize, 1, 5, 100] {
            let mut buf = Vec::new();
            let magic = 0x184D_2A50u32 + magic_lo;
            buf.extend_from_slice(&magic.to_le_bytes());
            buf.extend_from_slice(&(payload as u32).to_le_bytes());
            let junk = gen_src(Shape::Random, payload, &mut rng);
            buf.extend_from_slice(&junk);
            let src = gen_src(Shape::Texty, 3000, &mut rng);
            let p = prefs_of(4, 0, 1, 1, 0);
            let f = frame_both(&ap, &src, Some(&p), "row138 real frame");
            buf.extend_from_slice(&f);
            // LZ4F_headerSize must report 8 for the skippable frame.
            let ch = unsafe { (ap[0].headerSize)(buf.as_ptr() as *const c_void, 19) };
            let rh = unsafe { (ap[1].headerSize)(buf.as_ptr() as *const c_void, 19) };
            assert_ret_eq(ch, rh, "row138 headerSize");
            assert_eq!(ch, 8, "row138 skippable headerSize");
            for &(s, d) in &[
                (Chunk::All, Chunk::All),
                (Chunk::Fixed(1), Chunk::All),
                (Chunk::Fixed(3), Chunk::Fixed(5)),
                (Chunk::Rand(50), Chunk::Rand(500)),
            ] {
                decode_both(
                    &ap,
                    &buf,
                    &src,
                    s,
                    d,
                    None,
                    None,
                    43,
                    &format!("row138 magic={magic:#x} payload={payload} s={s:?}"),
                );
            }
        }
    }
}

// ===========================================================================
// Row 139 — an empty frame with every checksum combination
// ===========================================================================
#[test]
fn row139_empty_frame() {
    let ap = apis();
    let empty = gen_src(Shape::Random, 0, &mut Rng::new(1));
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &with_cs in &[false, true] {
                        let mut p = prefs_of(bsid, bmode, cc, bc, 0);
                        if with_cs {
                            p.frameInfo.contentSize = 1; // rewritten to srcSize (0)
                        }
                        let ctx = format!(
                            "row139 bsid={bsid} bmode={bmode} cc={cc} bc={bc} cs={with_cs}"
                        );
                        let frame = frame_both(&ap, &empty, Some(&p), &ctx);
                        for &(s, d) in &[
                            (Chunk::All, Chunk::All),
                            (Chunk::Fixed(1), Chunk::All),
                            (Chunk::Fixed(2), Chunk::Fixed(1)),
                            (Chunk::Hint, Chunk::All),
                        ] {
                            decode_both(
                                &ap,
                                &frame,
                                &empty,
                                s,
                                d,
                                None,
                                None,
                                47,
                                &format!("{ctx} s={s:?}"),
                            );
                        }
                    }
                }
            }
        }
    }
    // The same via the streaming API (compressEnd on an empty frame).
    for &cc in &[0, 1] {
        for &bc in &[0, 1] {
            let p = prefs_of(4, 0, cc, bc, 0);
            let plan: Vec<(usize, bool, bool)> = vec![];
            stream_both(
                &ap,
                &empty,
                Some(&p),
                &plan,
                None,
                None,
                &format!("row139 stream-noupdate cc={cc} bc={bc}"),
            );
            stream_both(
                &ap,
                &empty,
                Some(&p),
                &[(0, false, false)],
                None,
                None,
                &format!("row139 stream-empty-update cc={cc} bc={bc}"),
            );
            stream_both(
                &ap,
                &empty,
                Some(&p),
                &[(0, false, true)],
                None,
                None,
                &format!("row139 stream-empty-flush cc={cc} bc={bc}"),
            );
        }
    }
}

// ===========================================================================
// Row 140 — LZ4F_decompress_usingDict over rows 116-119, chunked src
// ===========================================================================
#[test]
fn row140_decompress_using_dict() {
    let ap = apis();
    let mut rng = Rng::new(0x140);
    for &ds in DICT_SIZES {
        let dict = gen_src(Shape::Texty, ds, &mut rng);
        let cdicts = [
            unsafe { (ap[0].createCDict)(dict.as_ptr() as *const c_void, ds) },
            unsafe { (ap[1].createCDict)(dict.as_ptr() as *const c_void, ds) },
        ];
        for &lvl in &[0, 9] {
            for &bmode in &[0, 1] {
                let len = 40_000usize;
                let src = gen_src(Shape::Texty, len, &mut rng);
                let p = prefs_of(4, bmode, 1, 1, lvl);
                let ctx = format!("row140 ds={ds} lvl={lvl} bmode={bmode}");
                // usingDict / usingDictOnce
                for &once in &[false, true] {
                    let plan = plan_uniform(len, 12_000);
                    let c = cctx_new(&ap[0]);
                    let r = cctx_new(&ap[1]);
                    let b = if once {
                        Begin::DictOnce(&dict)
                    } else {
                        Begin::Dict(&dict)
                    };
                    let cr = run_stream(&ap[0], c, &src, &p, &plan, ptr::null(), b, ptr::null());
                    let rr = run_stream(&ap[1], r, &src, &p, &plan, ptr::null(), b, ptr::null());
                    assert_bytes_eq(&cr.frame, &rr.frame, &format!("{ctx} once={once}: frame"));
                    unsafe {
                        (ap[0].freeCctx)(c);
                        (ap[1].freeCctx)(r);
                    }
                    for &(s, d) in &[
                        (Chunk::All, Chunk::All),
                        (Chunk::Fixed(17), Chunk::Fixed(11)),
                        (Chunk::Rand(3000), Chunk::Rand(3000)),
                    ] {
                        decode_both(
                            &ap,
                            &cr.frame,
                            &src,
                            s,
                            d,
                            None,
                            Some(&dict),
                            53,
                            &format!("{ctx} once={once} s={s:?}"),
                        );
                    }
                }
                // usingCDict
                let plan = plan_uniform(len, 15_000);
                let c = cctx_new(&ap[0]);
                let r = cctx_new(&ap[1]);
                let cr = run_stream(
                    &ap[0],
                    c,
                    &src,
                    &p,
                    &plan,
                    ptr::null(),
                    Begin::CDict,
                    cdicts[0],
                );
                let rr = run_stream(
                    &ap[1],
                    r,
                    &src,
                    &p,
                    &plan,
                    ptr::null(),
                    Begin::CDict,
                    cdicts[1],
                );
                assert_bytes_eq(&cr.frame, &rr.frame, &format!("{ctx} cdict: frame"));
                unsafe {
                    (ap[0].freeCctx)(c);
                    (ap[1].freeCctx)(r);
                }
                for &(s, d) in &[
                    (Chunk::All, Chunk::All),
                    (Chunk::Rand(2000), Chunk::Rand(2000)),
                ] {
                    decode_both(
                        &ap,
                        &cr.frame,
                        &src,
                        s,
                        d,
                        None,
                        Some(&dict),
                        59,
                        &format!("{ctx} cdict s={s:?}"),
                    );
                }
            }
        }
        // dict == NULL / dictSize == 0 must behave like plain LZ4F_decompress.
        let src = gen_src(Shape::Texty, 5000, &mut rng);
        let p = prefs_of(4, 0, 1, 0, 0);
        let frame = frame_both(&ap, &src, Some(&p), "row140 nodict");
        let nodict: &[u8] = &[];
        decode_both(
            &ap,
            &frame,
            &src,
            Chunk::Rand(500),
            Chunk::All,
            None,
            Some(nodict),
            61,
            "row140 empty dict",
        );
        unsafe {
            (ap[0].freeCDict)(cdicts[0]);
            (ap[1].freeCDict)(cdicts[1]);
        }
    }
}

// ===========================================================================
// Row 141 — LZ4F_resetDecompressionContext mid-frame, then a fresh frame
// ===========================================================================
#[test]
fn row141_reset_decompression_context() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[1000, 70000], &[0], &[4, 7], 0x141);
    for c in &corpus {
        let ctx = format!("row141 {}", c.name);
        for &cut in &[1usize, 3, 8, 19, 25] {
            let cut = cut.min(c.frame.len());
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut frees = [0usize; 2];
            for k in 0..2 {
                let d = dctx_new(&ap[k]);
                let mut out = vec![0xA5u8; c.orig.len() + 64];
                let mut ds = out.len();
                let mut ss = cut;
                let _ = unsafe {
                    (ap[k].decompress)(
                        d,
                        out.as_mut_ptr() as *mut c_void,
                        &mut ds,
                        c.frame.as_ptr() as *const c_void,
                        &mut ss,
                        ptr::null(),
                    )
                };
                unsafe { (ap[k].resetDctx)(d) };
                // A full, fresh frame must now decode cleanly.
                let mut out2 = vec![0xA5u8; c.orig.len() + 64];
                let mut ds2 = out2.len();
                let mut ss2 = c.frame.len();
                let r = unsafe {
                    (ap[k].decompress)(
                        d,
                        out2.as_mut_ptr() as *mut c_void,
                        &mut ds2,
                        c.frame.as_ptr() as *const c_void,
                        &mut ss2,
                        ptr::null(),
                    )
                };
                assert_eq!(r, 0, "{ctx} cut={cut}: post-reset decode ret {r:#x}");
                out2.truncate(ds2);
                outs[k] = out2;
                frees[k] = unsafe { (ap[k].freeDctx)(d) };
            }
            assert_bytes_eq(&outs[0], &outs[1], &format!("{ctx} cut={cut}: decoded"));
            assert_bytes_eq(&outs[0], &c.orig, &format!("{ctx} cut={cut}: round trip"));
            assert_ret_eq(frees[0], frees[1], &format!("{ctx} cut={cut}: freeDctx"));
        }
    }
}

// ===========================================================================
// Row 142 — createDecompressionContext_advanced + ONE dctx for many frames
// ===========================================================================
#[test]
fn row142_dctx_advanced_reused() {
    let ap = apis();
    let corpus = make_corpus(&ap, &[0, 1, 1000, 70000], &[0], &[4, 7], 0x142);
    let d = [
        unsafe { (ap[0].createDctxAdvanced)(DEFAULT_CMEM, LZ4F_VERSION) },
        unsafe { (ap[1].createDctxAdvanced)(DEFAULT_CMEM, LZ4F_VERSION) },
    ];
    assert!(
        !d[0].is_null() && !d[1].is_null(),
        "createDecompressionContext_advanced"
    );
    for (i, c) in corpus.iter().enumerate() {
        let ctx = format!("row142 {}", c.name);
        let mut outs: [DecRun; 2] = [
            run_decode(
                &ap[0],
                d[0],
                &c.frame,
                c.orig.len() + 64,
                Chunk::Rand(4000),
                Chunk::Rand(4000),
                ptr::null(),
                None,
                0x1420 + i as u64,
            ),
            run_decode(
                &ap[1],
                d[1],
                &c.frame,
                c.orig.len() + 64,
                Chunk::Rand(4000),
                Chunk::Rand(4000),
                ptr::null(),
                None,
                0x1420 + i as u64,
            ),
        ];
        assert_ret_eq(&outs[0].rets, &outs[1].rets, &format!("{ctx}: returns"));
        assert_bytes_eq(&outs[0].out, &outs[1].out, &format!("{ctx}: decoded"));
        assert_bytes_eq(&outs[0].out, &c.orig, &format!("{ctx}: round trip"));
        outs[0].out.clear();
        outs[1].out.clear();
    }
    let cf = unsafe { (ap[0].freeDctx)(d[0]) };
    let rf = unsafe { (ap[1].freeDctx)(d[1]) };
    assert_ret_eq(cf, rf, "row142 freeDecompressionContext");
}

// ===========================================================================
// Row 143 — CROSS-DECODE: C frame decoded by Rust and vice versa
// ===========================================================================
#[test]
fn row143_cross_decode() {
    let ap = apis();
    let mut rng = Rng::new(0x143);
    for &bsid in BSIDS {
        for &bmode in &[0, 1] {
            for &cc in &[0, 1] {
                for &bc in &[0, 1] {
                    for &len in &[0usize, 1000, 70000] {
                        let src = gen_src(Shape::Mixed, len, &mut rng);
                        let mut p = prefs_of(bsid, bmode, cc, bc, [0, 9][len % 2]);
                        p.frameInfo.contentSize = len as u64;
                        let ctx = format!(
                            "row143 bsid={bsid} bmode={bmode} cc={cc} bc={bc} len={len}"
                        );
                        // Each library produces its own frame...
                        let bound = unsafe { (ap[0].compressFrameBound)(len, &p) };
                        let mut frames: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
                        for k in 0..2 {
                            let mut buf = vec![0xA5u8; bound + 32];
                            let n = unsafe {
                                (ap[k].compressFrame)(
                                    buf.as_mut_ptr() as *mut c_void,
                                    bound,
                                    src.as_ptr() as *const c_void,
                                    len,
                                    &p,
                                )
                            };
                            assert!(!is_lz4f_error(n), "{ctx}: compressFrame[{k}] {n:#x}");
                            buf.truncate(n);
                            frames[k] = buf;
                        }
                        assert_bytes_eq(&frames[0], &frames[1], &format!("{ctx}: frames"));
                        // ...and each library decodes the OTHER library's frame.
                        for prod in 0..2usize {
                            let dec = 1 - prod;
                            let d = dctx_new(&ap[dec]);
                            let run = run_decode(
                                &ap[dec],
                                d,
                                &frames[prod],
                                len + 64,
                                Chunk::Rand(3000),
                                Chunk::Rand(3000),
                                ptr::null(),
                                None,
                                67,
                            );
                            assert_bytes_eq(
                                &run.out,
                                &src,
                                &format!("{ctx}: frame from {prod} decoded by {dec}"),
                            );
                            let f = unsafe { (ap[dec].freeDctx)(d) };
                            assert_eq!(f, 0, "{ctx}: dctx not clean");
                        }
                    }
                }
            }
        }
    }
}
