// Phase C — error-path differential tests (one per ERRORS.md row).
// Asserts C and Rust return the SAME error code / sentinel.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct FrameInfo {
    block_size_id: c_uint,
    block_mode: c_uint,
    content_checksum: c_uint,
    frame_type: c_uint,
    content_size: u64,
    dict_id: c_uint,
    block_checksum: c_uint,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Preferences {
    frame_info: FrameInfo,
    compression_level: c_int,
    auto_flush: c_uint,
    favor_dec_speed: c_uint,
    reserved: [c_uint; 3],
}
fn base_prefs() -> Preferences {
    Preferences {
        frame_info: FrameInfo { block_size_id: 0, block_mode: 0, content_checksum: 0, frame_type: 0, content_size: 0, dict_id: 0, block_checksum: 0 },
        compression_level: 0, auto_flush: 0, favor_dec_speed: 0, reserved: [0; 3],
    }
}
const LZ4F_VERSION: c_uint = 100;

// ---- helpers for querying error codes on both libs ----
type IsError = unsafe extern "C" fn(usize) -> c_uint;
type GetErrorCode = unsafe extern "C" fn(usize) -> c_int; // LZ4F_getErrorCode returns LZ4F_errorCodes
type GetErrorName = unsafe extern "C" fn(usize) -> *const c_char;

macro_rules! csym_t { ($libs:expr, $t:ty, $n:expr) => {{ let s: libloading::Symbol<$t> = csym($libs, $n); s }} }
macro_rules! rsym_t { ($libs:expr, $t:ty, $n:expr) => {{ let s: libloading::Symbol<$t> = rsym($libs, $n); s }} }

// ============ Block API errors (lz4.c / lz4hc.c) ============

#[test]
fn err_compress_bound_oversize() {
    // ERRORS.md #1
    let libs = Libs::load();
    unsafe {
        type F = unsafe extern "C" fn(c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_compressBound");
        let r = rsym_t!(&libs, F, b"LZ4_compressBound");
        for sz in [-1, 0, 0x7E000000i32, 0x7E000001i32, i32::MAX, i32::MIN] {
            assert_eq!(c(sz), r(sz), "compressBound({})", sz);
            if sz < 0 || sz > 0x7E000000 { assert_eq!(c(sz), 0); }
        }
    }
}

#[test]
fn err_compress_default_too_large_and_dst_small() {
    // ERRORS.md #2, #3
    let libs = Libs::load();
    let mut rng = Rng::new(0xe002);
    unsafe {
        type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_compress_default");
        let r = rsym_t!(&libs, F, b"LZ4_compress_default");
        let backing = rng.compressible(1000);
        // dst way too small -> returns 0
        for dstcap in [0i32, 1, 2, 5] {
            let mut cd = vec![0u8; 16];
            let mut rd = vec![0u8; 16];
            let cn = c(backing.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, 1000, dstcap);
            let rn = r(backing.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, 1000, dstcap);
            assert_eq!(cn, rn, "dst too small cap={}", dstcap);
            assert_eq!(cn, 0);
        }
        // srcSize negative / too large -> returns 0
        for badsrc in [-1i32, i32::MIN, 0x7E000001i32] {
            let mut cd = vec![0u8; 64];
            let mut rd = vec![0u8; 64];
            let cn = c(backing.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, badsrc, 64);
            let rn = r(backing.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, badsrc, 64);
            assert_eq!(cn, rn, "bad srcSize={}", badsrc);
        }
    }
}

#[test]
fn err_decompress_safe_null_and_negative() {
    // ERRORS.md #6, #7, #8, #9
    let libs = Libs::load();
    let mut rng = Rng::new(0xe006);
    unsafe {
        type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_decompress_safe");
        let r = rsym_t!(&libs, F, b"LZ4_decompress_safe");
        // NULL src => -1
        let mut cd = vec![0u8; 64];
        let mut rd = vec![0u8; 64];
        let cn = c(std::ptr::null(), cd.as_mut_ptr() as *mut c_char, 10, 64);
        let rn = r(std::ptr::null(), rd.as_mut_ptr() as *mut c_char, 10, 64);
        assert_eq!(cn, rn, "null src"); assert!(cn < 0);

        // build a valid compressed buffer, then feed too-small dstCapacity & corrupt data
        type C = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let comp = csym_t!(&libs, C, b"LZ4_compress_default");
        let src = rng.compressible(500);
        let mut cbuf = vec![0u8; 1000];
        let cn = comp(src.as_ptr() as *const c_char, cbuf.as_mut_ptr() as *mut c_char, 500, 1000);

        // dstCapacity too small
        for dcap in [0i32, 1, 10, 100] {
            let mut co = vec![0u8; dcap.max(1) as usize];
            let mut ro = vec![0u8; dcap.max(1) as usize];
            let cr = c(cbuf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, dcap);
            let rr = r(cbuf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, dcap);
            assert_eq!(cr, rr, "small dstcap {}", dcap);
        }
        // corrupted / malformed input
        for seed in 0..30u64 {
            let mut bad = cbuf[..cn as usize].to_vec();
            let mut rr2 = Rng::new(seed ^ 0xabc);
            for _ in 0..3 { let i = rr2.range(bad.len()); bad[i] = rr2.byte(); }
            let mut co = vec![0u8; 600];
            let mut ro = vec![0u8; 600];
            let cr = c(bad.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, bad.len() as c_int, 600);
            let rr = r(bad.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, bad.len() as c_int, 600);
            assert_eq!(cr, rr, "corrupt seed {}", seed);
            if cr >= 0 { assert_eq!(&co[..cr as usize], &ro[..rr as usize]); }
        }
        // srcSize == 0 (non-partial) => -1
        let cr = c(cbuf.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, 0, 64);
        let rr = r(cbuf.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, 0, 64);
        assert_eq!(cr, rr, "srcSize 0"); assert!(cr < 0);
    }
}

#[test]
fn err_hc_negative_params() {
    // ERRORS.md #11, #12, #13
    let libs = Libs::load();
    let mut rng = Rng::new(0xe011);
    unsafe {
        type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_compress_HC");
        let r = rsym_t!(&libs, F, b"LZ4_compress_HC");
        let src = rng.compressible(1000);
        // dst too small
        for dcap in [0i32, 1, 3] {
            let mut cd = vec![0u8; 16];
            let mut rd = vec![0u8; 16];
            let cn = c(src.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, 1000, dcap, 9);
            let rn = r(src.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, 1000, dcap, 9);
            assert_eq!(cn, rn, "HC dst small {}", dcap);
            assert_eq!(cn, 0);
        }
    }
}

#[test]
fn err_hc_destsize_bounds() {
    // ERRORS.md #14, #15
    let libs = Libs::load();
    let mut rng = Rng::new(0xe014);
    unsafe {
        type Sz = unsafe extern "C" fn() -> c_int;
        let css = csym_t!(&libs, Sz, b"LZ4_sizeofStateHC");
        let state_sz = css() as usize;
        type F = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_compress_HC_destSize");
        let r = rsym_t!(&libs, F, b"LZ4_compress_HC_destSize");
        let src = rng.compressible(2000);
        // dstCapacity < 1
        for dcap in [0i32] {
            let mut cs = vec![0u8; state_sz + 16];
            let mut rs = vec![0u8; state_sz + 16];
            let mut csrc = 2000; let mut rsrc = 2000;
            let mut cd = vec![0u8; 16]; let mut rd = vec![0u8; 16];
            let cn = c(cs.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, &mut csrc, dcap, 9);
            let rn = r(rs.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, &mut rsrc, dcap, 9);
            assert_eq!(cn, rn, "HC destSize dcap {}", dcap);
        }
        // srcSize too large
        {
            let mut cs = vec![0u8; state_sz + 16];
            let mut rs = vec![0u8; state_sz + 16];
            let mut csrc = 0x7E000001i32; let mut rsrc = 0x7E000001i32;
            let mut cd = vec![0u8; 64]; let mut rd = vec![0u8; 64];
            let cn = c(cs.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, &mut csrc, 64, 9);
            let rn = r(rs.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, &mut rsrc, 64, 9);
            assert_eq!(cn, rn, "HC destSize srcSize too large");
        }
    }
}

#[test]
fn err_initstream_bad_and_free_null() {
    // ERRORS.md #16, #17
    let libs = Libs::load();
    unsafe {
        type Init = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        let c = csym_t!(&libs, Init, b"LZ4_initStream");
        let r = rsym_t!(&libs, Init, b"LZ4_initStream");
        // NULL buffer
        assert_eq!(c(std::ptr::null_mut(), 100).is_null(), r(std::ptr::null_mut(), 100).is_null());
        // too-small size
        let mut buf = vec![0u8; 4096];
        let cp = c(buf.as_mut_ptr() as *mut c_void, 4);
        let rp = r(buf.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(cp.is_null(), rp.is_null(), "initStream small size");

        // free on NULL
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        let cf = csym_t!(&libs, Free, b"LZ4_freeStream");
        let rf = rsym_t!(&libs, Free, b"LZ4_freeStream");
        assert_eq!(cf(std::ptr::null_mut()), rf(std::ptr::null_mut()));
        let cfh = csym_t!(&libs, Free, b"LZ4_freeStreamHC");
        let rfh = rsym_t!(&libs, Free, b"LZ4_freeStreamHC");
        assert_eq!(cfh(std::ptr::null_mut()), rfh(std::ptr::null_mut()));
    }
}

#[test]
fn err_decoder_ringbuffer_invalid() {
    // ERRORS.md #18
    let libs = Libs::load();
    unsafe {
        type F = unsafe extern "C" fn(c_int) -> c_int;
        let c = csym_t!(&libs, F, b"LZ4_decoderRingBufferSize");
        let r = rsym_t!(&libs, F, b"LZ4_decoderRingBufferSize");
        for mbs in [-1i32, 0, 1, 16, 65536, i32::MAX] {
            assert_eq!(c(mbs), r(mbs), "decoderRingBufferSize({})", mbs);
        }
    }
}

// ============ Frame API errors (lz4frame.c) ============

fn cmp_error(libs: &Libs, ce: usize, re: usize, ctx: &str) {
    unsafe {
        let c_ie = csym_t!(libs, IsError, b"LZ4F_isError");
        let r_ie = rsym_t!(libs, IsError, b"LZ4F_isError");
        assert_eq!(c_ie(ce), r_ie(re), "isError disagree: {}", ctx);
        if c_ie(ce) != 0 {
            let c_gc = csym_t!(libs, GetErrorCode, b"LZ4F_getErrorCode");
            let r_gc = rsym_t!(libs, GetErrorCode, b"LZ4F_getErrorCode");
            assert_eq!(c_gc(ce), r_gc(re), "error code disagree: {}", ctx);
            // and the raw sentinel value must match too
            assert_eq!(ce, re, "raw error sentinel disagree: {}", ctx);
        }
    }
}

#[test]
fn err_frame_compress_dst_too_small() {
    // ERRORS.md #19
    let libs = Libs::load();
    let mut rng = Rng::new(0xe019);
    unsafe {
        type F = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
        let c = csym_t!(&libs, F, b"LZ4F_compressFrame");
        let r = rsym_t!(&libs, F, b"LZ4F_compressFrame");
        let prefs = base_prefs();
        let data = rng.compressible(5000);
        for dcap in [0usize, 1, 10, 20] {
            let mut cd = vec![0u8; dcap.max(1)];
            let mut rd = vec![0u8; dcap.max(1)];
            let ce = c(cd.as_mut_ptr() as *mut c_void, dcap, data.as_ptr() as *const c_void, data.len(), &prefs);
            let re = r(rd.as_mut_ptr() as *mut c_void, dcap, data.as_ptr() as *const c_void, data.len(), &prefs);
            cmp_error(&libs, ce, re, &format!("compressFrame dcap={}", dcap));
        }
    }
}

#[test]
fn err_create_ctx_null() {
    // ERRORS.md #20, #28
    let libs = Libs::load();
    unsafe {
        type F = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        let cc = csym_t!(&libs, F, b"LZ4F_createCompressionContext");
        let rc = rsym_t!(&libs, F, b"LZ4F_createCompressionContext");
        let ce = cc(std::ptr::null_mut(), LZ4F_VERSION);
        let re = rc(std::ptr::null_mut(), LZ4F_VERSION);
        cmp_error(&libs, ce, re, "createCompressionContext null");

        let cd = csym_t!(&libs, F, b"LZ4F_createDecompressionContext");
        let rd = rsym_t!(&libs, F, b"LZ4F_createDecompressionContext");
        let ce = cd(std::ptr::null_mut(), LZ4F_VERSION);
        let re = rd(std::ptr::null_mut(), LZ4F_VERSION);
        cmp_error(&libs, ce, re, "createDecompressionContext null");
    }
}

#[test]
fn err_headersize_and_getframeinfo() {
    // ERRORS.md #29, #30, #31, #32, #33, #34, #35, #36
    let libs = Libs::load();
    unsafe {
        type HS = unsafe extern "C" fn(*const c_void, usize) -> usize;
        let c = csym_t!(&libs, HS, b"LZ4F_headerSize");
        let r = rsym_t!(&libs, HS, b"LZ4F_headerSize");
        // too-small srcSize
        let buf = [0u8; 20];
        for avail in [0usize, 1, 2, 3, 4] {
            let ce = c(buf.as_ptr() as *const c_void, avail);
            let re = r(buf.as_ptr() as *const c_void, avail);
            cmp_error(&libs, ce, re, &format!("headerSize avail={}", avail));
        }
        // Bad magic numbers / reserved bits / bad version -> various errors
        let cases: &[[u8; 7]] = &[
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // unknown frameType/magic
            [0x04, 0x22, 0x4D, 0x18, 0x00, 0x00, 0x00], // magic ok, FLG=0 -> bad version
            [0x04, 0x22, 0x4D, 0x18, 0x40, 0x40, 0x00], // version=1, various flags
            [0x04, 0x22, 0x4D, 0x18, 0x60, 0x00, 0x00], // version 1, blockSizeID 0
            [0x04, 0x22, 0x4D, 0x18, 0x62, 0x70, 0x00], // reserved bits set
        ];
        for (i, c7) in cases.iter().enumerate() {
            let ce = c(c7.as_ptr() as *const c_void, 7);
            let re = r(c7.as_ptr() as *const c_void, 7);
            cmp_error(&libs, ce, re, &format!("headerSize case {}", i));
        }

        // getFrameInfo with NULL src, and with a real dctx on bad headers
        type CD = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type GFI = unsafe extern "C" fn(*mut c_void, *mut FrameInfo, *const c_void, *mut usize) -> usize;
        type FD = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_cd = csym_t!(&libs, CD, b"LZ4F_createDecompressionContext");
        let r_cd = rsym_t!(&libs, CD, b"LZ4F_createDecompressionContext");
        let c_gfi = csym_t!(&libs, GFI, b"LZ4F_getFrameInfo");
        let r_gfi = rsym_t!(&libs, GFI, b"LZ4F_getFrameInfo");
        let c_fd = csym_t!(&libs, FD, b"LZ4F_freeDecompressionContext");
        let r_fd = rsym_t!(&libs, FD, b"LZ4F_freeDecompressionContext");

        for (i, c7) in cases.iter().enumerate() {
            let mut cdx: *mut c_void = std::ptr::null_mut();
            let mut rdx: *mut c_void = std::ptr::null_mut();
            c_cd(&mut cdx, LZ4F_VERSION); r_cd(&mut rdx, LZ4F_VERSION);
            let mut cfi: FrameInfo = std::mem::zeroed();
            let mut rfi: FrameInfo = std::mem::zeroed();
            let mut cs = 7usize; let mut rs = 7usize;
            let ce = c_gfi(cdx, &mut cfi, c7.as_ptr() as *const c_void, &mut cs);
            let re = r_gfi(rdx, &mut rfi, c7.as_ptr() as *const c_void, &mut rs);
            cmp_error(&libs, ce, re, &format!("getFrameInfo case {}", i));
            c_fd(cdx); r_fd(rdx);
        }
    }
}

#[test]
fn err_decompress_corrupted_frames() {
    // ERRORS.md #38, #39, #40, #41, #42 — feed corrupted valid frames
    let libs = Libs::load();
    let mut rng = Rng::new(0xe040);
    unsafe {
        type CF = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
        type FB = unsafe extern "C" fn(usize, *const Preferences) -> usize;
        let c_cf = csym_t!(&libs, CF, b"LZ4F_compressFrame");
        let c_fb = csym_t!(&libs, FB, b"LZ4F_compressFrameBound");

        type CD = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FD = unsafe extern "C" fn(*mut c_void) -> usize;
        type DEC = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize, *const c_void, *mut usize, *const c_void) -> usize;
        let c_cd = csym_t!(&libs, CD, b"LZ4F_createDecompressionContext");
        let r_cd = rsym_t!(&libs, CD, b"LZ4F_createDecompressionContext");
        let c_fd = csym_t!(&libs, FD, b"LZ4F_freeDecompressionContext");
        let r_fd = rsym_t!(&libs, FD, b"LZ4F_freeDecompressionContext");
        let c_dec = csym_t!(&libs, DEC, b"LZ4F_decompress");
        let r_dec = rsym_t!(&libs, DEC, b"LZ4F_decompress");

        // enable checksums to exercise checksum-mismatch paths
        let mut prefs = base_prefs();
        prefs.frame_info.content_checksum = 1;
        prefs.frame_info.block_checksum = 1;
        let data = rng.compressible(8000);
        let bound = c_fb(data.len(), &prefs);
        let mut frame = vec![0u8; bound];
        let n = c_cf(frame.as_mut_ptr() as *mut c_void, bound, data.as_ptr() as *const c_void, data.len(), &prefs);
        frame.truncate(n);

        for seed in 0..60u64 {
            let mut bad = frame.clone();
            let mut rr = Rng::new(seed ^ 0x9e37);
            // corrupt a few bytes after the header (offset >= 7)
            let nflip = 1 + rr.range(4);
            for _ in 0..nflip {
                let idx = 7 + rr.range(bad.len().saturating_sub(7).max(1));
                let idx = idx.min(bad.len() - 1);
                bad[idx] ^= 1 << (rr.range(8));
            }
            // decode with both
            let mut cdx: *mut c_void = std::ptr::null_mut();
            let mut rdx: *mut c_void = std::ptr::null_mut();
            c_cd(&mut cdx, LZ4F_VERSION); r_cd(&mut rdx, LZ4F_VERSION);
            let mut cout = vec![0u8; data.len() + 100];
            let mut rout = vec![0u8; data.len() + 100];

            let c_res = drive_decompress(&c_dec, cdx, &bad, &mut cout);
            let r_res = drive_decompress(&r_dec, rdx, &bad, &mut rout);
            // Both must either error identically, or succeed identically
            match (c_res, r_res) {
                (Err(ce), Err(re)) => cmp_error(&libs, ce, re, &format!("corrupt frame seed {}", seed)),
                (Ok(cv), Ok(rv)) => assert_eq!(cv, rv, "corrupt-but-valid decode seed {}", seed),
                (a, b) => panic!("decode outcome mismatch seed {}: C={:?} Rust={:?}", seed, a.is_ok(), b.is_ok()),
            }
            c_fd(cdx); r_fd(rdx);
        }
    }
}

// Returns Ok(decoded bytes) or Err(error code from the first erroring call).
unsafe fn drive_decompress(
    dec: &libloading::Symbol<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize, *const c_void, *mut usize, *const c_void) -> usize>,
    dctx: *mut c_void,
    frame: &[u8],
    out: &mut [u8],
) -> Result<Vec<u8>, usize> {
    let mut sc = 0usize;
    let mut dp = 0usize;
    let mut iters = 0;
    loop {
        iters += 1;
        if iters > 100000 { break; }
        let mut src_sz = frame.len() - sc;
        let mut dst_sz = out.len() - dp;
        let ret = dec(
            dctx,
            out.as_mut_ptr().add(dp) as *mut c_void,
            &mut dst_sz,
            frame.as_ptr().add(sc) as *const c_void,
            &mut src_sz,
            std::ptr::null(),
        );
        // isError: LZ4F error codes are huge values (wrapping of -code)
        if ret > usize::MAX - 0x10000 {
            return Err(ret);
        }
        sc += src_sz;
        dp += dst_sz;
        if ret == 0 { break; }
        if src_sz == 0 && dst_sz == 0 { break; }
        if sc >= frame.len() && dst_sz == 0 { break; }
    }
    Ok(out[..dp].to_vec())
}

#[test]
fn err_getblocksize_invalid() {
    // ERRORS.md #33 (getBlockSize invalid id)
    let libs = Libs::load();
    unsafe {
        type F = unsafe extern "C" fn(c_uint) -> usize;
        let c = csym_t!(&libs, F, b"LZ4F_getBlockSize");
        let r = rsym_t!(&libs, F, b"LZ4F_getBlockSize");
        for id in [0u32, 1, 2, 3, 8, 9, 100, u32::MAX] {
            let ce = c(id);
            let re = r(id);
            cmp_error(&libs, ce, re, &format!("getBlockSize({})", id));
        }
    }
}

#[test]
fn err_compressupdate_uninitialized() {
    // ERRORS.md #22 — call compressUpdate on a freshly created (not begun) cctx
    let libs = Libs::load();
    let mut rng = Rng::new(0xe022);
    unsafe {
        type CC = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FC = unsafe extern "C" fn(*mut c_void) -> usize;
        type CU = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, *const c_void) -> usize;
        let c_cc = csym_t!(&libs, CC, b"LZ4F_createCompressionContext");
        let r_cc = rsym_t!(&libs, CC, b"LZ4F_createCompressionContext");
        let c_fc = csym_t!(&libs, FC, b"LZ4F_freeCompressionContext");
        let r_fc = rsym_t!(&libs, FC, b"LZ4F_freeCompressionContext");
        let c_cu = csym_t!(&libs, CU, b"LZ4F_compressUpdate");
        let r_cu = rsym_t!(&libs, CU, b"LZ4F_compressUpdate");
        let data = rng.compressible(100);
        let mut cctx: *mut c_void = std::ptr::null_mut();
        let mut rctx: *mut c_void = std::ptr::null_mut();
        c_cc(&mut cctx, LZ4F_VERSION); r_cc(&mut rctx, LZ4F_VERSION);
        let mut cd = vec![0u8; 1000]; let mut rd = vec![0u8; 1000];
        let ce = c_cu(cctx, cd.as_mut_ptr() as *mut c_void, 1000, data.as_ptr() as *const c_void, 100, std::ptr::null());
        let re = r_cu(rctx, rd.as_mut_ptr() as *mut c_void, 1000, data.as_ptr() as *const c_void, 100, std::ptr::null());
        cmp_error(&libs, ce, re, "compressUpdate uninitialized");
        c_fc(cctx); r_fc(rctx);
    }
}

#[test]
fn err_xxh_update_null() {
    // ERRORS.md #61 — XXH update with NULL input
    let libs = Libs::load();
    unsafe {
        type CS = unsafe extern "C" fn() -> *mut c_void;
        type RST = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
        type UPD = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type FS = unsafe extern "C" fn(*mut c_void) -> c_int;
        let c_cs = csym_t!(&libs, CS, b"LZ4_XXH32_createState");
        let r_cs = rsym_t!(&libs, CS, b"LZ4_XXH32_createState");
        let c_rst = csym_t!(&libs, RST, b"LZ4_XXH32_reset");
        let r_rst = rsym_t!(&libs, RST, b"LZ4_XXH32_reset");
        let c_upd = csym_t!(&libs, UPD, b"LZ4_XXH32_update");
        let r_upd = rsym_t!(&libs, UPD, b"LZ4_XXH32_update");
        let c_fs = csym_t!(&libs, FS, b"LZ4_XXH32_freeState");
        let r_fs = rsym_t!(&libs, FS, b"LZ4_XXH32_freeState");
        let cst = c_cs(); let rst = r_cs();
        c_rst(cst, 0); r_rst(rst, 0);
        // NULL input with len>0 => XXH_ERROR (1); with len==0 => XXH_OK (0)
        let ce = c_upd(cst, std::ptr::null(), 10);
        let re = r_upd(rst, std::ptr::null(), 10);
        assert_eq!(ce, re, "XXH32_update null len>0");
        let ce0 = c_upd(cst, std::ptr::null(), 0);
        let re0 = r_upd(rst, std::ptr::null(), 0);
        assert_eq!(ce0, re0, "XXH32_update null len==0");
        c_fs(cst); r_fs(rst);
    }
}

#[test]
fn err_file_null_params() {
    // ERRORS.md #47, #51, #53, #54, #57, #59 — NULL params on file API
    let libs = Libs::load();
    unsafe {
        type RO = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
        type RC = unsafe extern "C" fn(*mut c_void) -> usize;
        type WC = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_ro = csym_t!(&libs, RO, b"LZ4F_readOpen");
        let r_ro = rsym_t!(&libs, RO, b"LZ4F_readOpen");
        // readOpen with NULL fp
        let mut h: *mut c_void = std::ptr::null_mut();
        let ce = c_ro(&mut h, std::ptr::null_mut());
        let mut h2: *mut c_void = std::ptr::null_mut();
        let re = r_ro(&mut h2, std::ptr::null_mut());
        cmp_error(&libs, ce, re, "readOpen null fp");
        // readClose NULL
        let c_rc = csym_t!(&libs, RC, b"LZ4F_readClose");
        let r_rc = rsym_t!(&libs, RC, b"LZ4F_readClose");
        cmp_error(&libs, c_rc(std::ptr::null_mut()), r_rc(std::ptr::null_mut()), "readClose null");
        // writeClose NULL
        let c_wc = csym_t!(&libs, WC, b"LZ4F_writeClose");
        let r_wc = rsym_t!(&libs, WC, b"LZ4F_writeClose");
        cmp_error(&libs, c_wc(std::ptr::null_mut()), r_wc(std::ptr::null_mut()), "writeClose null");
    }
}

#[test]
fn err_out_of_range_enums_and_names() {
    // ERRORS.md #45, #46 — isError / getErrorName across the full code range,
    // including out-of-range values.
    let libs = Libs::load();
    unsafe {
        let c_ie = csym_t!(&libs, IsError, b"LZ4F_isError");
        let r_ie = rsym_t!(&libs, IsError, b"LZ4F_isError");
        let c_gc = csym_t!(&libs, GetErrorCode, b"LZ4F_getErrorCode");
        let r_gc = rsym_t!(&libs, GetErrorCode, b"LZ4F_getErrorCode");
        let c_gn = csym_t!(&libs, GetErrorName, b"LZ4F_getErrorName");
        let r_gn = rsym_t!(&libs, GetErrorName, b"LZ4F_getErrorName");
        // Sweep small return values 0..40 and the error-encoded values (-(0..40)).
        for v in 0usize..40 {
            assert_eq!(c_ie(v), r_ie(v), "isError({})", v);
            // encoded error: (size_t)-(ptrdiff_t)code
            let enc = (0usize).wrapping_sub(v);
            assert_eq!(c_ie(enc), r_ie(enc), "isError(enc {})", v);
            assert_eq!(c_gc(enc), r_gc(enc), "getErrorCode(enc {})", v);
            let cn = c_gn(enc);
            let rn = r_gn(enc);
            let cs = std::ffi::CStr::from_ptr(cn);
            let rs = std::ffi::CStr::from_ptr(rn);
            assert_eq!(cs, rs, "getErrorName(enc {})", v);
        }
    }
}
