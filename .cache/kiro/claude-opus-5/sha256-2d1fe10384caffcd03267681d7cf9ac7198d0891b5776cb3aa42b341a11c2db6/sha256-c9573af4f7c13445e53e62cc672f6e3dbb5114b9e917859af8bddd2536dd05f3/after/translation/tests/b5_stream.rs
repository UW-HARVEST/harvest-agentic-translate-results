//! Phase B, CONFIGS.md rows 33–44: the streaming (L1) API driven the way a
//! real consumer drives it — chunked input, chunked output, every
//! `ZSTD_EndDirective`, and the `stableIn/OutBuffer` modes.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCStream2 =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer, c_int) -> size_t;
type FnStream2 = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> size_t;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZSTD_frameProgression {
    ingested: c_ulonglong,
    consumed: c_ulonglong,
    produced: c_ulonglong,
    flushed: c_ulonglong,
    currentJobID: std::os::raw::c_uint,
    nbActiveWorkers: std::os::raw::c_uint,
}
type FnProgression = unsafe extern "C" fn(*const c_void) -> ZSTD_frameProgression;

struct Str {
    e: Err2,
    cc: *mut c_void,
    rc: *mut c_void,
    cd: *mut c_void,
    rd: *mut c_void,
}

impl Str {
    fn new() -> Str {
        unsafe {
            let (a, b) = both::<FnVoidToPtr>("ZSTD_createCCtx");
            let (c, d) = both::<FnVoidToPtr>("ZSTD_createDCtx");
            Str { e: Err2::new(), cc: a(), rc: b(), cd: c(), rd: d() }
        }
    }
    fn reset(&self) {
        unsafe {
            let (a, b) = both::<FnReset>("ZSTD_CCtx_reset");
            a(self.cc, ZSTD_reset_session_and_parameters);
            b(self.rc, ZSTD_reset_session_and_parameters);
            let (c, d) = both::<FnReset>("ZSTD_DCtx_reset");
            c(self.cd, ZSTD_reset_session_and_parameters);
            d(self.rd, ZSTD_reset_session_and_parameters);
        }
    }
    #[track_caller]
    fn cset(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let (a, b) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
            let x = a(self.cc, id, v);
            let y = b(self.rc, id, v);
            self.e.eq_or_oom(&format!("{ctx}: CCtx_setParameter({id},{v})"), x, y);
            !self.e.c.is_err(x) && !self.e.r.is_err(y)
        }
    }
    #[track_caller]
    fn dset(&self, ctx: &str, id: c_int, v: c_int) -> bool {
        unsafe {
            let (a, b) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
            let x = a(self.cd, id, v);
            let y = b(self.rd, id, v);
            self.e.eq_or_oom(&format!("{ctx}: DCtx_setParameter({id},{v})"), x, y);
            !self.e.c.is_err(x) && !self.e.r.is_err(y)
        }
    }
}
impl Drop for Str {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrToSize>("ZSTD_freeCCtx");
            a(self.cc);
            b(self.rc);
            let (c, d) = both::<FnPtrToSize>("ZSTD_freeDCtx");
            c(self.cd);
            d(self.rd);
        }
    }
}

/// Drive `ZSTD_compressStream2` on both libraries in lock-step, asserting that
/// after EVERY call the return value, the input position, the output position
/// and the produced bytes are identical.
///
/// `end_every`: emit `endOp` = `flush` every N input chunks (0 = never), and
/// always finish with `ZSTD_e_end`.
#[track_caller]
fn drive_compress(
    s: &Str,
    ctx: &str,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    flush_every: usize,
    sample_progression: bool,
) -> Option<Vec<u8>> {
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (cpg, rpg) = both::<FnProgression>("ZSTD_getFrameProgression");
        let (ctf, rtf) = both::<unsafe extern "C" fn(*const c_void) -> size_t>("ZSTD_toFlushNow");

        let mut co: Vec<u8> = Vec::new();
        let mut ro: Vec<u8> = Vec::new();
        let mut cbuf = vec![0u8; out_chunk.max(1)];
        let mut rbuf = vec![0u8; out_chunk.max(1)];
        let mut cin_pos = 0usize;
        let mut rin_pos = 0usize;
        let mut step = 0usize;
        let mut chunk_idx = 0usize;

        loop {
            step += 1;
            assert!(step < 2_000_000, "{ctx}: runaway loop");
            let remaining = src.len() - cin_pos;
            let this_in = in_chunk.min(remaining);
            let last = cin_pos + this_in >= src.len();
            let endop = if last {
                ZSTD_e_end
            } else if flush_every != 0 && chunk_idx % flush_every == flush_every - 1 {
                ZSTD_e_flush
            } else {
                ZSTD_e_continue
            };
            chunk_idx += 1;

            let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
            let mut cib = ZSTD_inBuffer { src: sp, size: cin_pos + this_in, pos: cin_pos };
            let mut rib = ZSTD_inBuffer { src: sp, size: rin_pos + this_in, pos: rin_pos };
            let mut cob = ZSTD_outBuffer {
                dst: cbuf.as_mut_ptr() as *mut c_void,
                size: out_chunk,
                pos: 0,
            };
            let mut rob = ZSTD_outBuffer {
                dst: rbuf.as_mut_ptr() as *mut c_void,
                size: out_chunk,
                pos: 0,
            };
            let a = ccs(s.cc, &mut cob, &mut cib, endop);
            let b = rcs(s.rc, &mut rob, &mut rib, endop);
            let sctx = format!("{ctx} step={step} endop={endop}");
            if !s.e.eq_or_oom(&sctx, a, b) {
                return None;
            }
            if s.e.c.is_err(a) {
                return None;
            }
            assert_eq!(cib.pos, rib.pos, "{sctx}: input pos");
            assert_eq!(cob.pos, rob.pos, "{sctx}: output pos");
            assert_bytes_eq(&format!("{sctx}: emitted"), &cbuf[..cob.pos], &rbuf[..rob.pos]);
            co.extend_from_slice(&cbuf[..cob.pos]);
            ro.extend_from_slice(&rbuf[..rob.pos]);
            cin_pos = cib.pos;
            rin_pos = rib.pos;

            if sample_progression {
                assert_eq!(cpg(s.cc), rpg(s.rc), "{sctx}: getFrameProgression");
                assert_eq!(ctf(s.cc), rtf(s.rc), "{sctx}: toFlushNow");
            }

            if last && a == 0 && cob.pos < out_chunk {
                break;
            }
            if last && a == 0 {
                break;
            }
            // No progress and nothing left to feed -> the frame is stuck.
            if cob.pos == 0 && cin_pos == src.len() && !last {
                break;
            }
        }
        assert_bytes_eq(&format!("{ctx}: full frame"), &co, &ro);
        Some(co)
    }
}

/// Drive `ZSTD_decompressStream` on both libraries in lock-step.
#[track_caller]
fn drive_decompress(
    s: &Str,
    ctx: &str,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
) -> Option<Vec<u8>> {
    unsafe {
        let (cds, rds) = both::<FnStream2>("ZSTD_decompressStream");
        let mut co: Vec<u8> = Vec::new();
        let mut ro: Vec<u8> = Vec::new();
        let mut cbuf = vec![0u8; out_chunk.max(1)];
        let mut rbuf = vec![0u8; out_chunk.max(1)];
        let mut cpos = 0usize;
        let mut rpos = 0usize;
        let mut step = 0usize;
        loop {
            step += 1;
            assert!(step < 2_000_000, "{ctx}: runaway loop");
            let avail = (cpos + in_chunk).min(frame.len());
            let mut cib =
                ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: avail, pos: cpos };
            let mut rib =
                ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: avail, pos: rpos };
            let mut cob =
                ZSTD_outBuffer { dst: cbuf.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            let mut rob =
                ZSTD_outBuffer { dst: rbuf.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            let a = cds(s.cd, &mut cob, &mut cib);
            let b = rds(s.rd, &mut rob, &mut rib);
            let sctx = format!("{ctx} dstep={step}");
            if !s.e.eq_or_oom(&sctx, a, b) {
                return None;
            }
            if s.e.c.is_err(a) {
                return None;
            }
            assert_eq!(cib.pos, rib.pos, "{sctx}: input pos");
            assert_eq!(cob.pos, rob.pos, "{sctx}: output pos");
            assert_bytes_eq(&format!("{sctx}: emitted"), &cbuf[..cob.pos], &rbuf[..rob.pos]);
            co.extend_from_slice(&cbuf[..cob.pos]);
            ro.extend_from_slice(&rbuf[..rob.pos]);
            cpos = cib.pos;
            rpos = rib.pos;
            if a == 0 {
                break;
            }
            if cob.pos == 0 && cpos == frame.len() {
                break; // no more input available, no progress
            }
        }
        assert_bytes_eq(&format!("{ctx}: full plaintext"), &co, &ro);
        Some(co)
    }
}

fn chunk_sizes() -> Vec<usize> {
    unsafe {
        let (cis, _) = both::<FnVoidToSize>("ZSTD_CStreamInSize");
        let (cos, _) = both::<FnVoidToSize>("ZSTD_CStreamOutSize");
        vec![1, 7, 64, 1024, cis(), cos()]
    }
}

/// CONFIGS row 35 + 44: `compressStream2` over the endOp × chunk-size matrix,
/// sampling `getFrameProgression` / `toFlushNow` at every step.
#[test]
fn compress_stream2_chunk_matrix() {
    let s = Str::new();
    let mut rng = Rng::new(0xB501);
    let chunks = chunk_sizes();
    for &shape in ALL_SHAPES {
        for &len in &[0usize, 1, 100, 5000, 70_000] {
            let src = gen(shape, len, &mut rng);
            for &ic in &chunks {
                for &oc in &chunks {
                    for &fe in &[0usize, 1, 3] {
                        s.reset();
                        let ctx = format!(
                            "shape={shape:?} len={} ic={ic} oc={oc} flushEvery={fe}",
                            src.len()
                        );
                        let frame =
                            match drive_compress(&s, &ctx, &src, ic, oc, fe, true) {
                                Some(f) => f,
                                None => continue,
                            };
                        // decode it back with each library
                        s.reset();
                        if let Some(pt) = drive_decompress(&s, &ctx, &frame, oc.max(1), 4096) {
                            assert_bytes_eq(&format!("{ctx}: round-trip"), &pt, &src);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS row 36: `stableInBuffer` × `stableOutBuffer`, whole-input call.
#[test]
fn stable_buffers() {
    let s = Str::new();
    let mut rng = Rng::new(0xB502);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        for si in [0i32, 1] {
            for so in [0i32, 1] {
                for &shape in ALL_SHAPES {
                    for &len in &[0usize, 1, 1024, 70_000] {
                        let src = gen(shape, len, &mut rng);
                        s.reset();
                        let ctx =
                            format!("stableIn={si} stableOut={so} shape={shape:?} len={}", src.len());
                        if !s.cset(&ctx, ZSTD_c_stableInBuffer, si) { continue; }
                        if !s.cset(&ctx, ZSTD_c_stableOutBuffer, so) { continue; }
                        let cap = bnd(src.len()) + 64;
                        let mut o1 = vec![0u8; cap];
                        let mut o2 = vec![0u8; cap];
                        let sp = if src.is_empty() {
                            std::ptr::null()
                        } else {
                            src.as_ptr() as *const c_void
                        };
                        let mut cib = ZSTD_inBuffer { src: sp, size: src.len(), pos: 0 };
                        let mut rib = cib;
                        let mut cob = ZSTD_outBuffer {
                            dst: o1.as_mut_ptr() as *mut c_void, size: cap, pos: 0,
                        };
                        let mut rob = ZSTD_outBuffer {
                            dst: o2.as_mut_ptr() as *mut c_void, size: cap, pos: 0,
                        };
                        let a = ccs(s.cc, &mut cob, &mut cib, ZSTD_e_end);
                        let b = rcs(s.rc, &mut rob, &mut rib, ZSTD_e_end);
                        if !s.e.eq_or_oom(&ctx, a, b) { continue; }
                        assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                        assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
                        assert_bytes_eq(&ctx, &o1[..cob.pos], &o2[..rob.pos]);
                    }
                }
            }
        }
    }
}

/// CONFIGS row 37: the legacy `compressStream` + `flushStream` + `endStream`
/// triple.
#[test]
fn legacy_compress_stream_triple() {
    let s = Str::new();
    let mut rng = Rng::new(0xB503);
    unsafe {
        let (cics, rics) = both::<FnInitCStream>("ZSTD_initCStream");
        let (ccs, rcs) = both::<FnStream2>("ZSTD_compressStream");
        let (cfs, rfs) = both::<FnFlush>("ZSTD_flushStream");
        let (ces, res) = both::<FnFlush>("ZSTD_endStream");
        let chunks = chunk_sizes();
        for lvl in [-5i32, 1, 3, 9, 19, 22] {
            for &shape in ALL_SHAPES {
                for &len in &[0usize, 1, 1000, 40_000] {
                    let src = gen(shape, len, &mut rng);
                    for &ic in &chunks {
                        for &oc in &[7usize, 1024, 131_075] {
                            s.reset();
                            let ctx = format!(
                                "lvl={lvl} shape={shape:?} len={} ic={ic} oc={oc}",
                                src.len()
                            );
                            s.e.eq(&format!("{ctx}: initCStream"), cics(s.cc, lvl), rics(s.rc, lvl));
                            let mut co = Vec::new();
                            let mut ro = Vec::new();
                            let mut cbuf = vec![0u8; oc];
                            let mut rbuf = vec![0u8; oc];
                            let mut pos = 0usize;
                            let mut ok = true;
                            while pos < src.len() {
                                let end = (pos + ic).min(src.len());
                                let mut cib = ZSTD_inBuffer {
                                    src: src.as_ptr() as *const c_void, size: end, pos,
                                };
                                let mut rib = cib;
                                let mut cob = ZSTD_outBuffer {
                                    dst: cbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let mut rob = ZSTD_outBuffer {
                                    dst: rbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let a = ccs(s.cc, &mut cob, &mut cib);
                                let b = rcs(s.rc, &mut rob, &mut rib);
                                if !s.e.eq_or_oom(&format!("{ctx}: compressStream"), a, b) {
                                    ok = false;
                                    break;
                                }
                                assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                                assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
                                assert_bytes_eq(
                                    &format!("{ctx}: compressStream out"),
                                    &cbuf[..cob.pos], &rbuf[..rob.pos],
                                );
                                co.extend_from_slice(&cbuf[..cob.pos]);
                                ro.extend_from_slice(&rbuf[..rob.pos]);
                                assert!(
                                    cib.pos > pos || cob.pos > 0,
                                    "{ctx}: compressStream made no progress at pos={pos}"
                                );
                                pos = cib.pos;
                            }
                            if !ok { continue; }
                            // flushStream to completion
                            let mut flushed = false;
                            for _ in 0..2_000_000 {
                                let mut cob = ZSTD_outBuffer {
                                    dst: cbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let mut rob = ZSTD_outBuffer {
                                    dst: rbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let a = cfs(s.cc, &mut cob);
                                let b = rfs(s.rc, &mut rob);
                                if !s.e.eq_or_oom(&format!("{ctx}: flushStream"), a, b) {
                                    ok = false;
                                    break;
                                }
                                assert_eq!(cob.pos, rob.pos, "{ctx}: flush out pos");
                                assert_bytes_eq(
                                    &format!("{ctx}: flushStream out"),
                                    &cbuf[..cob.pos], &rbuf[..rob.pos],
                                );
                                co.extend_from_slice(&cbuf[..cob.pos]);
                                ro.extend_from_slice(&rbuf[..rob.pos]);
                                if a == 0 { flushed = true; break; }
                            }
                            if !ok { continue; }
                            assert!(flushed, "{ctx}: flushStream never completed");
                            // endStream to completion
                            let mut ended = false;
                            for _ in 0..2_000_000 {
                                let mut cob = ZSTD_outBuffer {
                                    dst: cbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let mut rob = ZSTD_outBuffer {
                                    dst: rbuf.as_mut_ptr() as *mut c_void, size: oc, pos: 0,
                                };
                                let a = ces(s.cc, &mut cob);
                                let b = res(s.rc, &mut rob);
                                if !s.e.eq_or_oom(&format!("{ctx}: endStream"), a, b) {
                                    ok = false;
                                    break;
                                }
                                assert_eq!(cob.pos, rob.pos, "{ctx}: end out pos");
                                assert_bytes_eq(
                                    &format!("{ctx}: endStream out"),
                                    &cbuf[..cob.pos], &rbuf[..rob.pos],
                                );
                                co.extend_from_slice(&cbuf[..cob.pos]);
                                ro.extend_from_slice(&rbuf[..rob.pos]);
                                if a == 0 { ended = true; break; }
                            }
                            if !ok { continue; }
                            assert!(ended, "{ctx}: endStream never completed");
                            assert_bytes_eq(&format!("{ctx}: full frame"), &co, &ro);
                            // it must decode back to src
                            s.reset();
                            if let Some(pt) = drive_decompress(&s, &ctx, &co, 4096, 4096) {
                                assert_bytes_eq(&format!("{ctx}: round-trip"), &pt, &src);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS row 38 + 41: the `initCStream*` / `initDStream*` families.
#[test]
fn init_stream_families() {
    let s = Str::new();
    let mut rng = Rng::new(0xB504);
    unsafe {
        type FnInitSrcSize = unsafe extern "C" fn(*mut c_void, c_int, c_ulonglong) -> size_t;
        type FnInitUsingDict =
            unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
        type FnInitAdvanced = unsafe extern "C" fn(
            *mut c_void, *const c_void, size_t, ZSTD_parameters, c_ulonglong,
        ) -> size_t;
        type FnInitDStreamDict =
            unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
        let (cics, rics) = both::<FnInitCStream>("ZSTD_initCStream");
        let (cicss, ricss) = both::<FnInitSrcSize>("ZSTD_initCStream_srcSize");
        let (cicud, ricud) = both::<FnInitUsingDict>("ZSTD_initCStream_usingDict");
        let (cica, rica) = both::<FnInitAdvanced>("ZSTD_initCStream_advanced");
        let (cids, rids) = both::<FnPtrToSize>("ZSTD_initDStream");
        let (cidud, ridud) = both::<FnInitDStreamDict>("ZSTD_initDStream_usingDict");
        let (crds, rrds) = both::<FnPtrToSize>("ZSTD_resetDStream");
        let (cgp, _) = both::<unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters>(
            "ZSTD_getParams",
        );

        let dict: Vec<u8> = gen(Shape::Text, 4096, &mut rng);
        for lvl in [-5i32, 1, 3, 19] {
            for &shape in ALL_SHAPES {
                let src = gen(shape, 30_000, &mut rng);
                for variant in 0..4 {
                    s.reset();
                    let ctx = format!("initCStream variant={variant} lvl={lvl} shape={shape:?}");
                    let (a, b) = match variant {
                        0 => (cics(s.cc, lvl), rics(s.rc, lvl)),
                        1 => (
                            cicss(s.cc, lvl, src.len() as c_ulonglong),
                            ricss(s.rc, lvl, src.len() as c_ulonglong),
                        ),
                        2 => (
                            cicud(s.cc, dict.as_ptr() as *const c_void, dict.len(), lvl),
                            ricud(s.rc, dict.as_ptr() as *const c_void, dict.len(), lvl),
                        ),
                        _ => {
                            let p = cgp(lvl, src.len() as c_ulonglong, dict.len());
                            (
                                cica(s.cc, dict.as_ptr() as *const c_void, dict.len(), p,
                                     ZSTD_CONTENTSIZE_UNKNOWN),
                                rica(s.rc, dict.as_ptr() as *const c_void, dict.len(), p,
                                     ZSTD_CONTENTSIZE_UNKNOWN),
                            )
                        }
                    };
                    if !s.e.eq_or_oom(&ctx, a, b) { continue; }
                    if s.e.c.is_err(a) { continue; }
                    let frame = match drive_compress(&s, &ctx, &src, 4096, 4096, 0, false) {
                        Some(f) => f,
                        None => continue,
                    };
                    // decode with the matching initDStream variant
                    for dv in 0..3 {
                        let dctx = format!("{ctx} dvariant={dv}");
                        let (x, y) = match dv {
                            0 => (cids(s.cd), rids(s.rd)),
                            1 => (
                                cidud(s.cd, dict.as_ptr() as *const c_void, dict.len()),
                                ridud(s.rd, dict.as_ptr() as *const c_void, dict.len()),
                            ),
                            _ => (crds(s.cd), rrds(s.rd)),
                        };
                        if !s.e.eq_or_oom(&dctx, x, y) { continue; }
                        drive_decompress(&s, &dctx, &frame, 4096, 4096);
                    }
                }
            }
        }
    }
}

/// CONFIGS row 39: `decompressStream` over the in/out chunk matrix, using
/// frames from several configurations.
#[test]
fn decompress_stream_chunk_matrix() {
    let s = Str::new();
    let mut rng = Rng::new(0xB505);
    unsafe {
        let (cc, _) = both::<unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t>("ZSTD_compress");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let (dis, _) = both::<FnVoidToSize>("ZSTD_DStreamInSize");
        let (dos, _) = both::<FnVoidToSize>("ZSTD_DStreamOutSize");
        let ins = [1usize, 7, 64, 1024, dis()];
        let outs = [1usize, 7, 64, 1024, dos()];
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 1000, 70_000] {
                let src = gen(shape, len, &mut rng);
                for lvl in [1i32, 9, 19] {
                    let cap = bnd(src.len()) + 64;
                    let mut fr = vec![0u8; cap];
                    let sp = if src.is_empty() {
                        std::ptr::null()
                    } else {
                        src.as_ptr() as *const c_void
                    };
                    let n = cc(fr.as_mut_ptr() as *mut c_void, cap, sp, src.len(), lvl);
                    if s.e.c.is_err(n) { continue; }
                    fr.truncate(n);
                    for &ic in &ins {
                        for &oc in &outs {
                            s.reset();
                            let ctx = format!(
                                "shape={shape:?} len={} lvl={lvl} ic={ic} oc={oc}",
                                src.len()
                            );
                            if let Some(pt) = drive_decompress(&s, &ctx, &fr, ic, oc) {
                                assert_bytes_eq(&format!("{ctx}: plaintext"), &pt, &src);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS rows 40, 42, 43: `d_stableOutBuffer`, `d_windowLogMax`,
/// `d_forceIgnoreChecksum`.
#[test]
fn decompress_params() {
    let s = Str::new();
    let mut rng = Rng::new(0xB506);
    unsafe {
        let (cds, rds) = both::<FnStream2>("ZSTD_decompressStream");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let (ccs, _) = both::<FnCStream2>("ZSTD_compressStream2");
        let (csp, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (crst, _) = both::<FnReset>("ZSTD_CCtx_reset");

        // Build frames with a range of windowLogs and with/without checksum,
        // using the C library (identity of the frame bytes is already proven
        // elsewhere).
        let src = gen(Shape::Text, 200_000, &mut rng);
        let mut frames: Vec<(String, Vec<u8>)> = Vec::new();
        for wl in [10i32, 17, 20, 27, 31] {
            for ck in [0i32, 1] {
                crst(s.cc, ZSTD_reset_session_and_parameters);
                if csp(s.cc, ZSTD_c_windowLog, wl) > usize::MAX / 2 { continue; }
                csp(s.cc, ZSTD_c_checksumFlag, ck);
                let cap = bnd(src.len()) + 64;
                let mut out = vec![0u8; cap];
                let mut ib = ZSTD_inBuffer {
                    src: src.as_ptr() as *const c_void, size: src.len(), pos: 0,
                };
                let mut ob = ZSTD_outBuffer {
                    dst: out.as_mut_ptr() as *mut c_void, size: cap, pos: 0,
                };
                let r = ccs(s.cc, &mut ob, &mut ib, ZSTD_e_end);
                if s.e.c.is_err(r) || r != 0 { continue; }
                out.truncate(ob.pos);
                frames.push((format!("wl={wl} ck={ck}"), out));
            }
        }
        assert!(frames.len() >= 6, "expected several frames, got {}", frames.len());

        // row 42: windowLogMax
        for (fname, fr) in &frames {
            for wlm in [10i32, 17, 20, 27, 31] {
                s.reset();
                let ctx = format!("{fname} windowLogMax={wlm}");
                if !s.dset(&ctx, ZSTD_d_windowLogMax, wlm) { continue; }
                drive_decompress(&s, &ctx, fr, 4096, 4096);
            }
        }
        // row 43: forceIgnoreChecksum, on intact and on checksum-corrupted frames
        for (fname, fr) in &frames {
            for fic in [0i32, 1] {
                for corrupt in [false, true] {
                    let mut f = fr.clone();
                    if corrupt {
                        let n = f.len();
                        f[n - 1] ^= 0xFF;
                    }
                    s.reset();
                    let ctx = format!("{fname} forceIgnoreChecksum={fic} corrupt={corrupt}");
                    if !s.dset(&ctx, ZSTD_d_windowLogMax, 31) { continue; }
                    if !s.dset(&ctx, ZSTD_d_forceIgnoreChecksum, fic) { continue; }
                    drive_decompress(&s, &ctx, &f, 4096, 4096);
                }
            }
        }
        // row 40: d_stableOutBuffer with a whole-output buffer
        for (fname, fr) in &frames {
            for sob in [0i32, 1] {
                s.reset();
                let ctx = format!("{fname} d_stableOutBuffer={sob}");
                if !s.dset(&ctx, ZSTD_d_windowLogMax, 31) { continue; }
                if !s.dset(&ctx, ZSTD_d_stableOutBuffer, sob) { continue; }
                let mut o1 = vec![0u8; src.len() + 64];
                let mut o2 = vec![0u8; src.len() + 64];
                let mut cib = ZSTD_inBuffer {
                    src: fr.as_ptr() as *const c_void, size: fr.len(), pos: 0,
                };
                let mut rib = cib;
                let mut cob = ZSTD_outBuffer {
                    dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0,
                };
                let mut rob = ZSTD_outBuffer {
                    dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0,
                };
                let a = cds(s.cd, &mut cob, &mut cib);
                let b = rds(s.rd, &mut rob, &mut rib);
                if !s.e.eq_or_oom(&ctx, a, b) { continue; }
                assert_eq!(cob.pos, rob.pos, "{ctx}: out pos");
                assert_eq!(cib.pos, rib.pos, "{ctx}: in pos");
                assert_bytes_eq(&ctx, &o1[..cob.pos], &o2[..rob.pos]);
            }
        }
    }
}

/// CONFIGS rows 33–34: the three reset directives applied at each stage of a
/// stream.
#[test]
fn reset_directives_mid_stream() {
    let s = Str::new();
    let mut rng = Rng::new(0xB507);
    unsafe {
        let (ccs, rcs) = both::<FnCStream2>("ZSTD_compressStream2");
        let (crst, rrst) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cdrst, rdrst) = both::<FnReset>("ZSTD_DCtx_reset");
        let (cds, rds) = both::<FnStream2>("ZSTD_decompressStream");
        let src = gen(Shape::Text, 80_000, &mut rng);
        for directive in [1i32, 2, 3] {
            for after_steps in [0usize, 1, 2, 5] {
                s.reset();
                let ctx = format!("reset={directive} afterSteps={after_steps}");
                s.cset(&ctx, ZSTD_c_compressionLevel, 5);
                let mut cbuf = vec![0u8; 1024];
                let mut rbuf = vec![0u8; 1024];
                let mut pos = 0usize;
                for _ in 0..after_steps {
                    let end = (pos + 8192).min(src.len());
                    let mut cib = ZSTD_inBuffer {
                        src: src.as_ptr() as *const c_void, size: end, pos,
                    };
                    let mut rib = cib;
                    let mut cob = ZSTD_outBuffer {
                        dst: cbuf.as_mut_ptr() as *mut c_void, size: cbuf.len(), pos: 0,
                    };
                    let mut rob = ZSTD_outBuffer {
                        dst: rbuf.as_mut_ptr() as *mut c_void, size: rbuf.len(), pos: 0,
                    };
                    let a = ccs(s.cc, &mut cob, &mut cib, ZSTD_e_continue);
                    let b = rcs(s.rc, &mut rob, &mut rib, ZSTD_e_continue);
                    s.e.eq(&format!("{ctx}: warm-up"), a, b);
                    assert_bytes_eq(&format!("{ctx}: warm-up out"),
                                    &cbuf[..cob.pos], &rbuf[..rob.pos]);
                    pos = cib.pos;
                }
                s.e.eq(&format!("{ctx}: CCtx_reset"),
                       crst(s.cc, directive), rrst(s.rc, directive));
                // and after the reset, a fresh full compression must still match
                let frame = drive_compress(&s, &format!("{ctx}: post-reset"), &src, 8192, 4096, 0, false);
                if let Some(f) = frame {
                    // DCtx reset mid-stream
                    s.reset();
                    let mut o1 = vec![0u8; 4096];
                    let mut o2 = vec![0u8; 4096];
                    let mut cib = ZSTD_inBuffer {
                        src: f.as_ptr() as *const c_void, size: f.len() / 2, pos: 0,
                    };
                    let mut rib = cib;
                    let mut cob = ZSTD_outBuffer {
                        dst: o1.as_mut_ptr() as *mut c_void, size: o1.len(), pos: 0,
                    };
                    let mut rob = ZSTD_outBuffer {
                        dst: o2.as_mut_ptr() as *mut c_void, size: o2.len(), pos: 0,
                    };
                    let a = cds(s.cd, &mut cob, &mut cib);
                    let b = rds(s.rd, &mut rob, &mut rib);
                    s.e.eq(&format!("{ctx}: dstream warm-up"), a, b);
                    s.e.eq(&format!("{ctx}: DCtx_reset"),
                           cdrst(s.cd, directive), rdrst(s.rd, directive));
                    drive_decompress(&s, &format!("{ctx}: post-dreset"), &f, 4096, 4096);
                }
            }
        }
    }
}

/// Randomized streaming property sweep.
#[test]
fn stream_random_property_sweep() {
    let s = Str::new();
    let mut rng = Rng::new(0xB508);
    unsafe {
        let (cbnd, _) = both::<unsafe extern "C" fn(c_int) -> ZSTD_bounds>("ZSTD_cParam_getBounds");
        for i in 0..800 {
            s.reset();
            let mut desc = String::new();
            for _ in 0..(1 + rng.below(5)) {
                let (name, id) = ALL_CPARAMS[rng.below(ALL_CPARAMS.len())];
                let b = cbnd(id);
                if b.error != 0 { continue; }
                let v = rng.range(b.lowerBound as i64, b.upperBound as i64) as c_int;
                desc.push_str(&format!("{name}={v} "));
                s.cset(&format!("#{i}"), id, v);
            }
            s.dset(&format!("#{i}"), ZSTD_d_windowLogMax, 31);
            s.dset(&format!("#{i}"), ZSTD_d_maxBlockSize, 131_072);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = LENS[rng.below(LENS.len())];
            let src = gen(shape, len, &mut rng);
            let ic = 1 + rng.below(70_000);
            let oc = 1 + rng.below(70_000);
            let ctx = format!("#{i} [{desc}] shape={shape:?} len={} ic={ic} oc={oc}", src.len());
            if let Some(f) = drive_compress(&s, &ctx, &src, ic, oc, rng.below(4), false) {
                s.reset();
                s.dset(&ctx, ZSTD_d_windowLogMax, 31);
                s.dset(&ctx, ZSTD_d_maxBlockSize, 131_072);
                if let Some(pt) = drive_decompress(&s, &ctx, &f, 1 + rng.below(70_000),
                                                   1 + rng.below(70_000)) {
                    assert_bytes_eq(&format!("{ctx}: round-trip"), &pt, &src);
                }
            }
        }
    }
}
