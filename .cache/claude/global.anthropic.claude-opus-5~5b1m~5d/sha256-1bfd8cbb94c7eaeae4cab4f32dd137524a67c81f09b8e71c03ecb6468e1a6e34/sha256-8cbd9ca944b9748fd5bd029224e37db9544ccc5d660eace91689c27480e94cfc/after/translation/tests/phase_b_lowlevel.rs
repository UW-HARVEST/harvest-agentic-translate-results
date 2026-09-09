//! Phase B: LOW-LEVEL entry points — compressBegin/Continue/End, the block API,
//! decompressBegin/nextSrcSizeToDecompress/decompressContinue/nextInputType,
//! copyCCtx/copyDCtx, insertBlock, getFrameHeader, writeLastEmptyBlock.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnBegin = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnBeginDict = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnBeginAdv =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, Parameters, c_ulonglong) -> usize;
type FnBeginCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnBeginCDictAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, FParams, c_ulonglong) -> usize;
type FnChunk = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> usize;
type FnCopyDCtx = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnCtxSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnDBegin = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDBeginDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnDBeginDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnNextSize = unsafe extern "C" fn(*mut c_void) -> usize;
type FnNextType = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnSetFormat = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnGetFrameHeader = unsafe extern "C" fn(*mut FrameHeader, *const u8, usize) -> usize;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut FrameHeader, *const u8, usize, c_int) -> usize;
type FnWriteLastEmpty = unsafe extern "C" fn(*mut u8, usize) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> Parameters;
type FnCreateCDict = unsafe extern "C" fn(*const u8, usize, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;
type FnFreeDict = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnCheckContinuity = unsafe extern "C" fn(*mut c_void, *const u8, usize);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CParams {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FParams {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Parameters {
    pub cParams: CParams,
    pub fParams: FParams,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: c_int,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _r1: c_uint,
    pub _r2: c_uint,
}

const C_COMPRESSIONLEVEL: c_int = 100;
const C_MAXBLOCKSIZE: c_int = 1015;
const C_WINDOWLOG: c_int = 101;
const C_FORMAT: c_int = 10;
const C_CHECKSUMFLAG: c_int = 201;
const C_CONTENTSIZEFLAG: c_int = 200;
const D_WINDOWLOGMAX: c_int = 100;

struct Both {
    c: *mut c_void,
    r: *mut c_void,
}

fn new_cctx_pair() -> (Both, libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    (Both { c: unsafe { cn() }, r: unsafe { rn() } }, cf, rf)
}

fn new_dctx_pair() -> (Both, libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    (Both { c: unsafe { cn() }, r: unsafe { rn() } }, cf, rf)
}

/// Run the compressBegin/compressContinue/compressEnd pipeline in both libs.
/// `begin` performs the chosen begin variant on the given context.
fn begin_continue_end(
    begin: &dyn Fn(*mut c_void, bool) -> usize,
    src: &[u8],
    chunk: usize,
    ctx: &str,
) -> Option<Vec<u8>> {
    let (cont_c, cont_r) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (end_c, end_r) = unsafe { pair::<FnChunk>("ZSTD_compressEnd") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (both, cf, rf) = new_cctx_pair();

    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut ok = true;
    for which in 0..2 {
        let z = if which == 0 { both.c } else { both.r };
        let rc = begin(z, which == 1);
        rets[which].push(rc);
        if is_error(rc) {
            ok = false;
            continue;
        }
        let mut collected = Vec::new();
        let mut pos = 0usize;
        let mut buf = vec![0u8; unsafe { cb(chunk.max(1)) } + 1024];
        while pos < src.len() {
            let n = chunk.min(src.len() - pos);
            let last = pos + n == src.len();
            let f = if last {
                if which == 0 { &end_c } else { &end_r }
            } else if which == 0 {
                &cont_c
            } else {
                &cont_r
            };
            let w = unsafe { f(z, buf.as_mut_ptr(), buf.len(), src[pos..].as_ptr(), n) };
            rets[which].push(w);
            if is_error(w) {
                ok = false;
                break;
            }
            collected.extend_from_slice(&buf[..w]);
            pos += n;
        }
        if src.is_empty() {
            let f = if which == 0 { &end_c } else { &end_r };
            let w = unsafe { f(z, buf.as_mut_ptr(), buf.len(), src.as_ptr(), 0) };
            rets[which].push(w);
            collected.extend_from_slice(&buf[..w]);
        }
        outs[which] = collected;
    }
    unsafe {
        cf(both.c);
        rf(both.r);
    }
    assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
    if !ok {
        return None;
    }
    assert_bytes_eq(&format!("{ctx}: low-level frame"), &outs[0], &outs[1]);
    Some(outs[0].clone())
}

fn decompress_check(frame: &[u8], expect: &[u8], dparams: &[(c_int, c_int)], ctx: &str) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnChunk>("ZSTD_decompressDCtx") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, v) in dparams {
        assert_eq!(unsafe { cs(cc, p, v) }, unsafe { rs(rc, p, v) }, "{ctx}: dparam");
    }
    let cap = expect.len() + 64;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let a = unsafe { cd(cc, co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    let b = unsafe { rd(rc, ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    assert_eq!(a, b, "{ctx}: decompress ret (C err {})", err_code(a));
    if !is_error(a) {
        assert_bytes_eq(&format!("{ctx}: dec"), &co[..a], expect);
        assert_bytes_eq(&format!("{ctx}: dec-rust"), &ro[..b], expect);
    }
    unsafe {
        cf(cc);
        rf(rc);
    }
}

#[test]
fn cfg_compressBegin_continue_end() {
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (cbs, rbs) = unsafe { pair::<FnCtxSize>("ZSTD_getBlockSize") };
    let mut rng = Rng::new(0x1A1A);
    for &lvl in &[1, 3, 5, 9, 19] {
        for &chunk in &[1usize, 100, 65536, 131072, 200000] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Rle] {
                let size = rng.range(0, 400000);
                let src = gen(shape, size, &mut rng);
                let ctx = format!("begin/cont lvl={lvl} chunk={chunk} shape={shape:?} n={size}");
                let begin = |z: *mut c_void, is_rust: bool| -> usize {
                    let f = if is_rust { &rbg } else { &cbg };
                    let rc = unsafe { f(z, lvl) };
                    // query block size as an extra differential observation
                    let g = if is_rust { &rbs } else { &cbs };
                    let _ = unsafe { g(z) };
                    rc
                };
                if let Some(frame) = begin_continue_end(&begin, &src, chunk, &ctx) {
                    decompress_check(&frame, &src, &[], &ctx);
                }
            }
        }
    }
}

#[test]
fn cfg_compressBegin_advanced() {
    let (ca, ra) = unsafe { pair::<FnBeginAdv>("ZSTD_compressBegin_advanced") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let mut rng = Rng::new(0x2B2B);
    for &lvl in &[1, 6, 13, 19] {
        for &(cksum, csize) in &[(0, 0), (1, 1), (1, 0), (0, 1)] {
            for &pledge in &[true, false] {
                let size = rng.range(0, 300000);
                let src = gen(Shape::Text, size, &mut rng);
                let ctx =
                    format!("beginAdv lvl={lvl} cksum={cksum} csize={csize} pledge={pledge} n={size}");
                let begin = |z: *mut c_void, is_rust: bool| -> usize {
                    let mut p = if is_rust {
                        unsafe { rgp(lvl, size as u64, 0) }
                    } else {
                        unsafe { cgp(lvl, size as u64, 0) }
                    };
                    p.fParams.checksumFlag = cksum;
                    p.fParams.contentSizeFlag = csize;
                    let pl = if pledge { size as u64 } else { 0 };
                    let f = if is_rust { &ra } else { &ca };
                    unsafe { f(z, std::ptr::null(), 0, p, pl) }
                };
                if let Some(frame) = begin_continue_end(&begin, &src, 70000, &ctx) {
                    decompress_check(&frame, &src, &[(D_WINDOWLOGMAX, 31)], &ctx);
                }
            }
        }
    }
}

#[test]
fn cfg_compressBegin_dict() {
    let (cd, rd) = unsafe { pair::<FnBeginDict>("ZSTD_compressBegin_usingDict") };
    let (cc, rc) = unsafe { pair::<FnBeginCDict>("ZSTD_compressBegin_usingCDict") };
    let (cca, rca) =
        unsafe { pair::<FnBeginCDictAdv>("ZSTD_compressBegin_usingCDict_advanced") };
    let (ccd, rcd) = unsafe { pair::<FnCreateCDict>("ZSTD_createCDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeCDict") };
    let (cdb, rdb) = unsafe { pair::<FnDBeginDict>("ZSTD_decompressBegin_usingDict") };
    let mut rng = Rng::new(0x3C3C);
    let dict = gen(Shape::Text, 8192, &mut rng);

    for &lvl in &[1, 9, 19] {
        let cdict = unsafe { ccd(dict.as_ptr(), dict.len(), lvl) };
        let rdict = unsafe { rcd(dict.as_ptr(), dict.len(), lvl) };
        for mode in 0..3 {
            let size = rng.range(0, 120000);
            let src = gen(Shape::Text, size, &mut rng);
            let ctx = format!("beginDict lvl={lvl} mode={mode} n={size}");
            let begin = |z: *mut c_void, is_rust: bool| -> usize {
                unsafe {
                    match (mode, is_rust) {
                        (0, false) => cd(z, dict.as_ptr(), dict.len(), lvl),
                        (0, true) => rd(z, dict.as_ptr(), dict.len(), lvl),
                        (1, false) => cc(z, cdict),
                        (1, true) => rc(z, rdict),
                        (_, false) => cca(
                            z,
                            cdict,
                            FParams { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 0 },
                            size as u64,
                        ),
                        (_, true) => rca(
                            z,
                            rdict,
                            FParams { contentSizeFlag: 1, checksumFlag: 1, noDictIDFlag: 0 },
                            size as u64,
                        ),
                    }
                }
            };
            if let Some(frame) = begin_continue_end(&begin, &src, 60000, &ctx) {
                // decode with decompressBegin_usingDict + decompressContinue loop
                low_level_decompress(&frame, Some(&src), &|z, is_rust| {
                    let f = if is_rust { &rdb } else { &cdb };
                    unsafe { f(z, dict.as_ptr(), dict.len()) }
                }, &[], &ctx);
            }
        }
        unsafe {
            cfd(cdict);
            rfd(rdict);
        }
    }
}

#[test]
fn cfg_copyCCtx() {
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (ccp, rcp) = unsafe { pair::<FnCopyCCtx>("ZSTD_copyCCtx") };
    let (cont_c, cont_r) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (end_c, end_r) = unsafe { pair::<FnChunk>("ZSTD_compressEnd") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (src_pair, cf1, rf1) = new_cctx_pair();
    let (dst_pair, cf2, rf2) = new_cctx_pair();
    let mut rng = Rng::new(0x4D4D);

    for &lvl in &[1, 9, 19] {
        let src = gen(Shape::Text, rng.range(1, 120000), &mut rng);
        let ctx = format!("copyCCtx lvl={lvl} n={}", src.len());
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let s = if which == 0 { src_pair.c } else { src_pair.r };
            let d = if which == 0 { dst_pair.c } else { dst_pair.r };
            let bg = if which == 0 { &cbg } else { &rbg };
            rets[which].push(unsafe { bg(s, lvl) });
            let cp = if which == 0 { &ccp } else { &rcp };
            rets[which].push(unsafe { cp(d, s, src.len() as u64) });
            let mut buf = vec![0u8; unsafe { cb(src.len()) } + 1024];
            let cont = if which == 0 { &cont_c } else { &cont_r };
            let end = if which == 0 { &end_c } else { &end_r };
            let half = src.len() / 2;
            let mut collected = Vec::new();
            let w = unsafe { cont(d, buf.as_mut_ptr(), buf.len(), src.as_ptr(), half) };
            rets[which].push(w);
            collected.extend_from_slice(&buf[..w]);
            let w = unsafe {
                end(d, buf.as_mut_ptr(), buf.len(), src[half..].as_ptr(), src.len() - half)
            };
            rets[which].push(w);
            collected.extend_from_slice(&buf[..w]);
            outs[which] = collected;
        }
        assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
        assert_bytes_eq(&ctx, &outs[0], &outs[1]);
        decompress_check(&outs[0], &src, &[], &ctx);
    }
    unsafe {
        cf1(src_pair.c);
        rf1(src_pair.r);
        cf2(dst_pair.c);
        rf2(dst_pair.r);
    }
}

/// Rows 104-108, 115: the raw block API + insertBlock.
#[test]
fn cfg_block_api() {
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (cbs, rbs) = unsafe { pair::<FnCtxSize>("ZSTD_getBlockSize") };
    let (ccb, rcb) = unsafe { pair::<FnChunk>("ZSTD_compressBlock") };
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (cdcb, rdcb) = unsafe { pair::<FnChunk>("ZSTD_decompressBlock") };
    let (cib, rib) = unsafe { pair::<FnInsertBlock>("ZSTD_insertBlock") };
    let (ccc, rcc) = unsafe { pair::<FnCheckContinuity>("ZSTD_checkContinuity") };
    let (csp, rsp) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (cpair, cf, rf) = new_cctx_pair();
    let (dpair, dcf, drf) = new_dctx_pair();
    let mut rng = Rng::new(0x5E5E);

    for &maxbs in &[0, 1024, 131072] {
        for &lvl in &[1, 5, 19] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Rle, Shape::Repetitive] {
                let ctx = format!("block api maxbs={maxbs} lvl={lvl} shape={shape:?}");
                // set up both compression contexts
                let mut bs = [0usize; 2];
                for which in 0..2 {
                    let z = if which == 0 { cpair.c } else { cpair.r };
                    let sp = if which == 0 { &csp } else { &rsp };
                    if maxbs != 0 {
                        assert!(!is_error(unsafe { sp(z, C_MAXBLOCKSIZE, maxbs) }));
                    } else {
                        assert!(!is_error(unsafe { sp(z, C_MAXBLOCKSIZE, 131072) }));
                    }
                    let bg = if which == 0 { &cbg } else { &rbg };
                    assert!(!is_error(unsafe { bg(z, lvl) }));
                    let gb = if which == 0 { &cbs } else { &rbs };
                    bs[which] = unsafe { gb(z) };
                    let db = if which == 0 { &cdb } else { &rdb };
                    assert!(!is_error(unsafe { db(if which == 0 { dpair.c } else { dpair.r }) }));
                }
                assert_eq!(bs[0], bs[1], "{ctx}: getBlockSize");
                let block_size = bs[0];
                // The block API requires CONTIGUOUS src (compression history) and
                // CONTIGUOUS dst (decompression history), so use one buffer for each
                // and slice consecutive blocks out of them.
                let sizes: Vec<usize> =
                    vec![block_size, block_size, 1, 7, 1000, block_size.min(1024)]
                        .into_iter()
                        .filter(|&n| n > 0)
                        .collect();
                let total: usize = sizes.iter().sum();
                let whole = gen(shape, total, &mut rng);
                let dcap = total + (1 << 18);
                let mut cdec = vec![0u8; dcap];
                let mut rdec = vec![0u8; dcap];
                let mut written = 0usize;
                let mut spos = 0usize;
                for &n in &sizes {
                    let raw = &whole[spos..spos + n];
                    let cap = unsafe { cb(n) } + 64;
                    let mut cout = vec![0u8; cap];
                    let mut rout = vec![0u8; cap];
                    let a = unsafe { ccb(cpair.c, cout.as_mut_ptr(), cap, raw.as_ptr(), n) };
                    let b = unsafe { rcb(cpair.r, rout.as_mut_ptr(), cap, raw.as_ptr(), n) };
                    assert_eq!(a, b, "{ctx}: compressBlock({n})");
                    assert!(!is_error(a), "{ctx}: compressBlock err {}", err_code(a));
                    assert_bytes_eq(&format!("{ctx}: block bytes n={n}"), &cout[..a], &rout[..b]);
                    if a == 0 {
                        // block not compressible: emit raw + register it in the
                        // decoder's history with ZSTD_insertBlock
                        cdec[written..written + n].copy_from_slice(raw);
                        rdec[written..written + n].copy_from_slice(raw);
                        let ia = unsafe { cib(dpair.c, cdec[written..].as_ptr(), n) };
                        let ib = unsafe { rib(dpair.r, rdec[written..].as_ptr(), n) };
                        assert_eq!(ia, ib, "{ctx}: insertBlock");
                        written += n;
                    } else {
                        let da = unsafe {
                            cdcb(dpair.c, cdec[written..].as_mut_ptr(), dcap - written,
                                 cout.as_ptr(), a)
                        };
                        let db2 = unsafe {
                            rdcb(dpair.r, rdec[written..].as_mut_ptr(), dcap - written,
                                 rout.as_ptr(), b)
                        };
                        assert_eq!(da, db2, "{ctx}: decompressBlock n={n}");
                        if !is_error(da) {
                            assert_bytes_eq(
                                &format!("{ctx}: decompressBlock bytes n={n}"),
                                &cdec[written..written + da],
                                &rdec[written..written + db2],
                            );
                            assert_bytes_eq(
                                &format!("{ctx}: decompressBlock vs orig n={n}"),
                                &cdec[written..written + da],
                                raw,
                            );
                            written += da;
                        }
                    }
                    spos += n;
                    let history = &cdec[..written];
                    // ZSTD_checkContinuity returns void; call it in both libs so the
                    // subsequent block decodes exercise the same (extDict) state.
                    unsafe { ccc(dpair.c, history.as_ptr(), history.len()) };
                    unsafe { rcc(dpair.r, rdec[..written].as_ptr(), history.len()) };
                }
            }
        }
    }
    unsafe {
        cf(cpair.c);
        rf(cpair.r);
        dcf(dpair.c);
        drf(dpair.r);
    }
}

/// Drive the decompressBegin/nextSrcSizeToDecompress/decompressContinue/nextInputType
/// loop in both libraries and compare every intermediate value.
fn low_level_decompress(
    frame: &[u8],
    expect: Option<&[u8]>,
    begin: &dyn Fn(*mut c_void, bool) -> usize,
    dparams: &[(c_int, c_int)],
    ctx: &str,
) {
    let (cns, rns) = unsafe { pair::<FnNextSize>("ZSTD_nextSrcSizeToDecompress") };
    let (cnt, rnt) = unsafe { pair::<FnNextType>("ZSTD_nextInputType") };
    let (cdc, rdc) = unsafe { pair::<FnChunk>("ZSTD_decompressContinue") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (pairs, cf, rf) = new_dctx_pair();

    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut trace: [Vec<(usize, c_int, usize)>; 2] = [Vec::new(), Vec::new()];
    for which in 0..2 {
        let z = if which == 0 { pairs.c } else { pairs.r };
        for &(p, v) in dparams {
            let sp = if which == 0 { &cs } else { &rs };
            assert!(!is_error(unsafe { sp(z, p, v) }));
        }
        let rc = begin(z, which == 1);
        assert!(!is_error(rc), "{ctx}: begin err {}", err_code(rc));
        let mut ip = 0usize;
        // ZSTD_decompressContinue requires a CONTIGUOUS destination that keeps the
        // decoded history, so decode into one big buffer and advance inside it.
        let cap = expect.map(|e| e.len()).unwrap_or(4 << 20) + (1 << 18);
        let mut outbuf = vec![0u8; cap];
        let mut written = 0usize;
        loop {
            let ns = if which == 0 { unsafe { cns(z) } } else { unsafe { rns(z) } };
            let nt = if which == 0 { unsafe { cnt(z) } } else { unsafe { rnt(z) } };
            if ns == 0 {
                trace[which].push((ns, nt, 0));
                break;
            }
            let avail = (frame.len() - ip).min(ns);
            if avail == 0 {
                trace[which].push((ns, nt, usize::MAX));
                break;
            }
            let f = if which == 0 { &cdc } else { &rdc };
            let w = unsafe {
                f(z, outbuf[written..].as_mut_ptr(), cap - written, frame[ip..].as_ptr(), avail)
            };
            trace[which].push((ns, nt, w));
            if is_error(w) {
                break;
            }
            written += w;
            ip += avail;
        }
        outbuf.truncate(written);
        outs[which] = outbuf;
    }
    unsafe {
        cf(pairs.c);
        rf(pairs.r);
    }
    assert_eq!(trace[0], trace[1], "{ctx}: (nextSrcSize, nextInputType, written) trace differs");
    assert_bytes_eq(&format!("{ctx}: low-level decompressed"), &outs[0], &outs[1]);
    if let Some(e) = expect {
        assert_bytes_eq(&format!("{ctx}: low-level vs orig"), &outs[0], e);
    }
}

fn compress2_both(params: &[(c_int, c_int)], src: &[u8], ctx: &str) -> Vec<u8> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cc = unsafe { cn() };
    let rc = unsafe { rn() };
    for &(p, v) in params {
        assert_eq!(unsafe { cs(cc, p, v) }, unsafe { rs(rc, p, v) }, "{ctx}: setparam");
    }
    let cap = unsafe { cb(src.len()) } + 64;
    let mut cbuf = vec![0u8; cap];
    let mut rbuf = vec![0u8; cap];
    let a = unsafe { c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    let b = unsafe { r2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
    assert_eq!(a, b, "{ctx}: compress2");
    assert!(!is_error(a), "{ctx}: compress2 err {}", err_code(a));
    assert_bytes_eq(&format!("{ctx}: compress2 bytes"), &cbuf[..a], &rbuf[..b]);
    unsafe {
        cf(cc);
        rf(rc);
    }
    cbuf.truncate(a);
    cbuf
}

#[test]
fn cfg_decompressContinue_loop() {
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let mut rng = Rng::new(0x6F6F);
    for &(cksum, csize) in &[(0, 0), (1, 1), (1, 0), (0, 1)] {
        for &shape in ALL_SHAPES {
            for &size in &[0usize, 1, 1000, 131072, 262144] {
                let src = gen(shape, size, &mut rng);
                let ctx = format!("llDec cksum={cksum} csize={csize} shape={shape:?} n={size}");
                let frame = compress2_both(
                    &[(C_CHECKSUMFLAG, cksum), (C_CONTENTSIZEFLAG, csize),
                      (C_COMPRESSIONLEVEL, 5)],
                    &src,
                    &ctx,
                );
                low_level_decompress(
                    &frame,
                    Some(&src),
                    &|z, is_rust| {
                        let f = if is_rust { &rdb } else { &cdb };
                        unsafe { f(z) }
                    },
                    &[],
                    &ctx,
                );
            }
        }
    }
}

#[test]
fn cfg_decompressContinue_magicless() {
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (csf, rsf) = unsafe { pair::<FnSetFormat>("ZSTD_DCtx_setFormat") };
    let mut rng = Rng::new(0x7070);
    for &size in &[0usize, 100, 65536, 200000] {
        let src = gen(Shape::Text, size, &mut rng);
        let ctx = format!("llDec magicless n={size}");
        let frame = compress2_both(&[(C_FORMAT, 1), (C_COMPRESSIONLEVEL, 5)], &src, &ctx);
        low_level_decompress(
            &frame,
            Some(&src),
            &|z, is_rust| {
                let sf = if is_rust { &rsf } else { &csf };
                let rc = unsafe { sf(z, 1) };
                if is_error(rc) {
                    return rc;
                }
                let f = if is_rust { &rdb } else { &cdb };
                unsafe { f(z) }
            },
            &[],
            &ctx,
        );
    }
}

#[test]
fn cfg_decompressContinue_raw_rle() {
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let mut rng = Rng::new(0x8181);
    // random => raw blocks, all-one-byte => RLE blocks
    for &shape in &[Shape::Random, Shape::Rle] {
        for &size in &[131072usize, 262144, 131073] {
            let src = gen(shape, size, &mut rng);
            let ctx = format!("llDec raw/rle shape={shape:?} n={size}");
            let frame = compress2_both(&[(C_COMPRESSIONLEVEL, 1)], &src, &ctx);
            low_level_decompress(
                &frame,
                Some(&src),
                &|z, is_rust| {
                    let f = if is_rust { &rdb } else { &cdb };
                    unsafe { f(z) }
                },
                &[],
                &ctx,
            );
        }
    }
}

#[test]
fn cfg_decompressBegin_dict() {
    let (cdb, rdb) = unsafe { pair::<FnDBeginDict>("ZSTD_decompressBegin_usingDict") };
    let (cdd, rdd) = unsafe { pair::<FnDBeginDDict>("ZSTD_decompressBegin_usingDDict") };
    let (ccd, rcd) = unsafe { pair::<FnCreateDDict>("ZSTD_createDDict") };
    let (cfd, rfd) = unsafe { pair::<FnFreeDict>("ZSTD_freeDDict") };
    let (cld, rld) = unsafe { pair::<FnDBeginDict>("ZSTD_CCtx_loadDictionary") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0x9292);
    let dict = gen(Shape::Text, 8192, &mut rng);
    let cddict = unsafe { ccd(dict.as_ptr(), dict.len()) };
    let rddict = unsafe { rcd(dict.as_ptr(), dict.len()) };

    for &size in &[0usize, 1000, 65536] {
        let src = gen(Shape::Text, size, &mut rng);
        let ctx = format!("llDec dict n={size}");
        // dict-compressed frame
        let cc = unsafe { cn() };
        let rc = unsafe { rn() };
        assert_eq!(unsafe { cld(cc, dict.as_ptr(), dict.len()) },
                   unsafe { rld(rc, dict.as_ptr(), dict.len()) });
        let cap = unsafe { cb(src.len()) } + 64;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let a = unsafe { c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        let b = unsafe { r2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
        assert_eq!(a, b);
        assert_bytes_eq(&ctx, &cbuf[..a], &rbuf[..b]);
        unsafe {
            cf(cc);
            rf(rc);
        }
        let frame = cbuf[..a].to_vec();
        low_level_decompress(&frame, Some(&src), &|z, is_rust| {
            let f = if is_rust { &rdb } else { &cdb };
            unsafe { f(z, dict.as_ptr(), dict.len()) }
        }, &[], &format!("{ctx} usingDict"));
        low_level_decompress(&frame, Some(&src), &|z, is_rust| {
            if is_rust {
                unsafe { rdd(z, rddict) }
            } else {
                unsafe { cdd(z, cddict) }
            }
        }, &[], &format!("{ctx} usingDDict"));
    }
    unsafe {
        cfd(cddict);
        rfd(rddict);
    }
}

#[test]
fn cfg_copyDCtx() {
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (ccp, rcp) = unsafe { pair::<FnCopyDCtx>("ZSTD_copyDCtx") };
    let (cns, rns) = unsafe { pair::<FnNextSize>("ZSTD_nextSrcSizeToDecompress") };
    let (cdc, rdc) = unsafe { pair::<FnChunk>("ZSTD_decompressContinue") };
    let (prep, pcf, prf) = new_dctx_pair();
    let (work, wcf, wrf) = new_dctx_pair();
    let mut rng = Rng::new(0xA3A3);

    for &size in &[1000usize, 131072, 200000] {
        let src = gen(Shape::Text, size, &mut rng);
        let ctx = format!("copyDCtx n={size}");
        let frame = compress2_both(&[(C_COMPRESSIONLEVEL, 5), (C_CHECKSUMFLAG, 1)], &src, &ctx);
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let p = if which == 0 { prep.c } else { prep.r };
            let w = if which == 0 { work.c } else { work.r };
            let bg = if which == 0 { &cdb } else { &rdb };
            rets[which].push(unsafe { bg(p) });
            // consume the frame header on the prepared context
            let ns = if which == 0 { unsafe { cns(p) } } else { unsafe { rns(p) } };
            rets[which].push(ns);
            let mut obuf = vec![0u8; 1 << 18];
            let dc = if which == 0 { &cdc } else { &rdc };
            let rc =
                unsafe { dc(p, obuf.as_mut_ptr(), obuf.len(), frame.as_ptr(), ns) };
            rets[which].push(rc);
            // copy the prepared context and continue there
            if which == 0 {
                unsafe { ccp(w, p) };
            } else {
                unsafe { rcp(w, p) };
            }
            let mut ip = ns;
            let mut collected = Vec::new();
            loop {
                let ns = if which == 0 { unsafe { cns(w) } } else { unsafe { rns(w) } };
                if ns == 0 || ip >= frame.len() {
                    rets[which].push(ns);
                    break;
                }
                let avail = (frame.len() - ip).min(ns);
                let rc = unsafe {
                    dc(w, obuf.as_mut_ptr(), obuf.len(), frame[ip..].as_ptr(), avail)
                };
                rets[which].push(rc);
                if is_error(rc) {
                    break;
                }
                collected.extend_from_slice(&obuf[..rc]);
                ip += avail;
            }
            outs[which] = collected;
        }
        assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
        assert_bytes_eq(&ctx, &outs[0], &outs[1]);
    }
    unsafe {
        pcf(prep.c);
        prf(prep.r);
        wcf(work.c);
        wrf(work.r);
    }
}

/// Row 116: getFrameHeader / _advanced / frameHeaderSize over all header shapes.
#[test]
fn cfg_getFrameHeader_variants() {
    let (cgh, rgh) = unsafe { pair::<FnGetFrameHeader>("ZSTD_getFrameHeader") };
    let (cga, rga) = unsafe { pair::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced") };
    let (cfh, rfh) = unsafe { pair::<FnBound>("ZSTD_frameHeaderSize") };
    let (cld, rld) = unsafe { pair::<FnDBeginDict>("ZSTD_CCtx_loadDictionary") };
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (c2, r2) = unsafe { pair::<FnCompress2>("ZSTD_compress2") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let mut rng = Rng::new(0xB4B4);
    let dict = gen(Shape::Text, 4096, &mut rng);

    let mut frames: Vec<(String, Vec<u8>, c_int)> = Vec::new();
    for &use_dict in &[false, true] {
        for &(csize, cksum, dictid, fmt) in &[
            (1, 0, 1, 0),
            (0, 1, 0, 0),
            (1, 1, 1, 1),
            (0, 0, 0, 1),
        ] {
            for &size in &[0usize, 1, 200, 70000, 300000] {
                let src = gen(Shape::Text, size, &mut rng);
                let cc = unsafe { cn() };
                let rc = unsafe { rn() };
                for (p, v) in [
                    (C_CONTENTSIZEFLAG, csize),
                    (C_CHECKSUMFLAG, cksum),
                    (202, dictid),
                    (C_FORMAT, fmt),
                    (C_COMPRESSIONLEVEL, 3),
                ] {
                    assert_eq!(unsafe { cs(cc, p, v) }, unsafe { rs(rc, p, v) });
                }
                if use_dict {
                    assert_eq!(unsafe { cld(cc, dict.as_ptr(), dict.len()) },
                               unsafe { rld(rc, dict.as_ptr(), dict.len()) });
                }
                let cap = unsafe { cb(src.len()) } + 64;
                let mut cbuf = vec![0u8; cap];
                let mut rbuf = vec![0u8; cap];
                let a = unsafe { c2(cc, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                let b = unsafe { r2(rc, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len()) };
                assert_eq!(a, b);
                assert_bytes_eq("frame", &cbuf[..a], &rbuf[..b]);
                cbuf.truncate(a);
                frames.push((
                    format!("dict={use_dict} csize={csize} cksum={cksum} did={dictid} fmt={fmt} n={size}"),
                    cbuf,
                    fmt,
                ));
                unsafe {
                    cf(cc);
                    rf(rc);
                }
            }
        }
    }
    for (ctx, frame, fmt) in &frames {
        for n in 0..=frame.len().min(24) {
            let mut ca = FrameHeader::default();
            let mut ra = FrameHeader::default();
            let x = unsafe { cgh(&mut ca, frame.as_ptr(), n) };
            let y = unsafe { rgh(&mut ra, frame.as_ptr(), n) };
            assert_eq!(x, y, "{ctx}: getFrameHeader(n={n})");
            if x == 0 {
                assert_eq!(ca, ra, "{ctx}: header struct (n={n})");
            }
            let mut ca2 = FrameHeader::default();
            let mut ra2 = FrameHeader::default();
            let x2 = unsafe { cga(&mut ca2, frame.as_ptr(), n, *fmt) };
            let y2 = unsafe { rga(&mut ra2, frame.as_ptr(), n, *fmt) };
            assert_eq!(x2, y2, "{ctx}: getFrameHeader_advanced(n={n},fmt={fmt})");
            if x2 == 0 {
                assert_eq!(ca2, ra2, "{ctx}: adv header struct (n={n})");
            }
            // both formats, including the mismatching one
            let other = 1 - *fmt;
            let mut ca3 = FrameHeader::default();
            let mut ra3 = FrameHeader::default();
            assert_eq!(
                unsafe { cga(&mut ca3, frame.as_ptr(), n, other) },
                unsafe { rga(&mut ra3, frame.as_ptr(), n, other) },
                "{ctx}: getFrameHeader_advanced(n={n},fmt={other})"
            );
            assert_eq!(
                unsafe { cfh(frame.as_ptr() as usize) as usize },
                unsafe { rfh(frame.as_ptr() as usize) as usize },
                "sanity"
            );
        }
    }
    // frameHeaderSize with real arguments
    let (cfhs, rfhs) = unsafe { pair::<unsafe extern "C" fn(*const u8, usize) -> usize>("ZSTD_frameHeaderSize") };
    for (ctx, frame, _) in &frames {
        for n in 0..=frame.len().min(20) {
            assert_eq!(
                unsafe { cfhs(frame.as_ptr(), n) },
                unsafe { rfhs(frame.as_ptr(), n) },
                "{ctx}: frameHeaderSize(n={n})"
            );
        }
    }
}

/// Row 124: ZSTD_writeLastEmptyBlock composed into a hand-assembled frame.
#[test]
fn cfg_write_last_empty_block() {
    let (cw, rw) = unsafe { pair::<FnWriteLastEmpty>("ZSTD_writeLastEmptyBlock") };
    let (cbg, rbg) = unsafe { pair::<FnBegin>("ZSTD_compressBegin") };
    let (cont_c, cont_r) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (cpair, cf, rf) = new_cctx_pair();
    let mut rng = Rng::new(0xC5C5);

    for &size in &[100usize, 5000, 131072] {
        let src = gen(Shape::Text, size, &mut rng);
        let ctx = format!("writeLastEmptyBlock n={size}");
        let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { cpair.c } else { cpair.r };
            let bg = if which == 0 { &cbg } else { &rbg };
            assert!(!is_error(unsafe { bg(z, 3) }));
            let mut buf = vec![0u8; unsafe { cb(src.len()) } + 1024];
            let cont = if which == 0 { &cont_c } else { &cont_r };
            let w = unsafe { cont(z, buf.as_mut_ptr(), buf.len(), src.as_ptr(), src.len()) };
            assert!(!is_error(w));
            let mut collected = buf[..w].to_vec();
            let mut tail = [0u8; 8];
            let t = if which == 0 {
                unsafe { cw(tail.as_mut_ptr(), tail.len()) }
            } else {
                unsafe { rw(tail.as_mut_ptr(), tail.len()) }
            };
            assert!(!is_error(t), "{ctx}: writeLastEmptyBlock");
            collected.extend_from_slice(&tail[..t]);
            outs[which] = collected;
        }
        assert_bytes_eq(&ctx, &outs[0], &outs[1]);
        decompress_check(&outs[0], &src, &[], &ctx);
    }
    unsafe {
        cf(cpair.c);
        rf(cpair.r);
    }
}

/// Row 222: non-contiguous buffers + small window => extDict paths in the low-level API.
#[test]
fn cfg_extdict_noncontiguous() {
    let (cbg, rbg) = unsafe { pair::<FnBeginAdv>("ZSTD_compressBegin_advanced") };
    let (cgp, rgp) = unsafe { pair::<FnGetParams>("ZSTD_getParams") };
    let (cont_c, cont_r) = unsafe { pair::<FnChunk>("ZSTD_compressContinue") };
    let (end_c, end_r) = unsafe { pair::<FnChunk>("ZSTD_compressEnd") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (cdb, rdb) = unsafe { pair::<FnDBegin>("ZSTD_decompressBegin") };
    let (cpair, cf, rf) = new_cctx_pair();
    let mut rng = Rng::new(0xD6D6);

    for &wlog in &[10, 11, 15] {
        for &nchunks in &[2usize, 3, 5] {
            // each chunk in a SEPARATE allocation => non-contiguous history
            let mut chunks: Vec<Vec<u8>> = Vec::new();
            let mut all = Vec::new();
            for _ in 0..nchunks {
                let c = gen(Shape::Text, rng.range(1, 4096), &mut rng);
                all.extend_from_slice(&c);
                chunks.push(c);
            }
            let ctx = format!("extdict wlog={wlog} nchunks={nchunks} total={}", all.len());
            let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
            let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            for which in 0..2 {
                let z = if which == 0 { cpair.c } else { cpair.r };
                let mut p = if which == 0 {
                    unsafe { cgp(5, all.len() as u64, 0) }
                } else {
                    unsafe { rgp(5, all.len() as u64, 0) }
                };
                p.cParams.windowLog = wlog;
                if p.cParams.chainLog > wlog {
                    p.cParams.chainLog = wlog;
                }
                if p.cParams.hashLog > wlog {
                    p.cParams.hashLog = wlog;
                }
                let bg = if which == 0 { &cbg } else { &rbg };
                let rc = unsafe { bg(z, std::ptr::null(), 0, p, all.len() as u64) };
                rets[which].push(rc);
                assert!(!is_error(rc), "{ctx}: begin_advanced err {}", err_code(rc));
                let mut collected = Vec::new();
                let mut buf = vec![0u8; unsafe { cb(8192) } + 1024];
                for (i, c) in chunks.iter().enumerate() {
                    let last = i + 1 == chunks.len();
                    let f = if last {
                        if which == 0 { &end_c } else { &end_r }
                    } else if which == 0 {
                        &cont_c
                    } else {
                        &cont_r
                    };
                    let w = unsafe { f(z, buf.as_mut_ptr(), buf.len(), c.as_ptr(), c.len()) };
                    rets[which].push(w);
                    assert!(!is_error(w), "{ctx}: chunk {i} err {}", err_code(w));
                    collected.extend_from_slice(&buf[..w]);
                }
                outs[which] = collected;
            }
            assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
            assert_bytes_eq(&ctx, &outs[0], &outs[1]);
            low_level_decompress(&outs[0], Some(&all), &|z, is_rust| {
                let f = if is_rust { &rdb } else { &cdb };
                unsafe { f(z) }
            }, &[], &ctx);
        }
    }
    unsafe {
        cf(cpair.c);
        rf(cpair.r);
    }
}
