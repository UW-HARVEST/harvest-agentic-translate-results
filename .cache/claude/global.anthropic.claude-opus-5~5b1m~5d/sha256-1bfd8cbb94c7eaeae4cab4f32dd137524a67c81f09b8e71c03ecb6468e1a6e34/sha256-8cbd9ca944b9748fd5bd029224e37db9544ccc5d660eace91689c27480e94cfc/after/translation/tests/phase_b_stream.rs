//! Phase B: streaming API differential tests (new ZSTD_compressStream2 and the
//! old ZSTD_compressStream / flushStream / endStream), plus ZSTD_decompressStream.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct InBuffer {
    src: *const u8,
    size: usize,
    pos: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct OutBuffer {
    dst: *mut u8,
    size: usize,
    pos: usize,
}

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnSize = unsafe extern "C" fn() -> usize;
type FnStream2 =
    unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
type FnStream = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut OutBuffer) -> usize;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnInitDStream = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCtxSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnPledged = unsafe extern "C" fn(*mut c_void, u64) -> usize;

const E_CONTINUE: c_int = 0;
const E_FLUSH: c_int = 1;
const E_END: c_int = 2;

const C_COMPRESSIONLEVEL: c_int = 100;
const C_CHECKSUMFLAG: c_int = 201;
const C_CONTENTSIZEFLAG: c_int = 200;
const C_FORMAT: c_int = 10;
const C_NBWORKERS: c_int = 400;
const C_WINDOWLOG: c_int = 101;
const C_STRATEGY: c_int = 107;
const C_JOBSIZE: c_int = 401;
const C_OVERLAPLOG: c_int = 402;
const D_FORMAT: c_int = 1000;
const D_WINDOWLOGMAX: c_int = 100;

#[test]
fn cfg_stream_sizes() {
    for name in [
        "ZSTD_CStreamInSize",
        "ZSTD_CStreamOutSize",
        "ZSTD_DStreamInSize",
        "ZSTD_DStreamOutSize",
    ] {
        let (c, r) = unsafe { pair::<FnSize>(name) };
        assert_eq!(unsafe { c() }, unsafe { r() }, "{name}");
    }
}

/// Drive compressStream2 in both libs with identical chunking, comparing the
/// full byte stream and each call's return value.
fn stream_compress_diff(
    src: &[u8],
    params: &[(c_int, c_int)],
    pledged: Option<u64>,
    in_chunk: usize,
    out_chunk: usize,
    end_mode_pattern: &[c_int],
    ctx: &str,
) -> Option<Vec<u8>> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cp, rp) = unsafe { pair::<FnPledged>("ZSTD_CCtx_setPledgedSrcSize") };
    let (c2, r2) = unsafe { pair::<FnStream2>("ZSTD_compressStream2") };

    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut set_rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];

    for which in 0..2 {
        let ctxp = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
        let mut skip = false;
        for &(p, v) in params {
            let rc = if which == 0 {
                unsafe { cs(ctxp, p, v) }
            } else {
                unsafe { rs(ctxp, p, v) }
            };
            set_rets[which].push(rc);
            if is_error(rc) {
                skip = true;
            }
        }
        if let Some(n) = pledged {
            let rc = if which == 0 { unsafe { cp(ctxp, n) } } else { unsafe { rp(ctxp, n) } };
            set_rets[which].push(rc);
            if is_error(rc) {
                skip = true;
            }
        }
        if skip {
            if which == 0 {
                unsafe { cf(ctxp) };
            } else {
                unsafe { rf(ctxp) };
            }
            continue;
        }

        let mut input = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
        let mut outbuf = vec![0u8; out_chunk.max(1)];
        let mut collected: Vec<u8> = Vec::new();
        let mut mode_i = 0usize;
        let mut consumed_target = 0usize;
        let mut finished = false;
        let mut guard = 0;
        let mut prev_rc = 0usize; // 0 == nothing left to flush
        while !finished {
            guard += 1;
            assert!(guard < 2_000_000, "{ctx}: stream loop did not terminate");
            // Feed the next chunk only when the previous operation is fully
            // flushed and the current input has been consumed (adding input
            // after requesting an end/flush would violate the API contract).
            if prev_rc == 0 && input.pos == input.size && consumed_target < src.len() {
                consumed_target = (consumed_target + in_chunk).min(src.len());
                input.size = consumed_target;
            }
            let last_chunk = consumed_target == src.len();
            let mode = if last_chunk {
                E_END
            } else {
                end_mode_pattern[mode_i % end_mode_pattern.len()]
            };
            mode_i += 1;
            let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
            let rc = if which == 0 {
                unsafe { c2(ctxp, &mut out, &mut input, mode) }
            } else {
                unsafe { r2(ctxp, &mut out, &mut input, mode) }
            };
            rets[which].push(rc);
            assert!(!is_error(rc), "{ctx}: compressStream2 err {}", err_code(rc));
            collected.extend_from_slice(&outbuf[..out.pos]);
            prev_rc = if input.pos < input.size { 1 } else { rc };
            if last_chunk && rc == 0 && input.pos == input.size {
                finished = true;
            }
        }
        outs[which] = collected;
        if which == 0 {
            unsafe { cf(ctxp) };
        } else {
            unsafe { rf(ctxp) };
        }
    }
    assert_eq!(set_rets[0], set_rets[1], "{ctx}: setParameter return sequence differs");
    if set_rets[0].iter().any(|&x| is_error(x)) {
        return None; // configuration unsupported by BOTH builds (verified identical)
    }
    assert_eq!(rets[0], rets[1], "{ctx}: per-call return sequence differs");
    assert_bytes_eq(&format!("{ctx}: streamed frame"), &outs[0], &outs[1]);
    Some(outs[0].clone())
}

/// Drive decompressStream in both libs with identical chunking.
fn stream_decompress_diff(
    frame: &[u8],
    expect: Option<&[u8]>,
    dparams: &[(c_int, c_int)],
    in_chunk: usize,
    out_chunk: usize,
    ctx: &str,
) {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_DCtx_setParameter") };
    let (cd, rd) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };

    let mut outs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    for which in 0..2 {
        let dctx = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
        for &(p, v) in dparams {
            let rc = if which == 0 {
                unsafe { cs(dctx, p, v) }
            } else {
                unsafe { rs(dctx, p, v) }
            };
            assert!(!is_error(rc), "{ctx}: DCtx_setParameter({p},{v})");
        }
        let mut input = InBuffer { src: frame.as_ptr(), size: 0, pos: 0 };
        let mut outbuf = vec![0u8; out_chunk.max(1)];
        let mut collected = Vec::new();
        let mut avail = 0usize;
        let mut guard = 0;
        loop {
            guard += 1;
            assert!(guard < 2_000_000, "{ctx}: decompress loop stuck");
            if avail < frame.len() {
                avail = (avail + in_chunk).min(frame.len());
                input.size = avail;
            }
            let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
            let before_in = input.pos;
            let rc = if which == 0 {
                unsafe { cd(dctx, &mut out, &mut input) }
            } else {
                unsafe { rd(dctx, &mut out, &mut input) }
            };
            rets[which].push(rc);
            collected.extend_from_slice(&outbuf[..out.pos]);
            if is_error(rc) {
                break;
            }
            if rc == 0 && input.pos == frame.len() {
                break;
            }
            // no forward progress and no more input available => done/stuck
            if input.pos == before_in && out.pos == 0 && avail == frame.len() {
                break;
            }
        }
        outs[which] = collected;
        if which == 0 {
            unsafe { cf(dctx) };
        } else {
            unsafe { rf(dctx) };
        }
    }
    assert_eq!(rets[0], rets[1], "{ctx}: decompressStream return sequence differs");
    assert_bytes_eq(&format!("{ctx}: decompressed stream"), &outs[0], &outs[1]);
    if let Some(e) = expect {
        assert_bytes_eq(&format!("{ctx}: decompressed vs orig"), &outs[0], e);
    }
}

#[test]
fn cfg_stream2_chunk_matrix() {
    let mut rng = Rng::new(0x5EED);
    for &in_chunk in &[1usize, 7, 1000, 131072, 1 << 20] {
        for &out_chunk in &[1usize, 7, 1000, 131591, 1 << 20] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Rle] {
                let size = if in_chunk <= 7 || out_chunk <= 7 {
                    rng.range(0, 3000)
                } else {
                    rng.range(0, 300000)
                };
                let src = gen(shape, size, &mut rng);
                let ctx = format!(
                    "stream2 in={in_chunk} out={out_chunk} shape={shape:?} size={size}"
                );
                let frame_opt = stream_compress_diff(
                    &src,
                    &[(C_COMPRESSIONLEVEL, 3)],
                    None,
                    in_chunk,
                    out_chunk,
                    &[E_CONTINUE],
                    &ctx,
                );
                let Some(frame) = frame_opt else { continue };
                stream_decompress_diff(&frame, Some(&src), &[], in_chunk, out_chunk, &ctx);
            }
        }
    }
}

#[test]
fn cfg_stream2_endmode_patterns() {
    let mut rng = Rng::new(0xE0DE);
    let patterns: &[&[c_int]] = &[
        &[E_CONTINUE],
        &[E_FLUSH],
        &[E_CONTINUE, E_FLUSH],
        &[E_FLUSH, E_FLUSH, E_CONTINUE],
        &[E_END],
        &[E_CONTINUE, E_END],
    ];
    for pat in patterns {
        for &shape in &[Shape::Text, Shape::Mixed] {
            for &lvl in &[1, 3, 9, 19] {
                let size = rng.range(0, 200000);
                let src = gen(shape, size, &mut rng);
                let ctx = format!("endmode {pat:?} shape={shape:?} lvl={lvl} size={size}");
                let frame_opt = stream_compress_diff(
                    &src,
                    &[(C_COMPRESSIONLEVEL, lvl), (C_CHECKSUMFLAG, 1)],
                    None,
                    rng.range(1, 40000),
                    rng.range(1, 40000),
                    pat,
                    &ctx,
                );
                let Some(frame) = frame_opt else { continue };
                stream_decompress_diff(&frame, Some(&src), &[], 4096, 4096, &ctx);
            }
        }
    }
}

#[test]
fn cfg_stream2_params_matrix() {
    let mut rng = Rng::new(0x77);
    for &cksum in &[0, 1] {
        for &csize in &[0, 1] {
            for &fmt in &[0, 1] {
                for &pledge in &[false, true] {
                    for &lvl in &[-3, 1, 6, 12, 19] {
                        let size = rng.range(0, 250000);
                        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                        let src = gen(shape, size, &mut rng);
                        let ctx = format!(
                            "s2params cksum={cksum} csize={csize} fmt={fmt} pledge={pledge} lvl={lvl} shape={shape:?} size={size}"
                        );
                        let frame_opt = stream_compress_diff(
                            &src,
                            &[
                                (C_COMPRESSIONLEVEL, lvl),
                                (C_CHECKSUMFLAG, cksum),
                                (C_CONTENTSIZEFLAG, csize),
                                (C_FORMAT, fmt),
                            ],
                            if pledge { Some(src.len() as u64) } else { None },
                            rng.range(1, 70000),
                            rng.range(1, 70000),
                            &[E_CONTINUE],
                            &ctx,
                        );
                        let dp: Vec<(c_int, c_int)> =
                            if fmt == 1 { vec![(D_FORMAT, 1)] } else { vec![] };
                        let Some(frame) = frame_opt else { continue };
                        stream_decompress_diff(&frame, Some(&src), &dp, 8192, 8192, &ctx);
                    }
                }
            }
        }
    }
}

#[test]
fn cfg_stream2_multithreaded() {
    let mut rng = Rng::new(0x4711);
    for &workers in &[0, 1, 2, 4] {
        for &(jobsize, overlap) in &[(0, 0), (1 << 18, 0), (1 << 20, 6), (1 << 17, 9)] {
            for &shape in &[Shape::Text, Shape::Random] {
                let size = rng.range(100000, 900000);
                let src = gen(shape, size, &mut rng);
                let ctx = format!(
                    "mt workers={workers} job={jobsize} ovl={overlap} shape={shape:?} size={size}"
                );
                let frame_opt = stream_compress_diff(
                    &src,
                    &[
                        (C_COMPRESSIONLEVEL, 5),
                        (C_NBWORKERS, workers),
                        (C_JOBSIZE, jobsize),
                        (C_OVERLAPLOG, overlap),
                        (C_CHECKSUMFLAG, 1),
                    ],
                    None,
                    rng.range(1000, 200000),
                    rng.range(1000, 200000),
                    &[E_CONTINUE, E_FLUSH],
                    &ctx,
                );
                let Some(frame) = frame_opt else { continue };
                stream_decompress_diff(&frame, Some(&src), &[], 16384, 16384, &ctx);
            }
        }
    }
}

#[test]
fn cfg_old_cstream_api() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCStream") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCStream") };
    let (ci, ri) = unsafe { pair::<FnInitCStream>("ZSTD_initCStream") };
    let (cc, rc) = unsafe { pair::<FnStream>("ZSTD_compressStream") };
    let (cfl, rfl) = unsafe { pair::<FnFlush>("ZSTD_flushStream") };
    let (ce, re) = unsafe { pair::<FnFlush>("ZSTD_endStream") };
    let (csz, rsz) = unsafe { pair::<FnCtxSize>("ZSTD_sizeof_CStream") };
    let (ctf, rtf) = unsafe { pair::<FnCtxSize>("ZSTD_toFlushNow") };

    let mut rng = Rng::new(0x1234);
    let cs = unsafe { cn() };
    let ss = unsafe { rn() };
    for round in 0..12 {
        let lvl = [1, 3, 6, 11, 19][rng.below(5)];
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = rng.range(0, 150000);
        let src = gen(shape, size, &mut rng);
        let in_chunk = rng.range(1, 30000);
        let out_chunk = rng.range(1, 30000);
        let ctx = format!("oldCStream round={round} lvl={lvl} shape={shape:?} size={size}");

        let mut got: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { cs } else { ss };
            let rc0 = if which == 0 { unsafe { ci(z, lvl) } } else { unsafe { ri(z, lvl) } };
            assert!(!is_error(rc0), "{ctx}: initCStream");
            rets[which].push(rc0);
            let mut input = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
            let mut outbuf = vec![0u8; out_chunk];
            let mut coll = Vec::new();
            let mut avail = 0usize;
            while avail < src.len() || input.pos < input.size {
                if input.pos == input.size && avail < src.len() {
                    avail = (avail + in_chunk).min(src.len());
                    input.size = avail;
                }
                let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
                let rc = if which == 0 {
                    unsafe { cc(z, &mut out, &mut input) }
                } else {
                    unsafe { rc(z, &mut out, &mut input) }
                };
                rets[which].push(rc);
                assert!(!is_error(rc), "{ctx}: compressStream err {}", err_code(rc));
                coll.extend_from_slice(&outbuf[..out.pos]);
                let tf = if which == 0 { unsafe { ctf(z) } } else { unsafe { rtf(z) } };
                rets[which].push(tf);
            }
            // flush
            loop {
                let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
                let rc = if which == 0 {
                    unsafe { cfl(z, &mut out) }
                } else {
                    unsafe { rfl(z, &mut out) }
                };
                rets[which].push(rc);
                assert!(!is_error(rc));
                coll.extend_from_slice(&outbuf[..out.pos]);
                if rc == 0 {
                    break;
                }
            }
            // end
            loop {
                let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
                let rc = if which == 0 {
                    unsafe { ce(z, &mut out) }
                } else {
                    unsafe { re(z, &mut out) }
                };
                rets[which].push(rc);
                assert!(!is_error(rc));
                coll.extend_from_slice(&outbuf[..out.pos]);
                if rc == 0 {
                    break;
                }
            }
            let sz = if which == 0 { unsafe { csz(z) } } else { unsafe { rsz(z) } };
            rets[which].push(sz);
            got[which] = coll;
        }
        assert_eq!(rets[0], rets[1], "{ctx}: old-API return sequence");
        assert_bytes_eq(&ctx, &got[0], &got[1]);
        stream_decompress_diff(&got[0], Some(&src), &[], 4096, 4096, &ctx);
    }
    unsafe {
        cf(cs);
        rf(ss);
    }
}

#[test]
fn cfg_old_dstream_api() {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createDStream") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeDStream") };
    let (ci, ri) = unsafe { pair::<FnInitDStream>("ZSTD_initDStream") };
    let (cd, rd) = unsafe { pair::<FnStream>("ZSTD_decompressStream") };
    let (csz, rsz) = unsafe { pair::<FnCtxSize>("ZSTD_sizeof_DStream") };
    let (cbnd, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (ccomp, _rcomp) = unsafe {
        pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize>(
            "ZSTD_compress",
        )
    };

    let mut rng = Rng::new(0x8765);
    let cs = unsafe { cn() };
    let ss = unsafe { rn() };
    for round in 0..12 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = rng.range(0, 200000);
        let src = gen(shape, size, &mut rng);
        let cap = unsafe { cbnd(src.len()) } + 64;
        let mut frame = vec![0u8; cap];
        let n = unsafe {
            ccomp(frame.as_mut_ptr(), cap, src.as_ptr(), src.len(), [1, 5, 12, 19][rng.below(4)])
        };
        assert!(!is_error(n));
        frame.truncate(n);
        let in_chunk = rng.range(1, 20000);
        let out_chunk = rng.range(1, 20000);
        let ctx = format!("oldDStream round={round} shape={shape:?} size={size}");

        let mut got: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut rets: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        for which in 0..2 {
            let z = if which == 0 { cs } else { ss };
            let rc0 = if which == 0 { unsafe { ci(z) } } else { unsafe { ri(z) } };
            rets[which].push(rc0);
            let mut input = InBuffer { src: frame.as_ptr(), size: 0, pos: 0 };
            let mut outbuf = vec![0u8; out_chunk];
            let mut coll = Vec::new();
            let mut avail = 0usize;
            loop {
                if avail < frame.len() {
                    avail = (avail + in_chunk).min(frame.len());
                    input.size = avail;
                }
                let mut out = OutBuffer { dst: outbuf.as_mut_ptr(), size: outbuf.len(), pos: 0 };
                let rc = if which == 0 {
                    unsafe { cd(z, &mut out, &mut input) }
                } else {
                    unsafe { rd(z, &mut out, &mut input) }
                };
                rets[which].push(rc);
                assert!(!is_error(rc), "{ctx}: decompressStream err {}", err_code(rc));
                coll.extend_from_slice(&outbuf[..out.pos]);
                if rc == 0 {
                    break;
                }
            }
            let sz = if which == 0 { unsafe { csz(z) } } else { unsafe { rsz(z) } };
            rets[which].push(sz);
            got[which] = coll;
        }
        assert_eq!(rets[0], rets[1], "{ctx}: return sequence");
        assert_bytes_eq(&ctx, &got[0], &got[1]);
        assert_bytes_eq(&format!("{ctx} vs orig"), &got[0], &src);
    }
    unsafe {
        cf(cs);
        rf(ss);
    }
}

#[test]
fn cfg_stream_decompress_multiframe_and_skippable() {
    let mut rng = Rng::new(0xAAAA);
    // build: skippable frame, data frame, skippable, data frame
    let (cw, rw) = unsafe {
        pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, u32) -> usize>(
            "ZSTD_writeSkippableFrame",
        )
    };
    let mut all = Vec::new();
    let mut raw = Vec::new();
    for i in 0..3u32 {
        let payload = gen(Shape::Random, rng.range(0, 100), &mut rng);
        let mut cbuf = vec![0u8; payload.len() + 16];
        let mut rbuf = vec![0u8; payload.len() + 16];
        let a = unsafe {
            cw(cbuf.as_mut_ptr(), cbuf.len(), payload.as_ptr(), payload.len(), i % 16)
        };
        let b = unsafe {
            rw(rbuf.as_mut_ptr(), rbuf.len(), payload.as_ptr(), payload.len(), i % 16)
        };
        assert_eq!(a, b, "writeSkippableFrame");
        assert!(!is_error(a), "writeSkippableFrame err {}", err_code(a));
        assert_bytes_eq("skippable frame bytes", &cbuf[..a], &rbuf[..b]);
        all.extend_from_slice(&cbuf[..a]);

        let src = gen(Shape::Text, rng.range(1, 50000), &mut rng);
        let frame_opt = stream_compress_diff(
            &src,
            &[(C_COMPRESSIONLEVEL, 4), (C_CHECKSUMFLAG, (i % 2) as c_int)],
            None,
            9999,
            9999,
            &[E_CONTINUE],
            "multiframe part",
        );
        let frame = frame_opt.expect("multiframe part config supported");
        all.extend_from_slice(&frame);
        raw.extend_from_slice(&src);
    }
    stream_decompress_diff(&all, Some(&raw), &[], 1000, 1000, "multiframe+skippable");
    stream_decompress_diff(&all, Some(&raw), &[], 1, 1, "multiframe+skippable tiny");
}
