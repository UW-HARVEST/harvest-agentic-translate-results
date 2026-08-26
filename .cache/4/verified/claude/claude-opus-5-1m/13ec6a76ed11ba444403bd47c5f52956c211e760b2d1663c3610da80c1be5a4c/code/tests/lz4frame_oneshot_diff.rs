//! Differential tests for the ONE-SHOT + METADATA surface of `lz4frame.c`.
//!
//! Symbols under test (and only these):
//!   LZ4F_getVersion, LZ4F_compressionLevel_max, LZ4F_getBlockSize, LZ4F_isError,
//!   LZ4F_getErrorName, LZ4F_getErrorCode, LZ4F_compressFrameBound, LZ4F_compressBound,
//!   LZ4F_compressFrame, LZ4F_compressFrame_usingCDict, LZ4F_createCDict,
//!   LZ4F_createCDict_advanced, LZ4F_freeCDict, LZ4F_headerSize
//!
//! `LZ4F_decompress` (+ its dctx create/free) is used ONLY as a round-trip oracle for
//! frames produced by the one-shot compressor; it is not itself the subject of tests.
//!
//! Every destination buffer of BOTH libraries is pre-filled with the same 0xAA
//! sentinel and the FULL buffer is compared, so untouched bytes cannot mask a
//! divergence and cannot create a false positive either.

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

mod common;

use common::*;
use std::cell::Cell;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;

const SENTINEL: u8 = 0xAA;

// ---------------------------------------------------------------------------
// Signatures — each checked against c_src/include/lz4frame.h (+ _static) and
// c_src/src/lz4frame.c.
// ---------------------------------------------------------------------------

/// `unsigned LZ4F_getVersion(void)` — lz4frame.c:329
type FnGetVersion = unsafe extern "C" fn() -> c_uint;
/// `int LZ4F_compressionLevel_max(void)` — lz4frame.c:331
type FnLevelMax = unsafe extern "C" fn() -> c_int;
/// `size_t LZ4F_getBlockSize(LZ4F_blockSizeID_t)` — lz4frame.c:333 (enum param == int)
type FnGetBlockSize = unsafe extern "C" fn(c_int) -> usize;
/// `unsigned LZ4F_isError(LZ4F_errorCode_t)` — lz4frame.c:293 (LZ4F_errorCode_t == size_t)
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
/// `const char* LZ4F_getErrorName(LZ4F_errorCode_t)` — lz4frame.c:298
type FnGetErrorName = unsafe extern "C" fn(usize) -> *const c_char;
/// `LZ4F_errorCodes LZ4F_getErrorCode(size_t)` — lz4frame.c:305 (enum return == int)
type FnGetErrorCode = unsafe extern "C" fn(usize) -> c_int;
/// `size_t LZ4F_compressFrameBound(size_t, const LZ4F_preferences_t*)` — lz4frame.c:406
/// `size_t LZ4F_compressBound(size_t, const LZ4F_preferences_t*)`      — lz4frame.c:867
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
/// `size_t LZ4F_compressFrame(void*, size_t, const void*, size_t, const LZ4F_preferences_t*)`
/// — lz4frame.c:484
type FnCompressFrame = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
/// `size_t LZ4F_compressFrame_usingCDict(LZ4F_cctx*, void*, size_t, const void*, size_t,
///                                      const LZ4F_CDict*, const LZ4F_preferences_t*)`
/// — lz4frame.c:428
type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
/// `LZ4F_CDict* LZ4F_createCDict(const void*, size_t)` — lz4frame.c:575
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
/// `LZ4F_CDict* LZ4F_createCDict_advanced(LZ4F_CustomMem, const void*, size_t)` — lz4frame.c:539
type FnCreateCDictAdv = unsafe extern "C" fn(LZ4F_CustomMem, *const c_void, usize) -> *mut c_void;
/// `void LZ4F_freeCDict(LZ4F_CDict*)` — lz4frame.c:581
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
/// `size_t LZ4F_headerSize(const void*, size_t)` — lz4frame.c:1444
type FnHeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

// Support symbols (not under test; needed to drive the CDict entry point and to
// round-trip the produced frames).
type FnCreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const c_void,
    usize,
    *const LZ4F_decompressOptions_t,
) -> usize;

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------

/// Source buffer whose `as_ptr()` is guaranteed to be a *real* allocation even for
/// length 0 (so a 0-length src is a valid, non-dangling pointer).
fn gen_src(rng: &mut Rng, shape: usize, len: usize) -> Vec<u8> {
    let mut v = gen_shape(rng, shape, len);
    v.reserve(1);
    v
}

fn err_str(code: usize) -> String {
    if lz4f_is_error(code) {
        format!("error {}", lz4f_error_code(code))
    } else {
        format!("ok {}", code)
    }
}

/// `LZ4F_getBlockSize` from both libs, asserting parity, returning the value.
fn block_size(id: c_int) -> usize {
    let (c, r) = both::<FnGetBlockSize>("LZ4F_getBlockSize");
    let cv = unsafe { c(id) };
    let rv = unsafe { r(id) };
    assert_eq!(
        cv, rv,
        "LZ4F_getBlockSize({}): C={} Rust={}",
        id, cv, rv
    );
    cv
}

/// `LZ4F_compressFrameBound` from both libs, asserting parity.
fn frame_bound(src_size: usize, prefs: *const LZ4F_preferences_t) -> usize {
    let (c, r) = both::<FnBound>("LZ4F_compressFrameBound");
    let cv = unsafe { c(src_size, prefs) };
    let rv = unsafe { r(src_size, prefs) };
    assert_eq!(
        cv, rv,
        "LZ4F_compressFrameBound({}, {:?}): C={} Rust={}",
        src_size, prefs, cv, rv
    );
    cv
}

// ---------------------------------------------------------------------------
// Preference-set description
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct P {
    bsid: c_int,
    bmode: c_int,
    ccs: c_int,
    bcs: c_int,
    ftype: c_int,
    autoflush: c_uint,
    /// true => frameInfo.contentSize is set (compressFrame auto-corrects it to srcSize)
    csize: bool,
    dict_id: c_uint,
    level: c_int,
    favor: c_uint,
}

impl P {
    fn base() -> P {
        P {
            bsid: LZ4F_default,
            bmode: LZ4F_blockLinked,
            ccs: LZ4F_noContentChecksum,
            bcs: LZ4F_noBlockChecksum,
            ftype: LZ4F_frame,
            autoflush: 0,
            csize: false,
            dict_id: 0,
            level: 0,
            favor: 0,
        }
    }

    fn to_prefs(&self, src_len: usize) -> LZ4F_preferences_t {
        let mut pr = LZ4F_preferences_t::default();
        pr.frameInfo.blockSizeID = self.bsid;
        pr.frameInfo.blockMode = self.bmode;
        pr.frameInfo.contentChecksumFlag = self.ccs;
        pr.frameInfo.blockChecksumFlag = self.bcs;
        pr.frameInfo.frameType = self.ftype;
        pr.frameInfo.contentSize = if self.csize { src_len as u64 } else { 0 };
        pr.frameInfo.dictID = self.dict_id;
        pr.compressionLevel = self.level;
        pr.autoFlush = self.autoflush;
        pr.favorDecSpeed = self.favor;
        pr
    }
}

impl std::fmt::Display for P {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bsid={} bmode={} ccs={} bcs={} ftype={} autoFlush={} contentSize={} dictID={:#x} level={} favor={}",
            self.bsid,
            self.bmode,
            self.ccs,
            self.bcs,
            self.ftype,
            self.autoflush,
            self.csize,
            self.dict_id,
            self.level,
            self.favor
        )
    }
}

// ---------------------------------------------------------------------------
// Round-trip oracle: decode a frame with BOTH libraries' LZ4F_decompress
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct DecApi {
    create: FnCreateDctx,
    free: FnFreeDctx,
    dec: FnDecompress,
    dec_dict: FnDecompressUsingDict,
    tag: &'static str,
}

fn dec_apis() -> (DecApi, DecApi) {
    let (cc, rc) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cf, rf) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cd, rd) = both::<FnDecompress>("LZ4F_decompress");
    let (cdd, rdd) = both::<FnDecompressUsingDict>("LZ4F_decompress_usingDict");
    (
        DecApi { create: cc, free: cf, dec: cd, dec_dict: cdd, tag: "C" },
        DecApi { create: rc, free: rf, dec: rd, dec_dict: rdd, tag: "Rust" },
    )
}

/// Decode `frame` with `api`. When `dict` is `Some`, `LZ4F_decompress_usingDict`
/// is used (a CDict is *not* recorded inside the frame, so the same dictionary has
/// to be supplied at decode time).
unsafe fn decode_frame(
    api: &DecApi,
    ctx: &str,
    frame: &[u8],
    expect: &[u8],
    dict: Option<&[u8]>,
) {
    let mut dctx: *mut c_void = ptr::null_mut();
    let cr = (api.create)(&mut dctx, LZ4F_VERSION);
    assert!(
        !lz4f_is_error(cr) && !dctx.is_null(),
        "{}: {} createDecompressionContext failed ({})",
        ctx,
        api.tag,
        err_str(cr)
    );

    let mut out: Vec<u8> = Vec::with_capacity(expect.len() + 64);
    let mut chunk = vec![0u8; expect.len().max(64) + 64];
    let mut spos = 0usize;
    loop {
        let mut dsz = chunk.len();
        let mut ssz = frame.len() - spos;
        let hint = match dict {
            None => (api.dec)(
                dctx,
                chunk.as_mut_ptr() as *mut c_void,
                &mut dsz,
                frame.as_ptr().add(spos) as *const c_void,
                &mut ssz,
                ptr::null(),
            ),
            Some(d) => (api.dec_dict)(
                dctx,
                chunk.as_mut_ptr() as *mut c_void,
                &mut dsz,
                frame.as_ptr().add(spos) as *const c_void,
                &mut ssz,
                d.as_ptr() as *const c_void,
                d.len(),
                ptr::null(),
            ),
        };
        assert!(
            !lz4f_is_error(hint),
            "{}: {} LZ4F_decompress failed: {}",
            ctx,
            api.tag,
            err_str(hint)
        );
        out.extend_from_slice(&chunk[..dsz]);
        spos += ssz;
        if hint == 0 {
            break;
        }
        assert!(
            !(ssz == 0 && dsz == 0),
            "{}: {} LZ4F_decompress stalled (hint={} consumed={}/{})",
            ctx,
            api.tag,
            hint,
            spos,
            frame.len()
        );
    }
    (api.free)(dctx);
    assert_bytes_eq(&format!("{} [{} round-trip]", ctx, api.tag), expect, &out);
}

fn round_trip_both(ctx: &str, frame: &[u8], expect: &[u8]) {
    let (c, r) = dec_apis();
    unsafe {
        decode_frame(&c, ctx, frame, expect, None);
        decode_frame(&r, ctx, frame, expect, None);
    }
}

/// Round-trip a frame produced with a CDict. `LZ4F_createCDict*` keeps only the
/// LAST 64 KB of the dictionary buffer (lz4frame.c:546-549), so the decoder has to
/// be given exactly that suffix.
fn round_trip_both_with_dict(ctx: &str, frame: &[u8], expect: &[u8], dict: &[u8]) {
    let eff = if dict.len() > 64 * 1024 {
        &dict[dict.len() - 64 * 1024..]
    } else {
        dict
    };
    let (c, r) = dec_apis();
    unsafe {
        decode_frame(&c, ctx, frame, expect, Some(eff));
        decode_frame(&r, ctx, frame, expect, Some(eff));
    }
}

// ---------------------------------------------------------------------------
// LZ4F_compressFrame differential driver
// ---------------------------------------------------------------------------

/// Run `LZ4F_compressFrame` on both libraries with identical 0xAA-pre-filled dst
/// buffers of `cap` bytes; assert identical return AND identical full buffer.
/// Returns `(ret, c_buffer)`.
fn frame_raw(
    ctx: &str,
    prefs: *const LZ4F_preferences_t,
    src: *const c_void,
    src_len: usize,
    cap: usize,
) -> (usize, Vec<u8>) {
    let (cf, rf) = both::<FnCompressFrame>("LZ4F_compressFrame");
    let mut cbuf = vec![SENTINEL; cap.max(1)];
    let mut rbuf = vec![SENTINEL; cap.max(1)];
    let cr = unsafe { cf(cbuf.as_mut_ptr() as *mut c_void, cap, src, src_len, prefs) };
    let rr = unsafe { rf(rbuf.as_mut_ptr() as *mut c_void, cap, src, src_len, prefs) };
    if cr != rr {
        panic!(
            "{}\n  LZ4F_compressFrame return mismatch: C={} ({})  Rust={} ({})",
            ctx,
            cr,
            err_str(cr),
            rr,
            err_str(rr)
        );
    }
    assert_bytes_eq(ctx, &cbuf, &rbuf);
    if !lz4f_is_error(cr) {
        assert!(cr <= cap, "{}: returned {} > dstCapacity {}", ctx, cr, cap);
    }
    (cr, cbuf)
}

fn frame_case(ctx: &str, prefs: &LZ4F_preferences_t, src: &[u8], cap: usize) -> (usize, Vec<u8>) {
    frame_raw(
        ctx,
        prefs as *const LZ4F_preferences_t,
        src.as_ptr() as *const c_void,
        src.len(),
        cap,
    )
}

/// Full "good path" case: compute the bound (asserting parity), compress with
/// dstCapacity == bound and (optionally) with dstCapacity == bound + slack, and
/// (optionally) round-trip the C-produced frame through both decompressors.
fn frame_good_opts(label: &str, p: &P, src: &[u8], do_round_trip: bool, also_larger_dst: bool) {
    let prefs = p.to_prefs(src.len());
    let bound = frame_bound(src.len(), &prefs as *const LZ4F_preferences_t);
    assert!(
        !lz4f_is_error(bound) && bound < 64 << 20,
        "{}: unexpected bound {} for {}",
        label,
        bound,
        p
    );

    let ctx = format!("{} [{}] srcSize={} cap=bound({})", label, p, src.len(), bound);
    let (n, buf) = frame_case(&ctx, &prefs, src, bound);
    assert!(!lz4f_is_error(n), "{}: compressFrame failed: {}", ctx, err_str(n));

    if also_larger_dst {
        // A larger destination must produce exactly the same bytes and length.
        let ctx2 = format!("{} [{}] srcSize={} cap=bound+37", label, p, src.len());
        let (n2, buf2) = frame_case(&ctx2, &prefs, src, bound + 37);
        assert_eq!(n, n2, "{}: length changed with a larger dst", ctx2);
        assert_bytes_eq(&format!("{} (vs exact-bound output)", ctx2), &buf[..n], &buf2[..n2]);
    }

    if do_round_trip {
        round_trip_both(&ctx, &buf[..n], src);
    }
}

fn frame_good(label: &str, p: &P, src: &[u8], do_round_trip: bool) {
    frame_good_opts(label, p, src, do_round_trip, true)
}

// ===========================================================================
// Metadata / trivial accessors
// ===========================================================================

#[test]
fn version_and_compression_level_max() {
    let (cv, rv) = both::<FnGetVersion>("LZ4F_getVersion");
    let c = unsafe { cv() };
    let r = unsafe { rv() };
    assert_eq!(c, r, "LZ4F_getVersion: C={} Rust={}", c, r);
    assert_eq!(c, LZ4F_VERSION, "LZ4F_getVersion must be LZ4F_VERSION (100)");

    let (cl, rl) = both::<FnLevelMax>("LZ4F_compressionLevel_max");
    let c = unsafe { cl() };
    let r = unsafe { rl() };
    assert_eq!(c, r, "LZ4F_compressionLevel_max: C={} Rust={}", c, r);
    assert_eq!(
        c, LZ4HC_CLEVEL_MAX,
        "LZ4F_compressionLevel_max must be LZ4HC_CLEVEL_MAX (12)"
    );

    // Calling repeatedly must be stable (no hidden state).
    for _ in 0..8 {
        assert_eq!(unsafe { cv() }, unsafe { rv() });
        assert_eq!(unsafe { cl() }, unsafe { rl() });
    }
}

// ===========================================================================
// LZ4F_getBlockSize — ERRORS.md rows 168 / 169
// ===========================================================================

#[test]
fn get_block_size_every_id_and_random() {
    let (c, r) = both::<FnGetBlockSize>("LZ4F_getBlockSize");

    let mut ids: Vec<c_int> = (-8..=16).collect();
    ids.push(c_int::MIN);
    ids.push(c_int::MIN + 1);
    ids.push(c_int::MAX);
    ids.push(0x7FFF_FFFF);
    ids.push(-0x7FFF_FFFF);

    for &id in &ids {
        let cv = unsafe { c(id) };
        let rv = unsafe { r(id) };
        assert_eq!(cv, rv, "LZ4F_getBlockSize({}): C={} Rust={}", id, cv, rv);

        // Documented semantics, pinned in both libraries.
        let expect_ok = matches!(id, 0 | 4 | 5 | 6 | 7);
        if expect_ok {
            let want = match id {
                0 | 4 => 64 * 1024,
                5 => 256 * 1024,
                6 => 1024 * 1024,
                _ => 4 * 1024 * 1024,
            };
            assert_eq!(cv, want, "LZ4F_getBlockSize({}) should be {}", id, want);
            assert!(!lz4f_is_error(cv), "LZ4F_getBlockSize({}) must not error", id);
        } else {
            // rows 168 (1..3) and 169 (>=8, and every negative value)
            assert!(
                lz4f_is_error(cv),
                "LZ4F_getBlockSize({}) must be an error, got {}",
                id,
                cv
            );
            assert_eq!(
                lz4f_error_code(cv),
                err::ERROR_maxBlockSize_invalid,
                "LZ4F_getBlockSize({}) must be maxBlockSize_invalid(2), got {}",
                id,
                lz4f_error_code(cv)
            );
            assert_eq!(
                lz4f_error_code(rv),
                err::ERROR_maxBlockSize_invalid,
                "Rust LZ4F_getBlockSize({}) must be maxBlockSize_invalid(2)",
                id
            );
        }
    }

    // Randomised ints — a C enum parameter accepts any int value.
    let mut rng = Rng::new(0x0B10_C551_2E00_1D01);
    for _ in 0..20000 {
        let id = rng.next_u32() as c_int;
        let cv = unsafe { c(id) };
        let rv = unsafe { r(id) };
        assert_eq!(cv, rv, "LZ4F_getBlockSize({}): C={} Rust={}", id, cv, rv);
    }
    // Small-magnitude randoms, which is where the interesting boundary lies.
    for _ in 0..20000 {
        let id = (rng.below(64) as c_int) - 32;
        let cv = unsafe { c(id) };
        let rv = unsafe { r(id) };
        assert_eq!(cv, rv, "LZ4F_getBlockSize({}): C={} Rust={}", id, cv, rv);
    }
}

// ===========================================================================
// LZ4F_isError / LZ4F_getErrorCode / LZ4F_getErrorName
// ===========================================================================

fn error_probe_values(rng: &mut Rng) -> Vec<usize> {
    let mut v: Vec<usize> = vec![0, 1, 100, usize::MAX, usize::MAX / 2, usize::MAX - 1];
    for code in 0..=24usize {
        v.push(code); // raw positive value
        v.push(0usize.wrapping_sub(code)); // -(size_t)code
    }
    v.push(0usize.wrapping_sub(25));
    v.push(0usize.wrapping_sub(100));
    v.push(0usize.wrapping_sub(0x7FFF_FFFF));
    v.push(1usize << 63);
    v.push((1usize << 63) | 1);
    for _ in 0..3000 {
        v.push(rng.next_u64() as usize);
        // biased towards the error region (just below 2^64)
        v.push(usize::MAX - rng.below(64));
    }
    v
}

#[test]
fn is_error_and_get_error_code() {
    let (ci, ri) = both::<FnIsError>("LZ4F_isError");
    let (cc, rc) = both::<FnGetErrorCode>("LZ4F_getErrorCode");
    let mut rng = Rng::new(0x1E22_0BAD_C0DE_4F1A);
    let values = error_probe_values(&mut rng);

    for &v in &values {
        let cis = unsafe { ci(v) };
        let ris = unsafe { ri(v) };
        assert_eq!(
            cis, ris,
            "LZ4F_isError({:#x}): C={} Rust={}",
            v, cis, ris
        );
        let cec = unsafe { cc(v) };
        let rec = unsafe { rc(v) };
        assert_eq!(
            cec, rec,
            "LZ4F_getErrorCode({:#x}): C={} Rust={}",
            v, cec, rec
        );
        // Cross-check against the harness's own re-implementation.
        assert_eq!(
            cis != 0,
            lz4f_is_error(v),
            "harness lz4f_is_error disagrees with C for {:#x}",
            v
        );
        assert_eq!(
            cec,
            lz4f_error_code(v),
            "harness lz4f_error_code disagrees with C for {:#x}",
            v
        );
    }

    // Pin the documented boundary: -(size_t)LZ4F_ERROR_maxCode is NOT an error,
    // -(size_t)(maxCode-1) is.
    assert_eq!(unsafe { ci(0usize.wrapping_sub(24)) }, 0);
    assert_eq!(unsafe { ri(0usize.wrapping_sub(24)) }, 0);
    assert_ne!(unsafe { ci(0usize.wrapping_sub(23)) }, 0);
    assert_ne!(unsafe { ri(0usize.wrapping_sub(23)) }, 0);
    assert_eq!(unsafe { cc(0usize.wrapping_sub(11)) }, err::ERROR_dstMaxSize_tooSmall);
    assert_eq!(unsafe { rc(0usize.wrapping_sub(11)) }, err::ERROR_dstMaxSize_tooSmall);
}

#[test]
fn get_error_name_bytes_identical() {
    let (cn, rn) = both::<FnGetErrorName>("LZ4F_getErrorName");
    let mut rng = Rng::new(0x4E44_4D45_5F00_0001);
    let mut values = error_probe_values(&mut rng);
    // A few more explicitly out-of-range codes.
    for extra in [
        0usize.wrapping_sub(30),
        0usize.wrapping_sub(1000),
        1234567,
        usize::MAX / 3,
    ] {
        values.push(extra);
    }

    for &v in &values {
        let cp = unsafe { cn(v) };
        let rp = unsafe { rn(v) };
        assert!(!cp.is_null(), "C LZ4F_getErrorName({:#x}) returned NULL", v);
        assert!(!rp.is_null(), "Rust LZ4F_getErrorName({:#x}) returned NULL", v);
        let cb = unsafe { CStr::from_ptr(cp) }.to_bytes();
        let rb = unsafe { CStr::from_ptr(rp) }.to_bytes();
        assert_bytes_eq(
            &format!("LZ4F_getErrorName({:#x}) [error code {}]", v, lz4f_error_code(v)),
            cb,
            rb,
        );
    }

    // Spot-check a couple of concrete strings, so an "both return the same wrong
    // thing" regression in the table is still visible.
    let name = |v: usize| -> Vec<u8> {
        unsafe { CStr::from_ptr(cn(v)) }.to_bytes().to_vec()
    };
    assert_eq!(name(0), b"Unspecified error code".to_vec());
    assert_eq!(
        name(0usize.wrapping_sub(err::ERROR_maxBlockSize_invalid as usize)),
        b"ERROR_maxBlockSize_invalid".to_vec()
    );
    assert_eq!(
        name(0usize.wrapping_sub(err::ERROR_dstMaxSize_tooSmall as usize)),
        b"ERROR_dstMaxSize_tooSmall".to_vec()
    );
    assert_eq!(
        name(0usize.wrapping_sub(err::ERROR_frameType_unknown as usize)),
        b"ERROR_frameType_unknown".to_vec()
    );
}

// ===========================================================================
// LZ4F_compressFrameBound / LZ4F_compressBound
// ===========================================================================

const BOUND_SIZES: [usize; 12] = [
    0, 1, 2, 15, 16, 65535, 65536, 65537, 262144, 1048576, 4194304, 4194305,
];

#[test]
fn bounds_null_prefs_and_full_cross_product() {
    let (cfb, rfb) = both::<FnBound>("LZ4F_compressFrameBound");
    let (ccb, rcb) = both::<FnBound>("LZ4F_compressBound");

    // --- NULL prefs (documented as "worst case") -------------------------
    let mut sizes: Vec<usize> = BOUND_SIZES.to_vec();
    sizes.extend_from_slice(&[
        3,
        7,
        4096,
        65534,
        131072,
        16 * 1024 * 1024,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 3,
    ]);
    for &s in &sizes {
        let a = unsafe { cfb(s, ptr::null()) };
        let b = unsafe { rfb(s, ptr::null()) };
        assert_eq!(a, b, "LZ4F_compressFrameBound({}, NULL): C={} Rust={}", s, a, b);
        let a = unsafe { ccb(s, ptr::null()) };
        let b = unsafe { rcb(s, ptr::null()) };
        assert_eq!(a, b, "LZ4F_compressBound({}, NULL): C={} Rust={}", s, a, b);
    }

    // --- full documented cross product ----------------------------------
    let levels: [c_int; 11] = [c_int::MIN, -1, 0, 1, 2, 3, 9, 10, 12, 13, c_int::MAX];
    let mut combos = 0usize;
    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &ccs in &[0, 1] {
                for &bcs in &[0, 1] {
                    for &af in &[0u32, 1u32] {
                        for &use_cs in &[false, true] {
                            for &did in &[0u32, 0xDEAD_BEEFu32] {
                                for &lvl in &levels {
                                    combos += 1;
                                    let p = P {
                                        bsid,
                                        bmode,
                                        ccs,
                                        bcs,
                                        ftype: LZ4F_frame,
                                        autoflush: af,
                                        csize: use_cs,
                                        dict_id: did,
                                        level: lvl,
                                        favor: 0,
                                    };
                                    for &s in BOUND_SIZES.iter() {
                                        let prefs = p.to_prefs(s);
                                        let pp = &prefs as *const LZ4F_preferences_t;
                                        let a = unsafe { cfb(s, pp) };
                                        let b = unsafe { rfb(s, pp) };
                                        if a != b {
                                            panic!(
                                                "LZ4F_compressFrameBound({}, {{{}}}): C={} Rust={}",
                                                s, p, a, b
                                            );
                                        }
                                        let a = unsafe { ccb(s, pp) };
                                        let b = unsafe { rcb(s, pp) };
                                        if a != b {
                                            panic!(
                                                "LZ4F_compressBound({}, {{{}}}): C={} Rust={}",
                                                s, p, a, b
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
    }
    assert_eq!(combos, 5 * 2 * 2 * 2 * 2 * 2 * 2 * 11, "cross-product size");
}

#[test]
fn bounds_randomised_including_out_of_range_enums_and_huge_sizes() {
    let (cfb, rfb) = both::<FnBound>("LZ4F_compressFrameBound");
    let (ccb, rcb) = both::<FnBound>("LZ4F_compressBound");
    let mut rng = Rng::new(0x0B0D_0F11_1315_1719);

    for i in 0..30000usize {
        // Deliberately include out-of-range enum values: a C enum parameter (here
        // reached through a struct field) accepts any int.
        let bsid = match rng.below(8) {
            0 => 0,
            1 => 4,
            2 => 5,
            3 => 6,
            4 => 7,
            5 => rng.range(1, 3) as c_int,
            6 => rng.range(8, 40) as c_int,
            _ => -(rng.range(1, 40) as c_int),
        };
        let p = P {
            bsid,
            bmode: rng.below(4) as c_int - 1,
            ccs: rng.below(4) as c_int - 1,
            bcs: rng.below(4) as c_int - 1,
            ftype: rng.below(4) as c_int - 1,
            autoflush: rng.below(3) as u32,
            csize: rng.bool(),
            dict_id: rng.next_u32(),
            level: rng.next_u32() as c_int,
            favor: rng.below(2) as u32,
        };
        let s = match i % 5 {
            0 => rng.below(1 << 22),
            1 => rng.below(1 << 12),
            2 => rng.next_u64() as usize,
            3 => usize::MAX - rng.below(1 << 20),
            _ => rng.below(1 << 18) * 65536,
        };
        let prefs = p.to_prefs(s);
        let pp = &prefs as *const LZ4F_preferences_t;
        let a = unsafe { cfb(s, pp) };
        let b = unsafe { rfb(s, pp) };
        if a != b {
            panic!("LZ4F_compressFrameBound({}, {{{}}}): C={} Rust={}", s, p, a, b);
        }
        let a = unsafe { ccb(s, pp) };
        let b = unsafe { rcb(s, pp) };
        if a != b {
            panic!("LZ4F_compressBound({}, {{{}}}): C={} Rust={}", s, p, a, b);
        }
    }
}

// ===========================================================================
// LZ4F_compressFrame — every blockSizeID x blockMode x checksum combination
// ===========================================================================

/// A source size that makes `LZ4F_optimalBSID()` actually *keep* the requested
/// blockSizeID (and produce more than one block, exercising blockMode).
fn size_for_bsid(bsid: c_int) -> usize {
    match bsid {
        LZ4F_default | LZ4F_max64KB => 70_000,   // 2 x 64 KB blocks
        LZ4F_max256KB => 300_000,                // 2 x 256 KB blocks
        LZ4F_max1MB => 1_200_000,                // 2 x 1 MB blocks
        _ => 5_000_000,                          // 2 x 4 MB blocks
    }
}

/// The complete documented preference cross product
/// (blockSizeID x blockMode x contentChecksum x blockChecksum x autoFlush x
///  contentSize x dictID x compressionLevel = 7040 combinations), at a small
/// source size so that every combination is affordable. Data shapes rotate so all
/// `N_SHAPES` appear against every axis value.
#[test]
fn compress_frame_full_preference_cross_product() {
    let levels: [c_int; 11] = [c_int::MIN, -1, 0, 1, 2, 3, 9, 10, 12, 13, c_int::MAX];
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    let mut idx = 0usize;
    // Six pre-generated sources of the same length, one per shape.
    let len = 2000usize;
    let srcs: Vec<Vec<u8>> = (0..N_SHAPES).map(|s| gen_src(&mut rng, s, len)).collect();

    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &ccs in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
                for &bcs in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                    for &af in &[0u32, 1u32] {
                        for &use_cs in &[false, true] {
                            for &did in &[0u32, 0xDEAD_BEEFu32] {
                                for &level in &levels {
                                    let shape = idx % N_SHAPES;
                                    let p = P {
                                        bsid,
                                        bmode,
                                        ccs,
                                        bcs,
                                        ftype: if idx % 7 == 0 {
                                            LZ4F_skippableFrame
                                        } else {
                                            LZ4F_frame
                                        },
                                        autoflush: af,
                                        csize: use_cs,
                                        dict_id: did,
                                        level,
                                        favor: ((idx / 11) % 2) as u32,
                                    };
                                    frame_good_opts(
                                        &format!(
                                            "full-xprod #{} shape={}",
                                            idx,
                                            shape_name(shape)
                                        ),
                                        &p,
                                        &srcs[shape],
                                        true,
                                        idx % 4 == 0,
                                    );
                                    idx += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(idx, 5 * 2 * 2 * 2 * 2 * 2 * 2 * 11, "7040 combinations");
}

/// Same complete cross product, but at a source size large enough to produce
/// several blocks (so blockMode / autoFlush / block checksums actually apply to
/// more than one block). Levels are trimmed to keep the runtime bounded, while
/// still covering LZ4-fast, LZ4HC-mid, LZ4HC-hashChain and LZ4HC-optimal.
#[test]
fn compress_frame_multiblock_preference_cross_product() {
    let levels: [c_int; 4] = [-1, 1, 2, 9];
    let mut rng = Rng::new(0xB10C_C0DE_0BAD_F00D);
    let mut idx = 0usize;
    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB] {
        let len = if bsid == LZ4F_max256KB { 300_000 } else { 70_000 };
        let srcs: Vec<Vec<u8>> = (0..N_SHAPES).map(|s| gen_src(&mut rng, s, len)).collect();
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &ccs in &[0, 1] {
                for &bcs in &[0, 1] {
                    for &af in &[0u32, 1u32] {
                        for &use_cs in &[false, true] {
                            for &did in &[0u32, 0xDEAD_BEEFu32] {
                                for &level in &levels {
                                    let shape = idx % N_SHAPES;
                                    let p = P {
                                        bsid,
                                        bmode,
                                        ccs,
                                        bcs,
                                        ftype: LZ4F_frame,
                                        autoflush: af,
                                        csize: use_cs,
                                        dict_id: did,
                                        level,
                                        favor: ((idx / 4) % 2) as u32,
                                    };
                                    frame_good_opts(
                                        &format!(
                                            "multiblock-xprod #{} shape={}",
                                            idx,
                                            shape_name(shape)
                                        ),
                                        &p,
                                        &srcs[shape],
                                        true,
                                        false,
                                    );
                                    idx += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(idx, 3 * 2 * 2 * 2 * 2 * 2 * 2 * 4);
}

#[test]
fn compress_frame_blocksize_blockmode_checksum_cross_product() {
    let mut rng = Rng::new(0xB10C_5A1E_C0DE_0001);
    let mut idx = 0usize;
    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
        let len = size_for_bsid(bsid);
        // Verify our block-size derivation goes through the symbol under test.
        let bs = block_size(if bsid == 0 { LZ4F_max64KB } else { bsid });
        assert!(len > bs, "size_for_bsid({}) must span >1 block of {}", bsid, bs);

        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &ccs in &[LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled] {
                for &bcs in &[LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled] {
                    let shape = idx % N_SHAPES;
                    let src = gen_src(&mut rng, shape, len);
                    let p = P {
                        bsid,
                        bmode,
                        ccs,
                        bcs,
                        ftype: LZ4F_frame,
                        autoflush: (idx % 2) as u32,
                        csize: idx % 3 == 0,
                        dict_id: if idx % 4 == 0 { 0xDEAD_BEEF } else { 0 },
                        level: 1,
                        favor: 0,
                    };
                    frame_good(
                        &format!("cross-product #{} shape={}", idx, shape_name(shape)),
                        &p,
                        &src,
                        true,
                    );
                    idx += 1;
                }
            }
        }
    }
    assert_eq!(idx, 5 * 2 * 2 * 2, "40 blockSizeID x blockMode x checksum combos");
}

// ===========================================================================
// LZ4F_compressFrame — block-size boundary source sizes x every data shape
// ===========================================================================

#[test]
fn compress_frame_block_boundaries_64kb_all_shapes() {
    let bs = block_size(LZ4F_max64KB);
    assert_eq!(bs, 65536);
    let sizes = [0usize, 1, 2, 15, bs - 1, bs, bs + 1, 2 * bs + 7, 5 * bs];
    let mut rng = Rng::new(0x6400_0BAD_0000_0001);
    let mut idx = 0usize;
    for shape in 0..N_SHAPES {
        for &len in &sizes {
            for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
                let src = gen_src(&mut rng, shape, len);
                let p = P {
                    bsid: LZ4F_max64KB,
                    bmode,
                    ccs: (idx % 2) as c_int,
                    bcs: ((idx / 2) % 2) as c_int,
                    ftype: LZ4F_frame,
                    autoflush: (idx % 2) as u32,
                    csize: idx % 3 == 0,
                    dict_id: if idx % 5 == 0 { 0x1234_5678 } else { 0 },
                    level: if idx % 2 == 0 { 1 } else { 0 },
                    favor: 0,
                };
                frame_good(
                    &format!("64KB-boundary shape={}", shape_name(shape)),
                    &p,
                    &src,
                    true,
                );
                idx += 1;
            }
        }
    }
}

/// Dense source-size sweep: every size 0..=80, and every size straddling the
/// 64 KB / 128 KB block boundaries. Catches off-by-one differences in the
/// block-splitting arithmetic that a coarse sweep would miss.
#[test]
fn compress_frame_dense_small_and_boundary_size_sweep() {
    let bs = block_size(LZ4F_max64KB);
    let mut sizes: Vec<usize> = (0..=80usize).collect();
    sizes.extend((bs - 8)..=(bs + 8));
    sizes.extend((2 * bs - 8)..=(2 * bs + 8));
    sizes.extend([bs / 2, bs - 1, bs, bs + 1, 3 * bs, 3 * bs + 1]);

    let psets = [
        P { bsid: LZ4F_max64KB, bmode: LZ4F_blockLinked, level: 1, autoflush: 1, ..P::base() },
        P {
            bsid: LZ4F_max64KB,
            bmode: LZ4F_blockIndependent,
            ccs: 1,
            bcs: 1,
            csize: true,
            dict_id: 0xDEAD_BEEF,
            level: 2,
            autoflush: 0,
            ..P::base()
        },
        P { bsid: LZ4F_default, bmode: LZ4F_blockLinked, ccs: 1, level: -5, ..P::base() },
    ];

    let mut rng = Rng::new(0x0DE5_5E00_9ABC_DEF0);
    let mut idx = 0usize;
    for (pi, p) in psets.iter().enumerate() {
        for &len in &sizes {
            let shape = idx % N_SHAPES;
            let src = gen_src(&mut rng, shape, len);
            frame_good_opts(
                &format!("dense pset#{} len={} shape={}", pi, len, shape_name(shape)),
                p,
                &src,
                true,
                len < 4096,
            );
            idx += 1;
        }
    }
}

#[test]
fn compress_frame_block_boundaries_large_ids() {
    let mut rng = Rng::new(0x1A26_4B10_C51A_2E55);
    // Large block sizes are expensive, so use the fast compression level and a
    // rotating (but complete over the test file) selection of shapes.
    for (i, &bsid) in [LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB].iter().enumerate() {
        let bs = block_size(bsid);
        let sizes = [bs - 1, bs, bs + 1, 2 * bs + 7];
        let shapes: &[usize] = match bsid {
            LZ4F_max256KB => &[0, 1, 2, 3, 4, 5],
            LZ4F_max1MB => &[0, 2, 3, 5],
            _ => &[1, 4],
        };
        for (j, &len) in sizes.iter().enumerate() {
            for &shape in shapes {
                let src = gen_src(&mut rng, shape, len);
                let p = P {
                    bsid,
                    bmode: if (i + j) % 2 == 0 { LZ4F_blockLinked } else { LZ4F_blockIndependent },
                    ccs: ((i + j) % 2) as c_int,
                    bcs: ((i + j + 1) % 2) as c_int,
                    ftype: LZ4F_frame,
                    autoflush: 1,
                    csize: (i + j) % 3 == 0,
                    dict_id: 0,
                    level: 1,
                    favor: 0,
                };
                frame_good(
                    &format!("bsid={} boundary shape={}", bsid, shape_name(shape)),
                    &p,
                    &src,
                    true,
                );
            }
        }
    }
}

// ===========================================================================
// LZ4F_compressFrame — every compression-level regime
// ===========================================================================

#[test]
fn compress_frame_all_compression_level_regimes() {
    // LZ4HC_CLEVEL_MIN == 2 in this tree: level 1 is still LZ4-fast, level 2 is
    // already LZ4HC (lz4mid), 3..9 hashChain, 10..12 optimal, >12 clamped.
    let levels: [c_int; 14] = [
        c_int::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        5,
        9,
        10,
        11,
        12,
        13,
        c_int::MAX,
    ];
    let sizes = [0usize, 1, 15, 4096, 40_000, 100_000];
    let mut rng = Rng::new(0x7EFE_1233_4455_6677);
    let mut idx = 0usize;
    for &level in &levels {
        for shape in 0..N_SHAPES {
            for &len in &sizes {
                let src = gen_src(&mut rng, shape, len);
                let p = P {
                    bsid: LZ4F_max64KB,
                    bmode: if idx % 2 == 0 { LZ4F_blockLinked } else { LZ4F_blockIndependent },
                    ccs: (idx % 2) as c_int,
                    bcs: ((idx / 3) % 2) as c_int,
                    ftype: LZ4F_frame,
                    autoflush: (idx % 2) as u32,
                    csize: idx % 2 == 1,
                    dict_id: if idx % 3 == 0 { 0xABCD_0123 } else { 0 },
                    level,
                    favor: ((idx / 2) % 2) as u32,
                };
                frame_good(
                    &format!("level={} shape={} len={}", level, shape_name(shape), len),
                    &p,
                    &src,
                    true,
                );
                idx += 1;
            }
        }
    }
    assert_eq!(idx, levels.len() * N_SHAPES * sizes.len());
}

// ===========================================================================
// LZ4F_compressFrame — frameType, favorDecSpeed, randomised sizes
// ===========================================================================

#[test]
fn compress_frame_frametype_favor_and_random_sizes() {
    let mut rng = Rng::new(0x5DEE_CE66_D0FA_1E77);
    // frameType is *not* encoded by the compressor (the FLG byte carries only
    // blockMode/checksums/contentSize/dictID) — both libraries must therefore
    // produce identical bytes for LZ4F_frame and LZ4F_skippableFrame.
    for &ftype in &[LZ4F_frame, LZ4F_skippableFrame] {
        for &favor in &[0u32, 1u32] {
            for &level in &[1, 2, 10] {
                let src = gen_src(&mut rng, 4, 50_000);
                let p = P {
                    bsid: LZ4F_max64KB,
                    bmode: LZ4F_blockIndependent,
                    ccs: 1,
                    bcs: 1,
                    ftype,
                    autoflush: 1,
                    csize: true,
                    dict_id: 7,
                    level,
                    favor,
                };
                frame_good(
                    &format!("frameType={} favor={} level={}", ftype, favor, level),
                    &p,
                    &src,
                    true,
                );
            }
        }
    }

    // Randomised sizes / shapes / preference mixes.
    for i in 0..600usize {
        let shape = i % N_SHAPES;
        let len = match i % 6 {
            0 => rng.below(64),
            1 => rng.range(60_000, 70_000),
            2 => rng.below(4096),
            3 => rng.range(130_000, 200_000),
            4 => rng.range(1, 40_000),
            _ => rng.range(250_000, 280_000),
        };
        let src = gen_src(&mut rng, shape, len);
        let p = P {
            bsid: [LZ4F_default, LZ4F_max64KB, LZ4F_max256KB][i % 3],
            bmode: (i % 2) as c_int,
            ccs: ((i / 2) % 2) as c_int,
            bcs: ((i / 4) % 2) as c_int,
            ftype: ((i / 8) % 2) as c_int,
            autoflush: ((i / 3) % 2) as u32,
            csize: i % 3 == 0,
            dict_id: if i % 4 == 0 { rng.next_u32() } else { 0 },
            level: [-3, 0, 1, 2, 4, 9, 11][i % 7],
            favor: ((i / 5) % 2) as u32,
        };
        frame_good(
            &format!("random#{} shape={} len={}", i, shape_name(shape), len),
            &p,
            &src,
            true,
        );
    }
}

// ===========================================================================
// ERRORS.md row 170 — dstCapacity < LZ4F_compressFrameBound
// ===========================================================================

#[test]
fn row_170_compress_frame_dst_too_small() {
    let mut rng = Rng::new(0x1701_7017_0170_1701);

    let cases: Vec<(P, usize)> = vec![
        (P::base(), 0),
        (P::base(), 100),
        (
            P {
                ccs: 1,
                bcs: 1,
                csize: true,
                dict_id: 0xDEAD_BEEF,
                ..P::base()
            },
            100,
        ),
        (
            P {
                bsid: LZ4F_max256KB,
                bmode: LZ4F_blockIndependent,
                level: 2,
                ..P::base()
            },
            300,
        ),
        (
            P {
                bsid: LZ4F_max64KB,
                bmode: LZ4F_blockLinked,
                ccs: 1,
                level: -2,
                ..P::base()
            },
            70_000,
        ),
    ];

    for (ci, (p, len)) in cases.iter().enumerate() {
        let src = gen_src(&mut rng, ci % N_SHAPES, *len);
        let prefs = p.to_prefs(src.len());
        let bound = frame_bound(src.len(), &prefs as *const LZ4F_preferences_t);
        assert!(bound > 0 && !lz4f_is_error(bound));

        // Sweep every capacity below the bound when that is cheap, otherwise a
        // dense sample that still includes 0 and bound-1.
        let caps: Vec<usize> = if bound <= 600 {
            (0..bound).collect()
        } else {
            let mut v: Vec<usize> = vec![0, 1, 2, 18, 19, 20, 22, 23, 26, 27];
            v.push(bound / 4);
            v.push(bound / 2);
            v.push(bound - 3);
            v.push(bound - 2);
            v.push(bound - 1);
            v.retain(|&c| c < bound);
            v
        };

        for &cap in &caps {
            let ctx = format!(
                "row170 case#{} [{}] srcSize={} cap={} bound={}",
                ci, p, src.len(), cap, bound
            );
            let (n, buf) = frame_case(&ctx, &prefs, &src, cap);
            assert!(lz4f_is_error(n), "{}: expected an error, got {}", ctx, n);
            assert_eq!(
                lz4f_error_code(n),
                err::ERROR_dstMaxSize_tooSmall,
                "{}: expected dstMaxSize_tooSmall(11), got {}",
                ctx,
                lz4f_error_code(n)
            );
            // Nothing may be written into dst on this error path.
            assert!(
                buf.iter().all(|&b| b == SENTINEL),
                "{}: dst was modified on the dstMaxSize_tooSmall path",
                ctx
            );
        }

        // Exactly the bound must succeed.
        let ctx = format!("row170 case#{} exact bound {}", ci, bound);
        let (n, _) = frame_case(&ctx, &prefs, &src, bound);
        assert!(!lz4f_is_error(n), "{}: exact bound must succeed", ctx);
    }

    // Smallest documented reproduction: srcSize == 0, dstCapacity == 0.
    let empty = gen_src(&mut rng, 0, 0);
    let (n, _) = frame_raw(
        "row170 minimal (srcSize=0, cap=0, NULL prefs)",
        ptr::null(),
        empty.as_ptr() as *const c_void,
        0,
        0,
    );
    assert_eq!(lz4f_error_code(n), err::ERROR_dstMaxSize_tooSmall);
}

// ===========================================================================
// Generic boundaries: NULL prefs, NULL src, out-of-range enum values
// ===========================================================================

#[test]
fn compress_frame_null_prefs_and_null_src() {
    let mut rng = Rng::new(0x4E17_4C15_0B0B_0B0B);

    // NULL prefsPtr is documented as valid (all preferences = 0 / defaults).
    for &len in &[0usize, 1, 2, 15, 4096, 70_000, 300_000] {
        for shape in 0..N_SHAPES {
            let src = gen_src(&mut rng, shape, len);
            let bound = frame_bound(len, ptr::null());
            let ctx = format!("NULL prefs len={} shape={}", len, shape_name(shape));
            let (n, buf) = frame_raw(
                &ctx,
                ptr::null(),
                src.as_ptr() as *const c_void,
                len,
                bound,
            );
            assert!(!lz4f_is_error(n), "{}: {}", ctx, err_str(n));
            round_trip_both(&ctx, &buf[..n], &src);

            // NULL prefs must match an explicitly all-zero preferences struct
            // (that is exactly what the C MEM_INITs internally).
            let zero = LZ4F_preferences_t {
                frameInfo: LZ4F_frameInfo_t {
                    blockSizeID: 0,
                    blockMode: 0,
                    contentChecksumFlag: 0,
                    frameType: 0,
                    contentSize: 0,
                    dictID: 0,
                    blockChecksumFlag: 0,
                },
                compressionLevel: 0,
                autoFlush: 0,
                favorDecSpeed: 0,
                reserved: [0; 3],
            };
            let (n2, buf2) = frame_case(
                &format!("{} (explicit zeroed prefs)", ctx),
                &zero,
                &src,
                bound,
            );
            assert_eq!(n, n2, "{}: NULL prefs != zeroed prefs (length)", ctx);
            assert_bytes_eq(
                &format!("{}: NULL prefs vs zeroed prefs", ctx),
                &buf[..n],
                &buf2[..n2],
            );
        }
    }

    // NULL src with srcSize == 0.
    for p in [
        P::base(),
        P { ccs: 1, ..P::base() },
        P { ccs: 1, bcs: 1, dict_id: 9, level: 5, ..P::base() },
    ] {
        let prefs = p.to_prefs(0);
        let bound = frame_bound(0, &prefs as *const LZ4F_preferences_t);
        let ctx = format!("NULL src, srcSize=0 [{}]", p);
        let (n, buf) = frame_raw(&ctx, &prefs as *const LZ4F_preferences_t, ptr::null(), 0, bound);
        assert!(!lz4f_is_error(n), "{}: {}", ctx, err_str(n));
        round_trip_both(&ctx, &buf[..n], &[]);
    }
    // NULL src, srcSize 0, NULL prefs.
    let b = frame_bound(0, ptr::null());
    let (n, buf) = frame_raw("NULL src + NULL prefs", ptr::null(), ptr::null(), 0, b);
    assert!(!lz4f_is_error(n));
    round_trip_both("NULL src + NULL prefs", &buf[..n], &[]);
}

#[test]
fn compress_frame_out_of_range_enum_values() {
    // C enums accept any int across the FFI. The compress side masks blockMode /
    // checksum flags with `& 1` when writing the FLG byte and does NOT validate
    // them, so C and Rust must simply agree on the resulting frame bytes.
    // (No round-trip here: e.g. blockChecksumFlag==2 emits a block checksum that
    // the FLG byte does not advertise, which is an intentionally malformed frame.)
    let mut rng = Rng::new(0x0E4F_0E4F_0E4F_0E4F);
    let mut checked = 0usize;

    let odd: [c_int; 6] = [-2, -1, 2, 3, 0x7F, c_int::MAX];
    for (i, &v) in odd.iter().enumerate() {
        for field in 0..5usize {
            let mut p = P {
                bsid: LZ4F_max64KB,
                bmode: LZ4F_blockLinked,
                ccs: 0,
                bcs: 0,
                ftype: LZ4F_frame,
                autoflush: 1,
                csize: i % 2 == 0,
                dict_id: 0,
                level: 1,
                favor: 0,
            };
            match field {
                0 => p.bsid = v,
                1 => p.bmode = v,
                2 => p.ccs = v,
                3 => p.bcs = v,
                _ => p.ftype = v,
            }
            for &len in &[0usize, 1, 3000, 70_000] {
                let src = gen_src(&mut rng, (i + field) % N_SHAPES, len);
                let prefs = p.to_prefs(len);
                let bound = frame_bound(len, &prefs as *const LZ4F_preferences_t);
                // An out-of-range blockSizeID makes LZ4F_getBlockSize() return an
                // error sentinel, which propagates into an astronomically large
                // bound. In that case only the (identical) error return can be
                // compared, using a small capacity.
                let cap = if lz4f_is_error(bound) || bound > (64 << 20) {
                    4096
                } else {
                    bound
                };
                let ctx = format!(
                    "out-of-range field#{}={} [{}] len={} cap={}",
                    field, v, p, len, cap
                );
                let (n, buf) = frame_case(&ctx, &prefs, &src, cap);
                checked += 1;
                // Well-formed configurations (only the ignored frameType, or a
                // masked blockMode with a single block) still round-trip; we do
                // not assume which, we only require C/Rust agreement, already
                // asserted inside frame_case. Sanity: a success must fit.
                if !lz4f_is_error(n) {
                    assert!(n <= cap, "{}: length {} > cap {}", ctx, n, cap);
                    assert!(buf.len() >= n);
                }
            }
        }
    }
    assert!(checked >= 100, "expected a decent sweep, got {}", checked);

    // Explicitly check the two documented masking behaviours produce the SAME
    // header byte in both libraries for blockMode 3 (== 1 after `& 1`).
    for (a, b) in [(1, 3), (0, 2), (0, -2), (1, -1)] {
        let len = 3000usize;
        let src = gen_src(&mut rng, 3, len);
        let pa = P { bsid: LZ4F_max64KB, bmode: a, autoflush: 1, level: 1, ..P::base() };
        let pb = P { bsid: LZ4F_max64KB, bmode: b, autoflush: 1, level: 1, ..P::base() };
        let (pra, prb) = (pa.to_prefs(len), pb.to_prefs(len));
        let bound = frame_bound(len, &pra as *const LZ4F_preferences_t);
        let (na, ba) = frame_case(&format!("mask blockMode={}", a), &pra, &src, bound);
        let (nb, bb) = frame_case(&format!("mask blockMode={}", b), &prb, &src, bound);
        assert!(!lz4f_is_error(na) && !lz4f_is_error(nb));
        // FLG byte lives at offset 4; only bit 5 encodes blockMode.
        assert_eq!(
            ba[4] & 0x20,
            bb[4] & 0x20,
            "blockMode {} vs {} must give the same FLG bit 5",
            a,
            b
        );
        assert_eq!(na, nb, "blockMode {} vs {}: header length differs", a, b);
    }
}

// ===========================================================================
// CDict path
// ===========================================================================

struct CdictApi {
    create: FnCreateCDict,
    create_adv: FnCreateCDictAdv,
    free: FnFreeCDict,
    compress: FnCompressFrameCDict,
    cctx_new: FnCreateCctx,
    cctx_free: FnFreeCctx,
    tag: &'static str,
}

fn cdict_apis() -> (CdictApi, CdictApi) {
    let (c1, r1) = both::<FnCreateCDict>("LZ4F_createCDict");
    let (c2, r2) = both::<FnCreateCDictAdv>("LZ4F_createCDict_advanced");
    let (c3, r3) = both::<FnFreeCDict>("LZ4F_freeCDict");
    let (c4, r4) = both::<FnCompressFrameCDict>("LZ4F_compressFrame_usingCDict");
    let (c5, r5) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (c6, r6) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    (
        CdictApi {
            create: c1,
            create_adv: c2,
            free: c3,
            compress: c4,
            cctx_new: c5,
            cctx_free: c6,
            tag: "C",
        },
        CdictApi {
            create: r1,
            create_adv: r2,
            free: r3,
            compress: r4,
            cctx_new: r5,
            cctx_free: r6,
            tag: "Rust",
        },
    )
}

const DICT_SIZES: [usize; 8] = [0, 1, 8, 100, 4096, 65535, 65536, 70000];

/// Compress with `LZ4F_compressFrame_usingCDict` on one library, using that
/// library's own CDict and cctx. Returns `(ret, full_dst_buffer)`.
unsafe fn cdict_compress(
    api: &CdictApi,
    dict: Option<&[u8]>,
    advanced: bool,
    src: &[u8],
    cap: usize,
    prefs: *const LZ4F_preferences_t,
    ctx: &str,
) -> (usize, Vec<u8>) {
    let cdict: *mut c_void = match dict {
        None => ptr::null_mut(),
        Some(d) => {
            let p = if advanced {
                (api.create_adv)(
                    LZ4F_CustomMem::default(),
                    d.as_ptr() as *const c_void,
                    d.len(),
                )
            } else {
                (api.create)(d.as_ptr() as *const c_void, d.len())
            };
            assert!(!p.is_null(), "{}: {} createCDict returned NULL", ctx, api.tag);
            p
        }
    };

    let mut cctx: *mut c_void = ptr::null_mut();
    let cr = (api.cctx_new)(&mut cctx, LZ4F_VERSION);
    assert!(
        !lz4f_is_error(cr) && !cctx.is_null(),
        "{}: {} createCompressionContext failed ({})",
        ctx,
        api.tag,
        err_str(cr)
    );

    let mut buf = vec![SENTINEL; cap.max(1)];
    let n = (api.compress)(
        cctx,
        buf.as_mut_ptr() as *mut c_void,
        cap,
        src.as_ptr() as *const c_void,
        src.len(),
        cdict,
        prefs,
    );

    (api.cctx_free)(cctx);
    (api.free)(cdict); // includes LZ4F_freeCDict(NULL) when dict is None
    (n, buf)
}

#[test]
fn create_cdict_all_sizes_and_free_null() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0xCD1C_7000_1234_5678);

    for &ds in &DICT_SIZES {
        for shape in 0..N_SHAPES {
            let dict = gen_src(&mut rng, shape, ds);
            unsafe {
                let cp = (capi.create)(dict.as_ptr() as *const c_void, ds);
                let rp = (rapi.create)(dict.as_ptr() as *const c_void, ds);
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "LZ4F_createCDict(dictSize={}): C null={} Rust null={}",
                    ds,
                    cp.is_null(),
                    rp.is_null()
                );
                assert!(!cp.is_null(), "LZ4F_createCDict(dictSize={}) must succeed", ds);
                (capi.free)(cp);
                (rapi.free)(rp);

                let cp = (capi.create_adv)(
                    LZ4F_CustomMem::default(),
                    dict.as_ptr() as *const c_void,
                    ds,
                );
                let rp = (rapi.create_adv)(
                    LZ4F_CustomMem::default(),
                    dict.as_ptr() as *const c_void,
                    ds,
                );
                assert_eq!(
                    cp.is_null(),
                    rp.is_null(),
                    "LZ4F_createCDict_advanced(dictSize={}): C null={} Rust null={}",
                    ds,
                    cp.is_null(),
                    rp.is_null()
                );
                assert!(
                    !cp.is_null(),
                    "LZ4F_createCDict_advanced(dictSize={}) must succeed",
                    ds
                );
                (capi.free)(cp);
                (rapi.free)(rp);
            }
        }
    }

    // free-on-NULL is explicitly supported (lz4frame.c:583).
    unsafe {
        (capi.free)(ptr::null_mut());
        (rapi.free)(ptr::null_mut());
        (capi.free)(ptr::null_mut());
        (rapi.free)(ptr::null_mut());
    }
}

#[test]
fn compress_frame_using_cdict_cross_product() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0xCD1C_C0DE_0000_0001);

    let mut idx = 0usize;
    for &ds in &DICT_SIZES {
        let dict = gen_src(&mut rng, idx % N_SHAPES, ds);
        for &advanced in &[false, true] {
            for &(bsid, len) in &[
                (LZ4F_default, 0usize),
                (LZ4F_max64KB, 1usize),
                (LZ4F_max64KB, 3000usize),
                (LZ4F_max64KB, 70_000usize),
                (LZ4F_max256KB, 300_000usize),
            ] {
                for &level in &[c_int::MIN, -1, 0, 1, 2, 3, 9, 11, 12, c_int::MAX] {
                    let shape = idx % N_SHAPES;
                    // Make the source partly overlap the dictionary content so the
                    // dictionary is actually useful (and the dict paths differ from
                    // the no-dict paths).
                    let mut src = gen_src(&mut rng, shape, len);
                    if !dict.is_empty() && src.len() >= dict.len() {
                        let k = dict.len().min(src.len());
                        src[..k].copy_from_slice(&dict[..k]);
                    }
                    let p = P {
                        bsid,
                        bmode: (idx % 2) as c_int,
                        ccs: ((idx / 2) % 2) as c_int,
                        bcs: ((idx / 4) % 2) as c_int,
                        ftype: LZ4F_frame,
                        autoflush: (idx % 2) as u32,
                        csize: idx % 3 == 0,
                        dict_id: if idx % 2 == 0 { 0xDEAD_BEEF } else { 0 },
                        level,
                        favor: ((idx / 3) % 2) as u32,
                    };
                    let prefs = p.to_prefs(len);
                    let pp = &prefs as *const LZ4F_preferences_t;
                    let bound = frame_bound(len, pp);
                    let ctx = format!(
                        "usingCDict dictSize={} advanced={} [{}] len={} shape={}",
                        ds, advanced, p, len, shape_name(shape)
                    );
                    let (cn, cbuf) = unsafe {
                        cdict_compress(&capi, Some(&dict), advanced, &src, bound, pp, &ctx)
                    };
                    let (rn, rbuf) = unsafe {
                        cdict_compress(&rapi, Some(&dict), advanced, &src, bound, pp, &ctx)
                    };
                    if cn != rn {
                        panic!(
                            "{}\n  return mismatch: C={} ({}) Rust={} ({})",
                            ctx,
                            cn,
                            err_str(cn),
                            rn,
                            err_str(rn)
                        );
                    }
                    assert!(!lz4f_is_error(cn), "{}: {}", ctx, err_str(cn));
                    assert_bytes_eq(&ctx, &cbuf, &rbuf);
                    round_trip_both_with_dict(&ctx, &cbuf[..cn], &src, &dict);
                    idx += 1;
                }
            }
        }
    }
    assert_eq!(idx, DICT_SIZES.len() * 2 * 5 * 10);
}

/// `LZ4F_createCDict*` keeps only the LAST 64 KB of the dictionary buffer
/// (lz4frame.c:546-549): a 70000-byte dictionary must therefore produce exactly the
/// same frame as its final 65536 bytes, in BOTH libraries.
#[test]
fn create_cdict_truncates_to_last_64kb() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0x7000_0000_6553_6000);
    let big = gen_src(&mut rng, 5, 70_000);
    let tail = big[big.len() - 65536..].to_vec();
    let mut src = gen_src(&mut rng, 5, 40_000);
    src[..20_000].copy_from_slice(&big[big.len() - 20_000..]);

    for &level in &[1, 2, 9] {
        let p = P { bsid: LZ4F_max64KB, ccs: 1, bcs: 1, level, autoflush: 1, ..P::base() };
        let prefs = p.to_prefs(src.len());
        let pp = &prefs as *const LZ4F_preferences_t;
        let bound = frame_bound(src.len(), pp);
        let mut outs: Vec<(usize, Vec<u8>)> = Vec::new();
        for api in [&capi, &rapi] {
            for dict in [&big, &tail] {
                let ctx = format!("truncation level={} {}", level, api.tag);
                let (n, buf) =
                    unsafe { cdict_compress(api, Some(dict), false, &src, bound, pp, &ctx) };
                assert!(!lz4f_is_error(n), "{}: {}", ctx, err_str(n));
                outs.push((n, buf));
            }
        }
        // C(70000) == C(65536) == Rust(70000) == Rust(65536)
        for k in 1..outs.len() {
            assert_eq!(
                outs[0].0, outs[k].0,
                "level={}: 70000-byte dict must behave as its last 64 KB (variant {})",
                level, k
            );
            assert_bytes_eq(
                &format!("CDict truncation level={} variant {}", level, k),
                &outs[0].1,
                &outs[k].1,
            );
        }
        round_trip_both_with_dict(
            &format!("CDict truncation level={}", level),
            &outs[0].1[..outs[0].0],
            &src,
            &tail,
        );
    }
}

#[test]
fn compress_frame_using_cdict_null_dict_and_null_cctx() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0xCD1C_0000_DEAD_0001);

    // cdict == NULL is documented as "compress without a dictionary" and must
    // then match plain LZ4F_compressFrame byte-for-byte.
    for &len in &[0usize, 1, 3000, 70_000] {
        for &level in &[-1, 0, 1, 2, 10] {
            let src = gen_src(&mut rng, len % N_SHAPES, len);
            let p = P {
                bsid: LZ4F_max64KB,
                bmode: LZ4F_blockLinked,
                ccs: 1,
                bcs: 1,
                ftype: LZ4F_frame,
                autoflush: 0,
                csize: true,
                dict_id: 0x5555_AAAA,
                level,
                favor: 0,
            };
            let prefs = p.to_prefs(len);
            let pp = &prefs as *const LZ4F_preferences_t;
            let bound = frame_bound(len, pp);
            let ctx = format!("usingCDict(NULL dict) len={} level={}", len, level);
            let (cn, cbuf) = unsafe { cdict_compress(&capi, None, false, &src, bound, pp, &ctx) };
            let (rn, rbuf) = unsafe { cdict_compress(&rapi, None, false, &src, bound, pp, &ctx) };
            assert_eq!(cn, rn, "{}: C={} Rust={}", ctx, err_str(cn), err_str(rn));
            assert_bytes_eq(&ctx, &cbuf, &rbuf);
            assert!(!lz4f_is_error(cn), "{}: {}", ctx, err_str(cn));

            // Identical to LZ4F_compressFrame (which passes cdict=NULL internally).
            let (fn_, fbuf) = frame_case(&format!("{} vs compressFrame", ctx), &prefs, &src, bound);
            assert_eq!(cn, fn_, "{}: usingCDict(NULL) != compressFrame length", ctx);
            assert_bytes_eq(
                &format!("{}: usingCDict(NULL) vs compressFrame", ctx),
                &cbuf[..cn],
                &fbuf[..fn_],
            );
            round_trip_both(&ctx, &cbuf[..cn], &src);
        }
    }

    // NULL cctx: lz4frame.c:456 rejects a too-small dstCapacity *before* the cctx
    // is ever dereferenced, so this specific combination is well defined.
    let src = gen_src(&mut rng, 0, 0);
    let (cnf, rnf) = both::<FnCompressFrameCDict>("LZ4F_compressFrame_usingCDict");
    for &cap in &[0usize, 1, 18, 22] {
        let mut cbuf = vec![SENTINEL; cap.max(1)];
        let mut rbuf = vec![SENTINEL; cap.max(1)];
        let cn = unsafe {
            cnf(
                ptr::null_mut(),
                cbuf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                0,
                ptr::null(),
                ptr::null(),
            )
        };
        let rn = unsafe {
            rnf(
                ptr::null_mut(),
                rbuf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                0,
                ptr::null(),
                ptr::null(),
            )
        };
        assert_eq!(
            cn, rn,
            "usingCDict(NULL cctx, cap={}): C={} Rust={}",
            cap,
            err_str(cn),
            err_str(rn)
        );
        assert_eq!(
            lz4f_error_code(cn),
            err::ERROR_dstMaxSize_tooSmall,
            "usingCDict(NULL cctx, cap={}) must be dstMaxSize_tooSmall(11)",
            cap
        );
        assert_bytes_eq(
            &format!("usingCDict(NULL cctx, cap={}) dst untouched", cap),
            &cbuf,
            &rbuf,
        );
    }
}

#[test]
fn row_170_using_cdict_dst_too_small() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0x1701_CD1C_0000_0001);
    let dict = gen_src(&mut rng, 3, 4096);

    for (i, &(len, level)) in [(0usize, 1i32), (100, 2), (3000, 9), (70_000, 1)].iter().enumerate() {
        let src = gen_src(&mut rng, i % N_SHAPES, len);
        let p = P {
            bsid: LZ4F_max64KB,
            bmode: (i % 2) as c_int,
            ccs: 1,
            bcs: (i % 2) as c_int,
            ftype: LZ4F_frame,
            autoflush: 1,
            csize: i % 2 == 0,
            dict_id: 0x1234,
            level,
            favor: 0,
        };
        let prefs = p.to_prefs(len);
        let pp = &prefs as *const LZ4F_preferences_t;
        let bound = frame_bound(len, pp);
        let caps: Vec<usize> = if bound <= 400 {
            (0..bound).collect()
        } else {
            vec![0, 1, 18, 19, 22, bound / 2, bound - 2, bound - 1]
        };
        for &cap in &caps {
            let ctx = format!("row170 usingCDict len={} level={} cap={} bound={}", len, level, cap, bound);
            let (cn, cbuf) =
                unsafe { cdict_compress(&capi, Some(&dict), false, &src, cap, pp, &ctx) };
            let (rn, rbuf) =
                unsafe { cdict_compress(&rapi, Some(&dict), false, &src, cap, pp, &ctx) };
            assert_eq!(cn, rn, "{}: C={} Rust={}", ctx, err_str(cn), err_str(rn));
            assert_eq!(
                lz4f_error_code(cn),
                err::ERROR_dstMaxSize_tooSmall,
                "{}: expected dstMaxSize_tooSmall(11), got {}",
                ctx,
                lz4f_error_code(cn)
            );
            assert_bytes_eq(&format!("{} dst untouched", ctx), &cbuf, &rbuf);
            assert!(
                cbuf.iter().all(|&b| b == SENTINEL),
                "{}: dst modified on error path",
                ctx
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 179 / 180 — LZ4F_createCDict* allocation failure
// ---------------------------------------------------------------------------

thread_local! {
    static ALLOC_SEQ: Cell<u64> = const { Cell::new(0) };
    static ALLOC_FAIL_AT: Cell<u64> = const { Cell::new(u64::MAX) };
}

/// Header stored in front of every block handed out, so `test_free` can rebuild
/// the exact `Layout`. 16 bytes keeps the returned pointer 16-byte aligned, which
/// LZ4_initStream / LZ4_initStreamHC require.
const HDR: usize = 16;

extern "C" fn test_alloc(_opaque: *mut c_void, size: usize) -> *mut c_void {
    let n = ALLOC_SEQ.with(|c| {
        let v = c.get() + 1;
        c.set(v);
        v
    });
    if n == ALLOC_FAIL_AT.with(|c| c.get()) {
        return ptr::null_mut();
    }
    let total = size + HDR;
    let layout = std::alloc::Layout::from_size_align(total, HDR).unwrap();
    let p = unsafe { std::alloc::alloc(layout) };
    if p.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        (p as *mut usize).write(total);
        p.add(HDR) as *mut c_void
    }
}

extern "C" fn test_calloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    let p = test_alloc(opaque, size);
    if !p.is_null() {
        unsafe { ptr::write_bytes(p as *mut u8, 0, size) };
    }
    p
}

extern "C" fn test_free(_opaque: *mut c_void, address: *mut c_void) {
    if address.is_null() {
        return;
    }
    unsafe {
        let base = (address as *mut u8).sub(HDR);
        let total = (base as *mut usize).read();
        let layout = std::alloc::Layout::from_size_align(total, HDR).unwrap();
        std::alloc::dealloc(base, layout);
    }
}

fn arm_allocator(fail_at: u64) {
    ALLOC_SEQ.with(|c| c.set(0));
    ALLOC_FAIL_AT.with(|c| c.set(fail_at));
}

#[test]
fn rows_179_180_create_cdict_allocation_failure() {
    let (capi, rapi) = cdict_apis();
    let mut rng = Rng::new(0x179_0180_ABCD_EF01);

    let failing = LZ4F_CustomMem {
        customAlloc: Some(test_alloc),
        customCalloc: Some(test_calloc),
        customFree: Some(test_free),
        opaqueState: ptr::null_mut(),
    };

    // LZ4F_createCDict_advanced performs exactly four LZ4F_malloc() calls:
    //   1: the LZ4F_CDict struct        (row 179)
    //   2: cdict->dictContent           (row 180)
    //   3: cdict->fastCtx               (row 180)
    //   4: cdict->HCCtx                 (row 180)
    // Failing each one independently must make BOTH libraries return NULL.
    for &ds in &DICT_SIZES {
        let dict = gen_src(&mut rng, 4, ds);
        for fail_at in 1..=4u64 {
            arm_allocator(fail_at);
            let cp = unsafe {
                (capi.create_adv)(failing, dict.as_ptr() as *const c_void, ds)
            };
            arm_allocator(fail_at);
            let rp = unsafe {
                (rapi.create_adv)(failing, dict.as_ptr() as *const c_void, ds)
            };
            assert!(
                cp.is_null(),
                "C LZ4F_createCDict_advanced(dictSize={}) must return NULL when allocation #{} fails",
                ds,
                fail_at
            );
            assert!(
                rp.is_null(),
                "Rust LZ4F_createCDict_advanced(dictSize={}) must return NULL when allocation #{} fails",
                ds,
                fail_at
            );
        }

        // Never failing (fail_at beyond the 4 allocations) must succeed in both,
        // and be releasable through the same custom allocator.
        arm_allocator(u64::MAX);
        let cp = unsafe { (capi.create_adv)(failing, dict.as_ptr() as *const c_void, ds) };
        let calls_c = ALLOC_SEQ.with(|c| c.get());
        arm_allocator(u64::MAX);
        let rp = unsafe { (rapi.create_adv)(failing, dict.as_ptr() as *const c_void, ds) };
        let calls_r = ALLOC_SEQ.with(|c| c.get());
        assert!(!cp.is_null(), "C createCDict_advanced(dictSize={}) failed", ds);
        assert!(!rp.is_null(), "Rust createCDict_advanced(dictSize={}) failed", ds);
        assert_eq!(
            calls_c, calls_r,
            "dictSize={}: allocation count differs (C={} Rust={})",
            ds, calls_c, calls_r
        );
        assert_eq!(calls_c, 4, "dictSize={}: expected 4 allocations", ds);
        unsafe {
            (capi.free)(cp);
            (rapi.free)(rp);
        }
    }

    // An allocator that always fails.
    let always_fail_alloc = LZ4F_CustomMem {
        customAlloc: Some(never_alloc),
        customCalloc: Some(never_alloc),
        customFree: Some(test_free),
        opaqueState: ptr::null_mut(),
    };
    for &ds in &[0usize, 100, 70000] {
        let dict = gen_src(&mut rng, 0, ds);
        let cp = unsafe {
            (capi.create_adv)(always_fail_alloc, dict.as_ptr() as *const c_void, ds)
        };
        let rp = unsafe {
            (rapi.create_adv)(always_fail_alloc, dict.as_ptr() as *const c_void, ds)
        };
        assert!(cp.is_null() && rp.is_null(), "row 179: both must return NULL");
    }

    // LZ4F_createCDict_advanced only ever uses customAlloc (never customCalloc),
    // so a CustomMem with *only* customCalloc set must fall back to stdlib
    // malloc/free and succeed in both libraries.
    let calloc_only = LZ4F_CustomMem {
        customAlloc: None,
        customCalloc: Some(never_alloc),
        customFree: None,
        opaqueState: ptr::null_mut(),
    };
    for &ds in &[0usize, 100, 70000] {
        let dict = gen_src(&mut rng, 1, ds);
        let cp = unsafe { (capi.create_adv)(calloc_only, dict.as_ptr() as *const c_void, ds) };
        let rp = unsafe { (rapi.create_adv)(calloc_only, dict.as_ptr() as *const c_void, ds) };
        assert!(
            !cp.is_null() && !rp.is_null(),
            "customCalloc is unused by createCDict_advanced (dictSize={}): C null={} Rust null={}",
            ds,
            cp.is_null(),
            rp.is_null()
        );
        unsafe {
            (capi.free)(cp);
            (rapi.free)(rp);
        }
    }

    // A CDict built through a custom allocator must still compress identically.
    let dict = gen_src(&mut rng, 5, 8192);
    let src = gen_src(&mut rng, 5, 40_000);
    let p = P { bsid: LZ4F_max64KB, ccs: 1, bcs: 1, level: 3, autoflush: 1, ..P::base() };
    let prefs = p.to_prefs(src.len());
    let pp = &prefs as *const LZ4F_preferences_t;
    let bound = frame_bound(src.len(), pp);
    let mut results: Vec<(usize, Vec<u8>)> = Vec::new();
    for api in [&capi, &rapi] {
        arm_allocator(u64::MAX);
        let cd = unsafe { (api.create_adv)(failing, dict.as_ptr() as *const c_void, dict.len()) };
        assert!(!cd.is_null());
        let mut cctx: *mut c_void = ptr::null_mut();
        let cr = unsafe { (api.cctx_new)(&mut cctx, LZ4F_VERSION) };
        assert!(!lz4f_is_error(cr) && !cctx.is_null());
        let mut buf = vec![SENTINEL; bound];
        let n = unsafe {
            (api.compress)(
                cctx,
                buf.as_mut_ptr() as *mut c_void,
                bound,
                src.as_ptr() as *const c_void,
                src.len(),
                cd,
                pp,
            )
        };
        unsafe {
            (api.cctx_free)(cctx);
            (api.free)(cd);
        }
        assert!(!lz4f_is_error(n), "{}: custom-alloc CDict compress: {}", api.tag, err_str(n));
        results.push((n, buf));
    }
    assert_eq!(results[0].0, results[1].0, "custom-alloc CDict: length differs");
    assert_bytes_eq("custom-alloc CDict frame", &results[0].1, &results[1].1);
    round_trip_both_with_dict(
        "custom-alloc CDict frame",
        &results[0].1[..results[0].0],
        &src,
        &dict,
    );
}

extern "C" fn never_alloc(_opaque: *mut c_void, _size: usize) -> *mut c_void {
    ptr::null_mut()
}

// ===========================================================================
// LZ4F_headerSize
// ===========================================================================

fn header_size_both(ctx: &str, buf: &[u8], src_size: usize) -> usize {
    let (c, r) = both::<FnHeaderSize>("LZ4F_headerSize");
    let cv = unsafe { c(buf.as_ptr() as *const c_void, src_size) };
    let rv = unsafe { r(buf.as_ptr() as *const c_void, src_size) };
    if cv != rv {
        panic!(
            "{}\n  LZ4F_headerSize mismatch: C={} ({}) Rust={} ({})",
            ctx,
            cv,
            err_str(cv),
            rv,
            err_str(rv)
        );
    }
    cv
}

#[test]
fn header_size_on_real_frames_every_option_combination() {
    let mut rng = Rng::new(0x4EAD_5123_0000_0001);
    let mut seen: Vec<usize> = Vec::new();

    for &bsid in &[LZ4F_default, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB] {
        for &bmode in &[LZ4F_blockLinked, LZ4F_blockIndependent] {
            for &ccs in &[0, 1] {
                for &bcs in &[0, 1] {
                    for &use_cs in &[false, true] {
                        for &did in &[0u32, 0xDEAD_BEEFu32] {
                            let len = 1000usize;
                            let src = gen_src(&mut rng, 3, len);
                            let p = P {
                                bsid,
                                bmode,
                                ccs,
                                bcs,
                                ftype: LZ4F_frame,
                                autoflush: 1,
                                csize: use_cs,
                                dict_id: did,
                                level: 1,
                                favor: 0,
                            };
                            let prefs = p.to_prefs(len);
                            let pp = &prefs as *const LZ4F_preferences_t;
                            let bound = frame_bound(len, pp);
                            let ctx = format!("headerSize on frame [{}]", p);
                            let (n, frame) = frame_case(&ctx, &prefs, &src, bound);
                            assert!(!lz4f_is_error(n));

                            // The header length varies with contentSize / dictID.
                            let expect = LZ4F_HEADER_SIZE_MIN
                                + if use_cs { 8 } else { 0 }
                                + if did != 0 { 4 } else { 0 };
                            for probe in [5usize, 6, 7, 19, n.min(frame.len())] {
                                if probe > frame.len() {
                                    continue;
                                }
                                let hs = header_size_both(
                                    &format!("{} (srcSize={})", ctx, probe),
                                    &frame,
                                    probe,
                                );
                                assert_eq!(
                                    hs, expect,
                                    "{}: LZ4F_headerSize={} expected {}",
                                    ctx, hs, expect
                                );
                            }
                            if !seen.contains(&expect) {
                                seen.push(expect);
                            }
                        }
                    }
                }
            }
        }
    }
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec![7usize, 11, 15, 19],
        "all four possible header lengths must have been produced"
    );
}

#[test]
fn header_size_truncations_skippable_magics_and_garbage() {
    // A real 19-byte header (contentSize + dictID present).
    let mut rng = Rng::new(0x4EAD_7000_0000_0002);
    let src = gen_src(&mut rng, 3, 500);
    let p = P {
        bsid: LZ4F_max64KB,
        bmode: LZ4F_blockIndependent,
        ccs: 1,
        bcs: 1,
        ftype: LZ4F_frame,
        autoflush: 1,
        csize: true,
        dict_id: 0xDEAD_BEEF,
        level: 1,
        favor: 0,
    };
    let prefs = p.to_prefs(src.len());
    let bound = frame_bound(src.len(), &prefs as *const LZ4F_preferences_t);
    let (n, frame19) = frame_case("headerSize truncation base frame", &prefs, &src, bound);
    assert!(!lz4f_is_error(n));

    // --- truncated headers, every length 0..=19 --------------------------
    for cut in 0..=19usize {
        let ctx = format!("truncated header, srcSize={}", cut);
        let hs = header_size_both(&ctx, &frame19, cut);
        if cut < 5 {
            // LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH == 5
            assert_eq!(
                lz4f_error_code(hs),
                err::ERROR_frameHeader_incomplete,
                "{}: expected frameHeader_incomplete(12), got {}",
                ctx,
                err_str(hs)
            );
        } else {
            assert_eq!(hs, 19, "{}: expected 19, got {}", ctx, err_str(hs));
        }
    }
    // The same sweep on a minimal (7-byte) header must report 7 from srcSize 5 up.
    let p7 = P { bsid: LZ4F_max64KB, autoflush: 1, level: 1, ..P::base() };
    let prefs7 = p7.to_prefs(src.len());
    let bound7 = frame_bound(src.len(), &prefs7 as *const LZ4F_preferences_t);
    let (n7, frame7) = frame_case("headerSize 7-byte header", &prefs7, &src, bound7);
    assert!(!lz4f_is_error(n7));
    for cut in 0..=19usize {
        let hs = header_size_both(&format!("7-byte header srcSize={}", cut), &frame7, cut);
        if cut < 5 {
            assert_eq!(lz4f_error_code(hs), err::ERROR_frameHeader_incomplete);
        } else {
            assert_eq!(hs, 7, "srcSize={}: expected 7, got {}", cut, err_str(hs));
        }
    }

    // --- NULL src -------------------------------------------------------
    let (chs, rhs) = both::<FnHeaderSize>("LZ4F_headerSize");
    for &sz in &[0usize, 1, 5, 19, 1000] {
        let cv = unsafe { chs(ptr::null(), sz) };
        let rv = unsafe { rhs(ptr::null(), sz) };
        assert_eq!(
            cv, rv,
            "LZ4F_headerSize(NULL, {}): C={} Rust={}",
            sz,
            err_str(cv),
            err_str(rv)
        );
        assert_eq!(
            lz4f_error_code(cv),
            err::ERROR_srcPtr_wrong,
            "LZ4F_headerSize(NULL, {}) must be srcPtr_wrong(15)",
            sz
        );
    }

    // --- all 16 skippable magics ---------------------------------------
    for m in 0..16u32 {
        let magic = LZ4F_MAGIC_SKIPPABLE_START + m;
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        buf[4..8].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        for &sz in &[5usize, 6, 7, 8, 19, 32] {
            let hs = header_size_both(
                &format!("skippable magic {:#010x} srcSize={}", magic, sz),
                &buf,
                sz,
            );
            assert_eq!(hs, 8, "skippable magic {:#010x} must give 8", magic);
        }
        // Below the 5-byte minimum, the magic is irrelevant.
        for &sz in &[0usize, 1, 4] {
            let hs = header_size_both(
                &format!("skippable magic {:#010x} srcSize={}", magic, sz),
                &buf,
                sz,
            );
            assert_eq!(lz4f_error_code(hs), err::ERROR_frameHeader_incomplete);
        }
    }
    // Just outside the skippable range.
    for magic in [
        LZ4F_MAGIC_SKIPPABLE_START - 1,
        LZ4F_MAGIC_SKIPPABLE_START + 16,
        LZ4F_MAGICNUMBER ^ 0xFF,
        0,
        0xFFFF_FFFF,
        0x0422_4D18, // byte-swapped magic
    ] {
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&magic.to_le_bytes());
        buf[4] = 0x64;
        let hs = header_size_both(&format!("non-skippable magic {:#010x}", magic), &buf, 32);
        assert_eq!(
            lz4f_error_code(hs),
            err::ERROR_frameType_unknown,
            "magic {:#010x} must be frameType_unknown(13), got {}",
            magic,
            err_str(hs)
        );
    }
    // The real magic with every possible FLG byte.
    for flg in 0..=255u8 {
        let mut buf = vec![0u8; 32];
        buf[0..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes());
        buf[4] = flg;
        let hs = header_size_both(&format!("magic + FLG={:#04x}", flg), &buf, 32);
        let expect = LZ4F_HEADER_SIZE_MIN
            + if (flg >> 3) & 1 != 0 { 8 } else { 0 }
            + if flg & 1 != 0 { 4 } else { 0 };
        assert_eq!(hs, expect, "FLG={:#04x}: expected {}", flg, expect);
    }

    // --- garbage / random buffers ---------------------------------------
    for i in 0..4000usize {
        let len = rng.range(1, 40);
        let mut buf = gen_src(&mut rng, i % N_SHAPES, len);
        if buf.len() < 8 {
            buf.resize(8, 0);
        }
        // occasionally plant a valid or near-valid magic
        match i % 5 {
            0 => buf[0..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes()),
            1 => buf[0..4]
                .copy_from_slice(&(LZ4F_MAGIC_SKIPPABLE_START + rng.below(20) as u32).to_le_bytes()),
            _ => {}
        }
        let sz = rng.range(0, buf.len());
        header_size_both(
            &format!("garbage #{} len={} srcSize={}", i, buf.len(), sz),
            &buf,
            sz,
        );
    }
}
