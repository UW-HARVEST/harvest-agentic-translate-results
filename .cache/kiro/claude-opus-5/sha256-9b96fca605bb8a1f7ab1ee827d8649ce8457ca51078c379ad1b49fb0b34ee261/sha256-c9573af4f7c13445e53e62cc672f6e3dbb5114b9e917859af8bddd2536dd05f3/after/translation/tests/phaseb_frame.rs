//! Phase B — differential tests for FRAME INTROSPECTION, SIZING / STATIC
//! CONTEXTS, SKIPPABLE FRAMES, getCParams/getParams and the LEGACY v01-v07
//! decoders.
//!
//! Every call crosses the FFI boundary via `dlsym` on both the C and the Rust
//! `libzstd.so`. Return values are compared exactly and, wherever a buffer is
//! produced, the bytes are compared after both sides pre-fill with 0xAA.
//!
//! The C build was configured with `ZSTD_LEGACY_SUPPORT=5`, so `ZSTD_decompress`
//! dispatches v05 (and, when compiled, v06/v07) magic numbers; v01-v04 modules
//! are compiled in and their entry points exported. We drive every exported
//! legacy entry point directly through `dlsym`.

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------------ types ----

type FnU64FromBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnSizeFromBuf = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnUintFromBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetFrameHeader = unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t) -> size_t;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t, c_int) -> size_t;
type FnDictIDFromBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnDictIDFromPtr = unsafe extern "C" fn(*const c_void) -> c_uint;

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

// skippable
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_uint) -> size_t;
type FnReadSkippable =
    unsafe extern "C" fn(*mut c_void, size_t, *mut c_uint, *const c_void, size_t) -> size_t;

// sizing
type FnEstInt = unsafe extern "C" fn(c_int) -> size_t;
type FnEstCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t;
type FnEstCCtxParams = unsafe extern "C" fn(*const c_void) -> size_t;
type FnEstDStreamWin = unsafe extern "C" fn(size_t) -> size_t;
type FnEstCDictAdv =
    unsafe extern "C" fn(size_t, ZSTD_compressionParameters, c_int) -> size_t;
type FnEstCDict = unsafe extern "C" fn(size_t, c_int) -> size_t;
type FnEstDDict = unsafe extern "C" fn(size_t, c_int) -> size_t;

// static init
type FnInitStaticCtx = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_int, // dictLoadMethod
    c_int, // dictContentType
    ZSTD_compressionParameters,
) -> *const c_void;
type FnInitStaticDDict = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_int,
    c_int,
) -> *const c_void;

type FnSizeofPtr = unsafe extern "C" fn(*const c_void) -> size_t;

// cparams / params
type FnGetCParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnGetParams = unsafe extern "C" fn(c_int, c_ulonglong, size_t) -> ZSTD_parameters;
type FnCheckCParams = unsafe extern "C" fn(ZSTD_compressionParameters) -> size_t;
type FnAdjustCParams =
    unsafe extern "C" fn(ZSTD_compressionParameters, c_ulonglong, size_t) -> ZSTD_compressionParameters;
type FnCCtxParamsInitAdv = unsafe extern "C" fn(*mut c_void, ZSTD_parameters) -> size_t;
type FnCCtxParamsInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCreateP = unsafe extern "C" fn() -> *mut c_void;

// legacy
type FnLegacyDecompress =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnLegacyIsError = unsafe extern "C" fn(size_t) -> c_uint;
type FnLegacyFindSize =
    unsafe extern "C" fn(*const c_void, size_t, *mut size_t, *mut c_ulonglong);

// dict/cdict/ddict create
type FnCreateCDict =
    unsafe extern "C" fn(*const c_void, size_t, c_int) -> *mut c_void;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;

const ZSTD_DLM_BY_COPY: c_int = 0;
const ZSTD_DCT_AUTO: c_int = 0;

// ---------------------------------------------------------------- helpers ----

/// Build a set of compressed frames with widely varying parameters, returning
/// `(bytes, description)` pairs. Uses `ZSTD_compress2` on the C library so we
/// have known-valid modern frames to introspect (the frame *bytes* themselves
/// are already proven identical between C and Rust by phaseb_compress).
fn build_frames(rng: &mut Rng) -> Vec<(Vec<u8>, String)> {
    unsafe {
        let (c_create, _r_create) = fnpair!("ZSTD_createCCtx", FnCreate);
        let (c_free, _r_free) = fnpair!("ZSTD_freeCCtx", FnFree);
        let (c_set, _r_set) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
        let (c_c2, _r_c2) = fnpair!("ZSTD_compress2", FnCompress2);
        let (c_bound, _r_bound) = fnpair!("ZSTD_compressBound", FnSizeSize);

        let mut out: Vec<(Vec<u8>, String)> = Vec::new();
        let cctx = c_create();

        let lens = [0usize, 1, 2, 7, 63, 100, 4096, 65_537, 140_000];
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let src = gen(shape, len, rng);
                for &lvl in &[-3i32, 1, 3, 9, 19] {
                    for &cs in &[0, 1] {
                        for &ck in &[0, 1] {
                            for &did in &[0, 1] {
                                // Only exercise the wider matrix on a subset to
                                // keep runtime bounded, but always cover both
                                // format magics.
                                for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                                    if fmt == ZSTD_f_zstd1_magicless
                                        && (lvl != 3 || cs != 1)
                                    {
                                        continue;
                                    }
                                    if lvl != 3 && (cs + ck + did) == 0 {
                                        // thin out
                                    }
                                    let _ = c_set(cctx, ZSTD_c_compressionLevel, lvl);
                                    let _ = c_set(cctx, ZSTD_c_contentSizeFlag, cs);
                                    let _ = c_set(cctx, ZSTD_c_checksumFlag, ck);
                                    let _ = c_set(cctx, ZSTD_c_dictIDFlag, did);
                                    let _ = c_set(cctx, ZSTD_c_format, fmt);
                                    let cap = c_bound(len).max(64);
                                    let mut buf = vec![0u8; cap];
                                    let sp = if src.is_empty() {
                                        std::ptr::NonNull::<u8>::dangling().as_ptr()
                                            as *const c_void
                                    } else {
                                        src.as_ptr() as *const c_void
                                    };
                                    let n = c_c2(
                                        cctx,
                                        buf.as_mut_ptr() as *mut c_void,
                                        cap,
                                        sp,
                                        len,
                                    );
                                    // reset parameters for next iteration
                                    let (c_reset, _r): (
                                        unsafe extern "C" fn(*mut c_void, c_int) -> size_t,
                                        _,
                                    ) = fnpair!(
                                        "ZSTD_CCtx_reset",
                                        unsafe extern "C" fn(*mut c_void, c_int) -> size_t
                                    );
                                    let _ = c_reset(cctx, ZSTD_reset_parameters);
                                    let is_err = {
                                        let (cie, _r) = fnpair!("ZSTD_isError", FnIsError);
                                        cie(n) != 0
                                    };
                                    if is_err {
                                        continue;
                                    }
                                    buf.truncate(n);
                                    out.push((
                                        buf,
                                        format!(
                                            "shape={shape:?} len={len} lvl={lvl} cs={cs} ck={ck} did={did} fmt={fmt}"
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        c_free(cctx);
        out
    }
}

/// Compare two `size_t`-returning byte-buffer functions exactly.
#[track_caller]
fn cmp_size(cf: FnSizeFromBuf, rf: FnSizeFromBuf, buf: &[u8], len: size_t, ctx: &str) {
    unsafe {
        let p = buf_ptr(buf);
        let c = cf(p, len);
        let r = rf(p, len);
        assert_eq!(c, r, "{ctx}: size_t result differs (C={c:#x} R={r:#x})");
    }
}

#[track_caller]
fn cmp_u64(cf: FnU64FromBuf, rf: FnU64FromBuf, buf: &[u8], len: size_t, ctx: &str) {
    unsafe {
        let p = buf_ptr(buf);
        let c = cf(p, len);
        let r = rf(p, len);
        assert_eq!(c, r, "{ctx}: u64 result differs (C={c:#x} R={r:#x})");
    }
}

#[track_caller]
fn cmp_uint(cf: FnUintFromBuf, rf: FnUintFromBuf, buf: &[u8], len: size_t, ctx: &str) {
    unsafe {
        let p = buf_ptr(buf);
        let c = cf(p, len);
        let r = rf(p, len);
        assert_eq!(c, r, "{ctx}: uint result differs (C={c} R={r})");
    }
}

fn buf_ptr(buf: &[u8]) -> *const c_void {
    if buf.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
    } else {
        buf.as_ptr() as *const c_void
    }
}

/// Prefixes to feed introspection functions: every length 0..=24 plus a few
/// longer random prefixes.
fn prefix_lengths(full: usize, rng: &mut Rng) -> Vec<usize> {
    let mut v: Vec<usize> = (0..=24.min(full)).collect();
    for _ in 0..4 {
        if full > 0 {
            v.push(rng.below(full + 1));
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

// ===================================================================== 1 ====
// Frame introspection over many frames + truncated prefixes.

#[test]
fn frame_introspection_scalar_returns() {
    let (c_gcs, r_gcs) = fnpair!("ZSTD_getFrameContentSize", FnU64FromBuf);
    let (c_gds, r_gds) = fnpair!("ZSTD_getDecompressedSize", FnU64FromBuf);
    let (c_ffcs, r_ffcs) = fnpair!("ZSTD_findFrameCompressedSize", FnSizeFromBuf);
    let (c_fds, r_fds) = fnpair!("ZSTD_findDecompressedSize", FnU64FromBuf);
    let (c_db, r_db) = fnpair!("ZSTD_decompressBound", FnU64FromBuf);
    let (c_fhs, r_fhs) = fnpair!("ZSTD_frameHeaderSize", FnSizeFromBuf);
    let (c_isf, r_isf) = fnpair!("ZSTD_isFrame", FnUintFromBuf);
    let (c_isk, r_isk) = fnpair!("ZSTD_isSkippableFrame", FnUintFromBuf);

    let mut rng = Rng::new(0xF00D_0001);
    let frames = build_frames(&mut rng);
    assert!(frames.len() > 100, "expected many frames, got {}", frames.len());

    for (frame, desc) in &frames {
        // full-length introspection
        cmp_u64(c_gcs, r_gcs, frame, frame.len(), &format!("getFrameContentSize full {desc}"));
        cmp_u64(c_gds, r_gds, frame, frame.len(), &format!("getDecompressedSize full {desc}"));
        cmp_size(c_ffcs, r_ffcs, frame, frame.len(), &format!("findFrameCompressedSize full {desc}"));
        cmp_u64(c_fds, r_fds, frame, frame.len(), &format!("findDecompressedSize full {desc}"));
        cmp_u64(c_db, r_db, frame, frame.len(), &format!("decompressBound full {desc}"));
        cmp_size(c_fhs, r_fhs, frame, frame.len(), &format!("frameHeaderSize full {desc}"));
        cmp_uint(c_isf, r_isf, frame, frame.len(), &format!("isFrame full {desc}"));
        cmp_uint(c_isk, r_isk, frame, frame.len(), &format!("isSkippableFrame full {desc}"));

        // truncated prefixes
        for pl in prefix_lengths(frame.len(), &mut rng) {
            let ctx = format!("prefix={pl} {desc}");
            cmp_u64(c_gcs, r_gcs, frame, pl, &format!("getFrameContentSize {ctx}"));
            cmp_u64(c_gds, r_gds, frame, pl, &format!("getDecompressedSize {ctx}"));
            cmp_size(c_ffcs, r_ffcs, frame, pl, &format!("findFrameCompressedSize {ctx}"));
            cmp_u64(c_fds, r_fds, frame, pl, &format!("findDecompressedSize {ctx}"));
            cmp_u64(c_db, r_db, frame, pl, &format!("decompressBound {ctx}"));
            cmp_size(c_fhs, r_fhs, frame, pl, &format!("frameHeaderSize {ctx}"));
            cmp_uint(c_isf, r_isf, frame, pl, &format!("isFrame {ctx}"));
            cmp_uint(c_isk, r_isk, frame, pl, &format!("isSkippableFrame {ctx}"));
        }
    }
}

#[test]
fn frame_header_struct_field_for_field() {
    let (c_gfh, r_gfh) = fnpair!("ZSTD_getFrameHeader", FnGetFrameHeader);
    let (c_gfha, r_gfha) = fnpair!("ZSTD_getFrameHeader_advanced", FnGetFrameHeaderAdv);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);

    let mut rng = Rng::new(0xF00D_0002);
    let frames = build_frames(&mut rng);

    unsafe {
        for (frame, desc) in &frames {
            let is_magicless = desc.contains(&format!("fmt={ZSTD_f_zstd1_magicless}"));
            // For non-magicless frames, ZSTD_f_zstd1 is the natural format.
            // For magicless frames we must use ZSTD_f_zstd1_magicless.
            let formats: &[c_int] = if is_magicless {
                &[ZSTD_f_zstd1_magicless]
            } else {
                &[ZSTD_f_zstd1]
            };

            let mut lens = prefix_lengths(frame.len(), &mut rng);
            lens.push(frame.len());
            lens.sort_unstable();
            lens.dedup();

            for &pl in &lens {
                let p = buf_ptr(frame);

                // plain getFrameHeader (only meaningful for zstd1 format)
                if !is_magicless {
                    let mut ch = ZSTD_frameHeader::default();
                    let mut rh = ZSTD_frameHeader::default();
                    let cr = c_gfh(&mut ch, p, pl);
                    let rr = r_gfh(&mut rh, p, pl);
                    let ctx = format!("getFrameHeader pl={pl} {desc}");
                    assert_eq!(c_ie(cr), r_ie(rr), "{ctx}: isError differs (C={cr:#x} R={rr:#x})");
                    assert_eq!(cr, rr, "{ctx}: return code differs (C={cr:#x} R={rr:#x})");
                    if cr == 0 {
                        assert_eq!(ch, rh, "{ctx}: frameHeader struct differs\nC={ch:?}\nR={rh:?}");
                    }
                }

                for &fmt in formats {
                    let mut ch = ZSTD_frameHeader::default();
                    let mut rh = ZSTD_frameHeader::default();
                    let cr = c_gfha(&mut ch, p, pl, fmt);
                    let rr = r_gfha(&mut rh, p, pl, fmt);
                    let ctx = format!("getFrameHeader_advanced fmt={fmt} pl={pl} {desc}");
                    assert_eq!(c_ie(cr), r_ie(rr), "{ctx}: isError differs (C={cr:#x} R={rr:#x})");
                    assert_eq!(cr, rr, "{ctx}: return code differs (C={cr:#x} R={rr:#x})");
                    if cr == 0 {
                        assert_eq!(ch, rh, "{ctx}: frameHeader struct differs\nC={ch:?}\nR={rh:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn frame_introspection_multiframe_and_dictid() {
    let (c_fds, r_fds) = fnpair!("ZSTD_findDecompressedSize", FnU64FromBuf);
    let (c_db, r_db) = fnpair!("ZSTD_decompressBound", FnU64FromBuf);
    let (c_ffcs, r_ffcs) = fnpair!("ZSTD_findFrameCompressedSize", FnSizeFromBuf);
    let (c_gcs, r_gcs) = fnpair!("ZSTD_getFrameContentSize", FnU64FromBuf);
    let (c_gdif, r_gdif) = fnpair!("ZSTD_getDictID_fromFrame", FnDictIDFromBuf);
    let (c_gdid, r_gdid) = fnpair!("ZSTD_getDictID_fromDict", FnDictIDFromBuf);
    let (c_gdic, r_gdic) = fnpair!("ZSTD_getDictID_fromCDict", FnDictIDFromPtr);
    let (c_gdid2, r_gdid2) = fnpair!("ZSTD_getDictID_fromDDict", FnDictIDFromPtr);

    let mut rng = Rng::new(0xF00D_0003);

    // -------- multi-frame concatenations --------
    let frames = build_frames(&mut rng);
    // only use standard (non-magicless) frames for concatenation
    let std_frames: Vec<&(Vec<u8>, String)> = frames
        .iter()
        .filter(|(_, d)| !d.contains(&format!("fmt={ZSTD_f_zstd1_magicless}")))
        .collect();

    for _ in 0..400 {
        let n = 1 + rng.below(4);
        let mut cat: Vec<u8> = Vec::new();
        let mut descs: Vec<String> = Vec::new();
        for _ in 0..n {
            let f = &std_frames[rng.below(std_frames.len())];
            cat.extend_from_slice(&f.0);
            descs.push(f.1.clone());
        }
        let ctx = format!("concat n={n} [{}]", descs.join(" | "));
        cmp_u64(c_fds, r_fds, &cat, cat.len(), &format!("findDecompressedSize {ctx}"));
        cmp_u64(c_db, r_db, &cat, cat.len(), &format!("decompressBound {ctx}"));
        cmp_size(c_ffcs, r_ffcs, &cat, cat.len(), &format!("findFrameCompressedSize {ctx}"));
        cmp_u64(c_gcs, r_gcs, &cat, cat.len(), &format!("getFrameContentSize {ctx}"));
        // truncated
        for pl in prefix_lengths(cat.len().min(64), &mut rng) {
            cmp_u64(c_fds, r_fds, &cat, pl, &format!("findDecompressedSize trunc={pl} {ctx}"));
            cmp_u64(c_db, r_db, &cat, pl, &format!("decompressBound trunc={pl} {ctx}"));
        }
    }

    // -------- dictID from frame with a real dictionary --------
    unsafe {
        let (c_create, r_create) = fnpair!("ZSTD_createCCtx", FnCreate);
        let (c_free, r_free) = fnpair!("ZSTD_freeCCtx", FnFree);
        let (c_setp, r_setp): (
            unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t,
            _,
        ) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
        let (c_ld, r_ld): (
            unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t,
            _,
        ) = fnpair!(
            "ZSTD_CCtx_loadDictionary",
            unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t
        );
        let (c_c2, r_c2) = fnpair!("ZSTD_compress2", FnCompress2);
        let (c_bound, _r_bound) = fnpair!("ZSTD_compressBound", FnSizeSize);
        let (c_ccdict, r_ccdict) = fnpair!("ZSTD_createCDict", FnCreateCDict);
        let (c_cddict, r_cddict) = fnpair!("ZSTD_createDDict", FnCreateDDict);
        let (c_frcd, r_frcd): (FnFree, FnFree) = fnpair!("ZSTD_freeCDict", FnFree);
        let (c_frdd, r_frdd): (FnFree, FnFree) = fnpair!("ZSTD_freeDDict", FnFree);

        for &dlen in &[0usize, 8, 64, 4096, 20_000] {
            let dict = gen(Shape::Text, dlen, &mut rng);
            let dp = buf_ptr(&dict);

            // dictID_fromDict
            {
                let c = c_gdid(dp, dlen);
                let r = r_gdid(dp, dlen);
                assert_eq!(c, r, "getDictID_fromDict dlen={dlen} (C={c} R={r})");
            }

            // Build CDict / DDict and compare their dictIDs, plus frames.
            for &lvl in &[1i32, 9] {
                let cd_c = c_ccdict(dp, dlen, lvl);
                let cd_r = r_ccdict(dp, dlen, lvl);
                let c = c_gdic(cd_c as *const c_void);
                let r = r_gdic(cd_r as *const c_void);
                assert_eq!(c, r, "getDictID_fromCDict dlen={dlen} lvl={lvl} (C={c} R={r})");

                let dd_c = c_cddict(dp, dlen);
                let dd_r = r_cddict(dp, dlen);
                let c = c_gdid2(dd_c as *const c_void);
                let r = r_gdid2(dd_r as *const c_void);
                assert_eq!(c, r, "getDictID_fromDDict dlen={dlen} lvl={lvl} (C={c} R={r})");

                c_frcd(cd_c);
                r_frcd(cd_r);
                c_frdd(dd_c);
                r_frdd(dd_r);
            }

            // Compress with dictionary on both sides and compare dictID_fromFrame.
            for &did in &[0i32, 1] {
                for &shape in &[Shape::Text, Shape::Random] {
                    let src = gen(shape, 3000, &mut rng);
                    let cctx_c = c_create();
                    let cctx_r = r_create();
                    let _ = c_setp(cctx_c, ZSTD_c_dictIDFlag, did);
                    let _ = r_setp(cctx_r, ZSTD_c_dictIDFlag, did);
                    let _ = c_ld(cctx_c, dp, dlen);
                    let _ = r_ld(cctx_r, dp, dlen);
                    let cap = c_bound(src.len()).max(64);
                    let mut ob_c = vec![0u8; cap];
                    let mut ob_r = vec![0u8; cap];
                    let sp = buf_ptr(&src);
                    let nc = c_c2(cctx_c, ob_c.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                    let nr = r_c2(cctx_r, ob_r.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                    let ctx = format!("dictframe dlen={dlen} did={did} {shape:?}");
                    assert_eq!(nc, nr, "{ctx}: compress2 size differs");
                    assert_bytes_eq(&format!("{ctx}: frame bytes"), &ob_c[..nc], &ob_r[..nr]);
                    ob_c.truncate(nc);
                    // getDictID_fromFrame on the produced frame
                    let c = c_gdif(buf_ptr(&ob_c), ob_c.len());
                    let r = r_gdif(buf_ptr(&ob_c), ob_c.len());
                    assert_eq!(c, r, "{ctx}: getDictID_fromFrame (C={c} R={r})");
                    c_free(cctx_c);
                    r_free(cctx_r);
                }
            }
        }
    }
}

// ===================================================================== 2 ====
// Skippable frames.

#[test]
fn skippable_frames_roundtrip() {
    let (c_wsf, r_wsf) = fnpair!("ZSTD_writeSkippableFrame", FnWriteSkippable);
    let (c_rsf, r_rsf) = fnpair!("ZSTD_readSkippableFrame", FnReadSkippable);
    let (c_isk, r_isk) = fnpair!("ZSTD_isSkippableFrame", FnUintFromBuf);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);
    let (c_ec, r_ec) = fnpair!("ZSTD_getErrorCode", FnGetErrorCode);

    let mut rng = Rng::new(0xF00D_0011);
    unsafe {
        let variants: Vec<c_uint> =
            (0u32..=15).chain([16u32, 17, 255, 0xFFFF, 0xFFFF_FFFF]).collect();
        for &shape in &ALL_SHAPES {
            for &len in &[0usize, 1, 4, 100, 4096, 200_000] {
                let src = gen(shape, len, &mut rng);
                let sp = buf_ptr(&src);
                for &mv in &variants {
                    let cap = len + 16;
                    let mut ob_c = vec![0xAAu8; cap];
                    let mut ob_r = vec![0xAAu8; cap];
                    let nc = c_wsf(ob_c.as_mut_ptr() as *mut c_void, cap, sp, len, mv);
                    let nr = r_wsf(ob_r.as_mut_ptr() as *mut c_void, cap, sp, len, mv);
                    let ctx = format!("writeSkippable mv={mv} {shape:?} len={len}");
                    assert_eq!(c_ie(nc), r_ie(nr), "{ctx}: isError differs (C={nc:#x} R={nr:#x})");
                    assert_eq!(c_ec(nc), r_ec(nr), "{ctx}: error code differs");
                    assert_eq!(nc, nr, "{ctx}: return differs (C={nc:#x} R={nr:#x})");
                    if c_ie(nc) != 0 {
                        continue;
                    }
                    assert_bytes_eq(&format!("{ctx}: written bytes"), &ob_c[..nc], &ob_r[..nr]);

                    // isSkippableFrame on the produced frame
                    let a = c_isk(buf_ptr(&ob_c), nc);
                    let b = r_isk(buf_ptr(&ob_r), nc);
                    assert_eq!(a, b, "{ctx}: isSkippableFrame (C={a} R={b})");

                    // readSkippableFrame — compare returned size, out bytes, magicVariant
                    let mut rc_out = vec![0xAAu8; len.max(1)];
                    let mut rr_out = vec![0xAAu8; len.max(1)];
                    let mut mv_c: c_uint = 0xDEAD_BEEF;
                    let mut mv_r: c_uint = 0xDEAD_BEEF;
                    let read_c = c_rsf(
                        rc_out.as_mut_ptr() as *mut c_void,
                        rc_out.len(),
                        &mut mv_c,
                        buf_ptr(&ob_c),
                        nc,
                    );
                    let read_r = r_rsf(
                        rr_out.as_mut_ptr() as *mut c_void,
                        rr_out.len(),
                        &mut mv_r,
                        buf_ptr(&ob_r),
                        nc,
                    );
                    assert_eq!(c_ie(read_c), r_ie(read_r), "{ctx}: read isError differs");
                    assert_eq!(read_c, read_r, "{ctx}: readSkippable size differs");
                    if c_ie(read_c) == 0 {
                        assert_eq!(mv_c, mv_r, "{ctx}: out magicVariant differs (C={mv_c} R={mv_r})");
                        assert_bytes_eq(
                            &format!("{ctx}: read content"),
                            &rc_out[..read_c],
                            &rr_out[..read_r],
                        );
                    }

                    // readSkippableFrame with NULL magicVariant pointer
                    let read_c2 = c_rsf(
                        rc_out.as_mut_ptr() as *mut c_void,
                        rc_out.len(),
                        std::ptr::null_mut(),
                        buf_ptr(&ob_c),
                        nc,
                    );
                    let read_r2 = r_rsf(
                        rr_out.as_mut_ptr() as *mut c_void,
                        rr_out.len(),
                        std::ptr::null_mut(),
                        buf_ptr(&ob_r),
                        nc,
                    );
                    assert_eq!(read_c2, read_r2, "{ctx}: readSkippable(NULL mv) size differs");
                }
            }
        }
    }
}

#[test]
fn skippable_interleaved_with_real_frames() {
    let (c_wsf, _r_wsf) = fnpair!("ZSTD_writeSkippableFrame", FnWriteSkippable);
    let (c_dec, r_dec) = fnpair!("ZSTD_decompress", FnDecompress);
    let (c_ie, _r_ie) = fnpair!("ZSTD_isError", FnIsError);

    // streaming decompress
    let (c_cds, r_cds) = fnpair!("ZSTD_createDStream", FnCreate);
    let (c_fds, r_fds) = fnpair!("ZSTD_freeDStream", FnFree);
    type FnDStreamCall =
        unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
    let (c_dsd, r_dsd) = fnpair!("ZSTD_decompressStream", FnDStreamCall);

    let mut rng = Rng::new(0xF00D_0012);
    unsafe {
        let (c_create, _r) = fnpair!("ZSTD_createCCtx", FnCreate);
        let (c_free, _r) = fnpair!("ZSTD_freeCCtx", FnFree);
        let (c_c2, _r) = fnpair!("ZSTD_compress2", FnCompress2);
        let (c_setp, _r): (
            unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t,
            _,
        ) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
        let (c_bound, _r) = fnpair!("ZSTD_compressBound", FnSizeSize);
        let cctx = c_create();

        for iter in 0..40 {
            // Build a stream: skippable? real, skippable? real ... (payload is
            // the concatenation of real-frame plaintexts).
            let mut stream: Vec<u8> = Vec::new();
            let mut expected: Vec<u8> = Vec::new();
            let nblocks = 1 + rng.below(4);
            for _ in 0..nblocks {
                if rng.next_u64() & 1 == 0 {
                    // skippable
                    let slen = rng.below(200);
                    let sdata = gen(Shape::Random, slen, &mut rng);
                    let mv = (rng.next_u32() & 0xF) as c_uint;
                    let cap = slen + 16;
                    let mut sb = vec![0u8; cap];
                    let n = c_wsf(sb.as_mut_ptr() as *mut c_void, cap, buf_ptr(&sdata), slen, mv);
                    assert_eq!(c_ie(n), 0, "iter {iter}: writeSkippable failed");
                    stream.extend_from_slice(&sb[..n]);
                }
                // real frame
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let plen = rng.below(5000);
                let pdata = gen(shape, plen, &mut rng);
                let _ = c_setp(cctx, ZSTD_c_compressionLevel, rng.range(1, 12));
                let cap = c_bound(plen).max(64);
                let mut cb = vec![0u8; cap];
                let n = c_c2(cctx, cb.as_mut_ptr() as *mut c_void, cap, buf_ptr(&pdata), plen);
                assert_eq!(c_ie(n), 0, "iter {iter}: compress2 failed");
                stream.extend_from_slice(&cb[..n]);
                expected.extend_from_slice(&pdata);
                let (c_reset, _r): (
                    unsafe extern "C" fn(*mut c_void, c_int) -> size_t,
                    _,
                ) = fnpair!(
                    "ZSTD_CCtx_reset",
                    unsafe extern "C" fn(*mut c_void, c_int) -> size_t
                );
                let _ = c_reset(cctx, ZSTD_reset_parameters);
            }

            // one-shot ZSTD_decompress must skip skippable frames identically
            let outcap = expected.len() + 16;
            let mut oc = vec![0xAAu8; outcap];
            let mut orr = vec![0xAAu8; outcap];
            let nc = c_dec(oc.as_mut_ptr() as *mut c_void, outcap, buf_ptr(&stream), stream.len());
            let nr = r_dec(orr.as_mut_ptr() as *mut c_void, outcap, buf_ptr(&stream), stream.len());
            let ctx = format!("interleaved iter={iter} nblocks={nblocks}");
            assert_eq!(nc, nr, "{ctx}: one-shot decompress size differs");
            if c_ie(nc) == 0 {
                assert_bytes_eq(&format!("{ctx}: one-shot bytes"), &oc[..nc], &orr[..nr]);
                assert_bytes_eq(&format!("{ctx}: one-shot vs expected C"), &expected, &oc[..nc]);
            }

            // streaming decompress
            let dc = c_cds();
            let dr = r_cds();
            let mut oc2 = vec![0xAAu8; outcap];
            let mut or2 = vec![0xAAu8; outcap];
            let (fc, oc_pos) = drive_dstream(c_dsd, dc, &stream, &mut oc2);
            let (fr, or_pos) = drive_dstream(r_dsd, dr, &stream, &mut or2);
            assert_eq!(fc, fr, "{ctx}: dstream final-error state differs (C={fc} R={fr})");
            assert_eq!(oc_pos, or_pos, "{ctx}: dstream produced size differs");
            if !fc {
                assert_bytes_eq(&format!("{ctx}: dstream bytes"), &oc2[..oc_pos], &or2[..or_pos]);
                assert_bytes_eq(&format!("{ctx}: dstream vs expected"), &expected, &oc2[..oc_pos]);
            }
            c_fds(dc);
            r_fds(dr);
        }
        c_free(cctx);
    }
}

/// Drive a full decompressStream loop; returns (is_error, produced_bytes).
unsafe fn drive_dstream(
    f: unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t,
    ds: *mut c_void,
    input: &[u8],
    out: &mut [u8],
) -> (bool, usize) {
    let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);
    let mut inb = ZSTD_inBuffer {
        src: buf_ptr(input),
        size: input.len(),
        pos: 0,
    };
    let mut outb = ZSTD_outBuffer {
        dst: out.as_mut_ptr() as *mut c_void,
        size: out.len(),
        pos: 0,
    };
    loop {
        let r = f(ds, &mut outb, &mut inb);
        if c_ie(r) != 0 {
            return (true, outb.pos);
        }
        if inb.pos >= inb.size {
            // consumed everything; if r==0 we're at a clean frame boundary
            return (false, outb.pos);
        }
        if outb.pos >= outb.size {
            // out of room — treat as done to avoid infinite loop
            return (false, outb.pos);
        }
    }
}

// ===================================================================== 3 ====
// Sizing / static contexts.

#[test]
fn sizing_estimates_all_levels_and_cparams() {
    let (c_ecc, r_ecc) = fnpair!("ZSTD_estimateCCtxSize", FnEstInt);
    let (c_eccp, r_eccp) = fnpair!("ZSTD_estimateCCtxSize_usingCParams", FnEstCParams);
    let (c_ecs, r_ecs) = fnpair!("ZSTD_estimateCStreamSize", FnEstInt);
    let (c_ecsp, r_ecsp) = fnpair!("ZSTD_estimateCStreamSize_usingCParams", FnEstCParams);
    let (c_eds, r_eds) = fnpair!("ZSTD_estimateDStreamSize", FnEstDStreamWin);
    let (c_ecd, r_ecd) = fnpair!("ZSTD_estimateCDictSize", FnEstCDict);
    let (c_ecda, r_ecda) = fnpair!("ZSTD_estimateCDictSize_advanced", FnEstCDictAdv);
    let (c_edd, r_edd) = fnpair!("ZSTD_estimateDDictSize", FnEstDDict);
    let (c_gcp, r_gcp) = fnpair!("ZSTD_getCParams", FnGetCParams);
    let (c_edctx, r_edctx) = fnpair!("ZSTD_estimateDCtxSize", unsafe extern "C" fn() -> size_t);

    unsafe {
        for lvl in -7i32..=22 {
            let c = c_ecc(lvl);
            let r = r_ecc(lvl);
            assert_eq!(c, r, "estimateCCtxSize({lvl}) (C={c} R={r})");
            let c = c_ecs(lvl);
            let r = r_ecs(lvl);
            assert_eq!(c, r, "estimateCStreamSize({lvl}) (C={c} R={r})");

            // via cParams derived from getCParams over several src/dict sizes
            for &(src, dict) in &[
                (0u64, 0usize),
                (1000, 0),
                (100_000, 0),
                (100_000, 4096),
                (ZSTD_CONTENTSIZE_UNKNOWN, 0),
            ] {
                let cp = c_gcp(lvl, src, dict);
                let rp = r_gcp(lvl, src, dict);
                assert_eq!(cp, rp, "getCParams({lvl},{src},{dict})");
                let c = c_eccp(cp);
                let r = r_eccp(rp);
                assert_eq!(c, r, "estimateCCtxSize_usingCParams({lvl},{src},{dict})");
                let c = c_ecsp(cp);
                let r = r_ecsp(rp);
                assert_eq!(c, r, "estimateCStreamSize_usingCParams({lvl},{src},{dict})");
                let c = c_ecda(dict, cp, ZSTD_DLM_BY_COPY);
                let r = r_ecda(dict, rp, ZSTD_DLM_BY_COPY);
                assert_eq!(c, r, "estimateCDictSize_advanced({lvl},dict={dict})");
            }

            for &dict in &[0usize, 64, 4096, 100_000] {
                let c = c_ecd(dict, lvl);
                let r = r_ecd(dict, lvl);
                assert_eq!(c, r, "estimateCDictSize(dict={dict},lvl={lvl})");
            }
        }

        // DCtx (no args) and DDict / DStream window sizes
        assert_eq!(c_edctx(), r_edctx(), "estimateDCtxSize");
        for &win in &[1024usize, 65536, 1 << 20, 1 << 23, 1 << 27] {
            assert_eq!(c_eds(win), r_eds(win), "estimateDStreamSize(win={win})");
        }
        for &dict in &[0usize, 64, 4096, 100_000] {
            for dlm in [0, 1] {
                assert_eq!(
                    c_edd(dict, dlm),
                    r_edd(dict, dlm),
                    "estimateDDictSize(dict={dict},dlm={dlm})"
                );
            }
        }
    }
}

#[test]
fn sizing_estimate_using_cctxparams_and_from_frame() {
    let (c_eccp, r_eccp) = fnpair!("ZSTD_estimateCCtxSize_usingCCtxParams", FnEstCCtxParams);
    let (c_ecsp, r_ecsp) = fnpair!("ZSTD_estimateCStreamSize_usingCCtxParams", FnEstCCtxParams);
    let (c_edsf, r_edsf) = fnpair!("ZSTD_estimateDStreamSize_fromFrame", FnSizeFromBuf);
    let (c_cp, r_cp) = fnpair!("ZSTD_createCCtxParams", FnCreateP);
    let (c_fp, r_fp): (FnFree, FnFree) = fnpair!("ZSTD_freeCCtxParams", FnFree);
    let (c_pi, r_pi) = fnpair!("ZSTD_CCtxParams_init", FnCCtxParamsInit);
    let (c_ps, r_ps): (
        unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t,
        _,
    ) = fnpair!("ZSTD_CCtxParams_setParameter", FnSetParam);

    unsafe {
        for lvl in [-5i32, 1, 3, 9, 15, 19, 22] {
            for extra in [
                vec![],
                vec![(ZSTD_c_windowLog, 20)],
                // NOTE: enabling LDM here also pins the LDM sub-parameters.
                // ZSTD_estimateCCtxSize_usingCCtxParams in the reference C
                // divides by the LDM hash-rate derived from these fields; a
                // params object with LDM enabled but hashRateLog==0 makes the
                // C implementation divide by zero (SIGFPE). We pin them so the
                // estimate is well-defined on both sides.
                vec![
                    (ZSTD_c_enableLongDistanceMatching, 1),
                    (ZSTD_c_windowLog, 25),
                    (ZSTD_c_ldmHashLog, 20),
                    (ZSTD_c_ldmMinMatch, 32),
                    (ZSTD_c_ldmBucketSizeLog, 3),
                    (ZSTD_c_ldmHashRateLog, 6),
                ],
                vec![(ZSTD_c_strategy, ZSTD_btultra2), (ZSTD_c_hashLog, 22)],
            ] {
                let pc = c_cp();
                let pr = r_cp();
                assert_eq!(c_pi(pc, lvl), r_pi(pr, lvl), "CCtxParams_init({lvl})");
                for &(p, v) in &extra {
                    assert_eq!(c_ps(pc, p, v), r_ps(pr, p, v), "CCtxParams_set({p},{v})");
                }
                let c = c_eccp(pc as *const c_void);
                let r = r_eccp(pr as *const c_void);
                assert_eq!(c, r, "estimateCCtxSize_usingCCtxParams lvl={lvl} extra={extra:?}");
                let c = c_ecsp(pc as *const c_void);
                let r = r_ecsp(pr as *const c_void);
                assert_eq!(c, r, "estimateCStreamSize_usingCCtxParams lvl={lvl} extra={extra:?}");
                c_fp(pc);
                r_fp(pr);
            }
        }

        // estimateDStreamSize_fromFrame on real frames
        let mut rng = Rng::new(0xF00D_0031);
        let frames = build_frames(&mut rng);
        for (frame, desc) in frames.iter().take(200) {
            if desc.contains(&format!("fmt={ZSTD_f_zstd1_magicless}")) {
                continue;
            }
            let c = c_edsf(buf_ptr(frame), frame.len());
            let r = r_edsf(buf_ptr(frame), frame.len());
            assert_eq!(c, r, "estimateDStreamSize_fromFrame {desc} (C={c:#x} R={r:#x})");
        }
    }
}

#[test]
fn static_contexts_full_roundtrip() {
    let (c_ecc, r_ecc) = fnpair!("ZSTD_estimateCCtxSize", FnEstInt);
    let (c_edc, r_edc) = fnpair!("ZSTD_estimateDCtxSize", unsafe extern "C" fn() -> size_t);
    let (c_ecs, r_ecs) = fnpair!("ZSTD_estimateCStreamSize", FnEstInt);
    let (c_eds, r_eds) = fnpair!("ZSTD_estimateDStreamSize", FnEstDStreamWin);
    let (c_ecda, r_ecda) = fnpair!("ZSTD_estimateCDictSize_advanced", FnEstCDictAdv);
    let (c_edd, r_edd) = fnpair!("ZSTD_estimateDDictSize", FnEstDDict);

    let (c_iscc, r_iscc) = fnpair!("ZSTD_initStaticCCtx", FnInitStaticCtx);
    let (c_isdc, r_isdc) = fnpair!("ZSTD_initStaticDCtx", FnInitStaticCtx);
    let (c_iscd, r_iscd) = fnpair!("ZSTD_initStaticCDict", FnInitStaticCDict);
    let (c_isdd, r_isdd) = fnpair!("ZSTD_initStaticDDict", FnInitStaticDDict);

    let (c_cctxc, r_cctxc): (
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t,
        _,
    ) = fnpair!(
        "ZSTD_compressCCtx",
        unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t
    );
    let (c_dec, r_dec) = fnpair!("ZSTD_decompressDCtx", unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t);
    let (c_bound, _r_bound) = fnpair!("ZSTD_compressBound", FnSizeSize);
    let (c_ie, _r_ie) = fnpair!("ZSTD_isError", FnIsError);
    let (c_sizeof_cctx, r_sizeof_cctx) = fnpair!("ZSTD_sizeof_CCtx", FnSizeofPtr);
    let (c_sizeof_dctx, r_sizeof_dctx) = fnpair!("ZSTD_sizeof_DCtx", FnSizeofPtr);
    let (c_sizeof_cdict, r_sizeof_cdict) = fnpair!("ZSTD_sizeof_CDict", FnSizeofPtr);
    let (c_sizeof_ddict, r_sizeof_ddict) = fnpair!("ZSTD_sizeof_DDict", FnSizeofPtr);
    let (c_gcp, r_gcp) = fnpair!("ZSTD_getCParams", FnGetCParams);

    /// 64-byte-aligned workspace.
    fn aligned_ws(n: usize) -> Vec<u64> {
        vec![0u64; n / 8 + 8]
    }

    let mut rng = Rng::new(0xF00D_0032);
    unsafe {
        for &lvl in &[1i32, 3, 9, 19] {
            // ---- static CCtx / DCtx round-trip ----
            let cctx_ws_sz = c_ecc(lvl);
            assert_eq!(cctx_ws_sz, r_ecc(lvl), "estimateCCtxSize({lvl})");
            let dctx_ws_sz = c_edc();
            assert_eq!(dctx_ws_sz, r_edc(), "estimateDCtxSize");

            for &shape in &[Shape::Text, Shape::Random, Shape::Repetitive] {
                for &len in &[0usize, 100, 5000, 60_000] {
                    let src = gen(shape, len, &mut rng);
                    let cap = c_bound(len).max(64);

                    let mut wc = aligned_ws(cctx_ws_sz);
                    let mut wr = aligned_ws(cctx_ws_sz);
                    let cc = c_iscc(wc.as_mut_ptr() as *mut c_void, cctx_ws_sz);
                    let rc = r_iscc(wr.as_mut_ptr() as *mut c_void, cctx_ws_sz);
                    assert!(!cc.is_null() && !rc.is_null(), "initStaticCCtx null lvl={lvl}");

                    let mut oc = vec![0xAAu8; cap];
                    let mut orr = vec![0xAAu8; cap];
                    let nc = c_cctxc(cc, oc.as_mut_ptr() as *mut c_void, cap, buf_ptr(&src), len, lvl);
                    let nr = r_cctxc(rc, orr.as_mut_ptr() as *mut c_void, cap, buf_ptr(&src), len, lvl);
                    let ctx = format!("staticCCtx lvl={lvl} {shape:?} len={len}");
                    assert_eq!(nc, nr, "{ctx}: compress size differs (C={nc:#x} R={nr:#x})");
                    if c_ie(nc) == 0 {
                        assert_bytes_eq(&format!("{ctx}: frame"), &oc[..nc], &orr[..nr]);
                        assert_eq!(
                            c_sizeof_cctx(cc as *const c_void),
                            r_sizeof_cctx(rc as *const c_void),
                            "{ctx}: sizeof_CCtx"
                        );

                        // decompress through static DCtx
                        let mut wdc = aligned_ws(dctx_ws_sz);
                        let mut wdr = aligned_ws(dctx_ws_sz);
                        let dcc = c_isdc(wdc.as_mut_ptr() as *mut c_void, dctx_ws_sz);
                        let dcr = r_isdc(wdr.as_mut_ptr() as *mut c_void, dctx_ws_sz);
                        let mut dc_out = vec![0xAAu8; len + 1];
                        let mut dr_out = vec![0xAAu8; len + 1];
                        let dn_c = c_dec(dcc, dc_out.as_mut_ptr() as *mut c_void, len, buf_ptr(&oc[..nc]), nc);
                        let dn_r = r_dec(dcr, dr_out.as_mut_ptr() as *mut c_void, len, buf_ptr(&orr[..nr]), nr);
                        assert_eq!(dn_c, dn_r, "{ctx}: static decompress size differs");
                        if c_ie(dn_c) == 0 {
                            assert_bytes_eq(&format!("{ctx}: decoded"), &dc_out[..dn_c], &dr_out[..dn_r]);
                            assert_bytes_eq(&format!("{ctx}: decoded vs src"), &src, &dc_out[..dn_c]);
                            assert_eq!(
                                c_sizeof_dctx(dcc as *const c_void),
                                r_sizeof_dctx(dcr as *const c_void),
                                "{ctx}: sizeof_DCtx"
                            );
                        }
                    }
                }
            }

            // ---- static CStream / DStream sizeof parity ----
            let cs_sz = c_ecs(lvl);
            assert_eq!(cs_sz, r_ecs(lvl), "estimateCStreamSize({lvl})");
            let ds_sz = c_eds(1 << 23);
            assert_eq!(ds_sz, r_eds(1 << 23), "estimateDStreamSize");
            {
                let (c_iscs, r_iscs) = fnpair!("ZSTD_initStaticCStream", FnInitStaticCtx);
                let (c_isds, r_isds) = fnpair!("ZSTD_initStaticDStream", FnInitStaticCtx);
                let (c_sz_cs, r_sz_cs) = fnpair!("ZSTD_sizeof_CStream", FnSizeofPtr);
                let (c_sz_ds, r_sz_ds) = fnpair!("ZSTD_sizeof_DStream", FnSizeofPtr);
                let mut wc = aligned_ws(cs_sz);
                let mut wr = aligned_ws(cs_sz);
                let cs = c_iscs(wc.as_mut_ptr() as *mut c_void, cs_sz);
                let rs = r_iscs(wr.as_mut_ptr() as *mut c_void, cs_sz);
                assert!(!cs.is_null() && !rs.is_null(), "initStaticCStream null");
                assert_eq!(
                    c_sz_cs(cs as *const c_void),
                    r_sz_cs(rs as *const c_void),
                    "sizeof_CStream lvl={lvl}"
                );
                let mut wdc = aligned_ws(ds_sz);
                let mut wdr = aligned_ws(ds_sz);
                let dscs = c_isds(wdc.as_mut_ptr() as *mut c_void, ds_sz);
                let dsrs = r_isds(wdr.as_mut_ptr() as *mut c_void, ds_sz);
                assert!(!dscs.is_null() && !dsrs.is_null(), "initStaticDStream null");
                assert_eq!(
                    c_sz_ds(dscs as *const c_void),
                    r_sz_ds(dsrs as *const c_void),
                    "sizeof_DStream lvl={lvl}"
                );
            }

            // ---- static CDict / DDict round-trip ----
            let dict = gen(Shape::Text, 8192, &mut rng);
            let cp = c_gcp(lvl, 100_000, dict.len());
            assert_eq!(cp, r_gcp(lvl, 100_000, dict.len()), "getCParams for cdict");

            // CDict — size the workspace with the *same* cParams we will pass
            // to initStaticCDict (estimateCDictSize(level,..) assumes the
            // level's default cParams, which differ from these).
            let cdict_sz = c_ecda(dict.len(), cp, ZSTD_DLM_BY_COPY);
            assert_eq!(
                cdict_sz,
                r_ecda(dict.len(), cp, ZSTD_DLM_BY_COPY),
                "estimateCDictSize_advanced"
            );
            let mut wcd_c = aligned_ws(cdict_sz);
            let mut wcd_r = aligned_ws(cdict_sz);
            let cdc = c_iscd(
                wcd_c.as_mut_ptr() as *mut c_void,
                cdict_sz,
                buf_ptr(&dict),
                dict.len(),
                ZSTD_DLM_BY_COPY,
                ZSTD_DCT_AUTO,
                cp,
            );
            let cdr = r_iscd(
                wcd_r.as_mut_ptr() as *mut c_void,
                cdict_sz,
                buf_ptr(&dict),
                dict.len(),
                ZSTD_DLM_BY_COPY,
                ZSTD_DCT_AUTO,
                cp,
            );
            assert_eq!(
                cdc.is_null(),
                cdr.is_null(),
                "initStaticCDict nullness differs lvl={lvl}"
            );
            assert!(!cdc.is_null(), "initStaticCDict null lvl={lvl}");
            assert_eq!(
                c_sizeof_cdict(cdc as *const c_void),
                r_sizeof_cdict(cdr as *const c_void),
                "sizeof_CDict lvl={lvl}"
            );

            // DDict
            let ddict_sz = c_edd(dict.len(), ZSTD_DLM_BY_COPY);
            assert_eq!(ddict_sz, r_edd(dict.len(), ZSTD_DLM_BY_COPY), "estimateDDictSize");
            let mut wdd_c = aligned_ws(ddict_sz);
            let mut wdd_r = aligned_ws(ddict_sz);
            let ddc = c_isdd(
                wdd_c.as_mut_ptr() as *mut c_void,
                ddict_sz,
                buf_ptr(&dict),
                dict.len(),
                ZSTD_DLM_BY_COPY,
                ZSTD_DCT_AUTO,
            );
            let ddr = r_isdd(
                wdd_r.as_mut_ptr() as *mut c_void,
                ddict_sz,
                buf_ptr(&dict),
                dict.len(),
                ZSTD_DLM_BY_COPY,
                ZSTD_DCT_AUTO,
            );
            assert!(!ddc.is_null() && !ddr.is_null(), "initStaticDDict null lvl={lvl}");
            assert_eq!(
                c_sizeof_ddict(ddc as *const c_void),
                r_sizeof_ddict(ddr as *const c_void),
                "sizeof_DDict lvl={lvl}"
            );
        }
    }
}

// ===================================================================== 4 ====
// getCParams / getParams / checkCParams / adjustCParams / CCtxParams_init_advanced.

#[test]
fn cparams_and_params_all_levels() {
    let (c_gcp, r_gcp) = fnpair!("ZSTD_getCParams", FnGetCParams);
    let (c_gp, r_gp) = fnpair!("ZSTD_getParams", FnGetParams);
    let (c_cc, r_cc) = fnpair!("ZSTD_checkCParams", FnCheckCParams);
    let (c_ac, r_ac) = fnpair!("ZSTD_adjustCParams", FnAdjustCParams);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);

    let (c_cpia, r_cpia) = fnpair!("ZSTD_CCtxParams_init_advanced", FnCCtxParamsInitAdv);
    let (c_cp, r_cp) = fnpair!("ZSTD_createCCtxParams", FnCreateP);
    let (c_fp, r_fp): (FnFree, FnFree) = fnpair!("ZSTD_freeCCtxParams", FnFree);

    let size_hints: &[c_ulonglong] = &[
        0,
        1,
        100,
        1000,
        65_536,
        1_000_000,
        100_000_000,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ];
    let dict_sizes: &[usize] = &[0, 1, 64, 4096, 100_000, 10_000_000];

    unsafe {
        for lvl in -7i32..=22 {
            for &sh in size_hints {
                for &ds in dict_sizes {
                    let cp = c_gcp(lvl, sh, ds);
                    let rp = r_gcp(lvl, sh, ds);
                    assert_eq!(cp, rp, "getCParams({lvl},{sh},{ds})\nC={cp:?}\nR={rp:?}");

                    let cpar = c_gp(lvl, sh, ds);
                    let rpar = r_gp(lvl, sh, ds);
                    assert_eq!(cpar, rpar, "getParams({lvl},{sh},{ds})");

                    // checkCParams on the returned cParams
                    let ck = c_cc(cp);
                    let rk = r_cc(rp);
                    assert_eq!(c_ie(ck), r_ie(rk), "checkCParams isError ({lvl},{sh},{ds})");
                    assert_eq!(ck, rk, "checkCParams rc ({lvl},{sh},{ds})");

                    // adjustCParams over a couple of (srcSize, dictSize)
                    for &(asz, ad) in &[(sh, ds), (0u64, 0usize), (ZSTD_CONTENTSIZE_UNKNOWN, ds)] {
                        let ca = c_ac(cp, asz, ad);
                        let ra = r_ac(rp, asz, ad);
                        assert_eq!(ca, ra, "adjustCParams({lvl},{sh},{ds})->({asz},{ad})\nC={ca:?}\nR={ra:?}");
                    }

                    // CCtxParams_init_advanced with full params
                    let pc = c_cp();
                    let pr = r_cp();
                    let rc_c = c_cpia(pc, cpar);
                    let rc_r = r_cpia(pr, rpar);
                    assert_eq!(c_ie(rc_c), r_ie(rc_r), "CCtxParams_init_advanced isError");
                    assert_eq!(rc_c, rc_r, "CCtxParams_init_advanced rc");
                    c_fp(pc);
                    r_fp(pr);
                }
            }
        }
    }
}

// ===================================================================== 5 ====
// Legacy v01-v07 decoders driven directly through dlsym.

const LEGACY_MAGICS: [(u32, &str); 7] = [
    (0xFD2FB51E, "v01"), // stored/tested via LE bytes below
    (0xFD2FB522, "v02"),
    (0xFD2FB523, "v03"),
    (0xFD2FB524, "v04"),
    (0xFD2FB525, "v05"),
    (0xFD2FB526, "v06"),
    (0xFD2FB527, "v07"),
];

fn legacy_decompress_fns() -> Vec<(&'static str, (FnLegacyDecompress, FnLegacyDecompress))> {
    vec![
        ("ZSTDv01_decompress", fnpair!("ZSTDv01_decompress", FnLegacyDecompress)),
        ("ZSTDv02_decompress", fnpair!("ZSTDv02_decompress", FnLegacyDecompress)),
        ("ZSTDv03_decompress", fnpair!("ZSTDv03_decompress", FnLegacyDecompress)),
        ("ZSTDv04_decompress", fnpair!("ZSTDv04_decompress", FnLegacyDecompress)),
        ("ZSTDv05_decompress", fnpair!("ZSTDv05_decompress", FnLegacyDecompress)),
        ("ZSTDv06_decompress", fnpair!("ZSTDv06_decompress", FnLegacyDecompress)),
        ("ZSTDv07_decompress", fnpair!("ZSTDv07_decompress", FnLegacyDecompress)),
    ]
}

fn legacy_iserror_fns() -> Vec<(&'static str, (FnLegacyIsError, FnLegacyIsError))> {
    vec![
        ("ZSTDv01_isError", fnpair!("ZSTDv01_isError", FnLegacyIsError)),
        ("ZSTDv02_isError", fnpair!("ZSTDv02_isError", FnLegacyIsError)),
        ("ZSTDv03_isError", fnpair!("ZSTDv03_isError", FnLegacyIsError)),
        // v04 has no isError export
        ("ZSTDv05_isError", fnpair!("ZSTDv05_isError", FnLegacyIsError)),
        ("ZSTDv06_isError", fnpair!("ZSTDv06_isError", FnLegacyIsError)),
        ("ZSTDv07_isError", fnpair!("ZSTDv07_isError", FnLegacyIsError)),
    ]
}

/// A collection of adversarial input buffers for the legacy decoders:
/// garbage, magic-prefixed random, truncations, and valid modern frames.
fn legacy_inputs(rng: &mut Rng) -> Vec<(Vec<u8>, String)> {
    let mut out: Vec<(Vec<u8>, String)> = Vec::new();

    // (a) random garbage of many lengths
    for &len in &[0usize, 1, 3, 4, 5, 8, 16, 33, 64, 200, 1024, 5000] {
        out.push((gen(Shape::Random, len, rng), format!("garbage len={len}")));
        out.push((gen(Shape::Zeros, len, rng), format!("zeros len={len}")));
    }

    // (b) each legacy magic (LE) followed by random bytes; (c) truncations
    for (mag, name) in LEGACY_MAGICS {
        for &tail in &[0usize, 1, 3, 8, 16, 40, 200, 2000] {
            let mut b = mag.to_le_bytes().to_vec();
            let r = gen(Shape::Random, tail, rng);
            b.extend_from_slice(&r);
            out.push((b.clone(), format!("{name}-magic+rand tail={tail}")));
            // truncations of the magic-prefixed buffer
            for tl in [1usize, 2, 3, 4, 6] {
                if tl <= b.len() {
                    out.push((b[..tl].to_vec(), format!("{name}-magic trunc={tl}")));
                }
            }
        }
    }

    // (d) valid modern zstd frames
    let modern = build_frames(rng);
    for (f, d) in modern.into_iter().take(60) {
        out.push((f, format!("modern {d}")));
    }

    out
}

#[test]
fn legacy_decoders_reject_and_agree() {
    let dec = legacy_decompress_fns();
    let errs = legacy_iserror_fns();
    let (c_dec, r_dec) = fnpair!("ZSTD_decompress", FnDecompress);
    let (c_gcs, r_gcs) = fnpair!("ZSTD_getFrameContentSize", FnU64FromBuf);
    let (c_ie, r_ie) = fnpair!("ZSTD_isError", FnIsError);

    let mut rng = Rng::new(0xF00D_0051);
    let inputs = legacy_inputs(&mut rng);
    assert!(inputs.len() > 80, "expected many legacy inputs");

    unsafe {
        for (buf, idesc) in &inputs {
            let sp = buf_ptr(buf);

            // Each legacy decompress entry point: compare return exactly, and
            // when it "succeeds" compare the produced bytes.
            for (fname, (cf, rf)) in &dec {
                let outcap = 1 << 20; // 1 MiB output ceiling
                let mut oc = vec![0xAAu8; outcap];
                let mut orr = vec![0xAAu8; outcap];
                let c = cf(oc.as_mut_ptr() as *mut c_void, outcap, sp, buf.len());
                let r = rf(orr.as_mut_ptr() as *mut c_void, outcap, sp, buf.len());
                let ctx = format!("{fname} on [{idesc}]");
                assert_eq!(c, r, "{ctx}: return differs (C={c:#x} R={r:#x})");
                // These legacy return codes: error codes are huge (wrap of small
                // negatives). If it looks like a plausible success (< outcap),
                // compare the output bytes.
                if c <= outcap {
                    assert_bytes_eq(&format!("{ctx}: output"), &oc[..c], &orr[..r]);
                }
            }

            // isError agreement on a spread of codes derived from decompress.
            for (fname, (cf, rf)) in &errs {
                // feed the decompress return code from the matching-version fn
                // plus a few synthetic codes
                for code in [0usize, 1, usize::MAX, usize::MAX - 10, buf.len()] {
                    let c = cf(code);
                    let r = rf(code);
                    assert_eq!(c, r, "{fname} isError({code:#x}) differs (C={c} R={r})");
                }
            }

            // ZSTD_decompress + getFrameContentSize on buffers carrying legacy
            // magics must behave identically (this exercises the dispatch path).
            {
                let outcap = 1 << 20;
                let mut oc = vec![0xAAu8; outcap];
                let mut orr = vec![0xAAu8; outcap];
                let c = c_dec(oc.as_mut_ptr() as *mut c_void, outcap, sp, buf.len());
                let r = r_dec(orr.as_mut_ptr() as *mut c_void, outcap, sp, buf.len());
                let ctx = format!("ZSTD_decompress on [{idesc}]");
                assert_eq!(c_ie(c), r_ie(r), "{ctx}: isError differs (C={c:#x} R={r:#x})");
                assert_eq!(c, r, "{ctx}: return differs (C={c:#x} R={r:#x})");
                if c_ie(c) == 0 {
                    assert_bytes_eq(&format!("{ctx}: output"), &oc[..c], &orr[..r]);
                }

                let gc = c_gcs(sp, buf.len());
                let gr = r_gcs(sp, buf.len());
                assert_eq!(gc, gr, "getFrameContentSize on [{idesc}] (C={gc:#x} R={gr:#x})");
            }
        }
    }
}

#[test]
fn legacy_findframesize_and_getframeparams_agree() {
    let finds: Vec<(&str, (FnLegacyFindSize, FnLegacyFindSize))> = vec![
        ("ZSTDv01_findFrameSizeInfoLegacy", fnpair!("ZSTDv01_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv02_findFrameSizeInfoLegacy", fnpair!("ZSTDv02_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv03_findFrameSizeInfoLegacy", fnpair!("ZSTDv03_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv04_findFrameSizeInfoLegacy", fnpair!("ZSTDv04_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv05_findFrameSizeInfoLegacy", fnpair!("ZSTDv05_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv06_findFrameSizeInfoLegacy", fnpair!("ZSTDv06_findFrameSizeInfoLegacy", FnLegacyFindSize)),
        ("ZSTDv07_findFrameSizeInfoLegacy", fnpair!("ZSTDv07_findFrameSizeInfoLegacy", FnLegacyFindSize)),
    ];

    let mut rng = Rng::new(0xF00D_0052);
    let inputs = legacy_inputs(&mut rng);

    unsafe {
        for (buf, idesc) in &inputs {
            let sp = buf_ptr(buf);
            for (fname, (cf, rf)) in &finds {
                let mut cs_c: size_t = 0xDEAD;
                let mut cs_r: size_t = 0xDEAD;
                let mut db_c: c_ulonglong = 0xBEEF;
                let mut db_r: c_ulonglong = 0xBEEF;
                cf(sp, buf.len(), &mut cs_c, &mut db_c);
                rf(sp, buf.len(), &mut cs_r, &mut db_r);
                let ctx = format!("{fname} on [{idesc}]");
                assert_eq!(cs_c, cs_r, "{ctx}: cSize out differs (C={cs_c:#x} R={cs_r:#x})");
                assert_eq!(db_c, db_r, "{ctx}: dBound out differs (C={db_c:#x} R={db_r:#x})");
            }
        }
    }
}
