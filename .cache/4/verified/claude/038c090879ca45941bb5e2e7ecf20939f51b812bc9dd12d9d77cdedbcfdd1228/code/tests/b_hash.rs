//! Phase B — CONFIGS.md rows 1..9: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed`.
//!
//! Lowest level of the library: everything else is built on these, so they are
//! verified first, with many randomized inputs per configuration.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: [usize; 8] = [
    0,
    1,
    2,
    0x3141_5926,
    0xFFFF_FFFF,
    usize::MAX,
    0x8000_0000_0000_0000,
    0x0123_4567_89AB_CDEF,
];

fn hash_bytes_both(p: &Pair, buf: &mut [u8], len: usize, seed: usize) {
    let (hc, hr) = unsafe {
        (
            (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
            (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
        )
    };
    assert_eq!(
        hc, hr,
        "stbds_hash_bytes diverged: len={len} seed={seed:#x} bytes={:x?}",
        &buf[..len]
    );
}

/// Row 1 — `len == 0`, NULL and non-NULL `p`, all seeds.
#[test]
fn cfg_01_hash_bytes_len0() {
    let p = Pair::new();
    for &s in SEEDS.iter() {
        let (hc, hr) = unsafe {
            (
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, s),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, s),
            )
        };
        assert_eq!(hc, hr, "hash_bytes(NULL,0,{s:#x})");
    }
    let mut rng = Rng::new(0xC0FFEE_01);
    let mut buf = [0u8; 8];
    for _ in 0..64 {
        let s = rng.next_u64() as usize;
        hash_bytes_both(&p, &mut buf, 0, s);
    }
}

/// Row 2 — `len = 1..7`: every `switch (len - i)` fall-through case.
#[test]
fn cfg_02_hash_bytes_tail_1_to_7() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_02);
    for len in 1..=7usize {
        for _ in 0..256 {
            let mut buf = rng.bytes(8);
            let s = if rng.next_u64() % 4 == 0 {
                SEEDS[rng.below(SEEDS.len())]
            } else {
                rng.next_u64() as usize
            };
            hash_bytes_both(&p, &mut buf, len, s);
        }
    }
}

/// Row 3 — `len = 1..7` with the high bit set in byte 3 / byte 6, exercising the
/// `(d[3] << 24)` int-promotion sign extension and `((size_t)d[6] << 24) << 24`.
#[test]
fn cfg_03_hash_bytes_tail_high_bit() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_03);
    for len in 1..=7usize {
        for _ in 0..256 {
            let mut buf = rng.bytes(8);
            buf[3] |= 0x80;
            buf[6] |= 0x80;
            buf[5] |= 0x80;
            buf[4] |= 0x80;
            hash_bytes_both(&p, &mut buf, len, rng.next_u64() as usize);
        }
        // and the extremes
        for pat in [0x00u8, 0x7F, 0x80, 0xFF] {
            let mut buf = [pat; 8];
            for &s in SEEDS.iter() {
                hash_bytes_both(&p, &mut buf, len, s);
            }
        }
    }
}

/// Row 4 — `len == 8`: exactly one siphash block, `len - i == 0`.
#[test]
fn cfg_04_hash_bytes_len8() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_04);
    for _ in 0..512 {
        let mut buf = rng.bytes(8);
        hash_bytes_both(&p, &mut buf, 8, rng.next_u64() as usize);
    }
    // sign-extension of d[3] and d[7] inside the loop body
    for i in 0..8 {
        for pat in [0x80u8, 0xFF] {
            let mut buf = [0u8; 8];
            buf[i] = pat;
            for &s in SEEDS.iter() {
                hash_bytes_both(&p, &mut buf, 8, s);
            }
        }
    }
}

/// Row 5 — `len = 9..15`: one full block plus each remainder.
#[test]
fn cfg_05_hash_bytes_len9_15() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_05);
    for len in 9..=15usize {
        for _ in 0..128 {
            let mut buf = rng.bytes(16);
            hash_bytes_both(&p, &mut buf, len, rng.next_u64() as usize);
        }
        for &s in SEEDS.iter() {
            let mut buf = vec![0xFFu8; 16];
            hash_bytes_both(&p, &mut buf, len, s);
            let mut buf = vec![0x00u8; 16];
            hash_bytes_both(&p, &mut buf, len, s);
        }
    }
}

/// Row 6 — exact multiples of `sizeof(size_t)`.
#[test]
fn cfg_06_hash_bytes_multiples() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_06);
    for len in [16usize, 24, 32, 40, 64, 128, 256] {
        for _ in 0..64 {
            let mut buf = rng.bytes(len);
            hash_bytes_both(&p, &mut buf, len, rng.next_u64() as usize);
        }
    }
}

/// Row 7 — `len = 17..64` random plus all-`0x00` / all-`0xFF` payloads.
#[test]
fn cfg_07_hash_bytes_len17_64() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_07);
    for len in 17..=64usize {
        for _ in 0..24 {
            let mut buf = rng.bytes(len);
            hash_bytes_both(&p, &mut buf, len, rng.next_u64() as usize);
        }
        let mut z = vec![0x00u8; len];
        let mut f = vec![0xFFu8; len];
        let mut h = vec![0x80u8; len];
        for &s in SEEDS.iter() {
            hash_bytes_both(&p, &mut z, len, s);
            hash_bytes_both(&p, &mut f, len, s);
            hash_bytes_both(&p, &mut h, len, s);
        }
    }
}

/// Row 8 — `stbds_hash_string`: empty, 1 char, long, and bytes >= 0x80.
#[test]
fn cfg_08_hash_string() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_08);

    let check = |bytes: &[u8], seed: usize| {
        let mut v = bytes.to_vec();
        v.push(0);
        let (hc, hr) = unsafe {
            (
                (p.c.hash_string)(v.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(v.as_mut_ptr() as *mut c_char, seed),
            )
        };
        assert_eq!(
            hc, hr,
            "stbds_hash_string diverged: seed={seed:#x} s={bytes:x?}"
        );
    };

    for &s in SEEDS.iter() {
        check(b"", s);
        check(b"a", s);
        check(b"test_0", s);
        check(b"test_-2147483648", s);
        check(&[0xFFu8], s);
        check(&[0x80u8, 0x81, 0xFE, 0xFF], s);
        check(&vec![0xAAu8; 300], s);
    }

    for len in 1..=64usize {
        for _ in 0..24 {
            // printable ASCII
            let ascii: Vec<u8> = (0..len).map(|_| 0x21 + (rng.next_u64() % 0x5E) as u8).collect();
            check(&ascii, rng.next_u64() as usize);
            // arbitrary non-zero bytes (covers the (unsigned char) cast of a
            // negative `char`)
            let raw: Vec<u8> = (0..len)
                .map(|_| {
                    let b = rng.next_u64() as u8;
                    if b == 0 {
                        1
                    } else {
                        b
                    }
                })
                .collect();
            check(&raw, rng.next_u64() as usize);
        }
    }
}

/// Row 9 — `stbds_rand_seed`: the global seed is a pure input; setting it must
/// not perturb `stbds_hash_bytes`/`stbds_hash_string` (which take `seed`
/// explicitly), and the two sides must stay in lock-step.
#[test]
fn cfg_09_rand_seed_is_pure_for_hashes() {
    let p = Pair::new();
    let mut rng = Rng::new(0xC0FFEE_09);
    let mut buf = rng.bytes(32);
    for &g in SEEDS.iter() {
        p.seed(g);
        for &s in SEEDS.iter() {
            hash_bytes_both(&p, &mut buf, 32, s);
        }
    }
    // ...and the seed really is observable through a fresh table (row 51 covers
    // the full lock-step case).
    for &g in SEEDS.iter() {
        p.seed(g);
        let (hc, hr) = unsafe { ((p.c.shmode_func)(16, STBDS_SH_ARENA), (p.r.shmode_func)(16, STBDS_SH_ARENA)) };
        let (sc, sr) = unsafe {
            (
                snap_map(hc, 16, KeyKind::Binary, false),
                snap_map(hr, 16, KeyKind::Binary, false),
            )
        };
        eq_snap(&format!("shmode_func after rand_seed({g:#x})"), &sc, &sr);
        assert_eq!(sc.seed, g, "table seed should be the global seed");
        unsafe {
            (p.c.hmfree_func)((hc as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((hr as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

/// Exhaustive reinforcement of rows 2..7: for every length `0..=17` and every
/// byte position, sweep all 256 byte values. This covers the `int` promotion /
/// sign-extension of `d[3] << 24` and `d[7] << 24` in *every* position, both in
/// the loop body and in each `switch (len - i)` fall-through case.
#[test]
fn cfg_02_07_hash_bytes_exhaustive_bytes() {
    let p = Pair::new();
    for len in 0..=17usize {
        for pos in 0..len {
            for b in 0..=255u8 {
                let mut buf = vec![0x5Au8; 24];
                buf[pos] = b;
                hash_bytes_both(&p, &mut buf, len, 0x3141_5926);
            }
        }
    }
    // ...and with an all-0xFF background so the OR-ed bits differ
    for len in 0..=17usize {
        for pos in 0..len {
            for b in [0x00u8, 0x01, 0x7F, 0x80, 0x81, 0xFE, 0xFF] {
                let mut buf = vec![0xFFu8; 24];
                buf[pos] = b;
                for &s in SEEDS.iter() {
                    hash_bytes_both(&p, &mut buf, len, s);
                }
            }
        }
    }
}

/// Exhaustive reinforcement of row 8: every single-byte string (1..=255, 0 would
/// terminate it), every two-byte string over a representative alphabet, and the
/// full byte range in each position of a 4-byte string.
#[test]
fn cfg_08_hash_string_exhaustive_bytes() {
    let p = Pair::new();
    let check = |bytes: &[u8], seed: usize| {
        let mut v = bytes.to_vec();
        v.push(0);
        let (hc, hr) = unsafe {
            (
                (p.c.hash_string)(v.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(v.as_mut_ptr() as *mut c_char, seed),
            )
        };
        assert_eq!(hc, hr, "hash_string diverged: seed={seed:#x} s={bytes:x?}");
    };

    // every single-byte string, every seed
    for b in 1..=255u8 {
        for &s in SEEDS.iter() {
            check(&[b], s);
        }
    }
    // every byte value in each position of a 4-byte string (covers the
    // `(unsigned char) *str` cast for bytes that are negative as `char`)
    for pos in 0..4usize {
        for b in 1..=255u8 {
            let mut v = [0x41u8; 4];
            v[pos] = b;
            check(&v, 0x3141_5926);
            check(&v, usize::MAX);
        }
    }
    // all 2-byte strings over the extremes
    for a in [1u8, 0x7F, 0x80, 0xFF] {
        for b in 1..=255u8 {
            check(&[a, b], 0);
            check(&[b, a], 1);
        }
    }
}
