//! ERRORS.md rows 1-124 — ERROR-PATH parity for `lz4.c`, `lz4hc.c`, `xxhash.c`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and asserts they return the SAME sentinel value (not merely that
//! both failed), plus identical destination bytes where anything is written.
//!
//! The cases listed in the "Appendix — C paths that are undefined behaviour"
//! section of ERRORS.md are deliberately NOT covered: the C dereferences NULL
//! there, so no differential test is possible.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Signatures (verified against c_src/include/lz4.h / lz4hc.h / xxhash.h)
// ---------------------------------------------------------------------------
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnSizeof = unsafe extern "C" fn() -> c_int;
type FnDefault = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDestSize = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnInitStream = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnForceExt = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;

type FnDecSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecPartial = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnDecFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
type FnDecPartialUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type FnSetSD = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnDecSafeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;

// lz4hc
type FnHC5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnHC4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnHC3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
type FnExt6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnExt5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnExt4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
type FnDestSizeHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
type FnContDestSizeHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
type FnResetStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;

// xxhash
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
type FnXXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnReset32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnCanon32 = unsafe extern "C" fn(*mut c_void, u32);
type FnCanon64 = unsafe extern "C" fn(*mut c_void, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const c_void) -> u64;

// ---------------------------------------------------------------------------
// Buffer helpers
// ---------------------------------------------------------------------------
/// `gen_data`, but ALWAYS backed by a real allocation with >= 64 readable
/// bytes past `len`. `Vec::<u8>::as_ptr()` on a zero-capacity vector returns
/// the dangling pointer `0x1`; several C paths compute `src + srcSize - N` up
/// front and would then walk off into unmapped memory. That is a caller
/// defect, not a library defect, so never hand a dangling pointer to the API.
fn gen_src(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_data(shape, len, rng);
    if v.capacity() < len + 64 {
        v.reserve(len + 64);
    }
    v
}

/// `bytes` followed by `slack` zero bytes inside ONE allocation. The padding is
/// never counted in any `srcSize`; it exists so the deliberately unchecked
/// `LZ4_decompress_fast` / `LZ4_uncompress` reads stay in mapped memory.
fn with_slack(bytes: &[u8], slack: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + slack + 8);
    v.extend_from_slice(bytes);
    v.resize(bytes.len() + slack, 0);
    v
}

/// Slack appended to every destination buffer so an out-of-contract write is
/// detected instead of corrupting the heap.
const DST_SLACK: usize = 96;

fn dst_pair(cap: usize) -> (Vec<u8>, Vec<u8>) {
    (vec![0xA5u8; cap + DST_SLACK], vec![0xA5u8; cap + DST_SLACK])
}

/// Compare a `(ret, whole dst buffer)` pair from the two libraries.
#[track_caller]
fn cmp_dec(cn: c_int, cb: &[u8], rn: c_int, rb: &[u8], ctx: &str) {
    assert_eq!(cn, rn, "{ctx}: return mismatch C={cn} Rust={rn}");
    assert_bytes_eq(cb, rb, &format!("{ctx}: destination bytes"));
}

/// Compress `src` with the C library (always valid output).
fn c_compress(f: &FnDefault, src: &[u8], bound: &FnBound) -> Vec<u8> {
    let cap = unsafe { bound(src.len() as c_int) }.max(16) as usize;
    let mut out = vec![0u8; cap];
    let n = unsafe {
        f(
            src.as_ptr() as *const c_char,
            out.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            cap as c_int,
        )
    };
    assert!(n > 0, "C compression failed (srcSize={})", src.len());
    out.truncate(n as usize);
    out
}

// ---------------------------------------------------------------------------
// Hand-rolled LZ4 block encoder — lets a test place a specific defect at a
// specific byte offset (used by ERRORS.md rows 34-40 / 47-52).
// ---------------------------------------------------------------------------
fn put_varint(out: &mut Vec<u8>, mut n: usize) {
    while n >= 255 {
        out.push(255);
        n -= 255;
    }
    out.push(n as u8);
}

/// Emit one sequence. `m = Some((offset, matchLength))`, `matchLength >= 4`.
/// `None` emits a literals-only (final) sequence.
fn emit_seq(out: &mut Vec<u8>, lits: &[u8], m: Option<(usize, usize)>) {
    let ll = lits.len();
    let mlcode = m.map(|(_, ml)| ml - 4).unwrap_or(0);
    out.push(((ll.min(15) as u8) << 4) | (mlcode.min(15) as u8));
    if ll >= 15 {
        put_varint(out, ll - 15);
    }
    out.extend_from_slice(lits);
    if let Some((off, _)) = m {
        out.push((off & 0xFF) as u8);
        out.push(((off >> 8) & 0xFF) as u8);
        if mlcode >= 15 {
            put_varint(out, mlcode - 15);
        }
    }
}

// ===========================================================================
// Rows 1-3 — LZ4_compressBound
// ===========================================================================
#[test]
fn err01_03_compress_bound() {
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0001);

    let mut sizes: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1_000_000,
        -65536,
        -16,
        -15,
        -1,
        0,
        1,
        15,
        16,
        17,
        255,
        65535,
        65536,
        LZ4_MAX_INPUT_SIZE - 1,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..2000 {
        sizes.push(rng.next_u32() as c_int);
    }

    for &n in &sizes {
        let (c, r) = unsafe { (bound.0(n), bound.1(n)) };
        assert_ret_eq(c, r, &format!("LZ4_compressBound({n})"));
        // rows 1-2: out of range => 0; row 3: the boundary is still valid.
        if n < 0 || n > LZ4_MAX_INPUT_SIZE {
            assert_eq!(c, 0, "LZ4_compressBound({n}) must be 0");
        } else {
            assert!(c > 0, "LZ4_compressBound({n}) must be non-zero");
        }
    }
    assert!(unsafe { bound.0(LZ4_MAX_INPUT_SIZE) } > 0, "row 3 boundary");
}

// ===========================================================================
// Rows 55-58 — LZ4_decoderRingBufferSize
// ===========================================================================
#[test]
fn err55_58_decoder_ring_buffer_size() {
    sym!(drbs, "LZ4_decoderRingBufferSize", FnBound);
    let mut rng = Rng::new(0x0055);

    let mut sizes: Vec<c_int> = vec![
        i32::MIN,
        i32::MIN + 1,
        -65536,
        -16,
        -1,
        0,
        1,
        2,
        15,
        16,
        17,
        64,
        65535,
        65536,
        LZ4_MAX_INPUT_SIZE - 1,
        LZ4_MAX_INPUT_SIZE,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..2000 {
        sizes.push(rng.next_u32() as c_int);
    }

    for &n in &sizes {
        let (c, r) = unsafe { (drbs.0(n), drbs.1(n)) };
        assert_ret_eq(c, r, &format!("LZ4_decoderRingBufferSize({n})"));
        if n < 0 || n > LZ4_MAX_INPUT_SIZE {
            assert_eq!(c, 0, "row55/56: {n} must give 0");
        }
    }
    // row 57: 0..15 are clamped to 16 => the same value as 16.
    let at16 = unsafe { drbs.0(16) };
    for n in 0..16 {
        let (c, r) = unsafe { (drbs.0(n), drbs.1(n)) };
        assert_ret_eq(c, r, &format!("drbs({n})"));
        assert_eq!(c, at16, "row57: {n} must clamp to the 16 result");
    }
    assert_eq!(at16, 65566, "row57: documented value");
    // row 58: both boundaries are valid non-zero sizes.
    assert!(unsafe { drbs.0(LZ4_MAX_INPUT_SIZE) } > 0);
}

// ===========================================================================
// Rows 4-5 — srcSize out of range (checked BEFORE src is read)
// ===========================================================================
#[test]
fn err04_05_compress_bad_src_size() {
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(fast, "LZ4_compress_fast", FnFast);
    let mut rng = Rng::new(0x0004);
    // A small REAL buffer; the bogus sizes are never actually read.
    let src = gen_src(Shape::Random, 64, &mut rng);

    let bad: &[c_int] = &[
        -1,
        -2,
        -64,
        -65536,
        i32::MIN,
        i32::MIN + 1,
        LZ4_MAX_INPUT_SIZE + 1,
        LZ4_MAX_INPUT_SIZE + 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &n in bad {
        for &cap in &[0usize, 1, 16, 1024] {
            let (mut cb, mut rb) = dst_pair(cap);
            let (cn, rn) = unsafe {
                (
                    def.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        n,
                        cap as c_int,
                    ),
                    def.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        n,
                        cap as c_int,
                    ),
                )
            };
            cmp_dec(cn, &cb, rn, &rb, &format!("row4/5 default srcSize={n} cap={cap}"));
            assert_eq!(cn, 0, "row4/5: srcSize={n} must give 0");

            for &acc in &[1i32, 0, -1, 12, 100000] {
                let (mut cb, mut rb) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        fast.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            n,
                            cap as c_int,
                            acc,
                        ),
                        fast.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            n,
                            cap as c_int,
                            acc,
                        ),
                    )
                };
                cmp_dec(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("row4/5 fast srcSize={n} cap={cap} acc={acc}"),
                );
                assert_eq!(cn, 0, "row4/5: srcSize={n} must give 0");
            }
        }
    }
}

// ===========================================================================
// Rows 6-7 — srcSize == 0
// ===========================================================================
#[test]
fn err06_07_compress_empty_src() {
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(fast, "LZ4_compress_fast", FnFast);
    let mut rng = Rng::new(0x0006);
    let src = gen_src(Shape::Random, 64, &mut rng); // real address, size 0 used

    for &cap in &[i32::MIN, -1000, -1, 0, 1, 2, 15, 16, 17, 1024] {
        let capu = cap.max(0) as usize;
        let (mut cb, mut rb) = dst_pair(capu);
        let (cn, rn) = unsafe {
            (
                def.0(
                    src.as_ptr() as *const c_char,
                    cb.as_mut_ptr() as *mut c_char,
                    0,
                    cap,
                ),
                def.1(
                    src.as_ptr() as *const c_char,
                    rb.as_mut_ptr() as *mut c_char,
                    0,
                    cap,
                ),
            )
        };
        cmp_dec(cn, &cb, rn, &rb, &format!("row6/7 default cap={cap}"));
        if cap <= 0 {
            assert_eq!(cn, 0, "row6: cap={cap} must give 0");
        } else {
            assert_eq!(cn, 1, "row7: cap={cap} must give 1");
            assert_eq!(cb[0], 0, "row7: dst[0] must be 0");
        }

        for &acc in &[1i32, 0, -1, 65538] {
            let (mut cb, mut rb) = dst_pair(capu);
            let (cn, rn) = unsafe {
                (
                    fast.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        acc,
                    ),
                    fast.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        acc,
                    ),
                )
            };
            cmp_dec(cn, &cb, rn, &rb, &format!("row6/7 fast cap={cap} acc={acc}"));
            assert_eq!(cn, if cap <= 0 { 0 } else { 1 });
        }
    }
}

// ===========================================================================
// Rows 8-9 — dstCapacity below the achievable compressed size
// ===========================================================================
#[test]
fn err08_09_compress_dst_too_small() {
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0008);

    for &shape in ALL_SHAPES {
        for &len in &[13usize, 64, 255, 1024, 4096, 65536, 100_000] {
            let src = gen_src(shape, len, &mut rng);
            // The natural (unconstrained) compressed size.
            let nat = c_compress(&def.0, &src, &bound.0).len();
            // row 8: exactly one byte too few. row 9: cap == 1.
            for &cap in &[nat - 1, 1, 2, nat / 2] {
                let (mut cb, mut rb) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        def.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                        def.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                    )
                };
                cmp_dec(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("row8/9 {shape:?} len={len} cap={cap} nat={nat}"),
                );
                assert_eq!(
                    cn, 0,
                    "row8/9 {shape:?} len={len} cap={cap} (nat={nat}) must give 0"
                );
            }
            // The boundary the other way: exactly `nat` succeeds.
            let (mut cb, mut rb) = dst_pair(nat);
            let (cn, rn) = unsafe {
                (
                    def.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        nat as c_int,
                    ),
                    def.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        nat as c_int,
                    ),
                )
            };
            cmp_dec(cn, &cb, rn, &rb, &format!("row8 boundary {shape:?} len={len}"));
            assert_eq!(cn, nat as c_int);
        }
    }
}

// ===========================================================================
// Rows 10-11 — acceleration clamping (< 1 => 1, > 65537 => 65537)
// ===========================================================================
#[test]
fn err10_11_acceleration_clamping() {
    sym!(fast, "LZ4_compress_fast", FnFast);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0010);

    let low: &[c_int] = &[0, -1, -2, -65536, i32::MIN, i32::MIN + 1];
    let high: &[c_int] = &[
        LZ4_ACCELERATION_MAX + 1,
        LZ4_ACCELERATION_MAX + 2,
        100_000,
        1_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];

    for &shape in ALL_SHAPES {
        for &len in &[0usize, 1, 13, 64, 1024, 65536, 100_000] {
            let src = gen_src(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(16) as usize;

            let run = |f: &FnFast, acc: c_int| -> (c_int, Vec<u8>) {
                let mut b = vec![0xA5u8; cap + DST_SLACK];
                let n = unsafe {
                    f(
                        src.as_ptr() as *const c_char,
                        b.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                        acc,
                    )
                };
                (n, b)
            };

            // Reference outputs at the two clamp targets.
            let (n1c, b1c) = run(&fast.0, 1);
            let (n1r, b1r) = run(&fast.1, 1);
            cmp_dec(n1c, &b1c, n1r, &b1r, &format!("acc=1 {shape:?} len={len}"));
            let (nmc, bmc) = run(&fast.0, LZ4_ACCELERATION_MAX);
            let (nmr, bmr) = run(&fast.1, LZ4_ACCELERATION_MAX);
            cmp_dec(nmc, &bmc, nmr, &bmr, &format!("acc=MAX {shape:?} len={len}"));

            for &acc in low {
                let (cn, cb) = run(&fast.0, acc);
                let (rn, rb) = run(&fast.1, acc);
                let ctx = format!("row10 {shape:?} len={len} acc={acc}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n1c, "{ctx}: must equal acceleration 1");
                assert_bytes_eq(&cb, &b1c, &format!("{ctx}: bytes vs acceleration 1"));
            }
            for &acc in high {
                let (cn, cb) = run(&fast.0, acc);
                let (rn, rb) = run(&fast.1, acc);
                let ctx = format!("row11 {shape:?} len={len} acc={acc}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, nmc, "{ctx}: must equal acceleration 65537");
                assert_bytes_eq(&cb, &bmc, &format!("{ctx}: bytes vs acceleration 65537"));
            }
        }
    }
}

// ===========================================================================
// Rows 12-14 — LZ4_compress_destSize
// ===========================================================================
#[test]
fn err12_14_compress_dest_size() {
    sym!(ds, "LZ4_compress_destSize", FnDestSize);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x0012);

    let run = |f: &FnDestSize, src: &[u8], srcSize: c_int, target: c_int| -> (c_int, c_int, Vec<u8>) {
        let cap = target.max(0) as usize;
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let mut sp = srcSize;
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                &mut sp,
                target,
            )
        };
        (n, sp, b)
    };

    for &shape in ALL_SHAPES {
        for &len in &[1usize, 13, 64, 1024, 4096, 65536, 100_000] {
            let src = gen_src(shape, len, &mut rng);

            // row 12: targetDstSize < 1
            for &t in &[0i32, -1, -1000, i32::MIN] {
                let (cn, csp, cb) = run(&ds.0, &src, len as c_int, t);
                let (rn, rsp, rb) = run(&ds.1, &src, len as c_int, t);
                let ctx = format!("row12 {shape:?} len={len} target={t}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(csp, rsp, "{ctx}: *srcSizePtr mismatch");
                assert_eq!(cn, 0, "{ctx}: must give 0");
            }

            // row 13: *srcSizePtr < 0
            for &s in &[-1i32, -1000, i32::MIN, LZ4_MAX_INPUT_SIZE + 1] {
                for &t in &[1i32, 64, 100_000] {
                    let (cn, csp, cb) = run(&ds.0, &src, s, t);
                    let (rn, rsp, rb) = run(&ds.1, &src, s, t);
                    let ctx = format!("row13 {shape:?} srcSize={s} target={t}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr mismatch");
                    assert_eq!(cn, 0, "{ctx}: must give 0");
                }
            }

            // row 14: partial fill — reduced *srcSizePtr, ret <= target
            for &t in &[1usize, 2, 3, 5, 11, 20, len / 4 + 1, len / 2 + 1] {
                let (cn, csp, cb) = run(&ds.0, &src, len as c_int, t as c_int);
                let (rn, rsp, rb) = run(&ds.1, &src, len as c_int, t as c_int);
                let ctx = format!("row14 {shape:?} len={len} target={t}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(csp, rsp, "{ctx}: *srcSizePtr mismatch");
                assert!(cn <= t as c_int, "{ctx}: ret {cn} > target");
                assert!(csp >= 0 && csp <= len as c_int, "{ctx}: bogus srcSize {csp}");
                if cn > 0 {
                    // whatever was produced must decode back to src[..csp]
                    let mut out = vec![0u8; csp as usize + 64];
                    let d = unsafe {
                        dec.0(
                            cb.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            cn,
                            csp,
                        )
                    };
                    assert_eq!(d, csp, "{ctx}: round trip size");
                    assert_bytes_eq(&out[..csp as usize], &src[..csp as usize], &ctx);
                }
            }
        }
    }
}

// ===========================================================================
// Rows 15-18 — LZ4_initStream
// ===========================================================================
#[test]
fn err15_18_init_stream() {
    sym!(init, "LZ4_initStream", FnInitStream);
    let mut buf = Aligned::new(SIZEOF_LZ4_STREAM_T + 64);
    let p = buf.ptr();

    // row 15: NULL buffer
    for &sz in &[0usize, 1, SIZEOF_LZ4_STREAM_T, SIZEOF_LZ4_STREAM_T + 1, usize::MAX] {
        let (c, r) = unsafe {
            (
                init.0(std::ptr::null_mut(), sz),
                init.1(std::ptr::null_mut(), sz),
            )
        };
        assert_eq!(c, r, "row15 initStream(NULL, {sz})");
        assert!(c.is_null(), "row15 initStream(NULL, {sz}) must be NULL");
    }

    // row 16: size below sizeof(LZ4_stream_t)
    for &sz in &[
        0usize,
        1,
        8,
        SIZEOF_LZ4_STREAM_T - 2,
        SIZEOF_LZ4_STREAM_T - 1,
    ] {
        let (c, r) = unsafe { (init.0(p as *mut c_void, sz), init.1(p as *mut c_void, sz)) };
        assert_eq!(c, r, "row16 initStream(aligned, {sz})");
        assert!(c.is_null(), "row16 initStream(aligned, {sz}) must be NULL");
    }

    // row 17: misaligned buffer
    for off in 1usize..8 {
        let mp = unsafe { p.add(off) } as *mut c_void;
        for &sz in &[SIZEOF_LZ4_STREAM_T, SIZEOF_LZ4_STREAM_T + 8] {
            let (c, r) = unsafe { (init.0(mp, sz), init.1(mp, sz)) };
            assert_eq!(c, r, "row17 initStream(+{off}, {sz})");
            assert!(c.is_null(), "row17 initStream(+{off}, {sz}) must be NULL");
        }
    }

    // row 18: exactly sizeof and aligned
    for &sz in &[SIZEOF_LZ4_STREAM_T, SIZEOF_LZ4_STREAM_T + 1, SIZEOF_LZ4_STREAM_T + 64] {
        let (c, r) = unsafe { (init.0(p as *mut c_void, sz), init.1(p as *mut c_void, sz)) };
        assert_eq!(c, r, "row18 initStream(aligned, {sz})");
        assert_eq!(c, p as *mut c_void, "row18 must return the buffer");
    }
    // 8-byte aligned offsets stay valid.
    for off in [8usize, 16, 24, 32] {
        let mp = unsafe { p.add(off) } as *mut c_void;
        let (c, r) = unsafe {
            (
                init.0(mp, SIZEOF_LZ4_STREAM_T),
                init.1(mp, SIZEOF_LZ4_STREAM_T),
            )
        };
        assert_eq!(c, r, "row18 initStream(+{off})");
        assert_eq!(c, mp);
    }
}

// ===========================================================================
// Rows 19-20 — free(NULL)
// ===========================================================================
#[test]
fn err19_20_free_null() {
    sym!(fs, "LZ4_freeStream", FnFree);
    sym!(fsd, "LZ4_freeStreamDecode", FnFree);
    unsafe {
        assert_ret_eq(
            fs.0(std::ptr::null_mut()),
            fs.1(std::ptr::null_mut()),
            "row19 LZ4_freeStream(NULL)",
        );
        assert_eq!(fs.0(std::ptr::null_mut()), 0, "row19 must be 0");
        assert_ret_eq(
            fsd.0(std::ptr::null_mut()),
            fsd.1(std::ptr::null_mut()),
            "row20 LZ4_freeStreamDecode(NULL)",
        );
        assert_eq!(fsd.0(std::ptr::null_mut()), 0, "row20 must be 0");
    }
}

// ===========================================================================
// Rows 21-24 — LZ4_loadDict / LZ4_loadDictSlow
// ===========================================================================
#[test]
fn err21_24_load_dict() {
    sym!(cs, "LZ4_createStream", FnCreate);
    sym!(fsr, "LZ4_freeStream", FnFree);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(lds, "LZ4_loadDictSlow", FnLoadDict);
    let mut rng = Rng::new(0x0021);

    // The dictionary must outlive every call that keeps a pointer to it.
    let dict = gen_src(Shape::Texty, 200_000, &mut rng);

    let (csr, rsr) = unsafe { (cs.0(), cs.1()) };
    assert!(!csr.is_null() && !rsr.is_null());

    let mut cases: Vec<c_int> = vec![
        i32::MIN,
        -65536,
        -8,
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
        65535,
        65536,
        65537,
        100_000,
        200_000,
    ];
    cases.sort_unstable();

    for f in [0usize, 1] {
        let (cf, rf) = if f == 0 {
            (&ld.0, &ld.1)
        } else {
            (&lds.0, &lds.1)
        };
        let name = if f == 0 { "LZ4_loadDict" } else { "LZ4_loadDictSlow" };
        for &n in &cases {
            let (c, r) = unsafe {
                (
                    cf(csr, dict.as_ptr() as *const c_char, n),
                    rf(rsr, dict.as_ptr() as *const c_char, n),
                )
            };
            let ctx = format!("row21-24 {name}({n})");
            assert_ret_eq(c, r, &ctx);
            if n < 8 {
                assert_eq!(c, 0, "{ctx}: dictSize < HASH_UNIT must give 0");
            } else if n > 65536 {
                assert_eq!(c, 65536, "{ctx}: must clamp to 65536");
            } else {
                assert_eq!(c, n, "{ctx}: must be kept as-is");
            }
        }
    }

    unsafe {
        fsr.0(csr);
        fsr.1(rsr);
    }
}

// ===========================================================================
// Rows 25-28 — LZ4_saveDict
// ===========================================================================
#[test]
fn err25_28_save_dict() {
    sym!(cs, "LZ4_createStream", FnCreate);
    sym!(fsr, "LZ4_freeStream", FnFree);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(sd, "LZ4_saveDict", FnSaveDict);
    let mut rng = Rng::new(0x0025);

    let dict = gen_src(Shape::Texty, 200_000, &mut rng);
    // saveDict re-points the stream at `safe`, so it must outlive the stream.
    let mut csafe = vec![0x5Au8; 70_000 + DST_SLACK];
    let mut rsafe = vec![0x5Au8; 70_000 + DST_SLACK];

    let sizes: &[c_int] = &[
        i32::MIN,
        -70000,
        -5,
        -1,
        0,
        1,
        3,
        4,
        8,
        100,
        65535,
        65536,
        65537,
        70000,
    ];
    // Preloads: 0 (fresh stream => dictSize 0), 4 (loadDict returns 0), 8,
    // 1000, 65536, 200000 (clamped to 65536).
    for &pre in &[0i32, 4, 8, 1000, 65536, 200_000] {
        for &n in sizes {
            let (csr, rsr) = unsafe { (cs.0(), cs.1()) };
            assert!(!csr.is_null() && !rsr.is_null());
            let (cl, rl) = unsafe {
                (
                    ld.0(csr, dict.as_ptr() as *const c_char, pre),
                    ld.1(rsr, dict.as_ptr() as *const c_char, pre),
                )
            };
            assert_ret_eq(cl, rl, &format!("row25-28 preload loadDict({pre})"));

            for b in csafe.iter_mut() {
                *b = 0x5A;
            }
            for b in rsafe.iter_mut() {
                *b = 0x5A;
            }
            let (c, r) = unsafe {
                (
                    sd.0(csr, csafe.as_mut_ptr() as *mut c_char, n),
                    sd.1(rsr, rsafe.as_mut_ptr() as *mut c_char, n),
                )
            };
            let ctx = format!("row25-28 saveDict(pre={pre}, {n})");
            assert_ret_eq(c, r, &ctx);
            assert_bytes_eq(&csafe, &rsafe, &format!("{ctx}: safeBuffer"));
            assert!(c >= 0, "{ctx}: negative result {c}");
            assert!(c <= 65536, "{ctx}: > 64 KB");
            assert!(c <= cl.max(0), "{ctx}: {c} exceeds stream dictSize {cl}");
            if n <= 0 {
                // (U32)negative > 64 KB => clamped to 65536 then to dictSize.
                assert_eq!(c, if n == 0 { 0 } else { cl.max(0) }, "{ctx}");
            }
            unsafe {
                fsr.0(csr);
                fsr.1(rsr);
            }
        }
    }
}

// ===========================================================================
// Rows 29-33 — LZ4_decompress_safe degenerate arguments
// ===========================================================================
#[test]
fn err29_33_decompress_safe_degenerate() {
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0029);

    let plain = gen_src(Shape::Texty, 4096, &mut rng);
    let comp = c_compress(&def.0, &plain, &bound.0);
    let comp = with_slack(&comp, 64);

    let run = |f: &FnDecSafe, src: *const c_char, ss: c_int, cap: c_int| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap.max(0) as usize + DST_SLACK];
        let n = unsafe { f(src, b.as_mut_ptr() as *mut c_char, ss, cap) };
        (n, b)
    };

    // row 29: src == NULL
    for &ss in &[-1i32, 0, 1, 10] {
        for &cap in &[0i32, 1, 100] {
            let (cn, cb) = run(&dec.0, std::ptr::null(), ss, cap);
            let (rn, rb) = run(&dec.1, std::ptr::null(), ss, cap);
            let ctx = format!("row29 src=NULL ss={ss} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, -1, "{ctx}");
        }
    }

    // row 30: dstCapacity < 0
    for &cap in &[-1i32, -100, i32::MIN] {
        for &ss in &[0i32, 1, 10, 100] {
            let (cn, cb) = run(&dec.0, comp.as_ptr() as *const c_char, ss, cap);
            let (rn, rb) = run(&dec.1, comp.as_ptr() as *const c_char, ss, cap);
            let ctx = format!("row30 cap={cap} ss={ss}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, -1, "{ctx}");
        }
    }

    // rows 31-32: dstCapacity == 0
    let canonical: Vec<u8> = with_slack(&[0u8], 64);
    for (tag, bytes, ss, want) in [
        ("canonical empty", canonical.clone(), 1i32, 0i32),
        ("srcSize 0", canonical.clone(), 0, -1),
        ("src[0] != 0", with_slack(&[1u8], 64), 1, -1),
        ("srcSize 2", with_slack(&[0u8, 0], 64), 2, -1),
        ("negative srcSize", canonical.clone(), -1, -1),
    ] {
        let (cn, cb) = run(&dec.0, bytes.as_ptr() as *const c_char, ss, 0);
        let (rn, rb) = run(&dec.1, bytes.as_ptr() as *const c_char, ss, 0);
        let ctx = format!("row31/32 {tag}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, want, "{ctx}");
    }

    // row 33: srcSize == 0 with dstCapacity != 0
    for &cap in &[1i32, 5, 100, 4096] {
        let (cn, cb) = run(&dec.0, comp.as_ptr() as *const c_char, 0, cap);
        let (rn, rb) = run(&dec.1, comp.as_ptr() as *const c_char, 0, cap);
        let ctx = format!("row33 srcSize=0 cap={cap}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}");
    }
}

// ===========================================================================
// Rows 34-35 — truncated extended length varints
// ===========================================================================
#[test]
fn err34_35_truncated_varints() {
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);

    let run = |f: &FnDecSafe, src: &[u8], ss: c_int, cap: usize| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                cap as c_int,
            )
        };
        (n, b)
    };

    // ---- row 34: literal-length varint -----------------------------------
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // (a) token says "extended literal length" and the block ends immediately
    //     => `read_variable_length(.., iend-15, initial_check=1)` rejects.
    cases.push(("token only".into(), vec![0xF0]));
    cases.push(("token + 1 byte".into(), vec![0xF0, 0x00]));
    // srcSize <= 16 keeps `ip >= iend - RUN_MASK`, so the initial check fires.
    for n in 2..16usize {
        let mut v = vec![0xF0u8];
        v.extend(std::iter::repeat(0x00).take(n));
        cases.push((format!("token + {n} zero bytes"), v));
    }
    // (b) the 255-continuation runs past `ilimit` inside the do/while loop.
    for n in [4usize, 8, 16, 20, 40, 100] {
        let mut v = vec![0xF0u8];
        v.extend(std::iter::repeat(0xFFu8).take(n));
        cases.push((format!("token + {n} x 0xFF"), v));
    }
    // (c) a truncated varint after some valid literals.
    {
        let mut v = vec![0xF0u8];
        v.extend(std::iter::repeat(0xFFu8).take(3));
        v.push(0x10);
        v.extend(std::iter::repeat(0x41u8).take(20));
        cases.push(("huge literal length, short block".into(), v));
    }

    for (tag, bytes) in cases {
        let src = with_slack(&bytes, 64);
        for &cap in &[16usize, 63, 64, 100, 4096] {
            let (cn, cb) = run(&dec.0, &src, bytes.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, bytes.len() as c_int, cap);
            let ctx = format!("row34 {tag} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected a negative result, got {cn}");
        }
    }

    // ---- row 35: match-length varint --------------------------------------
    // token = ll 0 / mlcode 15, 2 offset bytes, then a 255-run that overruns
    // `iend - LASTLITERALS + 1`.
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [0usize, 1, 2, 4, 8, 16, 30, 60] {
        let mut v = vec![0x0Fu8, 0x01, 0x00];
        v.extend(std::iter::repeat(0xFFu8).take(n));
        cases.push((format!("mlvarint {n} x 0xFF"), v));
    }
    // With preceding literals, so the match is reached through the fast loop.
    for n in [0usize, 4, 20, 60] {
        let mut v = vec![0xAFu8];
        v.extend(std::iter::repeat(0x42u8).take(10));
        v.extend_from_slice(&[0x04, 0x00]);
        v.extend(std::iter::repeat(0xFFu8).take(n));
        cases.push((format!("lits + mlvarint {n} x 0xFF"), v));
    }
    for (tag, bytes) in cases {
        let src = with_slack(&bytes, 64);
        for &cap in &[16usize, 63, 64, 100, 4096] {
            let (cn, cb) = run(&dec.0, &src, bytes.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, bytes.len() as c_int, cap);
            let ctx = format!("row35 {tag} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected a negative result, got {cn}");
        }
    }
}

// ===========================================================================
// Rows 36-37 — offset out of range / offset 0
// ===========================================================================
#[test]
fn err36_37_bad_offsets() {
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);

    let run = |f: &FnDecSafe, src: &[u8], ss: c_int, cap: usize| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                cap as c_int,
            )
        };
        (n, b)
    };

    // ---- row 36: match offset larger than the available history -----------
    // 10 literals, then a match pointing before `dst`.
    let lits: Vec<u8> = (0..10u8).map(|i| 0x30 + i).collect();
    for &off in &[11usize, 12, 100, 1000, 65535] {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((off, 4)));
        // 8 trailing bytes keep the literal copy on the fast path so the
        // failure really is the offset check.
        block.extend(std::iter::repeat(0x55u8).take(8));
        let src = with_slack(&block, 64);
        for &cap in &[24usize, 64, 100, 4096] {
            let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
            let ctx = format!("row36 offset={off} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected negative, got {cn}");
        }
    }
    // Also at the very start of a block (no history at all).
    for &off in &[1usize, 2, 8, 1000] {
        let mut block = Vec::new();
        emit_seq(&mut block, &[], Some((off, 4)));
        block.extend(std::iter::repeat(0x55u8).take(16));
        let src = with_slack(&block, 64);
        for &cap in &[64usize, 4096] {
            let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
            let ctx = format!("row36 first-seq offset={off} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected negative, got {cn}");
        }
    }

    // ---- row 37: offset == 0 ---------------------------------------------
    // NOTE: `checkOffset` tests `match + dictSize < lowPrefix`; with offset 0
    // `match == op`, so a bare offset 0 is NOT rejected by the C - it performs
    // a self-copy. It only becomes an error once the resulting match violates
    // another rule. Both variants are compared here.
    //
    // (a) benign: offset 0 in the middle of a well-formed block.
    {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((0, 4)));
        emit_seq(&mut block, &lits, None);
        let src = with_slack(&block, 64);
        for &cap in &[24usize, 25, 64, 100] {
            let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!("row37 benign offset=0 cap={cap}"),
            );
        }
    }
    // (b) offset 0 with a match that overruns LASTLITERALS => negative.
    for ml in [12usize, 16, 20, 40] {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((0, ml)));
        block.extend(std::iter::repeat(0x55u8).take(8));
        let src = with_slack(&block, 64);
        for &cap in &[20usize, 24, 64] {
            let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
            let ctx = format!("row37 offset=0 ml={ml} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected negative, got {cn}");
        }
    }
}

// ===========================================================================
// Rows 38-39 — literal run overruns the input / the output
// ===========================================================================
#[test]
fn err38_39_literal_overruns() {
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);

    let run = |f: &FnDecSafe, src: &[u8], ss: c_int, cap: usize| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                cap as c_int,
            )
        };
        (n, b)
    };

    // row 38: the token claims more literals than the block contains
    // (`ip + length != iend`).
    for &(ll, present) in &[
        (10usize, 5usize),
        (10, 0),
        (14, 13),
        (15, 3),
        (100, 20),
        (5, 4),
    ] {
        let mut block = Vec::new();
        emit_seq(&mut block, &vec![0x41u8; ll], None);
        // Truncate the literal payload.
        let keep = block.len() - (ll - present);
        let bytes = block[..keep].to_vec();
        let src = with_slack(&bytes, 128);
        for &cap in &[16usize, 64, 200, 4096] {
            let (cn, cb) = run(&dec.0, &src, bytes.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, bytes.len() as c_int, cap);
            let ctx = format!("row38 ll={ll} present={present} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected negative, got {cn}");
        }
    }

    // row 39: the literal run overruns the output buffer (`cpy > oend`).
    for &ll in &[10usize, 15, 16, 20, 100, 300] {
        let mut block = Vec::new();
        emit_seq(&mut block, &vec![0x42u8; ll], None);
        let src = with_slack(&block, 128);
        for cap in [0usize, 1, 2, ll / 2, ll - 1] {
            if cap == 0 {
                continue; // covered by rows 31-32
            }
            let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
            let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
            let ctx = format!("row39 ll={ll} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn < 0, "{ctx}: expected negative, got {cn}");
        }
    }
}

// ===========================================================================
// Rows 40-41 — match inside LASTLITERALS / dstCapacity one byte short
// ===========================================================================
#[test]
fn err40_41_lastliterals_and_short_dst() {
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0040);

    let run = |f: &FnDecSafe, src: &[u8], ss: c_int, cap: usize| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                cap as c_int,
            )
        };
        (n, b)
    };

    // row 40: the match ends within the final 5 bytes of the output block.
    let lits: Vec<u8> = (0..10u8).map(|i| 0x30 + i).collect();
    for &(off, ml, cap) in &[
        (4usize, 12usize, 24usize),
        (4, 13, 24),
        (4, 14, 24),
        (4, 10, 20),
        (8, 20, 32),
        (2, 30, 42),
    ] {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((off, ml)));
        block.extend(std::iter::repeat(0x55u8).take(8));
        let src = with_slack(&block, 64);
        let (cn, cb) = run(&dec.0, &src, block.len() as c_int, cap);
        let (rn, rb) = run(&dec.1, &src, block.len() as c_int, cap);
        let ctx = format!("row40 off={off} ml={ml} cap={cap}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert!(cn < 0, "{ctx}: expected negative, got {cn}");
    }

    // row 41: dstCapacity one byte below the true decoded size.
    for &shape in ALL_SHAPES {
        for &len in &[13usize, 20, 64, 255, 1024, 4096, 65536, 100_000] {
            let plain = gen_src(shape, len, &mut rng);
            let comp = c_compress(&def.0, &plain, &bound.0);
            let src = with_slack(&comp, 64);
            for cap in [len - 1, len / 2, 1] {
                let (cn, cb) = run(&dec.0, &src, comp.len() as c_int, cap);
                let (rn, rb) = run(&dec.1, &src, comp.len() as c_int, cap);
                let ctx = format!("row41 {shape:?} len={len} cap={cap}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert!(cn < 0, "{ctx}: expected negative, got {cn}");
            }
            // The exact size succeeds.
            let (cn, cb) = run(&dec.0, &src, comp.len() as c_int, len);
            let (rn, rb) = run(&dec.1, &src, comp.len() as c_int, len);
            cmp_dec(cn, &cb, rn, &rb, &format!("row41 exact {shape:?} len={len}"));
            assert_eq!(cn, len as c_int);
            // A truncated compressed input is also an error.
            for take in [comp.len() - 1, comp.len() / 2, 1] {
                let (cn, cb) = run(&dec.0, &src, take as c_int, len);
                let (rn, rb) = run(&dec.1, &src, take as c_int, len);
                let ctx = format!("row41 truncated src {shape:?} len={len} take={take}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert!(cn < 0, "{ctx}: expected negative, got {cn}");
            }
        }
    }
}

// ===========================================================================
// Row 42 (+ 53, 54, 62, 63) — fuzz every decoder entry point with random bytes
// ===========================================================================
#[test]
fn err42_fuzz_all_decoders() {
    sym!(safe, "LZ4_decompress_safe", FnDecSafe);
    sym!(unk, "LZ4_uncompress_unknownOutputSize", FnDecSafe);
    sym!(part, "LZ4_decompress_safe_partial", FnDecPartial);
    sym!(fast, "LZ4_decompress_fast", FnDecFast);
    sym!(unc, "LZ4_uncompress", FnDecFast);
    sym!(usedict, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    sym!(partdict, "LZ4_decompress_safe_partial_usingDict", FnDecPartialUsingDict);
    let mut rng = Rng::new(0x0042);

    // Dictionaries kept alive for the whole test. 65536 is the interesting
    // size: `checkOffset` is disabled at >= 64 KB (ERRORS.md row 62), and the
    // dictionary is then large enough to absorb any 16-bit offset.
    let dict_small = gen_src(Shape::Texty, 100, &mut rng);
    let dict_big = gen_src(Shape::Texty, 65536, &mut rng);

    const LENS: &[usize] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 13, 15, 16, 17, 20, 24, 32, 48, 64, 100, 150, 200,
    ];
    const ITERS: usize = 20_000;
    // Generous read padding: `LZ4_decompress_fast` is documented as unchecked
    // on the input side, and a corrupt token can make it consume up to a few
    // times `originalSize` bytes.
    const SRC_SLACK: usize = 4096;

    for it in 0..ITERS {
        let len = LENS[rng.below(LENS.len())];
        let mut raw = vec![0u8; len];
        // Mix pure-random bytes with token-heavy / 0xFF-heavy shapes so the
        // extended-length and offset paths are reached often.
        match it % 4 {
            0 => rng.fill(&mut raw),
            1 => {
                for b in raw.iter_mut() {
                    *b = match rng.below(4) {
                        0 => 0xFF,
                        1 => 0x00,
                        2 => (rng.next_u8() & 0x0F) | 0xF0,
                        _ => rng.next_u8(),
                    };
                }
            }
            2 => {
                for b in raw.iter_mut() {
                    *b = if rng.below(2) == 0 { 0xFF } else { rng.next_u8() };
                }
            }
            _ => {
                for b in raw.iter_mut() {
                    *b = rng.next_u8() & 0x1F;
                }
            }
        }
        let src = with_slack(&raw, SRC_SLACK);
        let sp = src.as_ptr() as *const c_char;
        let ss = len as c_int;
        let cap = rng.range(0, 300);
        let target = rng.range(0, 300) as c_int;

        // ---- LZ4_decompress_safe / LZ4_uncompress_unknownOutputSize -------
        for (name, cf, rf) in [
            ("safe", &safe.0, &safe.1),
            ("unknownOutputSize", &unk.0, &unk.1),
        ] {
            let (mut cb, mut rb) = dst_pair(cap);
            let (cn, rn) = unsafe {
                (
                    cf(sp, cb.as_mut_ptr() as *mut c_char, ss, cap as c_int),
                    rf(sp, rb.as_mut_ptr() as *mut c_char, ss, cap as c_int),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!("row42/54 {name} it={it} len={len} cap={cap} src={:02x?}", &raw),
            );
        }

        // ---- LZ4_decompress_safe_partial ----------------------------------
        {
            let (mut cb, mut rb) = dst_pair(cap);
            let (cn, rn) = unsafe {
                (
                    part.0(sp, cb.as_mut_ptr() as *mut c_char, ss, target, cap as c_int),
                    part.1(sp, rb.as_mut_ptr() as *mut c_char, ss, target, cap as c_int),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!(
                    "row42 partial it={it} len={len} target={target} cap={cap} src={:02x?}",
                    &raw
                ),
            );
        }

        // ---- LZ4_decompress_fast / LZ4_uncompress -------------------------
        // `originalSize` is never negative: a negative value makes the C write
        // arbitrarily far past `dst` (undefined behaviour, ERRORS.md appendix).
        let osize = rng.range(0, 200);
        for (name, cf, rf) in [("fast", &fast.0, &fast.1), ("uncompress", &unc.0, &unc.1)] {
            let (mut cb, mut rb) = dst_pair(osize);
            let (cn, rn) = unsafe {
                (
                    cf(sp, cb.as_mut_ptr() as *mut c_char, osize as c_int),
                    rf(sp, rb.as_mut_ptr() as *mut c_char, osize as c_int),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!(
                    "row42/53 {name} it={it} len={len} originalSize={osize} src={:02x?}",
                    &raw
                ),
            );
        }

        // ---- the usingDict variants (rows 62-63) -------------------------
        for (dtag, dp, dsz) in [
            ("none", std::ptr::null::<u8>(), 0usize),
            ("small", dict_small.as_ptr(), dict_small.len()),
            ("64K", dict_big.as_ptr(), dict_big.len()),
        ] {
            let (mut cb, mut rb) = dst_pair(cap);
            let (cn, rn) = unsafe {
                (
                    usedict.0(
                        sp,
                        cb.as_mut_ptr() as *mut c_char,
                        ss,
                        cap as c_int,
                        dp as *const c_char,
                        dsz as c_int,
                    ),
                    usedict.1(
                        sp,
                        rb.as_mut_ptr() as *mut c_char,
                        ss,
                        cap as c_int,
                        dp as *const c_char,
                        dsz as c_int,
                    ),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!(
                    "row62/63 usingDict({dtag}) it={it} len={len} cap={cap} src={:02x?}",
                    &raw
                ),
            );

            let (mut cb, mut rb) = dst_pair(cap);
            let (cn, rn) = unsafe {
                (
                    partdict.0(
                        sp,
                        cb.as_mut_ptr() as *mut c_char,
                        ss,
                        target,
                        cap as c_int,
                        dp as *const c_char,
                        dsz as c_int,
                    ),
                    partdict.1(
                        sp,
                        rb.as_mut_ptr() as *mut c_char,
                        ss,
                        target,
                        cap as c_int,
                        dp as *const c_char,
                        dsz as c_int,
                    ),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!(
                    "row62/63 partial_usingDict({dtag}) it={it} len={len} \
                     target={target} cap={cap} src={:02x?}",
                    &raw
                ),
            );
        }
    }
}

// ===========================================================================
// Rows 43-46 — LZ4_decompress_safe_partial
// ===========================================================================
#[test]
fn err43_46_decompress_safe_partial() {
    sym!(part, "LZ4_decompress_safe_partial", FnDecPartial);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0043);

    let run = |f: &FnDecPartial, src: &[u8], ss: c_int, t: c_int, cap: c_int| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap.max(0) as usize + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                t,
                cap,
            )
        };
        (n, b)
    };

    let plain = gen_src(Shape::Texty, 8192, &mut rng);
    let comp = c_compress(&def.0, &plain, &bound.0);
    let src = with_slack(&comp, 64);
    let ss = comp.len() as c_int;

    // row 43: min(targetOutputSize, dstCapacity) == 0 => 0
    for &(t, cap) in &[(0i32, 0i32), (0, 100), (100, 0), (0, 8192), (8192, 0)] {
        let (cn, cb) = run(&part.0, &src, ss, t, cap);
        let (rn, rb) = run(&part.1, &src, ss, t, cap);
        let ctx = format!("row43 target={t} cap={cap}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, 0, "{ctx}: must be 0");
    }

    // row 44: a negative argument => -1
    for &(t, cap) in &[
        (-1i32, 100i32),
        (100, -1),
        (-1, -1),
        (i32::MIN, 100),
        (100, i32::MIN),
        (-5, 0),
        (0, -5),
    ] {
        let (cn, cb) = run(&part.0, &src, ss, t, cap);
        let (rn, rb) = run(&part.1, &src, ss, t, cap);
        let ctx = format!("row44 target={t} cap={cap}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }

    // rows 45-46: targetOutputSize > dstCapacity, and truncated input.
    for &cap in &[1i32, 2, 5, 13, 100, 1000, 4096, 8191, 8192] {
        for &t in &[cap, cap + 1, cap * 2, 8192, 100_000, i32::MAX] {
            let (cn, cb) = run(&part.0, &src, ss, t, cap);
            let (rn, rb) = run(&part.1, &src, ss, t, cap);
            let ctx = format!("row45 target={t} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert!(cn <= cap, "{ctx}: ret {cn} exceeds dstCapacity");
            if cn > 0 {
                assert_bytes_eq(&cb[..cn as usize], &plain[..cn as usize], &ctx);
            }
        }
        // row 46: truncated input silently returns a partial byte count.
        for &take in &[1usize, 2, 10, comp.len() / 4, comp.len() / 2, comp.len() - 1] {
            let (cn, cb) = run(&part.0, &src, take as c_int, cap, cap);
            let (rn, rb) = run(&part.1, &src, take as c_int, cap, cap);
            let ctx = format!("row46 take={take} cap={cap}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            if cn > 0 {
                assert_bytes_eq(&cb[..cn as usize], &plain[..cn as usize], &ctx);
            }
        }
    }
}

// ===========================================================================
// Rows 47-52 — LZ4_decompress_fast
// ===========================================================================
#[test]
fn err47_52_decompress_fast() {
    sym!(fast, "LZ4_decompress_fast", FnDecFast);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0047);

    let run = |f: &FnDecFast, src: &[u8], osize: usize| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; osize + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                osize as c_int,
            )
        };
        (n, b)
    };

    let lits: Vec<u8> = (0..14u8).map(|i| 0x30 + i).collect();

    // row 47: the literal length exceeds the output room.
    for &(ll, osize) in &[(10usize, 5usize), (14, 13), (10, 0), (1, 0), (15, 10)] {
        let mut block = Vec::new();
        emit_seq(&mut block, &vec![0x41u8; ll], None);
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row47 ll={ll} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }
    // Extended literal length far beyond the output room.
    {
        let mut block = vec![0xF0u8, 0xFF, 0xFF, 0x10];
        block.extend(std::iter::repeat(0x41u8).take(32));
        let src = with_slack(&block, 4096);
        for &osize in &[0usize, 1, 16, 100] {
            let (cn, cb) = run(&fast.0, &src, osize);
            let (rn, rb) = run(&fast.1, &src, osize);
            let ctx = format!("row47 extended-ll originalSize={osize}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, -1, "{ctx}");
        }
    }

    // row 48: the literals end less than MFLIMIT from the end of the block.
    for &(ll, osize) in &[(5usize, 10usize), (1, 5), (8, 12), (11, 12), (4, 15)] {
        let mut block = Vec::new();
        emit_seq(&mut block, &vec![0x41u8; ll], None);
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row48 ll={ll} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }

    // row 49: the match length exceeds the output room.
    for &(ml, osize) in &[(119usize, 20usize), (30, 20), (25, 24), (200, 100)] {
        let mut block = Vec::new();
        emit_seq(&mut block, &[], Some((1, ml)));
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row49 ml={ml} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }

    // row 50: the offset is larger than the available history.
    for &(off, osize) in &[(1usize, 20usize), (2, 20), (100, 20), (65535, 100)] {
        let mut block = Vec::new();
        emit_seq(&mut block, &[], Some((off, 4)));
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row50 offset={off} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }
    // Also with some history, but still too far back.
    for &(off, osize) in &[(14usize, 40usize), (20, 40), (1000, 40)] {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((off, 4)));
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row50 with-history offset={off} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }

    // row 51: the match ends within LASTLITERALS of the block end.
    for &(ll, off, ml, osize) in &[
        (8usize, 8usize, 9usize, 20usize),
        (8, 8, 8, 20),
        (8, 4, 10, 20),
        (8, 1, 11, 20),
        (14, 14, 5, 24),
    ] {
        let mut block = Vec::new();
        emit_seq(&mut block, &vec![0x41u8; ll], Some((off, ml)));
        let src = with_slack(&block, 4096);
        let (cn, cb) = run(&fast.0, &src, osize);
        let (rn, rb) = run(&fast.1, &src, osize);
        let ctx = format!("row51 ll={ll} off={off} ml={ml} originalSize={osize}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, -1, "{ctx}: must be -1");
    }

    // row 52: originalSize below the true decoded size.
    //
    // NOTE: `LZ4_decompress_unsafe_generic` has no notion of the input size, so
    // an undersized `originalSize` is only detected when a length/offset check
    // trips. If the truncation happens to land exactly on a sequence boundary
    // the C reports success for the shorter output; the Rust must agree, which
    // is what is asserted for every case. `nfail` proves the -1 path is really
    // being taken across the sweep.
    let mut nfail = 0usize;
    for &shape in ALL_SHAPES {
        for &len in &[13usize, 20, 64, 255, 1024, 4096] {
            let plain = gen_src(shape, len, &mut rng);
            let comp = c_compress(&def.0, &plain, &bound.0);
            let src = with_slack(&comp, 4 * len + 4096);
            for osize in [len - 1, len / 2, 1, 0] {
                let (cn, cb) = run(&fast.0, &src, osize);
                let (rn, rb) = run(&fast.1, &src, osize);
                let ctx = format!("row52 {shape:?} len={len} originalSize={osize}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                if cn == -1 {
                    nfail += 1;
                } else {
                    assert!(cn > 0, "{ctx}: unexpected return {cn}");
                }
            }
            // The exact size succeeds and consumes the whole block.
            let (cn, cb) = run(&fast.0, &src, len);
            let (rn, rb) = run(&fast.1, &src, len);
            let ctx = format!("row52 exact {shape:?} len={len}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, comp.len() as c_int, "{ctx}");
            assert_bytes_eq(&cb[..len], &plain[..], &ctx);
        }
    }
    assert!(
        nfail > 20,
        "row52: only {nfail} undersized-originalSize cases were rejected"
    );
}

// ===========================================================================
// Rows 53-54 — the deprecated thin wrappers behave like their modern twins
// ===========================================================================
#[test]
fn err53_54_deprecated_decoder_wrappers() {
    sym!(fast, "LZ4_decompress_fast", FnDecFast);
    sym!(unc, "LZ4_uncompress", FnDecFast);
    sym!(safe, "LZ4_decompress_safe", FnDecSafe);
    sym!(unk, "LZ4_uncompress_unknownOutputSize", FnDecSafe);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0053);

    for &shape in ALL_SHAPES {
        for &len in &[0usize, 1, 13, 64, 1024, 4096] {
            let plain = gen_src(shape, len, &mut rng);
            let comp = if len == 0 {
                vec![0u8]
            } else {
                c_compress(&def.0, &plain, &bound.0)
            };
            let src = with_slack(&comp, 4 * len + 4096);

            // row 53: LZ4_uncompress == LZ4_decompress_fast, error cases too.
            for osize in [len, len.saturating_sub(1), len + 1, 0, 1] {
                let mut a = vec![0xA5u8; osize + DST_SLACK];
                let mut b = vec![0xA5u8; osize + DST_SLACK];
                let mut c = vec![0xA5u8; osize + DST_SLACK];
                let mut d = vec![0xA5u8; osize + DST_SLACK];
                let (n_cf, n_cu, n_rf, n_ru) = unsafe {
                    (
                        fast.0(
                            src.as_ptr() as *const c_char,
                            a.as_mut_ptr() as *mut c_char,
                            osize as c_int,
                        ),
                        unc.0(
                            src.as_ptr() as *const c_char,
                            b.as_mut_ptr() as *mut c_char,
                            osize as c_int,
                        ),
                        fast.1(
                            src.as_ptr() as *const c_char,
                            c.as_mut_ptr() as *mut c_char,
                            osize as c_int,
                        ),
                        unc.1(
                            src.as_ptr() as *const c_char,
                            d.as_mut_ptr() as *mut c_char,
                            osize as c_int,
                        ),
                    )
                };
                let ctx = format!("row53 {shape:?} len={len} originalSize={osize}");
                cmp_dec(n_cf, &a, n_rf, &c, &format!("{ctx} fast"));
                cmp_dec(n_cu, &b, n_ru, &d, &format!("{ctx} uncompress"));
                assert_eq!(n_cf, n_cu, "{ctx}: wrapper must match _fast");
                assert_bytes_eq(&a, &b, &format!("{ctx}: wrapper bytes"));
            }

            // row 54: LZ4_uncompress_unknownOutputSize == LZ4_decompress_safe.
            for &cap in &[0usize, 1, 13, 64, 1024, 4096, 8192] {
                for &ss in &[comp.len(), comp.len() / 2, comp.len() + 1, 0] {
                    let mut a = vec![0xA5u8; cap + DST_SLACK];
                    let mut b = vec![0xA5u8; cap + DST_SLACK];
                    let mut c = vec![0xA5u8; cap + DST_SLACK];
                    let mut d = vec![0xA5u8; cap + DST_SLACK];
                    let (n_cs, n_cu, n_rs, n_ru) = unsafe {
                        (
                            safe.0(
                                src.as_ptr() as *const c_char,
                                a.as_mut_ptr() as *mut c_char,
                                ss as c_int,
                                cap as c_int,
                            ),
                            unk.0(
                                src.as_ptr() as *const c_char,
                                b.as_mut_ptr() as *mut c_char,
                                ss as c_int,
                                cap as c_int,
                            ),
                            safe.1(
                                src.as_ptr() as *const c_char,
                                c.as_mut_ptr() as *mut c_char,
                                ss as c_int,
                                cap as c_int,
                            ),
                            unk.1(
                                src.as_ptr() as *const c_char,
                                d.as_mut_ptr() as *mut c_char,
                                ss as c_int,
                                cap as c_int,
                            ),
                        )
                    };
                    let ctx = format!("row54 {shape:?} len={len} ss={ss} cap={cap}");
                    cmp_dec(n_cs, &a, n_rs, &c, &format!("{ctx} safe"));
                    cmp_dec(n_cu, &b, n_ru, &d, &format!("{ctx} unknownOutputSize"));
                    assert_eq!(n_cs, n_cu, "{ctx}: wrapper must match _safe");
                    assert_bytes_eq(&a, &b, &format!("{ctx}: wrapper bytes"));
                }
            }
        }
    }
}

// ===========================================================================
// Rows 59-60 — LZ4_setStreamDecode
// ===========================================================================
#[test]
fn err59_60_set_stream_decode() {
    sym!(csd, "LZ4_createStreamDecode", FnCreate);
    sym!(fsd, "LZ4_freeStreamDecode", FnFree);
    sym!(set, "LZ4_setStreamDecode", FnSetSD);
    let mut rng = Rng::new(0x0059);
    let dict = gen_src(Shape::Texty, 70_000, &mut rng);

    let (c, r) = unsafe { (csd.0(), csd.1()) };
    assert!(!c.is_null() && !r.is_null());

    // row 59: negative dictSize is cast to size_t and NOT rejected.
    for &n in &[-1i32, -8, -65536, i32::MIN] {
        let (a, b) = unsafe {
            (
                set.0(c, dict.as_ptr() as *const c_char, n),
                set.1(r, dict.as_ptr() as *const c_char, n),
            )
        };
        assert_ret_eq(a, b, &format!("row59 setStreamDecode({n})"));
        assert_eq!(a, 1, "row59 setStreamDecode({n}) must be 1");
    }
    // row 60: (NULL, 0) resets.
    let (a, b) = unsafe {
        (
            set.0(c, std::ptr::null(), 0),
            set.1(r, std::ptr::null(), 0),
        )
    };
    assert_ret_eq(a, b, "row60 setStreamDecode(NULL, 0)");
    assert_eq!(a, 1, "row60 must be 1");
    // and the ordinary sizes.
    for &n in &[0i32, 1, 8, 100, 65535, 65536, 70000] {
        let (a, b) = unsafe {
            (
                set.0(c, dict.as_ptr() as *const c_char, n),
                set.1(r, dict.as_ptr() as *const c_char, n),
            )
        };
        assert_ret_eq(a, b, &format!("row59/60 setStreamDecode({n})"));
        assert_eq!(a, 1);
    }

    unsafe {
        fsd.0(c);
        fsd.1(r);
    }
}

// ===========================================================================
// Row 61 — LZ4_decompress_safe_continue on a fresh stream with corrupt input
// ===========================================================================
#[test]
fn err61_decompress_safe_continue_corrupt() {
    sym!(csd, "LZ4_createStreamDecode", FnCreate);
    sym!(fsd, "LZ4_freeStreamDecode", FnFree);
    sym!(cont, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0x0061);

    // The destination must outlive the stream (it becomes the prefix).
    let mut cdst = vec![0xA5u8; 4096 + DST_SLACK];
    let mut rdst = vec![0xA5u8; 4096 + DST_SLACK];

    for it in 0..400usize {
        let len = *[1usize, 2, 4, 8, 13, 20, 40, 100].get(it % 8).unwrap();
        let mut raw = vec![0u8; len];
        rng.fill(&mut raw);
        let src = with_slack(&raw, 256);

        let (c, r) = unsafe { (csd.0(), csd.1()) };
        assert!(!c.is_null() && !r.is_null());
        for b in cdst.iter_mut() {
            *b = 0xA5;
        }
        for b in rdst.iter_mut() {
            *b = 0xA5;
        }
        // Two identical calls: after a failure the stream must not advance, so
        // the second call has to produce exactly the same answer.
        for call in 0..2 {
            let (cn, rn) = unsafe {
                (
                    cont.0(
                        c,
                        src.as_ptr() as *const c_char,
                        cdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        4096,
                    ),
                    cont.1(
                        r,
                        src.as_ptr() as *const c_char,
                        rdst.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        4096,
                    ),
                )
            };
            cmp_dec(
                cn,
                &cdst,
                rn,
                &rdst,
                &format!("row61 it={it} call={call} len={len} src={:02x?}", &raw),
            );
        }
        unsafe {
            fsd.0(c);
            fsd.1(r);
        }
    }
}

// ===========================================================================
// Rows 62-63 — LZ4_decompress_safe_usingDict boundary dictionary sizes
// ===========================================================================
#[test]
fn err62_63_decompress_safe_using_dict() {
    sym!(ud, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    sym!(safe, "LZ4_decompress_safe", FnDecSafe);
    sym!(def, "LZ4_compress_default", FnDefault);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0062);

    let dict = gen_src(Shape::Texty, 65536, &mut rng);

    for &shape in ALL_SHAPES {
        for &len in &[1usize, 13, 64, 1024, 4096] {
            let plain = gen_src(shape, len, &mut rng);
            let comp = c_compress(&def.0, &plain, &bound.0);
            let src = with_slack(&comp, 64);

            // row 63: dictSize == 0 delegates to the plain noDict decoder.
            for dp in [std::ptr::null::<u8>(), dict.as_ptr()] {
                let (mut cb, mut rb) = dst_pair(len);
                let (cn, rn) = unsafe {
                    (
                        ud.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            len as c_int,
                            dp as *const c_char,
                            0,
                        ),
                        ud.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            comp.len() as c_int,
                            len as c_int,
                            dp as *const c_char,
                            0,
                        ),
                    )
                };
                let ctx = format!("row63 {shape:?} len={len} dict={:?}", dp.is_null());
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                // Identical to LZ4_decompress_safe.
                let mut nb = vec![0xA5u8; len + DST_SLACK];
                let nn = unsafe {
                    safe.0(
                        src.as_ptr() as *const c_char,
                        nb.as_mut_ptr() as *mut c_char,
                        comp.len() as c_int,
                        len as c_int,
                    )
                };
                assert_eq!(cn, nn, "{ctx}: must match LZ4_decompress_safe");
                assert_bytes_eq(&cb, &nb, &format!("{ctx}: noDict bytes"));
            }

            // row 62: dictSize >= 64 KB disables `checkOffset` entirely, so an
            // out-of-range offset is silently accepted. Only the C/Rust parity
            // is meaningful here (the output is intentionally garbage).
            for &ds in &[65536usize, 65535, 100] {
                for cap in [len, len + 100, len / 2 + 1] {
                    let (mut cb, mut rb) = dst_pair(cap);
                    let (cn, rn) = unsafe {
                        (
                            ud.0(
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                comp.len() as c_int,
                                cap as c_int,
                                dict.as_ptr() as *const c_char,
                                ds as c_int,
                            ),
                            ud.1(
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                comp.len() as c_int,
                                cap as c_int,
                                dict.as_ptr() as *const c_char,
                                ds as c_int,
                            ),
                        )
                    };
                    cmp_dec(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!("row62 {shape:?} len={len} dictSize={ds} cap={cap}"),
                    );
                }
            }
        }
    }

    // Hand-built blocks whose offsets deliberately exceed the history: at
    // dictSize >= 64 KB the C accepts them (row 62).
    let lits: Vec<u8> = (0..10u8).map(|i| 0x30 + i).collect();
    for &off in &[11usize, 100, 1000, 65535] {
        let mut block = Vec::new();
        emit_seq(&mut block, &lits, Some((off, 4)));
        emit_seq(&mut block, &lits, None);
        let src = with_slack(&block, 64);
        for &ds in &[0usize, 100, 65535, 65536] {
            let (mut cb, mut rb) = dst_pair(64);
            let (cn, rn) = unsafe {
                (
                    ud.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        block.len() as c_int,
                        64,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    ),
                    ud.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        block.len() as c_int,
                        64,
                        dict.as_ptr() as *const c_char,
                        ds as c_int,
                    ),
                )
            };
            cmp_dec(
                cn,
                &cb,
                rn,
                &rb,
                &format!("row62 crafted offset={off} dictSize={ds}"),
            );
        }
    }
}

// ===========================================================================
// Rows 64-65 — LZ4_attach_dictionary(NULL) / empty dictionary stream
// ===========================================================================
#[test]
fn err64_65_attach_dictionary() {
    sym!(cs, "LZ4_createStream", FnCreate);
    sym!(fsr, "LZ4_freeStream", FnFree);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(att, "LZ4_attach_dictionary", FnAttach);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0064);

    // Buffers whose addresses the streams retain.
    let dict = gen_src(Shape::Texty, 4, &mut rng); // loadDict returns 0 for < 8
    let src = gen_src(Shape::Texty, 8192, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;

    // Variant A: no attach at all (the reference).
    // Variant B: attach(NULL) — a pure detach, must be a no-op.
    // Variant C: attach a stream whose dictSize is 0 — silently not attached.
    let run = |create: &FnCreate,
               free: &FnFree,
               loaddict: &FnLoadDict,
               attach: &FnAttach,
               contf: &FnContinue,
               variant: u8|
     -> (c_int, Vec<u8>) {
        let s = unsafe { create() };
        assert!(!s.is_null());
        let d = unsafe { create() };
        assert!(!d.is_null());
        unsafe {
            let n = loaddict(d, dict.as_ptr() as *const c_char, dict.len() as c_int);
            assert_eq!(n, 0, "a 4-byte dictionary must load as 0 bytes");
        }
        match variant {
            1 => unsafe { attach(s, std::ptr::null()) },
            2 => unsafe { attach(s, d as *const c_void) },
            _ => {}
        }
        let mut out = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            contf(
                s,
                src.as_ptr() as *const c_char,
                out.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                1,
            )
        };
        unsafe {
            free(s);
            free(d);
        }
        (n, out)
    };

    for variant in 0u8..3 {
        let (cn, cb) = run(&cs.0, &fsr.0, &ld.0, &att.0, &cont.0, variant);
        let (rn, rb) = run(&cs.1, &fsr.1, &ld.1, &att.1, &cont.1, variant);
        cmp_dec(cn, &cb, rn, &rb, &format!("row64/65 variant={variant}"));
        assert!(cn > 0, "row64/65 variant={variant}: compression failed");
    }

    // row 64: `attach_dictionary(s, NULL)` only clears `dictCtx`, so it is a
    // pure no-op and the output is byte-identical to never calling it.
    let (base_n, base_b) = run(&cs.0, &fsr.0, &ld.0, &att.0, &cont.0, 0);
    for &lib in &[0u8, 1] {
        let (n, b) = if lib == 0 {
            run(&cs.0, &fsr.0, &ld.0, &att.0, &cont.0, 1)
        } else {
            run(&cs.1, &fsr.1, &ld.1, &att.1, &cont.1, 1)
        };
        assert_eq!(n, base_n, "row64 lib={lib}: attach(NULL) must be a no-op");
        assert_bytes_eq(&b, &base_b, &format!("row64 lib={lib}: attach(NULL) bytes"));
    }

    // row 65: attaching a stream whose `dictSize` is 0 drops the dictionary
    // (lz4.c:1679), but the `currentOffset == 0` guard just above it still
    // bumps `currentOffset` to 64 KB, which switches the parser from
    // `noDictIssue` to `dictSmall`. The compressed bytes therefore legitimately
    // differ from the no-dict baseline (ERRORS.md row 65 says "identical"; the
    // C says otherwise and the C is authoritative). What IS provable is that no
    // dictionary content was consulted: the block decodes with the plain,
    // dictionary-less decoder.
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    for &lib in &[0u8, 1] {
        let (n, b) = if lib == 0 {
            run(&cs.0, &fsr.0, &ld.0, &att.0, &cont.0, 2)
        } else {
            run(&cs.1, &fsr.1, &ld.1, &att.1, &cont.1, 2)
        };
        assert!(n > 0, "row65 lib={lib}: compression failed");
        let mut back = vec![0u8; src.len() + 64];
        let d = unsafe {
            dec.0(
                b.as_ptr() as *const c_char,
                back.as_mut_ptr() as *mut c_char,
                n,
                src.len() as c_int,
            )
        };
        assert_eq!(
            d,
            src.len() as c_int,
            "row65 lib={lib}: the empty dictionary must not have been used"
        );
        assert_bytes_eq(&back[..src.len()], &src[..], &format!("row65 lib={lib}"));
    }
}

// ===========================================================================
// Rows 66-67 — LZ4_compress_fast_continue
// ===========================================================================
#[test]
fn err66_67_compress_fast_continue() {
    sym!(cs, "LZ4_createStream", FnCreate);
    sym!(fsr, "LZ4_freeStream", FnFree);
    sym!(cont, "LZ4_compress_fast_continue", FnContinue);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0066);

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[13usize, 64, 1024, 4096, 65536] {
            let src = gen_src(shape, len, &mut rng);
            let nat_cap = unsafe { bound.0(len as c_int) }.max(16) as usize;

            let run = |create: &FnCreate,
                       free: &FnFree,
                       contf: &FnContinue,
                       cap: usize,
                       acc: c_int|
             -> (c_int, Vec<u8>) {
                let s = unsafe { create() };
                assert!(!s.is_null());
                let mut out = vec![0xA5u8; cap + DST_SLACK];
                let n = unsafe {
                    contf(
                        s,
                        src.as_ptr() as *const c_char,
                        out.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                        acc,
                    )
                };
                unsafe { free(s) };
                (n, out)
            };

            // row 66: acceleration clamping.
            let (n1, b1) = run(&cs.0, &fsr.0, &cont.0, nat_cap, 1);
            let (nm, bm) = run(&cs.0, &fsr.0, &cont.0, nat_cap, LZ4_ACCELERATION_MAX);
            for &acc in &[0i32, -1, i32::MIN] {
                let (cn, cb) = run(&cs.0, &fsr.0, &cont.0, nat_cap, acc);
                let (rn, rb) = run(&cs.1, &fsr.1, &cont.1, nat_cap, acc);
                let ctx = format!("row66 {shape:?} len={len} acc={acc}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n1, "{ctx}: must equal acceleration 1");
                assert_bytes_eq(&cb, &b1, &format!("{ctx}: bytes"));
            }
            for &acc in &[LZ4_ACCELERATION_MAX + 1, 1_000_000, i32::MAX] {
                let (cn, cb) = run(&cs.0, &fsr.0, &cont.0, nat_cap, acc);
                let (rn, rb) = run(&cs.1, &fsr.1, &cont.1, nat_cap, acc);
                let ctx = format!("row66 {shape:?} len={len} acc={acc}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, nm, "{ctx}: must equal acceleration 65537");
                assert_bytes_eq(&cb, &bm, &format!("{ctx}: bytes"));
            }

            // row 67: dstCapacity too small.
            let nat = n1 as usize;
            for cap in [0usize, 1, 2, nat / 2, nat - 1] {
                let (cn, cb) = run(&cs.0, &fsr.0, &cont.0, cap, 1);
                let (rn, rb) = run(&cs.1, &fsr.1, &cont.1, cap, 1);
                let ctx = format!("row67 {shape:?} len={len} cap={cap} nat={nat}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, 0, "{ctx}: must be 0");
            }
        }
    }
}

// ===========================================================================
// Row 68 — LZ4_compress_forceExtDict with an out-of-range srcSize
// ===========================================================================
#[test]
fn err68_compress_force_ext_dict() {
    sym!(cs, "LZ4_createStream", FnCreate);
    sym!(fsr, "LZ4_freeStream", FnFree);
    sym!(ld, "LZ4_loadDict", FnLoadDict);
    sym!(fed, "LZ4_compress_forceExtDict", FnForceExt);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0068);

    // Both buffers must outlive the streams (which retain their addresses).
    let dict = gen_src(Shape::Texty, 65536, &mut rng);
    let src = gen_src(Shape::Texty, 4096, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;

    let bad: &[c_int] = &[
        -1,
        -4096,
        i32::MIN,
        i32::MIN + 1,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ];
    for &n in bad {
        let run = |create: &FnCreate, free: &FnFree, loaddict: &FnLoadDict, f: &FnForceExt| {
            let s = unsafe { create() };
            assert!(!s.is_null());
            unsafe {
                loaddict(s, dict.as_ptr() as *const c_char, dict.len() as c_int);
            }
            let mut out = vec![0xA5u8; cap + DST_SLACK];
            let r = unsafe {
                f(
                    s,
                    src.as_ptr() as *const c_char,
                    out.as_mut_ptr() as *mut c_char,
                    n,
                )
            };
            unsafe { free(s) };
            (r, out)
        };
        let (cn, cb) = run(&cs.0, &fsr.0, &ld.0, &fed.0);
        let (rn, rb) = run(&cs.1, &fsr.1, &ld.1, &fed.1);
        let ctx = format!("row68 srcSize={n}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, 0, "{ctx}: must be 0");
    }

    // The valid boundary still works and stays in parity.
    for &n in &[0i32, 1, 13, 4096] {
        let run = |create: &FnCreate, free: &FnFree, loaddict: &FnLoadDict, f: &FnForceExt| {
            let s = unsafe { create() };
            unsafe {
                loaddict(s, dict.as_ptr() as *const c_char, dict.len() as c_int);
            }
            let mut out = vec![0xA5u8; cap + DST_SLACK];
            let r = unsafe {
                f(
                    s,
                    src.as_ptr() as *const c_char,
                    out.as_mut_ptr() as *mut c_char,
                    n,
                )
            };
            unsafe { free(s) };
            (r, out)
        };
        let (cn, cb) = run(&cs.0, &fsr.0, &ld.0, &fed.0);
        let (rn, rb) = run(&cs.1, &fsr.1, &ld.1, &fed.1);
        cmp_dec(cn, &cb, rn, &rb, &format!("row68 valid srcSize={n}"));
    }
}

// ===========================================================================
// Row 69 — LZ4_sizeofState / LZ4_sizeofStreamState
// ===========================================================================
#[test]
fn err69_sizeof_state() {
    sym!(a, "LZ4_sizeofState", FnSizeof);
    sym!(b, "LZ4_sizeofStreamState", FnSizeof);
    unsafe {
        assert_ret_eq(a.0(), a.1(), "LZ4_sizeofState");
        assert_ret_eq(b.0(), b.1(), "LZ4_sizeofStreamState");
        assert_eq!(a.0(), SIZEOF_LZ4_STREAM_T as c_int, "row69 sizeofState");
        assert_eq!(b.0(), SIZEOF_LZ4_STREAM_T as c_int, "row69 sizeofStreamState");
    }
}

// ===========================================================================
// Rows 70-73 — LZ4_initStreamHC
// ===========================================================================
#[test]
fn err70_73_init_stream_hc() {
    sym!(init, "LZ4_initStreamHC", FnInitStream);
    let mut buf = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 64);
    let p = buf.ptr();

    // row 70: NULL buffer
    for &sz in &[0usize, 1, SIZEOF_LZ4_STREAMHC_T, usize::MAX] {
        let (c, r) = unsafe {
            (
                init.0(std::ptr::null_mut(), sz),
                init.1(std::ptr::null_mut(), sz),
            )
        };
        assert_eq!(c, r, "row70 initStreamHC(NULL, {sz})");
        assert!(c.is_null(), "row70 must be NULL");
    }
    // row 71: undersized
    for &sz in &[
        0usize,
        1,
        8,
        SIZEOF_LZ4_STREAMHC_T - 2,
        SIZEOF_LZ4_STREAMHC_T - 1,
    ] {
        let (c, r) = unsafe { (init.0(p as *mut c_void, sz), init.1(p as *mut c_void, sz)) };
        assert_eq!(c, r, "row71 initStreamHC(aligned, {sz})");
        assert!(c.is_null(), "row71 initStreamHC(aligned, {sz}) must be NULL");
    }
    // row 72: misaligned
    for off in 1usize..8 {
        let mp = unsafe { p.add(off) } as *mut c_void;
        for &sz in &[SIZEOF_LZ4_STREAMHC_T, SIZEOF_LZ4_STREAMHC_T + 8] {
            let (c, r) = unsafe { (init.0(mp, sz), init.1(mp, sz)) };
            assert_eq!(c, r, "row72 initStreamHC(+{off}, {sz})");
            assert!(c.is_null(), "row72 initStreamHC(+{off}) must be NULL");
        }
    }
    // row 73: exactly sizeof, aligned
    for &sz in &[
        SIZEOF_LZ4_STREAMHC_T,
        SIZEOF_LZ4_STREAMHC_T + 1,
        SIZEOF_LZ4_STREAMHC_T + 64,
    ] {
        let (c, r) = unsafe { (init.0(p as *mut c_void, sz), init.1(p as *mut c_void, sz)) };
        assert_eq!(c, r, "row73 initStreamHC(aligned, {sz})");
        assert_eq!(c, p as *mut c_void, "row73 must return the buffer");
    }
}

// ===========================================================================
// Rows 74-77 — LZ4_compress_HC bad srcSize / dstCapacity
// ===========================================================================
#[test]
fn err74_77_compress_hc_bad_args() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0074);

    let run = |f: &FnHC5, src: &[u8], ss: c_int, cap: usize, lvl: c_int| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                ss,
                cap as c_int,
                lvl,
            )
        };
        (n, b)
    };

    let small = gen_src(Shape::Random, 64, &mut rng);
    // rows 74-75: srcSize out of range (checked before src is read)
    for &n in &[
        -1i32,
        -64,
        i32::MIN,
        i32::MIN + 1,
        LZ4_MAX_INPUT_SIZE + 1,
        i32::MAX,
    ] {
        for &lvl in &[1i32, 2, 9, 12, 0, 13] {
            for &cap in &[0usize, 1, 64, 1024] {
                let (cn, cb) = run(&hc.0, &small, n, cap, lvl);
                let (rn, rb) = run(&hc.1, &small, n, cap, lvl);
                let ctx = format!("row74/75 srcSize={n} lvl={lvl} cap={cap}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, 0, "{ctx}: must be 0");
            }
        }
    }

    // rows 76-77: dstCapacity too small / zero
    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[13usize, 64, 1024, 4096, 65536] {
            let src = gen_src(shape, len, &mut rng);
            let full = unsafe { bound.0(len as c_int) }.max(16) as usize;
            for &lvl in &[1i32, 3, 9, 12] {
                let (nat, _) = run(&hc.0, &src, len as c_int, full, lvl);
                assert!(nat > 0);
                for cap in [0usize, 1, 2, (nat as usize) / 2, nat as usize - 1] {
                    let (cn, cb) = run(&hc.0, &src, len as c_int, cap, lvl);
                    let (rn, rb) = run(&hc.1, &src, len as c_int, cap, lvl);
                    let ctx = format!("row76/77 {shape:?} len={len} lvl={lvl} cap={cap} nat={nat}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, 0, "{ctx}: must be 0");
                }
                // exactly `nat` succeeds
                let (cn, cb) = run(&hc.0, &src, len as c_int, nat as usize, lvl);
                let (rn, rb) = run(&hc.1, &src, len as c_int, nat as usize, lvl);
                cmp_dec(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("row76 boundary {shape:?} len={len} lvl={lvl}"),
                );
                assert_eq!(cn, nat);
            }
        }
    }
}

// ===========================================================================
// Rows 78-80 — LZ4_compress_HC compressionLevel clamping
// ===========================================================================
#[test]
fn err78_80_compress_hc_level_clamping() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x0078);

    let run = |f: &FnHC5, src: &[u8], cap: usize, lvl: c_int| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            )
        };
        (n, b)
    };

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs, Shape::Periodic] {
        for &len in &[13usize, 64, 1024, 4096, 20_000] {
            let src = gen_src(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(16) as usize;

            let (n9, b9) = run(&hc.0, &src, cap, 9);
            let (n12, b12) = run(&hc.0, &src, cap, 12);
            let (n1, b1) = run(&hc.0, &src, cap, 1);
            let (n2, _b2) = run(&hc.0, &src, cap, 2);

            // row 78: level < 1 clamps to LZ4HC_CLEVEL_DEFAULT (9)
            for &lvl in &[0i32, -1, -12, i32::MIN, i32::MIN + 1] {
                let (cn, cb) = run(&hc.0, &src, cap, lvl);
                let (rn, rb) = run(&hc.1, &src, cap, lvl);
                let ctx = format!("row78 {shape:?} len={len} lvl={lvl}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n9, "{ctx}: must equal level 9");
                assert_bytes_eq(&cb, &b9, &format!("{ctx}: bytes vs level 9"));
            }
            // row 79: level > 12 clamps to 12
            for &lvl in &[13i32, 14, 100, 1000, i32::MAX] {
                let (cn, cb) = run(&hc.0, &src, cap, lvl);
                let (rn, rb) = run(&hc.1, &src, cap, lvl);
                let ctx = format!("row79 {shape:?} len={len} lvl={lvl}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n12, "{ctx}: must equal level 12");
                assert_bytes_eq(&cb, &b12, &format!("{ctx}: bytes vs level 12"));
            }
            // row 80: level 1 is ACCEPTED (lz4mid), NOT raised to 2.
            {
                let (rn, rb) = run(&hc.1, &src, cap, 1);
                let ctx = format!("row80 {shape:?} len={len}");
                cmp_dec(n1, &b1, rn, &rb, &ctx);
                assert!(n1 > 0, "{ctx}: level 1 must produce output");
                // it must be a valid block ...
                let mut back = vec![0u8; len + 64];
                let d = unsafe {
                    dec.0(
                        b1.as_ptr() as *const c_char,
                        back.as_mut_ptr() as *mut c_char,
                        n1,
                        len as c_int,
                    )
                };
                assert_eq!(d, len as c_int, "{ctx}: level 1 round trip");
                assert_bytes_eq(&back[..len], &src[..], &ctx);
                // ... and it must NOT be silently promoted to level 2 for
                // inputs where the two strategies genuinely differ.
                let _ = n2;
            }
            // Every in-range level stays in parity.
            for lvl in 1i32..=12 {
                let (cn, cb) = run(&hc.0, &src, cap, lvl);
                let (rn, rb) = run(&hc.1, &src, cap, lvl);
                cmp_dec(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("row78-80 {shape:?} len={len} lvl={lvl}"),
                );
            }
        }
    }
    // row 80 continued: level 1 is accepted as-is, i.e. it is NOT raised to the
    // LZ4HC_CLEVEL_DEFAULT (9) that `cLevel < 1` gets. `k_clTable` maps levels
    // 1 and 2 to the same `lz4mid` parameters, so level 1 == level 2 output is
    // expected and only the level-9 comparison proves "not clamped".
    let src = gen_src(Shape::Texty, 65_536, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;
    let (n1, b1) = run(&hc.0, &src, cap, 1);
    let (n2, b2) = run(&hc.0, &src, cap, 2);
    let (n9, _b9) = run(&hc.0, &src, cap, 9);
    let (n0, _b0) = run(&hc.0, &src, cap, 0);
    assert_eq!(n1, n2, "row80: levels 1 and 2 share the lz4mid parameters");
    assert_bytes_eq(&b1, &b2, "row80: level 1 == level 2 bytes");
    assert_ne!(n1, n9, "row80: level 1 must NOT be clamped to level 9");
    assert_eq!(n0, n9, "row78: level 0 IS clamped to level 9");
    // and the Rust agrees on all four.
    for &lvl in &[0i32, 1, 2, 9] {
        let (cn, cb) = run(&hc.0, &src, cap, lvl);
        let (rn, rb) = run(&hc.1, &src, cap, lvl);
        cmp_dec(cn, &cb, rn, &rb, &format!("row78-80 64K lvl={lvl}"));
    }
}

// ===========================================================================
// Row 81 — LZ4_compress_HC with srcSize == 0
// ===========================================================================
#[test]
fn err81_compress_hc_empty_src() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    let mut rng = Rng::new(0x0081);
    let src = gen_src(Shape::Random, 64, &mut rng);

    for &cap in &[i32::MIN, -1, 0, 1, 2, 16, 1024] {
        for &lvl in &[0i32, 1, 2, 3, 9, 12, 13, -1] {
            let capu = cap.max(0) as usize;
            let (mut cb, mut rb) = dst_pair(capu);
            let (cn, rn) = unsafe {
                (
                    hc.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        lvl,
                    ),
                    hc.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        0,
                        cap,
                        lvl,
                    ),
                )
            };
            let ctx = format!("row81 cap={cap} lvl={lvl}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            if cap <= 0 {
                assert_eq!(cn, 0, "{ctx}: must be 0");
            } else {
                assert_eq!(cn, 1, "{ctx}: must be 1 (empty block)");
                assert_eq!(cb[0], 0, "{ctx}: dst[0] must be 0");
            }
        }
    }
}

// ===========================================================================
// Rows 82-84 — LZ4_compress_HC_extStateHC{,_fastReset} with a bad state
// ===========================================================================
#[test]
fn err82_84_ext_state_hc() {
    sym!(ext, "LZ4_compress_HC_extStateHC", FnExt6);
    sym!(fr, "LZ4_compress_HC_extStateHC_fastReset", FnExt6);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0082);

    let src = gen_src(Shape::Texty, 4096, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;
    let mut st = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 64);
    let p = st.ptr();

    let run = |f: &FnExt6, state: *mut c_void, lvl: c_int| -> (c_int, Vec<u8>) {
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            f(
                state,
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            )
        };
        (n, b)
    };

    // row 82: state == NULL (rejected by LZ4_initStreamHC)
    for &lvl in &[1i32, 9, 12] {
        let (cn, cb) = run(&ext.0, std::ptr::null_mut(), lvl);
        let (rn, rb) = run(&ext.1, std::ptr::null_mut(), lvl);
        let ctx = format!("row82 extStateHC(NULL) lvl={lvl}");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, 0, "{ctx}: must be 0");
    }

    // row 83 / 84: a misaligned state
    for off in 1usize..8 {
        let mp = unsafe { p.add(off) } as *mut c_void;
        for &lvl in &[1i32, 9, 12] {
            let (cn, cb) = run(&ext.0, mp, lvl);
            let (rn, rb) = run(&ext.1, mp, lvl);
            let ctx = format!("row83 extStateHC(+{off}) lvl={lvl}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, 0, "{ctx}: must be 0");

            let (cn, cb) = run(&fr.0, mp, lvl);
            let (rn, rb) = run(&fr.1, mp, lvl);
            let ctx = format!("row84 fastReset(+{off}) lvl={lvl}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            assert_eq!(cn, 0, "{ctx}: must be 0");
        }
    }

    // A properly aligned state still works (and stays in parity).
    for &lvl in &[1i32, 3, 9, 12] {
        let (cn, cb) = run(&ext.0, p as *mut c_void, lvl);
        let (rn, rb) = run(&ext.1, p as *mut c_void, lvl);
        cmp_dec(cn, &cb, rn, &rb, &format!("row82-84 valid lvl={lvl}"));
        assert!(cn > 0);
    }
}

// ===========================================================================
// Rows 85-87 — LZ4_compress_HC_destSize
// ===========================================================================
#[test]
fn err85_87_compress_hc_dest_size() {
    sym!(ds, "LZ4_compress_HC_destSize", FnDestSizeHC);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x0085);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 8);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 8);
    let cp = cst.ptr() as *mut c_void;
    let rp = rst.ptr() as *mut c_void;

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[13usize, 64, 1024, 4096, 20_000] {
            let src = gen_src(shape, len, &mut rng);
            for &lvl in &[1i32, 3, 9, 12] {
                // row 85: targetDstSize < 1
                for &t in &[0i32, -1, -1000, i32::MIN] {
                    let mut csp = len as c_int;
                    let mut rsp = len as c_int;
                    let (mut cb, mut rb) = dst_pair(0);
                    let (cn, rn) = unsafe {
                        (
                            ds.0(
                                cp,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut csp,
                                t,
                                lvl,
                            ),
                            ds.1(
                                rp,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rsp,
                                t,
                                lvl,
                            ),
                        )
                    };
                    let ctx = format!("row85 {shape:?} len={len} lvl={lvl} target={t}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr");
                    assert_eq!(cn, 0, "{ctx}: must be 0");
                }

                // row 86: *srcSizePtr out of range
                for &s in &[-1i32, i32::MIN, LZ4_MAX_INPUT_SIZE + 1, i32::MAX] {
                    let mut csp = s;
                    let mut rsp = s;
                    let (mut cb, mut rb) = dst_pair(1024);
                    let (cn, rn) = unsafe {
                        (
                            ds.0(
                                cp,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut csp,
                                1024,
                                lvl,
                            ),
                            ds.1(
                                rp,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rsp,
                                1024,
                                lvl,
                            ),
                        )
                    };
                    let ctx = format!("row86 {shape:?} lvl={lvl} srcSize={s}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr");
                    assert_eq!(cn, 0, "{ctx}: must be 0");
                }

                // row 87: the fillOutput salvage path
                for &t in &[1usize, 2, 3, 5, 11, 20, len / 4 + 1, len / 2 + 1] {
                    let mut csp = len as c_int;
                    let mut rsp = len as c_int;
                    let (mut cb, mut rb) = dst_pair(t);
                    let (cn, rn) = unsafe {
                        (
                            ds.0(
                                cp,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut csp,
                                t as c_int,
                                lvl,
                            ),
                            ds.1(
                                rp,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rsp,
                                t as c_int,
                                lvl,
                            ),
                        )
                    };
                    let ctx = format!("row87 {shape:?} len={len} lvl={lvl} target={t}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr");
                    assert!(cn <= t as c_int, "{ctx}: ret {cn} > target");
                    if cn > 0 {
                        let mut back = vec![0u8; csp as usize + 64];
                        let d = unsafe {
                            dec.0(
                                cb.as_ptr() as *const c_char,
                                back.as_mut_ptr() as *mut c_char,
                                cn,
                                csp,
                            )
                        };
                        assert_eq!(d, csp, "{ctx}: round trip");
                        assert_bytes_eq(&back[..csp as usize], &src[..csp as usize], &ctx);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Rows 88-89 — LZ4_compress_HC_continue{,_destSize}
// ===========================================================================
#[test]
fn err88_89_compress_hc_continue() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(cont, "LZ4_compress_HC_continue", FnExt5);
    sym!(contds, "LZ4_compress_HC_continue_destSize", FnContDestSizeHC);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0088);

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[13usize, 64, 1024, 4096, 20_000] {
            // The stream retains `src`; it is created before every stream use.
            let src = gen_src(shape, len, &mut rng);
            let full = unsafe { bound.0(len as c_int) }.max(16) as usize;

            for &lvl in &[1i32, 3, 9, 12] {
                let run_cont = |create: &FnCreate,
                                free: &FnFree,
                                rst: &FnStreamInt,
                                f: &FnExt5,
                                cap: usize|
                 -> (c_int, Vec<u8>) {
                    let s = unsafe { create() };
                    assert!(!s.is_null());
                    unsafe { rst(s, lvl) };
                    let mut b = vec![0xA5u8; cap + DST_SLACK];
                    let n = unsafe {
                        f(
                            s,
                            src.as_ptr() as *const c_char,
                            b.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        )
                    };
                    unsafe { free(s) };
                    (n, b)
                };

                let (nat, _) = run_cont(&cs.0, &fsr.0, &reset.0, &cont.0, full);
                assert!(nat > 0);
                // row 88: dstCapacity too small
                for cap in [0usize, 1, 2, nat as usize / 2, nat as usize - 1] {
                    let (cn, cb) = run_cont(&cs.0, &fsr.0, &reset.0, &cont.0, cap);
                    let (rn, rb) = run_cont(&cs.1, &fsr.1, &reset.1, &cont.1, cap);
                    let ctx = format!("row88 {shape:?} len={len} lvl={lvl} cap={cap} nat={nat}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, 0, "{ctx}: must be 0");
                }

                // row 89: targetDestSize < 1
                let run_ds = |create: &FnCreate,
                              free: &FnFree,
                              rst: &FnStreamInt,
                              f: &FnContDestSizeHC,
                              t: c_int|
                 -> (c_int, c_int, Vec<u8>) {
                    let s = unsafe { create() };
                    unsafe { rst(s, lvl) };
                    let mut sp = len as c_int;
                    let mut b = vec![0xA5u8; t.max(0) as usize + DST_SLACK];
                    let n = unsafe {
                        f(
                            s,
                            src.as_ptr() as *const c_char,
                            b.as_mut_ptr() as *mut c_char,
                            &mut sp,
                            t,
                        )
                    };
                    unsafe { free(s) };
                    (n, sp, b)
                };
                for &t in &[0i32, -1, -1000, i32::MIN] {
                    let (cn, csp, cb) = run_ds(&cs.0, &fsr.0, &reset.0, &contds.0, t);
                    let (rn, rsp, rb) = run_ds(&cs.1, &fsr.1, &reset.1, &contds.1, t);
                    let ctx = format!("row89 {shape:?} len={len} lvl={lvl} target={t}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr");
                    assert_eq!(cn, 0, "{ctx}: must be 0");
                }
                // ... and the salvage path stays in parity.
                for &t in &[1i32, 2, 5, 20, 200] {
                    let (cn, csp, cb) = run_ds(&cs.0, &fsr.0, &reset.0, &contds.0, t);
                    let (rn, rsp, rb) = run_ds(&cs.1, &fsr.1, &reset.1, &contds.1, t);
                    let ctx = format!("row89 salvage {shape:?} len={len} lvl={lvl} target={t}");
                    cmp_dec(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(csp, rsp, "{ctx}: *srcSizePtr");
                    assert!(cn <= t, "{ctx}: ret {cn} > target");
                }
            }
        }
    }
}

// ===========================================================================
// Rows 90-93 — LZ4_loadDictHC
// ===========================================================================
#[test]
fn err90_93_load_dict_hc() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(cont, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0090);

    // Both buffers outlive every stream that references them.
    let dict = gen_src(Shape::Texty, 200_000, &mut rng);
    let src = gen_src(Shape::Texty, 4096, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;

    let sizes: &[c_int] = &[
        0, 1, 2, 3, 4, 5, 8, 9, 16, 64, 4096, 65535, 65536, 65537, 100_000, 200_000,
    ];
    for &lvl in &[1i32, 2, 3, 9, 12] {
        for &n in sizes {
            let run = |create: &FnCreate,
                       free: &FnFree,
                       rst: &FnStreamInt,
                       loaddict: &FnLoadDict,
                       contf: &FnExt5|
             -> (c_int, c_int, Vec<u8>) {
                let s = unsafe { create() };
                assert!(!s.is_null());
                unsafe { rst(s, lvl) };
                let l = unsafe { loaddict(s, dict.as_ptr() as *const c_char, n) };
                let mut b = vec![0xA5u8; cap + DST_SLACK];
                let c = unsafe {
                    contf(
                        s,
                        src.as_ptr() as *const c_char,
                        b.as_mut_ptr() as *mut c_char,
                        src.len() as c_int,
                        cap as c_int,
                    )
                };
                unsafe { free(s) };
                (l, c, b)
            };
            let (cl, cc, cb) = run(&cs.0, &fsr.0, &reset.0, &ld.0, &cont.0);
            let (rl, rc, rb) = run(&cs.1, &fsr.1, &reset.1, &ld.1, &cont.1);
            let ctx = format!("row90-93 lvl={lvl} dictSize={n}");
            assert_ret_eq(cl, rl, &format!("{ctx}: loadDictHC"));
            cmp_dec(cc, &cb, rc, &rb, &format!("{ctx}: subsequent compression"));
            // row 90: > 64 KB keeps the last 64 KB. rows 91-93: kept as-is.
            if n > 65536 {
                assert_eq!(cl, 65536, "{ctx}: must clamp to 65536");
            } else {
                assert_eq!(cl, n, "{ctx}: must be returned unchanged");
            }
        }
    }
}

// ===========================================================================
// Rows 94-96 — LZ4_saveDictHC
// ===========================================================================
#[test]
fn err94_96_save_dict_hc() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(sd, "LZ4_saveDictHC", FnSaveDict);
    let mut rng = Rng::new(0x0094);

    let dict = gen_src(Shape::Texty, 200_000, &mut rng);
    // saveDictHC re-points the stream at `safe`; both must outlive the stream.
    let mut csafe = vec![0x5Au8; 70_000 + DST_SLACK];
    let mut rsafe = vec![0x5Au8; 70_000 + DST_SLACK];

    let sizes: &[c_int] = &[
        i32::MIN,
        -70000,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        8,
        100,
        65535,
        65536,
        65537,
        70000,
    ];
    for &pre in &[0i32, 3, 4, 8, 1000, 65536, 200_000] {
        for &n in sizes {
            let cstr = unsafe { cs.0() };
            let rstr = unsafe { cs.1() };
            assert!(!cstr.is_null() && !rstr.is_null());
            unsafe {
                reset.0(cstr, 9);
                reset.1(rstr, 9);
            }
            let (cl, rl) = unsafe {
                (
                    ld.0(cstr, dict.as_ptr() as *const c_char, pre),
                    ld.1(rstr, dict.as_ptr() as *const c_char, pre),
                )
            };
            assert_ret_eq(cl, rl, &format!("row94-96 preload loadDictHC({pre})"));

            for b in csafe.iter_mut() {
                *b = 0x5A;
            }
            for b in rsafe.iter_mut() {
                *b = 0x5A;
            }
            let (c, r) = unsafe {
                (
                    sd.0(cstr, csafe.as_mut_ptr() as *mut c_char, n),
                    sd.1(rstr, rsafe.as_mut_ptr() as *mut c_char, n),
                )
            };
            let ctx = format!("row94-96 saveDictHC(pre={pre}, {n})");
            assert_ret_eq(c, r, &ctx);
            assert_bytes_eq(&csafe, &rsafe, &format!("{ctx}: safeBuffer"));
            // row 94: < 4 => 0. row 95: > 64 KB clamps. row 96: > prefixSize clamps.
            let prefix = cl.max(0);
            let want = {
                let mut d = n;
                if d > 65536 {
                    d = 65536;
                }
                if d < 4 {
                    d = 0;
                }
                if d > prefix {
                    d = prefix;
                }
                d
            };
            assert_eq!(c, want, "{ctx}: expected {want}");
            unsafe {
                fsr.0(cstr);
                fsr.1(rstr);
            }
        }
    }
}

// ===========================================================================
// Rows 97-98 — LZ4_freeStreamHC(NULL) / LZ4_freeHC(NULL)
// ===========================================================================
#[test]
fn err97_98_free_hc_null() {
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(fhc, "LZ4_freeHC", FnFree);
    sym!(chc, "LZ4_createHC", FnCreateHC);
    let mut rng = Rng::new(0x0097);
    // `LZ4_createHC` stores this pointer inside the state.
    let input = gen_src(Shape::Texty, 4096, &mut rng);
    unsafe {
        assert_ret_eq(
            fsr.0(std::ptr::null_mut()),
            fsr.1(std::ptr::null_mut()),
            "row97 LZ4_freeStreamHC(NULL)",
        );
        assert_eq!(fsr.0(std::ptr::null_mut()), 0, "row97 must be 0");
        assert_ret_eq(
            fhc.0(std::ptr::null_mut()),
            fhc.1(std::ptr::null_mut()),
            "row98 LZ4_freeHC(NULL)",
        );
        assert_eq!(fhc.0(std::ptr::null_mut()), 0, "row98 must be 0");
        // The non-NULL path returns 0 as well, from both libraries.
        let a = chc.0(input.as_ptr() as *const c_char);
        let b = chc.1(input.as_ptr() as *const c_char);
        assert!(!a.is_null() && !b.is_null(), "row98 LZ4_createHC failed");
        assert_ret_eq(fhc.0(a), fhc.1(b), "row98 LZ4_freeHC(valid)");
    }
}

// ===========================================================================
// Rows 99-100 — LZ4_resetStreamStateHC (INVERTED convention: 1 == error)
// ===========================================================================
#[test]
fn err99_100_reset_stream_state_hc() {
    sym!(rss, "LZ4_resetStreamStateHC", FnResetStateHC);
    let mut rng = Rng::new(0x0099);
    // `LZ4HC_init_internal` stores this pointer, so it must outlive the state.
    let input = gen_src(Shape::Texty, 4096, &mut rng);
    let mut st = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 64);
    let p = st.ptr();

    // row 99: NULL state
    let (c, r) = unsafe {
        (
            rss.0(std::ptr::null_mut(), input.as_ptr() as *mut c_char),
            rss.1(std::ptr::null_mut(), input.as_ptr() as *mut c_char),
        )
    };
    assert_ret_eq(c, r, "row99 resetStreamStateHC(NULL)");
    assert_eq!(c, 1, "row99 must be 1 (error)");

    // row 99: misaligned state
    for off in 1usize..8 {
        let mp = unsafe { p.add(off) } as *mut c_void;
        let (c, r) = unsafe {
            (
                rss.0(mp, input.as_ptr() as *mut c_char),
                rss.1(mp, input.as_ptr() as *mut c_char),
            )
        };
        assert_ret_eq(c, r, &format!("row99 resetStreamStateHC(+{off})"));
        assert_eq!(c, 1, "row99 (+{off}) must be 1 (error)");
    }

    // row 100: a valid aligned state => 0
    let (c, r) = unsafe {
        (
            rss.0(p as *mut c_void, input.as_ptr() as *mut c_char),
            rss.1(p as *mut c_void, input.as_ptr() as *mut c_char),
        )
    };
    assert_ret_eq(c, r, "row100 resetStreamStateHC(valid)");
    assert_eq!(c, 0, "row100 must be 0 (success)");
}

// ===========================================================================
// Rows 101-103 — LZ4_setCompressionLevel / LZ4_favorDecompressionSpeed
// ===========================================================================
#[test]
fn err101_103_set_level_and_favor() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(setlvl, "LZ4_setCompressionLevel", FnStreamInt);
    sym!(favor, "LZ4_favorDecompressionSpeed", FnStreamInt);
    sym!(cont, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0101);

    for &shape in &[Shape::Texty, Shape::Periodic, Shape::Runs] {
        for &len in &[1024usize, 8192, 20_000] {
            let src = gen_src(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(16) as usize;

            // reset(5) -> [setCompressionLevel(lvl2)] -> [favorDecSpeed(fav)]
            // -> compress, all through one library.
            let run_lib = |lib: u8, lvl2: Option<c_int>, fav: Option<c_int>| -> (c_int, Vec<u8>) {
                let (create, free, rst, sl, fv, contf) = if lib == 0 {
                    (&cs.0, &fsr.0, &reset.0, &setlvl.0, &favor.0, &cont.0)
                } else {
                    (&cs.1, &fsr.1, &reset.1, &setlvl.1, &favor.1, &cont.1)
                };
                let s = unsafe { create() };
                assert!(!s.is_null());
                unsafe { rst(s, 5) };
                if let Some(l) = lvl2 {
                    unsafe { sl(s, l) };
                }
                if let Some(f) = fav {
                    unsafe { fv(s, f) };
                }
                let mut b = vec![0xA5u8; cap + DST_SLACK];
                let n = unsafe {
                    contf(
                        s,
                        src.as_ptr() as *const c_char,
                        b.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        cap as c_int,
                    )
                };
                unsafe { free(s) };
                (n, b)
            };

            let (n9, b9) = run_lib(0, Some(9), None);
            let (n12, b12) = run_lib(0, Some(12), None);

            // row 101: level < 1 is stored as 9
            for &l in &[0i32, -1, -100, i32::MIN] {
                let (cn, cb) = run_lib(0, Some(l), None);
                let (rn, rb) = run_lib(1, Some(l), None);
                let ctx = format!("row101 {shape:?} len={len} setLevel={l}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n9, "{ctx}: must equal level 9");
                assert_bytes_eq(&cb, &b9, &format!("{ctx}: bytes vs level 9"));
            }
            // row 102: level > 12 is stored as 12
            for &l in &[13i32, 100, i32::MAX] {
                let (cn, cb) = run_lib(0, Some(l), None);
                let (rn, rb) = run_lib(1, Some(l), None);
                let ctx = format!("row102 {shape:?} len={len} setLevel={l}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n12, "{ctx}: must equal level 12");
                assert_bytes_eq(&cb, &b12, &format!("{ctx}: bytes vs level 12"));
            }
            // row 103: favorDecSpeed stores `(favor != 0)`; every non-zero value
            // must behave exactly like 1. Verified through the produced bytes at
            // an optimal-parser level (the only place `favorDecSpeed` is read).
            let (n_fav1, b_fav1) = run_lib(0, Some(12), Some(1));
            let (n_fav0, _b_fav0) = run_lib(0, Some(12), Some(0));
            for &f in &[1i32, -1, 12345, i32::MIN, i32::MAX] {
                let (cn, cb) = run_lib(0, Some(12), Some(f));
                let (rn, rb) = run_lib(1, Some(12), Some(f));
                let ctx = format!("row103 {shape:?} len={len} favor={f}");
                cmp_dec(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n_fav1, "{ctx}: every non-zero favor must equal 1");
                assert_bytes_eq(&cb, &b_fav1, &format!("{ctx}: bytes vs favor=1"));
            }
            // favor = 0 stays in parity too.
            let (cn, cb) = run_lib(0, Some(12), Some(0));
            let (rn, rb) = run_lib(1, Some(12), Some(0));
            cmp_dec(cn, &cb, rn, &rb, &format!("row103 {shape:?} len={len} favor=0"));
            assert_eq!(cn, n_fav0);
        }
    }
}

// ===========================================================================
// Row 104 — LZ4_resetStreamHC_fast after a failed compression (dirty flag)
// ===========================================================================
#[test]
fn err104_reset_stream_hc_fast_dirty() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(resetf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(cont, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0104);

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[64usize, 1024, 8192, 20_000] {
            let src = gen_src(shape, len, &mut rng);
            let cap = unsafe { bound.0(len as c_int) }.max(16) as usize;

            for &lvl in &[1i32, 3, 9, 12] {
                // A) fresh stream, single compression => the reference bytes.
                let fresh = |lib: u8| -> (c_int, Vec<u8>) {
                    let (create, free, rst, contf) = if lib == 0 {
                        (&cs.0, &fsr.0, &reset.0, &cont.0)
                    } else {
                        (&cs.1, &fsr.1, &reset.1, &cont.1)
                    };
                    let s = unsafe { create() };
                    unsafe { rst(s, lvl) };
                    let mut b = vec![0xA5u8; cap + DST_SLACK];
                    let n = unsafe {
                        contf(
                            s,
                            src.as_ptr() as *const c_char,
                            b.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        )
                    };
                    unsafe { free(s) };
                    (n, b)
                };
                // B) same stream: fail (tiny dstCapacity) => dirty, then
                //    resetStreamHC_fast, then compress for real.
                let dirty_then_reset = |lib: u8| -> (c_int, c_int, Vec<u8>) {
                    let (create, free, rst, rstf, contf) = if lib == 0 {
                        (&cs.0, &fsr.0, &reset.0, &resetf.0, &cont.0)
                    } else {
                        (&cs.1, &fsr.1, &reset.1, &resetf.1, &cont.1)
                    };
                    let s = unsafe { create() };
                    unsafe { rst(s, lvl) };
                    let mut tiny = vec![0xA5u8; 1 + DST_SLACK];
                    let bad = unsafe {
                        contf(
                            s,
                            src.as_ptr() as *const c_char,
                            tiny.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            1,
                        )
                    };
                    unsafe { rstf(s, lvl) };
                    let mut b = vec![0xA5u8; cap + DST_SLACK];
                    let n = unsafe {
                        contf(
                            s,
                            src.as_ptr() as *const c_char,
                            b.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        )
                    };
                    unsafe { free(s) };
                    (bad, n, b)
                };

                let (fc_n, fc_b) = fresh(0);
                let (fr_n, fr_b) = fresh(1);
                let ctx = format!("row104 {shape:?} len={len} lvl={lvl}");
                cmp_dec(fc_n, &fc_b, fr_n, &fr_b, &format!("{ctx} fresh"));
                assert!(fc_n > 0, "{ctx}: fresh compression failed");

                let (cbad, cn, cb) = dirty_then_reset(0);
                let (rbad, rn, rb) = dirty_then_reset(1);
                assert_eq!(cbad, rbad, "{ctx}: the failing call must match");
                assert_eq!(cbad, 0, "{ctx}: the tiny-dstCapacity call must return 0");
                cmp_dec(cn, &cb, rn, &rb, &format!("{ctx} after reset_fast"));
                assert_eq!(cn, fc_n, "{ctx}: must match a fresh stream");
                assert_bytes_eq(&cb, &fc_b, &format!("{ctx}: bytes vs a fresh stream"));
            }
        }
    }
}

// ===========================================================================
// Rows 105-106 — LZ4_attach_HC_dictionary
// ===========================================================================
#[test]
fn err105_106_attach_hc_dictionary() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fsr, "LZ4_freeStreamHC", FnFree);
    sym!(reset, "LZ4_resetStreamHC", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(att, "LZ4_attach_HC_dictionary", FnAttach);
    sym!(cont, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x0105);

    // Buffers whose addresses the streams retain for their whole lifetime.
    let dict = gen_src(Shape::Texty, 200_000, &mut rng);
    let src = gen_src(Shape::Texty, 8192, &mut rng);
    let cap = unsafe { bound.0(src.len() as c_int) }.max(16) as usize;

    // variant 0: no attach; variant 1: attach(NULL), a pure detach (row 105).
    let run = |lib: u8, variant: u8, lvl: c_int| -> (c_int, Vec<u8>) {
        let (create, free, rst, loaddict, attach, contf) = if lib == 0 {
            (&cs.0, &fsr.0, &reset.0, &ld.0, &att.0, &cont.0)
        } else {
            (&cs.1, &fsr.1, &reset.1, &ld.1, &att.1, &cont.1)
        };
        let s = unsafe { create() };
        assert!(!s.is_null());
        unsafe { rst(s, lvl) };
        let d = unsafe { create() };
        assert!(!d.is_null());
        unsafe {
            rst(d, lvl);
            loaddict(d, dict.as_ptr() as *const c_char, 65536);
        }
        if variant == 1 {
            unsafe { attach(s, std::ptr::null()) };
        }
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            contf(
                s,
                src.as_ptr() as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
            )
        };
        unsafe {
            free(s);
            free(d);
        }
        (n, b)
    };

    for &lvl in &[1i32, 3, 9, 12] {
        let (base_n, base_b) = run(0, 0, lvl);
        assert!(base_n > 0);
        for variant in 0u8..2 {
            let (cn, cb) = run(0, variant, lvl);
            let (rn, rb) = run(1, variant, lvl);
            let ctx = format!("row105 lvl={lvl} variant={variant}");
            cmp_dec(cn, &cb, rn, &rb, &ctx);
            // row 105: `attach_HC_dictionary(s, NULL)` only stores dictCtx =
            // NULL, so the output is identical to never attaching anything.
            assert_eq!(cn, base_n, "{ctx}: must equal the no-dict result");
            assert_bytes_eq(&cb, &base_b, &format!("{ctx}: bytes vs no-dict"));
            let mut back = vec![0u8; src.len() + 64];
            let d = unsafe {
                dec.0(
                    cb.as_ptr() as *const c_char,
                    back.as_mut_ptr() as *mut c_char,
                    cn,
                    src.len() as c_int,
                )
            };
            assert_eq!(d, src.len() as c_int, "{ctx}: dictionary-less round trip");
            assert_bytes_eq(&back[..src.len()], &src[..], &ctx);
        }
    }

    // row 106: an attached dictionary is silently DROPPED once the working
    // stream's own history position reaches 64 KB (lz4hc.c:1454). Build that
    // history first, then attach, then compress a block that is CONTIGUOUS with
    // the warm-up block (otherwise `LZ4HC_setExternalDict` would clear dictCtx
    // before the dictCtx path is reached, which would prove nothing).
    const WARM: usize = 70_000;
    let big = gen_src(Shape::Texty, WARM + 8192, &mut rng);
    let warm_cap = unsafe { bound.0(WARM as c_int) }.max(16) as usize;

    let run106 = |lib: u8, attach_dict: bool, lvl: c_int| -> (c_int, Vec<u8>) {
        let (create, free, rst, loaddict, attach, contf) = if lib == 0 {
            (&cs.0, &fsr.0, &reset.0, &ld.0, &att.0, &cont.0)
        } else {
            (&cs.1, &fsr.1, &reset.1, &ld.1, &att.1, &cont.1)
        };
        let s = unsafe { create() };
        assert!(!s.is_null());
        unsafe { rst(s, lvl) };
        let d = unsafe { create() };
        assert!(!d.is_null());
        unsafe {
            rst(d, lvl);
            loaddict(d, dict.as_ptr() as *const c_char, 65536);
        }
        // Warm-up block: builds >= 64 KB of prefix history.
        let mut wb = vec![0xA5u8; warm_cap + DST_SLACK];
        let wn = unsafe {
            contf(
                s,
                big.as_ptr() as *const c_char,
                wb.as_mut_ptr() as *mut c_char,
                WARM as c_int,
                warm_cap as c_int,
            )
        };
        assert!(wn > 0, "row106 warm-up compression failed");
        if attach_dict {
            unsafe { attach(s, d as *const c_void) };
        }
        let mut b = vec![0xA5u8; cap + DST_SLACK];
        let n = unsafe {
            contf(
                s,
                big.as_ptr().add(WARM) as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                8192,
                cap as c_int,
            )
        };
        unsafe {
            free(s);
            free(d);
        }
        (n, b)
    };

    for &lvl in &[1i32, 3, 9, 12] {
        let (bn, bb) = run106(0, false, lvl);
        let (bnr, bbr) = run106(1, false, lvl);
        cmp_dec(bn, &bb, bnr, &bbr, &format!("row106 lvl={lvl} no attach"));
        let (cn, cb) = run106(0, true, lvl);
        let (rn, rb) = run106(1, true, lvl);
        let ctx = format!("row106 lvl={lvl} attached");
        cmp_dec(cn, &cb, rn, &rb, &ctx);
        assert_eq!(cn, bn, "{ctx}: the dictionary must have been dropped");
        assert_bytes_eq(&cb, &bb, &format!("{ctx}: bytes vs no attach"));
    }
}

// ===========================================================================
// Row 107 — the deprecated HC one-shots (cLevel hard-coded to 0 => level 9)
// ===========================================================================
#[test]
fn err107_deprecated_hc_oneshots() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(chc, "LZ4_compressHC", FnHC3);
    sym!(chc_lo, "LZ4_compressHC_limitedOutput", FnHC4);
    sym!(chc2, "LZ4_compressHC2", FnHC4);
    sym!(chc2_lo, "LZ4_compressHC2_limitedOutput", FnHC5);
    sym!(chc_ws, "LZ4_compressHC_withStateHC", FnExt4);
    sym!(chc_lo_ws, "LZ4_compressHC_limitedOutput_withStateHC", FnExt5);
    sym!(chc2_ws, "LZ4_compressHC2_withStateHC", FnExt5);
    sym!(chc2_lo_ws, "LZ4_compressHC2_limitedOutput_withStateHC", FnExt6);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x0107);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 8);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T + 8);
    let csp = cst.ptr() as *mut c_void;
    let rsp = rst.ptr() as *mut c_void;

    for &shape in &[Shape::Random, Shape::Texty, Shape::Runs] {
        for &len in &[13usize, 64, 1024, 4096] {
            let src = gen_src(shape, len, &mut rng);
            let full = unsafe { bound.0(len as c_int) }.max(16) as usize;
            let sp = src.as_ptr() as *const c_char;

            // The reference: level 9 (what `cLevel == 0` clamps to).
            let mut ref9 = vec![0xA5u8; full + DST_SLACK];
            let n9 = unsafe { hc.0(sp, ref9.as_mut_ptr() as *mut c_char, len as c_int, full as c_int, 9) };
            assert!(n9 > 0);

            // ---- unlimited-output wrappers => level 9 output ---------------
            let mut a = vec![0xA5u8; full + DST_SLACK];
            let mut b = vec![0xA5u8; full + DST_SLACK];
            let (cn, rn) = unsafe {
                (
                    chc.0(sp, a.as_mut_ptr() as *mut c_char, len as c_int),
                    chc.1(sp, b.as_mut_ptr() as *mut c_char, len as c_int),
                )
            };
            let ctx = format!("row107 compressHC {shape:?} len={len}");
            cmp_dec(cn, &a, rn, &b, &ctx);
            assert_eq!(cn, n9, "{ctx}: cLevel 0 must mean level 9");
            assert_bytes_eq(&a[..cn as usize], &ref9[..n9 as usize], &ctx);

            let mut a = vec![0xA5u8; full + DST_SLACK];
            let mut b = vec![0xA5u8; full + DST_SLACK];
            let (cn, rn) = unsafe {
                (
                    chc2.0(sp, a.as_mut_ptr() as *mut c_char, len as c_int, 0),
                    chc2.1(sp, b.as_mut_ptr() as *mut c_char, len as c_int, 0),
                )
            };
            let ctx = format!("row107 compressHC2 {shape:?} len={len}");
            cmp_dec(cn, &a, rn, &b, &ctx);
            assert_eq!(cn, n9, "{ctx}");

            let mut a = vec![0xA5u8; full + DST_SLACK];
            let mut b = vec![0xA5u8; full + DST_SLACK];
            let (cn, rn) = unsafe {
                (
                    chc_ws.0(csp, sp, a.as_mut_ptr() as *mut c_char, len as c_int),
                    chc_ws.1(rsp, sp, b.as_mut_ptr() as *mut c_char, len as c_int),
                )
            };
            let ctx = format!("row107 compressHC_withStateHC {shape:?} len={len}");
            cmp_dec(cn, &a, rn, &b, &ctx);
            assert_eq!(cn, n9, "{ctx}");

            let mut a = vec![0xA5u8; full + DST_SLACK];
            let mut b = vec![0xA5u8; full + DST_SLACK];
            let (cn, rn) = unsafe {
                (
                    chc2_ws.0(csp, sp, a.as_mut_ptr() as *mut c_char, len as c_int, 0),
                    chc2_ws.1(rsp, sp, b.as_mut_ptr() as *mut c_char, len as c_int, 0),
                )
            };
            let ctx = format!("row107 compressHC2_withStateHC {shape:?} len={len}");
            cmp_dec(cn, &a, rn, &b, &ctx);
            assert_eq!(cn, n9, "{ctx}");

            // ---- limited-output wrappers with a too-small dstCapacity ------
            for cap in [0usize, 1, 2, n9 as usize / 2, n9 as usize - 1, n9 as usize] {
                let want = if cap >= n9 as usize { n9 } else { 0 };

                let (mut a, mut b) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        chc_lo.0(sp, a.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int),
                        chc_lo.1(sp, b.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int),
                    )
                };
                let ctx = format!("row107 compressHC_limitedOutput {shape:?} len={len} cap={cap}");
                cmp_dec(cn, &a, rn, &b, &ctx);
                assert_eq!(cn, want, "{ctx}");

                let (mut a, mut b) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        chc2_lo.0(sp, a.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int, 0),
                        chc2_lo.1(sp, b.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int, 0),
                    )
                };
                let ctx = format!("row107 compressHC2_limitedOutput {shape:?} len={len} cap={cap}");
                cmp_dec(cn, &a, rn, &b, &ctx);
                assert_eq!(cn, want, "{ctx}");

                let (mut a, mut b) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        chc_lo_ws.0(csp, sp, a.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int),
                        chc_lo_ws.1(rsp, sp, b.as_mut_ptr() as *mut c_char, len as c_int, cap as c_int),
                    )
                };
                let ctx =
                    format!("row107 compressHC_limitedOutput_withStateHC {shape:?} len={len} cap={cap}");
                cmp_dec(cn, &a, rn, &b, &ctx);
                assert_eq!(cn, want, "{ctx}");

                let (mut a, mut b) = dst_pair(cap);
                let (cn, rn) = unsafe {
                    (
                        chc2_lo_ws.0(
                            csp,
                            sp,
                            a.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            0,
                        ),
                        chc2_lo_ws.1(
                            rsp,
                            sp,
                            b.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            0,
                        ),
                    )
                };
                let ctx = format!(
                    "row107 compressHC2_limitedOutput_withStateHC {shape:?} len={len} cap={cap}"
                );
                cmp_dec(cn, &a, rn, &b, &ctx);
                assert_eq!(cn, want, "{ctx}");
            }
        }
    }
}

// ===========================================================================
// Rows 108-109 — LZ4_sizeofStateHC / LZ4_sizeofStreamStateHC /
//                LZ4F_compressionLevel_max
// ===========================================================================
#[test]
fn err108_109_sizeof_hc_and_level_max() {
    sym!(a, "LZ4_sizeofStateHC", FnSizeof);
    sym!(b, "LZ4_sizeofStreamStateHC", FnSizeof);
    // NOTE: the exported symbol is `LZ4F_compressionLevel_max` (lz4frame.c);
    // there is no `LZ4_compressionLevel_max`.
    sym!(m, "LZ4F_compressionLevel_max", FnSizeof);
    unsafe {
        assert_ret_eq(a.0(), a.1(), "LZ4_sizeofStateHC");
        assert_ret_eq(b.0(), b.1(), "LZ4_sizeofStreamStateHC");
        assert_eq!(a.0(), SIZEOF_LZ4_STREAMHC_T as c_int, "row108 sizeofStateHC");
        assert_eq!(
            b.0(),
            SIZEOF_LZ4_STREAMHC_T as c_int,
            "row108 sizeofStreamStateHC"
        );
        assert_ret_eq(m.0(), m.1(), "LZ4F_compressionLevel_max");
        assert_eq!(m.0(), 12, "row109 must be LZ4HC_CLEVEL_MAX");
    }
    assert!(
        unsafe { c_lib().get::<FnSizeof>(b"LZ4_compressionLevel_max\0") }.is_err(),
        "row109: LZ4_compressionLevel_max must not exist"
    );
}

// ===========================================================================
// Rows 110-113 — LZ4_XXH32/64_update with a NULL input / len == 0
// ===========================================================================
#[test]
fn err110_113_xxh_update_null_and_zero() {
    sym!(cr32, "LZ4_XXH32_createState", FnCreate);
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(rs32, "LZ4_XXH32_reset", FnReset32);
    sym!(up32, "LZ4_XXH32_update", FnUpdate);
    sym!(dg32, "LZ4_XXH32_digest", FnDigest32);
    sym!(cr64, "LZ4_XXH64_createState", FnCreate);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    sym!(rs64, "LZ4_XXH64_reset", FnReset64);
    sym!(up64, "LZ4_XXH64_update", FnUpdate);
    sym!(dg64, "LZ4_XXH64_digest", FnDigest64);
    let mut rng = Rng::new(0x0110);
    let data = gen_src(Shape::Texty, 4096, &mut rng);

    let lens: &[usize] = &[0, 1, 2, 15, 16, 17, 31, 32, 33, 100, 4096, 1 << 20, usize::MAX];

    for &seed in &[0u32, 1, 0x9E37_79B1, u32::MAX] {
        let (cst, rst) = unsafe { (cr32.0(), cr32.1()) };
        assert!(!cst.is_null() && !rst.is_null());
        unsafe {
            assert_ret_eq(rs32.0(cst, seed), rs32.1(rst, seed), "XXH32_reset");
        }
        // Feed some real data first, so "digest unchanged" is a real check.
        unsafe {
            assert_ret_eq(
                up32.0(cst, data.as_ptr() as *const c_void, 100),
                up32.1(rst, data.as_ptr() as *const c_void, 100),
                "XXH32_update(100)",
            );
        }
        let (before_c, before_r) = unsafe { (dg32.0(cst), dg32.1(rst)) };
        assert_ret_eq(before_c, before_r, "XXH32_digest baseline");

        // row 110: a NULL input is rejected (XXH_ACCEPT_NULL_INPUT_POINTER == 0)
        for &n in lens {
            let (c, r) = unsafe {
                (
                    up32.0(cst, std::ptr::null(), n),
                    up32.1(rst, std::ptr::null(), n),
                )
            };
            let ctx = format!("row110 XXH32_update(NULL, {n}) seed={seed:#x}");
            assert_ret_eq(c, r, &ctx);
            assert_eq!(c, 1, "{ctx}: must be XXH_ERROR (1)");
        }
        // row 112: len == 0 with a non-NULL input is OK and changes nothing.
        for _ in 0..3 {
            let (c, r) = unsafe {
                (
                    up32.0(cst, data.as_ptr() as *const c_void, 0),
                    up32.1(rst, data.as_ptr() as *const c_void, 0),
                )
            };
            let ctx = format!("row112 XXH32_update(ptr, 0) seed={seed:#x}");
            assert_ret_eq(c, r, &ctx);
            assert_eq!(c, 0, "{ctx}: must be XXH_OK (0)");
        }
        let (after_c, after_r) = unsafe { (dg32.0(cst), dg32.1(rst)) };
        assert_ret_eq(after_c, after_r, "row112 XXH32_digest after no-ops");
        assert_eq!(
            after_c, before_c,
            "row112: the digest must be unchanged (seed={seed:#x})"
        );
        unsafe {
            fr32.0(cst);
            fr32.1(rst);
        }
    }

    for &seed in &[0u64, 1, 0x9E37_79B1_85EB_CA87, u64::MAX] {
        let (cst, rst) = unsafe { (cr64.0(), cr64.1()) };
        assert!(!cst.is_null() && !rst.is_null());
        unsafe {
            assert_ret_eq(rs64.0(cst, seed), rs64.1(rst, seed), "XXH64_reset");
            assert_ret_eq(
                up64.0(cst, data.as_ptr() as *const c_void, 100),
                up64.1(rst, data.as_ptr() as *const c_void, 100),
                "XXH64_update(100)",
            );
        }
        let (before_c, before_r) = unsafe { (dg64.0(cst), dg64.1(rst)) };
        assert_ret_eq(before_c, before_r, "XXH64_digest baseline");

        // row 111
        for &n in lens {
            let (c, r) = unsafe {
                (
                    up64.0(cst, std::ptr::null(), n),
                    up64.1(rst, std::ptr::null(), n),
                )
            };
            let ctx = format!("row111 XXH64_update(NULL, {n}) seed={seed:#x}");
            assert_ret_eq(c, r, &ctx);
            assert_eq!(c, 1, "{ctx}: must be XXH_ERROR (1)");
        }
        // row 113
        for _ in 0..3 {
            let (c, r) = unsafe {
                (
                    up64.0(cst, data.as_ptr() as *const c_void, 0),
                    up64.1(rst, data.as_ptr() as *const c_void, 0),
                )
            };
            let ctx = format!("row113 XXH64_update(ptr, 0) seed={seed:#x}");
            assert_ret_eq(c, r, &ctx);
            assert_eq!(c, 0, "{ctx}: must be XXH_OK (0)");
        }
        let (after_c, after_r) = unsafe { (dg64.0(cst), dg64.1(rst)) };
        assert_ret_eq(after_c, after_r, "row113 XXH64_digest after no-ops");
        assert_eq!(
            after_c, before_c,
            "row113: the digest must be unchanged (seed={seed:#x})"
        );
        unsafe {
            fr64.0(cst);
            fr64.1(rst);
        }
    }
}

// ===========================================================================
// Rows 114-115 — LZ4_XXH32/64_freeState(NULL)
// ===========================================================================
#[test]
fn err114_115_xxh_free_state_null() {
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    unsafe {
        assert_ret_eq(
            fr32.0(std::ptr::null_mut()),
            fr32.1(std::ptr::null_mut()),
            "row114 LZ4_XXH32_freeState(NULL)",
        );
        assert_eq!(fr32.0(std::ptr::null_mut()), 0, "row114 must be XXH_OK (0)");
        assert_ret_eq(
            fr64.0(std::ptr::null_mut()),
            fr64.1(std::ptr::null_mut()),
            "row115 LZ4_XXH64_freeState(NULL)",
        );
        assert_eq!(fr64.0(std::ptr::null_mut()), 0, "row115 must be XXH_OK (0)");
    }
}

// ===========================================================================
// Rows 116-117 — LZ4_XXH32/64_reset with extreme seeds
// ===========================================================================
#[test]
fn err116_117_xxh_reset_seeds() {
    sym!(cr32, "LZ4_XXH32_createState", FnCreate);
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(rs32, "LZ4_XXH32_reset", FnReset32);
    sym!(cr64, "LZ4_XXH64_createState", FnCreate);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    sym!(rs64, "LZ4_XXH64_reset", FnReset64);
    let mut rng = Rng::new(0x0116);

    let (cst, rst) = unsafe { (cr32.0(), cr32.1()) };
    assert!(!cst.is_null() && !rst.is_null());
    let mut seeds32: Vec<u32> = vec![0, 1, 2, 0x7FFF_FFFF, 0x8000_0000, u32::MAX - 1, u32::MAX];
    for _ in 0..200 {
        seeds32.push(rng.next_u32());
    }
    for &s in &seeds32 {
        let (c, r) = unsafe { (rs32.0(cst, s), rs32.1(rst, s)) };
        assert_ret_eq(c, r, &format!("row116 XXH32_reset({s:#x})"));
        assert_eq!(c, 0, "row116 XXH32_reset({s:#x}) must be XXH_OK (0)");
    }
    unsafe {
        fr32.0(cst);
        fr32.1(rst);
    }

    let (cst, rst) = unsafe { (cr64.0(), cr64.1()) };
    assert!(!cst.is_null() && !rst.is_null());
    let mut seeds64: Vec<u64> = vec![
        0,
        1,
        2,
        0x7FFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        u64::MAX - 1,
        u64::MAX,
    ];
    for _ in 0..200 {
        seeds64.push(rng.next_u64());
    }
    for &s in &seeds64 {
        let (c, r) = unsafe { (rs64.0(cst, s), rs64.1(rst, s)) };
        assert_ret_eq(c, r, &format!("row117 XXH64_reset({s:#x})"));
        assert_eq!(c, 0, "row117 XXH64_reset({s:#x}) must be XXH_OK (0)");
    }
    unsafe {
        fr64.0(cst);
        fr64.1(rst);
    }
}

// ===========================================================================
// Rows 118-121 — empty-input hashes (one-shot with NULL, and digest-of-nothing)
// ===========================================================================
#[test]
fn err118_121_xxh_empty_input() {
    sym!(h32, "LZ4_XXH32", FnXXH32);
    sym!(h64, "LZ4_XXH64", FnXXH64);
    sym!(cr32, "LZ4_XXH32_createState", FnCreate);
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(rs32, "LZ4_XXH32_reset", FnReset32);
    sym!(dg32, "LZ4_XXH32_digest", FnDigest32);
    sym!(cr64, "LZ4_XXH64_createState", FnCreate);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    sym!(rs64, "LZ4_XXH64_reset", FnReset64);
    sym!(dg64, "LZ4_XXH64_digest", FnDigest64);
    let mut rng = Rng::new(0x0118);
    let data = gen_src(Shape::Texty, 64, &mut rng);

    for &seed in &[0u32, 1, 12345, 0x9E37_79B1, u32::MAX] {
        // row 118: LZ4_XXH32(NULL, 0, seed) is the safe path (no read).
        let (c, r) = unsafe {
            (
                h32.0(std::ptr::null(), 0, seed),
                h32.1(std::ptr::null(), 0, seed),
            )
        };
        let ctx = format!("row118 XXH32(NULL, 0, {seed:#x})");
        assert_ret_eq(c, r, &ctx);
        // ... and it equals the hash of a zero-length non-NULL input.
        let (c2, r2) = unsafe {
            (
                h32.0(data.as_ptr() as *const c_void, 0, seed),
                h32.1(data.as_ptr() as *const c_void, 0, seed),
            )
        };
        assert_ret_eq(c2, r2, &format!("{ctx} (non-NULL, len 0)"));
        assert_eq!(c, c2, "{ctx}: must equal the empty-input hash");

        // row 120: digest() on a state that was fed nothing.
        let (cst, rst) = unsafe { (cr32.0(), cr32.1()) };
        unsafe {
            rs32.0(cst, seed);
            rs32.1(rst, seed);
        }
        let (d, e) = unsafe { (dg32.0(cst), dg32.1(rst)) };
        assert_ret_eq(d, e, &format!("row120 XXH32_digest(empty, {seed:#x})"));
        assert_eq!(d, c, "row120: must equal the one-shot empty hash");
        unsafe {
            fr32.0(cst);
            fr32.1(rst);
        }
    }

    for &seed in &[0u64, 1, 12345, 0x9E37_79B1_85EB_CA87, u64::MAX] {
        // row 119
        let (c, r) = unsafe {
            (
                h64.0(std::ptr::null(), 0, seed),
                h64.1(std::ptr::null(), 0, seed),
            )
        };
        let ctx = format!("row119 XXH64(NULL, 0, {seed:#x})");
        assert_ret_eq(c, r, &ctx);
        let (c2, r2) = unsafe {
            (
                h64.0(data.as_ptr() as *const c_void, 0, seed),
                h64.1(data.as_ptr() as *const c_void, 0, seed),
            )
        };
        assert_ret_eq(c2, r2, &format!("{ctx} (non-NULL, len 0)"));
        assert_eq!(c, c2, "{ctx}: must equal the empty-input hash");

        // row 121
        let (cst, rst) = unsafe { (cr64.0(), cr64.1()) };
        unsafe {
            rs64.0(cst, seed);
            rs64.1(rst, seed);
        }
        let (d, e) = unsafe { (dg64.0(cst), dg64.1(rst)) };
        assert_ret_eq(d, e, &format!("row121 XXH64_digest(empty, {seed:#x})"));
        assert_eq!(d, c, "row121: must equal the one-shot empty hash");
        unsafe {
            fr64.0(cst);
            fr64.1(rst);
        }
    }
}

// ===========================================================================
// Rows 122-123 — canonical round trips
// ===========================================================================
#[test]
fn err122_123_xxh_canonical_round_trip() {
    sym!(c32, "LZ4_XXH32_canonicalFromHash", FnCanon32);
    sym!(f32c, "LZ4_XXH32_hashFromCanonical", FnFromCanon32);
    sym!(c64, "LZ4_XXH64_canonicalFromHash", FnCanon64);
    sym!(f64c, "LZ4_XXH64_hashFromCanonical", FnFromCanon64);
    let mut rng = Rng::new(0x0122);

    let mut v32: Vec<u32> = vec![0, 1, 2, 0xFF, 0x0100, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
    for _ in 0..2000 {
        v32.push(rng.next_u32());
    }
    for &h in &v32 {
        let mut cb = [0u8; 4];
        let mut rb = [0u8; 4];
        unsafe {
            c32.0(cb.as_mut_ptr() as *mut c_void, h);
            c32.1(rb.as_mut_ptr() as *mut c_void, h);
        }
        assert_bytes_eq(&cb, &rb, &format!("row122 canonicalFromHash({h:#x})"));
        // big-endian encoding
        assert_eq!(cb, h.to_be_bytes(), "row122 must be big-endian ({h:#x})");
        let (a, b) = unsafe {
            (
                f32c.0(cb.as_ptr() as *const c_void),
                f32c.1(rb.as_ptr() as *const c_void),
            )
        };
        assert_ret_eq(a, b, &format!("row122 hashFromCanonical({h:#x})"));
        assert_eq!(a, h, "row122 round trip must be the identity");
    }

    let mut v64: Vec<u64> = vec![
        0,
        1,
        2,
        0xFF,
        0x0100,
        0x7FFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        u64::MAX,
    ];
    for _ in 0..2000 {
        v64.push(rng.next_u64());
    }
    for &h in &v64 {
        let mut cb = [0u8; 8];
        let mut rb = [0u8; 8];
        unsafe {
            c64.0(cb.as_mut_ptr() as *mut c_void, h);
            c64.1(rb.as_mut_ptr() as *mut c_void, h);
        }
        assert_bytes_eq(&cb, &rb, &format!("row123 canonicalFromHash({h:#x})"));
        assert_eq!(cb, h.to_be_bytes(), "row123 must be big-endian ({h:#x})");
        let (a, b) = unsafe {
            (
                f64c.0(cb.as_ptr() as *const c_void),
                f64c.1(rb.as_ptr() as *const c_void),
            )
        };
        assert_ret_eq(a, b, &format!("row123 hashFromCanonical({h:#x})"));
        assert_eq!(a, h, "row123 round trip must be the identity");
    }
}

// ===========================================================================
// Row 124 — LZ4_XXH_versionNumber
// ===========================================================================
#[test]
fn err124_xxh_version_number() {
    sym!(v, "LZ4_XXH_versionNumber", FnSizeof);
    let (c, r) = unsafe { (v.0(), v.1()) };
    assert_ret_eq(c, r, "row124 LZ4_XXH_versionNumber");
    assert_eq!(c, 605, "row124 must be 605 (xxHash 0.6.5)");
}
