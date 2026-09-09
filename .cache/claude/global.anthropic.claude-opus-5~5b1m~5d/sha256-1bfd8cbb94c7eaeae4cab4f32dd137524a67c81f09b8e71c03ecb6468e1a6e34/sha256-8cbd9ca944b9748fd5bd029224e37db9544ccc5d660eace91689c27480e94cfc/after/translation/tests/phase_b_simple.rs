//! Phase B: differential tests for the simple / one-shot API and frame-info API.
#![allow(non_snake_case)]
mod common;
use common::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnCCtxCompress =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDCtxDecompress = unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnCreateCCtx = unsafe extern "C" fn() -> *mut c_void;
type FnFreeCCtx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    c_int,
) -> usize;
type FnDecUsingDict =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize, *const u8, usize) -> usize;
type FnU64FromBuf = unsafe extern "C" fn(*const u8, usize) -> c_ulonglong;
type FnSizeFromBuf = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnCLevel = unsafe extern "C" fn() -> c_int;
type FnBound = unsafe extern "C" fn(usize) -> usize;

const LEVELS: &[c_int] = &[
    -131072, -10000, -100, -22, -7, -5, -3, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 20, 21, 22,
];

const SIZES: &[usize] = &[0, 1, 2, 3, 7, 16, 63, 100, 999, 4096, 65535, 131071, 131072, 131073,
    200000];

fn levels_quick() -> Vec<c_int> {
    vec![-5, -1, 1, 3, 6, 9, 12, 15, 17, 19, 22]
}

/// compress with both libs and require byte-identical output; return the bytes.
fn diff_compress(level: c_int, src: &[u8], ctx: &str) -> Vec<u8> {
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let cap = unsafe { cb(src.len()) } + 64;
    let mut cbuf = vec![0xAAu8; cap];
    let mut rbuf = vec![0x55u8; cap];
    let cn = unsafe { cc(cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    let rn = unsafe { rc(rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    assert_eq!(cn, rn, "{ctx}: ZSTD_compress return value differs");
    assert!(!is_error(cn), "{ctx}: C compress errored: {}", err_code(cn));
    assert_bytes_eq(&format!("{ctx}: ZSTD_compress bytes"), &cbuf[..cn], &rbuf[..rn]);
    cbuf.truncate(cn);
    cbuf
}

fn diff_decompress(frame: &[u8], expect: &[u8], ctx: &str) {
    let (cd, rd) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let cap = expect.len() + 16;
    let mut co = vec![0u8; cap];
    let mut ro = vec![0u8; cap];
    let cn = unsafe { cd(co.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    let rn = unsafe { rd(ro.as_mut_ptr(), cap, frame.as_ptr(), frame.len()) };
    assert_eq!(cn, rn, "{ctx}: ZSTD_decompress return differs");
    assert!(!is_error(cn), "{ctx}: C decompress errored {}", err_code(cn));
    assert_eq!(cn, expect.len(), "{ctx}: decompressed size");
    assert_bytes_eq(&format!("{ctx}: C decompressed"), &co[..cn], expect);
    assert_bytes_eq(&format!("{ctx}: Rust decompressed"), &ro[..rn], expect);
}

#[test]
fn cfg_levels_x_shapes_simple_api() {
    let mut rng = Rng::new(0xC0FFEE);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 7, 100, 4096, 65535, 131072, 131073] {
            let src = gen(shape, size, &mut rng);
            for &lvl in &levels_quick() {
                let ctx = format!("shape={shape:?} size={size} level={lvl}");
                let frame = diff_compress(lvl, &src, &ctx);
                diff_decompress(&frame, &src, &ctx);
            }
        }
    }
}

#[test]
fn cfg_all_levels_random_sizes() {
    let mut rng = Rng::new(7);
    for &lvl in LEVELS {
        for _ in 0..4 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let size = rng.range(0, 40000);
            let src = gen(shape, size, &mut rng);
            let ctx = format!("all-levels lvl={lvl} shape={shape:?} size={size}");
            let frame = diff_compress(lvl, &src, &ctx);
            diff_decompress(&frame, &src, &ctx);
        }
    }
}

#[test]
fn cfg_all_sizes_boundaries() {
    let mut rng = Rng::new(99);
    for &size in SIZES {
        for &shape in &[Shape::Random, Shape::Rle, Shape::Text] {
            let src = gen(shape, size, &mut rng);
            for &lvl in &[1, 3, 9, 19] {
                let ctx = format!("boundary size={size} shape={shape:?} lvl={lvl}");
                let frame = diff_compress(lvl, &src, &ctx);
                diff_decompress(&frame, &src, &ctx);
            }
        }
    }
}

#[test]
fn cfg_cctx_dctx_oneshot() {
    let (c_new, r_new) = unsafe { pair::<FnCreateCCtx>("ZSTD_createCCtx") };
    let (c_free, r_free) = unsafe { pair::<FnFreeCCtx>("ZSTD_freeCCtx") };
    let (c_cc, r_cc) = unsafe { pair::<FnCCtxCompress>("ZSTD_compressCCtx") };
    let (c_dnew, r_dnew) = unsafe { pair::<FnCreateCCtx>("ZSTD_createDCtx") };
    let (c_dfree, r_dfree) = unsafe { pair::<FnFreeCCtx>("ZSTD_freeDCtx") };
    let (c_dd, r_dd) = unsafe { pair::<FnDCtxDecompress>("ZSTD_decompressDCtx") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let cctx = unsafe { c_new() };
    let rctx = unsafe { r_new() };
    let cdctx = unsafe { c_dnew() };
    let rdctx = unsafe { r_dnew() };
    assert!(!cctx.is_null() && !rctx.is_null() && !cdctx.is_null() && !rdctx.is_null());

    let mut rng = Rng::new(4242);
    // reuse the same context many times (exercises context reuse paths)
    for round in 0..40 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = rng.range(0, 70000);
        let lvl: c_int = [1, 2, 3, 5, 8, 11, 14, 17, 19, 22, -1, -5][rng.below(12)];
        let src = gen(shape, size, &mut rng);
        let cap = unsafe { cb(src.len()) } + 32;
        let mut cbuf = vec![0u8; cap];
        let mut rbuf = vec![0u8; cap];
        let cn = unsafe { c_cc(cctx, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        let rn = unsafe { r_cc(rctx, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), lvl) };
        let ctx = format!("compressCCtx round={round} shape={shape:?} size={size} lvl={lvl}");
        assert_eq!(cn, rn, "{ctx}");
        assert!(!is_error(cn), "{ctx} err {}", err_code(cn));
        assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);

        let mut co = vec![0u8; size + 8];
        let mut ro = vec![0u8; size + 8];
        let cdn = unsafe { c_dd(cdctx, co.as_mut_ptr(), co.len(), cbuf.as_ptr(), cn) };
        let rdn = unsafe { r_dd(rdctx, ro.as_mut_ptr(), ro.len(), rbuf.as_ptr(), rn) };
        assert_eq!(cdn, rdn, "{ctx}: decompressDCtx");
        assert_eq!(cdn, size, "{ctx}");
        assert_bytes_eq(&format!("{ctx} dec"), &co[..cdn], &src);
        assert_bytes_eq(&format!("{ctx} dec-rust"), &ro[..rdn], &src);
    }
    unsafe {
        c_free(cctx);
        r_free(rctx);
        c_dfree(cdctx);
        r_dfree(rdctx);
    }
}

#[test]
fn cfg_compress_using_dict_rawcontent() {
    let (c_new, r_new) = unsafe { pair::<FnCreateCCtx>("ZSTD_createCCtx") };
    let (c_free, r_free) = unsafe { pair::<FnFreeCCtx>("ZSTD_freeCCtx") };
    let (c_ud, r_ud) = unsafe { pair::<FnUsingDict>("ZSTD_compress_usingDict") };
    let (c_dnew, r_dnew) = unsafe { pair::<FnCreateCCtx>("ZSTD_createDCtx") };
    let (c_dfree, r_dfree) = unsafe { pair::<FnFreeCCtx>("ZSTD_freeDCtx") };
    let (c_dud, r_dud) = unsafe { pair::<FnDecUsingDict>("ZSTD_decompress_usingDict") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };

    let cctx = unsafe { c_new() };
    let rctx = unsafe { r_new() };
    let cdctx = unsafe { c_dnew() };
    let rdctx = unsafe { r_dnew() };
    let mut rng = Rng::new(31337);

    for &dsize in &[0usize, 1, 100, 8192, 100000] {
        let dict = gen(Shape::Text, dsize, &mut rng);
        for &lvl in &[1, 3, 9, 19] {
            let src = gen(Shape::Text, rng.range(10, 30000), &mut rng);
            let cap = unsafe { cb(src.len()) } + 32;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cn = unsafe {
                c_ud(cctx, cbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), dict.as_ptr(),
                     dict.len(), lvl)
            };
            let rn = unsafe {
                r_ud(rctx, rbuf.as_mut_ptr(), cap, src.as_ptr(), src.len(), dict.as_ptr(),
                     dict.len(), lvl)
            };
            let ctx = format!("usingDict dsize={dsize} lvl={lvl} srclen={}", src.len());
            assert_eq!(cn, rn, "{ctx}");
            assert!(!is_error(cn), "{ctx} err {}", err_code(cn));
            assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);

            let mut co = vec![0u8; src.len() + 8];
            let mut ro = vec![0u8; src.len() + 8];
            let cdn = unsafe {
                c_dud(cdctx, co.as_mut_ptr(), co.len(), cbuf.as_ptr(), cn, dict.as_ptr(), dict.len())
            };
            let rdn = unsafe {
                r_dud(rdctx, ro.as_mut_ptr(), ro.len(), rbuf.as_ptr(), rn, dict.as_ptr(), dict.len())
            };
            assert_eq!(cdn, rdn, "{ctx}: decompress_usingDict");
            assert_eq!(cdn, src.len(), "{ctx}");
            assert_bytes_eq(&format!("{ctx} dec"), &co[..cdn], &src);
            assert_bytes_eq(&format!("{ctx} decr"), &ro[..rdn], &src);
        }
    }
    unsafe {
        c_free(cctx);
        r_free(rctx);
        c_dfree(cdctx);
        r_dfree(rdctx);
    }
}

#[test]
fn cfg_frame_info_functions() {
    let (c_fcs, r_fcs) = unsafe { pair::<FnU64FromBuf>("ZSTD_getFrameContentSize") };
    let (c_gds, r_gds) = unsafe { pair::<FnU64FromBuf>("ZSTD_getDecompressedSize") };
    let (c_ffcs, r_ffcs) = unsafe { pair::<FnSizeFromBuf>("ZSTD_findFrameCompressedSize") };
    let (c_fds, r_fds) = unsafe { pair::<FnU64FromBuf>("ZSTD_findDecompressedSize") };
    let (c_db, r_db) = unsafe { pair::<FnU64FromBuf>("ZSTD_decompressBound") };
    let (c_fhs, r_fhs) = unsafe { pair::<FnSizeFromBuf>("ZSTD_frameHeaderSize") };

    let mut rng = Rng::new(555);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 100, 70000, 200000] {
            let src = gen(shape, size, &mut rng);
            let frame = diff_compress(3, &src, "frameinfo");
            let ctx = format!("frameinfo shape={shape:?} size={size}");
            unsafe {
                assert_eq!(
                    c_fcs(frame.as_ptr(), frame.len()),
                    r_fcs(frame.as_ptr(), frame.len()),
                    "{ctx} getFrameContentSize"
                );
                assert_eq!(
                    c_gds(frame.as_ptr(), frame.len()),
                    r_gds(frame.as_ptr(), frame.len()),
                    "{ctx} getDecompressedSize"
                );
                assert_eq!(
                    c_ffcs(frame.as_ptr(), frame.len()),
                    r_ffcs(frame.as_ptr(), frame.len()),
                    "{ctx} findFrameCompressedSize"
                );
                assert_eq!(
                    c_fds(frame.as_ptr(), frame.len()),
                    r_fds(frame.as_ptr(), frame.len()),
                    "{ctx} findDecompressedSize"
                );
                assert_eq!(
                    c_db(frame.as_ptr(), frame.len()),
                    r_db(frame.as_ptr(), frame.len()),
                    "{ctx} decompressBound"
                );
                // frameHeaderSize on truncated prefixes too
                for n in 1..=frame.len().min(20) {
                    assert_eq!(
                        c_fhs(frame.as_ptr(), n),
                        r_fhs(frame.as_ptr(), n),
                        "{ctx} frameHeaderSize(n={n})"
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_multi_frame_concatenated() {
    let (c_fds, r_fds) = unsafe { pair::<FnU64FromBuf>("ZSTD_findDecompressedSize") };
    let (c_db, r_db) = unsafe { pair::<FnU64FromBuf>("ZSTD_decompressBound") };
    let mut rng = Rng::new(2024);
    for nframes in 1..=5 {
        let mut all = Vec::new();
        let mut raw = Vec::new();
        for i in 0..nframes {
            let src = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 5000), &mut rng);
            let f = diff_compress((i % 5 + 1) as c_int, &src, "multiframe");
            all.extend_from_slice(&f);
            raw.extend_from_slice(&src);
        }
        let ctx = format!("multiframe n={nframes}");
        unsafe {
            assert_eq!(
                c_fds(all.as_ptr(), all.len()),
                r_fds(all.as_ptr(), all.len()),
                "{ctx} findDecompressedSize"
            );
            assert_eq!(
                c_db(all.as_ptr(), all.len()),
                r_db(all.as_ptr(), all.len()),
                "{ctx} decompressBound"
            );
        }
        diff_decompress(&all, &raw, &ctx);
    }
}

#[test]
fn cfg_level_query_functions() {
    let (c_min, r_min) = unsafe { pair::<FnCLevel>("ZSTD_minCLevel") };
    let (c_max, r_max) = unsafe { pair::<FnCLevel>("ZSTD_maxCLevel") };
    let (c_def, r_def) = unsafe { pair::<FnCLevel>("ZSTD_defaultCLevel") };
    unsafe {
        assert_eq!(c_min(), r_min());
        assert_eq!(c_max(), r_max());
        assert_eq!(c_def(), r_def());
    }
    let (cb, rb) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    for n in [0usize, 1, 2, 1024, 131072, 1 << 20, 1 << 30, usize::MAX / 2, usize::MAX] {
        assert_eq!(unsafe { cb(n) }, unsafe { rb(n) }, "compressBound({n})");
    }
}
