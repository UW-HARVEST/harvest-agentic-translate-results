//! Shared LZ4F driver used by the frame and file test files.
#![allow(dead_code)]
#![allow(non_snake_case)]

use super::*;
use libloading::Library;

pub type FnCreateCtx = unsafe extern "C" fn(*mut *mut CVoid, u32) -> usize;
pub type FnFreeCtx = unsafe extern "C" fn(*mut CVoid) -> usize;
pub type FnCompressFrame =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const LZ4F_preferences_t) -> usize;
pub type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
pub type FnBegin =
    unsafe extern "C" fn(*mut CVoid, *mut u8, usize, *const LZ4F_preferences_t) -> usize;
pub type FnBeginDict = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
pub type FnBeginCDict =
    unsafe extern "C" fn(*mut CVoid, *mut u8, usize, *const CVoid, *const LZ4F_preferences_t) -> usize;
pub type FnBeginInternal = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const CVoid,
    *const LZ4F_preferences_t,
) -> usize;
pub type FnUpdate = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
pub type FnFlush =
    unsafe extern "C" fn(*mut CVoid, *mut u8, usize, *const LZ4F_compressOptions_t) -> usize;
pub type FnHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
pub type FnGetFrameInfo =
    unsafe extern "C" fn(*mut CVoid, *mut LZ4F_frameInfo_t, *const u8, *mut usize) -> usize;
pub type FnDecompress = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
pub type FnDecompressDict = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const u8,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
pub type FnResetDctx = unsafe extern "C" fn(*mut CVoid);
pub type FnCreateCDict = unsafe extern "C" fn(*const u8, usize) -> *mut CVoid;
pub type FnFreeCDict = unsafe extern "C" fn(*mut CVoid);
pub type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut CVoid,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const CVoid,
    *const LZ4F_preferences_t,
) -> usize;
pub type FnIsError = unsafe extern "C" fn(usize) -> u32;
pub type FnErrName = unsafe extern "C" fn(usize) -> *const CChar;
pub type FnErrCode = unsafe extern "C" fn(usize) -> i32;
pub type FnGetBlockSize = unsafe extern "C" fn(i32) -> usize;
pub type FnCreateCtxAdv = unsafe extern "C" fn(LZ4F_CustomMem, u32) -> *mut CVoid;
pub type FnCreateCDictAdv = unsafe extern "C" fn(LZ4F_CustomMem, *const u8, usize) -> *mut CVoid;

/// Result of a full compression run: return codes plus the produced frame.
#[derive(Debug, PartialEq, Eq)]
pub struct CompRun {
    pub codes: Vec<i64>,
    pub frame: Vec<u8>,
}

fn as_code(r: usize) -> i64 {
    // Error codes are tiny negative values when reinterpreted as ptrdiff_t.
    let s = r as i64;
    if s < 0 && s > -1024 { s } else { r as i64 }
}

/// One-shot frame compression.
pub fn compress_frame(
    lib: &Library,
    src: &[u8],
    prefs: Option<&LZ4F_preferences_t>,
    cap_adjust: i64,
) -> CompRun {
    unsafe {
        let pp = prefs.map_or(std::ptr::null(), |p| p as *const _);
        let bound = sym::<FnBound>(lib, "LZ4F_compressFrameBound")(src.len(), pp);
        // Nonsense preference values (e.g. a blockChecksumFlag of 2^20) make the
        // bound arithmetic in the C overflow into absurd numbers. Compare the
        // bound itself, but do not try to allocate it.
        if bound > (1usize << 27) {
            return CompRun {
                codes: vec![as_code(bound), -1_000_000_001, -1],
                frame: Vec::new(),
            };
        }
        let cap = if (bound as i64) + cap_adjust < 0 {
            0usize
        } else {
            ((bound as i64) + cap_adjust) as usize
        };
        let mut dst = vec![0xA5u8; cap + 64];
        let r = sym::<FnCompressFrame>(lib, "LZ4F_compressFrame")(
            dst.as_mut_ptr(),
            cap,
            src.as_ptr(),
            src.len(),
            pp,
        );
        let ok = sym::<FnIsError>(lib, "LZ4F_isError")(r) == 0;
        let frame = if ok { dst[..r].to_vec() } else { Vec::new() };
        CompRun {
            codes: vec![as_code(bound), as_code(r), ok as i64],
            frame,
        }
    }
}

/// How the streaming compressor should be started.
#[derive(Copy, Clone, Debug)]
pub enum BeginMode<'a> {
    Plain,
    UsingDict(&'a [u8]),
    UsingDictOnce(&'a [u8]),
    UsingCDict(&'a [u8]),
    /// `LZ4F_compressBegin_internal` with an explicit dict / cdict pair.
    Internal(Option<&'a [u8]>, Option<&'a [u8]>),
}

/// Which update entry point to use for a chunk.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpdKind {
    Compressed,
    Uncompressed,
}

pub struct StreamPlan<'a> {
    pub begin: BeginMode<'a>,
    pub prefs: Option<LZ4F_preferences_t>,
    pub copts: Option<LZ4F_compressOptions_t>,
    /// (chunk length, entry point, flush-after?)
    pub steps: Vec<(usize, UpdKind, bool)>,
}

/// Full streaming compression: begin → update* (+flush) → end.
pub fn compress_stream(lib: &Library, src: &[u8], plan: &StreamPlan) -> CompRun {
    unsafe {
        let mut cctx: *mut CVoid = std::ptr::null_mut();
        let cr = sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut cctx, 100);
        let mut codes = vec![as_code(cr)];
        if sym::<FnIsError>(lib, "LZ4F_isError")(cr) != 0 || cctx.is_null() {
            return CompRun { codes, frame: Vec::new() };
        }
        let pp = plan.prefs.as_ref().map_or(std::ptr::null(), |p| p as *const _);
        let cp = plan.copts.as_ref().map_or(std::ptr::null(), |p| p as *const _);
        let mut frame = Vec::new();
        let mut hdr = vec![0u8; 64];

        let hr = match plan.begin {
            BeginMode::Plain => sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                cctx,
                hdr.as_mut_ptr(),
                hdr.len(),
                pp,
            ),
            BeginMode::UsingDict(d) => sym::<FnBeginDict>(lib, "LZ4F_compressBegin_usingDict")(
                cctx,
                hdr.as_mut_ptr(),
                hdr.len(),
                d.as_ptr(),
                d.len(),
                pp,
            ),
            BeginMode::UsingDictOnce(d) => {
                sym::<FnBeginDict>(lib, "LZ4F_compressBegin_usingDictOnce")(
                    cctx,
                    hdr.as_mut_ptr(),
                    hdr.len(),
                    d.as_ptr(),
                    d.len(),
                    pp,
                )
            }
            BeginMode::UsingCDict(d) => {
                let cd = sym::<FnCreateCDict>(lib, "LZ4F_createCDict")(d.as_ptr(), d.len());
                let r = sym::<FnBeginCDict>(lib, "LZ4F_compressBegin_usingCDict")(
                    cctx,
                    hdr.as_mut_ptr(),
                    hdr.len(),
                    cd,
                    pp,
                );
                // NOTE: the CDict must outlive the session, so it is leaked here
                // deliberately and freed after compressEnd below via `cd_hold`.
                CD_HOLD.with(|h| h.set(cd as usize));
                r
            }
            BeginMode::Internal(d, c) => {
                let cd = c.map(|dd| {
                    sym::<FnCreateCDict>(lib, "LZ4F_createCDict")(dd.as_ptr(), dd.len())
                });
                if let Some(x) = cd {
                    CD_HOLD.with(|h| h.set(x as usize));
                }
                sym::<FnBeginInternal>(lib, "LZ4F_compressBegin_internal")(
                    cctx,
                    hdr.as_mut_ptr(),
                    hdr.len(),
                    d.map_or(std::ptr::null(), |x| x.as_ptr()),
                    d.map_or(0, |x| x.len()),
                    cd.unwrap_or(std::ptr::null_mut()) as *const CVoid,
                    pp,
                )
            }
        };
        codes.push(as_code(hr));
        if sym::<FnIsError>(lib, "LZ4F_isError")(hr) != 0 {
            drop_cdict(lib);
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
            return CompRun { codes, frame };
        }
        frame.extend_from_slice(&hdr[..hr]);

        let upd = sym::<FnUpdate>(lib, "LZ4F_compressUpdate");
        let uupd = sym::<FnUpdate>(lib, "LZ4F_uncompressedUpdate");
        let flush = sym::<FnFlush>(lib, "LZ4F_flush");
        let bound = sym::<FnBound>(lib, "LZ4F_compressBound");
        let is_err = sym::<FnIsError>(lib, "LZ4F_isError");

        // Allocate the scratch buffers once: `LZ4F_compressBound` for a 4 MB
        // block size is multi-megabyte, and re-allocating per step turns a
        // 1-byte-chunk run into gigabytes of allocator churn.
        let max_chunk = plan
            .steps
            .iter()
            .map(|&(c, _, _)| c.min(src.len()))
            .max()
            .unwrap_or(0);
        let raw_cap = bound(max_chunk, pp).max(max_chunk + 64);
        if raw_cap > (1usize << 27) {
            codes.push(-1_000_000_002);
            drop_cdict(lib);
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
            return CompRun { codes, frame: Vec::new() };
        }
        let step_cap = raw_cap;
        let mut dst = vec![0u8; step_cap];
        let flush_cap = bound(0, pp).max(64);
        let mut fd = vec![0u8; flush_cap];

        let mut off = 0usize;
        let mut failed = false;
        for &(clen, kind, do_flush) in plan.steps.iter() {
            if off >= src.len() {
                break;
            }
            let n = clen.min(src.len() - off);
            let cap = bound(n, pp).max(n + 64).min(step_cap);
            let f = if kind == UpdKind::Compressed { &upd } else { &uupd };
            let r = f(cctx, dst.as_mut_ptr(), cap, src[off..].as_ptr(), n, cp);
            codes.push(as_code(r));
            if is_err(r) != 0 {
                failed = true;
                break;
            }
            frame.extend_from_slice(&dst[..r]);
            off += n;
            if do_flush {
                let fr = flush(cctx, fd.as_mut_ptr(), flush_cap, cp);
                codes.push(as_code(fr));
                if is_err(fr) != 0 {
                    failed = true;
                    break;
                }
                frame.extend_from_slice(&fd[..fr]);
            }
        }
        if !failed {
            let ecap = flush_cap;
            let er = sym::<FnFlush>(lib, "LZ4F_compressEnd")(cctx, fd.as_mut_ptr(), ecap, cp);
            codes.push(as_code(er));
            if is_err(er) == 0 {
                frame.extend_from_slice(&fd[..er]);
            } else {
                frame.clear();
            }
        } else {
            frame.clear();
        }
        drop_cdict(lib);
        sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
        CompRun { codes, frame }
    }
}

thread_local! {
    static CD_HOLD: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn drop_cdict(lib: &Library) {
    let p = CD_HOLD.with(|h| h.replace(0));
    if p != 0 {
        unsafe { sym::<FnFreeCDict>(lib, "LZ4F_freeCDict")(p as *mut CVoid) }
    }
}

/// Result of a full decompression run.
#[derive(Debug, PartialEq, Eq)]
pub struct DecRun {
    pub codes: Vec<i64>,
    pub out: Vec<u8>,
    pub consumed: usize,
}

/// Drive `LZ4F_decompress` (or `_usingDict`) to completion.
///
/// `src_chunk == 0` feeds the whole frame in one call; `dst_chunk == 0` offers
/// the whole output buffer at once.
pub fn decompress_frame(
    lib: &Library,
    frame: &[u8],
    expected_out: usize,
    src_chunk: usize,
    dst_chunk: usize,
    dopts: Option<&LZ4F_decompressOptions_t>,
    dict: Option<&[u8]>,
    use_getframeinfo: bool,
) -> DecRun {
    unsafe {
        let mut dctx: *mut CVoid = std::ptr::null_mut();
        let cr = sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
        let mut codes = vec![as_code(cr)];
        let is_err = sym::<FnIsError>(lib, "LZ4F_isError");
        if is_err(cr) != 0 || dctx.is_null() {
            return DecRun { codes, out: Vec::new(), consumed: 0 };
        }
        let dp = dopts.map_or(std::ptr::null(), |p| p as *const _);
        let mut out = vec![0u8; expected_out + 4096];
        let mut written = 0usize;
        let mut consumed = 0usize;

        if use_getframeinfo {
            let mut fi = LZ4F_frameInfo_t::default();
            let mut ss = frame.len();
            let r = sym::<FnGetFrameInfo>(lib, "LZ4F_getFrameInfo")(
                dctx,
                &mut fi,
                frame.as_ptr(),
                &mut ss,
            );
            codes.push(as_code(r));
            codes.push(ss as i64);
            codes.push(fi.blockSizeID as i64);
            codes.push(fi.blockMode as i64);
            codes.push(fi.contentChecksumFlag as i64);
            codes.push(fi.frameType as i64);
            codes.push(fi.contentSize as i64);
            codes.push(fi.dictID as i64);
            codes.push(fi.blockChecksumFlag as i64);
            if is_err(r) != 0 {
                sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
                return DecRun { codes, out: Vec::new(), consumed: 0 };
            }
            consumed = ss;
        }

        let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
        let decd = sym::<FnDecompressDict>(lib, "LZ4F_decompress_usingDict");
        let mut guard = 0usize;
        loop {
            guard += 1;
            if guard > 4_000_000 {
                codes.push(-9_999_999);
                break;
            }
            if consumed >= frame.len() {
                break;
            }
            let avail_src = frame.len() - consumed;
            let mut sn = if src_chunk == 0 { avail_src } else { src_chunk.min(avail_src) };
            let avail_dst = out.len() - written;
            let mut dn = if dst_chunk == 0 { avail_dst } else { dst_chunk.min(avail_dst) };
            if dn == 0 {
                codes.push(-8_888_888);
                break;
            }
            let r = match dict {
                None => dec(
                    dctx,
                    out[written..].as_mut_ptr(),
                    &mut dn,
                    frame[consumed..].as_ptr(),
                    &mut sn,
                    dp,
                ),
                Some(d) => decd(
                    dctx,
                    out[written..].as_mut_ptr(),
                    &mut dn,
                    frame[consumed..].as_ptr(),
                    &mut sn,
                    d.as_ptr(),
                    d.len(),
                    dp,
                ),
            };
            codes.push(as_code(r));
            if is_err(r) != 0 {
                out.truncate(written);
                sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
                return DecRun { codes, out, consumed };
            }
            written += dn;
            consumed += sn;
            if r == 0 {
                break; // frame fully decoded
            }
            if sn == 0 && dn == 0 {
                codes.push(-7_777_777);
                break;
            }
        }
        out.truncate(written);
        sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
        DecRun { codes, out, consumed }
    }
}

/* ------------------------------------------------------------------ */
/* preference enumeration                                             */
/* ------------------------------------------------------------------ */

pub const BLOCK_SIZE_IDS: [i32; 5] = [0, 4, 5, 6, 7];
pub const FRAME_LEVELS: [i32; 10] = [-5, -1, 0, 1, 2, 3, 9, 10, 12, 20];

/// The pruned cross-product of the frame options the C branches on.
pub fn pref_matrix() -> Vec<LZ4F_preferences_t> {
    let mut v = Vec::new();
    // full cross-product over the header-affecting flags at a fast and an HC level
    for &bsid in BLOCK_SIZE_IDS.iter() {
        for &bmode in [0i32, 1].iter() {
            for &cc in [0i32, 1].iter() {
                for &bc in [0i32, 1].iter() {
                    for &lvl in [0i32, 9].iter() {
                        for &af in [0u32, 1].iter() {
                            v.push(LZ4F_preferences_t {
                                frameInfo: LZ4F_frameInfo_t {
                                    blockSizeID: bsid,
                                    blockMode: bmode,
                                    contentChecksumFlag: cc,
                                    frameType: 0,
                                    contentSize: 0,
                                    dictID: 0,
                                    blockChecksumFlag: bc,
                                },
                                compressionLevel: lvl,
                                autoFlush: af,
                                favorDecSpeed: 0,
                                reserved: [0; 3],
                            });
                        }
                    }
                }
            }
        }
    }
    // level / favorDecSpeed sweep
    for &lvl in FRAME_LEVELS.iter() {
        for &fav in [0u32, 1].iter() {
            for &bsid in [0i32, 7].iter() {
                v.push(LZ4F_preferences_t {
                    frameInfo: LZ4F_frameInfo_t {
                        blockSizeID: bsid,
                        blockMode: 1,
                        contentChecksumFlag: 1,
                        frameType: 0,
                        contentSize: 0,
                        dictID: 0xDEAD_BEEF,
                        blockChecksumFlag: 1,
                    },
                    compressionLevel: lvl,
                    autoFlush: 0,
                    favorDecSpeed: fav,
                    reserved: [0; 3],
                });
            }
        }
    }
    v
}
