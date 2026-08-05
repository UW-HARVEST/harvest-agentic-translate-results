//! Phase B — valid-path differential tests. Loads BOTH C and Rust .so and
//! compares outputs byte-for-byte across randomized inputs.
mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_int, c_void};

// ---------- CONFIGS row 1: ZSTD_compressBound ----------
#[test]
fn config01_compress_bound() {
    let libs = Libs::load();
    let c: libloading::Symbol<FnCompressBound> = sym(&libs.c, b"ZSTD_compressBound");
    let r: libloading::Symbol<FnCompressBound> = sym(&libs.rust, b"ZSTD_compressBound");
    let mut rng = Rng::new(1);
    let mut sizes = vec![0usize, 1, 2, 127, 128, 255, 256, 1023, 1024, 65535, 65536,
        (128 << 10) - 1, 128 << 10, (128 << 10) + 1, 1 << 20];
    for _ in 0..2000 {
        sizes.push((rng.next_u64() % (1u64 << 34)) as usize);
    }
    for s in sizes {
        unsafe {
            assert_eq!(c(s), r(s), "compressBound({})", s);
        }
    }
}

// ---------- CONFIGS row 2: error helpers ----------
#[test]
fn config02_error_helpers() {
    let libs = Libs::load();
    let ce: libloading::Symbol<FnIsError> = sym(&libs.c, b"ZSTD_isError");
    let re: libloading::Symbol<FnIsError> = sym(&libs.rust, b"ZSTD_isError");
    let cc: libloading::Symbol<FnGetErrorCode> = sym(&libs.c, b"ZSTD_getErrorCode");
    let rc: libloading::Symbol<FnGetErrorCode> = sym(&libs.rust, b"ZSTD_getErrorCode");
    let cn: libloading::Symbol<FnGetErrorName> = sym(&libs.c, b"ZSTD_getErrorName");
    let rn: libloading::Symbol<FnGetErrorName> = sym(&libs.rust, b"ZSTD_getErrorName");
    let mut rng = Rng::new(2);
    let mut vals: Vec<usize> = vec![0, 1, 2, 5, 100, 1000, usize::MAX, usize::MAX - 1];
    for code in 0..=130usize {
        vals.push(0usize.wrapping_sub(code));
    }
    for _ in 0..1000 {
        vals.push(rng.next_u64() as usize);
    }
    unsafe {
        for v in vals {
            assert_eq!(ce(v), re(v), "isError({})", v);
            assert_eq!(cc(v), rc(v), "getErrorCode({})", v);
            let cs = CStr::from_ptr(cn(v));
            let rs = CStr::from_ptr(rn(v));
            assert_eq!(cs, rs, "getErrorName({})", v);
        }
    }
}

// ---------- CONFIGS row 3: version + clevel constants ----------
#[test]
fn config03_constants() {
    let libs = Libs::load();
    unsafe {
        let cvn: libloading::Symbol<FnVersionNumber> = sym(&libs.c, b"ZSTD_versionNumber");
        let rvn: libloading::Symbol<FnVersionNumber> = sym(&libs.rust, b"ZSTD_versionNumber");
        assert_eq!(cvn(), rvn());
        let cvs: libloading::Symbol<FnVersionString> = sym(&libs.c, b"ZSTD_versionString");
        let rvs: libloading::Symbol<FnVersionString> = sym(&libs.rust, b"ZSTD_versionString");
        assert_eq!(CStr::from_ptr(cvs()), CStr::from_ptr(rvs()));
        for name in [
            &b"ZSTD_minCLevel"[..],
            &b"ZSTD_maxCLevel"[..],
            &b"ZSTD_defaultCLevel"[..],
        ] {
            let cf: libloading::Symbol<FnClevel> = sym(&libs.c, name);
            let rf: libloading::Symbol<FnClevel> = sym(&libs.rust, name);
            assert_eq!(cf(), rf(), "{:?}", String::from_utf8_lossy(name));
        }
    }
}

/// Helper: one-shot compress with a given lib and level, returns compressed bytes.
unsafe fn compress_with(f: &FnCompress, src: &[u8], level: c_int) -> Result<Vec<u8>, usize> {
    let bound = {
        // Use a generous bound.
        src.len() + (src.len() >> 8) + 512
    };
    let mut dst = vec![0u8; bound.max(64)];
    let r = f(
        dst.as_mut_ptr() as *mut c_void,
        dst.len(),
        src.as_ptr() as *const c_void,
        src.len(),
        level,
    );
    let is_err = r > (0usize.wrapping_sub(130));
    if is_err {
        return Err(r);
    }
    dst.truncate(r);
    Ok(dst)
}

// ---------- CONFIGS rows 4-8, 27: ZSTD_compress <-> ZSTD_decompress ----------
#[test]
fn config04to08_roundtrip_oneshot() {
    let libs = Libs::load();
    let cc: libloading::Symbol<FnCompress> = sym(&libs.c, b"ZSTD_compress");
    let rc: libloading::Symbol<FnCompress> = sym(&libs.rust, b"ZSTD_compress");
    let cd: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");
    let rd: libloading::Symbol<FnDecompress> = sym(&libs.rust, b"ZSTD_decompress");
    let mut rng = Rng::new(42);

    let levels: [c_int; 8] = [-5, -1, 0, 1, 3, 9, 19, 22];
    let sizes: [usize; 8] = [0, 1, 2, 100, 1000, 65536, 200_000, 300_000];

    for &size in &sizes {
        for kind in 0..2 {
            let mut src = vec![0u8; size];
            if kind == 0 {
                rng.fill_compressible(&mut src);
            } else {
                rng.fill_random(&mut src);
            }
            for &lvl in &levels {
                unsafe {
                    let cbuf = compress_with(&cc, &src, lvl).expect("C compress");
                    let rbuf = compress_with(&rc, &src, lvl).expect("Rust compress");
                    assert_eq!(
                        cbuf, rbuf,
                        "compressed bytes differ size={} kind={} lvl={}",
                        size, kind, lvl
                    );
                    // decompress both compressed buffers with both decompressors.
                    let mut out_c = vec![0u8; size + 16];
                    let mut out_r = vec![0u8; size + 16];
                    let dc = cd(
                        out_c.as_mut_ptr() as *mut c_void,
                        out_c.len(),
                        cbuf.as_ptr() as *const c_void,
                        cbuf.len(),
                    );
                    let dr = rd(
                        out_r.as_mut_ptr() as *mut c_void,
                        out_r.len(),
                        cbuf.as_ptr() as *const c_void,
                        cbuf.len(),
                    );
                    assert_eq!(dc, dr, "decompress return differ size={} lvl={}", size, lvl);
                    assert_eq!(dc, size, "decompressed size");
                    assert_eq!(&out_c[..size], &src[..], "C decompress mismatch");
                    assert_eq!(&out_r[..size], &src[..], "Rust decompress mismatch");
                }
            }
        }
    }
}

// ---------- CONFIGS rows 9: ctx-based compress/decompress ----------
#[test]
fn config09_ctx_roundtrip() {
    let libs = Libs::load();
    let create_cc_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
    let create_cc_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createCCtx");
    let free_cc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
    let free_cc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeCCtx");
    let comp_c: libloading::Symbol<FnCompressCCtx> = sym(&libs.c, b"ZSTD_compressCCtx");
    let comp_r: libloading::Symbol<FnCompressCCtx> = sym(&libs.rust, b"ZSTD_compressCCtx");
    let create_dc_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createDCtx");
    let create_dc_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createDCtx");
    let free_dc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeDCtx");
    let free_dc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeDCtx");
    let dec_c: libloading::Symbol<FnDecompressDCtx> = sym(&libs.c, b"ZSTD_decompressDCtx");
    let dec_r: libloading::Symbol<FnDecompressDCtx> = sym(&libs.rust, b"ZSTD_decompressDCtx");

    let mut rng = Rng::new(7);
    unsafe {
        let cctx_c = create_cc_c();
        let cctx_r = create_cc_r();
        let dctx_c = create_dc_c();
        let dctx_r = create_dc_r();
        for &lvl in &[1i32, 3, 9, 19] {
            for &size in &[0usize, 50, 5000, 150_000] {
                let mut src = vec![0u8; size];
                rng.fill_compressible(&mut src);
                let cap = size + (size >> 8) + 512;
                let mut db_c = vec![0u8; cap];
                let mut db_r = vec![0u8; cap];
                let n_c = comp_c(cctx_c, db_c.as_mut_ptr() as *mut c_void, db_c.len(),
                    src.as_ptr() as *const c_void, src.len(), lvl);
                let n_r = comp_r(cctx_r, db_r.as_mut_ptr() as *mut c_void, db_r.len(),
                    src.as_ptr() as *const c_void, src.len(), lvl);
                assert_eq!(n_c, n_r, "compressCCtx size return lvl={} size={}", lvl, size);
                assert_eq!(&db_c[..n_c], &db_r[..n_r], "compressCCtx bytes lvl={} size={}", lvl, size);
                let mut out_c = vec![0u8; size + 16];
                let mut out_r = vec![0u8; size + 16];
                let d_c = dec_c(dctx_c, out_c.as_mut_ptr() as *mut c_void, out_c.len(),
                    db_c.as_ptr() as *const c_void, n_c);
                let d_r = dec_r(dctx_r, out_r.as_mut_ptr() as *mut c_void, out_r.len(),
                    db_r.as_ptr() as *const c_void, n_r);
                assert_eq!(d_c, d_r);
                assert_eq!(d_c, size);
                assert_eq!(&out_c[..size], &src[..]);
                assert_eq!(&out_r[..size], &src[..]);
            }
        }
        assert_eq!(free_cc_c(cctx_c), free_cc_r(cctx_r));
        assert_eq!(free_dc_c(dctx_c), free_dc_r(dctx_r));
    }
}

// ---------- CONFIGS rows 10-12,30: frame content size introspection ----------
#[test]
fn config10to12_frame_introspection() {
    let libs = Libs::load();
    let cc: libloading::Symbol<FnCompress> = sym(&libs.c, b"ZSTD_compress");
    let fcs_c: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.c, b"ZSTD_getFrameContentSize");
    let fcs_r: libloading::Symbol<FnGetFrameContentSize> = sym(&libs.rust, b"ZSTD_getFrameContentSize");
    let ds_c: libloading::Symbol<FnGetDecompressedSize> = sym(&libs.c, b"ZSTD_getDecompressedSize");
    let ds_r: libloading::Symbol<FnGetDecompressedSize> = sym(&libs.rust, b"ZSTD_getDecompressedSize");
    let ffcs_c: libloading::Symbol<FnFindFrameCompressedSize> = sym(&libs.c, b"ZSTD_findFrameCompressedSize");
    let ffcs_r: libloading::Symbol<FnFindFrameCompressedSize> = sym(&libs.rust, b"ZSTD_findFrameCompressedSize");
    let db_c: libloading::Symbol<FnDecompressBound> = sym(&libs.c, b"ZSTD_decompressBound");
    let db_r: libloading::Symbol<FnDecompressBound> = sym(&libs.rust, b"ZSTD_decompressBound");
    let mut rng = Rng::new(11);
    unsafe {
        for &size in &[0usize, 1, 100, 5000, 200_000] {
            let mut src = vec![0u8; size];
            rng.fill_compressible(&mut src);
            let frame = compress_with(&cc, &src, 3).unwrap();
            let f = frame.as_ptr() as *const c_void;
            assert_eq!(fcs_c(f, frame.len()), fcs_r(f, frame.len()), "fcs size={}", size);
            assert_eq!(ds_c(f, frame.len()), ds_r(f, frame.len()), "ds size={}", size);
            assert_eq!(ffcs_c(f, frame.len()), ffcs_r(f, frame.len()), "ffcs size={}", size);
            assert_eq!(db_c(f, frame.len()), db_r(f, frame.len()), "dbound size={}", size);
        }
    }
}

/// Set a parameter, compress2, verify roundtrip and byte-identity vs C.
unsafe fn compress2_with(
    create: &FnCreateCtx,
    setp: &FnSetParameter,
    comp2: &FnCompress2,
    free: &FnFreeCtx,
    params: &[(c_int, c_int)],
    src: &[u8],
) -> Vec<u8> {
    let ctx = create();
    for &(p, v) in params {
        let r = setp(ctx, p, v);
        assert_eq!(r > (0usize.wrapping_sub(130)), false, "setParameter({},{}) failed", p, v);
    }
    let cap = src.len() + (src.len() >> 8) + 512;
    let mut dst = vec![0u8; cap];
    let n = comp2(ctx, dst.as_mut_ptr() as *mut c_void, dst.len(),
        src.as_ptr() as *const c_void, src.len());
    assert_eq!(n > (0usize.wrapping_sub(130)), false, "compress2 failed: {}", n);
    dst.truncate(n);
    free(ctx);
    dst
}

// ---------- CONFIGS rows 13-18, 26: compress2 + parameters ----------
#[test]
fn config13to18_compress2_params() {
    let libs = Libs::load();
    let cr_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
    let cr_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createCCtx");
    let sp_c: libloading::Symbol<FnSetParameter> = sym(&libs.c, b"ZSTD_CCtx_setParameter");
    let sp_r: libloading::Symbol<FnSetParameter> = sym(&libs.rust, b"ZSTD_CCtx_setParameter");
    let c2_c: libloading::Symbol<FnCompress2> = sym(&libs.c, b"ZSTD_compress2");
    let c2_r: libloading::Symbol<FnCompress2> = sym(&libs.rust, b"ZSTD_compress2");
    let fc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
    let fc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeCCtx");
    let dec_c: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");

    let mut rng = Rng::new(99);

    // Build parameter sets to exercise.
    let mut param_sets: Vec<Vec<(c_int, c_int)>> = Vec::new();
    for lvl in [1, 3, 9, 19] {
        param_sets.push(vec![(ZSTD_C_COMPRESSION_LEVEL, lvl)]);
    }
    param_sets.push(vec![(ZSTD_C_CHECKSUMFLAG, 1)]);
    param_sets.push(vec![(ZSTD_C_CONTENTSIZEFLAG, 0)]);
    param_sets.push(vec![(ZSTD_C_DICTIDFLAG, 0)]);
    for wl in [10, 15, 20, 23] {
        param_sets.push(vec![(ZSTD_C_WINDOWLOG, wl)]);
    }
    for strat in 1..=9 {
        param_sets.push(vec![(ZSTD_C_STRATEGY, strat), (ZSTD_C_COMPRESSION_LEVEL, 5)]);
    }
    param_sets.push(vec![(ZSTD_C_ENABLE_LDM, 1), (ZSTD_C_WINDOWLOG, 20)]);
    for mm in [3, 4, 5, 6, 7] {
        param_sets.push(vec![(ZSTD_C_MINMATCH, mm)]);
    }
    for tl in [0, 32, 256, 999] {
        param_sets.push(vec![(ZSTD_C_TARGETLENGTH, tl), (ZSTD_C_STRATEGY, 8)]);
    }

    let sizes = [0usize, 100, 5000, 150_000];
    unsafe {
        for pset in &param_sets {
            for &size in &sizes {
                let mut src = vec![0u8; size];
                rng.fill_compressible(&mut src);
                let cbuf = compress2_with(&cr_c, &sp_c, &c2_c, &fc_c, pset, &src);
                let rbuf = compress2_with(&cr_r, &sp_r, &c2_r, &fc_r, pset, &src);
                assert_eq!(cbuf, rbuf, "compress2 bytes differ params={:?} size={}", pset, size);
                // roundtrip decompress (both frames identical, use C decompress).
                let mut out = vec![0u8; size + 16];
                let d = dec_c(out.as_mut_ptr() as *mut c_void, out.len(),
                    cbuf.as_ptr() as *const c_void, cbuf.len());
                assert_eq!(d, size, "roundtrip params={:?} size={}", pset, size);
                assert_eq!(&out[..size], &src[..]);
            }
        }
    }
}

// ---------- CONFIGS rows 20-21: parameter bounds ----------
#[test]
fn config20_21_bounds() {
    let libs = Libs::load();
    let cb_c: libloading::Symbol<FnGetBounds> = sym(&libs.c, b"ZSTD_cParam_getBounds");
    let cb_r: libloading::Symbol<FnGetBounds> = sym(&libs.rust, b"ZSTD_cParam_getBounds");
    let db_c: libloading::Symbol<FnGetBounds> = sym(&libs.c, b"ZSTD_dParam_getBounds");
    let db_r: libloading::Symbol<FnGetBounds> = sym(&libs.rust, b"ZSTD_dParam_getBounds");
    unsafe {
        // sweep a wide range of enum ints incl invalid ones.
        for p in [-5i32, 0, 100, 101, 102, 103, 104, 105, 106, 107, 130, 160,
            161, 162, 163, 164, 200, 201, 202, 400, 401, 402, 500, 10, 1000,
            1001, 1002, 1004, 1017, 9999] {
            let a = cb_c(p);
            let b = cb_r(p);
            assert_eq!(a, b, "cParam_getBounds({})", p);
        }
        for p in [-1i32, 0, 100, 101, 1000, 9999] {
            let a = db_c(p);
            let b = db_r(p);
            assert_eq!(a, b, "dParam_getBounds({})", p);
        }
    }
}

// ---------- CONFIGS row 23: stream sizes ----------
#[test]
fn config23_stream_sizes() {
    let libs = Libs::load();
    for name in [
        &b"ZSTD_CStreamInSize"[..],
        &b"ZSTD_CStreamOutSize"[..],
        &b"ZSTD_DStreamInSize"[..],
        &b"ZSTD_DStreamOutSize"[..],
    ] {
        unsafe {
            let cf: libloading::Symbol<FnSizeVoid> = sym(&libs.c, name);
            let rf: libloading::Symbol<FnSizeVoid> = sym(&libs.rust, name);
            assert_eq!(cf(), rf(), "{:?}", String::from_utf8_lossy(name));
        }
    }
}

// ---------- CONFIGS row 24: XXH via ZSTD_ namespace ----------
#[test]
fn config24_xxhash() {
    let libs = Libs::load();
    unsafe {
        let x64c: libloading::Symbol<FnXxh64> = sym(&libs.c, b"ZSTD_XXH64");
        let x64r: libloading::Symbol<FnXxh64> = sym(&libs.rust, b"ZSTD_XXH64");
        let x32c: libloading::Symbol<FnXxh32> = sym(&libs.c, b"ZSTD_XXH32");
        let x32r: libloading::Symbol<FnXxh32> = sym(&libs.rust, b"ZSTD_XXH32");
        let mut rng = Rng::new(2024);
        for len in 0..300usize {
            let mut buf = vec![0u8; len];
            rng.fill_random(&mut buf);
            let seed = rng.next_u64();
            let p = buf.as_ptr() as *const c_void;
            assert_eq!(x64c(p, len, seed), x64r(p, len, seed), "XXH64 len={}", len);
            assert_eq!(x32c(p, len, seed as u32), x32r(p, len, seed as u32), "XXH32 len={}", len);
        }
        // a few large buffers
        for _ in 0..20 {
            let len = 4096 + (rng.next_u32() % 100000) as usize;
            let mut buf = vec![0u8; len];
            rng.fill_random(&mut buf);
            let p = buf.as_ptr() as *const c_void;
            assert_eq!(x64c(p, len, 0), x64r(p, len, 0), "XXH64 big len={}", len);
            assert_eq!(x32c(p, len, 0), x32r(p, len, 0), "XXH32 big len={}", len);
        }
    }
}

// ---------- CONFIGS row 19: streaming compress/decompress ----------
#[test]
fn config19_streaming() {
    let libs = Libs::load();
    let cr_c: libloading::Symbol<FnCreateCtx> = sym(&libs.c, b"ZSTD_createCCtx");
    let cr_r: libloading::Symbol<FnCreateCtx> = sym(&libs.rust, b"ZSTD_createCCtx");
    let sp_c: libloading::Symbol<FnSetParameter> = sym(&libs.c, b"ZSTD_CCtx_setParameter");
    let sp_r: libloading::Symbol<FnSetParameter> = sym(&libs.rust, b"ZSTD_CCtx_setParameter");
    let cs2_c: libloading::Symbol<FnCompressStream2> = sym(&libs.c, b"ZSTD_compressStream2");
    let cs2_r: libloading::Symbol<FnCompressStream2> = sym(&libs.rust, b"ZSTD_compressStream2");
    let fc_c: libloading::Symbol<FnFreeCtx> = sym(&libs.c, b"ZSTD_freeCCtx");
    let fc_r: libloading::Symbol<FnFreeCtx> = sym(&libs.rust, b"ZSTD_freeCCtx");
    let dec_c: libloading::Symbol<FnDecompress> = sym(&libs.c, b"ZSTD_decompress");

    let mut rng = Rng::new(555);
    unsafe {
        for &size in &[0usize, 10, 1000, 100_000] {
            for &chunk in &[1usize, 7, 4096] {
                let mut src = vec![0u8; size];
                rng.fill_compressible(&mut src);
                let out_c = stream_compress(&cr_c, &sp_c, &cs2_c, &fc_c, &src, chunk);
                let out_r = stream_compress(&cr_r, &sp_r, &cs2_r, &fc_r, &src, chunk);
                assert_eq!(out_c, out_r, "stream bytes differ size={} chunk={}", size, chunk);
                let mut d = vec![0u8; size + 16];
                let n = dec_c(d.as_mut_ptr() as *mut c_void, d.len(),
                    out_c.as_ptr() as *const c_void, out_c.len());
                assert_eq!(n, size);
                assert_eq!(&d[..size], &src[..]);
            }
        }
    }
}

unsafe fn stream_compress(
    create: &FnCreateCtx,
    setp: &FnSetParameter,
    cs2: &FnCompressStream2,
    free: &FnFreeCtx,
    src: &[u8],
    chunk: usize,
) -> Vec<u8> {
    let ctx = create();
    setp(ctx, ZSTD_C_COMPRESSION_LEVEL, 5);
    let mut output: Vec<u8> = Vec::new();
    let mut outbuf = vec![0u8; 1 << 16];
    let mut in_pos = 0usize;
    loop {
        let end = (in_pos + chunk).min(src.len());
        let last = end == src.len();
        let mut inb = ZstdInBuffer {
            src: src.as_ptr() as *const c_void,
            size: end,
            pos: in_pos,
        };
        loop {
            let mut ob = ZstdOutBuffer {
                dst: outbuf.as_mut_ptr() as *mut c_void,
                size: outbuf.len(),
                pos: 0,
            };
            let mode = if last { ZSTD_E_END } else { ZSTD_E_CONTINUE };
            let rem = cs2(ctx, &mut ob, &mut inb, mode);
            assert_eq!(rem > (0usize.wrapping_sub(130)), false, "compressStream2 err {}", rem);
            output.extend_from_slice(&outbuf[..ob.pos]);
            if last {
                if rem == 0 {
                    break;
                }
            } else if inb.pos == inb.size {
                break;
            }
        }
        in_pos = end;
        if last {
            break;
        }
    }
    free(ctx);
    output
}
