//! CONFIGS.md rows 12-47 — lz4.c block-compression valid-path parity.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and compares the return code AND the produced bytes.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnSizeof = unsafe extern "C" fn() -> c_int;
type FnDefault = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnFastExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDestSize = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnDestSizeExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FnCompress3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnResetStream = unsafe extern "C" fn(*mut c_void);
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnContinue4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnContinue5 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnDecSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;

/// Verify that a compressed block produced by BOTH libraries decompresses back
/// to the original through the C decoder (round-trip sanity on top of parity).
fn check_roundtrip(dec: &FnDecSafe, comp: &[u8], orig: &[u8], ctx: &str) {
    let mut out = vec![0u8; orig.len() + 64];
    let n = unsafe {
        dec(
            comp.as_ptr() as *const c_char,
            out.as_mut_ptr() as *mut c_char,
            comp.len() as c_int,
            orig.len() as c_int,
        )
    };
    assert_eq!(n, orig.len() as c_int, "{ctx}: round-trip size mismatch");
    assert_bytes_eq(&out[..orig.len()], orig, &format!("{ctx}: round-trip data"));
}

// ---------------------------------------------------------------------------
// Rows 30-31 — pure functions / constants
// ---------------------------------------------------------------------------
#[test]
fn rows30_31_bound_sizeof_version() {
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(sos, "LZ4_sizeofState", FnSizeof);
    sym!(soss, "LZ4_sizeofStreamState", FnSizeof);
    sym!(vn, "LZ4_versionNumber", FnSizeof);
    sym!(
        vs,
        "LZ4_versionString",
        unsafe extern "C" fn() -> *const c_char
    );
    sym!(drbs, "LZ4_decoderRingBufferSize", FnBound);

    unsafe {
        assert_ret_eq(sos.0(), sos.1(), "LZ4_sizeofState");
        assert_ret_eq(soss.0(), soss.1(), "LZ4_sizeofStreamState");
        assert_ret_eq(vn.0(), vn.1(), "LZ4_versionNumber");
        let (cs, rs) = (
            std::ffi::CStr::from_ptr(vs.0()),
            std::ffi::CStr::from_ptr(vs.1()),
        );
        assert_eq!(cs, rs, "LZ4_versionString");

        let mut sizes: Vec<c_int> = vec![
            i32::MIN,
            -1,
            0,
            1,
            2,
            15,
            16,
            17,
            64,
            1024,
            65535,
            65536,
            LZ4_MAX_INPUT_SIZE - 1,
            LZ4_MAX_INPUT_SIZE,
            LZ4_MAX_INPUT_SIZE + 1,
            i32::MAX,
        ];
        let mut rng = Rng::new(0xB0_0001);
        for _ in 0..2000 {
            sizes.push(rng.next_u32() as c_int);
        }
        for &s in &sizes {
            assert_ret_eq(bound.0(s), bound.1(s), &format!("LZ4_compressBound({s})"));
            assert_ret_eq(
                drbs.0(s),
                drbs.1(s),
                &format!("LZ4_decoderRingBufferSize({s})"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-17 — LZ4_compress_default across shapes, tableTypes and sizes
// ---------------------------------------------------------------------------
#[test]
fn rows12_17_compress_default_shapes_and_sizes() {
    sym!(cd, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xB0_0012);

    for &len in KEY_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_data(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(1) as usize;
            let mut cb = vec![0u8; cap + 16];
            let mut rb = vec![0u8; cap + 16];
            let (cn, rn) = unsafe {
                (
                    cd.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    ),
                    cd.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    ),
                )
            };
            let ctx = format!("compress_default len={len} shape={shape:?}");
            assert_out_eq(cn, &cb, rn, &rb, &ctx);
            if cn > 0 {
                check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 18, 20 — limitedOutput: dstCapacity swept below the bound
// ---------------------------------------------------------------------------
#[test]
fn rows18_20_limited_output_capacity_sweep() {
    sym!(cd, "LZ4_compress_default", FnDefault);
    sym!(cf, "LZ4_compress_fast", FnFast);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0018);

    for &len in &[13usize, 64, 300, 1024, 4096, 65536, 65547, 100_000] {
        for &shape in ALL_SHAPES {
            let src = gen_data(shape, len, &mut rng);
            let full = unsafe { bound.0(len as c_int) }.max(1) as usize;
            // Determine the natural compressed size, then sweep around it.
            let mut probe = vec![0u8; full + 16];
            let natural = unsafe {
                cd.0(
                    src.as_ptr() as *const c_char,
                    probe.as_mut_ptr() as *mut c_char,
                    len as c_int,
                    full as c_int,
                )
            };
            let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 5];
            if natural > 0 {
                let n = natural as usize;
                for d in [0usize, 1, 2, 3, 5, 9, 17] {
                    if n >= d {
                        caps.push(n - d);
                    }
                    caps.push(n + d);
                }
                caps.push(n / 2);
                caps.push(n / 4);
                caps.push(n * 3 / 4);
            }
            for _ in 0..12 {
                caps.push(rng.range(0, full));
            }
            caps.sort_unstable();
            caps.dedup();

            for &cap in &caps {
                let mut cb = vec![0u8; cap + 32];
                let mut rb = vec![0u8; cap + 32];
                let (cn, rn) = unsafe {
                    (
                        cd.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                        cd.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                    )
                };
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("limited default len={len} {shape:?} cap={cap}"),
                );

                for &acc in &[1i32, 3, 64, 65537] {
                    let mut cb = vec![0u8; cap + 32];
                    let mut rb = vec![0u8; cap + 32];
                    let (cn, rn) = unsafe {
                        (
                            cf.0(
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                            cf.1(
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                        )
                    };
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("limited fast len={len} {shape:?} cap={cap} acc={acc}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — acceleration sweep (incl. the clamp boundaries)
// ---------------------------------------------------------------------------
#[test]
fn row19_acceleration_sweep() {
    sym!(cf, "LZ4_compress_fast", FnFast);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xB0_0019);
    let accs: [c_int; 14] = [
        i32::MIN,
        -1,
        0,
        1,
        2,
        3,
        7,
        17,
        64,
        1000,
        65536,
        65537,
        65538,
        i32::MAX,
    ];

    for &len in &[0usize, 1, 13, 64, 1024, 4096, 65535, 65536, 65547, 100_000] {
        for &shape in ALL_SHAPES {
            let src = gen_data(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(1) as usize;
            for &acc in &accs {
                let mut cb = vec![0u8; cap + 16];
                let mut rb = vec![0u8; cap + 16];
                let (cn, rn) = unsafe {
                    (
                        cf.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            acc,
                        ),
                        cf.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            acc,
                        ),
                    )
                };
                let ctx = format!("compress_fast len={len} {shape:?} acc={acc}");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                if cn > 0 {
                    check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 21-23 — caller-owned state (extState / extState_fastReset), incl. reuse
// ---------------------------------------------------------------------------
#[test]
fn rows21_23_ext_state() {
    sym!(ce, "LZ4_compress_fast_extState", FnFastExt);
    sym!(cr, "LZ4_compress_fast_extState_fastReset", FnFastExt);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0021);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAM_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAM_T);
    // Separate states for the reuse (fast-reset) sequence.
    let mut cst2 = Aligned::new(SIZEOF_LZ4_STREAM_T);
    let mut rst2 = Aligned::new(SIZEOF_LZ4_STREAM_T);

    for round in 0..3 {
        for &len in &[0usize, 1, 13, 64, 1024, 4096, 65535, 65536, 65547, 100_000] {
            for &shape in ALL_SHAPES {
                let src = gen_data(shape, len, &mut rng);
                let cap = unsafe { bound.0(len as c_int) }.max(1) as usize;
                for &acc in &[1i32, 0, 5, 65537] {
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = unsafe {
                        (
                            ce.0(
                                cst.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                            ce.1(
                                rst.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                        )
                    };
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("extState r={round} len={len} {shape:?} acc={acc}"),
                    );

                    // fastReset path: the SAME state is reused across every
                    // iteration, so currentOffset keeps growing (row 23).
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = unsafe {
                        (
                            cr.0(
                                cst2.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                            cr.1(
                                rst2.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                acc,
                            ),
                        )
                    };
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("extState_fastReset r={round} len={len} {shape:?} acc={acc}"),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26 — LZ4_compress_destSize / _destSize_extState (fillOutput)
// ---------------------------------------------------------------------------
#[test]
fn rows24_26_dest_size() {
    sym!(ds, "LZ4_compress_destSize", FnDestSize);
    sym!(dse, "LZ4_compress_destSize_extState", FnDestSizeExt);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xB0_0024);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAM_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAM_T);

    for &len in &[0usize, 1, 13, 64, 300, 1024, 4096, 65535, 65536, 65547, 100_000] {
        for &shape in ALL_SHAPES {
            let src = gen_data(shape, len, &mut rng);
            let full = unsafe { bound.0(len as c_int) }.max(1) as usize;
            let mut targets: Vec<usize> = vec![0, 1, 2, 3, 4, 5, 10, 17, full];
            for f in [1usize, 2, 4, 8, 16] {
                targets.push(full / f);
            }
            for _ in 0..10 {
                targets.push(rng.range(0, full));
            }
            targets.sort_unstable();
            targets.dedup();

            for &tgt in &targets {
                // one-shot
                let mut csz = len as c_int;
                let mut rsz = len as c_int;
                let mut cb = vec![0u8; tgt + 32];
                let mut rb = vec![0u8; tgt + 32];
                let (cn, rn) = unsafe {
                    (
                        ds.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            &mut csz,
                            tgt as c_int,
                        ),
                        ds.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            &mut rsz,
                            tgt as c_int,
                        ),
                    )
                };
                let ctx = format!("destSize len={len} {shape:?} tgt={tgt}");
                assert_eq!(csz, rsz, "{ctx}: *srcSizePtr mismatch");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                if cn > 0 {
                    assert!(cn as usize <= tgt, "{ctx}: exceeded targetDstSize");
                    check_roundtrip(&dec.0, &cb[..cn as usize], &src[..csz as usize], &ctx);
                }

                // ext-state variant, with an acceleration sweep
                for &acc in &[1i32, 0, 7, 65537] {
                    let mut csz = len as c_int;
                    let mut rsz = len as c_int;
                    let mut cb = vec![0u8; tgt + 32];
                    let mut rb = vec![0u8; tgt + 32];
                    let (cn, rn) = unsafe {
                        (
                            dse.0(
                                cst.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut csz,
                                tgt as c_int,
                                acc,
                            ),
                            dse.1(
                                rst.ptr() as *mut c_void,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rsz,
                                tgt as c_int,
                                acc,
                            ),
                        )
                    };
                    let ctx = format!("destSize_extState len={len} {shape:?} tgt={tgt} acc={acc}");
                    assert_eq!(csz, rsz, "{ctx}: *srcSizePtr mismatch");
                    assert_out_eq(cn, &cb, rn, &rb, &ctx);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-29 — deprecated one-shot wrappers
// ---------------------------------------------------------------------------
#[test]
fn rows27_29_deprecated_oneshot() {
    sym!(c3, "LZ4_compress", FnCompress3);
    sym!(clo, "LZ4_compress_limitedOutput", FnDefault);
    sym!(cws, "LZ4_compress_withState", FnFastExt);
    sym!(clows, "LZ4_compress_limitedOutput_withState", FnFastExt);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0027);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAM_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAM_T);

    for &len in &[0usize, 1, 13, 64, 1024, 4096, 65535, 65536, 65547, 100_000] {
        for &shape in ALL_SHAPES {
            let src = gen_data(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(1) as usize;

            // LZ4_compress: notLimited, dst must be >= bound.
            let mut cb = vec![0u8; cap + 16];
            let mut rb = vec![0u8; cap + 16];
            let (cn, rn) = unsafe {
                (
                    c3.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                    c3.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                )
            };
            assert_out_eq(cn, &cb, rn, &rb, &format!("LZ4_compress len={len} {shape:?}"));

            // LZ4_compress_withState (notLimited, needs cap >= bound)
            let mut cb = vec![0u8; cap + 16];
            let mut rb = vec![0u8; cap + 16];
            let (cn, rn) = unsafe {
                (
                    cws.0(
                        cst.ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        0,
                        0,
                    ),
                    cws.1(
                        rst.ptr() as *mut c_void,
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        0,
                        0,
                    ),
                )
            };
            assert_out_eq(
                cn,
                &cb,
                rn,
                &rb,
                &format!("LZ4_compress_withState len={len} {shape:?}"),
            );

            // limitedOutput variants across a capacity sweep
            let mut caps: Vec<usize> = vec![0, 1, 2, 5, cap];
            for f in [2usize, 4, 8] {
                caps.push(cap / f);
            }
            caps.sort_unstable();
            caps.dedup();
            for &c in &caps {
                let mut cb = vec![0u8; c + 32];
                let mut rb = vec![0u8; c + 32];
                let (cn, rn) = unsafe {
                    (
                        clo.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            c as c_int,
                        ),
                        clo.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            c as c_int,
                        ),
                    )
                };
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("LZ4_compress_limitedOutput len={len} {shape:?} cap={c}"),
                );

                let mut cb = vec![0u8; c + 32];
                let mut rb = vec![0u8; c + 32];
                let (cn, rn) = unsafe {
                    (
                        clows.0(
                            cst.ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            c as c_int,
                            0,
                        ),
                        clows.1(
                            rst.ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            c as c_int,
                            0,
                        ),
                    )
                };
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("limitedOutput_withState len={len} {shape:?} cap={c}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 32-35 — contiguous-prefix streaming chains
// ---------------------------------------------------------------------------
#[test]
fn rows32_35_prefix_streaming() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0xB0_0032);

    // Uniform block sizes (row 32) + random block sizes (row 33) + sizes that
    // keep the prefix under 64 KB so `dictSmall` engages (row 34) + chains that
    // cross 64 KB so `withPrefix64k` engages (row 35).
    let plans: &[(&str, usize, usize)] = &[
        ("uniform-1k", 1024, 40),
        ("uniform-64", 64, 60),
        ("uniform-4k", 4096, 40),
        ("uniform-70k", 70_000, 6),
        ("dictSmall-100", 100, 80),
        ("cross64k-9k", 9000, 20),
    ];

    for &(name, blk, nblocks) in plans {
        for &shape in ALL_SHAPES {
            let total = blk * nblocks;
            let data = gen_data(shape, total, &mut rng);
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                assert!(!csp.is_null() && !rsp.is_null());
                for i in 0..nblocks {
                    let off = i * blk;
                    let n = blk.min(total - off);
                    let cap = bound.0(n as c_int).max(1) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            csp,
                            data[off..].as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rsp,
                            data[off..].as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("prefix chain {name} {shape:?} block {i}"),
                    );
                    // Decode with the preceding bytes as the dictionary.
                    if cn > 0 {
                        let mut out = vec![0u8; n + 32];
                        let d = dec.0(
                            cb.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            cn,
                            n as c_int,
                            data.as_ptr() as *const c_char,
                            off as c_int,
                        );
                        assert_eq!(d, n as c_int, "{name} {shape:?} block {i}: decode size");
                        assert_bytes_eq(
                            &out[..n],
                            &data[off..off + n],
                            &format!("{name} {shape:?} block {i}: decoded data"),
                        );
                    }
                }
                assert_ret_eq(fs.0(csp), fs.1(rsp), "LZ4_freeStream");
            }
        }
    }

    // Random block sizes (row 33)
    for &shape in ALL_SHAPES {
        let total = 250_000usize;
        let data = gen_data(shape, total, &mut rng);
        unsafe {
            let (csp, rsp) = (cs.0(), cs.1());
            let mut off = 0usize;
            let mut i = 0;
            while off < total {
                let n = rng.range(1, 20_000).min(total - off);
                let cap = bound.0(n as c_int).max(1) as usize;
                let mut cb = vec![0u8; cap + 16];
                let mut rb = vec![0u8; cap + 16];
                let acc = *[1i32, 2, 9].get(rng.below(3)).unwrap();
                let (cn, rn) = (
                    cont.0(
                        csp,
                        data[off..].as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        acc,
                    ),
                    cont.1(
                        rsp,
                        data[off..].as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        cap as c_int,
                        acc,
                    ),
                );
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("random prefix chain {shape:?} block {i} n={n} acc={acc}"),
                );
                off += n;
                i += 1;
            }
            fs.0(csp);
            fs.1(rsp);
        }
    }
}

type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Row 36 — non-contiguous blocks => usingExtDict
// ---------------------------------------------------------------------------
#[test]
fn row36_ext_dict_streaming() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0036);

    for &shape in ALL_SHAPES {
        for &blk in &[64usize, 1024, 9000, 70_000] {
            // Each block lives in its OWN allocation => guaranteed non-contiguous.
            let blocks: Vec<Vec<u8>> = (0..12).map(|_| gen_data(shape, blk, &mut rng)).collect();
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                for (i, b) in blocks.iter().enumerate() {
                    let cap = bound.0(blk as c_int).max(1) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            csp,
                            b.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rsp,
                            b.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("extDict chain {shape:?} blk={blk} i={i}"),
                    );
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 37 — ring-buffer source
// ---------------------------------------------------------------------------
#[test]
fn row37_ring_buffer_source() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(drbs, "LZ4_decoderRingBufferSize", FnBound);
    let mut rng = Rng::new(0xB0_0037);

    for &blk in &[1024usize, 4096, 9000] {
        let ring_sz = unsafe { drbs.0(blk as c_int) } as usize;
        assert!(ring_sz > 0);
        for &shape in ALL_SHAPES {
            let mut cring = vec![0u8; ring_sz];
            let mut rring = vec![0u8; ring_sz];
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                let mut pos = 0usize;
                for i in 0..40 {
                    if pos + blk > ring_sz {
                        pos = 0;
                    }
                    let chunk = gen_data(shape, blk, &mut rng);
                    cring[pos..pos + blk].copy_from_slice(&chunk);
                    rring[pos..pos + blk].copy_from_slice(&chunk);
                    let cap = bound.0(blk as c_int).max(1) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            csp,
                            cring[pos..].as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rsp,
                            rring[pos..].as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("ring src {shape:?} blk={blk} i={i} pos={pos}"),
                    );
                    pos += blk;
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 38-39 — LZ4_loadDict / LZ4_loadDictSlow + continue
// ---------------------------------------------------------------------------
#[test]
fn rows38_39_load_dict() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(lds, "LZ4_loadDictSlow", FnLoadDict);
    sym!(ldi, "LZ4_loadDict_internal", FnLoadDictInternal);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0038);

    let dict_sizes = [0usize, 1, 4, 7, 8, 9, 64, 1024, 65535, 65536, 70_000];
    for &dsz in &dict_sizes {
        for &shape in ALL_SHAPES {
            let dict = gen_data(shape, dsz, &mut rng);
            for (which, f) in [("loadDict", &ld), ("loadDictSlow", &lds)] {
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    let (cd, rd) = (
                        f.0(csp, dict.as_ptr() as *const c_char, dsz as c_int),
                        f.1(rsp, dict.as_ptr() as *const c_char, dsz as c_int),
                    );
                    assert_ret_eq(cd, rd, &format!("{which} dsz={dsz} {shape:?}"));
                    for &blk in &[64usize, 1024, 9000] {
                        for i in 0..6 {
                            let src = gen_data(shape, blk, &mut rng);
                            let cap = bound.0(blk as c_int).max(1) as usize;
                            let mut cb = vec![0u8; cap + 16];
                            let mut rb = vec![0u8; cap + 16];
                            let (cn, rn) = (
                                cont.0(
                                    csp,
                                    src.as_ptr() as *const c_char,
                                    cb.as_mut_ptr() as *mut c_char,
                                    blk as c_int,
                                    cap as c_int,
                                    1,
                                ),
                                cont.1(
                                    rsp,
                                    src.as_ptr() as *const c_char,
                                    rb.as_mut_ptr() as *mut c_char,
                                    blk as c_int,
                                    cap as c_int,
                                    1,
                                ),
                            );
                            assert_out_eq(
                                cn,
                                &cb,
                                rn,
                                &rb,
                                &format!("{which} dsz={dsz} {shape:?} blk={blk} i={i}"),
                            );
                        }
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
            // LZ4_loadDict_internal is exported too: exercise both modes.
            for mode in 0..2i32 {
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    let (cd, rd) = (
                        ldi.0(csp, dict.as_ptr() as *const c_char, dsz as c_int, mode),
                        ldi.1(rsp, dict.as_ptr() as *const c_char, dsz as c_int, mode),
                    );
                    assert_ret_eq(
                        cd,
                        rd,
                        &format!("loadDict_internal mode={mode} dsz={dsz} {shape:?}"),
                    );
                    let src = gen_data(shape, 4096, &mut rng);
                    let cap = bound.0(4096).max(1) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            csp,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            4096,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rsp,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            4096,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("loadDict_internal mode={mode} dsz={dsz} {shape:?} compress"),
                    );
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

type FnLoadDictInternal = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Rows 40-41 — LZ4_attach_dictionary (usingDictCtx), below/above the 4096 cutoff
// ---------------------------------------------------------------------------
#[test]
fn rows40_41_attach_dictionary() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(at, "LZ4_attach_dictionary", FnAttach);
    sym!(rsf, "LZ4_resetStream_fast", FnResetStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0040);

    for &dsz in &[0usize, 8, 64, 1024, 65536, 70_000] {
        for &shape in ALL_SHAPES {
            let dict = gen_data(shape, dsz, &mut rng);
            unsafe {
                let (cdict, rdict) = (cs.0(), cs.1());
                let (cw, rw) = (cs.0(), cs.1());
                assert_ret_eq(
                    ld.0(cdict, dict.as_ptr() as *const c_char, dsz as c_int),
                    ld.1(rdict, dict.as_ptr() as *const c_char, dsz as c_int),
                    "loadDict for attach",
                );
                // srcSize <= 4096 (row 40) and > 4096 (bulk table copy, row 41)
                for &blk in &[64usize, 1024, 4096, 4097, 9000, 70_000] {
                    rsf.0(cw);
                    rsf.1(rw);
                    at.0(cw, cdict);
                    at.1(rw, rdict);
                    let src = gen_data(shape, blk, &mut rng);
                    let cap = bound.0(blk as c_int).max(1) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            cw,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rw,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("attach_dictionary dsz={dsz} {shape:?} blk={blk}"),
                    );
                    // A second block continues from the working prefix.
                    let src2 = gen_data(shape, blk, &mut rng);
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn, rn) = (
                        cont.0(
                            cw,
                            src2.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rw,
                            src2.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("attach_dictionary 2nd dsz={dsz} {shape:?} blk={blk}"),
                    );
                }
                // Detach (NULL dictionaryStream) — ERRORS.md row 64.
                rsf.0(cw);
                rsf.1(rw);
                at.0(cw, std::ptr::null());
                at.1(rw, std::ptr::null());
                let src = gen_data(shape, 2048, &mut rng);
                let cap = bound.0(2048) as usize;
                let mut cb = vec![0u8; cap + 16];
                let mut rb = vec![0u8; cap + 16];
                let (cn, rn) = (
                    cont.0(
                        cw,
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        2048,
                        cap as c_int,
                        1,
                    ),
                    cont.1(
                        rw,
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        2048,
                        cap as c_int,
                        1,
                    ),
                );
                assert_out_eq(cn, &cb, rn, &rb, &format!("detach dsz={dsz} {shape:?}"));
                for p in [cdict, cw] {
                    fs.0(p);
                }
                for p in [rdict, rw] {
                    fs.1(p);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 42 — LZ4_saveDict
// ---------------------------------------------------------------------------
#[test]
fn row42_save_dict() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(sd, "LZ4_saveDict", FnSaveDict);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0042);

    for &shape in ALL_SHAPES {
        for &pre in &[0usize, 64, 1024, 40_000, 100_000] {
            for &want in &[0i32, 1, 4, 64, 1024, 65536, 70_000, -1] {
                // NOTE: `data` must outlive the saveDict call — the stream keeps
                // a raw `dictionary` pointer into it.
                let data = gen_data(shape, pre, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    if pre > 0 {
                        let cap = bound.0(pre as c_int) as usize;
                        let mut cb = vec![0u8; cap + 16];
                        let mut rb = vec![0u8; cap + 16];
                        let (cn, rn) = (
                            cont.0(
                                csp,
                                data.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                pre as c_int,
                                cap as c_int,
                                1,
                            ),
                            cont.1(
                                rsp,
                                data.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                pre as c_int,
                                cap as c_int,
                                1,
                            ),
                        );
                        assert_out_eq(cn, &cb, rn, &rb, "saveDict setup compress");
                    }
                    let mut csafe = vec![0u8; 80_000];
                    let mut rsafe = vec![0u8; 80_000];
                    let (cn, rn) = (
                        sd.0(csp, csafe.as_mut_ptr() as *mut c_char, want),
                        sd.1(rsp, rsafe.as_mut_ptr() as *mut c_char, want),
                    );
                    let ctx = format!("saveDict {shape:?} pre={pre} want={want}");
                    assert_ret_eq(cn, rn, &ctx);
                    assert!(cn >= 0, "{ctx}: saveDict returned {cn}");
                    assert_bytes_eq(&csafe[..cn as usize], &rsafe[..cn as usize], &ctx);

                    // The stream must remain usable, and identically so.
                    let nxt = gen_data(shape, 3000, &mut rng);
                    let cap = bound.0(3000) as usize;
                    let mut cb = vec![0u8; cap + 16];
                    let mut rb = vec![0u8; cap + 16];
                    let (cn2, rn2) = (
                        cont.0(
                            csp,
                            nxt.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            3000,
                            cap as c_int,
                            1,
                        ),
                        cont.1(
                            rsp,
                            nxt.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            3000,
                            cap as c_int,
                            1,
                        ),
                    );
                    assert_out_eq(cn2, &cb, rn2, &rb, &format!("{ctx}: after-save compress"));
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 43 — reset paths must be equivalent to a fresh stream
// ---------------------------------------------------------------------------
#[test]
fn row43_reset_paths() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(rst, "LZ4_resetStream", FnResetStream);
    sym!(rstf, "LZ4_resetStream_fast", FnResetStream);
    sym!(init, "LZ4_initStream", FnInitStream);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0043);

    for &shape in ALL_SHAPES {
        let a = gen_data(shape, 5000, &mut rng);
        let b = gen_data(shape, 5000, &mut rng);
        let cap = unsafe { bound.0(5000) } as usize;

        // Reference: fresh stream compressing `b` alone.
        let mut refbuf = vec![0u8; cap + 16];
        let refn = unsafe {
            let s = cs.0();
            let n = cont.0(
                s,
                b.as_ptr() as *const c_char,
                refbuf.as_mut_ptr() as *mut c_char,
                5000,
                cap as c_int,
                1,
            );
            fs.0(s);
            n
        };

        for which in 0..3 {
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                // Dirty both streams with `a`.
                let mut t = vec![0u8; cap + 16];
                cont.0(
                    csp,
                    a.as_ptr() as *const c_char,
                    t.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    1,
                );
                cont.1(
                    rsp,
                    a.as_ptr() as *const c_char,
                    t.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    1,
                );
                match which {
                    0 => {
                        rst.0(csp);
                        rst.1(rsp);
                    }
                    1 => {
                        rstf.0(csp);
                        rstf.1(rsp);
                    }
                    _ => {
                        assert!(!init.0(csp, SIZEOF_LZ4_STREAM_T).is_null());
                        assert!(!init.1(rsp, SIZEOF_LZ4_STREAM_T).is_null());
                    }
                }
                let mut cb = vec![0u8; cap + 16];
                let mut rb = vec![0u8; cap + 16];
                let (cn, rn) = (
                    cont.0(
                        csp,
                        b.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        5000,
                        cap as c_int,
                        1,
                    ),
                    cont.1(
                        rsp,
                        b.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        5000,
                        cap as c_int,
                        1,
                    ),
                );
                let ctx = format!("reset which={which} {shape:?}");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                // resetStream / initStream must fully restore the fresh state.
                if which != 1 {
                    assert_eq!(cn, refn, "{ctx}: not equivalent to a fresh stream");
                    assert_bytes_eq(
                        &cb[..cn as usize],
                        &refbuf[..refn as usize],
                        &format!("{ctx}: fresh-stream bytes"),
                    );
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }

        // LZ4_initStream on caller memory (valid boundary: exactly sizeof, aligned)
        unsafe {
            let mut ca = Aligned::new(SIZEOF_LZ4_STREAM_T);
            let mut ra = Aligned::new(SIZEOF_LZ4_STREAM_T);
            let cp = init.0(ca.ptr() as *mut c_void, SIZEOF_LZ4_STREAM_T);
            let rp = init.1(ra.ptr() as *mut c_void, SIZEOF_LZ4_STREAM_T);
            assert!(!cp.is_null() && !rp.is_null(), "initStream boundary");
            let mut cb = vec![0u8; cap + 16];
            let mut rb = vec![0u8; cap + 16];
            let (cn, rn) = (
                cont.0(
                    cp,
                    b.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    1,
                ),
                cont.1(
                    rp,
                    b.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    1,
                ),
            );
            assert_out_eq(cn, &cb, rn, &rb, &format!("initStream own mem {shape:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 44, 46 — deprecated streaming lifecycle (LZ4_create / slideInputBuffer)
// ---------------------------------------------------------------------------
#[test]
fn rows44_46_deprecated_streaming() {
    sym!(create, "LZ4_create", FnCreateChar);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(cc, "LZ4_compress_continue", FnContinue4);
    sym!(clc, "LZ4_compress_limitedOutput_continue", FnContinue5);
    sym!(slide, "LZ4_slideInputBuffer", FnSlide);
    sym!(rss, "LZ4_resetStreamState", FnResetState);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0xB0_0044);

    for &shape in ALL_SHAPES {
        // LZ4_create takes a buffer pointer; the chain must be contiguous.
        let total = 60_000usize;
        let data = gen_data(shape, total, &mut rng);
        let blk = 4096usize;
        unsafe {
            let (cst, rst) = (
                create.0(data.as_ptr() as *const c_char),
                create.1(data.as_ptr() as *const c_char),
            );
            assert!(!cst.is_null() && !rst.is_null(), "LZ4_create");
            let mut off = 0usize;
            let mut i = 0;
            while off + blk <= total {
                let cap = bound.0(blk as c_int) as usize;
                let mut cb = vec![0u8; cap + 16];
                let mut rb = vec![0u8; cap + 16];
                let (cn, rn) = (
                    cc.0(
                        cst,
                        data[off..].as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        blk as c_int,
                    ),
                    cc.1(
                        rst,
                        data[off..].as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        blk as c_int,
                    ),
                );
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("compress_continue {shape:?} i={i}"),
                );
                off += blk;
                i += 1;
            }
            // slideInputBuffer must return the same relative dictionary pointer.
            let cslid = slide.0(cst);
            let rslid = slide.1(rst);
            assert!(!cslid.is_null() && !rslid.is_null(), "slideInputBuffer");
            assert_ret_eq(rss.0(cst, data.as_ptr() as *const c_char), rss.1(rst, data.as_ptr() as *const c_char), "resetStreamState");
            fs.0(cst as *mut c_void);
            fs.1(rst as *mut c_void);
        }

        // limitedOutput_continue with a capacity sweep
        unsafe {
            let (cst, rst) = (
                create.0(data.as_ptr() as *const c_char),
                create.1(data.as_ptr() as *const c_char),
            );
            let mut off = 0usize;
            let mut i = 0;
            while off + blk <= total {
                let full = bound.0(blk as c_int) as usize;
                for &cap in &[0usize, 1, 10, full / 4, full / 2, full] {
                    let mut cb = vec![0u8; cap + 32];
                    let mut rb = vec![0u8; cap + 32];
                    let (cn, rn) = (
                        clc.0(
                            cst,
                            data[off..].as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                        ),
                        clc.1(
                            rst,
                            data[off..].as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("limitedOutput_continue {shape:?} i={i} cap={cap}"),
                    );
                }
                off += blk;
                i += 1;
            }
            fs.0(cst as *mut c_void);
            fs.1(rst as *mut c_void);
        }
    }
}

type FnCreateChar = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnSlide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnResetState = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;

// ---------------------------------------------------------------------------
// Row 45 — LZ4_compress_forceExtDict
// ---------------------------------------------------------------------------
#[test]
fn row45_compress_force_ext_dict() {
    sym!(cs, "LZ4_createStream", FnCreateStream);
    sym!(fs, "LZ4_freeStream", FnFreeStream);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(fed, "LZ4_compress_forceExtDict", FnContinue4);
    let mut rng = Rng::new(0xB0_0045);

    for &dsz in &[0usize, 4, 8, 1024, 65536] {
        for &shape in ALL_SHAPES {
            let dict = gen_data(shape, dsz, &mut rng);
            for &blk in &[64usize, 1024, 9000] {
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    ld.0(csp, dict.as_ptr() as *const c_char, dsz as c_int);
                    ld.1(rsp, dict.as_ptr() as *const c_char, dsz as c_int);
                    let src = gen_data(shape, blk, &mut rng);
                    // notLimited: dst is generously sized (the C hard-codes
                    // dstCapacity 0 with notLimited internally).
                    let mut cb = vec![0u8; blk * 2 + 1024];
                    let mut rb = vec![0u8; blk * 2 + 1024];
                    let (cn, rn) = (
                        fed.0(
                            csp,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                        ),
                        fed.1(
                            rsp,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                        ),
                    );
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("forceExtDict dsz={dsz} {shape:?} blk={blk}"),
                    );
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}
