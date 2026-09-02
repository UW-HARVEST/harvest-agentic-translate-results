//! Phase B — block API (lz4.c) differential tests. CONFIGS.md rows 1-45.
//!
//! Every call goes through the `.so` exports of BOTH implementations.

mod common;
use common::*;

// -------------------------------------------------------------- symbol tables

type FnCompressDefault = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnCompressFast = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnCompressExtState = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnDestSize = unsafe extern "C" fn(*const u8, *mut u8, *mut i32, i32) -> i32;
type FnDestSizeExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FnI32I32 = unsafe extern "C" fn(i32) -> i32;
type FnVoidI32 = unsafe extern "C" fn() -> i32;
type FnDecSafe = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnDecPartial = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnDecFast = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnDecUsingDict = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, *const u8, i32) -> i32;
type FnDecFastUsingDict = unsafe extern "C" fn(*const u8, *mut u8, i32, *const u8, i32) -> i32;
type FnDecPartialUsingDict =
    unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32, *const u8, i32) -> i32;
type FnCreateStream = unsafe extern "C" fn() -> *mut u8;
type FnFreeStream = unsafe extern "C" fn(*mut u8) -> i32;
type FnInitStream = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
type FnResetStream = unsafe extern "C" fn(*mut u8);
type FnLoadDict = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnAttach = unsafe extern "C" fn(*mut u8, *const u8);
type FnContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnSaveDict = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
type FnSetStreamDecode = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnDecContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnDecFastContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnCompress3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
type FnCompress4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
type FnCompressWithState = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnCompressWithStateLim = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnCompressContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnCompressContinueLim = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnForceExtDict = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
type FnCreate = unsafe extern "C" fn(*mut u8) -> *mut u8;
type FnResetStreamState = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FnSlide = unsafe extern "C" fn(*mut u8) -> *mut u8;
type FnLoadDictInternal = unsafe extern "C" fn(*mut u8, *const u8, i32, i32) -> i32;

/// Two independent scratch buffers so the C and Rust runs never share state.
fn two_bufs(n: usize) -> (Vec<u8>, Vec<u8>) {
    (vec![0xAAu8; n], vec![0xAAu8; n])
}

// ============================================================ rows 1-8, 13-16

#[test]
fn row01_08_compress_default_and_fast() {
    let (cc, rc) = sym::<FnCompressDefault>("LZ4_compress_default");
    let (cf, rf) = sym::<FnCompressFast>("LZ4_compress_fast");
    let mut rng = Rng::new(0xB10C_0001);

    // rows 1-4: notLimited (dst == compressBound) over shapes and sizes
    for &shape in &SHAPES {
        for &len in BOUNDARY_SIZES.iter() {
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;
            let (mut cd, mut rd) = two_bufs(bound + 8);
            let (a, b) = unsafe {
                (
                    cc(src.as_ptr(), cd.as_mut_ptr(), len as i32, bound as i32),
                    rc(src.as_ptr(), rd.as_mut_ptr(), len as i32, bound as i32),
                )
            };
            let ctx = format!("compress_default shape={shape:?} len={len}");
            eq(&ctx, a, b);
            assert!(a > 0 || len == 0, "{ctx}: unexpected failure {a}");
            eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
        }
    }

    // rows 6-8: acceleration sweep, including out-of-range values
    let accels: [i32; 12] = [
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        5,
        17,
        100,
        65537,
        1 << 20,
    ];
    for &acc in &accels {
        for &shape in &SHAPES {
            for _ in 0..6 {
                let len = rng.range(0, 90_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        cf(
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                        rf(
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                    )
                };
                let ctx = format!("compress_fast acc={acc} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }
        }
    }
}

#[test]
fn row05_compress_default_limited_output() {
    let (cc, rc) = sym::<FnCompressDefault>("LZ4_compress_default");
    let mut rng = Rng::new(0xB10C_0005);
    for &shape in &SHAPES {
        for _ in 0..40 {
            let len = rng.range(1, 40_000);
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;
            // sweep capacities from 0 to just over bound
            for cap in [
                0usize,
                1,
                2,
                len / 8,
                len / 4,
                len / 2,
                len.saturating_sub(1),
                len,
                bound - 1,
                bound,
            ] {
                let (mut cd, mut rd) = two_bufs(cap + 16);
                let (a, b) = unsafe {
                    (
                        cc(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32),
                        rc(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32),
                    )
                };
                let ctx = format!("limitedOutput shape={shape:?} len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
                // Whatever the C wrote outside the reported length must match too.
                eq_bytes(&format!("{ctx} (full buffer)"), &cd, &rd);
            }
        }
    }
}

#[test]
fn row09_10_ext_state() {
    let (ce, re) = sym::<FnCompressExtState>("LZ4_compress_fast_extState");
    let (cfr, rfr) = sym::<FnCompressExtState>("LZ4_compress_fast_extState_fastReset");
    let (csz, rsz) = sym::<FnVoidI32>("LZ4_sizeofState");
    let ssz = unsafe { csz() } as usize;
    eq("LZ4_sizeofState", ssz as i32, unsafe { rsz() });

    let mut rng = Rng::new(0xB10C_0009);
    let mut cst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();

    for &acc in &[1i32, 0, -3, 4, 64, 65537] {
        for &shape in &SHAPES {
            // row 9: fresh state each call
            for _ in 0..5 {
                let len = rng.range(0, 80_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                cst.fill0();
                rst.fill0();
                let (a, b) = unsafe {
                    (
                        ce(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                        re(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                    )
                };
                let ctx = format!("extState acc={acc} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }

            // row 10: fastReset, state reused across successive calls
            cst.fill0();
            rst.fill0();
            for _ in 0..8 {
                let len = rng.range(0, 30_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let (mut cd, mut rd) = two_bufs(bound + 8);
                let (a, b) = unsafe {
                    (
                        cfr(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                        rfr(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            acc,
                        ),
                    )
                };
                let ctx = format!("extState_fastReset acc={acc} shape={shape:?} len={len}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
                eq_bytes(&format!("{ctx} state"), &cst.bytes()[..ssz], &rst.bytes()[..ssz]);
            }
        }
    }
}

#[test]
fn row11_12_compress_destsize() {
    let (cd_, rd_) = sym::<FnDestSize>("LZ4_compress_destSize");
    let (ce, re) = sym::<FnDestSizeExt>("LZ4_compress_destSize_extState");
    let mut rng = Rng::new(0xB10C_0011);
    let mut cst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();

    for &shape in &SHAPES {
        for _ in 0..25 {
            let len = rng.range(1, 70_000);
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;
            for target in [
                0usize,
                1,
                2,
                3,
                12,
                13,
                len / 16 + 1,
                len / 4,
                len / 2,
                len,
                bound,
                bound + 100,
            ] {
                // row 11
                let (mut co, mut ro) = two_bufs(target + 16);
                let mut cs = len as i32;
                let mut rs = len as i32;
                let (a, b) = unsafe {
                    (
                        cd_(src.as_ptr(), co.as_mut_ptr(), &mut cs, target as i32),
                        rd_(src.as_ptr(), ro.as_mut_ptr(), &mut rs, target as i32),
                    )
                };
                let ctx = format!("destSize shape={shape:?} len={len} target={target}");
                eq(&ctx, a, b);
                eq(&format!("{ctx} srcSizePtr"), cs, rs);
                eq_bytes(&ctx, &co[..a.max(0) as usize], &ro[..b.max(0) as usize]);

                // row 12
                for &acc in &[1i32, 0, 8] {
                    let (mut co, mut ro) = two_bufs(target + 16);
                    let mut cs = len as i32;
                    let mut rs = len as i32;
                    cst.fill0();
                    rst.fill0();
                    let (a, b) = unsafe {
                        (
                            ce(
                                cst.ptr(),
                                src.as_ptr(),
                                co.as_mut_ptr(),
                                &mut cs,
                                target as i32,
                                acc,
                            ),
                            re(
                                rst.ptr(),
                                src.as_ptr(),
                                ro.as_mut_ptr(),
                                &mut rs,
                                target as i32,
                                acc,
                            ),
                        )
                    };
                    let ctx = format!("destSize_extState acc={acc} len={len} target={target}");
                    eq(&ctx, a, b);
                    eq(&format!("{ctx} srcSizePtr"), cs, rs);
                    eq_bytes(&ctx, &co[..a.max(0) as usize], &ro[..b.max(0) as usize]);
                }
            }
        }
    }
}

#[test]
fn row13_16_bounds_and_constants() {
    let (cb, rb) = sym::<FnI32I32>("LZ4_compressBound");
    for n in [
        i32::MIN,
        -1,
        0,
        1,
        2,
        100,
        65535,
        65536,
        1 << 20,
        LZ4_MAX_INPUT_SIZE - 1,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ] {
        eq(&format!("compressBound({n})"), unsafe { cb(n) }, unsafe {
            rb(n)
        });
    }
    for name in [
        "LZ4_sizeofState",
        "LZ4_sizeofStreamState",
        "LZ4_versionNumber",
    ] {
        let (c, r) = sym::<FnVoidI32>(name);
        eq(name, unsafe { c() }, unsafe { r() });
    }
    // version string
    type FnStr = unsafe extern "C" fn() -> *const std::os::raw::c_char;
    let (c, r) = sym::<FnStr>("LZ4_versionString");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c());
        let b = std::ffi::CStr::from_ptr(r());
        eq("LZ4_versionString", a, b);
    }
}

#[test]
fn row14_15_obsolete_oneshot() {
    let (c1, r1) = sym::<FnCompress3>("LZ4_compress");
    let (c2, r2) = sym::<FnCompress4>("LZ4_compress_limitedOutput");
    let (c3, r3) = sym::<FnCompressWithState>("LZ4_compress_withState");
    let (c4, r4) = sym::<FnCompressWithStateLim>("LZ4_compress_limitedOutput_withState");
    let mut rng = Rng::new(0xB10C_0014);
    let mut cst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rst = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();

    for &shape in &SHAPES {
        for _ in 0..12 {
            let len = rng.range(0, 80_000);
            let src = make_data(&mut rng, len, shape);
            let bound = lz4_compress_bound(len as i32) as usize;

            let (mut cd, mut rd) = two_bufs(bound + 8);
            let (a, b) = unsafe {
                (
                    c1(src.as_ptr(), cd.as_mut_ptr(), len as i32),
                    r1(src.as_ptr(), rd.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("LZ4_compress shape={shape:?} len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);

            for cap in [0usize, 1, len / 2, bound] {
                let (mut cd, mut rd) = two_bufs(cap + 16);
                let (a, b) = unsafe {
                    (
                        c2(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap as i32),
                        r2(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap as i32),
                    )
                };
                let ctx = format!("LZ4_compress_limitedOutput len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }

            cst.fill0();
            rst.fill0();
            let (mut cd, mut rd) = two_bufs(bound + 8);
            let (a, b) = unsafe {
                (
                    c3(cst.ptr(), src.as_ptr(), cd.as_mut_ptr(), len as i32),
                    r3(rst.ptr(), src.as_ptr(), rd.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("LZ4_compress_withState len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);

            for cap in [0usize, 1, len / 2, bound] {
                cst.fill0();
                rst.fill0();
                let (mut cd, mut rd) = two_bufs(cap + 16);
                let (a, b) = unsafe {
                    (
                        c4(
                            cst.ptr(),
                            src.as_ptr(),
                            cd.as_mut_ptr(),
                            len as i32,
                            cap as i32,
                        ),
                        r4(
                            rst.ptr(),
                            src.as_ptr(),
                            rd.as_mut_ptr(),
                            len as i32,
                            cap as i32,
                        ),
                    )
                };
                let ctx = format!("LZ4_compress_limitedOutput_withState len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &cd[..a.max(0) as usize], &rd[..b.max(0) as usize]);
            }
        }
    }
}

// =========================================================== rows 17-30 decompress

/// Compress with the C implementation so both decoders see identical input.
fn c_compress(src: &[u8]) -> Vec<u8> {
    let (cc, _) = sym::<FnCompressDefault>("LZ4_compress_default");
    let bound = lz4_compress_bound(src.len() as i32) as usize;
    let mut dst = vec![0u8; bound + 8];
    let n = unsafe {
        cc(
            src.as_ptr(),
            dst.as_mut_ptr(),
            src.len() as i32,
            bound as i32,
        )
    };
    assert!(n > 0 || src.is_empty());
    dst.truncate(n.max(0) as usize);
    dst
}

#[test]
fn row17_18_decompress_safe() {
    let (cd, rd) = sym::<FnDecSafe>("LZ4_decompress_safe");
    let mut rng = Rng::new(0xB10C_0017);
    for &shape in &SHAPES {
        for &len in BOUNDARY_SIZES.iter() {
            let src = make_data(&mut rng, len, shape);
            let comp = c_compress(&src);
            if comp.is_empty() {
                continue;
            }
            for slack in [0usize, 1, 17, 1000] {
                let cap = len + slack;
                let (mut co, mut ro) = two_bufs(cap + 8);
                let (a, b) = unsafe {
                    (
                        cd(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                        rd(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                    )
                };
                let ctx = format!("decompress_safe shape={shape:?} len={len} slack={slack}");
                eq(&ctx, a, b);
                eq(&format!("{ctx} roundtrip"), a, len as i32);
                eq_bytes(&ctx, &co[..len], &ro[..len]);
                eq_bytes(&format!("{ctx} full"), &co, &ro);
            }
        }
    }
}

#[test]
fn row19_20_decompress_safe_partial() {
    let (cd, rd) = sym::<FnDecPartial>("LZ4_decompress_safe_partial");
    let mut rng = Rng::new(0xB10C_0019);
    for &shape in &SHAPES {
        for _ in 0..14 {
            let len = rng.range(1, 60_000);
            let src = make_data(&mut rng, len, shape);
            let comp = c_compress(&src);
            let mut targets = vec![0usize, 1, 2, len / 3, len / 2, len - 1, len, len + 5];
            for _ in 0..6 {
                targets.push(rng.range(0, len + 2));
            }
            for t in targets {
                // row 19: dstCapacity == target
                let (mut co, mut ro) = two_bufs(t + 8);
                let (a, b) = unsafe {
                    (
                        cd(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            t as i32,
                        ),
                        rd(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            t as i32,
                        ),
                    )
                };
                let ctx = format!("partial shape={shape:?} len={len} t={t} cap=t");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);

                // row 20: dstCapacity > target
                let cap = len + 32;
                let (mut co, mut ro) = two_bufs(cap + 8);
                let (a, b) = unsafe {
                    (
                        cd(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            cap as i32,
                        ),
                        rd(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            cap as i32,
                        ),
                    )
                };
                let ctx = format!("partial shape={shape:?} len={len} t={t} cap=large");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
            }
        }
    }
}

#[test]
fn row21_29_decompress_fast_and_legacy() {
    let (cf, rf) = sym::<FnDecFast>("LZ4_decompress_fast");
    let (cu, ru) = sym::<FnDecFast>("LZ4_uncompress");
    let (cu2, ru2) = sym::<FnDecSafe>("LZ4_uncompress_unknownOutputSize");
    let mut rng = Rng::new(0xB10C_0021);
    for &shape in &SHAPES {
        for &len in BOUNDARY_SIZES.iter() {
            if len == 0 {
                continue;
            }
            let src = make_data(&mut rng, len, shape);
            let comp = c_compress(&src);
            // decompress_fast may read up to 64 bytes past; give slack.
            let (mut co, mut ro) = two_bufs(len + 128);
            let (a, b) = unsafe {
                (
                    cf(comp.as_ptr(), co.as_mut_ptr(), len as i32),
                    rf(comp.as_ptr(), ro.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("decompress_fast shape={shape:?} len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &co[..len], &ro[..len]);

            let (mut co, mut ro) = two_bufs(len + 128);
            let (a, b) = unsafe {
                (
                    cu(comp.as_ptr(), co.as_mut_ptr(), len as i32),
                    ru(comp.as_ptr(), ro.as_mut_ptr(), len as i32),
                )
            };
            let ctx = format!("LZ4_uncompress shape={shape:?} len={len}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &co[..len], &ro[..len]);

            for cap in [len, len + 40, len / 2] {
                let (mut co, mut ro) = two_bufs(cap + 8);
                let (a, b) = unsafe {
                    (
                        cu2(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                        ru2(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            cap as i32,
                        ),
                    )
                };
                let ctx = format!("uncompress_unknownOutputSize len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&ctx, &co, &ro);
            }
        }
    }
}

#[test]
fn row22_23_prefix64k() {
    // Compress with a 64KB prefix loaded, then decompress with the prefix present.
    let (ccs, _) = sym::<FnCreateStream>("LZ4_createStream");
    let (cfs, _) = sym::<FnFreeStream>("LZ4_freeStream");
    let (cld, _) = sym::<FnLoadDict>("LZ4_loadDict");
    let (ccont, _) = sym::<FnContinue>("LZ4_compress_fast_continue");
    let (csp, rsp) = sym::<FnDecSafe>("LZ4_decompress_safe_withPrefix64k");
    let (cfp, rfp) = sym::<FnDecFast>("LZ4_decompress_fast_withPrefix64k");

    let mut rng = Rng::new(0xB10C_0022);
    for &shape in &SHAPES {
        for _ in 0..6 {
            let dictlen = 65536usize;
            let blocklen = rng.range(1, 30_000);
            // contiguous prefix+block so the prefix path is valid
            let total = make_data(&mut rng, dictlen + blocklen, shape);
            let comp = unsafe {
                let s = ccs();
                cld(s, total.as_ptr(), dictlen as i32);
                let bound = lz4_compress_bound(blocklen as i32) as usize;
                let mut d = vec![0u8; bound + 8];
                let n = ccont(
                    s,
                    total.as_ptr().add(dictlen),
                    d.as_mut_ptr(),
                    blocklen as i32,
                    bound as i32,
                    1,
                );
                cfs(s);
                assert!(n > 0);
                d.truncate(n as usize);
                d
            };

            // Layout: [64KB prefix][dst]
            let mut cbuf = vec![0u8; dictlen + blocklen + 128];
            let mut rbuf = vec![0u8; dictlen + blocklen + 128];
            cbuf[..dictlen].copy_from_slice(&total[..dictlen]);
            rbuf[..dictlen].copy_from_slice(&total[..dictlen]);
            let (a, b) = unsafe {
                (
                    csp(
                        comp.as_ptr(),
                        cbuf.as_mut_ptr().add(dictlen),
                        comp.len() as i32,
                        blocklen as i32,
                    ),
                    rsp(
                        comp.as_ptr(),
                        rbuf.as_mut_ptr().add(dictlen),
                        comp.len() as i32,
                        blocklen as i32,
                    ),
                )
            };
            let ctx = format!("safe_withPrefix64k shape={shape:?} blocklen={blocklen}");
            eq(&ctx, a, b);
            eq(&format!("{ctx} roundtrip"), a, blocklen as i32);
            eq_bytes(&ctx, &cbuf, &rbuf);

            let mut cbuf = vec![0u8; dictlen + blocklen + 128];
            let mut rbuf = vec![0u8; dictlen + blocklen + 128];
            cbuf[..dictlen].copy_from_slice(&total[..dictlen]);
            rbuf[..dictlen].copy_from_slice(&total[..dictlen]);
            let (a, b) = unsafe {
                (
                    cfp(
                        comp.as_ptr(),
                        cbuf.as_mut_ptr().add(dictlen),
                        blocklen as i32,
                    ),
                    rfp(
                        comp.as_ptr(),
                        rbuf.as_mut_ptr().add(dictlen),
                        blocklen as i32,
                    ),
                )
            };
            let ctx = format!("fast_withPrefix64k shape={shape:?} blocklen={blocklen}");
            eq(&ctx, a, b);
            eq_bytes(&ctx, &cbuf[..dictlen + blocklen], &rbuf[..dictlen + blocklen]);
        }
    }
}

#[test]
fn row24_28_decompress_using_dict() {
    let (cud, rud) = sym::<FnDecUsingDict>("LZ4_decompress_safe_usingDict");
    let (cfd, rfd) = sym::<FnDecFastUsingDict>("LZ4_decompress_fast_usingDict");
    let (cpd, rpd) = sym::<FnDecPartialUsingDict>("LZ4_decompress_safe_partial_usingDict");
    let (cfe, rfe) = sym::<FnDecUsingDict>("LZ4_decompress_safe_forceExtDict");
    let (cpe, rpe) = sym::<FnDecPartialUsingDict>("LZ4_decompress_safe_partial_forceExtDict");
    let (ccs, _) = sym::<FnCreateStream>("LZ4_createStream");
    let (cfs, _) = sym::<FnFreeStream>("LZ4_freeStream");
    let (cld, _) = sym::<FnLoadDict>("LZ4_loadDict");
    let (ccont, _) = sym::<FnContinue>("LZ4_compress_fast_continue");

    let mut rng = Rng::new(0xB10C_0024);
    for &shape in &SHAPES {
        for &dictlen in &[0usize, 1, 100, 4096, 65535, 65536, 70000] {
            let blocklen = rng.range(1, 25_000);
            let dict = make_data(&mut rng, dictlen, shape);
            // Block content deliberately shares spans with dict to force ext-dict matches.
            let mut block = make_data(&mut rng, blocklen, shape);
            if dictlen > 8 && blocklen > 8 {
                let n = block.len().min(dict.len()) / 2;
                block[..n].copy_from_slice(&dict[dict.len() - n..]);
            }
            let comp = unsafe {
                let s = ccs();
                cld(s, dict.as_ptr(), dictlen as i32);
                let bound = lz4_compress_bound(blocklen as i32) as usize;
                let mut d = vec![0u8; bound + 8];
                let n = ccont(
                    s,
                    block.as_ptr(),
                    d.as_mut_ptr(),
                    blocklen as i32,
                    bound as i32,
                    1,
                );
                cfs(s);
                assert!(n > 0);
                d.truncate(n as usize);
                d
            };
            let ctx0 = format!("usingDict shape={shape:?} dict={dictlen} block={blocklen}");

            // row 24
            let (mut co, mut ro) = two_bufs(blocklen + 64);
            let (a, b) = unsafe {
                (
                    cud(
                        comp.as_ptr(),
                        co.as_mut_ptr(),
                        comp.len() as i32,
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                    rud(
                        comp.as_ptr(),
                        ro.as_mut_ptr(),
                        comp.len() as i32,
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                )
            };
            eq(&format!("{ctx0} safe"), a, b);
            eq(&format!("{ctx0} safe roundtrip"), a, blocklen as i32);
            eq_bytes(&format!("{ctx0} safe"), &co, &ro);

            // row 25
            let (mut co, mut ro) = two_bufs(blocklen + 128);
            let (a, b) = unsafe {
                (
                    cfd(
                        comp.as_ptr(),
                        co.as_mut_ptr(),
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                    rfd(
                        comp.as_ptr(),
                        ro.as_mut_ptr(),
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                )
            };
            eq(&format!("{ctx0} fast"), a, b);
            eq_bytes(
                &format!("{ctx0} fast"),
                &co[..blocklen],
                &ro[..blocklen],
            );

            // rows 26, 27, 28
            for t in [0usize, 1, blocklen / 2, blocklen, blocklen + 3] {
                let (mut co, mut ro) = two_bufs(blocklen + 64);
                let (a, b) = unsafe {
                    (
                        cpd(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            (blocklen + 64) as i32,
                            dict.as_ptr(),
                            dictlen as i32,
                        ),
                        rpd(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            (blocklen + 64) as i32,
                            dict.as_ptr(),
                            dictlen as i32,
                        ),
                    )
                };
                eq(&format!("{ctx0} partial_usingDict t={t}"), a, b);
                eq_bytes(&format!("{ctx0} partial_usingDict t={t}"), &co, &ro);

                let (mut co, mut ro) = two_bufs(blocklen + 64);
                let (a, b) = unsafe {
                    (
                        cpe(
                            comp.as_ptr(),
                            co.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            (blocklen + 64) as i32,
                            dict.as_ptr(),
                            dictlen as i32,
                        ),
                        rpe(
                            comp.as_ptr(),
                            ro.as_mut_ptr(),
                            comp.len() as i32,
                            t as i32,
                            (blocklen + 64) as i32,
                            dict.as_ptr(),
                            dictlen as i32,
                        ),
                    )
                };
                eq(&format!("{ctx0} partial_forceExtDict t={t}"), a, b);
                eq_bytes(&format!("{ctx0} partial_forceExtDict t={t}"), &co, &ro);
            }

            let (mut co, mut ro) = two_bufs(blocklen + 64);
            let (a, b) = unsafe {
                (
                    cfe(
                        comp.as_ptr(),
                        co.as_mut_ptr(),
                        comp.len() as i32,
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                    rfe(
                        comp.as_ptr(),
                        ro.as_mut_ptr(),
                        comp.len() as i32,
                        blocklen as i32,
                        dict.as_ptr(),
                        dictlen as i32,
                    ),
                )
            };
            eq(&format!("{ctx0} safe_forceExtDict"), a, b);
            eq_bytes(&format!("{ctx0} safe_forceExtDict"), &co, &ro);
        }
    }
}

#[test]
fn row30_ring_buffer_size() {
    let (c, r) = sym::<FnI32I32>("LZ4_decoderRingBufferSize");
    for n in [
        i32::MIN,
        -1,
        0,
        1,
        64,
        65536,
        1 << 20,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ] {
        eq(
            &format!("decoderRingBufferSize({n})"),
            unsafe { c(n) },
            unsafe { r(n) },
        );
    }
}

// ============================================ rows 31-45 streaming / dictionary

/// Drive a full streaming compress session on ONE implementation, returning
/// the concatenated compressed blocks plus the final saveDict output.
struct StreamOps {
    create: libloading::Symbol<'static, FnCreateStream>,
    free: libloading::Symbol<'static, FnFreeStream>,
    cont: libloading::Symbol<'static, FnContinue>,
    load: libloading::Symbol<'static, FnLoadDict>,
    load_slow: libloading::Symbol<'static, FnLoadDict>,
    attach: libloading::Symbol<'static, FnAttach>,
    save: libloading::Symbol<'static, FnSaveDict>,
    reset_fast: libloading::Symbol<'static, FnResetStream>,
    reset: libloading::Symbol<'static, FnResetStream>,
    init: libloading::Symbol<'static, FnInitStream>,
}

fn stream_ops() -> (StreamOps, StreamOps) {
    macro_rules! p {
        ($n:literal, $t:ty) => {
            sym::<$t>($n)
        };
    }
    let (cc, rc) = p!("LZ4_createStream", FnCreateStream);
    let (cf, rf) = p!("LZ4_freeStream", FnFreeStream);
    let (cn, rn) = p!("LZ4_compress_fast_continue", FnContinue);
    let (cl, rl) = p!("LZ4_loadDict", FnLoadDict);
    let (cls, rls) = p!("LZ4_loadDictSlow", FnLoadDict);
    let (ca, ra) = p!("LZ4_attach_dictionary", FnAttach);
    let (cs, rs) = p!("LZ4_saveDict", FnSaveDict);
    let (crf, rrf) = p!("LZ4_resetStream_fast", FnResetStream);
    let (cr, rr) = p!("LZ4_resetStream", FnResetStream);
    let (ci, ri) = p!("LZ4_initStream", FnInitStream);
    (
        StreamOps {
            create: cc,
            free: cf,
            cont: cn,
            load: cl,
            load_slow: cls,
            attach: ca,
            save: cs,
            reset_fast: crf,
            reset: cr,
            init: ci,
        },
        StreamOps {
            create: rc,
            free: rf,
            cont: rn,
            load: rl,
            load_slow: rls,
            attach: ra,
            save: rs,
            reset_fast: rrf,
            reset: rr,
            init: ri,
        },
    )
}

/// How the stream is seeded before the chunk loop.
#[derive(Clone, Copy, Debug)]
enum Seed {
    None,
    ResetFast,
    Reset,
    LoadDict(usize),
    LoadDictSlow(usize),
    AttachDict(usize),
    AttachNull,
    InitStream,
}

/// Run a streaming compression session and return (blocks, saveDict result).
///
/// The source is kept in ONE contiguous buffer so blockLinked matches resolve
/// against the true preceding bytes, exactly as a real consumer would do.
fn run_stream(
    o: &StreamOps,
    seed: Seed,
    dict: &[u8],
    src: &[u8],
    chunks: &[usize],
    accel: i32,
    tight: bool,
    scratch: *mut u8,
    scratch_len: usize,
) -> (Vec<Vec<u8>>, Vec<u8>, i32) {
    unsafe {
    let mut owned_dict_stream: *mut u8 = std::ptr::null_mut();
    let s = match seed {
        Seed::InitStream => {
            let p = (o.init)(scratch, scratch_len);
            assert!(!p.is_null());
            p
        }
        _ => (o.create)(),
    };
    match seed {
        Seed::None | Seed::InitStream => {}
        Seed::ResetFast => (o.reset_fast)(s),
        Seed::Reset => (o.reset)(s),
        Seed::LoadDict(n) => {
            (o.load)(s, dict.as_ptr(), n as i32);
        }
        Seed::LoadDictSlow(n) => {
            (o.load_slow)(s, dict.as_ptr(), n as i32);
        }
        Seed::AttachDict(n) => {
            owned_dict_stream = (o.create)();
            (o.load)(owned_dict_stream, dict.as_ptr(), n as i32);
            (o.attach)(s, owned_dict_stream);
        }
        Seed::AttachNull => (o.attach)(s, std::ptr::null()),
    }

    let mut out = Vec::new();
    let mut off = 0usize;
    for &clen in chunks {
        if off >= src.len() {
            break;
        }
        let clen = clen.min(src.len() - off);
        let bound = lz4_compress_bound(clen as i32) as usize;
        let cap = if tight { bound.saturating_sub(1) } else { bound };
        let mut d = vec![0xCCu8; cap + 16];
        let n = (o.cont)(
            s,
            src.as_ptr().add(off),
            d.as_mut_ptr(),
            clen as i32,
            cap as i32,
            accel,
        );
        d.truncate(if n > 0 { n as usize } else { 0 });
        out.push(d);
        off += clen;
    }

    let mut safe = vec![0xDDu8; 70_000];
    let saved = (o.save)(s, safe.as_mut_ptr(), 65536);
    safe.truncate(if saved > 0 { saved as usize } else { 0 });

    if !matches!(seed, Seed::InitStream) {
        (o.free)(s);
    }
    if !owned_dict_stream.is_null() {
        (o.free)(owned_dict_stream);
    }
    (out, safe, saved)
    }
}

#[test]
fn row31_40_streaming_compress() {
    let (co, ro) = stream_ops();
    let mut rng = Rng::new(0xB10C_0031);
    let mut cscratch = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
    let mut rscratch = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();

    let seeds = [
        Seed::None,
        Seed::ResetFast,
        Seed::Reset,
        Seed::InitStream,
        Seed::LoadDict(0),
        Seed::LoadDict(1),
        Seed::LoadDict(1000),
        Seed::LoadDict(65535),
        Seed::LoadDict(65536),
        Seed::LoadDict(100_000),
        Seed::LoadDictSlow(1),
        Seed::LoadDictSlow(1000),
        Seed::LoadDictSlow(65536),
        Seed::LoadDictSlow(100_000),
        Seed::AttachDict(1000),
        Seed::AttachDict(65536),
        Seed::AttachNull,
    ];

    for seed in seeds {
        for &shape in &SHAPES {
            for &tight in &[false, true] {
                for &accel in &[1i32, 0, 7] {
                    let dict = make_data(&mut rng, 100_000, shape);
                    let total = rng.range(1, 120_000);
                    let src = make_data(&mut rng, total, shape);
                    // random chunk sizes, straddling the 64K table boundary
                    let mut chunks = Vec::new();
                    let mut acc = 0usize;
                    while acc < total {
                        let c = match rng.below(6) {
                            0 => 1,
                            1 => rng.range(1, 64),
                            2 => rng.range(1, 4096),
                            3 => 65536,
                            4 => 65535,
                            _ => rng.range(1, 40_000),
                        };
                        chunks.push(c);
                        acc += c;
                    }
                    cscratch.fill0();
                    rscratch.fill0();
                    let (cb, cs, cn) = unsafe {
                        run_stream(
                            &co,
                            seed,
                            &dict,
                            &src,
                            &chunks,
                            accel,
                            tight,
                            cscratch.ptr(),
                            LZ4_STREAM_SIZE + 128,
                        )
                    };
                    let (rb, rs, rn) = unsafe {
                        run_stream(
                            &ro,
                            seed,
                            &dict,
                            &src,
                            &chunks,
                            accel,
                            tight,
                            rscratch.ptr(),
                            LZ4_STREAM_SIZE + 128,
                        )
                    };
                    let ctx = format!(
                        "stream seed={seed:?} shape={shape:?} tight={tight} accel={accel} total={total}"
                    );
                    eq(&format!("{ctx} nblocks"), cb.len(), rb.len());
                    for (i, (a, b)) in cb.iter().zip(rb.iter()).enumerate() {
                        eq_bytes(&format!("{ctx} block[{i}]"), a, b);
                    }
                    eq(&format!("{ctx} saveDict ret"), cn, rn);
                    eq_bytes(&format!("{ctx} saveDict"), &cs, &rs);
                }
            }
        }
    }
}

#[test]
fn row39_save_dict_sizes() {
    let (co, ro) = stream_ops();
    let mut rng = Rng::new(0xB10C_0039);
    for &shape in &SHAPES {
        for &maxdict in &[0i32, 1, 100, 4096, 65535, 65536, 100_000] {
            for _ in 0..4 {
                let total = rng.range(1, 90_000);
                let src = make_data(&mut rng, total, shape);
                let chunks = [total / 3 + 1, total / 3 + 1, total];
                let mut got = Vec::new();
                for o in [&co, &ro] {
                    unsafe {
                        let s = (o.create)();
                        let mut off = 0usize;
                        for &c in &chunks {
                            if off >= total {
                                break;
                            }
                            let c = c.min(total - off);
                            let bound = lz4_compress_bound(c as i32) as usize;
                            let mut d = vec![0u8; bound + 8];
                            (o.cont)(
                                s,
                                src.as_ptr().add(off),
                                d.as_mut_ptr(),
                                c as i32,
                                bound as i32,
                                1,
                            );
                            off += c;
                        }
                        let mut safe = vec![0x7Eu8; 120_000];
                        let n = (o.save)(s, safe.as_mut_ptr(), maxdict);
                        (o.free)(s);
                        safe.truncate(if n > 0 { n as usize } else { 0 });
                        got.push((n, safe));
                    }
                }
                let ctx = format!("saveDict shape={shape:?} maxdict={maxdict} total={total}");
                eq(&format!("{ctx} ret"), got[0].0, got[1].0);
                eq_bytes(&ctx, &got[0].1, &got[1].1);
            }
        }
    }
}

#[test]
fn row40_compress_force_ext_dict() {
    let (cc, rc) = sym::<FnForceExtDict>("LZ4_compress_forceExtDict");
    let (co, ro) = stream_ops();
    let mut rng = Rng::new(0xB10C_0040);
    for &shape in &SHAPES {
        for &dictlen in &[0usize, 1, 1000, 65536] {
            for _ in 0..4 {
                let dict = make_data(&mut rng, dictlen, shape);
                let len = rng.range(1, 30_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let mut got = Vec::new();
                for (o, f) in [(&co, &cc), (&ro, &rc)] {
                    unsafe {
                        let s = (o.create)();
                        (o.load)(s, dict.as_ptr(), dictlen as i32);
                        let mut d = vec![0xEEu8; bound + 16];
                        let n = f(s, src.as_ptr(), d.as_mut_ptr(), len as i32);
                        (o.free)(s);
                        got.push((n, d));
                    }
                }
                let ctx = format!("forceExtDict shape={shape:?} dict={dictlen} len={len}");
                eq(&format!("{ctx} ret"), got[0].0, got[1].0);
                eq_bytes(&ctx, &got[0].1, &got[1].1);
            }
        }
    }
}

#[test]
fn row41_42_obsolete_stream_lifecycle() {
    let (cc, rc) = sym::<FnCreate>("LZ4_create");
    let (crss, rrss) = sym::<FnResetStreamState>("LZ4_resetStreamState");
    let (csl, rsl) = sym::<FnSlide>("LZ4_slideInputBuffer");
    let (ccc, rcc) = sym::<FnCompressContinue>("LZ4_compress_continue");
    let (ccl, rcl) = sym::<FnCompressContinueLim>("LZ4_compress_limitedOutput_continue");
    let (cfree, rfree) = sym::<FnFreeStream>("LZ4_freeStream");
    let (csss, rsss) = sym::<FnVoidI32>("LZ4_sizeofStreamState");
    eq("sizeofStreamState", unsafe { csss() }, unsafe { rsss() });

    let mut rng = Rng::new(0xB10C_0041);
    for &shape in &SHAPES {
        for _ in 0..5 {
            let total = rng.range(1, 60_000);
            let src = make_data(&mut rng, total, shape);
            let nchunks = rng.range(1, 6);
            let chunk = total / nchunks + 1;

            // row 41: LZ4_create + slideInputBuffer + free
            let mut got = Vec::new();
            for (create, cont, slide, free) in
                [(&cc, &ccc, &csl, &cfree), (&rc, &rcc, &rsl, &rfree)]
            {
                unsafe {
                    // LZ4_create takes an input buffer; the legacy API keeps src in it.
                    let mut inbuf = vec![0u8; 65536 + chunk + 16];
                    let s = create(inbuf.as_mut_ptr());
                    assert!(!s.is_null());
                    let mut blocks = Vec::new();
                    let mut off = 0usize;
                    let mut cursor = 0usize;
                    while off < total {
                        let c = chunk.min(total - off);
                        if cursor + c > inbuf.len() {
                            let p = slide(s);
                            assert!(!p.is_null());
                            cursor = p.offset_from(inbuf.as_ptr()) as usize;
                        }
                        inbuf[cursor..cursor + c].copy_from_slice(&src[off..off + c]);
                        let bound = lz4_compress_bound(c as i32) as usize;
                        let mut d = vec![0u8; bound + 8];
                        let n = cont(
                            s,
                            inbuf.as_ptr().add(cursor),
                            d.as_mut_ptr(),
                            c as i32,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        blocks.push((n, d));
                        cursor += c;
                        off += c;
                    }
                    free(s);
                    got.push(blocks);
                }
            }
            let ctx = format!("LZ4_create legacy shape={shape:?} total={total}");
            eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
            for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
            }

            // row 42: resetStreamState + compress_limitedOutput_continue
            let mut got = Vec::new();
            for (reset, cont) in [(&crss, &ccl), (&rrss, &rcl)] {
                let mut st = Aligned::<{ LZ4_STREAM_SIZE + 128 }>::new();
                let mut inbuf = vec![0u8; total + 16];
                inbuf[..total].copy_from_slice(&src);
                unsafe {
                    let rr = reset(st.ptr(), inbuf.as_mut_ptr());
                    let mut blocks = vec![(rr, Vec::new())];
                    let mut off = 0usize;
                    while off < total {
                        let c = chunk.min(total - off);
                        let bound = lz4_compress_bound(c as i32) as usize;
                        let mut d = vec![0u8; bound + 8];
                        let n = cont(
                            st.ptr(),
                            inbuf.as_ptr().add(off),
                            d.as_mut_ptr(),
                            c as i32,
                            bound as i32,
                        );
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        blocks.push((n, d));
                        off += c;
                    }
                    got.push(blocks);
                }
            }
            let ctx = format!("resetStreamState legacy shape={shape:?} total={total}");
            eq(&format!("{ctx} nblocks"), got[0].len(), got[1].len());
            for (i, (a, b)) in got[0].iter().zip(got[1].iter()).enumerate() {
                eq(&format!("{ctx} ret[{i}]"), a.0, b.0);
                eq_bytes(&format!("{ctx} block[{i}]"), &a.1, &b.1);
            }
        }
    }
}

#[test]
fn row43_45_streaming_decompress() {
    let (ccsd, rcsd) = sym::<FnCreateStream>("LZ4_createStreamDecode");
    let (cfsd, rfsd) = sym::<FnFreeStream>("LZ4_freeStreamDecode");
    let (cssd, rssd) = sym::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let (cdc, rdc) = sym::<FnDecContinue>("LZ4_decompress_safe_continue");
    let (cdf, rdf) = sym::<FnDecFastContinue>("LZ4_decompress_fast_continue");
    let (co, _ro) = stream_ops();

    let mut rng = Rng::new(0xB10C_0043);
    for &shape in &SHAPES {
        for &dictlen in &[0usize, 1000, 65536] {
            for _ in 0..5 {
                let dict = make_data(&mut rng, dictlen, shape);
                let total = rng.range(1, 80_000);
                let src = make_data(&mut rng, total, shape);
                let nchunks = rng.range(1, 8);
                let chunk = total / nchunks + 1;

                // Compress with the C implementation (blockLinked stream).
                let mut blocks: Vec<(usize, Vec<u8>)> = Vec::new();
                unsafe {
                    let s = (co.create)();
                    if dictlen > 0 {
                        (co.load)(s, dict.as_ptr(), dictlen as i32);
                    }
                    let mut off = 0usize;
                    while off < total {
                        let c = chunk.min(total - off);
                        let bound = lz4_compress_bound(c as i32) as usize;
                        let mut d = vec![0u8; bound + 8];
                        let n = (co.cont)(
                            s,
                            src.as_ptr().add(off),
                            d.as_mut_ptr(),
                            c as i32,
                            bound as i32,
                            1,
                        );
                        assert!(n > 0);
                        d.truncate(n as usize);
                        blocks.push((c, d));
                        off += c;
                    }
                    (co.free)(s);
                }

                // Decode with a linear output buffer (valid for blockLinked).
                for &use_fast in &[false, true] {
                    let mut got = Vec::new();
                    for (create, free, set, dc, df) in [
                        (&ccsd, &cfsd, &cssd, &cdc, &cdf),
                        (&rcsd, &rfsd, &rssd, &rdc, &rdf),
                    ] {
                        unsafe {
                            let sd = create();
                            assert!(!sd.is_null());
                            let sr = if dictlen > 0 {
                                set(sd, dict.as_ptr(), dictlen as i32)
                            } else {
                                set(sd, std::ptr::null(), 0)
                            };
                            let mut out = vec![0x5Au8; total + 128];
                            let mut off = 0usize;
                            let mut rets = vec![sr];
                            for (plain, blk) in &blocks {
                                let n = if use_fast {
                                    df(
                                        sd,
                                        blk.as_ptr(),
                                        out.as_mut_ptr().add(off),
                                        *plain as i32,
                                    )
                                } else {
                                    dc(
                                        sd,
                                        blk.as_ptr(),
                                        out.as_mut_ptr().add(off),
                                        blk.len() as i32,
                                        *plain as i32,
                                    )
                                };
                                rets.push(n);
                                if n <= 0 {
                                    break;
                                }
                                // `_fast_continue` returns bytes READ from src;
                                // `_safe_continue` returns bytes WRITTEN to dst.
                                off += if use_fast { *plain } else { n as usize };
                            }
                            free(sd);
                            out.truncate(off);
                            got.push((rets, out));
                        }
                    }
                    let ctx = format!(
                        "dec_continue fast={use_fast} shape={shape:?} dict={dictlen} total={total}"
                    );
                    eq(&format!("{ctx} rets"), &got[0].0, &got[1].0);
                    eq_bytes(&ctx, &got[0].1, &got[1].1);
                    eq_bytes(&format!("{ctx} roundtrip"), &got[0].1, &src);
                }
            }
        }
    }
}

#[test]
fn row36_load_dict_internal_export() {
    // LZ4_loadDict_internal is exported by the C .so; exercise both modes.
    let (c, r) = sym::<FnLoadDictInternal>("LZ4_loadDict_internal");
    let (co, ro) = stream_ops();
    let mut rng = Rng::new(0xB10C_0036);
    for &shape in &SHAPES {
        for &dictlen in &[0usize, 1, 12, 1000, 65536, 90_000] {
            for mode in [0i32, 1] {
                let dict = make_data(&mut rng, dictlen.max(1), shape);
                let len = rng.range(1, 20_000);
                let src = make_data(&mut rng, len, shape);
                let bound = lz4_compress_bound(len as i32) as usize;
                let mut got = Vec::new();
                for (o, f) in [(&co, &c), (&ro, &r)] {
                    unsafe {
                        let s = (o.create)();
                        let ld = f(s, dict.as_ptr(), dictlen as i32, mode);
                        let mut d = vec![0u8; bound + 8];
                        let n = (o.cont)(
                            s,
                            src.as_ptr(),
                            d.as_mut_ptr(),
                            len as i32,
                            bound as i32,
                            1,
                        );
                        (o.free)(s);
                        d.truncate(if n > 0 { n as usize } else { 0 });
                        got.push((ld, n, d));
                    }
                }
                let ctx = format!("loadDict_internal mode={mode} dict={dictlen} shape={shape:?}");
                eq(&format!("{ctx} loadRet"), got[0].0, got[1].0);
                eq(&format!("{ctx} compRet"), got[0].1, got[1].1);
                eq_bytes(&ctx, &got[0].2, &got[1].2);
            }
        }
    }
}
