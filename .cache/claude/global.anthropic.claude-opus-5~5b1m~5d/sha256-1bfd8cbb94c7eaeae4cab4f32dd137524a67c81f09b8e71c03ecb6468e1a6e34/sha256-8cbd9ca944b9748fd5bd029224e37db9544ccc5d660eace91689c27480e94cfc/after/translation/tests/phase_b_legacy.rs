//! Phase B: the legacy v0.1 .. v0.7 decoders
//! (headers: c_src/src/legacy/zstd_v0*.h, zstd_legacy.h).
//!
//! There is no legacy-format sample data in the repository and no legacy
//! encoder, so every exported legacy entry point is exercised on hand-built
//! buffers (correct magic, magic + deterministic garbage, plausible-but-corrupt
//! frame/block headers, a modern zstd frame, tiny buffers, all-zero buffers)
//! and the C ground truth is required to return byte-identical results to the
//! Rust port for every single call.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------- symbol types ---

type FnDec = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnDecCtx = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnDecDict = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
) -> usize;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCtxInt = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCtxVoid = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnFFSI = unsafe extern "C" fn(*const u8, usize, *mut usize, *mut u64);
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;
type FnSize0 = unsafe extern "C" fn() -> usize;
type FnBeginDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnGetDecompressedSize = unsafe extern "C" fn(*const u8, usize) -> u64;
type FnCtxSizeOf = unsafe extern "C" fn(*const c_void) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnIsFrame = unsafe extern "C" fn(*const u8, usize) -> c_uint;

type FnZCont = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
) -> usize;

type AllocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
struct CustomMem {
    customAlloc: Option<AllocFn>,
    customFree: Option<FreeFn>,
    opaque: *mut c_void,
}
const NO_CUSTOM_MEM: CustomMem =
    CustomMem { customAlloc: None, customFree: None, opaque: std::ptr::null_mut() };
type FnCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;

// ------- legacy frame params structs (layouts taken from the headers) ------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct V05Params {
    srcSize: u64,
    windowLog: u32,
    contentLog: u32,
    hashLog: u32,
    searchLog: u32,
    searchLength: u32,
    targetLength: u32,
    strategy: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct V06Params {
    frameContentSize: u64,
    windowLog: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct V07Params {
    frameContentSize: u64,
    windowSize: u32,
    dictID: u32,
    checksumFlag: u32,
}

// ------------------------------------------------------------- symbol IO ---

/// `[C, Rust]` fn pointers for a symbol that must exist in both libraries.
macro_rules! both {
    ($name:expr, $t:ty) => {{
        let (c, r) = unsafe { pair::<$t>($name) };
        [*c, *r]
    }};
}

fn pair_opt<T: Copy>(name: &str) -> Option<[T; 2]> {
    let c = unsafe { clib().get::<T>(name.as_bytes()) }.ok()?;
    let r = unsafe { rlib().get::<T>(name.as_bytes()) }.ok()?;
    Some([*c, *r])
}

/// `[C, Rust]` fn pointers, or `None` if the symbol is exported by neither.
macro_rules! both_opt {
    ($name:expr, $t:ty) => {{
        let n: &str = $name;
        let got = pair_opt::<$t>(n);
        if got.is_none() {
            // Neither library may export it; if only one does that's a bug.
            let c = unsafe { clib().get::<$t>(n.as_bytes()) }.is_ok();
            let r = unsafe { rlib().get::<$t>(n.as_bytes()) }.is_ok();
            assert_eq!(c, r, "symbol {n} exported by only one library (C={c}, Rust={r})");
        }
        got
    }};
}

// --------------------------------------------------------- test corpora ----

fn magic_bytes(magic: u32, be: bool) -> [u8; 4] {
    if be { magic.to_be_bytes() } else { magic.to_le_bytes() }
}

/// Deterministic modern (v1.x) zstd frame; identical in both libraries.
fn modern_frame() -> Vec<u8> {
    let compress = both!("ZSTD_compress", FnCompress);
    let bound = both!("ZSTD_compressBound", FnBound);
    let mut rng = Rng::new(0x1234_5678);
    let src = gen(Shape::Text, 5000, &mut rng);
    let cap = unsafe { bound[0](src.len()) } + 64;
    let mut a = vec![0u8; cap];
    let mut b = vec![0u8; cap];
    let na = unsafe { compress[0](a.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
    let nb = unsafe { compress[1](b.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
    assert_eq!(na, nb, "ZSTD_compress return differs");
    assert!(!is_error(na), "ZSTD_compress failed");
    assert_bytes_eq("modern reference frame", &a[..na], &b[..nb]);
    a.truncate(na);
    a
}

/// Structured "plausible but corrupt" tails appended after a legacy magic.
const CORRUPT_TAILS: &[&[u8]] = &[
    &[0x00, 0x00, 0x00],
    &[0x01, 0x00, 0x00],
    &[0x02, 0x00, 0x00],
    &[0x04, 0x00, 0x00],
    &[0x0c, 0x00, 0x00],
    &[0xff, 0xff, 0xff],
    &[0x00, 0x01, 0x00, 0x00],
    &[0x01, 0x01, 0x00, 0x00],
    &[0x02, 0xff, 0x0f, 0x00],
    &[0x40, 0x01, 0x00, 0x00, 0xaa, 0xbb],
    &[0x60, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    &[0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // block header claiming 8 raw bytes, followed by exactly 8 bytes
    &[0x00, 0x21, 0x00, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0],
    // block header claiming a huge size
    &[0x00, 0xfc, 0xff, 0x7f, 0, 0, 0, 0],
];

fn build_bufs(magic: u32, be: bool, modern: &[u8], seed: u64) -> Vec<(String, Vec<u8>)> {
    let m = magic_bytes(magic, be);
    let mut rng = Rng::new(seed);
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();

    // (a) the correct 4-byte magic alone
    v.push(("magic-only".to_string(), m.to_vec()));

    // (e) buffers of 0, 1, 2, 3, 4, 5 bytes -- both magic prefixes and filler
    for n in [0usize, 1, 2, 3, 4, 5] {
        let mut b: Vec<u8> = m.iter().copied().take(n.min(4)).collect();
        while b.len() < n {
            b.push(0x5a);
        }
        v.push((format!("magicprefix{n}"), b));
        v.push((format!("filler{n}"), vec![0xa5u8; n]));
    }

    // (b) magic + 64 / 256 bytes of deterministic pseudo-random garbage
    for &n in &[64usize, 256] {
        for k in 0..3 {
            let mut b = m.to_vec();
            b.extend_from_slice(&gen(Shape::Random, n, &mut rng));
            v.push((format!("magic+rand{n}#{k}"), b));
        }
        // garbage without any magic
        let mut b = gen(Shape::Random, n + 4, &mut rng);
        b.truncate(n + 4);
        v.push((format!("rand{n}-nomagic"), b));
    }

    // (c) magic + plausible but corrupt frame header / block header
    for (i, tail) in CORRUPT_TAILS.iter().enumerate() {
        let mut b = m.to_vec();
        b.extend_from_slice(tail);
        v.push((format!("magic+corrupt{i}"), b));
    }

    // (d) a modern zstd frame fed to the legacy decoder
    v.push(("modern-frame".to_string(), modern.to_vec()));
    // ... and a modern frame body behind the legacy magic
    let mut b = m.to_vec();
    if modern.len() > 4 {
        b.extend_from_slice(&modern[4..]);
    }
    v.push(("magic+modern-body".to_string(), b));

    // (f) 1024 all-zero bytes (and behind the magic)
    v.push(("zeros1024".to_string(), vec![0u8; 1024]));
    let mut b = m.to_vec();
    b.extend_from_slice(&vec![0u8; 1024]);
    v.push(("magic+zeros1024".to_string(), b));

    v
}

/// Exhaustive one-byte sweeps just after the magic: this is what actually
/// drives the legacy header / block-header decoders past their magic check.
/// v0.4+ have a 1-byte frame-header descriptor there; v0.1..v0.3 start with a
/// 3-byte block header.
fn build_sweep(magic: u32, be: bool, seed: u64) -> Vec<(String, Vec<u8>)> {
    let m = magic_bytes(magic, be);
    let mut rng = Rng::new(seed);
    let payload = gen(Shape::Random, 96, &mut rng);
    let zeros = vec![0u8; 96];
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    for d in 0u16..256 {
        for (tag, tail) in [("rand", &payload), ("zero", &zeros)] {
            // frame-header descriptor sweep
            let mut b = m.to_vec();
            b.push(d as u8);
            b.extend_from_slice(tail);
            v.push((format!("fhd{d:02x}-{tag}"), b));
            // 3-byte block-header sweep
            let mut b = m.to_vec();
            b.extend_from_slice(&[d as u8, 0x00, 0x00]);
            b.extend_from_slice(tail);
            v.push((format!("bh{d:02x}-{tag}"), b));
        }
    }
    v
}

/// The full corpus for one version: hand-built shapes + the header sweeps.
fn full_corpus(magic: u32, be: bool, seed: u64) -> Vec<(String, Vec<u8>)> {
    let mut v = build_bufs(magic, be, &modern_frame(), seed);
    v.extend(build_sweep(magic, be, seed ^ 0xFFFF));
    v
}

// ------------------------------------------------------------- recording ---

#[derive(Debug, PartialEq, Eq, Clone)]
enum Rec {
    Rc(&'static str, usize),
    U64(&'static str, u64),
    I32(&'static str, c_int),
    Bytes(&'static str, Vec<u8>),
    Null(&'static str, bool),
}

const DST_CAP: usize = 1 << 18; // 256 KB: larger than any legacy block

// --------------------------------------------------- generic version desc --

struct Ver {
    name: &'static str,
    magic: u32,
    magic_be: bool,
    decompress: [FnDec; 2],
    decompressDCtx: Option<[FnDecCtx; 2]>,
    createDCtx: [FnCreate; 2],
    freeDCtx: [FnCtx; 2],
    resetDCtx: Option<[FnCtx; 2]>,
    nextSrcSize: [FnCtx; 2],
    decompressContinue: [FnDecCtx; 2],
    isError: Option<[FnIsError; 2]>,
    getErrorName: Option<[FnErrName; 2]>,
    findFrameSizeInfo: [FnFFSI; 2],
    /// v05..v07 require an explicit decompressBegin before streaming
    decompressBegin: Option<[FnCtx; 2]>,
}

macro_rules! ver {
    ($v:literal, $magic:expr, $be:expr) => {
        Ver {
            name: $v,
            magic: $magic,
            magic_be: $be,
            decompress: both!(concat!("ZSTDv", $v, "_decompress"), FnDec),
            decompressDCtx: both_opt!(concat!("ZSTDv", $v, "_decompressDCtx"), FnDecCtx),
            createDCtx: both!(concat!("ZSTDv", $v, "_createDCtx"), FnCreate),
            freeDCtx: both!(concat!("ZSTDv", $v, "_freeDCtx"), FnCtx),
            resetDCtx: both_opt!(concat!("ZSTDv", $v, "_resetDCtx"), FnCtx),
            nextSrcSize: both!(concat!("ZSTDv", $v, "_nextSrcSizeToDecompress"), FnCtx),
            decompressContinue: both!(concat!("ZSTDv", $v, "_decompressContinue"), FnDecCtx),
            isError: both_opt!(concat!("ZSTDv", $v, "_isError"), FnIsError),
            getErrorName: both_opt!(concat!("ZSTDv", $v, "_getErrorName"), FnErrName),
            findFrameSizeInfo: both!(
                concat!("ZSTDv", $v, "_findFrameSizeInfoLegacy"),
                FnFFSI
            ),
            decompressBegin: both_opt!(concat!("ZSTDv", $v, "_decompressBegin"), FnCtx),
        }
    };
}

/// Run every generic legacy entry point of one version over one input buffer,
/// separately in each library, and compare the resulting record sequences.
fn run_generic(v: &Ver, label: &str, buf: &[u8]) {
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let r = &mut recs[w];

        // --- one-shot decompress ---
        for &cap in &[0usize, 16, DST_CAP] {
            let mut dst = vec![0u8; cap.max(1)];
            let n = unsafe {
                v.decompress[w](dst.as_mut_ptr(), cap, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompress", n));
            if !is_error(n) && n <= cap {
                r.push(Rec::Bytes("decompress.out", dst[..n].to_vec()));
            }
        }

        // --- decompressDCtx ---
        if let Some(f) = v.decompressDCtx {
            let ctx = unsafe { v.createDCtx[w]() };
            r.push(Rec::Null("createDCtx", ctx.is_null()));
            if !ctx.is_null() {
                let mut dst = vec![0u8; DST_CAP];
                let n = unsafe {
                    f[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
                };
                r.push(Rec::Rc("decompressDCtx", n));
                if !is_error(n) && n <= DST_CAP {
                    r.push(Rec::Bytes("decompressDCtx.out", dst[..n].to_vec()));
                }
                r.push(Rec::Rc("freeDCtx", unsafe { v.freeDCtx[w](ctx) }));
            }
        }

        // --- findFrameSizeInfoLegacy: BOTH out params ---
        {
            let mut c_size: usize = 0xDEAD_BEEF;
            let mut d_bound: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            unsafe {
                v.findFrameSizeInfo[w](buf.as_ptr(), buf.len(), &mut c_size, &mut d_bound)
            };
            r.push(Rec::Rc("ffsi.cSize", c_size));
            r.push(Rec::U64("ffsi.dBound", d_bound));
        }

        // --- streaming: reset/begin + nextSrcSizeToDecompress + continue ---
        {
            let ctx = unsafe { v.createDCtx[w]() };
            r.push(Rec::Null("createDCtx.stream", ctx.is_null()));
            if !ctx.is_null() {
                if let Some(f) = v.resetDCtx {
                    r.push(Rec::Rc("resetDCtx", unsafe { f[w](ctx) }));
                }
                if let Some(f) = v.decompressBegin {
                    r.push(Rec::Rc("decompressBegin", unsafe { f[w](ctx) }));
                }
                let mut dst = vec![0u8; DST_CAP];
                let mut pos = 0usize;
                let mut written = 0usize;
                for _ in 0..2000 {
                    let need = unsafe { v.nextSrcSize[w](ctx) };
                    r.push(Rec::Rc("nextSrcSize", need));
                    if need == 0 || is_error(need) {
                        break;
                    }
                    if need > buf.len() - pos || need > DST_CAP {
                        break; // cannot satisfy the protocol
                    }
                    let rc = unsafe {
                        v.decompressContinue[w](
                            ctx,
                            dst.as_mut_ptr().add(written),
                            DST_CAP - written,
                            buf.as_ptr().add(pos),
                            need,
                        )
                    };
                    r.push(Rec::Rc("decompressContinue", rc));
                    if is_error(rc) {
                        break;
                    }
                    pos += need;
                    written += rc;
                    if written > DST_CAP {
                        break;
                    }
                }
                r.push(Rec::Bytes("stream.out", dst[..written.min(DST_CAP)].to_vec()));
                // deliberately wrong srcSize must be rejected identically
                let rc = unsafe {
                    v.decompressContinue[w](
                        ctx,
                        dst.as_mut_ptr(),
                        DST_CAP,
                        buf.as_ptr(),
                        buf.len().wrapping_add(1),
                    )
                };
                r.push(Rec::Rc("continue.wrongSize", rc));
                r.push(Rec::Rc("freeDCtx.stream", unsafe { v.freeDCtx[w](ctx) }));
            }
        }
    }
    assert!(recs[0].len() >= 8, "ZSTDv{} battery recorded nothing", v.name);
    assert_eq!(
        recs[0], recs[1],
        "ZSTDv{} generic battery differs on `{label}` (len={})",
        v.name,
        buf.len()
    );
}

/// ZSTD_isFrame / ZSTD_decompress auto-dispatch over the same buffers.
fn run_autodispatch(label: &str, buf: &[u8]) {
    let isframe = both!("ZSTD_isFrame", FnIsFrame);
    let dec = both!("ZSTD_decompress", FnDec);
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let r = &mut recs[w];
        r.push(Rec::Rc("isFrame", unsafe {
            isframe[w](buf.as_ptr(), buf.len()) as usize
        }));
        let mut dst = vec![0u8; DST_CAP];
        let n = unsafe { dec[w](dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len()) };
        r.push(Rec::Rc("ZSTD_decompress", n));
        if !is_error(n) && n <= DST_CAP {
            r.push(Rec::Bytes("ZSTD_decompress.out", dst[..n].to_vec()));
        }
    }
    assert!(!recs[0].is_empty(), "auto-dispatch recorded nothing");
    assert_eq!(recs[0], recs[1], "auto-dispatch differs on `{label}`");
}

/// The ZBUFFv04..v07 buffered decoders.
struct ZbuffVer {
    name: &'static str,
    create: [FnCreate; 2],
    create_adv: Option<[FnCreateAdv; 2]>,
    free: [FnCtx; 2],
    init: [FnCtx; 2],
    init_dict: [FnBeginDict; 2],
    cont: [FnZCont; 2],
    iserr: [FnIsError; 2],
    errname: [FnErrName; 2],
    din: [FnSize0; 2],
    dout: [FnSize0; 2],
}

macro_rules! zver {
    ($v:literal, $dictfn:literal) => {
        ZbuffVer {
            name: $v,
            create: both!(concat!("ZBUFFv", $v, "_createDCtx"), FnCreate),
            create_adv: both_opt!(concat!("ZBUFFv", $v, "_createDCtx_advanced"), FnCreateAdv),
            free: both!(concat!("ZBUFFv", $v, "_freeDCtx"), FnCtx),
            init: both!(concat!("ZBUFFv", $v, "_decompressInit"), FnCtx),
            init_dict: both!(concat!("ZBUFFv", $v, $dictfn), FnBeginDict),
            cont: both!(concat!("ZBUFFv", $v, "_decompressContinue"), FnZCont),
            iserr: both!(concat!("ZBUFFv", $v, "_isError"), FnIsError),
            errname: both!(concat!("ZBUFFv", $v, "_getErrorName"), FnErrName),
            din: both!(concat!("ZBUFFv", $v, "_recommendedDInSize"), FnSize0),
            dout: both!(concat!("ZBUFFv", $v, "_recommendedDOutSize"), FnSize0),
        }
    };
}

fn run_zbuff(z: &ZbuffVer, label: &str, buf: &[u8], dict: &[u8]) {
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let r = &mut recs[w];
        r.push(Rec::Rc("DInSize", unsafe { z.din[w]() }));
        r.push(Rec::Rc("DOutSize", unsafe { z.dout[w]() }));

        for mode in 0..3 {
            // 0: plain init, 1: init with dict, 2: advanced ctor + plain init
            let ctx = if mode == 2 {
                match z.create_adv {
                    Some(f) => unsafe { f[w](NO_CUSTOM_MEM) },
                    None => continue,
                }
            } else {
                unsafe { z.create[w]() }
            };
            r.push(Rec::Null("create", ctx.is_null()));
            if ctx.is_null() {
                continue;
            }
            let rc0 = if mode == 1 {
                unsafe { z.init_dict[w](ctx, dict.as_ptr(), dict.len()) }
            } else {
                unsafe { z.init[w](ctx) }
            };
            r.push(Rec::Rc("init", rc0));

            for &(inc, outc) in &[(buf.len().max(1), 1usize << 16), (1usize, 7usize), (3, 1000)] {
                let mut out = vec![0u8; outc];
                let mut collected: Vec<u8> = Vec::new();
                let mut pos = 0usize;
                for _ in 0..4000 {
                    let mut s = inc.min(buf.len() - pos);
                    let mut d = out.len();
                    let rc = unsafe {
                        z.cont[w](
                            ctx,
                            out.as_mut_ptr(),
                            &mut d,
                            buf.as_ptr().add(pos),
                            &mut s,
                        )
                    };
                    r.push(Rec::Rc("zcont.rc", rc));
                    r.push(Rec::Rc("zcont.dst", d));
                    r.push(Rec::Rc("zcont.src", s));
                    if is_error(rc) {
                        break;
                    }
                    collected.extend_from_slice(&out[..d.min(out.len())]);
                    pos += s;
                    if rc == 0 {
                        break;
                    }
                    if s == 0 && d == 0 {
                        break;
                    }
                }
                r.push(Rec::Bytes("zcont.out", collected));
                // re-init for the next chunking scheme
                let ri = if mode == 1 {
                    unsafe { z.init_dict[w](ctx, dict.as_ptr(), dict.len()) }
                } else {
                    unsafe { z.init[w](ctx) }
                };
                r.push(Rec::Rc("reinit", ri));
            }
            r.push(Rec::Rc("free", unsafe { z.free[w](ctx) }));
        }
    }
    assert!(!recs[0].is_empty(), "ZBUFFv{} recorded nothing", z.name);
    assert_eq!(recs[0], recs[1], "ZBUFFv{} differs on `{label}`", z.name);
}

fn zbuff_errors(z: &ZbuffVer) {
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let r = &mut recs[w];
        for &code in ERROR_CODES {
            r.push(Rec::Rc("isError", unsafe { z.iserr[w](code) } as usize));
            let s = unsafe { CStr::from_ptr(z.errname[w](code)) };
            r.push(Rec::Bytes("getErrorName", s.to_bytes().to_vec()));
        }
    }
    assert!(!recs[0].is_empty(), "ZBUFFv{} helpers recorded nothing", z.name);
    assert_eq!(recs[0], recs[1], "ZBUFFv{} error helpers differ", z.name);
}

const ERROR_CODES: &[usize] = &[
    0,
    1,
    usize::MAX,                     // (size_t)-1
    usize::MAX - 9,                 // (size_t)-10
    usize::MAX - 71,                // (size_t)-72
    usize::MAX - 119,               // (size_t)-120
    usize::MAX - 120,               // (size_t)-121
    2,
    100,
    1 << 20,
];

// ================================================================ v0.1 ====

fn corpus(magic: u32, be: bool, seed: u64) -> Vec<(String, Vec<u8>)> {
    full_corpus(magic, be, seed)
}

/// The buffered (ZBUFF) drivers are much more expensive per buffer, so they get
/// the hand-built shapes plus a strided sample of the header sweeps.
fn zbuff_subset(all: &[(String, Vec<u8>)]) -> Vec<usize> {
    (0..all.len()).filter(|i| *i < 60 || *i % 23 == 0).collect()
}

#[test]
fn legacy_v01() {
    // zstd_v01.h: ZSTDv01_magicNumber 0xFD2FB51E, read with ZSTD_readBE32()
    let v = ver!("01", 0xFD2F_B51E, true);
    for (label, buf) in corpus(0xFD2F_B51E, true, 0x0101) {
        run_generic(&v, &label, &buf);
        run_autodispatch(&label, &buf);
    }
    // the little-endian spelling of the same constant must NOT be recognised
    for (label, buf) in corpus(0x1EB5_2FFD, true, 0x0102) {
        run_generic(&v, &format!("LE-spelling {label}"), &buf);
    }
}

#[test]
fn legacy_v02() {
    // zstd_v02.h: 0xFD2FB522, read with MEM_readLE32()
    let v = ver!("02", 0xFD2F_B522, false);
    for (label, buf) in corpus(0xFD2F_B522, false, 0x0201) {
        run_generic(&v, &label, &buf);
        run_autodispatch(&label, &buf);
    }
    for (label, buf) in corpus(0xFD2F_B522, true, 0x0202) {
        run_generic(&v, &format!("BE-spelling {label}"), &buf);
    }
}

#[test]
fn legacy_v03() {
    // zstd_v03.h: 0xFD2FB523, read with MEM_readLE32()
    let v = ver!("03", 0xFD2F_B523, false);
    for (label, buf) in corpus(0xFD2F_B523, false, 0x0301) {
        run_generic(&v, &label, &buf);
        run_autodispatch(&label, &buf);
    }
    for (label, buf) in corpus(0xFD2F_B523, true, 0x0302) {
        run_generic(&v, &format!("BE-spelling {label}"), &buf);
    }
}

#[test]
fn legacy_v04() {
    let v = ver!("04", 0xFD2F_B524, false);
    let z = zver!("04", "_decompressWithDictionary");
    let mut rng = Rng::new(0x0401);
    let dict = gen(Shape::Text, 4096, &mut rng);
    zbuff_errors(&z);
    let all = corpus(0xFD2F_B524, false, 0x0402);
    let zsub = zbuff_subset(&all);
    for (i, (label, buf)) in all.iter().enumerate() {
        run_generic(&v, label, buf);
        run_autodispatch(label, buf);
        if zsub.binary_search(&i).is_ok() {
            run_zbuff(&z, label, buf, &dict);
        }
    }
}

#[test]
fn legacy_v05() {
    let v = ver!("05", 0xFD2F_B525, false);
    let z = zver!("05", "_decompressInitDictionary");
    let mut rng = Rng::new(0x0501);
    let dict = gen(Shape::Text, 4096, &mut rng);

    let getFrameParams = both!("ZSTDv05_getFrameParams", unsafe extern "C" fn(
        *mut V05Params, *const u8, usize,
    ) -> usize);
    let beginDict = both!("ZSTDv05_decompressBegin_usingDict", FnBeginDict);
    let begin = both!("ZSTDv05_decompressBegin", FnCtx);
    let block = both!("ZSTDv05_decompressBlock", FnDecCtx);
    let usingDict = both!("ZSTDv05_decompress_usingDict", FnDecDict);
    let prepared = both!("ZSTDv05_decompress_usingPreparedDCtx", unsafe extern "C" fn(
        *mut c_void, *const c_void, *mut u8, usize, *const u8, usize,
    ) -> usize);
    let copyDCtx = both!("ZSTDv05_copyDCtx", FnCtxVoid);
    let sizeofDCtx = both!("ZSTDv05_sizeofDCtx", FnSize0);
    let create = both!("ZSTDv05_createDCtx", FnCreate);
    let free = both!("ZSTDv05_freeDCtx", FnCtx);

    // sizeofDCtx is a pure struct-size query
    assert_eq!(
        unsafe { sizeofDCtx[0]() },
        unsafe { sizeofDCtx[1]() },
        "ZSTDv05_sizeofDCtx differs"
    );

    zbuff_errors(&z);
    let all = corpus(0xFD2F_B525, false, 0x0502);
    let zsub = zbuff_subset(&all);
    for (bi, (label, buf)) in all.iter().enumerate() {
        let buf = buf.as_slice();
        run_generic(&v, label, buf);
        run_autodispatch(label, buf);
        if zsub.binary_search(&bi).is_ok() {
            run_zbuff(&z, label, buf, &dict);
        }

        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];
            // getFrameParams: compare the whole out struct
            let mut p = V05Params { srcSize: 0xAAAA, windowLog: 7, ..Default::default() };
            let rc = unsafe { getFrameParams[w](&mut p, buf.as_ptr(), buf.len()) };
            r.push(Rec::Rc("getFrameParams", rc));
            r.push(Rec::U64("fp.srcSize", p.srcSize));
            r.push(Rec::Rc("fp.windowLog", p.windowLog as usize));
            r.push(Rec::Rc("fp.contentLog", p.contentLog as usize));
            r.push(Rec::Rc("fp.hashLog", p.hashLog as usize));
            r.push(Rec::Rc("fp.searchLog", p.searchLog as usize));
            r.push(Rec::Rc("fp.searchLength", p.searchLength as usize));
            r.push(Rec::Rc("fp.targetLength", p.targetLength as usize));
            r.push(Rec::I32("fp.strategy", p.strategy));

            // decompress_usingDict, with and without a dictionary
            for d in [None, Some(&dict)] {
                let ctx = unsafe { create[w]() };
                let mut dst = vec![0u8; DST_CAP];
                let (dp, dl) = match d {
                    None => (std::ptr::null(), 0usize),
                    Some(x) => (x.as_ptr(), x.len()),
                };
                let n = unsafe {
                    usingDict[w](
                        ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len(), dp, dl,
                    )
                };
                r.push(Rec::Rc("usingDict", n));
                if !is_error(n) && n <= DST_CAP {
                    r.push(Rec::Bytes("usingDict.out", dst[..n].to_vec()));
                }
                unsafe { free[w](ctx) };
            }

            // decompressBegin / decompressBegin_usingDict / decompressBlock
            let ctx = unsafe { create[w]() };
            r.push(Rec::Rc("begin", unsafe { begin[w](ctx) }));
            let mut dst = vec![0u8; DST_CAP];
            let nb = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock", nb));
            if !is_error(nb) && nb <= DST_CAP {
                r.push(Rec::Bytes("decompressBlock.out", dst[..nb].to_vec()));
            }
            r.push(Rec::Rc("beginDict", unsafe {
                beginDict[w](ctx, dict.as_ptr(), dict.len())
            }));
            let nb2 = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock.dict", nb2));

            // copyDCtx + decompress_usingPreparedDCtx (both ctxs from same lib)
            let ctx2 = unsafe { create[w]() };
            unsafe { copyDCtx[w](ctx2, ctx as *const c_void) };
            let np = unsafe {
                prepared[w](
                    ctx2,
                    ctx as *const c_void,
                    dst.as_mut_ptr(),
                    DST_CAP,
                    buf.as_ptr(),
                    buf.len(),
                )
            };
            r.push(Rec::Rc("usingPreparedDCtx", np));
            if !is_error(np) && np <= DST_CAP {
                r.push(Rec::Bytes("usingPreparedDCtx.out", dst[..np].to_vec()));
            }
            r.push(Rec::Rc("free2", unsafe { free[w](ctx2) }));
            r.push(Rec::Rc("free1", unsafe { free[w](ctx) }));
        }
        assert!(recs[0].len() >= 10, "ZSTDv05 extras recorded nothing");
        assert_eq!(recs[0], recs[1], "ZSTDv05 extras differ on `{label}`");
    }
}

#[test]
fn legacy_v06() {
    let v = ver!("06", 0xFD2F_B526, false);
    let z = zver!("06", "_decompressInitDictionary");
    let mut rng = Rng::new(0x0601);
    let dict = gen(Shape::Text, 4096, &mut rng);

    let getFrameParams = both!("ZSTDv06_getFrameParams", unsafe extern "C" fn(
        *mut V06Params, *const u8, usize,
    ) -> usize);
    let beginDict = both!("ZSTDv06_decompressBegin_usingDict", FnBeginDict);
    let begin = both!("ZSTDv06_decompressBegin", FnCtx);
    let block = both!("ZSTDv06_decompressBlock", FnDecCtx);
    let usingDict = both!("ZSTDv06_decompress_usingDict", FnDecDict);
    let prepared = both!("ZSTDv06_decompress_usingPreparedDCtx", unsafe extern "C" fn(
        *mut c_void, *const c_void, *mut u8, usize, *const u8, usize,
    ) -> usize);
    let copyDCtx = both!("ZSTDv06_copyDCtx", FnCtxVoid);
    let sizeofDCtx = both!("ZSTDv06_sizeofDCtx", FnSize0);
    // ZSTDv06_compressBound is declared in zstd_v06.h but is not exported by
    // the built library (there is no legacy encoder), so it cannot be tested.
    let bound = both_opt!("ZSTDv06_compressBound", FnBound);
    let create = both!("ZSTDv06_createDCtx", FnCreate);
    let free = both!("ZSTDv06_freeDCtx", FnCtx);

    assert_eq!(
        unsafe { sizeofDCtx[0]() },
        unsafe { sizeofDCtx[1]() },
        "ZSTDv06_sizeofDCtx differs"
    );
    if let Some(b) = bound {
        for &n in &[0usize, 1, 1000, 1 << 20] {
            assert_eq!(
                unsafe { b[0](n) },
                unsafe { b[1](n) },
                "ZSTDv06_compressBound({n}) differs"
            );
        }
    }

    zbuff_errors(&z);
    let all = corpus(0xFD2F_B526, false, 0x0602);
    let zsub = zbuff_subset(&all);
    for (bi, (label, buf)) in all.iter().enumerate() {
        let buf = buf.as_slice();
        run_generic(&v, label, buf);
        run_autodispatch(label, buf);
        if zsub.binary_search(&bi).is_ok() {
            run_zbuff(&z, label, buf, &dict);
        }

        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];
            let mut p = V06Params { frameContentSize: 0xAAAA, windowLog: 7 };
            let rc = unsafe { getFrameParams[w](&mut p, buf.as_ptr(), buf.len()) };
            r.push(Rec::Rc("getFrameParams", rc));
            r.push(Rec::U64("fp.frameContentSize", p.frameContentSize));
            r.push(Rec::Rc("fp.windowLog", p.windowLog as usize));

            for d in [None, Some(&dict)] {
                let ctx = unsafe { create[w]() };
                let mut dst = vec![0u8; DST_CAP];
                let (dp, dl) = match d {
                    None => (std::ptr::null(), 0usize),
                    Some(x) => (x.as_ptr(), x.len()),
                };
                let n = unsafe {
                    usingDict[w](
                        ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len(), dp, dl,
                    )
                };
                r.push(Rec::Rc("usingDict", n));
                if !is_error(n) && n <= DST_CAP {
                    r.push(Rec::Bytes("usingDict.out", dst[..n].to_vec()));
                }
                unsafe { free[w](ctx) };
            }

            let ctx = unsafe { create[w]() };
            r.push(Rec::Rc("begin", unsafe { begin[w](ctx) }));
            let mut dst = vec![0u8; DST_CAP];
            let nb = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock", nb));
            if !is_error(nb) && nb <= DST_CAP {
                r.push(Rec::Bytes("decompressBlock.out", dst[..nb].to_vec()));
            }
            r.push(Rec::Rc("beginDict", unsafe {
                beginDict[w](ctx, dict.as_ptr(), dict.len())
            }));
            let nb2 = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock.dict", nb2));

            let ctx2 = unsafe { create[w]() };
            unsafe { copyDCtx[w](ctx2, ctx as *const c_void) };
            let np = unsafe {
                prepared[w](
                    ctx2,
                    ctx as *const c_void,
                    dst.as_mut_ptr(),
                    DST_CAP,
                    buf.as_ptr(),
                    buf.len(),
                )
            };
            r.push(Rec::Rc("usingPreparedDCtx", np));
            r.push(Rec::Rc("free2", unsafe { free[w](ctx2) }));
            r.push(Rec::Rc("free1", unsafe { free[w](ctx) }));
        }
        assert!(recs[0].len() >= 10, "ZSTDv06 extras recorded nothing");
        assert_eq!(recs[0], recs[1], "ZSTDv06 extras differ on `{label}`");
    }
}

#[test]
fn legacy_v07() {
    let v = ver!("07", 0xFD2F_B527, false);
    let z = zver!("07", "_decompressInitDictionary");
    let mut rng = Rng::new(0x0701);
    let dict = gen(Shape::Text, 4096, &mut rng);

    let getFrameParams = both!("ZSTDv07_getFrameParams", unsafe extern "C" fn(
        *mut V07Params, *const u8, usize,
    ) -> usize);
    let beginDict = both!("ZSTDv07_decompressBegin_usingDict", FnBeginDict);
    let begin = both!("ZSTDv07_decompressBegin", FnCtx);
    let block = both!("ZSTDv07_decompressBlock", FnDecCtx);
    let usingDict = both!("ZSTDv07_decompress_usingDict", FnDecDict);
    let usingDDict = both!("ZSTDv07_decompress_usingDDict", unsafe extern "C" fn(
        *mut c_void, *mut u8, usize, *const u8, usize, *const c_void,
    ) -> usize);
    let createDDict = both!("ZSTDv07_createDDict", unsafe extern "C" fn(
        *const u8, usize,
    ) -> *mut c_void);
    let freeDDict = both!("ZSTDv07_freeDDict", FnCtx);
    let copyDCtx = both!("ZSTDv07_copyDCtx", FnCtxVoid);
    let sizeofDCtx = both!("ZSTDv07_sizeofDCtx", FnCtxSizeOf);
    let estimateDCtxSize = both!("ZSTDv07_estimateDCtxSize", FnSize0);
    let insertBlock = both!("ZSTDv07_insertBlock", FnInsertBlock);
    let isSkipFrame = both!("ZSTDv07_isSkipFrame", FnCtxInt);
    let getDecompressedSize =
        both!("ZSTDv07_getDecompressedSize", FnGetDecompressedSize);
    let createAdv = both!("ZSTDv07_createDCtx_advanced", FnCreateAdv);
    let create = both!("ZSTDv07_createDCtx", FnCreate);
    let free = both!("ZSTDv07_freeDCtx", FnCtx);

    assert_eq!(
        unsafe { estimateDCtxSize[0]() },
        unsafe { estimateDCtxSize[1]() },
        "ZSTDv07_estimateDCtxSize differs"
    );
    {
        let a = unsafe { create[0]() };
        let b = unsafe { create[1]() };
        assert_eq!(
            unsafe { sizeofDCtx[0](a as *const c_void) },
            unsafe { sizeofDCtx[1](b as *const c_void) },
            "ZSTDv07_sizeofDCtx differs"
        );
        unsafe { free[0](a) };
        unsafe { free[1](b) };
    }

    zbuff_errors(&z);
    let all = corpus(0xFD2F_B527, false, 0x0702);
    let zsub = zbuff_subset(&all);
    for (bi, (label, buf)) in all.iter().enumerate() {
        let buf = buf.as_slice();
        run_generic(&v, label, buf);
        run_autodispatch(label, buf);
        if zsub.binary_search(&bi).is_ok() {
            run_zbuff(&z, label, buf, &dict);
        }

        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];
            let mut p = V07Params {
                frameContentSize: 0xAAAA,
                windowSize: 7,
                dictID: 3,
                checksumFlag: 9,
            };
            let rc = unsafe { getFrameParams[w](&mut p, buf.as_ptr(), buf.len()) };
            r.push(Rec::Rc("getFrameParams", rc));
            r.push(Rec::U64("fp.frameContentSize", p.frameContentSize));
            r.push(Rec::Rc("fp.windowSize", p.windowSize as usize));
            r.push(Rec::Rc("fp.dictID", p.dictID as usize));
            r.push(Rec::Rc("fp.checksumFlag", p.checksumFlag as usize));
            r.push(Rec::U64("getDecompressedSize", unsafe {
                getDecompressedSize[w](buf.as_ptr(), buf.len())
            }));

            for d in [None, Some(&dict)] {
                let ctx = unsafe { create[w]() };
                let mut dst = vec![0u8; DST_CAP];
                let (dp, dl) = match d {
                    None => (std::ptr::null(), 0usize),
                    Some(x) => (x.as_ptr(), x.len()),
                };
                let n = unsafe {
                    usingDict[w](
                        ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len(), dp, dl,
                    )
                };
                r.push(Rec::Rc("usingDict", n));
                if !is_error(n) && n <= DST_CAP {
                    r.push(Rec::Bytes("usingDict.out", dst[..n].to_vec()));
                }
                unsafe { free[w](ctx) };
            }

            // createDDict / decompress_usingDDict / freeDDict
            {
                let ddict = unsafe { createDDict[w](dict.as_ptr(), dict.len()) };
                r.push(Rec::Null("createDDict", ddict.is_null()));
                if !ddict.is_null() {
                    let ctx = unsafe { create[w]() };
                    let mut dst = vec![0u8; DST_CAP];
                    let n = unsafe {
                        usingDDict[w](
                            ctx,
                            dst.as_mut_ptr(),
                            DST_CAP,
                            buf.as_ptr(),
                            buf.len(),
                            ddict as *const c_void,
                        )
                    };
                    r.push(Rec::Rc("usingDDict", n));
                    if !is_error(n) && n <= DST_CAP {
                        r.push(Rec::Bytes("usingDDict.out", dst[..n].to_vec()));
                    }
                    unsafe { free[w](ctx) };
                    r.push(Rec::Rc("freeDDict", unsafe { freeDDict[w](ddict) }));
                }
            }

            // advanced ctor, begin, block, insertBlock, isSkipFrame, copyDCtx
            let ctx = unsafe { createAdv[w](NO_CUSTOM_MEM) };
            r.push(Rec::Null("createDCtx_advanced", ctx.is_null()));
            r.push(Rec::Rc("begin", unsafe { begin[w](ctx) }));
            r.push(Rec::I32("isSkipFrame", unsafe { isSkipFrame[w](ctx) }));
            let mut dst = vec![0u8; DST_CAP];
            let nb = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock", nb));
            if !is_error(nb) && nb <= DST_CAP {
                r.push(Rec::Bytes("decompressBlock.out", dst[..nb].to_vec()));
            }
            r.push(Rec::Rc("insertBlock", unsafe {
                insertBlock[w](ctx, buf.as_ptr(), buf.len())
            }));
            r.push(Rec::I32("isSkipFrame2", unsafe { isSkipFrame[w](ctx) }));
            r.push(Rec::Rc("beginDict", unsafe {
                beginDict[w](ctx, dict.as_ptr(), dict.len())
            }));
            let nb2 = unsafe {
                block[w](ctx, dst.as_mut_ptr(), DST_CAP, buf.as_ptr(), buf.len())
            };
            r.push(Rec::Rc("decompressBlock.dict", nb2));

            let ctx2 = unsafe { create[w]() };
            unsafe { copyDCtx[w](ctx2, ctx as *const c_void) };
            r.push(Rec::Rc("sizeofDCtx", unsafe {
                sizeofDCtx[w](ctx2 as *const c_void)
            }));
            r.push(Rec::Rc("free2", unsafe { free[w](ctx2) }));
            r.push(Rec::Rc("free1", unsafe { free[w](ctx) }));
        }
        assert!(recs[0].len() >= 10, "ZSTDv07 extras recorded nothing");
        assert_eq!(recs[0], recs[1], "ZSTDv07 extras differ on `{label}`");
    }
}

// ========================================================= isError sweep ==

#[test]
fn legacy_isError() {
    let codes: &[usize] = &[
        0,
        1,
        usize::MAX,       // (size_t)-1
        usize::MAX - 9,   // (size_t)-10
        usize::MAX - 71,  // (size_t)-72
        usize::MAX - 119, // (size_t)-120
        usize::MAX - 120, // (size_t)-121
    ];

    // ZSTDv04 does not export isError / getErrorName.
    for v in ["01", "02", "03", "05", "06", "07"] {
        let name = format!("ZSTDv{v}_isError");
        let f = pair_opt::<FnIsError>(&name).unwrap_or_else(|| panic!("missing {name}"));
        for &c in codes {
            assert_eq!(
                unsafe { f[0](c) },
                unsafe { f[1](c) },
                "{name}({c:#x})"
            );
        }
        // full sweep over all defined error codes
        for k in 0..=130usize {
            let c = 0usize.wrapping_sub(k);
            assert_eq!(unsafe { f[0](c) }, unsafe { f[1](c) }, "{name}(-{k})");
        }
    }
    for v in ["05", "06", "07"] {
        let name = format!("ZSTDv{v}_getErrorName");
        let f = pair_opt::<FnErrName>(&name).unwrap_or_else(|| panic!("missing {name}"));
        for k in 0..=130usize {
            let c = 0usize.wrapping_sub(k);
            let a = unsafe { CStr::from_ptr(f[0](c)) }.to_owned();
            let b = unsafe { CStr::from_ptr(f[1](c)) }.to_owned();
            assert_eq!(a, b, "{name}(-{k})");
        }
        for &c in codes {
            let a = unsafe { CStr::from_ptr(f[0](c)) }.to_owned();
            let b = unsafe { CStr::from_ptr(f[1](c)) }.to_owned();
            assert_eq!(a, b, "{name}({c:#x})");
        }
    }
    // and the buffered wrappers
    for v in ["04", "05", "06", "07"] {
        let n1 = format!("ZBUFFv{v}_isError");
        let n2 = format!("ZBUFFv{v}_getErrorName");
        let f1 = pair_opt::<FnIsError>(&n1).unwrap_or_else(|| panic!("missing {n1}"));
        let f2 = pair_opt::<FnErrName>(&n2).unwrap_or_else(|| panic!("missing {n2}"));
        for k in 0..=130usize {
            let c = 0usize.wrapping_sub(k);
            assert_eq!(unsafe { f1[0](c) }, unsafe { f1[1](c) }, "{n1}(-{k})");
            let a = unsafe { CStr::from_ptr(f2[0](c)) }.to_owned();
            let b = unsafe { CStr::from_ptr(f2[1](c)) }.to_owned();
            assert_eq!(a, b, "{n2}(-{k})");
        }
        for &c in codes {
            assert_eq!(unsafe { f1[0](c) }, unsafe { f1[1](c) }, "{n1}({c:#x})");
            let a = unsafe { CStr::from_ptr(f2[0](c)) }.to_owned();
            let b = unsafe { CStr::from_ptr(f2[1](c)) }.to_owned();
            assert_eq!(a, b, "{n2}({c:#x})");
        }
    }
    // recommended sizes are constants
    for v in ["04", "05", "06", "07"] {
        for s in ["_recommendedDInSize", "_recommendedDOutSize"] {
            let name = format!("ZBUFFv{v}{s}");
            let f = pair_opt::<FnSize0>(&name).unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(unsafe { f[0]() }, unsafe { f[1]() }, "{name}");
        }
    }
}

// ========================================== legacy entropy stages (FSE/HUF) =

type FnFseCreate = unsafe extern "C" fn(c_uint) -> *mut u32;
type FnFseFree = unsafe extern "C" fn(*mut u32);
type FnFseRaw = unsafe extern "C" fn(*mut u32, c_uint) -> usize;
type FnFseRle = unsafe extern "C" fn(*mut u32, u8) -> usize;
type FnFseBuild = unsafe extern "C" fn(*mut u32, *const i16, c_uint, c_uint) -> usize;
type FnFseReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const u8, usize) -> usize;
type FnFseDecDT = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32) -> usize;
type FnHufReadDT16 = unsafe extern "C" fn(*mut u16, *const u8, usize) -> usize;
type FnHufReadDT32 = unsafe extern "C" fn(*mut u32, *const u8, usize) -> usize;
type FnHufDecDT16 = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u16) -> usize;
type FnHufDecDT32 = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u32) -> usize;
type FnHufDecCtx = unsafe extern "C" fn(*mut u32, *mut u8, usize, *const u8, usize) -> usize;
type FnHufReadStats = unsafe extern "C" fn(
    *mut u8,
    usize,
    *mut u32,
    *mut u32,
    *mut u32,
    *const u8,
    usize,
) -> usize;
type FnHufSelect = unsafe extern "C" fn(usize, usize) -> u32;

/// FSE DTable big enough for tableLog 12 plus header.
const FSE_DT_U32: usize = 1 + (1 << 12) + 16;
/// HUF X4 DTables need 1.5x the nominal size.
const HUF_DT_U32: usize = 1 + (1 << 12) * 3 / 2 + 32;
const HUF_DT_U16: usize = 1 + (1 << 12) + 32;

fn entropy_inputs(seed: u64) -> Vec<(String, Vec<u8>)> {
    let mut rng = Rng::new(seed);
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    for &n in &[64usize, 128, 256, 512] {
        for k in 0..3 {
            v.push((format!("rand{n}#{k}"), gen(Shape::Random, n, &mut rng)));
        }
        v.push((format!("zeros{n}"), vec![0u8; n]));
        v.push((format!("ones{n}"), vec![0xffu8; n]));
        v.push((format!("text{n}"), gen(Shape::Text, n, &mut rng)));
    }
    for n in [0usize, 1, 2, 3] {
        v.push((format!("tiny{n}"), vec![0x31u8; n]));
    }
    v
}

fn fse_version(vname: &str, inputs: &[(String, Vec<u8>)]) {
    let createDTable =
        pair_opt::<FnFseCreate>(&format!("FSE{vname}_createDTable")).unwrap();
    let freeDTable = pair_opt::<FnFseFree>(&format!("FSE{vname}_freeDTable")).unwrap();
    let buildRaw = pair_opt::<FnFseRaw>(&format!("FSE{vname}_buildDTable_raw")).unwrap();
    let buildRle = pair_opt::<FnFseRle>(&format!("FSE{vname}_buildDTable_rle")).unwrap();
    let build = pair_opt::<FnFseBuild>(&format!("FSE{vname}_buildDTable")).unwrap();
    let readNCount =
        pair_opt::<FnFseReadNCount>(&format!("FSE{vname}_readNCount")).unwrap();
    let decompress = pair_opt::<FnDec>(&format!("FSE{vname}_decompress")).unwrap();
    let decDT =
        pair_opt::<FnFseDecDT>(&format!("FSE{vname}_decompress_usingDTable")).unwrap();
    let isError = pair_opt::<FnIsError>(&format!("FSE{vname}_isError")).unwrap();
    let errName = pair_opt::<FnErrName>(&format!("FSE{vname}_getErrorName")).unwrap();

    // error helpers
    for k in 0..=130usize {
        let c = 0usize.wrapping_sub(k);
        assert_eq!(
            unsafe { isError[0](c) },
            unsafe { isError[1](c) },
            "FSE{vname}_isError(-{k})"
        );
        let a = unsafe { CStr::from_ptr(errName[0](c)) }.to_owned();
        let b = unsafe { CStr::from_ptr(errName[1](c)) }.to_owned();
        assert_eq!(a, b, "FSE{vname}_getErrorName(-{k})");
    }

    // createDTable / buildDTable_raw / buildDTable_rle / decompress_usingDTable
    for &tableLog in &[5u32, 9, 11, 12] {
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];
            let dt = unsafe { createDTable[w](tableLog) };
            r.push(Rec::Null("createDTable", dt.is_null()));
            if dt.is_null() {
                continue;
            }
            for &nbBits in &[1u32, 4, 8] {
                if nbBits > tableLog {
                    continue; // would overflow the allocated table
                }
                let rc = unsafe { buildRaw[w](dt, nbBits) };
                r.push(Rec::Rc("buildDTable_raw", rc));
                if !is_error(rc) {
                    for (lbl, src) in inputs.iter().take(8) {
                        let mut dst = vec![0u8; 4096];
                        let n = unsafe {
                            decDT[w](dst.as_mut_ptr(), 1024, src.as_ptr(), src.len(), dt)
                        };
                        r.push(Rec::Rc("decDT.raw", n));
                        let keep = if is_error(n) { 1024 } else { n.min(1024) };
                        r.push(Rec::Bytes("decDT.raw.out", dst[..keep].to_vec()));
                        let _ = lbl;
                    }
                }
            }
            for &sym in &[0u8, 127, 255] {
                let rc = unsafe { buildRle[w](dt, sym) };
                r.push(Rec::Rc("buildDTable_rle", rc));
                if !is_error(rc) {
                    for (_lbl, src) in inputs.iter().take(4) {
                        let mut dst = vec![0u8; 4096];
                        let n = unsafe {
                            decDT[w](dst.as_mut_ptr(), 512, src.as_ptr(), src.len(), dt)
                        };
                        r.push(Rec::Rc("decDT.rle", n));
                        let keep = if is_error(n) { 512 } else { n.min(512) };
                        r.push(Rec::Bytes("decDT.rle.out", dst[..keep].to_vec()));
                    }
                }
            }
            unsafe { freeDTable[w](dt) };
        }
        assert!(!recs[0].is_empty(), "FSE{vname} DTable recorded nothing");
        assert_eq!(recs[0], recs[1], "FSE{vname} DTable(tableLog={tableLog}) differs");
    }

    // readNCount + buildDTable + decompress
    for (label, src) in inputs {
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];
            let mut norm = [0i16; 256];
            let mut maxSV: c_uint = 255;
            let mut tl: c_uint = 0;
            let rc = unsafe {
                readNCount[w](
                    norm.as_mut_ptr(),
                    &mut maxSV,
                    &mut tl,
                    src.as_ptr(),
                    src.len(),
                )
            };
            r.push(Rec::Rc("readNCount", rc));
            r.push(Rec::Rc("readNCount.maxSV", maxSV as usize));
            r.push(Rec::Rc("readNCount.tableLog", tl as usize));
            r.push(Rec::Bytes(
                "readNCount.norm",
                norm.iter().flat_map(|x| x.to_le_bytes()).collect(),
            ));
            if !is_error(rc) && tl >= 5 && tl <= 12 && maxSV <= 255 {
                let dt = unsafe { createDTable[w](tl) };
                if !dt.is_null() {
                    let rb = unsafe { build[w](dt, norm.as_ptr(), maxSV, tl) };
                    r.push(Rec::Rc("buildDTable", rb));
                    if !is_error(rb) && src.len() > rc {
                        let mut dst = vec![0u8; 4096];
                        let n = unsafe {
                            decDT[w](
                                dst.as_mut_ptr(),
                                1024,
                                src.as_ptr().add(rc),
                                src.len() - rc,
                                dt,
                            )
                        };
                        r.push(Rec::Rc("decDT.built", n));
                        let keep = if is_error(n) { 1024 } else { n.min(1024) };
                        r.push(Rec::Bytes("decDT.built.out", dst[..keep].to_vec()));
                    }
                    unsafe { freeDTable[w](dt) };
                }
            }
            // one-shot FSE decompress
            for &cap in &[0usize, 256, 4096] {
                let mut dst = vec![0u8; cap.max(1)];
                let n = unsafe {
                    decompress[w](dst.as_mut_ptr(), cap, src.as_ptr(), src.len())
                };
                r.push(Rec::Rc("FSE_decompress", n));
                let keep = if is_error(n) { cap } else { n.min(cap) };
                r.push(Rec::Bytes("FSE_decompress.out", dst[..keep].to_vec()));
            }
        }
        assert!(!recs[0].is_empty(), "FSE{vname} readNCount recorded nothing");
        assert_eq!(recs[0], recs[1], "FSE{vname} readNCount/decompress differs on `{label}`");
    }
}

fn huf_version(vname: &str, inputs: &[(String, Vec<u8>)]) {
    let readX2 =
        pair_opt::<FnHufReadDT16>(&format!("HUF{vname}_readDTableX2"));
    let readX2_32 =
        pair_opt::<FnHufReadDT32>(&format!("HUF{vname}_readDTableX2"));
    let readX4 = pair_opt::<FnHufReadDT32>(&format!("HUF{vname}_readDTableX4")).unwrap();
    let decompress = pair_opt::<FnDec>(&format!("HUF{vname}_decompress")).unwrap();
    let d1x2 = pair_opt::<FnDec>(&format!("HUF{vname}_decompress1X2")).unwrap();
    let d1x4 = pair_opt::<FnDec>(&format!("HUF{vname}_decompress1X4")).unwrap();
    let d4x2 = pair_opt::<FnDec>(&format!("HUF{vname}_decompress4X2")).unwrap();
    let d4x4 = pair_opt::<FnDec>(&format!("HUF{vname}_decompress4X4")).unwrap();
    // v05/v06 use u16 X2 tables, v07 uses u32 everywhere
    let is_v07 = vname == "v07";

    let d1x2dt16 =
        pair_opt::<FnHufDecDT16>(&format!("HUF{vname}_decompress1X2_usingDTable"));
    let d4x2dt16 =
        pair_opt::<FnHufDecDT16>(&format!("HUF{vname}_decompress4X2_usingDTable"));
    let d1x2dt32 =
        pair_opt::<FnHufDecDT32>(&format!("HUF{vname}_decompress1X2_usingDTable"));
    let d4x2dt32 =
        pair_opt::<FnHufDecDT32>(&format!("HUF{vname}_decompress4X2_usingDTable"));
    let d1x4dt =
        pair_opt::<FnHufDecDT32>(&format!("HUF{vname}_decompress1X4_usingDTable")).unwrap();
    let d4x4dt =
        pair_opt::<FnHufDecDT32>(&format!("HUF{vname}_decompress4X4_usingDTable")).unwrap();

    let isError = pair_opt::<FnIsError>(&format!("HUF{vname}_isError"));
    let errName = pair_opt::<FnErrName>(&format!("HUF{vname}_getErrorName"));
    if let (Some(ie), Some(en)) = (isError, errName) {
        for k in 0..=130usize {
            let c = 0usize.wrapping_sub(k);
            assert_eq!(
                unsafe { ie[0](c) },
                unsafe { ie[1](c) },
                "HUF{vname}_isError(-{k})"
            );
            let a = unsafe { CStr::from_ptr(en[0](c)) }.to_owned();
            let b = unsafe { CStr::from_ptr(en[1](c)) }.to_owned();
            assert_eq!(a, b, "HUF{vname}_getErrorName(-{k})");
        }
    }

    // v07-only extras
    let readStats = pair_opt::<FnHufReadStats>("HUFv07_readStats");
    let selectDecoder = pair_opt::<FnHufSelect>("HUFv07_selectDecoder");
    let dctx_fns: Vec<(&'static str, [FnHufDecCtx; 2])> = if is_v07 {
        vec![
            ("1X2_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress1X2_DCtx").unwrap()),
            ("1X4_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress1X4_DCtx").unwrap()),
            ("1X_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress1X_DCtx").unwrap()),
            ("4X2_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress4X2_DCtx").unwrap()),
            ("4X4_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress4X4_DCtx").unwrap()),
            ("4X_DCtx", pair_opt::<FnHufDecCtx>("HUFv07_decompress4X_DCtx").unwrap()),
            ("4X_hufOnly", pair_opt::<FnHufDecCtx>("HUFv07_decompress4X_hufOnly").unwrap()),
        ]
    } else {
        vec![]
    };
    let d1xdt = pair_opt::<FnHufDecDT32>("HUFv07_decompress1X_usingDTable");
    let d4xdt = pair_opt::<FnHufDecDT32>("HUFv07_decompress4X_usingDTable");

    if let Some(sel) = selectDecoder {
        for &dstSize in &[1usize, 100, 256, 4096, 1 << 17] {
            for &cSrcSize in &[1usize, 2, 99, 255, 4095] {
                if cSrcSize >= dstSize {
                    continue; // documented precondition: cSrcSize < dstSize
                }
                assert_eq!(
                    unsafe { sel[0](dstSize, cSrcSize) },
                    unsafe { sel[1](dstSize, cSrcSize) },
                    "HUFv07_selectDecoder({dstSize},{cSrcSize})"
                );
            }
        }
    }

    for (label, src) in inputs {
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let r = &mut recs[w];

            // one-shot decoders
            for &cap in &[0usize, 256, 512, 4096] {
                for (nm, f) in [
                    ("decompress", decompress),
                    ("1X2", d1x2),
                    ("1X4", d1x4),
                    ("4X2", d4x2),
                    ("4X4", d4x4),
                ] {
                    let mut dst = vec![0u8; cap.max(1)];
                    let n =
                        unsafe { f[w](dst.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                    r.push(Rec::Rc(nm, n));
                    let keep = if is_error(n) { cap } else { n.min(cap) };
                    r.push(Rec::Bytes("out", dst[..keep].to_vec()));
                }
            }

            // readDTableX2 (u16 tables for v05/v06, u32 for v07)
            if is_v07 {
                let f = readX2_32.unwrap();
                let mut dt = vec![0u32; HUF_DT_U32];
                dt[0] = 11 * 0x0100_0001; // DTableDesc{ maxTableLog: 11, .. }
                let rc = unsafe { f[w](dt.as_mut_ptr(), src.as_ptr(), src.len()) };
                r.push(Rec::Rc("readDTableX2", rc));
                r.push(Rec::Bytes(
                    "readDTableX2.table",
                    dt.iter().flat_map(|x| x.to_le_bytes()).collect(),
                ));
                if !is_error(rc) {
                    for &cap in &[256usize, 512] {
                        for (nm, g) in [
                            ("1X2dt", d1x2dt32.unwrap()),
                            ("4X2dt", d4x2dt32.unwrap()),
                            ("1Xdt", d1xdt.unwrap()),
                            ("4Xdt", d4xdt.unwrap()),
                        ] {
                            let mut dst = vec![0u8; cap];
                            let n = unsafe {
                                g[w](
                                    dst.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    src.len(),
                                    dt.as_ptr(),
                                )
                            };
                            r.push(Rec::Rc(nm, n));
                            let keep = if is_error(n) { cap } else { n.min(cap) };
                            r.push(Rec::Bytes("dtout", dst[..keep].to_vec()));
                        }
                    }
                }
            } else {
                let f = readX2.unwrap();
                let mut dt = vec![0u16; HUF_DT_U16];
                dt[0] = 12;
                let rc = unsafe { f[w](dt.as_mut_ptr(), src.as_ptr(), src.len()) };
                r.push(Rec::Rc("readDTableX2", rc));
                r.push(Rec::Bytes(
                    "readDTableX2.table",
                    dt.iter().flat_map(|x| x.to_le_bytes()).collect(),
                ));
                if !is_error(rc) {
                    for &cap in &[256usize, 512] {
                        for (nm, g) in
                            [("1X2dt", d1x2dt16.unwrap()), ("4X2dt", d4x2dt16.unwrap())]
                        {
                            let mut dst = vec![0u8; cap];
                            let n = unsafe {
                                g[w](
                                    dst.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    src.len(),
                                    dt.as_ptr(),
                                )
                            };
                            r.push(Rec::Rc(nm, n));
                            let keep = if is_error(n) { cap } else { n.min(cap) };
                            r.push(Rec::Bytes("dtout", dst[..keep].to_vec()));
                        }
                    }
                }
            }

            // readDTableX4 (always u32)
            {
                let mut dt = vec![0u32; HUF_DT_U32];
                dt[0] = if is_v07 { 12 * 0x0100_0001 } else { 12 };
                let rc = unsafe { readX4[w](dt.as_mut_ptr(), src.as_ptr(), src.len()) };
                r.push(Rec::Rc("readDTableX4", rc));
                r.push(Rec::Bytes(
                    "readDTableX4.table",
                    dt.iter().flat_map(|x| x.to_le_bytes()).collect(),
                ));
                if !is_error(rc) {
                    for &cap in &[256usize, 512] {
                        for (nm, g) in [("1X4dt", d1x4dt), ("4X4dt", d4x4dt)] {
                            let mut dst = vec![0u8; cap];
                            let n = unsafe {
                                g[w](
                                    dst.as_mut_ptr(),
                                    cap,
                                    src.as_ptr(),
                                    src.len(),
                                    dt.as_ptr(),
                                )
                            };
                            r.push(Rec::Rc(nm, n));
                            let keep = if is_error(n) { cap } else { n.min(cap) };
                            r.push(Rec::Bytes("dtout", dst[..keep].to_vec()));
                        }
                    }
                }
            }

            // v07 _DCtx entry points (the DTable doubles as the workspace)
            for (nm, f) in &dctx_fns {
                for &cap in &[512usize, 4096] {
                    if *nm == "4X_hufOnly" && (src.len() >= cap || src.len() <= 1) {
                        continue; // documented precondition
                    }
                    let mut dt = vec![0u32; HUF_DT_U32];
                    dt[0] = 12 * 0x0100_0001;
                    let mut dst = vec![0u8; cap];
                    let n = unsafe {
                        f[w](
                            dt.as_mut_ptr(),
                            dst.as_mut_ptr(),
                            cap,
                            src.as_ptr(),
                            src.len(),
                        )
                    };
                    r.push(Rec::Rc(nm, n));
                    let keep = if is_error(n) { cap } else { n.min(cap) };
                    r.push(Rec::Bytes("dctxout", dst[..keep].to_vec()));
                    r.push(Rec::Bytes(
                        "dctxtable",
                        dt.iter().flat_map(|x| x.to_le_bytes()).collect(),
                    ));
                }
            }

            // HUFv07_readStats
            if let Some(f) = readStats {
                let mut weights = vec![0u8; 256];
                let mut rank = vec![0u32; 32];
                let mut nbSymbols: u32 = 0;
                let mut tableLog: u32 = 0;
                let rc = unsafe {
                    f[w](
                        weights.as_mut_ptr(),
                        256,
                        rank.as_mut_ptr(),
                        &mut nbSymbols,
                        &mut tableLog,
                        src.as_ptr(),
                        src.len(),
                    )
                };
                r.push(Rec::Rc("readStats", rc));
                r.push(Rec::Rc("readStats.nbSymbols", nbSymbols as usize));
                r.push(Rec::Rc("readStats.tableLog", tableLog as usize));
                r.push(Rec::Bytes("readStats.weights", weights.clone()));
                r.push(Rec::Bytes(
                    "readStats.rank",
                    rank.iter().flat_map(|x| x.to_le_bytes()).collect(),
                ));
            }
        }
        assert!(recs[0].len() >= 10, "HUF{vname} recorded nothing");
        assert_eq!(recs[0], recs[1], "HUF{vname} differs on `{label}`");
    }
}

#[test]
fn legacy_entropy() {
    let inputs = entropy_inputs(0xE117);
    for v in ["v05", "v06", "v07"] {
        fse_version(v, &inputs);
    }
    for v in ["v05", "v06", "v07"] {
        huf_version(v, &inputs);
    }
}
