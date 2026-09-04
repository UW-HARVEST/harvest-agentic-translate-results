//! Phase B — CONFIGS.md rows 65..81: streaming compression / decompression.
//!
//! Every entry point is resolved with `dlsym` in BOTH shared libraries and the
//! two implementations are driven in lock-step: after *every* call pair we
//! compare the return value, the full destination buffer, `ZSTD_inBuffer.pos`
//! and `ZSTD_outBuffer.pos`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------------ fn types

type FnStream2Simple = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *mut usize,
    *const c_void,
    usize,
    *mut usize,
    c_int,
) -> usize;

type FnDStreamSimple = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *mut usize,
    *const c_void,
    usize,
    *mut usize,
) -> usize;

type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void;
type FnLevel1 = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnInitCStreamSrcSize = unsafe extern "C" fn(*mut c_void, c_int, c_ulonglong) -> usize;
type FnInitCStreamUsingDict =
    unsafe extern "C" fn(*mut c_void, *const c_void, usize, c_int) -> usize;
type FnInitCStreamAdvanced = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    ZSTD_parameters,
    c_ulonglong,
) -> usize;
type FnPtrArg1 = unsafe extern "C" fn(*mut c_void, *const c_void) -> usize;
type FnInitCStreamUsingCDictAdv = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    ZSTD_frameParameters,
    c_ulonglong,
) -> usize;
type FnInitCStreamInternal = unsafe extern "C" fn(
    *mut c_void,
    *const c_void,
    usize,
    *const c_void,
    *const c_void,
    c_ulonglong,
) -> usize;
type FnU64Arg = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> usize;
type FnFlushStream = unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer) -> usize;
type FnCreateAdvanced = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnGetProgression = unsafe extern "C" fn(*const c_void) -> ZSTD_frameProgression;
type FnConstPtrToSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnConstPtrToInt = unsafe extern "C" fn(*const c_void) -> c_int;
type FnGetParamsFn = unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_parameters;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnSizeTArg = unsafe extern "C" fn(*mut c_void, usize) -> usize;
type FnIntArg = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
type FnGetFrameHeader =
    unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize) -> usize;
type FnGetFrameContentSize = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
type FnDecodingBufMin = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> usize;

// ------------------------------------------------------------------ misc helpers

const ALL_AT_ONCE: usize = usize::MAX;

/// A pair of library objects created by a non-trivial constructor.
struct PairPtr {
    c: *mut c_void,
    r: *mut c_void,
    free_c: FnFreePtr,
    free_r: FnFreePtr,
}

impl Drop for PairPtr {
    fn drop(&mut self) {
        unsafe {
            if !self.c.is_null() {
                (self.free_c)(self.c);
            }
            if !self.r.is_null() {
                (self.free_r)(self.r);
            }
        }
    }
}

unsafe fn cdict_pair(dict: &[u8], level: c_int) -> PairPtr {
    let (cc, cr) = duo::<FnCreateCDict>("ZSTD_createCDict");
    let (fc, fr) = duo::<FnFreePtr>("ZSTD_freeCDict");
    let c = cc(dict.as_ptr() as *const c_void, dict.len(), level);
    let r = cr(dict.as_ptr() as *const c_void, dict.len(), level);
    assert!(!c.is_null() && !r.is_null(), "createCDict returned NULL");
    PairPtr { c, r, free_c: fc, free_r: fr }
}

unsafe fn advanced_pair(create: &str, free: &str) -> PairPtr {
    let (cc, cr) = duo::<FnCreateAdvanced>(create);
    let (fc, fr) = duo::<FnFreePtr>(free);
    let cm = ZSTD_customMem::default();
    let c = cc(cm);
    let r = cr(cm);
    assert!(!c.is_null(), "{create} returned NULL in C");
    assert!(!r.is_null(), "{create} returned NULL in Rust");
    PairPtr { c, r, free_c: fc, free_r: fr }
}

unsafe fn cstream_in_size() -> usize {
    duo::<FnSizeT0>("ZSTD_CStreamInSize").0()
}
unsafe fn cstream_out_size() -> usize {
    duo::<FnSizeT0>("ZSTD_CStreamOutSize").0()
}
unsafe fn dstream_in_size() -> usize {
    duo::<FnSizeT0>("ZSTD_DStreamInSize").0()
}
unsafe fn dstream_out_size() -> usize {
    duo::<FnSizeT0>("ZSTD_DStreamOutSize").0()
}
unsafe fn compress_bound(n: usize) -> usize {
    duo::<FnSizeT1>("ZSTD_compressBound").0(n)
}

/// Build a reference frame with the **C** library only (ground truth input for
/// the decompression rows).
unsafe fn c_frame(src: &[u8], level: c_int, params: &[(c_int, c_int)]) -> Vec<u8> {
    let (create, _) = duo::<FnPtr0>("ZSTD_createCCtx");
    let (free, _) = duo::<FnFreePtr>("ZSTD_freeCCtx");
    let (setp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
    let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
    let cctx = create();
    assert!(!cctx.is_null());
    let n0 = setp(cctx, ZSTD_c_compressionLevel, level);
    assert!(!is_err(n0), "setParameter(level) failed");
    for &(p, v) in params {
        let n = setp(cctx, p, v);
        assert!(!is_err(n), "setParameter({p},{v}) failed");
    }
    let mut dst = vec![0u8; compress_bound(src.len()) + 64];
    let n = c2(
        cctx,
        dst.as_mut_ptr() as *mut c_void,
        dst.len(),
        src.as_ptr() as *const c_void,
        src.len(),
    );
    free(cctx);
    assert!(!is_err(n), "c_frame: compress2 failed");
    dst.truncate(n);
    dst
}

/// Decompress with the C library (used to validate round trips).
unsafe fn c_decompress(frame: &[u8], cap: usize) -> Vec<u8> {
    let (d, _) = duo::<FnDecompress>("ZSTD_decompress");
    let mut dst = vec![0u8; cap + 8];
    let n = d(
        dst.as_mut_ptr() as *mut c_void,
        dst.len(),
        frame.as_ptr() as *const c_void,
        frame.len(),
    );
    assert!(!is_err(n), "c_decompress failed (frame len {})", frame.len());
    dst.truncate(n);
    dst
}

/// Keep the amount of lock-step work per configuration bounded.
fn budget_size(in_chunk: usize, out_chunk: usize, want: usize) -> usize {
    let mut s = want;
    loop {
        let ic = if in_chunk == ALL_AT_ONCE { s.max(1) } else { in_chunk.max(1) };
        let iters = s / ic + s / out_chunk.max(1) + 4;
        let cost = iters.saturating_mul(out_chunk.min(1 << 21) + 32);
        if (cost <= 150_000_000 && iters <= 1_200_000) || s <= 64 {
            return s;
        }
        s /= 4;
    }
}

/// The output-chunk ladder {1, 3, 17, 1KB, `ZSTD_CStreamOutSize()`, oversized}
/// paired with an affordable input size for that combination.
unsafe fn pick_out_c(ic: usize, oi: usize, want: usize, cout: usize) -> (usize, usize) {
    let fixed = [1usize, 3, 17, 1024];
    if oi < 4 {
        let oc = fixed[oi];
        return (budget_size(ic, oc, want), oc);
    }
    if oi == 4 {
        return (budget_size(ic, cout, want), cout);
    }
    // "oversized": one buffer able to hold the whole frame
    let mut size = budget_size(ic, 4096, want);
    for _ in 0..4 {
        let oc = compress_bound(size) + 64;
        size = budget_size(ic, oc, size);
    }
    (size, compress_bound(size) + 64)
}

/// Same, for the decompression side (`ZSTD_DStreamOutSize()` + oversized).
fn pick_out_d(ic: usize, oi: usize, want: usize, dout: usize) -> (usize, usize) {
    let fixed = [1usize, 3, 17, 1024];
    if oi < 4 {
        let oc = fixed[oi];
        return (budget_size(ic, oc, want), oc);
    }
    if oi == 4 {
        return (budget_size(ic, dout, want), dout);
    }
    let mut size = budget_size(ic, 4096, want);
    for _ in 0..4 {
        let oc = size + 4096;
        size = budget_size(ic, oc, size);
    }
    (size, size + 4096)
}

// ------------------------------------------------------------------ sanity

/// Guard against `dlopen` aliasing the two objects (SONAME collision): every
/// symbol used below must resolve to a *different* address in each library,
/// otherwise the whole file would silently compare the C build with itself.
#[test]
fn sanity_two_distinct_libraries() {
    unsafe {
        for name in [
            "ZSTD_compressStream2",
            "ZSTD_compressStream2_simpleArgs",
            "ZSTD_decompressStream",
            "ZSTD_decompressStream_simpleArgs",
            "ZSTD_initCStream",
            "ZSTD_initCStream_srcSize",
            "ZSTD_initCStream_usingDict",
            "ZSTD_initCStream_advanced",
            "ZSTD_initCStream_usingCDict",
            "ZSTD_initCStream_usingCDict_advanced",
            "ZSTD_initCStream_internal",
            "ZSTD_compressStream",
            "ZSTD_flushStream",
            "ZSTD_endStream",
            "ZSTD_resetCStream",
            "ZSTD_initDStream",
            "ZSTD_resetDStream",
            "ZSTD_DCtx_reset",
            "ZSTD_nextSrcSizeToDecompress",
            "ZSTD_nextInputType",
            "ZSTD_DCtx_setMaxWindowSize",
            "ZSTD_DCtx_setFormat",
            "ZSTD_createCStream_advanced",
            "ZSTD_createCCtx_advanced",
            "ZSTD_createDStream_advanced",
            "ZSTD_createDCtx_advanced",
            "ZSTD_getFrameProgression",
            "ZSTD_toFlushNow",
        ] {
            let (c, r) = duo_addr::<c_void>(name);
            assert!(!c.is_null() && !r.is_null(), "{name}: null symbol");
            assert_ne!(
                c as usize, r as usize,
                "{name}: the C and Rust libraries resolved to the SAME address — \
                 dlopen aliased the two objects, the differential test would be vacuous"
            );
        }
    }
}

// ------------------------------------------------------------------ drivers

/// Lock-step `ZSTD_compressStream2` (or `_simpleArgs`) driver.
///
/// `script` supplies the `ZSTD_EndDirective` for each fresh input chunk; once
/// the whole input has been consumed the driver switches to `ZSTD_e_end` and
/// loops until the call returns 0.
#[allow(clippy::too_many_arguments)]
unsafe fn drive_cstream2(
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
    label: &str,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    script: &[c_int],
    simple: bool,
    probe: bool,
) -> Result<Vec<u8>, usize> {
    assert!(out_chunk >= 1);
    let (f2c, f2r) = duo::<FnStream2>("ZSTD_compressStream2");
    let (fsc, fsr) = duo::<FnStream2Simple>("ZSTD_compressStream2_simpleArgs");
    let (pgc, pgr) = duo::<FnGetProgression>("ZSTD_getFrameProgression");
    let (tfc, tfr) = duo::<FnConstPtrToSize>("ZSTD_toFlushNow");

    let mut out_c = vec![0xA5u8; out_chunk];
    let mut out_r = vec![0xA5u8; out_chunk];
    let mut acc_c: Vec<u8> = Vec::new();
    let mut acc_r: Vec<u8> = Vec::new();
    let mut spos = 0usize;
    let mut step = 0usize;
    let mut pending: Option<c_int> = None;
    let mut iters = 0usize;
    let mut stall = 0usize;

    loop {
        iters += 1;
        assert!(iters < 2_000_000, "{label}: runaway loop");
        let remaining = src.len() - spos;
        let take = if in_chunk == ALL_AT_ONCE { remaining } else { in_chunk.min(remaining) };
        let op = match pending {
            Some(o) => o,
            None => {
                if remaining == 0 {
                    ZSTD_e_end
                } else {
                    let o = script[step % script.len()];
                    step += 1;
                    o
                }
            }
        };
        out_c.iter_mut().for_each(|b| *b = 0xA5);
        out_r.iter_mut().for_each(|b| *b = 0xA5);
        let sp = src.as_ptr().add(spos) as *const c_void;

        let (retc, retr, cin, rin, cout, rout);
        if simple {
            let mut dp_c = 0usize;
            let mut sp_c = 0usize;
            let mut dp_r = 0usize;
            let mut sp_r = 0usize;
            retc = fsc(
                cctx_c,
                out_c.as_mut_ptr() as *mut c_void,
                out_chunk,
                &mut dp_c,
                sp,
                take,
                &mut sp_c,
                op,
            );
            retr = fsr(
                cctx_r,
                out_r.as_mut_ptr() as *mut c_void,
                out_chunk,
                &mut dp_r,
                sp,
                take,
                &mut sp_r,
                op,
            );
            cin = sp_c;
            rin = sp_r;
            cout = dp_c;
            rout = dp_r;
        } else {
            let mut ib_c = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
            let mut ib_r = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
            let mut ob_c =
                ZSTD_outBuffer { dst: out_c.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            let mut ob_r =
                ZSTD_outBuffer { dst: out_r.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            retc = f2c(cctx_c, &mut ob_c, &mut ib_c, op);
            retr = f2r(cctx_r, &mut ob_r, &mut ib_r, op);
            assert_eq!(ib_c.size, take, "{label}: C mutated in.size");
            assert_eq!(ib_r.size, take, "{label}: Rust mutated in.size");
            assert_eq!(ob_c.size, out_chunk, "{label}: C mutated out.size");
            assert_eq!(ob_r.size, out_chunk, "{label}: Rust mutated out.size");
            cin = ib_c.pos;
            rin = ib_r.pos;
            cout = ob_c.pos;
            rout = ob_r.pos;
        }

        // Fast path: only build the (allocating) label when something differs.
        let bad = retc != retr || cin != rin || cout != rout || out_c != out_r;
        if bad || probe {
            let tag = format!("{label} step{iters} op{op}");
            eqv(&format!("{tag} ret"), retc, retr);
            eqv(&format!("{tag} in.pos"), cin, rin);
            eqv(&format!("{tag} out.pos"), cout, rout);
            eqbuf(&format!("{tag} dst"), &out_c, &out_r);
            if probe {
                eqv(&format!("{tag} frameProgression"), pgc(cctx_c), pgr(cctx_r));
                eqv(&format!("{tag} toFlushNow"), tfc(cctx_c), tfr(cctx_r));
            }
            assert!(!bad, "{tag}: mismatch not localized");
        }
        if is_err(retc) {
            return Err(retc);
        }
        assert!(cout <= out_chunk && cin <= take, "{label}: position out of range");
        acc_c.extend_from_slice(&out_c[..cout]);
        acc_r.extend_from_slice(&out_r[..rout]);
        spos += cin;

        if cin > 0 || cout > 0 {
            stall = 0;
        } else {
            stall += 1;
            assert!(stall < 48, "{label}: no forward progress");
        }

        if op == ZSTD_e_continue {
            pending = None;
        } else if retc == 0 {
            pending = None;
            if op == ZSTD_e_end && spos >= src.len() {
                break;
            }
        } else {
            pending = Some(op);
        }
    }
    eqbuf(&format!("{label} whole compressed stream"), &acc_c, &acc_r);
    Ok(acc_c)
}

/// Lock-step legacy streaming compression: `ZSTD_compressStream` for the body,
/// then `ZSTD_flushStream` / `ZSTD_endStream`.
unsafe fn drive_legacy_cstream(
    zcs_c: *mut c_void,
    zcs_r: *mut c_void,
    label: &str,
    src: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    flush_every: bool,
) -> Result<Vec<u8>, usize> {
    let (fcc, fcr) = duo::<FnDStream>("ZSTD_compressStream");
    let (flc, flr) = duo::<FnFlushStream>("ZSTD_flushStream");
    let (fec, fer) = duo::<FnFlushStream>("ZSTD_endStream");

    let mut out_c = vec![0xA5u8; out_chunk];
    let mut out_r = vec![0xA5u8; out_chunk];
    let mut acc_c: Vec<u8> = Vec::new();
    let mut acc_r: Vec<u8> = Vec::new();
    let mut spos = 0usize;
    let mut iters = 0usize;

    // 0 = compressStream body, 1 = flushStream, 2 = endStream
    let mut phase = 0u8;
    loop {
        iters += 1;
        assert!(iters < 2_000_000, "{label}: runaway loop");
        out_c.iter_mut().for_each(|b| *b = 0xA5);
        out_r.iter_mut().for_each(|b| *b = 0xA5);
        let mut ob_c =
            ZSTD_outBuffer { dst: out_c.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
        let mut ob_r =
            ZSTD_outBuffer { dst: out_r.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };

        let (retc, retr, cin, rin);
        match phase {
            0 => {
                let remaining = src.len() - spos;
                let take =
                    if in_chunk == ALL_AT_ONCE { remaining } else { in_chunk.min(remaining) };
                let sp = src.as_ptr().add(spos) as *const c_void;
                let mut ib_c = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
                let mut ib_r = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
                retc = fcc(zcs_c, &mut ob_c, &mut ib_c);
                retr = fcr(zcs_r, &mut ob_r, &mut ib_r);
                cin = ib_c.pos;
                rin = ib_r.pos;
            }
            1 => {
                retc = flc(zcs_c, &mut ob_c);
                retr = flr(zcs_r, &mut ob_r);
                cin = 0;
                rin = 0;
            }
            _ => {
                retc = fec(zcs_c, &mut ob_c);
                retr = fer(zcs_r, &mut ob_r);
                cin = 0;
                rin = 0;
            }
        }
        if retc != retr || cin != rin || ob_c.pos != ob_r.pos || out_c != out_r {
            let tag = format!("{label} step{iters} phase{phase}");
            eqv(&format!("{tag} ret"), retc, retr);
            eqv(&format!("{tag} in.pos"), cin, rin);
            eqv(&format!("{tag} out.pos"), ob_c.pos, ob_r.pos);
            eqbuf(&format!("{tag} dst"), &out_c, &out_r);
            unreachable!("{tag}: mismatch not localized");
        }
        if is_err(retc) {
            return Err(retc);
        }
        acc_c.extend_from_slice(&out_c[..ob_c.pos]);
        acc_r.extend_from_slice(&out_r[..ob_r.pos]);
        spos += cin;

        match phase {
            0 => {
                if spos >= src.len() {
                    phase = if flush_every { 1 } else { 2 };
                } else if flush_every && cin > 0 {
                    phase = 1;
                }
            }
            1 => {
                if retc == 0 {
                    phase = if spos >= src.len() { 2 } else { 0 };
                }
            }
            _ => {
                if retc == 0 {
                    break;
                }
            }
        }
    }
    eqbuf(&format!("{label} whole compressed stream"), &acc_c, &acc_r);
    Ok(acc_c)
}

/// Lock-step `ZSTD_decompressStream` (or `_simpleArgs`) driver.
#[allow(clippy::too_many_arguments)]
unsafe fn drive_dstream(
    dctx_c: *mut c_void,
    dctx_r: *mut c_void,
    label: &str,
    frame: &[u8],
    in_chunk: usize,
    out_chunk: usize,
    simple: bool,
) -> Result<Vec<u8>, usize> {
    assert!(out_chunk >= 1);
    let (fdc, fdr) = duo::<FnDStream>("ZSTD_decompressStream");
    let (fsc, fsr) = duo::<FnDStreamSimple>("ZSTD_decompressStream_simpleArgs");

    let mut out_c = vec![0xA5u8; out_chunk];
    let mut out_r = vec![0xA5u8; out_chunk];
    let mut acc_c: Vec<u8> = Vec::new();
    let mut acc_r: Vec<u8> = Vec::new();
    let mut ipos = 0usize;
    let mut last = 1usize;
    let mut iters = 0usize;
    let mut stall = 0usize;

    loop {
        if last == 0 && ipos >= frame.len() {
            break;
        }
        iters += 1;
        assert!(iters < 2_000_000, "{label}: runaway loop");
        let remaining = frame.len() - ipos;
        let take = if in_chunk == ALL_AT_ONCE { remaining } else { in_chunk.min(remaining) };
        out_c.iter_mut().for_each(|b| *b = 0xA5);
        out_r.iter_mut().for_each(|b| *b = 0xA5);
        let sp = frame.as_ptr().add(ipos) as *const c_void;

        let (retc, retr, cin, rin, cout, rout);
        if simple {
            let mut dp_c = 0usize;
            let mut sp_c = 0usize;
            let mut dp_r = 0usize;
            let mut sp_r = 0usize;
            retc = fsc(
                dctx_c,
                out_c.as_mut_ptr() as *mut c_void,
                out_chunk,
                &mut dp_c,
                sp,
                take,
                &mut sp_c,
            );
            retr = fsr(
                dctx_r,
                out_r.as_mut_ptr() as *mut c_void,
                out_chunk,
                &mut dp_r,
                sp,
                take,
                &mut sp_r,
            );
            cin = sp_c;
            rin = sp_r;
            cout = dp_c;
            rout = dp_r;
        } else {
            let mut ib_c = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
            let mut ib_r = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
            let mut ob_c =
                ZSTD_outBuffer { dst: out_c.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            let mut ob_r =
                ZSTD_outBuffer { dst: out_r.as_mut_ptr() as *mut c_void, size: out_chunk, pos: 0 };
            retc = fdc(dctx_c, &mut ob_c, &mut ib_c);
            retr = fdr(dctx_r, &mut ob_r, &mut ib_r);
            cin = ib_c.pos;
            rin = ib_r.pos;
            cout = ob_c.pos;
            rout = ob_r.pos;
        }

        if retc != retr || cin != rin || cout != rout || out_c != out_r {
            let tag = format!("{label} step{iters}");
            eqv(&format!("{tag} ret"), retc, retr);
            eqv(&format!("{tag} in.pos"), cin, rin);
            eqv(&format!("{tag} out.pos"), cout, rout);
            eqbuf(&format!("{tag} dst"), &out_c, &out_r);
            unreachable!("{tag}: mismatch not localized");
        }
        if is_err(retc) {
            return Err(retc);
        }
        acc_c.extend_from_slice(&out_c[..cout]);
        acc_r.extend_from_slice(&out_r[..rout]);
        ipos += cin;
        last = retc;
        if cin > 0 || cout > 0 {
            stall = 0;
        } else {
            stall += 1;
            assert!(stall < 48, "{label}: no forward progress");
        }
    }
    eqbuf(&format!("{label} whole decompressed stream"), &acc_c, &acc_r);
    Ok(acc_c)
}

// ================================================================== row 65

#[test]
fn row65_compressStream2_continue_grid() {
    unsafe {
        let cin = cstream_in_size();
        let cout = cstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, cin, ALL_AT_ONCE];
        let mut rng = Rng::new(0x6500_0065);
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let sizes = [
            0usize,
            1024,
            8192,
            128 * 1024 - 1,
            128 * 1024,
            128 * 1024 + 1,
            256 * 1024,
            1024 * 1024,
        ];
        let mut combos = 0usize;
        for (ii, &ic) in in_grid.iter().enumerate() {
            for oi in 0..6usize {
                for (si, &want) in sizes.iter().enumerate() {
                    let (size, oc) = pick_out_c(ic, oi, want, cout);
                    let class = (ii * 6 + oi + si) % N_CLASSES;
                    let level = [1, 3, 6, 9][(ii + oi + si) % 4];
                    let src = gen_class(class, size, rng.next_u64());
                    let ctx = CtxPair::cctx();
                    eqv(
                        "setParameter(level)",
                        setp(ctx.c, ZSTD_c_compressionLevel, level),
                        setpr(ctx.r, ZSTD_c_compressionLevel, level),
                    );
                    let label = format!(
                        "row65 in={ic} out={oc} size={size} class={} lvl={level}",
                        CLASS_NAMES[class]
                    );
                    let got = drive_cstream2(
                        ctx.c,
                        ctx.r,
                        &label,
                        &src,
                        ic,
                        oc,
                        &[ZSTD_e_continue],
                        false,
                        false,
                    )
                    .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                    combos += 1;
                }
            }
        }
        assert_eq!(combos, 36 * sizes.len());
    }
}

// ================================================================== row 66

#[test]
fn row66_compressStream2_flush_every_chunk() {
    unsafe {
        let cin = cstream_in_size();
        let cout = cstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, cin, ALL_AT_ONCE];
        let mut rng = Rng::new(0x6600_0066);
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let sizes = [0usize, 1, 1024, 8192, 64 * 1024, 128 * 1024 + 1, 200_000];
        for (ii, &ic) in in_grid.iter().enumerate() {
            for oi in 0..6usize {
                for (si, &want) in sizes.iter().enumerate() {
                    let (size, oc) = pick_out_c(ic, oi, want, cout);
                    let class = (ii + oi * 3 + si) % N_CLASSES;
                    let level = [1, 4, 9][(ii + oi + si) % 3];
                    let src = gen_class(class, size, rng.next_u64());
                    let ctx = CtxPair::cctx();
                    eqv(
                        "setParameter(level)",
                        setp(ctx.c, ZSTD_c_compressionLevel, level),
                        setpr(ctx.r, ZSTD_c_compressionLevel, level),
                    );
                    let label = format!(
                        "row66 in={ic} out={oc} size={size} class={} lvl={level}",
                        CLASS_NAMES[class]
                    );
                    let got = drive_cstream2(
                        ctx.c,
                        ctx.r,
                        &label,
                        &src,
                        ic,
                        oc,
                        &[ZSTD_e_flush],
                        false,
                        false,
                    )
                    .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    // a frame built out of many flushes must still decode
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
    }
}

// ================================================================== row 67

#[test]
fn row67_compressStream2_random_endop_scripts() {
    unsafe {
        let mut rng = Rng::new(0x6700_0067);
        let cin = cstream_in_size();
        let cout = cstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, cin, ALL_AT_ONCE];
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        for script_no in 0..200usize {
            let ic = in_grid[rng.below(in_grid.len())];
            let oi = rng.below(6);
            let want =
                [0usize, 64, 1024, 5000, 20_000, 128 * 1024 + 1, 200_000][rng.below(7)];
            let (size, oc) = pick_out_c(ic, oi, want, cout);
            let class = rng.below(N_CLASSES);
            let level = rng.range(-2, 12);
            let src = gen_class(class, size, rng.next_u64());
            let slen = 1 + rng.below(5);
            let mut script = Vec::with_capacity(slen);
            for _ in 0..slen {
                script.push([ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end][rng.below(3)]);
            }
            let ctx = CtxPair::cctx();
            eqv(
                "setParameter(level)",
                setp(ctx.c, ZSTD_c_compressionLevel, level),
                setpr(ctx.r, ZSTD_c_compressionLevel, level),
            );
            // a randomized slice of the sticky-parameter surface, so the
            // streaming buffer logic is exercised against several code paths
            let extra: [(c_int, c_int); 6] = [
                (ZSTD_c_checksumFlag, (script_no % 2) as c_int),
                (ZSTD_c_contentSizeFlag, ((script_no / 2) % 2) as c_int),
                (ZSTD_c_dictIDFlag, ((script_no / 4) % 2) as c_int),
                (ZSTD_c_enableLongDistanceMatching, rng.range(0, 2)),
                (ZSTD_c_strategy, rng.range(0, 9)),
                (ZSTD_c_targetCBlockSize, [0, 1340, 8192, 131072][rng.below(4)]),
            ];
            for &(p, v) in &extra {
                eqv(
                    &format!("setParameter({p},{v})"),
                    setp(ctx.c, p, v),
                    setpr(ctx.r, p, v),
                );
            }
            let label = format!(
                "row67 #{script_no} in={ic} out={oc} size={size} lvl={level} extra={extra:?} script={script:?}"
            );
            let got = drive_cstream2(ctx.c, ctx.r, &label, &src, ic, oc, &script, false, false)
                .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
            // The output is one or more concatenated frames; it must round trip.
            let back = c_decompress(&got, src.len());
            eqbuf(&format!("{label} round trip"), &src, &back);
        }
    }
}

// ================================================================== row 68

#[test]
fn row68_compressStream2_simpleArgs_grid() {
    unsafe {
        let cin = cstream_in_size();
        let cout = cstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, cin, ALL_AT_ONCE];
        let scripts: [&[c_int]; 3] =
            [&[ZSTD_e_continue], &[ZSTD_e_flush], &[ZSTD_e_continue, ZSTD_e_flush, ZSTD_e_end]];
        let mut rng = Rng::new(0x6800_0068);
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let sizes = [0usize, 1, 1024, 8192, 70_000, 128 * 1024 + 1, 256 * 1024];
        for (ii, &ic) in in_grid.iter().enumerate() {
            for oi in 0..6usize {
                for (si, &want) in sizes.iter().enumerate() {
                    let (size, oc) = pick_out_c(ic, oi, want, cout);
                    let class = (ii * 5 + oi + si) % N_CLASSES;
                    let level = [2, 5, 8][(ii + 2 * oi + si) % 3];
                    let script = scripts[(ii + oi + si) % 3];
                    let src = gen_class(class, size, rng.next_u64());
                    let ctx = CtxPair::cctx();
                    eqv(
                        "setParameter(level)",
                        setp(ctx.c, ZSTD_c_compressionLevel, level),
                        setpr(ctx.r, ZSTD_c_compressionLevel, level),
                    );
                    let label = format!("row68 in={ic} out={oc} size={size} script={script:?}");
                    let got =
                        drive_cstream2(ctx.c, ctx.r, &label, &src, ic, oc, script, true, false)
                            .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
    }
}

// ================================================================== row 69

/// Lock-step driver for the `stableInBuffer` / `stableOutBuffer` modes: the
/// input buffer (and/or the output buffer) is presented once and only its
/// `pos` field moves.
unsafe fn drive_stable(
    cctx_c: *mut c_void,
    cctx_r: *mut c_void,
    label: &str,
    src: &[u8],
    stable_in: bool,
    stable_out: bool,
    in_chunk: usize,
    out_cap: usize,
    script: &[c_int],
) -> Result<Vec<u8>, usize> {
    let (f2c, f2r) = duo::<FnStream2>("ZSTD_compressStream2");
    let mut big_c = vec![0xA5u8; out_cap];
    let mut big_r = vec![0xA5u8; out_cap];
    let base_c = big_c.as_mut_ptr();
    let base_r = big_r.as_mut_ptr();

    let mut abs_in = 0usize;
    let mut abs_out = 0usize;
    let mut step = 0usize;
    let mut iters = 0usize;
    let mut pending: Option<c_int> = None;
    let mut stall = 0usize;
    loop {
        iters += 1;
        assert!(iters < 1_000_000, "{label}: runaway loop");
        let op = match pending {
            Some(o) => o,
            None => {
                if abs_in >= src.len() {
                    ZSTD_e_end
                } else {
                    let o = script[step % script.len()];
                    step += 1;
                    o
                }
            }
        };
        // A "stable" buffer is presented in full, every call, with only `pos`
        // moving (exactly what ZSTD_checkBufferStability expects).
        let (isrc, isize_, ipos) = if stable_in {
            (src.as_ptr() as *const c_void, src.len(), abs_in)
        } else {
            let take = in_chunk.min(src.len() - abs_in);
            (src.as_ptr().add(abs_in) as *const c_void, take, 0usize)
        };
        let (odst_c, odst_r, osize, opos) = if stable_out {
            (base_c as *mut c_void, base_r as *mut c_void, out_cap, abs_out)
        } else {
            let cap = in_chunk.max(7).min(out_cap - abs_out);
            assert!(cap > 0, "{label}: output capacity exhausted");
            (
                base_c.add(abs_out) as *mut c_void,
                base_r.add(abs_out) as *mut c_void,
                cap,
                0usize,
            )
        };
        let mut ib_c = ZSTD_inBuffer { src: isrc, size: isize_, pos: ipos };
        let mut ib_r = ZSTD_inBuffer { src: isrc, size: isize_, pos: ipos };
        let mut ob_c = ZSTD_outBuffer { dst: odst_c, size: osize, pos: opos };
        let mut ob_r = ZSTD_outBuffer { dst: odst_r, size: osize, pos: opos };

        let retc = f2c(cctx_c, &mut ob_c, &mut ib_c, op);
        let retr = f2r(cctx_r, &mut ob_r, &mut ib_r, op);
        if retc != retr || ib_c.pos != ib_r.pos || ob_c.pos != ob_r.pos || big_c != big_r {
            let tag = format!("{label} step{iters} op{op}");
            eqv(&format!("{tag} ret"), retc, retr);
            eqv(&format!("{tag} in.pos"), ib_c.pos, ib_r.pos);
            eqv(&format!("{tag} out.pos"), ob_c.pos, ob_r.pos);
            eqbuf(&format!("{tag} dst"), &big_c, &big_r);
            unreachable!("{tag}: mismatch not localized");
        }
        if is_err(retc) {
            return Err(retc);
        }
        let consumed = ib_c.pos - ipos;
        let produced = ob_c.pos - opos;
        abs_in += consumed;
        abs_out += produced;
        if consumed > 0 || produced > 0 {
            stall = 0;
        } else {
            stall += 1;
            assert!(stall < 48, "{label}: no forward progress");
        }

        if op == ZSTD_e_continue {
            pending = None;
        } else if retc == 0 {
            pending = None;
            if op == ZSTD_e_end && abs_in >= src.len() {
                break;
            }
        } else {
            pending = Some(op);
        }
    }
    eqbuf(&format!("{label} whole compressed stream"), &big_c[..abs_out], &big_r[..abs_out]);
    Ok(big_c[..abs_out].to_vec())
}

#[test]
fn row69_stable_in_out_buffer() {
    unsafe {
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let mut rng = Rng::new(0x6900_0069);
        let sizes = [0usize, 1, 7, 1024, 8192, 70_000];
        for &(sin, sout) in &[(1, 0), (0, 1), (1, 1)] {
            for (si, &size) in sizes.iter().enumerate() {
                for class in 0..N_CLASSES {
                    let src = gen_class(class, size, rng.next_u64());
                    let level = [1, 3, 7][(si + class) % 3] as c_int;
                    let ctx = CtxPair::cctx();
                    for &(p, v) in &[
                        (ZSTD_c_compressionLevel, level),
                        (ZSTD_c_stableInBuffer, sin),
                        (ZSTD_c_stableOutBuffer, sout),
                    ] {
                        eqv(
                            &format!("row69 setParameter({p},{v})"),
                            setp(ctx.c, p, v),
                            setpr(ctx.r, p, v),
                        );
                    }
                    let cap = compress_bound(size) + 64;
                    let script: &[c_int] = match (si + class) % 3 {
                        0 => &[ZSTD_e_continue],
                        1 => &[ZSTD_e_flush],
                        _ => &[ZSTD_e_continue, ZSTD_e_flush],
                    };
                    let label = format!(
                        "row69 stableIn={sin} stableOut={sout} size={size} class={}",
                        CLASS_NAMES[class]
                    );
                    let got = drive_stable(
                        ctx.c,
                        ctx.r,
                        &label,
                        &src,
                        sin == 1,
                        sout == 1,
                        1 + rng.below(4096),
                        cap,
                        script,
                    )
                    .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
        // stableInBuffer where `size` GROWS between calls (the documented
        // "append more data after compression started" path).
        {
            let (f2c, f2r) = duo::<FnStream2>("ZSTD_compressStream2");
            for &total in &[1024usize, 40_000, 140_000] {
                for class in [0usize, 3, 4, 5] {
                    let src = gen_class(class, total, rng.next_u64());
                    let ctx = CtxPair::cctx();
                    for &(p, v) in
                        &[(ZSTD_c_compressionLevel, 5), (ZSTD_c_stableInBuffer, 1)]
                    {
                        eqv("row69c setParameter", setp(ctx.c, p, v), setpr(ctx.r, p, v));
                    }
                    let cap = compress_bound(total) + 64;
                    let mut oc = vec![0xA5u8; cap];
                    let mut or_ = vec![0xA5u8; cap];
                    let label = format!("row69 growing-stableIn total={total} class={}", CLASS_NAMES[class]);
                    let mut visible = 0usize;
                    let mut abs_in = 0usize;
                    let mut abs_out = 0usize;
                    let mut step = 0usize;
                    loop {
                        step += 1;
                        assert!(step < 100_000, "{label}: runaway");
                        if visible < total {
                            visible = (visible + 1 + rng.below(9000)).min(total);
                        }
                        let op = if visible == total { ZSTD_e_end } else { ZSTD_e_continue };
                        let mut ib_c = ZSTD_inBuffer {
                            src: src.as_ptr() as *const c_void,
                            size: visible,
                            pos: abs_in,
                        };
                        let mut ib_r = ib_c;
                        let mut ob_c = ZSTD_outBuffer {
                            dst: oc.as_mut_ptr() as *mut c_void,
                            size: cap,
                            pos: abs_out,
                        };
                        let mut ob_r = ZSTD_outBuffer {
                            dst: or_.as_mut_ptr() as *mut c_void,
                            size: cap,
                            pos: abs_out,
                        };
                        let a = f2c(ctx.c, &mut ob_c, &mut ib_c, op);
                        let b = f2r(ctx.r, &mut ob_r, &mut ib_r, op);
                        if a != b || ib_c.pos != ib_r.pos || ob_c.pos != ob_r.pos || oc != or_ {
                            eqv(&format!("{label} step{step} ret"), a, b);
                            eqv(&format!("{label} step{step} in.pos"), ib_c.pos, ib_r.pos);
                            eqv(&format!("{label} step{step} out.pos"), ob_c.pos, ob_r.pos);
                            eqbuf(&format!("{label} step{step} dst"), &oc, &or_);
                            unreachable!();
                        }
                        assert!(!is_err(a), "{label}: error {a:#x}");
                        abs_in = ib_c.pos;
                        abs_out = ob_c.pos;
                        if op == ZSTD_e_end && a == 0 {
                            break;
                        }
                    }
                    let back = c_decompress(&oc[..abs_out], src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
        // stableOutBuffer with a too-small buffer must fail identically.
        for &size in &[1024usize, 40_000] {
            let src = gen_class(3, size, 0xABCD);
            let ctx = CtxPair::cctx();
            for &(p, v) in
                &[(ZSTD_c_compressionLevel, 3), (ZSTD_c_stableOutBuffer, 1)]
            {
                eqv("row69b setParameter", setp(ctx.c, p, v), setpr(ctx.r, p, v));
            }
            let label = format!("row69 stableOut-too-small size={size}");
            let r = drive_stable(
                ctx.c,
                ctx.r,
                &label,
                &src,
                false,
                true,
                4096,
                64,
                &[ZSTD_e_continue],
            );
            assert!(r.is_err(), "{label}: expected an error from both libraries");
        }
    }
}

// ================================================================== row 70

#[test]
fn row70_legacy_initCStream_compress_flush_end() {
    unsafe {
        let (initc, initr) = duo::<FnLevel1>("ZSTD_initCStream");
        let cin = cstream_in_size();
        let cout = cstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, cin, ALL_AT_ONCE];
        let mut rng = Rng::new(0x7000_0070);
        for level in [1i32, 3, 6, 11, 19] {
            for (ii, &ic) in in_grid.iter().enumerate() {
                for oi in 0..6usize {
                    let want = [0usize, 512, 4096, 30_000, 128 * 1024 + 1][(ii + oi) % 5];
                    let (size, oc) = pick_out_c(ic, oi, want, cout);
                    let class = (ii + oi + level as usize) % N_CLASSES;
                    let src = gen_class(class, size, rng.next_u64());
                    let zcs = CtxPair::cstream();
                    eqv(
                        &format!("row70 initCStream({level})"),
                        initc(zcs.c, level),
                        initr(zcs.r, level),
                    );
                    let flush_every = (ii + oi) % 2 == 0;
                    let label = format!(
                        "row70 lvl={level} in={ic} out={oc} size={size} flushEvery={flush_every}"
                    );
                    let got =
                        drive_legacy_cstream(zcs.c, zcs.r, &label, &src, ic, oc, flush_every)
                            .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
    }
}

// ================================================================== row 71

#[test]
fn row71_initCStream_variants() {
    unsafe {
        let (isc, isr) = duo::<FnInitCStreamSrcSize>("ZSTD_initCStream_srcSize");
        let (idc, idr) = duo::<FnInitCStreamUsingDict>("ZSTD_initCStream_usingDict");
        let (iac, iar) = duo::<FnInitCStreamAdvanced>("ZSTD_initCStream_advanced");
        let (icc, icr) = duo::<FnPtrArg1>("ZSTD_initCStream_usingCDict");
        let (icac, icar) =
            duo::<FnInitCStreamUsingCDictAdv>("ZSTD_initCStream_usingCDict_advanced");
        let (iic, iir) = duo::<FnInitCStreamInternal>("ZSTD_initCStream_internal");
        let (gpc, _) = duo::<FnGetParamsFn>("ZSTD_getParams");
        let (cpi_c, cpi_r) = duo::<FnLevel1>("ZSTD_CCtxParams_init");

        let mut rng = Rng::new(0x7100_0071);
        let dict_raw = gen_class(4, 4096, 0x1111);
        let dict_tiny = gen_class(3, 4, 0x2222);

        for level in [1i32, 3, 9, 17] {
            for &size in &[0usize, 1, 200, 4096, 40_000] {
                for class in [0usize, 3, 4, 5] {
                    let ic = [1usize, 17, 1024, ALL_AT_ONCE][rng.below(4)];
                    let oc = [3usize, 1024, cstream_out_size()][rng.below(3)];
                    let sz = budget_size(ic, oc, size);
                    let owned = gen_class(class, sz, rng.next_u64());
                    let src: &[u8] = &owned;
                    let pledges: [c_ulonglong; 3] =
                        [0, ZSTD_CONTENTSIZE_UNKNOWN, src.len() as c_ulonglong];

                    // --- _srcSize
                    for &pl in &pledges {
                        let zcs = CtxPair::cstream();
                        let label = format!(
                            "row71 initCStream_srcSize lvl={level} pledged={pl:#x} size={}",
                            src.len()
                        );
                        eqv(&label, isc(zcs.c, level, pl), isr(zcs.r, level, pl));
                        let got = drive_cstream2(
                            zcs.c,
                            zcs.r,
                            &label,
                            src,
                            ic,
                            oc,
                            &[ZSTD_e_continue],
                            false,
                            false,
                        )
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                        let back = c_decompress(&got, src.len());
                        eqbuf(&format!("{label} round trip"), src, &back);
                    }

                    // --- _usingDict (real dict, tiny dict, NULL)
                    for (dn, d) in [
                        ("raw4k", Some(&dict_raw[..])),
                        ("tiny4", Some(&dict_tiny[..])),
                        ("null", None),
                    ] {
                        let zcs = CtxPair::cstream();
                        let (dp, dl) = match d {
                            Some(b) => (b.as_ptr() as *const c_void, b.len()),
                            None => (std::ptr::null(), 0),
                        };
                        let label =
                            format!("row71 initCStream_usingDict[{dn}] lvl={level} size={}", src.len());
                        eqv(&label, idc(zcs.c, dp, dl, level), idr(zcs.r, dp, dl, level));
                        let got = drive_cstream2(
                            zcs.c,
                            zcs.r,
                            &label,
                            src,
                            ic,
                            oc,
                            &[ZSTD_e_flush],
                            false,
                            false,
                        )
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                        assert!(!got.is_empty());
                    }

                    // --- _advanced (full ZSTD_parameters, by value)
                    for &pl in &[ZSTD_CONTENTSIZE_UNKNOWN, src.len() as c_ulonglong] {
                        for cks in [0, 1] {
                            let mut p = gpc(level, pl, dict_raw.len());
                            p.fParams.checksumFlag = cks;
                            p.fParams.contentSizeFlag = 1 - cks;
                            p.fParams.noDictIDFlag = cks;
                            let zcs = CtxPair::cstream();
                            let label = format!(
                                "row71 initCStream_advanced lvl={level} pledged={pl:#x} cks={cks} size={}",
                                src.len()
                            );
                            eqv(
                                &label,
                                iac(
                                    zcs.c,
                                    dict_raw.as_ptr() as *const c_void,
                                    dict_raw.len(),
                                    p,
                                    pl,
                                ),
                                iar(
                                    zcs.r,
                                    dict_raw.as_ptr() as *const c_void,
                                    dict_raw.len(),
                                    p,
                                    pl,
                                ),
                            );
                            let got = drive_cstream2(
                                zcs.c,
                                zcs.r,
                                &label,
                                src,
                                ic,
                                oc,
                                &[ZSTD_e_continue],
                                false,
                                false,
                            )
                            .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                            assert!(!got.is_empty());
                        }
                    }

                    // --- _usingCDict / _usingCDict_advanced
                    let cd = cdict_pair(&dict_raw, level);
                    {
                        let zcs = CtxPair::cstream();
                        let label =
                            format!("row71 initCStream_usingCDict lvl={level} size={}", src.len());
                        eqv(&label, icc(zcs.c, cd.c), icr(zcs.r, cd.r));
                        let got = drive_cstream2(
                            zcs.c,
                            zcs.r,
                            &label,
                            src,
                            ic,
                            oc,
                            &[ZSTD_e_continue],
                            false,
                            false,
                        )
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                        assert!(!got.is_empty());
                    }
                    for cks in [0, 1] {
                        let fp = ZSTD_frameParameters {
                            contentSizeFlag: 1 - cks,
                            checksumFlag: cks,
                            noDictIDFlag: cks,
                        };
                        let pl = if cks == 1 { src.len() as c_ulonglong } else { ZSTD_CONTENTSIZE_UNKNOWN };
                        let zcs = CtxPair::cstream();
                        let label = format!(
                            "row71 initCStream_usingCDict_advanced lvl={level} cks={cks} size={}",
                            src.len()
                        );
                        eqv(&label, icac(zcs.c, cd.c, fp, pl), icar(zcs.r, cd.r, fp, pl));
                        let got = drive_cstream2(
                            zcs.c,
                            zcs.r,
                            &label,
                            src,
                            ic,
                            oc,
                            &[ZSTD_e_flush],
                            false,
                            false,
                        )
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                        assert!(!got.is_empty());
                    }

                    // --- _internal (dict-only, cdict-only, neither)
                    for mode in 0..3usize {
                        let params = CtxPair::cctx_params();
                        eqv(
                            "row71 CCtxParams_init",
                            cpi_c(params.c, level),
                            cpi_r(params.r, level),
                        );
                        let zcs = CtxPair::cstream();
                        let pl = if mode == 2 {
                            src.len() as c_ulonglong
                        } else {
                            ZSTD_CONTENTSIZE_UNKNOWN
                        };
                        let (dp, dl): (*const c_void, usize) = if mode == 0 {
                            (dict_raw.as_ptr() as *const c_void, dict_raw.len())
                        } else {
                            (std::ptr::null(), 0)
                        };
                        let (cdc, cdr): (*const c_void, *const c_void) = if mode == 1 {
                            (cd.c as *const c_void, cd.r as *const c_void)
                        } else {
                            (std::ptr::null(), std::ptr::null())
                        };
                        let label = format!(
                            "row71 initCStream_internal mode={mode} lvl={level} size={}",
                            src.len()
                        );
                        eqv(
                            &label,
                            iic(zcs.c, dp, dl, cdc, params.c as *const c_void, pl),
                            iir(zcs.r, dp, dl, cdr, params.r as *const c_void, pl),
                        );
                        let got = drive_cstream2(
                            zcs.c,
                            zcs.r,
                            &label,
                            src,
                            ic,
                            oc,
                            &[ZSTD_e_continue],
                            false,
                            false,
                        )
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                        assert!(!got.is_empty());
                    }
                }
            }
        }

        // A dictionary carrying the dictionary magic but garbage content must
        // be rejected identically by every init entry point (ZSTD_dct_auto).
        let mut bogus = Vec::new();
        bogus.extend_from_slice(&ZSTD_MAGIC_DICTIONARY.to_le_bytes());
        bogus.extend_from_slice(&gen_class(3, 512, 0x7777));
        for level in [1i32, 6] {
            let zcs = CtxPair::cstream();
            eqv(
                &format!("row71 initCStream_usingDict[bogus-magic] lvl={level}"),
                idc(zcs.c, bogus.as_ptr() as *const c_void, bogus.len(), level),
                idr(zcs.r, bogus.as_ptr() as *const c_void, bogus.len(), level),
            );
            let p = gpc(level, ZSTD_CONTENTSIZE_UNKNOWN, bogus.len());
            let z2 = CtxPair::cstream();
            eqv(
                &format!("row71 initCStream_advanced[bogus-magic] lvl={level}"),
                iac(
                    z2.c,
                    bogus.as_ptr() as *const c_void,
                    bogus.len(),
                    p,
                    ZSTD_CONTENTSIZE_UNKNOWN,
                ),
                iar(
                    z2.r,
                    bogus.as_ptr() as *const c_void,
                    bogus.len(),
                    p,
                    ZSTD_CONTENTSIZE_UNKNOWN,
                ),
            );
        }
    }
}

// ================================================================== row 72

#[test]
fn row72_resetCStream_pledged_sizes() {
    unsafe {
        let (initc, initr) = duo::<FnLevel1>("ZSTD_initCStream");
        let (rsc, rsr) = duo::<FnU64Arg>("ZSTD_resetCStream");
        let mut rng = Rng::new(0x7200_0072);
        for level in [1i32, 4, 12] {
            let zcs = CtxPair::cstream();
            eqv("row72 initCStream", initc(zcs.c, level), initr(zcs.r, level));
            for frame in 0..8usize {
                let size = [0usize, 1, 300, 4096, 33_000][frame % 5];
                let class = (frame + level as usize) % N_CLASSES;
                let src = gen_class(class, size, rng.next_u64());
                if frame > 0 {
                    let pl: c_ulonglong = match frame % 3 {
                        0 => 0,
                        1 => ZSTD_CONTENTSIZE_UNKNOWN,
                        _ => src.len() as c_ulonglong,
                    };
                    eqv(
                        &format!("row72 resetCStream({pl:#x})"),
                        rsc(zcs.c, pl),
                        rsr(zcs.r, pl),
                    );
                }
                let ic = [1usize, 17, 1024, ALL_AT_ONCE][frame % 4];
                let oc = [3usize, 17, 1024, cstream_out_size()][frame % 4];
                let label = format!("row72 lvl={level} frame{frame} size={size}");
                let got = drive_cstream2(
                    zcs.c,
                    zcs.r,
                    &label,
                    &src,
                    ic,
                    oc,
                    &[ZSTD_e_continue],
                    false,
                    true, // probe getFrameProgression / toFlushNow
                )
                .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                let back = c_decompress(&got, src.len());
                eqbuf(&format!("{label} round trip"), &src, &back);
            }
        }
    }
}

// ================================================================== row 73

#[test]
fn row73_create_advanced_custom_mem() {
    unsafe {
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let mut rng = Rng::new(0x7300_0073);
        for (cn, fn_) in
            [("ZSTD_createCStream_advanced", "ZSTD_freeCStream"), ("ZSTD_createCCtx_advanced", "ZSTD_freeCCtx")]
        {
            for &size in &[0usize, 7, 1024, 30_000] {
                for class in [0usize, 3, 4] {
                    let p = advanced_pair(cn, fn_);
                    let level = 1 + rng.range(0, 8);
                    eqv(
                        &format!("row73 {cn} setParameter"),
                        setp(p.c, ZSTD_c_compressionLevel, level),
                        setpr(p.r, ZSTD_c_compressionLevel, level),
                    );
                    let src = gen_class(class, size, rng.next_u64());
                    let label = format!("row73 {cn} size={size} class={} lvl={level}", CLASS_NAMES[class]);
                    let got = drive_cstream2(
                        p.c,
                        p.r,
                        &label,
                        &src,
                        17,
                        1024,
                        &[ZSTD_e_continue, ZSTD_e_flush],
                        false,
                        false,
                    )
                    .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    let back = c_decompress(&got, src.len());
                    eqbuf(&format!("{label} round trip"), &src, &back);
                }
            }
        }
        for (cn, fn_) in [
            ("ZSTD_createDStream_advanced", "ZSTD_freeDStream"),
            ("ZSTD_createDCtx_advanced", "ZSTD_freeDCtx"),
        ] {
            for &size in &[0usize, 7, 1024, 30_000] {
                for class in [0usize, 3, 6] {
                    let src = gen_class(class, size, rng.next_u64());
                    let frame = c_frame(&src, 5, &[(ZSTD_c_checksumFlag, 1)]);
                    let p = advanced_pair(cn, fn_);
                    let label = format!("row73 {cn} size={size} class={}", CLASS_NAMES[class]);
                    let out = drive_dstream(p.c, p.r, &label, &frame, 13, 1000, false)
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    eqbuf(&format!("{label} content"), &src, &out);
                }
            }
        }
    }
}

// ================================================================== row 74

#[test]
fn row74_decompressStream_grid() {
    unsafe {
        let din = dstream_in_size();
        let dout = dstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, din, ALL_AT_ONCE];
        let mut rng = Rng::new(0x7400_0074);
        let sizes = [
            0usize,
            1,
            7,
            1024,
            8192,
            70_000,
            128 * 1024 - 1,
            128 * 1024,
            128 * 1024 + 1,
            256 * 1024,
        ];
        for (ii, &ic) in in_grid.iter().enumerate() {
            for oi in 0..6usize {
                for (si, &want) in sizes.iter().enumerate() {
                    let (size, oc) = pick_out_d(ic, oi, want, dout);
                    let class = (ii * 3 + oi + si) % N_CLASSES;
                    let src = gen_class(class, size, rng.next_u64());
                    let level = [1, 5, 13][(ii + oi + si) % 3];
                    let cks = ((ii + oi) % 2) as c_int;
                    let cs = (1 - cks) as c_int;
                    let frame = c_frame(
                        &src,
                        level,
                        &[(ZSTD_c_checksumFlag, cks), (ZSTD_c_contentSizeFlag, cs)],
                    );
                    let ctx = CtxPair::dstream();
                    let label = format!(
                        "row74 in={ic} out={oc} size={size} class={} lvl={level} cks={cks}",
                        CLASS_NAMES[class]
                    );
                    let out = drive_dstream(ctx.c, ctx.r, &label, &frame, ic, oc, false)
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    eqbuf(&format!("{label} content"), &src, &out);
                }
            }
        }
        // multi-frame streams
        for n in 2..5usize {
            let mut all = Vec::new();
            let mut cat = Vec::new();
            for k in 0..n {
                let s = gen_class(k % N_CLASSES, 100 * (k + 1) + 7, rng.next_u64());
                cat.extend_from_slice(&c_frame(&s, 3, &[(ZSTD_c_checksumFlag, (k % 2) as c_int)]));
                all.extend_from_slice(&s);
            }
            for &ic in &[1usize, 17, ALL_AT_ONCE] {
                let ctx = CtxPair::dstream();
                let label = format!("row74 multiframe n={n} in={ic}");
                let out = drive_dstream(ctx.c, ctx.r, &label, &cat, ic, 333, false)
                    .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                eqbuf(&format!("{label} content"), &all, &out);
            }
        }
    }
}

// ================================================================== row 75

#[test]
fn row75_decompressStream_simpleArgs() {
    unsafe {
        let din = dstream_in_size();
        let dout = dstream_out_size();
        let in_grid = [1usize, 3, 17, 1024, din, ALL_AT_ONCE];
        let mut rng = Rng::new(0x7500_0075);
        let sizes = [0usize, 1, 512, 8192, 40_000, 128 * 1024 + 1, 200_000];
        for (ii, &ic) in in_grid.iter().enumerate() {
            for oi in 0..6usize {
                for (si, &want) in sizes.iter().enumerate() {
                    let (size, oc) = pick_out_d(ic, oi, want, dout);
                    let class = (ii + oi * 5 + si) % N_CLASSES;
                    let src = gen_class(class, size, rng.next_u64());
                    let frame = c_frame(
                        &src,
                        [3, 7, 12][si % 3],
                        &[(ZSTD_c_checksumFlag, (si % 2) as c_int)],
                    );
                    let ctx = CtxPair::dctx();
                    let label = format!("row75 in={ic} out={oc} size={size}");
                    let out = drive_dstream(ctx.c, ctx.r, &label, &frame, ic, oc, true)
                        .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                    eqbuf(&format!("{label} content"), &src, &out);
                }
            }
        }
    }
}

// ================================================================== row 76

#[test]
fn row76_initDStream_resetDStream_DCtx_reset() {
    unsafe {
        let (idc, idr) = duo::<FnFreePtr>("ZSTD_initDStream");
        let (rdc, rdr) = duo::<FnFreePtr>("ZSTD_resetDStream");
        let (drc, drr) = duo::<FnReset>("ZSTD_DCtx_reset");
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let mut rng = Rng::new(0x7600_0076);
        let zds = CtxPair::dstream();
        for round in 0..30usize {
            let size = [0usize, 1, 9, 700, 5000, 40_000][round % 6];
            let class = round % N_CLASSES;
            let src = gen_class(class, size, rng.next_u64());
            let frame = c_frame(
                &src,
                [1, 3, 9][round % 3],
                &[(ZSTD_c_checksumFlag, (round % 2) as c_int)],
            );
            let how = round % 5;
            let label = format!("row76 round={round} how={how} size={size}");
            match how {
                0 => eqv(&format!("{label} initDStream"), idc(zds.c), idr(zds.r)),
                1 => eqv(&format!("{label} resetDStream"), rdc(zds.c), rdr(zds.r)),
                2 => eqv(
                    &format!("{label} DCtx_reset(session_only)"),
                    drc(zds.c, ZSTD_reset_session_only),
                    drr(zds.r, ZSTD_reset_session_only),
                ),
                3 => eqv(
                    &format!("{label} DCtx_reset(parameters)"),
                    drc(zds.c, ZSTD_reset_parameters),
                    drr(zds.r, ZSTD_reset_parameters),
                ),
                _ => eqv(
                    &format!("{label} DCtx_reset(session_and_parameters)"),
                    drc(zds.c, ZSTD_reset_session_and_parameters),
                    drr(zds.r, ZSTD_reset_session_and_parameters),
                ),
            }
            if how == 3 || how == 4 {
                // parameters were wiped: set one back, both must agree
                eqv(
                    &format!("{label} setParameter(windowLogMax)"),
                    setp(zds.c, ZSTD_d_windowLogMax, 27),
                    setpr(zds.r, ZSTD_d_windowLogMax, 27),
                );
            }
            let ic = [1usize, 17, 1024, ALL_AT_ONCE][round % 4];
            let oc = [1usize, 17, 1024, dstream_out_size()][(round + 1) % 4];
            let out = drive_dstream(zds.c, zds.r, &label, &frame, ic, oc, false)
                .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
            eqbuf(&format!("{label} content"), &src, &out);
        }
        // A reset in the middle of a decode must also behave identically.
        let src = gen_class(4, 20_000, 0x9999);
        let frame = c_frame(&src, 6, &[(ZSTD_c_checksumFlag, 1)]);
        let (fdc, fdr) = duo::<FnDStream>("ZSTD_decompressStream");
        for &directive in &[ZSTD_reset_session_only, ZSTD_reset_session_and_parameters] {
            let z = CtxPair::dstream();
            let mut oc = vec![0xA5u8; 4096];
            let mut or_ = vec![0xA5u8; 4096];
            let half = frame.len() / 2;
            let mut ib_c =
                ZSTD_inBuffer { src: frame.as_ptr() as *const c_void, size: half, pos: 0 };
            let mut ib_r = ib_c;
            let mut ob_c =
                ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: 4096, pos: 0 };
            let mut ob_r =
                ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: 4096, pos: 0 };
            let a = fdc(z.c, &mut ob_c, &mut ib_c);
            let b = fdr(z.r, &mut ob_r, &mut ib_r);
            eqv("row76 mid ret", a, b);
            eqv("row76 mid in.pos", ib_c.pos, ib_r.pos);
            eqv("row76 mid out.pos", ob_c.pos, ob_r.pos);
            eqbuf("row76 mid dst", &oc, &or_);
            eqv(
                "row76 mid DCtx_reset",
                drc(z.c, directive),
                drr(z.r, directive),
            );
            let label = format!("row76 after-mid-reset({directive})");
            let out = drive_dstream(z.c, z.r, &label, &frame, 1024, 2048, false)
                .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
            eqbuf(&format!("{label} content"), &src, &out);
        }
    }
}

// ================================================================== row 77

#[test]
fn row77_d_stableOutBuffer() {
    unsafe {
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (fdc, fdr) = duo::<FnDStream>("ZSTD_decompressStream");
        let mut rng = Rng::new(0x7700_0077);
        for &size in &[0usize, 1, 100, 4096, 33_000, 140_000] {
            for class in 0..N_CLASSES {
                let src = gen_class(class, size, rng.next_u64());
                let frame = c_frame(
                    &src,
                    [1, 5, 11][class % 3],
                    &[(ZSTD_c_checksumFlag, (class % 2) as c_int)],
                );
                for &ic in &[1usize, 17, 1024, ALL_AT_ONCE] {
                    let ctx = CtxPair::dstream();
                    eqv(
                        "row77 setParameter(d_stableOutBuffer)",
                        setp(ctx.c, ZSTD_d_stableOutBuffer, 1),
                        setpr(ctx.r, ZSTD_d_stableOutBuffer, 1),
                    );
                    let cap = src.len() + 32;
                    let mut oc = vec![0xA5u8; cap];
                    let mut or_ = vec![0xA5u8; cap];
                    let mut ob_c =
                        ZSTD_outBuffer { dst: oc.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                    let mut ob_r =
                        ZSTD_outBuffer { dst: or_.as_mut_ptr() as *mut c_void, size: cap, pos: 0 };
                    let label = format!(
                        "row77 size={size} class={} in={ic}",
                        CLASS_NAMES[class]
                    );
                    let mut ipos = 0usize;
                    let mut last = 1usize;
                    let mut step = 0usize;
                    let mut stall = 0;
                    while last != 0 || ipos < frame.len() {
                        step += 1;
                        assert!(step < 500_000, "{label}: runaway");
                        let rem = frame.len() - ipos;
                        let take = if ic == ALL_AT_ONCE { rem } else { ic.min(rem) };
                        let sp = frame.as_ptr().add(ipos) as *const c_void;
                        let mut ib_c = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
                        let mut ib_r = ZSTD_inBuffer { src: sp, size: take, pos: 0 };
                        let before = ob_c.pos;
                        let a = fdc(ctx.c, &mut ob_c, &mut ib_c);
                        let b = fdr(ctx.r, &mut ob_r, &mut ib_r);
                        if a != b || ib_c.pos != ib_r.pos || ob_c.pos != ob_r.pos || oc != or_ {
                            eqv(&format!("{label} step{step} ret"), a, b);
                            eqv(&format!("{label} step{step} in.pos"), ib_c.pos, ib_r.pos);
                            eqv(&format!("{label} step{step} out.pos"), ob_c.pos, ob_r.pos);
                            eqbuf(&format!("{label} step{step} dst"), &oc, &or_);
                            unreachable!("{label}: mismatch not localized");
                        }
                        assert!(!is_err(a), "{label}: unexpected error {a:#x}");
                        ipos += ib_c.pos;
                        last = a;
                        if ib_c.pos > 0 || ob_c.pos > before {
                            stall = 0;
                        } else {
                            stall += 1;
                            assert!(stall < 48, "{label}: no progress");
                        }
                    }
                    eqv(&format!("{label} final out.pos"), ob_c.pos, src.len());
                    eqbuf(&format!("{label} content"), &src, &oc[..ob_c.pos]);
                }
            }
        }
    }
}

// ================================================================== row 78

#[test]
fn row78_d_maxBlockSize_and_huffman_assembly() {
    unsafe {
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let mut rng = Rng::new(0x7800_0078);
        for &c_mbs in &[0i32, 1024, 65536, 131072] {
            for &size in &[0usize, 700, 5000, 40_000, 140_000] {
                for class in [0usize, 3, 4, 6] {
                    let src = gen_class(class, size, rng.next_u64());
                    let frame = c_frame(
                        &src,
                        [3, 8][class % 2],
                        &[(ZSTD_c_maxBlockSize, c_mbs), (ZSTD_c_checksumFlag, 1)],
                    );
                    for &d_mbs in &[0i32, 1024, 131072] {
                        for &asm_off in &[0i32, 1] {
                            let ctx = CtxPair::dstream();
                            eqv(
                                "row78 setParameter(d_maxBlockSize)",
                                setp(ctx.c, ZSTD_d_maxBlockSize, d_mbs),
                                setpr(ctx.r, ZSTD_d_maxBlockSize, d_mbs),
                            );
                            eqv(
                                "row78 setParameter(d_disableHuffmanAssembly)",
                                setp(ctx.c, ZSTD_d_disableHuffmanAssembly, asm_off),
                                setpr(ctx.r, ZSTD_d_disableHuffmanAssembly, asm_off),
                            );
                            let label = format!(
                                "row78 c_mbs={c_mbs} d_mbs={d_mbs} asmOff={asm_off} size={size} class={}",
                                CLASS_NAMES[class]
                            );
                            let ic = [17usize, 1024, ALL_AT_ONCE][(class + size) % 3];
                            match drive_dstream(ctx.c, ctx.r, &label, &frame, ic, 4096, false) {
                                Ok(out) => eqbuf(&format!("{label} content"), &src, &out),
                                Err(_) => { /* identical error in both, already asserted */ }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================================== row 79

#[test]
fn row79_d_forceIgnoreChecksum() {
    unsafe {
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let mut rng = Rng::new(0x7900_0079);
        for &size in &[1usize, 100, 5000, 60_000] {
            for class in 0..N_CLASSES {
                let src = gen_class(class, size, rng.next_u64());
                let good = c_frame(&src, [2, 7][class % 2], &[(ZSTD_c_checksumFlag, 1)]);
                // corrupt the 4-byte xxhash trailer
                let mut bad = good.clone();
                let n = bad.len();
                let flip = n - 1 - rng.below(4);
                bad[flip] ^= 0x40;
                for &ignore in &[0i32, 1] {
                    for (which, frame) in [("good", &good), ("bad", &bad)] {
                        for &ic in &[1usize, 1024, ALL_AT_ONCE] {
                            let ctx = CtxPair::dstream();
                            eqv(
                                "row79 setParameter(forceIgnoreChecksum)",
                                setp(ctx.c, ZSTD_d_forceIgnoreChecksum, ignore),
                                setpr(ctx.r, ZSTD_d_forceIgnoreChecksum, ignore),
                            );
                            let label = format!(
                                "row79 {which} ignore={ignore} size={size} class={} in={ic}",
                                CLASS_NAMES[class]
                            );
                            let res =
                                drive_dstream(ctx.c, ctx.r, &label, frame, ic, 2048, false);
                            match res {
                                Ok(out) => {
                                    eqbuf(&format!("{label} content"), &src, &out);
                                    assert!(
                                        which == "good" || ignore == 1,
                                        "{label}: corrupt checksum accepted while validating"
                                    );
                                }
                                Err(e) => {
                                    assert!(
                                        which == "bad",
                                        "{label}: good frame rejected ({e:#x})"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================================== row 80

#[test]
fn row80_nextSrcSize_nextInputType_bufferless() {
    unsafe {
        let (dbc, dbr) = duo::<FnFreePtr>("ZSTD_decompressBegin");
        let (nsc, nsr) = duo::<FnConstPtrToSize>("ZSTD_nextSrcSizeToDecompress");
        let (nic, nir) = duo::<FnConstPtrToInt>("ZSTD_nextInputType");
        let (dcc, dcr) = duo::<FnDecompressContinue>("ZSTD_decompressContinue");
        let (gfh, _) = duo::<FnGetFrameHeader>("ZSTD_getFrameHeader");
        let mut rng = Rng::new(0x8000_0080);
        for &size in &[0usize, 1, 33, 700, 9000, 70_000, 200_000] {
            for class in 0..N_CLASSES {
                let src = gen_class(class, size, rng.next_u64());
                let cks = ((class + size) % 2) as c_int;
                let cs = 1 - cks;
                let frame = c_frame(
                    &src,
                    [1, 4, 10][class % 3],
                    &[(ZSTD_c_checksumFlag, cks), (ZSTD_c_contentSizeFlag, cs)],
                );
                let mut zfh = ZSTD_frameHeader::default();
                let h = gfh(&mut zfh, frame.as_ptr() as *const c_void, frame.len());
                assert_eq!(h, 0, "getFrameHeader failed");
                // every other case is preceded by a skippable frame, which
                // drives ZSTD_nextInputType() through ZSTDnit_skippableFrame
                let frame = if class % 2 == 1 {
                    let (wsf, _) = duo::<FnWriteSkippable>("ZSTD_writeSkippableFrame");
                    let payload = gen_class(class, 1 + (size % 97), 0x5151);
                    let mut sk = vec![0u8; payload.len() + 16];
                    let n = wsf(
                        sk.as_mut_ptr() as *mut c_void,
                        sk.len(),
                        payload.as_ptr() as *const c_void,
                        payload.len(),
                        (class % 16) as c_uint,
                    );
                    assert!(!is_err(n), "writeSkippableFrame failed");
                    sk.truncate(n);
                    sk.extend_from_slice(&frame);
                    sk
                } else {
                    frame
                };
                let cap = src.len() + ZSTD_BLOCKSIZE_MAX + zfh.windowSize as usize + 64;
                let ctx = CtxPair::dctx();
                let label = format!("row80 size={size} class={} cks={cks}", CLASS_NAMES[class]);
                eqv(&format!("{label} decompressBegin"), dbc(ctx.c), dbr(ctx.r));
                let mut dst_c = vec![0xA5u8; cap];
                let mut dst_r = vec![0xA5u8; cap];
                let mut ipos = 0usize;
                let mut dpos = 0usize;
                let mut step = 0usize;
                loop {
                    step += 1;
                    assert!(step < 200_000, "{label}: runaway");
                    let nc = nsc(ctx.c);
                    let nr = nsr(ctx.r);
                    let tc = nic(ctx.c);
                    let tr = nir(ctx.r);
                    if nc != nr || tc != tr {
                        eqv(&format!("{label} step{step} nextSrcSizeToDecompress"), nc, nr);
                        eqv(&format!("{label} step{step} nextInputType"), tc, tr);
                        unreachable!();
                    }
                    if nc == 0 {
                        // frame boundary: restart for the next concatenated frame
                        if ipos < frame.len() {
                            eqv(
                                &format!("{label} step{step} re-decompressBegin"),
                                dbc(ctx.c),
                                dbr(ctx.r),
                            );
                            continue;
                        }
                        break;
                    }
                    assert!(!is_err(nc), "{label}: nextSrcSize error {nc:#x}");
                    assert!(ipos + nc <= frame.len(), "{label}: frame exhausted");
                    let sp = frame.as_ptr().add(ipos) as *const c_void;
                    let a = dcc(
                        ctx.c,
                        dst_c.as_mut_ptr().add(dpos) as *mut c_void,
                        cap - dpos,
                        sp,
                        nc,
                    );
                    let b = dcr(
                        ctx.r,
                        dst_r.as_mut_ptr().add(dpos) as *mut c_void,
                        cap - dpos,
                        sp,
                        nc,
                    );
                    if a != b || dst_c != dst_r {
                        eqv(&format!("{label} step{step} decompressContinue"), a, b);
                        eqbuf(&format!("{label} step{step} dst"), &dst_c, &dst_r);
                        unreachable!();
                    }
                    assert!(!is_err(a), "{label}: decompressContinue error {a:#x}");
                    ipos += nc;
                    dpos += a;
                }
                eqv(&format!("{label} consumed"), ipos, frame.len());
                eqbuf(&format!("{label} content"), &src, &dst_c[..dpos]);
            }
        }
    }
}

// ================================================================== row 81

#[test]
fn row81_DCtx_setMaxWindowSize_and_setFormat() {
    unsafe {
        let (smc, smr) = duo::<FnSizeTArg>("ZSTD_DCtx_setMaxWindowSize");
        let (sfc, sfr) = duo::<FnIntArg>("ZSTD_DCtx_setFormat");
        let mut rng = Rng::new(0x8100_0081);

        // --- maxWindowSize grid
        for &wlog in &[10i32, 11, 15, 18, 21] {
            let size = 1usize << (wlog + 1).min(18);
            let src = gen_class(5, size, rng.next_u64());
            let frame = c_frame(
                &src,
                3,
                &[(ZSTD_c_windowLog, wlog), (ZSTD_c_checksumFlag, 1)],
            );
            for &mw in &[
                1usize << 10,
                1usize << 11,
                (1usize << 15) + 1,
                1usize << 18,
                1usize << 22,
                1usize << 27,
                1usize << 31,
                // out of bounds on both ends
                1,
                usize::MAX,
            ] {
                let ctx = CtxPair::dstream();
                let a = smc(ctx.c, mw);
                let b = smr(ctx.r, mw);
                eqv(&format!("row81 setMaxWindowSize({mw})"), a, b);
                if is_err(a) {
                    continue;
                }
                let label = format!("row81 wlog={wlog} maxWindow={mw} size={size}");
                match drive_dstream(ctx.c, ctx.r, &label, &frame, 1024, 4096, false) {
                    Ok(out) => eqbuf(&format!("{label} content"), &src, &out),
                    Err(_) => {}
                }
            }
        }

        // --- setFormat × magicless frames
        for &size in &[0usize, 1, 500, 9000, 60_000] {
            for class in [0usize, 3, 4, 5] {
                let src = gen_class(class, size, rng.next_u64());
                for &cfmt in &[0i32, 1] {
                    let frame = c_frame(
                        &src,
                        [2, 9][class % 2],
                        &[(ZSTD_c_format, cfmt), (ZSTD_c_checksumFlag, (class % 2) as c_int)],
                    );
                    for &dfmt in &[0i32, 1] {
                        for &ic in &[1usize, 17, ALL_AT_ONCE] {
                            let ctx = CtxPair::dstream();
                            let a = sfc(ctx.c, dfmt);
                            let b = sfr(ctx.r, dfmt);
                            eqv(&format!("row81 DCtx_setFormat({dfmt})"), a, b);
                            let label = format!(
                                "row81 cfmt={cfmt} dfmt={dfmt} size={size} class={} in={ic}",
                                CLASS_NAMES[class]
                            );
                            match drive_dstream(ctx.c, ctx.r, &label, &frame, ic, 3000, false) {
                                Ok(out) => {
                                    if cfmt == dfmt {
                                        eqbuf(&format!("{label} content"), &src, &out);
                                    }
                                }
                                Err(_) => {
                                    assert_ne!(
                                        cfmt, dfmt,
                                        "{label}: matching formats must decode"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================================== progression

#[test]
fn row65b_frame_progression_and_toFlushNow() {
    unsafe {
        let (pgc, pgr) = duo::<FnGetProgression>("ZSTD_getFrameProgression");
        let (tfc, tfr) = duo::<FnConstPtrToSize>("ZSTD_toFlushNow");
        let (setp, setpr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let mut rng = Rng::new(0x6501_0065);
        // fresh contexts
        for name in ["ZSTD_createCCtx", "ZSTD_createCStream"] {
            let p = CtxPair::new(
                name,
                if name == "ZSTD_createCCtx" { "ZSTD_freeCCtx" } else { "ZSTD_freeCStream" },
            );
            eqv(&format!("fresh {name} progression"), pgc(p.c), pgr(p.r));
            eqv(&format!("fresh {name} toFlushNow"), tfc(p.c), tfr(p.r));
        }
        for &size in &[0usize, 1, 1024, 40_000, 150_000] {
            for class in [0usize, 3, 4, 5, 6] {
                let src = gen_class(class, size, rng.next_u64());
                let ctx = CtxPair::cctx();
                let level = 1 + rng.range(0, 9);
                eqv(
                    "progression setParameter",
                    setp(ctx.c, ZSTD_c_compressionLevel, level),
                    setpr(ctx.r, ZSTD_c_compressionLevel, level),
                );
                let label = format!("row65b size={size} class={} lvl={level}", CLASS_NAMES[class]);
                let got = drive_cstream2(
                    ctx.c,
                    ctx.r,
                    &label,
                    &src,
                    777,
                    999,
                    &[ZSTD_e_continue, ZSTD_e_continue, ZSTD_e_flush],
                    false,
                    true,
                )
                .unwrap_or_else(|e| panic!("{label}: both errored {e:#x}"));
                let back = c_decompress(&got, src.len());
                eqbuf(&format!("{label} round trip"), &src, &back);
                let pc = pgc(ctx.c);
                eqv(&format!("{label} final progression"), pc, pgr(ctx.r));
                assert_eq!(pc.consumed, src.len() as c_ulonglong, "{label}: consumed");
            }
        }
        let _ = (c_uint::MAX, dstream_in_size());
    }
}
