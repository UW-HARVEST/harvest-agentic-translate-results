//! Phase B — CONFIGS.md rows 1..11: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed` seed chaining.

mod common;
use common::*;
use std::ffi::c_void;

fn cmp_bytes(buf: &[u8], seed: usize, ctx: &str) {
    let (c, r) = pair();
    let b = CBuf::new(buf);
    unsafe {
        let hc = (c.hash_bytes)(b.as_void(), buf.len(), seed);
        let hr = (r.hash_bytes)(b.as_void(), buf.len(), seed);
        assert_eq!(
            hc, hr,
            "hash_bytes mismatch ({ctx}) len={} seed={seed:#x} buf={:02x?}",
            buf.len(),
            buf
        );
    }
}

fn cmp_string(s: &[u8], seed: usize, ctx: &str) {
    let (c, r) = pair();
    let b = CBuf::cstr(s);
    unsafe {
        let hc = (c.hash_string)(b.as_char(), seed);
        let hr = (r.hash_string)(b.as_char(), seed);
        assert_eq!(hc, hr, "hash_string mismatch ({ctx}) seed={seed:#x} s={s:02x?}");
    }
}

// ---------------------------------------------------------------- row 1
#[test]
fn bytes_len0() {
    let mut rng = Rng::new(0xA1);
    cmp_bytes(&[], 0, "len0 seed0");
    for _ in 0..512 {
        cmp_bytes(&[], rng.next_u64() as usize, "len0 random seed");
    }
    // NULL pointer with len 0 must not be dereferenced (ERRORS.md row 31)
    let (c, r) = pair();
    unsafe {
        for s in [0usize, 1, 0x31415926, usize::MAX] {
            assert_eq!(
                (c.hash_bytes)(std::ptr::null_mut::<c_void>(), 0, s),
                (r.hash_bytes)(std::ptr::null_mut::<c_void>(), 0, s)
            );
        }
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn bytes_tail_lengths() {
    let mut rng = Rng::new(0xB2);
    for len in 1..=7usize {
        for _ in 0..512 {
            let buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            cmp_bytes(&buf, seed, "tail");
        }
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn bytes_whole_blocks() {
    let mut rng = Rng::new(0xC3);
    for k in 1..=32usize {
        let len = k * 8;
        for _ in 0..64 {
            let buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            cmp_bytes(&buf, seed, "whole blocks");
        }
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn bytes_random_lengths() {
    let mut rng = Rng::new(0xD4);
    for _ in 0..4000 {
        let len = rng.below(256);
        let buf = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        cmp_bytes(&buf, seed, "random len");
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn bytes_high_bit() {
    let mut rng = Rng::new(0xE5);
    // every byte >= 0x80 exercises the sign-extending `d[3] << 24` in both the
    // block loop and the `case 4` tail arm.
    for len in 0..=64usize {
        for _ in 0..32 {
            let buf = rng.cbytes(len, 0x80, 0xff);
            let seed = rng.next_u64() as usize;
            cmp_bytes(&buf, seed, "high bit");
        }
    }
    // exactly the 4th byte high, everything else low (isolates `case 4`)
    for len in 4..=11usize {
        let mut buf = vec![0x01u8; len];
        buf[3] = 0x80;
        cmp_bytes(&buf, 0, "byte3 = 0x80");
        buf[3] = 0xff;
        cmp_bytes(&buf, 0, "byte3 = 0xff");
        if len >= 8 {
            buf[7] = 0xff;
            cmp_bytes(&buf, 0, "byte7 = 0xff");
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn bytes_boundary_seeds() {
    let seeds = [
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        1usize << 63,
        (1usize << 63) - 1,
        0x3141_5926,
        0xffff_ffff,
        0x1_0000_0000,
    ];
    let pats: [&[u8]; 8] = [
        &[],
        &[0x00],
        &[0xff],
        &[0x80],
        &[0x7f],
        &[0x00; 8],
        &[0xff; 16],
        &[0x80; 7],
    ];
    for &s in &seeds {
        for p in pats.iter() {
            cmp_bytes(p, s, "boundary");
        }
        for len in 0..=17usize {
            cmp_bytes(&vec![0xffu8; len], s, "all ff");
            cmp_bytes(&vec![0x00u8; len], s, "all 00");
            cmp_bytes(&vec![0x80u8; len], s, "all 80");
            cmp_bytes(&vec![0x7fu8; len], s, "all 7f");
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn string_empty() {
    let mut rng = Rng::new(0xF7);
    cmp_string(b"", 0, "empty seed0");
    for _ in 0..512 {
        cmp_string(b"", rng.next_u64() as usize, "empty random seed");
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn string_random() {
    let mut rng = Rng::new(0x18);
    for _ in 0..4000 {
        let len = 1 + rng.below(64);
        let s = rng.cbytes(len, 0x20, 0x7e);
        let seed = rng.next_u64() as usize;
        cmp_string(&s, seed, "random ascii");
    }
    for seed in [0usize, 1, usize::MAX, 0x3141_5926] {
        for len in 1..=32usize {
            cmp_string(&vec![b'a'; len], seed, "aaa");
            cmp_string(&vec![b'\x7f'; len], seed, "del");
        }
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn string_high_bytes() {
    let mut rng = Rng::new(0x29);
    for _ in 0..2000 {
        let len = 1 + rng.below(64);
        let s = rng.cbytes(len, 0x80, 0xff);
        let seed = rng.next_u64() as usize;
        cmp_string(&s, seed, "high bytes");
    }
    for len in 1..=16usize {
        cmp_string(&vec![0xffu8; len], 0, "all ff");
        cmp_string(&vec![0x80u8; len], usize::MAX, "all 80");
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn string_long() {
    let mut rng = Rng::new(0x3A);
    for len in [256usize, 1024, 4096] {
        for _ in 0..8 {
            let s = rng.cbytes(len, 0x01, 0xff);
            let seed = rng.next_u64() as usize;
            cmp_string(&s, seed, "long");
        }
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn seed_lcg_chain() {
    // Every fresh `make_hash_index` snapshots the global seed and then advances
    // it with an LCG.  Create a chain of fresh tables and compare the whole
    // `seed` sequence.
    let (c, r) = pair();
    let elemsize = 16usize;
    for start in [0usize, 1, 0x3141_5926, usize::MAX, 0xdead_beef_cafe_babe] {
        unsafe {
            (c.rand_seed)(start);
            (r.rand_seed)(start);
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            let mut cmaps = Vec::new();
            let mut rmaps = Vec::new();
            for _ in 0..40 {
                let tc = (c.shmode_func)(elemsize, SH_ARENA);
                let tr = (r.shmode_func)(elemsize, SH_ARENA);
                cs.push((*map_table(tc, elemsize)).seed);
                rs.push((*map_table(tr, elemsize)).seed);
                cmaps.push(tc);
                rmaps.push(tr);
            }
            assert_eq!(cs, rs, "seed chain diverged for start={start:#x}");
            for t in cmaps {
                (c.hmfree_func)(raw_of(t, elemsize), elemsize);
            }
            for t in rmaps {
                (r.hmfree_func)(raw_of(t, elemsize), elemsize);
            }
        }
    }
}
