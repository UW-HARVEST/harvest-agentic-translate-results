//! Phase B — valid-path differential tests for the STREAMING surface.
//!
//! Covers `CONFIGS.md` sections "Streaming compression / reset",
//! "Streaming decompression / reset" and "Decompression parameters".
//!
//! Streaming is driven the way a real consumer drives it: randomized input and
//! output chunk sizes (including 1-byte buffers, which force the internal
//! buffered path), every `ZSTD_EndDirective`, and every reset directive.

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnSetPledged = unsafe extern "C" fn(*mut c_void, u64) -> size_t;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnInitCStreamSrcSize = unsafe extern "C" fn(*mut c_void, c_int, u64) -> size_t;
type FnInitCStreamUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnInitCStreamAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    size_t,
    ZSTD_parameters,
    u64,
) -> size_t;
type FnFlushEnd = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> size_t;
type FnCompressStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
type FnResetCStream = unsafe extern "C" fn(*mut c_void, u64) -> size_t;
type FnInitDStreamDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnSimpleArgs = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *mut size_t,
    *const c_void,
    size_t,
    *mut size_t,
    c_int,
) -> size_t;
type FnDSimpleArgs = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *mut size_t,
    *const c_void,
    size_t,
    *mut size_t,
) -> size_t;

struct S {
    create_c: (FnCreate, FnCreate),
    free_c: (FnFree, FnFree),
    create_d: (FnCreate, FnCreate),
    free_d: (FnFree, FnFree),
    setp: (FnSetParam, FnSetParam),
    setdp: (FnSetParam, FnSetParam),
    getdp: (FnGetParam, FnGetParam),
    reset_c: (FnReset, FnReset),
    reset_d: (FnReset, FnReset),
    pledged: (FnSetPledged, FnSetPledged),
    cs2: (FnStream, FnStream),
    ds: (FnDStream, FnDStream),
    is_err: (FnIsError, FnIsError),
    ecode: (FnGetErrorCode, FnGetErrorCode),
    bound: (FnSizeSize, FnSizeSize),
}

fn s() -> S {
    S {
        create_c: fnpair!("ZSTD_createCStream", FnCreate),
        free_c: fnpair!("ZSTD_freeCStream", FnFree),
        create_d: fnpair!("ZSTD_createDStream", FnCreate),
        free_d: fnpair!("ZSTD_freeDStream", FnFree),
        setp: fnpair!("ZSTD_CCtx_setParameter", FnSetParam),
        setdp: fnpair!("ZSTD_DCtx_setParameter", FnSetParam),
        getdp: fnpair!("ZSTD_DCtx_getParameter", FnGetParam),
        reset_c: fnpair!("ZSTD_CCtx_reset", FnReset),
        reset_d: fnpair!("ZSTD_DCtx_reset", FnReset),
        pledged: fnpair!("ZSTD_CCtx_setPledgedSrcSize", FnSetPledged),
        cs2: fnpair!("ZSTD_compressStream2", FnStream),
        ds: fnpair!("ZSTD_decompressStream", FnDStream),
        is_err: fnpair!("ZSTD_isError", FnIsError),
        ecode: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        bound: fnpair!("ZSTD_compressBound", FnSizeSize),
    }
}

/// Drive `ZSTD_compressStream2` to completion on one library, recording the
/// exact sequence of `(ret, in.pos, out.pos)` triples so the two
/// implementations can be compared step-by-step, not just on final output.
unsafe fn run_cstream(
    f: FnStream,
    ctx: *mut c_void,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    endop: c_int,
) -> (Vec<u8>, Vec<(size_t, size_t, size_t)>, bool) {
    let mut out = Vec::new();
    let mut trace = Vec::new();
    let mut buf = vec![0xAAu8; out_chunk.max(1)];
    let mut consumed = 0usize;
    let mut errored = false;
    loop {
        let take = in_chunk.min(src.len() - consumed);
        let mut ib = ZSTD_inBuffer {
            src: if src.is_empty() {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
            } else {
                src.as_ptr().add(consumed) as *const c_void
            },
            size: take,
            pos: 0,
        };
        let last = consumed + take == src.len();
        let op = if last { endop } else { ZSTD_e_continue };
        loop {
            let mut ob = ZSTD_outBuffer {
                dst: buf.as_mut_ptr() as *mut c_void,
                size: buf.len(),
                pos: 0,
            };
            let r = f(ctx, &mut ob, &mut ib, op);
            trace.push((r, ib.pos, ob.pos));
            out.extend_from_slice(&buf[..ob.pos]);
            if (0usize.wrapping_sub(r)) <= 120 {
                errored = true;
                return (out, trace, errored);
            }
            if op == ZSTD_e_continue {
                if ib.pos == ib.size {
                    break;
                }
            } else if r == 0 {
                break;
            }
            if ob.pos == 0 && ib.pos == ib.size && r == 0 {
                break;
            }
        }
        consumed += ib.pos;
        if last {
            break;
        }
    }
    (out, trace, errored)
}

unsafe fn run_dstream(
    f: FnDStream,
    ctx: *mut c_void,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
) -> (Vec<u8>, Vec<(size_t, size_t, size_t)>, bool) {
    let mut out = Vec::new();
    let mut trace = Vec::new();
    let mut buf = vec![0xAAu8; out_chunk.max(1)];
    let mut pos = 0usize;
    loop {
        let take = in_chunk.min(frame.len() - pos);
        let mut ib = ZSTD_inBuffer {
            src: if frame.is_empty() {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
            } else {
                frame.as_ptr().add(pos) as *const c_void
            },
            size: take,
            pos: 0,
        };
        loop {
            let mut ob = ZSTD_outBuffer {
                dst: buf.as_mut_ptr() as *mut c_void,
                size: buf.len(),
                pos: 0,
            };
            let r = f(ctx, &mut ob, &mut ib);
            trace.push((r, ib.pos, ob.pos));
            out.extend_from_slice(&buf[..ob.pos]);
            if (0usize.wrapping_sub(r)) <= 120 {
                return (out, trace, true);
            }
            if r == 0 {
                pos += ib.pos;
                return (out, trace, false);
            }
            if ib.pos == ib.size && ob.pos == 0 {
                break;
            }
        }
        pos += ib.pos;
        if pos >= frame.len() {
            // no forward progress possible: truncated input
            return (out, trace, false);
        }
    }
}

#[track_caller]
fn cmp_trace(ctx: &str, a: &[(size_t, size_t, size_t)], b: &[(size_t, size_t, size_t)]) {
    assert_eq!(a.len(), b.len(), "{ctx}: step count C={} R={}", a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "{ctx}: step {i} differs C={x:?} R={y:?}");
    }
}

// ================ CONFIGS: compressStream2 x chunking x endOp ==============

#[test]
fn b_compress_stream2_chunking() {
    let s = s();
    let mut rng = Rng::new(0x51EA);
    let chunks = [1usize, 2, 7, 100, 1024, 4096, 1 << 16, usize::MAX / 2];
    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 5, 1000, 70_000, 200_000] {
                let src = gen(shape, len, &mut rng);
                for &ic in &chunks {
                    for &oc in &[1usize, 3, 129, 4096, 1 << 17] {
                        for endop in [ZSTD_e_end, ZSTD_e_flush] {
                            for lvl in [1, 6, 19] {
                                let cc = (s.create_c.0)();
                                let rc = (s.create_c.1)();
                                assert_eq!(
                                    (s.setp.0)(cc, ZSTD_c_compressionLevel, lvl),
                                    (s.setp.1)(rc, ZSTD_c_compressionLevel, lvl)
                                );
                                let tag = format!(
                                    "cs2 {shape:?} len={len} ic={ic} oc={oc} endop={endop} lvl={lvl}"
                                );
                                let (o1, t1, e1) = run_cstream(s.cs2.0, cc, &src, ic, oc, endop);
                                let (o2, t2, e2) = run_cstream(s.cs2.1, rc, &src, ic, oc, endop);
                                assert_eq!(e1, e2, "{tag}: errored differs");
                                cmp_trace(&tag, &t1, &t2);
                                assert_bytes_eq(&tag, &o1, &o2);
                                (s.free_c.0)(cc);
                                (s.free_c.1)(rc);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================== CONFIGS: decompressStream x chunking ===================

#[test]
fn b_decompress_stream_chunking() {
    let s = s();
    let (cc2, rc2) = fnpair!(
        "ZSTD_compress2",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );
    let (ccc, rcc) = fnpair!("ZSTD_createCCtx", FnCreate);
    let (cfc, rfc) = fnpair!("ZSTD_freeCCtx", FnFree);
    let mut rng = Rng::new(0xD51EA);
    unsafe {
        for &shape in &[Shape::Text, Shape::Random, Shape::Zeros, Shape::LongRange] {
            for &len in &[0usize, 1, 1000, 140_000] {
                for &(csf, ckf) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let src = gen(shape, len, &mut rng);
                    // produce the frame with the C library (ground truth bytes)
                    let cap = (s.bound.0)(len).max(64);
                    let mut fbuf = vec![0u8; cap];
                    let cx = ccc();
                    let rx = rcc();
                    assert_eq!(
                        (s.setp.0)(cx, ZSTD_c_contentSizeFlag, csf),
                        (s.setp.1)(rx, ZSTD_c_contentSizeFlag, csf)
                    );
                    assert_eq!(
                        (s.setp.0)(cx, ZSTD_c_checksumFlag, ckf),
                        (s.setp.1)(rx, ZSTD_c_checksumFlag, ckf)
                    );
                    let n = cc2(
                        cx,
                        fbuf.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                    );
                    let mut fb2 = vec![0u8; cap];
                    let n2 = rc2(
                        rx,
                        fb2.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                    );
                    assert_eq!(n, n2);
                    assert_bytes_eq("frame gen", &fbuf[..n], &fb2[..n2]);
                    cfc(cx);
                    rfc(rx);
                    let frame = &fbuf[..n];

                    for &ic in &[1usize, 3, 64, 4096, usize::MAX / 2] {
                        for &oc in &[1usize, 5, 1024, 1 << 17] {
                            let cd = (s.create_d.0)();
                            let rd = (s.create_d.1)();
                            let tag = format!(
                                "ds {shape:?} len={len} csf={csf} ckf={ckf} ic={ic} oc={oc}"
                            );
                            let (o1, t1, e1) = run_dstream(s.ds.0, cd, frame, ic, oc);
                            let (o2, t2, e2) = run_dstream(s.ds.1, rd, frame, ic, oc);
                            assert_eq!(e1, e2, "{tag}: errored differs");
                            cmp_trace(&tag, &t1, &t2);
                            assert_bytes_eq(&tag, &o1, &o2);
                            if !e1 {
                                assert_bytes_eq(&format!("{tag}: roundtrip"), &src, &o1);
                            }
                            (s.free_d.0)(cd);
                            (s.free_d.1)(rd);
                        }
                    }
                }
            }
        }
    }
}

// ============= CONFIGS: legacy init/reset streaming entry points ============

#[test]
fn b_init_cstream_variants() {
    let s = s();
    let (c_i, r_i) = fnpair!("ZSTD_initCStream", FnInitCStream);
    let (c_is, r_is) = fnpair!("ZSTD_initCStream_srcSize", FnInitCStreamSrcSize);
    let (c_iud, r_iud) = fnpair!("ZSTD_initCStream_usingDict", FnInitCStreamUsingDict);
    let (c_ia, r_ia) = fnpair!("ZSTD_initCStream_advanced", FnInitCStreamAdvanced);
    let (c_rs, r_rs) = fnpair!("ZSTD_resetCStream", FnResetCStream);
    let (c_cs, r_cs) = fnpair!("ZSTD_compressStream", FnCompressStream);
    let (c_fl, r_fl) = fnpair!("ZSTD_flushStream", FnFlushEnd);
    let (c_en, r_en) = fnpair!("ZSTD_endStream", FnFlushEnd);
    let (c_gp, r_gp) = fnpair!(
        "ZSTD_getParams",
        unsafe extern "C" fn(c_int, u64, size_t) -> ZSTD_parameters
    );
    let (c_cis, r_cis) = fnpair!("ZSTD_CStreamInSize", unsafe extern "C" fn() -> size_t);
    let (c_cos, r_cos) = fnpair!("ZSTD_CStreamOutSize", unsafe extern "C" fn() -> size_t);

    unsafe {
        assert_eq!(c_cis(), r_cis(), "CStreamInSize");
        assert_eq!(c_cos(), r_cos(), "CStreamOutSize");
    }

    /// Drive the legacy 3-call streaming protocol and record every step.
    unsafe fn legacy_stream(
        cs: FnCompressStream,
        fl: FnFlushEnd,
        en: FnFlushEnd,
        ctx: *mut c_void,
        src: &[u8],
        ic: usize,
        oc: usize,
        flush_every: usize,
    ) -> (Vec<u8>, Vec<(size_t, size_t, size_t)>) {
        let mut out = Vec::new();
        let mut trace = Vec::new();
        let mut buf = vec![0xAAu8; oc.max(1)];
        let mut pos = 0usize;
        let mut i = 0usize;
        while pos < src.len() {
            let take = ic.min(src.len() - pos);
            let mut ib = ZSTD_inBuffer {
                src: src.as_ptr().add(pos) as *const c_void,
                size: take,
                pos: 0,
            };
            while ib.pos < ib.size {
                let mut ob = ZSTD_outBuffer {
                    dst: buf.as_mut_ptr() as *mut c_void,
                    size: buf.len(),
                    pos: 0,
                };
                let r = cs(ctx, &mut ob, &mut ib);
                trace.push((r, ib.pos, ob.pos));
                out.extend_from_slice(&buf[..ob.pos]);
                if (0usize.wrapping_sub(r)) <= 120 {
                    return (out, trace);
                }
            }
            pos += ib.pos;
            i += 1;
            if flush_every != 0 && i % flush_every == 0 {
                loop {
                    let mut ob = ZSTD_outBuffer {
                        dst: buf.as_mut_ptr() as *mut c_void,
                        size: buf.len(),
                        pos: 0,
                    };
                    let r = fl(ctx, &mut ob);
                    trace.push((r, 0, ob.pos));
                    out.extend_from_slice(&buf[..ob.pos]);
                    if (0usize.wrapping_sub(r)) <= 120 || r == 0 {
                        break;
                    }
                }
            }
        }
        loop {
            let mut ob = ZSTD_outBuffer {
                dst: buf.as_mut_ptr() as *mut c_void,
                size: buf.len(),
                pos: 0,
            };
            let r = en(ctx, &mut ob);
            trace.push((r, 0, ob.pos));
            out.extend_from_slice(&buf[..ob.pos]);
            if (0usize.wrapping_sub(r)) <= 120 || r == 0 {
                break;
            }
        }
        (out, trace)
    }

    let mut rng = Rng::new(0x11EE);
    unsafe {
        for &shape in &[Shape::Text, Shape::Mixed, Shape::Random, Shape::Zeros] {
            for &len in &[0usize, 1, 900, 70_000, 200_000] {
                let src = gen(shape, len, &mut rng);
                let dict = gen(Shape::Text, 4096, &mut rng);
                for lvl in [1, 3, 12, 19] {
                    for &ic in &[1usize, 37, 8192, usize::MAX / 2] {
                        for &oc in &[1usize, 64, 1 << 17] {
                            for flush_every in [0usize, 1, 3] {
                                let cc = (s.create_c.0)();
                                let rc = (s.create_c.1)();
                                let variants: Vec<(&str, Box<dyn Fn(*mut c_void, bool) -> size_t>)> = vec![];
                                drop(variants);

                                // --- ZSTD_initCStream
                                let tag = format!("initCStream {shape:?} len={len} lvl={lvl} ic={ic} oc={oc} fe={flush_every}");
                                assert_eq!(c_i(cc, lvl), r_i(rc, lvl), "{tag}: init rc");
                                let (o1, t1) = legacy_stream(c_cs, c_fl, c_en, cc, &src, ic, oc, flush_every);
                                let (o2, t2) = legacy_stream(r_cs, r_fl, r_en, rc, &src, ic, oc, flush_every);
                                cmp_trace(&tag, &t1, &t2);
                                assert_bytes_eq(&tag, &o1, &o2);

                                // --- ZSTD_resetCStream (reuse the same ctx)
                                let tag = format!("resetCStream {shape:?} len={len} lvl={lvl}");
                                assert_eq!(
                                    c_rs(cc, len as u64),
                                    r_rs(rc, len as u64),
                                    "{tag}: reset rc"
                                );
                                let (o1, t1) = legacy_stream(c_cs, c_fl, c_en, cc, &src, ic, oc, flush_every);
                                let (o2, t2) = legacy_stream(r_cs, r_fl, r_en, rc, &src, ic, oc, flush_every);
                                cmp_trace(&tag, &t1, &t2);
                                assert_bytes_eq(&tag, &o1, &o2);

                                // --- ZSTD_initCStream_srcSize (known + unknown)
                                for pledged in [len as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                                    let tag = format!("initCStream_srcSize={pledged} {shape:?} len={len} lvl={lvl}");
                                    assert_eq!(
                                        c_is(cc, lvl, pledged),
                                        r_is(rc, lvl, pledged),
                                        "{tag}: rc"
                                    );
                                    let (o1, t1) = legacy_stream(c_cs, c_fl, c_en, cc, &src, ic, oc, flush_every);
                                    let (o2, t2) = legacy_stream(r_cs, r_fl, r_en, rc, &src, ic, oc, flush_every);
                                    cmp_trace(&tag, &t1, &t2);
                                    assert_bytes_eq(&tag, &o1, &o2);
                                }

                                // --- ZSTD_initCStream_usingDict
                                let tag = format!("initCStream_usingDict {shape:?} len={len} lvl={lvl}");
                                assert_eq!(
                                    c_iud(cc, dict.as_ptr() as *const c_void, dict.len(), lvl),
                                    r_iud(rc, dict.as_ptr() as *const c_void, dict.len(), lvl),
                                    "{tag}: rc"
                                );
                                let (o1, t1) = legacy_stream(c_cs, c_fl, c_en, cc, &src, ic, oc, flush_every);
                                let (o2, t2) = legacy_stream(r_cs, r_fl, r_en, rc, &src, ic, oc, flush_every);
                                cmp_trace(&tag, &t1, &t2);
                                assert_bytes_eq(&tag, &o1, &o2);

                                // --- ZSTD_initCStream_advanced
                                let p1 = c_gp(lvl, len as u64, dict.len());
                                let p2 = r_gp(lvl, len as u64, dict.len());
                                assert_eq!(p1, p2, "getParams");
                                let tag = format!("initCStream_advanced {shape:?} len={len} lvl={lvl}");
                                let a = c_ia(cc, dict.as_ptr() as *const c_void, dict.len(), p1, len as u64);
                                let b = r_ia(rc, dict.as_ptr() as *const c_void, dict.len(), p2, len as u64);
                                assert_eq!(a, b, "{tag}: rc");
                                if (s.is_err.0)(a) == 0 {
                                    let (o1, t1) = legacy_stream(c_cs, c_fl, c_en, cc, &src, ic, oc, flush_every);
                                    let (o2, t2) = legacy_stream(r_cs, r_fl, r_en, rc, &src, ic, oc, flush_every);
                                    cmp_trace(&tag, &t1, &t2);
                                    assert_bytes_eq(&tag, &o1, &o2);
                                }

                                (s.free_c.0)(cc);
                                (s.free_c.1)(rc);
                            }
                        }
                    }
                }
            }
        }
    }
}

// =============== CONFIGS: initDStream variants + resetDStream ==============

#[test]
fn b_init_dstream_variants() {
    let s = s();
    let (c_id, r_id) = fnpair!("ZSTD_initDStream", FnFree);
    let (c_idd, r_idd) = fnpair!("ZSTD_initDStream_usingDict", FnInitDStreamDict);
    let (c_rd, r_rd) = fnpair!("ZSTD_resetDStream", FnFree);
    let (c_dis, r_dis) = fnpair!("ZSTD_DStreamInSize", unsafe extern "C" fn() -> size_t);
    let (c_dos, r_dos) = fnpair!("ZSTD_DStreamOutSize", unsafe extern "C" fn() -> size_t);
    let (c_cud, r_cud) = fnpair!(
        "ZSTD_compress_usingDict",
        unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            size_t,
            *const c_void,
            size_t,
            *const c_void,
            size_t,
            c_int,
        ) -> size_t
    );
    let (ccc, rcc) = fnpair!("ZSTD_createCCtx", FnCreate);
    let (cfc, rfc) = fnpair!("ZSTD_freeCCtx", FnFree);

    unsafe {
        assert_eq!(c_dis(), r_dis(), "DStreamInSize");
        assert_eq!(c_dos(), r_dos(), "DStreamOutSize");
    }

    let mut rng = Rng::new(0x22FF);
    unsafe {
        let cx = ccc();
        let rx = rcc();
        for &shape in &[Shape::Text, Shape::Mixed, Shape::Random] {
            for &len in &[0usize, 1, 5000, 150_000] {
                let src = gen(shape, len, &mut rng);
                let dict = gen(Shape::Text, 4096, &mut rng);
                for lvl in [1, 9, 19] {
                    let cap = (s.bound.0)(len).max(64);
                    let mut f1 = vec![0u8; cap];
                    let mut f2 = vec![0u8; cap];
                    let n1 = c_cud(
                        cx,
                        f1.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    );
                    let n2 = r_cud(
                        rx,
                        f2.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        len,
                        dict.as_ptr() as *const c_void,
                        dict.len(),
                        lvl,
                    );
                    assert_eq!(n1, n2, "usingDict frame size");
                    assert_bytes_eq("usingDict frame", &f1[..n1], &f2[..n2]);
                    let frame = &f1[..n1];
                    for &ic in &[1usize, 91, usize::MAX / 2] {
                        for &oc in &[1usize, 777, 1 << 17] {
                            let cd = (s.create_d.0)();
                            let rd = (s.create_d.1)();
                            // initDStream_usingDict then decode
                            let tag = format!("initDStream_usingDict {shape:?} len={len} lvl={lvl} ic={ic} oc={oc}");
                            assert_eq!(
                                c_idd(cd, dict.as_ptr() as *const c_void, dict.len()),
                                r_idd(rd, dict.as_ptr() as *const c_void, dict.len()),
                                "{tag}: init rc"
                            );
                            let (o1, t1, e1) = run_dstream(s.ds.0, cd, frame, ic, oc);
                            let (o2, t2, e2) = run_dstream(s.ds.1, rd, frame, ic, oc);
                            assert_eq!(e1, e2, "{tag}: errored");
                            cmp_trace(&tag, &t1, &t2);
                            assert_bytes_eq(&tag, &o1, &o2);
                            if !e1 {
                                assert_bytes_eq(&format!("{tag} roundtrip"), &src, &o1);
                            }
                            // resetDStream then decode again (must forget nothing but session)
                            let tag = format!("resetDStream {shape:?} len={len} lvl={lvl}");
                            assert_eq!(c_rd(cd), r_rd(rd), "{tag}: reset rc");
                            let (o1, t1, _) = run_dstream(s.ds.0, cd, frame, ic, oc);
                            let (o2, t2, _) = run_dstream(s.ds.1, rd, frame, ic, oc);
                            cmp_trace(&tag, &t1, &t2);
                            assert_bytes_eq(&tag, &o1, &o2);
                            // plain initDStream on a dictless frame
                            assert_eq!(c_id(cd), r_id(rd), "initDStream rc");
                            (s.free_d.0)(cd);
                            (s.free_d.1)(rd);
                        }
                    }
                }
            }
        }
        cfc(cx);
        rfc(rx);
    }
}

// ============= CONFIGS: reset directives x streaming interaction ===========

#[test]
fn b_reset_directives() {
    let s = s();
    let mut rng = Rng::new(0x3333);
    // includes out-of-range directives (0, 4, -1) — the C treats them as no-ops
    let directives = [
        0,
        ZSTD_reset_session_only,
        ZSTD_reset_parameters,
        ZSTD_reset_session_and_parameters,
        4,
        -1,
    ];
    unsafe {
        for &d in &directives {
            for &shape in &[Shape::Text, Shape::Random] {
                let src = gen(shape, 30_000, &mut rng);
                let cc = (s.create_c.0)();
                let rc = (s.create_c.1)();
                // set a non-default parameter, then reset, then compress: whether
                // the parameter survives is exactly what the directive controls.
                assert_eq!(
                    (s.setp.0)(cc, ZSTD_c_compressionLevel, 17),
                    (s.setp.1)(rc, ZSTD_c_compressionLevel, 17)
                );
                assert_eq!(
                    (s.setp.0)(cc, ZSTD_c_checksumFlag, 1),
                    (s.setp.1)(rc, ZSTD_c_checksumFlag, 1)
                );
                let a = (s.reset_c.0)(cc, d);
                let b = (s.reset_c.1)(rc, d);
                assert_eq!(a, b, "CCtx_reset({d}) rc");
                assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "CCtx_reset({d}) ecode");
                let tag = format!("reset({d}) {shape:?}");
                let (o1, t1, e1) = run_cstream(s.cs2.0, cc, &src, 4096, 4096, ZSTD_e_end);
                let (o2, t2, e2) = run_cstream(s.cs2.1, rc, &src, 4096, 4096, ZSTD_e_end);
                assert_eq!(e1, e2, "{tag} errored");
                cmp_trace(&tag, &t1, &t2);
                assert_bytes_eq(&tag, &o1, &o2);

                // mid-stream reset: start a stream, then reset while dirty
                let mut ib = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void,
                    size: 100,
                    pos: 0,
                };
                let mut ib2 = ib;
                let mut buf1 = vec![0xAAu8; 1 << 16];
                let mut buf2 = vec![0xAAu8; 1 << 16];
                let mut ob = ZSTD_outBuffer {
                    dst: buf1.as_mut_ptr() as *mut c_void,
                    size: buf1.len(),
                    pos: 0,
                };
                let mut ob2 = ZSTD_outBuffer {
                    dst: buf2.as_mut_ptr() as *mut c_void,
                    size: buf2.len(),
                    pos: 0,
                };
                let x = (s.cs2.0)(cc, &mut ob, &mut ib, ZSTD_e_continue);
                let y = (s.cs2.1)(rc, &mut ob2, &mut ib2, ZSTD_e_continue);
                assert_eq!((x, ib.pos, ob.pos), (y, ib2.pos, ob2.pos), "{tag}: mid-stream step");
                let a = (s.reset_c.0)(cc, d);
                let b = (s.reset_c.1)(rc, d);
                assert_eq!(a, b, "{tag}: mid-stream reset rc");
                assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "{tag}: mid-stream reset ecode");
                (s.free_c.0)(cc);
                (s.free_c.1)(rc);

                // same for DCtx
                let cd = (s.create_d.0)();
                let rd = (s.create_d.1)();
                assert_eq!(
                    (s.setdp.0)(cd, ZSTD_d_windowLogMax, 27),
                    (s.setdp.1)(rd, ZSTD_d_windowLogMax, 27)
                );
                let a = (s.reset_d.0)(cd, d);
                let b = (s.reset_d.1)(rd, d);
                assert_eq!(a, b, "DCtx_reset({d}) rc");
                assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "DCtx_reset({d}) ecode");
                let mut g1: c_int = 0;
                let mut g2: c_int = 0;
                assert_eq!(
                    (s.getdp.0)(cd, ZSTD_d_windowLogMax, &mut g1),
                    (s.getdp.1)(rd, ZSTD_d_windowLogMax, &mut g2)
                );
                assert_eq!(g1, g2, "DCtx windowLogMax after reset({d})");
                (s.free_d.0)(cd);
                (s.free_d.1)(rd);
            }
        }
    }
}

// ==================== CONFIGS: decompression parameters ====================

#[test]
fn b_decompression_parameters() {
    let s = s();
    let (c_dec, r_dec) = fnpair!("ZSTD_decompressDCtx", FnCCtxCompressLike);
    type FnCCtxCompressLike =
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
    let (cc2, rc2) = fnpair!("ZSTD_compress2", FnCCtxCompressLike);
    let (ccc, rcc) = fnpair!("ZSTD_createCCtx", FnCreate);
    let (cfc, rfc) = fnpair!("ZSTD_freeCCtx", FnFree);
    let (c_sf, r_sf) = fnpair!("ZSTD_DCtx_setFormat", FnSetParamI);
    let (c_sw, r_sw) = fnpair!("ZSTD_DCtx_setMaxWindowSize", FnSizeSize2);
    type FnSetParamI = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
    type FnSizeSize2 = unsafe extern "C" fn(*mut c_void, size_t) -> size_t;

    let mut rng = Rng::new(0x4444);
    unsafe {
        let cx = ccc();
        let rx = rcc();
        for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive] {
            for &len in &[0usize, 1, 3000, 200_000] {
                let src = gen(shape, len, &mut rng);
                for &(fmt, ck) in &[
                    (ZSTD_f_zstd1, 0),
                    (ZSTD_f_zstd1, 1),
                    (ZSTD_f_zstd1_magicless, 0),
                    (ZSTD_f_zstd1_magicless, 1),
                ] {
                    for wl in [10, 20, 27] {
                        assert_eq!((s.reset_c.0)(cx, 3), (s.reset_c.1)(rx, 3));
                        for &(p, v) in &[
                            (ZSTD_c_format, fmt),
                            (ZSTD_c_checksumFlag, ck),
                            (ZSTD_c_windowLog, wl),
                        ] {
                            assert_eq!((s.setp.0)(cx, p, v), (s.setp.1)(rx, p, v), "set {p}={v}");
                        }
                        let cap = (s.bound.0)(len).max(64);
                        let mut f1 = vec![0u8; cap];
                        let mut f2 = vec![0u8; cap];
                        let n1 = cc2(cx, f1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        let n2 = rc2(rx, f2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        assert_eq!(n1, n2, "frame {shape:?} len={len} fmt={fmt} ck={ck} wl={wl}");
                        if (s.is_err.0)(n1) != 0 {
                            continue;
                        }
                        assert_bytes_eq("frame", &f1[..n1], &f2[..n2]);
                        let frame = &f1[..n1];

                        for dwl in [0, 10, 20, 27, 31] {
                            for ign in [ZSTD_d_validateChecksum, ZSTD_d_ignoreChecksum] {
                                for sob in [0, 1] {
                                    for dha in [0, 1] {
                                        for mbs in [0, 1024, 131_072] {
                                            let cd = (s.create_d.0)();
                                            let rd = (s.create_d.1)();
                                            let params = [
                                                (ZSTD_d_format, fmt),
                                                (ZSTD_d_windowLogMax, dwl),
                                                (ZSTD_d_forceIgnoreChecksum, ign),
                                                (ZSTD_d_stableOutBuffer, sob),
                                                (ZSTD_d_disableHuffmanAssembly, dha),
                                                (ZSTD_d_maxBlockSize, mbs),
                                            ];
                                            let mut ok = true;
                                            for &(p, v) in &params {
                                                let a = (s.setdp.0)(cd, p, v);
                                                let b = (s.setdp.1)(rd, p, v);
                                                assert_eq!(a, b, "DCtx set {p}={v}");
                                                assert_eq!((s.ecode.0)(a), (s.ecode.1)(b));
                                                if (s.is_err.0)(a) != 0 {
                                                    ok = false;
                                                }
                                                let mut g1 = 0;
                                                let mut g2 = 0;
                                                assert_eq!(
                                                    (s.getdp.0)(cd, p, &mut g1),
                                                    (s.getdp.1)(rd, p, &mut g2),
                                                    "DCtx get {p} rc"
                                                );
                                                assert_eq!(g1, g2, "DCtx get {p} value");
                                            }
                                            if ok {
                                                let tag = format!("dparam fmt={fmt} ck={ck} wl={wl} dwl={dwl} ign={ign} sob={sob} dha={dha} mbs={mbs} {shape:?} len={len}");
                                                let mut o1 = vec![0xAAu8; len + 64];
                                                let mut o2 = vec![0xAAu8; len + 64];
                                                let a = c_dec(
                                                    cd,
                                                    o1.as_mut_ptr() as *mut c_void,
                                                    o1.len(),
                                                    frame.as_ptr() as *const c_void,
                                                    frame.len(),
                                                );
                                                let b = r_dec(
                                                    rd,
                                                    o2.as_mut_ptr() as *mut c_void,
                                                    o2.len(),
                                                    frame.as_ptr() as *const c_void,
                                                    frame.len(),
                                                );
                                                assert_eq!(a, b, "{tag}: decompressDCtx rc");
                                                assert_eq!(
                                                    (s.ecode.0)(a),
                                                    (s.ecode.1)(b),
                                                    "{tag}: ecode"
                                                );
                                                assert_bytes_eq(&tag, &o1, &o2);
                                            }
                                            (s.free_d.0)(cd);
                                            (s.free_d.1)(rd);
                                        }
                                    }
                                }
                            }
                        }

                        // deprecated setters must behave the same
                        let cd = (s.create_d.0)();
                        let rd = (s.create_d.1)();
                        for f in [ZSTD_f_zstd1, ZSTD_f_zstd1_magicless, 2, -1] {
                            let a = c_sf(cd, f);
                            let b = r_sf(rd, f);
                            assert_eq!(a, b, "DCtx_setFormat({f})");
                            assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "DCtx_setFormat({f}) ecode");
                        }
                        for w in [0usize, 1024, 1 << 27, usize::MAX] {
                            let a = c_sw(cd, w);
                            let b = r_sw(rd, w);
                            assert_eq!(a, b, "DCtx_setMaxWindowSize({w})");
                            assert_eq!((s.ecode.0)(a), (s.ecode.1)(b));
                        }
                        (s.free_d.0)(cd);
                        (s.free_d.1)(rd);
                    }
                }
            }
        }
        cfc(cx);
        rfc(rx);
    }
}

// ================= CONFIGS: stableInBuffer / stableOutBuffer ==============

#[test]
fn b_stable_buffers() {
    let s = s();
    let mut rng = Rng::new(0x5555);
    unsafe {
        for sib in [0, 1] {
            for sob in [0, 1] {
                for &shape in &[Shape::Text, Shape::Random] {
                    for &len in &[0usize, 1, 5000, 150_000] {
                        let src = gen(shape, len, &mut rng);
                        let cc = (s.create_c.0)();
                        let rc = (s.create_c.1)();
                        for &(p, v) in &[
                            (ZSTD_c_stableInBuffer, sib),
                            (ZSTD_c_stableOutBuffer, sob),
                        ] {
                            assert_eq!((s.setp.0)(cc, p, v), (s.setp.1)(rc, p, v));
                        }
                        // stable buffers require the *same* buffers each call:
                        // present the whole input and a compressBound-sized output.
                        let cap = (s.bound.0)(len).max(64);
                        let mut b1 = vec![0xAAu8; cap];
                        let mut b2 = vec![0xAAu8; cap];
                        let mut t1 = Vec::new();
                        let mut t2 = Vec::new();
                        let mut ib1 = ZSTD_inBuffer {
                            src: src.as_ptr() as *const c_void,
                            size: len,
                            pos: 0,
                        };
                        let mut ib2 = ib1;
                        let mut ob1 = ZSTD_outBuffer {
                            dst: b1.as_mut_ptr() as *mut c_void,
                            size: cap,
                            pos: 0,
                        };
                        let mut ob2 = ZSTD_outBuffer {
                            dst: b2.as_mut_ptr() as *mut c_void,
                            size: cap,
                            pos: 0,
                        };
                        loop {
                            let a = (s.cs2.0)(cc, &mut ob1, &mut ib1, ZSTD_e_end);
                            let b = (s.cs2.1)(rc, &mut ob2, &mut ib2, ZSTD_e_end);
                            t1.push((a, ib1.pos, ob1.pos));
                            t2.push((b, ib2.pos, ob2.pos));
                            if (0usize.wrapping_sub(a)) <= 120 || a == 0 {
                                break;
                            }
                        }
                        let tag = format!("stable sib={sib} sob={sob} {shape:?} len={len}");
                        cmp_trace(&tag, &t1, &t2);
                        assert_bytes_eq(&tag, &b1, &b2);
                        (s.free_c.0)(cc);
                        (s.free_c.1)(rc);
                    }
                }
            }
        }
    }
}

// ================== CONFIGS: *_simpleArgs streaming shims =================

#[test]
fn b_simple_args() {
    let s = s();
    let (c_cs, r_cs) = fnpair!("ZSTD_compressStream2_simpleArgs", FnSimpleArgs);
    let (c_ds, r_ds) = fnpair!("ZSTD_decompressStream_simpleArgs", FnDSimpleArgs);
    let (ccc, rcc) = fnpair!("ZSTD_createCCtx", FnCreate);
    let (cfc, rfc) = fnpair!("ZSTD_freeCCtx", FnFree);
    let (cdc, rdc) = fnpair!("ZSTD_createDCtx", FnCreate);
    let (cfd, rfd) = fnpair!("ZSTD_freeDCtx", FnFree);
    let mut rng = Rng::new(0x6666);
    unsafe {
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 1000, 140_000] {
                let src = gen(shape, len, &mut rng);
                for endop in [ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end] {
                    for &ocap in &[1usize, 64, 1 << 18] {
                        let cx = ccc();
                        let rx = rcc();
                        let mut o1 = vec![0xAAu8; ocap];
                        let mut o2 = vec![0xAAu8; ocap];
                        let mut dp1: size_t = 0;
                        let mut dp2: size_t = 0;
                        let mut sp1: size_t = 0;
                        let mut sp2: size_t = 0;
                        let a = c_cs(
                            cx,
                            o1.as_mut_ptr() as *mut c_void,
                            ocap,
                            &mut dp1,
                            src.as_ptr() as *const c_void,
                            len,
                            &mut sp1,
                            endop,
                        );
                        let b = r_cs(
                            rx,
                            o2.as_mut_ptr() as *mut c_void,
                            ocap,
                            &mut dp2,
                            src.as_ptr() as *const c_void,
                            len,
                            &mut sp2,
                            endop,
                        );
                        let tag = format!("cs2_simpleArgs {shape:?} len={len} endop={endop} ocap={ocap}");
                        assert_eq!(a, b, "{tag}: rc");
                        assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "{tag}: ecode");
                        assert_eq!((dp1, sp1), (dp2, sp2), "{tag}: positions");
                        assert_bytes_eq(&tag, &o1, &o2);
                        cfc(cx);
                        rfc(rx);
                    }
                }

                // decompressStream_simpleArgs on a real frame
                let (cc2, rc2) = fnpair!(
                    "ZSTD_compress",
                    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t
                );
                let cap = (s.bound.0)(len).max(64);
                let mut f1 = vec![0u8; cap];
                let mut f2 = vec![0u8; cap];
                let n1 = cc2(f1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, 5);
                let n2 = rc2(f2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len, 5);
                assert_eq!(n1, n2);
                assert_bytes_eq("frame", &f1[..n1], &f2[..n2]);
                for &ocap in &[1usize, 64, len + 16] {
                    for &icap in &[1usize, 7, n1] {
                        let cx = cdc();
                        let rx = rdc();
                        let mut o1 = vec![0xAAu8; ocap.max(1)];
                        let mut o2 = vec![0xAAu8; ocap.max(1)];
                        let mut dp1: size_t = 0;
                        let mut dp2: size_t = 0;
                        let mut sp1: size_t = 0;
                        let mut sp2: size_t = 0;
                        let a = c_ds(
                            cx,
                            o1.as_mut_ptr() as *mut c_void,
                            o1.len(),
                            &mut dp1,
                            f1.as_ptr() as *const c_void,
                            icap.min(n1),
                            &mut sp1,
                        );
                        let b = r_ds(
                            rx,
                            o2.as_mut_ptr() as *mut c_void,
                            o2.len(),
                            &mut dp2,
                            f2.as_ptr() as *const c_void,
                            icap.min(n2),
                            &mut sp2,
                        );
                        let tag = format!("ds_simpleArgs {shape:?} len={len} ocap={ocap} icap={icap}");
                        assert_eq!(a, b, "{tag}: rc");
                        assert_eq!((s.ecode.0)(a), (s.ecode.1)(b), "{tag}: ecode");
                        assert_eq!((dp1, sp1), (dp2, sp2), "{tag}: positions");
                        assert_bytes_eq(&tag, &o1, &o2);
                        cfd(cx);
                        rfd(rx);
                    }
                }
            }
        }
    }
}

// ============= CONFIGS: createCCtx_advanced / custom allocators ============

#[test]
fn b_advanced_creators() {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ZSTD_customMem {
        alloc: Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>,
        free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
        opaque: *mut c_void,
    }
    unsafe extern "C" fn my_alloc(_o: *mut c_void, n: size_t) -> *mut c_void {
        // over-allocate a header so we can recover the layout on free
        let layout = std::alloc::Layout::from_size_align(n + 16, 16).unwrap();
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut usize).write(n + 16);
        p.add(16) as *mut c_void
    }
    unsafe extern "C" fn my_free(_o: *mut c_void, p: *mut c_void) {
        if p.is_null() {
            return;
        }
        let base = (p as *mut u8).sub(16);
        let n = (base as *mut usize).read();
        std::alloc::dealloc(base, std::alloc::Layout::from_size_align(n, 16).unwrap());
    }

    type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
    let s = s();
    let (c_ca, r_ca) = fnpair!("ZSTD_createCCtx_advanced", FnCreateAdv);
    let (c_da, r_da) = fnpair!("ZSTD_createDCtx_advanced", FnCreateAdv);
    let (c_csa, r_csa) = fnpair!("ZSTD_createCStream_advanced", FnCreateAdv);
    let (c_dsa, r_dsa) = fnpair!("ZSTD_createDStream_advanced", FnCreateAdv);
    let (c_fc, r_fc) = fnpair!("ZSTD_freeCCtx", FnFree);
    let (c_fd, r_fd) = fnpair!("ZSTD_freeDCtx", FnFree);
    let (cc2, rc2) = fnpair!(
        "ZSTD_compress2",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );
    let (c_dd, r_dd) = fnpair!(
        "ZSTD_decompressDCtx",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t
    );

    let null_mem = ZSTD_customMem {
        alloc: None,
        free: None,
        opaque: std::ptr::null_mut(),
    };
    let custom = ZSTD_customMem {
        alloc: Some(my_alloc),
        free: Some(my_free),
        opaque: std::ptr::null_mut(),
    };

    let mut rng = Rng::new(0x7777);
    unsafe {
        for mem in [null_mem, custom] {
            for &shape in &[Shape::Text, Shape::Random, Shape::Mixed] {
                for &len in &[0usize, 1, 4096, 200_000] {
                    let src = gen(shape, len, &mut rng);
                    for lvl in [1, 9, 19] {
                        let cx = c_ca(mem);
                        let rx = r_ca(mem);
                        assert_eq!(cx.is_null(), rx.is_null(), "createCCtx_advanced nullness");
                        assert_eq!(
                            (s.setp.0)(cx, ZSTD_c_compressionLevel, lvl),
                            (s.setp.1)(rx, ZSTD_c_compressionLevel, lvl)
                        );
                        let cap = (s.bound.0)(len).max(64);
                        let mut f1 = vec![0xAAu8; cap];
                        let mut f2 = vec![0xAAu8; cap];
                        let n1 = cc2(cx, f1.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        let n2 = rc2(rx, f2.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len);
                        let tag = format!("adv-alloc {shape:?} len={len} lvl={lvl}");
                        assert_eq!(n1, n2, "{tag}: size");
                        assert_bytes_eq(&tag, &f1, &f2);
                        c_fc(cx);
                        r_fc(rx);

                        let dx = c_da(mem);
                        let dy = r_da(mem);
                        assert_eq!(dx.is_null(), dy.is_null(), "createDCtx_advanced nullness");
                        let mut o1 = vec![0xAAu8; len + 8];
                        let mut o2 = vec![0xAAu8; len + 8];
                        let a = c_dd(dx, o1.as_mut_ptr() as *mut c_void, o1.len(), f1.as_ptr() as *const c_void, n1);
                        let b = r_dd(dy, o2.as_mut_ptr() as *mut c_void, o2.len(), f2.as_ptr() as *const c_void, n2);
                        assert_eq!(a, b, "{tag}: decompress rc");
                        assert_bytes_eq(&format!("{tag}: decompressed"), &o1, &o2);
                        c_fd(dx);
                        r_fd(dy);
                    }
                }
            }
            // stream variants must at least create/free identically
            let a = c_csa(mem);
            let b = r_csa(mem);
            assert_eq!(a.is_null(), b.is_null(), "createCStream_advanced");
            (s.free_c.0)(a);
            (s.free_c.1)(b);
            let a = c_dsa(mem);
            let b = r_dsa(mem);
            assert_eq!(a.is_null(), b.is_null(), "createDStream_advanced");
            (s.free_d.0)(a);
            (s.free_d.1)(b);
        }
    }
}
