//! Phase B + Phase C — FRAME INSPECTION, the low-level BLOCK API, the
//! bufferless begin/continue/end drivers, STATIC (in-place) contexts, the memory
//! estimators and CUSTOM ALLOCATORS.
//!
//! These entry points are all "pure" in the sense that they either parse bytes
//! that somebody else produced (`ZSTD_getFrameHeader*`, `ZSTD_findFrameSizeInfo`
//! and friends) or they hand the caller direct control over the encoder /
//! decoder state machine (`ZSTD_compressBegin`/`compressContinue`/`compressEnd`,
//! `ZSTD_decompressBegin`/`decompressContinue`). Both properties make them very
//! sensitive to translation slips that the one-shot API rounds away:
//!
//!   * the "need more input" *hint* values (`5`, `8`, `6..18`) are part of the
//!     contract and are indistinguishable from success unless compared exactly;
//!   * `ZSTD_FrameHeader` is only partly written on the short-input paths, so the
//!     out-parameter is pre-poisoned here and every field is compared;
//!   * `ZSTD_nextSrcSizeToDecompress` / `ZSTD_nextInputType` expose the raw
//!     `ZSTD_dStage` machine, so a mis-ordered state assignment is visible;
//!   * `ZSTD_estimate*Size` and the `ZSTD_initStatic*` family expose the exact
//!     workspace layout, which is the strongest available structural check on the
//!     translation of `zstd_cwksp.h`;
//!   * a counting custom allocator pins the *number* and *sizes* of allocations.
//!
//! Everything is compared through `diff`/`diff_bytes` against the reference C
//! `.so`; nothing in here asserts an absolute value except where the C source
//! makes it a hard constant.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Constants taken from the C headers
// ---------------------------------------------------------------------------

/// `ZSTD_FRAMEHEADERSIZE_PREFIX(ZSTD_f_zstd1)` (`zstd.h:1257`).
const FHSIZE_PREFIX_ZSTD1: usize = 5;
/// `ZSTD_FRAMEHEADERSIZE_PREFIX(ZSTD_f_zstd1_magicless)`.
const FHSIZE_PREFIX_MAGICLESS: usize = 1;
/// `ZSTD_FRAMEHEADERSIZE_MAX` (`zstd.h:1259`).
const FHSIZE_MAX: usize = 18;
/// `ZSTD_SKIPPABLEHEADERSIZE` (`zstd.h:1260`).
const SKIPPABLEHEADERSIZE: usize = 8;
/// `ZSTD_blockHeaderSize` (`zstd_internal.h:85`).
const BLOCKHEADERSIZE: usize = 3;
/// `MIN_CBLOCK_SIZE` (`zstd_internal.h:91`).
const MIN_CBLOCK_SIZE: usize = 2;
/// `ZSTD_WINDOWLOG_MAX` on a 64-bit build (`zstd.h:1265`).
const WINDOWLOG_MAX: u32 = 31;

/// `ZSTD_nextInputType_e` (`zstd.h:3136`).
const ZSTDnit_frameHeader: c_int = 0;
const ZSTDnit_blockHeader: c_int = 1;
const ZSTDnit_block: c_int = 2;
const ZSTDnit_lastBlock: c_int = 3;
const ZSTDnit_checksum: c_int = 4;
const ZSTDnit_skippableFrame: c_int = 5;

fn nit_name(v: c_int) -> &'static str {
    match v {
        ZSTDnit_frameHeader => "frameHeader",
        ZSTDnit_blockHeader => "blockHeader",
        ZSTDnit_block => "block",
        ZSTDnit_lastBlock => "lastBlock",
        ZSTDnit_checksum => "checksum",
        ZSTDnit_skippableFrame => "skippableFrame",
        _ => "??",
    }
}

/// The three legacy magic numbers this build recognises (`ZSTD_LEGACY_SUPPORT=5`
/// makes `zstd_legacy.h`'s `ZSTD_isLegacy` compile only the `<=5`, `<=6` and
/// `<=7` arms), plus the four it must *not*.
const LEGACY_MAGICS_LE: &[(&str, u32)] = &[
    ("v01", 0x1EB5_2FFD), // read big-endian by v01, so this LE value is what isLegacy sees
    ("v02", 0xFD2F_B522),
    ("v03", 0xFD2F_B523),
    ("v04", 0xFD2F_B524),
    ("v05", 0xFD2F_B525),
    ("v06", 0xFD2F_B526),
    ("v07", 0xFD2F_B527),
];

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnGetFrameHeader =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT) -> SizeT;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT, c_int) -> SizeT;
type FnPtrLenSz = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnPtrLenU64 = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
type FnPtrLenU32 = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;
type FnReadSkippable =
    unsafe extern "C" fn(*mut c_void, SizeT, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_uint) -> SizeT;
type FnDecodingBufferSizeMin = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> SizeT;

type FnCtxSz = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnCtxNit = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnBlock5 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnCompressBegin = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnCompressBeginDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnCompressBeginAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    SizeT,
    ZSTD_parameters,
    c_ulonglong,
) -> SizeT;
type FnCompressBeginCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnCompressBeginCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    ZSTD_frameParameters,
    c_ulonglong,
) -> SizeT;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> SizeT;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnDecompressBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnDecompressBeginDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> SizeT;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnRefPrefix = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;

type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnGetCParams =
    unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_compressionParameters;

type FnEstFromInt = unsafe extern "C" fn(c_int) -> SizeT;
type FnEstFromCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> SizeT;
type FnEstFromPtr = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnEstVoid = unsafe extern "C" fn() -> SizeT;
type FnEstFromSize = unsafe extern "C" fn(SizeT) -> SizeT;
type FnEstCDict = unsafe extern "C" fn(SizeT, c_int) -> SizeT;
type FnEstCDictAdv =
    unsafe extern "C" fn(SizeT, ZSTD_compressionParameters, c_int) -> SizeT;
type FnEstDDict = unsafe extern "C" fn(SizeT, c_int) -> SizeT;

type FnInitStatic = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    c_int,
    c_int,
    ZSTD_compressionParameters,
) -> *mut c_void;
type FnInitStaticDDict =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_int, c_int) -> *mut c_void;

type FnCreateAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(
    *const c_void,
    SizeT,
    c_int,
    c_int,
    ZSTD_compressionParameters,
    ZSTD_customMem,
) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, SizeT, c_int, c_int, ZSTD_customMem) -> *mut c_void;

type FnCompressUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
) -> SizeT;
type FnDictIdFromObj = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDDictContent = unsafe extern "C" fn(*const c_void) -> *const c_void;
type FnDDictSize = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnSizeofObj = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnRefThreadPool = unsafe extern "C" fn(*mut c_void, *mut c_void) -> SizeT;

type FnZdictFinalize = unsafe extern "C" fn(
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    *const SizeT,
    c_uint,
    ZDICT_params_t,
) -> SizeT;

// ---------------------------------------------------------------------------
// Comparable result wrappers
// ---------------------------------------------------------------------------

/// `unsigned long long` returns with the two sentinels spelled out, so a
/// divergence names `UNKNOWN`/`ERROR` instead of `18446744073709551615`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum U64R {
    Unknown,
    Error,
    V(u64),
}

impl std::fmt::Debug for U64R {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            U64R::Unknown => write!(f, "CONTENTSIZE_UNKNOWN"),
            U64R::Error => write!(f, "CONTENTSIZE_ERROR"),
            U64R::V(v) => write!(f, "{v}"),
        }
    }
}

fn u64r(v: u64) -> U64R {
    if v == ZSTD_CONTENTSIZE_UNKNOWN {
        U64R::Unknown
    } else if v == ZSTD_CONTENTSIZE_ERROR {
        U64R::Error
    } else {
        U64R::V(v)
    }
}

/// A `ZSTD_FrameHeader` out-parameter plus the return value. The struct is
/// pre-poisoned with a recognisable pattern because `ZSTD_getFrameHeader_advanced`
/// only `memset`s it once `srcSize >= minInputSize` — the "untouched" case is
/// itself part of the contract.
#[derive(Clone, PartialEq, Eq)]
struct Fh {
    ret: R,
    zfh: ZSTD_FrameHeader,
}

impl std::fmt::Debug for Fh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let z = &self.zfh;
        if z == &POISON_FH {
            return write!(f, "{:?}/zfh=<untouched>", self.ret);
        }
        write!(
            f,
            "{:?}/fcs={:?} win={} bsMax={} type={} hSize={} dictID={} cks={} r1={} r2={}",
            self.ret,
            u64r(z.frameContentSize),
            z.windowSize,
            z.blockSizeMax,
            z.frameType,
            z.headerSize,
            z.dictID,
            z.checksumFlag,
            z._reserved1,
            z._reserved2
        )
    }
}

const POISON_FH: ZSTD_FrameHeader = ZSTD_FrameHeader {
    frameContentSize: 0xA5A5_A5A5_A5A5_A5A5,
    windowSize: 0x5A5A_5A5A_5A5A_5A5A,
    blockSizeMax: 0xA5A5_A5A5,
    frameType: 0x5A5A_5A5A,
    headerSize: 0xA5A5_A5A5,
    dictID: 0x5A5A_5A5A,
    checksumFlag: 0xA5A5_A5A5,
    _reserved1: 0x5A5A_5A5A,
    _reserved2: 0xA5A5_A5A5,
};

fn fh(l: &Lib, src: &[u8], n: usize) -> Fh {
    let f = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
    let mut zfh = POISON_FH;
    let r = unsafe { f(&mut zfh, src.as_ptr() as *const c_void, n) };
    Fh { ret: res(l, r), zfh }
}

fn fh_adv(l: &Lib, src: &[u8], n: usize, format: c_int) -> Fh {
    let f = l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced");
    let mut zfh = POISON_FH;
    let r = unsafe { f(&mut zfh, src.as_ptr() as *const c_void, n, format) };
    Fh { ret: res(l, r), zfh }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn ptr(b: &[u8]) -> *const c_void {
    b.as_ptr() as *const c_void
}

fn setp(l: &Lib, ctx: *mut c_void, p: c_int, v: c_int) -> R {
    let f = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    res(l, unsafe { f(ctx, p, v) })
}

fn setdp(l: &Lib, ctx: *mut c_void, p: c_int, v: c_int) -> R {
    let f = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    res(l, unsafe { f(ctx, p, v) })
}

/// The prefix lengths every frame-inspection sweep uses: every "need more input"
/// boundary (`0..=20`) plus a couple past the longest possible header.
fn prefix_lens(total: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..=20).collect();
    for extra in [21usize, 24, 30, 64] {
        if extra <= total {
            v.push(extra);
        }
    }
    if total > 0 {
        v.push(total - 1);
    }
    v.push(total);
    v.sort_unstable();
    v.dedup();
    v.retain(|&n| n <= total);
    v
}

/// A well-formed zstd dictionary with an explicitly chosen `dictID`, built once
/// with the C library (a *fixture*: the dictionary builder itself is covered
/// elsewhere). `ZDICT_finalizeDictionary` is used rather than
/// `ZDICT_trainFromBuffer` because it lets the dictID be pinned and is cheap.
fn dict_fixture() -> &'static Vec<u8> {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    D.get_or_init(|| {
        let l = &pair().c;
        let f = l.sym::<FnZdictFinalize>("ZDICT_finalizeDictionary");
        let content = corpus(Corpus::Text, 8192, 0x1CE_5EED);
        // 24 samples of related text so the entropy tables are meaningful.
        let mut samples = Vec::new();
        let mut sizes: Vec<SizeT> = Vec::new();
        for i in 0..24u64 {
            let s = corpus(Corpus::Text, 1024, 0x1CE_5EED ^ (i << 8));
            sizes.push(s.len());
            samples.extend_from_slice(&s);
        }
        let mut out = vec![0u8; 24 * 1024];
        let params = ZDICT_params_t {
            compressionLevel: 3,
            notificationLevel: 0,
            dictID: 0x1234_5678, // >= 65536 -> the 4-byte dictID field code
        };
        let n = unsafe {
            f(
                out.as_mut_ptr() as *mut c_void,
                out.len(),
                ptr(&content),
                content.len(),
                ptr(&samples),
                sizes.as_ptr(),
                sizes.len() as c_uint,
                params,
            )
        };
        assert!(
            !is_error(l, n),
            "dictionary fixture could not be built: {}",
            err_name(l, n)
        );
        out.truncate(n);
        assert_eq!(
            u32::from_le_bytes(out[0..4].try_into().unwrap()),
            ZSTD_MAGIC_DICTIONARY
        );
        assert_eq!(
            u32::from_le_bytes(out[4..8].try_into().unwrap()),
            0x1234_5678
        );
        out
    })
}

/// A 4 KB raw (non-conformant, magic-less) dictionary buffer.
fn raw_dict() -> Vec<u8> {
    corpus(Corpus::Text, 4096, 0xD1C7_0000)
}

// ---------------------------------------------------------------------------
// Frame fixture construction
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Compressed with `ZSTD_compress2`; the pledged size is the real size.
    Known(usize),
    /// Compressed with `ZSTD_compressStream2(ZSTD_e_end)` and no pledged size,
    /// so the frame header carries no content size.
    Unknown(usize),
    /// `ZSTD_CCtx_setPledgedSrcSize(p)` then a single
    /// `ZSTD_compressStream2(ZSTD_e_flush)` over a few bytes. This produces a
    /// *header-only* fixture: the frame is deliberately incomplete, so only the
    /// header parsers are driven over it. It is the only way to obtain a frame
    /// header whose frameContentSize field is 2^32-ish without compressing 4 GB.
    Pledged(u64),
}

#[derive(Clone, Copy, Debug)]
struct Spec {
    csf: c_int,
    cks: c_int,
    did: c_int,
    fmt: c_int,
    wlog: c_int,
    kind: Kind,
    dict: bool,
}

impl Spec {
    fn label(&self) -> String {
        format!(
            "csf{}cks{}did{}fmt{}wlog{}{}{}",
            self.csf,
            self.cks,
            self.did,
            self.fmt,
            self.wlog,
            match self.kind {
                Kind::Known(n) => format!("/known{n}"),
                Kind::Unknown(n) => format!("/unknown{n}"),
                Kind::Pledged(p) => format!("/pledged0x{p:x}"),
            },
            if self.dict { "/dict" } else { "" }
        )
    }
}

fn build_frame(l: &Lib, sp: Spec, payload: &[u8]) -> (R, Blob) {
    let cctx = Ctx::cctx(l);
    for (p, v) in [
        (ZSTD_c_compressionLevel, 1),
        (ZSTD_c_contentSizeFlag, sp.csf),
        (ZSTD_c_checksumFlag, sp.cks),
        (ZSTD_c_dictIDFlag, sp.did),
        (ZSTD_c_format, sp.fmt),
        (ZSTD_c_windowLog, sp.wlog),
    ] {
        let r = setp(l, cctx.ptr, p, v);
        // Note: ZSTD_CCtx_setParameter returns the *value it set* for some
        // parameters (compressionLevel among them), so only "not an error" can
        // be asserted here.
        assert!(
            matches!(r, R::Ok(_)),
            "[{}] setParameter({p},{v}) -> {r:?}",
            l.tag
        );
    }
    if sp.dict {
        let d = dict_fixture();
        let f = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let r = res(l, unsafe { f(cctx.ptr, ptr(d), d.len()) });
        assert_eq!(r, R::Ok(0), "[{}] loadDictionary", l.tag);
    }
    let cap = compress_bound(l, payload.len().max(64)) + 64;
    let mut dst = vec![0xCDu8; cap];
    match sp.kind {
        Kind::Known(_) => {
            let f = l.sym::<FnCompress2>("ZSTD_compress2");
            let n = unsafe {
                f(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr(payload),
                    payload.len(),
                )
            };
            let r = res(l, n);
            if let R::Ok(n) = r {
                dst.truncate(n);
            }
            (r, Blob(dst))
        }
        Kind::Unknown(_) => {
            let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let mut inb = ZSTD_inBuffer {
                src: ptr(payload),
                size: payload.len(),
                pos: 0,
            };
            let mut outb = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: cap,
                pos: 0,
            };
            let mut last = R::Ok(0);
            for _ in 0..64 {
                let n = unsafe { f(cctx.ptr, &mut outb, &mut inb, ZSTD_e_end) };
                last = res(l, n);
                match last {
                    R::Ok(0) => break,
                    R::Ok(_) => {}
                    R::Err(..) => break,
                }
            }
            if let R::Ok(_) = last {
                dst.truncate(outb.pos);
            }
            (last, Blob(dst))
        }
        Kind::Pledged(p) => {
            let sps = l.sym::<FnSetPledged>("ZSTD_CCtx_setPledgedSrcSize");
            let r0 = res(l, unsafe { sps(cctx.ptr, p) });
            assert_eq!(r0, R::Ok(0), "[{}] setPledgedSrcSize", l.tag);
            let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let mut inb = ZSTD_inBuffer {
                src: ptr(payload),
                size: payload.len(),
                pos: 0,
            };
            let mut outb = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: cap,
                pos: 0,
            };
            let n = unsafe { f(cctx.ptr, &mut outb, &mut inb, ZSTD_e_flush) };
            let r = res(l, n);
            if let R::Ok(_) = r {
                dst.truncate(outb.pos);
            }
            (r, Blob(dst))
        }
    }
}

/// Build one frame with BOTH libraries and require the bytes to match, then
/// return them. A divergence in the *fixture* is reported at the point it
/// happens rather than confusing a later inspection comparison.
fn frame_fixture(sp: Spec, payload: &[u8]) -> Vec<u8> {
    let (_, b) = diff_bytes(&format!("build[{}]", sp.label()), |l| {
        build_frame(l, sp, payload)
    });
    b.0
}

// ===========================================================================
// 1. Frame-header parsing: synthetic headers
// ===========================================================================

/// Exhaustive sweep of the frame-header-descriptor byte.
///
/// Targets `ZSTD_frameHeaderSize_internal` (`zstd_decompress.c:416`) and the
/// field-decode cascade in `ZSTD_getFrameHeader_advanced`
/// (`zstd_decompress.c:447`). Every one of the 256 FHD byte values is placed at
/// the byte the C reads (`src[minInputSize-1]`, i.e. offset 4 for `ZSTD_f_zstd1`
/// and offset 0 for `ZSTD_f_zstd1_magicless`) and each is probed at every
/// `srcSize` from 0 to past the longest possible header. This covers, in one
/// sweep: the `srcSize < minInputSize` magic pre-check (returns `5` or
/// `prefix_unknown`), the skippable branch's `8`, `frameHeaderSize_internal`'s
/// `6..18` / `2..14`, the `fhdByte & 0x08` reserved-bit rejection, the
/// `windowLog > 31` rejection, and all 4x4 (dictIDSizeCode, fcsID) decodes.
#[test]
fn frame_header_fhd_byte_sweep() {
    covers(&[
        "CFG:225",
        "CFG:227",
        "CFG:228",
        "CFG:122",
        "ERR:decompress/zstd_decompress.c:419",
        "ERR:decompress/zstd_decompress.c:473",
        "ERR:decompress/zstd_decompress.c:476",
        "ERR:decompress/zstd_decompress.c:485",
        "ERR:decompress/zstd_decompress.c:493",
        "ERR:decompress/zstd_decompress.c:498",
        "ERR:decompress/zstd_decompress.c:511",
        "ERR:decompress/zstd_decompress.c:517",
    ]);

    // A deterministic tail so the decoded dictID / frameContentSize values are
    // distinctive rather than all-zero.
    let tail = corpus(Corpus::Counter, 64, 7);

    for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
        let off = if fmt == ZSTD_f_zstd1 {
            FHSIZE_PREFIX_ZSTD1 - 1
        } else {
            FHSIZE_PREFIX_MAGICLESS - 1
        };
        for fhd in 0u32..=255 {
            let mut buf = vec![0u8; 64];
            if fmt == ZSTD_f_zstd1 {
                buf[0..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
            }
            buf[off] = fhd as u8;
            buf[off + 1..].copy_from_slice(&tail[..64 - off - 1]);
            let label = format!("fhd/fmt{fmt}/0x{fhd:02x}");
            diff(&label, |l| {
                let fha = l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced");
                let fhs = l.sym::<FnPtrLenSz>("ZSTD_frameHeaderSize");
                let mut out = Vec::with_capacity(64);
                for n in 0..=FHSIZE_MAX + 2 {
                    let mut zfh = POISON_FH;
                    let r = unsafe { fha(&mut zfh, ptr(&buf), n, fmt) };
                    out.push((n, Fh { ret: res(l, r), zfh }));
                }
                // ZSTD_frameHeaderSize hardcodes ZSTD_f_zstd1, so for the
                // magicless buffers it reads a *different* byte -- pinned too.
                let mut hs = Vec::new();
                for n in [0usize, 1, 4, 5, 6, FHSIZE_MAX] {
                    hs.push((n, res(l, unsafe { fhs(ptr(&buf), n) })));
                }
                (out, hs)
            });
        }
    }
}

/// Sweep of the window-descriptor byte, which only exists when
/// `singleSegment == 0`.
///
/// Targets `windowLog = (wlByte >> 3) + ZSTD_WINDOWLOG_ABSOLUTEMIN` and
/// `windowSize = (1 << wl) + (1 << wl >> 3) * (wlByte & 7)`
/// (`zstd_decompress.c:514-519`) plus the `windowLog > ZSTD_WINDOWLOG_MAX`
/// rejection at `:517`. `blockSizeMax = MIN(windowSize, ZSTD_BLOCKSIZE_MAX)` is
/// compared for every value, which pins the 3-bit mantissa arithmetic.
#[test]
fn frame_header_window_descriptor_sweep() {
    covers(&[
        "CFG:225",
        "CFG:227",
        "ERR:decompress/zstd_decompress.c:517",
    ]);
    // fhd: dictIDSizeCode=0, checksum=0, singleSegment=0, fcsID=2 (4-byte fcs)
    let fhd = 0b1000_0000u8;
    for wl in 0u32..=255 {
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
        buf[4] = fhd;
        buf[5] = wl as u8;
        buf[6..10].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        diff(&format!("wlbyte/0x{wl:02x}"), |l| fh(l, &buf, 32));
    }
    // Also the 4 fcsID codes x 4 dictID codes with singleSegment=1, where the
    // C substitutes `windowSize = frameContentSize` and where fcsID==0 means
    // "1 byte" only because singleSegment is set.
    for dic in 0u8..4 {
        for fcs in 0u8..4 {
            let fhd = (fcs << 6) | (1 << 5) | dic;
            let mut buf = vec![0u8; 32];
            buf[0..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
            buf[4] = fhd;
            for (i, b) in buf[5..32].iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(37).wrapping_add(1);
            }
            diff(&format!("singleSeg/dic{dic}/fcs{fcs}"), |l| {
                let mut v = Vec::new();
                for n in prefix_lens(20) {
                    v.push((n, fh(l, &buf, n)));
                }
                v
            });
        }
    }
}

/// Non-zstd prefixes: the `hbuf` magic pre-check for `0 < srcSize < 5`, the
/// `prefix_unknown` for a complete bad magic, the legacy magics (which
/// `ZSTD_getFrameHeader` deliberately does *not* special-case) and `src == NULL`.
///
/// `src == NULL` with `srcSize > 0` is a real, documented error return
/// (`RETURN_ERROR_IF(src==NULL, GENERIC)`, `zstd_decompress.c:456`) and not an
/// assert, so it is safe to drive.
#[test]
fn frame_header_bad_prefixes() {
    covers(&[
        "CFG:225",
        "CFG:227",
        "ERR:decompress/zstd_decompress.c:456",
        "ERR:decompress/zstd_decompress.c:473",
        "ERR:decompress/zstd_decompress.c:493",
    ]);

    // src == NULL, srcSize > 0 -> ERROR(GENERIC); srcSize == 0 -> minInputSize.
    diff("fh/null-src", |l| {
        let f = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
        let fa = l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced");
        let mut v = Vec::new();
        for n in [0usize, 1, 4, 5, 8, 18] {
            let mut zfh = POISON_FH;
            let r = unsafe { f(&mut zfh, std::ptr::null(), n) };
            v.push((n, Fh { ret: res(l, r), zfh }));
            let mut zfh = POISON_FH;
            let r = unsafe { fa(&mut zfh, std::ptr::null(), n, ZSTD_f_zstd1_magicless) };
            v.push((n, Fh { ret: res(l, r), zfh }));
        }
        v
    });

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // Every 1..4 byte prefix of the zstd magic, of a skippable magic, and of
    // some non-magic values.
    for (name, m) in [
        ("zstd", ZSTD_MAGICNUMBER),
        ("skip0", ZSTD_MAGIC_SKIPPABLE_START),
        ("skipF", ZSTD_MAGIC_SKIPPABLE_START + 15),
        ("skipBelow", ZSTD_MAGIC_SKIPPABLE_START - 1),
        ("skipAbove", ZSTD_MAGIC_SKIPPABLE_START + 16),
        ("dictMagic", ZSTD_MAGIC_DICTIONARY),
        ("zero", 0),
        ("ones", 0xFFFF_FFFF),
    ] {
        let mut b = m.to_le_bytes().to_vec();
        b.extend_from_slice(&[0u8; 28]);
        cases.push((name.to_string(), b));
    }
    for (name, m) in LEGACY_MAGICS_LE {
        let mut b = m.to_le_bytes().to_vec();
        b.extend_from_slice(&[0u8; 28]);
        cases.push((format!("legacy-{name}"), b));
    }
    let mut rng = Rng::new(0x2525);
    for i in 0..8 {
        cases.push((format!("rand{i}"), rng.bytes(32)));
    }

    for (name, buf) in &cases {
        diff(&format!("badprefix/{name}"), |l| {
            let mut v = Vec::new();
            for n in prefix_lens(20) {
                v.push((n, fh(l, buf, n), fh_adv(l, buf, n, ZSTD_f_zstd1_magicless)));
            }
            v
        });
        // isFrame / isSkippableFrame over the same bytes at sizes 0..=4.
        diff(&format!("isframe/{name}"), |l| {
            let isf = l.sym::<FnPtrLenU32>("ZSTD_isFrame");
            let iss = l.sym::<FnPtrLenU32>("ZSTD_isSkippableFrame");
            let gdi = l.sym::<FnPtrLenU32>("ZSTD_getDictID_fromFrame");
            let mut v = Vec::new();
            for n in [0usize, 1, 2, 3, 4, 5, 8, 32] {
                v.push((
                    n,
                    unsafe { isf(ptr(buf), n) },
                    unsafe { iss(ptr(buf), n) },
                    unsafe { gdi(ptr(buf), n) },
                ));
            }
            v
        });
    }
}

// ===========================================================================
// 2. Frame-header parsing: real frames over the whole option matrix
// ===========================================================================

/// Every whole-input query the frame-inspection surface offers, gathered into
/// one comparable record so a single `diff` pins them all.
#[derive(Debug, PartialEq, Eq)]
struct Whole {
    /// `ZSTD_frameHeaderSize` (`zstd_decompress.c:435`).
    fhsize: R,
    /// `ZSTD_getFrameContentSize` (`:569`) and `ZSTD_getDecompressedSize` (`:690`).
    fcs: U64R,
    dsize: U64R,
    /// `ZSTD_findFrameCompressedSize` (`:809`).
    ffcs: R,
    /// `ZSTD_findDecompressedSize` (`:643`).
    fds: U64R,
    /// `ZSTD_decompressBound` (`:820`).
    dbound: U64R,
    /// `ZSTD_decompressionMargin` (`:838`).
    margin: R,
    /// `ZSTD_isFrame` (`:385`) / `ZSTD_isSkippableFrame` (`:402`).
    isframe: c_uint,
    isskip: c_uint,
    /// `ZSTD_getDictID_fromFrame` (`:1644`).
    dictid: c_uint,
    /// `ZSTD_estimateDStreamSize_fromFrame` (`:2001`).
    edss: R,
}

fn whole_probe(l: &Lib, src: &[u8]) -> Whole {
    let n = src.len();
    let p = ptr(src);
    unsafe {
        Whole {
            fhsize: res(l, l.sym::<FnPtrLenSz>("ZSTD_frameHeaderSize")(p, n)),
            fcs: u64r(l.sym::<FnPtrLenU64>("ZSTD_getFrameContentSize")(p, n)),
            dsize: u64r(l.sym::<FnPtrLenU64>("ZSTD_getDecompressedSize")(p, n)),
            ffcs: res(l, l.sym::<FnPtrLenSz>("ZSTD_findFrameCompressedSize")(p, n)),
            fds: u64r(l.sym::<FnPtrLenU64>("ZSTD_findDecompressedSize")(p, n)),
            dbound: u64r(l.sym::<FnPtrLenU64>("ZSTD_decompressBound")(p, n)),
            margin: res(l, l.sym::<FnPtrLenSz>("ZSTD_decompressionMargin")(p, n)),
            isframe: l.sym::<FnPtrLenU32>("ZSTD_isFrame")(p, n),
            isskip: l.sym::<FnPtrLenU32>("ZSTD_isSkippableFrame")(p, n),
            dictid: l.sym::<FnPtrLenU32>("ZSTD_getDictID_fromFrame")(p, n),
            edss: res(
                l,
                l.sym::<FnPtrLenSz>("ZSTD_estimateDStreamSize_fromFrame")(p, n),
            ),
        }
    }
}

/// Drive the whole frame-inspection surface over the entire
/// `contentSizeFlag` x `checksumFlag` x `dictIDFlag` x `format` x `windowLog` x
/// `pledgedSrcSize` x `with/without dictionary` matrix, and over every prefix of
/// each frame.
///
/// The payload sizes straddle every `frameContentSize` field encoding the writer
/// can pick (`ZSTD_writeFrameHeader`, `zstd_compress.c:4700`): `0`, `255`/`256`
/// (the 1-byte singleSegment form and the `LE16+256` form) and `65791`/`65792`
/// (the `LE16+256` upper bound and the first size needing `LE32`). windowLog
/// {10,17,27} flips `singleSegment = contentSizeFlag && (windowSize >=
/// pledgedSrcSize)`, which in turn removes the window-descriptor byte and makes
/// the decoder reconstruct `windowSize` from `frameContentSize`.
#[test]
fn frame_inspection_real_frames_known_size() {
    covers(&[
        "CFG:40",
        "CFG:42",
        "CFG:82",
        "CFG:83",
        "CFG:97",
        "CFG:225",
        "CFG:229",
        "CFG:230",
        "CFG:231",
        "CFG:232",
        "CFG:235",
        "CFG:78",
        "CFG:165",
        "ERR:decompress/zstd_decompress.c:579",
        "ERR:decompress/zstd_decompress.c:661",
        "ERR:decompress/zstd_decompress.c:669",
        "ERR:decompress/zstd_decompress.c:677",
        "ERR:decompress/zstd_decompress.c:694",
        "ERR:decompress/zstd_decompress.c:760",
        "ERR:decompress/zstd_decompress.c:762",
        "ERR:decompress/zstd_decompress.c:773",
        "ERR:decompress/zstd_decompress.c:776",
        "ERR:decompress/zstd_decompress.c:788",
        "ERR:decompress/zstd_decompress.c:828",
        "ERR:decompress/zstd_decompress.c:850",
        "ERR:decompress/zstd_decompress.c:852",
        "ERR:decompress/zstd_decompress.c:2006",
        "ERR:decompress/zstd_decompress.c:2007",
    ]);

    // 0 / 255 / 256 / 65791 / 65792: the four frameContentSize encodings plus
    // the empty frame (which ZSTD_writeEpilogue special-cases).
    const PAYLOADS: &[usize] = &[0, 255, 256, 65791, 65792];

    for csf in [0, 1] {
        for cks in [0, 1] {
            for did in [0, 1] {
                for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                    for wlog in [10, 17, 27] {
                        for dict in [false, true] {
                            for &nb in PAYLOADS {
                                let sp = Spec {
                                    csf,
                                    cks,
                                    did,
                                    fmt,
                                    wlog,
                                    kind: Kind::Known(nb),
                                    dict,
                                };
                                let payload = corpus(Corpus::Text, nb, 0x2500 ^ nb as u64);
                                let frame = frame_fixture(sp, &payload);
                                let lab = sp.label();
                                diff(&format!("prefix[{lab}]"), |l| {
                                    let mut v = Vec::new();
                                    for n in prefix_lens(frame.len()) {
                                        v.push((
                                            n,
                                            fh(l, &frame, n),
                                            fh_adv(l, &frame, n, ZSTD_f_zstd1_magicless),
                                        ));
                                    }
                                    v
                                });
                                diff(&format!("whole[{lab}]"), |l| whole_probe(l, &frame));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The same surface for frames whose header carries *no* content size
/// (`ZSTD_compressStream2` without a pledged size) and for headers that declare
/// a 2^32-ish content size.
///
/// The `Kind::Pledged` fixtures are header-only by construction (see `Kind`), so
/// only the header parsers run over them: that is the only practical way to
/// exercise the `fcsID == 2` upper edge and the `fcsID == 3` (`MEM_readLE64`)
/// branch of `ZSTD_getFrameHeader_advanced`.
#[test]
fn frame_inspection_unknown_and_huge_content_size() {
    covers(&[
        "CFG:122",
        "CFG:225",
        "CFG:229",
        "CFG:235",
        "CFG:165",
        "ERR:decompress/zstd_decompress.c:498",
        "ERR:decompress/zstd_decompress.c:762",
    ]);

    let payload = corpus(Corpus::Text, 1000, 0x2501);
    for csf in [0, 1] {
        for cks in [0, 1] {
            for did in [0, 1] {
                for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                    for dict in [false, true] {
                        // Unknown content size, windowLog 10 and 17 (a
                        // windowLog-27 unknown-size frame would allocate a
                        // 128 MB window; two of those are built below).
                        for wlog in [10, 17] {
                            let sp = Spec {
                                csf,
                                cks,
                                did,
                                fmt,
                                wlog,
                                kind: Kind::Unknown(1000),
                                dict,
                            };
                            let frame = frame_fixture(sp, &payload);
                            let lab = sp.label();
                            diff(&format!("prefix[{lab}]"), |l| {
                                let mut v = Vec::new();
                                for n in prefix_lens(frame.len()) {
                                    v.push((
                                        n,
                                        fh(l, &frame, n),
                                        fh_adv(l, &frame, n, ZSTD_f_zstd1_magicless),
                                    ));
                                }
                                v
                            });
                            diff(&format!("whole[{lab}]"), |l| whole_probe(l, &frame));
                        }
                        // Header-only fixtures with a 4 GB-ish pledged size:
                        // 0xFFFFFFFF picks fcsID 2, 0x100000000 picks fcsID 3.
                        for p in [0xFFFF_FFFFu64, 0x1_0000_0000u64] {
                            let sp = Spec {
                                csf,
                                cks,
                                did,
                                fmt,
                                wlog: 10,
                                kind: Kind::Pledged(p),
                                dict,
                            };
                            let frame = frame_fixture(sp, &payload[..8]);
                            let lab = sp.label();
                            diff(&format!("prefix[{lab}]"), |l| {
                                let mut v = Vec::new();
                                for n in prefix_lens(frame.len().min(24)) {
                                    v.push((
                                        n,
                                        fh(l, &frame, n),
                                        fh_adv(l, &frame, n, ZSTD_f_zstd1_magicless),
                                    ));
                                }
                                v
                            });
                        }
                    }
                }
            }
        }
    }

    // Two real windowLog-27 frames with an unknown content size, so that
    // ZSTD_decompressBound / ZSTD_decompressionMargin /
    // ZSTD_estimateDStreamSize_fromFrame see a 128 MB window for real.
    for cks in [0, 1] {
        let sp = Spec {
            csf: 1,
            cks,
            did: 1,
            fmt: ZSTD_f_zstd1,
            wlog: 27,
            kind: Kind::Unknown(1000),
            dict: false,
        };
        let frame = frame_fixture(sp, &payload);
        let lab = sp.label();
        diff(&format!("whole[{lab}]"), |l| whole_probe(l, &frame));
        diff(&format!("prefix[{lab}]"), |l| {
            let mut v = Vec::new();
            for n in prefix_lens(frame.len().min(24)) {
                v.push((n, fh(l, &frame, n)));
            }
            v
        });
    }
}

// ===========================================================================
// 3. Skippable frames
// ===========================================================================

/// Hand-build a skippable frame header: magic + 32-bit little-endian length.
fn skippable(magic_variant: u32, declared_len: u32, content: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(8 + content.len());
    v.extend_from_slice(&(ZSTD_MAGIC_SKIPPABLE_START + magic_variant).to_le_bytes());
    v.extend_from_slice(&declared_len.to_le_bytes());
    v.extend_from_slice(content);
    v
}

/// `ZSTD_writeSkippableFrame` (`zstd_compress.c:4751`) and
/// `ZSTD_readSkippableFrame` (`zstd_decompress.c:614`).
///
/// The three `RETURN_ERROR_IF`s of the writer fire in a fixed order
/// (`dstCapacity < srcSize+8` -> `dstSize_tooSmall`, `srcSize > 0xFFFFFFFF` ->
/// `srcSize_wrong`, `magicVariant > 15` -> `parameter_outOfBound`), so each is
/// probed with the earlier conditions satisfied. The reader's guards are checked
/// the same way, including the case where `readSkippableFrameSize`'s *error code*
/// leaks into `skippableFrameSize` and is caught only by the
/// `skippableFrameSize > srcSize` test — which is why a 32-bit-overflowing length
/// field surfaces as `srcSize_wrong` from the reader but as
/// `frameParameter_unsupported` from `ZSTD_findFrameCompressedSize`.
#[test]
fn skippable_frame_write_read_roundtrip() {
    covers(&[
        "CFG:226",
        "CFG:230",
        "CFG:232",
        "CFG:233",
        "CFG:234",
        "ERR:compress/zstd_compress.c:4754",
        "ERR:compress/zstd_compress.c:4756",
        "ERR:compress/zstd_compress.c:4757",
        "ERR:decompress/zstd_decompress.c:592",
        "ERR:decompress/zstd_decompress.c:595",
        "ERR:decompress/zstd_decompress.c:598",
        "ERR:decompress/zstd_decompress.c:618",
        "ERR:decompress/zstd_decompress.c:625",
        "ERR:decompress/zstd_decompress.c:626",
        "ERR:decompress/zstd_decompress.c:627",
    ]);

    const CONTENT_SIZES: &[usize] = &[0, 1, 100, 65536];

    // --- writer: all 16 valid variants x 4 content sizes x 3 capacities ------
    for mv in 0u32..=15 {
        for &cs in CONTENT_SIZES {
            let content = corpus(Corpus::Mixed, cs, 0x5C1B_u64 ^ cs as u64);
            for extra in [7usize, 8, 100] {
                let cap = cs + extra;
                let out = diff_bytes(&format!("write/mv{mv}/cs{cs}/cap+{extra}"), |l| {
                    let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
                    let mut dst = vec![0xCDu8; cap + 16];
                    let n = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr(&content),
                            content.len(),
                            mv,
                        )
                    };
                    (res(l, n), Blob(dst))
                });
                if extra == 7 {
                    // dstCapacity is exactly one byte short of srcSize+8.
                    assert!(matches!(out.0, R::Err(..)));
                    continue;
                }
                let frame = &out.1 .0[..cs + 8];
                assert_eq!(out.0, R::Ok(cs + 8));
                assert_eq!(
                    u32::from_le_bytes(frame[0..4].try_into().unwrap()),
                    ZSTD_MAGIC_SKIPPABLE_START + mv
                );

                // --- reader round-trip over the frame just written -----------
                diff_bytes(&format!("read/mv{mv}/cs{cs}/cap+{extra}"), |l| {
                    let f = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
                    let mut dst = vec![0xCDu8; cs + 16];
                    let mut mvout: c_uint = 0xDEAD_BEEF;
                    let n = unsafe {
                        f(
                            dst.as_mut_ptr() as *mut c_void,
                            cs + 16,
                            &mut mvout,
                            ptr(frame),
                            frame.len(),
                        )
                    };
                    (res(l, n), mvout, Blob(dst))
                });
                assert_eq!(&out.1 .0[8..8 + cs], &content[..]);
            }
        }
    }

    // --- writer: the two later guards ---------------------------------------
    diff("write/mv-out-of-bound", |l| {
        let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
        let src = [1u8, 2, 3, 4];
        let mut dst = vec![0xCDu8; 128];
        let mut v = Vec::new();
        for mv in [16u32, 17, 255, 0xFFFF_FFFF] {
            let n = unsafe {
                f(dst.as_mut_ptr() as *mut c_void, 64, ptr(&src), src.len(), mv)
            };
            v.push((mv, res(l, n), dst[0], dst[63]));
        }
        v
    });
    // srcSize > 0xFFFFFFFF. The C returns before touching either buffer (the
    // dstCapacity guard is satisfied by the nominal capacity, then srcSize is
    // rejected), so a nominal size over a small real buffer is safe -- this is
    // exactly the reproducer ERRORS.md row 193 records.
    diff("write/srcSize-over-4G", |l| {
        let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
        let src = [0u8; 16];
        let mut dst = vec![0xCDu8; 64];
        let n = unsafe {
            f(
                dst.as_mut_ptr() as *mut c_void,
                0x1_0000_0008usize,
                ptr(&src),
                0x1_0000_0000usize,
                0,
            )
        };
        (res(l, n), dst[0])
    });

    // --- reader: every rejection and the NULL-argument allowances ------------
    let content100 = corpus(Corpus::Text, 100, 0x5100);
    let good = skippable(3, 100, &content100);
    let real_frame = c_compress(&content100, 3);

    let cases: Vec<(String, Vec<u8>, usize, bool)> = vec![
        // (label, src, dstCapacity, pass_null_magicVariant)
        ("exact".into(), good.clone(), 100, false),
        ("magicVariant-NULL".into(), good.clone(), 100, true),
        ("cap-one-short".into(), good.clone(), 99, false),
        ("cap-zero".into(), good.clone(), 0, false),
        ("cap-huge".into(), good.clone(), 4096, false),
        ("trunc7".into(), good[..7].to_vec(), 100, false),
        ("trunc4".into(), good[..4].to_vec(), 100, false),
        ("trunc0".into(), Vec::new(), 100, false),
        ("not-skippable".into(), real_frame.clone(), 4096, false),
        ("declared-too-big".into(), skippable(0, 200, &content100), 4096, false),
        (
            "declared-0xFFFFFFF7".into(),
            skippable(0, 0xFFFF_FFF7, &content100),
            4096,
            false,
        ),
        (
            "declared-0xFFFFFFF8".into(),
            skippable(0, 0xFFFF_FFF8, &content100),
            4096,
            false,
        ),
        (
            "declared-0xFFFFFFFF".into(),
            skippable(0, 0xFFFF_FFFF, &content100),
            4096,
            false,
        ),
        ("declared-0".into(), skippable(7, 0, &[]), 4096, false),
    ];

    for (label, src, cap, null_mv) in &cases {
        diff_bytes(&format!("read/{label}"), |l| {
            let f = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
            let mut dst = vec![0xCDu8; 4096 + 16];
            let mut mvout: c_uint = 0xDEAD_BEEF;
            let n = unsafe {
                f(
                    dst.as_mut_ptr() as *mut c_void,
                    *cap,
                    if *null_mv {
                        std::ptr::null_mut()
                    } else {
                        &mut mvout
                    },
                    ptr(src),
                    src.len(),
                )
            };
            (res(l, n), mvout, Blob(dst))
        });
        // dst == NULL with enough capacity is documented as legal: the size is
        // returned and nothing is written.
        diff(&format!("read-nulldst/{label}"), |l| {
            let f = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
            let mut mvout: c_uint = 0xDEAD_BEEF;
            let n = unsafe {
                f(
                    std::ptr::null_mut(),
                    *cap,
                    &mut mvout,
                    ptr(src),
                    src.len(),
                )
            };
            (res(l, n), mvout)
        });
    }
}

/// The recognition and sizing side of skippable frames:
/// `ZSTD_isSkippableFrame`, `ZSTD_isFrame`, `ZSTD_getFrameHeader`'s skippable
/// branch (`zstd_decompress.c:482-491`, which fills `dictID = magic -
/// ZSTD_MAGIC_SKIPPABLE_START` and leaves windowSize/blockSizeMax/checksumFlag
/// zero), and `readSkippableFrameSize` as reached through
/// `ZSTD_findFrameCompressedSize` / `ZSTD_findDecompressedSize` /
/// `ZSTD_decompressBound` / `ZSTD_decompressionMargin`.
#[test]
fn skippable_frame_recognition_and_sizing() {
    covers(&[
        "CFG:226",
        "CFG:229",
        "CFG:230",
        "CFG:231",
        "CFG:232",
        "CFG:235",
        "CFG:78",
        "ERR:decompress/zstd_decompress.c:485",
        "ERR:decompress/zstd_decompress.c:581",
        "ERR:decompress/zstd_decompress.c:592",
        "ERR:decompress/zstd_decompress.c:595",
        "ERR:decompress/zstd_decompress.c:598",
        "ERR:decompress/zstd_decompress.c:652",
    ]);

    // All 16 magics x 4 declared content sizes, plus the two magics just
    // outside the skippable range.
    for mv in 0u32..=15 {
        for &cs in &[0usize, 1, 100, 0xFFFF_FFFF] {
            let content = if cs == 0xFFFF_FFFF {
                corpus(Corpus::Counter, 100, 1)
            } else {
                corpus(Corpus::Counter, cs, 1)
            };
            let declared = if cs == 0xFFFF_FFFF {
                0xFFFF_FFFFu32
            } else {
                cs as u32
            };
            let frame = skippable(mv, declared, &content);
            let lab = format!("mv{mv}/cs{cs}");
            diff(&format!("skip-header[{lab}]"), |l| {
                let mut v = Vec::new();
                for n in [0usize, 1, 3, 4, 5, 6, 7, 8, 9, frame.len()] {
                    if n > frame.len() {
                        continue;
                    }
                    v.push((n, fh(l, &frame, n)));
                }
                v
            });
            diff(&format!("skip-whole[{lab}]"), |l| whole_probe(l, &frame));
            // Truncations to 5, 6 and 7 bytes fall out of the skippable fast
            // path in ZSTD_findFrameSizeInfo (which needs srcSize >= 8).
            for n in [4usize, 5, 6, 7] {
                let t = frame[..n.min(frame.len())].to_vec();
                diff(&format!("skip-trunc[{lab}/{n}]"), |l| whole_probe(l, &t));
            }
        }
    }

    // The magic just below and just above the skippable range, and the zstd
    // magic itself, through both predicates.
    diff("skip-boundary", |l| {
        let iss = l.sym::<FnPtrLenU32>("ZSTD_isSkippableFrame");
        let isf = l.sym::<FnPtrLenU32>("ZSTD_isFrame");
        let mut v = Vec::new();
        for m in [
            ZSTD_MAGIC_SKIPPABLE_START - 1,
            ZSTD_MAGIC_SKIPPABLE_START,
            ZSTD_MAGIC_SKIPPABLE_START + 15,
            ZSTD_MAGIC_SKIPPABLE_START + 16,
            ZSTD_MAGICNUMBER,
        ] {
            let mut b = m.to_le_bytes().to_vec();
            b.extend_from_slice(&[0u8; 8]);
            for n in [0usize, 1, 2, 3, 4, 12] {
                v.push((m, n, unsafe { iss(ptr(&b), n) }, unsafe {
                    isf(ptr(&b), n)
                }));
            }
        }
        v
    });
}

// ===========================================================================
// 4. Multi-frame inputs
// ===========================================================================

/// Drive `ZSTD_decompressStream` to completion over `src`, making `in_chunk`
/// more bytes available whenever the decoder has consumed everything it was
/// given, and record every return value plus the output.
///
/// Termination: a `0` return with all of the input consumed means the last frame
/// ended; otherwise the loop stops as soon as a call makes no progress and there
/// is no more input left to offer, which is exactly the "no forward progress"
/// condition the streaming API itself detects.
fn dstream_all(l: &Lib, src: &[u8], out_cap: usize, in_chunk: usize) -> (Vec<R>, Blob) {
    assert!(in_chunk > 0);
    let ds = Ctx::dstream(l);
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut out = vec![0xCDu8; out_cap];
    let mut rets = Vec::new();
    let mut inb = ZSTD_inBuffer {
        src: ptr(src),
        size: 0,
        pos: 0,
    };
    let mut outb = ZSTD_outBuffer {
        dst: out.as_mut_ptr() as *mut c_void,
        size: out_cap,
        pos: 0,
    };
    let budget = 4 * (src.len() / in_chunk + 8) + 1024;
    let mut offered = 0usize;
    for i in 0..budget {
        if inb.pos == inb.size && offered < src.len() {
            offered = (offered + in_chunk).min(src.len());
            inb.size = offered;
        }
        let before = (inb.pos, outb.pos);
        let n = unsafe { f(ds.ptr, &mut outb, &mut inb) };
        let r = res(l, n);
        rets.push(r.clone());
        if let R::Err(..) = r {
            break;
        }
        let all_input_seen = offered >= src.len() && inb.pos >= src.len();
        if matches!(r, R::Ok(0)) && all_input_seen {
            break;
        }
        if (inb.pos, outb.pos) == before && offered >= src.len() {
            // No progress and nothing more to offer.
            break;
        }
        if outb.pos == outb.size && !matches!(r, R::Ok(0)) {
            // Output buffer exhausted mid-frame.
            break;
        }
        if i + 1 == budget {
            rets.push(R::Err(-1, "<runaway>".into()));
        }
    }
    out.truncate(outb.pos);
    (rets, Blob(out))
}

/// Splice a frame so its header declares an 8-byte (`fcsID == 3`)
/// frameContentSize of `fcs` while the block data is unchanged. The input must be
/// a `contentSizeFlag == 0`, `checksumFlag == 0`, non-singleSegment,
/// dictID-less frame, whose header is therefore exactly
/// `magic(4) + FHD(1) + windowDescriptor(1)`.
fn relabel_content_size(frame: &[u8], fcs: u64) -> Vec<u8> {
    assert_eq!(frame[4] & 0b1110_0011, 0, "expected FHD == 0x00");
    let mut v = Vec::with_capacity(frame.len() + 8);
    v.extend_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
    v.push(0b1100_0000); // fcsID = 3, singleSegment = 0, no checksum, no dictID
    v.push(frame[5]); // keep the original window descriptor
    v.extend_from_slice(&fcs.to_le_bytes());
    v.extend_from_slice(&frame[6..]);
    v
}

/// 1..5 concatenated frames, with skippable frames interleaved before, between
/// and after, plus trailing garbage and a truncated tail frame.
///
/// Targets `ZSTD_decompressMultiFrame`'s loop (`zstd_decompress.c:1080`+),
/// `ZSTD_findDecompressedSize` (`:643`, including its `totalDstSize + fcs <
/// totalDstSize` overflow guard at `:663`), `ZSTD_decompressBound` (`:820`) and
/// `ZSTD_decompressionMargin` (`:838`, whose `maxBlockSize` term is added exactly
/// once at the very end and so is only observable with mixed windowLogs).
#[test]
fn multi_frame_inputs() {
    covers(&[
        "CFG:81",
        "CFG:229",
        "CFG:230",
        "CFG:235",
        "CFG:83",
        "ERR:decompress/zstd_decompress.c:652",
        "ERR:decompress/zstd_decompress.c:661",
        "ERR:decompress/zstd_decompress.c:664",
        "ERR:decompress/zstd_decompress.c:669",
        "ERR:decompress/zstd_decompress.c:677",
        "ERR:decompress/zstd_decompress.c:828",
        "ERR:decompress/zstd_decompress.c:852",
    ]);

    // Building blocks: frames of differing size / checksum / windowLog, and
    // skippable frames of two different magics.
    let mk = |nb: usize, cks: c_int, wlog: c_int, csf: c_int| -> (Vec<u8>, usize) {
        let payload = corpus(Corpus::Text, nb, 0x81_0000 ^ nb as u64);
        let sp = Spec {
            csf,
            cks,
            did: 1,
            fmt: ZSTD_f_zstd1,
            wlog,
            kind: Kind::Known(nb),
            dict: false,
        };
        (frame_fixture(sp, &payload), nb)
    };
    let f_small = mk(300, 0, 11, 1);
    let f_ck = mk(5000, 1, 17, 1);
    let f_nocs = mk(4000, 0, 17, 0);
    let f_empty = mk(0, 1, 11, 1);
    let f_big = mk(200_000, 1, 17, 1);
    let sk0 = skippable(0, 0, &[]);
    let sk100 = skippable(15, 100, &corpus(Corpus::Counter, 100, 3));

    // A frame with a lying (huge) declared content size, used twice to make
    // ZSTD_findDecompressedSize's 64-bit overflow guard fire. The base frame is
    // built with contentSizeFlag=0 and checksumFlag=0 so its header is exactly
    // 6 bytes and can be replaced wholesale.
    let base_nocs = {
        let payload = corpus(Corpus::Text, 2000, 0x1EE);
        let sp = Spec {
            csf: 0,
            cks: 0,
            did: 1,
            fmt: ZSTD_f_zstd1,
            wlog: 17,
            kind: Kind::Known(2000),
            dict: false,
        };
        frame_fixture(sp, &payload)
    };
    let lying = relabel_content_size(&base_nocs, 0xF000_0000_0000_0000);

    let mut cases: Vec<(String, Vec<u8>, usize)> = Vec::new();
    let mut push = |name: &str, parts: &[&Vec<u8>], plain: usize| {
        let mut v = Vec::new();
        for p in parts {
            v.extend_from_slice(p);
        }
        cases.push((name.to_string(), v, plain));
    };

    push("1frame", &[&f_small.0], 300);
    push("2frames", &[&f_small.0, &f_ck.0], 5300);
    push("3frames-mixed", &[&f_small.0, &f_ck.0, &f_nocs.0], 9300);
    push(
        "4frames",
        &[&f_small.0, &f_ck.0, &f_nocs.0, &f_empty.0],
        9300,
    );
    push(
        "5frames",
        &[&f_small.0, &f_ck.0, &f_nocs.0, &f_empty.0, &f_big.0],
        209_300,
    );
    push("skip-first", &[&sk100, &f_small.0], 300);
    push("skip-between", &[&f_small.0, &sk100, &f_ck.0], 5300);
    push("skip-last", &[&f_small.0, &sk0], 300);
    push("skip-only", &[&sk100], 0);
    push("skip-only-empty", &[&sk0], 0);
    push("skip-skip-frame", &[&sk0, &sk100, &f_small.0], 300);
    push("empty-frames-x3", &[&f_empty.0, &f_empty.0, &f_empty.0], 0);
    push("mixed-wlog", &[&f_small.0, &f_big.0, &f_small.0], 200_600);
    push("lying-x1", &[&lying], 2000);
    push("lying-x2", &[&lying, &lying], 2000);

    // Trailing garbage of 1..5 bytes, and a truncated second frame.
    for k in 1..=5usize {
        let mut v = f_small.0.clone();
        v.extend_from_slice(&[0xAAu8; 5][..k]);
        cases.push((format!("trailing{k}"), v, 300));
    }
    for cut in [1usize, 4, 5, 6, 9] {
        let mut v = f_small.0.clone();
        v.extend_from_slice(&f_ck.0[..cut.min(f_ck.0.len())]);
        cases.push((format!("trunc-second-{cut}"), v, 300));
    }
    {
        let mut v = f_small.0.clone();
        v.truncate(v.len() - 1);
        cases.push(("trunc-only-frame".into(), v, 300));
        cases.push(("three-bytes".into(), vec![0x28, 0xB5, 0x2F], 0));
        cases.push((
            "skip-trunc-tail".into(),
            {
                let mut w = f_small.0.clone();
                w.extend_from_slice(&sk100[..6]);
                w
            },
            300,
        ));
    }

    for (name, input, plain) in &cases {
        diff(&format!("mf-whole[{name}]"), |l| whole_probe(l, input));
        // one-shot, both through ZSTD_decompress and ZSTD_decompressDCtx
        diff_bytes(&format!("mf-oneshot[{name}]"), |l| {
            decompress_simple(l, input, plain + 64)
        });
        diff_bytes(&format!("mf-dctx[{name}]"), |l| {
            let d = Ctx::dctx(l);
            let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let mut dst = vec![0xCDu8; plain + 64];
            let n = unsafe {
                f(
                    d.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr(input),
                    input.len(),
                )
            };
            let r = res(l, n);
            (r, Blob(dst))
        });
        // streaming, three chunk sizes
        let oneshot = decompress_simple(&pair().c, input, plain + 64);
        for chunk in [1usize, 13, 7919] {
            let (_, got) = diff_bytes(&format!("mf-stream[{name}/{chunk}]"), |l| {
                dstream_all(l, input, plain + 64, chunk)
            });
            // Where the one-shot decode succeeded, the streamed decode must have
            // produced exactly the same bytes -- otherwise the driver above would
            // be silently bailing out early on both libraries at once.
            if let R::Ok(_) = oneshot.0 {
                assert_eq!(
                    got.0, oneshot.1 .0,
                    "[{name}/{chunk}] streamed output differs from the one-shot output"
                );
            }
        }
    }
}

// ===========================================================================
// 5. The low-level block API
// ===========================================================================

/// `ZSTD_getBlockSize` (`zstd_compress.c:4877`) reads the *applied* parameters,
/// so it is `MIN(0, 1<<0) == 0` before any `ZSTD_compressBegin*` and
/// `MIN(maxBlockSize, 1 << windowLog)` afterwards. The `assert(!ZSTD_checkCParams)`
/// above it is compiled out at `DEBUGLEVEL=0`, so the never-initialised case
/// returns 0 rather than trapping.
#[test]
fn block_api_get_block_size() {
    covers(&[
        "CFG:210",
        "ERR:compress/zstd_compress.c:4877",
    ]);
    diff("getBlockSize/fresh", |l| {
        let c = Ctx::cctx(l);
        let g = l.sym::<FnCtxSz>("ZSTD_getBlockSize");
        unsafe { g(c.ptr) }
    });
    for lvl in [1, 3, 19] {
        diff(&format!("getBlockSize/begin{lvl}"), |l| {
            let c = Ctx::cctx(l);
            let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
            let g = l.sym::<FnCtxSz>("ZSTD_getBlockSize");
            let before = unsafe { g(c.ptr) };
            let r = res(l, unsafe { b(c.ptr, lvl) });
            (before, r, unsafe { g(c.ptr) })
        });
    }
    for wlog in [10, 11, 17] {
        for mbs in [0, 1024, 4096, 131072] {
            diff(&format!("getBlockSize/wlog{wlog}/mbs{mbs}"), |l| {
                let c = Ctx::cctx(l);
                let r1 = setp(l, c.ptr, ZSTD_c_windowLog, wlog);
                let r2 = setp(l, c.ptr, ZSTD_c_maxBlockSize, mbs);
                let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
                let g = l.sym::<FnCtxSz>("ZSTD_getBlockSize");
                // ZSTD_compressBegin re-initialises the parameters from the
                // level, so the windowLog/maxBlockSize set above are applied
                // through the *advanced* path only; both are recorded either way.
                let r3 = res(l, unsafe { b(c.ptr, 3) });
                (r1, r2, r3, unsafe { g(c.ptr) })
            });
        }
    }
}

/// The documented manual block protocol
/// (`zstd.h:3160`+): `ZSTD_compressBegin` / `ZSTD_getBlockSize` /
/// `ZSTD_compressBlock` on the encoder side, and `ZSTD_decompressBegin` /
/// `ZSTD_decompressBlock` / `ZSTD_insertBlock` on the decoder side, where a
/// `ZSTD_compressBlock` return of 0 means "not compressible, deal with the raw
/// bytes yourself" and the decoder must be told about those raw bytes via
/// `ZSTD_insertBlock` so that `ZSTD_checkContinuity` keeps
/// `prefixStart`/`dictEnd` consistent.
///
/// Both the compressed bytes and the fully reassembled output are compared.
#[test]
fn block_api_manual_roundtrip() {
    covers(&[
        "CFG:211",
        "CFG:212",
        "CFG:213",
        "CFG:224",
        "ERR:compress/zstd_compress.c:4887",
        "ERR:decompress/zstd_decompress_block.c:2086",
        "ERR:decompress/zstd_decompress_block.c:2125",
        "ERR:decompress/zstd_decompress_block.c:2197",
    ]);

    // Block layouts to drive. Each entry is a list of (corpus, len) pieces; the
    // random pieces come back from ZSTD_compressBlock as 0 (incompressible).
    let layouts: Vec<(&str, Vec<(Corpus, usize)>)> = vec![
        (
            "16k-alternating",
            vec![
                (Corpus::Text, 16384),
                (Corpus::Random, 16384),
                (Corpus::Text, 16384),
                (Corpus::Random, 16384),
            ],
        ),
        (
            "exact-blocksize",
            vec![(Corpus::Text, 131072), (Corpus::Text, 131072)],
        ),
        (
            "tiny",
            vec![
                (Corpus::Text, 1),
                (Corpus::Text, 6),
                (Corpus::Text, 7),
                (Corpus::Text, 8),
                (Corpus::Zeros, 26),
                (Corpus::Zeros, 27),
            ],
        ),
        (
            "backref",
            vec![
                (Corpus::Text, 16384),
                (Corpus::Random, 16384),
                (Corpus::Text, 16384),
            ],
        ),
        (
            "mixed-sizes",
            vec![
                (Corpus::Zeros, 100),
                (Corpus::Periodic, 5000),
                (Corpus::Random, 300),
                (Corpus::LongRepeats, 40000),
                (Corpus::SmallAlphabet, 12345),
            ],
        ),
    ];

    for lvl in [1, 3, 19] {
        for (name, pieces) in &layouts {
            let mut plain = Vec::new();
            let mut bounds = Vec::new();
            for (i, (c, n)) in pieces.iter().enumerate() {
                // "backref" deliberately repeats piece 0 as piece 2 (same seed)
                // so that the third block matches across the *inserted* raw
                // block in between -- which only decodes correctly if
                // ZSTD_insertBlock kept prefixStart/dictEnd consistent.
                let seed = if *name == "backref" && i == 2 { 0xB10C } else { 0xB10C ^ i as u64 };
                let d = corpus(*c, *n, seed);
                bounds.push((plain.len(), d.len()));
                plain.extend_from_slice(&d);
            }
            diff_bytes(&format!("blockrt[{name}/lvl{lvl}]"), |l| {
                let cctx = Ctx::cctx(l);
                let beg = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
                let gbs = l.sym::<FnCtxSz>("ZSTD_getBlockSize");
                let cb = l.sym::<FnBlock5>("ZSTD_compressBlock");
                let mut steps: Vec<R> = Vec::new();
                steps.push(res(l, unsafe { beg(cctx.ptr, lvl) }));
                let bs = unsafe { gbs(cctx.ptr) };
                steps.push(R::Ok(bs));

                let mut cbuf: Vec<Vec<u8>> = Vec::new();
                let mut all_c = Vec::new();
                for &(off, len) in &bounds {
                    let cap = compress_bound(l, len) + 64;
                    let mut dst = vec![0xCDu8; cap];
                    let n = unsafe {
                        cb(
                            cctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr(&plain[off..off + len]),
                            len,
                        )
                    };
                    let r = res(l, n);
                    steps.push(r.clone());
                    let taken = if let R::Ok(k) = r { k } else { 0 };
                    all_c.extend_from_slice(&dst[..taken]);
                    dst.truncate(taken);
                    cbuf.push(dst);
                }

                // --- decode -------------------------------------------------
                let dctx = Ctx::dctx(l);
                let dbeg = l.sym::<FnCtxSz>("ZSTD_decompressBegin");
                let db = l.sym::<FnBlock5>("ZSTD_decompressBlock");
                let ib = l.sym::<FnInsertBlock>("ZSTD_insertBlock");
                steps.push(res(l, unsafe { dbeg(dctx.ptr) }));
                let mut out = vec![0xCDu8; plain.len()];
                for (i, &(off, len)) in bounds.iter().enumerate() {
                    if cbuf[i].is_empty() {
                        // Incompressible: the caller owns the raw bytes.
                        out[off..off + len].copy_from_slice(&plain[off..off + len]);
                        let n = unsafe {
                            ib(dctx.ptr, out[off..].as_ptr() as *const c_void, len)
                        };
                        steps.push(res(l, n));
                    } else {
                        let n = unsafe {
                            db(
                                dctx.ptr,
                                out[off..].as_mut_ptr() as *mut c_void,
                                plain.len() - off,
                                ptr(&cbuf[i]),
                                cbuf[i].len(),
                            )
                        };
                        steps.push(res(l, n));
                    }
                }
                let mut blob = all_c;
                blob.extend_from_slice(&out);
                (steps, Blob(blob))
            });
        }
    }
}

/// The error surface of the block API.
///
/// * `ZSTD_compressBlock` with `srcSize > ZSTD_getBlockSize()` ->
///   `srcSize_wrong` (`zstd_compress.c:4887`);
/// * `ZSTD_compressBlock` with no preceding `compressBegin` -> `stage_wrong`
///   (`zstd_compress.c:4802`, shared with `compressContinue`/`compressEnd`);
/// * a `dstCapacity` too small for the entropy-coded block: in *block* mode there
///   is no 6-byte block-header reservation, and
///   `ZSTD_entropyCompressSeqStore`'s `dstSize_tooSmall` is converted to "not
///   compressible" (return 0) whenever `blockSize <= dstCapacity`
///   (`zstd_compress.c:3026`), so only `dstCapacity < srcSize` can surface it;
/// * `ZSTD_decompressBlock` with `srcSize > ZSTD_blockSizeMax` (131072, fixed
///   because `isFrameDecompression` is cleared) -> `srcSize_wrong`;
/// * `ZSTD_decompressBlock` with `dst == NULL` / `dstCapacity == 0` on a block
///   that carries sequences -> `dstSize_tooSmall`.
#[test]
fn block_api_error_paths() {
    covers(&[
        "CFG:211",
        "CFG:213",
        "ERR:compress/zstd_compress.c:4887",
        "ERR:compress/zstd_compress.c:4802",
        "ERR:decompress/zstd_decompress_block.c:2081",
        "ERR:decompress/zstd_decompress_block.c:2129",
        "ERR:decompress/zstd_decompress_block.c:2197",
    ]);

    let big = corpus(Corpus::Text, 131073 + 16, 0xE55);

    // srcSize sweep around ZSTD_getBlockSize().
    diff_bytes("cb/srcSize-sweep", |l| {
        let cctx = Ctx::cctx(l);
        let beg = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
        let gbs = l.sym::<FnCtxSz>("ZSTD_getBlockSize");
        let cb = l.sym::<FnBlock5>("ZSTD_compressBlock");
        let mut steps = Vec::new();
        steps.push(res(l, unsafe { beg(cctx.ptr, 3) }));
        let bs = unsafe { gbs(cctx.ptr) };
        steps.push(R::Ok(bs));
        let cap = compress_bound(l, bs + 8) + 64;
        let mut dst = vec![0xCDu8; cap];
        let mut blob = Vec::new();
        for n in [0usize, 1, 6, 7, 8, bs - 1, bs, bs + 1, bs + 2] {
            for b in dst.iter_mut() {
                *b = 0xCD;
            }
            let r = unsafe {
                cb(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr(&big),
                    n,
                )
            };
            let r = res(l, r);
            if let R::Ok(k) = r {
                blob.extend_from_slice(&dst[..k]);
            }
            steps.push(r);
            // A failed ZSTD_compressBlock leaves the ctx usable per the docs
            // only for the srcSize_wrong case (it errors before touching state),
            // so re-begin between probes to keep the sequence deterministic.
            steps.push(res(l, unsafe { beg(cctx.ptr, 3) }));
        }
        (steps, Blob(blob))
    });

    // dstCapacity sweep on a highly compressible block.
    let z = corpus(Corpus::Zeros, 4096, 1);
    let t = corpus(Corpus::Text, 4096, 2);
    for (nm, src) in [("zeros", &z), ("text", &t)] {
        diff_bytes(&format!("cb/dstCap-{nm}"), |l| {
            let cb = l.sym::<FnBlock5>("ZSTD_compressBlock");
            let beg = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
            let mut steps = Vec::new();
            let mut blob = Vec::new();
            for cap in [0usize, 1, 2, 3, 5, 6, 7, 16, 64, 4095, 4096, 8192] {
                let cctx = Ctx::cctx(l);
                steps.push(res(l, unsafe { beg(cctx.ptr, 3) }));
                let mut dst = vec![0xCDu8; cap + 32];
                let r = unsafe {
                    cb(
                        cctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        ptr(src),
                        src.len(),
                    )
                };
                let r = res(l, r);
                if let R::Ok(k) = r {
                    blob.extend_from_slice(&dst[..k]);
                }
                steps.push(r);
                // the guard bytes past `cap` must never be touched
                blob.extend_from_slice(&dst[cap..]);
            }
            (steps, Blob(blob))
        });
    }

    // compressBlock / compressContinue / compressEnd with no compressBegin.
    diff("cb/no-begin", |l| {
        let cb = l.sym::<FnBlock5>("ZSTD_compressBlock");
        let cc = l.sym::<FnBlock5>("ZSTD_compressContinue");
        let ce = l.sym::<FnBlock5>("ZSTD_compressEnd");
        let mut v = Vec::new();
        for (i, f) in [cb, cc, ce].iter().enumerate() {
            let cctx = Ctx::cctx(l);
            let mut dst = vec![0xCDu8; 4096];
            let r = unsafe {
                f(
                    cctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr(&z),
                    16,
                )
            };
            v.push((i, res(l, r), dst[0]));
        }
        v
    });

    // decompressBlock: the srcSize boundary and the NULL/zero-capacity guards.
    // A *real* compressed block body (the output of ZSTD_compressBlock) is used
    // for the NULL/zero-capacity probes so that nbSeq > 0.
    let cblock = {
        let l = &pair().c;
        let cctx = Ctx::cctx(l);
        let beg = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
        let cb = l.sym::<FnBlock5>("ZSTD_compressBlock");
        unsafe { beg(cctx.ptr, 3) };
        let src = corpus(Corpus::Text, 8192, 9);
        let mut dst = vec![0u8; compress_bound(l, src.len()) + 64];
        let n = unsafe {
            cb(
                cctx.ptr,
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                ptr(&src),
                src.len(),
            )
        };
        assert!(!is_error(l, n) && n > 1, "expected a compressible block");
        dst.truncate(n);
        dst
    };

    diff_bytes("db/errors", |l| {
        let dbeg = l.sym::<FnCtxSz>("ZSTD_decompressBegin");
        let db = l.sym::<FnBlock5>("ZSTD_decompressBlock");
        let mut steps = Vec::new();
        let mut blob = Vec::new();
        // srcSize == 131072 exactly (allowed by the spec) and 131073 (rejected).
        // 131072 zero bytes decode as: literals header 0x00 (raw, litSize 0),
        // then nbSeq == 0 with 131070 bytes left -> corruption_detected. A fully
        // bounded, deterministic path.
        let zeros = vec![0u8; 131073];
        for n in [131072usize, 131073] {
            let dctx = Ctx::dctx(l);
            steps.push(res(l, unsafe { dbeg(dctx.ptr) }));
            let mut dst = vec![0xCDu8; 1 << 18];
            let r = unsafe {
                db(
                    dctx.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr(&zeros),
                    n,
                )
            };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[..64]);
        }
        // dst == NULL, dstCapacity == 0, and dstCapacity one byte short.
        for (dstnull, cap) in [(true, 0usize), (true, 8192), (false, 0), (false, 1), (false, 8191), (false, 8192)] {
            let dctx = Ctx::dctx(l);
            steps.push(res(l, unsafe { dbeg(dctx.ptr) }));
            let mut dst = vec![0xCDu8; 8192 + 64];
            let p = if dstnull {
                std::ptr::null_mut()
            } else {
                dst.as_mut_ptr() as *mut c_void
            };
            let r = unsafe { db(dctx.ptr, p, cap, ptr(&cblock), cblock.len()) };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[..64]);
        }
        (steps, Blob(blob))
    });

    // ZSTD_insertBlock can never fail: it returns blockSize unconditionally.
    // Probed with blockSize 0 and at an address discontiguous with previousDstEnd.
    diff("insertBlock/never-fails", |l| {
        let dctx = Ctx::dctx(l);
        let dbeg = l.sym::<FnCtxSz>("ZSTD_decompressBegin");
        let ib = l.sym::<FnInsertBlock>("ZSTD_insertBlock");
        let mut buf = vec![0u8; 65536];
        let mut v = Vec::new();
        v.push(res(l, unsafe { dbeg(dctx.ptr) }));
        for (off, len) in [(0usize, 0usize), (0, 100), (100, 100), (40000, 100), (200, 0)] {
            let n = unsafe { ib(dctx.ptr, buf[off..].as_ptr() as *const c_void, len) };
            v.push(res(l, n));
        }
        let _ = &mut buf;
        v
    });
}

// ===========================================================================
// 6. The bufferless compression drivers
// ===========================================================================

/// Drive `compressBegin* / compressContinue* / compressEnd` over `src` in chunks
/// of `chunk` bytes, recording every return value and the produced bytes.
fn drive_continue(
    l: &Lib,
    cctx: *mut c_void,
    src: &[u8],
    chunk: usize,
    cap: usize,
) -> (Vec<R>, Blob) {
    let cc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressContinue");
    let ce: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressEnd");
    let mut dst = vec![0xCDu8; cap];
    let mut steps = Vec::new();
    let mut pos = 0usize;
    let mut opos = 0usize;
    while pos < src.len() {
        let n = chunk.min(src.len() - pos);
        let last = pos + n >= src.len();
        let f = if last { ce } else { cc };
        let r = unsafe {
            f(
                cctx,
                dst[opos..].as_mut_ptr() as *mut c_void,
                cap - opos,
                ptr(&src[pos..pos + n]),
                n,
            )
        };
        let r = res(l, r);
        steps.push(r.clone());
        match r {
            R::Ok(k) => opos += k,
            R::Err(..) => {
                dst.truncate(opos);
                return (steps, Blob(dst));
            }
        }
        pos += n;
    }
    if src.is_empty() {
        // A zero-length input still needs one ZSTD_compressEnd to close the
        // frame; ZSTD_writeEpilogue has a dedicated empty-frame arm that writes
        // the header with pledgedSrcSize and dictID hardcoded to 0.
        let r = unsafe {
            ce(
                cctx,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                ptr(src),
                0,
            )
        };
        let r = res(l, r);
        steps.push(r.clone());
        if let R::Ok(k) = r {
            opos += k;
        }
    }
    dst.truncate(opos);
    (steps, Blob(dst))
}

/// `ZSTD_compressBegin` / `ZSTD_compressBegin_usingDict` /
/// `ZSTD_compressBegin_advanced` followed by `ZSTD_compressContinue` and
/// `ZSTD_compressEnd`, at several chunk sizes.
///
/// The `_advanced` form is additionally compared against `ZSTD_compress2` at the
/// same level: `ZSTD_compressBegin_advanced(cctx, NULL, 0, ZSTD_getParams(lvl,
/// srcSize, 0), srcSize)` plus a single `ZSTD_compressEnd` derives its cParams
/// from the same `ZSTD_getCParams_internal` call that `ZSTD_compress2` uses, so
/// the two are expected to agree byte-for-byte. Rather than assert that (which
/// would encode an assumption), the boolean "are they equal" is itself part of
/// the compared record, so the two libraries must agree about the equivalence.
#[test]
fn compress_begin_continue_end() {
    covers(&[
        "CFG:214",
        "CFG:215",
        "CFG:217",
        "ERR:compress/zstd_compress.c:4802",
        "ERR:compress/zstd_compress.c:4815",
        "ERR:compress/zstd_compress.c:4842",
        "ERR:compress/zstd_compress.c:4623",
        "ERR:compress/zstd_compress.c:4712",
        "ERR:compress/zstd_compress.c:5295",
        "ERR:compress/zstd_compress.c:5365",
        "ERR:compress/zstd_compress.c:5373",
        "ERR:compress/zstd_compress.c:5422",
    ]);

    let src256 = corpus(Corpus::Text, 256 * 1024, 0xBCE);
    let rawd = raw_dict();
    let realdict = dict_fixture();

    // --- ZSTD_compressBegin + continue/end at several chunk sizes ------------
    for lvl in [0, 1, 3, 19] {
        for &chunk in &[1usize, 7, 1000, 131072, 300000] {
            diff_bytes(&format!("begin/lvl{lvl}/chunk{chunk}"), |l| {
                let c = Ctx::cctx(l);
                let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
                let r0 = res(l, unsafe { b(c.ptr, lvl) });
                let cap = compress_bound(l, src256.len()) + 4096;
                let (mut steps, blob) = drive_continue(l, c.ptr, &src256, chunk, cap);
                steps.insert(0, r0);
                (steps, blob)
            });
        }
    }
    // Empty input, and a 1-byte input.
    for (nm, n) in [("empty", 0usize), ("one", 1)] {
        for lvl in [1, 3] {
            let s = corpus(Corpus::Text, n, 5);
            diff_bytes(&format!("begin/{nm}/lvl{lvl}"), |l| {
                let c = Ctx::cctx(l);
                let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
                let r0 = res(l, unsafe { b(c.ptr, lvl) });
                let (mut steps, blob) = drive_continue(l, c.ptr, &s, 16, 4096);
                steps.insert(0, r0);
                (steps, blob)
            });
        }
    }

    // --- ZSTD_compressBegin_usingDict ---------------------------------------
    let dicts: Vec<(&str, Vec<u8>)> = vec![
        ("none", Vec::new()),
        ("7bytes", vec![1u8, 2, 3, 4, 5, 6, 7]),
        ("raw4k", rawd.clone()),
        ("real", realdict.clone()),
    ];
    for (dn, d) in &dicts {
        for lvl in [0, 3] {
            diff_bytes(&format!("beginDict/{dn}/lvl{lvl}"), |l| {
                let c = Ctx::cctx(l);
                let b = l.sym::<FnCompressBeginDict>("ZSTD_compressBegin_usingDict");
                let (dp, ds) = if d.is_empty() {
                    (std::ptr::null(), 0usize)
                } else {
                    (ptr(d), d.len())
                };
                let r0 = res(l, unsafe { b(c.ptr, dp, ds, lvl) });
                let cap = compress_bound(l, src256.len()) + 4096;
                let (mut steps, blob) = drive_continue(l, c.ptr, &src256, 100_000, cap);
                steps.insert(0, r0);
                (steps, blob)
            });
        }
    }

    // --- ZSTD_compressBegin_advanced ----------------------------------------
    let payload = corpus(Corpus::Text, 1 << 20, 0xAD_0007);
    for (nm, pledged) in [
        ("exact", (1u64 << 20)),
        ("unknown", ZSTD_CONTENTSIZE_UNKNOWN),
        ("too-small", 1000u64),
        ("zero", 0u64),
    ] {
        diff_bytes(&format!("beginAdv/{nm}"), |l| {
            let c = Ctx::cctx(l);
            let gp = l.sym::<FnGetParams>("ZSTD_getParams");
            let ba = l.sym::<FnCompressBeginAdv>("ZSTD_compressBegin_advanced");
            let mut p = unsafe { gp(19, 1 << 20, 0) };
            p.fParams.checksumFlag = 1;
            let r0 = res(l, unsafe { ba(c.ptr, std::ptr::null(), 0, p, pledged) });
            let cap = compress_bound(l, payload.len()) + 4096;
            let (mut steps, blob) = drive_continue(l, c.ptr, &payload, 300_000, cap);
            steps.insert(0, r0);
            (steps, blob)
        });
    }
    // windowLog = 40 fails ZSTD_checkCParams inside
    // ZSTD_compressBegin_advanced_internal (zstd_compress.c:5295).
    diff("beginAdv/bad-cparams", |l| {
        let c = Ctx::cctx(l);
        let gp = l.sym::<FnGetParams>("ZSTD_getParams");
        let ba = l.sym::<FnCompressBeginAdv>("ZSTD_compressBegin_advanced");
        let mut v = Vec::new();
        for (wl, st, mm) in [(40u32, 4i32, 4u32), (9, 4, 4), (17, 10, 4), (17, 4, 2), (17, 4, 8)] {
            let mut p = unsafe { gp(3, 1 << 20, 0) };
            p.cParams.windowLog = wl;
            p.cParams.strategy = st;
            p.cParams.minMatch = mm;
            v.push((wl, st, mm, res(l, unsafe {
                ba(c.ptr, std::ptr::null(), 0, p, 1 << 20)
            })));
        }
        v
    });

    // --- equivalence with ZSTD_compress2 ------------------------------------
    for lvl in [1, 3, 9, 19] {
        for nb in [1000usize, 131072, 300000] {
            let s = corpus(Corpus::Text, nb, 0xE07 ^ nb as u64);
            diff_bytes(&format!("equiv/lvl{lvl}/n{nb}"), |l| {
                let cap = compress_bound(l, nb) + 64;
                // (a) bufferless: begin_advanced with the exact pledged size
                let a = {
                    let c = Ctx::cctx(l);
                    let gp = l.sym::<FnGetParams>("ZSTD_getParams");
                    let ba = l.sym::<FnCompressBeginAdv>("ZSTD_compressBegin_advanced");
                    let p = unsafe { gp(lvl, nb as c_ulonglong, 0) };
                    let r0 = res(l, unsafe {
                        ba(c.ptr, std::ptr::null(), 0, p, nb as c_ulonglong)
                    });
                    let (mut st, b) = drive_continue(l, c.ptr, &s, nb.max(1), cap);
                    st.insert(0, r0);
                    (st, b)
                };
                // (b) ZSTD_compress2 at the same level
                let b2 = {
                    let c = Ctx::cctx(l);
                    let _ = setp(l, c.ptr, ZSTD_c_compressionLevel, lvl);
                    let f = l.sym::<FnCompress2>("ZSTD_compress2");
                    let mut dst = vec![0xCDu8; cap];
                    let n = unsafe {
                        f(
                            c.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr(&s),
                            s.len(),
                        )
                    };
                    let r = res(l, n);
                    if let R::Ok(k) = r {
                        dst.truncate(k);
                    }
                    (r, Blob(dst))
                };
                let equal = a.1 == b2.1;
                ((a.0, b2.0, equal), a.1, b2.1)
            });
        }
    }

    // --- stage_wrong, dstCapacity and pledged-size mismatches ---------------
    diff_bytes("cce/errors", |l| {
        let cc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressContinue");
        let ce: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressEnd");
        let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
        let gp = l.sym::<FnGetParams>("ZSTD_getParams");
        let ba = l.sym::<FnCompressBeginAdv>("ZSTD_compressBegin_advanced");
        let s = corpus(Corpus::Text, 4096, 11);
        let mut steps = Vec::new();
        let mut blob = Vec::new();

        // (a) continue / end before any begin -> stage_wrong (60)
        for which in 0..2 {
            let c = Ctx::cctx(l);
            let mut dst = vec![0xCDu8; 8192];
            let f = if which == 0 { cc } else { ce };
            let r = unsafe {
                f(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr(&s),
                    s.len(),
                )
            };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[..16]);
        }
        // (b) begin then continue with dstCapacity 0..19: the frame header alone
        //     needs ZSTD_FRAMEHEADERSIZE_MAX (18) bytes.
        for cap in [0usize, 1, 5, 17, 18, 19, 23, 24] {
            let c = Ctx::cctx(l);
            steps.push(res(l, unsafe { b(c.ptr, 3) }));
            let mut dst = vec![0xCDu8; cap + 32];
            let r = unsafe {
                f_call(cc, c.ptr, &mut dst, cap, &s[..100])
            };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[cap..cap + 8]);
        }
        // (c) begin then compressEnd with srcSize 0 -> the empty-frame epilogue
        for cap in [0usize, 2, 3, 6, 8, 32] {
            let c = Ctx::cctx(l);
            steps.push(res(l, unsafe { b(c.ptr, 3) }));
            let mut dst = vec![0xCDu8; cap + 32];
            let r = unsafe { f_call(ce, c.ptr, &mut dst, cap, &[]) };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[..cap.min(32)]);
        }
        // (d) pledged 1000 then feed 999 (-> srcSize_wrong at compressEnd) and
        //     then 1001 (-> srcSize_wrong inside compressContinue).
        for feed in [999usize, 1000, 1001] {
            let c = Ctx::cctx(l);
            let p = unsafe { gp(3, 1000, 0) };
            steps.push(res(l, unsafe {
                ba(c.ptr, std::ptr::null(), 0, p, 1000)
            }));
            let mut dst = vec![0xCDu8; 8192];
            let r = unsafe {
                ce(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    ptr(&s),
                    feed,
                )
            };
            steps.push(res(l, r));
            blob.extend_from_slice(&dst[..16]);
        }
        // (e) pledged 1000, feed 600 with continue then 600 more -> the
        //     consumedSrcSize > pledged check inside compressContinue.
        {
            let c = Ctx::cctx(l);
            let p = unsafe { gp(3, 1000, 0) };
            steps.push(res(l, unsafe {
                ba(c.ptr, std::ptr::null(), 0, p, 1000)
            }));
            let mut dst = vec![0xCDu8; 8192];
            for _ in 0..2 {
                let r = unsafe {
                    cc(
                        c.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        ptr(&s),
                        600,
                    )
                };
                steps.push(res(l, r));
            }
        }
        // (f) checksumFlag with only 3 bytes left for the epilogue.
        {
            let c = Ctx::cctx(l);
            let mut p = unsafe { gp(3, 100, 0) };
            p.fParams.checksumFlag = 1;
            steps.push(res(l, unsafe { ba(c.ptr, std::ptr::null(), 0, p, 100) }));
            let mut dst = vec![0xCDu8; 64];
            let r = unsafe {
                ce(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    24,
                    ptr(&s),
                    100,
                )
            };
            steps.push(res(l, r));
        }
        (steps, Blob(blob))
    });
}

/// Tiny shim so the `dstCapacity` probes above can pass a capacity that is
/// smaller than the real allocation (so an overrun is detectable).
unsafe fn f_call(
    f: FnBlock5,
    ctx: *mut c_void,
    dst: &mut [u8],
    cap: usize,
    src: &[u8],
) -> SizeT {
    f(
        ctx,
        dst.as_mut_ptr() as *mut c_void,
        cap,
        ptr(src),
        src.len(),
    )
}

type FnCreateCDict = unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void;

/// `ZSTD_compressBegin_usingCDict` and `ZSTD_compressBegin_usingCDict_advanced`
/// (`zstd_compress.c:5823`+).
///
/// `ZSTD_compressBegin_usingCDict_internal` rejects a NULL cdict with
/// `dictionary_wrong`, and then chooses between `ZSTD_getCParamsFromCDict` and
/// `ZSTD_getCParams(cdict->compressionLevel, ...)` depending on
/// `pledgedSrcSize < 128 KB || pledgedSrcSize < dictContentSize*6 ||
/// pledgedSrcSize == UNKNOWN || cdict->compressionLevel == 0`; the pledged sizes
/// below sit exactly on those two thresholds. The non-advanced form additionally
/// forces `fParams.contentSizeFlag = 0`.
#[test]
fn compress_begin_using_cdict() {
    covers(&[
        "CFG:216",
        "ERR:compress/zstd_compress.c:5829",
    ]);

    let realdict = dict_fixture();
    let dsz = realdict.len();
    let payload = corpus(Corpus::Text, 200_000, 0xCD1C7);

    // NULL cdict, both entry points.
    diff("beginCDict/null", |l| {
        let c = Ctx::cctx(l);
        let f1 = l.sym::<FnCompressBeginCDict>("ZSTD_compressBegin_usingCDict");
        let f2 = l.sym::<FnCompressBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
        let fp = ZSTD_frameParameters {
            contentSizeFlag: 1,
            checksumFlag: 1,
            noDictIDFlag: 0,
        };
        (
            res(l, unsafe { f1(c.ptr, std::ptr::null()) }),
            res(l, unsafe { f2(c.ptr, std::ptr::null(), fp, 1000) }),
            res(l, unsafe {
                f2(c.ptr, std::ptr::null(), fp, ZSTD_CONTENTSIZE_UNKNOWN)
            }),
        )
    });

    let pledges: Vec<(String, u64)> = vec![
        ("1k".into(), 1024),
        ("128k-1".into(), (128 * 1024 - 1) as u64),
        ("128k".into(), (128 * 1024) as u64),
        ("6dict-1".into(), (dsz * 6 - 1) as u64),
        ("6dict".into(), (dsz * 6) as u64),
        ("unknown".into(), ZSTD_CONTENTSIZE_UNKNOWN),
    ];

    for lvl in [0, 5] {
        for (pn, pledged) in &pledges {
            // The pledged size must match what is actually fed, or
            // ZSTD_compressEnd's final check fires; feed exactly `pledged` bytes
            // where that is possible, else the whole payload.
            let feed = if *pledged == ZSTD_CONTENTSIZE_UNKNOWN {
                payload.len()
            } else {
                (*pledged as usize).min(payload.len())
            };
            let src = corpus(Corpus::Text, feed, 0xCD1C7);
            diff_bytes(&format!("beginCDict/lvl{lvl}/{pn}"), |l| {
                let cd = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                let cdp = unsafe { cd(ptr(realdict), dsz, lvl) };
                assert!(!cdp.is_null());
                let cdict = Ctx::from_raw(l, cdp, "ZSTD_freeCDict");
                let c = Ctx::cctx(l);
                let f1 = l.sym::<FnCompressBeginCDict>("ZSTD_compressBegin_usingCDict");
                let f2 =
                    l.sym::<FnCompressBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced");
                let cap = compress_bound(l, src.len()) + 4096;

                // (a) plain form: contentSizeFlag forced to 0, pledged UNKNOWN
                let r1 = res(l, unsafe { f1(c.ptr, cdict.ptr) });
                let (mut s1, b1) = drive_continue(l, c.ptr, &src, 70_000, cap);
                s1.insert(0, r1);

                // (b) advanced form with explicit fParams and pledged size
                let c2 = Ctx::cctx(l);
                let fp = ZSTD_frameParameters {
                    contentSizeFlag: 1,
                    checksumFlag: 1,
                    noDictIDFlag: 0,
                };
                let r2 = res(l, unsafe { f2(c2.ptr, cdict.ptr, fp, *pledged) });
                let (mut s2, b2) = drive_continue(l, c2.ptr, &src, 70_000, cap);
                s2.insert(0, r2);

                let mut blob = b1.0;
                blob.extend_from_slice(&b2.0);
                ((s1, s2), Blob(blob))
            });
        }
    }
}

/// `ZSTD_copyCCtx` (`zstd_compress.c:2591`).
///
/// The copy is only legal while the source is in stage `ZSTDcs_init`; a
/// freshly-created source (`ZSTDcs_created`) and one that has already emitted a
/// block (`ZSTDcs_ongoing`) both give `stage_wrong` (`:2519`). `pledgedSrcSize ==
/// 0` is remapped to `ZSTD_CONTENTSIZE_UNKNOWN` and drives
/// `fParams.contentSizeFlag`. Only cParams / useRowMatchFinder /
/// postBlockSplitter / ldmParams / maxBlockSize are copied, so the destination's
/// own requested parameters supply the rest -- which is why the destination is
/// probed both fresh and with parameters of its own.
#[test]
fn copy_cctx() {
    covers(&[
        "CFG:218",
        "ERR:compress/zstd_compress.c:2519",
    ]);

    let realdict = dict_fixture();
    let payload = corpus(Corpus::Text, 1 << 20, 0xC0FFEE);

    for (pn, pledged) in [
        ("zero", 0u64),
        ("exact", 1u64 << 20),
        ("unknown", ZSTD_CONTENTSIZE_UNKNOWN),
    ] {
        for stage in ["created", "init", "ongoing"] {
            for dst_has_params in [false, true] {
                let label = format!("copyCCtx/{pn}/{stage}/dstparams{dst_has_params}");
                diff_bytes(&label, |l| {
                    let cp = l.sym::<FnCopyCCtx>("ZSTD_copyCCtx");
                    let bd = l.sym::<FnCompressBeginDict>("ZSTD_compressBegin_usingDict");
                    let cc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressContinue");
                    let cap = compress_bound(l, payload.len()) + 4096;

                    let srcctx = Ctx::cctx(l);
                    let mut steps = Vec::new();
                    if stage != "created" {
                        steps.push(res(l, unsafe {
                            bd(srcctx.ptr, ptr(realdict), realdict.len(), 12)
                        }));
                    }
                    let mut scratch = vec![0xCDu8; cap];
                    if stage == "ongoing" {
                        let r = unsafe {
                            cc(
                                srcctx.ptr,
                                scratch.as_mut_ptr() as *mut c_void,
                                cap,
                                ptr(&payload),
                                4096,
                            )
                        };
                        steps.push(res(l, r));
                    }

                    let dstctx = Ctx::cctx(l);
                    if dst_has_params {
                        // These are *requested* params; copyCCtx overwrites only
                        // the table-related ones, so checksumFlag survives.
                        let _ = setp(l, dstctx.ptr, ZSTD_c_checksumFlag, 1);
                        let _ = setp(l, dstctx.ptr, ZSTD_c_compressionLevel, 1);
                    }
                    let rc = res(l, unsafe { cp(dstctx.ptr, srcctx.ptr, pledged) });
                    steps.push(rc.clone());

                    let mut blob = Vec::new();
                    if matches!(rc, R::Ok(_)) {
                        // Feed exactly as much as was pledged so compressEnd's
                        // consumed-size check is satisfied where it applies.
                        let feed = match pledged {
                            0 | ZSTD_CONTENTSIZE_UNKNOWN => payload.len(),
                            p => (p as usize).min(payload.len()),
                        };
                        let (s, b) = drive_continue(l, dstctx.ptr, &payload[..feed], 300_000, cap);
                        steps.extend(s);
                        blob.extend_from_slice(&b.0);
                        // A second copy from the same source must behave the same.
                        let dst2 = Ctx::cctx(l);
                        steps.push(res(l, unsafe { cp(dst2.ptr, srcctx.ptr, pledged) }));
                        let (s2, b2) =
                            drive_continue(l, dst2.ptr, &payload[..feed], 130_000, cap);
                        steps.extend(s2);
                        blob.extend_from_slice(&b2.0);
                        // ...and the *source* must still be usable itself.
                        if stage == "init" {
                            let (s3, b3) =
                                drive_continue(l, srcctx.ptr, &payload[..feed], 300_000, cap);
                            steps.extend(s3);
                            blob.extend_from_slice(&b3.0);
                        }
                    }
                    let _ = &mut scratch;
                    (steps, Blob(blob))
                });
            }
        }
    }
}

// ===========================================================================
// 7. The bufferless decompression state machine
// ===========================================================================

type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;

/// One `ZSTD_decompressContinue` step: what `ZSTD_nextSrcSizeToDecompress` and
/// `ZSTD_nextInputType` said beforehand, how many bytes were actually fed, and
/// what came back.
#[derive(Debug, PartialEq, Eq)]
struct DStep {
    expected: SizeT,
    nit: &'static str,
    fed: SizeT,
    ret: R,
}

/// Drive `ZSTD_decompressBegin` + `ZSTD_decompressContinue` strictly according to
/// `ZSTD_nextSrcSizeToDecompress`.
///
/// `block_chunk == 0` feeds exactly `expected` every time. A non-zero value feeds
/// `min(block_chunk, expected)` while the stage is `decompressBlock` /
/// `decompressLastBlock`, which is only legal for `bt_raw`
/// (`ZSTD_nextSrcSizeToDecompressWithInputSize` returns
/// `BOUNDED(1, inputSize, expected)` there) and must be `srcSize_wrong`
/// otherwise.
///
/// `expected == 0` means "frame fully decoded" and it is *not* legal to call
/// `ZSTD_decompressContinue` again -- the `ZSTDds_getFrameHeaderSize` arm does an
/// unconditional `MEM_readLE32(src)` guarded only by a compiled-out `assert`. The
/// single exception is the `ZSTDds_skipFrame` stage with a zero-length skippable
/// payload, whose arm reads nothing at all.
fn drive_dcontinue(
    l: &Lib,
    src: &[u8],
    out_cap: usize,
    restart: bool,
    block_chunk: usize,
) -> (R, Vec<DStep>, Blob) {
    let dctx = Ctx::dctx(l);
    let dbeg: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_decompressBegin");
    let nss: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_nextSrcSizeToDecompress");
    let nitf: FnCtxNit = *l.sym::<FnCtxNit>("ZSTD_nextInputType");
    let dc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_decompressContinue");
    let r0 = res(l, unsafe { dbeg(dctx.ptr) });
    let mut out = vec![0xCDu8; out_cap];
    let mut steps = Vec::new();
    let mut ip = 0usize;
    let mut op = 0usize;
    for _ in 0..20000 {
        let e = unsafe { nss(dctx.ptr) };
        let t = unsafe { nitf(dctx.ptr) };
        if e == 0 {
            if t == ZSTDnit_skippableFrame {
                // ZSTDds_skipFrame with an empty payload: safe, reads nothing.
                let r = unsafe {
                    dc(
                        dctx.ptr,
                        out[op..].as_mut_ptr() as *mut c_void,
                        out_cap - op,
                        ptr(&src[ip..]),
                        0,
                    )
                };
                steps.push(DStep {
                    expected: 0,
                    nit: nit_name(t),
                    fed: 0,
                    ret: res(l, r),
                });
                continue;
            }
            steps.push(DStep {
                expected: 0,
                nit: nit_name(t),
                fed: 0,
                ret: R::Ok(0),
            });
            if restart && ip < src.len() {
                steps.push(DStep {
                    expected: 0,
                    nit: "re-begin",
                    fed: 0,
                    ret: res(l, unsafe { dbeg(dctx.ptr) }),
                });
                continue;
            }
            break;
        }
        let want = if block_chunk > 0 && (t == ZSTDnit_block || t == ZSTDnit_lastBlock) {
            block_chunk.min(e)
        } else {
            e
        };
        if ip + want > src.len() {
            steps.push(DStep {
                expected: e,
                nit: nit_name(t),
                fed: want,
                ret: R::Err(-2, "<input exhausted>".into()),
            });
            break;
        }
        let r = unsafe {
            dc(
                dctx.ptr,
                out[op..].as_mut_ptr() as *mut c_void,
                out_cap - op,
                ptr(&src[ip..ip + want]),
                want,
            )
        };
        let rr = res(l, r);
        steps.push(DStep {
            expected: e,
            nit: nit_name(t),
            fed: want,
            ret: rr.clone(),
        });
        match rr {
            R::Ok(k) => {
                op += k;
                ip += want;
            }
            R::Err(..) => break,
        }
    }
    out.truncate(op);
    (r0, steps, Blob(out))
}

/// Build a frame whose blocks are of a chosen type, plus the plaintext.
fn typed_frame(kind: &str, cks: c_int, wlog: c_int) -> (Vec<u8>, Vec<u8>) {
    let (c, n) = match kind {
        "raw" => (Corpus::Random, 300_000),
        "rle" => (Corpus::Zeros, 300_000),
        "compressed" => (Corpus::Text, 300_000),
        "small" => (Corpus::Text, 300),
        "empty" => (Corpus::Text, 0),
        _ => unreachable!(),
    };
    let payload = corpus(c, n, 0xB10CC ^ n as u64);
    let sp = Spec {
        csf: 1,
        cks,
        did: 1,
        fmt: ZSTD_f_zstd1,
        wlog,
        kind: Kind::Known(n),
        dict: false,
    };
    (frame_fixture(sp, &payload), payload)
}

/// `ZSTD_decompressBegin` / `ZSTD_nextSrcSizeToDecompress` /
/// `ZSTD_nextInputType` / `ZSTD_decompressContinue`
/// (`zstd_decompress.c:1224`-`1432`).
///
/// The `(expected, nextInputType)` pair is captured *before* every single
/// `ZSTD_decompressContinue` call, so the whole `ZSTD_dStage` walk is pinned:
/// `5` then `hSize-5` then `3` then `cBlockSize` then (with a checksum) `4`, with
/// `ZSTDnit_frameHeader` / `blockHeader` / `block` / `lastBlock` / `checksum` /
/// `skippableFrame` in the matching order.
#[test]
fn decompress_continue_state_machine() {
    covers(&[
        "CFG:219",
        "CFG:220",
        "CFG:221",
        "CFG:222",
        "ERR:decompress/zstd_decompress.c:1279",
        "ERR:decompress/zstd_decompress.c:1297",
        "ERR:decompress/zstd_decompress.c:1314",
        "ERR:decompress/zstd_decompress.c:1315",
        "ERR:decompress/zstd_decompress.c:1354",
        "ERR:decompress/zstd_decompress.c:1364",
        "ERR:decompress/zstd_decompress.c:1367",
        "ERR:decompress/zstd_decompress.c:1380",
        "ERR:decompress/zstd_decompress.c:706",
    ]);

    // --- the happy paths, one per block type, with and without a checksum ----
    for kind in ["raw", "rle", "compressed", "small", "empty"] {
        for cks in [0, 1] {
            for wlog in [11, 17] {
                let (frame, plain) = typed_frame(kind, cks, wlog);
                let lab = format!("{kind}/cks{cks}/wlog{wlog}");
                let (_, steps, out) = diff_bytes(&format!("dcont[{lab}]"), |l| {
                    drive_dcontinue(l, &frame, plain.len() + 64, false, 0)
                });
                // Hard contract, straight out of ZSTD_decompressBegin and
                // ZSTD_decompressContinue: the first request is exactly
                // ZSTD_FRAMEHEADERSIZE_PREFIX(ZSTD_f_zstd1) == 5 bytes of frame
                // header, the second is the rest of the header, and the third is
                // a 3-byte block header.
                assert_eq!(steps[0].expected, FHSIZE_PREFIX_ZSTD1);
                assert_eq!(steps[0].nit, "frameHeader");
                assert_eq!(steps[1].nit, "frameHeader");
                assert_eq!(steps[2].expected, BLOCKHEADERSIZE);
                assert_eq!(steps[2].nit, "blockHeader");
                // ...and the final step is the checksum iff checksumFlag is set.
                let last = &steps[steps.len() - 2];
                if cks == 1 {
                    assert_eq!(last.expected, 4, "[{lab}] {steps:?}");
                    assert_eq!(last.nit, "checksum");
                }
                assert_eq!(out.0, plain, "[{lab}] round-trip mismatch");
                // Partial feeding of block payloads: legal only for bt_raw.
                for chunk in [1usize, 1000] {
                    diff_bytes(&format!("dcont-partial[{lab}/{chunk}]"), |l| {
                        drive_dcontinue(l, &frame, plain.len() + 64, false, chunk)
                    });
                }
            }
        }
    }

    // --- a skippable frame followed by a real frame --------------------------
    for (skname, sk) in [
        ("sk100", skippable(3, 100, &corpus(Corpus::Counter, 100, 3))),
        ("sk0", skippable(0, 0, &[])),
    ] {
        let (frame, plain) = typed_frame("small", 1, 17);
        let mut input = sk.clone();
        input.extend_from_slice(&frame);
        diff_bytes(&format!("dcont-skip[{skname}]"), |l| {
            drive_dcontinue(l, &input, plain.len() + 64, true, 0)
        });
        let mut input2 = frame.clone();
        input2.extend_from_slice(&sk);
        diff_bytes(&format!("dcont-skip-after[{skname}]"), |l| {
            drive_dcontinue(l, &input2, plain.len() + 64, true, 0)
        });
    }

    // --- wrong srcSize at every stage ---------------------------------------
    // At step k of a clean walk, deliberately feed expected-1, expected+1 or 0.
    let (frame, plain) = typed_frame("compressed", 1, 17);
    for k in 0..8usize {
        for delta in [-1i64, 1, i64::MIN /* means 0 */] {
            let dl = if delta == i64::MIN {
                "zero".to_string()
            } else {
                format!("{delta:+}")
            };
            diff(&format!("dcont-badsize[step{k}/{dl}]"), |l| {
                let dctx = Ctx::dctx(l);
                let dbeg: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_decompressBegin");
                let nss: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_nextSrcSizeToDecompress");
                let nitf: FnCtxNit = *l.sym::<FnCtxNit>("ZSTD_nextInputType");
                let dc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_decompressContinue");
                let mut out = vec![0xCDu8; plain.len() + 64];
                let mut steps = Vec::new();
                steps.push(res(l, unsafe { dbeg(dctx.ptr) }));
                let mut ip = 0usize;
                let mut op = 0usize;
                for step in 0..=k {
                    let e = unsafe { nss(dctx.ptr) };
                    let t = unsafe { nitf(dctx.ptr) };
                    if e == 0 {
                        steps.push(R::Ok(0));
                        break;
                    }
                    let want = if step == k {
                        if delta == i64::MIN {
                            0
                        } else if delta < 0 {
                            e.saturating_sub(1)
                        } else {
                            e + 1
                        }
                    } else {
                        e
                    };
                    if ip + want > frame.len() {
                        steps.push(R::Err(-2, "<exhausted>".into()));
                        break;
                    }
                    let r = unsafe {
                        dc(
                            dctx.ptr,
                            out[op..].as_mut_ptr() as *mut c_void,
                            out.len() - op,
                            ptr(&frame[ip..ip + want]),
                            want,
                        )
                    };
                    let rr = res(l, r);
                    steps.push(R::Ok(t as usize));
                    steps.push(rr.clone());
                    match rr {
                        R::Ok(n) => {
                            op += n;
                            ip += want;
                        }
                        R::Err(..) => break,
                    }
                }
                steps
            });
        }
    }

    // --- corrupt block headers ----------------------------------------------
    // The block header is 3 bytes LE24: bit0 lastBlock, bits1-2 blockType,
    // bits3.. size. Rewriting it exercises the bt_reserved rejection (:1314 via
    // ZSTD_getcBlockSize) and the "Block Size Exceeds Maximum" check (:1315).
    let (base, baseplain) = typed_frame("compressed", 0, 11);
    let hsize = {
        let l = &pair().c;
        match fh(l, &base, base.len()).ret {
            R::Ok(0) => (),
            other => panic!("unexpected getFrameHeader: {other:?}"),
        }
        let f = l.sym::<FnPtrLenSz>("ZSTD_frameHeaderSize");
        unsafe { f(ptr(&base), base.len()) }
    };
    let mk_bad = |hdr: u32| -> Vec<u8> {
        let mut v = base.clone();
        v[hsize] = (hdr & 0xFF) as u8;
        v[hsize + 1] = ((hdr >> 8) & 0xFF) as u8;
        v[hsize + 2] = ((hdr >> 16) & 0xFF) as u8;
        v
    };
    let orig_hdr = u32::from(base[hsize]) | (u32::from(base[hsize + 1]) << 8)
        | (u32::from(base[hsize + 2]) << 16);
    let orig_size = orig_hdr >> 3;
    let bads: Vec<(String, Vec<u8>)> = vec![
        ("bt_reserved".into(), mk_bad((orig_size << 3) | (3 << 1))),
        ("bt_raw-huge".into(), mk_bad((100000u32 << 3) | (0 << 1))),
        ("bt_rle-huge".into(), mk_bad((100000u32 << 3) | (1 << 1))),
        (
            "bt_compressed-over-max".into(),
            mk_bad((3000u32 << 3) | (2 << 1)),
        ),
        ("size-zero-last".into(), mk_bad(1)),
        ("size-zero-notlast".into(), mk_bad(0)),
        ("bt_rle-small".into(), mk_bad((7u32 << 3) | (1 << 1) | 1)),
        ("bt_raw-small".into(), mk_bad((7u32 << 3) | (0 << 1) | 1)),
    ];
    for (name, f) in &bads {
        diff_bytes(&format!("dcont-badblock[{name}]"), |l| {
            drive_dcontinue(l, f, baseplain.len() + 64, false, 0)
        });
        diff(&format!("dcont-badblock-whole[{name}]"), |l| whole_probe(l, f));
    }
}

/// `ZSTD_decompressBegin_usingDict` (`zstd_decompress.c:1588`),
/// `ZSTD_decompressBegin_usingDDict` (`:1601`, whose `ddictIsCold` is computed
/// *before* `ZSTD_decompressBegin` clears `dictEnd`, so calling it twice in a row
/// with the same DDict flips the flag) and `ZSTD_copyDCtx` (`:346`, which memcpys
/// only `offsetof(ZSTD_DCtx, inBuff)` bytes -- inBuff / maxWindowSize /
/// litBuffer / headerBuffer are deliberately *not* copied).
#[test]
fn decompress_begin_dict_and_copy_dctx() {
    covers(&[
        "CFG:214",
        "CFG:223",
        "ERR:decompress/zstd_decompress.c:1592",
        "ERR:decompress/zstd_decompress.c:1550",
    ]);

    let realdict = dict_fixture();
    let rawd = raw_dict();
    let plain = corpus(Corpus::Text, 60_000, 0xD1C7_D1C7);

    // A frame compressed with the real dictionary, and one with the raw one.
    let mk = |d: &[u8]| -> Vec<u8> {
        diff_bytes("build-dictframe", |l| {
            let c = Ctx::cctx(l);
            let _ = setp(l, c.ptr, ZSTD_c_compressionLevel, 5);
            let _ = setp(l, c.ptr, ZSTD_c_checksumFlag, 1);
            let ld = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
            let r = res(l, unsafe { ld(c.ptr, ptr(d), d.len()) });
            assert!(matches!(r, R::Ok(_)));
            let f = l.sym::<FnCompress2>("ZSTD_compress2");
            let cap = compress_bound(l, plain.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                f(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr(&plain),
                    plain.len(),
                )
            };
            let r = res(l, n);
            if let R::Ok(k) = r {
                dst.truncate(k);
            }
            (r, Blob(dst))
        })
        .1
         .0
    };
    let frame_real = mk(realdict);
    let frame_raw = mk(&rawd);

    let dicts: Vec<(&str, Vec<u8>)> = vec![
        ("none", Vec::new()),
        ("7bytes", vec![9u8, 8, 7, 6, 5, 4, 3]),
        ("raw4k", rawd.clone()),
        ("real", realdict.clone()),
        (
            "corrupt-magic",
            {
                let mut v = realdict.clone();
                v[9] ^= 0xFF;
                v[13] ^= 0xFF;
                v
            },
        ),
    ];

    for (dn, d) in &dicts {
        for (fn_, frame) in [("real", &frame_real), ("raw", &frame_raw)] {
            diff_bytes(&format!("dbeginDict[{dn}/{fn_}]"), |l| {
                let dctx = Ctx::dctx(l);
                let bd = l.sym::<FnDecompressBeginDict>("ZSTD_decompressBegin_usingDict");
                let nss: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_nextSrcSizeToDecompress");
                let dc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_decompressContinue");
                let (dp, ds) = if d.is_empty() {
                    (std::ptr::null(), 0usize)
                } else {
                    (ptr(d), d.len())
                };
                let mut steps = Vec::new();
                steps.push(res(l, unsafe { bd(dctx.ptr, dp, ds) }));
                let mut out = vec![0xCDu8; plain.len() + 64];
                let mut ip = 0usize;
                let mut op = 0usize;
                for _ in 0..10000 {
                    let e = unsafe { nss(dctx.ptr) };
                    if e == 0 || ip + e > frame.len() {
                        steps.push(R::Ok(e));
                        break;
                    }
                    let r = unsafe {
                        dc(
                            dctx.ptr,
                            out[op..].as_mut_ptr() as *mut c_void,
                            out.len() - op,
                            ptr(&frame[ip..ip + e]),
                            e,
                        )
                    };
                    let rr = res(l, r);
                    steps.push(rr.clone());
                    match rr {
                        R::Ok(k) => {
                            op += k;
                            ip += e;
                        }
                        R::Err(..) => break,
                    }
                }
                out.truncate(op);
                (steps, Blob(out))
            });
        }
    }

    // --- usingDDict, called twice (ddictIsCold flips), plus copyDCtx ---------
    for use_ddict in [false, true] {
        diff_bytes(&format!("dbeginDDict[{use_ddict}]"), |l| {
            let cdd = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let ddp = if use_ddict {
                unsafe { cdd(ptr(realdict), realdict.len()) }
            } else {
                std::ptr::null_mut()
            };
            let _ddict = if ddp.is_null() {
                None
            } else {
                Some(Ctx::from_raw(l, ddp, "ZSTD_freeDDict"))
            };
            let dctx = Ctx::dctx(l);
            let bdd =
                l.sym::<FnDecompressBeginDDict>("ZSTD_decompressBegin_usingDDict");
            let cpd = l.sym::<FnCopyDCtx>("ZSTD_copyDCtx");
            let nss: FnCtxSz = *l.sym::<FnCtxSz>("ZSTD_nextSrcSizeToDecompress");
            let dc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_decompressContinue");

            let mut steps = Vec::new();
            // twice in a row -> ddictIsCold flips on the second call
            steps.push(res(l, unsafe { bdd(dctx.ptr, ddp) }));
            steps.push(res(l, unsafe { bdd(dctx.ptr, ddp) }));

            let mut out = vec![0xCDu8; plain.len() + 64];
            let mut ip = 0usize;
            let mut op = 0usize;
            // Two steps on the original DCtx (frame header), then copy and
            // continue on the copy.
            let mut copied: Option<Ctx> = None;
            let mut cur = dctx.ptr;
            for step in 0..10000 {
                if step == 3 {
                    let d2 = Ctx::dctx(l);
                    unsafe { cpd(d2.ptr, cur) };
                    cur = d2.ptr;
                    copied = Some(d2);
                }
                let e = unsafe { nss(cur) };
                if e == 0 || ip + e > frame_real.len() {
                    steps.push(R::Ok(e));
                    break;
                }
                let r = unsafe {
                    dc(
                        cur,
                        out[op..].as_mut_ptr() as *mut c_void,
                        out.len() - op,
                        ptr(&frame_real[ip..ip + e]),
                        e,
                    )
                };
                let rr = res(l, r);
                steps.push(rr.clone());
                match rr {
                    R::Ok(k) => {
                        op += k;
                        ip += e;
                    }
                    R::Err(..) => break,
                }
            }
            out.truncate(op);
            drop(copied);
            (steps, Blob(out))
        });
    }
}

// ===========================================================================
// 8. Memory estimation
// ===========================================================================

type FnCCtxParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;

/// A hand-built `ZSTD_compressionParameters` for row 160's greedy /
/// searchLog=6 / windowLog=27 case. The estimators do not validate cParams, so
/// this is passed straight through to `ZSTD_sizeof_matchState`.
fn handmade_cparams() -> ZSTD_compressionParameters {
    ZSTD_compressionParameters {
        windowLog: 27,
        chainLog: 27,
        hashLog: 27,
        searchLog: 6,
        minMatch: 4,
        targetLength: 0,
        strategy: ZSTD_greedy,
    }
}

/// Every `ZSTD_estimate*Size` entry point.
///
/// These are pure arithmetic over the `zstd_cwksp.h` allocation sizes, so an
/// exact match is the strongest cheap structural check available on the
/// workspace layout: `ZSTD_estimateCCtxSize` MAXes over 4 srcSize tiers *and*
/// iterates `MIN(level,1)..level`; `ZSTD_estimateCCtxSize_usingCParams` MAXes the
/// row-matchfinder-enabled and -disabled estimates for greedy..lazy2 only; and
/// `ZSTD_estimateCStreamSize_usingCCtxParams` deliberately resolves
/// `useRowMatchFinder` from `&params->cParams` rather than the derived cParams.
#[test]
fn memory_estimates() {
    covers(&[
        "CFG:159",
        "CFG:160",
        "CFG:161",
        "CFG:162",
        "CFG:163",
        "CFG:164",
        "CFG:165",
        "CFG:166",
        "CFG:135",
        "CFG:236",
        "ERR:decompress/zstd_decompress.c:2006",
        "ERR:decompress/zstd_decompress.c:2007",
        "ERR:decompress/zstd_decompress.c:2008",
        "ERR:compress/zstd_compress.c:5521",
        "ERR:compress/zstd_compress.c:5536",
    ]);

    const LEVELS: &[c_int] = &[-131072, -1000, -5, -1, 0, 1, 2, 3, 12, 19, 22, 23, 100];

    diff("estimateCCtxSize/levels", |l| {
        let f = l.sym::<FnEstFromInt>("ZSTD_estimateCCtxSize");
        LEVELS.iter().map(|&lv| (lv, res(l, unsafe { f(lv) }))).collect::<Vec<_>>()
    });
    diff("estimateCStreamSize/levels", |l| {
        let f = l.sym::<FnEstFromInt>("ZSTD_estimateCStreamSize");
        LEVELS.iter().map(|&lv| (lv, res(l, unsafe { f(lv) }))).collect::<Vec<_>>()
    });

    diff("estimate/usingCParams", |l| {
        let gc = l.sym::<FnGetCParams>("ZSTD_getCParams");
        let ec = l.sym::<FnEstFromCParams>("ZSTD_estimateCCtxSize_usingCParams");
        let es = l.sym::<FnEstFromCParams>("ZSTD_estimateCStreamSize_usingCParams");
        let mut v = Vec::new();
        for lv in [1, 3, 5, 7, 19, 22] {
            for &srcHint in &[0u64, 16384, 131072, 262144, ZSTD_CONTENTSIZE_UNKNOWN] {
                let cp = unsafe { gc(lv, srcHint, 0) };
                v.push((
                    lv,
                    srcHint,
                    cp,
                    res(l, unsafe { ec(cp) }),
                    res(l, unsafe { es(cp) }),
                ));
            }
        }
        let hm = handmade_cparams();
        v.push((
            -999,
            0,
            hm,
            res(l, unsafe { ec(hm) }),
            res(l, unsafe { es(hm) }),
        ));
        v
    });

    // --- the CCtxParams forms ------------------------------------------------
    // (nbWorkers = 1 is rejected by ZSTD_CCtxParams_setParameter itself in this
    // non-multithreaded build, so the estimator's own nbWorkers>0 guard is
    // unreachable; the setter's rejection is recorded instead.)
    let knobs: Vec<(&str, Vec<(c_int, c_int)>)> = vec![
        ("default", vec![]),
        ("nbWorkers1", vec![(ZSTD_c_nbWorkers, 1)]),
        ("maxBlockSize0", vec![(ZSTD_c_maxBlockSize, 0)]),
        ("maxBlockSize1024", vec![(ZSTD_c_maxBlockSize, 1024)]),
        ("maxBlockSize131072", vec![(ZSTD_c_maxBlockSize, 131072)]),
        (
            "greedy-row-auto",
            vec![(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_auto)],
        ),
        (
            "greedy-row-enable",
            vec![(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_enable)],
        ),
        (
            "greedy-row-disable",
            vec![(ZSTD_c_strategy, ZSTD_greedy), (ZSTD_c_useRowMatchFinder, ZSTD_ps_disable)],
        ),
        // NOTE (out of contract): `enableLongDistanceMatching = ZSTD_ps_enable`
        // *without* also setting `ZSTD_c_ldmMinMatch` makes the reference C
        // SIGFPE. `ZSTD_estimateCCtxSize_usingCCtxParams_internal` passes
        // `params->ldmParams` verbatim to `ZSTD_ldm_getMaxNbSeq`
        // (`zstd_ldm.c:179`), which computes `maxChunkSize /
        // params.minMatchLength`; on a `ZSTD_CCtx_params` that has never been
        // through `ZSTD_ldm_adjustParameters` (only `ZSTD_resetCCtx_internal`
        // calls it) `minMatchLength` is still 0, so this is an integer division
        // by zero with no guard and no assert. Verified by running the C `.so`:
        // `estimate/ccp[ldm/lvl1]` -> SIGFPE. Every LDM case below therefore
        // sets `ldmMinMatch` first.
        (
            "ldm-minmatch",
            vec![
                (ZSTD_c_ldmMinMatch, 64),
                (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
            ],
        ),
        (
            "ldm-tuned",
            vec![
                (ZSTD_c_enableLongDistanceMatching, ZSTD_ps_enable),
                (ZSTD_c_ldmHashLog, 20),
                (ZSTD_c_ldmMinMatch, 32),
                (ZSTD_c_ldmBucketSizeLog, 4),
                (ZSTD_c_ldmHashRateLog, 7),
            ],
        ),
        ("minMatch3", vec![(ZSTD_c_minMatch, 3)]),
        ("minMatch4", vec![(ZSTD_c_minMatch, 4)]),
        ("stableIn", vec![(ZSTD_c_stableInBuffer, 1)]),
        ("stableOut", vec![(ZSTD_c_stableOutBuffer, 1)]),
        (
            "stableBoth",
            vec![(ZSTD_c_stableInBuffer, 1), (ZSTD_c_stableOutBuffer, 1)],
        ),
        ("wlog10", vec![(ZSTD_c_windowLog, 10)]),
        (
            "wlog27-mbs1024",
            vec![(ZSTD_c_windowLog, 27), (ZSTD_c_maxBlockSize, 1024)],
        ),
    ];
    for lvl in [1, 3, 19] {
        for (kn, sets) in &knobs {
            diff(&format!("estimate/ccp[{kn}/lvl{lvl}]"), |l| {
                let create = l.sym::<FnCreateCCtx>("ZSTD_createCCtxParams");
                let p = unsafe { create() };
                assert!(!p.is_null());
                let params = Ctx::from_raw(l, p, "ZSTD_freeCCtxParams");
                let init = l.sym::<FnCCtxParamsInit>("ZSTD_CCtxParams_init");
                let sp = l.sym::<FnCCtxSetParameter>("ZSTD_CCtxParams_setParameter");
                let mut steps = Vec::new();
                steps.push(res(l, unsafe { init(params.ptr, lvl) }));
                for &(k, v) in sets {
                    steps.push(res(l, unsafe { sp(params.ptr, k, v) }));
                }
                let ec = l.sym::<FnEstFromPtr>("ZSTD_estimateCCtxSize_usingCCtxParams");
                let es = l.sym::<FnEstFromPtr>("ZSTD_estimateCStreamSize_usingCCtxParams");
                (
                    steps,
                    res(l, unsafe { ec(params.ptr) }),
                    res(l, unsafe { es(params.ptr) }),
                )
            });
        }
    }

    // --- decompression-side estimators --------------------------------------
    diff("estimateDCtxSize", |l| {
        let f = l.sym::<FnEstVoid>("ZSTD_estimateDCtxSize");
        res(l, unsafe { f() })
    });
    diff("estimateDStreamSize", |l| {
        let f = l.sym::<FnEstFromSize>("ZSTD_estimateDStreamSize");
        [
            0usize,
            1,
            1024,
            131071,
            131072,
            131073,
            1 << 20,
            1 << 27,
            1u64.wrapping_shl(31) as usize,
        ]
        .iter()
        .map(|&w| (w, res(l, unsafe { f(w) })))
        .collect::<Vec<_>>()
    });
    diff("decodingBufferSize_min", |l| {
        let f = l.sym::<FnDecodingBufferSizeMin>("ZSTD_decodingBufferSize_min");
        let mut v = Vec::new();
        for w in [0u64, 1, 1024, 131071, 131072, 1 << 27, 1 << 31, 1 << 40] {
            for fcs in [0u64, 1, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
                v.push((w, u64r(fcs), res(l, unsafe { f(w, fcs) })));
            }
        }
        v
    });

    // ZSTD_estimateDStreamSize_fromFrame: the error forwarding, the ">0 means
    // need more input" -> srcSize_wrong remap, and the windowSize > 1<<31 check.
    {
        let (f10, _) = typed_frame("small", 0, 10);
        let (f27, _) = typed_frame("small", 1, 17);
        let sk = skippable(2, 100, &corpus(Corpus::Counter, 100, 1));
        // Hand-built headers whose window descriptor encodes windowLog 31 and 32.
        let mk_wl = |wl: u32| -> Vec<u8> {
            let mut b = vec![0u8; 24];
            b[0..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
            b[4] = 0b0000_0000; // dictID 0, no checksum, !singleSegment, fcsID 0
            b[5] = ((wl - 10) << 3) as u8; // mantissa 0
            b
        };
        let cases: Vec<(String, Vec<u8>)> = vec![
            ("wlog10-frame".into(), f10.clone()),
            ("wlog17-frame".into(), f27.clone()),
            ("first3".into(), f27[..3].to_vec()),
            ("first5".into(), f27[..5].to_vec()),
            ("skippable".into(), sk.clone()),
            ("wl31".into(), mk_wl(31)),
            ("wl32".into(), mk_wl(32)),
            ("wl41".into(), mk_wl(41)),
            ("empty".into(), Vec::new()),
        ];
        for (nm, b) in &cases {
            diff(&format!("estimateDStreamSize_fromFrame[{nm}]"), |l| {
                let f = l.sym::<FnPtrLenSz>("ZSTD_estimateDStreamSize_fromFrame");
                (res(l, unsafe { f(ptr(b), b.len()) }), fh(l, b, b.len()))
            });
        }
    }

    // --- dictionary estimators ----------------------------------------------
    diff("estimateCDictSize", |l| {
        let f = l.sym::<FnEstCDict>("ZSTD_estimateCDictSize");
        let fa = l.sym::<FnEstCDictAdv>("ZSTD_estimateCDictSize_advanced");
        let gc = l.sym::<FnGetCParams>("ZSTD_getCParams");
        let mut v = Vec::new();
        for ds in [0usize, 1, 7, 8, 4096, 1 << 20] {
            for lvl in [1, 3, 19] {
                v.push((ds, lvl, -1, res(l, unsafe { f(ds, lvl) })));
                let cp = unsafe { gc(lvl, ZSTD_CONTENTSIZE_UNKNOWN, ds) };
                for m in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    v.push((ds, lvl, m, res(l, unsafe { fa(ds, cp, m) })));
                }
            }
            let hm = handmade_cparams();
            v.push((ds, -999, ZSTD_dlm_byCopy, res(l, unsafe { fa(ds, hm, ZSTD_dlm_byCopy) })));
        }
        v
    });
    diff("estimateDDictSize", |l| {
        let f = l.sym::<FnEstDDict>("ZSTD_estimateDDictSize");
        let mut v = Vec::new();
        for ds in [0usize, 1, 8, 4096, 1 << 20] {
            for m in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                v.push((ds, m, res(l, unsafe { f(ds, m) })));
            }
        }
        v
    });
}

// ===========================================================================
// 9. Static (in-place) contexts
// ===========================================================================

/// A workspace whose *alignment* the caller controls: the backing store is a
/// `Vec<u64>` (so 8-byte aligned) and `at(k)` hands out `base + k`.
struct Ws(Vec<u64>);

impl Ws {
    fn new(bytes: usize) -> Ws {
        Ws(vec![0u64; bytes / 8 + 8])
    }
    fn at(&mut self, off: usize) -> *mut c_void {
        unsafe { (self.0.as_mut_ptr() as *mut u8).add(off) as *mut c_void }
    }
}

/// `ZSTD_initStaticCCtx` (`zstd_compress.c:126`) and `ZSTD_initStaticCStream`
/// (`:5933`, literally the same function).
///
/// The four NULL conditions are `workspaceSize <= sizeof(ZSTD_CCtx)`,
/// `(size_t)workspace & 7`, a failed object reservation, and
/// `!ZSTD_cwksp_check_available(TMP_WORKSPACE_SIZE + 2*sizeof(compressedBlockState))`.
/// A workspace one byte short of the estimate still *initialises* (the estimate
/// covers the tables, which are reserved later), so the failure surfaces from
/// `ZSTD_resetCCtx_internal`'s "static cctx : no resize" instead -- both are
/// recorded. `ZSTD_freeCCtx` on a static context must return `memory_allocation`.
#[test]
fn static_cctx_and_cstream() {
    covers(&[
        "CFG:167",
        "CFG:168",
        "CFG:169",
        "CFG:170",
        "CFG:114",
        "ERR:compress/zstd_compress.c:130",
        "ERR:compress/zstd_compress.c:131",
        "ERR:compress/zstd_compress.c:135",
        "ERR:compress/zstd_compress.c:142",
        "ERR:compress/zstd_compress.c:185",
    ]);

    let src = corpus(Corpus::Text, 65536, 0x57A71C);
    let realdict = dict_fixture();

    // --- absolute size sweep, including the sizeof(ZSTD_CCtx) boundary -------
    diff("staticCCtx/size-sweep", |l| {
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticCCtx");
        let mut ws = Ws::new(1 << 20);
        let mut v = Vec::new();
        for sz in [
            0usize, 1, 8, 16, 64, 1024, 4096, 5000, 5272, 5280, 5288, 8192, 16384, 65536,
            1 << 19,
        ] {
            let p = unsafe { init(ws.at(0), sz) };
            v.push((sz, p.is_null()));
        }
        v
    });

    // --- misalignment -------------------------------------------------------
    diff("staticCCtx/misaligned", |l| {
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticCCtx");
        let initcs = l.sym::<FnInitStatic>("ZSTD_initStaticCStream");
        let est = l.sym::<FnEstFromInt>("ZSTD_estimateCCtxSize");
        let need = unsafe { est(3) };
        let mut ws = Ws::new(need + 64);
        let mut v = Vec::new();
        for k in 0..8usize {
            let p = unsafe { init(ws.at(k), need) };
            let q = unsafe { initcs(ws.at(k), need) };
            v.push((k, p.is_null(), q.is_null()));
        }
        v
    });

    // --- exact / one-short / much-larger, then a real compression -----------
    for lvl in [1, 3, 19] {
        for delta in [-1i64, 0, 1, 4096, 1 << 20] {
            diff_bytes(&format!("staticCCtx/lvl{lvl}/delta{delta}"), |l| {
                let init = l.sym::<FnInitStatic>("ZSTD_initStaticCCtx");
                let est = l.sym::<FnEstFromInt>("ZSTD_estimateCCtxSize");
                let cc = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
                let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_CCtx");
                let free = l.sym::<FnFreeCCtx>("ZSTD_freeCCtx");
                let need = unsafe { est(lvl) };
                let size = (need as i64 + delta).max(0) as usize;
                let mut ws = Ws::new(size + 64);
                let p = unsafe { init(ws.at(0), size) };
                let mut steps: Vec<R> = Vec::new();
                steps.push(R::Ok(need));
                steps.push(R::Ok(p.is_null() as usize));
                let mut blob = Vec::new();
                if !p.is_null() {
                    steps.push(R::Ok(unsafe { szf(p) }));
                    let cap = compress_bound(l, src.len()) + 64;
                    let mut dst = vec![0xCDu8; cap];
                    let n = unsafe {
                        cc(
                            p,
                            dst.as_mut_ptr() as *mut c_void,
                            cap,
                            ptr(&src),
                            src.len(),
                            lvl,
                        )
                    };
                    let r = res(l, n);
                    if let R::Ok(k) = r {
                        blob.extend_from_slice(&dst[..k]);
                    }
                    steps.push(r);
                    steps.push(R::Ok(unsafe { szf(p) }));
                    // ZSTD_freeCCtx must refuse a static context.
                    steps.push(res(l, unsafe { free(p) }));
                }
                let _ = &mut ws;
                (steps, Blob(blob))
            });
        }
    }

    // --- a static CCtx sized for level 1 asked to compress at level 19 -------
    diff("staticCCtx/no-resize", |l| {
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticCCtx");
        let est = l.sym::<FnEstFromInt>("ZSTD_estimateCCtxSize");
        let f = l.sym::<FnCompress2>("ZSTD_compress2");
        let need = unsafe { est(1) };
        let mut ws = Ws::new(need + 64);
        let p = unsafe { init(ws.at(0), need) };
        assert!(!p.is_null());
        let mut out = Vec::new();
        for lvl in [1, 3, 19] {
            let _ = setp(l, p, ZSTD_c_compressionLevel, lvl);
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                f(
                    p,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr(&src),
                    src.len(),
                )
            };
            out.push((lvl, res(l, n)));
        }
        let _ = &mut ws;
        out
    });

    // --- nbWorkers on a static CCtx, and the dictionary limitations ----------
    diff("staticCCtx/params-and-dicts", |l| {
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticCCtx");
        let est = l.sym::<FnEstFromInt>("ZSTD_estimateCCtxSize");
        let need = unsafe { est(3) };
        let mut ws = Ws::new(need + 64);
        let p = unsafe { init(ws.at(0), need) };
        assert!(!p.is_null());
        let ldc = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        let ldr = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary_byReference");
        let rp = l.sym::<FnRefPrefix>("ZSTD_CCtx_refPrefix");
        let rst = l.sym::<FnDCtxReset>("ZSTD_CCtx_reset");
        let out = (
            // ZSTD_c_nbWorkers is special-cased before the generic path:
            // `value != 0 && cctx->staticSize` -> parameter_unsupported.
            setp(l, p, ZSTD_c_nbWorkers, 0),
            setp(l, p, ZSTD_c_nbWorkers, 1),
            // byCopy needs an allocation -> memory_allocation; byRef and
            // refPrefix are fine.
            res(l, unsafe { ldc(p, ptr(realdict), realdict.len()) }),
            res(l, unsafe { rst(p, ZSTD_reset_session_and_parameters) }),
            res(l, unsafe { ldr(p, ptr(realdict), realdict.len()) }),
            res(l, unsafe { rst(p, ZSTD_reset_session_and_parameters) }),
            res(l, unsafe { rp(p, ptr(realdict), realdict.len()) }),
        );
        let _ = &mut ws;
        out
    });

    // --- ZSTD_initStaticCStream + a full streaming compression ---------------
    for delta in [-1i64, 0, 1 << 16] {
        diff_bytes(&format!("staticCStream/delta{delta}"), |l| {
            let init = l.sym::<FnInitStatic>("ZSTD_initStaticCStream");
            let est = l.sym::<FnEstFromInt>("ZSTD_estimateCStreamSize");
            let cs2 = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let need = unsafe { est(3) };
            let size = (need as i64 + delta).max(0) as usize;
            let mut ws = Ws::new(size + 64);
            let p = unsafe { init(ws.at(0), size) };
            let mut steps: Vec<R> = vec![R::Ok(need), R::Ok(p.is_null() as usize)];
            let mut blob = Vec::new();
            if !p.is_null() {
                let big = corpus(Corpus::Text, 256 * 1024, 0x57C5);
                let cap = compress_bound(l, big.len()) + 4096;
                let mut dst = vec![0xCDu8; cap];
                let mut outb = ZSTD_outBuffer {
                    dst: dst.as_mut_ptr() as *mut c_void,
                    size: cap,
                    pos: 0,
                };
                let mut pos = 0usize;
                let mut failed = false;
                while pos < big.len() && !failed {
                    let n = (16 * 1024).min(big.len() - pos);
                    let mut inb = ZSTD_inBuffer {
                        src: ptr(&big[pos..pos + n]),
                        size: n,
                        pos: 0,
                    };
                    let dir = if pos + n >= big.len() {
                        ZSTD_e_end
                    } else {
                        ZSTD_e_continue
                    };
                    loop {
                        let r = unsafe { cs2(p, &mut outb, &mut inb, dir) };
                        let rr = res(l, r);
                        steps.push(rr.clone());
                        match rr {
                            R::Err(..) => {
                                failed = true;
                                break;
                            }
                            R::Ok(0) if inb.pos == inb.size => break,
                            R::Ok(_) if inb.pos == inb.size && dir != ZSTD_e_end => break,
                            R::Ok(_) => {}
                        }
                        if steps.len() > 500 {
                            break;
                        }
                    }
                    pos += n;
                }
                blob.extend_from_slice(&dst[..outb.pos]);
            }
            let _ = &mut ws;
            (steps, Blob(blob))
        });
    }
}

/// `ZSTD_initStaticDCtx` (`zstd_decompress.c:281`) and `ZSTD_initStaticDStream`
/// (`:1678`, the same function).
///
/// `ZSTD_initStaticDCtx` rejects `(size_t)workspace & 7` and `workspaceSize <
/// sizeof(ZSTD_DCtx)`; it then sets `inBuff = (char*)(dctx+1)` but leaves
/// `inBuffSize == 0`, so the *first* `ZSTD_decompressStream` always takes the
/// "buffers too small" path and returns `memory_allocation` when
/// `neededInBuffSize + neededOutBuffSize > staticSize - sizeof(ZSTD_DCtx)`.
/// `ZSTD_freeDCtx` on a static context returns `memory_allocation`, and
/// `ZSTD_d_refMultipleDDicts` is the one parameter with a `staticSize` guard.
#[test]
fn static_dctx_and_dstream() {
    covers(&[
        "CFG:172",
        "CFG:173",
        "CFG:174",
        "ERR:decompress/zstd_decompress.c:285",
        "ERR:decompress/zstd_decompress.c:286",
        "ERR:decompress/zstd_decompress.c:327",
        "ERR:decompress/zstd_decompress.c:1930",
        "ERR:decompress/zstd_decompress.c:2256",
    ]);

    let plain = corpus(Corpus::Text, 60_000, 0x5D7C);
    let frame = c_compress(&plain, 3);
    let win = {
        let l = &pair().c;
        match fh(l, &frame, frame.len()) {
            Fh { ret: R::Ok(0), zfh } => zfh.windowSize as usize,
            other => panic!("bad fixture: {other:?}"),
        }
    };

    diff("staticDCtx/size-and-align", |l| {
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticDCtx");
        let initds = l.sym::<FnInitStatic>("ZSTD_initStaticDStream");
        let est = l.sym::<FnEstVoid>("ZSTD_estimateDCtxSize");
        let need = unsafe { est() };
        let mut ws = Ws::new(need + 128);
        let mut v = Vec::new();
        v.push((0usize, 0usize, need, false, false));
        for sz in [0usize, 8, 1024, need - 1, need, need + 1] {
            for k in 0..8usize {
                let p = unsafe { init(ws.at(k), sz) };
                let q = unsafe { initds(ws.at(k), sz) };
                v.push((sz, k, need, p.is_null(), q.is_null()));
            }
        }
        v
    });

    // exact / +much, then a one-shot decompression through the static DCtx
    for delta in [0i64, 4096, 1 << 20] {
        diff_bytes(&format!("staticDCtx/decompress/delta{delta}"), |l| {
            let init = l.sym::<FnInitStatic>("ZSTD_initStaticDCtx");
            let est = l.sym::<FnEstVoid>("ZSTD_estimateDCtxSize");
            let dd = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
            let free = l.sym::<FnFreeDCtx>("ZSTD_freeDCtx");
            let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_DCtx");
            let need = (unsafe { est() } as i64 + delta) as usize;
            let mut ws = Ws::new(need + 64);
            let p = unsafe { init(ws.at(0), need) };
            let mut steps: Vec<R> = vec![R::Ok(p.is_null() as usize)];
            let mut blob = Vec::new();
            if !p.is_null() {
                steps.push(R::Ok(unsafe { szf(p) }));
                // ZSTD_d_refMultipleDDicts is refused on a static DCtx.
                steps.push(setdp(l, p, ZSTD_d_refMultipleDDicts, 1));
                steps.push(setdp(l, p, ZSTD_d_refMultipleDDicts, 0));
                steps.push(setdp(l, p, ZSTD_d_windowLogMax, 27));
                let mut dst = vec![0xCDu8; plain.len() + 64];
                let n = unsafe {
                    dd(
                        p,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        ptr(&frame),
                        frame.len(),
                    )
                };
                let r = res(l, n);
                if let R::Ok(k) = r {
                    blob.extend_from_slice(&dst[..k]);
                }
                steps.push(r);
                steps.push(res(l, unsafe { free(p) }));
            }
            let _ = &mut ws;
            (steps, Blob(blob))
        });
    }

    // ZSTD_initStaticDStream sized for the frame's windowSize, and too small.
    for (nm, wsz) in [
        ("exact", win),
        ("half", win / 2),
        ("tiny", 1024usize),
        ("big", win * 4),
    ] {
        diff_bytes(&format!("staticDStream/{nm}"), |l| {
            let init = l.sym::<FnInitStatic>("ZSTD_initStaticDStream");
            let est = l.sym::<FnEstFromSize>("ZSTD_estimateDStreamSize");
            let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let need = unsafe { est(wsz) };
            let mut ws = Ws::new(need + 64);
            let p = unsafe { init(ws.at(0), need) };
            let mut steps: Vec<R> = vec![R::Ok(need), R::Ok(p.is_null() as usize)];
            let mut blob = Vec::new();
            if !p.is_null() {
                let mut out = vec![0xCDu8; plain.len() + 64];
                let mut inb = ZSTD_inBuffer {
                    src: ptr(&frame),
                    size: frame.len(),
                    pos: 0,
                };
                let mut outb = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                for _ in 0..64 {
                    let r = unsafe { ds(p, &mut outb, &mut inb) };
                    let rr = res(l, r);
                    steps.push(rr.clone());
                    match rr {
                        R::Err(..) => break,
                        R::Ok(0) => break,
                        R::Ok(_) if inb.pos == inb.size && outb.pos == outb.size => break,
                        R::Ok(_) if inb.pos == inb.size => break,
                        _ => {}
                    }
                }
                blob.extend_from_slice(&out[..outb.pos]);
            }
            let _ = &mut ws;
            (steps, Blob(blob))
        });
    }
}

type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;

/// `ZSTD_initStaticCDict` (`zstd_compress.c:5758`) and `ZSTD_initStaticDDict`
/// (`zstd_ddict.c:187`).
///
/// `ZSTD_initStaticCDict` checks alignment *first*, then reserves the CDict
/// object, and only *then* compares `workspaceSize < neededSize`; it also forces
/// `compressionLevel = ZSTD_NO_CLEVEL`. `ZSTD_dct_fullDict` over raw content
/// makes `ZSTD_initCDict_internal` fail, so the function returns NULL rather than
/// an error code -- same for `ZSTD_initStaticDDict` via
/// `ZSTD_loadEntropy_intoDDict`.
#[test]
fn static_cdict_and_ddict() {
    let _serial = serial_alloc_lock();
    covers(&[
        "CFG:171",
        "CFG:175",
        "CFG:163",
        "CFG:166",
        "CFG:78",
        "ERR:compress/zstd_compress.c:5777",
        "ERR:compress/zstd_compress.c:5783",
        "ERR:compress/zstd_compress.c:5787",
        "ERR:compress/zstd_compress.c:5799",
        "ERR:decompress/zstd_ddict.c:198",
        "ERR:decompress/zstd_ddict.c:199",
        "ERR:decompress/zstd_ddict.c:204",
        "ERR:decompress/zstd_ddict.c:99",
        "ERR:decompress/zstd_ddict.c:112",
        "ERR:decompress/zstd_ddict.c:140",
        "ERR:decompress/zstd_ddict.c:105",
    ]);

    let realdict = dict_fixture().clone();
    let rawd = raw_dict();
    let src = corpus(Corpus::Text, 65536, 0x5CD1);
    let dicts: Vec<(&str, &Vec<u8>)> = vec![("real", &realdict), ("raw4k", &rawd)];

    // ZSTD_getDictID_fromDict (`zstd_decompress.c:1624`) is just the two guards
    // `dictSize < 8` and `MEM_readLE32 != ZSTD_MAGIC_DICTIONARY`; it is pinned
    // here next to the CDict/DDict identity accessors it is documented against.
    diff("getDictID_fromDict", |l| {
        let f = l.sym::<FnPtrLenU32>("ZSTD_getDictID_fromDict");
        let mut v: Vec<(String, c_uint)> = Vec::new();
        v.push(("null0".into(), unsafe { f(std::ptr::null(), 0) }));
        for n in 0..=8usize {
            v.push((format!("real{n}"), unsafe { f(ptr(&realdict), n) }));
        }
        v.push(("real-full".into(), unsafe {
            f(ptr(&realdict), realdict.len())
        }));
        v.push(("raw4k".into(), unsafe { f(ptr(&rawd), rawd.len()) }));
        for id in [0u32, 1, 255, 256, 65535, 65536, 0xFFFF_FFFF] {
            let mut b = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
            b.extend_from_slice(&id.to_le_bytes());
            b.extend_from_slice(&[0u8; 8]);
            v.push((format!("magic+{id}"), unsafe { f(ptr(&b), 8) }));
            v.push((format!("magic+{id}/16"), unsafe { f(ptr(&b), 16) }));
        }
        v.push(("zeros8".into(), unsafe { f(ptr(&[0u8; 8]), 8) }));
        v.push(("garbage4".into(), unsafe { f(ptr(&[0xAAu8; 4]), 4) }));
        v
    });

    for (dn, d) in &dicts {
        for &lm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
            for &ct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                for delta in [-1i64, 0, 1, 1 << 16] {
                    for align in [0usize, 1, 3, 7] {
                        let lab =
                            format!("staticCDict/{dn}/lm{lm}/ct{ct}/delta{delta}/align{align}");
                        diff_bytes(&lab, |l| {
                            let gc = l.sym::<FnGetCParams>("ZSTD_getCParams");
                            let esta =
                                l.sym::<FnEstCDictAdv>("ZSTD_estimateCDictSize_advanced");
                            let init = l.sym::<FnInitStaticCDict>("ZSTD_initStaticCDict");
                            let cp = unsafe { gc(5, ZSTD_CONTENTSIZE_UNKNOWN, d.len()) };
                            let need = unsafe { esta(d.len(), cp, lm) };
                            let size = (need as i64 + delta).max(0) as usize;
                            let mut ws = Ws::new(size + 64);
                            let p = unsafe {
                                init(ws.at(align), size, ptr(d), d.len(), lm, ct, cp)
                            };
                            let mut steps: Vec<R> =
                                vec![R::Ok(need), R::Ok(p.is_null() as usize)];
                            let mut blob = Vec::new();
                            if !p.is_null() {
                                let gid = l.sym::<FnDictIdFromObj>("ZSTD_getDictID_fromCDict");
                                let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_CDict");
                                steps.push(R::Ok(unsafe { gid(p) } as usize));
                                steps.push(R::Ok(unsafe { szf(p) }));
                                let c = Ctx::cctx(l);
                                let cu = l
                                    .sym::<FnCompressUsingCDict>("ZSTD_compress_usingCDict");
                                let cap = compress_bound(l, src.len()) + 64;
                                let mut dst = vec![0xCDu8; cap];
                                let n = unsafe {
                                    cu(
                                        c.ptr,
                                        dst.as_mut_ptr() as *mut c_void,
                                        cap,
                                        ptr(&src),
                                        src.len(),
                                        p,
                                    )
                                };
                                let r = res(l, n);
                                if let R::Ok(k) = r {
                                    blob.extend_from_slice(&dst[..k]);
                                }
                                steps.push(r);
                            }
                            let _ = &mut ws;
                            (steps, Blob(blob))
                        });
                    }
                }
            }
        }
    }

    // --- static DDict --------------------------------------------------------
    let plain = corpus(Corpus::Text, 40_000, 0x5DD1);
    let frame_with_real = {
        let l = &pair().c;
        let c = Ctx::cctx(l);
        let _ = setp(l, c.ptr, ZSTD_c_compressionLevel, 5);
        let ld = l.sym::<FnLoadDict>("ZSTD_CCtx_loadDictionary");
        unsafe { ld(c.ptr, ptr(&realdict), realdict.len()) };
        let f = l.sym::<FnCompress2>("ZSTD_compress2");
        let cap = compress_bound(l, plain.len()) + 64;
        let mut dst = vec![0u8; cap];
        let n = unsafe {
            f(
                c.ptr,
                dst.as_mut_ptr() as *mut c_void,
                cap,
                ptr(&plain),
                plain.len(),
            )
        };
        assert!(!is_error(l, n));
        dst.truncate(n);
        dst
    };

    for (dn, d) in &dicts {
        for &lm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
            for &ct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                for delta in [-1i64, 0, 1 << 16] {
                    for align in [0usize, 1, 7] {
                        let lab =
                            format!("staticDDict/{dn}/lm{lm}/ct{ct}/delta{delta}/align{align}");
                        diff_bytes(&lab, |l| {
                            let est = l.sym::<FnEstDDict>("ZSTD_estimateDDictSize");
                            let init = l.sym::<FnInitStaticDDict>("ZSTD_initStaticDDict");
                            let need = unsafe { est(d.len(), lm) };
                            let size = (need as i64 + delta).max(0) as usize;
                            let mut ws = Ws::new(size + 64);
                            let p =
                                unsafe { init(ws.at(align), size, ptr(d), d.len(), lm, ct) };
                            let mut steps: Vec<R> =
                                vec![R::Ok(need), R::Ok(p.is_null() as usize)];
                            let mut blob = Vec::new();
                            if !p.is_null() {
                                let gid = l.sym::<FnDictIdFromObj>("ZSTD_getDictID_fromDDict");
                                let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_DDict");
                                let dc = l.sym::<FnDDictContent>("ZSTD_DDict_dictContent");
                                let dsz = l.sym::<FnDDictSize>("ZSTD_DDict_dictSize");
                                steps.push(R::Ok(unsafe { gid(p) } as usize));
                                steps.push(R::Ok(unsafe { szf(p) }));
                                let content = unsafe { dc(p) };
                                let clen = unsafe { dsz(p) };
                                steps.push(R::Ok(clen));
                                // Whether the content pointer aliases the
                                // caller's buffer distinguishes byRef from
                                // byCopy without comparing raw addresses.
                                steps.push(R::Ok((content == ptr(d)) as usize));
                                if clen > 0 && !content.is_null() {
                                    blob.extend_from_slice(unsafe {
                                        std::slice::from_raw_parts(content as *const u8, clen)
                                    });
                                }
                                // Use it for a real decompression.
                                let dctx = Ctx::dctx(l);
                                let rd = l.sym::<FnRefDDict>("ZSTD_DCtx_refDDict");
                                steps.push(res(l, unsafe { rd(dctx.ptr, p) }));
                                let dd = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
                                let mut out = vec![0xCDu8; plain.len() + 64];
                                let n = unsafe {
                                    dd(
                                        dctx.ptr,
                                        out.as_mut_ptr() as *mut c_void,
                                        out.len(),
                                        ptr(&frame_with_real),
                                        frame_with_real.len(),
                                    )
                                };
                                let r = res(l, n);
                                if let R::Ok(k) = r {
                                    blob.extend_from_slice(&out[..k]);
                                }
                                steps.push(r);
                            }
                            let _ = &mut ws;
                            (steps, Blob(blob))
                        });
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 10. Custom allocators
// ===========================================================================

/// The custom-allocator bookkeeping below lives in process-wide `static`s,
/// because an `extern "C"` allocator callback has nowhere else to record what it
/// was asked for. Any test that reads that bookkeeping must therefore hold this
/// lock for its whole body: two such tests on different `--test-threads` would
/// otherwise interleave their reset/collect pairs and attribute one another's
/// allocations, producing a phantom "divergence" whose compressed output is in
/// fact identical.
static SERIAL_ALLOC: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Ignore poisoning: a panicking test has already failed the run, and the next
/// one should still be able to produce a real diagnosis rather than a poison
/// error.
fn serial_alloc_lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL_ALLOC.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

static ALLOC_LOG: Mutex<Vec<SizeT>> = Mutex::new(Vec::new());
static ALLOC_FAIL: AtomicBool = AtomicBool::new(false);
static FREE_COUNT: AtomicUsize = AtomicUsize::new(0);
static OPAQUE_SEEN: AtomicUsize = AtomicUsize::new(0);
/// The sentinel the test threads through `ZSTD_customMem::opaque`.
const OPAQUE_TAG: usize = 0x0BAD_F00D;

extern "C" fn counting_alloc(opaque: *mut c_void, size: SizeT) -> *mut c_void {
    OPAQUE_SEEN.store(opaque as usize, SeqCst);
    ALLOC_LOG.lock().unwrap().push(size);
    if ALLOC_FAIL.load(SeqCst) {
        return std::ptr::null_mut();
    }
    // 16 bytes of header keep the size for the matching free, and give at least
    // the 8-byte alignment zstd's workspaces need.
    let layout = std::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    unsafe {
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut SizeT).write(size);
        p.add(16) as *mut c_void
    }
}

extern "C" fn counting_free(opaque: *mut c_void, p: *mut c_void) {
    OPAQUE_SEEN.store(opaque as usize, SeqCst);
    FREE_COUNT.fetch_add(1, SeqCst);
    if p.is_null() {
        return;
    }
    unsafe {
        let base = (p as *mut u8).sub(16);
        let size = (base as *mut SizeT).read();
        let layout = std::alloc::Layout::from_size_align(size + 16, 16).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

fn reset_alloc_log(fail: bool) {
    ALLOC_LOG.lock().unwrap().clear();
    FREE_COUNT.store(0, SeqCst);
    OPAQUE_SEEN.store(0, SeqCst);
    ALLOC_FAIL.store(fail, SeqCst);
}

fn take_alloc_log() -> (Vec<SizeT>, usize, usize) {
    let v = ALLOC_LOG.lock().unwrap().clone();
    (v, FREE_COUNT.load(SeqCst), OPAQUE_SEEN.load(SeqCst))
}

fn counting_mem() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(counting_alloc),
        customFree: Some(counting_free),
        opaque: OPAQUE_TAG as *mut c_void,
    }
}

/// `ZSTD_createCCtx_advanced` / `ZSTD_createDCtx_advanced` /
/// `ZSTD_createCStream_advanced` / `ZSTD_createDStream_advanced` /
/// `ZSTD_createCDict_advanced` / `ZSTD_createDDict_advanced` with a custom
/// allocator.
///
/// Every `_advanced` constructor rejects `(!customAlloc) ^ (!customFree)` with
/// NULL and returns NULL when its first allocation fails. Beyond those two error
/// paths, the *sequence of allocation sizes* observed by a counting allocator is
/// compared: because zstd allocates its workspace as a single sized block whose
/// size is computed from `zstd_cwksp.h`'s alignment arithmetic, an identical
/// sequence is strong evidence that the whole workspace layout matches.
#[test]
fn custom_allocators() {
    let _serial = serial_alloc_lock();
    covers(&[
        "CFG:176",
        "CFG:177",
        "CFG:180",
        "CFG:8",
        "ERR:compress/zstd_compress.c:118",
        "ERR:compress/zstd_compress.c:120",
        "ERR:decompress/zstd_decompress.c:295",
        "ERR:decompress/zstd_decompress.c:298",
        "ERR:compress/zstd_compress.c:5612",
        "ERR:compress/zstd_compress.c:5627",
        "ERR:decompress/zstd_ddict.c:150",
        "ERR:decompress/zstd_ddict.c:153",
        "ERR:decompress/zstd_ddict.c:158",
        "ERR:decompress/zstd_ddict.c:112",
        "ERR:decompress/zstd_ddict.c:140",
        "ERR:decompress/zstd_ddict.c:214",
        "ERR:decompress/zstd_ddict.c:232",
        "ERR:decompress/zstd_ddict.c:242",
        "ERR:decompress/zstd_decompress.c:223",
        "ERR:compress/zstd_compress.c:5544",
        "ERR:compress/zstd_compress.c:5734",
        "ERR:compress/zstd_compress.c:5816",
        "ERR:common/allocations.h:28",
    ]);

    let only_alloc = ZSTD_customMem {
        customAlloc: Some(counting_alloc),
        customFree: None,
        opaque: std::ptr::null_mut(),
    };
    let only_free = ZSTD_customMem {
        customAlloc: None,
        customFree: Some(counting_free),
        opaque: std::ptr::null_mut(),
    };

    // --- the four context constructors --------------------------------------
    const CTORS: &[(&str, &str, &str)] = &[
        ("cctx", "ZSTD_createCCtx_advanced", "ZSTD_freeCCtx"),
        ("dctx", "ZSTD_createDCtx_advanced", "ZSTD_freeDCtx"),
        ("cstream", "ZSTD_createCStream_advanced", "ZSTD_freeCStream"),
        ("dstream", "ZSTD_createDStream_advanced", "ZSTD_freeDStream"),
    ];
    for (nm, ctor, dtor) in CTORS {
        // (a) inconsistent customMem -> NULL, and (b) a failing allocator -> NULL
        diff(&format!("ctor/{nm}/reject"), |l| {
            let f = l.sym::<FnCreateAdvanced>(ctor);
            reset_alloc_log(false);
            let a = unsafe { f(only_alloc) };
            let b = unsafe { f(only_free) };
            let (log1, free1, _) = take_alloc_log();
            reset_alloc_log(true);
            let c = unsafe { f(counting_mem()) };
            let (log2, free2, opq) = take_alloc_log();
            ALLOC_FAIL.store(false, SeqCst);
            (
                a.is_null(),
                b.is_null(),
                c.is_null(),
                log1,
                free1,
                log2,
                free2,
                opq,
            )
        });
        // (c) default ZSTD_defaultCMem-equivalent (all NULL) works
        diff(&format!("ctor/{nm}/defaultmem"), |l| {
            let f = l.sym::<FnCreateAdvanced>(ctor);
            let d = l.sym::<FnFreeCCtx>(dtor);
            let p = unsafe { f(ZSTD_customMem::default()) };
            let ok = !p.is_null();
            let r = res(l, unsafe { d(p) });
            (ok, r)
        });
    }

    // --- allocation transcript of a whole compression ------------------------
    let src = corpus(Corpus::Text, 200_000, 0xA110C);
    for lvl in [1, 3, 19] {
        diff_bytes(&format!("ctor/cctx/alloc-transcript/lvl{lvl}"), |l| {
            reset_alloc_log(false);
            let f = l.sym::<FnCreateAdvanced>("ZSTD_createCCtx_advanced");
            let free = l.sym::<FnFreeCCtx>("ZSTD_freeCCtx");
            let cc = l.sym::<FnCompressCCtx>("ZSTD_compressCCtx");
            let p = unsafe { f(counting_mem()) };
            assert!(!p.is_null());
            let cap = compress_bound(l, src.len()) + 64;
            let mut dst = vec![0xCDu8; cap];
            let n = unsafe {
                cc(
                    p,
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr(&src),
                    src.len(),
                    lvl,
                )
            };
            let r = res(l, n);
            if let R::Ok(k) = r {
                dst.truncate(k);
            } else {
                dst.truncate(0);
            }
            let rf = res(l, unsafe { free(p) });
            let (log, frees, opq) = take_alloc_log();
            ((r, rf, log, frees, opq), Blob(dst))
        });
    }
    diff_bytes("ctor/dctx/alloc-transcript", |l| {
        let frame = c_compress(&src, 3);
        reset_alloc_log(false);
        let f = l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced");
        let free = l.sym::<FnFreeDCtx>("ZSTD_freeDCtx");
        let dd = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let p = unsafe { f(counting_mem()) };
        assert!(!p.is_null());
        let mut dst = vec![0xCDu8; src.len() + 64];
        let n = unsafe {
            dd(
                p,
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                ptr(&frame),
                frame.len(),
            )
        };
        let r = res(l, n);
        if let R::Ok(k) = r {
            dst.truncate(k);
        } else {
            dst.truncate(0);
        }
        let rf = res(l, unsafe { free(p) });
        let (log, frees, opq) = take_alloc_log();
        ((r, rf, log, frees, opq), Blob(dst))
    });
    diff_bytes("ctor/dstream/alloc-transcript", |l| {
        let frame = c_compress(&src, 3);
        reset_alloc_log(false);
        let f = l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced");
        let free = l.sym::<FnFreeDCtx>("ZSTD_freeDStream");
        let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let p = unsafe { f(counting_mem()) };
        assert!(!p.is_null());
        let mut out = vec![0xCDu8; src.len() + 64];
        let mut inb = ZSTD_inBuffer {
            src: ptr(&frame),
            size: frame.len(),
            pos: 0,
        };
        let mut outb = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: out.len(),
            pos: 0,
        };
        let mut rets = Vec::new();
        for _ in 0..64 {
            let r = res(l, unsafe { ds(p, &mut outb, &mut inb) });
            rets.push(r.clone());
            match r {
                R::Err(..) | R::Ok(0) => break,
                _ if inb.pos == inb.size => break,
                _ => {}
            }
        }
        out.truncate(outb.pos);
        let rf = res(l, unsafe { free(p) });
        let (log, frees, opq) = take_alloc_log();
        ((rets, rf, log, frees, opq), Blob(out))
    });

    // --- ZSTD_createCDict_advanced / ZSTD_createDDict_advanced ---------------
    let realdict = dict_fixture().clone();
    let rawd = raw_dict();
    let short7 = vec![1u8, 2, 3, 4, 5, 6, 7];
    let dicts: Vec<(&str, &Vec<u8>)> =
        vec![("real", &realdict), ("raw4k", &rawd), ("short7", &short7)];
    for (dn, d) in &dicts {
        for &lm in &[ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
            for &ct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                diff(&format!("cdictAdv/{dn}/lm{lm}/ct{ct}"), |l| {
                    let gc = l.sym::<FnGetCParams>("ZSTD_getCParams");
                    let f = l.sym::<FnCreateCDictAdv>("ZSTD_createCDict_advanced");
                    let free = l.sym::<FnFreeCCtx>("ZSTD_freeCDict");
                    let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_CDict");
                    let gid = l.sym::<FnDictIdFromObj>("ZSTD_getDictID_fromCDict");
                    let cp = unsafe { gc(5, ZSTD_CONTENTSIZE_UNKNOWN, d.len()) };
                    // inconsistent customMem
                    let bad1 = unsafe { f(ptr(d), d.len(), lm, ct, cp, only_alloc) };
                    let bad2 = unsafe { f(ptr(d), d.len(), lm, ct, cp, only_free) };
                    // failing allocator
                    reset_alloc_log(true);
                    let bad3 = unsafe { f(ptr(d), d.len(), lm, ct, cp, counting_mem()) };
                    let (blog, bfree, _) = take_alloc_log();
                    // real
                    reset_alloc_log(false);
                    let p = unsafe { f(ptr(d), d.len(), lm, ct, cp, counting_mem()) };
                    let (mut info, mut sz, mut id) = (false, 0usize, 0u32);
                    if !p.is_null() {
                        info = true;
                        sz = unsafe { szf(p) };
                        id = unsafe { gid(p) };
                    }
                    let rf = res(l, unsafe { free(p) });
                    let (log, frees, opq) = take_alloc_log();
                    (
                        bad1.is_null(),
                        bad2.is_null(),
                        bad3.is_null(),
                        blog,
                        bfree,
                        info,
                        sz,
                        id,
                        rf,
                        log,
                        frees,
                        opq,
                    )
                });
                diff(&format!("ddictAdv/{dn}/lm{lm}/ct{ct}"), |l| {
                    let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                    let free = l.sym::<FnFreeCCtx>("ZSTD_freeDDict");
                    let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_DDict");
                    let gid = l.sym::<FnDictIdFromObj>("ZSTD_getDictID_fromDDict");
                    let bad1 = unsafe { f(ptr(d), d.len(), lm, ct, only_alloc) };
                    let bad2 = unsafe { f(ptr(d), d.len(), lm, ct, only_free) };
                    reset_alloc_log(true);
                    let bad3 = unsafe { f(ptr(d), d.len(), lm, ct, counting_mem()) };
                    let (blog, bfree, _) = take_alloc_log();
                    reset_alloc_log(false);
                    let p = unsafe { f(ptr(d), d.len(), lm, ct, counting_mem()) };
                    let (mut info, mut sz, mut id) = (false, 0usize, 0u32);
                    if !p.is_null() {
                        info = true;
                        sz = unsafe { szf(p) };
                        id = unsafe { gid(p) };
                    }
                    let rf = res(l, unsafe { free(p) });
                    let (log, frees, opq) = take_alloc_log();
                    (
                        bad1.is_null(),
                        bad2.is_null(),
                        bad3.is_null(),
                        blog,
                        bfree,
                        info,
                        sz,
                        id,
                        rf,
                        log,
                        frees,
                        opq,
                    )
                });
            }
        }
    }
    // dict == NULL with dictSize 0 is explicitly allowed for DDicts.
    diff("ddictAdv/null-dict", |l| {
        let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
        let free = l.sym::<FnFreeCCtx>("ZSTD_freeDDict");
        let szf = l.sym::<FnSizeofObj>("ZSTD_sizeof_DDict");
        let gid = l.sym::<FnDictIdFromObj>("ZSTD_getDictID_fromDDict");
        let mut v = Vec::new();
        for &ct in &[ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
            reset_alloc_log(false);
            let p = unsafe { f(std::ptr::null(), 0, ZSTD_dlm_byRef, ct, counting_mem()) };
            let (ok, sz, id) = if p.is_null() {
                (false, 0usize, 0u32)
            } else {
                (true, unsafe { szf(p) }, unsafe { gid(p) })
            };
            let rf = res(l, unsafe { free(p) });
            let (log, frees, _) = take_alloc_log();
            v.push((ct, ok, sz, id, rf, log, frees));
        }
        v
    });
    // ZSTD_sizeof_* / free on NULL for every object type.
    diff("sizeof-and-free-null", |l| {
        let names = [
            "ZSTD_sizeof_CCtx",
            "ZSTD_sizeof_CStream",
            "ZSTD_sizeof_DCtx",
            "ZSTD_sizeof_DStream",
            "ZSTD_sizeof_CDict",
            "ZSTD_sizeof_DDict",
        ];
        let mut v = Vec::new();
        for n in names {
            let f = l.sym::<FnSizeofObj>(n);
            v.push((n, unsafe { f(std::ptr::null()) }));
        }
        for n in [
            "ZSTD_freeCCtx",
            "ZSTD_freeCStream",
            "ZSTD_freeDCtx",
            "ZSTD_freeDStream",
            "ZSTD_freeCDict",
            "ZSTD_freeDDict",
        ] {
            let f = l.sym::<FnFreeCCtx>(n);
            v.push((n, unsafe { f(std::ptr::null_mut()) }));
        }
        v
    });
}

// ===========================================================================
// 11. Thread pools (ZSTD_MULTITHREAD is undefined in this build)
// ===========================================================================

/// `ZSTD_CCtx_refThreadPool` (`zstd_compress.c:1338`).
///
/// `ZSTD_createThreadPool` / `ZSTD_freeThreadPool` are defined inside the
/// `#ifdef ZSTD_MULTITHREAD` arm of `common/pool.c` (`pool.c:24`..`:313`), which
/// this build does not compile, so they are *not exported at all*; that absence
/// is asserted on both `.so`s rather than assumed. `ZSTD_CCtx_refThreadPool`
/// itself is always compiled: it only checks `streamStage != zcss_init` ->
/// `stage_wrong` and otherwise stores the pointer (which, without
/// `ZSTD_MULTITHREAD`, is never dereferenced because `cctx->mtctx` stays NULL).
/// A pool built with `POOL_create` (the synchronous stub, which returns the
/// address of a file-static `g_poolCtx`) is passed as well as NULL.
#[test]
fn thread_pool_surface() {
    covers(&["CFG:238", "ERR:compress/zstd_compress.c:1340"]);
    let p = pair();
    for n in ["ZSTD_createThreadPool", "ZSTD_freeThreadPool"] {
        assert!(
            !p.c.has(n),
            "{n} unexpectedly exported by the C .so; this build leaves \
             ZSTD_MULTITHREAD undefined so pool.c's public thread-pool API \
             should be compiled out"
        );
        assert!(!p.r.has(n), "{n} unexpectedly exported by the Rust .so");
    }

    type FnPoolCreate = unsafe extern "C" fn(SizeT, SizeT) -> *mut c_void;
    type FnPoolFree = unsafe extern "C" fn(*mut c_void);

    let src = corpus(Corpus::Text, 200_000, 0x7900_u64);
    for use_pool in [false, true] {
        diff_bytes(&format!("refThreadPool/{use_pool}"), |l| {
            let rtp = l.sym::<FnRefThreadPool>("ZSTD_CCtx_refThreadPool");
            let pc = l.sym::<FnPoolCreate>("POOL_create");
            let pf = l.sym::<FnPoolFree>("POOL_free");
            let cs2 = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
            let pool = if use_pool {
                unsafe { pc(4, 8) }
            } else {
                std::ptr::null_mut()
            };
            let c = Ctx::cctx(l);
            let mut steps = Vec::new();
            // (a) on a fresh cctx
            steps.push(res(l, unsafe { rtp(c.ptr, pool) }));
            // (b) after one ZSTD_compressStream2(ZSTD_e_continue) -> stage_wrong
            let cap = compress_bound(l, src.len()) + 4096;
            let mut dst = vec![0xCDu8; cap];
            let mut inb = ZSTD_inBuffer {
                src: ptr(&src),
                size: 1024,
                pos: 0,
            };
            let mut outb = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: cap,
                pos: 0,
            };
            steps.push(res(l, unsafe {
                cs2(c.ptr, &mut outb, &mut inb, ZSTD_e_continue)
            }));
            steps.push(res(l, unsafe { rtp(c.ptr, pool) }));
            // (c) finish the frame; the output must be unaffected by the pool
            inb = ZSTD_inBuffer {
                src: ptr(&src[1024..]),
                size: src.len() - 1024,
                pos: 0,
            };
            for _ in 0..64 {
                let r = res(l, unsafe { cs2(c.ptr, &mut outb, &mut inb, ZSTD_e_end) });
                steps.push(r.clone());
                match r {
                    R::Err(..) | R::Ok(0) => break,
                    _ => {}
                }
            }
            dst.truncate(outb.pos);
            if use_pool {
                unsafe { pf(pool) };
            }
            (steps, Blob(dst))
        });
    }
}

// ===========================================================================
// 12. Header constants and the decompression-margin macro
// ===========================================================================

/// `ZSTD_DECOMPRESSION_MARGIN(originalSize, blockSize)` (`zstd.h:1575`) as a Rust
/// expression, for the single-frame cross-check the function is documented
/// against.
fn margin_macro(original_size: usize, block_size: usize) -> usize {
    FHSIZE_MAX
        + 4
        + if original_size == 0 {
            0
        } else {
            3 * ((original_size + block_size - 1) / block_size)
        }
        + block_size
}

/// Pin the header constants through observable behaviour, cross-check
/// `ZSTD_decompressionMargin` against `ZSTD_DECOMPRESSION_MARGIN`, and drive the
/// legacy magics through the frame-sizing surface.
///
/// With `ZSTD_LEGACY_SUPPORT == 5`, `ZSTD_isLegacy` only recognises the v0.5,
/// v0.6 and v0.7 magics, so `ZSTD_findFrameSizeInfo` routes those into
/// `ZSTD_findFrameSizeInfoLegacy` while the v0.1..v0.4 magics fall through to
/// `prefix_unknown`. The legacy inputs used here are 32 zero-filled bytes behind
/// each magic, which every `ZSTDv0x_findFrameSizeInfoLegacy` guards against with
/// its own up-front size check (`CONFIGS.md` rows 408/409) -- no block decoding
/// happens.
#[test]
fn frame_constants_and_margin() {
    covers(&[
        "CFG:230",
        "CFG:231",
        "CFG:235",
        "ERR:decompress/zstd_decompress.c:850",
        "ERR:compress/zstd_compress.c:4623",
        "ERR:compress/zstd_compress.c:4712",
    ]);

    // --- ZSTD_SKIPPABLEHEADERSIZE (8): the skippable branch of
    //     ZSTD_getFrameHeader returns exactly this while srcSize is short.
    let sk = skippable(0, 4, &[1u8, 2, 3, 4]);
    for n in 0..SKIPPABLEHEADERSIZE {
        let got = diff(&format!("const/skippableHeaderSize/{n}"), |l| fh(l, &sk, n));
        if n >= FHSIZE_PREFIX_ZSTD1 {
            assert_eq!(
                got.ret,
                R::Ok(SKIPPABLEHEADERSIZE),
                "the skippable branch must ask for exactly ZSTD_SKIPPABLEHEADERSIZE"
            );
        }
    }

    // --- ZSTD_WINDOWLOG_MAX (31 on 64-bit): windowLog 31 parses, 32 does not.
    for wl in [WINDOWLOG_MAX - 1, WINDOWLOG_MAX, WINDOWLOG_MAX + 1] {
        let mut b = vec![0u8; 24];
        b[0..4].copy_from_slice(&ZSTD_MAGICNUMBER.to_le_bytes());
        b[4] = 0;
        b[5] = ((wl - 10) << 3) as u8;
        let got = diff(&format!("const/windowLogMax/{wl}"), |l| fh(l, &b, 24));
        if wl <= WINDOWLOG_MAX {
            assert_eq!(got.ret, R::Ok(0), "windowLog {wl} must be accepted");
        } else {
            assert!(
                matches!(got.ret, R::Err(..)),
                "windowLog {wl} must be rejected"
            );
        }
    }

    // --- ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1 (== 6): after the 18-byte
    //     frame header, ZSTD_compress_frameChunk needs 6 more bytes.
    let payload = corpus(Corpus::Text, 100, 4);
    let threshold = FHSIZE_MAX + BLOCKHEADERSIZE + MIN_CBLOCK_SIZE + 1;
    for cap in [threshold - 1, threshold] {
        let got = diff(&format!("const/frameChunkMin/{cap}"), |l| {
            let c = Ctx::cctx(l);
            let b = l.sym::<FnCompressBegin>("ZSTD_compressBegin");
            let cc: FnBlock5 = *l.sym::<FnBlock5>("ZSTD_compressContinue");
            let r0 = res(l, unsafe { b(c.ptr, 3) });
            let mut dst = vec![0xCDu8; cap + 64];
            let r = unsafe { f_call(cc, c.ptr, &mut dst, cap, &payload) };
            (r0, res(l, r), dst[cap], dst[cap + 1])
        });
        if cap < threshold {
            assert!(matches!(got.1, R::Err(..)));
        }
    }

    // --- ZSTD_decompressionMargin vs the macro, single frame -----------------
    for nb in [0usize, 1, 300, 65536, 131072, 200_000] {
        for cks in [0, 1] {
            for wlog in [11, 17] {
                let plain = corpus(Corpus::Text, nb, 0x4A_0000 ^ nb as u64);
                let sp = Spec {
                    csf: 1,
                    cks,
                    did: 1,
                    fmt: ZSTD_f_zstd1,
                    wlog,
                    kind: Kind::Known(nb),
                    dict: false,
                };
                let frame = frame_fixture(sp, &plain);
                let lab = format!("margin/n{nb}/cks{cks}/wlog{wlog}");
                let (margin, bsmax) = diff(&lab, |l| {
                    let m = l.sym::<FnPtrLenSz>("ZSTD_decompressionMargin");
                    let got = res(l, unsafe { m(ptr(&frame), frame.len()) });
                    let hdr = fh(l, &frame, frame.len());
                    (got, hdr.zfh.blockSizeMax)
                });
                let m = match margin {
                    R::Ok(m) => m,
                    other => panic!("[{lab}] unexpected margin {other:?}"),
                };
                let bound = margin_macro(nb, bsmax.max(1) as usize);
                assert!(
                    m <= bound,
                    "[{lab}] ZSTD_decompressionMargin {m} exceeds \
                     ZSTD_DECOMPRESSION_MARGIN({nb}, {bsmax}) = {bound}"
                );
                // In-place decompression must actually fit inside the margin.
                let mut buf = vec![0xCDu8; nb + m + 16];
                let off = nb + m - frame.len();
                buf[off..off + frame.len()].copy_from_slice(&frame);
                diff_bytes(&format!("margin-inplace[{lab}]"), |l| {
                    let mut b = buf.clone();
                    let f = l.sym::<FnDecompress>("ZSTD_decompress");
                    let n = unsafe {
                        f(
                            b.as_mut_ptr() as *mut c_void,
                            nb + m,
                            b[off..].as_ptr() as *const c_void,
                            frame.len(),
                        )
                    };
                    let r = res(l, n);
                    if let R::Ok(k) = r {
                        b.truncate(k);
                    }
                    (r, Blob(b))
                });
            }
        }
    }

    // --- legacy magics through the frame-sizing surface ----------------------
    for (name, m) in LEGACY_MAGICS_LE {
        let mut b = m.to_le_bytes().to_vec();
        b.extend_from_slice(&[0u8; 28]);
        diff(&format!("legacy-sizing[{name}]"), |l| {
            let mut v = Vec::new();
            for n in [4usize, 5, 6, 7, 8, 12, 32] {
                let p = ptr(&b);
                v.push((
                    n,
                    unsafe { l.sym::<FnPtrLenU32>("ZSTD_isFrame")(p, n) },
                    unsafe { l.sym::<FnPtrLenU32>("ZSTD_isSkippableFrame")(p, n) },
                    res(l, unsafe {
                        l.sym::<FnPtrLenSz>("ZSTD_findFrameCompressedSize")(p, n)
                    }),
                    u64r(unsafe { l.sym::<FnPtrLenU64>("ZSTD_decompressBound")(p, n) }),
                    u64r(unsafe {
                        l.sym::<FnPtrLenU64>("ZSTD_getFrameContentSize")(p, n)
                    }),
                    u64r(unsafe {
                        l.sym::<FnPtrLenU64>("ZSTD_findDecompressedSize")(p, n)
                    }),
                    res(l, unsafe {
                        l.sym::<FnPtrLenSz>("ZSTD_decompressionMargin")(p, n)
                    }),
                ));
            }
            v
        });
    }
}
