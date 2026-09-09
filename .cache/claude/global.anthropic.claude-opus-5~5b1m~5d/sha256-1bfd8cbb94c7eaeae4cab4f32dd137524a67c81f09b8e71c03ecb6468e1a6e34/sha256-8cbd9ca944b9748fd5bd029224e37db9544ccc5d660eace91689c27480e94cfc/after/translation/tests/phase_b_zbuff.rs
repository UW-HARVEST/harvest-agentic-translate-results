//! Phase B: the deprecated ZBUFF_* buffered streaming API
//! (headers: c_src/src/deprecated/zbuff.h).
//!
//! Every call's return value AND the values written back through the
//! `size_t*` in/out pointers are recorded and compared between the C ground
//! truth and the Rust port, together with the full output byte stream.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// ------------------------------------------------------------ ABI types ----

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CParams {
    windowLog: c_uint,
    chainLog: c_uint,
    hashLog: c_uint,
    searchLog: c_uint,
    minMatch: c_uint,
    targetLength: c_uint,
    strategy: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FParams {
    contentSizeFlag: c_int,
    checksumFlag: c_int,
    noDictIDFlag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Params {
    cParams: CParams,
    fParams: FParams,
}

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

// --------------------------------------------------------- symbol types ----

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSize = unsafe extern "C" fn() -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

type FnCInit = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnCInitDict = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnCInitAdv = unsafe extern "C" fn(*mut c_void, *const u8, usize, Params, u64) -> usize;
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize, *const u8, *mut usize) -> usize;
type FnFlushEnd = unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize) -> usize;
type FnDInit = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDInitDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;

type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnGetParams = unsafe extern "C" fn(c_int, u64, usize) -> Params;

/// Fetch a symbol pair and immediately deref both `Symbol`s to plain fn
/// pointers: `[C, Rust]`. Index 0 is always the C ground truth.
macro_rules! both {
    ($name:expr, $t:ty) => {{
        let (c, r) = unsafe { pair::<$t>($name) };
        [*c, *r]
    }};
}

// --------------------------------------------------------------- drivers ---

/// Record of one ZBUFF call: return value plus everything written back
/// through the `size_t*` parameters.
#[derive(Debug, PartialEq, Eq, Clone)]
enum Rec {
    Cont { rc: usize, dst: usize, src: usize },
    Flush { rc: usize, dst: usize },
    End { rc: usize, dst: usize },
    Init(usize),
}

struct CompressOut {
    bytes: Vec<u8>,
    recs: Vec<Rec>,
}

/// Drive compressContinue over `src` in `in_chunk` pieces, then flush, then end.
fn drive_compress(
    zbc: *mut c_void,
    cont: FnContinue,
    flush: FnFlushEnd,
    end: FnFlushEnd,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    ctx: &str,
) -> CompressOut {
    let mut bytes = Vec::new();
    let mut recs = Vec::new();
    let mut outbuf = vec![0u8; out_chunk.max(1)];
    let mut pos = 0usize;
    let mut guard = 0u64;
    while pos < src.len() {
        guard += 1;
        assert!(guard < 20_000_000, "{ctx}: compressContinue loop stuck");
        let mut s = in_chunk.min(src.len() - pos);
        let mut d = outbuf.len();
        let rc = unsafe {
            cont(zbc, outbuf.as_mut_ptr(), &mut d, src.as_ptr().add(pos), &mut s)
        };
        recs.push(Rec::Cont { rc, dst: d, src: s });
        if is_error(rc) {
            return CompressOut { bytes, recs };
        }
        assert!(d <= outbuf.len(), "{ctx}: dst overflow reported");
        bytes.extend_from_slice(&outbuf[..d]);
        pos += s;
        if s == 0 && d == 0 {
            break; // no forward progress possible
        }
    }
    for stage in 0..2 {
        let f = if stage == 0 { flush } else { end };
        let mut guard2 = 0u64;
        loop {
            guard2 += 1;
            assert!(guard2 < 20_000_000, "{ctx}: flush/end loop stuck");
            let mut d = outbuf.len();
            let rc = unsafe { f(zbc, outbuf.as_mut_ptr(), &mut d) };
            recs.push(if stage == 0 {
                Rec::Flush { rc, dst: d }
            } else {
                Rec::End { rc, dst: d }
            });
            if is_error(rc) {
                return CompressOut { bytes, recs };
            }
            bytes.extend_from_slice(&outbuf[..d]);
            if rc == 0 || d == 0 {
                break;
            }
        }
    }
    CompressOut { bytes, recs }
}

/// Drive decompressContinue over `frame` in `in_chunk` pieces.
fn drive_decompress(
    zbd: *mut c_void,
    cont: FnContinue,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    ctx: &str,
) -> CompressOut {
    let mut bytes = Vec::new();
    let mut recs = Vec::new();
    let mut outbuf = vec![0u8; out_chunk.max(1)];
    let mut pos = 0usize;
    let mut guard = 0u64;
    loop {
        guard += 1;
        assert!(guard < 20_000_000, "{ctx}: decompressContinue loop stuck");
        let mut s = in_chunk.min(frame.len() - pos);
        let mut d = outbuf.len();
        let rc = unsafe {
            cont(zbd, outbuf.as_mut_ptr(), &mut d, frame.as_ptr().add(pos), &mut s)
        };
        recs.push(Rec::Cont { rc, dst: d, src: s });
        if is_error(rc) {
            break;
        }
        bytes.extend_from_slice(&outbuf[..d]);
        pos += s;
        if rc == 0 {
            break;
        }
        if s == 0 && d == 0 && pos == frame.len() {
            break; // truncated input: nothing more can happen
        }
    }
    CompressOut { bytes, recs }
}

/// Round-trip helper: compress `src` in both libs with the given init closure,
/// compare everything, decode the frame in both libs and compare to `src`.
fn zbuff_roundtrip(
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    init: &dyn Fn(usize, *mut c_void) -> usize,
    dict: Option<&[u8]>,
    ctx: &str,
) -> Vec<u8> {
    let create = both!("ZBUFF_createCCtx", FnCreate);
    let freec = both!("ZBUFF_freeCCtx", FnFree);
    let cont = both!("ZBUFF_compressContinue", FnContinue);
    let flush = both!("ZBUFF_compressFlush", FnFlushEnd);
    let end = both!("ZBUFF_compressEnd", FnFlushEnd);

    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let zbc = unsafe { create[w]() };
        assert!(!zbc.is_null(), "{ctx}: createCCtx returned NULL");
        let rc0 = init(w, zbc);
        recs[w].push(Rec::Init(rc0));
        assert!(!is_error(rc0), "{ctx}: init failed {}", err_code(rc0));
        let o = drive_compress(zbc, cont[w], flush[w], end[w], src, in_chunk, out_chunk, ctx);
        recs[w].extend(o.recs);
        outs[w] = o.bytes;
        let fr = unsafe { freec[w](zbc) };
        recs[w].push(Rec::Init(fr));
    }
    assert_eq!(recs[0], recs[1], "{ctx}: ZBUFF compress call/ptr-value sequence differs");
    assert_bytes_eq(&format!("{ctx}: zbuff frame"), &outs[0], &outs[1]);

    // Decode with ZSTD_decompress in both libs and check == src.
    let dec = both!("ZSTD_decompress", FnDecompress);
    let ddict = both!("ZSTD_decompress_usingDict", unsafe extern "C" fn(
        *mut c_void, *mut u8, usize, *const u8, usize, *const u8, usize,
    ) -> usize);
    let dctx_new = both!("ZSTD_createDCtx", FnCreate);
    let dctx_free = both!("ZSTD_freeDCtx", FnFree);
    let cap = src.len() + 1024;
    for w in 0..2 {
        let mut buf = vec![0u8; cap];
        let n = match dict {
            None => unsafe { dec[w](buf.as_mut_ptr(), cap, outs[0].as_ptr(), outs[0].len()) },
            Some(d) => {
                let dc = unsafe { dctx_new[w]() };
                let n = unsafe {
                    ddict[w](
                        dc,
                        buf.as_mut_ptr(),
                        cap,
                        outs[0].as_ptr(),
                        outs[0].len(),
                        d.as_ptr(),
                        d.len(),
                    )
                };
                unsafe { dctx_free[w](dc) };
                n
            }
        };
        assert!(!is_error(n), "{ctx}: decode(lib {w}) err {}", err_code(n));
        assert_bytes_eq(&format!("{ctx}: decoded (lib {w}) vs orig"), &buf[..n], src);
    }
    outs[0].clone()
}

// ------------------------------------------------------ CONFIGS row 170 ----

#[test]
fn cfg_zbuff_compress() {
    let cinit = both!("ZBUFF_compressInit", FnCInit);
    let cout = both!("ZBUFF_recommendedCOutSize", FnSize);
    let out_rec = unsafe { cout[0]() };
    assert_eq!(out_rec, unsafe { cout[1]() }, "recommendedCOutSize differs");

    let mut rng = Rng::new(0xB0FF);
    let src = gen(Shape::Text, 100000, &mut rng);
    let init = |w: usize, z: *mut c_void| unsafe { cinit[w](z, 3) };

    // row 170: in 1000, out = ZBUFF_recommendedCOutSize(), 100000 B text
    zbuff_roundtrip(&src, 1000, out_rec, &init, None, "zbuff cfg170 in=1000 out=COutSize");

    // ... plus degenerate in/out chunks of 1 and 7.
    for &(i, o) in &[(1usize, 1usize), (1, 7), (7, 1), (7, 7), (1, out_rec), (7, 1000)] {
        let ctx = format!("zbuff compress in={i} out={o}");
        zbuff_roundtrip(&src, i, o, &init, None, &ctx);
    }

    // several randomized buffers / shapes / levels for good measure
    for round in 0..8 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = rng.range(0, 120000);
        let s = gen(shape, size, &mut rng);
        let lvl = [1, 3, 6, 12, 19][rng.below(5)];
        let ini = |w: usize, z: *mut c_void| unsafe { cinit[w](z, lvl) };
        let ctx = format!("zbuff compress round={round} shape={shape:?} size={size} lvl={lvl}");
        zbuff_roundtrip(&s, rng.range(1, 40000), rng.range(1, 40000), &ini, None, &ctx);
    }
}

// ------------------------------------------------------ CONFIGS row 171 ----

#[test]
fn cfg_zbuff_compress_advanced() {
    let create_adv = both!("ZBUFF_createCCtx_advanced", FnCreateAdv);
    let freec = both!("ZBUFF_freeCCtx", FnFree);
    let init_adv = both!("ZBUFF_compressInit_advanced", FnCInitAdv);
    let cont = both!("ZBUFF_compressContinue", FnContinue);
    let flush = both!("ZBUFF_compressFlush", FnFlushEnd);
    let end = both!("ZBUFF_compressEnd", FnFlushEnd);
    let getp = both!("ZSTD_getParams", FnGetParams);
    let dec = both!("ZSTD_decompress", FnDecompress);

    let mut rng = Rng::new(0xADFF);

    for round in 0..10 {
        let shape = if round == 0 { Shape::Text } else { ALL_SHAPES[rng.below(ALL_SHAPES.len())] };
        let size = if round == 0 { 65536 } else { rng.range(0, 200000) };
        let src = gen(shape, size, &mut rng);
        let lvl = [1, 3, 5, 9, 17, 19][rng.below(6)];

        // ZSTD_getParams must agree between the two libraries.
        let pc = unsafe { getp[0](lvl, src.len() as u64, 0) };
        let pr = unsafe { getp[1](lvl, src.len() as u64, 0) };
        assert_eq!(pc, pr, "ZSTD_getParams({lvl},{},0) differs", src.len());

        for &pledge_exact in &[true, false] {
            for &cksum in &[1, 0] {
                let mut p = pc;
                p.fParams.checksumFlag = cksum;
                p.fParams.contentSizeFlag = 1;
                p.fParams.noDictIDFlag = 0;
                let pledged = if pledge_exact { src.len() as u64 } else { 0 };
                let in_chunk = if round == 0 { 7 } else { rng.range(1, 50000) };
                let out_chunk = if round == 0 { 7 } else { rng.range(1, 50000) };
                let ctx = format!(
                    "zbuff adv round={round} shape={shape:?} size={size} lvl={lvl} cksum={cksum} pledge={pledged} in={in_chunk} out={out_chunk}"
                );

                let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
                let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
                for w in 0..2 {
                    let zbc = unsafe { create_adv[w](NO_CUSTOM_MEM) };
                    assert!(!zbc.is_null(), "{ctx}: createCCtx_advanced NULL");
                    let rc0 =
                        unsafe { init_adv[w](zbc, std::ptr::null(), 0, p, pledged) };
                    recs[w].push(Rec::Init(rc0));
                    assert!(!is_error(rc0), "{ctx}: init_advanced err {}", err_code(rc0));
                    let o = drive_compress(
                        zbc, cont[w], flush[w], end[w], &src, in_chunk, out_chunk, &ctx,
                    );
                    recs[w].extend(o.recs);
                    outs[w] = o.bytes;
                    recs[w].push(Rec::Init(unsafe { freec[w](zbc) }));
                }
                assert_eq!(recs[0], recs[1], "{ctx}: advanced call sequence differs");
                assert_bytes_eq(&format!("{ctx}: frame"), &outs[0], &outs[1]);
                for w in 0..2 {
                    let cap = src.len() + 1024;
                    let mut buf = vec![0u8; cap];
                    let n = unsafe {
                        dec[w](buf.as_mut_ptr(), cap, outs[0].as_ptr(), outs[0].len())
                    };
                    assert!(!is_error(n), "{ctx}: decode err {}", err_code(n));
                    assert_bytes_eq(&format!("{ctx}: decoded lib{w}"), &buf[..n], &src);
                }
            }
        }
    }
}

// ------------------------------------------------- CONFIGS rows 172, 174 ---

#[test]
fn cfg_zbuff_dict() {
    let cinit_dict = both!("ZBUFF_compressInitDictionary", FnCInitDict);
    let dcreate = both!("ZBUFF_createDCtx", FnCreate);
    let dfree = both!("ZBUFF_freeDCtx", FnFree);
    let dinit_dict = both!("ZBUFF_decompressInitDictionary", FnDInitDict);
    let dcont = both!("ZBUFF_decompressContinue", FnContinue);

    let mut rng = Rng::new(0xD1C7);
    // 8192 B raw dictionary (not a zstd dictionary => loaded as raw content)
    let dict = gen(Shape::Text, 8192, &mut rng);

    for round in 0..6 {
        let shape = if round == 0 { Shape::Text } else { ALL_SHAPES[rng.below(ALL_SHAPES.len())] };
        let size = if round == 0 { 60000 } else { rng.range(0, 150000) };
        let src = gen(shape, size, &mut rng);
        let lvl = if round == 0 { 6 } else { [1, 4, 9, 16][rng.below(4)] };
        let init = |w: usize, z: *mut c_void| unsafe {
            cinit_dict[w](z, dict.as_ptr(), dict.len(), lvl)
        };
        let ctx = format!("zbuff dict round={round} shape={shape:?} size={size} lvl={lvl}");
        let frame = zbuff_roundtrip(&src, 4096, 4096, &init, Some(&dict), &ctx);

        // row 174: ZBUFF_decompressInitDictionary + decompressContinue, out-chunk 7
        for &(i, o) in &[(7usize, 7usize), (1, 7), (4096, 7), (100, 1000)] {
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
            for w in 0..2 {
                let zbd = unsafe { dcreate[w]() };
                assert!(!zbd.is_null(), "{ctx}: createDCtx NULL");
                let rc0 = unsafe { dinit_dict[w](zbd, dict.as_ptr(), dict.len()) };
                recs[w].push(Rec::Init(rc0));
                assert!(!is_error(rc0), "{ctx}: decompressInitDictionary err");
                let d = drive_decompress(zbd, dcont[w], &frame, i, o, &ctx);
                recs[w].extend(d.recs);
                outs[w] = d.bytes;
                recs[w].push(Rec::Init(unsafe { dfree[w](zbd) }));
            }
            let c2 = format!("{ctx} dictdecode in={i} out={o}");
            assert_eq!(recs[0], recs[1], "{c2}: decompress call sequence differs");
            assert_bytes_eq(&format!("{c2}: bytes"), &outs[0], &outs[1]);
            assert_bytes_eq(&format!("{c2}: vs orig"), &outs[0], &src);
        }

        // A dictionary-compressed frame decoded *without* the dictionary must
        // fail identically in both libraries.
        let dcreate2 = both!("ZBUFF_createDCtx", FnCreate);
        let dinit = both!("ZBUFF_decompressInit", FnDInit);
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let zbd = unsafe { dcreate2[w]() };
            recs[w].push(Rec::Init(unsafe { dinit[w](zbd) }));
            let d = drive_decompress(zbd, dcont[w], &frame, 1000, 1000, &ctx);
            recs[w].extend(d.recs);
            recs[w].push(Rec::Init(unsafe { dfree[w](zbd) }));
        }
        assert_eq!(recs[0], recs[1], "{ctx}: no-dict decode sequence differs");
    }
}

// ------------------------------------------------------ CONFIGS row 173 ----

#[test]
fn cfg_zbuff_decompress() {
    let cinit = both!("ZBUFF_compressInit", FnCInit);
    let dcreate = both!("ZBUFF_createDCtx", FnCreate);
    let dcreate_adv = both!("ZBUFF_createDCtx_advanced", FnCreateAdv);
    let dfree = both!("ZBUFF_freeDCtx", FnFree);
    let dinit = both!("ZBUFF_decompressInit", FnDInit);
    let dcont = both!("ZBUFF_decompressContinue", FnContinue);
    let din = both!("ZBUFF_recommendedDInSize", FnSize);
    let dout = both!("ZBUFF_recommendedDOutSize", FnSize);

    let in_rec = unsafe { din[0]() };
    let out_rec = unsafe { dout[0]() };
    assert_eq!(in_rec, unsafe { din[1]() }, "recommendedDInSize differs");
    assert_eq!(out_rec, unsafe { dout[1]() }, "recommendedDOutSize differs");

    let mut rng = Rng::new(0xDEC0);
    let mut cases: Vec<(Shape, usize, c_int)> = vec![(Shape::Text, 100000, 3)];
    for _ in 0..4 {
        cases.push((
            ALL_SHAPES[rng.below(ALL_SHAPES.len())],
            rng.range(0, 150000),
            [1, 5, 11, 19][rng.below(4)],
        ));
    }

    for (shape, size, lvl) in cases {
        let src = gen(shape, size, &mut rng);
        let init = |w: usize, z: *mut c_void| unsafe { cinit[w](z, lvl) };
        let base = format!("zbuff dec shape={shape:?} size={size} lvl={lvl}");
        let frame = zbuff_roundtrip(&src, 16384, 16384, &init, None, &base);

        for &advanced in &[false, true] {
            for &i in &[1usize, in_rec] {
                for &o in &[7usize, 1000, out_rec] {
                    let ctx = format!("{base} adv={advanced} in={i} out={o}");
                    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
                    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
                    for w in 0..2 {
                        let zbd = if advanced {
                            unsafe { dcreate_adv[w](NO_CUSTOM_MEM) }
                        } else {
                            unsafe { dcreate[w]() }
                        };
                        assert!(!zbd.is_null(), "{ctx}: createDCtx NULL");
                        let rc0 = unsafe { dinit[w](zbd) };
                        recs[w].push(Rec::Init(rc0));
                        assert!(!is_error(rc0), "{ctx}: decompressInit err");
                        let d = drive_decompress(zbd, dcont[w], &frame, i, o, &ctx);
                        recs[w].extend(d.recs);
                        outs[w] = d.bytes;
                        recs[w].push(Rec::Init(unsafe { dfree[w](zbd) }));
                    }
                    assert_eq!(recs[0], recs[1], "{ctx}: decompress call sequence differs");
                    assert_bytes_eq(&format!("{ctx}: bytes"), &outs[0], &outs[1]);
                    assert_bytes_eq(&format!("{ctx}: vs orig"), &outs[0], &src);
                }
            }
        }
    }
}

// -------------------------------------- CONFIGS row 175 / ERRORS row 272 ---

#[test]
fn cfg_zbuff_helpers() {
    for name in [
        "ZBUFF_recommendedCInSize",
        "ZBUFF_recommendedCOutSize",
        "ZBUFF_recommendedDInSize",
        "ZBUFF_recommendedDOutSize",
    ] {
        let f = both!(name, FnSize);
        assert_eq!(unsafe { f[0]() }, unsafe { f[1]() }, "{name}");
    }

    let iserr = both!("ZBUFF_isError", FnIsError);
    let errname = both!("ZBUFF_getErrorName", FnErrName);
    let z_iserr = both!("ZSTD_isError", FnIsError);
    let z_errname = both!("ZSTD_getErrorName", FnErrName);

    let mut codes: Vec<usize> = vec![
        0,
        1,
        usize::MAX,                  // (size_t)-1
        0usize.wrapping_sub(72),
        0usize.wrapping_sub(120),
        0usize.wrapping_sub(121),
    ];
    // full sweep of every error code plus a few sizes
    for k in 0..=130usize {
        codes.push(0usize.wrapping_sub(k));
    }
    for k in [2usize, 100, 1 << 20, usize::MAX / 2] {
        codes.push(k);
    }

    for &code in &codes {
        let ec = unsafe { iserr[0](code) };
        let er = unsafe { iserr[1](code) };
        assert_eq!(ec, er, "ZBUFF_isError({code:#x})");
        // and it must match ZSTD_isError, per ERRORS row 272
        assert_eq!(ec, unsafe { z_iserr[0](code) }, "ZBUFF vs ZSTD isError({code:#x}) in C");
        assert_eq!(er, unsafe { z_iserr[1](code) }, "ZBUFF vs ZSTD isError({code:#x}) in Rust");

        let nc = unsafe { CStr::from_ptr(errname[0](code)) }.to_owned();
        let nr = unsafe { CStr::from_ptr(errname[1](code)) }.to_owned();
        assert_eq!(nc, nr, "ZBUFF_getErrorName({code:#x})");
        let zc = unsafe { CStr::from_ptr(z_errname[0](code)) }.to_owned();
        let zr = unsafe { CStr::from_ptr(z_errname[1](code)) }.to_owned();
        assert_eq!(nc, zc, "ZBUFF vs ZSTD getErrorName({code:#x}) in C");
        assert_eq!(nr, zr, "ZBUFF vs ZSTD getErrorName({code:#x}) in Rust");
    }
}

// ------------------------------------------------- ERRORS rows 274, 275 ----

#[test]
fn err_zbuff_params() {
    let create = both!("ZBUFF_createCCtx", FnCreate);
    let freec = both!("ZBUFF_freeCCtx", FnFree);
    let init_adv = both!("ZBUFF_compressInit_advanced", FnCInitAdv);
    let init_dict = both!("ZBUFF_compressInitDictionary", FnCInitDict);
    let cinit = both!("ZBUFF_compressInit", FnCInit);
    let cont = both!("ZBUFF_compressContinue", FnContinue);
    let getp = both!("ZSTD_getParams", FnGetParams);

    let base = unsafe { getp[0](3, 100000, 0) };
    assert_eq!(base, unsafe { getp[1](3, 100000, 0) }, "getParams(3,100000,0)");

    // ---- row 274: out-of-bounds cParams => parameter_outOfBound ----
    // `must_fail` marks values that are outside the documented bounds
    // (zstd.h: WINDOWLOG 10..31, CHAINLOG 6..30, HASHLOG 6..30,
    //  SEARCHLOG 1..30, MINMATCH 3..7, STRATEGY 1..9).
    let mut cases: Vec<(String, Params, bool)> = Vec::new();
    for &(v, must_fail) in &[(0u32, true), (1, true), (9, true), (10, false), (31, false), (32, true), (64, true), (u32::MAX, true)] {
        let mut p = base;
        p.cParams.windowLog = v;
        cases.push((format!("windowLog={v}"), p, must_fail));
    }
    for &(v, must_fail) in &[(0u32, true), (1, true), (5, true), (6, false), (30, false), (31, true), (40, true), (u32::MAX, true)] {
        let mut p = base;
        p.cParams.chainLog = v;
        cases.push((format!("chainLog={v}"), p, must_fail));
        let mut p = base;
        p.cParams.hashLog = v;
        cases.push((format!("hashLog={v}"), p, must_fail));
    }
    for &(v, must_fail) in &[(0u32, true), (1, false), (5, false), (30, false), (31, true), (40, true), (u32::MAX, true)] {
        let mut p = base;
        p.cParams.searchLog = v;
        cases.push((format!("searchLog={v}"), p, must_fail));
    }
    for &(v, must_fail) in &[(0u32, true), (2, true), (3, false), (7, false), (8, true), (100, true)] {
        let mut p = base;
        p.cParams.minMatch = v;
        cases.push((format!("minMatch={v}"), p, must_fail));
    }
    for &(v, must_fail) in &[(-1i32, true), (0, true), (1, false), (9, false), (10, true), (99, true)] {
        let mut p = base;
        p.cParams.strategy = v;
        cases.push((format!("strategy={v}"), p, must_fail));
    }
    for &v in &[0u32, 1, 131072, 131073, u32::MAX] {
        let mut p = base;
        p.cParams.targetLength = v;
        cases.push((format!("targetLength={v}"), p, false));
    }
    // and a sane one as a control
    cases.push(("control-valid".to_string(), base, false));

    for (label, p, must_fail) in &cases {
        let mut rcs = [0usize; 2];
        for w in 0..2 {
            let z = unsafe { create[w]() };
            rcs[w] = unsafe { init_adv[w](z, std::ptr::null(), 0, *p, 0) };
            unsafe { freec[w](z) };
        }
        assert_eq!(
            rcs[0], rcs[1],
            "compressInit_advanced {label}: C={} ({}) Rust={} ({})",
            rcs[0] as isize,
            err_code(rcs[0]),
            rcs[1] as isize,
            err_code(rcs[1])
        );
        if *must_fail {
            assert!(is_error(rcs[0]), "{label} should be rejected (C returned {})", rcs[0]);
        }
    }

    // ---- row 275: (re)init called mid-compression ----
    let mut rng = Rng::new(0x2755);
    let src = gen(Shape::Text, 60000, &mut rng);
    let dict = gen(Shape::Text, 4096, &mut rng);
    let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
    for w in 0..2 {
        let z = unsafe { create[w]() };
        recs[w].push(Rec::Init(unsafe { cinit[w](z, 5) }));
        // consume some input so the context is mid-frame
        let mut outbuf = vec![0u8; 1 << 12];
        let mut s = src.len();
        let mut d = outbuf.len();
        let rc = unsafe { cont[w](z, outbuf.as_mut_ptr(), &mut d, src.as_ptr(), &mut s) };
        recs[w].push(Rec::Cont { rc, dst: d, src: s });
        // now re-init in the middle of the compression
        recs[w].push(Rec::Init(unsafe {
            init_adv[w](z, std::ptr::null(), 0, base, src.len() as u64)
        }));
        recs[w].push(Rec::Init(unsafe {
            init_dict[w](z, dict.as_ptr(), dict.len(), 5)
        }));
        // ... and again after a second chunk
        let mut s2 = src.len();
        let mut d2 = outbuf.len();
        let rc2 = unsafe { cont[w](z, outbuf.as_mut_ptr(), &mut d2, src.as_ptr(), &mut s2) };
        recs[w].push(Rec::Cont { rc: rc2, dst: d2, src: s2 });
        recs[w].push(Rec::Init(unsafe {
            init_dict[w](z, std::ptr::null(), 0, 1)
        }));
        recs[w].push(Rec::Init(unsafe { freec[w](z) }));
    }
    assert_eq!(recs[0], recs[1], "mid-compression re-init sequence differs");
}

// ------------------------------------------------- ERRORS rows 276, 277 ----

#[test]
fn err_zbuff_forwarding() {
    let create = both!("ZBUFF_createCCtx", FnCreate);
    let freec = both!("ZBUFF_freeCCtx", FnFree);
    let cinit = both!("ZBUFF_compressInit", FnCInit);
    let cont = both!("ZBUFF_compressContinue", FnContinue);
    let flush = both!("ZBUFF_compressFlush", FnFlushEnd);
    let end = both!("ZBUFF_compressEnd", FnFlushEnd);
    let dcreate = both!("ZBUFF_createDCtx", FnCreate);
    let dfree = both!("ZBUFF_freeDCtx", FnFree);
    let dinit = both!("ZBUFF_decompressInit", FnDInit);
    let dcont = both!("ZBUFF_decompressContinue", FnContinue);
    let compress = both!("ZSTD_compress", FnCompress);
    let bound = both!("ZSTD_compressBound", FnBound);

    let mut rng = Rng::new(0x2767);

    // ---- row 276: compressContinue / Flush / End with a tiny dst ----
    for &dst_cap in &[0usize, 1, 2, 3] {
        let src = gen(Shape::Random, 200000, &mut rng);
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let z = unsafe { create[w]() };
            recs[w].push(Rec::Init(unsafe { cinit[w](z, 1) }));
            let mut outbuf = vec![0u8; 8];
            // repeatedly feed with an almost-zero dst: the internal buffer
            // eventually fills and everything must behave identically.
            for _ in 0..40 {
                let mut s = src.len();
                let mut d = dst_cap;
                let rc =
                    unsafe { cont[w](z, outbuf.as_mut_ptr(), &mut d, src.as_ptr(), &mut s) };
                recs[w].push(Rec::Cont { rc, dst: d, src: s });
                if is_error(rc) {
                    break;
                }
            }
            for _ in 0..4 {
                let mut d = dst_cap;
                let rc = unsafe { flush[w](z, outbuf.as_mut_ptr(), &mut d) };
                recs[w].push(Rec::Flush { rc, dst: d });
            }
            for _ in 0..4 {
                let mut d = dst_cap;
                let rc = unsafe { end[w](z, outbuf.as_mut_ptr(), &mut d) };
                recs[w].push(Rec::End { rc, dst: d });
            }
            recs[w].push(Rec::Init(unsafe { freec[w](z) }));
        }
        assert_eq!(recs[0], recs[1], "compress forwarding dst_cap={dst_cap} differs");
    }

    // compressContinue on a fresh, never-initialised CCtx
    {
        let src = gen(Shape::Text, 4096, &mut rng);
        let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
        for w in 0..2 {
            let z = unsafe { create[w]() };
            let mut outbuf = vec![0u8; 4096];
            let mut s = src.len();
            let mut d = outbuf.len();
            let rc = unsafe { cont[w](z, outbuf.as_mut_ptr(), &mut d, src.as_ptr(), &mut s) };
            recs[w].push(Rec::Cont { rc, dst: d, src: s });
            let mut d2 = outbuf.len();
            let rc2 = unsafe { flush[w](z, outbuf.as_mut_ptr(), &mut d2) };
            recs[w].push(Rec::Flush { rc: rc2, dst: d2 });
            let mut d3 = outbuf.len();
            let rc3 = unsafe { end[w](z, outbuf.as_mut_ptr(), &mut d3) };
            recs[w].push(Rec::End { rc: rc3, dst: d3 });
            recs[w].push(Rec::Init(unsafe { freec[w](z) }));
        }
        assert_eq!(recs[0], recs[1], "uninitialised CCtx forwarding differs");
    }

    // ---- row 277: decompressContinue error forwarding ----
    let src = gen(Shape::Text, 50000, &mut rng);
    let cap = unsafe { bound[0](src.len()) } + 64;
    let mut good = vec![0u8; cap];
    let n = unsafe { compress[0](good.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
    assert!(!is_error(n));
    good.truncate(n);
    {
        // sanity: the Rust lib produces the same frame
        let mut g2 = vec![0u8; cap];
        let n2 = unsafe { compress[1](g2.as_mut_ptr(), cap, src.as_ptr(), src.len(), 5) };
        assert_eq!(n, n2, "ZSTD_compress size differs");
        assert_bytes_eq("reference frame", &good, &g2[..n2]);
    }

    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    // truncations
    for &frac in &[1usize, 2, 4, 8] {
        let mut t = good.clone();
        t.truncate(good.len() / frac);
        inputs.push((format!("truncated/{frac}"), t));
    }
    // single-byte corruptions at deterministic positions
    for k in 0..12 {
        let mut c = good.clone();
        let i = (k * 7 + 3) % c.len();
        c[i] ^= 0x5A;
        inputs.push((format!("corrupt@{i}"), c));
    }
    // random garbage with an unknown magic
    for k in 0..6u32 {
        let mut b = Vec::new();
        b.extend_from_slice(&(0xDEADBEEFu32 ^ k).to_le_bytes());
        for _ in 0..rng.range(0, 256) {
            b.push(rng.byte());
        }
        inputs.push((format!("unknown magic {k}"), b));
    }
    // pure random buffers (no magic at all)
    for k in 0..6 {
        inputs.push((format!("random {k}"), gen(Shape::Random, rng.range(0, 300), &mut rng)));
    }
    // all-zero and tiny buffers
    inputs.push(("zeros1024".into(), vec![0u8; 1024]));
    for l in 0..6usize {
        inputs.push((format!("short{l}"), vec![0x28u8; l]));
    }

    for (label, buf) in &inputs {
        for &(i, o) in &[(usize::MAX, 1usize << 16), (1, 1), (3, 0), (7, 5)] {
            let ic = i.min(buf.len().max(1));
            let ctx = format!("zbuff dec forward {label} in={ic} out={o}");
            let mut recs: [Vec<Rec>; 2] = [Vec::new(), Vec::new()];
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            for w in 0..2 {
                let z = unsafe { dcreate[w]() };
                recs[w].push(Rec::Init(unsafe { dinit[w](z) }));
                if o == 0 {
                    // explicit zero-capacity dst, one shot
                    let mut s = buf.len();
                    let mut d = 0usize;
                    let rc = unsafe {
                        dcont[w](z, std::ptr::null_mut(), &mut d, buf.as_ptr(), &mut s)
                    };
                    recs[w].push(Rec::Cont { rc, dst: d, src: s });
                } else {
                    let d = drive_decompress(z, dcont[w], buf, ic, o, &ctx);
                    recs[w].extend(d.recs);
                    outs[w] = d.bytes;
                }
                recs[w].push(Rec::Init(unsafe { dfree[w](z) }));
            }
            assert_eq!(recs[0], recs[1], "{ctx}: decompress forwarding differs");
            assert_bytes_eq(&format!("{ctx}: partial output"), &outs[0], &outs[1]);
        }
    }
}
