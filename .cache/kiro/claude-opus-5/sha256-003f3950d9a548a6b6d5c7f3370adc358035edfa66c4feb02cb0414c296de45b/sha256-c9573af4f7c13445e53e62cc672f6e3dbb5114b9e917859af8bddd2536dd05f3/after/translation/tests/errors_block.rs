//! Phase C — error-path differential tests. One test per ERRORS.md row group.
//!
//! Each case constructs the exact invalid input/condition, calls BOTH the C and
//! the Rust `.so`, and asserts they return the SAME error code / sentinel.

mod common;
use common::*;

type F4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type F5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FDestSize = unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32) -> i32;
type FDestSizeExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FHCDestSize = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FI32 = unsafe extern "C" fn(i32) -> i32;
type FInitStream = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
type FFree = unsafe extern "C" fn(*mut u8) -> i32;
type FCreate = unsafe extern "C" fn() -> *mut u8;
type FLoadDict = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FAttach = unsafe extern "C" fn(*mut u8, *const u8);
type FContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FHCContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FSaveDict = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
type FDecPartial = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FDecFast = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FDecDict = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32;
type FSetLevel = unsafe extern "C" fn(*mut u8, i32);

fn valid_compressed(src: &[u8]) -> Vec<u8> {
    let (c, _) = sym::<F4>("LZ4_compress_default");
    let bound = lz4_compress_bound(src.len() as i32) as usize;
    let mut d = vec![0u8; bound + 8];
    let n = unsafe { c(src.as_ptr(), d.as_mut_ptr(), src.len() as i32, bound as i32) };
    d.truncate(n.max(0) as usize);
    d
}

/// Rows 1-5: oversized / negative / zero srcSize and exhausted dst budget.
#[test]
fn rows01_05_compress_size_rejections() {
    let (c, r) = sym::<F4>("LZ4_compress_default");
    let (cf, rf) = sym::<F5>("LZ4_compress_fast");
    let mut rng = Rng::new(0xE001);
    let src = make_data(&mut rng, 4096, Shape::Text);
    let mut cd = vec![0u8; 8192];
    let mut rd = vec![0u8; 8192];

    // rows 1-2: srcSize > LZ4_MAX_INPUT_SIZE and srcSize < 0 -> 0
    for bad in [
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
        -1,
        -4096,
        i32::MIN,
        i32::MIN + 1,
    ] {
        let (a, b) = unsafe {
            (
                c(src.as_ptr(), cd.as_mut_ptr(), bad, cd.len() as i32),
                r(src.as_ptr(), rd.as_mut_ptr(), bad, rd.len() as i32),
            )
        };
        eq(&format!("row1/2 compress_default srcSize={bad}"), a, b);
        eq(&format!("row1/2 compress_default srcSize={bad} == 0"), a, 0);
        for acc in [1i32, 0, -1, 9] {
            let (a, b) = unsafe {
                (
                    cf(src.as_ptr(), cd.as_mut_ptr(), bad, cd.len() as i32, acc),
                    rf(src.as_ptr(), rd.as_mut_ptr(), bad, rd.len() as i32, acc),
                )
            };
            eq(&format!("row1/2 compress_fast srcSize={bad} acc={acc}"), a, b);
            eq(
                &format!("row1/2 compress_fast srcSize={bad} acc={acc} == 0"),
                a,
                0,
            );
        }
    }

    // row 3: dst budget exhausted -> 0 (incompressible data, tiny dst)
    for &shape in &SHAPES {
        for len in [16usize, 100, 4096, 65540] {
            let s = make_data(&mut rng, len, shape);
            for cap in [0i32, 1, 2, 3, 4, 8] {
                let (a, b) = unsafe {
                    (
                        c(s.as_ptr(), cd.as_mut_ptr(), len as i32, cap),
                        r(s.as_ptr(), rd.as_mut_ptr(), len as i32, cap),
                    )
                };
                eq(&format!("row3 shape={shape:?} len={len} cap={cap}"), a, b);
            }
        }
    }

    // rows 4-5: srcSize == 0 with dstCapacity <= 0 and >= 1
    for cap in [i32::MIN, -1, 0, 1, 2, 16] {
        let empty = make_data(&mut rng, 0, Shape::Zeros);
        let (a, b) = unsafe {
            (
                c(empty.as_ptr(), cd.as_mut_ptr(), 0, cap),
                r(empty.as_ptr(), rd.as_mut_ptr(), 0, cap),
            )
        };
        eq(&format!("row4/5 srcSize=0 cap={cap}"), a, b);
        let want = if cap <= 0 { 0 } else { 1 };
        eq(&format!("row4/5 srcSize=0 cap={cap} value"), a, want);
    }

    // row 5 variant: src == NULL with srcSize == 0 (documented as supported)
    for cap in [0i32, 1, 16] {
        let (a, b) = unsafe {
            (
                c(std::ptr::null(), cd.as_mut_ptr(), 0, cap),
                r(std::ptr::null(), rd.as_mut_ptr(), 0, cap),
            )
        };
        eq(&format!("row5 src=NULL srcSize=0 cap={cap}"), a, b);
    }
}

/// Rows 6-7, 9: external-state guards.
///
/// IMPORTANT (verified against the C source): `LZ4_compress_fast_extState`
/// computes `&LZ4_initStream(state, ...)->internal_donotuse`. Because
/// `internal_donotuse` sits at offset 0, a NULL/misaligned/undersized state
/// makes `ctx` NULL, the following `assert(ctx != NULL)` is compiled out under
/// NDEBUG, and `LZ4_compress_generic` then dereferences NULL. The same holds
/// for `LZ4_compress_fast_extState_fastReset`, which has no state validation at
/// all. Those inputs therefore FAULT IN THE C REFERENCE ITSELF and are not
/// differentially testable -- see ERRORS.md rows 6/7/9.
///
/// What IS observable is the guard the C actually implements: `LZ4_initStream`
/// rejects NULL / undersized / misaligned buffers (rows 10-12, covered in
/// `rows10_13_init_stream_guards`). Here we verify that a state which
/// `LZ4_initStream` ACCEPTS behaves identically in both implementations, and
/// that `LZ4_compress_destSize_extState` agrees across the acceleration range.
#[test]
fn rows06_09_ext_state_guarded_behaviour() {
    let (ce, re) = sym::<FExt>("LZ4_compress_fast_extState");
    let (cfr, rfr) = sym::<FExt>("LZ4_compress_fast_extState_fastReset");
    let (cds, rds) = sym::<FDestSizeExt>("LZ4_compress_destSize_extState");
    let mut rng = Rng::new(0xE006);

    let mut cbuf = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rbuf = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();

    for &shape in &SHAPES {
        for len in [0usize, 1, 16, 4096, 70_000] {
            let src = make_data(&mut rng, len, shape);
            for acc in [i32::MIN, -1, 0, 1, 2, 17, 65537, i32::MAX] {
                for cap in [0usize, 1, 8, lz4_compress_bound(len as i32) as usize] {
                    let mut cd = vec![0xA1u8; cap + 16];
                    let mut rd = vec![0xA1u8; cap + 16];
                    cbuf.fill0();
                    rbuf.fill0();
                    let (a, b) = unsafe {
                        (
                            ce(
                                cbuf.ptr(),
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                acc,
                            ),
                            re(
                                rbuf.ptr(),
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                acc,
                            ),
                        )
                    };
                    let ctx =
                        format!("row6/9 extState shape={shape:?} len={len} acc={acc} cap={cap}");
                    eq(&ctx, a, b);
                    eq_bytes(&ctx, &cd, &rd);

                    // fastReset on an already-initialised state
                    let (a, b) = unsafe {
                        (
                            cfr(
                                cbuf.ptr(),
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                acc,
                            ),
                            rfr(
                                rbuf.ptr(),
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                len as i32,
                                cap as i32,
                                acc,
                            ),
                        )
                    };
                    let ctx = format!(
                        "row7 extState_fastReset shape={shape:?} len={len} acc={acc} cap={cap}"
                    );
                    eq(&ctx, a, b);
                    eq_bytes(&ctx, &cd, &rd);

                    // row 9: destSize_extState over the same ranges
                    let mut s1 = len as i32;
                    let mut s2 = len as i32;
                    cbuf.fill0();
                    rbuf.fill0();
                    let (a, b) = unsafe {
                        (
                            cds(
                                cbuf.ptr(),
                                src.as_ptr(),
                                cd.as_mut_ptr(),
                                &mut s1,
                                cap as i32,
                                acc,
                            ),
                            rds(
                                rbuf.ptr(),
                                src.as_ptr(),
                                rd.as_mut_ptr(),
                                &mut s2,
                                cap as i32,
                                acc,
                            ),
                        )
                    };
                    let ctx = format!(
                        "row9 destSize_extState shape={shape:?} len={len} acc={acc} cap={cap}"
                    );
                    eq(&ctx, a, b);
                    eq(&format!("{ctx} srcSizePtr"), s1, s2);
                    eq_bytes(&ctx, &cd, &rd);
                }
            }
        }
    }
}

/// Row 8: `LZ4_compress_destSize` with targetDstSize < 1.
#[test]
fn row08_dest_size_too_small() {
    let (c, r) = sym::<FDestSize>("LZ4_compress_destSize");
    let mut rng = Rng::new(0xE008);
    for &shape in &SHAPES {
        for len in [1usize, 100, 4096, 70_000] {
            let src = make_data(&mut rng, len, shape);
            for target in [i32::MIN, -1, 0] {
                let mut cd = vec![0u8; 64];
                let mut rd = vec![0u8; 64];
                let mut s1 = len as i32;
                let mut s2 = len as i32;
                let (a, b) = unsafe {
                    (
                        c(src.as_ptr(), cd.as_mut_ptr(), &mut s1, target),
                        r(src.as_ptr(), rd.as_mut_ptr(), &mut s2, target),
                    )
                };
                let ctx = format!("row8 destSize shape={shape:?} len={len} target={target}");
                eq(&ctx, a, b);
                eq(&format!("{ctx} == 0"), a, 0);
                eq(&format!("{ctx} srcSizePtr"), s1, s2);
            }
        }
    }
}

/// Rows 10-13: `LZ4_initStream` guards and free-on-NULL.
#[test]
fn rows10_13_init_stream_guards() {
    let (c, r) = sym::<FInitStream>("LZ4_initStream");
    let (cf, rf) = sym::<FFree>("LZ4_freeStream");
    type FVoidI32 = unsafe extern "C" fn() -> i32;
    let (csz, _) = sym::<FVoidI32>("LZ4_sizeofState");
    let need = unsafe { csz() } as usize;

    // row 10: buffer == NULL
    for size in [0usize, 1, need, need * 2] {
        let (a, b) = unsafe {
            (
                c(std::ptr::null_mut(), size),
                r(std::ptr::null_mut(), size),
            )
        };
        let ctx = format!("row10 initStream(NULL,{size})");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }

    // row 11: size < sizeof(LZ4_stream_t)
    let mut cbuf = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rbuf = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    for size in [0usize, 1, 8, need / 2, need - 1] {
        let (a, b) = unsafe { (c(cbuf.ptr(), size), r(rbuf.ptr(), size)) };
        let ctx = format!("row11 initStream(size={size} < {need})");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }
    // sufficient size succeeds in both
    for size in [need, need + 1, need + 127] {
        let (a, b) = unsafe { (c(cbuf.ptr(), size), r(rbuf.ptr(), size)) };
        eq(
            &format!("row11 initStream(size={size}) non-null"),
            a.is_null(),
            b.is_null(),
        );
        assert!(!a.is_null());
    }

    // row 12: misaligned buffer
    for off in 1usize..8 {
        let (a, b) = unsafe {
            (
                c(cbuf.ptr().add(off), need),
                r(rbuf.ptr().add(off), need),
            )
        };
        let ctx = format!("row12 initStream misaligned off={off}");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }

    // row 13: free on NULL
    let (a, b) = unsafe { (cf(std::ptr::null_mut()), rf(std::ptr::null_mut())) };
    eq("row13 freeStream(NULL)", a, b);
    eq("row13 freeStream(NULL) == 0", a, 0);

    // and LZ4_freeStreamHC / LZ4_freeStreamDecode on NULL (row 50 + generic)
    for name in ["LZ4_freeStreamHC", "LZ4_freeStreamDecode"] {
        let (c2, r2) = sym::<FFree>(name);
        let (a, b) = unsafe { (c2(std::ptr::null_mut()), r2(std::ptr::null_mut())) };
        eq(&format!("{name}(NULL)"), a, b);
    }
}

/// Rows 14-21, 23-24: decompression rejections.
#[test]
fn rows14_24_decompress_rejections() {
    let (c, r) = sym::<F4>("LZ4_decompress_safe");
    let (cp, rp) = sym::<FDecPartial>("LZ4_decompress_safe_partial");
    let mut rng = Rng::new(0xE014);
    let mut cd = vec![0u8; 300_000];
    let mut rd = vec![0u8; 300_000];

    // row 14: src == NULL
    for (cs, cap) in [(0i32, 0i32), (0, 100), (10, 100), (-1, 100)] {
        let (a, b) = unsafe {
            (
                c(std::ptr::null(), cd.as_mut_ptr(), cs, cap),
                r(std::ptr::null(), rd.as_mut_ptr(), cs, cap),
            )
        };
        eq(&format!("row14 decompress_safe(NULL,{cs},{cap})"), a, b);
        eq(
            &format!("row14 decompress_safe(NULL,{cs},{cap}) == -1"),
            a,
            -1,
        );
    }

    let plain = make_data(&mut rng, 4096, Shape::Text);
    let comp = valid_compressed(&plain);

    // row 15: outputSize < 0
    for cap in [i32::MIN, -1000, -1] {
        let (a, b) = unsafe {
            (
                c(comp.as_ptr(), cd.as_mut_ptr(), comp.len() as i32, cap),
                r(comp.as_ptr(), rd.as_mut_ptr(), comp.len() as i32, cap),
            )
        };
        eq(&format!("row15 dstCapacity={cap}"), a, b);
        eq(&format!("row15 dstCapacity={cap} == -1"), a, -1);
    }

    // row 16: srcSize == 0
    for cap in [0i32, 1, 4096] {
        let (a, b) = unsafe {
            (
                c(comp.as_ptr(), cd.as_mut_ptr(), 0, cap),
                r(comp.as_ptr(), rd.as_mut_ptr(), 0, cap),
            )
        };
        eq(&format!("row16 srcSize=0 cap={cap}"), a, b);
        eq(&format!("row16 srcSize=0 cap={cap} == -1"), a, -1);
    }
    // negative srcSize too
    for cs in [i32::MIN, -1] {
        let (a, b) = unsafe {
            (
                c(comp.as_ptr(), cd.as_mut_ptr(), cs, 4096),
                r(comp.as_ptr(), rd.as_mut_ptr(), cs, 4096),
            )
        };
        eq(&format!("row16 srcSize={cs}"), a, b);
    }

    // rows 17-19: corrupted / truncated streams
    for &shape in &SHAPES {
        for len in [1usize, 64, 1000, 4096, 70_000] {
            let plain = make_data(&mut rng, len, shape);
            let comp = valid_compressed(&plain);
            if comp.is_empty() {
                continue;
            }
            // truncated input
            for t in [1usize, comp.len() / 4, comp.len() / 2, comp.len() - 1] {
                if t == 0 || t >= comp.len() {
                    continue;
                }
                let (a, b) = unsafe {
                    (
                        c(comp.as_ptr(), cd.as_mut_ptr(), t as i32, len as i32),
                        r(comp.as_ptr(), rd.as_mut_ptr(), t as i32, len as i32),
                    )
                };
                eq(&format!("row17 truncated shape={shape:?} len={len} t={t}"), a, b);
            }
            // random single-byte corruption
            for _ in 0..40 {
                let mut bad = comp.clone();
                let i = rng.below(bad.len());
                bad[i] ^= 1 << rng.below(8);
                let (a, b) = unsafe {
                    (
                        c(bad.as_ptr(), cd.as_mut_ptr(), bad.len() as i32, len as i32),
                        r(bad.as_ptr(), rd.as_mut_ptr(), bad.len() as i32, len as i32),
                    )
                };
                let ctx = format!("row17/19 corrupt shape={shape:?} len={len} i={i}");
                eq(&ctx, a, b);
                // whatever was written before failing must also match
                eq_bytes(&format!("{ctx} out"), &cd[..len], &rd[..len]);
            }
            // fully random garbage input
            for _ in 0..40 {
                let n = rng.range(1, 200);
                let garbage = make_data(&mut rng, n, Shape::Random);
                let (a, b) = unsafe {
                    (
                        c(garbage.as_ptr(), cd.as_mut_ptr(), n as i32, len as i32),
                        r(garbage.as_ptr(), rd.as_mut_ptr(), n as i32, len as i32),
                    )
                };
                eq(&format!("row17 garbage n={n} cap={len}"), a, b);
            }

            // row 20: dstCapacity smaller than the true decompressed size
            for cap in [0usize, 1, len / 4, len / 2, len.saturating_sub(1)] {
                let (a, b) = unsafe {
                    (
                        c(
                            comp.as_ptr(),
                            cd.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                        r(
                            comp.as_ptr(),
                            rd.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                    )
                };
                eq(&format!("row20 short dst len={len} cap={cap}"), a, b);
            }

            // row 21: compressedSize larger than the real block
            for extra in [1usize, 2, 17] {
                let mut padded = comp.clone();
                padded.extend(std::iter::repeat(0u8).take(extra));
                let (a, b) = unsafe {
                    (
                        c(
                            padded.as_ptr(),
                            cd.as_mut_ptr(),
                            padded.len() as i32,
                            len as i32,
                        ),
                        r(
                            padded.as_ptr(),
                            rd.as_mut_ptr(),
                            padded.len() as i32,
                            len as i32,
                        ),
                    )
                };
                eq(&format!("row21 overlong srcSize len={len} extra={extra}"), a, b);
            }
        }
    }

    // rows 22-23: partial decoding guards
    for (cs, t, cap) in [
        (0i32, 0i32, 0i32),
        (0, 10, 100),
        (10, -1, 100),
        (10, 10, -1),
        (-1, 10, 100),
    ] {
        let (a, b) = unsafe {
            (
                cp(std::ptr::null(), cd.as_mut_ptr(), cs, t, cap),
                rp(std::ptr::null(), rd.as_mut_ptr(), cs, t, cap),
            )
        };
        eq(&format!("row23 partial(NULL,{cs},{t},{cap})"), a, b);
        let (a, b) = unsafe {
            (
                cp(comp.as_ptr(), cd.as_mut_ptr(), cs, t, cap),
                rp(comp.as_ptr(), rd.as_mut_ptr(), cs, t, cap),
            )
        };
        eq(&format!("row22/23 partial(valid,{cs},{t},{cap})"), a, b);
    }
}

/// Row 24: `LZ4_decompress_fast` on corrupt input.
#[test]
fn row24_decompress_fast_corrupt() {
    let (c, r) = sym::<FDecFast>("LZ4_decompress_fast");
    let mut rng = Rng::new(0xE024);
    for &shape in &SHAPES {
        for len in [1usize, 64, 1000, 4096] {
            let plain = make_data(&mut rng, len, shape);
            let comp = valid_compressed(&plain);
            if comp.is_empty() {
                continue;
            }
            for _ in 0..30 {
                let mut bad = comp.clone();
                let i = rng.below(bad.len());
                bad[i] ^= 1 << rng.below(8);
                // generous slack: decompress_fast may overwrite up to 64B
                let mut cd = vec![0u8; len + 512];
                let mut rd = vec![0u8; len + 512];
                let (a, b) = unsafe {
                    (
                        c(bad.as_ptr(), cd.as_mut_ptr(), len as i32),
                        r(bad.as_ptr(), rd.as_mut_ptr(), len as i32),
                    )
                };
                eq(&format!("row24 fast corrupt shape={shape:?} len={len} i={i}"), a, b);
            }
            // wrong (negative / zero) originalSize
            for os in [i32::MIN + 1, -1, 0] {
                let mut cd = vec![0u8; len + 512];
                let mut rd = vec![0u8; len + 512];
                let (a, b) = unsafe {
                    (
                        c(comp.as_ptr(), cd.as_mut_ptr(), os),
                        r(comp.as_ptr(), rd.as_mut_ptr(), os),
                    )
                };
                eq(&format!("row24 fast originalSize={os} len={len}"), a, b);
            }
        }
    }
}

/// Rows 25-28: `LZ4_decoderRingBufferSize` and `LZ4_compressBound` ranges.
#[test]
fn rows25_28_bound_ranges() {
    for name in ["LZ4_decoderRingBufferSize", "LZ4_compressBound"] {
        let (c, r) = sym::<FI32>(name);
        let mut vals: Vec<i32> = vec![
            i32::MIN,
            i32::MIN + 1,
            -65536,
            -1,
            0,
            1,
            2,
            64,
            65535,
            65536,
            1 << 20,
            LZ4_MAX_INPUT_SIZE - 1,
            LZ4_MAX_INPUT_SIZE,
            LZ4_MAX_INPUT_SIZE + 1,
            i32::MAX - 1,
            i32::MAX,
        ];
        let mut rng = Rng::new(0xE025);
        for _ in 0..2000 {
            vals.push(rng.next_u32() as i32);
        }
        for n in vals {
            eq(&format!("rows25-28 {name}({n})"), unsafe { c(n) }, unsafe {
                r(n)
            });
        }
    }
}

/// Rows 29-30, 32-33: dictionary loading edge cases.
#[test]
fn rows29_33_dict_edges() {
    let (ccs, rcs) = sym::<FCreate>("LZ4_createStream");
    let (cfs, rfs) = sym::<FFree>("LZ4_freeStream");
    let (cld, rld) = sym::<FLoadDict>("LZ4_loadDict");
    let (cls, rls) = sym::<FLoadDict>("LZ4_loadDictSlow");
    let (cat, rat) = sym::<FAttach>("LZ4_attach_dictionary");
    let (ccd, rcd) = sym::<FCreate>("LZ4_createStreamDecode");
    let (cfd, rfd) = sym::<FFree>("LZ4_freeStreamDecode");
    let (csd, rsd) = sym::<FLoadDict>("LZ4_setStreamDecode");
    let (cud, rud) = sym::<FDecDict>("LZ4_decompress_safe_usingDict");

    let mut rng = Rng::new(0xE029);
    let dict = make_data(&mut rng, 200_000, Shape::Text);

    // rows 29-30: loadDict / loadDictSlow with 0, negative, and >64K sizes
    for &n in &[
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        65535,
        65536,
        65537,
        100_000,
        200_000,
    ] {
        for (name, cl, rl) in [
            ("loadDict", &cld, &rld),
            ("loadDictSlow", &cls, &rls),
        ] {
            let (a, b) = unsafe {
                let cs = ccs();
                let rs = rcs();
                let a = cl(cs, dict.as_ptr(), n);
                let b = rl(rs, dict.as_ptr(), n);
                cfs(cs);
                rfs(rs);
                (a, b)
            };
            eq(&format!("row29/30 {name}(dictSize={n})"), a, b);
        }
        // dictionary == NULL is only defined for dictSize < HASH_UNIT (4):
        // LZ4_loadDict_internal returns 0 before touching the pointer. For
        // dictSize >= 4 it does `p = dictEnd - 64KB` and hashes from there,
        // which dereferences NULL in the C reference too (ERRORS.md row 30).
        if n < 4 {
            let (a, b) = unsafe {
                let cs = ccs();
                let rs = rcs();
                let a = cld(cs, std::ptr::null(), n);
                let b = rld(rs, std::ptr::null(), n);
                cfs(cs);
                rfs(rs);
                (a, b)
            };
            eq(&format!("row30 loadDict(NULL,{n})"), a, b);
            eq(&format!("row30 loadDict(NULL,{n}) == 0"), a, 0);
        }
    }

    // row 54-analogue: attach_dictionary(NULL)
    unsafe {
        let cs = ccs();
        let rs = rcs();
        cat(cs, std::ptr::null());
        rat(rs, std::ptr::null());
        // then compress: both must behave identically
        let src = make_data(&mut rng, 8192, Shape::Text);
        let bound = lz4_compress_bound(8192) as usize;
        let mut cd = vec![0u8; bound + 8];
        let mut rd = vec![0u8; bound + 8];
        let (cc, rc) = sym::<FContinue>("LZ4_compress_fast_continue");
        let a = cc(cs, src.as_ptr(), cd.as_mut_ptr(), 8192, bound as i32, 1);
        let b = rc(rs, src.as_ptr(), rd.as_mut_ptr(), 8192, bound as i32, 1);
        eq("attach_dictionary(NULL) ret", a, b);
        eq_bytes("attach_dictionary(NULL) out", &cd, &rd);
        cfs(cs);
        rfs(rs);
    }

    // row 32: setStreamDecode with negative / oversized dictSize
    for &n in &[i32::MIN, -1, 0, 1, 65536, 200_000] {
        let (a, b) = unsafe {
            let cs = ccd();
            let rs = rcd();
            let a = csd(cs, dict.as_ptr(), n);
            let b = rsd(rs, dict.as_ptr(), n);
            cfd(cs);
            rfd(rs);
            (a, b)
        };
        eq(&format!("row32 setStreamDecode(dictSize={n})"), a, b);
        // NULL dictionary
        let (a, b) = unsafe {
            let cs = ccd();
            let rs = rcd();
            let a = csd(cs, std::ptr::null(), n);
            let b = rsd(rs, std::ptr::null(), n);
            cfd(cs);
            rfd(rs);
            (a, b)
        };
        eq(&format!("row32 setStreamDecode(NULL,{n})"), a, b);
    }

    // row 33: decompress_safe_usingDict with NULL / zero dict, and bad dictSize
    let plain = make_data(&mut rng, 4096, Shape::Text);
    let comp = valid_compressed(&plain);
    for &n in &[i32::MIN, -1, 0, 1, 65536] {
        for null_dict in [false, true] {
            // A NULL dictStart is only a defined input when dictSize is 0:
            // lz4.c asserts `dictSize == 0` in that case (ERRORS.md row 33).
            if null_dict && n != 0 {
                continue;
            }
            let dp = if null_dict {
                std::ptr::null()
            } else {
                dict.as_ptr()
            };
            let mut cd = vec![0u8; 8192];
            let mut rd = vec![0u8; 8192];
            let (a, b) = unsafe {
                (
                    cud(
                        comp.as_ptr(),
                        cd.as_mut_ptr(),
                        comp.len() as i32,
                        4096,
                        dp,
                        n,
                    ),
                    rud(
                        comp.as_ptr(),
                        rd.as_mut_ptr(),
                        comp.len() as i32,
                        4096,
                        dp,
                        n,
                    ),
                )
            };
            let ctx = format!("row33 usingDict(null={null_dict}, dictSize={n})");
            eq(&ctx, a, b);
            eq_bytes(&format!("{ctx} out"), &cd, &rd);
        }
    }
}

/// Row 31 / 55: `LZ4_saveDict` / `LZ4_saveDictHC` boundary sizes.
#[test]
fn rows31_55_save_dict_edges() {
    let (ccs, rcs) = sym::<FCreate>("LZ4_createStream");
    let (cfs, rfs) = sym::<FFree>("LZ4_freeStream");
    let (ccont, rcont) = sym::<FContinue>("LZ4_compress_fast_continue");
    let (csv, rsv) = sym::<FSaveDict>("LZ4_saveDict");
    let (chs, rhs) = sym::<FCreate>("LZ4_createStreamHC");
    let (chf, rhf) = sym::<FFree>("LZ4_freeStreamHC");
    let (chc, rhc) = sym::<FHCContinue>("LZ4_compress_HC_continue");
    let (chsv, rhsv) = sym::<FSaveDict>("LZ4_saveDictHC");

    let mut rng = Rng::new(0xE031);
    let src = make_data(&mut rng, 90_000, Shape::Text);
    let bound = lz4_compress_bound(90_000) as usize;

    for &maxdict in &[i32::MIN, -1000, -1, 0, 1, 100, 65535, 65536, 65537, 200_000] {
        // block stream
        let mut got = Vec::new();
        for (create, free, cont, save) in
            [(&ccs, &cfs, &ccont, &csv), (&rcs, &rfs, &rcont, &rsv)]
        {
            unsafe {
                let s = create();
                let mut d = vec![0u8; bound + 8];
                cont(s, src.as_ptr(), d.as_mut_ptr(), 90_000, bound as i32, 1);
                let mut safe = vec![0x5Au8; 300_000];
                let n = save(s, safe.as_mut_ptr(), maxdict);
                free(s);
                safe.truncate(if n > 0 { n as usize } else { 0 });
                got.push((n, safe));
            }
        }
        eq(&format!("row31 saveDict(max={maxdict}) ret"), got[0].0, got[1].0);
        eq_bytes(&format!("row31 saveDict(max={maxdict})"), &got[0].1, &got[1].1);

        // HC stream
        let mut got = Vec::new();
        for (create, free, cont, save) in
            [(&chs, &chf, &chc, &chsv), (&rhs, &rhf, &rhc, &rhsv)]
        {
            unsafe {
                let s = create();
                let mut d = vec![0u8; bound + 8];
                cont(s, src.as_ptr(), d.as_mut_ptr(), 90_000, bound as i32);
                let mut safe = vec![0x5Au8; 300_000];
                let n = save(s, safe.as_mut_ptr(), maxdict);
                free(s);
                safe.truncate(if n > 0 { n as usize } else { 0 });
                got.push((n, safe));
            }
        }
        eq(&format!("row55 saveDictHC(max={maxdict}) ret"), got[0].0, got[1].0);
        eq_bytes(
            &format!("row55 saveDictHC(max={maxdict})"),
            &got[0].1,
            &got[1].1,
        );
    }
}

/// Rows 34-46, 53, 56: HC rejections and compression-level clamping.
#[test]
fn rows34_56_hc_rejections() {
    let (c, r) = sym::<F5>("LZ4_compress_HC");
    let (ce, re) = sym::<FExt>("LZ4_compress_HC_extStateHC");
    let (cfr, rfr) = sym::<FExt>("LZ4_compress_HC_extStateHC_fastReset");
    let (cds, rds) = sym::<FHCDestSize>("LZ4_compress_HC_destSize");
    let (cinit, rinit) = sym::<FInitStream>("LZ4_initStreamHC");
    let mut rng = Rng::new(0xE034);
    let src = make_data(&mut rng, 8192, Shape::Text);
    let mut cd = vec![0u8; 16384];
    let mut rd = vec![0u8; 16384];

    // rows 34-35: srcSize out of range
    for bad in [LZ4_MAX_INPUT_SIZE + 1, i32::MAX, -1, i32::MIN] {
        for &lvl in &[0i32, 1, 9, 12] {
            let (a, b) = unsafe {
                (
                    c(src.as_ptr(), cd.as_mut_ptr(), bad, cd.len() as i32, lvl),
                    r(src.as_ptr(), rd.as_mut_ptr(), bad, rd.len() as i32, lvl),
                )
            };
            eq(&format!("row34/35 HC srcSize={bad} lvl={lvl}"), a, b);
            eq(&format!("row34/35 HC srcSize={bad} lvl={lvl} == 0"), a, 0);
        }
    }

    // row 36: dst too small
    for &shape in &SHAPES {
        for len in [16usize, 1000, 8192] {
            let s = make_data(&mut rng, len, shape);
            for &lvl in &[1i32, 3, 9, 12] {
                for cap in [0i32, 1, 2, 4, 8] {
                    let (a, b) = unsafe {
                        (
                            c(s.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl),
                            r(s.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl),
                        )
                    };
                    eq(
                        &format!("row36 HC shape={shape:?} len={len} lvl={lvl} cap={cap}"),
                        a,
                        b,
                    );
                }
            }
        }
    }

    // rows 37-40, 56: level clamping. Levels <1 -> 9, >12 -> 12.
    // Assert the CLAMPED output is byte-identical to the explicit level.
    for &shape in &SHAPES {
        for len in [64usize, 1000, 20_000] {
            let s = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;
            let mut ref9 = vec![0u8; bound + 8];
            let n9 = unsafe {
                c(s.as_ptr(), ref9.as_mut_ptr(), len as i32, bound as i32, 9)
            };
            let mut ref12 = vec![0u8; bound + 8];
            let n12 = unsafe {
                c(s.as_ptr(), ref12.as_mut_ptr(), len as i32, bound as i32, 12)
            };
            for &lvl in &[i32::MIN, -1000, -5, -1, 0] {
                let mut a = vec![0u8; bound + 8];
                let mut b = vec![0u8; bound + 8];
                let (x, y) = unsafe {
                    (
                        c(s.as_ptr(), a.as_mut_ptr(), len as i32, bound as i32, lvl),
                        r(s.as_ptr(), b.as_mut_ptr(), len as i32, bound as i32, lvl),
                    )
                };
                let ctx = format!("row37/39/40 clamp-low lvl={lvl} shape={shape:?} len={len}");
                eq(&ctx, x, y);
                eq_bytes(&ctx, &a, &b);
                eq(&format!("{ctx} == level 9 len"), x, n9);
                eq_bytes(&format!("{ctx} == level 9 bytes"), &a[..x as usize], &ref9[..n9 as usize]);
            }
            for &lvl in &[13i32, 100, 9999, i32::MAX] {
                let mut a = vec![0u8; bound + 8];
                let mut b = vec![0u8; bound + 8];
                let (x, y) = unsafe {
                    (
                        c(s.as_ptr(), a.as_mut_ptr(), len as i32, bound as i32, lvl),
                        r(s.as_ptr(), b.as_mut_ptr(), len as i32, bound as i32, lvl),
                    )
                };
                let ctx = format!("row38 clamp-high lvl={lvl} shape={shape:?} len={len}");
                eq(&ctx, x, y);
                eq_bytes(&ctx, &a, &b);
                eq(&format!("{ctx} == level 12 len"), x, n12);
                eq_bytes(
                    &format!("{ctx} == level 12 bytes"),
                    &a[..x as usize],
                    &ref12[..n12 as usize],
                );
            }
        }
    }

    // rows 41-42: NULL / misaligned HC state
    let mut cbuf = Aligned::<{ LZ4_STREAMHC_SIZE + 128 }>::new();
    let mut rbuf = Aligned::<{ LZ4_STREAMHC_SIZE + 128 }>::new();
    for &lvl in &[0i32, 9, 12] {
        let (a, b) = unsafe {
            (
                ce(
                    std::ptr::null_mut(),
                    src.as_ptr(),
                    cd.as_mut_ptr(),
                    src.len() as i32,
                    cd.len() as i32,
                    lvl,
                ),
                re(
                    std::ptr::null_mut(),
                    src.as_ptr(),
                    rd.as_mut_ptr(),
                    src.len() as i32,
                    rd.len() as i32,
                    lvl,
                ),
            )
        };
        eq(&format!("row42 HC extState NULL lvl={lvl}"), a, b);
        eq(&format!("row42 HC extState NULL lvl={lvl} == 0"), a, 0);

        for off in 1usize..8 {
            let (a, b) = unsafe {
                (
                    cfr(
                        cbuf.ptr().add(off),
                        src.as_ptr(),
                        cd.as_mut_ptr(),
                        src.len() as i32,
                        cd.len() as i32,
                        lvl,
                    ),
                    rfr(
                        rbuf.ptr().add(off),
                        src.as_ptr(),
                        rd.as_mut_ptr(),
                        src.len() as i32,
                        rd.len() as i32,
                        lvl,
                    ),
                )
            };
            eq(&format!("row41 HC fastReset misaligned off={off} lvl={lvl}"), a, b);
            eq(
                &format!("row41 HC fastReset misaligned off={off} lvl={lvl} == 0"),
                a,
                0,
            );
        }
    }

    // rows 43-46: HC destSize guards
    for &lvl in &[0i32, 1, 9, 12] {
        for (sz, target) in [
            (-1i32, 1024i32),
            (i32::MIN, 1024),
            (LZ4_MAX_INPUT_SIZE + 1, 1024),
            (i32::MAX, 1024),
            (8192, -1),
            (8192, i32::MIN),
            (8192, 0),
        ] {
            let mut s1 = sz;
            let mut s2 = sz;
            let (a, b) = unsafe {
                (
                    cds(
                        cbuf.ptr(),
                        src.as_ptr(),
                        cd.as_mut_ptr(),
                        &mut s1,
                        target,
                        lvl,
                    ),
                    rds(
                        rbuf.ptr(),
                        src.as_ptr(),
                        rd.as_mut_ptr(),
                        &mut s2,
                        target,
                        lvl,
                    ),
                )
            };
            let ctx = format!("row43-46 HC destSize src={sz} target={target} lvl={lvl}");
            eq(&ctx, a, b);
            eq(&format!("{ctx} == 0"), a, 0);
            eq(&format!("{ctx} srcSizePtr"), s1, s2);
        }
    }

    // rows 47-49: initStreamHC guards
    type FVoidI32 = unsafe extern "C" fn() -> i32;
    let (csz, _) = sym::<FVoidI32>("LZ4_sizeofStateHC");
    let need = unsafe { csz() } as usize;
    for size in [0usize, 1, 8, need / 2, need - 1] {
        let (a, b) = unsafe { (cinit(cbuf.ptr(), size), rinit(rbuf.ptr(), size)) };
        let ctx = format!("row48 initStreamHC(size={size} < {need})");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }
    for size in [0usize, need, need * 2] {
        let (a, b) = unsafe {
            (
                cinit(std::ptr::null_mut(), size),
                rinit(std::ptr::null_mut(), size),
            )
        };
        let ctx = format!("row47 initStreamHC(NULL,{size})");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }
    for off in 1usize..8 {
        let (a, b) = unsafe {
            (
                cinit(cbuf.ptr().add(off), need),
                rinit(rbuf.ptr().add(off), need),
            )
        };
        let ctx = format!("row49 initStreamHC misaligned off={off}");
        eq(&ctx, a.is_null(), b.is_null());
        assert!(a.is_null(), "{ctx}: C should return NULL");
    }
}

/// Rows 51-54: HC dictionary edge cases and `LZ4_setCompressionLevel` clamping.
#[test]
fn rows51_54_hc_dict_edges() {
    let (chs, rhs) = sym::<FCreate>("LZ4_createStreamHC");
    let (chf, rhf) = sym::<FFree>("LZ4_freeStreamHC");
    let (cld, rld) = sym::<FLoadDict>("LZ4_loadDictHC");
    let (cat, rat) = sym::<FAttach>("LZ4_attach_HC_dictionary");
    let (ccont, rcont) = sym::<FHCContinue>("LZ4_compress_HC_continue");
    let (csl, rsl) = sym::<FSetLevel>("LZ4_setCompressionLevel");

    let mut rng = Rng::new(0xE051);
    let dict = make_data(&mut rng, 200_000, Shape::Text);
    let src = make_data(&mut rng, 16_384, Shape::Text);
    let bound = lz4_compress_bound(16_384) as usize;

    // rows 51-52: loadDictHC with 0 / negative / >64K / NULL
    // NOTE: `LZ4_loadDictHC` validates only via `assert(dictSize >= 0)`, which
    // is compiled out under NDEBUG, and never NULL-checks `dictionary`. A
    // negative size or a NULL dictionary therefore faults in the C reference
    // itself, so only non-negative sizes with a real pointer are testable
    // (ERRORS.md rows 51-52).
    for &n in &[0i32, 1, 3, 4, 65535, 65536, 65537, 200_000] {
        for null_dict in [false] {
            for &lvl in &[1i32, 3, 9, 12] {
                let dp = if null_dict {
                    std::ptr::null()
                } else {
                    dict.as_ptr()
                };
                let mut got = Vec::new();
                for (create, free, load, cont, setl) in [
                    (&chs, &chf, &cld, &ccont, &csl),
                    (&rhs, &rhf, &rld, &rcont, &rsl),
                ] {
                    unsafe {
                        let s = create();
                        setl(s, lvl);
                        let ld = load(s, dp, n);
                        let mut d = vec![0x4Cu8; bound + 8];
                        let cn = cont(
                            s,
                            src.as_ptr(),
                            d.as_mut_ptr(),
                            16_384,
                            bound as i32,
                        );
                        free(s);
                        got.push((ld, cn, d));
                    }
                }
                let ctx = format!("row51/52 loadDictHC(null={null_dict},{n}) lvl={lvl}");
                eq(&format!("{ctx} loadRet"), got[0].0, got[1].0);
                eq(&format!("{ctx} compRet"), got[0].1, got[1].1);
                eq_bytes(&ctx, &got[0].2, &got[1].2);
            }
        }
    }

    // row 54: attach_HC_dictionary(NULL) and attaching an lz4mid-level dict ctx
    for &dict_lvl in &[1i32, 2, 3, 9, 12] {
        for attach_null in [false, true] {
            let mut got = Vec::new();
            for (create, free, load, attach, cont, setl) in [
                (&chs, &chf, &cld, &cat, &ccont, &csl),
                (&rhs, &rhf, &rld, &rat, &rcont, &rsl),
            ] {
                unsafe {
                    let ds = create();
                    setl(ds, dict_lvl);
                    load(ds, dict.as_ptr(), 65536);
                    let s = create();
                    setl(s, 9);
                    if attach_null {
                        attach(s, std::ptr::null());
                    } else {
                        attach(s, ds);
                    }
                    let mut d = vec![0x4Du8; bound + 8];
                    let cn = cont(s, src.as_ptr(), d.as_mut_ptr(), 16_384, bound as i32);
                    free(s);
                    free(ds);
                    got.push((cn, d));
                }
            }
            let ctx = format!("row54 attach_HC_dictionary(null={attach_null}) dictLvl={dict_lvl}");
            eq(&format!("{ctx} ret"), got[0].0, got[1].0);
            eq_bytes(&ctx, &got[0].1, &got[1].1);
        }
    }

    // row 53: HC continue with dst too small
    for &lvl in &[1i32, 3, 9, 12] {
        for cap in [0i32, 1, 2, 8, 64] {
            let mut got = Vec::new();
            for (create, free, cont, setl) in
                [(&chs, &chf, &ccont, &csl), (&rhs, &rhf, &rcont, &rsl)]
            {
                unsafe {
                    let s = create();
                    setl(s, lvl);
                    let mut d = vec![0x4Eu8; 128];
                    let n = cont(s, src.as_ptr(), d.as_mut_ptr(), 16_384, cap);
                    free(s);
                    got.push((n, d));
                }
            }
            let ctx = format!("row53 HC_continue lvl={lvl} cap={cap}");
            eq(&format!("{ctx} ret"), got[0].0, got[1].0);
            eq_bytes(&ctx, &got[0].1, &got[1].1);
        }
    }

    // row 56: setCompressionLevel clamping observed through output equality
    for &lvl in &[i32::MIN, -1000, -1, 0, 1, 2, 12, 13, 9999, i32::MAX] {
        let mut got = Vec::new();
        for (create, free, cont, setl) in
            [(&chs, &chf, &ccont, &csl), (&rhs, &rhf, &rcont, &rsl)]
        {
            unsafe {
                let s = create();
                setl(s, lvl);
                let mut d = vec![0x4Fu8; bound + 8];
                let n = cont(s, src.as_ptr(), d.as_mut_ptr(), 16_384, bound as i32);
                free(s);
                got.push((n, d));
            }
        }
        let ctx = format!("row56 setCompressionLevel({lvl})");
        eq(&format!("{ctx} ret"), got[0].0, got[1].0);
        eq_bytes(&ctx, &got[0].1, &got[1].1);
    }
}
