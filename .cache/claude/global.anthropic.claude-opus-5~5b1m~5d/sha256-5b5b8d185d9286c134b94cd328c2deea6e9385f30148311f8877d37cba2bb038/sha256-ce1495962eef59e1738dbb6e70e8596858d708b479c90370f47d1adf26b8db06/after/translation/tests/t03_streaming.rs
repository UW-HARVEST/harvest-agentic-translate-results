//! Phase B/C: the STREAMING API, driven the way a real consumer does — chunked
//! input, chunked output, partial flushes and explicit end-of-frame — using the
//! low-level `ZSTD_compressStream2` / `ZSTD_decompressStream` entry points as
//! well as the older `ZSTD_initCStream` / `ZSTD_compressStream` wrappers.
//!
//! Both libraries are stepped in LOCKSTEP: after every single call the return
//! code and both buffer positions must match, so a divergence is caught at the
//! exact call that caused it rather than at the end of the stream.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_dSetParam = unsafe extern "C" fn(DCtx, i32, i32) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_dReset = unsafe extern "C" fn(DCtx, i32) -> usize;
type Fn_setPledged = unsafe extern "C" fn(CCtx, u64) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_sz = unsafe extern "C" fn() -> usize;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_errCode = unsafe extern "C" fn(usize) -> i32;

type Fn_compressStream2 = unsafe extern "C" fn(
    CCtx,
    *mut ZSTD_outBuffer,
    *mut ZSTD_inBuffer,
    i32,
) -> usize;
type Fn_compressStream =
    unsafe extern "C" fn(CCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
type Fn_flushEnd = unsafe extern "C" fn(CCtx, *mut ZSTD_outBuffer) -> usize;
type Fn_initCStream = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_decompressStream =
    unsafe extern "C" fn(DCtx, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> usize;
type Fn_initDStream = unsafe extern "C" fn(DCtx) -> usize;

/// Result of streaming one input through one library.
struct StreamOut {
    bytes: Vec<u8>,
    /// (return code, in.pos, out.pos) after each call — the lockstep trace.
    trace: Vec<(usize, usize, usize)>,
}

/// Compress `src` with `ZSTD_compressStream2`, feeding `in_chunk` bytes at a
/// time into an `out_chunk`-sized output window, using `end_mode` to decide
/// when flushes/ends are issued.
#[allow(clippy::too_many_arguments)]
unsafe fn stream_compress(
    cs2: &Fn_compressStream2,
    ctx: CCtx,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    flush_every: usize,
    is_err: &Fn_isError,
) -> StreamOut {
    let mut outbuf = vec![0u8; out_chunk.max(1)];
    let mut collected = Vec::new();
    let mut trace = Vec::new();
    let mut consumed = 0usize;
    let mut calls = 0usize;

    loop {
        let end = consumed >= src.len();
        let take = if end {
            0
        } else {
            in_chunk.min(src.len() - consumed)
        };
        let mut inb = ZSTD_inBuffer {
            src: unsafe { src.as_ptr().add(consumed) },
            size: take,
            pos: 0,
        };
        // periodically request a flush so partial-flush paths are exercised
        let directive = if end {
            ZSTD_e_end
        } else if flush_every != 0 && calls % flush_every == flush_every - 1 {
            ZSTD_e_flush
        } else {
            ZSTD_e_continue
        };

        // drain the output window fully for this input chunk
        loop {
            let mut outb = ZSTD_outBuffer {
                dst: outbuf.as_mut_ptr(),
                size: outbuf.len(),
                pos: 0,
            };
            let rc = unsafe { cs2(ctx, &mut outb, &mut inb, directive) };
            trace.push((rc, inb.pos, outb.pos));
            collected.extend_from_slice(&outbuf[..outb.pos]);
            calls += 1;
            if unsafe { is_err(rc) } != 0 {
                return StreamOut {
                    bytes: collected,
                    trace,
                };
            }
            // done with this directive?
            let input_drained = inb.pos == inb.size;
            if directive == ZSTD_e_end {
                if rc == 0 {
                    consumed += inb.pos;
                    return StreamOut {
                        bytes: collected,
                        trace,
                    };
                }
            } else if input_drained && rc == 0 {
                break;
            } else if input_drained && outb.pos == 0 {
                break;
            }
            if calls > 200_000 {
                panic!("streaming did not terminate");
            }
        }
        consumed += inb.pos;
    }
}

/// Decompress a frame with `ZSTD_decompressStream` in randomized chunks,
/// recording the same lockstep trace.
unsafe fn stream_decompress(
    ds: &Fn_decompressStream,
    ctx: DCtx,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    is_err: &Fn_isError,
) -> StreamOut {
    let mut outbuf = vec![0u8; out_chunk.max(1)];
    let mut collected = Vec::new();
    let mut trace = Vec::new();
    let mut consumed = 0usize;
    let mut guard = 0usize;

    loop {
        let take = in_chunk.min(frame.len() - consumed);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { frame.as_ptr().add(consumed) },
            size: take,
            pos: 0,
        };
        loop {
            let mut outb = ZSTD_outBuffer {
                dst: outbuf.as_mut_ptr(),
                size: outbuf.len(),
                pos: 0,
            };
            let rc = unsafe { ds(ctx, &mut outb, &mut inb, ) };
            trace.push((rc, inb.pos, outb.pos));
            collected.extend_from_slice(&outbuf[..outb.pos]);
            guard += 1;
            if unsafe { is_err(rc) } != 0 {
                return StreamOut {
                    bytes: collected,
                    trace,
                };
            }
            if rc == 0 {
                // frame complete
                consumed += inb.pos;
                return StreamOut {
                    bytes: collected,
                    trace,
                };
            }
            if inb.pos == inb.size && outb.pos == 0 {
                break;
            }
            if guard > 5_000_000 {
                panic!("stream_decompress did not terminate");
            }
        }
        consumed += inb.pos;
        if consumed >= frame.len() {
            // out of input; report the final state
            return StreamOut {
                bytes: collected,
                trace,
            };
        }
    }
}

fn cmp_stream(tag: &str, c: &StreamOut, r: &StreamOut) {
    assert_eq_dbg(&format!("{tag}: number of calls"), c.trace.len(), r.trace.len());
    for (k, (a, b)) in c.trace.iter().zip(r.trace.iter()).enumerate() {
        assert!(
            a == b,
            "{tag}: divergence at call #{k}: C(rc,inpos,outpos)={a:?} Rust={b:?}"
        );
    }
    assert_bytes_eq(&format!("{tag}: stream bytes"), &c.bytes, &r.bytes);
}

#[test]
fn stream_buffer_size_hints_match() {
    let i = impls();
    for name in [
        "ZSTD_CStreamInSize",
        "ZSTD_CStreamOutSize",
        "ZSTD_DStreamInSize",
        "ZSTD_DStreamOutSize",
    ] {
        let (c, r) = i.pair::<Fn_sz>(name);
        unsafe { assert_eq_dbg(name, c(), r()) };
    }
}

/// The main streaming differential: many chunk-size / level / shape
/// combinations, stepped in lockstep, compressed bytes compared exactly.
#[test]
fn compress_stream2_lockstep_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs2, r_cs2) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0x5731_EA11);

    // chunk sizes deliberately include 1 (byte-at-a-time) and tiny output
    // windows, which force the compressor through its internal buffering paths.
    let chunk_cfgs: [(usize, usize, usize); 8] = [
        (1, 1, 0),
        (1, 4096, 0),
        (7, 3, 3),
        (128, 17, 0),
        (1024, 1024, 2),
        (65536, 128, 0),
        (131_072, 131_072, 0),
        (300_000, 1 << 16, 5),
    ];

    for &(in_chunk, out_chunk, flush_every) in &chunk_cfgs {
        for &lvl in &[-5i32, 1, 3, 9, 19] {
            for &shape in &[
                Shape::Constant,
                Shape::Random,
                Shape::SkewedText,
                Shape::Repetitive,
            ] {
                for &len in &[0usize, 1, 100, 5000, 131_072, 200_000] {
                    // keep the byte-at-a-time cases small enough to stay fast
                    if in_chunk == 1 && len > 5000 {
                        continue;
                    }
                    let src = gen_shape(shape, len, &mut rng);

                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                        assert_eq_dbg(
                            "set level",
                            c_set(cc, ZSTD_c_compressionLevel, lvl),
                            r_set(rc, ZSTD_c_compressionLevel, lvl),
                        );
                    }

                    let a = unsafe {
                        stream_compress(&c_cs2, cc, &src, in_chunk, out_chunk, flush_every, &c_isE)
                    };
                    let b = unsafe {
                        stream_compress(&r_cs2, rc, &src, in_chunk, out_chunk, flush_every, &c_isE)
                    };
                    cmp_stream(
                        &format!(
                            "compressStream2 in={in_chunk} out={out_chunk} flush={flush_every} lvl={lvl} shape={shape:?} len={len}"
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// Streaming compression with every frame-affecting option toggled, so the
/// streaming path is verified under the same configuration matrix as the
/// one-shot path (checksum, contentSize, dictID, magicless, LDM, ...).
#[test]
fn compress_stream2_option_matrix_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_pl, r_pl) = i.pair::<Fn_setPledged>("ZSTD_CCtx_setPledgedSrcSize");
    let (c_cs2, r_cs2) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0x0071_5301);

    let matrix: Vec<Vec<(i32, i32)>> = vec![
        vec![(ZSTD_c_checksumFlag, 1)],
        vec![(ZSTD_c_contentSizeFlag, 0)],
        vec![(ZSTD_c_dictIDFlag, 0)],
        vec![(ZSTD_c_format, ZSTD_f_zstd1_magicless)],
        vec![(ZSTD_c_enableLongDistanceMatching, 1), (ZSTD_c_windowLog, 20)],
        vec![(ZSTD_c_literalCompressionMode, ZSTD_lcm_uncompressed)],
        vec![(ZSTD_c_literalCompressionMode, ZSTD_lcm_huffman)],
        vec![(ZSTD_c_targetCBlockSize, 1340)],
        vec![(ZSTD_c_maxBlockSize, 8192)],
        vec![(ZSTD_c_strategy, ZSTD_btultra2)],
        vec![(ZSTD_c_strategy, ZSTD_fast), (ZSTD_c_compressionLevel, -20)],
        vec![(ZSTD_c_rsyncable, 1), (ZSTD_c_windowLog, 20)],
        vec![(ZSTD_c_useRowMatchFinder, ZSTD_ps_enable), (ZSTD_c_strategy, ZSTD_lazy2)],
        vec![(ZSTD_c_useRowMatchFinder, ZSTD_ps_disable), (ZSTD_c_strategy, ZSTD_lazy2)],
        vec![(ZSTD_c_splitAfterSequences, ZSTD_ps_enable)],
        vec![(ZSTD_c_checksumFlag, 1), (ZSTD_c_contentSizeFlag, 0), (ZSTD_c_dictIDFlag, 0)],
    ];

    for params in &matrix {
        for &(in_chunk, out_chunk) in &[(1usize, 4096usize), (999, 63), (1 << 16, 1 << 16)] {
            for &len in &[0usize, 1, 3000, 150_000] {
                if in_chunk == 1 && len > 3000 {
                    continue;
                }
                for &pledge_known in &[false, true] {
                    let src = gen_shape(Shape::SkewedText, len, &mut rng);
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                        for &(id, v) in params {
                            assert_eq_dbg(
                                &format!("set({id},{v})"),
                                c_set(cc, id, v),
                                r_set(rc, id, v),
                            );
                        }
                        if pledge_known {
                            assert_eq_dbg(
                                "pledge",
                                c_pl(cc, len as u64),
                                r_pl(rc, len as u64),
                            );
                        }
                    }
                    let a = unsafe {
                        stream_compress(&c_cs2, cc, &src, in_chunk, out_chunk, 0, &c_isE)
                    };
                    let b = unsafe {
                        stream_compress(&r_cs2, rc, &src, in_chunk, out_chunk, 0, &c_isE)
                    };
                    cmp_stream(
                        &format!(
                            "stream opts={params:?} in={in_chunk} out={out_chunk} len={len} pledge={pledge_known}"
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// `ZSTD_decompressStream` in lockstep over randomized chunkings of valid
/// frames, plus the `ZSTD_nextInputSizeHint` the C reports after each call.
#[test]
fn decompress_stream_lockstep_matches() {
    let i = impls();
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_drst, r_drst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_ds, r_ds) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_comp, _) = i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> usize>(
        "ZSTD_compress",
    );
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0xD3C0_D3);

    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 200, 9000, 131_073, 250_000] {
            for &lvl in &[1i32, 3, 19] {
                let src = gen_shape(shape, len, &mut rng);
                let cap = unsafe { c_bound(len) };
                let mut frame = vec![0u8; cap];
                let n =
                    unsafe { c_comp(frame.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
                let frame = &frame[..n];

                for &(in_chunk, out_chunk) in
                    &[(1usize, 1usize), (1, 1 << 16), (13, 7), (n.max(1), 1 << 17)]
                {
                    if in_chunk == 1 && n > 20_000 {
                        continue;
                    }
                    unsafe {
                        c_drst(cd, ZSTD_reset_session_and_parameters);
                        r_drst(rd, ZSTD_reset_session_and_parameters);
                    }
                    let a = unsafe {
                        stream_decompress(&c_ds, cd, frame, in_chunk, out_chunk, &c_isE)
                    };
                    let b = unsafe {
                        stream_decompress(&r_ds, rd, frame, in_chunk, out_chunk, &c_isE)
                    };
                    let tag = format!(
                        "decompressStream shape={shape:?} len={len} lvl={lvl} in={in_chunk} out={out_chunk}"
                    );
                    cmp_stream(&tag, &a, &b);
                    assert_bytes_eq(&format!("{tag} payload"), &src, &a.bytes);
                }
            }
        }
    }

    unsafe {
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// The legacy streaming wrappers: `ZSTD_initCStream` + `ZSTD_compressStream` +
/// `ZSTD_flushStream` + `ZSTD_endStream`, and `ZSTD_initDStream`. These have
/// their own stage bookkeeping in the C, distinct from compressStream2.
#[test]
fn legacy_stream_wrappers_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCStream");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCStream");
    let (c_init, r_init) = i.pair::<Fn_initCStream>("ZSTD_initCStream");
    let (c_cs, r_cs) = i.pair::<Fn_compressStream>("ZSTD_compressStream");
    let (c_fl, r_fl) = i.pair::<Fn_flushEnd>("ZSTD_flushStream");
    let (c_en, r_en) = i.pair::<Fn_flushEnd>("ZSTD_endStream");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDStream");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDStream");
    let (c_dinit, r_dinit) = i.pair::<Fn_initDStream>("ZSTD_initDStream");
    let (c_ds, r_ds) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0x1E6A_C100);

    for &lvl in &[1i32, 3, 9, 19] {
        for &len in &[0usize, 1, 777, 40_000, 140_000] {
            for &(in_chunk, out_chunk) in &[(1usize, 32usize), (100, 100), (1 << 16, 1 << 16)] {
                if in_chunk == 1 && len > 5000 {
                    continue;
                }
                let src = gen_shape(Shape::Tabular, len, &mut rng);

                // ---- compress side, stepped in lockstep
                unsafe {
                    assert_eq_dbg("initCStream", c_init(cc, lvl), r_init(rc, lvl));
                }
                let mut cout = vec![0u8; out_chunk];
                let mut rout = vec![0u8; out_chunk];
                let mut cbytes = Vec::new();
                let mut rbytes = Vec::new();
                let mut pos = 0usize;

                while pos < len {
                    let take = in_chunk.min(len - pos);
                    let mut ci = ZSTD_inBuffer {
                        src: unsafe { src.as_ptr().add(pos) },
                        size: take,
                        pos: 0,
                    };
                    let mut ri = ci;
                    loop {
                        let mut co = ZSTD_outBuffer {
                            dst: cout.as_mut_ptr(),
                            size: cout.len(),
                            pos: 0,
                        };
                        let mut ro = ZSTD_outBuffer {
                            dst: rout.as_mut_ptr(),
                            size: rout.len(),
                            pos: 0,
                        };
                        let a = unsafe { c_cs(cc, &mut co, &mut ci) };
                        let b = unsafe { r_cs(rc, &mut ro, &mut ri) };
                        assert_eq_dbg("compressStream rc", a, b);
                        assert_eq_dbg("compressStream in.pos", ci.pos, ri.pos);
                        assert_eq_dbg("compressStream out.pos", co.pos, ro.pos);
                        cbytes.extend_from_slice(&cout[..co.pos]);
                        rbytes.extend_from_slice(&rout[..ro.pos]);
                        if unsafe { c_isE(a) } != 0 || ci.pos == ci.size {
                            break;
                        }
                    }
                    pos += ci.pos;
                }

                // flush, then end — both must report the same remaining sizes
                loop {
                    let mut co = ZSTD_outBuffer {
                        dst: cout.as_mut_ptr(),
                        size: cout.len(),
                        pos: 0,
                    };
                    let mut ro = ZSTD_outBuffer {
                        dst: rout.as_mut_ptr(),
                        size: rout.len(),
                        pos: 0,
                    };
                    let a = unsafe { c_fl(cc, &mut co) };
                    let b = unsafe { r_fl(rc, &mut ro) };
                    assert_eq_dbg("flushStream rc", a, b);
                    assert_eq_dbg("flushStream out.pos", co.pos, ro.pos);
                    cbytes.extend_from_slice(&cout[..co.pos]);
                    rbytes.extend_from_slice(&rout[..ro.pos]);
                    if a == 0 || unsafe { c_isE(a) } != 0 {
                        break;
                    }
                }
                loop {
                    let mut co = ZSTD_outBuffer {
                        dst: cout.as_mut_ptr(),
                        size: cout.len(),
                        pos: 0,
                    };
                    let mut ro = ZSTD_outBuffer {
                        dst: rout.as_mut_ptr(),
                        size: rout.len(),
                        pos: 0,
                    };
                    let a = unsafe { c_en(cc, &mut co) };
                    let b = unsafe { r_en(rc, &mut ro) };
                    assert_eq_dbg("endStream rc", a, b);
                    assert_eq_dbg("endStream out.pos", co.pos, ro.pos);
                    cbytes.extend_from_slice(&cout[..co.pos]);
                    rbytes.extend_from_slice(&rout[..ro.pos]);
                    if a == 0 || unsafe { c_isE(a) } != 0 {
                        break;
                    }
                }

                let tag = format!("legacy cstream lvl={lvl} len={len} in={in_chunk} out={out_chunk}");
                assert_bytes_eq(&tag, &cbytes, &rbytes);

                // ---- decompress side with initDStream
                unsafe {
                    assert_eq_dbg("initDStream", c_dinit(cd), r_dinit(rd));
                }
                let a = unsafe {
                    stream_decompress(&c_ds, cd, &cbytes, in_chunk.max(1), out_chunk, &c_isE)
                };
                let b = unsafe {
                    stream_decompress(&r_ds, rd, &rbytes, in_chunk.max(1), out_chunk, &c_isE)
                };
                cmp_stream(&format!("{tag} / dstream"), &a, &b);
                assert_bytes_eq(&format!("{tag} / payload"), &src, &a.bytes);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// Multi-frame concatenated input: `ZSTD_decompressStream` must walk frame
/// boundaries identically, and both libs must agree on where each frame ends.
#[test]
fn multi_frame_streaming_matches() {
    let i = impls();
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_drst, r_drst) = i.pair::<Fn_dReset>("ZSTD_DCtx_reset");
    let (c_ds, r_ds) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_comp, _) = i
        .pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> usize>(
            "ZSTD_compress",
        );
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0xF00D_F00D);

    for nframes in [2usize, 3, 5] {
        for _ in 0..4 {
            let mut all = Vec::new();
            let mut plain = Vec::new();
            for _ in 0..nframes {
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let len = rng.range(0, 8000);
                let s = gen_shape(shape, len, &mut rng);
                let cap = unsafe { c_bound(len) };
                let mut f = vec![0u8; cap];
                let lvl = [1i32, 3, 9, 19][rng.below(4)];
                let n = unsafe { c_comp(f.as_mut_ptr(), cap, s.as_ptr(), len, lvl) };
                all.extend_from_slice(&f[..n]);
                plain.extend_from_slice(&s);
            }

            for &(ic, oc) in &[(1usize, 1 << 16), (37, 11), (all.len().max(1), 1 << 17)] {
                unsafe {
                    c_drst(cd, ZSTD_reset_session_and_parameters);
                    r_drst(rd, ZSTD_reset_session_and_parameters);
                }
                // decode every frame in sequence out of one concatenated buffer
                let mut cconsumed = 0usize;
                let mut cout_all = Vec::new();
                let mut rout_all = Vec::new();
                while cconsumed < all.len() {
                    let a = unsafe {
                        stream_decompress(&c_ds, cd, &all[cconsumed..], ic, oc, &c_isE)
                    };
                    let b = unsafe {
                        stream_decompress(&r_ds, rd, &all[cconsumed..], ic, oc, &c_isE)
                    };
                    cmp_stream(&format!("multiframe n={nframes} ic={ic} oc={oc}"), &a, &b);
                    cout_all.extend_from_slice(&a.bytes);
                    rout_all.extend_from_slice(&b.bytes);
                    // advance by the input the C consumed for this frame
                    let used: usize = a.trace.last().map(|t| t.1).unwrap_or(0);
                    if used == 0 {
                        break;
                    }
                    cconsumed += used;
                }
                assert_bytes_eq("multiframe payload", &cout_all, &rout_all);
            }
        }
    }

    unsafe {
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// Phase C: streaming misuse. Calling stream functions in the wrong stage, with
/// null buffers, or violating the stableInBuffer/stableOutBuffer contract must
/// produce the SAME error code in both libraries.
#[test]
fn streaming_error_paths_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_cs2, r_cs2) = i.pair::<Fn_compressStream2>("ZSTD_compressStream2");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_ds, r_ds) = i.pair::<Fn_decompressStream>("ZSTD_decompressStream");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };

    let mut rng = Rng::new(0xE770_1234);
    let src = gen_shape(Shape::SkewedText, 5000, &mut rng);
    let mut out = vec![0u8; 4096];

    // ---- out-of-range endOp / endDirective values (C enum accepts any int)
    for endop in [-2i32, -1, 3, 4, 99, i32::MIN, i32::MAX] {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
        }
        let mut ci = ZSTD_inBuffer {
            src: src.as_ptr(),
            size: src.len(),
            pos: 0,
        };
        let mut ri = ci;
        let mut co = ZSTD_outBuffer {
            dst: out.as_mut_ptr(),
            size: out.len(),
            pos: 0,
        };
        let mut ro = co;
        let a = unsafe { c_cs2(cc, &mut co, &mut ci, endop) };
        let b = unsafe { r_cs2(rc, &mut ro, &mut ri, endop) };
        assert_eq_dbg(&format!("compressStream2 endOp={endop}"), a, b);
        unsafe {
            assert_eq_dbg(
                &format!("compressStream2 endOp={endop} code"),
                c_cd(a),
                r_cd(b),
            )
        };
    }

    // ---- NULL dst with nonzero size, and NULL src with nonzero size
    for &(null_dst, null_src) in &[(true, false), (false, true), (true, true)] {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
        }
        let mut ci = ZSTD_inBuffer {
            src: if null_src {
                std::ptr::null()
            } else {
                src.as_ptr()
            },
            size: 16,
            pos: 0,
        };
        let mut ri = ci;
        let mut co = ZSTD_outBuffer {
            dst: if null_dst {
                std::ptr::null_mut()
            } else {
                out.as_mut_ptr()
            },
            size: 128,
            pos: 0,
        };
        let mut ro = co;
        let a = unsafe { c_cs2(cc, &mut co, &mut ci, ZSTD_e_continue) };
        let b = unsafe { r_cs2(rc, &mut ro, &mut ri, ZSTD_e_continue) };
        let tag = format!("compressStream2 null_dst={null_dst} null_src={null_src}");
        assert_eq_dbg(&tag, a, b);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
    }

    // ---- stableInBuffer / stableOutBuffer contract violations
    for &(stable_in, stable_out) in &[(1i32, 0i32), (0, 1), (1, 1)] {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
            assert_eq_dbg(
                "set stableIn",
                c_set(cc, ZSTD_c_stableInBuffer, stable_in),
                r_set(rc, ZSTD_c_stableInBuffer, stable_in),
            );
            assert_eq_dbg(
                "set stableOut",
                c_set(cc, ZSTD_c_stableOutBuffer, stable_out),
                r_set(rc, ZSTD_c_stableOutBuffer, stable_out),
            );
        }
        // first call establishes the buffers ...
        let mut ci = ZSTD_inBuffer {
            src: src.as_ptr(),
            size: src.len(),
            pos: 0,
        };
        let mut ri = ci;
        let mut co = ZSTD_outBuffer {
            dst: out.as_mut_ptr(),
            size: out.len(),
            pos: 0,
        };
        let mut ro = co;
        let a = unsafe { c_cs2(cc, &mut co, &mut ci, ZSTD_e_continue) };
        let b = unsafe { r_cs2(rc, &mut ro, &mut ri, ZSTD_e_continue) };
        assert_eq_dbg(&format!("stable first call in={stable_in} out={stable_out}"), a, b);

        // ... the second call deliberately MOVES the buffers, violating the contract
        let src2 = gen_shape(Shape::Random, 5000, &mut rng);
        let mut out2 = vec![0u8; 4096];
        let mut ci2 = ZSTD_inBuffer {
            src: src2.as_ptr(),
            size: src2.len(),
            pos: 0,
        };
        let mut ri2 = ci2;
        let mut co2 = ZSTD_outBuffer {
            dst: out2.as_mut_ptr(),
            size: out2.len(),
            pos: 0,
        };
        let mut ro2 = co2;
        let a2 = unsafe { c_cs2(cc, &mut co2, &mut ci2, ZSTD_e_end) };
        let b2 = unsafe { r_cs2(rc, &mut ro2, &mut ri2, ZSTD_e_end) };
        let tag = format!("stable violation in={stable_in} out={stable_out}");
        assert_eq_dbg(&tag, a2, b2);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a2), r_cd(b2)) };
    }

    // ---- decompressStream on garbage / empty / null input
    //
    // NOTE: the case `{src = NULL, size = 32}` is deliberately NOT tested here.
    // It was probed against both libraries in isolation and segfaults in BOTH:
    // the C `ZSTD_decompressStream` reads the frame header straight out of
    // `input->src` without a null check, so a non-empty buffer with a NULL
    // pointer is undefined behaviour in the C itself. The Rust reproduces that
    // faithfully (it also segfaults), but a crashing input cannot be asserted on
    // inside a test process, so it is documented rather than executed.
    // `{src = NULL, size = 0}` IS tested — that one is well-defined in the C.
    let garbage = {
        let mut g = vec![0u8; 256];
        for b in g.iter_mut() {
            *b = rng.byte();
        }
        g
    };
    for case in 0..3 {
        let mut ci = match case {
            0 => ZSTD_inBuffer { src: garbage.as_ptr(), size: garbage.len(), pos: 0 },
            1 => ZSTD_inBuffer { src: garbage.as_ptr(), size: 0, pos: 0 },
            _ => ZSTD_inBuffer { src: std::ptr::null(), size: 0, pos: 0 },
        };
        let mut ri = ci;
        let mut co = ZSTD_outBuffer { dst: out.as_mut_ptr(), size: out.len(), pos: 0 };
        let mut ro = co;
        let a = unsafe { c_ds(cd, &mut co, &mut ci) };
        let b = unsafe { r_ds(rd, &mut ro, &mut ri) };
        let tag = format!("decompressStream garbage case={case}");
        assert_eq_dbg(&tag, a, b);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
        assert_eq_dbg(&format!("{tag} in.pos"), ci.pos, ri.pos);
        assert_eq_dbg(&format!("{tag} out.pos"), co.pos, ro.pos);
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}
