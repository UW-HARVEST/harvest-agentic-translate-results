//! CONFIGS.md rows 1-11 — xxHash (namespaced `LZ4_XXH*`) valid-path parity.
//! ERRORS.md rows 110-124 are covered in `errors.rs`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_int, c_void};

type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
type FnXXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
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
// Row 11 — LZ4_XXH_versionNumber
// ---------------------------------------------------------------------------
#[test]
fn row11_version_number() {
    sym!(f, "LZ4_XXH_versionNumber", unsafe extern "C" fn() -> c_int);
    let (c, r) = unsafe { (f.0(), f.1()) };
    assert_ret_eq(c, r, "LZ4_XXH_versionNumber");
    assert_eq!(c, 605, "xxHash 0.6.5 expected");
}

// ---------------------------------------------------------------------------
// Rows 1-4 — one-shot hashes over a length sweep and several seeds
// ---------------------------------------------------------------------------
#[test]
fn rows1_2_oneshot_len_sweep() {
    sym!(h32, "LZ4_XXH32", FnXXH32);
    sym!(h64, "LZ4_XXH64", FnXXH64);
    let mut rng = Rng::new(0x5EED_0001);
    let seeds32: [u32; 5] = [0, 1, 0x9E37_79B1, 0x7FFF_FFFF, u32::MAX];
    let seeds64: [u64; 5] = [0, 1, 0x9E37_79B1_85EB_CA87, 0x7FFF_FFFF_FFFF_FFFF, u64::MAX];

    for len in 0..=300usize {
        for &shape in ALL_SHAPES {
            let data = gen_data(shape, len, &mut rng);
            let p = data.as_ptr() as *const c_void;
            for &s in seeds32.iter() {
                let (c, r) = unsafe { (h32.0(p, len, s), h32.1(p, len, s)) };
                assert_ret_eq(c, r, &format!("XXH32 len={len} shape={shape:?} seed={s:#x}"));
            }
            for &s in seeds64.iter() {
                let (c, r) = unsafe { (h64.0(p, len, s), h64.1(p, len, s)) };
                assert_ret_eq(c, r, &format!("XXH64 len={len} shape={shape:?} seed={s:#x}"));
            }
        }
    }
}

#[test]
fn rows3_4_oneshot_large() {
    sym!(h32, "LZ4_XXH32", FnXXH32);
    sym!(h64, "LZ4_XXH64", FnXXH64);
    let mut rng = Rng::new(0x5EED_0002);
    for &len in &[1024usize, 4096, 16384, 65536, 100_000, 262_144] {
        for &shape in ALL_SHAPES {
            let data = gen_data(shape, len, &mut rng);
            let p = data.as_ptr() as *const c_void;
            for &s in &[0u32, 12345, u32::MAX] {
                let (c, r) = unsafe { (h32.0(p, len, s), h32.1(p, len, s)) };
                assert_ret_eq(c, r, &format!("XXH32 large len={len} {shape:?} seed={s}"));
            }
            for &s in &[0u64, 12345, u64::MAX] {
                let (c, r) = unsafe { (h64.0(p, len, s), h64.1(p, len, s)) };
                assert_ret_eq(c, r, &format!("XXH64 large len={len} {shape:?} seed={s}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5-6 — streaming with FIXED chunk sizes (crosses the 16/32-byte buffer)
// ---------------------------------------------------------------------------
#[test]
fn rows5_6_streaming_fixed_chunks() {
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
    sym!(h32, "LZ4_XXH32", FnXXH32);
    sym!(h64, "LZ4_XXH64", FnXXH64);

    let mut rng = Rng::new(0x5EED_0003);
    let data = gen_data(Shape::Texty, 4096, &mut rng);

    unsafe {
        let (cs32, rs32p) = (cr32.0(), cr32.1());
        let (cs64, rs64p) = (cr64.0(), cr64.1());
        assert!(!cs32.is_null() && !rs32p.is_null() && !cs64.is_null() && !rs64p.is_null());

        for chunk in 1..=40usize {
            for &seed in &[0u32, 0xABCD_1234] {
                assert_ret_eq(rs32.0(cs32, seed), rs32.1(rs32p, seed), "XXH32_reset");
                assert_ret_eq(
                    rs64.0(cs64, seed as u64),
                    rs64.1(rs64p, seed as u64),
                    "XXH64_reset",
                );
                let mut off = 0;
                while off < data.len() {
                    let n = chunk.min(data.len() - off);
                    let p = data[off..].as_ptr() as *const c_void;
                    assert_ret_eq(
                        up32.0(cs32, p, n),
                        up32.1(rs32p, p, n),
                        &format!("XXH32_update chunk={chunk} off={off}"),
                    );
                    assert_ret_eq(
                        up64.0(cs64, p, n),
                        up64.1(rs64p, p, n),
                        &format!("XXH64_update chunk={chunk} off={off}"),
                    );
                    off += n;
                }
                let (c32, r32) = (dg32.0(cs32), dg32.1(rs32p));
                let (c64, r64) = (dg64.0(cs64), dg64.1(rs64p));
                assert_ret_eq(c32, r32, &format!("XXH32_digest chunk={chunk}"));
                assert_ret_eq(c64, r64, &format!("XXH64_digest chunk={chunk}"));
                // Streaming must also equal the one-shot hash.
                let one32 = h32.0(data.as_ptr() as *const c_void, data.len(), seed);
                let one64 = h64.0(data.as_ptr() as *const c_void, data.len(), seed as u64);
                assert_eq!(c32, one32, "XXH32 streaming != one-shot chunk={chunk}");
                assert_eq!(c64, one64, "XXH64 streaming != one-shot chunk={chunk}");
            }
        }
        assert_ret_eq(fr32.0(cs32), fr32.1(rs32p), "XXH32_freeState");
        assert_ret_eq(fr64.0(cs64), fr64.1(rs64p), "XXH64_freeState");
    }
}

// ---------------------------------------------------------------------------
// Row 7 — streaming with RANDOM chunk sizes + repeated mid-stream digest()
// ---------------------------------------------------------------------------
#[test]
fn row7_streaming_random_chunks_and_midstream_digest() {
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

    let mut rng = Rng::new(0x5EED_0004);
    unsafe {
        let (cs32, rp32) = (cr32.0(), cr32.1());
        let (cs64, rp64) = (cr64.0(), cr64.1());
        for iter in 0..200 {
            let len = rng.range(0, 5000);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let data = gen_data(shape, len, &mut rng);
            let seed = rng.next_u32();
            assert_ret_eq(rs32.0(cs32, seed), rs32.1(rp32, seed), "reset32");
            assert_ret_eq(
                rs64.0(cs64, seed as u64 | ((seed as u64) << 32)),
                rs64.1(rp64, seed as u64 | ((seed as u64) << 32)),
                "reset64",
            );
            let mut off = 0;
            while off < len {
                let n = rng.range(0, 70).min(len - off);
                let p = data[off..].as_ptr() as *const c_void;
                assert_ret_eq(up32.0(cs32, p, n), up32.1(rp32, p, n), "update32");
                assert_ret_eq(up64.0(cs64, p, n), up64.1(rp64, p, n), "update64");
                off += n;
                // Mid-stream digest must not disturb the state.
                if rng.below(4) == 0 {
                    assert_ret_eq(
                        dg32.0(cs32),
                        dg32.1(rp32),
                        &format!("mid digest32 iter={iter} off={off}"),
                    );
                    assert_ret_eq(
                        dg64.0(cs64),
                        dg64.1(rp64),
                        &format!("mid digest64 iter={iter} off={off}"),
                    );
                }
                if n == 0 {
                    break;
                }
            }
            assert_ret_eq(dg32.0(cs32), dg32.1(rp32), &format!("digest32 iter={iter}"));
            assert_ret_eq(dg64.0(cs64), dg64.1(rp64), &format!("digest64 iter={iter}"));
        }
        fr32.0(cs32);
        fr32.1(rp32);
        fr64.0(cs64);
        fr64.1(rp64);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — copyState mid-stream; both copies must continue identically
// ---------------------------------------------------------------------------
#[test]
fn row8_copy_state() {
    sym!(cr32, "LZ4_XXH32_createState", FnCreate);
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(rs32, "LZ4_XXH32_reset", FnReset32);
    sym!(up32, "LZ4_XXH32_update", FnUpdate);
    sym!(dg32, "LZ4_XXH32_digest", FnDigest32);
    sym!(cp32, "LZ4_XXH32_copyState", FnCopy);
    sym!(cr64, "LZ4_XXH64_createState", FnCreate);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    sym!(rs64, "LZ4_XXH64_reset", FnReset64);
    sym!(up64, "LZ4_XXH64_update", FnUpdate);
    sym!(dg64, "LZ4_XXH64_digest", FnDigest64);
    sym!(cp64, "LZ4_XXH64_copyState", FnCopy);

    let mut rng = Rng::new(0x5EED_0005);
    unsafe {
        for iter in 0..50 {
            let (a32c, a32r) = (cr32.0(), cr32.1());
            let (b32c, b32r) = (cr32.0(), cr32.1());
            let (a64c, a64r) = (cr64.0(), cr64.1());
            let (b64c, b64r) = (cr64.0(), cr64.1());
            let seed = rng.next_u32();
            rs32.0(a32c, seed);
            rs32.1(a32r, seed);
            rs64.0(a64c, seed as u64);
            rs64.1(a64r, seed as u64);

            let first = gen_data(Shape::Random, rng.range(0, 200), &mut rng);
            up32.0(a32c, first.as_ptr() as *const c_void, first.len());
            up32.1(a32r, first.as_ptr() as *const c_void, first.len());
            up64.0(a64c, first.as_ptr() as *const c_void, first.len());
            up64.1(a64r, first.as_ptr() as *const c_void, first.len());

            // Snapshot.
            cp32.0(b32c, a32c);
            cp32.1(b32r, a32r);
            cp64.0(b64c, a64c);
            cp64.1(b64r, a64r);

            let second = gen_data(Shape::Texty, rng.range(0, 200), &mut rng);
            for (s, is_c) in [(a32c, true), (b32c, true)] {
                let _ = is_c;
                up32.0(s, second.as_ptr() as *const c_void, second.len());
            }
            for s in [a32r, b32r] {
                up32.1(s, second.as_ptr() as *const c_void, second.len());
            }
            for s in [a64c, b64c] {
                up64.0(s, second.as_ptr() as *const c_void, second.len());
            }
            for s in [a64r, b64r] {
                up64.1(s, second.as_ptr() as *const c_void, second.len());
            }

            let (ca, ra) = (dg32.0(a32c), dg32.1(a32r));
            let (cb, rb) = (dg32.0(b32c), dg32.1(b32r));
            assert_ret_eq(ca, ra, &format!("copyState orig32 iter={iter}"));
            assert_ret_eq(cb, rb, &format!("copyState copy32 iter={iter}"));
            assert_eq!(ca, cb, "copy must track the original (C, 32)");
            let (ca, ra) = (dg64.0(a64c), dg64.1(a64r));
            let (cb, rb) = (dg64.0(b64c), dg64.1(b64r));
            assert_ret_eq(ca, ra, &format!("copyState orig64 iter={iter}"));
            assert_ret_eq(cb, rb, &format!("copyState copy64 iter={iter}"));
            assert_eq!(ca, cb, "copy must track the original (C, 64)");

            for s in [a32c, b32c] {
                fr32.0(s);
            }
            for s in [a32r, b32r] {
                fr32.1(s);
            }
            for s in [a64c, b64c] {
                fr64.0(s);
            }
            for s in [a64r, b64r] {
                fr64.1(s);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — create/free lifecycle
// ---------------------------------------------------------------------------
#[test]
fn row9_state_lifecycle() {
    sym!(cr32, "LZ4_XXH32_createState", FnCreate);
    sym!(fr32, "LZ4_XXH32_freeState", FnFree);
    sym!(cr64, "LZ4_XXH64_createState", FnCreate);
    sym!(fr64, "LZ4_XXH64_freeState", FnFree);
    unsafe {
        for _ in 0..100 {
            let (a, b) = (cr32.0(), cr32.1());
            assert!(!a.is_null() && !b.is_null(), "createState returned NULL");
            assert_ret_eq(fr32.0(a), fr32.1(b), "XXH32_freeState");
            let (a, b) = (cr64.0(), cr64.1());
            assert!(!a.is_null() && !b.is_null(), "createState returned NULL");
            assert_ret_eq(fr64.0(a), fr64.1(b), "XXH64_freeState");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — canonical (big-endian) representation round trip
// ---------------------------------------------------------------------------
#[test]
fn row10_canonical_roundtrip() {
    sym!(c32, "LZ4_XXH32_canonicalFromHash", FnCanon32);
    sym!(f32s, "LZ4_XXH32_hashFromCanonical", FnFromCanon32);
    sym!(c64, "LZ4_XXH64_canonicalFromHash", FnCanon64);
    sym!(f64s, "LZ4_XXH64_hashFromCanonical", FnFromCanon64);

    let mut rng = Rng::new(0x5EED_0006);
    unsafe {
        let mut h32vals: Vec<u32> = vec![0, 1, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
        let mut h64vals: Vec<u64> = vec![0, 1, 0x7FFF_FFFF_FFFF_FFFF, 1 << 63, u64::MAX];
        for _ in 0..500 {
            h32vals.push(rng.next_u32());
            h64vals.push(rng.next_u64());
        }
        for &h in &h32vals {
            let mut cb = [0u8; 4];
            let mut rb = [0u8; 4];
            c32.0(cb.as_mut_ptr() as *mut c_void, h);
            c32.1(rb.as_mut_ptr() as *mut c_void, h);
            assert_bytes_eq(&cb, &rb, &format!("XXH32_canonicalFromHash {h:#x}"));
            assert_eq!(cb, h.to_be_bytes(), "canonical must be big-endian");
            let (cv, rv) = (
                f32s.0(cb.as_ptr() as *const c_void),
                f32s.1(rb.as_ptr() as *const c_void),
            );
            assert_ret_eq(cv, rv, &format!("XXH32_hashFromCanonical {h:#x}"));
            assert_eq!(cv, h, "round trip must be the identity");
        }
        for &h in &h64vals {
            let mut cb = [0u8; 8];
            let mut rb = [0u8; 8];
            c64.0(cb.as_mut_ptr() as *mut c_void, h);
            c64.1(rb.as_mut_ptr() as *mut c_void, h);
            assert_bytes_eq(&cb, &rb, &format!("XXH64_canonicalFromHash {h:#x}"));
            assert_eq!(cb, h.to_be_bytes(), "canonical must be big-endian");
            let (cv, rv) = (
                f64s.0(cb.as_ptr() as *const c_void),
                f64s.1(rb.as_ptr() as *const c_void),
            );
            assert_ret_eq(cv, rv, &format!("XXH64_hashFromCanonical {h:#x}"));
            assert_eq!(cv, h, "round trip must be the identity");
        }
    }
}
