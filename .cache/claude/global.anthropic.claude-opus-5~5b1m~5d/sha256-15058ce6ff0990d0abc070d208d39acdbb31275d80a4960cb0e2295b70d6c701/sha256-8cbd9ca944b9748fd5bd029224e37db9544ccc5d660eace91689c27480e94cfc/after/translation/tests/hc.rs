//! Differential tests for the **`lz4hc.c` high-compression API**
//! (`c_src/include/lz4hc.h`).
//!
//! Every call goes through `dlsym` on both shared libraries (C and Rust) and
//! every return code / output byte / public state byte is compared.
//!
//! # compressionLevel thresholds found in `c_src/src/lz4hc.c`
//!
//! `LZ4HC_getCLevelParams()` (lz4hc.c:108-116):
//! ```text
//! if (cLevel < 1) cLevel = LZ4HC_CLEVEL_DEFAULT (9);
//! cLevel = MIN(LZ4HC_CLEVEL_MAX (12), cLevel);
//! return k_clTable[cLevel];
//! ```
//! `k_clTable` (lz4hc.c:92-106):
//! ```text
//! level 0  : lz4mid,     2 searches, targetLength 16   (unreachable: 0 -> 9)
//! level 1  : lz4mid,     2 searches, targetLength 16
//! level 2  : lz4mid,     2 searches, targetLength 16   (== LZ4HC_CLEVEL_MIN)
//! level 3  : lz4hc,      4 searches   (LZ4HC_compress_hashChain)
//! level 4  : lz4hc,      8 searches
//! level 5  : lz4hc,     16 searches
//! level 6  : lz4hc,     32 searches
//! level 7  : lz4hc,     64 searches
//! level 8  : lz4hc,    128 searches
//! level 9  : lz4hc,    256 searches   (patternAnalysis = nbSearches > 128)
//! level 10 : lz4opt,    96 searches, targetLength 64   (== LZ4HC_CLEVEL_OPT_MIN)
//! level 11 : lz4opt,   512 searches, targetLength 128
//! level 12 : lz4opt, 16384 searches, targetLength 4096 (== LZ4HC_CLEVEL_MAX)
//! ```
//! Consequences exercised below:
//! * every level `<= 0` (including `INT_MIN`) behaves **exactly** like level 9
//!   (`LZ4HC_compress_hashChain`, 256 searches, patternAnalysis on).  Note that
//!   *this* version of lz4hc.c has **no** `LZ4_compress_fast` fall-through for
//!   `compressionLevel <= 0`; the whole "fast mode" path is gone, levels < 1
//!   are simply remapped to LZ4HC_CLEVEL_DEFAULT.
//! * levels 1 and 2 select `LZ4MID_compress` and share identical parameters,
//!   so they must produce byte-identical output.
//! * levels 3..9 select `LZ4HC_compress_hashChain`.
//! * levels 10..12 select `LZ4HC_compress_optimal`; `fullUpdate` ("ultra") is
//!   `cLevel >= LZ4HC_CLEVEL_MAX` and uses the **raw** (unclamped) level, so
//!   12, 13, 20, 100 and `INT_MAX` all behave like 12.
//! * `LZ4_setCompressionLevel()` (lz4hc.c:1611) clamps the *stored* level with
//!   the same rules: `<1 -> 9`, `>12 -> 12`.
//! * `ctx->favorDecSpeed` only ever reaches `LZ4HC_compress_optimal`, so it can
//!   only change output for effective levels >= 10.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// ---------------------------------------------------------------------------
// tables
// ---------------------------------------------------------------------------

/// Full compressionLevel sweep, including out-of-range values that the C
/// *clamps* (never rejects).
const LEVELS: &[c_int] = &[
    c_int::MIN,
    -100,
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
    10,
    11,
    12,
    13,
    20,
    100,
    c_int::MAX,
];

/// At least one level per distinct internal compressor / clamping band.
const BAND_LEVELS: &[c_int] = &[
    c_int::MIN, // clamped up to 9
    0,          // < 1  -> 9  : hashChain, 256 searches, patternAnalysis
    1,          // lz4mid
    2,          // lz4mid (LZ4HC_CLEVEL_MIN)
    3,          // hashChain, 4 searches
    5,          // hashChain, 16 searches
    8,          // hashChain, 128 searches (patternAnalysis still off)
    9,          // hashChain, 256 searches (patternAnalysis on)
    10,         // optimal, 96 searches
    11,         // optimal, 512 searches
    12,         // optimal, 16384 searches, fullUpdate
    13,         // > 12 -> 12
    100,        // > 12 -> 12
];

/// Sizes straddling every length threshold in lz4hc.c / lz4.c:
/// MINMATCH(4), LASTLITERALS(5), MFLIMIT(12), LZ4_minLength(13),
/// LZ4MID_HASHSIZE(8), RUN_MASK(15) literal encoding, the 255 continuation
/// bytes, and the 64 KB dictionary window.
const SMALL_SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 5, 11, 12, 13, 15, 16, 63, 64, 65, 254, 255, 256, 4095, 4096,
];
const BIG_SIZES: &[usize] = &[65535, 65536, 65537, 131072, 200000, 300000];

/// `offsetof(LZ4HC_CCtx_internal, dictCtx)`; verified with the C headers:
/// hashTable 131072 + chainTable 131072 + 3 ptrs + 3 U32 + short + 2 i8 = 262184.
/// The field holds a *pointer to the other library's* dictionary context, so it
/// necessarily differs between the two runs and is masked out where relevant.
const DICTCTX_FIELD_OFF: usize = 262184;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn sizeof_state_hc() -> usize {
    let (a, b) = both(|l| unsafe { l.get::<FnI>("LZ4_sizeofStateHC")() });
    assert_eq!(a, b, "LZ4_sizeofStateHC: C={} Rust={}", a, b);
    assert!(a > 0);
    a as usize
}

fn cbound(n: usize) -> usize {
    let (a, b) = both(|l| unsafe { l.get::<Fn_compressBound>("LZ4_compressBound")(n as c_int) });
    assert_eq!(a, b, "LZ4_compressBound({}): C={} Rust={}", n, a, b);
    a as usize
}

/// Run `f` once per library with a private, sentinel-filled destination buffer.
/// Returns `(rc_c, out_c, rc_r, out_r)` where `out` is the first `rc` bytes.
///
/// The buffers are over-allocated so that a Rust-side overrun cannot corrupt
/// the harness heap (and `LZ4_wildCopy8` slop past the reported size is never
/// compared, since it is unspecified).
fn dual<F>(bufsize: usize, mut f: F) -> (c_int, Vec<u8>, c_int, Vec<u8>)
where
    F: FnMut(&'static Lib, *mut c_char) -> c_int,
{
    let mut cb = vec![0xA5u8; bufsize + 64];
    let mut rb = vec![0xA5u8; bufsize + 64];
    let crc = f(c(), cb.as_mut_ptr() as *mut c_char);
    let rrc = f(r(), rb.as_mut_ptr() as *mut c_char);
    let cn = if crc > 0 {
        (crc as usize).min(cb.len())
    } else {
        0
    };
    let rn = if rrc > 0 {
        (rrc as usize).min(rb.len())
    } else {
        0
    };
    (crc, cb[..cn].to_vec(), rrc, rb[..rn].to_vec())
}

/// `dual()` for the `*srcSizePtr` (destSize / fillOutput) entry points.
/// Returns `(rc_c, srcSize_c, out_c, rc_r, srcSize_r, out_r)`.
fn dual_ss<F>(
    bufsize: usize,
    srcsize: c_int,
    mut f: F,
) -> (c_int, c_int, Vec<u8>, c_int, c_int, Vec<u8>)
where
    F: FnMut(&'static Lib, *mut c_char, *mut c_int) -> c_int,
{
    let mut cb = vec![0xA5u8; bufsize + 64];
    let mut rb = vec![0xA5u8; bufsize + 64];
    let mut css: c_int = srcsize;
    let mut rss: c_int = srcsize;
    let crc = f(c(), cb.as_mut_ptr() as *mut c_char, &mut css);
    let rrc = f(r(), rb.as_mut_ptr() as *mut c_char, &mut rss);
    let cn = if crc > 0 {
        (crc as usize).min(cb.len())
    } else {
        0
    };
    let rn = if rrc > 0 {
        (rrc as usize).min(rb.len())
    } else {
        0
    };
    (crc, css, cb[..cn].to_vec(), rrc, rss, rb[..rn].to_vec())
}

/// Decompress `comp` in BOTH libraries and require an exact recovery of `orig`.
fn assert_roundtrip(ctx: &str, comp: &[u8], orig: &[u8]) {
    for l in [c(), r()] {
        let mut out = vec![0u8; orig.len() + 64];
        let rc = unsafe {
            l.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                comp.as_ptr() as *const c_char,
                out.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                orig.len() as c_int,
            )
        };
        assert_eq!(
            rc,
            orig.len() as c_int,
            "{}: {} LZ4_decompress_safe returned {} (expected {})",
            ctx,
            l.which,
            rc,
            orig.len()
        );
        assert_bytes_eq!(
            format!("{} [{} round-trip]", ctx, l.which),
            out[..orig.len()],
            orig[..]
        );
    }
}

/// Same, but the block references an external dictionary.
fn assert_roundtrip_dict(ctx: &str, comp: &[u8], orig: &[u8], dict: &[u8]) {
    for l in [c(), r()] {
        let mut out = vec![0u8; orig.len() + 64];
        let rc = unsafe {
            l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                comp.as_ptr() as *const c_char,
                out.as_mut_ptr() as *mut c_char,
                comp.len() as c_int,
                orig.len() as c_int,
                dict.as_ptr() as *const c_char,
                dict.len() as c_int,
            )
        };
        assert_eq!(
            rc,
            orig.len() as c_int,
            "{}: {} LZ4_decompress_safe_usingDict returned {} (expected {}, dictSize={})",
            ctx,
            l.which,
            rc,
            orig.len(),
            dict.len()
        );
        assert_bytes_eq!(
            format!("{} [{} dict round-trip]", ctx, l.which),
            out[..orig.len()],
            orig[..]
        );
    }
}

fn assert_state_eq(ctx: &str, cs: *const c_void, rs: *const c_void, n: usize, ignore_dictctx: bool) {
    let a: &[u8] = unsafe { std::slice::from_raw_parts(cs as *const u8, n) };
    let b: &[u8] = unsafe { std::slice::from_raw_parts(rs as *const u8, n) };
    if ignore_dictctx {
        assert!(n >= DICTCTX_FIELD_OFF + 8);
        assert_bytes_eq!(
            format!("{} [LZ4_streamHC_t bytes 0..{}]", ctx, DICTCTX_FIELD_OFF),
            a[..DICTCTX_FIELD_OFF],
            b[..DICTCTX_FIELD_OFF]
        );
        assert_bytes_eq!(
            format!("{} [LZ4_streamHC_t bytes {}..]", ctx, DICTCTX_FIELD_OFF + 8),
            a[DICTCTX_FIELD_OFF + 8..],
            b[DICTCTX_FIELD_OFF + 8..]
        );
    } else {
        assert_bytes_eq!(format!("{} [LZ4_streamHC_t bytes]", ctx), a, b);
    }
}

/// A pair of `LZ4_streamHC_t` allocated by each library.
struct Streams {
    cs: *mut c_void,
    rs: *mut c_void,
    n: usize,
}

impl Streams {
    fn new() -> Streams {
        let n = sizeof_state_hc();
        let (cs, rs) = both(|l| unsafe { l.get::<Fn_createStream>("LZ4_createStreamHC")() });
        assert!(!cs.is_null(), "C LZ4_createStreamHC returned NULL");
        assert!(!rs.is_null(), "Rust LZ4_createStreamHC returned NULL");
        Streams { cs, rs, n }
    }
    fn of(&self, l: &Lib) -> *mut c_void {
        if l.which == "C" {
            self.cs
        } else {
            self.rs
        }
    }
    fn call<T, F: Fn(&'static Lib, *mut c_void) -> T>(&self, f: F) -> (T, T) {
        (f(c(), self.cs), f(r(), self.rs))
    }
    fn reset(&self, level: c_int) {
        self.call(|l, s| unsafe {
            l.get::<Fn_resetStreamHC>("LZ4_resetStreamHC")(s, level);
        });
    }
    fn reset_fast(&self, level: c_int) {
        self.call(|l, s| unsafe {
            l.get::<Fn_resetStreamHC>("LZ4_resetStreamHC_fast")(s, level);
        });
    }
    fn set_level(&self, level: c_int) {
        self.call(|l, s| unsafe {
            l.get::<Fn_setCompressionLevel>("LZ4_setCompressionLevel")(s, level);
        });
    }
    fn favor(&self, favor: c_int) {
        self.call(|l, s| unsafe {
            l.get::<Fn_favorDecompressionSpeed>("LZ4_favorDecompressionSpeed")(s, favor);
        });
    }
    fn load_dict(&self, ctx: &str, dict: &[u8], dict_size: c_int) -> c_int {
        let (a, b) = self.call(|l, s| unsafe {
            l.get::<Fn_loadDictHC>("LZ4_loadDictHC")(s, dict.as_ptr() as *const c_char, dict_size)
        });
        assert_eq!(a, b, "{}: LZ4_loadDictHC C={} Rust={}", ctx, a, b);
        a
    }
    fn assert_state(&self, ctx: &str, ignore_dictctx: bool) {
        assert_state_eq(ctx, self.cs, self.rs, self.n, ignore_dictctx);
    }
    /// `LZ4_compress_HC_continue` on both streams; asserts rc + bytes identical.
    fn continue_block(&self, ctx: &str, src: &[u8], cap: usize) -> (c_int, Vec<u8>) {
        let (crc, cv, rrc, rv) = dual(cap, |l, dst| unsafe {
            l.get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                self.of(l),
                src.as_ptr() as *const c_char,
                dst,
                src.len() as c_int,
                cap as c_int,
            )
        });
        assert_eq!(
            crc, rrc,
            "{}: LZ4_compress_HC_continue rc C={} Rust={}",
            ctx, crc, rrc
        );
        assert_bytes_eq!(ctx, cv, rv);
        (crc, cv)
    }
}

impl Drop for Streams {
    fn drop(&mut self) {
        let (a, b) = both(|l| unsafe {
            let p = if l.which == "C" { self.cs } else { self.rs };
            l.get::<Fn_freeStream>("LZ4_freeStreamHC")(p)
        });
        assert_eq!(a, 0, "C LZ4_freeStreamHC returned {}", a);
        assert_eq!(b, 0, "Rust LZ4_freeStreamHC returned {}", b);
    }
}

/// A pair of externally allocated `LZ4_streamHC_t`-sized scratch buffers.
struct States {
    c: Aligned,
    r: Aligned,
    n: usize,
}

impl States {
    fn new() -> States {
        let n = sizeof_state_hc();
        States {
            c: Aligned::new(n, 8),
            r: Aligned::new(n, 8),
            n,
        }
    }
    fn of(&self, l: &Lib) -> *mut c_void {
        if l.which == "C" {
            self.c.ptr()
        } else {
            self.r.ptr()
        }
    }
    fn assert_state(&self, ctx: &str) {
        assert_eq!(self.c.size(), self.n);
        assert_eq!(self.r.size(), self.n);
        assert_bytes_eq!(
            format!("{} [extState bytes]", ctx),
            self.c.bytes(),
            self.r.bytes()
        );
    }
}

/// `gen()`, but guaranteed to be backed by a real heap allocation.
///
/// `Vec::as_ptr()` on a zero-capacity `Vec` returns the dangling address `0x1`.
/// Handing that to the library breaks a documented precondition: with
/// `srcSize == 0`, `LZ4HC_compress_optimal()` computes `mflimit = iend - MFLIMIT`
/// (lz4hc.c:1846) which *wraps around* for such an address, so `ip <= mflimit`
/// is true and the parser dereferences `src`.  Real callers always pass a real
/// buffer, so the tests do too.
fn gen_buf(rng: &mut Rng, len: usize, shape: Shape) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(len + 64);
    v.extend_from_slice(&gen(rng, len, shape));
    assert_eq!(v.len(), len);
    v
}

/// Build a buffer that shares many fragments with `dict` (so that dictionary
/// matches are actually found).
fn mix_from(rng: &mut Rng, dict: &[u8], len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(len);
    while v.len() < len {
        let remain = len - v.len();
        if dict.len() > 16 && rng.bool() {
            let want = rng.range(4, 64);
            let n = want.min(dict.len()).min(remain);
            let off = rng.below(dict.len() - n + 1);
            v.extend_from_slice(&dict[off..off + n]);
        } else {
            let n = rng.range(1, 16).min(remain);
            for _ in 0..n {
                v.push(rng.byte());
            }
        }
    }
    v.truncate(len);
    v
}

// ===========================================================================
// 0. sizes / constants
// ===========================================================================

#[test]
fn hc_sizeof_state_agreement() {
    let n = sizeof_state_hc();
    // LZ4_STREAMHC_MINSIZE from lz4hc.h
    assert_eq!(n, 262200, "LZ4_sizeofStateHC() = {}", n);
    let (a, b) = both(|l| unsafe { l.get::<FnI>("LZ4_sizeofStreamStateHC")() });
    assert_eq!(
        a, b,
        "LZ4_sizeofStreamStateHC: C={} Rust={}",
        a, b
    );
    assert_eq!(a as usize, n);
}

// ===========================================================================
// A. LZ4_compress_HC — full level sweep x all shapes x all size thresholds
// ===========================================================================

#[test]
fn compress_hc_level_sweep_small_sizes() {
    let mut rng = Rng::new(0x4843_0001);
    for &shape in ALL_SHAPES {
        for &size in SMALL_SIZES {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            for &lvl in LEVELS {
                let ctx = format!(
                    "LZ4_compress_HC shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        lvl,
                    )
                });
                assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                assert!(crc > 0, "{}: C returned {}", ctx, crc);
                assert_bytes_eq!(ctx, cv, rv);
                assert_roundtrip(&ctx, &cv, &data);
            }
        }
    }
}

#[test]
fn compress_hc_big_sizes() {
    const LVLS: &[c_int] = LEVELS;
    let mut rng = Rng::new(0x4843_0002);
    for &shape in ALL_SHAPES {
        for &size in BIG_SIZES {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            for &lvl in LVLS {
                let ctx = format!(
                    "LZ4_compress_HC shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        lvl,
                    )
                });
                assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                assert!(crc > 0, "{}: C returned {}", ctx, crc);
                assert_bytes_eq!(ctx, cv, rv);
                assert_roundtrip(&ctx, &cv, &data);
            }
        }
    }
}

/// Out-of-range levels are *clamped*, not rejected: assert the equivalence
/// classes {..,-1,0} == {9}, {1} == {2}, {12,13,20,100,INT_MAX} hold *inside*
/// each library as well as across libraries.
#[test]
fn hc_level_clamping_equivalence_classes() {
    let classes: &[(&[c_int], c_int)] = &[
        (&[c_int::MIN, -100, -1, 0], 9),
        (&[1], 2),
        (&[13, 20, 100, c_int::MAX], 12),
    ];
    let mut rng = Rng::new(0x4843_0003);
    for &shape in ALL_SHAPES {
        for &size in &[13usize, 64, 4096, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            for (aliases, canonical) in classes {
                let (_, ref_c, _, ref_r) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        *canonical,
                    )
                });
                for &lvl in *aliases {
                    let ctx = format!(
                        "clamping level={} ~ {} shape={:?} size={}",
                        lvl, canonical, shape, size
                    );
                    let (_, cv, _, rv) = dual(bound, |l, dst| unsafe {
                        l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                            data.as_ptr() as *const c_char,
                            dst,
                            size as c_int,
                            bound as c_int,
                            lvl,
                        )
                    });
                    assert_bytes_eq!(format!("{} [C self-consistency]", ctx), cv, ref_c);
                    assert_bytes_eq!(format!("{} [Rust self-consistency]", ctx), rv, ref_r);
                    assert_bytes_eq!(format!("{} [C vs Rust]", ctx), cv, rv);
                }
            }
        }
    }
}

// ===========================================================================
// B. LZ4_compress_HC_extStateHC / _fastReset  (+ state bytes)
// ===========================================================================

#[test]
fn compress_hc_ext_state() {
    let st = States::new();
    let mut rng = Rng::new(0x4843_0011);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 4, 5, 12, 13, 16, 64, 255, 4096, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            for &lvl in LEVELS {
                let ctx = format!(
                    "LZ4_compress_HC_extStateHC shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC_extStateHC>("LZ4_compress_HC_extStateHC")(
                        st.of(l),
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        lvl,
                    )
                });
                assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                assert!(crc > 0, "{}: C returned {}", ctx, crc);
                assert_bytes_eq!(ctx, cv, rv);
                st.assert_state(&ctx);
                assert_roundtrip(&ctx, &cv, &data);

                // ... and the fastReset variant on the (now initialised) state
                let ctx2 = format!(
                    "LZ4_compress_HC_extStateHC_fastReset shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (crc2, cv2, rrc2, rv2) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC_extStateHC>(
                        "LZ4_compress_HC_extStateHC_fastReset",
                    )(
                        st.of(l),
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        lvl,
                    )
                });
                assert_eq!(crc2, rrc2, "{}: rc C={} Rust={}", ctx2, crc2, rrc2);
                assert_bytes_eq!(ctx2, cv2, rv2);
                st.assert_state(&ctx2);
                if crc2 > 0 {
                    assert_roundtrip(&ctx2, &cv2, &data);
                }
            }
        }
    }
}

/// Repeated re-use of one state across many calls — what `_fastReset`
/// optimises — including calls that FAIL (which sets `ctx->dirty` and forces
/// the next `LZ4_resetStreamHC_fast()` to do a full re-init).
#[test]
fn compress_hc_ext_state_reuse_fast_reset() {
    let st = States::new();
    let mut rng = Rng::new(0x4843_0012);
    // prime the state with the full-init entry point
    {
        let data = gen_buf(&mut rng, 100, Shape::Text);
        let bound = cbound(100);
        let (crc, _, rrc, _) = dual(bound, |l, dst| unsafe {
            l.get::<Fn_compress_HC_extStateHC>("LZ4_compress_HC_extStateHC")(
                st.of(l),
                data.as_ptr() as *const c_char,
                dst,
                100,
                bound as c_int,
                9,
            )
        });
        assert_eq!(crc, rrc);
    }
    for i in 0..120 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = rng.range(0, 5000);
        let data = gen_buf(&mut rng, size, shape);
        let bound = cbound(size);
        let lvl = LEVELS[rng.below(LEVELS.len())];
        // every 3rd iteration deliberately starves the output buffer
        let cap = if i % 3 == 2 {
            rng.below(bound.max(1))
        } else {
            bound
        };
        let ctx = format!(
            "extStateHC_fastReset reuse iter={} shape={:?} size={} level={} cap={}",
            i, shape, size, lvl, cap
        );
        let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
            l.get::<Fn_compress_HC_extStateHC>("LZ4_compress_HC_extStateHC_fastReset")(
                st.of(l),
                data.as_ptr() as *const c_char,
                dst,
                size as c_int,
                cap as c_int,
                lvl,
            )
        });
        assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
        assert_bytes_eq!(ctx, cv, rv);
        st.assert_state(&ctx);
        if crc > 0 {
            assert_roundtrip(&ctx, &cv, &data);
        }
    }
}

// ===========================================================================
// C. LZ4_compress_HC_destSize
// ===========================================================================

#[test]
fn compress_hc_dest_size() {
    let st = States::new();
    let mut rng = Rng::new(0x4843_0021);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 4, 12, 13, 64, 300, 1000, 5000, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            let mut targets: Vec<usize> = vec![0, 1, 2, 5, 12, 16, 64, 300];
            targets.push(bound / 8);
            targets.push(bound / 4);
            targets.push(bound / 2);
            targets.push(bound * 3 / 4);
            targets.push(bound);
            for &lvl in BAND_LEVELS {
                for &target in &targets {
                    let ctx = format!(
                        "LZ4_compress_HC_destSize shape={:?} size={} level={} target={}",
                        shape, size, lvl, target
                    );
                    let (crc, css, cv, rrc, rss, rv) =
                        dual_ss(target, size as c_int, |l, dst, ssp| unsafe {
                            l.get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                                st.of(l),
                                data.as_ptr() as *const c_char,
                                dst,
                                ssp,
                                target as c_int,
                                lvl,
                            )
                        });
                    assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                    assert_eq!(
                        css, rss,
                        "{}: *srcSizePtr C={} Rust={}",
                        ctx, css, rss
                    );
                    assert_bytes_eq!(ctx, cv, rv);
                    st.assert_state(&ctx);
                    assert!(
                        crc <= target as c_int,
                        "{}: C wrote {} > target {}",
                        ctx,
                        crc,
                        target
                    );
                    assert!(
                        css >= 0 && css as usize <= size,
                        "{}: C consumed {} of {}",
                        ctx,
                        css,
                        size
                    );
                    if crc > 0 {
                        // the block must decode to exactly the consumed prefix
                        assert_roundtrip(&ctx, &cv, &data[..css as usize]);
                    }
                }
            }
        }
    }
}

#[test]
fn compress_hc_continue_dest_size() {
    let mut rng = Rng::new(0x4843_0022);
    for &shape in &[Shape::Incompressible, Shape::Text, Shape::Runs] {
        let data = gen_buf(&mut rng, 40000, shape);
        for &lvl in &[1 as c_int, 2, 3, 9, 12] {
            for &target in &[0usize, 1, 2, 5, 12, 16, 64, 300, 3000, 20000] {
                let st = Streams::new();
                st.reset(lvl);
                let mut off = 0usize;
                let mut out = vec![0u8; data.len() + 64];
                for step in 0..6 {
                    if off >= data.len() {
                        break;
                    }
                    let avail = data.len() - off;
                    let ctx = format!(
                        "LZ4_compress_HC_continue_destSize shape={:?} level={} target={} step={} off={}",
                        shape, lvl, target, step, off
                    );
                    let src = &data[off..];
                    let (crc, css, cv, rrc, rss, rv) =
                        dual_ss(target, avail as c_int, |l, dst, ssp| unsafe {
                            l.get::<Fn_compress_HC_continue_destSize>(
                                "LZ4_compress_HC_continue_destSize",
                            )(
                                st.of(l),
                                src.as_ptr() as *const c_char,
                                dst,
                                ssp,
                                target as c_int,
                            )
                        });
                    assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                    assert_eq!(css, rss, "{}: *srcSizePtr C={} Rust={}", ctx, css, rss);
                    assert_bytes_eq!(ctx, cv, rv);
                    st.assert_state(&ctx, false);
                    if crc <= 0 {
                        break;
                    }
                    assert!(crc <= target as c_int, "{}: rc {} > target", ctx, crc);
                    let consumed = css as usize;
                    // decode against the already-decoded contiguous prefix
                    for l in [c(), r()] {
                        let rc = unsafe {
                            l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                                cv.as_ptr() as *const c_char,
                                out[off..].as_mut_ptr() as *mut c_char,
                                crc,
                                consumed as c_int,
                                out.as_ptr() as *const c_char,
                                off as c_int,
                            )
                        };
                        assert_eq!(
                            rc, consumed as c_int,
                            "{}: {} decode rc={} expected {}",
                            ctx, l.which, rc, consumed
                        );
                    }
                    assert_bytes_eq!(
                        format!("{} [decoded prefix]", ctx),
                        out[off..off + consumed],
                        data[off..off + consumed]
                    );
                    if consumed == 0 {
                        break;
                    }
                    off += consumed;
                }
            }
        }
    }
}

// ===========================================================================
// D. dstCapacity sweep — limitedOutput / _dest_overflow rejection branches
//    (ERRORS.md rows 49,50,54,55,56,57,58,59)
// ===========================================================================

#[test]
fn compress_hc_dst_capacity_sweep() {
    let mut rng = Rng::new(0x4843_0031);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 12, 13, 64, 300, 4096, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);
            for &lvl in BAND_LEVELS {
                // reference output with a guaranteed-sufficient buffer
                let (exact, _, rexact, _) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        bound as c_int,
                        lvl,
                    )
                });
                assert_eq!(exact, rexact);
                assert!(exact > 0);
                let e = exact as usize;
                let mut caps: Vec<usize> = vec![bound, e, e.saturating_sub(1), e / 2, size, 1, 0];
                caps.sort();
                caps.dedup();
                for &cap in &caps {
                    let ctx = format!(
                        "LZ4_compress_HC dstCapacity sweep shape={:?} size={} level={} cap={} (exact={})",
                        shape, size, lvl, cap, e
                    );
                    let (crc, cv, rrc, rv) = dual(cap, |l, dst| unsafe {
                        l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                            data.as_ptr() as *const c_char,
                            dst,
                            size as c_int,
                            cap as c_int,
                            lvl,
                        )
                    });
                    assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                    assert_bytes_eq!(ctx, cv, rv);
                    assert!(
                        crc >= 0 && crc as usize <= cap,
                        "{}: rc {} exceeds capacity",
                        ctx,
                        crc
                    );
                    if crc > 0 {
                        assert_roundtrip(&ctx, &cv, &data);
                    }
                }
                // fillOutput counterpart of the same capacity sweep
                let st = States::new();
                for &cap in &caps {
                    let ctx = format!(
                        "LZ4_compress_HC_destSize capacity sweep shape={:?} size={} level={} cap={}",
                        shape, size, lvl, cap
                    );
                    let (crc, css, cv, rrc, rss, rv) =
                        dual_ss(cap, size as c_int, |l, dst, ssp| unsafe {
                            l.get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                                st.of(l),
                                data.as_ptr() as *const c_char,
                                dst,
                                ssp,
                                cap as c_int,
                                lvl,
                            )
                        });
                    assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                    assert_eq!(css, rss, "{}: *srcSizePtr C={} Rust={}", ctx, css, rss);
                    assert_bytes_eq!(ctx, cv, rv);
                    st.assert_state(&ctx);
                    if crc > 0 {
                        assert_roundtrip(&ctx, &cv, &data[..css as usize]);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// E. Streaming HC
// ===========================================================================

/// prefix / contiguous regime: N consecutive chunks of one buffer.
#[test]
fn stream_hc_prefix_chunks() {
    const LVLS: &[c_int] = &[0, 1, 2, 3, 9, 10, 12, 13];
    let mut rng = Rng::new(0x4843_0041);
    for &shape in ALL_SHAPES {
        let data = gen_buf(&mut rng, 30000, shape);
        for nchunks in 1usize..=8 {
            for &lvl in LVLS {
                let use_fast = nchunks % 2 == 0;
                let st = Streams::new();
                if use_fast {
                    // resetStreamHC_fast is legal on a freshly created stream
                    st.reset_fast(lvl);
                } else {
                    st.reset(lvl);
                }
                st.assert_state(&format!("reset(level={})", lvl), false);
                let mut out = vec![0u8; data.len() + 64];
                let chunk = data.len() / nchunks;
                let mut off = 0usize;
                for i in 0..nchunks {
                    let end = if i + 1 == nchunks {
                        data.len()
                    } else {
                        off + chunk
                    };
                    let src = &data[off..end];
                    let bound = cbound(src.len());
                    let ctx = format!(
                        "LZ4_compress_HC_continue prefix shape={:?} nchunks={} chunk={} level={} fast={}",
                        shape, nchunks, i, lvl, use_fast
                    );
                    let (crc, cv) = st.continue_block(&ctx, src, bound);
                    assert!(crc > 0, "{}: rc={}", ctx, crc);
                    st.assert_state(&ctx, false);
                    for l in [c(), r()] {
                        let rc = unsafe {
                            l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                                cv.as_ptr() as *const c_char,
                                out[off..].as_mut_ptr() as *mut c_char,
                                crc,
                                src.len() as c_int,
                                out.as_ptr() as *const c_char,
                                off as c_int,
                            )
                        };
                        assert_eq!(
                            rc,
                            src.len() as c_int,
                            "{}: {} decode rc={}",
                            ctx,
                            l.which,
                            rc
                        );
                    }
                    assert_bytes_eq!(
                        format!("{} [decoded]", ctx),
                        out[off..end],
                        data[off..end]
                    );
                    off = end;
                }
            }
        }
    }
}

/// extDict regime: `LZ4_loadDictHC` from a separate buffer, then compress a
/// block from an unrelated buffer.  Dict sizes > 64 KB are truncated to the
/// last 64 KB by the C (lz4hc.c:1634).
#[test]
fn stream_hc_load_dict_ext_dict() {
    const DICT_SIZES: &[usize] = &[0, 1, 3, 4, 8, 100, 65535, 65536, 65537, 100000];
    const LVLS: &[c_int] = &[1, 2, 3, 9, 10, 12];
    let mut rng = Rng::new(0x4843_0042);
    for &shape in ALL_SHAPES {
        for &dsize in DICT_SIZES {
            let dict = gen_buf(&mut rng, dsize, shape);
            let src = mix_from(&mut rng, &dict, 9000);
            for &lvl in LVLS {
                let st = Streams::new();
                st.reset(lvl);
                let ctx0 = format!(
                    "LZ4_loadDictHC shape={:?} dictSize={} level={}",
                    shape, dsize, lvl
                );
                let loaded = st.load_dict(&ctx0, &dict, dsize as c_int);
                assert_eq!(
                    loaded as usize,
                    dsize.min(65536),
                    "{}: loaded {} (expected min(dictSize,64KB))",
                    ctx0,
                    loaded
                );
                st.assert_state(&ctx0, false);
                let used = &dict[dsize - loaded as usize..];
                let bound = cbound(src.len());
                let ctx = format!(
                    "LZ4_compress_HC_continue extDict shape={:?} dictSize={} level={}",
                    shape, dsize, lvl
                );
                let (crc, cv) = st.continue_block(&ctx, &src, bound);
                assert!(crc > 0, "{}: rc={}", ctx, crc);
                st.assert_state(&ctx, false);
                assert_roundtrip_dict(&ctx, &cv, &src, used);
            }
        }
    }
}

/// dictCtx regime: `LZ4_attach_HC_dictionary` against a separately loaded
/// stream.  Sizes <= 4 KB take the `usingDictCtxHc` path, sizes > 4 KB take the
/// `LZ4_memcpy(ctx, ctx->dictCtx, ...)` path (lz4hc.c:1457), and mismatched
/// strategies (mid vs hc) defeat `isStateCompatible()`.
#[test]
fn stream_hc_attach_dictionary() {
    // one level per strategy: 2 = lz4mid, 9 = hashChain, 12 = optimal.  Mixed
    // pairs exercise `isStateCompatible()` failing (lz4hc.c:1434).
    const LVLS: &[c_int] = &[2, 9, 12];
    let mut rng = Rng::new(0x4843_0043);
    for &shape in &[
        Shape::Incompressible,
        Shape::Text,
        Shape::Runs,
        Shape::Periodic(64),
    ] {
        for &dsize in &[0usize, 100, 5000, 100000] {
            let dict = gen_buf(&mut rng, dsize, shape);
            for &dict_lvl in LVLS {
                let dst = Streams::new();
                dst.reset(dict_lvl);
                let loaded = dst.load_dict(
                    &format!("attach: LZ4_loadDictHC dictSize={}", dsize),
                    &dict,
                    dsize as c_int,
                );
                let used = &dict[dsize - loaded as usize..];
                for &work_lvl in LVLS {
                    for &size in &[100usize, 5000] {
                        let src = mix_from(&mut rng, &dict, size);
                        let work = Streams::new();
                        work.reset(work_lvl);
                        work.call(|l, s| unsafe {
                            let d = if l.which == "C" { dst.cs } else { dst.rs };
                            l.get::<Fn_attach_dictionary>("LZ4_attach_HC_dictionary")(
                                s,
                                d as *const c_void,
                            );
                        });
                        let bound = cbound(size);
                        let ctx = format!(
                            "attach_HC_dictionary shape={:?} dictSize={} dictLevel={} workLevel={} size={}",
                            shape, dsize, dict_lvl, work_lvl, size
                        );
                        let (crc, cv) = work.continue_block(&ctx, &src, bound);
                        assert!(crc > 0, "{}: rc={}", ctx, crc);
                        // the dictCtx field holds a pointer into the other
                        // library's allocation, so it is masked out.
                        work.assert_state(&ctx, true);
                        assert_roundtrip_dict(&ctx, &cv, &src, used);
                        // detach and compress again without any dictionary
                        work.reset(work_lvl);
                        work.call(|l, s| unsafe {
                            l.get::<Fn_attach_dictionary>("LZ4_attach_HC_dictionary")(
                                s,
                                std::ptr::null(),
                            );
                        });
                        let ctx2 = format!("{} [detached]", ctx);
                        let (crc2, cv2) = work.continue_block(&ctx2, &src, bound);
                        assert!(crc2 > 0, "{}: rc={}", ctx2, crc2);
                        work.assert_state(&ctx2, false);
                        assert_roundtrip(&ctx2, &cv2, &src);
                    }
                }
            }
        }
    }
}

/// `LZ4_saveDictHC` — the C forces `<4 -> 0`, clamps to 64 KB and to the
/// current prefix size (lz4hc.c:1748-1750).
#[test]
fn stream_hc_save_dict() {
    const SAVE_SIZES: &[usize] = &[0, 1, 3, 4, 100, 65536, 100000];
    const LVLS: &[c_int] = &[2, 9, 12];
    let mut rng = Rng::new(0x4843_0044);
    // one shared "safe buffer" so that the resulting stream state (which
    // points into it) stays comparable between the two libraries
    let mut safe = vec![0u8; 100000 + 64];
    for &shape in ALL_SHAPES {
        for &prefix in &[0usize, 3, 4, 100, 70000] {
            let data = gen_buf(&mut rng, prefix, shape);
            let next = gen_buf(&mut rng, 3000, shape);
            for &lvl in LVLS {
                for &save in SAVE_SIZES {
                    let st = Streams::new();
                    st.reset(lvl);
                    let ctx0 = format!(
                        "saveDictHC setup shape={:?} prefix={} level={} save={}",
                        shape, prefix, lvl, save
                    );
                    if prefix > 0 {
                        let bound = cbound(prefix);
                        let (crc, cv) = st.continue_block(&ctx0, &data, bound);
                        assert!(crc > 0, "{}: rc={}", ctx0, crc);
                        assert_roundtrip(&ctx0, &cv, &data);
                    }
                    // C first, snapshot, then Rust into the same buffer
                    for b in safe.iter_mut() {
                        *b = 0x5A;
                    }
                    let crc = unsafe {
                        c().get::<Fn_saveDictHC>("LZ4_saveDictHC")(
                            st.cs,
                            safe.as_mut_ptr() as *mut c_char,
                            save as c_int,
                        )
                    };
                    let c_saved = safe.clone();
                    for b in safe.iter_mut() {
                        *b = 0x5A;
                    }
                    let rrc = unsafe {
                        r().get::<Fn_saveDictHC>("LZ4_saveDictHC")(
                            st.rs,
                            safe.as_mut_ptr() as *mut c_char,
                            save as c_int,
                        )
                    };
                    let ctx = format!(
                        "LZ4_saveDictHC shape={:?} prefix={} level={} save={}",
                        shape, prefix, lvl, save
                    );
                    assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                    let expect = {
                        let mut d = save;
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
                    assert_eq!(
                        crc as usize, expect,
                        "{}: C returned {} (expected clamped {})",
                        ctx, crc, expect
                    );
                    assert_bytes_eq!(format!("{} [safeBuffer]", ctx), c_saved, safe);
                    if crc > 0 {
                        assert_bytes_eq!(
                            format!("{} [content]", ctx),
                            safe[..crc as usize],
                            data[prefix - crc as usize..]
                        );
                    }
                    st.assert_state(&ctx, false);
                    // Continue compressing after the save.
                    // Skipped when nothing was ever compressed: saving from a
                    // virgin stream leaves dictLimit == 0 (lz4hc.c:1757), and
                    // compressing from index 0 trips `assert(matchIndex <
                    // ipIndex)` inside the assert-enabled C build.
                    if prefix > 0 {
                        let bound = cbound(next.len());
                        let ctx2 = format!("{} [continue after save]", ctx);
                        let (crc2, cv2) = st.continue_block(&ctx2, &next, bound);
                        assert!(crc2 > 0, "{}: rc={}", ctx2, crc2);
                        st.assert_state(&ctx2, false);
                        assert_roundtrip_dict(&ctx2, &cv2, &next, &safe[..crc as usize]);
                    }
                }
            }
        }
    }
}

/// `LZ4_initStreamHC` on a user buffer, `LZ4_setCompressionLevel`,
/// `LZ4_favorDecompressionSpeed` (0/1) across the whole level range, and
/// `LZ4_resetStreamHC_fast` sequencing.
#[test]
fn stream_hc_init_level_and_favor() {
    let n = sizeof_state_hc();
    let mut rng = Rng::new(0x4843_0045);
    let buf_c = Aligned::new(n, 8);
    let buf_r = Aligned::new(n, 8);
    let (pc, pr) = (
        unsafe { c().get::<Fn_initStreamHC>("LZ4_initStreamHC")(buf_c.ptr(), n) },
        unsafe { r().get::<Fn_initStreamHC>("LZ4_initStreamHC")(buf_r.ptr(), n) },
    );
    assert_eq!(pc, buf_c.ptr(), "C LZ4_initStreamHC did not return buffer");
    assert_eq!(pr, buf_r.ptr(), "Rust LZ4_initStreamHC did not return buffer");
    assert_bytes_eq!(
        "LZ4_initStreamHC [state]".to_string(),
        buf_c.bytes(),
        buf_r.bytes()
    );

    for &shape in &[Shape::Text, Shape::Runs, Shape::Incompressible] {
        let data = gen_buf(&mut rng, 6000, shape);
        let bound = cbound(data.len());
        for &lvl in LEVELS {
            for &favor in &[0 as c_int, 1] {
                // statically allocated stream, initialised by the user
                let ctx = format!(
                    "initStreamHC+setCompressionLevel shape={:?} level={} favor={}",
                    shape, lvl, favor
                );
                for l in [c(), r()] {
                    let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
                    unsafe {
                        l.get::<Fn_resetStreamHC>("LZ4_resetStreamHC")(p, 9);
                        l.get::<Fn_setCompressionLevel>("LZ4_setCompressionLevel")(p, lvl);
                        l.get::<Fn_favorDecompressionSpeed>("LZ4_favorDecompressionSpeed")(p, favor);
                    }
                }
                assert_bytes_eq!(
                    format!("{} [state after setup]", ctx),
                    buf_c.bytes(),
                    buf_r.bytes()
                );
                let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
                    let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
                    l.get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                        p,
                        data.as_ptr() as *const c_char,
                        dst,
                        data.len() as c_int,
                        bound as c_int,
                    )
                });
                assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
                assert!(crc > 0, "{}: rc={}", ctx, crc);
                assert_bytes_eq!(ctx, cv, rv);
                assert_bytes_eq!(
                    format!("{} [state after compress]", ctx),
                    buf_c.bytes(),
                    buf_r.bytes()
                );
                assert_roundtrip(&ctx, &cv, &data);
                // favorDecSpeed only reaches LZ4HC_compress_optimal, so for
                // effective levels < LZ4HC_CLEVEL_OPT_MIN it cannot change the
                // output.  (levels <= 0 are remapped to 9.)
                if favor == 1 && lvl <= 9 {
                    for l in [c(), r()] {
                        let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
                        unsafe {
                            l.get::<Fn_resetStreamHC>("LZ4_resetStreamHC")(p, lvl);
                            l.get::<Fn_favorDecompressionSpeed>("LZ4_favorDecompressionSpeed")(p, 0);
                        }
                    }
                    let (crc0, cv0, rrc0, rv0) = dual(bound, |l, dst| unsafe {
                        let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
                        l.get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                            p,
                            data.as_ptr() as *const c_char,
                            dst,
                            data.len() as c_int,
                            bound as c_int,
                        )
                    });
                    assert_eq!(crc0, rrc0);
                    assert_bytes_eq!(format!("{} [favor irrelevant, C]", ctx), cv0, cv);
                    assert_bytes_eq!(format!("{} [favor irrelevant, Rust]", ctx), rv0, rv);
                }
            }
        }
    }

    // resetStreamHC_fast sequencing on a heap stream, alternating levels
    let st = Streams::new();
    let data = gen_buf(&mut rng, 20000, Shape::Runs);
    for (i, &lvl) in LEVELS.iter().enumerate() {
        if i % 2 == 0 {
            st.reset(lvl);
        } else {
            st.reset_fast(lvl);
        }
        st.set_level(lvl);
        st.favor((i % 2) as c_int);
        let ctx = format!("reset sequencing i={} level={}", i, lvl);
        st.assert_state(&ctx, false);
        let bound = cbound(data.len());
        let (crc, cv) = st.continue_block(&ctx, &data, bound);
        assert!(crc > 0, "{}: rc={}", ctx, crc);
        st.assert_state(&ctx, false);
        assert_roundtrip(&ctx, &cv, &data);
    }
}

// ===========================================================================
// F. LZ4HC_searchExtDict — exported internal, called directly
// ===========================================================================

#[test]
fn hc_search_ext_dict() {
    // `LZ4HC_CCtx_internal*` == `LZ4_streamHC_t*`: `internal_donotuse` is the
    // second member of a union whose first member is `char[LZ4_STREAMHC_MINSIZE]`,
    // so it lives at offset 0 (verified against c_src/include/lz4hc.h:253-256).
    const LVLS: &[c_int] = &[2, 9, 12];
    const NB_ATTEMPTS: &[c_int] = &[0, 1, 2, 8, 256];
    const BEST_ML: &[c_int] = &[3 /* MINMATCH-1 */, 0, 4, 8, 64];
    // U32 offsets added to BOTH gDictEndIndex and ipIndex: only their
    // difference is semantically relevant, so this exercises the U32 wrap-around
    // arithmetic without leaving the safe index window.
    const GBASE: &[u32] = &[0, 65536, 1 << 20, 0xFFFF_0000, 0x1234_5678];
    let mut rng = Rng::new(0x4843_0051);
    let mut found = 0usize;
    let mut total = 0usize;

    for &dsize in &[8usize, 100, 1000, 20000, 65536, 70000] {
        for &shape in &[Shape::Text, Shape::Runs, Shape::Periodic(64)] {
            let dict = gen_buf(&mut rng, dsize, shape);
            let src = mix_from(&mut rng, &dict, 4000);
            for &lvl in LVLS {
                let st = Streams::new();
                st.reset(lvl);
                let loaded =
                    st.load_dict(&format!("searchExtDict dict lvl={}", lvl), &dict, dsize as c_int)
                        as u32;
                // LZ4HC_init_internal() places the dictionary at index 64 KB
                // (lz4hc.c:252-258), so lDictEndIndex == 64KB + loadedSize.
                let l_dict_end: u32 = 65536 + loaded;
                for iter in 0..250 {
                    let pos = rng.below(src.len() - 8);
                    let ip = unsafe { src.as_ptr().add(pos) };
                    let lo = rng.below(pos + 1);
                    let hi = rng.range(pos, src.len());
                    let ilow = unsafe { src.as_ptr().add(lo) };
                    let ihigh = unsafe { src.as_ptr().add(hi) };
                    let gbase = GBASE[rng.below(GBASE.len())];
                    // k == ipIndex - gDictEndIndex; keeping it small guarantees
                    // that only *real* dictionary indices satisfy the
                    // `ipIndex - matchIndex <= LZ4_DISTANCE_MAX` window.
                    let k = rng.below(4) as u32;
                    let g_dict_end = l_dict_end.wrapping_add(gbase);
                    let ip_index = g_dict_end.wrapping_add(k);
                    let best_ml = BEST_ML[rng.below(BEST_ML.len())];
                    let nb = NB_ATTEMPTS[rng.below(NB_ATTEMPTS.len())];
                    let ctx = format!(
                        "LZ4HC_searchExtDict dictSize={} shape={:?} level={} iter={} pos={} lo={} hi={} gbase={:#x} k={} bestML={} nbAttempts={}",
                        dsize, shape, lvl, iter, pos, lo, hi, gbase, k, best_ml, nb
                    );
                    let (cm, rm) = both(|l| unsafe {
                        l.get::<Fn_HC_searchExtDict>("LZ4HC_searchExtDict")(
                            ip,
                            ip_index,
                            ilow,
                            ihigh,
                            st.of(l) as *const c_void,
                            g_dict_end,
                            best_ml,
                            nb,
                        )
                    });
                    assert_eq!(cm, rm, "{}: C={:?} Rust={:?}", ctx, cm, rm);
                    total += 1;
                    if cm.off != 0 {
                        found += 1;
                    }
                }
            }
        }
    }
    // coverage: the search must actually hit dictionary matches, otherwise the
    // comparison above would be vacuous
    assert!(
        found * 20 > total,
        "LZ4HC_searchExtDict found only {} matches out of {} calls",
        found,
        total
    );
}

// ===========================================================================
// G. Deprecated HC family
// ===========================================================================

#[test]
fn deprecated_hc_stateless() {
    // LZ4_compressHC / _limitedOutput hard-code compressionLevel 0 (-> 9);
    // the HC2 variants take an explicit (possibly out-of-range) level.
    const LVLS: &[c_int] = &[0, -5, 1, 2, 9, 12, 13, 99];
    let mut rng = Rng::new(0x4843_0061);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 4, 13, 64, 255, 4096, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);

            let ctx = format!("LZ4_compressHC shape={:?} size={}", shape, size);
            let (crc, cv, rrc, rv) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compress>("LZ4_compressHC")(
                    data.as_ptr() as *const c_char,
                    dst,
                    size as c_int,
                )
            });
            assert_eq!(crc, rrc, "{}: rc C={} Rust={}", ctx, crc, rrc);
            assert_bytes_eq!(ctx, cv, rv);
            assert!(crc > 0);
            assert_roundtrip(&ctx, &cv, &data);

            for &cap in &[bound, (crc as usize).max(1) - 1, 1, 0] {
                let ctx = format!(
                    "LZ4_compressHC_limitedOutput shape={:?} size={} cap={}",
                    shape, size, cap
                );
                let (a, av, b, bv) = dual(cap, |l, dst| unsafe {
                    l.get::<Fn_compress_limitedOutput>("LZ4_compressHC_limitedOutput")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        cap as c_int,
                    )
                });
                assert_eq!(a, b, "{}: rc C={} Rust={}", ctx, a, b);
                assert_bytes_eq!(ctx, av, bv);
                if a > 0 {
                    assert_roundtrip(&ctx, &av, &data);
                }
            }

            for &lvl in LVLS {
                let ctx = format!(
                    "LZ4_compressHC2 shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (a, av, b, bv) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compressHC2>("LZ4_compressHC2")(
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        lvl,
                    )
                });
                assert_eq!(a, b, "{}: rc C={} Rust={}", ctx, a, b);
                assert_bytes_eq!(ctx, av, bv);
                assert!(a > 0, "{}: rc={}", ctx, a);
                assert_roundtrip(&ctx, &av, &data);

                for &cap in &[bound, (a as usize).max(1) - 1, 1, 0] {
                    let ctx = format!(
                        "LZ4_compressHC2_limitedOutput shape={:?} size={} level={} cap={}",
                        shape, size, lvl, cap
                    );
                    let (x, xv, y, yv) = dual(cap, |l, dst| unsafe {
                        l.get::<Fn_compressHC2_limitedOutput>("LZ4_compressHC2_limitedOutput")(
                            data.as_ptr() as *const c_char,
                            dst,
                            size as c_int,
                            cap as c_int,
                            lvl,
                        )
                    });
                    assert_eq!(x, y, "{}: rc C={} Rust={}", ctx, x, y);
                    assert_bytes_eq!(ctx, xv, yv);
                    if x > 0 {
                        assert_roundtrip(&ctx, &xv, &data);
                    }
                }
            }
        }
    }
}

#[test]
fn deprecated_hc_with_state() {
    const LVLS: &[c_int] = &[0, -5, 1, 2, 9, 12, 13, 99];
    let st = States::new();
    let mut rng = Rng::new(0x4843_0062);
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 13, 64, 4096, 70000] {
            let data = gen_buf(&mut rng, size, shape);
            let bound = cbound(size);

            let ctx = format!("LZ4_compressHC_withStateHC shape={:?} size={}", shape, size);
            let (a, av, b, bv) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compress_withState>("LZ4_compressHC_withStateHC")(
                    st.of(l),
                    data.as_ptr() as *const c_char,
                    dst,
                    size as c_int,
                )
            });
            assert_eq!(a, b, "{}: rc C={} Rust={}", ctx, a, b);
            assert_bytes_eq!(ctx, av, bv);
            st.assert_state(&ctx);
            assert!(a > 0);
            assert_roundtrip(&ctx, &av, &data);

            for &cap in &[bound, (a as usize).max(1) - 1, 1, 0] {
                let ctx = format!(
                    "LZ4_compressHC_limitedOutput_withStateHC shape={:?} size={} cap={}",
                    shape, size, cap
                );
                let (x, xv, y, yv) = dual(cap, |l, dst| unsafe {
                    l.get::<Fn_compress_limitedOutput_withState>(
                        "LZ4_compressHC_limitedOutput_withStateHC",
                    )(
                        st.of(l),
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        cap as c_int,
                    )
                });
                assert_eq!(x, y, "{}: rc C={} Rust={}", ctx, x, y);
                assert_bytes_eq!(ctx, xv, yv);
                st.assert_state(&ctx);
                if x > 0 {
                    assert_roundtrip(&ctx, &xv, &data);
                }
            }

            for &lvl in LVLS {
                let ctx = format!(
                    "LZ4_compressHC2_withStateHC shape={:?} size={} level={}",
                    shape, size, lvl
                );
                let (x, xv, y, yv) = dual(bound, |l, dst| unsafe {
                    l.get::<Fn_compressHC2_withStateHC>("LZ4_compressHC2_withStateHC")(
                        st.of(l),
                        data.as_ptr() as *const c_char,
                        dst,
                        size as c_int,
                        lvl,
                    )
                });
                assert_eq!(x, y, "{}: rc C={} Rust={}", ctx, x, y);
                assert_bytes_eq!(ctx, xv, yv);
                st.assert_state(&ctx);
                assert!(x > 0, "{}: rc={}", ctx, x);
                assert_roundtrip(&ctx, &xv, &data);

                for &cap in &[bound, (x as usize).max(1) - 1, 1, 0] {
                    let ctx = format!(
                        "LZ4_compressHC2_limitedOutput_withStateHC shape={:?} size={} level={} cap={}",
                        shape, size, lvl, cap
                    );
                    let (p, pv, q, qv) = dual(cap, |l, dst| unsafe {
                        l.get::<Fn_compressHC2_limitedOutput_withStateHC>(
                            "LZ4_compressHC2_limitedOutput_withStateHC",
                        )(
                            st.of(l),
                            data.as_ptr() as *const c_char,
                            dst,
                            size as c_int,
                            cap as c_int,
                            lvl,
                        )
                    });
                    assert_eq!(p, q, "{}: rc C={} Rust={}", ctx, p, q);
                    assert_bytes_eq!(ctx, pv, qv);
                    st.assert_state(&ctx);
                    if p > 0 {
                        assert_roundtrip(&ctx, &pv, &data);
                    }
                }
            }
        }
    }
}

#[test]
fn deprecated_hc_continue() {
    const LVLS: &[c_int] = &[1, 2, 3, 9, 12];
    let mut rng = Rng::new(0x4843_0063);
    for &shape in ALL_SHAPES {
        let data = gen_buf(&mut rng, 24000, shape);
        for &lvl in LVLS {
            let st = Streams::new();
            st.reset(lvl);
            let mut out = vec![0u8; data.len() + 64];
            let mut off = 0usize;
            let mut i = 0usize;
            while off < data.len() {
                let n = (data.len() - off).min(3000);
                let src = &data[off..off + n];
                let bound = cbound(n);
                let limited = i % 2 == 1;
                let ctx = format!(
                    "{} shape={:?} level={} chunk={}",
                    if limited {
                        "LZ4_compressHC_limitedOutput_continue"
                    } else {
                        "LZ4_compressHC_continue"
                    },
                    shape,
                    lvl,
                    i
                );
                let (a, av, b, bv) = if limited {
                    dual(bound, |l, dst| unsafe {
                        l.get::<Fn_compress_limitedOutput_continue>(
                            "LZ4_compressHC_limitedOutput_continue",
                        )(
                            st.of(l),
                            src.as_ptr() as *const c_char,
                            dst,
                            n as c_int,
                            bound as c_int,
                        )
                    })
                } else {
                    dual(bound, |l, dst| unsafe {
                        l.get::<Fn_compress_continue>("LZ4_compressHC_continue")(
                            st.of(l),
                            src.as_ptr() as *const c_char,
                            dst,
                            n as c_int,
                        )
                    })
                };
                assert_eq!(a, b, "{}: rc C={} Rust={}", ctx, a, b);
                assert_bytes_eq!(ctx, av, bv);
                st.assert_state(&ctx, false);
                assert!(a > 0, "{}: rc={}", ctx, a);
                for l in [c(), r()] {
                    let rc = unsafe {
                        l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                            av.as_ptr() as *const c_char,
                            out[off..].as_mut_ptr() as *mut c_char,
                            a,
                            n as c_int,
                            out.as_ptr() as *const c_char,
                            off as c_int,
                        )
                    };
                    assert_eq!(rc, n as c_int, "{}: {} decode rc={}", ctx, l.which, rc);
                }
                assert_bytes_eq!(
                    format!("{} [decoded]", ctx),
                    out[off..off + n],
                    data[off..off + n]
                );
                off += n;
                i += 1;
            }
        }
    }
}

/// Obsolete streaming API: `LZ4_createHC` / `LZ4_freeHC` /
/// `LZ4_slideInputBufferHC` / `LZ4_compressHC2*_continue` /
/// `LZ4_resetStreamStateHC`.  These require the payload to live inside the
/// buffer handed to `LZ4_createHC()`.
#[test]
fn deprecated_hc_obsolete_streaming() {
    const LVLS: &[c_int] = &[0, 1, 2, 3, 9, 12, 13];
    let mut rng = Rng::new(0x4843_0064);
    for &shape in ALL_SHAPES {
        let data = gen_buf(&mut rng, 20000, shape);
        for &lvl in LVLS {
            for &limited in &[false, true] {
                let (ch, rh) = both(|l| unsafe {
                    l.get::<Fn_createHC>("LZ4_createHC")(data.as_ptr() as *const c_char)
                });
                assert!(!ch.is_null() && !rh.is_null());
                let n = sizeof_state_hc();
                assert_state_eq(
                    &format!("LZ4_createHC shape={:?} level={}", shape, lvl),
                    ch,
                    rh,
                    n,
                    false,
                );
                let mut out = vec![0u8; data.len() + 64];
                let mut off = 0usize;
                let mut i = 0usize;
                while off < data.len() {
                    let sz = (data.len() - off).min(4000);
                    let src = &data[off..off + sz];
                    let bound = cbound(sz);
                    let ctx = format!(
                        "{} shape={:?} level={} chunk={}",
                        if limited {
                            "LZ4_compressHC2_limitedOutput_continue"
                        } else {
                            "LZ4_compressHC2_continue"
                        },
                        shape,
                        lvl,
                        i
                    );
                    let (a, av, b, bv) = if limited {
                        dual(bound, |l, dst| unsafe {
                            let h = if l.which == "C" { ch } else { rh };
                            l.get::<Fn_compressHC2_limitedOutput_continue>(
                                "LZ4_compressHC2_limitedOutput_continue",
                            )(
                                h,
                                src.as_ptr() as *const c_char,
                                dst,
                                sz as c_int,
                                bound as c_int,
                                lvl,
                            )
                        })
                    } else {
                        dual(bound, |l, dst| unsafe {
                            let h = if l.which == "C" { ch } else { rh };
                            l.get::<Fn_compressHC2_continue>("LZ4_compressHC2_continue")(
                                h,
                                src.as_ptr() as *const c_char,
                                dst,
                                sz as c_int,
                                lvl,
                            )
                        })
                    };
                    assert_eq!(a, b, "{}: rc C={} Rust={}", ctx, a, b);
                    assert_bytes_eq!(ctx, av, bv);
                    assert_state_eq(&ctx, ch, rh, n, false);
                    assert!(a > 0, "{}: rc={}", ctx, a);
                    for l in [c(), r()] {
                        let rc = unsafe {
                            l.get::<Fn_decompress_usingDict>("LZ4_decompress_safe_usingDict")(
                                av.as_ptr() as *const c_char,
                                out[off..].as_mut_ptr() as *mut c_char,
                                a,
                                sz as c_int,
                                out.as_ptr() as *const c_char,
                                off as c_int,
                            )
                        };
                        assert_eq!(rc, sz as c_int, "{}: {} decode rc={}", ctx, l.which, rc);
                    }
                    assert_bytes_eq!(
                        format!("{} [decoded]", ctx),
                        out[off..off + sz],
                        data[off..off + sz]
                    );
                    off += sz;
                    i += 1;
                }
                // LZ4_slideInputBufferHC returns prefixStart-dictLimit+lowLimit,
                // a pointer into the *shared* input buffer, so both libraries
                // must return the very same address.
                let (cp, rp) = both(|l| unsafe {
                    let h = if l.which == "C" { ch } else { rh };
                    l.get::<Fn_slideInputBufferHC>("LZ4_slideInputBufferHC")(h)
                });
                assert_eq!(
                    cp as usize as isize - data.as_ptr() as isize,
                    rp as usize as isize - data.as_ptr() as isize,
                    "LZ4_slideInputBufferHC shape={:?} level={}: C={:?} Rust={:?} base={:?}",
                    shape,
                    lvl,
                    cp,
                    rp,
                    data.as_ptr()
                );
                assert_state_eq(
                    &format!("after LZ4_slideInputBufferHC level={}", lvl),
                    ch,
                    rh,
                    n,
                    false,
                );
                let (cf, rf) = both(|l| unsafe {
                    let h = if l.which == "C" { ch } else { rh };
                    l.get::<Fn_freeHC>("LZ4_freeHC")(h)
                });
                assert_eq!(cf, 0, "C LZ4_freeHC returned {}", cf);
                assert_eq!(rf, 0, "Rust LZ4_freeHC returned {}", rf);
            }
        }
    }

    // LZ4_resetStreamStateHC on an external state
    let st = States::new();
    let data = gen_buf(&mut rng, 5000, Shape::Text);
    let (a, b) = both(|l| unsafe {
        l.get::<Fn_resetStreamStateHC>("LZ4_resetStreamStateHC")(
            st.of(l),
            data.as_ptr() as *mut c_char,
        )
    });
    assert_eq!(a, 0, "C LZ4_resetStreamStateHC returned {}", a);
    assert_eq!(b, 0, "Rust LZ4_resetStreamStateHC returned {}", b);
    st.assert_state("LZ4_resetStreamStateHC");
    let bound = cbound(data.len());
    let ctx = "LZ4_resetStreamStateHC + compressHC2_continue".to_string();
    let (x, xv, y, yv) = dual(bound, |l, dst| unsafe {
        l.get::<Fn_compressHC2_limitedOutput_continue>("LZ4_compressHC2_limitedOutput_continue")(
            st.of(l),
            data.as_ptr() as *const c_char,
            dst,
            data.len() as c_int,
            bound as c_int,
            9,
        )
    });
    assert_eq!(x, y, "{}: rc C={} Rust={}", ctx, x, y);
    assert_bytes_eq!(ctx, xv, yv);
    st.assert_state(&ctx);
    assert!(x > 0);
    assert_roundtrip(&ctx, &xv, &data);
}

// ===========================================================================
// H. Error paths (ERRORS.md rows 49-70)
// ===========================================================================

#[test]
fn hc_error_paths_init_and_free() {
    let n = sizeof_state_hc();
    let buf_c = Aligned::new(n + 16, 8);
    let buf_r = Aligned::new(n + 16, 8);

    // row 66: buffer == NULL
    let (a, b) = both(|l| unsafe {
        l.get::<Fn_initStreamHC>("LZ4_initStreamHC")(std::ptr::null_mut(), n)
    });
    assert!(a.is_null(), "C LZ4_initStreamHC(NULL, {}) = {:?}", n, a);
    assert!(b.is_null(), "Rust LZ4_initStreamHC(NULL, {}) = {:?}", n, b);

    // row 67: size < sizeof(LZ4_streamHC_t)
    for &sz in &[0usize, 1, n - 1] {
        let (a, b) = both(|l| unsafe {
            let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
            l.get::<Fn_initStreamHC>("LZ4_initStreamHC")(p, sz)
        });
        assert!(a.is_null(), "C LZ4_initStreamHC(buf, {}) = {:?}", sz, a);
        assert!(b.is_null(), "Rust LZ4_initStreamHC(buf, {}) = {:?}", sz, b);
    }

    // row 68: misaligned buffer (LZ4_streamHC_t_alignment() == 8 here)
    let (a, b) = both(|l| unsafe {
        let p = if l.which == "C" {
            buf_c.misaligned()
        } else {
            buf_r.misaligned()
        };
        l.get::<Fn_initStreamHC>("LZ4_initStreamHC")(p, n)
    });
    assert!(a.is_null(), "C LZ4_initStreamHC(misaligned) = {:?}", a);
    assert!(b.is_null(), "Rust LZ4_initStreamHC(misaligned) = {:?}", b);

    // success cases
    for &sz in &[n, n + 16] {
        let (a, b) = both(|l| unsafe {
            let p = if l.which == "C" { buf_c.ptr() } else { buf_r.ptr() };
            l.get::<Fn_initStreamHC>("LZ4_initStreamHC")(p, sz)
        });
        assert_eq!(a, buf_c.ptr(), "C LZ4_initStreamHC(buf, {})", sz);
        assert_eq!(b, buf_r.ptr(), "Rust LZ4_initStreamHC(buf, {})", sz);
        assert_bytes_eq!(
            format!("LZ4_initStreamHC(buf,{}) [state]", sz),
            buf_c.bytes(),
            buf_r.bytes()
        );
    }

    // row 65 / 70: free(NULL)
    let (a, b) =
        both(|l| unsafe { l.get::<Fn_freeStream>("LZ4_freeStreamHC")(std::ptr::null_mut()) });
    assert_eq!(a, 0, "C LZ4_freeStreamHC(NULL) = {}", a);
    assert_eq!(b, 0, "Rust LZ4_freeStreamHC(NULL) = {}", b);
    let (a, b) = both(|l| unsafe { l.get::<Fn_freeHC>("LZ4_freeHC")(std::ptr::null_mut()) });
    assert_eq!(a, 0, "C LZ4_freeHC(NULL) = {}", a);
    assert_eq!(b, 0, "Rust LZ4_freeHC(NULL) = {}", b);

    // row 69: LZ4_resetStreamStateHC failure -> nonzero
    let dummy = vec![0u8; 64];
    let (a, b) = both(|l| unsafe {
        l.get::<Fn_resetStreamStateHC>("LZ4_resetStreamStateHC")(
            std::ptr::null_mut(),
            dummy.as_ptr() as *mut c_char,
        )
    });
    assert_eq!(a, 1, "C LZ4_resetStreamStateHC(NULL, buf) = {}", a);
    assert_eq!(b, 1, "Rust LZ4_resetStreamStateHC(NULL, buf) = {}", b);
    let (a, b) = both(|l| unsafe {
        let p = if l.which == "C" {
            buf_c.misaligned()
        } else {
            buf_r.misaligned()
        };
        l.get::<Fn_resetStreamStateHC>("LZ4_resetStreamStateHC")(p, dummy.as_ptr() as *mut c_char)
    });
    assert_eq!(a, 1, "C LZ4_resetStreamStateHC(misaligned) = {}", a);
    assert_eq!(b, 1, "Rust LZ4_resetStreamStateHC(misaligned) = {}", b);
}

#[test]
fn hc_error_paths_ext_state() {
    let n = sizeof_state_hc();
    let mut rng = Rng::new(0x4843_0071);
    let data = gen_buf(&mut rng, 500, Shape::Text);
    let bound = cbound(data.len());
    let buf_c = Aligned::new(n + 16, 8);
    let buf_r = Aligned::new(n + 16, 8);

    // row 62: NULL state -> 0 (LZ4_initStreamHC fails)
    let (a, _, b, _) = dual(bound, |l, dst| unsafe {
        l.get::<Fn_compress_HC_extStateHC>("LZ4_compress_HC_extStateHC")(
            std::ptr::null_mut(),
            data.as_ptr() as *const c_char,
            dst,
            data.len() as c_int,
            bound as c_int,
            9,
        )
    });
    assert_eq!(a, 0, "C LZ4_compress_HC_extStateHC(NULL) = {}", a);
    assert_eq!(b, 0, "Rust LZ4_compress_HC_extStateHC(NULL) = {}", b);

    // row 62 / 61: misaligned state -> 0, for both entry points.
    // NOTE: `LZ4_compress_HC_extStateHC_fastReset(NULL, ...)` is *not* tested:
    // NULL is 8-byte "aligned", so the C dereferences it (lz4hc.c:1503-1504)
    // and crashes; that is documented UB, not an error path.
    for name in [
        "LZ4_compress_HC_extStateHC",
        "LZ4_compress_HC_extStateHC_fastReset",
    ] {
        let (a, _, b, _) = dual(bound, |l, dst| unsafe {
            let p = if l.which == "C" {
                buf_c.misaligned()
            } else {
                buf_r.misaligned()
            };
            l.get::<Fn_compress_HC_extStateHC>(name)(
                p,
                data.as_ptr() as *const c_char,
                dst,
                data.len() as c_int,
                bound as c_int,
                9,
            )
        });
        assert_eq!(a, 0, "C {}(misaligned) = {}", name, a);
        assert_eq!(b, 0, "Rust {}(misaligned) = {}", name, b);
    }

    // row 63: LZ4_compress_HC_destSize with a NULL / misaligned state -> 0
    for which in 0..2 {
        let mut cdst = vec![0u8; bound + 64];
        let mut rdst = vec![0u8; bound + 64];
        let mut css: c_int = data.len() as c_int;
        let mut rss: c_int = data.len() as c_int;
        let (cp, rp) = if which == 0 {
            (std::ptr::null_mut(), std::ptr::null_mut())
        } else {
            (buf_c.misaligned(), buf_r.misaligned())
        };
        let a = unsafe {
            c().get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                cp,
                data.as_ptr() as *const c_char,
                cdst.as_mut_ptr() as *mut c_char,
                &mut css,
                bound as c_int,
                9,
            )
        };
        let b = unsafe {
            r().get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                rp,
                data.as_ptr() as *const c_char,
                rdst.as_mut_ptr() as *mut c_char,
                &mut rss,
                bound as c_int,
                9,
            )
        };
        assert_eq!(a, 0, "C LZ4_compress_HC_destSize(bad state {}) = {}", which, a);
        assert_eq!(
            b, 0,
            "Rust LZ4_compress_HC_destSize(bad state {}) = {}",
            which, b
        );
        assert_eq!(
            css, rss,
            "LZ4_compress_HC_destSize(bad state {}) *srcSizePtr C={} Rust={}",
            which, css, rss
        );
    }

    // row 59: fillOutput with dstCapacity < 1 -> 0, *srcSizePtr untouched
    let st = States::new();
    let (a, css, _, b, rss, _) = dual_ss(0, data.len() as c_int, |l, dst, ssp| unsafe {
        l.get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
            st.of(l),
            data.as_ptr() as *const c_char,
            dst,
            ssp,
            0,
            9,
        )
    });
    assert_eq!(a, 0, "C LZ4_compress_HC_destSize(target=0) = {}", a);
    assert_eq!(b, 0, "Rust LZ4_compress_HC_destSize(target=0) = {}", b);
    assert_eq!(
        css, rss,
        "LZ4_compress_HC_destSize(target=0) *srcSizePtr C={} Rust={}",
        css, rss
    );
    let s2 = Streams::new();
    s2.reset(9);
    let (a, css, _, b, rss, _) = dual_ss(0, data.len() as c_int, |l, dst, ssp| unsafe {
        l.get::<Fn_compress_HC_continue_destSize>("LZ4_compress_HC_continue_destSize")(
            s2.of(l),
            data.as_ptr() as *const c_char,
            dst,
            ssp,
            0,
        )
    });
    assert_eq!(a, 0, "C LZ4_compress_HC_continue_destSize(target=0) = {}", a);
    assert_eq!(
        b, 0,
        "Rust LZ4_compress_HC_continue_destSize(target=0) = {}",
        b
    );
    assert_eq!(css, rss, "*srcSizePtr C={} Rust={}", css, rss);
    s2.assert_state("continue_destSize(target=0)", false);
}

#[test]
fn hc_error_paths_srcsize_range() {
    // rows 51/53/60: srcSize negative or > LZ4_MAX_INPUT_SIZE must be rejected
    // *before* any memory access — verified against lz4hc.c:1389 (which runs
    // before `ctx->end += *srcSizePtr`) and lz4hc.c:559-563.  A deliberately
    // tiny real buffer is passed with a huge/negative srcSize.
    let small = vec![0x42u8; 64];
    let bound = cbound(4096);
    let bad: &[c_int] = &[
        -1,
        -64,
        c_int::MIN,
        LZ4_MAX_INPUT_SIZE + 1,
        0x7F00_0000,
        c_int::MAX,
    ];
    let st = States::new();
    for &lvl in BAND_LEVELS {
        for &sz in bad {
            let ctx = format!("srcSize={} level={}", sz, lvl);
            let (a, _, b, _) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                    small.as_ptr() as *const c_char,
                    dst,
                    sz,
                    bound as c_int,
                    lvl,
                )
            });
            assert_eq!(a, 0, "C LZ4_compress_HC {} = {}", ctx, a);
            assert_eq!(b, 0, "Rust LZ4_compress_HC {} = {}", ctx, b);

            let (a, _, b, _) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compress_HC_extStateHC>("LZ4_compress_HC_extStateHC")(
                    st.of(l),
                    small.as_ptr() as *const c_char,
                    dst,
                    sz,
                    bound as c_int,
                    lvl,
                )
            });
            assert_eq!(a, 0, "C LZ4_compress_HC_extStateHC {} = {}", ctx, a);
            assert_eq!(b, 0, "Rust LZ4_compress_HC_extStateHC {} = {}", ctx, b);

            let (a, css, _, b, rss, _) = dual_ss(bound, sz, |l, dst, ssp| unsafe {
                l.get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                    st.of(l),
                    small.as_ptr() as *const c_char,
                    dst,
                    ssp,
                    bound as c_int,
                    lvl,
                )
            });
            assert_eq!(a, 0, "C LZ4_compress_HC_destSize {} = {}", ctx, a);
            assert_eq!(b, 0, "Rust LZ4_compress_HC_destSize {} = {}", ctx, b);
            assert_eq!(css, rss, "destSize {} *srcSizePtr C={} Rust={}", ctx, css, rss);

            let (a, _, b, _) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compressHC2>("LZ4_compressHC2")(
                    small.as_ptr() as *const c_char,
                    dst,
                    sz,
                    lvl,
                )
            });
            assert_eq!(a, 0, "C LZ4_compressHC2 {} = {}", ctx, a);
            assert_eq!(b, 0, "Rust LZ4_compressHC2 {} = {}", ctx, b);

            let s = Streams::new();
            s.reset(lvl);
            let (a, _, b, _) = dual(bound, |l, dst| unsafe {
                l.get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                    s.of(l),
                    small.as_ptr() as *const c_char,
                    dst,
                    sz,
                    bound as c_int,
                )
            });
            assert_eq!(a, 0, "C LZ4_compress_HC_continue {} = {}", ctx, a);
            assert_eq!(b, 0, "Rust LZ4_compress_HC_continue {} = {}", ctx, b);
        }
    }
}

#[test]
fn hc_error_paths_load_dict() {
    let mut rng = Rng::new(0x4843_0072);
    let dict = gen_buf(&mut rng, 1000, Shape::Text);
    let data = gen_buf(&mut rng, 2000, Shape::Text);
    let bound = cbound(data.len());
    for &lvl in BAND_LEVELS {
        // dictSize == 0, with a real pointer and with NULL.
        // (negative dictSize is excluded: lz4hc.c:1632 asserts dictSize >= 0
        //  and the C library is built with assertions enabled)
        for null_dict in [false, true] {
            let st = Streams::new();
            st.reset(lvl);
            let (a, b) = st.call(|l, s| unsafe {
                let p = if null_dict {
                    std::ptr::null()
                } else {
                    dict.as_ptr() as *const c_char
                };
                l.get::<Fn_loadDictHC>("LZ4_loadDictHC")(s, p, 0)
            });
            assert_eq!(
                a, 0,
                "C LZ4_loadDictHC(dictSize=0, null={}) = {}",
                null_dict, a
            );
            assert_eq!(
                b, 0,
                "Rust LZ4_loadDictHC(dictSize=0, null={}) = {}",
                null_dict, b
            );
            let ctx = format!("loadDictHC(0, null={}) level={}", null_dict, lvl);
            st.assert_state(&ctx, false);
            let (crc, cv) = st.continue_block(&ctx, &data, bound);
            assert!(crc > 0, "{}: rc={}", ctx, crc);
            st.assert_state(&ctx, false);
            assert_roundtrip(&ctx, &cv, &data);
        }
    }
}

// ===========================================================================
// Randomised sweeps (fixed seeds)
// ===========================================================================

/// Random `(shape, size, level, dstCapacity)` combinations through
/// `LZ4_compress_HC` and `LZ4_compress_HC_destSize`.
#[test]
fn compress_hc_random_fuzz() {
    let st = States::new();
    let mut rng = Rng::new(0x4843_0081);
    for i in 0..1200 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let size = if rng.below(12) == 0 {
            rng.range(0, 120000)
        } else {
            rng.range(0, 3000)
        };
        let data = gen_buf(&mut rng, size, shape);
        let bound = cbound(size);
        let lvl = if rng.bool() {
            LEVELS[rng.below(LEVELS.len())]
        } else {
            rng.range(0, 40) as c_int - 20
        };
        let cap = match rng.below(5) {
            0 => bound,
            1 => rng.range(0, bound),
            2 => size,
            3 => rng.below(64),
            _ => bound / 2,
        };
        let ctx = format!(
            "fuzz iter={} shape={:?} size={} level={} cap={}",
            i, shape, size, lvl, cap
        );
        let (crc, cv, rrc, rv) = dual(cap, |l, dst| unsafe {
            l.get::<Fn_compress_HC>("LZ4_compress_HC")(
                data.as_ptr() as *const c_char,
                dst,
                size as c_int,
                cap as c_int,
                lvl,
            )
        });
        assert_eq!(crc, rrc, "{}: LZ4_compress_HC rc C={} Rust={}", ctx, crc, rrc);
        assert_bytes_eq!(ctx, cv, rv);
        if crc > 0 {
            assert_roundtrip(&ctx, &cv, &data);
        }

        let (drc, css, dcv, drrc, rss, drv) = dual_ss(cap, size as c_int, |l, dst, ssp| unsafe {
            l.get::<Fn_compress_HC_destSize>("LZ4_compress_HC_destSize")(
                st.of(l),
                data.as_ptr() as *const c_char,
                dst,
                ssp,
                cap as c_int,
                lvl,
            )
        });
        let ctx2 = format!("{} [destSize]", ctx);
        assert_eq!(drc, drrc, "{}: rc C={} Rust={}", ctx2, drc, drrc);
        assert_eq!(css, rss, "{}: *srcSizePtr C={} Rust={}", ctx2, css, rss);
        assert_bytes_eq!(ctx2, dcv, drv);
        st.assert_state(&ctx2);
        if drc > 0 {
            assert_roundtrip(&ctx2, &dcv, &data[..css as usize]);
        }
    }
}

/// Random sequences of streaming operations on one `LZ4_streamHC_t`:
/// `LZ4_compress_HC_continue`, `LZ4_compress_HC_continue_destSize`,
/// `LZ4_saveDictHC`, `LZ4_setCompressionLevel`, `LZ4_favorDecompressionSpeed`,
/// `LZ4_resetStreamHC`, `LZ4_resetStreamHC_fast`, `LZ4_loadDictHC` and
/// `LZ4_attach_HC_dictionary`.  Return codes, produced bytes and the whole
/// `LZ4_streamHC_t` (minus the `dictCtx` pointer) must stay in lock-step.
#[test]
fn stream_hc_random_operation_fuzz() {
    const SAVE_SIZES: &[usize] = &[0, 1, 3, 4, 100, 4096, 65536, 100000];
    let mut rng = Rng::new(0x4843_0082);
    let bufs: Vec<Vec<u8>> = vec![
        gen_buf(&mut rng, 50000, Shape::Text),
        gen_buf(&mut rng, 50000, Shape::Runs),
        gen_buf(&mut rng, 50000, Shape::Incompressible),
        gen_buf(&mut rng, 50000, Shape::Periodic(3)),
    ];
    let dicts: Vec<Vec<u8>> = vec![
        gen_buf(&mut rng, 0, Shape::Text),
        gen_buf(&mut rng, 3, Shape::Text),
        gen_buf(&mut rng, 300, Shape::Text),
        gen_buf(&mut rng, 70000, Shape::Runs),
    ];
    let mut safe = vec![0x5Au8; 100000 + 64];

    let dctx = Streams::new();
    dctx.reset(9);
    dctx.load_dict("fuzz dictCtx", &dicts[3], dicts[3].len() as c_int);
    let dctx_mid = Streams::new();
    dctx_mid.reset(2);
    dctx_mid.load_dict("fuzz dictCtx (mid)", &dicts[2], dicts[2].len() as c_int);

    let st = Streams::new();
    st.reset(9);
    // `initialized` tracks whether the index space has been set up by
    // LZ4HC_init_internal (i.e. dictLimit >= 64 KB).  Saving a dictionary from a
    // *virgin* stream would leave dictLimit == 0 and the next compression would
    // then index from 0, which trips `assert(matchIndex < ipIndex)` in the
    // assertion-enabled C build (documented misuse, not an error path).
    let mut initialized = false;
    let mut bi = 0usize;
    let mut off = 0usize;

    for i in 0..700 {
        let op = rng.below(12);
        let tag = format!("stream fuzz iter={} op={}", i, op);
        match op {
            0..=5 => {
                if off >= bufs[bi].len() {
                    bi = (bi + 1) % bufs.len();
                    off = 0;
                }
                let remain = bufs[bi].len() - off;
                let len = rng.range(0, remain.min(4000));
                let bound = cbound(len);
                let cap = if rng.below(4) == 0 {
                    rng.range(0, bound)
                } else {
                    bound
                };
                let src = &bufs[bi][off..off + len];
                let (crc, cv, rrc, rv) = dual(cap, |l, dst| unsafe {
                    l.get::<Fn_compress_HC_continue>("LZ4_compress_HC_continue")(
                        st.of(l),
                        src.as_ptr() as *const c_char,
                        dst,
                        len as c_int,
                        cap as c_int,
                    )
                });
                assert_eq!(
                    crc, rrc,
                    "{}: LZ4_compress_HC_continue(len={},cap={}) rc C={} Rust={}",
                    tag, len, cap, crc, rrc
                );
                assert_bytes_eq!(tag, cv, rv);
                st.assert_state(&tag, true);
                if crc > 0 {
                    initialized = true;
                    off += len;
                } else {
                    // the API requires a reset after a failed compression
                    st.reset(9);
                    initialized = false;
                }
            }
            6 => {
                if off >= bufs[bi].len() {
                    bi = (bi + 1) % bufs.len();
                    off = 0;
                }
                let remain = bufs[bi].len() - off;
                let len = rng.range(0, remain.min(6000));
                let target = match rng.below(4) {
                    0 => 0,
                    1 => rng.range(0, 32),
                    2 => rng.range(0, 400),
                    _ => rng.range(0, cbound(len)),
                };
                let src = &bufs[bi][off..off + len];
                let (crc, css, cv, rrc, rss, rv) =
                    dual_ss(target, len as c_int, |l, dst, ssp| unsafe {
                        l.get::<Fn_compress_HC_continue_destSize>(
                            "LZ4_compress_HC_continue_destSize",
                        )(
                            st.of(l), src.as_ptr() as *const c_char, dst, ssp, target as c_int
                        )
                    });
                assert_eq!(
                    crc, rrc,
                    "{}: continue_destSize(len={},target={}) rc C={} Rust={}",
                    tag, len, target, crc, rrc
                );
                assert_eq!(css, rss, "{}: *srcSizePtr C={} Rust={}", tag, css, rss);
                assert_bytes_eq!(tag, cv, rv);
                st.assert_state(&tag, true);
                if crc > 0 {
                    initialized = true;
                    off += css as usize;
                } else {
                    st.reset(9);
                    initialized = false;
                }
            }
            7 => {
                if !initialized {
                    continue;
                }
                let want = SAVE_SIZES[rng.below(SAVE_SIZES.len())];
                // both libraries must see the *same* safeBuffer content, so the
                // buffer is restored between the two calls (a previous save may
                // have made safeBuffer the stream's own prefix).
                let before = safe.clone();
                let crc = unsafe {
                    c().get::<Fn_saveDictHC>("LZ4_saveDictHC")(
                        st.cs,
                        safe.as_mut_ptr() as *mut c_char,
                        want as c_int,
                    )
                };
                let c_saved = safe.clone();
                safe.copy_from_slice(&before);
                let rrc = unsafe {
                    r().get::<Fn_saveDictHC>("LZ4_saveDictHC")(
                        st.rs,
                        safe.as_mut_ptr() as *mut c_char,
                        want as c_int,
                    )
                };
                assert_eq!(
                    crc, rrc,
                    "{}: LZ4_saveDictHC({}) rc C={} Rust={}",
                    tag, want, crc, rrc
                );
                assert_bytes_eq!(format!("{} [safeBuffer]", tag), c_saved, safe);
                st.assert_state(&tag, true);
            }
            8 => {
                let lvl = LEVELS[rng.below(LEVELS.len())];
                st.set_level(lvl);
                st.favor(rng.below(2) as c_int);
                st.assert_state(&format!("{} setCompressionLevel({})", tag, lvl), true);
            }
            9 => {
                let lvl = LEVELS[rng.below(LEVELS.len())];
                if rng.bool() {
                    st.reset(lvl);
                } else {
                    st.reset_fast(lvl);
                }
                st.assert_state(&format!("{} reset({})", tag, lvl), true);
                initialized = false;
                bi = rng.below(bufs.len());
                off = 0;
            }
            10 => {
                let d = &dicts[rng.below(dicts.len())];
                let n = if rng.below(4) == 0 {
                    rng.range(0, d.len())
                } else {
                    d.len()
                };
                let got = st.load_dict(&tag, d, n as c_int);
                assert_eq!(
                    got as usize,
                    n.min(65536),
                    "{}: LZ4_loadDictHC({}) = {}",
                    tag,
                    n,
                    got
                );
                st.assert_state(&tag, true);
                initialized = true;
                bi = rng.below(bufs.len());
                off = 0;
            }
            _ => {
                // a dictionary may only be attached to a history-less stream
                if initialized {
                    continue;
                }
                let which = rng.below(3);
                st.call(|l, s| unsafe {
                    let d = match which {
                        0 => std::ptr::null(),
                        1 => {
                            if l.which == "C" {
                                dctx.cs as *const c_void
                            } else {
                                dctx.rs as *const c_void
                            }
                        }
                        _ => {
                            if l.which == "C" {
                                dctx_mid.cs as *const c_void
                            } else {
                                dctx_mid.rs as *const c_void
                            }
                        }
                    };
                    l.get::<Fn_attach_dictionary>("LZ4_attach_HC_dictionary")(s, d);
                });
                st.assert_state(&format!("{} attach({})", tag, which), true);
            }
        }
    }
}
