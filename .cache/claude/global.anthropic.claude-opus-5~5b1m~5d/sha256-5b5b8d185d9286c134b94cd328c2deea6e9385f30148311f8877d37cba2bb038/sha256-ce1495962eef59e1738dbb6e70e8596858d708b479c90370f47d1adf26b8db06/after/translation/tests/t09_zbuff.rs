//! Differential tests for the DEPRECATED ZBUFF streaming API
//! (`c_src/src/deprecated/zbuff.h`, `zbuff_{common,compress,decompress}.c`).
//!
//! The whole compression state machine is driven in lock step through `dlopen`ed
//! exports of both shared libraries:
//!
//!   `ZBUFF_compressInit*` -> repeated `ZBUFF_compressContinue` with randomized
//!   input *and* output chunk sizes (including 0-, 1- and 2-byte output buffers
//!   that force partial flushes) -> `ZBUFF_compressFlush` until drained ->
//!   `ZBUFF_compressEnd` until drained.
//!
//! At *every* step C and Rust must return the same code, report the same number
//! of consumed and produced bytes, and emit identical bytes. The resulting
//! frames are then fed back through `ZBUFF_decompressContinue` in randomized
//! chunks with the same step-by-step assertions, and finally checked to
//! regenerate the original input.
//!
//! Also covered: every compression level, dictionaries (empty / tiny / large),
//! 0-byte input, input larger than one block, `ZBUFF_compressInit_advanced`
//! with valid and rejected `ZSTD_parameters`, custom allocators, NULL/zero-size
//! buffers and calling the API out of order (continue/flush/end before init,
//! continue after end, ...).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

mod common;
use common::*;

use std::os::raw::{c_char, c_int, c_uint, c_void};

// ------------------------------------------------------------------ fn types

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnCreateAdv = unsafe extern "C" fn(CustomMem) -> *mut c_void;
type FnCtxToSz = unsafe extern "C" fn(*mut c_void) -> usize;
type FnSzVoid = unsafe extern "C" fn() -> usize;
type FnIsError = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;

type FnInitLevel = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
type FnInitDict = unsafe extern "C" fn(*mut c_void, *const u8, usize, c_int) -> usize;
type FnInitAdv =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, ZSTDParameters, u64) -> usize;
type FnCont =
    unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize, *const u8, *mut usize) -> usize;
type FnFlush = unsafe extern "C" fn(*mut c_void, *mut u8, *mut usize) -> usize;
type FnDInit = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDInitDict = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> usize;

type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnCompressBound = unsafe extern "C" fn(usize) -> usize;
type FnGetParams = unsafe extern "C" fn(c_int, u64, usize) -> ZSTDParameters;
type FnIntVoid = unsafe extern "C" fn() -> c_int;
type FnBufToU64 = unsafe extern "C" fn(*const u8, usize) -> u64;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;

/// `ZSTD_customMem` — `{ customAlloc, customFree, opaque }`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CustomMem {
    pub custom_alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub custom_free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaque: *mut c_void,
}

/// `ZSTD_compressionParameters` (zstd.h) — field order is ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTDCompressionParameters {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: c_int,
}

/// `ZSTD_frameParameters`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTDFrameParameters {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}

/// `ZSTD_parameters`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ZSTDParameters {
    pub cParams: ZSTDCompressionParameters,
    pub fParams: ZSTDFrameParameters,
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

fn looks_like_error(v: usize) -> bool {
    v >= usize::MAX - 250
}

// ------------------------------------------------------------- symbol bundles

/// The ZBUFF compression surface as plain function pointers (one bundle per
/// library) so the streaming driver can be written once.
#[derive(Copy, Clone)]
struct CApi {
    create: FnCreate,
    create_adv: FnCreateAdv,
    free: FnCtxToSz,
    init: FnInitLevel,
    init_dict: FnInitDict,
    init_adv: FnInitAdv,
    cont: FnCont,
    flush: FnFlush,
    end: FnFlush,
    rec_in: FnSzVoid,
    rec_out: FnSzVoid,
}

#[derive(Copy, Clone)]
struct DApi {
    create: FnCreate,
    create_adv: FnCreateAdv,
    free: FnCtxToSz,
    init: FnDInit,
    init_dict: FnDInitDict,
    cont: FnCont,
    rec_in: FnSzVoid,
    rec_out: FnSzVoid,
}

macro_rules! grab {
    ($i:expr, $t:ty, $n:expr) => {{
        let (a, b) = $i.pair::<$t>($n);
        (*a, *b)
    }};
}

fn capi() -> (CApi, CApi) {
    let i = impls();
    let (c1, r1) = grab!(i, FnCreate, "ZBUFF_createCCtx");
    let (c2, r2) = grab!(i, FnCreateAdv, "ZBUFF_createCCtx_advanced");
    let (c3, r3) = grab!(i, FnCtxToSz, "ZBUFF_freeCCtx");
    let (c4, r4) = grab!(i, FnInitLevel, "ZBUFF_compressInit");
    let (c5, r5) = grab!(i, FnInitDict, "ZBUFF_compressInitDictionary");
    let (c6, r6) = grab!(i, FnInitAdv, "ZBUFF_compressInit_advanced");
    let (c7, r7) = grab!(i, FnCont, "ZBUFF_compressContinue");
    let (c8, r8) = grab!(i, FnFlush, "ZBUFF_compressFlush");
    let (c9, r9) = grab!(i, FnFlush, "ZBUFF_compressEnd");
    let (c10, r10) = grab!(i, FnSzVoid, "ZBUFF_recommendedCInSize");
    let (c11, r11) = grab!(i, FnSzVoid, "ZBUFF_recommendedCOutSize");
    (
        CApi {
            create: c1,
            create_adv: c2,
            free: c3,
            init: c4,
            init_dict: c5,
            init_adv: c6,
            cont: c7,
            flush: c8,
            end: c9,
            rec_in: c10,
            rec_out: c11,
        },
        CApi {
            create: r1,
            create_adv: r2,
            free: r3,
            init: r4,
            init_dict: r5,
            init_adv: r6,
            cont: r7,
            flush: r8,
            end: r9,
            rec_in: r10,
            rec_out: r11,
        },
    )
}

fn dapi() -> (DApi, DApi) {
    let i = impls();
    let (c1, r1) = grab!(i, FnCreate, "ZBUFF_createDCtx");
    let (c2, r2) = grab!(i, FnCreateAdv, "ZBUFF_createDCtx_advanced");
    let (c3, r3) = grab!(i, FnCtxToSz, "ZBUFF_freeDCtx");
    let (c4, r4) = grab!(i, FnDInit, "ZBUFF_decompressInit");
    let (c5, r5) = grab!(i, FnDInitDict, "ZBUFF_decompressInitDictionary");
    let (c6, r6) = grab!(i, FnCont, "ZBUFF_decompressContinue");
    let (c7, r7) = grab!(i, FnSzVoid, "ZBUFF_recommendedDInSize");
    let (c8, r8) = grab!(i, FnSzVoid, "ZBUFF_recommendedDOutSize");
    (
        DApi {
            create: c1,
            create_adv: c2,
            free: c3,
            init: c4,
            init_dict: c5,
            cont: c6,
            rec_in: c7,
            rec_out: c8,
        },
        DApi {
            create: r1,
            create_adv: r2,
            free: r3,
            init: r4,
            init_dict: r5,
            cont: r6,
            rec_in: r7,
            rec_out: r8,
        },
    )
}

/// `ZBUFF_isError` / `ZBUFF_getErrorName` on both sides, used everywhere so a
/// divergence is reported with the error's *meaning*.
struct Namer {
    c: FnErrName,
    r: FnErrName,
}

impl Namer {
    fn new() -> Namer {
        let i = impls();
        let (c, r) = grab!(i, FnErrName, "ZBUFF_getErrorName");
        Namer { c, r }
    }
    fn check(&self, ctx: &str, cret: usize, rret: usize) {
        assert_eq_dbg(ctx, cret, rret);
        if looks_like_error(cret) {
            unsafe {
                let (a, b) = (cstr((self.c)(cret)), cstr((self.r)(rret)));
                assert_eq_dbg(&format!("{ctx}: errorName"), a, b);
            }
        }
    }
}

// ------------------------------------------------------------ chunk size picks

const BIG_OUT: usize = 1 << 18;

/// Output-buffer size policy. `tiny` forces the partial-flush paths (0/1/2-byte
/// output buffers); `stalled` breaks a no-forward-progress deadlock by handing
/// over a large buffer.
fn pick_out(rng: &mut Rng, tiny: bool, stalled: bool) -> usize {
    if stalled {
        return BIG_OUT;
    }
    if tiny {
        match rng.below(8) {
            0 => 0,
            1 => 1,
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 7,
            6 => 17,
            _ => rng.range(1, 400),
        }
    } else {
        match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => rng.range(1, 64),
            3 => rng.range(1, 4096),
            4 => 1 << 16,
            _ => BIG_OUT,
        }
    }
}

fn pick_in(rng: &mut Rng, left: usize, stalled: bool) -> usize {
    if left == 0 {
        return 0;
    }
    if stalled {
        return left.min(1024).max(1);
    }
    match rng.below(7) {
        0 => 0,
        1 => 1,
        2 => 2.min(left),
        3 => 3.min(left),
        4 => left.min(rng.range(1, 300)),
        5 => left.min(rng.range(1, 70_000)),
        _ => left,
    }
}

// ---------------------------------------------------------- streaming drivers

/// Run the compression state machine on both libraries in lock step.
/// The contexts must already be initialized identically. Returns the frame each
/// library produced (they are asserted equal chunk-by-chunk on the way).
fn drive_compress(
    tag: &str,
    c: &CApi,
    r: &CApi,
    ctxc: *mut c_void,
    ctxr: *mut c_void,
    src: &[u8],
    rng: &mut Rng,
    tiny: bool,
    namer: &Namer,
) -> Option<Vec<u8>> {
    let mut outc: Vec<u8> = Vec::new();
    let mut outr: Vec<u8> = Vec::new();
    let mut bufc = vec![0u8; BIG_OUT];
    let mut bufr = vec![0u8; BIG_OUT];

    let mut pos = 0usize;
    let mut stall = 0usize;
    let mut steps = 0usize;

    // ---- compressContinue
    while pos < src.len() {
        steps += 1;
        assert!(steps < 500_000, "{tag}: compressContinue did not terminate");
        let stalled = stall >= 3;
        let inb = pick_in(rng, src.len() - pos, stalled);
        let outb = pick_out(rng, tiny, stalled);
        bufc[..outb].fill(0xE1);
        bufr[..outb].fill(0xE1);
        let mut dc = outb;
        let mut dr = outb;
        let mut sc = inb;
        let mut sr = inb;
        let (cret, rret) = unsafe {
            (
                (c.cont)(
                    ctxc,
                    bufc.as_mut_ptr(),
                    &mut dc,
                    src[pos..].as_ptr(),
                    &mut sc,
                ),
                (r.cont)(
                    ctxr,
                    bufr.as_mut_ptr(),
                    &mut dr,
                    src[pos..].as_ptr(),
                    &mut sr,
                ),
            )
        };
        let ctx = format!("{tag}: compressContinue[#{steps} in={inb} out={outb}]");
        namer.check(&ctx, cret, rret);
        assert_eq_dbg(&format!("{ctx} consumed"), sc, sr);
        assert_eq_dbg(&format!("{ctx} produced"), dc, dr);
        assert_bytes_eq(&ctx, &bufc[..dc], &bufr[..dr]);
        if looks_like_error(cret) {
            return None;
        }
        outc.extend_from_slice(&bufc[..dc]);
        outr.extend_from_slice(&bufr[..dr]);
        pos += sc;
        if sc == 0 && dc == 0 {
            stall += 1;
        } else {
            stall = 0;
        }
    }

    // ---- compressFlush until the internal buffer is drained
    stall = 0;
    loop {
        steps += 1;
        assert!(steps < 500_000, "{tag}: compressFlush did not terminate");
        let outb = pick_out(rng, tiny, stall >= 3);
        bufc[..outb].fill(0xE2);
        bufr[..outb].fill(0xE2);
        let mut dc = outb;
        let mut dr = outb;
        let (cret, rret) = unsafe {
            (
                (c.flush)(ctxc, bufc.as_mut_ptr(), &mut dc),
                (r.flush)(ctxr, bufr.as_mut_ptr(), &mut dr),
            )
        };
        let ctx = format!("{tag}: compressFlush[#{steps} out={outb}]");
        namer.check(&ctx, cret, rret);
        assert_eq_dbg(&format!("{ctx} produced"), dc, dr);
        assert_bytes_eq(&ctx, &bufc[..dc], &bufr[..dr]);
        if looks_like_error(cret) {
            return None;
        }
        outc.extend_from_slice(&bufc[..dc]);
        outr.extend_from_slice(&bufr[..dr]);
        if cret == 0 {
            break;
        }
        if dc == 0 {
            stall += 1;
        } else {
            stall = 0;
        }
    }

    // ---- compressEnd until the epilogue is fully written
    stall = 0;
    loop {
        steps += 1;
        assert!(steps < 500_000, "{tag}: compressEnd did not terminate");
        let outb = pick_out(rng, tiny, stall >= 3);
        bufc[..outb].fill(0xE3);
        bufr[..outb].fill(0xE3);
        let mut dc = outb;
        let mut dr = outb;
        let (cret, rret) = unsafe {
            (
                (c.end)(ctxc, bufc.as_mut_ptr(), &mut dc),
                (r.end)(ctxr, bufr.as_mut_ptr(), &mut dr),
            )
        };
        let ctx = format!("{tag}: compressEnd[#{steps} out={outb}]");
        namer.check(&ctx, cret, rret);
        assert_eq_dbg(&format!("{ctx} produced"), dc, dr);
        assert_bytes_eq(&ctx, &bufc[..dc], &bufr[..dr]);
        if looks_like_error(cret) {
            return None;
        }
        outc.extend_from_slice(&bufc[..dc]);
        outr.extend_from_slice(&bufr[..dr]);
        if cret == 0 {
            break;
        }
        if dc == 0 {
            stall += 1;
        } else {
            stall = 0;
        }
    }

    assert_bytes_eq(&format!("{tag}: whole frame"), &outc, &outr);
    Some(outc)
}

/// Run the decompression state machine on both libraries in lock step.
fn drive_decompress(
    tag: &str,
    c: &DApi,
    r: &DApi,
    ctxc: *mut c_void,
    ctxr: *mut c_void,
    frame: &[u8],
    rng: &mut Rng,
    tiny: bool,
    namer: &Namer,
) -> Option<Vec<u8>> {
    let mut outc: Vec<u8> = Vec::new();
    let mut outr: Vec<u8> = Vec::new();
    let mut bufc = vec![0u8; BIG_OUT];
    let mut bufr = vec![0u8; BIG_OUT];

    let mut pos = 0usize;
    let mut stall = 0usize;
    let mut steps = 0usize;

    loop {
        steps += 1;
        assert!(steps < 500_000, "{tag}: decompressContinue did not terminate");
        let stalled = stall >= 3;
        let inb = pick_in(rng, frame.len() - pos, stalled);
        let outb = pick_out(rng, tiny, stalled);
        bufc[..outb].fill(0xD1);
        bufr[..outb].fill(0xD1);
        let mut dc = outb;
        let mut dr = outb;
        let mut sc = inb;
        let mut sr = inb;
        let (cret, rret) = unsafe {
            (
                (c.cont)(
                    ctxc,
                    bufc.as_mut_ptr(),
                    &mut dc,
                    frame[pos..].as_ptr(),
                    &mut sc,
                ),
                (r.cont)(
                    ctxr,
                    bufr.as_mut_ptr(),
                    &mut dr,
                    frame[pos..].as_ptr(),
                    &mut sr,
                ),
            )
        };
        let ctx = format!("{tag}: decompressContinue[#{steps} in={inb} out={outb}]");
        namer.check(&ctx, cret, rret);
        assert_eq_dbg(&format!("{ctx} consumed"), sc, sr);
        assert_eq_dbg(&format!("{ctx} produced"), dc, dr);
        assert_bytes_eq(&ctx, &bufc[..dc], &bufr[..dr]);
        if looks_like_error(cret) {
            return None;
        }
        outc.extend_from_slice(&bufc[..dc]);
        outr.extend_from_slice(&bufr[..dr]);
        pos += sc;
        if cret == 0 {
            break; // frame fully decoded
        }
        if sc == 0 && dc == 0 {
            stall += 1;
            if stall > 8 && pos >= frame.len() {
                break; // truncated input, no more progress possible
            }
        } else {
            stall = 0;
        }
    }

    assert_bytes_eq(&format!("{tag}: whole output"), &outc, &outr);
    Some(outc)
}

// ==================================================== 1. tools + error surface

#[test]
fn zbuff_tool_functions_and_error_surface_match() {
    let i = impls();
    let (c, r) = capi();
    let (dc, dr) = dapi();

    unsafe {
        assert_eq_dbg("ZBUFF_recommendedCInSize", (c.rec_in)(), (r.rec_in)());
        assert_eq_dbg("ZBUFF_recommendedCOutSize", (c.rec_out)(), (r.rec_out)());
        assert_eq_dbg("ZBUFF_recommendedDInSize", (dc.rec_in)(), (dr.rec_in)());
        assert_eq_dbg("ZBUFF_recommendedDOutSize", (dc.rec_out)(), (dr.rec_out)());

        // and they must agree with the modern equivalents they wrap
        let (cci, rci) = grab!(i, FnSzVoid, "ZSTD_CStreamInSize");
        let (cco, rco) = grab!(i, FnSzVoid, "ZSTD_CStreamOutSize");
        let (cdi, rdi) = grab!(i, FnSzVoid, "ZSTD_DStreamInSize");
        let (cdo, rdo) = grab!(i, FnSzVoid, "ZSTD_DStreamOutSize");
        assert_eq_dbg("recommendedCInSize == ZSTD_CStreamInSize", (c.rec_in)(), cci());
        assert_eq_dbg("recommendedCOutSize == ZSTD_CStreamOutSize", (c.rec_out)(), cco());
        assert_eq_dbg("recommendedDInSize == ZSTD_DStreamInSize", (dc.rec_in)(), cdi());
        assert_eq_dbg("recommendedDOutSize == ZSTD_DStreamOutSize", (dc.rec_out)(), cdo());
        assert_eq_dbg("rust recCIn == ZSTD_CStreamInSize", (r.rec_in)(), rci());
        assert_eq_dbg("rust recCOut == ZSTD_CStreamOutSize", (r.rec_out)(), rco());
        assert_eq_dbg("rust recDIn == ZSTD_DStreamInSize", (dr.rec_in)(), rdi());
        assert_eq_dbg("rust recDOut == ZSTD_DStreamOutSize", (dr.rec_out)(), rdo());
    }

    // ZBUFF_isError / ZBUFF_getErrorName over the whole plausible code space
    let (c_is, r_is) = grab!(i, FnIsError, "ZBUFF_isError");
    let (c_nm, r_nm) = grab!(i, FnErrName, "ZBUFF_getErrorName");
    let mut probes: Vec<usize> = Vec::new();
    for e in 0..=260usize {
        probes.push(0usize.wrapping_sub(e));
    }
    probes.extend([0, 1, 2, 3, 100, 1 << 20, usize::MAX / 2]);
    let mut rng = Rng::new(0x0B0F_E770_0000_0001);
    for _ in 0..300 {
        probes.push(rng.next_u64() as usize);
    }
    for p in probes {
        unsafe {
            assert_eq_dbg(&format!("ZBUFF_isError({p:#x})"), c_is(p), r_is(p));
            let (a, b) = (cstr(c_nm(p)), cstr(r_nm(p)));
            assert_eq_dbg(&format!("ZBUFF_getErrorName({p:#x})"), a, b);
        }
    }
}

// ================================================ 2. create / free / allocators

unsafe extern "C" fn t_alloc(_opaque: *mut c_void, size: usize) -> *mut c_void {
    use std::alloc::{alloc, Layout};
    let total = size + 16;
    let l = Layout::from_size_align(total, 16).unwrap();
    let p = alloc(l);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    (p as *mut usize).write(total);
    p.add(16) as *mut c_void
}

unsafe extern "C" fn t_free(_opaque: *mut c_void, addr: *mut c_void) {
    use std::alloc::{dealloc, Layout};
    if addr.is_null() {
        return;
    }
    let p = (addr as *mut u8).sub(16);
    let total = (p as *mut usize).read();
    dealloc(p, Layout::from_size_align(total, 16).unwrap());
}

#[test]
fn zbuff_create_free_and_advanced_match() {
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();

    unsafe {
        // plain create/free
        for _ in 0..8 {
            let a = (c.create)();
            let b = (r.create)();
            assert_eq_dbg("ZBUFF_createCCtx null-ness", a.is_null(), b.is_null());
            namer.check("ZBUFF_freeCCtx", (c.free)(a), (r.free)(b));

            let a = (dc.create)();
            let b = (dr.create)();
            assert_eq_dbg("ZBUFF_createDCtx null-ness", a.is_null(), b.is_null());
            namer.check("ZBUFF_freeDCtx", (dc.free)(a), (dr.free)(b));
        }

        // freeing NULL must be a no-op in both
        namer.check(
            "ZBUFF_freeCCtx(NULL)",
            (c.free)(std::ptr::null_mut()),
            (r.free)(std::ptr::null_mut()),
        );
        namer.check(
            "ZBUFF_freeDCtx(NULL)",
            (dc.free)(std::ptr::null_mut()),
            (dr.free)(std::ptr::null_mut()),
        );

        // custom allocators, including the half-specified cases
        let variants: [(&str, CustomMem); 4] = [
            (
                "all-null (=> default)",
                CustomMem {
                    custom_alloc: None,
                    custom_free: None,
                    opaque: std::ptr::null_mut(),
                },
            ),
            (
                "alloc-only",
                CustomMem {
                    custom_alloc: Some(t_alloc),
                    custom_free: None,
                    opaque: std::ptr::null_mut(),
                },
            ),
            (
                "free-only",
                CustomMem {
                    custom_alloc: None,
                    custom_free: Some(t_free),
                    opaque: std::ptr::null_mut(),
                },
            ),
            (
                "both",
                CustomMem {
                    custom_alloc: Some(t_alloc),
                    custom_free: Some(t_free),
                    opaque: 0xABCDusize as *mut c_void,
                },
            ),
        ];
        for (tag, cm) in variants {
            let a = (c.create_adv)(cm);
            let b = (r.create_adv)(cm);
            assert_eq_dbg(
                &format!("ZBUFF_createCCtx_advanced[{tag}] null-ness"),
                a.is_null(),
                b.is_null(),
            );
            if !a.is_null() {
                namer.check(
                    &format!("ZBUFF_freeCCtx[{tag}]"),
                    (c.free)(a),
                    (r.free)(b),
                );
            }
            let a = (dc.create_adv)(cm);
            let b = (dr.create_adv)(cm);
            assert_eq_dbg(
                &format!("ZBUFF_createDCtx_advanced[{tag}] null-ness"),
                a.is_null(),
                b.is_null(),
            );
            if !a.is_null() {
                namer.check(
                    &format!("ZBUFF_freeDCtx[{tag}]"),
                    (dc.free)(a),
                    (dr.free)(b),
                );
            }
        }
    }
}

// ============================================= 3. the full compression machine

/// Compress + decompress every shape at several sizes and levels, driving the
/// state machine with randomized chunking (and, for the smaller inputs, with
/// tiny output buffers that force partial flushes at every stage).
#[test]
fn zbuff_compress_stream_matches() {
    let i = impls();
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let (c_dec, r_dec) = grab!(i, FnDecompress, "ZSTD_decompress");
    let mut rng = Rng::new(0x0C01_2345_6789_AB01);

    let mut cases: Vec<(usize, i32, bool)> = Vec::new();
    for &len in &[0usize, 1, 2, 17, 300, 4096] {
        for &lvl in &[1i32, 3, 9] {
            cases.push((len, lvl, true)); // tiny output buffers
        }
    }
    for &len in &[20_000usize, 131_072, 131_073, 260_000] {
        for &lvl in &[1i32, 5] {
            cases.push((len, lvl, false));
        }
    }

    for shape in ALL_SHAPES {
        for &(len, lvl, tiny) in &cases {
            let src = gen_shape(shape, len, &mut rng);
            let tag = format!("cstream[{shape:?} len={len} lvl={lvl} tiny={tiny}]");
            unsafe {
                let ctxc = (c.create)();
                let ctxr = (r.create)();
                assert!(!ctxc.is_null() && !ctxr.is_null());
                namer.check(
                    &format!("{tag}: compressInit"),
                    (c.init)(ctxc, lvl),
                    (r.init)(ctxr, lvl),
                );
                let frame = drive_compress(
                    &tag, &c, &r, ctxc, ctxr, &src, &mut rng, tiny, &namer,
                );
                namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));

                let frame = frame.expect("compression must not fail on valid input");

                // the frame must decode back to the input in *both* libraries
                let mut oc = vec![0u8; len + 64];
                let mut orr = vec![0u8; len + 64];
                let a = c_dec(oc.as_mut_ptr(), oc.len(), frame.as_ptr(), frame.len());
                let b = r_dec(orr.as_mut_ptr(), orr.len(), frame.as_ptr(), frame.len());
                namer.check(&format!("{tag}: ZSTD_decompress"), a, b);
                assert_eq_dbg(&format!("{tag}: decoded size"), a, len);
                assert_bytes_eq(&format!("{tag}: decoded"), &src, &oc[..a]);
                assert_bytes_eq(&format!("{tag}: decoded (rust)"), &src, &orr[..b]);

                // ... and through the ZBUFF decompression machine, in lock step
                let dtag = format!("{tag} -> dstream");
                let a = (dc.create)();
                let b = (dr.create)();
                namer.check(
                    &format!("{dtag}: decompressInit"),
                    (dc.init)(a),
                    (dr.init)(b),
                );
                let out = drive_decompress(
                    &dtag, &dc, &dr, a, b, &frame, &mut rng, tiny, &namer,
                );
                namer.check(&format!("{dtag}: freeDCtx"), (dc.free)(a), (dr.free)(b));
                let out = out.expect("decompression must not fail on a valid frame");
                assert_bytes_eq(&format!("{dtag}: regenerated"), &src, &out);
            }
        }
    }
}

// ================================================== 4. every compression level

/// `ZBUFF_compressInit` over the *entire* level range reported by the library,
/// including the negative (fast) levels and the ultra levels.
#[test]
fn zbuff_all_compression_levels_match() {
    let i = impls();
    let (c, r) = capi();
    let namer = Namer::new();
    let (c_min, r_min) = grab!(i, FnIntVoid, "ZSTD_minCLevel");
    let (c_max, r_max) = grab!(i, FnIntVoid, "ZSTD_maxCLevel");
    let (c_dec, r_dec) = grab!(i, FnDecompress, "ZSTD_decompress");
    let (mn, mx) = unsafe {
        assert_eq_dbg("ZSTD_minCLevel", c_min(), r_min());
        assert_eq_dbg("ZSTD_maxCLevel", c_max(), r_max());
        (c_min(), c_max())
    };
    assert!(mn < 0 && mx >= 19);

    let mut rng = Rng::new(0x0C00_1EFF_0000_0001);
    let src = gen_shape(Shape::SkewedText, 3000, &mut rng);

    // sample the negative range coarsely, then every level from -5 upwards, plus
    // out-of-range levels which must be clamped or rejected identically.
    let mut levels: Vec<i32> = vec![mn, mn / 2, mn / 8, -1000, -100, -20];
    levels.extend(-5..=mx);
    levels.extend([mx + 1, mx + 100, i32::MAX, i32::MIN, mn - 1]);

    for lvl in levels {
        let tag = format!("level[{lvl}]");
        unsafe {
            let ctxc = (c.create)();
            let ctxr = (r.create)();
            let ic = (c.init)(ctxc, lvl);
            let ir = (r.init)(ctxr, lvl);
            namer.check(&format!("{tag}: compressInit"), ic, ir);
            if !looks_like_error(ic) {
                let frame =
                    drive_compress(&tag, &c, &r, ctxc, ctxr, &src, &mut rng, false, &namer);
                if let Some(frame) = frame {
                    let mut oc = vec![0u8; src.len() + 64];
                    let mut orr = vec![0u8; src.len() + 64];
                    let a = c_dec(oc.as_mut_ptr(), oc.len(), frame.as_ptr(), frame.len());
                    let b = r_dec(orr.as_mut_ptr(), orr.len(), frame.as_ptr(), frame.len());
                    namer.check(&format!("{tag}: ZSTD_decompress"), a, b);
                    if !looks_like_error(a) {
                        assert_bytes_eq(&format!("{tag}: decoded"), &src, &oc[..a]);
                    }
                }
            }
            namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));
        }
    }
}

// ======================================================== 5. dictionaries

/// `ZBUFF_compressInitDictionary` + `ZBUFF_decompressInitDictionary`, with
/// empty, tiny, large and raw-vs-structured dictionaries. Also checks that
/// decompressing with the *wrong* (or no) dictionary fails identically.
#[test]
fn zbuff_dictionaries_match() {
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let mut rng = Rng::new(0x0D1C_7000_0000_0001);

    let dicts: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), Vec::new()),
        ("one".into(), vec![0x5Au8]),
        ("tiny".into(), b"the quick brown fox".to_vec()),
        ("text8k".into(), gen_shape(Shape::SkewedText, 8192, &mut rng)),
        ("rand64k".into(), gen_shape(Shape::Random, 64 * 1024, &mut rng)),
        ("tabular1M".into(), gen_shape(Shape::Tabular, 1 << 20, &mut rng)),
        (
            "dictmagic".into(),
            {
                // starts with ZSTD_MAGIC_DICTIONARY but is not a real dictionary
                let mut v = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
                v.extend(gen_shape(Shape::Random, 500, &mut rng));
                v
            },
        ),
    ];

    let src = gen_shape(Shape::SkewedText, 5000, &mut rng);
    let other = gen_shape(Shape::Tabular, 3000, &mut rng);

    for (dname, d) in &dicts {
        for &lvl in &[1i32, 3, 12] {
            let tag = format!("dict[{dname} lvl={lvl}]");
            unsafe {
                let ctxc = (c.create)();
                let ctxr = (r.create)();
                let ic = (c.init_dict)(ctxc, d.as_ptr(), d.len(), lvl);
                let ir = (r.init_dict)(ctxr, d.as_ptr(), d.len(), lvl);
                namer.check(&format!("{tag}: compressInitDictionary"), ic, ir);
                if looks_like_error(ic) {
                    namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));
                    continue;
                }
                let frame = drive_compress(
                    &tag, &c, &r, ctxc, ctxr, &src, &mut rng, false, &namer,
                );
                namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));
                // e.g. a buffer that merely *starts* with ZSTD_MAGIC_DICTIONARY is
                // rejected lazily, during compression; both libraries must agree
                // (drive_compress already asserted that they did) and there is
                // then nothing left to decompress.
                let Some(frame) = frame else { continue };

                // correct dictionary -> must regenerate the input
                let a = (dc.create)();
                let b = (dr.create)();
                namer.check(
                    &format!("{tag}: decompressInitDictionary"),
                    (dc.init_dict)(a, d.as_ptr(), d.len()),
                    (dr.init_dict)(b, d.as_ptr(), d.len()),
                );
                let out = drive_decompress(
                    &format!("{tag} correct-dict"),
                    &dc,
                    &dr,
                    a,
                    b,
                    &frame,
                    &mut rng,
                    false,
                    &namer,
                );
                if let Some(out) = out {
                    assert_bytes_eq(&format!("{tag}: regenerated"), &src, &out);
                }
                namer.check(&format!("{tag}: freeDCtx"), (dc.free)(a), (dr.free)(b));

                // wrong dictionary and no dictionary: whatever happens, it must
                // happen identically in both libraries.
                for (wname, wd) in [("wrong", &other), ("none", &Vec::new())] {
                    let a = (dc.create)();
                    let b = (dr.create)();
                    namer.check(
                        &format!("{tag}: decompressInitDictionary({wname})"),
                        (dc.init_dict)(a, wd.as_ptr(), wd.len()),
                        (dr.init_dict)(b, wd.as_ptr(), wd.len()),
                    );
                    drive_decompress(
                        &format!("{tag} {wname}-dict"),
                        &dc,
                        &dr,
                        a,
                        b,
                        &frame,
                        &mut rng,
                        false,
                        &namer,
                    );
                    namer.check(&format!("{tag}: freeDCtx"), (dc.free)(a), (dr.free)(b));
                }
            }
        }
    }
}

// ============================================== 6. compressInit_advanced params

/// `ZBUFF_compressInit_advanced` takes a full `ZSTD_parameters` by value and
/// pushes each field through `ZSTD_CCtx_setParameter` after `ZSTD_checkCParams`,
/// so both the accepted and the rejected combinations are compared.
#[test]
fn zbuff_compress_init_advanced_matches() {
    let i = impls();
    let (c, r) = capi();
    let namer = Namer::new();
    let (c_gp, r_gp) = grab!(i, FnGetParams, "ZSTD_getParams");
    let (c_dec, r_dec) = grab!(i, FnDecompress, "ZSTD_decompress");
    let (c_fcs, r_fcs) = grab!(i, FnBufToU64, "ZSTD_getFrameContentSize");
    let mut rng = Rng::new(0x0AD0_0000_0000_0001);

    let src = gen_shape(Shape::SkewedText, 6000, &mut rng);

    // baseline: valid parameter sets straight from the library
    let mut param_sets: Vec<(String, ZSTDParameters)> = Vec::new();
    for &lvl in &[1i32, 3, 9, 19] {
        for &hint in &[0u64, 6000, 1 << 30] {
            let a = unsafe { c_gp(lvl, hint, 0) };
            let b = unsafe { r_gp(lvl, hint, 0) };
            assert_eq_dbg(&format!("ZSTD_getParams({lvl},{hint},0)"), a, b);
            param_sets.push((format!("getParams(lvl={lvl},hint={hint})"), a));
        }
    }

    // ... then mutate them, including into out-of-bounds territory
    let base = param_sets[1].1;
    for (name, mutate) in [
        ("windowLog=0", 0usize),
        ("windowLog=9", 1),
        ("windowLog=31", 2),
        ("windowLog=99", 3),
        ("chainLog=0", 4),
        ("chainLog=99", 5),
        ("hashLog=0", 6),
        ("hashLog=99", 7),
        ("searchLog=99", 8),
        ("minMatch=0", 9),
        ("minMatch=2", 10),
        ("minMatch=8", 11),
        ("minMatch=99", 12),
        ("targetLength=0", 13),
        ("targetLength=1<<20", 14),
        ("strategy=0", 15),
        ("strategy=9", 16),
        ("strategy=99", 17),
        ("strategy=-1", 18),
        ("flags=all-0", 19),
        ("flags=all-1", 20),
        ("flags=weird", 21),
    ] {
        let mut p = base;
        match mutate {
            0 => p.cParams.windowLog = 0,
            1 => p.cParams.windowLog = 9,
            2 => p.cParams.windowLog = 31,
            3 => p.cParams.windowLog = 99,
            4 => p.cParams.chainLog = 0,
            5 => p.cParams.chainLog = 99,
            6 => p.cParams.hashLog = 0,
            7 => p.cParams.hashLog = 99,
            8 => p.cParams.searchLog = 99,
            9 => p.cParams.minMatch = 0,
            10 => p.cParams.minMatch = 2,
            11 => p.cParams.minMatch = 8,
            12 => p.cParams.minMatch = 99,
            13 => p.cParams.targetLength = 0,
            14 => p.cParams.targetLength = 1 << 20,
            15 => p.cParams.strategy = 0,
            16 => p.cParams.strategy = 9,
            17 => p.cParams.strategy = 99,
            18 => p.cParams.strategy = -1,
            19 => {
                p.fParams.contentSizeFlag = 0;
                p.fParams.checksumFlag = 0;
                p.fParams.noDictIDFlag = 0;
            }
            20 => {
                p.fParams.contentSizeFlag = 1;
                p.fParams.checksumFlag = 1;
                p.fParams.noDictIDFlag = 1;
            }
            _ => {
                p.fParams.contentSizeFlag = 7;
                p.fParams.checksumFlag = -3;
                p.fParams.noDictIDFlag = 42;
            }
        }
        param_sets.push((name.to_string(), p));
    }

    let dict = gen_shape(Shape::SkewedText, 4096, &mut rng);

    for (pname, p) in &param_sets {
        for &pledged in &[0u64, 1, 6000, 5999, 6001, u64::MAX - 1] {
            for withdict in [false, true] {
                let tag = format!("initAdv[{pname} pledged={pledged} dict={withdict}]");
                let d: &[u8] = if withdict { &dict } else { &[] };
                unsafe {
                    let ctxc = (c.create)();
                    let ctxr = (r.create)();
                    let ic = (c.init_adv)(ctxc, d.as_ptr(), d.len(), *p, pledged);
                    let ir = (r.init_adv)(ctxr, d.as_ptr(), d.len(), *p, pledged);
                    namer.check(&format!("{tag}: compressInit_advanced"), ic, ir);
                    if !looks_like_error(ic) {
                        let frame = drive_compress(
                            &tag, &c, &r, ctxc, ctxr, &src, &mut rng, false, &namer,
                        );
                        if let Some(frame) = frame {
                            let a = c_fcs(frame.as_ptr(), frame.len());
                            let b = r_fcs(frame.as_ptr(), frame.len());
                            assert_eq_dbg(&format!("{tag}: frameContentSize"), a, b);
                            let mut oc = vec![0u8; src.len() + 64];
                            let mut orr = vec![0u8; src.len() + 64];
                            let x =
                                c_dec(oc.as_mut_ptr(), oc.len(), frame.as_ptr(), frame.len());
                            let y =
                                r_dec(orr.as_mut_ptr(), orr.len(), frame.as_ptr(), frame.len());
                            namer.check(&format!("{tag}: ZSTD_decompress"), x, y);
                            if !looks_like_error(x) && !withdict {
                                assert_bytes_eq(&format!("{tag}: decoded"), &src, &oc[..x]);
                            }
                        }
                    }
                    namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));
                }
            }
        }
    }
}

// ================================================ 7. out-of-order / misuse

/// Calling the API out of order must fail (or succeed) *identically*.
#[test]
fn zbuff_out_of_order_calls_match() {
    let i = impls();
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let (c_comp, _r_comp) = grab!(i, FnCompress, "ZSTD_compress");

    let mut rng = Rng::new(0x0000_00DE_0000_0001);
    let src = gen_shape(Shape::SkewedText, 900, &mut rng);
    let mut frame = vec![0u8; 4096];
    let fl = unsafe {
        c_comp(
            frame.as_mut_ptr(),
            frame.len(),
            src.as_ptr(),
            src.len(),
            3,
        )
    };
    assert!(!looks_like_error(fl));
    frame.truncate(fl);

    let mut bufc = vec![0u8; 1 << 17];
    let mut bufr = vec![0u8; 1 << 17];

    unsafe {
        // ---- compressContinue / Flush / End on a freshly created, un-inited ctx
        for scenario in 0..3 {
            let ctxc = (c.create)();
            let ctxr = (r.create)();
            let mut szc = bufc.len();
            let mut szr = bufr.len();
            let mut inc = src.len();
            let mut inr = src.len();
            let (a, b) = match scenario {
                0 => (
                    (c.cont)(ctxc, bufc.as_mut_ptr(), &mut szc, src.as_ptr(), &mut inc),
                    (r.cont)(ctxr, bufr.as_mut_ptr(), &mut szr, src.as_ptr(), &mut inr),
                ),
                1 => (
                    (c.flush)(ctxc, bufc.as_mut_ptr(), &mut szc),
                    (r.flush)(ctxr, bufr.as_mut_ptr(), &mut szr),
                ),
                _ => (
                    (c.end)(ctxc, bufc.as_mut_ptr(), &mut szc),
                    (r.end)(ctxr, bufr.as_mut_ptr(), &mut szr),
                ),
            };
            namer.check(&format!("uninit compress scenario {scenario}"), a, b);
            assert_eq_dbg(
                &format!("uninit compress scenario {scenario} produced"),
                szc,
                szr,
            );
            assert_eq_dbg(
                &format!("uninit compress scenario {scenario} consumed"),
                inc,
                inr,
            );
            assert_bytes_eq(
                &format!("uninit compress scenario {scenario} bytes"),
                &bufc[..szc],
                &bufr[..szr],
            );
            namer.check("freeCCtx", (c.free)(ctxc), (r.free)(ctxr));
        }

        // ---- decompressContinue on an un-inited DCtx
        {
            let a = (dc.create)();
            let b = (dr.create)();
            let mut szc = bufc.len();
            let mut szr = bufr.len();
            let mut inc = frame.len();
            let mut inr = frame.len();
            let x = (dc.cont)(a, bufc.as_mut_ptr(), &mut szc, frame.as_ptr(), &mut inc);
            let y = (dr.cont)(b, bufr.as_mut_ptr(), &mut szr, frame.as_ptr(), &mut inr);
            namer.check("uninit decompressContinue", x, y);
            assert_eq_dbg("uninit decompressContinue produced", szc, szr);
            assert_eq_dbg("uninit decompressContinue consumed", inc, inr);
            assert_bytes_eq("uninit decompressContinue bytes", &bufc[..szc], &bufr[..szr]);
            namer.check("freeDCtx", (dc.free)(a), (dr.free)(b));
        }

        // ---- flush/end before feeding anything, then continue *after* end
        {
            let ctxc = (c.create)();
            let ctxr = (r.create)();
            namer.check("compressInit", (c.init)(ctxc, 3), (r.init)(ctxr, 3));

            let mut szc = bufc.len();
            let mut szr = bufr.len();
            namer.check(
                "flush-before-any-input",
                (c.flush)(ctxc, bufc.as_mut_ptr(), &mut szc),
                (r.flush)(ctxr, bufr.as_mut_ptr(), &mut szr),
            );
            assert_eq_dbg("flush-before-any-input produced", szc, szr);
            assert_bytes_eq("flush-before-any-input", &bufc[..szc], &bufr[..szr]);

            let mut szc = bufc.len();
            let mut szr = bufr.len();
            namer.check(
                "end-with-no-input",
                (c.end)(ctxc, bufc.as_mut_ptr(), &mut szc),
                (r.end)(ctxr, bufr.as_mut_ptr(), &mut szr),
            );
            assert_eq_dbg("end-with-no-input produced", szc, szr);
            assert_bytes_eq("end-with-no-input", &bufc[..szc], &bufr[..szr]);

            // end again (already ended)
            let mut szc = bufc.len();
            let mut szr = bufr.len();
            namer.check(
                "double-end",
                (c.end)(ctxc, bufc.as_mut_ptr(), &mut szc),
                (r.end)(ctxr, bufr.as_mut_ptr(), &mut szr),
            );
            assert_eq_dbg("double-end produced", szc, szr);
            assert_bytes_eq("double-end", &bufc[..szc], &bufr[..szr]);

            // continue after end
            let mut szc = bufc.len();
            let mut szr = bufr.len();
            let mut inc = src.len();
            let mut inr = src.len();
            namer.check(
                "continue-after-end",
                (c.cont)(ctxc, bufc.as_mut_ptr(), &mut szc, src.as_ptr(), &mut inc),
                (r.cont)(ctxr, bufr.as_mut_ptr(), &mut szr, src.as_ptr(), &mut inr),
            );
            assert_eq_dbg("continue-after-end produced", szc, szr);
            assert_eq_dbg("continue-after-end consumed", inc, inr);
            assert_bytes_eq("continue-after-end", &bufc[..szc], &bufr[..szr]);

            namer.check("freeCCtx", (c.free)(ctxc), (r.free)(ctxr));
        }

        // ---- re-init in the middle of a stream (ZBUFF_CCtx objects are reusable)
        {
            let ctxc = (c.create)();
            let ctxr = (r.create)();
            namer.check("compressInit", (c.init)(ctxc, 3), (r.init)(ctxr, 3));
            let mut szc = bufc.len();
            let mut szr = bufr.len();
            let mut inc = src.len();
            let mut inr = src.len();
            namer.check(
                "mid-stream continue",
                (c.cont)(ctxc, bufc.as_mut_ptr(), &mut szc, src.as_ptr(), &mut inc),
                (r.cont)(ctxr, bufr.as_mut_ptr(), &mut szr, src.as_ptr(), &mut inr),
            );
            assert_eq_dbg("mid-stream produced", szc, szr);
            assert_bytes_eq("mid-stream", &bufc[..szc], &bufr[..szr]);
            namer.check("re-init", (c.init)(ctxc, 7), (r.init)(ctxr, 7));
            let frame2 = drive_compress(
                "reinit", &c, &r, ctxc, ctxr, &src, &mut rng, false, &namer,
            );
            assert!(frame2.is_some());
            namer.check("freeCCtx", (c.free)(ctxc), (r.free)(ctxr));
        }

        // ---- decompressContinue past the end of a completed frame
        {
            let a = (dc.create)();
            let b = (dr.create)();
            namer.check("decompressInit", (dc.init)(a), (dr.init)(b));
            let out = drive_decompress(
                "past-end", &dc, &dr, a, b, &frame, &mut rng, false, &namer,
            );
            assert!(out.is_some());
            for k in 0..3 {
                let mut szc = bufc.len();
                let mut szr = bufr.len();
                let mut inc = frame.len();
                let mut inr = frame.len();
                namer.check(
                    &format!("decompressContinue past end #{k}"),
                    (dc.cont)(a, bufc.as_mut_ptr(), &mut szc, frame.as_ptr(), &mut inc),
                    (dr.cont)(b, bufr.as_mut_ptr(), &mut szr, frame.as_ptr(), &mut inr),
                );
                assert_eq_dbg(&format!("past end #{k} produced"), szc, szr);
                assert_eq_dbg(&format!("past end #{k} consumed"), inc, inr);
                assert_bytes_eq(&format!("past end #{k}"), &bufc[..szc], &bufr[..szr]);
            }
            namer.check("freeDCtx", (dc.free)(a), (dr.free)(b));
        }
    }
}

// ============================================== 8. NULL and zero-size buffers

#[test]
fn zbuff_null_and_zero_size_buffers_match() {
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let mut rng = Rng::new(0x0000_0000_0000_0011);
    let src = gen_shape(Shape::Counter, 500, &mut rng);

    unsafe {
        // NULL dst with capacity 0, NULL src with size 0, and every combination
        for (tag, dstnull, srcnull) in [
            ("dst=NULL", true, false),
            ("src=NULL", false, true),
            ("both=NULL", true, true),
            ("neither", false, false),
        ] {
            let ctxc = (c.create)();
            let ctxr = (r.create)();
            namer.check("compressInit", (c.init)(ctxc, 3), (r.init)(ctxr, 3));

            let mut dbufc = vec![0u8; 64];
            let mut dbufr = vec![0u8; 64];
            let dpc: *mut u8 = if dstnull {
                std::ptr::null_mut()
            } else {
                dbufc.as_mut_ptr()
            };
            let dpr: *mut u8 = if dstnull {
                std::ptr::null_mut()
            } else {
                dbufr.as_mut_ptr()
            };
            let sp: *const u8 = if srcnull {
                std::ptr::null()
            } else {
                src.as_ptr()
            };
            let ssz = if srcnull { 0 } else { src.len() };

            let mut szc = 0usize;
            let mut szr = 0usize;
            let mut inc = ssz;
            let mut inr = ssz;
            namer.check(
                &format!("compressContinue[{tag}, dstCap=0]"),
                (c.cont)(ctxc, dpc, &mut szc, sp, &mut inc),
                (r.cont)(ctxr, dpr, &mut szr, sp, &mut inr),
            );
            assert_eq_dbg(&format!("{tag} produced"), szc, szr);
            assert_eq_dbg(&format!("{tag} consumed"), inc, inr);

            let mut szc = 0usize;
            let mut szr = 0usize;
            namer.check(
                &format!("compressFlush[{tag}, dstCap=0]"),
                (c.flush)(ctxc, dpc, &mut szc),
                (r.flush)(ctxr, dpr, &mut szr),
            );
            assert_eq_dbg(&format!("{tag} flush produced"), szc, szr);

            let mut szc = 0usize;
            let mut szr = 0usize;
            namer.check(
                &format!("compressEnd[{tag}, dstCap=0]"),
                (c.end)(ctxc, dpc, &mut szc),
                (r.end)(ctxr, dpr, &mut szr),
            );
            assert_eq_dbg(&format!("{tag} end produced"), szc, szr);

            namer.check("freeCCtx", (c.free)(ctxc), (r.free)(ctxr));

            // same for the decompressor
            let a = (dc.create)();
            let b = (dr.create)();
            namer.check("decompressInit", (dc.init)(a), (dr.init)(b));
            let mut szc = 0usize;
            let mut szr = 0usize;
            let mut inc = ssz;
            let mut inr = ssz;
            namer.check(
                &format!("decompressContinue[{tag}, dstCap=0]"),
                (dc.cont)(a, dpc, &mut szc, sp, &mut inc),
                (dr.cont)(b, dpr, &mut szr, sp, &mut inr),
            );
            assert_eq_dbg(&format!("{tag} d-produced"), szc, szr);
            assert_eq_dbg(&format!("{tag} d-consumed"), inc, inr);
            namer.check("freeDCtx", (dc.free)(a), (dr.free)(b));
        }

        // NULL dictionary pointers with size 0
        let ctxc = (c.create)();
        let ctxr = (r.create)();
        namer.check(
            "compressInitDictionary(NULL,0)",
            (c.init_dict)(ctxc, std::ptr::null(), 0, 5),
            (r.init_dict)(ctxr, std::ptr::null(), 0, 5),
        );
        namer.check("freeCCtx", (c.free)(ctxc), (r.free)(ctxr));

        let a = (dc.create)();
        let b = (dr.create)();
        namer.check(
            "decompressInitDictionary(NULL,0)",
            (dc.init_dict)(a, std::ptr::null(), 0),
            (dr.init_dict)(b, std::ptr::null(), 0),
        );
        namer.check("freeDCtx", (dc.free)(a), (dr.free)(b));
    }
}

// ======================================== 9. decompressing hostile / odd input

/// The decompression machine on inputs it must reject: random bytes, truncated
/// and bit-flipped frames, concatenated frames, skippable frames.
#[test]
fn zbuff_decompress_hostile_input_matches() {
    let i = impls();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let (c_comp, _r) = grab!(i, FnCompress, "ZSTD_compress");
    let mut rng = Rng::new(0x0057_1EFF_0000_0002);

    // a valid reference frame
    let src = gen_shape(Shape::SkewedText, 3000, &mut rng);
    let mut good = vec![0u8; 8192];
    let n = unsafe {
        c_comp(good.as_mut_ptr(), good.len(), src.as_ptr(), src.len(), 5)
    };
    assert!(!looks_like_error(n));
    good.truncate(n);

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), Vec::new()));
    for k in 0..40 {
        let len = [1usize, 2, 3, 4, 5, 9, 40, 200, 3000][k % 9];
        cases.push((
            format!("random[{len}]#{k}"),
            gen_shape(Shape::Random, len, &mut rng),
        ));
    }
    for t in 0..12 {
        let cut = (good.len() * t) / 12;
        cases.push((format!("trunc{cut}"), good[..cut].to_vec()));
    }
    for k in 0..24 {
        let mut b = good.clone();
        let at = rng.below(b.len());
        b[at] ^= 1u8 << rng.below(8);
        cases.push((format!("flip{at}#{k}"), b));
    }
    {
        let mut two = good.clone();
        two.extend_from_slice(&good);
        cases.push(("two-frames".into(), two));
        let mut skip = ZSTD_MAGIC_SKIPPABLE_START.to_le_bytes().to_vec();
        skip.extend_from_slice(&8u32.to_le_bytes());
        skip.extend_from_slice(b"12345678");
        skip.extend_from_slice(&good);
        cases.push(("skippable+frame".into(), skip));
        let mut trailing = good.clone();
        trailing.extend_from_slice(b"trailing garbage");
        cases.push(("frame+garbage".into(), trailing));
    }
    cases.push(("good".into(), good.clone()));

    for (label, buf) in &cases {
        for tiny in [false, true] {
            unsafe {
                let a = (dc.create)();
                let b = (dr.create)();
                namer.check("decompressInit", (dc.init)(a), (dr.init)(b));
                drive_decompress(
                    &format!("hostile[{label} tiny={tiny}]"),
                    &dc,
                    &dr,
                    a,
                    b,
                    buf,
                    &mut rng,
                    tiny,
                    &namer,
                );
                namer.check("freeDCtx", (dc.free)(a), (dr.free)(b));
            }
        }
    }
}

// =========================================== 10. large input / multi-block runs

/// Inputs comfortably larger than one 128 KB block, with the recommended buffer
/// sizes as well as deliberately awkward ones, so the block-splitting and
/// internal-buffer bookkeeping is compared over many iterations.
#[test]
fn zbuff_multiblock_and_large_input_matches() {
    let i = impls();
    let (c, r) = capi();
    let (dc, dr) = dapi();
    let namer = Namer::new();
    let (c_bound, r_bound) = grab!(i, FnCompressBound, "ZSTD_compressBound");
    let mut rng = Rng::new(0x0B16_0000_1234_5678);

    for len in [
        ZSTD_BLOCKSIZE_MAX - 1,
        ZSTD_BLOCKSIZE_MAX,
        ZSTD_BLOCKSIZE_MAX + 1,
        2 * ZSTD_BLOCKSIZE_MAX,
        3 * ZSTD_BLOCKSIZE_MAX + 7,
    ] {
        unsafe {
            assert_eq_dbg(
                &format!("ZSTD_compressBound({len})"),
                c_bound(len),
                r_bound(len),
            );
        }
        for shape in [Shape::Random, Shape::SkewedText, Shape::Constant] {
            let src = gen_shape(shape, len, &mut rng);
            let tag = format!("large[{shape:?} len={len}]");
            // one multi-block case is also driven with byte-at-a-time output
            // buffers, which is where partial-flush bookkeeping tends to break.
            let tiny = shape == Shape::Constant && len == ZSTD_BLOCKSIZE_MAX + 1;
            unsafe {
                let ctxc = (c.create)();
                let ctxr = (r.create)();
                namer.check(
                    &format!("{tag}: compressInit"),
                    (c.init)(ctxc, 3),
                    (r.init)(ctxr, 3),
                );
                let frame =
                    drive_compress(&tag, &c, &r, ctxc, ctxr, &src, &mut rng, tiny, &namer)
                        .expect("large compression must succeed");
                namer.check(&format!("{tag}: freeCCtx"), (c.free)(ctxc), (r.free)(ctxr));

                let a = (dc.create)();
                let b = (dr.create)();
                namer.check(
                    &format!("{tag}: decompressInit"),
                    (dc.init)(a),
                    (dr.init)(b),
                );
                let out = drive_decompress(
                    &format!("{tag} -> dstream"),
                    &dc,
                    &dr,
                    a,
                    b,
                    &frame,
                    &mut rng,
                    tiny,
                    &namer,
                )
                .expect("large decompression must succeed");
                namer.check(&format!("{tag}: freeDCtx"), (dc.free)(a), (dr.free)(b));
                assert_bytes_eq(&format!("{tag}: regenerated"), &src, &out);
            }
        }
    }
}
