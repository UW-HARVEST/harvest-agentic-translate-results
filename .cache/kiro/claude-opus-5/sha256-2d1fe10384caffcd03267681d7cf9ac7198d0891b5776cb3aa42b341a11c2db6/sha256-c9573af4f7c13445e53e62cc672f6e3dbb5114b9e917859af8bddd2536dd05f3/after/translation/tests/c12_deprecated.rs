//! Phase C: differential tests for the DEPRECATED ZBUFF streaming API —
//! ERROR paths (complements the HAPPY paths in `b14_deprecated.rs`).
//!
//! Every ZBUFF entry point can fail: called out of order (before init),
//! with a zero/tiny destination buffer, on corrupted or truncated input, with
//! an out-of-range compression level, or with a bogus dictionary. The C
//! implementation is ground truth; the Rust translation must return the
//! byte-for-byte identical result for every such input.
//!
//! For every call we compare BOTH the `ZBUFF_isError` boolean AND the
//! `ZBUFF_getErrorName` string (never merely "both failed"). When neither is an
//! error we additionally require the raw `size_t` return (a hint / bytes-left
//! value) to match exactly.
//!
//! Every call crosses the FFI boundary via `both::<T>(name)`; a handle from one
//! library is NEVER passed to the other.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

use std::os::raw::c_ulonglong;

// ---------------------------------------------------------------- FFI typedefs

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnCompressInit = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompressInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t, c_int) -> size_t;
type FnDecompressInit = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecompressInitDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;
type FnCompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnCompressFlush = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t) -> size_t;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, *mut size_t, *const c_void, *mut size_t) -> size_t;
type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;

// -------------------------------------------------------------- ZBUFF err API

/// Compares C and Rust `size_t` returns through ZBUFF's OWN error classifier,
/// asserting both the boolean and (when an error) the string are identical, and
/// (when OK) the raw value is identical.
struct ZbuffErr {
    is_err: (libloading::Symbol<'static, FnIsError>, libloading::Symbol<'static, FnIsError>),
    name: (libloading::Symbol<'static, FnGetErrorName>, libloading::Symbol<'static, FnGetErrorName>),
}
impl ZbuffErr {
    unsafe fn new() -> Self {
        ZbuffErr {
            is_err: both::<FnIsError>("ZBUFF_isError"),
            name: both::<FnGetErrorName>("ZBUFF_getErrorName"),
        }
    }
    #[track_caller]
    unsafe fn eq(&self, ctx: &str, cr: size_t, rr: size_t) {
        let c_is = (self.is_err.0)(cr) != 0;
        let r_is = (self.is_err.1)(rr) != 0;
        assert_eq!(c_is, r_is, "{ctx}: isError mismatch C={c_is} RS={r_is} (raw C={cr:#x} RS={rr:#x})");
        let cn = cstr((self.name.0)(cr));
        let rn = cstr((self.name.1)(rr));
        assert_eq!(cn, rn, "{ctx}: getErrorName mismatch C={cn:?} RS={rn:?} (raw C={cr:#x} RS={rr:#x})");
        if !c_is {
            assert_eq!(cr, rr, "{ctx}: OK return value differs C={cr:#x} RS={rr:#x}");
        }
    }
    unsafe fn is_c_err(&self, r: size_t) -> bool {
        (self.is_err.0)(r) != 0
    }
}

// ----------------------------------------------------------------- ctx guards

struct CCtx {
    c: *mut c_void,
    r: *mut c_void,
    free: (libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>),
}
impl CCtx {
    unsafe fn new() -> Self {
        let (cc, rc) = both::<FnCreate>("ZBUFF_createCCtx");
        let c = cc();
        let r = rc();
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createCCtx null");
        CCtx { c, r, free: both::<FnFree>("ZBUFF_freeCCtx") }
    }
}
impl Drop for CCtx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}

struct DCtx {
    c: *mut c_void,
    r: *mut c_void,
    free: (libloading::Symbol<'static, FnFree>, libloading::Symbol<'static, FnFree>),
}
impl DCtx {
    unsafe fn new() -> Self {
        let (cc, rc) = both::<FnCreate>("ZBUFF_createDCtx");
        let c = cc();
        let r = rc();
        assert!(!c.is_null() && !r.is_null(), "ZBUFF_createDCtx null");
        DCtx { c, r, free: both::<FnFree>("ZBUFF_freeDCtx") }
    }
}
impl Drop for DCtx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}

// One valid ZBUFF-produced frame + the source it decodes to, used as the seed
// for corruption / truncation sweeps.
unsafe fn make_valid_frame(src: &[u8], level: c_int) -> Vec<u8> {
    let (cc, _) = both::<FnCompress>("ZSTD_compress");
    let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
    let cap = cb(src.len()) + 64;
    let mut buf = vec![0u8; cap];
    let n = cc(buf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void, src.len(), level);
    let e = Err2::new();
    assert!(!e.c.is_err(n), "make_valid_frame: compress failed");
    buf.truncate(n);
    buf
}

// -------------------------------------------------------------------- tests

/// Compression finalizers (continue/flush/end) called on a *freshly created*
/// context (never init'd), plus continue after only create. The modern stream
/// state machine rejects these; C and Rust must agree exactly.
#[test]
fn compress_calls_without_init() {
    unsafe {
        let ze = ZbuffErr::new();
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");

        let mut rng = Rng::new(0xC12_0001);
        for _ in 0..64 {
            // fresh, un-init'd contexts each round
            let cc = CCtx::new();
            let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], 1 + rng.below(4096), &mut rng);

            // compressContinue without init
            {
                let mut c_ss = src.len();
                let mut r_ss = src.len();
                let mut c_dc = 4096usize;
                let mut r_dc = 4096usize;
                let mut cout = vec![0u8; 4096];
                let mut rout = vec![0u8; 4096];
                let a = (cont.0)(cc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc,
                    src.as_ptr() as *const c_void, &mut c_ss);
                let b = (cont.1)(cc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc,
                    src.as_ptr() as *const c_void, &mut r_ss);
                ze.eq("compressContinue-no-init", a, b);
                assert_eq!(c_ss, r_ss, "no-init continue srcConsumed");
                assert_eq!(c_dc, r_dc, "no-init continue dstWritten");
            }
            // compressFlush without init
            {
                let mut c_dc = 4096usize;
                let mut r_dc = 4096usize;
                let mut cout = vec![0u8; 4096];
                let mut rout = vec![0u8; 4096];
                let a = (flush.0)(cc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc);
                let b = (flush.1)(cc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc);
                ze.eq("compressFlush-no-init", a, b);
                assert_eq!(c_dc, r_dc, "no-init flush dstWritten");
            }
            // compressEnd without init
            {
                let mut c_dc = 4096usize;
                let mut r_dc = 4096usize;
                let mut cout = vec![0u8; 4096];
                let mut rout = vec![0u8; 4096];
                let a = (end.0)(cc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc);
                let b = (end.1)(cc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc);
                ze.eq("compressEnd-no-init", a, b);
                assert_eq!(c_dc, r_dc, "no-init end dstWritten");
            }
        }
    }
}

/// `ZBUFF_decompressContinue` on a never-init'd DCtx.
#[test]
fn decompress_continue_without_init() {
    unsafe {
        let ze = ZbuffErr::new();
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let mut rng = Rng::new(0xC12_0002);
        for _ in 0..64 {
            let dc = DCtx::new();
            let frame = gen(Shape::Random, 1 + rng.below(256), &mut rng);
            let mut c_ss = frame.len();
            let mut r_ss = frame.len();
            let mut c_dc = 4096usize;
            let mut r_dc = 4096usize;
            let mut cout = vec![0u8; 4096];
            let mut rout = vec![0u8; 4096];
            let a = (dcont.0)(dc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc,
                frame.as_ptr() as *const c_void, &mut c_ss);
            let b = (dcont.1)(dc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc,
                frame.as_ptr() as *const c_void, &mut r_ss);
            // NOTE: ZBUFF_decompressContinue lazily initializes, so this may
            // succeed or fail; either way C and Rust must match identically.
            ze.eq("decompressContinue-no-init", a, b);
            assert_eq!(c_ss, r_ss, "no-init dcontinue srcConsumed");
            assert_eq!(c_dc, r_dc, "no-init dcontinue dstWritten");
        }
    }
}

/// Tiny destination buffers (capacity 0 and 1) on every streaming finalizer,
/// after a correct init. The finalizers must make identical partial progress
/// (or none) and report identical bytes-left / hint values.
#[test]
fn tiny_dst_capacity_streaming() {
    unsafe {
        let ze = ZbuffErr::new();
        let cinit = both::<FnCompressInit>("ZBUFF_compressInit");
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let flush = both::<FnCompressFlush>("ZBUFF_compressFlush");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");

        let mut rng = Rng::new(0xC12_0003);
        for &shape in &[Shape::Text, Shape::Random, Shape::Repeating, Shape::Zeros] {
            for &len in &[1usize, 64, 1024, 20_000] {
                let src = gen(shape, len, &mut rng);
                for &cap in &[0usize, 1] {
                    // compression side
                    let cc = CCtx::new();
                    ze.eq("cinit", (cinit.0)(cc.c, 3), (cinit.1)(cc.r, 3));
                    // continue with a 0/1-byte output window
                    let mut ipos = 0usize;
                    let mut steps = 0usize;
                    let mut cbuf = vec![0u8; 1];
                    let mut rbuf = vec![0u8; 1];
                    while ipos < src.len() && steps < 8 {
                        let mut c_ss = src.len() - ipos;
                        let mut r_ss = src.len() - ipos;
                        let mut c_dc = cap;
                        let mut r_dc = cap;
                        let a = (cont.0)(cc.c, cbuf.as_mut_ptr() as *mut c_void, &mut c_dc,
                            src[ipos..].as_ptr() as *const c_void, &mut c_ss);
                        let b = (cont.1)(cc.r, rbuf.as_mut_ptr() as *mut c_void, &mut r_dc,
                            src[ipos..].as_ptr() as *const c_void, &mut r_ss);
                        let ctx = format!("continue cap={cap} shape={shape:?} len={len} step={steps}");
                        ze.eq(&ctx, a, b);
                        assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
                        assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
                        if c_ss == 0 && c_dc == 0 { break; }
                        ipos += c_ss;
                        steps += 1;
                    }
                    // flush + end with tiny buffers
                    let mut c_dc = cap;
                    let mut r_dc = cap;
                    ze.eq(&format!("flush cap={cap}"),
                        (flush.0)(cc.c, cbuf.as_mut_ptr() as *mut c_void, &mut c_dc),
                        (flush.1)(cc.r, rbuf.as_mut_ptr() as *mut c_void, &mut r_dc));
                    assert_eq!(c_dc, r_dc, "flush dstWritten cap={cap}");
                    let mut c_dc = cap;
                    let mut r_dc = cap;
                    ze.eq(&format!("end cap={cap}"),
                        (end.0)(cc.c, cbuf.as_mut_ptr() as *mut c_void, &mut c_dc),
                        (end.1)(cc.r, rbuf.as_mut_ptr() as *mut c_void, &mut r_dc));
                    assert_eq!(c_dc, r_dc, "end dstWritten cap={cap}");

                    // decompression side: feed a valid frame with 0/1-byte out window
                    let frame = make_valid_frame(&src, 3);
                    let dc = DCtx::new();
                    ze.eq("dinit", (dinit.0)(dc.c), (dinit.1)(dc.r));
                    let mut ipos = 0usize;
                    let mut steps = 0usize;
                    while ipos <= frame.len() && steps < 16 {
                        let avail = frame.len() - ipos;
                        let mut c_ss = avail;
                        let mut r_ss = avail;
                        let mut c_dc = cap;
                        let mut r_dc = cap;
                        let sp = if avail == 0 { std::ptr::null() } else { frame[ipos..].as_ptr() as *const c_void };
                        let a = (dcont.0)(dc.c, cbuf.as_mut_ptr() as *mut c_void, &mut c_dc, sp, &mut c_ss);
                        let b = (dcont.1)(dc.r, rbuf.as_mut_ptr() as *mut c_void, &mut r_dc, sp, &mut r_ss);
                        let ctx = format!("dcontinue cap={cap} shape={shape:?} len={len} step={steps}");
                        ze.eq(&ctx, a, b);
                        assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
                        assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
                        if ze.is_c_err(a) || a == 0 { break; }
                        if c_ss == 0 && c_dc == 0 && avail == 0 { break; }
                        ipos += c_ss;
                        steps += 1;
                    }
                }
            }
        }
    }
}

/// Exhaustive single-byte mutation sweep of a valid ZBUFF-produced frame fed to
/// `ZBUFF_decompressContinue`. For every mutated frame, C and Rust must produce
/// the identical (error or partial-output) trajectory.
#[test]
fn decompress_single_byte_mutation_sweep() {
    unsafe {
        let ze = ZbuffErr::new();
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");

        let mut rng = Rng::new(0xC12_0004);
        // A few compact frames keep the (byte x value) sweep tractable.
        let seeds: Vec<Vec<u8>> = [
            gen(Shape::Text, 200, &mut rng),
            gen(Shape::Random, 120, &mut rng),
            gen(Shape::Repeating, 300, &mut rng),
            gen(Shape::Zeros, 64, &mut rng),
        ]
        .iter()
        .map(|s| make_valid_frame(s, 5))
        .collect();

        for (fi, frame) in seeds.iter().enumerate() {
            // Sweep every byte position; try a spread of mutation values to keep
            // it bounded but thorough.
            let mut_vals: [u8; 6] = [0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff];
            for pos in 0..frame.len() {
                for &mv in &mut_vals {
                    let mut m = frame.clone();
                    m[pos] ^= mv;
                    if m == *frame && mv != 0 { continue; }
                    // full-frame decode in one shot with generous out buffer
                    let dc = DCtx::new();
                    ze.eq("dinit", (dinit.0)(dc.c), (dinit.1)(dc.r));
                    let mut c_ss = m.len();
                    let mut r_ss = m.len();
                    let mut c_dc = 1 << 18;
                    let mut r_dc = 1 << 18;
                    let mut cout = vec![0u8; 1 << 18];
                    let mut rout = vec![0u8; 1 << 18];
                    let a = (dcont.0)(dc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc,
                        m.as_ptr() as *const c_void, &mut c_ss);
                    let b = (dcont.1)(dc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc,
                        m.as_ptr() as *const c_void, &mut r_ss);
                    let ctx = format!("mutate frame#{fi} pos={pos} xor={mv:#x}");
                    ze.eq(&ctx, a, b);
                    assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
                    assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
                    if !ze.is_c_err(a) {
                        assert_bytes_eq(&format!("{ctx} out"), &cout[..c_dc], &rout[..r_dc]);
                    }
                }
            }
        }
    }
}

/// Every truncation length (0..=N) of a valid ZBUFF frame fed to
/// `ZBUFF_decompressContinue`.
#[test]
fn decompress_truncation_sweep() {
    unsafe {
        let ze = ZbuffErr::new();
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let mut rng = Rng::new(0xC12_0005);

        let seeds: Vec<Vec<u8>> = [
            gen(Shape::Text, 500, &mut rng),
            gen(Shape::Random, 300, &mut rng),
            gen(Shape::LongMatches, 1500, &mut rng),
        ]
        .iter()
        .map(|s| make_valid_frame(s, 7))
        .collect();

        for (fi, frame) in seeds.iter().enumerate() {
            for cut in 0..=frame.len() {
                let dc = DCtx::new();
                ze.eq("dinit", (dinit.0)(dc.c), (dinit.1)(dc.r));
                let mut c_ss = cut;
                let mut r_ss = cut;
                let mut c_dc = 1 << 18;
                let mut r_dc = 1 << 18;
                let mut cout = vec![0u8; 1 << 18];
                let mut rout = vec![0u8; 1 << 18];
                let sp = if cut == 0 { std::ptr::null() } else { frame.as_ptr() as *const c_void };
                let a = (dcont.0)(dc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc, sp, &mut c_ss);
                let b = (dcont.1)(dc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc, sp, &mut r_ss);
                let ctx = format!("truncate frame#{fi} cut={cut}");
                ze.eq(&ctx, a, b);
                assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
                assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
                if !ze.is_c_err(a) {
                    assert_bytes_eq(&format!("{ctx} out"), &cout[..c_dc], &rout[..r_dc]);
                }
            }
        }
    }
}

/// Thousands of random-garbage buffers fed to `ZBUFF_decompressContinue`.
#[test]
fn decompress_random_garbage() {
    unsafe {
        let ze = ZbuffErr::new();
        let dinit = both::<FnDecompressInit>("ZBUFF_decompressInit");
        let dcont = both::<FnDecompressContinue>("ZBUFF_decompressContinue");
        let mut rng = Rng::new(0xC12_0006);
        for i in 0..4000 {
            let n = rng.below(300);
            let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            // bias some towards a real magic prefix to exercise header parsing
            if i % 4 == 0 && n >= 4 {
                buf[..4].copy_from_slice(&0xFD2FB528u32.to_le_bytes());
            }
            let dc = DCtx::new();
            ze.eq("dinit", (dinit.0)(dc.c), (dinit.1)(dc.r));
            let mut c_ss = buf.len();
            let mut r_ss = buf.len();
            let mut c_dc = 1 << 16;
            let mut r_dc = 1 << 16;
            let mut cout = vec![0u8; 1 << 16];
            let mut rout = vec![0u8; 1 << 16];
            let sp = if buf.is_empty() { std::ptr::null() } else { buf.as_ptr() as *const c_void };
            let a = (dcont.0)(dc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc, sp, &mut c_ss);
            let b = (dcont.1)(dc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc, sp, &mut r_ss);
            let ctx = format!("garbage #{i} n={n}");
            ze.eq(&ctx, a, b);
            assert_eq!(c_ss, r_ss, "{ctx} srcConsumed");
            assert_eq!(c_dc, r_dc, "{ctx} dstWritten");
            if !ze.is_c_err(a) {
                assert_bytes_eq(&format!("{ctx} out"), &cout[..c_dc], &rout[..r_dc]);
            }
        }
    }
}

/// `ZBUFF_compressInit` across the full int range of "compression levels",
/// including out-of-range values that the modern API clamps or rejects.
#[test]
fn compress_init_bad_levels() {
    unsafe {
        let ze = ZbuffErr::new();
        let cinit = both::<FnCompressInit>("ZBUFF_compressInit");
        let cont = both::<FnCompressContinue>("ZBUFF_compressContinue");
        let end = both::<FnCompressFlush>("ZBUFF_compressEnd");

        let mut rng = Rng::new(0xC12_0007);
        let levels: &[c_int] =
            &[c_int::MIN, -1_000_000, -1000, -100, -22, -1, 0, 1, 3, 19, 22, 23, 100, 1000, 1_000_000, c_int::MAX];
        for &lvl in levels {
            for _ in 0..8 {
                let cc = CCtx::new();
                let a = (cinit.0)(cc.c, lvl);
                let b = (cinit.1)(cc.r, lvl);
                ze.eq(&format!("compressInit lvl={lvl}"), a, b);
                // If init succeeded, a subsequent compress must also agree.
                if !ze.is_c_err(a) {
                    let src = gen(Shape::Text, 1 + rng.below(2048), &mut rng);
                    let mut c_ss = src.len();
                    let mut r_ss = src.len();
                    let mut c_dc = 1 << 16;
                    let mut r_dc = 1 << 16;
                    let mut cout = vec![0u8; 1 << 16];
                    let mut rout = vec![0u8; 1 << 16];
                    let ca = (cont.0)(cc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc,
                        src.as_ptr() as *const c_void, &mut c_ss);
                    let cb = (cont.1)(cc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc,
                        src.as_ptr() as *const c_void, &mut r_ss);
                    ze.eq(&format!("continue after lvl={lvl}"), ca, cb);
                    assert_eq!(c_ss, r_ss, "lvl={lvl} srcConsumed");
                    assert_eq!(c_dc, r_dc, "lvl={lvl} dstWritten");
                    assert_bytes_eq(&format!("lvl={lvl} bytes"), &cout[..c_dc], &rout[..r_dc]);
                    // finalize
                    let mut c_dc = 1 << 16;
                    let mut r_dc = 1 << 16;
                    ze.eq(&format!("end after lvl={lvl}"),
                        (end.0)(cc.c, cout.as_mut_ptr() as *mut c_void, &mut c_dc),
                        (end.1)(cc.r, rout.as_mut_ptr() as *mut c_void, &mut r_dc));
                    assert_eq!(c_dc, r_dc, "lvl={lvl} end dstWritten");
                }
            }
        }
    }
}

/// `ZBUFF_compressInitDictionary` / `ZBUFF_decompressInitDictionary` error
/// paths: NULL dict + nonzero size, a corrupted trained-dictionary blob, and
/// dictSize 0 (valid). Compare boolean + string in all cases.
#[test]
fn init_dictionary_error_paths() {
    unsafe {
        let ze = ZbuffErr::new();
        let cinitd = both::<FnCompressInitDict>("ZBUFF_compressInitDictionary");
        let dinitd = both::<FnDecompressInitDict>("ZBUFF_decompressInitDictionary");

        let mut rng = Rng::new(0xC12_0008);

        // (a) NULL dict pointer with a nonzero declared size.
        for &sz in &[1usize, 8, 100, 4096, 100_000] {
            let cc = CCtx::new();
            ze.eq(&format!("cinitd NULL dict sz={sz}"),
                (cinitd.0)(cc.c, std::ptr::null(), sz, 3),
                (cinitd.1)(cc.r, std::ptr::null(), sz, 3));
            let dc = DCtx::new();
            ze.eq(&format!("dinitd NULL dict sz={sz}"),
                (dinitd.0)(dc.c, std::ptr::null(), sz),
                (dinitd.1)(dc.r, std::ptr::null(), sz));
        }

        // (b) A corrupted "trained dictionary": start with the dictionary magic
        // (ZSTD_MAGIC_DICTIONARY = 0xEC30A437) then random garbage so header
        // parsing is exercised on the failure path.
        for _ in 0..64 {
            let n = 8 + rng.below(2048);
            let mut dict: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            dict[..4].copy_from_slice(&0xEC30A437u32.to_le_bytes());
            let dptr = dict.as_ptr() as *const c_void;
            let cc = CCtx::new();
            ze.eq(&format!("cinitd corrupt dict n={n}"),
                (cinitd.0)(cc.c, dptr, dict.len(), 3),
                (cinitd.1)(cc.r, dptr, dict.len(), 3));
            let dc = DCtx::new();
            ze.eq(&format!("dinitd corrupt dict n={n}"),
                (dinitd.0)(dc.c, dptr, dict.len()),
                (dinitd.1)(dc.r, dptr, dict.len()));
        }

        // (c) A raw (content-only) corrupted dict without the magic.
        for _ in 0..64 {
            let n = 1 + rng.below(4096);
            let dict: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let dptr = dict.as_ptr() as *const c_void;
            let cc = CCtx::new();
            ze.eq(&format!("cinitd raw dict n={n}"),
                (cinitd.0)(cc.c, dptr, dict.len(), 3),
                (cinitd.1)(cc.r, dptr, dict.len(), 3));
            let dc = DCtx::new();
            ze.eq(&format!("dinitd raw dict n={n}"),
                (dinitd.0)(dc.c, dptr, dict.len()),
                (dinitd.1)(dc.r, dptr, dict.len()));
        }

        // (d) dictSize 0 with NULL and non-NULL pointer (valid => no-dict init).
        {
            let cc = CCtx::new();
            ze.eq("cinitd null,0", (cinitd.0)(cc.c, std::ptr::null(), 0, 3),
                (cinitd.1)(cc.r, std::ptr::null(), 0, 3));
            let dc = DCtx::new();
            ze.eq("dinitd null,0", (dinitd.0)(dc.c, std::ptr::null(), 0),
                (dinitd.1)(dc.r, std::ptr::null(), 0));
            let some = [1u8, 2, 3, 4];
            let cc = CCtx::new();
            ze.eq("cinitd ptr,0", (cinitd.0)(cc.c, some.as_ptr() as *const c_void, 0, 3),
                (cinitd.1)(cc.r, some.as_ptr() as *const c_void, 0, 3));
            let dc = DCtx::new();
            ze.eq("dinitd ptr,0", (dinitd.0)(dc.c, some.as_ptr() as *const c_void, 0),
                (dinitd.1)(dc.r, some.as_ptr() as *const c_void, 0));
        }
    }
}

/// `ZBUFF_getErrorName` / `ZBUFF_isError` over every int from -200 to 400,
/// reinterpreted as a `size_t` return code. This covers valid enum variants,
/// the "no error" region, and out-of-range codes with no valid variant.
#[test]
fn error_name_full_range() {
    unsafe {
        let (cis, ris) = both::<FnIsError>("ZBUFF_isError");
        let (cname, rname) = both::<FnGetErrorName>("ZBUFF_getErrorName");
        // A ZSTD error code is encoded as (size_t)(-code). We probe both the
        // raw small values AND the negated-code encoding for -200..=400.
        for v in -200i64..=400 {
            let raw = v as size_t; // wraps for negatives -> huge size_t (error region)
            let ci = cis(raw) != 0;
            let ri = ris(raw) != 0;
            assert_eq!(ci, ri, "ZBUFF_isError({v}) raw={raw:#x}");
            let cn = cstr(cname(raw));
            let rn = cstr(rname(raw));
            assert_eq!(cn, rn, "ZBUFF_getErrorName({v}) raw={raw:#x} C={cn:?} RS={rn:?}");
        }
        // Also probe the canonical error encoding 0-(size_t)code for code 0..=400,
        // which is how ZSTD actually returns error codes.
        for code in 0i64..=400 {
            let raw = (0u64).wrapping_sub(code as u64) as size_t;
            let ci = cis(raw) != 0;
            let ri = ris(raw) != 0;
            assert_eq!(ci, ri, "ZBUFF_isError(-{code}) raw={raw:#x}");
            let cn = cstr(cname(raw));
            let rn = cstr(rname(raw));
            assert_eq!(cn, rn, "ZBUFF_getErrorName(-{code}) raw={raw:#x} C={cn:?} RS={rn:?}");
        }
    }
}


// ------------------------------------------------------- advanced / recommended

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_customMem {
    alloc: *mut c_void,
    free: *mut c_void,
    opaque: *mut c_void,
}
impl ZSTD_customMem {
    fn null() -> Self {
        ZSTD_customMem { alloc: std::ptr::null_mut(), free: std::ptr::null_mut(), opaque: std::ptr::null_mut() }
    }
}
type FnCreateAdv = unsafe extern "C" fn(ZSTD_customMem) -> *mut c_void;
type FnVoidSize = unsafe extern "C" fn() -> size_t;
type FnCompressInitAdv =
    unsafe extern "C" fn(*mut c_void, *const c_void, size_t, ZSTD_parameters, c_ulonglong) -> size_t;

/// The recommended-buffer-size helpers must agree, and the `_advanced` context
/// creators must both allocate (non-null) and free cleanly. `ZBUFF_compressInit_advanced`
/// is driven with invalid `ZSTD_parameters` (out-of-range windowLog/strategy) so
/// the `ZSTD_checkCParams` error path is exercised identically in both libraries.
#[test]
fn recommended_sizes_and_advanced_ctx() {
    unsafe {
        let ze = ZbuffErr::new();
        // recommended sizes
        for name in [
            "ZBUFF_recommendedCInSize",
            "ZBUFF_recommendedCOutSize",
            "ZBUFF_recommendedDInSize",
            "ZBUFF_recommendedDOutSize",
        ] {
            let (a, b) = both::<FnVoidSize>(name);
            assert_eq!(a(), b(), "{name}");
        }
        // advanced create/free lifecycle
        let (cca, rca) = both::<FnCreateAdv>("ZBUFF_createCCtx_advanced");
        let (cda, rda) = both::<FnCreateAdv>("ZBUFF_createDCtx_advanced");
        let (cfree, rfree) = both::<FnFree>("ZBUFF_freeCCtx");
        let (cdfree, rdfree) = both::<FnFree>("ZBUFF_freeDCtx");
        for _ in 0..32 {
            let cc = cca(ZSTD_customMem::null());
            let rc = rca(ZSTD_customMem::null());
            let cd = cda(ZSTD_customMem::null());
            let rd = rda(ZSTD_customMem::null());
            assert!(!cc.is_null() && !rc.is_null(), "createCCtx_advanced null");
            assert!(!cd.is_null() && !rd.is_null(), "createDCtx_advanced null");
            ze.eq("freeCCtx adv", cfree(cc), rfree(rc));
            ze.eq("freeDCtx adv", cdfree(cd), rdfree(rd));
        }

        // compressInit_advanced with INVALID parameters (error path)
        let cia = both::<FnCompressInitAdv>("ZBUFF_compressInit_advanced");
        let bad_params: &[ZSTD_parameters] = &[
            // wildly out-of-range windowLog + strategy => checkCParams fails
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters {
                    windowLog: 99, chainLog: 99, hashLog: 99, searchLog: 99,
                    minMatch: 99, targetLength: 0, strategy: 99,
                },
                fParams: ZSTD_frameParameters::default(),
            },
            ZSTD_parameters {
                cParams: ZSTD_compressionParameters {
                    windowLog: 0, chainLog: 0, hashLog: 0, searchLog: 0,
                    minMatch: 0, targetLength: 0, strategy: 0,
                },
                fParams: ZSTD_frameParameters::default(),
            },
            ZSTD_parameters::default(),
        ];
        for (i, p) in bad_params.iter().enumerate() {
            let cc = cca(ZSTD_customMem::null());
            let rc = rca(ZSTD_customMem::null());
            for &pledged in &[0u64, 100, u64::MAX] {
                ze.eq(&format!("compressInit_advanced bad#{i} pledged={pledged}"),
                    (cia.0)(cc, std::ptr::null(), 0, *p, pledged),
                    (cia.1)(rc, std::ptr::null(), 0, *p, pledged));
            }
            cfree(cc);
            rfree(rc);
        }
    }
}
