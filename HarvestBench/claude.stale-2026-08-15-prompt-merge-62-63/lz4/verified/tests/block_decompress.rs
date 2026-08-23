//! CONFIGS.md rows 48-69 — lz4.c block-DECOMPRESSION valid-path parity.
//!
//! Every payload fed to a decoder here is produced by the **C** library's own
//! compressor (`LZ4_compress_default` / `_fast` / `LZ4_compress_HC` /
//! `LZ4_compress_fast_continue`), so the inputs are always valid LZ4 blocks.
//! Each decoder entry point is then driven through BOTH the C `.so` and the
//! Rust `.so` and the return code AND the complete destination buffer are
//! compared byte-for-byte.
//!
//! NOTE on lifetimes: `LZ4_stream_t` / `LZ4_streamDecode_t` retain raw pointers
//! into caller buffers. Every buffer whose address is retained by a stream is
//! allocated *before* the loop that uses it and kept alive until the stream is
//! done with it.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

type Sym<T> = libloading::Symbol<'static, T>;

// ---------------------------------------------------------------------------
// C signatures (verified against c_src/include/lz4.h + c_src/src/lz4.c)
// ---------------------------------------------------------------------------
type FnBound = unsafe extern "C" fn(c_int) -> c_int;
type FnCompDefault = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnCompFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCompHC = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type FnCreateStream = unsafe extern "C" fn() -> *mut c_void;
type FnFreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnCompContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;

/// `int LZ4_decompress_safe(const char*, char*, int, int)`
/// (also `LZ4_uncompress_unknownOutputSize`, `_safe_withPrefix64k`)
type FnDecSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
/// `int LZ4_decompress_safe_partial(const char*, char*, int, int, int)`
type FnDecPartial = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
/// `int LZ4_decompress_fast(const char*, char*, int)`
/// (also `LZ4_uncompress`, `_fast_withPrefix64k`)
type FnDecFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
/// `int LZ4_decompress_safe_usingDict(const char*, char*, int, int, const char*, int)`
type FnDecUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_char, c_int) -> c_int;
/// `int LZ4_decompress_safe_partial_usingDict(const char*, char*, int, int, int, const char*, int)`
type FnDecPartialUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
/// `int LZ4_decompress_fast_usingDict(const char*, char*, int, const char*, int)`
type FnDecFastUsingDict =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
/// `int LZ4_decompress_safe_forceExtDict(const char*, char*, int, int, const void*, size_t)`
type FnDecForceExt =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_void, usize) -> c_int;
/// `int LZ4_decompress_safe_partial_forceExtDict(const char*, char*, int, int, int, const void*, size_t)`
type FnDecPartialForceExt = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    c_int,
    *const c_void,
    usize,
) -> c_int;
type FnCreateSD = unsafe extern "C" fn() -> *mut c_void;
type FnFreeSD = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnSetSD = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type FnDecSafeContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type FnDecFastContinue = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Payload production — always the C implementation
// ---------------------------------------------------------------------------

/// A compressed block: a padded buffer plus the exact compressed size.
struct Comp {
    buf: Vec<u8>,
    n: usize,
}

impl Comp {
    fn ptr(&self) -> *const c_char {
        self.buf.as_ptr() as *const c_char
    }
    fn n(&self) -> c_int {
        self.n as c_int
    }
}

/// The C-side compressors, used only to build valid payloads.
struct Enc {
    def: Sym<FnCompDefault>,
    fast: Sym<FnCompFast>,
    hc: Sym<FnCompHC>,
    bnd: Sym<FnBound>,
    cs: Sym<FnCreateStream>,
    fsy: Sym<FnFreeStream>,
    ld: Sym<FnLoadDict>,
    cont: Sym<FnCompContinue>,
}

/// Number of distinct encoders (different sequence shapes => different decoder
/// branches: short/long literal lengths, short/long match lengths, offsets).
const NENC: usize = 4;

impl Enc {
    fn new() -> Self {
        Enc {
            def: common::pair::<FnCompDefault>("LZ4_compress_default").0,
            fast: common::pair::<FnCompFast>("LZ4_compress_fast").0,
            hc: common::pair::<FnCompHC>("LZ4_compress_HC").0,
            bnd: common::pair::<FnBound>("LZ4_compressBound").0,
            cs: common::pair::<FnCreateStream>("LZ4_createStream").0,
            fsy: common::pair::<FnFreeStream>("LZ4_freeStream").0,
            ld: common::pair::<FnLoadDict>("LZ4_loadDict").0,
            cont: common::pair::<FnCompContinue>("LZ4_compress_fast_continue").0,
        }
    }

    fn bound(&self, n: usize) -> usize {
        unsafe { (self.bnd)(n as c_int) }.max(1) as usize
    }

    /// One-shot compression of `src` with encoder variant `v` (0..NENC).
    fn compress(&self, src: &[u8], v: usize) -> Comp {
        let cap = self.bound(src.len());
        let mut buf = vec![0u8; cap + 64];
        let s = src.as_ptr() as *const c_char;
        let d = buf.as_mut_ptr() as *mut c_char;
        let n = unsafe {
            match v % NENC {
                0 => (self.def)(s, d, src.len() as c_int, cap as c_int),
                1 => (self.fast)(s, d, src.len() as c_int, cap as c_int, 7),
                2 => (self.hc)(s, d, src.len() as c_int, cap as c_int, 9),
                _ => (self.hc)(s, d, src.len() as c_int, cap as c_int, 12),
            }
        };
        assert!(n > 0, "C compressor v={v} failed on {} bytes", src.len());
        Comp { buf, n: n as usize }
    }

    /// Compress `block` as the continuation of `dict` (prefix mode), i.e. the
    /// exact counterpart of `LZ4_decompress_safe_usingDict(.., dict, dictSize)`.
    /// `dict` and `block` MUST be adjacent slices of one buffer.
    fn compress_after_dict(&self, dict: &[u8], block: &[u8]) -> Comp {
        let cap = self.bound(block.len());
        let mut buf = vec![0u8; cap + 64];
        let n = unsafe {
            let s = (self.cs)();
            assert!(!s.is_null());
            (self.ld)(s, dict.as_ptr() as *const c_char, dict.len() as c_int);
            let r = (self.cont)(
                s,
                block.as_ptr() as *const c_char,
                buf.as_mut_ptr() as *mut c_char,
                block.len() as c_int,
                cap as c_int,
                1,
            );
            (self.fsy)(s);
            r
        };
        assert!(n > 0, "compress_after_dict failed");
        Comp { buf, n: n as usize }
    }

    /// Compress a contiguous buffer as a chain of linked blocks.
    fn compress_chain(&self, data: &[u8], sizes: &[usize], acc: c_int) -> Vec<Comp> {
        let mut out = Vec::with_capacity(sizes.len());
        unsafe {
            let s = (self.cs)();
            assert!(!s.is_null());
            let mut off = 0usize;
            for &n in sizes {
                let cap = self.bound(n);
                let mut buf = vec![0u8; cap + 64];
                let r = (self.cont)(
                    s,
                    data[off..].as_ptr() as *const c_char,
                    buf.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                    acc,
                );
                assert!(r > 0, "compress_fast_continue failed (n={n})");
                out.push(Comp { buf, n: r as usize });
                off += n;
            }
            (self.fsy)(s);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Compare a decoder result: return code AND the whole destination buffer.
#[track_caller]
fn chk(cn: c_int, cbuf: &[u8], rn: c_int, rbuf: &[u8], ctx: &str) {
    assert_ret_eq(cn, rn, ctx);
    assert_bytes_eq(cbuf, rbuf, ctx);
}

// ---------------------------------------------------------------------------
// Data shaping helpers
// ---------------------------------------------------------------------------

/// Like `gen_data`, but the result always owns a real allocation with at least
/// 64 readable zero bytes past `len`. Needed because `Vec` yields a DANGLING
/// pointer for a zero-length allocation while the C compressors (e.g.
/// `LZ4_compress_HC` at level 12) dereference `src` even when `srcSize == 0`.
fn gen_src(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_data(shape, len, rng);
    if v.capacity() < len + 64 {
        v.reserve(len + 64 - v.capacity());
    }
    unsafe {
        std::ptr::write_bytes(v.as_mut_ptr().add(len), 0, v.capacity() - len);
    }
    assert_eq!(v.len(), len);
    v
}

/// Overwrite parts of `whole[from..]` with copies of earlier bytes (back
/// distance <= LZ4_DISTANCE_MAX) so the compressed tail really does contain
/// matches that reach back into `whole[..from]` (the dictionary/prefix).
fn splice_backrefs(whole: &mut [u8], from: usize, rng: &mut Rng) {
    let n = whole.len();
    let mut i = from;
    while i < n {
        if i > 8 && rng.below(2) == 0 {
            let maxback = i.min(65535);
            let back = rng.range(4, maxback);
            let cl = rng.range(4, 400).min(n - i).min(back);
            for k in 0..cl {
                whole[i + k] = whole[i - back + k];
            }
            i += cl;
        } else {
            i += rng.range(1, 100);
        }
    }
}

/// `dict || block` where the block references the dict region.
fn dict_case(shape: Shape, dsz: usize, n: usize, rng: &mut Rng) -> Vec<u8> {
    let mut whole = gen_src(shape, dsz + n, rng);
    splice_backrefs(&mut whole, dsz, rng);
    whole
}

/// A block sharing content with `prev`, so a linked (extDict) chain of
/// separately-allocated blocks still produces cross-block matches.
fn derive_block(prev: &[u8], shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = gen_src(shape, len, rng);
    if prev.is_empty() || len == 0 {
        return v;
    }
    let mut i = 0usize;
    while i < len {
        if rng.below(2) == 0 {
            let cl = rng.range(4, 300).min(len - i).min(prev.len());
            let q = rng.below(prev.len() - cl + 1);
            v[i..i + cl].copy_from_slice(&prev[q..q + cl]);
            i += cl;
        } else {
            i += rng.range(1, 200);
        }
    }
    v
}

/// ~12 target sizes spanning `0..=decoded+1`.
fn targets_for(decoded: usize) -> Vec<usize> {
    let mut t = vec![0usize, 1, decoded / 8, decoded / 4, decoded / 3, decoded / 2];
    t.push(decoded * 3 / 4);
    if decoded >= 1 {
        t.push(decoded - 1);
    }
    if decoded >= 13 {
        t.push(decoded - 13);
    }
    t.push(decoded);
    t.push(decoded + 1);
    t.push(decoded + 64);
    t.sort_unstable();
    t.dedup();
    t
}

// ===========================================================================
// Row 48 — LZ4_decompress_safe, exact dstCapacity, KEY_LENS x ALL_SHAPES
// ===========================================================================
#[test]
fn row48_decompress_safe_exact_capacity() {
    let enc = Enc::new();
    sym!(ds, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xD0_0048);

    for &len in KEY_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                let mut cb = vec![0u8; len + 64];
                let mut rb = vec![0u8; len + 64];
                let (cn, rn) = unsafe {
                    (
                        ds.0(
                            comp.ptr(),
                            cb.as_mut_ptr() as *mut c_char,
                            comp.n(),
                            len as c_int,
                        ),
                        ds.1(
                            comp.ptr(),
                            rb.as_mut_ptr() as *mut c_char,
                            comp.n(),
                            len as c_int,
                        ),
                    )
                };
                let ctx = format!("safe len={len} {shape:?} enc={v}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, len as c_int, "{ctx}: decoded size");
                assert_bytes_eq(&cb[..len], &src, &format!("{ctx}: round trip"));
            }
        }
    }
}

// ===========================================================================
// Row 49 — LZ4_decompress_safe with dstCapacity > decoded size
// ===========================================================================
#[test]
fn row49_decompress_safe_oversized_capacity() {
    let enc = Enc::new();
    sym!(ds, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xD0_0049);

    for &len in KEY_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                for extra in [1usize, 7, 64, 4096] {
                    let cap = len + extra;
                    let mut cb = vec![0u8; cap + 64];
                    let mut rb = vec![0u8; cap + 64];
                    let (cn, rn) = unsafe {
                        (
                            ds.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                            ),
                            ds.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                            ),
                        )
                    };
                    let ctx = format!("safe-oversized len={len} {shape:?} enc={v} cap={cap}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, len as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(&cb[..len], &src, &format!("{ctx}: round trip"));
                }
            }
        }
    }
}

// ===========================================================================
// Row 50 — LZ4_decompress_safe_partial, targetOutputSize sweep,
//          dstCapacity == decodedSize
// ===========================================================================
#[test]
fn row50_partial_target_sweep() {
    let enc = Enc::new();
    sym!(dp, "LZ4_decompress_safe_partial", FnDecPartial);
    let mut rng = Rng::new(0xD0_0050);

    for &len in SMALL_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                for &t in &targets_for(len) {
                    let mut cb = vec![0u8; len + 64];
                    let mut rb = vec![0u8; len + 64];
                    let (cn, rn) = unsafe {
                        (
                            dp.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                t as c_int,
                                len as c_int,
                            ),
                            dp.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                t as c_int,
                                len as c_int,
                            ),
                        )
                    };
                    let ctx = format!("partial len={len} {shape:?} enc={v} tgt={t}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert!(cn >= 0, "{ctx}: negative return {cn}");
                    let got = cn as usize;
                    assert!(got <= len, "{ctx}: decoded {got} > {len}");
                    // Whatever was produced must be a prefix of the original.
                    assert_bytes_eq(&cb[..got], &src[..got], &format!("{ctx}: prefix"));
                    if t >= len {
                        assert_eq!(cn, len as c_int, "{ctx}: full decode expected");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 51 — _safe_partial with targetOutputSize == dstCapacity < decodedSize
// ===========================================================================
#[test]
fn row51_partial_truncating() {
    let enc = Enc::new();
    sym!(dp, "LZ4_decompress_safe_partial", FnDecPartial);
    let mut rng = Rng::new(0xD0_0051);

    for &len in SMALL_LENS {
        if len == 0 {
            continue; // nothing is "< decodedSize"
        }
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                let mut caps: Vec<usize> = vec![0, 1, 2, 3, 5, 11, 12, 13];
                for f in [2usize, 3, 4, 8, 16] {
                    caps.push(len / f);
                }
                caps.push(len - 1);
                for _ in 0..4 {
                    caps.push(rng.below(len));
                }
                caps.retain(|&c| c < len);
                caps.sort_unstable();
                caps.dedup();

                for &cap in &caps {
                    let mut cb = vec![0u8; cap + 64];
                    let mut rb = vec![0u8; cap + 64];
                    let (cn, rn) = unsafe {
                        (
                            dp.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                cap as c_int,
                            ),
                            dp.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                cap as c_int,
                            ),
                        )
                    };
                    let ctx = format!("partial-trunc len={len} {shape:?} enc={v} cap={cap}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert!(cn >= 0, "{ctx}: negative return {cn}");
                    let got = cn as usize;
                    assert!(got <= cap, "{ctx}: wrote {got} > cap {cap}");
                    assert_bytes_eq(&cb[..got], &src[..got], &format!("{ctx}: prefix"));
                }
            }
        }
    }
}

// ===========================================================================
// Row 52 — _safe_partial with targetOutputSize > dstCapacity
// ===========================================================================
#[test]
fn row52_partial_target_above_capacity() {
    let enc = Enc::new();
    sym!(dp, "LZ4_decompress_safe_partial", FnDecPartial);
    let mut rng = Rng::new(0xD0_0052);

    for &len in SMALL_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                let mut caps: Vec<usize> = vec![0, 1, 3, 12, 13, len / 4, len / 2, len];
                caps.retain(|&c| c <= len);
                caps.sort_unstable();
                caps.dedup();
                for &cap in &caps {
                    for extra in [1usize, 7, 64, len + 1, 1 << 20] {
                        let tgt = cap + extra;
                        let mut cb = vec![0u8; cap + 64];
                        let mut rb = vec![0u8; cap + 64];
                        let (cn, rn) = unsafe {
                            (
                                dp.0(
                                    comp.ptr(),
                                    cb.as_mut_ptr() as *mut c_char,
                                    comp.n(),
                                    tgt as c_int,
                                    cap as c_int,
                                ),
                                dp.1(
                                    comp.ptr(),
                                    rb.as_mut_ptr() as *mut c_char,
                                    comp.n(),
                                    tgt as c_int,
                                    cap as c_int,
                                ),
                            )
                        };
                        let ctx =
                            format!("partial-tgt>cap len={len} {shape:?} enc={v} cap={cap} tgt={tgt}");
                        chk(cn, &cb, rn, &rb, &ctx);
                        assert!(cn >= 0, "{ctx}: negative return {cn}");
                        let got = cn as usize;
                        assert!(got <= cap, "{ctx}: wrote {got} > cap {cap}");
                        assert_bytes_eq(&cb[..got], &src[..got], &format!("{ctx}: prefix"));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 53 — LZ4_decompress_fast with the exact originalSize
// ===========================================================================
#[test]
fn row53_decompress_fast() {
    let enc = Enc::new();
    sym!(df, "LZ4_decompress_fast", FnDecFast);
    let mut rng = Rng::new(0xD0_0053);

    for &len in KEY_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);
                let mut cb = vec![0u8; len + 64];
                let mut rb = vec![0u8; len + 64];
                let (cn, rn) = unsafe {
                    (
                        df.0(comp.ptr(), cb.as_mut_ptr() as *mut c_char, len as c_int),
                        df.1(comp.ptr(), rb.as_mut_ptr() as *mut c_char, len as c_int),
                    )
                };
                let ctx = format!("fast len={len} {shape:?} enc={v}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                assert_bytes_eq(&cb[..len], &src, &format!("{ctx}: round trip"));
            }
        }
    }
}

// ===========================================================================
// Row 54 — LZ4_uncompress / LZ4_uncompress_unknownOutputSize
// ===========================================================================
#[test]
fn row54_deprecated_uncompress_wrappers() {
    let enc = Enc::new();
    sym!(unc, "LZ4_uncompress", FnDecFast);
    sym!(uunk, "LZ4_uncompress_unknownOutputSize", FnDecSafe);
    sym!(df, "LZ4_decompress_fast", FnDecFast);
    sym!(ds, "LZ4_decompress_safe", FnDecSafe);
    let mut rng = Rng::new(0xD0_0054);

    for &len in KEY_LENS {
        for &shape in ALL_SHAPES {
            let src = gen_src(shape, len, &mut rng);
            for v in 0..NENC {
                let comp = enc.compress(&src, v);

                // LZ4_uncompress == LZ4_decompress_fast
                let mut cb = vec![0u8; len + 64];
                let mut rb = vec![0u8; len + 64];
                let mut refb = vec![0u8; len + 64];
                let (cn, rn, refn) = unsafe {
                    (
                        unc.0(comp.ptr(), cb.as_mut_ptr() as *mut c_char, len as c_int),
                        unc.1(comp.ptr(), rb.as_mut_ptr() as *mut c_char, len as c_int),
                        df.0(comp.ptr(), refb.as_mut_ptr() as *mut c_char, len as c_int),
                    )
                };
                let ctx = format!("uncompress len={len} {shape:?} enc={v}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, refn, "{ctx}: differs from LZ4_decompress_fast");
                assert_bytes_eq(&cb, &refb, &format!("{ctx}: vs decompress_fast bytes"));
                assert_bytes_eq(&cb[..len], &src, &format!("{ctx}: round trip"));

                // LZ4_uncompress_unknownOutputSize == LZ4_decompress_safe
                for cap in [len, len + 1, len + 64] {
                    let mut cb = vec![0u8; cap + 64];
                    let mut rb = vec![0u8; cap + 64];
                    let mut refb = vec![0u8; cap + 64];
                    let (cn, rn, refn) = unsafe {
                        (
                            uunk.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                            ),
                            uunk.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                            ),
                            ds.0(
                                comp.ptr(),
                                refb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                            ),
                        )
                    };
                    let ctx = format!("uncompress_unknown len={len} {shape:?} enc={v} cap={cap}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, refn, "{ctx}: differs from LZ4_decompress_safe");
                    assert_bytes_eq(&cb, &refb, &format!("{ctx}: vs decompress_safe bytes"));
                    assert_eq!(cn, len as c_int, "{ctx}: decoded size");
                }
            }
        }
    }
}

/// Build a `[pad][prefix][block]` destination buffer and return the byte offset
/// of the block within it.
fn prefixed_dst(prefix: &[u8], block_len: usize) -> (Vec<u8>, usize) {
    const PAD: usize = 128;
    let mut v = vec![0u8; PAD + prefix.len() + block_len + 64];
    v[PAD..PAD + prefix.len()].copy_from_slice(prefix);
    (v, PAD + prefix.len())
}

// ===========================================================================
// Rows 55/56 — _safe_withPrefix64k / _fast_withPrefix64k
// ===========================================================================
#[test]
fn rows55_56_with_prefix_64k() {
    let enc = Enc::new();
    sym!(dsp, "LZ4_decompress_safe_withPrefix64k", FnDecSafe);
    sym!(dfp, "LZ4_decompress_fast_withPrefix64k", FnDecFast);
    let mut rng = Rng::new(0xD0_0055);

    for &psz in &[65536usize, 70_000, 100_000] {
        for &shape in ALL_SHAPES {
            for &n in &[1usize, 13, 64, 1024, 9000, 40_000] {
                let whole = dict_case(shape, psz, n, &mut rng);
                let (prefix, block) = whole.split_at(psz);
                let comp = enc.compress_after_dict(prefix, block);

                // ---- row 55 : LZ4_decompress_safe_withPrefix64k
                let (mut cb, at) = prefixed_dst(prefix, n);
                let (mut rb, _) = prefixed_dst(prefix, n);
                let (cn, rn) = unsafe {
                    (
                        dsp.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                        ),
                        dsp.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                        ),
                    )
                };
                let ctx = format!("safe_withPrefix64k psz={psz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                assert_bytes_eq(&cb[at..at + n], block, &format!("{ctx}: round trip"));

                // dstCapacity larger than the block, too
                let (mut cb, at) = prefixed_dst(prefix, n + 33);
                let (mut rb, _) = prefixed_dst(prefix, n + 33);
                let (cn, rn) = unsafe {
                    (
                        dsp.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            (n + 33) as c_int,
                        ),
                        dsp.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            (n + 33) as c_int,
                        ),
                    )
                };
                let ctx = format!("safe_withPrefix64k-oversized psz={psz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n as c_int, "{ctx}: decoded size");

                // ---- row 56 : LZ4_decompress_fast_withPrefix64k
                let (mut cb, at) = prefixed_dst(prefix, n);
                let (mut rb, _) = prefixed_dst(prefix, n);
                let (cn, rn) = unsafe {
                    (
                        dfp.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            n as c_int,
                        ),
                        dfp.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            n as c_int,
                        ),
                    )
                };
                let ctx = format!("fast_withPrefix64k psz={psz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                assert_bytes_eq(&cb[at..at + n], block, &format!("{ctx}: round trip"));
            }
        }
    }
}

/// The dictionary-size sweep of rows 57-62. `>= 65536` disables `checkOffset`.
const DICT_SIZES: &[usize] = &[0, 4, 1024, 65535, 65536, 70_000];

/// A separately-allocated copy of `dict`, padded so that
/// `dictStart + dictSize` can never coincide with any `dst`.
fn separate_dict(dict: &[u8]) -> Vec<u8> {
    let mut v = vec![0u8; dict.len() + 64];
    v[..dict.len()].copy_from_slice(dict);
    v
}

// ===========================================================================
// Row 57 — LZ4_decompress_safe_usingDict over the dictSize sweep
// ===========================================================================
#[test]
fn row57_safe_using_dict_size_sweep() {
    let enc = Enc::new();
    sym!(dud, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0xD0_0057);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[13usize, 64, 1024, 4096, 20_000] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);

                // separate dictionary buffer => usingExtDict path
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);
                for extra in [0usize, 5, 64] {
                    let cap = n + extra;
                    let mut cb = vec![0u8; cap + 64];
                    let mut rb = vec![0u8; cap + 64];
                    let (cn, rn) = unsafe {
                        (
                            dud.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                cdict.as_ptr() as *const c_char,
                                dsz as c_int,
                            ),
                            dud.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                rdict.as_ptr() as *const c_char,
                                dsz as c_int,
                            ),
                        )
                    };
                    let ctx = format!("usingDict-ext dsz={dsz} {shape:?} n={n} cap={cap}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(&cb[..n], block, &format!("{ctx}: round trip"));
                }
            }
        }
    }
}

// ===========================================================================
// Row 58 — usingDict with a CONTIGUOUS dict vs a separate buffer
// ===========================================================================
#[test]
fn row58_safe_using_dict_contiguous_vs_separate() {
    let enc = Enc::new();
    sym!(dud, "LZ4_decompress_safe_usingDict", FnDecUsingDict);
    let mut rng = Rng::new(0xD0_0058);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[13usize, 64, 1024, 4096, 20_000] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);

                // ---- contiguous : dictStart + dictSize == dst
                let (mut cb, at) = prefixed_dst(dict, n);
                let (mut rb, _) = prefixed_dst(dict, n);
                let (cn, rn) = unsafe {
                    (
                        dud.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                            cb.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                        dud.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                            rb.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                    )
                };
                let ctx = format!("usingDict-contig dsz={dsz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                assert_bytes_eq(&cb[at..at + n], block, &format!("{ctx}: round trip"));

                // ---- separate buffer, same payload : must decode identically
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);
                let mut cb2 = vec![0u8; n + 64];
                let mut rb2 = vec![0u8; n + 64];
                let (cn2, rn2) = unsafe {
                    (
                        dud.0(
                            comp.ptr(),
                            cb2.as_mut_ptr() as *mut c_char,
                            comp.n(),
                            n as c_int,
                            cdict.as_ptr() as *const c_char,
                            dsz as c_int,
                        ),
                        dud.1(
                            comp.ptr(),
                            rb2.as_mut_ptr() as *mut c_char,
                            comp.n(),
                            n as c_int,
                            rdict.as_ptr() as *const c_char,
                            dsz as c_int,
                        ),
                    )
                };
                let ctx2 = format!("usingDict-separate dsz={dsz} {shape:?} n={n}");
                chk(cn2, &cb2, rn2, &rb2, &ctx2);
                assert_eq!(cn2, n as c_int, "{ctx2}: decoded size");
                assert_bytes_eq(&cb2[..n], block, &format!("{ctx2}: round trip"));
                // Both dictionary layouts must yield the same decoded bytes.
                assert_bytes_eq(
                    &cb[at..at + n],
                    &cb2[..n],
                    &format!("{ctx2}: contiguous vs separate"),
                );
            }
        }
    }
}

// ===========================================================================
// Row 59 — LZ4_decompress_safe_partial_usingDict, dict x target sweep
// ===========================================================================
#[test]
fn row59_partial_using_dict() {
    let enc = Enc::new();
    sym!(dpd, "LZ4_decompress_safe_partial_usingDict", FnDecPartialUsingDict);
    let mut rng = Rng::new(0xD0_0059);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[64usize, 1024, 4096] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);

                for &t in &targets_for(n) {
                    // (a) separate dict, dstCapacity == n
                    let mut cb = vec![0u8; n + 64];
                    let mut rb = vec![0u8; n + 64];
                    let (cn, rn) = unsafe {
                        (
                            dpd.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                t as c_int,
                                n as c_int,
                                cdict.as_ptr() as *const c_char,
                                dsz as c_int,
                            ),
                            dpd.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                t as c_int,
                                n as c_int,
                                rdict.as_ptr() as *const c_char,
                                dsz as c_int,
                            ),
                        )
                    };
                    let ctx = format!("partial_usingDict-ext dsz={dsz} {shape:?} n={n} tgt={t}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert!(cn >= 0, "{ctx}: negative return {cn}");
                    let got = cn as usize;
                    assert!(got <= n, "{ctx}: wrote {got} > {n}");
                    assert_bytes_eq(&cb[..got], &block[..got], &format!("{ctx}: prefix"));
                    if t >= n {
                        assert_eq!(cn, n as c_int, "{ctx}: full decode expected");
                    }

                    // (b) contiguous dict, dstCapacity == min(t, n) (truncating)
                    let cap = t.min(n);
                    let (mut cb, at) = prefixed_dst(dict, cap);
                    let (mut rb, _) = prefixed_dst(dict, cap);
                    let (cn, rn) = unsafe {
                        (
                            dpd.0(
                                comp.ptr(),
                                cb.as_mut_ptr().add(at) as *mut c_char,
                                comp.n(),
                                t as c_int,
                                cap as c_int,
                                cb.as_ptr().add(at - dsz) as *const c_char,
                                dsz as c_int,
                            ),
                            dpd.1(
                                comp.ptr(),
                                rb.as_mut_ptr().add(at) as *mut c_char,
                                comp.n(),
                                t as c_int,
                                cap as c_int,
                                rb.as_ptr().add(at - dsz) as *const c_char,
                                dsz as c_int,
                            ),
                        )
                    };
                    let ctx = format!(
                        "partial_usingDict-contig dsz={dsz} {shape:?} n={n} tgt={t} cap={cap}"
                    );
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert!(cn >= 0, "{ctx}: negative return {cn}");
                    let got = cn as usize;
                    assert!(got <= cap, "{ctx}: wrote {got} > cap {cap}");
                    assert_bytes_eq(
                        &cb[at..at + got],
                        &block[..got],
                        &format!("{ctx}: prefix"),
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 60 — LZ4_decompress_fast_usingDict, contiguous and non-contiguous
// ===========================================================================
#[test]
fn row60_fast_using_dict() {
    let enc = Enc::new();
    sym!(dfd, "LZ4_decompress_fast_usingDict", FnDecFastUsingDict);
    let mut rng = Rng::new(0xD0_0060);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[13usize, 64, 1024, 4096, 20_000] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);

                // ---- non-contiguous dictionary
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);
                let mut cb = vec![0u8; n + 64];
                let mut rb = vec![0u8; n + 64];
                let (cn, rn) = unsafe {
                    (
                        dfd.0(
                            comp.ptr(),
                            cb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            cdict.as_ptr() as *const c_char,
                            dsz as c_int,
                        ),
                        dfd.1(
                            comp.ptr(),
                            rb.as_mut_ptr() as *mut c_char,
                            n as c_int,
                            rdict.as_ptr() as *const c_char,
                            dsz as c_int,
                        ),
                    )
                };
                let ctx = format!("fast_usingDict-ext dsz={dsz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                assert_bytes_eq(&cb[..n], block, &format!("{ctx}: round trip"));

                // ---- contiguous dictionary (prefix mode)
                let (mut cb, at) = prefixed_dst(dict, n);
                let (mut rb, _) = prefixed_dst(dict, n);
                let (cn, rn) = unsafe {
                    (
                        dfd.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            n as c_int,
                            cb.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                        dfd.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            n as c_int,
                            rb.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                    )
                };
                let ctx = format!("fast_usingDict-contig dsz={dsz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                assert_bytes_eq(&cb[at..at + n], block, &format!("{ctx}: round trip"));
            }
        }
    }
}

// ===========================================================================
// Row 61 — LZ4_decompress_safe_forceExtDict over the dictSize sweep
// ===========================================================================
#[test]
fn row61_safe_force_ext_dict() {
    let enc = Enc::new();
    sym!(fed, "LZ4_decompress_safe_forceExtDict", FnDecForceExt);
    let mut rng = Rng::new(0xD0_0061);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[13usize, 64, 1024, 4096, 20_000] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);

                for extra in [0usize, 1, 64] {
                    let cap = n + extra;
                    // separate dictionary buffer
                    let mut cb = vec![0u8; cap + 64];
                    let mut rb = vec![0u8; cap + 64];
                    let (cn, rn) = unsafe {
                        (
                            fed.0(
                                comp.ptr(),
                                cb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                cdict.as_ptr() as *const c_void,
                                dsz,
                            ),
                            fed.1(
                                comp.ptr(),
                                rb.as_mut_ptr() as *mut c_char,
                                comp.n(),
                                cap as c_int,
                                rdict.as_ptr() as *const c_void,
                                dsz,
                            ),
                        )
                    };
                    let ctx = format!("forceExtDict dsz={dsz} {shape:?} n={n} cap={cap}");
                    chk(cn, &cb, rn, &rb, &ctx);
                    assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(&cb[..n], block, &format!("{ctx}: round trip"));
                }

                // Also force extDict on a physically CONTIGUOUS dictionary
                // (dictStart+dictSize == dst): the extDict resolution must give
                // the very same bytes as the prefix path would.
                let (mut cb, at) = prefixed_dst(dict, n);
                let (mut rb, _) = prefixed_dst(dict, n);
                let (cn, rn) = unsafe {
                    (
                        fed.0(
                            comp.ptr(),
                            cb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                            cb.as_ptr().add(at - dsz) as *const c_void,
                            dsz,
                        ),
                        fed.1(
                            comp.ptr(),
                            rb.as_mut_ptr().add(at) as *mut c_char,
                            comp.n(),
                            n as c_int,
                            rb.as_ptr().add(at - dsz) as *const c_void,
                            dsz,
                        ),
                    )
                };
                let ctx = format!("forceExtDict-contig dsz={dsz} {shape:?} n={n}");
                chk(cn, &cb, rn, &rb, &ctx);
                assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                assert_bytes_eq(&cb[at..at + n], block, &format!("{ctx}: round trip"));
            }
        }
    }
}

// ===========================================================================
// Row 62 — LZ4_decompress_safe_partial_forceExtDict, dict x target sweep
// ===========================================================================
#[test]
fn row62_partial_force_ext_dict() {
    let enc = Enc::new();
    sym!(pfed, "LZ4_decompress_safe_partial_forceExtDict", FnDecPartialForceExt);
    let mut rng = Rng::new(0xD0_0062);

    for &dsz in DICT_SIZES {
        for &shape in ALL_SHAPES {
            for &n in &[64usize, 1024, 4096] {
                let whole = dict_case(shape, dsz, n, &mut rng);
                let (dict, block) = whole.split_at(dsz);
                let comp = enc.compress_after_dict(dict, block);
                let cdict = separate_dict(dict);
                let rdict = separate_dict(dict);

                for &t in &targets_for(n) {
                    for &cap in &[n, t.min(n)] {
                        let mut cb = vec![0u8; cap + 64];
                        let mut rb = vec![0u8; cap + 64];
                        let (cn, rn) = unsafe {
                            (
                                pfed.0(
                                    comp.ptr(),
                                    cb.as_mut_ptr() as *mut c_char,
                                    comp.n(),
                                    t as c_int,
                                    cap as c_int,
                                    cdict.as_ptr() as *const c_void,
                                    dsz,
                                ),
                                pfed.1(
                                    comp.ptr(),
                                    rb.as_mut_ptr() as *mut c_char,
                                    comp.n(),
                                    t as c_int,
                                    cap as c_int,
                                    rdict.as_ptr() as *const c_void,
                                    dsz,
                                ),
                            )
                        };
                        let ctx = format!(
                            "partial_forceExtDict dsz={dsz} {shape:?} n={n} tgt={t} cap={cap}"
                        );
                        chk(cn, &cb, rn, &rb, &ctx);
                        assert!(cn >= 0, "{ctx}: negative return {cn}");
                        let got = cn as usize;
                        assert!(got <= cap, "{ctx}: wrote {got} > cap {cap}");
                        assert_bytes_eq(&cb[..got], &block[..got], &format!("{ctx}: prefix"));
                        if t >= n && cap >= n {
                            assert_eq!(cn, n as c_int, "{ctx}: full decode expected");
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 63 — LZ4_setStreamDecode + _safe_continue, UNIFORM blocks, one
//          contiguous output buffer
// ===========================================================================
#[test]
fn row63_safe_continue_uniform_contiguous() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(ssd, "LZ4_setStreamDecode", FnSetSD);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0xD0_0063);

    for &blk in &[64usize, 1024, 4096, 9000, 70_000] {
        let nblocks = (200_000 / blk).max(3);
        for &shape in ALL_SHAPES {
            let total = blk * nblocks;
            let data = gen_src(shape, total, &mut rng);
            let sizes = vec![blk; nblocks];
            let comps = enc.compress_chain(&data, &sizes, 1);

            // Output buffers live for the whole chain (the decode stream keeps
            // a raw pointer into them).
            let mut cout = vec![0u8; total + 64];
            let mut rout = vec![0u8; total + 64];
            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                assert!(!cs.is_null() && !rs.is_null());
                assert_ret_eq(
                    ssd.0(cs, std::ptr::null(), 0),
                    ssd.1(rs, std::ptr::null(), 0),
                    "setStreamDecode(NULL,0)",
                );
                for (i, comp) in comps.iter().enumerate() {
                    let off = i * blk;
                    let (cn, rn) = (
                        dsc.0(
                            cs,
                            comp.ptr(),
                            cout.as_mut_ptr().add(off) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                        dsc.1(
                            rs,
                            comp.ptr(),
                            rout.as_mut_ptr().add(off) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                    );
                    let ctx = format!("safe_continue uniform blk={blk} {shape:?} i={i}");
                    chk(cn, &cout, rn, &rout, &ctx);
                    assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(
                        &cout[off..off + blk],
                        &data[off..off + blk],
                        &format!("{ctx}: round trip"),
                    );
                }
                assert_ret_eq(fsd.0(cs), fsd.1(rs), "freeStreamDecode");
            }
        }
    }

    // ---- LZ4_setStreamDecode() with a REAL dictionary -------------------
    // The decode stream is primed with `dsz` bytes of history, exactly as the
    // compressor was primed with LZ4_loadDict(). dsz == 65536 selects
    // `withPrefix64k`, dsz == 1024 selects `withSmallPrefix` / `doubleDict`.
    for &dsz in &[1024usize, 65536] {
        for &blk in &[1024usize, 9000] {
            let nblocks = 10usize;
            for &shape in ALL_SHAPES {
                let total = dsz + blk * nblocks;
                let mut data = gen_src(shape, total, &mut rng);
                splice_backrefs(&mut data, dsz, &mut rng);

                let mut comps: Vec<Comp> = Vec::with_capacity(nblocks);
                unsafe {
                    let s = (enc.cs)();
                    (enc.ld)(s, data.as_ptr() as *const c_char, dsz as c_int);
                    for i in 0..nblocks {
                        let off = dsz + i * blk;
                        let cap = enc.bound(blk);
                        let mut buf = vec![0u8; cap + 64];
                        let r = (enc.cont)(
                            s,
                            data[off..].as_ptr() as *const c_char,
                            buf.as_mut_ptr() as *mut c_char,
                            blk as c_int,
                            cap as c_int,
                            1,
                        );
                        assert!(r > 0);
                        comps.push(Comp { buf, n: r as usize });
                    }
                    (enc.fsy)(s);
                }

                // (b) contiguous: the dictionary sits directly before the output
                let (mut cout, at) = prefixed_dst(&data[..dsz], blk * nblocks);
                let (mut rout, _) = prefixed_dst(&data[..dsz], blk * nblocks);
                unsafe {
                    let (cs, rs) = (csd.0(), csd.1());
                    assert_ret_eq(
                        ssd.0(
                            cs,
                            cout.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                        ssd.1(
                            rs,
                            rout.as_ptr().add(at - dsz) as *const c_char,
                            dsz as c_int,
                        ),
                        "setStreamDecode(contiguous dict)",
                    );
                    for (i, comp) in comps.iter().enumerate() {
                        let off = at + i * blk;
                        let (cn, rn) = (
                            dsc.0(
                                cs,
                                comp.ptr(),
                                cout.as_mut_ptr().add(off) as *mut c_char,
                                comp.n(),
                                blk as c_int,
                            ),
                            dsc.1(
                                rs,
                                comp.ptr(),
                                rout.as_mut_ptr().add(off) as *mut c_char,
                                comp.n(),
                                blk as c_int,
                            ),
                        );
                        let ctx = format!(
                            "setStreamDecode-contig dsz={dsz} blk={blk} {shape:?} i={i}"
                        );
                        chk(cn, &cout, rn, &rout, &ctx);
                        assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                        assert_bytes_eq(
                            &cout[off..off + blk],
                            &data[dsz + i * blk..dsz + (i + 1) * blk],
                            &format!("{ctx}: round trip"),
                        );
                    }
                    fsd.0(cs);
                    fsd.1(rs);
                }

                // (c) separate dictionary buffer: the FIRST block takes the
                //     extDict promotion branch, later blocks the doubleDict one.
                let cdict = separate_dict(&data[..dsz]);
                let rdict = separate_dict(&data[..dsz]);
                let mut cout = vec![0u8; blk * nblocks + 64];
                let mut rout = vec![0u8; blk * nblocks + 64];
                unsafe {
                    let (cs, rs) = (csd.0(), csd.1());
                    assert_ret_eq(
                        ssd.0(cs, cdict.as_ptr() as *const c_char, dsz as c_int),
                        ssd.1(rs, rdict.as_ptr() as *const c_char, dsz as c_int),
                        "setStreamDecode(separate dict)",
                    );
                    for (i, comp) in comps.iter().enumerate() {
                        let off = i * blk;
                        let (cn, rn) = (
                            dsc.0(
                                cs,
                                comp.ptr(),
                                cout.as_mut_ptr().add(off) as *mut c_char,
                                comp.n(),
                                blk as c_int,
                            ),
                            dsc.1(
                                rs,
                                comp.ptr(),
                                rout.as_mut_ptr().add(off) as *mut c_char,
                                comp.n(),
                                blk as c_int,
                            ),
                        );
                        let ctx =
                            format!("setStreamDecode-separate dsz={dsz} blk={blk} {shape:?} i={i}");
                        chk(cn, &cout, rn, &rout, &ctx);
                        assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                        assert_bytes_eq(
                            &cout[off..off + blk],
                            &data[dsz + i * blk..dsz + (i + 1) * blk],
                            &format!("{ctx}: round trip"),
                        );
                    }
                    fsd.0(cs);
                    fsd.1(rs);
                }
            }
        }
    }
}

// ===========================================================================
// Row 64 — _safe_continue, RANDOM block sizes, many blocks
// ===========================================================================
#[test]
fn row64_safe_continue_random_blocks() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(ssd, "LZ4_setStreamDecode", FnSetSD);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0xD0_0064);

    for &shape in ALL_SHAPES {
        for round in 0..2 {
            let total = 300_000usize;
            let data = gen_src(shape, total, &mut rng);
            let mut sizes: Vec<usize> = Vec::new();
            let mut acc = 0usize;
            while acc < total {
                // Occasionally an EMPTY block (a valid 1-byte payload): the C
                // returns 0 and leaves the stream state untouched.
                if !sizes.is_empty() && rng.below(11) == 0 {
                    sizes.push(0);
                    continue;
                }
                let n = if round == 0 {
                    rng.range(1, 3_000)
                } else {
                    rng.range(1, 40_000)
                }
                .min(total - acc);
                sizes.push(n);
                acc += n;
            }
            let comps = enc.compress_chain(&data, &sizes, 1);

            let mut cout = vec![0u8; total + 64];
            let mut rout = vec![0u8; total + 64];
            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                ssd.0(cs, std::ptr::null(), 0);
                ssd.1(rs, std::ptr::null(), 0);
                let mut off = 0usize;
                for (i, comp) in comps.iter().enumerate() {
                    let n = sizes[i];
                    let (cn, rn) = (
                        dsc.0(
                            cs,
                            comp.ptr(),
                            cout.as_mut_ptr().add(off) as *mut c_char,
                            comp.n(),
                            n as c_int,
                        ),
                        dsc.1(
                            rs,
                            comp.ptr(),
                            rout.as_mut_ptr().add(off) as *mut c_char,
                            comp.n(),
                            n as c_int,
                        ),
                    );
                    let ctx = format!("safe_continue random r={round} {shape:?} i={i} n={n}");
                    chk(cn, &cout, rn, &rout, &ctx);
                    assert_eq!(cn, n as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(
                        &cout[off..off + n],
                        &data[off..off + n],
                        &format!("{ctx}: round trip"),
                    );
                    off += n;
                }
                fsd.0(cs);
                fsd.1(rs);
            }
        }
    }
}

// ===========================================================================
// Row 65 — _safe_continue with a SEPARATE output buffer per block
//          => the `forceExtDict` promotion branch (lz4.c:2656)
// ===========================================================================
#[test]
fn row65_safe_continue_separate_buffers() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0xD0_0065);

    for &blk in &[64usize, 1024, 8192, 40_000] {
        let nblocks = 14usize;
        for &shape in ALL_SHAPES {
            // Every block lives in its own allocation => the compressor uses
            // extDict, and the decoder must promote its prefix to extDict too.
            let mut blocks: Vec<Vec<u8>> = Vec::with_capacity(nblocks);
            for i in 0..nblocks {
                let b = if i == 0 {
                    gen_src(shape, blk, &mut rng)
                } else {
                    derive_block(&blocks[i - 1], shape, blk, &mut rng)
                };
                blocks.push(b);
            }

            // Compress the chain from the separate source buffers.
            let mut comps: Vec<Comp> = Vec::with_capacity(nblocks);
            unsafe {
                let s = (enc.cs)();
                for b in blocks.iter() {
                    let cap = enc.bound(blk);
                    let mut buf = vec![0u8; cap + 64];
                    let r = (enc.cont)(
                        s,
                        b.as_ptr() as *const c_char,
                        buf.as_mut_ptr() as *mut c_char,
                        blk as c_int,
                        cap as c_int,
                        1,
                    );
                    assert!(r > 0);
                    comps.push(Comp { buf, n: r as usize });
                }
                (enc.fsy)(s);
            }

            // Pre-allocate every output buffer: the stream keeps a pointer into
            // the previous one, so they must all outlive the whole chain.
            let mut couts: Vec<Vec<u8>> = (0..nblocks).map(|_| vec![0u8; blk + 64]).collect();
            let mut routs: Vec<Vec<u8>> = (0..nblocks).map(|_| vec![0u8; blk + 64]).collect();

            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                for i in 0..nblocks {
                    let (cn, rn) = (
                        dsc.0(
                            cs,
                            comps[i].ptr(),
                            couts[i].as_mut_ptr() as *mut c_char,
                            comps[i].n(),
                            blk as c_int,
                        ),
                        dsc.1(
                            rs,
                            comps[i].ptr(),
                            routs[i].as_mut_ptr() as *mut c_char,
                            comps[i].n(),
                            blk as c_int,
                        ),
                    );
                    let ctx = format!("safe_continue separate blk={blk} {shape:?} i={i}");
                    chk(cn, &couts[i], rn, &routs[i], &ctx);
                    assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(&couts[i][..blk], &blocks[i], &format!("{ctx}: round trip"));
                }
                fsd.0(cs);
                fsd.1(rs);
            }
        }
    }
}

// ===========================================================================
// Row 66 — _safe_continue into a ring buffer sized by
//          LZ4_decoderRingBufferSize(maxBlockSize), wrapping several times
// ===========================================================================
#[test]
fn row66_safe_continue_ring_buffer() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(drbs, "LZ4_decoderRingBufferSize", FnBound);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0xD0_0066);

    for &blk in &[1024usize, 4096, 9000, 40_000] {
        let ring = unsafe { drbs.0(blk as c_int) } as usize;
        assert_ret_eq(
            ring as c_int,
            unsafe { drbs.1(blk as c_int) },
            "decoderRingBufferSize",
        );
        assert!(ring > blk);
        for &shape in ALL_SHAPES {
            // Enough blocks to wrap the ring at least 3 times.
            let nblocks = (3 * ring / blk) + 4;
            let total = blk * nblocks;
            let data = gen_src(shape, total, &mut rng);
            let comps = enc.compress_chain(&data, &vec![blk; nblocks], 1);

            let mut cring = vec![0u8; ring + 64];
            let mut rring = vec![0u8; ring + 64];
            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                let mut pos = 0usize;
                let mut wraps = 0usize;
                for (i, comp) in comps.iter().enumerate() {
                    if pos + blk > ring {
                        pos = 0;
                        wraps += 1;
                    }
                    let (cn, rn) = (
                        dsc.0(
                            cs,
                            comp.ptr(),
                            cring.as_mut_ptr().add(pos) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                        dsc.1(
                            rs,
                            comp.ptr(),
                            rring.as_mut_ptr().add(pos) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                    );
                    let ctx = format!("safe_continue ring blk={blk} {shape:?} i={i} pos={pos}");
                    chk(cn, &cring, rn, &rring, &ctx);
                    assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(
                        &cring[pos..pos + blk],
                        &data[i * blk..i * blk + blk],
                        &format!("{ctx}: round trip"),
                    );
                    pos += blk;
                }
                assert!(wraps >= 3, "ring blk={blk}: only {wraps} wraps");
                fsd.0(cs);
                fsd.1(rs);
            }
        }
    }
}

// ===========================================================================
// Row 67 — _safe_continue small-prefix (prefixSize < 65535) and doubleDict
//          paths: a ring buffer that wraps long before the prefix reaches 64 KB
// ===========================================================================
#[test]
fn row67_safe_continue_small_prefix_and_double_dict() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    let mut rng = Rng::new(0xD0_0067);

    for &(blk, mult) in &[(64usize, 8usize), (512, 4), (1024, 6), (4000, 5), (9000, 4)] {
        let ring = blk * mult; // far below 64 KB => small prefix at every wrap
        let nblocks = mult * 5;
        for &shape in ALL_SHAPES {
            // The COMPRESSOR uses a ring buffer of the same geometry, so its
            // history matches exactly what the decoder still has available.
            let mut sring = vec![0u8; ring];
            let mut comps: Vec<Comp> = Vec::with_capacity(nblocks);
            let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(nblocks);
            unsafe {
                let s = (enc.cs)();
                let mut pos = 0usize;
                let mut prev: Vec<u8> = Vec::new();
                for _ in 0..nblocks {
                    if pos + blk > ring {
                        pos = 0;
                    }
                    let chunk = if prev.is_empty() {
                        gen_src(shape, blk, &mut rng)
                    } else {
                        derive_block(&prev, shape, blk, &mut rng)
                    };
                    sring[pos..pos + blk].copy_from_slice(&chunk);
                    let cap = enc.bound(blk);
                    let mut buf = vec![0u8; cap + 64];
                    let r = (enc.cont)(
                        s,
                        sring[pos..].as_ptr() as *const c_char,
                        buf.as_mut_ptr() as *mut c_char,
                        blk as c_int,
                        cap as c_int,
                        1,
                    );
                    assert!(r > 0);
                    comps.push(Comp { buf, n: r as usize });
                    prev = chunk.clone();
                    chunks.push(chunk);
                    pos += blk;
                }
                (enc.fsy)(s);
            }

            let mut cring = vec![0u8; ring + 64];
            let mut rring = vec![0u8; ring + 64];
            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                let mut pos = 0usize;
                for (i, comp) in comps.iter().enumerate() {
                    if pos + blk > ring {
                        pos = 0;
                    }
                    let (cn, rn) = (
                        dsc.0(
                            cs,
                            comp.ptr(),
                            cring.as_mut_ptr().add(pos) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                        dsc.1(
                            rs,
                            comp.ptr(),
                            rring.as_mut_ptr().add(pos) as *mut c_char,
                            comp.n(),
                            blk as c_int,
                        ),
                    );
                    let ctx =
                        format!("safe_continue smallprefix blk={blk} ring={ring} {shape:?} i={i}");
                    chk(cn, &cring, rn, &rring, &ctx);
                    assert_eq!(cn, blk as c_int, "{ctx}: decoded size");
                    assert_bytes_eq(
                        &cring[pos..pos + blk],
                        &chunks[i],
                        &format!("{ctx}: round trip"),
                    );
                    pos += blk;
                }
                fsd.0(cs);
                fsd.1(rs);
            }
        }
    }
}

// ===========================================================================
// Row 68 — LZ4_decompress_fast_continue, uniform and random block sizes
// ===========================================================================
#[test]
fn row68_fast_continue() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(ssd, "LZ4_setStreamDecode", FnSetSD);
    sym!(dfc, "LZ4_decompress_fast_continue", FnDecFastContinue);
    let mut rng = Rng::new(0xD0_0068);

    // (a) uniform block sizes
    for &blk in &[64usize, 1024, 4096, 9000, 70_000] {
        let nblocks = (200_000 / blk).max(3);
        for &shape in ALL_SHAPES {
            let total = blk * nblocks;
            let data = gen_src(shape, total, &mut rng);
            let comps = enc.compress_chain(&data, &vec![blk; nblocks], 1);

            let mut cout = vec![0u8; total + 64];
            let mut rout = vec![0u8; total + 64];
            unsafe {
                let (cs, rs) = (csd.0(), csd.1());
                ssd.0(cs, std::ptr::null(), 0);
                ssd.1(rs, std::ptr::null(), 0);
                for (i, comp) in comps.iter().enumerate() {
                    let off = i * blk;
                    let (cn, rn) = (
                        dfc.0(
                            cs,
                            comp.ptr(),
                            cout.as_mut_ptr().add(off) as *mut c_char,
                            blk as c_int,
                        ),
                        dfc.1(
                            rs,
                            comp.ptr(),
                            rout.as_mut_ptr().add(off) as *mut c_char,
                            blk as c_int,
                        ),
                    );
                    let ctx = format!("fast_continue uniform blk={blk} {shape:?} i={i}");
                    chk(cn, &cout, rn, &rout, &ctx);
                    assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                    assert_bytes_eq(
                        &cout[off..off + blk],
                        &data[off..off + blk],
                        &format!("{ctx}: round trip"),
                    );
                }
                fsd.0(cs);
                fsd.1(rs);
            }
        }
    }

    // (b) random block sizes
    for &shape in ALL_SHAPES {
        let total = 250_000usize;
        let data = gen_src(shape, total, &mut rng);
        let mut sizes: Vec<usize> = Vec::new();
        let mut acc = 0usize;
        while acc < total {
            let n = rng.range(1, 20_000).min(total - acc);
            sizes.push(n);
            acc += n;
        }
        let comps = enc.compress_chain(&data, &sizes, 1);

        let mut cout = vec![0u8; total + 64];
        let mut rout = vec![0u8; total + 64];
        unsafe {
            let (cs, rs) = (csd.0(), csd.1());
            let mut off = 0usize;
            for (i, comp) in comps.iter().enumerate() {
                let n = sizes[i];
                let (cn, rn) = (
                    dfc.0(
                        cs,
                        comp.ptr(),
                        cout.as_mut_ptr().add(off) as *mut c_char,
                        n as c_int,
                    ),
                    dfc.1(
                        rs,
                        comp.ptr(),
                        rout.as_mut_ptr().add(off) as *mut c_char,
                        n as c_int,
                    ),
                );
                let ctx = format!("fast_continue random {shape:?} i={i} n={n}");
                chk(cn, &cout, rn, &rout, &ctx);
                assert_eq!(cn, comp.n as c_int, "{ctx}: consumed input size");
                assert_bytes_eq(
                    &cout[off..off + n],
                    &data[off..off + n],
                    &format!("{ctx}: round trip"),
                );
                off += n;
            }
            fsd.0(cs);
            fsd.1(rs);
        }
    }
}

// ===========================================================================
// Row 69 — createStreamDecode / freeStreamDecode lifecycle +
//          LZ4_decoderRingBufferSize value sweep
// ===========================================================================
#[test]
fn row69_stream_decode_lifecycle_and_ring_size() {
    let enc = Enc::new();
    sym!(csd, "LZ4_createStreamDecode", FnCreateSD);
    sym!(fsd, "LZ4_freeStreamDecode", FnFreeSD);
    sym!(ssd, "LZ4_setStreamDecode", FnSetSD);
    sym!(dsc, "LZ4_decompress_safe_continue", FnDecSafeContinue);
    sym!(drbs, "LZ4_decoderRingBufferSize", FnBound);
    let mut rng = Rng::new(0xD0_0069);

    unsafe {
        // ---- lifecycle : many create/free rounds, plus free(NULL)
        for _ in 0..64 {
            let (cs, rs) = (csd.0(), csd.1());
            assert!(!cs.is_null(), "C createStreamDecode returned NULL");
            assert!(!rs.is_null(), "Rust createStreamDecode returned NULL");
            // A freshly created stream must behave like a zeroed one.
            assert_ret_eq(
                ssd.0(cs, std::ptr::null(), 0),
                ssd.1(rs, std::ptr::null(), 0),
                "setStreamDecode on a fresh stream",
            );
            assert_ret_eq(fsd.0(cs), fsd.1(rs), "freeStreamDecode");
        }
        assert_ret_eq(
            fsd.0(std::ptr::null_mut()),
            fsd.1(std::ptr::null_mut()),
            "freeStreamDecode(NULL)",
        );

        // A fresh (never `setStreamDecode`d) stream must decode a first block.
        let src = gen_src(Shape::Texty, 5000, &mut rng);
        let comp = enc.compress(&src, 0);
        let (cs, rs) = (csd.0(), csd.1());
        let mut cb = vec![0u8; 5000 + 64];
        let mut rb = vec![0u8; 5000 + 64];
        let (cn, rn) = (
            dsc.0(cs, comp.ptr(), cb.as_mut_ptr() as *mut c_char, comp.n(), 5000),
            dsc.1(rs, comp.ptr(), rb.as_mut_ptr() as *mut c_char, comp.n(), 5000),
        );
        chk(cn, &cb, rn, &rb, "fresh streamDecode first block");
        assert_eq!(cn, 5000);
        assert_bytes_eq(&cb[..5000], &src, "fresh streamDecode data");
        fsd.0(cs);
        fsd.1(rs);

        // ---- LZ4_decoderRingBufferSize value sweep
        let mut sizes: Vec<c_int> = vec![
            i32::MIN,
            i32::MIN + 1,
            -70_000,
            -16,
            -1,
            0,
            1,
            14,
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
        for _ in 0..500 {
            sizes.push(rng.next_u32() as c_int);
        }
        for &s in &sizes {
            assert_ret_eq(
                drbs.0(s),
                drbs.1(s),
                &format!("LZ4_decoderRingBufferSize({s})"),
            );
        }
    }
}
