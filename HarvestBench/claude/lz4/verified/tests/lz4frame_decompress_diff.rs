//! Differential tests for the DECOMPRESSION side of `lz4frame.c`.
//!
//! Frames are always *built* with the C library (the ground truth) and then fed
//! to BOTH decoders through the `.so` export tables, so any divergence observed
//! here is a decoder divergence.
//!
//! Every `LZ4F_decompress` call compares
//!   1. the return value (the streaming "hint"),
//!   2. both mutated out-params (`*dstSizePtr`, `*srcSizePtr`),
//!   3. the destination buffers (identically 0xAA-prefilled; the newly written
//!      window after every call, and the FULL buffer once the loop ends),
//!   4. the final `dStage` — obtained for free, because
//!      `LZ4F_freeDecompressionContext()` returns `dctx->dStage`.
//!
//! ---------------------------------------------------------------------------
//! dStage coverage map (`dStage_t`, lz4frame.c:1248-1258)
//! ---------------------------------------------------------------------------
//!  dstage_getFrameHeader   : every test. The `>= maxFHSize` shortcut is taken by
//!                            every `SrcPlan::OneShot` row on a frame >= 19 B
//!                            (`decompress_one_shot_option_matrix`).
//!  dstage_storeFrameHeader : `SrcPlan::Fixed(1..16)` rows in
//!                            `decompress_src_chunk_granularity` /
//!                            `decompress_src_dst_cross_product`, and every
//!                            one-shot row of a frame shorter than 19 bytes
//!                            (`decompress_tiny_and_boundary_sizes`, len 0/1/2).
//!  dstage_init             : every non-skippable frame.
//!  dstage_getBlockHeader   : every non-skippable frame.
//!  dstage_storeBlockHeader : `SrcPlan::Fixed(1|2|3)` rows (chunk < BHSize) in
//!                            `decompress_src_chunk_granularity`,
//!                            `decompress_src_dst_cross_product`,
//!                            `decompress_random_chunking`.
//!  dstage_copyDirect       : stored/uncompressed blocks — the `random` shape at
//!                            block-filling sizes, plus every frame built with
//!                            `LZ4F_uncompressedUpdate` in
//!                            `decompress_uncompressed_blocks`. The *partial*
//!                            copyDirect return (`tmpInTarget -= sizeToCopy`) is
//!                            forced there by the `Fixed(1|2|3)` rows.
//!  dstage_getBlockChecksum : `blockChecksumFlag=1` + an uncompressed block:
//!                            `decompress_uncompressed_blocks` covers both the
//!                            "4 bytes immediately available" path and the
//!                            byte-at-a-time `dctx->header` accumulation path.
//!  dstage_getCBlock        : every compressible shape.
//!  dstage_storeCBlock      : any `SrcPlan::Fixed(n)` with n < cBlockSize, i.e.
//!                            all of `decompress_src_chunk_granularity`.
//!  dstage_flushOut         : dst room < `maxBlockSize` — every small-content row
//!                            (out buffer < 64 KB) and every small
//!                            `DstPlan::Fixed(..)`. Partial flushOut (dst
//!                            exhausted mid-block) is forced by `Fixed(1)`.
//!  dstage_getSuffix        : every non-skippable frame (end mark).
//!  dstage_storeSuffix      : `contentChecksumFlag=1` + `SrcPlan::Fixed(1|2|3)`.
//!  dstage_getSFrameSize    : `decompress_skippable_frames`, one-shot rows
//!                            (>= 19 B available, so `src != dctx->header`).
//!  dstage_storeSFrameSize  : `decompress_skippable_frames`, `Fixed(1..3)` rows.
//!                            Both entries are reached: via `storeFrameHeader`
//!                            (first call has < 19 B, so `LZ4F_decodeHeader` is
//!                            called on `dctx->header`) and via
//!                            `getSFrameSize` with < 4 bytes left.
//!  dstage_skipSkippable    : `decompress_skippable_frames`; multi-call skipping
//!                            is forced by the `Fixed(1)`/`Fixed(3)` rows, and
//!                            `get_frame_info_during_skippable_skip` stops inside
//!                            it on purpose.
//!
//! Every stage above is also reached as the *entry* state of a fresh
//! `LZ4F_decompress` call (not only by fall-through) except `dstage_getSuffix`,
//! which the C state machine can only reach by falling through from the block
//! header decode inside one call. `dstage_init` as an entry state needs
//! `LZ4F_getFrameInfo` to decode the header first — that is
//! `decompress_entering_at_dstage_init`.
//!
//! Destination-buffer geometry is swept two ways, because they drive completely
//! different `LZ4F_updateDict()` branches:
//!   * ADVANCING dst (default): all output stays live and contiguous, so the
//!     dictionary stays in "prefix mode" inside the caller's buffer.
//!   * RECYCLED dst (`decompress_recycled_dst_buffer`): the same buffer is handed
//!     to every call, so the decoder must migrate the 64 KB history into
//!     `tmpOutBuffer` — that covers "continue history within tmpOutBuffer",
//!     "copy relevant dict portion in front of tmpOut", "copy dst into tmp to
//!     complete dict", "join dict & dest into tmp", the `dictSize > 128 KB`
//!     truncation, and the end-of-call "preserve history within tmpOut" block in
//!     both its `flushOut` and non-`flushOut` forms.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Signatures — verified line by line against c_src/src/lz4frame.c
// ---------------------------------------------------------------------------

// --- decompression -----------------------------------------------------------
type FnCreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateDctxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnFreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnResetDctx = unsafe extern "C" fn(*mut c_void);
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnGetFrameInfo =
    unsafe extern "C" fn(*mut c_void, *mut LZ4F_frameInfo_t, *const c_void, *mut usize) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;

// --- compression (used only to *build* the frames under test) ----------------
type FnCreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompressBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
type FnCompressBeginUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressBeginUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCompressUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnCompressEnd =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;
type FnCompressBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);

const SENTINEL: u8 = 0xAA;
/// `LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (lz4frame.h:404)
const MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

// ---------------------------------------------------------------------------
// Cached symbol table (dlsym once, not once per decompress call)
// ---------------------------------------------------------------------------

struct Api {
    create_dctx: (FnCreateDctx, FnCreateDctx),
    create_dctx_adv: (FnCreateDctxAdv, FnCreateDctxAdv),
    free_dctx: (FnFreeDctx, FnFreeDctx),
    reset_dctx: (FnResetDctx, FnResetDctx),
    header_size: (FnHeaderSize, FnHeaderSize),
    get_frame_info: (FnGetFrameInfo, FnGetFrameInfo),
    decompress: (FnDecompress, FnDecompress),
    decompress_dict: (FnDecompressUsingDict, FnDecompressUsingDict),
    get_block_size: (FnGetBlockSize, FnGetBlockSize),

    c_create_cctx: FnCreateCctx,
    c_free_cctx: FnFreeCctx,
    c_begin: FnCompressBegin,
    c_begin_dict: FnCompressBeginUsingDict,
    c_begin_cdict: FnCompressBeginUsingCDict,
    c_update: FnCompressUpdate,
    c_uncompressed_update: FnCompressUpdate,
    c_end: FnCompressEnd,
    c_bound: FnCompressBound,
    c_create_cdict: FnCreateCDict,
    c_free_cdict: FnFreeCDict,
}

fn api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    let a = A.get_or_init(|| {
        let l = libs();
        Api {
            create_dctx: both::<FnCreateDctx>("LZ4F_createDecompressionContext"),
            create_dctx_adv: both::<FnCreateDctxAdv>("LZ4F_createDecompressionContext_advanced"),
            free_dctx: both::<FnFreeDctx>("LZ4F_freeDecompressionContext"),
            reset_dctx: both::<FnResetDctx>("LZ4F_resetDecompressionContext"),
            header_size: both::<FnHeaderSize>("LZ4F_headerSize"),
            get_frame_info: both::<FnGetFrameInfo>("LZ4F_getFrameInfo"),
            decompress: both::<FnDecompress>("LZ4F_decompress"),
            decompress_dict: both::<FnDecompressUsingDict>("LZ4F_decompress_usingDict"),
            get_block_size: both::<FnGetBlockSize>("LZ4F_getBlockSize"),

            c_create_cctx: l.c.sym::<FnCreateCctx>("LZ4F_createCompressionContext"),
            c_free_cctx: l.c.sym::<FnFreeCctx>("LZ4F_freeCompressionContext"),
            c_begin: l.c.sym::<FnCompressBegin>("LZ4F_compressBegin"),
            c_begin_dict: l.c.sym::<FnCompressBeginUsingDict>("LZ4F_compressBegin_usingDict"),
            c_begin_cdict: l.c.sym::<FnCompressBeginUsingCDict>("LZ4F_compressBegin_usingCDict"),
            c_update: l.c.sym::<FnCompressUpdate>("LZ4F_compressUpdate"),
            c_uncompressed_update: l.c.sym::<FnCompressUpdate>("LZ4F_uncompressedUpdate"),
            c_end: l.c.sym::<FnCompressEnd>("LZ4F_compressEnd"),
            c_bound: l.c.sym::<FnCompressBound>("LZ4F_compressBound"),
            c_create_cdict: l.c.sym::<FnCreateCDict>("LZ4F_createCDict"),
            c_free_cdict: l.c.sym::<FnFreeCDict>("LZ4F_freeCDict"),
        }
    });
    // Sanity: the two libraries must really be distinct code objects, otherwise
    // the whole differential setup would be vacuous.
    assert_ne!(
        a.decompress.0 as usize, a.decompress.1 as usize,
        "C and Rust LZ4F_decompress resolved to the same address"
    );
    assert_ne!(
        a.get_frame_info.0 as usize, a.get_frame_info.1 as usize,
        "C and Rust LZ4F_getFrameInfo resolved to the same address"
    );
    assert_ne!(
        a.header_size.0 as usize, a.header_size.1 as usize,
        "C and Rust LZ4F_headerSize resolved to the same address"
    );
    a
}

fn ret_str(r: usize) -> String {
    if lz4f_is_error(r) {
        format!("ERROR({})", lz4f_error_code(r))
    } else {
        format!("{}", r)
    }
}

fn block_size_of(bsid: c_int) -> usize {
    match bsid {
        0 | 4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => panic!("bad blockSizeID {}", bsid),
    }
}

fn poison_fi(fi: &mut LZ4F_frameInfo_t) {
    unsafe {
        std::ptr::write_bytes(
            fi as *mut LZ4F_frameInfo_t as *mut u8,
            0x5A,
            std::mem::size_of::<LZ4F_frameInfo_t>(),
        );
    }
}

fn fi_bytes(fi: &LZ4F_frameInfo_t) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            fi as *const LZ4F_frameInfo_t as *const u8,
            std::mem::size_of::<LZ4F_frameInfo_t>(),
        )
    }
}

// ---------------------------------------------------------------------------
// Frame construction (C library only)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DictKind {
    None,
    Raw,
    CDict,
}

#[derive(Clone, Copy)]
struct FrameSpec<'a> {
    prefs: LZ4F_preferences_t,
    /// Size of each `LZ4F_compressUpdate` call.
    update_chunk: usize,
    dict: Option<&'a [u8]>,
    dict_kind: DictKind,
    /// Use `LZ4F_uncompressedUpdate` instead of `LZ4F_compressUpdate`.
    uncompressed: bool,
}

impl<'a> FrameSpec<'a> {
    fn new(prefs: LZ4F_preferences_t) -> Self {
        FrameSpec {
            prefs,
            update_chunk: usize::MAX,
            dict: None,
            dict_kind: DictKind::None,
            uncompressed: false,
        }
    }
}

/// Build one LZ4 frame with the **C** library, honouring every field of `prefs`
/// exactly (unlike `LZ4F_compressFrame`, which overrides blockMode /
/// blockSizeID / autoFlush).
fn build_frame(spec: &FrameSpec, data: &[u8]) -> Vec<u8> {
    let a = api();
    unsafe {
        let mut cctx: *mut c_void = std::ptr::null_mut();
        let r = (a.c_create_cctx)(&mut cctx, LZ4F_VERSION);
        assert!(!lz4f_is_error(r), "C createCompressionContext failed");
        assert!(!cctx.is_null());

        let prefs = &spec.prefs as *const LZ4F_preferences_t;
        let mut out: Vec<u8> = vec![0u8; LZ4F_HEADER_SIZE_MAX];

        let mut cdict: *mut c_void = std::ptr::null_mut();
        let hn = match (spec.dict, spec.dict_kind) {
            (Some(d), DictKind::CDict) => {
                cdict = (a.c_create_cdict)(d.as_ptr() as *const c_void, d.len());
                assert!(!cdict.is_null(), "LZ4F_createCDict failed");
                (a.c_begin_cdict)(
                    cctx,
                    out.as_mut_ptr() as *mut c_void,
                    LZ4F_HEADER_SIZE_MAX,
                    cdict,
                    prefs,
                )
            }
            (Some(d), DictKind::Raw) => (a.c_begin_dict)(
                cctx,
                out.as_mut_ptr() as *mut c_void,
                LZ4F_HEADER_SIZE_MAX,
                d.as_ptr() as *const c_void,
                d.len(),
                prefs,
            ),
            _ => (a.c_begin)(
                cctx,
                out.as_mut_ptr() as *mut c_void,
                LZ4F_HEADER_SIZE_MAX,
                prefs,
            ),
        };
        assert!(!lz4f_is_error(hn), "C compressBegin failed: {}", ret_str(hn));
        out.truncate(hn);

        let step = spec.update_chunk.max(1);
        let mut off = 0usize;
        while off < data.len() {
            let end = (off + step).min(data.len());
            let chunk = &data[off..end];
            let bound = (a.c_bound)(chunk.len(), prefs).max(chunk.len() + 32);
            let start = out.len();
            out.resize(start + bound, 0);
            let f = if spec.uncompressed {
                a.c_uncompressed_update
            } else {
                a.c_update
            };
            let w = f(
                cctx,
                out.as_mut_ptr().add(start) as *mut c_void,
                bound,
                chunk.as_ptr() as *const c_void,
                chunk.len(),
                std::ptr::null(),
            );
            assert!(
                !lz4f_is_error(w),
                "C compressUpdate failed: {} (chunk {})",
                ret_str(w),
                chunk.len()
            );
            out.truncate(start + w);
            off = end;
        }

        let bound = (a.c_bound)(0, prefs).max(64);
        let start = out.len();
        out.resize(start + bound, 0);
        let w = (a.c_end)(
            cctx,
            out.as_mut_ptr().add(start) as *mut c_void,
            bound,
            std::ptr::null(),
        );
        assert!(!lz4f_is_error(w), "C compressEnd failed: {}", ret_str(w));
        out.truncate(start + w);

        if !cdict.is_null() {
            (a.c_free_cdict)(cdict);
        }
        (a.c_free_cctx)(cctx);
        out
    }
}

#[allow(clippy::too_many_arguments)]
fn prefs_of(
    bsid: c_int,
    bmode: c_int,
    cc: c_int,
    bc: c_int,
    content_size: u64,
    dict_id: c_uint,
    level: c_int,
    auto_flush: c_uint,
) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = bmode;
    p.frameInfo.contentChecksumFlag = cc;
    p.frameInfo.blockChecksumFlag = bc;
    p.frameInfo.contentSize = content_size;
    p.frameInfo.dictID = dict_id;
    p.frameInfo.frameType = LZ4F_frame;
    p.compressionLevel = level;
    p.autoFlush = auto_flush;
    p
}

/// A hand-crafted skippable frame: magic, 4-byte LE size, then `payload`.
/// (`LZ4F_compressBegin` ignores `frameType`, so these cannot be produced by the
/// compressor.)
fn skippable_frame(magic: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + payload.len());
    v.extend_from_slice(&magic.to_le_bytes());
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

// ---------------------------------------------------------------------------
// The differential decompression driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum SrcPlan {
    /// Offer the whole remaining frame in one call.
    OneShot,
    /// Offer exactly `n` bytes per call.
    Fixed(usize),
    /// Offer `range(lo,hi)` bytes per call, from a seeded PRNG.
    Random(u64, usize, usize),
    /// Offer exactly as many bytes as the previous call's hint requested.
    Hint,
}

#[derive(Clone, Copy, Debug)]
enum DstPlan {
    /// All remaining room in the output buffer.
    All,
    /// At most `n` bytes per call.
    Fixed(usize),
}

#[derive(Clone, Copy)]
struct DecodeCfg<'a> {
    src: SrcPlan,
    dst: DstPlan,
    opts: Option<LZ4F_decompressOptions_t>,
    /// `Some` => drive `LZ4F_decompress_usingDict` with this dictionary.
    dict: Option<&'a [u8]>,
    /// `false` (default): the destination pointer ADVANCES inside one large
    ///   buffer, so all output stays contiguous and live (the pattern that keeps
    ///   `LZ4F_updateDict` in "prefix mode").
    /// `true`: the SAME small buffer is handed to every call and its contents are
    ///   consumed in between — the classic streaming pattern, which forces
    ///   `LZ4F_updateDict` through its tmpOutBuffer branches and forces the
    ///   end-of-call "preserve history within tmpOut" block to actually copy.
    recycle: bool,
}

impl<'a> DecodeCfg<'a> {
    fn new(src: SrcPlan, dst: DstPlan) -> Self {
        DecodeCfg {
            src,
            dst,
            opts: None,
            dict: None,
            recycle: false,
        }
    }
    fn recycled(src: SrcPlan, buf: usize) -> Self {
        DecodeCfg {
            src,
            dst: DstPlan::Fixed(buf),
            opts: None,
            dict: None,
            recycle: true,
        }
    }
}

struct DriveResult {
    out: Vec<u8>,
    /// Return value of the last `LZ4F_decompress` call.
    last_ret: usize,
    consumed: usize,
    calls: usize,
}

fn dopts(stable: c_uint, skip: c_uint) -> LZ4F_decompressOptions_t {
    LZ4F_decompressOptions_t {
        stableDst: stable,
        skipChecksums: skip,
        reserved1: 0,
        reserved0: 0,
    }
}

/// Drive both decoders over `frame` with identical call granularity.
fn drive_with(
    cd: *mut c_void,
    rd: *mut c_void,
    frame: &[u8],
    out_len: usize,
    cfg: &DecodeCfg,
    label: &str,
) -> DriveResult {
    let a = api();
    let buf_len = if cfg.recycle {
        match cfg.dst {
            DstPlan::All => out_len,
            DstPlan::Fixed(n) => n.max(1),
        }
    } else {
        out_len
    };
    let mut out_c = vec![SENTINEL; buf_len];
    let mut out_r = vec![SENTINEL; buf_len];
    let mut acc_c: Vec<u8> = Vec::new();
    let mut acc_r: Vec<u8> = Vec::new();

    let opt_ptr = match &cfg.opts {
        Some(o) => o as *const LZ4F_decompressOptions_t,
        None => std::ptr::null(),
    };
    let (dict_ptr, dict_len) = match cfg.dict {
        Some(d) => (d.as_ptr() as *const c_void, d.len()),
        None => (std::ptr::null(), 0usize),
    };

    let mut rng = match cfg.src {
        SrcPlan::Random(seed, _, _) => Some(Rng::new(seed)),
        _ => None,
    };

    let mut consumed = 0usize;
    let mut produced = 0usize;
    let mut last_ret = 0usize;
    let mut calls = 0usize;

    unsafe {
        loop {
            calls += 1;
            assert!(
                calls < 8_000_000,
                "{}: runaway decompress loop ({} calls)",
                label,
                calls
            );

            let want = match cfg.src {
                SrcPlan::OneShot => usize::MAX,
                SrcPlan::Fixed(n) => n.max(1),
                SrcPlan::Random(_, lo, hi) => rng.as_mut().unwrap().range(lo.max(1), hi.max(1)),
                SrcPlan::Hint => {
                    if last_ret == 0 || lz4f_is_error(last_ret) {
                        LZ4F_HEADER_SIZE_MAX
                    } else {
                        last_ret
                    }
                }
            };
            let src_len = want.min(frame.len() - consumed);
            let (off, dst_len) = if cfg.recycle {
                assert!(
                    produced < 512 << 20,
                    "{}: recycled-dst decode produced absurdly much output",
                    label
                );
                (0usize, buf_len)
            } else {
                let room = buf_len - produced;
                if room == 0 {
                    // The frame decoded to more than the caller predicted (only
                    // possible for deliberately corrupted frames).
                    break;
                }
                (
                    produced,
                    match cfg.dst {
                        DstPlan::All => room,
                        DstPlan::Fixed(n) => n.min(room),
                    },
                )
            };

            let mut c_src = src_len;
            let mut r_src = src_len;
            let mut c_dst = dst_len;
            let mut r_dst = dst_len;

            let src_at = frame.as_ptr().add(consumed) as *const c_void;
            let c_dst_at = out_c.as_mut_ptr().add(off) as *mut c_void;
            let r_dst_at = out_r.as_mut_ptr().add(off) as *mut c_void;

            let (cret, rret) = if cfg.dict.is_some() {
                (
                    (a.decompress_dict.0)(
                        cd, c_dst_at, &mut c_dst, src_at, &mut c_src, dict_ptr, dict_len, opt_ptr,
                    ),
                    (a.decompress_dict.1)(
                        rd, r_dst_at, &mut r_dst, src_at, &mut r_src, dict_ptr, dict_len, opt_ptr,
                    ),
                )
            } else {
                (
                    (a.decompress.0)(cd, c_dst_at, &mut c_dst, src_at, &mut c_src, opt_ptr),
                    (a.decompress.1)(rd, r_dst_at, &mut r_dst, src_at, &mut r_src, opt_ptr),
                )
            };

            // Cheap comparisons first; only format a message on divergence.
            if cret != rret || c_src != r_src || c_dst != r_dst {
                let ctx = format!(
                    "{} [call {} srcOffer={} dstOffer={} consumed={} produced={}]",
                    label, calls, src_len, dst_len, consumed, produced
                );
                assert_eq!(
                    ret_str(cret),
                    ret_str(rret),
                    "{}: LZ4F_decompress return (hint) mismatch",
                    ctx
                );
                assert_eq!(c_src, r_src, "{}: *srcSizePtr mismatch", ctx);
                assert_eq!(c_dst, r_dst, "{}: *dstSizePtr mismatch", ctx);
                unreachable!();
            }

            // Window just written (+ sentinel margin). The FULL buffers are
            // compared once the loop ends.
            let lo = off;
            let hi = if cfg.recycle {
                buf_len
            } else {
                (off + c_dst + 64).min(buf_len)
            };
            if out_c[lo..hi] != out_r[lo..hi] {
                let ctx = format!(
                    "{} [call {} srcOffer={} dstOffer={} consumed={} produced={}]",
                    label, calls, src_len, dst_len, consumed, produced
                );
                assert_bytes_eq(&format!("{}: dst bytes", ctx), &out_c[lo..hi], &out_r[lo..hi]);
            }

            last_ret = cret;
            if lz4f_is_error(cret) {
                break;
            }
            assert!(
                c_src <= src_len && c_dst <= dst_len,
                "{}: out-param exceeds what was offered (src {}>{} dst {}>{})",
                label,
                c_src,
                src_len,
                c_dst,
                dst_len
            );

            consumed += c_src;
            produced += c_dst;
            if cfg.recycle {
                acc_c.extend_from_slice(&out_c[..c_dst]);
                acc_r.extend_from_slice(&out_r[..c_dst]);
            }

            if c_src == 0 && c_dst == 0 {
                // No forward progress possible with what we are offering.
                break;
            }
            if cret == 0 && consumed == frame.len() {
                break;
            }
        }
    }

    assert_bytes_eq(&format!("{}: full dst buffer", label), &out_c, &out_r);
    let out = if cfg.recycle {
        assert_bytes_eq(
            &format!("{}: accumulated output", label),
            &acc_c,
            &acc_r,
        );
        acc_c
    } else {
        out_c.truncate(produced);
        out_c
    };
    DriveResult {
        out,
        last_ret,
        consumed,
        calls,
    }
}

/// Create a fresh dctx pair, drive them, then free both and compare the freed
/// return value (which is `dctx->dStage`).
fn drive(frame: &[u8], expect_len: usize, cfg: &DecodeCfg, label: &str) -> DriveResult {
    let a = api();
    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        let c0 = (a.create_dctx.0)(&mut cd, LZ4F_VERSION);
        let r0 = (a.create_dctx.1)(&mut rd, LZ4F_VERSION);
        assert_eq!(c0, r0, "{}: createDecompressionContext return", label);
        assert!(!cd.is_null() && !rd.is_null(), "{}: null dctx", label);

        let res = drive_with(cd, rd, frame, expect_len + 64, cfg, label);

        let cf = (a.free_dctx.0)(cd);
        let rf = (a.free_dctx.1)(rd);
        assert_eq!(
            cf, rf,
            "{}: freeDecompressionContext return (final dStage) mismatch",
            label
        );
        res
    }
}

/// Full round trip: build a frame from `data`, decode with both libraries and
/// require the decoded bytes to equal `data`.
fn roundtrip(spec: &FrameSpec, data: &[u8], cfg: &DecodeCfg, label: &str) {
    let frame = build_frame(spec, data);
    let res = drive(&frame, data.len(), cfg, label);
    assert!(
        !lz4f_is_error(res.last_ret),
        "{}: decode failed with {} after {} calls",
        label,
        ret_str(res.last_ret),
        res.calls
    );
    assert_eq!(
        res.consumed,
        frame.len(),
        "{}: decoder did not consume the whole frame ({} of {})",
        label,
        res.consumed,
        frame.len()
    );
    assert_eq!(
        res.last_ret,
        0,
        "{}: frame not reported complete (hint {})",
        label,
        ret_str(res.last_ret)
    );
    assert_bytes_eq(&format!("{}: decoded content", label), data, &res.out);
}

// ===========================================================================
// 1. context lifecycle: create / create_advanced / reset / free
// ===========================================================================

#[test]
fn dctx_lifecycle_create_reset_free() {
    let a = api();
    unsafe {
        for version in [LZ4F_VERSION, 0, 1, 99, 101, 0xFFFF_FFFF] {
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            let c = (a.create_dctx.0)(&mut cd, version);
            let r = (a.create_dctx.1)(&mut rd, version);
            assert_eq!(
                ret_str(c),
                ret_str(r),
                "createDecompressionContext(version={})",
                version
            );
            assert_eq!(cd.is_null(), rd.is_null(), "createDecompressionContext null");
            let cf = (a.free_dctx.0)(cd);
            let rf = (a.free_dctx.1)(rd);
            assert_eq!(cf, rf, "freeDecompressionContext(fresh)");
            assert_eq!(cf, 0, "fresh dctx dStage must be dstage_getFrameHeader");
        }

        // free(NULL) is explicitly supported
        let cf = (a.free_dctx.0)(std::ptr::null_mut());
        let rf = (a.free_dctx.1)(std::ptr::null_mut());
        assert_eq!(cf, rf, "freeDecompressionContext(NULL)");
        assert_eq!(cf, 0);

        // _advanced with the default (all-NULL) custom allocator
        for version in [LZ4F_VERSION, 0, 12345] {
            let cm = LZ4F_CustomMem::default();
            let cd = (a.create_dctx_adv.0)(cm, version);
            let rd = (a.create_dctx_adv.1)(cm, version);
            assert_eq!(cd.is_null(), rd.is_null(), "create_advanced nullness");
            assert!(!cd.is_null());
            assert_eq!(
                (a.free_dctx.0)(cd),
                (a.free_dctx.1)(rd),
                "free after _advanced"
            );
        }

        let mut rng = Rng::new(0x5EED_0001);
        let data = gen_shape(&mut rng, 4, 5000);
        let spec = FrameSpec::new(prefs_of(
            LZ4F_max64KB,
            LZ4F_blockLinked,
            LZ4F_contentChecksumEnabled,
            LZ4F_blockChecksumEnabled,
            data.len() as u64,
            0x1234_5678,
            0,
            0,
        ));
        let frame = build_frame(&spec, &data);

        // A dctx frozen mid-frame must report the same dStage on free.
        let cd2 = (a.create_dctx_adv.0)(LZ4F_CustomMem::default(), LZ4F_VERSION);
        let rd2 = (a.create_dctx_adv.1)(LZ4F_CustomMem::default(), LZ4F_VERSION);
        {
            let mut o_c = vec![SENTINEL; 8192];
            let mut o_r = vec![SENTINEL; 8192];
            let mut cs = 30usize;
            let mut rs = 30usize;
            let mut cdst = o_c.len();
            let mut rdst = o_r.len();
            let cret = (a.decompress.0)(
                cd2,
                o_c.as_mut_ptr() as *mut c_void,
                &mut cdst,
                frame.as_ptr() as *const c_void,
                &mut cs,
                std::ptr::null(),
            );
            let rret = (a.decompress.1)(
                rd2,
                o_r.as_mut_ptr() as *mut c_void,
                &mut rdst,
                frame.as_ptr() as *const c_void,
                &mut rs,
                std::ptr::null(),
            );
            assert_eq!(ret_str(cret), ret_str(rret), "partial decode hint");
            assert_eq!((cs, cdst), (rs, rdst), "partial decode out-params");
            assert_bytes_eq("partial decode dst", &o_c, &o_r);
        }
        let cf = (a.free_dctx.0)(cd2);
        let rf = (a.free_dctx.1)(rd2);
        assert_eq!(cf, rf, "free mid-stream must return the same dStage");
        assert_ne!(cf, 0, "mid-stream dStage should not be getFrameHeader");

        // reset then reuse the same contexts for a full decode
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        assert_eq!(
            (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
            (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
        );
        {
            let cfg = DecodeCfg::new(SrcPlan::Fixed(23), DstPlan::Fixed(77));
            let r1 = drive_with(cd, rd, &frame[..40], data.len() + 64, &cfg, "partial-then-reset");
            assert!(!lz4f_is_error(r1.last_ret));
        }
        (a.reset_dctx.0)(cd);
        (a.reset_dctx.1)(rd);
        let cfg = DecodeCfg::new(SrcPlan::OneShot, DstPlan::All);
        let res = drive_with(cd, rd, &frame, data.len() + 64, &cfg, "reset-then-decode");
        assert_eq!(res.last_ret, 0, "reset-then-decode should complete");
        assert_bytes_eq("reset-then-decode content", &data, &res.out);

        // reset is idempotent
        (a.reset_dctx.0)(cd);
        (a.reset_dctx.0)(cd);
        (a.reset_dctx.1)(rd);
        (a.reset_dctx.1)(rd);
        let cf = (a.free_dctx.0)(cd);
        let rf = (a.free_dctx.1)(rd);
        assert_eq!(cf, rf, "free after reset");
        assert_eq!(cf, 0);
    }
}

// ===========================================================================
// 2. LZ4F_getBlockSize
// ===========================================================================

#[test]
fn get_block_size_all_ids() {
    let a = api();
    unsafe {
        for id in -8i32..=16i32 {
            let c = (a.get_block_size.0)(id);
            let r = (a.get_block_size.1)(id);
            assert_eq!(ret_str(c), ret_str(r), "LZ4F_getBlockSize({})", id);
        }
        for id in [i32::MIN, i32::MIN + 1, -1, 0, 4, 5, 6, 7, 8, i32::MAX] {
            let c = (a.get_block_size.0)(id);
            let r = (a.get_block_size.1)(id);
            assert_eq!(ret_str(c), ret_str(r), "LZ4F_getBlockSize({})", id);
        }
        // sanity: the documented values
        assert_eq!((a.get_block_size.0)(0), 64 * 1024);
        assert_eq!((a.get_block_size.0)(4), 64 * 1024);
        assert_eq!((a.get_block_size.0)(7), 4 * 1024 * 1024);
        assert_eq!(
            lz4f_error_code((a.get_block_size.0)(8)),
            err::ERROR_maxBlockSize_invalid
        );
    }
}

// ===========================================================================
// 3. LZ4F_headerSize
// ===========================================================================

/// All header shapes: the header length varies with contentSize / dictID.
fn all_header_option_combos() -> Vec<LZ4F_preferences_t> {
    let mut v = Vec::new();
    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &cc in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
                for &bc in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                    for &csz in &[0u64, 1234u64] {
                        for &did in &[0u32, 0xDEAD_BEEFu32] {
                            v.push(prefs_of(bsid, bmode, cc, bc, csz, did, 0, 0));
                        }
                    }
                }
            }
        }
    }
    v
}

#[test]
fn header_size_real_headers_and_truncations() {
    let a = api();
    unsafe {
        // NULL src => LZ4F_ERROR_srcPtr_wrong, whatever srcSize is
        for s in [0usize, 1, 4, 5, 7, 19, 1000] {
            let c = (a.header_size.0)(std::ptr::null(), s);
            let r = (a.header_size.1)(std::ptr::null(), s);
            assert_eq!(ret_str(c), ret_str(r), "LZ4F_headerSize(NULL, {})", s);
            assert_eq!(lz4f_error_code(c), err::ERROR_srcPtr_wrong);
        }

        let mut cctx: *mut c_void = std::ptr::null_mut();
        assert!(!lz4f_is_error((a.c_create_cctx)(&mut cctx, LZ4F_VERSION)));

        for (i, prefs) in all_header_option_combos().iter().enumerate() {
            let mut hdr = vec![0u8; 64];
            let n = (a.c_begin)(
                cctx,
                hdr.as_mut_ptr() as *mut c_void,
                LZ4F_HEADER_SIZE_MAX,
                prefs as *const LZ4F_preferences_t,
            );
            assert!(!lz4f_is_error(n), "compressBegin failed: {}", ret_str(n));
            assert!((7..=19).contains(&n), "unexpected header length {}", n);

            // truncated header of EVERY length 0..=19
            for s in 0..=LZ4F_HEADER_SIZE_MAX {
                let c = (a.header_size.0)(hdr.as_ptr() as *const c_void, s);
                let r = (a.header_size.1)(hdr.as_ptr() as *const c_void, s);
                assert_eq!(
                    ret_str(c),
                    ret_str(r),
                    "LZ4F_headerSize(combo #{} realLen={}, srcSize={})",
                    i,
                    n,
                    s
                );
                if s >= MIN_SIZE_TO_KNOW_HEADER_LENGTH {
                    assert_eq!(c, n, "headerSize must equal the real header length");
                } else {
                    assert_eq!(lz4f_error_code(c), err::ERROR_frameHeader_incomplete);
                }
            }
            let c = (a.header_size.0)(hdr.as_ptr() as *const c_void, usize::MAX);
            let r = (a.header_size.1)(hdr.as_ptr() as *const c_void, usize::MAX);
            assert_eq!(ret_str(c), ret_str(r), "LZ4F_headerSize(SIZE_MAX)");
        }
        (a.c_free_cctx)(cctx);

        // skippable magics: the whole valid range plus both neighbourhoods
        for magic in 0x184D_2A4Eu32..=0x184D_2A62u32 {
            let f = skippable_frame(magic, &[0u8; 24]);
            for &s in &[5usize, 6, 7, 8, 19, 32] {
                let c = (a.header_size.0)(f.as_ptr() as *const c_void, s);
                let r = (a.header_size.1)(f.as_ptr() as *const c_void, s);
                assert_eq!(
                    ret_str(c),
                    ret_str(r),
                    "LZ4F_headerSize(magic {:#x}, srcSize={})",
                    magic,
                    s
                );
                if (0x184D_2A50..=0x184D_2A5F).contains(&magic) {
                    assert_eq!(c, 8, "skippable header size must be 8 ({:#x})", magic);
                } else {
                    assert_eq!(
                        lz4f_error_code(c),
                        err::ERROR_frameType_unknown,
                        "{:#x} must not be treated as skippable",
                        magic
                    );
                }
            }
        }

        // random garbage
        let mut rng = Rng::new(0x0BAD_C0DE);
        for _ in 0..3000 {
            let buf = gen_random(&mut rng, 24);
            let s = rng.range(0, 24);
            let c = (a.header_size.0)(buf.as_ptr() as *const c_void, s);
            let r = (a.header_size.1)(buf.as_ptr() as *const c_void, s);
            if c != r {
                panic!(
                    "LZ4F_headerSize(random, {}) C={} Rust={} bytes={}",
                    s,
                    ret_str(c),
                    ret_str(r),
                    hexdump(&buf, 24)
                );
            }
        }
    }
}

// ===========================================================================
// 4. LZ4F_getFrameInfo before any decompression
// ===========================================================================

#[test]
fn get_frame_info_before_decompress() {
    let a = api();
    let mut rng = Rng::new(0x11FE_0001);
    unsafe {
        for (i, prefs) in all_header_option_combos().iter().enumerate() {
            let mut p = *prefs;
            let data = gen_shape(&mut rng, i, 300);
            if p.frameInfo.contentSize != 0 {
                p.frameInfo.contentSize = data.len() as u64;
            }
            let frame = build_frame(&FrameSpec::new(p), &data);

            let sizes: [usize; 14] = [
                0,
                1,
                2,
                4,
                5,
                6,
                7,
                8,
                11,
                14,
                15,
                18,
                19,
                frame.len(),
            ];
            for &s0 in sizes.iter() {
                let s = s0.min(frame.len());
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                assert_eq!(
                    (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                    (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                );

                let mut c_fi = LZ4F_frameInfo_t::default();
                let mut r_fi = LZ4F_frameInfo_t::default();
                poison_fi(&mut c_fi);
                poison_fi(&mut r_fi);
                let mut cs = s;
                let mut rs = s;
                let c =
                    (a.get_frame_info.0)(cd, &mut c_fi, frame.as_ptr() as *const c_void, &mut cs);
                let r =
                    (a.get_frame_info.1)(rd, &mut r_fi, frame.as_ptr() as *const c_void, &mut rs);
                let ctx = format!("getFrameInfo(combo #{}, srcSize={})", i, s);
                assert_eq!(ret_str(c), ret_str(r), "{}: return", ctx);
                assert_eq!(cs, rs, "{}: *srcSizePtr", ctx);
                assert_bytes_eq(
                    &format!("{}: frameInfo", ctx),
                    fi_bytes(&c_fi),
                    fi_bytes(&r_fi),
                );
                assert_eq!(
                    (a.free_dctx.0)(cd),
                    (a.free_dctx.1)(rd),
                    "{}: dStage after getFrameInfo",
                    ctx
                );
            }
        }

        // skippable frames and non-frames
        for magic in [
            0x184D_2A50u32,
            0x184D_2A57u32,
            0x184D_2A5Fu32,
            0x184D_2A60u32,
            0u32,
            0x184D_2204u32,
        ] {
            let frame = skippable_frame(magic, &[7u8; 40]);
            for &s in &[0usize, 4, 5, 7, 8, 19, 48] {
                let mut cd: *mut c_void = std::ptr::null_mut();
                let mut rd: *mut c_void = std::ptr::null_mut();
                assert_eq!(
                    (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                    (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                );
                let mut c_fi = LZ4F_frameInfo_t::default();
                let mut r_fi = LZ4F_frameInfo_t::default();
                poison_fi(&mut c_fi);
                poison_fi(&mut r_fi);
                let mut cs = s;
                let mut rs = s;
                let c =
                    (a.get_frame_info.0)(cd, &mut c_fi, frame.as_ptr() as *const c_void, &mut cs);
                let r =
                    (a.get_frame_info.1)(rd, &mut r_fi, frame.as_ptr() as *const c_void, &mut rs);
                let ctx = format!("getFrameInfo(magic {:#x}, srcSize={})", magic, s);
                assert_eq!(ret_str(c), ret_str(r), "{}: return", ctx);
                assert_eq!(cs, rs, "{}: *srcSizePtr", ctx);
                assert_bytes_eq(
                    &format!("{}: frameInfo", ctx),
                    fi_bytes(&c_fi),
                    fi_bytes(&r_fi),
                );
                assert_eq!((a.free_dctx.0)(cd), (a.free_dctx.1)(rd), "{}: dStage", ctx);
            }
        }
    }
}

// ===========================================================================
// 5. LZ4F_getFrameInfo mid-stream
// ===========================================================================

/// Continue decoding into already-partially-filled output buffers.
#[allow(clippy::too_many_arguments)]
fn drive_tail(
    cd: *mut c_void,
    rd: *mut c_void,
    rest: &[u8],
    out_c: &mut [u8],
    out_r: &mut [u8],
    mut produced: usize,
    label: &str,
) -> Vec<u8> {
    let a = api();
    let mut consumed = 0usize;
    unsafe {
        loop {
            let src_len = rest.len() - consumed;
            let room = out_c.len() - produced;
            assert!(room > 0, "{}: tail out of room", label);
            let mut cs = src_len;
            let mut rs = src_len;
            let mut cdst = room;
            let mut rdst = room;
            let cret = (a.decompress.0)(
                cd,
                out_c.as_mut_ptr().add(produced) as *mut c_void,
                &mut cdst,
                rest.as_ptr().add(consumed) as *const c_void,
                &mut cs,
                std::ptr::null(),
            );
            let rret = (a.decompress.1)(
                rd,
                out_r.as_mut_ptr().add(produced) as *mut c_void,
                &mut rdst,
                rest.as_ptr().add(consumed) as *const c_void,
                &mut rs,
                std::ptr::null(),
            );
            assert_eq!(ret_str(cret), ret_str(rret), "{}: tail hint", label);
            assert_eq!(cs, rs, "{}: tail srcSize", label);
            assert_eq!(cdst, rdst, "{}: tail dstSize", label);
            assert_bytes_eq(&format!("{}: tail dst", label), &out_c[..], &out_r[..]);
            assert!(
                !lz4f_is_error(cret),
                "{}: tail error {}",
                label,
                ret_str(cret)
            );
            consumed += cs;
            produced += cdst;
            if cs == 0 && cdst == 0 {
                break;
            }
            if cret == 0 && consumed == rest.len() {
                break;
            }
        }
    }
    out_c[..produced].to_vec()
}

#[test]
fn get_frame_info_midstream() {
    let a = api();
    let mut rng = Rng::new(0x11FE_0002);

    let rows: &[(c_int, c_int, c_int, c_int, bool, c_uint)] = &[
        (LZ4F_max64KB, LZ4F_blockLinked, 0, 0, false, 0),
        (LZ4F_max64KB, LZ4F_blockLinked, 1, 1, true, 0x0102_0304),
        (LZ4F_max64KB, LZ4F_blockIndependent, 1, 0, true, 0),
        (LZ4F_max256KB, LZ4F_blockLinked, 0, 1, false, 42),
    ];

    unsafe {
        for (ri, &(bsid, bmode, cc, bc, csz, did)) in rows.iter().enumerate() {
            for shape in 0..N_SHAPES {
                let data = gen_shape(&mut rng, shape, 20_000);
                let prefs = prefs_of(
                    bsid,
                    bmode,
                    cc,
                    bc,
                    if csz { data.len() as u64 } else { 0 },
                    did,
                    0,
                    0,
                );
                let mut spec = FrameSpec::new(prefs);
                spec.update_chunk = 4096;
                let frame = build_frame(&spec, &data);

                // ---- (a) getFrameInfo while the header is only half-read ----
                {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(
                        (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                        (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                    );
                    let mut o_c = vec![SENTINEL; 4096];
                    let mut o_r = vec![SENTINEL; 4096];
                    let mut cs = 3usize;
                    let mut rs = 3usize;
                    let mut cdst = o_c.len();
                    let mut rdst = o_r.len();
                    let cret = (a.decompress.0)(
                        cd,
                        o_c.as_mut_ptr() as *mut c_void,
                        &mut cdst,
                        frame.as_ptr() as *const c_void,
                        &mut cs,
                        std::ptr::null(),
                    );
                    let rret = (a.decompress.1)(
                        rd,
                        o_r.as_mut_ptr() as *mut c_void,
                        &mut rdst,
                        frame.as_ptr() as *const c_void,
                        &mut rs,
                        std::ptr::null(),
                    );
                    assert_eq!(ret_str(cret), ret_str(rret), "3-byte prefix hint");
                    assert_eq!((cs, cdst), (rs, rdst), "3-byte prefix out-params");
                    assert_bytes_eq("3-byte prefix dst", &o_c, &o_r);

                    let mut c_fi = LZ4F_frameInfo_t::default();
                    let mut r_fi = LZ4F_frameInfo_t::default();
                    poison_fi(&mut c_fi);
                    poison_fi(&mut r_fi);
                    let mut cs2 = frame.len();
                    let mut rs2 = frame.len();
                    let c = (a.get_frame_info.0)(
                        cd,
                        &mut c_fi,
                        frame.as_ptr() as *const c_void,
                        &mut cs2,
                    );
                    let r = (a.get_frame_info.1)(
                        rd,
                        &mut r_fi,
                        frame.as_ptr() as *const c_void,
                        &mut rs2,
                    );
                    let ctx = format!(
                        "row {} shape {}: getFrameInfo in dstage_storeFrameHeader",
                        ri, shape
                    );
                    assert_eq!(ret_str(c), ret_str(r), "{}: return", ctx);
                    assert_eq!(
                        lz4f_error_code(c),
                        err::ERROR_frameDecoding_alreadyStarted,
                        "{}: expected frameDecoding_alreadyStarted",
                        ctx
                    );
                    assert_eq!(cs2, rs2, "{}: *srcSizePtr", ctx);
                    assert_bytes_eq(
                        &format!("{}: frameInfo", ctx),
                        fi_bytes(&c_fi),
                        fi_bytes(&r_fi),
                    );
                    assert_eq!((a.free_dctx.0)(cd), (a.free_dctx.1)(rd), "{}: dStage", ctx);
                }

                // ---- (b) getFrameInfo after partial decompression ----------
                // NOTE: on this path LZ4F_getFrameInfo re-enters
                // LZ4F_decompress(dctx, NULL, &0, NULL, &0, NULL) and can
                // legitimately advance dStage. Both libraries must agree.
                for &prefix in &[19usize, 40, 1500, 9000] {
                    let prefix = prefix.min(frame.len());
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(
                        (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                        (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                    );

                    let out_len = data.len() + 64;
                    let mut o_c = vec![SENTINEL; out_len];
                    let mut o_r = vec![SENTINEL; out_len];
                    let mut cs = prefix;
                    let mut rs = prefix;
                    let mut cdst = out_len;
                    let mut rdst = out_len;
                    let cret = (a.decompress.0)(
                        cd,
                        o_c.as_mut_ptr() as *mut c_void,
                        &mut cdst,
                        frame.as_ptr() as *const c_void,
                        &mut cs,
                        std::ptr::null(),
                    );
                    let rret = (a.decompress.1)(
                        rd,
                        o_r.as_mut_ptr() as *mut c_void,
                        &mut rdst,
                        frame.as_ptr() as *const c_void,
                        &mut rs,
                        std::ptr::null(),
                    );
                    let ctx = format!("row {} shape {} prefix {}", ri, shape, prefix);
                    assert_eq!(ret_str(cret), ret_str(rret), "{}: partial hint", ctx);
                    assert_eq!(cs, rs, "{}: partial srcSize", ctx);
                    assert_eq!(cdst, rdst, "{}: partial dstSize", ctx);
                    assert_bytes_eq(&format!("{}: partial dst", ctx), &o_c, &o_r);

                    let mut c_fi = LZ4F_frameInfo_t::default();
                    let mut r_fi = LZ4F_frameInfo_t::default();
                    poison_fi(&mut c_fi);
                    poison_fi(&mut r_fi);
                    let mut cs2 = 12345usize;
                    let mut rs2 = 12345usize;
                    let c = (a.get_frame_info.0)(
                        cd,
                        &mut c_fi,
                        frame.as_ptr() as *const c_void,
                        &mut cs2,
                    );
                    let r = (a.get_frame_info.1)(
                        rd,
                        &mut r_fi,
                        frame.as_ptr() as *const c_void,
                        &mut rs2,
                    );
                    assert_eq!(ret_str(c), ret_str(r), "{}: midstream getFrameInfo", ctx);
                    assert_eq!(cs2, rs2, "{}: midstream *srcSizePtr", ctx);
                    assert_bytes_eq(
                        &format!("{}: midstream frameInfo", ctx),
                        fi_bytes(&c_fi),
                        fi_bytes(&r_fi),
                    );
                    assert_eq!(
                        c_fi.blockSizeID,
                        if bsid == 0 { LZ4F_max64KB } else { bsid },
                        "{}: blockSizeID",
                        ctx
                    );
                    assert_eq!(c_fi.blockMode, bmode, "{}: blockMode", ctx);
                    assert_eq!(c_fi.contentChecksumFlag, cc, "{}: contentChecksumFlag", ctx);
                    assert_eq!(c_fi.blockChecksumFlag, bc, "{}: blockChecksumFlag", ctx);
                    assert_eq!(c_fi.dictID, did, "{}: dictID", ctx);

                    let consumed = cs;
                    let produced = cdst;
                    let rest = drive_tail(
                        cd,
                        rd,
                        &frame[consumed..],
                        &mut o_c[..],
                        &mut o_r[..],
                        produced,
                        &ctx,
                    );
                    assert_bytes_eq(
                        &format!("{}: content after midstream getFrameInfo", ctx),
                        &data,
                        &rest,
                    );
                    assert_eq!((a.free_dctx.0)(cd), (a.free_dctx.1)(rd), "{}: dStage", ctx);
                }
            }
        }
    }
}

// ===========================================================================
// 6. Full frame-option matrix decoded in one shot / following the hint
// ===========================================================================

#[test]
fn decompress_one_shot_option_matrix() {
    let mut rng = Rng::new(0x0071_0001);
    let mut n = 0usize;
    for &bsid in &[0, 4, 5, 6, 7] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &cc in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
                for &bc in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                    for &with_csz in &[false, true] {
                        for &did in &[0u32, 0xC0FF_EE00u32] {
                            for &af in &[0u32, 1u32] {
                              for &len in &[0usize, 1, 2, 17, 700, 5000, 9001] {
                                let shape = (n + len) % N_SHAPES;
                                let data = gen_shape(&mut rng, shape, len);
                                let prefs = prefs_of(
                                    bsid,
                                    bmode,
                                    cc,
                                    bc,
                                    if with_csz { data.len() as u64 } else { 0 },
                                    did,
                                    0,
                                    af,
                                );
                                let mut spec = FrameSpec::new(prefs);
                                spec.update_chunk = if n % 3 == 0 { usize::MAX } else { 333 };
                                let label = format!(
                                    "matrix bsid={} bmode={} cc={} bc={} csz={} did={:#x} af={} shape={} len={}",
                                    bsid, bmode, cc, bc, with_csz, did, af, shape_name(shape), len
                                );
                                roundtrip(
                                    &spec,
                                    &data,
                                    &DecodeCfg::new(SrcPlan::OneShot, DstPlan::All),
                                    &label,
                                );
                                roundtrip(
                                    &spec,
                                    &data,
                                    &DecodeCfg::new(SrcPlan::Hint, DstPlan::All),
                                    &format!("{} [hint]", label),
                                );
                                roundtrip(
                                    &spec,
                                    &data,
                                    &DecodeCfg::new(SrcPlan::Fixed(1), DstPlan::Fixed(1)),
                                    &format!("{} [1/1]", label),
                                );
                                roundtrip(
                                    &spec,
                                    &data,
                                    &DecodeCfg::new(SrcPlan::Fixed(3), DstPlan::Fixed(7)),
                                    &format!("{} [3/7]", label),
                                );
                                roundtrip(
                                    &spec,
                                    &data,
                                    &DecodeCfg::new(SrcPlan::Fixed(4), DstPlan::Fixed(4)),
                                    &format!("{} [4/4]", label),
                                );
                              }
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 5 * 2 * 2 * 2 * 2 * 2 * 2);
}

// ===========================================================================
// 7. src chunk granularity (the dStage store* paths)
// ===========================================================================

#[test]
fn decompress_src_chunk_granularity() {
    let mut rng = Rng::new(0xC401_0001);
    let rows: &[(c_int, c_int, c_int, bool)] = &[
        (LZ4F_blockLinked, 0, 0, false),
        (LZ4F_blockLinked, 1, 1, true),
        (LZ4F_blockIndependent, 1, 0, false),
        (LZ4F_blockIndependent, 0, 1, true),
    ];
    // Small inputs only: chunk == 1 means one LZ4F_decompress call per byte.
    for &len in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 13, 300, 2500, 9000] {
        for shape in 0..N_SHAPES {
            let data = gen_shape(&mut rng, shape, len);
            for &(bmode, cc, bc, csz) in rows {
              for &bsid in &[LZ4F_max64KB, LZ4F_max256KB] {
                let prefs = prefs_of(
                    bsid,
                    bmode,
                    cc,
                    bc,
                    if csz { data.len() as u64 } else { 0 },
                    7,
                    0,
                    1,
                );
                let spec = FrameSpec::new(prefs);
                for &chunk in &[1usize, 2, 3, 4, 5, 6, 7, 8, 11, 16, 64, 1000] {
                    let label = format!(
                        "chunkgran bsid={} bmode={} cc={} bc={} csz={} shape={} len={} srcChunk={}",
                        bsid,
                        bmode,
                        cc,
                        bc,
                        csz,
                        shape_name(shape),
                        len,
                        chunk
                    );
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::Fixed(chunk), DstPlan::All),
                        &label,
                    );
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::Fixed(chunk), DstPlan::Fixed(chunk)),
                        &format!("{} [dst=src]", label),
                    );
                }
              }
            }
        }
    }
}

// ===========================================================================
// 8. dst capacity granularity (tmpOut / partial flushOut)
// ===========================================================================

#[test]
fn decompress_dst_capacity_granularity() {
    let mut rng = Rng::new(0xD57C_0001);
    for &bsid in &[LZ4F_max64KB, LZ4F_max256KB] {
        let bs = block_size_of(bsid);
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for shape in 0..N_SHAPES {
                for &len in &[1usize, 100, bs - 1, bs, bs + 1, 2 * bs + 37] {
                    let data = gen_shape(&mut rng, shape, len);
                    let prefs = prefs_of(bsid, bmode, 1, 1, data.len() as u64, 3, 0, 1);
                    let spec = FrameSpec::new(prefs);
                    for &cap in &[1usize, 2, 7, 100, 4095, bs - 1, bs, bs + 1, usize::MAX] {
                        // one output byte at a time on a >100 KB frame is too slow
                        if cap < 8 && len > 5000 {
                            continue;
                        }
                        if cap < 4096 && len > 300_000 {
                            continue;
                        }
                        let label = format!(
                            "dstgran bsid={} bmode={} shape={} len={} dstCap={}",
                            bsid,
                            bmode,
                            shape_name(shape),
                            len,
                            cap
                        );
                        let plan = if cap == usize::MAX {
                            DstPlan::All
                        } else {
                            DstPlan::Fixed(cap)
                        };
                        roundtrip(&spec, &data, &DecodeCfg::new(SrcPlan::OneShot, plan), &label);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 9. src x dst cross product
// ===========================================================================

#[test]
fn decompress_src_dst_cross_product() {
    let mut rng = Rng::new(0xC205_0001);
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 2, 5, 13, 130, 1500, 4000, 20_000] {
            let data = gen_shape(&mut rng, shape, len);
            for &(bmode, cc, bc) in &[
                (LZ4F_blockLinked, 0, 0),
                (LZ4F_blockLinked, 1, 1),
                (LZ4F_blockIndependent, 1, 1),
            ] {
                let prefs = prefs_of(LZ4F_max64KB, bmode, cc, bc, 0, 0, 0, 1);
                let spec = FrameSpec::new(prefs);
                for &sc in &[1usize, 2, 3, 4, 5, 17, 1000, usize::MAX] {
                    for &dc in &[1usize, 2, 3, 4, 17, 1000, 70_000, usize::MAX] {
                        if (sc < 6 || dc < 6) && len > 4000 {
                            continue; // byte-at-a-time on 20 KB x 64 rows is wasteful
                        }
                        let sp = if sc == usize::MAX {
                            SrcPlan::OneShot
                        } else {
                            SrcPlan::Fixed(sc)
                        };
                        let dp = if dc == usize::MAX {
                            DstPlan::All
                        } else {
                            DstPlan::Fixed(dc)
                        };
                        let label = format!(
                            "cross shape={} len={} bmode={} cc={} bc={} src={} dst={}",
                            shape_name(shape),
                            len,
                            bmode,
                            cc,
                            bc,
                            sc,
                            dc
                        );
                        roundtrip(&spec, &data, &DecodeCfg::new(sp, dp), &label);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 10. random (seeded) chunking
// ===========================================================================

#[test]
fn decompress_random_chunking() {
    let mut rng = Rng::new(0x2A4D_0001);
    for iter in 0..2500u64 {
        let shape = (iter as usize) % N_SHAPES;
        let len = rng.range(0, 30_000);
        let data = gen_shape(&mut rng, shape, len);
        let bsid = [0, 4, 5][(iter as usize) % 3];
        let bmode = if rng.bool() {
            LZ4F_blockLinked
        } else {
            LZ4F_blockIndependent
        };
        let cc = rng.bool() as c_int;
        let bc = rng.bool() as c_int;
        let with_csz = rng.bool();
        let af = rng.bool() as c_uint;
        let prefs = prefs_of(
            bsid,
            bmode,
            cc,
            bc,
            if with_csz { data.len() as u64 } else { 0 },
            rng.next_u32(),
            0,
            af,
        );
        let mut spec = FrameSpec::new(prefs);
        spec.update_chunk = rng.range(1, 40_000);

        let seed = rng.next_u64();
        let lo = rng.range(1, 8);
        let span = rng.range(1, 5000);
        let hi = rng.range(lo, lo + span);
        let dcap = rng.range(1, 70_000);
        let label = format!(
            "rand iter={} shape={} len={} bsid={} bmode={} cc={} bc={} af={} chunk=[{},{}] dstCap={}",
            iter, shape_name(shape), len, bsid, bmode, cc, bc, af, lo, hi, dcap
        );
        roundtrip(
            &spec,
            &data,
            &DecodeCfg::new(SrcPlan::Random(seed, lo, hi), DstPlan::Fixed(dcap)),
            &label,
        );
    }
}

// ===========================================================================
// 11. LZ4F_decompressOptions_t axes + NULL options + NULL dst
// ===========================================================================

#[test]
fn decompress_options_axes() {
    let a = api();
    let mut rng = Rng::new(0x0975_0001);

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 900, 40_000] {
            let data = gen_shape(&mut rng, shape, len);
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                for &cc in &[0, 1] {
                    for &bc in &[0, 1] {
                        let prefs =
                            prefs_of(LZ4F_max64KB, bmode, cc, bc, data.len() as u64, 9, 0, 0);
                        let mut spec = FrameSpec::new(prefs);
                        spec.update_chunk = 5000;
                        for &stable in &[0u32, 1u32] {
                            for &skip in &[0u32, 1u32] {
                                for &(sp, dp) in &[
                                    (SrcPlan::OneShot, DstPlan::All),
                                    (SrcPlan::Fixed(1), DstPlan::Fixed(1)),
                                    (SrcPlan::Fixed(3), DstPlan::Fixed(29)),
                                    (SrcPlan::Fixed(700), DstPlan::Fixed(4096)),
                                    (SrcPlan::Hint, DstPlan::Fixed(333)),
                                ] {
                                    if len > 5000 && matches!(sp, SrcPlan::Fixed(1)) {
                                        continue;
                                    }
                                    let mut cfg = DecodeCfg::new(sp, dp);
                                    cfg.opts = Some(dopts(stable, skip));
                                    let label = format!(
                                        "opts shape={} len={} bmode={} cc={} bc={} stableDst={} skipChecksums={} src={:?} dst={:?}",
                                        shape_name(shape), len, bmode, cc, bc, stable, skip, sp, dp
                                    );
                                    roundtrip(&spec, &data, &cfg, &label);
                                }
                            }
                        }
                        // NULL options pointer
                        roundtrip(
                            &spec,
                            &data,
                            &DecodeCfg::new(SrcPlan::Fixed(11), DstPlan::Fixed(97)),
                            &format!(
                                "opts NULL shape={} len={} bmode={} cc={} bc={}",
                                shape_name(shape),
                                len,
                                bmode,
                                cc,
                                bc
                            ),
                        );
                    }
                }
            }
        }
    }

    // Explicit dstBuffer == NULL / dstCapacity == 0 probes (what
    // LZ4F_getFrameInfo does internally).
    let data = gen_shape(&mut rng, 4, 30_000);
    let prefs = prefs_of(LZ4F_max64KB, LZ4F_blockLinked, 1, 1, 0, 0, 0, 0);
    let frame = build_frame(&FrameSpec::new(prefs), &data);
    unsafe {
        for &dst_null in &[true, false] {
            let mut cd: *mut c_void = std::ptr::null_mut();
            let mut rd: *mut c_void = std::ptr::null_mut();
            assert_eq!(
                (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
            );
            let mut o_c = vec![SENTINEL; 64];
            let mut o_r = vec![SENTINEL; 64];
            let mut consumed = 0usize;
            for step in 0..60 {
                let src_len = (frame.len() - consumed).min(700);
                let mut cs = src_len;
                let mut rs = src_len;
                let mut cdst = 0usize;
                let mut rdst = 0usize;
                let (cp, rp) = if dst_null {
                    (std::ptr::null_mut(), std::ptr::null_mut())
                } else {
                    (
                        o_c.as_mut_ptr() as *mut c_void,
                        o_r.as_mut_ptr() as *mut c_void,
                    )
                };
                let cret = (a.decompress.0)(
                    cd,
                    cp,
                    &mut cdst,
                    frame.as_ptr().add(consumed) as *const c_void,
                    &mut cs,
                    std::ptr::null(),
                );
                let rret = (a.decompress.1)(
                    rd,
                    rp,
                    &mut rdst,
                    frame.as_ptr().add(consumed) as *const c_void,
                    &mut rs,
                    std::ptr::null(),
                );
                let ctx = format!("nulldst(dst_null={}) step {}", dst_null, step);
                assert_eq!(ret_str(cret), ret_str(rret), "{}: hint", ctx);
                assert_eq!(cs, rs, "{}: srcSize", ctx);
                assert_eq!(cdst, rdst, "{}: dstSize", ctx);
                assert_bytes_eq(&format!("{}: dst", ctx), &o_c, &o_r);
                if lz4f_is_error(cret) || (cs == 0 && cdst == 0) {
                    break;
                }
                consumed += cs;
                if consumed == frame.len() && cret == 0 {
                    break;
                }
            }
            assert_eq!(
                (a.free_dctx.0)(cd),
                (a.free_dctx.1)(rd),
                "nulldst dStage (dst_null={})",
                dst_null
            );
        }
    }
}

// ===========================================================================
// 12. Large multi-block frames (direct-into-dst decode path)
// ===========================================================================

#[test]
fn decompress_large_multiblock_frames() {
    let mut rng = Rng::new(0x1A26_0001);

    let rows: &[(c_int, usize)] = &[
        (LZ4F_max64KB, 65_535),
        (LZ4F_max64KB, 65_536),
        (LZ4F_max64KB, 65_537),
        (LZ4F_max64KB, 200_000),
        (LZ4F_max256KB, 262_144),
        (LZ4F_max256KB, 700_000),
        (LZ4F_max1MB, 1_048_577),
        (LZ4F_max4MB, 4_194_305),
    ];

    for &(bsid, len) in rows {
        let shapes: &[usize] = if len > 1_000_000 {
            &[1, 5]
        } else {
            &[0, 1, 4, 5]
        };
        for &shape in shapes {
            let data = gen_shape(&mut rng, shape, len);
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                let prefs = prefs_of(bsid, bmode, 1, 1, data.len() as u64, 0x77, 0, 1);
                let spec = FrameSpec::new(prefs);
                let label = format!(
                    "large bsid={} len={} shape={} bmode={}",
                    bsid,
                    len,
                    shape_name(shape),
                    bmode
                );
                // dst large enough for the direct-into-destination decode path
                roundtrip(
                    &spec,
                    &data,
                    &DecodeCfg::new(SrcPlan::OneShot, DstPlan::All),
                    &format!("{} [oneshot/all]", label),
                );
                // follow the hint (the intended streaming usage)
                roundtrip(
                    &spec,
                    &data,
                    &DecodeCfg::new(SrcPlan::Hint, DstPlan::All),
                    &format!("{} [hint/all]", label),
                );
                // coarse src chunks with a dst smaller than the block => tmpOut
                roundtrip(
                    &spec,
                    &data,
                    &DecodeCfg::new(SrcPlan::Fixed(4096), DstPlan::Fixed(4096)),
                    &format!("{} [4096/4096]", label),
                );
                if len < 300_000 {
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::Fixed(64), DstPlan::Fixed(65_536)),
                        &format!("{} [64/64K]", label),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 13. compression levels: lz4 fast, lz4mid (1-2), hashChain (3-9), optimal (10-12)
// ===========================================================================

#[test]
fn decompress_all_compression_levels() {
    let mut rng = Rng::new(0x1E5E_0001);
    for &level in &[-9i32, -5, -1, 0, 1, 2, 3, 4, 6, 8, 9, 10, 11, 12] {
        for shape in 0..N_SHAPES {
            for &len in &[1usize, 900, 70_000] {
                let data = gen_shape(&mut rng, shape, len);
                for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                    let prefs = prefs_of(
                        LZ4F_max64KB,
                        bmode,
                        1,
                        1,
                        data.len() as u64,
                        0,
                        level,
                        (len % 2) as c_uint,
                    );
                    let mut spec = FrameSpec::new(prefs);
                    spec.update_chunk = 30_000;
                    let label = format!(
                        "level={} shape={} len={} bmode={}",
                        level,
                        shape_name(shape),
                        len,
                        bmode
                    );
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::OneShot, DstPlan::All),
                        &label,
                    );
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::Fixed(37), DstPlan::Fixed(211)),
                        &format!("{} [37/211]", label),
                    );
                }
            }
        }
    }

    // favorDecSpeed only matters for the HC levels; sweep it too.
    for &level in &[10i32, 11, 12] {
        for &fav in &[0u32, 1u32] {
            let data = gen_shape(&mut rng, 5, 120_000);
            let mut prefs = prefs_of(LZ4F_max64KB, LZ4F_blockLinked, 1, 0, 0, 0, level, 1);
            prefs.favorDecSpeed = fav;
            let spec = FrameSpec::new(prefs);
            roundtrip(
                &spec,
                &data,
                &DecodeCfg::new(SrcPlan::Hint, DstPlan::All),
                &format!("favorDecSpeed level={} fav={}", level, fav),
            );
        }
    }
}

// ===========================================================================
// 14. Dictionaries: LZ4F_decompress_usingDict
// ===========================================================================

#[test]
fn decompress_using_dict_axes() {
    let mut rng = Rng::new(0xD1C7_0001);
    let dict_sizes = [0usize, 1, 100, 4096, 65_535, 65_536, 70_000];

    for &dsz in &dict_sizes {
        let dbuf = gen_selfref(&mut rng, dsz.max(1));
        let dict = &dbuf[..dsz];

        for &kind in &[DictKind::Raw, DictKind::CDict] {
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                for shape in 0..N_SHAPES {
                    // Content that partially repeats the dictionary tail, so the
                    // compressor really emits dictionary-relative matches.
                    let mut data = Vec::new();
                    if dsz > 0 {
                        let take = dsz.min(4000);
                        data.extend_from_slice(&dict[dsz - take..]);
                    }
                    data.extend_from_slice(&gen_shape(&mut rng, shape, 6000));
                    if dsz > 0 {
                        let take = dsz.min(2000);
                        data.extend_from_slice(&dict[dsz - take..]);
                    }

                  for &cc in &[0, 1] {
                  for &bc in &[0, 1] {
                    let prefs = prefs_of(
                        LZ4F_max64KB,
                        bmode,
                        cc,
                        bc,
                        data.len() as u64,
                        0xABCD,
                        if shape % 3 == 0 { 0 } else { 9 },
                        1,
                    );
                    let mut spec = FrameSpec::new(prefs);
                    spec.dict = Some(dict);
                    spec.dict_kind = kind;
                    spec.update_chunk = 3000;

                    let kn = if kind == DictKind::Raw { "raw" } else { "cdict" };
                    let label = format!(
                        "usingDict dsz={} kind={} bmode={} shape={} cc={} bc={}",
                        dsz,
                        kn,
                        bmode,
                        shape_name(shape),
                        cc,
                        bc
                    );
                    for &(sp, dp) in &[
                        (SrcPlan::OneShot, DstPlan::All),
                        (SrcPlan::Fixed(17), DstPlan::Fixed(64)),
                        (SrcPlan::Fixed(1000), DstPlan::Fixed(333)),
                        (SrcPlan::Fixed(3), DstPlan::All),
                        (SrcPlan::Hint, DstPlan::All),
                    ] {
                        let mut cfg = DecodeCfg::new(sp, dp);
                        cfg.dict = Some(dict);
                        roundtrip(
                            &spec,
                            &data,
                            &cfg,
                            &format!("{} src={:?} dst={:?}", label, sp, dp),
                        );
                    }
                    // stableDst=1 disables the "preserve history in tmpOut" tail.
                    let mut cfg = DecodeCfg::new(SrcPlan::Fixed(700), DstPlan::Fixed(4096));
                    cfg.dict = Some(dict);
                    cfg.opts = Some(dopts(1, 0));
                    roundtrip(&spec, &data, &cfg, &format!("{} [stableDst]", label));
                    // skipChecksums on the dictionary path
                    let mut cfg = DecodeCfg::new(SrcPlan::Fixed(64), DstPlan::Fixed(1024));
                    cfg.dict = Some(dict);
                    cfg.opts = Some(dopts(0, 1));
                    roundtrip(&spec, &data, &cfg, &format!("{} [skipChecksums]", label));
                  }
                  }
                }
            }
        }
    }

    // usingDict driven over a frame compressed WITHOUT a dictionary.
    let data = gen_shape(&mut rng, 3, 9000);
    let plain = FrameSpec::new(prefs_of(
        LZ4F_max64KB,
        LZ4F_blockLinked,
        1,
        1,
        data.len() as u64,
        0,
        0,
        1,
    ));
    let dbuf = gen_selfref(&mut rng, 40_000);
    for &dl in &[0usize, 1, 40_000] {
        let mut cfg = DecodeCfg::new(SrcPlan::Fixed(64), DstPlan::Fixed(1024));
        cfg.dict = Some(&dbuf[..dl]);
        roundtrip(
            &plain,
            &data,
            &cfg,
            &format!("usingDict on dict-less frame, dictSize={}", dl),
        );
    }
}

// ===========================================================================
// 15. Uncompressed (stored) blocks: dstage_copyDirect + dstage_getBlockChecksum
// ===========================================================================

#[test]
fn decompress_uncompressed_blocks() {
    let mut rng = Rng::new(0x0AC0_0001);
    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 2, 3, 4, 5, 999, 1000, 1001, 3000, 70_000] {
            let data = gen_shape(&mut rng, shape, len);
            for &bc in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                for &cc in &[0, 1] {
                    let prefs = prefs_of(
                        LZ4F_max64KB,
                        LZ4F_blockIndependent,
                        cc,
                        bc,
                        data.len() as u64,
                        0,
                        0,
                        1,
                    );
                    let mut spec = FrameSpec::new(prefs);
                    spec.uncompressed = true;
                    spec.update_chunk = 999;
                    let label = format!(
                        "uncompressed shape={} len={} bc={} cc={}",
                        shape_name(shape),
                        len,
                        bc,
                        cc
                    );
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::OneShot, DstPlan::All),
                        &format!("{} [oneshot]", label),
                    );
                    if len <= 3000 {
                        // byte-at-a-time forces the partial-copyDirect return and
                        // the dctx->header accumulation of the block checksum
                        roundtrip(
                            &spec,
                            &data,
                            &DecodeCfg::new(SrcPlan::Fixed(1), DstPlan::All),
                            &format!("{} [1/all]", label),
                        );
                        roundtrip(
                            &spec,
                            &data,
                            &DecodeCfg::new(SrcPlan::Fixed(2), DstPlan::Fixed(3)),
                            &format!("{} [2/3]", label),
                        );
                        roundtrip(
                            &spec,
                            &data,
                            &DecodeCfg::new(SrcPlan::Fixed(3), DstPlan::Fixed(1)),
                            &format!("{} [3/1]", label),
                        );
                    }
                    roundtrip(
                        &spec,
                        &data,
                        &DecodeCfg::new(SrcPlan::Fixed(700), DstPlan::Fixed(64)),
                        &format!("{} [700/64]", label),
                    );
                    let mut cfg = DecodeCfg::new(SrcPlan::Fixed(9), DstPlan::Fixed(17));
                    cfg.opts = Some(dopts(0, 1));
                    roundtrip(&spec, &data, &cfg, &format!("{} [skipChecksums]", label));
                }
            }
        }
    }

    // Random data at max64KB is incompressible, so the *compressor* also emits
    // stored blocks through the ordinary LZ4F_compressUpdate path.
    for &len in &[65_536usize, 130_000] {
        let data = gen_random(&mut rng, len);
        for &bc in &[0, 1] {
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                let prefs = prefs_of(LZ4F_max64KB, bmode, 1, bc, 0, 0, 0, 1);
                let spec = FrameSpec::new(prefs);
                roundtrip(
                    &spec,
                    &data,
                    &DecodeCfg::new(SrcPlan::Fixed(4096), DstPlan::Fixed(4096)),
                    &format!("stored-blocks len={} bc={} bmode={}", len, bc, bmode),
                );
                roundtrip(
                    &spec,
                    &data,
                    &DecodeCfg::new(SrcPlan::Hint, DstPlan::All),
                    &format!("stored-blocks len={} bc={} bmode={} [hint]", len, bc, bmode),
                );
            }
        }
    }
}

// ===========================================================================
// 16. Skippable frames
// ===========================================================================

#[test]
fn decompress_skippable_frames() {
    let mut rng = Rng::new(0x5C19_0001);

    // (a) a lone skippable frame across the whole valid magic range
    for magic in 0x184D_2A50u32..=0x184D_2A5Fu32 {
        for &plen in &[0usize, 1, 7, 8, 19, 100, 5000] {
            let payload = gen_random(&mut rng, plen);
            let frame = skippable_frame(magic, &payload);
            for &(sp, dp) in &[
                (SrcPlan::OneShot, DstPlan::All),
                (SrcPlan::Fixed(1), DstPlan::All),
                (SrcPlan::Fixed(2), DstPlan::All),
                (SrcPlan::Fixed(3), DstPlan::Fixed(1)),
                (SrcPlan::Fixed(4), DstPlan::All),
                (SrcPlan::Fixed(7), DstPlan::All),
                (SrcPlan::Hint, DstPlan::All),
            ] {
                let label = format!(
                    "skippable magic={:#x} payload={} src={:?} dst={:?}",
                    magic, plen, sp, dp
                );
                let res = drive(&frame, 0, &DecodeCfg::new(sp, dp), &label);
                assert!(
                    !lz4f_is_error(res.last_ret),
                    "{}: error {}",
                    label,
                    ret_str(res.last_ret)
                );
                assert_eq!(res.out.len(), 0, "{}: must emit no output", label);
                assert_eq!(res.consumed, frame.len(), "{}: must consume all", label);
                assert_eq!(res.last_ret, 0, "{}: hint should be 0", label);
            }
        }
    }

    // (b)/(c) skippable next to a real frame in the same buffer
    for shape in 0..N_SHAPES {
        let data = gen_shape(&mut rng, shape, 4000);
        for &(cc, bc, bmode) in &[
            (0, 0, LZ4F_blockLinked),
            (1, 1, LZ4F_blockLinked),
            (1, 0, LZ4F_blockIndependent),
        ] {
            let prefs = prefs_of(LZ4F_max64KB, bmode, cc, bc, data.len() as u64, 0, 0, 1);
            let real = build_frame(&FrameSpec::new(prefs), &data);

            for &plen in &[0usize, 3, 40, 1000] {
                let mut buf = skippable_frame(0x184D_2A55, &gen_random(&mut rng, plen));
                buf.extend_from_slice(&real);
                for &(sp, dp) in &[
                    (SrcPlan::OneShot, DstPlan::All),
                    (SrcPlan::Fixed(1), DstPlan::All),
                    (SrcPlan::Fixed(5), DstPlan::Fixed(7)),
                    (SrcPlan::Hint, DstPlan::All),
                ] {
                    let label = format!(
                        "skippable+real shape={} cc={} bc={} bmode={} payload={} src={:?} dst={:?}",
                        shape_name(shape), cc, bc, bmode, plen, sp, dp
                    );
                    let res = drive(&buf, data.len(), &DecodeCfg::new(sp, dp), &label);
                    assert!(
                        !lz4f_is_error(res.last_ret),
                        "{}: error {}",
                        label,
                        ret_str(res.last_ret)
                    );
                    assert_eq!(res.consumed, buf.len(), "{}: consumed", label);
                    assert_bytes_eq(&format!("{}: content", label), &data, &res.out);
                }
            }

            let mut buf = real.clone();
            buf.extend_from_slice(&skippable_frame(0x184D_2A5A, &gen_random(&mut rng, 33)));
            for &(sp, dp) in &[
                (SrcPlan::OneShot, DstPlan::All),
                (SrcPlan::Fixed(3), DstPlan::Fixed(9)),
            ] {
                let label = format!(
                    "real+skippable shape={} cc={} bc={} bmode={} src={:?} dst={:?}",
                    shape_name(shape),
                    cc,
                    bc,
                    bmode,
                    sp,
                    dp
                );
                let res = drive(&buf, data.len(), &DecodeCfg::new(sp, dp), &label);
                assert!(
                    !lz4f_is_error(res.last_ret),
                    "{}: {}",
                    label,
                    ret_str(res.last_ret)
                );
                assert_eq!(res.consumed, buf.len(), "{}: consumed", label);
                assert_bytes_eq(&format!("{}: content", label), &data, &res.out);
            }
        }
    }

    // (d) two skippable frames back to back, and a truncated skippable frame
    {
        let mut buf = skippable_frame(0x184D_2A50, &[1, 2, 3, 4, 5]);
        buf.extend_from_slice(&skippable_frame(0x184D_2A5F, &[9u8; 300]));
        for &sp in &[SrcPlan::OneShot, SrcPlan::Fixed(1), SrcPlan::Fixed(6)] {
            let label = format!("two-skippables src={:?}", sp);
            let res = drive(&buf, 0, &DecodeCfg::new(sp, DstPlan::All), &label);
            assert_eq!(res.consumed, buf.len(), "{}", label);
            assert_eq!(res.last_ret, 0, "{}", label);
        }
        let short = &buf[..buf.len() - 100];
        for &sp in &[SrcPlan::OneShot, SrcPlan::Fixed(1)] {
            let label = format!("truncated skippable src={:?}", sp);
            let res = drive(short, 0, &DecodeCfg::new(sp, DstPlan::All), &label);
            assert!(res.last_ret != 0, "{}: should still want more input", label);
        }
    }
}

// ===========================================================================
// 17. Multiple concatenated real frames
// ===========================================================================

#[test]
fn decompress_multi_frame() {
    let a = api();
    let mut rng = Rng::new(0x3F2A_0001);

    for shape in 0..N_SHAPES {
        let d1 = gen_shape(&mut rng, shape, 3000);
        let d2 = gen_shape(&mut rng, (shape + 1) % N_SHAPES, 7000);
        for &(bmode1, cc1, bc1) in &[
            (LZ4F_blockLinked, 0, 0),
            (LZ4F_blockLinked, 1, 1),
            (LZ4F_blockIndependent, 1, 0),
        ] {
            for &(bmode2, cc2, bc2) in &[
                (LZ4F_blockLinked, 1, 0),
                (LZ4F_blockIndependent, 0, 1),
            ] {
                let f1 = build_frame(
                    &FrameSpec::new(prefs_of(
                        LZ4F_max64KB,
                        bmode1,
                        cc1,
                        bc1,
                        d1.len() as u64,
                        1,
                        0,
                        1,
                    )),
                    &d1,
                );
                let f2 = build_frame(
                    &FrameSpec::new(prefs_of(LZ4F_max256KB, bmode2, cc2, bc2, 0, 0, 0, 1)),
                    &d2,
                );
                let mut cat = f1.clone();
                cat.extend_from_slice(&f2);
                let mut expect = d1.clone();
                expect.extend_from_slice(&d2);

                // ---- (a) same dctx, NO reset in between --------------------
                for &(sp, dp) in &[
                    (SrcPlan::OneShot, DstPlan::All),
                    (SrcPlan::Fixed(1), DstPlan::All),
                    (SrcPlan::Fixed(13), DstPlan::Fixed(29)),
                    (SrcPlan::Hint, DstPlan::All),
                ] {
                    let label = format!(
                        "multiframe(no reset) shape={} f1=({},{},{}) f2=({},{},{}) src={:?} dst={:?}",
                        shape_name(shape), bmode1, cc1, bc1, bmode2, cc2, bc2, sp, dp
                    );
                    let res = drive(&cat, expect.len(), &DecodeCfg::new(sp, dp), &label);
                    assert!(
                        !lz4f_is_error(res.last_ret),
                        "{}: {}",
                        label,
                        ret_str(res.last_ret)
                    );
                    assert_eq!(res.consumed, cat.len(), "{}: consumed", label);
                    assert_bytes_eq(&format!("{}: content", label), &expect, &res.out);
                }

                // ---- (b) same dctx WITH LZ4F_resetDecompressionContext -----
                unsafe {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(
                        (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                        (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                    );
                    let cfg = DecodeCfg::new(SrcPlan::Fixed(97), DstPlan::Fixed(512));
                    let r1 = drive_with(cd, rd, &f1, d1.len() + 64, &cfg, "multiframe reset f1");
                    assert_bytes_eq("multiframe reset f1 content", &d1, &r1.out);
                    (a.reset_dctx.0)(cd);
                    (a.reset_dctx.1)(rd);
                    let r2 = drive_with(cd, rd, &f2, d2.len() + 64, &cfg, "multiframe reset f2");
                    assert_bytes_eq("multiframe reset f2 content", &d2, &r2.out);
                    // a third frame, this time without a reset
                    let r3 = drive_with(cd, rd, &f1, d1.len() + 64, &cfg, "multiframe noreset f3");
                    assert_bytes_eq("multiframe noreset f3 content", &d1, &r3.out);
                    assert_eq!(
                        (a.free_dctx.0)(cd),
                        (a.free_dctx.1)(rd),
                        "multiframe reset dStage"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 18. Tiny sizes and block boundaries
// ===========================================================================

#[test]
fn decompress_tiny_and_boundary_sizes() {
    let mut rng = Rng::new(0x7189_0001);
    for &bsid in &[LZ4F_max64KB, LZ4F_max256KB] {
        let bs = block_size_of(bsid);
        for &len in &[
            0usize,
            1,
            2,
            3,
            4,
            5,
            12,
            13,
            bs - 1,
            bs,
            bs + 1,
            2 * bs,
            2 * bs + 1,
        ] {
            for shape in 0..N_SHAPES {
                let data = gen_shape(&mut rng, shape, len);
                for &with_csz in &[false, true] {
                    for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                        let prefs = prefs_of(
                            bsid,
                            bmode,
                            1,
                            1,
                            if with_csz { data.len() as u64 } else { 0 },
                            0,
                            0,
                            1,
                        );
                        let spec = FrameSpec::new(prefs);
                        let label = format!(
                            "boundary bsid={} len={} shape={} csz={} bmode={}",
                            bsid,
                            len,
                            shape_name(shape),
                            with_csz,
                            bmode
                        );
                        roundtrip(
                            &spec,
                            &data,
                            &DecodeCfg::new(SrcPlan::OneShot, DstPlan::All),
                            &label,
                        );
                        if len <= 3000 {
                            roundtrip(
                                &spec,
                                &data,
                                &DecodeCfg::new(SrcPlan::Fixed(1), DstPlan::Fixed(1)),
                                &format!("{} [1/1]", label),
                            );
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 19. Error / corruption paths
// ===========================================================================

#[test]
fn decompress_error_paths() {
    let mut rng = Rng::new(0xE220_0001);
    let data = gen_shape(&mut rng, 4, 20_000);

    // contentSize + dictID present => a 19-byte frame header.
    let base = prefs_of(
        LZ4F_max64KB,
        LZ4F_blockLinked,
        LZ4F_contentChecksumEnabled,
        LZ4F_blockChecksumEnabled,
        data.len() as u64,
        0xFEED,
        0,
        1,
    );
    let good = build_frame(&FrameSpec::new(base), &data);
    assert!(good.len() > 100);

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    for &delta in &[1u32, 0x10, 0xFFFF] {
        let mut f = good.clone();
        let m = u32::from_le_bytes([f[0], f[1], f[2], f[3]]).wrapping_add(delta);
        f[..4].copy_from_slice(&m.to_le_bytes());
        cases.push((format!("bad magic +{:#x}", delta), f));
    }
    {
        let mut f = good.clone();
        f[4] |= 0x02;
        cases.push(("FLG reserved bit set".into(), f));
    }
    for &v in &[0u8, 2, 3] {
        let mut f = good.clone();
        f[4] = (f[4] & 0x3F) | (v << 6);
        cases.push((format!("FLG version={}", v), f));
    }
    for &bd in &[0x00u8, 0x10, 0x20, 0x30, 0x80, 0x41, 0x4F, 0xF0] {
        let mut f = good.clone();
        f[5] = bd;
        cases.push((format!("BD={:#04x}", bd), f));
    }
    {
        let mut f = good.clone();
        f[18] ^= 0xFF;
        cases.push(("bad header checksum".into(), f));
    }
    {
        let mut f = good.clone();
        f[19..23].copy_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
        cases.push(("oversized block header".into(), f));
    }
    {
        let mut f = good.clone();
        f[19..23].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        cases.push(("oversized uncompressed block header".into(), f));
    }
    for &off in &[24usize, 40, 100] {
        let mut f = good.clone();
        f[off] ^= 0xFF;
        cases.push((format!("payload flip @{}", off), f));
    }
    {
        let mut f = good.clone();
        let n = f.len();
        f[n - 1] ^= 0xFF;
        cases.push(("bad content checksum".into(), f));
    }
    for &t in &[
        0usize, 1, 2, 3, 4, 5, 6, 7, 8, 12, 18, 19, 20, 22, 23, 30, 100, 1000,
    ] {
        cases.push((format!("truncated to {}", t), good[..t].to_vec()));
    }
    {
        let mut f = good.clone();
        f[6..14].copy_from_slice(&((data.len() as u64) + 1).to_le_bytes());
        cases.push(("contentSize +1 (breaks the header checksum)".into(), f));
    }
    for i in 0..40 {
        let n = 1 + (i * 7) % 64;
        cases.push((format!("garbage #{} ({}B)", i, n), gen_random(&mut rng, n)));
    }

    for (name, frame) in &cases {
        for &(sp, dp) in &[
            (SrcPlan::OneShot, DstPlan::All),
            (SrcPlan::Fixed(1), DstPlan::All),
            (SrcPlan::Fixed(7), DstPlan::Fixed(5)),
        ] {
            let label = format!("error [{}] src={:?} dst={:?}", name, sp, dp);
            // The driver asserts identical returns / out-params / buffers and
            // stops at the first error.
            let _ = drive(frame, data.len(), &DecodeCfg::new(sp, dp), &label);
        }
    }

    // Randomly mutated frames: differential fuzzing of the decoder state machine.
    for iter in 0..12_000u32 {
        let mut f = good.clone();
        let nmut = 1 + (iter as usize % 4);
        for _ in 0..nmut {
            let i = rng.below(f.len());
            f[i] ^= 1u8 << rng.below(8);
        }
        if iter % 3 == 0 {
            let cut = rng.below(f.len());
            f.truncate(cut);
        }
        let label = format!("fuzz iter={} nmut={}", iter, nmut);
        let _ = drive(
            &f,
            data.len(),
            &DecodeCfg::new(
                SrcPlan::Fixed(rng.range(1, 400)),
                DstPlan::Fixed(rng.range(1, 8000)),
            ),
            &label,
        );
    }
}

// ===========================================================================
// 20. RECYCLED destination buffer — the classic streaming pattern.
//
// The same buffer is handed to every LZ4F_decompress call and its contents are
// consumed in between, so previously emitted bytes do NOT stay live at their
// old address. That forces LZ4F_updateDict() through its tmpOutBuffer branches
// ("continue history within tmpOutBuffer", "copy relevant dict portion in front
// of tmpOut", "copy dst into tmp to complete dict", "join dict & dest into tmp")
// and makes the end-of-call "preserve history within tmpOut" block actually copy
// up to 64 KB. `stableDst` is deliberately NOT set here, because with a recycled
// buffer that promise would be false.
// ===========================================================================

#[test]
fn decompress_recycled_dst_buffer() {
    let mut rng = Rng::new(0x2EC7_0001);

    for &bsid in &[LZ4F_max64KB, LZ4F_max256KB] {
        let bs = block_size_of(bsid);
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for shape in 0..N_SHAPES {
                for &len in &[
                    0usize,
                    1,
                    17,
                    4000,
                    bs - 1,
                    bs,
                    bs + 1,
                    2 * bs + 4321,
                ] {
                    let data = gen_shape(&mut rng, shape, len);
                    for &cc in &[0, 1] {
                        for &bc in &[0, 1] {
                            let prefs =
                                prefs_of(bsid, bmode, cc, bc, data.len() as u64, 0x5151, 0, 1);
                            let mut spec = FrameSpec::new(prefs);
                            spec.update_chunk = 7777;
                            let frame = build_frame(&spec, &data);
                            for &buf in &[
                                1usize,
                                7,
                                100,
                                4096,
                                65_535,
                                65_536,
                                65_537,
                                131_072,
                                131_073,
                                200_003,
                            ] {
                                for &sp in &[
                                    SrcPlan::OneShot,
                                    SrcPlan::Fixed(3),
                                    SrcPlan::Fixed(1000),
                                    SrcPlan::Hint,
                                ] {
                                    // Bound the work: skip rows that would need
                                    // an excessive number of round trips.
                                    let src_step = match sp {
                                        SrcPlan::Fixed(n) => n,
                                        _ => 4096,
                                    };
                                    let est = len / buf.max(1) + frame.len() / src_step + 8;
                                    if est > 6000 {
                                        continue;
                                    }
                                    let label = format!(
                                        "recycle bsid={} bmode={} shape={} len={} cc={} bc={} buf={} src={:?}",
                                        bsid, bmode, shape_name(shape), len, cc, bc, buf, sp
                                    );
                                    let cfg = DecodeCfg::recycled(sp, buf);
                                    let res = drive(&frame, buf, &cfg, &label);
                                    assert!(
                                        !lz4f_is_error(res.last_ret),
                                        "{}: {}",
                                        label,
                                        ret_str(res.last_ret)
                                    );
                                    assert_eq!(
                                        res.consumed,
                                        frame.len(),
                                        "{}: consumed",
                                        label
                                    );
                                    assert_eq!(res.last_ret, 0, "{}: hint", label);
                                    assert_bytes_eq(
                                        &format!("{}: content", label),
                                        &data,
                                        &res.out,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // recycled dst + a dictionary (LZ4F_decompress_usingDict)
    let dbuf = gen_selfref(&mut rng, 70_000);
    for &dsz in &[1usize, 100, 4096, 65_536, 70_000] {
        let dict = &dbuf[..dsz];
        for &kind in &[DictKind::Raw, DictKind::CDict] {
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                let mut data = Vec::new();
                data.extend_from_slice(&dict[dsz - dsz.min(3000)..]);
                data.extend_from_slice(&gen_shape(&mut rng, 5, 90_000));
                let prefs = prefs_of(LZ4F_max64KB, bmode, 1, 1, data.len() as u64, 3, 0, 1);
                let mut spec = FrameSpec::new(prefs);
                spec.dict = Some(dict);
                spec.dict_kind = kind;
                spec.update_chunk = 20_000;
                let frame = build_frame(&spec, &data);
                for &buf in &[100usize, 4096, 65_536, 131_072] {
                    for &sp in &[SrcPlan::OneShot, SrcPlan::Fixed(777), SrcPlan::Hint] {
                        let label = format!(
                            "recycle+dict dsz={} kind={:?} bmode={} buf={} src={:?}",
                            dsz, kind, bmode, buf, sp
                        );
                        let mut cfg = DecodeCfg::recycled(sp, buf);
                        cfg.dict = Some(dict);
                        let res = drive(&frame, buf, &cfg, &label);
                        assert!(
                            !lz4f_is_error(res.last_ret),
                            "{}: {}",
                            label,
                            ret_str(res.last_ret)
                        );
                        assert_bytes_eq(&format!("{}: content", label), &data, &res.out);
                    }
                }
            }
        }
    }

    // recycled dst + skipChecksums, and recycled dst with stableDst=1 (invalid
    // usage: no content assertion, but C and Rust must still agree exactly).
    let data = gen_shape(&mut rng, 4, 300_000);
    for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
        let prefs = prefs_of(LZ4F_max64KB, bmode, 1, 1, data.len() as u64, 0, 0, 1);
        let spec = FrameSpec::new(prefs);
        let frame = build_frame(&spec, &data);
        for &buf in &[1000usize, 65_536, 150_000] {
            let mut cfg = DecodeCfg::recycled(SrcPlan::Fixed(5000), buf);
            cfg.opts = Some(dopts(0, 1));
            let label = format!("recycle skipChecksums bmode={} buf={}", bmode, buf);
            let res = drive(&frame, buf, &cfg, &label);
            assert!(!lz4f_is_error(res.last_ret), "{}: {}", label, ret_str(res.last_ret));
            assert_bytes_eq(&format!("{}: content", label), &data, &res.out);

            let mut cfg = DecodeCfg::recycled(SrcPlan::Fixed(5000), buf);
            cfg.opts = Some(dopts(1, 1));
            let label = format!("recycle stableDst(invalid) bmode={} buf={}", bmode, buf);
            let _ = drive(&frame, buf, &cfg, &label);
        }
    }
}

// ===========================================================================
// 21. harness self-check: the comparison logic really does detect divergence
// ===========================================================================

#[test]
#[should_panic(expected = "SELFCHECK")]
fn harness_self_check_detects_divergence() {
    let a = api();
    let mut rng = Rng::new(0x5E1F_0001);
    let data = gen_shape(&mut rng, 3, 5000);
    let spec = FrameSpec::new(prefs_of(
        LZ4F_max64KB,
        LZ4F_blockLinked,
        LZ4F_contentChecksumEnabled,
        LZ4F_blockChecksumEnabled,
        0,
        0,
        0,
        1,
    ));
    let good = build_frame(&spec, &data);
    let mut bad = good.clone();
    bad[40] ^= 0xFF; // inside the first compressed block payload

    unsafe {
        let mut cd: *mut c_void = std::ptr::null_mut();
        let mut rd: *mut c_void = std::ptr::null_mut();
        (a.create_dctx.0)(&mut cd, LZ4F_VERSION);
        (a.create_dctx.1)(&mut rd, LZ4F_VERSION);
        let mut oc = vec![SENTINEL; data.len() + 64];
        let mut orr = vec![SENTINEL; data.len() + 64];
        let mut cs = good.len();
        let mut rs = bad.len();
        let mut cdst = oc.len();
        let mut rdst = orr.len();
        // Deliberately feed the Rust side a DIFFERENT frame.
        let cret = (a.decompress.0)(
            cd,
            oc.as_mut_ptr() as *mut c_void,
            &mut cdst,
            good.as_ptr() as *const c_void,
            &mut cs,
            std::ptr::null(),
        );
        let rret = (a.decompress.1)(
            rd,
            orr.as_mut_ptr() as *mut c_void,
            &mut rdst,
            bad.as_ptr() as *const c_void,
            &mut rs,
            std::ptr::null(),
        );
        assert_eq!(ret_str(cret), ret_str(rret), "SELFCHECK hint");
        assert_eq!(cdst, rdst, "SELFCHECK dstSize");
        assert_bytes_eq("SELFCHECK dst bytes", &oc, &orr);
        panic!("SELFCHECK never reached: divergence was NOT detected");
    }
}

// ===========================================================================
// 22. Entering LZ4F_decompress with dStage == dstage_init.
//
// `dstage_init` is normally reached by falling through from the frame-header
// stages inside a single call. The ONLY way it becomes the *entry* state of a
// call is to decode the header with LZ4F_getFrameInfo() first, so this row
// exercises that switch entry (and, for LZ4F_decompress_usingDict, the
// `dStage <= dstage_init` dictionary-installation branch).
// ===========================================================================

#[test]
fn decompress_entering_at_dstage_init() {
    let a = api();
    let mut rng = Rng::new(0x1417_0001);
    let dictbuf = gen_selfref(&mut rng, 40_000);

    for (i, prefs) in all_header_option_combos().iter().enumerate() {
        let mut p = *prefs;
        let data = gen_shape(&mut rng, i, 9000);
        if p.frameInfo.contentSize != 0 {
            p.frameInfo.contentSize = data.len() as u64;
        }
        for &use_dict in &[false, true] {
            let mut spec = FrameSpec::new(p);
            if use_dict {
                spec.dict = Some(&dictbuf);
                spec.dict_kind = DictKind::Raw;
            }
            spec.update_chunk = 2000;
            let frame = build_frame(&spec, &data);

            for &hdr_src in &[LZ4F_HEADER_SIZE_MAX, 19usize, frame.len()] {
                let hdr_src = hdr_src.min(frame.len());
                unsafe {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(
                        (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                        (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                    );
                    let mut c_fi = LZ4F_frameInfo_t::default();
                    let mut r_fi = LZ4F_frameInfo_t::default();
                    poison_fi(&mut c_fi);
                    poison_fi(&mut r_fi);
                    let mut cs = hdr_src;
                    let mut rs = hdr_src;
                    let c = (a.get_frame_info.0)(
                        cd,
                        &mut c_fi,
                        frame.as_ptr() as *const c_void,
                        &mut cs,
                    );
                    let r = (a.get_frame_info.1)(
                        rd,
                        &mut r_fi,
                        frame.as_ptr() as *const c_void,
                        &mut rs,
                    );
                    let ctx = format!(
                        "init-entry combo #{} use_dict={} hdr_src={}",
                        i, use_dict, hdr_src
                    );
                    assert_eq!(ret_str(c), ret_str(r), "{}: getFrameInfo", ctx);
                    assert_eq!(cs, rs, "{}: header bytes consumed", ctx);
                    assert_bytes_eq(
                        &format!("{}: frameInfo", ctx),
                        fi_bytes(&c_fi),
                        fi_bytes(&r_fi),
                    );
                    assert!(!lz4f_is_error(c), "{}: getFrameInfo failed", ctx);

                    // Now decompress the remainder; dStage == dstage_init on entry.
                    let mut cfg = DecodeCfg::new(SrcPlan::Fixed(511), DstPlan::Fixed(1023));
                    if use_dict {
                        cfg.dict = Some(&dictbuf);
                    }
                    let res = drive_with(
                        cd,
                        rd,
                        &frame[cs..],
                        data.len() + 64,
                        &cfg,
                        &ctx,
                    );
                    assert!(
                        !lz4f_is_error(res.last_ret),
                        "{}: {}",
                        ctx,
                        ret_str(res.last_ret)
                    );
                    assert_eq!(res.last_ret, 0, "{}: frame incomplete", ctx);
                    assert_bytes_eq(&format!("{}: content", ctx), &data, &res.out);
                    assert_eq!((a.free_dctx.0)(cd), (a.free_dctx.1)(rd), "{}: dStage", ctx);
                }
            }
        }
    }
}

// ===========================================================================
// 23. LZ4F_getFrameInfo while a skippable frame is still being skipped
//     (dStage == dstage_getSFrameSize / storeSFrameSize / skipSkippable, all of
//      which are > dstage_storeFrameHeader, so getFrameInfo re-enters
//      LZ4F_decompress with a zero-size src and a NULL dst).
// ===========================================================================

#[test]
fn get_frame_info_during_skippable_skip() {
    let a = api();
    let mut rng = Rng::new(0x1417_0002);

    for magic in [0x184D_2A50u32, 0x184D_2A55, 0x184D_2A5F] {
        for &plen in &[0usize, 1, 5, 400, 5000] {
            let payload = gen_random(&mut rng, plen);
            let sk = skippable_frame(magic, &payload);
            let data = gen_shape(&mut rng, 3, 3000);
            let real = build_frame(
                &FrameSpec::new(prefs_of(
                    LZ4F_max64KB,
                    LZ4F_blockLinked,
                    1,
                    1,
                    data.len() as u64,
                    0,
                    0,
                    1,
                )),
                &data,
            );
            let mut buf = sk.clone();
            buf.extend_from_slice(&real);

            for &prefix in &[1usize, 2, 3, 4, 5, 6, 7, 8, 9, 12, 19, 20] {
                let prefix = prefix.min(buf.len());
                unsafe {
                    let mut cd: *mut c_void = std::ptr::null_mut();
                    let mut rd: *mut c_void = std::ptr::null_mut();
                    assert_eq!(
                        (a.create_dctx.0)(&mut cd, LZ4F_VERSION),
                        (a.create_dctx.1)(&mut rd, LZ4F_VERSION)
                    );
                    let out_len = data.len() + 64;
                    let mut o_c = vec![SENTINEL; out_len];
                    let mut o_r = vec![SENTINEL; out_len];
                    let mut cs = prefix;
                    let mut rs = prefix;
                    let mut cdst = out_len;
                    let mut rdst = out_len;
                    let cret = (a.decompress.0)(
                        cd,
                        o_c.as_mut_ptr() as *mut c_void,
                        &mut cdst,
                        buf.as_ptr() as *const c_void,
                        &mut cs,
                        std::ptr::null(),
                    );
                    let rret = (a.decompress.1)(
                        rd,
                        o_r.as_mut_ptr() as *mut c_void,
                        &mut rdst,
                        buf.as_ptr() as *const c_void,
                        &mut rs,
                        std::ptr::null(),
                    );
                    let ctx = format!(
                        "skip-getFrameInfo magic={:#x} payload={} prefix={}",
                        magic, plen, prefix
                    );
                    assert_eq!(ret_str(cret), ret_str(rret), "{}: hint", ctx);
                    assert_eq!((cs, cdst), (rs, rdst), "{}: out-params", ctx);
                    assert_bytes_eq(&format!("{}: dst", ctx), &o_c, &o_r);

                    let mut c_fi = LZ4F_frameInfo_t::default();
                    let mut r_fi = LZ4F_frameInfo_t::default();
                    poison_fi(&mut c_fi);
                    poison_fi(&mut r_fi);
                    // NOTE: getFrameInfo is handed the *remaining* input. If the
                    // skippable frame happened to be fully skipped by the call
                    // above, LZ4F_resetDecompressionContext() already ran, so
                    // dStage == dstage_getFrameHeader and getFrameInfo will decode
                    // the following real frame's header (consuming bytes).
                    let mut cs2 = buf.len() - cs;
                    let mut rs2 = buf.len() - cs;
                    let c = (a.get_frame_info.0)(
                        cd,
                        &mut c_fi,
                        buf.as_ptr().add(cs) as *const c_void,
                        &mut cs2,
                    );
                    let r = (a.get_frame_info.1)(
                        rd,
                        &mut r_fi,
                        buf.as_ptr().add(cs) as *const c_void,
                        &mut rs2,
                    );
                    assert_eq!(ret_str(c), ret_str(r), "{}: getFrameInfo", ctx);
                    assert_eq!(cs2, rs2, "{}: getFrameInfo *srcSizePtr", ctx);
                    assert_bytes_eq(
                        &format!("{}: frameInfo", ctx),
                        fi_bytes(&c_fi),
                        fi_bytes(&r_fi),
                    );

                    // finish the buffer
                    let consumed = cs + cs2;
                    let mut cons2 = 0usize;
                    let mut prod = cdst;
                    loop {
                        let rest = &buf[consumed + cons2..];
                        let sl = rest.len().min(7);
                        let room = out_len - prod;
                        if room == 0 {
                            break;
                        }
                        let dl = room.min(13);
                        let mut a1 = sl;
                        let mut a2 = sl;
                        let mut b1 = dl;
                        let mut b2 = dl;
                        let x = (a.decompress.0)(
                            cd,
                            o_c.as_mut_ptr().add(prod) as *mut c_void,
                            &mut b1,
                            rest.as_ptr() as *const c_void,
                            &mut a1,
                            std::ptr::null(),
                        );
                        let y = (a.decompress.1)(
                            rd,
                            o_r.as_mut_ptr().add(prod) as *mut c_void,
                            &mut b2,
                            rest.as_ptr() as *const c_void,
                            &mut a2,
                            std::ptr::null(),
                        );
                        assert_eq!(ret_str(x), ret_str(y), "{}: tail hint", ctx);
                        assert_eq!((a1, b1), (a2, b2), "{}: tail out-params", ctx);
                        assert_bytes_eq(&format!("{}: tail dst", ctx), &o_c, &o_r);
                        assert!(!lz4f_is_error(x), "{}: tail {}", ctx, ret_str(x));
                        cons2 += a1;
                        prod += b1;
                        if a1 == 0 && b1 == 0 {
                            break;
                        }
                        if x == 0 && consumed + cons2 == buf.len() {
                            break;
                        }
                    }
                    assert_bytes_eq(
                        &format!("{}: content after getFrameInfo mid-skip", ctx),
                        &data,
                        &o_c[..prod],
                    );
                    assert_eq!((a.free_dctx.0)(cd), (a.free_dctx.1)(rd), "{}: dStage", ctx);
                }
            }
        }
    }
}

// ===========================================================================
// 24. Valid header, WRONG declared contentSize  =>  dstage_getSuffix must return
//     LZ4F_ERROR_frameSize_wrong. Built by splicing a correct 19-byte header
//     (contentSize = len(a), correct header checksum) onto the body of a frame
//     that actually carries len(b) != len(a) bytes, which also exercises the
//     wrap-around of `dctx->frameRemainingSize -= n`.
// ===========================================================================

#[test]
fn decompress_wrong_content_size() {
    let mut rng = Rng::new(0x1417_0003);

    for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
        for &cc in &[0, 1] {
            for &bc in &[0, 1] {
                for &uncompressed in &[false, true] {
                    if uncompressed && bmode == LZ4F_blockLinked {
                        continue; // LZ4F_uncompressedUpdate needs independent blocks
                    }
                    for &(la, lb) in &[
                        (1000usize, 2000usize),
                        (2000, 1000),
                        (1, 5000),
                        (5000, 1),
                        (3000, 3001),
                        (3001, 3000),
                    ] {
                        let da = gen_shape(&mut rng, 4, la);
                        let db = gen_shape(&mut rng, 4, lb);
                        // dictID != 0 and contentSize != 0 => 19-byte header for both
                        let mut sa = FrameSpec::new(prefs_of(
                            LZ4F_max64KB,
                            bmode,
                            cc,
                            bc,
                            la as u64,
                            0xAB,
                            0,
                            1,
                        ));
                        let mut sb = FrameSpec::new(prefs_of(
                            LZ4F_max64KB,
                            bmode,
                            cc,
                            bc,
                            lb as u64,
                            0xAB,
                            0,
                            1,
                        ));
                        sa.uncompressed = uncompressed;
                        sb.uncompressed = uncompressed;
                        sa.update_chunk = 700;
                        sb.update_chunk = 700;
                        let fa = build_frame(&sa, &da);
                        let fb = build_frame(&sb, &db);
                        assert_eq!(&fa[..19].len(), &19usize);

                        let mut spliced = fa[..19].to_vec();
                        spliced.extend_from_slice(&fb[19..]);

                        for &(sp, dp) in &[
                            (SrcPlan::OneShot, DstPlan::All),
                            (SrcPlan::Fixed(1), DstPlan::All),
                            (SrcPlan::Fixed(31), DstPlan::Fixed(17)),
                        ] {
                            let label = format!(
                                "wrong-contentSize bmode={} cc={} bc={} uncompressed={} declared={} actual={} src={:?} dst={:?}",
                                bmode, cc, bc, uncompressed, la, lb, sp, dp
                            );
                            let res =
                                drive(&spliced, lb.max(la), &DecodeCfg::new(sp, dp), &label);
                            assert!(
                                lz4f_is_error(res.last_ret),
                                "{}: expected an error, got hint {}",
                                label,
                                ret_str(res.last_ret)
                            );
                            assert_eq!(
                                lz4f_error_code(res.last_ret),
                                err::ERROR_frameSize_wrong,
                                "{}: expected LZ4F_ERROR_frameSize_wrong",
                                label
                            );
                        }
                    }
                }
            }
        }
    }
}
