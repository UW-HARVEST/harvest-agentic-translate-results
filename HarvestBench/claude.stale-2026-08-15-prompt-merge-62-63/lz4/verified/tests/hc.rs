//! CONFIGS.md rows 70-99 — lz4hc.c HIGH-COMPRESSION API parity.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols and compares the return code AND the produced bytes.
//!
//! Compression-level classes the C branches on (`LZ4HC_getCLevelParams`):
//!   * `cLevel < 1`  -> clamped to `LZ4HC_CLEVEL_DEFAULT` (9)
//!   * `cLevel > 12` -> clamped to `LZ4HC_CLEVEL_MAX` (12)
//!   * `cLevel == 1` is ACCEPTED as-is (it is *not* raised to 2)
//!   * levels 1-2 => `lz4mid`, 3-9 => HC hash chain, 10-12 => optimal parser
//!   * `favorDecSpeed` is only consulted by the optimal parser (levels >= 10)
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Signatures (verified against c_src/include/lz4hc.h + c_src/src/lz4hc.c)
// ---------------------------------------------------------------------------
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnSizeof = unsafe extern "C" fn() -> c_int;
/// `LZ4_compress_HC(src, dst, srcSize, dstCapacity, cLevel)`
type FnHC5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compressHC_limitedOutput(src, dst, srcSize, maxOut)` / `LZ4_compressHC2(.., cLevel)`
type FnHC4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compressHC(src, dst, srcSize)`
type FnHC3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_compress_HC_extStateHC(state, src, dst, srcSize, maxDst, cLevel)`
type FnExt6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_continue(stream, src, dst, srcSize, dstCapacity)`
type FnExt5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compressHC_withStateHC(state, src, dst, srcSize)`
type FnExt4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_compress_HC_destSize(state, src, dst, srcSizePtr, targetDst, cLevel)`
type FnDestSizeHC = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    *mut c_int,
    c_int,
    c_int,
) -> c_int;
/// `LZ4_compress_HC_continue_destSize(stream, src, dst, srcSizePtr, targetDst)`
type FnContDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnInit = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
/// `LZ4_resetStreamHC(stream, cLevel)` / `_fast` / `LZ4_setCompressionLevel` / `_favor…`
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnAttach = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnSaveDict = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnSlideHC = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
type FnResetStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
type FnDecSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Level sweeps
// ---------------------------------------------------------------------------

/// Every level plus the out-of-range values that clamp (`<1` -> 9, `>12` -> 12).
const ALL_LEVELS: &[c_int] = &[-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 100];
/// One representative of each algorithm, plus both clamp directions.
const REP_LEVELS: &[c_int] = &[1, 2, 3, 6, 9, 10, 11, 12, 0, 13];
/// The optimal-parser levels (expensive in a debug build).
const OPT_LEVELS: &[c_int] = &[10, 11, 12];

/// Mirrors `LZ4HC_getCLevelParams` clamping.
fn eff_level(l: c_int) -> c_int {
    if l < 1 {
        9
    } else if l > 12 {
        12
    } else {
        l
    }
}

/// True when the level lands in the optimal parser (slow in a debug build).
fn is_opt(l: c_int) -> bool {
    eff_level(l) >= 10
}

/// Length sweep appropriate for the cost of `level`.
fn lens_for(l: c_int) -> &'static [usize] {
    if is_opt(l) {
        SMALL_LENS
    } else {
        KEY_LENS
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// `LZ4_compressBound(n)`, never 0 (so a `notLimited` call always has room).
fn bound_of(bound: &FnBound, n: usize) -> usize {
    unsafe { bound(n as c_int) }.max(1) as usize
}

/// `gen_data`, but the buffer is ALWAYS backed by a real allocation, so even a
/// zero-length source has a *valid* address.
///
/// `Vec::<u8>::as_ptr()` on a zero-capacity vector returns the dangling
/// pointer `0x1`. Every HC parser computes `src + srcSize - MFLIMIT` (and
/// friends) up front; around address `0x1` those wrap to the top of the address
/// space, the `ip <= mflimit` guard then passes, and the C `.so` walks off into
/// unmapped memory. That is a defect of the *caller*, not of either library, so
/// never hand a dangling pointer to the API.
fn gen_src(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_data(shape, len, rng);
    if v.capacity() < 64 {
        v.reserve(64);
    }
    v
}

/// Round-trip a compressed block through `LZ4_decompress_safe`.
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

/// Round-trip a compressed block through `LZ4_decompress_safe_usingDict`.
fn check_roundtrip_dict(dec: &FnDecUsingDict, comp: &[u8], orig: &[u8], dict: &[u8], ctx: &str) {
    let mut out = vec![0u8; orig.len() + 64];
    let n = unsafe {
        dec(
            comp.as_ptr() as *const c_char,
            out.as_mut_ptr() as *mut c_char,
            comp.len() as c_int,
            orig.len() as c_int,
            dict.as_ptr() as *const c_char,
            dict.len() as c_int,
        )
    };
    assert_eq!(
        n,
        orig.len() as c_int,
        "{ctx}: dict round-trip size mismatch (dictSize={})",
        dict.len()
    );
    assert_bytes_eq(
        &out[..orig.len()],
        orig,
        &format!("{ctx}: dict round-trip data"),
    );
}

/// Drive a 5-arg one-shot (`LZ4_compress_HC`) through both libraries.
/// Returns `(ret, C output buffer)`.
fn oneshot(
    cf: &FnHC5,
    rf: &FnHC5,
    src: &[u8],
    cap: usize,
    lvl: c_int,
    ctx: &str,
) -> (c_int, Vec<u8>) {
    let mut cb = vec![0u8; cap + 32];
    let mut rb = vec![0u8; cap + 32];
    let (cn, rn) = unsafe {
        (
            cf(
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            ),
            rf(
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            ),
        )
    };
    assert_out_eq(cn, &cb, rn, &rb, ctx);
    (cn, cb)
}

/// Drive a 6-arg ext-state one-shot through both libraries.
#[allow(clippy::too_many_arguments)]
fn oneshot_state(
    cf: &FnExt6,
    rf: &FnExt6,
    cst: *mut c_void,
    rst: *mut c_void,
    src: &[u8],
    cap: usize,
    lvl: c_int,
    ctx: &str,
) -> (c_int, Vec<u8>) {
    let mut cb = vec![0u8; cap + 32];
    let mut rb = vec![0u8; cap + 32];
    let (cn, rn) = unsafe {
        (
            cf(
                cst,
                src.as_ptr() as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            ),
            rf(
                rst,
                src.as_ptr() as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                src.len() as c_int,
                cap as c_int,
                lvl,
            ),
        )
    };
    assert_out_eq(cn, &cb, rn, &rb, ctx);
    (cn, cb)
}

/// Drive `LZ4_compress_HC_continue` on both streams. `src` is a raw pointer so
/// chains can hand out interior pointers of one contiguous buffer.
#[allow(clippy::too_many_arguments)]
fn cont(
    cf: &FnExt5,
    rf: &FnExt5,
    cs: *mut c_void,
    rs: *mut c_void,
    src: *const u8,
    n: usize,
    cap: usize,
    ctx: &str,
) -> (c_int, Vec<u8>) {
    let mut cb = vec![0u8; cap + 32];
    let mut rb = vec![0u8; cap + 32];
    let (cn, rn) = unsafe {
        (
            cf(
                cs,
                src as *const c_char,
                cb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
            ),
            rf(
                rs,
                src as *const c_char,
                rb.as_mut_ptr() as *mut c_char,
                n as c_int,
                cap as c_int,
            ),
        )
    };
    assert_out_eq(cn, &cb, rn, &rb, ctx);
    (cn, cb)
}

/// A capacity sweep around the natural compressed size `nat`.
fn cap_sweep(nat: c_int, full: usize, rng: &mut Rng, nrand: usize) -> Vec<usize> {
    let mut caps: Vec<usize> = vec![0, 1, 2, 5];
    if nat > 0 {
        let n = nat as usize;
        caps.push(n);
        caps.push(n - 1);
        caps.push(n / 2);
        caps.push(n / 4);
        caps.push(n * 3 / 4);
    }
    caps.push(full);
    for _ in 0..nrand {
        caps.push(rng.range(0, full));
    }
    caps.sort_unstable();
    caps.dedup();
    caps
}

// ---------------------------------------------------------------------------
// Rows 70-72 — LZ4_compress_HC: level sweep x shapes x sizes (notLimited)
// ---------------------------------------------------------------------------
#[test]
fn rows70_72_compress_hc_levels_shapes_sizes() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x700072);

    for &lvl in ALL_LEVELS {
        for &len in lens_for(lvl) {
            for &shape in ALL_SHAPES {
                let src = gen_src(shape, len, &mut rng);
                let cap = bound_of(&bound.0, len);
                let ctx = format!("compress_HC lvl={lvl} len={len} {shape:?}");
                let (cn, cb) = oneshot(&hc.0, &hc.1, &src, cap, lvl, &ctx);
                assert!(cn > 0 || len == 0, "{ctx}: unexpected failure");
                if cn > 0 {
                    // Row 99: every HC block must round-trip.
                    check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                    check_roundtrip(&dec.1, &cb[..cn as usize], &src, &ctx);
                }
            }
        }
    }

    // Levels that clamp must be byte-identical to the level they clamp to.
    for (raw, same) in [
        (-1i32, 9i32),
        (0, 9),
        (i32::MIN, 9),
        (13, 12),
        (100, 12),
        (i32::MAX, 12),
    ] {
        for &len in &[13usize, 1024, 8192, 65547] {
            for &shape in ALL_SHAPES {
                let src = gen_src(shape, len, &mut rng);
                let cap = bound_of(&bound.0, len);
                let ctx = format!("clamp lvl={raw}=>{same} len={len} {shape:?}");
                let (an, ab) = oneshot(&hc.0, &hc.1, &src, cap, raw, &ctx);
                let (bn, bb) = oneshot(&hc.0, &hc.1, &src, cap, same, &ctx);
                assert_eq!(an, bn, "{ctx}: clamped level changed the size");
                assert_bytes_eq(&ab[..an as usize], &bb[..bn as usize], &ctx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 73 — LZ4_compress_HC: level sweep x limitedOutput dstCapacity sweep
// ---------------------------------------------------------------------------
#[test]
fn row73_compress_hc_limited_output_capacity_sweep() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x730073);

    for &lvl in REP_LEVELS {
        // The optimal parser is slow in a debug build: keep its inputs small.
        let lens: &[usize] = if is_opt(lvl) {
            &[13, 64, 300, 1024, 4096, 8192]
        } else {
            &[13, 64, 300, 1024, 4096, 65536, 100_000]
        };
        for &len in lens {
            for &shape in ALL_SHAPES {
                let src = gen_src(shape, len, &mut rng);
                let full = bound_of(&bound.0, len);
                // Natural (unconstrained) size first, then sweep around it.
                let nat = {
                    let mut probe = vec![0u8; full + 32];
                    unsafe {
                        hc.0(
                            src.as_ptr() as *const c_char,
                            probe.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            full as c_int,
                            lvl,
                        )
                    }
                };
                for &cap in &cap_sweep(nat, full, &mut rng, 6) {
                    let ctx = format!("HC limited lvl={lvl} len={len} {shape:?} cap={cap}");
                    let (cn, cb) = oneshot(&hc.0, &hc.1, &src, cap, lvl, &ctx);
                    if cn > 0 {
                        assert!(cn as usize <= cap, "{ctx}: wrote past dstCapacity");
                        check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 74-76 — the three algorithm classes pinned explicitly
// ---------------------------------------------------------------------------
#[test]
fn rows74_76_level_classes() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x740076);

    // (rows, levels, max random length, #random lengths)
    let groups: &[(&str, &[c_int], usize, usize)] = &[
        ("row74 lz4mid", &[1, 2], 200_000, 10),
        ("row75 hashChain", &[3, 4, 5, 6, 7, 8, 9], 120_000, 6),
        ("row76 optimal", &[10, 11, 12], 60_000, 5),
    ];

    for &(name, levels, maxlen, nrand) in groups {
        for &lvl in levels {
            for &shape in ALL_SHAPES {
                // Fixed boundary lengths + randomized lengths.
                let mut lens: Vec<usize> = vec![0, 1, 12, 13, 14, 65535, 65536, 65547];
                for _ in 0..nrand {
                    lens.push(rng.range(0, maxlen));
                }
                for &len in &lens {
                    let src = gen_src(shape, len, &mut rng);
                    let cap = bound_of(&bound.0, len);
                    let ctx = format!("{name} lvl={lvl} len={len} {shape:?}");
                    let (cn, cb) = oneshot(&hc.0, &hc.1, &src, cap, lvl, &ctx);
                    if cn > 0 {
                        check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 77 — LZ4_compress_HC_extStateHC with caller-owned aligned state
// ---------------------------------------------------------------------------
#[test]
fn row77_compress_hc_ext_state() {
    sym!(ext, "LZ4_compress_HC_extStateHC", FnExt6);
    sym!(sos, "LZ4_sizeofStateHC", FnSizeof);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x770077);

    unsafe {
        assert_ret_eq(sos.0(), sos.1(), "LZ4_sizeofStateHC");
        assert_eq!(sos.0() as usize, SIZEOF_LZ4_STREAMHC_T);
    }

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (cp, rp) = (cst.ptr() as *mut c_void, rst.ptr() as *mut c_void);

    for &lvl in ALL_LEVELS {
        let lens: &[usize] = if is_opt(lvl) {
            &[0, 1, 13, 64, 1024, 4096, 65536, 65547]
        } else {
            &[0, 1, 13, 64, 1024, 4096, 65535, 65536, 65547, 100_000]
        };
        for &len in lens {
            for &shape in ALL_SHAPES {
                let src = gen_src(shape, len, &mut rng);
                let full = bound_of(&bound.0, len);
                // notLimited (cap >= bound) and limitedOutput (cap < bound).
                for &cap in &[full, full / 3, 1] {
                    let ctx = format!("extStateHC lvl={lvl} len={len} {shape:?} cap={cap}");
                    let (cn, cb) = oneshot_state(&ext.0, &ext.1, cp, rp, &src, cap, lvl, &ctx);
                    if cn > 0 {
                        check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 78 — LZ4_compress_HC_extStateHC_fastReset: fresh state AND reused state
// ---------------------------------------------------------------------------
#[test]
fn row78_compress_hc_ext_state_fast_reset() {
    sym!(fast, "LZ4_compress_HC_extStateHC_fastReset", FnExt6);
    sym!(ext, "LZ4_compress_HC_extStateHC", FnExt6);
    sym!(init, "LZ4_initStreamHC", FnInit);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x780078);

    // The REUSED pair — never re-initialized after the first init, so
    // `LZ4_resetStreamHC_fast` keeps advancing `dictLimit` across calls.
    let mut creuse = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rreuse = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (crp, rrp) = (creuse.ptr() as *mut c_void, rreuse.ptr() as *mut c_void);
    unsafe {
        assert!(!init.0(crp, SIZEOF_LZ4_STREAMHC_T).is_null());
        assert!(!init.1(rrp, SIZEOF_LZ4_STREAMHC_T).is_null());
    }

    // The FRESH pair — fully re-initialized before every call.
    let mut cfresh = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rfresh = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (cfp, rfp) = (cfresh.ptr() as *mut c_void, rfresh.ptr() as *mut c_void);

    for round in 0..3 {
        for &lvl in REP_LEVELS {
            let lens: &[usize] = if is_opt(lvl) {
                &[0, 13, 64, 1024, 4096, 65547]
            } else {
                &[0, 13, 64, 1024, 4096, 65535, 65547, 100_000]
            };
            for &len in lens {
                for &shape in ALL_SHAPES {
                    let src = gen_src(shape, len, &mut rng);
                    let full = bound_of(&bound.0, len);
                    for &cap in &[full, full / 3] {
                        // Fresh state: identical to LZ4_compress_HC_extStateHC.
                        unsafe {
                            assert!(!init.0(cfp, SIZEOF_LZ4_STREAMHC_T).is_null());
                            assert!(!init.1(rfp, SIZEOF_LZ4_STREAMHC_T).is_null());
                        }
                        let ctx = format!(
                            "fastReset fresh r={round} lvl={lvl} len={len} {shape:?} cap={cap}"
                        );
                        let (fn_, fb) =
                            oneshot_state(&fast.0, &fast.1, cfp, rfp, &src, cap, lvl, &ctx);
                        let (en, eb) =
                            oneshot_state(&ext.0, &ext.1, cfp, rfp, &src, cap, lvl, &ctx);
                        assert_eq!(fn_, en, "{ctx}: fastReset != extStateHC on a fresh state");
                        assert_bytes_eq(
                            &fb[..fn_.max(0) as usize],
                            &eb[..en.max(0) as usize],
                            &ctx,
                        );

                        // Reused state: the fast-reset path proper.
                        let ctx = format!(
                            "fastReset reuse r={round} lvl={lvl} len={len} {shape:?} cap={cap}"
                        );
                        let (cn, cb) =
                            oneshot_state(&fast.0, &fast.1, crp, rrp, &src, cap, lvl, &ctx);
                        if cn > 0 {
                            check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 79-80 — LZ4_compress_HC_destSize (fillOutput) at all three level classes
// ---------------------------------------------------------------------------
#[test]
fn rows79_80_compress_hc_dest_size() {
    sym!(ds, "LZ4_compress_HC_destSize", FnDestSizeHC);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x790080);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (cp, rp) = (cst.ptr() as *mut c_void, rst.ptr() as *mut c_void);

    // Pass 1: EVERY level (incl. both clamp directions) over small inputs with a
    // dense target sweep. Pass 2: the level representatives over inputs past the
    // 64 KB window with a coarser sweep (keeps the debug-build cost bounded).
    for pass in 0..2 {
        let levels: &[c_int] = if pass == 0 { ALL_LEVELS } else { REP_LEVELS };
        for &lvl in levels {
            let lens: &[usize] = match (pass, is_opt(lvl)) {
                (0, _) => &[0, 1, 13, 64, 300, 1024, 4096, 8192],
                (_, true) => &[65536, 65547],
                (_, false) => &[65535, 65536, 65547, 100_000],
            };
            for &len in lens {
                for &shape in ALL_SHAPES {
                    let src = gen_src(shape, len, &mut rng);
                    let full = bound_of(&bound.0, len);
                    // The sweep must reach deep into truncation so the
                    // "dest overflow salvage" path is hit at every level class.
                    let mut targets: Vec<usize> = vec![0, 1, 2, 3, 5, 10, 17, full];
                    if pass == 0 {
                        for f in [1usize, 2, 3, 4, 8, 16, 64] {
                            targets.push(full / f);
                        }
                        for _ in 0..6 {
                            targets.push(rng.range(0, full));
                        }
                    } else {
                        for f in [1usize, 2, 4, 16] {
                            targets.push(full / f);
                        }
                        for _ in 0..2 {
                            targets.push(rng.range(0, full));
                        }
                    }
                    targets.sort_unstable();
                    targets.dedup();

                    for &tgt in &targets {
                        let mut csz = len as c_int;
                        let mut rsz = len as c_int;
                        let mut cb = vec![0u8; tgt + 32];
                        let mut rb = vec![0u8; tgt + 32];
                        let (cn, rn) = unsafe {
                            (
                                ds.0(
                                    cp,
                                    src.as_ptr() as *const c_char,
                                    cb.as_mut_ptr() as *mut c_char,
                                    &mut csz,
                                    tgt as c_int,
                                    lvl,
                                ),
                                ds.1(
                                    rp,
                                    src.as_ptr() as *const c_char,
                                    rb.as_mut_ptr() as *mut c_char,
                                    &mut rsz,
                                    tgt as c_int,
                                    lvl,
                                ),
                            )
                        };
                        let ctx = format!("HC_destSize lvl={lvl} len={len} {shape:?} tgt={tgt}");
                        assert_eq!(csz, rsz, "{ctx}: *srcSizePtr mismatch C={csz} Rust={rsz}");
                        assert_out_eq(cn, &cb, rn, &rb, &ctx);
                        if cn > 0 {
                            assert!(cn as usize <= tgt, "{ctx}: exceeded targetDstSize");
                            assert!(csz >= 0 && csz as usize <= len, "{ctx}: bogus *srcSizePtr");
                            check_roundtrip(&dec.0, &cb[..cn as usize], &src[..csz as usize], &ctx);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 81-82 — LZ4_favorDecompressionSpeed (only consulted at levels >= 10)
// ---------------------------------------------------------------------------
fn favor_body(rows: &str, levels: &[c_int], seed: u64) {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(fav, "LZ4_favorDecompressionSpeed", FnStreamInt);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(seed);

    let blk = 6000usize;
    let nblk = 5usize;
    for &lvl in levels {
        for &favor in &[0i32, 1, -1, 12345] {
            for &shape in ALL_SHAPES {
                // One contiguous buffer: it must outlive both streams.
                let data = gen_src(shape, blk * nblk, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    assert!(!csp.is_null() && !rsp.is_null());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    fav.0(csp, favor);
                    fav.1(rsp, favor);
                    for i in 0..nblk {
                        let off = i * blk;
                        let cap = bound_of(&bound.0, blk);
                        let ctx = format!("{rows} favor={favor} lvl={lvl} {shape:?} blk={i}");
                        let (cn, cb) = cont(
                            &cont5.0,
                            &cont5.1,
                            csp,
                            rsp,
                            data[off..].as_ptr(),
                            blk,
                            cap,
                            &ctx,
                        );
                        if cn > 0 {
                            if i == 0 {
                                check_roundtrip(&dec.0, &cb[..cn as usize], &data[..blk], &ctx);
                            } else {
                                check_roundtrip_dict(
                                    &decd.0,
                                    &cb[..cn as usize],
                                    &data[off..off + blk],
                                    &data[..off],
                                    &ctx,
                                );
                            }
                        }
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

#[test]
fn row81_favor_decompression_speed_optimal_levels() {
    favor_body("row81", OPT_LEVELS, 0x810081);
}

#[test]
fn row82_favor_decompression_speed_ignored_levels() {
    favor_body("row82", &[1, 2, 3, 4, 5, 6, 7, 8, 9], 0x820082);
}

// ---------------------------------------------------------------------------
// Row 83 — LZ4_setCompressionLevel between blocks (incl. 1 <-> 12)
// ---------------------------------------------------------------------------
#[test]
fn row83_set_compression_level_between_blocks() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(scl, "LZ4_setCompressionLevel", FnStreamInt);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x830083);

    // Straddles the lz4mid boundary in both directions, and includes the
    // out-of-range values that clamp to 9 and 12.
    let seqs: &[&[c_int]] = &[
        &[1, 12, 1, 12, 1],
        &[12, 1, 2, 11, 2],
        &[9, 1, 9, 2, 9],
        &[2, 3, 2, 3, 2],
        &[0, 13, -7, 100, 1],
        &[3, 9, 10, 11, 12],
    ];
    let blk = 5000usize;
    for seq in seqs {
        for &shape in ALL_SHAPES {
            let data = gen_src(shape, blk * seq.len(), &mut rng);
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                for (i, &lvl) in seq.iter().enumerate() {
                    scl.0(csp, lvl);
                    scl.1(rsp, lvl);
                    let off = i * blk;
                    let cap = bound_of(&bound.0, blk);
                    let ctx = format!("setCompressionLevel {seq:?} step={i} lvl={lvl} {shape:?}");
                    let (cn, cb) = cont(
                        &cont5.0,
                        &cont5.1,
                        csp,
                        rsp,
                        data[off..].as_ptr(),
                        blk,
                        cap,
                        &ctx,
                    );
                    if cn > 0 {
                        if i == 0 {
                            check_roundtrip(&dec.0, &cb[..cn as usize], &data[..blk], &ctx);
                        } else {
                            check_roundtrip_dict(
                                &decd.0,
                                &cb[..cn as usize],
                                &data[off..off + blk],
                                &data[..off],
                                &ctx,
                            );
                        }
                    }
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 84-85 — contiguous streaming chains (uniform then random block sizes)
// ---------------------------------------------------------------------------
#[test]
fn rows84_85_continue_contiguous_chains() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x840085);

    for &lvl in REP_LEVELS {
        // (name, block size, #blocks) — smaller totals for the optimal parser.
        let plans: &[(&str, usize, usize)] = if is_opt(lvl) {
            &[
                ("uniform-64", 64, 40),
                ("uniform-1k", 1024, 16),
                ("uniform-4k", 4096, 8),
                ("cross64k-9k", 9000, 9),
            ]
        } else {
            &[
                ("uniform-64", 64, 60),
                ("uniform-1k", 1024, 40),
                ("uniform-4k", 4096, 30),
                ("uniform-70k", 70_000, 4),
                ("cross64k-9k", 9000, 14),
            ]
        };

        // Row 84 — uniform block sizes.
        for &(name, blk, nblocks) in plans {
            for &shape in ALL_SHAPES {
                let total = blk * nblocks;
                let data = gen_src(shape, total, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    for i in 0..nblocks {
                        let off = i * blk;
                        let cap = bound_of(&bound.0, blk);
                        let ctx = format!("row84 {name} lvl={lvl} {shape:?} blk={i}");
                        let (cn, cb) = cont(
                            &cont5.0,
                            &cont5.1,
                            csp,
                            rsp,
                            data[off..].as_ptr(),
                            blk,
                            cap,
                            &ctx,
                        );
                        if cn > 0 {
                            // History = every preceding byte (contiguous prefix),
                            // capped at the 64 KB window by the decoder itself.
                            check_roundtrip_dict(
                                &decd.0,
                                &cb[..cn as usize],
                                &data[off..off + blk],
                                &data[..off],
                                &ctx,
                            );
                        }
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }

        // Row 85 — random block sizes.
        let total = if is_opt(lvl) { 60_000usize } else { 250_000 };
        let maxblk = if is_opt(lvl) { 8_000usize } else { 20_000 };
        for &shape in ALL_SHAPES {
            let data = gen_src(shape, total, &mut rng);
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                rf.0(csp, lvl);
                rf.1(rsp, lvl);
                let mut off = 0usize;
                let mut i = 0;
                while off < total {
                    let n = rng.range(1, maxblk).min(total - off);
                    let cap = bound_of(&bound.0, n);
                    let ctx = format!("row85 random lvl={lvl} {shape:?} blk={i} n={n}");
                    let (cn, cb) = cont(
                        &cont5.0,
                        &cont5.1,
                        csp,
                        rsp,
                        data[off..].as_ptr(),
                        n,
                        cap,
                        &ctx,
                    );
                    if cn > 0 && off > 0 {
                        check_roundtrip_dict(
                            &decd.0,
                            &cb[..cn as usize],
                            &data[off..off + n],
                            &data[..off],
                            &ctx,
                        );
                    } else if cn > 0 {
                        check_roundtrip(&dec.0, &cb[..cn as usize], &data[..n], &ctx);
                    }
                    off += n;
                    i += 1;
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 86 (+ row 98) — every block in its own allocation => LZ4HC_setExternalDict
// ---------------------------------------------------------------------------
#[test]
fn row86_continue_external_dict() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x860086);

    for &lvl in REP_LEVELS {
        let blks: &[usize] = if is_opt(lvl) {
            &[64, 1024, 9000]
        } else {
            &[64, 1024, 9000, 70_000]
        };
        let nblocks = if is_opt(lvl) { 8usize } else { 12 };
        for &blk in blks {
            for &shape in ALL_SHAPES {
                // Each block is a separate allocation => never contiguous with
                // the previous one => the extDict path. The whole Vec lives for
                // the entire chain, so nothing the stream points at dangles.
                let blocks: Vec<Vec<u8>> = (0..nblocks)
                    .map(|_| gen_src(shape, blk, &mut rng))
                    .collect();
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    for (i, b) in blocks.iter().enumerate() {
                        let cap = bound_of(&bound.0, blk);
                        let ctx = format!("row86 extDict lvl={lvl} {shape:?} blk={blk} i={i}");
                        let (cn, cb) =
                            cont(&cont5.0, &cont5.1, csp, rsp, b.as_ptr(), blk, cap, &ctx);
                        if cn > 0 {
                            // Only ONE extDict segment is retained by the HC
                            // stream: the immediately preceding block.
                            if i == 0 {
                                check_roundtrip(&dec.0, &cb[..cn as usize], b, &ctx);
                            } else {
                                check_roundtrip_dict(
                                    &decd.0,
                                    &cb[..cn as usize],
                                    b,
                                    &blocks[i - 1],
                                    &ctx,
                                );
                            }
                        }
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 87 — long chains crossing 64 KB many times (dictionary reload path)
// ---------------------------------------------------------------------------
#[test]
fn row87_long_chain_over_64k() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(sd, "LZ4_saveDictHC", FnSaveDict);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x870087);

    for &lvl in &[1i32, 2, 9, 12] {
        for &shape in ALL_SHAPES {
            // ~500 KB in many small blocks: the prefix crosses 64 KB early and
            // keeps growing, then `LZ4_saveDictHC` re-anchors the history.
            let nblocks = if is_opt(lvl) { 120usize } else { 260 };
            let blk = 2000usize;
            let total = blk * nblocks;
            let data = gen_src(shape, total, &mut rng);
            let mut csafe = vec![0u8; 70_000];
            let mut rsafe = vec![0u8; 70_000];
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                rf.0(csp, lvl);
                rf.1(rsp, lvl);
                for i in 0..nblocks {
                    let off = i * blk;
                    let cap = bound_of(&bound.0, blk);
                    let ctx = format!("row87 chain lvl={lvl} {shape:?} blk={i}");
                    let (cn, cb) = cont(
                        &cont5.0,
                        &cont5.1,
                        csp,
                        rsp,
                        data[off..].as_ptr(),
                        blk,
                        cap,
                        &ctx,
                    );
                    if cn > 0 && off > 0 {
                        check_roundtrip_dict(
                            &decd.0,
                            &cb[..cn as usize],
                            &data[off..off + blk],
                            &data[..off],
                            &ctx,
                        );
                    }
                    // Half-way through, re-anchor via saveDictHC: the stream
                    // then references csafe/rsafe, which outlive it.
                    if i == nblocks / 2 {
                        let (a, b) = (
                            sd.0(csp, csafe.as_mut_ptr() as *mut c_char, 65536),
                            sd.1(rsp, rsafe.as_mut_ptr() as *mut c_char, 65536),
                        );
                        assert_ret_eq(a, b, "row87 saveDictHC");
                        assert_bytes_eq(
                            &csafe[..a as usize],
                            &rsafe[..b as usize],
                            "row87 saveDictHC bytes",
                        );
                    }
                }
                fs.0(csp);
                fs.1(rsp);
            }

            // Same idea, but the history is (re)established with loadDictHC on a
            // dictionary larger than the 64 KB window.
            let dict = gen_src(shape, 200_000, &mut rng);
            let blocks: Vec<Vec<u8>> = (0..10).map(|_| gen_src(shape, 7000, &mut rng)).collect();
            unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                rf.0(csp, lvl);
                rf.1(rsp, lvl);
                assert_ret_eq(
                    ld.0(csp, dict.as_ptr() as *const c_char, dict.len() as c_int),
                    ld.1(rsp, dict.as_ptr() as *const c_char, dict.len() as c_int),
                    "row87 loadDictHC 200k",
                );
                for (i, b) in blocks.iter().enumerate() {
                    let cap = bound_of(&bound.0, b.len());
                    let ctx = format!("row87 bigdict lvl={lvl} {shape:?} i={i}");
                    cont(&cont5.0, &cont5.1, csp, rsp, b.as_ptr(), b.len(), cap, &ctx);
                }
                fs.0(csp);
                fs.1(rsp);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 88 — LZ4_loadDictHC dictSize sweep x level sweep, then a chain
// ---------------------------------------------------------------------------
#[test]
fn row88_load_dict_hc_size_sweep() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x880088);

    let dict_sizes = [0usize, 1, 3, 4, 8, 9, 64, 1024, 65535, 65536, 70_000];
    for &lvl in REP_LEVELS {
        for &dsz in &dict_sizes {
            for &shape in ALL_SHAPES {
                // The dictionary buffer must outlive the stream: the stream
                // keeps `prefixStart`/`dictStart` pointers into it.
                let dict = gen_src(shape, dsz, &mut rng);
                let blk = 4096usize;
                let nblk = if is_opt(lvl) { 3usize } else { 5 };
                let data = gen_src(shape, blk * nblk, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    let (cd, rd) = (
                        ld.0(csp, dict.as_ptr() as *const c_char, dsz as c_int),
                        ld.1(rsp, dict.as_ptr() as *const c_char, dsz as c_int),
                    );
                    let ctx = format!("row88 loadDictHC lvl={lvl} dsz={dsz} {shape:?}");
                    assert_ret_eq(cd, rd, &ctx);
                    assert_eq!(cd as usize, dsz.min(65536), "{ctx}: clamped size");
                    // The window the decoder must use is the last 64 KB.
                    let dwin = &dict[dsz - cd as usize..];
                    for i in 0..nblk {
                        let off = i * blk;
                        let cap = bound_of(&bound.0, blk);
                        let ctx = format!("{ctx} blk={i}");
                        let (cn, cb) = cont(
                            &cont5.0,
                            &cont5.1,
                            csp,
                            rsp,
                            data[off..].as_ptr(),
                            blk,
                            cap,
                            &ctx,
                        );
                        if cn > 0 && i == 0 {
                            check_roundtrip_dict(
                                &decd.0,
                                &cb[..cn as usize],
                                &data[..blk],
                                dwin,
                                &ctx,
                            );
                        }
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 89-90 — LZ4_attach_HC_dictionary, compatible and incompatible levels
// ---------------------------------------------------------------------------
#[test]
fn rows89_90_attach_hc_dictionary() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(scl, "LZ4_setCompressionLevel", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(at, "LZ4_attach_HC_dictionary", FnAttach);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x890090);

    // (dict level, working level): compatible pairs (both lz4mid, or both not)
    // and INCOMPATIBLE pairs straddling the lz4mid boundary (lz4hc.c:1434).
    let pairs: &[(c_int, c_int)] = &[
        (1, 1),
        (2, 2),
        (1, 2),
        (2, 1),
        (3, 9),
        (9, 3),
        (9, 9),
        (12, 10),
        (10, 12),
        (2, 9),
        (9, 2),
        (1, 12),
        (12, 1),
        (2, 3),
        (3, 2),
    ];
    for &(dlvl, wlvl) in pairs {
        for &dsz in &[0usize, 8, 64, 1024, 65536, 70_000] {
            for &shape in ALL_SHAPES {
                // dict + every source block must outlive both streams.
                let dict = gen_src(shape, dsz, &mut rng);
                let srcs: Vec<Vec<u8>> = [64usize, 1024, 4096, 4097, 9000]
                    .iter()
                    .map(|&n| gen_src(shape, n, &mut rng))
                    .collect();
                let nxt = gen_src(shape, 2048, &mut rng);
                unsafe {
                    let (cdict, rdict) = (cs.0(), cs.1());
                    let (cw, rw) = (cs.0(), cs.1());
                    scl.0(cdict, dlvl);
                    scl.1(rdict, dlvl);
                    let cdn = ld.0(cdict, dict.as_ptr() as *const c_char, dsz as c_int);
                    let rdn = ld.1(rdict, dict.as_ptr() as *const c_char, dsz as c_int);
                    assert_ret_eq(cdn, rdn, "attach loadDictHC");
                    let dwin = &dict[dsz - cdn as usize..];

                    // srcSize <= 4096 (usingDictCtxHc) and > 4096 (the
                    // `position == 0 && *srcSizePtr > 4 KB` memcpy shortcut,
                    // which additionally requires COMPATIBLE levels).
                    for src in &srcs {
                        rf.0(cw, wlvl);
                        rf.1(rw, wlvl);
                        at.0(cw, cdict as *const c_void);
                        at.1(rw, rdict as *const c_void);
                        let cap = bound_of(&bound.0, src.len());
                        let ctx = format!(
                            "row89_90 attach d={dlvl} w={wlvl} dsz={dsz} {shape:?} n={}",
                            src.len()
                        );
                        let (cn, cb) = cont(
                            &cont5.0,
                            &cont5.1,
                            cw,
                            rw,
                            src.as_ptr(),
                            src.len(),
                            cap,
                            &ctx,
                        );
                        if cn > 0 {
                            check_roundtrip_dict(&decd.0, &cb[..cn as usize], src, dwin, &ctx);
                        }
                        // A second block continues from the working prefix.
                        let cap2 = bound_of(&bound.0, nxt.len());
                        cont(
                            &cont5.0,
                            &cont5.1,
                            cw,
                            rw,
                            nxt.as_ptr(),
                            nxt.len(),
                            cap2,
                            &format!("{ctx} 2nd"),
                        );
                    }

                    // Attaching NULL detaches (ERRORS.md row 105).
                    rf.0(cw, wlvl);
                    rf.1(rw, wlvl);
                    at.0(cw, std::ptr::null());
                    at.1(rw, std::ptr::null());
                    let cap = bound_of(&bound.0, nxt.len());
                    let ctx = format!("row89_90 detach d={dlvl} w={wlvl} dsz={dsz} {shape:?}");
                    let (cn, cb) = cont(
                        &cont5.0,
                        &cont5.1,
                        cw,
                        rw,
                        nxt.as_ptr(),
                        nxt.len(),
                        cap,
                        &ctx,
                    );
                    if cn > 0 {
                        check_roundtrip(&dec.0, &cb[..cn as usize], &nxt, &ctx);
                    }
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
}

// ---------------------------------------------------------------------------
// Row 91 — LZ4_saveDictHC after a chain, maxDictSize sweep
// ---------------------------------------------------------------------------
#[test]
fn row91_save_dict_hc() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(sd, "LZ4_saveDictHC", FnSaveDict);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x910091);

    // One representative per algorithm class; the data shape is rotated across
    // the sweep instead of crossed with it (saveDictHC itself is shape-blind).
    let mut k = 0usize;
    for &lvl in &[1i32, 2, 9, 12, 0] {
        for &pre in &[0usize, 64, 1024, 40_000, 100_000] {
            for &want in &[0i32, 1, 3, 4, 64, 1024, 65536, 70_000, -1] {
                {
                    let shape = ALL_SHAPES[k % ALL_SHAPES.len()];
                    k += 1;
                    // Every buffer the stream can retain a pointer into is
                    // declared here so it outlives the stream:
                    //   `data`  -> prefixStart/end before the save
                    //   `csafe` -> prefixStart/end after the save
                    let data = gen_src(shape, pre, &mut rng);
                    let nxt = gen_src(shape, 3000, &mut rng);
                    let mut csafe = vec![0u8; 80_000];
                    let mut rsafe = vec![0u8; 80_000];
                    unsafe {
                        let (csp, rsp) = (cs.0(), cs.1());
                        rf.0(csp, lvl);
                        rf.1(rsp, lvl);
                        if pre > 0 {
                            let cap = bound_of(&bound.0, pre);
                            cont(
                                &cont5.0,
                                &cont5.1,
                                csp,
                                rsp,
                                data.as_ptr(),
                                pre,
                                cap,
                                "row91 setup compress",
                            );
                        }
                        let cn = sd.0(csp, csafe.as_mut_ptr() as *mut c_char, want);
                        let rn = sd.1(rsp, rsafe.as_mut_ptr() as *mut c_char, want);
                        let ctx =
                            format!("row91 saveDictHC lvl={lvl} pre={pre} want={want} {shape:?}");
                        assert_ret_eq(cn, rn, &ctx);
                        assert!(cn >= 0 && cn <= 65536, "{ctx}: bogus return {cn}");
                        assert_bytes_eq(&csafe[..cn as usize], &rsafe[..cn as usize], &ctx);
                        if cn > 0 {
                            assert_bytes_eq(
                                &csafe[..cn as usize],
                                &data[pre - cn as usize..],
                                &format!("{ctx}: saved tail"),
                            );
                        }

                        // The stream must stay usable, and identically so.
                        //
                        // NOTE: only when the stream actually had a history.
                        // `LZ4_saveDictHC` on a stream that was never used
                        // (prefixStart == NULL) re-anchors it with
                        // dictLimit == lowLimit == 0, i.e. the reserved
                        // "no entry" index 0 becomes a *valid* match index.
                        // Compressing after that is UNDEFINED BEHAVIOUR: the C
                        // underflows `matchIndex -= DELTANEXTU16(...)` in
                        // LZ4HC_InsertAndGetWiderMatch and segfaults on the
                        // resulting wild pointer (verified against the C .so).
                        // Both libraries crash, so it is not differentially
                        // testable — see the ERRORS.md UB appendix.
                        if pre > 0 {
                            let cap = bound_of(&bound.0, nxt.len());
                            cont(
                                &cont5.0,
                                &cont5.1,
                                csp,
                                rsp,
                                nxt.as_ptr(),
                                nxt.len(),
                                cap,
                                &format!("{ctx}: after-save compress"),
                            );
                            // And a second save right after the first.
                            let (cn2, rn2) = (
                                sd.0(csp, csafe.as_mut_ptr() as *mut c_char, want),
                                sd.1(rsp, rsafe.as_mut_ptr() as *mut c_char, want),
                            );
                            assert_ret_eq(cn2, rn2, &format!("{ctx}: second save"));
                            assert_bytes_eq(
                                &csafe[..cn2 as usize],
                                &rsafe[..cn2 as usize],
                                &format!("{ctx}: second save bytes"),
                            );
                        }
                        fs.0(csp);
                        fs.1(rsp);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 92 — reset paths: resetStreamHC / _fast / initStreamHC
// ---------------------------------------------------------------------------
#[test]
fn row92_reset_paths() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rst, "LZ4_resetStreamHC", FnStreamInt);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(scl, "LZ4_setCompressionLevel", FnStreamInt);
    sym!(init, "LZ4_initStreamHC", FnInit);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    let mut rng = Rng::new(0x920092);

    for &lvl in REP_LEVELS {
        for &shape in ALL_SHAPES {
            let a = gen_src(shape, 5000, &mut rng);
            let b = gen_src(shape, 5000, &mut rng);
            let cap = bound_of(&bound.0, 5000);

            // Reference: a brand-new stream compressing `b` on its own.
            let mut cref = vec![0u8; cap + 32];
            let mut rref = vec![0u8; cap + 32];
            let (crefn, rrefn) = unsafe {
                let (csp, rsp) = (cs.0(), cs.1());
                scl.0(csp, lvl);
                scl.1(rsp, lvl);
                let n = (
                    cont5.0(
                        csp,
                        b.as_ptr() as *const c_char,
                        cref.as_mut_ptr() as *mut c_char,
                        5000,
                        cap as c_int,
                    ),
                    cont5.1(
                        rsp,
                        b.as_ptr() as *const c_char,
                        rref.as_mut_ptr() as *mut c_char,
                        5000,
                        cap as c_int,
                    ),
                );
                fs.0(csp);
                fs.1(rsp);
                n
            };
            assert_out_eq(crefn, &cref, rrefn, &rref, "row92 reference stream");

            // which: 0 = resetStreamHC, 1 = resetStreamHC_fast,
            //        2 = initStreamHC, 3 = resetStreamHC_fast after a failure
            //            (the `dirty` flag forces a full re-init).
            for which in 0..4 {
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    scl.0(csp, lvl);
                    scl.1(rsp, lvl);
                    // Dirty both streams with `a`.
                    let dcap = if which == 3 { 4usize } else { cap };
                    let mut t1 = vec![0u8; dcap + 32];
                    let mut t2 = vec![0u8; dcap + 32];
                    let (dn, dr) = (
                        cont5.0(
                            csp,
                            a.as_ptr() as *const c_char,
                            t1.as_mut_ptr() as *mut c_char,
                            5000,
                            dcap as c_int,
                        ),
                        cont5.1(
                            rsp,
                            a.as_ptr() as *const c_char,
                            t2.as_mut_ptr() as *mut c_char,
                            5000,
                            dcap as c_int,
                        ),
                    );
                    assert_out_eq(dn, &t1, dr, &t2, "row92 dirty pass");
                    if which == 3 {
                        assert_eq!(dn, 0, "row92: capacity 4 should have failed");
                    }
                    match which {
                        0 => {
                            rst.0(csp, lvl);
                            rst.1(rsp, lvl);
                        }
                        1 | 3 => {
                            rf.0(csp, lvl);
                            rf.1(rsp, lvl);
                        }
                        _ => {
                            assert!(!init.0(csp, SIZEOF_LZ4_STREAMHC_T).is_null());
                            assert!(!init.1(rsp, SIZEOF_LZ4_STREAMHC_T).is_null());
                            scl.0(csp, lvl);
                            scl.1(rsp, lvl);
                        }
                    }
                    let ctx = format!("row92 reset which={which} lvl={lvl} {shape:?}");
                    let (cn, cb) = cont(&cont5.0, &cont5.1, csp, rsp, b.as_ptr(), 5000, cap, &ctx);
                    // resetStreamHC / initStreamHC (and _fast on a dirty
                    // stream) must be exactly equivalent to a fresh stream.
                    if which != 1 {
                        assert_eq!(cn, crefn, "{ctx}: not equivalent to a fresh stream");
                        assert_bytes_eq(
                            &cb[..cn as usize],
                            &cref[..crefn as usize],
                            &format!("{ctx}: fresh-stream bytes"),
                        );
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }

            // LZ4_initStreamHC on caller-owned memory (valid boundary).
            let mut ca = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
            let mut ra = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
            unsafe {
                let cp = init.0(ca.ptr() as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
                let rp = init.1(ra.ptr() as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
                assert!(!cp.is_null() && !rp.is_null(), "initStreamHC boundary");
                scl.0(cp, lvl);
                scl.1(rp, lvl);
                let ctx = format!("row92 initStreamHC own-mem lvl={lvl} {shape:?}");
                let (cn, cb) = cont(&cont5.0, &cont5.1, cp, rp, b.as_ptr(), 5000, cap, &ctx);
                assert_eq!(cn, crefn, "{ctx}: not equivalent to a fresh stream");
                assert_bytes_eq(&cb[..cn as usize], &cref[..crefn as usize], &ctx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 93 — LZ4_compress_HC_continue_destSize chain (fillOutput)
// ---------------------------------------------------------------------------
#[test]
fn row93_continue_dest_size_chain() {
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(cds, "LZ4_compress_HC_continue_destSize", FnContDestSize);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x930093);

    for &lvl in REP_LEVELS {
        for &tgt in &[0usize, 1, 2, 5, 17, 64, 300, 1024, 4096, 20_000] {
            for &shape in ALL_SHAPES {
                let total = if is_opt(lvl) { 40_000usize } else { 120_000 };
                let blk = 9000usize;
                let data = gen_src(shape, total, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    let mut off = 0usize;
                    let mut i = 0;
                    // A small `targetDestSize` consumes only a handful of source
                    // bytes per call, so cap the block count instead of walking
                    // the whole buffer a few bytes at a time.
                    while off < total && i < 30 {
                        let n = blk.min(total - off);
                        let mut csz = n as c_int;
                        let mut rsz = n as c_int;
                        let mut cb = vec![0u8; tgt + 32];
                        let mut rb = vec![0u8; tgt + 32];
                        let (cn, rn) = (
                            cds.0(
                                csp,
                                data[off..].as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                &mut csz,
                                tgt as c_int,
                            ),
                            cds.1(
                                rsp,
                                data[off..].as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                &mut rsz,
                                tgt as c_int,
                            ),
                        );
                        let ctx =
                            format!("row93 continue_destSize lvl={lvl} tgt={tgt} {shape:?} i={i}");
                        assert_eq!(csz, rsz, "{ctx}: *srcSizePtr mismatch C={csz} Rust={rsz}");
                        assert_out_eq(cn, &cb, rn, &rb, &ctx);
                        if cn > 0 {
                            assert!(cn as usize <= tgt, "{ctx}: exceeded targetDestSize");
                            let consumed = csz as usize;
                            if off == 0 {
                                check_roundtrip(
                                    &dec.0,
                                    &cb[..cn as usize],
                                    &data[..consumed],
                                    &ctx,
                                );
                            } else {
                                check_roundtrip_dict(
                                    &decd.0,
                                    &cb[..cn as usize],
                                    &data[off..off + consumed],
                                    &data[..off],
                                    &ctx,
                                );
                            }
                        }
                        if cn <= 0 || csz <= 0 {
                            break; // no progress possible with this target size
                        }
                        off += csz as usize;
                        i += 1;
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 94-95 — deprecated one-shot wrappers (plain and with an external state)
// ---------------------------------------------------------------------------
#[test]
fn rows94_95_deprecated_oneshots() {
    sym!(hc3, "LZ4_compressHC", FnHC3);
    sym!(hclo, "LZ4_compressHC_limitedOutput", FnHC4);
    sym!(hc2, "LZ4_compressHC2", FnHC4);
    sym!(hc2lo, "LZ4_compressHC2_limitedOutput", FnHC5);
    sym!(ws4, "LZ4_compressHC_withStateHC", FnExt4);
    sym!(ws5, "LZ4_compressHC_limitedOutput_withStateHC", FnExt5);
    sym!(ws2, "LZ4_compressHC2_withStateHC", FnExt5);
    sym!(ws2lo, "LZ4_compressHC2_limitedOutput_withStateHC", FnExt6);
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0x940095);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (cp, rp) = (cst.ptr() as *mut c_void, rst.ptr() as *mut c_void);

    for &len in &[0usize, 1, 13, 64, 1024, 4096, 65535, 65536, 65547, 100_000] {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            let full = bound_of(&bound.0, len);

            // LZ4_compressHC / _withStateHC: notLimited, cLevel hard-coded 0 => 9.
            let mut cb = vec![0u8; full + 32];
            let mut rb = vec![0u8; full + 32];
            let (cn, rn) = unsafe {
                (
                    hc3.0(
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                    hc3.1(
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                )
            };
            let ctx = format!("LZ4_compressHC len={len} {shape:?}");
            assert_out_eq(cn, &cb, rn, &rb, &ctx);
            // Must equal LZ4_compress_HC(.., 0) i.e. the default level 9.
            let (rn9, rb9) = oneshot(&hc.0, &hc.1, &src, full, 0, &ctx);
            assert_eq!(cn, rn9, "{ctx}: != level 9");
            assert_bytes_eq(&cb[..cn as usize], &rb9[..rn9 as usize], &ctx);
            if cn > 0 {
                check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
            }

            let mut cb = vec![0u8; full + 32];
            let mut rb = vec![0u8; full + 32];
            let (cn, rn) = unsafe {
                (
                    ws4.0(
                        cp,
                        src.as_ptr() as *const c_char,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                    ws4.1(
                        rp,
                        src.as_ptr() as *const c_char,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                    ),
                )
            };
            assert_out_eq(
                cn,
                &cb,
                rn,
                &rb,
                &format!("LZ4_compressHC_withStateHC len={len} {shape:?}"),
            );

            for &lvl in ALL_LEVELS {
                if is_opt(lvl) && len > 65_547 {
                    continue; // keep the optimal parser off the biggest inputs
                }
                // LZ4_compressHC2: notLimited at an explicit level.
                let mut cb = vec![0u8; full + 32];
                let mut rb = vec![0u8; full + 32];
                let (cn, rn) = unsafe {
                    (
                        hc2.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            lvl,
                        ),
                        hc2.1(
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            lvl,
                        ),
                    )
                };
                let ctx = format!("LZ4_compressHC2 lvl={lvl} len={len} {shape:?}");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                if cn > 0 {
                    check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                }

                let mut cb = vec![0u8; full + 32];
                let mut rb = vec![0u8; full + 32];
                let (cn, rn) = unsafe {
                    (
                        ws2.0(
                            cp,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            lvl,
                        ),
                        ws2.1(
                            rp,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            lvl,
                        ),
                    )
                };
                assert_out_eq(
                    cn,
                    &cb,
                    rn,
                    &rb,
                    &format!("LZ4_compressHC2_withStateHC lvl={lvl} len={len} {shape:?}"),
                );

                // limitedOutput variants across a capacity sweep.
                for &cap in &[0usize, 1, 5, full / 8, full / 4, full / 2, full] {
                    let mut cb = vec![0u8; cap + 32];
                    let mut rb = vec![0u8; cap + 32];
                    let (cn, rn) = unsafe {
                        (
                            hc2lo.0(
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                lvl,
                            ),
                            hc2lo.1(
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                lvl,
                            ),
                        )
                    };
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!(
                            "compressHC2_limitedOutput lvl={lvl} len={len} {shape:?} cap={cap}"
                        ),
                    );

                    let mut cb = vec![0u8; cap + 32];
                    let mut rb = vec![0u8; cap + 32];
                    let (cn, rn) = unsafe {
                        (
                            ws2lo.0(
                                cp,
                                src.as_ptr() as *const c_char,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                lvl,
                            ),
                            ws2lo.1(
                                rp,
                                src.as_ptr() as *const c_char,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                cap as c_int,
                                lvl,
                            ),
                        )
                    };
                    assert_out_eq(
                        cn,
                        &cb,
                        rn,
                        &rb,
                        &format!(
                            "compressHC2_limitedOutput_withStateHC lvl={lvl} len={len} {shape:?} cap={cap}"
                        ),
                    );
                }
            }

            // The two level-less limitedOutput wrappers (cLevel 0 => 9).
            for &cap in &[0usize, 1, 5, full / 4, full / 2, full] {
                let mut cb = vec![0u8; cap + 32];
                let mut rb = vec![0u8; cap + 32];
                let (cn, rn) = unsafe {
                    (
                        hclo.0(
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                        hclo.1(
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
                    &format!("compressHC_limitedOutput len={len} {shape:?} cap={cap}"),
                );

                let mut cb = vec![0u8; cap + 32];
                let mut rb = vec![0u8; cap + 32];
                let (cn, rn) = unsafe {
                    (
                        ws5.0(
                            cp,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            len as c_int,
                            cap as c_int,
                        ),
                        ws5.1(
                            rp,
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
                    &format!("compressHC_limitedOutput_withStateHC len={len} {shape:?} cap={cap}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 96-97 — deprecated streaming lifecycle
// ---------------------------------------------------------------------------
#[test]
fn rows96_97_deprecated_streaming() {
    sym!(create, "LZ4_createHC", FnCreateHC);
    sym!(free, "LZ4_freeHC", FnFree);
    sym!(cc, "LZ4_compressHC_continue", FnExt4);
    sym!(clc, "LZ4_compressHC_limitedOutput_continue", FnExt5);
    sym!(slide, "LZ4_slideInputBufferHC", FnSlideHC);
    sym!(rss, "LZ4_resetStreamStateHC", FnResetStateHC);
    sym!(sos, "LZ4_sizeofStateHC", FnSizeof);
    sym!(soss, "LZ4_sizeofStreamStateHC", FnSizeof);
    sym!(clmax, "LZ4F_compressionLevel_max", FnSizeof);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x960097);

    unsafe {
        assert_ret_eq(sos.0(), sos.1(), "LZ4_sizeofStateHC");
        assert_ret_eq(soss.0(), soss.1(), "LZ4_sizeofStreamStateHC");
        assert_eq!(soss.0() as usize, SIZEOF_LZ4_STREAMHC_T);
        assert_ret_eq(clmax.0(), clmax.1(), "LZ4F_compressionLevel_max");
        assert_eq!(clmax.0(), 12, "LZ4F_compressionLevel_max");
        // LZ4_freeHC(NULL) must be a no-op returning 0 (ERRORS.md row 98).
        assert_ret_eq(
            free.0(std::ptr::null_mut()),
            free.1(std::ptr::null_mut()),
            "LZ4_freeHC(NULL)",
        );
    }

    for &shape in ALL_SHAPES {
        let total = 120_000usize;
        let blk = 4096usize;
        // `data` is the buffer handed to LZ4_createHC: the chain must be
        // contiguous with it, and it must outlive the stream.
        let data = gen_src(shape, total, &mut rng);
        unsafe {
            let (cst, rst) = (
                create.0(data.as_ptr() as *const c_char),
                create.1(data.as_ptr() as *const c_char),
            );
            assert!(!cst.is_null() && !rst.is_null(), "LZ4_createHC");
            let mut off = 0usize;
            let mut i = 0;
            while off + blk <= total {
                let mut cb = vec![0u8; bound_of(&bound.0, blk) + 32];
                let mut rb = vec![0u8; bound_of(&bound.0, blk) + 32];
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
                let ctx = format!("LZ4_compressHC_continue {shape:?} i={i}");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                if cn > 0 && off > 0 {
                    check_roundtrip_dict(
                        &decd.0,
                        &cb[..cn as usize],
                        &data[off..off + blk],
                        &data[..off],
                        &ctx,
                    );
                } else if cn > 0 {
                    check_roundtrip(&dec.0, &cb[..cn as usize], &data[..blk], &ctx);
                }
                off += blk;
                i += 1;
            }

            // slideInputBufferHC returns `prefixStart - dictLimit + lowLimit`.
            // Both libraries were handed the SAME buffer, so the pointers must
            // be identical, and the stream stays usable afterwards.
            let cslid = slide.0(cst);
            let rslid = slide.1(rst);
            assert_eq!(
                cslid as usize, rslid as usize,
                "LZ4_slideInputBufferHC {shape:?}: pointer mismatch"
            );
            assert_eq!(
                cslid as usize,
                data.as_ptr() as usize,
                "LZ4_slideInputBufferHC {shape:?}: expected the buffer start"
            );
            let cap = bound_of(&bound.0, blk);
            cont(
                &cont5.0,
                &cont5.1,
                cst,
                rst,
                data.as_ptr(),
                blk,
                cap,
                &format!("after slideInputBufferHC {shape:?}"),
            );

            // LZ4_resetStreamStateHC on the same state (inverted convention:
            // 0 = success) then keep compressing.
            assert_ret_eq(
                rss.0(cst, data.as_ptr() as *mut c_char),
                rss.1(rst, data.as_ptr() as *mut c_char),
                "LZ4_resetStreamStateHC",
            );
            cont(
                &cont5.0,
                &cont5.1,
                cst,
                rst,
                data.as_ptr(),
                blk,
                cap,
                &format!("after resetStreamStateHC {shape:?}"),
            );
            assert_ret_eq(free.0(cst), free.1(rst), "LZ4_freeHC");
        }

        // limitedOutput_continue with a capacity sweep.
        unsafe {
            let (cst, rst) = (
                create.0(data.as_ptr() as *const c_char),
                create.1(data.as_ptr() as *const c_char),
            );
            let mut off = 0usize;
            let mut i = 0;
            while off + blk <= total {
                let full = bound_of(&bound.0, blk);
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
                        &format!("compressHC_limitedOutput_continue {shape:?} i={i} cap={cap}"),
                    );
                }
                off += blk;
                i += 1;
            }
            free.0(cst);
            free.1(rst);
        }

        // LZ4_resetStreamStateHC on caller-owned aligned memory, then use it.
        let mut ca = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
        let mut ra = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
        unsafe {
            let (cpp, rpp) = (ca.ptr() as *mut c_void, ra.ptr() as *mut c_void);
            assert_ret_eq(
                rss.0(cpp, data.as_ptr() as *mut c_char),
                rss.1(rpp, data.as_ptr() as *mut c_char),
                "resetStreamStateHC own memory",
            );
            let cap = bound_of(&bound.0, blk);
            cont(
                &cont5.0,
                &cont5.1,
                cpp,
                rpp,
                data.as_ptr(),
                blk,
                cap,
                &format!("resetStreamStateHC own memory compress {shape:?}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 99 — every HC-produced block round-trips back to the original bytes
// ---------------------------------------------------------------------------
#[test]
fn row99_round_trip_all_apis() {
    sym!(hc, "LZ4_compress_HC", FnHC5);
    sym!(ext, "LZ4_compress_HC_extStateHC", FnExt6);
    sym!(fast, "LZ4_compress_HC_extStateHC_fastReset", FnExt6);
    sym!(ds, "LZ4_compress_HC_destSize", FnDestSizeHC);
    sym!(cs, "LZ4_createStreamHC", FnCreate);
    sym!(fs, "LZ4_freeStreamHC", FnFree);
    sym!(rf, "LZ4_resetStreamHC_fast", FnStreamInt);
    sym!(ld, "LZ4_loadDictHC", FnLoadDict);
    sym!(cont5, "LZ4_compress_HC_continue", FnExt5);
    sym!(bound, "LZ4_compressBound", FnBound);
    sym!(dec, "LZ4_decompress_safe", FnDecSafe);
    sym!(decd, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0x990099);

    let mut cst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let mut rst = Aligned::new(SIZEOF_LZ4_STREAMHC_T);
    let (cp, rp) = (cst.ptr() as *mut c_void, rst.ptr() as *mut c_void);

    for &lvl in ALL_LEVELS {
        let lens: &[usize] = if is_opt(lvl) {
            &[1, 13, 300, 4096, 65536]
        } else {
            &[1, 13, 300, 4096, 65536, 100_000]
        };
        for &len in lens {
            for &shape in ALL_SHAPES {
                let src = gen_src(shape, len, &mut rng);
                let cap = bound_of(&bound.0, len);

                // One-shot, ext-state and fast-reset outputs.
                let ctx = format!("row99 one-shot lvl={lvl} len={len} {shape:?}");
                let (cn, cb) = oneshot(&hc.0, &hc.1, &src, cap, lvl, &ctx);
                check_roundtrip(&dec.0, &cb[..cn as usize], &src, &ctx);
                check_roundtrip(&dec.1, &cb[..cn as usize], &src, &ctx);

                let ctx = format!("row99 extState lvl={lvl} len={len} {shape:?}");
                let (cn, cb) = oneshot_state(&ext.0, &ext.1, cp, rp, &src, cap, lvl, &ctx);
                check_roundtrip(&dec.1, &cb[..cn as usize], &src, &ctx);
                let ctx = format!("row99 fastReset lvl={lvl} len={len} {shape:?}");
                let (cn, cb) = oneshot_state(&fast.0, &fast.1, cp, rp, &src, cap, lvl, &ctx);
                check_roundtrip(&dec.1, &cb[..cn as usize], &src, &ctx);

                // destSize (truncating) output.
                let tgt = (cap / 3).max(1);
                let mut csz = len as c_int;
                let mut rsz = len as c_int;
                let mut cb = vec![0u8; tgt + 32];
                let mut rb = vec![0u8; tgt + 32];
                let (cn, rn) = unsafe {
                    (
                        ds.0(
                            cp,
                            src.as_ptr() as *const c_char,
                            cb.as_mut_ptr() as *mut c_char,
                            &mut csz,
                            tgt as c_int,
                            lvl,
                        ),
                        ds.1(
                            rp,
                            src.as_ptr() as *const c_char,
                            rb.as_mut_ptr() as *mut c_char,
                            &mut rsz,
                            tgt as c_int,
                            lvl,
                        ),
                    )
                };
                let ctx = format!("row99 destSize lvl={lvl} len={len} {shape:?} tgt={tgt}");
                assert_eq!(csz, rsz, "{ctx}: *srcSizePtr mismatch");
                assert_out_eq(cn, &cb, rn, &rb, &ctx);
                if cn > 0 {
                    check_roundtrip(&dec.0, &cb[..cn as usize], &src[..csz as usize], &ctx);
                }

                // Dictionary-primed single block.
                let dict = gen_src(shape, 20_000, &mut rng);
                unsafe {
                    let (csp, rsp) = (cs.0(), cs.1());
                    rf.0(csp, lvl);
                    rf.1(rsp, lvl);
                    let cdn = ld.0(csp, dict.as_ptr() as *const c_char, dict.len() as c_int);
                    let rdn = ld.1(rsp, dict.as_ptr() as *const c_char, dict.len() as c_int);
                    assert_ret_eq(cdn, rdn, "row99 loadDictHC");
                    let ctx = format!("row99 usingDict lvl={lvl} len={len} {shape:?}");
                    let (cn, cb) = cont(&cont5.0, &cont5.1, csp, rsp, src.as_ptr(), len, cap, &ctx);
                    if cn > 0 {
                        check_roundtrip_dict(&decd.0, &cb[..cn as usize], &src, &dict, &ctx);
                        check_roundtrip_dict(&decd.1, &cb[..cn as usize], &src, &dict, &ctx);
                    }
                    fs.0(csp);
                    fs.1(rsp);
                }
            }
        }
    }
}
