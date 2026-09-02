//! Phase B: differential tests for the LEGACY decoders (v01..v07) — the
//! "structured / valid-shaped" input side.
//!
//! We cannot produce genuine legacy frames with the modern encoder, so instead
//! we synthesize inputs that START with each version's exact magic number
//! (read out of the legacy C headers, NOT guessed) followed by random and
//! structured payloads, then drive them through every legacy entry point and
//! assert C and Rust agree bit-for-bit.
//!
//! This file focuses on the "well-formed prefix" and cross-API-interop cases:
//!   * legacy one-shot decoders (`ZSTDv0x_decompress[DCtx]`) with dst-capacity
//!     sweeps {0,1,small,exact,large},
//!   * `ZSTDv0x_findFrameSizeInfoLegacy` (out params),
//!   * `ZSTDv0x_getFrameParams` (v05/v06/v07) struct output,
//!   * the streaming legacy entry points
//!     (create/free/reset DCtx, nextSrcSizeToDecompress, decompressContinue,
//!      and the ZBUFFv0x_* buffered decoders),
//!   * feeding legacy-magic buffers to the MODERN API (`ZSTD_decompress`,
//!     `ZSTD_getFrameContentSize`, `ZSTD_isFrame`, `ZSTD_findFrameCompressedSize`,
//!     `ZSTD_decompressStream`, `ZSTD_getFrameHeader`),
//!   * feeding valid MODERN frames to the legacy entry points.
//!
//! The exhaustive mutation / truncation / garbage error sweeps live in
//! `c13_legacy.rs`. Every call crosses the FFI boundary via `both`.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

use std::os::raw::c_ulonglong;

// ------------------------------------------------------- legacy magic numbers
// (exact constants copied from c_src/src/legacy/zstd_v0x.h)
const MAGIC_V01: u32 = 0xFD2FB51E;
const MAGIC_V02: u32 = 0xFD2FB522;
const MAGIC_V03: u32 = 0xFD2FB523;
const MAGIC_V04: u32 = 0xFD2FB524;
const MAGIC_V05: u32 = 0xFD2FB525;
const MAGIC_V06: u32 = 0xFD2FB526;
const MAGIC_V07: u32 = 0xFD2FB527;
const MAGIC_MODERN: u32 = 0xFD2FB528; // ZSTD v0.8+ (current)

const LEGACY: &[(u32, &str)] = &[
    (MAGIC_V01, "v01"),
    (MAGIC_V02, "v02"),
    (MAGIC_V03, "v03"),
    (MAGIC_V04, "v04"),
    (MAGIC_V05, "v05"),
    (MAGIC_V06, "v06"),
    (MAGIC_V07, "v07"),
];

// ---------------------------------------------------------------- FFI typedefs

// one-shot: (dst, dstCap, src, srcSize) -> size_t
type FnDec4 = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
// decompressDCtx: (ctx, dst, dstCap, src, srcSize) -> size_t
type FnDecDCtx = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
// findFrameSizeInfoLegacy: (src, srcSize, *cSize, *dBound) -> void
type FnFindInfo = unsafe extern "C" fn(*const c_void, size_t, *mut size_t, *mut c_ulonglong);
type FnIsErr = unsafe extern "C" fn(size_t) -> c_uint;
type FnErrName = unsafe extern "C" fn(size_t) -> *const std::os::raw::c_char;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnNextSrc = unsafe extern "C" fn(*mut c_void) -> size_t;
// decompressContinue (legacy raw): (dctx, dst, maxDst, src, srcSize) -> size_t
type FnDecCont = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
// ZBUFFv04 continue: (dctx, dst, *maxDst, src, *srcSize) -> size_t  (note *maxDstSizePtr)
type FnZbuffCont =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnZbuffInit = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnZbuffInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
// modern
type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;
type FnGetFCS = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnIsFrame = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnFindComp = unsafe extern "C" fn(*const c_void, size_t) -> size_t;

// --------------------------------------------------------- per-version err api

/// A legacy error classifier. Prefers `ZSTDv0x_isError` / `ZSTDv0x_getErrorName`
/// when the version exports them; otherwise falls back to a classifier that is
/// ABI-identical (all of them delegate to the shared `ERR_*` implementation):
/// `ZBUFFv04_isError`/`getErrorName` for v04, else the modern `ZSTD_isError`.
/// Compared as (boolean, string) across libraries.
struct LegErr {
    is_err: (libloading::Symbol<'static, FnIsErr>, libloading::Symbol<'static, FnIsErr>),
    name: Option<(libloading::Symbol<'static, FnErrName>, libloading::Symbol<'static, FnErrName>)>,
}
impl LegErr {
    unsafe fn new(ver: &str) -> Self {
        // isError source
        let own_is = format!("ZSTD{ver}_isError");
        let is_err = if has_both(&own_is) {
            both::<FnIsErr>(&own_is)
        } else if has_both(&format!("ZBUFF{ver}_isError")) {
            both::<FnIsErr>(&format!("ZBUFF{ver}_isError"))
        } else {
            both::<FnIsErr>("ZSTD_isError")
        };
        // getErrorName source (optional but almost always available via a fallback)
        let own_nm = format!("ZSTD{ver}_getErrorName");
        let name = if has_both(&own_nm) {
            Some(both::<FnErrName>(&own_nm))
        } else if has_both(&format!("ZBUFF{ver}_getErrorName")) {
            Some(both::<FnErrName>(&format!("ZBUFF{ver}_getErrorName")))
        } else {
            Some(both::<FnErrName>("ZSTD_getErrorName"))
        };
        LegErr { is_err, name }
    }
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let c_is = (self.is_err.0)(cr) != 0;
        let r_is = (self.is_err.1)(rr) != 0;
        assert_eq!(c_is, r_is, "{ctx}: isError C={c_is} RS={r_is} (raw C={cr:#x} RS={rr:#x})");
        if let Some((cn_fn, rn_fn)) = &self.name {
            let cn = cstr(cn_fn(cr));
            let rn = cstr(rn_fn(rr));
            assert_eq!(cn, rn, "{ctx}: getErrorName C={cn:?} RS={rn:?} (raw C={cr:#x} RS={rr:#x})");
        }
        if !c_is {
            assert_eq!(cr, rr, "{ctx}: OK value differs C={cr:#x} RS={rr:#x}");
        }
    }
    unsafe fn is_c_err(&self, r: size_t) -> bool {
        (self.is_err.0)(r) != 0
    }
}

// ------------------------------------------------------------- input builders

/// Build a buffer that starts with a 4-byte little-endian magic, followed by a
/// structured payload of `body_len` bytes.
fn magic_buf(magic: u32, shape: Shape, body_len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + body_len);
    v.extend_from_slice(&magic.to_le_bytes());
    let body = gen(shape, body_len, rng);
    v.extend_from_slice(&body);
    v
}

// The set of "one-shot decompress" symbols and whether the ctx form takes ctx
// as first arg (all of them do: void* ctx / DCtx*).
fn oneshot_name(ver: &str) -> String {
    format!("ZSTD{ver}_decompress")
}

// -------------------------------------------------------------------- helpers

/// Run a legacy one-shot decoder over `input` with a sweep of dst capacities
/// {0,1,small,exact-ish,large}. C and Rust must agree on every capacity.
unsafe fn sweep_oneshot(le: &LegErr, dec: &(libloading::Symbol<'static, FnDec4>, libloading::Symbol<'static, FnDec4>),
                        input: &[u8], ctx: &str) {
    // Choose a spread of dst capacities. "exact" is unknowable for garbage, so
    // we approximate with body length and a large value.
    let caps: [usize; 6] = [0, 1, 16, input.len().max(1), 4096, 1 << 17];
    for &cap in &caps {
        let mut cbuf = vec![0u8; cap.max(1)];
        let mut rbuf = vec![0u8; cap.max(1)];
        let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
        let a = (dec.0)(cbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
        let b = (dec.1)(rbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
        let c2 = format!("{ctx} oneshot cap={cap}");
        le.eq(&c2, a, b);
        if !le.is_c_err(a) {
            assert_bytes_eq(&format!("{c2} out"), &cbuf[..a], &rbuf[..b]);
        }
    }
}

// -------------------------------------------------------------------- tests

/// One-shot legacy decoders over magic-prefixed structured buffers, across all
/// shapes / lengths, with a full dst-capacity sweep. Exercises the auto-legacy
/// dispatch inside each `ZSTDv0x_decompress`.
#[test]
fn oneshot_decoders_magic_prefixed() {
    unsafe {
        let mut rng = Rng::new(0xB15_0001);
        for &(magic, ver) in LEGACY {
            let le = LegErr::new(ver);
            let dec = both::<FnDec4>(&oneshot_name(ver));
            for &shape in ALL_SHAPES {
                for &blen in &[0usize, 1, 3, 8, 32, 100, 512, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    let ctx = format!("{ver} magic shape={shape:?} blen={blen}");
                    sweep_oneshot(&le, &dec, &input, &ctx);
                }
            }
            // bare magic with nothing after, and magic truncated to 1..4 bytes
            for cut in 0..=4 {
                let input = &magic.to_le_bytes()[..cut];
                sweep_oneshot(&le, &dec, input, &format!("{ver} bare-magic cut={cut}"));
            }
        }
    }
}

/// `ZSTDv0x_decompressDCtx` over magic-prefixed buffers, verifying the
/// explicit-context form matches the one-shot form and matches across libs.
/// (v04 uses `ZSTDv04_decompressDCtx(DCtx*, ...)`, same ABI as the others.)
#[test]
fn dctx_decoders_magic_prefixed() {
    unsafe {
        let mut rng = Rng::new(0xB15_0002);
        for &(magic, ver) in LEGACY {
            let name = format!("ZSTD{ver}_decompressDCtx");
            if !has_both(&name) {
                continue; // v02/v03 expose only ZSTDv0x_decompress
            }
            let le = LegErr::new(ver);
            let ddctx = both::<FnDecDCtx>(&name);
            let (cc, rc) = both::<FnCreate>(&format!("ZSTD{ver}_createDCtx"));
            let (cf, rf) = both::<FnFree>(&format!("ZSTD{ver}_freeDCtx"));
            for &shape in &[Shape::Random, Shape::Text, Shape::Zeros, Shape::Sequential] {
                for &blen in &[0usize, 4, 40, 400, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    for &cap in &[0usize, 1, 64, input.len().max(1), 1 << 16] {
                        let cctx = cc();
                        let rctx = rc();
                        assert!(!cctx.is_null() && !rctx.is_null(), "{ver} createDCtx null");
                        let mut cbuf = vec![0u8; cap.max(1)];
                        let mut rbuf = vec![0u8; cap.max(1)];
                        let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                        let a = (ddctx.0)(cctx, cbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
                        let b = (ddctx.1)(rctx, rbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
                        let ctx = format!("{ver} dctx shape={shape:?} blen={blen} cap={cap}");
                        le.eq(&ctx, a, b);
                        if !le.is_c_err(a) {
                            assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                        }
                        cf(cctx);
                        rf(rctx);
                    }
                }
            }
        }
    }
}

/// `ZSTDv0x_findFrameSizeInfoLegacy`: writes `cSize` and `dBound` out-params.
/// Both must be identical across C and Rust for every magic-prefixed input.
#[test]
fn find_frame_size_info_legacy() {
    unsafe {
        let mut rng = Rng::new(0xB15_0003);
        for &(magic, ver) in LEGACY {
            let name = format!("ZSTD{ver}_findFrameSizeInfoLegacy");
            let f = both::<FnFindInfo>(&name);
            let le = LegErr::new(ver);
            for &shape in &[Shape::Random, Shape::Text, Shape::Zeros, Shape::Sequential, Shape::Repeating] {
                for &blen in &[4usize, 8, 32, 100, 512, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    // note: header requires cSize/dBound non-NULL; provide both.
                    let mut c_cs: size_t = 0xdead;
                    let mut r_cs: size_t = 0xdead;
                    let mut c_db: c_ulonglong = 0xbeef;
                    let mut r_db: c_ulonglong = 0xbeef;
                    (f.0)(input.as_ptr() as *const c_void, input.len(), &mut c_cs, &mut c_db);
                    (f.1)(input.as_ptr() as *const c_void, input.len(), &mut r_cs, &mut r_db);
                    let ctx = format!("{ver} findInfo shape={shape:?} blen={blen}");
                    // cSize may itself be an error code; compare via the classifier.
                    le.eq(&format!("{ctx} cSize"), c_cs, r_cs);
                    assert_eq!(c_db, r_db, "{ctx}: dBound C={c_db:#x} RS={r_db:#x}");
                }
            }
        }
    }
}

/// Legacy streaming decode entry points: create / reset / nextSrcSizeToDecompress
/// / decompressContinue, plus free. We prime with the exact requested next-size
/// and feed magic-prefixed data. C and Rust must advance identically.
#[test]
fn streaming_legacy_entry_points() {
    unsafe {
        let mut rng = Rng::new(0xB15_0004);
        for &(magic, ver) in LEGACY {
            let cont_name = format!("ZSTD{ver}_decompressContinue");
            let next_name = format!("ZSTD{ver}_nextSrcSizeToDecompress");
            if !has_both(&cont_name) || !has_both(&next_name) {
                continue;
            }
            let le = LegErr::new(ver);
            let (cc, rc) = both::<FnCreate>(&format!("ZSTD{ver}_createDCtx"));
            let (cf, rf) = both::<FnFree>(&format!("ZSTD{ver}_freeDCtx"));
            let cont = both::<FnDecCont>(&cont_name);
            let next = both::<FnNextSrc>(&next_name);
            let reset_name = format!("ZSTD{ver}_resetDCtx");
            let reset = if has_both(&reset_name) { Some(both::<FnReset>(&reset_name)) } else { None };

            for &shape in &[Shape::Random, Shape::Text, Shape::Sequential] {
                for &blen in &[8usize, 64, 512, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    let cctx = cc();
                    let rctx = rc();
                    assert!(!cctx.is_null() && !rctx.is_null(), "{ver} createDCtx null");
                    if let Some((cre, rre)) = &reset {
                        le.eq(&format!("{ver} reset"), cre(cctx), rre(rctx));
                    }
                    let mut ipos = 0usize;
                    let mut steps = 0usize;
                    loop {
                        // nextSrcSizeToDecompress must agree
                        let cn = (next.0)(cctx);
                        let rn = (next.1)(rctx);
                        le.eq(&format!("{ver} next step={steps}"), cn, rn);
                        if le.is_c_err(cn) || cn == 0 { break; }
                        let want = cn;
                        let avail = input.len().saturating_sub(ipos);
                        let take = want.min(avail);
                        if take == 0 { break; }
                        let mut cbuf = vec![0u8; 1 << 17];
                        let mut rbuf = vec![0u8; 1 << 17];
                        let a = (cont.0)(cctx, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(),
                            input[ipos..].as_ptr() as *const c_void, take);
                        let b = (cont.1)(rctx, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(),
                            input[ipos..].as_ptr() as *const c_void, take);
                        let ctx = format!("{ver} continue step={steps} blen={blen}");
                        le.eq(&ctx, a, b);
                        if !le.is_c_err(a) {
                            assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                        } else {
                            break;
                        }
                        ipos += take;
                        steps += 1;
                        if steps > 10_000 { break; }
                    }
                    cf(cctx);
                    rf(rctx);
                }
            }
        }
    }
}

/// The ZBUFFv0x buffered legacy decoders (v04/v05/v06/v07), driven over
/// magic-prefixed input with a dst-capacity/in-chunk sweep. Also exercises the
/// v04 dictionary variant `ZBUFFv04_decompressWithDictionary` and the
/// InitDictionary variants where exported.
#[test]
fn zbuff_legacy_decoders() {
    unsafe {
        let mut rng = Rng::new(0xB15_0005);
        // (version tag used by ZBUFF symbols, magic, init symbol)
        let versions = [
            ("v04", MAGIC_V04),
            ("v05", MAGIC_V05),
            ("v06", MAGIC_V06),
            ("v07", MAGIC_V07),
        ];
        for &(ver, magic) in &versions {
            let create = format!("ZBUFF{ver}_createDCtx");
            let free = format!("ZBUFF{ver}_freeDCtx");
            let init = format!("ZBUFF{ver}_decompressInit");
            let cont = format!("ZBUFF{ver}_decompressContinue");
            if !has_both(&create) || !has_both(&cont) {
                continue;
            }
            // ZBUFF error api for this legacy version.
            let (cis, ris) = both::<FnIsErr>(&format!("ZBUFF{ver}_isError"));
            let (cnm, rnm) = both::<FnErrName>(&format!("ZBUFF{ver}_getErrorName"));
            let eq = |ctx: &str, cr: size_t, rr: size_t| {
                let ci = cis(cr) != 0;
                let ri = ris(rr) != 0;
                assert_eq!(ci, ri, "{ctx}: ZBUFF{ver} isError C={ci} RS={ri} (raw C={cr:#x} RS={rr:#x})");
                let cn = cstr(cnm(cr));
                let rn = cstr(rnm(rr));
                assert_eq!(cn, rn, "{ctx}: ZBUFF{ver} name C={cn:?} RS={rn:?}");
                if !ci {
                    assert_eq!(cr, rr, "{ctx}: ZBUFF{ver} OK value C={cr:#x} RS={rr:#x}");
                }
                ci
            };

            let (cc, rc) = both::<FnCreate>(&create);
            let (cf, rf) = both::<FnFree>(&free);
            let cont = both::<FnZbuffCont>(&cont);
            let has_init = has_both(&init);
            let initf = if has_init { Some(both::<FnZbuffInit>(&init)) } else { None };
            // v04 uses ZBUFFv04_decompressWithDictionary instead of InitDictionary
            let dictname = if ver == "v04" {
                format!("ZBUFF{ver}_decompressWithDictionary")
            } else {
                format!("ZBUFF{ver}_decompressInitDictionary")
            };
            let dictf = if has_both(&dictname) { Some(both::<FnZbuffInitDict>(&dictname)) } else { None };

            for &shape in &[Shape::Random, Shape::Text, Shape::Sequential] {
                for &blen in &[0usize, 8, 64, 512, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    for &oc in &[0usize, 1, 64, 4096] {
                        let cctx = cc();
                        let rctx = rc();
                        assert!(!cctx.is_null() && !rctx.is_null(), "ZBUFF{ver} createDCtx null");
                        if let Some((ci, ri)) = &initf {
                            eq(&format!("ZBUFF{ver} init"), ci(cctx), ri(rctx));
                        } else if let Some((cd, rd)) = &dictf {
                            // no plain init exported (shouldn't happen); init with empty dict
                            eq(&format!("ZBUFF{ver} initDict(empty)"),
                               cd(cctx, std::ptr::null(), 0), rd(rctx, std::ptr::null(), 0));
                        }
                        let mut ipos = 0usize;
                        let mut steps = 0usize;
                        let mut cbuf = vec![0u8; oc.max(1)];
                        let mut rbuf = vec![0u8; oc.max(1)];
                        loop {
                            let avail = input.len().saturating_sub(ipos);
                            let mut c_ss = avail;
                            let mut r_ss = avail;
                            let mut c_dc = oc;
                            let mut r_dc = oc;
                            let sp = if avail == 0 { std::ptr::null() } else { input[ipos..].as_ptr() as *const c_void };
                            let a = (cont.0)(cctx, cbuf.as_mut_ptr() as *mut c_void, &mut c_dc, sp, &mut c_ss);
                            let b = (cont.1)(rctx, rbuf.as_mut_ptr() as *mut c_void, &mut r_dc, sp, &mut r_ss);
                            let ctx = format!("ZBUFF{ver} cont shape={shape:?} blen={blen} oc={oc} step={steps}");
                            let is_err = eq(&ctx, a, b);
                            assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
                            assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
                            if !is_err {
                                assert_bytes_eq(&format!("{ctx} out"), &cbuf[..c_dc], &rbuf[..r_dc]);
                            }
                            if is_err || a == 0 { break; }
                            if c_ss == 0 && c_dc == 0 { break; }
                            ipos += c_ss;
                            steps += 1;
                            if steps > 100_000 { break; }
                        }
                        cf(cctx);
                        rf(rctx);
                    }
                }
            }

            // dictionary init error/valid path: NULL/0 and a small dict
            if let Some((cd, rd)) = &dictf {
                let cctx = cc();
                let rctx = rc();
                eq("dict null,0", cd(cctx, std::ptr::null(), 0), rd(rctx, std::ptr::null(), 0));
                let dict = gen(Shape::Text, 256, &mut rng);
                let cctx2 = cc();
                let rctx2 = rc();
                eq("dict small", cd(cctx2, dict.as_ptr() as *const c_void, dict.len()),
                   rd(rctx2, dict.as_ptr() as *const c_void, dict.len()));
                cf(cctx); rf(rctx); cf(cctx2); rf(rctx2);
            }
        }
    }
}

/// Feed legacy-magic buffers to the MODERN top-level API and assert identical
/// results. The modern `ZSTD_decompress` auto-handles legacy magics >= v05
/// (build sets ZSTD_LEGACY_SUPPORT=5); older magics must produce identical
/// errors in both libs.
#[test]
fn legacy_magic_into_modern_api() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDec4>("ZSTD_decompress");
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let (cif, rif) = both::<FnIsFrame>("ZSTD_isFrame");
        let (cffc, rffc) = both::<FnFindComp>("ZSTD_findFrameCompressedSize");
        type FnGetFH = unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t) -> size_t;
        let (cgfh, rgfh) = both::<FnGetFH>("ZSTD_getFrameHeader");

        let mut rng = Rng::new(0xB15_0006);
        for &(magic, ver) in LEGACY {
            for &shape in ALL_SHAPES {
                for &blen in &[0usize, 1, 4, 32, 200, 2048] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    let p = input.as_ptr() as *const c_void;
                    let ctx = format!("{ver}->modern shape={shape:?} blen={blen}");

                    // getFrameContentSize (returns UNKNOWN/ERROR sentinels)
                    assert_eq!(cfcs(p, input.len()), rfcs(p, input.len()), "{ctx} getFCS");
                    // isFrame
                    assert_eq!(cif(p, input.len()), rif(p, input.len()), "{ctx} isFrame");
                    // findFrameCompressedSize
                    e.eq(&format!("{ctx} findComp"), cffc(p, input.len()), rffc(p, input.len()));
                    // getFrameHeader
                    let mut ch: ZSTD_frameHeader = std::mem::zeroed();
                    let mut rh: ZSTD_frameHeader = std::mem::zeroed();
                    let a = cgfh(&mut ch, p, input.len());
                    let b = rgfh(&mut rh, p, input.len());
                    e.eq(&format!("{ctx} getFrameHeader"), a, b);
                    if a == 0 {
                        assert_eq!(ch, rh, "{ctx} frameHeader struct");
                    }
                    // one-shot decompress with a large dst
                    let mut cbuf = vec![0u8; 1 << 18];
                    let mut rbuf = vec![0u8; 1 << 18];
                    let da = cd(cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), p, input.len());
                    let db = rd(rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), p, input.len());
                    e.eq(&format!("{ctx} modern decompress"), da, db);
                    if !e.c.is_err(da) {
                        assert_bytes_eq(&format!("{ctx} modern out"), &cbuf[..da], &rbuf[..db]);
                    }
                }
            }
        }
    }
}

/// Feed legacy-magic buffers to `ZSTD_decompressStream` and `ZSTD_getFrameHeader`
/// (streaming path) and assert identical trajectories.
#[test]
fn legacy_magic_into_modern_stream() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCreate>("ZSTD_createDStream");
        let (cf, rf) = both::<FnFree>("ZSTD_freeDStream");
        type FnDecStream =
            unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;
        let (cds, rds) = both::<FnDecStream>("ZSTD_decompressStream");

        let mut rng = Rng::new(0xB15_0007);
        for &(magic, ver) in LEGACY {
            for &shape in &[Shape::Random, Shape::Text, Shape::Sequential, Shape::Repeating] {
                for &blen in &[0usize, 8, 64, 512, 4096] {
                    let input = magic_buf(magic, shape, blen, &mut rng);
                    let cctx = cc();
                    let rctx = rc();
                    assert!(!cctx.is_null() && !rctx.is_null(), "createDStream null");
                    let mut cout = vec![0u8; 1 << 17];
                    let mut rout = vec![0u8; 1 << 17];
                    let mut steps = 0usize;
                    let mut cin_pos = 0usize;
                    let mut rin_pos = 0usize;
                    loop {
                        let mut cib = ZSTD_inBuffer { src: input.as_ptr() as *const c_void, size: input.len(), pos: cin_pos };
                        let mut rib = ZSTD_inBuffer { src: input.as_ptr() as *const c_void, size: input.len(), pos: rin_pos };
                        let mut cob = ZSTD_outBuffer { dst: cout.as_mut_ptr() as *mut c_void, size: cout.len(), pos: 0 };
                        let mut rob = ZSTD_outBuffer { dst: rout.as_mut_ptr() as *mut c_void, size: rout.len(), pos: 0 };
                        let a = cds(cctx, &mut cob, &mut cib);
                        let b = rds(rctx, &mut rob, &mut rib);
                        let ctx = format!("{ver} stream shape={shape:?} blen={blen} step={steps}");
                        e.eq(&ctx, a, b);
                        assert_eq!(cib.pos, rib.pos, "{ctx} in.pos");
                        assert_eq!(cob.pos, rob.pos, "{ctx} out.pos");
                        assert_bytes_eq(&format!("{ctx} out"), &cout[..cob.pos], &rout[..rob.pos]);
                        cin_pos = cib.pos;
                        rin_pos = rib.pos;
                        if e.c.is_err(a) || a == 0 { break; }
                        if cib.pos >= input.len() && cob.pos == 0 { break; }
                        steps += 1;
                        if steps > 1000 { break; }
                    }
                    cf(cctx);
                    rf(rctx);
                }
            }
        }
    }
}

/// Feed valid MODERN frames to the LEGACY one-shot entry points. A modern frame
/// carries the v0.8 magic which none of the v01..v07 decoders recognise, so
/// each must return an identical (error) result in C and Rust.
#[test]
fn modern_frames_into_legacy_decoders() {
    unsafe {
        let (cc, _) = both::<FnCompress>("ZSTD_compress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0xB15_0008);

        // produce a spread of real modern frames
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating, Shape::Zeros] {
            for &len in &[0usize, 1, 100, 4096, 40_000] {
                for &lvl in &[1i32, 9, 19] {
                    let src = gen(shape, len, &mut rng);
                    let mut buf = vec![0u8; cb(src.len()) + 64];
                    let n = cc(buf.as_mut_ptr() as *mut c_void, buf.len(), src.as_ptr() as *const c_void, src.len(), lvl);
                    if !Err2::new().c.is_err(n) {
                        buf.truncate(n);
                        frames.push(buf);
                    }
                }
            }
        }
        // also a v08 skippable frame
        frames.push(vec![0x50, 0x2A, 0x4D, 0x18, 4, 0, 0, 0, 9, 9, 9, 9]);

        for &(_magic, ver) in LEGACY {
            let le = LegErr::new(ver);
            let dec = both::<FnDec4>(&oneshot_name(ver));
            for (fi, f) in frames.iter().enumerate() {
                sweep_oneshot(&le, &dec, f, &format!("{ver} modern-frame#{fi}"));
            }
        }
    }
}


/// The ZBUFFv0x buffered-decoder recommended-size helpers must agree between C
/// and Rust, and `ZBUFFv07_createDCtx_advanced` must allocate identically.
#[test]
fn zbuff_legacy_recommended_and_advanced() {
    unsafe {
        type FnVoidSize = unsafe extern "C" fn() -> size_t;
        for ver in ["v04", "v05", "v06", "v07"] {
            for kind in ["recommendedDInSize", "recommendedDOutSize"] {
                let name = format!("ZBUFF{ver}_{kind}");
                if !has_both(&name) {
                    continue;
                }
                let (a, b) = both::<FnVoidSize>(&name);
                assert_eq!(a(), b(), "{name}");
            }
        }
        // ZBUFFv07_createDCtx_advanced(customMem) + free lifecycle
        if has_both("ZBUFFv07_createDCtx_advanced") {
            #[repr(C)]
            #[derive(Clone, Copy)]
            struct CustomMem {
                alloc: *mut c_void,
                free: *mut c_void,
                opaque: *mut c_void,
            }
            type FnCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;
            let (cca, rca) = both::<FnCreateAdv>("ZBUFFv07_createDCtx_advanced");
            let (cfree, rfree) = both::<FnFree>("ZBUFFv07_freeDCtx");
            let null = CustomMem { alloc: std::ptr::null_mut(), free: std::ptr::null_mut(), opaque: std::ptr::null_mut() };
            for _ in 0..32 {
                let cx = cca(null);
                let rx = rca(null);
                assert!(!cx.is_null() && !rx.is_null(), "ZBUFFv07_createDCtx_advanced null");
                assert_eq!(cfree(cx) != 0, rfree(rx) != 0, "free adv isErr");
            }
        }
    }
}
