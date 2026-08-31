//! Differential tests for the *compression* half of the lz4frame API.
//!
//! Covers CONFIGS.md rows 104..=141 (the `## lz4frame` section up to and
//! including row 141).
//!
//! Every call goes through a `.so` export via libloading; the C library and the
//! Rust library each get their OWN opaque contexts (`LZ4F_cctx`, `LZ4F_dctx`,
//! `LZ4F_CDict`), created and destroyed by the same library. Only return values
//! and *bytes* are ever compared.
//!
//! Destination buffers are always allocated per-library, pre-filled with a
//! `0xCD` sentinel and followed by a guard region, and the WHOLE allocation is
//! compared afterwards so that differing scribbles are caught too.
#![allow(unused_imports, non_snake_case, non_upper_case_globals)]

mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc (for the custom-allocator shims)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn calloc(n: usize, s: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// LZ4F_CustomMem mirror (lz4frame.h, LZ4F_STATIC_LINKING_ONLY section)
// ---------------------------------------------------------------------------

type AllocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type CallocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Copy, Clone)]
struct LZ4F_CustomMem {
    customAlloc: Option<AllocFn>,
    customCalloc: Option<CallocFn>,
    customFree: Option<FreeFn>,
    opaqueState: *mut c_void,
}

const DEFAULT_CMEM: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: ptr::null_mut(),
};

/// Opaque state handed to the shims: counts live allocations.
#[repr(C)]
struct MemStat {
    allocs: usize,
    callocs: usize,
    frees: usize,
}

unsafe extern "C" fn shim_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    if !opaque.is_null() {
        (*(opaque as *mut MemStat)).allocs += 1;
    }
    malloc(size)
}

unsafe extern "C" fn shim_calloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    if !opaque.is_null() {
        (*(opaque as *mut MemStat)).callocs += 1;
    }
    calloc(1, size)
}

unsafe extern "C" fn shim_free(opaque: *mut c_void, p: *mut c_void) {
    if !opaque.is_null() {
        (*(opaque as *mut MemStat)).frees += 1;
    }
    free(p)
}

fn cmem_alloc_only(st: &mut MemStat) -> LZ4F_CustomMem {
    LZ4F_CustomMem {
        customAlloc: Some(shim_alloc),
        customCalloc: None, // documented: "optional; when not defined, uses customAlloc + memset"
        customFree: Some(shim_free),
        opaqueState: st as *mut MemStat as *mut c_void,
    }
}

fn cmem_full(st: &mut MemStat) -> LZ4F_CustomMem {
    LZ4F_CustomMem {
        customAlloc: Some(shim_alloc),
        customCalloc: Some(shim_calloc),
        customFree: Some(shim_free),
        opaqueState: st as *mut MemStat as *mut c_void,
    }
}

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCompressFrameUsingCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
type FnCreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnCreateCtxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
type FnFreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnCreateCDictAdv =
    unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
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
type FnFlushEnd = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
type FnGetBlockSize = unsafe extern "C" fn(c_uint) -> usize;
type FnGetVersion = unsafe extern "C" fn() -> c_uint;
type FnLevelMax = unsafe extern "C" fn() -> c_int;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnGetFrameInfo = unsafe extern "C" fn(
    *mut c_void,
    *mut LZ4F_frameInfo_t,
    *const c_void,
    *mut usize,
) -> usize;
type FnResetDCtx = unsafe extern "C" fn(*mut c_void);
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

#[derive(Copy, Clone)]
struct Api {
    tag: &'static str,
    compress_frame: FnCompressFrame,
    compress_frame_bound: FnBound,
    compress_bound: FnBound,
    compress_frame_using_cdict: FnCompressFrameUsingCDict,
    create_cctx: FnCreateCtx,
    create_cctx_adv: FnCreateCtxAdv,
    free_cctx: FnFreeCtx,
    create_cdict: FnCreateCDict,
    create_cdict_adv: FnCreateCDictAdv,
    free_cdict: FnFreeCDict,
    compress_begin: FnCompressBegin,
    compress_begin_using_dict: FnCompressBeginUsingDict,
    compress_begin_using_dict_once: FnCompressBeginUsingDict,
    compress_begin_using_cdict: FnCompressBeginUsingCDict,
    compress_update: FnCompressUpdate,
    uncompressed_update: FnCompressUpdate,
    flush: FnFlushEnd,
    compress_end: FnFlushEnd,
    get_block_size: FnGetBlockSize,
    get_version: FnGetVersion,
    level_max: FnLevelMax,
    is_error: FnIsError,
    get_error_name: FnGetErrorName,
    get_error_code: FnGetErrorCode,
    header_size: FnHeaderSize,
    get_frame_info: FnGetFrameInfo,
    create_dctx: FnCreateCtx,
    create_dctx_adv: FnCreateCtxAdv,
    free_dctx: FnFreeCtx,
    reset_dctx: FnResetDCtx,
    decompress: FnDecompress,
    decompress_using_dict: FnDecompressUsingDict,
}

macro_rules! pair {
    ($l:expr, $t:ty, $n:expr) => {{
        let (a, b) = $l.sym::<$t>($n);
        (*a, *b)
    }};
}

fn apis() -> (&'static Api, &'static Api) {
    static P: OnceLock<(Api, Api)> = OnceLock::new();
    let p = P.get_or_init(|| unsafe {
        let l = libs();
        {
            // Paranoia: the two libraries must be two distinct code objects.
            let (a, b) = l.sym::<FnCompressFrame>("LZ4F_compressFrame");
            assert_ne!(
                *a as usize, *b as usize,
                "harness bug: LZ4F_compressFrame resolved to the same address in both libraries"
            );
        }
        let cf = pair!(l, FnCompressFrame, "LZ4F_compressFrame");
        let cfb = pair!(l, FnBound, "LZ4F_compressFrameBound");
        let cb = pair!(l, FnBound, "LZ4F_compressBound");
        let cfd = pair!(l, FnCompressFrameUsingCDict, "LZ4F_compressFrame_usingCDict");
        let ccc = pair!(l, FnCreateCtx, "LZ4F_createCompressionContext");
        let ccca = pair!(l, FnCreateCtxAdv, "LZ4F_createCompressionContext_advanced");
        let fcc = pair!(l, FnFreeCtx, "LZ4F_freeCompressionContext");
        let ccd = pair!(l, FnCreateCDict, "LZ4F_createCDict");
        let ccda = pair!(l, FnCreateCDictAdv, "LZ4F_createCDict_advanced");
        let fcd = pair!(l, FnFreeCDict, "LZ4F_freeCDict");
        let cbg = pair!(l, FnCompressBegin, "LZ4F_compressBegin");
        let cbd = pair!(l, FnCompressBeginUsingDict, "LZ4F_compressBegin_usingDict");
        let cbdo = pair!(l, FnCompressBeginUsingDict, "LZ4F_compressBegin_usingDictOnce");
        let cbcd = pair!(l, FnCompressBeginUsingCDict, "LZ4F_compressBegin_usingCDict");
        let cu = pair!(l, FnCompressUpdate, "LZ4F_compressUpdate");
        let uu = pair!(l, FnCompressUpdate, "LZ4F_uncompressedUpdate");
        let fl = pair!(l, FnFlushEnd, "LZ4F_flush");
        let ce = pair!(l, FnFlushEnd, "LZ4F_compressEnd");
        let gbs = pair!(l, FnGetBlockSize, "LZ4F_getBlockSize");
        let gv = pair!(l, FnGetVersion, "LZ4F_getVersion");
        let lm = pair!(l, FnLevelMax, "LZ4F_compressionLevel_max");
        let ie = pair!(l, FnIsError, "LZ4F_isError");
        let gen_ = pair!(l, FnGetErrorName, "LZ4F_getErrorName");
        let gec = pair!(l, FnGetErrorCode, "LZ4F_getErrorCode");
        let hs = pair!(l, FnHeaderSize, "LZ4F_headerSize");
        let gfi = pair!(l, FnGetFrameInfo, "LZ4F_getFrameInfo");
        let cdc = pair!(l, FnCreateCtx, "LZ4F_createDecompressionContext");
        let cdca = pair!(l, FnCreateCtxAdv, "LZ4F_createDecompressionContext_advanced");
        let fdc = pair!(l, FnFreeCtx, "LZ4F_freeDecompressionContext");
        let rdc = pair!(l, FnResetDCtx, "LZ4F_resetDecompressionContext");
        let dc = pair!(l, FnDecompress, "LZ4F_decompress");
        let dcd = pair!(l, FnDecompressUsingDict, "LZ4F_decompress_usingDict");
        (
            Api {
                tag: "C",
                compress_frame: cf.0,
                compress_frame_bound: cfb.0,
                compress_bound: cb.0,
                compress_frame_using_cdict: cfd.0,
                create_cctx: ccc.0,
                create_cctx_adv: ccca.0,
                free_cctx: fcc.0,
                create_cdict: ccd.0,
                create_cdict_adv: ccda.0,
                free_cdict: fcd.0,
                compress_begin: cbg.0,
                compress_begin_using_dict: cbd.0,
                compress_begin_using_dict_once: cbdo.0,
                compress_begin_using_cdict: cbcd.0,
                compress_update: cu.0,
                uncompressed_update: uu.0,
                flush: fl.0,
                compress_end: ce.0,
                get_block_size: gbs.0,
                get_version: gv.0,
                level_max: lm.0,
                is_error: ie.0,
                get_error_name: gen_.0,
                get_error_code: gec.0,
                header_size: hs.0,
                get_frame_info: gfi.0,
                create_dctx: cdc.0,
                create_dctx_adv: cdca.0,
                free_dctx: fdc.0,
                reset_dctx: rdc.0,
                decompress: dc.0,
                decompress_using_dict: dcd.0,
            },
            Api {
                tag: "Rust",
                compress_frame: cf.1,
                compress_frame_bound: cfb.1,
                compress_bound: cb.1,
                compress_frame_using_cdict: cfd.1,
                create_cctx: ccc.1,
                create_cctx_adv: ccca.1,
                free_cctx: fcc.1,
                create_cdict: ccd.1,
                create_cdict_adv: ccda.1,
                free_cdict: fcd.1,
                compress_begin: cbg.1,
                compress_begin_using_dict: cbd.1,
                compress_begin_using_dict_once: cbdo.1,
                compress_begin_using_cdict: cbcd.1,
                compress_update: cu.1,
                uncompressed_update: uu.1,
                flush: fl.1,
                compress_end: ce.1,
                get_block_size: gbs.1,
                get_version: gv.1,
                level_max: lm.1,
                is_error: ie.1,
                get_error_name: gen_.1,
                get_error_code: gec.1,
                header_size: hs.1,
                get_frame_info: gfi.1,
                create_dctx: cdc.1,
                create_dctx_adv: cdca.1,
                free_dctx: fdc.1,
                reset_dctx: rdc.1,
                decompress: dc.1,
                decompress_using_dict: dcd.1,
            },
        )
    });
    (&p.0, &p.1)
}

// ---------------------------------------------------------------------------
// Destination buffers with sentinel + guard
// ---------------------------------------------------------------------------

const SENT: u8 = 0xCD;
const GUARD: usize = 32;

struct Dst {
    v: Vec<u8>,
    cap: usize,
}

impl Dst {
    fn new(cap: usize) -> Dst {
        Dst {
            v: vec![SENT; cap + GUARD],
            cap,
        }
    }
    fn p(&mut self) -> *mut c_void {
        self.v.as_mut_ptr() as *mut c_void
    }
    fn all(&self) -> &[u8] {
        &self.v
    }
    /// Verify the guard region past `cap` is untouched.
    #[track_caller]
    fn check_guard(&self, ctx: &str, tag: &str) {
        for (i, b) in self.v[self.cap..].iter().enumerate() {
            assert_eq!(
                *b, SENT,
                "{ctx}: {tag} wrote past dstCapacity ({} bytes) at guard offset {i}",
                self.cap
            );
        }
    }
}

fn optp(p: Option<&LZ4F_preferences_t>) -> *const LZ4F_preferences_t {
    match p {
        Some(x) => x as *const LZ4F_preferences_t,
        None => ptr::null(),
    }
}

fn optc(p: Option<&LZ4F_compressOptions_t>) -> *const LZ4F_compressOptions_t {
    match p {
        Some(x) => x as *const LZ4F_compressOptions_t,
        None => ptr::null(),
    }
}

/// Run the same operation in both libraries with separate destination buffers
/// and compare the return value, the produced bytes, and the whole allocation.
///
/// `f` receives `(api, cctx, dstPtr, dstCapacity, which)` where `which` is 0
/// for C and 1 for Rust (used to select the per-library opaque objects).
#[track_caller]
unsafe fn duo<F>(
    ctx: &str,
    cap: usize,
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
    mut f: F,
) -> (usize, Vec<u8>)
where
    F: FnMut(&'static Api, *mut c_void, *mut c_void, usize, usize) -> usize,
{
    let (c, r) = apis();
    let mut dc = Dst::new(cap);
    let mut dr = Dst::new(cap);
    let rc = f(c, cctx_c, dc.p(), cap, 0);
    let rr = f(r, cctx_r, dr.p(), cap, 1);
    same_usize_and_bytes(ctx, rc, rr, dc.all(), dr.all());
    same_full_buffers(ctx, dc.all(), dr.all());
    dc.check_guard(ctx, "C");
    dr.check_guard(ctx, "Rust");
    let out = if is_err_range(rc) {
        Vec::new()
    } else {
        dc.v[..rc.min(dc.cap)].to_vec()
    };
    (rc, out)
}

/// `LZ4F_compressFrame` on both libraries.
#[track_caller]
unsafe fn cf(
    ctx: &str,
    src: &[u8],
    cap: usize,
    prefs: Option<&LZ4F_preferences_t>,
) -> (usize, Vec<u8>) {
    let pp = optp(prefs);
    let sp = src.as_ptr() as *const c_void;
    let n = src.len();
    duo(ctx, cap, ptr::null_mut(), ptr::null_mut(), |a, _, dp, capv, _| {
        (a.compress_frame)(dp, capv, sp, n, pp)
    })
}

#[track_caller]
unsafe fn cfbound(n: usize, prefs: Option<&LZ4F_preferences_t>) -> usize {
    let (c, r) = apis();
    let pp = optp(prefs);
    let a = (c.compress_frame_bound)(n, pp);
    let b = (r.compress_frame_bound)(n, pp);
    assert_eq!(a, b, "LZ4F_compressFrameBound({n}) mismatch");
    a
}

#[track_caller]
unsafe fn cbound(n: usize, prefs: Option<&LZ4F_preferences_t>) -> usize {
    let (c, r) = apis();
    let pp = optp(prefs);
    let a = (c.compress_bound)(n, pp);
    let b = (r.compress_bound)(n, pp);
    assert_eq!(a, b, "LZ4F_compressBound({n}) mismatch");
    a
}

unsafe fn new_cctx(a: &Api) -> *mut c_void {
    let mut p: *mut c_void = ptr::null_mut();
    let r = (a.create_cctx)(&mut p, LZ4F_VERSION);
    assert_eq!(r, 0, "{}: createCompressionContext failed", a.tag);
    assert!(!p.is_null(), "{}: createCompressionContext gave NULL", a.tag);
    p
}

unsafe fn new_dctx(a: &Api) -> *mut c_void {
    let mut p: *mut c_void = ptr::null_mut();
    let r = (a.create_dctx)(&mut p, LZ4F_VERSION);
    assert_eq!(r, 0, "{}: createDecompressionContext failed", a.tag);
    assert!(!p.is_null());
    p
}

// ---------------------------------------------------------------------------
// Frame parsing helpers
// ---------------------------------------------------------------------------

const MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

#[derive(Debug, Clone, Copy)]
struct Hdr {
    size: usize,
    flg: u8,
    bd: u8,
    content_size: u64,
    dict_id: u32,
}

#[track_caller]
fn parse_header(f: &[u8]) -> Hdr {
    assert!(f.len() >= 7, "frame too short: {}", f.len());
    assert_eq!(&f[0..4], &MAGIC[..], "bad magic: {}", hexdump(&f[..4.min(f.len())]));
    let flg = f[4];
    let bd = f[5];
    let cs_flag = (flg >> 3) & 1;
    let did_flag = flg & 1;
    let size = 7 + if cs_flag != 0 { 8 } else { 0 } + if did_flag != 0 { 4 } else { 0 };
    assert!(f.len() >= size);
    let content_size = if cs_flag != 0 {
        u64::from_le_bytes(f[6..14].try_into().unwrap())
    } else {
        0
    };
    let dict_id = if did_flag != 0 {
        u32::from_le_bytes(f[size - 5..size - 1].try_into().unwrap())
    } else {
        0
    };
    Hdr {
        size,
        flg,
        bd,
        content_size,
        dict_id,
    }
}

fn bsid_to_size(id: u32) -> usize {
    match id {
        4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => 64 * 1024,
    }
}

fn frame_block_size(f: &[u8]) -> usize {
    if f.len() < 7 || f[0..4] != MAGIC {
        return 4 * 1024 * 1024;
    }
    bsid_to_size(((f[5] >> 4) & 7) as u32)
}

/// `(stored_flag, data_offset, data_size)` for each block of a well-formed frame.
#[track_caller]
fn blocks(f: &[u8]) -> Vec<(bool, usize, usize)> {
    let h = parse_header(f);
    let bc = ((h.flg >> 4) & 1) != 0;
    let mut out = Vec::new();
    let mut o = h.size;
    loop {
        assert!(o + 4 <= f.len(), "truncated block header at {o}");
        let bh = u32::from_le_bytes(f[o..o + 4].try_into().unwrap());
        if bh == 0 {
            break;
        }
        let stored = (bh & 0x8000_0000) != 0;
        let sz = (bh & 0x7FFF_FFFF) as usize;
        out.push((stored, o + 4, sz));
        o += 4 + sz + if bc { 4 } else { 0 };
    }
    out
}

/// The `LZ4F_optimalBSID` computation from lz4frame.c, mirrored for assertions.
fn optimal_bsid(requested: u32, src_size: usize) -> u32 {
    let mut proposed = 4u32;
    let mut max_block = 64 * 1024usize;
    while requested > proposed {
        if src_size <= max_block {
            return proposed;
        }
        proposed += 1;
        max_block <<= 2;
    }
    requested
}

// ---------------------------------------------------------------------------
// Decompression drivers (used to cross-check the produced frames)
// ---------------------------------------------------------------------------

unsafe fn decode_all(a: &Api, frame: &[u8], dict: Option<&[u8]>) -> Result<Vec<u8>, usize> {
    let dctx = new_dctx(a);
    let bs = frame_block_size(frame);
    let mut tmp = vec![0u8; bs + 16];
    let mut out: Vec<u8> = Vec::new();
    let mut ip = 0usize;
    let res = loop {
        let mut dsz = tmp.len();
        let mut ssz = frame.len() - ip;
        let hint = match dict {
            Some(d) => (a.decompress_using_dict)(
                dctx,
                tmp.as_mut_ptr() as *mut c_void,
                &mut dsz,
                frame.as_ptr().add(ip) as *const c_void,
                &mut ssz,
                d.as_ptr() as *const c_void,
                d.len(),
                ptr::null(),
            ),
            None => (a.decompress)(
                dctx,
                tmp.as_mut_ptr() as *mut c_void,
                &mut dsz,
                frame.as_ptr().add(ip) as *const c_void,
                &mut ssz,
                ptr::null(),
            ),
        };
        if is_err_range(hint) {
            break Err(hint);
        }
        out.extend_from_slice(&tmp[..dsz]);
        ip += ssz;
        if hint == 0 {
            break Ok(out);
        }
        if ssz == 0 && dsz == 0 {
            // no progress possible (truncated input)
            break Err(err(16));
        }
    };
    (a.free_dctx)(dctx);
    res
}

/// Cross-decompress: decode the frame with BOTH libraries and check the payload.
#[track_caller]
unsafe fn xdec(ctx: &str, frame: &[u8], expect: &[u8], dict: Option<&[u8]>) {
    let (c, r) = apis();
    for a in [c, r] {
        match decode_all(a, frame, dict) {
            Ok(out) => {
                assert_eq!(
                    out.len(),
                    expect.len(),
                    "{ctx}: {} decoded {} bytes, expected {}",
                    a.tag,
                    out.len(),
                    expect.len()
                );
                if let Some(i) = first_diff(&out, expect) {
                    panic!("{ctx}: {} decoded payload differs at {i}", a.tag);
                }
            }
            Err(e) => panic!(
                "{ctx}: {} failed to decode the frame: {} (frame {} bytes: {})",
                a.tag,
                e as isize,
                frame.len(),
                hexdump(frame)
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Low-level pipeline driver
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
enum Step {
    Update(usize),
    Uncompressed(usize),
    Flush,
}

#[derive(Copy, Clone)]
enum Begin<'a> {
    Plain,
    UsingDict(&'a [u8]),
    UsingDictOnce(&'a [u8]),
    UsingCDict(&'a [u8]),
}

/// Drive `compressBegin` → steps → `compressEnd` on both libraries in lockstep,
/// comparing every intermediate return value and every destination buffer.
/// Returns `(frame_bytes, bytes_consumed_from_src)`.
#[track_caller]
unsafe fn run_pipeline(
    ctx: &str,
    src: &[u8],
    steps: &[Step],
    prefs: Option<&LZ4F_preferences_t>,
    copts: Option<&LZ4F_compressOptions_t>,
    begin: Begin,
) -> (Vec<u8>, usize) {
    let (c, r) = apis();
    let cc = new_cctx(c);
    let cr = new_cctx(r);
    let pp = optp(prefs);
    let op = optc(copts);
    let (cdc, cdr) = match begin {
        Begin::UsingCDict(d) => {
            let a = (c.create_cdict)(d.as_ptr() as *const c_void, d.len());
            let b = (r.create_cdict)(d.as_ptr() as *const c_void, d.len());
            assert!(!a.is_null() && !b.is_null(), "{ctx}: createCDict returned NULL");
            (a, b)
        }
        _ => (ptr::null_mut(), ptr::null_mut()),
    };

    let mut frame: Vec<u8> = Vec::new();
    {
        let (n, out) = duo(
            &format!("{ctx}: compressBegin"),
            19,
            cc,
            cr,
            |a, cx, dp, capv, w| match begin {
                Begin::Plain => (a.compress_begin)(cx, dp, capv, pp),
                Begin::UsingDict(d) => (a.compress_begin_using_dict)(
                    cx,
                    dp,
                    capv,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    pp,
                ),
                Begin::UsingDictOnce(d) => (a.compress_begin_using_dict_once)(
                    cx,
                    dp,
                    capv,
                    d.as_ptr() as *const c_void,
                    d.len(),
                    pp,
                ),
                Begin::UsingCDict(_) => {
                    (a.compress_begin_using_cdict)(cx, dp, capv, if w == 0 { cdc } else { cdr }, pp)
                }
            },
        );
        assert!(
            !is_err_range(n),
            "{ctx}: compressBegin failed: {}",
            n as isize
        );
        frame.extend_from_slice(&out);
    }

    let mut off = 0usize;
    for (si, st) in steps.iter().enumerate() {
        match *st {
            Step::Update(want) => {
                let n = want.min(src.len() - off);
                let cap = cbound(n, prefs).max(n) + 8;
                let sp = src.as_ptr().add(off) as *const c_void;
                let (ret, out) = duo(
                    &format!("{ctx}: compressUpdate[{si}] n={n}"),
                    cap,
                    cc,
                    cr,
                    |a, cx, dp, capv, _| (a.compress_update)(cx, dp, capv, sp, n, op),
                );
                assert!(
                    !is_err_range(ret),
                    "{ctx}: compressUpdate[{si}] n={n} cap={cap} failed: {}",
                    ret as isize
                );
                frame.extend_from_slice(&out);
                off += n;
            }
            Step::Uncompressed(want) => {
                let n = want.min(src.len() - off);
                let cap = cbound(n, prefs).max(n) + 8;
                let sp = src.as_ptr().add(off) as *const c_void;
                let (ret, out) = duo(
                    &format!("{ctx}: uncompressedUpdate[{si}] n={n}"),
                    cap,
                    cc,
                    cr,
                    |a, cx, dp, capv, _| (a.uncompressed_update)(cx, dp, capv, sp, n, op),
                );
                assert!(
                    !is_err_range(ret),
                    "{ctx}: uncompressedUpdate[{si}] n={n} failed: {}",
                    ret as isize
                );
                frame.extend_from_slice(&out);
                off += n;
            }
            Step::Flush => {
                let cap = cbound(0, prefs).max(8);
                let (ret, out) = duo(
                    &format!("{ctx}: flush[{si}]"),
                    cap,
                    cc,
                    cr,
                    |a, cx, dp, capv, _| (a.flush)(cx, dp, capv, op),
                );
                assert!(!is_err_range(ret), "{ctx}: flush[{si}] failed: {}", ret as isize);
                frame.extend_from_slice(&out);
            }
        }
    }

    {
        let cap = cbound(0, prefs).max(8);
        let (ret, out) = duo(
            &format!("{ctx}: compressEnd"),
            cap,
            cc,
            cr,
            |a, cx, dp, capv, _| (a.compress_end)(cx, dp, capv, op),
        );
        assert!(!is_err_range(ret), "{ctx}: compressEnd failed: {}", ret as isize);
        frame.extend_from_slice(&out);
    }

    if !cdc.is_null() {
        (c.free_cdict)(cdc);
        (r.free_cdict)(cdr);
    }
    assert_eq!((c.free_cctx)(cc), 0);
    assert_eq!((r.free_cctx)(cr), 0);
    (frame, off)
}

fn pref() -> LZ4F_preferences_t {
    LZ4F_preferences_t::default()
}

// ===========================================================================
// Row 104 — LZ4F_compressFrame with preferencesPtr == NULL
// ===========================================================================

#[test]
fn row_104_compress_frame_prefs_null() {
    let mut rng = Rng::new(104);
    unsafe {
        // property-style: every (shape, size) pair is retried with fresh random
        // payloads REPS times (the RNG is never reset)
        const REPS: usize = 80;
        for shape in ALL_SHAPES.iter().cycle().take(ALL_SHAPES.len() * REPS).copied() {
            for n in interesting_sizes() {
                let src = gen(&mut rng, shape, n);
                let cap = cfbound(n, None);
                let ctx = format!("row104 {shape:?} n={n}");
                let (ret, frame) = cf(&ctx, &src, cap, None);
                assert!(!is_err_range(ret), "{ctx}: compressFrame failed {}", ret as isize);
                let h = parse_header(&frame);
                // defaults: version 01, no checksums, no contentSize, no dictID
                assert_eq!(h.size, 7, "{ctx}: header size");
                assert_eq!(h.bd, 0x40, "{ctx}: BD byte (max64KB expected)");
                // blockMode is forced independent when srcSize <= blockSize
                let expect_indep = n <= 64 * 1024;
                assert_eq!(
                    (h.flg >> 5) & 1,
                    expect_indep as u8,
                    "{ctx}: FLG blockIndependent bit (flg={:#02x})",
                    h.flg
                );
                assert_eq!(h.flg & 0xC0, 0x40, "{ctx}: FLG version bits");
                xdec(&ctx, &frame, &src, None);
            }
        }
    }
}

// ===========================================================================
// Row 105 — every blockSizeID with srcSize above the block size
// ===========================================================================

#[test]
fn row_105_block_size_ids_multi_block() {
    let mut rng = Rng::new(105);
    let cases: &[(c_uint, usize, &[c_int])] = &[
        (LZ4F_DEFAULT, 64 * 1024 + 1, &[0, 1, 9]),
        (LZ4F_DEFAULT, 3 * 64 * 1024 + 7, &[0, 2]),
        (LZ4F_MAX64KB, 64 * 1024 - 1, &[0, 1]),
        (LZ4F_MAX64KB, 64 * 1024 + 1, &[0, 1, 9, 12]),
        (LZ4F_MAX64KB, 5 * 64 * 1024, &[0, 2]),
        (LZ4F_MAX256KB, 256 * 1024 - 1, &[0]),
        (LZ4F_MAX256KB, 256 * 1024 + 1, &[0, 1]),
        (LZ4F_MAX256KB, 2 * 256 * 1024 + 3, &[0, 9]),
        (LZ4F_MAX1MB, 1024 * 1024 - 1, &[0]),
        (LZ4F_MAX1MB, 1024 * 1024 + 1, &[0]),
        (LZ4F_MAX1MB, 2 * 1024 * 1024 + 9, &[0]),
        (LZ4F_MAX4MB, 4 * 1024 * 1024 + 1, &[0]),
        (LZ4F_MAX4MB, 5 * 1024 * 1024, &[0]),
    ];
    unsafe {
        const REPS: usize = 8;
        for &(bsid, n, levels) in cases.iter().cycle().take(cases.len() * REPS) {
            let mut shapes: Vec<Shape> = vec![Shape::Compressible, Shape::TextLike];
            if n <= 512 * 1024 {
                shapes.push(Shape::Incompressible);
                shapes.push(Shape::Periodic);
            }
            for shape in shapes {
                let src = gen(&mut rng, shape, n);
                for &level in levels {
                    let mut p = pref();
                    p.frameInfo.blockSizeID = bsid;
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row105 bsid={bsid} n={n} level={level} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    let h = parse_header(&frame);
                    let want = optimal_bsid(if bsid == 0 { 0 } else { bsid }, n);
                    let want = if want == 0 { 4 } else { want };
                    assert_eq!(
                        ((h.bd >> 4) & 7) as u32,
                        want,
                        "{ctx}: stored blockSizeID (bd={:#02x})",
                        h.bd
                    );
                    let bs = bsid_to_size(want);
                    let nb = blocks(&frame).len();
                    let expect_nb = if n == 0 { 0 } else { (n + bs - 1) / bs };
                    assert_eq!(nb, expect_nb, "{ctx}: number of blocks");
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 106 — LZ4F_optimalBSID downgrades the stored BD byte
// ===========================================================================

#[test]
fn row_106_optimal_bsid_downgrade() {
    let mut rng = Rng::new(106);
    unsafe {
        const REPS: usize = 24;
        for requested in [LZ4F_MAX256KB, LZ4F_MAX1MB, LZ4F_MAX4MB]
            .iter()
            .cycle()
            .take(3 * REPS)
            .copied()
        {
            for n in [
                0usize,
                1,
                1024,
                64 * 1024,
                64 * 1024 + 1,
                256 * 1024,
                256 * 1024 + 1,
                1024 * 1024,
                1024 * 1024 + 1,
            ] {
                for shape in [Shape::Compressible, Shape::TextLike] {
                    let src = gen(&mut rng, shape, n);
                    let mut p = pref();
                    p.frameInfo.blockSizeID = requested;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row106 req={requested} n={n} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    let h = parse_header(&frame);
                    let want = optimal_bsid(requested, n);
                    assert_eq!(
                        ((h.bd >> 4) & 7) as u32,
                        want,
                        "{ctx}: BD byte {:#02x}",
                        h.bd
                    );
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
        // the row's headline case: max4MB with 1 KB of input becomes max64KB
        let src = gen(&mut rng, Shape::TextLike, 1024);
        let mut p = pref();
        p.frameInfo.blockSizeID = LZ4F_MAX4MB;
        let cap = cfbound(1024, Some(&p));
        let (_, frame) = cf("row106 headline", &src, cap, Some(&p));
        assert_eq!(frame[5], 0x40, "max4MB + 1 KB must store max64KB in BD");
    }
}

// ===========================================================================
// Row 107 — blockMode linked vs independent
// ===========================================================================

#[test]
fn row_107_block_mode() {
    let mut rng = Rng::new(107);
    unsafe {
        const REPS: usize = 24;
        for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT]
            .iter()
            .cycle()
            .take(2 * REPS)
            .copied()
        {
            for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB] {
                let bs = bsid_to_size(bsid);
                for n in [0usize, 1, bs / 2, bs - 1, bs, bs + 1, 3 * bs + 7] {
                    for &level in &[0i32, 2] {
                        for shape in [Shape::Compressible, Shape::TextLike] {
                            let src = gen(&mut rng, shape, n);
                            let mut p = pref();
                            p.frameInfo.blockSizeID = bsid;
                            p.frameInfo.blockMode = mode;
                            p.compressionLevel = level;
                            let cap = cfbound(n, Some(&p));
                            let ctx =
                                format!("row107 mode={mode} bsid={bsid} n={n} lvl={level} {shape:?}");
                            let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                            assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                            let h = parse_header(&frame);
                            let stored_bs = bsid_to_size(((h.bd >> 4) & 7) as u32);
                            // srcSize <= blockSize forces blockIndependent
                            let want_indep = mode == LZ4F_BLOCK_INDEPENDENT || n <= stored_bs;
                            assert_eq!(
                                (h.flg >> 5) & 1,
                                want_indep as u8,
                                "{ctx}: FLG blockMode bit (flg={:#02x})",
                                h.flg
                            );
                            xdec(&ctx, &frame, &src, None);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 108 — contentChecksum x blockChecksum
// ===========================================================================

#[test]
fn row_108_checksum_combinations() {
    let mut rng = Rng::new(108);
    unsafe {
        const REPS: usize = 40;
        for cc in [LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED]
            .iter()
            .cycle()
            .take(2 * REPS)
            .copied()
        {
            for bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED] {
                for n in [0usize, 1, 100, 65535, 65536, 70000, 200000] {
                    for shape in [Shape::Compressible, Shape::Incompressible, Shape::TextLike] {
                        for &level in &[0i32, 1] {
                            let src = gen(&mut rng, shape, n);
                            let mut p = pref();
                            p.frameInfo.contentChecksumFlag = cc;
                            p.frameInfo.blockChecksumFlag = bc;
                            p.compressionLevel = level;
                            let cap = cfbound(n, Some(&p));
                            let ctx = format!(
                                "row108 cc={cc} bc={bc} n={n} lvl={level} {shape:?}"
                            );
                            let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                            assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                            let h = parse_header(&frame);
                            assert_eq!((h.flg >> 2) & 1, cc as u8, "{ctx}: FLG contentChecksum");
                            assert_eq!((h.flg >> 4) & 1, bc as u8, "{ctx}: FLG blockChecksum");
                            // footer: endMark (+ content checksum)
                            let bl = blocks(&frame);
                            let mut end = h.size;
                            for (_, o, s) in &bl {
                                end = o + s + if bc != 0 { 4 } else { 0 };
                            }
                            if bl.is_empty() {
                                end = h.size;
                            }
                            let want_len = end + 4 + if cc != 0 { 4 } else { 0 };
                            assert_eq!(frame.len(), want_len, "{ctx}: frame length/footer");
                            xdec(&ctx, &frame, &src, None);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 109 — contentSize 0 vs non-zero (auto-corrected to srcSize)
// ===========================================================================

#[test]
fn row_109_content_size() {
    let mut rng = Rng::new(109);
    unsafe {
        const REPS: usize = 40;
        for n in [0usize, 1, 1000, 65536, 70000, 200000]
            .iter()
            .cycle()
            .take(6 * REPS)
            .copied()
        {
            for &declared in &[0u64, 1, n as u64, 12345, u64::MAX] {
                for shape in [Shape::Compressible, Shape::TextLike] {
                    let src = gen(&mut rng, shape, n);
                    let mut p = pref();
                    p.frameInfo.contentSize = declared;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row109 n={n} declared={declared} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(
                        !is_err_range(ret),
                        "{ctx}: compressFrame failed {}",
                        ret as isize
                    );
                    let h = parse_header(&frame);
                    // A non-zero declared contentSize is auto-corrected to srcSize
                    // *before* the FLG bit is computed, so declaring a size for a
                    // 0-byte payload still produces a 7-byte header.
                    let effective = if declared == 0 { 0 } else { n as u64 };
                    if effective == 0 {
                        assert_eq!(h.size, 7, "{ctx}: no contentSize field expected");
                        assert_eq!((h.flg >> 3) & 1, 0, "{ctx}: FLG contentSize bit");
                    } else {
                        assert_eq!(h.size, 15, "{ctx}: contentSize field expected");
                        assert_eq!((h.flg >> 3) & 1, 1, "{ctx}: FLG contentSize bit");
                        assert_eq!(
                            h.content_size, n as u64,
                            "{ctx}: declared contentSize must be auto-corrected to srcSize"
                        );
                    }
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 110 — dictID and the four possible header sizes 7 / 11 / 15 / 19
// ===========================================================================

#[test]
fn row_110_dict_id_and_header_sizes() {
    let mut rng = Rng::new(110);
    unsafe {
        let (c, r) = apis();
        let mut seen: Vec<usize> = Vec::new();
        const REPS: usize = 40;
        for &dict_id in [0u32, 1, 0xDEAD_BEEF, u32::MAX].iter().cycle().take(4 * REPS) {
            for &declared in &[0u64, 1] {
                for n in [1usize, 5000, 70000] {
                    let src = gen(&mut rng, Shape::TextLike, n);
                    let mut p = pref();
                    p.frameInfo.dictID = dict_id;
                    p.frameInfo.contentSize = declared;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row110 dictID={dict_id} cs={declared} n={n}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    let h = parse_header(&frame);
                    let want = 7 + if declared != 0 { 8 } else { 0 }
                        + if dict_id != 0 { 4 } else { 0 };
                    assert_eq!(h.size, want, "{ctx}: header size");
                    assert_eq!(h.dict_id, dict_id, "{ctx}: dictID field");
                    assert_eq!(h.flg & 1, (dict_id != 0) as u8, "{ctx}: FLG dictID bit");
                    // LZ4F_headerSize must agree in both libraries
                    let a = (c.header_size)(frame.as_ptr() as *const c_void, frame.len());
                    let b = (r.header_size)(frame.as_ptr() as *const c_void, frame.len());
                    assert_eq!(a, b, "{ctx}: LZ4F_headerSize mismatch");
                    assert_eq!(a, want, "{ctx}: LZ4F_headerSize value");
                    if !seen.contains(&want) {
                        seen.push(want);
                    }
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
        seen.sort();
        assert_eq!(seen, vec![7, 11, 15, 19], "all four header sizes produced");

        // srcSize == 0 with a declared contentSize: auto-correction to 0 clears
        // the FLG contentSize bit, so only the dictID field can remain.
        for &dict_id in &[0u32, 7] {
            let mut p = pref();
            p.frameInfo.dictID = dict_id;
            p.frameInfo.contentSize = 999;
            let cap = cfbound(0, Some(&p));
            let ctx = format!("row110 n=0 dictID={dict_id}");
            let (ret, frame) = cf(&ctx, &[], cap, Some(&p));
            assert!(!is_err_range(ret));
            let h = parse_header(&frame);
            assert_eq!(h.size, 7 + if dict_id != 0 { 4 } else { 0 }, "{ctx}: header size");
            assert_eq!(h.dict_id, dict_id);
            xdec(&ctx, &frame, &[], None);
        }
    }
}

// ===========================================================================
// Row 111 — compressionLevel 0 and 1 (fast context)
// ===========================================================================

#[test]
fn row_111_levels_0_and_1() {
    let mut rng = Rng::new(111);
    unsafe {
        const REPS: usize = 60;
        for &level in [0i32, 1].iter().cycle().take(2 * REPS) {
            for shape in ALL_SHAPES {
                for n in interesting_sizes() {
                    let src = gen(&mut rng, shape, n);
                    let mut p = pref();
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row111 lvl={level} n={n} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 112 — negative compressionLevel (LZ4 acceleration = -level + 1)
// ===========================================================================

#[test]
fn row_112_negative_levels() {
    let mut rng = Rng::new(112);
    unsafe {
        const REPS: usize = 60;
        for &level in [-1i32, -2, -10, -1000, -65536].iter().cycle().take(5 * REPS) {
            for shape in ALL_SHAPES {
                for n in [0usize, 1, 13, 100, 4096, 65535, 65536, 65537, 200000] {
                    let src = gen(&mut rng, shape, n);
                    let mut p = pref();
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row112 lvl={level} n={n} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 113 — compressionLevel 2 and 3..9 (HC context)
// ===========================================================================

#[test]
fn row_113_levels_2_to_9() {
    let mut rng = Rng::new(113);
    unsafe {
        const REPS: usize = 18;
        for level in (2i32..=9).cycle().take(8 * REPS) {
            for shape in ALL_SHAPES {
                for n in [0usize, 1, 13, 100, 4096, 65535, 65536, 65537, 150000] {
                    let src = gen(&mut rng, shape, n);
                    let mut p = pref();
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row113 lvl={level} n={n} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 114 — compressionLevel 10, 12, and 13/100 (clamped to 12)
// ===========================================================================

#[test]
fn row_114_levels_10_12_and_clamped() {
    let mut rng = Rng::new(114);
    unsafe {
        const REPS: usize = 10;
        for shape in ALL_SHAPES.iter().cycle().take(ALL_SHAPES.len() * REPS).copied() {
            for n in [0usize, 1, 13, 100, 4096, 65535, 65536, 65537, 120000] {
                let src = gen(&mut rng, shape, n);
                let mut frames: Vec<(c_int, Vec<u8>)> = Vec::new();
                for &level in &[10i32, 11, 12, 13, 100, 1000] {
                    let mut p = pref();
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row114 lvl={level} n={n} {shape:?}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    xdec(&ctx, &frame, &src, None);
                    frames.push((level, frame));
                }
                // levels above LZ4HC_CLEVEL_MAX are clamped to 12
                let l12 = frames.iter().find(|f| f.0 == 12).unwrap().1.clone();
                for (level, f) in &frames {
                    if *level > 12 {
                        assert_eq!(
                            f, &l12,
                            "row114 n={n} {shape:?}: level {level} must match level 12"
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 115 — favorDecSpeed
// ===========================================================================

#[test]
fn row_115_favor_dec_speed() {
    let mut rng = Rng::new(115);
    unsafe {
        const REPS: usize = 24;
        for &level in [0i32, 1, 10, 12].iter().cycle().take(4 * REPS) {
            for n in [0usize, 1, 100, 4096, 65537, 150000] {
                for shape in [Shape::Compressible, Shape::TextLike, Shape::Incompressible] {
                    let src = gen(&mut rng, shape, n);
                    let mut out: Vec<Vec<u8>> = Vec::new();
                    for fds in [0u32, 1] {
                        let mut p = pref();
                        p.compressionLevel = level;
                        p.favorDecSpeed = fds;
                        let cap = cfbound(n, Some(&p));
                        let ctx = format!("row115 lvl={level} fds={fds} n={n} {shape:?}");
                        let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                        assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                        xdec(&ctx, &frame, &src, None);
                        out.push(frame);
                    }
                    if level < 2 {
                        // no HC context => favorDecSpeed cannot be applied
                        assert_eq!(
                            out[0], out[1],
                            "row115 lvl={level} n={n} {shape:?}: favorDecSpeed must be a no-op below LZ4HC_CLEVEL_MIN"
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 116 — srcSize at the block-size boundaries
// ===========================================================================

#[test]
fn row_116_src_size_boundaries() {
    let mut rng = Rng::new(116);
    unsafe {
        const REPS: usize = 32;
        for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB].iter().cycle().take(2 * REPS).copied() {
            let bs = bsid_to_size(bsid);
            for n in [0usize, 1, bs - 1, bs, bs + 1, 2 * bs, 3 * bs] {
                for &level in &[0i32, 1, 2] {
                    for shape in [Shape::Compressible, Shape::TextLike] {
                        let src = gen(&mut rng, shape, n);
                        let mut p = pref();
                        p.frameInfo.blockSizeID = bsid;
                        p.compressionLevel = level;
                        let cap = cfbound(n, Some(&p));
                        let ctx = format!("row116 bsid={bsid} n={n} lvl={level} {shape:?}");
                        let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                        assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                        let h = parse_header(&frame);
                        let stored_bs = bsid_to_size(((h.bd >> 4) & 7) as u32);
                        let nb = blocks(&frame).len();
                        let want = if n == 0 {
                            0
                        } else {
                            (n + stored_bs - 1) / stored_bs
                        };
                        assert_eq!(nb, want, "{ctx}: block count");
                        if n == 0 {
                            assert_eq!(frame.len(), h.size + 4, "{ctx}: header + endMark only");
                        }
                        xdec(&ctx, &frame, &src, None);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 117 — incompressible data takes the stored-block path
// ===========================================================================

#[test]
fn row_117_incompressible_stored_blocks() {
    let mut rng = Rng::new(117);
    unsafe {
        const REPS: usize = 60;
        for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB].iter().cycle().take(2 * REPS).copied() {
            let bs = bsid_to_size(bsid);
            for n in [1usize, 4, 10, 100, 1000, 20000, bs, bs + 1000] {
                for &level in &[0i32, 1, 9, 12] {
                    if level >= 9 && n > 70000 {
                        continue; // keep the budget in check
                    }
                    let src = gen_incompressible(&mut rng, n);
                    let mut p = pref();
                    p.frameInfo.blockSizeID = bsid;
                    p.compressionLevel = level;
                    let cap = cfbound(n, Some(&p));
                    let ctx = format!("row117 bsid={bsid} n={n} lvl={level}");
                    let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
                    assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                    let bl = blocks(&frame);
                    assert!(!bl.is_empty(), "{ctx}: expected at least one block");
                    let mut any_stored = false;
                    let mut off = 0usize;
                    for &(stored, o, sz) in &bl {
                        if stored {
                            any_stored = true;
                            assert_eq!(
                                &frame[o..o + sz],
                                &src[off..off + sz],
                                "{ctx}: stored block content must be a verbatim copy"
                            );
                            off += sz;
                        } else {
                            // a compressed block: its decompressed size is not
                            // recoverable from the frame alone, so stop walking
                            break;
                        }
                    }
                    assert!(
                        any_stored,
                        "{ctx}: random data should produce at least one stored block"
                    );
                    xdec(&ctx, &frame, &src, None);
                }
            }
        }
    }
}

// ===========================================================================
// Row 118 — dstCapacity below LZ4F_compressFrameBound
// ===========================================================================

#[test]
fn row_118_dst_capacity_too_small() {
    let mut rng = Rng::new(118);
    unsafe {
        const REPS: usize = 40;
        for n in [0usize, 1, 100, 65536, 70000].iter().cycle().take(5 * REPS).copied() {
            for &level in &[0i32, 2] {
                for cc in [0u32, 1] {
                    let mut p = pref();
                    p.compressionLevel = level;
                    p.frameInfo.contentChecksumFlag = cc;
                    let src = gen(&mut rng, Shape::TextLike, n);
                    let bound = cfbound(n, Some(&p));
                    let ctx = format!("row118 n={n} lvl={level} cc={cc}");
                    // exactly the bound succeeds
                    let (ok, frame) = cf(&format!("{ctx} exact"), &src, bound, Some(&p));
                    assert!(!is_err_range(ok), "{ctx}: bound-sized dst must succeed");
                    xdec(&ctx, &frame, &src, None);
                    // one byte less => ERROR_dstMaxSize_tooSmall
                    for cap in [bound - 1, bound / 2, 1, 0] {
                        let (bad, _) = cf(&format!("{ctx} cap={cap}"), &src, cap, Some(&p));
                        assert_eq!(
                            bad,
                            err(11),
                            "{ctx}: cap={cap} expected ERROR_dstMaxSize_tooSmall, got {}",
                            bad as isize
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 119 — LZ4F_compressFrameBound / LZ4F_compressBound
// ===========================================================================

#[test]
fn row_119_bounds() {
    unsafe {
        let mut sizes: Vec<usize> = interesting_sizes();
        sizes.extend_from_slice(&[
            1024 * 1024,
            1024 * 1024 + 1,
            4 * 1024 * 1024,
            4 * 1024 * 1024 + 1,
            16 * 1024 * 1024,
        ]);
        // prefs == NULL: worst case (both checksums enabled)
        for &n in &sizes {
            let a = cfbound(n, None);
            let b = cbound(n, None);
            assert!(a > 0 && b > 0, "bounds must be positive for n={n}");
        }
        for bsid in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 100, u32::MAX] {
            for af in [0u32, 1] {
                for cc in [0u32, 1] {
                    for bc in [0u32, 1] {
                        for &n in &sizes {
                            let mut p = pref();
                            p.frameInfo.blockSizeID = bsid;
                            p.frameInfo.contentChecksumFlag = cc;
                            p.frameInfo.blockChecksumFlag = bc;
                            p.autoFlush = af;
                            // compares C vs Rust internally
                            let _fb = cfbound(n, Some(&p));
                            let _cb = cbound(n, Some(&p));
                            if bsid >= 4 && bsid <= 7 {
                                let bs = bsid_to_size(bsid);
                                let end = 4 + 4 * cc as usize;
                                if n == 0 {
                                    // srcSize == 0 bounds a flush / compressEnd
                                    assert_eq!(
                                        _cb,
                                        if af == 1 {
                                            end
                                        } else {
                                            4 + 4 * bc as usize + (bs - 1) + end
                                        },
                                        "compressBound(0) bsid={bsid} af={af} cc={cc} bc={bc}"
                                    );
                                }
                                assert!(
                                    _fb >= _cb.min(_fb),
                                    "sanity: frameBound {_fb} vs bound {_cb}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 120 — LZ4F_createCDict / _advanced / LZ4F_freeCDict
// ===========================================================================

#[test]
fn row_120_cdict_lifecycle() {
    let mut rng = Rng::new(120);
    unsafe {
        let (c, r) = apis();
        // freeCDict(NULL) must be a no-op in both libraries
        (c.free_cdict)(ptr::null_mut());
        (r.free_cdict)(ptr::null_mut());

        const REPS: usize = 10;
        for dsize in [0usize, 1, 3, 4, 100, 65535, 65536, 65537, 100_000, 200_000]
            .iter()
            .cycle()
            .take(10 * REPS)
            .copied()
        {
            let dict = gen(&mut rng, Shape::TextLike, dsize);
            let payload = gen(&mut rng, Shape::TextLike, 30_000);
            for &level in &[0i32, 1, 2, 9] {
                let mut p = pref();
                p.compressionLevel = level;
                let cap = cfbound(payload.len(), Some(&p));
                let ctx = format!("row120 dsize={dsize} lvl={level}");

                // three creation flavours per library; all must give identical frames
                let mut per_lib: Vec<Vec<Vec<u8>>> = Vec::new();
                for a in [c, r] {
                    let mut st_alloc = MemStat {
                        allocs: 0,
                        callocs: 0,
                        frees: 0,
                    };
                    let mut st_full = MemStat {
                        allocs: 0,
                        callocs: 0,
                        frees: 0,
                    };
                    let dp = dict.as_ptr() as *const c_void;
                    let plain = (a.create_cdict)(dp, dsize);
                    let adv_default = (a.create_cdict_adv)(DEFAULT_CMEM, dp, dsize);
                    let adv_alloc_only =
                        (a.create_cdict_adv)(cmem_alloc_only(&mut st_alloc), dp, dsize);
                    let adv_full = (a.create_cdict_adv)(cmem_full(&mut st_full), dp, dsize);
                    for (name, cd) in [
                        ("plain", plain),
                        ("adv_default", adv_default),
                        ("adv_alloc_only", adv_alloc_only),
                        ("adv_full", adv_full),
                    ] {
                        assert!(!cd.is_null(), "{ctx}: {} createCDict {name} = NULL", a.tag);
                    }
                    let mut frames = Vec::new();
                    let cctx = new_cctx(a);
                    for cd in [plain, adv_default, adv_alloc_only, adv_full] {
                        let mut d = Dst::new(cap);
                        let ret = (a.compress_frame_using_cdict)(
                            cctx,
                            d.p(),
                            cap,
                            payload.as_ptr() as *const c_void,
                            payload.len(),
                            cd,
                            &p,
                        );
                        assert!(
                            !is_err_range(ret),
                            "{ctx}: {} compressFrame_usingCDict failed {}",
                            a.tag,
                            ret as isize
                        );
                        d.check_guard(&ctx, a.tag);
                        frames.push(d.v[..ret].to_vec());
                    }
                    assert_eq!((a.free_cctx)(cctx), 0);
                    (a.free_cdict)(plain);
                    (a.free_cdict)(adv_default);
                    (a.free_cdict)(adv_alloc_only);
                    (a.free_cdict)(adv_full);
                    // the custom allocators must be balanced
                    assert_eq!(
                        st_alloc.allocs + st_alloc.callocs,
                        st_alloc.frees,
                        "{ctx}: {} alloc-only CustomMem leaked ({} allocs / {} frees)",
                        a.tag,
                        st_alloc.allocs,
                        st_alloc.frees
                    );
                    assert_eq!(
                        st_full.allocs + st_full.callocs,
                        st_full.frees,
                        "{ctx}: {} full CustomMem leaked",
                        a.tag
                    );
                    assert!(st_alloc.allocs > 0, "{ctx}: {} customAlloc unused", a.tag);
                    // createCDict_advanced only mallocs (no calloc) in lz4frame.c
                    assert!(st_full.allocs > 0, "{ctx}: {} customAlloc unused", a.tag);
                    for i in 1..frames.len() {
                        assert_eq!(
                            frames[0], frames[i],
                            "{ctx}: {} CDict creation flavour {i} changed the output",
                            a.tag
                        );
                    }
                    per_lib.push(frames);
                }
                assert_eq!(
                    per_lib[0], per_lib[1],
                    "{ctx}: C and Rust CDict-compressed frames differ"
                );
                xdec(&ctx, &per_lib[0][0], &payload, Some(&dict));
            }
        }
    }
}

// ===========================================================================
// Row 121 — LZ4F_compressFrame_usingCDict
// ===========================================================================

#[test]
fn row_121_compress_frame_using_cdict() {
    let mut rng = Rng::new(121);
    unsafe {
        let (c, r) = apis();
        const REPS: usize = 12;
        for dsize in [4096usize, 100_000].iter().cycle().take(2 * REPS).copied() {
            let dict = gen(&mut rng, Shape::TextLike, dsize);
            let dp = dict.as_ptr() as *const c_void;
            let cdc = (c.create_cdict)(dp, dsize);
            let cdr = (r.create_cdict)(dp, dsize);
            assert!(!cdc.is_null() && !cdr.is_null());
            let cctx_c = new_cctx(c);
            let cctx_r = new_cctx(r);
            for &level in &[0i32, 1, 2, 9] {
                for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    for n in [0usize, 1, 1000, 65536, 200_000] {
                        for use_dict in [false, true] {
                            let src = gen(&mut rng, Shape::TextLike, n);
                            let mut p = pref();
                            p.compressionLevel = level;
                            p.frameInfo.blockMode = mode;
                            p.frameInfo.blockSizeID = LZ4F_MAX64KB;
                            p.frameInfo.dictID = if use_dict { 0x1234 } else { 0 };
                            let cap = cfbound(n, Some(&p));
                            let ctx = format!(
                                "row121 dsize={dsize} lvl={level} mode={mode} n={n} dict={use_dict}"
                            );
                            let sp = src.as_ptr() as *const c_void;
                            let pp = &p as *const LZ4F_preferences_t;
                            let (ret, frame) =
                                duo(&ctx, cap, cctx_c, cctx_r, |a, cx, dpr, capv, w| {
                                    let cd = if !use_dict {
                                        ptr::null_mut()
                                    } else if w == 0 {
                                        cdc
                                    } else {
                                        cdr
                                    };
                                    (a.compress_frame_using_cdict)(cx, dpr, capv, sp, n, cd, pp)
                                });
                            assert!(!is_err_range(ret), "{ctx}: failed {}", ret as isize);
                            xdec(
                                &ctx,
                                &frame,
                                &src,
                                if use_dict { Some(&dict) } else { None },
                            );
                        }
                    }
                }
            }
            assert_eq!((c.free_cctx)(cctx_c), 0);
            assert_eq!((r.free_cctx)(cctx_r), 0);
            (c.free_cdict)(cdc);
            (r.free_cdict)(cdr);
        }
    }
}

// ===========================================================================
// Row 122 — cctx creation / destruction
// ===========================================================================

#[test]
fn row_122_cctx_lifecycle() {
    let mut rng = Rng::new(122);
    unsafe {
        let (c, r) = apis();
        // freeCompressionContext(NULL) must return OK in both libraries
        assert_eq!((c.free_cctx)(ptr::null_mut()), 0);
        assert_eq!((r.free_cctx)(ptr::null_mut()), 0);

        for version in [LZ4F_VERSION, 0, 99, 101, u32::MAX] {
            let mut pc: *mut c_void = ptr::null_mut();
            let mut pr: *mut c_void = ptr::null_mut();
            let a = (c.create_cctx)(&mut pc, version);
            let b = (r.create_cctx)(&mut pr, version);
            assert_eq!(a, b, "createCompressionContext(version={version}) mismatch");
            assert_eq!(a, 0);
            assert!(!pc.is_null() && !pr.is_null());
            assert_eq!((c.free_cctx)(pc), 0);
            assert_eq!((r.free_cctx)(pr), 0);
        }

        // _advanced with the default and with custom allocators, then actually use it
        let src = gen(&mut rng, Shape::TextLike, 130_000);
        let mut p = pref();
        p.frameInfo.blockSizeID = LZ4F_MAX64KB;
        p.compressionLevel = 3;
        let mut ref_frame: Option<Vec<u8>> = None;
        for flavour in 0..3 {
            let mut per_lib: Vec<Vec<u8>> = Vec::new();
            for a in [c, r] {
                let mut st = MemStat {
                    allocs: 0,
                    callocs: 0,
                    frees: 0,
                };
                let cctx = match flavour {
                    0 => (a.create_cctx_adv)(DEFAULT_CMEM, LZ4F_VERSION),
                    1 => (a.create_cctx_adv)(cmem_alloc_only(&mut st), LZ4F_VERSION),
                    _ => (a.create_cctx_adv)(cmem_full(&mut st), LZ4F_VERSION),
                };
                assert!(
                    !cctx.is_null(),
                    "{}: createCompressionContext_advanced flavour {flavour} = NULL",
                    a.tag
                );
                let cap = cfbound(src.len(), Some(&p));
                let mut d = Dst::new(cap);
                let ret = (a.compress_frame_using_cdict)(
                    cctx,
                    d.p(),
                    cap,
                    src.as_ptr() as *const c_void,
                    src.len(),
                    ptr::null_mut(),
                    &p,
                );
                assert!(!is_err_range(ret), "{}: frame failed {}", a.tag, ret as isize);
                d.check_guard("row122", a.tag);
                per_lib.push(d.v[..ret].to_vec());
                assert_eq!((a.free_cctx)(cctx), 0);
                if flavour > 0 {
                    assert!(st.allocs + st.callocs > 0, "{}: custom alloc unused", a.tag);
                    assert_eq!(
                        st.allocs + st.callocs,
                        st.frees,
                        "{}: custom allocator leaked (flavour {flavour})",
                        a.tag
                    );
                }
            }
            assert_eq!(per_lib[0], per_lib[1], "row122 flavour {flavour}: frames differ");
            match &ref_frame {
                None => ref_frame = Some(per_lib[0].clone()),
                Some(f) => assert_eq!(
                    f, &per_lib[0],
                    "row122: allocator flavour must not change the frame"
                ),
            }
        }
        xdec("row122", ref_frame.as_ref().unwrap(), &src, None);
    }
}

// ===========================================================================
// Row 123 — LZ4F_compressBegin
// ===========================================================================

#[test]
fn row_123_compress_begin() {
    unsafe {
        let (c, r) = apis();
        let cc = new_cctx(c);
        let cr = new_cctx(r);

        let mut full = pref();
        full.frameInfo.blockSizeID = LZ4F_MAX1MB;
        full.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
        full.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
        full.frameInfo.contentSize = 123_456;
        full.frameInfo.dictID = 0xABCD_1234;
        full.frameInfo.blockChecksumFlag = LZ4F_BLOCK_CHECKSUM_ENABLED;
        full.compressionLevel = 7;
        full.autoFlush = 1;
        full.favorDecSpeed = 1;

        for prefs in [None, Some(&full)] {
            let pp = optp(prefs);
            // dstCapacity < LZ4F_HEADER_SIZE_MAX
            for cap in 0usize..19 {
                let (ret, _) = duo(
                    &format!("row123 cap={cap} prefs={}", prefs.is_some()),
                    cap,
                    cc,
                    cr,
                    |a, cx, dp, capv, _| (a.compress_begin)(cx, dp, capv, pp),
                );
                assert_eq!(
                    ret,
                    err(11),
                    "row123: cap={cap} must be ERROR_dstMaxSize_tooSmall, got {}",
                    ret as isize
                );
            }
            // exactly 19
            let (ret, out) = duo(
                &format!("row123 cap=19 prefs={}", prefs.is_some()),
                19,
                cc,
                cr,
                |a, cx, dp, capv, _| (a.compress_begin)(cx, dp, capv, pp),
            );
            assert!(!is_err_range(ret), "row123: cap=19 failed {}", ret as isize);
            let h = parse_header(&out);
            assert_eq!(h.size, ret, "row123: header size == return value");
            if prefs.is_none() {
                assert_eq!(ret, 7, "row123: default header is 7 bytes");
                assert_eq!(out[4], 0x40, "row123: default FLG");
                assert_eq!(out[5], 0x40, "row123: default BD");
            } else {
                assert_eq!(ret, 19, "row123: fully populated header is 19 bytes");
                assert_eq!(h.content_size, 123_456);
                assert_eq!(h.dict_id, 0xABCD_1234);
                assert_eq!((h.bd >> 4) & 7, 6, "row123: max1MB");
            }
            // larger capacity: same bytes
            let (ret2, out2) = duo(
                &format!("row123 cap=64 prefs={}", prefs.is_some()),
                64,
                cc,
                cr,
                |a, cx, dp, capv, _| (a.compress_begin)(cx, dp, capv, pp),
            );
            assert_eq!(ret, ret2);
            assert_eq!(out, out2, "row123: header must not depend on dstCapacity");
        }
        assert_eq!((c.free_cctx)(cc), 0);
        assert_eq!((r.free_cctx)(cr), 0);
    }
}

// ===========================================================================
// Row 124 — begin/update/end with autoFlush = 1, blockLinked
// ===========================================================================

#[test]
fn row_124_pipeline_autoflush_on() {
    let mut rng = Rng::new(124);
    unsafe {
        for iter in 0..1500usize {
            let bsid = [LZ4F_MAX64KB, LZ4F_MAX256KB][iter % 2];
            let level = [0i32, 1, 2, 9, -3][iter % 5];
            let mut p = pref();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = LZ4F_BLOCK_LINKED;
            p.frameInfo.contentChecksumFlag = (iter % 3 == 0) as c_uint;
            p.frameInfo.blockChecksumFlag = (iter % 5 == 0) as c_uint;
            p.compressionLevel = level;
            p.autoFlush = 1;
            let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
            let total = rng.range(1, 300_000);
            let src = gen(&mut rng, shape, total);
            let mut steps: Vec<Step> = vec![Step::Update(0)];
            let mut left = total;
            while left > 0 && steps.len() < 50 {
                let n = rng.range(0, left.min(90_000));
                steps.push(Step::Update(n));
                left -= n;
            }
            if left > 0 {
                steps.push(Step::Update(left));
            }
            steps.push(Step::Update(0));
            let copt = LZ4F_compressOptions_t::default(); // stableSrc = 0
            let ctx = format!("row124 iter={iter} bsid={bsid} lvl={level} total={total} {shape:?}");
            let (frame, used) =
                run_pipeline(&ctx, &src, &steps, Some(&p), Some(&copt), Begin::Plain);
            assert_eq!(used, total, "{ctx}: all input consumed");
            xdec(&ctx, &frame, &src, None);
            // autoFlush=1 => exactly ceil(total/blockSize) blocks
            let bs = bsid_to_size(bsid);
            let nb = blocks(&frame).len();
            assert!(nb >= (total + bs - 1) / bs, "{ctx}: too few blocks ({nb})");
        }
    }
}

// ===========================================================================
// Row 125 — autoFlush = 0: residual input buffered in tmpIn
// ===========================================================================

#[test]
fn row_125_pipeline_autoflush_off() {
    let mut rng = Rng::new(125);
    unsafe {
        for iter in 0..700usize {
            let bsid = [LZ4F_MAX64KB, LZ4F_MAX256KB][iter % 2];
            let bs = bsid_to_size(bsid);
            let level = [0i32, 1, 2, 9][iter % 4];
            let mode = [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT][iter % 2];
            let mut p = pref();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = mode;
            p.frameInfo.contentChecksumFlag = (iter % 2) as c_uint;
            p.frameInfo.blockChecksumFlag = (iter % 3 == 0) as c_uint;
            p.compressionLevel = level;
            p.autoFlush = 0;
            let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
            let src = gen(&mut rng, shape, 3 * bs + 12345);
            // several sub-blockSize updates, then one crossing blockSize
            let mut steps: Vec<Step> = Vec::new();
            for _ in 0..6 {
                steps.push(Step::Update(rng.range(1, bs / 4)));
            }
            steps.push(Step::Update(bs + 33));
            for _ in 0..4 {
                steps.push(Step::Update(rng.range(0, bs / 3)));
            }
            steps.push(Step::Update(bs * 2));
            let ctx = format!("row125 iter={iter} bsid={bsid} mode={mode} lvl={level} {shape:?}");
            let (frame, used) = run_pipeline(&ctx, &src, &steps, Some(&p), None, Begin::Plain);
            xdec(&ctx, &frame, &src[..used], None);
        }
    }
}

// ===========================================================================
// Row 126 — compressOptions stableSrc
// ===========================================================================

#[test]
fn row_126_stable_src() {
    let mut rng = Rng::new(126);
    unsafe {
        for iter in 0..400usize {
            let bsid = LZ4F_MAX64KB;
            let bs = bsid_to_size(bsid);
            let level = [0i32, 1, 2, 9][iter % 4];
            let mode = [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT][(iter / 4) % 2];
            let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
            let src = gen(&mut rng, shape, 6 * bs + 777);
            for stable in [0u32, 1] {
                let mut p = pref();
                p.frameInfo.blockSizeID = bsid;
                p.frameInfo.blockMode = mode;
                p.compressionLevel = level;
                p.autoFlush = 1;
                let mut copt = LZ4F_compressOptions_t::default();
                copt.stableSrc = stable;
                // updates larger than blockSize => lastBlockCompressed == fromSrcBuffer
                let steps = vec![
                    Step::Update(bs + 100),
                    Step::Update(2 * bs),
                    Step::Update(bs + 1),
                    Step::Update(bs - 1),
                    Step::Update(bs + 677),
                    Step::Update(usize::MAX),
                ];
                let ctx = format!(
                    "row126 iter={iter} stable={stable} mode={mode} lvl={level} {shape:?}"
                );
                let (frame, used) =
                    run_pipeline(&ctx, &src, &steps, Some(&p), Some(&copt), Begin::Plain);
                assert_eq!(used, src.len(), "{ctx}: all input consumed");
                xdec(&ctx, &frame, &src, None);
            }
        }
    }
}

// ===========================================================================
// Row 127 — autoFlush = 0 + blockLinked: forced localSaveDict to keep 64 KB
// ===========================================================================

#[test]
fn row_127_tmpbuff_wraparound() {
    let mut rng = Rng::new(127);
    unsafe {
        const REPS: usize = 18;
        for &level in [0i32, 1, 2, 9].iter().cycle().take(4 * REPS) {
            for shape in [Shape::Compressible, Shape::TextLike, Shape::Incompressible] {
                let mut p = pref();
                p.frameInfo.blockSizeID = LZ4F_MAX64KB;
                p.frameInfo.blockMode = LZ4F_BLOCK_LINKED;
                p.compressionLevel = level;
                p.autoFlush = 0;
                // 40 x 20000 bytes => ~12 blocks => several tmpBuff wraps
                let chunk = 20_000usize;
                let src = gen(&mut rng, shape, chunk * 40);
                let steps: Vec<Step> = (0..40).map(|_| Step::Update(chunk)).collect();
                let ctx = format!("row127 lvl={level} {shape:?}");
                let (frame, used) = run_pipeline(&ctx, &src, &steps, Some(&p), None, Begin::Plain);
                assert_eq!(used, src.len());
                xdec(&ctx, &frame, &src, None);
            }
        }
    }
}

// ===========================================================================
// Row 128 — compressUpdate srcSize 0 and exact / short dstCapacity
// ===========================================================================

#[test]
fn row_128_update_bounds() {
    let mut rng = Rng::new(128);
    unsafe {
        let (c, r) = apis();
        const REPS: usize = 12;
        for &level in [0i32, 2].iter().cycle().take(2 * REPS) {
            for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB] {
                for cc in [0u32, 1] {
                    for bc in [0u32, 1] {
                        let mut p = pref();
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.contentChecksumFlag = cc;
                        p.frameInfo.blockChecksumFlag = bc;
                        p.compressionLevel = level;
                        p.autoFlush = 1; // keeps tmpInSize == 0 so the bound is exact
                        let bs = bsid_to_size(bsid);
                        let src = gen(&mut rng, Shape::TextLike, 3 * bs);
                        for n in [0usize, 1, 100, bs - 1, bs, bs + 1, 2 * bs + 5] {
                            let cctx_c = new_cctx(c);
                            let cctx_r = new_cctx(r);
                            let ctx =
                                format!("row128 lvl={level} bsid={bsid} cc={cc} bc={bc} n={n}");
                            let pp = &p as *const LZ4F_preferences_t;
                            let (hb, _) = duo(&format!("{ctx} begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                                (a.compress_begin)(cx, dp, capv, pp)
                            });
                            assert!(!is_err_range(hb));
                            let bound = cbound(n, Some(&p));
                            let sp = src.as_ptr() as *const c_void;
                            // exactly the bound must succeed
                            let (ok, _) = duo(
                                &format!("{ctx} exact"),
                                bound,
                                cctx_c,
                                cctx_r,
                                |a, cx, dp, capv, _| {
                                    (a.compress_update)(cx, dp, capv, sp, n, ptr::null())
                                },
                            );
                            assert!(
                                !is_err_range(ok),
                                "{ctx}: dstCapacity == compressBound must succeed, got {}",
                                ok as isize
                            );
                            // one byte less must fail
                            if bound > 0 {
                                let (bad, _) = duo(
                                    &format!("{ctx} short"),
                                    bound - 1,
                                    cctx_c,
                                    cctx_r,
                                    |a, cx, dp, capv, _| {
                                        (a.compress_update)(cx, dp, capv, sp, n, ptr::null())
                                    },
                                );
                                assert_eq!(
                                    bad,
                                    err(11),
                                    "{ctx}: compressBound-1 must be ERROR_dstMaxSize_tooSmall, got {}",
                                    bad as isize
                                );
                            }
                            assert_eq!((c.free_cctx)(cctx_c), 0);
                            assert_eq!((r.free_cctx)(cctx_r), 0);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 129 — update / flush / end outside an initialized frame
// ===========================================================================

#[test]
fn row_129_state_uninitialized() {
    let mut rng = Rng::new(129);
    unsafe {
        let (c, r) = apis();
        let src = gen(&mut rng, Shape::TextLike, 5000);
        let sp = src.as_ptr() as *const c_void;

        for cc in [0u32, 1].iter().cycle().take(8).copied() {
            let mut p = pref();
            p.frameInfo.contentChecksumFlag = cc;
            p.autoFlush = 1;
            let pp = &p as *const LZ4F_preferences_t;

            let cctx_c = new_cctx(c);
            let cctx_r = new_cctx(r);

            // --- before LZ4F_compressBegin ---
            let (ru, _) = duo("row129 update-before-begin", 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_update)(cx, dp, capv, sp, 100, ptr::null())
            });
            assert_eq!(
                ru,
                err(20),
                "row129: compressUpdate before compressBegin must be compressionState_uninitialized, got {}",
                ru as isize
            );
            // NOTE: LZ4F_flush() short-circuits on tmpInSize == 0 *before* the
            // cStage check, so it returns 0 rather than an error.
            let (rf, _) = duo("row129 flush-before-begin", 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.flush)(cx, dp, capv, ptr::null())
            });
            assert_eq!(rf, 0, "row129: flush on a fresh cctx returns 0 in C");
            // LZ4F_compressEnd() likewise writes an endMark and succeeds.
            let (re, out) = duo("row129 end-before-begin", 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_end)(cx, dp, capv, ptr::null())
            });
            assert_eq!(re, 4, "row129: compressEnd on a fresh cctx writes just the endMark");
            assert_eq!(&out[..4], &[0, 0, 0, 0]);

            // --- a complete frame, then the same calls again ---
            let (hb, _) = duo("row129 begin", 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_begin)(cx, dp, capv, pp)
            });
            assert!(!is_err_range(hb));
            let bnd = cbound(src.len(), Some(&p));
            let (u1, _) = duo("row129 update", bnd, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_update)(cx, dp, capv, sp, src.len(), ptr::null())
            });
            assert!(!is_err_range(u1));
            let (e1, _) = duo("row129 end", cbound(0, Some(&p)).max(8), cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_end)(cx, dp, capv, ptr::null())
            });
            assert!(!is_err_range(e1));

            // after a completed compressEnd, cStage == 0 again
            let (ru2, _) = duo("row129 update-after-end", bnd, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_update)(cx, dp, capv, sp, 100, ptr::null())
            });
            assert_eq!(
                ru2,
                err(20),
                "row129: compressUpdate after compressEnd must be compressionState_uninitialized, got {}",
                ru2 as isize
            );
            let (rf2, _) = duo("row129 flush-after-end", 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.flush)(cx, dp, capv, ptr::null())
            });
            assert_eq!(rf2, 0, "row129: flush after compressEnd returns 0");
            let (re2, _) = duo("row129 end-after-end", 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_end)(cx, dp, capv, ptr::null())
            });
            assert_eq!(
                re2,
                4 + 4 * cc as usize,
                "row129: compressEnd after compressEnd re-emits the footer"
            );

            assert_eq!((c.free_cctx)(cctx_c), 0);
            assert_eq!((r.free_cctx)(cctx_r), 0);
        }
    }
}

// ===========================================================================
// Row 130 — LZ4F_flush
// ===========================================================================

#[test]
fn row_130_flush() {
    let mut rng = Rng::new(130);
    unsafe {
        let (c, r) = apis();
        const REPS: usize = 12;
        for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT]
            .iter()
            .cycle()
            .take(2 * REPS)
            .copied()
        {
            for &level in &[0i32, 1, 2, 9] {
                for bc in [0u32, 1] {
                    let mut p = pref();
                    p.frameInfo.blockSizeID = LZ4F_MAX64KB;
                    p.frameInfo.blockMode = mode;
                    p.frameInfo.blockChecksumFlag = bc;
                    p.compressionLevel = level;
                    p.autoFlush = 0;
                    let pp = &p as *const LZ4F_preferences_t;
                    let ctx = format!("row130 mode={mode} lvl={level} bc={bc}");
                    let src = gen(&mut rng, Shape::TextLike, 40_000);
                    let sp = src.as_ptr() as *const c_void;
                    let cctx_c = new_cctx(c);
                    let cctx_r = new_cctx(r);
                    let (hb, _) = duo(&format!("{ctx} begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.compress_begin)(cx, dp, capv, pp)
                    });
                    assert!(!is_err_range(hb));

                    // nothing buffered => 0
                    let (f0, _) = duo(&format!("{ctx} flush-empty"), 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.flush)(cx, dp, capv, ptr::null())
                    });
                    assert_eq!(f0, 0, "{ctx}: flush with nothing buffered must return 0");
                    // flush with dstCapacity 0 while nothing is buffered: still 0
                    let (f0b, _) = duo(&format!("{ctx} flush-empty-cap0"), 0, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.flush)(cx, dp, capv, ptr::null())
                    });
                    assert_eq!(f0b, 0, "{ctx}: flush(cap=0) with nothing buffered");

                    // buffer a partial block
                    let n = 12_345usize;
                    let bnd = cbound(n, Some(&p));
                    let (u1, _) = duo(&format!("{ctx} update"), bnd, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.compress_update)(cx, dp, capv, sp, n, ptr::null())
                    });
                    assert_eq!(u1, 0, "{ctx}: autoFlush=0 buffers the whole partial block");

                    // dstCapacity < tmpInSize + BHSize + BFSize => dstMaxSize_tooSmall
                    for cap in [0usize, 1, n, n + 7] {
                        let (bad, _) = duo(
                            &format!("{ctx} flush-short cap={cap}"),
                            cap,
                            cctx_c,
                            cctx_r,
                            |a, cx, dp, capv, _| (a.flush)(cx, dp, capv, ptr::null()),
                        );
                        assert_eq!(
                            bad,
                            err(11),
                            "{ctx}: flush cap={cap} must be ERROR_dstMaxSize_tooSmall, got {}",
                            bad as isize
                        );
                    }
                    // exactly tmpInSize + 8 succeeds
                    let (fok, _) = duo(&format!("{ctx} flush-exact"), n + 8, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.flush)(cx, dp, capv, ptr::null())
                    });
                    assert!(
                        !is_err_range(fok),
                        "{ctx}: flush with tmpInSize+8 must succeed, got {}",
                        fok as isize
                    );
                    assert!(fok > 0, "{ctx}: flush must emit a block");
                    // now nothing is buffered again
                    let (f1, _) = duo(&format!("{ctx} flush-again"), 4096, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.flush)(cx, dp, capv, ptr::null())
                    });
                    assert_eq!(f1, 0, "{ctx}: second flush must return 0");
                    let (e1, _) = duo(&format!("{ctx} end"), cbound(0, Some(&p)).max(8), cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.compress_end)(cx, dp, capv, ptr::null())
                    });
                    assert!(!is_err_range(e1));
                    assert_eq!((c.free_cctx)(cctx_c), 0);
                    assert_eq!((r.free_cctx)(cctx_r), 0);
                }
            }
        }

        // full pipelines with explicit flushes, byte-compared end to end
        for iter in 0..300usize {
            let bsid = [LZ4F_MAX64KB, LZ4F_MAX256KB][iter % 2];
            let bs = bsid_to_size(bsid);
            let mut p = pref();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT][(iter / 2) % 2];
            p.frameInfo.blockChecksumFlag = (iter % 3 == 0) as c_uint;
            p.frameInfo.contentChecksumFlag = (iter % 2) as c_uint;
            p.compressionLevel = [0i32, 2, 9][iter % 3];
            p.autoFlush = 0;
            let src = gen(&mut rng, ALL_SHAPES[iter % ALL_SHAPES.len()], 3 * bs);
            let steps = vec![
                Step::Update(bs / 3),
                Step::Flush,
                Step::Flush,
                Step::Update(bs / 2),
                Step::Update(bs / 2),
                Step::Flush,
                Step::Update(bs + 5),
                Step::Flush,
                Step::Update(usize::MAX),
                Step::Flush,
            ];
            let ctx = format!("row130 pipeline iter={iter}");
            let (frame, used) = run_pipeline(&ctx, &src, &steps, Some(&p), None, Begin::Plain);
            assert_eq!(used, src.len());
            xdec(&ctx, &frame, &src, None);
        }
    }
}

// ===========================================================================
// Row 131 — LZ4F_uncompressedUpdate
// ===========================================================================

#[test]
fn row_131_uncompressed_update() {
    let mut rng = Rng::new(131);
    unsafe {
        let (c, r) = apis();
        const REPS: usize = 24;
        for bsid in [LZ4F_MAX64KB, LZ4F_MAX256KB].iter().cycle().take(2 * REPS).copied() {
            let bs = bsid_to_size(bsid);
            for af in [0u32, 1] {
                for bc in [0u32, 1] {
                    let mut p = pref();
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
                    p.frameInfo.blockChecksumFlag = bc;
                    p.frameInfo.contentChecksumFlag = 1;
                    p.autoFlush = af;
                    let src = gen(&mut rng, Shape::TextLike, 3 * bs + 999);
                    let steps = vec![
                        Step::Uncompressed(0),
                        Step::Uncompressed(bs / 2),
                        Step::Uncompressed(bs),
                        Step::Uncompressed(bs + 1),
                        Step::Uncompressed(usize::MAX),
                    ];
                    let ctx = format!("row131 bsid={bsid} af={af} bc={bc}");
                    let (frame, used) =
                        run_pipeline(&ctx, &src, &steps, Some(&p), None, Begin::Plain);
                    assert_eq!(used, src.len());
                    // every block must be stored verbatim
                    for &(stored, _, _) in &blocks(&frame) {
                        assert!(stored, "{ctx}: uncompressedUpdate must emit stored blocks");
                    }
                    xdec(&ctx, &frame, &src, None);

                    // short dstCapacity
                    let cctx_c = new_cctx(c);
                    let cctx_r = new_cctx(r);
                    let pp = &p as *const LZ4F_preferences_t;
                    let (hb, _) = duo(&format!("{ctx} begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.compress_begin)(cx, dp, capv, pp)
                    });
                    assert!(!is_err_range(hb));
                    let n = 5000usize;
                    let sp = src.as_ptr() as *const c_void;
                    let bnd = cbound(n, Some(&p));
                    // Note: with autoFlush == 0 the *internal* requirement is only
                    // LZ4F_compressBound_internal(srcSize, prefs, tmpInSize==0),
                    // which is far below LZ4F_compressBound()'s worst case, so
                    // short capacities legitimately succeed there. Only the
                    // autoFlush == 1 case has an exact bound.
                    if af == 1 {
                        for cap in [0usize, 1, n / 2, n, bnd - 1] {
                            let (bad, _) = duo(
                                &format!("{ctx} short cap={cap}"),
                                cap,
                                cctx_c,
                                cctx_r,
                                |a, cx, dp, capv, _| {
                                    (a.uncompressed_update)(cx, dp, capv, sp, n, ptr::null())
                                },
                            );
                            assert_eq!(
                                bad,
                                err(11),
                                "{ctx}: uncompressedUpdate cap={cap} must be dstMaxSize_tooSmall, got {}",
                                bad as isize
                            );
                        }
                    } else {
                        // still compare C vs Rust across the whole capacity range
                        for cap in [0usize, 1, n / 2, n, n + 8, bnd - 1] {
                            let _ = duo(
                                &format!("{ctx} cap={cap}"),
                                cap,
                                cctx_c,
                                cctx_r,
                                |a, cx, dp, capv, _| {
                                    (a.uncompressed_update)(cx, dp, capv, sp, n, ptr::null())
                                },
                            );
                        }
                    }
                    let (ok, _) = duo(
                        &format!("{ctx} exact"),
                        bnd,
                        cctx_c,
                        cctx_r,
                        |a, cx, dp, capv, _| {
                            (a.uncompressed_update)(cx, dp, capv, sp, n, ptr::null())
                        },
                    );
                    assert!(!is_err_range(ok), "{ctx}: bound-sized dst must succeed");
                    let (e1, _) = duo(&format!("{ctx} end"), cbound(0, Some(&p)).max(8), cctx_c, cctx_r, |a, cx, dp, capv, _| {
                        (a.compress_end)(cx, dp, capv, ptr::null())
                    });
                    assert!(!is_err_range(e1));
                    assert_eq!((c.free_cctx)(cctx_c), 0);
                    assert_eq!((r.free_cctx)(cctx_r), 0);
                }
            }
        }
    }
}

// ===========================================================================
// Row 132 — compressUpdate interleaved with uncompressedUpdate
// ===========================================================================

#[test]
fn row_132_interleaved_compressed_uncompressed() {
    let mut rng = Rng::new(132);
    unsafe {
        for iter in 0..600usize {
            let bsid = [LZ4F_MAX64KB, LZ4F_MAX256KB][iter % 2];
            let bs = bsid_to_size(bsid);
            let mut p = pref();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
            p.frameInfo.blockChecksumFlag = (iter % 3 == 0) as c_uint;
            p.frameInfo.contentChecksumFlag = (iter % 2) as c_uint;
            p.compressionLevel = [0i32, 1, 2, 9][iter % 4];
            p.autoFlush = (iter / 4 % 2) as c_uint;
            let shape = ALL_SHAPES[iter % ALL_SHAPES.len()];
            let src = gen(&mut rng, shape, 4 * bs);
            // switching compression mode forces an implicit flush of tmpIn
            let mut steps: Vec<Step> = Vec::new();
            for k in 0..12 {
                let n = rng.range(0, bs / 2 + 100);
                if k % 2 == 0 {
                    steps.push(Step::Update(n));
                } else {
                    steps.push(Step::Uncompressed(n));
                }
            }
            steps.push(Step::Uncompressed(bs + 7));
            steps.push(Step::Update(bs + 9));
            steps.push(Step::Flush);
            steps.push(Step::Uncompressed(100));
            steps.push(Step::Update(200));
            let ctx = format!("row132 iter={iter} bsid={bsid}");
            let (frame, used) = run_pipeline(&ctx, &src, &steps, Some(&p), None, Begin::Plain);
            xdec(&ctx, &frame, &src[..used], None);
            let bl = blocks(&frame);
            assert!(
                bl.iter().any(|b| b.0),
                "{ctx}: expected at least one stored block"
            );
        }
    }
}

// ===========================================================================
// Row 133 — LZ4F_compressEnd error paths
// ===========================================================================

#[test]
fn row_133_compress_end_errors() {
    let mut rng = Rng::new(133);
    unsafe {
        let (c, r) = apis();
        let src = gen(&mut rng, Shape::TextLike, 10_000);
        let sp = src.as_ptr() as *const c_void;

        // (a) declared contentSize != total fed => frameSize_wrong
        for &(declared, fed) in &[
            (1000u64, 999usize),
            (1000, 1001),
            (1, 0),
            (10_000, 5000),
            (5000, 10_000),
        ] {
            for cc in [0u32, 1] {
                let mut p = pref();
                p.frameInfo.contentSize = declared;
                p.frameInfo.contentChecksumFlag = cc;
                p.autoFlush = 1;
                let pp = &p as *const LZ4F_preferences_t;
                let cctx_c = new_cctx(c);
                let cctx_r = new_cctx(r);
                let ctx = format!("row133 declared={declared} fed={fed} cc={cc}");
                let (hb, _) = duo(&format!("{ctx} begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                    (a.compress_begin)(cx, dp, capv, pp)
                });
                assert!(!is_err_range(hb));
                let bnd = cbound(fed, Some(&p));
                let (u, _) = duo(&format!("{ctx} update"), bnd, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                    (a.compress_update)(cx, dp, capv, sp, fed, ptr::null())
                });
                assert!(!is_err_range(u));
                let (e, _) = duo(
                    &format!("{ctx} end"),
                    cbound(0, Some(&p)).max(8),
                    cctx_c,
                    cctx_r,
                    |a, cx, dp, capv, _| (a.compress_end)(cx, dp, capv, ptr::null()),
                );
                assert_eq!(
                    e,
                    err(14),
                    "{ctx}: expected ERROR_frameSize_wrong, got {}",
                    e as isize
                );
                assert_eq!((c.free_cctx)(cctx_c), 0);
                assert_eq!((r.free_cctx)(cctx_r), 0);
            }
        }

        // (b) dstCapacity < 4, and < 8 with a content checksum
        for cc in [0u32, 1] {
            let mut p = pref();
            p.frameInfo.contentChecksumFlag = cc;
            p.autoFlush = 1;
            let pp = &p as *const LZ4F_preferences_t;
            let cctx_c = new_cctx(c);
            let cctx_r = new_cctx(r);
            let ctx = format!("row133 short cc={cc}");
            let (hb, _) = duo(&format!("{ctx} begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_begin)(cx, dp, capv, pp)
            });
            assert!(!is_err_range(hb));
            let bnd = cbound(1000, Some(&p));
            let (u, _) = duo(&format!("{ctx} update"), bnd, cctx_c, cctx_r, |a, cx, dp, capv, _| {
                (a.compress_update)(cx, dp, capv, sp, 1000, ptr::null())
            });
            assert!(!is_err_range(u));
            let limit = 4 + 4 * cc as usize;
            for cap in 0..limit {
                let (bad, _) = duo(
                    &format!("{ctx} end cap={cap}"),
                    cap,
                    cctx_c,
                    cctx_r,
                    |a, cx, dp, capv, _| (a.compress_end)(cx, dp, capv, ptr::null()),
                );
                assert_eq!(
                    bad,
                    err(11),
                    "{ctx}: end cap={cap} must be ERROR_dstMaxSize_tooSmall, got {}",
                    bad as isize
                );
            }
            let (ok, _) = duo(
                &format!("{ctx} end cap={limit}"),
                limit,
                cctx_c,
                cctx_r,
                |a, cx, dp, capv, _| (a.compress_end)(cx, dp, capv, ptr::null()),
            );
            assert_eq!(ok, limit, "{ctx}: end with exactly {limit} bytes");
            assert_eq!((c.free_cctx)(cctx_c), 0);
            assert_eq!((r.free_cctx)(cctx_r), 0);
        }
    }
}

// ===========================================================================
// Row 134 — cctx reuse across frames (ctx type switch + tmpBuff realloc)
// ===========================================================================

#[test]
fn row_134_cctx_reuse() {
    let mut rng = Rng::new(134);
    unsafe {
        let (c, r) = apis();
        // levels crossing LZ4HC_CLEVEL_MIN in both directions, growing blockSizeID
        let plans: &[[(c_int, c_uint); 3]] = &[
            [
                (0, LZ4F_MAX64KB),
                (9, LZ4F_MAX256KB),
                (1, LZ4F_MAX1MB),
            ],
            [
                (12, LZ4F_MAX64KB),
                (0, LZ4F_MAX256KB),
                (2, LZ4F_MAX1MB),
            ],
            [
                (1, LZ4F_MAX64KB),
                (2, LZ4F_MAX64KB),
                (1, LZ4F_MAX256KB),
            ],
            [
                (9, LZ4F_MAX256KB),
                (-5, LZ4F_MAX256KB),
                (10, LZ4F_MAX1MB),
            ],
        ];
        const REPS: usize = 8;
        for (pi, plan) in plans.iter().cycle().take(plans.len() * REPS).enumerate() {
            for af in [0u32, 1] {
                for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    // one shared cctx per library, three frames on it
                    let cctx_c = new_cctx(c);
                    let cctx_r = new_cctx(r);
                    let mut reused: Vec<Vec<u8>> = Vec::new();
                    let mut fresh: Vec<Vec<u8>> = Vec::new();
                    for (fi, &(level, bsid)) in plan.iter().enumerate() {
                        let mut p = pref();
                        p.compressionLevel = level;
                        p.frameInfo.blockSizeID = bsid;
                        p.frameInfo.blockMode = mode;
                        p.autoFlush = af;
                        let bs = bsid_to_size(bsid);
                        let n = (2 * bs + 1234).min(600_000);
                        let src = gen(&mut rng, ALL_SHAPES[fi % ALL_SHAPES.len()], n);
                        let ctx = format!(
                            "row134 plan={pi} af={af} mode={mode} frame={fi} lvl={level} bsid={bsid}"
                        );
                        // frame on the reused cctx
                        let f_reused =
                            drive_one_frame(&ctx, cctx_c, cctx_r, &src, &p, &format!("{ctx} reused"));
                        // the same frame on brand-new contexts
                        let nc = new_cctx(c);
                        let nr = new_cctx(r);
                        let f_fresh =
                            drive_one_frame(&ctx, nc, nr, &src, &p, &format!("{ctx} fresh"));
                        assert_eq!((c.free_cctx)(nc), 0);
                        assert_eq!((r.free_cctx)(nr), 0);
                        assert_eq!(
                            f_reused, f_fresh,
                            "{ctx}: reused cctx produced a different frame than a fresh cctx"
                        );
                        xdec(&ctx, &f_reused, &src, None);
                        reused.push(f_reused);
                        fresh.push(f_fresh);
                    }
                    assert_eq!(reused, fresh);
                    assert_eq!((c.free_cctx)(cctx_c), 0);
                    assert_eq!((r.free_cctx)(cctx_r), 0);
                }
            }
        }
    }
}

/// begin + one big update + end on already-created contexts; returns the frame.
#[track_caller]
unsafe fn drive_one_frame(
    _ctx: &str,
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
    src: &[u8],
    p: &LZ4F_preferences_t,
    tag: &str,
) -> Vec<u8> {
    let pp = p as *const LZ4F_preferences_t;
    let mut frame = Vec::new();
    let (hb, out) = duo(&format!("{tag}: begin"), 19, cctx_c, cctx_r, |a, cx, dp, capv, _| {
        (a.compress_begin)(cx, dp, capv, pp)
    });
    assert!(!is_err_range(hb), "{tag}: begin failed {}", hb as isize);
    frame.extend_from_slice(&out);
    let sp = src.as_ptr() as *const c_void;
    let n = src.len();
    let cap = cbound(n, Some(p)).max(n) + 8;
    let (ru, out) = duo(&format!("{tag}: update"), cap, cctx_c, cctx_r, |a, cx, dp, capv, _| {
        (a.compress_update)(cx, dp, capv, sp, n, ptr::null())
    });
    assert!(!is_err_range(ru), "{tag}: update failed {}", ru as isize);
    frame.extend_from_slice(&out);
    let cap = cbound(0, Some(p)).max(8);
    let (re, out) = duo(&format!("{tag}: end"), cap, cctx_c, cctx_r, |a, cx, dp, capv, _| {
        (a.compress_end)(cx, dp, capv, ptr::null())
    });
    assert!(!is_err_range(re), "{tag}: end failed {}", re as isize);
    frame.extend_from_slice(&out);
    frame
}

// ===========================================================================
// Row 135 — LZ4F_compressBegin_usingDict / _usingDictOnce
// ===========================================================================

#[test]
fn row_135_begin_using_dict() {
    let mut rng = Rng::new(135);
    unsafe {
        const REPS: usize = 8;
        for dsize in [0usize, 1, 100, 65535, 65536, 65537, 150_000]
            .iter()
            .cycle()
            .take(7 * REPS)
            .copied()
        {
            let dict = gen(&mut rng, Shape::TextLike, dsize);
            for &level in &[0i32, 1, 2, 9] {
                for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    for use_prefs in [false, true] {
                        for n in [0usize, 1, 1000, 70_000, 200_000] {
                            let src = gen(&mut rng, Shape::TextLike, n);
                            let mut p = pref();
                            p.compressionLevel = level;
                            p.frameInfo.blockMode = mode;
                            p.frameInfo.blockSizeID = LZ4F_MAX64KB;
                            p.frameInfo.dictID = 0xFEED;
                            p.autoFlush = 1;
                            let prefs = if use_prefs { Some(&p) } else { None };
                            let steps = vec![
                                Step::Update(n / 2),
                                Step::Update(usize::MAX),
                                Step::Update(0),
                            ];
                            for once in [false, true] {
                                let ctx = format!(
                                    "row135 d={dsize} lvl={level} mode={mode} prefs={use_prefs} n={n} once={once}"
                                );
                                let b = if once {
                                    Begin::UsingDictOnce(&dict)
                                } else {
                                    Begin::UsingDict(&dict)
                                };
                                let (frame, used) =
                                    run_pipeline(&ctx, &src, &steps, prefs, None, b);
                                assert_eq!(used, n);
                                if use_prefs {
                                    let h = parse_header(&frame);
                                    assert_eq!(h.dict_id, 0xFEED, "{ctx}: dictID in header");
                                } else {
                                    let h = parse_header(&frame);
                                    assert_eq!(h.dict_id, 0, "{ctx}: no dictID without prefs");
                                }
                                xdec(&ctx, &frame, &src, Some(&dict));
                            }
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 136 — LZ4F_compressBegin_usingCDict
// ===========================================================================

#[test]
fn row_136_begin_using_cdict() {
    let mut rng = Rng::new(136);
    unsafe {
        const REPS: usize = 12;
        for dsize in [0usize, 100, 65536, 150_000].iter().cycle().take(4 * REPS).copied() {
            let dict = gen(&mut rng, Shape::TextLike, dsize);
            for &level in &[0i32, 1, 2, 12] {
                for mode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
                    for use_prefs in [false, true] {
                        for n in [0usize, 1, 1000, 70_000, 200_000] {
                            if level >= 12 && n > 100_000 {
                                continue;
                            }
                            let src = gen(&mut rng, Shape::TextLike, n);
                            let mut p = pref();
                            p.compressionLevel = level;
                            p.frameInfo.blockMode = mode;
                            p.frameInfo.blockSizeID = LZ4F_MAX64KB;
                            p.frameInfo.dictID = 0xC0FFEE;
                            p.autoFlush = (n % 2) as c_uint;
                            let prefs = if use_prefs { Some(&p) } else { None };
                            let steps = vec![
                                Step::Update(n / 3),
                                Step::Update(usize::MAX),
                            ];
                            let ctx = format!(
                                "row136 d={dsize} lvl={level} mode={mode} prefs={use_prefs} n={n}"
                            );
                            let (frame, used) = run_pipeline(
                                &ctx,
                                &src,
                                &steps,
                                prefs,
                                None,
                                Begin::UsingCDict(&dict),
                            );
                            assert_eq!(used, n);
                            let h = parse_header(&frame);
                            assert_eq!(
                                h.dict_id,
                                if use_prefs { 0xC0FFEE } else { 0 },
                                "{ctx}: dictID"
                            );
                            xdec(&ctx, &frame, &src, Some(&dict));
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 137 — LZ4F_getBlockSize / getVersion / compressionLevel_max
// ===========================================================================

#[test]
fn row_137_get_block_size_version_level_max() {
    unsafe {
        let (c, r) = apis();
        let cases: &[(c_uint, usize)] = &[
            (0, 64 * 1024),
            (1, err(2)),
            (2, err(2)),
            (3, err(2)),
            (4, 64 * 1024),
            (5, 256 * 1024),
            (6, 1024 * 1024),
            (7, 4 * 1024 * 1024),
            (8, err(2)),
            (9, err(2)),
            (100, err(2)),
            (0x7FFF_FFFF, err(2)),
            (0x8000_0000, err(2)),
            (u32::MAX, err(2)),
        ];
        for &(id, want) in cases {
            let a = (c.get_block_size)(id);
            let b = (r.get_block_size)(id);
            assert_eq!(
                a as isize, b as isize,
                "LZ4F_getBlockSize({id}) mismatch (C={} Rust={})",
                a as isize, b as isize
            );
            assert_eq!(
                a as isize, want as isize,
                "LZ4F_getBlockSize({id}) value (C={}, expected {})",
                a as isize, want as isize
            );
        }
        assert_eq!((c.get_version)(), (r.get_version)());
        assert_eq!((c.get_version)(), LZ4F_VERSION);
        assert_eq!((c.level_max)(), (r.level_max)());
        assert_eq!((c.level_max)(), 12);
    }
}

// ===========================================================================
// Row 138 — isError / getErrorName / getErrorCode
// ===========================================================================

#[test]
fn row_138_error_api() {
    unsafe {
        let (c, r) = apis();
        let mut vals: Vec<usize> = vec![
            0,
            1,
            2,
            1000,
            usize::MAX / 2,
            usize::MAX / 2 + 1,
            0x7FFF_FFFF,
        ];
        for code in 1..=24usize {
            vals.push(err(code));
        }
        vals.push(err(25));
        vals.push(err(26));
        vals.push(err(100));
        vals.push(err(1000));
        for &v in &vals {
            let ia = (c.is_error)(v);
            let ib = (r.is_error)(v);
            assert_eq!(ia, ib, "LZ4F_isError({}) mismatch", v as isize);
            // mirror of lz4frame.c: code > (size_t)(-LZ4F_ERROR_maxCode)
            let want = (v > err(24)) as c_uint;
            assert_eq!(
                ia != 0,
                want != 0,
                "LZ4F_isError({}) value (C={ia})",
                v as isize
            );

            let na = (c.get_error_name)(v);
            let nb = (r.get_error_name)(v);
            assert!(!na.is_null() && !nb.is_null(), "getErrorName NULL for {}", v as isize);
            let sa = CStr::from_ptr(na).to_bytes();
            let sb = CStr::from_ptr(nb).to_bytes();
            assert_eq!(
                sa, sb,
                "LZ4F_getErrorName({}) differs: C={:?} Rust={:?}",
                v as isize,
                String::from_utf8_lossy(sa),
                String::from_utf8_lossy(sb)
            );
            if ia == 0 {
                assert_eq!(
                    sa, b"Unspecified error code",
                    "getErrorName for the non-error {}",
                    v as isize
                );
            }

            let ca = (c.get_error_code)(v);
            let cb = (r.get_error_code)(v);
            assert_eq!(ca, cb, "LZ4F_getErrorCode({}) mismatch", v as isize);
            let want_code = if ia == 0 { 0 } else { -(v as isize) as c_int };
            assert_eq!(
                ca, want_code,
                "LZ4F_getErrorCode({}) value",
                v as isize
            );
        }
        // a few well-known names
        for (code, name) in [
            (1usize, "ERROR_GENERIC"),
            (2, "ERROR_maxBlockSize_invalid"),
            (11, "ERROR_dstMaxSize_tooSmall"),
            (12, "ERROR_frameHeader_incomplete"),
            (14, "ERROR_frameSize_wrong"),
            (15, "ERROR_srcPtr_wrong"),
            (19, "ERROR_frameDecoding_alreadyStarted"),
            (20, "ERROR_compressionState_uninitialized"),
        ] {
            let s = CStr::from_ptr((c.get_error_name)(err(code)))
                .to_str()
                .unwrap()
                .to_string();
            let t = CStr::from_ptr((r.get_error_name)(err(code)))
                .to_str()
                .unwrap()
                .to_string();
            assert_eq!(s, name, "C name for err({code})");
            assert_eq!(t, name, "Rust name for err({code})");
        }
    }
}

// ===========================================================================
// Row 139 — LZ4F_headerSize
// ===========================================================================

#[test]
fn row_139_header_size() {
    unsafe {
        let (c, r) = apis();
        let check = |src: *const c_void, n: usize, want: usize, what: &str| {
            let a = (c.header_size)(src, n);
            let b = (r.header_size)(src, n);
            assert_eq!(
                a as isize, b as isize,
                "LZ4F_headerSize({what}, {n}) mismatch (C={} Rust={})",
                a as isize, b as isize
            );
            assert_eq!(
                a as isize, want as isize,
                "LZ4F_headerSize({what}, {n}) value (C={}, expected {})",
                a as isize, want as isize
            );
        };

        // src == NULL => srcPtr_wrong, checked before srcSize
        for n in [0usize, 4, 5, 19, usize::MAX] {
            check(ptr::null(), n, err(15), "NULL");
        }

        let mut buf = [0u8; 32];
        buf[..4].copy_from_slice(&MAGIC);
        buf[4] = 0x40; // version 01, no optional fields
        // srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH
        for n in 0usize..5 {
            check(buf.as_ptr() as *const c_void, n, err(12), "plain");
        }
        // plain frame => 7
        for n in [5usize, 6, 7, 19, 32] {
            check(buf.as_ptr() as *const c_void, n, 7, "plain");
        }
        // + contentSize => 15
        buf[4] = 0x40 | 0x08;
        check(buf.as_ptr() as *const c_void, 5, 15, "contentSize");
        // + dictID => 11
        buf[4] = 0x40 | 0x01;
        check(buf.as_ptr() as *const c_void, 5, 11, "dictID");
        // both => 19
        buf[4] = 0x40 | 0x08 | 0x01;
        check(buf.as_ptr() as *const c_void, 5, 19, "both");
        // the FLG version bits are NOT validated by LZ4F_headerSize
        buf[4] = 0x00;
        check(buf.as_ptr() as *const c_void, 5, 7, "version 0");

        // skippable magic 0x184D2A50 .. 0x184D2A5F => 8
        for lo in 0x50u8..=0x5F {
            let mut sk = [0u8; 8];
            sk[0] = lo;
            sk[1] = 0x2A;
            sk[2] = 0x4D;
            sk[3] = 0x18;
            sk[4] = 0xFF;
            check(sk.as_ptr() as *const c_void, 5, 8, "skippable");
            check(sk.as_ptr() as *const c_void, 8, 8, "skippable");
        }
        // just outside the skippable range => frameType_unknown
        for lo in [0x4Fu8, 0x60] {
            let mut sk = [0u8; 8];
            sk[0] = lo;
            sk[1] = 0x2A;
            sk[2] = 0x4D;
            sk[3] = 0x18;
            check(sk.as_ptr() as *const c_void, 8, err(13), "near-skippable");
        }
        // bad magic => frameType_unknown
        for bad in [
            [0u8, 0, 0, 0],
            [0x04, 0x22, 0x4D, 0x19],
            [0x05, 0x22, 0x4D, 0x18],
            [0xFF, 0xFF, 0xFF, 0xFF],
        ] {
            let mut b = [0u8; 8];
            b[..4].copy_from_slice(&bad);
            check(b.as_ptr() as *const c_void, 8, err(13), "bad magic");
        }
    }
}

// ===========================================================================
// Row 140 — LZ4F_getFrameInfo
// ===========================================================================

#[test]
fn row_140_get_frame_info() {
    let mut rng = Rng::new(140);
    unsafe {
        let (c, r) = apis();

        for (bsid, mode, cc, bc, csize, did) in [
            (LZ4F_MAX64KB, LZ4F_BLOCK_INDEPENDENT, 0u32, 0u32, 0u64, 0u32),
            (LZ4F_MAX256KB, LZ4F_BLOCK_LINKED, 1, 0, 0, 0x1234),
            (LZ4F_MAX1MB, LZ4F_BLOCK_LINKED, 0, 1, 1, 0),
            (LZ4F_MAX4MB, LZ4F_BLOCK_INDEPENDENT, 1, 1, 1, 0xABCDEF),
        ] {
            let bs = bsid_to_size(bsid);
            let n = (2 * bs + 4321).min(600_000);
            let src = gen(&mut rng, Shape::TextLike, n);
            let mut p = pref();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = mode;
            p.frameInfo.contentChecksumFlag = cc;
            p.frameInfo.blockChecksumFlag = bc;
            p.frameInfo.contentSize = csize;
            p.frameInfo.dictID = did;
            let cap = cfbound(n, Some(&p));
            let ctx = format!("row140 bsid={bsid} mode={mode} cc={cc} bc={bc} cs={csize} did={did}");
            let (ret, frame) = cf(&ctx, &src, cap, Some(&p));
            assert!(!is_err_range(ret));
            let h = parse_header(&frame);

            // ---- 1) fresh dctx with >= headerSize bytes ----
            for extra in [0usize, 1, 4, 100] {
                let give = (h.size + extra).min(frame.len());
                let mut fic = LZ4F_frameInfo_t::default();
                let mut fir = LZ4F_frameInfo_t::default();
                let mut sc = give;
                let mut sr = give;
                let dc = new_dctx(c);
                let dr = new_dctx(r);
                let a = (c.get_frame_info)(dc, &mut fic, frame.as_ptr() as *const c_void, &mut sc);
                let b = (r.get_frame_info)(dr, &mut fir, frame.as_ptr() as *const c_void, &mut sr);
                assert_eq!(
                    a as isize, b as isize,
                    "{ctx}: getFrameInfo(give={give}) return mismatch"
                );
                assert_eq!(sc, sr, "{ctx}: *srcSizePtr mismatch");
                assert_eq!(fic, fir, "{ctx}: frameInfo struct mismatch");
                assert_eq!(a, 4, "{ctx}: hint must be BHSize (4)");
                assert_eq!(sc, h.size, "{ctx}: header consumed");
                let stored_bsid = ((h.bd >> 4) & 7) as u32;
                assert_eq!(fic.blockSizeID, stored_bsid, "{ctx}: blockSizeID");
                assert_eq!(fic.blockMode, ((h.flg >> 5) & 1) as u32, "{ctx}: blockMode");
                assert_eq!(fic.contentChecksumFlag, cc, "{ctx}: contentChecksumFlag");
                assert_eq!(fic.blockChecksumFlag, bc, "{ctx}: blockChecksumFlag");
                assert_eq!(fic.frameType, LZ4F_FRAME, "{ctx}: frameType");
                assert_eq!(
                    fic.contentSize,
                    if csize != 0 { n as u64 } else { 0 },
                    "{ctx}: contentSize"
                );
                assert_eq!(fic.dictID, did, "{ctx}: dictID");
                // dStage after consuming the header == dstage_init == 2
                assert_eq!((c.free_dctx)(dc), 2, "{ctx}: C dStage after header");
                assert_eq!((r.free_dctx)(dr), 2, "{ctx}: Rust dStage after header");
            }

            // ---- 2) fewer bytes than the header => frameHeader_incomplete, *srcSizePtr = 0 ----
            for give in 0..h.size {
                let mut fic = LZ4F_frameInfo_t::default();
                let mut fir = LZ4F_frameInfo_t::default();
                let mut sc = give;
                let mut sr = give;
                let dc = new_dctx(c);
                let dr = new_dctx(r);
                let a = (c.get_frame_info)(dc, &mut fic, frame.as_ptr() as *const c_void, &mut sc);
                let b = (r.get_frame_info)(dr, &mut fir, frame.as_ptr() as *const c_void, &mut sr);
                assert_eq!(a as isize, b as isize, "{ctx}: short getFrameInfo({give})");
                assert_eq!(
                    a,
                    err(12),
                    "{ctx}: give={give} must be frameHeader_incomplete, got {}",
                    a as isize
                );
                assert_eq!(sc, 0, "{ctx}: *srcSizePtr must be 0 on failure (C)");
                assert_eq!(sr, 0, "{ctx}: *srcSizePtr must be 0 on failure (Rust)");
                assert_eq!(fic, fir, "{ctx}: frameInfo after failure");
                // dctx untouched => dStage still 0
                assert_eq!((c.free_dctx)(dc), 0);
                assert_eq!((r.free_dctx)(dr), 0);
            }

            // ---- 3) dctx stopped mid-header => frameDecoding_alreadyStarted ----
            if h.size > 7 {
                let dc = new_dctx(c);
                let dr = new_dctx(r);
                let give = h.size - 1; // >= 7 but < headerSize
                let mut out = vec![0u8; 64];
                for (a, dx) in [(c, dc), (r, dr)] {
                    let mut dsz = out.len();
                    let mut ssz = give;
                    let hint = (a.decompress)(
                        dx,
                        out.as_mut_ptr() as *mut c_void,
                        &mut dsz,
                        frame.as_ptr() as *const c_void,
                        &mut ssz,
                        ptr::null(),
                    );
                    assert!(!is_err_range(hint), "{ctx}: {} partial header decode", a.tag);
                }
                let mut fic = LZ4F_frameInfo_t::default();
                let mut fir = LZ4F_frameInfo_t::default();
                let mut sc = frame.len();
                let mut sr = frame.len();
                let a = (c.get_frame_info)(dc, &mut fic, frame.as_ptr() as *const c_void, &mut sc);
                let b = (r.get_frame_info)(dr, &mut fir, frame.as_ptr() as *const c_void, &mut sr);
                assert_eq!(a as isize, b as isize, "{ctx}: mid-header getFrameInfo");
                assert_eq!(
                    a,
                    err(19),
                    "{ctx}: mid-header must be frameDecoding_alreadyStarted, got {}",
                    a as isize
                );
                assert_eq!(sc, 0);
                assert_eq!(sr, 0);
                // dStage == dstage_storeFrameHeader == 1
                assert_eq!((c.free_dctx)(dc), 1, "{ctx}: C dStage mid-header");
                assert_eq!((r.free_dctx)(dr), 1, "{ctx}: Rust dStage mid-header");
            }

            // ---- 4) after decoding has started => cached frameInfo + next-size hint ----
            {
                let dc = new_dctx(c);
                let dr = new_dctx(r);
                let bsz = frame_block_size(&frame);
                let mut out = vec![0u8; bsz + 16];
                let mut hints = Vec::new();
                for (a, dx) in [(c, dc), (r, dr)] {
                    let mut dsz = out.len();
                    let mut ssz = (h.size + 8).min(frame.len());
                    let hint = (a.decompress)(
                        dx,
                        out.as_mut_ptr() as *mut c_void,
                        &mut dsz,
                        frame.as_ptr() as *const c_void,
                        &mut ssz,
                        ptr::null(),
                    );
                    assert!(!is_err_range(hint), "{ctx}: {} start decode", a.tag);
                    hints.push(hint);
                }
                assert_eq!(hints[0], hints[1], "{ctx}: decompress hint mismatch");
                let mut fic = LZ4F_frameInfo_t::default();
                let mut fir = LZ4F_frameInfo_t::default();
                let mut sc = 12345usize;
                let mut sr = 12345usize;
                let a = (c.get_frame_info)(dc, &mut fic, ptr::null(), &mut sc);
                let b = (r.get_frame_info)(dr, &mut fir, ptr::null(), &mut sr);
                assert_eq!(
                    a as isize, b as isize,
                    "{ctx}: cached getFrameInfo hint mismatch"
                );
                assert_eq!(sc, 0, "{ctx}: cached getFrameInfo consumes nothing (C)");
                assert_eq!(sr, 0, "{ctx}: cached getFrameInfo consumes nothing (Rust)");
                assert_eq!(fic, fir, "{ctx}: cached frameInfo mismatch");
                assert_eq!(fic.blockSizeID, ((h.bd >> 4) & 7) as u32);
                let sc2 = (c.free_dctx)(dc);
                let sr2 = (r.free_dctx)(dr);
                assert_eq!(sc2, sr2, "{ctx}: dStage mismatch mid-frame");
            }
        }
    }
}

// ===========================================================================
// Row 141 — dctx creation / free (returns dStage) / reset
// ===========================================================================

#[test]
fn row_141_dctx_lifecycle() {
    let mut rng = Rng::new(141);
    unsafe {
        let (c, r) = apis();

        // free(NULL) is OK
        assert_eq!((c.free_dctx)(ptr::null_mut()), 0);
        assert_eq!((r.free_dctx)(ptr::null_mut()), 0);

        for version in [LZ4F_VERSION, 0, 99, u32::MAX] {
            let mut pc: *mut c_void = ptr::null_mut();
            let mut pr: *mut c_void = ptr::null_mut();
            let a = (c.create_dctx)(&mut pc, version);
            let b = (r.create_dctx)(&mut pr, version);
            assert_eq!(a, b, "createDecompressionContext(version={version})");
            assert_eq!(a, 0);
            // fresh dctx => dStage == dstage_getFrameHeader == 0
            assert_eq!((c.free_dctx)(pc), 0, "fresh C dctx dStage");
            assert_eq!((r.free_dctx)(pr), 0, "fresh Rust dctx dStage");
        }

        // _advanced, default and custom allocators
        for flavour in 0..3 {
            for a in [c, r] {
                let mut st = MemStat {
                    allocs: 0,
                    callocs: 0,
                    frees: 0,
                };
                let dctx = match flavour {
                    0 => (a.create_dctx_adv)(DEFAULT_CMEM, LZ4F_VERSION),
                    1 => (a.create_dctx_adv)(cmem_alloc_only(&mut st), LZ4F_VERSION),
                    _ => (a.create_dctx_adv)(cmem_full(&mut st), LZ4F_VERSION),
                };
                assert!(!dctx.is_null(), "{}: createDecompressionContext_advanced", a.tag);
                assert_eq!((a.free_dctx)(dctx), 0);
                if flavour > 0 {
                    assert!(st.allocs + st.callocs > 0, "{}: custom alloc unused", a.tag);
                    assert_eq!(
                        st.allocs + st.callocs,
                        st.frees,
                        "{}: custom allocator leaked (flavour {flavour})",
                        a.tag
                    );
                }
            }
        }

        // dStage for a fresh / mid-frame / completed dctx, and reset-after-error
        let mut p = pref();
        p.frameInfo.blockSizeID = LZ4F_MAX64KB;
        p.frameInfo.contentChecksumFlag = 1;
        p.frameInfo.contentSize = 1;
        let src = gen(&mut rng, Shape::TextLike, 200_000);
        let cap = cfbound(src.len(), Some(&p));
        let (ret, frame) = cf("row141 frame", &src, cap, Some(&p));
        assert!(!is_err_range(ret));
        let h = parse_header(&frame);

        for a in [c, r] {
            // completed frame => dStage back to 0
            let dctx = new_dctx(a);
            let bsz = frame_block_size(&frame);
            let mut out = vec![0u8; bsz + 16];
            let mut ip = 0usize;
            loop {
                let mut dsz = out.len();
                let mut ssz = frame.len() - ip;
                let hint = (a.decompress)(
                    dctx,
                    out.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    frame.as_ptr().add(ip) as *const c_void,
                    &mut ssz,
                    ptr::null(),
                );
                assert!(!is_err_range(hint));
                ip += ssz;
                if hint == 0 {
                    break;
                }
            }
            assert_eq!((a.free_dctx)(dctx), 0, "{}: completed dctx dStage", a.tag);

            // mid-frame (header consumed, waiting for a block header) => 3
            let dctx = new_dctx(a);
            let mut dsz = 0usize;
            let mut ssz = h.size;
            let hint = (a.decompress)(
                dctx,
                ptr::null_mut(),
                &mut dsz,
                frame.as_ptr() as *const c_void,
                &mut ssz,
                ptr::null(),
            );
            assert!(!is_err_range(hint), "{}: header-only decode", a.tag);
            // getBlockHeader runs out of input immediately and parks in
            // dstage_storeBlockHeader == 4
            assert_eq!(
                (a.free_dctx)(dctx),
                4,
                "{}: dStage after the header (dstage_storeBlockHeader)",
                a.tag
            );
        }

        // reset after an error, then reuse for a new frame
        let mut bad = frame.clone();
        // corrupt the content checksum in the footer
        let l = bad.len();
        bad[l - 1] ^= 0xFF;
        for a in [c, r] {
            let dctx = new_dctx(a);
            let bsz = frame_block_size(&bad);
            let mut out = vec![0u8; bsz + 16];
            let mut ip = 0usize;
            let mut e = 0usize;
            loop {
                let mut dsz = out.len();
                let mut ssz = bad.len() - ip;
                let hint = (a.decompress)(
                    dctx,
                    out.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    bad.as_ptr().add(ip) as *const c_void,
                    &mut ssz,
                    ptr::null(),
                );
                if is_err_range(hint) {
                    e = hint;
                    break;
                }
                ip += ssz;
                if hint == 0 {
                    break;
                }
            }
            assert_eq!(
                e,
                err(18),
                "{}: corrupted content checksum must be contentChecksum_invalid, got {}",
                a.tag,
                e as isize
            );
            // reset, then decode the good frame with the same dctx
            (a.reset_dctx)(dctx);
            let mut got: Vec<u8> = Vec::new();
            let mut ip = 0usize;
            loop {
                let mut dsz = out.len();
                let mut ssz = frame.len() - ip;
                let hint = (a.decompress)(
                    dctx,
                    out.as_mut_ptr() as *mut c_void,
                    &mut dsz,
                    frame.as_ptr().add(ip) as *const c_void,
                    &mut ssz,
                    ptr::null(),
                );
                assert!(
                    !is_err_range(hint),
                    "{}: reuse after reset failed with {}",
                    a.tag,
                    hint as isize
                );
                got.extend_from_slice(&out[..dsz]);
                ip += ssz;
                if hint == 0 {
                    break;
                }
            }
            assert_eq!(got.len(), src.len(), "{}: reuse after reset payload", a.tag);
            assert!(first_diff(&got, &src).is_none());
            assert_eq!((a.free_dctx)(dctx), 0);
        }
    }
}
