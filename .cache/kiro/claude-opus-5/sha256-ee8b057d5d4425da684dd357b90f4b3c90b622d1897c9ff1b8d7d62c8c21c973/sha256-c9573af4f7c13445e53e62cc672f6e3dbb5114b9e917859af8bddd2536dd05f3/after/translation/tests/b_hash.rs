//! Phase B — CONFIGS.md rows 1..15: the two pure hash functions.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

fn hb(l: &Pair, buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        (
            (l.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
            (l.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed),
        )
    }
}

fn hs(l: &Pair, s: &mut [u8], seed: usize) -> (usize, usize) {
    assert_eq!(*s.last().unwrap(), 0, "must be NUL terminated");
    unsafe {
        (
            (l.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
            (l.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
        )
    }
}

const SEEDS: [usize; 8] = [
    0,
    1,
    2,
    0x3141_5926,
    0xFFFF_FFFF,
    0x8000_0000_0000_0000,
    usize::MAX,
    0xDEAD_BEEF_CAFE_BABE,
];

/// rows 1-6: every length class, randomized bytes and seeds.
#[test]
fn row_1_6_hash_bytes_all_lengths() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0001);
    for len in 0..=136usize {
        for _ in 0..40 {
            let mut buf = rng.bytes(len.max(1));
            let seed = if rng.below(2) == 0 {
                SEEDS[rng.below(SEEDS.len())]
            } else {
                rng.next_u64() as usize
            };
            let (a, b) = hb(l, &mut buf, len, seed);
            assert_eq!(a, b, "hash_bytes len={len} seed={seed:#x} buf={buf:?}");
        }
    }
}

/// row 7: all-zero buffer, every len 0..64.
#[test]
fn row_7_hash_bytes_all_zero() {
    let (l, _g) = libs();
    let mut buf = vec![0u8; 96];
    for len in 0..=64usize {
        for &seed in SEEDS.iter() {
            let (a, b) = hb(l, &mut buf, len, seed);
            assert_eq!(a, b, "zeros len={len} seed={seed:#x}");
        }
    }
}

/// row 8: all-0xff buffer, every len 0..64.
#[test]
fn row_8_hash_bytes_all_ff() {
    let (l, _g) = libs();
    let mut buf = vec![0xffu8; 96];
    for len in 0..=64usize {
        for &seed in SEEDS.iter() {
            let (a, b) = hb(l, &mut buf, len, seed);
            assert_eq!(a, b, "0xff len={len} seed={seed:#x}");
        }
    }
}

/// row 9: the `int` sign-extension quirk — byte 3 / byte 7 of each 8-byte
/// group with the high bit set.  This is the branch most likely to diverge.
#[test]
fn row_9_hash_bytes_sign_extension() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0009);
    for len in 0..=72usize {
        for trial in 0..64 {
            let mut buf = rng.bytes(len.max(1));
            // Force the high bit in every byte at index %8 == 3 and/or 7.
            for (i, b) in buf.iter_mut().enumerate() {
                match i % 8 {
                    3 => {
                        if trial & 1 != 0 {
                            *b |= 0x80
                        } else {
                            *b &= 0x7f
                        }
                    }
                    7 => {
                        if trial & 2 != 0 {
                            *b |= 0x80
                        } else {
                            *b &= 0x7f
                        }
                    }
                    _ => {}
                }
            }
            let seed = SEEDS[trial % SEEDS.len()];
            let (a, b) = hb(l, &mut buf, len, seed);
            assert_eq!(a, b, "signext len={len} trial={trial} buf={buf:?}");
        }
        // Also the exhaustive single-byte-value sweep for the `case 4` tail.
        if len >= 4 {
            for v in 0..=255u8 {
                let mut buf = vec![0u8; len];
                buf[3] = v;
                if len >= 8 {
                    buf[7] = v;
                }
                let (a, b) = hb(l, &mut buf, len, 0x3141_5926);
                assert_eq!(a, b, "signext sweep len={len} v={v}");
            }
        }
    }
}

/// row 10: seed sweep on a fixed buffer.
#[test]
fn row_10_hash_bytes_seed_sweep() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0010);
    let mut buf = rng.bytes(37);
    for &seed in SEEDS.iter() {
        let (a, b) = hb(l, &mut buf, 37, seed);
        assert_eq!(a, b, "seed={seed:#x}");
    }
    for i in 0..64 {
        // single-bit seeds
        let seed = 1usize << i;
        let (a, b) = hb(l, &mut buf, 37, seed);
        assert_eq!(a, b, "seed bit {i}");
    }
    for _ in 0..2000 {
        let seed = rng.next_u64() as usize;
        let (a, b) = hb(l, &mut buf, 37, seed);
        assert_eq!(a, b, "random seed {seed:#x}");
    }
}

/// rows 11-12: hash_string, empty and random ASCII.
#[test]
fn row_11_12_hash_string_ascii() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0011);
    let mut empty = vec![0u8];
    for &seed in SEEDS.iter() {
        let (a, b) = hs(l, &mut empty, seed);
        assert_eq!(a, b, "empty seed={seed:#x}");
    }
    for n in 0..=40usize {
        for _ in 0..40 {
            let mut s = rng.cstring(n);
            let seed = if rng.below(2) == 0 {
                SEEDS[rng.below(SEEDS.len())]
            } else {
                rng.next_u64() as usize
            };
            let (a, b) = hs(l, &mut s, seed);
            assert_eq!(a, b, "hash_string n={n} seed={seed:#x} s={s:?}");
        }
    }
}

/// row 13: high-bit bytes (`(unsigned char)` promotion, no sign extension).
#[test]
fn row_13_hash_string_high_bit() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0013);
    // exhaustive single high byte
    for v in 1..=255u8 {
        let mut s = vec![v, 0];
        for &seed in SEEDS.iter() {
            let (a, b) = hs(l, &mut s, seed);
            assert_eq!(a, b, "single byte {v} seed={seed:#x}");
        }
    }
    for n in 1..=40usize {
        for _ in 0..40 {
            let mut s: Vec<u8> = (0..n).map(|_| 0x80 | (rng.below(0x80) as u8)).collect();
            s.push(0);
            let seed = rng.next_u64() as usize;
            let (a, b) = hs(l, &mut s, seed);
            assert_eq!(a, b, "high-bit n={n} seed={seed:#x}");
        }
    }
    // mixed
    for _ in 0..2000 {
        let n = rng.below(64) + 1;
        let mut s: Vec<u8> = (0..n).map(|_| (rng.below(255) as u8) + 1).collect();
        s.push(0);
        let seed = rng.next_u64() as usize;
        let (a, b) = hs(l, &mut s, seed);
        assert_eq!(a, b, "mixed n={n}");
    }
}

/// row 14: long string (> arena blocksize).
#[test]
fn row_14_hash_string_long() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0014);
    for n in [128usize, 511, 512, 513, 600, 1023, 4096] {
        for _ in 0..8 {
            let mut s = rng.cstring(n);
            let seed = rng.next_u64() as usize;
            let (a, b) = hs(l, &mut s, seed);
            assert_eq!(a, b, "long n={n}");
        }
    }
}

/// row 15: `stbds_rand_seed` must not affect the two pure hash functions
/// (they take the seed as an argument).
#[test]
fn row_15_rand_seed_does_not_affect_pure_hashes() {
    let (l, _g) = libs();
    let mut rng = Rng::new(0xB0_0015);
    let mut buf = rng.bytes(29);
    let mut s = rng.cstring(17);
    let base_b = hb(l, &mut buf, 29, 7);
    let base_s = hs(l, &mut s, 7);
    assert_eq!(base_b.0, base_b.1);
    assert_eq!(base_s.0, base_s.1);
    for g in [0usize, 1, 12345, usize::MAX, 0x3141_5926] {
        seed_both(l, g);
        let nb = hb(l, &mut buf, 29, 7);
        let ns = hs(l, &mut s, 7);
        assert_eq!(nb, base_b, "hash_bytes changed after rand_seed({g})");
        assert_eq!(ns, base_s, "hash_string changed after rand_seed({g})");
    }
    seed_both(l, 0x3141_5926);
}
