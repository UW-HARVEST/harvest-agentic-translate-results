//! Anti-blind-spot differential tests: the code paths that only trigger after
//! a very large amount of data has passed through a *stream*, which per-call
//! tests can never reach.
//!
//! Specifically:
//!   * `LZ4_renormDictT` — `currentOffset + inputSize > 0x80000000`
//!     (`lz4.c:1690`), CONFIGS.md row 30.
//!   * `LZ4_prepareTable` — `tableType == byU32 && currentOffset > 1 GB`
//!     (`lz4.c:895`), CONFIGS.md row 12.
//!   * `LZ4HC_setExternalDict` — `newStartingOffset > 1 GB` (`lz4hc.c:248`) and
//!     `LZ4_compress_HC_continue` — `(end-prefixStart) + dictLimit > 2 GB`
//!     (`lz4hc.c:1695`), CONFIGS.md rows 70, 71.
//!   * Single inputs far above 64 KB / 1 MB so the long-offset and
//!     255-continuation encodings are used heavily.
//!
//! Every one of these compares the full `LZ4_stream_t` / `LZ4_streamHC_t` state
//! bytes as well as the emitted output, because the whole point is that an
//! internal counter is being rewritten.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

fn sizeof_state() -> usize {
    let n = unsafe { c().get::<FnI>("LZ4_sizeofState")() };
    let m = unsafe { r().get::<FnI>("LZ4_sizeofState")() };
    assert_eq!(n, m, "LZ4_sizeofState");
    n as usize
}

fn sizeof_state_hc() -> usize {
    let n = unsafe { c().get::<FnI>("LZ4_sizeofStateHC")() };
    let m = unsafe { r().get::<FnI>("LZ4_sizeofStateHC")() };
    assert_eq!(n, m, "LZ4_sizeofStateHC");
    n as usize
}

/// Compare the raw state bytes of the two streams.
fn assert_state_eq(ctx: &str, cs: *const c_void, rs: *const c_void, n: usize) {
    let a = unsafe { std::slice::from_raw_parts(cs as *const u8, n) };
    let b = unsafe { std::slice::from_raw_parts(rs as *const u8, n) };
    assert_bytes_eq!(ctx, a, b);
}


/// Byte offset of `currentOffset` inside `LZ4_stream_t_internal`
/// (`c_src/include/lz4.h:719`):
///   `LZ4_u32 hashTable[1<<12]` = 16384 bytes,
///   `const LZ4_byte* dictionary` = 8,
///   `const LZ4_stream_t_internal* dictCtx` = 8,
///   then `LZ4_u32 currentOffset`.
const CURRENT_OFFSET_AT: usize = 16384 + 8 + 8;

fn current_offset(state: *const c_void) -> u32 {
    let b = unsafe {
        std::slice::from_raw_parts((state as *const u8).add(CURRENT_OFFSET_AT), 4)
    };
    u32::from_le_bytes(b.try_into().unwrap())
}

/// Assert that `LZ4_renormDictT` / `LZ4_prepareTable` really did rewind
/// `currentOffset`, i.e. that the branch under test was actually reached.
/// Without this the loop could silently fail to reach the threshold and the
/// test would pass vacuously.
fn assert_renormalized(ctx: &str, cs: *const c_void, rs: *const c_void, fed: u64) {
    let co = current_offset(cs);
    let ro = current_offset(rs);
    assert_eq!(co, ro, "{}: currentOffset C={} Rust={}", ctx, co, ro);
    assert!(
        fed > 0x8000_0000,
        "{}: only {} bytes fed, the 0x80000000 threshold was never crossed",
        ctx, fed
    );
    assert!(
        (co as u64) < fed,
        "{}: currentOffset {} was never rewound (fed {} bytes)",
        ctx, co, fed
    );
}

// ===========================================================================
// CONFIGS.md rows 12, 30 — LZ4_renormDictT and the >1 GB prepareTable reset
// ===========================================================================

/// Push more than 2 GiB through a single `LZ4_stream_t` with
/// `LZ4_compress_fast_continue` so `currentOffset` crosses `0x80000000` and
/// `LZ4_renormDictT` rescales the whole hash table.  The rescale rewrites
/// 4096 table entries plus `currentOffset`, `dictSize` and `dictionary`, so the
/// state bytes are compared on every iteration around the crossing point.
#[test]
#[ignore = "streams >2 GiB; run explicitly with --ignored"]
fn renorm_dict_currentOffset_crosses_2gb() {
    let ssize = sizeof_state();
    let mut rng = Rng::new(0xB16_0001);

    // One 64 KiB ring buffer reused for every block, so `src` is never
    // contiguous with the previous block => the extDict path, and
    // `currentOffset` grows by exactly 64 KiB per call.
    const BLK: usize = 64 * 1024;
    let data = gen(&mut rng, BLK * 4, Shape::Text);

    let cs = unsafe { c().get::<Fn_createStream>("LZ4_createStream")() };
    let rs = unsafe { r().get::<Fn_createStream>("LZ4_createStream")() };
    assert!(!cs.is_null() && !rs.is_null());

    let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(BLK as c_int) };
    let mut cbuf = vec![0u8; bound as usize];
    let mut rbuf = vec![0u8; bound as usize];

    // 0x80000000 / 64 KiB == 32768 calls to reach the threshold.
    let iters = 33_000usize;
    for i in 0..iters {
        let off = (i % 4) * BLK;
        let src = &data[off..off + BLK];
        let cn = unsafe {
            c().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                cs,
                src.as_ptr() as *const c_char,
                cbuf.as_mut_ptr() as *mut c_char,
                BLK as c_int,
                cbuf.len() as c_int,
                1,
            )
        };
        let rn = unsafe {
            r().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                rs,
                src.as_ptr() as *const c_char,
                rbuf.as_mut_ptr() as *mut c_char,
                BLK as c_int,
                rbuf.len() as c_int,
                1,
            )
        };
        assert_eq!(cn, rn, "iteration {} return value", i);
        assert!(cn > 0, "iteration {} failed to compress", i);
        assert_bytes_eq!(
            format!("iteration {} output", i),
            cbuf[..cn as usize],
            rbuf[..rn as usize]
        );
        // Comparing 16 KiB of state on every one of 33 000 iterations is
        // affordable and is exactly what catches a mis-translated rescale.
        assert_state_eq(&format!("iteration {} state", i), cs, rs, ssize);
    }

    assert_renormalized(
        "renorm_dict_currentOffset_crosses_2gb",
        cs,
        rs,
        (iters as u64) * (BLK as u64),
    );

    unsafe {
        c().get::<Fn_freeStream>("LZ4_freeStream")(cs);
        r().get::<Fn_freeStream>("LZ4_freeStream")(rs);
    }
}

/// The same crossing, but reached with `LZ4_loadDict` + `LZ4_saveDict` cycles
/// (so `dictionary`/`dictSize` are non-trivial when the rescale happens) and
/// with a much cheaper iteration count by using 1 MiB blocks.
#[test]
#[ignore = "streams >2 GiB; run explicitly with --ignored"]
fn renorm_dict_with_saveDict_cycles() {
    let ssize = sizeof_state();
    let mut rng = Rng::new(0xB16_0002);
    const BLK: usize = 1024 * 1024;
    let data = gen(&mut rng, BLK * 2, Shape::Runs);

    let cs = unsafe { c().get::<Fn_createStream>("LZ4_createStream")() };
    let rs = unsafe { r().get::<Fn_createStream>("LZ4_createStream")() };
    let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(BLK as c_int) };
    let mut cbuf = vec![0u8; bound as usize];
    let mut rbuf = vec![0u8; bound as usize];
    // ONE shared safeBuffer for both libraries.  `LZ4_saveDict` repoints
    // `stream->dictionary` AT the safeBuffer, and that pointer is part of the
    // comparable state, so giving each library its own buffer would make the
    // state differ for a legitimate reason (different addresses).  Instead the
    // C writes first, its bytes are snapshotted, then the Rust writes into the
    // same buffer and both the returned length and the bytes are compared.
    let mut sdict = vec![0u8; 64 * 1024];

    // 0x80000000 / 1 MiB == 2048 calls.
    for i in 0..2_200usize {
        let off = (i % 2) * BLK;
        let src = &data[off..off + BLK];
        let cn = unsafe {
            c().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                cs,
                src.as_ptr() as *const c_char,
                cbuf.as_mut_ptr() as *mut c_char,
                BLK as c_int,
                cbuf.len() as c_int,
                1,
            )
        };
        let rn = unsafe {
            r().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                rs,
                src.as_ptr() as *const c_char,
                rbuf.as_mut_ptr() as *mut c_char,
                BLK as c_int,
                rbuf.len() as c_int,
                1,
            )
        };
        assert_eq!(cn, rn, "iteration {} return value", i);
        assert_bytes_eq!(
            format!("iteration {} output", i),
            cbuf[..cn as usize],
            rbuf[..rn as usize]
        );

        // save the dictionary out to the shared safeBuffer every few rounds
        if i % 7 == 3 {
            let a = unsafe {
                c().get::<Fn_saveDict>("LZ4_saveDict")(
                    cs,
                    sdict.as_mut_ptr() as *mut c_char,
                    sdict.len() as c_int,
                )
            };
            let c_saved = sdict[..a.max(0) as usize].to_vec();
            let b = unsafe {
                r().get::<Fn_saveDict>("LZ4_saveDict")(
                    rs,
                    sdict.as_mut_ptr() as *mut c_char,
                    sdict.len() as c_int,
                )
            };
            assert_eq!(a, b, "iteration {} saveDict return", i);
            assert_bytes_eq!(
                format!("iteration {} saveDict bytes", i),
                c_saved,
                sdict[..b.max(0) as usize]
            );
        }
        assert_state_eq(&format!("iteration {} state", i), cs, rs, ssize);
    }

    assert_renormalized(
        "renorm_dict_with_saveDict_cycles",
        cs,
        rs,
        2_200u64 * (BLK as u64),
    );

    unsafe {
        c().get::<Fn_freeStream>("LZ4_freeStream")(cs);
        r().get::<Fn_freeStream>("LZ4_freeStream")(rs);
    }
}

/// `LZ4_prepareTable` clears a `byU32` table once `currentOffset > 1 GB`
/// (`lz4.c:895`).  Drive `LZ4_compress_fast_extState_fastReset`, whose
/// `currentOffset` grows by 64 KiB per non-reset call, past that threshold.
#[test]
#[ignore = "streams >1 GiB; run explicitly with --ignored"]
fn prepareTable_clears_above_1gb_currentOffset() {
    let ssize = sizeof_state();
    let mut rng = Rng::new(0xB16_0003);
    // srcSize must stay below 4 KiB, otherwise prepareTable resets every call
    // and currentOffset never accumulates.
    const N: usize = 4000;
    let data = gen(&mut rng, N * 4, Shape::Text);

    let mut cst = Aligned::new(ssize, 64);
    let mut rst = Aligned::new(ssize, 64);
    let cinit = unsafe {
        c().get::<Fn_initStream>("LZ4_initStream")(cst.ptr(), ssize)
    };
    let rinit = unsafe {
        r().get::<Fn_initStream>("LZ4_initStream")(rst.ptr(), ssize)
    };
    assert!(!cinit.is_null() && !rinit.is_null());

    let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(N as c_int) };
    let mut cbuf = vec![0u8; bound as usize];
    let mut rbuf = vec![0u8; bound as usize];

    // currentOffset gains 64 KiB per call once it is non-zero, so
    // 1 GiB / 64 KiB == 16384 calls; go a bit past.
    for i in 0..18_000usize {
        let off = (i % 4) * N;
        let src = &data[off..off + N];
        let cn = unsafe {
            c().get::<Fn_compress_fast_extState>("LZ4_compress_fast_extState_fastReset")(
                cst.ptr(),
                src.as_ptr() as *const c_char,
                cbuf.as_mut_ptr() as *mut c_char,
                N as c_int,
                cbuf.len() as c_int,
                1,
            )
        };
        let rn = unsafe {
            r().get::<Fn_compress_fast_extState>("LZ4_compress_fast_extState_fastReset")(
                rst.ptr(),
                src.as_ptr() as *const c_char,
                rbuf.as_mut_ptr() as *mut c_char,
                N as c_int,
                rbuf.len() as c_int,
                1,
            )
        };
        assert_eq!(cn, rn, "iteration {} return value", i);
        assert!(cn > 0);
        assert_bytes_eq!(
            format!("iteration {} output", i),
            cbuf[..cn as usize],
            rbuf[..rn as usize]
        );
        assert_state_eq(&format!("iteration {} state", i), cst.ptr(), rst.ptr(), ssize);

        // and every block must still decode correctly in both libraries
        if i % 997 == 0 {
            for lib in [c(), r()] {
                let mut out = vec![0u8; N];
                let d = unsafe {
                    lib.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                        cbuf.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        cn,
                        N as c_int,
                    )
                };
                assert_eq!(d, N as c_int, "iteration {} decode via {}", i, lib.which);
                assert_bytes_eq!(format!("iteration {} decode", i), out, src);
            }
        }
    }
    cst.zero();
    rst.zero();
}

// ===========================================================================
// CONFIGS.md rows 70, 71 — the HC >1 GB / >2 GB re-seed paths
// ===========================================================================

/// `LZ4HC_setExternalDict` re-seeds when `newStartingOffset > 1 GB`
/// (`lz4hc.c:248`), and `LZ4_compress_HC_continue` forces a full
/// `LZ4_loadDictHC` when `(end-prefixStart) + dictLimit > 2 GB`
/// (`lz4hc.c:1695`).  Reach both by feeding a non-contiguous 1 MiB block
/// thousands of times into one `LZ4_streamHC_t`.
#[test]
#[ignore = "streams >2 GiB; run explicitly with --ignored"]
fn hc_continue_crosses_1gb_and_2gb() {
    let ssize = sizeof_state_hc();
    let mut rng = Rng::new(0xB16_0004);
    const BLK: usize = 1024 * 1024;
    let data = gen(&mut rng, BLK * 2, Shape::Text);

    for &level in &[1i32, 2, 3, 9, 10] {
        let cs = unsafe { c().get::<Fn_createStream>("LZ4_createStreamHC")() };
        let rs = unsafe { r().get::<Fn_createStream>("LZ4_createStreamHC")() };
        assert!(!cs.is_null() && !rs.is_null());
        unsafe {
            c().get::<Fn_resetStreamHC>("LZ4_resetStreamHC")(cs, level);
            r().get::<Fn_resetStreamHC>("LZ4_resetStreamHC")(rs, level);
        }

        let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(BLK as c_int) };
        let mut cbuf = vec![0u8; bound as usize];
        let mut rbuf = vec![0u8; bound as usize];

        // 2 GiB / 1 MiB == 2048 non-contiguous calls; go past it.
        for i in 0..2_300usize {
            let off = (i % 2) * BLK;
            let src = &data[off..off + BLK];
            let cn = unsafe {
                c().get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                    cs,
                    src.as_ptr() as *const c_char,
                    cbuf.as_mut_ptr() as *mut c_char,
                    BLK as c_int,
                    cbuf.len() as c_int,
                )
            };
            let rn = unsafe {
                r().get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                    rs,
                    src.as_ptr() as *const c_char,
                    rbuf.as_mut_ptr() as *mut c_char,
                    BLK as c_int,
                    rbuf.len() as c_int,
                )
            };
            assert_eq!(cn, rn, "level={} iteration {} return value", level, i);
            assert!(cn > 0, "level={} iteration {} failed", level, i);
            assert_bytes_eq!(
                format!("level={} iteration {} output", level, i),
                cbuf[..cn as usize],
                rbuf[..rn as usize]
            );
            assert_state_eq(
                &format!("level={} iteration {} state", level, i),
                cs,
                rs,
                ssize,
            );
        }

        unsafe {
            c().get::<Fn_freeStream>("LZ4_freeStreamHC")(cs);
            r().get::<Fn_freeStream>("LZ4_freeStreamHC")(rs);
        }
    }
}

// ===========================================================================
// Large single inputs — long offsets, 255-continuation runs, wildCopy tails
// ===========================================================================

/// Multi-megabyte one-shot compression across every entry point and shape.
/// Sizes are chosen to straddle 1 MiB, 4 MiB (the largest frame block) and
/// 16 MiB, well past `LZ4_64Klimit`, so the `byU32` table, long match offsets
/// and the `ML_MASK`/`RUN_MASK` 255-byte continuation chains all get heavy use.
#[test]
fn large_inputs_one_shot_all_entry_points() {
    let mut rng = Rng::new(0xB16_0005);
    let sizes = [
        1 << 20,
        (1 << 20) + 1,
        3_000_000usize,
        4 << 20,
        (4 << 20) + 1,
        9_999_999,
    ];
    for &shape in &[
        Shape::Text,
        Shape::Runs,
        Shape::Constant,
        Shape::Periodic(65535),
        Shape::Sparse,
        Shape::Incompressible,
    ] {
        for &n in &sizes {
            let src = gen(&mut rng, n, shape);
            let bound = {
                let (a, b) = both(|l| unsafe {
                    l.get::<Fn_compressBound>("LZ4_compressBound")(n as c_int)
                });
                assert_eq!(a, b, "compressBound({})", n);
                a as usize
            };

            // LZ4_compress_default
            let vals = both(|l| {
                let mut out = vec![0u8; bound];
                let k = unsafe {
                    l.get::<Fn_compress_default>("LZ4_compress_default")(
                        src.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        bound as c_int,
                    )
                };
                out.truncate(k.max(0) as usize);
                (k, out)
            });
            assert_eq!(
                vals.0 .0, vals.1 .0,
                "compress_default n={} shape={:?}",
                n, shape
            );
            assert_bytes_eq!(
                format!("compress_default n={} shape={:?}", n, shape),
                vals.0 .1,
                vals.1 .1
            );

            // decode the block with BOTH libraries
            for lib in [c(), r()] {
                let mut out = vec![0u8; n];
                let d = unsafe {
                    lib.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                        vals.0 .1.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        vals.0 .0,
                        n as c_int,
                    )
                };
                assert_eq!(d, n as c_int, "decode n={} shape={:?} via {}", n, shape, lib.which);
                assert_bytes_eq!(format!("decode n={} via {}", n, lib.which), out, src);
            }

            // LZ4_compress_fast over a few accelerations
            for &acc in &[1i32, 3, 64, 65537] {
                let vals = both(|l| {
                    let mut out = vec![0u8; bound];
                    let k = unsafe {
                        l.get::<Fn_compress_fast>("LZ4_compress_fast")(
                            src.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            bound as c_int,
                            acc,
                        )
                    };
                    out.truncate(k.max(0) as usize);
                    (k, out)
                });
                assert_eq!(
                    vals.0 .0, vals.1 .0,
                    "compress_fast n={} shape={:?} acc={}",
                    n, shape, acc
                );
                assert_bytes_eq!(
                    format!("compress_fast n={} acc={}", n, acc),
                    vals.0 .1,
                    vals.1 .1
                );
            }

            // LZ4_decompress_safe_partial across the whole output range
            let comp = &vals.0 .1;
            for &frac in &[0usize, 1, 5, 12, 13, 64, 65535, 65536] {
                let target = frac.min(n);
                let vals2 = both(|l| {
                    let mut out = vec![0u8; target.max(1)];
                    let k = unsafe {
                        l.get::<Fn_decompress_safe_partial>("LZ4_decompress_safe_partial")(
                            comp.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            target as c_int,
                            out.len() as c_int,
                        )
                    };
                    out.truncate(k.max(0) as usize);
                    (k, out)
                });
                assert_eq!(
                    vals2.0 .0, vals2.1 .0,
                    "partial n={} target={}",
                    n, target
                );
                assert_bytes_eq!(
                    format!("partial n={} target={}", n, target),
                    vals2.0 .1,
                    vals2.1 .1
                );
            }
        }
    }
}

/// Multi-megabyte HC compression at every strategy boundary (lz4mid /
/// hashChain / optimal), which is where the pattern-analysis and chain-swap
/// code only warms up on large inputs.
#[test]
fn large_inputs_hc_all_strategies() {
    let mut rng = Rng::new(0xB16_0006);
    // one level per internal strategy, plus the ultra level
    for &level in &[1i32, 2, 3, 8, 9, 10, 12] {
        for &shape in &[Shape::Text, Shape::Constant, Shape::Periodic(64), Shape::Runs] {
            // level 12 on many megabytes is very slow; scale the size down
            let n = if level >= 10 { 600_000usize } else { 2_500_000 };
            let src = gen(&mut rng, n, shape);
            let bound =
                unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(n as c_int) } as usize;
            let vals = both(|l| {
                let mut out = vec![0u8; bound];
                let k = unsafe {
                    l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                        src.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        n as c_int,
                        bound as c_int,
                        level,
                    )
                };
                out.truncate(k.max(0) as usize);
                (k, out)
            });
            assert_eq!(
                vals.0 .0, vals.1 .0,
                "compress_HC n={} shape={:?} level={}",
                n, shape, level
            );
            assert_bytes_eq!(
                format!("compress_HC n={} shape={:?} level={}", n, shape, level),
                vals.0 .1,
                vals.1 .1
            );
            for lib in [c(), r()] {
                let mut out = vec![0u8; n];
                let d = unsafe {
                    lib.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                        vals.0 .1.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        vals.0 .0,
                        n as c_int,
                    )
                };
                assert_eq!(d, n as c_int, "HC decode level={} via {}", level, lib.which);
                assert_bytes_eq!(format!("HC decode level={}", level), out, src);
            }
        }
    }
}

/// Multi-megabyte FRAME round trips with 4 MiB blocks, so a frame really does
/// contain several maximum-size blocks and every `LZ4F_updateDict` branch runs.
#[test]
fn large_inputs_frame_multiblock() {
    use std::ptr;
    let mut rng = Rng::new(0xB16_0007);
    for &bsid in &[LZ4F_MAX64KB, LZ4F_MAX1MB, LZ4F_MAX4MB] {
        for &bmode in &[LZ4F_BLOCK_LINKED, LZ4F_BLOCK_INDEPENDENT] {
            for &(ccs, bcs) in &[(0i32, 0i32), (1, 1)] {
                for &level in &[0i32, 2, 9] {
                    let mut prefs = LZ4F_preferences_t::default();
                    prefs.frameInfo.blockSizeID = bsid;
                    prefs.frameInfo.blockMode = bmode;
                    prefs.frameInfo.contentChecksumFlag = ccs;
                    prefs.frameInfo.blockChecksumFlag = bcs;
                    prefs.compressionLevel = level;

                    let n = 10_000_000usize;
                    let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                    let src = gen(&mut rng, n, shape);

                    let bound = {
                        let (a, b) = both(|l| unsafe {
                            l.get::<Fn_F_compressFrameBound>("LZ4F_compressFrameBound")(
                                n, &prefs,
                            )
                        });
                        assert_eq!(a, b, "compressFrameBound n={} bsid={}", n, bsid);
                        a
                    };

                    let vals = both(|l| {
                        let mut out = vec![0u8; bound];
                        let k = unsafe {
                            l.get::<Fn_F_compressFrame>("LZ4F_compressFrame")(
                                out.as_mut_ptr() as *mut c_void,
                                out.len(),
                                src.as_ptr() as *const c_void,
                                src.len(),
                                &prefs,
                            )
                        };
                        assert!(!is_error(k), "{}: compressFrame -> {}", l.which, show(k));
                        out.truncate(k);
                        (k, out)
                    });
                    assert_eq!(
                        vals.0 .0, vals.1 .0,
                        "compressFrame n={} bsid={} bmode={} ccs={} bcs={} lvl={}",
                        n, bsid, bmode, ccs, bcs, level
                    );
                    assert_bytes_eq!(
                        format!(
                            "compressFrame bytes bsid={} bmode={} ccs={} bcs={} lvl={}",
                            bsid, bmode, ccs, bcs, level
                        ),
                        vals.0 .1,
                        vals.1 .1
                    );

                    // decode with several output chunk sizes so both the
                    // decode-into-dst and decode-into-tmpOut paths run
                    let frame = &vals.0 .1;
                    for &out_chunk in &[1024usize, 65535, 1 << 22] {
                        let dvals = both(|l| {
                            let mut dctx: *mut c_void = ptr::null_mut();
                            let rc = unsafe {
                                l.get::<Fn_F_createDecompressionContext>(
                                    "LZ4F_createDecompressionContext",
                                )(&mut dctx, LZ4F_VERSION)
                            };
                            assert!(!is_error(rc));
                            let mut produced: Vec<u8> = Vec::with_capacity(n);
                            let mut buf = vec![0u8; out_chunk];
                            let mut consumed = 0usize;
                            let mut last = 1usize;
                            while consumed < frame.len() && last != 0 {
                                let mut ds = buf.len();
                                let mut ss = frame.len() - consumed;
                                last = unsafe {
                                    l.get::<Fn_F_decompress>("LZ4F_decompress")(
                                        dctx,
                                        buf.as_mut_ptr() as *mut c_void,
                                        &mut ds,
                                        frame[consumed..].as_ptr() as *const c_void,
                                        &mut ss,
                                        ptr::null(),
                                    )
                                };
                                assert!(
                                    !is_error(last),
                                    "{}: decompress -> {}",
                                    l.which,
                                    show(last)
                                );
                                produced.extend_from_slice(&buf[..ds]);
                                consumed += ss;
                                if ds == 0 && ss == 0 {
                                    break;
                                }
                            }
                            let stage = unsafe {
                                l.get::<Fn_F_freeDecompressionContext>(
                                    "LZ4F_freeDecompressionContext",
                                )(dctx)
                            };
                            (last, consumed, stage, produced)
                        });
                        assert_eq!(
                            (dvals.0 .0, dvals.0 .1, dvals.0 .2),
                            (dvals.1 .0, dvals.1 .1, dvals.1 .2),
                            "decompress bsid={} bmode={} out_chunk={}",
                            bsid, bmode, out_chunk
                        );
                        assert_bytes_eq!(
                            format!("decompress output out_chunk={}", out_chunk),
                            dvals.0 .3,
                            dvals.1 .3
                        );
                        assert_bytes_eq!("decompress == src", dvals.0 .3, src);
                    }
                }
            }
        }
    }
}

/// xxHash over multi-megabyte inputs (many main-loop iterations), streaming in
/// awkward chunk sizes so the internal 16/32-byte buffer wraps thousands of
/// times.
#[test]
fn large_inputs_xxhash_streaming() {
    let mut rng = Rng::new(0xB16_0008);
    let data = gen(&mut rng, 8_000_000, Shape::Text);

    for &(bits, create, reset, update, free) in &[
        (
            32u32,
            "LZ4_XXH32_createState",
            "LZ4_XXH32_reset",
            "LZ4_XXH32_update",
            "LZ4_XXH32_freeState",
        ),
        (
            64,
            "LZ4_XXH64_createState",
            "LZ4_XXH64_reset",
            "LZ4_XXH64_update",
            "LZ4_XXH64_freeState",
        ),
    ] {
        for &chunk in &[1usize, 3, 7, 15, 16, 17, 31, 32, 33, 4095, 65536] {
            let cs = unsafe { c().get::<Fn_XXH_createState>(create)() };
            let rs = unsafe { r().get::<Fn_XXH_createState>(create)() };
            unsafe {
                if bits == 32 {
                    c().get::<Fn_XXH32_reset>(reset)(cs, 0x1234_5678);
                    r().get::<Fn_XXH32_reset>(reset)(rs, 0x1234_5678);
                } else {
                    c().get::<Fn_XXH64_reset>(reset)(cs, 0x1234_5678_9ABC_DEF0);
                    r().get::<Fn_XXH64_reset>(reset)(rs, 0x1234_5678_9ABC_DEF0);
                }
            }
            // Only feed a prefix for the tiny chunk sizes, otherwise the test
            // would take minutes.
            let total = if chunk < 64 { 400_000 } else { data.len() };
            let mut off = 0usize;
            while off < total {
                let k = chunk.min(total - off);
                let a = unsafe {
                    c().get::<Fn_XXH_update>(update)(
                        cs,
                        data[off..].as_ptr() as *const c_void,
                        k,
                    )
                };
                let b = unsafe {
                    r().get::<Fn_XXH_update>(update)(
                        rs,
                        data[off..].as_ptr() as *const c_void,
                        k,
                    )
                };
                assert_eq!(a, b, "XXH{} update chunk={} off={}", bits, chunk, off);
                off += k;
            }
            if bits == 32 {
                let a = unsafe { c().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(cs) };
                let b = unsafe { r().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(rs) };
                assert_eq!(a, b, "XXH32 digest chunk={} total={}", chunk, total);
                let one = unsafe {
                    c().get::<Fn_XXH32>("LZ4_XXH32")(
                        data.as_ptr() as *const c_void,
                        total,
                        0x1234_5678,
                    )
                };
                assert_eq!(a, one, "XXH32 streaming vs one-shot chunk={}", chunk);
            } else {
                let a = unsafe { c().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(cs) };
                let b = unsafe { r().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(rs) };
                assert_eq!(a, b, "XXH64 digest chunk={} total={}", chunk, total);
                let one = unsafe {
                    c().get::<Fn_XXH64>("LZ4_XXH64")(
                        data.as_ptr() as *const c_void,
                        total,
                        0x1234_5678_9ABC_DEF0,
                    )
                };
                assert_eq!(a, one, "XXH64 streaming vs one-shot chunk={}", chunk);
            }
            unsafe {
                c().get::<Fn_XXH_freeState>(free)(cs);
                r().get::<Fn_XXH_freeState>(free)(rs);
            }
        }
    }
}

/// A long-running streaming compression + streaming decompression pipeline over
/// a ring buffer, sized by `LZ4_decoderRingBufferSize`, driven for thousands of
/// blocks so the wrap-around / `forceExtDict` promotion happens many times and
/// the two libraries' stream states are compared throughout.
#[test]
fn ring_buffer_long_run() {
    let ssize = sizeof_state();
    let mut rng = Rng::new(0xB16_0009);
    const MAX_BLK: usize = 8192;

    let ring_size = {
        let (a, b) = both(|l| unsafe {
            l.get::<Fn_decoderRingBufferSize>("LZ4_decoderRingBufferSize")(MAX_BLK as c_int)
        });
        assert_eq!(a, b, "decoderRingBufferSize({})", MAX_BLK);
        a as usize
    };
    assert!(ring_size > MAX_BLK);

    let cs = unsafe { c().get::<Fn_createStream>("LZ4_createStream")() };
    let rs = unsafe { r().get::<Fn_createStream>("LZ4_createStream")() };
    let cd = unsafe { c().get::<Fn_createStreamDecode>("LZ4_createStreamDecode")() };
    let rd = unsafe { r().get::<Fn_createStreamDecode>("LZ4_createStreamDecode")() };
    assert!(!cs.is_null() && !rs.is_null() && !cd.is_null() && !rd.is_null());

    // input ring (shared: identical addresses for both libraries)
    let in_ring_size = 65536 + MAX_BLK;
    let mut in_ring = vec![0u8; in_ring_size];
    // separate output rings so the two decoders cannot influence each other
    let mut c_out = vec![0u8; ring_size];
    let mut r_out = vec![0u8; ring_size];

    let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(MAX_BLK as c_int) };
    let mut cbuf = vec![0u8; bound as usize];
    let mut rbuf = vec![0u8; bound as usize];

    let mut in_off = 0usize;
    let mut c_out_off = 0usize;
    let mut r_out_off = 0usize;

    for i in 0..4000usize {
        let blk = rng.range(1, MAX_BLK);
        if in_off + blk > in_ring_size {
            in_off = 0;
        }
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let chunk = gen(&mut rng, blk, shape);
        in_ring[in_off..in_off + blk].copy_from_slice(&chunk);

        let cn = unsafe {
            c().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                cs,
                in_ring[in_off..].as_ptr() as *const c_char,
                cbuf.as_mut_ptr() as *mut c_char,
                blk as c_int,
                cbuf.len() as c_int,
                1,
            )
        };
        let rn = unsafe {
            r().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue")(
                rs,
                in_ring[in_off..].as_ptr() as *const c_char,
                rbuf.as_mut_ptr() as *mut c_char,
                blk as c_int,
                rbuf.len() as c_int,
                1,
            )
        };
        assert_eq!(cn, rn, "iteration {} compress", i);
        assert!(cn > 0, "iteration {} compress failed", i);
        assert_bytes_eq!(
            format!("iteration {} compressed", i),
            cbuf[..cn as usize],
            rbuf[..rn as usize]
        );
        assert_state_eq(&format!("iteration {} cstream", i), cs, rs, ssize);

        // ring-buffer decode: reset the write offset when the block cannot fit
        if c_out_off + MAX_BLK > ring_size {
            c_out_off = 0;
            r_out_off = 0;
        }
        let cdn = unsafe {
            c().get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue")(
                cd,
                cbuf.as_ptr() as *const c_char,
                c_out[c_out_off..].as_mut_ptr() as *mut c_char,
                cn,
                MAX_BLK as c_int,
            )
        };
        let rdn = unsafe {
            r().get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue")(
                rd,
                rbuf.as_ptr() as *const c_char,
                r_out[r_out_off..].as_mut_ptr() as *mut c_char,
                rn,
                MAX_BLK as c_int,
            )
        };
        assert_eq!(cdn, rdn, "iteration {} decompress", i);
        assert_eq!(cdn, blk as c_int, "iteration {} short decode", i);
        assert_bytes_eq!(
            format!("iteration {} decoded", i),
            c_out[c_out_off..c_out_off + blk],
            r_out[r_out_off..r_out_off + blk]
        );
        assert_bytes_eq!(
            format!("iteration {} decoded == src", i),
            c_out[c_out_off..c_out_off + blk],
            chunk
        );

        in_off += blk;
        c_out_off += blk;
        r_out_off += blk;
    }

    unsafe {
        c().get::<Fn_freeStream>("LZ4_freeStream")(cs);
        r().get::<Fn_freeStream>("LZ4_freeStream")(rs);
        c().get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(cd);
        r().get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(rd);
    }
}
