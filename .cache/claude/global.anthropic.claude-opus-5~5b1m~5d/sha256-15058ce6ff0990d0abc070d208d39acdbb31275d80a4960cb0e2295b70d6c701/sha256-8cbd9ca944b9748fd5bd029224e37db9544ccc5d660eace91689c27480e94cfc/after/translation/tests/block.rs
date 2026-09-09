//! Differential tests for the **`lz4.c` block API** (`c_src/include/lz4.h`).
//!
//! Every call is made through `dlsym` against BOTH shared libraries (the C
//! ground truth and the Rust translation) and the results — return codes AND
//! output bytes — are compared byte-for-byte.
//!
//! Coverage map (see `translation/ERRORS.md` "Rejection table" for row numbers):
//!
//! * A — bounds / constants: rows 1, 45, 46
//! * B — one-shot compression, all shapes x threshold sizes x acceleration
//!       clamping (`lz4.c:1386`, `lz4.c:1417`), `byU16` vs `byU32` table type
//!       selection at `LZ4_64Klimit` (== 64KB + MFLIMIT-1 == 65547, `lz4.c:710`)
//! * C — `dstCapacity` sweep: rows 3, 5, 6, 7
//! * D — `LZ4_compress_destSize` / `_extState`: row 4
//! * E — decompression of valid blocks, `_partial` clamping (`lz4.c:2461`),
//!       `withPrefix64k`
//! * F — dictionary / streaming compression: prefix, extDict, dictCtx regimes;
//!       rows 9, 10, 11, 13, 14
//! * G — streaming decompression (linear + ring buffer) and the stateless
//!       `usingDict` / `forceExtDict` entry points: rows 47, 48
//! * H — deprecated / legacy family
//! * I — error paths and corrupt/truncated input fuzzing: rows 2, 12, 15-44
#![allow(clippy::too_many_arguments)]
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// small shared helpers
// ---------------------------------------------------------------------------

/// `LZ4_sizeofState()`, asserted identical in both libraries.
fn sizeof_state() -> usize {
    let (cv, rv) = both(|l| unsafe { l.get::<FnI>("LZ4_sizeofState")() });
    assert_eq!(cv, rv, "LZ4_sizeofState differs");
    assert!(cv > 0);
    cv as usize
}

fn c_bound(n: usize) -> usize {
    let v = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(n as c_int) };
    assert!(v > 0, "compressBound({}) == 0", n);
    v as usize
}

fn zero_at(p: *mut c_void, n: usize) {
    unsafe { std::ptr::write_bytes(p as *mut u8, 0, n) }
}

/// One side of a dual call: the return code plus the whole destination buffer
/// (padding included, so any out-of-contract write is caught too).
struct Out {
    ret: c_int,
    buf: Vec<u8>,
}

const DST_SLACK: usize = 64;

/// Run `f` against both libraries with private, identically pre-filled
/// destination buffers.
fn dual<F>(cap: usize, f: F) -> (Out, Out)
where
    F: Fn(&'static Lib, *mut c_char) -> c_int,
{
    let mut cb = vec![0x5Au8; cap + DST_SLACK];
    let mut rb = vec![0x5Au8; cap + DST_SLACK];
    let cr = f(c(), cb.as_mut_ptr() as *mut c_char);
    let rr = f(r(), rb.as_mut_ptr() as *mut c_char);
    (Out { ret: cr, buf: cb }, Out { ret: rr, buf: rb })
}

fn expect_same(ctx: &str, a: &Out, b: &Out) {
    assert_eq!(a.ret, b.ret, "{}: return C={} Rust={}", ctx, a.ret, b.ret);
    if let Some(m) = diff_report(ctx, &a.buf, &b.buf) {
        panic!("{}", m);
    }
}

/// Compare a decompression-style call on both libraries, reusing buffers.
fn dcmp<F>(ctx: &str, cd: &mut [u8], rd: &mut [u8], f: F)
where
    F: Fn(&'static Lib, *mut c_char) -> c_int,
{
    cd.fill(0x5A);
    rd.fill(0x5A);
    let cr = f(c(), cd.as_mut_ptr() as *mut c_char);
    let rr = f(r(), rd.as_mut_ptr() as *mut c_char);
    assert_eq!(cr, rr, "{}: return C={} Rust={}", ctx, cr, rr);
    if let Some(m) = diff_report(ctx, cd, rd) {
        panic!("{}", m);
    }
}

/// Produce a valid block with the **C** library (ground truth) so that
/// decompression tests are not contaminated by a compression divergence.
fn c_compress(src: &[u8]) -> Vec<u8> {
    let cap = c_bound(src.len());
    let mut d = vec![0u8; cap];
    let n = unsafe {
        c().get::<Fn_compress_default>("LZ4_compress_default")(
            src.as_ptr() as *const c_char,
            d.as_mut_ptr() as *mut c_char,
            src.len() as c_int,
            cap as c_int,
        )
    };
    assert!(n > 0, "C compression failed for len {}", src.len());
    d.truncate(n as usize);
    d
}

/// A block plus enough trailing readable slack that `LZ4_decompress_fast*()`
/// (which does not know its input size) reads identical bytes in both runs.
const IN_PAD: usize = 1 << 16;

fn padded(block: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(block.len() + IN_PAD);
    v.extend_from_slice(block);
    v.resize(block.len() + IN_PAD, 0);
    v
}

/// Compress `chunks` consecutive slices of `src` with the C library's
/// streaming compressor (optionally preceded by `dict`).
fn c_stream_blocks(dict: Option<&[u8]>, src: &[u8], chunks: &[usize]) -> Vec<Vec<u8>> {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let mut out = Vec::new();
    unsafe {
        let init = c().get::<Fn_initStream>("LZ4_initStream");
        assert!(!init(st.ptr(), ss).is_null());
        if let Some(d) = dict {
            let ld = c().get::<Fn_loadDict>("LZ4_loadDict");
            ld(st.ptr(), d.as_ptr() as *const c_char, d.len() as c_int);
        }
        let cont = c().get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
        let mut off = 0usize;
        for &n in chunks {
            let cap = c_bound(n);
            let mut b = vec![0u8; cap];
            let ret = cont(
                st.ptr(),
                src.as_ptr().add(off) as *const c_char,
                b.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
                1,
            );
            assert!(ret > 0, "C stream compression failed (chunk {})", n);
            b.truncate(ret as usize);
            out.push(b);
            off += n;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// traces, used for the multi-step streaming scenarios
// ---------------------------------------------------------------------------

struct Rec {
    tag: String,
    ret: i64,
    bytes: Vec<u8>,
}

fn rec(tag: impl Into<String>, ret: i64, bytes: &[u8]) -> Rec {
    Rec {
        tag: tag.into(),
        ret,
        bytes: bytes.to_vec(),
    }
}

fn cmp_traces(ctx: &str, ct: &[Rec], rt: &[Rec]) {
    assert_eq!(ct.len(), rt.len(), "{}: trace length differs", ctx);
    for (i, (a, b)) in ct.iter().zip(rt.iter()).enumerate() {
        assert_eq!(a.tag, b.tag, "{}: step {} tag differs", ctx, i);
        assert_eq!(
            a.ret, b.ret,
            "{}: step {} [{}] return C={} Rust={}",
            ctx, i, a.tag, a.ret, b.ret
        );
        let c2 = format!("{}: step {} [{}]", ctx, i, a.tag);
        if let Some(m) = diff_report(&c2, &a.bytes, &b.bytes) {
            panic!("{}", m);
        }
    }
}

// ---------------------------------------------------------------------------
// parameter sweeps
// ---------------------------------------------------------------------------

/// Sizes chosen to straddle every threshold `lz4.c` branches on:
/// MINMATCH(4), LASTLITERALS(5), MFLIMIT(12), LZ4_minLength(13),
/// the 15/255 length-continuation boundaries, LZ4_DISTANCE_MAX(65535)
/// and LZ4_64Klimit (65547) which selects `byU16` vs `byU32`.
const SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 5, 11, 12, 13, 15, 16, 63, 64, 65, 254, 255, 256, 511, 512, 513, 4095, 4096,
    65534, 65535, 65536, 65537, 65538, 65546, 65547, 65548, 131072, 200000, 300000,
];

/// Same list, trimmed of the two largest entries, for the O(n*m) sweeps.
const SIZES_MID: &[usize] = &[
    0, 1, 2, 3, 4, 5, 11, 12, 13, 15, 16, 63, 64, 65, 254, 255, 256, 512, 513, 4096, 65535, 65536,
    65537, 65547, 131072,
];

const SIZES_SMALL: &[usize] = &[
    0, 1, 4, 5, 12, 13, 16, 64, 255, 256, 513, 4096, 65535, 65536, 65537, 65547,
];

/// Acceleration values: below the minimum (clamped to
/// `LZ4_ACCELERATION_DEFAULT` == 1) and above `LZ4_ACCELERATION_MAX` == 65537
/// (clamped down), `lz4.c:1386-1387`.
const ACCELS: &[c_int] = &[
    c_int::MIN,
    -1,
    0,
    1,
    2,
    3,
    5,
    17,
    100,
    1000,
    65536,
    65537,
    65538,
    c_int::MAX,
];

/// A wide sweep of `int` values for the pure integer functions.
fn int_sweep() -> Vec<c_int> {
    let mut v: Vec<c_int> = vec![
        0,
        1,
        2,
        3,
        4,
        5,
        11,
        12,
        13,
        15,
        16,
        17,
        63,
        64,
        65,
        254,
        255,
        256,
        511,
        512,
        513,
        4095,
        4096,
        65534,
        65535,
        65536,
        65537,
        65538,
        131072,
        200000,
        300000,
        0x7DFF_FFFF,
        0x7E00_0000,
        0x7E00_0001,
        0x7E00_0002,
        c_int::MAX - 1,
        c_int::MAX,
        -1,
        -2,
        -16,
        -255,
        -65536,
        -0x7E00_0000,
        c_int::MIN + 1,
        c_int::MIN,
    ];
    let mut rng = Rng::new(0xB0_0000_0001);
    for _ in 0..4000 {
        v.push(rng.next_u32() as c_int);
    }
    v
}

// ===========================================================================
// A.  bounds and constants
// ===========================================================================

/// ERRORS.md row 1 (`LZ4_compressBound` rejects `(unsigned)isize > 0x7E000000`)
/// and rows 45/46 (`LZ4_decoderRingBufferSize` rejects negative / oversized).
#[test]
fn a_bounds_and_ring_buffer_size_sweep() {
    for v in int_sweep() {
        let (cb, rb) = both(|l| unsafe { l.get::<Fn_compressBound>("LZ4_compressBound")(v) });
        assert_eq!(cb, rb, "LZ4_compressBound({}) C={} Rust={}", v, cb, rb);
        let (cd, rd) =
            both(|l| unsafe { l.get::<Fn_decoderRingBufferSize>("LZ4_decoderRingBufferSize")(v) });
        assert_eq!(
            cd, rd,
            "LZ4_decoderRingBufferSize({}) C={} Rust={}",
            v, cd, rd
        );
    }
    // explicit sentinels
    for &v in &[
        0x7E00_0001i32,
        0x7EFF_FFFF,
        c_int::MAX,
        -1,
        c_int::MIN,
        -0x7E00_0000,
    ] {
        let (cb, rb) = both(|l| unsafe { l.get::<Fn_compressBound>("LZ4_compressBound")(v) });
        assert_eq!(cb, 0, "C compressBound({}) should be 0", v);
        assert_eq!(rb, 0, "Rust compressBound({}) should be 0", v);
    }
    for &v in &[0x7E00_0001i32, c_int::MAX, -1, c_int::MIN] {
        let (cd, rd) =
            both(|l| unsafe { l.get::<Fn_decoderRingBufferSize>("LZ4_decoderRingBufferSize")(v) });
        assert_eq!(cd, 0, "C decoderRingBufferSize({}) should be 0", v);
        assert_eq!(rd, 0, "Rust decoderRingBufferSize({}) should be 0", v);
    }
}

#[test]
fn a_constants_and_version() {
    for name in ["LZ4_sizeofState", "LZ4_sizeofStreamState", "LZ4_versionNumber"] {
        let (cv, rv) = both(|l| unsafe { l.get::<FnI>(name)() });
        assert_eq!(cv, rv, "{}() C={} Rust={}", name, cv, rv);
    }
    let (cs, rs) = both(|l| cstr(unsafe { l.get::<FnStr>("LZ4_versionString")() }));
    assert_eq!(cs, rs, "LZ4_versionString");
    // sanity: the size the API advertises must accept LZ4_initStream
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let (cp, rp) = both(|l| unsafe { l.get::<Fn_initStream>("LZ4_initStream")(st.ptr(), ss) });
    assert!(!cp.is_null() && !rp.is_null(), "initStream(sizeofState) failed");
}

// ===========================================================================
// B.  one-shot compression
// ===========================================================================

#[test]
fn b_compress_default_all_shapes_all_sizes() {
    let mut rng = Rng::new(0x5EED_1001);
    for &shape in ALL_SHAPES {
        for &len in SIZES {
            let src = gen(&mut rng, len, shape);
            let cap = c_bound(len);
            let sp = src.as_ptr() as *const c_char;
            let (a, b) = dual(cap, |l, dp| unsafe {
                l.get::<Fn_compress_default>("LZ4_compress_default")(
                    sp,
                    dp,
                    len as c_int,
                    cap as c_int,
                )
            });
            let ctx = format!("LZ4_compress_default len={} shape={:?}", len, shape);
            expect_same(&ctx, &a, &b);
            assert!(a.ret > 0, "{}: C failed unexpectedly", ctx);
        }
    }
}

/// Acceleration below 1 must be clamped to `LZ4_ACCELERATION_DEFAULT`, above
/// `LZ4_ACCELERATION_MAX` (65537) clamped down — identically in both libs
/// (`lz4.c:1386`).
#[test]
fn b_compress_fast_acceleration_clamping() {
    let mut rng = Rng::new(0x5EED_1002);
    for &shape in ALL_SHAPES {
        for &len in SIZES_MID {
            let src = gen(&mut rng, len, shape);
            let cap = c_bound(len);
            let sp = src.as_ptr() as *const c_char;
            let mut per_accel: Vec<(c_int, Vec<u8>, c_int)> = Vec::new();
            for &acc in ACCELS {
                let (a, b) = dual(cap, |l, dp| unsafe {
                    l.get::<Fn_compress_fast>("LZ4_compress_fast")(
                        sp,
                        dp,
                        len as c_int,
                        cap as c_int,
                        acc,
                    )
                });
                let ctx = format!(
                    "LZ4_compress_fast len={} shape={:?} accel={}",
                    len, shape, acc
                );
                expect_same(&ctx, &a, &b);
                per_accel.push((acc, a.buf[..a.ret.max(0) as usize].to_vec(), a.ret));
            }
            // clamping is observable: acceleration <=0 must behave like 1, and
            // 65538 / INT_MAX like 65537.
            let one = per_accel.iter().find(|e| e.0 == 1).unwrap().clone();
            for &acc in &[c_int::MIN, -1, 0] {
                let e = per_accel.iter().find(|x| x.0 == acc).unwrap();
                assert_eq!(
                    (e.2, &e.1),
                    (one.2, &one.1),
                    "acceleration {} not clamped to 1 (len={} shape={:?})",
                    acc,
                    len,
                    shape
                );
            }
            let max = per_accel.iter().find(|e| e.0 == 65537).unwrap().clone();
            for &acc in &[65538, c_int::MAX] {
                let e = per_accel.iter().find(|x| x.0 == acc).unwrap();
                assert_eq!(
                    (e.2, &e.1),
                    (max.2, &max.1),
                    "acceleration {} not clamped to 65537 (len={} shape={:?})",
                    acc,
                    len,
                    shape
                );
            }
        }
    }
}

#[test]
fn b_compress_fast_extState_output_and_state_bytes() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let mut rng = Rng::new(0x5EED_1003);
    for &shape in ALL_SHAPES {
        for &len in SIZES {
            let src = gen(&mut rng, len, shape);
            let cap = c_bound(len);
            let sp = src.as_ptr() as *const c_char;
            for &acc in &[1i32, 2, 7, 65537] {
                let mut states: Vec<Vec<u8>> = Vec::new();
                let mut outs: Vec<Out> = Vec::new();
                for lib in [c(), r()] {
                    zero_at(st.ptr(), ss);
                    let mut buf = vec![0x5Au8; cap + DST_SLACK];
                    let ret = unsafe {
                        lib.get::<Fn_compress_fast_extState>("LZ4_compress_fast_extState")(
                            st.ptr(),
                            sp,
                            buf.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            acc,
                        )
                    };
                    states.push(st.bytes().to_vec());
                    outs.push(Out { ret, buf });
                }
                let ctx = format!(
                    "LZ4_compress_fast_extState len={} shape={:?} accel={}",
                    len, shape, acc
                );
                expect_same(&ctx, &outs[0], &outs[1]);
                let sctx = format!("{} :: state bytes", ctx);
                if let Some(m) = diff_report(&sctx, &states[0], &states[1]) {
                    panic!("{}", m);
                }
            }
        }
    }
}

/// `LZ4_compress_fast_extState_fastReset` re-uses an already-initialised state
/// across several calls; the whole call sequence (outputs + state) must match.
#[test]
fn b_compress_fast_extState_fastReset_sequence() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let mut rng = Rng::new(0x5EED_1004);
    for &shape in ALL_SHAPES {
        for &len in SIZES_SMALL {
            let inputs: Vec<Vec<u8>> = (0..4).map(|_| gen(&mut rng, len, shape)).collect();
            let cap = c_bound(len);
            let ip: Vec<*const c_char> =
                inputs.iter().map(|v| v.as_ptr() as *const c_char).collect();
            let stp = st.ptr();
            let run = |l: &'static Lib| -> Vec<Rec> {
                zero_at(stp, ss);
                let mut t = Vec::new();
                unsafe {
                    let init = l.get::<Fn_initStream>("LZ4_initStream");
                    let ok = init(stp, ss);
                    t.push(rec("initStream", !ok.is_null() as i64, &[]));
                    let f = l.get::<Fn_compress_fast_extState>(
                        "LZ4_compress_fast_extState_fastReset",
                    );
                    for (i, p) in ip.iter().enumerate() {
                        let mut buf = vec![0x5Au8; cap + DST_SLACK];
                        let ret = f(
                            stp,
                            *p,
                            buf.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                            1,
                        );
                        t.push(rec(format!("fastReset[{}]", i), ret as i64, &buf));
                        t.push(rec(
                            format!("fastReset[{}].state", i),
                            0,
                            std::slice::from_raw_parts(stp as *const u8, ss),
                        ));
                    }
                }
                t
            };
            let ct = run(c());
            let rt = run(r());
            cmp_traces(
                &format!("extState_fastReset len={} shape={:?}", len, shape),
                &ct,
                &rt,
            );
        }
    }
}

// ===========================================================================
// C.  dstCapacity sweep -- the limitedOutput rejection branches
// ===========================================================================

/// ERRORS.md rows 3, 5, 6, 7: the three `limitedOutput` bail-outs plus the
/// `srcSize == 0 && dstCapacity <= 0` rejection.
#[test]
fn c_dst_capacity_sweep_rejection_paths() {
    let mut rng = Rng::new(0x5EED_2001);
    for &shape in ALL_SHAPES {
        for &len in SIZES_MID {
            let src = gen(&mut rng, len, shape);
            let bound = c_bound(len);
            let csize = c_compress(&src).len();
            let sp = src.as_ptr() as *const c_char;
            let mut caps: Vec<usize> = vec![
                bound,
                csize,
                csize.saturating_sub(1),
                csize / 2,
                len,
                len / 2,
                1,
                0,
            ];
            caps.dedup();
            for &cap in &caps {
                let (a, b) = dual(cap, |l, dp| unsafe {
                    l.get::<Fn_compress_default>("LZ4_compress_default")(
                        sp,
                        dp,
                        len as c_int,
                        cap as c_int,
                    )
                });
                let ctx = format!(
                    "LZ4_compress_default len={} shape={:?} cap={} (csize={} bound={})",
                    len, shape, cap, csize, bound
                );
                expect_same(&ctx, &a, &b);
                if cap >= csize {
                    assert!(a.ret > 0, "{}: should have succeeded", ctx);
                }
                // the fast path with acceleration too
                for &acc in &[1i32, 4, 65537] {
                    let (a, b) = dual(cap, |l, dp| unsafe {
                        l.get::<Fn_compress_fast>("LZ4_compress_fast")(
                            sp,
                            dp,
                            len as c_int,
                            cap as c_int,
                            acc,
                        )
                    });
                    expect_same(
                        &format!(
                            "LZ4_compress_fast len={} shape={:?} cap={} accel={}",
                            len, shape, cap, acc
                        ),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

/// A randomized dstCapacity walk: every capacity from 0 to `csize + 4` for a
/// handful of inputs, which lands squarely on the literal-run / match-length /
/// last-literals rejection branches (ERRORS.md rows 5, 6, 7).
#[test]
fn c_dst_capacity_exhaustive_small() {
    let mut rng = Rng::new(0x5EED_2002);
    for &shape in ALL_SHAPES {
        for &len in &[13usize, 20, 40, 100, 300, 1000, 4096] {
            let src = gen(&mut rng, len, shape);
            let csize = c_compress(&src).len();
            let sp = src.as_ptr() as *const c_char;
            for cap in 0..=(csize + 4) {
                let (a, b) = dual(cap, |l, dp| unsafe {
                    l.get::<Fn_compress_default>("LZ4_compress_default")(
                        sp,
                        dp,
                        len as c_int,
                        cap as c_int,
                    )
                });
                expect_same(
                    &format!(
                        "exhaustive cap: len={} shape={:?} cap={} csize={}",
                        len, shape, cap, csize
                    ),
                    &a,
                    &b,
                );
            }
        }
    }
}

// ===========================================================================
// D.  LZ4_compress_destSize / _extState
// ===========================================================================

fn destsize_targets(bound: usize) -> Vec<c_int> {
    let mut v: Vec<c_int> = vec![0, 1, 2, 5, 12, 16, 64, 300];
    for d in [1usize, 2, 3, 4, 8, 16, 64] {
        v.push((bound / d) as c_int);
    }
    v.push(bound as c_int);
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn d_compress_destSize_sweep() {
    let mut rng = Rng::new(0x5EED_3001);
    for &shape in ALL_SHAPES {
        for &len in SIZES_SMALL {
            let src = gen(&mut rng, len, shape);
            let bound = c_bound(len);
            let sp = src.as_ptr() as *const c_char;
            for target in destsize_targets(bound) {
                let t = target.max(0) as usize;
                let mut cb = vec![0x5Au8; t + DST_SLACK];
                let mut rb = vec![0x5Au8; t + DST_SLACK];
                let mut cs = len as c_int;
                let mut rs = len as c_int;
                let cret = unsafe {
                    c().get::<Fn_compress_destSize>("LZ4_compress_destSize")(
                        sp,
                        cb.as_mut_ptr() as *mut c_char,
                        &mut cs,
                        target,
                    )
                };
                let rret = unsafe {
                    r().get::<Fn_compress_destSize>("LZ4_compress_destSize")(
                        sp,
                        rb.as_mut_ptr() as *mut c_char,
                        &mut rs,
                        target,
                    )
                };
                let ctx = format!(
                    "LZ4_compress_destSize len={} shape={:?} target={}",
                    len, shape, target
                );
                assert_eq!(cret, rret, "{}: return C={} Rust={}", ctx, cret, rret);
                assert_eq!(cs, rs, "{}: *srcSizePtr C={} Rust={}", ctx, cs, rs);
                if let Some(m) = diff_report(&ctx, &cb, &rb) {
                    panic!("{}", m);
                }
                if cret > 0 {
                    assert!(cs >= 0 && cs as usize <= len, "{}: consumed out of range", ctx);
                    // the produced block must decode to exactly the consumed prefix
                    let consumed = cs as usize;
                    let mut out = vec![0u8; consumed + 1];
                    let n = unsafe {
                        c().get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                            cb.as_ptr() as *const c_char,
                            out.as_mut_ptr() as *mut c_char,
                            cret,
                            (consumed + 1) as c_int,
                        )
                    };
                    assert_eq!(n, consumed as c_int, "{}: round-trip size", ctx);
                    assert_eq!(&out[..consumed], &src[..consumed], "{}: round-trip data", ctx);
                }
            }
        }
    }
}

#[test]
fn d_compress_destSize_extState_sweep() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let mut rng = Rng::new(0x5EED_3002);
    for &shape in ALL_SHAPES {
        for &len in SIZES_SMALL {
            let src = gen(&mut rng, len, shape);
            let bound = c_bound(len);
            let sp = src.as_ptr() as *const c_char;
            for target in destsize_targets(bound) {
                for &acc in &[1i32, 0, 3, 65538] {
                    let t = target.max(0) as usize;
                    let mut rets: Vec<(c_int, c_int, Vec<u8>, Vec<u8>)> = Vec::new();
                    for lib in [c(), r()] {
                        zero_at(st.ptr(), ss);
                        let mut buf = vec![0x5Au8; t + DST_SLACK];
                        let mut sz = len as c_int;
                        let ret = unsafe {
                            lib.get::<Fn_compress_destSize_extState>(
                                "LZ4_compress_destSize_extState",
                            )(
                                st.ptr(),
                                sp,
                                buf.as_mut_ptr() as *mut c_char,
                                &mut sz,
                                target,
                                acc,
                            )
                        };
                        rets.push((ret, sz, buf, st.bytes().to_vec()));
                    }
                    let ctx = format!(
                        "LZ4_compress_destSize_extState len={} shape={:?} target={} accel={}",
                        len, shape, target, acc
                    );
                    assert_eq!(rets[0].0, rets[1].0, "{}: return", ctx);
                    assert_eq!(rets[0].1, rets[1].1, "{}: *srcSizePtr", ctx);
                    if let Some(m) = diff_report(&ctx, &rets[0].2, &rets[1].2) {
                        panic!("{}", m);
                    }
                    if let Some(m) =
                        diff_report(&format!("{} :: state bytes", ctx), &rets[0].3, &rets[1].3)
                    {
                        panic!("{}", m);
                    }
                    // Round-trip verification only makes sense for in-contract
                    // acceleration: `LZ4_compress_destSize_extState` does NOT
                    // clamp `acceleration` on its `fillOutput` path
                    // (`lz4.c:1488`, unlike `LZ4_compress_fast_extState`), so
                    // acceleration == 0 drives `LZ4_compress_generic` with
                    // step == 0 and emits self-referencing (offset 0) matches.
                    // Both libraries agree on that output (checked above); it
                    // simply is not a decodable block.
                    if rets[0].0 > 0 && acc >= 1 {
                        let consumed = rets[0].1 as usize;
                        let mut out = vec![0u8; consumed + 1];
                        let n = unsafe {
                            c().get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                                rets[0].2.as_ptr() as *const c_char,
                                out.as_mut_ptr() as *mut c_char,
                                rets[0].0,
                                (consumed + 1) as c_int,
                            )
                        };
                        assert_eq!(n, consumed as c_int, "{}: round-trip size", ctx);
                        assert_eq!(&out[..consumed], &src[..consumed], "{}: round-trip", ctx);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// E.  decompression of valid blocks
// ===========================================================================

#[test]
fn e_decompress_safe_capacity_variants() {
    let mut rng = Rng::new(0x5EED_4001);
    for &shape in ALL_SHAPES {
        for &len in SIZES {
            let src = gen(&mut rng, len, shape);
            let blk = c_compress(&src);
            let bp = blk.as_ptr() as *const c_char;
            let clen = blk.len() as c_int;
            for &cap in &[len, len + 1, len.saturating_sub(1), 0] {
                let mut cd = vec![0u8; cap + DST_SLACK];
                let mut rd = vec![0u8; cap + DST_SLACK];
                let ctx = format!(
                    "LZ4_decompress_safe len={} shape={:?} cap={}",
                    len, shape, cap
                );
                dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                    l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(bp, dp, clen, cap as c_int)
                });
                if cap >= len && !(len == 0 && cap == 0) {
                    assert_eq!(
                        &cd[..len],
                        &src[..],
                        "{}: decoded payload differs from original",
                        ctx
                    );
                }
            }
        }
    }
}

#[test]
fn e_decompress_fast_exact_size() {
    let mut rng = Rng::new(0x5EED_4002);
    for &shape in ALL_SHAPES {
        for &len in SIZES {
            let src = gen(&mut rng, len, shape);
            let blk = padded(&c_compress(&src));
            let bp = blk.as_ptr() as *const c_char;
            let mut cd = vec![0u8; len + DST_SLACK];
            let mut rd = vec![0u8; len + DST_SLACK];
            let ctx = format!("LZ4_decompress_fast len={} shape={:?}", len, shape);
            dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                l.get::<Fn_decompress_fast>("LZ4_decompress_fast")(bp, dp, len as c_int)
            });
            if len > 0 {
                assert_eq!(&cd[..len], &src[..], "{}: payload", ctx);
            }
        }
    }
}

/// `LZ4_decompress_safe_partial` clamps `dstCapacity = MIN(target, dstCapacity)`
/// (`lz4.c:2461`); both the >= and < cases are swept.
#[test]
fn e_decompress_safe_partial_target_sweep() {
    let mut rng = Rng::new(0x5EED_4003);
    for &shape in ALL_SHAPES {
        for &len in SIZES_MID {
            let src = gen(&mut rng, len, shape);
            let blk = c_compress(&src);
            let bp = blk.as_ptr() as *const c_char;
            let clen = blk.len() as c_int;
            let mut targets: Vec<usize> = vec![0, 1, 2, 5, 12, 13, 16, 64, 255, 256];
            for d in [1usize, 2, 3, 4, 8, 16, 64, 255] {
                targets.push(len / d);
            }
            targets.push(len);
            targets.push(len + 1);
            targets.push(len + 1000);
            targets.retain(|&t| t <= len + 1000);
            targets.sort_unstable();
            targets.dedup();
            for &target in &targets {
                for &cap in &[target, target + 1, target / 2, 0, len] {
                    let alloc = target.max(cap) + DST_SLACK;
                    let mut cd = vec![0u8; alloc];
                    let mut rd = vec![0u8; alloc];
                    let ctx = format!(
                        "LZ4_decompress_safe_partial len={} shape={:?} target={} cap={}",
                        len, shape, target, cap
                    );
                    dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                        l.get::<Fn_decompress_safe_partial>("LZ4_decompress_safe_partial")(
                            bp,
                            dp,
                            clen,
                            target as c_int,
                            cap as c_int,
                        )
                    });
                }
            }
        }
    }
}

/// `LZ4_decompress_safe_withPrefix64k` / `_fast_withPrefix64k` read the 64 KB
/// *preceding* `dst`; the prefix content is byte-identical in both runs.
#[test]
fn e_decompress_with_prefix_64k() {
    const PFX: usize = 65536;
    let mut rng = Rng::new(0x5EED_4004);
    for &shape in ALL_SHAPES {
        for &len in &[1usize, 5, 13, 64, 255, 4096, 65535, 65536, 131072] {
            // one contiguous buffer: [ prefix | payload ]
            let whole = gen(&mut rng, PFX + len, shape);
            let payload = &whole[PFX..];
            // compress the payload with the prefix as history
            let blocks = c_stream_blocks(None, &whole, &[PFX, len]);
            let blk = padded(&blocks[1]);
            let bp = blk.as_ptr() as *const c_char;
            let clen = blocks[1].len() as c_int;

            for &cap in &[len, len + 1, len.saturating_sub(1)] {
                let mut cd = vec![0u8; PFX + cap + DST_SLACK];
                let mut rd = vec![0u8; PFX + cap + DST_SLACK];
                cd[..PFX].copy_from_slice(&whole[..PFX]);
                rd[..PFX].copy_from_slice(&whole[..PFX]);
                let ctx = format!(
                    "LZ4_decompress_safe_withPrefix64k len={} shape={:?} cap={}",
                    len, shape, cap
                );
                let cr = unsafe {
                    c().get::<Fn_decompress_safe_withPrefix64k>(
                        "LZ4_decompress_safe_withPrefix64k",
                    )(bp, cd.as_mut_ptr().add(PFX) as *mut c_char, clen, cap as c_int)
                };
                let rr = unsafe {
                    r().get::<Fn_decompress_safe_withPrefix64k>(
                        "LZ4_decompress_safe_withPrefix64k",
                    )(bp, rd.as_mut_ptr().add(PFX) as *mut c_char, clen, cap as c_int)
                };
                assert_eq!(cr, rr, "{}: return C={} Rust={}", ctx, cr, rr);
                if let Some(m) = diff_report(&ctx, &cd, &rd) {
                    panic!("{}", m);
                }
                if cap >= len {
                    assert_eq!(cr, len as c_int, "{}", ctx);
                    assert_eq!(&cd[PFX..PFX + len], payload, "{}: payload", ctx);
                }
            }

            let mut cd = vec![0u8; PFX + len + DST_SLACK];
            let mut rd = vec![0u8; PFX + len + DST_SLACK];
            cd[..PFX].copy_from_slice(&whole[..PFX]);
            rd[..PFX].copy_from_slice(&whole[..PFX]);
            let ctx = format!(
                "LZ4_decompress_fast_withPrefix64k len={} shape={:?}",
                len, shape
            );
            let cr = unsafe {
                c().get::<Fn_decompress_fast_withPrefix64k>("LZ4_decompress_fast_withPrefix64k")(
                    bp,
                    cd.as_mut_ptr().add(PFX) as *mut c_char,
                    len as c_int,
                )
            };
            let rr = unsafe {
                r().get::<Fn_decompress_fast_withPrefix64k>("LZ4_decompress_fast_withPrefix64k")(
                    bp,
                    rd.as_mut_ptr().add(PFX) as *mut c_char,
                    len as c_int,
                )
            };
            assert_eq!(cr, rr, "{}: return C={} Rust={}", ctx, cr, rr);
            if let Some(m) = diff_report(&ctx, &cd, &rd) {
                panic!("{}", m);
            }
            assert_eq!(&cd[PFX..PFX + len], payload, "{}: payload", ctx);
        }
    }
}

// ===========================================================================
// F.  dictionary / streaming compression
// ===========================================================================

const DICT_SIZES: &[usize] = &[0, 1, 4, 7, 8, 9, 100, 65535, 65536, 65537, 100_000];

fn chunk_plan(rng: &mut Rng, total: usize, n: usize) -> Vec<usize> {
    let mut left = total;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        if i + 1 == n {
            v.push(left);
        } else {
            let take = rng.range(0, left / (n - i) * 2).min(left);
            v.push(take);
            left -= take;
        }
    }
    v
}

/// Prefix (contiguous) regime: consecutive chunks of one buffer, so
/// `dictEnd == source` and `LZ4_compress_fast_continue` takes the
/// `withPrefix64k` path (`lz4.c:1750`).
#[test]
fn f_stream_prefix_contiguous_chunks() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_5001);
    for &shape in ALL_SHAPES {
        for nchunks in 1..=8usize {
            let total = 200_000;
            let src = gen(&mut rng, total, shape);
            let plan = chunk_plan(&mut rng, total, nchunks);
            let sp = src.as_ptr();
            let planr = plan.clone();
            let run = |l: &'static Lib| -> Vec<Rec> {
                zero_at(stp, ss);
                let mut t = Vec::new();
                unsafe {
                    let init = l.get::<Fn_initStream>("LZ4_initStream");
                    t.push(rec("initStream", !init(stp, ss).is_null() as i64, &[]));
                    let cont =
                        l.get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
                    let mut off = 0usize;
                    let mut blocks: Vec<(Vec<u8>, usize)> = Vec::new();
                    for (i, &n) in planr.iter().enumerate() {
                        let cap = c_bound(n);
                        let mut buf = vec![0x5Au8; cap + DST_SLACK];
                        let ret = cont(
                            stp,
                            sp.add(off) as *const c_char,
                            buf.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cap as c_int,
                            1,
                        );
                        t.push(rec(format!("continue[{}] n={}", i, n), ret as i64, &buf));
                        t.push(rec(
                            format!("continue[{}].state", i),
                            0,
                            std::slice::from_raw_parts(stp as *const u8, ss),
                        ));
                        if ret > 0 {
                            blocks.push((buf[..ret as usize].to_vec(), n));
                        }
                        off += n;
                    }
                    // every produced stream must decode back to the source
                    let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                    assert!(!sd.is_null());
                    let dec =
                        l.get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue");
                    let mut dst = vec![0u8; total + DST_SLACK];
                    let mut doff = 0usize;
                    for (i, (b, n)) in blocks.iter().enumerate() {
                        let ret = dec(
                            sd,
                            b.as_ptr() as *const c_char,
                            dst.as_mut_ptr().add(doff) as *mut c_char,
                            b.len() as c_int,
                            (total - doff) as c_int,
                        );
                        t.push(rec(format!("decode[{}]", i), ret as i64, &[]));
                        assert_eq!(ret, *n as c_int, "{}: stream decode size", l.which);
                        doff += *n;
                    }
                    l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
                    assert_eq!(&dst[..total], &src[..], "{}: stream round-trip", l.which);
                }
                t
            };
            let ct = run(c());
            let rt = run(r());
            cmp_traces(
                &format!("prefix stream shape={:?} nchunks={}", shape, nchunks),
                &ct,
                &rt,
            );
        }
    }
}

/// extDict regime: a separate dictionary buffer loaded with
/// `LZ4_loadDict` / `LZ4_loadDictSlow` / `LZ4_loadDict_internal`
/// (4th argument `LoadDict_mode_e`: 0 == `_ld_fast`, 1 == `_ld_slow`,
/// `lz4.c:1585`).  ERRORS.md rows 13, 14.
#[test]
fn f_stream_extdict_loaddict_variants() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_5002);
    for &shape in ALL_SHAPES {
        for &dsize in DICT_SIZES {
            let dict = gen(&mut rng, dsize, shape);
            let total = 30_000;
            let src = gen(&mut rng, total, shape);
            let plan = chunk_plan(&mut rng, total, 4);
            let dp = dict.as_ptr();
            let sp = src.as_ptr();
            for mode in 0..4 {
                let planr = plan.clone();
                let run = |l: &'static Lib| -> Vec<Rec> {
                    zero_at(stp, ss);
                    let mut t = Vec::new();
                    unsafe {
                        let init = l.get::<Fn_initStream>("LZ4_initStream");
                        t.push(rec("initStream", !init(stp, ss).is_null() as i64, &[]));
                        let ld = match mode {
                            0 => {
                                let f = l.get::<Fn_loadDict>("LZ4_loadDict");
                                f(stp, dp as *const c_char, dsize as c_int)
                            }
                            1 => {
                                let f = l.get::<Fn_loadDict>("LZ4_loadDictSlow");
                                f(stp, dp as *const c_char, dsize as c_int)
                            }
                            2 => {
                                let f = l.get::<Fn_loadDict_internal>("LZ4_loadDict_internal");
                                f(stp, dp as *const c_char, dsize as c_int, 0)
                            }
                            _ => {
                                let f = l.get::<Fn_loadDict_internal>("LZ4_loadDict_internal");
                                f(stp, dp as *const c_char, dsize as c_int, 1)
                            }
                        };
                        t.push(rec("loadDict", ld as i64, &[]));
                        t.push(rec(
                            "loadDict.state",
                            0,
                            std::slice::from_raw_parts(stp as *const u8, ss),
                        ));
                        let cont =
                            l.get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
                        let mut off = 0usize;
                        let mut blocks: Vec<(Vec<u8>, usize, usize)> = Vec::new();
                        for (i, &n) in planr.iter().enumerate() {
                            let cap = c_bound(n);
                            let mut buf = vec![0x5Au8; cap + DST_SLACK];
                            let ret = cont(
                                stp,
                                sp.add(off) as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                n as c_int,
                                cap as c_int,
                                1,
                            );
                            t.push(rec(format!("continue[{}] n={}", i, n), ret as i64, &buf));
                            t.push(rec(
                                format!("continue[{}].state", i),
                                0,
                                std::slice::from_raw_parts(stp as *const u8, ss),
                            ));
                            if ret > 0 {
                                blocks.push((buf[..ret as usize].to_vec(), n, off));
                            }
                            off += n;
                        }
                        // decode with the same dictionary.  `LZ4_loadDict` keeps
                        // only the LAST 64 KB, so the decoder must be pointed at
                        // that same tail (`lz4.c:1621`).
                        let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                        let set = l.get::<Fn_setStreamDecode>("LZ4_setStreamDecode");
                        let dtail = dp.add(dsize - ld.max(0) as usize);
                        let sr = set(sd, dtail as *const c_char, ld);
                        t.push(rec("setStreamDecode", sr as i64, &[]));
                        let dec =
                            l.get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue");
                        let mut dst = vec![0u8; total + DST_SLACK];
                        for (i, (b, n, off)) in blocks.iter().enumerate() {
                            let ret = dec(
                                sd,
                                b.as_ptr() as *const c_char,
                                dst.as_mut_ptr().add(*off) as *mut c_char,
                                b.len() as c_int,
                                (total - *off) as c_int,
                            );
                            t.push(rec(format!("decode[{}]", i), ret as i64, &[]));
                            assert_eq!(ret, *n as c_int, "{} extdict decode", l.which);
                        }
                        l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
                        assert_eq!(&dst[..total], &src[..], "{}: extdict round-trip", l.which);
                    }
                    t
                };
                let ct = run(c());
                let rt = run(r());
                cmp_traces(
                    &format!(
                        "extdict shape={:?} dsize={} loadmode={}",
                        shape, dsize, mode
                    ),
                    &ct,
                    &rt,
                );
            }
        }
    }
}

/// dictCtx regime: `LZ4_attach_dictionary` against a separately loaded stream.
/// Both streams live in shared `Aligned` buffers so the `dictCtx` pointer
/// stored in the working state is identical in the C and Rust runs.
#[test]
fn f_stream_dictctx_attach_dictionary() {
    let ss = sizeof_state();
    let dst_stream = Aligned::new(ss, 8);
    let work = Aligned::new(ss, 8);
    let dsp = dst_stream.ptr();
    let wp = work.ptr();
    let mut rng = Rng::new(0x5EED_5003);
    for &shape in ALL_SHAPES {
        for &dsize in DICT_SIZES {
            let dict = gen(&mut rng, dsize, shape);
            // inputs both below and above the 4 KB dictCtx-copy threshold
            for &total in &[100usize, 3000, 4096, 4097, 20_000] {
                let src = gen(&mut rng, total, shape);
                let dp = dict.as_ptr();
                let sp = src.as_ptr();
                for attach_null in [false, true] {
                    let run = |l: &'static Lib| -> Vec<Rec> {
                        zero_at(dsp, ss);
                        zero_at(wp, ss);
                        let mut t = Vec::new();
                        unsafe {
                            let init = l.get::<Fn_initStream>("LZ4_initStream");
                            assert!(!init(dsp, ss).is_null());
                            assert!(!init(wp, ss).is_null());
                            let ld = l.get::<Fn_loadDict>("LZ4_loadDict")(
                                dsp,
                                dp as *const c_char,
                                dsize as c_int,
                            );
                            t.push(rec("loadDict", ld as i64, &[]));
                            let att = l.get::<Fn_attach_dictionary>("LZ4_attach_dictionary");
                            if attach_null {
                                att(wp, std::ptr::null());
                            } else {
                                att(wp, dsp as *const c_void);
                            }
                            t.push(rec(
                                "attach.state",
                                0,
                                std::slice::from_raw_parts(wp as *const u8, ss),
                            ));
                            let cont =
                                l.get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
                            // two blocks: the dictCtx is cleared after the first
                            let mut blocks: Vec<(Vec<u8>, usize, usize)> = Vec::new();
                            let half = total / 2;
                            for (i, (off, n)) in
                                [(0usize, half), (half, total - half)].iter().enumerate()
                            {
                                let cap = c_bound(*n);
                                let mut buf = vec![0x5Au8; cap + DST_SLACK];
                                let ret = cont(
                                    wp,
                                    sp.add(*off) as *const c_char,
                                    buf.as_mut_ptr() as *mut c_char,
                                    *n as c_int,
                                    cap as c_int,
                                    1,
                                );
                                t.push(rec(format!("continue[{}]", i), ret as i64, &buf));
                                t.push(rec(
                                    format!("continue[{}].state", i),
                                    0,
                                    std::slice::from_raw_parts(wp as *const u8, ss),
                                ));
                                if ret > 0 {
                                    blocks.push((buf[..ret as usize].to_vec(), *n, *off));
                                }
                            }
                            let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                            // only the last 64 KB of the dictionary were loaded
                            let dsz = if attach_null { 0 } else { ld };
                            l.get::<Fn_setStreamDecode>("LZ4_setStreamDecode")(
                                sd,
                                dp.add(dsize - dsz.max(0) as usize) as *const c_char,
                                dsz,
                            );
                            let dec = l
                                .get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue");
                            let mut dst = vec![0u8; total + DST_SLACK];
                            for (i, (b, n, off)) in blocks.iter().enumerate() {
                                let ret = dec(
                                    sd,
                                    b.as_ptr() as *const c_char,
                                    dst.as_mut_ptr().add(*off) as *mut c_char,
                                    b.len() as c_int,
                                    (total - *off) as c_int,
                                );
                                t.push(rec(format!("decode[{}]", i), ret as i64, &[]));
                                assert_eq!(ret, *n as c_int, "{}: dictCtx decode", l.which);
                            }
                            l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
                            assert_eq!(&dst[..total], &src[..], "{}: dictCtx round-trip", l.which);
                        }
                        t
                    };
                    let ct = run(c());
                    let rt = run(r());
                    cmp_traces(
                        &format!(
                            "dictCtx shape={:?} dsize={} total={} attach_null={}",
                            shape, dsize, total, attach_null
                        ),
                        &ct,
                        &rt,
                    );
                }
            }
        }
    }
}

/// `LZ4_resetStream` / `LZ4_resetStream_fast` / `LZ4_initStream` on the same
/// buffer, interleaved with compressions.
#[test]
fn f_stream_reset_variants() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_5004);
    for &shape in ALL_SHAPES {
        for &len in &[0usize, 1, 13, 100, 4096, 65535, 65547, 131072] {
            let a = gen(&mut rng, len, shape);
            let b = gen(&mut rng, len, shape);
            let ap = a.as_ptr();
            let bp = b.as_ptr();
            for reset in ["LZ4_resetStream", "LZ4_resetStream_fast", "LZ4_initStream"] {
                let run = |l: &'static Lib| -> Vec<Rec> {
                    zero_at(stp, ss);
                    let mut t = Vec::new();
                    unsafe {
                        assert!(!l.get::<Fn_initStream>("LZ4_initStream")(stp, ss).is_null());
                        let cont =
                            l.get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
                        let cap = c_bound(len);
                        for (i, p) in [ap, bp, ap].iter().enumerate() {
                            if i > 0 {
                                match reset {
                                    "LZ4_initStream" => {
                                        let ok = l.get::<Fn_initStream>("LZ4_initStream")(stp, ss);
                                        t.push(rec("initStream", !ok.is_null() as i64, &[]));
                                    }
                                    other => {
                                        l.get::<Fn_resetStream>(other)(stp);
                                        t.push(rec(other, 0, &[]));
                                    }
                                }
                                t.push(rec(
                                    format!("{}.state", reset),
                                    0,
                                    std::slice::from_raw_parts(stp as *const u8, ss),
                                ));
                            }
                            let mut buf = vec![0x5Au8; cap + DST_SLACK];
                            let ret = cont(
                                stp,
                                *p as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                1,
                            );
                            t.push(rec(format!("continue[{}]", i), ret as i64, &buf));
                            t.push(rec(
                                format!("continue[{}].state", i),
                                0,
                                std::slice::from_raw_parts(stp as *const u8, ss),
                            ));
                        }
                    }
                    t
                };
                let ct = run(c());
                let rt = run(r());
                cmp_traces(
                    &format!("reset={} shape={:?} len={}", reset, shape, len),
                    &ct,
                    &rt,
                );
            }
        }
    }
}

/// `LZ4_saveDict` after each round; sizes cover 0, tiny, and > 64 KB (clamped).
#[test]
fn f_stream_savedict_sweep() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    const SAFE: usize = 100_000 + 64;
    let safe_buf = Aligned::new(SAFE, 8);
    let sbp = safe_buf.ptr();
    let mut rng = Rng::new(0x5EED_5005);
    for &shape in ALL_SHAPES {
        for &save in &[0usize, 1, 4, 100, 65536, 100_000] {
            for &len in &[13usize, 100, 4096, 65536, 70_000] {
                let src = gen(&mut rng, len * 3, shape);
                let sp = src.as_ptr();
                let run = |l: &'static Lib| -> Vec<Rec> {
                    zero_at(stp, ss);
                    zero_at(sbp, SAFE);
                    let mut t = Vec::new();
                    unsafe {
                        assert!(!l.get::<Fn_initStream>("LZ4_initStream")(stp, ss).is_null());
                        let cont =
                            l.get::<Fn_compress_fast_continue>("LZ4_compress_fast_continue");
                        let sd = l.get::<Fn_saveDict>("LZ4_saveDict");
                        let cap = c_bound(len);
                        for i in 0..3 {
                            let mut buf = vec![0x5Au8; cap + DST_SLACK];
                            let ret = cont(
                                stp,
                                sp.add(i * len) as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                1,
                            );
                            t.push(rec(format!("continue[{}]", i), ret as i64, &buf));
                            let sv = sd(stp, sbp as *mut c_char, save as c_int);
                            t.push(rec(
                                format!("saveDict[{}]", i),
                                sv as i64,
                                std::slice::from_raw_parts(sbp as *const u8, SAFE),
                            ));
                            t.push(rec(
                                format!("saveDict[{}].state", i),
                                0,
                                std::slice::from_raw_parts(stp as *const u8, ss),
                            ));
                        }
                    }
                    t
                };
                let ct = run(c());
                let rt = run(r());
                cmp_traces(
                    &format!("saveDict shape={:?} save={} len={}", shape, save, len),
                    &ct,
                    &rt,
                );
            }
        }
    }
}

/// `LZ4_compress_forceExtDict` — the hidden debug entry point that always takes
/// the `usingExtDict` path (`lz4.c:1787`).
#[test]
fn f_compress_force_ext_dict() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_5006);
    for &shape in ALL_SHAPES {
        for &dsize in &[0usize, 8, 100, 65535, 65536] {
            for &len in &[0usize, 1, 13, 100, 4096, 65536] {
                let dict = gen(&mut rng, dsize, shape);
                let src = gen(&mut rng, len * 2, shape);
                let dp = dict.as_ptr();
                let sp = src.as_ptr();
                let run = |l: &'static Lib| -> Vec<Rec> {
                    zero_at(stp, ss);
                    let mut t = Vec::new();
                    unsafe {
                        assert!(!l.get::<Fn_initStream>("LZ4_initStream")(stp, ss).is_null());
                        let ld = l.get::<Fn_loadDict>("LZ4_loadDict")(
                            stp,
                            dp as *const c_char,
                            dsize as c_int,
                        );
                        t.push(rec("loadDict", ld as i64, &[]));
                        let f = l.get::<Fn_compress_forceExtDict>("LZ4_compress_forceExtDict");
                        for i in 0..2 {
                            let cap = c_bound(len);
                            let mut buf = vec![0x5Au8; cap + DST_SLACK];
                            let ret = f(
                                stp,
                                sp.add(i * len) as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                len as c_int,
                            );
                            t.push(rec(format!("forceExtDict[{}]", i), ret as i64, &buf));
                            t.push(rec(
                                format!("forceExtDict[{}].state", i),
                                0,
                                std::slice::from_raw_parts(stp as *const u8, ss),
                            ));
                        }
                    }
                    t
                };
                let ct = run(c());
                let rt = run(r());
                cmp_traces(
                    &format!("forceExtDict shape={:?} dsize={} len={}", shape, dsize, len),
                    &ct,
                    &rt,
                );
            }
        }
    }
}

// ===========================================================================
// G.  streaming decompression
// ===========================================================================

#[test]
fn g_streamdecode_linear_buffer() {
    let mut rng = Rng::new(0x5EED_6001);
    for &shape in ALL_SHAPES {
        for nchunks in 1..=8usize {
            let total = 150_000;
            let src = gen(&mut rng, total, shape);
            let plan = chunk_plan(&mut rng, total, nchunks);
            let blocks = c_stream_blocks(None, &src, &plan);
            let planr = plan.clone();
            let blocksr = blocks.clone();
            let srcr = src.clone();
            let run = |l: &'static Lib| -> Vec<Rec> {
                let mut t = Vec::new();
                unsafe {
                    let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                    assert!(!sd.is_null());
                    let dec = l.get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue");
                    let mut dst = vec![0u8; total + DST_SLACK];
                    let mut off = 0usize;
                    for (i, (b, &n)) in blocksr.iter().zip(planr.iter()).enumerate() {
                        let ret = dec(
                            sd,
                            b.as_ptr() as *const c_char,
                            dst.as_mut_ptr().add(off) as *mut c_char,
                            b.len() as c_int,
                            (total - off) as c_int,
                        );
                        t.push(rec(format!("safe_continue[{}]", i), ret as i64, &[]));
                        assert_eq!(ret, n as c_int, "{}", l.which);
                        off += n;
                    }
                    t.push(rec("decoded", 0, &dst));
                    assert_eq!(&dst[..total], &srcr[..], "{}: linear round-trip", l.which);
                    t.push(rec(
                        "freeStreamDecode",
                        l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd) as i64,
                        &[],
                    ));

                    // and the deprecated fast_continue variant
                    let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                    let decf =
                        l.get::<Fn_decompress_fast_continue>("LZ4_decompress_fast_continue");
                    let mut dst2 = vec![0u8; total + DST_SLACK];
                    let mut off = 0usize;
                    for (i, (b, &n)) in blocksr.iter().zip(planr.iter()).enumerate() {
                        let pb = padded(b);
                        let ret = decf(
                            sd,
                            pb.as_ptr() as *const c_char,
                            dst2.as_mut_ptr().add(off) as *mut c_char,
                            n as c_int,
                        );
                        t.push(rec(format!("fast_continue[{}]", i), ret as i64, &[]));
                        off += n;
                    }
                    t.push(rec("decoded_fast", 0, &dst2));
                    l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
                }
                t
            };
            let ct = run(c());
            let rt = run(r());
            cmp_traces(
                &format!("linear streamdecode shape={:?} nchunks={}", shape, nchunks),
                &ct,
                &rt,
            );
        }
    }
}

#[test]
fn g_streamdecode_ring_buffer() {
    let mut rng = Rng::new(0x5EED_6002);
    for &shape in ALL_SHAPES {
        for &maxblk in &[17usize, 64, 1024, 4096, 65536] {
            let nblk = 24usize;
            let total = maxblk * nblk;
            let src = gen(&mut rng, total, shape);
            let plan: Vec<usize> = (0..nblk)
                .map(|i| if i % 3 == 0 { maxblk } else { rng.range(1, maxblk) })
                .collect();
            let sum: usize = plan.iter().sum();
            let src = src[..sum].to_vec();
            let blocks = c_stream_blocks(None, &src, &plan);
            let rbs = unsafe {
                c().get::<Fn_decoderRingBufferSize>("LZ4_decoderRingBufferSize")(maxblk as c_int)
            } as usize;
            let (rbs_c, rbs_r) = both(|l| unsafe {
                l.get::<Fn_decoderRingBufferSize>("LZ4_decoderRingBufferSize")(maxblk as c_int)
            });
            assert_eq!(rbs_c, rbs_r, "decoderRingBufferSize({})", maxblk);
            let ring = Aligned::new(rbs, 8);
            let rp = ring.ptr();
            let planr = plan.clone();
            let blocksr = blocks.clone();
            let srcr = src.clone();
            let run = |l: &'static Lib| -> Vec<Rec> {
                zero_at(rp, rbs);
                let mut t = Vec::new();
                unsafe {
                    let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
                    let dec = l.get::<Fn_decompress_safe_continue>("LZ4_decompress_safe_continue");
                    let mut pos = 0usize;
                    let mut sofar = 0usize;
                    for (i, (b, &n)) in blocksr.iter().zip(planr.iter()).enumerate() {
                        if rbs - pos < maxblk {
                            pos = 0;
                        }
                        let ret = dec(
                            sd,
                            b.as_ptr() as *const c_char,
                            (rp as *mut u8).add(pos) as *mut c_char,
                            b.len() as c_int,
                            (rbs - pos) as c_int,
                        );
                        t.push(rec(format!("ring safe_continue[{}]", i), ret as i64, &[]));
                        assert_eq!(ret, n as c_int, "{}: ring decode", l.which);
                        let got = std::slice::from_raw_parts((rp as *const u8).add(pos), n);
                        assert_eq!(got, &srcr[sofar..sofar + n], "{}: ring payload", l.which);
                        t.push(rec(format!("ring payload[{}]", i), 0, got));
                        pos += n;
                        sofar += n;
                    }
                    l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
                    t.push(rec(
                        "ring final",
                        0,
                        std::slice::from_raw_parts(rp as *const u8, rbs),
                    ));
                }
                t
            };
            let ct = run(c());
            let rt = run(r());
            cmp_traces(
                &format!("ring streamdecode shape={:?} maxblk={}", shape, maxblk),
                &ct,
                &rt,
            );
        }
    }
}

/// `LZ4_setStreamDecode` with a dictionary / a zero-size dictionary / NULL
/// (ERRORS.md row 48 — always returns 1).
#[test]
fn g_set_stream_decode_forms() {
    let mut rng = Rng::new(0x5EED_6003);
    let dict = gen(&mut rng, 30_000, Shape::Text);
    for &(dp_null, dsize) in &[
        (true, 0usize),
        (false, 0),
        (false, 1),
        (false, 8),
        (false, 30_000),
    ] {
        let (cr, rr) = both(|l| unsafe {
            let sd = l.get::<Fn_createStreamDecode>("LZ4_createStreamDecode")();
            let p = if dp_null {
                std::ptr::null()
            } else {
                dict.as_ptr() as *const c_char
            };
            let ret = l.get::<Fn_setStreamDecode>("LZ4_setStreamDecode")(sd, p, dsize as c_int);
            l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(sd);
            ret
        });
        assert_eq!(
            cr, rr,
            "LZ4_setStreamDecode(null={}, size={}) C={} Rust={}",
            dp_null, dsize, cr, rr
        );
        assert_eq!(cr, 1, "C setStreamDecode should return 1");
    }
}

/// Stateless `*_usingDict` entry points in all three regimes:
/// no dictionary, dictionary adjacent to `dst` (prefix), and a separate buffer
/// (extDict) — `lz4.c:2719..2758`.
#[test]
fn g_decompress_using_dict_all_regimes() {
    let mut rng = Rng::new(0x5EED_6004);
    for &shape in ALL_SHAPES {
        for &dsize in &[0usize, 1, 8, 100, 65534, 65535, 65536, 70_000] {
            for &len in &[1usize, 13, 100, 4096, 65536] {
                // contiguous [dict | payload] so both the prefix and the extDict
                // layout describe the *same* logical history
                let whole = gen(&mut rng, dsize + len, shape);
                let dict = whole[..dsize].to_vec();
                let payload = whole[dsize..].to_vec();
                let blocks = if dsize == 0 {
                    vec![Vec::new(), c_compress(&payload)]
                } else {
                    c_stream_blocks(None, &whole, &[dsize, len])
                };
                let blk = padded(&blocks[1]);
                let bp = blk.as_ptr() as *const c_char;
                let clen = blocks[1].len() as c_int;

                // (a) prefix layout: dictStart + dictSize == dst
                {
                    let mut cd = vec![0u8; dsize + len + DST_SLACK];
                    let mut rd = vec![0u8; dsize + len + DST_SLACK];
                    cd[..dsize].copy_from_slice(&dict);
                    rd[..dsize].copy_from_slice(&dict);
                    let ctx = format!(
                        "safe_usingDict(prefix) shape={:?} dsize={} len={}",
                        shape, dsize, len
                    );
                    let cr = unsafe {
                        c().get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                            bp,
                            cd.as_mut_ptr().add(dsize) as *mut c_char,
                            clen,
                            len as c_int,
                            cd.as_ptr() as *const c_char,
                            dsize as c_int,
                        )
                    };
                    let rr = unsafe {
                        r().get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                            bp,
                            rd.as_mut_ptr().add(dsize) as *mut c_char,
                            clen,
                            len as c_int,
                            rd.as_ptr() as *const c_char,
                            dsize as c_int,
                        )
                    };
                    assert_eq!(cr, rr, "{}: C={} Rust={}", ctx, cr, rr);
                    assert_eq!(cr, len as c_int, "{}", ctx);
                    assert_eq!(&cd[dsize..dsize + len], &payload[..], "{}: payload", ctx);
                    if let Some(m) = diff_report(&ctx, &cd, &rd) {
                        panic!("{}", m);
                    }
                }

                // (b) extDict layout: dictionary in a separate buffer
                {
                    let mut cd = vec![0u8; len + DST_SLACK];
                    let mut rd = vec![0u8; len + DST_SLACK];
                    let dpp = dict.as_ptr() as *const c_char;
                    let ctx = format!(
                        "safe_usingDict(extDict) shape={:?} dsize={} len={}",
                        shape, dsize, len
                    );
                    dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                        l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                            bp,
                            dp,
                            clen,
                            len as c_int,
                            dpp,
                            dsize as c_int,
                        )
                    });
                    assert_eq!(&cd[..len], &payload[..], "{}: payload", ctx);

                    // fast_usingDict
                    let mut cd = vec![0u8; len + DST_SLACK];
                    let mut rd = vec![0u8; len + DST_SLACK];
                    let ctx = format!(
                        "fast_usingDict(extDict) shape={:?} dsize={} len={}",
                        shape, dsize, len
                    );
                    dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                        l.get::<Fn_decompress_fast_usingDict>("LZ4_decompress_fast_usingDict")(
                            bp,
                            dp,
                            len as c_int,
                            dpp,
                            dsize as c_int,
                        )
                    });
                    assert_eq!(&cd[..len], &payload[..], "{}: payload", ctx);

                    // partial_usingDict, sweeping targetOutputSize
                    for &target in &[0usize, 1, 5, 12, len / 4, len / 2, len, len + 10] {
                        for &cap in &[target, target + 1, target / 2, len] {
                            let alloc = target.max(cap).max(len) + DST_SLACK;
                            let mut cd = vec![0u8; alloc];
                            let mut rd = vec![0u8; alloc];
                            let ctx = format!(
                                "partial_usingDict shape={:?} dsize={} len={} target={} cap={}",
                                shape, dsize, len, target, cap
                            );
                            dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                                l.get::<Fn_decompress_partial_usingDict>(
                                    "LZ4_decompress_safe_partial_usingDict",
                                )(
                                    bp,
                                    dp,
                                    clen,
                                    target as c_int,
                                    cap as c_int,
                                    dpp,
                                    dsize as c_int,
                                )
                            });
                        }
                    }
                }

                // (c) NULL / zero dictionary
                {
                    let mut cd = vec![0u8; len + DST_SLACK];
                    let mut rd = vec![0u8; len + DST_SLACK];
                    let ctx = format!(
                        "safe_usingDict(null dict) shape={:?} dsize={} len={}",
                        shape, dsize, len
                    );
                    dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                        l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                            bp,
                            dp,
                            clen,
                            len as c_int,
                            std::ptr::null(),
                            0,
                        )
                    });
                }
            }
        }
    }
}

/// The internal `*_forceExtDict` entry points (`lz4.c:2523`, `2534`).
#[test]
fn g_decompress_force_ext_dict_variants() {
    let mut rng = Rng::new(0x5EED_6005);
    for &shape in ALL_SHAPES {
        for &dsize in &[0usize, 8, 100, 65535, 65536] {
            for &len in &[1usize, 13, 100, 4096, 65536] {
                let whole = gen(&mut rng, dsize + len, shape);
                let dict = whole[..dsize].to_vec();
                let payload = whole[dsize..].to_vec();
                let blocks = if dsize == 0 {
                    vec![Vec::new(), c_compress(&payload)]
                } else {
                    c_stream_blocks(None, &whole, &[dsize, len])
                };
                let blk = padded(&blocks[1]);
                let bp = blk.as_ptr() as *const c_char;
                let clen = blocks[1].len() as c_int;
                let dpp = dict.as_ptr() as *const c_void;

                for &cap in &[len, len + 1, len.saturating_sub(1), 0] {
                    let mut cd = vec![0u8; cap.max(len) + DST_SLACK];
                    let mut rd = vec![0u8; cap.max(len) + DST_SLACK];
                    let ctx = format!(
                        "safe_forceExtDict shape={:?} dsize={} len={} cap={}",
                        shape, dsize, len, cap
                    );
                    dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                        l.get::<Fn_decompress_safe_forceExtDict>(
                            "LZ4_decompress_safe_forceExtDict",
                        )(bp, dp, clen, cap as c_int, dpp, dsize)
                    });
                }
                for &target in &[0usize, 1, 12, len / 2, len, len + 5] {
                    for &cap in &[target, target + 1, len] {
                        let alloc = target.max(cap).max(len) + DST_SLACK;
                        let mut cd = vec![0u8; alloc];
                        let mut rd = vec![0u8; alloc];
                        let ctx = format!(
                            "partial_forceExtDict shape={:?} dsize={} len={} target={} cap={}",
                            shape, dsize, len, target, cap
                        );
                        dcmp(&ctx, &mut cd, &mut rd, |l, dp| unsafe {
                            l.get::<Fn_decompress_safe_partial_forceExtDict>(
                                "LZ4_decompress_safe_partial_forceExtDict",
                            )(bp, dp, clen, target as c_int, cap as c_int, dpp, dsize)
                        });
                    }
                }
            }
        }
    }
}

// ===========================================================================
// H.  deprecated / legacy family
// ===========================================================================

#[test]
fn h_deprecated_compress_family() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_7001);
    for &shape in ALL_SHAPES {
        for &len in SIZES_SMALL {
            let src = gen(&mut rng, len, shape);
            let sp = src.as_ptr() as *const c_char;
            let bound = c_bound(len);

            let (a, b) = dual(bound, |l, dp| unsafe {
                l.get::<Fn_compress>("LZ4_compress")(sp, dp, len as c_int)
            });
            expect_same(
                &format!("LZ4_compress len={} shape={:?}", len, shape),
                &a,
                &b,
            );

            for &cap in &[bound, a.ret.max(0) as usize, len, 1, 0] {
                let (x, y) = dual(cap, |l, dp| unsafe {
                    l.get::<Fn_compress_limitedOutput>("LZ4_compress_limitedOutput")(
                        sp,
                        dp,
                        len as c_int,
                        cap as c_int,
                    )
                });
                expect_same(
                    &format!(
                        "LZ4_compress_limitedOutput len={} shape={:?} cap={}",
                        len, shape, cap
                    ),
                    &x,
                    &y,
                );
            }

            // withState variants
            for lib_pass in 0..2 {
                let _ = lib_pass;
            }
            let mut outs: Vec<Out> = Vec::new();
            let mut states: Vec<Vec<u8>> = Vec::new();
            for lib in [c(), r()] {
                zero_at(stp, ss);
                let mut buf = vec![0x5Au8; bound + DST_SLACK];
                let ret = unsafe {
                    lib.get::<Fn_compress_withState>("LZ4_compress_withState")(
                        stp,
                        sp,
                        buf.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    )
                };
                states.push(st.bytes().to_vec());
                outs.push(Out { ret, buf });
            }
            let ctx = format!("LZ4_compress_withState len={} shape={:?}", len, shape);
            expect_same(&ctx, &outs[0], &outs[1]);
            if let Some(m) = diff_report(&format!("{} :: state", ctx), &states[0], &states[1]) {
                panic!("{}", m);
            }

            for &cap in &[bound, len, 1, 0] {
                let mut outs: Vec<Out> = Vec::new();
                let mut states: Vec<Vec<u8>> = Vec::new();
                for lib in [c(), r()] {
                    zero_at(stp, ss);
                    let mut buf = vec![0x5Au8; cap + DST_SLACK];
                    let ret = unsafe {
                        lib.get::<Fn_compress_limitedOutput_withState>(
                            "LZ4_compress_limitedOutput_withState",
                        )(
                            stp,
                            sp,
                            buf.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        )
                    };
                    states.push(st.bytes().to_vec());
                    outs.push(Out { ret, buf });
                }
                let ctx = format!(
                    "LZ4_compress_limitedOutput_withState len={} shape={:?} cap={}",
                    len, shape, cap
                );
                expect_same(&ctx, &outs[0], &outs[1]);
                if let Some(m) = diff_report(&format!("{} :: state", ctx), &states[0], &states[1]) {
                    panic!("{}", m);
                }
            }
        }
    }
}

#[test]
fn h_deprecated_stream_family() {
    let ss = sizeof_state();
    let sss = unsafe { c().get::<FnI>("LZ4_sizeofStreamState")() } as usize;
    assert_eq!(ss, sss, "sizeofState vs sizeofStreamState");
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let inbuf = Aligned::new(1 << 16, 8);
    let ibp = inbuf.ptr();
    let mut rng = Rng::new(0x5EED_7002);
    for &shape in ALL_SHAPES {
        for &len in &[0usize, 1, 13, 100, 4096, 65536] {
            let src = gen(&mut rng, len * 3, shape);
            let sp = src.as_ptr();
            let run = |l: &'static Lib| -> Vec<Rec> {
                zero_at(stp, ss);
                let mut t = Vec::new();
                unsafe {
                    // LZ4_create allocates its own stream from the library heap
                    let s = l.get::<Fn_create>("LZ4_create")(ibp as *mut c_char);
                    assert!(!s.is_null(), "LZ4_create returned NULL");
                    t.push(rec(
                        "create.state",
                        0,
                        std::slice::from_raw_parts(s as *const u8, ss),
                    ));
                    let cont = l.get::<Fn_compress_continue>("LZ4_compress_continue");
                    let lcont = l
                        .get::<Fn_compress_limitedOutput_continue>(
                            "LZ4_compress_limitedOutput_continue",
                        );
                    let cap = c_bound(len);
                    let mut blocks: Vec<(Vec<u8>, usize)> = Vec::new();
                    for i in 0..3 {
                        let mut buf = vec![0x5Au8; cap + DST_SLACK];
                        let ret = if i % 2 == 0 {
                            cont(
                                s,
                                sp.add(i * len) as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                len as c_int,
                            )
                        } else {
                            lcont(
                                s,
                                sp.add(i * len) as *const c_char,
                                buf.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                            )
                        };
                        t.push(rec(format!("continue[{}]", i), ret as i64, &buf));
                        if ret > 0 {
                            blocks.push((buf[..ret as usize].to_vec(), len));
                        }
                    }
                    // LZ4_slideInputBuffer returns the tracked dictionary pointer;
                    // it must be the very same address in both libraries.
                    let slide = l.get::<Fn_slideInputBuffer>("LZ4_slideInputBuffer")(s);
                    t.push(rec("slideInputBuffer", slide as usize as i64, &[]));
                    t.push(rec(
                        "post-slide.state",
                        0,
                        std::slice::from_raw_parts(s as *const u8, ss),
                    ));
                    t.push(rec(
                        "freeStream",
                        l.get::<Fn_freeStream>("LZ4_freeStream")(s) as i64,
                        &[],
                    ));

                    // LZ4_resetStreamState on a caller-provided buffer
                    let rr =
                        l.get::<Fn_resetStreamState>("LZ4_resetStreamState")(stp, ibp as *mut c_char);
                    t.push(rec(
                        "resetStreamState",
                        rr as i64,
                        std::slice::from_raw_parts(stp as *const u8, ss),
                    ));

                    // LZ4_uncompress / LZ4_uncompress_unknownOutputSize
                    for (i, (b, n)) in blocks.iter().enumerate() {
                        let pb = padded(b);
                        let mut d1 = vec![0x5Au8; *n + DST_SLACK];
                        let r1 = l.get::<Fn_uncompress>("LZ4_uncompress")(
                            pb.as_ptr() as *const c_char,
                            d1.as_mut_ptr() as *mut c_char,
                            *n as c_int,
                        );
                        t.push(rec(format!("uncompress[{}]", i), r1 as i64, &d1));
                        let mut d2 = vec![0x5Au8; *n + DST_SLACK];
                        let r2 = l
                            .get::<Fn_uncompress_unknownOutputSize>(
                                "LZ4_uncompress_unknownOutputSize",
                            )(
                            b.as_ptr() as *const c_char,
                            d2.as_mut_ptr() as *mut c_char,
                            b.len() as c_int,
                            (*n + DST_SLACK) as c_int,
                        );
                        t.push(rec(format!("uncompress_unknown[{}]", i), r2 as i64, &d2));
                    }
                }
                t
            };
            let ct = run(c());
            let rt = run(r());
            cmp_traces(
                &format!("deprecated stream shape={:?} len={}", shape, len),
                &ct,
                &rt,
            );
        }
    }
}

// ===========================================================================
// I.  error paths
// ===========================================================================

/// ERRORS.md row 2: `(U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE` is rejected at
/// `lz4.c:1360`, i.e. *before* any access to `src`, so a tiny real buffer with
/// an absurd `srcSize` is safe to pass.
#[test]
fn i_err_compress_srcsize_out_of_range() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let src = vec![0x42u8; 64];
    let sp = src.as_ptr() as *const c_char;
    for &bad in &[
        0x7E00_0001i32,
        0x7FFF_FFFF,
        c_int::MAX,
        -1,
        -64,
        c_int::MIN,
        c_int::MIN + 1,
    ] {
        for &cap in &[0usize, 1, 64, 4096] {
            let (a, b) = dual(cap, |l, dp| unsafe {
                l.get::<Fn_compress_default>("LZ4_compress_default")(sp, dp, bad, cap as c_int)
            });
            let ctx = format!("LZ4_compress_default srcSize={} cap={}", bad, cap);
            expect_same(&ctx, &a, &b);
            assert_eq!(a.ret, 0, "{}: C should reject", ctx);

            let (a, b) = dual(cap, |l, dp| unsafe {
                l.get::<Fn_compress_fast>("LZ4_compress_fast")(sp, dp, bad, cap as c_int, 1)
            });
            expect_same(
                &format!("LZ4_compress_fast srcSize={} cap={}", bad, cap),
                &a,
                &b,
            );
            assert_eq!(a.ret, 0);

            let mut outs: Vec<Out> = Vec::new();
            for lib in [c(), r()] {
                zero_at(st.ptr(), ss);
                let mut buf = vec![0x5Au8; cap + DST_SLACK];
                let ret = unsafe {
                    lib.get::<Fn_compress_fast_extState>("LZ4_compress_fast_extState")(
                        st.ptr(),
                        sp,
                        buf.as_mut_ptr() as *mut c_char,
                        bad,
                        cap as c_int,
                        1,
                    )
                };
                outs.push(Out { ret, buf });
            }
            expect_same(
                &format!("LZ4_compress_fast_extState srcSize={} cap={}", bad, cap),
                &outs[0],
                &outs[1],
            );
            assert_eq!(outs[0].ret, 0);
        }

        // destSize: *srcSizePtr out of range
        let mut cb = vec![0x5Au8; 4096];
        let mut rb = vec![0x5Au8; 4096];
        let mut cs = bad;
        let mut rs = bad;
        let cret = unsafe {
            c().get::<Fn_compress_destSize>("LZ4_compress_destSize")(
                sp,
                cb.as_mut_ptr() as *mut c_char,
                &mut cs,
                4096,
            )
        };
        let rret = unsafe {
            r().get::<Fn_compress_destSize>("LZ4_compress_destSize")(
                sp,
                rb.as_mut_ptr() as *mut c_char,
                &mut rs,
                4096,
            )
        };
        assert_eq!(
            cret, rret,
            "LZ4_compress_destSize *srcSizePtr={}: C={} Rust={}",
            bad, cret, rret
        );
        assert_eq!(cs, rs, "LZ4_compress_destSize *srcSizePtr={} out", bad);
        assert_eq!(cret, 0, "C should reject srcSize={}", bad);
    }
}

/// ERRORS.md rows 9, 10, 11 (`LZ4_initStream` NULL / undersized / misaligned)
/// and row 12 (`LZ4_freeStream(NULL)`), plus row 44
/// (`LZ4_freeStreamDecode(NULL)`).
#[test]
fn i_err_init_stream_and_free_null() {
    let ss = sizeof_state();
    let st = Aligned::new(ss + 16, 8);

    let (cp, rp) = both(|l| unsafe {
        l.get::<Fn_initStream>("LZ4_initStream")(std::ptr::null_mut(), ss)
    });
    assert!(cp.is_null(), "C initStream(NULL) should be NULL");
    assert!(rp.is_null(), "Rust initStream(NULL) should be NULL");

    for size in [0usize, 1, ss - 1] {
        let (cp, rp) = both(|l| unsafe { l.get::<Fn_initStream>("LZ4_initStream")(st.ptr(), size) });
        assert!(cp.is_null(), "C initStream(size={}) should be NULL", size);
        assert!(rp.is_null(), "Rust initStream(size={}) should be NULL", size);
    }

    let (cp, rp) =
        both(|l| unsafe { l.get::<Fn_initStream>("LZ4_initStream")(st.misaligned(), ss) });
    assert!(cp.is_null(), "C initStream(misaligned) should be NULL");
    assert!(rp.is_null(), "Rust initStream(misaligned) should be NULL");

    // success returns the buffer itself
    let (cp, rp) = both(|l| unsafe { l.get::<Fn_initStream>("LZ4_initStream")(st.ptr(), ss) });
    assert_eq!(cp, st.ptr(), "C initStream should return its buffer");
    assert_eq!(rp, st.ptr(), "Rust initStream should return its buffer");
    // and larger sizes are accepted too
    let (cp, rp) = both(|l| unsafe { l.get::<Fn_initStream>("LZ4_initStream")(st.ptr(), ss + 16) });
    assert_eq!(cp, rp);

    let (cf, rf) =
        both(|l| unsafe { l.get::<Fn_freeStream>("LZ4_freeStream")(std::ptr::null_mut()) });
    assert_eq!((cf, rf), (0, 0), "LZ4_freeStream(NULL)");
    let (cf, rf) = both(|l| unsafe {
        l.get::<Fn_freeStreamDecode>("LZ4_freeStreamDecode")(std::ptr::null_mut())
    });
    assert_eq!((cf, rf), (0, 0), "LZ4_freeStreamDecode(NULL)");

    // createStream / freeStream round-trip: fresh state must be all-zero and
    // identical between the two libraries
    let mut snaps: Vec<Vec<u8>> = Vec::new();
    for lib in [c(), r()] {
        unsafe {
            let s = lib.get::<Fn_createStream>("LZ4_createStream")();
            assert!(!s.is_null());
            snaps.push(std::slice::from_raw_parts(s as *const u8, ss).to_vec());
            assert_eq!(lib.get::<Fn_freeStream>("LZ4_freeStream")(s), 0);
        }
    }
    if let Some(m) = diff_report("LZ4_createStream initial state", &snaps[0], &snaps[1]) {
        panic!("{}", m);
    }
    assert!(
        snaps[0].iter().all(|&b| b == 0),
        "createStream state not zeroed"
    );
}

/// ERRORS.md rows 13, 14: `dictSize < HASH_UNIT` (8) and NULL dictionary are
/// silently rejected (return 0) — but the `currentOffset += 64 KB` side effect
/// still happens, so the resulting state bytes are compared too.
#[test]
fn i_err_load_dict_rejections() {
    let ss = sizeof_state();
    let st = Aligned::new(ss, 8);
    let stp = st.ptr();
    let mut rng = Rng::new(0x5EED_8001);
    let dict = gen(&mut rng, 1000, Shape::Text);
    for name in ["LZ4_loadDict", "LZ4_loadDictSlow"] {
        for &(null, dsize) in &[
            (true, 0i32),
            (true, 1),
            (true, 4),
            (true, 7),
            (true, -1),
            (false, 0),
            (false, 1),
            (false, 4),
            (false, 7),
            (false, -1),
            (false, -1000),
            (false, 8),
            (false, 9),
            (false, 1000),
        ] {
            let mut rets: Vec<(c_int, Vec<u8>)> = Vec::new();
            for lib in [c(), r()] {
                zero_at(stp, ss);
                unsafe {
                    assert!(!lib.get::<Fn_initStream>("LZ4_initStream")(stp, ss).is_null());
                    let p = if null {
                        std::ptr::null()
                    } else {
                        dict.as_ptr() as *const c_char
                    };
                    let ret = lib.get::<Fn_loadDict>(name)(stp, p, dsize);
                    rets.push((ret, st.bytes().to_vec()));
                }
            }
            let ctx = format!("{}(null={}, dictSize={})", name, null, dsize);
            assert_eq!(
                rets[0].0, rets[1].0,
                "{}: C={} Rust={}",
                ctx, rets[0].0, rets[1].0
            );
            if let Some(m) = diff_report(&format!("{} :: state", ctx), &rets[0].1, &rets[1].1) {
                panic!("{}", m);
            }
            if dsize < 8 {
                assert_eq!(rets[0].0, 0, "{}: C should reject", ctx);
            }
        }
    }
}

/// ERRORS.md rows 24, 25, 26, 27: the degenerate `LZ4_decompress_safe` inputs.
#[test]
fn i_err_decompress_safe_degenerate() {
    let mut dst = vec![0u8; 4096];
    let dp: *mut c_char = dst.as_mut_ptr() as *mut c_char;
    let src = vec![0u8; 16];
    let sp = src.as_ptr() as *const c_char;

    // src == NULL  -> -1
    for &(n, m) in &[(0i32, 0i32), (0, 10), (1, 10), (5, 100), (-1, 10)] {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                std::ptr::null(),
                dp,
                n,
                m,
            )
        });
        assert_eq!(cv, rv, "decompress_safe(NULL,{},{})", n, m);
        assert_eq!(cv, -1, "C decompress_safe(NULL,{},{}) should be -1", n, m);
    }

    // outputSize < 0 -> -1
    for &m in &[-1i32, -100, c_int::MIN] {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                sp,
                dp,
                1,
                m,
            )
        });
        assert_eq!(cv, rv, "decompress_safe(outputSize={})", m);
        assert_eq!(cv, -1, "C should be -1 for outputSize={}", m);
    }

    // srcSize == 0 with outputSize > 0 -> -1
    for &m in &[1i32, 10, 4096] {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                sp,
                dp,
                0,
                m,
            )
        });
        assert_eq!(cv, rv, "decompress_safe(srcSize=0, out={})", m);
        assert_eq!(cv, -1, "C should be -1");
    }

    // outputSize == 0: only (srcSize==1 && src[0]==0) is the valid empty block
    let zero = [0u8, 0, 0, 0];
    let nonzero = [1u8, 2, 3, 4];
    for (label, buf, n, expect) in [
        ("srcSize=1 src[0]=0", &zero[..], 1i32, 0i32),
        ("srcSize=1 src[0]=1", &nonzero[..], 1, -1),
        ("srcSize=2 zeros", &zero[..], 2, -1),
        ("srcSize=0", &zero[..], 0, -1),
        ("srcSize=4 zeros", &zero[..], 4, -1),
    ] {
        let bp = buf.as_ptr() as *const c_char;
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                bp,
                dp,
                n,
                0,
            )
        });
        assert_eq!(cv, rv, "decompress_safe(outputSize=0, {})", label);
        assert_eq!(cv, expect, "C decompress_safe(outputSize=0, {})", label);
    }

    // partial decoding with outputSize == 0 always returns 0
    for &n in &[0i32, 1, 2, 4] {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_decompress_safe_partial>("LZ4_decompress_safe_partial")(
                sp,
                dp,
                n,
                0,
                0,
            )
        });
        assert_eq!(cv, rv, "decompress_safe_partial(target=0, srcSize={})", n);
    }
}

// ---------------------------------------------------------------------------
// I.  corrupt / truncated input fuzzing -- the exact `_output_error` value
//     encodes how far into the input the error was found, so exact int
//     equality is what matters (ERRORS.md rows 15-23 and 28-43).
// ---------------------------------------------------------------------------

/// The four decoders under test, applied to one (possibly corrupt) input.
fn fuzz_all_decoders(ctx: &str, inp: &[u8], srclen: c_int, orig: usize) {
    let padded_in = padded(inp);
    let bp = padded_in.as_ptr() as *const c_char;
    let cap = orig + 16;
    let mut cd = vec![0u8; cap + DST_SLACK];
    let mut rd = vec![0u8; cap + DST_SLACK];

    dcmp(&format!("{} safe", ctx), &mut cd, &mut rd, |l, dp| unsafe {
        l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(bp, dp, srclen, cap as c_int)
    });
    dcmp(
        &format!("{} safe(exact cap)", ctx),
        &mut cd[..orig + DST_SLACK],
        &mut rd[..orig + DST_SLACK],
        |l, dp| unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(bp, dp, srclen, orig as c_int)
        },
    );
    for &t in &[0usize, 1, orig / 2, orig, orig + 8] {
        dcmp(
            &format!("{} partial(target={})", ctx, t),
            &mut cd,
            &mut rd,
            |l, dp| unsafe {
                l.get::<Fn_decompress_safe_partial>("LZ4_decompress_safe_partial")(
                    bp,
                    dp,
                    srclen,
                    t as c_int,
                    cap as c_int,
                )
            },
        );
    }
    dcmp(&format!("{} fast", ctx), &mut cd, &mut rd, |l, dp| unsafe {
        l.get::<Fn_decompress_fast>("LZ4_decompress_fast")(bp, dp, orig as c_int)
    });
}

#[test]
fn i_fuzz_truncated_blocks() {
    let mut rng = Rng::new(0x5EED_9001);
    for &shape in ALL_SHAPES {
        for &len in &[1usize, 5, 13, 40, 200, 700, 2000] {
            let src = gen(&mut rng, len, shape);
            let blk = c_compress(&src);
            for cut in 0..=blk.len() {
                let ctx = format!(
                    "truncated shape={:?} len={} clen={} cut={}",
                    shape, len, blk.len(), cut
                );
                fuzz_all_decoders(&ctx, &blk[..cut], cut as c_int, len);
            }
        }
    }
}

#[test]
fn i_fuzz_bitflips_and_byte_smashes() {
    let mut rng = Rng::new(0x5EED_9002);
    let mut iters = 0usize;
    for &shape in ALL_SHAPES {
        for &len in &[13usize, 40, 200, 700, 2000] {
            let src = gen(&mut rng, len, shape);
            let blk = c_compress(&src);
            for _ in 0..120 {
                let mut bad = blk.clone();
                let nmut = rng.range(1, 3);
                for _ in 0..nmut {
                    let pos = rng.below(bad.len());
                    if rng.bool() {
                        bad[pos] ^= 1u8 << rng.below(8);
                    } else {
                        bad[pos] = rng.byte();
                    }
                }
                // also vary the declared srcSize around the true length
                let lens: [c_int; 3] = [
                    bad.len() as c_int,
                    (bad.len() as c_int - 1).max(0),
                    bad.len() as c_int,
                ];
                let sl = lens[rng.below(3)];
                let ctx = format!(
                    "bitflip shape={:?} len={} srcSize={} iter={}",
                    shape, len, sl, iters
                );
                fuzz_all_decoders(&ctx, &bad, sl, len);
                iters += 1;
            }
        }
    }
    assert!(iters >= 4000, "expected several thousand iterations, got {}", iters);
}

#[test]
fn i_fuzz_random_bytes_as_compressed() {
    let mut rng = Rng::new(0x5EED_9003);
    for i in 0..2500usize {
        let n = rng.range(0, 300);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let junk = gen(&mut rng, n, shape);
        let orig = rng.range(0, 1500);
        let ctx = format!("random-junk iter={} n={} orig={} shape={:?}", i, n, orig, shape);
        fuzz_all_decoders(&ctx, &junk, n as c_int, orig);
    }
}

/// The `usingDict` / `forceExtDict` decoders on corrupt input, in both the
/// prefix-adjacent and separate-buffer dictionary layouts.
#[test]
fn i_fuzz_corrupt_using_dict() {
    let mut rng = Rng::new(0x5EED_9004);
    let dict = gen(&mut rng, 40_000, Shape::Text);
    let dpc = dict.as_ptr() as *const c_char;
    let dpv = dict.as_ptr() as *const c_void;
    for &shape in ALL_SHAPES {
        for &len in &[13usize, 200, 2000] {
            let whole = {
                let mut w = dict.clone();
                w.extend_from_slice(&gen(&mut rng, len, shape));
                w
            };
            let blocks = c_stream_blocks(None, &whole, &[dict.len(), len]);
            let base = blocks[1].clone();
            for it in 0..120usize {
                let mut bad = base.clone();
                if it > 0 {
                    let nmut = rng.range(1, 3);
                    for _ in 0..nmut {
                        let pos = rng.below(bad.len());
                        bad[pos] ^= 1u8 << rng.below(8);
                    }
                }
                let cut = if it % 4 == 0 {
                    rng.range(0, bad.len())
                } else {
                    bad.len()
                };
                let inp = padded(&bad[..cut]);
                let bp = inp.as_ptr() as *const c_char;
                let sl = cut as c_int;
                let cap = len + 16;
                let mut cd = vec![0u8; cap + DST_SLACK];
                let mut rd = vec![0u8; cap + DST_SLACK];
                let ctx = format!(
                    "corrupt usingDict shape={:?} len={} it={} cut={}",
                    shape, len, it, cut
                );

                dcmp(&format!("{} safe_usingDict", ctx), &mut cd, &mut rd, |l, dp| unsafe {
                    l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                        bp,
                        dp,
                        sl,
                        cap as c_int,
                        dpc,
                        dict.len() as c_int,
                    )
                });
                dcmp(&format!("{} fast_usingDict", ctx), &mut cd, &mut rd, |l, dp| unsafe {
                    l.get::<Fn_decompress_fast_usingDict>("LZ4_decompress_fast_usingDict")(
                        bp,
                        dp,
                        len as c_int,
                        dpc,
                        dict.len() as c_int,
                    )
                });
                for &t in &[0usize, 1, len / 2, len, len + 8] {
                    dcmp(
                        &format!("{} partial_usingDict target={}", ctx, t),
                        &mut cd,
                        &mut rd,
                        |l, dp| unsafe {
                            l.get::<Fn_decompress_partial_usingDict>(
                                "LZ4_decompress_safe_partial_usingDict",
                            )(bp, dp, sl, t as c_int, cap as c_int, dpc, dict.len() as c_int)
                        },
                    );
                }
                dcmp(
                    &format!("{} safe_forceExtDict", ctx),
                    &mut cd,
                    &mut rd,
                    |l, dp| unsafe {
                        l.get::<Fn_decompress_safe_forceExtDict>(
                            "LZ4_decompress_safe_forceExtDict",
                        )(bp, dp, sl, cap as c_int, dpv, dict.len())
                    },
                );
                dcmp(
                    &format!("{} partial_forceExtDict", ctx),
                    &mut cd,
                    &mut rd,
                    |l, dp| unsafe {
                        l.get::<Fn_decompress_safe_partial_forceExtDict>(
                            "LZ4_decompress_safe_partial_forceExtDict",
                        )(bp, dp, sl, len as c_int, cap as c_int, dpv, dict.len())
                    },
                );

                // prefix-adjacent layout (dictStart + dictSize == dst)
                let pfx = 4096usize;
                let mut cd2 = vec![0u8; pfx + cap + DST_SLACK];
                let mut rd2 = vec![0u8; pfx + cap + DST_SLACK];
                cd2[..pfx].copy_from_slice(&dict[..pfx]);
                rd2[..pfx].copy_from_slice(&dict[..pfx]);
                let cr = unsafe {
                    c().get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                        bp,
                        cd2.as_mut_ptr().add(pfx) as *mut c_char,
                        sl,
                        cap as c_int,
                        cd2.as_ptr() as *const c_char,
                        pfx as c_int,
                    )
                };
                let rr = unsafe {
                    r().get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                        bp,
                        rd2.as_mut_ptr().add(pfx) as *mut c_char,
                        sl,
                        cap as c_int,
                        rd2.as_ptr() as *const c_char,
                        pfx as c_int,
                    )
                };
                assert_eq!(
                    cr, rr,
                    "{} safe_usingDict(prefix): C={} Rust={}",
                    ctx, cr, rr
                );
                if let Some(m) = diff_report(&format!("{} safe_usingDict(prefix)", ctx), &cd2, &rd2)
                {
                    panic!("{}", m);
                }
            }
        }
    }
}
