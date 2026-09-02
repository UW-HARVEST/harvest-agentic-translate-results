//! Phase C: differential tests for the LEGACY decoders (v01..v07) — ERROR
//! paths and full symbol coverage of the remaining legacy entry points.
//!
//! We cannot synthesize genuine legacy frames, so we drive the decoders with:
//!   (a) buffers that start with each version's exact magic number followed by
//!       random / structured payloads,
//!   (b) exhaustive single-byte mutation sweeps of those buffers,
//!   (c) every truncation length from 0 to N,
//!   (d) thousands of fixed-seed random-garbage buffers.
//! For every input, the C and Rust libraries MUST return the identical result:
//! we compare BOTH the version's own `ZSTDv0x_isError` boolean AND, where the
//! version exports it, the `ZSTDv0x_getErrorName` string (never just "both
//! failed"), plus the raw value on success.
//!
//! This file also covers the remaining exported legacy symbols not exercised in
//! `b15_legacy.rs`: the advanced ZSTD entry points (`sizeofDCtx`,
//! `estimateDCtxSize`, `decompressBegin[_usingDict]`, `copyDCtx`,
//! `decompress_usingDict`, `decompress_usingPreparedDCtx`, `decompressBlock`,
//! `insertBlock`, `isSkipFrame`, `getDecompressedSize`, `createDCtx_advanced`,
//! `getFrameParams`, the v07 DDict API) and the FSE/HUF v05/v06/v07 sub-decoders.
//!
//! Every call crosses the FFI boundary via `both`; a handle from one library is
//! NEVER passed to the other.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

use std::os::raw::{c_char, c_ulonglong};

// ------------------------------------------------------- legacy magic numbers
const MAGIC_V01: u32 = 0xFD2FB51E;
const MAGIC_V02: u32 = 0xFD2FB522;
const MAGIC_V03: u32 = 0xFD2FB523;
const MAGIC_V04: u32 = 0xFD2FB524;
const MAGIC_V05: u32 = 0xFD2FB525;
const MAGIC_V06: u32 = 0xFD2FB526;
const MAGIC_V07: u32 = 0xFD2FB527;

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
type FnDec4 = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnIsErr = unsafe extern "C" fn(size_t) -> c_uint;
type FnErrName = unsafe extern "C" fn(size_t) -> *const c_char;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(ZSTDv07_customMem) -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnBegin = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnBeginDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnSizeofDCtx = unsafe extern "C" fn(*const c_void) -> size_t;
type FnVoidToSize = unsafe extern "C" fn() -> size_t;
type FnDecUsingDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecBlock = unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnInsertBlock = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnIsSkip = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnGetDecompSize = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnGetFrameParams = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
// v05 uses ZSTDv05_decompress_usingPreparedDCtx(dctx, refDCtx, dst, cap, src, srcSize)
type FnDecPrepared =
    unsafe extern "C" fn(*mut c_void, *const c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;
type FnFreeDDict = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecUsingDDict =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, *const c_void) -> size_t;

// FSE/HUF
type FnFHDec = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t; // (dst,dstSize,cSrc,cSrcSize)
type FnReadNCount =
    unsafe extern "C" fn(*mut i16, *mut c_uint, *mut c_uint, *const c_void, size_t) -> size_t;

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTDv07_customMem {
    alloc: *mut c_void,
    free: *mut c_void,
    opaque: *mut c_void,
}
impl ZSTDv07_customMem {
    fn null() -> Self {
        ZSTDv07_customMem { alloc: std::ptr::null_mut(), free: std::ptr::null_mut(), opaque: std::ptr::null_mut() }
    }
}

// --------------------------------------------------------- per-version err api
struct LegErr {
    is_err: (libloading::Symbol<'static, FnIsErr>, libloading::Symbol<'static, FnIsErr>),
    name: Option<(libloading::Symbol<'static, FnErrName>, libloading::Symbol<'static, FnErrName>)>,
}
impl LegErr {
    unsafe fn new(ver: &str) -> Self {
        let own_is = format!("ZSTD{ver}_isError");
        let is_err = if has_both(&own_is) {
            both::<FnIsErr>(&own_is)
        } else if has_both(&format!("ZBUFF{ver}_isError")) {
            both::<FnIsErr>(&format!("ZBUFF{ver}_isError"))
        } else {
            both::<FnIsErr>("ZSTD_isError")
        };
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

// A generic (isError,getErrorName) pair used for FSE/HUF prefixes. Some HUF
// versions (e.g. HUFv06) do not export their own error API; since every legacy
// error function delegates to the shared `ERR_*` implementation, we fall back
// to the sibling FSE prefix of the same version, then to the modern `ZSTD_*`.
struct GenErr {
    is_err: (libloading::Symbol<'static, FnIsErr>, libloading::Symbol<'static, FnIsErr>),
    name: (libloading::Symbol<'static, FnErrName>, libloading::Symbol<'static, FnErrName>),
}
impl GenErr {
    unsafe fn new(prefix: &str) -> Self {
        // prefix like "FSEv06" / "HUFv07"; sibling FSE prefix shares the version
        let ver = &prefix[3..]; // "v06"
        let is_name = if has_both(&format!("{prefix}_isError")) {
            format!("{prefix}_isError")
        } else if has_both(&format!("FSE{ver}_isError")) {
            format!("FSE{ver}_isError")
        } else {
            "ZSTD_isError".to_string()
        };
        let nm_name = if has_both(&format!("{prefix}_getErrorName")) {
            format!("{prefix}_getErrorName")
        } else if has_both(&format!("FSE{ver}_getErrorName")) {
            format!("FSE{ver}_getErrorName")
        } else {
            "ZSTD_getErrorName".to_string()
        };
        GenErr {
            is_err: both::<FnIsErr>(&is_name),
            name: both::<FnErrName>(&nm_name),
        }
    }
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let c_is = (self.is_err.0)(cr) != 0;
        let r_is = (self.is_err.1)(rr) != 0;
        assert_eq!(c_is, r_is, "{ctx}: isError C={c_is} RS={r_is} (raw C={cr:#x} RS={rr:#x})");
        let cn = cstr((self.name.0)(cr));
        let rn = cstr((self.name.1)(rr));
        assert_eq!(cn, rn, "{ctx}: getErrorName C={cn:?} RS={rn:?} (raw C={cr:#x} RS={rr:#x})");
        if !c_is {
            assert_eq!(cr, rr, "{ctx}: OK value differs C={cr:#x} RS={rr:#x}");
        }
    }
}

// ------------------------------------------------------------- input builders
fn magic_buf(magic: u32, shape: Shape, body_len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + body_len);
    v.extend_from_slice(&magic.to_le_bytes());
    let body = gen(shape, body_len, rng);
    v.extend_from_slice(&body);
    v
}

unsafe fn run_oneshot(le: &LegErr, dec: &(libloading::Symbol<'static, FnDec4>, libloading::Symbol<'static, FnDec4>),
                      input: &[u8], cap: usize, ctx: &str) {
    let mut cbuf = vec![0u8; cap.max(1)];
    let mut rbuf = vec![0u8; cap.max(1)];
    let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
    let a = (dec.0)(cbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
    let b = (dec.1)(rbuf.as_mut_ptr() as *mut c_void, cap, sp, input.len());
    le.eq(ctx, a, b);
    if !le.is_c_err(a) {
        assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
    }
}

// -------------------------------------------------------------------- tests

/// Exhaustive single-byte mutation sweep of magic-prefixed buffers through each
/// legacy one-shot decoder. Every mutation must produce identical results.
#[test]
fn oneshot_single_byte_mutation_sweep() {
    unsafe {
        let mut rng = Rng::new(0xC13_0001);
        let mut_vals: [u8; 5] = [0x01, 0x40, 0x80, 0xc0, 0xff];
        for &(magic, ver) in LEGACY {
            let le = LegErr::new(ver);
            let dec = both::<FnDec4>(&format!("ZSTD{ver}_decompress"));
            // compact seeds keep the (byte x val) product tractable
            for &shape in &[Shape::Random, Shape::Text, Shape::Sequential, Shape::Zeros] {
                let seed = magic_buf(magic, shape, 60, &mut rng);
                for pos in 0..seed.len() {
                    for &mv in &mut_vals {
                        let mut m = seed.clone();
                        m[pos] ^= mv;
                        run_oneshot(&le, &dec, &m, 1 << 16,
                            &format!("{ver} mut shape={shape:?} pos={pos} xor={mv:#x}"));
                    }
                }
            }
        }
    }
}

/// Every truncation length 0..=N of magic-prefixed buffers through each legacy
/// one-shot decoder, over a small dst and a large dst.
#[test]
fn oneshot_truncation_sweep() {
    unsafe {
        let mut rng = Rng::new(0xC13_0002);
        for &(magic, ver) in LEGACY {
            let le = LegErr::new(ver);
            let dec = both::<FnDec4>(&format!("ZSTD{ver}_decompress"));
            for &shape in &[Shape::Random, Shape::Text, Shape::Repeating] {
                let full = magic_buf(magic, shape, 200, &mut rng);
                for cut in 0..=full.len() {
                    for &cap in &[1usize, 1 << 16] {
                        run_oneshot(&le, &dec, &full[..cut], cap,
                            &format!("{ver} trunc shape={shape:?} cut={cut} cap={cap}"));
                    }
                }
            }
        }
    }
}

/// Thousands of fixed-seed random-garbage buffers through each legacy one-shot
/// decoder, plus a dst-capacity sweep {0,1,small,large}.
#[test]
fn oneshot_random_garbage() {
    unsafe {
        let mut rng = Rng::new(0xC13_0003);
        for &(magic, ver) in LEGACY {
            let le = LegErr::new(ver);
            let dec = both::<FnDec4>(&format!("ZSTD{ver}_decompress"));
            for i in 0..2500 {
                let n = rng.below(400);
                let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
                // half the time, prepend this version's magic to reach deeper code
                if i % 2 == 0 && n >= 4 {
                    buf[..4].copy_from_slice(&magic.to_le_bytes());
                }
                let cap = *[0usize, 1, 64, 1 << 16].get(i % 4).unwrap();
                run_oneshot(&le, &dec, &buf, cap, &format!("{ver} garbage #{i} n={n} cap={cap}"));
            }
        }
    }
}

/// Advanced legacy DCtx entry points for v05/v06/v07:
///   sizeofDCtx, estimateDCtxSize (v07), createDCtx_advanced (v07),
///   decompressBegin, decompressBegin_usingDict, copyDCtx, isSkipFrame (v07),
///   getDecompressedSize (v07), getFrameParams, decompress_usingDict,
///   decompress_usingPreparedDCtx (v05/v06), decompressBlock, insertBlock,
///   and the v07 DDict API.
#[test]
fn advanced_dctx_entry_points() {
    unsafe {
        let mut rng = Rng::new(0xC13_0004);
        let versions = [("v05", MAGIC_V05), ("v06", MAGIC_V06), ("v07", MAGIC_V07)];
        for &(ver, magic) in &versions {
            let le = LegErr::new(ver);
            let (cc, rc) = both::<FnCreate>(&format!("ZSTD{ver}_createDCtx"));
            let (cf, rf) = both::<FnFree>(&format!("ZSTD{ver}_freeDCtx"));

            // sizeofDCtx must match on a live context
            {
                let sd = both::<FnSizeofDCtx>(&format!("ZSTD{ver}_sizeofDCtx"));
                let cx = cc();
                let rx = rc();
                assert_eq!((sd.0)(cx), (sd.1)(rx), "{ver} sizeofDCtx");
                cf(cx);
                rf(rx);
            }
            // estimateDCtxSize (v07 only)
            if has_both(&format!("ZSTD{ver}_estimateDCtxSize")) {
                let ed = both::<FnVoidToSize>(&format!("ZSTD{ver}_estimateDCtxSize"));
                assert_eq!((ed.0)(), (ed.1)(), "{ver} estimateDCtxSize");
            }
            // createDCtx_advanced (v07 only) with null customMem
            if has_both(&format!("ZSTD{ver}_createDCtx_advanced")) {
                let ca = both::<FnCreateAdv>(&format!("ZSTD{ver}_createDCtx_advanced"));
                let cx = (ca.0)(ZSTDv07_customMem::null());
                let rx = (ca.1)(ZSTDv07_customMem::null());
                assert!(!cx.is_null() && !rx.is_null(), "{ver} createDCtx_advanced null");
                cf(cx);
                rf(rx);
            }

            // decompressBegin + decompressBegin_usingDict + copyDCtx
            {
                let beg = both::<FnBegin>(&format!("ZSTD{ver}_decompressBegin"));
                let begd = both::<FnBeginDict>(&format!("ZSTD{ver}_decompressBegin_usingDict"));
                let cpy = both::<FnCopy>(&format!("ZSTD{ver}_copyDCtx"));
                for _ in 0..16 {
                    let cx = cc();
                    let rx = rc();
                    le.eq(&format!("{ver} decompressBegin"), (beg.0)(cx), (beg.1)(rx));
                    // begin_usingDict with a small random dict + NULL/0
                    let dict = gen(Shape::Text, 1 + rng.below(512), &mut rng);
                    let cx2 = cc();
                    let rx2 = rc();
                    le.eq(&format!("{ver} beginDict"),
                        (begd.0)(cx2, dict.as_ptr() as *const c_void, dict.len()),
                        (begd.1)(rx2, dict.as_ptr() as *const c_void, dict.len()));
                    le.eq(&format!("{ver} beginDict null,0"),
                        (begd.0)(cx2, std::ptr::null(), 0), (begd.1)(rx2, std::ptr::null(), 0));
                    // copyDCtx: copy prepared -> fresh; no return, just must not diverge/crash
                    let cdst = cc();
                    let rdst = rc();
                    (cpy.0)(cdst, cx2 as *const c_void);
                    (cpy.1)(rdst, rx2 as *const c_void);
                    cf(cx); rf(rx); cf(cx2); rf(rx2); cf(cdst); rf(rdst);
                }
            }

            // isSkipFrame (v07 only): after a begin, query on the ctx
            if has_both(&format!("ZSTD{ver}_isSkipFrame")) {
                let isk = both::<FnIsSkip>(&format!("ZSTD{ver}_isSkipFrame"));
                let beg = both::<FnBegin>(&format!("ZSTD{ver}_decompressBegin"));
                let cx = cc();
                let rx = rc();
                (beg.0)(cx);
                (beg.1)(rx);
                assert_eq!((isk.0)(cx), (isk.1)(rx), "{ver} isSkipFrame");
                cf(cx);
                rf(rx);
            }

            // getDecompressedSize (v07): over magic-prefixed buffers
            if has_both(&format!("ZSTD{ver}_getDecompressedSize")) {
                let gds = both::<FnGetDecompSize>(&format!("ZSTD{ver}_getDecompressedSize"));
                for &shape in &[Shape::Random, Shape::Text] {
                    for &blen in &[0usize, 4, 40, 400] {
                        let input = magic_buf(magic, shape, blen, &mut rng);
                        assert_eq!(
                            (gds.0)(input.as_ptr() as *const c_void, input.len()),
                            (gds.1)(input.as_ptr() as *const c_void, input.len()),
                            "{ver} getDecompressedSize shape={shape:?} blen={blen}"
                        );
                    }
                }
            }

            // getFrameParams: compare return code (classifier) + output struct
            // bytes on success. Struct size differs per version; over-allocate.
            {
                let gfp = both::<FnGetFrameParams>(&format!("ZSTD{ver}_getFrameParams"));
                for &shape in &[Shape::Random, Shape::Text, Shape::Sequential] {
                    for &blen in &[0usize, 4, 8, 40, 400, 4096] {
                        let input = magic_buf(magic, shape, blen, &mut rng);
                        let mut cparm = [0u8; 128];
                        let mut rparm = [0u8; 128];
                        let a = (gfp.0)(cparm.as_mut_ptr() as *mut c_void, input.as_ptr() as *const c_void, input.len());
                        let b = (gfp.1)(rparm.as_mut_ptr() as *mut c_void, input.as_ptr() as *const c_void, input.len());
                        let ctx = format!("{ver} getFrameParams shape={shape:?} blen={blen}");
                        le.eq(&ctx, a, b);
                        if !le.is_c_err(a) {
                            assert_eq!(cparm, rparm, "{ctx}: params struct differs");
                        }
                    }
                }
            }

            // decompress_usingDict over magic-prefixed buffers with dict sweep
            {
                let dud = both::<FnDecUsingDict>(&format!("ZSTD{ver}_decompress_usingDict"));
                for &blen in &[0usize, 8, 64, 512] {
                    let input = magic_buf(magic, Shape::Random, blen, &mut rng);
                    for &dlen in &[0usize, 32, 512] {
                        let dict = gen(Shape::Text, dlen, &mut rng);
                        let dptr = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
                        let cx = cc();
                        let rx = rc();
                        let mut cbuf = vec![0u8; 1 << 16];
                        let mut rbuf = vec![0u8; 1 << 16];
                        let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                        let a = (dud.0)(cx, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), sp, input.len(), dptr, dict.len());
                        let b = (dud.1)(rx, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), sp, input.len(), dptr, dict.len());
                        let ctx = format!("{ver} decompress_usingDict blen={blen} dlen={dlen}");
                        le.eq(&ctx, a, b);
                        if !le.is_c_err(a) {
                            assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                        }
                        cf(cx);
                        rf(rx);
                    }
                }
            }

            // decompress_usingPreparedDCtx (v05/v06): prepare via begin_usingDict
            if has_both(&format!("ZSTD{ver}_decompress_usingPreparedDCtx")) {
                let dpp = both::<FnDecPrepared>(&format!("ZSTD{ver}_decompress_usingPreparedDCtx"));
                let begd = both::<FnBeginDict>(&format!("ZSTD{ver}_decompressBegin_usingDict"));
                for &blen in &[0usize, 8, 64, 512] {
                    let input = magic_buf(magic, Shape::Random, blen, &mut rng);
                    let dict = gen(Shape::Text, 128, &mut rng);
                    let cref = cc();
                    let rref = rc();
                    (begd.0)(cref, dict.as_ptr() as *const c_void, dict.len());
                    (begd.1)(rref, dict.as_ptr() as *const c_void, dict.len());
                    let cx = cc();
                    let rx = rc();
                    let mut cbuf = vec![0u8; 1 << 16];
                    let mut rbuf = vec![0u8; 1 << 16];
                    let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                    let a = (dpp.0)(cx, cref as *const c_void, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), sp, input.len());
                    let b = (dpp.1)(rx, rref as *const c_void, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), sp, input.len());
                    let ctx = format!("{ver} decompress_usingPreparedDCtx blen={blen}");
                    le.eq(&ctx, a, b);
                    if !le.is_c_err(a) {
                        assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                    }
                    cf(cx); rf(rx); cf(cref); rf(rref);
                }
            }

            // decompressBlock (v05/v06/v07) + insertBlock (v07 only): after a
            // begin, feed raw block bytes.
            if has_both(&format!("ZSTD{ver}_decompressBlock")) {
                let beg = both::<FnBegin>(&format!("ZSTD{ver}_decompressBegin"));
                let dblk = both::<FnDecBlock>(&format!("ZSTD{ver}_decompressBlock"));
                let iblk = if has_both(&format!("ZSTD{ver}_insertBlock")) {
                    Some(both::<FnInsertBlock>(&format!("ZSTD{ver}_insertBlock")))
                } else {
                    None
                };
                for &blen in &[0usize, 3, 16, 128, 1024] {
                    let block = gen(Shape::Random, blen, &mut rng);
                    let cx = cc();
                    let rx = rc();
                    (beg.0)(cx);
                    (beg.1)(rx);
                    let mut cbuf = vec![0u8; 1 << 17];
                    let mut rbuf = vec![0u8; 1 << 17];
                    let sp = if block.is_empty() { std::ptr::null() } else { block.as_ptr() as *const c_void };
                    let a = (dblk.0)(cx, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), sp, block.len());
                    let b = (dblk.1)(rx, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), sp, block.len());
                    let ctx = format!("{ver} decompressBlock blen={blen}");
                    le.eq(&ctx, a, b);
                    if !le.is_c_err(a) {
                        assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                    }
                    // insertBlock (records history); returns blockSize, must match
                    if let Some((ci, ri)) = &iblk {
                        le.eq(&format!("{ver} insertBlock blen={blen}"),
                            ci(cx, sp, block.len()), ri(rx, sp, block.len()));
                    }
                    cf(cx);
                    rf(rx);
                }
            }
        }

        // v07 DDict API
        if has_both("ZSTDv07_createDDict") {
            let le = LegErr::new("v07");
            let cd = both::<FnCreateDDict>("ZSTDv07_createDDict");
            let fd = both::<FnFreeDDict>("ZSTDv07_freeDDict");
            let dud = both::<FnDecUsingDDict>("ZSTDv07_decompress_usingDDict");
            let (cc, rc) = both::<FnCreate>("ZSTDv07_createDCtx");
            let (cf, rf) = both::<FnFree>("ZSTDv07_freeDCtx");
            for &dlen in &[0usize, 1, 64, 512, 4096] {
                let dict = gen(Shape::Text, dlen, &mut rng);
                let dptr = if dict.is_empty() { std::ptr::null() } else { dict.as_ptr() as *const c_void };
                let cddict = (cd.0)(dptr, dict.len());
                let rddict = (cd.1)(dptr, dict.len());
                // createDDict may return null for empty dict; compare nullness
                assert_eq!(cddict.is_null(), rddict.is_null(), "createDDict null-ness dlen={dlen}");
                if !cddict.is_null() {
                    for &blen in &[0usize, 8, 128] {
                        let input = magic_buf(MAGIC_V07, Shape::Random, blen, &mut rng);
                        let cx = cc();
                        let rx = rc();
                        let mut cbuf = vec![0u8; 1 << 16];
                        let mut rbuf = vec![0u8; 1 << 16];
                        let sp = if input.is_empty() { std::ptr::null() } else { input.as_ptr() as *const c_void };
                        let a = (dud.0)(cx, cbuf.as_mut_ptr() as *mut c_void, cbuf.len(), sp, input.len(), cddict as *const c_void);
                        let b = (dud.1)(rx, rbuf.as_mut_ptr() as *mut c_void, rbuf.len(), sp, input.len(), rddict as *const c_void);
                        let ctx = format!("v07 usingDDict dlen={dlen} blen={blen}");
                        le.eq(&ctx, a, b);
                        if !le.is_c_err(a) {
                            assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                        }
                        cf(cx);
                        rf(rx);
                    }
                    le.eq("v07 freeDDict", (fd.0)(cddict), (fd.1)(rddict));
                }
            }
        }
    }
}

/// FSE/HUF v05/v06/v07 error-classification functions over the full error-code
/// range, plus the buffer-taking decoders (`FSEv0x_decompress`,
/// `HUFv0x_decompress`, `HUFv0x_decompress{1,4}X{2,4}`) and `readNCount` /
/// `readStats` fed random and structured garbage. All results must match.
#[test]
fn fse_huf_error_and_decoders() {
    unsafe {
        // (a) isError / getErrorName over the full range for each prefix that
        // exports its own error API (HUFv06 does not — it's covered via FSEv06).
        for prefix in ["FSEv05", "HUFv05", "FSEv06", "FSEv07", "HUFv07"] {
            if !has_both(&format!("{prefix}_isError")) { continue; }
            let (cis, ris) = both::<FnIsErr>(&format!("{prefix}_isError"));
            let (cnm, rnm) = both::<FnErrName>(&format!("{prefix}_getErrorName"));
            for code in -50i64..=400 {
                let raw = (0u64).wrapping_sub(code as u64) as size_t;
                assert_eq!(cis(raw) != 0, ris(raw) != 0, "{prefix}_isError code={code}");
                assert_eq!(cstr(cnm(raw)), cstr(rnm(raw)), "{prefix}_getErrorName code={code}");
            }
        }

        // (b) buffer-taking decoders fed random garbage + structured buffers.
        let mut rng = Rng::new(0xC13_0005);
        let decoders: &[&str] = &[
            "FSEv05_decompress", "FSEv06_decompress", "FSEv07_decompress",
            "HUFv05_decompress", "HUFv06_decompress", "HUFv07_decompress",
            "HUFv05_decompress1X2", "HUFv05_decompress4X2", "HUFv05_decompress1X4", "HUFv05_decompress4X4",
            "HUFv06_decompress1X2", "HUFv06_decompress4X2", "HUFv06_decompress1X4", "HUFv06_decompress4X4",
            "HUFv07_decompress1X2", "HUFv07_decompress4X2", "HUFv07_decompress1X4", "HUFv07_decompress4X4",
        ];
        for &name in decoders {
            if !has_both(name) { continue; }
            // pick the matching error classifier prefix (FSEv0x / HUFv0x)
            let prefix = &name[..6];
            let ge = GenErr::new(prefix);
            let dec = both::<FnFHDec>(name);
            for i in 0..800 {
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let n = rng.below(300);
                let src = gen(shape, n, &mut rng);
                // dst capacity sweep {0,1,small,large}
                let cap = *[0usize, 1, 64, 1 << 15].get(i % 4).unwrap();
                let mut cbuf = vec![0u8; cap.max(1)];
                let mut rbuf = vec![0u8; cap.max(1)];
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let a = (dec.0)(cbuf.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                let b = (dec.1)(rbuf.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                let ctx = format!("{name} #{i} shape={shape:?} n={n} cap={cap}");
                ge.eq(&ctx, a, b);
                let c_is = (ge.is_err.0)(a) != 0;
                if !c_is {
                    assert_bytes_eq(&format!("{ctx} out"), &cbuf[..a], &rbuf[..b]);
                }
            }
        }

        // (c) FSE readNCount fed garbage: writes normalizedCounter/maxSV/tableLog.
        for prefix in ["FSEv05", "FSEv06", "FSEv07"] {
            if !has_both(&format!("{prefix}_readNCount")) { continue; }
            let ge = GenErr::new(prefix);
            let rn = both::<FnReadNCount>(&format!("{prefix}_readNCount"));
            for i in 0..600 {
                let n = rng.below(64);
                let src: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
                let mut c_nc = [0i16; 256];
                let mut r_nc = [0i16; 256];
                let mut c_msv: c_uint = 255;
                let mut r_msv: c_uint = 255;
                let mut c_tl: c_uint = 0;
                let mut r_tl: c_uint = 0;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let a = (rn.0)(c_nc.as_mut_ptr(), &mut c_msv, &mut c_tl, sp, src.len());
                let b = (rn.1)(r_nc.as_mut_ptr(), &mut r_msv, &mut r_tl, sp, src.len());
                let ctx = format!("{prefix}_readNCount #{i} n={n}");
                ge.eq(&ctx, a, b);
                if (ge.is_err.0)(a) == 0 {
                    assert_eq!(c_msv, r_msv, "{ctx} maxSV");
                    assert_eq!(c_tl, r_tl, "{ctx} tableLog");
                    assert_eq!(&c_nc[..], &r_nc[..], "{ctx} normalizedCounter");
                }
            }
        }

        // (d) HUFv07_readStats: (huffWeight[hwSize], rankStats[HUF_TABLELOG_MAX+1],
        //     *nbSymbols, *tableLog, src, srcSize). Feed garbage; compare all.
        if has_both("HUFv07_readStats") {
            let ge = GenErr::new("HUFv07");
            type FnReadStats = unsafe extern "C" fn(
                *mut u8, size_t, *mut c_uint, *mut c_uint, *mut c_uint, *const c_void, size_t,
            ) -> size_t;
            let rs = both::<FnReadStats>("HUFv07_readStats");
            for i in 0..600 {
                let n = rng.below(64);
                let src: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
                let mut c_hw = [0u8; 256];
                let mut r_hw = [0u8; 256];
                let mut c_rank = [0u32; 16];
                let mut r_rank = [0u32; 16];
                let mut c_ns: c_uint = 0;
                let mut r_ns: c_uint = 0;
                let mut c_tl: c_uint = 0;
                let mut r_tl: c_uint = 0;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let a = (rs.0)(c_hw.as_mut_ptr(), 256, c_rank.as_mut_ptr(), &mut c_ns, &mut c_tl, sp, src.len());
                let b = (rs.1)(r_hw.as_mut_ptr(), 256, r_rank.as_mut_ptr(), &mut r_ns, &mut r_tl, sp, src.len());
                let ctx = format!("HUFv07_readStats #{i} n={n}");
                ge.eq(&ctx, a, b);
                if (ge.is_err.0)(a) == 0 {
                    assert_eq!(c_ns, r_ns, "{ctx} nbSymbols");
                    assert_eq!(c_tl, r_tl, "{ctx} tableLog");
                    assert_eq!(&c_hw[..], &r_hw[..], "{ctx} huffWeight");
                    assert_eq!(&c_rank[..], &r_rank[..], "{ctx} rankStats");
                }
            }
        }
    }
}


// ---------------------------------------------- FSE/HUF DTable-level functions

type FnCreateDTable = unsafe extern "C" fn(c_uint) -> *mut c_uint;
type FnFreeDTable = unsafe extern "C" fn(*mut c_uint);
type FnBuildRaw = unsafe extern "C" fn(*mut c_uint, c_uint) -> size_t;
type FnBuildDTable = unsafe extern "C" fn(*mut c_uint, *const i16, c_uint, c_uint) -> size_t;
type FnBuildRle = unsafe extern "C" fn(*mut c_uint, u8) -> size_t;
type FnFseDecUsingDT =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint) -> size_t;
type FnReadDTable = unsafe extern "C" fn(*mut c_uint, *const c_void, size_t) -> size_t;
type FnHufDecUsingDT =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const c_uint) -> size_t;
// short* DTable variant for HUFv06_readDTableX2 (unsigned short*) and its usingDTable
type FnReadDTableU16 = unsafe extern "C" fn(*mut u16, *const c_void, size_t) -> size_t;
type FnHufDecUsingDTU16 =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, *const u16) -> size_t;
type FnHufDecDCtx =
    unsafe extern "C" fn(*mut c_uint, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnHufSelect = unsafe extern "C" fn(size_t, size_t) -> c_uint;

/// FSE `createDTable`/`freeDTable`/`buildDTable_raw`/`buildDTable_rle`/
/// `decompress_usingDTable` for v05/v06/v07. We build a *valid* DTable via the
/// raw/rle constructors (which fill every entry), then feed garbage compressed
/// streams to `decompress_usingDTable`; the result is bounded by dst capacity
/// and MUST match between C and Rust.
#[test]
fn fse_dtable_functions() {
    unsafe {
        let mut rng = Rng::new(0xC13_0006);
        for ver in ["v05", "v06", "v07"] {
            if !has_both(&format!("FSE{ver}_createDTable")) { continue; }
            let ge = GenErr::new(&format!("FSE{ver}"));
            let cdt = both::<FnCreateDTable>(&format!("FSE{ver}_createDTable"));
            let fdt = both::<FnFreeDTable>(&format!("FSE{ver}_freeDTable"));
            let braw = both::<FnBuildRaw>(&format!("FSE{ver}_buildDTable_raw"));
            let brle = both::<FnBuildRle>(&format!("FSE{ver}_buildDTable_rle"));
            let dud = both::<FnFseDecUsingDT>(&format!("FSE{ver}_decompress_usingDTable"));

            for &tableLog in &[1u32, 5, 9, 12] {
                let ct = (cdt.0)(tableLog);
                let rt = rt_or_panic(&cdt, tableLog);
                assert!(!ct.is_null() && !rt.is_null(), "FSE{ver} createDTable null");

                // build_raw with nbBits <= tableLog (raw builds a tableLog==nbBits
                // table, which must fit the allocated DTable), then decode garbage
                for &nbBits in &[1u32, 2, 4, 8] {
                    if nbBits > tableLog { continue; }
                    ge.eq(&format!("FSE{ver} buildDTable_raw tl={tableLog} nb={nbBits}"),
                        (braw.0)(ct, nbBits), (braw.1)(rt, nbBits));
                    for _ in 0..16 {
                        let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(128), &mut rng);
                        let cap = *[0usize, 1, 64, 4096].get(rng.below(4)).unwrap();
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                        let a = (dud.0)(cb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), ct);
                        let b = (dud.1)(rb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), rt);
                        let ctx = format!("FSE{ver} decompress_usingDTable(raw) tl={tableLog} nb={nbBits} cap={cap}");
                        ge.eq(&ctx, a, b);
                        if (ge.is_err.0)(a) == 0 {
                            assert_bytes_eq(&format!("{ctx} out"), &cb[..a], &rb[..b]);
                        }
                    }
                }
                // build_rle then decode garbage
                for &sym in &[0u8, 1, 128, 255] {
                    ge.eq(&format!("FSE{ver} buildDTable_rle tl={tableLog} sym={sym}"),
                        (brle.0)(ct, sym), (brle.1)(rt, sym));
                    for _ in 0..8 {
                        let src = gen(Shape::Random, rng.below(64), &mut rng);
                        let cap = *[0usize, 1, 64].get(rng.below(3)).unwrap();
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                        let a = (dud.0)(cb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), ct);
                        let b = (dud.1)(rb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), rt);
                        let ctx = format!("FSE{ver} decompress_usingDTable(rle) tl={tableLog} sym={sym} cap={cap}");
                        ge.eq(&ctx, a, b);
                        if (ge.is_err.0)(a) == 0 {
                            assert_bytes_eq(&format!("{ctx} out"), &cb[..a], &rb[..b]);
                        }
                    }
                }
                (fdt.0)(ct);
                (fdt.1)(rt);
            }

            // buildDTable (the normalizedCounter form): construct a valid
            // normalized distribution by hand (must sum to 2^tableLog) and build.
            // A wrong/divergent build would surface as differing table bytes or a
            // differing decode result below.
            {
                let bdt = both::<FnBuildDTable>(&format!("FSE{ver}_buildDTable"));
                let dud = both::<FnFseDecUsingDT>(&format!("FSE{ver}_decompress_usingDTable"));
                // tableLog=5, maxSymbolValue=3, counts summing to 32.
                let table_log: c_uint = 5;
                let max_sv: c_uint = 3;
                let norm: [i16; 4] = [16, 8, 4, 4]; // sums to 32 == 1<<5
                let ct = (cdt.0)(table_log);
                let rt = rt_or_panic(&cdt, table_log);
                ge.eq(&format!("FSE{ver} buildDTable"),
                    (bdt.0)(ct, norm.as_ptr(), max_sv, table_log),
                    (bdt.1)(rt, norm.as_ptr(), max_sv, table_log));
                for _ in 0..32 {
                    let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(128), &mut rng);
                    let cap = *[0usize, 1, 64, 4096].get(rng.below(4)).unwrap();
                    let mut cb = vec![0u8; cap.max(1)];
                    let mut rb = vec![0u8; cap.max(1)];
                    let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                    let a = (dud.0)(cb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), ct);
                    let b = (dud.1)(rb.as_mut_ptr() as *mut c_void, cap, sp, src.len(), rt);
                    let ctx = format!("FSE{ver} decompress_usingDTable(built) cap={cap}");
                    ge.eq(&ctx, a, b);
                    if (ge.is_err.0)(a) == 0 {
                        assert_bytes_eq(&format!("{ctx} out"), &cb[..a], &rb[..b]);
                    }
                }
                (fdt.0)(ct);
                (fdt.1)(rt);
            }
            // freeDTable(NULL) must be a safe no-op in both
            (fdt.0)(std::ptr::null_mut());
            (fdt.1)(std::ptr::null_mut());
        }
    }
}

// helper to create the Rust-side DTable (keeps borrow checker happy)
unsafe fn rt_or_panic(cdt: &(libloading::Symbol<'static, FnCreateDTable>, libloading::Symbol<'static, FnCreateDTable>), tl: c_uint) -> *mut c_uint {
    (cdt.1)(tl)
}

/// HUF `readDTableX2`/`readDTableX4` and the `_usingDTable` decoders for
/// v05/v06/v07, plus the v07 `_DCtx` variants, `selectDecoder`,
/// `decompress4X_hufOnly`, and `decompress{1,4}X_usingDTable`.
///
/// We build the DTable via `readDTableX2/X4` fed garbage; the build itself is a
/// differential test. Only when the build SUCCEEDS (a valid, fully-populated
/// table — never a length-0 hole) do we invoke the matching `_usingDTable`
/// decoder, so no decode can loop forever. Output is bounded by dst capacity.
#[test]
fn huf_dtable_functions() {
    unsafe {
        let mut rng = Rng::new(0xC13_0007);

        // X4 tables use unsigned* DTable for v05/v07; v06 readDTableX4 uses unsigned* too.
        for ver in ["v05", "v06", "v07"] {
            if !has_both(&format!("HUF{ver}_readDTableX4")) { continue; }
            let ge = GenErr::new(&format!("HUF{ver}"));
            let rdt4 = both::<FnReadDTable>(&format!("HUF{ver}_readDTableX4"));
            let dud1x4 = both::<FnHufDecUsingDT>(&format!("HUF{ver}_decompress1X4_usingDTable"));
            let dud4x4 = both::<FnHufDecUsingDT>(&format!("HUF{ver}_decompress4X4_usingDTable"));
            // DTable is unsigned[ 1 + (1<<tableLog) ] worst-case; over-allocate.
            for i in 0..1200 {
                let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(256), &mut rng);
                let mut cdt = vec![0u32; 1 << 14];
                let mut rdt = vec![0u32; 1 << 14];
                cdt[0] = 12; // HUF max tableLog
                rdt[0] = 12;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let cbld = (rdt4.0)(cdt.as_mut_ptr(), sp, src.len());
                let rbld = (rdt4.1)(rdt.as_mut_ptr(), sp, src.len());
                let ctx = format!("HUF{ver} readDTableX4 #{i}");
                ge.eq(&ctx, cbld, rbld);
                if (ge.is_err.0)(cbld) == 0 {
                    // tables must be byte-identical after a successful build
                    assert_eq!(&cdt[..], &rdt[..], "{ctx}: DTable bytes differ");
                    // now decode a bounded garbage stream through both usingDTable forms
                    let cs = gen(Shape::Random, 8 + rng.below(64), &mut rng);
                    let csp = cs.as_ptr() as *const c_void;
                    for &cap in &[0usize, 1, 64, 512] {
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let a = (dud1x4.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud1x4.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUF{ver} 1X4_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("1X4 out", &cb[..a], &rb[..b]); }
                        let a = (dud4x4.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud4x4.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUF{ver} 4X4_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("4X4 out", &cb[..a], &rb[..b]); }
                    }
                }
            }
        }

        // X2 tables: v05/v07 use unsigned* DTable; v06 uses unsigned short* DTable.
        for ver in ["v05", "v07"] {
            if !has_both(&format!("HUF{ver}_readDTableX2")) { continue; }
            let ge = GenErr::new(&format!("HUF{ver}"));
            let rdt2 = both::<FnReadDTable>(&format!("HUF{ver}_readDTableX2"));
            let dud1x2 = both::<FnHufDecUsingDT>(&format!("HUF{ver}_decompress1X2_usingDTable"));
            let dud4x2 = both::<FnHufDecUsingDT>(&format!("HUF{ver}_decompress4X2_usingDTable"));
            for i in 0..1200 {
                let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(256), &mut rng);
                let mut cdt = vec![0u32; 1 << 13];
                let mut rdt = vec![0u32; 1 << 13];
                cdt[0] = 12;
                rdt[0] = 12;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let cbld = (rdt2.0)(cdt.as_mut_ptr(), sp, src.len());
                let rbld = (rdt2.1)(rdt.as_mut_ptr(), sp, src.len());
                let ctx = format!("HUF{ver} readDTableX2 #{i}");
                ge.eq(&ctx, cbld, rbld);
                if (ge.is_err.0)(cbld) == 0 {
                    assert_eq!(&cdt[..], &rdt[..], "{ctx}: DTable bytes differ");
                    let cs = gen(Shape::Random, 8 + rng.below(64), &mut rng);
                    let csp = cs.as_ptr() as *const c_void;
                    for &cap in &[0usize, 1, 64, 512] {
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let a = (dud1x2.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud1x2.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUF{ver} 1X2_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("1X2 out", &cb[..a], &rb[..b]); }
                        let a = (dud4x2.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud4x2.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUF{ver} 4X2_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("4X2 out", &cb[..a], &rb[..b]); }
                    }
                }
            }
        }

        // v06 X2 uses unsigned short* DTable.
        if has_both("HUFv06_readDTableX2") {
            let ge = GenErr::new("HUFv06");
            let rdt2 = both::<FnReadDTableU16>("HUFv06_readDTableX2");
            let dud1x2 = both::<FnHufDecUsingDTU16>("HUFv06_decompress1X2_usingDTable");
            let dud4x2 = both::<FnHufDecUsingDTU16>("HUFv06_decompress4X2_usingDTable");
            for i in 0..1200 {
                let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(256), &mut rng);
                // DTable[0] header holds tableLog (as u16 pair). Over-allocate.
                let mut cdt = vec![0u16; 1 << 13];
                let mut rdt = vec![0u16; 1 << 13];
                cdt[0] = 12;
                rdt[0] = 12;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let cbld = (rdt2.0)(cdt.as_mut_ptr(), sp, src.len());
                let rbld = (rdt2.1)(rdt.as_mut_ptr(), sp, src.len());
                let ctx = format!("HUFv06 readDTableX2 #{i}");
                ge.eq(&ctx, cbld, rbld);
                if (ge.is_err.0)(cbld) == 0 {
                    assert_eq!(&cdt[..], &rdt[..], "{ctx}: DTable bytes differ");
                    let cs = gen(Shape::Random, 8 + rng.below(64), &mut rng);
                    let csp = cs.as_ptr() as *const c_void;
                    for &cap in &[0usize, 1, 64, 512] {
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let a = (dud1x2.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud1x2.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUFv06 1X2_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("v06 1X2 out", &cb[..a], &rb[..b]); }
                        let a = (dud4x2.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud4x2.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("HUFv06 4X2_usingDTable #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("v06 4X2 out", &cb[..a], &rb[..b]); }
                    }
                }
            }
        }

        // v07-only: _DCtx decoders, selectDecoder, decompress4X_hufOnly,
        // decompress{1,4}X_usingDTable. The _DCtx variants take a caller DTable
        // buffer (they build it internally), so garbage is safe & bounded.
        // HUFv07_selectDecoder assumes dstSize > cSrcSize (so Q = cSrcSize*16/dstSize
        // is < 16, a valid index into the internal timing table). Outside that
        // documented precondition C reads out of bounds (UB), so we respect it.
        if has_both("HUFv07_selectDecoder") {
            let sel = both::<FnHufSelect>("HUFv07_selectDecoder");
            for _ in 0..2000 {
                let dst = 1 + rng.below(1 << 18);
                // ensure 0 < csrc < dst  =>  Q in 0..=15
                let csrc = rng.below(dst);
                assert_eq!((sel.0)(dst, csrc), (sel.1)(dst, csrc),
                    "HUFv07_selectDecoder dst={dst} csrc={csrc}");
            }
        }
        for name in [
            "HUFv07_decompress1X2_DCtx", "HUFv07_decompress4X2_DCtx",
            "HUFv07_decompress1X4_DCtx", "HUFv07_decompress4X4_DCtx",
            "HUFv07_decompress1X_DCtx", "HUFv07_decompress4X_DCtx",
            "HUFv07_decompress4X_hufOnly",
        ] {
            if !has_both(name) { continue; }
            let ge = GenErr::new("HUFv07");
            let dec = both::<FnHufDecDCtx>(name);
            for i in 0..600 {
                let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(256), &mut rng);
                let mut cdt = vec![0u32; 1 << 14];
                let mut rdt = vec![0u32; 1 << 14];
                cdt[0] = 12;
                rdt[0] = 12;
                // generic 1X/4X _DCtx decoders call selectDecoder internally, which
                // assumes dstSize > cSrcSize; keep dst strictly larger than src (but
                // still bounded) so we stay inside the documented precondition.
                let cap = src.len() + 1 + *[0usize, 16, 256, 4096].get(i % 4).unwrap();
                let mut cb = vec![0u8; cap.max(1)];
                let mut rb = vec![0u8; cap.max(1)];
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let a = (dec.0)(cdt.as_mut_ptr(), cb.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                let b = (dec.1)(rdt.as_mut_ptr(), rb.as_mut_ptr() as *mut c_void, cap, sp, src.len());
                let ctx = format!("{name} #{i} cap={cap}");
                ge.eq(&ctx, a, b);
                if (ge.is_err.0)(a) == 0 {
                    assert_bytes_eq(&format!("{ctx} out"), &cb[..a], &rb[..b]);
                }
            }
        }
        // v07 decompress{1,4}X_usingDTable: build via readDTableX4 (single-symbol
        // path uses X2, but the combined _usingDTable dispatches on the DTable's
        // own descriptor). Build a valid table via readDTableX2 then feed garbage.
        for name in ["HUFv07_decompress1X_usingDTable", "HUFv07_decompress4X_usingDTable"] {
            if !has_both(name) { continue; }
            let ge = GenErr::new("HUFv07");
            let rdt2 = both::<FnReadDTable>("HUFv07_readDTableX2");
            let dud = both::<FnHufDecUsingDT>(name);
            for i in 0..600 {
                let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.below(256), &mut rng);
                let mut cdt = vec![0u32; 1 << 13];
                let mut rdt = vec![0u32; 1 << 13];
                cdt[0] = 12;
                rdt[0] = 12;
                let sp = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                let cbld = (rdt2.0)(cdt.as_mut_ptr(), sp, src.len());
                let rbld = (rdt2.1)(rdt.as_mut_ptr(), sp, src.len());
                ge.eq(&format!("{name} build #{i}"), cbld, rbld);
                if (ge.is_err.0)(cbld) == 0 {
                    assert_eq!(&cdt[..], &rdt[..], "{name} #{i} DTable bytes");
                    let cs = gen(Shape::Random, 8 + rng.below(64), &mut rng);
                    let csp = cs.as_ptr() as *const c_void;
                    for &cap in &[0usize, 1, 64, 512] {
                        let mut cb = vec![0u8; cap.max(1)];
                        let mut rb = vec![0u8; cap.max(1)];
                        let a = (dud.0)(cb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), cdt.as_ptr());
                        let b = (dud.1)(rb.as_mut_ptr() as *mut c_void, cap, csp, cs.len(), rdt.as_ptr());
                        ge.eq(&format!("{name} #{i} cap={cap}"), a, b);
                        if (ge.is_err.0)(a) == 0 { assert_bytes_eq("out", &cb[..a], &rb[..b]); }
                    }
                }
            }
        }
    }
}
