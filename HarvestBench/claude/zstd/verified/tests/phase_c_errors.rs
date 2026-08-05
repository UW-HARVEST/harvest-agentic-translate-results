//! Phase C — error-path differential tests. Each test maps to a row in
//! ERRORS.md and asserts C and Rust return the SAME error/sentinel.
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_int, c_void};

const ERR_LIMIT: usize = 0usize.wrapping_sub(130); // values > this are error codes

fn is_err(v: usize) -> bool {
    v > ERR_LIMIT
}

/// Compress a small buffer with the C lib; return a valid frame for tests.
unsafe fn make_frame(libs: &Libs, data: &[u8]) -> Vec<u8> {
    let cc: libloading::Symbol<FnCompress> = sym(&libs.c, b"ZSTD_compress");
    let cap = data.len() + 512;
    let mut dst = vec![0u8; cap];
    let n = cc(
        dst.as_mut_ptr() as *mut c_void,
        dst.len(),
        data.as_ptr() as *const c_void,
        data.len(),
        3,
    );
    assert!(!is_err(n));
    dst.truncate(n);
    dst
}

// ERRORS row 1,24: ZSTD_compress dst too small
#[test]
fn err01_compress_dst_too_small() {
    let libs = Libs::load();
    let cc: libloading::Symbol<FnCompress> = sym(&libs.c, b"ZSTD_compress");
    let rc: libloading::Symbol<FnCompress> = sym(&libs.rust, b"ZSTD_compress");
    let src = vec![7u8; 4096];
    let mut d1 = vec![0u8; 8];
    let mut d2 = vec![0u8; 8];
    unsafe {
        let a = cc(d1.as_mut_ptr() as *mut c_void, d1.len(), src.as_ptr() as *const c_void, src.len(), 3);
        let b = rc(d2.as_mut_ptr() as *mut c_void, d2.len(), src.as_ptr() as *const c_void, src.len(), 3);
        assert_eq!(a, b, "return codes differ");
        assert!(is_err(a));
        assert_eq!(err_code(&libs, a), err_code(&libs, b));
    }
}

fn err_code(libs: &Libs, v: usize) -> c_int {
    unsafe {
        let gc: libloading::Symbol<FnGetErrorCode> = sym(&libs.c, b"ZSTD_getErrorCode");
        gc(v)
    }
}

// ERRORS row 3,25: decompress bad magic
#[test]
fn err03_decompress_bad_magic() {
    let libs = Libs::load();
    let cd: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");
    let rd: libloading::Symbol<FnDecompress> = sym(&libs.rust, b"ZSTD_decompress");
    let bad = vec![0xDEu8, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
    let mut o1 = vec![0u8; 256];
    let mut o2 = vec![0u8; 256];
    unsafe {
        let a = cd(o1.as_mut_ptr() as *mut c_void, o1.len(), bad.as_ptr() as *const c_void, bad.len());
        let b = rd(o2.as_mut_ptr() as *mut c_void, o2.len(), bad.as_ptr() as *const c_void, bad.len());
        assert_eq!(a, b);
        assert!(is_err(a));
        assert_eq!(err_code(&libs, a), err_code(&libs, b));
    }
}

// ERRORS row 4: decompress dst too small
#[test]
fn err04_decompress_dst_too_small() {
    let libs = Libs::load();
    let src = vec![9u8; 2000];
    unsafe {
        let libs2 = Libs::load();
        let frame = make_frame(&libs2, &src);
        let cd: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");
        let rd: libloading::Symbol<FnDecompress> = sym(&libs.rust, b"ZSTD_decompress");
        let mut o1 = vec![0u8; 10];
        let mut o2 = vec![0u8; 10];
        let a = cd(o1.as_mut_ptr() as *mut c_void, o1.len(), frame.as_ptr() as *const c_void, frame.len());
        let b = rd(o2.as_mut_ptr() as *mut c_void, o2.len(), frame.as_ptr() as *const c_void, frame.len());
        assert_eq!(a, b);
        assert!(is_err(a));
        assert_eq!(err_code(&libs, a), err_code(&libs, b));
    }
}

// ERRORS row 5,34: truncated / empty frame
#[test]
fn err05_decompress_truncated_and_empty() {
    let libs = Libs::load();
    let src = vec![3u8; 5000];
    unsafe {
        let libs2 = Libs::load();
        let frame = make_frame(&libs2, &src);
        let cd: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");
        let rd: libloading::Symbol<FnDecompress> = sym(&libs.rust, b"ZSTD_decompress");
        for cut in [0usize, 1, 2, 4, frame.len() / 2, frame.len() - 1] {
            let trunc = &frame[..cut];
            let mut o1 = vec![0u8; src.len() + 16];
            let mut o2 = vec![0u8; src.len() + 16];
            let a = cd(o1.as_mut_ptr() as *mut c_void, o1.len(), trunc.as_ptr() as *const c_void, trunc.len());
            let b = rd(o2.as_mut_ptr() as *mut c_void, o2.len(), trunc.as_ptr() as *const c_void, trunc.len());
            assert_eq!(a, b, "cut={}", cut);
            if is_err(a) {
                assert_eq!(err_code(&libs, a), err_code(&libs, b), "cut={}", cut);
            }
        }
    }
}

// ERRORS row 6: corrupted payload
#[test]
fn err06_corrupted_payload() {
    let libs = Libs::load();
    let mut rng = Rng::new(123);
    let mut src = vec![0u8; 8000];
    rng.fill_compressible(&mut src);
    unsafe {
        let libs2 = Libs::load();
        let mut frame = make_frame(&libs2, &src);
        let cd: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");
        let rd: libloading::Symbol<FnDecompress> = sym(&libs.rust, b"ZSTD_decompress");
        // Flip several bytes in the payload region (after the 6-byte header).
        for i in 0..40 {
            let idx = 6 + (i * 7) % (frame.len().saturating_sub(6)).max(1);
            let orig = frame[idx];
            frame[idx] ^= 0xA5;
            let mut o1 = vec![0u8; src.len() + 16];
            let mut o2 = vec![0u8; src.len() + 16];
            let a = cd(o1.as_mut_ptr() as *mut c_void, o1.len(), frame.as_ptr() as *const c_void, frame.len());
            let b = rd(o2.as_mut_ptr() as *mut c_void, o2.len(), frame.as_ptr() as *const c_void, frame.len());
            assert_eq!(a, b, "flip idx={}", idx);
            if is_err(a) {
                assert_eq!(err_code(&libs, a), err_code(&libs, b), "flip idx={}", idx);
            } else {
                // Non-error: outputs must match too.
                assert_eq!(o1[..a], o2[..a], "flip idx={}", idx);
            }
            frame[idx] = orig;
        }
    }
}

// ERRORS row 7,8,35: getFrameContentSize error / small / zero
#[test]
fn err07_08_35_frame_content_size() {
    let libs = Libs::load();
    let fc: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.c, b"ZSTD_getFrameContentSize");
    let fr: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.rust, b"ZSTD_getFrameContentSize");
    let bad = vec![0x00u8, 0x11, 0x22, 0x33];
    unsafe {
        // invalid magic
        let a = fc(bad.as_ptr() as *const c_void, bad.len());
        let b = fr(bad.as_ptr() as *const c_void, bad.len());
        assert_eq!(a, b);
        // zero size
        let a0 = fc(bad.as_ptr() as *const c_void, 0);
        let b0 = fr(bad.as_ptr() as *const c_void, 0);
        assert_eq!(a0, b0);
        // truncated valid header
        let libs2 = Libs::load();
        let frame = make_frame(&libs2, &vec![1u8; 100]);
        for cut in [1usize, 2, 3, 4, 5] {
            let a = fc(frame.as_ptr() as *const c_void, cut);
            let b = fr(frame.as_ptr() as *const c_void, cut);
            assert_eq!(a, b, "cut={}", cut);
        }
    }
}

// ERRORS row 9: unknown content size frame via streaming (contentSizeFlag / unknown pledged)
#[test]
fn err09_content_size_unknown() {
    let libs = Libs::load();
    // Build a frame with content size disabled via compress2.
    unsafe {
        let cr: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
        let sp: libloading::Symbol<FnSetParameter> = sym(&libs.c, b"ZSTD_CCtx_setParameter");
        let c2: libloading::Symbol<FnCompress2> = sym(&libs.c, b"ZSTD_compress2");
        let fc: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
        let ctx = cr();
        sp(ctx, ZSTD_C_CONTENTSIZEFLAG, 0);
        let src = vec![5u8; 3000];
        let mut dst = vec![0u8; 4096];
        let n = c2(ctx, dst.as_mut_ptr() as *mut c_void, dst.len(), src.as_ptr() as *const c_void, src.len());
        fc(ctx);
        dst.truncate(n);
        let gc: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.c, b"ZSTD_getFrameContentSize");
        let gr: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.rust, b"ZSTD_getFrameContentSize");
        let a = gc(dst.as_ptr() as *const c_void, dst.len());
        let b = gr(dst.as_ptr() as *const c_void, dst.len());
        assert_eq!(a, b);
        assert_eq!(a, CONTENTSIZE_UNKNOWN);
    }
}

// ERRORS row 10: findFrameCompressedSize invalid
#[test]
fn err10_find_frame_compressed_size_invalid() {
    let libs = Libs::load();
    let fc: libloading::Symbol<FnFindFrameCompressedSize> = sym(&libs.c, b"ZSTD_findFrameCompressedSize");
    let fr: libloading::Symbol<FnFindFrameCompressedSize> = sym(&libs.rust, b"ZSTD_findFrameCompressedSize");
    let bad = vec![0u8, 1, 2, 3];
    unsafe {
        let a = fc(bad.as_ptr() as *const c_void, bad.len());
        let b = fr(bad.as_ptr() as *const c_void, bad.len());
        assert_eq!(a, b);
        assert!(is_err(a));
        assert_eq!(err_code(&libs, a), err_code(&libs, b));
        // empty
        let a0 = fc(bad.as_ptr() as *const c_void, 0);
        let b0 = fr(bad.as_ptr() as *const c_void, 0);
        assert_eq!(a0, b0);
    }
}

// ERRORS row 11,22-boundary: compressBound at ZSTD_MAX_INPUT_SIZE
#[test]
fn err11_compress_bound_max() {
    let libs = Libs::load();
    let cb: libloading::Symbol<FnCompressBound> = sym(&libs.c, b"ZSTD_compressBound");
    let rb: libloading::Symbol<FnCompressBound> = sym(&libs.rust, b"ZSTD_compressBound");
    unsafe {
        for s in [usize::MAX, usize::MAX - 1, usize::MAX / 2, (1usize << 62), (1usize << 40)] {
            assert_eq!(cb(s), rb(s), "compressBound({})", s);
        }
    }
}

// ERRORS row 16: getErrorString out-of-range enum int
#[test]
fn err16_get_error_string_oob() {
    let libs = Libs::load();
    let cs: libloading::Symbol<FnGetErrorString> = sym(&libs.c, b"ZSTD_getErrorString");
    let rs: libloading::Symbol<FnGetErrorString> = sym(&libs.rust, b"ZSTD_getErrorString");
    unsafe {
        for code in [-100i32, -1, 0, 1, 10, 20, 40, 62, 120, 121, 200, 9999, i32::MAX, i32::MIN] {
            let a = CStr::from_ptr(cs(code));
            let b = CStr::from_ptr(rs(code));
            assert_eq!(a, b, "getErrorString({})", code);
        }
    }
}

// ERRORS row 17,18: CCtx_setParameter invalid param / OOB value
#[test]
fn err17_18_set_parameter() {
    let libs = Libs::load();
    unsafe {
        let cr_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
        let cr_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createCCtx");
        let sp_c: libloading::Symbol<FnSetParameter> = sym(&libs.c, b"ZSTD_CCtx_setParameter");
        let sp_r: libloading::Symbol<FnSetParameter> = sym(&libs.rust, b"ZSTD_CCtx_setParameter");
        let fc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
        let fc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeCCtx");
        let ctx_c = cr_c();
        let ctx_r = cr_r();
        // param, value pairs incl invalid params & OOB values, incl out-of-range enum ints
        let cases: &[(c_int, c_int)] = &[
            (99999, 1),          // unknown param
            (-1, 1),             // invalid param int
            (0, 1),              // invalid param int
            (ZSTD_C_WINDOWLOG, 99),   // OOB high
            (ZSTD_C_WINDOWLOG, 1),    // OOB low
            (ZSTD_C_STRATEGY, 999),   // OOB
            (ZSTD_C_STRATEGY, -1),    // OOB
            (ZSTD_C_MINMATCH, 99),    // OOB
            (ZSTD_C_HASHLOG, 999),    // OOB
            (ZSTD_C_COMPRESSION_LEVEL, 99999), // clamped, not error
            (ZSTD_C_CHECKSUMFLAG, 5), // bool clamp/error?
        ];
        for &(p, v) in cases {
            let a = sp_c(ctx_c, p, v);
            let b = sp_r(ctx_r, p, v);
            assert_eq!(a, b, "setParameter({},{}) return", p, v);
            if is_err(a) {
                assert_eq!(err_code(&libs, a), err_code(&libs, b), "setParameter({},{}) code", p, v);
            }
        }
        fc_c(ctx_c);
        fc_r(ctx_r);
    }
}

// ERRORS row 19,20: cParam/dParam_getBounds invalid enum
#[test]
fn err19_20_bounds_invalid() {
    let libs = Libs::load();
    unsafe {
        let cb_c: libloading::Symbol<FnGetBounds> = sym(&libs.c, b"ZSTD_cParam_getBounds");
        let cb_r: libloading::Symbol<FnGetBounds> = sym(&libs.rust, b"ZSTD_cParam_getBounds");
        let db_c: libloading::Symbol<FnGetBounds> = sym(&libs.c, b"ZSTD_dParam_getBounds");
        let db_r: libloading::Symbol<FnGetBounds> = sym(&libs.rust, b"ZSTD_dParam_getBounds");
        for p in [-999i32, -1, 0, 1, 50, 99, 999, 12345, i32::MAX, i32::MIN] {
            assert_eq!(cb_c(p), cb_r(p), "cParam bounds({})", p);
            assert_eq!(db_c(p), db_r(p), "dParam bounds({})", p);
        }
    }
}

// ERRORS row 21,22: DCtx_setParameter invalid / OOB
#[test]
fn err21_22_dctx_set_parameter() {
    let libs = Libs::load();
    unsafe {
        let cr_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createDCtx");
        let cr_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createDCtx");
        let sp_c: libloading::Symbol<FnSetParameter> = sym(&libs.c, b"ZSTD_DCtx_setParameter");
        let sp_r: libloading::Symbol<FnSetParameter> = sym(&libs.rust, b"ZSTD_DCtx_setParameter");
        let fc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeDCtx");
        let fc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeDCtx");
        let ctx_c = cr_c();
        let ctx_r = cr_r();
        let cases: &[(c_int, c_int)] = &[
            (99999, 1),
            (-1, 0),
            (ZSTD_D_WINDOWLOGMAX, 99),
            (ZSTD_D_WINDOWLOGMAX, 1),
            (ZSTD_D_WINDOWLOGMAX, 27),
        ];
        for &(p, v) in cases {
            let a = sp_c(ctx_c, p, v);
            let b = sp_r(ctx_r, p, v);
            assert_eq!(a, b, "DCtx setParameter({},{})", p, v);
            if is_err(a) {
                assert_eq!(err_code(&libs, a), err_code(&libs, b));
            }
        }
        fc_c(ctx_c);
        fc_r(ctx_r);
    }
}

// ERRORS row 26,27: free NULL returns 0
#[test]
fn err26_27_free_null() {
    let libs = Libs::load();
    unsafe {
        for name in [&b"ZSTD_freeCCtx"[..], &b"ZSTD_freeDCtx"[..]] {
            let fc: libloading::Symbol<FnFreeCtx> = sym(&libs.c, name);
            let fr: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, name);
            let a = fc(std::ptr::null_mut());
            let b = fr(std::ptr::null_mut());
            assert_eq!(a, b, "{:?}", String::from_utf8_lossy(name));
            assert_eq!(a, 0);
        }
    }
}

// ERRORS row 28: CCtx_reset invalid ResetDirective
#[test]
fn err28_cctx_reset_invalid() {
    let libs = Libs::load();
    unsafe {
        let cr_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
        let cr_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createCCtx");
        let rst_c: libloading::Symbol<FnCctxReset> = sym(&libs.c, b"ZSTD_CCtx_reset");
        let rst_r: libloading::Symbol<FnCctxReset> = sym(&libs.rust, b"ZSTD_CCtx_reset");
        let fc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
        let fc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeCCtx");
        let ctx_c = cr_c();
        let ctx_r = cr_r();
        for d in [-1i32, 0, 1, 2, 3, 4, 99] {
            let a = rst_c(ctx_c, d);
            let b = rst_r(ctx_r, d);
            assert_eq!(a, b, "reset({})", d);
            if is_err(a) {
                assert_eq!(err_code(&libs, a), err_code(&libs, b), "reset({})", d);
            }
        }
        fc_c(ctx_c);
        fc_r(ctx_r);
    }
}

// ERRORS row 29: getDictID_fromFrame non-dict frame -> 0
#[test]
fn err29_dictid_from_frame() {
    let libs = Libs::load();
    unsafe {
        let f_c: libloading::Symbol<unsafe extern "C" fn(*const c_void, usize) -> u32> =
            sym(&libs.c, b"ZSTD_getDictID_fromFrame");
        let f_r: libloading::Symbol<unsafe extern "C" fn(*const c_void, usize) -> u32> =
            sym(&libs.rust, b"ZSTD_getDictID_fromFrame");
        let libs2 = Libs::load();
        let frame = make_frame(&libs2, &vec![1u8; 200]);
        assert_eq!(
            f_c(frame.as_ptr() as *const c_void, frame.len()),
            f_r(frame.as_ptr() as *const c_void, frame.len())
        );
        let bad = vec![0u8; 4];
        assert_eq!(
            f_c(bad.as_ptr() as *const c_void, bad.len()),
            f_r(bad.as_ptr() as *const c_void, bad.len())
        );
    }
}

// ERRORS row 30,31: decompressBound / findDecompressedSize invalid
#[test]
fn err30_31_decompress_bound_invalid() {
    let libs = Libs::load();
    unsafe {
        let db_c: libloading::Symbol<FnDecompressBound> = sym(&libs.c, b"ZSTD_decompressBound");
        let db_r: libloading::Symbol<FnDecompressBound> = sym(&libs.rust, b"ZSTD_decompressBound");
        let fd_c: libloading::Symbol<FnDecompressBound> = sym(&libs.c, b"ZSTD_findDecompressedSize");
        let fd_r: libloading::Symbol<FnDecompressBound> = sym(&libs.rust, b"ZSTD_findDecompressedSize");
        let bad = vec![9u8, 8, 7, 6];
        assert_eq!(db_c(bad.as_ptr() as *const c_void, bad.len()), db_r(bad.as_ptr() as *const c_void, bad.len()));
        assert_eq!(fd_c(bad.as_ptr() as *const c_void, bad.len()), fd_r(bad.as_ptr() as *const c_void, bad.len()));
    }
}

// ERRORS row 33: ZDICT_trainFromBuffer insufficient samples
#[test]
fn err33_zdict_train_insufficient() {
    let libs = Libs::load();
    unsafe {
        type FnTrain = unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, u32) -> usize;
        let t_c: libloading::Symbol<FnTrain> = sym(&libs.c, b"ZDICT_trainFromBuffer");
        let t_r: libloading::Symbol<FnTrain> = sym(&libs.rust, b"ZDICT_trainFromBuffer");
        let ie_c: libloading::Symbol<FnIsError> = sym(&libs.c, b"ZDICT_isError");
        let ie_r: libloading::Symbol<FnIsError> = sym(&libs.rust, b"ZDICT_isError");
        // tiny samples -> should fail
        let samples = vec![0u8; 16];
        let sizes = vec![4usize, 4, 4, 4];
        let mut dict = vec![0u8; 1024];
        let a = t_c(dict.as_mut_ptr() as *mut c_void, dict.len(), samples.as_ptr() as *const c_void, sizes.as_ptr(), sizes.len() as u32);
        let b = t_r(dict.as_mut_ptr() as *mut c_void, dict.len(), samples.as_ptr() as *const c_void, sizes.as_ptr(), sizes.len() as u32);
        // Both should report error identically.
        assert_eq!(ie_c(a), ie_r(b), "isError mismatch a={} b={}", a, b);
        assert_eq!(a, b, "train return differ");
    }
}
