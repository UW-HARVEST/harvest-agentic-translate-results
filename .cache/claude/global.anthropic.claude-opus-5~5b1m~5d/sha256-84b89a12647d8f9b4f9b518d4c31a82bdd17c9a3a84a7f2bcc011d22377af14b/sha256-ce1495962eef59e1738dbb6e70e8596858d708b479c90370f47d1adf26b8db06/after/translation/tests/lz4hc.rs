//! Differential tests for `CONFIGS.md` rows 66-103:
//! "lz4hc block" (66-84) and "lz4hc streaming + legacy" (85-103).
//!
//! Every call goes through a `.so` export (both the C reference and the Rust
//! translation) via `libloading`.  Opaque HC state is always created, used and
//! released by one and the same library -- a context produced by one library is
//! never handed to the other.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Local FFI signature aliases
// ---------------------------------------------------------------------------

/// `LZ4_compress_HC(src, dst, srcSize, dstCapacity, level)`
type FnHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_extStateHC[_fastReset](state, src, dst, srcSize, dstCap, level)`
type FnExtHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_destSize(state, src, dst, srcSizePtr, targetDstSize, level)`
type FnHCDestSize = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *mut c_char,
    *mut c_int,
    c_int,
    c_int,
) -> c_int;
/// `LZ4_initStreamHC(buffer, size)`
type FnInitStreamHC = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
/// `LZ4_resetStreamHC` / `LZ4_resetStreamHC_fast` / `LZ4_setCompressionLevel` /
/// `LZ4_favorDecompressionSpeed`
type FnStreamInt = unsafe extern "C" fn(*mut c_void, c_int);
/// `LZ4_loadDictHC(stream, dict, dictSize)`
type FnLoadDictHC = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
/// `LZ4_compress_HC_continue(stream, src, dst, srcSize, dstCap)`
type FnContHC =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compress_HC_continue_destSize(stream, src, dst, srcSizePtr, targetDstSize)`
type FnContHCDestSize =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
/// `LZ4_saveDictHC(stream, safeBuffer, dictSize)`
type FnSaveDictHC = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
/// `LZ4_attach_HC_dictionary(working, dict)`
type FnAttachHC = unsafe extern "C" fn(*mut c_void, *const c_void);
/// `LZ4_decompress_safe_usingDict(src, dst, cSize, dstCap, dictStart, dictSize)`
type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;

/// `LZ4_compressHC(src, dst, srcSize)`
type FnDep3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_compressHC_limitedOutput` / `LZ4_compressHC2`
type FnDep4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compressHC_withStateHC(state, src, dst, srcSize)`
type FnDepSt4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
/// `LZ4_compressHC_limitedOutput_withStateHC` / `LZ4_compressHC2_withStateHC` /
/// `LZ4_compressHC2_continue`
type FnDepSt5 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `LZ4_compressHC2_limitedOutput_withStateHC` / `LZ4_compressHC2_limitedOutput_continue`
type FnDepSt6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `LZ4_createHC(inputBuffer)`
type FnCreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
/// `LZ4_slideInputBufferHC(data)`
type FnSlideHC = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
/// `LZ4_resetStreamStateHC(state, inputBuffer)`
type FnResetStreamStateHC = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;

/// Mirror of the exported `LZ4HC_match_t` (`{int off; int len; int back;}`).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct HcMatch {
    off: c_int,
    len: c_int,
    back: c_int,
}

/// `LZ4HC_searchExtDict(ip, ipIndex, iLowLimit, iHighLimit, dictCtx,
///                      gDictEndIndex, currentBestML, nbAttempts)`
type FnSearchExtDict = unsafe extern "C" fn(
    *const u8,
    u32,
    *const u8,
    *const u8,
    *const c_void,
    u32,
    c_int,
    c_int,
) -> HcMatch;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Levels touching every strategy: lz4mid (1,2), hashChain (3,6,8,9) and the
/// optimal parser (10,11,12).
const LEVELS_WIDE: [c_int; 9] = [1, 2, 3, 6, 8, 9, 10, 11, 12];

/// Sentinel byte pre-filled into every destination buffer.
const SENT: u8 = 0xCD;
/// Extra guard bytes appended past the advertised capacity.
const TAIL: usize = 32;

/// `common::gen`, but guaranteeing a *real* heap allocation even for length 0.
///
/// This matters: `Vec::new().as_ptr()` is the dangling value `0x1`, and the
/// level >= 10 optimal parser computes `iend - MFLIMIT`, which then wraps around
/// to a huge address and makes `ip <= mflimit` true.  That is an artifact of
/// handing lz4 a bogus pointer, not a library bug, so the tests always pass a
/// genuine buffer.
fn gen(rng: &mut Rng, shape: Shape, len: usize) -> Vec<u8> {
    let mut v = common::gen(rng, shape, len);
    if v.capacity() == 0 {
        v.reserve(64);
    }
    v
}

fn gen_incompressible(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = common::gen_incompressible(rng, len);
    if v.capacity() == 0 {
        v.reserve(64);
    }
    v
}

/// 8-byte aligned scratch memory (for externally provided HC state buffers).
struct Aligned(Vec<u64>);

impl Aligned {
    fn new(bytes: usize) -> Aligned {
        Aligned(vec![0u64; bytes / 8 + 4])
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    /// Pointer offset by `off` bytes (used to build deliberately misaligned state).
    fn at(&mut self, off: usize) -> *mut c_void {
        unsafe { (self.0.as_mut_ptr() as *mut u8).add(off) as *mut c_void }
    }
}

unsafe fn cbound(l: &Pair, n: c_int) -> c_int {
    let (c, r) = l.sym::<FnCompressBound>("LZ4_compressBound");
    let (a, b) = (c(n), r(n));
    assert_eq!(a, b, "LZ4_compressBound({n}) mismatch");
    a
}

unsafe fn state_size(l: &Pair) -> usize {
    let (c, r) = l.sym::<FnVoidToInt>("LZ4_sizeofStateHC");
    let (a, b) = (c(), r());
    assert_eq!(a, b, "LZ4_sizeofStateHC mismatch (C={a} Rust={b})");
    assert!(a > 0);
    a as usize
}

/// `LZ4_compress_HC` on both libraries with separate sentinel-filled buffers.
unsafe fn hc_pair(
    l: &Pair,
    tag: &str,
    src: *const u8,
    ssz: c_int,
    cap: usize,
    level: c_int,
) -> (c_int, Vec<u8>, Vec<u8>) {
    let (fc, fr) = l.sym::<FnHC>("LZ4_compress_HC");
    let mut dc = vec![SENT; cap + TAIL];
    let mut dr = vec![SENT; cap + TAIL];
    let rc = fc(
        src as *const c_char,
        dc.as_mut_ptr() as *mut c_char,
        ssz,
        cap as c_int,
        level,
    );
    let rr = fr(
        src as *const c_char,
        dr.as_mut_ptr() as *mut c_char,
        ssz,
        cap as c_int,
        level,
    );
    same_int_and_bytes(tag, rc, rr, &dc, &dr);
    same_full_buffers(tag, &dc, &dr);
    assert!(
        rc <= cap as c_int,
        "{tag}: LZ4_compress_HC returned {rc} > dstCapacity {cap}"
    );
    (rc, dc, dr)
}

/// Cross-decompress: C output decoded by Rust and vice versa, with an optional
/// external dictionary (the block's history).
unsafe fn dec_cross_dict(
    l: &Pair,
    tag: &str,
    cb: &[u8],
    rb: &[u8],
    clen: c_int,
    expect: &[u8],
    dict: &[u8],
) {
    if clen <= 0 {
        return;
    }
    let (dc, dr) = l.sym::<FnDecUsingDict>("LZ4_decompress_safe_usingDict");
    let dp = if dict.is_empty() {
        ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };
    let ds = dict.len() as c_int;
    let n = expect.len();
    let mut o1 = vec![SENT; n + TAIL];
    let mut o2 = vec![SENT; n + TAIL];
    // C's compressed block, decoded by the Rust decoder.
    let n1 = dr(
        cb.as_ptr() as *const c_char,
        o1.as_mut_ptr() as *mut c_char,
        clen,
        n as c_int,
        dp,
        ds,
    );
    // Rust's compressed block, decoded by the C decoder.
    let n2 = dc(
        rb.as_ptr() as *const c_char,
        o2.as_mut_ptr() as *mut c_char,
        clen,
        n as c_int,
        dp,
        ds,
    );
    assert_eq!(
        n1, n as c_int,
        "{tag}: Rust decoder on C HC output returned {n1} (expected {n})"
    );
    assert_eq!(
        n2, n as c_int,
        "{tag}: C decoder on Rust HC output returned {n2} (expected {n})"
    );
    if let Some(i) = first_diff(&o1[..n], expect) {
        panic!("{tag}: Rust decode of C output differs from original at {i}");
    }
    if let Some(i) = first_diff(&o2[..n], expect) {
        panic!("{tag}: C decode of Rust output differs from original at {i}");
    }
    assert!(
        o1[n..].iter().all(|&b| b == SENT),
        "{tag}: Rust decoder overran destination"
    );
    assert!(
        o2[n..].iter().all(|&b| b == SENT),
        "{tag}: C decoder overran destination"
    );
}

unsafe fn dec_cross(l: &Pair, tag: &str, cb: &[u8], rb: &[u8], clen: c_int, expect: &[u8]) {
    dec_cross_dict(l, tag, cb, rb, clen, expect, &[]);
}

/// Full bound-sized `LZ4_compress_HC` case: identical output + round-trip.
unsafe fn hc_case(l: &Pair, tag: &str, src: &[u8], level: c_int) -> c_int {
    let cap = cbound(l, src.len() as c_int) as usize;
    let (ret, dc, dr) = hc_pair(l, tag, src.as_ptr(), src.len() as c_int, cap, level);
    assert!(ret > 0, "{tag}: compression unexpectedly failed (ret={ret})");
    dec_cross(l, tag, &dc, &dr, ret, src);
    ret
}

/// Level sweep shared by rows 66-74.  Levels >= 10 (optimal parser) are much
/// slower, so they get the smaller size list.
unsafe fn sweep_level(seed: u64, level: c_int) {
    let l = libs();
    let mut rng = Rng::new(seed);
    let slow = level >= LZ4HC_CLEVEL_OPT_MIN || level < 1 || level > LZ4HC_CLEVEL_MAX;
    let tiny: &[usize] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 16, 17, 19, 20, 31, 32, 33, 63,
        64, 65, 100, 127, 128, 129, 255, 256, 257, 511, 512, 513,
    ];
    let mid: &[usize] = &[1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 16384, 32768];
    let big_fast: &[usize] = &[
        65534, 65535, 65536, 65537, 65546, 65547, 65548, 65600, 131072, 262143, 262144,
        262145, 400_000, 700_000,
    ];
    let big_slow: &[usize] = &[65534, 65536, 65547, 65600, 131072, 262145];
    let mut sizes: Vec<usize> = Vec::new();
    sizes.extend_from_slice(tiny);
    sizes.extend_from_slice(mid);
    sizes.extend_from_slice(if slow { big_slow } else { big_fast });
    let reps = if slow { 3 } else { 5 };
    for &sz in &sizes {
        for shape in ALL_SHAPES {
            for rep in 0..reps {
                let src = gen(&mut rng, shape, sz);
                hc_case(
                    l,
                    &format!(
                        "LZ4_compress_HC level={level} shape={shape:?} size={sz} rep={rep}"
                    ),
                    &src,
                    level,
                );
            }
        }
    }
}

/// A buffer with `period`-byte periodicity.
fn periodic(period: usize, len: usize, seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let base: Vec<u8> = (0..period).map(|_| rng.byte()).collect();
    (0..len).map(|i| base[i % period]).collect()
}

// --- stream helpers --------------------------------------------------------

struct Streams {
    c: *mut c_void,
    r: *mut c_void,
}

unsafe fn create_streams(l: &Pair) -> Streams {
    let (fc, fr) = l.sym::<FnVoidToPtr>("LZ4_createStreamHC");
    let s = Streams { c: fc(), r: fr() };
    assert!(!s.c.is_null(), "C LZ4_createStreamHC returned NULL");
    assert!(!s.r.is_null(), "Rust LZ4_createStreamHC returned NULL");
    s
}

unsafe fn free_streams(l: &Pair, s: Streams) {
    let (fc, fr) = l.sym::<FnFreePtr>("LZ4_freeStreamHC");
    let (a, b) = (fc(s.c), fr(s.r));
    assert_eq!(a, 0, "C LZ4_freeStreamHC returned {a}");
    assert_eq!(b, 0, "Rust LZ4_freeStreamHC returned {b}");
}

unsafe fn stream_call(l: &Pair, name: &str, s: &Streams, arg: c_int) {
    let (fc, fr) = l.sym::<FnStreamInt>(name);
    fc(s.c, arg);
    fr(s.r, arg);
}

unsafe fn load_dict(l: &Pair, tag: &str, s: &Streams, dict: &[u8]) -> c_int {
    let (fc, fr) = l.sym::<FnLoadDictHC>("LZ4_loadDictHC");
    let p = if dict.is_empty() {
        ptr::null()
    } else {
        dict.as_ptr() as *const c_char
    };
    let a = fc(s.c, p, dict.len() as c_int);
    let b = fr(s.r, p, dict.len() as c_int);
    assert_eq!(a, b, "{tag}: LZ4_loadDictHC return mismatch (C={a} Rust={b})");
    a
}

/// One `LZ4_compress_HC_continue` block driven identically on both streams.
unsafe fn cont_pair(
    l: &Pair,
    tag: &str,
    s: &Streams,
    src: *const u8,
    n: usize,
    cap: usize,
) -> (c_int, Vec<u8>, Vec<u8>) {
    let (fc, fr) = l.sym::<FnContHC>("LZ4_compress_HC_continue");
    let mut dc = vec![SENT; cap + TAIL];
    let mut dr = vec![SENT; cap + TAIL];
    let rc = fc(
        s.c,
        src as *const c_char,
        dc.as_mut_ptr() as *mut c_char,
        n as c_int,
        cap as c_int,
    );
    let rr = fr(
        s.r,
        src as *const c_char,
        dr.as_mut_ptr() as *mut c_char,
        n as c_int,
        cap as c_int,
    );
    same_int_and_bytes(tag, rc, rr, &dc, &dr);
    same_full_buffers(tag, &dc, &dr);
    (rc, dc, dr)
}

/// `LZ4_compress_HC_continue_destSize` on both streams (compares `*srcSizePtr`).
unsafe fn cont_destsize_pair(
    l: &Pair,
    tag: &str,
    s: &Streams,
    src: *const u8,
    n: usize,
    target: usize,
) -> (c_int, c_int, Vec<u8>, Vec<u8>) {
    let (fc, fr) = l.sym::<FnContHCDestSize>("LZ4_compress_HC_continue_destSize");
    let mut dc = vec![SENT; target + TAIL];
    let mut dr = vec![SENT; target + TAIL];
    let mut sc = n as c_int;
    let mut sr = n as c_int;
    let rc = fc(
        s.c,
        src as *const c_char,
        dc.as_mut_ptr() as *mut c_char,
        &mut sc,
        target as c_int,
    );
    let rr = fr(
        s.r,
        src as *const c_char,
        dr.as_mut_ptr() as *mut c_char,
        &mut sr,
        target as c_int,
    );
    same_int_and_bytes(tag, rc, rr, &dc, &dr);
    same_full_buffers(tag, &dc, &dr);
    assert_eq!(sc, sr, "{tag}: *srcSizePtr mismatch (C={sc} Rust={sr})");
    assert!(
        rc <= target as c_int,
        "{tag}: ret {rc} exceeds targetDstSize {target}"
    );
    (rc, sc, dc, dr)
}

// ===========================================================================
// Row 66-74 : LZ4_compress_HC compression levels
// ===========================================================================

#[test]
fn row_66_compress_hc_level_zero_and_negative() {
    let l = libs();
    unsafe {
        for lvl in [-1000, -1, 0] {
            sweep_level(66, lvl);
        }
        // 0 / negative are coerced to LZ4HC_CLEVEL_DEFAULT: output must be
        // bit-identical to level 9 in BOTH libraries.
        let mut rng = Rng::new(66);
        for shape in ALL_SHAPES {
            for &sz in &[13usize, 500, 30000, 70000] {
                let src = gen(&mut rng, shape, sz);
                let cap = cbound(l, sz as c_int) as usize;
                let (r9, d9c, d9r) =
                    hc_pair(l, "level 9 ref", src.as_ptr(), sz as c_int, cap, 9);
                for lvl in [-1000, -1, 0] {
                    let tag = format!("HC level {lvl} == level 9 (shape={shape:?} size={sz})");
                    let (r0, d0c, d0r) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, lvl);
                    assert_eq!(r0, r9, "{tag}: size differs from level 9");
                    same_full_buffers(&format!("{tag} (C)"), &d9c, &d0c);
                    same_full_buffers(&format!("{tag} (Rust)"), &d9r, &d0r);
                }
            }
        }
    }
}

#[test]
fn row_67_compress_hc_level_1_and_2_lz4mid() {
    unsafe {
        sweep_level(67, 1);
        sweep_level(67, LZ4HC_CLEVEL_MIN);
    }
}

#[test]
fn row_68_compress_hc_level_3_and_6_hashchain() {
    unsafe {
        sweep_level(68, 3);
        sweep_level(68, 6);
    }
}

#[test]
fn row_69_compress_hc_level_8_pattern_analysis_off() {
    unsafe {
        sweep_level(69, 8);
        // level 8 keeps patternAnalysis off -> drive strongly periodic data
        let l = libs();
        for period in [1usize, 2, 4] {
            let src = periodic(period, 90_000, 69 + period as u64);
            hc_case(l, &format!("level 8 period={period}"), &src, 8);
        }
    }
}

#[test]
fn row_70_compress_hc_level_9_pattern_analysis_on() {
    unsafe {
        sweep_level(70, LZ4HC_CLEVEL_DEFAULT);
    }
}

#[test]
fn row_71_compress_hc_level_10_opt_min() {
    unsafe {
        sweep_level(71, LZ4HC_CLEVEL_OPT_MIN);
    }
}

#[test]
fn row_72_compress_hc_level_11() {
    unsafe {
        sweep_level(72, 11);
    }
}

#[test]
fn row_73_compress_hc_level_12_max() {
    unsafe {
        sweep_level(73, LZ4HC_CLEVEL_MAX);
    }
}

#[test]
fn row_74_compress_hc_level_13_and_100_clamped() {
    let l = libs();
    unsafe {
        sweep_level(74, 13);
        sweep_level(74, 100);
        let mut rng = Rng::new(74);
        for shape in ALL_SHAPES {
            for &sz in &[13usize, 700, 40000] {
                let src = gen(&mut rng, shape, sz);
                let cap = cbound(l, sz as c_int) as usize;
                let (r12, d12c, d12r) =
                    hc_pair(l, "level 12 ref", src.as_ptr(), sz as c_int, cap, 12);
                for lvl in [13, 100, 1000, c_int::MAX] {
                    let tag = format!("HC level {lvl} == level 12 (shape={shape:?} size={sz})");
                    let (rx, dxc, dxr) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, lvl);
                    assert_eq!(rx, r12, "{tag}: size differs from level 12");
                    same_full_buffers(&format!("{tag} (C)"), &d12c, &dxc);
                    same_full_buffers(&format!("{tag} (Rust)"), &d12r, &dxr);
                }
            }
        }
    }
}

// ===========================================================================
// Row 75-80 : srcSize / dstCapacity / input-shape edge cases
// ===========================================================================

#[test]
fn row_75_compress_hc_tiny_src_sizes() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(75);
        for &level in &LEVELS_WIDE {
            for &sz in &[0usize, 1, 2, 3, 11, 12, 13, 14] {
                for shape in ALL_SHAPES {
                    for _ in 0..3 {
                        let src = gen(&mut rng, shape, sz);
                        let tag =
                            format!("tiny src level={level} size={sz} shape={shape:?}");
                        let ret = hc_case(l, &tag, &src, level);
                        // < LZ4_minLength (13) is pure last-literals: one token
                        // byte (litlen < RUN_MASK) followed by the literals
                        if sz < 13 {
                            assert_eq!(
                                ret,
                                1 + sz as c_int,
                                "{tag}: expected all-literals encoding of {sz} bytes"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row_76_compress_hc_invalid_src_sizes() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(76);
        let src = gen(&mut rng, Shape::TextLike, 4096);
        for &level in &[LZ4HC_CLEVEL_MIN, LZ4HC_CLEVEL_DEFAULT, LZ4HC_CLEVEL_MAX, 0] {
            for &ssz in &[
                -1i32,
                -13,
                -4096,
                c_int::MIN,
                LZ4_MAX_INPUT_SIZE as c_int + 1,
                LZ4_MAX_INPUT_SIZE as c_int + 4096,
                c_int::MAX,
            ] {
                for &cap in &[0usize, 64, 4096 + 64] {
                    let tag = format!("invalid srcSize={ssz} level={level} cap={cap}");
                    let (ret, dc, dr) = hc_pair(l, &tag, src.as_ptr(), ssz, cap, level);
                    assert_eq!(ret, 0, "{tag}: expected 0, got {ret}");
                    // nothing at all must be written
                    assert!(
                        dc.iter().all(|&b| b == SENT),
                        "{tag}: C wrote into dst for an invalid srcSize"
                    );
                    assert!(
                        dr.iter().all(|&b| b == SENT),
                        "{tag}: Rust wrote into dst for an invalid srcSize"
                    );
                }
            }
        }
    }
}

#[test]
fn row_77_compress_hc_dst_capacity_variants() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(77);
        for &level in &LEVELS_WIDE {
            for &sz in &[13usize, 20, 100, 257, 1000, 4097, 20_000, 65_547, 70_000, 200_000]
            {
                for shape in ALL_SHAPES {
                    let src = gen(&mut rng, shape, sz);
                    let bound = cbound(l, sz as c_int) as usize;
                    // 1) dstCapacity >= bound  -> notLimited, must succeed
                    let tag = format!("cap=bound level={level} size={sz} shape={shape:?}");
                    let (r1, d1c, d1r) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, bound, level);
                    assert!(r1 > 0, "{tag}: bound-sized dst must succeed");
                    dec_cross(l, &tag, &d1c, &d1r, r1, &src);

                    // 2) exactly one byte less than bound -> limitedOutput
                    let tag = format!("cap=bound-1 level={level} size={sz} shape={shape:?}");
                    let (r2, d2c, d2r) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, bound - 1, level);
                    if r2 > 0 {
                        dec_cross(l, &tag, &d2c, &d2r, r2, &src);
                    }

                    // 3) exactly the compressed size, and one byte less
                    let tag = format!("cap=csize level={level} size={sz} shape={shape:?}");
                    let (r3, d3c, d3r) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, r1 as usize, level);
                    if r3 > 0 {
                        dec_cross(l, &tag, &d3c, &d3r, r3, &src);
                    }
                    if r1 > 1 {
                        let tag =
                            format!("cap=csize-1 level={level} size={sz} shape={shape:?}");
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, r1 as usize - 1, level);
                    }

                    // 4) far too small -> 0
                    for &cap in &[0usize, 1, 2, 3] {
                        let tag =
                            format!("cap={cap} level={level} size={sz} shape={shape:?}");
                        let (r4, _, _) =
                            hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, level);
                        assert_eq!(r4, 0, "{tag}: tiny dstCapacity must fail");
                    }
                }
            }
        }
    }
}

#[test]
fn row_78_compress_hc_repeated_patterns() {
    let l = libs();
    unsafe {
        for &level in &LEVELS_WIDE {
            for &period in &[1usize, 2, 3, 4, 5, 8, 16] {
                for &len in &[
                    13usize,
                    100,
                    1000,
                    5000,
                    65_535,
                    65_536,
                    70_000,
                    120 * 1024,
                    600 * 1024,
                ] {
                    for salt in 0..2u64 {
                        let src =
                            periodic(period, len, 78 * 1000 + period as u64 * 7 + salt);
                        hc_case(
                            l,
                            &format!(
                                "period={period} len={len} level={level} salt={salt}"
                            ),
                            &src,
                            level,
                        );
                    }
                }
            }
            // long single-byte runs (>= 100 KB)
            for &b in &[0u8, 0x5A, 0xFF] {
                for &len in &[100 * 1024usize, 130 * 1024, 700 * 1024] {
                    let src = vec![b; len];
                    hc_case(
                        l,
                        &format!("run byte={b} len={len} level={level}"),
                        &src,
                        level,
                    );
                }
            }
            // pattern with a break: exercises reverseCountPattern / protectDictEnd
            let mut rng = Rng::new(78 + level as u64);
            let mut src = gen(&mut rng, Shape::Incompressible, 300);
            src.extend_from_slice(&periodic(1, 100 * 1024, 781));
            src.extend_from_slice(&gen(&mut rng, Shape::Incompressible, 300));
            src.extend_from_slice(&periodic(2, 100 * 1024, 782));
            src.extend_from_slice(&gen(&mut rng, Shape::Incompressible, 300));
            src.extend_from_slice(&periodic(4, 100 * 1024, 784));
            hc_case(l, &format!("pattern+break level={level}"), &src, level);
        }
    }
}

#[test]
fn row_79_compress_hc_incompressible() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(79);
        for &level in &[1, LZ4HC_CLEVEL_MIN, 3, 6, 8, LZ4HC_CLEVEL_DEFAULT, 10, 11, LZ4HC_CLEVEL_MAX]
        {
            for &sz in &[13usize, 100, 5000, 70_000, 200_000, 1_000_000] {
                for rep in 0..3 {
                    let src = gen_incompressible(&mut rng, sz);
                    hc_case(
                        l,
                        &format!("incompressible level={level} size={sz} rep={rep}"),
                        &src,
                        level,
                    );
                }
            }
        }
    }
}

#[test]
fn row_80_compress_hc_large_inputs_and_long_match() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(80);
        // > 64 KB and multi-MB inputs at the cheap levels
        for &level in &[1, LZ4HC_CLEVEL_MIN, LZ4HC_CLEVEL_DEFAULT] {
            for &sz in &[65_600usize, 200_000, 1_000_000, 4_500_000] {
                for shape in ALL_SHAPES {
                    if sz >= 4_000_000 && level == LZ4HC_CLEVEL_DEFAULT && shape == Shape::Periodic
                    {
                        continue; // redundant with row 78 and comparatively slow
                    }
                    let src = gen(&mut rng, shape, sz);
                    hc_case(
                        l,
                        &format!("large level={level} size={sz} shape={shape:?}"),
                        &src,
                        level,
                    );
                }
            }
        }
        // one 8 MB input at level 2 (chainTable / DELTANEXTU16 wrap-around)
        for shape in [Shape::TextLike, Shape::Compressible] {
            let src = gen(&mut rng, shape, 8 * 1024 * 1024);
            hc_case(l, &format!("8MB level=2 shape={shape:?}"), &src, 2);
        }
        // LZ4_DISTANCE_MAX clamping: a match ~68 KB back (out of range) plus one
        // just inside the window.
        let head = gen_incompressible(&mut rng, 8192);
        let mut src = head.clone();
        src.extend_from_slice(&gen_incompressible(&mut rng, 70_000));
        src.extend_from_slice(&head); // > 64 KB away
        src.extend_from_slice(&gen_incompressible(&mut rng, 1000));
        src.extend_from_slice(&head); // within 64 KB of the previous copy
        for &level in &[1, 2, 3, 6, 8, 9, 10, 11, 12] {
            hc_case(l, &format!("distance-max level={level}"), &src, level);
        }
        // single match longer than LZ4_OPT_NUM (4096) at level 12
        for &mlen in &[4095usize, 4096, 4097, 8192, 20_000, 70_000] {
            let blk = gen(&mut rng, Shape::TextLike, mlen);
            let mut src = gen_incompressible(&mut rng, 500);
            src.extend_from_slice(&blk);
            src.extend_from_slice(&gen_incompressible(&mut rng, 500));
            src.extend_from_slice(&blk);
            for &level in &[2, 9, 10, 11, 12] {
                hc_case(
                    l,
                    &format!("long match mlen={mlen} level={level}"),
                    &src,
                    level,
                );
            }
        }
    }
}

// ===========================================================================
// Row 81-83 : external state, fastReset, destSize
// ===========================================================================

#[test]
fn row_81_sizeof_state_hc_and_ext_state_hc() {
    let l = libs();
    unsafe {
        let ssz = state_size(l);
        let (fc, fr) = l.sym::<FnExtHC>("LZ4_compress_HC_extStateHC");
        let mut rng = Rng::new(81);
        // Each library gets its OWN state buffer.
        let mut sc = Aligned::new(ssz);
        let mut sr = Aligned::new(ssz);
        for &level in &LEVELS_WIDE {
            for &sz in &[0usize, 1, 12, 13, 700, 4097, 20_000, 65_547, 70_000, 200_000] {
                for shape in ALL_SHAPES {
                    let src = gen(&mut rng, shape, sz);
                    let cap = cbound(l, sz as c_int) as usize;
                    let mut dc = vec![SENT; cap + TAIL];
                    let mut dr = vec![SENT; cap + TAIL];
                    let tag =
                        format!("extStateHC level={level} size={sz} shape={shape:?}");
                    let rc = fc(
                        sc.ptr(),
                        src.as_ptr() as *const c_char,
                        dc.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                        level,
                    );
                    let rr = fr(
                        sr.ptr(),
                        src.as_ptr() as *const c_char,
                        dr.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                        level,
                    );
                    same_int_and_bytes(&tag, rc, rr, &dc, &dr);
                    same_full_buffers(&tag, &dc, &dr);
                    assert!(rc > 0, "{tag}: expected success");
                    dec_cross(l, &tag, &dc, &dr, rc, &src);
                    // must be identical to the one-shot LZ4_compress_HC
                    let (r1, d1c, d1r) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, level);
                    assert_eq!(r1, rc, "{tag}: extStateHC != LZ4_compress_HC");
                    same_full_buffers(&format!("{tag} vs one-shot (C)"), &d1c, &dc);
                    same_full_buffers(&format!("{tag} vs one-shot (Rust)"), &d1r, &dr);
                }
            }
        }

        // Misaligned state -> 0 (LZ4_initStreamHC rejects it).
        let src = gen(&mut rng, Shape::TextLike, 5000);
        let cap = cbound(l, 5000) as usize;
        let mut big_c = Aligned::new(ssz + 16);
        let mut big_r = Aligned::new(ssz + 16);
        for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
            for &level in &[2, 9, 12] {
                let mut dc = vec![SENT; cap + TAIL];
                let mut dr = vec![SENT; cap + TAIL];
                let tag = format!("misaligned extStateHC off={off} level={level}");
                let rc = fc(
                    big_c.at(off),
                    src.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    level,
                );
                let rr = fr(
                    big_r.at(off),
                    src.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    5000,
                    cap as c_int,
                    level,
                );
                assert_eq!(rc, 0, "{tag}: C returned {rc}, expected 0");
                assert_eq!(rr, 0, "{tag}: Rust returned {rr}, expected 0");
                same_full_buffers(&tag, &dc, &dr);
            }
        }

        // Undersized state: only observable through LZ4_initStreamHC (the
        // extStateHC entry points hard-code sizeof(LZ4_streamHC_t)).
        let (ic, ir) = l.sym::<FnInitStreamHC>("LZ4_initStreamHC");
        for &size in &[0usize, 1, 8, 4096, 262_199] {
            let a = ic(big_c.ptr(), size);
            let b = ir(big_r.ptr(), size);
            assert!(
                a.is_null(),
                "C LZ4_initStreamHC(size={size}) should return NULL"
            );
            assert!(
                b.is_null(),
                "Rust LZ4_initStreamHC(size={size}) should return NULL"
            );
        }
    }
}

#[test]
fn row_82_ext_state_hc_fast_reset() {
    let l = libs();
    unsafe {
        let ssz = state_size(l);
        let (ic, ir) = l.sym::<FnInitStreamHC>("LZ4_initStreamHC");
        let (fc, fr) = l.sym::<FnExtHC>("LZ4_compress_HC_extStateHC_fastReset");
        let mut sc = Aligned::new(ssz);
        let mut sr = Aligned::new(ssz);
        assert!(!ic(sc.ptr(), ssz).is_null(), "C init of ext state failed");
        assert!(!ir(sr.ptr(), ssz).is_null(), "Rust init of ext state failed");

        let mut rng = Rng::new(82);
        // Levels chosen so every mid <-> hashChain <-> optimal transition occurs.
        let levels = [
            2, 9, 12, 2, 12, 3, 10, 1, 11, 8, 2, 9, 1, 10, 2, 11, 9, 1, 12, 6, 2, 10, 3, 1,
        ];
        for (i, &level) in levels.iter().enumerate() {
            for shape in ALL_SHAPES {
                let sz = [13usize, 800, 9000, 40_000, 1, 70_000, 200_000, 4097][i % 8];
                let src = gen(&mut rng, shape, sz);
                let cap = cbound(l, sz as c_int) as usize;
                let mut dc = vec![SENT; cap + TAIL];
                let mut dr = vec![SENT; cap + TAIL];
                let tag = format!(
                    "fastReset call#{i} level={level} size={sz} shape={shape:?}"
                );
                let rc = fc(
                    sc.ptr(),
                    src.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                    cap as c_int,
                    level,
                );
                let rr = fr(
                    sr.ptr(),
                    src.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                    cap as c_int,
                    level,
                );
                same_int_and_bytes(&tag, rc, rr, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(rc > 0, "{tag}: expected success");
                dec_cross(l, &tag, &dc, &dr, rc, &src);
            }
        }

        // A limited-output failure leaves the state dirty; fastReset must cope.
        let src = gen(&mut rng, Shape::Compressible, 30_000);
        let mut dc = vec![SENT; 40];
        let mut dr = vec![SENT; 40];
        let rc = fc(
            sc.ptr(),
            src.as_ptr() as *const c_char,
            dc.as_mut_ptr() as *mut c_char,
            30_000,
            8,
            9,
        );
        let rr = fr(
            sr.ptr(),
            src.as_ptr() as *const c_char,
            dr.as_mut_ptr() as *mut c_char,
            30_000,
            8,
            9,
        );
        assert_eq!(rc, 0, "expected failure with dstCapacity=8");
        assert_eq!(rr, 0, "expected failure with dstCapacity=8");
        same_full_buffers("fastReset failure buffers", &dc, &dr);
        for &level in &[2, 9, 12] {
            let cap = cbound(l, 30_000) as usize;
            let mut dc = vec![SENT; cap + TAIL];
            let mut dr = vec![SENT; cap + TAIL];
            let tag = format!("fastReset after dirty level={level}");
            let rc = fc(
                sc.ptr(),
                src.as_ptr() as *const c_char,
                dc.as_mut_ptr() as *mut c_char,
                30_000,
                cap as c_int,
                level,
            );
            let rr = fr(
                sr.ptr(),
                src.as_ptr() as *const c_char,
                dr.as_mut_ptr() as *mut c_char,
                30_000,
                cap as c_int,
                level,
            );
            same_int_and_bytes(&tag, rc, rr, &dc, &dr);
            same_full_buffers(&tag, &dc, &dr);
            dec_cross(l, &tag, &dc, &dr, rc, &src);
        }

        // Misaligned state is rejected without touching it.
        let mut bc = Aligned::new(ssz + 16);
        let mut br = Aligned::new(ssz + 16);
        let mut dc = vec![SENT; 64];
        let mut dr = vec![SENT; 64];
        let a = fc(
            bc.at(3),
            src.as_ptr() as *const c_char,
            dc.as_mut_ptr() as *mut c_char,
            13,
            64,
            9,
        );
        let b = fr(
            br.at(3),
            src.as_ptr() as *const c_char,
            dr.as_mut_ptr() as *mut c_char,
            13,
            64,
            9,
        );
        assert_eq!(a, 0, "C fastReset with misaligned state returned {a}");
        assert_eq!(b, 0, "Rust fastReset with misaligned state returned {b}");
    }
}

#[test]
fn row_83_compress_hc_dest_size() {
    let l = libs();
    unsafe {
        let ssz = state_size(l);
        let (fc, fr) = l.sym::<FnHCDestSize>("LZ4_compress_HC_destSize");
        let mut sc = Aligned::new(ssz);
        let mut sr = Aligned::new(ssz);
        let mut rng = Rng::new(83);
        for &level in &LEVELS_WIDE {
            for &sz in &[0usize, 1, 13, 14, 200, 5000, 40_000, 70_000, 200_000] {
                for shape in ALL_SHAPES {
                    let src = gen(&mut rng, shape, sz);
                    let bound = cbound(l, sz as c_int) as usize;
                    let targets = [
                        0usize,
                        1,
                        2,
                        5,
                        16,
                        64,
                        bound / 16,
                        bound / 4,
                        bound / 2,
                        (bound * 3) / 4,
                        bound.saturating_sub(1),
                        bound,
                    ];
                    for &target in &targets {
                        let tag = format!(
                            "destSize level={level} size={sz} shape={shape:?} target={target}"
                        );
                        let mut dc = vec![SENT; target + TAIL];
                        let mut dr = vec![SENT; target + TAIL];
                        let mut nc = sz as c_int;
                        let mut nr = sz as c_int;
                        let rc = fc(
                            sc.ptr(),
                            src.as_ptr() as *const c_char,
                            dc.as_mut_ptr() as *mut c_char,
                            &mut nc,
                            target as c_int,
                            level,
                        );
                        let rr = fr(
                            sr.ptr(),
                            src.as_ptr() as *const c_char,
                            dr.as_mut_ptr() as *mut c_char,
                            &mut nr,
                            target as c_int,
                            level,
                        );
                        same_int_and_bytes(&tag, rc, rr, &dc, &dr);
                        same_full_buffers(&tag, &dc, &dr);
                        assert_eq!(nc, nr, "{tag}: *srcSizePtr mismatch C={nc} Rust={nr}");
                        assert!(
                            rc <= target as c_int,
                            "{tag}: ret {rc} > targetDstSize {target}"
                        );
                        if rc > 0 {
                            assert!(
                                nc >= 0 && nc <= sz as c_int,
                                "{tag}: bogus *srcSizePtr {nc}"
                            );
                            dec_cross(l, &tag, &dc, &dr, rc, &src[..nc as usize]);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 84 : favorDecompressionSpeed + LZ4HC_searchExtDict
// ===========================================================================

#[test]
fn row_84_favor_decompression_speed_and_search_ext_dict() {
    let l = libs();
    unsafe {
        // --- part 1: LZ4_favorDecompressionSpeed on a stream, levels 10 / 12 --
        let mut rng = Rng::new(84);
        let mut favor_changes_output = false;
        for &level in &[LZ4HC_CLEVEL_OPT_MIN, LZ4HC_CLEVEL_MAX] {
            for &sz in &[13usize, 2000, 30_000, 80_000, 200_000] {
                for shape in ALL_SHAPES {
                    let src = gen(&mut rng, shape, sz);
                    let cap = cbound(l, sz as c_int) as usize;
                    let mut outs: Vec<(c_int, Vec<u8>)> = Vec::new();
                    for &favor in &[1, 0] {
                        let s = create_streams(l);
                        stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                        stream_call(l, "LZ4_favorDecompressionSpeed", &s, favor);
                        let tag = format!(
                            "favor={favor} level={level} size={sz} shape={shape:?}"
                        );
                        let (ret, dc, dr) = cont_pair(l, &tag, &s, src.as_ptr(), sz, cap);
                        assert!(ret > 0, "{tag}: expected success");
                        dec_cross(l, &tag, &dc, &dr, ret, &src);
                        outs.push((ret, dc[..ret as usize].to_vec()));
                        free_streams(l, s);
                    }
                    if outs[0] != outs[1] {
                        favor_changes_output = true;
                    }
                }
            }
        }
        assert!(
            favor_changes_output,
            "LZ4_favorDecompressionSpeed never changed the output -- test is not \
             exercising the flag"
        );
        // favor=1 on a level below OPT_MIN must be a no-op for both
        for &level in &[2, 9] {
            let src = gen(&mut rng, Shape::TextLike, 40_000);
            let cap = cbound(l, src.len() as c_int) as usize;
            let mut outs: Vec<Vec<u8>> = Vec::new();
            for &favor in &[0, 1] {
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                stream_call(l, "LZ4_favorDecompressionSpeed", &s, favor);
                let tag = format!("favor={favor} level={level} (no-op expected)");
                let (ret, dc, _dr) = cont_pair(l, &tag, &s, src.as_ptr(), src.len(), cap);
                outs.push(dc[..ret as usize].to_vec());
                free_streams(l, s);
            }
            assert_eq!(
                outs[0], outs[1],
                "favorDecSpeed must not affect level {level}"
            );
        }

        // --- part 2: direct LZ4HC_searchExtDict against each library's own HC
        //             dictionary context (nbAttempts = 2, as LZ4MID_searchHCDict
        //             uses, plus 1) ------------------------------------------------
        let (sc, sr) = l.sym::<FnSearchExtDict>("LZ4HC_searchExtDict");
        // Pad in front of the dictionary so that a chain walk that steps below
        // the dictionary start still reads mapped (and identical) memory.
        const PAD: usize = 65_536;
        let mut ext_matches = 0usize;
        let mut ext_backs = 0usize;
        for &dict_size in &[64usize, 4096, 8192, 40_000, 65_536] {
            let mut padded = gen_incompressible(&mut rng, PAD);
            let dict_body = gen(&mut rng, Shape::TextLike, dict_size);
            padded.extend_from_slice(&dict_body);
            padded.extend_from_slice(&gen_incompressible(&mut rng, 4096));
            let dict = &padded[PAD..PAD + dict_size];

            // src replays chunks of the dictionary, including its very end.
            let mut src = Vec::new();
            let a = dict_size / 3;
            src.extend_from_slice(&dict[a..(a + 900).min(dict_size)]);
            src.extend_from_slice(&gen_incompressible(&mut rng, 200));
            src.extend_from_slice(&dict[dict_size.saturating_sub(600)..]);
            src.extend_from_slice(&gen_incompressible(&mut rng, 200));
            src.extend_from_slice(&dict[..800.min(dict_size)]);
            src.extend_from_slice(&gen_incompressible(&mut rng, 64));

            for &dict_level in &[3, 6, 9, 11, 12] {
                let ds = create_streams(l);
                stream_call(l, "LZ4_setCompressionLevel", &ds, dict_level);
                let got = load_dict(
                    l,
                    &format!("searchExtDict dict load size={dict_size}"),
                    &ds,
                    dict,
                );
                assert_eq!(got, dict_size as c_int);

                let l_dict_end_index = dict_size as u32 + 65_536;
                for &g_shift in &[0i64, 1, 100, 4096, 30_000] {
                    let g_dict_end_index =
                        (l_dict_end_index as i64 + g_shift) as u32;
                    for &nb in &[1, 2] {
                        for &best in &[0, 3] {
                            let mut k = 0usize;
                            while k + 8 < src.len() {
                                let ip = src.as_ptr().add(k);
                                let ip_index = g_dict_end_index + k as u32;
                                let hi = src.as_ptr().add(src.len() - 5);
                                for &low in &[src.as_ptr(), ip] {
                                    let a = sc(
                                        ip,
                                        ip_index,
                                        low,
                                        hi,
                                        ds.c as *const c_void,
                                        g_dict_end_index,
                                        best,
                                        nb,
                                    );
                                    let b = sr(
                                        ip,
                                        ip_index,
                                        low,
                                        hi,
                                        ds.r as *const c_void,
                                        g_dict_end_index,
                                        best,
                                        nb,
                                    );
                                    assert_eq!(
                                        a, b,
                                        "LZ4HC_searchExtDict mismatch: dictSize={dict_size} \
                                         dictLevel={dict_level} gShift={g_shift} nb={nb} \
                                         best={best} k={k}: C={a:?} Rust={b:?}"
                                    );
                                    if a.len > best {
                                        ext_matches += 1;
                                        if a.back != 0 {
                                            ext_backs += 1;
                                        }
                                    }
                                }
                                k += 11;
                            }
                        }
                    }
                }
                free_streams(l, ds);
            }
        }
        // Guard against a vacuous test: the direct calls must really locate
        // matches inside the dictionary (and at least once walk backwards).
        assert!(
            ext_matches > 100,
            "LZ4HC_searchExtDict found only {ext_matches} matches -- the direct \
             sweep is not exercising the dictionary chain"
        );
        assert!(
            ext_backs > 0,
            "LZ4HC_searchExtDict never reported a non-zero `back` (LZ4HC_countBack \
             path uncovered)"
        );

        // --- part 3: the same code path indirectly, through a level-2 working
        //             stream with an attached HC (level 9) dictionary ---------
        let dict = gen(&mut rng, Shape::TextLike, 40_000);
        for &work_level in &[1, 2] {
            for &dict_level in &[9, 12] {
                let ds = create_streams(l);
                stream_call(l, "LZ4_setCompressionLevel", &ds, dict_level);
                load_dict(l, "attach dict for searchExtDict", &ds, &dict);
                let ws = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &ws, work_level);
                let (ac, ar) = l.sym::<FnAttachHC>("LZ4_attach_HC_dictionary");
                ac(ws.c, ds.c as *const c_void);
                ar(ws.r, ds.r as *const c_void);
                // < 4 KB keeps the usingDictCtxHc path (no memcpy shortcut)
                let mut blk = Vec::new();
                blk.extend_from_slice(&dict[30_000..33_000]);
                let cap = cbound(l, blk.len() as c_int) as usize;
                let tag =
                    format!("searchExtDict via mid dictCtx work={work_level} dict={dict_level}");
                let (ret, dc, dr) = cont_pair(l, &tag, &ws, blk.as_ptr(), blk.len(), cap);
                assert!(ret > 0, "{tag}: expected success");
                dec_cross_dict(l, &tag, &dc, &dr, ret, &blk, &dict);
                free_streams(l, ws);
                free_streams(l, ds);
            }
        }
    }
}

// ===========================================================================
// Row 85-88 : stream creation / reset / level changes
// ===========================================================================

#[test]
fn row_85_create_free_init_stream_hc() {
    let l = libs();
    unsafe {
        // create / free / free(NULL)
        for _ in 0..8 {
            let s = create_streams(l);
            free_streams(l, s);
        }
        let (freec, freer) = l.sym::<FnFreePtr>("LZ4_freeStreamHC");
        assert_eq!(freec(ptr::null_mut()), 0, "C LZ4_freeStreamHC(NULL)");
        assert_eq!(freer(ptr::null_mut()), 0, "Rust LZ4_freeStreamHC(NULL)");

        // a freshly created stream must default to LZ4HC_CLEVEL_DEFAULT
        let mut rng = Rng::new(85);
        for shape in ALL_SHAPES {
            for &sz in &[1usize, 13, 900, 40_000, 70_000, 200_000] {
                let src = gen(&mut rng, shape, sz);
                let cap = cbound(l, sz as c_int) as usize;
                let s = create_streams(l);
                let tag = format!("fresh stream default level shape={shape:?} size={sz}");
                let (ret, dc, dr) = cont_pair(l, &tag, &s, src.as_ptr(), sz, cap);
                assert!(ret > 0, "{tag}: expected success");
                dec_cross(l, &tag, &dc, &dr, ret, &src);
                free_streams(l, s);
                let (r9, d9c, d9r) =
                    hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, LZ4HC_CLEVEL_DEFAULT);
                assert_eq!(ret, r9, "{tag}: fresh stream != one-shot level 9");
                same_full_buffers(&format!("{tag} C vs one-shot"), &dc, &d9c);
                same_full_buffers(&format!("{tag} Rust vs one-shot"), &dr, &d9r);
            }
        }

        // LZ4_initStreamHC: valid / undersized / misaligned / NULL
        let ssz = state_size(l);
        let (ic, ir) = l.sym::<FnInitStreamHC>("LZ4_initStreamHC");
        let mut bc = Aligned::new(ssz + 16);
        let mut br = Aligned::new(ssz + 16);
        let (pc, pr) = (bc.ptr(), br.ptr());
        let a = ic(pc, ssz);
        let b = ir(pr, ssz);
        assert_eq!(a, pc as *mut c_void, "C initStreamHC should return buffer");
        assert_eq!(b, pr as *mut c_void, "Rust initStreamHC should return buffer");
        for &size in &[0usize, 1, 7, 8, 1024, ssz - 1] {
            let a = ic(pc, size);
            let b = ir(pr, size);
            assert!(a.is_null(), "C initStreamHC(size={size}) != NULL");
            assert!(b.is_null(), "Rust initStreamHC(size={size}) != NULL");
        }
        for &off in &[1usize, 2, 3, 4, 5, 6, 7] {
            let a = ic(bc.at(off), ssz);
            let b = ir(br.at(off), ssz);
            assert!(a.is_null(), "C initStreamHC(misaligned+{off}) != NULL");
            assert!(b.is_null(), "Rust initStreamHC(misaligned+{off}) != NULL");
        }
        assert!(ic(ptr::null_mut(), ssz).is_null(), "C initStreamHC(NULL)");
        assert!(ir(ptr::null_mut(), ssz).is_null(), "Rust initStreamHC(NULL)");
        assert!(ic(ptr::null_mut(), 0).is_null(), "C initStreamHC(NULL,0)");
        assert!(ir(ptr::null_mut(), 0).is_null(), "Rust initStreamHC(NULL,0)");

        // an externally initialised stream behaves exactly like a created one
        assert!(!ic(pc, ssz).is_null());
        assert!(!ir(pr, ssz).is_null());
        let s = Streams { c: pc, r: pr };
        let src = gen(&mut rng, Shape::TextLike, 12_000);
        let cap = cbound(l, 12_000) as usize;
        let (ret, dc, dr) = cont_pair(l, "initStreamHC stream", &s, src.as_ptr(), 12_000, cap);
        assert!(ret > 0);
        dec_cross(l, "initStreamHC stream", &dc, &dr, ret, &src);
    }
}

#[test]
fn row_86_reset_stream_hc_levels() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(86);
        // level -> effective level after LZ4_setCompressionLevel clamping
        let cases: [(c_int, c_int); 14] = [
            (0, 9),
            (-1, 9),
            (-1000, 9),
            (c_int::MIN, 9),
            (1, 1),
            (2, 2),
            (3, 3),
            (6, 6),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 12),
        ];
        for (level, effective) in cases {
            for shape in ALL_SHAPES {
                for &sz in &[13usize, 1000, 40_000, 70_000] {
                    let src = gen(&mut rng, shape, sz);
                    let cap = cbound(l, sz as c_int) as usize;
                    let s = create_streams(l);
                    stream_call(l, "LZ4_resetStreamHC", &s, level);
                    let tag =
                        format!("resetStreamHC level={level} shape={shape:?} size={sz}");
                    let (ret, dc, dr) = cont_pair(l, &tag, &s, src.as_ptr(), sz, cap);
                    assert!(ret > 0, "{tag}: expected success");
                    dec_cross(l, &tag, &dc, &dr, ret, &src);
                    free_streams(l, s);
                    // clamping check against the one-shot API
                    let (re, dec, der) =
                        hc_pair(l, &tag, src.as_ptr(), sz as c_int, cap, effective);
                    assert_eq!(ret, re, "{tag}: level {level} should behave as {effective}");
                    same_full_buffers(&format!("{tag} C clamp"), &dc, &dec);
                    same_full_buffers(&format!("{tag} Rust clamp"), &dr, &der);
                }
            }
        }
        // LZ4_resetStreamHC also works as a "re-init from garbage" for a stream
        // that has already been used.
        let s = create_streams(l);
        let a = gen(&mut rng, Shape::TextLike, 20_000);
        let cap = cbound(l, 20_000) as usize;
        for &level in &[2, 9, 12, 0, 13] {
            stream_call(l, "LZ4_resetStreamHC", &s, level);
            let tag = format!("resetStreamHC reuse level={level}");
            let (ret, dc, dr) = cont_pair(l, &tag, &s, a.as_ptr(), 20_000, cap);
            assert!(ret > 0);
            dec_cross(l, &tag, &dc, &dr, ret, &a);
        }
        free_streams(l, s);
    }
}

#[test]
fn row_87_reset_stream_hc_fast_clean_and_dirty() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(87);
        let a = gen(&mut rng, Shape::TextLike, 120_000);
        let b = gen(&mut rng, Shape::Compressible, 120_000);
        let cap = cbound(l, 120_000) as usize;

        // --- clean stream (dirty == 0): cheap reset -------------------------
        for &level in &LEVELS_WIDE {
            let s = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            let tag = format!("resetFast clean level={level} block1");
            let (r1, d1c, d1r) = cont_pair(l, &tag, &s, a.as_ptr(), 120_000, cap);
            assert!(r1 > 0, "{tag}: expected success");
            dec_cross(l, &tag, &d1c, &d1r, r1, &a);
            // cheap reset then a brand new stream of blocks from another buffer
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            let tag = format!("resetFast clean level={level} block2");
            let (r2, d2c, d2r) = cont_pair(l, &tag, &s, b.as_ptr(), 120_000, cap);
            assert!(r2 > 0, "{tag}: expected success");
            dec_cross(l, &tag, &d2c, &d2r, r2, &b);
            free_streams(l, s);
        }

        // --- dirty stream: full LZ4_initStreamHC ----------------------------
        for &level in &LEVELS_WIDE {
            let s = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            // force a failure -> dirty = 1
            let tag = format!("resetFast dirty level={level} failing block");
            let (rf, _, _) = cont_pair(l, &tag, &s, a.as_ptr(), 120_000, 12);
            assert_eq!(rf, 0, "{tag}: expected failure");
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            let tag = format!("resetFast dirty level={level} recovery");
            let (r2, d2c, d2r) = cont_pair(l, &tag, &s, b.as_ptr(), 120_000, cap);
            assert!(r2 > 0, "{tag}: expected success after dirty reset");
            dec_cross(l, &tag, &d2c, &d2r, r2, &b);
            free_streams(l, s);
            // a dirty resetFast is a full re-init, so the output must equal a
            // brand new stream's output
            let s2 = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s2, level);
            let tag2 = format!("fresh reference level={level}");
            let (r3, d3c, d3r) = cont_pair(l, &tag2, &s2, b.as_ptr(), 120_000, cap);
            assert_eq!(r2, r3, "{tag}: dirty resetFast != fresh stream");
            same_full_buffers(&format!("{tag} C vs fresh"), &d2c, &d3c);
            same_full_buffers(&format!("{tag} Rust vs fresh"), &d2r, &d3r);
            free_streams(l, s2);
        }
    }
}

#[test]
fn row_88_set_compression_level_between_blocks() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(88);
        let whole = gen(&mut rng, Shape::TextLike, 1_000_000);
        let blk = 40_000usize;
        // sequences crossing every mid <-> hashChain <-> optimal boundary
        let seqs: [&[c_int]; 8] = [
            &[2, 9, 12, 2, 12, 9, 3, 10, 1, 11, 8, 2],
            &[12, 11, 10, 9, 8, 6, 3, 2, 1, 2, 9, 12],
            &[9, 9, 2, 2, 12, 12, 9, 2, 10, 3, 11, 1],
            &[1, 12, 1, 12, 1, 12, 2, 10, 2, 10, 2, 10],
            &[2, 3, 2, 3, 2, 3, 1, 6, 1, 6, 1, 6],
            &[10, 2, 11, 1, 12, 2, 8, 1, 9, 2, 6, 1],
            &[9, 12, 11, 10, 9, 8, 6, 3, 2, 1, 12, 2],
            &[0, 13, -5, 100, 2, 9, 12, 1, 3, 10, 8, 11],
        ];
        for (si, seq) in seqs.iter().enumerate() {
            let s = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s, seq[0]);
            for (i, &level) in seq.iter().enumerate() {
                stream_call(l, "LZ4_setCompressionLevel", &s, level);
                let off = i * blk;
                let cap = cbound(l, blk as c_int) as usize;
                let tag = format!("setLevel seq#{si} block#{i} level={level}");
                let (ret, dc, dr) =
                    cont_pair(l, &tag, &s, whole[off..].as_ptr(), blk, cap);
                assert!(ret > 0, "{tag}: expected success");
                dec_cross_dict(
                    l,
                    &tag,
                    &dc,
                    &dr,
                    ret,
                    &whole[off..off + blk],
                    &whole[..off],
                );
            }
            free_streams(l, s);
        }
    }
}

// ===========================================================================
// Row 89-90 : LZ4_loadDictHC
// ===========================================================================

#[test]
fn row_89_load_dict_hc_sizes() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(89);
        let big = gen(&mut rng, Shape::TextLike, 140 * 1024);
        for &level in &LEVELS_WIDE {
            for &ds in &[
                0usize, 1, 3, 4, 5, 8, 4096, 32 * 1024, 65_535, 65_536, 65_537,
                100 * 1024, 140 * 1024,
            ] {
                let dict = &big[..ds];
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let tag = format!("loadDictHC dictSize={ds} level={level}");
                let got = load_dict(l, &tag, &s, dict);
                let expect = if ds > 65_536 { 65_536 } else { ds } as c_int;
                assert_eq!(got, expect, "{tag}: return value");

                // a block that matches into the dictionary
                let mut blk = Vec::new();
                if ds >= 64 {
                    blk.extend_from_slice(&dict[ds - 64.min(ds)..]);
                }
                blk.extend_from_slice(&gen(&mut rng, Shape::TextLike, 30_000));
                if ds >= 2048 {
                    blk.extend_from_slice(&dict[ds - 2048..ds - 1024]);
                }
                let cap = cbound(l, blk.len() as c_int) as usize;
                let (ret, dc, dr) = cont_pair(l, &tag, &s, blk.as_ptr(), blk.len(), cap);
                assert!(ret > 0, "{tag}: expected success");
                let eff = &dict[ds.saturating_sub(65_536)..];
                dec_cross_dict(l, &tag, &dc, &dr, ret, &blk, eff);
                free_streams(l, s);
            }
        }
    }
}

#[test]
fn row_90_load_dict_hc_level_dependent_fill() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(90);
        // >= 32 KB + LZ4MID_HASHSIZE exercises LZ4MID_fillHTable's second pass
        for &ds in &[
            4usize, 8, 9, 4096, 32 * 1024, 32 * 1024 + 8, 32 * 1024 + 16, 50_000,
            64 * 1024,
        ] {
            let dict = gen(&mut rng, Shape::TextLike, ds);
            for &level in &LEVELS_WIDE {
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let tag = format!("loadDictHC fill level={level} dictSize={ds}");
                assert_eq!(load_dict(l, &tag, &s, &dict), ds as c_int);
                // block replaying several dictionary windows
                let mut blk = Vec::new();
                let step = (ds / 4).max(1);
                let mut off = 0usize;
                while off + step <= ds {
                    blk.extend_from_slice(&dict[off..off + step]);
                    blk.extend_from_slice(&gen_incompressible(&mut rng, 40));
                    off += step;
                }
                blk.extend_from_slice(&dict[ds.saturating_sub(300)..]);
                let cap = cbound(l, blk.len() as c_int) as usize;
                let (ret, dc, dr) = cont_pair(l, &tag, &s, blk.as_ptr(), blk.len(), cap);
                assert!(ret > 0, "{tag}: expected success");
                dec_cross_dict(l, &tag, &dc, &dr, ret, &blk, &dict);
                free_streams(l, s);
            }
        }
    }
}

// ===========================================================================
// Row 91-96 : LZ4_compress_HC_continue
// ===========================================================================

#[test]
fn row_91_continue_contiguous_blocks() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(91);
        for &level in &LEVELS_WIDE {
            for shape in ALL_SHAPES {
                let whole = gen(&mut rng, shape, 600_000);
                for &blk in &[1usize, 4, 13, 64, 1000, 5000, 40_000, 65_536, 70_000] {
                    let s = create_streams(l);
                    stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                    let mut off = 0usize;
                    let mut i = 0;
                    while off < whole.len() && i < 16 {
                        let n = blk.min(whole.len() - off);
                        let cap = cbound(l, n as c_int) as usize;
                        let tag = format!(
                            "contiguous level={level} shape={shape:?} blk={blk} #{i}"
                        );
                        let (ret, dc, dr) =
                            cont_pair(l, &tag, &s, whole[off..].as_ptr(), n, cap);
                        assert!(ret > 0, "{tag}: expected success");
                        dec_cross_dict(
                            l,
                            &tag,
                            &dc,
                            &dr,
                            ret,
                            &whole[off..off + n],
                            &whole[..off],
                        );
                        off += n;
                        i += 1;
                    }
                    free_streams(l, s);
                }
            }
        }
    }
}

#[test]
fn row_92_continue_non_contiguous_src() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(92);
        for &level in &LEVELS_WIDE {
            for &n in &[13usize, 500, 3000, 40_000, 70_000, 200_000] {
                // three completely separate buffers, sharing content so the
                // ext-dict search actually finds matches
                let base = gen(&mut rng, Shape::TextLike, n);
                let mut bufs: Vec<Vec<u8>> = Vec::new();
                for k in 0..6 {
                    let mut v = base.clone();
                    for j in 0..(n / 8) {
                        v[j * 8 % n] = (j as u8).wrapping_add(k as u8);
                    }
                    bufs.push(v);
                }
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let cap = cbound(l, n as c_int) as usize;
                let mut prev: Option<&Vec<u8>> = None;
                for (i, buf) in bufs.iter().enumerate() {
                    let tag = format!("non-contiguous level={level} n={n} #{i}");
                    let (ret, dc, dr) = cont_pair(l, &tag, &s, buf.as_ptr(), n, cap);
                    assert!(ret > 0, "{tag}: expected success");
                    let empty: Vec<u8> = Vec::new();
                    let dict: &[u8] = prev.map(|v| v.as_slice()).unwrap_or(&empty);
                    dec_cross_dict(l, &tag, &dc, &dr, ret, buf, dict);
                    prev = Some(buf);
                }
                free_streams(l, s);
            }
        }
    }
}

#[test]
fn row_93_continue_ring_buffer_overlapping_dict() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(93);
        for &level in &LEVELS_WIDE {
            // ring buffers both smaller and larger than 64 KB, plus a tiny one
            // whose remaining dictionary drops below LZ4HC_HASHSIZE
            for &ring_size in &[64usize, 600, 8192, 40_000, 65_536, 70_000, 200_000] {
                for &blk in &[3usize, 13, 200, 5000, 33_000] {
                    if blk * 2 > ring_size {
                        continue;
                    }
                    let mut ring = vec![0u8; ring_size];
                    let s = create_streams(l);
                    stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                    let cap = cbound(l, blk as c_int) as usize;
                    let mut pos = 0usize;
                    for i in 0..24 {
                        if pos + blk > ring_size {
                            pos = 0;
                        }
                        // refresh this slot with (partly repeated) data
                        let fresh = if i % 3 == 0 {
                            gen(&mut rng, Shape::TextLike, blk)
                        } else {
                            gen(&mut rng, Shape::Compressible, blk)
                        };
                        ring[pos..pos + blk].copy_from_slice(&fresh);
                        let tag = format!(
                            "ring level={level} ring={ring_size} blk={blk} #{i} pos={pos}"
                        );
                        let (ret, dc, dr) =
                            cont_pair(l, &tag, &s, ring[pos..].as_ptr(), blk, cap);
                        assert!(ret > 0, "{tag}: expected success");
                        // the state now points into the ring; both libraries must
                        // agree byte-for-byte (round-trip needs the decoder's own
                        // ring which rows 58-60 cover)
                        same_full_buffers(&tag, &dc, &dr);
                        pos += blk;
                    }
                    free_streams(l, s);
                }
            }
        }
    }
}

#[test]
fn row_94_continue_matching_into_ext_dict() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(94);
        for &level in &LEVELS_WIDE {
            for &ds in &[4usize, 700, 4096, 32 * 1024, 65_535, 65_536, 100 * 1024] {
                let dict = gen(&mut rng, Shape::TextLike, ds);
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let tag = format!("extDict level={level} dictSize={ds}");
                let kept = load_dict(l, &tag, &s, &dict);
                assert_eq!(kept, ds.min(65_536) as c_int);
                // block built almost entirely out of dictionary fragments
                let mut blk = Vec::new();
                let mut o = 0usize;
                while o + 700 < ds {
                    blk.extend_from_slice(&dict[o..o + 700]);
                    blk.extend_from_slice(&gen_incompressible(&mut rng, 11));
                    o += 1300;
                }
                blk.extend_from_slice(&dict[ds.saturating_sub(500)..]);
                blk.extend_from_slice(&gen(&mut rng, Shape::TextLike, 30_000));
                let cap = cbound(l, blk.len() as c_int) as usize;
                let (ret, dc, dr) = cont_pair(l, &tag, &s, blk.as_ptr(), blk.len(), cap);
                assert!(ret > 0, "{tag}: expected success");
                dec_cross_dict(l, &tag, &dc, &dr, ret, &blk, &dict[ds - kept as usize..]);
                // a second, contiguous block: prefix + extDict (doubleDict)
                let mut blk2 = Vec::new();
                blk2.extend_from_slice(&blk[..blk.len().min(3000)]);
                blk2.extend_from_slice(&dict[..1500.min(ds)]);
                let cap2 = cbound(l, blk2.len() as c_int) as usize;
                let tag2 = format!("{tag} second block");
                let (r2, d2c, d2r) =
                    cont_pair(l, &tag2, &s, blk2.as_ptr(), blk2.len(), cap2);
                assert!(r2 > 0, "{tag2}: expected success");
                free_streams(l, s);
                let _ = (d2c, d2r);
            }
        }
    }
}

#[test]
fn row_95_continue_dst_capacity_and_dirty_flag() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(95);
        for &level in &LEVELS_WIDE {
            for shape in ALL_SHAPES {
                let whole = gen(&mut rng, shape, 480_000);
                for &blk in &[13usize, 1000, 20_000, 70_000] {
                let bound = cbound(l, blk as c_int) as usize;
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                // block 0: >= bound (notLimited)
                let tag = format!("cont cap=bound level={level} shape={shape:?}");
                let (r0, d0c, d0r) = cont_pair(l, &tag, &s, whole.as_ptr(), blk, bound);
                assert!(r0 > 0, "{tag}: expected success");
                dec_cross_dict(l, &tag, &d0c, &d0r, r0, &whole[..blk], &[]);
                // block 1: bound - 1 (limitedOutput but still enough)
                let tag = format!("cont cap=bound-1 level={level} shape={shape:?}");
                let (r1, d1c, d1r) =
                    cont_pair(l, &tag, &s, whole[blk..].as_ptr(), blk, bound - 1);
                if r1 > 0 {
                    dec_cross_dict(
                        l,
                        &tag,
                        &d1c,
                        &d1r,
                        r1,
                        &whole[blk..2 * blk],
                        &whole[..blk],
                    );
                }
                // an intermediate capacity: may or may not fit, but both
                // libraries must agree
                let tag = format!("cont cap=16 level={level} shape={shape:?}");
                cont_pair(l, &tag, &s, whole[blk..].as_ptr(), blk, 16);
                // block 2: far too small -> 0, stream becomes dirty
                let tag = format!("cont cap=1 level={level} shape={shape:?}");
                let (r2, _, _) =
                    cont_pair(l, &tag, &s, whole[2 * blk..].as_ptr(), blk, 1);
                assert_eq!(r2, 0, "{tag}: expected failure");
                // keep driving the (now dirty) stream: both must behave alike
                for i in 3..6 {
                    let tag = format!("cont dirty #{i} level={level} shape={shape:?}");
                    let (_ri, dic, dir) = cont_pair(
                        l,
                        &tag,
                        &s,
                        whole[i * blk..].as_ptr(),
                        blk,
                        bound,
                    );
                    same_full_buffers(&tag, &dic, &dir);
                }
                // explicit reset recovers
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let tag = format!("cont after reset level={level} shape={shape:?}");
                let (r6, d6c, d6r) = cont_pair(l, &tag, &s, whole.as_ptr(), blk, bound);
                assert!(r6 > 0, "{tag}: expected success");
                dec_cross_dict(l, &tag, &d6c, &d6r, r6, &whole[..blk], &[]);
                free_streams(l, s);
                }
            }
        }
    }
}

#[test]
fn row_96_continue_two_gb_overflow_reanchor() {
    let l = libs();
    unsafe {
        // Each failing call advances ctx->end by srcSize and the following
        // (non-contiguous) call folds that into dictLimit, so dictLimit grows by
        // 64 MB per round and crosses the 2 GB automatic-re-anchor threshold
        // after 32 rounds -- without ever compressing 2 GB of data.
        let mut rng = Rng::new(96);
        let n = 64 * 1024 * 1024usize;
        let src = gen_incompressible(&mut rng, n);
        let s = create_streams(l);
        stream_call(l, "LZ4_resetStreamHC_fast", &s, LZ4HC_CLEVEL_MIN);
        for i in 0..70 {
            let tag = format!("2GB overflow round #{i}");
            let (ret, dc, dr) = cont_pair(l, &tag, &s, src.as_ptr(), n, 0);
            assert_eq!(ret, 0, "{tag}: expected failure with dstCapacity 0");
            same_full_buffers(&tag, &dc, &dr);
        }
        // after the re-anchor a normal block must still compress identically
        let small = gen(&mut rng, Shape::TextLike, 20_000);
        let cap = cbound(l, 20_000) as usize;
        let tag = "2GB overflow: block after re-anchor";
        let (ret, dc, dr) = cont_pair(l, tag, &s, small.as_ptr(), 20_000, cap);
        assert!(ret > 0, "{tag}: expected success");
        same_full_buffers(tag, &dc, &dr);
        // the resulting (re-anchored) state must also be identical as observed
        // through LZ4_saveDictHC
        let (svc, svr) = l.sym::<FnSaveDictHC>("LZ4_saveDictHC");
        let mut sbc = vec![SENT; 64 * 1024 + TAIL];
        let mut sbr = vec![SENT; 64 * 1024 + TAIL];
        let a = svc(s.c, sbc.as_mut_ptr() as *mut c_char, 64 * 1024);
        let b = svr(s.r, sbr.as_mut_ptr() as *mut c_char, 64 * 1024);
        assert_eq!(a, b, "2GB overflow: saveDictHC return mismatch");
        same_full_buffers("2GB overflow: saveDictHC content", &sbc, &sbr);
        free_streams(l, s);
    }
}

// ===========================================================================
// Row 97-100 : attach dictionary, continue_destSize, saveDictHC
// ===========================================================================

#[test]
fn row_97_attach_hc_dictionary_paths() {
    let l = libs();
    unsafe {
        let (ac, ar) = l.sym::<FnAttachHC>("LZ4_attach_HC_dictionary");
        let mut rng = Rng::new(97);
        let dict = gen(&mut rng, Shape::TextLike, 60_000);
        // (dict level, working level, first block size)
        let cases: [(c_int, c_int, usize); 18] = [
            (9, 9, 5000),      // position 0, > 4 KB, compatible -> memcpy path
            (9, 9, 70_000),    // same, large block
            (12, 12, 8000),    // compatible (both optimal)
            (9, 12, 9000),     // compatible (both non-mid)
            (2, 2, 6000),      // compatible (both mid)
            (2, 9, 6000),      // INcompatible -> usingDictCtxHc
            (9, 2, 6000),      // INcompatible -> usingDictCtxHc
            (2, 12, 7000),     // INcompatible
            (9, 9, 3000),      // position 0 but <= 4 KB -> usingDictCtxHc
            (2, 2, 1000),      // small block, mid dictCtx
            (1, 1, 5000),      // mid/mid, > 4 KB
            (1, 9, 5000),      // INcompatible
            (12, 2, 5000),     // INcompatible
            (12, 1, 4096),     // INcompatible, exactly 4 KB -> usingDictCtxHc
            (9, 9, 4096),      // exactly 4 KB (not > 4 KB) -> usingDictCtxHc
            (9, 9, 4097),      // just over 4 KB -> memcpy path
            (10, 11, 20_000),  // both optimal, large block
            (2, 2, 120_000),   // mid/mid, block far beyond 64 KB
        ];
        for (i, &(dl, wl, n)) in cases.iter().enumerate() {
            let ds = create_streams(l);
            stream_call(l, "LZ4_setCompressionLevel", &ds, dl);
            let tag = format!("attach #{i} dictLvl={dl} workLvl={wl} n={n}");
            assert_eq!(load_dict(l, &tag, &ds, &dict), 60_000);

            let ws = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &ws, wl);
            ac(ws.c, ds.c as *const c_void);
            ar(ws.r, ds.r as *const c_void);

            // block built from dictionary fragments so the dictCtx is used
            let mut blk = Vec::new();
            let mut o = 1000usize;
            while blk.len() < n {
                let take = (n - blk.len()).min(400);
                blk.extend_from_slice(&dict[o..o + take]);
                blk.extend_from_slice(&gen_incompressible(&mut rng, 7));
                o = (o + 977) % 50_000;
            }
            blk.truncate(n);
            let cap = cbound(l, n as c_int) as usize;
            let (ret, dc, dr) = cont_pair(l, &tag, &ws, blk.as_ptr(), n, cap);
            assert!(ret > 0, "{tag}: expected success");
            dec_cross_dict(l, &tag, &dc, &dr, ret, &blk, &dict);

            // a follow-up contiguous block (dictCtx dropped or extDict in use)
            let blk2 = gen(&mut rng, Shape::TextLike, 90_000);
            let cap2 = cbound(l, 90_000) as usize;
            let tag2 = format!("{tag} follow-up");
            let (r2, d2c, d2r) = cont_pair(l, &tag2, &ws, blk2.as_ptr(), 90_000, cap2);
            assert!(r2 > 0, "{tag2}: expected success");
            same_full_buffers(&tag2, &d2c, &d2r);

            free_streams(l, ws);
            free_streams(l, ds);
        }
    }
}

#[test]
fn row_98_attach_hc_dictionary_dropped_and_null() {
    let l = libs();
    unsafe {
        let (ac, ar) = l.sym::<FnAttachHC>("LZ4_attach_HC_dictionary");
        let mut rng = Rng::new(98);
        let dict = gen(&mut rng, Shape::TextLike, 60_000);
        let whole = gen(&mut rng, Shape::TextLike, 500_000);

        // position >= 64 KB -> the dictCtx is dropped.  Use incompatible levels
        // for the first call so the memcpy shortcut does not clear dictCtx early.
        for &(dl, wl) in &[(2, 9), (9, 2), (2, 12), (9, 9)] {
            let ds = create_streams(l);
            stream_call(l, "LZ4_setCompressionLevel", &ds, dl);
            let tag = format!("attach drop dictLvl={dl} workLvl={wl}");
            assert_eq!(load_dict(l, &tag, &ds, &dict), 60_000);
            let ws = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &ws, wl);
            ac(ws.c, ds.c as *const c_void);
            ar(ws.r, ds.r as *const c_void);
            let mut off = 0usize;
            for i in 0..6 {
                let n = 60_000usize;
                let cap = cbound(l, n as c_int) as usize;
                let t = format!("{tag} block#{i}");
                let (ret, dc, dr) = cont_pair(l, &t, &ws, whole[off..].as_ptr(), n, cap);
                assert!(ret > 0, "{t}: expected success");
                same_full_buffers(&t, &dc, &dr);
                off += n;
            }
            free_streams(l, ws);
            free_streams(l, ds);
        }

        // dictionary_stream == NULL unsets any attached dictionary
        for &wl in &[2, 9, 12] {
            let ds = create_streams(l);
            stream_call(l, "LZ4_setCompressionLevel", &ds, 9);
            load_dict(l, "attach then unset", &ds, &dict);
            let ws = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &ws, wl);
            ac(ws.c, ds.c as *const c_void);
            ar(ws.r, ds.r as *const c_void);
            ac(ws.c, ptr::null());
            ar(ws.r, ptr::null());
            let blk = &dict[10_000..25_000];
            let cap = cbound(l, blk.len() as c_int) as usize;
            let tag = format!("attach NULL workLvl={wl}");
            let (ret, dc, dr) = cont_pair(l, &tag, &ws, blk.as_ptr(), blk.len(), cap);
            assert!(ret > 0, "{tag}: expected success");
            dec_cross(l, &tag, &dc, &dr, ret, blk);
            // with the dictionary unset the result must equal a plain stream
            let ps = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &ps, wl);
            let (r2, d2c, d2r) =
                cont_pair(l, &format!("{tag} plain"), &ps, blk.as_ptr(), blk.len(), cap);
            assert_eq!(ret, r2, "{tag}: unset dictionary still influenced output");
            same_full_buffers(&format!("{tag} C plain"), &dc, &d2c);
            same_full_buffers(&format!("{tag} Rust plain"), &dr, &d2r);
            free_streams(l, ps);
            free_streams(l, ws);
            free_streams(l, ds);
        }
    }
}

#[test]
fn row_99_continue_dest_size() {
    let l = libs();
    unsafe {
        let mut rng = Rng::new(99);
        for &level in &LEVELS_WIDE {
            for shape in ALL_SHAPES {
                let whole = gen(&mut rng, shape, 200_000);
                for &target in &[0usize, 1, 2, 5, 17, 64, 512, 2048, 6000, 40_000, 80_000]
                {
                    let s = create_streams(l);
                    stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                    let mut off = 0usize;
                    for i in 0..4 {
                        let n = 30_000usize.min(whole.len() - off);
                        let tag = format!(
                            "cont_destSize level={level} shape={shape:?} target={target} #{i}"
                        );
                        let (ret, consumed, dc, dr) =
                            cont_destsize_pair(l, &tag, &s, whole[off..].as_ptr(), n, target);
                        if ret > 0 {
                            assert!(
                                consumed >= 0 && consumed <= n as c_int,
                                "{tag}: bogus *srcSizePtr {consumed}"
                            );
                            dec_cross_dict(
                                l,
                                &tag,
                                &dc,
                                &dr,
                                ret,
                                &whole[off..off + consumed as usize],
                                &whole[..off],
                            );
                            off += consumed as usize;
                        } else {
                            break;
                        }
                        if off >= whole.len() {
                            break;
                        }
                    }
                    free_streams(l, s);
                }
            }
        }
    }
}

#[test]
fn row_100_save_dict_hc() {
    let l = libs();
    unsafe {
        let (fc, fr) = l.sym::<FnSaveDictHC>("LZ4_saveDictHC");
        let mut rng = Rng::new(100);
        for &level in &LEVELS_WIDE {
            for &prefix in &[3usize, 4, 5, 100, 4096, 30_000, 65_536, 70_000] {
                for &want in &[0usize, 1, 3, 4, 5, 100, 32 * 1024, 64 * 1024, 100 * 1024] {
                    let src = gen(&mut rng, Shape::TextLike, prefix);
                    let s = create_streams(l);
                    stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                    let cap = cbound(l, prefix as c_int) as usize;
                    let tag =
                        format!("saveDictHC level={level} prefix={prefix} want={want}");
                    let (r0, _, _) = cont_pair(l, &tag, &s, src.as_ptr(), prefix, cap);
                    assert!(r0 > 0, "{tag}: setup block failed");

                    // separate save buffers -- each library writes its own
                    let mut sbc = vec![SENT; want + TAIL];
                    let mut sbr = vec![SENT; want + TAIL];
                    let a = fc(s.c, sbc.as_mut_ptr() as *mut c_char, want as c_int);
                    let b = fr(s.r, sbr.as_mut_ptr() as *mut c_char, want as c_int);
                    assert_eq!(a, b, "{tag}: return mismatch (C={a} Rust={b})");
                    let expect = {
                        let mut d = want.min(64 * 1024);
                        if d < 4 {
                            d = 0;
                        }
                        d.min(prefix) as c_int
                    };
                    assert_eq!(a, expect, "{tag}: unexpected saved dictSize");
                    same_full_buffers(&tag, &sbc, &sbr);
                    if a > 0 {
                        assert_eq!(
                            &sbc[..a as usize],
                            &src[prefix - a as usize..],
                            "{tag}: saved dictionary content wrong"
                        );
                    }

                    // compression continues against the saved dictionary
                    let blk = if prefix >= 200 {
                        let mut v = src[prefix - 200..].to_vec();
                        v.extend_from_slice(&gen(&mut rng, Shape::TextLike, 3000));
                        v
                    } else {
                        gen(&mut rng, Shape::TextLike, 3000)
                    };
                    let cap2 = cbound(l, blk.len() as c_int) as usize;
                    let tag2 = format!("{tag} continue");
                    let (r2, d2c, d2r) =
                        cont_pair(l, &tag2, &s, blk.as_ptr(), blk.len(), cap2);
                    assert!(r2 > 0, "{tag2}: expected success");
                    dec_cross_dict(
                        l,
                        &tag2,
                        &d2c,
                        &d2r,
                        r2,
                        &blk,
                        &sbc[..a as usize],
                    );
                    free_streams(l, s);
                }
            }
        }

        // safeBuffer == NULL: only legal when the resulting dictSize is 0
        for &level in &[2, 9, 12] {
            for &want in &[0usize, 1, 2, 3] {
                let src = gen(&mut rng, Shape::TextLike, 10_000);
                let s = create_streams(l);
                stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
                let cap = cbound(l, 10_000) as usize;
                cont_pair(l, "saveDictHC NULL setup", &s, src.as_ptr(), 10_000, cap);
                let a = fc(s.c, ptr::null_mut(), want as c_int);
                let b = fr(s.r, ptr::null_mut(), want as c_int);
                assert_eq!(a, 0, "C saveDictHC(NULL, {want}) returned {a}");
                assert_eq!(b, 0, "Rust saveDictHC(NULL, {want}) returned {b}");
                free_streams(l, s);
            }
        }
    }
}

// ===========================================================================
// Row 101-103 : deprecated entry points
// ===========================================================================

#[test]
fn row_101_deprecated_one_shots() {
    let l = libs();
    unsafe {
        let (c1, r1) = l.sym::<FnDep3>("LZ4_compressHC");
        let (c2, r2) = l.sym::<FnDep4>("LZ4_compressHC_limitedOutput");
        let (c3, r3) = l.sym::<FnDep4>("LZ4_compressHC2");
        let (c4, r4) = l.sym::<FnHC>("LZ4_compressHC2_limitedOutput");
        let mut rng = Rng::new(101);
        for &sz in &[0usize, 1, 12, 13, 700, 4097, 20_000, 65_547, 70_000, 200_000] {
            for shape in ALL_SHAPES {
                let src = gen(&mut rng, shape, sz);
                let bound = cbound(l, sz as c_int) as usize;

                // LZ4_compressHC : bound-sized, level 0 -> default (9)
                let mut dc = vec![SENT; bound + TAIL];
                let mut dr = vec![SENT; bound + TAIL];
                let tag = format!("LZ4_compressHC size={sz} shape={shape:?}");
                let a = c1(
                    src.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                );
                let b = r1(
                    src.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(a > 0, "{tag}: expected success");
                dec_cross(l, &tag, &dc, &dr, a, &src);
                let (r9, d9c, d9r) =
                    hc_pair(l, &tag, src.as_ptr(), sz as c_int, bound, 9);
                assert_eq!(a, r9, "{tag}: != LZ4_compress_HC level 9");
                same_full_buffers(&format!("{tag} C vs HC9"), &dc, &d9c);
                same_full_buffers(&format!("{tag} Rust vs HC9"), &dr, &d9r);

                // LZ4_compressHC_limitedOutput
                for &cap in &[bound, bound.saturating_sub(1), a as usize, 4, 1, 0] {
                    let mut dc = vec![SENT; cap + TAIL];
                    let mut dr = vec![SENT; cap + TAIL];
                    let tag = format!(
                        "LZ4_compressHC_limitedOutput size={sz} shape={shape:?} cap={cap}"
                    );
                    let a = c2(
                        src.as_ptr() as *const c_char,
                        dc.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                    );
                    let b = r2(
                        src.as_ptr() as *const c_char,
                        dr.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                    );
                    same_int_and_bytes(&tag, a, b, &dc, &dr);
                    same_full_buffers(&tag, &dc, &dr);
                    if a > 0 {
                        dec_cross(l, &tag, &dc, &dr, a, &src);
                    }
                }

                // LZ4_compressHC2 / LZ4_compressHC2_limitedOutput
                for &level in &[0, 1, 9, 12, -5, 13] {
                    let mut dc = vec![SENT; bound + TAIL];
                    let mut dr = vec![SENT; bound + TAIL];
                    let tag = format!(
                        "LZ4_compressHC2 size={sz} shape={shape:?} level={level}"
                    );
                    let a = c3(
                        src.as_ptr() as *const c_char,
                        dc.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        level,
                    );
                    let b = r3(
                        src.as_ptr() as *const c_char,
                        dr.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        level,
                    );
                    same_int_and_bytes(&tag, a, b, &dc, &dr);
                    same_full_buffers(&tag, &dc, &dr);
                    assert!(a > 0, "{tag}: expected success");
                    dec_cross(l, &tag, &dc, &dr, a, &src);

                    for &cap in &[bound, a as usize, (a as usize) / 2, 3, 0] {
                        let mut dc = vec![SENT; cap + TAIL];
                        let mut dr = vec![SENT; cap + TAIL];
                        let tag = format!(
                            "LZ4_compressHC2_limitedOutput size={sz} shape={shape:?} \
                             level={level} cap={cap}"
                        );
                        let a2 = c4(
                            src.as_ptr() as *const c_char,
                            dc.as_mut_ptr() as *mut c_char,
                            sz as c_int,
                            cap as c_int,
                            level,
                        );
                        let b2 = r4(
                            src.as_ptr() as *const c_char,
                            dr.as_mut_ptr() as *mut c_char,
                            sz as c_int,
                            cap as c_int,
                            level,
                        );
                        same_int_and_bytes(&tag, a2, b2, &dc, &dr);
                        same_full_buffers(&tag, &dc, &dr);
                        if a2 > 0 {
                            dec_cross(l, &tag, &dc, &dr, a2, &src);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn row_102_deprecated_with_state_hc() {
    let l = libs();
    unsafe {
        let ssz = state_size(l);
        let (c1, r1) = l.sym::<FnDepSt4>("LZ4_compressHC_withStateHC");
        let (c2, r2) = l.sym::<FnDepSt5>("LZ4_compressHC_limitedOutput_withStateHC");
        let (c3, r3) = l.sym::<FnDepSt5>("LZ4_compressHC2_withStateHC");
        let (c4, r4) = l.sym::<FnDepSt6>("LZ4_compressHC2_limitedOutput_withStateHC");
        let mut sc = Aligned::new(ssz);
        let mut sr = Aligned::new(ssz);
        let mut rng = Rng::new(102);
        for &sz in &[0usize, 1, 12, 13, 900, 4097, 20_000, 65_547, 70_000] {
            for shape in ALL_SHAPES {
                let src = gen(&mut rng, shape, sz);
                let bound = cbound(l, sz as c_int) as usize;

                let mut dc = vec![SENT; bound + TAIL];
                let mut dr = vec![SENT; bound + TAIL];
                let tag = format!("compressHC_withStateHC size={sz} shape={shape:?}");
                let a = c1(
                    sc.ptr(),
                    src.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                );
                let b = r1(
                    sr.ptr(),
                    src.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    sz as c_int,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(a > 0, "{tag}: expected success");
                dec_cross(l, &tag, &dc, &dr, a, &src);

                for &cap in &[bound, a as usize, (a as usize) / 3, 2, 0] {
                    let mut dc = vec![SENT; cap + TAIL];
                    let mut dr = vec![SENT; cap + TAIL];
                    let tag = format!(
                        "compressHC_limitedOutput_withStateHC size={sz} cap={cap} \
                         shape={shape:?}"
                    );
                    let a2 = c2(
                        sc.ptr(),
                        src.as_ptr() as *const c_char,
                        dc.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                    );
                    let b2 = r2(
                        sr.ptr(),
                        src.as_ptr() as *const c_char,
                        dr.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        cap as c_int,
                    );
                    same_int_and_bytes(&tag, a2, b2, &dc, &dr);
                    same_full_buffers(&tag, &dc, &dr);
                    if a2 > 0 {
                        dec_cross(l, &tag, &dc, &dr, a2, &src);
                    }
                }

                for &level in &[0, 1, 9, 12, 13] {
                    let mut dc = vec![SENT; bound + TAIL];
                    let mut dr = vec![SENT; bound + TAIL];
                    let tag = format!(
                        "compressHC2_withStateHC size={sz} level={level} shape={shape:?}"
                    );
                    let a3 = c3(
                        sc.ptr(),
                        src.as_ptr() as *const c_char,
                        dc.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        level,
                    );
                    let b3 = r3(
                        sr.ptr(),
                        src.as_ptr() as *const c_char,
                        dr.as_mut_ptr() as *mut c_char,
                        sz as c_int,
                        level,
                    );
                    same_int_and_bytes(&tag, a3, b3, &dc, &dr);
                    same_full_buffers(&tag, &dc, &dr);
                    assert!(a3 > 0, "{tag}: expected success");
                    dec_cross(l, &tag, &dc, &dr, a3, &src);

                    for &cap in &[bound, a3 as usize, (a3 as usize) / 2, 1, 0] {
                        let mut dc = vec![SENT; cap + TAIL];
                        let mut dr = vec![SENT; cap + TAIL];
                        let tag = format!(
                            "compressHC2_limitedOutput_withStateHC size={sz} \
                             level={level} cap={cap} shape={shape:?}"
                        );
                        let a4 = c4(
                            sc.ptr(),
                            src.as_ptr() as *const c_char,
                            dc.as_mut_ptr() as *mut c_char,
                            sz as c_int,
                            cap as c_int,
                            level,
                        );
                        let b4 = r4(
                            sr.ptr(),
                            src.as_ptr() as *const c_char,
                            dr.as_mut_ptr() as *mut c_char,
                            sz as c_int,
                            cap as c_int,
                            level,
                        );
                        same_int_and_bytes(&tag, a4, b4, &dc, &dr);
                        same_full_buffers(&tag, &dc, &dr);
                        if a4 > 0 {
                            dec_cross(l, &tag, &dc, &dr, a4, &src);
                        }
                    }
                }
            }
        }

        // misaligned external state -> 0 from both
        let src = gen(&mut rng, Shape::TextLike, 5000);
        let bound = cbound(l, 5000) as usize;
        let mut bc = Aligned::new(ssz + 16);
        let mut br = Aligned::new(ssz + 16);
        let mut dc = vec![SENT; bound + TAIL];
        let mut dr = vec![SENT; bound + TAIL];
        let a = c1(
            bc.at(1),
            src.as_ptr() as *const c_char,
            dc.as_mut_ptr() as *mut c_char,
            5000,
        );
        let b = r1(
            br.at(1),
            src.as_ptr() as *const c_char,
            dr.as_mut_ptr() as *mut c_char,
            5000,
        );
        assert_eq!(a, 0, "C compressHC_withStateHC(misaligned) = {a}");
        assert_eq!(b, 0, "Rust compressHC_withStateHC(misaligned) = {b}");
    }
}

#[test]
fn row_103_deprecated_streaming_and_state_api() {
    let l = libs();
    unsafe {
        // ---- LZ4_sizeofStreamStateHC / LZ4_resetStreamStateHC --------------
        let (szc, szr) = l.sym::<FnVoidToInt>("LZ4_sizeofStreamStateHC");
        let (a, b) = (szc(), szr());
        assert_eq!(a, b, "LZ4_sizeofStreamStateHC mismatch (C={a} Rust={b})");
        let sss = a as usize;
        assert_eq!(sss, state_size(l), "sizeofStreamStateHC != sizeofStateHC");

        let mut rng = Rng::new(103);
        let mut buf = gen(&mut rng, Shape::TextLike, 400_000);

        let (rsc, rsr) = l.sym::<FnResetStreamStateHC>("LZ4_resetStreamStateHC");
        let mut stc = Aligned::new(sss + 16);
        let mut str_ = Aligned::new(sss + 16);
        // success -> 0
        let a = rsc(stc.ptr(), buf.as_mut_ptr() as *mut c_char);
        let b = rsr(str_.ptr(), buf.as_mut_ptr() as *mut c_char);
        assert_eq!(a, 0, "C LZ4_resetStreamStateHC should return 0 on success");
        assert_eq!(b, 0, "Rust LZ4_resetStreamStateHC should return 0 on success");
        // failure (misaligned / NULL) -> 1
        for &off in &[1usize, 3, 5, 7] {
            let a = rsc(stc.at(off), buf.as_mut_ptr() as *mut c_char);
            let b = rsr(str_.at(off), buf.as_mut_ptr() as *mut c_char);
            assert_eq!(a, 1, "C LZ4_resetStreamStateHC(misaligned+{off}) = {a}");
            assert_eq!(b, 1, "Rust LZ4_resetStreamStateHC(misaligned+{off}) = {b}");
        }
        let a = rsc(ptr::null_mut(), buf.as_mut_ptr() as *mut c_char);
        let b = rsr(ptr::null_mut(), buf.as_mut_ptr() as *mut c_char);
        assert_eq!(a, 1, "C LZ4_resetStreamStateHC(NULL) = {a}");
        assert_eq!(b, 1, "Rust LZ4_resetStreamStateHC(NULL) = {b}");

        // ---- LZ4_compressHC_continue / _limitedOutput_continue -------------
        let (cc, cr) = l.sym::<FnDepSt4>("LZ4_compressHC_continue");
        let (lc, lr) = l.sym::<FnDepSt5>("LZ4_compressHC_limitedOutput_continue");
        for &level in &LEVELS_WIDE {
            let s = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            let blk = 20_000usize;
            let bound = cbound(l, blk as c_int) as usize;
            for i in 0..12 {
                let off = i * blk;
                let mut dc = vec![SENT; bound + TAIL];
                let mut dr = vec![SENT; bound + TAIL];
                let tag = format!("compressHC_continue level={level} #{i}");
                let a = cc(
                    s.c,
                    buf[off..].as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                );
                let b = cr(
                    s.r,
                    buf[off..].as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(a > 0, "{tag}: expected success");
                dec_cross_dict(
                    l,
                    &tag,
                    &dc,
                    &dr,
                    a,
                    &buf[off..off + blk],
                    &buf[..off],
                );
            }
            free_streams(l, s);

            // limited-output variant, including a failing capacity
            let s = create_streams(l);
            stream_call(l, "LZ4_resetStreamHC_fast", &s, level);
            for (i, &cap) in [bound, bound - 1, 32, bound].iter().enumerate() {
                let off = i * blk;
                let mut dc = vec![SENT; cap + TAIL];
                let mut dr = vec![SENT; cap + TAIL];
                let tag = format!(
                    "compressHC_limitedOutput_continue level={level} #{i} cap={cap}"
                );
                let a = lc(
                    s.c,
                    buf[off..].as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    cap as c_int,
                );
                let b = lr(
                    s.r,
                    buf[off..].as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    cap as c_int,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
            }
            free_streams(l, s);
        }

        // ---- LZ4_createHC / LZ4_freeHC / LZ4_slideInputBufferHC ------------
        let (crc, crr) = l.sym::<FnCreateHC>("LZ4_createHC");
        let (fhc, fhr) = l.sym::<FnFreePtr>("LZ4_freeHC");
        assert_eq!(fhc(ptr::null_mut()), 0, "C LZ4_freeHC(NULL)");
        assert_eq!(fhr(ptr::null_mut()), 0, "Rust LZ4_freeHC(NULL)");

        let (slc, slr) = l.sym::<FnSlideHC>("LZ4_slideInputBufferHC");
        let (c2c, c2r) = l.sym::<FnDepSt5>("LZ4_compressHC2_continue");
        let (c2lc, c2lr) = l.sym::<FnDepSt6>("LZ4_compressHC2_limitedOutput_continue");

        // LZ4_compressHC2_continue is called with dstCapacity == 0 and the
        // notLimited directive, so the destination must be bound-sized.  Levels
        // 1/2 are avoided here: with dstCapacity == 0 the C reference (built
        // with assertions enabled) trips assert(op <= oend) inside
        // LZ4MID_compress.
        for &level in &[0, 3, 9, 12] {
            let hc = crc(buf.as_ptr() as *const c_char);
            let hr = crr(buf.as_ptr() as *const c_char);
            assert!(!hc.is_null(), "C LZ4_createHC returned NULL");
            assert!(!hr.is_null(), "Rust LZ4_createHC returned NULL");

            let blk = 20_000usize;
            let bound = cbound(l, blk as c_int) as usize;
            // These deprecated entry points reach LZ4HC_compress_generic
            // directly: there is no auto-init and no contiguity handling, so the
            // blocks must be strictly sequential inside the registered buffer.
            let mut off = 0usize;
            for i in 0..5 {
                let mut dc = vec![SENT; bound + TAIL];
                let mut dr = vec![SENT; bound + TAIL];
                let tag = format!("compressHC2_continue level={level} #{i}");
                let a = c2c(
                    hc,
                    buf[off..].as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    level,
                );
                let b = c2r(
                    hr,
                    buf[off..].as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    level,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(a > 0, "{tag}: expected success");
                dec_cross_dict(
                    l,
                    &tag,
                    &dc,
                    &dr,
                    a,
                    &buf[off..off + blk],
                    &buf[..off],
                );
                off += blk;
            }

            // limited-output variant, still contiguous (including a capacity
            // that is far too small)
            for (i, &cap) in [bound, bound - 1, 24].iter().enumerate() {
                let mut dc = vec![SENT; cap + TAIL];
                let mut dr = vec![SENT; cap + TAIL];
                let tag = format!(
                    "compressHC2_limitedOutput_continue level={level} #{i} cap={cap}"
                );
                let a = c2lc(
                    hc,
                    buf[off..].as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    cap as c_int,
                    level,
                );
                let b = c2lr(
                    hr,
                    buf[off..].as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    cap as c_int,
                    level,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                off += blk;
            }

            // LZ4_slideInputBufferHC returns the start of the tracked buffer and
            // performs a fast reset (which truncates the history and leaves
            // prefixStart == NULL); both libraries must agree.
            let pc = slc(hc);
            let pr = slr(hr);
            assert_eq!(
                pc as usize, pr as usize,
                "LZ4_slideInputBufferHC pointer mismatch (level={level})"
            );
            assert_eq!(
                pc as usize,
                buf.as_ptr() as usize,
                "LZ4_slideInputBufferHC should return the input buffer start"
            );

            // Re-arm the deprecated context (the *_continue wrappers above do no
            // auto-init) and prove it still works after the slide.
            let a = rsc(hc, pc);
            let b = rsr(hr, pr);
            assert_eq!(a, 0, "C resetStreamStateHC after slide returned {a}");
            assert_eq!(b, 0, "Rust resetStreamStateHC after slide returned {b}");
            let mut dc = vec![SENT; bound + TAIL];
            let mut dr = vec![SENT; bound + TAIL];
            let tag = format!("compressHC2_continue after slide level={level}");
            let a = c2c(
                hc,
                buf.as_ptr() as *const c_char,
                dc.as_mut_ptr() as *mut c_char,
                blk as c_int,
                level,
            );
            let b = c2r(
                hr,
                buf.as_ptr() as *const c_char,
                dr.as_mut_ptr() as *mut c_char,
                blk as c_int,
                level,
            );
            same_int_and_bytes(&tag, a, b, &dc, &dr);
            same_full_buffers(&tag, &dc, &dr);
            assert!(a > 0, "{tag}: expected success");
            dec_cross(l, &tag, &dc, &dr, a, &buf[..blk]);

            assert_eq!(fhc(hc), 0, "C LZ4_freeHC");
            assert_eq!(fhr(hr), 0, "Rust LZ4_freeHC");
        }

        // LZ4_compressHC2_limitedOutput_continue at the lz4mid levels (a real
        // dstCapacity is supplied here, so no assertion hazard).
        for &level in &[1, 2] {
            let hc = crc(buf.as_ptr() as *const c_char);
            let hr = crr(buf.as_ptr() as *const c_char);
            let blk = 20_000usize;
            let bound = cbound(l, blk as c_int) as usize;
            for i in 0..4 {
                let off = i * blk;
                let mut dc = vec![SENT; bound + TAIL];
                let mut dr = vec![SENT; bound + TAIL];
                let tag = format!("compressHC2_limitedOutput_continue mid level={level} #{i}");
                let a = c2lc(
                    hc,
                    buf[off..].as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    bound as c_int,
                    level,
                );
                let b = c2lr(
                    hr,
                    buf[off..].as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    blk as c_int,
                    bound as c_int,
                    level,
                );
                same_int_and_bytes(&tag, a, b, &dc, &dr);
                same_full_buffers(&tag, &dc, &dr);
                assert!(a > 0, "{tag}: expected success");
                dec_cross_dict(
                    l,
                    &tag,
                    &dc,
                    &dr,
                    a,
                    &buf[off..off + blk],
                    &buf[..off],
                );
            }
            assert_eq!(fhc(hc), 0);
            assert_eq!(fhr(hr), 0);
        }
    }
}
