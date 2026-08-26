//! Phase C — error-path differential tests for `lz4frame.c`.
//!
//! One `#[test]` (or one clearly-labelled block inside one) per row of the
//! `## lz4frame.c` section of `ERRORS.md` (**rows 168-221**). Every case builds
//! the exact invalid input / state described by the row, calls BOTH the C `.so`
//! and the Rust `.so`, and asserts they return the SAME value — for the LZ4F
//! API that means the identical `LZ4F_getErrorCode()` number (e.g.
//! `err::ERROR_dstMaxSize_tooSmall` == 11), never merely "both failed".
//!
//! Harness rules observed throughout:
//!   * functions are only ever obtained via `common::both::<T>("symbol")`;
//!   * both destination buffers are pre-filled with the SAME `0xAA` sentinel and
//!     compared in FULL after every call;
//!   * `LZ4F_uncompressedUpdate` is only driven with `LZ4F_blockIndependent`
//!     (lz4frame.h:707 — the `blockLinked` path violates
//!     `assert(blockCompression == LZ4B_COMPRESSED)` at lz4frame.c:1071 and
//!     corrupts the heap inside the C itself);
//!   * paths that can legitimately write past `dstCapacity` (lz4frame.c:1006-1016)
//!     get a large slack region past the declared capacity.
//!
//! The comment block at the END of this file lists every row 168-221 with its
//! covering test, or the precise reason it cannot be tested.

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_void};
use std::ptr;

/// Fill byte for BOTH destination buffers, so untouched bytes compare equal.
const SENTINEL: u8 = 0xAA;

// ---------------------------------------------------------------------------
// Signatures — verified against c_src/src/lz4frame.c and c_src/include/lz4frame.h
// ---------------------------------------------------------------------------

type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnFrameBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;

/// `size_t LZ4F_compressFrame(void* dst, size_t dstCapacity, const void* src, size_t srcSize, const LZ4F_preferences_t*)`
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
/// `size_t LZ4F_compressFrame_usingCDict(cctx, dst, dstCapacity, src, srcSize, cdict, prefs)`
type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;

type FnCreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateCctxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnFreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;

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
type FnBeginInternal = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;

/// `LZ4F_compressUpdate` / `LZ4F_uncompressedUpdate`
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
/// `LZ4F_flush` / `LZ4F_compressEnd`
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;

type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateCDictAdv = unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);

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

// helpers only (used to build valid inputs that are then corrupted)
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;
type FnLZ4CompressDefault = unsafe extern "C" fn(*const c_void, *mut c_void, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

fn describe(code: usize) -> String {
    if lz4f_is_error(code) {
        format!("ERROR({})", lz4f_error_code(code))
    } else {
        format!("{}", code)
    }
}

#[track_caller]
fn same_ret(label: &str, c: usize, r: usize) {
    if c != r {
        panic!(
            "{}: return mismatch\n  C   = {} (raw 0x{:x})\n  Rust= {} (raw 0x{:x})",
            label,
            describe(c),
            c,
            describe(r),
            r
        );
    }
}

/// Assert C and Rust returned the identical value AND that it is exactly the
/// expected `LZ4F_errorCodes` number.
#[track_caller]
fn expect_err(label: &str, c: usize, r: usize, want: i32) {
    same_ret(label, c, r);
    assert!(
        lz4f_is_error(c),
        "{}: expected LZ4F error {} but the call SUCCEEDED with {}",
        label,
        want,
        c
    );
    assert_eq!(
        lz4f_error_code(c),
        want,
        "{}: wrong error code (raw 0x{:x})",
        label,
        c
    );
}

#[track_caller]
fn expect_ok(label: &str, c: usize, r: usize) {
    same_ret(label, c, r);
    assert!(
        !lz4f_is_error(c),
        "{}: expected success, got ERROR({})",
        label,
        lz4f_error_code(c)
    );
}

/// `XXH32(data, 0)` computed by BOTH libraries and cross-checked, so the frames
/// this file builds are not silently keyed to one implementation.
fn xxh32(data: &[u8]) -> u32 {
    let (c, r) = both::<FnXXH32>("LZ4_XXH32");
    unsafe {
        let a = c(data.as_ptr() as *const c_void, data.len(), 0);
        let b = r(data.as_ptr() as *const c_void, data.len(), 0);
        assert_eq!(a, b, "test helper: LZ4_XXH32 disagreement");
        a
    }
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

// ---------------------------------------------------------------------------
// Hand-built LZ4 frames
//
// Building frames byte-by-byte (instead of via LZ4F_compressFrame) is what makes
// rows 197-202 and 213-219 addressable: every header bit, every block header and
// every checksum can be set to exactly the invalid value the row names.
// ---------------------------------------------------------------------------

/// Magic + FLG + BD + optional contentSize + optional dictID + header checksum.
/// `flg`/`bd` are written verbatim so invalid bit patterns can be injected; the
/// header checksum is always made CORRECT so the only defect is the intended one.
fn raw_header(flg: u8, bd: u8, csize: Option<u64>, dictid: Option<u32>) -> Vec<u8> {
    let mut inner: Vec<u8> = vec![flg, bd];
    if let Some(cs) = csize {
        inner.extend_from_slice(&cs.to_le_bytes());
    }
    if let Some(di) = dictid {
        inner.extend_from_slice(&le32(di));
    }
    let hc = (xxh32(&inner) >> 8) as u8;
    let mut out = Vec::with_capacity(4 + inner.len() + 1);
    out.extend_from_slice(&le32(LZ4F_MAGICNUMBER));
    out.extend_from_slice(&inner);
    out.push(hc);
    out
}

#[derive(Clone, Copy, Debug)]
struct Hdr {
    bsid: c_int,
    independent: bool,
    content_ck: bool,
    block_ck: bool,
    content_size: Option<u64>,
    dict_id: Option<u32>,
}

impl Hdr {
    fn new(bsid: c_int) -> Hdr {
        Hdr {
            bsid,
            independent: true,
            content_ck: false,
            block_ck: false,
            content_size: None,
            dict_id: None,
        }
    }
    fn flg(&self) -> u8 {
        (1u8 << 6)
            | ((self.independent as u8) << 5)
            | ((self.block_ck as u8) << 4)
            | ((self.content_size.is_some() as u8) << 3)
            | ((self.content_ck as u8) << 2)
            | (self.dict_id.is_some() as u8)
    }
    fn bd(&self) -> u8 {
        ((self.bsid as u8) & 0x07) << 4
    }
    fn bytes(&self) -> Vec<u8> {
        raw_header(self.flg(), self.bd(), self.content_size, self.dict_id)
    }
}

fn block_size_of(bsid: c_int) -> usize {
    match bsid {
        0 | 4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        other => panic!("block_size_of: invalid blockSizeID {}", other),
    }
}

/// LZ4-block-compress `data` with the C library (a pure helper).
fn lz4_block(data: &[u8]) -> Vec<u8> {
    let (c, _) = both::<FnLZ4CompressDefault>("LZ4_compress_default");
    let cap = data.len() + data.len() / 2 + 64;
    let mut dst = vec![0u8; cap];
    let n = unsafe {
        c(
            data.as_ptr() as *const c_void,
            dst.as_mut_ptr() as *mut c_void,
            data.len() as c_int,
            cap as c_int,
        )
    };
    assert!(n > 0, "test helper: LZ4_compress_default failed");
    dst.truncate(n as usize);
    dst
}

/// A compressed frame block: `LE32(cSize)` + payload + optional block checksum.
///
/// Mirrors `LZ4F_makeBlock` (lz4frame.c:886-905): when compression does not
/// shrink the input, the block is STORED with `LZ4F_BLOCKUNCOMPRESSED_FLAG`.
fn compressed_block(data: &[u8], block_ck: bool) -> Vec<u8> {
    let comp = lz4_block(data);
    if comp.is_empty() || comp.len() >= data.len() {
        return uncompressed_block(data, block_ck);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&le32(comp.len() as u32));
    out.extend_from_slice(&comp);
    if block_ck {
        out.extend_from_slice(&le32(xxh32(&comp)));
    }
    out
}

/// An uncompressed frame block: `LE32(0x80000000|size)` + payload + optional ck.
fn uncompressed_block(data: &[u8], block_ck: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&le32(0x8000_0000u32 | data.len() as u32));
    out.extend_from_slice(data);
    if block_ck {
        out.extend_from_slice(&le32(xxh32(data)));
    }
    out
}

/// endMark + optional content checksum over `content`.
fn frame_tail(content: &[u8], content_ck: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&le32(0));
    if content_ck {
        out.extend_from_slice(&le32(xxh32(content)));
    }
    out
}

/// A complete single-block frame (compressed block).
fn frame_1block(h: &Hdr, data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= block_size_of(h.bsid));
    let mut f = h.bytes();
    f.extend_from_slice(&compressed_block(data, h.block_ck));
    f.extend_from_slice(&frame_tail(data, h.content_ck));
    f
}

// ---------------------------------------------------------------------------
// Lock-step decompression driver
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DecCfg {
    /// Destination capacity offered per call.
    dst_cap: usize,
    /// Source bytes offered per call; 0 = the whole remaining input.
    src_chunk: usize,
    opts: Option<LZ4F_decompressOptions_t>,
    /// When `Some`, `LZ4F_decompress_usingDict` is used instead.
    dict: Option<Vec<u8>>,
}

impl DecCfg {
    fn new(dst_cap: usize) -> DecCfg {
        DecCfg { dst_cap, src_chunk: 0, opts: None, dict: None }
    }
}

struct DecOut {
    /// Return value of the LAST `LZ4F_decompress` call.
    ret: usize,
    consumed: usize,
    produced: Vec<u8>,
    /// The FULL destination buffer of every call (sentinel-filled beforehand).
    raw: Vec<Vec<u8>>,
    calls: usize,
    /// `LZ4F_freeDecompressionContext` return value (row 221 relevance).
    free_ret: usize,
}

fn dec_drive(is_c: bool, frame: &[u8], cfg: &DecCfg) -> DecOut {
    let l = libs();
    let lib = if is_c { &l.c } else { &l.rust };
    let create: FnCreateDctx = lib.sym("LZ4F_createDecompressionContext");
    let free: FnFreeDctx = lib.sym("LZ4F_freeDecompressionContext");
    let dec: FnDecompress = lib.sym("LZ4F_decompress");
    let decd: FnDecompressUsingDict = lib.sym("LZ4F_decompress_usingDict");

    let mut out = DecOut {
        ret: 0,
        consumed: 0,
        produced: Vec::new(),
        raw: Vec::new(),
        calls: 0,
        free_ret: 0,
    };
    unsafe {
        let mut dctx: *mut c_void = ptr::null_mut();
        let rc = create(&mut dctx, LZ4F_VERSION);
        assert!(!lz4f_is_error(rc), "createDecompressionContext failed");
        let mut sp = 0usize;
        loop {
            out.calls += 1;
            assert!(out.calls <= 8192, "dec_drive: runaway loop");
            let mut dst = vec![SENTINEL; cfg.dst_cap.max(1)];
            let mut dsz = cfg.dst_cap;
            let avail = frame.len() - sp;
            let mut ssz = if cfg.src_chunk == 0 {
                avail
            } else {
                cfg.src_chunk.min(avail)
            };
            let optp = match &cfg.opts {
                Some(o) => o as *const LZ4F_decompressOptions_t,
                None => ptr::null(),
            };
            let ret = match &cfg.dict {
                None => dec(
                    dctx,
                    dst.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    frame.as_ptr().add(sp) as *const c_void,
                    &mut ssz,
                    optp,
                ),
                Some(d) => decd(
                    dctx,
                    dst.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    frame.as_ptr().add(sp) as *const c_void,
                    &mut ssz,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    optp,
                ),
            };
            out.raw.push(dst.clone());
            out.ret = ret;
            if lz4f_is_error(ret) {
                break;
            }
            out.produced.extend_from_slice(&dst[..dsz]);
            out.consumed += ssz;
            sp += ssz;
            if ret == 0 {
                break; // frame fully decoded
            }
            if ssz == 0 && dsz == 0 {
                break; // no progress possible with the input we have
            }
        }
        out.free_ret = free(dctx);
    }
    out
}

/// Run `dec_drive` on both libraries and require complete agreement.
#[track_caller]
fn dec_both(label: &str, frame: &[u8], cfg: &DecCfg) -> usize {
    let c = dec_drive(true, frame, cfg);
    let r = dec_drive(false, frame, cfg);
    same_ret(&format!("{}: LZ4F_decompress return", label), c.ret, r.ret);
    assert_eq!(c.calls, r.calls, "{}: number of decompress calls", label);
    assert_eq!(c.consumed, r.consumed, "{}: src bytes consumed", label);
    assert_bytes_eq(&format!("{}: regenerated data", label), &c.produced, &r.produced);
    assert_eq!(c.raw.len(), r.raw.len(), "{}: dst buffer count", label);
    for i in 0..c.raw.len() {
        assert_bytes_eq(
            &format!("{}: FULL dst buffer of call #{}", label, i + 1),
            &c.raw[i],
            &r.raw[i],
        );
    }
    same_ret(
        &format!("{}: LZ4F_freeDecompressionContext", label),
        c.free_ret,
        r.free_ret,
    );
    c.ret
}

#[track_caller]
fn dec_expect_err(label: &str, frame: &[u8], cfg: &DecCfg, want: i32) {
    let ret = dec_both(label, frame, cfg);
    assert!(
        lz4f_is_error(ret),
        "{}: expected LZ4F error {} but decoding succeeded ({})",
        label,
        want,
        ret
    );
    assert_eq!(
        lz4f_error_code(ret),
        want,
        "{}: wrong error code (raw 0x{:x})",
        label,
        ret
    );
}

// ---------------------------------------------------------------------------
// Allocation-failure injection (rows 172/173/178/179/180/194/211/212)
//
// `customAlloc` / `customCalloc` count their invocations and return NULL on the
// Nth one; `customFree` decrements a live counter so every test can prove it
// neither leaked nor double-freed. Each allocation carries a 16-byte header
// holding its total size, so `customFree` can reconstruct the exact Layout.
// ---------------------------------------------------------------------------

#[repr(C)]
struct AllocState {
    calls: u64,
    fail_at: u64,
    live: i64,
}

impl AllocState {
    fn new(fail_at: u64) -> AllocState {
        AllocState { calls: 0, fail_at, live: 0 }
    }
}

const AHDR: usize = 16;

fn alloc_raw(opaque: *mut c_void, size: usize, zero: bool) -> *mut c_void {
    unsafe {
        let st = &mut *(opaque as *mut AllocState);
        st.calls += 1;
        if st.fail_at != 0 && st.calls == st.fail_at {
            return ptr::null_mut();
        }
        let total = size + AHDR;
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        let p = if zero {
            std::alloc::alloc_zeroed(layout)
        } else {
            std::alloc::alloc(layout)
        };
        if p.is_null() {
            return ptr::null_mut();
        }
        *(p as *mut usize) = total;
        st.live += 1;
        p.add(AHDR) as *mut c_void
    }
}

extern "C" fn test_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    alloc_raw(opaque, size, false)
}

extern "C" fn test_calloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    alloc_raw(opaque, size, true)
}

extern "C" fn test_free(opaque: *mut c_void, address: *mut c_void) {
    if address.is_null() {
        return;
    }
    unsafe {
        let st = &mut *(opaque as *mut AllocState);
        let base = (address as *mut u8).sub(AHDR);
        let total = *(base as *mut usize);
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        std::alloc::dealloc(base, layout);
        st.live -= 1;
    }
}

fn cmem_for(st: &mut AllocState, with_calloc: bool) -> LZ4F_CustomMem {
    LZ4F_CustomMem {
        customAlloc: Some(test_alloc),
        customCalloc: if with_calloc { Some(test_calloc) } else { None },
        customFree: Some(test_free),
        opaqueState: st as *mut AllocState as *mut c_void,
    }
}

#[track_caller]
fn assert_no_leak(label: &str, c: &AllocState, r: &AllocState) {
    assert_eq!(
        c.calls, r.calls,
        "{}: allocation call COUNT differs (C={} Rust={})",
        label, c.calls, r.calls
    );
    assert_eq!(
        (c.live, r.live),
        (0, 0),
        "{}: custom-allocator leak (C live={} Rust live={})",
        label,
        c.live,
        r.live
    );
}

// ===========================================================================
// Guard: the harness's error decoding matches BOTH libraries' own exports
//
// Every assertion in this file is phrased in terms of `lz4f_error_code()`, which
// is a Rust re-implementation of `LZ4F_getErrorCode()`. This test proves the
// re-implementation agrees with the exported `LZ4F_isError` / `LZ4F_getErrorCode`
// of BOTH libraries for the whole `LZ4F_errorCodes` range plus the values around
// its boundaries, so "identical error code" really does mean identical.
// ===========================================================================

type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;

#[test]
fn error_code_decoding_matches_both_libraries() {
    let (cie, rie) = both::<FnIsError>("LZ4F_isError");
    let (cgc, rgc) = both::<FnGetErrorCode>("LZ4F_getErrorCode");

    let mut probes: Vec<usize> = Vec::new();
    for code in 0..=(err::ERROR_maxCode + 4) {
        probes.push(0usize.wrapping_sub(code as usize));
    }
    probes.extend_from_slice(&[0, 1, 2, 19, 4096, usize::MAX / 2, usize::MAX]);

    for &v in &probes {
        unsafe {
            let ci = cie(v);
            let ri = rie(v);
            assert_eq!(ci, ri, "LZ4F_isError(0x{:x}): C={} Rust={}", v, ci, ri);
            assert_eq!(
                ci != 0,
                lz4f_is_error(v),
                "harness lz4f_is_error disagrees with the libraries for 0x{:x}",
                v
            );
            let cc = cgc(v);
            let rc = rgc(v);
            assert_eq!(cc, rc, "LZ4F_getErrorCode(0x{:x}): C={} Rust={}", v, cc, rc);
            if ci != 0 {
                assert_eq!(
                    cc,
                    lz4f_error_code(v),
                    "harness lz4f_error_code disagrees for 0x{:x}",
                    v
                );
            }
        }
    }
}

// ===========================================================================
// Rows 168-169 — LZ4F_getBlockSize rejects every id outside 4..7
// ===========================================================================

/// ERRORS.md rows 168 (`blockSizeID` in 1..3) and 169 (`blockSizeID` >= 8).
///
/// `LZ4F_blockSizeID_t` has only non-negative enumerators, so GCC's compatible
/// type is `unsigned int`; negative ints therefore arrive as huge unsigned
/// values. Both signs are pushed across the FFI here.
/// (Also exercised by `get_block_size_every_id_and_random` in
/// tests/lz4frame_oneshot_diff.rs and `get_block_size_all_ids` in
/// tests/lz4frame_decompress_diff.rs; this test pins the error *code*.)
#[test]
fn row_168_169_get_block_size_invalid_id() {
    let (c, r) = both::<FnGetBlockSize>("LZ4F_getBlockSize");

    // row 168: 1..3 are below LZ4F_max64KB(4) after the 0 -> 4 remap.
    for id in 1..4i32 {
        let (cv, rv) = unsafe { (c(id), r(id)) };
        expect_err(
            &format!("row 168: LZ4F_getBlockSize({})", id),
            cv,
            rv,
            err::ERROR_maxBlockSize_invalid,
        );
    }

    // row 169: anything above LZ4F_max4MB(7), plus the negative encodings.
    let mut bad: Vec<c_int> = vec![
        8, 9, 10, 15, 16, 63, 64, 127, 255, 256, 65535, 65536, 1 << 20, c_int::MAX,
    ];
    bad.extend_from_slice(&[-1, -2, -3, -4, -7, -8, -100, c_int::MIN]);
    for &id in &bad {
        let (cv, rv) = unsafe { (c(id), r(id)) };
        expect_err(
            &format!("row 169: LZ4F_getBlockSize({})", id),
            cv,
            rv,
            err::ERROR_maxBlockSize_invalid,
        );
    }

    // Accept boundary, for contrast: 0 (remapped) and 4..7 must succeed.
    for &(id, want) in &[
        (0i32, 64 * 1024usize),
        (4, 64 * 1024),
        (5, 256 * 1024),
        (6, 1024 * 1024),
        (7, 4 * 1024 * 1024),
    ] {
        let (cv, rv) = unsafe { (c(id), r(id)) };
        expect_ok(&format!("LZ4F_getBlockSize({})", id), cv, rv);
        assert_eq!(cv, want, "LZ4F_getBlockSize({}) value", id);
    }
}

// ===========================================================================
// Row 170 — LZ4F_compressFrame{,_usingCDict}: dstCapacity < frame bound
// ===========================================================================

/// ERRORS.md row 170: `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)`
/// where `prefs` has already been auto-corrected (lz4frame.c:456) =>
/// `dstMaxSize_tooSmall`. The smallest reproduction named by the row is
/// `srcSize == 0, dstCapacity == 0` (the corrected bound is 19+4+4 = 27).
#[test]
fn row_170_compress_frame_dst_too_small() {
    let (cf, rf) = both::<FnCompressFrame>("LZ4F_compressFrame");
    let (ccd, rcd) = both::<FnCompressFrameCDict>("LZ4F_compressFrame_usingCDict");
    let (cfb, rfb) = both::<FnFrameBound>("LZ4F_compressFrameBound");
    let (cnew, rnew) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cdel, rdel) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (ccdict, rcdict) = both::<FnCreateCDict>("LZ4F_createCDict");
    let (cfcd, rfcd) = both::<FnFreeCDict>("LZ4F_freeCDict");

    let mut rng = Rng::new(0x0170_0000_0000_0001);
    let dict = gen_shape(&mut rng, 3, 4096);

    // A spread of preferences so the auto-correction (autoFlush=1, optimal
    // blockSizeID, contentSize=srcSize) is exercised in several shapes.
    let mut prefs_set: Vec<LZ4F_preferences_t> = Vec::new();
    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max4MB] {
        for &(cck, bck) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            for &cs in &[0u64, 1] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = bsid;
                p.frameInfo.blockMode = LZ4F_blockLinked;
                p.frameInfo.contentChecksumFlag = cck;
                p.frameInfo.blockChecksumFlag = bck;
                p.frameInfo.contentSize = cs; // non-zero => auto-corrected to srcSize
                p.compressionLevel = 1;
                prefs_set.push(p);
            }
        }
    }

    unsafe {
        let mut cctx_c: *mut c_void = ptr::null_mut();
        let mut cctx_r: *mut c_void = ptr::null_mut();
        expect_ok(
            "row 170: createCompressionContext",
            cnew(&mut cctx_c, LZ4F_VERSION),
            rnew(&mut cctx_r, LZ4F_VERSION),
        );
        let cdict_c = ccdict(dict.as_ptr() as *const c_void, dict.len());
        let cdict_r = rcdict(dict.as_ptr() as *const c_void, dict.len());
        assert!(!cdict_c.is_null() && !cdict_r.is_null(), "row 170: createCDict");

        for (pi, p) in prefs_set.iter().enumerate() {
            for &src_len in &[0usize, 1, 17, 1000, 70_000] {
                let src = gen_shape(&mut rng, pi % N_SHAPES, src_len);
                let bound = cfb(src_len, p as *const _);
                assert_eq!(bound, rfb(src_len, p as *const _), "compressFrameBound");
                assert!(bound > 0);

                // Every capacity strictly below the bound must be rejected. Sweep
                // the interesting neighbourhood plus the exact boundary.
                let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 18, 19, 26, 27];
                for d in 1..=4usize {
                    if bound >= d {
                        caps.push(bound - d);
                    }
                }
                caps.push(bound);
                caps.retain(|&c| c <= bound);
                caps.sort_unstable();
                caps.dedup();

                for &cap in &caps {
                    // dst is always allocated at the FULL bound so a rejected call
                    // that (wrongly) wrote something would be visible.
                    let mut cd = vec![SENTINEL; bound + 64];
                    let mut rd = vec![SENTINEL; bound + 64];
                    let a = cf(
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src_len,
                        p as *const _,
                    );
                    let b = rf(
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src_len,
                        p as *const _,
                    );
                    let label = format!(
                        "row 170: LZ4F_compressFrame prefs#{} srcSize={} cap={} (bound={})",
                        pi, src_len, cap, bound
                    );
                    if cap < bound {
                        expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                        assert_bytes_eq(&format!("{}: dst untouched", label), &cd, &rd);
                        assert!(
                            cd.iter().all(|&x| x == SENTINEL),
                            "{}: C wrote into dst despite rejecting",
                            label
                        );
                    } else {
                        expect_ok(&label, a, b);
                    }
                    assert_bytes_eq(&label, &cd, &rd);

                    // Same branch through LZ4F_compressFrame_usingCDict.
                    let mut cd = vec![SENTINEL; bound + 64];
                    let mut rd = vec![SENTINEL; bound + 64];
                    let a = ccd(
                        cctx_c,
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src_len,
                        cdict_c as *const c_void,
                        p as *const _,
                    );
                    let b = rcd(
                        cctx_r,
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        src_len,
                        cdict_r as *const c_void,
                        p as *const _,
                    );
                    let label = format!(
                        "row 170: LZ4F_compressFrame_usingCDict prefs#{} srcSize={} cap={} (bound={})",
                        pi, src_len, cap, bound
                    );
                    if cap < bound {
                        expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                    } else {
                        expect_ok(&label, a, b);
                    }
                    assert_bytes_eq(&label, &cd, &rd);
                }
            }
        }

        // NULL prefs: the bound is then computed from the worst-case defaults.
        for &src_len in &[0usize, 1, 100] {
            let src = vec![0x5Au8; src_len];
            let bound = cfb(src_len, ptr::null());
            assert_eq!(bound, rfb(src_len, ptr::null()));
            for &cap in &[0usize, 1, bound - 1, bound] {
                let mut cd = vec![SENTINEL; bound + 64];
                let mut rd = vec![SENTINEL; bound + 64];
                let a = cf(
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src_len,
                    ptr::null(),
                );
                let b = rf(
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    src_len,
                    ptr::null(),
                );
                let label = format!(
                    "row 170: NULL prefs srcSize={} cap={} (bound={})",
                    src_len, cap, bound
                );
                if cap < bound {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                } else {
                    expect_ok(&label, a, b);
                }
                assert_bytes_eq(&label, &cd, &rd);
            }
        }

        cfcd(cdict_c);
        rfcd(cdict_r);
        same_ret("row 170: free cctx", cdel(cctx_c), rdel(cctx_r));
    }
}

// ---------------------------------------------------------------------------
// A pair of compression contexts (one per library), freed on drop.
// ---------------------------------------------------------------------------

struct CctxPair {
    c: *mut c_void,
    r: *mut c_void,
}

impl CctxPair {
    fn new(label: &str) -> CctxPair {
        let (cn, rn) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
        let mut c: *mut c_void = ptr::null_mut();
        let mut r: *mut c_void = ptr::null_mut();
        unsafe {
            expect_ok(
                &format!("{}: LZ4F_createCompressionContext", label),
                cn(&mut c, LZ4F_VERSION),
                rn(&mut r, LZ4F_VERSION),
            );
        }
        assert!(!c.is_null() && !r.is_null(), "{}: NULL cctx", label);
        CctxPair { c, r }
    }
}

impl Drop for CctxPair {
    fn drop(&mut self) {
        let (cf, rf) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
        unsafe {
            let a = cf(self.c);
            let b = rf(self.r);
            assert_eq!(a, b, "LZ4F_freeCompressionContext return mismatch");
        }
    }
}

// ===========================================================================
// Row 171 — every LZ4F_compressBegin* entry rejects dstCapacity < 19
// ===========================================================================

/// ERRORS.md row 171: `dstCapacity < maxFHSize` (== `LZ4F_HEADER_SIZE_MAX` == 19)
/// at lz4frame.c:700 => `dstMaxSize_tooSmall`, for `LZ4F_compressBegin`,
/// `_usingDict`, `_usingDictOnce`, `_usingCDict` and `_internal`.
/// (Partially covered by `frame_stream_error_begin_capacity` in
/// tests/lz4frame_stream_diff.rs; this test sweeps every capacity 0..=19 through
/// all five entry points and pins the code.)
#[test]
fn row_171_compress_begin_dst_too_small() {
    let (cb, rb) = both::<FnBegin>("LZ4F_compressBegin");
    let (cbd, rbd) = both::<FnBeginDict>("LZ4F_compressBegin_usingDict");
    let (cbo, rbo) = both::<FnBeginDict>("LZ4F_compressBegin_usingDictOnce");
    let (cbc, rbc) = both::<FnBeginCDict>("LZ4F_compressBegin_usingCDict");
    let (cbi, rbi) = both::<FnBeginInternal>("LZ4F_compressBegin_internal");
    let (ccd, rcd) = both::<FnCreateCDict>("LZ4F_createCDict");
    let (cfd, rfd) = both::<FnFreeCDict>("LZ4F_freeCDict");

    let mut rng = Rng::new(0x0171_0000_0000_0001);
    let dict = gen_shape(&mut rng, 2, 8192);

    // Preferences that make the real header 7, 15 and 19 bytes long: the check is
    // against the MAXIMUM header size (19) regardless of the actual size.
    let mut prefs_set: Vec<Option<LZ4F_preferences_t>> = vec![None];
    for &(cs, di) in &[(0u64, 0u32), (1234, 0), (0, 0xDEAD_BEEF), (1234, 0xDEAD_BEEF)] {
        for &lvl in &[1i32, 9] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = LZ4F_max64KB;
            p.frameInfo.contentSize = cs;
            p.frameInfo.dictID = di;
            p.compressionLevel = lvl;
            prefs_set.push(Some(p));
        }
    }

    unsafe {
        let cdict_c = ccd(dict.as_ptr() as *const c_void, dict.len());
        let cdict_r = rcd(dict.as_ptr() as *const c_void, dict.len());
        assert!(!cdict_c.is_null() && !cdict_r.is_null(), "row 171: createCDict");

        for (pi, p) in prefs_set.iter().enumerate() {
            let pp = match p {
                Some(x) => x as *const LZ4F_preferences_t,
                None => ptr::null(),
            };
            for cap in 0..=LZ4F_HEADER_SIZE_MAX {
                let s = CctxPair::new("row 171");
                // Always allocate 19+slack so an unexpected write is visible.
                let mut cd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let mut rd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let label = format!("row 171: LZ4F_compressBegin prefs#{} cap={}", pi, cap);

                let a = cb(s.c, cd.as_mut_ptr() as *mut c_void, cap, pp);
                let b = rb(s.r, rd.as_mut_ptr() as *mut c_void, cap, pp);
                if cap < LZ4F_HEADER_SIZE_MAX {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                    assert!(
                        cd.iter().all(|&x| x == SENTINEL),
                        "{}: C wrote a header despite rejecting",
                        label
                    );
                } else {
                    expect_ok(&label, a, b);
                }
                assert_bytes_eq(&label, &cd, &rd);

                // _usingDict / _usingDictOnce
                for (name, cfn, rfn) in [
                    ("LZ4F_compressBegin_usingDict", cbd, rbd),
                    ("LZ4F_compressBegin_usingDictOnce", cbo, rbo),
                ] {
                    let mut cd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                    let mut rd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                    let label = format!("row 171: {} prefs#{} cap={}", name, pi, cap);
                    let a = cfn(
                        s.c,
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        pp,
                    );
                    let b = rfn(
                        s.r,
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        pp,
                    );
                    if cap < LZ4F_HEADER_SIZE_MAX {
                        expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                    } else {
                        expect_ok(&label, a, b);
                    }
                    assert_bytes_eq(&label, &cd, &rd);
                }

                // _usingCDict
                let mut cd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let mut rd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let label =
                    format!("row 171: LZ4F_compressBegin_usingCDict prefs#{} cap={}", pi, cap);
                let a = cbc(
                    s.c,
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    cdict_c as *const c_void,
                    pp,
                );
                let b = rbc(
                    s.r,
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    cdict_r as *const c_void,
                    pp,
                );
                if cap < LZ4F_HEADER_SIZE_MAX {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                } else {
                    expect_ok(&label, a, b);
                }
                assert_bytes_eq(&label, &cd, &rd);

                // _internal, with neither dict nor cdict
                let mut cd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let mut rd = vec![SENTINEL; LZ4F_HEADER_SIZE_MAX + 32];
                let label =
                    format!("row 171: LZ4F_compressBegin_internal prefs#{} cap={}", pi, cap);
                let a = cbi(
                    s.c,
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                    0,
                    ptr::null(),
                    pp,
                );
                let b = rbi(
                    s.r,
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    ptr::null(),
                    0,
                    ptr::null(),
                    pp,
                );
                if cap < LZ4F_HEADER_SIZE_MAX {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                } else {
                    expect_ok(&label, a, b);
                }
                assert_bytes_eq(&label, &cd, &rd);
            }
        }

        cfd(cdict_c);
        rfd(cdict_r);
    }
}

// ===========================================================================
// Rows 172-173 — allocation failures inside LZ4F_compressBegin_internal
// ===========================================================================

/// ERRORS.md row 172 (`cctx->lz4CtxPtr == NULL` after `LZ4F_malloc`,
/// lz4frame.c:714-722) and row 173 (`cctx->tmpBuff == NULL` after
/// `LZ4F_malloc(requiredBuffSize)`, lz4frame.c:749-750) => `allocation_failed`.
///
/// Forced with `LZ4F_createCompressionContext_advanced` and an
/// `LZ4F_CustomMem` whose allocator returns NULL on the Nth call. Allocation
/// order is: #1 the `LZ4F_cctx` itself, #2 `lz4CtxPtr`, #3 `tmpBuff`.
/// (Also reached by `frame_stream_allocation_failures` in
/// tests/lz4frame_stream_diff.rs.)
#[test]
fn row_172_173_compress_begin_allocation_failures() {
    let (cca, rca) = both::<FnCreateCctxAdv>("LZ4F_createCompressionContext_advanced");
    let (cfr, rfr) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cb, rb) = both::<FnBegin>("LZ4F_compressBegin");
    let (cbd, rbd) = both::<FnBeginDict>("LZ4F_compressBegin_usingDict");
    let (cbc, rbc) = both::<FnBeginCDict>("LZ4F_compressBegin_usingCDict");
    let (ccd, rcd) = both::<FnCreateCDict>("LZ4F_createCDict");
    let (cfd, rfd) = both::<FnFreeCDict>("LZ4F_freeCDict");

    let dict = vec![0x6Cu8; 4096];

    unsafe {
        let cdict_c = ccd(dict.as_ptr() as *const c_void, dict.len());
        let cdict_r = rcd(dict.as_ptr() as *const c_void, dict.len());
        assert!(!cdict_c.is_null() && !cdict_r.is_null());

        for with_calloc in [false, true] {
            // fail_at 2 => row 172 (lz4CtxPtr), fail_at 3 => row 173 (tmpBuff).
            for fail_at in [2u64, 3] {
                // `requiredBuffSize` is 0 for autoFlush=1 + blockIndependent, and
                // malloc(0) does NOT return NULL, so allocation #3 only happens for
                // the shapes that actually need a tmp buffer.
                for &(af, mode, lvl) in &[
                    (0u32, LZ4F_blockLinked, 1i32),
                    (0, LZ4F_blockIndependent, 1),
                    (1, LZ4F_blockLinked, 1),
                    (0, LZ4F_blockLinked, 9),
                ] {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockSizeID = LZ4F_max64KB;
                    p.frameInfo.blockMode = mode;
                    p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
                    p.compressionLevel = lvl;
                    p.autoFlush = af;

                    for entry in 0..3 {
                        let mut cst = AllocState::new(fail_at);
                        let mut rst = AllocState::new(fail_at);
                        let mut cd = vec![SENTINEL; 64];
                        let mut rd = vec![SENTINEL; 64];
                        let label = format!(
                            "rows 172/173: fail_at={} calloc={} af={} mode={} lvl={} entry={}",
                            fail_at, with_calloc, af, mode, lvl, entry
                        );

                        let c = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                        let r = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                        assert!(!c.is_null() && !r.is_null(), "{}: cctx alloc", label);

                        let (a, b) = match entry {
                            0 => (
                                cb(c, cd.as_mut_ptr() as *mut c_void, 64, &p as *const _),
                                rb(r, rd.as_mut_ptr() as *mut c_void, 64, &p as *const _),
                            ),
                            1 => (
                                cbd(
                                    c,
                                    cd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dict.len(),
                                    &p as *const _,
                                ),
                                rbd(
                                    r,
                                    rd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dict.len(),
                                    &p as *const _,
                                ),
                            ),
                            _ => (
                                cbc(
                                    c,
                                    cd.as_mut_ptr() as *mut c_void,
                                    64,
                                    cdict_c as *const c_void,
                                    &p as *const _,
                                ),
                                rbc(
                                    r,
                                    rd.as_mut_ptr() as *mut c_void,
                                    64,
                                    cdict_r as *const c_void,
                                    &p as *const _,
                                ),
                            ),
                        };

                        // Whether allocation #3 is reached at all depends on the
                        // preferences; when it is not, the call succeeds. Either way
                        // C and Rust must agree exactly.
                        same_ret(&label, a, b);
                        if cst.calls >= fail_at {
                            expect_err(&label, a, b, err::ERROR_allocation_failed);
                            assert!(
                                cd.iter().all(|&x| x == SENTINEL),
                                "{}: C wrote a header despite failing",
                                label
                            );
                        } else {
                            expect_ok(&format!("{} (site not reached)", label), a, b);
                        }
                        assert_bytes_eq(&label, &cd, &rd);
                        same_ret(&format!("{}: free", label), cfr(c), rfr(r));
                        assert_no_leak(&label, &cst, &rst);
                    }
                }
            }
        }
        cfd(cdict_c);
        rfd(cdict_r);
    }
}

// ===========================================================================
// Row 174 — LZ4F_compressBegin_usingDict[Once] with dictSize > INT_MAX
// ===========================================================================

/// ERRORS.md row 174: non-NULL `dictBuffer` with `dictSize > INT_MAX`
/// (lz4frame.c:766-768) => `parameter_invalid`.
///
/// The size is rejected BEFORE the dictionary is read, so a lying (huge) size on
/// a small real buffer is safe.
/// (Also covered by `frame_stream_error_dict_size_too_large` in
/// tests/lz4frame_stream_diff.rs.)
#[test]
fn row_174_compress_begin_dict_size_too_large() {
    let (cbd, rbd) = both::<FnBeginDict>("LZ4F_compressBegin_usingDict");
    let (cbo, rbo) = both::<FnBeginDict>("LZ4F_compressBegin_usingDictOnce");
    let (cbi, rbi) = both::<FnBeginInternal>("LZ4F_compressBegin_internal");

    let dict = vec![0x77u8; 4096];
    let int_max = c_int::MAX as usize; // 2147483647

    for &lvl in &[1i32, 9] {
        for &mode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = LZ4F_max64KB;
            p.frameInfo.blockMode = mode;
            p.compressionLevel = lvl;
            p.autoFlush = 1;

            for &dsz in &[
                int_max + 1, // 0x80000000
                int_max + 2,
                0xFFFF_FFFFusize,
                0x1_0000_0000usize,
                usize::MAX / 2,
                usize::MAX,
            ] {
                for entry in 0..3 {
                    let s = CctxPair::new("row 174");
                    let mut cd = vec![SENTINEL; 64];
                    let mut rd = vec![SENTINEL; 64];
                    let label = format!(
                        "row 174: dictSize={:#x} lvl={} mode={} entry={}",
                        dsz, lvl, mode, entry
                    );
                    unsafe {
                        let (a, b) = match entry {
                            0 => (
                                cbd(
                                    s.c,
                                    cd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    &p as *const _,
                                ),
                                rbd(
                                    s.r,
                                    rd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    &p as *const _,
                                ),
                            ),
                            1 => (
                                cbo(
                                    s.c,
                                    cd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    &p as *const _,
                                ),
                                rbo(
                                    s.r,
                                    rd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    &p as *const _,
                                ),
                            ),
                            _ => (
                                cbi(
                                    s.c,
                                    cd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    ptr::null(),
                                    &p as *const _,
                                ),
                                rbi(
                                    s.r,
                                    rd.as_mut_ptr() as *mut c_void,
                                    64,
                                    dict.as_ptr() as *const c_void,
                                    dsz,
                                    ptr::null(),
                                    &p as *const _,
                                ),
                            ),
                        };
                        expect_err(&label, a, b, err::ERROR_parameter_invalid);
                        // The header has NOT been written yet at lz4frame.c:766.
                        assert!(
                            cd.iter().all(|&x| x == SENTINEL),
                            "{}: C wrote into dst",
                            label
                        );
                        assert_bytes_eq(&label, &cd, &rd);
                    }
                }
            }

            // Accept boundary: INT_MAX itself passes the size check (we do not
            // follow through with a real 2 GB load), and a normal size succeeds.
            let s = CctxPair::new("row 174 boundary");
            let mut cd = vec![SENTINEL; 64];
            let mut rd = vec![SENTINEL; 64];
            unsafe {
                let a = cbd(
                    s.c,
                    cd.as_mut_ptr() as *mut c_void,
                    64,
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    &p as *const _,
                );
                let b = rbd(
                    s.r,
                    rd.as_mut_ptr() as *mut c_void,
                    64,
                    dict.as_ptr() as *const c_void,
                    dict.len(),
                    &p as *const _,
                );
                expect_ok("row 174: valid dictSize", a, b);
                assert_bytes_eq("row 174: valid dictSize header", &cd, &rd);
            }
        }
    }
}

// ===========================================================================
// Rows 176 / 192 — NULL out-parameter for the context creators
// ===========================================================================

/// ERRORS.md row 176 (`LZ4F_createCompressionContext`, lz4frame.c:622) and row
/// 192 (`LZ4F_createDecompressionContext`, lz4frame.c:1304): a NULL out-pointer
/// => `parameter_null`.
///
/// The `assert()` that precedes each check (rows 175 / 191) is compiled out in
/// this build — lz4frame.c:143-149 defines `assert(condition)` as `((void)0)`
/// unless `LZ4_DEBUG >= 1`, and the CMake build defines neither — so the
/// production fallback IS the behaviour of the library under test.
#[test]
fn row_176_192_create_context_null_out_pointer() {
    let (cc, rc) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cd, rd) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");

    // The version argument is never validated (it is merely stored), so sweep it
    // to prove the NULL check is what fires.
    for &v in &[0u32, 1, LZ4F_VERSION, LZ4F_VERSION + 1, 99, 101, u32::MAX] {
        unsafe {
            let a = cc(ptr::null_mut(), v);
            let b = rc(ptr::null_mut(), v);
            expect_err(
                &format!("row 176: LZ4F_createCompressionContext(NULL, {})", v),
                a,
                b,
                err::ERROR_parameter_null,
            );

            let a = cd(ptr::null_mut(), v);
            let b = rd(ptr::null_mut(), v);
            expect_err(
                &format!("row 192: LZ4F_createDecompressionContext(NULL, {})", v),
                a,
                b,
                err::ERROR_parameter_null,
            );
        }
    }

    // Contrast: a real out-pointer succeeds for ANY version (no version check).
    let (cfc, rfc) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cfd, rfd) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    for &v in &[0u32, LZ4F_VERSION, u32::MAX] {
        unsafe {
            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            expect_ok(
                &format!("createCompressionContext(version={})", v),
                cc(&mut a, v),
                rc(&mut b, v),
            );
            assert!(!a.is_null() && !b.is_null());
            same_ret("free cctx", cfc(a), rfc(b));

            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            expect_ok(
                &format!("createDecompressionContext(version={})", v),
                cd(&mut a, v),
                rd(&mut b, v),
            );
            assert!(!a.is_null() && !b.is_null());
            same_ret("free dctx", cfd(a), rfd(b));
        }
    }

    // Free-on-NULL is supported by both and returns OK_NoError.
    unsafe {
        same_ret(
            "LZ4F_freeCompressionContext(NULL)",
            cfc(ptr::null_mut()),
            rfc(ptr::null_mut()),
        );
        assert_eq!(cfc(ptr::null_mut()), 0);
        same_ret(
            "LZ4F_freeDecompressionContext(NULL)",
            cfd(ptr::null_mut()),
            rfd(ptr::null_mut()),
        );
        assert_eq!(cfd(ptr::null_mut()), 0);
    }
}

// ===========================================================================
// Rows 178 / 194 — the *_advanced creators return NULL when the calloc fails
// ===========================================================================

/// ERRORS.md row 178 (`LZ4F_createCompressionContext_advanced`,
/// lz4frame.c:598-600) and row 194 (`LZ4F_createDecompressionContext_advanced`,
/// lz4frame.c:1286-1287): the single `LZ4F_calloc` fails => the function returns
/// NULL (no error code — the return type is a pointer).
///
/// Both `customCalloc`-provided and `customAlloc`-only `LZ4F_CustomMem` shapes
/// are used, because `LZ4F_calloc` takes a different branch for each
/// (lz4frame.c:174-186).
#[test]
fn row_178_194_create_advanced_allocation_failure() {
    let (cca, rca) = both::<FnCreateCctxAdv>("LZ4F_createCompressionContext_advanced");
    let (cda, rda) = both::<FnCreateDctxAdv>("LZ4F_createDecompressionContext_advanced");
    let (cfc, rfc) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cfd, rfd) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");

    for with_calloc in [false, true] {
        // --- row 178
        {
            let mut cst = AllocState::new(1);
            let mut rst = AllocState::new(1);
            unsafe {
                let a = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let b = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(
                    a.is_null(),
                    "row 178: C must return NULL when the cctx calloc fails"
                );
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "row 178: nullness differs (calloc={}) C={:?} Rust={:?}",
                    with_calloc,
                    a,
                    b
                );
            }
            assert_eq!(cst.calls, 1, "row 178: expected exactly one allocation");
            assert_no_leak("row 178", &cst, &rst);
        }
        // --- row 194
        {
            let mut cst = AllocState::new(1);
            let mut rst = AllocState::new(1);
            unsafe {
                let a = cda(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let b = rda(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(
                    a.is_null(),
                    "row 194: C must return NULL when the dctx calloc fails"
                );
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "row 194: nullness differs (calloc={}) C={:?} Rust={:?}",
                    with_calloc,
                    a,
                    b
                );
            }
            assert_eq!(cst.calls, 1, "row 194: expected exactly one allocation");
            assert_no_leak("row 194", &cst, &rst);
        }
        // --- contrast: no injected failure => both succeed, both free cleanly
        {
            let mut cst = AllocState::new(0);
            let mut rst = AllocState::new(0);
            unsafe {
                let a = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let b = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(!a.is_null() && !b.is_null(), "advanced cctx create");
                same_ret("free advanced cctx", cfc(a), rfc(b));
                let a = cda(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let b = rda(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(!a.is_null() && !b.is_null(), "advanced dctx create");
                same_ret("free advanced dctx", cfd(a), rfd(b));
            }
            assert_no_leak("rows 178/194 success path", &cst, &rst);
        }
    }
}

// ===========================================================================
// Rows 179-180 — LZ4F_createCDict{,_advanced} allocation failures
// ===========================================================================

/// ERRORS.md row 179 (`LZ4F_malloc(sizeof(LZ4F_CDict))` fails,
/// lz4frame.c:542-544) and row 180 (any of `dictContent`, `fastCtx`, `HCCtx`
/// fails, lz4frame.c:550-557, after which `LZ4F_freeCDict(cdict)` runs) => the
/// function returns NULL.
///
/// Allocation order inside `LZ4F_createCDict_advanced`: #1 the `LZ4F_CDict`
/// struct, #2 `dictContent` (min(dictSize, 64 KB) bytes), #3 `fastCtx`
/// (`sizeof(LZ4_stream_t)`), #4 `HCCtx` (`sizeof(LZ4_streamHC_t)`). The C issues
/// all three sub-allocations unconditionally and only THEN tests them, so the
/// call count must be 4 for every one of fail_at = 2, 3, 4.
#[test]
fn row_179_180_create_cdict_allocation_failures() {
    let (cca, rca) = both::<FnCreateCDictAdv>("LZ4F_createCDict_advanced");
    let (cfd, rfd) = both::<FnFreeCDict>("LZ4F_freeCDict");

    let mut rng = Rng::new(0x0179_0000_0000_0001);
    // > 64 KB so the "keep only the last 64 KB" path is also taken.
    let big = gen_shape(&mut rng, 4, 100_000);

    for with_calloc in [false, true] {
        for &dsz in &[1usize, 8, 4096, 65536, 100_000] {
            let dict = &big[..dsz];
            for fail_at in 1u64..=4 {
                let mut cst = AllocState::new(fail_at);
                let mut rst = AllocState::new(fail_at);
                let label = format!(
                    "rows 179/180: fail_at={} dictSize={} calloc={}",
                    fail_at, dsz, with_calloc
                );
                unsafe {
                    let a = cca(
                        cmem_for(&mut cst, with_calloc),
                        dict.as_ptr() as *const c_void,
                        dsz,
                    );
                    let b = rca(
                        cmem_for(&mut rst, with_calloc),
                        dict.as_ptr() as *const c_void,
                        dsz,
                    );
                    assert!(a.is_null(), "{}: C must return NULL", label);
                    assert_eq!(
                        a.is_null(),
                        b.is_null(),
                        "{}: nullness differs (C={:?} Rust={:?})",
                        label,
                        a,
                        b
                    );
                }
                let want_calls = if fail_at == 1 { 1 } else { 4 };
                assert_eq!(
                    cst.calls, want_calls,
                    "{}: C allocation call count (expected {})",
                    label, want_calls
                );
                // row 180 explicitly calls LZ4F_freeCDict before returning NULL,
                // so nothing may be left live in either library.
                assert_no_leak(&label, &cst, &rst);
            }

            // Contrast: no injected failure => a usable CDict, freed cleanly.
            let mut cst = AllocState::new(0);
            let mut rst = AllocState::new(0);
            unsafe {
                let a = cca(
                    cmem_for(&mut cst, with_calloc),
                    dict.as_ptr() as *const c_void,
                    dsz,
                );
                let b = rca(
                    cmem_for(&mut rst, with_calloc),
                    dict.as_ptr() as *const c_void,
                    dsz,
                );
                assert!(!a.is_null() && !b.is_null(), "createCDict_advanced success");
                cfd(a);
                rfd(b);
            }
            assert_eq!(cst.calls, 4, "success path allocation count");
            assert_no_leak("rows 179/180 success path", &cst, &rst);
        }
    }

    // LZ4F_freeCDict(NULL) is supported (lz4frame.c:583).
    unsafe {
        cfd(ptr::null_mut());
        rfd(ptr::null_mut());
    }
}

// ===========================================================================
// Rows 177 / 193 — the DEFAULT (stdlib) allocator failing
// ===========================================================================
//
// `LZ4F_createCompressionContext` / `LZ4F_createDecompressionContext` always use
// `LZ4F_defaultCMem`, i.e. plain `calloc()`, so there is no allocator hook to
// inject into. They are therefore driven in a CHILD PROCESS (this very test
// binary, re-executed with a marker in the environment) which
//   1. loads both `.so`s and warms every code path (so no lazy work remains),
//   2. caps `RLIMIT_AS` below its current usage, so all future `mmap`/`brk` fail,
//   3. drains the remaining heap with decreasing request sizes, so even a
//      few-hundred-byte `calloc` can no longer be served,
//   4. calls both libraries and compares the returned error code, then `_exit`s
//      with a distinctive status.
// Nothing is formatted or allocated after step 3.

const CHILD_ENV: &str = "LZ4FRAME_ERRORS_ALLOC_FAIL_CHILD";
const CHILD_TEST: &str = "row_177_193_default_allocator_failure";
/// Deliberately NOT 0: a child that never ran the test would exit 0.
const CHILD_OK: i32 = 77;

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

/// `RLIMIT_AS` on Linux.
const RLIMIT_AS: c_int = 9;

extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn _exit(code: c_int);
}

fn die(code: c_int) -> ! {
    unsafe { _exit(code) };
    #[allow(clippy::empty_loop)]
    loop {}
}

fn child_meaning(code: Option<i32>) -> &'static str {
    match code {
        Some(20) => "warm-up: C LZ4F_createCompressionContext failed",
        Some(21) => "warm-up: Rust LZ4F_createCompressionContext failed",
        Some(22) => "warm-up: C LZ4F_createDecompressionContext failed",
        Some(23) => "warm-up: Rust LZ4F_createDecompressionContext failed",
        Some(24) => "setrlimit(RLIMIT_AS) failed",
        Some(c @ 40..=46) => match c - 40 {
            0 => "heap drain stuck at 1 MB requests (RLIMIT_AS not effective)",
            1 => "heap drain stuck at 64 KB requests",
            2 => "heap drain stuck at 4 KB requests",
            3 => "heap drain stuck at 512 B requests",
            4 => "heap drain stuck at 64 B requests",
            5 => "heap drain stuck at 16 B requests",
            _ => "heap drain stuck at 8 B requests",
        },
        Some(30) => "row 177: C did NOT return allocation_failed (9)",
        Some(31) => "row 177: C and Rust returned different values",
        Some(32) => "row 177: an out-parameter was left non-NULL",
        Some(33) => "row 193: C did NOT return allocation_failed (9)",
        Some(34) => "row 193: C and Rust returned different values",
        Some(35) => "row 193: an out-parameter was left non-NULL",
        Some(77) => "success",
        Some(_) => "child harness exited without running the test body",
        None => "child was killed by a signal",
    }
}

fn row_177_193_child() -> ! {
    let (cc, rc) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cd, rd) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfc, rfc) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cfd, rfd) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");

    unsafe {
        // ---- (1) warm-up: resolve lazy PLT entries, settle the allocators.
        for _ in 0..8 {
            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            if lz4f_is_error(cc(&mut a, LZ4F_VERSION)) {
                die(20);
            }
            if lz4f_is_error(rc(&mut b, LZ4F_VERSION)) {
                die(21);
            }
            cfc(a);
            rfc(b);
            let mut a: *mut c_void = ptr::null_mut();
            let mut b: *mut c_void = ptr::null_mut();
            if lz4f_is_error(cd(&mut a, LZ4F_VERSION)) {
                die(22);
            }
            if lz4f_is_error(rd(&mut b, LZ4F_VERSION)) {
                die(23);
            }
            cfd(a);
            rfd(b);
        }

        // ---- (2) cap the address space below current usage.
        let rl = RLimit { rlim_cur: 4 << 20, rlim_max: 4 << 20 };
        if setrlimit(RLIMIT_AS, &rl) != 0 {
            die(24);
        }

        // ---- (3) drain the remaining heap, largest requests first.
        // NOTE: the allocator is reached through an opaque function pointer.
        // Calling `malloc` directly lets LLVM reason about the allocation family
        // and optimise the "result unused / cannot be NULL" drain loop away.
        let mfn: unsafe extern "C" fn(usize) -> *mut c_void = std::hint::black_box(malloc);
        for (i, &sz) in [1usize << 20, 1 << 16, 4096, 512, 64, 16, 8].iter().enumerate() {
            let mut guard: u64 = 0;
            loop {
                let p = std::hint::black_box(mfn(std::hint::black_box(sz)));
                if p.is_null() {
                    break;
                }
                guard += 1;
                if guard > 4_000_000 {
                    die(40 + i as c_int);
                }
            }
        }

        // ---- (4) row 177
        let mut a: *mut c_void = ptr::null_mut();
        let mut b: *mut c_void = ptr::null_mut();
        let ca = cc(&mut a, LZ4F_VERSION);
        let ra = rc(&mut b, LZ4F_VERSION);
        if !lz4f_is_error(ca) || lz4f_error_code(ca) != err::ERROR_allocation_failed {
            die(30);
        }
        if ca != ra {
            die(31);
        }
        if !a.is_null() || !b.is_null() {
            die(32);
        }

        // ---- row 193
        let mut a: *mut c_void = ptr::null_mut();
        let mut b: *mut c_void = ptr::null_mut();
        let ca = cd(&mut a, LZ4F_VERSION);
        let ra = rd(&mut b, LZ4F_VERSION);
        if !lz4f_is_error(ca) || lz4f_error_code(ca) != err::ERROR_allocation_failed {
            die(33);
        }
        if ca != ra {
            die(34);
        }
        if !a.is_null() || !b.is_null() {
            die(35);
        }

        die(CHILD_OK);
    }
}

/// ERRORS.md row 177 (`LZ4F_createCompressionContext`, lz4frame.c:624-625) and
/// row 193 (`LZ4F_createDecompressionContext`, lz4frame.c:1306-1309): the
/// `_advanced` helper returns NULL because the default `calloc` failed =>
/// `allocation_failed` (9), with `*ctxPtr` left NULL.
#[test]
fn row_177_193_default_allocator_failure() {
    if std::env::var_os(CHILD_ENV).is_some() {
        row_177_193_child();
    }
    let exe = std::env::current_exe().expect("current_exe()");
    let out = std::process::Command::new(&exe)
        .arg(CHILD_TEST)
        .arg("--exact")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .output()
        .expect("could not re-execute the test binary as a child process");
    let code = out.status.code();
    assert_eq!(
        code,
        Some(CHILD_OK),
        "rows 177/193: child exited with {:?} ({})\n--- child stdout ---\n{}\n--- child stderr ---\n{}",
        code,
        child_meaning(code),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// ---------------------------------------------------------------------------
// `LZ4F_compressBound_internal` (lz4frame.c:900-925) replicated locally, so the
// tests can name the exact capacity that must be rejected instead of guessing.
// ---------------------------------------------------------------------------

fn bound_internal(src_size: usize, p: &LZ4F_preferences_t, already_buffered: usize) -> usize {
    let block_size = block_size_of(p.frameInfo.blockSizeID);
    let flush = p.autoFlush != 0 || src_size == 0;
    let max_buffered = block_size - 1;
    let buffered = already_buffered.min(max_buffered);
    let max_src = src_size + buffered;
    let nb_full = max_src / block_size;
    let partial = max_src & (block_size - 1);
    let last = if flush { partial } else { 0 };
    let nb_blocks = nb_full + usize::from(last > 0);
    let block_crc = LZ4F_BLOCK_CHECKSUM_SIZE * p.frameInfo.blockChecksumFlag as usize;
    let frame_end =
        LZ4F_BLOCK_HEADER_SIZE + LZ4F_CONTENT_CHECKSUM_SIZE * p.frameInfo.contentChecksumFlag as usize;
    (LZ4F_BLOCK_HEADER_SIZE + block_crc) * nb_blocks + block_size * nb_full + last + frame_end
}

/// A pair of contexts that have already had `LZ4F_compressBegin` applied.
fn begin_pair(label: &str, p: &LZ4F_preferences_t) -> CctxPair {
    let s = CctxPair::new(label);
    let (cb, rb) = both::<FnBegin>("LZ4F_compressBegin");
    let mut cd = vec![SENTINEL; 64];
    let mut rd = vec![SENTINEL; 64];
    unsafe {
        let a = cb(s.c, cd.as_mut_ptr() as *mut c_void, 64, p as *const _);
        let b = rb(s.r, rd.as_mut_ptr() as *mut c_void, 64, p as *const _);
        expect_ok(&format!("{}: LZ4F_compressBegin", label), a, b);
    }
    assert_bytes_eq(&format!("{}: frame header", label), &cd, &rd);
    s
}

// ===========================================================================
// Row 181 — compressUpdate / uncompressedUpdate on an uninitialised state
// ===========================================================================

/// ERRORS.md row 181: `cctxPtr->cStage != 1` (lz4frame.c:1005) =>
/// `compressionState_uninitialized`, i.e. `LZ4F_compressBegin*` was never called
/// or `LZ4F_compressEnd` already reset the context.
/// (Also covered by `frame_stream_error_state_uninitialized` in
/// tests/lz4frame_stream_diff.rs.)
#[test]
fn row_181_update_before_begin() {
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");
    let (cn, rn) = both::<FnUpdate>("LZ4F_uncompressedUpdate");
    let (ce, re) = both::<FnFlush>("LZ4F_compressEnd");

    let mut rng = Rng::new(0x0181_0000_0000_0001);
    let src = gen_shape(&mut rng, 4, 4096);
    let copts = LZ4F_compressOptions_t::default();

    // ---- (a) never begun
    for (name, cfn, rfn) in [
        ("LZ4F_compressUpdate", cu, ru),
        ("LZ4F_uncompressedUpdate", cn, rn),
    ] {
        for &srcsz in &[0usize, 1, 4096] {
            for &cap in &[0usize, 1, 8, 70_000] {
                for use_opts in [false, true] {
                    let s = CctxPair::new("row 181");
                    let mut cd = vec![SENTINEL; cap.max(1)];
                    let mut rd = vec![SENTINEL; cap.max(1)];
                    let op = if use_opts {
                        &copts as *const LZ4F_compressOptions_t
                    } else {
                        ptr::null()
                    };
                    let label = format!(
                        "row 181: fresh cctx {} srcSize={} cap={} opts={}",
                        name, srcsz, cap, use_opts
                    );
                    unsafe {
                        let a = cfn(
                            s.c,
                            cd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            srcsz,
                            op,
                        );
                        let b = rfn(
                            s.r,
                            rd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            srcsz,
                            op,
                        );
                        expect_err(&label, a, b, err::ERROR_compressionState_uninitialized);
                    }
                    assert_bytes_eq(&format!("{}: dst untouched", label), &cd, &rd);
                    assert!(cd.iter().all(|&x| x == SENTINEL), "{}: C wrote", label);
                }
            }
        }
    }

    // ---- (b) after LZ4F_compressEnd reset cStage back to 0
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = LZ4F_max64KB;
    p.frameInfo.blockMode = LZ4F_blockIndependent;
    p.compressionLevel = 1;
    p.autoFlush = 1;
    for (name, cfn, rfn) in [
        ("LZ4F_compressUpdate", cu, ru),
        ("LZ4F_uncompressedUpdate", cn, rn),
    ] {
        let s = begin_pair("row 181 (b)", &p);
        let mut cd = vec![SENTINEL; 70_000];
        let mut rd = vec![SENTINEL; 70_000];
        unsafe {
            // finish the frame: cStage -> 0
            let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cd.len(), ptr::null());
            let b = re(s.r, rd.as_mut_ptr() as *mut c_void, rd.len(), ptr::null());
            expect_ok("row 181: compressEnd", a, b);
            assert_bytes_eq("row 181: frame tail", &cd, &rd);

            let mut cd = vec![SENTINEL; 70_000];
            let mut rd = vec![SENTINEL; 70_000];
            let a = cfn(
                s.c,
                cd.as_mut_ptr() as *mut c_void,
                cd.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            let b = rfn(
                s.r,
                rd.as_mut_ptr() as *mut c_void,
                rd.len(),
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            expect_err(
                &format!("row 181: {} after compressEnd", name),
                a,
                b,
                err::ERROR_compressionState_uninitialized,
            );
            assert_bytes_eq(
                &format!("row 181: {} after compressEnd dst", name),
                &cd,
                &rd,
            );
        }
    }
}

// ===========================================================================
// Rows 182-183 — the two dstCapacity checks of LZ4F_compressUpdateImpl
// ===========================================================================

/// ERRORS.md row 182: `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs,
/// tmpInSize)` (lz4frame.c:1006-1007) and row 183: the extra
/// `blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize` check
/// (lz4frame.c:1009-1010). Both => `dstMaxSize_tooSmall`.
///
/// Row 183 is only distinguishable from row 182 when `bound_internal < srcSize`,
/// which needs `autoFlush == 0` (so `lastBlockSize == 0`) and
/// `srcSize < maxBlockSize`; then the bound collapses to just the frame end
/// (4 or 8 bytes) while `srcSize` is large.
/// (Row 182 is also covered by `frame_stream_error_update_capacity` in
/// tests/lz4frame_stream_diff.rs.)
#[test]
fn row_182_183_update_dst_too_small() {
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");
    let (cn, rn) = both::<FnUpdate>("LZ4F_uncompressedUpdate");

    let mut rng = Rng::new(0x0182_0000_0000_0001);

    // ---------------- row 182, LZ4F_compressUpdate
    for &af in &[0u32, 1] {
        for &(cck, bck) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            for &mode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = LZ4F_max64KB;
                p.frameInfo.blockMode = mode;
                p.frameInfo.contentChecksumFlag = cck;
                p.frameInfo.blockChecksumFlag = bck;
                p.compressionLevel = 1;
                p.autoFlush = af;

                for &srcsz in &[0usize, 1, 100, 65_535, 65_536, 70_000] {
                    let bnd = bound_internal(srcsz, &p, 0);
                    let src = gen_shape(&mut rng, srcsz % N_SHAPES, srcsz);
                    let mut caps: Vec<usize> = vec![0, 1, 2, 3];
                    for d in 1..=3usize {
                        if bnd >= d {
                            caps.push(bnd - d);
                        }
                    }
                    caps.push(bnd / 2);
                    caps.push(bnd);
                    caps.retain(|&c| c <= bnd);
                    caps.sort_unstable();
                    caps.dedup();

                    for &cap in &caps {
                        let s = begin_pair("row 182", &p);
                        // generous allocation: a rejected call must write nothing
                        let mut cd = vec![SENTINEL; bnd + 64];
                        let mut rd = vec![SENTINEL; bnd + 64];
                        let label = format!(
                            "row 182: af={} cck={} bck={} mode={} srcSize={} cap={} (bound={})",
                            af, cck, bck, mode, srcsz, cap, bnd
                        );
                        unsafe {
                            let a = cu(
                                s.c,
                                cd.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                srcsz,
                                ptr::null(),
                            );
                            let b = ru(
                                s.r,
                                rd.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                srcsz,
                                ptr::null(),
                            );
                            if cap < bnd {
                                expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                                assert!(
                                    cd.iter().all(|&x| x == SENTINEL),
                                    "{}: C wrote despite rejecting",
                                    label
                                );
                            } else {
                                expect_ok(&label, a, b);
                            }
                        }
                        assert_bytes_eq(&label, &cd, &rd);
                    }
                }
            }
        }
    }

    // ---------------- row 183, LZ4F_uncompressedUpdate (blockIndependent only)
    for &(cck, bck) in &[(0, 0), (1, 1)] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = LZ4F_max64KB;
        p.frameInfo.blockMode = LZ4F_blockIndependent; // lz4frame.h:707
        p.frameInfo.contentChecksumFlag = cck;
        p.frameInfo.blockChecksumFlag = bck;
        p.compressionLevel = 1;
        p.autoFlush = 0; // required so bound_internal < srcSize

        for &srcsz in &[1usize, 9, 1000, 5000] {
            let bnd = bound_internal(srcsz, &p, 0);
            assert!(
                bnd <= LZ4F_BLOCK_HEADER_SIZE + LZ4F_CONTENT_CHECKSUM_SIZE,
                "row 183 setup: expected the bound to collapse to the frame end, got {}",
                bnd
            );
            let src = gen_shape(&mut rng, srcsz % N_SHAPES, srcsz);
            let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 8];
            for d in 1..=2usize {
                if srcsz >= d {
                    caps.push(srcsz - d);
                }
            }
            caps.push(srcsz);
            caps.push(srcsz + 1);
            caps.sort_unstable();
            caps.dedup();

            for &cap in &caps {
                let s = begin_pair("row 183", &p);
                let mut cd = vec![SENTINEL; srcsz + 64];
                let mut rd = vec![SENTINEL; srcsz + 64];
                let label = format!(
                    "rows 182/183: uncompressedUpdate cck={} bck={} srcSize={} cap={} (bound={})",
                    cck, bck, srcsz, cap, bnd
                );
                unsafe {
                    let a = cn(
                        s.c,
                        cd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        srcsz,
                        ptr::null(),
                    );
                    let b = rn(
                        s.r,
                        rd.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        srcsz,
                        ptr::null(),
                    );
                    if cap < bnd.max(srcsz) {
                        // cap < bound  => row 182 ; bound <= cap < srcSize => row 183
                        expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                        assert!(
                            cd.iter().all(|&x| x == SENTINEL),
                            "{}: C wrote despite rejecting",
                            label
                        );
                    } else {
                        expect_ok(&label, a, b);
                    }
                }
                assert_bytes_eq(&label, &cd, &rd);
            }
        }
    }
}

/// `begin` + one `LZ4F_compressUpdate` that only BUFFERS `n` bytes
/// (`autoFlush == 0`, `n < maxBlockSize`), leaving `tmpInSize == n`.
fn buffered_pair(label: &str, p: &LZ4F_preferences_t, data: &[u8]) -> CctxPair {
    assert_eq!(p.autoFlush, 0, "{}: buffering needs autoFlush == 0", label);
    assert!(data.len() < block_size_of(p.frameInfo.blockSizeID));
    let s = begin_pair(label, p);
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");
    let cap = bound_internal(data.len(), p, 0) + 64;
    let mut cd = vec![SENTINEL; cap];
    let mut rd = vec![SENTINEL; cap];
    unsafe {
        let a = cu(
            s.c,
            cd.as_mut_ptr() as *mut c_void,
            cap,
            data.as_ptr() as *const c_void,
            data.len(),
            ptr::null(),
        );
        let b = ru(
            s.r,
            rd.as_mut_ptr() as *mut c_void,
            cap,
            data.as_ptr() as *const c_void,
            data.len(),
            ptr::null(),
        );
        expect_ok(&format!("{}: buffering update", label), a, b);
        assert_eq!(a, 0, "{}: buffering update should emit nothing", label);
    }
    assert_bytes_eq(&format!("{}: buffering update dst", label), &cd, &rd);
    s
}

// ===========================================================================
// Rows 185-186 — LZ4F_flush state check and capacity check
// ===========================================================================

/// ERRORS.md row 185: `LZ4F_flush` only reports `compressionState_uninitialized`
/// when `tmpInSize != 0` **and** `cStage != 1` (lz4frame.c:1167-1168), and row
/// 186: `dstCapacity < tmpInSize + BHSize + BFSize` (== `tmpInSize + 8`) with
/// `tmpInSize > 0` (lz4frame.c:1169) => `dstMaxSize_tooSmall`.
///
/// ROW 185 — reachability of the error branch. `cStage` only ever takes the
/// values 0 and 1: it is set to 0 in `LZ4F_createCompressionContext_advanced`
/// (lz4frame.c:604) and in `LZ4F_compressEnd` (lz4frame.c:1233), and to 1 at the
/// end of `LZ4F_compressBegin_internal` (lz4frame.c:811). Both `cStage == 0`
/// sites necessarily have `tmpInSize == 0`:
///   * a freshly created context is `calloc`ed, so `tmpInSize == 0`;
///   * `LZ4F_compressEnd` runs `LZ4F_flush()` FIRST and forwards its error
///     (lz4frame.c:1213-1214), so line 1233 is only reached after a flush that
///     set `tmpInSize = 0`.
/// Every `LZ4F_compressBegin*` failure path also leaves `cStage` at its previous
/// value, and the only path that leaves data buffered after an internal flush
/// failure (row 187) keeps `cStage == 1`. So `tmpInSize != 0 && cStage != 1` is
/// unreachable through the public API; what IS observable — and what this test
/// pins — is the early `return 0` that makes `LZ4F_flush` / `LZ4F_compressEnd`
/// on a fresh or finished context NOT report an uninitialised state.
#[test]
fn row_185_186_flush_state_and_capacity() {
    let (cfl, rfl) = both::<FnFlush>("LZ4F_flush");
    let (ce, re) = both::<FnFlush>("LZ4F_compressEnd");

    let mut rng = Rng::new(0x0185_0000_0000_0001);

    // ---------------- row 185: flush/compressEnd on a NEVER-BEGUN context
    for &cap in &[0usize, 1, 3, 4, 8, 64] {
        let s = CctxPair::new("row 185 fresh");
        let mut cd = vec![SENTINEL; cap.max(1)];
        let mut rd = vec![SENTINEL; cap.max(1)];
        let label = format!("row 185: LZ4F_flush on a fresh cctx, cap={}", cap);
        unsafe {
            let a = cfl(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
            let b = rfl(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
            same_ret(&label, a, b);
            assert_eq!(
                a, 0,
                "{}: tmpInSize == 0 must return 0 BEFORE the state check",
                label
            );
        }
        assert_bytes_eq(&label, &cd, &rd);
    }

    // ... and compressEnd on a fresh context: emits a bare endMark, no error 20.
    for &cap in &[0usize, 1, 3, 4, 8, 64] {
        let s = CctxPair::new("row 185 fresh end");
        let mut cd = vec![SENTINEL; cap.max(1)];
        let mut rd = vec![SENTINEL; cap.max(1)];
        let label = format!("row 185: LZ4F_compressEnd on a fresh cctx, cap={}", cap);
        unsafe {
            let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
            let b = re(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
            same_ret(&label, a, b);
            if cap < 4 {
                // row 188 (no room for the endMark), NOT compressionState_uninitialized
                expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
            } else {
                expect_ok(&label, a, b);
                assert_eq!(a, 4, "{}: expected a bare 4-byte endMark", label);
            }
        }
        assert_bytes_eq(&label, &cd, &rd);
    }

    // ... and flush AFTER a completed frame (cStage back to 0, tmpInSize == 0).
    {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = LZ4F_max64KB;
        p.frameInfo.blockMode = LZ4F_blockIndependent;
        p.compressionLevel = 1;
        p.autoFlush = 0;
        let data = gen_shape(&mut rng, 2, 1000);
        let s = buffered_pair("row 185 finished", &p, &data);
        let mut cd = vec![SENTINEL; 70_000];
        let mut rd = vec![SENTINEL; 70_000];
        unsafe {
            let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cd.len(), ptr::null());
            let b = re(s.r, rd.as_mut_ptr() as *mut c_void, rd.len(), ptr::null());
            expect_ok("row 185: compressEnd", a, b);
            assert_bytes_eq("row 185: frame tail", &cd, &rd);
            for &cap in &[0usize, 1, 7, 8, 64] {
                let mut cd = vec![SENTINEL; cap.max(1)];
                let mut rd = vec![SENTINEL; cap.max(1)];
                let label = format!("row 185: LZ4F_flush after compressEnd, cap={}", cap);
                let a = cfl(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                let b = rfl(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                same_ret(&label, a, b);
                assert_eq!(a, 0, "{}: must return 0, not an error", label);
                assert_bytes_eq(&label, &cd, &rd);
            }
        }
    }

    // ---------------- row 186: dstCapacity < tmpInSize + 8, tmpInSize > 0
    for &(cck, bck) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
        for &mode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &n in &[1usize, 9, 1000, 40_000] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = LZ4F_max64KB;
                p.frameInfo.blockMode = mode;
                p.frameInfo.contentChecksumFlag = cck;
                p.frameInfo.blockChecksumFlag = bck;
                p.compressionLevel = 1;
                p.autoFlush = 0;
                let data = gen_shape(&mut rng, n % N_SHAPES, n);
                let threshold = n + LZ4F_BLOCK_HEADER_SIZE + LZ4F_BLOCK_CHECKSUM_SIZE; // tmpInSize+8

                let mut caps: Vec<usize> = vec![0, 1, 7, 8];
                for d in 1..=2usize {
                    if threshold >= d {
                        caps.push(threshold - d);
                    }
                }
                caps.push(threshold);
                caps.push(threshold + 1);
                caps.sort_unstable();
                caps.dedup();

                for &cap in &caps {
                    // ---- through LZ4F_flush
                    let s = buffered_pair("row 186", &p, &data);
                    let mut cd = vec![SENTINEL; threshold + 64];
                    let mut rd = vec![SENTINEL; threshold + 64];
                    let label = format!(
                        "row 186: flush cck={} bck={} mode={} tmpInSize={} cap={} (threshold={})",
                        cck, bck, mode, n, cap, threshold
                    );
                    unsafe {
                        let a = cfl(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                        let b = rfl(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                        if cap < threshold {
                            expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                            assert!(
                                cd.iter().all(|&x| x == SENTINEL),
                                "{}: C wrote despite rejecting",
                                label
                            );
                        } else {
                            expect_ok(&label, a, b);
                        }
                    }
                    assert_bytes_eq(&label, &cd, &rd);

                    // ---- and through LZ4F_compressEnd, which forwards it
                    let s = buffered_pair("row 186 end", &p, &data);
                    let mut cd = vec![SENTINEL; threshold + 64];
                    let mut rd = vec![SENTINEL; threshold + 64];
                    let label = format!(
                        "row 186: compressEnd cck={} bck={} mode={} tmpInSize={} cap={}",
                        cck, bck, mode, n, cap
                    );
                    unsafe {
                        let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                        let b = re(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                        same_ret(&label, a, b);
                        if cap < threshold {
                            expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                        }
                    }
                    assert_bytes_eq(&label, &cd, &rd);
                }
            }
        }
    }
}

// ===========================================================================
// Row 187 — the un-checked internal LZ4F_flush inside LZ4F_compressUpdateImpl
// ===========================================================================

/// ERRORS.md row 187: on a `blockCompressMode` switch with buffered data,
/// `LZ4F_compressUpdateImpl` calls `LZ4F_flush()` at lz4frame.c:1014 and does
/// **not** error-check the result — it simply does `dstPtr += bytesWritten`.
///
/// Two observable consequences, both pinned here:
///   (a) if that flush FAILS, `-(size_t)LZ4F_ERROR_dstMaxSize_tooSmall` is added
///       to `dstPtr` and the function keeps going; with no further output the
///       return value is the wrapped `size_t`, which happens to look exactly
///       like `dstMaxSize_tooSmall`;
///   (b) if that flush SUCCEEDS, the remaining budget is never reduced, so the
///       call can legitimately write and RETURN MORE than `dstCapacity`.
/// Case (b) needs a slack region past `dstCapacity` so the overrun lands in our
/// own allocation. (`tests/lz4frame_dstcapacity_overrun.rs` pins case (b) as a
/// dedicated regression test; it is repeated here so the row is self-contained.)
#[test]
fn row_187_update_internal_flush_not_error_checked() {
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");
    let (cn, rn) = both::<FnUpdate>("LZ4F_uncompressedUpdate");

    let mut rng = Rng::new(0x0187_0000_0000_0001);
    let bs = 64 * 1024usize;
    /// Slack past `dstCapacity`, large enough for a whole flushed block + a full
    /// stored block.
    const SLACK: usize = 1 << 18;

    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = LZ4F_max64KB;
    // `LZ4F_uncompressedUpdate` is only supported with blockIndependent
    // (lz4frame.h:707); blockLinked violates the assert at lz4frame.c:1071.
    p.frameInfo.blockMode = LZ4F_blockIndependent;
    p.compressionLevel = 1;
    p.autoFlush = 0;

    // ---------------- (a) the internal flush FAILS
    {
        let buffered = gen_shape(&mut rng, 0, 1000);
        let s = CctxPair::new("row 187a");
        {
            // begin, then buffer 1000 bytes through the UNCOMPRESSED path so that
            // the next compressUpdate has to switch mode.
            let (cb, rb) = both::<FnBegin>("LZ4F_compressBegin");
            let mut cd = vec![SENTINEL; 64];
            let mut rd = vec![SENTINEL; 64];
            unsafe {
                expect_ok(
                    "row 187a: begin",
                    cb(s.c, cd.as_mut_ptr() as *mut c_void, 64, &p as *const _),
                    rb(s.r, rd.as_mut_ptr() as *mut c_void, 64, &p as *const _),
                );
            }
            assert_bytes_eq("row 187a: header", &cd, &rd);

            let cap = buffered.len() + 64;
            let mut cd = vec![SENTINEL; cap];
            let mut rd = vec![SENTINEL; cap];
            unsafe {
                let a = cn(
                    s.c,
                    cd.as_mut_ptr() as *mut c_void,
                    cap,
                    buffered.as_ptr() as *const c_void,
                    buffered.len(),
                    ptr::null(),
                );
                let b = rn(
                    s.r,
                    rd.as_mut_ptr() as *mut c_void,
                    cap,
                    buffered.as_ptr() as *const c_void,
                    buffered.len(),
                    ptr::null(),
                );
                expect_ok("row 187a: uncompressedUpdate buffering", a, b);
                assert_eq!(a, 0, "row 187a: expected the data to be buffered only");
            }
            assert_bytes_eq("row 187a: buffering dst", &cd, &rd);
        }

        // Now compressUpdate with a capacity that satisfies row 182 (the internal
        // bound is just the 4-byte frame end) but is far below tmpInSize+8 = 1008.
        let tail = [0x5Au8; 1];
        let cap = bound_internal(tail.len(), &p, 1000);
        assert_eq!(cap, LZ4F_BLOCK_HEADER_SIZE, "row 187a: unexpected internal bound");
        let mut cd = vec![SENTINEL; cap + SLACK];
        let mut rd = vec![SENTINEL; cap + SLACK];
        unsafe {
            let a = cu(
                s.c,
                cd.as_mut_ptr() as *mut c_void,
                cap,
                tail.as_ptr() as *const c_void,
                tail.len(),
                ptr::null(),
            );
            let b = ru(
                s.r,
                rd.as_mut_ptr() as *mut c_void,
                cap,
                tail.as_ptr() as *const c_void,
                tail.len(),
                ptr::null(),
            );
            same_ret("row 187a: compressUpdate after a failing internal flush", a, b);
            // The wrapped size_t is bit-identical to the dstMaxSize_tooSmall code.
            assert_eq!(
                a,
                0usize.wrapping_sub(err::ERROR_dstMaxSize_tooSmall as usize),
                "row 187a: expected the wrapped dstMaxSize_tooSmall value, got {}",
                describe(a)
            );
            assert!(
                cd.iter().all(|&x| x == SENTINEL),
                "row 187a: the C wrote into dst"
            );
        }
        assert_bytes_eq("row 187a: dst + slack", &cd, &rd);
        // The context is in a UB state per lz4frame.h:704; it is only freed.
    }

    // ---------------- (b) the internal flush SUCCEEDS and the budget is not reduced
    {
        // Incompressible data, so the flushed block is stored at full size and the
        // overrun is unambiguous.
        let buffered = gen_random(&mut rng, bs - 10);
        let full = gen_random(&mut rng, bs);
        let s = begin_pair("row 187b", &p);

        // buffer bs-10 bytes through the COMPRESSED path
        let cap0 = bound_internal(buffered.len(), &p, 0) + 64;
        let mut cd = vec![SENTINEL; cap0];
        let mut rd = vec![SENTINEL; cap0];
        unsafe {
            let a = cu(
                s.c,
                cd.as_mut_ptr() as *mut c_void,
                cap0,
                buffered.as_ptr() as *const c_void,
                buffered.len(),
                ptr::null(),
            );
            let b = ru(
                s.r,
                rd.as_mut_ptr() as *mut c_void,
                cap0,
                buffered.as_ptr() as *const c_void,
                buffered.len(),
                ptr::null(),
            );
            expect_ok("row 187b: buffering compressUpdate", a, b);
            assert_eq!(a, 0, "row 187b: expected buffering only");
        }
        assert_bytes_eq("row 187b: buffering dst", &cd, &rd);

        // Switch to UNCOMPRESSED with a FULL block: the internal flush succeeds and
        // then a whole stored block is appended past the declared capacity.
        let cap = bound_internal(full.len(), &p, buffered.len());
        let mut cd = vec![SENTINEL; cap + SLACK];
        let mut rd = vec![SENTINEL; cap + SLACK];
        unsafe {
            let a = cn(
                s.c,
                cd.as_mut_ptr() as *mut c_void,
                cap,
                full.as_ptr() as *const c_void,
                full.len(),
                ptr::null(),
            );
            let b = rn(
                s.r,
                rd.as_mut_ptr() as *mut c_void,
                cap,
                full.as_ptr() as *const c_void,
                full.len(),
                ptr::null(),
            );
            same_ret("row 187b: uncompressedUpdate across a mode switch", a, b);
            expect_ok("row 187b: uncompressedUpdate across a mode switch", a, b);
            assert!(
                a > cap,
                "row 187b: expected the documented over-write (returned {} for dstCapacity {})",
                a,
                cap
            );
            assert!(
                a <= cap + SLACK,
                "row 187b: the overrun ({}) exceeded the slack region ({})",
                a,
                cap + SLACK
            );
        }
        assert_bytes_eq("row 187b: dst + slack (including the overrun)", &cd, &rd);
    }
}

// ===========================================================================
// Rows 188-190 — LZ4F_compressEnd errors
// ===========================================================================

/// ERRORS.md row 188: after the internal flush, `dstCapacity - flushSize < 4`
/// (no room for the endMark, lz4frame.c:1221); row 189: content checksum
/// requested and `dstCapacity < 8` after the endMark (lz4frame.c:1225-1227);
/// row 190: `frameInfo.contentSize != 0 && contentSize != totalInSize`
/// (lz4frame.c:1235-1237) => `frameSize_wrong`, with `cStage` already reset and
/// the endMark/checksum already written into `dstBuffer`.
/// (Rows 188/189 are also covered by `frame_stream_error_end_capacity`, row 190
/// by `frame_stream_error_content_size_mismatch`, in
/// tests/lz4frame_stream_diff.rs.)
#[test]
fn row_188_189_190_compress_end_errors() {
    let (ce, re) = both::<FnFlush>("LZ4F_compressEnd");
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");

    let mut rng = Rng::new(0x0188_0000_0000_0001);

    // ---------------- rows 188 & 189: no buffered data, so flushSize == 0
    for &cck in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = LZ4F_max64KB;
        p.frameInfo.blockMode = LZ4F_blockIndependent;
        p.frameInfo.contentChecksumFlag = cck;
        p.compressionLevel = 1;
        p.autoFlush = 1;
        let need = if cck == LZ4F_contentChecksumEnabled { 8 } else { 4 };

        for cap in 0..=12usize {
            let s = begin_pair("rows 188/189", &p);
            let mut cd = vec![SENTINEL; 16];
            let mut rd = vec![SENTINEL; 16];
            let label = format!("rows 188/189: cck={} cap={} (need={})", cck, cap, need);
            unsafe {
                let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                let b = re(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                if cap < need {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                    if cap >= 4 {
                        // row 189: the endMark HAS already been written
                        assert_eq!(
                            &cd[..4],
                            &[0u8, 0, 0, 0],
                            "{}: the endMark must already be in dst",
                            label
                        );
                    } else {
                        // row 188: nothing was written at all
                        assert!(
                            cd.iter().all(|&x| x == SENTINEL),
                            "{}: nothing may be written",
                            label
                        );
                    }
                } else {
                    expect_ok(&label, a, b);
                    assert_eq!(a, need, "{}: expected exactly {} bytes", label, need);
                }
            }
            assert_bytes_eq(&label, &cd, &rd);
        }
    }

    // ---------------- row 188 again, this time with buffered data so that
    // flushSize > 0 and the endMark check works on the REMAINING capacity.
    //
    // The window is narrow: `LZ4F_compressEnd`'s internal flush needs
    // `dstCapacity >= tmpInSize + 8` (row 186), while row 188 needs
    // `dstCapacity - flushSize < 4`. That is only satisfiable when the flush
    // writes the FULL `tmpInSize + 8` bytes, i.e. INCOMPRESSIBLE data (stored via
    // `LZ4F_BLOCKUNCOMPRESSED_FLAG`, lz4frame.c:895-899) with the block checksum
    // enabled so `flushSize == tmpInSize + BHSize + BFSize`.
    {
        let n = 1000usize;
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = LZ4F_max64KB;
        p.frameInfo.blockMode = LZ4F_blockIndependent;
        p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
        p.compressionLevel = 1;
        p.autoFlush = 0;
        let data = gen_random(&mut rng, n); // incompressible => stored at full size
        let flushed = {
            // find out how many bytes the flush itself produces
            let s = buffered_pair("row 188 probe", &p, &data);
            let (cfl, rfl) = both::<FnFlush>("LZ4F_flush");
            let mut cd = vec![SENTINEL; 70_000];
            let mut rd = vec![SENTINEL; 70_000];
            unsafe {
                let a = cfl(s.c, cd.as_mut_ptr() as *mut c_void, cd.len(), ptr::null());
                let b = rfl(s.r, rd.as_mut_ptr() as *mut c_void, rd.len(), ptr::null());
                expect_ok("row 188 probe: flush", a, b);
                assert_bytes_eq("row 188 probe: flushed block", &cd, &rd);
                a
            }
        };
        assert_eq!(
            flushed,
            n + LZ4F_BLOCK_HEADER_SIZE + LZ4F_BLOCK_CHECKSUM_SIZE,
            "row 188 probe: expected the block to be STORED at full size"
        );

        for extra in 0..=5usize {
            let cap = flushed + extra;
            let s = buffered_pair("row 188", &p, &data);
            let mut cd = vec![SENTINEL; flushed + 32];
            let mut rd = vec![SENTINEL; flushed + 32];
            let label = format!("row 188: flushed={} cap={} (extra={})", flushed, cap, extra);
            unsafe {
                let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                let b = re(s.r, rd.as_mut_ptr() as *mut c_void, cap, ptr::null());
                if extra < 4 {
                    expect_err(&label, a, b, err::ERROR_dstMaxSize_tooSmall);
                } else {
                    expect_ok(&label, a, b);
                    assert_eq!(a, flushed + 4, "{}: expected flush + endMark", label);
                }
            }
            assert_bytes_eq(&label, &cd, &rd);
        }
    }

    // ---------------- row 190: declared contentSize != bytes actually fed
    for &cck in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
        for &declared in &[1u64, 100, 1000, 70_000] {
            for &fed in &[0usize, 1, 99, 100, 101, 1000] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = LZ4F_max64KB;
                p.frameInfo.blockMode = LZ4F_blockIndependent;
                p.frameInfo.contentChecksumFlag = cck;
                p.frameInfo.contentSize = declared;
                p.compressionLevel = 1;
                p.autoFlush = 1;

                let src = gen_shape(&mut rng, fed % N_SHAPES, fed);
                let s = begin_pair("row 190", &p);
                if fed > 0 {
                    let cap = bound_internal(fed, &p, 0) + 64;
                    let mut cd = vec![SENTINEL; cap];
                    let mut rd = vec![SENTINEL; cap];
                    unsafe {
                        let a = cu(
                            s.c,
                            cd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            fed,
                            ptr::null(),
                        );
                        let b = ru(
                            s.r,
                            rd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            fed,
                            ptr::null(),
                        );
                        expect_ok("row 190: update", a, b);
                    }
                    assert_bytes_eq("row 190: update dst", &cd, &rd);
                }

                let mut cd = vec![SENTINEL; 64];
                let mut rd = vec![SENTINEL; 64];
                let label = format!(
                    "row 190: cck={} declared={} fed={}",
                    cck, declared, fed
                );
                unsafe {
                    let a = ce(s.c, cd.as_mut_ptr() as *mut c_void, 64, ptr::null());
                    let b = re(s.r, rd.as_mut_ptr() as *mut c_void, 64, ptr::null());
                    if declared as usize == fed {
                        expect_ok(&label, a, b);
                    } else {
                        expect_err(&label, a, b, err::ERROR_frameSize_wrong);
                        // The endMark (and checksum) were written BEFORE the check.
                        assert_eq!(
                            &cd[..4],
                            &[0u8, 0, 0, 0],
                            "{}: the endMark must already be in dst",
                            label
                        );
                    }
                }
                assert_bytes_eq(&label, &cd, &rd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Decompression-context pair + LZ4F_getFrameInfo helper
// ---------------------------------------------------------------------------

struct DctxPair {
    c: *mut c_void,
    r: *mut c_void,
}

impl DctxPair {
    fn new(label: &str) -> DctxPair {
        let (cn, rn) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");
        let mut c: *mut c_void = ptr::null_mut();
        let mut r: *mut c_void = ptr::null_mut();
        unsafe {
            expect_ok(
                &format!("{}: LZ4F_createDecompressionContext", label),
                cn(&mut c, LZ4F_VERSION),
                rn(&mut r, LZ4F_VERSION),
            );
        }
        assert!(!c.is_null() && !r.is_null(), "{}: NULL dctx", label);
        DctxPair { c, r }
    }
}

impl Drop for DctxPair {
    fn drop(&mut self) {
        let (cf, rf) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
        unsafe {
            let a = cf(self.c);
            let b = rf(self.r);
            assert_eq!(
                a, b,
                "LZ4F_freeDecompressionContext returned {} (C) vs {} (Rust)",
                a, b
            );
        }
    }
}

/// A recognisable pattern so "the out-parameter was not written" is itself
/// compared between the two libraries.
fn poisoned_fi() -> LZ4F_frameInfo_t {
    LZ4F_frameInfo_t {
        blockSizeID: 0x5A5A_5A5A,
        blockMode: 0x5A5A_5A5A,
        contentChecksumFlag: 0x5A5A_5A5A,
        frameType: 0x5A5A_5A5A,
        contentSize: 0x5A5A_5A5A_5A5A_5A5A,
        dictID: 0x5A5A_5A5A,
        blockChecksumFlag: 0x5A5A_5A5A,
    }
}

/// `LZ4F_getFrameInfo` on both libraries, comparing the return value, the
/// updated `*srcSizePtr` and the whole `LZ4F_frameInfo_t` out-parameter.
#[track_caller]
fn gfi_both(
    label: &str,
    d: &DctxPair,
    src: *const c_void,
    src_len: usize,
) -> (usize, usize, LZ4F_frameInfo_t) {
    let (cg, rg) = both::<FnGetFrameInfo>("LZ4F_getFrameInfo");
    let mut cfi = poisoned_fi();
    let mut rfi = poisoned_fi();
    let mut cs = src_len;
    let mut rs = src_len;
    unsafe {
        let a = cg(d.c, &mut cfi, src, &mut cs);
        let b = rg(d.r, &mut rfi, src, &mut rs);
        same_ret(&format!("{}: LZ4F_getFrameInfo return", label), a, b);
        assert_eq!(cs, rs, "{}: *srcSizePtr (C={} Rust={})", label, cs, rs);
        assert_eq!(
            cfi, rfi,
            "{}: LZ4F_frameInfo_t out-parameter\n  C   ={:?}\n  Rust={:?}",
            label, cfi, rfi
        );
        (a, cs, cfi)
    }
}

// ===========================================================================
// Rows 203-205 — LZ4F_headerSize input validation
// ===========================================================================

/// ERRORS.md row 203: `src == NULL` (lz4frame.c:1446) => `srcPtr_wrong`;
/// row 204: `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (5)
/// (lz4frame.c:1449-1450) => `frameHeader_incomplete`;
/// row 205: neither the frame magic nor a skippable magic
/// (lz4frame.c:1458-1459) => `frameType_unknown`.
/// (Also exercised by `header_size_real_headers_and_truncations` in
/// tests/lz4frame_decompress_diff.rs; this test pins each error code.)
#[test]
fn row_203_205_header_size_errors() {
    let (ch, rh) = both::<FnHeaderSize>("LZ4F_headerSize");
    let good = Hdr::new(LZ4F_max64KB).bytes();

    unsafe {
        // ---- row 203: NULL src is rejected before the size test, for ANY size.
        for &n in &[0usize, 1, 4, 5, 7, 19, usize::MAX] {
            let a = ch(ptr::null(), n);
            let b = rh(ptr::null(), n);
            expect_err(
                &format!("row 203: LZ4F_headerSize(NULL, {})", n),
                a,
                b,
                err::ERROR_srcPtr_wrong,
            );
        }

        // ---- row 204: srcSize 0..4 on a perfectly valid header.
        for n in 0..5usize {
            let a = ch(good.as_ptr() as *const c_void, n);
            let b = rh(good.as_ptr() as *const c_void, n);
            expect_err(
                &format!("row 204: LZ4F_headerSize(valid, {})", n),
                a,
                b,
                err::ERROR_frameHeader_incomplete,
            );
        }

        // ---- row 205: bad magic numbers (>= 5 bytes so row 204 does not fire).
        let bads: Vec<[u8; 5]> = vec![
            [0x00, 0x00, 0x00, 0x00, 0x40], // all zero
            [0x18, 0x4D, 0x22, 0x04, 0x40], // byte-swapped magic
            [0x03, 0x22, 0x4D, 0x18, 0x40], // magic - 1
            [0x05, 0x22, 0x4D, 0x18, 0x40], // magic + 1
            [0x4F, 0x2A, 0x4D, 0x18, 0x40], // just below the skippable range
            [0x60, 0x2A, 0x4D, 0x18, 0x40], // just above the skippable range
            [0xFF, 0xFF, 0xFF, 0xFF, 0x40],
        ];
        for (i, bad) in bads.iter().enumerate() {
            for &n in &[5usize, 6, 7, 19, 100] {
                let a = ch(bad.as_ptr() as *const c_void, n);
                let b = rh(bad.as_ptr() as *const c_void, n);
                expect_err(
                    &format!("row 205: bad magic #{} srcSize={}", i, n),
                    a,
                    b,
                    err::ERROR_frameType_unknown,
                );
            }
        }

        // ---- accept boundary: real headers of every length, and skippable frames.
        for &(cs, di, want) in &[
            (None, None, 7usize),
            (Some(1234u64), None, 15),
            (None, Some(7u32), 11),
            (Some(1234), Some(7), 19),
        ] {
            let mut h = Hdr::new(LZ4F_max64KB);
            h.content_size = cs;
            h.dict_id = di;
            let bytes = h.bytes();
            assert_eq!(bytes.len(), want, "test helper: header length");
            for n in 5..=bytes.len() {
                let a = ch(bytes.as_ptr() as *const c_void, n);
                let b = rh(bytes.as_ptr() as *const c_void, n);
                expect_ok(&format!("LZ4F_headerSize(real header, {})", n), a, b);
                assert_eq!(a, want, "LZ4F_headerSize value for a {}-byte header", want);
            }
        }
        for low in 0..16u32 {
            let magic = LZ4F_MAGIC_SKIPPABLE_START + low;
            let mut sk = Vec::new();
            sk.extend_from_slice(&le32(magic));
            sk.extend_from_slice(&le32(0)); // payload size
            let a = ch(sk.as_ptr() as *const c_void, sk.len());
            let b = rh(sk.as_ptr() as *const c_void, sk.len());
            expect_ok(&format!("skippable magic {:#x}", magic), a, b);
            assert_eq!(a, 8, "skippable frames report a header size of 8");
        }
    }
}

// ===========================================================================
// Rows 195 / 207 / 208 — LZ4F_getFrameInfo with an incomplete frame header
// ===========================================================================

/// ERRORS.md row 207: `LZ4F_headerSize()` fails inside `LZ4F_getFrameInfo`
/// (lz4frame.c:1503-1504) => that error verbatim, `*srcSizePtr` forced to 0;
/// row 208: `*srcSizePtr < hSize` (lz4frame.c:1505-1508) =>
/// `frameHeader_incomplete`, `*srcSizePtr` set to 0;
/// row 195: fewer than `minFHSize` (7) bytes of header available where
/// `LZ4F_decodeHeader()` is entered (lz4frame.c:1354).
///
/// NOTE on row 195: supplying a 5- or 6-byte buffer for a 7-byte header is
/// exactly the condition the row names, but `LZ4F_getFrameInfo` tests
/// `*srcSizePtr < hSize` one line EARLIER (lz4frame.c:1505) and returns the same
/// `frameHeader_incomplete` code, while `LZ4F_decompress` buffers the header up
/// to `minFHSize` before calling `LZ4F_decodeHeader` at all. The observable
/// result is therefore identical (code 12) on every public path, and that is
/// what is asserted here — through both entry points.
#[test]
fn row_195_207_208_get_frame_info_incomplete_header() {
    // ---- row 207 via a NULL src (LZ4F_headerSize -> srcPtr_wrong).
    for &n in &[0usize, 1, 5, 7, 19] {
        let d = DctxPair::new("row 207");
        let (ret, ss, _) = gfi_both(
            &format!("row 207: getFrameInfo(NULL src, {})", n),
            &d,
            ptr::null(),
            n,
        );
        assert!(lz4f_is_error(ret), "row 207: expected an error");
        assert_eq!(
            lz4f_error_code(ret),
            err::ERROR_srcPtr_wrong,
            "row 207: NULL src must forward srcPtr_wrong"
        );
        assert_eq!(ss, 0, "row 207: *srcSizePtr must be forced to 0");
    }

    // ---- row 207 via srcSize < 5 and via a bad magic.
    let good = Hdr::new(LZ4F_max64KB).bytes();
    for n in 0..5usize {
        let d = DctxPair::new("row 207b");
        let (ret, ss, _) = gfi_both(
            &format!("row 207: getFrameInfo(valid, {})", n),
            &d,
            good.as_ptr() as *const c_void,
            n,
        );
        assert_eq!(lz4f_error_code(ret), err::ERROR_frameHeader_incomplete);
        assert_eq!(ss, 0, "row 207: *srcSizePtr must be forced to 0");
    }
    let bad = [0x00u8, 0x00, 0x00, 0x00, 0x40, 0x70, 0x00];
    for &n in &[5usize, 6, 7] {
        let d = DctxPair::new("row 207c");
        let (ret, ss, _) = gfi_both(
            &format!("row 207: getFrameInfo(bad magic, {})", n),
            &d,
            bad.as_ptr() as *const c_void,
            n,
        );
        assert_eq!(lz4f_error_code(ret), err::ERROR_frameType_unknown);
        assert_eq!(ss, 0, "row 207: *srcSizePtr must be forced to 0");
    }

    // ---- rows 195 & 208: *srcSizePtr below the announced header size.
    for &(cs, di) in &[
        (None, None),
        (Some(1234u64), None),
        (None, Some(0xABCDu32)),
        (Some(1234), Some(0xABCD)),
    ] {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.content_size = cs;
        h.dict_id = di;
        let bytes = h.bytes();
        let hsize = bytes.len();
        for n in 5..hsize {
            let d = DctxPair::new("rows 195/208");
            let (ret, ss, fi) = gfi_both(
                &format!("rows 195/208: hSize={} but only {} bytes supplied", hsize, n),
                &d,
                bytes.as_ptr() as *const c_void,
                n,
            );
            assert_eq!(
                lz4f_error_code(ret),
                err::ERROR_frameHeader_incomplete,
                "rows 195/208: hSize={} n={}",
                hsize,
                n
            );
            assert_eq!(ss, 0, "rows 195/208: *srcSizePtr must be forced to 0");
            // The out-parameter is NOT written on this path: the C returns at
            // lz4frame.c:1506-1507, before `*frameInfoPtr = dctx->frameInfo`.
            assert_eq!(
                fi,
                poisoned_fi(),
                "rows 195/208: frameInfo must be left untouched"
            );
        }
        // Accept boundary: exactly hSize bytes decodes the header.
        let d = DctxPair::new("rows 195/208 boundary");
        let (ret, ss, _) = gfi_both(
            &format!("rows 195/208: exactly hSize={} bytes", hsize),
            &d,
            bytes.as_ptr() as *const c_void,
            hsize,
        );
        assert!(
            !lz4f_is_error(ret),
            "rows 195/208 boundary: hSize={} unexpectedly failed",
            hsize
        );
        assert_eq!(ss, hsize, "boundary: the whole header is consumed");
    }

    // ---- and the same truncations through LZ4F_decompress, which buffers the
    // header first (dstage_storeFrameHeader) and therefore simply asks for more.
    for n in 1..7usize {
        let cfg = DecCfg::new(4096);
        let label = format!("rows 195/208: LZ4F_decompress with only {} header bytes", n);
        let ret = dec_both(&label, &good[..n], &cfg);
        assert!(
            !lz4f_is_error(ret),
            "{}: the C asks for more input rather than failing (got {})",
            label,
            describe(ret)
        );
    }
}

// ===========================================================================
// Rows 196-202 + 209 — LZ4F_decodeHeader field validation
// ===========================================================================

/// A header whose FLG/BD are written verbatim; the optional fields (and hence the
/// header length reported by `LZ4F_headerSize`) follow the FLG bits, and the
/// header checksum is correct, so the ONLY defect is the injected bit pattern.
fn header_from_flg_bd(flg: u8, bd: u8) -> Vec<u8> {
    let cs = if flg & 0x08 != 0 {
        Some(0x1122_3344_5566_7788u64)
    } else {
        None
    };
    let di = if flg & 0x01 != 0 { Some(0xAABB_CCDDu32) } else { None };
    raw_header(flg, bd, cs, di)
}

fn zeroed_fi() -> LZ4F_frameInfo_t {
    LZ4F_frameInfo_t {
        blockSizeID: 0,
        blockMode: 0,
        contentChecksumFlag: 0,
        frameType: 0,
        contentSize: 0,
        dictID: 0,
        blockChecksumFlag: 0,
    }
}

/// ERRORS.md rows 196-202 (every validation inside `LZ4F_decodeHeader`) and row
/// 209 (a `LZ4F_decodeHeader` failure surfacing through `LZ4F_getFrameInfo`:
/// the error is returned verbatim, `*srcSizePtr` is forced to 0, and
/// `*frameInfoPtr` is still overwritten with the `MEM_INIT`-zeroed
/// `dctx->frameInfo`).
///
///   row 196 — bad magic (lz4frame.c:1358, 1372-1374)        => frameType_unknown (13)
///   row 197 — FLG reserved bit 1 set (lz4frame.c:1388)       => reservedFlag_set (8)
///   row 198 — FLG version field != 01 (lz4frame.c:1389)      => headerVersion_wrong (6)
///   row 199 — BD reserved bit 7 set (lz4frame.c:1409)        => reservedFlag_set (8)
///   row 200 — BD blockSizeID < 4 (lz4frame.c:1410)           => maxBlockSize_invalid (2)
///   row 201 — BD low nibble != 0 (lz4frame.c:1411)           => reservedFlag_set (8)
///   row 202 — header checksum mismatch (lz4frame.c:1417-18)  => headerChecksum_invalid (17)
#[test]
fn row_196_202_209_decode_header_validation() {
    // (row, FLG, BD, expected code). The order of the C's tests matters: the FLG
    // reserved bit is checked before the version, and within BD the reserved bit 7
    // comes before blockSizeID, which comes before the low nibble.
    let mut cases: Vec<(&str, u8, u8, i32)> = Vec::new();

    // row 197: FLG bit 1 set. Version bits deliberately valid AND invalid, to
    // prove the reserved-bit test wins.
    for &flg in &[0x42u8, 0x43, 0x4A, 0x4B, 0x02, 0x82, 0xC2, 0x7E, 0xFF] {
        cases.push(("row 197", flg, 0x70, err::ERROR_reservedFlag_set));
    }
    // row 198: version field != 01, with bit 1 clear.
    for &flg in &[0x00u8, 0x08, 0x09, 0x20, 0x3D, 0x80, 0x84, 0xB0, 0xC0, 0xFD] {
        cases.push(("row 198", flg, 0x70, err::ERROR_headerVersion_wrong));
    }
    // row 199: BD bit 7 set (blockSizeID field kept legal so it cannot be blamed).
    for &bd in &[0x80u8, 0xC0, 0xD0, 0xE0, 0xF0, 0xFF, 0x90] {
        cases.push(("row 199", 0x40, bd, err::ERROR_reservedFlag_set));
    }
    // row 200: blockSizeID < 4 with bit 7 clear.
    for &bd in &[0x00u8, 0x10, 0x20, 0x30] {
        cases.push(("row 200", 0x40, bd, err::ERROR_maxBlockSize_invalid));
    }
    // row 201: legal blockSizeID, bit 7 clear, but a non-zero low nibble.
    for &bd in &[0x41u8, 0x4F, 0x51, 0x6A, 0x71, 0x7F] {
        cases.push(("row 201", 0x40, bd, err::ERROR_reservedFlag_set));
    }
    // A few valid-FLG variants (contentSize / dictID present) crossed with a bad BD,
    // so the longer header layouts also reach the BD tests.
    for &flg in &[0x48u8, 0x41, 0x49, 0x4C, 0x64, 0x70] {
        cases.push(("rows 199-201", flg, 0x00, err::ERROR_maxBlockSize_invalid));
        cases.push(("rows 199-201", flg, 0x81, err::ERROR_reservedFlag_set));
        cases.push(("rows 199-201", flg, 0x72, err::ERROR_reservedFlag_set));
    }

    for (row, flg, bd, want) in cases {
        let h = header_from_flg_bd(flg, bd);
        let label = format!("{}: FLG={:#04x} BD={:#04x}", row, flg, bd);

        // ---- through LZ4F_getFrameInfo (row 209 semantics)
        {
            let d = DctxPair::new(&label);
            let (ret, ss, fi) =
                gfi_both(&format!("{} via getFrameInfo", label), &d, h.as_ptr() as *const c_void, h.len());
            assert!(lz4f_is_error(ret), "{}: expected an error", label);
            assert_eq!(
                lz4f_error_code(ret),
                want,
                "{}: wrong error code via LZ4F_getFrameInfo",
                label
            );
            // row 209: *srcSizePtr forced to 0, *frameInfoPtr overwritten with the
            // zeroed dctx->frameInfo (lz4frame.c:1510-1517).
            assert_eq!(ss, 0, "{}: *srcSizePtr must be 0", label);
            assert_eq!(
                fi,
                zeroed_fi(),
                "{}: row 209 requires *frameInfoPtr to be the zeroed frameInfo",
                label
            );
        }

        // ---- through LZ4F_decompress, both the "store the header first" path
        // (input < 19 bytes) and the shortcut path (>= 19 bytes).
        for extra in [0usize, 20] {
            let mut f = h.clone();
            f.extend(std::iter::repeat(0xCD).take(extra));
            for &dst_cap in &[0usize, 64, 70_000] {
                let cfg = DecCfg::new(dst_cap);
                dec_expect_err(
                    &format!("{} via decompress (+{} filler, dstCap={})", label, extra, dst_cap),
                    &f,
                    &cfg,
                    want,
                );
            }
        }
    }

    // ---- row 196: a bad magic number, through LZ4F_decompress (LZ4F_decodeHeader
    // is where the magic is validated; via LZ4F_getFrameInfo the earlier
    // LZ4F_headerSize call reports the same code, which row 207 covers).
    for &magic in &[
        0x0000_0000u32,
        0x0422_4D18,
        LZ4F_MAGICNUMBER - 1,
        LZ4F_MAGICNUMBER + 1,
        LZ4F_MAGIC_SKIPPABLE_START - 1,
        LZ4F_MAGIC_SKIPPABLE_START + 16,
        0xFFFF_FFFF,
    ] {
        let mut f = Vec::new();
        f.extend_from_slice(&le32(magic));
        f.extend_from_slice(&[0x40, 0x70, 0x00]);
        f.extend(std::iter::repeat(0xCDu8).take(24));
        for &dst_cap in &[0usize, 4096] {
            dec_expect_err(
                &format!("row 196: magic={:#010x} dstCap={}", magic, dst_cap),
                &f,
                &DecCfg::new(dst_cap),
                err::ERROR_frameType_unknown,
            );
        }
    }

    // ---- row 202: a valid header with a corrupted checksum byte.
    for &(cs, di) in &[
        (None, None),
        (Some(4096u64), None),
        (None, Some(0x1234u32)),
        (Some(4096), Some(0x1234)),
    ] {
        for &bsid in &[LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
            let mut hh = Hdr::new(bsid);
            hh.content_size = cs;
            hh.dict_id = di;
            let good = hh.bytes();
            let last = good.len() - 1;
            for delta in [1u8, 0x7F, 0x80, 0xFF] {
                let mut h = good.clone();
                h[last] = h[last].wrapping_add(delta);
                if h[last] == good[last] {
                    continue;
                }
                let label = format!(
                    "row 202: bsid={} cs={:?} di={:?} HC {:#04x}->{:#04x}",
                    bsid, cs, di, good[last], h[last]
                );
                let d = DctxPair::new(&label);
                let (ret, ss, fi) =
                    gfi_both(&label, &d, h.as_ptr() as *const c_void, h.len());
                assert_eq!(
                    lz4f_error_code(ret),
                    err::ERROR_headerChecksum_invalid,
                    "{}: wrong error code",
                    label
                );
                assert_eq!(ss, 0, "{}: *srcSizePtr must be 0", label);
                assert_eq!(fi, zeroed_fi(), "{}: row 209 frameInfo", label);

                let mut f = h.clone();
                f.extend(std::iter::repeat(0xCDu8).take(24));
                dec_expect_err(
                    &format!("{} via decompress", label),
                    &f,
                    &DecCfg::new(4096),
                    err::ERROR_headerChecksum_invalid,
                );
            }
        }
    }
}

/// One `LZ4F_decompress` call on both libraries of a `DctxPair`, comparing the
/// return value, `*srcSizePtr`, `*dstSizePtr` and the FULL destination buffer.
#[track_caller]
fn dec_step(label: &str, d: &DctxPair, src: &[u8], dst_cap: usize) -> (usize, usize, usize) {
    let (cdec, rdec) = both::<FnDecompress>("LZ4F_decompress");
    let mut cbuf = vec![SENTINEL; dst_cap.max(1)];
    let mut rbuf = vec![SENTINEL; dst_cap.max(1)];
    let mut cds = dst_cap;
    let mut rds = dst_cap;
    let mut css = src.len();
    let mut rss = src.len();
    unsafe {
        let a = cdec(
            d.c,
            cbuf.as_mut_ptr() as *mut c_void,
            &mut cds,
            src.as_ptr() as *const c_void,
            &mut css,
            ptr::null(),
        );
        let b = rdec(
            d.r,
            rbuf.as_mut_ptr() as *mut c_void,
            &mut rds,
            src.as_ptr() as *const c_void,
            &mut rss,
            ptr::null(),
        );
        same_ret(&format!("{}: LZ4F_decompress return", label), a, b);
        assert_eq!(css, rss, "{}: *srcSizePtr (C={} Rust={})", label, css, rss);
        assert_eq!(cds, rds, "{}: *dstSizePtr (C={} Rust={})", label, cds, rds);
        assert_bytes_eq(&format!("{}: FULL dst buffer", label), &cbuf, &rbuf);
        (a, css, cds)
    }
}

// ===========================================================================
// Row 206 — LZ4F_getFrameInfo while the frame header is only partially stored
// ===========================================================================

/// ERRORS.md row 206: `dctx->dStage == dstage_storeFrameHeader` (1), i.e.
/// `LZ4F_decompress()` was first fed 1..6 bytes, and then `LZ4F_getFrameInfo()`
/// is called (lz4frame.c:1498-1501) => `frameDecoding_alreadyStarted` (19), with
/// `*srcSizePtr` set to 0.
/// (Also touched by `get_frame_info_before_decompress` in
/// tests/lz4frame_decompress_diff.rs.)
#[test]
fn row_206_get_frame_info_partial_header() {
    let mut rng = Rng::new(0x0206_0000_0000_0001);
    let data = gen_shape(&mut rng, 3, 4096);

    for &(cs, di) in &[(None, None), (Some(4096u64), Some(9u32))] {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.content_size = cs;
        h.dict_id = di;
        let frame = frame_1block(&h, &data);

        // 1..6 bytes leave dStage == dstage_storeFrameHeader (the C only calls
        // LZ4F_decodeHeader once minFHSize == 7 bytes are buffered).
        for n in 1..7usize {
            let d = DctxPair::new("row 206");
            let (ret, consumed, produced) = dec_step(
                &format!("row 206: prime the dctx with {} bytes", n),
                &d,
                &frame[..n],
                4096,
            );
            assert!(!lz4f_is_error(ret), "row 206: priming must not fail");
            assert_eq!(consumed, n, "row 206: the partial header is buffered");
            assert_eq!(produced, 0, "row 206: nothing can be produced yet");

            for &avail in &[0usize, 5, 7, frame.len()] {
                let (r2, ss, fi) = gfi_both(
                    &format!("row 206: getFrameInfo after {} header bytes, avail={}", n, avail),
                    &d,
                    frame.as_ptr() as *const c_void,
                    avail,
                );
                assert_eq!(
                    lz4f_error_code(r2),
                    err::ERROR_frameDecoding_alreadyStarted,
                    "row 206: n={} avail={}",
                    n,
                    avail
                );
                assert_eq!(ss, 0, "row 206: *srcSizePtr must be 0");
                assert_eq!(
                    fi,
                    poisoned_fi(),
                    "row 206: *frameInfoPtr is not written on this path"
                );
            }
        }
    }
}

// ===========================================================================
// Row 210 — LZ4F_getFrameInfo after the header has been decoded
// ===========================================================================

/// ERRORS.md row 210: `dctx->dStage > dstage_storeFrameHeader`, so
/// `LZ4F_getFrameInfo` fills `*frameInfoPtr`, forces `*srcSizePtr = 0` and
/// tail-calls `LZ4F_decompress(dctx, NULL, &0, NULL, &0, NULL)`
/// (lz4frame.c:1490-1496) — whatever that returns is returned, notably
/// `frameSize_wrong` when the context is parked in `dstage_getSuffix` with a
/// non-zero `frameRemainingSize`.
/// (`get_frame_info_midstream` in tests/lz4frame_decompress_diff.rs covers the
/// success shapes; this test pins the error-returning shape.)
#[test]
fn row_210_get_frame_info_after_header_decoded() {
    let mut rng = Rng::new(0x0210_0000_0000_0001);
    let data = gen_shape(&mut rng, 5, 5000);

    // ---- (a) parked in dstage_getSuffix with frameRemainingSize != 0.
    for &(declared, cck) in &[
        (5000u64 + 7, false),
        (5000 - 7, false),
        (5000 + 7, true),
        (1, false),
    ] {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.content_size = Some(declared);
        h.content_ck = cck;
        let frame = frame_1block(&h, &data);
        let label = format!("row 210: declared={} actual={} cck={}", declared, data.len(), cck);

        let d = DctxPair::new(&label);
        let (ret, _, _) = dec_step(&format!("{}: feed the frame", label), &d, &frame, 70_000);
        assert_eq!(
            lz4f_error_code(ret),
            err::ERROR_frameSize_wrong,
            "{}: the initial decompress must already report frameSize_wrong",
            label
        );

        // Now the row-210 call: the SAME error comes back out of getFrameInfo.
        let (r2, ss, fi) = gfi_both(&format!("{}: getFrameInfo", label), &d, ptr::null(), 0);
        assert_eq!(
            lz4f_error_code(r2),
            err::ERROR_frameSize_wrong,
            "{}: getFrameInfo must forward the decompress error",
            label
        );
        assert_eq!(ss, 0, "{}: *srcSizePtr must be 0", label);
        assert_ne!(
            fi,
            poisoned_fi(),
            "{}: *frameInfoPtr is filled BEFORE the tail call",
            label
        );
        assert_eq!(fi.contentSize, declared, "{}: decoded contentSize", label);
    }

    // ---- (b) parked in dstage_getBlockHeader (a healthy mid-frame state): the
    // tail call returns a size hint, not an error, and both libraries agree.
    {
        let h = Hdr::new(LZ4F_max64KB);
        let frame = frame_1block(&h, &data);
        let stop = frame.len() - 4; // everything except the endMark
        let d = DctxPair::new("row 210b");
        let (ret, consumed, _) =
            dec_step("row 210b: feed everything but the endMark", &d, &frame[..stop], 70_000);
        assert!(!lz4f_is_error(ret), "row 210b: feeding must not fail");
        assert_eq!(consumed, stop, "row 210b: all of it should be consumed");
        let (r2, ss, fi) = gfi_both("row 210b: getFrameInfo", &d, ptr::null(), 0);
        assert!(
            !lz4f_is_error(r2),
            "row 210b: expected a size hint, got ERROR({})",
            lz4f_error_code(r2)
        );
        assert_eq!(ss, 0, "row 210b: *srcSizePtr must be 0");
        assert_eq!(fi.blockSizeID, LZ4F_max64KB, "row 210b: decoded blockSizeID");
    }
}

// ===========================================================================
// Rows 211-212 — dstage_init buffer allocation failures
// ===========================================================================

/// ERRORS.md row 211: `dctx->tmpIn = LZ4F_malloc(maxBlockSize + BFSize)` fails
/// (lz4frame.c:1685-1686) and row 212: `dctx->tmpOutBuffer =
/// LZ4F_malloc(maxBlockSize + (blockLinked ? 128 KB : 0))` fails
/// (lz4frame.c:1687-1689) => `allocation_failed`.
///
/// Forced through `LZ4F_createDecompressionContext_advanced` with an
/// `LZ4F_CustomMem` that fails on the Nth call. Allocation order: #1 the
/// `LZ4F_dctx` itself, #2 `tmpIn`, #3 `tmpOutBuffer`.
#[test]
fn row_211_212_decompress_allocation_failures() {
    let (cda, rda) = both::<FnCreateDctxAdv>("LZ4F_createDecompressionContext_advanced");
    let (cfd, rfd) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cdec, rdec) = both::<FnDecompress>("LZ4F_decompress");

    let mut rng = Rng::new(0x0211_0000_0000_0001);
    let data = gen_shape(&mut rng, 4, 4096);

    for with_calloc in [false, true] {
        // fail_at == 2 => row 211 (tmpIn) ; fail_at == 3 => row 212 (tmpOutBuffer)
        for fail_at in [2u64, 3] {
            for &bsid in &[LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
                for &independent in &[true, false] {
                    for &(cck, bck) in &[(false, false), (true, true)] {
                        let mut h = Hdr::new(bsid);
                        h.independent = independent;
                        h.content_ck = cck;
                        h.block_ck = bck;
                        let frame = frame_1block(&h, &data);
                        let label = format!(
                            "rows 211/212: fail_at={} calloc={} bsid={} indep={} cck={} bck={}",
                            fail_at, with_calloc, bsid, independent, cck, bck
                        );

                        let mut cst = AllocState::new(fail_at);
                        let mut rst = AllocState::new(fail_at);
                        let mut cbuf = vec![SENTINEL; 8192];
                        let mut rbuf = vec![SENTINEL; 8192];
                        unsafe {
                            let cd = cda(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                            let rd = rda(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                            assert!(!cd.is_null() && !rd.is_null(), "{}: dctx create", label);

                            let mut cds = cbuf.len();
                            let mut rds = rbuf.len();
                            let mut css = frame.len();
                            let mut rss = frame.len();
                            let a = cdec(
                                cd,
                                cbuf.as_mut_ptr() as *mut c_void,
                                &mut cds,
                                frame.as_ptr() as *const c_void,
                                &mut css,
                                ptr::null(),
                            );
                            let b = rdec(
                                rd,
                                rbuf.as_mut_ptr() as *mut c_void,
                                &mut rds,
                                frame.as_ptr() as *const c_void,
                                &mut rss,
                                ptr::null(),
                            );
                            expect_err(&label, a, b, err::ERROR_allocation_failed);
                            assert_eq!(css, rss, "{}: *srcSizePtr", label);
                            assert_eq!(cds, rds, "{}: *dstSizePtr", label);
                            assert_bytes_eq(&format!("{}: dst", label), &cbuf, &rbuf);
                            same_ret(&format!("{}: free", label), cfd(cd), rfd(rd));
                        }
                        assert_eq!(
                            cst.calls, fail_at,
                            "{}: expected to stop at allocation #{}",
                            label, fail_at
                        );
                        assert_no_leak(&label, &cst, &rst);
                    }
                }
            }
        }

        // Contrast: no injected failure => the frame decodes through the custom
        // allocators and everything is released.
        {
            let mut cst = AllocState::new(0);
            let mut rst = AllocState::new(0);
            let h = Hdr::new(LZ4F_max64KB);
            let frame = frame_1block(&h, &data);
            let mut cbuf = vec![SENTINEL; 8192];
            let mut rbuf = vec![SENTINEL; 8192];
            unsafe {
                let cd = cda(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let rd = rda(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                let mut cds = cbuf.len();
                let mut rds = rbuf.len();
                let mut css = frame.len();
                let mut rss = frame.len();
                let a = cdec(
                    cd,
                    cbuf.as_mut_ptr() as *mut c_void,
                    &mut cds,
                    frame.as_ptr() as *const c_void,
                    &mut css,
                    ptr::null(),
                );
                let b = rdec(
                    rd,
                    rbuf.as_mut_ptr() as *mut c_void,
                    &mut rds,
                    frame.as_ptr() as *const c_void,
                    &mut rss,
                    ptr::null(),
                );
                expect_ok("rows 211/212 success path", a, b);
                assert_eq!(cds, data.len(), "rows 211/212: regenerated size");
                assert_bytes_eq("rows 211/212: regenerated data", &cbuf[..cds], &data);
                assert_bytes_eq("rows 211/212: FULL dst", &cbuf, &rbuf);
                same_ret("rows 211/212: free", cfd(cd), rfd(rd));
            }
            assert_eq!(cst.calls, 3, "rows 211/212: expected 3 allocations");
            assert_no_leak("rows 211/212 success path", &cst, &rst);
        }
    }
}

// ===========================================================================
// Row 213 — a block header larger than the frame's maxBlockSize
// ===========================================================================

/// ERRORS.md row 213: `(blockHeader & 0x7FFFFFFF) > dctx->maxBlockSize`
/// (lz4frame.c:1737-1739) => `maxBlockSize_invalid`, for both compressed and
/// uncompressed (`LZ4F_BLOCKUNCOMPRESSED_FLAG`) blocks.
#[test]
fn row_213_block_header_too_large() {
    for &bsid in &[LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
        let bs = block_size_of(bsid);
        for &bck in &[false, true] {
            let mut h = Hdr::new(bsid);
            h.block_ck = bck;
            let hdr = h.bytes();

            for &size in &[
                bs as u32 + 1,
                bs as u32 + 2,
                bs as u32 * 2,
                0x7FFF_FFFF,
                0x7FFF_FFFE,
            ] {
                for &uncompressed in &[false, true] {
                    let raw = if uncompressed { size | 0x8000_0000 } else { size };
                    // Feed the header + just the 4-byte block header, and also a
                    // padded version so the >= 19-byte header shortcut is used.
                    for &pad in &[0usize, 24] {
                        let mut f = hdr.clone();
                        f.extend_from_slice(&le32(raw));
                        f.extend(std::iter::repeat(0xE7u8).take(pad));
                        for &dst_cap in &[0usize, 64, bs + 16] {
                            dec_expect_err(
                                &format!(
                                    "row 213: bsid={} bck={} size={} uncompressed={} pad={} dstCap={}",
                                    bsid, bck, size, uncompressed, pad, dst_cap
                                ),
                                &f,
                                &DecCfg::new(dst_cap),
                                err::ERROR_maxBlockSize_invalid,
                            );
                        }
                    }
                }
            }

            // Accept boundary: exactly maxBlockSize is allowed through (the block
            // itself is then truncated, which is a different, later error).
            let mut f = hdr.clone();
            f.extend_from_slice(&le32(bs as u32));
            let label = format!("row 213 boundary: bsid={} bck={}", bsid, bck);
            let ret = dec_both(&label, &f, &DecCfg::new(bs + 16));
            assert!(
                !lz4f_is_error(ret),
                "{}: exactly maxBlockSize must be accepted, got ERROR({})",
                label,
                lz4f_error_code(ret)
            );
        }
    }
}

// ===========================================================================
// Rows 214-215 — block checksum verification
// ===========================================================================

/// ERRORS.md row 214: an **uncompressed** block (bit 31 set) in a frame with
/// `blockChecksumFlag == 1` whose trailing checksum does not match
/// `XXH32(blockData, 0)` (lz4frame.c:1821-1830) => `blockChecksum_invalid`; and
/// row 215: the same for a **compressed** block, checked in `dstage_getCBlock`
/// (lz4frame.c:1871-1878).
///
/// The two paths differ in one important way: row 214 lives behind
/// `if (!dctx->skipChecksum)` and is therefore skipped when
/// `decompressOptions.skipChecksums != 0`, while row 215 is **not** gated and
/// fires regardless. Both halves are asserted.
#[test]
fn row_214_215_block_checksum_invalid() {
    let mut rng = Rng::new(0x0214_0000_0000_0001);
    let data = gen_shape(&mut rng, 1, 4096); // compressible, so a real cSize < 4096

    let skip = LZ4F_decompressOptions_t {
        stableDst: 0,
        skipChecksums: 1,
        reserved1: 0,
        reserved0: 0,
    };

    for &cck in &[false, true] {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.block_ck = true;
        h.content_ck = cck;
        let hdr = h.bytes();

        // ---------------- row 214: uncompressed block
        {
            let blk = uncompressed_block(&data, true);
            let mut good = hdr.clone();
            good.extend_from_slice(&blk);
            good.extend_from_slice(&frame_tail(&data, cck));
            // sanity: the untouched frame decodes
            let ret = dec_both(
                &format!("row 214: valid uncompressed frame (cck={})", cck),
                &good,
                &DecCfg::new(70_000),
            );
            assert_eq!(ret, 0, "row 214: the reference frame must decode");

            let crc_off = hdr.len() + LZ4F_BLOCK_HEADER_SIZE + data.len();
            for bit in [0usize, 7, 8, 31] {
                let mut f = good.clone();
                f[crc_off + bit / 8] ^= 1u8 << (bit % 8);
                for &dst_cap in &[100usize, 4096, 70_000] {
                    dec_expect_err(
                        &format!(
                            "row 214: uncompressed block checksum bit {} flipped (cck={}, dstCap={})",
                            bit, cck, dst_cap
                        ),
                        &f,
                        &DecCfg::new(dst_cap),
                        err::ERROR_blockChecksum_invalid,
                    );
                }
                // ... and skipChecksums == 1 makes the C accept it (the content
                // checksum is skipped too, so the frame completes).
                let mut cfg = DecCfg::new(70_000);
                cfg.opts = Some(skip);
                let ret = dec_both(
                    &format!("row 214: bit {} flipped WITH skipChecksums (cck={})", bit, cck),
                    &f,
                    &cfg,
                );
                assert!(
                    !lz4f_is_error(ret),
                    "row 214: skipChecksums must suppress the uncompressed-block check, got ERROR({})",
                    lz4f_error_code(ret)
                );
            }
        }

        // ---------------- row 215: compressed block
        {
            let comp = lz4_block(&data);
            let blk = compressed_block(&data, true);
            let mut good = hdr.clone();
            good.extend_from_slice(&blk);
            good.extend_from_slice(&frame_tail(&data, cck));
            let ret = dec_both(
                &format!("row 215: valid compressed frame (cck={})", cck),
                &good,
                &DecCfg::new(70_000),
            );
            assert_eq!(ret, 0, "row 215: the reference frame must decode");

            let crc_off = hdr.len() + LZ4F_BLOCK_HEADER_SIZE + comp.len();
            for bit in [0usize, 7, 8, 31] {
                let mut f = good.clone();
                f[crc_off + bit / 8] ^= 1u8 << (bit % 8);
                for &dst_cap in &[100usize, 4096, 70_000] {
                    dec_expect_err(
                        &format!(
                            "row 215: compressed block checksum bit {} flipped (cck={}, dstCap={})",
                            bit, cck, dst_cap
                        ),
                        &f,
                        &DecCfg::new(dst_cap),
                        err::ERROR_blockChecksum_invalid,
                    );
                }
                // row 215 is NOT gated by skipChecksum (lz4frame.c:1871-1878).
                let mut cfg = DecCfg::new(70_000);
                cfg.opts = Some(skip);
                dec_expect_err(
                    &format!(
                        "row 215: bit {} flipped WITH skipChecksums (cck={}) must STILL fail",
                        bit, cck
                    ),
                    &f,
                    &cfg,
                    err::ERROR_blockChecksum_invalid,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 216-217 — a corrupt LZ4 block payload
// ===========================================================================

/// ERRORS.md row 216: `LZ4_decompress_safe_usingDict() < 0` while decoding
/// **directly into `dstBuffer`** — taken when `(dstEnd-dstPtr) >= maxBlockSize`
/// (lz4frame.c:1901-1905); row 217: the same failure while decoding **into
/// `dctx->tmpOut`** — taken when dst has less than `maxBlockSize` of room
/// (lz4frame.c:1946-1950). Both => `decompressionFailed`.
///
/// The two rows are selected purely by the destination capacity, so each payload
/// is driven with `dstCapacity >= 64 KB` (row 216) and with a handful of small
/// capacities (row 217).
#[test]
fn row_216_217_corrupt_block_payload() {
    // Payloads that `LZ4_decompress_safe` rejects for ANY output bound.
    let payloads: Vec<(&str, Vec<u8>)> = vec![
        ("0xFF x40", vec![0xFFu8; 40]),
        (
            "huge literal-length extension",
            vec![0xF0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
        ),
        (
            "huge match-length extension",
            vec![0x0F, 0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
        ),
        ("offset 65535 with no history", vec![0x40, 1, 2, 3, 4, 0xFF, 0xFF]),
        ("offset 0", vec![0x40, 1, 2, 3, 4, 0x00, 0x00]),
    ];

    for &bck in &[false, true] {
        for &cck in &[false, true] {
            for &independent in &[true, false] {
                let mut h = Hdr::new(LZ4F_max64KB);
                h.block_ck = bck;
                h.content_ck = cck;
                h.independent = independent;
                let hdr = h.bytes();

                for (name, p) in &payloads {
                    let mut f = hdr.clone();
                    f.extend_from_slice(&le32(p.len() as u32));
                    f.extend_from_slice(p);
                    if bck {
                        f.extend_from_slice(&le32(xxh32(p)));
                    }
                    f.extend_from_slice(&le32(0)); // endMark
                    if cck {
                        f.extend_from_slice(&le32(0)); // never reached
                    }

                    // row 216: dst has at least maxBlockSize of room
                    for &dst_cap in &[65_536usize, 70_000, 200_000] {
                        dec_expect_err(
                            &format!(
                                "row 216: [{}] bck={} cck={} indep={} dstCap={}",
                                name, bck, cck, independent, dst_cap
                            ),
                            &f,
                            &DecCfg::new(dst_cap),
                            err::ERROR_decompressionFailed,
                        );
                    }
                    // row 217: dst is smaller than maxBlockSize -> decode into tmpOut
                    for &dst_cap in &[1usize, 16, 100, 4096, 65_535] {
                        dec_expect_err(
                            &format!(
                                "row 217: [{}] bck={} cck={} indep={} dstCap={}",
                                name, bck, cck, independent, dst_cap
                            ),
                            &f,
                            &DecCfg::new(dst_cap),
                            err::ERROR_decompressionFailed,
                        );
                    }
                    // and with the source dribbled in a few bytes at a time
                    for &chunk in &[1usize, 3, 7] {
                        let mut cfg = DecCfg::new(200);
                        cfg.src_chunk = chunk;
                        dec_expect_err(
                            &format!(
                                "row 217: [{}] bck={} cck={} indep={} srcChunk={}",
                                name, bck, cck, independent, chunk
                            ),
                            &f,
                            &cfg,
                            err::ERROR_decompressionFailed,
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 218 — dstage_getSuffix with a non-zero frameRemainingSize
// ===========================================================================

/// ERRORS.md row 218: the endMark is reached while `dctx->frameRemainingSize != 0`
/// (lz4frame.c:1984) => `frameSize_wrong`. Both directions trigger it: a declared
/// `contentSize` LARGER than the regenerated data (positive remainder) and one
/// SMALLER than it (the `U64` remainder underflows to a huge value).
#[test]
fn row_218_frame_size_wrong() {
    let mut rng = Rng::new(0x0218_0000_0000_0001);

    for &actual in &[1usize, 100, 5000, 65_536] {
        let data = gen_shape(&mut rng, actual % N_SHAPES, actual);
        for &cck in &[false, true] {
            for &bck in &[false, true] {
                for &declared in &[
                    1u64,
                    actual as u64 - 1,
                    actual as u64 + 1,
                    actual as u64 * 2,
                    u64::MAX,
                ] {
                    if declared == actual as u64 || declared == 0 {
                        continue;
                    }
                    let mut h = Hdr::new(LZ4F_max64KB);
                    h.content_size = Some(declared);
                    h.content_ck = cck;
                    h.block_ck = bck;
                    let frame = frame_1block(&h, &data);
                    for &dst_cap in &[100usize, 70_000, 200_000] {
                        dec_expect_err(
                            &format!(
                                "row 218: declared={} actual={} cck={} bck={} dstCap={}",
                                declared, actual, cck, bck, dst_cap
                            ),
                            &frame,
                            &DecCfg::new(dst_cap),
                            err::ERROR_frameSize_wrong,
                        );
                    }
                }

                // Accept boundary: the declared size matches, so the frame decodes.
                let mut h = Hdr::new(LZ4F_max64KB);
                h.content_size = Some(actual as u64);
                h.content_ck = cck;
                h.block_ck = bck;
                let frame = frame_1block(&h, &data);
                let label = format!(
                    "row 218 boundary: declared==actual=={} cck={} bck={}",
                    actual, cck, bck
                );
                let ret = dec_both(&label, &frame, &DecCfg::new(200_000));
                assert_eq!(ret, 0, "{}: the matching frame must decode", label);
            }
        }
    }
}

// ===========================================================================
// Row 219 — content checksum mismatch
// ===========================================================================

/// ERRORS.md row 219: the trailing 4-byte content checksum differs from
/// `XXH32_digest(&dctx->xxh)` (lz4frame.c:2016-2021) =>
/// `contentChecksum_invalid`; skipped when
/// `decompressOptions.skipChecksums != 0`.
#[test]
fn row_219_content_checksum_invalid() {
    let mut rng = Rng::new(0x0219_0000_0000_0001);
    let skip = LZ4F_decompressOptions_t {
        stableDst: 0,
        skipChecksums: 1,
        reserved1: 0,
        reserved0: 0,
    };

    for &n in &[1usize, 100, 5000] {
        let data = gen_shape(&mut rng, n % N_SHAPES, n);
        for &bck in &[false, true] {
            for &declared in &[false, true] {
                let mut h = Hdr::new(LZ4F_max64KB);
                h.content_ck = true;
                h.block_ck = bck;
                h.content_size = if declared { Some(n as u64) } else { None };
                let good = frame_1block(&h, &data);
                let ret = dec_both(
                    &format!("row 219: reference frame n={} bck={} cs={}", n, bck, declared),
                    &good,
                    &DecCfg::new(200_000),
                );
                assert_eq!(ret, 0, "row 219: the reference frame must decode");

                let off = good.len() - 4;
                for bit in [0usize, 7, 15, 31] {
                    let mut f = good.clone();
                    f[off + bit / 8] ^= 1u8 << (bit % 8);
                    for &(dst_cap, src_chunk) in
                        &[(200_000usize, 0usize), (100, 0), (100, 3), (200_000, 1)]
                    {
                        let mut cfg = DecCfg::new(dst_cap);
                        cfg.src_chunk = src_chunk;
                        dec_expect_err(
                            &format!(
                                "row 219: n={} bck={} cs={} bit={} dstCap={} srcChunk={}",
                                n, bck, declared, bit, dst_cap, src_chunk
                            ),
                            &f,
                            &cfg,
                            err::ERROR_contentChecksum_invalid,
                        );
                    }
                    // skipChecksums == 1 suppresses it
                    let mut cfg = DecCfg::new(200_000);
                    cfg.opts = Some(skip);
                    let ret = dec_both(
                        &format!("row 219: bit={} WITH skipChecksums", bit),
                        &f,
                        &cfg,
                    );
                    assert!(
                        !lz4f_is_error(ret),
                        "row 219: skipChecksums must suppress the content checksum, got ERROR({})",
                        lz4f_error_code(ret)
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 220 — dstBuffer == NULL with *dstSizePtr != 0
// ===========================================================================

/// ERRORS.md row 220: `dstBuffer == NULL` while `*dstSizePtr != 0`. The
/// `assert(*dstSizePtr == 0)` at lz4frame.c:1632 is compiled out in this build
/// (lz4frame.c:143-149 defines `assert` as `((void)0)` because `LZ4_DEBUG` is not
/// defined), so the documented release behaviour applies: NO error is returned,
/// `dstEnd` is NULL so every copy stage sees a capacity of 0, and the frame makes
/// no output progress.
#[test]
fn row_220_null_dst_with_nonzero_size() {
    let (cdec, rdec) = both::<FnDecompress>("LZ4F_decompress");
    let mut rng = Rng::new(0x0220_0000_0000_0001);
    let data = gen_shape(&mut rng, 4, 5000);

    for &cck in &[false, true] {
        for &bck in &[false, true] {
            for &uncompressed in &[false, true] {
                let mut h = Hdr::new(LZ4F_max64KB);
                h.content_ck = cck;
                h.block_ck = bck;
                let mut frame = h.bytes();
                if uncompressed {
                    frame.extend_from_slice(&uncompressed_block(&data, bck));
                } else {
                    frame.extend_from_slice(&compressed_block(&data, bck));
                }
                frame.extend_from_slice(&frame_tail(&data, cck));

                for &claim in &[1usize, 100, 65_536, 200_000] {
                    let d = DctxPair::new("row 220");
                    let mut sp = 0usize;
                    for call in 0..8 {
                        let mut cds = claim;
                        let mut rds = claim;
                        let mut css = frame.len() - sp;
                        let mut rss = css;
                        let label = format!(
                            "row 220: cck={} bck={} unc={} claim={} call#{}",
                            cck, bck, uncompressed, claim, call
                        );
                        unsafe {
                            let a = cdec(
                                d.c,
                                ptr::null_mut(),
                                &mut cds,
                                frame.as_ptr().add(sp) as *const c_void,
                                &mut css,
                                ptr::null(),
                            );
                            let b = rdec(
                                d.r,
                                ptr::null_mut(),
                                &mut rds,
                                frame.as_ptr().add(sp) as *const c_void,
                                &mut rss,
                                ptr::null(),
                            );
                            same_ret(&label, a, b);
                            assert_eq!(css, rss, "{}: *srcSizePtr", label);
                            assert_eq!(cds, rds, "{}: *dstSizePtr", label);
                            assert!(
                                !lz4f_is_error(a),
                                "{}: no error is expected in a release build, got ERROR({})",
                                label,
                                lz4f_error_code(a)
                            );
                            assert_eq!(
                                cds, 0,
                                "{}: a NULL dstBuffer must produce nothing",
                                label
                            );
                            sp += css;
                            if css == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 221 — LZ4F_freeDecompressionContext returns the dStage
// ===========================================================================

/// ERRORS.md row 221: freeing a context whose frame is only partially decoded
/// returns `(LZ4F_errorCode_t)dctx->dStage` (lz4frame.c:1316-1317) — a NON-ZERO
/// but NON-ERROR value in 1..14, so `LZ4F_isError()` is 0 on it and
/// `LZ4F_getErrorCode()` maps it to `LZ4F_OK_NoError`; callers must compare
/// against 0 directly.
#[test]
fn row_221_free_dctx_returns_dstage() {
    let (cn, rn) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cf, rf) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cre, rre) = both::<FnResetDctx>("LZ4F_resetDecompressionContext");

    let mut rng = Rng::new(0x0221_0000_0000_0001);
    let data = gen_shape(&mut rng, 1, 5000);

    // A frame whose block is COMPRESSED and one whose block is UNCOMPRESSED, so
    // several different mid-frame stages are reachable.
    let h = Hdr::new(LZ4F_max64KB);
    let hdr = h.bytes();
    let comp = lz4_block(&data);
    let mut cframe = hdr.clone();
    cframe.extend_from_slice(&compressed_block(&data, false));
    cframe.extend_from_slice(&frame_tail(&data, false));
    let mut uframe = hdr.clone();
    uframe.extend_from_slice(&uncompressed_block(&data, false));
    uframe.extend_from_slice(&frame_tail(&data, false));

    // (label, frame, bytes to feed, dst capacity)
    let cases: Vec<(String, &Vec<u8>, usize, usize)> = vec![
        ("nothing fed".to_string(), &cframe, 0, 70_000),
        ("3 header bytes".to_string(), &cframe, 3, 70_000),
        ("6 header bytes".to_string(), &cframe, 6, 70_000),
        ("header only".to_string(), &cframe, hdr.len(), 70_000),
        (
            "header + 2 block-header bytes".to_string(),
            &cframe,
            hdr.len() + 2,
            70_000,
        ),
        (
            "header + block header + half the payload".to_string(),
            &cframe,
            hdr.len() + 4 + comp.len() / 2,
            70_000,
        ),
        (
            "uncompressed block, half copied".to_string(),
            &uframe,
            hdr.len() + 4 + data.len() / 2,
            70_000,
        ),
        (
            "compressed block decoded into tmpOut, flush pending".to_string(),
            &cframe,
            cframe.len() - 4,
            100,
        ),
        ("whole frame".to_string(), &cframe, cframe.len(), 70_000),
    ];

    for (label, frame, feed, dst_cap) in cases {
        unsafe {
            let mut cd: *mut c_void = ptr::null_mut();
            let mut rd: *mut c_void = ptr::null_mut();
            expect_ok("row 221: create", cn(&mut cd, LZ4F_VERSION), rn(&mut rd, LZ4F_VERSION));
            if feed > 0 {
                let d = DctxPair { c: cd, r: rd };
                let (ret, _, _) = dec_step(
                    &format!("row 221: feed [{}]", label),
                    &d,
                    &frame[..feed],
                    dst_cap,
                );
                assert!(!lz4f_is_error(ret), "row 221: [{}] feeding failed", label);
                // Do NOT let DctxPair's Drop free them: we free manually below.
                std::mem::forget(d);
            }
            let a = cf(cd);
            let b = rf(rd);
            same_ret(&format!("row 221: free after [{}]", label), a, b);
            assert!(
                !lz4f_is_error(a),
                "row 221: [{}] the dStage value must NOT look like an error (got {})",
                label,
                a
            );
            assert_eq!(
                lz4f_error_code(a),
                err::OK_NoError,
                "row 221: [{}] LZ4F_getErrorCode must map it to OK_NoError",
                label
            );
            assert!(
                a <= 14,
                "row 221: [{}] expected a dStage in 0..=14, got {}",
                label,
                a
            );
        }
    }

    // A fresh context, and a context reset with LZ4F_resetDecompressionContext,
    // both report dStage == 0 (dstage_getFrameHeader).
    unsafe {
        let mut cd: *mut c_void = ptr::null_mut();
        let mut rd: *mut c_void = ptr::null_mut();
        expect_ok("row 221: create", cn(&mut cd, LZ4F_VERSION), rn(&mut rd, LZ4F_VERSION));
        let d = DctxPair { c: cd, r: rd };
        let (ret, _, _) = dec_step("row 221: feed a partial header", &d, &cframe[..3], 4096);
        assert!(!lz4f_is_error(ret));
        std::mem::forget(d);
        cre(cd);
        rre(rd);
        let a = cf(cd);
        let b = rf(rd);
        same_ret("row 221: free after reset", a, b);
        assert_eq!(a, 0, "row 221: a reset context reports dStage 0");
    }
}

// ===========================================================================
// Rows 195-219 through LZ4F_decompress_usingDict
// ===========================================================================

/// Rows 195-219 all name `LZ4F_decompress_usingDict` alongside `LZ4F_decompress`
/// (it forwards to the same state machine after installing the dictionary,
/// lz4frame.c:2119-2135). Every error construction from the tests above is
/// replayed through that entry point here, with a real dictionary installed, so
/// the second entry point is covered too.
#[test]
fn rows_196_219_via_decompress_using_dict() {
    let mut rng = Rng::new(0x0D1C_0000_1234_5678);
    let dict = gen_shape(&mut rng, 3, 8192);
    let data = gen_shape(&mut rng, 1, 4096);

    let mut cases: Vec<(String, Vec<u8>, i32)> = Vec::new();

    // row 196 — bad magic
    {
        let mut f = Vec::new();
        f.extend_from_slice(&le32(0x1234_5678));
        f.extend_from_slice(&[0x40, 0x70, 0x00]);
        f.extend(std::iter::repeat(0xCDu8).take(24));
        cases.push(("row 196: bad magic".to_string(), f, err::ERROR_frameType_unknown));
    }
    // row 197 — FLG reserved bit 1
    cases.push((
        "row 197: FLG reserved bit".to_string(),
        header_from_flg_bd(0x42, 0x70),
        err::ERROR_reservedFlag_set,
    ));
    // row 198 — FLG version
    cases.push((
        "row 198: FLG version".to_string(),
        header_from_flg_bd(0x00, 0x70),
        err::ERROR_headerVersion_wrong,
    ));
    // row 199 — BD reserved bit 7
    cases.push((
        "row 199: BD reserved bit 7".to_string(),
        header_from_flg_bd(0x40, 0xC0),
        err::ERROR_reservedFlag_set,
    ));
    // row 200 — blockSizeID < 4
    cases.push((
        "row 200: blockSizeID 3".to_string(),
        header_from_flg_bd(0x40, 0x30),
        err::ERROR_maxBlockSize_invalid,
    ));
    // row 201 — BD low nibble
    cases.push((
        "row 201: BD low nibble".to_string(),
        header_from_flg_bd(0x40, 0x71),
        err::ERROR_reservedFlag_set,
    ));
    // row 202 — header checksum
    {
        let mut h = Hdr::new(LZ4F_max64KB).bytes();
        let last = h.len() - 1;
        h[last] ^= 0xFF;
        cases.push((
            "row 202: header checksum".to_string(),
            h,
            err::ERROR_headerChecksum_invalid,
        ));
    }
    // row 213 — block header > maxBlockSize
    {
        let mut f = Hdr::new(LZ4F_max64KB).bytes();
        f.extend_from_slice(&le32(65_537));
        cases.push((
            "row 213: block header too large".to_string(),
            f,
            err::ERROR_maxBlockSize_invalid,
        ));
    }
    // row 214 — uncompressed block checksum
    {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.block_ck = true;
        let hdr = h.bytes();
        let mut f = hdr.clone();
        f.extend_from_slice(&uncompressed_block(&data, true));
        f.extend_from_slice(&frame_tail(&data, false));
        let off = hdr.len() + LZ4F_BLOCK_HEADER_SIZE + data.len();
        f[off] ^= 0xFF;
        cases.push((
            "row 214: uncompressed block checksum".to_string(),
            f,
            err::ERROR_blockChecksum_invalid,
        ));
    }
    // row 215 — compressed block checksum
    {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.block_ck = true;
        let hdr = h.bytes();
        let comp = lz4_block(&data);
        assert!(comp.len() < data.len(), "helper: data must compress");
        let mut f = hdr.clone();
        f.extend_from_slice(&compressed_block(&data, true));
        f.extend_from_slice(&frame_tail(&data, false));
        let off = hdr.len() + LZ4F_BLOCK_HEADER_SIZE + comp.len();
        f[off] ^= 0xFF;
        cases.push((
            "row 215: compressed block checksum".to_string(),
            f,
            err::ERROR_blockChecksum_invalid,
        ));
    }
    // rows 216/217 — corrupt payload
    {
        let mut f = Hdr::new(LZ4F_max64KB).bytes();
        let bad = vec![0xFFu8; 40];
        f.extend_from_slice(&le32(bad.len() as u32));
        f.extend_from_slice(&bad);
        f.extend_from_slice(&le32(0));
        cases.push((
            "rows 216/217: corrupt payload".to_string(),
            f,
            err::ERROR_decompressionFailed,
        ));
    }
    // row 218 — contentSize mismatch
    {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.content_size = Some(data.len() as u64 + 5);
        cases.push((
            "row 218: contentSize mismatch".to_string(),
            frame_1block(&h, &data),
            err::ERROR_frameSize_wrong,
        ));
    }
    // row 219 — content checksum mismatch
    {
        let mut h = Hdr::new(LZ4F_max64KB);
        h.content_ck = true;
        let mut f = frame_1block(&h, &data);
        let last = f.len() - 1;
        f[last] ^= 0xFF;
        cases.push((
            "row 219: content checksum".to_string(),
            f,
            err::ERROR_contentChecksum_invalid,
        ));
    }

    for (label, frame, want) in &cases {
        for &dst_cap in &[100usize, 70_000] {
            for &src_chunk in &[0usize, 5] {
                let mut cfg = DecCfg::new(dst_cap);
                cfg.src_chunk = src_chunk;
                cfg.dict = Some(dict.clone());
                dec_expect_err(
                    &format!(
                        "{} via LZ4F_decompress_usingDict (dstCap={} srcChunk={})",
                        label, dst_cap, src_chunk
                    ),
                    frame,
                    &cfg,
                    *want,
                );
            }
        }
        // ... and with an empty dictionary, which takes the same code path.
        let mut cfg = DecCfg::new(70_000);
        cfg.dict = Some(Vec::new());
        dec_expect_err(
            &format!("{} via LZ4F_decompress_usingDict (empty dict)", label),
            frame,
            &cfg,
            *want,
        );
    }
}

// ===========================================================================
// Rows 168-169 x 182 — an out-of-range blockSizeID crossing the FFI
// ===========================================================================

/// `LZ4F_blockSizeID_t` has only non-negative enumerators, so GCC's compatible
/// type is `unsigned int` and a negative `int` arrives as a huge unsigned value.
/// `LZ4F_compressBegin*` does **not** validate `frameInfo.blockSizeID`
/// (lz4frame.c:740 assigns `LZ4F_getBlockSize()`'s result — possibly an ERROR
/// code — straight into `cctx->maxBlockSize`), and that value then feeds
/// `LZ4F_compressBound_internal`, i.e. the row-182 rejection test. This pins that
/// C and Rust agree on the resulting (garbage) bound, in both signs.
///
/// Only `autoFlush == 1` is driven: with `autoFlush == 0` the "keep tmpIn within
/// limits" block at lz4frame.c:1081-1090 uses the bogus `blockSize` and reads
/// out of bounds inside the C itself.
#[test]
fn row_168_169_182_out_of_range_blocksizeid_across_ffi() {
    let (cb, rb) = both::<FnBound>("LZ4F_compressBound");
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");

    let src = [0x33u8; 64];
    // Enough room that, if the C decides the capacity is sufficient and emits a
    // block, the write lands inside our own allocation.
    const ROOM: usize = 1 << 20;

    for &bsid in &[
        -1i32,
        1,
        2,
        3,
        8,
        9,
        100,
        255,
        65_536,
        c_int::MIN,
        c_int::MAX,
    ] {
        for &mode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &(cck, bck) in &[(0, 0), (1, 1)] {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.blockSizeID = bsid;
                p.frameInfo.blockMode = mode;
                p.frameInfo.contentChecksumFlag = cck;
                p.frameInfo.blockChecksumFlag = bck;
                p.compressionLevel = 1;
                p.autoFlush = 1;

                // LZ4F_compressBound swallows the LZ4F_getBlockSize error too.
                unsafe {
                    let a = cb(src.len(), &p as *const _);
                    let b = rb(src.len(), &p as *const _);
                    assert_eq!(
                        a, b,
                        "LZ4F_compressBound(64, bsid={}) C={} Rust={}",
                        bsid, a, b
                    );
                }

                let s = begin_pair(&format!("row 168/169/182: bsid={}", bsid), &p);
                for &cap in &[0usize, 1, 8, 100] {
                    let mut cd = vec![SENTINEL; ROOM];
                    let mut rd = vec![SENTINEL; ROOM];
                    let label = format!(
                        "rows 168/169/182: bsid={} mode={} cck={} bck={} cap={}",
                        bsid, mode, cck, bck, cap
                    );
                    unsafe {
                        let a = cu(
                            s.c,
                            cd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            ptr::null(),
                        );
                        let b = ru(
                            s.r,
                            rd.as_mut_ptr() as *mut c_void,
                            cap,
                            src.as_ptr() as *const c_void,
                            src.len(),
                            ptr::null(),
                        );
                        same_ret(&label, a, b);
                        if lz4f_is_error(a) {
                            assert_eq!(
                                lz4f_error_code(a),
                                err::ERROR_dstMaxSize_tooSmall,
                                "{}: the only reachable error here is row 182",
                                label
                            );
                        }
                    }
                    assert_bytes_eq(&label, &cd, &rd);
                }
            }
        }
    }
}

// ===========================================================================
// ROW COVERAGE MAP — ERRORS.md rows 168-221 (the `## lz4frame.c` section)
//
// row  covering #[test]                                            (or reason)
// ---- ----------------------------------------------------------------------
// 168  row_168_169_get_block_size_invalid_id
//      + row_168_169_182_out_of_range_blocksizeid_across_ffi
// 169  row_168_169_get_block_size_invalid_id
//      + row_168_169_182_out_of_range_blocksizeid_across_ffi
// 170  row_170_compress_frame_dst_too_small
// 171  row_171_compress_begin_dst_too_small
// 172  row_172_173_compress_begin_allocation_failures            (fail_at = 2)
// 173  row_172_173_compress_begin_allocation_failures            (fail_at = 3)
// 174  row_174_compress_begin_dict_size_too_large
// 175  NOT TESTABLE — not compiled into the library under test. The row IS the
//      `assert(LZ4F_compressionContextPtr != NULL)` at lz4frame.c:620, and
//      lz4frame.c:143-149 defines `assert(condition)` as `((void)0)` unless
//      `LZ4_DEBUG >= 1`; c_src/CMakeLists.txt defines only `LZ4_HEAPMODE=0`
//      and `LZ4F_HEAPMODE=0`, so the assert is a no-op here. The production
//      behaviour the row points at is row 176, which IS tested.
// 176  row_176_192_create_context_null_out_pointer
// 177  row_177_193_default_allocator_failure   (child process: RLIMIT_AS +
//                                               drained heap => a real calloc
//                                               failure through the DEFAULT
//                                               allocator)
// 178  row_178_194_create_advanced_allocation_failure
// 179  row_179_180_create_cdict_allocation_failures              (fail_at = 1)
// 180  row_179_180_create_cdict_allocation_failures        (fail_at = 2, 3, 4)
// 181  row_181_update_before_begin
// 182  row_182_183_update_dst_too_small
//      + row_168_169_182_out_of_range_blocksizeid_across_ffi
// 183  row_182_183_update_dst_too_small           (autoFlush == 0 sub-block, so
//                                                  bound_internal < srcSize)
// 184  NOT TESTABLE — `assert(blockCompression == LZ4B_COMPRESSED)` at
//      lz4frame.c:1071, i.e. `LZ4F_uncompressedUpdate` on a `LZ4F_blockLinked`
//      frame. lz4frame.h:707 documents that combination as unsupported and the
//      assert is compiled out here (see row 175), so the C falls into
//      `LZ4F_localSaveDict()` with a `tmpBuff` that was sized for 64 KB and
//      corrupts its OWN heap (lz4frame.c:1068-1079). Driving it would crash
//      both libraries inside undefined behaviour and prove nothing.
// 185  row_185_186_flush_state_and_capacity — the reachable half (the early
//      `return 0` at lz4frame.c:1167 that makes `LZ4F_flush` /
//      `LZ4F_compressEnd` on a fresh or finished cctx NOT report an
//      uninitialised state) is asserted. The error branch itself needs
//      `tmpInSize != 0 && cStage != 1`; the doc comment on that test carries
//      the full case analysis showing no public call sequence can produce it
//      (`cStage` is only ever 0 or 1, and both `cStage = 0` sites necessarily
//      have `tmpInSize == 0`).
// 186  row_185_186_flush_state_and_capacity      (via LZ4F_flush AND
//                                                 via LZ4F_compressEnd)
// 187  row_187_update_internal_flush_not_error_checked
//      (a) the failing internal flush, (b) the > dstCapacity over-write with a
//      slack region; case (b) is also pinned by
//      tests/lz4frame_dstcapacity_overrun.rs
// 188  row_188_189_190_compress_end_errors    (flushSize == 0 sweep, and the
//                                              flushSize > 0 narrow window)
// 189  row_188_189_190_compress_end_errors
// 190  row_188_189_190_compress_end_errors
// 191  NOT TESTABLE — the same situation as row 175: the row IS
//      `assert(LZ4F_decompressionContextPtr != NULL)` at lz4frame.c:1303,
//      compiled out via lz4frame.c:143-149. Its production behaviour is row
//      192, which IS tested.
// 192  row_176_192_create_context_null_out_pointer
// 193  row_177_193_default_allocator_failure
// 194  row_178_194_create_advanced_allocation_failure
// 195  row_195_207_208_get_frame_info_incomplete_header — the exact condition
//      (fewer than 7 bytes of header where LZ4F_decodeHeader is entered) is
//      constructed through both public entry points; see the test's doc comment
//      for why the C reports it one line earlier with the identical code (12).
// 196  row_196_202_209_decode_header_validation
//      + rows_196_219_via_decompress_using_dict
// 197  row_196_202_209_decode_header_validation  + _using_dict
// 198  row_196_202_209_decode_header_validation  + _using_dict
// 199  row_196_202_209_decode_header_validation  + _using_dict
// 200  row_196_202_209_decode_header_validation  + _using_dict
// 201  row_196_202_209_decode_header_validation  + _using_dict
// 202  row_196_202_209_decode_header_validation  + _using_dict
// 203  row_203_205_header_size_errors
// 204  row_203_205_header_size_errors
// 205  row_203_205_header_size_errors
// 206  row_206_get_frame_info_partial_header
// 207  row_195_207_208_get_frame_info_incomplete_header
// 208  row_195_207_208_get_frame_info_incomplete_header
// 209  row_196_202_209_decode_header_validation (asserts `*srcSizePtr == 0` and
//      that `*frameInfoPtr` is the MEM_INIT-zeroed frameInfo)
// 210  row_210_get_frame_info_after_header_decoded
// 211  row_211_212_decompress_allocation_failures                (fail_at = 2)
// 212  row_211_212_decompress_allocation_failures                (fail_at = 3)
// 213  row_213_block_header_too_large  + rows_196_219_via_decompress_using_dict
// 214  row_214_215_block_checksum_invalid                        + _using_dict
// 215  row_214_215_block_checksum_invalid                        + _using_dict
// 216  row_216_217_corrupt_block_payload      (dstCapacity >= maxBlockSize)
// 217  row_216_217_corrupt_block_payload      (dstCapacity <  maxBlockSize)
// 218  row_218_frame_size_wrong                                  + _using_dict
// 219  row_219_content_checksum_invalid                          + _using_dict
// 220  row_220_null_dst_with_nonzero_size (the assert at lz4frame.c:1632 is
//      compiled out, so the documented release behaviour is what is asserted)
// 221  row_221_free_dctx_returns_dstage
//
// No C-vs-Rust divergence was found: every construction above produced the
// identical return value, the identical LZ4F_getErrorCode(), identical
// out-parameters and byte-identical destination buffers in both libraries, so
// src/lz4frame.rs was not modified.
// ===========================================================================
