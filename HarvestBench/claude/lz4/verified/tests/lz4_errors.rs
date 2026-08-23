//! Phase C — error-path differential tests for lz4.c.
//!
//! One test (or one clearly-labelled block) per row of the `lz4.c` section of
//! `ERRORS.md`. Every case asserts that C and Rust return the SAME error
//! code / sentinel — not merely that both "failed somehow".
//!
//! Rows that are NOT covered are documented inline with the reason
//! (compile-time `#error`s, un-forceable allocation failures, and rows the C
//! itself documents as undefined behaviour, which would segfault both
//! libraries identically and tell us nothing).

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

const SENTINEL: u8 = 0xAA;

type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnRingBuf = unsafe extern "C" fn(c_int) -> c_int;
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
type FnDecompPartialUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int, *const c_char, c_int) -> c_int;
type FnDecompFastUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnLoadDictInternal =
    unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
type FnAttachDict = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnCompContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCreateStreamDecode = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStreamDecode = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnSetStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnDecompSafeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecompFastContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnSizeofState = unsafe extern "C" fn() -> c_int;
type FnResetStreamState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

fn stream_size() -> usize {
    let (c, r) = both::<FnSizeofState>("LZ4_sizeofStreamState");
    let cs = unsafe { c() } as usize;
    assert_eq!(cs, unsafe { r() } as usize, "LZ4_sizeofStreamState");
    cs
}

/// Compress `src` with C, returning the compressed bytes (used to build valid
/// inputs that are then corrupted).
fn compress_c(src: &[u8]) -> Vec<u8> {
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let (cc, _) = both::<FnCompDefault>("LZ4_compress_default");
    let bound = unsafe { cb(src.len() as c_int) }.max(1) as usize;
    let mut dst = vec![0u8; bound];
    let n = unsafe {
        cc(
            src.as_ptr() as *const c_char,
            dst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
        )
    };
    assert!(n > 0, "helper compression failed");
    dst.truncate(n as usize);
    dst
}

// ===========================================================================
// ERRORS.md rows 1-2 — LZ4_compressBound rejects out-of-range sizes
// ===========================================================================

#[test]
fn row_01_02_compress_bound_out_of_range() {
    let (c, r) = both::<FnBound>("LZ4_compressBound");
    // row 1: isize < 0 -> 0 ; row 2: isize > LZ4_MAX_INPUT_SIZE -> 0
    let cases: Vec<c_int> = vec![
        c_int::MIN,
        -1000,
        -1,                                  // row 1
        LZ4_MAX_INPUT_SIZE as c_int + 1,     // row 2
        0x7E00_0001,
        0x7FFF_FFFF,
        c_int::MAX,
        // accept boundary for contrast
        0,
        1,
        LZ4_MAX_INPUT_SIZE as c_int,
    ];
    for &v in &cases {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        assert_eq!(cv, rv, "LZ4_compressBound({}) C={} Rust={}", v, cv, rv);
    }
    // Explicitly pin the documented values.
    assert_eq!(unsafe { c(-1) }, 0, "row 1: negative must give 0");
    assert_eq!(
        unsafe { c(LZ4_MAX_INPUT_SIZE as c_int + 1) },
        0,
        "row 2: oversized must give 0"
    );
}

// ===========================================================================
// ERRORS.md rows 3-8 — srcSize / dstCapacity rejection in LZ4_compress_generic
// ===========================================================================

#[test]
fn row_03_08_compress_srcsize_and_dstcapacity_rejection() {
    let (cd, rd) = both::<FnCompDefault>("LZ4_compress_default");
    let (cf, rf) = both::<FnCompFast>("LZ4_compress_fast");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    let src = vec![7u8; 4096];
    let bound = unsafe { cb(4096) } as usize;
    let mut cdst = vec![SENTINEL; bound];
    let mut rdst = vec![SENTINEL; bound];

    // rows 3 & 4: srcSize < 0, and srcSize > LZ4_MAX_INPUT_SIZE.
    // NOTE: we pass a LYING srcSize with a real (small) buffer. This is safe
    // because the C rejects the size BEFORE touching `src` (lz4.c:1360).
    let bad_sizes: Vec<c_int> = vec![
        c_int::MIN,
        -1_000_000,
        -1,                              // row 3
        LZ4_MAX_INPUT_SIZE as c_int + 1, // row 4
        0x7FFF_FFFF,
        c_int::MAX,
    ];
    for &n in &bad_sizes {
        let cv = unsafe {
            cd(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                n,
                bound as c_int,
            )
        };
        let rv = unsafe {
            rd(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                n,
                bound as c_int,
            )
        };
        assert_eq!(cv, rv, "LZ4_compress_default srcSize={}", n);
        assert_eq!(cv, 0, "rows 3/4: srcSize={} must return 0", n);
        assert_bytes_eq(
            &format!("rows 3/4 dst untouched srcSize={}", n),
            &cdst,
            &rdst,
        );
    }

    // row 7: srcSize == 0 AND dstCapacity <= 0  -> 0
    for &cap in &[c_int::MIN, -1, 0] {
        let cv = unsafe {
            cd(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                0,
                cap,
            )
        };
        let rv = unsafe {
            rd(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                0,
                cap,
            )
        };
        assert_eq!(cv, rv, "row 7: srcSize=0 dstCapacity={}", cap);
        assert_eq!(cv, 0, "row 7: srcSize=0 dstCapacity={} must return 0", cap);
    }

    // row 8: srcSize == 0 with dstCapacity >= 1 -> returns 1 (writes one 0 byte)
    let mut cdst1 = vec![SENTINEL; 8];
    let mut rdst1 = vec![SENTINEL; 8];
    let cv = unsafe {
        cd(
            src.as_ptr() as *const c_char,
            cdst1.as_mut_ptr() as *mut c_char,
            0,
            1,
        )
    };
    let rv = unsafe {
        rd(
            src.as_ptr() as *const c_char,
            rdst1.as_mut_ptr() as *mut c_char,
            0,
            1,
        )
    };
    assert_eq!(cv, rv, "row 8: srcSize=0 dstCapacity=1 return");
    assert_eq!(cv, 1, "row 8: must return 1");
    assert_bytes_eq("row 8: emitted empty block", &cdst1, &rdst1);

    // Same rejections through LZ4_compress_fast (rows 3/4/7 share the branch).
    for &n in &bad_sizes {
        for &acc in &[1i32, 7] {
            let cv = unsafe {
                cf(
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    n,
                    bound as c_int,
                    acc,
                )
            };
            let rv = unsafe {
                rf(
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    n,
                    bound as c_int,
                    acc,
                )
            };
            assert_eq!(cv, rv, "LZ4_compress_fast srcSize={} accel={}", n, acc);
            assert_eq!(cv, 0, "rows 3/4 via fast: srcSize={}", n);
        }
    }
}

#[test]
fn row_05_compress_fast_continue_bad_input_size() {
    // row 5: inputSize < 0 or > LZ4_MAX_INPUT_SIZE via the streaming entry.
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    let src = vec![3u8; 8192];
    let bound = unsafe { cb(8192) } as usize;
    unsafe {
        let cst = c_cs();
        let rst = r_cs();
        for &n in &[
            c_int::MIN,
            -1,
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            c_int::MAX,
        ] {
            let mut cdst = vec![SENTINEL; bound];
            let mut rdst = vec![SENTINEL; bound];
            let cv = c_cc(
                cst,
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                n,
                bound as c_int,
                1,
            );
            let rv = r_cc(
                rst,
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                n,
                bound as c_int,
                1,
            );
            assert_eq!(cv, rv, "row 5: compress_fast_continue inputSize={}", n);
            assert_eq!(cv, 0, "row 5: inputSize={} must return 0", n);
            assert_bytes_eq(&format!("row 5 dst inputSize={}", n), &cdst, &rdst);
        }
        assert_eq!(c_fs(cst), r_fs(rst));
    }
}

#[test]
fn row_06_09_23_compress_destsize_rejections() {
    let (cc, rc) = both::<FnCompDestSize>("LZ4_compress_destSize");
    let src = vec![9u8; 4096];

    // row 6: *srcSizePtr < 0 or > LZ4_MAX_INPUT_SIZE
    for &n in &[
        c_int::MIN,
        -1,
        LZ4_MAX_INPUT_SIZE as c_int + 1,
        c_int::MAX,
    ] {
        let mut c_ss = n;
        let mut r_ss = n;
        let mut cdst = vec![SENTINEL; 4096];
        let mut rdst = vec![SENTINEL; 4096];
        let cv = unsafe {
            cc(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                &mut c_ss,
                4096,
            )
        };
        let rv = unsafe {
            rc(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                &mut r_ss,
                4096,
            )
        };
        assert_eq!(cv, rv, "row 6: destSize srcSize={}", n);
        assert_eq!(c_ss, r_ss, "row 6: srcSize out-param for {}", n);
        assert_bytes_eq(&format!("row 6 dst srcSize={}", n), &cdst, &rdst);
    }

    // row 9: targetDstSize < 1 while *srcSizePtr >= 1 -> 0
    for &target in &[c_int::MIN, -5, 0] {
        let mut c_ss = 4096;
        let mut r_ss = 4096;
        let mut cdst = vec![SENTINEL; 64];
        let mut rdst = vec![SENTINEL; 64];
        let cv = unsafe {
            cc(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                &mut c_ss,
                target,
            )
        };
        let rv = unsafe {
            rc(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                &mut r_ss,
                target,
            )
        };
        assert_eq!(cv, rv, "row 9: targetDstSize={}", target);
        assert_eq!(cv, 0, "row 9: targetDstSize={} must return 0", target);
        assert_eq!(c_ss, r_ss, "row 9: srcSize out-param target={}", target);
    }

    // row 23: targetDstSize < compressBound -> silently truncates the input and
    // rewrites *srcSizePtr. Assert both libraries truncate identically.
    let mut rng = Rng::new(0x2317);
    for shape in 0..N_SHAPES {
        let s = gen_shape(&mut rng, shape, 30000);
        for &target in &[1usize, 2, 5, 17, 100, 1000, 5000] {
            let mut c_ss = s.len() as c_int;
            let mut r_ss = s.len() as c_int;
            let mut cdst = vec![SENTINEL; target + 16];
            let mut rdst = vec![SENTINEL; target + 16];
            let cv = unsafe {
                cc(
                    s.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    &mut c_ss,
                    target as c_int,
                )
            };
            let rv = unsafe {
                rc(
                    s.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    &mut r_ss,
                    target as c_int,
                )
            };
            let l = format!("row 23: shape={} target={}", shape_name(shape), target);
            assert_eq!(cv, rv, "{}: return", l);
            assert_eq!(c_ss, r_ss, "{}: truncated srcSize", l);
            assert_bytes_eq(&l, &cdst, &rdst);
            assert!(
                c_ss <= s.len() as c_int,
                "{}: srcSize must not grow ({} > {})",
                l,
                c_ss,
                s.len()
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 10-12 — limitedOutput dstCapacity exhaustion returns 0
// ===========================================================================

#[test]
fn row_10_12_limited_output_capacity_exhaustion() {
    // Sweep EVERY dstCapacity from 0 up to just past the true compressed size,
    // for every data shape. This necessarily crosses all three overflow checks
    // (literal length, match length, and last-literal run).
    let (cd, rd) = both::<FnCompDefault>("LZ4_compress_default");
    let mut rng = Rng::new(0x1012);

    for shape in 0..N_SHAPES {
        for &len in &[13usize, 40, 100, 300, 1000, 5000] {
            let src = gen_shape(&mut rng, shape, len);
            let real = compress_c(&src).len();
            for cap in 0..=(real + 4) {
                let mut cdst = vec![SENTINEL; cap.max(1)];
                let mut rdst = vec![SENTINEL; cap.max(1)];
                let cv = unsafe {
                    cd(
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    )
                };
                let rv = unsafe {
                    rd(
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    )
                };
                let l = format!(
                    "rows 10-12: shape={} len={} cap={} (real={})",
                    shape_name(shape),
                    len,
                    cap,
                    real
                );
                assert_eq!(cv, rv, "{}: return", l);
                assert_bytes_eq(&l, &cdst, &rdst);
                if cap < real {
                    assert_eq!(cv, 0, "{}: undersized capacity must return 0", l);
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 15-20 — acceleration clamping (silent "rejection")
// ===========================================================================

#[test]
fn row_15_20_acceleration_clamping() {
    let (cf, rf) = both::<FnCompFast>("LZ4_compress_fast");
    let (cx, rx) = both::<FnCompFastExt>("LZ4_compress_fast_extState");
    let (cxf, rxf) = both::<FnCompFastExt>("LZ4_compress_fast_extState_fastReset");
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let ssz = stream_size();

    let mut rng = Rng::new(0x1520);
    let src = gen_shape(&mut rng, 4, 20000);
    let bound = unsafe { cb(src.len() as c_int) } as usize;

    // Compress once at the clamp TARGET to obtain the reference output.
    let reference = |acc: c_int| -> Vec<u8> {
        let mut d = vec![SENTINEL; bound];
        let n = unsafe {
            cf(
                src.as_ptr() as *const c_char,
                d.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            )
        };
        d.truncate(n.max(0) as usize);
        d
    };
    let ref_1 = reference(1);
    let ref_max = reference(LZ4_ACCELERATION_MAX);

    // rows 15/16 on LZ4_compress_fast
    for &acc in &[c_int::MIN, -12345, -1, 0] {
        let mut cdst = vec![SENTINEL; bound];
        let mut rdst = vec![SENTINEL; bound];
        let cv = unsafe {
            cf(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            )
        };
        let rv = unsafe {
            rf(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            )
        };
        assert_eq!(cv, rv, "row 15: accel={}", acc);
        assert_bytes_eq(&format!("row 15: accel={}", acc), &cdst, &rdst);
        assert_eq!(
            &cdst[..cv as usize],
            &ref_1[..],
            "row 15: accel={} must behave exactly like accel=1",
            acc
        );
    }
    for &acc in &[LZ4_ACCELERATION_MAX + 1, 1_000_000, c_int::MAX] {
        let mut cdst = vec![SENTINEL; bound];
        let mut rdst = vec![SENTINEL; bound];
        let cv = unsafe {
            cf(
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            )
        };
        let rv = unsafe {
            rf(
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            )
        };
        assert_eq!(cv, rv, "row 16: accel={}", acc);
        assert_bytes_eq(&format!("row 16: accel={}", acc), &cdst, &rdst);
        assert_eq!(
            &cdst[..cv as usize],
            &ref_max[..],
            "row 16: accel={} must behave exactly like accel=LZ4_ACCELERATION_MAX",
            acc
        );
    }

    // rows 17/18 on LZ4_compress_fast_extState_fastReset, plus extState itself
    for (name, cfn, rfn) in [
        ("LZ4_compress_fast_extState", cx, rx),
        ("LZ4_compress_fast_extState_fastReset", cxf, rxf),
    ] {
        for &acc in &[
            c_int::MIN,
            -1,
            0,
            1,
            LZ4_ACCELERATION_MAX,
            LZ4_ACCELERATION_MAX + 1,
            c_int::MAX,
        ] {
            let mut cstate = AlignedBuf::new(ssz, 64);
            let mut rstate = AlignedBuf::new(ssz, 64);
            let mut cdst = vec![SENTINEL; bound];
            let mut rdst = vec![SENTINEL; bound];
            let cv = unsafe {
                cfn(
                    cstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    cdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    acc,
                )
            };
            let rv = unsafe {
                rfn(
                    rstate.as_mut_ptr() as *mut c_void,
                    src.as_ptr() as *const c_char,
                    rdst.as_mut_ptr() as *mut c_char,
                    src.len() as c_int,
                    bound as c_int,
                    acc,
                )
            };
            assert_eq!(cv, rv, "rows 17/18: {} accel={}", name, acc);
            assert_bytes_eq(&format!("rows 17/18: {} accel={}", name, acc), &cdst, &rdst);
        }
    }

    // rows 19/20 on LZ4_compress_fast_continue
    unsafe {
        for &acc in &[
            c_int::MIN,
            -1,
            0,
            1,
            LZ4_ACCELERATION_MAX,
            LZ4_ACCELERATION_MAX + 1,
            c_int::MAX,
        ] {
            let cst = c_cs();
            let rst = r_cs();
            let mut cdst = vec![SENTINEL; bound];
            let mut rdst = vec![SENTINEL; bound];
            let cv = c_cc(
                cst,
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            );
            let rv = r_cc(
                rst,
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                acc,
            );
            assert_eq!(cv, rv, "rows 19/20: continue accel={}", acc);
            assert_bytes_eq(&format!("rows 19/20: accel={}", acc), &cdst, &rdst);
            assert_eq!(c_fs(cst), r_fs(rst));
        }
    }
}

// ===========================================================================
// ERRORS.md rows 26-28 — LZ4_initStream rejects NULL / too small / misaligned
// ===========================================================================

#[test]
fn row_26_28_init_stream_rejections() {
    let (c_is, r_is) = both::<FnInitStream>("LZ4_initStream");
    let ssz = stream_size();

    unsafe {
        // row 26: buffer == NULL -> NULL
        for &sz in &[0usize, 1, ssz, ssz * 2] {
            let cp = c_is(std::ptr::null_mut(), sz);
            let rp = r_is(std::ptr::null_mut(), sz);
            assert!(cp.is_null(), "row 26: C must return NULL for NULL buffer");
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 26: NULL buffer size={} nullness",
                sz
            );
        }

        // row 27: size < sizeof(LZ4_stream_t) -> NULL
        let mut buf = AlignedBuf::new(ssz, 64);
        for &sz in &[0usize, 1, 8, ssz / 2, ssz - 1] {
            let cp = c_is(buf.as_mut_ptr() as *mut c_void, sz);
            let rp = r_is(buf.as_mut_ptr() as *mut c_void, sz);
            assert!(cp.is_null(), "row 27: C must reject size={} (< {})", sz, ssz);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 27: size={} nullness (C={:?} Rust={:?})",
                sz,
                cp,
                rp
            );
        }
        // accept boundary
        let cp = c_is(buf.as_mut_ptr() as *mut c_void, ssz);
        let rp = r_is(buf.as_mut_ptr() as *mut c_void, ssz);
        assert!(!cp.is_null(), "row 27: size == sizeof must be accepted");
        assert_eq!(cp.is_null(), rp.is_null(), "row 27: accept boundary");

        // row 28: misaligned buffer -> NULL. Try every offset 1..=7 from an
        // 8-byte-aligned base (LZ4_stream_t requires natural alignment).
        for off in 1..8usize {
            let mut mb = AlignedBuf::with_offset(ssz + 8, 64, off);
            let cp = c_is(mb.as_mut_ptr() as *mut c_void, ssz);
            let rp = r_is(mb.as_mut_ptr() as *mut c_void, ssz);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row 28: misalign offset={} nullness (C={:?} Rust={:?})",
                off,
                cp,
                rp
            );
        }
    }
}

// ===========================================================================
// ERRORS.md rows 30, 86 — free-on-NULL is supported and returns 0
// ===========================================================================

#[test]
fn row_30_86_free_null_pointers() {
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_fd, r_fd) = both::<FnFreeStreamDecode>("LZ4_freeStreamDecode");
    unsafe {
        // row 30
        let cv = c_fs(std::ptr::null_mut());
        let rv = r_fs(std::ptr::null_mut());
        assert_eq!(cv, rv, "row 30: LZ4_freeStream(NULL)");
        assert_eq!(cv, 0, "row 30: LZ4_freeStream(NULL) must return 0");
        // row 86
        let cv = c_fd(std::ptr::null_mut());
        let rv = r_fd(std::ptr::null_mut());
        assert_eq!(cv, rv, "row 86: LZ4_freeStreamDecode(NULL)");
        assert_eq!(cv, 0, "row 86: LZ4_freeStreamDecode(NULL) must return 0");
    }
}

// ===========================================================================
// ERRORS.md rows 31-32 — LZ4_loadDict / LZ4_loadDictSlow size handling
// ===========================================================================

#[test]
fn row_31_32_load_dict_size_handling() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let ssz = stream_size();

    let dict = vec![0x5Au8; 100_000];

    for name in ["LZ4_loadDict", "LZ4_loadDictSlow"] {
        let (c_ld, r_ld) = both::<FnLoadDict>(name);
        unsafe {
            // row 31: dictSize < HASH_UNIT (8 on 64-bit), incl. 0 and negative
            //         -> returns 0, no dictionary registered
            for &n in &[c_int::MIN, -100, -1, 0, 1, 2, 3, 4, 5, 6, 7] {
                let cst = c_cs();
                let rst = r_cs();
                let cv = c_ld(cst, dict.as_ptr() as *const c_char, n);
                let rv = r_ld(rst, dict.as_ptr() as *const c_char, n);
                assert_eq!(cv, rv, "row 31: {}(dictSize={})", name, n);
                assert_eq!(cv, 0, "row 31: {}(dictSize={}) must return 0", name, n);
                assert_bytes_eq(
                    &format!("row 31: {} state dictSize={}", name, n),
                    std::slice::from_raw_parts(cst as *const u8, ssz),
                    std::slice::from_raw_parts(rst as *const u8, ssz),
                );
                assert_eq!(c_fs(cst), r_fs(rst));
            }
            // accept boundary: exactly HASH_UNIT
            for &n in &[8i32, 9, 16] {
                let cst = c_cs();
                let rst = r_cs();
                let cv = c_ld(cst, dict.as_ptr() as *const c_char, n);
                let rv = r_ld(rst, dict.as_ptr() as *const c_char, n);
                assert_eq!(cv, rv, "row 31 boundary: {}(dictSize={})", name, n);
                assert_eq!(c_fs(cst), r_fs(rst));
            }
            // row 32: dictSize > 64 KB -> retains only the LAST 64 KB, returns 65536
            for &n in &[65536i32 + 1, 70000, 100_000] {
                let cst = c_cs();
                let rst = r_cs();
                let cv = c_ld(cst, dict.as_ptr() as *const c_char, n);
                let rv = r_ld(rst, dict.as_ptr() as *const c_char, n);
                assert_eq!(cv, rv, "row 32: {}(dictSize={})", name, n);
                assert_eq!(
                    cv, 65536,
                    "row 32: {}(dictSize={}) must return 65536",
                    name, n
                );
                assert_bytes_eq(
                    &format!("row 32: {} state dictSize={}", name, n),
                    std::slice::from_raw_parts(cst as *const u8, ssz),
                    std::slice::from_raw_parts(rst as *const u8, ssz),
                );
                assert_eq!(c_fs(cst), r_fs(rst));
            }
            // NULL dictionary with size 0 is the documented "reset" usage.
            let cst = c_cs();
            let rst = r_cs();
            let cv = c_ld(cst, std::ptr::null(), 0);
            let rv = r_ld(rst, std::ptr::null(), 0);
            assert_eq!(cv, rv, "row 31: {}(NULL, 0)", name);
            assert_eq!(c_fs(cst), r_fs(rst));
        }
    }
}

#[test]
fn load_dict_internal_out_of_range_enum() {
    // `LZ4_loadDict_internal`'s 4th parameter is a C enum
    // `LoadDict_mode_e { _ld_fast = 0, _ld_slow = 1 }`. A C enum accepts ANY
    // int across the FFI boundary, so values with no valid variant are real
    // inputs that the C handles (its only test is `if (_ld == _ld_slow)`).
    // The Rust must handle them identically.
    let (c_ld, r_ld) = both::<FnLoadDictInternal>("LZ4_loadDict_internal");
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let ssz = stream_size();
    let dict = vec![0x33u8; 40000];

    unsafe {
        for &mode in &[
            c_int::MIN,
            -2,
            -1,
            0, // _ld_fast
            1, // _ld_slow
            2, // out of range
            3,
            7,
            255,
            256,
            65535,
            c_int::MAX,
        ] {
            for &dsz in &[0i32, 8, 4096, 40000] {
                let cst = c_cs();
                let rst = r_cs();
                let cv = c_ld(cst, dict.as_ptr() as *const c_char, dsz, mode);
                let rv = r_ld(rst, dict.as_ptr() as *const c_char, dsz, mode);
                assert_eq!(
                    cv, rv,
                    "LZ4_loadDict_internal(mode={}, dictSize={}) return",
                    mode, dsz
                );
                assert_bytes_eq(
                    &format!(
                        "LZ4_loadDict_internal(mode={}, dictSize={}) STATE",
                        mode, dsz
                    ),
                    std::slice::from_raw_parts(cst as *const u8, ssz),
                    std::slice::from_raw_parts(rst as *const u8, ssz),
                );
                assert_eq!(c_fs(cst), r_fs(rst));
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 33-34 — LZ4_attach_dictionary silent detach
// ===========================================================================

#[test]
fn row_33_34_attach_dictionary_edge_cases() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ad, r_ad) = both::<FnAttachDict>("LZ4_attach_dictionary");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let ssz = stream_size();

    let mut rng = Rng::new(0x3334);
    let src = gen_shape(&mut rng, 5, 20000);
    let bound = unsafe { cb(src.len() as c_int) } as usize;

    unsafe {
        // row 33: dictionaryStream == NULL -> silently detaches
        let cst = c_cs();
        let rst = r_cs();
        c_ad(cst, std::ptr::null());
        r_ad(rst, std::ptr::null());
        assert_bytes_eq(
            "row 33: attach_dictionary(NULL) STATE",
            std::slice::from_raw_parts(cst as *const u8, ssz),
            std::slice::from_raw_parts(rst as *const u8, ssz),
        );
        let mut cdst = vec![SENTINEL; bound];
        let mut rdst = vec![SENTINEL; bound];
        let cv = c_cc(
            cst,
            src.as_ptr() as *const c_char,
            cdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            1,
        );
        let rv = r_cc(
            rst,
            src.as_ptr() as *const c_char,
            rdst.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            bound as c_int,
            1,
        );
        assert_eq!(cv, rv, "row 33: compress after NULL attach");
        assert_bytes_eq("row 33: output after NULL attach", &cdst, &rdst);
        assert_eq!(c_fs(cst), r_fs(rst));

        // row 34: dictionary stream whose dictSize == 0 -> NOT attached.
        // Build it by loading a dict too small to register (see row 31).
        let dict = vec![0x77u8; 4];
        for &tiny in &[0i32, 1, 4, 7] {
            let cdict = c_cs();
            let rdict = r_cs();
            assert_eq!(
                c_ld(cdict, dict.as_ptr() as *const c_char, tiny.min(4)),
                r_ld(rdict, dict.as_ptr() as *const c_char, tiny.min(4)),
                "row 34: loadDict tiny={}",
                tiny
            );
            let cst = c_cs();
            let rst = r_cs();
            c_ad(cst, cdict as *const c_void);
            r_ad(rst, rdict as *const c_void);
            assert_bytes_eq(
                &format!("row 34: STATE after attach of empty dict (tiny={})", tiny),
                std::slice::from_raw_parts(cst as *const u8, ssz),
                std::slice::from_raw_parts(rst as *const u8, ssz),
            );
            let mut cdst = vec![SENTINEL; bound];
            let mut rdst = vec![SENTINEL; bound];
            let cv = c_cc(
                cst,
                src.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                1,
            );
            let rv = r_cc(
                rst,
                src.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                1,
            );
            assert_eq!(cv, rv, "row 34: compress return tiny={}", tiny);
            assert_bytes_eq(&format!("row 34: output tiny={}", tiny), &cdst, &rdst);
            assert_eq!(c_fs(cst), r_fs(rst));
            assert_eq!(c_fs(cdict), r_fs(rdict));
        }
    }
}

// ===========================================================================
// ERRORS.md rows 35-36 — LZ4_saveDict clamping
// ===========================================================================

#[test]
fn row_35_36_save_dict_clamping() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (c_sv, r_sv) = both::<FnSaveDict>("LZ4_saveDict");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");

    let mut rng = Rng::new(0x3536);
    let dict = gen_shape(&mut rng, 3, 100_000);

    // Build streams with differing amounts of retained history so that both
    // clamp branches (64 KB cap, and "more than exists") are hit.
    for &(dict_len, blk_len) in &[
        (0usize, 100usize),
        (8, 100),
        (1000, 100),
        (65536, 100),
        (100_000, 100),
        (0, 70000),
        (1000, 70000),
    ] {
        let blk = gen_shape(&mut rng, 4, blk_len);
        // row 35: dictSize > 64 KB (and negative -> huge U32) -> clamp to 65536
        // row 36: dictSize > available history -> clamp to available
        for &want in &[
            c_int::MIN,
            -100_000,
            -1,
            0,
            1,
            8,
            100,
            1000,
            65535,
            65536,
            65537,
            100_000,
            c_int::MAX,
        ] {
            unsafe {
                let cst = c_cs();
                let rst = r_cs();
                let dptr = if dict_len == 0 {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                assert_eq!(
                    c_ld(cst, dptr, dict_len as c_int),
                    r_ld(rst, dptr, dict_len as c_int),
                    "rows 35/36: loadDict {}",
                    dict_len
                );
                let bound = cb(blk_len as c_int) as usize;
                let mut cd = vec![SENTINEL; bound];
                let mut rd = vec![SENTINEL; bound];
                assert_eq!(
                    c_cc(
                        cst,
                        blk.as_ptr() as *const c_char,
                        cd.as_mut_ptr() as *mut c_char,
                        blk_len as c_int,
                        bound as c_int,
                        1
                    ),
                    r_cc(
                        rst,
                        blk.as_ptr() as *const c_char,
                        rd.as_mut_ptr() as *mut c_char,
                        blk_len as c_int,
                        bound as c_int,
                        1
                    ),
                    "rows 35/36: compress"
                );

                // A generously sized safe buffer so a clamp cannot overflow it.
                let safe_len = 200_000usize;
                let mut csafe = vec![SENTINEL; safe_len];
                let mut rsafe = vec![SENTINEL; safe_len];
                let cv = c_sv(cst, csafe.as_mut_ptr() as *mut c_char, want);
                let rv = r_sv(rst, rsafe.as_mut_ptr() as *mut c_char, want);
                let l = format!(
                    "rows 35/36: dict={} blk={} maxDictSize={}",
                    dict_len, blk_len, want
                );
                assert_eq!(cv, rv, "{}: return", l);
                assert!(
                    cv >= 0 && cv <= 65536,
                    "{}: saveDict must return 0..=65536, got {}",
                    l,
                    cv
                );
                assert_bytes_eq(&l, &csafe, &rsafe);
                assert_eq!(c_fs(cst), r_fs(rst));
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 38-39 — silent dictionary discard / shrink in continue
// ===========================================================================

#[test]
fn row_38_39_continue_dictionary_discard_and_overlap() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_ld, r_ld) = both::<FnLoadDict>("LZ4_loadDict");
    let (c_cc, r_cc) = both::<FnCompContinue>("LZ4_compress_fast_continue");
    let (cb, _) = both::<FnBound>("LZ4_compressBound");
    let ssz = stream_size();

    let mut rng = Rng::new(0x3839);

    // row 38: registered dictSize < 4 -> dictionary silently discarded.
    // (loadDict already refuses < 8, so drive it via a tiny saved history.)
    for &dsz in &[0i32, 1, 2, 3, 8, 9] {
        let dict = gen_shape(&mut rng, 2, 64);
        let src = gen_shape(&mut rng, 2, 5000);
        let bound = unsafe { cb(src.len() as c_int) } as usize;
        unsafe {
            let cst = c_cs();
            let rst = r_cs();
            assert_eq!(
                c_ld(cst, dict.as_ptr() as *const c_char, dsz),
                r_ld(rst, dict.as_ptr() as *const c_char, dsz),
                "row 38: loadDict {}",
                dsz
            );
            let mut cd = vec![SENTINEL; bound];
            let mut rd = vec![SENTINEL; bound];
            let cv = c_cc(
                cst,
                src.as_ptr() as *const c_char,
                cd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                1,
            );
            let rv = r_cc(
                rst,
                src.as_ptr() as *const c_char,
                rd.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                bound as c_int,
                1,
            );
            assert_eq!(cv, rv, "row 38: dictSize={} return", dsz);
            assert_bytes_eq(&format!("row 38: dictSize={}", dsz), &cd, &rd);
            assert_bytes_eq(
                &format!("row 38: dictSize={} STATE", dsz),
                std::slice::from_raw_parts(cst as *const u8, ssz),
                std::slice::from_raw_parts(rst as *const u8, ssz),
            );
            assert_eq!(c_fs(cst), r_fs(rst));
        }
    }

    // row 39: the new source OVERLAPS the registered dictionary. Achieved by
    // loading a dictionary from one buffer, then compressing from a region of
    // that SAME buffer that starts inside the dictionary range.
    let buf = gen_shape(&mut rng, 5, 200_000);
    for &(dict_off, dict_len, src_off, src_len) in &[
        (0usize, 65536usize, 30000usize, 20000usize), // src starts inside dict
        (0, 65536, 65000, 20000),                     // src straddles dict end
        (0, 65536, 65536, 20000),                     // src starts exactly at dict end
        (0, 65536, 65540, 20000),                     // src starts just past dict end
        (0, 20000, 10000, 5000),                      // small dict, deep overlap
        (0, 20000, 19999, 5000),
        (10000, 30000, 15000, 30000),
    ] {
        let bound = unsafe { cb(src_len as c_int) } as usize;
        unsafe {
            let cst = c_cs();
            let rst = r_cs();
            let dptr = buf.as_ptr().add(dict_off) as *const c_char;
            assert_eq!(
                c_ld(cst, dptr, dict_len as c_int),
                r_ld(rst, dptr, dict_len as c_int),
                "row 39: loadDict"
            );
            let sptr = buf.as_ptr().add(src_off) as *const c_char;
            let mut cd = vec![SENTINEL; bound];
            let mut rd = vec![SENTINEL; bound];
            let cv = c_cc(
                cst,
                sptr,
                cd.as_mut_ptr() as *mut c_char,
                src_len as c_int,
                bound as c_int,
                1,
            );
            let rv = r_cc(
                rst,
                sptr,
                rd.as_mut_ptr() as *mut c_char,
                src_len as c_int,
                bound as c_int,
                1,
            );
            let l = format!(
                "row 39: dict[{}..+{}] src[{}..+{}]",
                dict_off, dict_len, src_off, src_len
            );
            assert_eq!(cv, rv, "{}: return", l);
            assert_bytes_eq(&l, &cd, &rd);
            assert_bytes_eq(
                &format!("{} STATE", l),
                std::slice::from_raw_parts(cst as *const u8, ssz),
                std::slice::from_raw_parts(rst as *const u8, ssz),
            );
            assert_eq!(c_fs(cst), r_fs(rst));
        }
    }
}

// ===========================================================================
// ERRORS.md rows 42-44 — LZ4_decoderRingBufferSize
// ===========================================================================

#[test]
fn row_42_44_decoder_ring_buffer_size() {
    let (c, r) = both::<FnRingBuf>("LZ4_decoderRingBufferSize");
    unsafe {
        // row 42: maxBlockSize < 0 -> 0
        for &v in &[c_int::MIN, -1_000_000, -1] {
            let cv = c(v);
            let rv = r(v);
            assert_eq!(cv, rv, "row 42: ringBufferSize({})", v);
            assert_eq!(cv, 0, "row 42: ringBufferSize({}) must be 0", v);
        }
        // row 43: maxBlockSize > LZ4_MAX_INPUT_SIZE -> 0
        for &v in &[
            LZ4_MAX_INPUT_SIZE as c_int + 1,
            0x7FFF_FFFF,
            c_int::MAX,
        ] {
            let cv = c(v);
            let rv = r(v);
            assert_eq!(cv, rv, "row 43: ringBufferSize({})", v);
            assert_eq!(cv, 0, "row 43: ringBufferSize({}) must be 0", v);
        }
        // row 44: 0 <= maxBlockSize < 16 -> clamps to 16 -> 65566
        for v in 0..16i32 {
            let cv = c(v);
            let rv = r(v);
            assert_eq!(cv, rv, "row 44: ringBufferSize({})", v);
            assert_eq!(
                cv, 65566,
                "row 44: ringBufferSize({}) must clamp to 65566",
                v
            );
        }
        // just past the clamp
        for v in 16..24i32 {
            assert_eq!(c(v), r(v), "row 44 boundary: ringBufferSize({})", v);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 45-51 — decompress_safe input validation
// ===========================================================================

#[test]
fn row_45_51_decompress_safe_input_validation() {
    let (cds, rds) = both::<FnDecompSafe>("LZ4_decompress_safe");
    let (cdp, rdp) = both::<FnDecompPartial>("LZ4_decompress_safe_partial");
    let (cuu, ruu) = both::<FnDecompSafe>("LZ4_uncompress_unknownOutputSize");

    let src = vec![0x42u8; 1000];
    let comp = compress_c(&src);
    let mut cout = vec![SENTINEL; 4096];
    let mut rout = vec![SENTINEL; 4096];

    unsafe {
        // row 45: source == NULL -> -1
        for &cap in &[1i32, 100, 4096] {
            for &csz in &[0i32, 1, 100] {
                let cv = cds(
                    std::ptr::null(),
                    cout.as_mut_ptr() as *mut c_char,
                    csz,
                    cap,
                );
                let rv = rds(
                    std::ptr::null(),
                    rout.as_mut_ptr() as *mut c_char,
                    csz,
                    cap,
                );
                assert_eq!(cv, rv, "row 45: NULL src csz={} cap={}", csz, cap);
                assert_eq!(cv, -1, "row 45: NULL src must give -1");
            }
        }
        // same branch via the obsolete alias
        let cv = cuu(std::ptr::null(), cout.as_mut_ptr() as *mut c_char, 10, 100);
        let rv = ruu(std::ptr::null(), rout.as_mut_ptr() as *mut c_char, 10, 100);
        assert_eq!(cv, rv, "row 45: NULL src via LZ4_uncompress_unknownOutputSize");
        assert_eq!(cv, -1);

        // row 46: maxDecompressedSize < 0 -> -1
        for &cap in &[c_int::MIN, -1_000_000, -1] {
            let cv = cds(
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                cap,
            );
            let rv = rds(
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                cap,
            );
            assert_eq!(cv, rv, "row 46: negative dstCapacity={}", cap);
            assert_eq!(cv, -1, "row 46: negative dstCapacity must give -1");
        }

        // row 47: targetOutputSize < 0 or dstCapacity < 0 in the partial API
        for &t in &[c_int::MIN, -1, 0, 10] {
            for &cap in &[c_int::MIN, -1, 0, 10] {
                let cv = cdp(
                    comp.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    t,
                    cap,
                );
                let rv = rdp(
                    comp.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    comp.len() as c_int,
                    t,
                    cap,
                );
                assert_eq!(cv, rv, "row 47/50: partial target={} cap={}", t, cap);
                if t < 0 || cap < 0 {
                    assert_eq!(
                        cv, -1,
                        "row 47: partial target={} cap={} must give -1",
                        t, cap
                    );
                } else if t == 0 || cap == 0 {
                    // row 50: partialDecoding with a zero limit returns 0, never -1
                    assert_eq!(
                        cv, 0,
                        "row 50: partial target={} cap={} must give 0",
                        t, cap
                    );
                }
            }
        }

        // row 48: dstCapacity == 0 and input is NOT the 1-byte empty block -> -1
        // row 49: dstCapacity == 0 with compressedSize==1 and src[0]==0 -> 0
        let empty_block = [0u8];
        let cv = cds(
            empty_block.as_ptr() as *const c_char,
            cout.as_mut_ptr() as *mut c_char,
            1,
            0,
        );
        let rv = rds(
            empty_block.as_ptr() as *const c_char,
            rout.as_mut_ptr() as *mut c_char,
            1,
            0,
        );
        assert_eq!(cv, rv, "row 49: empty block with cap 0");
        assert_eq!(cv, 0, "row 49: must return 0");

        for (desc, bytes) in [
            ("srcSize=1 but non-zero byte", vec![1u8]),
            ("srcSize=1 byte 0xFF", vec![0xFFu8]),
            ("srcSize=2", vec![0u8, 0u8]),
            ("srcSize=3", vec![0u8, 1, 2]),
            ("full valid frame", comp.clone()),
        ] {
            let cv = cds(
                bytes.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                bytes.len() as c_int,
                0,
            );
            let rv = rds(
                bytes.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                bytes.len() as c_int,
                0,
            );
            assert_eq!(cv, rv, "row 48: cap=0 {}", desc);
            assert_eq!(cv, -1, "row 48: cap=0 {} must give -1", desc);
        }

        // row 51: compressedSize == 0 with dstCapacity > 0 -> -1
        for &cap in &[1i32, 10, 4096] {
            let cv = cds(
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                0,
                cap,
            );
            let rv = rds(
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                0,
                cap,
            );
            assert_eq!(cv, rv, "row 51: compressedSize=0 cap={}", cap);
            assert_eq!(cv, -1, "row 51: compressedSize=0 cap={} must give -1", cap);
        }
    }
}

// ===========================================================================
// ERRORS.md rows 52-72 — malformed / truncated / corrupted compressed input
//
// Rather than hand-crafting one input per branch (several branches are only
// reachable through very specific token/length/offset combinations), this
// drives BOTH libraries with a large corpus of systematically corrupted and
// truncated blocks and requires bit-exact agreement on the returned value AND
// on every byte of the destination buffer. That covers rows 52-67 (all the
// `_output_error` paths) plus 68-72 (the partial-mode clamping paths).
//
// Only the *safe* decoders are used here: row 78 documents that
// LZ4_decompress_fast performs NO input-side bounds checks at all, so feeding
// it malformed input reads past the source buffer (undefined behaviour in both
// libraries). Those are covered separately in `row_73_77_*` using valid blocks
// with undersized output.
// ===========================================================================

#[test]
fn row_52_72_corrupted_input_differential() {
    let (cds, rds) = both::<FnDecompSafe>("LZ4_decompress_safe");
    let (cdp, rdp) = both::<FnDecompPartial>("LZ4_decompress_safe_partial");
    let (cud, rud) = both::<FnDecompUsingDict>("LZ4_decompress_safe_usingDict");
    let (cpd, rpd) =
        both::<FnDecompPartialUsingDict>("LZ4_decompress_safe_partial_usingDict");

    let mut rng = Rng::new(0x5272);
    let dict = gen_shape(&mut rng, 3, 8192);

    let mut corpus: Vec<(String, Vec<u8>)> = Vec::new();

    // 1. Truncations of valid blocks at EVERY length.
    for shape in 0..N_SHAPES {
        for &len in &[13usize, 60, 200, 1000] {
            let src = gen_shape(&mut rng, shape, len);
            let comp = compress_c(&src);
            for t in 0..=comp.len() {
                corpus.push((
                    format!("truncate shape={} len={} to {}", shape_name(shape), len, t),
                    comp[..t].to_vec(),
                ));
            }
        }
    }

    // 2. Single-byte mutations of valid blocks (every position x several values).
    for shape in 0..N_SHAPES {
        let src = gen_shape(&mut rng, shape, 300);
        let comp = compress_c(&src);
        for pos in 0..comp.len() {
            for &v in &[0x00u8, 0x0F, 0xF0, 0xFF, 0x10, 0x01] {
                let mut m = comp.clone();
                m[pos] = v;
                corpus.push((format!("mutate shape={} pos={} -> {:#04x}", shape_name(shape), pos, v), m));
            }
        }
    }

    // 3. Fully random byte strings (arbitrary token/length/offset sequences,
    //    including offset == 0 which row 62 says is NOT rejected).
    for n in 0..3000usize {
        let len = rng.range(1, 80);
        corpus.push((format!("random#{} len={}", n, len), gen_random(&mut rng, len)));
    }

    // 4. Hand-built blocks aimed at specific branches: offset == 0, maximal
    //    offsets, 15/255 length extensions, and short last-literal runs.
    let handmade: Vec<(&str, Vec<u8>)> = vec![
        ("offset0: 4 lits then match offset 0", vec![0x40, 1, 2, 3, 4, 0x00, 0x00]),
        ("offset0 then last literals", vec![0x40, 1, 2, 3, 4, 0x00, 0x00, 0x50, 9, 9, 9, 9, 9]),
        ("offset0 long match", vec![0x4F, 1, 2, 3, 4, 0x00, 0x00, 0xFF, 0xFF, 0x10]),
        ("lit len 15 no ext byte", vec![0xF0]),
        ("lit len 15 ext 255 truncated", vec![0xF0, 0xFF]),
        ("lit len 15 ext 255,255 truncated", vec![0xF0, 0xFF, 0xFF]),
        ("match len 15 no ext", vec![0x0F, 0x01, 0x00]),
        ("match len ext truncated", vec![0x0F, 0x01, 0x00, 0xFF]),
        ("offset beyond output", vec![0x40, 1, 2, 3, 4, 0xFF, 0xFF]),
        ("offset max 65535", vec![0x40, 1, 2, 3, 4, 0xFF, 0xFF, 0x50, 5, 5, 5, 5, 5]),
        ("only token 0", vec![0x00]),
        ("token 0x10 (match, no lits)", vec![0x10, 0x01, 0x00]),
        ("lastlit run of 4 (< LASTLITERALS)", vec![0x40, 1, 2, 3, 4]),
        ("lastlit run of 5", vec![0x50, 1, 2, 3, 4, 5]),
        ("huge lit ext", vec![0xF0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]),
        ("huge match ext", vec![0x0F, 0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]),
    ];
    for (n, b) in handmade {
        corpus.push((n.to_string(), b));
    }

    // Destination capacities spanning zero, tiny, and generous.
    let caps: [usize; 7] = [0, 1, 2, 5, 16, 300, 4096];

    for (name, input) in &corpus {
        for &cap in &caps {
            let mut cout = vec![SENTINEL; cap + 8];
            let mut rout = vec![SENTINEL; cap + 8];
            unsafe {
                // --- LZ4_decompress_safe (rows 52-67)
                let cv = cds(
                    input.as_ptr() as *const c_char,
                    cout.as_mut_ptr() as *mut c_char,
                    input.len() as c_int,
                    cap as c_int,
                );
                let rv = rds(
                    input.as_ptr() as *const c_char,
                    rout.as_mut_ptr() as *mut c_char,
                    input.len() as c_int,
                    cap as c_int,
                );
                assert_eq!(cv, rv, "safe [{}] cap={}", name, cap);
                assert_bytes_eq(&format!("safe [{}] cap={}", name, cap), &cout, &rout);

                // --- LZ4_decompress_safe_partial (rows 68-72)
                for &t in &[0usize, 1, cap / 2, cap, cap + 1] {
                    let mut cout = vec![SENTINEL; cap + 8];
                    let mut rout = vec![SENTINEL; cap + 8];
                    let cv = cdp(
                        input.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        t as c_int,
                        cap as c_int,
                    );
                    let rv = rdp(
                        input.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        t as c_int,
                        cap as c_int,
                    );
                    assert_eq!(cv, rv, "partial [{}] cap={} target={}", name, cap, t);
                    assert_bytes_eq(
                        &format!("partial [{}] cap={} target={}", name, cap, t),
                        &cout,
                        &rout,
                    );
                }

                // --- extDict variants (rows 66-67, 70)
                for &dsz in &[0usize, 8, 8192] {
                    let dptr = if dsz == 0 {
                        std::ptr::null()
                    } else {
                        dict.as_ptr() as *const c_char
                    };
                    let mut cout = vec![SENTINEL; cap + 8];
                    let mut rout = vec![SENTINEL; cap + 8];
                    let cv = cud(
                        input.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        cap as c_int,
                        dptr,
                        dsz as c_int,
                    );
                    let rv = rud(
                        input.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        cap as c_int,
                        dptr,
                        dsz as c_int,
                    );
                    assert_eq!(cv, rv, "usingDict [{}] cap={} dict={}", name, cap, dsz);
                    assert_bytes_eq(
                        &format!("usingDict [{}] cap={} dict={}", name, cap, dsz),
                        &cout,
                        &rout,
                    );

                    let mut cout = vec![SENTINEL; cap + 8];
                    let mut rout = vec![SENTINEL; cap + 8];
                    let cv = cpd(
                        input.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        cap as c_int,
                        cap as c_int,
                        dptr,
                        dsz as c_int,
                    );
                    let rv = rpd(
                        input.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        input.len() as c_int,
                        cap as c_int,
                        cap as c_int,
                        dptr,
                        dsz as c_int,
                    );
                    assert_eq!(
                        cv, rv,
                        "partial_usingDict [{}] cap={} dict={}",
                        name, cap, dsz
                    );
                    assert_bytes_eq(
                        &format!("partial_usingDict [{}] cap={} dict={}", name, cap, dsz),
                        &cout,
                        &rout,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 73-77 — LZ4_decompress_fast output-side checks
//
// These are the ONLY validations the unsafe decoder performs. They are reached
// with a VALID compressed block plus an `originalSize` SMALLER than the true
// decompressed size, so the input is never over-read.
// ===========================================================================

#[test]
fn row_73_77_decompress_fast_output_checks() {
    let (cdf, rdf) = both::<FnDecompFast>("LZ4_decompress_fast");
    let (cun, run) = both::<FnDecompFast>("LZ4_uncompress");
    let (cfd, rfd) = both::<FnDecompFastUsingDict>("LZ4_decompress_fast_usingDict");

    let mut rng = Rng::new(0x7377);
    for shape in 0..N_SHAPES {
        for &len in &[13usize, 60, 200, 1000, 5000] {
            let src = gen_shape(&mut rng, shape, len);
            let comp = compress_c(&src);
            // Every understated originalSize from 0 to len-1 must be rejected
            // identically (rows 73/74/75/77), and `len` itself must succeed.
            for out in 0..=len {
                let mut cout = vec![SENTINEL; len + 64];
                let mut rout = vec![SENTINEL; len + 64];
                unsafe {
                    let cv = cdf(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                    );
                    let rv = rdf(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                    );
                    let l = format!(
                        "rows 73-77: shape={} len={} originalSize={}",
                        shape_name(shape),
                        len,
                        out
                    );
                    assert_eq!(cv, rv, "{}: return", l);
                    assert_bytes_eq(&l, &cout, &rout);
                    // The differential equality above IS the test. We do NOT
                    // assert our own guess about WHICH understated sizes are
                    // rejected: `LZ4_decompress_unsafe_generic` only checks the
                    // OUTPUT bound, so a short `originalSize` that happens to
                    // land exactly at the end of a literal run succeeds and
                    // returns the bytes consumed (e.g. len=13 constant data,
                    // originalSize=1 -> 2). Only the exact-size case has a
                    // value we can legitimately predict.
                    if out == len {
                        assert_eq!(cv, comp.len() as c_int, "{}: exact size must succeed", l);
                    } else {
                        assert!(
                            cv == -1 || cv > 0,
                            "{}: expected -1 or a positive consumed count, got {}",
                            l,
                            cv
                        );
                    }
                }
            }
            // Same branch through the obsolete alias and the usingDict variant.
            for &out in &[0usize, len / 2, len] {
                let mut cout = vec![SENTINEL; len + 64];
                let mut rout = vec![SENTINEL; len + 64];
                unsafe {
                    let cv = cun(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                    );
                    let rv = run(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                    );
                    assert_eq!(cv, rv, "row 73: LZ4_uncompress out={}", out);
                    assert_bytes_eq(&format!("row 73: LZ4_uncompress out={}", out), &cout, &rout);

                    let mut cout = vec![SENTINEL; len + 64];
                    let mut rout = vec![SENTINEL; len + 64];
                    let cv = cfd(
                        comp.as_ptr() as *const c_char,
                        cout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                        std::ptr::null(),
                        0,
                    );
                    let rv = rfd(
                        comp.as_ptr() as *const c_char,
                        rout.as_mut_ptr() as *mut c_char,
                        out as c_int,
                        std::ptr::null(),
                        0,
                    );
                    assert_eq!(cv, rv, "row 76: fast_usingDict out={}", out);
                    assert_bytes_eq(
                        &format!("row 76: fast_usingDict out={}", out),
                        &cout,
                        &rout,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md rows 79-80 — *_continue propagates the underlying failure
// ===========================================================================

#[test]
fn row_79_80_decompress_continue_error_propagation() {
    let (c_cd, r_cd) = both::<FnCreateStreamDecode>("LZ4_createStreamDecode");
    let (c_fd, r_fd) = both::<FnFreeStreamDecode>("LZ4_freeStreamDecode");
    let (c_sd, r_sd) = both::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let (c_dc, r_dc) = both::<FnDecompSafeContinue>("LZ4_decompress_safe_continue");
    let (c_df, r_df) = both::<FnDecompFastContinue>("LZ4_decompress_fast_continue");

    let mut rng = Rng::new(0x7980);
    let src = gen_shape(&mut rng, 4, 2000);
    let comp = compress_c(&src);

    unsafe {
        // row 79: safe_continue with a too-small dst, and with corrupted input.
        for &cap in &[0usize, 1, 5, 100, 1999, 2000] {
            let cds = c_cd();
            let rds = r_cd();
            assert_eq!(
                c_sd(cds, std::ptr::null(), 0),
                r_sd(rds, std::ptr::null(), 0),
                "row 83: setStreamDecode(NULL,0)"
            );
            let mut cout = vec![SENTINEL; cap + 16];
            let mut rout = vec![SENTINEL; cap + 16];
            let cv = c_dc(
                cds,
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                cap as c_int,
            );
            let rv = r_dc(
                rds,
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                cap as c_int,
            );
            assert_eq!(cv, rv, "row 79: safe_continue cap={}", cap);
            assert_eq!(c_fd(cds), r_fd(rds));
        }

        // corrupted / truncated input through safe_continue
        for t in 0..comp.len() {
            let cds = c_cd();
            let rds = r_cd();
            c_sd(cds, std::ptr::null(), 0);
            r_sd(rds, std::ptr::null(), 0);
            let mut cout = vec![SENTINEL; 2048];
            let mut rout = vec![SENTINEL; 2048];
            let cv = c_dc(
                cds,
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                t as c_int,
                2048,
            );
            let rv = r_dc(
                rds,
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                t as c_int,
                2048,
            );
            assert_eq!(cv, rv, "row 79: safe_continue truncated to {}", t);
            assert_bytes_eq(
                &format!("row 79: safe_continue truncated to {}", t),
                &cout,
                &rout,
            );
            assert_eq!(c_fd(cds), r_fd(rds));
        }

        // row 80: fast_continue with an understated originalSize
        for &out in &[0usize, 1, 100, 1999, 2000] {
            let cds = c_cd();
            let rds = r_cd();
            c_sd(cds, std::ptr::null(), 0);
            r_sd(rds, std::ptr::null(), 0);
            let mut cout = vec![SENTINEL; 2100];
            let mut rout = vec![SENTINEL; 2100];
            let cv = c_df(
                cds,
                comp.as_ptr() as *const c_char,
                cout.as_mut_ptr() as *mut c_char,
                out as c_int,
            );
            let rv = r_df(
                rds,
                comp.as_ptr() as *const c_char,
                rout.as_mut_ptr() as *mut c_char,
                out as c_int,
            );
            assert_eq!(cv, rv, "row 80: fast_continue originalSize={}", out);
            assert_bytes_eq(
                &format!("row 80: fast_continue originalSize={}", out),
                &cout,
                &rout,
            );
            assert_eq!(c_fd(cds), r_fd(rds));
        }
    }
}

// ===========================================================================
// ERRORS.md row 83 — LZ4_setStreamDecode has no failure return
// ===========================================================================

#[test]
fn row_83_set_stream_decode_always_succeeds() {
    let (c_cd, r_cd) = both::<FnCreateStreamDecode>("LZ4_createStreamDecode");
    let (c_fd, r_fd) = both::<FnFreeStreamDecode>("LZ4_freeStreamDecode");
    let (c_sd, r_sd) = both::<FnSetStreamDecode>("LZ4_setStreamDecode");
    let dict = vec![0x11u8; 4096];

    unsafe {
        // NULL dictionary with size 0 is the documented reset form. Non-zero
        // size with a NULL pointer is `assert`-only in C (row 83) and would be
        // UB, so it is NOT exercised here — only the well-defined shapes are.
        for &(has_dict, dsz) in &[
            (false, 0i32),
            (true, 0),
            (true, 1),
            (true, 8),
            (true, 4096),
            (true, -1), // negative size: no check in C, only reads the pointer
        ] {
            let cds = c_cd();
            let rds = r_cd();
            let p = if has_dict {
                dict.as_ptr() as *const c_char
            } else {
                std::ptr::null()
            };
            let cv = c_sd(cds, p, dsz);
            let rv = r_sd(rds, p, dsz);
            assert_eq!(cv, rv, "row 83: setStreamDecode(has_dict={}, {})", has_dict, dsz);
            assert_eq!(cv, 1, "row 83: setStreamDecode always returns 1");
            assert_eq!(c_fd(cds), r_fd(rds));
        }
    }
}

// ===========================================================================
// ERRORS.md row 88 — LZ4_resetStreamState always returns 0
// ===========================================================================

#[test]
fn row_88_reset_stream_state_always_zero() {
    let (c_cs, r_cs) = both::<FnCreateStream>("LZ4_createStream");
    let (c_fs, r_fs) = both::<FnFreeStream>("LZ4_freeStream");
    let (c_rs, r_rs) = both::<FnResetStreamState>("LZ4_resetStreamState");
    let ssz = stream_size();
    unsafe {
        let cst = c_cs();
        let rst = r_cs();
        let mut inbuf = vec![0u8; 64];
        for pass_buf in [false, true] {
            let p = if pass_buf {
                inbuf.as_mut_ptr() as *mut c_char
            } else {
                std::ptr::null_mut()
            };
            let cv = c_rs(cst, p);
            let rv = r_rs(rst, p);
            assert_eq!(cv, rv, "row 88: resetStreamState(buf={})", pass_buf);
            assert_eq!(cv, 0, "row 88: must always return 0");
            assert_bytes_eq(
                &format!("row 88: STATE (buf={})", pass_buf),
                std::slice::from_raw_parts(cst as *const u8, ssz),
                std::slice::from_raw_parts(rst as *const u8, ssz),
            );
        }
        assert_eq!(c_fs(cst), r_fs(rst));
    }
}

// ===========================================================================
// Rows deliberately NOT tested, with the reason:
//
// 13, 14  — the `notLimited` behavioural split. Exercising the documented
//           consequence requires LYING about dstCapacity, which by definition
//           writes out of bounds in BOTH libraries. The reachable, well-defined
//           half (that an honestly-sized buffer returns > 0) is covered by
//           `tests/lz4_block.rs`.
// 21, 22  — `LZ4_HEAPMODE=1` allocation failure. The C is compiled with
//           `LZ4_HEAPMODE=0` (see c_src/CMakeLists.txt), so this code is not
//           even present in the library under test.
// 24, 25  — NULL/misaligned `state` for `LZ4_compress_fast_extState` and
//           `LZ4_compress_destSize_extState`. The C documents these as
//           `assert`-only, i.e. UB in a release build; calling them would
//           segfault both libraries and prove nothing.
// 29, 87  — `malloc` failure inside `LZ4_createStream` /
//           `LZ4_createStreamDecode`. Not forceable without an allocator hook,
//           which these APIs do not provide.
// 37      — `LZ4_saveDict(stream, NULL, nonzero)`: `assert`-only, UB.
// 40, 41  — `assert`-only paths whose release behaviour is exactly the
//           `return 0` already asserted by rows 3/5.
// 56      — 32-bit-only overflow branch (`sizeof(size_t) < 8`); unreachable on
//           this x86-64 build.
// 57-59   — address-space wrap of `op + length` / `ip + length`. Requires a
//           length near 2^64, which the preceding `ilimit` check rejects first
//           on any buffer that can actually be allocated.
// 78      — `LZ4_decompress_fast` has no input-side bound at all; feeding it
//           malformed input over-reads the source in BOTH libraries. Its
//           output-side checks are covered by `row_73_77_*`.
// 81, 82  — NULL `LZ4_streamDecode` / negative `originalSize` for
//           `LZ4_decompress_fast_continue`: `assert`-only, UB.
// 84, 85  — negative `dictSize`: `assert`-only, cast to a huge `size_t`, UB.
// 89-93   — compile-time `#error` / `LZ4_STATIC_ASSERT` guards. Both libraries
//           compiled successfully, which is itself the proof these hold.
// ===========================================================================
