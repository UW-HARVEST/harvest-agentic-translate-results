//! Differential tests for the lz4.c block API (Phase B / valid paths).
//!
//! Every call is dispatched through BOTH shared libraries' export tables.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Signature aliases
// ---------------------------------------------------------------------------

type FnVersionNum = unsafe extern "C" fn() -> c_int;
type FnVersionStr = unsafe extern "C" fn() -> *const c_char;
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnSizeofState = unsafe extern "C" fn() -> c_int;
type FnCompDefault = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCompFastExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCompDestSize = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnCompDestSizeExt =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FnDecompSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompPartial =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDecompFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnDecompUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
type FnDecompPartialUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type FnDecompFastUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
type FnRingBufSize = unsafe extern "C" fn(c_int) -> c_int;

// ---------------------------------------------------------------------------
// Size sweep: covers every input-shape boundary lz4.c branches on
// ---------------------------------------------------------------------------

/// Sizes chosen to straddle every documented threshold in lz4.c:
/// 0, 1, `< LASTLITERALS`, `< MFLIMIT`, `== MINMATCH`, the `byU16`/`byU32`
/// `LZ4_64Klimit` switch point (65547) on both sides, and `> 64 KB`.
fn boundary_sizes() -> Vec<usize> {
    vec![
        0,
        1,
        2,
        3,
        4, // MINMATCH
        5, // LASTLITERALS
        6,
        11,
        12, // MFLIMIT
        13,
        15,
        16,
        17,
        63,
        64,
        65,
        127,
        128,
        255,
        256,
        257,
        511,
        512,
        1024,
        4095,
        4096,
        65535,
        65536,
        LZ4_64Klimit - 1, // 65546 — last byU16 size
        LZ4_64Klimit,     // 65547 — first byU32 size
        LZ4_64Klimit + 1,
        70000,
        131072,
        200000,
    ]
}

// ===========================================================================
// CONFIGS row group: trivial / metadata entry points
// ===========================================================================

#[test]
fn version_and_metadata() {
    let (cv, rv) = both::<FnVersionNum>("LZ4_versionNumber");
    let (cs, rs) = both::<FnVersionStr>("LZ4_versionString");
    let (cb, rb) = both::<FnBound>("LZ4_compressBound");
    let (css, rss) = both::<FnSizeofState>("LZ4_sizeofState");
    let (csss, rsss) = both::<FnSizeofState>("LZ4_sizeofStreamState");
    let (crb, rrb) = both::<FnRingBufSize>("LZ4_decoderRingBufferSize");

    unsafe {
        assert_eq!(cv(), rv(), "LZ4_versionNumber");
        let c = std::ffi::CStr::from_ptr(cs()).to_bytes().to_vec();
        let r = std::ffi::CStr::from_ptr(rs()).to_bytes().to_vec();
        assert_eq!(c, r, "LZ4_versionString");
        assert_eq!(css(), rss(), "LZ4_sizeofState");
        assert_eq!(csss(), rsss(), "LZ4_sizeofStreamState");

        // LZ4_compressBound / LZ4_decoderRingBufferSize across the full
        // interesting range including the LZ4_MAX_INPUT_SIZE cutoff.
        let mut probes: Vec<c_int> = vec![
            c_int::MIN,
            -1000000,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            15,
            16,
            255,
            256,
            65535,
            65536,
            LZ4_64Klimit as c_int,
            0x7DFF_FFFE,
            0x7DFF_FFFF,
            LZ4_MAX_INPUT_SIZE as c_int, // 0x7E000000 — last valid
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            0x7FFF_FFFE,
            c_int::MAX,
        ];
        let mut rng = Rng::new(0xB0);
        for _ in 0..2000 {
            probes.push(rng.next_u32() as c_int);
        }
        for &p in &probes {
            assert_eq!(cb(p), rb(p), "LZ4_compressBound({})", p);
            assert_eq!(crb(p), rrb(p), "LZ4_decoderRingBufferSize({})", p);
        }
    }
}

// ===========================================================================
// CONFIGS row group: one-shot compression, all shapes x all sizes
// ===========================================================================

/// Compress with C and Rust and require byte-identical compressed output.
/// Returns the C-produced compressed bytes for reuse by decompression tests.
fn diff_compress_default(src: &[u8], label: &str) -> Vec<u8> {
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let (cc, rc) = both::<FnCompDefault>("LZ4_compress_default");
    let bound = unsafe { cb(src.len() as c_int) }.max(1) as usize;

    let mut cdst = vec![0xAAu8; bound];
    let mut rdst = vec![0xAAu8; bound];
    let cn = unsafe {
        cc(
            src.as_ptr() as *const c_char,
            cdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
        )
    };
    let rn = unsafe {
        rc(
            src.as_ptr() as *const c_char,
            rdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
        )
    };
    assert_eq!(cn, rn, "{}: LZ4_compress_default return value", label);
    assert!(cn >= 0, "{}: unexpected negative return {}", label, cn);
    let n = cn as usize;
    // Compare the ENTIRE destination buffer, not just `[..n]`: both sides were
    // pre-filled with the same sentinel, so this also detects a write past the
    // reported compressed length.
    assert_bytes_eq(
        &format!("{}: LZ4_compress_default payload+tail", label),
        &cdst,
        &rdst,
    );
    cdst.truncate(n);
    cdst
}

#[test]
fn compress_default_all_shapes_and_sizes() {
    let mut rng = Rng::new(0x1234_5678);
    for shape in 0..N_SHAPES {
        for &len in &boundary_sizes() {
            let src = gen_shape(&mut rng, shape, len);
            diff_compress_default(&src, &format!("shape={} len={}", shape_name(shape), len));
        }
    }
}

#[test]
fn compress_default_randomized() {
    let mut rng = Rng::new(0xDEAD_BEEF);
    for i in 0..400 {
        let shape = rng.below(N_SHAPES);
        let len = match rng.below(4) {
            0 => rng.range(0, 64),
            1 => rng.range(0, 4096),
            2 => rng.range(60000, 70000), // straddle LZ4_64Klimit
            _ => rng.range(0, 150000),
        };
        let src = gen_shape(&mut rng, shape, len);
        diff_compress_default(
            &src,
            &format!("iter={} shape={} len={}", i, shape_name(shape), len),
        );
    }
}

// ===========================================================================
// CONFIGS row group: LZ4_compress_fast — acceleration axis
// ===========================================================================

#[test]
fn compress_fast_acceleration_axis() {
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let (cc, rc) = both::<FnCompFast>("LZ4_compress_fast");
    let mut rng = Rng::new(0xACCE_1234);

    // acceleration values spanning: <=0 (clamped to default 1), 1, small,
    // large, exactly LZ4_ACCELERATION_MAX, and beyond (clamped down).
    let accels: Vec<c_int> = vec![
        c_int::MIN,
        -100000,
        -1,
        0,
        1,
        2,
        3,
        7,
        64,
        1000,
        65536,
        LZ4_ACCELERATION_MAX - 1,
        LZ4_ACCELERATION_MAX,
        LZ4_ACCELERATION_MAX + 1,
        1_000_000,
        c_int::MAX,
    ];

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 12, 13, 500, 5000, 65546, 65547, 100000] {
            let src = gen_shape(&mut rng, shape, len);
            let bound = unsafe { cb(len as c_int) }.max(1) as usize;
            for &acc in &accels {
                let mut cdst = vec![0xAAu8; bound];
                let mut rdst = vec![0xAAu8; bound];
                let cn = unsafe {
                    cc(
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        acc,
                    )
                };
                let rn = unsafe {
                    rc(
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        bound as c_int,
                        acc,
                    )
                };
                let label = format!(
                    "LZ4_compress_fast shape={} len={} accel={}",
                    shape_name(shape),
                    len,
                    acc
                );
                assert_eq!(cn, rn, "{}: return value", label);
                assert!(cn >= 0, "{}: negative return", label);
                assert_bytes_eq(&label, &cdst[..cn as usize], &rdst[..cn as usize]);
            }
        }
    }
}

// ===========================================================================
// CONFIGS row group: extState variants (low-level entry points)
// ===========================================================================

#[test]
fn compress_fast_extstate_and_fastreset() {
    let (c_size, r_size) = both::<FnSizeofState>("LZ4_sizeofState");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let (cc, rc) = both::<FnCompFastExt>("LZ4_compress_fast_extState");
    let (ccf, rcf) = both::<FnCompFastExt>("LZ4_compress_fast_extState_fastReset");

    let cs = unsafe { c_size() } as usize;
    let rs = unsafe { r_size() } as usize;
    assert_eq!(cs, rs, "LZ4_sizeofState must agree");
    let ssz = cs;

    let mut rng = Rng::new(0xE5747);
    // Reuse the SAME state across successive calls, which is exactly how the
    // fastReset variant differs from the full-reset variant.
    let mut cstate = AlignedBuf::new(ssz, 64);
    let mut rstate = AlignedBuf::new(ssz, 64);
    let mut cstate_fr = AlignedBuf::new(ssz, 64);
    let mut rstate_fr = AlignedBuf::new(ssz, 64);

    for round in 0..3 {
        for shape in 0..N_SHAPES {
            for &len in &[0usize, 1, 13, 700, 5000, 65546, 65547, 100000] {
                let src = gen_shape(&mut rng, shape, len);
                let bound = unsafe { cb(len as c_int) }.max(1) as usize;
                for &acc in &[1i32, 2, 9, 65537] {
                    let label = format!(
                        "extState round={} shape={} len={} accel={}",
                        round,
                        shape_name(shape),
                        len,
                        acc
                    );
                    let mut cdst = vec![0xAAu8; bound];
                    let mut rdst = vec![0xAAu8; bound];
                    let cn = unsafe {
                        cc(
                            cstate.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            acc,
                        )
                    };
                    let rn = unsafe {
                        rc(
                            rstate.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            acc,
                        )
                    };
                    assert_eq!(cn, rn, "{}: extState return", label);
                    assert_bytes_eq(
                        &format!("{} extState", label),
                        &cdst[..cn as usize],
                        &rdst[..cn as usize],
                    );
                    // The state block itself must evolve identically.
                    assert_bytes_eq(
                        &format!("{} extState STATE BLOCK", label),
                        cstate.as_slice(),
                        rstate.as_slice(),
                    );

                    let mut cdst = vec![0xAAu8; bound];
                    let mut rdst = vec![0xAAu8; bound];
                    let cn = unsafe {
                        ccf(
                            cstate_fr.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            acc,
                        )
                    };
                    let rn = unsafe {
                        rcf(
                            rstate_fr.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            bound as c_int,
                            acc,
                        )
                    };
                    assert_eq!(cn, rn, "{}: fastReset return", label);
                    assert_bytes_eq(
                        &format!("{} fastReset", label),
                        &cdst[..cn as usize],
                        &rdst[..cn as usize],
                    );
                    assert_bytes_eq(
                        &format!("{} fastReset STATE BLOCK", label),
                        cstate_fr.as_slice(),
                        rstate_fr.as_slice(),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS row group: fillOutput / destSize (the third limitedOutput_directive)
// ===========================================================================

#[test]
fn compress_destsize_fill_output() {
    let (cc, rc) = both::<FnCompDestSize>("LZ4_compress_destSize");
    let mut rng = Rng::new(0xF111);

    for shape in 0..N_SHAPES {
        for &len in &[1usize, 13, 100, 1000, 20000, 65546, 65547, 130000] {
            let src = gen_shape(&mut rng, shape, len);
            // Sweep target capacities from far-too-small to more than enough:
            // this is the axis that drives the fillOutput partial-block logic.
            let caps: Vec<usize> = vec![
                0,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                16,
                17,
                len / 8 + 1,
                len / 4 + 1,
                len / 2 + 1,
                len,
                len + 16,
                len + len / 255 + 16,
            ];
            for &cap in &caps {
                let mut c_srcsize = len as c_int;
                let mut r_srcsize = len as c_int;
                let mut cdst = vec![0xAAu8; cap.max(1)];
                let mut rdst = vec![0xAAu8; cap.max(1)];
                let cn = unsafe {
                    cc(
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        &mut c_srcsize,
                        cap as c_int,
                    )
                };
                let rn = unsafe {
                    rc(
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        &mut r_srcsize,
                        cap as c_int,
                    )
                };
                let label = format!(
                    "LZ4_compress_destSize shape={} len={} cap={}",
                    shape_name(shape),
                    len,
                    cap
                );
                assert_eq!(cn, rn, "{}: return value", label);
                assert_eq!(
                    c_srcsize, r_srcsize,
                    "{}: consumed srcSize out-param",
                    label
                );
                if cn > 0 {
                    assert_bytes_eq(&label, &cdst[..cn as usize], &rdst[..cn as usize]);
                    // And the truncated compression must round-trip to the
                    // first `srcSize` bytes of the input.
                    let (cd, _) = both::<FnDecompSafe>("LZ4_decompress_safe");
                    let mut out = vec![0u8; c_srcsize as usize + 8];
                    let dn = unsafe {
                        cd(
                            cdst.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            cn,
                            out.len() as c_int,
                        )
                    };
                    assert_eq!(dn, c_srcsize, "{}: round-trip size", label);
                    assert_eq!(
                        &out[..dn as usize],
                        &src[..dn as usize],
                        "{}: round-trip content",
                        label
                    );
                }
            }
        }
    }
}

#[test]
fn compress_destsize_extstate() {
    let (c_size, _) = both::<FnSizeofState>("LZ4_sizeofState");
    let (cc, rc) = both::<FnCompDestSizeExt>("LZ4_compress_destSize_extState");
    let ssz = unsafe { c_size() } as usize;
    let mut cstate = AlignedBuf::new(ssz, 64);
    let mut rstate = AlignedBuf::new(ssz, 64);
    let mut rng = Rng::new(0xF12E);

    for shape in 0..N_SHAPES {
        for &len in &[1usize, 13, 900, 20000, 65547, 120000] {
            let src = gen_shape(&mut rng, shape, len);
            for &cap in &[1usize, 8, 64, len / 3 + 1, len + 16] {
                for &acc in &[1i32, 4, 65537] {
                    let mut c_ss = len as c_int;
                    let mut r_ss = len as c_int;
                    let mut cdst = vec![0xAAu8; cap.max(1)];
                    let mut rdst = vec![0xAAu8; cap.max(1)];
                    let cn = unsafe {
                        cc(
                            cstate.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            cdst.as_mut_ptr() as *mut c_char,
                            &mut c_ss,
                            cap as c_int,
                            acc,
                        )
                    };
                    let rn = unsafe {
                        rc(
                            rstate.as_mut_ptr() as *mut c_void,
                            src.as_ptr() as *const c_char,
                            rdst.as_mut_ptr() as *mut c_char,
                            &mut r_ss,
                            cap as c_int,
                            acc,
                        )
                    };
                    let label = format!(
                        "destSize_extState shape={} len={} cap={} accel={}",
                        shape_name(shape),
                        len,
                        cap,
                        acc
                    );
                    assert_eq!(cn, rn, "{}: return", label);
                    assert_eq!(c_ss, r_ss, "{}: srcSize out-param", label);
                    if cn > 0 {
                        assert_bytes_eq(&label, &cdst[..cn as usize], &rdst[..cn as usize]);
                    }
                    assert_bytes_eq(
                        &format!("{} STATE BLOCK", label),
                        cstate.as_slice(),
                        rstate.as_slice(),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS row group: decompression entry points
// ===========================================================================

#[test]
fn decompress_safe_and_fast_roundtrip() {
    let (cds, rds) = both::<FnDecompSafe>("LZ4_decompress_safe");
    let (cdf, rdf) = both::<FnDecompFast>("LZ4_decompress_fast");
    let mut rng = Rng::new(0xD3C0);

    for shape in 0..N_SHAPES {
        for &len in &boundary_sizes() {
            let src = gen_shape(&mut rng, shape, len);
            let label = format!("decompress shape={} len={}", shape_name(shape), len);
            let comp = diff_compress_default(&src, &label);

            // --- LZ4_decompress_safe with exactly-right capacity
            let mut cout = vec![0xAAu8; len.max(1)];
            let mut rout = vec![0xAAu8; len.max(1)];
            let cn = unsafe {
                cds(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    len as c_int,
                )
            };
            let rn = unsafe {
                rds(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    len as c_int,
                )
            };
            assert_eq!(cn, rn, "{}: LZ4_decompress_safe return", label);
            assert_eq!(cn, len as c_int, "{}: safe should decode fully", label);
            assert_bytes_eq(&format!("{} safe", label), &cout[..len], &rout[..len]);
            assert_eq!(&cout[..len], &src[..], "{}: safe content vs original", label);

            // --- LZ4_decompress_safe with an OVERSIZED capacity
            let mut cout = vec![0xAAu8; len + 137];
            let mut rout = vec![0xAAu8; len + 137];
            let cn = unsafe {
                cds(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 137) as c_int,
                )
            };
            let rn = unsafe {
                rds(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    (len + 137) as c_int,
                )
            };
            assert_eq!(cn, rn, "{}: safe oversized return", label);
            assert_bytes_eq(
                &format!("{} safe oversized (incl. untouched tail)", label),
                &cout,
                &rout,
            );

            // --- LZ4_decompress_fast (deprecated; needs exact original size)
            let mut cout = vec![0xAAu8; len.max(1)];
            let mut rout = vec![0xAAu8; len.max(1)];
            let cn = unsafe {
                cdf(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    len as c_int,
                )
            };
            let rn = unsafe {
                rdf(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    len as c_int,
                )
            };
            assert_eq!(cn, rn, "{}: LZ4_decompress_fast return", label);
            assert_bytes_eq(&format!("{} fast", label), &cout[..len], &rout[..len]);
        }
    }
}

#[test]
fn decompress_safe_partial_target_sweep() {
    let (cp, rp) = both::<FnDecompPartial>("LZ4_decompress_safe_partial");
    let mut rng = Rng::new(0x9A27);

    for shape in 0..N_SHAPES {
        for &len in &[0usize, 1, 5, 12, 13, 100, 1000, 20000, 65547, 90000] {
            let src = gen_shape(&mut rng, shape, len);
            let label = format!("partial shape={} len={}", shape_name(shape), len);
            let comp = diff_compress_default(&src, &label);

            // Sweep BOTH targetOutputSize and dstCapacity independently — the
            // C code treats `targetOutputSize > dstCapacity` specially.
            let targets: Vec<usize> = vec![0, 1, 2, 4, 5, len / 4, len / 2, len, len + 1, len + 99];
            let caps: Vec<usize> = vec![0, 1, 2, 4, 5, len / 4, len / 2, len, len + 1, len + 99];
            for &t in &targets {
                for &cap in &caps {
                    let mut cout = vec![0xAAu8; cap + 1];
                    let mut rout = vec![0xAAu8; cap + 1];
                    let cn = unsafe {
                        cp(
                            comp.as_ptr() as *const c_char,
                            cout.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            t as c_int,
                            cap as c_int,
                        )
                    };
                    let rn = unsafe {
                        rp(
                            comp.as_ptr() as *const c_char,
                            rout.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            t as c_int,
                            cap as c_int,
                        )
                    };
                    let l2 = format!("{} target={} cap={}", label, t, cap);
                    assert_eq!(cn, rn, "{}: return", l2);
                    // Compare the entire destination buffer: any divergence in
                    // how far each implementation wrote is a real difference.
                    assert_bytes_eq(&l2, &cout, &rout);
                }
            }
        }
    }
}

// ===========================================================================
// CONFIGS row group: dictionary-based decompression (dict_directive axis)
// ===========================================================================

#[test]
fn decompress_using_dict_axis() {
    let (cud, rud) = both::<FnDecompUsingDict>("LZ4_decompress_safe_usingDict");
    let (cfd, rfd) = both::<FnDecompFastUsingDict>("LZ4_decompress_fast_usingDict");
    let (cpd, rpd) =
        both::<FnDecompPartialUsingDict>("LZ4_decompress_safe_partial_usingDict");
    let mut rng = Rng::new(0xD1C7);

    // Dict sizes spanning: none, tiny, dictSmall(<16KB) issue path, just under
    // and over 64 KB (where the C truncates history to the last 64 KB).
    for &dict_len in &[0usize, 1, 4, 100, 4096, 16383, 16384, 65535, 65536, 70000] {
        for shape in 0..N_SHAPES {
            for &len in &[1usize, 13, 500, 5000, 65547, 90000] {
                let dict = gen_shape(&mut rng, shape, dict_len);
                let src = gen_shape(&mut rng, shape, len);
                let label = format!(
                    "usingDict dict={} shape={} len={}",
                    dict_len,
                    shape_name(shape),
                    len
                );
                // Produce the compressed frame with the streaming dictionary
                // API so the payload actually references the dictionary.
                let comp = compress_with_dict(&dict, &src, &label);

                let dptr = if dict.is_empty() {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };

                let mut cout = vec![0xAAu8; len + 32];
                let mut rout = vec![0xAAu8; len + 32];
                let cn = unsafe {
                    cud(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        comp.len() as c_int,
                        (len + 32) as c_int,
                        dptr,
                        dict_len as c_int,
                    )
                };
                let rn = unsafe {
                    rud(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        comp.len() as c_int,
                        (len + 32) as c_int,
                        dptr,
                        dict_len as c_int,
                    )
                };
                assert_eq!(cn, rn, "{}: safe_usingDict return", label);
                assert_bytes_eq(&format!("{} safe_usingDict", label), &cout, &rout);
                assert_eq!(cn, len as c_int, "{}: should decode fully", label);
                assert_eq!(&cout[..len], &src[..], "{}: content", label);

                // fast_usingDict
                let mut cout = vec![0xAAu8; len + 32];
                let mut rout = vec![0xAAu8; len + 32];
                let cn = unsafe {
                    cfd(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        dptr,
                        dict_len as c_int,
                    )
                };
                let rn = unsafe {
                    rfd(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        dptr,
                        dict_len as c_int,
                    )
                };
                assert_eq!(cn, rn, "{}: fast_usingDict return", label);
                assert_bytes_eq(&format!("{} fast_usingDict", label), &cout, &rout);

                // partial_usingDict across target sizes
                for &t in &[0usize, 1, len / 2, len, len + 5] {
                    let mut cout = vec![0xAAu8; len + 32];
                    let mut rout = vec![0xAAu8; len + 32];
                    let cn = unsafe {
                        cpd(
                            comp.as_ptr() as *const c_char,
                            cout.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            t as c_int,
                            (len + 32) as c_int,
                            dptr,
                            dict_len as c_int,
                        )
                    };
                    let rn = unsafe {
                        rpd(
                            comp.as_ptr() as *const c_char,
                            rout.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            t as c_int,
                            (len + 32) as c_int,
                            dptr,
                            dict_len as c_int,
                        )
                    };
                    let l2 = format!("{} partial_usingDict target={}", label, t);
                    assert_eq!(cn, rn, "{}: return", l2);
                    assert_bytes_eq(&l2, &cout, &rout);
                }
            }
        }
    }
}

/// Helper: compress `src` against `dict` using the C streaming API, asserting
/// the Rust streaming API produces byte-identical output on the way.
fn compress_with_dict(dict: &[u8], src: &[u8], label: &str) -> Vec<u8> {
    type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
    type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type FnCompCont =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;

    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompCont>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    let bound = unsafe { cb(src.len() as c_int) }.max(1) as usize;
    let dptr = if dict.is_empty() {
        std::ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };

    unsafe {
        let cst = c_cs();
        let rst = r_cs();
        assert!(!cst.is_null() && !rst.is_null(), "{}: createStream", label);

        let cl = c_ld(cst, dptr, dict.len() as c_int);
        let rl = r_ld(rst, dptr, dict.len() as c_int);
        assert_eq!(cl, rl, "{}: LZ4_loadDict return", label);

        let mut cdst = vec![0xAAu8; bound];
        let mut rdst = vec![0xAAu8; bound];
        let cn = c_cc(
            cst,
            src.as_ptr() as *const c_char,
            cdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            1,
        );
        let rn = r_cc(
            rst,
            src.as_ptr() as *const c_char,
            rdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            1,
        );
        assert_eq!(cn, rn, "{}: compress_fast_continue return", label);
        assert!(cn > 0 || src.is_empty(), "{}: compression failed", label);
        assert_bytes_eq(
            &format!("{} compress_fast_continue", label),
            &cdst[..cn.max(0) as usize],
            &rdst[..cn.max(0) as usize],
        );

        assert_eq!(c_fs(cst), r_fs(rst), "{}: freeStream", label);
        cdst.truncate(cn.max(0) as usize);
        cdst
    }
}
