//! Phase B — LZ4 frame API valid paths (`lz4frame.c`).
//! CONFIGS.md rows 57–95.
#![allow(non_snake_case)]

mod common;
use common::frame::*;
use common::*;

/* ================================================================== */
/* row 57 — LZ4F_getBlockSize                                          */
/* ================================================================== */

#[test]
fn r057_getBlockSize() {
    let cases: Vec<i32> = vec![
        i32::MIN,
        -99,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        99,
        255,
        1 << 30,
        i32::MAX,
    ];
    diff("LZ4F_getBlockSize", |lib| {
        let f = unsafe { sym::<FnGetBlockSize>(lib, "LZ4F_getBlockSize") };
        cases
            .iter()
            .map(|&c| unsafe { f(c) } as i64)
            .collect::<Vec<i64>>()
    });
}

/* ================================================================== */
/* rows 58,59 — bound functions across the whole option matrix          */
/* ================================================================== */

#[test]
fn r058_r059_bounds() {
    let prefs = pref_matrix();
    let sizes: Vec<usize> = vec![
        0, 1, 2, 15, 16, 17, 63, 64, 65535, 65536, 65537, 262143, 262144, 262145, 1048575, 1048576,
        1048577, 4194303, 4194304, 4194305, 10_000_000,
    ];
    diff("compressFrameBound/compressBound", |lib| {
        let fb = unsafe { sym::<FnBound>(lib, "LZ4F_compressFrameBound") };
        let cb = unsafe { sym::<FnBound>(lib, "LZ4F_compressBound") };
        let mut out: Vec<i64> = Vec::new();
        for s in sizes.iter() {
            out.push(unsafe { fb(*s, std::ptr::null()) } as i64);
            out.push(unsafe { cb(*s, std::ptr::null()) } as i64);
            for p in prefs.iter() {
                out.push(unsafe { fb(*s, p) } as i64);
                out.push(unsafe { cb(*s, p) } as i64);
            }
        }
        out
    });
    // invalid blockSizeID feeds an error code into the bound arithmetic —
    // the result must still match exactly (CONFIGS row 58 / ERRORS row 78).
    diff("bounds with invalid blockSizeID", |lib| {
        let fb = unsafe { sym::<FnBound>(lib, "LZ4F_compressFrameBound") };
        let cb = unsafe { sym::<FnBound>(lib, "LZ4F_compressBound") };
        let mut out: Vec<i64> = Vec::new();
        for bsid in [-1i32, 1, 2, 3, 8, 99, 255, 1 << 30] {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = bsid;
            for &s in [0usize, 1, 1000, 100000].iter() {
                out.push(unsafe { fb(s, &p) } as i64);
                out.push(unsafe { cb(s, &p) } as i64);
            }
        }
        out
    });
}

#[test]
fn r095_version_and_level_max() {
    diff("frame version/level", |lib| unsafe {
        (
            sym::<unsafe extern "C" fn() -> u32>(lib, "LZ4F_getVersion")(),
            sym::<FnVoidI32>(lib, "LZ4F_compressionLevel_max")(),
        )
    });
}

/* ================================================================== */
/* rows 60–65 — LZ4F_compressFrame over the option matrix               */
/* ================================================================== */

#[test]
fn r060_compressFrame_default_prefs() {
    let mut rng = Rng::new(0x5EED_0060);
    for &shape in ALL_SHAPES.iter() {
        for &len in [
            0usize, 1, 2, 15, 16, 100, 65535, 65536, 65537, 262145, 300000,
        ]
        .iter()
        {
            let src = mkdata(shape, len, &mut rng);
            diff(&format!("compressFrame NULL prefs {shape:?} len={len}"), |lib| {
                compress_frame(lib, &src, None, 0)
            });
        }
    }
}

#[test]
fn r061_r065_compressFrame_matrix() {
    let mut rng = Rng::new(0x5EED_0061);
    let prefs = pref_matrix();
    // sizes chosen to straddle every block-size boundary
    let sizes = [0usize, 1, 100, 65535, 65536, 65537, 130000, 300000];
    for (pi, p) in prefs.iter().enumerate() {
        for &shape in [Shape::Random, Shape::Textish, Shape::Constant].iter() {
            for &len in sizes.iter() {
                let src = mkdata(shape, len, &mut rng);
                diff(
                    &format!("compressFrame prefs#{pi} {shape:?} len={len}"),
                    |lib| compress_frame(lib, &src, Some(p), 0),
                );
            }
        }
    }
}

#[test]
fn r063_contentSize_and_dictID() {
    let mut rng = Rng::new(0x5EED_0063);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 1000, 65536, 200000].iter() {
            let src = mkdata(shape, len, &mut rng);
            for &(cs, did) in [
                (0u64, 0u32),
                (len as u64, 0),
                (0, 0xDEAD_BEEF),
                (len as u64, 0xDEAD_BEEF),
                (len as u64, 1),
                (u64::MAX, 0xFFFF_FFFF),
            ]
            .iter()
            {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentSize = cs;
                p.frameInfo.dictID = did;
                p.frameInfo.contentChecksumFlag = 1;
                diff(
                    &format!("compressFrame cs={cs} did={did} {shape:?} len={len}"),
                    |lib| compress_frame(lib, &src, Some(&p), 0),
                );
            }
        }
    }
}

#[test]
fn r061_random_sizes() {
    let mut rng = Rng::new(0x5EED_0611);
    let prefs = pref_matrix();
    for i in 0..400 {
        let p = &prefs[i % prefs.len()];
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(0, 200000);
        let src = mkdata(shape, len, &mut rng);
        diff(&format!("compressFrame rand #{i} len={len}"), |lib| {
            compress_frame(lib, &src, Some(p), 0)
        });
    }
}

/* ================================================================== */
/* rows 84–91 — decompression over the option matrix                    */
/* ================================================================== */

fn make_frame(src: &[u8], p: Option<&LZ4F_preferences_t>) -> Vec<u8> {
    let i = impls();
    let r = compress_frame(&i.c, src, p, 0);
    assert!(!r.frame.is_empty() || src.is_empty(), "C frame build failed");
    r.frame
}

#[test]
fn r084_decompress_whole_frame() {
    let mut rng = Rng::new(0x5EED_0084);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate() {
        for &shape in [Shape::Random, Shape::Textish].iter() {
            for &len in [0usize, 1, 100, 65536, 130000].iter() {
                let src = mkdata(shape, len, &mut rng);
                let frame = make_frame(&src, Some(p));
                diff(
                    &format!("decompress whole prefs#{pi} {shape:?} len={len}"),
                    |lib| decompress_frame(lib, &frame, len, 0, 0, None, None, false),
                );
                // verify correctness, not just agreement
                let i = impls();
                let d = decompress_frame(&i.r, &frame, len, 0, 0, None, None, false);
                assert_eq!(&d.out[..], &src[..], "roundtrip prefs#{pi} len={len}");
            }
        }
    }
}

#[test]
fn r085_decompress_byte_at_a_time() {
    let mut rng = Rng::new(0x5EED_0085);
    // exercises every dstage_store* branch in the state machine
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(3) {
        for &shape in [Shape::Textish, Shape::Random].iter() {
            for &len in [0usize, 1, 300, 70000].iter() {
                let src = mkdata(shape, len, &mut rng);
                let frame = make_frame(&src, Some(p));
                diff(
                    &format!("decompress 1by1 prefs#{pi} {shape:?} len={len}"),
                    |lib| decompress_frame(lib, &frame, len, 1, 0, None, None, false),
                );
                diff(
                    &format!("decompress src1 dst1 prefs#{pi} {shape:?} len={len}"),
                    |lib| decompress_frame(lib, &frame, len, 1, 1, None, None, false),
                );
            }
        }
    }
}

#[test]
fn r086_r087_decompress_random_chunks() {
    let mut rng = Rng::new(0x5EED_0086);
    let prefs = pref_matrix();
    for i in 0..300 {
        let p = &prefs[i % prefs.len()];
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(0, 150000);
        let src = mkdata(shape, len, &mut rng);
        let frame = make_frame(&src, Some(p));
        let sc = rng.range(1, 5000);
        let dc = rng.range(1, 5000);
        diff(
            &format!("decompress chunks #{i} len={len} sc={sc} dc={dc}"),
            |lib| decompress_frame(lib, &frame, len, sc, dc, None, None, false),
        );
        let d = decompress_frame(&impls().r, &frame, len, sc, dc, None, None, false);
        assert_eq!(&d.out[..], &src[..], "chunked roundtrip #{i}");
    }
    // dstCapacity deliberately smaller than a block: forces the tmpOut path
    for &bsid in BLOCK_SIZE_IDS.iter() {
        for &bmode in [0i32, 1].iter() {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.blockSizeID = bsid;
            p.frameInfo.blockMode = bmode;
            let src = mkdata(Shape::Textish, 200000, &mut rng);
            let frame = make_frame(&src, Some(&p));
            for dc in [1usize, 7, 100, 1000, 65535] {
                diff(
                    &format!("decompress tiny dst bsid={bsid} bm={bmode} dc={dc}"),
                    |lib| decompress_frame(lib, &frame, src.len(), 0, dc, None, None, false),
                );
            }
        }
    }
}

#[test]
fn r088_r089_decompress_options() {
    let mut rng = Rng::new(0x5EED_0088);
    for &cc in [0i32, 1].iter() {
        for &bc in [0i32, 1].iter() {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentChecksumFlag = cc;
            p.frameInfo.blockChecksumFlag = bc;
            let src = mkdata(Shape::Textish, 120000, &mut rng);
            let frame = make_frame(&src, Some(&p));
            for &sd in [0u32, 1, 2, 0xFFFF_FFFF].iter() {
                for &sk in [0u32, 1, 2, 0xFFFF_FFFF].iter() {
                    let d = LZ4F_decompressOptions_t {
                        stableDst: sd,
                        skipChecksums: sk,
                        reserved1: 0xAAAA_AAAA,
                        reserved0: 0x5555_5555,
                    };
                    for sc in [0usize, 1, 4096] {
                        diff(
                            &format!("dopts cc={cc} bc={bc} sd={sd} sk={sk} sc={sc}"),
                            |lib| {
                                decompress_frame(
                                    lib,
                                    &frame,
                                    src.len(),
                                    sc,
                                    0,
                                    Some(&d),
                                    None,
                                    false,
                                )
                            },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn r090_skippable_frames() {
    let mut rng = Rng::new(0x5EED_0090);
    for variant in 0u32..16 {
        for &payload in [0usize, 1, 7, 100, 70000].iter() {
            let mut f = Vec::new();
            f.extend_from_slice(&(LZ4F_MAGIC_SKIPPABLE_START + variant).to_le_bytes());
            f.extend_from_slice(&(payload as u32).to_le_bytes());
            let body = mkdata(Shape::Random, payload, &mut rng);
            f.extend_from_slice(&body);
            // and append a real frame so the dctx must return to a clean state
            let src = mkdata(Shape::Textish, 5000, &mut rng);
            let real = make_frame(&src, None);
            let mut both = f.clone();
            both.extend_from_slice(&real);
            for sc in [0usize, 1, 13, 4096] {
                diff(
                    &format!("skippable v={variant} payload={payload} sc={sc}"),
                    |lib| decompress_frame(lib, &f, 16, sc, 0, None, None, false),
                );
                diff(
                    &format!("skippable+real v={variant} payload={payload} sc={sc}"),
                    |lib| decompress_frame(lib, &both, src.len() + 16, sc, 0, None, None, false),
                );
            }
        }
    }
}

#[test]
fn r091_concatenated_frames() {
    let mut rng = Rng::new(0x5EED_0091);
    let prefs = pref_matrix();
    for k in 0..24usize {
        let mut all = Vec::new();
        let mut total = 0usize;
        for j in 0..4 {
            let p = &prefs[(k * 4 + j) % prefs.len()];
            let len = rng.range(0, 40000);
            let src = mkdata(ALL_SHAPES[j % ALL_SHAPES.len()], len, &mut rng);
            all.extend_from_slice(&make_frame(&src, Some(p)));
            total += len;
        }
        for sc in [0usize, 1, 777] {
            diff(&format!("concat #{k} sc={sc}"), |lib| {
                decompress_frame(lib, &all, total, sc, 0, None, None, false)
            });
        }
    }
}

/* ================================================================== */
/* rows 81,82,83 — headerSize / getFrameInfo                            */
/* ================================================================== */

#[test]
fn r081_headerSize() {
    let mut rng = Rng::new(0x5EED_0081);
    // every FLG contentSize/dictID flag combination
    let mut frames = Vec::new();
    for &cs in [0u64, 12345].iter() {
        for &did in [0u32, 7].iter() {
            let mut p = LZ4F_preferences_t::default();
            p.frameInfo.contentSize = cs;
            p.frameInfo.dictID = did;
            let src = mkdata(Shape::Textish, 12345, &mut rng);
            frames.push(make_frame(&src, Some(&p)));
        }
    }
    // skippable magics
    for v in 0u32..16 {
        let mut f = (LZ4F_MAGIC_SKIPPABLE_START + v).to_le_bytes().to_vec();
        f.extend_from_slice(&[0u8; 16]);
        frames.push(f);
    }
    // arbitrary junk
    for _ in 0..64 {
        let l = rng.range(0, 32);
        frames.push(mkdata(Shape::Random, l, &mut rng));
    }
    diff("LZ4F_headerSize", |lib| {
        let f = unsafe { sym::<FnHeaderSize>(lib, "LZ4F_headerSize") };
        let mut out: Vec<i64> = Vec::new();
        for fr in frames.iter() {
            for n in 0..=fr.len().min(24) {
                out.push(unsafe { f(fr.as_ptr(), n) } as i64);
            }
        }
        out
    });
}

#[test]
fn r082_r083_getFrameInfo() {
    let mut rng = Rng::new(0x5EED_0082);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(2) {
        let len = 30000usize;
        let src = mkdata(Shape::Textish, len, &mut rng);
        let frame = make_frame(&src, Some(p));
        // getFrameInfo then decompress the rest
        diff(&format!("getFrameInfo first prefs#{pi}"), |lib| {
            decompress_frame(lib, &frame, len, 0, 0, None, None, true)
        });
        // getFrameInfo with a truncated header, then again with more data
        diff(&format!("getFrameInfo partial prefs#{pi}"), |lib| unsafe {
            let mut dctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
            let gfi = sym::<FnGetFrameInfo>(lib, "LZ4F_getFrameInfo");
            let mut out: Vec<i64> = Vec::new();
            for n in 0..=frame.len().min(24) {
                let mut fi = LZ4F_frameInfo_t::default();
                let mut ss = n;
                let r = gfi(dctx, &mut fi, frame.as_ptr(), &mut ss);
                out.push(r as i64);
                out.push(ss as i64);
                out.push(fi.blockSizeID as i64);
                out.push(fi.contentSize as i64);
                out.push(fi.dictID as i64);
                sym::<FnResetDctx>(lib, "LZ4F_resetDecompressionContext")(dctx);
            }
            sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
            out
        });
        // getFrameInfo *after* decoding has started (dStage > storeFrameHeader)
        diff(&format!("getFrameInfo mid prefs#{pi}"), |lib| unsafe {
            let mut dctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
            let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
            let mut out = vec![0u8; len + 1024];
            let mut dn = 100usize;
            let mut sn = frame.len().min(40);
            let r1 = dec(
                dctx,
                out.as_mut_ptr(),
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
            (
                r1 as i64,
                dn as i64,
                sn as i64,
                r2 as i64,
                ss as i64,
                fi,
            )
        });
    }
}

/* ================================================================== */
/* rows 67–73, 70a–70c — streaming compression                          */
/* ================================================================== */

#[test]
fn r067_streaming_single_update() {
    let mut rng = Rng::new(0x5EED_0067);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate() {
        for &len in [0usize, 1, 100, 65536, 130000].iter() {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let plan = StreamPlan {
                begin: BeginMode::Plain,
                prefs: Some(*p),
                copts: None,
                steps: vec![(usize::MAX, UpdKind::Compressed, false)],
            };
            diff(&format!("stream single prefs#{pi} len={len}"), |lib| {
                compress_stream(lib, &src, &plan)
            });
        }
    }
}

#[test]
fn r068_r069_streaming_many_updates() {
    let mut rng = Rng::new(0x5EED_0068);
    let prefs = pref_matrix();
    for i in 0..200 {
        let p = &prefs[i % prefs.len()];
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(0, 200000);
        let src = mkdata(shape, len, &mut rng);
        let nsteps = rng.range(1, 40);
        let steps: Vec<(usize, UpdKind, bool)> = (0..nsteps)
            .map(|_| (rng.range(0, 40000), UpdKind::Compressed, false))
            .collect();
        let plan = StreamPlan {
            begin: BeginMode::Plain,
            prefs: Some(*p),
            copts: None,
            steps,
        };
        diff(&format!("stream many #{i} len={len}"), |lib| {
            compress_stream(lib, &src, &plan)
        });
    }
    // deterministic 1-byte-at-a-time over the whole matrix (subsampled)
    for (pi, p) in prefs.iter().enumerate().step_by(7) {
        let src = mkdata(Shape::Textish, 3000, &mut rng);
        let plan = StreamPlan {
            begin: BeginMode::Plain,
            prefs: Some(*p),
            copts: None,
            steps: vec![(1, UpdKind::Compressed, false); 3000],
        };
        diff(&format!("stream 1by1 prefs#{pi}"), |lib| {
            compress_stream(lib, &src, &plan)
        });
    }
}

#[test]
fn r070_streaming_with_flush() {
    let mut rng = Rng::new(0x5EED_0070);
    let prefs = pref_matrix();
    for (pi, p) in prefs.iter().enumerate().step_by(2) {
        for &(total, chunk) in [
            (3000usize, 1usize),
            (150000, 100),
            (150000, 30000),
            (150000, 70000),
        ]
        .iter()
        {
            let src = mkdata(Shape::Textish, total, &mut rng);
            let steps: Vec<(usize, UpdKind, bool)> = (0..(src.len() / chunk + 2))
                .map(|_| (chunk, UpdKind::Compressed, true))
                .collect();
            let plan = StreamPlan {
                begin: BeginMode::Plain,
                prefs: Some(*p),
                copts: None,
                steps,
            };
            diff(
                &format!("stream flush prefs#{pi} total={total} chunk={chunk}"),
                |lib| compress_stream(lib, &src, &plan),
            );
        }
    }
    // flush with nothing buffered (tmpInSize == 0)
    diff("flush empty", |lib| unsafe {
        let mut cctx: *mut CVoid = std::ptr::null_mut();
        sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut cctx, 100);
        let mut hdr = vec![0u8; 64];
        let hr = sym::<FnBegin>(lib, "LZ4F_compressBegin")(
            cctx,
            hdr.as_mut_ptr(),
            hdr.len(),
            std::ptr::null(),
        );
        let mut d = vec![0u8; 256];
        let a = sym::<FnFlush>(lib, "LZ4F_flush")(cctx, d.as_mut_ptr(), d.len(), std::ptr::null());
        let b = sym::<FnFlush>(lib, "LZ4F_flush")(cctx, d.as_mut_ptr(), 0, std::ptr::null());
        let e = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
            cctx,
            d.as_mut_ptr(),
            d.len(),
            std::ptr::null(),
        );
        sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
        (hr as i64, a as i64, b as i64, e as i64, d[..8].to_vec())
    });
}

#[test]
fn r071_r072_autoflush_and_stableSrc() {
    let mut rng = Rng::new(0x5EED_0071);
    for &af in [0u32, 1].iter() {
        for &bmode in [0i32, 1].iter() {
            for &bsid in BLOCK_SIZE_IDS.iter() {
                for &stable in [0u32, 1, 2].iter() {
                    let mut p = LZ4F_preferences_t::default();
                    p.autoFlush = af;
                    p.frameInfo.blockMode = bmode;
                    p.frameInfo.blockSizeID = bsid;
                    let copts = LZ4F_compressOptions_t {
                        stableSrc: stable,
                        reserved: [0xDEAD_BEEF, 0, 1],
                    };
                    let src = mkdata(Shape::Textish, 150000, &mut rng);
                    for chunk in [1usize, 333, 40000] {
                        let steps: Vec<(usize, UpdKind, bool)> = (0..(src.len() / chunk + 2))
                            .map(|_| (chunk, UpdKind::Compressed, false))
                            .collect();
                        let plan = StreamPlan {
                            begin: BeginMode::Plain,
                            prefs: Some(p),
                            copts: Some(copts),
                            steps,
                        };
                        diff(
                            &format!("af={af} bm={bmode} bsid={bsid} stable={stable} chunk={chunk}"),
                            |lib| compress_stream(lib, &src, &plan),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn r073_cctx_reuse_switches_ctx_type() {
    let mut rng = Rng::new(0x5EED_0073);
    // Reuse one cctx across frames whose compressionLevel crosses
    // LZ4HC_CLEVEL_MIN, forcing the fast<->HC internal context switch, and
    // whose blockSizeID grows, forcing a tmpBuff realloc.
    let seqs: Vec<Vec<(i32, i32, u32)>> = vec![
        vec![(0, 4, 0), (9, 7, 0), (0, 5, 1), (12, 6, 0)],
        vec![(12, 7, 0), (0, 4, 1), (10, 4, 0), (1, 7, 1)],
        vec![(9, 4, 0), (9, 7, 1), (0, 7, 0), (0, 4, 0)],
    ];
    for (si, seq) in seqs.iter().enumerate() {
        let srcs: Vec<Vec<u8>> = (0..seq.len())
            .map(|_| mkdata(Shape::Textish, rng.range(1000, 200000), &mut rng))
            .collect();
        diff(&format!("cctx reuse seq#{si}"), |lib| unsafe {
            let mut cctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut cctx, 100);
            let mut out: Vec<i64> = Vec::new();
            let mut bytes: Vec<u8> = Vec::new();
            for (k, &(lvl, bsid, af)) in seq.iter().enumerate() {
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = lvl;
                p.frameInfo.blockSizeID = bsid;
                p.autoFlush = af;
                p.frameInfo.contentChecksumFlag = 1;
                let mut hdr = vec![0u8; 64];
                let hr = sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                    cctx,
                    hdr.as_mut_ptr(),
                    hdr.len(),
                    &p,
                );
                out.push(hr as i64);
                bytes.extend_from_slice(&hdr[..hr]);
                let src = &srcs[k];
                let bound = sym::<FnBound>(lib, "LZ4F_compressBound")(src.len(), &p);
                let mut d = vec![0u8; bound + 64];
                let r = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
                    cctx,
                    d.as_mut_ptr(),
                    d.len(),
                    src.as_ptr(),
                    src.len(),
                    std::ptr::null(),
                );
                out.push(r as i64);
                bytes.extend_from_slice(&d[..r]);
                let ecap = sym::<FnBound>(lib, "LZ4F_compressBound")(0, &p) + 64;
                let mut e = vec![0u8; ecap];
                let er = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
                    cctx,
                    e.as_mut_ptr(),
                    ecap,
                    std::ptr::null(),
                );
                out.push(er as i64);
                bytes.extend_from_slice(&e[..er]);
            }
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
            (out, bytes)
        });
    }
}

#[test]
fn r070a_r070c_uncompressedUpdate() {
    let mut rng = Rng::new(0x5EED_070A);
    for &bmode in [0i32, 1].iter() {
        for &bsid in BLOCK_SIZE_IDS.iter() {
            for &cc in [0i32, 1].iter() {
                for &bc in [0i32, 1].iter() {
                    let mut p = LZ4F_preferences_t::default();
                    p.frameInfo.blockMode = bmode;
                    p.frameInfo.blockSizeID = bsid;
                    p.frameInfo.contentChecksumFlag = cc;
                    p.frameInfo.blockChecksumFlag = bc;
                    let src = mkdata(Shape::Textish, 120000, &mut rng);
                    for chunk in [1usize, 500, 40000] {
                        let n = src.len() / chunk + 2;
                        // all-uncompressed
                        let plan = StreamPlan {
                            begin: BeginMode::Plain,
                            prefs: Some(p),
                            copts: None,
                            steps: vec![(chunk, UpdKind::Uncompressed, false); n],
                        };
                        diff(
                            &format!("uncompUpd bm={bmode} bsid={bsid} cc={cc} bc={bc} c={chunk}"),
                            |lib| compress_stream(lib, &src, &plan),
                        );
                        // interleaved compressed/uncompressed
                        let steps: Vec<(usize, UpdKind, bool)> = (0..n)
                            .map(|k| {
                                (
                                    chunk,
                                    if k % 2 == 0 {
                                        UpdKind::Compressed
                                    } else {
                                        UpdKind::Uncompressed
                                    },
                                    k % 3 == 2,
                                )
                            })
                            .collect();
                        let plan2 = StreamPlan {
                            begin: BeginMode::Plain,
                            prefs: Some(p),
                            copts: None,
                            steps,
                        };
                        diff(
                            &format!("mixedUpd bm={bmode} bsid={bsid} cc={cc} bc={bc} c={chunk}"),
                            |lib| compress_stream(lib, &src, &plan2),
                        );
                    }
                }
            }
        }
    }
}

/* ================================================================== */
/* rows 66,74,75,76,77,92 — dictionaries                                */
/* ================================================================== */

const FDICTS: [usize; 7] = [1, 4, 64, 4096, 65535, 65536, 70000];

#[test]
fn r066_compressFrame_usingCDict() {
    let mut rng = Rng::new(0x5EED_0066);
    for &ds in FDICTS.iter() {
        for &lvl in [0i32, 1, 9, 12].iter() {
            for &bmode in [0i32, 1].iter() {
                for &len in [0usize, 1, 1000, 130000].iter() {
                    let dict = mkdata(Shape::Textish, ds, &mut rng);
                    let src = mkdata(Shape::Textish, len, &mut rng);
                    let mut p = LZ4F_preferences_t::default();
                    p.compressionLevel = lvl;
                    p.frameInfo.blockMode = bmode;
                    p.frameInfo.contentChecksumFlag = 1;
                    diff(
                        &format!("frame_usingCDict ds={ds} lvl={lvl} bm={bmode} len={len}"),
                        |lib| unsafe {
                            let mut cctx: *mut CVoid = std::ptr::null_mut();
                            sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(
                                &mut cctx, 100,
                            );
                            let cd = sym::<FnCreateCDict>(lib, "LZ4F_createCDict")(
                                dict.as_ptr(),
                                ds,
                            );
                            let bound = sym::<FnBound>(lib, "LZ4F_compressFrameBound")(len, &p);
                            let mut d = vec![0u8; bound + 64];
                            let r = sym::<FnCompressFrameCDict>(
                                lib,
                                "LZ4F_compressFrame_usingCDict",
                            )(
                                cctx,
                                d.as_mut_ptr(),
                                bound,
                                src.as_ptr(),
                                len,
                                cd,
                                &p,
                            );
                            let ok = sym::<FnIsError>(lib, "LZ4F_isError")(r) == 0;
                            let frame = if ok { d[..r].to_vec() } else { Vec::new() };
                            // and with cdict == NULL (documented: no dictionary)
                            let mut d2 = vec![0u8; bound + 64];
                            let r2 = sym::<FnCompressFrameCDict>(
                                lib,
                                "LZ4F_compressFrame_usingCDict",
                            )(
                                cctx,
                                d2.as_mut_ptr(),
                                bound,
                                src.as_ptr(),
                                len,
                                std::ptr::null(),
                                &p,
                            );
                            let ok2 = sym::<FnIsError>(lib, "LZ4F_isError")(r2) == 0;
                            let frame2 = if ok2 { d2[..r2].to_vec() } else { Vec::new() };
                            sym::<FnFreeCDict>(lib, "LZ4F_freeCDict")(cd);
                            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
                            (r as i64, frame, r2 as i64, frame2)
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn r074_r077_begin_using_dicts() {
    let mut rng = Rng::new(0x5EED_0074);
    for &ds in FDICTS.iter() {
        for &lvl in [0i32, 9, 12].iter() {
            for &bmode in [0i32, 1].iter() {
                let dict = mkdata(Shape::Textish, ds, &mut rng);
                let src = mkdata(Shape::Textish, 90000, &mut rng);
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = lvl;
                p.frameInfo.blockMode = bmode;
                p.frameInfo.contentChecksumFlag = 1;
                for chunk in [1usize, 7000, 90000] {
                    let n = src.len() / chunk + 2;
                    let steps = vec![(chunk, UpdKind::Compressed, false); n];
                    for mode in 0..4 {
                        let begin = match mode {
                            0 => BeginMode::UsingDict(&dict),
                            1 => BeginMode::UsingDictOnce(&dict),
                            2 => BeginMode::UsingCDict(&dict),
                            _ => BeginMode::Internal(Some(&dict), None),
                        };
                        let plan = StreamPlan {
                            begin,
                            prefs: Some(p),
                            copts: None,
                            steps: steps.clone(),
                        };
                        diff(
                            &format!("beginDict m={mode} ds={ds} lvl={lvl} bm={bmode} c={chunk}"),
                            |lib| compress_stream(lib, &src, &plan),
                        );
                    }
                    // internal with a cdict instead of a raw dict
                    let plan = StreamPlan {
                        begin: BeginMode::Internal(None, Some(&dict)),
                        prefs: Some(p),
                        copts: None,
                        steps: steps.clone(),
                    };
                    diff(
                        &format!("beginInternal cdict ds={ds} lvl={lvl} bm={bmode} c={chunk}"),
                        |lib| compress_stream(lib, &src, &plan),
                    );
                    // internal with neither
                    let plan = StreamPlan {
                        begin: BeginMode::Internal(None, None),
                        prefs: Some(p),
                        copts: None,
                        steps: steps.clone(),
                    };
                    diff(
                        &format!("beginInternal none ds={ds} lvl={lvl} bm={bmode} c={chunk}"),
                        |lib| compress_stream(lib, &src, &plan),
                    );
                }
            }
        }
    }
}

#[test]
fn r075_dictOnce_second_frame() {
    let mut rng = Rng::new(0x5EED_0075);
    for &ds in FDICTS.iter() {
        let dict = mkdata(Shape::Textish, ds, &mut rng);
        let a = mkdata(Shape::Textish, 20000, &mut rng);
        let b = mkdata(Shape::Textish, 20000, &mut rng);
        diff(&format!("dictOnce 2 frames ds={ds}"), |lib| unsafe {
            let mut cctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createCompressionContext")(&mut cctx, 100);
            let mut out: Vec<i64> = Vec::new();
            let mut bytes: Vec<u8> = Vec::new();
            for (k, src) in [&a, &b].iter().enumerate() {
                let mut hdr = vec![0u8; 64];
                let hr = if k == 0 {
                    sym::<FnBeginDict>(lib, "LZ4F_compressBegin_usingDictOnce")(
                        cctx,
                        hdr.as_mut_ptr(),
                        hdr.len(),
                        dict.as_ptr(),
                        ds,
                        std::ptr::null(),
                    )
                } else {
                    sym::<FnBegin>(lib, "LZ4F_compressBegin")(
                        cctx,
                        hdr.as_mut_ptr(),
                        hdr.len(),
                        std::ptr::null(),
                    )
                };
                out.push(hr as i64);
                bytes.extend_from_slice(&hdr[..hr]);
                let bound =
                    sym::<FnBound>(lib, "LZ4F_compressBound")(src.len(), std::ptr::null());
                let mut d = vec![0u8; bound + 64];
                let r = sym::<FnUpdate>(lib, "LZ4F_compressUpdate")(
                    cctx,
                    d.as_mut_ptr(),
                    d.len(),
                    src.as_ptr(),
                    src.len(),
                    std::ptr::null(),
                );
                out.push(r as i64);
                bytes.extend_from_slice(&d[..r]);
                let mut e = vec![0u8; sym::<FnBound>(lib, "LZ4F_compressBound")(0, std::ptr::null()) + 64];
                let ecap = e.len();
                let er = sym::<FnFlush>(lib, "LZ4F_compressEnd")(
                    cctx,
                    e.as_mut_ptr(),
                    ecap,
                    std::ptr::null(),
                );
                out.push(er as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(er) == 0 {
                    bytes.extend_from_slice(&e[..er]);
                }
            }
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cctx);
            (out, bytes)
        });
    }
}

#[test]
fn r092_decompress_usingDict() {
    let mut rng = Rng::new(0x5EED_0092);
    for &ds in FDICTS.iter() {
        for &lvl in [0i32, 9].iter() {
            for &bmode in [0i32, 1].iter() {
                let dict = mkdata(Shape::Textish, ds, &mut rng);
                let src = mkdata(Shape::Textish, 60000, &mut rng);
                let mut p = LZ4F_preferences_t::default();
                p.compressionLevel = lvl;
                p.frameInfo.blockMode = bmode;
                p.frameInfo.contentChecksumFlag = 1;
                // build the frame with the C, using the dict
                let i = impls();
                let plan = StreamPlan {
                    begin: BeginMode::UsingDict(&dict),
                    prefs: Some(p),
                    copts: None,
                    steps: vec![(usize::MAX, UpdKind::Compressed, false)],
                };
                let frame = compress_stream(&i.c, &src, &plan).frame;
                assert!(!frame.is_empty());
                for sc in [0usize, 1, 4096] {
                    diff(
                        &format!("decompress_usingDict ds={ds} lvl={lvl} bm={bmode} sc={sc}"),
                        |lib| {
                            decompress_frame(
                                lib,
                                &frame,
                                src.len(),
                                sc,
                                0,
                                None,
                                Some(&dict),
                                false,
                            )
                        },
                    );
                }
                let d = decompress_frame(&i.r, &frame, src.len(), 0, 0, None, Some(&dict), false);
                assert_eq!(&d.out[..], &src[..], "dict roundtrip ds={ds}");
            }
        }
    }
}

/* ================================================================== */
/* row 93 — resetDecompressionContext                                   */
/* ================================================================== */

#[test]
fn r093_reset_dctx() {
    let mut rng = Rng::new(0x5EED_0093);
    let src = mkdata(Shape::Textish, 80000, &mut rng);
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.contentChecksumFlag = 1;
    p.frameInfo.blockChecksumFlag = 1;
    let frame = make_frame(&src, Some(&p));
    for cut in [1usize, 7, 19, 100, 5000] {
        diff(&format!("reset dctx cut={cut}"), |lib| unsafe {
            let mut dctx: *mut CVoid = std::ptr::null_mut();
            sym::<FnCreateCtx>(lib, "LZ4F_createDecompressionContext")(&mut dctx, 100);
            let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
            let mut out = vec![0u8; src.len() + 1024];
            let mut dn = out.len();
            let mut sn = cut.min(frame.len());
            let r1 = dec(
                dctx,
                out.as_mut_ptr(),
                &mut dn,
                frame.as_ptr(),
                &mut sn,
                std::ptr::null(),
            );
            sym::<FnResetDctx>(lib, "LZ4F_resetDecompressionContext")(dctx);
            // now decode the whole frame from scratch on the reset context
            let mut written = 0usize;
            let mut consumed = 0usize;
            let mut codes = vec![r1 as i64, dn as i64, sn as i64];
            loop {
                if consumed >= frame.len() {
                    break;
                }
                let mut d2 = out.len() - written;
                let mut s2 = frame.len() - consumed;
                let r = dec(
                    dctx,
                    out[written..].as_mut_ptr(),
                    &mut d2,
                    frame[consumed..].as_ptr(),
                    &mut s2,
                    std::ptr::null(),
                );
                codes.push(r as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(r) != 0 {
                    break;
                }
                written += d2;
                consumed += s2;
                if r == 0 {
                    break;
                }
                if d2 == 0 && s2 == 0 {
                    break;
                }
            }
            sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dctx);
            (codes, out[..written].to_vec())
        });
    }
}

/* ================================================================== */
/* rows 78,79,80 — custom-memory context creation                       */
/* ================================================================== */

static ALLOC_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static FREE_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn my_alloc(_o: *mut CVoid, size: usize) -> *mut CVoid {
    ALLOC_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    unsafe { libc_malloc(size) }
}
unsafe extern "C" fn my_calloc(_o: *mut CVoid, size: usize) -> *mut CVoid {
    ALLOC_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    unsafe {
        let p = libc_malloc(size);
        if !p.is_null() {
            std::ptr::write_bytes(p as *mut u8, 0, size);
        }
        p
    }
}
unsafe extern "C" fn my_free(_o: *mut CVoid, p: *mut CVoid) {
    FREE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    unsafe { libc_free(p) }
}
unsafe extern "C" fn fail_alloc(_o: *mut CVoid, _size: usize) -> *mut CVoid {
    std::ptr::null_mut()
}

// Minimal libc bindings so the custom allocator uses the same heap as the C lib.
unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut CVoid;
    #[link_name = "free"]
    fn libc_free(p: *mut CVoid);
}

#[test]
fn r078_r080_custom_mem() {
    let mut rng = Rng::new(0x5EED_0078);
    let src = mkdata(Shape::Textish, 90000, &mut rng);
    let dict = mkdata(Shape::Textish, 5000, &mut rng);

    for variant in 0..3 {
        let cm = match variant {
            0 => LZ4F_CustomMem {
                customAlloc: None,
                customCalloc: None,
                customFree: None,
                opaque: std::ptr::null_mut(),
            },
            1 => LZ4F_CustomMem {
                customAlloc: Some(my_alloc),
                customCalloc: Some(my_calloc),
                customFree: Some(my_free),
                opaque: std::ptr::null_mut(),
            },
            // customCalloc == NULL: the C falls back to customAlloc + memset
            _ => LZ4F_CustomMem {
                customAlloc: Some(my_alloc),
                customCalloc: None,
                customFree: Some(my_free),
                opaque: std::ptr::null_mut(),
            },
        };
        diff(&format!("custom mem variant={variant}"), |lib| unsafe {
            let cc = sym::<FnCreateCtxAdv>(lib, "LZ4F_createCompressionContext_advanced")(cm, 100);
            let dc =
                sym::<FnCreateCtxAdv>(lib, "LZ4F_createDecompressionContext_advanced")(cm, 100);
            let cd = sym::<FnCreateCDictAdv>(lib, "LZ4F_createCDict_advanced")(
                cm,
                dict.as_ptr(),
                dict.len(),
            );
            let mut out: Vec<i64> = vec![
                cc.is_null() as i64,
                dc.is_null() as i64,
                cd.is_null() as i64,
            ];
            let mut frame = Vec::new();
            if !cc.is_null() {
                let mut p = LZ4F_preferences_t::default();
                p.frameInfo.contentChecksumFlag = 1;
                let bound = sym::<FnBound>(lib, "LZ4F_compressFrameBound")(src.len(), &p);
                let mut d = vec![0u8; bound + 64];
                let r = sym::<FnCompressFrameCDict>(lib, "LZ4F_compressFrame_usingCDict")(
                    cc,
                    d.as_mut_ptr(),
                    bound,
                    src.as_ptr(),
                    src.len(),
                    cd,
                    &p,
                );
                out.push(r as i64);
                if sym::<FnIsError>(lib, "LZ4F_isError")(r) == 0 {
                    frame = d[..r].to_vec();
                }
            }
            if !dc.is_null() && !frame.is_empty() {
                let dec = sym::<FnDecompress>(lib, "LZ4F_decompress");
                let mut o = vec![0u8; src.len() + 1024];
                let mut written = 0usize;
                let mut consumed = 0usize;
                loop {
                    if consumed >= frame.len() {
                        break;
                    }
                    let mut dn = o.len() - written;
                    let mut sn = frame.len() - consumed;
                    let r = dec(
                        dc,
                        o[written..].as_mut_ptr(),
                        &mut dn,
                        frame[consumed..].as_ptr(),
                        &mut sn,
                        std::ptr::null(),
                    );
                    out.push(r as i64);
                    if sym::<FnIsError>(lib, "LZ4F_isError")(r) != 0 {
                        break;
                    }
                    written += dn;
                    consumed += sn;
                    if r == 0 || (dn == 0 && sn == 0) {
                        break;
                    }
                }
                out.push(written as i64);
                out.push((o[..written] == src[..]) as i64);
            }
            sym::<FnFreeCDict>(lib, "LZ4F_freeCDict")(cd);
            sym::<FnFreeCtx>(lib, "LZ4F_freeCompressionContext")(cc);
            sym::<FnFreeCtx>(lib, "LZ4F_freeDecompressionContext")(dc);
            (out, frame)
        });
    }
    // allocator that always fails -> creation must return NULL (ERRORS row 80)
    let cm_fail = LZ4F_CustomMem {
        customAlloc: Some(fail_alloc),
        customCalloc: Some(fail_alloc),
        customFree: Some(my_free),
        opaque: std::ptr::null_mut(),
    };
    diff("custom mem failing allocator", |lib| unsafe {
        let cc =
            sym::<FnCreateCtxAdv>(lib, "LZ4F_createCompressionContext_advanced")(cm_fail, 100);
        let dc =
            sym::<FnCreateCtxAdv>(lib, "LZ4F_createDecompressionContext_advanced")(cm_fail, 100);
        let cd = sym::<FnCreateCDictAdv>(lib, "LZ4F_createCDict_advanced")(
            cm_fail,
            dict.as_ptr(),
            dict.len(),
        );
        (cc.is_null(), dc.is_null(), cd.is_null())
    });
}

/* ================================================================== */
/* row 108 — frame block extracted and decoded with the block API       */
/* ================================================================== */

#[test]
fn r108_frame_block_via_block_api() {
    let mut rng = Rng::new(0x5EED_0108);
    // blockIndependent + autoFlush: each block is a self-contained LZ4 block,
    // so LZ4_decompress_safe must decode it directly.
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockMode = 1;
    p.frameInfo.blockSizeID = 4;
    p.autoFlush = 1;
    let src = mkdata(Shape::Textish, 200000, &mut rng);
    let frame = make_frame(&src, Some(&p));
    diff("frame block via block api", |lib| unsafe {
        // walk the frame: 7-byte header (no contentSize/dictID), then blocks
        let hs = sym::<FnHeaderSize>(lib, "LZ4F_headerSize")(frame.as_ptr(), frame.len());
        let mut pos = hs;
        let mut outs: Vec<(i32, Vec<u8>)> = Vec::new();
        let dec = sym::<unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32>(
            lib,
            "LZ4_decompress_safe",
        );
        while pos + 4 <= frame.len() {
            let bh = u32::from_le_bytes(frame[pos..pos + 4].try_into().unwrap());
            pos += 4;
            if bh == 0 {
                break; // EndMark
            }
            let uncompressed = (bh & 0x8000_0000) != 0;
            let bs = (bh & 0x7FFF_FFFF) as usize;
            if pos + bs > frame.len() {
                break;
            }
            if uncompressed {
                outs.push((bs as i32, frame[pos..pos + bs].to_vec()));
            } else {
                let mut o = vec![0u8; 65536];
                let n = dec(
                    frame[pos..].as_ptr(),
                    o.as_mut_ptr(),
                    bs as i32,
                    o.len() as i32,
                );
                o.truncate(if n > 0 { n as usize } else { 0 });
                outs.push((n, o));
            }
            pos += bs;
        }
        (hs as i64, outs)
    });
}
