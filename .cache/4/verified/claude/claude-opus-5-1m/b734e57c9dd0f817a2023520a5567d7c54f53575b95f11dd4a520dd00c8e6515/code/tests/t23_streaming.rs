//! Phase B (valid path) + Phase C (error path) — STREAMING compression and
//! decompression.
//!
//! Everything here drives the *public streaming* entry points through the FFI
//! boundary of both shared objects and compares, after **every** call, the
//! return value (symbolically, via `R`), `in.pos`, `out.pos` and the bytes the
//! call appended to the output. The C branches under test are named in the
//! doc-comment of each `#[test]`.
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_ulonglong, c_void};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

type FnSz = unsafe extern "C" fn() -> SizeT;
type FnSizeofPtr = unsafe extern "C" fn(*const c_void) -> SizeT;
type FnPtrOnly = unsafe extern "C" fn(*mut c_void) -> SizeT;
type FnInitCStream = unsafe extern "C" fn(*mut c_void, c_int) -> SizeT;
type FnInitCStreamSrcSize = unsafe extern "C" fn(*mut c_void, c_int, c_ulonglong) -> SizeT;
type FnInitCStreamUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, c_int) -> SizeT;
type FnInitCStreamUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnInitCStreamAdvanced =
    unsafe extern "C" fn(*mut c_void, *const c_void, SizeT, ZSTD_parameters, c_ulonglong) -> SizeT;
type FnInitCStreamUsingCDictAdvanced =
    unsafe extern "C" fn(*mut c_void, *const c_void, ZSTD_frameParameters, c_ulonglong) -> SizeT;
type FnU64Arg = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> SizeT;
type FnCompressStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> SizeT;
type FnFlushStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> SizeT;
type FnFrameProgression = unsafe extern "C" fn(*const c_void) -> ZSTD_frameProgression;
type FnCS2Simple = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *mut SizeT,
    *const c_void,
    SizeT,
    *mut SizeT,
    c_int,
) -> SizeT;
type FnDSSimple = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *mut SizeT,
    *const c_void,
    SizeT,
    *mut SizeT,
) -> SizeT;
type FnSetMaxWindowSize = unsafe extern "C" fn(*mut c_void, SizeT) -> SizeT;
type FnInitDStreamUsingDict = unsafe extern "C" fn(*mut c_void, *const c_void, SizeT) -> SizeT;
type FnInitDStreamUsingDDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> SizeT;
type FnCreateAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, SizeT, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, SizeT) -> *mut c_void;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, SizeT) -> ZSTD_parameters;
type FnGetFrameHeader =
    unsafe extern "C" fn(*mut ZSTD_FrameHeader, *const c_void, SizeT) -> SizeT;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    SizeT,
    *const c_void,
    SizeT,
    *const c_void,
    SizeT,
) -> SizeT;

// ---------------------------------------------------------------------------
// Per-call record
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
struct Step {
    ret: R,
    ip: SizeT,
    op: SizeT,
}

impl fmt::Debug for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?} i{} o{}]", self.ret, self.ip, self.op)
    }
}

/// A whole call sequence. `Debug` stays bounded so a divergence in a 5000-call
/// drain does not print megabytes.
#[derive(Clone, PartialEq, Eq)]
struct Steps(Vec<Step>);

impl fmt::Debug for Steps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0.len();
        write!(f, "Steps(n={n}")?;
        if n <= 96 {
            for (i, s) in self.0.iter().enumerate() {
                write!(f, " {i}:{s:?}")?;
            }
        } else {
            for i in 0..48 {
                write!(f, " {i}:{:?}", self.0[i])?;
            }
            write!(f, " ..")?;
            for i in n - 48..n {
                write!(f, " {i}:{:?}", self.0[i])?;
            }
        }
        write!(f, ")")
    }
}

/// `((setup returns, per-call records, total input consumed, frame closed),
/// output bytes)`.
type Run = ((Vec<R>, Steps, usize, bool), Blob);

// ---------------------------------------------------------------------------
// Setup descriptors
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct CSetup {
    level: Option<c_int>,
    params: Vec<(c_int, c_int)>,
    pledged: Option<u64>,
}

impl CSetup {
    fn lvl(level: c_int) -> CSetup {
        CSetup {
            level: Some(level),
            ..Default::default()
        }
    }
    fn p(mut self, k: c_int, v: c_int) -> CSetup {
        self.params.push((k, v));
        self
    }
    fn pledge(mut self, n: u64) -> CSetup {
        self.pledged = Some(n);
        self
    }
}

#[derive(Clone, Debug, Default)]
struct DSetup {
    params: Vec<(c_int, c_int)>,
    max_window: Option<SizeT>,
}

impl DSetup {
    fn p(mut self, k: c_int, v: c_int) -> DSetup {
        self.params.push((k, v));
        self
    }
    fn win(mut self, n: SizeT) -> DSetup {
        self.max_window = Some(n);
        self
    }
}

fn apply_c(l: &Lib, cctx: *mut c_void, s: &CSetup) -> Vec<R> {
    let mut out = Vec::new();
    let setp = l.sym::<FnCCtxSetParameter>("ZSTD_CCtx_setParameter");
    if let Some(lv) = s.level {
        out.push(res(l, unsafe { setp(cctx, ZSTD_c_compressionLevel, lv) }));
    }
    for &(k, v) in &s.params {
        out.push(res(l, unsafe { setp(cctx, k, v) }));
    }
    if let Some(p) = s.pledged {
        let f = l.sym::<FnU64Arg>("ZSTD_CCtx_setPledgedSrcSize");
        out.push(res(l, unsafe { f(cctx, p as c_ulonglong) }));
    }
    out
}

fn apply_d(l: &Lib, dctx: *mut c_void, s: &DSetup) -> Vec<R> {
    let mut out = Vec::new();
    let setp = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
    for &(k, v) in &s.params {
        out.push(res(l, unsafe { setp(dctx, k, v) }));
    }
    if let Some(w) = s.max_window {
        let f = l.sym::<FnSetMaxWindowSize>("ZSTD_DCtx_setMaxWindowSize");
        out.push(res(l, unsafe { f(dctx, w) }));
    }
    out
}

// ---------------------------------------------------------------------------
// Fixtures built with the C library only (ground truth)
// ---------------------------------------------------------------------------

/// One-shot `ZSTD_compress2` with `s` applied, on the C library.
fn c_frame(src: &[u8], s: &CSetup) -> Vec<u8> {
    let l = &pair().c;
    let cctx = Ctx::cctx(l);
    for r in apply_c(l, cctx.ptr, s) {
        assert!(matches!(r, R::Ok(_)), "C fixture setup failed: {r:?}");
    }
    let cap = compress_bound(l, src.len()) + 128;
    let mut dst = vec![0u8; cap];
    let f = l.sym::<FnCompress2>("ZSTD_compress2");
    let n = unsafe {
        f(
            cctx.ptr,
            dst.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
        )
    };
    match res(l, n) {
        R::Ok(n) => {
            dst.truncate(n);
            dst
        }
        e => panic!("C fixture compression failed: {e:?}"),
    }
}

/// Build a fixture frame with the C library via **streaming**, leaving the first
/// call on `ZSTD_e_continue` so `ZSTD_CCtx_init_compressStream2` never runs
/// `pledgedSrcSizePlusOne = inSize + 1` (zstd_compress.c:6366) and
/// `pledgedSrcSize` stays `ZSTD_CONTENTSIZE_UNKNOWN`.
///
/// This matters for any test that needs the frame header to actually carry a
/// large `windowLog`: with a known `srcSize`,
/// `ZSTD_adjustCParams_internal` clamps `windowLog` down to `ceil(log2(srcSize))`
/// (zstd_compress.c:1551-1559) *and* `ZSTD_writeFrameHeader` sets
/// `singleSegment = contentSizeFlag && (windowSize >= pledgedSrcSize)` (:4704) so
/// the windowLog byte is omitted entirely (:4721).
fn c_frame_streamed(src: &[u8], s: &CSetup) -> Vec<u8> {
    let l = &pair().c;
    let cs = Ctx::cstream(l);
    for r in apply_c(l, cs.ptr, s) {
        assert!(matches!(r, R::Ok(_)), "C fixture setup failed: {r:?}");
    }
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let mut holder = vec![0u8; src.len() + 1];
    holder[..src.len()].copy_from_slice(src);
    let mut ov = vec![0u8; 1 << 18];
    let mut out: Vec<u8> = Vec::new();
    let mut consumed = 0usize;
    // Call 0 is an empty ZSTD_e_continue: it performs the transparent
    // initialisation with endOp != ZSTD_e_end, pinning pledgedSrcSize as unknown.
    for i in 0..(src.len() / 65536 + 4000) {
        let dir = if i == 0 {
            ZSTD_e_continue
        } else if consumed < src.len() {
            ZSTD_e_continue
        } else {
            ZSTD_e_end
        };
        let ilen = if i == 0 { 0 } else { src.len() - consumed };
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, dir) };
        match res(l, ret) {
            R::Ok(rem) => {
                out.extend_from_slice(&ov[..ob.pos]);
                consumed += inb.pos;
                if dir == ZSTD_e_end && rem == 0 {
                    return out;
                }
            }
            e => panic!("C streamed fixture failed: {e:?}"),
        }
    }
    panic!("C streamed fixture did not terminate");
}

/// `ZSTD_getFrameHeader` on the C library, used to assert a fixture really
/// carries the frame parameters the test intends.
fn c_header(comp: &[u8]) -> ZSTD_FrameHeader {
    let l = &pair().c;
    let f = l.sym::<FnGetFrameHeader>("ZSTD_getFrameHeader");
    let mut h = ZSTD_FrameHeader::default();
    let r = unsafe { f(&mut h, comp.as_ptr() as *const c_void, comp.len()) };
    assert_eq!(res(l, r), R::Ok(0), "ZSTD_getFrameHeader on fixture");
    h
}

fn cstream_in_size() -> usize {
    let l = &pair().c;
    unsafe { l.sym::<FnSz>("ZSTD_CStreamInSize")() }
}
fn cstream_out_size() -> usize {
    let l = &pair().c;
    unsafe { l.sym::<FnSz>("ZSTD_CStreamOutSize")() }
}

// ---------------------------------------------------------------------------
// Drivers
// ---------------------------------------------------------------------------

/// A schedule step: (input chunk offered, output capacity, `ZSTD_EndDirective`).
type Sched = Vec<(usize, usize, c_int)>;

/// Drive `ZSTD_compressStream2` over `sched`, then drain with `ZSTD_e_end` and
/// `drain_cap`-sized output until the frame closes.
///
/// The schedule is a pure function of its arguments, so both libraries execute
/// exactly the same call sequence unless they diverge — which is what the
/// returned per-call records detect.
fn drive_cs2(l: &Lib, src: &[u8], s: &CSetup, sched: &Sched, drain_cap: usize) -> Run {
    let cs = Ctx::cstream(l);
    let setup = apply_c(l, cs.ptr, s);
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    // +1 so `holder+consumed` is always a real, in-allocation address even when
    // the whole input has been consumed.
    let mut holder = vec![0u8; src.len() + 1];
    holder[..src.len()].copy_from_slice(src);

    let mut consumed = 0usize;
    let mut outall: Vec<u8> = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut finished = false;
    let mut failed = false;
    // `ZSTD_CCtx_init_compressStream2` turns the FIRST call's input size into
    // pledgedSrcSize when endOp == ZSTD_e_end (zstd_compress.c:6480 comment), so
    // once the frame is being ended the driver may never offer more input than
    // was visible on that call — doing so is a genuine `srcSize_wrong`, covered
    // separately in `t_pledged_src_size`.
    let mut end_limit: Option<usize> = None;

    let call = |dir: c_int,
                    ichunk: usize,
                    ocap: usize,
                    consumed: &mut usize,
                    outall: &mut Vec<u8>,
                    steps: &mut Vec<Step>,
                    end_limit: &mut Option<usize>|
     -> (bool, bool) {
        let avail = match *end_limit {
            Some(lim) => lim - *consumed,
            None => src.len() - *consumed,
        };
        let ilen = ichunk.min(avail);
        if dir == ZSTD_e_end && end_limit.is_none() {
            *end_limit = Some(*consumed + ilen);
        }
        let ocap = ocap.max(1);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(*consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ov = vec![0u8; ocap];
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ocap,
            pos: 0,
        };
        let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, dir) };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: inb.pos,
            op: ob.pos,
        });
        outall.extend_from_slice(&ov[..ob.pos]);
        *consumed += inb.pos;
        let err = matches!(r, R::Err(..));
        let done = !err && dir == ZSTD_e_end && ret == 0;
        (err, done)
    };

    for &(ichunk, ocap, dir) in sched {
        let (e, d) = call(
            dir,
            ichunk,
            ocap,
            &mut consumed,
            &mut outall,
            &mut steps,
            &mut end_limit,
        );
        if e {
            failed = true;
            break;
        }
        if d {
            finished = true;
            break;
        }
    }
    if !failed && !finished {
        for _ in 0..8000 {
            let (e, d) = call(
                ZSTD_e_end,
                usize::MAX,
                drain_cap,
                &mut consumed,
                &mut outall,
                &mut steps,
                &mut end_limit,
            );
            if e {
                failed = true;
                break;
            }
            if d {
                finished = true;
                break;
            }
        }
    }
    let _ = failed;
    ((setup, Steps(steps), consumed, finished), Blob(outall))
}

/// Drive `ZSTD_decompressStream` over `sched`, then drain with the whole
/// remaining input and `drain_cap`-sized output.
fn drive_ds(l: &Lib, comp: &[u8], s: &DSetup, sched: &Sched, drain_cap: usize) -> Run {
    let ds = Ctx::dstream(l);
    let setup = apply_d(l, ds.ptr, s);
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut holder = vec![0u8; comp.len() + 1];
    holder[..comp.len()].copy_from_slice(comp);

    let mut consumed = 0usize;
    let mut outall: Vec<u8> = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut stop = false;
    let mut stall = 0u32;
    let mut finished = false;

    let call = |ichunk: usize,
                    ocap: usize,
                    consumed: &mut usize,
                    outall: &mut Vec<u8>,
                    steps: &mut Vec<Step>,
                    stall: &mut u32,
                    finished: &mut bool|
     -> bool {
        let ilen = ichunk.min(comp.len() - *consumed);
        let ocap = ocap.max(1);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(*consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ov = vec![0u8; ocap];
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ocap,
            pos: 0,
        };
        let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: inb.pos,
            op: ob.pos,
        });
        outall.extend_from_slice(&ov[..ob.pos]);
        *consumed += inb.pos;
        if inb.pos == 0 && ob.pos == 0 {
            *stall += 1;
        } else {
            *stall = 0;
        }
        // `ZSTD_decompressStream` returns 0 exactly when a frame has been fully
        // decoded; anything else is "bytes still hoped for". Record which of the
        // three terminal conditions we hit so the two libraries are compared on
        // *why* they stopped, not just on the bytes produced.
        let err = matches!(r, R::Err(..));
        if !err && ret == 0 {
            *finished = true;
        }
        err || ret == 0 || *stall >= 3
    };

    for &(ichunk, ocap, _dir) in sched {
        if call(
            ichunk,
            ocap,
            &mut consumed,
            &mut outall,
            &mut steps,
            &mut stall,
            &mut finished,
        ) {
            stop = true;
            break;
        }
    }
    if !stop {
        for _ in 0..8000 {
            if call(
                usize::MAX,
                drain_cap,
                &mut consumed,
                &mut outall,
                &mut steps,
                &mut stall,
                &mut finished,
            ) {
                break;
            }
        }
    }
    ((setup, Steps(steps), consumed, finished), Blob(outall))
}

// ---------------------------------------------------------------------------
// Schedule generation
// ---------------------------------------------------------------------------

fn all_caps() -> Vec<usize> {
    vec![
        1,
        2,
        3,
        7,
        17,
        100,
        1000,
        4096,
        cstream_in_size(),
        cstream_out_size(),
        65535,
        131072,
    ]
}

/// Keep tiny buffers away from big inputs: a 2 MB payload flushed one byte at a
/// time would be millions of FFI calls.
fn caps_for(size: usize) -> Vec<usize> {
    let lo = if size <= 2_000 {
        1
    } else if size <= 65_536 {
        7
    } else if size <= 200_000 {
        100
    } else {
        4096
    };
    all_caps().into_iter().filter(|&c| c >= lo).collect()
}

fn gen_sched(rng: &mut Rng, nsteps: usize, caps: &[usize]) -> Sched {
    let mut out = Sched::new();
    let mut ended = false;
    for _ in 0..nsteps {
        let ichunk = *rng.pick(caps);
        let ocap = *rng.pick(caps);
        let dir = if ended {
            ZSTD_e_end
        } else {
            match rng.below(10) {
                0..=6 => ZSTD_e_continue,
                7 | 8 => ZSTD_e_flush,
                _ => ZSTD_e_end,
            }
        };
        if dir == ZSTD_e_end {
            ended = true;
        }
        out.push((ichunk, ocap, dir));
    }
    out
}

// ---------------------------------------------------------------------------
// 1. sizes, lifecycle, sizeof
// ---------------------------------------------------------------------------

/// `ZSTD_CStreamInSize` / `ZSTD_CStreamOutSize` / `ZSTD_DStreamInSize` /
/// `ZSTD_DStreamOutSize` (zstd_compress.c:5952/5954, zstd_decompress.c:1696/1697)
/// and `ZSTD_sizeof_CStream` / `ZSTD_sizeof_DStream` sampled at several points in
/// the lifecycle. Also `create`/`free` of both stream objects and `free(NULL)`.
#[test]
fn t_stream_sizes_and_lifecycle() {
    covers(&[
        "CFG:6",
        "CFG:7",
        "CFG:8",
        "ERR:compress/zstd_compress.c:5952,compress/zstd_compress.c:5954",
    ]);

    diff("stream/const-sizes", |l| {
        (
            unsafe { l.sym::<FnSz>("ZSTD_CStreamInSize")() },
            unsafe { l.sym::<FnSz>("ZSTD_CStreamOutSize")() },
            unsafe { l.sym::<FnSz>("ZSTD_DStreamInSize")() },
            unsafe { l.sym::<FnSz>("ZSTD_DStreamOutSize")() },
        )
    });

    // free(NULL) for both stream free functions.
    diff("stream/free-null", |l| {
        (
            res(l, unsafe {
                l.sym::<FnPtrOnly>("ZSTD_freeCStream")(std::ptr::null_mut())
            }),
            res(l, unsafe {
                l.sym::<FnPtrOnly>("ZSTD_freeDStream")(std::ptr::null_mut())
            }),
            unsafe { l.sym::<FnSizeofPtr>("ZSTD_sizeof_CStream")(std::ptr::null()) },
            unsafe { l.sym::<FnSizeofPtr>("ZSTD_sizeof_DStream")(std::ptr::null()) },
        )
    });

    let src = corpus(Corpus::Text, 300_000, 1);
    let cin = cstream_in_size();
    let cout = cstream_out_size();

    // sizeof_CStream: fresh, after a buffered streaming session (inBuff+outBuff
    // allocated), after ZSTD_compress2 (stable buffers, no inBuff/outBuff).
    diff("stream/sizeof-cstream-lifecycle", |l| {
        let szf = l.sym::<FnSizeofPtr>("ZSTD_sizeof_CStream");
        let cs = Ctx::cstream(l);
        let fresh = unsafe { szf(cs.ptr) };
        let init = l.sym::<FnInitCStream>("ZSTD_initCStream");
        assert_eq!(res(l, unsafe { init(cs.ptr, 3) }), R::Ok(0));
        let after_init = unsafe { szf(cs.ptr) };
        let sched: Sched = vec![(4096, 4096, ZSTD_e_continue); 6];
        let _ = drive_cs2(l, &src, &CSetup::lvl(3), &sched, cout);
        let after_session = unsafe { szf(cs.ptr) };
        (fresh, after_init, after_session)
    });

    diff("stream/sizeof-dstream-lifecycle", |l| {
        let szf = l.sym::<FnSizeofPtr>("ZSTD_sizeof_DStream");
        let ds = Ctx::dstream(l);
        let fresh = unsafe { szf(ds.ptr) };
        let f = l.sym::<FnPtrOnly>("ZSTD_initDStream");
        let init_ret = res(l, unsafe { f(ds.ptr) });
        let after_init = unsafe { szf(ds.ptr) };
        (fresh, after_init, init_ret)
    });

    // sizeof_DStream after decoding a wlog=10 frame and a wlog=27 frame.
    for &wlog in &[10i32, 27] {
        let comp = c_frame(&src, &CSetup::lvl(3).p(ZSTD_c_windowLog, wlog));
        let label = format!("stream/sizeof-dstream-wlog{wlog}");
        diff(&label, |l| {
            let ds = Ctx::dstream(l);
            let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
            let mut ov = vec![0u8; 1 << 16];
            let mut consumed = 0usize;
            let mut total = 0usize;
            loop {
                let mut inb = ZSTD_inBuffer {
                    src: unsafe { comp.as_ptr().add(consumed) } as *const c_void,
                    size: comp.len() - consumed,
                    pos: 0,
                };
                let mut ob = ZSTD_outBuffer {
                    dst: ov.as_mut_ptr() as *mut c_void,
                    size: ov.len(),
                    pos: 0,
                };
                let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
                if is_error(l, ret) {
                    return (res(l, ret), 0usize, 0usize);
                }
                consumed += inb.pos;
                total += ob.pos;
                if ret == 0 {
                    break;
                }
            }
            let sz = unsafe { l.sym::<FnSizeofPtr>("ZSTD_sizeof_DStream")(ds.ptr) };
            (R::Ok(0), total, sz)
        });
    }

    // CStreamInSize / CStreamOutSize consumed as chunk sizes (CFG:63).
    let sched: Sched = vec![
        (cin, cout, ZSTD_e_continue),
        (cin, cout, ZSTD_e_continue),
        (cin, cin, ZSTD_e_continue),
        (cin, cin, ZSTD_e_flush),
    ];
    let (_, comp) = diff_bytes("stream/cin-cout-chunks", |l| {
        drive_cs2(l, &src, &CSetup::lvl(3), &sched, cout)
    });
    let dsched: Sched = vec![(cin, 131072, 0); 8];
    diff_bytes("stream/cin-cout-chunks/dec", |l| {
        drive_ds(l, &comp.0, &DSetup::default(), &dsched, 131072)
    });
    covers(&["CFG:63"]);
}

// ---------------------------------------------------------------------------
// 2. the big randomized chunk-schedule sweep
// ---------------------------------------------------------------------------

/// `ZSTD_compressStream2` under a randomized chunk/capacity/directive schedule.
///
/// Targets `ZSTD_compressStream_generic` (zstd_compress.c:6143-6280) in full:
/// the `ZSTD_e_end` single-pass shortcut, the `zcss_load` buffered fill, the
/// `cDst == op` "compress straight into the output" branch vs the `outBuff`
/// detour, and the partial-`zcss_flush` path. Also the transparent
/// initialisation stage of `ZSTD_compressStream2` (:6461-6482).
#[test]
fn t_compress_stream2_random_schedule() {
    covers(&[
        "CFG:28-31",
        "CFG:61",
        "CFG:62",
        "CFG:64",
        "CFG:66",
        "CFG:101",
        "ERR:compress/zstd_compress.c:6303",
    ]);
    let cout = cstream_out_size();
    let mut rng = Rng::new(0x5723_0001);

    const SMALL: &[usize] = &[
        0, 1, 2, 3, 7, 17, 100, 1000, 4096, 16384, 65535, 65536, 131071, 131072, 131073, 200000,
    ];
    const BIG: &[usize] = &[300_000, 700_000, 1_500_000, 2_000_000];

    for trial in 0..220usize {
        let kind = *rng.pick(ALL_CORPORA);
        let big = trial % 12 == 5;
        let size = if big {
            *rng.pick(BIG)
        } else {
            *rng.pick(SMALL)
        };
        let level = if big {
            *rng.pick(&[-3i32, 1, 3, 6])
        } else {
            *rng.pick(&[-5i32, -1, 1, 3, 6, 9, 19])
        };
        let mut s = CSetup::lvl(level);
        // Fold in the frame/strategy axes CFG:28-31 name.
        if rng.bool() {
            s = s.p(ZSTD_c_checksumFlag, 1);
        }
        if rng.bool() {
            s = s.p(ZSTD_c_contentSizeFlag, 0);
        }
        match rng.below(6) {
            0 => s = s.p(ZSTD_c_strategy, ZSTD_fast).p(ZSTD_c_windowLog, 10),
            1 => s = s.p(ZSTD_c_strategy, ZSTD_dfast).p(ZSTD_c_windowLog, 10),
            2 => s = s.p(ZSTD_c_strategy, ZSTD_greedy).p(ZSTD_c_windowLog, 14),
            3 => s = s.p(ZSTD_c_strategy, ZSTD_lazy2).p(ZSTD_c_windowLog, 15),
            4 => s = s.p(ZSTD_c_strategy, ZSTD_btlazy2).p(ZSTD_c_windowLog, 11),
            _ => {}
        }
        let src = corpus(kind, size, 0x9e37 + trial as u64);
        let caps = caps_for(size);
        let nsteps = 1 + rng.below(24);
        let sched = gen_sched(&mut rng, nsteps, &caps);
        let label = format!("cs2/rand/{trial}/{kind:?}/n{size}/l{level}/steps{nsteps}");

        let ((_, _, consumed, finished), comp) =
            diff_bytes(&label, |l| drive_cs2(l, &src, &s, &sched, cout));

        // Whatever the two libraries produced, they produced the SAME thing —
        // that is the differential assertion and it is unconditional. The
        // round-trip below is an extra self-consistency check, and it only holds
        // when the schedule actually CLOSED the frame: a random schedule can run
        // out of steps mid-frame, in which case `ZSTD_decompress` on the partial
        // output legitimately returns `srcSize_wrong` (72) on both libraries.
        let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
            decompress_simple(l, &comp.0, consumed + 64)
        });
        if finished {
            assert_eq!(r, R::Ok(consumed), "{label}: round-trip status");
            assert_eq!(
                got.0.len(),
                consumed,
                "{label}: round-trip length {} != consumed {consumed}",
                got.0.len()
            );
            assert!(
                got.0[..] == src[..consumed],
                "{label}: round-trip payload mismatch: {}",
                first_diff(&got.0, &src[..consumed]).unwrap_or_default()
            );
        } else {
            // An unterminated frame must be rejected, not silently accepted.
            assert!(
                matches!(r, R::Err(..)),
                "{label}: an unterminated frame decoded successfully as {r:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. one byte at a time, and the "no progress" neighbourhood on the C side
// ---------------------------------------------------------------------------

/// One byte of input per call and one byte of output per call. The compressor
/// has **no** forward-progress detector (`grep -c noForwardProgress
/// compress/` == 0), so back-to-back identical calls must simply return the
/// same hint with `in.pos == out.pos == 0` and no error; the counter at
/// zstd_decompress.c:2357 exists only on the decode side.
#[test]
fn t_compress_stream2_byte_at_a_time() {
    covers(&["CFG:62", "CFG:64", "CFG:101"]);
    let cout = cstream_out_size();

    for &n in &[0usize, 1, 6, 100, 1000, 4096] {
        for &kind in &[Corpus::Zeros, Corpus::Text, Corpus::Random] {
            let src = corpus(kind, n, 7);
            // 1 byte in / big out
            let sched: Sched = vec![(1, cout, ZSTD_e_continue); n + 4];
            let label = format!("cs2/1in/{kind:?}/{n}");
            let ((_, _, consumed, _), comp) =
                diff_bytes(&label, |l| drive_cs2(l, &src, &CSetup::lvl(3), &sched, cout));
            assert_eq!(consumed, n);
            let (r, got) =
                diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &comp.0, n + 64));
            assert_eq!(r, R::Ok(n));
            assert_eq!(&got.0[..], &src[..]);

            // 1 byte out / big in — the partial-`zcss_flush` path, hit once per byte.
            let sched: Sched = vec![(usize::MAX, 1, ZSTD_e_continue); 4];
            let label = format!("cs2/1out/{kind:?}/{n}");
            let ((_, _, consumed, _), comp) =
                diff_bytes(&label, |l| drive_cs2(l, &src, &CSetup::lvl(3), &sched, 1));
            assert_eq!(consumed, n);
            let (r, got) =
                diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &comp.0, n + 64));
            assert_eq!(r, R::Ok(n));
            assert_eq!(&got.0[..], &src[..]);
        }
    }

    // Same buffers twice, no progress possible: `out.size == 0` and an input
    // shorter than a block with `ZSTD_e_continue`. 24 back-to-back calls.
    let src = corpus(Corpus::Text, 1000, 11);
    diff("cs2/no-progress/out0", |l| {
        let cs = Ctx::cstream(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut steps = Vec::new();
        let mut dst = [0u8; 1];
        for _ in 0..24 {
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: dst.as_mut_ptr() as *mut c_void,
                size: 0,
                pos: 0,
            };
            let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, ZSTD_e_flush) };
            steps.push(Step {
                ret: res(l, ret),
                ip: inb.pos,
                op: ob.pos,
            });
        }
        Steps(steps)
    });

    // Same input buffer presented repeatedly with `ZSTD_e_continue` and a
    // sub-block-size input: the C consumes it into inBuff every time (so the
    // frame content grows), which is legal and must match.
    diff("cs2/repeat-same-input", |l| {
        let cs = Ctx::cstream(l);
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut steps = Vec::new();
        let mut ov = vec![0u8; 1 << 16];
        for _ in 0..12 {
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, ZSTD_e_continue) };
            steps.push(Step {
                ret: res(l, ret),
                ip: inb.pos,
                op: ob.pos,
            });
        }
        Steps(steps)
    });
}

// ---------------------------------------------------------------------------
// 4. decompress-side noForwardProgress
// ---------------------------------------------------------------------------

/// `ZSTD_decompressStream`'s forward-progress guard, zstd_decompress.c:2357-2361.
/// `ZSTD_NO_FORWARD_PROGRESS_MAX` is 16, so the error appears on the 16th
/// consecutive call that consumed no input and produced no output — the exact
/// call index is part of what is compared.
#[test]
fn t_decompress_stream_no_forward_progress() {
    covers(&[
        "ERR:decompress/zstd_decompress.c:2359,decompress/zstd_decompress.c:2360",
        "CFG:85",
        "CFG:87",
    ]);
    let src = corpus(Corpus::Text, 300_000, 21);
    let comp = c_frame(&src, &CSetup::lvl(3));

    // (a) destFull: out.size == 0 for every call, full input available. The
    // first call buffers a block and cannot flush it; every later call is a
    // no-op -> code 80.
    diff("ds/nfp/destFull", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut steps = Vec::new();
        let mut consumed = 0usize;
        let mut one = [0u8; 1];
        for _ in 0..25 {
            let mut inb = ZSTD_inBuffer {
                src: unsafe { comp.as_ptr().add(consumed) } as *const c_void,
                size: comp.len() - consumed,
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: one.as_mut_ptr() as *mut c_void,
                size: 0,
                pos: 0,
            };
            let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
            consumed += inb.pos;
            let r = res(l, ret);
            let stop = matches!(r, R::Err(..));
            steps.push(Step {
                ret: r,
                ip: inb.pos,
                op: ob.pos,
            });
            if stop {
                break;
            }
        }
        Steps(steps)
    });

    // (b) inputEmpty: prime the stream with 30 bytes (past the frame header),
    // then call with an empty input and a roomy output -> code 82.
    diff("ds/nfp/inputEmpty", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut ov = vec![0u8; 1 << 17];
        let mut steps = Vec::new();
        let mut inb = ZSTD_inBuffer {
            src: comp.as_ptr() as *const c_void,
            size: 30,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
        steps.push(Step {
            ret: res(l, ret),
            ip: inb.pos,
            op: ob.pos,
        });
        let one = [0u8; 1];
        for _ in 0..25 {
            let mut inb = ZSTD_inBuffer {
                src: one.as_ptr() as *const c_void,
                size: 0,
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
            let r = res(l, ret);
            let stop = matches!(r, R::Err(..));
            steps.push(Step {
                ret: r,
                ip: inb.pos,
                op: ob.pos,
            });
            if stop {
                break;
            }
        }
        Steps(steps)
    });

    // (c) the counter must RESET on a call that makes progress: interleave a
    // productive call every 10 stalls and confirm neither library ever errors.
    diff("ds/nfp/reset", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut ov = vec![0u8; 1 << 17];
        let mut steps = Vec::new();
        let mut consumed = 0usize;
        let one = [0u8; 1];
        for round in 0..4 {
            for _ in 0..10 {
                let mut inb = ZSTD_inBuffer {
                    src: one.as_ptr() as *const c_void,
                    size: 0,
                    pos: 0,
                };
                let mut ob = ZSTD_outBuffer {
                    dst: ov.as_mut_ptr() as *mut c_void,
                    size: ov.len(),
                    pos: 0,
                };
                let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
                steps.push(Step {
                    ret: res(l, ret),
                    ip: inb.pos,
                    op: ob.pos,
                });
            }
            let _ = round;
            let mut inb = ZSTD_inBuffer {
                src: unsafe { comp.as_ptr().add(consumed) } as *const c_void,
                size: (comp.len() - consumed).min(1000),
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
            consumed += inb.pos;
            steps.push(Step {
                ret: res(l, ret),
                ip: inb.pos,
                op: ob.pos,
            });
        }
        Steps(steps)
    });
}

// ---------------------------------------------------------------------------
// 3b. ZSTD_CCtx_setPledgedSrcSize on a stream
// ---------------------------------------------------------------------------

/// `ZSTD_CCtx_setPledgedSrcSize` combined with streaming.
///
/// `drive_cs2`'s comment refers to this test: the pledge is compared against the
/// bytes actually fed, at two different sites — `ZSTD_compressContinue_internal`
/// rejects *more* than pledged (zstd_compress.c:4842) while
/// `ZSTD_compressEnd_public` rejects *fewer* (:5422) — and the pledge also
/// changes the frame header's FCS field width and
/// `inBuffTarget = blockSizeMax + (blockSizeMax == pledgedSrcSize)`
/// (:6434), which is why 131071 / 131072 / 131073 are swept explicitly.
#[test]
fn t_pledged_src_size() {
    covers(&[
        "CFG:45",
        "CFG:66",
        "ERR:compress/zstd_compress.c:1233",
        "ERR:compress/zstd_compress.c:4842",
        "ERR:compress/zstd_compress.c:5422",
    ]);
    let cout = cstream_out_size();

    for &n in &[0usize, 10, 1000, 131_071, 131_072, 131_073, 300_000] {
        let src = corpus(Corpus::Text, n, 0x4545 + n as u64);
        for &pledge in &[
            0u64,
            n as u64,
            n as u64 + 1,
            n.saturating_sub(1) as u64,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0x1_0000_0000u64,
        ] {
            for &chunk in &[1024usize, 16384, usize::MAX] {
                // The first step must be ZSTD_e_continue: a leading ZSTD_e_end
                // would make ZSTD_CCtx_init_compressStream2 overwrite the pledge
                // with that call's input size (zstd_compress.c:6366).
                let steps_n = if chunk == usize::MAX {
                    2
                } else {
                    n / chunk + 2
                };
                let sched: Sched = vec![(chunk, cout, ZSTD_e_continue); steps_n];
                let s = CSetup::lvl(3).pledge(pledge);
                let label = format!("pledge/n{n}/p{pledge}/c{chunk}");
                let ((_, steps, consumed, finished), comp) =
                    diff_bytes(&label, |l| drive_cs2(l, &src, &s, &sched, cout));
                let codes: Vec<c_int> = steps
                    .0
                    .iter()
                    .filter_map(|st| match &st.ret {
                        R::Err(c, _) => Some(*c),
                        _ => None,
                    })
                    .collect();
                // `0` means "unknown" only for the deprecated init helpers;
                // ZSTD_CCtx_setPledgedSrcSize(0) really does pledge zero bytes.
                let expect_ok = pledge == n as u64 || pledge == ZSTD_CONTENTSIZE_UNKNOWN;
                if expect_ok {
                    assert!(codes.is_empty(), "{label}: {steps:?}");
                    assert!(finished, "{label}: not closed");
                    assert_eq!(consumed, n, "{label}");
                    let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                        decompress_simple(l, &comp.0, n + 64)
                    });
                    assert_eq!(r, R::Ok(n), "{label}");
                    assert_eq!(&got.0[..], &src[..], "{label}");
                } else {
                    assert_eq!(
                        codes,
                        vec![72],
                        "{label}: expected srcSize_wrong, got {steps:?}"
                    );
                }
            }
        }
    }

    // Mid-stream the setter is refused with stage_wrong (60), and the frame that
    // was already under way must still finish with the ORIGINAL pledge.
    diff_bytes("pledge/midstream", |l| {
        let src = corpus(Corpus::Text, 300_000, 0x4646);
        let cs = Ctx::cstream(l);
        let setp = l.sym::<FnU64Arg>("ZSTD_CCtx_setPledgedSrcSize");
        let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
        let mut rets = Vec::new();
        rets.push(res(l, unsafe { setp(cs.ptr, src.len() as c_ulonglong) }));
        let mut ov = vec![0u8; cout];
        let mut out = Vec::new();
        let mut consumed = 0usize;
        let mut steps = Vec::new();
        // one ZSTD_e_continue call to leave zcss_init
        {
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: 200_000,
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, ZSTD_e_continue) };
            steps.push(Step {
                ret: res(l, ret),
                ip: inb.pos,
                op: ob.pos,
            });
            out.extend_from_slice(&ov[..ob.pos]);
            consumed += inb.pos;
        }
        rets.push(res(l, unsafe { setp(cs.ptr, 12345) }));
        rets.push(res(l, unsafe { setp(cs.ptr, src.len() as c_ulonglong) }));
        // finish the frame with the rest of the input
        for _ in 0..64 {
            let mut inb = ZSTD_inBuffer {
                src: unsafe { src.as_ptr().add(consumed) } as *const c_void,
                size: src.len() - consumed,
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, ZSTD_e_end) };
            let r = res(l, ret);
            let stop = matches!(r, R::Err(..)) || ret == 0;
            steps.push(Step {
                ret: r,
                ip: inb.pos,
                op: ob.pos,
            });
            out.extend_from_slice(&ov[..ob.pos]);
            consumed += inb.pos;
            if stop {
                break;
            }
        }
        ((rets, Steps(steps), consumed), Blob(out))
    });
}

// ---------------------------------------------------------------------------
// 4b. the big randomized DECODE-side schedule sweep
// ---------------------------------------------------------------------------

/// `ZSTD_decompressStream` under a randomized chunk/capacity schedule, over
/// frames built with every combination of `ZSTD_c_checksumFlag` x
/// `ZSTD_c_contentSizeFlag` x `ZSTD_c_dictIDFlag` x `ZSTD_c_format`.
///
/// Targets the whole state machine of `ZSTD_decompressStream`
/// (zstd_decompress.c:2085-2400): the partial-header accumulation in
/// `zdss_loadHeader` (including `ZSTD_startingInputLength` == 1 for magicless
/// vs 5 for zstd1), the single-pass shortcut, `zdss_read`'s
/// decode-straight-from-src fast path vs the `zdss_load` buffering detour,
/// `zdss_flush`'s partial flush, the hostage-byte handoff and the trailing
/// checksum stage.
#[test]
fn t_decompress_stream_random_schedule() {
    covers(&[
        "CFG:85", "CFG:86", "CFG:87", "CFG:101", "CFG:120",
        "ERR:decompress/zstd_decompress.c:2161",
        "ERR:decompress/zstd_decompress.c:2221",
    ]);

    const SMALL: &[usize] = &[
        0, 1, 2, 3, 7, 17, 100, 1000, 4096, 16384, 65535, 65536, 131071, 131072, 131073, 200000,
    ];
    const BIG: &[usize] = &[300_000, 700_000];

    let mut rng = Rng::new(0x5723_0002);
    for trial in 0..400usize {
        let kind = *rng.pick(ALL_CORPORA);
        let big = trial % 16 == 7;
        let size = if big { *rng.pick(BIG) } else { *rng.pick(SMALL) };
        let level = if big {
            *rng.pick(&[-3i32, 1, 3, 6])
        } else {
            *rng.pick(&[-3i32, 1, 3, 6, 12, 19])
        };
        // The four frame-format axes, enumerated exhaustively by trial index so
        // all 16 combinations are hit for every schedule shape.
        let cks = (trial & 1) as c_int;
        let csz = ((trial >> 1) & 1) as c_int;
        let did = ((trial >> 2) & 1) as c_int;
        let magicless = (trial >> 3) & 1;
        let mut s = CSetup::lvl(level)
            .p(ZSTD_c_checksumFlag, cks)
            .p(ZSTD_c_contentSizeFlag, csz)
            .p(ZSTD_c_dictIDFlag, did)
            .p(
                ZSTD_c_format,
                if magicless == 1 {
                    ZSTD_f_zstd1_magicless
                } else {
                    ZSTD_f_zstd1
                },
            );
        // Vary the *encoder* too, so the frames being decoded exercise small
        // windows (ring-buffer wrap), sub-128 KB blocks, RLE/raw block types and
        // long-distance matches rather than one shape of block over and over.
        match rng.below(8) {
            0 => s = s.p(ZSTD_c_windowLog, 10),
            1 => s = s.p(ZSTD_c_windowLog, 11).p(ZSTD_c_strategy, ZSTD_btultra2),
            2 => s = s.p(ZSTD_c_maxBlockSize, 1024),
            3 => s = s.p(ZSTD_c_maxBlockSize, 2048).p(ZSTD_c_windowLog, 14),
            4 => s = s.p(ZSTD_c_targetCBlockSize, 1024),
            5 => s = s.p(ZSTD_c_enableLongDistanceMatching, 1),
            6 => s = s.p(ZSTD_c_literalCompressionMode, ZSTD_lcm_uncompressed),
            _ => {}
        }
        let src = corpus(kind, size, 0x4242 + trial as u64);
        let comp = c_frame(&src, &s);
        let mut d = DSetup::default();
        if magicless == 1 {
            d = d.p(ZSTD_d_format, ZSTD_f_zstd1_magicless);
        }
        let caps = caps_for(size);
        let nsteps = 1 + rng.below(20);
        let mut sched = gen_sched(&mut rng, nsteps, &caps);
        // Occasionally offer the whole remaining input in one go, which is what
        // arms the single-pass shortcut in zdss_loadHeader.
        for st in sched.iter_mut() {
            if rng.below(6) == 0 {
                st.0 = usize::MAX;
            }
        }
        let drain = *rng.pick(&caps);
        let label = format!(
            "ds/rand/{trial}/{kind:?}/n{size}/l{level}/k{cks}c{csz}d{did}m{magicless}/steps{nsteps}"
        );
        let ((_, _, consumed, finished), out) =
            diff_bytes(&label, |l| drive_ds(l, &comp, &d, &sched, drain));
        // Unconditional differential assertion is `diff_bytes` above. The rest is
        // self-consistency, and only holds when the schedule actually ran the
        // frame to completion (a random schedule can stop mid-frame, which is a
        // legitimate "more input hoped for" state on both libraries).
        if finished {
            assert_eq!(consumed, comp.len(), "{label}: input not fully consumed");
            assert_eq!(out.0.len(), size, "{label}: decoded length");
            assert!(
                out.0[..] == src[..],
                "{label}: payload mismatch: {}",
                first_diff(&out.0, &src).unwrap_or_default()
            );
        } else {
            assert!(
                out.0.len() <= size,
                "{label}: produced {} > {size} without finishing",
                out.0.len()
            );
        }
    }

    // A magicless frame handed to a DEFAULT (zstd1) DStream: the first 4 bytes
    // are read as a magic number, no legacy magic matches, so
    // ZSTD_getFrameHeader_advanced reports prefix_unknown (10) via
    // zstd_decompress.c:2161. Both libraries must agree on the exact code and on
    // how much input was consumed before it was raised.
    for &n in &[0usize, 100, 1000, 200_000] {
        for &ichunk in &[1usize, 5, 4096, usize::MAX] {
            let src = corpus(Corpus::Text, n, 77);
            let comp = c_frame(&src, &CSetup::lvl(3).p(ZSTD_c_format, ZSTD_f_zstd1_magicless));
            let sched: Sched = vec![(ichunk, 1 << 16, 0); 12];
            let label = format!("ds/magicless-as-zstd1/n{n}/c{ichunk}");
            let ((_, steps, _, finished), _) =
                diff_bytes(&label, |l| drive_ds(l, &comp, &DSetup::default(), &sched, 1 << 16));
            assert!(
                !finished,
                "{label}: a magicless frame decoded as zstd1 must not succeed"
            );
            assert!(
                steps.0.iter().any(|s| matches!(s.ret, R::Err(..))),
                "{label}: expected an error, got {steps:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4b2. the single-pass shortcut and the hostage byte
// ---------------------------------------------------------------------------

/// The "check for single-pass mode opportunity" block of `zdss_loadHeader`
/// (zstd_decompress.c:2180-2199) and the hostage-byte handoff at the bottom of
/// `ZSTD_decompressStream` (:2364-2385).
///
/// The shortcut delegates to `ZSTD_decompress_usingDDict` and leaves
/// `expected == 0, streamStage == zdss_init`, skipping every streaming stage —
/// so whether it triggers changes the internal buffer allocation *and* the
/// return value. Each case below is engineered to be on one side of that
/// decision.
#[test]
fn t_decompress_stream_shortcut_and_hostage() {
    covers(&[
        "CFG:86",
        "CFG:87",
        "ERR:decompress/zstd_decompress.c:2193",
    ]);
    let n = 300_000usize;
    let src = corpus(Corpus::Text, n, 0x86_86);
    let known = c_frame(&src, &CSetup::lvl(3));
    let unknown = c_frame_streamed(&src, &CSetup::lvl(3));
    assert_ne!(c_header(&known).frameContentSize, ZSTD_CONTENTSIZE_UNKNOWN);
    assert_eq!(c_header(&unknown).frameContentSize, ZSTD_CONTENTSIZE_UNKNOWN);

    // A well-formed skippable frame: the shortcut requires
    // `frameType != ZSTD_skippableFrame`, so this one must go the long way.
    let skippable = {
        let payload = corpus(Corpus::Random, 1000, 7);
        let mut v = Vec::new();
        v.extend_from_slice(&ZSTD_MAGIC_SKIPPABLE_START.to_le_bytes());
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(&payload);
        v
    };

    // (a) whole frame in one input buffer, capacity on both sides of the
    // frameContentSize threshold.
    let cases: &[(&str, &[u8], usize, usize)] = &[
        ("known/exact", &known, n, n),
        ("known/plus1", &known, n + 1, n),
        ("known/minus1", &known, n - 1, n),
        ("unknown/exact", &unknown, n, n),
        ("unknown/plus1", &unknown, n + 1, n),
        ("unknown/minus1", &unknown, n - 1, n),
        ("skippable", &skippable, 1 << 16, 0),
    ];
    for &(tag, comp, ocap, _want) in cases {
        for &whole in &[true, false] {
            // `whole == false` offers the frame minus its last byte first, so
            // the "not all the compressed bytes are available" arm is taken.
            let ichunk = if whole { usize::MAX } else { comp.len() - 1 };
            let label = format!("shortcut/{tag}/o{ocap}/whole{whole}");
            diff_bytes(&label, |l| {
                let ds = Ctx::dstream(l);
                let (steps, consumed, fin, out) = dstream_frame(l, ds.ptr, comp, ichunk.max(1), ocap);
                let sz = unsafe { l.sym::<FnSizeofPtr>("ZSTD_sizeof_DStream")(ds.ptr) };
                ((steps, consumed, fin, sz), Blob(out))
            });
        }
    }

    // (b) the hostage byte. `nextSrcSizeHint == 0` with `outEnd != outStart`
    // makes the C do `input->pos--` and set `hostageByte`, releasing it with
    // `input->pos++` on a later call, so the *input* position momentarily goes
    // backwards. That is only observable when the output buffer runs out at
    // exactly the right moment, which the 1-byte and exact-size caps below force.
    let empty_last = {
        // ZSTD_writeEpilogue emits a 3-byte empty raw last block when the frame
        // is ended with nothing left in inBuff: feed exactly one full block with
        // ZSTD_e_continue, then ZSTD_e_end with no input.
        let s = corpus(Corpus::Text, 131_072, 0x87_87);
        let f = c_frame_streamed(&s, &CSetup::lvl(3));
        let t = &f[f.len() - 3..];
        assert_eq!(t, &[0x01, 0x00, 0x00], "expected an empty raw last block, got {t:02x?}");
        (s, f)
    };
    let hostage: &[(&str, &[u8], &[u8])] = &[
        ("known", &src, &known),
        ("unknown", &src, &unknown),
        ("empty-last-block", &empty_last.0, &empty_last.1),
    ];
    for &(tag, plain, comp) in hostage {
        for &ocap in &[1usize, plain.len().max(1), plain.len() + 1, 4096] {
            for &ichunk in &[1usize, 3, 4096, usize::MAX] {
                let label = format!("hostage/{tag}/o{ocap}/c{ichunk}");
                let ((_, _, fin, _), out) = diff_bytes(&label, |l| {
                    let ds = Ctx::dstream(l);
                    let (steps, consumed, fin, o) = dstream_frame(l, ds.ptr, comp, ichunk, ocap);
                    ((steps, consumed, fin, 0usize), Blob(o))
                });
                if fin {
                    assert!(out.0[..] == plain[..], "{label}: payload");
                }
            }
        }
    }

    // (c) calling ZSTD_decompressStream again after it returned 0, and with a
    // zero-size input after a completed frame: the stream is transparently reset
    // to zdss_init, so the next call starts looking for a new frame header.
    diff("hostage/after-zero", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut ov = vec![0u8; n + 64];
        let mut steps = Vec::new();
        let mut inb = ZSTD_inBuffer {
            src: known.as_ptr() as *const c_void,
            size: known.len(),
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let r0 = res(l, unsafe { f(ds.ptr, &mut ob, &mut inb) });
        steps.push(Step {
            ret: r0,
            ip: inb.pos,
            op: ob.pos,
        });
        // again with the same (now fully consumed) input
        for _ in 0..3 {
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(ds.ptr, &mut ob, &mut inb) };
            steps.push(Step {
                ret: res(l, ret),
                ip: inb.pos,
                op: ob.pos,
            });
        }
        // and with a genuinely empty input buffer
        let one = [0u8; 1];
        for _ in 0..3 {
            let mut inb2 = ZSTD_inBuffer {
                src: one.as_ptr() as *const c_void,
                size: 0,
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { f(ds.ptr, &mut ob, &mut inb2) };
            steps.push(Step {
                ret: res(l, ret),
                ip: inb2.pos,
                op: ob.pos,
            });
        }
        Steps(steps)
    });
}

// ---------------------------------------------------------------------------
// 4c. ZSTD_c_stableInBuffer / ZSTD_c_stableOutBuffer
// ---------------------------------------------------------------------------

/// The stability violations `ZSTD_checkBufferStability` (zstd_compress.c:6325)
/// and the `zcss_init` stable-input shortcut (:6466-6470) can detect. Read those
/// two sites before adding a variant: `checkBufferStability` compares only
/// `input->src`/`input->pos` and `output->size - output->pos`, so e.g. changing
/// `input->size` or `output->dst` alone is deliberately *not* rejected.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Viol {
    /// No violation — the reference behaviour to compare the rest against.
    None,
    /// `input->src` swapped for a byte-identical second allocation -> :6333 (or
    /// :6468 while still in `zcss_init`).
    SrcPtr,
    /// `input->size` grown. NOT a violation: CONFIGS row 133(a) requires success.
    InSizeGrow,
    /// `input->size` shrunk (but kept >= `pos`). Also not checked.
    InSizeShrink,
    /// `input->pos` rewound to 0 by the caller -> :6333 / :6469.
    InPosRewind,
    /// `output->dst` swapped (bytes pre-copied so the stream is continuous).
    /// Not checked by `checkBufferStability`.
    DstPtr,
    /// `output->size` reduced by one -> :6339.
    OutShrink,
    /// `output->size` increased by one -> :6339 as well (the check is `!=`).
    OutGrow,
}

/// Drive `ZSTD_compressStream2` keeping **one** `ZSTD_inBuffer` and **one**
/// `ZSTD_outBuffer` struct alive across every call — exactly the contract
/// `ZSTD_c_stableInBuffer` / `ZSTD_c_stableOutBuffer` impose — optionally
/// injecting `viol` immediately before call number `viol_at`.
fn drive_stable(
    l: &Lib,
    src: &[u8],
    s: &CSetup,
    ocap: usize,
    dirs: &[c_int],
    viol: Viol,
    viol_at: usize,
) -> Run {
    let cs = Ctx::cstream(l);
    let setup = apply_c(l, cs.ptr, s);
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");

    // Two distinct allocations holding identical bytes, so "the src pointer
    // changed" is a pure pointer change with no data change. `+1` keeps
    // `ptr.add(len)` inside the allocation.
    let mut a = vec![0u8; src.len() + 1];
    a[..src.len()].copy_from_slice(src);
    let mut b = vec![0u8; src.len() + 1];
    b[..src.len()].copy_from_slice(src);
    let mut oa = vec![0u8; ocap + 1];
    let mut ob2 = vec![0u8; ocap + 1];
    let pa = a.as_ptr() as *const c_void;
    let pb = b.as_ptr() as *const c_void;
    let poa = oa.as_mut_ptr();
    let pob = ob2.as_mut_ptr();

    let mut inb = ZSTD_inBuffer {
        src: pa,
        size: src.len(),
        pos: 0,
    };
    let mut out = ZSTD_outBuffer {
        dst: poa as *mut c_void,
        size: ocap,
        pos: 0,
    };
    let mut active = poa;
    let mut steps: Vec<Step> = Vec::new();
    let mut finished = false;
    let mut failed = false;
    let mut stall = 0u32;
    let mut n = 0usize;

    let total = dirs.len() + 400;
    while n < total {
        let dir = if n < dirs.len() { dirs[n] } else { ZSTD_e_end };
        if n == viol_at {
            match viol {
                Viol::None => {}
                Viol::SrcPtr => inb.src = pb,
                Viol::InSizeGrow => inb.size = src.len() + 1,
                Viol::InSizeShrink => inb.size = inb.size.max(inb.pos).max(1) - 1,
                Viol::InPosRewind => inb.pos = 0,
                Viol::DstPtr => {
                    unsafe { std::ptr::copy_nonoverlapping(poa, pob, ocap + 1) };
                    out.dst = pob as *mut c_void;
                    active = pob;
                }
                Viol::OutShrink => out.size = out.size.max(out.pos + 1) - 1,
                Viol::OutGrow => out.size = ocap + 1,
            }
        }
        let ip0 = inb.pos;
        let op0 = out.pos;
        let ret = unsafe { f(cs.ptr, &mut out, &mut inb, dir) };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: inb.pos,
            op: out.pos,
        });
        if matches!(r, R::Err(..)) {
            failed = true;
            break;
        }
        if dir == ZSTD_e_end && ret == 0 {
            finished = true;
            break;
        }
        // A stall is only terminal during the drain: while the caller-supplied
        // directives are being replayed, a no-progress `ZSTD_e_continue` on an
        // empty input is the expected outcome, not a livelock.
        if inb.pos == ip0 && out.pos == op0 {
            stall += 1;
            if stall >= 3 && n >= dirs.len() {
                break;
            }
        } else {
            stall = 0;
        }
        n += 1;
    }
    let _ = failed;
    let bytes = unsafe { std::slice::from_raw_parts(active, out.pos) }.to_vec();
    ((setup, Steps(steps), inb.pos, finished), Blob(bytes))
}

/// `ZSTD_c_stableInBuffer` / `ZSTD_c_stableOutBuffer` in all four combinations,
/// then every violation `ZSTD_checkBufferStability` (zstd_compress.c:6325-6342)
/// and the `zcss_init` stable-input shortcut (:6466-6470) can detect.
#[test]
fn t_stable_in_out_buffer() {
    covers(&[
        "CFG:133",
        "CFG:134",
        "ERR:compress/zstd_compress.c:6333",
        "ERR:compress/zstd_compress.c:6339",
        "ERR:compress/zstd_compress.c:6468",
        "ERR:compress/zstd_compress.c:6469",
    ]);
    let l0 = &pair().c;

    // ---- (A) valid usage in all four (stableIn, stableOut) combinations -----
    let dir_sets: &[&[c_int]] = &[
        &[ZSTD_e_continue, ZSTD_e_continue, ZSTD_e_continue, ZSTD_e_continue],
        &[ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_continue, ZSTD_e_flush],
        &[ZSTD_e_end],
        &[ZSTD_e_flush, ZSTD_e_end],
    ];
    for &si in &[0i32, 1] {
        for &so in &[0i32, 1] {
            for &n in &[0usize, 1, 1000, 131071, 131072, 131073, 300_000] {
                for &kind in &[Corpus::Text, Corpus::Random] {
                    for (di, dirs) in dir_sets.iter().enumerate() {
                        let src = corpus(kind, n, 0x5151 + n as u64);
                        let s = CSetup::lvl(3)
                            .p(ZSTD_c_stableInBuffer, si)
                            .p(ZSTD_c_stableOutBuffer, so);
                        let ocap = compress_bound(l0, n) + 64;
                        let label = format!("stable/ok/i{si}o{so}/{kind:?}/n{n}/d{di}");
                        let ((_, _, consumed, finished), comp) = diff_bytes(&label, |l| {
                            drive_stable(l, &src, &s, ocap, dirs, Viol::None, usize::MAX)
                        });
                        assert!(finished, "{label}: frame not closed");
                        assert_eq!(consumed, n, "{label}: input consumed");
                        let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                            decompress_simple(l, &comp.0, n + 64)
                        });
                        assert_eq!(r, R::Ok(n), "{label}");
                        assert_eq!(&got.0[..], &src[..], "{label}");
                    }
                }
            }
        }
    }

    // ---- (A2) crossed with the parameters that reshape the stable path -----
    // In stable-input mode the decision to compress is `(iend - ip) <
    // zcs->blockSizeMax` (zstd_compress.c:6182) rather than the `inBuffTarget`
    // fill used by buffered mode, so `ZSTD_c_maxBlockSize` and a small
    // `ZSTD_c_windowLog` (which caps blockSizeMax at the window size) change
    // *where* the slices fall, and a small window additionally forces the
    // `_extDict` block compressors while the input buffer is the caller's.
    for &(si, so) in &[(1i32, 0i32), (0, 1), (1, 1)] {
        for &(k, v) in &[
            (ZSTD_c_maxBlockSize, 1024),
            (ZSTD_c_maxBlockSize, 2048),
            (ZSTD_c_windowLog, 10),
            (ZSTD_c_windowLog, 17),
            (ZSTD_c_strategy, ZSTD_btultra2),
            (ZSTD_c_targetCBlockSize, 1024),
        ] {
            for &n in &[1000usize, 131_072, 300_000] {
                let levels: &[c_int] = if n >= 131_072 { &[-3, 3] } else { &[-3, 3, 19] };
                for &level in levels {
                    let src = corpus(Corpus::LongRepeats, n, 0x5252 + n as u64);
                    let s = CSetup::lvl(level)
                        .p(ZSTD_c_stableInBuffer, si)
                        .p(ZSTD_c_stableOutBuffer, so)
                        .p(k, v);
                    let ocap = compress_bound(l0, n) + 64;
                    let dirs: &[c_int] =
                        &[ZSTD_e_continue, ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_continue];
                    let label = format!("stable/ok2/i{si}o{so}/p{k}v{v}/n{n}/l{level}");
                    let ((_, _, consumed, finished), comp) = diff_bytes(&label, |l| {
                        drive_stable(l, &src, &s, ocap, dirs, Viol::None, usize::MAX)
                    });
                    assert!(finished, "{label}: frame not closed");
                    assert_eq!(consumed, n, "{label}");
                    let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                        decompress_simple(l, &comp.0, n + 64)
                    });
                    assert_eq!(r, R::Ok(n), "{label}");
                    assert!(
                        got.0[..] == src[..],
                        "{label}: {}",
                        first_diff(&got.0, &src).unwrap_or_default()
                    );
                }
            }
        }
    }

    // A stableOut buffer that cannot hold one block: the `outBufferMode ==
    // ZSTD_bm_stable` arm of the zcss_load shortcut passes the tiny capacity
    // straight to ZSTD_compressEnd_public, so this is dstSize_tooSmall (70)
    // rather than a buffered partial flush.
    for &ocap in &[1usize, 16, 100] {
        for &n in &[1000usize, 300_000] {
            let src = corpus(Corpus::Text, n, 0x6161);
            let s = CSetup::lvl(3).p(ZSTD_c_stableOutBuffer, 1);
            let label = format!("stable/out-too-small/n{n}/o{ocap}");
            diff_bytes(&label, |l| {
                drive_stable(l, &src, &s, ocap, &[ZSTD_e_end], Viol::None, usize::MAX)
            });
        }
    }

    // ---- (B) every violation, at several call indices ----------------------
    const VIOLS: &[Viol] = &[
        Viol::SrcPtr,
        Viol::InSizeGrow,
        Viol::InSizeShrink,
        Viol::InPosRewind,
        Viol::DstPtr,
        Viol::OutShrink,
        Viol::OutGrow,
    ];
    let dirs: &[c_int] = &[
        ZSTD_e_continue,
        ZSTD_e_continue,
        ZSTD_e_flush,
        ZSTD_e_continue,
    ];
    for &v in VIOLS {
        for &si in &[0i32, 1] {
            for &so in &[0i32, 1] {
                // n=1000 keeps the first call inside the `totalInputSize <
                // ZSTD_BLOCKSIZE_MAX` shortcut (so the violation is caught at
                // :6468/:6469 while streamStage is still zcss_init);
                // n=300000 forces real initialisation on call 1, so the same
                // violation is instead caught by ZSTD_checkBufferStability.
                for &n in &[1000usize, 300_000] {
                    for &at in &[0usize, 1, 2] {
                        // OUT OF CONTRACT — excluded. With stableInBuffer, a
                        // caller-rewound `in.pos` is only rejected by the guard
                        // at zstd_compress.c:6469, which is reached exclusively
                        // when `endOp == ZSTD_e_continue` (and the frame is still
                        // in `zcss_init` with `stableIn_notConsumed != 0`). For
                        // any other endOp, ZSTD_compressStream_generic runs
                        //   assert(input->pos >= zcs->stableIn_notConsumed);
                        //   input->pos -= zcs->stableIn_notConsumed;   /* :6121 */
                        //   if (ip) ip -= zcs->stableIn_notConsumed;   /* :6122 */
                        // and that `assert` is the ONLY precondition — compiled
                        // out at DEBUGLEVEL=0. `ip` then points
                        // stableIn_notConsumed bytes BEFORE the caller's buffer
                        // and the compressor reads whatever precedes it.
                        // Verified by re-running this exact case with 4096 bytes
                        // of deterministic 0xAB padding placed immediately before
                        // the offered buffer: C and Rust then agree byte for byte,
                        // i.e. the only difference was the heap contents preceding
                        // each `.so`'s allocation. There is no C behaviour to match.
                        if v == Viol::InPosRewind
                            && si == 1
                            && n < ZSTD_BLOCKSIZE_MAX
                            && dirs[at] != ZSTD_e_continue
                        {
                            continue;
                        }
                        let src = corpus(Corpus::Text, n, 0x7171);
                        let s = CSetup::lvl(3)
                            .p(ZSTD_c_stableInBuffer, si)
                            .p(ZSTD_c_stableOutBuffer, so);
                        let ocap = compress_bound(l0, n) + 64;
                        let label = format!("stable/viol/{v:?}/i{si}o{so}/n{n}/at{at}");
                        diff_bytes(&label, |l| {
                            drive_stable(l, &src, &s, ocap, dirs, v, at)
                        });
                    }
                }
            }
        }
    }

    // ---- (C) the exact code for each violation the guards DO detect ---------
    // Everything above only asserts C == RUST; this block additionally pins
    // *which* error, so a translation that agreed on the wrong error would fail.
    let cases: &[(Viol, c_int, c_int, usize, &str)] = &[
        // mid-frame: caught by ZSTD_checkBufferStability
        (Viol::SrcPtr, 1, 0, 300_000, "zstd_compress.c:6333"),
        (Viol::InPosRewind, 1, 0, 300_000, "zstd_compress.c:6333"),
        // still zcss_init (input smaller than one block): caught by the shortcut
        (Viol::SrcPtr, 1, 0, 1000, "zstd_compress.c:6468"),
        (Viol::InPosRewind, 1, 0, 1000, "zstd_compress.c:6469"),
        // output size changed either way
        (Viol::OutShrink, 0, 1, 1000, "zstd_compress.c:6339"),
        (Viol::OutGrow, 0, 1, 1000, "zstd_compress.c:6339"),
        (Viol::OutShrink, 0, 1, 300_000, "zstd_compress.c:6339"),
        (Viol::OutGrow, 0, 1, 300_000, "zstd_compress.c:6339"),
        (Viol::OutShrink, 1, 1, 300_000, "zstd_compress.c:6339"),
    ];
    let cont3: &[c_int] = &[ZSTD_e_continue, ZSTD_e_continue, ZSTD_e_continue];
    for &(v, si, so, n, site) in cases {
        let src = corpus(Corpus::Text, n, 0x7171);
        let s = CSetup::lvl(3)
            .p(ZSTD_c_stableInBuffer, si)
            .p(ZSTD_c_stableOutBuffer, so);
        let ocap = compress_bound(l0, n) + 64;
        let label = format!("stable/exact/{v:?}/i{si}o{so}/n{n}");
        let ((_, steps, _, _), _) =
            diff_bytes(&label, |l| drive_stable(l, &src, &s, ocap, cont3, v, 1));
        let codes: Vec<c_int> = steps
            .0
            .iter()
            .filter_map(|st| match &st.ret {
                R::Err(c, _) => Some(*c),
                _ => None,
            })
            .collect();
        assert_eq!(
            codes,
            vec![50],
            "{label}: expected stabilityCondition_notRespected (50) from {site}, got {steps:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4d. ZSTD_d_windowLogMax / ZSTD_DCtx_setMaxWindowSize
// ---------------------------------------------------------------------------

/// The window-size limit and its **streaming-only** asymmetry.
///
/// `RETURN_ERROR_IF(MAX(fParams.windowSize, 1<<10) > zds->maxWindowSize,
/// frameParameter_windowTooLarge)` lives in `ZSTD_decompressStream` at
/// zstd_decompress.c:2230-2232 and nowhere else, so the *same* frame that the
/// streaming decoder rejects must decode successfully through the one-shot
/// `ZSTD_decompressDCtx`. Both spellings of the limit are swept:
/// `ZSTD_DCtx_setParameter(ZSTD_d_windowLogMax, n)` (which maps 0 to 27 at
/// :1911 then `CHECK_DBOUNDS(10..31)`) and `ZSTD_DCtx_setMaxWindowSize(bytes)`
/// (:1804-1814, which accepts any byte count in `[1<<10, 1<<31]`, powers of two
/// or not).
#[test]
fn t_d_window_log_max() {
    covers(&[
        "CFG:67",
        "CFG:84",
        "CFG:313",
        "CFG:314",
        "ERR:decompress/zstd_decompress.c:2231",
        "ERR:decompress/zstd_decompress.c:1809",
        "ERR:decompress/zstd_decompress.c:1810",
        "ERR:decompress/zstd_decompress.c:1811",
    ]);
    let n = 300_000usize;
    let src = corpus(Corpus::LongRepeats, n, 0x8181);
    let sched: Sched = vec![(4096, 1 << 16, 0); 8];

    for &wl in &[10i32, 17, 20, 27] {
        let comp = c_frame_streamed(&src, &CSetup::lvl(3).p(ZSTD_c_windowLog, wl));
        // Pin the fixture: without the streamed construction the header would
        // carry a *different* windowSize and the whole sweep would be vacuous.
        let h = c_header(&comp);
        assert_eq!(
            h.windowSize,
            1u64 << wl,
            "fixture windowLog {wl}: header says windowSize={}",
            h.windowSize
        );
        assert_eq!(h.frameContentSize, ZSTD_CONTENTSIZE_UNKNOWN);

        // ---- (a) ZSTD_DCtx_setParameter(ZSTD_d_windowLogMax, n) -------------
        for &wlm in &[0i32, 10, 17, 20, 27, 31] {
            let d = DSetup::default().p(ZSTD_d_windowLogMax, wlm);
            let eff = 1u64 << if wlm == 0 { 27 } else { wlm };
            let want_fail = (1u64 << wl).max(1 << 10) > eff;
            let label = format!("wlm/param/wl{wl}/max{wlm}");
            let ((_, steps, _, finished), out) =
                diff_bytes(&label, |l| drive_ds(l, &comp, &d, &sched, 1 << 16));
            let codes: Vec<c_int> = steps
                .0
                .iter()
                .filter_map(|st| match &st.ret {
                    R::Err(c, _) => Some(*c),
                    _ => None,
                })
                .collect();
            if want_fail {
                assert_eq!(
                    codes,
                    vec![16],
                    "{label}: expected frameParameter_windowTooLarge, got {steps:?}"
                );
            } else {
                assert!(codes.is_empty(), "{label}: unexpected error {steps:?}");
                assert!(finished, "{label}: frame not completed");
                assert!(out.0[..] == src[..], "{label}: payload");
            }

            // The very same frame, the very same limit, through the one-shot
            // entry point: the check is absent there, so this must ALWAYS pass.
            let label1 = format!("wlm/param/wl{wl}/max{wlm}/oneshot");
            let (r1, got1) = diff_bytes(&label1, |l| {
                let dctx = Ctx::dctx(l);
                let setup = apply_d(l, dctx.ptr, &d);
                let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
                let mut dst = vec![0u8; n + 64];
                let ret = unsafe {
                    f(
                        dctx.ptr,
                        dst.as_mut_ptr() as *mut c_void,
                        dst.len(),
                        comp.as_ptr() as *const c_void,
                        comp.len(),
                    )
                };
                let r = res(l, ret);
                if let R::Ok(k) = r {
                    dst.truncate(k);
                }
                ((setup, r), Blob(dst))
            });
            assert_eq!(
                r1.1,
                R::Ok(n),
                "{label1}: the one-shot path must ignore maxWindowSize"
            );
            assert!(got1.0[..] == src[..], "{label1}: payload");
        }

        // ---- (b) ZSTD_DCtx_setMaxWindowSize(bytes), incl. non-powers-of-two --
        const BYTES: &[usize] = &[
            0,
            1023,
            1024,
            1025,
            100_000,
            1 << 17,
            (1 << 17) + 1,
            (1 << 20) + 12_345,
            (1 << 27) - 1,
            1 << 27,
            (1 << 27) + 1,
            1usize << 31,
            (1usize << 31) + 1,
        ];
        for &bytes in BYTES {
            let d = DSetup::default().win(bytes);
            let label = format!("wlm/bytes/wl{wl}/w{bytes}");
            // `ZSTD_DCtx_setMaxWindowSize` rejects `< 1<<10` and `> 1<<31`
            // (:1810/:1811) and then leaves maxWindowSize at its default, so the
            // effective limit for those inputs is ZSTD_MAXWINDOWSIZE_DEFAULT.
            let rejected = bytes < 1024 || bytes > (1usize << 31);
            let eff = if rejected {
                (1u64 << 27) + 1
            } else {
                bytes as u64
            };
            let want_fail = (1u64 << wl).max(1 << 10) > eff;
            let ((setup, steps, _, finished), out) =
                diff_bytes(&label, |l| drive_ds(l, &comp, &d, &sched, 1 << 16));
            assert_eq!(
                matches!(setup[0], R::Err(42, _)),
                rejected,
                "{label}: setMaxWindowSize acceptance, got {setup:?}"
            );
            let codes: Vec<c_int> = steps
                .0
                .iter()
                .filter_map(|st| match &st.ret {
                    R::Err(c, _) => Some(*c),
                    _ => None,
                })
                .collect();
            if want_fail {
                assert_eq!(codes, vec![16], "{label}: expected 16, got {steps:?}");
            } else {
                assert!(codes.is_empty(), "{label}: unexpected error {steps:?}");
                assert!(finished, "{label}: not finished");
                assert!(out.0[..] == src[..], "{label}: payload");
            }
        }
    }

    // ---- (c) both setters are frozen mid-stream ----------------------------
    // `ZSTD_DCtx_setMaxWindowSize` starts with
    // `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong)`
    // (zstd_decompress.c:1809), which is checked BEFORE the bounds, so even a
    // perfectly valid byte count is refused with 60 (not 42) mid-frame.
    let comp = c_frame(&src, &CSetup::lvl(3));
    diff("wlm/midstream", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut ov = vec![0u8; 4096];
        let mut inb = ZSTD_inBuffer {
            src: comp.as_ptr() as *const c_void,
            size: 64,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let r0 = res(l, unsafe { f(ds.ptr, &mut ob, &mut inb) });
        let smw = l.sym::<FnSetMaxWindowSize>("ZSTD_DCtx_setMaxWindowSize");
        let setp = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
        let mid = (
            res(l, unsafe { smw(ds.ptr, 1 << 20) }), // valid size, wrong stage -> 60
            res(l, unsafe { smw(ds.ptr, 1) }),       // invalid size, wrong stage -> 60
            res(l, unsafe { setp(ds.ptr, ZSTD_d_windowLogMax, 20) }),
        );
        // After a session reset both are accepted again.
        let rst = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
        let r1 = res(l, unsafe { rst(ds.ptr, ZSTD_reset_session_only) });
        let after = (
            res(l, unsafe { smw(ds.ptr, 1 << 20) }),
            res(l, unsafe { setp(ds.ptr, ZSTD_d_windowLogMax, 20) }),
        );
        (r0, mid, r1, after)
    });
}

// ---------------------------------------------------------------------------
// 4e. ZSTD_d_stableOutBuffer / forceIgnoreChecksum / disableHuffmanAssembly /
//     maxBlockSize
// ---------------------------------------------------------------------------

/// The out-buffer changes `ZSTD_checkOutBuffer` (zstd_decompress.c:2035-2051)
/// distinguishes. Unlike the compressor's `ZSTD_checkBufferStability`, this one
/// demands `dst`, `pos` **and** `size` all match, so all three are violations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DViol {
    None,
    /// `output->dst` swapped (bytes pre-copied so the stream stays continuous).
    DstPtr,
    /// `output->size` reduced by one.
    SizeShrink,
    /// `output->pos` rewound to 0 by the caller.
    PosRewind,
}

/// Drive `ZSTD_decompressStream` keeping **one** `ZSTD_outBuffer` struct alive
/// across every call, as `ZSTD_d_stableOutBuffer` requires, optionally injecting
/// `viol` before call `viol_at`.
fn drive_dstable(
    l: &Lib,
    comp: &[u8],
    s: &DSetup,
    ocap: usize,
    ichunk: usize,
    viol: DViol,
    viol_at: usize,
) -> Run {
    let ds = Ctx::dstream(l);
    let setup = apply_d(l, ds.ptr, s);
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut holder = vec![0u8; comp.len() + 1];
    holder[..comp.len()].copy_from_slice(comp);
    let mut oa = vec![0u8; ocap + 1];
    let mut ob2 = vec![0u8; ocap + 1];
    let poa = oa.as_mut_ptr();
    let pob = ob2.as_mut_ptr();

    let mut out = ZSTD_outBuffer {
        dst: poa as *mut c_void,
        size: ocap,
        pos: 0,
    };
    let mut active = poa;
    let mut consumed = 0usize;
    let mut steps: Vec<Step> = Vec::new();
    let mut finished = false;
    let mut stall = 0u32;

    // Enough calls to drain `comp` even at one byte per call, plus slack for the
    // flush-only and hostage-byte calls at the end of the frame.
    let iters = comp.len() / ichunk.max(1) + 4096;
    for n in 0..iters {
        if n == viol_at {
            match viol {
                DViol::None => {}
                DViol::DstPtr => {
                    unsafe { std::ptr::copy_nonoverlapping(poa, pob, ocap + 1) };
                    out.dst = pob as *mut c_void;
                    active = pob;
                }
                DViol::SizeShrink => out.size = out.size.max(out.pos + 1) - 1,
                DViol::PosRewind => out.pos = 0,
            }
        }
        let ilen = ichunk.min(comp.len() - consumed);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let op0 = out.pos;
        let ret = unsafe { f(ds.ptr, &mut out, &mut inb) };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: inb.pos,
            op: out.pos,
        });
        consumed += inb.pos;
        if matches!(r, R::Err(..)) {
            break;
        }
        if ret == 0 {
            finished = true;
            break;
        }
        if inb.pos == 0 && out.pos == op0 {
            stall += 1;
            if stall >= 3 {
                break;
            }
        } else {
            stall = 0;
        }
    }
    let bytes = unsafe { std::slice::from_raw_parts(active, out.pos) }.to_vec();
    ((setup, Steps(steps), consumed, finished), Blob(bytes))
}

/// `ZSTD_d_stableOutBuffer`, `ZSTD_d_forceIgnoreChecksum`,
/// `ZSTD_d_disableHuffmanAssembly` and `ZSTD_d_maxBlockSize` on the streaming
/// decoder.
///
/// Targets `ZSTD_checkOutBuffer` (zstd_decompress.c:2049), the `ZSTD_bm_stable`
/// "output too small for a known frameContentSize" guard (:2205), the
/// `ZSTDds_checkChecksum` stage (:1400-1408) and `forceIgnoreChecksum`'s
/// suppression of the XXH64 update, the `HUF_flags_disableAsm` plumbing (a no-op
/// because `ZSTD_ENABLE_ASM_X86_64_BMI2 == 0` in this build), and
/// `fParams.blockSizeMax = MIN(blockSizeMax, maxBlockSizeParam)` (:2233-2234).
#[test]
fn t_dstream_decode_params() {
    covers(&[
        "CFG:94",
        "CFG:311",
        "CFG:312",
        "CFG:315",
        "CFG:316",
        "CFG:318",
        "CFG:319",
        "ERR:decompress/zstd_decompress.c:2049",
        "ERR:decompress/zstd_decompress.c:2111",
        "ERR:decompress/zstd_decompress.c:2209",
        "ERR:decompress/zstd_decompress.c:1406",
        "ERR:decompress/zstd_decompress.c:1050",
        "ERR:decompress/zstd_decompress.c:1055",
        "ERR:decompress/zstd_decompress.c:1908",
        "ERR:decompress/zstd_decompress.c:1957",
    ]);

    // ---- (a) ZSTD_d_stableOutBuffer -------------------------------------
    for &n in &[0usize, 1, 1000, 131072, 300_000] {
        let src = corpus(Corpus::Text, n, 0x9191);
        // Known content size (contentSizeFlag defaults to 1 through compress2)
        // and unknown content size (streamed fixture, contentSizeFlag forced 0
        // by zstd_compress.c:2197).
        let known = c_frame(&src, &CSetup::lvl(3));
        let unknown = c_frame_streamed(&src, &CSetup::lvl(3));
        assert_eq!(c_header(&unknown).frameContentSize, ZSTD_CONTENTSIZE_UNKNOWN);
        for (tag, comp) in [("known", &known), ("unknown", &unknown)] {
            for &so in &[0i32, 1] {
                for &ichunk in &[1usize, 7, 4096, usize::MAX] {
                    let d = DSetup::default().p(ZSTD_d_stableOutBuffer, so);
                    // Exactly frameContentSize: the smallest buffer the stable
                    // path accepts for a known content size.
                    let label = format!("dstable/ok/{tag}/o{so}/c{ichunk}/n{n}");
                    let ((_, _, _, finished), out) = diff_bytes(&label, |l| {
                        drive_dstable(l, comp, &d, n.max(1), ichunk, DViol::None, usize::MAX)
                    });
                    assert!(finished, "{label}: not finished");
                    assert!(out.0[..] == src[..], "{label}: payload");
                }
            }
        }

        // An out buffer one byte short of a KNOWN frameContentSize: rejected up
        // front in zdss_loadHeader with dstSize_tooSmall (70) only when stable.
        if n > 0 {
            for &so in &[0i32, 1] {
                let d = DSetup::default().p(ZSTD_d_stableOutBuffer, so);
                let label = format!("dstable/short/o{so}/n{n}");
                diff_bytes(&label, |l| {
                    drive_dstable(l, &known, &d, n - 1, usize::MAX, DViol::None, usize::MAX)
                });
                // Same short buffer, but the frame's content size is UNKNOWN:
                // zdss_loadHeader deliberately does not check it, so the failure
                // (if any) surfaces later and from a different site.
                let label = format!("dstable/short-unknown/o{so}/n{n}");
                diff_bytes(&label, |l| {
                    drive_dstable(l, &unknown, &d, n - 1, usize::MAX, DViol::None, usize::MAX)
                });
            }
        }
    }

    // Every ZSTD_checkOutBuffer violation, at several call indices.
    let n = 300_000usize;
    let src = corpus(Corpus::Text, n, 0x9191);
    let known = c_frame(&src, &CSetup::lvl(3));
    for &v in &[DViol::DstPtr, DViol::SizeShrink, DViol::PosRewind] {
        for &so in &[0i32, 1] {
            for &at in &[0usize, 1, 2, 5, 8] {
                let d = DSetup::default().p(ZSTD_d_stableOutBuffer, so);
                let label = format!("dstable/viol/{v:?}/o{so}/at{at}");
                let ((_, steps, _, _), _) = diff_bytes(&label, |l| {
                    drive_dstable(l, &known, &d, n + 64, 4096, v, at)
                });
                let codes: Vec<c_int> = steps
                    .0
                    .iter()
                    .filter_map(|st| match &st.ret {
                        R::Err(c, _) => Some(*c),
                        _ => None,
                    })
                    .collect();
                // `ZSTD_checkOutBuffer` returns 0 while streamStage == zdss_init,
                // so a change applied before the very first call is legal. And
                // rewinding `pos` is only observable once `pos` has moved: the
                // early calls of this schedule only fill the internal buffer.
                let observable = match v {
                    DViol::DstPtr | DViol::SizeShrink => true,
                    DViol::PosRewind => {
                        at > 0 && steps.0.get(at - 1).map(|s| s.op > 0).unwrap_or(false)
                    }
                    DViol::None => false,
                };
                if so == 1 && at > 0 && observable {
                    assert_eq!(
                        codes,
                        vec![104],
                        "{label}: expected dstBuffer_wrong, got {steps:?}"
                    );
                }
            }
        }
    }

    // ---- (b) ZSTD_d_forceIgnoreChecksum ---------------------------------
    for &n in &[0usize, 1, 1000, 300_000] {
        let src = corpus(Corpus::Text, n, 0xA1A1);
        let good = c_frame(&src, &CSetup::lvl(3).p(ZSTD_c_checksumFlag, 1));
        let mut bad = good.clone();
        let k = bad.len() - 1;
        bad[k] ^= 1; // flip the top byte of the stored XXH64 low word
        let trunc = good[..good.len() - 1].to_vec();
        for &ig in &[0i32, 1] {
            for &ichunk in &[1usize, 4096, usize::MAX] {
                let d = DSetup::default().p(ZSTD_d_forceIgnoreChecksum, ig);
                let sched: Sched = vec![(ichunk, 1 << 16, 0); 12];

                let lg = format!("cks/good/i{ig}/c{ichunk}/n{n}");
                let ((_, _, _, fin), out) =
                    diff_bytes(&lg, |l| drive_ds(l, &good, &d, &sched, 1 << 16));
                assert!(fin, "{lg}: a correct checksum must verify");
                assert!(out.0[..] == src[..], "{lg}");

                let lb = format!("cks/bad/i{ig}/c{ichunk}/n{n}");
                let ((_, steps, _, fin), out) =
                    diff_bytes(&lb, |l| drive_ds(l, &bad, &d, &sched, 1 << 16));
                let codes: Vec<c_int> = steps
                    .0
                    .iter()
                    .filter_map(|st| match &st.ret {
                        R::Err(c, _) => Some(*c),
                        _ => None,
                    })
                    .collect();
                if ig == 1 {
                    assert!(fin, "{lb}: ignoreChecksum must accept the frame");
                    assert!(codes.is_empty(), "{lb}: {steps:?}");
                    assert!(out.0[..] == src[..], "{lb}");
                } else {
                    assert_eq!(codes, vec![22], "{lb}: expected checksum_wrong, {steps:?}");
                }

                // Only three of the four checksum bytes present.
                let lt = format!("cks/trunc/i{ig}/c{ichunk}/n{n}");
                diff_bytes(&lt, |l| drive_ds(l, &trunc, &d, &sched, 1 << 16));
            }

            // The same three frames through the ONE-SHOT path, where the
            // checksum is verified by ZSTD_decompressFrame instead: a mismatch
            // is zstd_decompress.c:1055 and a short tail is :1050 (which reports
            // checksum_wrong, not srcSize_wrong).
            let d1 = DSetup::default().p(ZSTD_d_forceIgnoreChecksum, ig);
            for (tag, buf) in [("good", &good), ("bad", &bad), ("trunc", &trunc)] {
                let lo = format!("cks/oneshot/{tag}/i{ig}/n{n}");
                let (r, got) = diff_bytes(&lo, |l| {
                    let dctx = Ctx::dctx(l);
                    let setup = apply_d(l, dctx.ptr, &d1);
                    for s in &setup {
                        assert!(matches!(s, R::Ok(_)), "{s:?}");
                    }
                    let f = l.sym::<FnDecompressDCtx>("ZSTD_decompressDCtx");
                    let mut dst = vec![0u8; n + 64];
                    let ret = unsafe {
                        f(
                            dctx.ptr,
                            dst.as_mut_ptr() as *mut c_void,
                            dst.len(),
                            buf.as_ptr() as *const c_void,
                            buf.len(),
                        )
                    };
                    let r = res(l, ret);
                    if let R::Ok(k) = r {
                        dst.truncate(k);
                    }
                    (r, Blob(dst))
                });
                if tag == "good" || (tag == "bad" && ig == 1) {
                    assert_eq!(r, R::Ok(n), "{lo}");
                    assert_eq!(&got.0[..], &src[..], "{lo}");
                } else if tag == "bad" {
                    assert_eq!(r, R::Err(22, "Restored data doesn't match checksum".into()), "{lo}");
                }
            }
        }
    }

    // ---- (b2) every dParam is frozen mid-stream ---------------------------
    // `ZSTD_DCtx_setParameter` opens with an unconditional
    // `streamStage != zdss_init -> stage_wrong` (zstd_decompress.c:1908); there
    // is no `ZSTD_isUpdateAuthorized` equivalent on the decode side, so ALL
    // seven parameters are refused. `ZSTD_DCtx_reset(ZSTD_reset_parameters)`
    // has the same guard (:1957) while its session arm never fails.
    let mid = c_frame(&corpus(Corpus::Text, 300_000, 0x3131), &CSetup::lvl(3));
    diff("dparams/midstream", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let setp = l.sym::<FnDCtxSetParameter>("ZSTD_DCtx_setParameter");
        let rst = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
        let mut ov = vec![0u8; 4096];
        // Fresh: every parameter is accepted.
        let before: Vec<R> = ALL_DPARAMS
            .iter()
            .map(|&(_, p)| res(l, unsafe { setp(ds.ptr, p, if p == ZSTD_d_maxBlockSize { 2048 } else { 1 }) }))
            .collect();
        let mut inb = ZSTD_inBuffer {
            src: mid.as_ptr() as *const c_void,
            size: 64,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let r0 = res(l, unsafe { f(ds.ptr, &mut ob, &mut inb) });
        // Mid-frame: every parameter must be refused with stage_wrong (60).
        let during: Vec<R> = ALL_DPARAMS
            .iter()
            .map(|&(_, p)| res(l, unsafe { setp(ds.ptr, p, if p == ZSTD_d_maxBlockSize { 2048 } else { 1 }) }))
            .collect();
        let resets = (
            res(l, unsafe { rst(ds.ptr, ZSTD_reset_parameters) }),
            res(l, unsafe { rst(ds.ptr, 0) }),
            res(l, unsafe { rst(ds.ptr, 4) }),
            res(l, unsafe { rst(ds.ptr, ZSTD_reset_session_only) }),
        );
        // After the session reset the parameters are writable again.
        let after: Vec<R> = ALL_DPARAMS
            .iter()
            .map(|&(_, p)| res(l, unsafe { setp(ds.ptr, p, if p == ZSTD_d_maxBlockSize { 2048 } else { 1 }) }))
            .collect();
        (before, r0, during, resets, after)
    });

    // ---- (c) ZSTD_d_disableHuffmanAssembly (a no-op in this build) --------
    // Both settings must produce byte-identical output. Compared across
    // libraries AND against each other, over inputs that force 4-stream, 1-stream
    // and set_repeat literal sections.
    for &kind in &[Corpus::Text, Corpus::Sparse, Corpus::SmallAlphabet] {
        for &n in &[1000usize, 200_000] {
            let src = corpus(kind, n, 0xB1B1);
            let comp = c_frame(&src, &CSetup::lvl(9));
            let sched: Sched = vec![(4096, 1 << 16, 0); 20];
            let mut outs = Vec::new();
            for &dis in &[0i32, 1] {
                let d = DSetup::default().p(ZSTD_d_disableHuffmanAssembly, dis);
                let label = format!("hufasm/{kind:?}/n{n}/d{dis}");
                let (_, out) = diff_bytes(&label, |l| drive_ds(l, &comp, &d, &sched, 1 << 16));
                assert!(out.0[..] == src[..], "{label}");
                outs.push(out.0);
            }
            assert_eq!(
                outs[0], outs[1],
                "disableHuffmanAssembly changed the output for {kind:?}/n{n}"
            );
        }
    }

    // ---- (d) ZSTD_d_maxBlockSize at its bounds and one past ---------------
    let bounds = {
        let l = &pair().c;
        let f = l.sym::<FnDParamGetBounds>("ZSTD_dParam_getBounds");
        unsafe { f(ZSTD_d_maxBlockSize) }
    };
    assert_eq!(bounds.error, 0);
    let lo = bounds.lowerBound;
    let hi = bounds.upperBound;
    let src = corpus(Corpus::Text, 400_000, 0xC1C1);
    for &cmax in &[1024i32, 2048, 131072] {
        let comp = c_frame(&src, &CSetup::lvl(3).p(ZSTD_c_maxBlockSize, cmax));
        for &dmax in &[0i32, lo - 1, lo, lo + 1, hi - 1, hi, hi + 1] {
            let d = DSetup::default().p(ZSTD_d_maxBlockSize, dmax);
            let sched: Sched = vec![(4096, 1 << 16, 0); 40];
            let label = format!("dmaxblk/c{cmax}/d{dmax}");
            let ((setup, _, _, _), _) =
                diff_bytes(&label, |l| drive_ds(l, &comp, &d, &sched, 1 << 16));
            // `ZSTD_d_maxBlockSize` skips CHECK_DBOUNDS entirely when value == 0.
            let want_rejected = dmax != 0 && (dmax < lo || dmax > hi);
            assert_eq!(
                matches!(setup[0], R::Err(42, _)),
                want_rejected,
                "{label}: setter acceptance, got {setup:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4f. ZSTD_getFrameProgression / ZSTD_toFlushNow after every step
// ---------------------------------------------------------------------------

/// One streaming call plus the full observable progression state after it.
#[derive(Clone, PartialEq, Eq)]
struct ProgStep {
    ret: R,
    ip: SizeT,
    op: SizeT,
    fp: ZSTD_frameProgression,
    flush: SizeT,
}

impl fmt::Debug for ProgStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?} i{} o{} ing{} con{} prod{} flu{} job{} wk{} tfn{}]",
            self.ret,
            self.ip,
            self.op,
            self.fp.ingested,
            self.fp.consumed,
            self.fp.produced,
            self.fp.flushed,
            self.fp.currentJobID,
            self.fp.nbActiveWorkers,
            self.flush
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
struct ProgSteps(Vec<ProgStep>);

impl fmt::Debug for ProgSteps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0.len();
        write!(f, "ProgSteps(n={n}")?;
        if n <= 64 {
            for (i, s) in self.0.iter().enumerate() {
                write!(f, " {i}:{s:?}")?;
            }
        } else {
            for i in 0..32 {
                write!(f, " {i}:{:?}", self.0[i])?;
            }
            write!(f, " ..")?;
            for i in n - 32..n {
                write!(f, " {i}:{:?}", self.0[i])?;
            }
        }
        write!(f, ")")
    }
}

/// `drive_cs2` plus a `ZSTD_getFrameProgression` + `ZSTD_toFlushNow` sample
/// taken *before* the first call and after **every** call.
fn drive_cs2_prog(
    l: &Lib,
    src: &[u8],
    s: &CSetup,
    sched: &Sched,
    drain_cap: usize,
) -> ((Vec<R>, ProgSteps, usize, bool), Blob) {
    let cs = Ctx::cstream(l);
    let setup = apply_c(l, cs.ptr, s);
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let fp = l.sym::<FnFrameProgression>("ZSTD_getFrameProgression");
    let tfn = l.sym::<FnSizeofPtr>("ZSTD_toFlushNow");
    let mut holder = vec![0u8; src.len() + 1];
    holder[..src.len()].copy_from_slice(src);

    let mut steps: Vec<ProgStep> = Vec::new();
    // Sample 0: the state of a freshly created / freshly reset CStream.
    steps.push(ProgStep {
        ret: R::Ok(SizeT::MAX),
        ip: 0,
        op: 0,
        fp: unsafe { fp(cs.ptr) },
        flush: unsafe { tfn(cs.ptr) },
    });

    let mut consumed = 0usize;
    let mut outall: Vec<u8> = Vec::new();
    let mut finished = false;
    let mut end_limit: Option<usize> = None;
    let mut n = 0usize;
    let total = sched.len() + 8000;
    while n < total {
        let (ichunk, ocap, dir) = if n < sched.len() {
            sched[n]
        } else {
            (usize::MAX, drain_cap, ZSTD_e_end)
        };
        let avail = match end_limit {
            Some(lim) => lim - consumed,
            None => src.len() - consumed,
        };
        let ilen = ichunk.min(avail);
        if dir == ZSTD_e_end && end_limit.is_none() {
            end_limit = Some(consumed + ilen);
        }
        let ocap = ocap.max(1);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ov = vec![0u8; ocap];
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ocap,
            pos: 0,
        };
        let ret = unsafe { f(cs.ptr, &mut ob, &mut inb, dir) };
        let r = res(l, ret);
        let err = matches!(r, R::Err(..));
        steps.push(ProgStep {
            ret: r,
            ip: inb.pos,
            op: ob.pos,
            fp: unsafe { fp(cs.ptr) },
            flush: unsafe { tfn(cs.ptr) },
        });
        outall.extend_from_slice(&ov[..ob.pos]);
        consumed += inb.pos;
        if err {
            break;
        }
        if dir == ZSTD_e_end && ret == 0 {
            finished = true;
            break;
        }
        n += 1;
    }
    ((setup, ProgSteps(steps), consumed, finished), Blob(outall))
}

/// `ZSTD_getFrameProgression` (zstd_compress.c:7620-7638) and `ZSTD_toFlushNow`
/// (:7660-7672) sampled after every `ZSTD_compressStream2` call. In this
/// single-threaded build the C computes
/// `buffered = (inBuff == NULL) ? 0 : inBuffPos - inToCompress`,
/// `ingested = consumedSrcSize + buffered`, `consumed = consumedSrcSize`,
/// `produced == flushed == producedCSize`, `currentJobID == nbActiveWorkers == 0`
/// and `ZSTD_toFlushNow` returns 0 unconditionally — every one of those six
/// fields plus `toFlushNow` is compared here.
#[test]
fn t_frame_progression() {
    covers(&["CFG:237", "CFG:64"]);
    let cout = cstream_out_size();
    let mut rng = Rng::new(0x5723_0003);

    // A fresh, never-used CStream and a never-used CCtx.
    diff("prog/fresh", |l| {
        let cs = Ctx::cstream(l);
        let cctx = Ctx::cctx(l);
        let fp = l.sym::<FnFrameProgression>("ZSTD_getFrameProgression");
        let tfn = l.sym::<FnSizeofPtr>("ZSTD_toFlushNow");
        (
            unsafe { fp(cs.ptr) },
            unsafe { tfn(cs.ptr) },
            unsafe { fp(cctx.ptr) },
            unsafe { tfn(cctx.ptr) },
        )
    });

    // Hand-picked schedules that walk the interesting progression states:
    // a zero-capacity output (input buffered but nothing produced), partial
    // flushes, and the transition through the final ZSTD_e_end.
    let fixed: &[&[(usize, usize, c_int)]] = &[
        &[(1000, 0, ZSTD_e_continue), (1000, 0, ZSTD_e_flush)],
        &[(usize::MAX, 0, ZSTD_e_flush), (usize::MAX, 17, ZSTD_e_flush)],
        &[(4096, 1, ZSTD_e_continue), (4096, 1, ZSTD_e_flush)],
        &[(usize::MAX, 1 << 20, ZSTD_e_end)],
        &[(65536, 100, ZSTD_e_continue), (65536, 100, ZSTD_e_flush), (65536, 100, ZSTD_e_end)],
        &[(131072, 131072, ZSTD_e_flush), (0, 131072, ZSTD_e_end)],
    ];
    for (i, sched) in fixed.iter().enumerate() {
        for &n in &[0usize, 1, 1000, 131072, 300_000] {
            for &kind in &[Corpus::Text, Corpus::Random] {
                let src = corpus(kind, n, 0xD1D1);
                let sc: Sched = sched.to_vec();
                let label = format!("prog/fixed{i}/{kind:?}/n{n}");
                let ((_, _, consumed, finished), comp) = diff_bytes(&label, |l| {
                    drive_cs2_prog(l, &src, &CSetup::lvl(3), &sc, cout)
                });
                assert!(finished, "{label}");
                // `consumed` can be < n: once a schedule step carries
                // ZSTD_e_end, ZSTD_CCtx_init_compressStream2 turns that call's
                // input size into pledgedSrcSize, so the driver may not offer
                // more afterwards (see drive_cs2's `end_limit`).
                let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                    decompress_simple(l, &comp.0, consumed + 64)
                });
                assert_eq!(r, R::Ok(consumed), "{label}");
                assert_eq!(&got.0[..], &src[..consumed], "{label}");
            }
        }
    }

    // Randomized schedules: the progression is sampled after every call, so any
    // divergence in the internal buffer accounting shows up immediately rather
    // than only in the final bytes.
    for trial in 0..60usize {
        let kind = *rng.pick(ALL_CORPORA);
        let size = *rng.pick(&[0usize, 1, 100, 4096, 65536, 131072, 200_000, 400_000]);
        let level = *rng.pick(&[-3i32, 1, 3, 9, 19]);
        let src = corpus(kind, size, 0xE1E1 + trial as u64);
        let caps = caps_for(size);
        let nsteps = 1 + rng.below(12);
        let sched = gen_sched(&mut rng, nsteps, &caps);
        let label = format!("prog/rand/{trial}/{kind:?}/n{size}/l{level}");
        let s = CSetup::lvl(level);
        let ((_, _, consumed, finished), comp) =
            diff_bytes(&label, |l| drive_cs2_prog(l, &src, &s, &sched, cout));
        if finished {
            let (r, _) = diff_bytes(&format!("{label}/rt"), |l| {
                decompress_simple(l, &comp.0, consumed + 64)
            });
            assert_eq!(r, R::Ok(consumed), "{label}");
        }
    }

    // ZSTD_getFrameProgression / ZSTD_toFlushNow on a CCtx driven by the
    // low-level ZSTD_compressBegin + ZSTD_compressContinue + ZSTD_compressEnd
    // API: `inBuff` is NULL there, so `buffered` must be 0 and
    // `ingested == consumed` for every sample.
    let src = corpus(Corpus::Text, 300_000, 0xF1F1);
    diff("prog/compressContinue", |l| {
        let cctx = Ctx::cctx(l);
        let begin = l.sym::<FnInitCStream>("ZSTD_compressBegin");
        let cont = l.sym::<FnDecompressDCtx>("ZSTD_compressContinue");
        let end = l.sym::<FnDecompressDCtx>("ZSTD_compressEnd");
        let fp = l.sym::<FnFrameProgression>("ZSTD_getFrameProgression");
        let tfn = l.sym::<FnSizeofPtr>("ZSTD_toFlushNow");
        let mut samples = Vec::new();
        let mut dst = vec![0u8; compress_bound(l, src.len()) + 128];
        let mut dpos = 0usize;
        let mut rets = Vec::new();
        rets.push(res(l, unsafe { begin(cctx.ptr, 3) }));
        samples.push((unsafe { fp(cctx.ptr) }, unsafe { tfn(cctx.ptr) }));
        let mut off = 0usize;
        while off < src.len() {
            let k = (src.len() - off).min(65536);
            let ret = unsafe {
                cont(
                    cctx.ptr,
                    dst.as_mut_ptr().add(dpos) as *mut c_void,
                    dst.len() - dpos,
                    src.as_ptr().add(off) as *const c_void,
                    k,
                )
            };
            let r = res(l, ret);
            if let R::Ok(k2) = r {
                dpos += k2;
            }
            rets.push(r);
            samples.push((unsafe { fp(cctx.ptr) }, unsafe { tfn(cctx.ptr) }));
            off += k;
        }
        let ret = unsafe {
            end(
                cctx.ptr,
                dst.as_mut_ptr().add(dpos) as *mut c_void,
                dst.len() - dpos,
                src.as_ptr() as *const c_void,
                0,
            )
        };
        rets.push(res(l, ret));
        samples.push((unsafe { fp(cctx.ptr) }, unsafe { tfn(cctx.ptr) }));
        (rets, samples)
    });
}

// ---------------------------------------------------------------------------
// 4g. ZSTD_compressStream2_simpleArgs / ZSTD_decompressStream_simpleArgs
// ---------------------------------------------------------------------------

/// `ZSTD_compressStream2_simpleArgs` (zstd_compress.c:6545-6566) driven with the
/// `dstPos`/`srcPos` out-params carried across calls, the exposed `srcSize` and
/// `dstCapacity` growing from those positions each step.
fn drive_cs2_simple(l: &Lib, src: &[u8], s: &CSetup, dstcap: usize, sched: &Sched) -> Run {
    let cs = Ctx::cstream(l);
    let setup = apply_c(l, cs.ptr, s);
    let f = l.sym::<FnCS2Simple>("ZSTD_compressStream2_simpleArgs");
    let mut holder = vec![0u8; src.len() + 64];
    holder[..src.len()].copy_from_slice(src);
    let mut dst = vec![0u8; dstcap + 64];
    let mut spos = 0usize;
    let mut dpos = 0usize;
    let mut steps: Vec<Step> = Vec::new();
    let mut finished = false;
    let mut end_limit: Option<usize> = None;
    let mut stall = 0u32;
    let total = sched.len() + 8000;
    let mut n = 0usize;
    while n < total {
        let (ichunk, ocap, dir) = if n < sched.len() {
            sched[n]
        } else {
            (usize::MAX, usize::MAX, ZSTD_e_end)
        };
        // Same pledgedSrcSize rule as `drive_cs2`: once ZSTD_e_end has been
        // issued, the visible input size may never grow again.
        let ssize = match end_limit {
            Some(lim) => lim,
            None => spos.saturating_add(ichunk).min(src.len()),
        };
        if dir == ZSTD_e_end && end_limit.is_none() {
            end_limit = Some(ssize);
        }
        let dcap = dpos.saturating_add(ocap.max(1)).min(dstcap);
        let sp0 = spos;
        let dp0 = dpos;
        let ret = unsafe {
            f(
                cs.ptr,
                dst.as_mut_ptr() as *mut c_void,
                dcap,
                &mut dpos,
                holder.as_ptr() as *const c_void,
                ssize,
                &mut spos,
                dir,
            )
        };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: spos,
            op: dpos,
        });
        if matches!(r, R::Err(..)) {
            break;
        }
        if dir == ZSTD_e_end && ret == 0 {
            finished = true;
            break;
        }
        if spos == sp0 && dpos == dp0 {
            stall += 1;
            if stall >= 3 && n >= sched.len() {
                break;
            }
        } else {
            stall = 0;
        }
        n += 1;
    }
    let bytes = dst[..dpos].to_vec();
    ((setup, Steps(steps), spos, finished), Blob(bytes))
}

/// `ZSTD_decompressStream_simpleArgs` (zstd_decompress.c:2392-2410), same shape.
fn drive_ds_simple(l: &Lib, comp: &[u8], s: &DSetup, dstcap: usize, sched: &Sched) -> Run {
    let ds = Ctx::dstream(l);
    let setup = apply_d(l, ds.ptr, s);
    let f = l.sym::<FnDSSimple>("ZSTD_decompressStream_simpleArgs");
    let mut holder = vec![0u8; comp.len() + 64];
    holder[..comp.len()].copy_from_slice(comp);
    let mut dst = vec![0u8; dstcap + 64];
    let mut spos = 0usize;
    let mut dpos = 0usize;
    let mut steps: Vec<Step> = Vec::new();
    let mut finished = false;
    let mut stall = 0u32;
    let total = sched.len() + 8000;
    let mut n = 0usize;
    while n < total {
        let (ichunk, ocap, _) = if n < sched.len() {
            sched[n]
        } else {
            (usize::MAX, usize::MAX, 0)
        };
        let ssize = spos.saturating_add(ichunk).min(comp.len());
        let dcap = dpos.saturating_add(ocap.max(1)).min(dstcap);
        let sp0 = spos;
        let dp0 = dpos;
        let ret = unsafe {
            f(
                ds.ptr,
                dst.as_mut_ptr() as *mut c_void,
                dcap,
                &mut dpos,
                holder.as_ptr() as *const c_void,
                ssize,
                &mut spos,
            )
        };
        let r = res(l, ret);
        steps.push(Step {
            ret: r.clone(),
            ip: spos,
            op: dpos,
        });
        if matches!(r, R::Err(..)) {
            break;
        }
        if ret == 0 {
            finished = true;
            break;
        }
        if spos == sp0 && dpos == dp0 {
            stall += 1;
            if stall >= 3 {
                break;
            }
        } else {
            stall = 0;
        }
        n += 1;
    }
    let bytes = dst[..dpos].to_vec();
    ((setup, Steps(steps), spos, finished), Blob(bytes))
}

/// Both `*_simpleArgs` wrappers: the return value **and** the two out-params
/// after every call, including on the three error returns of
/// `ZSTD_compressStream2` (zstd_compress.c:6454/6455/6456) and the two of
/// `ZSTD_decompressStream` (zstd_decompress.c:2100/2105), where the C still
/// writes `*dstPos`/`*srcPos` back before returning.
#[test]
fn t_simple_args() {
    covers(&[
        "CFG:116",
        "CFG:321",
        "ERR:compress/zstd_compress.c:6454",
        "ERR:compress/zstd_compress.c:6455",
        "ERR:compress/zstd_compress.c:6456",
        "ERR:decompress/zstd_decompress.c:2100",
        "ERR:decompress/zstd_decompress.c:2105",
    ]);
    let l0 = &pair().c;
    let mut rng = Rng::new(0x5723_0004);

    // ---- randomized schedules, compress then decompress -------------------
    for trial in 0..120usize {
        let kind = *rng.pick(ALL_CORPORA);
        let size = *rng.pick(&[
            0usize, 1, 7, 100, 1000, 4096, 65535, 65536, 131072, 131073, 200_000, 400_000,
        ]);
        let level = *rng.pick(&[-3i32, 1, 3, 6, 19]);
        let src = corpus(kind, size, 0x1357 + trial as u64);
        let caps = caps_for(size);
        let nsteps = 1 + rng.below(14);
        let sched = gen_sched(&mut rng, nsteps, &caps);
        // Sometimes a capacity too small to finish the frame, so the
        // "remaining to flush" return value and the stalled positions are
        // compared too.
        let bound = compress_bound(l0, size) + 64;
        let dstcap = if trial % 7 == 3 { bound / 3 + 1 } else { bound };
        let s = CSetup::lvl(level);
        let label = format!("simple/c/{trial}/{kind:?}/n{size}/l{level}/cap{dstcap}");
        let ((_, _, consumed, finished), comp) =
            diff_bytes(&label, |l| drive_cs2_simple(l, &src, &s, dstcap, &sched));

        if finished {
            let dn = 1 + rng.below(14);
            let dsched = gen_sched(&mut rng, dn, &caps);
            let label2 = format!("simple/d/{trial}");
            let ((_, _, _, dfin), got) = diff_bytes(&label2, |l| {
                drive_ds_simple(l, &comp.0, &DSetup::default(), consumed + 64, &dsched)
            });
            if dfin {
                assert!(
                    got.0[..] == src[..consumed],
                    "{label2}: {}",
                    first_diff(&got.0, &src[..consumed]).unwrap_or_default()
                );
            }
        }
    }

    // ---- the out-of-range positions and directives -------------------------
    let src = corpus(Corpus::Text, 64 * 1024, 0x2468);
    let comp = c_frame(&src, &CSetup::lvl(3));
    diff("simple/c/bad-args", |l| {
        let cs = Ctx::cstream(l);
        let f = l.sym::<FnCS2Simple>("ZSTD_compressStream2_simpleArgs");
        let mut dst = vec![0u8; 256];
        let mut out = Vec::new();
        // (dstCapacity, dstPos, srcSize, srcPos, endOp)
        let cases: &[(usize, usize, usize, usize, c_int)] = &[
            (100, 101, 10, 0, ZSTD_e_continue),  // *dstPos > dstCapacity -> 70
            (100, 0, 10, 11, ZSTD_e_continue),   // *srcPos > srcSize     -> 72
            (100, 0, 10, 0, 3),                  // endOp 3               -> 42
            (100, 0, 10, 0, -1),                 // endOp -1 (huge U32)   -> 42
            (100, 101, 10, 11, 3),               // all three at once
            (0, 0, 10, 0, ZSTD_e_end),           // zero capacity, e_end
            (100, 100, 10, 10, ZSTD_e_end),      // both positions at the limit
        ];
        for &(dcap, dp, ss, sp, endop) in cases {
            let mut dpos = dp;
            let mut spos = sp;
            let ret = unsafe {
                f(
                    cs.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dcap,
                    &mut dpos,
                    src.as_ptr() as *const c_void,
                    ss,
                    &mut spos,
                    endop,
                )
            };
            out.push((res(l, ret), dpos, spos));
        }
        out
    });

    diff("simple/d/bad-args", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDSSimple>("ZSTD_decompressStream_simpleArgs");
        let mut dst = vec![0u8; 1 << 17];
        let mut out = Vec::new();
        let cases: &[(usize, usize, usize, usize)] = &[
            (100, 101, 10, 0), // *dstPos > dstCapacity -> 70
            (100, 0, 10, 11),  // *srcPos > srcSize     -> 72
            (100, 101, 10, 11),
            (0, 0, 10, 0),
            (100, 100, 10, 10),
        ];
        for &(dcap, dp, ss, sp) in cases {
            let mut dpos = dp;
            let mut spos = sp;
            let ret = unsafe {
                f(
                    ds.ptr,
                    dst.as_mut_ptr() as *mut c_void,
                    dcap,
                    &mut dpos,
                    comp.as_ptr() as *const c_void,
                    ss,
                    &mut spos,
                )
            };
            out.push((res(l, ret), dpos, spos));
        }
        out
    });

    // Corrupt input through the simpleArgs wrapper: the positions must still be
    // written back on the error return.
    let mut bad = comp.clone();
    bad[comp.len() / 2] ^= 0xFF;
    for &(name, ref buf) in &[
        ("truncated", comp[..comp.len() / 2].to_vec()),
        ("corrupt", bad.clone()),
        ("garbage", vec![0xAAu8; 64]),
    ] {
        let sched: Sched = vec![(4096, 1 << 16, 0); 40];
        let label = format!("simple/d/{name}");
        diff_bytes(&label, |l| {
            drive_ds_simple(l, buf, &DSetup::default(), 1 << 17, &sched)
        });
    }
}

// ---------------------------------------------------------------------------
// 4h. window sliding, extDict, index rebasing
// ---------------------------------------------------------------------------

/// Streaming inputs several times larger than the window, so the encoder runs
/// the paths a single-block test can never reach: `ZSTD_window_update` reporting
/// non-contiguity, `ZSTD_window_enforceMaxDist` advancing `lowLimit` every block
/// (which switches `ZSTD_selectBlockCompressor` to the `_extDict` variants),
/// `ZSTD_window_needOverflowCorrection` -> `ZSTD_window_correctOverflow` +
/// `ZSTD_reduceIndex` / `ZSTD_reduceTable_btlazy2`, and on the decode side the
/// `outBuff` ring wrap (`outStart + blockSizeMax > outBuffSize -> outStart =
/// outEnd = 0`).
///
/// The exact compressed bytes are compared, then the round trip.
#[test]
fn t_window_sliding_extdict() {
    covers(&[
        "CFG:28-31", "CFG:60", "CFG:67",
    ]);
    let cout = cstream_out_size();

    // (a) The strategy x windowLog grid CONFIGS rows 28-31 name, at the size and
    // chunking they specify (262144 bytes fed in 16 KB chunks). Each row selects
    // a different `_extDict` block compressor.
    let grid: &[(&[c_int], c_int)] = &[
        (&[ZSTD_fast, ZSTD_dfast], 10),                       // row 28
        (&[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2], 14),          // row 29 (row MF off)
        (&[ZSTD_greedy, ZSTD_lazy, ZSTD_lazy2], 15),          // row 30 (row MF on)
        (&[ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2], 11), // row 31
    ];
    for (gi, (strats, wlog)) in grid.iter().enumerate() {
        for &strat in *strats {
            for &kind in &[Corpus::LongRepeats, Corpus::Text] {
                let n = 262_144usize;
                let src = corpus(kind, n, 0x2828 + gi as u64);
                let s = CSetup::lvl(3)
                    .p(ZSTD_c_strategy, strat)
                    .p(ZSTD_c_windowLog, *wlog);
                let sched: Sched = vec![(16384, cout, ZSTD_e_continue); n / 16384 + 2];
                let label = format!("slide/grid{gi}/s{strat}/w{wlog}/{kind:?}");
                let ((_, _, consumed, finished), comp) =
                    diff_bytes(&label, |l| drive_cs2(l, &src, &s, &sched, cout));
                assert!(finished, "{label}");
                assert_eq!(consumed, n, "{label}");
                let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                    decompress_simple(l, &comp.0, n + 64)
                });
                assert_eq!(r, R::Ok(n), "{label}");
                assert!(
                    got.0[..] == src[..],
                    "{label}: {}",
                    first_diff(&got.0, &src).unwrap_or_default()
                );
                // And through the streaming decoder, whose ring buffer wraps.
                let dsched: Sched = vec![(4096, 65536, 0); comp.0.len() / 4096 + 4];
                let ((_, _, _, dfin), dout) = diff_bytes(&format!("{label}/ds"), |l| {
                    drive_ds(l, &comp.0, &DSetup::default(), &dsched, 65536)
                });
                assert!(dfin, "{label}/ds");
                assert!(dout.0[..] == src[..], "{label}/ds payload");
            }
        }
    }

    // (b) Inputs many times the window, at the windowLogs the task names. The
    // schedule always opens with ZSTD_e_continue so pledgedSrcSize stays unknown
    // and ZSTD_adjustCParams_internal cannot shrink windowLog back down.
    let big: &[(c_int, usize, Corpus, c_int)] = &[
        (10, 2_000_000, Corpus::LongRepeats, 3),
        (10, 2_000_000, Corpus::Text, 1),
        (15, 2_000_000, Corpus::LongRepeats, 3),
        (15, 2_000_000, Corpus::Text, 5),
        (17, 2_000_000, Corpus::LongRepeats, 3),
        (20, 4_000_000, Corpus::LongRepeats, 3),
        (20, 4_000_000, Corpus::Text, 1),
        (27, 6_000_000, Corpus::LongRepeats, 1),
    ];
    for &(wlog, n, kind, level) in big {
        let src = corpus(kind, n, 0x3939 + wlog as u64);
        let s = CSetup::lvl(level).p(ZSTD_c_windowLog, wlog);
        let sched: Sched = vec![(131072, cout, ZSTD_e_continue); n / 131072 + 2];
        let label = format!("slide/big/w{wlog}/{kind:?}/n{n}/l{level}");
        let ((_, _, consumed, finished), comp) =
            diff_bytes(&label, |l| drive_cs2(l, &src, &s, &sched, cout));
        assert!(finished, "{label}");
        assert_eq!(consumed, n, "{label}");
        // The header must really carry the requested windowLog, otherwise the
        // window never slides and the case is vacuous.
        assert_eq!(
            c_header(&comp.0).windowSize,
            1u64 << wlog,
            "{label}: fixture windowSize"
        );
        let (r, got) =
            diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &comp.0, n + 64));
        assert_eq!(r, R::Ok(n), "{label}");
        assert!(
            got.0[..] == src[..],
            "{label}: {}",
            first_diff(&got.0, &src).unwrap_or_default()
        );
        let dsched: Sched = vec![(65536, 131072, 0); comp.0.len() / 65536 + 4];
        let ((_, _, _, dfin), dout) = diff_bytes(&format!("{label}/ds"), |l| {
            drive_ds(l, &comp.0, &DSetup::default(), &dsched, 131072)
        });
        assert!(dfin, "{label}/ds");
        assert!(dout.0[..] == src[..], "{label}/ds payload");
    }

    // (c) The same, with long-distance matching enabled: ZSTD_ldm_generateSequences
    // has its own chunked loop and its own window/overflow correction,
    // independent of the main match state (CONFIGS row 60).
    for &(wlog, n) in &[(20i32, 2_000_000usize), (27, 2_000_000)] {
        let src = corpus(Corpus::LongRepeats, n, 0x6060);
        let s = CSetup::lvl(3)
            .p(ZSTD_c_enableLongDistanceMatching, 1)
            .p(ZSTD_c_windowLog, wlog);
        let sched: Sched = vec![(131072, cout, ZSTD_e_continue); n / 131072 + 2];
        let label = format!("slide/ldm/w{wlog}/n{n}");
        let ((_, _, consumed, finished), comp) =
            diff_bytes(&label, |l| drive_cs2(l, &src, &s, &sched, cout));
        assert!(finished, "{label}");
        assert_eq!(consumed, n, "{label}");
        let (r, got) =
            diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &comp.0, n + 64));
        assert_eq!(r, R::Ok(n), "{label}");
        assert!(got.0[..] == src[..], "{label}");
    }
}

// ---------------------------------------------------------------------------
// 4i. context reuse across frames, and the three reset directives
// ---------------------------------------------------------------------------

/// Compress one frame on an **already existing** CStream, so the caller controls
/// the context's history. Returns the per-call records, the input consumed, the
/// frame-closed flag and the bytes.
fn stream_frame(
    l: &Lib,
    cs: *mut c_void,
    src: &[u8],
    sched: &Sched,
    drain_cap: usize,
) -> (Steps, usize, bool, Vec<u8>) {
    let f = l.sym::<FnCompressStream2>("ZSTD_compressStream2");
    let mut holder = vec![0u8; src.len() + 1];
    holder[..src.len()].copy_from_slice(src);
    let mut steps: Vec<Step> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut consumed = 0usize;
    let mut finished = false;
    let mut end_limit: Option<usize> = None;
    let total = sched.len() + 8000;
    let mut n = 0usize;
    while n < total {
        let (ichunk, ocap, dir) = if n < sched.len() {
            sched[n]
        } else {
            (usize::MAX, drain_cap, ZSTD_e_end)
        };
        let avail = match end_limit {
            Some(lim) => lim - consumed,
            None => src.len() - consumed,
        };
        let ilen = ichunk.min(avail);
        if dir == ZSTD_e_end && end_limit.is_none() {
            end_limit = Some(consumed + ilen);
        }
        let ocap = ocap.max(1);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ov = vec![0u8; ocap];
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ocap,
            pos: 0,
        };
        let ret = unsafe { f(cs, &mut ob, &mut inb, dir) };
        let r = res(l, ret);
        let err = matches!(r, R::Err(..));
        steps.push(Step {
            ret: r,
            ip: inb.pos,
            op: ob.pos,
        });
        out.extend_from_slice(&ov[..ob.pos]);
        consumed += inb.pos;
        if err {
            break;
        }
        if dir == ZSTD_e_end && ret == 0 {
            finished = true;
            break;
        }
        n += 1;
    }
    (Steps(steps), consumed, finished, out)
}

/// Decompress one frame on an already existing DStream.
fn dstream_frame(
    l: &Lib,
    ds: *mut c_void,
    comp: &[u8],
    ichunk: usize,
    ocap: usize,
) -> (Steps, usize, bool, Vec<u8>) {
    let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
    let mut holder = vec![0u8; comp.len() + 1];
    holder[..comp.len()].copy_from_slice(comp);
    let mut steps: Vec<Step> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut consumed = 0usize;
    let mut finished = false;
    let mut stall = 0u32;
    let iters = comp.len() / ichunk.max(1) + 64;
    let mut ov = vec![0u8; ocap.max(1)];
    for _ in 0..iters {
        let ilen = ichunk.min(comp.len() - consumed);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let ret = unsafe { f(ds, &mut ob, &mut inb) };
        let r = res(l, ret);
        let err = matches!(r, R::Err(..));
        steps.push(Step {
            ret: r,
            ip: inb.pos,
            op: ob.pos,
        });
        out.extend_from_slice(&ov[..ob.pos]);
        consumed += inb.pos;
        if err {
            break;
        }
        if ret == 0 {
            finished = true;
            break;
        }
        if inb.pos == 0 && ob.pos == 0 {
            stall += 1;
            if stall >= 3 {
                break;
            }
        } else {
            stall = 0;
        }
    }
    (Steps(steps), consumed, finished, out)
}

/// Four frames in a row on ONE CStream and ONE DStream, with each of the three
/// `ZSTD_CCtx_reset` / `ZSTD_DCtx_reset` directives between them and with no
/// reset at all.
///
/// This is the shape a per-call test cannot see: `ZSTD_CCtx_reset`'s session arm
/// only sets `streamStage = zcss_init` and `pledgedSrcSizePlusOne = 0`, its
/// parameter arm additionally runs `ZSTD_clearAllDicts` + `ZSTD_CCtxParams_reset`
/// (restoring level 3 / contentSizeFlag 1), and directives outside 1..3 hit
/// neither branch and return 0 — so frame N's bytes depend on frames 1..N-1
/// unless the right directive was used.
#[test]
fn t_context_reuse_and_reset() {
    covers(&["CFG:68", "CFG:75", "CFG:94", "CFG:95", "CFG:88"]);
    let cout = cstream_out_size();

    // Four deliberately dissimilar frames: different levels, window sizes,
    // frame flags and data shapes, so a leaked parameter changes the bytes.
    let frames: Vec<(Vec<u8>, CSetup)> = vec![
        (
            corpus(Corpus::Text, 100_000, 1),
            CSetup::lvl(1).p(ZSTD_c_checksumFlag, 1),
        ),
        (
            corpus(Corpus::Random, 50_000, 2),
            CSetup::lvl(19).p(ZSTD_c_windowLog, 10),
        ),
        (
            corpus(Corpus::LongRepeats, 200_000, 3),
            CSetup::lvl(-3).p(ZSTD_c_strategy, ZSTD_fast),
        ),
        (
            corpus(Corpus::Zeros, 1000, 4),
            CSetup::lvl(6).p(ZSTD_c_contentSizeFlag, 0),
        ),
    ];

    // `0` and `4` are outside 1..3: neither reset arm runs and the call returns 0.
    let resets: &[Option<c_int>] = &[
        None,
        Some(ZSTD_reset_session_only),
        Some(ZSTD_reset_parameters),
        Some(ZSTD_reset_session_and_parameters),
        Some(0),
        Some(4),
    ];

    for reset in resets {
        for &apply in &[true, false] {
            let tag = match reset {
                None => "noreset".to_string(),
                Some(d) => format!("reset{d}"),
            };
            let label = format!("reuse/c/{tag}/apply{apply}");
            let (recs, lens, blob) = diff_bytes(&label, |l| {
                let cs = Ctx::cstream(l);
                let rst = l.sym::<FnCCtxReset>("ZSTD_CCtx_reset");
                let mut recs = Vec::new();
                let mut lens = Vec::new();
                let mut all: Vec<u8> = Vec::new();
                for (i, (src, s)) in frames.iter().enumerate() {
                    let mut setup = Vec::new();
                    if i > 0 {
                        if let Some(dir) = *reset {
                            setup.push(res(l, unsafe { rst(cs.ptr, dir) }));
                        }
                    }
                    if apply || i == 0 {
                        setup.extend(apply_c(l, cs.ptr, s));
                    }
                    let sched: Sched =
                        vec![(16384, cout, ZSTD_e_continue); src.len() / 16384 + 2];
                    let (steps, consumed, fin, out) =
                        stream_frame(l, cs.ptr, src, &sched, cout);
                    lens.push(out.len());
                    all.extend_from_slice(&out);
                    recs.push((setup, steps, consumed, fin));
                }
                (recs, lens, Blob(all))
            });

            // Every frame must still be a valid, correctly-decoding frame.
            let mut off = 0usize;
            for (i, (src, _)) in frames.iter().enumerate() {
                let fin = recs[i].3;
                let one = blob.0[off..off + lens[i]].to_vec();
                off += lens[i];
                if !fin {
                    continue;
                }
                let n = recs[i].2;
                let (r, got) = diff_bytes(&format!("{label}/f{i}/rt"), |l| {
                    decompress_simple(l, &one, n + 64)
                });
                assert_eq!(r, R::Ok(n), "{label}/f{i}");
                assert!(got.0[..] == src[..n], "{label}/f{i} payload");
            }
        }
    }

    // ---- the decode side --------------------------------------------------
    // Frames that differ in exactly the ways a DStream keeps state about:
    // checksum presence, window size (buffer sizing / oversized-duration
    // shrinking), content size, and a skippable frame in the middle.
    let dframes: Vec<(Vec<u8>, Vec<u8>)> = {
        let a = corpus(Corpus::Text, 300_000, 11);
        let b = corpus(Corpus::Random, 1000, 12);
        let c = corpus(Corpus::LongRepeats, 150_000, 13);
        let d = corpus(Corpus::Zeros, 0, 14);
        vec![
            (a.clone(), c_frame(&a, &CSetup::lvl(3).p(ZSTD_c_checksumFlag, 1))),
            (
                b.clone(),
                c_frame(&b, &CSetup::lvl(19).p(ZSTD_c_windowLog, 10)),
            ),
            (
                c.clone(),
                c_frame(&c, &CSetup::lvl(1).p(ZSTD_c_contentSizeFlag, 0)),
            ),
            (d.clone(), c_frame(&d, &CSetup::lvl(3).p(ZSTD_c_checksumFlag, 1))),
        ]
    };
    let dresets: &[Option<c_int>] = &[
        None,
        Some(ZSTD_reset_session_only),
        Some(ZSTD_reset_parameters),
        Some(ZSTD_reset_session_and_parameters),
        Some(0),
        Some(4),
    ];
    for reset in dresets {
        for &ichunk in &[1usize, 4096, usize::MAX] {
            let tag = match reset {
                None => "noreset".to_string(),
                Some(d) => format!("reset{d}"),
            };
            let label = format!("reuse/d/{tag}/c{ichunk}");
            let (recs, _lens, blob) = diff_bytes(&label, |l| {
                let ds = Ctx::dstream(l);
                let rst = l.sym::<FnDCtxReset>("ZSTD_DCtx_reset");
                let mut recs = Vec::new();
                let mut lens = Vec::new();
                let mut all: Vec<u8> = Vec::new();
                for (i, (_plain, comp)) in dframes.iter().enumerate() {
                    let mut setup = Vec::new();
                    if i > 0 {
                        if let Some(dir) = *reset {
                            setup.push(res(l, unsafe { rst(ds.ptr, dir) }));
                        }
                    }
                    let (steps, consumed, fin, out) =
                        dstream_frame(l, ds.ptr, comp, ichunk, 1 << 16);
                    lens.push(out.len());
                    all.extend_from_slice(&out);
                    recs.push((setup, steps, consumed, fin));
                }
                (recs, lens, Blob(all))
            });
            // A DStream that decoded a complete frame is transparently reset, so
            // with any of these directives (or none) all four frames must decode.
            let mut off = 0usize;
            let mut want: Vec<u8> = Vec::new();
            for (plain, _) in &dframes {
                want.extend_from_slice(plain);
            }
            for (i, _) in dframes.iter().enumerate() {
                assert!(recs[i].3, "{label}/f{i}: frame not decoded");
                off += 1;
            }
            let _ = off;
            assert!(
                blob.0[..] == want[..],
                "{label}: concatenated payload: {}",
                first_diff(&blob.0, &want).unwrap_or_default()
            );
        }
    }

    // 200 tiny frames then a big one, on one DStream: the oversized-buffer
    // duration counter (ZSTD_DCtx_updateOversizedDuration /
    // ZSTD_DCtx_isOversizedTooLong, ZSTD_WORKSPACETOOLARGE_MAXDURATION = 128)
    // only shrinks the buffers after 128 consecutive small frames, so
    // ZSTD_sizeof_DStream must step down at a specific frame index.
    let bigsrc = corpus(Corpus::Text, 300_000, 21);
    let big = c_frame_streamed(&bigsrc, &CSetup::lvl(3).p(ZSTD_c_windowLog, 24));
    let smallsrc = corpus(Corpus::Text, 500, 22);
    let small = c_frame(&smallsrc, &CSetup::lvl(3).p(ZSTD_c_windowLog, 10));
    diff("reuse/d/oversized-duration", |l| {
        let ds = Ctx::dstream(l);
        let szf = l.sym::<FnSizeofPtr>("ZSTD_sizeof_DStream");
        let mut sizes = Vec::new();
        let (_, _, fin, _) = dstream_frame(l, ds.ptr, &big, usize::MAX, 1 << 19);
        assert!(fin);
        sizes.push(unsafe { szf(ds.ptr) });
        for _ in 0..200 {
            let (_, _, fin, out) = dstream_frame(l, ds.ptr, &small, usize::MAX, 1 << 16);
            assert!(fin);
            assert_eq!(out.len(), 500);
            sizes.push(unsafe { szf(ds.ptr) });
        }
        let (_, _, fin, _) = dstream_frame(l, ds.ptr, &big, usize::MAX, 1 << 19);
        assert!(fin);
        sizes.push(unsafe { szf(ds.ptr) });
        sizes
    });
}

// ---------------------------------------------------------------------------
// 4j. the remaining deprecated ZSTD_initCStream_* / ZSTD_initDStream_* variants
// ---------------------------------------------------------------------------

/// Drive an already-initialised CStream with the *legacy* quartet
/// (`ZSTD_compressStream` + `ZSTD_flushStream` + `ZSTD_endStream`).
///
/// The legacy calls are what the `ZSTD_initCStream_*` family is meant to pair
/// with, and crucially `ZSTD_compressStream` passes `ZSTD_e_continue`, so it does
/// **not** clobber the `pledgedSrcSize` those initialisers just set (which
/// `ZSTD_compressStream2(..., ZSTD_e_end)` would, at zstd_compress.c:6366).
fn legacy_run(l: &Lib, cs: *mut c_void, src: &[u8], ichunk: usize, ocap: usize) -> (Steps, usize, Vec<u8>) {
    let cstream = l.sym::<FnCompressStream>("ZSTD_compressStream");
    let flush = l.sym::<FnFlushStream>("ZSTD_flushStream");
    let end = l.sym::<FnFlushStream>("ZSTD_endStream");
    let mut holder = vec![0u8; src.len() + 1];
    holder[..src.len()].copy_from_slice(src);
    let mut ov = vec![0u8; ocap.max(1)];
    let mut steps: Vec<Step> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    let mut consumed = 0usize;
    let mut failed = false;

    // At least one ZSTD_compressStream call, even for an empty input, so the
    // session is opened by an ZSTD_e_continue call.
    let mut first = true;
    while first || consumed < src.len() {
        first = false;
        let ilen = ichunk.min(src.len() - consumed);
        let mut inb = ZSTD_inBuffer {
            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
            size: ilen,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let ret = unsafe { cstream(cs, &mut ob, &mut inb) };
        let r = res(l, ret);
        let err = matches!(r, R::Err(..));
        steps.push(Step {
            ret: r,
            ip: inb.pos,
            op: ob.pos,
        });
        out.extend_from_slice(&ov[..ob.pos]);
        consumed += inb.pos;
        if err {
            failed = true;
            break;
        }
    }
    if !failed {
        for _ in 0..64 {
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { flush(cs, &mut ob) };
            let r = res(l, ret);
            let stop = matches!(r, R::Err(..)) || ret == 0;
            steps.push(Step {
                ret: r,
                ip: 0,
                op: ob.pos,
            });
            out.extend_from_slice(&ov[..ob.pos]);
            if stop {
                failed |= ret != 0;
                break;
            }
        }
    }
    if !failed {
        for _ in 0..64 {
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let ret = unsafe { end(cs, &mut ob) };
            let r = res(l, ret);
            let stop = matches!(r, R::Err(..)) || ret == 0;
            steps.push(Step {
                ret: r,
                ip: 0,
                op: ob.pos,
            });
            out.extend_from_slice(&ov[..ob.pos]);
            if stop {
                break;
            }
        }
    }
    (Steps(steps), consumed, out)
}

/// Every `ZSTD_initCStream_*` / `ZSTD_initDStream_*` variant not already covered
/// by `t_legacy_cstream_quartet`, all of which are thin wrappers whose exact
/// composition of `ZSTD_CCtx_reset` / `setPledgedSrcSize` / `loadDictionary` /
/// `refCDict` decides both the resulting frame and the error they forward
/// (zstd_compress.c:5969-6075, zstd_decompress.c:1740-1780).
///
/// The dictionary is RAW CONTENT (plain text, no `ZSTD_MAGIC_DICTIONARY`), so
/// `ZSTD_dct_auto` resolves to `ZSTD_dct_rawContent` and `dictID` stays 0 — the
/// dictionary *builder* is out of scope here.
#[test]
fn t_legacy_init_variants() {
    let _serial = serial_alloc_lock();
    covers(&[
        "CFG:99",
        "ERR:compress/zstd_compress.c:5977",
        "ERR:compress/zstd_compress.c:6047",
        "ERR:compress/zstd_compress.c:6057",
        "ERR:decompress/zstd_decompress.c:1745",
        "ERR:decompress/zstd_decompress.c:1754",
        "ERR:decompress/zstd_decompress.c:1765",
        "ERR:decompress/zstd_decompress.c:1782",
    ]);
    let cout = cstream_out_size();
    let cin = cstream_in_size();
    let dict = corpus(Corpus::Text, 8192, 0x0D1C);
    let n = 100_000usize;
    let src = corpus(Corpus::Text, n, 0x1234);
    let l0 = &pair().c;

    /// Decode `comp` through a DStream initialised with one of the deprecated
    /// `ZSTD_initDStream_*` variants, so the encode-side and decode-side
    /// deprecated APIs are exercised as a matched pair.
    #[derive(Copy, Clone, Debug)]
    enum DInit {
        Plain,
        UsingDict,
        UsingDictNull,
        UsingDDict,
        UsingDDictNull,
        Reset,
    }
    fn decode_with(
        l: &Lib,
        comp: &[u8],
        dict: &[u8],
        how: DInit,
        ichunk: usize,
    ) -> (Vec<R>, Steps, usize, bool, Blob) {
        let ds = Ctx::dstream(l);
        let mut setup = Vec::new();
        let mut _ddict_keep: Option<Ctx> = None;
        match how {
            DInit::Plain => {
                setup.push(res(l, unsafe {
                    l.sym::<FnPtrOnly>("ZSTD_initDStream")(ds.ptr)
                }));
            }
            DInit::UsingDict => {
                let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
                setup.push(res(l, unsafe {
                    f(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
                }));
            }
            DInit::UsingDictNull => {
                let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
                setup.push(res(l, unsafe { f(ds.ptr, std::ptr::null(), 0) }));
            }
            DInit::UsingDDict => {
                let cd = l.sym::<FnCreateDDict>("ZSTD_createDDict");
                let p = unsafe { cd(dict.as_ptr() as *const c_void, dict.len()) };
                assert!(!p.is_null());
                let f = l.sym::<FnInitDStreamUsingDDict>("ZSTD_initDStream_usingDDict");
                setup.push(res(l, unsafe { f(ds.ptr, p) }));
                _ddict_keep = Some(Ctx::from_raw(l, p, "ZSTD_freeDDict"));
            }
            DInit::UsingDDictNull => {
                let f = l.sym::<FnInitDStreamUsingDDict>("ZSTD_initDStream_usingDDict");
                setup.push(res(l, unsafe { f(ds.ptr, std::ptr::null()) }));
            }
            DInit::Reset => {
                setup.push(res(l, unsafe {
                    l.sym::<FnPtrOnly>("ZSTD_resetDStream")(ds.ptr)
                }));
            }
        }
        let (steps, consumed, fin, out) = dstream_frame(l, ds.ptr, comp, ichunk, 1 << 17);
        (setup, steps, consumed, fin, Blob(out))
    }

    // ---- (A) ZSTD_initCStream_srcSize --------------------------------------
    for &level in &[-3i32, 1, 3, 19] {
        for &pss in &[
            0u64,
            n as u64,
            (n + 1) as u64,
            (n - 1) as u64,
            ZSTD_CONTENTSIZE_UNKNOWN,
        ] {
            for &ichunk in &[4096usize, cin] {
                let label = format!("init/srcSize/l{level}/p{pss}/c{ichunk}");
                let (recs, comp) = diff_bytes(&label, |l| {
                    let cs = Ctx::cstream(l);
                    let f = l.sym::<FnInitCStreamSrcSize>("ZSTD_initCStream_srcSize");
                    let r0 = res(l, unsafe { f(cs.ptr, level, pss) });
                    let (steps, consumed, out) = legacy_run(l, cs.ptr, &src, ichunk, cout);
                    ((r0, steps, consumed), Blob(out))
                });
                // A wrong pledge must be rejected; the exact pledges must work.
                let ok = pss == 0 || pss == n as u64 || pss == ZSTD_CONTENTSIZE_UNKNOWN;
                let has_err = recs.1 .0.iter().any(|s| matches!(s.ret, R::Err(..)));
                assert_eq!(has_err, !ok, "{label}: {:?}", recs.1);
                if ok {
                    let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                        decompress_simple(l, &comp.0, n + 64)
                    });
                    assert_eq!(r, R::Ok(n), "{label}");
                    assert_eq!(&got.0[..], &src[..], "{label}");
                }
            }
        }
    }

    // ---- (B) ZSTD_initCStream_usingDict ------------------------------------
    let seven = corpus(Corpus::Random, 7, 5);
    let dicts: &[(&str, &[u8])] = &[
        ("none", &[]),
        ("seven", &seven),
        ("raw8k", &dict),
    ];
    for &(dname, d) in dicts {
        for &level in &[-3i32, 1, 3, 19, 23] {
            let label = format!("init/usingDict/{dname}/l{level}");
            let (recs, comp) = diff_bytes(&label, |l| {
                let cs = Ctx::cstream(l);
                let f = l.sym::<FnInitCStreamUsingDict>("ZSTD_initCStream_usingDict");
                let p = if d.is_empty() {
                    std::ptr::null()
                } else {
                    d.as_ptr() as *const c_void
                };
                let r0 = res(l, unsafe { f(cs.ptr, p, d.len(), level) });
                let (steps, consumed, out) = legacy_run(l, cs.ptr, &src, 4096, cout);
                ((r0, steps, consumed), Blob(out))
            });
            if matches!(recs.0, R::Err(..)) {
                continue;
            }
            // Decode via every deprecated initDStream_* variant. A raw dict of
            // 8 KB was really used only in the "raw8k" case; the others produced
            // a dictionary-less frame, which every variant must still decode.
            for how in [
                DInit::Plain,
                DInit::UsingDict,
                DInit::UsingDictNull,
                DInit::UsingDDict,
                DInit::UsingDDictNull,
                DInit::Reset,
            ] {
                for &ichunk in &[7usize, usize::MAX] {
                    let lbl = format!("{label}/dec{how:?}/c{ichunk}");
                    let (_, _, _, fin, out) =
                        diff(&lbl, |l| decode_with(l, &comp.0, &dict, how, ichunk));
                    // With a raw-content dictionary the frame carries no dictID,
                    // so the decoder cannot tell it needs one: decoding without
                    // the dictionary either succeeds (if unused) or produces
                    // wrong/failed output — either way both libraries must agree,
                    // and when it DID succeed the payload must be right.
                    if fin {
                        assert!(out.0[..] == src[..], "{lbl}: payload");
                    }
                }
            }
        }
    }

    // ---- (C) ZSTD_initCStream_usingCDict [_advanced] -----------------------
    for &clevel in &[1i32, 3, 19] {
        for adv in [false, true] {
            for fp in 0..8u32 {
                if !adv && fp != 0 {
                    continue;
                }
                for &pss in &[n as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                    if !adv && pss != n as u64 {
                        continue;
                    }
                    let fparams = ZSTD_frameParameters {
                        contentSizeFlag: (fp & 1) as c_int,
                        checksumFlag: ((fp >> 1) & 1) as c_int,
                        noDictIDFlag: ((fp >> 2) & 1) as c_int,
                    };
                    let label = format!("init/usingCDict/l{clevel}/adv{adv}/fp{fp}/p{pss}");
                    let (recs, comp) = diff_bytes(&label, |l| {
                        let cs = Ctx::cstream(l);
                        let mk = l.sym::<FnCreateCDict>("ZSTD_createCDict");
                        let cd = unsafe {
                            mk(dict.as_ptr() as *const c_void, dict.len(), clevel)
                        };
                        assert!(!cd.is_null());
                        let keep = Ctx::from_raw(l, cd, "ZSTD_freeCDict");
                        let r0 = if adv {
                            let f = l.sym::<FnInitCStreamUsingCDictAdvanced>(
                                "ZSTD_initCStream_usingCDict_advanced",
                            );
                            res(l, unsafe { f(cs.ptr, cd, fparams, pss) })
                        } else {
                            let f =
                                l.sym::<FnInitCStreamUsingCDict>("ZSTD_initCStream_usingCDict");
                            res(l, unsafe { f(cs.ptr, cd) })
                        };
                        let (steps, consumed, out) = legacy_run(l, cs.ptr, &src, 4096, cout);
                        drop(keep);
                        ((r0, steps, consumed), Blob(out))
                    });
                    let has_err = recs.1 .0.iter().any(|s| matches!(s.ret, R::Err(..)));
                    if has_err {
                        continue;
                    }
                    let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                        let dctx = Ctx::dctx(l);
                        let f = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
                        let mut dst = vec![0u8; n + 64];
                        let ret = unsafe {
                            f(
                                dctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                dst.len(),
                                comp.0.as_ptr() as *const c_void,
                                comp.0.len(),
                                dict.as_ptr() as *const c_void,
                                dict.len(),
                            )
                        };
                        let r = res(l, ret);
                        if let R::Ok(k) = r {
                            dst.truncate(k);
                        }
                        (r, Blob(dst))
                    });
                    assert_eq!(r, R::Ok(n), "{label}");
                    assert_eq!(&got.0[..], &src[..], "{label}");
                }
            }
        }
    }

    // cdict == NULL: ZSTD_CCtx_refCDict(zcs, NULL) just clears the dictionary.
    diff_bytes("init/usingCDict/null", |l| {
        let cs = Ctx::cstream(l);
        let f = l.sym::<FnInitCStreamUsingCDict>("ZSTD_initCStream_usingCDict");
        let r0 = res(l, unsafe { f(cs.ptr, std::ptr::null()) });
        let (steps, consumed, out) = legacy_run(l, cs.ptr, &src, 4096, cout);
        ((r0, steps, consumed), Blob(out))
    });

    // ---- (D) ZSTD_initCStream_advanced -------------------------------------
    for &level in &[1i32, 3, 19] {
        for fp in 0..8u32 {
            for &pss in &[0u64, n as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                for &withdict in &[false, true] {
                    let label =
                        format!("init/advanced/l{level}/fp{fp}/p{pss}/d{withdict}");
                    let (recs, comp) = diff_bytes(&label, |l| {
                        let cs = Ctx::cstream(l);
                        let gp = l.sym::<FnGetParams>("ZSTD_getParams");
                        let mut params = unsafe {
                            gp(
                                level,
                                if pss == ZSTD_CONTENTSIZE_UNKNOWN { 0 } else { pss },
                                if withdict { dict.len() } else { 0 },
                            )
                        };
                        params.fParams.contentSizeFlag = (fp & 1) as c_int;
                        params.fParams.checksumFlag = ((fp >> 1) & 1) as c_int;
                        params.fParams.noDictIDFlag = ((fp >> 2) & 1) as c_int;
                        let f = l.sym::<FnInitCStreamAdvanced>("ZSTD_initCStream_advanced");
                        let (dp, dl) = if withdict {
                            (dict.as_ptr() as *const c_void, dict.len())
                        } else {
                            (std::ptr::null(), 0)
                        };
                        let r0 = res(l, unsafe { f(cs.ptr, dp, dl, params, pss) });
                        let (steps, consumed, out) = legacy_run(l, cs.ptr, &src, 4096, cout);
                        ((r0, steps, consumed), Blob(out))
                    });
                    let has_err = recs.1 .0.iter().any(|s| matches!(s.ret, R::Err(..)));
                    if matches!(recs.0, R::Err(..)) || has_err {
                        continue;
                    }
                    let (r, got) = diff_bytes(&format!("{label}/rt"), |l| {
                        let dctx = Ctx::dctx(l);
                        let f = l.sym::<FnDecompressUsingDict>("ZSTD_decompress_usingDict");
                        let mut dst = vec![0u8; n + 64];
                        let (dp, dl) = if withdict {
                            (dict.as_ptr() as *const c_void, dict.len())
                        } else {
                            (std::ptr::null(), 0usize)
                        };
                        let ret = unsafe {
                            f(
                                dctx.ptr,
                                dst.as_mut_ptr() as *mut c_void,
                                dst.len(),
                                comp.0.as_ptr() as *const c_void,
                                comp.0.len(),
                                dp,
                                dl,
                            )
                        };
                        let r = res(l, ret);
                        if let R::Ok(k) = r {
                            dst.truncate(k);
                        }
                        (r, Blob(dst))
                    });
                    assert_eq!(r, R::Ok(n), "{label}");
                    assert_eq!(&got.0[..], &src[..], "{label}");
                }
            }
        }
    }

    // Invalid cParams reach ZSTD_checkCParams at zstd_compress.c:6047 ->
    // parameter_outOfBound (42), and nothing must have been initialised.
    diff("init/advanced/bad-cparams", |l| {
        let gp = l.sym::<FnGetParams>("ZSTD_getParams");
        let f = l.sym::<FnInitCStreamAdvanced>("ZSTD_initCStream_advanced");
        let mut out = Vec::new();
        for &(wl, cl, hl, sl, mm, tl, st) in &[
            (99u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0i32),
            (0, 99, 0, 0, 0, 0, 0),
            (0, 0, 99, 0, 0, 0, 0),
            (0, 0, 0, 99, 0, 0, 0),
            (0, 0, 0, 0, 99, 0, 0),
            (0, 0, 0, 0, 0, 0, 99),
            (0, 0, 0, 0, 0, 0, -1),
        ] {
            let cs = Ctx::cstream(l);
            let mut params = unsafe { gp(3, 100_000, 0) };
            if wl != 0 {
                params.cParams.windowLog = wl;
            }
            if cl != 0 {
                params.cParams.chainLog = cl;
            }
            if hl != 0 {
                params.cParams.hashLog = hl;
            }
            if sl != 0 {
                params.cParams.searchLog = sl;
            }
            if mm != 0 {
                params.cParams.minMatch = mm;
            }
            if tl != 0 {
                params.cParams.targetLength = tl;
            }
            if st != 0 {
                params.cParams.strategy = st;
            }
            out.push(res(l, unsafe {
                f(cs.ptr, std::ptr::null(), 0, params, 100_000)
            }));
        }
        out
    });

    // ---- (E) ZSTD_resetCStream ---------------------------------------------
    for &pss in &[
        0u64,
        n as u64,
        (n + 1) as u64,
        (n - 1) as u64,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ] {
        let label = format!("init/resetCStream/p{pss}");
        let (recs, comp) = diff_bytes(&label, |l| {
            let cs = Ctx::cstream(l);
            let init = l.sym::<FnInitCStream>("ZSTD_initCStream");
            let rst = l.sym::<FnU64Arg>("ZSTD_resetCStream");
            // Frame 1 establishes some history and a level of 19.
            let r0 = res(l, unsafe { init(cs.ptr, 19) });
            let (s1, c1, o1) = legacy_run(l, cs.ptr, &src[..1000], 4096, cout);
            // Frame 2 after ZSTD_resetCStream: the level must still be 19
            // (session-only reset) but the pledge is whatever was asked for.
            let r1 = res(l, unsafe { rst(cs.ptr, pss) });
            let (s2, c2, o2) = legacy_run(l, cs.ptr, &src, 4096, cout);
            let mut all = o1;
            let n1 = all.len();
            all.extend_from_slice(&o2);
            ((r0, s1, c1, r1, s2, c2, n1), Blob(all))
        });
        // `ZSTD_resetCStream` maps pss == 0 to ZSTD_CONTENTSIZE_UNKNOWN
        // (zstd_compress.c:5975) — a real semantic difference from
        // `ZSTD_CCtx_setPledgedSrcSize(cctx, 0)`, which pledges *zero* bytes and
        // therefore fails on the first byte fed (see `t_pledged_src_size`).
        let (r1, s2, n1) = (recs.3.clone(), recs.4.clone(), recs.6);
        assert_eq!(r1, R::Ok(0), "{label}: resetCStream itself must succeed");
        let ok = pss == 0 || pss == n as u64 || pss == ZSTD_CONTENTSIZE_UNKNOWN;
        let has_err = s2.0.iter().any(|st| matches!(st.ret, R::Err(..)));
        assert_eq!(has_err, !ok, "{label}: frame 2 status {s2:?}");
        if ok {
            // Frame 1 (1000 bytes at level 19) and frame 2 (the whole input, still
            // at level 19 because the reset was session-only) must both decode.
            let f1 = comp.0[..n1].to_vec();
            let f2 = comp.0[n1..].to_vec();
            let (ra, ga) =
                diff_bytes(&format!("{label}/f1"), |l| decompress_simple(l, &f1, 1064));
            assert_eq!(ra, R::Ok(1000), "{label}/f1");
            assert_eq!(&ga.0[..], &src[..1000], "{label}/f1");
            let (rb, gb) =
                diff_bytes(&format!("{label}/f2"), |l| decompress_simple(l, &f2, n + 64));
            assert_eq!(rb, R::Ok(n), "{label}/f2");
            assert_eq!(&gb.0[..], &src[..], "{label}/f2");
        }
    }
    diff("init/resetCStream/edges", |l| {
        let rst = l.sym::<FnU64Arg>("ZSTD_resetCStream");
        let cstream = l.sym::<FnCompressStream>("ZSTD_compressStream");
        // (a) never initialised
        let a = {
            let cs = Ctx::cstream(l);
            res(l, unsafe { rst(cs.ptr, 1000) })
        };
        // (b) mid-frame: streamStage != zcss_init, so
        // ZSTD_CCtx_setPledgedSrcSize -> stage_wrong (zstd_compress.c:5978)
        let b = {
            let cs = Ctx::cstream(l);
            let init = l.sym::<FnInitCStream>("ZSTD_initCStream");
            let _ = unsafe { init(cs.ptr, 3) };
            let mut ov = vec![0u8; 1 << 16];
            let mut inb = ZSTD_inBuffer {
                src: src.as_ptr() as *const c_void,
                size: src.len(),
                pos: 0,
            };
            let mut ob = ZSTD_outBuffer {
                dst: ov.as_mut_ptr() as *mut c_void,
                size: ov.len(),
                pos: 0,
            };
            let r = res(l, unsafe { cstream(cs.ptr, &mut ob, &mut inb) });
            (r, res(l, unsafe { rst(cs.ptr, 1000) }))
        };
        (a, b)
    });

    // ---- (F) the decode-side variants called mid-stream --------------------
    let comp = c_frame(&src, &CSetup::lvl(3));
    diff("init/dstream/midstream", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnDecompressStream>("ZSTD_decompressStream");
        let mut ov = vec![0u8; 4096];
        let mut inb = ZSTD_inBuffer {
            src: comp.as_ptr() as *const c_void,
            size: comp.len().min(64),
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: ov.as_mut_ptr() as *mut c_void,
            size: ov.len(),
            pos: 0,
        };
        let r0 = res(l, unsafe { f(ds.ptr, &mut ob, &mut inb) });
        // Now streamStage != zdss_init for all three.
        let fd = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
        let r1 = res(l, unsafe {
            fd(ds.ptr, dict.as_ptr() as *const c_void, dict.len())
        });
        let cd = l.sym::<FnCreateDDict>("ZSTD_createDDict");
        let p = unsafe { cd(dict.as_ptr() as *const c_void, dict.len()) };
        assert!(!p.is_null());
        let keep = Ctx::from_raw(l, p, "ZSTD_freeDDict");
        let fdd = l.sym::<FnInitDStreamUsingDDict>("ZSTD_initDStream_usingDDict");
        let r2 = res(l, unsafe { fdd(ds.ptr, p) });
        // ZSTD_initDStream forwards ZSTD_DCtx_refDDict(zds, NULL)
        // (zstd_decompress.c:1754) and ZSTD_DCtx_refDDict itself gates on
        // streamStage at :1782 — both must be stage_wrong (60) here.
        let fref = l.sym::<FnInitDStreamUsingDDict>("ZSTD_DCtx_refDDict");
        let r2b = res(l, unsafe { fref(ds.ptr, p) });
        let r2c = res(l, unsafe { fref(ds.ptr, std::ptr::null()) });
        let r2d = res(l, unsafe {
            l.sym::<FnPtrOnly>("ZSTD_initDStream")(ds.ptr)
        });
        // ZSTD_resetDStream is session-only, so it always succeeds and re-arms
        // everything the four calls above refused to touch.
        let r3 = res(l, unsafe {
            l.sym::<FnPtrOnly>("ZSTD_resetDStream")(ds.ptr)
        });
        let r4 = res(l, unsafe { fref(ds.ptr, p) });
        let r5 = res(l, unsafe {
            l.sym::<FnPtrOnly>("ZSTD_initDStream")(ds.ptr)
        });
        drop(keep);
        (r0, r1, r2, r2b, r2c, r2d, r3, r4, r5)
    });

    // A dictionary buffer that begins with ZSTD_MAGIC_DICTIONARY but is garbage:
    // ZSTD_initDStream_usingDict must forward the load failure
    // (zstd_decompress.c:1745).
    let mut fake = vec![0u8; 64];
    fake[0..4].copy_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
    fake[4..8].copy_from_slice(&1u32.to_le_bytes());
    diff("init/dstream/bad-dict", |l| {
        let ds = Ctx::dstream(l);
        let f = l.sym::<FnInitDStreamUsingDict>("ZSTD_initDStream_usingDict");
        let r = res(l, unsafe {
            f(ds.ptr, fake.as_ptr() as *const c_void, fake.len())
        });
        let (steps, consumed, fin, _) = dstream_frame(l, ds.ptr, &comp, usize::MAX, 1 << 17);
        (r, steps, consumed, fin)
    });
    let _ = l0;
}

// ---------------------------------------------------------------------------
// 4k. ZSTD_createCStream_advanced / ZSTD_createDStream_advanced + customMem
// ---------------------------------------------------------------------------
//
// A counting allocator turns "how many blocks of what size does this object
// need" into an observable, so this is a *structural* check on the workspace
// layout: if the Rust translation sized any cwksp region differently, or split
// one allocation into two, the recorded sequence diverges even though every
// compressed byte would still match.

const AMAX: usize = 64;
const AZERO: AtomicUsize = AtomicUsize::new(0);

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
static A_CALLS: AtomicUsize = AtomicUsize::new(0);
static A_FREES: AtomicUsize = AtomicUsize::new(0);
static A_OPAQUE_OK: AtomicUsize = AtomicUsize::new(0);
static A_OPAQUE_BAD: AtomicUsize = AtomicUsize::new(0);
/// Allocations numbered >= this value return NULL. `usize::MAX` == never fail.
static A_FAIL_FROM: AtomicUsize = AtomicUsize::new(usize::MAX);
static A_SIZES: [AtomicUsize; AMAX] = [AZERO; AMAX];
/// The sentinel `opaque` value: never dereferenced, only compared.
const A_OPAQUE: usize = 0x00C0_FFEE;
/// Header bytes prepended to every block so `customFree` can rebuild the Layout.
const A_HDR: usize = 32;

extern "C" fn count_alloc(opaque: *mut c_void, size: SizeT) -> *mut c_void {
    if opaque as usize == A_OPAQUE {
        A_OPAQUE_OK.fetch_add(1, Ordering::SeqCst);
    } else {
        A_OPAQUE_BAD.fetch_add(1, Ordering::SeqCst);
    }
    let i = A_CALLS.fetch_add(1, Ordering::SeqCst);
    if i < AMAX {
        A_SIZES[i].store(size, Ordering::SeqCst);
    }
    if i >= A_FAIL_FROM.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    let total = size + A_HDR;
    unsafe {
        let layout = std::alloc::Layout::from_size_align(total, A_HDR).unwrap();
        let p = std::alloc::alloc(layout);
        if p.is_null() {
            return std::ptr::null_mut();
        }
        (p as *mut usize).write(total);
        p.add(A_HDR) as *mut c_void
    }
}

extern "C" fn count_free(opaque: *mut c_void, ptr: *mut c_void) {
    if opaque as usize == A_OPAQUE {
        A_OPAQUE_OK.fetch_add(1, Ordering::SeqCst);
    } else {
        A_OPAQUE_BAD.fetch_add(1, Ordering::SeqCst);
    }
    A_FREES.fetch_add(1, Ordering::SeqCst);
    if ptr.is_null() {
        return;
    }
    unsafe {
        let base = (ptr as *mut u8).sub(A_HDR);
        let total = (base as *mut usize).read();
        let layout = std::alloc::Layout::from_size_align(total, A_HDR).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

fn a_reset(fail_from: usize) {
    A_CALLS.store(0, Ordering::SeqCst);
    A_FREES.store(0, Ordering::SeqCst);
    A_OPAQUE_OK.store(0, Ordering::SeqCst);
    A_OPAQUE_BAD.store(0, Ordering::SeqCst);
    A_FAIL_FROM.store(fail_from, Ordering::SeqCst);
    for s in A_SIZES.iter() {
        s.store(0, Ordering::SeqCst);
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ASnap {
    allocs: usize,
    frees: usize,
    opaque_ok: usize,
    opaque_bad: usize,
    sizes: Vec<usize>,
}

fn a_snap() -> ASnap {
    let n = A_CALLS.load(Ordering::SeqCst);
    ASnap {
        allocs: n,
        frees: A_FREES.load(Ordering::SeqCst),
        opaque_ok: A_OPAQUE_OK.load(Ordering::SeqCst),
        opaque_bad: A_OPAQUE_BAD.load(Ordering::SeqCst),
        sizes: (0..n.min(AMAX))
            .map(|i| A_SIZES[i].load(Ordering::SeqCst))
            .collect(),
    }
}

fn cmem(alloc: bool, free: bool) -> ZSTD_customMem {
    ZSTD_customMem {
        customAlloc: if alloc { Some(count_alloc) } else { None },
        customFree: if free { Some(count_free) } else { None },
        opaque: A_OPAQUE as *mut c_void,
    }
}

/// `ZSTD_createCStream_advanced` / `ZSTD_createDStream_advanced` (and the CCtx /
/// DCtx forms) with a custom allocator.
///
/// Every `_advanced` constructor rejects a half-populated `customMem` with
/// `(!customMem.customAlloc) ^ (!customMem.customFree)` -> NULL, threads `opaque`
/// through verbatim, and returns NULL when an allocation fails. The number and
/// the exact sizes of the allocations are compared, which pins the cwksp layout.
#[test]
fn t_create_stream_advanced_custommem() {
    let _serial = serial_alloc_lock();
    covers(&[
        "CFG:176",
        "CFG:7",
        "ERR:decompress/zstd_decompress.c:2264",
    ]);
    let cout = cstream_out_size();
    let n = 200_000usize;
    let src = corpus(Corpus::Text, n, 0x1717);
    let comp = c_frame(&src, &CSetup::lvl(3).p(ZSTD_c_checksumFlag, 1));

    // (a) the four (customAlloc, customFree) presence combinations, plus the
    // all-NULL ZSTD_defaultCMem, on all four `_advanced` constructors.
    for &(ctor, freer) in &[
        ("ZSTD_createCStream_advanced", "ZSTD_freeCStream"),
        ("ZSTD_createDStream_advanced", "ZSTD_freeDStream"),
        ("ZSTD_createCCtx_advanced", "ZSTD_freeCCtx"),
        ("ZSTD_createDCtx_advanced", "ZSTD_freeDCtx"),
    ] {
        for &(a, fr) in &[(false, false), (true, false), (false, true), (true, true)] {
            let label = format!("cmem/{ctor}/a{a}f{fr}");
            diff(&label, |l| {
                a_reset(usize::MAX);
                let f = l.sym::<FnCreateAdvanced>(ctor);
                let p = unsafe { f(cmem(a, fr)) };
                let created = !p.is_null();
                let after_create = a_snap();
                if created {
                    let free = l.sym::<FnPtrOnly>(freer);
                    unsafe { free(p) };
                }
                (created, after_create, a_snap())
            });
        }
    }

    // (b) a full streaming compression session on a custom-allocated CStream:
    // the allocation trace covers create + the transparent init's workspace +
    // inBuff/outBuff + the free of all of it.
    for &level in &[-3i32, 1, 3, 19] {
        for &wlog in &[0i32, 10, 20] {
            let label = format!("cmem/cstream-session/l{level}/w{wlog}");
            let (snaps, out) = diff_bytes(&label, |l| {
                a_reset(usize::MAX);
                let f = l.sym::<FnCreateAdvanced>("ZSTD_createCStream_advanced");
                let p = unsafe { f(cmem(true, true)) };
                assert!(!p.is_null(), "[{}] createCStream_advanced NULL", l.tag);
                let after_create = a_snap();
                let cs = Ctx::from_raw(l, p, "ZSTD_freeCStream");
                let mut s = CSetup::lvl(level);
                if wlog != 0 {
                    s = s.p(ZSTD_c_windowLog, wlog);
                }
                let setup = apply_c(l, cs.ptr, &s);
                let sched: Sched = vec![(16384, cout, ZSTD_e_continue); n / 16384 + 2];
                let (steps, consumed, fin, bytes) = stream_frame(l, cs.ptr, &src, &sched, cout);
                let after_session = a_snap();
                drop(cs);
                let after_free = a_snap();
                (
                    (setup, steps, consumed, fin, after_create, after_session, after_free),
                    Blob(bytes),
                )
            });
            assert_eq!(snaps.3, true, "{label}: frame not closed");
            // Nothing may be leaked, and nothing may be freed twice.
            assert_eq!(
                snaps.6.allocs, snaps.6.frees,
                "{label}: {} allocs vs {} frees",
                snaps.6.allocs, snaps.6.frees
            );
            assert_eq!(snaps.6.opaque_bad, 0, "{label}: opaque not threaded through");
            let (r, got) =
                diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &out.0, n + 64));
            assert_eq!(r, R::Ok(n), "{label}");
            assert_eq!(&got.0[..], &src[..], "{label}");
        }
    }

    // (c) the same for the decompression side.
    for &wlmax in &[0i32, 27] {
        let label = format!("cmem/dstream-session/w{wlmax}");
        let (snaps, out) = diff_bytes(&label, |l| {
            a_reset(usize::MAX);
            let f = l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced");
            let p = unsafe { f(cmem(true, true)) };
            assert!(!p.is_null(), "[{}] createDStream_advanced NULL", l.tag);
            let after_create = a_snap();
            let ds = Ctx::from_raw(l, p, "ZSTD_freeDStream");
            let mut d = DSetup::default();
            if wlmax != 0 {
                d = d.p(ZSTD_d_windowLogMax, wlmax);
            }
            let setup = apply_d(l, ds.ptr, &d);
            let (steps, consumed, fin, bytes) =
                dstream_frame(l, ds.ptr, &comp, 4096, 1 << 16);
            let after_session = a_snap();
            drop(ds);
            let after_free = a_snap();
            (
                (setup, steps, consumed, fin, after_create, after_session, after_free),
                Blob(bytes),
            )
        });
        assert!(snaps.3, "{label}: not decoded");
        assert_eq!(snaps.6.allocs, snaps.6.frees, "{label}: allocs != frees");
        assert_eq!(snaps.6.opaque_bad, 0, "{label}");
        assert!(out.0[..] == src[..], "{label}: payload");
    }

    // (d) an allocator that fails: at the very first allocation and at every
    // later one, on all four constructors. The C returns NULL as soon as an
    // allocation fails, after freeing whatever it already took.
    for &(ctor, freer) in &[
        ("ZSTD_createCStream_advanced", "ZSTD_freeCStream"),
        ("ZSTD_createDStream_advanced", "ZSTD_freeDStream"),
        ("ZSTD_createCCtx_advanced", "ZSTD_freeCCtx"),
        ("ZSTD_createDCtx_advanced", "ZSTD_freeDCtx"),
    ] {
        for &fail_from in &[0usize, 1, 2, 3] {
            let label = format!("cmem/fail/{ctor}/from{fail_from}");
            diff(&label, |l| {
                a_reset(fail_from);
                let f = l.sym::<FnCreateAdvanced>(ctor);
                let p = unsafe { f(cmem(true, true)) };
                let created = !p.is_null();
                let after = a_snap();
                if created {
                    // The object exists but the allocator is still poisoned: the
                    // first internal allocation of a real session must fail too.
                    let free = l.sym::<FnPtrOnly>(freer);
                    unsafe { free(p) };
                }
                (created, after, a_snap())
            });
        }
    }

    // (e) a session whose *internal* allocation fails: the object is created
    // with a working allocator, then the allocator is poisoned before the
    // transparent init that allocates the workspace, so
    // ZSTD_compressStream2 must report memory_allocation (64) identically.
    diff("cmem/fail/mid-session-c", |l| {
        a_reset(usize::MAX);
        let f = l.sym::<FnCreateAdvanced>("ZSTD_createCStream_advanced");
        let p = unsafe { f(cmem(true, true)) };
        assert!(!p.is_null());
        let cs = Ctx::from_raw(l, p, "ZSTD_freeCStream");
        let created_allocs = A_CALLS.load(Ordering::SeqCst);
        A_FAIL_FROM.store(created_allocs, Ordering::SeqCst);
        let sched: Sched = vec![(16384, cout, ZSTD_e_continue); 4];
        let (steps, consumed, fin, bytes) = stream_frame(l, cs.ptr, &src, &sched, cout);
        A_FAIL_FROM.store(usize::MAX, Ordering::SeqCst);
        (steps, consumed, fin, bytes.len())
    });

    diff("cmem/fail/mid-session-d", |l| {
        a_reset(usize::MAX);
        let f = l.sym::<FnCreateAdvanced>("ZSTD_createDStream_advanced");
        let p = unsafe { f(cmem(true, true)) };
        assert!(!p.is_null());
        let ds = Ctx::from_raw(l, p, "ZSTD_freeDStream");
        let created_allocs = A_CALLS.load(Ordering::SeqCst);
        A_FAIL_FROM.store(created_allocs, Ordering::SeqCst);
        let (steps, consumed, fin, bytes) = dstream_frame(l, ds.ptr, &comp, 4096, 1 << 16);
        A_FAIL_FROM.store(usize::MAX, Ordering::SeqCst);
        (steps, consumed, fin, bytes.len())
    });
}

// ---------------------------------------------------------------------------
// 5. the legacy quartet
// ---------------------------------------------------------------------------

/// `ZSTD_initCStream` + `ZSTD_compressStream` + `ZSTD_flushStream` +
/// `ZSTD_endStream` end to end (zstd_compress.c:6303, :7650, :7658). The return
/// value of `ZSTD_compressStream` is `ZSTD_nextInputSizeHint_MTorST`, a different
/// quantity from `ZSTD_compressStream2`'s "remaining to flush", so the hints are
/// compared explicitly.
#[test]
fn t_legacy_cstream_quartet() {
    covers(&[
        "CFG:65",
        "CFG:99",
        "ERR:compress/zstd_compress.c:6303",
        "ERR:compress/zstd_compress.c:7650",
        "ERR:compress/zstd_compress.c:7658",
    ]);
    let cin = cstream_in_size();
    let cout = cstream_out_size();

    for &level in &[-5i32, 0, 1, 3, 19, 22] {
        for &n in &[0usize, 1, 300_000] {
            for &ichunk in &[1usize, 4096, cin] {
                if n == 300_000 && ichunk == 1 && level >= 19 {
                    continue; // 300k single-byte calls at level 19: too slow, and
                              // level 3 already covers the ichunk==1 schedule.
                }
                let src = corpus(Corpus::Text, n, 31);
                let label = format!("legacy/l{level}/n{n}/c{ichunk}");
                let ((_, _, _), comp) = diff_bytes(&label, |l| {
                    let cs = Ctx::cstream(l);
                    let init = l.sym::<FnInitCStream>("ZSTD_initCStream");
                    let cstream = l.sym::<FnCompressStream>("ZSTD_compressStream");
                    let flush = l.sym::<FnFlushStream>("ZSTD_flushStream");
                    let end = l.sym::<FnFlushStream>("ZSTD_endStream");
                    let mut setup = Vec::new();
                    setup.push(res(l, unsafe { init(cs.ptr, level) }));
                    let mut steps = Vec::new();
                    let mut outall = Vec::new();
                    let mut ov = vec![0u8; cout];
                    let mut consumed = 0usize;
                    let mut holder = vec![0u8; src.len() + 1];
                    holder[..src.len()].copy_from_slice(&src);
                    while consumed < src.len() {
                        let ilen = ichunk.min(src.len() - consumed);
                        let mut inb = ZSTD_inBuffer {
                            src: unsafe { holder.as_ptr().add(consumed) } as *const c_void,
                            size: ilen,
                            pos: 0,
                        };
                        let mut ob = ZSTD_outBuffer {
                            dst: ov.as_mut_ptr() as *mut c_void,
                            size: ov.len(),
                            pos: 0,
                        };
                        let ret = unsafe { cstream(cs.ptr, &mut ob, &mut inb) };
                        let r = res(l, ret);
                        steps.push(Step {
                            ret: r.clone(),
                            ip: inb.pos,
                            op: ob.pos,
                        });
                        outall.extend_from_slice(&ov[..ob.pos]);
                        consumed += inb.pos;
                        if matches!(r, R::Err(..)) {
                            break;
                        }
                    }
                    // one flush, then end until 0
                    for _ in 0..64 {
                        let mut ob = ZSTD_outBuffer {
                            dst: ov.as_mut_ptr() as *mut c_void,
                            size: ov.len(),
                            pos: 0,
                        };
                        let ret = unsafe { flush(cs.ptr, &mut ob) };
                        let r = res(l, ret);
                        steps.push(Step {
                            ret: r.clone(),
                            ip: 0,
                            op: ob.pos,
                        });
                        outall.extend_from_slice(&ov[..ob.pos]);
                        if matches!(r, R::Err(..)) || ret == 0 {
                            break;
                        }
                    }
                    for _ in 0..64 {
                        let mut ob = ZSTD_outBuffer {
                            dst: ov.as_mut_ptr() as *mut c_void,
                            size: ov.len(),
                            pos: 0,
                        };
                        let ret = unsafe { end(cs.ptr, &mut ob) };
                        let r = res(l, ret);
                        steps.push(Step {
                            ret: r.clone(),
                            ip: 0,
                            op: ob.pos,
                        });
                        outall.extend_from_slice(&ov[..ob.pos]);
                        if matches!(r, R::Err(..)) || ret == 0 {
                            break;
                        }
                    }
                    ((setup, Steps(steps), consumed), Blob(outall))
                });
                let (r, got) =
                    diff_bytes(&format!("{label}/rt"), |l| decompress_simple(l, &comp.0, n + 64));
                assert_eq!(r, R::Ok(n), "{label}");
                assert_eq!(&got.0[..], &src[..], "{label}");
            }
        }
    }

    // Error propagation through the quartet: out.pos > out.size.
    diff("legacy/badout", |l| {
        let cs = Ctx::cstream(l);
        let init = l.sym::<FnInitCStream>("ZSTD_initCStream");
        let cstream = l.sym::<FnCompressStream>("ZSTD_compressStream");
        let flush = l.sym::<FnFlushStream>("ZSTD_flushStream");
        let end = l.sym::<FnFlushStream>("ZSTD_endStream");
        let mut dst = [0u8; 32];
        let src = [0u8; 8];
        let r0 = res(l, unsafe { init(cs.ptr, 3) });
        let mut inb = ZSTD_inBuffer {
            src: src.as_ptr() as *const c_void,
            size: 8,
            pos: 0,
        };
        let mut ob = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 10,
            pos: 11,
        };
        let r1 = res(l, unsafe { cstream(cs.ptr, &mut ob, &mut inb) });
        let mut ob = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 10,
            pos: 11,
        };
        let r2 = res(l, unsafe { flush(cs.ptr, &mut ob) });
        let mut ob = ZSTD_outBuffer {
            dst: dst.as_mut_ptr() as *mut c_void,
            size: 10,
            pos: 11,
        };
        let r3 = res(l, unsafe { end(cs.ptr, &mut ob) });
        (r0, r1, r2, r3)
    });
}
