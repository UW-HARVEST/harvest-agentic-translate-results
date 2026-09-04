//! Phase C — LZ4 frame API error paths (`lz4frame.c`).
//! ERRORS.md rows 73–127 and 157–165.
#![allow(non_snake_case)]

mod common;
use common::frame::*;
use common::*;
use std::ffi::CStr;

fn err(name: &str) -> i64 {
    err_code(name) as i64
}

/* ================================================================== */
/* rows 123,124,125,126 — the error-reporting helpers themselves       */
/* ================================================================== */

#[test]
fn e123_error_helpers() {
    diff("isError/getErrorName/getErrorCode", |lib| unsafe {
        let is = sym::<FnIsError>(lib, "LZ4F_isError");
        let nm = sym::<FnErrName>(lib, "LZ4F_getErrorName");
        let cd = sym::<FnErrCode>(lib, "LZ4F_getErrorCode");
        let mut out: Vec<(u32, String, i32)> = Vec::new();
        // every defined error code, plus out-of-range values on both sides
        let mut codes: Vec<usize> = Vec::new();
        for k in 0..40usize {
            codes.push(0usize.wrapping_sub(k));
        }
        for v in [
            0usize,
            1,
            2,
            100,
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 2,
            1 << 40,
        ] {
            codes.push(v);
        }
        for c in codes {
            let s = CStr::from_ptr(nm(c)).to_string_lossy().into_owned();
            out.push((is(c), s, cd(c)));
        }
        out
    });
}

/* ================================================================== */
/* rows 73,74,75 — LZ4F_getBlockSize rejections                        */
/* ================================================================== */

#[test]
fn e073_getBlockSize_invalid() {
    diff("getBlockSize invalid ids", |lib| unsafe {
        let f = sym::<FnGetBlockSize>(lib, "LZ4F_getBlockSize");
        let is = sym::<FnIsError>(lib, "LZ4F_isError");
        let cd = sym::<FnErrCode>(lib, "LZ4F_getErrorCode");
        let mut out = Vec::new();
        for id in [
            i32::MIN,
            -7,
            -1,
            1,
            2,
            3,
            8,
            9,
            50,
            255,
            256,
            1 << 20,
            i32::MAX,
        ] {
            let r = f(id);
            out.push((r as i64, is(r), cd(r)));
        }
        // valid ones for contrast
        for id in [0i32, 4, 5, 6, 7] {
            let r = f(id);
            out.push((r as i64, is(r), cd(r)));
        }
        out
    });
    // the invalid ids must produce exactly ERROR_maxBlockSize_invalid
    let i = impls();
    for id in [-1i32, 1, 2, 3, 8, 255] {
        let f = unsafe { sym::<FnGetBlockSize>(&i.c, "LZ4F_getBlockSize") };
        let r = unsafe { f(id) } as i64;
        assert_eq!(r, err("ERROR_maxBlockSize_invalid"), "C code for id {id}");
    }
}

/* ================================================================== */
/* rows 76,77 — LZ4F_compressFrame rejections                          */
/* ================================================================== */

#[test]
fn e076_compressFrame_dst_too_small() {
    let mut rng = Rng::new(0x5EED_2076);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 1000, 70000].iter() {
            let src = mkdata(shape, len, &mut rng);
            for adj in [-1i64, -2, -100, -1000000, i64::from(i32::MIN)] {
                diff(
                    &format!("compressFrame small dst {shape:?} len={len} adj={adj}"),
                    |lib| compress_frame(lib, &src, None, adj),
                );
            }
        }
    }
    // exact error code
    let i = impls();
    let src = mkdata(Shape::Textish, 1000, &mut rng);
    let r = compress_frame(&i.c, &src, None, -1);
    assert_eq!(
        r.codes[1],
        err("ERROR_dstMaxSize_tooSmall"),
        "expected dstMaxSize_tooSmall"
    );
}

#[test]
fn e077_compressFrame_invalid_blockSizeID() {
    let mut rng = Rng::new(0x5EED_2077);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    for bsid in [i32::MIN, -1, 1, 2, 3, 8, 9, 255, 1 << 20, i32::MAX] {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.blockSizeID = bsid;
        diff(&format!("compressFrame bsid={bsid}"), |lib| unsafe {
            let bound = sym::<FnBound>(lib, "LZ4F_compressFrameBound")(src.len(), &p);
            // deliberately give an ample, fixed buffer so the failure is the
            // blockSizeID check and not a capacity check
            let mut d = vec![0u8; 1 << 20];
            let r = sym::<FnCompressFrame>(lib, "LZ4F_compressFrame")(
                d.as_mut_ptr(),
                d.len(),
                src.as_ptr(),
                src.len(),
                &p,
            );
            (bound as i64, r as i64, sym::<FnIsError>(lib, "LZ4F_isError")(r))
        });
    }
}

/* ================================================================== */
/* rows 79,80,81,93,94 — context lifecycle                             */
/* ================================================================== */

#[test]
fn e079_context_creation_and_free() {
    diff("ctx create/free", |lib| unsafe {
        let cc = sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext");
        let dc = sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext");
        let fc = sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext");
        let fd = sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext");
        let mut out: Vec<i64> = Vec::new();
        // every version value: the C ignores it
        for v in [0u32, 1, 99, 100, 101, 1000, u32::MAX] {
            let mut c: *mut CVoid = std::ptr::null_mut();
            let r = cc(&mut c, v);
            out.push(r as i64);
            out.push(c.is_null() as i64);
            out.push(fc(c) as i64);
            let mut d: *mut CVoid = std::ptr::null_mut();
            let r2 = dc(&mut d, v);
            out.push(r2 as i64);
            out.push(d.is_null() as i64);
            out.push(fd(d) as i64);
        }
        // free on NULL
        out.push(fc(std::ptr::null_mut()) as i64);
        out.push(fd(std::ptr::null_mut()) as i64);
        out
    });
    // NULL out-pointer: the C asserts then RETURN_ERROR(parameter_null) in
    // release builds (NDEBUG is not set by the CMake config, so assert() is
    // live — calling with NULL would abort). Only the documented, non-aborting
    // path is compared, matching row 81.
}

/* ================================================================== */
/* rows 82,122 — compressBegin capacity checks                         */
/* ================================================================== */

#[test]
fn e082_compressBegin_dst_too_small() {
    let mut rng = Rng::new(0x5EED_2082);
    let dict = mkdata(Shape::Textish, 4096, &mut rng);
    for cap in 0usize..=LZ4F_HEADER_SIZE_MAX + 2 {
        diff(&format!("compressBegin cap={cap}"), |lib| unsafe {
            let mut c: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
            let mut d = vec![0u8; LZ4F_HEADER_SIZE_MAX + 16];
            let a = sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                c,
                d.as_mut_ptr(),
                cap,
                std::ptr::null(),
            );
            let b = sym::<FnBeginDict>(lib, "LZ4F_compressBegin_usingDict")(
                c,
                d.as_mut_ptr(),
                cap,
                dict.as_ptr(),
                dict.len(),
                std::ptr::null(),
            );
            let e = sym::<FnBeginDict>(lib, "LZ4F_compressBegin_usingDictOnce")(
                c,
                d.as_mut_ptr(),
                cap,
                dict.as_ptr(),
                dict.len(),
                std::ptr::null(),
            );
            let cdict = sym::<FnCreateCDict>(lib, "LZ4F_createCDict")(dict.as_ptr(), dict.len());
            let g = sym::<FnBeginCDict>(lib, "LZ4F_compressBegin_usingCDict")(
                c,
                d.as_mut_ptr(),
                cap,
                cdict,
                std::ptr::null(),
            );
            sym::<FnFreeCDict>(lib, "LZ4F_freeCDict")(cdict);
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
            (a as i64, b as i64, e as i64, g as i64)
        });
    }
}

/* ================================================================== */
/* row 83 — compressBegin_internal dictSize > INT_MAX                   */
/* ================================================================== */

#[test]
fn e083_dictSize_too_large() {
    let mut rng = Rng::new(0x5EED_2083);
    let dict = mkdata(Shape::Textish, 4096, &mut rng);
    // The C checks `dictSize > INT_MAX` before ever dereferencing the buffer,
    // so an oversized length with a valid pointer is a real, safe input.
    for ds in [
        i32::MAX as usize,
        i32::MAX as usize + 1,
        u32::MAX as usize,
        usize::MAX,
    ] {
        diff(&format!("beginInternal dictSize={ds}"), |lib| unsafe {
            let mut c: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
            let mut d = vec![0u8; 64];
            let r = if ds <= i32::MAX as usize {
                // would actually read the (too short) dict — skip the real read
                // by reporting a sentinel instead.
                0usize.wrapping_sub(9999)
            } else {
                sym::<FnBeginInternal>(lib, "LZ4F_compressBegin_internal")(
                    c,
                    d.as_mut_ptr(),
                    d.len(),
                    dict.as_ptr(),
                    ds,
                    std::ptr::null(),
                    std::ptr::null(),
                )
            };
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
            (r as i64, sym::<FnIsError>(lib, "LZ4F_isError")(r))
        });
    }
    let i = impls();
    unsafe {
        let mut c: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(&i.c, "LZ4F_createCompressionContext")(&mut c, 100);
        let mut d = vec![0u8; 64];
        let r = sym::<FnBeginInternal>(&i.c, "LZ4F_compressBegin_internal")(
            c,
            d.as_mut_ptr(),
            d.len(),
            dict.as_ptr(),
            i32::MAX as usize + 1,
            std::ptr::null(),
            std::ptr::null(),
        );
        assert_eq!(r as i64, err("ERROR_parameter_invalid"));
        sym::<FnFreeCtx>(&i.c, "LZ4F_freeCompressionContext")(c);
    }
}

/* ================================================================== */
/* rows 84,85,86,87,88,89,90,91,92 — streaming state / capacity        */
/* ================================================================== */

#[test]
fn e084_update_without_begin() {
    let mut rng = Rng::new(0x5EED_2084);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    diff("update/flush/end before begin", |lib| unsafe {
        let mut c: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
        let mut d = vec![0u8; 1 << 20];
        let a = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            src.len(),
            std::ptr::null(),
        );
        let b = sym::<FnUpdate>(lib, "LZ4F_uncompressedUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            src.len(),
            std::ptr::null(),
        );
        let e = sym::<FnFlush>(lib, "LZ4F_flush")(c, d.as_mut_ptr(), d.len(), std::ptr::null());
        sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
        (a as i64, b as i64, e as i64)
    });
    diff("update/flush after end", |lib| unsafe {
        let mut c: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
        let mut d = vec![0u8; 1 << 20];
        sym::<FnBegin>(lib, "LZ4F_compressBegin")(c, d.as_mut_ptr(), d.len(), std::ptr::null());
        sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            src.len(),
            std::ptr::null(),
        );
        sym::<FnFlush>(lib, "LZ4F_compressEnd")(c, d.as_mut_ptr(), d.len(), std::ptr::null());
        let a = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            src.len(),
            std::ptr::null(),
        );
        let b = sym::<FnFlush>(lib, "LZ4F_flush")(c, d.as_mut_ptr(), d.len(), std::ptr::null());
        let e = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
            c,
            d.as_mut_ptr(),
            d.len(),
            std::ptr::null(),
        );
        sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
        (a as i64, b as i64, e as i64)
    });
    let i = impls();
    unsafe {
        let mut c: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(&i.c, "LZ4F_createCompressionContext")(&mut c, 100);
        let mut d = vec![0u8; 1 << 20];
        let r = sym::<FnUpdate>(&i.c, "LZ4F_compressUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            src.len(),
            std::ptr::null(),
        );
        assert_eq!(r as i64, err("ERROR_compressionState_uninitialized"));
        sym::<FnFreeCtx>(&i.c, "LZ4F_freeCompressionContext")(c);
    }
}

#[test]
fn e086_update_flush_end_capacity() {
    let mut rng = Rng::new(0x5EED_2086);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(5) {
        for &len in [1usize, 1000, 70000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            diff(
                &format!("update capacity prefs#{pi} len={len}"),
                |lib| unsafe {
                    let mut c: *mut CVoid = std::ptr::null_mut();
                    sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
                    let mut hdr = vec![0u8; 64];
                    sym::<FnBegin>(lib, "LZ4F_compressBegin")(c, hdr.as_mut_ptr(), hdr.len(), p);
                    let bound = sym::<FnBound>(lib, "LZ4F_compressBound")(len, p);
                    let mut d = vec![0u8; bound + 64];
                    let mut out: Vec<i64> = Vec::new();
                    for cap in [
                        0usize,
                        1,
                        4,
                        bound / 4,
                        bound / 2,
                        bound.saturating_sub(1),
                        bound,
                    ] {
                        let r = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
                            c,
                            d.as_mut_ptr(),
                            cap,
                            src.as_ptr(),
                            len,
                            std::ptr::null(),
                        );
                        out.push(r as i64);
                        if sym::<FnIsError>(lib, "LZ4F_isError")(r) != 0 {
                            // state is UB after an error: restart the frame
                            sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                                c,
                                hdr.as_mut_ptr(),
                                hdr.len(),
                                p,
                            );
                        }
                    }
                    // flush with insufficient capacity after buffering some data
                    sym::<FnBegin>(lib, "LZ4F_compressBegin")(c, hdr.as_mut_ptr(), hdr.len(), p);
                    sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
                        c,
                        d.as_mut_ptr(),
                        d.len(),
                        src.as_ptr(),
                        len,
                        std::ptr::null(),
                    );
                    for cap in [0usize, 1, 4, 8, 11, 100] {
                        let r = sym::<FnFlush>(lib, "LZ4F_flush")(
                            c,
                            d.as_mut_ptr(),
                            cap,
                            std::ptr::null(),
                        );
                        out.push(r as i64);
                    }
                    // compressEnd with insufficient capacity
                    for cap in [0usize, 1, 2, 3, 4, 7, 8] {
                        sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                            c,
                            hdr.as_mut_ptr(),
                            hdr.len(),
                            p,
                        );
                        let r = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
                            c,
                            d.as_mut_ptr(),
                            cap,
                            std::ptr::null(),
                        );
                        out.push(r as i64);
                    }
                    sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
                    out
                },
            );
        }
    }
}

#[test]
fn e092_compressEnd_frameSize_wrong() {
    let mut rng = Rng::new(0x5EED_2092);
    let src = mkdata(Shape::Textish, 10000, &mut rng);
    // declare a contentSize that does not match what is actually fed
    for declared in [1u64, 9999, 10001, 20000, u64::MAX] {
        for fed in [0usize, 1, 5000, 10000] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentSize = declared;
            diff(
                &format!("frameSize declared={declared} fed={fed}"),
                |lib| unsafe {
                    let mut c: *mut CVoid = std::ptr::null_mut();
                    sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut c, 100);
                    let mut d = vec![0u8; 1 << 20];
                    let h = sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                        c,
                        d.as_mut_ptr(),
                        d.len(),
                        &p,
                    );
                    let u = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
                        c,
                        d.as_mut_ptr(),
                        d.len(),
                        src.as_ptr(),
                        fed,
                        std::ptr::null(),
                    );
                    let e = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
                        c,
                        d.as_mut_ptr(),
                        d.len(),
                        std::ptr::null(),
                    );
                    sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(c);
                    (h as i64, u as i64, e as i64)
                },
            );
        }
    }
    let i = impls();
    unsafe {
        let mut p = LZ4F_preferences_t::default();
        p.frameInfo.contentSize = 12345;
        let mut c: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(&i.c, "LZ4F_createCompressionContext")(&mut c, 100);
        let mut d = vec![0u8; 1 << 20];
        sym::<FnBegin>(&i.c, "LZ4F_compressBegin")(c, d.as_mut_ptr(), d.len(), &p);
        sym::<FnUpdate>(&i.c, "LZ4F_compressUpdate")(
            c,
            d.as_mut_ptr(),
            d.len(),
            src.as_ptr(),
            100,
            std::ptr::null(),
        );
        let e = sym::<FnFlush>(&i.c, "LZ4F_compressEnd")(
            c,
            d.as_mut_ptr(),
            d.len(),
            std::ptr::null(),
        );
        assert_eq!(e as i64, err("ERROR_frameSize_wrong"));
        sym::<FnFreeCtx>(&i.c, "LZ4F_freeCompressionContext")(c);
    }
}

/* ================================================================== */
/* rows 95,96,97,98 — LZ4F_headerSize rejections                       */
/* ================================================================== */

#[test]
fn e095_headerSize_rejections() {
    let mut rng = Rng::new(0x5EED_2095);
    diff("headerSize NULL", |lib| unsafe {
        let f = sym::<FnHeaderSize>(lib, "LZ4F_headerSize");
        let mut out = Vec::new();
        for n in [0usize, 1, 4, 5, 7, 19, 1000] {
            out.push(f(std::ptr::null(), n) as i64);
        }
        out
    });
    let i = impls();
    unsafe {
        let f = sym::<FnHeaderSize>(&i.c, "LZ4F_headerSize");
        assert_eq!(f(std::ptr::null(), 100) as i64, err("ERROR_srcPtr_wrong"));
        let good = [0x04u8, 0x22, 0x4D, 0x18, 0x60, 0x40, 0x00];
        for n in 0..5usize {
            assert_eq!(
                f(good.as_ptr(), n) as i64,
                err("ERROR_frameHeader_incomplete"),
                "srcSize {n}"
            );
        }
        let bad = [0xFFu8, 0xFF, 0xFF, 0xFF, 0x60, 0x40, 0x00];
        assert_eq!(
            f(bad.as_ptr(), 7) as i64,
            err("ERROR_frameType_unknown")
        );
    }
    // fuzz: any 0..24 byte prefix
    for k in 0..2000 {
        let l = rng.range(0, 24);
        let mut b = mkdata(Shape::Random, l, &mut rng);
        // half the time start from a real magic so more paths are reached
        if k % 2 == 0 && l >= 4 {
            b[..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes());
        }
        diff(&format!("headerSize fuzz #{k} l={l}"), |lib| unsafe {
            let f = sym::<FnHeaderSize>(lib, "LZ4F_headerSize");
            (0..=l).map(|n| f(b.as_ptr(), n) as i64).collect::<Vec<i64>>()
        });
    }
}

/* ================================================================== */
/* rows 99–109 — header decode rejections via LZ4F_getFrameInfo         */
/* ================================================================== */

fn good_frame() -> Vec<u8> {
    let mut rng = Rng::new(0xABCD_0001);
    let src = mkdata(Shape::Textish, 20000, &mut rng);
    let i = impls();
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.contentChecksumFlag = 1;
    p.frameInfo.blockChecksumFlag = 1;
    p.frameInfo.contentSize = src.len() as u64;
    p.frameInfo.dictID = 0xC0FFEE;
    compress_frame(&i.c, &src, Some(&p), 0).frame
}

fn gfi(lib: &libloading::Library, buf: &[u8], n: usize) -> (i64, i64, LZ4F_frameInfo_t) {
    unsafe {
        let mut dctx: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
        let mut fi = LZ4F_frameInfo_t::default();
        let mut ss = n;
        let r = sym::<FnGetFrameInfo>(lib, "LZ4F_getFrameInfo")(
            dctx,
            &mut fi,
            buf.as_ptr(),
            &mut ss,
        );
        sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
        (r as i64, ss as i64, fi)
    }
}

#[test]
fn e099_header_decode_rejections() {
    let frame = good_frame();
    let hsize = 19usize; // magic + FLG + BD + 8 (contentSize) + 4 (dictID) + HC

    // row 99: truncated header
    diff("decodeHeader truncated", |lib| {
        (0..=hsize)
            .map(|n| {
                let (r, s, fi) = gfi(lib, &frame, n);
                (r, s, fi)
            })
            .collect::<Vec<_>>()
    });

    // row 100: bad magic
    diff("decodeHeader bad magic", |lib| {
        let mut out = Vec::new();
        for m in [
            0u32,
            1,
            0x184D2203,
            0x184D2205,
            0x184D2A4F,
            0x184D2A60,
            0xFFFFFFFF,
        ] {
            let mut f = frame.clone();
            f[..4].copy_from_slice(&m.to_le_bytes());
            out.push(gfi(lib, &f, f.len()));
        }
        out
    });

    // rows 101,102: FLG byte — every possible value
    diff("decodeHeader FLG sweep", |lib| {
        (0u16..256)
            .map(|v| {
                let mut f = frame.clone();
                f[4] = v as u8;
                gfi(lib, &f, f.len())
            })
            .collect::<Vec<_>>()
    });

    // rows 103,104,105: BD byte — every possible value
    diff("decodeHeader BD sweep", |lib| {
        (0u16..256)
            .map(|v| {
                let mut f = frame.clone();
                f[5] = v as u8;
                gfi(lib, &f, f.len())
            })
            .collect::<Vec<_>>()
    });

    // row 106: header checksum byte — every possible value
    diff("decodeHeader HC sweep", |lib| {
        (0u16..256)
            .map(|v| {
                let mut f = frame.clone();
                f[hsize - 1] = v as u8;
                gfi(lib, &f, f.len())
            })
            .collect::<Vec<_>>()
    });

    // exact error codes from the C
    let i = impls();
    let mut f = frame.clone();
    f[4] = 0x62; // set FLG reserved bit 1
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_reservedFlag_set"));
    let mut f = frame.clone();
    f[4] = 0x20 | (frame[4] & 0x1F); // version bits = 00
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_headerVersion_wrong"));
    let mut f = frame.clone();
    f[5] = 0x80; // BD reserved bit 7
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_reservedFlag_set"));
    let mut f = frame.clone();
    f[5] = 0x30; // blockSizeID 3
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_maxBlockSize_invalid"));
    let mut f = frame.clone();
    f[5] = 0x41; // low 4 bits non-zero
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_reservedFlag_set"));
    let mut f = frame.clone();
    f[hsize - 1] ^= 0xFF;
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_headerChecksum_invalid"));
    let mut f = frame.clone();
    f[..4].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(gfi(&i.c, &f, f.len()).0, err("ERROR_frameType_unknown"));
    assert_eq!(
        gfi(&i.c, &frame, 6).0,
        err("ERROR_frameHeader_incomplete")
    );
}

#[test]
fn e107_getFrameInfo_alreadyStarted() {
    let frame = good_frame();
    // Feed only part of the header so dStage == dstage_storeFrameHeader, then
    // call getFrameInfo: must be ERROR_frameDecoding_alreadyStarted.
    for cut in 1..=6usize {
        diff(&format!("gfi alreadyStarted cut={cut}"), |lib| unsafe {
            let mut dctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
            let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
            let mut o = vec![0u8; 1024];
            let mut dn = o.len();
            let mut sn = cut;
            let r1 = dec(
                dctx,
                o.as_mut_ptr(),
                &mut dn,
                frame.as_ptr(),
                &mut sn,
                std::ptr::null(),
            );
            let mut fi = LZ4F_frameInfo_t::default();
            let mut ss = frame.len();
            let r2 = sym::<FnGetFrameInfo>(lib, "LZ4F_getFrameInfo")(
                dctx,
                &mut fi,
                frame.as_ptr(),
                &mut ss,
            );
            sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
            (r1 as i64, dn as i64, sn as i64, r2 as i64, ss as i64, fi)
        });
    }
    let i = impls();
    unsafe {
        let mut dctx: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(&i.c, "LZ4F_createDecompressionContext")(&mut dctx, 100);
        let dec = sym::<FnDecompress>(&i.c, "LZ4F_decompress");
        let mut o = vec![0u8; 1024];
        let mut dn = o.len();
        let mut sn = 3usize;
        dec(
            dctx,
            o.as_mut_ptr(),
            &mut dn,
            frame.as_ptr(),
            &mut sn,
            std::ptr::null(),
        );
        let mut fi = LZ4F_frameInfo_t::default();
        let mut ss = frame.len();
        let r = sym::<FnGetFrameInfo>(&i.c, "LZ4F_getFrameInfo")(
            dctx,
            &mut fi,
            frame.as_ptr(),
            &mut ss,
        );
        assert_eq!(r as i64, err("ERROR_frameDecoding_alreadyStarted"));
        sym::<FnFreeCtx>(&i.c, "LZ4F_freeDecompressionContext")(dctx);
    }
}

#[test]
fn e109_getFrameInfo_null_src() {
    diff("gfi NULL src", |lib| unsafe {
        let mut dctx: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
        let mut out = Vec::new();
        for n in [0usize, 1, 7, 19, 100] {
            let mut fi = LZ4F_frameInfo_t::default();
            let mut ss = n;
            let r = sym::<FnGetFrameInfo>(lib, "LZ4F_getFrameInfo")(
                dctx,
                &mut fi,
                std::ptr::null(),
                &mut ss,
            );
            out.push((r as i64, ss as i64));
        }
        sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
        out
    });
}

/* ================================================================== */
/* rows 110–119 — LZ4F_decompress rejections                           */
/* ================================================================== */

#[test]
fn e111_block_size_too_large() {
    let mut rng = Rng::new(0x5EED_2111);
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let i = impls();
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = 4; // 64 KB max
    let frame = compress_frame(&i.c, &src, Some(&p), 0).frame;
    let hs = 7usize;
    for bad in [
        0x0001_0001u32,
        0x0002_0000,
        0x7FFF_FFFF,
        0x8001_0001,
        0xFFFF_FFFF,
    ] {
        let mut f = frame.clone();
        f[hs..hs + 4].copy_from_slice(&bad.to_le_bytes());
        for sc in [0usize, 1, 13] {
            diff(&format!("blockSize too large {bad:#x} sc={sc}"), |lib| {
                decompress_frame(lib, &f, src.len(), sc, 0, None, None, false)
            });
        }
    }
    let mut f = frame.clone();
    f[hs..hs + 4].copy_from_slice(&0x0002_0000u32.to_le_bytes());
    let d = decompress_frame(&i.c, &f, src.len(), 0, 0, None, None, false);
    assert!(
        d.codes.contains(&err("ERROR_maxBlockSize_invalid")),
        "expected maxBlockSize_invalid, got {:?}",
        d.codes
    );
}

#[test]
fn e112_block_checksum_invalid() {
    let mut rng = Rng::new(0x5EED_2112);
    let src = mkdata(Shape::Textish, 60000, &mut rng);
    let i = impls();
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = 4;
    p.frameInfo.blockChecksumFlag = 1;
    let frame = compress_frame(&i.c, &src, Some(&p), 0).frame;
    let hs = 7usize;
    let bsz = (u32::from_le_bytes(frame[hs..hs + 4].try_into().unwrap()) & 0x7FFF_FFFF) as usize;
    let crc_off = hs + 4 + bsz;
    for k in 0..8usize {
        let mut f = frame.clone();
        f[crc_off + (k % 4)] ^= 1 << (k / 4);
        for sc in [0usize, 1, 4096] {
            for &skip in [0u32, 1].iter() {
                let dopt = LZ4F_decompressOptions_t {
                    stableDst: 0,
                    skipChecksums: skip,
                    reserved1: 0,
                    reserved0: 0,
                };
                diff(
                    &format!("blockChecksum flip#{k} sc={sc} skip={skip}"),
                    |lib| {
                        decompress_frame(
                            lib,
                            &f,
                            src.len(),
                            sc,
                            0,
                            Some(&dopt),
                            None,
                            false,
                        )
                    },
                );
            }
        }
    }
    let mut f = frame.clone();
    f[crc_off] ^= 0xFF;
    let d = decompress_frame(&i.c, &f, src.len(), 0, 0, None, None, false);
    assert!(
        d.codes.contains(&err("ERROR_blockChecksum_invalid")),
        "expected blockChecksum_invalid, got {:?}",
        d.codes
    );
}

#[test]
fn e113_decompression_failed() {
    let mut rng = Rng::new(0x5EED_2113);
    let src = mkdata(Shape::Textish, 40000, &mut rng);
    let i = impls();
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = 4;
    let frame = compress_frame(&i.c, &src, Some(&p), 0).frame;
    let hs = 7usize;
    // corrupt bytes inside the first compressed block
    for k in 0..200usize {
        let mut f = frame.clone();
        let pos = hs + 4 + rng.below(f.len() - hs - 8);
        f[pos] = rng.byte();
        for sc in [0usize, 1] {
            diff(&format!("decompressionFailed #{k} sc={sc}"), |lib| {
                decompress_frame(lib, &f, src.len(), sc, 0, None, None, false)
            });
        }
    }
    // truncations at every offset
    for cut in 1..frame.len().min(40) {
        let f = &frame[..frame.len() - cut];
        diff(&format!("truncated frame cut={cut}"), |lib| {
            decompress_frame(lib, f, src.len(), 0, 0, None, None, false)
        });
    }
    // an uncompressed-block flag with an implausible size
    let mut f = frame.clone();
    let bsz = u32::from_le_bytes(f[hs..hs + 4].try_into().unwrap()) & 0x7FFF_FFFF;
    f[hs..hs + 4].copy_from_slice(&(bsz | 0x8000_0000).to_le_bytes());
    diff("uncompressed flag on compressed block", |lib| {
        decompress_frame(lib, &f, src.len(), 0, 0, None, None, false)
    });
}

#[test]
fn e115_e116_frameSize_and_contentChecksum() {
    let mut rng = Rng::new(0x5EED_2115);
    // 200 KB with a 64 KB block size => several blocks, so cutting after the
    // first one leaves frameRemainingSize != 0 at the EndMark.
    let src = mkdata(Shape::Textish, 200000, &mut rng);
    let i = impls();
    // row 115: declared contentSize larger than what the frame carries.
    // Build a valid frame declaring the true size, then splice in an EndMark
    // early so frameRemainingSize != 0 when the mark is seen.
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.contentSize = src.len() as u64;
    p.frameInfo.blockSizeID = 4;
    let frame = compress_frame(&i.c, &src, Some(&p), 0).frame;
    let hs = 15usize; // magic + FLG + BD + 8 contentSize + HC
    let bsz = (u32::from_le_bytes(frame[hs..hs + 4].try_into().unwrap()) & 0x7FFF_FFFF) as usize;
    let mut f = frame[..hs + 4 + bsz].to_vec();
    f.extend_from_slice(&0u32.to_le_bytes()); // EndMark
    for sc in [0usize, 1, 999] {
        diff(&format!("frameSize_wrong sc={sc}"), |lib| {
            decompress_frame(lib, &f, src.len(), sc, 0, None, None, false)
        });
    }
    let d = decompress_frame(&i.c, &f, src.len(), 0, 0, None, None, false);
    assert!(
        d.codes.contains(&err("ERROR_frameSize_wrong")),
        "expected frameSize_wrong, got {:?}",
        d.codes
    );

    // row 116: content checksum mismatch
    let mut p2 = LZ4F_preferences_t::default();
    p2.frameInfo.contentChecksumFlag = 1;
    let frame2 = compress_frame(&i.c, &src, Some(&p2), 0).frame;
    let n = frame2.len();
    for k in 0..8usize {
        let mut f2 = frame2.clone();
        f2[n - 4 + (k % 4)] ^= 1 << (k / 4);
        for &skip in [0u32, 1].iter() {
            let dopt = LZ4F_decompressOptions_t {
                stableDst: 0,
                skipChecksums: skip,
                reserved1: 0,
                reserved0: 0,
            };
            for sc in [0usize, 1] {
                diff(
                    &format!("contentChecksum flip#{k} skip={skip} sc={sc}"),
                    |lib| {
                        decompress_frame(
                            lib,
                            &f2,
                            src.len(),
                            sc,
                            0,
                            Some(&dopt),
                            None,
                            false,
                        )
                    },
                );
            }
        }
    }
    let mut f2 = frame2.clone();
    f2[n - 1] ^= 0xFF;
    let d2 = decompress_frame(&i.c, &f2, src.len(), 0, 0, None, None, false);
    assert!(
        d2.codes.contains(&err("ERROR_contentChecksum_invalid")),
        "expected contentChecksum_invalid, got {:?}",
        d2.codes
    );
}

#[test]
fn e117_e118_decompress_bad_src() {
    let mut rng = Rng::new(0x5EED_2117);
    // row 117: bad magic
    for m in [0u32, 1, 0x184D2205, 0xDEADBEEF] {
        let mut f = good_frame();
        f[..4].copy_from_slice(&m.to_le_bytes());
        for sc in [0usize, 1, 3, 7] {
            diff(&format!("decompress bad magic {m:#x} sc={sc}"), |lib| {
                decompress_frame(lib, &f, 20000, sc, 0, None, None, false)
            });
        }
    }
    // row 118: `LZ4F_decompress` (unlike `LZ4F_headerSize` /
    // `LZ4F_getFrameInfo`) has NO `src == NULL` guard: it computes
    // `srcEnd = src + *srcSizePtr` and dereferences immediately, so a NULL src
    // with a non-zero size faults in the C too and is not a testable
    // rejection. The defined case is NULL src with *srcSizePtr == 0, which
    // takes the "0-size input" shortcut and returns minFHSize.
    diff("decompress NULL src, size 0", |lib| unsafe {
        let mut dctx: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
        let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
        let mut o = vec![0u8; 1024];
        let mut out = Vec::new();
        for dcap in [0usize, 1, 1024] {
            let mut dn = dcap;
            let mut sn = 0usize;
            let r = dec(
                dctx,
                o.as_mut_ptr(),
                &mut dn,
                std::ptr::null(),
                &mut sn,
                std::ptr::null(),
            );
            out.push((r as i64, dn as i64, sn as i64));
            sym::<FnResetDctx>(lib, "LZ4F_resetDecompressionContext")(dctx);
        }
        // and a NULL dst with *dstSizePtr == 0, which the C explicitly allows
        let mut dn = 0usize;
        let mut sn = 0usize;
        let r = dec(
            dctx,
            std::ptr::null_mut(),
            &mut dn,
            std::ptr::null(),
            &mut sn,
            std::ptr::null(),
        );
        out.push((r as i64, dn as i64, sn as i64));
        sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
        out
    });
    // full-frame fuzz: random bytes that start with a valid magic
    for k in 0..1500usize {
        let l = rng.range(4, 120);
        let mut b = mkdata(Shape::Random, l, &mut rng);
        b[..4].copy_from_slice(&LZ4F_MAGICNUMBER.to_le_bytes());
        diff(&format!("decompress fuzz #{k} l={l}"), |lib| {
            decompress_frame(lib, &b, 4096, 0, 0, None, None, false)
        });
    }
}

#[test]
fn e119_decompress_usingDict_corrupt() {
    let mut rng = Rng::new(0x5EED_2119);
    let dict = mkdata(Shape::Textish, 8192, &mut rng);
    let src = mkdata(Shape::Textish, 30000, &mut rng);
    let i = impls();
    let plan = StreamPlan {
        begin: BeginMode::UsingDict(&dict),
        prefs: Some(LZ4F_preferences_t::default()),
        copts: None,
        steps: vec![(usize::MAX, UpdKind::Compressed, false)],
    };
    let frame = compress_stream(&i.c, &src, &plan).frame;
    assert!(!frame.is_empty());
    for k in 0..80usize {
        let mut f = frame.clone();
        let pos = 7 + rng.below(f.len() - 8);
        f[pos] = rng.byte();
        diff(&format!("usingDict corrupt #{k}"), |lib| {
            decompress_frame(lib, &f, src.len(), 0, 0, None, Some(&dict), false)
        });
    }
    // wrong dictionary entirely
    let other = mkdata(Shape::Random, 8192, &mut rng);
    for sc in [0usize, 1, 4096] {
        diff(&format!("usingDict wrong dict sc={sc}"), |lib| {
            decompress_frame(lib, &frame, src.len(), sc, 0, None, Some(&other), false)
        });
    }
    // empty dictionary
    let empty: Vec<u8> = Vec::new();
    diff("usingDict empty dict", |lib| {
        decompress_frame(lib, &frame, src.len(), 0, 0, None, Some(&empty), false)
    });
}

/* ================================================================== */
/* rows 120,121 — CDict lifecycle                                      */
/* ================================================================== */

#[test]
fn e120_cdict_edge_cases() {
    let mut rng = Rng::new(0x5EED_2120);
    let dict = mkdata(Shape::Textish, 4096, &mut rng);
    diff("CDict edge cases", |lib| unsafe {
        let mk = sym::<FnCreateCDict>(lib, "LZ4F_createCDict");
        let fr = sym::<FnFreeCDict>(lib, "LZ4F_freeCDict");
        let mut out: Vec<i64> = Vec::new();
        // dictSize 0 with a valid pointer
        let a = mk(dict.as_ptr(), 0);
        out.push(a.is_null() as i64);
        fr(a);
        // sizes across the 64 KB truncation boundary
        for ds in [1usize, 4, 64, 4095, 4096] {
            let p = mk(dict.as_ptr(), ds);
            out.push(p.is_null() as i64);
            fr(p);
        }
        // free on NULL must be a no-op
        fr(std::ptr::null_mut());
        out.push(0);
        out
    });
}

/* ================================================================== */
/* row 127 — flush bound for srcSize 0                                 */
/* ================================================================== */

#[test]
fn e127_compressBound_zero() {
    let prefs = pref_matrix();
    diff("compressBound(0)", |lib| unsafe {
        let cb = sym::<FnBound>(lib, "LZ4F_compressBound");
        let fb = sym::<FnBound>(lib, "LZ4F_compressFrameBound");
        let mut out: Vec<i64> = vec![
            cb(0, std::ptr::null()) as i64,
            fb(0, std::ptr::null()) as i64,
        ];
        for p in prefs.iter() {
            out.push(cb(0, p) as i64);
            out.push(fb(0, p) as i64);
        }
        out
    });
}

/* ================================================================== */
/* rows 157–163 — out-of-range enums / reserved fields over FFI         */
/* ================================================================== */

#[test]
fn e157_out_of_range_enums() {
    let mut rng = Rng::new(0x5EED_2157);
    let src = mkdata(Shape::Textish, 30000, &mut rng);
    // C enums accept any int: sweep values with no valid variant through every
    // field of LZ4F_frameInfo_t / LZ4F_preferences_t.
    let odd: [i32; 10] = [i32::MIN, -100, -2, -1, 2, 3, 8, 255, 1 << 20, i32::MAX];

    // A fixed, generous destination buffer so the comparison exercises the real
    // compression path even when the C's bound arithmetic overflows.
    let fixed = |lib: &libloading::Library, p: &LZ4F_preferences_t| unsafe {
        let mut d = vec![0u8; 1 << 21];
        let cap = d.len();
        let r = sym::<FnCompressFrame>(lib, "LZ4F_compressFrame")(
            d.as_mut_ptr(),
            cap,
            src.as_ptr(),
            src.len(),
            p,
        );
        let ok = sym::<FnIsError>(lib, "LZ4F_isError")(r) == 0;
        let bound = sym::<FnBound>(lib, "LZ4F_compressFrameBound")(src.len(), p) as i64;
        (
            bound,
            r as i64,
            ok,
            if ok { d[..r].to_vec() } else { Vec::new() },
        )
    };

    for &v in odd.iter() {
        for field in 0..5usize {
            let mut p = LZ4F_preferences_t::default();
            match field {
                0 => p.frameInfo.blockMode = v,
                1 => p.frameInfo.contentChecksumFlag = v,
                2 => p.frameInfo.blockChecksumFlag = v,
                3 => p.frameInfo.frameType = v,
                _ => p.frameInfo.blockSizeID = v,
            }
            diff(&format!("enum field={field} v={v} fixed"), |lib| fixed(lib, &p));
            diff(&format!("enum field={field} v={v} bound"), |lib| {
                compress_frame(lib, &src, Some(&p), 0)
            });
        }
        // and the non-enum knobs
        let mut p = LZ4F_preferences_t::default();
        p.compressionLevel = v;
        diff(&format!("level={v}"), |lib| fixed(lib, &p));
        let mut p = LZ4F_preferences_t::default();
        p.autoFlush = v as u32;
        diff(&format!("autoFlush={v}"), |lib| fixed(lib, &p));
        let mut p = LZ4F_preferences_t::default();
        p.favorDecSpeed = v as u32;
        p.compressionLevel = 12;
        diff(&format!("favorDecSpeed={v}"), |lib| fixed(lib, &p));
    }
    // full random preference structs, including nonsense in every field
    for k in 0..400usize {
        let p = LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t {
                blockSizeID: [0i32, 4, 5, 6, 7, -1, 3, 8, 99][rng.below(9)],
                blockMode: rng.next_u32() as i32,
                contentChecksumFlag: rng.next_u32() as i32,
                frameType: rng.next_u32() as i32,
                contentSize: if k % 3 == 0 { src.len() as u64 } else { rng.next_u64() },
                dictID: rng.next_u32(),
                blockChecksumFlag: rng.next_u32() as i32,
            },
            compressionLevel: (rng.next_u32() % 40) as i32 - 10,
            autoFlush: rng.next_u32(),
            favorDecSpeed: rng.next_u32(),
            reserved: [rng.next_u32(), rng.next_u32(), rng.next_u32()],
        };
        diff(&format!("random prefs #{k}"), |lib| fixed(lib, &p));
    }
}

#[test]
fn e161_reserved_fields_ignored() {
    let mut rng = Rng::new(0x5EED_2161);
    let src = mkdata(Shape::Textish, 40000, &mut rng);
    // The C never reads prefs.reserved / cOpt.reserved / dOpt.reserved*, so
    // garbage there must not change any output.
    let base = LZ4F_preferences_t::default();
    let mut poisoned = base;
    poisoned.reserved = [0xDEAD_BEEF, 0xFFFF_FFFF, 1];
    let i = impls();
    let a = compress_frame(&i.c, &src, Some(&base), 0).frame;
    let b = compress_frame(&i.c, &src, Some(&poisoned), 0).frame;
    assert_eq!(a, b, "C: reserved must be ignored");
    diff("prefs reserved poisoned", |lib| {
        compress_frame(lib, &src, Some(&poisoned), 0)
    });

    for stable in [0u32, 1, 2, 0xFFFF_FFFF] {
        let copts = LZ4F_compressOptions_t {
            stableSrc: stable,
            reserved: [0xAAAA_AAAA, 0x5555_5555, 7],
        };
        let plan = StreamPlan {
            begin: BeginMode::Plain,
            prefs: Some(base),
            copts: Some(copts),
            steps: vec![(7000, UpdKind::Compressed, false); 10],
        };
        diff(&format!("copts poisoned stable={stable}"), |lib| {
            compress_stream(lib, &src, &plan)
        });
    }

    let frame = compress_frame(&i.c, &src, Some(&base), 0).frame;
    for sd in [0u32, 1, 2, 0xFFFF_FFFF] {
        for sk in [0u32, 1, 2, 0xFFFF_FFFF] {
            let d = LZ4F_decompressOptions_t {
                stableDst: sd,
                skipChecksums: sk,
                reserved1: 0xDEAD_BEEF,
                reserved0: 0xFEED_FACE,
            };
            diff(&format!("dopts poisoned sd={sd} sk={sk}"), |lib| {
                decompress_frame(lib, &frame, src.len(), 4096, 0, Some(&d), None, false)
            });
        }
    }
}

/* ================================================================== */
/* row 164 — version constants                                         */
/* ================================================================== */

#[test]
fn e164_version_constants() {
    diff("version constants", |lib| unsafe {
        let vs = sym::<unsafe extern "C" fn() -> *const CChar>(lib, "LZ4_versionString");
        (
            sym::<unsafe extern "C" fn() -> u32>(lib, "LZ4F_getVersion")(),
            sym::<FnVoidI32>(lib, "LZ4_versionNumber")(),
            CStr::from_ptr(vs()).to_string_lossy().into_owned(),
            sym::<FnVoidI32>(lib, "LZ4F_compressionLevel_max")(),
            sym::<unsafe extern "C" fn() -> u32>(lib, "LZ4_XXH_versionNumber")(),
        )
    });
}
