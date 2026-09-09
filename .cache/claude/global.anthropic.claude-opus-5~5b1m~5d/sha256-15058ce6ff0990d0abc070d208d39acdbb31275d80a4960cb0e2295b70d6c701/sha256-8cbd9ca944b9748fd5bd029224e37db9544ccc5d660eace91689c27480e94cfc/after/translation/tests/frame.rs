//! Phase B — VALID-path differential tests for the `lz4frame.c` surface.
//!
//! Covers CONFIGS.md rows 89-137 (the valid configurations; the error paths
//! live in `frame_err.rs`).  Every assertion drives BOTH shared libraries
//! through their exported symbols and compares:
//!   * every `LZ4F_*` return value (header sizes, block sizes, decoder hints),
//!   * every in/out `*srcSizePtr` / `*dstSizePtr`,
//!   * the produced bytes, byte for byte.
//!
//! All randomness uses fixed seeds so failures are reproducible.
#![allow(dead_code)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};
use std::ptr;

// ---------------------------------------------------------------------------
// RAII wrappers + one-shot helpers (same shapes as frame_err.rs)
// ---------------------------------------------------------------------------

struct Cctx {
    lib: &'static Lib,
    p: *mut c_void,
}
impl Cctx {
    fn new(lib: &'static Lib) -> Cctx {
        let mut p: *mut c_void = ptr::null_mut();
        let rc = unsafe {
            lib.get::<Fn_F_createCompressionContext>("LZ4F_createCompressionContext")(
                &mut p,
                LZ4F_VERSION,
            )
        };
        assert!(!is_error(rc), "{}: createCompressionContext -> {}", lib.which, show(rc));
        assert!(!p.is_null());
        Cctx { lib, p }
    }
}
impl Drop for Cctx {
    fn drop(&mut self) {
        unsafe {
            self.lib
                .get::<Fn_F_freeCompressionContext>("LZ4F_freeCompressionContext")(self.p);
        }
    }
}

struct Dctx {
    lib: &'static Lib,
    p: *mut c_void,
}
impl Dctx {
    fn new(lib: &'static Lib) -> Dctx {
        let mut p: *mut c_void = ptr::null_mut();
        let rc = unsafe {
            lib.get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
                &mut p,
                LZ4F_VERSION,
            )
        };
        assert!(!is_error(rc), "{}: createDecompressionContext -> {}", lib.which, show(rc));
        assert!(!p.is_null());
        Dctx { lib, p }
    }
    fn reset(&self) {
        unsafe {
            self.lib
                .get::<Fn_F_resetDecompressionContext>("LZ4F_resetDecompressionContext")(self.p);
        }
    }
}
impl Drop for Dctx {
    fn drop(&mut self) {
        unsafe {
            self.lib
                .get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(self.p);
        }
    }
}

/// Digested dictionary handle.
struct CDictH {
    lib: &'static Lib,
    p: *mut c_void,
}
impl CDictH {
    fn new(lib: &'static Lib, dict: &[u8]) -> CDictH {
        let p = unsafe {
            lib.get::<Fn_F_createCDict>("LZ4F_createCDict")(
                dict.as_ptr() as *const c_void,
                dict.len(),
            )
        };
        assert!(!p.is_null(), "{}: createCDict({}) returned NULL", lib.which, dict.len());
        CDictH { lib, p }
    }
    fn new_advanced(lib: &'static Lib, cm: CustomMem, dict: &[u8]) -> CDictH {
        let p = unsafe {
            lib.get::<Fn_F_createCDict_advanced>("LZ4F_createCDict_advanced")(
                cm,
                dict.as_ptr() as *const c_void,
                dict.len(),
            )
        };
        assert!(!p.is_null(), "{}: createCDict_advanced({}) -> NULL", lib.which, dict.len());
        CDictH { lib, p }
    }
}
impl Drop for CDictH {
    fn drop(&mut self) {
        unsafe { self.lib.get::<Fn_F_freeCDict>("LZ4F_freeCDict")(self.p) }
    }
}

/// One-shot frame compression with the given library.
fn compress_frame(lib: &'static Lib, src: &[u8], prefs: Option<&LZ4F_preferences_t>) -> Vec<u8> {
    let pp = prefs.map(|p| p as *const _).unwrap_or(ptr::null());
    let bound =
        unsafe { lib.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(src.len(), pp) };
    assert!(!is_error(bound));
    let mut out = vec![0u8; bound];
    let n = unsafe {
        lib.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
            out.as_mut_ptr() as *mut c_void,
            out.len(),
            src.as_ptr() as *const c_void,
            src.len(),
            pp,
        )
    };
    assert!(!is_error(n), "{}: compressFrame -> {}", lib.which, show(n));
    out.truncate(n);
    out
}

/// Feed `frame` to `LZ4F_decompress` in one call with an ample output buffer
/// and return `(return_code, dst_written, src_consumed, output_bytes)`.
fn decompress_once(lib: &'static Lib, frame: &[u8], dst_cap: usize) -> (usize, usize, usize, Vec<u8>) {
    let dctx = Dctx::new(lib);
    let mut out = vec![0u8; dst_cap.max(1)];
    let mut dst_size = dst_cap;
    let mut src_size = frame.len();
    let rc = unsafe {
        lib.get::<Fn_F_decompress>("LZ4F_decompress")(
            dctx.p,
            out.as_mut_ptr() as *mut c_void,
            &mut dst_size,
            frame.as_ptr() as *const c_void,
            &mut src_size,
            ptr::null(),
        )
    };
    out.truncate(dst_size);
    (rc, dst_size, src_size, out)
}

/// Drive `LZ4F_decompress` in a loop until it returns 0, errors, or stalls.
/// Returns `(final_return_code, total_out, total_consumed, output)`.
fn decompress_loop(
    lib: &'static Lib,
    frame: &[u8],
    chunk: usize,
    out_chunk: usize,
) -> (usize, usize, usize, Vec<u8>) {
    let dctx = Dctx::new(lib);
    let mut consumed = 0usize;
    let mut produced = Vec::new();
    let mut buf = vec![0u8; out_chunk.max(1)];
    let mut rc = 1usize;
    let mut guard = 0;
    while consumed < frame.len() && rc != 0 {
        guard += 1;
        if guard > 100_000 {
            break;
        }
        let take = chunk.min(frame.len() - consumed);
        let mut src_size = take;
        let mut dst_size = buf.len();
        rc = unsafe {
            lib.get::<Fn_F_decompress>("LZ4F_decompress")(
                dctx.p,
                buf.as_mut_ptr() as *mut c_void,
                &mut dst_size,
                frame[consumed..].as_ptr() as *const c_void,
                &mut src_size,
                ptr::null(),
            )
        };
        if is_error(rc) {
            return (rc, produced.len(), consumed, produced);
        }
        produced.extend_from_slice(&buf[..dst_size]);
        consumed += src_size;
        if src_size == 0 && dst_size == 0 {
            break; // stalled
        }
    }
    (rc, produced.len(), consumed, produced)
}

// ---------------------------------------------------------------------------
// preference / option helpers
// ---------------------------------------------------------------------------

const BSIDS: [c_int; 5] = [0, 4, 5, 6, 7];
const LEVELS: [c_int; 6] = [-5, 0, 1, 2, 9, 12];
const SIZES: [usize; 10] = [
    0, 1, 100, 65535, 65536, 65537, 262144, 1048576, 4194304, 4194305,
];

fn mkprefs(
    bsid: c_int,
    bmode: c_int,
    cc: c_int,
    bc: c_int,
    af: c_uint,
    level: c_int,
) -> LZ4F_preferences_t {
    LZ4F_preferences_t {
        frameInfo: LZ4F_frameInfo_t {
            blockSizeID: bsid,
            blockMode: bmode,
            contentChecksumFlag: cc,
            frameType: LZ4F_FRAME,
            contentSize: 0,
            dictID: 0,
            blockChecksumFlag: bc,
        },
        compressionLevel: level,
        autoFlush: af,
        favorDecSpeed: 0,
        reserved: [0; 3],
    }
}

/// The full option cross-product required by rows 91-96.
fn all_prefs() -> Vec<LZ4F_preferences_t> {
    let mut v = Vec::new();
    for &bsid in BSIDS.iter() {
        for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
            for &cc in [LZ4F_NO_CONTENT_CHECKSUM, LZ4F_CONTENT_CHECKSUM_ENABLED].iter() {
                for &bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED].iter() {
                    for &af in [0u32, 1u32].iter() {
                        for &lvl in LEVELS.iter() {
                            v.push(mkprefs(bsid, bmode, cc, bc, af, lvl));
                        }
                    }
                }
            }
        }
    }
    v
}

fn blk_size(bsid: c_int) -> usize {
    unsafe { c().get::<Fn_F_getBlockSize>("LZ4F_getBlockSize")(bsid) }
}

/// Keep the suite inside its time budget: no HC work on multi-megabyte inputs.
fn affordable(size: usize, level: c_int) -> bool {
    !(size > 300_000 && level >= 2)
}

fn pname(p: &LZ4F_preferences_t) -> String {
    format!(
        "bsid={} bmode={} cc={} bc={} af={} lvl={} cs={} did={} fds={}",
        p.frameInfo.blockSizeID,
        p.frameInfo.blockMode,
        p.frameInfo.contentChecksumFlag,
        p.frameInfo.blockChecksumFlag,
        p.autoFlush,
        p.compressionLevel,
        p.frameInfo.contentSize,
        p.frameInfo.dictID,
        p.favorDecSpeed
    )
}

/// Decompress `frame` through both libraries in one call and check it round
/// trips back to `src` identically.
fn check_roundtrip(ctx: &str, frame: &[u8], src: &[u8]) {
    let (cv, rv) = both(|l| decompress_once(l, frame, src.len() + 1));
    assert_eq!(
        (cv.0, cv.1, cv.2),
        (rv.0, rv.1, rv.2),
        "{}: decompress C=({},{},{}) Rust=({},{},{})",
        ctx,
        show(cv.0),
        cv.1,
        cv.2,
        show(rv.0),
        rv.1,
        rv.2
    );
    assert_bytes_eq!(format!("{} plaintext C vs Rust", ctx), cv.3, rv.3);
    assert!(!is_error(cv.0), "{}: C decompress -> {}", ctx, show(cv.0));
    assert_eq!(cv.0, 0, "{}: frame not fully decoded, hint={}", ctx, cv.0);
    assert_eq!(cv.2, frame.len(), "{}: frame not fully consumed", ctx);
    assert_bytes_eq!(format!("{} plaintext vs original", ctx), cv.3, src);
}

/// Parse the block headers of a frame (needs the header length and whether the
/// frame carries per-block checksums).
fn block_headers(frame: &[u8], hdr_size: usize, block_checksum: bool) -> Vec<u32> {
    let mut v = Vec::new();
    let mut i = hdr_size;
    while i + 4 <= frame.len() {
        let bh = u32::from_le_bytes([frame[i], frame[i + 1], frame[i + 2], frame[i + 3]]);
        if bh == 0 {
            break;
        }
        v.push(bh);
        let sz = (bh & 0x7FFF_FFFF) as usize;
        i += 4 + sz + if block_checksum { 4 } else { 0 };
    }
    v
}

// ---------------------------------------------------------------------------
// streaming compression driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Begin {
    Plain,
    UsingDict,
    UsingDictOnce,
    UsingCDict,
    /// `LZ4F_compressBegin_internal(cctx, dst, cap, NULL, 0, NULL, prefs)`
    Internal,
    /// `LZ4F_compressBegin_internal` with a raw dictBuffer
    InternalDict,
    /// `LZ4F_compressBegin_internal` with a cdict
    InternalCDict,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Upd {
    Compressed,
    Uncompressed,
}

#[derive(Debug, PartialEq, Eq, Default, Clone)]
struct CLog {
    /// every return value in order: begin, updates/flushes…, end
    rets: Vec<usize>,
    out: Vec<u8>,
}

/// Full `compressBegin* -> N x update -> [flush] -> compressEnd` pipeline.
#[allow(clippy::too_many_arguments)]
fn stream_compress(
    lib: &'static Lib,
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    begin: Begin,
    dict: &[u8],
    chunks: &[usize],
    modes: &[Upd],
    flush_every: usize,
    copts: Option<&LZ4F_compressOptions_t>,
) -> CLog {
    let cctx = Cctx::new(lib);
    stream_compress_with(lib, &cctx, src, prefs, begin, dict, chunks, modes, flush_every, copts)
}

/// Same as `stream_compress` but on a caller-supplied (possibly reused) cctx.
#[allow(clippy::too_many_arguments)]
fn stream_compress_with(
    lib: &'static Lib,
    cctx: &Cctx,
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    begin: Begin,
    dict: &[u8],
    chunks: &[usize],
    modes: &[Upd],
    flush_every: usize,
    copts: Option<&LZ4F_compressOptions_t>,
) -> CLog {
    let mut log = CLog::default();
    let pp = prefs.map(|p| p as *const _).unwrap_or(ptr::null());
    let cop = copts.map(|o| o as *const _).unwrap_or(ptr::null());
    let dp = if dict.is_empty() {
        ptr::null()
    } else {
        dict.as_ptr() as *const c_void
    };

    let cdict = match begin {
        Begin::UsingCDict | Begin::InternalCDict => Some(CDictH::new(lib, dict)),
        _ => None,
    };
    let cdp = cdict.as_ref().map(|c| c.p as *const c_void).unwrap_or(ptr::null());

    // --- header (dstCapacity is exactly LZ4F_HEADER_SIZE_MAX) ---
    let mut hdr = vec![0u8; LZ4F_HEADER_SIZE_MAX];
    let hp = hdr.as_mut_ptr() as *mut c_void;
    let hn = hdr.len();
    let rc = unsafe {
        match begin {
            Begin::Plain => {
                lib.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(cctx.p, hp, hn, pp)
            }
            Begin::UsingDict => lib
                .get::<Fn_F_compressBegin_usingDict>("LZ4F_compressBegin_usingDict")(
                cctx.p, hp, hn, dp, dict.len(), pp,
            ),
            Begin::UsingDictOnce => lib
                .get::<Fn_F_compressBegin_usingDictOnce>("LZ4F_compressBegin_usingDictOnce")(
                cctx.p, hp, hn, dp, dict.len(), pp,
            ),
            Begin::UsingCDict => lib
                .get::<Fn_F_compressBegin_usingCDict>("LZ4F_compressBegin_usingCDict")(
                cctx.p, hp, hn, cdp, pp,
            ),
            Begin::Internal => lib
                .get::<Fn_F_compressBegin_internal>("LZ4F_compressBegin_internal")(
                cctx.p,
                hp,
                hn,
                ptr::null(),
                0,
                ptr::null(),
                pp,
            ),
            Begin::InternalDict => lib
                .get::<Fn_F_compressBegin_internal>("LZ4F_compressBegin_internal")(
                cctx.p,
                hp,
                hn,
                dp,
                dict.len(),
                ptr::null(),
                pp,
            ),
            Begin::InternalCDict => lib
                .get::<Fn_F_compressBegin_internal>("LZ4F_compressBegin_internal")(
                cctx.p,
                hp,
                hn,
                ptr::null(),
                0,
                cdp,
                pp,
            ),
        }
    };
    log.rets.push(rc);
    if is_error(rc) {
        return log;
    }
    log.out.extend_from_slice(&hdr[..rc]);

    // --- updates ---
    let f_upd = lib.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate");
    let f_raw = lib.get::<Fn_F_uncompressedUpdate>("LZ4F_uncompressedUpdate");
    let f_flush = lib.get::<Fn_F_flush>("LZ4F_flush");
    let f_end = lib.get::<Fn_F_compressEnd>("LZ4F_compressEnd");
    let f_bound = lib.get::<Fn_F_compressBound>("LZ4F_compressBound");

    let maxchunk = chunks.iter().cloned().max().unwrap_or(0).min(src.len().max(1));
    let bound = unsafe { f_bound(maxchunk, pp) };
    assert!(!is_error(bound));
    let mut buf = vec![0u8; bound.max(64) + 16];
    let bp = buf.as_mut_ptr() as *mut c_void;
    let bn = buf.len();

    let mut off = 0usize;
    let mut i = 0usize;
    while off < src.len() {
        let n = chunks[i % chunks.len()].max(1).min(src.len() - off);
        let mode = modes[i % modes.len()];
        let rc = unsafe {
            let f = if mode == Upd::Compressed { &f_upd } else { &f_raw };
            f(
                cctx.p,
                bp,
                bn,
                src[off..off + n].as_ptr() as *const c_void,
                n,
                cop,
            )
        };
        log.rets.push(rc);
        if is_error(rc) {
            return log;
        }
        log.out.extend_from_slice(&buf[..rc]);
        off += n;
        i += 1;
        if flush_every > 0 && i % flush_every == 0 {
            let rc = unsafe { f_flush(cctx.p, bp, bn, cop) };
            log.rets.push(rc);
            if is_error(rc) {
                return log;
            }
            log.out.extend_from_slice(&buf[..rc]);
        }
    }

    let rc = unsafe { f_end(cctx.p, bp, bn, cop) };
    log.rets.push(rc);
    if is_error(rc) {
        return log;
    }
    log.out.extend_from_slice(&buf[..rc]);
    log
}

// ---------------------------------------------------------------------------
// streaming decompression driver
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Default, Clone)]
struct DecLog {
    /// the "hint" returned by every LZ4F_decompress call
    rets: Vec<usize>,
    /// *srcSizePtr after every call
    srcs: Vec<usize>,
    /// *dstSizePtr after every call
    dsts: Vec<usize>,
    out: Vec<u8>,
}

/// Drive `LZ4F_decompress` (or `LZ4F_decompress_usingDict`) over `frame`,
/// feeding `in_chunk` bytes per call and offering `out_chunk` bytes of output
/// room.  When `stable_cap` is `Some(n)` the pledge of `stableDst` is honoured:
/// a single buffer of `n` bytes is used and the pointer only ever advances.
fn dec_run(
    lib: &'static Lib,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    dopts: Option<&LZ4F_decompressOptions_t>,
    stable_cap: Option<usize>,
    dict: Option<&[u8]>,
) -> DecLog {
    let dctx = Dctx::new(lib);
    dec_run_on(lib, &dctx, frame, in_chunk, out_chunk, dopts, stable_cap, dict)
}

#[allow(clippy::too_many_arguments)]
fn dec_run_on(
    lib: &'static Lib,
    dctx: &Dctx,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    dopts: Option<&LZ4F_decompressOptions_t>,
    stable_cap: Option<usize>,
    dict: Option<&[u8]>,
) -> DecLog {
    let f_dec = lib.get::<Fn_F_decompress>("LZ4F_decompress");
    let f_decd = lib.get::<Fn_F_decompress_usingDict>("LZ4F_decompress_usingDict");
    let op = dopts.map(|o| o as *const _).unwrap_or(ptr::null());
    let (dictp, dictn) = match dict {
        Some(d) if !d.is_empty() => (d.as_ptr() as *const c_void, d.len()),
        Some(_) => (ptr::null(), 0usize),
        None => (ptr::null(), 0usize),
    };

    let mut log = DecLog::default();
    let inch = if in_chunk == 0 { frame.len().max(1) } else { in_chunk };
    let outch = out_chunk.max(1);

    let cap = stable_cap.unwrap_or(0).max(1);
    let mut stable_buf = vec![0u8; if stable_cap.is_some() { cap } else { 1 }];
    let mut stable_off = 0usize;
    let mut chunk_buf = vec![0u8; if stable_cap.is_some() { 1 } else { outch }];

    let mut consumed = 0usize;
    let mut rc = 1usize;
    let mut guard = 0u32;
    loop {
        if rc == 0 && consumed >= frame.len() {
            break;
        }
        guard += 1;
        if guard > 400_000 {
            log.rets.push(usize::MAX / 4); // sentinel: driver gave up
            break;
        }
        let take = inch.min(frame.len() - consumed);
        let mut src_size = take;
        let (dstp, mut dst_size) = if stable_cap.is_some() {
            let avail = (cap - stable_off).min(outch);
            (
                unsafe { stable_buf.as_mut_ptr().add(stable_off) } as *mut c_void,
                avail,
            )
        } else {
            (chunk_buf.as_mut_ptr() as *mut c_void, chunk_buf.len())
        };
        let srcp = unsafe { frame.as_ptr().add(consumed) } as *const c_void;
        rc = unsafe {
            if dict.is_some() {
                f_decd(
                    dctx.p,
                    dstp,
                    &mut dst_size,
                    srcp,
                    &mut src_size,
                    dictp,
                    dictn,
                    op,
                )
            } else {
                f_dec(dctx.p, dstp, &mut dst_size, srcp, &mut src_size, op)
            }
        };
        log.rets.push(rc);
        if is_error(rc) {
            break;
        }
        log.srcs.push(src_size);
        log.dsts.push(dst_size);
        if stable_cap.is_some() {
            stable_off += dst_size;
        } else {
            log.out.extend_from_slice(&chunk_buf[..dst_size]);
        }
        consumed += src_size;
        if src_size == 0 && dst_size == 0 {
            break; // stalled
        }
    }
    if stable_cap.is_some() {
        log.out.extend_from_slice(&stable_buf[..stable_off]);
    }
    log
}

fn assert_dec_eq(ctx: &str, a: &DecLog, b: &DecLog) {
    assert_eq!(a.rets, b.rets, "{}: decoder hints differ", ctx);
    assert_eq!(a.srcs, b.srcs, "{}: srcSizePtr sequence differs", ctx);
    assert_eq!(a.dsts, b.dsts, "{}: dstSizePtr sequence differs", ctx);
    assert_bytes_eq!(format!("{}: plaintext", ctx), a.out, b.out);
}

// ---------------------------------------------------------------------------
// a *working* custom allocator (row 97 / 98 / 116)
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

unsafe extern "C" fn ok_alloc(_o: *mut c_void, size: usize) -> *mut c_void {
    libc_malloc(size)
}
unsafe extern "C" fn ok_calloc(_o: *mut c_void, size: usize) -> *mut c_void {
    let p = libc_malloc(size);
    if !p.is_null() {
        ptr::write_bytes(p as *mut u8, 0, size);
    }
    p
}
unsafe extern "C" fn ok_free(_o: *mut c_void, p: *mut c_void) {
    if !p.is_null() {
        libc_free(p);
    }
}

fn working_cmem() -> CustomMem {
    CustomMem {
        customAlloc: Some(ok_alloc),
        customCalloc: Some(ok_calloc),
        customFree: Some(ok_free),
        opaqueState: ptr::null_mut(),
    }
}

// ===========================================================================
// rows 89, 90 — version / level max / block sizes
// ===========================================================================

/// CONFIGS.md rows 89, 90 — `LZ4F_getVersion`, `LZ4F_compressionLevel_max`,
/// `LZ4F_getBlockSize` for every VALID block size ID (0 defaults to 64 KB).
#[test]
fn frame_version_level_max_and_block_sizes() {
    let (cv, rv) = both(|l| unsafe { l.get::<Fn_F_getVersion>("LZ4F_getVersion")() });
    assert_eq!(cv, rv, "LZ4F_getVersion C={} Rust={}", cv, rv);
    assert_eq!(cv, LZ4F_VERSION);

    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_compressionLevel_max>("LZ4F_compressionLevel_max")()
    });
    assert_eq!(cv, rv, "compressionLevel_max C={} Rust={}", cv, rv);
    assert_eq!(cv, LZ4HC_CLEVEL_MAX);

    let cases: [(c_int, usize); 5] = [
        (0, 64 * 1024),
        (4, 64 * 1024),
        (5, 256 * 1024),
        (6, 1024 * 1024),
        (7, 4 * 1024 * 1024),
    ];
    for (id, want) in cases {
        let (cv, rv) = both(|l| unsafe { l.get::<Fn_F_getBlockSize>("LZ4F_getBlockSize")(id) });
        assert_eq!(cv, rv, "getBlockSize({}) C={} Rust={}", id, show(cv), show(rv));
        assert_eq!(cv, want, "getBlockSize({})", id);
    }
}

// ===========================================================================
// rows 91, 92 — compressBound / compressFrameBound over the full cross-product
// ===========================================================================

/// CONFIGS.md row 91 — `LZ4F_compressBound` over the FULL cross-product of
/// {blockSizeID} x {blockMode} x {contentChecksum} x {blockChecksum} x
/// {autoFlush} x {compressionLevel} x {srcSize}, and with `prefsPtr == NULL`.
#[test]
fn frame_compressBound_full_cross_product() {
    for p in all_prefs() {
        for &s in SIZES.iter() {
            let (cv, rv) = both(|l| unsafe {
                l.get::<Fn_F_compressBound>("LZ4F_compressBound")(s, &p as *const _)
            });
            assert_eq!(
                cv,
                rv,
                "compressBound(src={}, {}) C={} Rust={}",
                s,
                pname(&p),
                show(cv),
                show(rv)
            );
            assert!(!is_error(cv));
        }
    }
    // prefsPtr == NULL -> worst case (both checksums forced on, 64 KB blocks,
    // alreadyBuffered = (size_t)-1)
    for &s in SIZES.iter() {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_F_compressBound>("LZ4F_compressBound")(s, ptr::null())
        });
        assert_eq!(cv, rv, "compressBound(src={}, NULL) C={} Rust={}", s, show(cv), show(rv));
    }
}

/// CONFIGS.md row 92 — `LZ4F_compressFrameBound` over the same full
/// cross-product plus `prefsPtr == NULL` (autoFlush forced to 1, +19 bytes).
#[test]
fn frame_compressFrameBound_full_cross_product() {
    for p in all_prefs() {
        for &s in SIZES.iter() {
            let (cv, rv) = both(|l| unsafe {
                l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(s, &p as *const _)
            });
            assert_eq!(
                cv,
                rv,
                "compressFrameBound(src={}, {}) C={} Rust={}",
                s,
                pname(&p),
                show(cv),
                show(rv)
            );
            // must be an upper bound on what compressFrame actually writes
            assert!(!is_error(cv));
        }
    }
    for &s in SIZES.iter() {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(s, ptr::null())
        });
        assert_eq!(cv, rv, "compressFrameBound(src={}, NULL)", s);
    }
}

// ===========================================================================
// rows 93-96 — one-shot LZ4F_compressFrame
// ===========================================================================

/// CONFIGS.md rows 93, 94, 95, 96 — `LZ4F_compressFrame` over the FULL option
/// cross-product x every `Shape` x the small end of the size list (plus
/// 262144 for the non-HC levels); every frame is decompressed back through
/// both libraries.  `prefsPtr == NULL` is covered too.
#[test]
fn frame_compressFrame_option_cross_product() {
    let mut rng = Rng::new(0x0F1A_5EED_0001);
    let sizes: [usize; 7] = [0, 1, 100, 65535, 65536, 65537, 262144];
    // pre-generate one buffer per (shape, size) so the data is shared by both
    // libraries and by every configuration.
    let mut data: Vec<Vec<Vec<u8>>> = Vec::new();
    for &sh in ALL_SHAPES.iter() {
        let mut per_size = Vec::new();
        for &s in sizes.iter() {
            per_size.push(gen(&mut rng, s, sh));
        }
        data.push(per_size);
    }

    for p in all_prefs() {
        for (si, &s) in sizes.iter().enumerate() {
            // 256 KB inputs only at the fast levels, to stay in budget
            if s >= 262144 && p.compressionLevel >= 2 {
                continue;
            }
            for (shi, &sh) in ALL_SHAPES.iter().enumerate() {
                let src = &data[shi][si];
                let ctx = format!("compressFrame {} shape={:?} size={}", pname(&p), sh, s);
                let (cf, rf) = both(|l| compress_frame(l, src, Some(&p)));
                assert_bytes_eq!(ctx, cf, rf);
                check_roundtrip(&ctx, &cf, src);
            }
        }
    }

    // prefsPtr == NULL, every shape / small size
    for (shi, &sh) in ALL_SHAPES.iter().enumerate() {
        for (si, &s) in sizes.iter().enumerate() {
            let src = &data[shi][si];
            let ctx = format!("compressFrame prefs=NULL shape={:?} size={}", sh, s);
            let (cf, rf) = both(|l| compress_frame(l, src, None));
            assert_bytes_eq!(ctx, cf, rf);
            check_roundtrip(&ctx, &cf, src);
        }
    }
}

/// CONFIGS.md rows 93, 95 — `LZ4F_compressFrame` for every `Shape` x every
/// size in the CONFIGS size list (including 4 MB+), with `prefsPtr == NULL`
/// and a representative set of valid preference records.
#[test]
fn frame_compressFrame_all_shapes_all_sizes() {
    let mut rng = Rng::new(0x0F1A_5EED_0002);
    let cfgs: Vec<Option<LZ4F_preferences_t>> = vec![
        None,
        Some(mkprefs(0, LZ4F_BLOCK_LINKED, 0, 0, 0, 0)),
        Some(mkprefs(
            5,
            LZ4F_BLOCK_INDEPENDENT,
            LZ4F_CONTENT_CHECKSUM_ENABLED,
            LZ4F_BLOCK_CHECKSUM_ENABLED,
            1,
            1,
        )),
        Some(mkprefs(6, LZ4F_BLOCK_LINKED, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 1, 0)),
        Some(mkprefs(7, LZ4F_BLOCK_LINKED, 0, LZ4F_BLOCK_CHECKSUM_ENABLED, 0, -3)),
        Some(mkprefs(4, LZ4F_BLOCK_INDEPENDENT, 0, 0, 0, 9)),
        Some(mkprefs(4, LZ4F_BLOCK_LINKED, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 1, 12)),
    ];

    for &sh in ALL_SHAPES.iter() {
        for &s in SIZES.iter() {
            let src = gen(&mut rng, s, sh);
            for cfg in cfgs.iter() {
                let lvl = cfg.map(|p| p.compressionLevel).unwrap_or(0);
                if !affordable(s, lvl) {
                    continue;
                }
                let ctx = format!(
                    "compressFrame shape={:?} size={} {}",
                    sh,
                    s,
                    cfg.map(|p| pname(&p)).unwrap_or_else(|| "prefs=NULL".into())
                );
                let (cf, rf) = both(|l| compress_frame(l, &src, cfg.as_ref()));
                assert_bytes_eq!(ctx, cf, rf);
                check_roundtrip(&ctx, &cf, &src);
            }
        }
    }
}

/// CONFIGS.md rows 94, 95 — `LZ4F_optimalBSID` downgrade (blockSizeID=7 with a
/// tiny srcSize collapses to 4) and the force-to-independent rule
/// (`srcSize <= getBlockSize(bsid)`), verified through the emitted BD byte and
/// FLG bit 5 plus `LZ4F_getFrameInfo`.
#[test]
fn frame_compressFrame_optimalBSID_and_blockmode_forcing() {
    let mut rng = Rng::new(0x0F1A_5EED_0003);
    let sizes = [0usize, 1, 1000, 65536, 65537, 262144, 262145, 1048576, 1048577];
    for &bsid in BSIDS.iter() {
        for &sz in sizes.iter() {
            for &lvl in [0i32, 1, 9].iter() {
                if !affordable(sz, lvl) {
                    continue;
                }
                let src = gen(&mut rng, sz, Shape::Text);
                let p = mkprefs(bsid, LZ4F_BLOCK_LINKED, 0, 0, 0, lvl);
                let ctx = format!("optimalBSID bsid={} size={} lvl={}", bsid, sz, lvl);
                let (cf, rf) = both(|l| compress_frame(l, &src, Some(&p)));
                assert_bytes_eq!(ctx, cf, rf);

                // expected block size ID after LZ4F_optimalBSID
                let requested = if bsid == 0 { 4 } else { bsid };
                let mut proposed = 4i32;
                let mut mbs = 64 * 1024usize;
                while requested > proposed {
                    if sz <= mbs {
                        break;
                    }
                    proposed += 1;
                    mbs <<= 2;
                }
                let expect_bsid = if requested > proposed { proposed } else { requested };
                assert_eq!(
                    (cf[5] >> 4) as c_int,
                    expect_bsid,
                    "{}: BD byte encodes bsid {} (expected {})",
                    ctx,
                    cf[5] >> 4,
                    expect_bsid
                );
                // blockMode forced to independent when everything fits one block
                let one_block = sz <= blk_size(expect_bsid);
                let flg_linked = (cf[4] >> 5) & 1;
                assert_eq!(
                    flg_linked,
                    if one_block { 1 } else { 0 },
                    "{}: FLG blockMode bit (one_block={})",
                    ctx,
                    one_block
                );

                // LZ4F_getFrameInfo must report the same thing in both libs
                let (ci, ri) = both(|l| {
                    let dctx = Dctx::new(l);
                    let mut fi = LZ4F_frameInfo_t::default();
                    let mut sz2 = cf.len();
                    let rc = unsafe {
                        l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                            dctx.p,
                            &mut fi,
                            cf.as_ptr() as *const c_void,
                            &mut sz2,
                        )
                    };
                    (rc, sz2, fi)
                });
                assert_eq!(ci.0, ri.0, "{}: getFrameInfo rc", ctx);
                assert_eq!(ci.1, ri.1, "{}: getFrameInfo consumed", ctx);
                assert_eq!(ci.2, ri.2, "{}: getFrameInfo fields", ctx);
                assert_eq!(ci.2.blockSizeID, expect_bsid, "{}: frameInfo bsid", ctx);
                check_roundtrip(&ctx, &cf, &src);
            }
        }
    }
}

/// CONFIGS.md rows 96, 102, 103 — non-zero declared `contentSize`
/// (auto-corrected to the real srcSize), non-zero `dictID`, and
/// `favorDecSpeed` 0/1 at levels below AND above 10.
#[test]
fn frame_compressFrame_contentSize_dictID_favorDecSpeed() {
    let mut rng = Rng::new(0x0F1A_5EED_0004);
    for &sz in [0usize, 1, 100, 65537, 200_000].iter() {
        let src = gen(&mut rng, sz, Shape::Runs);
        for &declared in [0u64, 1, 12345, u64::MAX].iter() {
            for &did in [0u32, 1, 0xDEAD_BEEF].iter() {
                for &lvl in [0i32, 1, 9, 10, 12].iter() {
                    for &fds in [0u32, 1].iter() {
                        if !affordable(sz, lvl) {
                            continue;
                        }
                        let mut p = mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 0, lvl);
                        p.frameInfo.contentSize = declared;
                        p.frameInfo.dictID = did;
                        p.favorDecSpeed = fds;
                        let ctx = format!("compressFrame {} size={}", pname(&p), sz);
                        let (cf, rf) = both(|l| compress_frame(l, &src, Some(&p)));
                        assert_bytes_eq!(ctx, cf, rf);

                        // the declared contentSize is auto-corrected to the
                        // real srcSize, so declaring N with srcSize==0 turns
                        // the field OFF again
                        let effective_cs = if declared != 0 { sz as u64 } else { 0 };
                        let expect_hdr = 7 + if effective_cs != 0 { 8 } else { 0 }
                            + if did != 0 { 4 } else { 0 };
                        let (chs, rhs) = both(|l| unsafe {
                            l.get::<Fn_F_headerSize>("LZ4F_headerSize")(
                                cf.as_ptr() as *const c_void,
                                cf.len(),
                            )
                        });
                        assert_eq!(chs, rhs, "{}: headerSize", ctx);
                        assert_eq!(chs, expect_hdr, "{}: headerSize value", ctx);
                        if effective_cs != 0 {
                            let got = u64::from_le_bytes(cf[6..14].try_into().unwrap());
                            assert_eq!(got, sz as u64, "{}: contentSize auto-correction", ctx);
                        }
                        if did != 0 {
                            let off = expect_hdr - 5;
                            let got = u32::from_le_bytes(cf[off..off + 4].try_into().unwrap());
                            assert_eq!(got, did, "{}: dictID field", ctx);
                        }
                        check_roundtrip(&ctx, &cf, &src);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// row 97 — CDict
// ===========================================================================

/// CONFIGS.md rows 97, 106 — `LZ4F_createCDict` / `LZ4F_freeCDict` /
/// `LZ4F_compressFrame_usingCDict` / `LZ4F_compressBegin_usingCDict` over dict
/// sizes 0..200000 (>64 KB keeps only the LAST 64 KB) x levels x blockMode
/// (independent re-attaches the cdict per block).
#[test]
fn frame_cdict_compress_and_roundtrip() {
    let mut rng = Rng::new(0x0F1A_5EED_0005);
    let dict_sizes = [0usize, 1, 4, 8, 64, 4096, 65535, 65536, 65537, 100_000, 200_000];
    let big_dict = gen(&mut rng, 200_000, Shape::Text);
    let src = gen(&mut rng, 150_000, Shape::Text);

    for &ds in dict_sizes.iter() {
        // use the TAIL of big_dict so that dictionaries of different sizes
        // still share content with src
        let dict = &big_dict[..ds];
        // the compressor only ever keeps the last 64 KB
        let eff = &dict[ds.saturating_sub(65536)..];
        for &lvl in [0i32, 1, 2, 9, 12].iter() {
            for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                let mut p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 0, lvl);
                p.frameInfo.dictID = 0x1234;
                let use_src: &[u8] = if lvl >= 2 { &src[..40_000] } else { &src };
                let ctx = format!("cdict ds={} {} srclen={}", ds, pname(&p), use_src.len());

                // --- one-shot LZ4F_compressFrame_usingCDict (needs a cctx) ---
                let (cf, rf) = both(|l| {
                    let cctx = Cctx::new(l);
                    let cd = CDictH::new(l, dict);
                    let bound = unsafe {
                        l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(
                            use_src.len(),
                            &p as *const _,
                        )
                    };
                    let mut out = vec![0u8; bound];
                    let n = unsafe {
                        l.get::<Fn_F_compressFrame_usingCDict>("LZ4F_compressFrame_usingCDict")(
                            cctx.p,
                            out.as_mut_ptr() as *mut c_void,
                            out.len(),
                            use_src.as_ptr() as *const c_void,
                            use_src.len(),
                            cd.p,
                            &p as *const _,
                        )
                    };
                    assert!(!is_error(n), "{}: compressFrame_usingCDict -> {}", l.which, show(n));
                    out.truncate(n);
                    out
                });
                assert_bytes_eq!(format!("{} one-shot", ctx), cf, rf);

                // round trip through LZ4F_decompress_usingDict with the
                // effective (last 64 KB) dictionary
                let (cd1, rd1) = both(|l| {
                    dec_run(l, &cf, 0, use_src.len() + 1, None, None, Some(eff))
                });
                assert_dec_eq(&format!("{} one-shot decode", ctx), &cd1, &rd1);
                assert_bytes_eq!(format!("{} one-shot plaintext", ctx), cd1.out, use_src);

                // --- streaming LZ4F_compressBegin_usingCDict ---
                let chunks = [7000usize, 65536, 1];
                let (cs, rs) = both(|l| {
                    stream_compress(
                        l,
                        use_src,
                        Some(&p),
                        Begin::UsingCDict,
                        dict,
                        &chunks,
                        &[Upd::Compressed],
                        0,
                        None,
                    )
                });
                assert_eq!(cs.rets, rs.rets, "{} streaming rets", ctx);
                assert_bytes_eq!(format!("{} streaming bytes", ctx), cs.out, rs.out);
                let (cd2, rd2) = both(|l| {
                    dec_run(l, &cs.out, 0, use_src.len() + 1, None, None, Some(eff))
                });
                assert_dec_eq(&format!("{} streaming decode", ctx), &cd2, &rd2);
                assert_bytes_eq!(format!("{} streaming plaintext", ctx), cd2.out, use_src);

                // --- LZ4F_compressBegin_internal with the cdict argument ---
                let (ci, ri) = both(|l| {
                    stream_compress(
                        l,
                        use_src,
                        Some(&p),
                        Begin::InternalCDict,
                        dict,
                        &chunks,
                        &[Upd::Compressed],
                        0,
                        None,
                    )
                });
                assert_eq!(ci.rets, ri.rets, "{} internal-cdict rets", ctx);
                assert_bytes_eq!(format!("{} internal-cdict bytes", ctx), ci.out, ri.out);
                assert_bytes_eq!(format!("{} internal-cdict == usingCDict", ctx), ci.out, cs.out);
            }
        }
    }
}

/// CONFIGS.md row 97 — `LZ4F_createCDict_advanced` with a WORKING
/// `LZ4F_CustomMem`: the resulting frames must be identical to the ones
/// produced with the default allocator, in both libraries.
#[test]
fn frame_cdict_advanced_custom_mem() {
    let mut rng = Rng::new(0x0F1A_5EED_0006);
    let dict = gen(&mut rng, 70_000, Shape::Text);
    let src = gen(&mut rng, 40_000, Shape::Text);
    let cm = working_cmem();

    for &lvl in [0i32, 1, 9, 12].iter() {
        for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
            let p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 0, lvl);
            let ctx = format!("cdict_advanced {}", pname(&p));
            let run = |l: &'static Lib, adv: bool| -> Vec<u8> {
                let cctx = Cctx::new(l);
                let cd = if adv {
                    CDictH::new_advanced(l, cm, &dict)
                } else {
                    CDictH::new(l, &dict)
                };
                let bound = unsafe {
                    l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(
                        src.len(),
                        &p as *const _,
                    )
                };
                let mut out = vec![0u8; bound];
                let n = unsafe {
                    l.get::<Fn_F_compressFrame_usingCDict>("LZ4F_compressFrame_usingCDict")(
                        cctx.p,
                        out.as_mut_ptr() as *mut c_void,
                        out.len(),
                        src.as_ptr() as *const c_void,
                        src.len(),
                        cd.p,
                        &p as *const _,
                    )
                };
                assert!(!is_error(n), "{} -> {}", l.which, show(n));
                out.truncate(n);
                out
            };
            let (cdef, rdef) = both(|l| run(l, false));
            let (cadv, radv) = both(|l| run(l, true));
            assert_bytes_eq!(format!("{} default C vs Rust", ctx), cdef, rdef);
            assert_bytes_eq!(format!("{} advanced C vs Rust", ctx), cadv, radv);
            assert_bytes_eq!(format!("{} advanced == default (C)", ctx), cadv, cdef);
            assert_bytes_eq!(format!("{} advanced == default (Rust)", ctx), radv, rdef);

            let eff = &dict[dict.len() - 65536..];
            let (cd1, rd1) = both(|l| dec_run(l, &cadv, 0, src.len() + 1, None, None, Some(eff)));
            assert_dec_eq(&format!("{} decode", ctx), &cd1, &rd1);
            assert_bytes_eq!(format!("{} plaintext", ctx), cd1.out, src);
        }
    }
}

/// CONFIGS.md rows 98, 116 — `LZ4F_createCompressionContext_advanced` /
/// `LZ4F_createDecompressionContext_advanced` with a WORKING `LZ4F_CustomMem`:
/// the produced frames and the recovered plaintext must be byte-identical to
/// the default-allocator results in both libraries.
#[test]
fn frame_contexts_with_custom_mem() {
    let mut rng = Rng::new(0x0F1A_5EED_001A);
    let cm = working_cmem();
    for &sz in [0usize, 100, 70_000, 300_000].iter() {
        let src = gen(&mut rng, sz, Shape::Text);
        for &bsid in [4i32, 5].iter() {
            for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                for &af in [0u32, 1].iter() {
                    let p = mkprefs(bsid, bmode, 1, 1, af, 0);
                    let ctx = format!("custom cmem {} size={}", pname(&p), sz);
                    // compress through a cctx created by the _advanced entry
                    let (cf, rf) = both(|l| {
                        let cp = unsafe {
                            l.get::<Fn_F_createCompressionContext_advanced>(
                                "LZ4F_createCompressionContext_advanced",
                            )(cm, LZ4F_VERSION)
                        };
                        assert!(!cp.is_null(), "{}: cctx_advanced -> NULL", l.which);
                        let cctx = Cctx { lib: l, p: cp };
                        stream_compress_with(
                            l,
                            &cctx,
                            &src,
                            Some(&p),
                            Begin::Plain,
                            &[],
                            &[7777usize],
                            &[Upd::Compressed],
                            0,
                            None,
                        )
                    });
                    assert_eq!(cf.rets, rf.rets, "{}: return values", ctx);
                    assert_bytes_eq!(ctx, cf.out, rf.out);

                    // identical to the default allocator
                    let (cd0, _) = both(|l| {
                        stream_compress(
                            l,
                            &src,
                            Some(&p),
                            Begin::Plain,
                            &[],
                            &[7777usize],
                            &[Upd::Compressed],
                            0,
                            None,
                        )
                    });
                    assert_bytes_eq!(format!("{} == default alloc", ctx), cf.out, cd0.out);

                    // and decompress through a dctx created by _advanced
                    let (cl, rl) = both(|l| {
                        let dp = unsafe {
                            l.get::<Fn_F_createDecompressionContext_advanced>(
                                "LZ4F_createDecompressionContext_advanced",
                            )(cm, LZ4F_VERSION)
                        };
                        assert!(!dp.is_null(), "{}: dctx_advanced -> NULL", l.which);
                        let dctx = Dctx { lib: l, p: dp };
                        dec_run_on(l, &dctx, &cf.out, 0, sz + 1, None, None, None)
                    });
                    assert_dec_eq(&format!("{} decode", ctx), &cl, &rl);
                    assert_bytes_eq!(format!("{} plaintext", ctx), cl.out, src);
                }
            }
        }
    }
}

// ===========================================================================
// rows 98-106 — streaming compression: begin variants and header layout
// ===========================================================================

/// CONFIGS.md rows 99, 102, 105, 106 — all four public `compressBegin`
/// variants plus the exported `LZ4F_compressBegin_internal`, across the four
/// header lengths (7 / 11 / 15 / 19).  Asserts the begin return value, the
/// bytes, `LZ4F_headerSize` agreement and the round trip.
#[test]
fn frame_compressBegin_variants_and_header_sizes() {
    let mut rng = Rng::new(0x0F1A_5EED_0007);
    let dict = gen(&mut rng, 20_000, Shape::Text);
    let src = gen(&mut rng, 30_000, Shape::Text);
    let variants = [
        Begin::Plain,
        Begin::UsingDict,
        Begin::UsingDictOnce,
        Begin::UsingCDict,
        Begin::Internal,
        Begin::InternalDict,
        Begin::InternalCDict,
    ];
    for &v in variants.iter() {
        for &declare_cs in [false, true].iter() {
            for &did in [0u32, 0xC0FF_EE00].iter() {
                for &lvl in [0i32, 1, 9].iter() {
                    for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                        let mut p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 1, lvl);
                        if declare_cs {
                            p.frameInfo.contentSize = src.len() as u64;
                        }
                        p.frameInfo.dictID = did;
                        let expect = 7usize
                            + if declare_cs { 8 } else { 0 }
                            + if did != 0 { 4 } else { 0 };
                        let ctx = format!("begin {:?} {}", v, pname(&p));
                        let (cl, rl) = both(|l| {
                            stream_compress(
                                l,
                                &src,
                                Some(&p),
                                v,
                                &dict,
                                &[8000usize],
                                &[Upd::Compressed],
                                0,
                                None,
                            )
                        });
                        assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                        assert_bytes_eq!(ctx, cl.out, rl.out);
                        assert_eq!(cl.rets[0], expect, "{}: header size returned", ctx);
                        let (chs, rhs) = both(|l| unsafe {
                            l.get::<Fn_F_headerSize>("LZ4F_headerSize")(
                                cl.out.as_ptr() as *const c_void,
                                cl.out.len(),
                            )
                        });
                        assert_eq!(chs, rhs, "{}: headerSize", ctx);
                        assert_eq!(chs, expect, "{}: headerSize value", ctx);

                        // frames built without a dictionary must round trip
                        // with no dictionary; the dict-based ones need it.
                        let uses_dict = !matches!(v, Begin::Plain | Begin::Internal);
                        if uses_dict {
                            let (cd, rd) = both(|l| {
                                dec_run(l, &cl.out, 0, src.len() + 1, None, None, Some(&dict))
                            });
                            assert_dec_eq(&format!("{} decode", ctx), &cd, &rd);
                            assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
                        } else {
                            check_roundtrip(&ctx, &cl.out, &src);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 100, 101 — the tmpBuff sizing matrix
/// (autoFlush, blockMode) in {(1,linked),(1,independent),(0,linked),
/// (0,independent)} for every valid blockSizeID, driven through a full
/// streaming pipeline with several chunk sizes.
#[test]
fn frame_compressBegin_tmpbuff_matrix() {
    let mut rng = Rng::new(0x0F1A_5EED_0008);
    for &bsid in BSIDS.iter() {
        let bs = blk_size(bsid);
        let srclen = (bs / 4 + 5000).min(300_000);
        let src = gen(&mut rng, srclen, Shape::Text);
        for &af in [0u32, 1].iter() {
            for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                for &lvl in [0i32, 9].iter() {
                    let p = mkprefs(
                        bsid,
                        bmode,
                        LZ4F_CONTENT_CHECKSUM_ENABLED,
                        LZ4F_BLOCK_CHECKSUM_ENABLED,
                        af,
                        lvl,
                    );
                    for chunks in [
                        vec![1usize],
                        vec![13usize, 4096, 100],
                        vec![srclen.max(1)],
                    ] {
                        if chunks == vec![1usize] && srclen > 20_000 {
                            continue; // keep the call count bounded
                        }
                        let ctx = format!("tmpbuff {} chunks={:?}", pname(&p), chunks);
                        let (cl, rl) = both(|l| {
                            stream_compress(
                                l,
                                &src,
                                Some(&p),
                                Begin::Plain,
                                &[],
                                &chunks,
                                &[Upd::Compressed],
                                0,
                                None,
                            )
                        });
                        assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                        assert_bytes_eq!(ctx, cl.out, rl.out);
                        check_roundtrip(&ctx, &cl.out, &src);
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 99, 100, 115 — REUSING one cctx for several frames
/// back-to-back, switching compressionLevel 9 -> 0 (buffer kept but
/// re-initialised as `LZ4_stream_t`) and switching blockSizeID between frames.
#[test]
fn frame_cctx_reuse_across_frames() {
    let mut rng = Rng::new(0x0F1A_5EED_0009);
    let srcs: Vec<Vec<u8>> = vec![
        gen(&mut rng, 30_000, Shape::Text),
        gen(&mut rng, 70_000, Shape::Runs),
        gen(&mut rng, 5, Shape::Constant),
        gen(&mut rng, 120_000, Shape::Sparse),
        gen(&mut rng, 0, Shape::Incompressible),
    ];
    let plans: Vec<LZ4F_preferences_t> = vec![
        mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 0, 9),
        mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 0, 0),
        mkprefs(6, LZ4F_BLOCK_INDEPENDENT, 0, 1, 1, 12),
        mkprefs(5, LZ4F_BLOCK_LINKED, 1, 1, 0, 2),
        mkprefs(0, LZ4F_BLOCK_INDEPENDENT, 0, 0, 1, -2),
    ];

    let (clogs, rlogs) = both(|l| {
        let cctx = Cctx::new(l);
        let mut out = Vec::new();
        for (i, p) in plans.iter().enumerate() {
            let log = stream_compress_with(
                l,
                &cctx,
                &srcs[i],
                Some(p),
                Begin::Plain,
                &[],
                &[4096usize, 70_000, 37],
                &[Upd::Compressed],
                0,
                None,
            );
            out.push(log);
        }
        out
    });
    for (i, (cl, rl)) in clogs.iter().zip(rlogs.iter()).enumerate() {
        let ctx = format!("cctx reuse frame#{} {}", i, pname(&plans[i]));
        assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
        assert_bytes_eq!(ctx, cl.out, rl.out);
        check_roundtrip(&ctx, &cl.out, &srcs[i]);
    }

    // and the same frames concatenated must decode through ONE dctx
    let mut all = Vec::new();
    let mut plain = Vec::new();
    for (i, cl) in clogs.iter().enumerate() {
        all.extend_from_slice(&cl.out);
        plain.extend_from_slice(&srcs[i]);
    }
    let (cd, rd) = both(|l| dec_run(l, &all, 0, plain.len() + 1, None, None, None));
    assert_dec_eq("cctx reuse concatenated", &cd, &rd);
    assert_bytes_eq!("cctx reuse concatenated plaintext", cd.out, plain);
}

/// CONFIGS.md rows 107, 108, 109 — `LZ4F_compressUpdate` chunk shapes relative
/// to the frame block size: 1-byte chunks, exactly blockSize, blockSize+1,
/// blockSize-1, random sizes and one giant chunk — for every blockSizeID.
#[test]
fn frame_compressUpdate_chunk_shapes() {
    let mut rng = Rng::new(0x0F1A_5EED_000A);
    for &bsid in BSIDS.iter() {
        let bs = blk_size(bsid);
        // src spanning at least one full block plus a remainder
        let srclen = if bs <= 65536 { bs * 2 + 777 } else { bs + 777 };
        let src = gen(&mut rng, srclen, Shape::Text);
        let mut rand_chunks = Vec::new();
        for _ in 0..12 {
            let n = rng.range(1, bs + 3);
            rand_chunks.push(n);
        }
        let plans: Vec<Vec<usize>> = vec![
            vec![bs - 1],
            vec![bs],
            vec![bs + 1],
            rand_chunks,
            vec![srclen],
        ];
        for &af in [0u32, 1].iter() {
            for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                let p = mkprefs(bsid, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, af, 0);
                for plan in plans.iter() {
                    let ctx = format!("chunks {} plan={:?}", pname(&p), &plan[..plan.len().min(4)]);
                    let (cl, rl) = both(|l| {
                        stream_compress(
                            l,
                            &src,
                            Some(&p),
                            Begin::Plain,
                            &[],
                            plan,
                            &[Upd::Compressed],
                            0,
                            None,
                        )
                    });
                    assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                    assert_bytes_eq!(ctx, cl.out, rl.out);
                    check_roundtrip(&ctx, &cl.out, &src);
                }
            }
        }
        // 1-byte chunks on a small source, for every blockSizeID
        let small = gen(&mut rng, 2048, Shape::Runs);
        for &af in [0u32, 1].iter() {
            let p = mkprefs(bsid, LZ4F_BLOCK_LINKED, 1, 1, af, 0);
            let ctx = format!("1-byte chunks {}", pname(&p));
            let (cl, rl) = both(|l| {
                stream_compress(
                    l,
                    &small,
                    Some(&p),
                    Begin::Plain,
                    &[],
                    &[1usize],
                    &[Upd::Compressed],
                    0,
                    None,
                )
            });
            assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
            assert_bytes_eq!(ctx, cl.out, rl.out);
            check_roundtrip(&ctx, &cl.out, &small);
        }
    }
}

/// CONFIGS.md row 110 — `LZ4F_compressOptions_t.stableSrc` = 0 and 1 with
/// blockMode=linked (a genuinely different internal dictionary path), plus
/// `compressOptionsPtr == NULL`.
#[test]
fn frame_compressUpdate_stableSrc() {
    let mut rng = Rng::new(0x0F1A_5EED_000B);
    let src = gen(&mut rng, 250_000, Shape::Text);
    let opt0 = LZ4F_compressOptions_t { stableSrc: 0, reserved: [0; 3] };
    let opt1 = LZ4F_compressOptions_t { stableSrc: 1, reserved: [0; 3] };
    for &bsid in [4i32, 5].iter() {
        let bs = blk_size(bsid);
        for &af in [0u32, 1].iter() {
            for &lvl in [0i32, 9].iter() {
                for (name, o) in [("NULL", None), ("stableSrc=0", Some(&opt0)), ("stableSrc=1", Some(&opt1))] {
                    let p = mkprefs(bsid, LZ4F_BLOCK_LINKED, 1, 0, af, lvl);
                    for plan in [
                        vec![bs / 3],
                        vec![bs],
                        vec![bs + 17, 999],
                        vec![src.len()],
                    ] {
                        let ctx = format!("stableSrc {} {} plan={:?}", name, pname(&p), plan);
                        let (cl, rl) = both(|l| {
                            stream_compress(
                                l,
                                &src,
                                Some(&p),
                                Begin::Plain,
                                &[],
                                &plan,
                                &[Upd::Compressed],
                                0,
                                o,
                            )
                        });
                        assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                        assert_bytes_eq!(ctx, cl.out, rl.out);
                        check_roundtrip(&ctx, &cl.out, &src);
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 101, 109, 111 — autoFlush=0 + blockLinked with enough data
/// to push `tmpIn + blockSize` past `tmpBuff + maxBufferSize`, which forces the
/// `LZ4F_localSaveDict` rewind inside `LZ4F_compressUpdate` (and the identical
/// rewind at the end of `LZ4F_flush`).
#[test]
fn frame_compressUpdate_tmpIn_rewind() {
    let mut rng = Rng::new(0x0F1A_5EED_000F);
    // (blockSizeID, source length, update chunk) — the chunk is always smaller
    // than the block size so blocks are assembled `fromTmpBuffer`.
    let cases: [(c_int, usize, usize); 3] = [
        (4, 400_000, 20_000),
        (5, 1_200_000, 100_000),
        (6, 3_500_000, 300_000),
    ];
    for &(bsid, len, chunk) in cases.iter() {
        let src = gen(&mut rng, len, Shape::Text);
        for &lvl in [0i32, 1].iter() {
            for &flush_every in [0usize, 3].iter() {
                let p = mkprefs(
                    bsid,
                    LZ4F_BLOCK_LINKED,
                    LZ4F_CONTENT_CHECKSUM_ENABLED,
                    LZ4F_BLOCK_CHECKSUM_ENABLED,
                    0,
                    lvl,
                );
                let ctx = format!(
                    "tmpIn rewind {} len={} chunk={} flush={}",
                    pname(&p), len, chunk, flush_every
                );
                let (cl, rl) = both(|l| {
                    stream_compress(
                        l,
                        &src,
                        Some(&p),
                        Begin::Plain,
                        &[],
                        &[chunk],
                        &[Upd::Compressed],
                        flush_every,
                        None,
                    )
                });
                assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                assert_bytes_eq!(ctx, cl.out, rl.out);
                check_roundtrip(&ctx, &cl.out, &src);
            }
        }
    }
}

/// CONFIGS.md row 115 — `LZ4F_flush` interleaved between updates, both when
/// there IS buffered data and when there is none (must return exactly 0).
#[test]
fn frame_flush_interleaved() {
    let mut rng = Rng::new(0x0F1A_5EED_000C);
    let src = gen(&mut rng, 90_000, Shape::Text);
    for &af in [0u32, 1].iter() {
        for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
            for &every in [1usize, 2, 3].iter() {
                let p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, af, 0);
                let ctx = format!("flush every {} {}", every, pname(&p));
                let (cl, rl) = both(|l| {
                    stream_compress(
                        l,
                        &src,
                        Some(&p),
                        Begin::Plain,
                        &[],
                        &[7000usize, 65536, 1],
                        &[Upd::Compressed],
                        every,
                        None,
                    )
                });
                assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                assert_bytes_eq!(ctx, cl.out, rl.out);
                check_roundtrip(&ctx, &cl.out, &src);
            }
        }
    }

    // explicit "nothing to flush" probe: flush twice in a row on a fresh frame
    let (cv, rv) = both(|l| {
        let cctx = Cctx::new(l);
        let p = mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 0, 0);
        let mut buf = vec![0u8; 1 << 17];
        let h = unsafe {
            l.get::<Fn_F_compressBegin>("LZ4F_compressBegin")(
                cctx.p,
                buf.as_mut_ptr() as *mut c_void,
                19,
                &p as *const _,
            )
        };
        let f1 = unsafe {
            l.get::<Fn_F_flush>("LZ4F_flush")(
                cctx.p,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                ptr::null(),
            )
        };
        let u = unsafe {
            l.get::<Fn_F_compressUpdate>("LZ4F_compressUpdate")(
                cctx.p,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                src[..100].as_ptr() as *const c_void,
                100,
                ptr::null(),
            )
        };
        let f2 = unsafe {
            l.get::<Fn_F_flush>("LZ4F_flush")(
                cctx.p,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                ptr::null(),
            )
        };
        let f3 = unsafe {
            l.get::<Fn_F_flush>("LZ4F_flush")(
                cctx.p,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                ptr::null(),
            )
        };
        (h, f1, u, f2, f3)
    });
    assert_eq!(cv, rv, "flush-with-nothing-buffered C={:?} Rust={:?}", cv, rv);
    assert_eq!(cv.1, 0, "flush on empty buffer must return 0");
    assert_eq!(cv.2, 0, "buffered 100 bytes must emit nothing");
    assert!(cv.3 > 0, "flush with buffered data must emit a block");
    assert_eq!(cv.4, 0, "second flush must return 0");
}

/// CONFIGS.md row 113 — incompressible input so blocks are STORED with
/// `LZ4F_BLOCKUNCOMPRESSED_FLAG`, with blockChecksum both off and on.
#[test]
fn frame_stored_blocks_incompressible() {
    let mut rng = Rng::new(0x0F1A_5EED_000D);
    for &sz in [1usize, 12, 65536, 65537, 200_000].iter() {
        let src = gen(&mut rng, sz, Shape::Incompressible);
        for &bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED].iter() {
            for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
                for &lvl in [0i32, 9].iter() {
                    let p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, bc, 1, lvl);
                    let ctx = format!("stored {} size={}", pname(&p), sz);
                    let (cf, rf) = both(|l| compress_frame(l, &src, Some(&p)));
                    assert_bytes_eq!(ctx, cf, rf);
                    let bhs = block_headers(&cf, 7, bc == LZ4F_BLOCK_CHECKSUM_ENABLED);
                    assert!(!bhs.is_empty(), "{}: no blocks?", ctx);
                    for bh in bhs.iter() {
                        assert_eq!(
                            bh & 0x8000_0000,
                            0x8000_0000,
                            "{}: block header {:#x} is not marked uncompressed",
                            ctx,
                            bh
                        );
                    }
                    check_roundtrip(&ctx, &cf, &src);
                    // and the same via the streaming API
                    let (cl, rl) = both(|l| {
                        stream_compress(
                            l,
                            &src,
                            Some(&p),
                            Begin::Plain,
                            &[],
                            &[4096usize, 65536],
                            &[Upd::Compressed],
                            0,
                            None,
                        )
                    });
                    assert_eq!(cl.rets, rl.rets, "{} streaming rets", ctx);
                    assert_bytes_eq!(format!("{} streaming", ctx), cl.out, rl.out);
                    check_roundtrip(&format!("{} streaming", ctx), &cl.out, &src);
                }
            }
        }
    }
}

/// CONFIGS.md row 114 — `LZ4F_uncompressedUpdate`: all-stored frames, and
/// ALTERNATING `LZ4F_compressUpdate` / `LZ4F_uncompressedUpdate` on one cctx
/// (the mode change triggers an implicit flush).
#[test]
fn frame_uncompressedUpdate() {
    let mut rng = Rng::new(0x0F1A_5EED_000E);
    for &sz in [1usize, 100, 65536, 65537, 150_000].iter() {
        for &sh in [Shape::Text, Shape::Incompressible].iter() {
            let src = gen(&mut rng, sz, sh);
            for &bc in [LZ4F_NO_BLOCK_CHECKSUM, LZ4F_BLOCK_CHECKSUM_ENABLED].iter() {
                for &af in [0u32, 1].iter() {
                    let p = mkprefs(
                        4,
                        LZ4F_BLOCK_INDEPENDENT,
                        LZ4F_CONTENT_CHECKSUM_ENABLED,
                        bc,
                        af,
                        0,
                    );
                    for (name, modes) in [
                        ("all-raw", vec![Upd::Uncompressed]),
                        (
                            "alternating",
                            vec![Upd::Compressed, Upd::Uncompressed],
                        ),
                        (
                            "alt2",
                            vec![Upd::Uncompressed, Upd::Compressed, Upd::Compressed],
                        ),
                    ] {
                        let ctx = format!("uncompressedUpdate {} {} size={} shape={:?}", name, pname(&p), sz, sh);
                        let (cl, rl) = both(|l| {
                            stream_compress(
                                l,
                                &src,
                                Some(&p),
                                Begin::Plain,
                                &[],
                                &[3000usize, 40_000, 7],
                                &modes,
                                0,
                                None,
                            )
                        });
                        assert_eq!(cl.rets, rl.rets, "{}: return values", ctx);
                        assert_bytes_eq!(ctx, cl.out, rl.out);
                        check_roundtrip(&ctx, &cl.out, &src);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// rows 116-137 — streaming decompression
// ===========================================================================

/// A small set of frame configurations used by the decoder tests.
fn dec_test_frames(rng: &mut Rng) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let mut v = Vec::new();
    let cfgs: Vec<(&str, LZ4F_preferences_t, usize, Shape)> = vec![
        ("bsid4 linked plain", mkprefs(4, LZ4F_BLOCK_LINKED, 0, 0, 1, 0), 90_000, Shape::Text),
        (
            "bsid4 linked cc+bc",
            mkprefs(4, LZ4F_BLOCK_LINKED, 1, 1, 1, 0),
            90_000,
            Shape::Runs,
        ),
        (
            "bsid4 indep cc",
            mkprefs(4, LZ4F_BLOCK_INDEPENDENT, 1, 0, 1, 0),
            90_000,
            Shape::Text,
        ),
        (
            "bsid5 indep bc",
            mkprefs(5, LZ4F_BLOCK_INDEPENDENT, 0, 1, 1, 0),
            300_000,
            Shape::Sparse,
        ),
        (
            "bsid4 linked stored",
            mkprefs(4, LZ4F_BLOCK_LINKED, 1, 1, 1, 0),
            70_000,
            Shape::Incompressible,
        ),
        ("bsid4 tiny", mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 1, 0), 3, Shape::Constant),
        ("bsid4 empty", mkprefs(4, LZ4F_BLOCK_LINKED, 1, 1, 1, 0), 0, Shape::Constant),
        (
            "bsid6 linked cc+bc",
            mkprefs(6, LZ4F_BLOCK_LINKED, 1, 1, 1, 0),
            250_000,
            Shape::Text,
        ),
        (
            "bsid4 linked periodic",
            mkprefs(4, LZ4F_BLOCK_LINKED, 0, 0, 1, 0),
            200_000,
            Shape::Periodic(65535),
        ),
    ];
    for (name, mut p, sz, sh) in cfgs {
        let src = gen(rng, sz, sh);
        p.frameInfo.contentSize =
            if name.contains("indep") || name.contains("periodic") { sz as u64 } else { 0 };
        if name.contains("periodic") {
            p.frameInfo.dictID = 0x5150_1234;
        }
        let frame = compress_frame(c(), &src, Some(&p));
        v.push((name.to_string(), frame, src));
    }
    v
}

/// CONFIGS.md rows 119, 123, 125, 126, 135 — input chunking: 1 byte at a time,
/// 4/5/6/7/19/20 bytes, random sizes and one giant call.  Header, block
/// header, block payload, block checksum and content checksum all end up split
/// across calls.  The decoder HINT, the bytes consumed and the bytes produced
/// must match exactly at every step.
#[test]
fn frame_decompress_input_chunking() {
    let mut rng = Rng::new(0x0F1A_5EED_0010);
    let frames = dec_test_frames(&mut rng);
    let small_src = gen(&mut rng, 5000, Shape::Text);
    let small_p = mkprefs(4, LZ4F_BLOCK_LINKED, 1, 1, 1, 0);
    let small_frame = compress_frame(c(), &small_src, Some(&small_p));

    for (name, frame, src) in frames.iter() {
        for &ch in [0usize, 4, 5, 6, 7, 19, 20, 300, 65537].iter() {
            let ctx = format!("in-chunk {} {} inch={}", name, frame.len(), ch);
            let (cd, rd) = both(|l| dec_run(l, frame, ch, src.len() + 1, None, None, None));
            assert_dec_eq(&ctx, &cd, &rd);
            assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
        }
        // random chunk sizes
        let mut r2 = Rng::new(0xABCD_0001 ^ frame.len() as u64);
        let mut plan = Vec::new();
        for _ in 0..64 {
            let n = r2.range(1, 5000);
            plan.push(n);
        }
        let ctx = format!("in-chunk random {}", name);
        let (cd, rd) = both(|l| {
            let dctx = Dctx::new(l);
            let f_dec = l.get::<Fn_F_decompress>("LZ4F_decompress");
            let mut log = DecLog::default();
            let mut buf = vec![0u8; src.len() + 1];
            let mut consumed = 0usize;
            let mut i = 0usize;
            let mut rc = 1usize;
            while consumed < frame.len() && !is_error(rc) {
                let take = plan[i % plan.len()].min(frame.len() - consumed);
                i += 1;
                let mut ss = take;
                let mut ds = buf.len();
                rc = unsafe {
                    f_dec(
                        dctx.p,
                        buf.as_mut_ptr() as *mut c_void,
                        &mut ds,
                        frame[consumed..].as_ptr() as *const c_void,
                        &mut ss,
                        ptr::null(),
                    )
                };
                log.rets.push(rc);
                if is_error(rc) {
                    break;
                }
                log.srcs.push(ss);
                log.dsts.push(ds);
                log.out.extend_from_slice(&buf[..ds]);
                consumed += ss;
                if ss == 0 && ds == 0 {
                    break;
                }
            }
            log
        });
        assert_dec_eq(&ctx, &cd, &rd);
        assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
    }

    // 1 byte at a time (small frame keeps the call count sane)
    let ctx = "in-chunk 1 byte at a time";
    let (cd, rd) = both(|l| dec_run(l, &small_frame, 1, small_src.len() + 1, None, None, None));
    assert_dec_eq(ctx, &cd, &rd);
    assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, small_src);

    // exhaustive: split a small frame at EVERY offset
    let tiny_src = gen(&mut rng, 40, Shape::Text);
    for &(cc, bc, cs) in [(0i32, 0i32, 0u64), (1, 0, 0), (0, 1, 0), (1, 1, 40)].iter() {
        let mut p = mkprefs(4, LZ4F_BLOCK_LINKED, cc, bc, 1, 0);
        p.frameInfo.contentSize = cs;
        p.frameInfo.dictID = if cs != 0 { 7 } else { 0 };
        let frame = compress_frame(c(), &tiny_src, Some(&p));
        for k in 0..=frame.len() {
            let ctx = format!("split at {} of {} ({})", k, frame.len(), pname(&p));
            let (cd, rd) = both(|l| {
                let dctx = Dctx::new(l);
                let f_dec = l.get::<Fn_F_decompress>("LZ4F_decompress");
                let mut log = DecLog::default();
                let mut buf = vec![0u8; 64];
                for part in [&frame[..k], &frame[k..]] {
                    let mut off = 0usize;
                    loop {
                        let mut ss = part.len() - off;
                        let mut ds = buf.len();
                        let rc = unsafe {
                            f_dec(
                                dctx.p,
                                buf.as_mut_ptr() as *mut c_void,
                                &mut ds,
                                part[off..].as_ptr() as *const c_void,
                                &mut ss,
                                ptr::null(),
                            )
                        };
                        log.rets.push(rc);
                        if is_error(rc) {
                            return log;
                        }
                        log.srcs.push(ss);
                        log.dsts.push(ds);
                        log.out.extend_from_slice(&buf[..ds]);
                        off += ss;
                        if off >= part.len() || (ss == 0 && ds == 0) {
                            break;
                        }
                    }
                }
                log
            });
            assert_dec_eq(&ctx, &cd, &rd);
            assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, tiny_src);
        }
    }
}

/// CONFIGS.md rows 127, 128 — output chunking: dst buffers of 1, 13,
/// blockSize-1, blockSize, blockSize+1 and huge, which selects between
/// decoding straight into the caller's dst and decoding into `tmpOut` +
/// `dstage_flushOut`.
#[test]
fn frame_decompress_output_chunking() {
    let mut rng = Rng::new(0x0F1A_5EED_0011);
    let big = gen(&mut rng, 200_000, Shape::Text);
    let small = gen(&mut rng, 4000, Shape::Runs);
    for &bsid in [4i32, 5].iter() {
    let bs = blk_size(bsid);
    for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
        for &bc in [0i32, 1].iter() {
            let p = mkprefs(bsid, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, bc, 1, 0);
            let big_frame = compress_frame(c(), &big, Some(&p));
            let small_frame = compress_frame(c(), &small, Some(&p));
            for &out in [1usize, 13, bs - 1, bs, bs + 1, 1 << 20].iter() {
                // tiny dst buffers only on the small source
                let (src, frame): (&Vec<u8>, &Vec<u8>) = if out < 1000 {
                    (&small, &small_frame)
                } else {
                    (&big, &big_frame)
                };
                for &inch in [0usize, 7, 4096].iter() {
                    let ctx = format!("out-chunk {} out={} inch={}", pname(&p), out, inch);
                    let (cd, rd) = both(|l| dec_run(l, frame, inch, out, None, None, None));
                    assert_dec_eq(&ctx, &cd, &rd);
                    assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
                }
            }
        }
    }
    }
}

/// CONFIGS.md rows 133, 134 — `LZ4F_decompressOptions_t.stableDst` 0/1 and
/// `skipChecksums` 0/1.  For stableDst=1 the pledge is honoured: one stable
/// destination buffer, the pointer only ever advances.
#[test]
fn frame_decompress_options_stableDst_skipChecksums() {
    let mut rng = Rng::new(0x0F1A_5EED_0012);
    let src = gen(&mut rng, 200_000, Shape::Text);
    for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
        for &cc in [0i32, 1].iter() {
            for &bc in [0i32, 1].iter() {
                let p = mkprefs(4, bmode, cc, bc, 1, 0);
                let frame = compress_frame(c(), &src, Some(&p));
                for &sd in [0u32, 1].iter() {
                    for &sc in [0u32, 1].iter() {
                        let o = LZ4F_decompressOptions_t {
                            stableDst: sd,
                            skipChecksums: sc,
                            reserved1: 0,
                            reserved0: 0,
                        };
                        for &out in [1000usize, 65535, 65536, 300_000].iter() {
                            let ctx = format!(
                                "dopts {} stableDst={} skipChecksums={} out={}",
                                pname(&p), sd, sc, out
                            );
                            let stable = if sd == 1 { Some(src.len() + 1) } else { None };
                            let (cd, rd) = both(|l| {
                                dec_run(l, &frame, 0, out, Some(&o), stable, None)
                            });
                            assert_dec_eq(&ctx, &cd, &rd);
                            assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 130, 131, 132, 136 — blockMode linked (all the
/// `LZ4F_updateDict` branches: prefix, repoint-into-dst, the tmpOut branches
/// and the >128 KB truncation) vs independent (no dictionary at all), driven
/// with many different output chunk sizes over a multi-block frame.
#[test]
fn frame_decompress_updateDict_branches() {
    let mut rng = Rng::new(0x0F1A_5EED_0013);
    let src = gen(&mut rng, 600_000, Shape::Text);
    for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
        for &bsid in [4i32, 5].iter() {
            let bs = blk_size(bsid);
            let p = mkprefs(bsid, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 1, 0);
            let frame = compress_frame(c(), &src, Some(&p));
            for &out in [
                333usize,
                1000,
                bs / 2,
                bs - 1,
                bs,
                bs + 1,
                65536 + 12345,
                200_000,
            ]
            .iter()
            {
                for &inch in [0usize, 1000].iter() {
                    let ctx = format!("updateDict {} out={} inch={}", pname(&p), out, inch);
                    let (cd, rd) = both(|l| dec_run(l, &frame, inch, out, None, None, None));
                    assert_dec_eq(&ctx, &cd, &rd);
                    assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
                }
            }
        }
    }
}

/// CONFIGS.md row 121 — SKIPPABLE FRAMES, hand-built: 4-byte magic in
/// `[LZ4F_MAGIC_SKIPPABLE_START, +15]`, 4-byte little-endian size, then that
/// many arbitrary bytes.  Fed alone, before a real frame, after a real frame
/// and several in a row, with several input chunk sizes (1 byte at a time hits
/// `dstage_storeSFrameSize` and `dstage_skipSkippable`).
#[test]
fn frame_decompress_skippable_frames() {
    let mut rng = Rng::new(0x0F1A_5EED_0014);
    let plain = gen(&mut rng, 30_000, Shape::Text);
    let p = mkprefs(4, LZ4F_BLOCK_LINKED, LZ4F_CONTENT_CHECKSUM_ENABLED, 1, 1, 0);
    let real = compress_frame(c(), &plain, Some(&p));

    let mk = |magic_idx: u32, size: usize, rng: &mut Rng| -> Vec<u8> {
        let m = LZ4F_MAGIC_SKIPPABLE_START + magic_idx;
        let mut v = m.to_le_bytes().to_vec();
        v.extend_from_slice(&(size as u32).to_le_bytes());
        for _ in 0..size {
            v.push(rng.byte());
        }
        v
    };

    // all 16 magic values, small payload, byte-by-byte and giant feeds
    for idx in 0..16u32 {
        let sk = mk(idx, 8, &mut rng);
        for &ch in [0usize, 1, 4, 5, 7, 8].iter() {
            let ctx = format!("skippable magic+{} alone inch={}", idx, ch);
            let (cd, rd) = both(|l| dec_run(l, &sk, ch, 64, None, None, None));
            assert_dec_eq(&ctx, &cd, &rd);
            assert!(cd.out.is_empty(), "{}: skippable produced output", ctx);
        }
    }

    // every documented size, several placements
    for &size in [0usize, 1, 4, 7, 8, 100, 70000].iter() {
        let sk = mk(3, size, &mut rng);
        let combos: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
            ("alone", sk.clone(), Vec::new()),
            (
                "before real",
                [sk.clone(), real.clone()].concat(),
                plain.clone(),
            ),
            (
                "after real",
                [real.clone(), sk.clone()].concat(),
                plain.clone(),
            ),
            (
                "3 in a row",
                [sk.clone(), sk.clone(), sk.clone()].concat(),
                Vec::new(),
            ),
            (
                "sandwich",
                [sk.clone(), real.clone(), sk.clone(), real.clone()].concat(),
                [plain.clone(), plain.clone()].concat(),
            ),
        ];
        for (name, frame, expect) in combos {
            let chunks: &[usize] = if size >= 70000 { &[0, 7, 4096] } else { &[0, 1, 4, 5, 7, 19, 4096] };
            for &ch in chunks.iter() {
                let ctx = format!("skippable size={} {} inch={}", size, name, ch);
                let (cd, rd) = both(|l| dec_run(l, &frame, ch, expect.len() + 1, None, None, None));
                assert_dec_eq(&ctx, &cd, &rd);
                assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, expect);
            }
        }
    }
}

/// CONFIGS.md rows 116, 122 — CONCATENATED frames: 2-5 valid frames back to
/// back with DIFFERENT preferences each, decoded through ONE dctx (which must
/// reuse / re-grow its internal buffers).
#[test]
fn frame_decompress_concatenated_frames() {
    let mut rng = Rng::new(0x0F1A_5EED_0015);
    let cfgs: Vec<(LZ4F_preferences_t, usize, Shape)> = vec![
        (mkprefs(4, LZ4F_BLOCK_LINKED, 1, 0, 1, 0), 40_000, Shape::Text),
        (mkprefs(6, LZ4F_BLOCK_INDEPENDENT, 0, 1, 1, 0), 130_000, Shape::Runs),
        (mkprefs(4, LZ4F_BLOCK_LINKED, 0, 0, 0, 9), 3, Shape::Constant),
        (mkprefs(5, LZ4F_BLOCK_LINKED, 1, 1, 1, -2), 90_000, Shape::Sparse),
        (mkprefs(4, LZ4F_BLOCK_INDEPENDENT, 1, 0, 1, 0), 0, Shape::Text),
    ];
    let mut frames = Vec::new();
    let mut plains = Vec::new();
    for (p, sz, sh) in cfgs.iter() {
        let src = gen(&mut rng, *sz, *sh);
        frames.push(compress_frame(c(), &src, Some(p)));
        plains.push(src);
    }
    for n in 2..=5usize {
        for start in 0..=(5 - n) {
            let mut blob = Vec::new();
            let mut expect = Vec::new();
            for i in start..start + n {
                blob.extend_from_slice(&frames[i]);
                expect.extend_from_slice(&plains[i]);
            }
            for &ch in [0usize, 7, 19, 3000].iter() {
                for &out in [500usize, 65536, 300_000].iter() {
                    let ctx = format!("concat n={} start={} inch={} out={}", n, start, ch, out);
                    let (cd, rd) = both(|l| dec_run(l, &blob, ch, out, None, None, None));
                    assert_dec_eq(&ctx, &cd, &rd);
                    assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, expect);
                }
            }
        }
    }
}

/// CONFIGS.md row 137 — `LZ4F_decompress_usingDict` with the dictionary
/// supplied while `dStage <= dstage_init` (honoured) and mid-frame (silently
/// ignored).  The frames really use a dictionary, built with
/// `LZ4F_compressBegin_usingDict`.
#[test]
fn frame_decompress_usingDict() {
    let mut rng = Rng::new(0x0F1A_5EED_0016);
    let dict = gen(&mut rng, 60_000, Shape::Text);
    let src = gen(&mut rng, 80_000, Shape::Text);
    for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
        for &lvl in [0i32, 9].iter() {
            let p = mkprefs(4, bmode, LZ4F_CONTENT_CHECKSUM_ENABLED, 0, 1, lvl);
            let ctx = format!("usingDict {}", pname(&p));
            let (cl, rl) = both(|l| {
                stream_compress(
                    l,
                    &src,
                    Some(&p),
                    Begin::UsingDict,
                    &dict,
                    &[9000usize],
                    &[Upd::Compressed],
                    0,
                    None,
                )
            });
            assert_eq!(cl.rets, rl.rets, "{}: compress rets", ctx);
            assert_bytes_eq!(ctx, cl.out, rl.out);

            // dict honoured (installed on a fresh dctx)
            for &inch in [0usize, 7, 5000].iter() {
                for &out in [1000usize, 100_000].iter() {
                    let c2 = format!("{} honoured inch={} out={}", ctx, inch, out);
                    let (cd, rd) = both(|l| {
                        dec_run(l, &cl.out, inch, out, None, None, Some(&dict))
                    });
                    assert_dec_eq(&c2, &cd, &rd);
                    assert_bytes_eq!(format!("{} plaintext", c2), cd.out, src);
                }
            }

            // dict supplied mid-frame -> silently ignored (both libraries must
            // agree on the resulting failure / garbage)
            let (cm, rm) = both(|l| {
                let dctx = Dctx::new(l);
                let f_dec = l.get::<Fn_F_decompress>("LZ4F_decompress");
                let f_decd = l.get::<Fn_F_decompress_usingDict>("LZ4F_decompress_usingDict");
                let mut log = DecLog::default();
                let mut buf = vec![0u8; 100_000];
                // step 1: header only, no dict
                let mut ss = 7usize;
                let mut ds = buf.len();
                let rc = unsafe {
                    f_dec(
                        dctx.p,
                        buf.as_mut_ptr() as *mut c_void,
                        &mut ds,
                        cl.out.as_ptr() as *const c_void,
                        &mut ss,
                        ptr::null(),
                    )
                };
                log.rets.push(rc);
                log.srcs.push(ss);
                log.dsts.push(ds);
                let mut consumed = ss;
                // step 2+: the rest, now offering the dictionary (ignored)
                let mut rc = 1usize;
                let mut guard = 0;
                while consumed < cl.out.len() && rc != 0 && guard < 10_000 {
                    guard += 1;
                    let mut ss = cl.out.len() - consumed;
                    let mut ds = buf.len();
                    rc = unsafe {
                        f_decd(
                            dctx.p,
                            buf.as_mut_ptr() as *mut c_void,
                            &mut ds,
                            cl.out[consumed..].as_ptr() as *const c_void,
                            &mut ss,
                            dict.as_ptr() as *const c_void,
                            dict.len(),
                            ptr::null(),
                        )
                    };
                    log.rets.push(rc);
                    if is_error(rc) {
                        break;
                    }
                    log.srcs.push(ss);
                    log.dsts.push(ds);
                    log.out.extend_from_slice(&buf[..ds]);
                    consumed += ss;
                    if ss == 0 && ds == 0 {
                        break;
                    }
                }
                log
            });
            assert_dec_eq(&format!("{} mid-frame dict ignored", ctx), &cm, &rm);
        }
    }
}

/// CONFIGS.md row 118 — `LZ4F_getFrameInfo` on the happy path for every
/// header-length combination: every field of the returned
/// `LZ4F_frameInfo_t` must match, as must the return value and
/// `*srcSizePtr`.  Also the "already started" variant (dStage > storeFrameHeader)
/// which consumes no input.
#[test]
fn frame_getFrameInfo_happy_path() {
    let mut rng = Rng::new(0x0F1A_5EED_0017);
    let src = gen(&mut rng, 70_000, Shape::Text);
    for &bsid in BSIDS.iter() {
        for &bmode in [LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT].iter() {
            for &cc in [0i32, 1].iter() {
                for &bc in [0i32, 1].iter() {
                    for &declare in [false, true].iter() {
                        for &did in [0u32, 0x0BAD_F00D].iter() {
                            let mut p = mkprefs(bsid, bmode, cc, bc, 1, 0);
                            if declare {
                                p.frameInfo.contentSize = src.len() as u64;
                            }
                            p.frameInfo.dictID = did;
                            // built with the streaming API so the header
                            // reflects the preferences verbatim (the one-shot
                            // API rewrites blockSizeID / blockMode)
                            let frame = stream_compress(
                                c(),
                                &src,
                                Some(&p),
                                Begin::Plain,
                                &[],
                                &[9000usize],
                                &[Upd::Compressed],
                                0,
                                None,
                            )
                            .out;
                            let hdr = 7 + if declare { 8 } else { 0 }
                                + if did != 0 { 4 } else { 0 };
                            // blockSizeID 0 is coerced to LZ4F_max64KB
                            let expect_bsid = if bsid == 0 { 4 } else { bsid };
                            let ctx = format!("getFrameInfo {}", pname(&p));

                            // fresh dctx, full frame available
                            for &avail in [hdr, hdr + 1, frame.len()].iter() {
                                let (cv, rv) = both(|l| {
                                    let dctx = Dctx::new(l);
                                    let mut fi = LZ4F_frameInfo_t::default();
                                    let mut sz = avail;
                                    let rc = unsafe {
                                        l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                                            dctx.p,
                                            &mut fi,
                                            frame.as_ptr() as *const c_void,
                                            &mut sz,
                                        )
                                    };
                                    // second call: already-decoded path
                                    let mut fi2 = LZ4F_frameInfo_t::default();
                                    let mut sz2 = frame.len() - sz;
                                    let rc2 = unsafe {
                                        l.get::<Fn_F_getFrameInfo>("LZ4F_getFrameInfo")(
                                            dctx.p,
                                            &mut fi2,
                                            frame[sz..].as_ptr() as *const c_void,
                                            &mut sz2,
                                        )
                                    };
                                    (rc, sz, fi, rc2, sz2, fi2)
                                });
                                assert_eq!(
                                    cv.0, rv.0,
                                    "{} avail={}: rc C={} Rust={}",
                                    ctx, avail, show(cv.0), show(rv.0)
                                );
                                assert_eq!(cv.1, rv.1, "{} avail={}: srcSizePtr", ctx, avail);
                                assert_eq!(cv.2, rv.2, "{} avail={}: frameInfo", ctx, avail);
                                assert_eq!(cv.3, rv.3, "{} avail={}: rc2", ctx, avail);
                                assert_eq!(cv.4, rv.4, "{} avail={}: srcSizePtr2", ctx, avail);
                                assert_eq!(cv.5, rv.5, "{} avail={}: frameInfo2", ctx, avail);
                                assert_eq!(cv.0, LZ4F_BLOCK_HEADER_SIZE, "{}: rc must be BHSize", ctx);
                                assert_eq!(cv.1, hdr, "{}: consumed header size", ctx);
                                assert_eq!(cv.4, 0, "{}: already-started consumes nothing", ctx);
                                assert_eq!(cv.2.blockSizeID, expect_bsid, "{}: blockSizeID", ctx);
                                assert_eq!(cv.2.blockMode, bmode, "{}: blockMode", ctx);
                                assert_eq!(cv.2.contentChecksumFlag, cc, "{}: cc", ctx);
                                assert_eq!(cv.2.blockChecksumFlag, bc, "{}: bc", ctx);
                                assert_eq!(cv.2.frameType, LZ4F_FRAME, "{}: frameType", ctx);
                                assert_eq!(cv.2.dictID, did, "{}: dictID", ctx);
                                assert_eq!(
                                    cv.2.contentSize,
                                    if declare { src.len() as u64 } else { 0 },
                                    "{}: contentSize",
                                    ctx
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 116, 122 — `LZ4F_resetDecompressionContext` mid-frame then
/// reuse for a fresh frame; reuse of one dctx for many frames WITHOUT a reset;
/// and the dStage reported by `LZ4F_freeDecompressionContext`.
#[test]
fn frame_dctx_reset_and_reuse() {
    let mut rng = Rng::new(0x0F1A_5EED_0018);
    let a = gen(&mut rng, 90_000, Shape::Text);
    let b = gen(&mut rng, 40_000, Shape::Runs);
    let pa = mkprefs(4, LZ4F_BLOCK_LINKED, 1, 1, 1, 0);
    let pb = mkprefs(6, LZ4F_BLOCK_INDEPENDENT, 0, 0, 1, 0);
    let fa = compress_frame(c(), &a, Some(&pa));
    let fb = compress_frame(c(), &b, Some(&pb));

    // (1) partial decode, reset, then a fresh frame on the same dctx
    for &cut in [1usize, 7, 19, 40, 5000].iter() {
        let ctx = format!("reset after {} bytes", cut);
        let (cv, rv) = both(|l| {
            let dctx = Dctx::new(l);
            let f_dec = l.get::<Fn_F_decompress>("LZ4F_decompress");
            let mut buf = vec![0u8; 100_000];
            let mut ss = cut.min(fa.len());
            let mut ds = buf.len();
            let rc1 = unsafe {
                f_dec(
                    dctx.p,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    fa.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                )
            };
            dctx.reset();
            let log = dec_run_on(l, &dctx, &fb, 0, buf.len(), None, None, None);
            (rc1, ss, ds, log)
        });
        assert_eq!(cv.0, rv.0, "{}: partial rc C={} Rust={}", ctx, show(cv.0), show(rv.0));
        assert_eq!(cv.1, rv.1, "{}: partial srcSizePtr", ctx);
        assert_eq!(cv.2, rv.2, "{}: partial dstSizePtr", ctx);
        assert_dec_eq(&format!("{} after reset", ctx), &cv.3, &rv.3);
        assert_bytes_eq!(format!("{} after reset plaintext", ctx), cv.3.out, b);
    }

    // (2) many frames through one dctx with NO reset in between
    let (cv, rv) = both(|l| {
        let dctx = Dctx::new(l);
        let mut logs = Vec::new();
        for i in 0..6 {
            let (f, out) = if i % 2 == 0 { (&fa, a.len() + 1) } else { (&fb, b.len() + 1) };
            logs.push(dec_run_on(l, &dctx, f, 0, out, None, None, None));
        }
        logs
    });
    for (i, (cl, rl)) in cv.iter().zip(rv.iter()).enumerate() {
        assert_dec_eq(&format!("no-reset reuse #{}", i), cl, rl);
        let want = if i % 2 == 0 { &a } else { &b };
        assert_bytes_eq!(format!("no-reset reuse #{} plaintext", i), cl.out, want);
    }

    // (3) LZ4F_freeDecompressionContext reports the current dStage
    for &cut in [0usize, 1, 5, 7, 19, 100, 5000].iter() {
        let ctx = format!("free reports dStage after {} bytes", cut);
        let (cd, rd) = both(|l| {
            let mut p: *mut c_void = ptr::null_mut();
            let rc = unsafe {
                l.get::<Fn_F_createDecompressionContext>("LZ4F_createDecompressionContext")(
                    &mut p,
                    LZ4F_VERSION,
                )
            };
            assert!(!is_error(rc));
            let mut buf = vec![0u8; 100_000];
            let mut ss = cut.min(fa.len());
            let mut ds = buf.len();
            let rc1 = unsafe {
                l.get::<Fn_F_decompress>("LZ4F_decompress")(
                    p,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut ds,
                    fa.as_ptr() as *const c_void,
                    &mut ss,
                    ptr::null(),
                )
            };
            let free_rc = unsafe {
                l.get::<Fn_F_freeDecompressionContext>("LZ4F_freeDecompressionContext")(p)
            };
            (rc1, ss, ds, free_rc)
        });
        assert_eq!(cd, rd, "{}: C={:?} Rust={:?}", ctx, cd, rd);
    }
}

// ===========================================================================
// large randomized end-to-end fuzz
// ===========================================================================

fn fuzz_prefs(rng: &mut Rng) -> LZ4F_preferences_t {
    const FBSIDS: [c_int; 4] = [0, 4, 5, 6];
    const FLEVELS: [c_int; 7] = [-5, -1, 0, 1, 2, 9, 12];
    let bsid = FBSIDS[rng.below(FBSIDS.len())];
    let bmode = if rng.bool() { LZ4F_BLOCK_LINKED } else { LZ4F_BLOCK_INDEPENDENT };
    let cc = if rng.bool() { 1 } else { 0 };
    let bc = if rng.bool() { 1 } else { 0 };
    let af = if rng.bool() { 1u32 } else { 0u32 };
    let lvl = FLEVELS[rng.below(FLEVELS.len())];
    let mut p = mkprefs(bsid, bmode, cc, bc, af, lvl);
    p.favorDecSpeed = if rng.bool() { 1 } else { 0 };
    if rng.bool() {
        p.frameInfo.dictID = rng.next_u32();
    }
    p
}

/// CONFIGS.md rows 127-137 — a decoder-focused randomized fuzz (fixed seed)
/// aimed at the `LZ4F_updateDict` / `tmpOut` / tail-history-preservation
/// machinery: random linked and independent frames decoded with random (mostly
/// small and odd) input and output chunk sizes, random `stableDst` /
/// `skipChecksums`, and sometimes an external dictionary.
#[test]
fn frame_decompress_dictionary_fuzz() {
    let mut rng = Rng::new(0xD1C7_F022_0001);
    let dict = gen(&mut rng, 90_000, Shape::Text);
    for it in 0..600usize {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let sz = match rng.below(5) {
            0 => rng.range(1, 2000),
            1 => rng.range(60_000, 140_000),
            2 => rng.range(1, 300_000),
            3 => 65536,
            _ => rng.range(1, 70_000),
        };
        let src = gen(&mut rng, sz, shape);
        let bsid = [4i32, 4, 4, 5, 6][rng.below(5)];
        let bmode = if rng.below(4) == 0 {
            LZ4F_BLOCK_INDEPENDENT
        } else {
            LZ4F_BLOCK_LINKED
        };
        let mut p = mkprefs(
            bsid,
            bmode,
            if rng.bool() { 1 } else { 0 },
            if rng.bool() { 1 } else { 0 },
            if rng.bool() { 1 } else { 0 },
            0,
        );
        if rng.bool() {
            p.frameInfo.contentSize = sz as u64;
        }
        let with_dict = rng.below(4) == 0;
        let dsz = if with_dict { rng.range(1, dict.len()) } else { 0 };
        let d = &dict[..dsz];

        let frame = stream_compress(
            c(),
            &src,
            Some(&p),
            if with_dict { Begin::UsingDict } else { Begin::Plain },
            d,
            &[rng.range(1, sz.max(1)).max(1)],
            &[Upd::Compressed],
            0,
            None,
        )
        .out;

        // small / odd chunk sizes, but keep the iteration count bounded
        let outch = match rng.below(7) {
            0 => 1,
            1 => 13,
            2 => 333,
            3 => 65535,
            4 => 65536,
            5 => 65537,
            _ => rng.range(1, 200_000),
        };
        let outch = outch.max(sz / 4096 + 1);
        let inch = match rng.below(5) {
            0 => 0,
            1 => 7,
            2 => 19,
            3 => rng.range(1, frame.len().max(1)),
            _ => rng.range(1, 5000),
        };
        let inch = if inch == 0 { 0 } else { inch.max(frame.len() / 4096 + 1) };
        let dopt = LZ4F_decompressOptions_t {
            stableDst: if rng.bool() { 1 } else { 0 },
            skipChecksums: if rng.bool() { 1 } else { 0 },
            reserved1: 0,
            reserved0: 0,
        };
        let stable_cap = if dopt.stableDst == 1 { Some(sz + 1) } else { None };
        let ctx = format!(
            "decfuzz#{} shape={:?} size={} {} dict={} inch={} outch={} dopt={:?}",
            it, shape, sz, pname(&p), dsz, inch, outch, dopt
        );
        let (cd, rd) = both(|l| {
            dec_run(
                l,
                &frame,
                inch,
                outch,
                Some(&dopt),
                stable_cap,
                if with_dict { Some(d) } else { None },
            )
        });
        assert_dec_eq(&ctx, &cd, &rd);
        assert_bytes_eq!(format!("{} plaintext", ctx), cd.out, src);
    }
}

/// A large randomized end-to-end fuzz (fixed seed): random valid preferences,
/// random shape/size, random compress-chunk sizes, random decompress input and
/// output chunk sizes, random decompressOptions.  Every intermediate return
/// code, the compressed bytes and the recovered plaintext must match between
/// the two libraries, and the plaintext must equal the original.
#[test]
fn frame_end_to_end_fuzz() {
    let mut rng = Rng::new(0x5EED_F022_ABCD);
    let iters = 3000usize;
    for it in 0..iters {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let sz = match rng.below(8) {
            0 => 0,
            1 => rng.below(64),
            2 => rng.range(64, 4096),
            3 => rng.range(4096, 70_000),
            4 => 65536,
            5 => 65537,
            6 => rng.range(1, 300_000),
            _ => rng.range(1, 20_000),
        };
        let mut p = fuzz_prefs(&mut rng);
        if !affordable(sz, p.compressionLevel) {
            p.compressionLevel = 0;
        }
        if rng.bool() {
            p.frameInfo.contentSize = sz as u64;
        }
        let src = gen(&mut rng, sz, shape);

        // random compression chunk plan
        let mut plan = Vec::new();
        let nch = rng.range(1, 4);
        for _ in 0..nch {
            let n = match rng.below(6) {
                0 => 1,
                1 => rng.range(1, 64),
                2 => rng.range(1, 5000),
                3 => blk_size(if p.frameInfo.blockSizeID == 0 { 4 } else { p.frameInfo.blockSizeID }),
                4 => sz.max(1),
                _ => rng.range(1, 100_000),
            };
            plan.push(n.max(1));
        }
        // avoid pathological call counts
        if sz / plan.iter().cloned().min().unwrap_or(1) > 4096 {
            plan = vec![rng.range(sz / 512 + 1, sz.max(1))];
        }
        let flush_every = if rng.below(4) == 0 { rng.range(1, 3) } else { 0 };
        let stable = if rng.bool() {
            Some(LZ4F_compressOptions_t { stableSrc: 1, reserved: [0; 3] })
        } else {
            None
        };
        let modes = if p.frameInfo.blockMode == LZ4F_BLOCK_INDEPENDENT && rng.below(4) == 0 {
            vec![Upd::Compressed, Upd::Uncompressed]
        } else {
            vec![Upd::Compressed]
        };

        let ctx = format!(
            "fuzz#{} shape={:?} size={} {} plan={:?} flush={} stableSrc={}",
            it,
            shape,
            sz,
            pname(&p),
            plan,
            flush_every,
            stable.is_some()
        );

        let (cl, rl) = both(|l| {
            stream_compress(
                l,
                &src,
                Some(&p),
                Begin::Plain,
                &[],
                &plan,
                &modes,
                flush_every,
                stable.as_ref(),
            )
        });
        assert_eq!(cl.rets, rl.rets, "{}: compression return values", ctx);
        assert_bytes_eq!(format!("{} compressed", ctx), cl.out, rl.out);
        assert!(
            !cl.rets.iter().any(|&r| is_error(r)),
            "{}: C compression failed: {:?}",
            ctx,
            cl.rets.iter().map(|&r| show(r)).collect::<Vec<_>>()
        );

        // one-shot must match too
        let (cf, rf) = both(|l| compress_frame(l, &src, Some(&p)));
        assert_bytes_eq!(format!("{} one-shot", ctx), cf, rf);

        // random decompression plan
        let flen = cl.out.len();
        let inch = match rng.below(5) {
            0 => 0,
            1 => rng.range((flen / 256).max(1), flen.max(1)),
            2 => 7,
            3 => rng.range((flen / 64).max(1), flen.max(1)),
            _ => flen.max(1),
        };
        let outch = match rng.below(6) {
            0 => rng.range((sz / 256).max(1), sz.max(1)),
            1 => 13,
            2 => 65536,
            3 => 65535,
            4 => sz + 1,
            _ => rng.range(1, 200_000),
        };
        let dopt = LZ4F_decompressOptions_t {
            stableDst: if rng.bool() { 1 } else { 0 },
            skipChecksums: if rng.bool() { 1 } else { 0 },
            reserved1: 0,
            reserved0: 0,
        };
        let stable_cap = if dopt.stableDst == 1 { Some(sz + 1) } else { None };
        let ctx2 = format!("{} | inch={} outch={} dopt={:?}", ctx, inch, outch, dopt);
        let (cd, rd) = both(|l| {
            dec_run(l, &cl.out, inch, outch, Some(&dopt), stable_cap, None)
        });
        assert_dec_eq(&ctx2, &cd, &rd);
        assert_bytes_eq!(format!("{} plaintext", ctx2), cd.out, src);
    }
}
