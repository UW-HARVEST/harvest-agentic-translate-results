//! Phase B — CONFIGS.md rows C1..C6: the pure hashing layer, driven through
//! both `.so` exports.
mod common;

use common::*;
use std::ffi::{c_char, c_void};

/// C1 — `stbds_hash_bytes` for every length 0..=72 with many random buffers.
#[test]
fn cfg_hash_bytes_all_lengths() {
    let l = libs();
    let mut rng = Rng::new(0xC1_0000);
    for len in 0..=72usize {
        for _ in 0..200 {
            let mut buf = rng.bytes(len + 8); // + slack so OOB reads would show
            let p = buf.as_mut_ptr() as *mut c_void;
            unsafe {
                let a = (l.c.hash_bytes)(p, len, 0x3141_5926);
                let b = (l.r.hash_bytes)(p, len, 0x3141_5926);
                assert_eq!(a, b, "hash_bytes len={len} buf={buf:02x?}");
            }
        }
    }
}

/// C2 — seed matrix.
#[test]
fn cfg_hash_bytes_seed_matrix() {
    let l = libs();
    let mut rng = Rng::new(0xC2_0000);
    let mut seeds: Vec<usize> = vec![
        0,
        1,
        2,
        3,
        0x3141_5926,
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        1 << 63,
        (1 << 63) | 1,
    ];
    for _ in 0..64 {
        seeds.push(rng.next_u64() as usize);
    }
    for seed in seeds {
        for _ in 0..64 {
            let len = rng.below(257);
            let mut buf = rng.bytes(len);
            let p = if len == 0 {
                std::ptr::null_mut()
            } else {
                buf.as_mut_ptr() as *mut c_void
            };
            unsafe {
                let a = (l.c.hash_bytes)(p, len, seed);
                let b = (l.r.hash_bytes)(p, len, seed);
                assert_eq!(a, b, "hash_bytes len={len} seed={seed:#x}");
            }
        }
    }
}

/// C3 — high-bit patterns: the `d[3]<<24` / `d[7]<<24` sign-extension paths.
#[test]
fn cfg_hash_bytes_high_bit_patterns() {
    let l = libs();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for len in 0..=17usize {
        cases.push(vec![0x00; len]);
        cases.push(vec![0xff; len]);
        cases.push(vec![0x80; len]);
        cases.push(vec![0x7f; len]);
        // 0x80 at every single position, 0 elsewhere
        for pos in 0..len {
            let mut v = vec![0u8; len];
            v[pos] = 0x80;
            cases.push(v);
            let mut v = vec![0u8; len];
            v[pos] = 0xff;
            cases.push(v);
        }
    }
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        for c in &cases {
            let mut buf = c.clone();
            let p = buf.as_mut_ptr() as *mut c_void;
            unsafe {
                let a = (l.c.hash_bytes)(p, buf.len(), seed);
                let b = (l.r.hash_bytes)(p, buf.len(), seed);
                assert_eq!(a, b, "hash_bytes {buf:02x?} seed={seed:#x}");
            }
        }
    }
}

/// C4 — `stbds_hash_string` over random printable strings and the seed matrix.
#[test]
fn cfg_hash_string_ascii() {
    let l = libs();
    let mut rng = Rng::new(0xC4_0000);
    for len in 0..=64usize {
        for _ in 0..60 {
            let mut s: Vec<u8> = (0..len).map(|_| 0x20 + (rng.byte() % 0x5f)).collect();
            s.push(0);
            for seed in [0usize, 1, 0x3141_5926, usize::MAX, rng.next_u64() as usize] {
                unsafe {
                    let a = (l.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    let b = (l.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    assert_eq!(a, b, "hash_string {s:02x?} seed={seed:#x}");
                }
            }
        }
    }
}

/// C5 — `stbds_hash_string` with bytes >= 0x80 (`(unsigned char) *str`).
#[test]
fn cfg_hash_string_high_bit() {
    let l = libs();
    let mut rng = Rng::new(0xC5_0000);
    let mut cases: Vec<Vec<u8>> = vec![vec![0u8], vec![0x80, 0], vec![0xff, 0]];
    for len in 1..=64usize {
        // only high-bit bytes
        let mut s: Vec<u8> = (0..len).map(|_| 0x80 | (rng.byte() & 0x7f)).collect();
        s.push(0);
        cases.push(s);
        // mixed
        let mut s: Vec<u8> = (0..len).map(|_| 1 + (rng.byte() % 255)).collect();
        s.push(0);
        cases.push(s);
    }
    for seed in [0usize, 1, 7, 0x3141_5926, usize::MAX, usize::MAX / 3] {
        for c in &cases {
            let mut s = c.clone();
            unsafe {
                let a = (l.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                let b = (l.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                assert_eq!(a, b, "hash_string {s:02x?} seed={seed:#x}");
            }
        }
    }
}

/// C6 — `stbds_rand_seed` + the seed-advancement chain
/// (`hash_seed = hash_seed*a + b` inside `stbds_make_hash_index`).
///
/// Observed through `stbds_shmode_func`, which creates a fresh index each time
/// and copies the *current* global seed into `table->seed`.
#[test]
fn cfg_rand_seed_and_advance() {
    let _g = seed_lock();
    let l = libs();
    let mut rng = Rng::new(0xC6_0000);
    let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1, 1 << 63];
    for _ in 0..32 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        seed_both(s);
        let shape = Shape::str_ptr(16);
        for round in 0..4 {
            unsafe {
                let cp = (l.c.shmode_func)(shape.elemsize, SH_ARENA);
                let rp = (l.r.shmode_func)(shape.elemsize, SH_ARENA);
                let ct = (*header_of(cp, shape.elemsize)).hash_table as *mut HashIndex;
                let rt = (*header_of(rp, shape.elemsize)).hash_table as *mut HashIndex;
                assert_eq!(
                    (*ct).seed,
                    (*rt).seed,
                    "table->seed after rand_seed({s:#x}) round {round}"
                );
                assert_same(
                    &format!("rand_seed({s:#x}) round {round}"),
                    &fingerprint(cp, shape),
                    &fingerprint(rp, shape),
                );
                (l.c.hmfree_func)(
                    (cp as *mut u8).sub(shape.elemsize) as *mut c_void,
                    shape.elemsize,
                );
                (l.r.hmfree_func)(
                    (rp as *mut u8).sub(shape.elemsize) as *mut c_void,
                    shape.elemsize,
                );
            }
        }
    }
    // Leave both libraries in the default state for the other tests.
    seed_both(0x3141_5926);
}
