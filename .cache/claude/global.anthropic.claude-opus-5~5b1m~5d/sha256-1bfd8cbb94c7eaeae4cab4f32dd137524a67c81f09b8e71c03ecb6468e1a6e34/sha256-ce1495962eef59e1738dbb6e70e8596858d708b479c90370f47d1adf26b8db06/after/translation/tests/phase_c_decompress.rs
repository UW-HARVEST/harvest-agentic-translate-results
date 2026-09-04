//! Phase C — ERRORS.md rows covered by `phase_c_decompress`.
//!
//! Every rejection the decoder can produce is reached by *constructing* the
//! invalid input and asserting that the C and the Rust `.so` return the SAME
//! `ZSTD_ErrorCode` (not merely "both failed").
//!
//! Covers `decompress/zstd_decompress.c`, `decompress/zstd_decompress_block.c`,
//! `decompress/huf_decompress.c`, `decompress/zstd_ddict.c` and the
//! `common/fse_decompress.c` / `common/entropy_common.c` sites reached from a
//! frame.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

type FnU64FromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_ulonglong;
type FnUFromBuf = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
type FnFromBuf = unsafe extern "C" fn(*const c_void, usize) -> usize;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

/// Compare the *error code*, not just "both failed".
#[track_caller]
fn eqcode(what: &str, c: usize, r: usize) {
    unsafe {
        let (gcc, gcr) = duo::<unsafe extern "C" fn(usize) -> c_uint>("ZSTD_getErrorCode");
        let (nc, nr) = duo::<FnErrName>("ZSTD_getErrorName");
        if c != r {
            panic!(
                "{what}: C returned {c:#x} (code {} = {}), Rust returned {r:#x} (code {} = {})",
                gcc(c),
                cstr(nc(c)),
                gcr(r),
                cstr(nr(r))
            );
        }
        // and the decoded code/name must agree too
        assert_eq!(gcc(c), gcr(r), "{what}: error code mismatch");
        assert_eq!(cstr(nc(c)), cstr(nr(r)), "{what}: error name mismatch");
    }
}

/// 64 KiB-aligned-ish workspace pre-filled with a fixed pattern, used for the
/// `ZSTD_initStatic*` contexts.
///
/// WHY STATIC CONTEXTS: on an error path zstd may leave partial output in `dst`
/// that was copied out of **uninitialised** context memory. Concretely,
/// `ZSTD_execSequence()` performs an unconditional 16-byte `ZSTD_copy16(op,
/// *litPtr)` even when `litLength < 16`, so a frame whose literals section was
/// copied into `dctx->litBuffer` leaks up to 14 bytes of never-written
/// `litBuffer` into `dst` before the following offset check returns
/// `corruption_detected`. With a heap-allocated DCtx those bytes are whatever
/// `malloc` last left there, which differs between the two libraries purely
/// because their allocation histories differ - it is not a translation
/// difference. Verified: with `ZSTD_initStaticDCtx` over a workspace filled
/// with 0x11 / 0x22 / 0x00, the C and the Rust `.so` emit *byte-identical*
/// output (the fill byte) in every case. So all context-based decodes here run
/// on a static workspace with an identical fill, which makes the error-path
/// output fully specified and comparable.
const WS_FILL: u8 = 0x6B;

struct StaticCtx {
    _ws_c: Vec<u8>,
    _ws_r: Vec<u8>,
    c: *mut c_void,
    r: *mut c_void,
}

unsafe fn static_dctx() -> StaticCtx {
    let (ic, ir) = duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>("ZSTD_initStaticDCtx");
    let (est, _) = duo::<FnSizeT0>("ZSTD_estimateDCtxSize");
    let need = est();
    let mut ws_c = vec![WS_FILL; need + 64];
    let mut ws_r = vec![WS_FILL; need + 64];
    let c = ic(ws_c.as_mut_ptr() as *mut c_void, need);
    let r = ir(ws_r.as_mut_ptr() as *mut c_void, need);
    assert!(!c.is_null() && !r.is_null(), "initStaticDCtx failed");
    StaticCtx { _ws_c: ws_c, _ws_r: ws_r, c, r }
}

unsafe fn static_dstream(window: usize) -> StaticCtx {
    let (ic, ir) =
        duo::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>("ZSTD_initStaticDStream");
    let (est, _) = duo::<FnSizeT1>("ZSTD_estimateDStreamSize");
    let need = est(window);
    let mut ws_c = vec![WS_FILL; need + 64];
    let mut ws_r = vec![WS_FILL; need + 64];
    let c = ic(ws_c.as_mut_ptr() as *mut c_void, need);
    let r = ir(ws_r.as_mut_ptr() as *mut c_void, need);
    assert!(!c.is_null() && !r.is_null(), "initStaticDStream failed");
    StaticCtx { _ws_c: ws_c, _ws_r: ws_r, c, r }
}

/// Decode `frame` through every one-shot / streaming / bufferless entry point in
/// both libraries and require identical results.
unsafe fn diff_decode_all(tag: &str, frame: &[u8], out_hint: usize) {
    let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
    let (ddc, ddr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
    let (dsc, dsr) = duo::<FnDStream>("ZSTD_decompressStream");
    let (idc, idr) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_initDStream");
    let (fcc, fcr) = duo::<FnFromBuf>("ZSTD_findFrameCompressedSize");
    let (gfc, gfr) = duo::<FnU64FromBuf>("ZSTD_getFrameContentSize");
    let (fdc, fdr) = duo::<FnU64FromBuf>("ZSTD_findDecompressedSize");
    let (dbc, dbr) = duo::<FnU64FromBuf>("ZSTD_decompressBound");
    let (ifc, ifr) = duo::<FnUFromBuf>("ZSTD_isFrame");
    let (isc, isr) = duo::<FnUFromBuf>("ZSTD_isSkippableFrame");
    let (fhc, fhr) = duo::<FnFromBuf>("ZSTD_frameHeaderSize");
    let (ghc, ghr) =
        duo::<unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, usize) -> usize>(
            "ZSTD_getFrameHeader",
        );
    let (mgc, mgr) = duo::<FnFromBuf>("ZSTD_decompressionMargin");
    let (efc, efr) = duo::<FnFromBuf>("ZSTD_estimateDStreamSize_fromFrame");
    let (gdc, gdr) = duo::<FnUFromBuf>("ZSTD_getDictID_fromFrame");

    let p = frame.as_ptr() as *const c_void;
    let n = frame.len();
    let cap = out_hint.max(64);

    // --- pure introspection
    eqcode(&format!("{tag} findFrameCompressedSize"), fcc(p, n), fcr(p, n));
    eqv(&format!("{tag} getFrameContentSize"), gfc(p, n), gfr(p, n));
    eqv(&format!("{tag} findDecompressedSize"), fdc(p, n), fdr(p, n));
    eqv(&format!("{tag} decompressBound"), dbc(p, n), dbr(p, n));
    eqv(&format!("{tag} isFrame"), ifc(p, n), ifr(p, n));
    eqv(&format!("{tag} isSkippableFrame"), isc(p, n), isr(p, n));
    eqcode(&format!("{tag} frameHeaderSize"), fhc(p, n), fhr(p, n));
    eqcode(&format!("{tag} decompressionMargin"), mgc(p, n), mgr(p, n));
    eqcode(
        &format!("{tag} estimateDStreamSize_fromFrame"),
        efc(p, n),
        efr(p, n),
    );
    eqv(&format!("{tag} getDictID_fromFrame"), gdc(p, n), gdr(p, n));
    {
        let mut hc = ZSTD_frameHeader::default();
        let mut hr = ZSTD_frameHeader::default();
        let a = ghc(&mut hc, p, n);
        let b = ghr(&mut hr, p, n);
        eqcode(&format!("{tag} getFrameHeader"), a, b);
        eqv(&format!("{tag} getFrameHeader out"), hc, hr);
    }

    // --- one-shot
    let mut oc = vec![0x5Cu8; cap];
    let mut or_ = vec![0x5Cu8; cap];
    let a = dc(oc.as_mut_ptr() as *mut c_void, cap, p, n);
    let b = dr(or_.as_mut_ptr() as *mut c_void, cap, p, n);
    eqcode(&format!("{tag} ZSTD_decompress"), a, b);
    if !is_err(a) {
        // ZSTD_decompress() allocates its own DCtx, so on the error path the
        // leaked litBuffer bytes come from uninitialised heap (see WS_FILL
        // above); dst is only specified when the call succeeds. The static-
        // workspace paths below cover the error-path output bytes.
        eqbuf(&format!("{tag} ZSTD_decompress dst"), &oc, &or_);
    }

    // --- DCtx one-shot (reused ctx)
    {
        let d = static_dctx();
        let mut oc = vec![0x5Cu8; cap];
        let mut or_ = vec![0x5Cu8; cap];
        let a = ddc(d.c, oc.as_mut_ptr() as *mut c_void, cap, p, n);
        let b = ddr(d.r, or_.as_mut_ptr() as *mut c_void, cap, p, n);
        eqcode(&format!("{tag} ZSTD_decompressDCtx"), a, b);
        // static workspace with an identical fill in both libraries => the
        // error-path partial output IS specified and must match byte for byte
        eqbuf(&format!("{tag} ZSTD_decompressDCtx dst"), &oc, &or_);
    }

    // --- streaming, whole input at once and one byte at a time
    for chunk in [n.max(1), 1usize] {
        let d = static_dstream(1 << 20);
        eqcode(&format!("{tag} initDStream"), idc(d.c), idr(d.r));
        let mut oc = vec![0x5Cu8; cap];
        let mut or_ = vec![0x5Cu8; cap];
        let mut ic = ZSTD_inBuffer { src: p, size: 0, pos: 0 };
        let mut ir = ZSTD_inBuffer { src: p, size: 0, pos: 0 };
        let mut obc = ZSTD_outBuffer {
            dst: oc.as_mut_ptr() as *mut c_void,
            size: cap,
            pos: 0,
        };
        let mut obr = ZSTD_outBuffer {
            dst: or_.as_mut_ptr() as *mut c_void,
            size: cap,
            pos: 0,
        };
        let mut step = 0;
        loop {
            step += 1;
            ic.size = (ic.size + chunk).min(n);
            ir.size = ic.size;
            let a = dsc(d.c, &mut obc, &mut ic);
            let b = dsr(d.r, &mut obr, &mut ir);
            eqcode(&format!("{tag} decompressStream chunk={chunk} step={step}"), a, b);
            eqv(
                &format!("{tag} decompressStream chunk={chunk} step={step} in.pos"),
                ic.pos,
                ir.pos,
            );
            eqv(
                &format!("{tag} decompressStream chunk={chunk} step={step} out.pos"),
                obc.pos,
                obr.pos,
            );
            if is_err(a) || a == 0 || step > 3000 {
                break;
            }
            if ic.size == n && ic.pos == n && obc.pos == cap {
                break;
            }
            if ic.size == n && ic.pos == ic.size && obc.pos < cap && step > 2 {
                // no forward progress possible; the libraries must agree that
                // this is the end (or return the same error)
                if a != 0 {
                    continue;
                }
                break;
            }
        }
        eqbuf(&format!("{tag} decompressStream chunk={chunk} dst"), &oc, &or_);
    }

    // --- bufferless
    {
        let (dbeg_c, dbeg_r) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_decompressBegin");
        let (nsz_c, nsz_r) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_nextSrcSizeToDecompress");
        let (nit_c, nit_r) = duo::<unsafe extern "C" fn(*const c_void) -> c_int>("ZSTD_nextInputType");
        let (cont_c, cont_r) = duo::<FnDecompressDCtx>("ZSTD_decompressContinue");
        let d = static_dctx();
        eqcode(&format!("{tag} decompressBegin"), dbeg_c(d.c), dbeg_r(d.r));
        let mut pos = 0usize;
        let mut oc = vec![0x5Cu8; cap];
        let mut or_ = vec![0x5Cu8; cap];
        let mut opos = 0usize;
        let mut step = 0;
        loop {
            step += 1;
            let want_c = nsz_c(d.c);
            let want_r = nsz_r(d.r);
            eqv(&format!("{tag} nextSrcSizeToDecompress step={step}"), want_c, want_r);
            eqv(
                &format!("{tag} nextInputType step={step}"),
                nit_c(d.c),
                nit_r(d.r),
            );
            if want_c == 0 || step > 3000 {
                break;
            }
            if pos + want_c > n {
                break; // truncated input; both agree via nextSrcSize above
            }
            let avail_out = cap - opos;
            let a = cont_c(
                d.c,
                oc.as_mut_ptr().add(opos) as *mut c_void,
                avail_out,
                frame[pos..].as_ptr() as *const c_void,
                want_c,
            );
            let b = cont_r(
                d.r,
                or_.as_mut_ptr().add(opos) as *mut c_void,
                avail_out,
                frame[pos..].as_ptr() as *const c_void,
                want_r,
            );
            eqcode(&format!("{tag} decompressContinue step={step}"), a, b);
            if is_err(a) {
                break;
            }
            opos += a;
            pos += want_c;
        }
        eqbuf(&format!("{tag} bufferless dst"), &oc, &or_);
    }
}

// ------------------------------------------------------------------ prefix_unknown / version_unsupported

#[test]
fn err_prefix_unknown_and_version() {
    unsafe {
        let mut rng = Rng::new(0xC001);
        // every 4-byte magic value we can think of, plus random garbage
        let mut magics: Vec<u32> = vec![
            0,
            1,
            0xFFFFFFFF,
            ZSTD_MAGICNUMBER,
            ZSTD_MAGICNUMBER ^ 1,
            ZSTD_MAGICNUMBER + 1,
            ZSTD_MAGICNUMBER - 1,
            ZSTD_MAGIC_DICTIONARY,
        ];
        magics.extend(LEGACY_MAGICS);
        for v in 0..17u32 {
            magics.push(ZSTD_MAGIC_SKIPPABLE_START + v);
        }
        for _ in 0..300 {
            magics.push(rng.next_u32());
        }
        for m in magics {
            for extra in [0usize, 1, 3, 4, 8, 20, 64] {
                let mut buf = m.to_le_bytes().to_vec();
                buf.extend(rng.bytes(extra));
                diff_decode_all(&format!("magic={m:#x} extra={extra}"), &buf, 4096);
            }
        }
        // empty / 1..3 byte inputs
        for n in 0..4usize {
            let buf = rng.bytes(n);
            diff_decode_all(&format!("short len={n}"), &buf, 64);
        }
    }
}

// ------------------------------------------------------------------ truncation

#[test]
fn err_truncated_frames() {
    unsafe {
        let mut rng = Rng::new(0xC002);
        for i in 0..25 {
            let sz = rng.below(40_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            let cctx = CtxPair::cctx();
            let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
            let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
            let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
            for cks in [0, 1] {
                for csf in [0, 1] {
                    sp(cctx.c, ZSTD_c_checksumFlag, cks);
                    sp(cctx.c, ZSTD_c_contentSizeFlag, csf);
                    sp(cctx.c, ZSTD_c_compressionLevel, rng.range(1, 9));
                    let cap = bd(sz) + 64;
                    let mut buf = vec![0u8; cap];
                    let n = c2(
                        cctx.c,
                        buf.as_mut_ptr() as *mut c_void,
                        cap,
                        src.as_ptr() as *const c_void,
                        sz,
                    );
                    assert!(!is_err(n));
                    let frame = &buf[..n];
                    // every truncation point, sampled
                    let mut cuts: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 18];
                    for k in 1..12 {
                        cuts.push(n * k / 12);
                    }
                    cuts.push(n.saturating_sub(1));
                    cuts.push(n);
                    for cut in cuts {
                        let cut = cut.min(n);
                        diff_decode_all(
                            &format!("trunc i={i} cks={cks} csf={csf} cut={cut}/{n}"),
                            &frame[..cut],
                            sz + 16,
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ checksum_wrong

#[test]
fn err_checksum_wrong() {
    unsafe {
        let cctx = CtxPair::cctx();
        let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dsp_c, dsp_r) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (ddc, ddr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let mut rng = Rng::new(0xC003);
        for i in 0..40 {
            let sz = 1 + rng.below(30_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            sp(cctx.c, ZSTD_c_checksumFlag, 1);
            sp(cctx.c, ZSTD_c_compressionLevel, rng.range(1, 12));
            let cap = bd(sz) + 64;
            let mut buf = vec![0u8; cap];
            let n = c2(
                cctx.c,
                buf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            assert!(!is_err(n));
            // flip every bit of the 4-byte checksum trailer
            for byte in 0..4usize {
                for bit in 0..8u32 {
                    let mut f = buf[..n].to_vec();
                    let idx = n - 4 + byte;
                    f[idx] ^= 1 << bit;
                    // with checksum validation on, and with it forced off
                    for ic in [ZSTD_d_validateChecksum(), ZSTD_d_ignoreChecksum()] {
                        let d = CtxPair::dctx();
                        eqcode(
                            "err_checksum setParameter",
                            dsp_c(d.c, ZSTD_d_forceIgnoreChecksum, ic),
                            dsp_r(d.r, ZSTD_d_forceIgnoreChecksum, ic),
                        );
                        let mut oc = vec![0u8; sz + 8];
                        let mut or_ = vec![0u8; sz + 8];
                        let a = ddc(
                            d.c,
                            oc.as_mut_ptr() as *mut c_void,
                            oc.len(),
                            f.as_ptr() as *const c_void,
                            n,
                        );
                        let b = ddr(
                            d.r,
                            or_.as_mut_ptr() as *mut c_void,
                            or_.len(),
                            f.as_ptr() as *const c_void,
                            n,
                        );
                        eqcode(
                            &format!("checksum i={i} byte={byte} bit={bit} ignore={ic}"),
                            a,
                            b,
                        );
                        eqbuf(
                            &format!("checksum i={i} byte={byte} bit={bit} ignore={ic} dst"),
                            &oc,
                            &or_,
                        );
                    }
                }
            }
        }
    }
}

fn ZSTD_d_validateChecksum() -> c_int {
    0
}
fn ZSTD_d_ignoreChecksum() -> c_int {
    1
}

// ------------------------------------------------------------------ dstSize_tooSmall

#[test]
fn err_dst_too_small() {
    unsafe {
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xC004);
        for i in 0..60 {
            let sz = 1 + rng.below(20_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            let frame = c_compress(&src, rng.range(1, 12));
            for cap in [
                0usize,
                1,
                sz / 4,
                sz / 2,
                sz - 1,
                sz,
                sz + 1,
            ] {
                let mut oc = vec![0x99u8; cap.max(1)];
                let mut or_ = vec![0x99u8; cap.max(1)];
                let a = dc(
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                let b = dr(
                    or_.as_mut_ptr() as *mut c_void,
                    cap,
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                eqcode(&format!("dstTooSmall i={i} cap={cap} sz={sz}"), a, b);
                eqbuf(&format!("dstTooSmall i={i} cap={cap} dst"), &oc, &or_);
            }
            // NULL dst with non-zero capacity (ZSTD_error_dstBuffer_null)
            let a = dc(
                std::ptr::null_mut(),
                sz,
                frame.as_ptr() as *const c_void,
                frame.len(),
            );
            let b = dr(
                std::ptr::null_mut(),
                sz,
                frame.as_ptr() as *const c_void,
                frame.len(),
            );
            eqcode(&format!("dstBuffer_null i={i}"), a, b);
        }
    }
}

// ------------------------------------------------------------------ windowTooLarge

#[test]
fn err_window_too_large() {
    unsafe {
        let cctx = CtxPair::cctx();
        let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dsp_c, dsp_r) = duo::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (mwc, mwr) =
            duo::<unsafe extern "C" fn(*mut c_void, usize) -> usize>("ZSTD_DCtx_setMaxWindowSize");
        let (dsc, dsr) = duo::<FnDStream>("ZSTD_decompressStream");
        let (idc, idr) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_initDStream");
        let sz = 400_000usize;
        let src = gen_class(4, sz, 5);
        for wl in [10, 15, 20, 23, 27] {
            sp(cctx.c, ZSTD_c_windowLog, wl);
            sp(cctx.c, ZSTD_c_compressionLevel, 3);
            let cap = bd(sz) + 64;
            let mut buf = vec![0u8; cap];
            let n = c2(
                cctx.c,
                buf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            assert!(!is_err(n));
            for dwlm in [0, 10, 11, 15, 16, 20, 21, 23, 24, 27, 31] {
                let d = CtxPair::dstream();
                eqcode(
                    &format!("wl={wl} dwlm={dwlm} setParameter"),
                    dsp_c(d.c, ZSTD_d_windowLogMax, dwlm),
                    dsp_r(d.r, ZSTD_d_windowLogMax, dwlm),
                );
                eqcode(&format!("wl={wl} dwlm={dwlm} initDStream"), idc(d.c), idr(d.r));
                let mut oc = vec![0u8; sz + 8];
                let mut or_ = vec![0u8; sz + 8];
                let mut ic = ZSTD_inBuffer {
                    src: buf.as_ptr() as *const c_void,
                    size: n,
                    pos: 0,
                };
                let mut ir = ic;
                let mut obc = ZSTD_outBuffer {
                    dst: oc.as_mut_ptr() as *mut c_void,
                    size: oc.len(),
                    pos: 0,
                };
                let mut obr = ZSTD_outBuffer {
                    dst: or_.as_mut_ptr() as *mut c_void,
                    size: or_.len(),
                    pos: 0,
                };
                let mut step = 0;
                loop {
                    step += 1;
                    let a = dsc(d.c, &mut obc, &mut ic);
                    let b = dsr(d.r, &mut obr, &mut ir);
                    eqcode(&format!("wl={wl} dwlm={dwlm} step={step} stream"), a, b);
                    if is_err(a) || a == 0 || step > 2000 {
                        break;
                    }
                }
                eqbuf(&format!("wl={wl} dwlm={dwlm} dst"), &oc, &or_);
            }
            // and the byte-size setter
            for ws in [0usize, 1, 1 << 9, 1 << 10, 1 << 15, 1 << 20, 1 << 27, usize::MAX] {
                let d = CtxPair::dctx();
                eqcode(
                    &format!("setMaxWindowSize({ws})"),
                    mwc(d.c, ws),
                    mwr(d.r, ws),
                );
            }
        }
    }
}

// ------------------------------------------------------------------ dictionary_wrong / dictionary_corrupted

#[test]
fn err_dictionary_wrong_and_corrupted() {
    unsafe {
        let (cuc, _) = duo::<
            unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                usize,
                *const c_void,
                usize,
                *const c_void,
                usize,
                c_int,
            ) -> usize,
        >("ZSTD_compress_usingDict");
        let (duc, dur) = duo::<
            unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                usize,
                *const c_void,
                usize,
                *const c_void,
                usize,
            ) -> usize,
        >("ZSTD_decompress_usingDict");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();
        let mut rng = Rng::new(0xC005);

        // a "real" dictionary produced by the C dictionary builder
        let (train, _) = duo::<
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize,
        >("ZDICT_trainFromBuffer");
        let nb = 64usize;
        let each = 2048usize;
        let mut corpus = Vec::new();
        let mut sizes = Vec::new();
        for k in 0..nb {
            corpus.extend_from_slice(&gen_class(4, each, k as u64));
            sizes.push(each);
        }
        let mut dictbuf = vec![0u8; 16 * 1024];
        let dn = train(
            dictbuf.as_mut_ptr() as *mut c_void,
            dictbuf.len(),
            corpus.as_ptr() as *const c_void,
            sizes.as_ptr(),
            nb as c_uint,
        );
        let real_dict: Vec<u8> = if is_err(dn) {
            Vec::new()
        } else {
            dictbuf[..dn].to_vec()
        };

        for i in 0..30 {
            let sz = 1 + rng.below(20_000);
            let src = gen_class(rng.below(N_CLASSES), sz, i);
            let dicts: Vec<Vec<u8>> = vec![
                Vec::new(),
                gen_class(3, 1, i),
                gen_class(3, 7, i),
                gen_class(3, 64, i),
                gen_class(4, 4096, i),
                real_dict.clone(),
            ];
            for (di, d) in dicts.iter().enumerate() {
                let (dp, ds) = if d.is_empty() {
                    (std::ptr::null(), 0usize)
                } else {
                    (d.as_ptr() as *const c_void, d.len())
                };
                let cap = bd(sz) + 64;
                let mut buf = vec![0u8; cap];
                let n = cuc(
                    cctx.c,
                    buf.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    dp,
                    ds,
                    3,
                );
                if is_err(n) {
                    continue;
                }
                // decode with EVERY dictionary (including the wrong one and none)
                for (dj, d2) in dicts.iter().enumerate() {
                    let (dp2, ds2) = if d2.is_empty() {
                        (std::ptr::null(), 0usize)
                    } else {
                        (d2.as_ptr() as *const c_void, d2.len())
                    };
                    let mut oc = vec![0u8; sz + 8];
                    let mut or_ = vec![0u8; sz + 8];
                    let a = duc(
                        dctx.c,
                        oc.as_mut_ptr() as *mut c_void,
                        oc.len(),
                        buf.as_ptr() as *const c_void,
                        n,
                        dp2,
                        ds2,
                    );
                    let b = dur(
                        dctx.r,
                        or_.as_mut_ptr() as *mut c_void,
                        or_.len(),
                        buf.as_ptr() as *const c_void,
                        n,
                        dp2,
                        ds2,
                    );
                    eqcode(&format!("dict i={i} enc={di} dec={dj}"), a, b);
                    eqbuf(&format!("dict i={i} enc={di} dec={dj} dst"), &oc, &or_);
                }
            }
        }

        // corrupted dictionaries: right magic, garbage payload
        for i in 0..200 {
            let mut d = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
            let extra = rng.below(400);
            d.extend(rng.bytes(extra));
            let src = gen_class(4, 1000, i);
            let cap = bd(1000) + 64;
            let mut buf = vec![0u8; cap];
            let a = cuc(
                cctx.c,
                buf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                1000,
                d.as_ptr() as *const c_void,
                d.len(),
                3,
            );
            let mut buf2 = vec![0u8; cap];
            let (cur, _) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                    usize,
                    c_int,
                ) -> usize,
            >("ZSTD_compress_usingDict");
            let _ = cur;
            let (_, cru) = duo::<
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *const c_void,
                    usize,
                    *const c_void,
                    usize,
                    c_int,
                ) -> usize,
            >("ZSTD_compress_usingDict");
            let b = cru(
                cctx.r,
                buf2.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                1000,
                d.as_ptr() as *const c_void,
                d.len(),
                3,
            );
            eqcode(&format!("corrupt dict compress i={i} len={}", d.len()), a, b);
            eqbuf(&format!("corrupt dict compress i={i} dst"), &buf, &buf2);
            // decode side
            let mut oc = vec![0u8; 1200];
            let mut or_ = vec![0u8; 1200];
            let frame = c_compress(&src, 3);
            let x = duc(
                dctx.c,
                oc.as_mut_ptr() as *mut c_void,
                oc.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                d.as_ptr() as *const c_void,
                d.len(),
            );
            let y = dur(
                dctx.r,
                or_.as_mut_ptr() as *mut c_void,
                or_.len(),
                frame.as_ptr() as *const c_void,
                frame.len(),
                d.as_ptr() as *const c_void,
                d.len(),
            );
            eqcode(&format!("corrupt dict decompress i={i}"), x, y);
            eqbuf(&format!("corrupt dict decompress i={i} dst"), &oc, &or_);
            // and via createDDict
            let (cdc, cdr) = duo::<
                unsafe extern "C" fn(*const c_void, usize) -> *mut c_void,
            >("ZSTD_createDDict");
            let (fdc, fdr) = duo::<FnFreePtr>("ZSTD_freeDDict");
            let pc = cdc(d.as_ptr() as *const c_void, d.len());
            let pr = cdr(d.as_ptr() as *const c_void, d.len());
            eqv(&format!("createDDict i={i} null?"), pc.is_null(), pr.is_null());
            if !pc.is_null() {
                let (idc, idr) =
                    duo::<unsafe extern "C" fn(*const c_void) -> c_uint>("ZSTD_getDictID_fromDDict");
                eqv(&format!("getDictID_fromDDict i={i}"), idc(pc), idr(pr));
                fdc(pc);
                fdr(pr);
            }
            let (gdc, gdr) = duo::<FnUFromBuf>("ZSTD_getDictID_fromDict");
            eqv(
                &format!("getDictID_fromDict i={i}"),
                gdc(d.as_ptr() as *const c_void, d.len()),
                gdr(d.as_ptr() as *const c_void, d.len()),
            );
        }
    }
}

// ------------------------------------------------------------------ corruption fuzz

#[test]
fn err_corruption_fuzz() {
    unsafe {
        let mut rng = Rng::new(0xC006);
        let cctx = CtxPair::cctx();
        let (sp, _) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = duo::<FnCompress2>("ZSTD_compress2");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (rst, _) = duo::<FnReset>("ZSTD_CCtx_reset");

        for round in 0..90 {
            let sz = 1 + rng.below(30_000);
            let src = gen_class(rng.below(N_CLASSES), sz, round);
            rst(cctx.c, ZSTD_reset_session_and_parameters);
            sp(cctx.c, ZSTD_c_compressionLevel, rng.range(-3, 19));
            sp(cctx.c, ZSTD_c_checksumFlag, rng.range(0, 1));
            sp(cctx.c, ZSTD_c_contentSizeFlag, rng.range(0, 1));
            sp(cctx.c, ZSTD_c_strategy, rng.range(1, 9));
            sp(cctx.c, ZSTD_c_targetCBlockSize, if rng.below(3) == 0 { 1340 } else { 0 });
            let cap = bd(sz) + 64;
            let mut buf = vec![0u8; cap];
            let n = c2(
                cctx.c,
                buf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                sz,
            );
            assert!(!is_err(n));
            let frame = buf[..n].to_vec();

            // 12 mutations per frame: single-bit flips, byte substitutions,
            // truncations, splices and header rewrites
            for m in 0..12 {
                let mut f = frame.clone();
                match m % 6 {
                    0 => {
                        let i = rng.below(f.len());
                        f[i] ^= 1 << rng.below(8);
                    }
                    1 => {
                        let i = rng.below(f.len());
                        f[i] = rng.byte();
                    }
                    2 => {
                        let k = 1 + rng.below(8.min(f.len()));
                        let i = rng.below(f.len() - k + 1);
                        for j in 0..k {
                            f[i + j] = rng.byte();
                        }
                    }
                    3 => {
                        let cut = rng.below(f.len() + 1);
                        f.truncate(cut);
                    }
                    4 => {
                        let extra = 1 + rng.below(16);
                        let tail = rng.bytes(extra);
                        f.extend_from_slice(&tail);
                    }
                    _ => {
                        // rewrite the frame-header byte
                        if f.len() > 4 {
                            f[4] = rng.byte();
                        }
                    }
                }
                diff_decode_all(&format!("fuzz round={round} m={m}"), &f, sz + 32);
            }
        }
    }
}

// ------------------------------------------------------------------ block-level errors

#[test]
fn err_block_level_getcblocksize() {
    unsafe {
        let (gcb_c, gcb_r) = duo::<
            unsafe extern "C" fn(*const c_void, usize, *mut ZSTD_blockProperties) -> usize,
        >("ZSTD_getcBlockSize");
        let (dbl_c, dbl_r) = duo::<FnDecompressDCtx>("ZSTD_decompressBlock");
        let (dbd_c, dbd_r) = duo::<FnDecompressDCtx>("ZSTD_decompressBlock_deprecated");
        let (dbeg_c, dbeg_r) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_decompressBegin");
        // size_t ZSTD_decodeSeqHeaders(ZSTD_DCtx*, int* nbSeqPtr, const void*, size_t)
        let (dsh_c, dsh_r) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_int, *const c_void, usize) -> usize,
        >("ZSTD_decodeSeqHeaders");
        let mut rng = Rng::new(0xC007);

        // ZSTD_getcBlockSize over every 3-byte block header, including the
        // reserved block type (3) which must be rejected, and every truncated
        // length. Signature is
        //   size_t ZSTD_getcBlockSize(const void*, size_t, blockProperties_t*)
        // (zstd_internal.h:306) - the out-param must be compared too.
        for hi in 0..256u32 {
            for _ in 0..8 {
                let b = vec![(hi & 0xFF) as u8, rng.byte(), rng.byte()];
                for l in [0usize, 1, 2, 3] {
                    let s = &b[..l.min(b.len())];
                    let mut bpc = ZSTD_blockProperties { blockType: 7, lastBlock: 0xAB, origSize: 0xCD };
                    let mut bpr = bpc;
                    let a = gcb_c(s.as_ptr() as *const c_void, s.len(), &mut bpc);
                    let b2 = gcb_r(s.as_ptr() as *const c_void, s.len(), &mut bpr);
                    eqcode(&format!("getcBlockSize hi={hi} len={l}"), a, b2);
                    eqv(&format!("getcBlockSize hi={hi} len={l} props"), bpc, bpr);
                }
            }
        }
    }
}

#[test]
fn err_block_level_decompressblock() {
    unsafe {
        let (dbl_c, dbl_r) = duo::<FnDecompressDCtx>("ZSTD_decompressBlock");
        let (dbd_c, dbd_r) = duo::<FnDecompressDCtx>("ZSTD_decompressBlock_deprecated");
        let (dbeg_c, dbeg_r) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_decompressBegin");
        let mut rng = Rng::new(0xC008);
        // NOTE: zstd.h documents ZSTD_decompressBlock() as *not* protected
        // against malicious input; fully random payloads are UB in the C.
        for i in 0..400 {
            let d = CtxPair::dctx();
            eqcode("decompressBegin", dbeg_c(d.c), dbeg_r(d.r));
            let n = rng.below(400);
            let payload = if rng.below(2) == 0 {
                rng.bytes(n)
            } else {
                gen_class(rng.below(N_CLASSES), n, i)
            };
            let cap = ZSTD_BLOCKSIZE_MAX;
            let mut oc = vec![0x22u8; cap];
            let mut or_ = vec![0x22u8; cap];
            let a = dbl_c(
                d.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                payload.as_ptr() as *const c_void,
                payload.len(),
            );
            let b = dbl_r(
                d.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                payload.as_ptr() as *const c_void,
                payload.len(),
            );
            eqcode(&format!("decompressBlock i={i} len={n}"), a, b);
            eqbuf(&format!("decompressBlock i={i} dst"), &oc, &or_);

            let d2 = CtxPair::dctx();
            eqcode("decompressBegin(dep)", dbeg_c(d2.c), dbeg_r(d2.r));
            let mut oc = vec![0x22u8; cap];
            let mut or_ = vec![0x22u8; cap];
            let a = dbd_c(
                d2.c,
                oc.as_mut_ptr() as *mut c_void,
                cap,
                payload.as_ptr() as *const c_void,
                payload.len(),
            );
            let b = dbd_r(
                d2.r,
                or_.as_mut_ptr() as *mut c_void,
                cap,
                payload.as_ptr() as *const c_void,
                payload.len(),
            );
            eqcode(&format!("decompressBlock_deprecated i={i} len={n}"), a, b);
            eqbuf(&format!("decompressBlock_deprecated i={i} dst"), &oc, &or_);

            // over-sized block
            let extra = rng.below(16);
            let big = rng.bytes(ZSTD_BLOCKSIZE_MAX + 1 + extra);
            let d3 = CtxPair::dctx();
            dbeg_c(d3.c);
            dbeg_r(d3.r);
            let mut oc = vec![0u8; 64];
            let mut or_ = vec![0u8; 64];
            let a = dbl_c(
                d3.c,
                oc.as_mut_ptr() as *mut c_void,
                oc.len(),
                big.as_ptr() as *const c_void,
                big.len(),
            );
            let b = dbl_r(
                d3.r,
                or_.as_mut_ptr() as *mut c_void,
                or_.len(),
                big.as_ptr() as *const c_void,
                big.len(),
            );
            eqcode(&format!("decompressBlock oversized i={i}"), a, b);
        }

    }
}

#[test]
fn err_block_level_seqheaders() {
    unsafe {
        let (dbeg_c, dbeg_r) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_decompressBegin");
        // size_t ZSTD_decodeSeqHeaders(ZSTD_DCtx*, int* nbSeqPtr, const void*, size_t)
        let (dsh_c, dsh_r) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_int, *const c_void, usize) -> usize,
        >("ZSTD_decodeSeqHeaders");
        let mut rng = Rng::new(0xC009);
        // ZSTD_decodeSeqHeaders on random payloads
        for i in 0..600 {
            let d = CtxPair::dctx();
            eqcode("decompressBegin(seq)", dbeg_c(d.c), dbeg_r(d.r));
            let n = rng.below(64);
            let payload = rng.bytes(n);
            let mut nc: c_int = -1;
            let mut nr: c_int = -1;
            let a = dsh_c(d.c, &mut nc, payload.as_ptr() as *const c_void, payload.len());
            let b = dsh_r(d.r, &mut nr, payload.as_ptr() as *const c_void, payload.len());
            eqcode(&format!("decodeSeqHeaders i={i} len={n}"), a, b);
            eqv(&format!("decodeSeqHeaders i={i} nbSeq"), nc, nr);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ZSTD_blockProperties {
    blockType: c_int,
    lastBlock: c_uint,
    origSize: c_uint,
}
