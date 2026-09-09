//! Phase B — CONFIGS.md groups 1, 2 and 4: `lz4.c` one-shot compression,
//! all decompression variants, and the deprecated/legacy exports.
mod common;
use common::*;

const SEED: u64 = 0x5A5A_1234_ABCD_0001;

// ============================================================ Group 1 ========

/// Rows 1-9: srcSize boundary classes x payload shapes, tight dstCapacity.
#[test]
fn g1_compress_default_sizes_and_shapes() {
    let (c, r) = syms::<FnCompressDefault>("LZ4_compress_default");
    let (cb, rb) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED);

    for &len in BOUNDARY_LENS.iter() {
        for &shape in ALL_SHAPES.iter() {
            for _ in 0..8 {
                let src = mkdata(shape, len, &mut rng);
                let bound = unsafe { cb(len as i32) };
                assert_eq!(bound, unsafe { rb(len as i32) }, "compressBound({len})");
                let cap = bound.max(1) as usize;
                // full capacity
                let mut cd = vec![0xAAu8; cap];
                let mut rd = vec![0xAAu8; cap];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32) };
                same_full(&format!("compress_default len={len} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
                // tight: exactly the produced size, and one byte short
                if cn > 0 {
                    for cap2 in [cn, cn - 1, (cn / 2).max(1)] {
                        let mut cd = vec![0x55u8; cap2.max(1) as usize];
                        let mut rd = vec![0x55u8; cap2.max(1) as usize];
                        let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap2) };
                        let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap2) };
                        same(
                            &format!("compress_default tight len={len} cap={cap2} {shape:?}"),
                            a as i64, &cd, b as i64, &rd,
                        );
                    }
                }
            }
        }
    }
}

/// Row 6/7: the byU16 -> byU32 hash-table switch at LZ4_64KLIMIT (65547).
#[test]
fn g1_compress_default_64k_limit() {
    let (c, r) = syms::<FnCompressDefault>("LZ4_compress_default");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x11);
    for &len in BIG_LENS.iter() {
        for &shape in &[Shape::Random, Shape::SmallAlphabet, Shape::Periodic, Shape::Chunky] {
            for _ in 0..3 {
                let src = mkdata(shape, len, &mut rng);
                let cap = unsafe { cb(len as i32) }.max(1) as usize;
                let mut cd = vec![0u8; cap];
                let mut rd = vec![0u8; cap];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32) };
                same(&format!("64klimit len={len} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
            }
        }
    }
}

/// Rows 10-13: every acceleration class, including the clamps.
#[test]
fn g1_compress_fast_acceleration() {
    let (c, r) = syms::<FnCompressFast>("LZ4_compress_fast");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let accels: [i32; 14] = [
        i32::MIN, -1, 0, 1, 2, 3, 7, 17, 64, 1000, 65536, 65537, 65538, i32::MAX,
    ];
    let mut rng = Rng::new(SEED ^ 0x22);
    for &acc in accels.iter() {
        for &len in &[0usize, 1, 12, 13, 100, 4096, 70000] {
            for &shape in &[Shape::Random, Shape::SmallAlphabet, Shape::Periodic, Shape::Textish] {
                let src = mkdata(shape, len, &mut rng);
                let cap = unsafe { cb(len as i32) }.max(1) as usize;
                let mut cd = vec![0u8; cap];
                let mut rd = vec![0u8; cap];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32, acc) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32, acc) };
                same(
                    &format!("compress_fast acc={acc} len={len} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }
        }
    }
}

/// Rows 14-15: explicit external state, fresh and reused (fastReset).
#[test]
fn g1_compress_fast_ext_state() {
    type FnSizeofState = unsafe extern "C" fn() -> i32;
    type FnExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
    let (cs, rs) = syms::<FnSizeofState>("LZ4_sizeofState");
    let sz_c = unsafe { cs() };
    let sz_r = unsafe { rs() };
    assert_eq!(sz_c, sz_r, "LZ4_sizeofState");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");

    for name in ["LZ4_compress_fast_extState", "LZ4_compress_fast_extState_fastReset"] {
        let (c, r) = syms::<FnExt>(name);
        // 16-byte over-aligned scratch so both see identical alignment.
        let mut cstate = vec![0u64; (sz_c as usize + 7) / 8 + 8];
        let mut rstate = vec![0u64; (sz_c as usize + 7) / 8 + 8];
        let mut rng = Rng::new(SEED ^ name.len() as u64);
        for round in 0..40 {
            let len = rng.range(0, 9000);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let src = mkdata(shape, len, &mut rng);
            let acc = [1i32, 0, 2, 9, 65537][round % 5];
            let cap = unsafe { cb(len as i32) }.max(1) as usize;
            let mut cd = vec![0u8; cap];
            let mut rd = vec![0u8; cap];
            let cn = unsafe {
                c(cstate.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32, acc)
            };
            let rn = unsafe {
                r(rstate.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32, acc)
            };
            same(&format!("{name} round={round} len={len} acc={acc}"), cn as i64, &cd, rn as i64, &rd);
            // state must stay bit-identical, otherwise later rounds diverge
            assert_eq!(
                &cstate[..(sz_c as usize) / 8],
                &rstate[..(sz_c as usize) / 8],
                "{name}: internal state diverged after round {round}"
            );
        }
    }
}

/// Rows 16-19: LZ4_compress_destSize / _extState (fillOutput directive).
#[test]
fn g1_compress_dest_size() {
    type FnDestSize = unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32) -> i32;
    type FnDestSizeExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
    type FnSizeofState = unsafe extern "C" fn() -> i32;
    let (c, r) = syms::<FnDestSize>("LZ4_compress_destSize");
    let mut rng = Rng::new(SEED ^ 0x33);

    for &len in &[0usize, 1, 12, 13, 100, 1000, 4096, 70000] {
        for &shape in ALL_SHAPES.iter() {
            let src = mkdata(shape, len, &mut rng);
            for &tgt in &[1i32, 2, 3, 12, 13, 30, 100, 1000, 100000] {
                let cap = tgt.max(1) as usize;
                let mut cd = vec![0u8; cap];
                let mut rd = vec![0u8; cap];
                let mut cin = len as i32;
                let mut rin = len as i32;
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), &mut cin, tgt) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), &mut rin, tgt) };
                let lbl = format!("destSize len={len} tgt={tgt} {shape:?}");
                assert_eq!(cin, rin, "{lbl}: *srcSizePtr C={cin} R={rin}");
                same(&lbl, cn as i64, &cd, rn as i64, &rd);
            }
        }
    }

    // extState variant with the same matrix plus acceleration
    let (cs, _) = syms::<FnSizeofState>("LZ4_sizeofState");
    let sz = unsafe { cs() } as usize;
    let (ce, re) = syms::<FnDestSizeExt>("LZ4_compress_destSize_extState");
    let mut cstate = vec![0u64; sz / 8 + 8];
    let mut rstate = vec![0u64; sz / 8 + 8];
    for &len in &[0usize, 13, 1000, 70000] {
        for &shape in &[Shape::Random, Shape::Periodic, Shape::Textish] {
            let src = mkdata(shape, len, &mut rng);
            for &tgt in &[1i32, 12, 100, 5000] {
                for &acc in &[1i32, 0, 5, 65537] {
                    let cap = tgt.max(1) as usize;
                    let mut cd = vec![0u8; cap];
                    let mut rd = vec![0u8; cap];
                    let mut cin = len as i32;
                    let mut rin = len as i32;
                    let cn = unsafe {
                        ce(cstate.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), &mut cin, tgt, acc)
                    };
                    let rn = unsafe {
                        re(rstate.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), &mut rin, tgt, acc)
                    };
                    let lbl = format!("destSize_extState len={len} tgt={tgt} acc={acc} {shape:?}");
                    assert_eq!(cin, rin, "{lbl}: *srcSizePtr C={cin} R={rin}");
                    same(&lbl, cn as i64, &cd, rn as i64, &rd);
                }
            }
        }
    }
}

/// Rows 20-21: pure scalar functions.
#[test]
fn g1_scalars() {
    let (cb, rb) = syms::<FnCompressBound>("LZ4_compressBound");
    for &n in &[
        0i32, 1, 2, 12, 13, 100, 65535, 65536, 65547, 0x7DFF_FFFF, 0x7E00_0000, 0x7E00_0001,
        0x7FFF_FFFF, -1, i32::MIN,
    ] {
        assert_eq!(unsafe { cb(n) }, unsafe { rb(n) }, "LZ4_compressBound({n})");
    }
    type FnI = unsafe extern "C" fn() -> i32;
    for name in ["LZ4_versionNumber", "LZ4_sizeofState", "LZ4_sizeofStreamState"] {
        let (c, r) = syms::<FnI>(name);
        assert_eq!(unsafe { c() }, unsafe { r() }, "{name}");
    }
    type FnStr = unsafe extern "C" fn() -> *const std::os::raw::c_char;
    let (c, r) = syms::<FnStr>("LZ4_versionString");
    let cv = unsafe { std::ffi::CStr::from_ptr(c()) };
    let rv = unsafe { std::ffi::CStr::from_ptr(r()) };
    assert_eq!(cv, rv, "LZ4_versionString");
}

// ============================================================ Group 2 ========

/// Rows 22-26: LZ4_decompress_safe on valid, oversized, undersized and
/// truncated input, plus fuzzed garbage.
#[test]
fn g2_decompress_safe() {
    let (c, r) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let mut rng = Rng::new(SEED ^ 0x44);

    for &shape in ALL_SHAPES.iter() {
        for &len in &[0usize, 1, 12, 13, 100, 1000, 4096, 70000] {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            for &cap in &[len as i32, len as i32 + 1, len as i32 + 100, (len as i32) - 1, 0] {
                if cap < 0 {
                    continue;
                }
                let mut cd = vec![0x3Cu8; cap.max(1) as usize];
                let mut rd = vec![0x3Cu8; cap.max(1) as usize];
                let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, cap) };
                let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, cap) };
                same_full(
                    &format!("decompress_safe len={len} cap={cap} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }
            // truncation at every prefix length (bounded)
            let step = (comp.len() / 40).max(1);
            let mut t = 0;
            while t <= comp.len() {
                let mut cd = vec![0u8; len.max(1)];
                let mut rd = vec![0u8; len.max(1)];
                let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), t as i32, len as i32) };
                let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), t as i32, len as i32) };
                same_full(
                    &format!("decompress_safe trunc len={len} t={t} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                t += step;
            }
        }
    }
}

/// Row 25: fuzzed / bit-flipped compressed streams. The exact negative return
/// value (which encodes the failure position) must match.
#[test]
fn g2_decompress_safe_fuzz() {
    let (c, r) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let mut rng = Rng::new(SEED ^ 0x55);
    for round in 0..4000 {
        let len = rng.range(0, 600);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let src = mkdata(shape, len, &mut rng);
        let mut comp = c_compress(&src);
        // corrupt 1..3 random bytes
        if !comp.is_empty() {
            for _ in 0..rng.range(1, 3) {
                let i = rng.below(comp.len());
                comp[i] = rng.byte();
            }
        }
        // pure garbage every 4th round
        if round % 4 == 0 {
            comp = mkdata(Shape::Random, rng.range(0, 80), &mut rng);
        }
        for &cap in &[len as i32, len as i32 * 2 + 8, 4, 0] {
            let mut cd = vec![0x77u8; cap.max(1) as usize];
            let mut rd = vec![0x77u8; cap.max(1) as usize];
            let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, cap) };
            let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, cap) };
            assert_eq!(
                cn, rn,
                "decompress_safe fuzz round={round} cap={cap} srcLen={} : C={cn} R={rn}\ninput={}",
                comp.len(), hexish(&comp)
            );
            if cn > 0 {
                assert_eq!(
                    &cd[..cn as usize], &rd[..cn as usize],
                    "decompress_safe fuzz round={round} output differs"
                );
            }
        }
    }
}

/// Rows 27-29: LZ4_decompress_safe_partial, incl. the MIN(target,capacity) clamp.
#[test]
fn g2_decompress_safe_partial() {
    let (c, r) = syms::<FnDecompressPartial>("LZ4_decompress_safe_partial");
    let mut rng = Rng::new(SEED ^ 0x66);
    for &shape in ALL_SHAPES.iter() {
        for &len in &[0usize, 1, 12, 13, 100, 1000, 4096] {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            for tgt in [0i32, 1, (len / 3) as i32, (len / 2) as i32, len as i32, len as i32 + 5] {
                for cap in [0i32, 1, (len / 2) as i32, len as i32, len as i32 + 10] {
                    let mut cd = vec![0x11u8; cap.max(1) as usize];
                    let mut rd = vec![0x11u8; cap.max(1) as usize];
                    let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, tgt, cap) };
                    let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, tgt, cap) };
                    same_full(
                        &format!("partial len={len} tgt={tgt} cap={cap} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
            // corrupted input with partial target
            let mut bad = comp.clone();
            if !bad.is_empty() {
                let bl = bad.len();
                bad[rng.below(bl)] ^= 0xFF;
            }
            for tgt in [0i32, 1, (len / 2) as i32, len as i32] {
                let mut cd = vec![0u8; len.max(1)];
                let mut rd = vec![0u8; len.max(1)];
                let cn = unsafe { c(bad.as_ptr(), cd.as_mut_ptr(), bad.len() as i32, tgt, len as i32) };
                let rn = unsafe { r(bad.as_ptr(), rd.as_mut_ptr(), bad.len() as i32, tgt, len as i32) };
                assert_eq!(cn, rn, "partial corrupt len={len} tgt={tgt}: C={cn} R={rn}");
            }
        }
    }
}

/// Rows 30-31: LZ4_decompress_fast (deprecated, unsafe generic).
#[test]
fn g2_decompress_fast() {
    let (c, r) = syms::<FnDecompressFast>("LZ4_decompress_fast");
    let mut rng = Rng::new(SEED ^ 0x77);
    for &shape in ALL_SHAPES.iter() {
        for &len in &[1usize, 12, 13, 100, 1000, 4096] {
            let src = mkdata(shape, len, &mut rng);
            let comp = c_compress(&src);
            // generous slack so the unsafe decoder cannot run off our allocation
            let mut cd = vec![0u8; len + 512];
            let mut rd = vec![0u8; len + 512];
            let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), len as i32) };
            let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), len as i32) };
            assert_eq!(cn, rn, "decompress_fast len={len} {shape:?}");
            assert_eq!(&cd[..len], &rd[..len], "decompress_fast output len={len}");
            assert_eq!(&cd[..len], &src[..], "decompress_fast roundtrip len={len}");

            // wrong (too small) originalSize -> must diverge identically
            for os in [0i32, 1, (len / 2) as i32] {
                let mut cd = vec![0u8; len + 512];
                let mut rd = vec![0u8; len + 512];
                let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), os) };
                let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), os) };
                assert_eq!(cn, rn, "decompress_fast bad os={os} len={len}");
                if cn >= 0 {
                    assert_eq!(&cd[..os.max(0) as usize], &rd[..os.max(0) as usize]);
                }
            }
        }
    }
}

/// Rows 32-40: every dictionary-aware decompression entry point.
#[test]
fn g2_decompress_using_dict() {
    type FnUsingDict = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32;
    type FnPartialUsingDict =
        unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32, *const u8, i32) -> i32;
    type FnFastUsingDict = unsafe extern "C" fn(*const u8, *mut u8, i32, *const u8, i32) -> i32;
    type FnPrefix64k = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
    type FnFastPrefix64k = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;

    // Build a stream that genuinely references a dictionary, using the C
    // streaming compressor so cross-block matches exist.
    type FnCreateStream = unsafe extern "C" fn() -> *mut u8;
    type FnFreeStream = unsafe extern "C" fn(*mut u8) -> i32;
    type FnLoadDict = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
    type FnContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;

    let (create, _) = syms::<FnCreateStream>("LZ4_createStream");
    let (free, _) = syms::<FnFreeStream>("LZ4_freeStream");
    let (load, _) = syms::<FnLoadDict>("LZ4_loadDict");
    let (cont, _) = syms::<FnContinue>("LZ4_compress_fast_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");

    let mut rng = Rng::new(SEED ^ 0x88);
    for &dict_size in &[0usize, 1, 3, 4, 100, 1000, 65535, 65536] {
        for &blk in &[1usize, 13, 200, 3000] {
            for &shape in &[Shape::SmallAlphabet, Shape::Periodic, Shape::Textish, Shape::Random] {
                let dict = mkdata(shape, dict_size, &mut rng);
                let blkdata = if dict_size >= 8 && blk >= 8 {
                    // guarantee cross-block matches
                    let mut v = dict[dict_size.saturating_sub(blk.min(dict_size))..].to_vec();
                    v.extend(mkdata(shape, blk.saturating_sub(v.len()), &mut rng));
                    v.truncate(blk);
                    v
                } else {
                    mkdata(shape, blk, &mut rng)
                };
                if blkdata.is_empty() {
                    continue;
                }
                let st = unsafe { create() };
                unsafe { load(st, dict.as_ptr(), dict_size as i32) };
                let cap = unsafe { cb(blkdata.len() as i32) }.max(1) as usize;
                let mut comp = vec![0u8; cap];
                let n = unsafe {
                    cont(st, blkdata.as_ptr(), comp.as_mut_ptr(), blkdata.len() as i32, cap as i32, 1)
                };
                unsafe { free(st) };
                assert!(n > 0);
                comp.truncate(n as usize);

                // (a) non-contiguous dict -> forceExtDict path
                for name in ["LZ4_decompress_safe_usingDict", "LZ4_decompress_safe_forceExtDict"] {
                    let (c, r) = syms::<FnUsingDict>(name);
                    let mut cd = vec![0u8; blkdata.len() + 16];
                    let mut rd = vec![0u8; blkdata.len() + 16];
                    let cn = unsafe {
                        c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32,
                          blkdata.len() as i32, dict.as_ptr(), dict_size as i32)
                    };
                    let rn = unsafe {
                        r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32,
                          blkdata.len() as i32, dict.as_ptr(), dict_size as i32)
                    };
                    same_full(
                        &format!("{name} dict={dict_size} blk={blk} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
                // (b) fast variant
                {
                    let (c, r) = syms::<FnFastUsingDict>("LZ4_decompress_fast_usingDict");
                    let mut cd = vec![0u8; blkdata.len() + 512];
                    let mut rd = vec![0u8; blkdata.len() + 512];
                    let cn = unsafe {
                        c(comp.as_ptr(), cd.as_mut_ptr(), blkdata.len() as i32,
                          dict.as_ptr(), dict_size as i32)
                    };
                    let rn = unsafe {
                        r(comp.as_ptr(), rd.as_mut_ptr(), blkdata.len() as i32,
                          dict.as_ptr(), dict_size as i32)
                    };
                    assert_eq!(cn, rn, "fast_usingDict dict={dict_size} blk={blk} {shape:?}");
                    assert_eq!(&cd[..blkdata.len()], &rd[..blkdata.len()]);
                }
                // (c) partial + dict
                {
                    let (c, r) = syms::<FnPartialUsingDict>("LZ4_decompress_safe_partial_usingDict");
                    let (c2, r2) = syms::<FnPartialUsingDict>("LZ4_decompress_safe_partial_forceExtDict");
                    for tgt in [0i32, 1, (blkdata.len() / 2) as i32, blkdata.len() as i32] {
                        for (nm, cf, rf) in [("partial_usingDict", c, r), ("partial_forceExtDict", c2, r2)] {
                            let mut cd = vec![0u8; blkdata.len() + 16];
                            let mut rd = vec![0u8; blkdata.len() + 16];
                            let cn = unsafe {
                                cf(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, tgt,
                                   blkdata.len() as i32, dict.as_ptr(), dict_size as i32)
                            };
                            let rn = unsafe {
                                rf(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, tgt,
                                   blkdata.len() as i32, dict.as_ptr(), dict_size as i32)
                            };
                            same_full(
                                &format!("{nm} dict={dict_size} blk={blk} tgt={tgt} {shape:?}"),
                                cn as i64, &cd, rn as i64, &rd,
                            );
                        }
                    }
                }
                // (d) contiguous placement: dict immediately before dst
                if dict_size > 0 {
                    let (c, r) = syms::<FnUsingDict>("LZ4_decompress_safe_usingDict");
                    let mut cbuf = vec![0u8; dict_size + blkdata.len() + 16];
                    let mut rbuf = vec![0u8; dict_size + blkdata.len() + 16];
                    cbuf[..dict_size].copy_from_slice(&dict);
                    rbuf[..dict_size].copy_from_slice(&dict);
                    let cn = unsafe {
                        c(comp.as_ptr(), cbuf.as_mut_ptr().add(dict_size), comp.len() as i32,
                          blkdata.len() as i32, cbuf.as_ptr(), dict_size as i32)
                    };
                    let rn = unsafe {
                        r(comp.as_ptr(), rbuf.as_mut_ptr().add(dict_size), comp.len() as i32,
                          blkdata.len() as i32, rbuf.as_ptr(), dict_size as i32)
                    };
                    same_full(
                        &format!("usingDict contiguous dict={dict_size} blk={blk} {shape:?}"),
                        cn as i64, &cbuf, rn as i64, &rbuf,
                    );
                }
            }
        }
    }

    // Row 38: withPrefix64k, contiguous 64 KB prefix.
    let mut rng = Rng::new(SEED ^ 0x99);
    for &shape in &[Shape::SmallAlphabet, Shape::Periodic, Shape::Textish] {
        let prefix = mkdata(shape, 65536, &mut rng);
        let blkdata = {
            let mut v = prefix[60000..].to_vec();
            v.extend(mkdata(shape, 2000, &mut rng));
            v
        };
        let st = unsafe { create() };
        unsafe { load(st, prefix.as_ptr(), 65536) };
        let cap = unsafe { cb(blkdata.len() as i32) } as usize;
        let mut comp = vec![0u8; cap];
        let n = unsafe {
            cont(st, blkdata.as_ptr(), comp.as_mut_ptr(), blkdata.len() as i32, cap as i32, 1)
        };
        unsafe { free(st) };
        comp.truncate(n as usize);

        let (c, r) = syms::<FnPrefix64k>("LZ4_decompress_safe_withPrefix64k");
        let mut cbuf = vec![0u8; 65536 + blkdata.len() + 16];
        let mut rbuf = vec![0u8; 65536 + blkdata.len() + 16];
        cbuf[..65536].copy_from_slice(&prefix);
        rbuf[..65536].copy_from_slice(&prefix);
        let cn = unsafe {
            c(comp.as_ptr(), cbuf.as_mut_ptr().add(65536), comp.len() as i32, blkdata.len() as i32)
        };
        let rn = unsafe {
            r(comp.as_ptr(), rbuf.as_mut_ptr().add(65536), comp.len() as i32, blkdata.len() as i32)
        };
        same_full(&format!("withPrefix64k {shape:?}"), cn as i64, &cbuf, rn as i64, &rbuf);

        let (c, r) = syms::<FnFastPrefix64k>("LZ4_decompress_fast_withPrefix64k");
        let mut cbuf = vec![0u8; 65536 + blkdata.len() + 512];
        let mut rbuf = vec![0u8; 65536 + blkdata.len() + 512];
        cbuf[..65536].copy_from_slice(&prefix);
        rbuf[..65536].copy_from_slice(&prefix);
        let cn = unsafe { c(comp.as_ptr(), cbuf.as_mut_ptr().add(65536), blkdata.len() as i32) };
        let rn = unsafe { r(comp.as_ptr(), rbuf.as_mut_ptr().add(65536), blkdata.len() as i32) };
        assert_eq!(cn, rn, "fast_withPrefix64k {shape:?}");
        assert_eq!(
            &cbuf[65536..65536 + blkdata.len()],
            &rbuf[65536..65536 + blkdata.len()]
        );
    }
}

/// Row 41.
#[test]
fn g2_decoder_ring_buffer_size() {
    let (c, r) = syms::<FnCompressBound>("LZ4_decoderRingBufferSize");
    for &n in &[
        0i32, 1, 2, 12, 13, 100, 65535, 65536, 65537, 0x7DFF_FFFF, 0x7E00_0000, 0x7E00_0001,
        i32::MAX, -1, i32::MIN,
    ] {
        assert_eq!(unsafe { c(n) }, unsafe { r(n) }, "LZ4_decoderRingBufferSize({n})");
    }
}

// ============================================================ Group 4 ========

/// Rows 64-66, 68-69: deprecated one-shot wrappers.
#[test]
fn g4_deprecated_oneshot() {
    type Fn4 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
    type Fn4L = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
    type Fn4S = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
    type Fn4SL = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    type FnSizeofState = unsafe extern "C" fn() -> i32;

    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (cs, _) = syms::<FnSizeofState>("LZ4_sizeofState");
    let sz = unsafe { cs() } as usize;
    let mut rng = Rng::new(SEED ^ 0xAA);

    for &len in BOUNDARY_LENS.iter().chain([65535usize, 65547, 70000].iter()) {
        for &shape in ALL_SHAPES.iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = unsafe { cb(len as i32) }.max(1) as usize;

            // LZ4_compress(src, dst, srcSize)  [dst assumed >= compressBound]
            {
                let (c, r) = syms::<Fn4>("LZ4_compress");
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32) };
                same(&format!("LZ4_compress len={len} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
            }
            // LZ4_compress_limitedOutput
            {
                let (c, r) = syms::<Fn4L>("LZ4_compress_limitedOutput");
                for cap in [bound as i32, 1, (bound / 2) as i32] {
                    let mut cd = vec![0u8; cap.max(1) as usize];
                    let mut rd = vec![0u8; cap.max(1) as usize];
                    let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap) };
                    let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap) };
                    same(
                        &format!("LZ4_compress_limitedOutput len={len} cap={cap} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
            // withState variants
            {
                let (c, r) = syms::<Fn4S>("LZ4_compress_withState");
                let mut cst = vec![0u64; sz / 8 + 8];
                let mut rst = vec![0u64; sz / 8 + 8];
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe {
                    c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32)
                };
                let rn = unsafe {
                    r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32)
                };
                same(&format!("compress_withState len={len} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
            }
            {
                let (c, r) = syms::<Fn4SL>("LZ4_compress_limitedOutput_withState");
                let mut cst = vec![0u64; sz / 8 + 8];
                let mut rst = vec![0u64; sz / 8 + 8];
                for cap in [bound as i32, 1] {
                    let mut cd = vec![0u8; cap.max(1) as usize];
                    let mut rd = vec![0u8; cap.max(1) as usize];
                    let cn = unsafe {
                        c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, cap)
                    };
                    let rn = unsafe {
                        r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, cap)
                    };
                    same(
                        &format!("compress_limitedOutput_withState len={len} cap={cap} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }

            // LZ4_uncompress / LZ4_uncompress_unknownOutputSize
            if len > 0 {
                let comp = c_compress(&src);
                let (c, r) = syms::<Fn4>("LZ4_uncompress");
                let mut cd = vec![0u8; len + 512];
                let mut rd = vec![0u8; len + 512];
                let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), len as i32) };
                let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), len as i32) };
                assert_eq!(cn, rn, "LZ4_uncompress len={len} {shape:?}");
                assert_eq!(&cd[..len], &rd[..len]);

                let (c, r) = syms::<Fn4L>("LZ4_uncompress_unknownOutputSize");
                for cap in [len as i32, len as i32 + 5, (len / 2) as i32] {
                    let mut cd = vec![0u8; cap.max(1) as usize];
                    let mut rd = vec![0u8; cap.max(1) as usize];
                    let cn = unsafe { c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, cap) };
                    let rn = unsafe { r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, cap) };
                    same_full(
                        &format!("uncompress_unknownOutputSize len={len} cap={cap} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
        }
    }
}

/// Rows 67, 70-72: legacy streaming state API.
#[test]
fn g4_deprecated_streaming() {
    type FnCreate = unsafe extern "C" fn(*const u8) -> *mut u8;
    type FnCreateStream = unsafe extern "C" fn() -> *mut u8;
    type FnFreeStream = unsafe extern "C" fn(*mut u8) -> i32;
    type FnResetStreamState = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
    type FnCont = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
    type FnContL = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    type FnSlide = unsafe extern "C" fn(*mut u8) -> *mut u8;
    type FnSizeofSS = unsafe extern "C" fn() -> i32;

    let (csz, rsz) = syms::<FnSizeofSS>("LZ4_sizeofStreamState");
    assert_eq!(unsafe { csz() }, unsafe { rsz() }, "LZ4_sizeofStreamState");
    let sz = unsafe { csz() } as usize;

    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0xBB);

    // LZ4_resetStreamState + LZ4_compress_continue
    for &shape in ALL_SHAPES.iter() {
        let (crs, rrs) = syms::<FnResetStreamState>("LZ4_resetStreamState");
        let (cc, rc) = syms::<FnCont>("LZ4_compress_continue");
        let (ccl, rcl) = syms::<FnContL>("LZ4_compress_limitedOutput_continue");

        let total = mkdata(shape, 40000, &mut rng);
        let mut cst = vec![0u64; sz / 8 + 8];
        let mut rst = vec![0u64; sz / 8 + 8];
        let a = unsafe { crs(cst.as_mut_ptr() as *mut u8, total.as_ptr()) };
        let b = unsafe { rrs(rst.as_mut_ptr() as *mut u8, total.as_ptr()) };
        assert_eq!(a, b, "LZ4_resetStreamState");

        let mut off = 0usize;
        let mut k = 0;
        while off < total.len() {
            let n = rng.range(1, 5000).min(total.len() - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let (cn, rn) = if k % 2 == 0 {
                (
                    unsafe { cc(cst.as_mut_ptr() as *mut u8, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32) },
                    unsafe { rc(rst.as_mut_ptr() as *mut u8, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32) },
                )
            } else {
                (
                    unsafe { ccl(cst.as_mut_ptr() as *mut u8, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32) },
                    unsafe { rcl(rst.as_mut_ptr() as *mut u8, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32) },
                )
            };
            same(&format!("compress_continue k={k} n={n} {shape:?}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            k += 1;
        }
        assert_eq!(&cst[..sz / 8], &rst[..sz / 8], "legacy stream state diverged {shape:?}");
    }

    // LZ4_create / LZ4_slideInputBuffer / LZ4_freeStream
    {
        let (ccr, rcr) = syms::<FnCreate>("LZ4_create");
        let (cfr, rfr) = syms::<FnFreeStream>("LZ4_freeStream");
        let (csl, rsl) = syms::<FnSlide>("LZ4_slideInputBuffer");
        let (cc, rc) = syms::<FnCont>("LZ4_compress_continue");
        let mut buf = mkdata(Shape::Textish, 200000, &mut rng);
        // Keep the buffer stable in memory for both libraries.
        let base = buf.as_mut_ptr();
        let cst = unsafe { ccr(base) };
        let rst = unsafe { rcr(base) };
        assert!(!cst.is_null() && !rst.is_null());
        let mut off = 0usize;
        for k in 0..8 {
            let n = 20000usize.min(buf.len() - off);
            if n == 0 { break; }
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(cst, base.add(off), cd.as_mut_ptr(), n as i32) };
            let rn = unsafe { rc(rst, base.add(off), rd.as_mut_ptr(), n as i32) };
            same(&format!("LZ4_create+continue k={k}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            if k == 3 {
                let cp = unsafe { csl(cst) };
                let rp = unsafe { rsl(rst) };
                assert_eq!(
                    cp as usize as isize - base as usize as isize,
                    rp as usize as isize - base as usize as isize,
                    "LZ4_slideInputBuffer returned offset"
                );
                off = (cp as usize) - (base as usize);
            }
        }
        assert_eq!(unsafe { cfr(cst) }, unsafe { rfr(rst) }, "LZ4_freeStream");
    }

    // free(NULL) must agree
    {
        let (cf, rf) = syms::<FnFreeStream>("LZ4_freeStream");
        assert_eq!(unsafe { cf(std::ptr::null_mut()) }, unsafe { rf(std::ptr::null_mut()) });
        let (cf, rf) = syms::<FnFreeStream>("LZ4_freeStreamDecode");
        assert_eq!(unsafe { cf(std::ptr::null_mut()) }, unsafe { rf(std::ptr::null_mut()) });
    }
    // createStream/freeStream lifecycle
    {
        let (cc, rc) = syms::<FnCreateStream>("LZ4_createStream");
        let (cf, rf) = syms::<FnFreeStream>("LZ4_freeStream");
        let a = unsafe { cc() };
        let b = unsafe { rc() };
        assert!(!a.is_null() && !b.is_null());
        assert_eq!(unsafe { cf(a) }, unsafe { rf(b) });
    }
}
