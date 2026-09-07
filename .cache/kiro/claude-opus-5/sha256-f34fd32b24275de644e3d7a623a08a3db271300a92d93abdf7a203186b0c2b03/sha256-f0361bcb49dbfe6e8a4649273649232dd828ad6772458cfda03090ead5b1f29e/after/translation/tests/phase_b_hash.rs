//! Phase B — `CONFIGS.md` rows 1..15: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed` and the global seed evolution.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn hb(buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    let (c, r) = libs();
    unsafe {
        (
            (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
            (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
        )
    }
}

fn hs(s: &mut [u8], seed: usize) -> (usize, usize) {
    let (c, r) = libs();
    unsafe {
        (
            (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
            (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
        )
    }
}

const SEEDS: [usize; 6] = [
    0,
    1,
    0x31415926,
    usize::MAX,
    0x0123_4567_89ab_cdef,
    0x8000_0000_0000_0000,
];

/// row 1 — `len == 0`
#[test]
fn row01_hash_bytes_len0() {
    let _g = serial();
    let mut buf = [0u8; 8];
    for &s in &SEEDS {
        let (a, b) = hb(&mut buf, 0, s);
        assert_eq!(a, b, "hash_bytes(len=0, seed={s:#x})");
    }
    // NULL pointer with len 0 is never dereferenced by the C code.
    let (c, r) = libs();
    unsafe {
        for &s in &SEEDS {
            assert_eq!(
                (c.hash_bytes)(std::ptr::null_mut(), 0, s),
                (r.hash_bytes)(std::ptr::null_mut(), 0, s),
                "hash_bytes(NULL, 0, {s:#x})"
            );
        }
    }
}

/// row 2 — `len = 1..7` (switch arms 1..7, no full block), random bytes
#[test]
fn row02_hash_bytes_len1_7() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0002);
    for len in 1..8usize {
        for &s in &SEEDS {
            for _ in 0..400 {
                let mut buf = rng.bytes(8);
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "len={len} seed={s:#x} buf={buf:?}");
            }
        }
    }
}

/// row 3 — exactly one full block
#[test]
fn row03_hash_bytes_len8() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0003);
    for &s in &SEEDS {
        for _ in 0..2000 {
            let mut buf = rng.bytes(8);
            let (a, b) = hb(&mut buf, 8, s);
            assert_eq!(a, b, "len=8 seed={s:#x} buf={buf:?}");
        }
    }
}

/// row 4 — one full block plus a 1..7 byte tail
#[test]
fn row04_hash_bytes_len9_15() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0004);
    for len in 9..16usize {
        for &s in &SEEDS {
            for _ in 0..300 {
                let mut buf = rng.bytes(16);
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "len={len} seed={s:#x}");
            }
        }
    }
}

/// row 5 — many full blocks (multiples of 8 up to 256)
#[test]
fn row05_hash_bytes_multiples_of_8() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0005);
    for k in 2..=32usize {
        let len = k * 8;
        for &s in &SEEDS {
            for _ in 0..40 {
                let mut buf = rng.bytes(len);
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "len={len} seed={s:#x}");
            }
        }
    }
}

/// row 6 — large, non multiple of 8
#[test]
fn row06_hash_bytes_large_odd() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0006);
    for len in [17usize, 63, 100, 255, 999, 1000, 4095, 4097, 8191] {
        for &s in &SEEDS {
            for _ in 0..20 {
                let mut buf = rng.bytes(len);
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "len={len} seed={s:#x}");
            }
        }
    }
}

/// row 7 — saturated sign-extension patterns
#[test]
fn row07_hash_bytes_sign_extension() {
    let _g = serial();
    for pat in [0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
        for len in 0..40usize {
            let mut buf = vec![pat; 40];
            for &s in &SEEDS {
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "pat={pat:#x} len={len} seed={s:#x}");
            }
        }
    }
    // Every single high-bit byte position in isolation (the `d[i] << 24` /
    // `(size_t) d[i] << 16 << 16` asymmetry in the fall-through switch).
    for pos in 0..8usize {
        for len in 1..8usize {
            let mut buf = [0u8; 8];
            buf[pos] = 0x80;
            for &s in &SEEDS {
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "pos={pos} len={len} seed={s:#x}");
            }
            buf[pos] = 0xff;
            for &s in &SEEDS {
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "pos={pos} len={len} seed={s:#x} (0xff)");
            }
        }
    }
}

/// row 8 — random 64-bit seeds
#[test]
fn row08_hash_bytes_random_seeds() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0008);
    for _ in 0..4000 {
        let len = rng.below(70);
        let mut buf = rng.bytes(len.max(1));
        let seed = rng.next_u64() as usize;
        let (a, b) = hb(&mut buf, len, seed);
        assert_eq!(a, b, "len={len} seed={seed:#x}");
    }
}

/// row 9 — empty string
#[test]
fn row09_hash_string_empty() {
    let _g = serial();
    let mut s = cstr(b"");
    for &sd in &SEEDS {
        let (a, b) = hs(&mut s, sd);
        assert_eq!(a, b, "hash_string(\"\", {sd:#x})");
    }
}

/// row 10 — single byte, every possible non-NUL value
#[test]
fn row10_hash_string_single_byte() {
    let _g = serial();
    for ch in 1u8..=255 {
        let mut s = cstr(&[ch]);
        for &sd in &SEEDS {
            let (a, b) = hs(&mut s, sd);
            assert_eq!(a, b, "hash_string({ch:#x}, {sd:#x})");
        }
    }
}

/// row 11 — random ASCII, length 1..64
#[test]
fn row11_hash_string_random_ascii() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0011);
    for _ in 0..4000 {
        let n = 1 + rng.below(64);
        let mut s = cstr(&rng.ascii(n));
        let sd = rng.next_u64() as usize;
        let (a, b) = hs(&mut s, sd);
        assert_eq!(a, b, "len={n} seed={sd:#x}");
    }
}

/// row 12 — bytes over the whole 0x01..0xFF range (the `(unsigned char)` cast)
#[test]
fn row12_hash_string_high_bytes() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0012);
    for _ in 0..4000 {
        let n = 1 + rng.below(40);
        let mut s = cstr(&rng.nonzero(n));
        let sd = rng.next_u64() as usize;
        let (a, b) = hs(&mut s, sd);
        assert_eq!(a, b, "len={n} seed={sd:#x}");
    }
}

/// row 13 — long strings
#[test]
fn row13_hash_string_long() {
    let _g = serial();
    let mut rng = Rng::new(0xB2_0013);
    for n in [255usize, 512, 1024, 4096] {
        for _ in 0..20 {
            let mut s = cstr(&rng.nonzero(n));
            for &sd in &SEEDS {
                let (a, b) = hs(&mut s, sd);
                assert_eq!(a, b, "len={n} seed={sd:#x}");
            }
        }
    }
}

/// row 14 — fixed extreme seeds against a fixed corpus
#[test]
fn row14_hash_string_extreme_seeds() {
    let _g = serial();
    let corpus: [&[u8]; 8] = [
        b"", b"a", b"ab", b"abc", b"test_0", b"test_123456789",
        b"\xff\xfe\xfd", b"the quick brown fox jumps over the lazy dog",
    ];
    for body in corpus {
        let mut s = cstr(body);
        for &sd in &SEEDS {
            let (a, b) = hs(&mut s, sd);
            assert_eq!(a, b, "body={body:?} seed={sd:#x}");
        }
    }
}

/// row 15 — global-seed evolution: every `make_hash_index(_, NULL)` advances
/// `stbds_hash_seed` (`seed = seed*a + b`).  The per-table `seed` field is the
/// pre-update value, so a chain of `shmode_func` calls exposes the whole
/// sequence.
#[test]
fn row15_seed_evolution() {
    let _g = serial();
    for &start in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0xdead_beef_cafe_babe] {
        reseed(start);
        let (lc, lr) = libs();
        let mut cs = Vec::new();
        let mut rs = Vec::new();
        unsafe {
            for _ in 0..40 {
                let a = (lc.shmode_func)(16, STBDS_SH_STRDUP);
                let b = (lr.shmode_func)(16, STBDS_SH_STRDUP);
                let ta = (*header_of(a, 16)).hash_table as *mut HashIndex;
                let tb = (*header_of(b, 16)).hash_table as *mut HashIndex;
                cs.push((*ta).seed);
                rs.push((*tb).seed);
                assert_eq!(table_snap(ta), table_snap(tb), "fresh table (start={start:#x})");
                (lc.hmfree_func)((a as *mut u8).sub(16) as *mut c_void, 16);
                (lr.hmfree_func)((b as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
        assert_eq!(cs, rs, "seed chain diverged for start={start:#x}");
        // The chain must actually move (guards against a no-op translation).
        assert!(cs.windows(2).any(|w| w[0] != w[1]), "seed never changed");
    }
    reseed(DEFAULT_SEED);
}
