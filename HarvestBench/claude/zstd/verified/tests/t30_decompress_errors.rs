//! Phase C — the DECOMPRESSION error surface (`ERRORS.md`, section `decompress/`).
//!
//! Every case builds an exact invalid input, runs it through *both* shared
//! objects and requires the same `ZSTD_getErrorCode` / `ZSTD_getErrorName`
//! (via `R`/`res`), plus the same out-params and the same partial output bytes.
//! Tests are grouped by C source file so the mapping to `ERRORS.md` is obvious.
//!
//! Conventions used throughout:
//!   * `set_basic = 0, set_rle = 1, set_compressed = 2, set_repeat = 3`
//!     (`common/zstd_internal.h:94`). NOTE: several `ERRORS.md` *trigger*
//!     descriptions in this section quote these the other way round
//!     ("`set_repeat (1)`", "`set_rle (3)`"); the `file:line` sites are right,
//!     the parenthetical numbers are not. The tests below use the real enum.
//!   * `bt_raw = 0, bt_rle = 1, bt_compressed = 2, bt_reserved = 3`.
//!   * Frame_Header_Descriptor bits: 0-1 dictIDSizeCode, 2 checksumFlag,
//!     3 reserved (must be 0), 5 singleSegment, 6-7 frameContentSizeFlag.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::too_many_arguments)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};
#[allow(unused_imports)]
use std::sync::Mutex;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Local FFI signatures (everything not already in tests/common/mod.rs)
// ---------------------------------------------------------------------------

type FnU32Buf = unsafe extern "C" fn(*const c_void, SizeT) -> c_uint;
type FnSzBuf = unsafe extern "C" fn(*const c_void, SizeT) -> SizeT;
type FnU64Buf = unsafe extern "C" fn(*const c_void, SizeT) -> c_ulonglong;
type FnGetFrameHeader = unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT) -> SizeT;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT, c_int) -> SizeT;
type FnReadSkippable =
    unsafe extern "C" fn(*mut c_void, SizeT, *mut c_uint, *const c_void, SizeT) -> SizeT;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_uint) -> SizeT;
type FnDecodingBufSize = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> SizeT;
type FnSzVoid = unsafe extern "C" fn() -> SizeT;
type FnSzSz = unsafe extern "C" fn(SizeT) -> SizeT;
type FnInitStatic = unsafe extern "C" fn(*mut c_void, SizeT) -> *mut c_void;
type FnInitStaticDDict =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, c_int, c_int) -> *mut c_void;
type FnCreateAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;
type FnCreateDDictAdv =
    unsafe extern "C" fn(*const c_void, SizeT, c_int, c_int, ZSTD_customMem) -> *mut c_void;
type FnSzPtr = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnU32Ptr = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnRefDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnLoadDictAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int, c_int) -> SizeT;
type FnRefPrefixAdv = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnDecompUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;
type FnDecompUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
) -> SizeT;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnDecodeLiterals =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, *mut c_void, SizeT) -> SizeT;
type FnDecodeSeqHeaders =
    unsafe extern "C" fn(*mut c_void, *mut c_int, *const c_void, SizeT) -> SizeT;
type FnGetcBlockSize = unsafe extern "C" fn(*const c_void, SizeT, *mut BlockProperties) -> SizeT;
type FnDCtxGetParam = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> SizeT;
type FnLoadDEntropy = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnHufReadDTable =
    unsafe extern "C" fn(*mut c_uint, *const c_void, SizeT, *mut c_void, SizeT, c_int) -> SizeT;
type FnHufUsingDTable =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, SizeT, *const c_uint, c_int) -> SizeT;
type FnHufDCtxWksp = unsafe extern "C" fn(
    *mut c_uint,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *mut c_void,
    SizeT,
    c_int,
) -> SizeT;
type FnSelectDecoder = unsafe extern "C" fn(SizeT, SizeT) -> c_uint;
type FnFseNormalize =
    unsafe extern "C" fn(*mut i16, c_uint, *const c_uint, SizeT, c_uint, c_uint) -> SizeT;
type FnFseWriteNCount = unsafe extern "C" fn(*mut c_void, SizeT, *const i16, c_uint, c_uint) -> SizeT;
type FnSimpleArgs = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *mut SizeT,
    *const c_void,
    SizeT,
    *mut SizeT,
) -> SizeT;
type FnDStreamStub = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnPtrInt = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnPtrSz = unsafe extern "C" fn(*mut c_void, SizeT) -> SizeT;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, SizeT, *const c_void, SizeT) -> SizeT;
type FnTrainDict =
    unsafe extern "C" fn(*mut c_void, SizeT, *const c_void, *const SizeT, c_uint) -> SizeT;

/// `blockProperties_t` from `common/zstd_internal.h:298`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct BlockProperties {
    block_type: c_int,
    last_block: c_uint,
    orig_size: c_uint,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
/// `ZSTD_LEGACY_SUPPORT=5` -> only these three legacy magics are recognised.
const LEGACY_MAGICS: [u32; 3] = [0xFD2F_B525, 0xFD2F_B526, 0xFD2F_B527];
const SKIPPABLE_BASE: u32 = 0x184D_2A50;
const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;
const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9); // 2560
const HUF_flags_disableFast: c_int = 1 << 5;
/// `ZSTD_LITBUFFEREXTRASIZE` (`zstd_decompress_internal.h:118`) — the litSize
/// above which `ZSTD_allocateLiteralsBuffer` switches to `ZSTD_split`.
const ZSTD_LITBUFFEREXTRASIZE: usize = 1 << 16;

const bt_raw: u8 = 0;
const bt_rle: u8 = 1;
const bt_compressed: u8 = 2;
const bt_reserved: u8 = 3;

// ---------------------------------------------------------------------------
// Byte-level frame construction helpers
// ---------------------------------------------------------------------------

/// A zstd frame prefix: magic + the raw Frame_Header_Descriptor byte + whatever
/// header fields the caller wants after it (window descriptor, dictID, FCS).
fn frame_hdr(fhd: u8, rest: &[u8]) -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.push(fhd);
    v.extend_from_slice(rest);
    v
}

/// 3-byte Block_Header: bit0 Last_Block, bits1-2 Block_Type, bits3.. Block_Size.
fn block_header(last: bool, btype: u8, size: u32) -> [u8; 3] {
    let h = (size << 3) | ((btype as u32) << 1) | (last as u32);
    [(h & 0xFF) as u8, ((h >> 8) & 0xFF) as u8, ((h >> 16) & 0xFF) as u8]
}

/// `FHD = 0x00`, `WD = 0x00` -> `!singleSegment`, windowLog 10 (windowSize
/// 1024), no dictID, no checksum, frameContentSize unknown. `blockSizeMax`
/// becomes `MIN(1024, ZSTD_BLOCKSIZE_MAX) == 1024`, which is what makes the
/// `litSize > blockSizeMax` / `cBlockSize > blockSizeMax` rows reachable.
fn hdr_wlog10() -> Vec<u8> {
    frame_hdr(0x00, &[0x00])
}

/// A complete, minimal, *valid* frame: `hdr_wlog10()` + one empty last raw block.
fn empty_frame_wlog10() -> Vec<u8> {
    let mut v = hdr_wlog10();
    v.extend_from_slice(&block_header(true, bt_raw, 0));
    v
}

/// `singleSegment` + `fcsId == 3` (8-byte frameContentSize) + no dictID.
/// header size = 5 + 0 + 0 + 8 = 13.
fn hdr_single_fcs8(fcs: u64) -> Vec<u8> {
    frame_hdr(0x20 | 0xC0, &fcs.to_le_bytes())
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

fn skippable(variant: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = le32(SKIPPABLE_BASE + variant).to_vec();
    v.extend_from_slice(&le32(payload.len() as u32));
    v.extend_from_slice(payload);
    v
}

fn contains_legacy_magic(b: &[u8]) -> bool {
    if b.len() < 4 {
        return false;
    }
    for i in 0..=b.len() - 4 {
        let m = u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        if LEGACY_MAGICS.contains(&m) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Call wrappers that return everything observable
// ---------------------------------------------------------------------------

/// On an **error** return, the contents of `dst` are not a defined observable of
/// the reference C and are therefore not compared.
///
/// Evidence: `ZSTD_execSequence` (`zstd_decompress_block.c:1040`) copies the
/// literals with an unconditional `ZSTD_copy16` plus `ZSTD_wildcopy`, both of
/// which deliberately over-read the literals buffer's `WILDCOPY_OVERLENGTH`
/// padding, and only *afterwards* runs the out-of-range-offset check at line
/// 1054. The padding bytes that land in `dst` are therefore whatever was left in
/// that buffer. Running the identical `ZSTD_decompress` call repeatedly against
/// the reference C `.so` yields 6 extra bytes in `dst` on the first call and none
/// on subsequent calls, so there is no C behaviour here for the Rust to match.
/// Everything the API *reports* (status, `output->pos`, and the full buffer on
/// success) is still compared exactly.
fn blob_if_ok(r: &R, dst: Vec<u8>) -> Blob {
    match r {
        R::Ok(_) => Blob(dst),
        R::Err(..) => Blob(Vec::new()),
    }
}

/// `ZSTD_decompress` returning the status plus the whole destination buffer on
/// success (see [`blob_if_ok`] for why the error case is not compared).
fn dec_full(l: &Lib, src: &[u8], cap: usize) -> (R, Blob) {
    let f = l.sym::<FnDecompress>("ZSTD_decompress");
    let mut dst = vec![0xCDu8; cap];
    let p = if cap == 0 {
        std::ptr::null_mut()
    } else {
        dst.as_mut_ptr() as *mut c_void
    };
    let n = unsafe { f(p, cap, src.as_ptr() as *const c_void, src.len()) };
    let r = res(l, n);
    let b = blob_if_ok(&r, dst);
    (r, b)
}

/// `ZSTD_decompress` with an explicit `dst` pointer (for NULL / crafted pointers).
fn dec_raw(l: &Lib, dst: *mut c_void, cap: usize, src: &[u8]) -> R {
    let f = l.sym::<FnDecompress>("ZSTD_decompress");
    res(l, unsafe {
        f(dst, cap, src.as_ptr() as *const c_void, src.len())
    })
}

fn dec_dctx(l: &Lib, dctx: *mut c_void, src: &[u8], cap: usize) -> (R, Blob) {
    let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    let mut dst = vec![0xCDu8; cap];
    let p = if cap == 0 {
        std::ptr::null_mut()
    } else {
        dst.as_mut_ptr() as *mut c_void
    };
    let n = unsafe { f(dctx, p, cap, src.as_ptr() as *const c_void, src.len()) };
    let r = res(l, n);
    let b = blob_if_ok(&r, dst);
    (r, b)
}

/// Drive `ZSTD_decompressStream` to completion (or first error), feeding the
/// input in `chunk`-sized pieces and draining into a `cap`-byte output buffer.
/// Returns the last status, the number of input bytes consumed and the output.
fn stream_all(l: &Lib, src: &[u8], cap: usize, chunk: usize) -> (R, usize, Blob) {
    let ds = Ctx::dstream(l);
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut out = vec![0xCDu8; cap.max(1)];
    let mut ob = ZSTD_outBuffer {
        dst: out.as_mut_ptr() as *mut c_void,
        size: cap,
        pos: 0,
    };
    let mut consumed = 0usize;
    #[allow(unused_assignments)]
    let mut last = R::Ok(0);
    loop {
        let end = (consumed + chunk.max(1)).min(src.len());
        let mut ib = ZSTD_inBuffer {
            src: unsafe { src.as_ptr().add(consumed) } as *const c_void,
            size: end - consumed,
            pos: 0,
        };
        let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
        last = res(l, n);
        consumed += ib.pos;
        match &last {
            R::Err(..) => break,
            R::Ok(0) => break, // frame complete
            R::Ok(_) => {
                if ib.pos == 0 && consumed >= src.len() {
                    break; // needs more input than we have
                }
                if ob.pos == ob.size && ib.pos == 0 {
                    break; // output full, no progress
                }
            }
        }
    }
    // `output->pos` is the API-reported amount of decoded data; bytes beyond it
    // are scratch (see `blob_if_ok`).
    out.truncate(ob.pos);
    (last, consumed, Blob(out))
}

fn get_frame_header(l: &Lib, src: &[u8], format: c_int) -> (R, ZSTD_FrameHeader) {
    let f = l.sym::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced");
    let mut h = ZSTD_FrameHeader::default();
    let n = unsafe { f(&mut h, src.as_ptr() as *const c_void, src.len(), format) };
    (res(l, n), h)
}

fn u64_of(l: &Lib, name: &str, src: &[u8]) -> c_ulonglong {
    let f = l.sym::<FnU64Buf>(name);
    unsafe { f(src.as_ptr() as *const c_void, src.len()) }
}

fn sz_of(l: &Lib, name: &str, src: &[u8]) -> R {
    let f = l.sym::<FnSzBuf>(name);
    res(l, unsafe { f(src.as_ptr() as *const c_void, src.len()) })
}

fn u32_of(l: &Lib, name: &str, src: &[u8]) -> c_uint {
    let f = l.sym::<FnU32Buf>(name);
    unsafe { f(src.as_ptr() as *const c_void, src.len()) }
}

fn set_dparam(l: &Lib, dctx: *mut c_void, p: c_int, v: c_int) -> R {
    let f = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    res(l, unsafe { f(dctx, p, v) })
}

// ---------------------------------------------------------------------------
// Fixtures — all built with the C library so both sides see identical bytes
// ---------------------------------------------------------------------------

/// Compress with `ZSTD_compress2` after applying `params`.
fn c_compress_params(src: &[u8], params: &[(c_int, c_int)]) -> Vec<u8> {
    let l = &pair().c;
    let cctx = Ctx::cctx(l);
    let sp = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    for &(p, v) in params {
        let n = unsafe { sp(cctx.ptr, p, v) };
        assert!(!is_error(l, n), "CCtx_setParameter({p},{v}) failed");
    }
    let cap = compress_bound(l, src.len()) + 64;
    let mut dst = vec![0u8; cap];
    let c2 = l.sym::<FnCompress2>("ZSTD_compress2");
    let n = unsafe {
        c2(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    assert!(!is_error(l, n), "fixture ZSTD_compress2 failed");
    dst.truncate(n);
    dst
}

/// An `FSE_readNCount`-parsable distribution header with a chosen accuracy log.
/// Used to prove the `tableLog > maxLog` rejections (`zstd_decompress_block.c:684`,
/// `zstd_decompress.c:1486/1501/1516`) rather than the `FSE_readNCount` ones:
/// the header itself is *valid*, only its accuracy log is out of range.
fn fse_ncount(table_log: u32, max_sv: u32) -> Vec<u8> {
    let l = &pair().c;
    let norm = l.sym::<FnFseNormalize>("FSE_normalizeCount");
    let wr = l.sym::<FnFseWriteNCount>("FSE_writeNCount");
    let counts: Vec<c_uint> = vec![100; (max_sv + 1) as usize];
    let total: SizeT = counts.len() * 100;
    let mut nrm = vec![0i16; (max_sv + 1) as usize];
    let n = unsafe { norm(nrm.as_mut_ptr(), table_log, counts.as_ptr(), total, max_sv, 0) };
    assert!(!is_error(l, n), "FSE_normalizeCount failed");
    let mut buf = vec![0u8; 1024];
    let m = unsafe {
        wr(
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            nrm.as_ptr(),
            max_sv,
            table_log,
        )
    };
    assert!(!is_error(l, m), "FSE_writeNCount failed");
    buf.truncate(m);
    buf
}

/// A real, trained zstd dictionary (magic `0x EC30A437`) built with the C
/// dictBuilder. Deterministic: fixed corpus, fixed sample split.
fn trained_dict() -> &'static Vec<u8> {
    static D: OnceLock<Vec<u8>> = OnceLock::new();
    D.get_or_init(|| {
        let l = &pair().c;
        let f = l.sym::<FnTrainDict>("ZDICT_trainFromBuffer");
        let text = corpus(Corpus::Text, 160_000, 0xD1C7);
        let n = 400usize;
        let each = text.len() / n;
        let sizes: Vec<SizeT> = vec![each; n];
        let mut dict = vec![0u8; 4096];
        let r = unsafe {
            f(
                dict.as_mut_ptr() as *mut c_void,
                dict.len(),
                text.as_ptr() as *const c_void,
                sizes.as_ptr(),
                n as c_uint,
            )
        };
        assert!(!is_error(l, r), "ZDICT_trainFromBuffer failed");
        dict.truncate(r);
        assert_eq!(&dict[..4], &[0x37, 0xA4, 0x30, 0xEC], "dictionary magic");
        dict
    })
}

/// Byte offsets of the sections inside `trained_dict()`, computed with the C
/// library's own parsers so the truncation tests provably land inside the
/// intended `ZSTD_loadDEntropy` section.
#[derive(Debug, Clone, Copy)]
struct DictLayout {
    huf: usize,
    off: usize,
    ml: usize,
    ll: usize,
    rep: usize,
    content: usize,
}

fn dict_layout() -> DictLayout {
    static L: OnceLock<DictLayout> = OnceLock::new();
    *L.get_or_init(|| {
        let l = &pair().c;
        let d = trained_dict();
        let rd = l.sym::<FnHufReadDTable>("HUF_readDTableX2_wksp");
        let rn = l.sym::<FnFseReadNCountLocal>("FSE_readNCount");
        let mut dt = huf_dtable();
        let mut w = vec![0u64; 512];
        let mut p = 8usize; // magic + dictID
        let huf = p;
        let h = unsafe {
            rd(
                dt.as_mut_ptr(),
                d[p..].as_ptr() as *const c_void,
                d.len() - p,
                w.as_mut_ptr() as *mut c_void,
                HUF_DECOMPRESS_WORKSPACE_SIZE,
                0,
            )
        };
        assert!(!is_error(l, h), "dictionary Huffman table unreadable");
        p += h;
        let mut marks = [0usize; 3];
        for (i, max_sv) in [31u32, 52, 35].iter().enumerate() {
            marks[i] = p;
            let mut nc = vec![0i16; 64];
            let mut mx: c_uint = *max_sv;
            let mut lg: c_uint = 0;
            let k = unsafe {
                rn(
                    nc.as_mut_ptr(),
                    &mut mx,
                    &mut lg,
                    d[p..].as_ptr() as *const c_void,
                    d.len() - p,
                )
            };
            assert!(!is_error(l, k), "dictionary FSE table {i} unreadable");
            p += k;
        }
        DictLayout {
            huf,
            off: marks[0],
            ml: marks[1],
            ll: marks[2],
            rep: p,
            content: p + 12,
        }
    })
}

type FnFseReadNCountLocal =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, SizeT) -> SizeT;

/// A `HUF_DTable` seeded exactly the way `ZSTD_decompressBegin` does it
/// (`hufTable[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x1000001`).
fn huf_dtable() -> Vec<u32> {
    let mut dt = vec![0u32; 1 + (1usize << ZSTD_HUFFDTABLE_CAPACITY_LOG)];
    dt[0] = ZSTD_HUFFDTABLE_CAPACITY_LOG * 0x0100_0001;
    dt
}

// ---------------------------------------------------------------------------
// Frame introspection (C-side only, used to aim the crafted mutations)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Lits {
    lh_size: usize,
    lit_size: usize,
    lit_c_size: usize,
    single: bool,
    /// offset of the Huffman payload (tree description + jump table + streams)
    huf_off: usize,
    /// size of the Huffman tree description
    h_size: usize,
    /// `HUF_selectDecoder` verdict: true = X2 (double-symbol) decoder
    x2: bool,
}

#[derive(Debug, Clone, Copy)]
struct Blk {
    hdr_off: usize,
    payload: usize,
    btype: u8,
    c_size: usize,
    last: bool,
    lits: Option<Lits>,
}

fn frame_header_size(f: &[u8]) -> usize {
    let l = &pair().c;
    let g = l.sym::<FnSzBuf>("ZSTD_frameHeaderSize");
    let n = unsafe { g(f.as_ptr() as *const c_void, f.len()) };
    assert!(!is_error(l, n), "frameHeaderSize failed on fixture");
    n
}

/// Parse the block header at `off` and, for a compressed block, the literals
/// section header plus the Huffman tree-description size.
fn parse_block(f: &[u8], off: usize) -> Blk {
    let h = u32::from_le_bytes([f[off], f[off + 1], f[off + 2], 0]);
    let btype = ((h >> 1) & 3) as u8;
    let last = (h & 1) != 0;
    let c_size = if btype == bt_rle { 1 } else { (h >> 3) as usize };
    let payload = off + 3;
    let mut blk = Blk {
        hdr_off: off,
        payload,
        btype,
        c_size,
        last,
        lits: None,
    };
    if btype != bt_compressed {
        return blk;
    }
    let i0 = f[payload];
    let lhl = (i0 >> 2) & 3;
    let lhc = u32::from_le_bytes([f[payload], f[payload + 1], f[payload + 2], f[payload + 3]]);
    let (lh_size, lit_size, lit_c_size, single) = match lhl {
        0 | 1 => (
            3usize,
            ((lhc >> 4) & 0x3FF) as usize,
            ((lhc >> 14) & 0x3FF) as usize,
            lhl == 0,
        ),
        2 => (4, ((lhc >> 4) & 0x3FFF) as usize, (lhc >> 18) as usize, false),
        _ => (
            5,
            ((lhc >> 4) & 0x3FFFF) as usize,
            ((lhc >> 22) as usize) + ((f[payload + 4] as usize) << 10),
            false,
        ),
    };
    let huf_off = payload + lh_size;
    let l = &pair().c;
    let rd = l.sym::<FnHufReadDTable>("HUF_readDTableX1_wksp");
    let mut dt = huf_dtable();
    let mut w = vec![0u64; 512];
    let hs = unsafe {
        rd(
            dt.as_mut_ptr(),
            f[huf_off..].as_ptr() as *const c_void,
            lit_c_size,
            w.as_mut_ptr() as *mut c_void,
            HUF_DECOMPRESS_WORKSPACE_SIZE,
            0,
        )
    };
    let sel = l.sym::<FnSelectDecoder>("HUF_selectDecoder");
    blk.lits = Some(Lits {
        lh_size,
        lit_size,
        lit_c_size,
        single,
        huf_off,
        h_size: if is_error(l, hs) { 0 } else { hs },
        x2: unsafe { sel(lit_size, lit_c_size) } != 0,
    });
    blk
}

/// Search a small grid of corpora/sizes/levels for a frame whose *first* block
/// is `bt_compressed` with a `set_compressed` literals section satisfying `want`.
/// Building the fixtures this way (rather than hard-coding a level) keeps the
/// tests aimed at the right code path even though the encoder is free to choose.
fn find_lit_fixture(label: &str, want: impl Fn(&Lits) -> bool) -> (Vec<u8>, Blk) {
    for &kind in &[
        Corpus::SmallAlphabet,
        Corpus::Text,
        Corpus::Mixed,
        Corpus::Periodic,
        Corpus::LongRepeats,
    ] {
        for &n in &[20_000usize, 40_000, 80_000, 120_000, 200_000] {
            for lvl in [1, 2, 3, 6, 9, 12, 19] {
                let src = corpus(kind, n, 0x5EED);
                let f = c_compress(&src, lvl);
                let off = frame_header_size(&f);
                let blk = parse_block(&f, off);
                if let Some(li) = blk.lits {
                    if li.h_size > 0 && want(&li) {
                        return (f, blk);
                    }
                }
            }
        }
    }
    panic!("no fixture frame found for `{label}`");
}

fn fx_4stream_x1() -> &'static (Vec<u8>, Blk) {
    static F: OnceLock<(Vec<u8>, Blk)> = OnceLock::new();
    F.get_or_init(|| {
        find_lit_fixture("4-stream X1 literals", |li| {
            !li.single && !li.x2 && li.lit_c_size > li.h_size + 40
        })
    })
}

fn fx_4stream_x2() -> &'static (Vec<u8>, Blk) {
    static F: OnceLock<(Vec<u8>, Blk)> = OnceLock::new();
    F.get_or_init(|| {
        find_lit_fixture("4-stream X2 literals", |li| {
            !li.single && li.x2 && li.lit_c_size > li.h_size + 40
        })
    })
}

fn fx_1stream() -> &'static (Vec<u8>, Blk) {
    static F: OnceLock<(Vec<u8>, Blk)> = OnceLock::new();
    F.get_or_init(|| {
        find_lit_fixture("1-stream literals", |li| {
            li.single && li.lit_c_size > li.h_size + 8
        })
    })
}

/// A frame whose first block has `litSize > ZSTD_LITBUFFEREXTRASIZE (65536)`,
/// which is what selects the `...SplitLitBuffer` decode paths.
fn fx_split_lits() -> &'static (Vec<u8>, Blk) {
    static F: OnceLock<(Vec<u8>, Blk)> = OnceLock::new();
    F.get_or_init(|| {
        find_lit_fixture("split literals (litSize > 65536)", |li| {
            li.lit_size > ZSTD_LITBUFFEREXTRASIZE
        })
    })
}

// ===========================================================================
// decompress/zstd_decompress.c — frame identification (387/395/404/408)
// ===========================================================================

#[test]
fn zd_is_frame_and_is_skippable_frame() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:387",
        "ERR:decompress/zstd_decompress.c:395",
        "ERR:decompress/zstd_decompress.c:404",
        "ERR:decompress/zstd_decompress.c:408",
    ]);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // srcSize < ZSTD_FRAMEIDSIZE (4) -> 0, for every truncation of the magic.
    for n in 0..=4usize {
        cases.push((format!("magic[..{n}]"), MAGIC[..n].to_vec()));
    }
    // every skippable magic variant, and the two out-of-range neighbours
    for v in 0..=16u32 {
        cases.push((format!("skippable+{v}"), le32(SKIPPABLE_BASE + v).to_vec()));
    }
    cases.push(("skippable-1".into(), le32(SKIPPABLE_BASE - 1).to_vec()));
    // legacy magics are `isFrame` but not `isSkippableFrame`
    for m in LEGACY_MAGICS {
        cases.push((format!("legacy {m:08x}"), le32(m).to_vec()));
    }
    // unrelated magics
    for m in [0u32, 0xFFFF_FFFF, 0xFD2F_B520, 0xFD2F_B529, 0x184D_2A60] {
        cases.push((format!("other {m:08x}"), le32(m).to_vec()));
    }
    for (name, b) in cases {
        diff(&format!("ZSTD_isFrame {name}"), |l| u32_of(l, "ZSTD_isFrame", &b));
        diff(&format!("ZSTD_isSkippableFrame {name}"), |l| {
            u32_of(l, "ZSTD_isSkippableFrame", &b)
        });
    }
    // NULL src with size 0 is accepted by both checks (`size < 4` fires first).
    diff("ZSTD_isFrame(NULL,0)", |l| {
        let f = l.sym::<FnU32Buf>("ZSTD_isFrame");
        unsafe { f(std::ptr::null(), 0) }
    });
    diff("ZSTD_isSkippableFrame(NULL,0)", |l| {
        let f = l.sym::<FnU32Buf>("ZSTD_isSkippableFrame");
        unsafe { f(std::ptr::null(), 0) }
    });
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_frameHeaderSize_internal (419) and
// ZSTD_getFrameHeader_advanced (456/473/476/485/493/498/511/517)
// ===========================================================================

/// Every Frame_Header_Descriptor byte, at every input length 0..=18, through
/// `ZSTD_frameHeaderSize`, `ZSTD_getFrameHeader` and
/// `ZSTD_getFrameHeader_advanced` in both formats. This single sweep covers the
/// "need more input" hints, the reserved bit, the windowLog bound, all four
/// dictID size codes and all four frameContentSize field widths.
#[test]
fn zd_frame_header_all_descriptors_all_lengths() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:419",
        "ERR:decompress/zstd_decompress.c:456",
        "ERR:decompress/zstd_decompress.c:473",
        "ERR:decompress/zstd_decompress.c:476",
        "ERR:decompress/zstd_decompress.c:485",
        "ERR:decompress/zstd_decompress.c:493",
        "ERR:decompress/zstd_decompress.c:498",
        "ERR:decompress/zstd_decompress.c:511",
        "ERR:decompress/zstd_decompress.c:517",
    ]);
    // A deterministic filler so the dictID / FCS fields are non-zero.
    let filler: Vec<u8> = (0u8..24).map(|i| i.wrapping_mul(37).wrapping_add(1)).collect();
    for fhd in 0u16..=255 {
        let fhd = fhd as u8;
        let mut full = MAGIC.to_vec();
        full.push(fhd);
        full.extend_from_slice(&filler);
        for n in 0..=18usize {
            let b = &full[..n.min(full.len())];
            let tag = format!("fhd={fhd:#04x} len={n}");
            diff(&format!("ZSTD_frameHeaderSize {tag}"), |l| {
                sz_of(l, "ZSTD_frameHeaderSize", b)
            });
            for fmt in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                diff(&format!("ZSTD_getFrameHeader_advanced fmt={fmt} {tag}"), |l| {
                    get_frame_header(l, b, fmt)
                });
            }
            diff(&format!("ZSTD_getFrameContentSize {tag}"), |l| {
                u64_of(l, "ZSTD_getFrameContentSize", b)
            });
            diff(&format!("ZSTD_getDecompressedSize {tag}"), |l| {
                u64_of(l, "ZSTD_getDecompressedSize", b)
            });
        }
    }
}

/// The magic-number rejections, including the 1..4-byte prefixes that the C
/// compares against *both* `ZSTD_MAGICNUMBER` and `ZSTD_MAGIC_SKIPPABLE_START`
/// (`zstd_decompress.c:473`).
#[test]
fn zd_frame_header_magic_prefix_rejections() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:473",
        "ERR:decompress/zstd_decompress.c:476",
        "ERR:decompress/zstd_decompress.c:485",
        "ERR:decompress/zstd_decompress.c:493",
        "ERR:decompress/zstd_decompress.c:456",
    ]);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for m in [
        ZSTD_MAGICNUMBER,
        SKIPPABLE_BASE,
        SKIPPABLE_BASE + 15,
        0,
        0xFFFF_FFFF,
        0x184D_2A4F,
        0x184D_2A60,
        ZSTD_MAGICNUMBER ^ 1,
        ZSTD_MAGICNUMBER ^ 0x0100_0000,
        LEGACY_MAGICS[0],
    ] {
        let full = {
            let mut v = le32(m).to_vec();
            v.extend_from_slice(&[0x00u8; 16]);
            v
        };
        for n in 0..=9usize {
            cases.push((format!("{m:08x}[..{n}]"), full[..n].to_vec()));
        }
    }
    for (name, b) in cases {
        diff(&format!("getFrameHeader {name}"), |l| {
            get_frame_header(l, &b, ZSTD_f_zstd1)
        });
        diff(&format!("getFrameHeader magicless {name}"), |l| {
            get_frame_header(l, &b, ZSTD_f_zstd1_magicless)
        });
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 256));
        diff(&format!("findFrameCompressedSize {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        diff(&format!("findDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_findDecompressedSize", &b)
        });
        diff(&format!("estimateDStreamSize_fromFrame {name}"), |l| {
            sz_of(l, "ZSTD_estimateDStreamSize_fromFrame", &b)
        });
        diff(&format!("decompressionMargin {name}"), |l| {
            sz_of(l, "ZSTD_decompressionMargin", &b)
        });
    }
    // src == NULL while srcSize > 0 -> ERROR(GENERIC) (zstd_decompress.c:456).
    diff("getFrameHeader(NULL,8)", |l| {
        let f = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
        let mut h = ZSTD_FrameHeader::default();
        (res(l, unsafe { f(&mut h, std::ptr::null(), 8) }), h)
    });
    diff("getFrameHeader(NULL,0)", |l| {
        let f = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
        let mut h = ZSTD_FrameHeader::default();
        (res(l, unsafe { f(&mut h, std::ptr::null(), 0) }), h)
    });
    // OUT OF CONTRACT (removed): `ZSTD_getFrameContentSize(NULL, 8)`.
    // Precondition: `src != NULL` whenever `srcSize >= 4`. The reference C goes
    // `ZSTD_getFrameContentSize -> ZSTD_isLegacy(src,srcSize) -> MEM_readLE32(src)`
    // (`legacy/zstd_legacy.h:60`) with no NULL guard at all — verified by
    // observing SIGSEGV inside the C `.so` for this exact call, so there is no
    // C behaviour for the Rust to match. `ZSTD_getFrameHeader(NULL, 8)` *is*
    // in contract (it is checked at `zstd_decompress.c:456`) and is asserted above.
}

/// The reserved bit (`fhdByte & 0x08`) and the `windowLog > 31` bound, reached
/// through every entry point that funnels into `ZSTD_getFrameHeader_advanced`.
#[test]
fn zd_frame_header_reserved_bit_and_window_log() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:511",
        "ERR:decompress/zstd_decompress.c:517",
        "ERR:decompress/zstd_decompress.c:705",
        "ERR:decompress/zstd_decompress.c:760",
        "ERR:decompress/zstd_decompress.c:828",
        "ERR:decompress/zstd_decompress.c:850",
        "ERR:decompress/zstd_decompress.c:977",
        "ERR:decompress/zstd_decompress.c:2006",
        "ERR:decompress/zstd_decompress.c:2161",
    ]);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // reserved bit set, with every other descriptor field held at 0
    for fhd in [0x08u8, 0x09, 0x0A, 0x0B, 0x28, 0x48, 0xC8, 0xFF] {
        let mut v = frame_hdr(fhd, &[0x00; 16]);
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        cases.push((format!("reserved fhd={fhd:#04x}"), v));
    }
    // !singleSegment: windowLog = (wl >> 3) + 10; wl>>3 >= 22 is out of range.
    for wl3 in [0u8, 1, 20, 21, 22, 23, 30, 31] {
        let wl = wl3 << 3;
        let mut v = frame_hdr(0x00, &[wl]);
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        cases.push((format!("windowLog={} (wl={wl:#04x})", wl3 as u32 + 10), v));
    }
    // the same with the 3 low "extra precision" bits set
    for wl3 in [21u8, 22] {
        for extra in 0..8u8 {
            let wl = (wl3 << 3) | extra;
            let mut v = frame_hdr(0x00, &[wl]);
            v.extend_from_slice(&block_header(true, bt_raw, 0));
            cases.push((format!("wl={wl:#04x}"), v));
        }
    }
    for (name, b) in cases {
        diff(&format!("getFrameHeader {name}"), |l| {
            get_frame_header(l, &b, ZSTD_f_zstd1)
        });
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 4096));
        diff(&format!("findFrameCompressedSize {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        diff(&format!("decompressionMargin {name}"), |l| {
            sz_of(l, "ZSTD_decompressionMargin", &b)
        });
        diff(&format!("estimateDStreamSize_fromFrame {name}"), |l| {
            sz_of(l, "ZSTD_estimateDStreamSize_fromFrame", &b)
        });
        diff(&format!("decompressStream {name}"), |l| stream_all(l, &b, 4096, 4096));
        diff(&format!("decompressStream 1B {name}"), |l| stream_all(l, &b, 4096, 1));
    }
}

/// `ZSTD_d_format = ZSTD_f_zstd1_magicless`: the same frame with and without its
/// 4 magic bytes, decoded with and without the parameter set.
#[test]
fn zd_magicless_format() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:419",
        "ERR:decompress/zstd_decompress.c:476",
        "ERR:decompress/zstd_decompress.c:967",
        "ERR:decompress/zstd_decompress.c:1916",
        "ERR:decompress/zstd_decompress.c:2161",
    ]);
    let src = corpus(Corpus::Text, 3000, 11);
    let frame = c_compress(&src, 3);
    let magicless = frame[4..].to_vec();
    for (name, bytes) in [("with-magic", &frame), ("magicless", &magicless)] {
        for fmt in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
            diff(&format!("decompressDCtx {name} fmt={fmt}"), |l| {
                let d = Ctx::dctx(l);
                let sp = set_dparam(l, d.ptr, ZSTD_d_format, fmt);
                (sp, dec_dctx(l, d.ptr, bytes, src.len() + 64))
            });
            diff(&format!("decompressStream {name} fmt={fmt}"), |l| {
                let ds = Ctx::dstream(l);
                let sp = set_dparam(l, ds.ptr, ZSTD_d_format, fmt);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xCDu8; src.len() + 64];
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut ib = ZSTD_inBuffer {
                    src: bytes.as_ptr() as *const c_void,
                    size: bytes.len(),
                    pos: 0,
                };
                let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
                { out.truncate(ob.pos); (sp, res(l, n), ib.pos, ob.pos, Blob(out)) }
            });
            // truncated magicless headers at every length
            for k in 0..=14usize {
                let b = magicless[..k.min(magicless.len())].to_vec();
                diff(&format!("getFrameHeader_advanced magicless[..{k}] fmt={fmt}"), |l| {
                    get_frame_header(l, &b, fmt)
                });
            }
        }
    }
    // out-of-range ZSTD_d_format values (a C enum accepts any int)
    for v in [-1, 2, 999, i32::MIN, i32::MAX] {
        diff(&format!("set ZSTD_d_format={v}"), |l| {
            let d = Ctx::dctx(l);
            set_dparam(l, d.ptr, ZSTD_d_format, v)
        });
        diff(&format!("ZSTD_DCtx_setFormat({v})"), |l| {
            let d = Ctx::dctx(l);
            let g = l.sym::<FnPtrInt>("ZSTD_DCtx_setFormat");
            res(l, unsafe { g(d.ptr, v) })
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — skippable frames
// readSkippableFrameSize (592/595/598), ZSTD_readSkippableFrame (618/625/626/627)
// ===========================================================================

#[test]
fn zd_skippable_frames() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:592",
        "ERR:decompress/zstd_decompress.c:595",
        "ERR:decompress/zstd_decompress.c:598",
        "ERR:decompress/zstd_decompress.c:618",
        "ERR:decompress/zstd_decompress.c:625",
        "ERR:decompress/zstd_decompress.c:626",
        "ERR:decompress/zstd_decompress.c:627",
        "ERR:decompress/zstd_decompress.c:581",
        "ERR:decompress/zstd_decompress.c:652",
        "ERR:decompress/zstd_decompress.c:1126",
    ]);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // every magic variant, well-formed with a 4-byte payload
    for v in 0..=15u32 {
        cases.push((format!("variant{v} ok"), skippable(v, b"abcd")));
    }
    // truncated skippable headers, length 0..8 (=> srcSize_wrong at 592 / 618)
    let full = skippable(0, b"abcd");
    for n in 0..=8usize {
        cases.push((format!("hdr[..{n}]"), full[..n].to_vec()));
    }
    // declares 16 payload bytes, none present (598 / 626)
    cases.push((
        "declares16 none".into(),
        [le32(SKIPPABLE_BASE).to_vec(), le32(16).to_vec()].concat(),
    ));
    // the C's unchecked-error quirk: length field 0xFFFFFFFF overflows
    // (U32)(sizeU32 + 8) => frameParameter_unsupported at 595, but
    // ZSTD_readSkippableFrame then only checks the *magnitude* of the returned
    // size at 626, so it reports srcSize_wrong (72) instead of 14.
    cases.push((
        "len=0xFFFFFFFF".into(),
        [le32(SKIPPABLE_BASE).to_vec(), le32(0xFFFF_FFFF).to_vec()].concat(),
    ));
    for extra in [0xFFFF_FFF7u32, 0xFFFF_FFF8, 0xFFFF_FFF9, 0xFFFF_FFFE] {
        cases.push((
            format!("len={extra:#010x}"),
            [le32(SKIPPABLE_BASE).to_vec(), le32(extra).to_vec()].concat(),
        ));
    }
    // not a skippable magic at all (625)
    cases.push(("zstd magic".into(), [MAGIC.to_vec(), vec![0u8; 4]].concat()));
    cases.push(("junk magic".into(), vec![0u8; 8]));

    for (name, b) in cases {
        for cap in [0usize, 1, 2, 4, 64] {
            diff(&format!("readSkippableFrame {name} cap={cap}"), |l| {
                let f = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
                let mut dst = vec![0xCDu8; cap.max(1)];
                let mut mv: c_uint = 0xDEAD_BEEF;
                let n = unsafe {
                    f(
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        &mut mv,
                        b.as_ptr() as *const c_void,
                        b.len(),
                    )
                };
                dst.truncate(cap);
                (res(l, n), mv, Blob(dst))
            });
        }
        // magicVariant == NULL is explicitly allowed by the API
        diff(&format!("readSkippableFrame {name} mv=NULL"), |l| {
            let f = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
            let mut dst = vec![0xCDu8; 64];
            let n = unsafe {
                f(
                    dst.as_mut_ptr() as *mut c_void,
                    64,
                    std::ptr::null_mut(),
                    b.as_ptr() as *const c_void,
                    b.len(),
                )
            };
            (res(l, n), Blob(dst))
        });
        diff(&format!("isSkippableFrame {name}"), |l| {
            u32_of(l, "ZSTD_isSkippableFrame", &b)
        });
        diff(&format!("getFrameContentSize {name}"), |l| {
            u64_of(l, "ZSTD_getFrameContentSize", &b)
        });
        diff(&format!("findDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_findDecompressedSize", &b)
        });
        diff(&format!("findFrameCompressedSize {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 128));
        diff(&format!("decompressStream {name}"), |l| stream_all(l, &b, 128, 128));
        diff(&format!("decompressStream 1B {name}"), |l| stream_all(l, &b, 128, 1));
        diff(&format!("getFrameHeader {name}"), |l| {
            get_frame_header(l, &b, ZSTD_f_zstd1)
        });
    }
    // a skippable frame followed by a real frame, and vice versa
    let real = c_compress(&corpus(Corpus::Text, 1000, 3), 3);
    for (name, b) in [
        ("skip+real", [skippable(3, b"hdr!"), real.clone()].concat()),
        ("real+skip", [real.clone(), skippable(3, b"hdr!")].concat()),
        (
            "skip+skip+real",
            [skippable(0, b""), skippable(15, b"xy"), real.clone()].concat(),
        ),
    ] {
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 2048));
        diff(&format!("findDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_findDecompressedSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        diff(&format!("decompressStream {name}"), |l| {
            stream_all(l, &b, 2048, 7)
        });
    }
}

/// `ZSTD_writeSkippableFrame`: `magicVariant > 15` and
/// `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE`, then round-trip through
/// `ZSTD_readSkippableFrame`.
#[test]
fn zd_write_skippable_frame() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:618",
        "ERR:decompress/zstd_decompress.c:625",
        "ERR:decompress/zstd_decompress.c:627",
    ]);
    let payload = b"skippable-payload".to_vec();
    for mv in [0u32, 1, 15, 16, 17, 255, 0xFFFF_FFFF] {
        for cap in [0usize, 7, 8, payload.len() + 7, payload.len() + 8, 128] {
            diff(&format!("writeSkippableFrame mv={mv} cap={cap}"), |l| {
                let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
                let mut dst = vec![0xCDu8; cap.max(1)];
                let n = unsafe {
                    f(
                        dst.as_mut_ptr() as *mut c_void,
                        cap,
                        payload.as_ptr() as *const c_void,
                        payload.len(),
                        mv,
                    )
                };
                dst.truncate(cap);
                (res(l, n), Blob(dst))
            });
        }
    }
    // round-trip a well-formed one through the reader
    let l = &pair().c;
    let f = l.sym::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
    let mut buf = vec![0u8; payload.len() + 8];
    let n = unsafe {
        f(
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            payload.as_ptr() as *const c_void,
            payload.len(),
            7,
        )
    };
    assert!(!is_error(l, n));
    buf.truncate(n);
    for cap in [0usize, 1, payload.len() - 1, payload.len(), payload.len() + 8] {
        diff(&format!("readSkippableFrame roundtrip cap={cap}"), |l| {
            let g = l.sym::<FnReadSkippable>("ZSTD_readSkippableFrame");
            let mut dst = vec![0xCDu8; cap.max(1)];
            let mut mv: c_uint = 0;
            let r = unsafe {
                g(
                    dst.as_mut_ptr() as *mut c_void,
                    cap,
                    &mut mv,
                    buf.as_ptr() as *const c_void,
                    buf.len(),
                )
            };
            dst.truncate(cap);
            (res(l, r), mv, Blob(dst))
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_findDecompressedSize (652/661/664/669/677),
// ZSTD_getDecompressedSize (694), ZSTD_decompressBound (828),
// ZSTD_decompressionMargin (850/852), ZSTD_findFrameSizeInfo (760/762/773/776/788)
// ===========================================================================

#[test]
fn zd_frame_size_queries() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:579",
        "ERR:decompress/zstd_decompress.c:652",
        "ERR:decompress/zstd_decompress.c:661",
        "ERR:decompress/zstd_decompress.c:664",
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
        "ERR:decompress/zstd_decompress_block.c:66",
        "ERR:decompress/zstd_decompress_block.c:74",
    ]);
    let good = c_compress(&corpus(Corpus::Text, 2000, 5), 3);
    let hs = frame_header_size(&good);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // header only (762: getFrameHeader_advanced returned > 0)
    for n in 0..=hs + 2 {
        cases.push((format!("good[..{n}]"), good[..n.min(good.len())].to_vec()));
    }
    // 773 via ZSTD_getcBlockSize: bt_reserved
    cases.push((
        "reserved block".into(),
        [&good[..hs], &block_header(false, bt_reserved, 0)[..]].concat(),
    ));
    // 773 via ZSTD_getcBlockSize: fewer than 3 bytes left for a block header
    for k in 0..3usize {
        cases.push((
            format!("blockhdr[..{k}]"),
            [&good[..hs], &block_header(true, bt_raw, 0)[..k]].concat(),
        ));
    }
    // 776: ZSTD_blockHeaderSize + cBlockSize > remainingSize
    cases.push((
        "block declares 1000".into(),
        [&good[..hs], &block_header(true, bt_compressed, 1000)[..]].concat(),
    ));
    cases.push((
        "block declares 1000 +1B".into(),
        [
            &good[..hs],
            &block_header(true, bt_compressed, 1000)[..],
            &[0u8][..],
        ]
        .concat(),
    ));
    // 788: checksum flag set but fewer than 4 trailing bytes
    let cks = c_compress_params(
        &corpus(Corpus::Text, 2000, 5),
        &[(ZSTD_c_checksumFlag, 1), (ZSTD_c_compressionLevel, 3)],
    );
    for drop in 1..=4usize {
        cases.push((
            format!("checksum -{drop}B"),
            cks[..cks.len() - drop].to_vec(),
        ));
    }
    // 677 / 1166: 1..4 trailing bytes that cannot start a frame
    for k in 1..=4usize {
        cases.push((
            format!("good + {k} junk"),
            [&good[..], &vec![0u8; k][..]].concat(),
        ));
    }
    // 664: two frames each declaring frameContentSize = 0x8000000000000000
    let huge_frame = {
        let mut v = hdr_single_fcs8(0x8000_0000_0000_0000);
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        v
    };
    cases.push(("huge fcs x1".into(), huge_frame.clone()));
    cases.push((
        "huge fcs x2".into(),
        [huge_frame.clone(), huge_frame.clone()].concat(),
    ));
    cases.push((
        "huge fcs x3".into(),
        [huge_frame.clone(), huge_frame.clone(), huge_frame.clone()].concat(),
    ));
    // 661: frameContentSize unknown (fcsId == 0, !singleSegment)
    cases.push(("fcs unknown".into(), empty_frame_wlog10()));
    cases.push((
        "fcs unknown + real".into(),
        [empty_frame_wlog10(), good.clone()].concat(),
    ));
    cases.push(("good".into(), good.clone()));
    cases.push(("empty".into(), Vec::new()));

    for (name, b) in cases {
        diff(&format!("findDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_findDecompressedSize", &b)
        });
        diff(&format!("getDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_getDecompressedSize", &b)
        });
        diff(&format!("getFrameContentSize {name}"), |l| {
            u64_of(l, "ZSTD_getFrameContentSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        diff(&format!("findFrameCompressedSize {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
        diff(&format!("decompressionMargin {name}"), |l| {
            sz_of(l, "ZSTD_decompressionMargin", &b)
        });
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 4096));
    }
}

/// `ZSTD_estimateDStreamSize_fromFrame` (2006/2007/2008) and
/// `ZSTD_decodingBufferSize_min` (1983).
#[test]
fn zd_dstream_size_estimation() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1983",
        "ERR:decompress/zstd_decompress.c:2006",
        "ERR:decompress/zstd_decompress.c:2007",
        "ERR:decompress/zstd_decompress.c:2008",
    ]);
    let good = c_compress(&corpus(Corpus::Text, 2000, 5), 3);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("good".into(), good.clone()));
    for n in 0..=8usize {
        cases.push((format!("good[..{n}]"), good[..n].to_vec()));
    }
    cases.push(("junk".into(), vec![0u8; 8]));
    // singleSegment + fcsId 3 => windowSize == frameContentSize, which can
    // exceed the U32 windowSizeMax (1U << 31) -> frameParameter_windowTooLarge.
    for fcs in [
        0u64,
        1,
        (1u64 << 31) - 1,
        1u64 << 31,
        (1u64 << 31) + 1,
        0xFF_FFFF_FFFF,
        u64::MAX - 2,
    ] {
        cases.push((format!("singleSegment fcs={fcs}"), hdr_single_fcs8(fcs)));
    }
    for (name, b) in cases {
        diff(&format!("estimateDStreamSize_fromFrame {name}"), |l| {
            sz_of(l, "ZSTD_estimateDStreamSize_fromFrame", &b)
        });
    }
    // ZSTD_decodingBufferSize_min: the `(size_t)neededSize != neededSize`
    // overflow check at line 1983 is evaluated on every call; it can only fire
    // on a 32-bit `size_t`, so on this build the row records the site being
    // exercised, not the branch being taken.
    for ws in [
        0u64,
        1 << 10,
        1 << 20,
        1u64 << 31,
        1u64 << 40,
        0xFFFF_FFFF_FFFF,
        u64::MAX,
    ] {
        for fcs in [0u64, 1 << 20, ZSTD_CONTENTSIZE_UNKNOWN] {
            diff(&format!("decodingBufferSize_min ws={ws} fcs={fcs}"), |l| {
                let f = l.sym::<FnDecodingBufSize>("ZSTD_decodingBufferSize_min");
                res(l, unsafe { f(ws, fcs) })
            });
        }
    }
}

/// The frame's dictID field is non-zero and no matching dictionary is loaded
/// (`zstd_decompress.c:717` -> `dictionary_wrong` 32), through every path that
/// calls `ZSTD_decodeFrameHeader` (977 one-shot, 2221 streaming, 1298 continue).
#[test]
fn zd_dict_id_mismatch() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:717",
        "ERR:decompress/zstd_decompress.c:977",
        "ERR:decompress/zstd_decompress.c:2221",
    ]);
    // FHD 0x23 => dictIDSizeCode 3 (4 bytes), singleSegment, fcsId 0 (1 byte FCS)
    // header size = 5 + 0 + 4 + 0 + 1 = 10
    let mk = |code: u8, id: &[u8]| -> Vec<u8> {
        let mut v = frame_hdr(0x20 | code, id);
        v.push(0x00); // frameContentSize = 0 (singleSegment && !fcsId => 1 byte)
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        v
    };
    let cases = [
        ("code1 id=0x11", mk(1, &[0x11])),
        ("code1 id=0x00", mk(1, &[0x00])),
        ("code2 id=0x2211", mk(2, &[0x11, 0x22])),
        ("code2 id=0x0000", mk(2, &[0x00, 0x00])),
        ("code3 id=0x11223344", mk(3, &[0x44, 0x33, 0x22, 0x11])),
        ("code3 id=0", mk(3, &[0, 0, 0, 0])),
        ("code0", mk(0, &[])),
    ];
    for (name, b) in cases {
        diff(&format!("decompress {name}"), |l| dec_full(l, &b, 64));
        diff(&format!("getDictID_fromFrame {name}"), |l| {
            u32_of(l, "ZSTD_getDictID_fromFrame", &b)
        });
        diff(&format!("getFrameHeader {name}"), |l| {
            get_frame_header(l, &b, ZSTD_f_zstd1)
        });
        diff(&format!("decompressStream {name}"), |l| stream_all(l, &b, 64, 64));
        diff(&format!("decompressStream 1B {name}"), |l| stream_all(l, &b, 64, 1));
        // and with a real dictionary loaded whose ID almost certainly differs
        diff(&format!("decompress_usingDict {name}"), |l| {
            let d = Ctx::dctx(l);
            let f = l.sym::<FnDecompUsingDict>("ZSTD_decompress_usingDict");
            let dict = trained_dict();
            let mut dst = vec![0xCDu8; 64];
            let n = unsafe {
                f(
                    d.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    b.as_ptr() as *const c_void,
                    b.len(),
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                )
            };
            (res(l, n), Blob(dst))
        });
    }
    diff("getDictID_fromDict trained", |l| {
        u32_of(l, "ZSTD_getDictID_fromDict", trained_dict())
    });
    for n in [0usize, 1, 4, 7, 8, 9] {
        let b = trained_dict()[..n].to_vec();
        diff(&format!("getDictID_fromDict[..{n}]"), |l| {
            u32_of(l, "ZSTD_getDictID_fromDict", &b)
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_decompressFrame
// (967/975/977/991/995/1031/1046) and the block-copy helpers (900/903/913/916)
// ===========================================================================

#[test]
fn zd_decompress_frame_truncations_and_block_scan() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:967",
        "ERR:decompress/zstd_decompress.c:975",
        "ERR:decompress/zstd_decompress.c:977",
        "ERR:decompress/zstd_decompress.c:991",
        "ERR:decompress/zstd_decompress.c:995",
        "ERR:decompress/zstd_decompress.c:1031",
        "ERR:decompress/zstd_decompress.c:1046",
        "ERR:decompress/zstd_decompress.c:1157",
        "ERR:decompress/zstd_decompress_block.c:66",
        "ERR:decompress/zstd_decompress_block.c:74",
        "ERR:decompress/zstd_decompress_block.c:2197",
    ]);
    let src = corpus(Corpus::Text, 3000, 21);
    let good = c_compress(&src, 3);
    let hs = frame_header_size(&good);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // 967: remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN + 3 == 9
    for n in 0..=12usize {
        cases.push((format!("good[..{n}]"), good[..n].to_vec()));
    }
    // 975: an 18-byte header with only some of its bytes present
    let big_hdr = frame_hdr(0xE3, &[0u8; 16]); // dictID code 3, singleSegment, fcsId 3
    for n in 9..=20usize {
        cases.push((
            format!("bigheader[..{n}]"),
            big_hdr[..n.min(big_hdr.len())].to_vec(),
        ));
    }
    // 991: bt_reserved / short block header
    cases.push((
        "reserved block".into(),
        [&good[..hs], &block_header(false, bt_reserved, 7)[..]].concat(),
    ));
    // 995: block header declares more compressed bytes than remain
    cases.push((
        "declares 1025 got 1".into(),
        [
            &good[..hs],
            &block_header(true, bt_compressed, 1025)[..],
            &[0u8][..],
        ]
        .concat(),
    ));
    // 1046: correct frame whose declared frameContentSize is bumped by one.
    // `good` has FHD 0x60 (singleSegment, fcsId 1) so the FCS is the LE16 at [5..7].
    {
        let mut b = good.clone();
        let fcs = u16::from_le_bytes([b[5], b[6]]).wrapping_add(1);
        b[5..7].copy_from_slice(&fcs.to_le_bytes());
        cases.push(("fcs+1".into(), b));
    }
    {
        let mut b = good.clone();
        let fcs = u16::from_le_bytes([b[5], b[6]]).wrapping_sub(1);
        b[5..7].copy_from_slice(&fcs.to_le_bytes());
        cases.push(("fcs-1".into(), b));
    }
    // 1031: truncate inside the last block's payload
    for cut in [1usize, 5, 17, 64] {
        if good.len() > cut {
            cases.push((
                format!("body -{cut}B"),
                good[..good.len() - cut].to_vec(),
            ));
        }
    }
    for (name, b) in cases {
        for cap in [0usize, 1, 16, src.len(), src.len() + 64] {
            diff(&format!("decompress {name} cap={cap}"), |l| {
                dec_full(l, &b, cap)
            });
        }
        diff(&format!("decompressStream {name}"), |l| {
            stream_all(l, &b, src.len() + 64, src.len() + 64)
        });
        diff(&format!("decompressStream 1B {name}"), |l| {
            stream_all(l, &b, src.len() + 64, 1)
        });
    }
}

/// `ZSTD_copyRawBlock` (900/903) and `ZSTD_setRleBlock` (913/916), plus the
/// `bt_raw`/`bt_rle` forwarding sites in `ZSTD_decompressContinue` (1354/1366).
#[test]
fn zd_raw_and_rle_block_dst_errors() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:900",
        "ERR:decompress/zstd_decompress.c:903",
        "ERR:decompress/zstd_decompress.c:913",
        "ERR:decompress/zstd_decompress.c:916",
        "ERR:decompress/zstd_decompress.c:1031",
        "ERR:decompress/zstd_decompress.c:1354",
        "ERR:decompress/zstd_decompress.c:1366",
    ]);
    // A frame with a single last `bt_raw` block of 8 bytes.
    let raw8 = {
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_raw, 8));
        v.extend_from_slice(b"01234567");
        v
    };
    // A frame with a single last `bt_rle` block regenerating 100 bytes.
    let rle100 = {
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_rle, 100));
        v.push(0xAA);
        v
    };
    // `bt_rle` regenerating 2000 bytes, which is > blockSizeMax (1024).
    let rle2000 = {
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_rle, 2000));
        v.push(0xAA);
        v
    };
    // `bt_raw` declaring 2000 bytes, > blockSizeMax
    let raw2000 = {
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_raw, 2000));
        v.extend_from_slice(&vec![0x5Au8; 2000]);
        v
    };
    for (name, b) in [
        ("raw8", &raw8),
        ("rle100", &rle100),
        ("rle2000", &rle2000),
        ("raw2000", &raw2000),
    ] {
        for cap in [0usize, 1, 4, 7, 8, 99, 100, 1024, 4096] {
            diff(&format!("decompress {name} cap={cap}"), |l| {
                dec_full(l, b, cap)
            });
            diff(&format!("decompressStream {name} cap={cap}"), |l| {
                stream_all(l, b, cap, b.len())
            });
        }
        // dst == NULL with dstCapacity == 0: `srcSize > dstCapacity` at line 900
        // fires first for a non-empty block, so this is dstSize_tooSmall (70),
        // not dstBuffer_null (74).
        diff(&format!("decompress {name} dst=NULL cap=0"), |l| {
            dec_raw(l, std::ptr::null_mut(), 0, b)
        });
    }
    // dstBuffer_null (74) needs dst == NULL with dstCapacity != 0, which the
    // one-shot API cannot express (it would be UB to pass a capacity for a NULL
    // buffer through ZSTD_decompress), but ZSTD_decompressContinue can:
    // feed the header, then the block header, then the block with dst == NULL.
    diff("decompressContinue raw dst=NULL cap=8", |l| {
        let d = Ctx::dctx(l);
        continue_script(
            l,
            d.ptr,
            &raw8,
            std::ptr::null_mut(),
            8,
        )
    });
    diff("decompressContinue rle dst=NULL cap=100", |l| {
        let d = Ctx::dctx(l);
        continue_script(l, d.ptr, &rle100, std::ptr::null_mut(), 100)
    });
    // and an *empty* raw block with dst == NULL, which the C accepts (returns 0)
    let raw0 = empty_frame_wlog10();
    diff("decompressContinue raw0 dst=NULL cap=8", |l| {
        let d = Ctx::dctx(l);
        continue_script(l, d.ptr, &raw0, std::ptr::null_mut(), 8)
    });
    diff("decompress raw0 dst=NULL cap=0", |l| {
        dec_raw(l, std::ptr::null_mut(), 0, &raw0)
    });
}

/// Drive `ZSTD_decompressBegin` + `ZSTD_decompressContinue` over `frame`,
/// always supplying exactly `ZSTD_nextSrcSizeToDecompress` bytes. Returns the
/// per-step statuses so a divergence names the step.
fn continue_script(
    l: &Lib,
    dctx: *mut c_void,
    frame: &[u8],
    dst: *mut c_void,
    cap: usize,
) -> Vec<(usize, R)> {
    let begin = l.sym::<FnSzPtr>("ZSTD_decompressBegin");
    let next = l.sym::<FnSzPtr>("ZSTD_nextSrcSizeToDecompress");
    let cont = l.sym::<FnDecompressContinue>("ZSTD_decompressContinue");
    let mut out = Vec::new();
    out.push((usize::MAX, res(l, unsafe { begin(dctx) })));
    let mut pos = 0usize;
    for _ in 0..64 {
        let want = unsafe { next(dctx) };
        if want == 0 {
            out.push((0, R::Ok(0)));
            break;
        }
        if pos + want > frame.len() {
            out.push((want, R::Ok(usize::MAX))); // not enough input left
            break;
        }
        let n = unsafe {
            cont(
                dctx,
                dst,
                cap,
                frame[pos..].as_ptr() as *const c_void,
                want,
            )
        };
        let r = res(l, n);
        out.push((want, r.clone()));
        if matches!(r, R::Err(..)) {
            break;
        }
        pos += want;
    }
    out
}

// ===========================================================================
// decompress/zstd_decompress.c — checksum (1050/1055/1406)
// ===========================================================================

#[test]
fn zd_checksum_errors_and_force_ignore() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1050",
        "ERR:decompress/zstd_decompress.c:1055",
        "ERR:decompress/zstd_decompress.c:1406",
        "ERR:decompress/zstd_decompress.c:788",
        "ERR:decompress/zstd_decompress.c:1924",
    ]);
    let src = corpus(Corpus::Text, 4000, 31);
    let good = c_compress_params(
        &src,
        &[(ZSTD_c_checksumFlag, 1), (ZSTD_c_compressionLevel, 3)],
    );
    let n = good.len();
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // every single-bit flip of the 4 checksum bytes -> checksum_wrong (22)
    for byte in 0..4usize {
        for bit in 0..8u32 {
            let mut b = good.clone();
            b[n - 4 + byte] ^= 1 << bit;
            cases.push((format!("flip cks[{byte}] bit{bit}"), b));
        }
    }
    // TRUNCATED checksum: the C deliberately reports checksum_wrong (22) rather
    // than srcSize_wrong, because `remainingSrcSize < 4` is checked *after* the
    // content-size check (zstd_decompress.c:1050).
    for drop in 1..=4usize {
        cases.push((format!("checksum -{drop}B"), good[..n - drop].to_vec()));
    }
    cases.push(("intact".into(), good.clone()));
    for (name, b) in cases {
        for ignore in [0, 1] {
            diff(&format!("decompressDCtx {name} ignore={ignore}"), |l| {
                let d = Ctx::dctx(l);
                let sp = set_dparam(l, d.ptr, ZSTD_d_forceIgnoreChecksum, ignore);
                (sp, dec_dctx(l, d.ptr, &b, src.len() + 64))
            });
            diff(&format!("decompressStream {name} ignore={ignore}"), |l| {
                let ds = Ctx::dstream(l);
                let sp = set_dparam(l, ds.ptr, ZSTD_d_forceIgnoreChecksum, ignore);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                // a small output buffer forces the buffered path, so the
                // checksum is verified in ZSTD_decompressContinue:1406 rather
                // than through the single-pass shortcut.
                let mut out = vec![0xCDu8; 512];
                let mut consumed = 0usize;
                let mut last = R::Ok(0);
                let mut total = 0usize;
                for _ in 0..4096 {
                    let mut ob = ZSTD_outBuffer {
                        dst: out.as_mut_ptr() as *mut c_void,
                        size: out.len(),
                        pos: 0,
                    };
                    let mut ib = ZSTD_inBuffer {
                        src: unsafe { b.as_ptr().add(consumed) } as *const c_void,
                        size: b.len() - consumed,
                        pos: 0,
                    };
                    let r = unsafe { f(ds.ptr, &mut ob, &mut ib) };
                    last = res(l, r);
                    consumed += ib.pos;
                    total += ob.pos;
                    match last {
                        R::Err(..) | R::Ok(0) => break,
                        _ => {}
                    }
                    if ib.pos == 0 && ob.pos == 0 {
                        break;
                    }
                }
                (sp, last, consumed, total)
            });
        }
        diff(&format!("decompressContinue {name}"), |l| {
            let d = Ctx::dctx(l);
            let mut dst = vec![0xCDu8; src.len() + 64];
            continue_script(l, d.ptr, &b, dst.as_mut_ptr() as *mut c_void, dst.len())
        });
        diff(&format!("findFrameCompressedSize {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
    }
    // out-of-range ZSTD_d_forceIgnoreChecksum values
    for v in [-1, 2, 999] {
        diff(&format!("set forceIgnoreChecksum={v}"), |l| {
            let d = Ctx::dctx(l);
            set_dparam(l, d.ptr, ZSTD_d_forceIgnoreChecksum, v)
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_decompressMultiFrame (1126/1146/1157/1166)
// and the legacy branch (1093/1094)
// ===========================================================================

#[test]
fn zd_multi_frame_and_trailing_bytes() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1126",
        "ERR:decompress/zstd_decompress.c:1146",
        "ERR:decompress/zstd_decompress.c:1157",
        "ERR:decompress/zstd_decompress.c:1166",
    ]);
    let src = corpus(Corpus::Text, 1500, 41);
    let f1 = c_compress(&src, 3);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // 1146: one complete frame then 9 bytes that cannot start a frame ->
    // prefix_unknown is re-mapped to srcSize_wrong because moreThan1Frame == 1
    cases.push((
        "frame + 9 zeros".into(),
        [&f1[..], &[0u8; 9][..]].concat(),
    ));
    cases.push((
        "frame + 32 zeros".into(),
        [&f1[..], &[0u8; 32][..]].concat(),
    ));
    // 1166: 1..4 trailing bytes (too few to even look at a magic)
    for k in 1..=4usize {
        cases.push((format!("frame + {k}B"), [&f1[..], &vec![0xAAu8; k][..]].concat()));
    }
    // 1157: second frame is corrupt
    {
        let mut second = f1.clone();
        let hs = frame_header_size(&second);
        second[hs..hs + 3].copy_from_slice(&block_header(false, bt_reserved, 0));
        cases.push(("frame + corrupt frame".into(), [&f1[..], &second[..]].concat()));
    }
    // 1126: skippable frame with a bad declared length in the middle
    cases.push((
        "frame + bad skippable".into(),
        [
            &f1[..],
            &le32(SKIPPABLE_BASE)[..],
            &le32(64)[..],
        ]
        .concat(),
    ));
    cases.push((
        "frame + skippable 0xFFFFFFFF".into(),
        [&f1[..], &le32(SKIPPABLE_BASE)[..], &le32(0xFFFF_FFFF)[..]].concat(),
    ));
    // two good frames, then a good frame + truncated frame
    cases.push(("frame x2".into(), [&f1[..], &f1[..]].concat()));
    cases.push((
        "frame + half frame".into(),
        [&f1[..], &f1[..f1.len() / 2]].concat(),
    ));
    for (name, b) in cases {
        for cap in [0usize, 16, src.len(), 2 * src.len() + 64] {
            diff(&format!("decompress {name} cap={cap}"), |l| {
                dec_full(l, &b, cap)
            });
        }
        diff(&format!("decompressStream {name}"), |l| {
            stream_all(l, &b, 2 * src.len() + 64, 4096)
        });
        diff(&format!("findDecompressedSize {name}"), |l| {
            u64_of(l, "ZSTD_findDecompressedSize", &b)
        });
        diff(&format!("decompressBound {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
    }
}

/// The `ZSTD_LEGACY_SUPPORT=5` branch of `ZSTD_decompressMultiFrame` /
/// `ZSTD_decompressStream`. Only *structurally minimal* legacy frames are used:
/// `ZSTD_findFrameCompressedSizeLegacy` on arbitrary bytes is safe (it only
/// walks block headers), and `25 B5 2F FD 00 00 00 00` is a complete, valid,
/// empty v0.5 frame — verified against the C. Arbitrary legacy *payloads* are
/// NOT exercised: the v05/v06/v07 decoders are not hardened.
#[test]
fn zd_legacy_frames() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1093",
        "ERR:decompress/zstd_decompress.c:1094",
        "ERR:decompress/zstd_decompress.c:1098",
        "ERR:decompress/zstd_decompress.c:1104",
        "ERR:decompress/zstd_decompress.c:2150",
    ]);
    let empty_v05: Vec<u8> = vec![0x25, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x00, 0x00];
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("v05 empty".into(), empty_v05.clone()));
    // A minimal *valid* v0.6 frame. Layout: magic(4) + Frame_Descriptor
    // (bits 0-3 windowLog-10, bit 5 reserved, bits 6-7 fcsId) + the fcs field +
    // blocks. A v0.6 block header is `blockType = in[0] >> 6`
    // (0 compressed / 1 raw / 2 rle / 3 end) and
    // `cSize = in[2] + (in[1]<<8) + ((in[0]&7)<<16)`.
    // 1104: fcsId 1 declares 5 bytes but the frame decodes to 0
    //       -> "Frame header size does not match decoded size!" (20)
    cases.push((
        "v06 fcs=5 empty body".into(),
        vec![0x26, 0xB5, 0x2F, 0xFD, 0x40, 0x05, 0x00, 0x00, 0x00],
    ));
    cases.push((
        "v06 fcs=255 empty body".into(),
        vec![0x26, 0xB5, 0x2F, 0xFD, 0x40, 0xFF, 0x00, 0x00, 0x00],
    ));
    cases.push((
        "v06 fcs=0 empty body".into(),
        vec![0x26, 0xB5, 0x2F, 0xFD, 0x40, 0x00, 0x00, 0x00, 0x00],
    ));
    // 1098: a well-formed v0.6 frame with one 8-byte *raw* block (a plain
    // memcpy, so the legacy decoder stays inside hardened code) — with a dst
    // smaller than 8 bytes `ZSTD_decompressLegacy` returns dstSize_tooSmall,
    // which is what line 1098 forwards.
    cases.push((
        "v06 raw8".into(),
        [
            &[0x26u8, 0xB5, 0x2F, 0xFD, 0x40, 0x08][..], // fcsId 1, contentSize 8
            &[0x40u8, 0x00, 0x08][..],                   // bt_raw, cSize 8
            b"ABCDEFGH",
            &[0xC0u8, 0x00, 0x00][..], // bt_end
        ]
        .concat(),
    ));
    cases.push((
        "v06 raw8 fcs=0".into(),
        [
            &[0x26u8, 0xB5, 0x2F, 0xFD, 0x00][..], // fcsId 0 -> size unknown
            &[0x40u8, 0x00, 0x08][..],
            b"ABCDEFGH",
            &[0xC0u8, 0x00, 0x00][..],
        ]
        .concat(),
    ));
    // reserved bit 5 of the v0.6 frame descriptor
    cases.push((
        "v06 reserved bit".into(),
        vec![0x26, 0xB5, 0x2F, 0xFD, 0x20, 0x00, 0x00, 0x00],
    ));
    // 1093: ZSTD_findFrameCompressedSizeLegacy fails (truncated block scan)
    for m in LEGACY_MAGICS {
        for n in 4..=8usize {
            let mut v = le32(m).to_vec();
            v.resize(n, 0x00);
            cases.push((format!("{m:08x}[..{n}]"), v));
        }
        // a block header declaring more bytes than are present
        let mut v = le32(m).to_vec();
        v.extend_from_slice(&[0x00, 0xFF, 0xFF, 0x00]);
        cases.push((format!("{m:08x} oversized block"), v));
    }
    for (name, b) in cases {
        for cap in [0usize, 1, 4, 7, 8, 16, 4096] {
            diff(&format!("decompress legacy {name} cap={cap}"), |l| {
                dec_full(l, &b, cap)
            });
        }
        diff(&format!("isFrame legacy {name}"), |l| {
            u32_of(l, "ZSTD_isFrame", &b)
        });
        diff(&format!("getFrameContentSize legacy {name}"), |l| {
            u64_of(l, "ZSTD_getFrameContentSize", &b)
        });
        diff(&format!("findFrameCompressedSize legacy {name}"), |l| {
            sz_of(l, "ZSTD_findFrameCompressedSize", &b)
        });
        diff(&format!("decompressBound legacy {name}"), |l| {
            u64_of(l, "ZSTD_decompressBound", &b)
        });
        // 1094 / 2150: legacy support is refused on a static context
        diff(&format!("static dctx legacy {name}"), |l| {
            let (mut _ws, p) = static_dctx(l);
            if p.is_null() {
                return R::Ok(usize::MAX);
            }
            let d = Ctx::from_raw(l, p, "ZSTD_freeDCtx");
            let (r, _) = dec_dctx(l, d.ptr, &b, 4096);
            std::mem::forget(d); // a static DCtx must not be freed (see 327)
            r
        });
        // the ordinary (non-static) streaming path: this is where
        // `ZSTD_initLegacyStream` is called (zstd_decompress.c:2152). It can only
        // fail on an allocation, and the legacy contexts are allocated with the
        // *default* allocator (`ZBUFFv05_createDCtx`), so there is no injection
        // point for the failure branch; only the success path is asserted here.
        diff(&format!("dstream legacy {name}"), |l| {
            let ds = Ctx::dstream(l);
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; 4096];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: b.as_ptr() as *const c_void,
                size: b.len(),
                pos: 0,
            };
            let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
            out.truncate(ob.pos);
            (res(l, n), ib.pos, Blob(out))
        });
        diff(&format!("static dstream legacy {name}"), |l| {
            let (mut _ws, p) = static_dstream(l, 1 << 18);
            if p.is_null() {
                return (R::Ok(usize::MAX), 0usize, Blob(Vec::new()));
            }
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; 4096];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: b.as_ptr() as *const c_void,
                size: b.len(),
                pos: 0,
            };
            let n = unsafe { f(p, &mut ob, &mut ib) };
            out.truncate(ob.pos);
            (res(l, n), ib.pos, Blob(out))
        });
    }
}

/// An 8-byte-aligned workspace plus a `ZSTD_initStaticDCtx` pointer into it.
/// The `Vec<u64>` must outlive the pointer, hence it is returned alongside.
fn static_dctx(l: &Lib) -> (Vec<u64>, *mut c_void) {
    let est = l.sym::<FnSzVoid>("ZSTD_estimateDCtxSize");
    let n = unsafe { est() };
    let mut ws = vec![0u64; n / 8 + 2];
    let init = l.sym::<FnInitStatic>("ZSTD_initStaticDCtx");
    let p = unsafe { init(ws.as_mut_ptr() as *mut c_void, n) };
    (ws, p)
}

fn static_dstream(l: &Lib, window: usize) -> (Vec<u64>, *mut c_void) {
    let est = l.sym::<FnSzSz>("ZSTD_estimateDStreamSize");
    let n = unsafe { est(window) };
    let mut ws = vec![0u64; n / 8 + 2];
    let init = l.sym::<FnInitStatic>("ZSTD_initStaticDStream");
    let p = unsafe { init(ws.as_mut_ptr() as *mut c_void, n) };
    (ws, p)
}

// ===========================================================================
// decompress/zstd_decompress.c — static contexts (223/285/286/327/700, 2256)
// and decompress/zstd_ddict.c static DDicts (198/199/204)
// ===========================================================================

#[test]
fn zd_static_contexts() {
    let _serial = serial_alloc_lock();
    covers(&[
        "ERR:decompress/zstd_decompress.c:223",
        "ERR:decompress/zstd_decompress.c:285",
        "ERR:decompress/zstd_decompress.c:286",
        "ERR:decompress/zstd_decompress.c:327",
        "ERR:decompress/zstd_decompress.c:1930",
        "ERR:decompress/zstd_decompress.c:2256",
        "ERR:decompress/zstd_ddict.c:198",
        "ERR:decompress/zstd_ddict.c:199",
        "ERR:decompress/zstd_ddict.c:204",
        "ERR:decompress/zstd_ddict.c:214",
        "ERR:decompress/zstd_ddict.c:232",
        "ERR:decompress/zstd_ddict.c:242",
    ]);
    // sizeof-NULL support
    diff("sizeof_DCtx(NULL)", |l| {
        let f = l.sym::<FnSzPtr>("ZSTD_sizeof_DCtx");
        unsafe { f(std::ptr::null()) }
    });
    diff("sizeof_DDict(NULL)", |l| {
        let f = l.sym::<FnSzPtr>("ZSTD_sizeof_DDict");
        unsafe { f(std::ptr::null()) }
    });
    diff("getDictID_fromDDict(NULL)", |l| {
        let f = l.sym::<FnU32Ptr>("ZSTD_getDictID_fromDDict");
        unsafe { f(std::ptr::null()) }
    });
    diff("freeDDict(NULL)", |l| {
        let f = l.sym::<FnSzPtr>("ZSTD_freeDDict");
        res(l, unsafe { f(std::ptr::null()) })
    });
    diff("freeDCtx(NULL)", |l| {
        let f = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
        res(l, unsafe { f(std::ptr::null()) })
    });
    diff("freeDStream(NULL)", |l| {
        let f = l.sym::<FnSzPtr>("ZSTD_freeDStream");
        res(l, unsafe { f(std::ptr::null()) })
    });

    // 285/286: misaligned workspace and undersized workspace -> NULL
    diff("initStaticDCtx alignment/size grid", |l| {
        let est = l.sym::<FnSzVoid>("ZSTD_estimateDCtxSize");
        let n = unsafe { est() };
        let mut ws = vec![0u64; n / 8 + 4];
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticDCtx");
        let base = ws.as_mut_ptr() as *mut u8;
        let mut out = Vec::new();
        for off in [0usize, 1, 2, 4, 7, 8] {
            for size in [0usize, 1, 8, n - 1, n, n + 8] {
                let p = unsafe { init(base.add(off) as *mut c_void, size) };
                out.push((off, size, !p.is_null()));
            }
        }
        (n, out)
    });
    diff("initStaticDStream alignment/size grid", |l| {
        let est = l.sym::<FnSzSz>("ZSTD_estimateDStreamSize");
        let n = unsafe { est(1 << 20) };
        let mut ws = vec![0u64; n / 8 + 4];
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticDStream");
        let base = ws.as_mut_ptr() as *mut u8;
        let mut out = Vec::new();
        for off in [0usize, 1, 8] {
            for size in [0usize, 8, n] {
                let p = unsafe { init(base.add(off) as *mut c_void, size) };
                out.push((off, size, !p.is_null()));
            }
        }
        (n, out)
    });
    // 327: freeing a static DCtx is an error (memory_allocation, 64);
    // 700: refMultipleDDicts is unsupported on a static DCtx (40);
    // 223: sizeof a static DCtx.
    diff("static dctx behaviours", |l| {
        let (mut ws, p) = static_dctx(l);
        assert!(!p.is_null());
        let szf = l.sym::<FnSzPtr>("ZSTD_sizeof_DCtx");
        let free = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
        let sz = unsafe { szf(p) };
        let rm = set_dparam(l, p, ZSTD_d_refMultipleDDicts, 1);
        let rm0 = set_dparam(l, p, ZSTD_d_refMultipleDDicts, 0);
        let wl = set_dparam(l, p, ZSTD_d_windowLogMax, 20);
        let fr = res(l, unsafe { free(p) });
        ws[0] = 0; // keep `ws` alive across the calls above
        (sz, rm, rm0, wl, fr)
    });
    // 2256: a static DStream whose workspace cannot hold the frame's buffers
    let src = corpus(Corpus::Text, 200_000, 51);
    let frame = c_compress(&src, 3);
    diff("static dstream too small for frame", |l| {
        let est = l.sym::<FnSzVoid>("ZSTD_estimateDCtxSize");
        let n = unsafe { est() } + 16;
        let mut ws = vec![0u64; n / 8 + 2];
        let init = l.sym::<FnInitStatic>("ZSTD_initStaticDStream");
        let p = unsafe { init(ws.as_mut_ptr() as *mut c_void, n) };
        if p.is_null() {
            return (R::Ok(usize::MAX), 0usize, 0usize);
        }
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        // a small out buffer prevents the single-pass shortcut, so the buffered
        // path must allocate inBuff/outBuff out of the static workspace
        let mut out = vec![0xCDu8; 1000];
        let mut ob = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: out.len(),
            pos: 0,
        };
        let mut ib = ZSTD_inBuffer {
            src: frame.as_ptr() as *const c_void,
            size: frame.len(),
            pos: 0,
        };
        let r = unsafe { f(p, &mut ob, &mut ib) };
        (res(l, r), ib.pos, ob.pos)
    });
    // zstd_ddict.c 198/199/204: static DDicts
    diff("initStaticDDict grid", |l| {
        let d = trained_dict();
        let est = l.sym::<
            unsafe extern "C" fn(SizeT, c_int) -> SizeT,
        >("ZSTD_estimateDDictSize");
        let by_copy = unsafe { est(d.len(), ZSTD_dlm_byCopy) };
        let by_ref = unsafe { est(d.len(), ZSTD_dlm_byRef) };
        let init = l.sym::<FnInitStaticDDict>("ZSTD_initStaticDDict");
        let mut ws = vec![0u64; by_copy / 8 + 4];
        let base = ws.as_mut_ptr() as *mut u8;
        let mut out = Vec::new();
        for off in [0usize, 1, 8] {
            for &size in &[0usize, 8, by_ref, by_copy] {
                for dlm in [ZSTD_dlm_byRef, ZSTD_dlm_byCopy] {
                    for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                        let p = unsafe {
                            init(
                                base.add(off) as *mut c_void,
                                size,
                                d.as_ptr() as *const c_void,
                                d.len(),
                                dlm,
                                dct,
                            )
                        };
                        out.push((off, size, dlm, dct, !p.is_null()));
                    }
                }
            }
        }
        // 204: a corrupt dictionary makes ZSTD_initDDict_internal fail -> NULL
        let bad = [&[0x37u8, 0xA4, 0x30, 0xEC][..], &[1, 0, 0, 0][..], &[0xFFu8; 24][..]].concat();
        let mut out2 = Vec::new();
        for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
            let p = unsafe {
                init(
                    base as *mut c_void,
                    by_copy,
                    bad.as_ptr() as *const c_void,
                    bad.len(),
                    ZSTD_dlm_byRef,
                    dct,
                )
            };
            out2.push((dct, !p.is_null()));
        }
        (by_copy, by_ref, out, out2)
    });
}

// ===========================================================================
// decompress/zstd_decompress.c — custom-allocator failure injection
// (139/212/298/1791/1795/2264) and decompress/zstd_ddict.c (133/150/153/158)
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

static ALLOC_N: AtomicUsize = AtomicUsize::new(0);
static ALLOC_FAIL_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
const ALLOC_HDR: usize = 16;

extern "C" fn t_alloc(_opaque: *mut c_void, size: SizeT) -> *mut c_void {
    let n = ALLOC_N.fetch_add(1, Ordering::SeqCst);
    if n >= ALLOC_FAIL_AT.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = std::alloc::Layout::from_size_align(size + ALLOC_HDR, 16).unwrap();
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut usize).write(size);
        p.add(ALLOC_HDR) as *mut c_void
    }
}

extern "C" fn t_free(_opaque: *mut c_void, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = (ptr as *mut u8).sub(ALLOC_HDR);
        let size = (base as *mut usize).read();
        let layout = std::alloc::Layout::from_size_align(size + ALLOC_HDR, 16).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

fn cmem_ok() -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: Some(t_alloc),
        customFree: Some(t_free),
        opaque: std::ptr::null_mut(),
    }
}

/// Exactly one of alloc/free NULL — rejected by
/// `(!customMem.customAlloc) ^ (!customMem.customFree)`.
fn cmem_half(alloc: bool) -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: if alloc { Some(t_alloc) } else { None },
        customFree: if alloc { None } else { Some(t_free) },
        opaque: std::ptr::null_mut(),
    }
}

fn with_alloc_failing_at<T>(k: usize, f: impl FnOnce() -> T) -> T {
    ALLOC_N.store(0, Ordering::SeqCst);
    ALLOC_FAIL_AT.store(k, Ordering::SeqCst);
    let r = f();
    ALLOC_FAIL_AT.store(usize::MAX, Ordering::SeqCst);
    r
}

#[test]
fn zd_custom_allocator_failures() {
    let _serial = serial_alloc_lock();
    covers(&[
        "ERR:decompress/zstd_decompress.c:295",
        "ERR:decompress/zstd_decompress.c:298",
        "ERR:decompress/zstd_decompress.c:1791",
        "ERR:decompress/zstd_decompress.c:1795",
        "ERR:decompress/zstd_decompress.c:2264",
        "ERR:decompress/zstd_ddict.c:133",
        "ERR:decompress/zstd_ddict.c:140",
        "ERR:decompress/zstd_ddict.c:150",
        "ERR:decompress/zstd_ddict.c:153",
        "ERR:decompress/zstd_ddict.c:158",
    ]);
    // 295 / zstd_ddict.c:150 — half-populated ZSTD_customMem
    for alloc_only in [true, false] {
        diff(&format!("createDCtx_advanced half-mem alloc={alloc_only}"), |l| {
            let f = l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced");
            let p = unsafe { f(cmem_half(alloc_only)) };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
                unsafe { fr(p) };
            }
            ok
        });
        diff(&format!("createDStream_advanced half-mem alloc={alloc_only}"), |l| {
            let f = l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced");
            let p = unsafe { f(cmem_half(alloc_only)) };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDStream");
                unsafe { fr(p) };
            }
            ok
        });
        diff(&format!("createDDict_advanced half-mem alloc={alloc_only}"), |l| {
            let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
            let d = trained_dict();
            let p = unsafe {
                f(
                    d.as_ptr() as *const c_void,
                    d.len(),
                    ZSTD_dlm_byRef,
                    ZSTD_dct_auto,
                    cmem_half(alloc_only),
                )
            };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                unsafe { fr(p) };
            }
            ok
        });
    }
    // 298 — the ZSTD_DCtx allocation itself fails
    diff("createDCtx_advanced alloc#0 fails", |l| {
        with_alloc_failing_at(0, || {
            let f = l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced");
            let p = unsafe { f(cmem_ok()) };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
                unsafe { fr(p) };
            }
            ok
        })
    });
    // zstd_ddict.c 153 (the DDict struct) and 133/140/158 (byCopy content)
    for k in [0usize, 1, 2] {
        for dlm in [ZSTD_dlm_byRef, ZSTD_dlm_byCopy] {
            diff(&format!("createDDict_advanced alloc#{k} fails dlm={dlm}"), |l| {
                with_alloc_failing_at(k, || {
                    let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                    let d = trained_dict();
                    let p = unsafe {
                        f(
                            d.as_ptr() as *const c_void,
                            d.len(),
                            dlm,
                            ZSTD_dct_auto,
                            cmem_ok(),
                        )
                    };
                    let ok = !p.is_null();
                    if ok {
                        let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                        unsafe { fr(p) };
                    }
                    ok
                })
            });
        }
    }
    // 2264 — ZSTD_decompressStream's inBuff/outBuff allocation fails
    let src = corpus(Corpus::Text, 200_000, 61);
    let frame = c_compress(&src, 3);
    diff("decompressStream buffer alloc fails", |l| {
        let f = l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced");
        let p = unsafe { f(cmem_ok()) };
        assert!(!p.is_null());
        let n_before = ALLOC_N.load(Ordering::SeqCst);
        let ds = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut out = vec![0xCDu8; 1000];
        let r = with_alloc_failing_at(n_before, || {
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void,
                size: frame.len(),
                pos: 0,
            };
            let v = unsafe { ds(p, &mut ob, &mut ib) };
            (res(l, v), ib.pos, ob.pos)
        });
        let fr = l.sym::<FnSzPtr>("ZSTD_freeDStream");
        unsafe { fr(p) };
        r
    });
    // 1791 — ZSTD_createDDictHashSet returns NULL because its *first*
    // (`ZSTD_customMalloc`) allocation fails. Distinct dictIDs are produced by
    // patching bytes 4..8 of a valid dictionary (the dictID field is only read
    // back, never validated).
    //
    // OUT OF CONTRACT (not tested): making any `ZSTD_customCalloc` call fail.
    // `common/allocations.h:39` does `ZSTD_memset(ptr, 0, size)` with **no NULL
    // check**, so an allocator that returns NULL for a calloc site crashes the
    // reference C. That is exactly what `zstd_decompress.c:139`
    // (`ZSTD_DDictHashSet_expand`) and the hash-set table allocation inside
    // `ZSTD_createDDictHashSet` are — verified by observing SIGSEGV in the C
    // `.so` when the 2nd (or the expand) allocation is refused.
    for fail_at_offset in [0usize] {
        diff(&format!("refDDict hashset alloc#+{fail_at_offset} fails"), |l| {
            let cf = l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced");
            let dctx = unsafe { cf(cmem_ok()) };
            assert!(!dctx.is_null());
            let rm = set_dparam(l, dctx, ZSTD_d_refMultipleDDicts, 1);
            let mk = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let refd = l.sym::<FnRefDDict>("ZSTD_DCtx_refDDict");
            let mut ddicts = Vec::new();
            for id in 1u32..=20 {
                let mut d = trained_dict().clone();
                d[4..8].copy_from_slice(&le32(id));
                let p = unsafe { mk(d.as_ptr() as *const c_void, d.len()) };
                assert!(!p.is_null());
                ddicts.push(p);
            }
            let base = ALLOC_N.load(Ordering::SeqCst);
            let mut out = Vec::new();
            ALLOC_FAIL_AT.store(base + fail_at_offset, Ordering::SeqCst);
            for (i, &p) in ddicts.iter().enumerate() {
                let r = res(l, unsafe { refd(dctx, p) });
                let err = matches!(r, R::Err(..));
                out.push((i, r));
                if err {
                    break;
                }
            }
            ALLOC_FAIL_AT.store(usize::MAX, Ordering::SeqCst);
            let fd = l.sym::<FnSzPtr>("ZSTD_freeDDict");
            let fdc = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
            unsafe { fdc(dctx) };
            for p in ddicts {
                unsafe { fd(p) };
            }
            (rm, out)
        });
    }
    // and the same without allocation failure, to walk the expand path itself
    diff("refDDict 20 distinct dictIDs (expand succeeds)", |l| {
        let cf = l.sym::<FnCreateAdvanced>("ZSTD_createDCtx_advanced");
        let dctx = unsafe { cf(cmem_ok()) };
        let rm = set_dparam(l, dctx, ZSTD_d_refMultipleDDicts, 1);
        let mk = l.sym::<FnCreateDDict>("ZSTD_createDDict");
        let refd = l.sym::<FnRefDDict>("ZSTD_DCtx_refDDict");
        let fd = l.sym::<FnSzPtr>("ZSTD_freeDDict");
        let fdc = l.sym::<FnSzPtr>("ZSTD_freeDCtx");
        let mut out = Vec::new();
        let mut ddicts = Vec::new();
        for id in 1u32..=40 {
            let mut d = trained_dict().clone();
            d[4..8].copy_from_slice(&le32(id));
            let p = unsafe { mk(d.as_ptr() as *const c_void, d.len()) };
            ddicts.push(p);
            out.push(res(l, unsafe { refd(dctx, p) }));
        }
        unsafe { fdc(dctx) };
        for p in ddicts {
            unsafe { fd(p) };
        }
        (rm, out)
    });
}

// ===========================================================================
// decompress/zstd_decompress.c — parameters
// 1810/1811 (setMaxWindowSize), 1857 (dParam_getBounds), 1874 (CHECK_DBOUNDS),
// 1903 (getParameter), 1912..1944 (setParameter), and out-of-range enums
// ===========================================================================

#[test]
fn zd_dparam_bounds_and_set_get_parameter() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1810",
        "ERR:decompress/zstd_decompress.c:1811",
        "ERR:decompress/zstd_decompress.c:1857",
        "ERR:decompress/zstd_decompress.c:1874",
        "ERR:decompress/zstd_decompress.c:1903",
        "ERR:decompress/zstd_decompress.c:1912",
        "ERR:decompress/zstd_decompress.c:1916",
        "ERR:decompress/zstd_decompress.c:1920",
        "ERR:decompress/zstd_decompress.c:1924",
        "ERR:decompress/zstd_decompress.c:1928",
        "ERR:decompress/zstd_decompress.c:1935",
        "ERR:decompress/zstd_decompress.c:1939",
        "ERR:decompress/zstd_decompress.c:1944",
    ]);
    // 1857: unsupported dParam ids
    let ids: Vec<c_int> = {
        let mut v: Vec<c_int> = ALL_DPARAMS.iter().map(|&(_, p)| p).collect();
        v.extend_from_slice(&[
            -1, 0, 1, 99, 100, 101, 999, 1006, 1007, 9999, i32::MIN, i32::MAX, 200, 1000,
        ]);
        v
    };
    for p in ids.iter().copied() {
        diff(&format!("dParam_getBounds({p})"), |l| {
            let f = l.sym::<FnDParamGetBounds>("ZSTD_dParam_getBounds");
            let b = unsafe { f(p) };
            (res(l, b.error), b.lowerBound, b.upperBound)
        });
        diff(&format!("DCtx_getParameter({p})"), |l| {
            let d = Ctx::dctx(l);
            let f = l.sym::<FnDCtxGetParam>("ZSTD_DCtx_getParameter");
            let mut v: c_int = -12345;
            (res(l, unsafe { f(d.ptr, p, &mut v) }), v)
        });
    }
    // 1874/1912..1944: every documented dParam against a wide value grid, then
    // read the value back so a silently-clamped value shows up as a divergence.
    let values: [c_int; 22] = [
        i32::MIN, -999, -2, -1, 0, 1, 2, 3, 7, 9, 10, 11, 27, 30, 31, 32, 1023, 1024, 131_072,
        131_073, 999_999, i32::MAX,
    ];
    for &(name, p) in ALL_DPARAMS {
        for v in values {
            diff(&format!("setParameter {name}={v}"), |l| {
                let d = Ctx::dctx(l);
                let set = set_dparam(l, d.ptr, p, v);
                let g = l.sym::<FnDCtxGetParam>("ZSTD_DCtx_getParameter");
                let mut got: c_int = -12345;
                let r = res(l, unsafe { g(d.ptr, p, &mut got) });
                (set, r, got)
            });
        }
    }
    // unknown parameter ids through setParameter (1944)
    for p in [-1, 0, 1, 99, 101, 1006, 9999, i32::MIN, i32::MAX] {
        for v in [0, 1, -1] {
            diff(&format!("setParameter unknown id {p}={v}"), |l| {
                let d = Ctx::dctx(l);
                set_dparam(l, d.ptr, p, v)
            });
        }
    }
    // 1810/1811: ZSTD_DCtx_setMaxWindowSize bounds
    for ws in [
        0usize,
        1,
        1023,
        1024,
        1025,
        1 << 27,
        (1usize << 31) - 1,
        1usize << 31,
        (1usize << 31) + 1,
        usize::MAX,
    ] {
        diff(&format!("setMaxWindowSize({ws})"), |l| {
            let d = Ctx::dctx(l);
            let f = l.sym::<FnPtrSz>("ZSTD_DCtx_setMaxWindowSize");
            res(l, unsafe { f(d.ptr, ws) })
        });
    }
    // ZSTD_DCtx_reset with out-of-enum directives (a C enum accepts any int)
    for r in [-1, 0, 1, 2, 3, 4, 999, i32::MIN, i32::MAX] {
        diff(&format!("DCtx_reset({r})"), |l| {
            let d = Ctx::dctx(l);
            let f = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
            res(l, unsafe { f(d.ptr, r) })
        });
    }
}

/// Out-of-range values for every enum that crosses the decompression FFI
/// boundary. A C enum parameter accepts any `int`, so these are real inputs.
#[test]
fn zd_out_of_range_enums() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1916",
        "ERR:decompress/zstd_decompress.c:1920",
        "ERR:decompress/zstd_decompress.c:1924",
        "ERR:decompress/zstd_decompress.c:1928",
        "ERR:decompress/zstd_decompress.c:1935",
        "ERR:decompress/zstd_decompress.c:1944",
        "ERR:decompress/zstd_ddict.c:99",
        "ERR:decompress/zstd_ddict.c:105",
    ]);
    let dict = trained_dict();
    let raw = corpus(Corpus::Text, 512, 71);
    let odd_enum: [c_int; 6] = [-1, 2, 3, 999, i32::MIN, i32::MAX];
    // ZSTD_dictContentType_e / ZSTD_dictLoadMethod_e through every consumer
    for dct in [
        ZSTD_dct_auto,
        ZSTD_dct_rawContent,
        ZSTD_dct_fullDict,
        3,
        -1,
        999,
    ] {
        for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef, 2, -1] {
            for (dname, d) in [("trained", dict as &Vec<u8>), ("raw", &raw)] {
                diff(&format!("createDDict_advanced {dname} dlm={dlm} dct={dct}"), |l| {
                    let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                    let p = unsafe {
                        f(
                            d.as_ptr() as *const c_void,
                            d.len(),
                            dlm,
                            dct,
                            ZSTD_customMem::default(),
                        )
                    };
                    let out = if p.is_null() {
                        (false, 0u32, 0usize)
                    } else {
                        let idf = l.sym::<FnU32Ptr>("ZSTD_getDictID_fromDDict");
                        let szf = l.sym::<FnSzPtr>("ZSTD_sizeof_DDict");
                        (true, unsafe { idf(p) }, unsafe { szf(p) })
                    };
                    if !p.is_null() {
                        let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                        unsafe { fr(p) };
                    }
                    out
                });
                diff(&format!("DCtx_loadDictionary_advanced {dname} dlm={dlm} dct={dct}"), |l| {
                    let c = Ctx::dctx(l);
                    let f = l.sym::<FnLoadDictAdv>("ZSTD_DCtx_loadDictionary_advanced");
                    res(l, unsafe {
                        f(c.ptr, d.as_ptr() as *const c_void, d.len(), dlm, dct)
                    })
                });
            }
            let _ = dlm;
        }
        diff(&format!("DCtx_refPrefix_advanced dct={dct}"), |l| {
            let c = Ctx::dctx(l);
            let f = l.sym::<FnRefPrefixAdv>("ZSTD_DCtx_refPrefix_advanced");
            res(l, unsafe {
                f(c.ptr, raw.as_ptr() as *const c_void, raw.len(), dct)
            })
        });
    }
    // every out-of-enum value for the boolean-ish dParams
    for &(name, p) in ALL_DPARAMS {
        for v in odd_enum {
            diff(&format!("setParameter {name}={v} (enum abuse)"), |l| {
                let d = Ctx::dctx(l);
                set_dparam(l, d.ptr, p, v)
            });
        }
    }
}

/// `stage_wrong` (60): every setter that refuses to run mid-stream.
/// 1704 / 1727 / 1754 / 1765 / 1782 / 1809 / 1908 / 1957.
#[test]
fn zd_stage_wrong_family() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1704",
        "ERR:decompress/zstd_decompress.c:1727",
        "ERR:decompress/zstd_decompress.c:1745",
        "ERR:decompress/zstd_decompress.c:1754",
        "ERR:decompress/zstd_decompress.c:1765",
        "ERR:decompress/zstd_decompress.c:1782",
        "ERR:decompress/zstd_decompress.c:1809",
        "ERR:decompress/zstd_decompress.c:1908",
        "ERR:decompress/zstd_decompress.c:1957",
        "ERR:decompress/zstd_decompress.c:1708",
    ]);
    let src = corpus(Corpus::Text, 200_000, 81);
    let frame = c_compress(&src, 3);
    let dict = trained_dict();
    // A corrupt-but-magic-bearing dictionary: `ZSTD_createDDict_advanced`
    // swallows the `dictionary_corrupted` error and returns NULL, so
    // `ZSTD_DCtx_loadDictionary` reports memory_allocation (64), not 30.
    let bad_dict = [
        &[0x37u8, 0xA4, 0x30, 0xEC][..],
        &le32(1)[..],
        &[0xFFu8; 24][..],
    ]
    .concat();

    // `mid` == true drives the stream partway into a frame first (streamStage
    // leaves zdss_init), so every guard below must fire.
    for mid in [false, true] {
        let prime = |l: &Lib, ds: *mut c_void| {
            if !mid {
                return;
            }
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; 64];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void,
                size: 64.min(frame.len()),
                pos: 0,
            };
            unsafe { f(ds, &mut ob, &mut ib) };
        };
        diff(&format!("loadDictionary mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
            let a = res(l, unsafe {
                f(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
            });
            let b = res(l, unsafe {
                f(ds.ptr, bad_dict.as_ptr() as *const c_void, bad_dict.len())
            });
            let g = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary_byReference");
            let c = res(l, unsafe {
                f(ds.ptr, std::ptr::null(), 0)
            });
            let d = res(l, unsafe {
                g(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
            });
            (a, b, c, d)
        });
        diff(&format!("refPrefix mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnLoadDict>("ZSTD_DCtx_refPrefix");
            res(l, unsafe {
                f(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
            })
        });
        diff(&format!("initDStream_usingDict mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnLoadDict>("ZSTD_initDStream_usingDict");
            let a = res(l, unsafe {
                f(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
            });
            let b = res(l, unsafe {
                f(ds.ptr, bad_dict.as_ptr() as *const c_void, bad_dict.len())
            });
            (a, b)
        });
        diff(&format!("initDStream mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnDStreamStub>("ZSTD_initDStream");
            let a = res(l, unsafe { f(ds.ptr) });
            let g = l.sym::<FnDStreamStub>("ZSTD_resetDStream");
            let b = res(l, unsafe { g(ds.ptr) });
            (a, b)
        });
        diff(&format!("initDStream_usingDDict mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let mk = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let dd = unsafe { mk(dict.as_ptr() as *const c_void, dict.len()) };
            let f = l.sym::<FnRefDDict>("ZSTD_initDStream_usingDDict");
            let a = res(l, unsafe { f(ds.ptr, dd) });
            let g = l.sym::<FnRefDDict>("ZSTD_DCtx_refDDict");
            let b = res(l, unsafe { g(ds.ptr, dd) });
            let c = res(l, unsafe { g(ds.ptr, std::ptr::null()) });
            let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
            unsafe { fr(dd) };
            (a, b, c)
        });
        diff(&format!("setMaxWindowSize mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnPtrSz>("ZSTD_DCtx_setMaxWindowSize");
            (
                res(l, unsafe { f(ds.ptr, 1 << 20) }),
                res(l, unsafe { f(ds.ptr, 1023) }),
            )
        });
        diff(&format!("setParameter mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let mut out = Vec::new();
            for &(_, p) in ALL_DPARAMS {
                out.push(set_dparam(l, ds.ptr, p, 1));
            }
            out
        });
        diff(&format!("DCtx_reset mid={mid}"), |l| {
            let ds = Ctx::dstream(l);
            prime(l, ds.ptr);
            let f = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
            let a = res(l, unsafe { f(ds.ptr, ZSTD_reset_parameters) });
            let b = res(l, unsafe { f(ds.ptr, ZSTD_reset_session_only) });
            let c = res(l, unsafe { f(ds.ptr, ZSTD_reset_parameters) });
            (a, b, c)
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_decompressStream
// 2100/2105/2111/2049 (buffer validation), 2065/2076/2193/2283/2317 (forwarding),
// 2209 (stable out too small), 2231 (window too large), 2359/2360 (no progress)
// ===========================================================================

#[test]
fn zd_decompress_stream_buffer_validation() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:2049",
        "ERR:decompress/zstd_decompress.c:2100",
        "ERR:decompress/zstd_decompress.c:2105",
        "ERR:decompress/zstd_decompress.c:2111",
        "ERR:decompress/zstd_decompress.c:2209",
    ]);
    let src = corpus(Corpus::Text, 5000, 91);
    let frame = c_compress(&src, 3);
    // 2100: input->pos > input->size ; 2105: output->pos > output->size
    for (opos, osize, ipos, isize_) in [
        (0usize, 64usize, 0usize, 8usize),
        (8, 4, 0, 8),
        (65, 64, 0, 8),
        (0, 64, 8, 4),
        (0, 64, 9, 8),
        (5, 4, 9, 8),
        (0, 0, 0, 0),
    ] {
        diff(
            &format!("decompressStream o=({opos}/{osize}) i=({ipos}/{isize_})"),
            |l| {
                let ds = Ctx::dstream(l);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xCDu8; 128];
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: osize,
                    pos: opos,
                };
                let mut ib = ZSTD_inBuffer {
                    src: frame.as_ptr() as *const c_void,
                    size: isize_,
                    pos: ipos,
                };
                let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
                (res(l, n), ib.pos, ob.pos)
            },
        );
        diff(
            &format!("simpleArgs o=({opos}/{osize}) i=({ipos}/{isize_})"),
            |l| {
                let d = Ctx::dctx(l);
                let f = l.sym::<FnSimpleArgs>("ZSTD_decompressStream_simpleArgs");
                let mut out = vec![0xCDu8; 128];
                let mut dpos = opos;
                let mut spos = ipos;
                let n = unsafe {
                    f(
                        d.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        osize,
                        &mut dpos,
                        frame.as_ptr() as *const c_void,
                        isize_,
                        &mut spos,
                    )
                };
                (res(l, n), dpos, spos)
            },
        );
    }
    // 2049/2111: ZSTD_d_stableOutBuffer then a *different* output buffer
    diff("stableOutBuffer changed dst", |l| {
        let ds = Ctx::dstream(l);
        let sp = set_dparam(l, ds.ptr, ZSTD_d_stableOutBuffer, 1);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut o1 = vec![0xCDu8; src.len() + 64];
        let mut o2 = vec![0xCDu8; src.len() + 64];
        let mut ob = ZSTD_outBuffer {
            dst: o1.as_mut_ptr() as *mut c_void,
            size: o1.len(),
            pos: 0,
        };
        let mut ib = ZSTD_inBuffer {
            src: frame.as_ptr() as *const c_void,
            size: 10.min(frame.len()),
            pos: 0,
        };
        let r1 = res(l, unsafe { f(ds.ptr, &mut ob, &mut ib) });
        // same content, different address -> dstBuffer_wrong (104)
        let mut ob2 = ZSTD_outBuffer {
            dst: o2.as_mut_ptr() as *mut c_void,
            size: o2.len(),
            pos: 0,
        };
        let mut ib2 = ZSTD_inBuffer {
            src: unsafe { frame.as_ptr().add(10) } as *const c_void,
            size: frame.len() - 10,
            pos: 0,
        };
        let r2 = res(l, unsafe { f(ds.ptr, &mut ob2, &mut ib2) });
        // and with a changed *size* on the original buffer
        let mut ob3 = ZSTD_outBuffer {
            dst: o1.as_mut_ptr() as *mut c_void,
            size: o1.len() - 1,
            pos: 0,
        };
        let r3 = res(l, unsafe { f(ds.ptr, &mut ob3, &mut ib2) });
        (sp, r1, r2, r3)
    });
    // 2209: stable out buffer smaller than the declared frameContentSize
    for cap in [0usize, 1, 100, src.len() - 1, src.len(), src.len() + 1] {
        diff(&format!("stableOutBuffer cap={cap}"), |l| {
            let ds = Ctx::dstream(l);
            let sp = set_dparam(l, ds.ptr, ZSTD_d_stableOutBuffer, 1);
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; cap.max(1)];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: cap,
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void,
                size: frame.len(),
                pos: 0,
            };
            let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
            out.truncate(ob.pos);
            (sp, res(l, n), ib.pos, ob.pos, Blob(out))
        });
    }
    // 2076: stable-out mode forwarding a ZSTD_decompressContinue error
    let mut corrupt = frame.clone();
    let n = corrupt.len();
    corrupt[n - 1] ^= 0xFF;
    for stable in [0, 1] {
        diff(&format!("corrupt tail stable={stable}"), |l| {
            let ds = Ctx::dstream(l);
            let sp = set_dparam(l, ds.ptr, ZSTD_d_stableOutBuffer, stable);
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; src.len() + 64];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut consumed = 0usize;
            let mut last = R::Ok(0);
            for _ in 0..4096 {
                let end = (consumed + 37).min(corrupt.len());
                let mut ib = ZSTD_inBuffer {
                    src: unsafe { corrupt.as_ptr().add(consumed) } as *const c_void,
                    size: end - consumed,
                    pos: 0,
                };
                let r = unsafe { f(ds.ptr, &mut ob, &mut ib) };
                last = res(l, r);
                consumed += ib.pos;
                if matches!(last, R::Err(..)) || matches!(last, R::Ok(0)) {
                    break;
                }
                if ib.pos == 0 && consumed >= corrupt.len() {
                    break;
                }
            }
            out.truncate(ob.pos);
            (sp, last, consumed, ob.pos, Blob(out))
        });
    }
}

/// 2231: `MAX(windowSize, 1<<10) > maxWindowSize`, both with the default limit
/// ((1<<27)+1) and with a lowered one.
#[test]
fn zd_stream_window_too_large() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:2231",
        "ERR:decompress/zstd_decompress.c:2161",
        "ERR:decompress/zstd_decompress.c:2174",
    ]);
    // FHD 0x00 (no singleSegment), WD encoding windowLog 10..=31
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for wl in 10u32..=31 {
        let wd = ((wl - 10) << 3) as u8;
        let mut v = frame_hdr(0x00, &[wd]);
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        cases.push((format!("windowLog={wl}"), v));
    }
    // a real frame whose declared windowSize is huge (singleSegment + fcs)
    cases.push(("singleSegment 1<<40".into(), {
        let mut v = hdr_single_fcs8(1u64 << 40);
        v.extend_from_slice(&block_header(true, bt_raw, 0));
        v
    }));
    for (name, b) in cases {
        for limit in [0usize, 1024, 1 << 20, 1usize << 27] {
            diff(&format!("decompressStream {name} maxWindow={limit}"), |l| {
                let ds = Ctx::dstream(l);
                let f = l.sym::<FnPtrSz>("ZSTD_DCtx_setMaxWindowSize");
                let sp = if limit == 0 {
                    R::Ok(0)
                } else {
                    res(l, unsafe { f(ds.ptr, limit) })
                };
                let g = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xCDu8; 4096];
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut ib = ZSTD_inBuffer {
                    src: b.as_ptr() as *const c_void,
                    size: b.len(),
                    pos: 0,
                };
                let n = unsafe { g(ds.ptr, &mut ob, &mut ib) };
                (sp, res(l, n), ib.pos, ob.pos)
            });
            diff(&format!("windowLogMax dParam {name} limit={limit}"), |l| {
                let ds = Ctx::dstream(l);
                let sp = set_dparam(l, ds.ptr, ZSTD_d_windowLogMax, 10);
                let g = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xCDu8; 4096];
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: out.len(),
                    pos: 0,
                };
                let mut ib = ZSTD_inBuffer {
                    src: b.as_ptr() as *const c_void,
                    size: b.len(),
                    pos: 0,
                };
                let n = unsafe { g(ds.ptr, &mut ob, &mut ib) };
                (sp, res(l, n), ib.pos, ob.pos)
            });
        }
    }
}

/// 2359 (`noForwardProgress_destFull`, 80) and 2360
/// (`noForwardProgress_inputEmpty`, 82) — 16 consecutive no-progress calls.
#[test]
fn zd_no_forward_progress() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:2359",
        "ERR:decompress/zstd_decompress.c:2360",
    ]);
    let src = corpus(Corpus::Text, 200_000, 101);
    let frame = c_compress(&src, 3);
    // destFull: out.size == 0 for 20 calls with plenty of input available
    diff("noForwardProgress destFull", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut out = vec![0xCDu8; 16];
        let mut ob = ZSTD_outBuffer {
            dst: out.as_mut_ptr() as *mut c_void,
            size: 0,
            pos: 0,
        };
        let mut log = Vec::new();
        for _ in 0..24 {
            let mut ib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void,
                size: frame.len(),
                pos: 0,
            };
            let r = res(l, unsafe { f(ds.ptr, &mut ob, &mut ib) });
            let err = matches!(r, R::Err(..));
            log.push((r, ib.pos, ob.pos));
            if err {
                break;
            }
        }
        log
    });
    // inputEmpty: the stream must already be *past* the frame header, because
    // the header-loading stage returns early and never touches the
    // noForwardProgress counter. Priming with `hs + 2` bytes gets us to
    // zdss_read/zdss_load, after which empty inputs make no progress.
    let hs = frame_header_size(&frame);
    for prime in [hs, hs + 2, hs + 20] {
        diff(&format!("noForwardProgress inputEmpty prime={prime}"), |l| {
            let ds = Ctx::dstream(l);
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut out = vec![0xCDu8; 300_000];
            let mut ob = ZSTD_outBuffer {
                dst: out.as_mut_ptr() as *mut c_void,
                size: out.len(),
                pos: 0,
            };
            let mut ib = ZSTD_inBuffer {
                src: frame.as_ptr() as *const c_void,
                size: prime,
                pos: 0,
            };
            let first = res(l, unsafe { f(ds.ptr, &mut ob, &mut ib) });
            let mut log = vec![(first, ib.pos, ob.pos)];
            for _ in 0..24 {
                let mut ib2 = ZSTD_inBuffer {
                    src: frame.as_ptr() as *const c_void,
                    size: 0,
                    pos: 0,
                };
                let r = res(l, unsafe { f(ds.ptr, &mut ob, &mut ib2) });
                let err = matches!(r, R::Err(..));
                log.push((r, ib2.pos, ob.pos));
                if err {
                    break;
                }
            }
            log
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_decompressContinue state machine
// 1279 (exact srcSize), 1314/1315 (block header), 1354/1366/1367, 1380, 1406
// plus ZSTD_insertBlock and ZSTD_nextSrcSizeToDecompress
// ===========================================================================

#[test]
fn zd_decompress_continue_state_machine() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1279",
        "ERR:decompress/zstd_decompress.c:1314",
        "ERR:decompress/zstd_decompress.c:1315",
        "ERR:decompress/zstd_decompress.c:1354",
        "ERR:decompress/zstd_decompress.c:1366",
        "ERR:decompress/zstd_decompress.c:1367",
        "ERR:decompress/zstd_decompress.c:1380",
        "ERR:decompress/zstd_decompress.c:1406",
        "ERR:decompress/zstd_decompress_block.c:74",
    ]);
    let src = corpus(Corpus::Text, 4000, 111);
    let good = c_compress(&src, 3);
    let cks = c_compress_params(
        &src,
        &[(ZSTD_c_checksumFlag, 1), (ZSTD_c_compressionLevel, 3)],
    );
    // 1279: supply the wrong number of bytes at each stage
    for delta in [-1i64, 1, -4] {
        diff(&format!("continue wrong srcSize delta={delta}"), |l| {
            let d = Ctx::dctx(l);
            let begin = l.sym::<FnSzPtr>("ZSTD_decompressBegin");
            let next = l.sym::<FnSzPtr>("ZSTD_nextSrcSizeToDecompress");
            let cont = l.sym::<FnDecompressContinue>("ZSTD_decompressContinue");
            let mut dst = vec![0xCDu8; src.len() + 64];
            let b0 = res(l, unsafe { begin(d.ptr) });
            let want = unsafe { next(d.ptr) };
            let give = (want as i64 + delta).max(0) as usize;
            let n = unsafe {
                cont(
                    d.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    good.as_ptr() as *const c_void,
                    give.min(good.len()),
                )
            };
            (b0, want, give, res(l, n))
        });
    }
    // 1314: a bt_reserved block header in the block-header stage
    // 1315: cBlockSize > blockSizeMax (windowLog 10 -> 1024)
    let mut frames: Vec<(String, Vec<u8>)> = Vec::new();
    frames.push(("good".into(), good.clone()));
    frames.push(("checksummed".into(), cks.clone()));
    {
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(false, bt_reserved, 4));
        v.extend_from_slice(&[0u8; 8]);
        frames.push(("reserved block".into(), v));
    }
    {
        // cBlockSize 2000 > blockSizeMax 1024
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_compressed, 2000));
        v.extend_from_slice(&vec![0u8; 2000]);
        frames.push(("cBlockSize 2000".into(), v));
    }
    {
        // bt_rle regenerating 2000 bytes > blockSizeMax 1024 (rSize check at 1367)
        let mut v = hdr_wlog10();
        v.extend_from_slice(&block_header(true, bt_rle, 2000));
        v.push(0x5A);
        frames.push(("rle 2000".into(), v));
    }
    {
        // 1380: declared frameContentSize does not match what was produced
        let mut v = good.clone();
        let fcs = u16::from_le_bytes([v[5], v[6]]).wrapping_add(3);
        v[5..7].copy_from_slice(&fcs.to_le_bytes());
        frames.push(("fcs+3".into(), v));
    }
    {
        // 1406: checksum stage mismatch
        let mut v = cks.clone();
        let n = v.len();
        v[n - 2] ^= 0x80;
        frames.push(("checksum flipped".into(), v));
    }
    for (name, b) in frames {
        for cap in [0usize, 4, 1024, src.len() + 64] {
            diff(&format!("continue {name} cap={cap}"), |l| {
                let d = Ctx::dctx(l);
                let mut dst = vec![0xCDu8; cap.max(1)];
                let p = if cap == 0 {
                    std::ptr::null_mut()
                } else {
                    dst.as_mut_ptr() as *mut c_void
                };
                let log = continue_script(l, d.ptr, &b, p, cap);
                // only compare `dst` when every step succeeded (see blob_if_ok)
                let ok = log.iter().all(|(_, r)| matches!(r, R::Ok(_)));
                dst.truncate(if ok { cap.min(4096) } else { 0 });
                (log, Blob(dst))
            });
        }
        // and via the skippable-frame stages
        let sk = [skippable(2, b"12345678"), b.clone()].concat();
        diff(&format!("continue skippable+{name}"), |l| {
            let d = Ctx::dctx(l);
            let mut dst = vec![0xCDu8; src.len() + 64];
            continue_script(l, d.ptr, &sk, dst.as_mut_ptr() as *mut c_void, dst.len())
        });
    }
    // ZSTD_insertBlock: registers an already-decoded block as history.
    diff("insertBlock", |l| {
        let d = Ctx::dctx(l);
        let begin = l.sym::<FnSzPtr>("ZSTD_decompressBegin");
        let ins = l.sym::<FnInsertBlock>("ZSTD_insertBlock");
        let next = l.sym::<FnSzPtr>("ZSTD_nextSrcSizeToDecompress");
        let b0 = res(l, unsafe { begin(d.ptr) });
        let hist = vec![0x11u8; 4096];
        let a = res(l, unsafe {
            ins(d.ptr, hist.as_ptr() as *const c_void, hist.len())
        });
        let b1 = res(l, unsafe { ins(d.ptr, std::ptr::null(), 0) });
        (b0, a, b1, unsafe { next(d.ptr) })
    });
}

// ===========================================================================
// decompress/zstd_decompress.c — ZSTD_loadDEntropy (1458..1531),
// ZSTD_decompress_insertDictionary (1550), ZSTD_decompressBegin_usingDict (1592),
// ZSTD_decompressMultiFrame's dict branch (1140)
// and decompress/zstd_ddict.c (99/105/112/158)
// ===========================================================================

/// `ZSTD_entropyDTables_t` is a private struct; 256 KiB of 8-byte-aligned
/// scratch is comfortably larger than it (LL/OF/ML seqSymbol tables ~10 KiB +
/// a 16 KiB hufTable + workspace), and both libraries get the identical buffer.
fn entropy_scratch() -> Vec<u64> {
    vec![0u64; 32 * 1024]
}

#[test]
fn zd_dictionary_entropy_loading() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:1458",
        "ERR:decompress/zstd_decompress.c:1477",
        "ERR:decompress/zstd_decompress.c:1484",
        "ERR:decompress/zstd_decompress.c:1486",
        "ERR:decompress/zstd_decompress.c:1499",
        "ERR:decompress/zstd_decompress.c:1501",
        "ERR:decompress/zstd_decompress.c:1514",
        "ERR:decompress/zstd_decompress.c:1516",
        "ERR:decompress/zstd_decompress.c:1526",
        "ERR:decompress/zstd_decompress.c:1531",
        "ERR:decompress/zstd_decompress.c:1550",
        "ERR:decompress/zstd_decompress.c:1592",
        "ERR:decompress/zstd_decompress.c:1140",
        "ERR:decompress/zstd_decompress.c:1708",
        "ERR:decompress/zstd_decompress.c:1745",
        "ERR:decompress/zstd_ddict.c:99",
        "ERR:decompress/zstd_ddict.c:105",
        "ERR:decompress/zstd_ddict.c:112",
        "ERR:decompress/zstd_ddict.c:140",
        "ERR:decompress/zstd_ddict.c:158",
        "ERR:decompress/huf_decompress.c:1204",
        "ERR:decompress/huf_decompress.c:1207",
    ]);
    let d = trained_dict();
    let ly = dict_layout();
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // 1458: dictSize <= 8 (every length up to and including the header)
    for n in 0..=9usize {
        cases.push((format!("dict[..{n}]"), d[..n].to_vec()));
    }
    // 1477: truncated inside the Huffman weight table -> HUF_readDTableX2_wksp fails
    for n in [ly.huf + 1, ly.huf + 2, (ly.huf + ly.off) / 2, ly.off - 1] {
        cases.push((format!("cut in HUF @{n}"), d[..n].to_vec()));
    }
    // 1484: truncated inside the offcode FSE table
    for n in [ly.off, ly.off + 1, ly.ml - 1] {
        cases.push((format!("cut in OF @{n}"), d[..n].to_vec()));
    }
    // 1499: truncated inside the matchlength FSE table
    for n in [ly.ml, ly.ml + 1, ly.ll - 1] {
        cases.push((format!("cut in ML @{n}"), d[..n].to_vec()));
    }
    // 1514: truncated inside the litlength FSE table
    for n in [ly.ll, ly.ll + 1, ly.rep - 1] {
        cases.push((format!("cut in LL @{n}"), d[..n].to_vec()));
    }
    // 1526: fewer than 12 bytes left for the 3 repcodes
    for n in ly.rep..ly.content {
        cases.push((format!("cut in repcodes @{n}"), d[..n].to_vec()));
    }
    // 1531: rep == 0, or rep > dictContentSize
    for (i, val) in [(0usize, 0u32), (1, 0), (2, 0), (0, 0xFFFF_FFFF), (2, 100_000)] {
        let mut b = d.clone();
        b[ly.rep + 4 * i..ly.rep + 4 * i + 4].copy_from_slice(&le32(val));
        cases.push((format!("rep[{i}]={val}"), b));
    }
    // 1486 / 1501 / 1516: a *valid* FSE distribution whose accuracy log exceeds
    // OffFSELog (8) / MLFSELog (9) / LLFSELog (9). The header parses fine, so the
    // only site that can reject it is the tableLog bound.
    {
        let mut b = d[..ly.off].to_vec();
        b.extend_from_slice(&fse_ncount(9, 31));
        b.extend_from_slice(&d[ly.ml..]);
        cases.push(("offcodeLog=9".into(), b));
    }
    {
        let mut b = d[..ly.ml].to_vec();
        b.extend_from_slice(&fse_ncount(10, 52));
        b.extend_from_slice(&d[ly.ll..]);
        cases.push(("matchlengthLog=10".into(), b));
    }
    {
        let mut b = d[..ly.ll].to_vec();
        b.extend_from_slice(&fse_ncount(10, 35));
        b.extend_from_slice(&d[ly.rep..]);
        cases.push(("litlengthLog=10".into(), b));
    }
    // corrupt Huffman weight header: a "direct weights" header (>= 128) with a
    // truncated payload, and an all-0xFF weight table
    {
        let mut b = d.clone();
        b[ly.huf] = 0xFF;
        cases.push(("huf hdr=0xFF".into(), b));
    }
    {
        let mut b = d.clone();
        for i in ly.huf..ly.off {
            b[i] = 0xFF;
        }
        cases.push(("huf all 0xFF".into(), b));
    }
    // a dictionary with the right magic but nothing else
    cases.push((
        "magic+id+garbage".into(),
        [&[0x37u8, 0xA4, 0x30, 0xEC][..], &le32(1)[..], &[0xFFu8; 24][..]].concat(),
    ));
    // pure content (no dictionary magic) -> accepted as raw content
    cases.push(("no magic".into(), vec![0x41u8; 64]));
    cases.push(("zeros 8".into(), vec![0u8; 8]));
    cases.push(("intact".into(), d.clone()));

    let frame = c_compress(&corpus(Corpus::Text, 2000, 121), 3);
    for (name, b) in cases {
        // ZSTD_loadDEntropy directly (only valid when the magic is present and
        // dictSize > 8; the C asserts the magic, which DEBUGLEVEL=0 removes, and
        // the function then simply skips the first 8 bytes, so any >8-byte input
        // is in contract).
        if b.len() > 8 {
            diff(&format!("loadDEntropy {name}"), |l| {
                let mut scratch = entropy_scratch();
                let f = l.sym::<FnLoadDEntropy>("ZSTD_loadDEntropy");
                res(l, unsafe {
                    f(
                        scratch.as_mut_ptr() as *mut c_void,
                        b.as_ptr() as *const c_void,
                        b.len(),
                    )
                })
            });
        }
        diff(&format!("decompressBegin_usingDict {name}"), |l| {
            let c = Ctx::dctx(l);
            let f = l.sym::<FnLoadDict>("ZSTD_decompressBegin_usingDict");
            res(l, unsafe {
                f(c.ptr, b.as_ptr() as *const c_void, b.len())
            })
        });
        // 1140: ZSTD_decompressMultiFrame forwards the dictionary error
        diff(&format!("decompress_usingDict {name}"), |l| {
            let c = Ctx::dctx(l);
            let f = l.sym::<FnDecompUsingDict>("ZSTD_decompress_usingDict");
            let mut dst = vec![0xCDu8; 4096];
            let n = unsafe {
                f(
                    c.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                    b.as_ptr() as *const c_void,
                    b.len(),
                )
            };
            let st = res(l, n);
            let bb = blob_if_ok(&st, dst);
            (st, bb)
        });
        // zstd_ddict.c 112/158: ZSTD_createDDict swallows the error -> NULL
        for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
            diff(&format!("createDDict_advanced {name} dct={dct}"), |l| {
                let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                let p = unsafe {
                    f(
                        b.as_ptr() as *const c_void,
                        b.len(),
                        ZSTD_dlm_byRef,
                        dct,
                        ZSTD_customMem::default(),
                    )
                };
                let out = if p.is_null() {
                    (false, 0u32)
                } else {
                    let idf = l.sym::<FnU32Ptr>("ZSTD_getDictID_fromDDict");
                    (true, unsafe { idf(p) })
                };
                if !p.is_null() {
                    let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                    unsafe { fr(p) };
                }
                out
            });
        }
        diff(&format!("createDDict {name}"), |l| {
            let f = l.sym::<FnCreateDDict>("ZSTD_createDDict");
            let p = unsafe { f(b.as_ptr() as *const c_void, b.len()) };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                unsafe { fr(p) };
            }
            ok
        });
        diff(&format!("createDDict_byReference {name}"), |l| {
            let f = l.sym::<FnCreateDDict>("ZSTD_createDDict_byReference");
            let p = unsafe { f(b.as_ptr() as *const c_void, b.len()) };
            let ok = !p.is_null();
            if ok {
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                unsafe { fr(p) };
            }
            ok
        });
        // 1708 / 1745: the NULL from ZSTD_createDDict_advanced surfaces as
        // memory_allocation (64), NOT dictionary_corrupted (30).
        diff(&format!("DCtx_loadDictionary {name}"), |l| {
            let c = Ctx::dctx(l);
            let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
            res(l, unsafe {
                f(c.ptr, b.as_ptr() as *const c_void, b.len())
            })
        });
        diff(&format!("initDStream_usingDict {name}"), |l| {
            let ds = Ctx::dstream(l);
            let f = l.sym::<FnLoadDict>("ZSTD_initDStream_usingDict");
            res(l, unsafe {
                f(ds.ptr, b.as_ptr() as *const c_void, b.len())
            })
        });
        diff(&format!("DCtx_refPrefix {name}"), |l| {
            let c = Ctx::dctx(l);
            let f = l.sym::<FnLoadDict>("ZSTD_DCtx_refPrefix");
            let a = res(l, unsafe {
                f(c.ptr, b.as_ptr() as *const c_void, b.len())
            });
            let mut dst = vec![0xCDu8; 4096];
            let (r, _) = dec_dctx(l, c.ptr, &frame, dst.len());
            dst.clear();
            (a, r)
        });
    }
    // zstd_ddict.c 99/105: dictContentType == ZSTD_dct_fullDict with a
    // too-small / wrong-magic dictionary content.
    for b in [
        vec![],
        vec![0u8; 1],
        vec![0u8; 4],
        vec![0u8; 7],
        vec![0u8; 8],
        vec![0u8; 9],
        vec![0x41u8; 64],
        [&le32(0xEC30_A437)[..], &[0u8; 4][..]].concat(),
    ] {
        for dlm in [ZSTD_dlm_byRef, ZSTD_dlm_byCopy] {
            diff(
                &format!("createDDict_advanced fullDict len={} dlm={dlm}", b.len()),
                |l| {
                    let f = l.sym::<FnCreateDDictAdv>("ZSTD_createDDict_advanced");
                    let p = unsafe {
                        f(
                            b.as_ptr() as *const c_void,
                            b.len(),
                            dlm,
                            ZSTD_dct_fullDict,
                            ZSTD_customMem::default(),
                        )
                    };
                    let ok = !p.is_null();
                    if ok {
                        let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                        unsafe { fr(p) };
                    }
                    ok
                },
            );
        }
    }
    // NULL dict pointers are accepted (treated as "no dictionary")
    diff("createDDict(NULL,0)", |l| {
        let f = l.sym::<FnCreateDDict>("ZSTD_createDDict");
        let p = unsafe { f(std::ptr::null(), 0) };
        let ok = !p.is_null();
        if ok {
            let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
            unsafe { fr(p) };
        }
        ok
    });
    diff("loadDictionary(NULL,0)", |l| {
        let c = Ctx::dctx(l);
        let f = l.sym::<FnLoadDict>("ZSTD_DCtx_loadDictionary");
        res(l, unsafe { f(c.ptr, std::ptr::null(), 0) })
    });
    diff("decompressBegin_usingDict(NULL,0)", |l| {
        let c = Ctx::dctx(l);
        let f = l.sym::<FnLoadDict>("ZSTD_decompressBegin_usingDict");
        res(l, unsafe { f(c.ptr, std::ptr::null(), 0) })
    });
    diff("decompressBegin_usingDDict(NULL)", |l| {
        let c = Ctx::dctx(l);
        let f = l.sym::<FnRefDDict>("ZSTD_decompressBegin_usingDDict");
        res(l, unsafe { f(c.ptr, std::ptr::null()) })
    });
    diff("decompress_usingDDict(NULL ddict)", |l| {
        let c = Ctx::dctx(l);
        let f = l.sym::<FnDecompUsingDDict>("ZSTD_decompress_usingDDict");
        let mut dst = vec![0xCDu8; 4096];
        let n = unsafe {
            f(
                c.ptr,
                dst.as_mut_ptr() as *mut c_void,
                dst.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                std::ptr::null(),
            )
        };
        let st = res(l, n);
        let bb = blob_if_ok(&st, dst);
        (st, bb)
    });
}

// ===========================================================================
// decompress/zstd_decompress_block.c — ZSTD_getcBlockSize (66/74)
// ===========================================================================

#[test]
fn dblk_getc_block_size() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:66",
        "ERR:decompress/zstd_decompress_block.c:74",
    ]);
    // every srcSize below the 3-byte minimum
    for n in 0..3usize {
        let b = vec![0x00u8; n];
        diff(&format!("getcBlockSize len={n}"), |l| {
            let f = l.sym::<FnGetcBlockSize>("ZSTD_getcBlockSize");
            let mut bp = BlockProperties::default();
            (
                res(l, unsafe {
                    f(b.as_ptr() as *const c_void, b.len(), &mut bp)
                }),
                bp,
            )
        });
    }
    // every block type x lastBlock x a spread of sizes
    for btype in [bt_raw, bt_rle, bt_compressed, bt_reserved] {
        for last in [false, true] {
            for size in [0u32, 1, 3, 1024, 131_071, 131_072, 131_073, 0x1F_FFFF] {
                let h = block_header(last, btype, size);
                let b = h.to_vec();
                diff(
                    &format!("getcBlockSize t={btype} last={last} size={size}"),
                    |l| {
                        let f = l.sym::<FnGetcBlockSize>("ZSTD_getcBlockSize");
                        let mut bp = BlockProperties::default();
                        (
                            res(l, unsafe {
                                f(b.as_ptr() as *const c_void, b.len(), &mut bp)
                            }),
                            bp,
                        )
                    },
                );
            }
        }
    }
    // exhaustive over the low byte, which carries lastBlock + blockType
    for lo in 0u16..=255 {
        let b = [lo as u8, 0x00, 0x00];
        diff(&format!("getcBlockSize lo={lo:#04x}"), |l| {
            let f = l.sym::<FnGetcBlockSize>("ZSTD_getcBlockSize");
            let mut bp = BlockProperties::default();
            (res(l, unsafe { f(b.as_ptr() as *const c_void, 3, &mut bp) }), bp)
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress_block.c — literals section
// 139/149/153/185/186/188/191/192/241/266/271/272/273/276/310/315/319/320/321
// ===========================================================================

/// Run a raw block payload through both the exported `ZSTD_decompressBlock` and
/// the exported `ZSTD_decodeLiteralsBlock_wrapper`, with an explicit `dst`.
fn block_and_literals(
    l: &Lib,
    payload: &[u8],
    dst: *mut c_void,
    cap: usize,
) -> (R, R) {
    let d = Ctx::dctx(l);
    let begin = l.sym::<FnSzPtr>("ZSTD_decompressBegin");
    unsafe { begin(d.ptr) };
    let db = l.sym::<FnDecompressDCtx>("ZSTD_decompressBlock");
    let a = res(l, unsafe {
        db(
            d.ptr,
            dst,
            cap,
            payload.as_ptr() as *const c_void,
            payload.len(),
        )
    });
    let d2 = Ctx::dctx(l);
    unsafe { begin(d2.ptr) };
    let dl = l.sym::<FnDecodeLiterals>("ZSTD_decodeLiteralsBlock_wrapper");
    let b = res(l, unsafe {
        dl(
            d2.ptr,
            payload.as_ptr() as *const c_void,
            payload.len(),
            dst,
            cap,
        )
    });
    (a, b)
}

#[test]
fn dblk_literals_section_header_validation() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:139",
        "ERR:decompress/zstd_decompress_block.c:149",
        "ERR:decompress/zstd_decompress_block.c:153",
        "ERR:decompress/zstd_decompress_block.c:185",
        "ERR:decompress/zstd_decompress_block.c:186",
        "ERR:decompress/zstd_decompress_block.c:188",
        "ERR:decompress/zstd_decompress_block.c:191",
        "ERR:decompress/zstd_decompress_block.c:192",
        "ERR:decompress/zstd_decompress_block.c:266",
        "ERR:decompress/zstd_decompress_block.c:271",
        "ERR:decompress/zstd_decompress_block.c:272",
        "ERR:decompress/zstd_decompress_block.c:273",
        "ERR:decompress/zstd_decompress_block.c:276",
        "ERR:decompress/zstd_decompress_block.c:310",
        "ERR:decompress/zstd_decompress_block.c:315",
        "ERR:decompress/zstd_decompress_block.c:319",
        "ERR:decompress/zstd_decompress_block.c:320",
        "ERR:decompress/zstd_decompress_block.c:321",
        "ERR:decompress/zstd_decompress_block.c:2081",
        "ERR:decompress/zstd_decompress_block.c:2086",
        "ERR:decompress/zstd_decompress_block.c:2129",
        "ERR:decompress/zstd_decompress_block.c:2130",
        "ERR:decompress/zstd_decompress_block.c:2197",
    ]);
    // Named crafted payloads. `L` = literals-section header byte:
    //   bits 0-1 Literals_Block_Type, bits 2-3 Size_Format.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // 139: srcSize < MIN_CBLOCK_SIZE (2)
        ("srcSize=0", vec![]),
        ("srcSize=1 basic", vec![0x00]),
        ("srcSize=1 rle", vec![0x01]),
        ("srcSize=1 comp", vec![0x02]),
        ("srcSize=1 repeat", vec![0x03]),
        // 149: set_repeat (3) with dctx->litEntropy == 0 -> dictionary_corrupted
        ("repeat lhl0", vec![0x03, 0, 0, 0, 0]),
        ("repeat lhl1", vec![0x07, 0, 0, 0, 0]),
        ("repeat lhl2", vec![0x0B, 0, 0, 0, 0]),
        ("repeat lhl3", vec![0x0F, 0, 0, 0, 0, 0]),
        // 153: set_compressed / set_repeat and srcSize < 5
        ("comp srcSize=2", vec![0x02, 0x00]),
        ("comp srcSize=3", vec![0x02, 0x00, 0x00]),
        ("comp srcSize=4", vec![0x02, 0x00, 0x00, 0x00]),
        // 185: compressed literals, litSize > 0 && dst == NULL (dst set below)
        ("comp litSize=256", vec![0x02, 0x10, 0x00, 0x00, 0x00]),
        // 186: litSize (18-bit, lhl 3) = 200000 > ZSTD_BLOCKSIZE_MAX
        ("comp litSize=200000", vec![0x0E, 0xD4, 0x30, 0x00, 0x00]),
        // 188: 4-stream (lhl != 0) with litSize < MIN_LITERALS_FOR_4_STREAMS (6)
        ("comp 4stream litSize=0", vec![0x06, 0x00, 0x00, 0x00, 0x00]),
        ("comp 4stream litSize=5", vec![0x06, 0x50, 0x00, 0x00, 0x00]),
        ("comp 4stream litSize=6", vec![0x06, 0x60, 0x00, 0x00, 0x00]),
        // 191: litCSize + lhSize > srcSize
        ("comp litCSize=256", vec![0x02, 0x10, 0x40, 0x00, 0x00]),
        // 266: set_basic with lhl == 3 and srcSize < 3
        ("basic lhl3 srcSize=2", vec![0x0C, 0x00]),
        // 271: set_basic litSize > 0 && dst == NULL
        ("basic litSize=1", vec![0x08, 0xFF, 0x00]),
        // 272: set_basic litSize = 200000 > blockSizeMax
        ("basic litSize=200000", vec![0x0C, 0xD4, 0x30, 0x00, 0x00, 0x00, 0x00]),
        // 273/276: set_basic litSize=10 with 0 / 10 literal bytes present
        ("basic litSize=10 no data", vec![0x50, 0x00]),
        (
            "basic litSize=10 data",
            [&[0x50u8][..], b"0123456789", &[0u8; 8][..]].concat(),
        ),
        // 310: set_rle with lhl == 1 and srcSize < 3
        ("rle lhl1 srcSize=2", vec![0x05, 0x00]),
        // 315: set_rle with lhl == 3 and srcSize < 4
        ("rle lhl3 srcSize=3", vec![0x0D, 0x00, 0x00]),
        // 319: set_rle litSize > 0 && dst == NULL
        ("rle litSize=1", vec![0x09, 0x41]),
        // 320: set_rle litSize = 200000 > blockSizeMax
        ("rle litSize=200000", vec![0x0D, 0xD4, 0x30, 0x41]),
        // 321: set_rle litSize=10 into a tiny dst
        ("rle litSize=10", vec![0x51, 0x41]),
    ];
    let mut big = vec![0xCDu8; 300_000];
    let big_ptr = big.as_mut_ptr() as *mut c_void;
    for (name, payload) in &cases {
        for cap in [0usize, 1, 4, 9, 10, 256, 300_000] {
            diff(&format!("block {name} cap={cap}"), |l| {
                let mut dst = vec![0xCDu8; cap.max(1)];
                let p = dst.as_mut_ptr() as *mut c_void;
                let r = block_and_literals(l, payload, p, cap);
                let ok = matches!(r, (R::Ok(_), R::Ok(_)));
                dst.truncate(if ok { cap.min(64) } else { 0 });
                (r, Blob(dst))
            });
        }
        // dst == NULL with a non-zero capacity: exactly what rows 185/271/319
        // describe ("litSize > 0 && dst == NULL"). This is safe because the C
        // returns before touching `dst`.
        diff(&format!("block {name} dst=NULL cap=1000"), |l| {
            block_and_literals(l, payload, std::ptr::null_mut(), 1000)
        });
        diff(&format!("block {name} dst=NULL cap=0"), |l| {
            block_and_literals(l, payload, std::ptr::null_mut(), 0)
        });
        let _ = big_ptr;
    }
    // 2081: srcSize > ZSTD_blockSizeMax(dctx) == 131072 outside frame mode
    for n in [131_071usize, 131_072, 131_073, 200_000] {
        let payload = vec![0u8; n];
        diff(&format!("block srcSize={n}"), |l| {
            block_and_literals(l, &payload, big_ptr, big.len())
        });
    }
    // 2129: (dst == NULL || dstCapacity == 0) && nbSeq > 0
    // literals `set_basic` with litSize 0, then nbSeq = 1
    for (name, payload) in [
        ("nbSeq=1 dst=NULL cap=0", vec![0x00u8, 0x01, 0x00]),
        ("nbSeq=1 lit0", vec![0x00u8, 0x01, 0x00]),
        ("nbSeq=0", vec![0x00u8, 0x00]),
    ] {
        diff(&format!("block {name} dst=NULL cap=0"), |l| {
            block_and_literals(l, &payload, std::ptr::null_mut(), 0)
        });
        diff(&format!("block {name} dst=ok cap=0"), |l| {
            block_and_literals(l, &payload, big_ptr, 0)
        });
    }
    // 2130: `dst` within 1 MiB of the top of the address space. The C only
    // *compares* the pointer (it never dereferences it before returning
    // dstSize_tooSmall), so this crafted pointer is in contract.
    for off in [1usize, 1024, (1 << 20) - 1, 1 << 20] {
        let p = (usize::MAX - off) as *mut c_void;
        diff(&format!("block dst=SIZE_MAX-{off}"), |l| {
            block_and_literals(l, &[0x00u8, 0x01, 0x00], p, 8)
        });
    }
}

/// `set_repeat` literals with a *previous* table available (loaded from a
/// dictionary, which `ZSTD_loadDEntropy` builds with the X2 reader) — this is
/// the only way a 1-stream literals section reaches the X2 decoder.
#[test]
fn dblk_literals_set_repeat_with_table() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:149",
        "ERR:decompress/zstd_decompress_block.c:241",
        "ERR:decompress/huf_decompress.c:1361",
        "ERR:decompress/huf_decompress.c:1373",
        "ERR:decompress/huf_decompress.c:1389",
        "ERR:decompress/huf_decompress.c:1390",
        "ERR:decompress/huf_decompress.c:1424",
        "ERR:decompress/huf_decompress.c:1427",
        "ERR:decompress/huf_decompress.c:1428",
        "ERR:decompress/huf_decompress.c:1429",
        "ERR:decompress/huf_decompress.c:1430",
    ]);
    let dict = trained_dict();
    // Payloads whose literals header says `set_repeat`, exercising both the
    // 1-stream (lhl 0) and 4-stream (lhl 1/2/3) branches with assorted
    // litSize/litCSize combinations and truncated bitstreams.
    let mut payloads: Vec<(String, Vec<u8>)> = Vec::new();
    for lhl in 0u8..=3 {
        let l0 = 0x03u8 | (lhl << 2);
        for (lit, litc) in [
            (0u32, 0u32),
            (1, 1),
            (5, 1),
            (6, 1),
            (6, 0),
            (16, 4),
            (64, 9),
            (64, 10),
            (100, 40),
            (1000, 100),
        ] {
            let mut v: Vec<u8> = Vec::new();
            match lhl {
                0 | 1 => {
                    let lhc = ((litc & 0x3FF) << 14) | ((lit & 0x3FF) << 4) | (l0 as u32);
                    v.extend_from_slice(&le32(lhc)[..3]);
                }
                2 => {
                    let lhc = (litc << 18) | ((lit & 0x3FFF) << 4) | (l0 as u32);
                    v.extend_from_slice(&le32(lhc));
                }
                _ => {
                    let lhc = ((litc & 0x3FF) << 22) | ((lit & 0x3FFFF) << 4) | (l0 as u32);
                    v.extend_from_slice(&le32(lhc));
                    v.push((litc >> 10) as u8);
                }
            }
            // a deterministic "bitstream" of litCSize bytes, then a sequences
            // section with nbSeq == 0
            for i in 0..litc {
                v.push((i as u8).wrapping_mul(37).wrapping_add(0x5A));
            }
            v.push(0x00);
            payloads.push((format!("repeat lhl={lhl} lit={lit} litc={litc}"), v));
        }
    }
    let mut dst = vec![0xCDu8; 300_000];
    let p = dst.as_mut_ptr() as *mut c_void;
    for (name, payload) in payloads {
        diff(&format!("set_repeat {name}"), |l| {
            let d = Ctx::dctx(l);
            let bd = l.sym::<FnLoadDict>("ZSTD_decompressBegin_usingDict");
            let a = res(l, unsafe {
                bd(d.ptr, dict.as_ptr() as *const c_void, dict.len())
            });
            let db = l.sym::<FnDecompressDCtx>("ZSTD_decompressBlock");
            let b = res(l, unsafe {
                db(
                    d.ptr,
                    p,
                    200_000,
                    payload.as_ptr() as *const c_void,
                    payload.len(),
                )
            });
            (a, b)
        });
    }
}

// ===========================================================================
// decompress/zstd_decompress_block.c — sequences section
// 658/659/671/683/684/705/711/715/723/729/730/745/757/769, 2125/2129
// ===========================================================================

#[test]
fn dblk_sequence_headers() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:658",
        "ERR:decompress/zstd_decompress_block.c:659",
        "ERR:decompress/zstd_decompress_block.c:671",
        "ERR:decompress/zstd_decompress_block.c:683",
        "ERR:decompress/zstd_decompress_block.c:684",
        "ERR:decompress/zstd_decompress_block.c:705",
        "ERR:decompress/zstd_decompress_block.c:711",
        "ERR:decompress/zstd_decompress_block.c:715",
        "ERR:decompress/zstd_decompress_block.c:723",
        "ERR:decompress/zstd_decompress_block.c:729",
        "ERR:decompress/zstd_decompress_block.c:730",
        "ERR:decompress/zstd_decompress_block.c:745",
        "ERR:decompress/zstd_decompress_block.c:757",
        "ERR:decompress/zstd_decompress_block.c:769",
        "ERR:decompress/zstd_decompress_block.c:2125",
        "ERR:decompress/zstd_decompress_block.c:2129",
    ]);
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // 705: srcSize < MIN_SEQUENCES_SIZE (1)
    cases.push(("empty".into(), vec![]));
    // nbSeq encodings: 0, 1..127, 128..0x7EFF, and the 0xFF + LE16 form
    cases.push(("nbSeq=0".into(), vec![0x00]));
    // 723: nbSeq == 0 with trailing bytes
    for k in 1..=3usize {
        cases.push((format!("nbSeq=0 +{k}B"), [vec![0x00u8], vec![0xAAu8; k]].concat()));
    }
    for n in [1u8, 2, 0x7E, 0x7F] {
        cases.push((format!("nbSeq={n} no types"), vec![n]));
        cases.push((format!("nbSeq={n} basic"), vec![n, 0x00]));
    }
    // 715: 0x80 <= nbSeq <= 0xFE with no second byte
    for n in [0x80u8, 0x81, 0xC0, 0xFE] {
        cases.push((format!("nbSeq2byte {n:#04x} trunc"), vec![n]));
        cases.push((format!("nbSeq2byte {n:#04x}"), vec![n, 0x05, 0x00]));
    }
    // 711: nbSeq == 0xFF with fewer than 2 bytes for the LE16 extension
    cases.push(("nbSeq=0xFF trunc0".into(), vec![0xFF]));
    cases.push(("nbSeq=0xFF trunc1".into(), vec![0xFF, 0x00]));
    cases.push(("nbSeq=0xFF ok".into(), vec![0xFF, 0x34, 0x12, 0x00]));
    cases.push(("nbSeq=0xFF max".into(), vec![0xFF, 0xFF, 0xFF, 0x00]));
    // 729: nbSeq > 0 with no symbol-encoding-types byte  (covered above)
    // 730: the 2 reserved low bits of the symbol-types byte are not zero
    for r in 1u8..=3 {
        cases.push((format!("reserved bits={r}"), vec![0x01, r]));
    }
    // 658/659: set_rle (1) per table with no symbol byte / an out-of-range symbol
    for (who, shift) in [("LL", 6u8), ("OF", 4), ("ML", 2)] {
        let types = 1u8 << shift;
        cases.push((format!("{who} set_rle no sym"), vec![0x01, types]));
        for sym in [0u8, 31, 32, 35, 36, 52, 53, 0xFF] {
            cases.push((format!("{who} set_rle sym={sym}"), vec![0x01, types, sym]));
        }
        // 671: set_repeat (3) with no previous table
        let types_rep = 3u8 << shift;
        cases.push((format!("{who} set_repeat"), vec![0x01, types_rep]));
        // 683: set_compressed (2) with a bogus FSE header
        let types_c = 2u8 << shift;
        cases.push((format!("{who} set_compressed bogus"), vec![0x01, types_c, 0x00]));
        cases.push((
            format!("{who} set_compressed trunc"),
            vec![0x01, types_c, 0xFF, 0xFF],
        ));
    }
    // 684: a *valid* FSE distribution whose accuracy log exceeds the per-table
    // maximum (OffFSELog 8, LLFSELog 9, MLFSELog 9).
    cases.push((
        "OF tableLog=9".into(),
        [vec![0x01u8, 2u8 << 4], fse_ncount(9, 31)].concat(),
    ));
    cases.push((
        "LL tableLog=10".into(),
        [vec![0x01u8, 2u8 << 6], fse_ncount(10, 35)].concat(),
    ));
    cases.push((
        "ML tableLog=10".into(),
        [vec![0x01u8, 2u8 << 2], fse_ncount(10, 52)].concat(),
    ));
    // in-range accuracy logs for the same tables, to show they are accepted
    cases.push((
        "OF tableLog=8".into(),
        [vec![0x01u8, 2u8 << 4], fse_ncount(8, 31)].concat(),
    ));
    cases.push((
        "LL tableLog=9".into(),
        [vec![0x01u8, 2u8 << 6], fse_ncount(9, 35)].concat(),
    ));
    // every symbol-types byte with zero reserved bits, nbSeq = 1
    for hi in 0u8..64 {
        cases.push((format!("types={:#04x}", hi << 2), vec![0x01, hi << 2, 0x00, 0x00]));
    }

    for (name, b) in cases {
        diff(&format!("decodeSeqHeaders {name}"), |l| {
            let d = Ctx::dctx(l);
            let begin = l.sym::<FnSzPtr>("ZSTD_decompressBegin");
            unsafe { begin(d.ptr) };
            let f = l.sym::<FnDecodeSeqHeaders>("ZSTD_decodeSeqHeaders");
            let mut nb: c_int = -12345;
            let n = unsafe {
                f(
                    d.ptr,
                    &mut nb,
                    b.as_ptr() as *const c_void,
                    b.len(),
                )
            };
            (res(l, n), nb)
        });
        // and the same bytes as the sequences section of a whole block whose
        // literals section is `set_basic` with litSize 0 (one leading 0x00 byte),
        // so the masking at 745/757/769 is observed through ZSTD_decompressBlock.
        let payload = [vec![0x00u8], b.clone()].concat();
        diff(&format!("block seq {name}"), |l| {
            let mut dst = vec![0xCDu8; 4096];
            let p = dst.as_mut_ptr() as *mut c_void;
            block_and_literals(l, &payload, p, dst.len())
        });
    }
}

// ===========================================================================
// decompress/huf_decompress.c — direct entry points
// 207/213/219/236/238/253/285/292/395/401/588/592/608/609/643/646..649/
// 680..682/693/851/883/886/937/938, 1193..1207, 1361..1496, 1678..1778,
// 1851/1852/1853/1899/1900/1928
// ===========================================================================

/// Build a `HUF_DTable` from a real Huffman tree description, returning
/// `(dtable, hSize)`. `x2` selects the double-symbol reader.
fn read_dtable(l: &Lib, desc: &[u8], x2: bool, max_log: u32) -> (Vec<u32>, R) {
    let mut dt = vec![0u32; 1 + (1usize << ZSTD_HUFFDTABLE_CAPACITY_LOG)];
    dt[0] = max_log * 0x0100_0001;
    let mut w = vec![0u64; 512];
    let name = if x2 {
        "HUF_readDTableX2_wksp"
    } else {
        "HUF_readDTableX1_wksp"
    };
    let f = l.sym::<FnHufReadDTable>(name);
    let n = unsafe {
        f(
            dt.as_mut_ptr(),
            desc.as_ptr() as *const c_void,
            desc.len(),
            w.as_mut_ptr() as *mut c_void,
            HUF_DECOMPRESS_WORKSPACE_SIZE,
            0,
        )
    };
    (dt, res(l, n))
}

#[test]
fn huf_read_dtable_and_direct_decode() {
    covers(&[
        "ERR:decompress/huf_decompress.c:207",
        "ERR:decompress/huf_decompress.c:213",
        "ERR:decompress/huf_decompress.c:219",
        "ERR:decompress/huf_decompress.c:395",
        "ERR:decompress/huf_decompress.c:401",
        "ERR:decompress/huf_decompress.c:588",
        "ERR:decompress/huf_decompress.c:592",
        "ERR:decompress/huf_decompress.c:608",
        "ERR:decompress/huf_decompress.c:609",
        "ERR:decompress/huf_decompress.c:1193",
        "ERR:decompress/huf_decompress.c:1200",
        "ERR:decompress/huf_decompress.c:1204",
        "ERR:decompress/huf_decompress.c:1207",
        "ERR:decompress/huf_decompress.c:1361",
        "ERR:decompress/huf_decompress.c:1373",
        "ERR:decompress/huf_decompress.c:1389",
        "ERR:decompress/huf_decompress.c:1390",
        "ERR:decompress/huf_decompress.c:1762",
        "ERR:decompress/huf_decompress.c:1763",
        "ERR:decompress/huf_decompress.c:1851",
        "ERR:decompress/huf_decompress.c:1852",
        "ERR:decompress/huf_decompress.c:1853",
        "ERR:decompress/huf_decompress.c:1899",
        "ERR:decompress/huf_decompress.c:1900",
        "ERR:decompress/huf_decompress.c:1928",
    ]);
    let (f4, b4) = fx_4stream_x1();
    let li4 = b4.lits.unwrap();
    let huf4 = f4[li4.huf_off..li4.huf_off + li4.lit_c_size].to_vec();
    let (f1, b1) = fx_1stream();
    let li1 = b1.lits.unwrap();
    let huf1 = f1[li1.huf_off..li1.huf_off + li1.lit_c_size].to_vec();
    let dict = trained_dict();
    let ly = dict_layout();
    let dict_huf = dict[ly.huf..ly.off].to_vec();

    // 395/1193: an undersized workspace, and 1200/1207: a small `maxTableLog`
    // in DTable[0]. Both are DIRECT-only (ZSTD always passes a big enough
    // workspace and a capacity log of 12).
    for (dname, desc) in [
        ("frame4", &huf4),
        ("frame1", &huf1),
        ("dict", &dict_huf),
    ] {
        for x2 in [false, true] {
            for wksp in [0usize, 8, 64, 1024, HUF_DECOMPRESS_WORKSPACE_SIZE, 4096] {
                diff(
                    &format!("readDTable {dname} x2={x2} wksp={wksp}"),
                    |l| {
                        let mut dt = huf_dtable();
                        let mut w = vec![0u64; 1024];
                        let name = if x2 {
                            "HUF_readDTableX2_wksp"
                        } else {
                            "HUF_readDTableX1_wksp"
                        };
                        let f = l.sym::<FnHufReadDTable>(name);
                        let n = unsafe {
                            f(
                                dt.as_mut_ptr(),
                                desc.as_ptr() as *const c_void,
                                desc.len(),
                                w.as_mut_ptr() as *mut c_void,
                                wksp,
                                0,
                            )
                        };
                        (res(l, n), dt[0])
                    },
                );
            }
            for max_log in [0u32, 1, 4, 5, 11, 12, 13, 15, 255] {
                diff(
                    &format!("readDTable {dname} x2={x2} maxTableLog={max_log}"),
                    |l| {
                        let (dt, r) = read_dtable(l, desc, x2, max_log);
                        (r, dt[0])
                    },
                );
            }
            // 401/1204: truncated / corrupt weight tables forwarded from HUF_readStats
            for n in 0..desc.len().min(24) {
                diff(&format!("readDTable {dname} x2={x2} desc[..{n}]"), |l| {
                    let (dt, r) = read_dtable(l, &desc[..n], x2, 12);
                    (r, dt[0])
                });
            }
            for byte0 in [0x00u8, 0x01, 0x7F, 0x80, 0x81, 0xC7, 0xFF] {
                let mut b = desc.to_vec();
                b[0] = byte0;
                diff(
                    &format!("readDTable {dname} x2={x2} desc[0]={byte0:#04x}"),
                    |l| {
                        let (dt, r) = read_dtable(l, &b, x2, 12);
                        (r, dt[0])
                    },
                );
            }
        }
    }

    // Direct 1-stream / 4-stream decoding through a *valid* DTable, with every
    // interesting (dstSize, cSrcSize) combination. This is where the
    // BIT_initDStream rejections (588/592/1361/1373), the `cSrcSize < 10` and
    // `dstSize < 6` guards (608/609/1389/1390) and the fast-path fallbacks
    // (207/213/219/253) live.
    let h4 = match read_dtable(&pair().c, &huf4, false, 12).1 {
        R::Ok(n) => n,
        e => panic!("fixture X1 table unreadable: {e:?}"),
    };
    let stream4 = huf4[h4..].to_vec();
    for x2 in [false, true] {
        for flags in [0, HUF_flags_disableFast] {
            for cs in [
                0usize,
                1,
                2,
                5,
                6,
                9,
                10,
                11,
                stream4.len().saturating_sub(1),
                stream4.len(),
            ] {
                for ds in [0usize, 1, 5, 6, 7, 8, 64, li4.lit_size, li4.lit_size + 1] {
                    let cs = cs.min(stream4.len());
                    diff(
                        &format!("HUF_decompress4X_usingDTable x2={x2} f={flags} cs={cs} ds={ds}"),
                        |l| {
                            let (dt, _) = read_dtable(l, &huf4, x2, 12);
                            let g = l.sym::<FnHufUsingDTable>("HUF_decompress4X_usingDTable");
                            let mut out = vec![0xCDu8; ds.max(1)];
                            let p = if ds == 0 {
                                std::ptr::null_mut()
                            } else {
                                out.as_mut_ptr() as *mut c_void
                            };
                            let n = unsafe {
                                g(
                                    p,
                                    ds,
                                    stream4.as_ptr() as *const c_void,
                                    cs,
                                    dt.as_ptr(),
                                    flags,
                                )
                            };
                            out.truncate(ds.min(64));
                            { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
                        },
                    );
                    diff(
                        &format!("HUF_decompress1X_usingDTable x2={x2} f={flags} cs={cs} ds={ds}"),
                        |l| {
                            let (dt, _) = read_dtable(l, &huf4, x2, 12);
                            let g = l.sym::<FnHufUsingDTable>("HUF_decompress1X_usingDTable");
                            let mut out = vec![0xCDu8; ds.max(1)];
                            let p = if ds == 0 {
                                std::ptr::null_mut()
                            } else {
                                out.as_mut_ptr() as *mut c_void
                            };
                            let n = unsafe {
                                g(
                                    p,
                                    ds,
                                    stream4.as_ptr() as *const c_void,
                                    cs,
                                    dt.as_ptr(),
                                    flags,
                                )
                            };
                            out.truncate(ds.min(64));
                            { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
                        },
                    );
                }
            }
        }
    }

    // 1851/1852/1853 (HUF_decompress1X_DCtx_wksp raw-copy / RLE shortcuts),
    // 1899/1900 (X1) 1762/1763 (X2) and 1928 (hufOnly cSrcSize == 0).
    for (wname, worker) in [
        ("1X_DCtx_wksp", "HUF_decompress1X_DCtx_wksp"),
        ("1X1_DCtx_wksp", "HUF_decompress1X1_DCtx_wksp"),
        ("1X2_DCtx_wksp", "HUF_decompress1X2_DCtx_wksp"),
        ("4X_hufOnly_wksp", "HUF_decompress4X_hufOnly_wksp"),
    ] {
        for cs in [0usize, 1, 2, 8, 9, 10, 16, huf4.len()] {
            for ds in [0usize, 1, 5, 6, 8, 16, 64, 4096] {
                diff(&format!("HUF_{wname} cs={cs} ds={ds}"), |l| {
                    let mut dt = huf_dtable();
                    let mut w = vec![0u64; 512];
                    let mut out = vec![0xCDu8; ds.max(1)];
                    let f = l.sym::<FnHufDCtxWksp>(worker);
                    let n = unsafe {
                        f(
                            dt.as_mut_ptr(),
                            out.as_mut_ptr() as *mut c_void,
                            ds,
                            huf4.as_ptr() as *const c_void,
                            cs,
                            w.as_mut_ptr() as *mut c_void,
                            HUF_DECOMPRESS_WORKSPACE_SIZE,
                            0,
                        )
                    };
                    out.truncate(ds.min(64));
                    { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
                });
            }
        }
        // 1762/1899: a corrupt weight table forwarded out of the reader
        for n in 0..8usize {
            diff(&format!("HUF_{wname} desc[..{n}]"), |l| {
                let mut dt = huf_dtable();
                let mut w = vec![0u64; 512];
                let mut out = vec![0xCDu8; 4096];
                let f = l.sym::<FnHufDCtxWksp>(worker);
                let r = unsafe {
                    f(
                        dt.as_mut_ptr(),
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        huf4.as_ptr() as *const c_void,
                        n,
                        w.as_mut_ptr() as *mut c_void,
                        HUF_DECOMPRESS_WORKSPACE_SIZE,
                        0,
                    )
                };
                res(l, r)
            });
        }
    }
    diff("HUF_selectDecoder grid", |l| {
        let f = l.sym::<FnSelectDecoder>("HUF_selectDecoder");
        let mut out = Vec::new();
        for ds in [1usize, 6, 255, 256, 768, 1280, 3072, 65535, 131_072] {
            for cs in [1usize, 8, 100, 1000, 10_000, 131_072] {
                out.push(unsafe { f(ds, cs) });
            }
        }
        out
    });
}

/// The 6-byte jump table that precedes the four Huffman bitstreams:
/// rows 236 / 238 / 253 / 501 / 643 / 646..649 / 851 / 1424 / 1427..1430.
#[test]
fn huf_jump_table_variants() {
    covers(&[
        "ERR:decompress/huf_decompress.c:236",
        "ERR:decompress/huf_decompress.c:238",
        "ERR:decompress/huf_decompress.c:253",
        "ERR:decompress/huf_decompress.c:643",
        "ERR:decompress/huf_decompress.c:646",
        "ERR:decompress/huf_decompress.c:647",
        "ERR:decompress/huf_decompress.c:648",
        "ERR:decompress/huf_decompress.c:649",
        "ERR:decompress/huf_decompress.c:851",
        "ERR:decompress/huf_decompress.c:1424",
        "ERR:decompress/huf_decompress.c:1427",
        "ERR:decompress/huf_decompress.c:1428",
        "ERR:decompress/huf_decompress.c:1429",
        "ERR:decompress/huf_decompress.c:1430",
        "ERR:decompress/huf_decompress.c:1678",
        "ERR:decompress/zstd_decompress_block.c:241",
    ]);
    for (fxname, fx) in [("X1", fx_4stream_x1()), ("X2", fx_4stream_x2())] {
        let (frame, blk) = fx;
        let li = blk.lits.unwrap();
        let huf = frame[li.huf_off..li.huf_off + li.lit_c_size].to_vec();
        let hs = match read_dtable(&pair().c, &huf, false, 12).1 {
            R::Ok(n) => n,
            e => panic!("{fxname} fixture table unreadable: {e:?}"),
        };
        let payload = huf[hs..].to_vec();
        assert!(payload.len() > 6, "{fxname} fixture has no jump table");
        // patch each of the three LE16 jump-table entries
        let lengths: [u32; 3] = [
            0,
            1,
            2, // (placeholder, overwritten below)
        ];
        let _ = lengths;
        let mut variants: Vec<(String, Vec<u8>)> = Vec::new();
        for idx in 0..3usize {
            for v in [
                0u16,
                1,
                7,
                8,
                9,
                (payload.len() as u16).saturating_sub(6),
                0xFFF0,
                0xFFFF,
            ] {
                let mut p = payload.clone();
                p[idx * 2..idx * 2 + 2].copy_from_slice(&v.to_le_bytes());
                variants.push((format!("len{}={v}", idx + 1), p));
            }
        }
        // all three huge at once -> the size_t subtraction wraps (238 / 643 / 1424)
        {
            let mut p = payload.clone();
            for idx in 0..3usize {
                p[idx * 2..idx * 2 + 2].copy_from_slice(&0xFFF0u16.to_le_bytes());
            }
            variants.push(("all=0xFFF0".into(), p));
        }
        // lengths summing to exactly payload.len() - 6 -> length4 == 0 (649/1430)
        {
            let mut p = payload.clone();
            let rest = payload.len() - 6;
            let each = (rest / 3) as u16;
            p[0..2].copy_from_slice(&each.to_le_bytes());
            p[2..4].copy_from_slice(&each.to_le_bytes());
            p[4..6].copy_from_slice(&((rest as u16) - 2 * each).to_le_bytes());
            variants.push(("length4=0".into(), p));
        }
        variants.push(("intact".into(), payload.clone()));

        for (name, p) in variants {
            for x2 in [false, true] {
                for flags in [0, HUF_flags_disableFast] {
                    for ds in [6usize, 7, 8, 64, li.lit_size] {
                        diff(
                            &format!("{fxname} jump {name} x2={x2} f={flags} ds={ds}"),
                            |l| {
                                let (dt, _) = read_dtable(l, &huf, x2, 12);
                                let g = l.sym::<FnHufUsingDTable>("HUF_decompress4X_usingDTable");
                                let mut out = vec![0xCDu8; ds.max(1)];
                                let n = unsafe {
                                    g(
                                        out.as_mut_ptr() as *mut c_void,
                                        ds,
                                        p.as_ptr() as *const c_void,
                                        p.len(),
                                        dt.as_ptr(),
                                        flags,
                                    )
                                };
                                out.truncate(ds.min(64));
                                { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
                            },
                        );
                    }
                }
            }
            // the same patch applied inside the frame: the HUF error must be
            // MASKED to corruption_detected (20) at zstd_decompress_block.c:241
            let mut f2 = frame.clone();
            f2[li.huf_off + hs..li.huf_off + hs + p.len()].copy_from_slice(&p);
            diff(&format!("{fxname} frame jump {name}"), |l| {
                dec_full(l, &f2, 300_000)
            });
            diff(&format!("{fxname} frame jump {name} stream"), |l| {
                stream_all(l, &f2, 300_000, 4096)
            });
        }
    }
}

/// Exhaustively corrupt the Huffman bitstream of a 4-stream literals section,
/// one byte / one bit at a time, and require identical results. This is what
/// drives the "stream ran past its segment" and "bitstream not fully consumed"
/// checks: 285/292/503/504/518..521/524/680..682/693/883/886 (X1) and
/// 1483..1496/1708/1711 (X2). Individual sites cannot be selected from the
/// outside — every one of them returns `corruption_detected` (20) — so the sweep
/// is the honest way to exercise them.
#[test]
fn huf_bitstream_corruption_sweep() {
    covers(&[
        "ERR:decompress/huf_decompress.c:285",
        "ERR:decompress/huf_decompress.c:292",
        "ERR:decompress/huf_decompress.c:680",
        "ERR:decompress/huf_decompress.c:681",
        "ERR:decompress/huf_decompress.c:682",
        "ERR:decompress/huf_decompress.c:693",
        "ERR:decompress/huf_decompress.c:883",
        "ERR:decompress/huf_decompress.c:886",
        "ERR:decompress/huf_decompress.c:937",
        "ERR:decompress/huf_decompress.c:938",
        "ERR:decompress/huf_decompress.c:1483",
        "ERR:decompress/huf_decompress.c:1484",
        "ERR:decompress/huf_decompress.c:1485",
        "ERR:decompress/huf_decompress.c:1496",
        "ERR:decompress/huf_decompress.c:1708",
        "ERR:decompress/huf_decompress.c:1711",
        "ERR:decompress/huf_decompress.c:1777",
        "ERR:decompress/huf_decompress.c:1778",
        "ERR:decompress/zstd_decompress_block.c:241",
    ]);
    for (fxname, fx) in [
        ("X1", fx_4stream_x1()),
        ("X2", fx_4stream_x2()),
        ("1stream", fx_1stream()),
    ] {
        let (frame, blk) = fx;
        let li = blk.lits.unwrap();
        let huf = frame[li.huf_off..li.huf_off + li.lit_c_size].to_vec();
        let hs = match read_dtable(&pair().c, &huf, false, 12).1 {
            R::Ok(n) => n,
            e => panic!("{fxname} fixture table unreadable: {e:?}"),
        };
        // Sample the payload rather than every byte, to stay inside the runtime
        // budget while still covering the jump table, both ends of each stream
        // and the interior.
        let payload_len = huf.len() - hs;
        let mut offsets: Vec<usize> = Vec::new();
        offsets.extend(0..payload_len.min(20));
        for q in 1..=8usize {
            let o = payload_len * q / 9;
            offsets.push(o);
            offsets.push(o.saturating_sub(1));
        }
        for k in 1..=8usize {
            offsets.push(payload_len.saturating_sub(k));
        }
        offsets.retain(|&o| o < payload_len);
        offsets.sort_unstable();
        offsets.dedup();
        for &off in &offsets {
            for mode in 0..3u8 {
                let mut f2 = frame.clone();
                let i = li.huf_off + hs + off;
                match mode {
                    0 => f2[i] = 0x00,
                    1 => f2[i] ^= 0x01,
                    _ => f2[i] ^= 0x80,
                }
                diff(&format!("{fxname} lit corrupt off={off} m={mode}"), |l| {
                    dec_full(l, &f2, 300_000)
                });
                // and directly through HUF, where the *unmasked* code is visible
                let mut p = huf[hs..].to_vec();
                match mode {
                    0 => p[off] = 0x00,
                    1 => p[off] ^= 0x01,
                    _ => p[off] ^= 0x80,
                }
                for x2 in [false, true] {
                    for flags in [0, HUF_flags_disableFast] {
                        diff(
                            &format!("{fxname} HUF corrupt off={off} m={mode} x2={x2} f={flags}"),
                            |l| {
                                let (dt, _) = read_dtable(l, &huf, x2, 12);
                                let g = if li.single {
                                    l.sym::<FnHufUsingDTable>("HUF_decompress1X_usingDTable")
                                } else {
                                    l.sym::<FnHufUsingDTable>("HUF_decompress4X_usingDTable")
                                };
                                let mut out = vec![0xCDu8; li.lit_size.max(1)];
                                let n = unsafe {
                                    g(
                                        out.as_mut_ptr() as *mut c_void,
                                        li.lit_size,
                                        p.as_ptr() as *const c_void,
                                        p.len(),
                                        dt.as_ptr(),
                                        flags,
                                    )
                                };
                                out.truncate(li.lit_size.min(64));
                                { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
                            },
                        );
                    }
                }
            }
        }
        // 937/938/1777/1778: `hSize >= cSrcSize` — the tree description fills the
        // whole literals section, leaving no bitstream.
        for extra in [0usize, 1, 2] {
            let mut f2 = frame.clone();
            let new_csize = hs + extra;
            patch_lit_c_size(&mut f2, *blk, &li,new_csize as u32);
            diff(&format!("{fxname} litCSize=hSize+{extra}"), |l| {
                dec_full(l, &f2, 300_000)
            });
            diff(&format!("{fxname} litCSize=hSize+{extra} hufOnly"), |l| {
                let mut dt = huf_dtable();
                let mut w = vec![0u64; 512];
                let mut out = vec![0xCDu8; li.lit_size.max(1)];
                let f = l.sym::<FnHufDCtxWksp>("HUF_decompress4X_hufOnly_wksp");
                let n = unsafe {
                    f(
                        dt.as_mut_ptr(),
                        out.as_mut_ptr() as *mut c_void,
                        li.lit_size,
                        huf.as_ptr() as *const c_void,
                        new_csize,
                        w.as_mut_ptr() as *mut c_void,
                        HUF_DECOMPRESS_WORKSPACE_SIZE,
                        0,
                    )
                };
                res(l, n)
            });
        }
        // litCSize == 0 (1928 via HUF_decompress4X_hufOnly_wksp)
        {
            let mut f2 = frame.clone();
            patch_lit_c_size(&mut f2, *blk, &li,0);
            diff(&format!("{fxname} litCSize=0"), |l| dec_full(l, &f2, 300_000));
        }
    }
}

/// Rewrite the `Compressed_Size` field of a `set_compressed` literals header
/// in place, keeping the block header and the rest of the frame untouched.
fn patch_lit_c_size(f: &mut [u8], blk: Blk, li: &Lits, new: u32) {
    let p = blk.payload;
    match li.lh_size {
        3 => {
            let mut lhc = u32::from_le_bytes([f[p], f[p + 1], f[p + 2], 0]);
            lhc &= !(0x3FFu32 << 14);
            lhc |= (new & 0x3FF) << 14;
            f[p..p + 3].copy_from_slice(&le32(lhc)[..3]);
        }
        4 => {
            let mut lhc = u32::from_le_bytes([f[p], f[p + 1], f[p + 2], f[p + 3]]);
            lhc &= (1u32 << 18) - 1;
            lhc |= new << 18;
            f[p..p + 4].copy_from_slice(&le32(lhc));
        }
        _ => {
            let mut lhc = u32::from_le_bytes([f[p], f[p + 1], f[p + 2], f[p + 3]]);
            lhc &= (1u32 << 22) - 1;
            lhc |= (new & 0x3FF) << 22;
            f[p..p + 4].copy_from_slice(&le32(lhc));
            f[p + 4] = (new >> 10) as u8;
        }
    }
}

// ===========================================================================
// decompress/zstd_decompress_block.c — sequence execution
// 919/920/932 (ZSTD_execSequenceEnd), 967/968/973/981 (…SplitLitBuffer),
// 1054/1147 (ZSTD_execSequence[SplitLitBuffer]), 1308 (repcode -> (size_t)-1),
// 1425/1521/1579/1581/1591/1603 (…bodySplitLitBuffer),
// 1637/1674/1682 (…body), 1765/1788/1824/1833/1871/1880 (…Long_body)
// ===========================================================================

/// A frame decompressed with an increasingly tight `dstCapacity`. Every
/// `dstSize_tooSmall` (70) site inside the sequence executors is on this path;
/// which one fires depends on where in the block the output runs out.
#[test]
fn dblk_sequence_execution_dst_too_small() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:919",
        "ERR:decompress/zstd_decompress_block.c:967",
        "ERR:decompress/zstd_decompress_block.c:973",
        "ERR:decompress/zstd_decompress_block.c:1521",
        "ERR:decompress/zstd_decompress_block.c:1591",
        "ERR:decompress/zstd_decompress_block.c:1603",
        "ERR:decompress/zstd_decompress_block.c:1682",
        "ERR:decompress/zstd_decompress_block.c:2086",
        "ERR:decompress/zstd_decompress_block.c:2125",
    ]);
    let mut fixtures: Vec<(String, Vec<u8>, usize)> = Vec::new();
    for &kind in &[Corpus::Text, Corpus::SmallAlphabet, Corpus::Mixed] {
        for &n in &[5_000usize, 70_000, 200_000] {
            for lvl in [1, 3, 9] {
                let src = corpus(kind, n, 0xC0FFEE);
                let f = c_compress(&src, lvl);
                fixtures.push((format!("{kind:?}/{n}/l{lvl}"), f, src.len()));
            }
        }
    }
    // a split-literals frame (litSize > 65536) so the ...SplitLitBuffer paths run
    {
        let (f, blk) = fx_split_lits();
        let li = blk.lits.unwrap();
        let full = match dec_len(&pair().c, f) {
            Some(n) => n,
            None => panic!("split-literals fixture does not decode"),
        };
        fixtures.push((format!("split(litSize={})", li.lit_size), f.clone(), full));
    }
    for (name, frame, full) in &fixtures {
        let mut caps: Vec<usize> = vec![0, 1, 2, 16, 100];
        for q in 1..=8usize {
            caps.push(full * q / 9);
        }
        for k in 1..=4usize {
            caps.push(full.saturating_sub(k));
        }
        caps.push(*full);
        caps.push(full + 1);
        caps.sort_unstable();
        caps.dedup();
        for cap in caps {
            diff(&format!("decompress {name} cap={cap}"), |l| {
                dec_full(l, frame, cap)
            });
            // the same through the streaming API with a stable output buffer, so
            // the direct-to-dst (non-buffered) executors are used
            diff(&format!("stableOut {name} cap={cap}"), |l| {
                let ds = Ctx::dstream(l);
                let sp = set_dparam(l, ds.ptr, ZSTD_d_stableOutBuffer, 1);
                let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
                let mut out = vec![0xCDu8; cap.max(1)];
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void,
                    size: cap,
                    pos: 0,
                };
                let mut ib = ZSTD_inBuffer {
                    src: frame.as_ptr() as *const c_void,
                    size: frame.len(),
                    pos: 0,
                };
                let n = unsafe { f(ds.ptr, &mut ob, &mut ib) };
                out.truncate(cap.min(256));
                { out.truncate(ob.pos); (sp, res(l, n), ib.pos, ob.pos, Blob(out)) }
            });
            // and with a *cold* DDict, which selects ZSTD_decompressSequencesLong
            diff(&format!("coldDDict {name} cap={cap}"), |l| {
                let d = Ctx::dctx(l);
                let mk = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                let raw = corpus(Corpus::Text, 4096, 7);
                let dd = unsafe { mk(raw.as_ptr() as *const c_void, raw.len()) };
                let g = l.sym::<FnDecompUsingDDict>("ZSTD_decompress_usingDDict");
                let mut out = vec![0xCDu8; cap.max(1)];
                let p = if cap == 0 {
                    std::ptr::null_mut()
                } else {
                    out.as_mut_ptr() as *mut c_void
                };
                let n = unsafe {
                    g(
                        d.ptr,
                        p,
                        cap,
                        frame.as_ptr() as *const c_void,
                        frame.len(),
                        dd,
                    )
                };
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                unsafe { fr(dd) };
                out.truncate(cap.min(256));
                { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
            });
        }
    }
}

fn dec_len(l: &Lib, frame: &[u8]) -> Option<usize> {
    let f = l.sym::<FnU64Buf>("ZSTD_getFrameContentSize");
    let n = unsafe { f(frame.as_ptr() as *const c_void, frame.len()) };
    if n >= ZSTD_CONTENTSIZE_ERROR {
        None
    } else {
        Some(n as usize)
    }
}

/// Structural corruption of the *sequences* section: destroy the end mark, move
/// `nbSeq`, and decompress individual blocks out of context so their offsets
/// point before the start of history. Drives 920/932/968/981/1054/1147/1308,
/// 1425/1579/1581/1637/1674 and 1765/1824.
#[test]
fn dblk_sequence_bitstream_and_offset_corruption() {
    covers(&[
        "ERR:decompress/zstd_decompress_block.c:920",
        "ERR:decompress/zstd_decompress_block.c:932",
        "ERR:decompress/zstd_decompress_block.c:968",
        "ERR:decompress/zstd_decompress_block.c:981",
        "ERR:decompress/zstd_decompress_block.c:1054",
        "ERR:decompress/zstd_decompress_block.c:1147",
        "ERR:decompress/zstd_decompress_block.c:1308",
        "ERR:decompress/zstd_decompress_block.c:1425",
        "ERR:decompress/zstd_decompress_block.c:1579",
        "ERR:decompress/zstd_decompress_block.c:1581",
        "ERR:decompress/zstd_decompress_block.c:1637",
        "ERR:decompress/zstd_decompress_block.c:1674",
        "ERR:decompress/zstd_decompress_block.c:1765",
        "ERR:decompress/zstd_decompress_block.c:1788",
        "ERR:decompress/zstd_decompress_block.c:1824",
        "ERR:decompress/zstd_decompress_block.c:1833",
        "ERR:decompress/zstd_decompress_block.c:1871",
        "ERR:decompress/zstd_decompress_block.c:1880",
    ]);
    let mut fixtures: Vec<(String, Vec<u8>, usize)> = Vec::new();
    for &kind in &[Corpus::Text, Corpus::SmallAlphabet, Corpus::LongRepeats] {
        for &n in &[5_000usize, 70_000, 200_000] {
            let src = corpus(kind, n, 0xBEEF);
            for lvl in [1, 3, 9] {
                let f = c_compress(&src, lvl);
                fixtures.push((format!("{kind:?}/{n}/l{lvl}"), f, src.len()));
            }
        }
    }
    {
        let (f, _) = fx_split_lits();
        let full = dec_len(&pair().c, f).unwrap();
        fixtures.push(("split".into(), f.clone(), full));
    }
    for (name, frame, full) in &fixtures {
        let hs = frame_header_size(frame);
        let blk = parse_block(frame, hs);
        if blk.btype != bt_compressed {
            continue;
        }
        let body_end = blk.payload + blk.c_size;
        // 1637/1425/1765: zero the last byte of the block (the sequences
        // bitstream end mark) -> BIT_initDStream reports "no end mark".
        let mut mutations: Vec<(String, Vec<u8>)> = Vec::new();
        {
            let mut f2 = frame.clone();
            f2[body_end - 1] = 0x00;
            mutations.push(("seq last byte = 0".into(), f2));
        }
        for k in 1..=3usize {
            let mut f2 = frame.clone();
            f2[body_end - k] ^= 0xFF;
            mutations.push((format!("seq byte -{k} flipped"), f2));
        }
        // 1674/1581/1824: nbSeq under/over-declared. `nbSeq` is the first byte of
        // the sequences section, i.e. right after the literals section.
        if let Some(li) = blk.lits {
            let seq_off = blk.payload + li.lh_size + li.lit_c_size;
            if seq_off < body_end {
                for delta in [-1i32, 1, 2, -2] {
                    let mut f2 = frame.clone();
                    let v = f2[seq_off] as i32 + delta;
                    if (0..=0x7F).contains(&v) {
                        f2[seq_off] = v as u8;
                        mutations.push((format!("nbSeq{delta:+}"), f2));
                    }
                }
                // 1054/932: force large offsets by flipping high bits mid-bitstream
                for q in [1usize, 2, 3] {
                    let mut f2 = frame.clone();
                    let i = seq_off + (body_end - seq_off) * q / 4;
                    if i < body_end {
                        f2[i] ^= 0xF0;
                        mutations.push((format!("seq mid q{q} ^0xF0"), f2));
                    }
                }
            }
        }
        for (mname, f2) in mutations {
            for cap in [0usize, 16, full / 2, *full, full + 64] {
                diff_bytes(&format!("{name} {mname} cap={cap}"), |l| {
                    dec_full(l, &f2, cap)
                });
            }
            diff(&format!("{name} {mname} coldDDict"), |l| {
                let d = Ctx::dctx(l);
                let mk = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                let raw = corpus(Corpus::Text, 4096, 7);
                let dd = unsafe { mk(raw.as_ptr() as *const c_void, raw.len()) };
                let g = l.sym::<FnDecompUsingDDict>("ZSTD_decompress_usingDDict");
                let mut out = vec![0xCDu8; full + 64];
                let n = unsafe {
                    g(
                        d.ptr,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        f2.as_ptr() as *const c_void,
                        f2.len(),
                        dd,
                    )
                };
                let fr = l.sym::<FnSzPtr>("ZSTD_freeDDict");
                unsafe { fr(dd) };
                out.truncate(256);
                { let st = res(l, n); let bb = blob_if_ok(&st, out); (st, bb) }
            });
            diff(&format!("{name} {mname} stream"), |l| {
                stream_all(l, &f2, full + 64, 4096)
            });
        }
        // 1054/1147/1308: decompress a *later* block standalone, with no history,
        // so every offset it carries points before `virtualStart`.
        let mut off = hs;
        let mut idx = 0usize;
        loop {
            let b = parse_block(frame, off);
            if idx > 0 && b.btype == bt_compressed {
                let payload = frame[b.payload..b.payload + b.c_size].to_vec();
                for cap in [0usize, 16, 131_072] {
                    diff(&format!("{name} standalone block#{idx} cap={cap}"), |l| {
                        let mut dst = vec![0xCDu8; cap.max(1)];
                        let p = if cap == 0 {
                            std::ptr::null_mut()
                        } else {
                            dst.as_mut_ptr() as *mut c_void
                        };
                        let r = block_and_literals(l, &payload, p, cap);
                        let ok = matches!(r, (R::Ok(_), R::Ok(_)));
                        dst.truncate(if ok { cap.min(256) } else { 0 });
                        (r, Blob(dst))
                    });
                }
                break;
            }
            if b.last {
                break;
            }
            off = b.payload + b.c_size;
            idx += 1;
            if off + 3 > frame.len() || idx > 8 {
                break;
            }
        }
    }
}

// ===========================================================================
// Randomized corruption fuzz (fixed seed)
// ===========================================================================

/// Take valid frames built over several corpora / levels / frame parameters and
/// mutate them four ways — truncate, flip a bit, flip a byte, splice in random
/// bytes — then require the C and the Rust to agree on the result (and, where it
/// is defined, on the produced bytes) for both `ZSTD_decompress` and
/// `ZSTD_decompressStream`.
///
/// IMPORTANT: bytes 0..4 (the frame magic) are never touched. This build has
/// `ZSTD_LEGACY_SUPPORT=5`, so a mutated magic can route the buffer into the
/// v0.5/v0.6/v0.7 legacy decoders, which are NOT hardened against arbitrary
/// input and segfault in the reference C. There is no C behaviour to match for
/// such inputs, so they are out of scope. For the same reason every mutated
/// buffer is additionally screened for an accidental legacy magic anywhere
/// inside it (a spliced-in run of random bytes could create one), and such
/// candidates are re-rolled.
#[test]
fn corruption_fuzz_fixed_seed() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:991",
        "ERR:decompress/zstd_decompress.c:995",
        "ERR:decompress/zstd_decompress.c:1031",
        "ERR:decompress/zstd_decompress.c:1046",
        "ERR:decompress/zstd_decompress.c:1050",
        "ERR:decompress/zstd_decompress.c:1055",
        "ERR:decompress/zstd_decompress.c:1157",
        "ERR:decompress/zstd_decompress.c:1366",
        "ERR:decompress/zstd_decompress.c:1367",
        "ERR:decompress/zstd_decompress.c:2065",
        "ERR:decompress/zstd_decompress.c:2076",
        "ERR:decompress/zstd_decompress.c:2193",
        "ERR:decompress/zstd_decompress.c:2283",
        "ERR:decompress/zstd_decompress.c:2317",
        "ERR:decompress/zstd_decompress_block.c:2086",
        "ERR:decompress/zstd_decompress_block.c:2125",
        "ERR:decompress/zstd_decompress_block.c:920",
        "ERR:decompress/zstd_decompress_block.c:932",
        "ERR:decompress/zstd_decompress_block.c:1054",
        "ERR:decompress/zstd_decompress_block.c:1308",
        "ERR:decompress/zstd_decompress_block.c:1637",
        "ERR:decompress/zstd_decompress_block.c:1674",
        "ERR:decompress/zstd_decompress_block.c:241",
        "ERR:decompress/huf_decompress.c:401",
        "ERR:decompress/huf_decompress.c:588",
        "ERR:decompress/huf_decompress.c:592",
        "ERR:decompress/huf_decompress.c:643",
        "ERR:decompress/huf_decompress.c:693",
        "ERR:decompress/huf_decompress.c:937",
        "ERR:decompress/huf_decompress.c:938",
    ]);
    // Frame fixtures: several shapes x levels x frame-parameter combinations.
    let mut frames: Vec<(String, Vec<u8>, usize)> = Vec::new();
    for &kind in &[
        Corpus::Zeros,
        Corpus::Random,
        Corpus::SmallAlphabet,
        Corpus::Text,
        Corpus::LongRepeats,
        Corpus::Mixed,
        Corpus::Sparse,
    ] {
        for &n in &[300usize, 5_000, 70_000, 200_000] {
            let src = corpus(kind, n, 0xFACE_FEED);
            for &(lvl, cks, cs) in &[(1i32, 0i32, 1i32), (3, 1, 1), (9, 0, 0), (19, 1, 0)] {
                let f = c_compress_params(
                    &src,
                    &[
                        (ZSTD_c_compressionLevel, lvl),
                        (ZSTD_c_checksumFlag, cks),
                        (ZSTD_c_contentSizeFlag, cs),
                    ],
                );
                frames.push((format!("{kind:?}/{n}/l{lvl}/c{cks}/s{cs}"), f, n));
            }
        }
    }
    // a small-window frame so blockSizeMax clamping participates
    for &wl in &[10i32, 12, 17] {
        let src = corpus(Corpus::Text, 70_000, 0xFACE_FEED);
        let f = c_compress_params(
            &src,
            &[
                (ZSTD_c_compressionLevel, 3),
                (ZSTD_c_windowLog, wl),
                (ZSTD_c_checksumFlag, 1),
            ],
        );
        frames.push((format!("windowLog{wl}"), f, src.len()));
    }

    let mut rng = Rng::new(0x30_C0FFEE_5EED);
    let mut done = 0usize;
    let target = 3200usize;
    let mut attempts = 0usize;
    while done < target && attempts < target * 8 {
        attempts += 1;
        let (fname, frame, orig_len) = &frames[rng.below(frames.len())];
        let mode = rng.below(4);
        let mut b = frame.clone();
        let label;
        match mode {
            // (a) truncate at a random offset (never below the 4 magic bytes)
            0 => {
                let cut = 4 + rng.below(b.len() - 3);
                b.truncate(cut);
                label = format!("trunc@{cut}");
            }
            // (b) flip a single random bit
            1 => {
                let i = 4 + rng.below(b.len() - 4);
                let bit = rng.below(8);
                b[i] ^= 1 << bit;
                label = format!("bit@{i}.{bit}");
            }
            // (c) replace a random byte with a random value
            2 => {
                let i = 4 + rng.below(b.len() - 4);
                let v = rng.u8();
                b[i] = v;
                label = format!("byte@{i}={v:#04x}");
            }
            // (d) splice a run of random bytes over the frame (length preserved)
            _ => {
                let i = 4 + rng.below(b.len() - 4);
                let n = 1 + rng.below(24.min(b.len() - i));
                let patch = rng.bytes(n);
                b[i..i + n].copy_from_slice(&patch);
                label = format!("splice@{i}+{n}");
            }
        }
        if contains_legacy_magic(&b[1..]) {
            continue; // see the note above: legacy decoders are not hardened
        }
        done += 1;
        let tag = format!("{fname} {label}");
        // exact output capacity, one byte short, and generous
        for cap in [*orig_len, orig_len.saturating_sub(1), orig_len + 64] {
            diff_bytes(&format!("fuzz decompress {tag} cap={cap}"), |l| {
                dec_full(l, &b, cap)
            });
        }
        diff_bytes(&format!("fuzz stream {tag}"), |l| {
            stream_all(l, &b, orig_len + 64, 1 + (b.len() / 3))
        });
        // and a tight streaming output buffer, which forces the buffered path
        diff_bytes(&format!("fuzz stream tight {tag}"), |l| {
            stream_all(l, &b, 4096, 1024)
        });
    }
    assert!(done >= 3000, "only {done} fuzz cases ran");
    eprintln!("corruption fuzz: {done} cases ({attempts} attempts)");
}
