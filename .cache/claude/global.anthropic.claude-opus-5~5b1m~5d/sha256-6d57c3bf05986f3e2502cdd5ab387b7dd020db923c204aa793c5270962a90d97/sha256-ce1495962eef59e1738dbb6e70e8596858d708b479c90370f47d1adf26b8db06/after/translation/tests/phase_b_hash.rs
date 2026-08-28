//! Phase B — CONFIGS.md rows 1..9: the lowest-level entry points
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`).

mod common;

use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: [usize; 6] = [
    0,
    1,
    0x3141_5926,
    usize::MAX,
    0x8000_0000_0000_0000,
    0x0f0e_0d0c_0b0a_0908,
];

unsafe fn hash_bytes_eq(c: &Api, r: &Api, buf: &[u8], len: usize, seed: usize) {
    let p = buf.as_ptr() as *mut c_void;
    let hc = (c.hash_bytes)(p, len, seed);
    let hr = (r.hash_bytes)(p, len, seed);
    assert_eq!(
        hc, hr,
        "stbds_hash_bytes(len={}, seed={:#x}, bytes={:?}) C={:#x} Rust={:#x}",
        len,
        seed,
        &buf[..len.min(buf.len())],
        hc,
        hr
    );
}

// row 1 -----------------------------------------------------------------------
#[test]
fn cfg_01_hash_bytes_len0() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_01);
    unsafe {
        let buf = [0u8; 8];
        for s in SEEDS {
            hash_bytes_eq(c, r, &buf, 0, s);
        }
        for _ in 0..1000 {
            hash_bytes_eq(c, r, &buf, 0, rng.next_u64() as usize);
        }
    }
}

// rows 2 + 3 ------------------------------------------------------------------
#[test]
fn cfg_02_03_hash_bytes_tail_lengths() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_02);
    unsafe {
        for len in 1..8usize {
            for _ in 0..200 {
                // row 2: all bytes < 0x80
                let lo: Vec<u8> = (0..8).map(|_| rng.byte() & 0x7f).collect();
                // row 3: all bytes >= 0x80 (int sign-extension in the C)
                let hi: Vec<u8> = (0..8).map(|_| rng.byte() | 0x80).collect();
                let mixed: Vec<u8> = (0..8).map(|_| rng.byte()).collect();
                for s in SEEDS {
                    hash_bytes_eq(c, r, &lo, len, s);
                    hash_bytes_eq(c, r, &hi, len, s);
                    hash_bytes_eq(c, r, &mixed, len, s);
                }
            }
        }
    }
}

// rows 4 + 5 ------------------------------------------------------------------
#[test]
fn cfg_04_05_hash_bytes_word_lengths() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_04);
    unsafe {
        for len in 8..72usize {
            for _ in 0..40 {
                let mixed = rng.bytes(len);
                let hi: Vec<u8> = (0..len).map(|_| rng.byte() | 0x80).collect();
                let lo: Vec<u8> = (0..len).map(|_| rng.byte() & 0x7f).collect();
                for s in [SEEDS[0], SEEDS[2], SEEDS[3]] {
                    hash_bytes_eq(c, r, &mixed, len, s);
                    hash_bytes_eq(c, r, &hi, len, s);
                    hash_bytes_eq(c, r, &lo, len, s);
                }
                hash_bytes_eq(c, r, &mixed, len, rng.next_u64() as usize);
            }
        }
    }
}

// row 6 -----------------------------------------------------------------------
#[test]
fn cfg_06_hash_bytes_large() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_06);
    unsafe {
        for len in [256usize, 257, 511, 512, 1000, 4096] {
            for _ in 0..20 {
                let b = rng.bytes(len);
                for s in SEEDS {
                    hash_bytes_eq(c, r, &b, len, s);
                }
                hash_bytes_eq(c, r, &b, len, rng.next_u64() as usize);
            }
        }
    }
}

// rows 7 + 8 ------------------------------------------------------------------
#[test]
fn cfg_07_08_hash_string() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_07);
    unsafe {
        let check = |s: &[u8], seed: usize| {
            let p = s.as_ptr() as *mut c_char;
            let hc = (c.hash_string)(p, seed);
            let hr = (r.hash_string)(p, seed);
            assert_eq!(
                hc, hr,
                "stbds_hash_string({:?}, {:#x}) C={:#x} Rust={:#x}",
                s, seed, hc, hr
            );
        };
        // fixed corner cases
        for s in SEEDS {
            check(b"\0", s);
            check(b"a\0", s);
            check(b"\xff\0", s);
            check(b"1234567\0", s);
            check(b"12345678\0", s);
            check(b"123456789\0", s);
            check(b"\x80\x81\x82\x83\x84\x85\x86\x87\x88\0", s);
            let long: Vec<u8> = (0..4096).map(|i| 1 + (i % 255) as u8).chain([0]).collect();
            check(&long, s);
        }
        // randomized
        for len in [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 33, 64, 129] {
            for _ in 0..60 {
                let s = rng.cstring(len);
                let ascii: Vec<u8> = (0..len)
                    .map(|_| 0x20 + (rng.byte() % 0x5f))
                    .chain([0])
                    .collect();
                for seed in SEEDS {
                    check(&s, seed);
                    check(&ascii, seed);
                }
                check(&s, rng.next_u64() as usize);
            }
        }
    }
}

// row 9 -----------------------------------------------------------------------
#[test]
fn cfg_09_rand_seed_chain() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE_09);
    unsafe {
        let mut seeds: Vec<usize> = SEEDS.to_vec();
        for _ in 0..20 {
            seeds.push(rng.next_u64() as usize);
        }
        for s in seeds {
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            // eight consecutive *fresh* hash indices: each takes the current
            // global seed and then advances it (seed = seed*a + b)
            let mut maps_c = Vec::new();
            let mut maps_r = Vec::new();
            for k in 0..8 {
                let tc = (c.shmode_func)(16, STBDS_SH_DEFAULT) as *mut u8;
                let tr = (r.shmode_func)(16, STBDS_SH_DEFAULT) as *mut u8;
                let hc = read_header(tc.sub(16));
                let hr = read_header(tr.sub(16));
                let ic = std::ptr::read_unaligned(hc.hash_table as *const HashIndex);
                let ir = std::ptr::read_unaligned(hr.hash_table as *const HashIndex);
                assert_eq!(
                    ic.seed, ir.seed,
                    "table #{} seed after rand_seed({:#x}): C={:#x} Rust={:#x}",
                    k, s, ic.seed, ir.seed
                );
                if k == 0 {
                    assert_eq!(ic.seed, s, "first table must use the seed verbatim");
                }
                maps_c.push(tc);
                maps_r.push(tr);
            }
            for (tc, tr) in maps_c.into_iter().zip(maps_r) {
                (c.hmfree_func)(tc.sub(16) as *mut c_void, 16);
                (r.hmfree_func)(tr.sub(16) as *mut c_void, 16);
            }
        }
    }
}
