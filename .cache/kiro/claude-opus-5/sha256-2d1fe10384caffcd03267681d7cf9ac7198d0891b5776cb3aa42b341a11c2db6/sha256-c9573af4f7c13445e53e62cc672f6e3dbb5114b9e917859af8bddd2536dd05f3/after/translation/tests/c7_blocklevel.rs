//! Phase C7: differential tests for the low-level "buffer-less" streaming API
//! and the raw block-level API — ERROR paths.
//!
//! Every error condition is constructed on both libraries and their error
//! codes compared via `Err2::eq`. All calls cross the FFI boundary through
//! `both::<T>(name)`.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------- FFI typedefs

type FnBeginLevel = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnBeginDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnBeginAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    size_t,
    ZSTD_parameters,
    c_ulonglong,
) -> size_t;
type FnBeginUsingCDict = unsafe extern "C" fn(*mut c_void, *const c_void) -> size_t;
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCopyCCtx = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulonglong) -> size_t;
type FnCctxToSize = unsafe extern "C" fn(*const c_void) -> size_t;
type FnBlock =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

type FnDecBegin = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnNextSrcSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnNextInputType = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;

type FnVoidPtr = unsafe extern "C" fn() -> *mut c_void;
type FnPtrSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

const ZSTD_BLOCKSIZE_MAX: usize = 1 << 17; // 131072

// ---------------------------------------------------------------- ctx pairs

struct CCtxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl CCtxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnVoidPtr>("ZSTD_createCCtx");
            let (x, y) = (a(), b());
            assert!(!x.is_null() && !y.is_null());
            CCtxPair { c: x, r: y }
        }
    }
}
impl Drop for CCtxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrSize>("ZSTD_freeCCtx");
            a(self.c);
            b(self.r);
        }
    }
}

struct DCtxPair {
    c: *mut c_void,
    r: *mut c_void,
}
impl DCtxPair {
    fn new() -> Self {
        unsafe {
            let (a, b) = both::<FnVoidPtr>("ZSTD_createDCtx");
            let (x, y) = (a(), b());
            assert!(!x.is_null() && !y.is_null());
            DCtxPair { c: x, r: y }
        }
    }
}
impl Drop for DCtxPair {
    fn drop(&mut self) {
        unsafe {
            let (a, b) = both::<FnPtrSize>("ZSTD_freeDCtx");
            a(self.c);
            b(self.r);
        }
    }
}

// ---------------------------------------------------------------- helpers

/// Build a valid frame with the C library via compress2 for corruption /
/// truncation tests.
unsafe fn make_frame(src: &[u8], checksum: c_int, content_size: c_int) -> Vec<u8> {
    let (create_c, _) = both::<FnVoidPtr>("ZSTD_createCCtx");
    let (free_c, _) = both::<FnPtrSize>("ZSTD_freeCCtx");
    let (setp_c, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c2_c, _) = both::<FnCompress2>("ZSTD_compress2");
    let (cbound_c, _) = both::<FnCompressBound>("ZSTD_compressBound");
    let (reset_c, _) = both::<FnReset>("ZSTD_CCtx_reset");
    let cctx = create_c();
    reset_c(cctx, ZSTD_reset_session_and_parameters);
    setp_c(cctx, ZSTD_c_compressionLevel, 3);
    setp_c(cctx, ZSTD_c_checksumFlag, checksum);
    setp_c(cctx, ZSTD_c_contentSizeFlag, content_size);
    let cap = cbound_c(src.len()) + 64;
    let mut out = vec![0u8; cap];
    let n = c2_c(
        cctx,
        out.as_mut_ptr() as *mut c_void,
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
    );
    free_c(cctx);
    assert!(!Err2::new().c.is_err(n));
    out.truncate(n);
    out
}

/// Run the full buffer-less decode loop on `frame` for BOTH libraries and
/// return a canonical outcome: either Ok(decoded bytes) or the first error
/// code encountered, paired with the step index at which it occurred. Used to
/// compare C vs Rust behaviour on corrupted/truncated input.
#[derive(Debug, PartialEq, Eq)]
enum DecodeOutcome {
    Ok(Vec<u8>),
    Err { code: c_int, step: usize },
}

unsafe fn decode_loop_one(
    lib_dctx: *mut c_void,
    is_c: bool,
    frame: &[u8],
    db: &libloading::Symbol<'static, FnDecBegin>,
    nsstd: &libloading::Symbol<'static, FnNextSrcSize>,
    dcont: &libloading::Symbol<'static, FnDecContinue>,
    e: &Err2,
) -> DecodeOutcome {
    let err = if is_c { &e.c } else { &e.r };
    let rc = (*db)(lib_dctx);
    if err.is_err(rc) {
        let code = classify_code(err, rc);
        return DecodeOutcome::Err { code, step: 0 };
    }
    let mut out = vec![0u8; frame.len() * 8 + 4096];
    let mut o = 0usize;
    let mut ic = 0usize;
    let mut step = 1usize;
    loop {
        let n = (*nsstd)(lib_dctx);
        // nextSrcSizeToDecompress itself can report an error via isError.
        if err.is_err(n) {
            return DecodeOutcome::Err { code: classify_code(err, n), step };
        }
        if n == 0 {
            break;
        }
        if ic + n > frame.len() {
            // Not enough input to satisfy the request: emulate srcSize_wrong by
            // stopping — but this should not happen for a full valid frame.
            return DecodeOutcome::Err { code: E_srcSize_wrong, step };
        }
        let inp = &frame[ic..ic + n];
        if o >= out.len() {
            out.resize(out.len() * 2, 0);
        }
        let rc = (*dcont)(
            lib_dctx,
            out.as_mut_ptr().add(o) as *mut c_void,
            out.len() - o,
            inp.as_ptr() as *const c_void,
            n,
        );
        if err.is_err(rc) {
            return DecodeOutcome::Err { code: classify_code(err, rc), step };
        }
        o += rc;
        ic += n;
        step += 1;
        if step > 5_000_000 {
            return DecodeOutcome::Err { code: E_GENERIC, step };
        }
    }
    out.truncate(o);
    DecodeOutcome::Ok(out)
}

fn classify_code(api: &ErrApi, r: size_t) -> c_int {
    match api.classify(r) {
        Ret::Err { code, .. } => code,
        Ret::Ok(_) => E_no_error,
    }
}

// -------------------------------------------------------------------- tests

/// compressContinue / compressEnd without a preceding compressBegin, on a
/// fresh CCtx and after a reset.
#[test]
fn continue_end_without_begin() {
    unsafe {
        let e = Err2::new();
        let (cont_c, cont_r) = both::<FnContinue>("ZSTD_compressContinue");
        let (end_c, end_r) = both::<FnContinue>("ZSTD_compressEnd");
        let (reset_c, reset_r) = both::<FnReset>("ZSTD_CCtx_reset");
        let mut rng = Rng::new(0xC7_0001);

        for trial in 0..64 {
            let src = gen(Shape::Text, 200 + rng.below(2000), &mut rng);
            let mut out_c = vec![0u8; 8192];
            let mut out_r = vec![0u8; 8192];

            let cp = CCtxPair::new();
            if trial % 2 == 1 {
                reset_c(cp.c, ZSTD_reset_session_and_parameters);
                reset_r(cp.r, ZSTD_reset_session_and_parameters);
            }
            let a = cont_c(cp.c, out_c.as_mut_ptr() as *mut c_void, out_c.len(),
                src.as_ptr() as *const c_void, src.len());
            let b = cont_r(cp.r, out_r.as_mut_ptr() as *mut c_void, out_r.len(),
                src.as_ptr() as *const c_void, src.len());
            e.eq(&format!("compressContinue without begin trial={trial}"), a, b);

            let cp2 = CCtxPair::new();
            if trial % 2 == 1 {
                reset_c(cp2.c, ZSTD_reset_session_and_parameters);
                reset_r(cp2.r, ZSTD_reset_session_and_parameters);
            }
            let a = end_c(cp2.c, out_c.as_mut_ptr() as *mut c_void, out_c.len(),
                src.as_ptr() as *const c_void, src.len());
            let b = end_r(cp2.r, out_r.as_mut_ptr() as *mut c_void, out_r.len(),
                src.as_ptr() as *const c_void, src.len());
            e.eq(&format!("compressEnd without begin trial={trial}"), a, b);
        }
    }
}

/// compressContinue with srcSize larger than the block size / BLOCKSIZE_MAX.
#[test]
fn continue_srcsize_too_large() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (cont_c, cont_r) = both::<FnContinue>("ZSTD_compressContinue");
        let (gbs_c, _) = both::<FnCctxToSize>("ZSTD_getBlockSize");
        let mut rng = Rng::new(0xC7_0002);

        for &big in &[131073usize, 200000, ZSTD_BLOCKSIZE_MAX + 1, ZSTD_BLOCKSIZE_MAX * 2] {
            let src = gen(Shape::Random, big, &mut rng);
            let cap = big + 4096;
            let mut out_c = vec![0u8; cap];
            let mut out_r = vec![0u8; cap];
            for &lvl in &[1i32, 3, 19] {
                let cp = CCtxPair::new();
                begin_c(cp.c, lvl);
                begin_r(cp.r, lvl);
                let bs = gbs_c(cp.c);
                assert!(big > bs, "block size {bs} should be < {big}");
                let a = cont_c(cp.c, out_c.as_mut_ptr() as *mut c_void, cap,
                    src.as_ptr() as *const c_void, big);
                let b = cont_r(cp.r, out_r.as_mut_ptr() as *mut c_void, cap,
                    src.as_ptr() as *const c_void, big);
                e.eq(&format!("compressContinue srcSize={big} > blockSize lvl={lvl}"), a, b);
            }
        }
    }
}

/// compressContinue / compressEnd with dstCapacity 0, 1, and one below the
/// exact needed size.
#[test]
fn continue_end_tight_dst() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (cont_c, cont_r) = both::<FnContinue>("ZSTD_compressContinue");
        let (end_c, end_r) = both::<FnContinue>("ZSTD_compressEnd");
        let (cbound_c, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xC7_0003);

        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating] {
            for &len in &[1usize, 100, 1024, 20000] {
                let src = gen(shape, len, &mut rng);
                let n = src.len();
                // Find exact needed size for the single continue with the C lib.
                let cap = cbound_c(n) + 1024;
                let cp0 = CCtxPair::new();
                begin_c(cp0.c, 3);
                let mut tmp = vec![0u8; cap];
                let need = cont_c(cp0.c, tmp.as_mut_ptr() as *mut c_void, cap,
                    src.as_ptr() as *const c_void, n);
                if e.c.is_err(need) {
                    continue;
                }
                for &dcap in &[0usize, 1, need.saturating_sub(1), need] {
                    let cp = CCtxPair::new();
                    begin_c(cp.c, 3);
                    begin_r(cp.r, 3);
                    let mut oc = vec![0u8; dcap.max(1)];
                    let mut orr = vec![0u8; dcap.max(1)];
                    let a = cont_c(cp.c, oc.as_mut_ptr() as *mut c_void, dcap,
                        src.as_ptr() as *const c_void, n);
                    let b = cont_r(cp.r, orr.as_mut_ptr() as *mut c_void, dcap,
                        src.as_ptr() as *const c_void, n);
                    e.eq(&format!("compressContinue tight dcap={dcap} shape={shape:?} len={len}"), a, b);
                }
                // compressEnd tight: begin then end directly (all in the final block)
                let cp1 = CCtxPair::new();
                begin_c(cp1.c, 3);
                let mut tmp2 = vec![0u8; cap];
                let need_end = end_c(cp1.c, tmp2.as_mut_ptr() as *mut c_void, cap,
                    src.as_ptr() as *const c_void, n);
                if e.c.is_err(need_end) {
                    continue;
                }
                for &dcap in &[0usize, 1, need_end.saturating_sub(1), need_end] {
                    let cp = CCtxPair::new();
                    begin_c(cp.c, 3);
                    begin_r(cp.r, 3);
                    let mut oc = vec![0u8; dcap.max(1)];
                    let mut orr = vec![0u8; dcap.max(1)];
                    let a = end_c(cp.c, oc.as_mut_ptr() as *mut c_void, dcap,
                        src.as_ptr() as *const c_void, n);
                    let b = end_r(cp.r, orr.as_mut_ptr() as *mut c_void, dcap,
                        src.as_ptr() as *const c_void, n);
                    e.eq(&format!("compressEnd tight dcap={dcap} shape={shape:?} len={len}"), a, b);
                }
            }
        }
    }
}

/// compressBegin / _advanced with invalid compression levels and invalid
/// ZSTD_parameters.
#[test]
fn begin_invalid_levels_and_params() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (ba_c, ba_r) = both::<FnBeginAdvanced>("ZSTD_compressBegin_advanced");
        let (gp_c, _) = both::<FnGetParams>("ZSTD_getParams");

        for &lvl in &[i32::MIN, -1_000_000, 23, 100, i32::MAX] {
            let cp = CCtxPair::new();
            e.eq(&format!("compressBegin invalid lvl={lvl}"),
                begin_c(cp.c, lvl), begin_r(cp.r, lvl));
        }

        // _advanced with invalid cParams: each field one step outside its bound
        // and u32::MAX.
        let base = gp_c(3, 1 << 16, 0);
        let bounds: [(usize, u32, u32); 7] = [
            (0, 10, 31),   // windowLog
            (1, 6, 30),    // chainLog
            (2, 6, 30),    // hashLog
            (3, 1, 30),    // searchLog
            (4, 3, 7),     // minMatch
            (5, 0, 131072),// targetLength
            (6, 1, 9),     // strategy
        ];
        for (idx, lo, hi) in bounds {
            for bad in [lo.wrapping_sub(1), hi + 1, u32::MAX] {
                let mut cp_params = base;
                match idx {
                    0 => cp_params.cParams.windowLog = bad,
                    1 => cp_params.cParams.chainLog = bad,
                    2 => cp_params.cParams.hashLog = bad,
                    3 => cp_params.cParams.searchLog = bad,
                    4 => cp_params.cParams.minMatch = bad,
                    5 => cp_params.cParams.targetLength = bad,
                    _ => cp_params.cParams.strategy = bad,
                }
                let cp = CCtxPair::new();
                e.eq(
                    &format!("compressBegin_advanced bad field={idx} val={bad}"),
                    ba_c(cp.c, std::ptr::null(), 0, cp_params, ZSTD_CONTENTSIZE_UNKNOWN),
                    ba_r(cp.r, std::ptr::null(), 0, cp_params, ZSTD_CONTENTSIZE_UNKNOWN),
                );
            }
        }
    }
}

/// compressBegin_usingDict / _usingCDict with degenerate dictionary args.
#[test]
fn begin_dict_null_conditions() {
    unsafe {
        let e = Err2::new();
        let (bd_c, bd_r) = both::<FnBeginDict>("ZSTD_compressBegin_usingDict");
        let (bc_c, bc_r) = both::<FnBeginUsingCDict>("ZSTD_compressBegin_usingCDict");

        // dictSize > 0 with dict == NULL
        for &dsize in &[1usize, 100, 4096] {
            let cp = CCtxPair::new();
            e.eq(
                &format!("compressBegin_usingDict null dict dsize={dsize}"),
                bd_c(cp.c, std::ptr::null(), dsize, 3),
                bd_r(cp.r, std::ptr::null(), dsize, 3),
            );
        }
        // NULL CDict (documented to fail)
        let cp = CCtxPair::new();
        e.eq(
            "compressBegin_usingCDict NULL cdict",
            bc_c(cp.c, std::ptr::null()),
            bc_r(cp.r, std::ptr::null()),
        );
    }
}

/// compressBlock with srcSize > BLOCKSIZE_MAX, dstCapacity too small, and
/// without a preceding compressBegin.
#[test]
fn compress_block_errors() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (cblk_c, cblk_r) = both::<FnBlock>("ZSTD_compressBlock");
        let mut rng = Rng::new(0xC7_0006);

        // srcSize > BLOCKSIZE_MAX
        for &big in &[ZSTD_BLOCKSIZE_MAX + 1, 200000] {
            let src = gen(Shape::Random, big, &mut rng);
            let cap = big + 4096;
            let mut oc = vec![0u8; cap];
            let mut orr = vec![0u8; cap];
            let cp = CCtxPair::new();
            begin_c(cp.c, 3);
            begin_r(cp.r, 3);
            e.eq(
                &format!("compressBlock srcSize={big} > MAX"),
                cblk_c(cp.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, big),
                cblk_r(cp.r, orr.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, big),
            );
        }

        // dstCapacity too small (random incompressible data forces raw block
        // which needs full size). Use tiny caps.
        for &len in &[100usize, 1024, 20000] {
            let src = gen(Shape::Random, len, &mut rng);
            for &dcap in &[0usize, 1, 2] {
                let cp = CCtxPair::new();
                begin_c(cp.c, 3);
                begin_r(cp.r, 3);
                let mut oc = vec![0u8; dcap.max(1)];
                let mut orr = vec![0u8; dcap.max(1)];
                e.eq(
                    &format!("compressBlock tight dcap={dcap} len={len}"),
                    cblk_c(cp.c, oc.as_mut_ptr() as *mut c_void, dcap, src.as_ptr() as *const c_void, len),
                    cblk_r(cp.r, orr.as_mut_ptr() as *mut c_void, dcap, src.as_ptr() as *const c_void, len),
                );
            }
        }

        // Without compressBegin (fresh cctx).
        for &len in &[1usize, 100, 1024] {
            let src = gen(Shape::Text, len, &mut rng);
            let cap = ZSTD_BLOCKSIZE_MAX;
            let mut oc = vec![0u8; cap];
            let mut orr = vec![0u8; cap];
            let cp = CCtxPair::new();
            e.eq(
                &format!("compressBlock without begin len={len}"),
                cblk_c(cp.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len),
                cblk_r(cp.r, orr.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, len),
            );
        }
    }
}

/// copyCCtx from a CCtx that is NOT freshly begun (after continue, after end).
#[test]
fn copy_cctx_wrong_stage() {
    unsafe {
        let e = Err2::new();
        let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
        let (cont_c, cont_r) = both::<FnContinue>("ZSTD_compressContinue");
        let (end_c, end_r) = both::<FnContinue>("ZSTD_compressEnd");
        let (copy_c, copy_r) = both::<FnCopyCCtx>("ZSTD_copyCCtx");
        let mut rng = Rng::new(0xC7_0007);

        for &len in &[100usize, 1024, 20000] {
            let src = gen(Shape::Text, len, &mut rng);
            let cap = len + 4096;

            // After compressContinue.
            let prep = CCtxPair::new();
            begin_c(prep.c, 3);
            begin_r(prep.r, 3);
            let mut oc = vec![0u8; cap];
            let mut orr = vec![0u8; cap];
            cont_c(prep.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            cont_r(prep.r, orr.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let dest = CCtxPair::new();
            e.eq(
                &format!("copyCCtx after continue len={len}"),
                copy_c(dest.c, prep.c, ZSTD_CONTENTSIZE_UNKNOWN),
                copy_r(dest.r, prep.r, ZSTD_CONTENTSIZE_UNKNOWN),
            );

            // After a full compressEnd.
            let prep2 = CCtxPair::new();
            begin_c(prep2.c, 3);
            begin_r(prep2.r, 3);
            let n1 = cont_c(prep2.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            let n1r = cont_r(prep2.r, orr.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len());
            end_c(prep2.c, oc.as_mut_ptr().add(n1) as *mut c_void, cap - n1, std::ptr::null(), 0);
            end_r(prep2.r, orr.as_mut_ptr().add(n1r) as *mut c_void, cap - n1r, std::ptr::null(), 0);
            let dest2 = CCtxPair::new();
            e.eq(
                &format!("copyCCtx after end len={len}"),
                copy_c(dest2.c, prep2.c, ZSTD_CONTENTSIZE_UNKNOWN),
                copy_r(dest2.r, prep2.r, ZSTD_CONTENTSIZE_UNKNOWN),
            );
        }
    }
}

/// decompressContinue with srcSize != nextSrcSizeToDecompress at several
/// points, and before decompressBegin.
#[test]
fn decompress_continue_wrong_srcsize() {
    unsafe {
        let e = Err2::new();
        let (db_c, db_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (nsstd_c, nsstd_r) = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let (dcont_c, dcont_r) = both::<FnDecContinue>("ZSTD_decompressContinue");
        let mut rng = Rng::new(0xC7_0008);

        // Before decompressBegin.
        {
            let dp = DCtxPair::new();
            let junk = gen(Shape::Random, 64, &mut rng);
            let mut oc = vec![0u8; 4096];
            let mut orr = vec![0u8; 4096];
            e.eq(
                "decompressContinue before begin",
                dcont_c(dp.c, oc.as_mut_ptr() as *mut c_void, oc.len(), junk.as_ptr() as *const c_void, 4),
                dcont_r(dp.r, orr.as_mut_ptr() as *mut c_void, orr.len(), junk.as_ptr() as *const c_void, 4),
            );
        }

        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating] {
            for &len in &[100usize, 1024, 20000] {
                let src = gen(shape, len, &mut rng);
                let frame = make_frame(&src, 1, 1);
                // Walk the frame several steps, at each point try a wrong srcSize.
                for &wrong_at in &[0usize, 1, 2] {
                    let dp = DCtxPair::new();
                    e.eq("wrongsz begin", db_c(dp.c), db_r(dp.r));
                    let mut oc = vec![0u8; src.len() * 4 + 4096];
                    let mut orr = vec![0u8; src.len() * 4 + 4096];
                    let mut ic = 0usize;
                    let mut oo = 0usize;
                    let mut step = 0usize;
                    loop {
                        let nc = nsstd_c(dp.c);
                        let nr = nsstd_r(dp.r);
                        assert_eq!(nc, nr, "nextSrcSize mismatch shape={shape:?} len={len} step={step}");
                        if nc == 0 {
                            break;
                        }
                        if step == wrong_at {
                            // Feed wrong srcSizes: n-1, n+1, 0, large.
                            for delta in [
                                nc.wrapping_sub(1),
                                nc + 1,
                                0,
                                nc + 100000,
                            ] {
                                let avail = frame.len() - ic;
                                let feed = delta.min(avail);
                                if feed == nc {
                                    continue; // not actually wrong
                                }
                                let inp = &frame[ic..ic + feed];
                                let a = dcont_c(dp.c, oc.as_mut_ptr().add(oo) as *mut c_void,
                                    oc.len() - oo, inp.as_ptr() as *const c_void, delta.min(avail));
                                let b = dcont_r(dp.r, orr.as_mut_ptr().add(oo) as *mut c_void,
                                    orr.len() - oo, inp.as_ptr() as *const c_void, delta.min(avail));
                                e.eq(
                                    &format!("decompressContinue wrong srcSize={} (want {nc}) shape={shape:?} len={len} step={step}", delta.min(avail)),
                                    a, b,
                                );
                            }
                            break;
                        }
                        // Feed the correct amount to advance.
                        let inp = &frame[ic..ic + nc];
                        let a = dcont_c(dp.c, oc.as_mut_ptr().add(oo) as *mut c_void,
                            oc.len() - oo, inp.as_ptr() as *const c_void, nc);
                        let b = dcont_r(dp.r, orr.as_mut_ptr().add(oo) as *mut c_void,
                            orr.len() - oo, inp.as_ptr() as *const c_void, nc);
                        e.eq(&format!("advance step={step}"), a, b);
                        if e.c.is_err(a) {
                            break;
                        }
                        oo += a;
                        ic += nc;
                        step += 1;
                    }
                }
            }
        }
    }
}

/// decompressContinue fed corrupted bytes: sweep every single-byte position of
/// a small frame, set to 0x00 / 0xFF and each bit flip, run the full decode
/// loop, and assert C and Rust agree (identical error code and step, or
/// identical success + output).
#[test]
fn decompress_continue_corruption_sweep() {
    unsafe {
        let e = Err2::new();
        let db = both::<FnDecBegin>("ZSTD_decompressBegin");
        let nsstd = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let dcont = both::<FnDecContinue>("ZSTD_decompressContinue");
        let mut rng = Rng::new(0xC7_0009);

        // Small frames keep the byte*mutation sweep well under budget.
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for &shape in &[Shape::Text, Shape::Repeating, Shape::Random] {
            for &len in &[8usize, 40, 120] {
                for &ck in &[0i32, 1] {
                    let src = gen(shape, len, &mut rng);
                    frames.push(make_frame(&src, ck, 1));
                }
            }
        }

        for (fi, base) in frames.iter().enumerate() {
            for pos in 0..base.len() {
                // mutants: 0x00, 0xFF, and 8 single-bit flips.
                let mut mutants: Vec<u8> = vec![0x00, 0xFF];
                for bit in 0..8 {
                    mutants.push(base[pos] ^ (1u8 << bit));
                }
                for m in mutants {
                    if m == base[pos] {
                        continue;
                    }
                    let mut frame = base.clone();
                    frame[pos] = m;
                    let dpc = DCtxPair::new();
                    let oc = decode_loop_one(dpc.c, true, &frame, &db.0, &nsstd.0, &dcont.0, &e);
                    let orr = decode_loop_one(dpc.r, false, &frame, &db.1, &nsstd.1, &dcont.1, &e);
                    assert_eq!(
                        oc, orr,
                        "corruption mismatch frame#{fi} pos={pos} byte={m:#04x}\n C={oc:?}\n RS={orr:?}"
                    );
                }
            }
            // Truncation sweep.
            for cut in 0..base.len() {
                let frame = &base[..cut];
                let dpc = DCtxPair::new();
                let oc = decode_loop_one(dpc.c, true, frame, &db.0, &nsstd.0, &dcont.0, &e);
                let orr = decode_loop_one(dpc.r, false, frame, &db.1, &nsstd.1, &dcont.1, &e);
                assert_eq!(oc, orr, "truncation mismatch frame#{fi} cut={cut}");
            }
        }
    }
}

/// decompressBlock error paths: srcSize > BLOCKSIZE_MAX, dstCapacity too small,
/// without decompressBegin, and 3000 random garbage buffers.
#[test]
fn decompress_block_errors() {
    unsafe {
        let e = Err2::new();
        let (dbegin_c, dbegin_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (dblk_c, dblk_r) = both::<FnBlock>("ZSTD_decompressBlock");
        let mut rng = Rng::new(0xC7_000A);

        // srcSize > BLOCKSIZE_MAX
        for &big in &[ZSTD_BLOCKSIZE_MAX + 1, 200000] {
            let src = gen(Shape::Random, big, &mut rng);
            let cap = big + 4096;
            let mut oc = vec![0u8; cap];
            let mut orr = vec![0u8; cap];
            let dp = DCtxPair::new();
            dbegin_c(dp.c);
            dbegin_r(dp.r);
            e.eq(
                &format!("decompressBlock srcSize={big} > MAX"),
                dblk_c(dp.c, oc.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, big),
                dblk_r(dp.r, orr.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, big),
            );
        }

        // dstCapacity too small (build a real compressed block first).
        {
            let (begin_c, begin_r) = both::<FnBeginLevel>("ZSTD_compressBegin");
            let (cblk_c, _) = both::<FnBlock>("ZSTD_compressBlock");
            let src = gen(Shape::Repeating, 4096, &mut rng);
            let cp = CCtxPair::new();
            begin_c(cp.c, 3);
            begin_r(cp.r, 3);
            let mut blk = vec![0u8; ZSTD_BLOCKSIZE_MAX];
            let bn = cblk_c(cp.c, blk.as_mut_ptr() as *mut c_void, blk.len(),
                src.as_ptr() as *const c_void, src.len());
            if !e.c.is_err(bn) && bn > 0 {
                for &dcap in &[0usize, 1, src.len() / 2] {
                    let dp = DCtxPair::new();
                    dbegin_c(dp.c);
                    dbegin_r(dp.r);
                    let mut oc = vec![0u8; dcap.max(1)];
                    let mut orr = vec![0u8; dcap.max(1)];
                    e.eq(
                        &format!("decompressBlock tight dcap={dcap}"),
                        dblk_c(dp.c, oc.as_mut_ptr() as *mut c_void, dcap, blk.as_ptr() as *const c_void, bn),
                        dblk_r(dp.r, orr.as_mut_ptr() as *mut c_void, dcap, blk.as_ptr() as *const c_void, bn),
                    );
                }
            }
        }

        // Without decompressBegin (fresh dctx).
        {
            let junk = gen(Shape::Random, 100, &mut rng);
            let mut oc = vec![0u8; 4096];
            let mut orr = vec![0u8; 4096];
            let dp = DCtxPair::new();
            e.eq(
                "decompressBlock without begin",
                dblk_c(dp.c, oc.as_mut_ptr() as *mut c_void, oc.len(), junk.as_ptr() as *const c_void, junk.len()),
                dblk_r(dp.r, orr.as_mut_ptr() as *mut c_void, orr.len(), junk.as_ptr() as *const c_void, junk.len()),
            );
        }

        // 3000 random garbage buffers.
        for i in 0..3000 {
            let len = 1 + rng.below(ZSTD_BLOCKSIZE_MAX.min(4096));
            let buf: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            let dp = DCtxPair::new();
            dbegin_c(dp.c);
            dbegin_r(dp.r);
            let cap = ZSTD_BLOCKSIZE_MAX + 16;
            let mut oc = vec![0u8; cap];
            let mut orr = vec![0u8; cap];
            let a = dblk_c(dp.c, oc.as_mut_ptr() as *mut c_void, cap, buf.as_ptr() as *const c_void, len);
            let b = dblk_r(dp.r, orr.as_mut_ptr() as *mut c_void, cap, buf.as_ptr() as *const c_void, len);
            e.eq(&format!("decompressBlock garbage #{i} len={len}"), a, b);
            if !e.c.is_err(a) {
                // If it "succeeded", decoded bytes must match too.
                assert_bytes_eq(&format!("garbage #{i} decoded"), &oc[..a], &orr[..b]);
            }
        }
    }
}

/// insertBlock with various block sizes.
#[test]
fn insert_block_sizes() {
    unsafe {
        let e = Err2::new();
        let (dbegin_c, dbegin_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (insert_c, insert_r) = both::<FnInsertBlock>("ZSTD_insertBlock");
        let mut rng = Rng::new(0xC7_000B);

        let buf = gen(Shape::Random, ZSTD_BLOCKSIZE_MAX, &mut rng);
        for &bs in &[0usize, 1, ZSTD_BLOCKSIZE_MAX, ZSTD_BLOCKSIZE_MAX + 1, 1 << 30] {
            let dp = DCtxPair::new();
            dbegin_c(dp.c);
            dbegin_r(dp.r);
            // Only pass a real buffer for sizes we actually allocated; for the
            // huge value use the buffer pointer with the huge length (the
            // function only records the size, it must not read it here for the
            // out-of-range case — but to be safe against a reading impl we cap
            // the pointer's backing to a large-but-safe value only for <= MAX).
            let (ptr, pass_bs) = if bs <= buf.len() {
                (buf.as_ptr() as *const c_void, bs)
            } else {
                // For over-max sizes, insertBlock should reject before reading.
                (buf.as_ptr() as *const c_void, bs)
            };
            let a = insert_c(dp.c, ptr, pass_bs);
            let b = insert_r(dp.r, ptr, pass_bs);
            e.eq(&format!("insertBlock bs={bs}"), a, b);
        }
    }
}

/// nextSrcSizeToDecompress / nextInputType on a fresh, mid-frame, and finished
/// DCtx.
#[test]
fn next_src_size_and_input_type_states() {
    unsafe {
        let e = Err2::new();
        let (db_c, db_r) = both::<FnDecBegin>("ZSTD_decompressBegin");
        let (nsstd_c, nsstd_r) = both::<FnNextSrcSize>("ZSTD_nextSrcSizeToDecompress");
        let (nit_c, nit_r) = both::<FnNextInputType>("ZSTD_nextInputType");
        let (dcont_c, dcont_r) = both::<FnDecContinue>("ZSTD_decompressContinue");
        let mut rng = Rng::new(0xC7_000C);

        // Fresh DCtx (before any begin).
        {
            let dp = DCtxPair::new();
            assert_eq!(nsstd_c(dp.c), nsstd_r(dp.r), "nextSrcSize fresh");
            assert_eq!(nit_c(dp.c), nit_r(dp.r), "nextInputType fresh");
        }

        let src = gen(Shape::Text, 40000, &mut rng);
        let frame = make_frame(&src, 1, 1);

        // Just after begin.
        {
            let dp = DCtxPair::new();
            db_c(dp.c);
            db_r(dp.r);
            assert_eq!(nsstd_c(dp.c), nsstd_r(dp.r), "nextSrcSize after begin");
            assert_eq!(nit_c(dp.c), nit_r(dp.r), "nextInputType after begin");
        }

        // Mid-frame: feed a couple of steps.
        {
            let dp = DCtxPair::new();
            db_c(dp.c);
            db_r(dp.r);
            let mut oc = vec![0u8; src.len() + 4096];
            let mut orr = vec![0u8; src.len() + 4096];
            let mut ic = 0usize;
            let mut oo = 0usize;
            for _ in 0..2 {
                let nc = nsstd_c(dp.c);
                let nr = nsstd_r(dp.r);
                assert_eq!(nc, nr);
                if nc == 0 { break; }
                let inp = &frame[ic..ic + nc];
                let a = dcont_c(dp.c, oc.as_mut_ptr().add(oo) as *mut c_void, oc.len() - oo,
                    inp.as_ptr() as *const c_void, nc);
                let b = dcont_r(dp.r, orr.as_mut_ptr().add(oo) as *mut c_void, orr.len() - oo,
                    inp.as_ptr() as *const c_void, nc);
                e.eq("mid-frame advance", a, b);
                oo += a; ic += nc;
                assert_eq!(nsstd_c(dp.c), nsstd_r(dp.r), "nextSrcSize mid-frame");
                assert_eq!(nit_c(dp.c), nit_r(dp.r), "nextInputType mid-frame");
            }
        }

        // Finished: run the full loop to completion, then query.
        {
            let dp = DCtxPair::new();
            db_c(dp.c);
            db_r(dp.r);
            let mut oc = vec![0u8; src.len() + 4096];
            let mut orr = vec![0u8; src.len() + 4096];
            let mut ic = 0usize;
            let mut oo = 0usize;
            loop {
                let nc = nsstd_c(dp.c);
                let nr = nsstd_r(dp.r);
                assert_eq!(nc, nr);
                if nc == 0 { break; }
                let inp = &frame[ic..ic + nc];
                let a = dcont_c(dp.c, oc.as_mut_ptr().add(oo) as *mut c_void, oc.len() - oo,
                    inp.as_ptr() as *const c_void, nc);
                let b = dcont_r(dp.r, orr.as_mut_ptr().add(oo) as *mut c_void, orr.len() - oo,
                    inp.as_ptr() as *const c_void, nc);
                e.eq("finish advance", a, b);
                oo += a; ic += nc;
            }
            assert_eq!(nsstd_c(dp.c), nsstd_r(dp.r), "nextSrcSize finished");
            assert_eq!(nit_c(dp.c), nit_r(dp.r), "nextInputType finished");
        }
    }
}
